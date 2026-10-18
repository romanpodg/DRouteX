use super::{SnapshotError, SnapshotErrorCode as Code};
use crate::{CheckedConfiguration, MAX_INPUT_BYTES};
use serde_json::Value;

fn invalid() -> SnapshotError {
    SnapshotError(Code::Canonical)
}

fn array<'a>(parent: &'a mut Value, field: &str) -> Result<&'a mut Vec<Value>, SnapshotError> {
    parent
        .get_mut(field)
        .and_then(Value::as_array_mut)
        .ok_or_else(invalid)
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum SortKey {
    Number(u64),
    Text(String),
}

fn rows(parent: &mut Value, field: &str, keys: &[&str]) -> Result<(), SnapshotError> {
    let array = array(parent, field)?;
    let mut keyed = Vec::with_capacity(array.len());
    for value in std::mem::take(array) {
        let mut key = Vec::with_capacity(keys.len());
        for field in keys {
            let component = value.get(*field).ok_or_else(invalid)?;
            key.push(if let Some(text) = component.as_str() {
                SortKey::Text(text.to_owned())
            } else if let Some(number) = component.as_u64() {
                SortKey::Number(number)
            } else {
                return Err(invalid());
            });
        }
        keyed.push((key, value));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    array.extend(keyed.into_iter().map(|(_, value)| value));
    Ok(())
}

fn ids(parent: &mut Value, field: &str) -> Result<(), SnapshotError> {
    let array = array(parent, field)?;
    if array.iter().any(|v| !v.is_string()) {
        return Err(invalid());
    }
    array.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    Ok(())
}

pub(super) fn configuration(checked: &CheckedConfiguration) -> Result<Value, SnapshotError> {
    // Serialize known typed fields, materializing defaults and normalized ID/IP text.
    let mut value = serde_json::to_value(checked.document()).map_err(|_| invalid())?;
    for field in [
        "profiles",
        "endpoints",
        "resolverCredentials",
        "listeners",
        "networkGroups",
        "legacyGrants",
        "gatewayGrants",
        "outbounds",
        "privateRecords",
    ] {
        rows(&mut value, field, &["id"])?;
    }
    for profile in array(&mut value, "profiles")? {
        rows(profile, "customRules", &["id"])?;
        rows(profile, "serviceRules", &["id"])?;
        rows(profile, "filterLists", &["revision"])?;
        for rule in array(profile, "customRules")? {
            rows(rule, "ports", &["from", "to"])?;
        }
    }
    for endpoint in array(&mut value, "endpoints")? {
        rows(endpoint, "access", &["transport", "listener"])?;
        for access in array(endpoint, "access")? {
            let field = if access.get("credentials").is_some() {
                "credentials"
            } else {
                "grants"
            };
            ids(access, field)?;
        }
    }
    for grant in array(&mut value, "gatewayGrants")? {
        rows(grant, "targets", &["mode", "value"])?;
        rows(grant, "ports", &["from", "to"])?;
        ids(grant, "services")?;
        ids(grant, "outbounds")?;
    }
    if serde_json::to_vec(&value).map_err(|_| invalid())?.len() > MAX_INPUT_BYTES {
        return Err(SnapshotError(Code::Size));
    }
    Ok(value)
}

/// serde_json bounds parsing recursion; independently enforce the narrower D11 tree.
pub(super) fn check_tree(value: &Value, depth: usize) -> Result<(), SnapshotError> {
    if depth > 40 {
        return Err(SnapshotError(Code::Size));
    }
    match value {
        Value::Array(values) => values.iter().try_for_each(|v| check_tree(v, depth + 1)),
        Value::Object(values) => values.values().try_for_each(|v| check_tree(v, depth + 1)),
        Value::Number(number) if number.is_f64() => Err(SnapshotError(Code::Format)),
        _ => Ok(()),
    }
}
