use super::*;
use clock::{Clock, Reading};

fn fake_clock(wall: u64) -> Clock {
    let mut clock = Clock::new(None);
    clock.fake = Some(Reading {
        wall: Duration::from_secs(wall),
        elapsed: Duration::ZERO,
    });
    clock.reanchor(1).unwrap();
    clock
}

#[test]
fn clock_discontinuities_latch_and_repair_cannot_lower_durable_floor() {
    let mut clock = fake_clock(100);
    let interval = clock.sample().unwrap();
    assert_eq!(interval.lower, Duration::from_secs(98));
    assert_eq!(interval.upper, Duration::from_secs(102));
    assert_eq!(interval.floor().unwrap(), 105);
    clock.fake = Some(Reading {
        wall: Duration::from_secs(110),
        elapsed: Duration::from_secs(1),
    });
    assert_eq!(clock.sample().err().unwrap().code(), AdmissionCode::Clock);
    clock.fake = Some(Reading {
        wall: Duration::from_secs(101),
        elapsed: Duration::from_secs(1),
    });
    assert_eq!(clock.sample().err().unwrap().code(), AdmissionCode::Clock);
    assert_eq!(
        clock.reanchor(105).err().unwrap().code(),
        AdmissionCode::RestartWait
    );
    clock.fake = Some(Reading {
        wall: Duration::from_secs(107),
        elapsed: Duration::from_secs(7),
    });
    clock.reanchor(105).unwrap();
    assert!(clock.sample().is_ok());
}

#[test]
fn restart_wait_is_retryable_but_backward_and_monotonic_failure_are_not() {
    let mut clock = fake_clock(100);
    clock.reanchor(98).unwrap();
    clock.fake = Some(Reading {
        wall: Duration::from_secs(99),
        elapsed: Duration::ZERO,
    });
    assert_eq!(
        clock.sample().err().unwrap().code(),
        AdmissionCode::RestartWait
    );
    clock.fake = Some(Reading {
        wall: Duration::from_secs(101),
        elapsed: Duration::from_secs(1),
    });
    assert!(clock.sample().is_ok());
    clock.fake = Some(Reading {
        wall: Duration::from_secs(90),
        elapsed: Duration::from_secs(2),
    });
    assert_eq!(clock.sample().err().unwrap().code(), AdmissionCode::Clock);
}

#[test]
fn global_and_gateway_concurrency_ceilings_are_independent_of_group_aliases() {
    use droutex_common::NetworkGroupId;
    let ids: BTreeSet<_> = (1..=32)
        .map(|n| {
            format!("10000000-0000-4000-8000-{n:012x}")
                .parse::<NetworkGroupId>()
                .unwrap()
        })
        .collect();
    let mut quotas = quota::Quotas::new();
    quotas.reconcile(&ids, Duration::ZERO);
    let first = *ids.first().unwrap();
    for second in 1..=16 {
        for id in &ids {
            for _ in 0..2 {
                quotas
                    .acquire(*id, Plane::Gateway, Duration::from_secs(second))
                    .unwrap();
            }
        }
    }
    assert_eq!(
        quotas
            .acquire(first, Plane::Gateway, Duration::from_secs(17))
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    quotas.release(first, Plane::Gateway);
    assert!(
        quotas
            .acquire(first, Plane::Gateway, Duration::from_secs(17))
            .is_ok()
    );
    for id in ids.iter().take(8) {
        for _ in 0..64 {
            quotas
                .acquire(*id, Plane::Dns, Duration::from_secs(17))
                .unwrap();
        }
    }
    assert_eq!(
        quotas
            .acquire(*ids.last().unwrap(), Plane::Dns, Duration::from_secs(17))
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
}

#[test]
fn clock_rejects_epoch_boundaries_and_regressing_monotonic_time() {
    for wall in [0, droutex_config::MAX_GRANT_TIMESTAMP] {
        let mut clock = Clock::new(None);
        clock.fake_start(wall);
        assert_eq!(clock.sample().err().unwrap().code(), AdmissionCode::Clock);
    }
    let mut clock = fake_clock(100);
    clock.fake = Some(Reading {
        wall: Duration::from_secs(101),
        elapsed: Duration::from_secs(1),
    });
    clock.reanchor(1).unwrap();
    clock.fake = Some(Reading {
        wall: Duration::from_secs(100),
        elapsed: Duration::ZERO,
    });
    assert_eq!(clock.sample().err().unwrap().code(), AdmissionCode::Clock);
}

#[test]
fn gateway_group_concurrency_and_counter_inventory_keep_their_own_limits() {
    let group = "10000000-0000-4000-8000-000000000001".parse().unwrap();
    let mut quotas = quota::Quotas::new();
    quotas.reconcile(&[group].into(), Duration::ZERO);
    for _ in 0..40 {
        quotas
            .acquire(group, Plane::Gateway, Duration::from_secs(2))
            .unwrap();
    }
    for _ in 0..20 {
        quotas
            .acquire(group, Plane::Gateway, Duration::from_secs(3))
            .unwrap();
    }
    for _ in 0..4 {
        quotas
            .acquire(group, Plane::Gateway, Duration::from_secs(4))
            .unwrap();
    }
    assert_eq!(
        quotas
            .acquire(group, Plane::Gateway, Duration::from_secs(4))
            .unwrap_err()
            .code(),
        AdmissionCode::Quota
    );
    let ids: BTreeSet<_> = (1..=10000)
        .map(|n| format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap())
        .collect();
    assert!(quotas.preflight(&ids).is_ok());
    let mut overflow = ids;
    overflow.insert("10000000-0000-4000-8000-00000000ffff".parse().unwrap());
    assert_eq!(
        quotas.preflight(&overflow).unwrap_err().code(),
        AdmissionCode::Quota
    );
}

#[test]
fn every_monotonic_sample_and_repair_preserve_observed_upper_time() {
    let mut clock = fake_clock(100);
    clock.fake = Some(Reading {
        wall: Duration::from_secs(112),
        elapsed: Duration::from_secs(10),
    });
    assert_eq!(clock.sample().unwrap().upper, Duration::from_secs(114));
    clock.fake = Some(Reading {
        wall: Duration::from_secs(111),
        elapsed: Duration::from_secs(11),
    });
    assert_eq!(clock.sample().unwrap().upper, Duration::from_secs(114));
    assert_eq!(
        clock.reanchor(1).unwrap_err().code(),
        AdmissionCode::RestartWait
    );
    clock.fake = Some(Reading {
        wall: Duration::from_secs(116),
        elapsed: Duration::from_secs(12),
    });
    clock.reanchor(1).unwrap();
    assert_eq!(clock.sample().unwrap().upper, Duration::from_secs(118));
    // This is above the anchor but below the most recent successful reading.
    clock.fake = Some(Reading {
        wall: Duration::from_secs(118),
        elapsed: Duration::from_secs(14),
    });
    clock.sample().unwrap();
    clock.fake = Some(Reading {
        wall: Duration::from_secs(117),
        elapsed: Duration::from_secs(13),
    });
    assert_eq!(clock.sample().err().unwrap().code(), AdmissionCode::Clock);
    assert_eq!(clock.reanchor(1).unwrap_err().code(), AdmissionCode::Clock);
}
