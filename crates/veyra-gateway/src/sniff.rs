//! The destination of a new connection, from its first bytes: the SNI of a TLS
//! ClientHello or the `Host` header of an HTTP request. The bytes are only
//! read, never changed (spec §10.1).

use veyra_policy::Name;

/// Which listener a connection arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    /// TLS, routed by SNI; the origin is dialed on port 443.
    Tls,
    /// HTTP/1.x, routed by `Host`; the origin is dialed on port 80.
    Http,
}

/// What the bytes read so far say about the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sniff {
    /// Not enough bytes yet: read more.
    More,
    /// The destination to route by.
    Name(Name),
    /// No usable destination: not TLS or HTTP, no SNI or `Host`, two of them,
    /// an IP literal, or `MAX_HEAD` bytes without an answer.
    Refuse,
}

/// The most a client may send before naming its destination. ClientHellos are
/// a few KiB at most; HTTP heads with cookies can reach tens of KiB.
pub const MAX_HEAD: usize = 32 * 1024;

const HANDSHAKE: u8 = 22;
const CLIENT_HELLO: usize = 1;
const SERVER_NAME: usize = 0;
const HOST_NAME: usize = 0;

/// Read the destination from the first bytes of a connection.
pub fn sniff(service: Service, head: &[u8]) -> Sniff {
    let found = match service {
        Service::Tls => server_name(head),
        Service::Http => http_host(head),
    };
    match found {
        Found::Host(host) => destination(&host).map_or(Sniff::Refuse, Sniff::Name),
        Found::Partial if head.len() < MAX_HEAD => Sniff::More,
        Found::Partial | Found::Nothing => Sniff::Refuse,
    }
}

/// What a parser found in the bytes so far.
enum Found {
    Partial,
    Host(String),
    Nothing,
}

/// The name to route by. IP literals are refused: the gateway dials names a
/// rule routes to it, never addresses a client picks (spec §10.2).
fn destination(host: &str) -> Option<Name> {
    let name = Name::parse(host).ok()?;
    // No top-level domain is all digits: this refuses `10.0.0.1`, `0x7f.1` and the like.
    let tld = name.as_str().rsplit('.').next()?;
    (!tld.bytes().all(|b| b.is_ascii_digit())).then_some(name)
}

/// TLS: the SNI of the ClientHello, which may span several handshake records
/// (RFC 8446 §5.1, RFC 6066 §3).
fn server_name(head: &[u8]) -> Found {
    let mut handshake = Vec::new();
    let mut rest = head;
    loop {
        let Some((header, after)) = rest.split_at_checked(5) else {
            return Found::Partial;
        };
        if header[0] != HANDSHAKE || header[1] != 3 {
            return Found::Nothing;
        }
        let len = usize::from(u16::from_be_bytes([header[3], header[4]]));
        if len == 0 || len > 1 << 14 {
            return Found::Nothing;
        }
        let Some((payload, after)) = after.split_at_checked(len) else {
            return Found::Partial;
        };
        handshake.extend_from_slice(payload);
        rest = after;
        if let Some(found) = client_hello(&handshake) {
            return found;
        }
    }
}

/// `None` until the whole handshake message is in.
fn client_hello(handshake: &[u8]) -> Option<Found> {
    let mut r = Reader(handshake);
    if r.number(1)? != CLIENT_HELLO {
        return Some(Found::Nothing);
    }
    let body = r.vector(3)?;
    Some(sni(body).map_or(Found::Nothing, Found::Host))
}

/// The host name of the only server_name extension; `None` without one, with
/// two (illegal, RFC 8446 §4.2: refuse rather than guess), or when malformed.
fn sni(body: &[u8]) -> Option<String> {
    let mut r = Reader(body);
    r.take(2 + 32)?; // legacy_version, random
    r.vector(1)?; // legacy_session_id
    r.vector(2)?; // cipher_suites
    r.vector(1)?; // legacy_compression_methods
    let mut extensions = Reader(r.vector(2)?);
    let mut names = Vec::new();
    while !extensions.is_empty() {
        let kind = extensions.number(2)?;
        let data = extensions.vector(2)?;
        if kind == SERVER_NAME {
            names.push(host_name(data)?);
        }
    }
    let [name] = <[String; 1]>::try_from(names).ok()?;
    Some(name)
}

/// A server_name extension body with exactly one entry, of type host_name.
fn host_name(data: &[u8]) -> Option<String> {
    let mut data = Reader(data);
    let mut list = Reader(data.vector(2)?);
    let kind = list.number(1)?;
    let name = list.vector(2)?;
    if kind != HOST_NAME || !list.is_empty() || !data.is_empty() {
        return None;
    }
    String::from_utf8(name.to_vec()).ok()
}

/// Bounds-checked reading of TLS wire structures (RFC 8446 §3).
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, rest) = self.0.split_at_checked(n)?;
        self.0 = rest;
        Some(head)
    }

    /// A big-endian number of `bytes` bytes.
    fn number(&mut self, bytes: usize) -> Option<usize> {
        Some(
            self.take(bytes)?
                .iter()
                .fold(0, |n, b| n << 8 | usize::from(*b)),
        )
    }

    /// A vector with a length prefix of `bytes` bytes.
    fn vector(&mut self, bytes: usize) -> Option<&'a [u8]> {
        let len = self.number(bytes)?;
        self.take(len)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// HTTP/1.x: the `Host` header of the request head (RFC 9112 §3.2).
fn http_host(head: &[u8]) -> Found {
    let Some(end) = head.windows(4).position(|w| w == b"\r\n\r\n") else {
        return Found::Partial;
    };
    let Ok(text) = std::str::from_utf8(&head[..end]) else {
        return Found::Nothing;
    };
    let mut hosts = Vec::new();
    for line in text.split("\r\n").skip(1) {
        // A folded line (obs-fold) could carry on a Host value: refuse it.
        if line.starts_with([' ', '\t']) {
            return Found::Nothing;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("host") {
                hosts.push(value.trim());
            }
        }
    }
    // HTTP/1.0 without Host, or two Host headers: no single destination.
    match hosts[..] {
        [host] => strip_port(host).map_or(Found::Nothing, |h| Found::Host(h.into())),
        _ => Found::Nothing,
    }
}

/// `example.com:8080` → `example.com`.
fn strip_port(host: &str) -> Option<&str> {
    match host.rsplit_once(':') {
        Some((name, port)) => port.parse::<u16>().is_ok().then_some(name),
        None => Some(host),
    }
}
