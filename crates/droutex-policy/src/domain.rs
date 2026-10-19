use idna::uts46::{AsciiDenyList, DnsLength, Hyphens, Uts46};
use std::{error::Error, fmt, str::FromStr};

/// Maximum UTF-8 input size, checked before IDNA processing (D37).
pub const MAX_DOMAIN_INPUT_BYTES: usize = 1024;

/// A validated lowercase ASCII DNS policy name without a final root dot.
///
/// This DNS/CDN profile permits underscores and non-strict hyphen positions.
/// It is not a URL, TLS hostname assertion, socket destination or authorization.
/// Original configuration text is retained separately in the config schema.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CanonicalDomain(String);

impl CanonicalDomain {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for CanonicalDomain {
    type Err = DomainError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.len() > MAX_DOMAIN_INPUT_BYTES {
            return Err(DomainError::InputTooLong);
        }
        let mut ascii = checked_ascii(input)?;
        if ascii.ends_with('.') {
            ascii.pop();
        }
        if !valid_ascii_name(&ascii) {
            return Err(DomainError::InvalidName);
        }
        Ok(Self(ascii))
    }
}

fn to_ascii(input: &str) -> Result<String, DomainError> {
    Uts46::new()
        .to_ascii(
            input.as_bytes(),
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::VerifyAllowRootDot,
        )
        .map(|name| name.into_owned())
        .map_err(|_| DomainError::InvalidName)
}

fn checked_ascii(input: &str) -> Result<String, DomainError> {
    let ascii = to_ascii(input)?;
    // The bare selected to_ascii accepts malformed nested ACE labels. Require
    // a roundtrip using the same qualified library; never relax to a fallback.
    let (unicode, status) =
        Uts46::new().to_unicode(ascii.as_bytes(), AsciiDenyList::URL, Hyphens::Allow);
    status.map_err(|_| DomainError::InvalidName)?;
    if to_ascii(&unicode)? != ascii {
        return Err(DomainError::InvalidName);
    }
    Ok(ascii)
}

fn valid_ascii_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name.split('.').count() <= 127
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || byte == b'-'
                        || byte == b'_'
                })
        })
}

/// Sanitized failures contain neither the input nor library diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainError {
    InputTooLong,
    InvalidName,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InputTooLong => "domain input exceeds the byte limit",
            Self::InvalidName => "invalid DNS policy name",
        })
    }
}

impl Error for DomainError {}

/// Explicit CR-0002 semantics; no wildcard/glob syntax is parsed from a name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainMatchMode {
    Exact,
    DomainAndSubdomains,
    SubdomainsOnly,
}

/// A pure matcher over validated names, with no access or routing authority.
///
/// Unvalidated text cannot be used as a query:
/// ```compile_fail
/// use droutex_policy::{CanonicalDomain, DomainMatcher, DomainMatchMode};
/// let domain: CanonicalDomain = "example.com".parse().unwrap();
/// let matcher = DomainMatcher::new(domain, DomainMatchMode::Exact);
/// matcher.matches("example.com");
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct DomainMatcher {
    domain: CanonicalDomain,
    mode: DomainMatchMode,
}

impl DomainMatcher {
    pub fn new(domain: CanonicalDomain, mode: DomainMatchMode) -> Self {
        Self { domain, mode }
    }

    pub fn domain(&self) -> &CanonicalDomain {
        &self.domain
    }

    pub fn mode(&self) -> DomainMatchMode {
        self.mode
    }

    pub fn matches(&self, query: &CanonicalDomain) -> bool {
        if query == &self.domain {
            return self.mode != DomainMatchMode::SubdomainsOnly;
        }
        self.mode != DomainMatchMode::Exact
            && query
                .as_str()
                .strip_suffix(self.domain.as_str())
                .is_some_and(|prefix| prefix.ends_with('.'))
    }
}
