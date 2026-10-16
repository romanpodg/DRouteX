use routira_common::*;
use std::net::{IpAddr, Ipv4Addr};

const V4: &str = "919108f7-52d1-4320-9bac-f847db4148a8";
const OTHER: &str = "ac2be36a-99c2-42ee-9bb3-2a2ec8147245";
const BYTES: [u8; 16] = [
    0x91, 0x91, 0x08, 0xf7, 0x52, 0xd1, 0x43, 0x20, 0x9b, 0xac, 0xf8, 0x47, 0xdb, 0x41, 0x48, 0xa8,
];

#[test]
fn resource_ids_preserve_v4_vectors_and_reject_wrong_formats() {
    macro_rules! check {
        ($($name:ident),+) => {$ (
            let id = $name::from_bytes(BYTES).unwrap();
            assert_eq!(id.to_string(), V4);
            assert_eq!(id.as_bytes(), &BYTES);
            assert_eq!(V4.parse::<$name>().unwrap(), id);
            for input in ["", "not-an-id", "00000000-0000-0000-0000-000000000000",
                "919108f7-52d1-7320-9bac-f847db4148a8",
                "919108f7-52d1-4320-1bac-f847db4148a8"] {
                assert_eq!(input.parse::<$name>(), Err(InvalidResourceId), "{}: {input}", stringify!($name));
            }
            assert_eq!($name::from_bytes([0; 16]), Err(InvalidResourceId));
        )+};
    }
    check!(
        ProfileId,
        EndpointId,
        ResolverCredentialId,
        DeviceId,
        FilterListId,
        FilterListRevisionId,
        ServiceGroupId,
        ServiceGroupRevisionId,
        PrivateDnsRecordId,
        RuleId,
        ProxyOutboundId,
        ListenerId,
        NetworkGroupId,
        LegacyNetworkGrantId,
        GatewayNetworkGrantId,
        SnapshotId,
        DeploymentId,
        SecretRefId,
        DnsUpstreamId
    );
}

#[test]
fn id_and_secret_errors_and_debug_never_echo_confidential_input() {
    let material = b"disposable-sensitive-resolver-material".to_vec();
    let secret = ResolverCapabilityMaterial::from_owned_bytes(material).unwrap();
    assert_eq!(
        format!("{secret:?}"),
        "ResolverCapabilityMaterial([REDACTED])"
    );
    let error = "disposable-sensitive-resolver-material"
        .parse::<EndpointId>()
        .unwrap_err();
    assert_eq!(error.to_string(), "expected a UUIDv4 resource identifier");
    assert_eq!(
        ResolverCapabilityMaterial::from_owned_bytes(Vec::new()).unwrap_err(),
        EmptyResolverCapability
    );
    assert_eq!(
        EmptyResolverCapability.to_string(),
        "empty resolver capability material"
    );
}

fn endpoint(id: &str, profile: &str) -> DnsEndpointAssignment {
    DnsEndpointAssignment {
        endpoint: id.parse().unwrap(),
        profile: profile.parse().unwrap(),
        state: AdministrativeState::Enabled,
        device: Some(V4.parse().unwrap()),
    }
}

fn credential(endpoint: EndpointId) -> ResolverCredentialMetadata {
    ResolverCredentialMetadata {
        id: V4.parse().unwrap(),
        endpoint,
        state: ResolverCredentialState::Active,
    }
}

#[test]
fn copied_credential_association_is_not_a_new_device_identity() {
    let assignment = endpoint(V4, OTHER);
    let metadata = credential(assignment.endpoint);
    let secret =
        ResolverCapabilityMaterial::from_owned_bytes(b"fixture bearer material".to_vec()).unwrap();
    // Copying bearer material requires explicit exposure; no implicit Clone exists.
    let copied =
        ResolverCapabilityMaterial::from_owned_bytes(secret.expose_secret().to_vec()).unwrap();
    assert_eq!(copied.expose_secret(), secret.expose_secret());
    let copied_metadata = metadata;
    let binding = assignment.binding_for_credential(&copied_metadata).unwrap();
    assert_eq!(
        binding,
        DnsProfileBinding {
            endpoint: assignment.endpoint,
            profile: assignment.profile
        }
    );
    // The binding contains no DeviceId or gateway grant; independent verification stays P3-07.
}

