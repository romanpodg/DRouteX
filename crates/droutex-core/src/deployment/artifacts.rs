use super::{DeploymentError as Error, ErrorCode, MAX_RESOURCE_BYTES};
use droutex_config::snapshot::{
    MAX_RESOURCE_PINS, ResourcePin, SnapshotResource, TrustedResources,
};
use droutex_security::SecretPurpose;
use std::collections::BTreeMap;

/// Actual public/encrypted bytes. No supplied digest is trusted instead of these bytes.
pub struct Artifacts {
    pub(super) entries: BTreeMap<String, (ResourcePin, Vec<u8>)>,
}
impl Artifacts {
    pub(super) fn data(&self) -> Vec<droutex_config::ResourceData<'_>> {
        // The owning Artifacts constructor already bounded count/total bytes.
        self.entries
            .values()
            .map(|(pin, bytes)| droutex_config::ResourceData {
                resource: pin.resource(),
                bytes,
            })
            .collect()
    }
    pub fn new(entries: Vec<(SnapshotResource, Vec<u8>)>) -> Result<Self, Error> {
        if entries.len() > MAX_RESOURCE_PINS {
            return Err(Error(ErrorCode::Size));
        }
        let mut result = BTreeMap::new();
        let mut total = 0_usize;
        for (resource, bytes) in entries {
            total = total
                .checked_add(bytes.len())
                .ok_or(Error(ErrorCode::Size))?;
            if total > MAX_RESOURCE_BYTES {
                return Err(Error(ErrorCode::Size));
            }
            let pin = ResourcePin::from_bytes(resource, &bytes).map_err(Error::snapshot)?;
            if result.insert(filename(resource), (pin, bytes)).is_some() {
                return Err(Error(ErrorCode::Resource));
            }
        }
        let artifacts = Self { entries: result };
        artifacts.trusted()?; // Also refuses the same secret identity with different purposes.
        Ok(artifacts)
    }
    pub(super) fn trusted(&self) -> Result<TrustedResources, Error> {
        TrustedResources::new(self.entries.values().map(|(pin, _)| *pin).collect())
            .map_err(Error::snapshot)
    }
    pub(super) fn check_exact(&self, pins: &[ResourcePin]) -> Result<(), Error> {
        if pins.len() != self.entries.len()
            || pins.iter().any(|pin| {
                self.entries
                    .get(&filename(pin.resource()))
                    .is_none_or(|(actual, _)| actual != pin)
            })
        {
            return Err(Error(ErrorCode::Resource));
        }
        Ok(())
    }
    pub(super) fn get(&self, resource: SnapshotResource) -> Option<&[u8]> {
        self.entries
            .get(&filename(resource))
            .filter(|(pin, _)| pin.resource() == resource)
            .map(|(_, bytes)| bytes.as_slice())
    }
}

pub(super) fn filename(resource: SnapshotResource) -> String {
    match resource {
        SnapshotResource::Device(id) => format!("d-{id}"),
        SnapshotResource::Upstream(id) => format!("u-{id}"),
        SnapshotResource::Filter(id) => format!("f-{id}"),
        SnapshotResource::Service(id) => format!("v-{id}"),
        SnapshotResource::Secret { reference, purpose } => format!(
            "{}-{reference}",
            match purpose {
                SecretPurpose::ResolverVerifier => "r",
                SecretPurpose::ProxyOutboundCredential => "p",
            }
        ),
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) fn parse_filename(name: &str) -> Result<SnapshotResource, Error> {
    let (kind, id) = name.split_once('-').ok_or(Error(ErrorCode::Resource))?;
    let invalid = |_| Error(ErrorCode::Resource);
    let resource = match kind {
        "d" => SnapshotResource::Device(id.parse().map_err(invalid)?),
        "u" => SnapshotResource::Upstream(id.parse().map_err(invalid)?),
        "f" => SnapshotResource::Filter(id.parse().map_err(invalid)?),
        "v" => SnapshotResource::Service(id.parse().map_err(invalid)?),
        "r" | "p" => SnapshotResource::Secret {
            reference: id.parse().map_err(invalid)?,
            purpose: if kind == "r" {
                SecretPurpose::ResolverVerifier
            } else {
                SecretPurpose::ProxyOutboundCredential
            },
        },
        _ => return Err(Error(ErrorCode::Resource)),
    };
    if filename(resource) != name {
        return Err(Error(ErrorCode::Resource));
    }
    Ok(resource)
}
