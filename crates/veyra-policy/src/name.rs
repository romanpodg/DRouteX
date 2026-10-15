//! Normalized domain names (spec §8.1): lowercase, IDNA/punycode, no trailing dot.

use std::fmt;

/// A normalized domain name: ASCII lowercase, punycode for IDN labels, no trailing dot.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("empty name")]
    Empty,
    #[error("not a valid internationalized domain name")]
    Idna,
    #[error("empty label")]
    EmptyLabel,
    #[error("label longer than 63 characters")]
    LabelTooLong,
    #[error("name longer than 253 characters")]
    NameTooLong,
    #[error("invalid character {0:?}")]
    InvalidChar(char),
    #[error("label starts or ends with '-'")]
    Hyphen,
}

impl Name {
    pub fn parse(input: &str) -> Result<Name, NameError> {
        let trimmed = input.strip_suffix('.').unwrap_or(input);
        if trimmed.is_empty() {
            return Err(NameError::Empty);
        }
        // idna lowercases and punycodes but accepts empty labels, spaces and
        // over-long labels, so the DNS rules are checked below.
        let ascii = idna::domain_to_ascii(trimmed).map_err(|_| NameError::Idna)?;
        if ascii.len() > 253 {
            return Err(NameError::NameTooLong);
        }
        for label in ascii.split('.') {
            if label.is_empty() {
                return Err(NameError::EmptyLabel);
            }
            if label.len() > 63 {
                return Err(NameError::LabelTooLong);
            }
            let bad = label.chars().find(|c| {
                !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_')
            });
            if let Some(c) = bad {
                return Err(NameError::InvalidChar(c));
            }
            if label.starts_with('-') || label.ends_with('-') {
                return Err(NameError::Hyphen);
            }
        }
        Ok(Name(ascii))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn label_count(&self) -> usize {
        self.0.split('.').count()
    }

    /// `a.example.com` is under `example.com`; a name is not under itself.
    pub fn is_strict_subdomain_of(&self, parent: &Name) -> bool {
        let (s, p) = (self.0.as_bytes(), parent.0.as_bytes());
        s.len() > p.len() && s.ends_with(p) && s[s.len() - p.len() - 1] == b'.'
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Name {
        Name::parse(s).unwrap()
    }

    #[test]
    fn lowercases_and_strips_one_trailing_dot() {
        assert_eq!(n("MEDIA.Example.").as_str(), "media.example");
    }

    #[test]
    fn converts_idn_to_punycode() {
        assert_eq!(n("пример.рф").as_str(), "xn--e1afmkfd.xn--p1ai");
        assert_eq!(n("xn--e1afmkfd.xn--p1ai"), n("ПРИМЕР.РФ"));
    }

    #[test]
    fn allows_underscore_labels() {
        assert_eq!(n("_dmarc.example.com").as_str(), "_dmarc.example.com");
    }

    #[test]
    fn rejects_malformed_names() {
        assert_eq!(Name::parse(""), Err(NameError::Empty));
        assert_eq!(Name::parse("."), Err(NameError::Empty));
        assert_eq!(Name::parse("a..b"), Err(NameError::EmptyLabel));
        assert_eq!(Name::parse("example.com.."), Err(NameError::EmptyLabel));
        assert_eq!(Name::parse("a b.com"), Err(NameError::InvalidChar(' ')));
        assert_eq!(Name::parse("-bad.com"), Err(NameError::Hyphen));
        let long_label = format!("{}.com", "a".repeat(64));
        assert_eq!(Name::parse(&long_label), Err(NameError::LabelTooLong));
        let long_name = vec!["a".repeat(63); 4].join(".");
        assert_eq!(Name::parse(&long_name), Err(NameError::NameTooLong));
    }

    #[test]
    fn strict_subdomain() {
        assert!(n("a.example.com").is_strict_subdomain_of(&n("example.com")));
        assert!(n("a.b.example.com").is_strict_subdomain_of(&n("example.com")));
        assert!(!n("example.com").is_strict_subdomain_of(&n("example.com")));
        assert!(!n("badexample.com").is_strict_subdomain_of(&n("example.com")));
    }

    #[test]
    fn counts_labels() {
        assert_eq!(n("login.media.example").label_count(), 3);
    }
}
