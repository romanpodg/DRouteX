//! Qualified D52 client. The root-owned worker keeps each TCP owner across
//! cancellable request futures and explicitly joins it before another transport.

use super::io::{ATTEMPT, Budget, MESSAGE_BYTES, Provider};
use hickory_net::{
    DnsMultiplexer, NetError,
    tcp::TcpClientStream,
    xfer::{DnsExchange, DnsHandle, DnsRequestSender, FirstAnswer},
};
use hickory_proto::{
    op::{DnsRequest, DnsResponse, Header, MessageType, OpCode, Query},
    rr::{Name, RData, Record, RecordType},
    serialize::binary::{BinDecodable, BinDecoder},
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::{
    task::JoinHandle,
    time::{Instant, timeout, timeout_at},
};

pub(super) const CANDIDATES: usize = 16;
const CLEANUP: Duration = Duration::from_secs(3);
#[derive(Debug)]
pub(super) enum Failure {
    Deadline,
    InvalidRequest,
    Rejected,
    Candidates,
    Transport,
    Cleanup,
    Background,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Failure {}
struct Expected {
    question: Query,
    opcode: OpCode,
}
impl Expected {
    fn new(request: &DnsRequest) -> Result<Self, Failure> {
        if request.metadata.message_type != MessageType::Query
            || request.metadata.op_code != OpCode::Query
            || request.queries.len() != 1
            || request.to_vec().map_err(|_| Failure::InvalidRequest)?.len() > MESSAGE_BYTES
        {
            return Err(Failure::InvalidRequest);
        }
        // The original question must outlive moving the request into the library.
        Ok(Self {
            question: request.queries[0].clone(),
            opcode: request.metadata.op_code,
        })
    }
    fn accepts(&self, response: &DnsResponse) -> bool {
        let bytes = response.as_buffer();
        if bytes.len() > MESSAGE_BYTES {
            return false;
        }
        let mut decoder = BinDecoder::new(bytes);
        let Ok(header) = Header::read(&mut decoder) else {
            return false;
        };
        if header.metadata.message_type != MessageType::Response
            || header.metadata.op_code != self.opcode
            || header.counts.queries != 1
        {
            return false;
        }
        if Query::read(&mut decoder).ok().as_ref() != Some(&self.question) {
            return false;
        }
        let mut opt_seen = false;
        for (section, count) in [
            header.counts.answers,
            header.counts.authorities,
            header.counts.additionals,
        ]
        .into_iter()
        .enumerate()
        {
            for _ in 0..count {
                let Ok(index) = u16::try_from(decoder.index()) else {
                    return false;
                };
                let mut metadata = decoder.clone(index);
                if Name::read(&mut metadata).is_err() {
                    return false;
                }
                let rdata_start = metadata.index() + 10;
                let Ok(record) = Record::read(&mut decoder) else {
                    return false;
                };
                match &record.data {
                    RData::OPT(options) => {
                        if section != 2 || opt_seen || !record.name.is_root() {
                            return false;
                        }
                        opt_seen = true;
                        // Proto's OPT decoder can clear incomplete options. Account
                        // for every original byte, as in the approved D47 decoder.
                        let decoded = options
                            .as_ref()
                            .iter()
                            .map(|(_, option)| 4 + usize::from(option.len()))
                            .sum::<usize>();
                        if decoder.index().checked_sub(rdata_start) != Some(decoded) {
                            return false;
                        }
                    }
                    RData::Update0(RecordType::OPT) => {
                        if section != 2 || opt_seen {
                            return false;
                        }
                        opt_seen = true;
                    }
                    RData::Update0(_) => return false,
                    _ if record.dns_class != self.question.query_class => return false,
                    _ => (),
                }
            }
        }
        decoder.is_empty()
    }
}
async fn current(budget: &Budget) -> Result<(), Failure> {
    if Instant::now() >= budget.deadline {
        Err(Failure::Deadline)
    } else {
        budget
            .live
            .current(budget.deadline)
            .await
            .map_err(|_| Failure::Rejected)
    }
}
fn transport(budget: &Budget, _: NetError) -> Failure {
    if Instant::now() >= budget.deadline {
        Failure::Deadline
    } else {
        Failure::Transport
    }
}

pub(super) async fn udp(budget: Arc<Budget>, request: DnsRequest) -> Result<DnsResponse, Failure> {
    let expected = Expected::new(&request)?;
    current(&budget).await?;
    let mut client =
        hickory_net::udp::UdpClientStream::builder(budget.peer, Provider(Arc::clone(&budget)))
            .with_timeout(Some(ATTEMPT))
            .with_max_retries(0)
            .build();
    let response = timeout_at(budget.deadline, client.send_message(request).first_answer())
        .await
        .map_err(|_| Failure::Deadline)?
        .map_err(|error| transport(&budget, error))?;
    current(&budget).await?;
    if !expected.accepts(&response) {
        budget.counters.rejected.fetch_add(1, Ordering::Relaxed);
        return Err(Failure::Rejected);
    }
    current(&budget).await?;
    Ok(response)
}

pub(super) struct TcpAttempt {
    exchange: Option<DnsExchange<Provider>>,
    background: Option<JoinHandle<()>>,
    budget: Arc<Budget>,
}
impl TcpAttempt {
    pub async fn connect(budget: Arc<Budget>) -> Result<Self, Failure> {
        current(&budget).await?;
        let (connecting, sender) = TcpClientStream::new(
            budget.peer,
            None,
            Some(ATTEMPT),
            Provider(Arc::clone(&budget)),
        );
        let stream = timeout_at(budget.deadline, connecting)
            .await
            .map_err(|_| Failure::Deadline)?
            .map_err(|error| transport(&budget, error))?;
        current(&budget).await?;
        let multiplexer = DnsMultiplexer::new(stream, sender)
            .with_timeout(ATTEMPT)
            .with_max_active_requests(1);
        let (exchange, background) = DnsExchange::from_stream(multiplexer);
        Ok(Self {
            exchange: Some(exchange),
            background: Some(tokio::spawn(background)),
            budget,
        })
    }
    pub async fn query(&mut self, request: DnsRequest) -> Result<DnsResponse, Failure> {
        let expected = Expected::new(&request)?;
        current(&self.budget).await?;
        // Taking the handle makes the attempt one-shot even if this query future
        // is cancelled. The caller retains only the owner needed for finish().
        let exchange = self.exchange.take().ok_or(Failure::Cleanup)?;
        let mut responses = exchange.send(request);
        timeout_at(self.budget.deadline, async {
            for _ in 0..CANDIDATES {
                let response = (&mut responses)
                    .first_answer()
                    .await
                    .map_err(|error| transport(&self.budget, error))?;
                current(&self.budget).await?;
                if expected.accepts(&response) {
                    current(&self.budget).await?;
                    return Ok(response);
                }
                self.budget
                    .counters
                    .rejected
                    .fetch_add(1, Ordering::Relaxed);
                #[cfg(test)]
                self.budget.counters.rejected_wake.notify_one();
            }
            Err(Failure::Candidates)
        })
        .await
        .map_err(|_| Failure::Deadline)?
    }
    pub async fn finish(mut self) -> Result<(), Failure> {
        drop(self.exchange.take());
        if let Some(mut background) = self.background.take() {
            background.abort();
            match timeout(CLEANUP, &mut background).await {
                Ok(Ok(())) => (),
                Ok(Err(error)) if error.is_cancelled() => (),
                Ok(Err(_)) => return Err(Failure::Background),
                Err(_) => {
                    // No fallback or late answer after the3s cleanup bound. Keep
                    // the root-owned worker/slot until this same handle joins.
                    let _ = background.await;
                    return Err(Failure::Cleanup);
                }
            }
        }
        Ok(())
    }
}
impl Drop for TcpAttempt {
    fn drop(&mut self) {
        // Emergency cancellation only. The qualified caller must await finish(),
        // preserving the owner outside any cancelled query future.
        if let Some(background) = &self.background {
            background.abort();
        }
    }
}
