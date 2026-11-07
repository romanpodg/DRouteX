//! Experimental direct relay used only by the opt-in preview listener.
use super::{OriginConnection, ResolveError, current, guarded};
use crate::tls_metadata::ObservedHello;
use droutex_core::deployment::{AdmissionScope, AuthorizedRequest};
use std::{error::Error, fmt, future::Future, io, num::NonZeroU16, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    sync::watch,
    time::Instant,
};
const LIFE: Duration = Duration::from_secs(120);
const IDLE: Duration = Duration::from_secs(15);
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_OPERATIONS: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelayError {
    Admission,
    Io,
    Timeout,
    Cancelled,
    Limit,
    NameMismatch,
}
impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gateway relay {self:?}")
    }
}
impl Error for RelayError {}
fn convert(error: ResolveError) -> RelayError {
    match error {
        ResolveError::Timeout => RelayError::Timeout,
        ResolveError::Admission => RelayError::Admission,
        _ => RelayError::Io,
    }
}

/// Counts bytes forwarded in each direction, including the original ClientHello.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelayReport {
    pub client_to_origin: usize,
    pub origin_to_client: usize,
}

impl OriginConnection<'_, '_> {
    /// Forward only the exact observed ClientHello's admitted destination.
    /// The caller still owns listener, pre-admission limits and shutdown.
    /// The opt-in preview listener uses this experimental method; the default
    /// fail-closed listener does not.
    pub async fn relay(
        self,
        hello: ObservedHello<TcpStream>,
        shutdown: watch::Receiver<bool>,
        outer_deadline: Instant,
    ) -> Result<RelayReport, RelayError> {
        self.relay_with_idle(hello, shutdown, outer_deadline, IDLE)
            .await
    }

    pub(super) async fn relay_with_idle(
        mut self,
        hello: ObservedHello<TcpStream>,
        shutdown: watch::Receiver<bool>,
        outer_deadline: Instant,
        idle: Duration,
    ) -> Result<RelayReport, RelayError> {
        let origin = &self._origin;
        let (mut client, name, hello) = hello.into_parts();
        if name.as_str() != origin.domain {
            return Err(RelayError::NameMismatch);
        }
        let deadline = outer_deadline.min(Instant::now() + LIFE);
        current(origin.owner, &origin.domain, origin.port, deadline).map_err(convert)?;
        origin
            .owner
            .verify_smartdns_gateway_destination(&origin.domain, origin.port)
            .map_err(|_| RelayError::Admission)?;
        let AdmissionScope::Gateway(candidate) = origin.owner.scope() else {
            return Err(RelayError::Admission);
        };
        let peer = client.peer_addr().map_err(|_| RelayError::Io)?;
        let local = client.local_addr().map_err(|_| RelayError::Io)?;
        let binding = origin
            .owner
            .decision()
            .gateway_configuration()
            .document()
            .listeners
            .iter()
            .any(|listener| {
                listener.id == candidate.identity().listener
                    && listener.enabled
                    && listener.bind == local
            });
        if peer.ip() != candidate.identity().source || !binding {
            return Err(RelayError::Admission);
        }
        let (activity, activity_rx) = watch::channel(Instant::now());
        let context = Context {
            auth: origin.owner,
            domain: &origin.domain,
            port: origin.port,
            deadline,
            activity: &activity,
        };
        tokio::select! {
            biased;
            result = idle_timeout(activity_rx, idle) => Err(result),
            result = transfer(&mut client, &mut self._stream, &hello, context, shutdown) => result,
        }
    }
}

async fn transfer(
    client: &mut TcpStream,
    origin: &mut TcpStream,
    hello: &[u8],
    context: Context<'_, '_>,
    shutdown: watch::Receiver<bool>,
) -> Result<RelayReport, RelayError> {
    let mut stop = shutdown.clone();
    let mut budget = Budget {
        bytes: hello.len(),
        operations: 0,
    };
    write_bytes(origin, hello, context, &mut stop, &mut budget).await?;
    let (client_read, client_write) = client.split();
    let (origin_read, origin_write) = origin.split();
    let left = pump(client_read, origin_write, context, shutdown.clone(), budget);
    let right = pump(
        origin_read,
        client_write,
        context,
        shutdown,
        Budget {
            bytes: 0,
            operations: 0,
        },
    );
    let (client_to_origin, origin_to_client) = tokio::try_join!(left, right)?;
    Ok(RelayReport {
        client_to_origin,
        origin_to_client,
    })
}

async fn idle_timeout(mut activity: watch::Receiver<Instant>, idle: Duration) -> RelayError {
    loop {
        let cutoff = *activity.borrow_and_update() + idle;
        tokio::select! {
            biased;
            changed = activity.changed() => if changed.is_err() { return RelayError::Cancelled; },
            () = tokio::time::sleep_until(cutoff) => return RelayError::Timeout,
        }
    }
}

#[derive(Clone, Copy)]
struct Context<'request, 'store> {
    auth: &'request AuthorizedRequest<'store>,
    domain: &'request str,
    port: NonZeroU16,
    deadline: Instant,
    activity: &'request watch::Sender<Instant>,
}
struct Budget {
    bytes: usize,
    operations: usize,
}
impl Budget {
    fn operation(&mut self) -> Result<(), RelayError> {
        self.operations += 1;
        if self.operations > MAX_OPERATIONS {
            return Err(RelayError::Limit);
        }
        Ok(())
    }
    fn add(&mut self, bytes: usize) -> Result<(), RelayError> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|count| *count <= MAX_BYTES)
            .ok_or(RelayError::Limit)?;
        Ok(())
    }
}
async fn operation<T>(
    context: Context<'_, '_>,
    stop: &mut watch::Receiver<bool>,
    future: impl Future<Output = io::Result<T>>,
) -> Result<T, RelayError> {
    let future = guarded(
        context.auth,
        context.domain,
        context.port,
        context.deadline,
        future,
    );
    tokio::pin!(future);
    loop {
        if *stop.borrow() {
            return Err(RelayError::Cancelled);
        }
        tokio::select! {
            biased;
            changed = stop.changed() => if changed.is_err() || *stop.borrow_and_update() { return Err(RelayError::Cancelled); },
            result = &mut future => return result.map_err(convert),
        }
    }
}
async fn write_bytes<W: AsyncWrite + Unpin>(
    writer: &mut W,
    bytes: &[u8],
    context: Context<'_, '_>,
    stop: &mut watch::Receiver<bool>,
    budget: &mut Budget,
) -> Result<(), RelayError> {
    let mut offset = 0;
    while offset < bytes.len() {
        budget.operation()?;
        let size = operation(context, stop, writer.write(&bytes[offset..])).await?;
        if size == 0 {
            return Err(RelayError::Io);
        }
        context.activity.send_replace(Instant::now());
        offset += size;
    }
    Ok(())
}
async fn pump<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    mut reader: R,
    mut writer: W,
    context: Context<'_, '_>,
    mut stop: watch::Receiver<bool>,
    mut budget: Budget,
) -> Result<usize, RelayError> {
    let mut buffer = [0u8; 8192];
    loop {
        budget.operation()?;
        let size = operation(context, &mut stop, reader.read(&mut buffer)).await?;
        if size == 0 {
            operation(context, &mut stop, writer.shutdown()).await?;
            return Ok(budget.bytes);
        }
        context.activity.send_replace(Instant::now());
        budget.add(size)?;
        write_bytes(
            &mut writer,
            &buffer[..size],
            context,
            &mut stop,
            &mut budget,
        )
        .await?;
    }
}
