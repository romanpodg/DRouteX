// Test-fixture decoding only. Surrogate escapes cannot be represented by the &str
// policy boundary and are counted separately, never claimed as passed vectors.
fn unescape(text: &str) -> Option<String> {
    if text == "\"\"" {
        return Some(String::new());
    }
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        let marker = chars.next().unwrap();
        let code = match marker {
            '\\' => {
                result.push('\\');
                continue;
            }
            'u' => chars.by_ref().take(4).collect::<String>(),
            'x' => {
                assert_eq!(chars.next(), Some('{'));
                chars.by_ref().take_while(|c| *c != '}').collect()
            }
            _ => panic!("unknown official fixture escape"),
        };
        result.push(char::from_u32(u32::from_str_radix(&code, 16).unwrap())?);
    }
    Some(result)
}

pub fn check(policy_name: impl Fn(&str) -> Result<String, ()>) {
    let mut success = 0;
    let mut refusal = 0;
    let mut flag_specific = 0;
    let mut non_scalar = 0;
    let mut mismatches = Vec::new();
    for (index, line) in include_str!("fixtures/IdnaTestV2-Unicode17.txt")
        .lines()
        .enumerate()
    {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(';').map(str::trim).collect();
        assert!(fields.len() >= 7);
        let Some(source) = unescape(fields[0]) else {
            non_scalar += 1;
            continue;
        };
        let unicode = if fields[1].is_empty() {
            source.clone()
        } else {
            let Some(unicode) = unescape(fields[1]) else {
                non_scalar += 1;
                continue;
            };
            unicode
        };
        let expected = if fields[3].is_empty() {
            unicode
        } else {
            let Some(expected) = unescape(fields[3]) else {
                non_scalar += 1;
                continue;
            };
            expected
        };
        let status = if fields[4].is_empty() {
            fields[2]
        } else {
            fields[4]
        };
        let status = status.trim_matches(['[', ']']);
        if status.is_empty() {
            if policy_name(&source).as_deref().ok()
                != Some(expected.strip_suffix('.').unwrap_or(&expected))
            {
                mismatches.push(format!("success line {} source {source:?}", index + 1));
            }
            success += 1;
        } else if status
            .split(',')
            .map(str::trim)
            .any(|code| !matches!(code, "V2" | "V3" | "U1" | "A4_1" | "A4_2"))
        {
            if policy_name(&source).is_ok() {
                mismatches.push(format!(
                    "required error line {} status {status}: {source:?}",
                    index + 1
                ));
            }
            refusal += 1;
        } else {
            // These vectors use the strict fixture's hyphen/STD3/root-dot flags.
            // Derive the D37 outcome from its independently supplied ASCII result
            // and the explicit DNS-label/input bounds, never from library output.
            let canonical = expected.strip_suffix('.').unwrap_or(&expected);
            let profile_valid = source.len() <= 1024
                && !canonical.is_empty()
                && canonical.len() <= 253
                && canonical.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && label.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || byte == b'-'
                                || byte == b'_'
                        })
                });
            let actual = policy_name(&source);
            if (profile_valid && actual.as_deref() != Ok(canonical))
                || (!profile_valid && actual.is_ok())
            {
                mismatches.push(format!(
                    "profile-specific line {} status {status}: {source:?}",
                    index + 1
                ));
            }
            flag_specific += 1;
        }
    }
    // This is an explicitly scoped profile check, not full Unicode conformance.
    println!(
        "Unicode17 vectors: shared successes={success}, required refusals={refusal}, profile-specific outcomes={flag_specific}, non-scalar &str-inexpressible={non_scalar}"
    );
    assert_eq!(success + refusal + flag_specific + non_scalar, 6391);
    assert!(
        mismatches.is_empty(),
        "{} qualification mismatches (no waiver): {:#?}",
        mismatches.len(),
        mismatches.iter().take(12).collect::<Vec<_>>()
    );
}
