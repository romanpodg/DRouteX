//! End-to-end checks of the `veyra` binary.

use std::process::{Command, Output};

const EXAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../configs/examples/home.yaml"
);
const CONFLICT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/conflict.yaml");
const BROKEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/broken.yaml");
const WARN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/warn.yaml");
const MODES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/modes.yaml");

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

#[test]
fn explain_reports_and_honours_the_mode() {
    let run = |extra: &[&str]| {
        let mut args = vec!["explain", "--config", MODES, "--profile", "home"];
        args.extend_from_slice(extra);
        args.push("x.example");
        let out = veyra(&args);
        assert!(out.status.success(), "{}", text(&out.stderr));
        serde_json::from_slice::<serde_json::Value>(&out.stdout).expect("json on stdout")
    };
    let default = run(&[]);
    assert_eq!(default["mode"], "smartdns_tcp");
    assert_eq!(default["decided_by"], "default");
    let tunnel = run(&["--mode", "tunnel"]);
    assert_eq!(tunnel["mode"], "tunnel");
    assert_eq!(tunnel["route"]["type"], "block");

    let bad = veyra(&[
        "explain",
        "--config",
        MODES,
        "--profile",
        "home",
        "--mode",
        "udp",
        "x.example",
    ]);
    assert_eq!(bad.status.code(), Some(2));
    assert!(
        text(&bad.stderr).contains("unknown mode"),
        "{}",
        text(&bad.stderr)
    );
}

#[test]
fn explain_by_source_address_uses_admission() {
    let out = veyra(&[
        "explain",
        "--config",
        EXAMPLE,
        "--source",
        "192.168.1.50",
        "media.example",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
    assert_eq!(v["profile"], "home");
    assert_eq!(v["device"], "tv");
    assert_eq!(v["source"], "192.168.1.50");
    assert_eq!(v["source_binding"], "192.168.1.50/32");

    let stranger = veyra(&[
        "explain",
        "--config",
        EXAMPLE,
        "--source",
        "10.9.9.9",
        "media.example",
    ]);
    assert_eq!(stranger.status.code(), Some(1));
    assert!(
        text(&stranger.stderr).contains("not admitted"),
        "{}",
        text(&stranger.stderr)
    );

    let both = veyra(&[
        "explain",
        "--config",
        EXAMPLE,
        "--source",
        "127.0.0.1",
        "--profile",
        "home",
        "media.example",
    ]);
    assert_eq!(both.status.code(), Some(2));
}

#[test]
fn explain_shows_the_dns_answer_in_dns_mode() {
    let run = |args: &[&str]| {
        let mut full = vec!["explain", "--config", EXAMPLE];
        full.extend_from_slice(args);
        let out = veyra(&full);
        assert!(out.status.success(), "{}", text(&out.stderr));
        serde_json::from_slice::<serde_json::Value>(&out.stdout).expect("json on stdout")
    };
    let proxied = run(&["--profile", "home", "media.example"]);
    assert_eq!(
        proxied["dns_answer"]["A"],
        serde_json::json!(["203.0.113.10"])
    );
    assert_eq!(proxied["dns_answer"]["AAAA"], "NODATA");
    assert_eq!(proxied["dns_answer"]["HTTPS"], "NODATA");
    assert_eq!(proxied["dns_answer"]["ttl"], 60);

    let direct = run(&["--profile", "home", "example.org"]);
    assert_eq!(direct["dns_answer"]["A"], "forward:default");

    let tunnel = run(&["--profile", "home", "--mode", "tunnel", "media.example"]);
    assert!(tunnel.get("dns_answer").is_none(), "{tunnel}");
}
