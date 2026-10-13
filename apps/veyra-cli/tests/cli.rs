//! End-to-end checks of the `veyra` binary.

use std::process::{Command, Output};

const EXAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../configs/examples/home.yaml"
);
const CONFLICT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/conflict.yaml");
const BROKEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/broken.yaml");
const WARN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/warn.yaml");

fn veyra(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_veyra"))
        .args(args)
        .output()
        .expect("run veyra")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn validate_accepts_the_example() {
    let out = veyra(&["validate", EXAMPLE]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("valid, 0 warning(s)"));
}

#[test]
fn validate_reports_conflicts_and_fails() {
    let out = veyra(&["validate", CONFLICT]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stdout).contains("error[PatternConflict] a, b"),
        "{}",
        text(&out.stdout)
    );
    assert!(text(&out.stderr).contains("1 error(s)"));
}

#[test]
fn malformed_yaml_names_the_line() {
    let out = veyra(&["validate", BROKEN]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("line"), "{}", text(&out.stderr));
    assert!(
        !text(&out.stderr).contains("`match`"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn explain_prints_the_deciding_rule_as_json() {
    let out = veyra(&[
        "explain",
        "--config",
        EXAMPLE,
        "--profile",
        "home",
        "WWW.Media.Example.",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
    assert_eq!(v["domain"], "www.media.example");
    assert_eq!(v["matched_rule_id"], "video-through-germany");
    assert_eq!(v["route"]["type"], "proxy_node");
    assert_eq!(v["route"]["node"], "de-01");
    assert_eq!(v["route"]["onFailure"], "block");
    assert_eq!(v["decided_by"], "only_match");
}

#[test]
fn explain_by_device_applies_device_rules() {
    let out = veyra(&[
        "explain",
        "--config",
        EXAMPLE,
        "--device",
        "work-laptop",
        "git.corp.example",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
    assert_eq!(v["profile"], "home");
    assert_eq!(v["matched_rule_id"], "work-resolver");
    assert_eq!(v["upstream_group"], "corp");
}

#[test]
fn explain_rejects_bad_requests_cleanly() {
    let cases: [&[&str]; 4] = [
        &["--profile", "nobody", "x.example"],
        &["--device", "tv", "--profile", "other", "x.example"],
        &["--profile", "home", "a..b"],
        &["--profile", "home", "a b.example"],
    ];
    for args in cases {
        let mut full = vec!["explain", "--config", EXAMPLE];
        full.extend_from_slice(args);
        let out = veyra(&full);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        let err = text(&out.stderr);
        assert!(
            !err.is_empty() && !err.contains("panicked"),
            "{args:?}: {err}"
        );
    }
}

#[test]
fn explain_refuses_a_config_with_errors() {
    let out = veyra(&[
        "explain",
        "--config",
        CONFLICT,
        "--profile",
        "home",
        "media.example",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("veyra validate"));
}

#[test]
fn explain_prints_compile_warnings_to_stderr() {
    let out = veyra(&[
        "explain",
        "--config",
        WARN,
        "--profile",
        "home",
        "a.x.example",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let err = text(&out.stderr);
    assert!(err.contains("warning[TieBrokenLate] a, b"), "{err}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
    assert_eq!(v["decided_by"], "action");
}
