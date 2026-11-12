//! # Manifest parsing and validation (VER08a §2)
//!
//! Malformed JSON, schema violations, duplicates, taxonomy errors.  Every
//! rejection must be an `Err` (fail closed); nothing may degrade to "pure".

mod common;

use coding_adventures_ca_capability_analyzer::manifest::JsonValue;
use coding_adventures_ca_capability_analyzer::{load_manifest, ManifestError};
use common::Pkg;

fn load(json: &str) -> Result<coding_adventures_ca_capability_analyzer::ManifestData, ManifestError> {
    let p = Pkg::with("man", &[("required_capabilities.json", json)]);
    load_manifest(p.dir())
}

fn err(json: &str) -> String {
    match load(json) {
        Ok(m) => panic!("expected rejection, got {m:?}\n{json}"),
        Err(e) => e.to_string(),
    }
}

const OK_TAIL: &str = r#""capabilities": [], "justification": "Valid justification text.""#;

fn with_tail(head: &str) -> String {
    format!("{{{head}, {OK_TAIL}}}")
}

#[test]
fn valid_minimal_manifest_loads() {
    let m = load(&with_tail(r#""version": 1, "package": "rust/ok-pkg_1""#)).unwrap();
    assert_eq!(m.package, "rust/ok-pkg_1");
    assert!(m.is_explicit);
    assert!(m.capabilities.is_empty());
}

#[test]
fn malformed_json_is_rejected() {
    let cases: &[(&str, &str)] = &[
        ("empty file", ""),
        ("whitespace only", "  \n "),
        ("open brace", "{"),
        ("root array", "[]"),
        ("root null", "null"),
        ("root string", "\"x\""),
        ("root number", "1"),
        ("trailing garbage", r#"{"version":1} x"#),
        ("trailing comma object", r#"{"version":1,}"#),
        ("trailing comma array", r#"{"capabilities":[1,]}"#),
        ("single quotes", "{'version': 1}"),
        ("unquoted key", "{version: 1}"),
        ("comment", "{/* c */ \"version\": 1}"),
        ("NaN", r#"{"version": NaN}"#),
        ("missing colon", r#"{"version" 1}"#),
        ("missing comma", r#"{"a": 1 "b": 2}"#),
        ("truncated string", r#"{"version": "abc"#),
        ("bad escape", r#"{"a": "\x41"}"#),
        ("bad unicode escape", r#"{"a": "\u12G4"}"#),
        ("truncated unicode escape", r#"{"a": "\u12"}"#),
        ("lone surrogate", r#"{"a": "\ud800"}"#),
        ("raw newline in string", "{\"a\": \"x\ny\"}"),
        ("raw tab in string", "{\"a\": \"x\ty\"}"),
        ("raw control char in string", "{\"a\": \"x\u{1}y\"}"),
        ("leading BOM", "\u{feff}{\"version\": 1}"),
        ("unicode whitespace", "{\u{a0}\"version\": 1}"),
        ("bad literal", r#"{"a": tru}"#),
        ("bad literal null", r#"{"a": nul}"#),
        ("leading zero number", r#"{"version": 01}"#),
        ("plus sign number", r#"{"version": +1}"#),
        ("bare minus", r#"{"version": -}"#),
        ("dot without digits", r#"{"version": 1.}"#),
        ("exponent without digits", r#"{"version": 1e}"#),
        ("hex number", r#"{"version": 0x1}"#),
        ("unterminated array", r#"{"capabilities": ["#),
        ("unterminated object", r#"{"capabilities": {"#),
    ];
    for (name, json) in cases {
        assert!(load(json).is_err(), "case '{name}' must be rejected: {json:?}");
    }
}

#[test]
fn deeply_nested_json_is_rejected_not_overflowed() {
    let deep = format!("{}{}", "[".repeat(150), "]".repeat(150));
    assert!(JsonValue::parse(&deep).is_err());
    let deep_obj = format!("{}1{}", "{\"a\":".repeat(150), "}".repeat(150));
    assert!(JsonValue::parse(&deep_obj).is_err());
    // 100 levels is allowed by the parser (then rejected by schema if used as a manifest).
    let ok = format!("{}{}", "[".repeat(50), "]".repeat(50));
    assert!(JsonValue::parse(&ok).is_ok());
}

#[test]
fn duplicate_json_keys_are_rejected_at_any_depth() {
    assert!(err(r#"{"version": 1, "version": 2, "package": "rust/x", "capabilities": [], "justification": "Valid justification text."}"#)
        .contains("duplicate"));
    // The classic differential: a benign list first, an evil list second (or vice versa).
    let sneaky = r#"{"version": 1, "package": "rust/x", "capabilities": [], "justification": "Valid justification text.",
        "capabilities": [{"category":"fs","action":"read","target":"*","justification":"Sneaky second list."}]}"#;
    assert!(err(sneaky).contains("duplicate"));
    // Nested.
    let nested = r#"{"version": 1, "package": "rust/x", "justification": "Valid justification text.",
        "capabilities": [{"category":"fs","category":"net","action":"read","target":"*","justification":"Duplicated key."}]}"#;
    assert!(err(nested).contains("duplicate"));
    assert!(JsonValue::parse(r#"{"a":1,"a":1}"#).is_err());
}

#[test]
fn schema_violations_are_rejected() {
    let base = |extra: &str| format!(r#"{{"version": 1, "package": "rust/x", "capabilities": [], "justification": "Valid justification text."{extra}}}"#);
    assert!(err(&base(r#", "surprise": 1"#)).contains("unknown top-level property"));
    // Missing required fields, each in turn.
    for missing in ["version", "package", "capabilities", "justification"] {
        let mut fields = vec![
            ("version", "1"),
            ("package", "\"rust/x\""),
            ("capabilities", "[]"),
            ("justification", "\"Valid justification text.\""),
        ];
        fields.retain(|(k, _)| *k != missing);
        let json = format!(
            "{{{}}}",
            fields.iter().map(|(k, v)| format!("\"{k}\": {v}")).collect::<Vec<_>>().join(", ")
        );
        assert!(load(&json).is_err(), "missing '{missing}' must be rejected");
    }
    // Wrong types.
    for (k, bad) in [
        ("version", "\"1\""),
        ("version", "1.0"),
        ("version", "1e0"),
        ("version", "true"),
        ("version", "null"),
        ("package", "1"),
        ("capabilities", "{}"),
        ("capabilities", "\"none\""),
        ("justification", "[]"),
    ] {
        let mut s = String::from("{");
        let vals = [
            ("version", "1"),
            ("package", "\"rust/x\""),
            ("capabilities", "[]"),
            ("justification", "\"Valid justification text.\""),
        ];
        let parts: Vec<String> = vals
            .iter()
            .map(|(kk, v)| format!("\"{kk}\": {}", if *kk == k { bad } else { v }))
            .collect();
        s.push_str(&parts.join(", "));
        s.push('}');
        assert!(load(&s).is_err(), "{k}={bad} must be rejected");
    }
    assert!(err(&with_tail(r#""version": 2, "package": "rust/x""#)).contains("unsupported version"));
    assert!(err(&with_tail(r#""version": 0, "package": "rust/x""#)).contains("unsupported version"));
    assert!(err(&with_tail(r#""version": -1, "package": "rust/x""#)).contains("unsupported version"));
}

#[test]
fn package_names_are_validated() {
    for bad in [
        "python/x", "rust/", "rust", "Rust/x", "rust/Foo", "rust/a b", "rust/a.b", "rust/a/b",
        "rust/\u{e9}", "", "go/ca-capability-analyzer",
    ] {
        let json = with_tail(&format!(r#""version": 1, "package": "{bad}""#));
        assert!(load(&json).is_err(), "package '{bad}' must be rejected");
    }
    for good in ["rust/a", "rust/a-b", "rust/a_b", "rust/a1", "rust/0", "rust/-", "rust/_"] {
        let json = with_tail(&format!(r#""version": 1, "package": "{good}""#));
        assert!(load(&json).is_ok(), "package '{good}' must be accepted");
    }
}

fn cap(c: &str, a: &str, t: &str, j: &str) -> String {
    format!(
        r#"{{"version": 1, "package": "rust/x", "justification": "Valid justification text.",
        "capabilities": [{{"category": "{c}", "action": "{a}", "target": "{t}", "justification": "{j}"}}]}}"#
    )
}

#[test]
fn unknown_capabilities_and_pairs_are_rejected() {
    let ok_j = "Long enough text.";
    assert!(load(&cap("fs", "read", "*", ok_j)).is_ok());
    for (c, a) in [
        ("bogus", "read"),
        ("fs", "fly"),
        ("fs", "connect"),
        ("net", "read"),
        ("net", "write"),
        ("proc", "read"),
        ("env", "exec"),
        ("ffi", "read"),
        ("time", "write"),
        ("stdin", "write"),
        ("stdout", "read"),
        ("rand", "write"),
        ("thread", "read"),
        ("ui", "call"),
        ("", "read"),
        ("fs", ""),
        ("FS", "read"),
        ("fs", "READ"),
        ("fs ", "read"),
    ] {
        let e = err(&cap(c, a, "*", ok_j));
        assert!(e.contains("invalid capability pair"), "{c}:{a} -> {e}");
    }
    // Every legal pair is accepted.
    for (c, a) in [
        ("fs", "read"), ("fs", "write"), ("fs", "create"), ("fs", "delete"), ("fs", "list"),
        ("net", "connect"), ("net", "listen"), ("net", "dns"),
        ("proc", "exec"), ("proc", "fork"), ("proc", "signal"),
        ("env", "read"), ("env", "write"),
        ("ffi", "call"), ("ffi", "load"),
        ("time", "read"), ("time", "sleep"),
        ("stdin", "read"), ("stdout", "write"), ("rand", "read"),
        ("thread", "spawn"), ("thread", "message"),
        ("ui", "read"), ("ui", "write"), ("ui", "event"),
    ] {
        assert!(load(&cap(c, a, "*", ok_j)).is_ok(), "{c}:{a} must be accepted");
    }
}

#[test]
fn capability_field_validation() {
    assert!(err(&cap("fs", "read", "", "Long enough text.")).contains("target must not be empty"));
    assert!(err(&cap("fs", "read", "   ", "Long enough text.")).contains("target must not be empty"));
    for bad in ["a b", "a;b", "a\\\\b", "$(x)", "a`b", "a\\u00e9", "a|b"] {
        assert!(load(&cap("fs", "read", bad, "Long enough text.")).is_err(), "target {bad:?}");
    }
    for ok in ["*", "a/b.txt", "../../grammars/*.tokens", "host.example-1_x/*"] {
        assert!(load(&cap("fs", "read", ok, "Long enough text.")).is_ok(), "target {ok:?}");
    }
    assert!(err(&cap("fs", "read", "*", "short")).contains("at least 10 characters"));
    assert!(err(&cap("fs", "read", "*", "         x")).contains("at least 10 characters"));
    // Unknown property inside a capability, wrong types, non-object entries.
    let unknown = r#"{"version": 1, "package": "rust/x", "justification": "Valid justification text.",
        "capabilities": [{"category":"fs","action":"read","target":"*","justification":"Long enough text.","extra":1}]}"#;
    assert!(err(unknown).contains("unknown capability"));
    let not_obj = r#"{"version": 1, "package": "rust/x", "justification": "Valid justification text.", "capabilities": ["fs:read:*"]}"#;
    assert!(err(not_obj).contains("must be an object"));
    for field in ["category", "action", "target", "justification"] {
        let mut parts = [
            ("category", "\"fs\""),
            ("action", "\"read\""),
            ("target", "\"*\""),
            ("justification", "\"Long enough text.\""),
        ];
        for p in parts.iter_mut() {
            if p.0 == field {
                p.1 = "5";
            }
        }
        let json = format!(
            r#"{{"version": 1, "package": "rust/x", "justification": "Valid justification text.", "capabilities": [{{{}}}]}}"#,
            parts.iter().map(|(k, v)| format!("\"{k}\": {v}")).collect::<Vec<_>>().join(", ")
        );
        assert!(load(&json).is_err(), "non-string {field}");
    }
}

#[test]
fn duplicate_capabilities_are_rejected_but_distinct_targets_are_not() {
    let two = |t1: &str, t2: &str| {
        format!(
            r#"{{"version": 1, "package": "rust/x", "justification": "Valid justification text.", "capabilities": [
            {{"category":"fs","action":"read","target":"{t1}","justification":"Long enough text."}},
            {{"category":"fs","action":"read","target":"{t2}","justification":"Long enough text."}}]}}"#
        )
    };
    assert!(err(&two("*", "*")).contains("duplicate capability"));
    assert!(err(&two("a.txt", "a.txt")).contains("duplicate capability"));
    let m = load(&two("a.txt", "b.txt")).unwrap();
    assert_eq!(m.capabilities.len(), 2);
}

#[test]
fn targets_are_not_enforced_any_target_covers_all_uses() {
    // VER08a G-3: documented limitation, pinned so it cannot change silently.
    let m = load(&cap("fs", "read", "/etc/hosts", "Long enough text.")).unwrap();
    assert!(m.covers_capability("fs", "read"));
    assert!(!m.covers_capability("fs", "write"));
    assert!(!m.covers_capability("net", "connect"));
    assert_eq!(m.capabilities[0].exact_string(), "fs:read:/etc/hosts");
    assert_eq!(m.capabilities[0].wildcard_string(), "fs:read:*");
}

fn exc(items: &str) -> String {
    format!(
        r#"{{"version": 1, "package": "rust/x", "capabilities": [], "justification": "Valid justification text.", "banned_construct_exceptions": [{items}]}}"#
    )
}

#[test]
fn banned_construct_exception_validation() {
    let good = r#"{"construct": "unsafe_code", "language": "rust", "justification": "Audited barrier code."}"#;
    let m = load(&exc(good)).unwrap();
    assert!(m.is_construct_exempt("unsafe_code"));
    assert!(!m.is_construct_exempt("inline_assembly"));
    assert!(!m.is_construct_exempt("foreign_linkage"));

    // Case-insensitive on input, normalised for matching.
    let upper = r#"{"construct": "UNSAFE_CODE", "language": "RUST", "justification": "Audited barrier code."}"#;
    assert!(load(&exc(upper)).unwrap().is_construct_exempt("unsafe_code"));

    // All three Rust constructs are individually accepted.
    for c in ["unsafe_code", "inline_assembly", "foreign_linkage"] {
        let one = format!(r#"{{"construct": "{c}", "language": "rust", "justification": "Audited barrier code."}}"#);
        assert!(load(&exc(&one)).unwrap().is_construct_exempt(c));
    }

    // Duplicates (including case variants) and unknown constructs are rejected.
    assert!(err(&exc(&format!("{good},{good}"))).contains("duplicate banned_construct_exceptions"));
    assert!(err(&exc(&format!("{good},{upper}"))).contains("duplicate banned_construct_exceptions"));
    for bad in ["eval", "exec", "unsafe", "", "unsafe_code ", "inline-assembly", "CAP002", "*"] {
        let one = format!(r#"{{"construct": "{bad}", "language": "rust", "justification": "Audited barrier code."}}"#);
        // "unsafe_code " (trailing space) is trimmed to a valid id; everything else is not.
        let r = load(&exc(&one));
        if bad == "unsafe_code " {
            assert!(r.is_ok());
        } else {
            assert!(r.is_err(), "construct {bad:?} must be rejected");
        }
    }
    // Wrong language, short justification, extra property, wrong types, not an array.
    let wrong_lang = r#"{"construct": "unsafe_code", "language": "python", "justification": "Audited barrier code."}"#;
    assert!(err(&exc(wrong_lang)).contains("must be 'rust'"));
    let short = r#"{"construct": "unsafe_code", "language": "rust", "justification": "short"}"#;
    assert!(err(&exc(short)).contains("at least 10 characters"));
    let extra = r#"{"construct": "unsafe_code", "language": "rust", "justification": "Audited barrier code.", "x": 1}"#;
    assert!(err(&exc(extra)).contains("unknown property"));
    let wrong_type = r#"{"construct": 1, "language": "rust", "justification": "Audited barrier code."}"#;
    assert!(load(&exc(wrong_type)).is_err());
    assert!(load(&exc("5")).is_err());
    let not_array = r#"{"version": 1, "package": "rust/x", "capabilities": [], "justification": "Valid justification text.", "banned_construct_exceptions": {}}"#;
    assert!(err(not_array).contains("must be an array"));
}

#[test]
fn json_unicode_and_escapes_decode_correctly() {
    let v = JsonValue::parse(r#"{"a": "A\n\t\"\\\/\b\f\r"}"#).unwrap();
    assert_eq!(v.get("a").unwrap().as_str().unwrap(), "A\n\t\"\\/\u{8}\u{c}\r");
    let n = JsonValue::parse("[0, -0, 12, -3, 1.5, 2e3, 1E-2, 9223372036854775807]").unwrap();
    let arr = n.as_array().unwrap();
    assert_eq!(arr[0].as_i64(), Some(0));
    assert_eq!(arr[2].as_i64(), Some(12));
    assert_eq!(arr[3].as_i64(), Some(-3));
    assert_eq!(arr[4].as_i64(), None);
    assert_eq!(arr[5].as_i64(), None);
    assert_eq!(arr[7].as_i64(), Some(i64::MAX));
    // Out-of-range integers are not integers (the version check then rejects them).
    let big = JsonValue::parse("9223372036854775808").unwrap();
    assert_eq!(big.as_i64(), None);
    assert!(JsonValue::parse("true").unwrap().as_str().is_none());
    assert!(JsonValue::parse("  [ ]  ").unwrap().as_array().unwrap().is_empty());
    assert!(JsonValue::parse("{ }").unwrap().as_object().unwrap().is_empty());
}

#[test]
fn absent_manifest_is_pure_computation() {
    let p = Pkg::with("noman", &[("src/lib.rs", "")]);
    let m = load_manifest(p.dir()).unwrap();
    assert!(!m.is_explicit);
    assert_eq!(m.package, "pure-computation");
    assert!(m.capabilities.is_empty());
    assert!(!m.covers_capability("fs", "read"));
    assert!(!m.is_construct_exempt("unsafe_code"));
}

#[test]
fn oversized_manifest_is_rejected() {
    let big = format!(
        r#"{{"version": 1, "package": "rust/x", "capabilities": [], "justification": "{}"}}"#,
        "x".repeat(1024 * 1024 + 10)
    );
    let e = err(&big);
    assert!(e.contains("exceeds 1 MiB"), "{e}");
}

#[cfg(unix)]
#[test]
fn symlinked_manifest_is_rejected() {
    let p = Pkg::with("sym", &[("real.json", &common::pure_manifest("x"))]);
    std::os::unix::fs::symlink(p.path("real.json"), p.path("required_capabilities.json")).unwrap();
    let e = load_manifest(p.dir()).unwrap_err().to_string();
    assert!(e.contains("symlink"), "{e}");
}

#[test]
fn non_utf8_manifest_is_an_io_error_not_a_pass() {
    let p = Pkg::empty("utf8");
    std::fs::write(p.path("required_capabilities.json"), [0xff, 0xfe, 0x7b, 0x7d]).unwrap();
    assert!(matches!(load_manifest(p.dir()), Err(ManifestError::Io(_))));
}

#[test]
fn manifest_error_display_is_informative() {
    for json in ["{", &with_tail(r#""version": 2, "package": "rust/x""#)] {
        let e = load(json).unwrap_err();
        assert!(!e.to_string().is_empty());
    }
}
