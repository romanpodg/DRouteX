use super::{CatalogFormat, CatalogImportError as Error, PreparedRevision};
use crate::schema::BoundedValue;
use droutex_policy::{
    CatalogConditions, CatalogPin, CatalogRevision, DomainMatchMode, DomainMatcher,
    normalize_catalog_conditions,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bundle {
    schema_version: u32,
    kind: String,
    entries: Vec<Condition>,
    dependencies: Vec<Pin>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Condition {
    domain: String,
    mode: Mode,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Exact,
    DomainAndSubdomains,
    SubdomainsOnly,
}
impl Mode {
    fn value(self) -> DomainMatchMode {
        match self {
            Self::Exact => DomainMatchMode::Exact,
            Self::DomainAndSubdomains => DomainMatchMode::DomainAndSubdomains,
            Self::SubdomainsOnly => DomainMatchMode::SubdomainsOnly,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    kind: Kind,
    revision: String,
    hash: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Filter,
    Service,
}

pub(super) fn json_bytes(input: &[u8]) -> &[u8] {
    input.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(input)
}
pub(super) fn service(input: &[u8]) -> Result<(CatalogConditions, Vec<CatalogPin>), Error> {
    let value: BoundedValue =
        serde_json::from_slice(json_bytes(input)).map_err(|e| Error::Document {
            line: e.line(),
            column: e.column(),
        })?;
    let bundle: Bundle = serde_path_to_error::deserialize(value).map_err(|error| {
        let mut path = error.path().iter();
        match (path.next(), path.next()) {
            (
                Some(serde_path_to_error::Segment::Map { key }),
                Some(serde_path_to_error::Segment::Seq { index }),
            ) if key == "entries" => Error::ServiceEntry { index: *index },
            (
                Some(serde_path_to_error::Segment::Map { key }),
                Some(serde_path_to_error::Segment::Seq { index }),
            ) if key == "dependencies" => Error::ServiceDependency { index: *index },
            _ => Error::Shape,
        }
    })?;
    if bundle.schema_version != 1 || bundle.kind != "service" {
        return Err(Error::Shape);
    }
    let entries = bundle
        .entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            if entry.domain.contains('\u{feff}') {
                return Err(Error::ServiceEntry { index });
            }
            let domain = entry
                .domain
                .parse()
                .map_err(|_| Error::ServiceEntry { index })?;
            Ok(DomainMatcher::new(domain, entry.mode.value()))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let conditions = normalize_catalog_conditions(entries).map_err(Error::Input)?;
    let mut dependencies = bundle
        .dependencies
        .into_iter()
        .enumerate()
        .map(|(index, pin)| {
            let revision = match pin.kind {
                Kind::Filter => CatalogRevision::Filter(
                    pin.revision
                        .parse()
                        .map_err(|_| Error::ServiceDependency { index })?,
                ),
                Kind::Service => CatalogRevision::Service(
                    pin.revision
                        .parse()
                        .map_err(|_| Error::ServiceDependency { index })?,
                ),
            };
            Ok(CatalogPin {
                revision,
                hash: decode_hash(pin.hash.as_bytes())
                    .map_err(|_| Error::ServiceDependency { index })?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    dependencies.sort_unstable();
    Ok((conditions, dependencies))
}
pub(super) fn decode_hash(bytes: &[u8]) -> Result<[u8; 32], Error> {
    if bytes.len() != 64 {
        return Err(Error::Shape);
    }
    let mut hash = [0; 32];
    for (result, pair) in hash.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *result = nibble(pair[0])? * 16 + nibble(pair[1])?;
    }
    Ok(hash)
}
fn nibble(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::Shape),
    }
}
fn identity(revision: CatalogRevision) -> (&'static str, String) {
    match revision {
        CatalogRevision::Filter(id) => ("filter", id.to_string()),
        CatalogRevision::Service(id) => ("service", id.to_string()),
    }
}
fn mode(mode: DomainMatchMode) -> &'static str {
    match mode {
        DomainMatchMode::Exact => "exact",
        DomainMatchMode::DomainAndSubdomains => "domain_and_subdomains",
        DomainMatchMode::SubdomainsOnly => "subdomains_only",
    }
}
#[derive(Serialize)]
struct EncodedCondition<'a> {
    domain: &'a str,
    mode: &'static str,
}
#[derive(Serialize)]
struct EncodedPin {
    kind: &'static str,
    revision: String,
    hash: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Resource<'a> {
    schema_version: u32,
    kind: &'static str,
    revision: String,
    format: &'static str,
    mode: Option<&'static str>,
    metadata: &'a super::CatalogMetadata,
    source_hash: String,
    approval_evidence: &'a str,
    entries: Vec<EncodedCondition<'a>>,
    dependencies: Vec<EncodedPin>,
}

/// Fixed struct field order plus sorted data is versioned canonical resource v1.
/// Output is capped BEFORE extending its allocation, including JSON escaping.
pub(super) fn resource(revision: &PreparedRevision, remaining: usize) -> Result<Vec<u8>, Error> {
    let (kind, id) = identity(revision.revision);
    let (format, explicit_mode) = match revision.format {
        CatalogFormat::Domains(m) => ("domains", Some(mode(m))),
        CatalogFormat::BlockingHosts(m) => ("blocking_hosts", Some(mode(m))),
        CatalogFormat::ServiceJson => ("service_json", None),
    };
    let data = Resource {
        schema_version: 1,
        kind,
        revision: id,
        format,
        mode: explicit_mode,
        metadata: &revision.metadata,
        source_hash: revision.source_hash.to_string(),
        approval_evidence: &revision.approval_evidence,
        entries: revision
            .conditions
            .entries()
            .iter()
            .map(|c| EncodedCondition {
                domain: c.domain().as_str(),
                mode: mode(c.mode()),
            })
            .collect(),
        dependencies: revision
            .dependencies
            .iter()
            .map(|pin| {
                let (kind, revision) = identity(pin.revision);
                EncodedPin {
                    kind,
                    revision,
                    hash: pin.hash.iter().map(|b| format!("{b:02x}")).collect(),
                }
            })
            .collect(),
    };
    canonical_json(&data, remaining)
}

pub(super) fn canonical_json<T: Serialize>(data: &T, remaining: usize) -> Result<Vec<u8>, Error> {
    let mut output = BoundedOutput {
        bytes: Vec::new(),
        remaining,
    };
    serde_json::to_writer(&mut output, data).map_err(|e| {
        if e.is_io() {
            Error::Size
        } else {
            Error::Encoding
        }
    })?;
    Ok(output.bytes)
}
struct BoundedOutput {
    bytes: Vec<u8>,
    remaining: usize,
}
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| io::Error::other("catalog resource byte limit"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_writer_refuses_before_appending_a_partial_escaped_resource() {
        let mut output = BoundedOutput {
            bytes: Vec::new(),
            remaining: 4,
        };
        output.write_all(b"abcd").unwrap();
        assert!(output.write_all(b"ef").is_err());
        assert_eq!(output.bytes, b"abcd");
        // A short input with quotes expands during JSON serialization.
        let mut output = BoundedOutput {
            bytes: Vec::new(),
            remaining: 4,
        };
        assert!(serde_json::to_writer(&mut output, "\"\"").is_err());
        assert!(output.bytes.len() <= 4);
    }
}
