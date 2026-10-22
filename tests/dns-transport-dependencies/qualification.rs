//! D47 library qualification only; not a DRouteX listener or live authorization.
mod codec;
mod network;

use std::{error::Error, time::Duration};
type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;
const DEADLINE: Duration = Duration::from_secs(5);

// Independent fixed wire vectors: ID1234, IN A example.test; no library encoder.
const QUERY: &[u8] =
    b"\x04\xd2\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01";
const FAILURE: &[u8] =
    b"\x04\xd2\x81\x02\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01";
const ANSWER: &[u8] = b"\x04\xd2\x81\x80\x00\x01\x00\x01\x00\x00\x00\x00\x07example\x04test\x00\x00\x01\x00\x01\xc0\x0c\x00\x01\x00\x01\x00\x00\x00\x3c\x00\x04\xcb\x00\x71\x09";
