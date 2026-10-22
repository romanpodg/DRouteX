use super::*;
use hickory_proto::{
    op::{Header, Message, MessageType, ResponseCode},
    rr::{RData, RecordType},
    serialize::binary::{BinDecodable, BinDecoder, BinEncodable, BinEncoder},
};

#[test]
fn independent_compressed_answer_and_every_truncated_prefix() -> TestResult {
    let answer = Message::from_vec(ANSWER)?;
    assert_eq!(answer.metadata.id, 1234);
    assert_eq!(answer.metadata.message_type, MessageType::Response);
    assert_eq!(answer.queries[0].name.to_ascii(), "example.test.");
    assert_eq!(answer.queries[0].query_type, RecordType::A);
    assert_eq!(answer.answers.len(), 1);
    assert_eq!(answer.answers[0].name.to_ascii(), "example.test.");
    assert_eq!(answer.answers[0].ttl, 60);
    assert!(
        matches!(answer.answers[0].data,RData::A(address) if address.0.octets()==[203,0,113,9])
    );
    for length in 0..ANSWER.len() {
        assert!(
            Message::from_vec(&ANSWER[..length]).is_err(),
            "prefix {length}"
        );
    }
    Ok(())
}

#[test]
fn decoder_exhaustion_is_required_beyond_from_vec() -> TestResult {
    let mut wire = QUERY.to_vec();
    wire.extend_from_slice(b"extra");
    // This convenience API accepts a complete message plus trailing bytes.
    assert!(Message::from_vec(&wire).is_ok());
    let mut decoder = BinDecoder::new(&wire);
    Message::read(&mut decoder)?;
    assert!(!decoder.is_empty());
    assert_eq!(decoder.index(), QUERY.len());
    assert_eq!(decoder.len(), 5);
    let mut exact = BinDecoder::new(QUERY);
    Message::read(&mut exact)?;
    assert!(exact.is_empty());
    Ok(())
}

#[test]
fn malformed_compression_and_impossible_counts_refuse() -> TestResult {
    let header = &QUERY[..12];
    for name in [b"\xc0\x0c".as_slice(), b"\xc0\xff", b"\x40", b"\x3fshort"] {
        let mut wire = header.to_vec();
        wire.extend_from_slice(name);
        wire.extend_from_slice(b"\x00\x01\x00\x01");
        assert!(Message::from_vec(&wire).is_err());
    }
    let mut wire = QUERY.to_vec();
    wire[6..8].copy_from_slice(&u16::MAX.to_be_bytes());
    let mut decoder = BinDecoder::new(&wire);
    let header = Header::read(&mut decoder)?;
    assert_eq!(header.counts.answers, u16::MAX);
    assert_eq!(decoder.index(), 12); // Application can reject counts before records.
    assert!(Message::from_vec(&wire).is_err());
    Ok(())
}

#[test]
fn independent_edns_version_size_and_unknown_option_are_visible() -> TestResult {
    let mut wire = QUERY.to_vec();
    wire[10..12].copy_from_slice(&1u16.to_be_bytes());
    // Root OPT, UDP1232, version0, DO, unknown option65001 with one byte.
    wire.extend_from_slice(b"\x00\x00\x29\x04\xd0\x00\x00\x80\x00\x00\x05\xfd\xe9\x00\x01\x2a");
    let message = Message::from_vec(&wire)?;
    let edns = message.edns.as_ref().ok_or("missing EDNS")?;
    assert_eq!(edns.max_payload(), 1232);
    assert_eq!(edns.version(), 0);
    assert!(edns.flags().dnssec_ok);
    assert_eq!(edns.options().as_ref().len(), 1);
    // Preserve an unsupported version for application BADVERS handling.
    wire[QUERY.len() + 6] = 1;
    assert_eq!(
        Message::from_vec(&wire)?
            .edns
            .ok_or("missing EDNS")?
            .version(),
        1
    );
    wire[QUERY.len() + 6] = 0;
    wire.pop();
    assert!(Message::from_vec(&wire).is_err());
    Ok(())
}

#[test]
fn encoder_budget_and_valid_tc_response_do_not_cut_raw_records() -> TestResult {
    let message = Message::from_vec(ANSWER)?;
    let mut output = Vec::new();
    let mut encoder = BinEncoder::new(&mut output);
    encoder.set_max_size(QUERY.len() as u16);
    // The pinned library emits a complete TC packet when answer records do not fit.
    message.emit(&mut encoder)?;
    assert!(output.len() <= QUERY.len());
    let bounded = Message::from_vec(&output)?;
    assert!(bounded.metadata.truncation);
    assert!(bounded.answers.is_empty());
    assert_eq!(bounded.queries, message.queries);
    let truncated = message.truncate();
    let mut output = Vec::new();
    let mut encoder = BinEncoder::new(&mut output);
    encoder.set_max_size(QUERY.len() as u16);
    truncated.emit(&mut encoder)?;
    let response = Message::from_vec(&output)?;
    assert!(response.metadata.truncation);
    assert_eq!(response.metadata.id, 1234);
    assert_eq!(response.queries, message.queries);
    assert!(response.answers.is_empty());
    assert_eq!(response.metadata.response_code, ResponseCode::NoError);
    assert!(output.len() <= QUERY.len());
    Ok(())
}
