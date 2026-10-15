//! Domain patterns (spec §8.1) and their specificity (spec §8.2.1, step 1).

use crate::name::{Name, NameError};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pattern {
    /// `example.com`: only that name.
    Exact(Name),
    /// `*.example.com`: any subdomain, not the apex.
    Wildcard(Name),
    /// `suffix:example.com`: the apex and any subdomain.
    Suffix(Name),
}

/// Greater is more specific: an exact name beats any wildcard or suffix,
/// then a pattern with more labels beats one with fewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity {
    pub exact: bool,
    pub labels: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    #[error("'*' is only allowed as a leading '*.' label")]
    BadWildcard,
    #[error(transparent)]
    Name(#[from] NameError),
}

impl Pattern {
    pub fn parse(s: &str) -> Result<Pattern, PatternError> {
        let (make, rest): (fn(Name) -> Pattern, &str) =
            if let Some(rest) = s.strip_prefix("suffix:") {
                (Pattern::Suffix, rest)
            } else if let Some(rest) = s.strip_prefix("*.") {
                (Pattern::Wildcard, rest)
            } else {
                (Pattern::Exact, s)
            };
        if rest.contains('*') {
            return Err(PatternError::BadWildcard);
        }
        Ok(make(Name::parse(rest)?))
    }

    pub fn matches(&self, name: &Name) -> bool {
        match self {
            Pattern::Exact(n) => name == n,
            Pattern::Wildcard(base) => name.is_strict_subdomain_of(base),
            Pattern::Suffix(base) => name == base || name.is_strict_subdomain_of(base),
        }
    }

    pub fn specificity(&self) -> Specificity {
        match self {
            Pattern::Exact(n) => Specificity {
                exact: true,
                labels: n.label_count(),
            },
            Pattern::Wildcard(base) | Pattern::Suffix(base) => Specificity {
                exact: false,
                labels: base.label_count(),
            },
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Exact(n) => write!(f, "{n}"),
            Pattern::Wildcard(n) => write!(f, "*.{n}"),
            Pattern::Suffix(n) => write!(f, "suffix:{n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Pattern {
        Pattern::parse(s).unwrap()
    }

    fn n(s: &str) -> Name {
        Name::parse(s).unwrap()
    }

    #[test]
    fn parses_three_forms_and_displays_them_back() {
        for s in ["media.example", "*.media.example", "suffix:media.example"] {
            assert_eq!(p(s).to_string(), s);
        }
    }

    #[test]
    fn normalizes_the_name_part() {
        assert_eq!(
            p("suffix:Media.Example.").to_string(),
            "suffix:media.example"
        );
        assert_eq!(p("*.пример.рф").to_string(), "*.xn--e1afmkfd.xn--p1ai");
    }

    #[test]
    fn rejects_misplaced_wildcards() {
        for s in [
            "*",
            "a*.example",
            "*.*.example",
            "suffix:*.example",
            "media.*",
        ] {
            assert_eq!(Pattern::parse(s), Err(PatternError::BadWildcard), "{s}");
        }
    }

    #[test]
    fn reports_bad_names() {
        assert_eq!(
            Pattern::parse("suffix:"),
            Err(PatternError::Name(NameError::Empty))
        );
    }

    #[test]
    fn exact_matches_only_itself() {
        assert!(p("media.example").matches(&n("media.example")));
        assert!(!p("media.example").matches(&n("a.media.example")));
    }

    #[test]
    fn wildcard_excludes_apex() {
        let w = p("*.media.example");
        assert!(w.matches(&n("a.media.example")));
        assert!(w.matches(&n("a.b.media.example")));
        assert!(!w.matches(&n("media.example")));
        assert!(!w.matches(&n("xmedia.example")));
    }

    #[test]
    fn suffix_includes_apex() {
        let s = p("suffix:media.example");
        assert!(s.matches(&n("media.example")));
        assert!(s.matches(&n("a.b.media.example")));
        assert!(!s.matches(&n("xmedia.example")));
    }

    #[test]
    fn specificity_order() {
        let s = |x: &str| p(x).specificity();
        assert!(s("login.media.example") > s("suffix:login.media.example"));
        assert!(s("suffix:login.media.example") > s("suffix:media.example"));
        assert_eq!(s("*.media.example"), s("suffix:media.example"));
    }
}
