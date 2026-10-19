use super::{
    ConfigError, ConfigErrorCode, Configuration, ReferenceCatalog, error::safe_path, validation,
    value::BoundedValue,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputFormat {
    Json,
    Yaml,
    Toml,
}

/// A checked import boundary, never compiled policy, a snapshot or admission proof.
#[derive(Debug)]
pub struct CheckedConfiguration(Configuration);

impl CheckedConfiguration {
    pub fn document(&self) -> &Configuration {
        &self.0
    }

    pub(crate) fn revalidate(&self, catalog: &ReferenceCatalog) -> Result<(), ConfigError> {
        validation::validate(&self.0, catalog)
    }
}

pub fn parse_configuration(
    input: &[u8],
    format: InputFormat,
    catalog: &ReferenceCatalog,
) -> Result<CheckedConfiguration, ConfigError> {
    if input.len() > super::MAX_INPUT_BYTES {
        return Err(ConfigError::at(
            ConfigErrorCode::InputTooLarge,
            "$".to_owned(),
        ));
    }
    let input = std::str::from_utf8(input)
        .map_err(|_| ConfigError::at(ConfigErrorCode::InvalidUtf8, "$".to_owned()))?;
    let value = parse_value(input, format)?;
    let document: Configuration = serde_path_to_error::deserialize(value)
        .map_err(|error| ConfigError::at(ConfigErrorCode::InvalidShape, safe_path(error.path())))?;
    validation::validate(&document, catalog)?;
    Ok(CheckedConfiguration(document))
}

fn parse_value(input: &str, format: InputFormat) -> Result<BoundedValue, ConfigError> {
    match format {
        InputFormat::Json => serde_json::from_str(input)
            .map_err(|error| ConfigError::document(Some((error.line(), error.column())))),
        InputFormat::Toml => toml::from_str(input).map_err(|error| {
            ConfigError::document(error.span().map(|span| position(input, span.start)))
        }),
        InputFormat::Yaml => {
            serde_saphyr::from_str_with_options(input, yaml_options()).map_err(|error| {
                ConfigError::document(error.location().and_then(|p| {
                    Some((
                        usize::try_from(p.line()).ok()?,
                        usize::try_from(p.column()).ok()?,
                    ))
                }))
            })
        }
    }
}

fn position(input: &str, offset: usize) -> (usize, usize) {
    input
        .bytes()
        .take(offset)
        .fold((1, 1), |(line, col), byte| {
            if byte == b'\n' {
                (line + 1, 1)
            } else {
                (line, col + 1)
            }
        })
}

fn yaml_options() -> serde_saphyr::Options {
    serde_saphyr::options! {
        with_snippet: false,
        reject_unsupported_tags: true,
        strict_booleans: true,
        duplicate_keys: serde_saphyr::DuplicateKeyPolicy::Error,
        merge_keys: serde_saphyr::MergeKeyPolicy::Error,
        budget: serde_saphyr::budget! {
            max_reader_input_bytes: Some(super::MAX_INPUT_BYTES),
            max_depth: super::MAX_DEPTH,
            flow_nesting_limit: super::MAX_DEPTH,
            max_documents: 1,
            max_aliases: 0,
            max_anchors: 0,
            max_recorded_anchor_events: 0,
            max_recorded_anchor_bytes: 0,
            max_inclusion_depth: 0,
            max_merge_keys: 0,
            // These counters are derived from the byte cap, not extra product budgets.
            max_nodes: super::MAX_INPUT_BYTES,
            max_events: super::MAX_INPUT_BYTES * 4,
            max_total_scalar_bytes: super::MAX_INPUT_BYTES,
        },
    }
}
