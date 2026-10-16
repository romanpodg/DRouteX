/// Administrative intent, separate from observed runtime availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdministrativeState {
    /// Explicitly enabled by configuration.
    Enabled,
    /// Disabled; no new policy selection is intended.
    Disabled,
}

/// Credential lifecycle observed by the owner of the verifier/clock policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolverCredentialState {
    /// May select its Endpoint only after independent capability verification.
    Active,
    /// Administratively disabled.
    Disabled,
    /// Expiry has been established by the owning runtime's clock policy.
    Expired,
    /// Revoked authority must not be restored through an old snapshot.
    Revoked,
}

/// Native MVP transport kind; unsupported engine/tunnel modes have no variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProxyOutboundKind {
    /// Direct at the gateway VPS, not direct from the original client.
    GatewayDirect,
    /// Owner-selected HTTP CONNECT inside a verified TLS connection.
    HttpConnectTls,
}

/// Observation, never a replacement for current connection checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboundAvailability {
    /// No successful evidence exists; no available capability may be inferred.
    Unknown,
    /// A check observed availability at its own time/generation.
    Available,
    /// The outbound was unavailable; strict proxy routes stay closed.
    Unavailable,
}

/// Control-plane revision lifecycle; publication is distinct from application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionState {
    /// Editable, not a runtime snapshot.
    Draft,
    /// Validation passed for this revision.
    Validated,
    /// Published for snapshot compilation/application.
    Published,
}

/// Snapshot application observation; authentication/sequence checks stay P1-05/06.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotState {
    /// Candidate staged for validation/application, not active authority.
    Staged,
    /// The owning coordinator has confirmed application.
    Applied,
    /// Validation/application failed; last-known-good remains separately tracked.
    Rejected,
}