#[test]
fn unavailable_credentials_and_disabled_endpoints_cannot_select_a_profile() {
    let mut assignment = endpoint(V4, OTHER);
    let mut metadata = credential(assignment.endpoint);
    for state in [
        ResolverCredentialState::Disabled,
        ResolverCredentialState::Expired,
        ResolverCredentialState::Revoked,
    ] {
        metadata.state = state;
        assert_eq!(
            assignment.binding_for_credential(&metadata),
            Err(DnsBindingError::Unavailable)
        );
    }
    metadata.state = ResolverCredentialState::Active;
    assignment.state = AdministrativeState::Disabled;
    assert_eq!(
        assignment.binding_for_credential(&metadata),
        Err(DnsBindingError::Unavailable)
    );
    assert_eq!(
        DnsBindingError::Unavailable.to_string(),
        "DNS association unavailable"
    );
}

#[test]
fn secure_endpoints_sharing_a_network_keep_distinct_profile_assignments() {
    let home = endpoint(V4, V4);
    let work = endpoint(OTHER, OTHER);
    let home_credential = credential(home.endpoint);
    let work_credential = credential(work.endpoint);
    assert_eq!(
        home.binding_for_credential(&home_credential)
            .unwrap()
            .profile,
        home.profile
    );
    assert_eq!(
        work.binding_for_credential(&work_credential)
            .unwrap()
            .profile,
        work.profile
    );
    assert_eq!(
        home.binding_for_credential(&work_credential),
        Err(DnsBindingError::WrongEndpoint)
    );
    assert_eq!(
        work.binding_for_credential(&home_credential),
        Err(DnsBindingError::WrongEndpoint)
    );
    assert_eq!(
        DnsBindingError::WrongEndpoint.to_string(),
        "credential Endpoint mismatch"
    );
}

fn legacy(endpoint: EndpointId) -> LegacyDnsAssignment {
    LegacyDnsAssignment {
        grant: V4.parse().unwrap(),
        group: V4.parse().unwrap(),
        listener: V4.parse().unwrap(),
        source: IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)),
        endpoint,
    }
}

#[test]
fn ambiguous_legacy_source_listener_fails_regardless_of_input_order() {
    let first = legacy(V4.parse().unwrap());
    let second = legacy(OTHER.parse().unwrap());
    for assignments in [[first, second], [second, first], [first, first]] {
        assert_eq!(
            unique_legacy_assignment(&assignments, first.listener, first.source),
            Err(AmbiguousLegacyAssignment)
        );
    }
    assert_eq!(
        AmbiguousLegacyAssignment.to_string(),
        "ambiguous legacy DNS assignment"
    );
}

#[test]
fn legacy_matching_uses_exact_peer_and_listener_without_ipv6_prefix_expansion() {
    let first = legacy(V4.parse().unwrap());
    let mut other_listener = legacy(OTHER.parse().unwrap());
    other_listener.listener = OTHER.parse().unwrap();
    let assignments = [first, other_listener];
    assert_eq!(
        unique_legacy_assignment(&assignments, first.listener, first.source),
        Ok(Some(&assignments[0]))
    );
    assert_eq!(
        unique_legacy_assignment(&assignments, other_listener.listener, first.source),
        Ok(Some(&assignments[1]))
    );
    assert_eq!(
        unique_legacy_assignment(&[], first.listener, first.source),
        Ok(None)
    );
    let mut ipv6 = first;
    ipv6.source = "2001:db8::1".parse().unwrap();
    assert_eq!(
        unique_legacy_assignment(&[ipv6], ipv6.listener, "2001:db8::2".parse().unwrap()),
        Ok(None)
    );
    assert_eq!(
        unique_legacy_assignment(&[ipv6], ipv6.listener, first.source),
        Ok(None)
    );
}
