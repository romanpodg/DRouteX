//! D41 offline list parsing. Conditions are data, never an action or permission.
use crate::{DomainMatchMode, DomainMatcher, MAX_POLICY_ENTRIES};
use droutex_common::MAX_LOCAL_INPUT_BYTES;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlocklistFormat {
    Domains,
    BlockingHosts,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogParseCode {
    Size,
    Utf8,
    Format,
    Domain,
    EntryLimit,
    Empty,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogParseError {
    pub code: CatalogParseCode,
    pub line: Option<usize>,
}
impl fmt::Display for CatalogParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid local catalog input")
    }
}
impl Error for CatalogParseError {}
fn error(code: CatalogParseCode, line: Option<usize>) -> CatalogParseError {
    CatalogParseError { code, line }
}

/// Canonical sorted/deduplicated conditions; raw duplicates still consume budget.
pub struct CatalogConditions {
    entries: Vec<DomainMatcher>,
    raw_entries: usize,
}
impl CatalogConditions {
    pub fn entries(&self) -> &[DomainMatcher] {
        &self.entries
    }
    pub fn raw_entries(&self) -> usize {
        self.raw_entries
    }
    pub fn into_entries(self) -> Vec<DomainMatcher> {
        self.entries
    }
}
impl fmt::Debug for CatalogConditions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CatalogConditions")
            .field("raw_entries", &self.raw_entries)
            .field("canonical_entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}

/// Preflight across a batch, before any normalization/collection.
pub fn blocklist_entry_count(
    input: &[u8],
    format: BlocklistFormat,
) -> Result<usize, CatalogParseError> {
    let mut count = 0usize;
    visit_domains(input, format, |_, line| {
        count += 1;
        if count > MAX_POLICY_ENTRIES {
            return Err(error(CatalogParseCode::EntryLimit, Some(line)));
        }
        Ok(())
    })?;
    if count == 0 {
        return Err(error(CatalogParseCode::Empty, None));
    }
    Ok(count)
}

/// Explicit mode is required; hosts addresses are discarded, never DNS answers.
pub fn parse_blocklist(
    input: &[u8],
    format: BlocklistFormat,
    mode: DomainMatchMode,
) -> Result<CatalogConditions, CatalogParseError> {
    let count = blocklist_entry_count(input, format)?;
    let mut entries = Vec::with_capacity(count);
    visit_domains(input, format, |name, line| {
        let domain = name
            .parse()
            .map_err(|_| error(CatalogParseCode::Domain, Some(line)))?;
        entries.push(DomainMatcher::new(domain, mode));
        Ok(())
    })?;
    normalize_catalog_conditions(entries)
}

/// Parsed D37 conditions only; this is not license/dependency verification.
pub fn normalize_catalog_conditions(
    mut entries: Vec<DomainMatcher>,
) -> Result<CatalogConditions, CatalogParseError> {
    let raw_entries = entries.len();
    if raw_entries > MAX_POLICY_ENTRIES {
        return Err(error(CatalogParseCode::EntryLimit, None));
    }
    if entries.is_empty() {
        return Err(error(CatalogParseCode::Empty, None));
    }
    entries.sort_unstable_by(|a, b| {
        a.domain()
            .cmp(b.domain())
            .then_with(|| mode_key(a.mode()).cmp(&mode_key(b.mode())))
    });
    entries.dedup();
    Ok(CatalogConditions {
        entries,
        raw_entries,
    })
}
fn mode_key(mode: DomainMatchMode) -> u8 {
    match mode {
        DomainMatchMode::Exact => 0,
        DomainMatchMode::DomainAndSubdomains => 1,
        DomainMatchMode::SubdomainsOnly => 2,
    }
}

fn visit_domains(
    input: &[u8],
    format: BlocklistFormat,
    mut visit: impl FnMut(&str, usize) -> Result<(), CatalogParseError>,
) -> Result<(), CatalogParseError> {
    if input.len() > MAX_LOCAL_INPUT_BYTES {
        return Err(error(CatalogParseCode::Size, None));
    }
    let input = std::str::from_utf8(input).map_err(|_| error(CatalogParseCode::Utf8, None))?;
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut characters = input.chars().peekable();
    while let Some(c) = characters.next() {
        if c == '\u{feff}'
            || (c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
            || (c == '\r' && characters.peek() != Some(&'\n'))
        {
            return Err(error(CatalogParseCode::Format, None));
        }
    }
    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = line.split_once('#').map_or(line, |(data, _)| data);
        let mut tokens = line.split([' ', '\t']).filter(|s| !s.is_empty());
        let Some(first) = tokens.next() else { continue };
        match format {
            BlocklistFormat::Domains => {
                if tokens.next().is_some() {
                    return Err(error(CatalogParseCode::Format, Some(line_number)));
                }
                visit(first, line_number)?;
            }
            BlocklistFormat::BlockingHosts => {
                if !matches!(first, "0.0.0.0" | "127.0.0.1" | "::" | "::1") {
                    return Err(error(CatalogParseCode::Format, Some(line_number)));
                }
                let mut found = false;
                for alias in tokens {
                    found = true;
                    visit(alias, line_number)?;
                }
                if !found {
                    return Err(error(CatalogParseCode::Format, Some(line_number)));
                }
            }
        }
    }
    Ok(())
}
