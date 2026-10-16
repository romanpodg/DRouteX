use routira_common::RuleLayer::{Custom, Default, Filter, Service};
use routira_config::{InvalidLayerOrder, validate_rule_layers};

#[test]
fn accepts_the_owner_approved_precedence() {
    assert_eq!(
        validate_rule_layers(&[Custom, Service, Filter, Default]),
        Ok(())
    );
}

#[test]
fn rejects_precedence_overrides_and_incomplete_layers() {
    let cases: &[&[_]] = &[
        &[],
        &[Service, Custom, Filter, Default],
        &[Custom, Filter, Service, Default],
        &[Custom, Service, Default, Filter],
        &[Custom, Service, Filter],
        &[Custom, Service, Filter, Filter],
        &[Custom, Service, Filter, Default, Default],
    ];
    for layers in cases {
        assert_eq!(
            validate_rule_layers(layers),
            Err(InvalidLayerOrder),
            "{layers:?}"
        );
    }
}
