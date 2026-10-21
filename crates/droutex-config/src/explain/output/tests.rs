use super::*;
use std::str::FromStr;
fn id<T: FromStr>() -> T
where
    T::Err: fmt::Debug,
{
    "10000000-0000-4000-8000-000000000001".parse().unwrap()
}
#[test]
fn every_typed_refusal_preserves_its_wire_reason_and_safe_reference() {
    let cases = [
        (ActionRefusal::PolicyDenied, "policy_denied", false),
        (
            ActionRefusal::UnknownUpstream(id()),
            "unknown_upstream",
            true,
        ),
        (
            ActionRefusal::UnknownOutbound(id()),
            "unknown_outbound",
            true,
        ),
        (
            ActionRefusal::DisabledOutbound(id()),
            "disabled_outbound",
            true,
        ),
        (
            ActionRefusal::UnavailableOutbound(id()),
            "unavailable_outbound",
            true,
        ),
        (
            ActionRefusal::UnknownAvailability(id()),
            "unknown_availability",
            true,
        ),
        (
            ActionRefusal::UnsupportedCapability(id()),
            "unsupported_capability",
            true,
        ),
        (
            ActionRefusal::UnknownCapability(id()),
            "unknown_capability",
            true,
        ),
        (
            ActionRefusal::ConnectionFailed(id()),
            "connection_failed",
            true,
        ),
        (
            ActionRefusal::UnsupportedTransport,
            "unsupported_transport",
            false,
        ),
        (ActionRefusal::GatewayNotReady, "gateway_not_ready", false),
        (
            ActionRefusal::PrivateProfileMismatch,
            "private_profile_mismatch",
            false,
        ),
        (
            ActionRefusal::PrivateNameMismatch,
            "private_name_mismatch",
            false,
        ),
    ];
    for (reason, code, reference) in cases {
        let value = serde_json::to_value(refused(reason)).unwrap();
        assert_eq!(value["kind"], "refused");
        assert_eq!(value["code"], code);
        assert_eq!(
            value["reference"],
            if reference {
                serde_json::json!("10000000-0000-4000-8000-000000000001")
            } else {
                serde_json::Value::Null
            }
        );
    }
}
