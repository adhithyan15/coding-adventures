//! # Certificate generation, canonical form and tamper detection (VER23a §3-4)
//!
//! Strategy: emit a genuine certificate for a real temp package, prove it
//! verifies, then alter exactly one thing (a certificate field, a source file,
//! the manifest, the generated module) and require verification to fail.

mod common;

use std::fs;

use coding_adventures_ca_capability_analyzer::certificate::{
    compute_manifest_digest, compute_source_digest, discover_public_entry_points, json_str,
    normalize_crlf, parse_certificate, EffectCertificate,
};
use coding_adventures_ca_capability_analyzer::{generate_certificate, verify_certificate};
use common::{manifest, package, pure_manifest, Pkg};

const LIB: &str = "pub fn first(a: &[u8; 4]) -> u8 { a[0] }\npub fn second(x: u8) -> u8 { x }\n";

fn pure_pkg(tag: &str) -> Pkg {
    package(tag, LIB, &pure_manifest("tamper-pure"))
}

fn effectful_pkg(tag: &str) -> Pkg {
    package(
        tag,
        "pub fn say(s: &str) { println!(\"{s}\"); }\npub fn quiet() {}\n",
        &manifest("tamper-fx", &[("stdout", "write")], &[]),
    )
}

fn emit(p: &Pkg) -> EffectCertificate {
    let cert = generate_certificate(p.dir()).unwrap();
    fs::write(p.path("cert.json"), cert.to_json_string()).unwrap();
    cert
}

fn verify(p: &Pkg) -> Result<(), String> {
    verify_certificate(p.dir(), &p.path("cert.json")).map(|_| ())
}

fn verify_cert(p: &Pkg, cert: &EffectCertificate) -> Result<(), String> {
    fs::write(p.path("tampered.json"), cert.to_json_string()).unwrap();
    verify_certificate(p.dir(), &p.path("tampered.json")).map(|_| ())
}

fn verify_text(p: &Pkg, text: &str) -> Result<(), String> {
    fs::write(p.path("tampered.json"), text).unwrap();
    verify_certificate(p.dir(), &p.path("tampered.json")).map(|_| ())
}

#[test]
fn genuine_certificates_verify_and_are_deterministic() {
    for p in [pure_pkg("gen_pure"), effectful_pkg("gen_fx")] {
        let c1 = emit(&p);
        let c2 = generate_certificate(p.dir()).unwrap();
        assert_eq!(c1, c2, "generation must be deterministic");
        assert_eq!(c1.to_json_string(), c2.to_json_string());
        verify(&p).unwrap();
        // Round trip.
        assert_eq!(parse_certificate(&c1.to_json_string()).unwrap(), c1);
        assert_eq!(c1.label, "proved");
        assert_eq!(c1.schema_version, "ver24-effect-cert-v1");
        assert!(c1.unconditional_roots.is_empty());
    }
}

#[test]
fn entry_points_have_bracket_aware_signatures_and_over_approximated_effects() {
    let p = effectful_pkg("ep_fx");
    let c = emit(&p);
    assert_eq!(c.entry_points.len(), 2);
    assert_eq!(c.entry_points[0].name, "say");
    assert_eq!(c.entry_points[0].signature, "pub fn say(s: &str)");
    // No per-entry attribution exists: every entry carries all declared effects.
    for ep in &c.entry_points {
        assert_eq!(ep.effects, vec!["stdout:write:*".to_string()]);
    }
    let pure = pure_pkg("ep_pure");
    let c = emit(&pure);
    assert_eq!(c.entry_points[0].signature, "pub fn first(a: &[u8; 4]) -> u8");
    assert!(c.entry_points.iter().all(|e| e.effects.is_empty()));
}

#[test]
fn committed_ct_compare_certificate_is_exactly_what_the_generator_derives() {
    // Not hardcoded: regenerate from the package and compare to the committed file byte for byte.
    let dir = std::path::Path::new("../ct-compare");
    let fresh = generate_certificate(dir).unwrap().to_json_string();
    let committed = fs::read_to_string(dir.join("effect_certificate.json")).unwrap();
    assert_eq!(fresh, committed, "effect_certificate.json is stale; re-run --emit-cert");
    verify_certificate(dir, &dir.join("effect_certificate.json")).unwrap();
    // The committed signature of the const-generic function is whole (it was truncated at ';').
    assert!(committed.contains("pub fn ct_eq_fixed<const N: usize>(a: &[u8; N], b: &[u8; N]) -> bool"));
}

// ---------------------------------------------------------------------------
// Tamper: every certificate field, one at a time
// ---------------------------------------------------------------------------

fn flip_hex(s: &str) -> String {
    let mut c: Vec<char> = s.chars().collect();
    c[0] = if c[0] == '0' { '1' } else { '0' };
    c.into_iter().collect()
}

#[test]
fn flipping_any_scalar_field_fails_verification() {
    let p = effectful_pkg("flip");
    let good = emit(&p);

    let mut c = good.clone();
    c.package = "rust/other".into();
    assert!(verify_cert(&p, &c).unwrap_err().contains("package"));

    let mut c = good.clone();
    c.source_digest = flip_hex(&good.source_digest);
    assert!(verify_cert(&p, &c).unwrap_err().contains("source_digest mismatch"));

    let mut c = good.clone();
    c.manifest_digest = flip_hex(&good.manifest_digest);
    assert!(verify_cert(&p, &c).unwrap_err().contains("manifest_digest mismatch"));

    let mut c = good.clone();
    c.source_digest = "0".repeat(64);
    assert!(verify_cert(&p, &c).is_err());

    let mut c = good.clone();
    c.manifest_digest = String::new();
    assert!(verify_cert(&p, &c).is_err());

    for bad_label in ["", "PROVED", "proved ", "assumed", "unverified", "proof"] {
        let mut c = good.clone();
        c.label = bad_label.into();
        assert!(verify_cert(&p, &c).unwrap_err().contains("label"), "label {bad_label:?}");
    }

    let mut c = good.clone();
    c.schema_version = "ver24-effect-cert-v2".into();
    assert!(verify_cert(&p, &c).unwrap_err().contains("schema_version"));
}

#[test]
fn tampering_declared_effects_fails() {
    let p = effectful_pkg("fx_decl");
    let good = emit(&p);
    for mutate in [
        |c: &mut EffectCertificate| c.declared_effects.clear(),
        |c: &mut EffectCertificate| c.declared_effects.push("fs:read:*".into()),
        |c: &mut EffectCertificate| c.declared_effects[0] = "stdout:write:other".into(),
        |c: &mut EffectCertificate| c.declared_effects.push("stdout:write:*".into()),
    ] {
        let mut c = good.clone();
        mutate(&mut c);
        assert!(verify_cert(&p, &c).is_err());
    }
    // Reordering declared effects (two capabilities).
    let two = package(
        "fx_two",
        "pub fn f() { println!(\"x\"); let _ = std::env::args(); }\n",
        &manifest("two", &[("stdout", "write"), ("env", "read")], &[]),
    );
    let mut c = emit(&two);
    c.declared_effects.reverse();
    assert!(verify_cert(&two, &c).unwrap_err().contains("declared_effects mismatch"));
}

#[test]
fn tampering_entry_points_fails() {
    let p = effectful_pkg("ep_tamper");
    let good = emit(&p);

    let mut c = good.clone();
    c.entry_points.pop();
    assert!(verify_cert(&p, &c).unwrap_err().contains("count mismatch"));

    let mut c = good.clone();
    c.entry_points.clear();
    assert!(verify_cert(&p, &c).unwrap_err().contains("count mismatch"));

    let mut c = good.clone();
    c.entry_points.push(c.entry_points[0].clone());
    assert!(verify_cert(&p, &c).unwrap_err().contains("count mismatch"));

    let mut c = good.clone();
    c.entry_points[0].name = "renamed".into();
    assert!(verify_cert(&p, &c).unwrap_err().contains("Uncertified public entry point"));

    let mut c = good.clone();
    c.entry_points.swap(0, 1);
    assert!(verify_cert(&p, &c).unwrap_err().contains("order mismatch"));

    let mut c = good.clone();
    c.entry_points[0].signature = "pub fn say(s: &str, extra: u8)".into();
    assert!(verify_cert(&p, &c).unwrap_err().contains("signature mismatch"));

    let mut c = good.clone();
    c.entry_points[0].signature.push(' ');
    assert!(verify_cert(&p, &c).unwrap_err().contains("signature mismatch"));

    // Under-approximation: claiming fewer effects than the package declares.
    let mut c = good.clone();
    c.entry_points[1].effects.clear();
    assert!(verify_cert(&p, &c).unwrap_err().contains("must equal declared_effects"));

    // Over-claim: an effect the package does not declare.
    let mut c = good.clone();
    c.entry_points[1].effects.push("fs:read:*".into());
    assert!(verify_cert(&p, &c).is_err());

    // Pure packages: any effect claim is rejected with the dedicated message.
    let pure = pure_pkg("ep_pure_tamper");
    let good = emit(&pure);
    let mut c = good.clone();
    c.entry_points[0].effects.push("fs:read:*".into());
    assert!(verify_cert(&pure, &c).unwrap_err().contains("claims non-empty effects"));
}

#[test]
fn tampering_roots_and_exceptions_fails() {
    let p = package(
        "exc",
        "pub fn f() { unsafe { } }\n",
        &manifest("exc-pkg", &[], &["unsafe_code"]),
    );
    let good = emit(&p);
    assert_eq!(good.banned_construct_exceptions.len(), 1);
    verify(&p).unwrap();

    let mut c = good.clone();
    c.unconditional_roots.push("init".into());
    assert!(verify_cert(&p, &c).unwrap_err().contains("unconditional_roots"));

    let mut c = good.clone();
    c.banned_construct_exceptions.clear();
    assert!(verify_cert(&p, &c).unwrap_err().contains("exceptions count mismatch"));

    let mut c = good.clone();
    c.banned_construct_exceptions[0].justification.push_str(" (edited)");
    assert!(verify_cert(&p, &c).unwrap_err().contains("unverified banned exception"));

    let mut c = good.clone();
    c.banned_construct_exceptions[0].construct = "inline_assembly".into();
    assert!(verify_cert(&p, &c).is_err());

    let mut c = good.clone();
    c.banned_construct_exceptions[0].language = "python".into();
    assert!(verify_cert(&p, &c).is_err());

    let mut c = good.clone();
    let extra = c.banned_construct_exceptions[0].clone();
    c.banned_construct_exceptions.push(extra);
    assert!(verify_cert(&p, &c).unwrap_err().contains("count mismatch"));
}

#[test]
fn strict_parse_rejects_missing_unknown_duplicate_and_mistyped_fields() {
    let p = pure_pkg("strict");
    let good = emit(&p).to_json_string();
    verify_text(&p, &good).unwrap();

    // Missing each required field.
    for field in [
        "schema_version", "package", "source_digest", "manifest_digest", "declared_effects",
        "entry_points", "unconditional_roots", "banned_construct_exceptions", "label",
    ] {
        let v = coding_adventures_ca_capability_analyzer::manifest::JsonValue::parse(&good).unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj.contains_key(field));
        // Rebuild without the field by textual surgery on a minimal doc.
        let mut parts = Vec::new();
        for (k, val) in obj {
            if k != field {
                parts.push(format!("{}: {}", json_str(k), render(val)));
            }
        }
        let doc = format!("{{{}}}", parts.join(","));
        let e = verify_text(&p, &doc).unwrap_err();
        assert!(e.contains(field), "missing {field}: {e}");
    }

    // Unknown fields (top level, entry point, exception).
    let extra_top = good.replacen('{', "{\"extra\": 1,", 1);
    assert!(verify_text(&p, &extra_top).unwrap_err().contains("Unknown field"));
    let extra_ep = good.replacen("\"name\"", "\"surplus\": 1, \"name\"", 1);
    assert!(verify_text(&p, &extra_ep).unwrap_err().contains("Unknown field"));

    // Duplicate keys.
    let dup = good.replacen('{', "{\"label\": \"proved\",", 1);
    assert!(verify_text(&p, &dup).unwrap_err().contains("duplicate"));
    let dup_digest = good.replacen('{', &format!("{{\"source_digest\": \"{}\",", "0".repeat(64)), 1);
    assert!(verify_text(&p, &dup_digest).unwrap_err().contains("duplicate"));

    // Wrong types.
    for (from, to) in [
        ("\"declared_effects\": []", "\"declared_effects\": {}"),
        ("\"declared_effects\": []", "\"declared_effects\": [1]"),
        ("\"declared_effects\": []", "\"declared_effects\": null"),
        ("\"unconditional_roots\": []", "\"unconditional_roots\": \"\""),
        ("\"label\": \"proved\"", "\"label\": true"),
        ("\"label\": \"proved\"", "\"label\": null"),
    ] {
        assert!(good.contains(from), "fixture changed: {from}");
        assert!(verify_text(&p, &good.replace(from, to)).is_err(), "{from} -> {to}");
    }

    // Not JSON / wrong shape / trailing garbage / empty.
    for bad in ["", "{", "[]", "null", "\"x\"", &format!("{good} x"), &format!("{good}{good}")] {
        assert!(verify_text(&p, bad).is_err(), "{bad:?}");
    }
}

fn render(v: &coding_adventures_ca_capability_analyzer::manifest::JsonValue) -> String {
    use coding_adventures_ca_capability_analyzer::manifest::JsonValue as J;
    match v {
        J::Null => "null".into(),
        J::Bool(b) => b.to_string(),
        J::Number(f, _) => f.to_string(),
        J::String(s) => json_str(s),
        J::Array(a) => format!("[{}]", a.iter().map(render).collect::<Vec<_>>().join(",")),
        J::Object(o) => format!(
            "{{{}}}",
            o.iter().map(|(k, v)| format!("{}:{}", json_str(k), render(v))).collect::<Vec<_>>().join(",")
        ),
    }
}

#[test]
fn whitespace_and_key_order_are_semantically_neutral() {
    // Documented (VER23a §4.1 item 1): the checker compares meaning, not bytes.
    let p = pure_pkg("ws");
    let good = emit(&p).to_json_string();
    let v = coding_adventures_ca_capability_analyzer::manifest::JsonValue::parse(&good).unwrap();
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    keys.reverse();
    let reordered = format!(
        "  {{ \n\t{}\n }}  \r\n",
        keys.iter()
            .map(|k| format!("{} :\n {}", json_str(k), render(v.get(k).unwrap())))
            .collect::<Vec<_>>()
            .join(" ,\n")
    );
    verify_text(&p, &reordered).unwrap();
    verify_text(&p, &good.replace('\n', "\r\n")).unwrap();
}

// ---------------------------------------------------------------------------
// Tamper: the package changes after the certificate was issued
// ---------------------------------------------------------------------------

#[test]
fn stale_source_is_detected_for_every_kind_of_edit() {
    type Edit = (&'static str, Box<dyn Fn(&Pkg)>);
    let edits: Vec<Edit> = vec![
        ("modify a byte", Box::new(|p| p.write("src/lib.rs", &LIB.replace("a[0]", "a[1]")))),
        ("append whitespace", Box::new(|p| p.write("src/lib.rs", &format!("{LIB} ")))),
        ("append comment", Box::new(|p| p.write("src/lib.rs", &format!("{LIB}// c\n")))),
        ("add a file", Box::new(|p| p.write("src/extra.rs", "pub fn x() {}\n"))),
        ("add nested file", Box::new(|p| p.write("src/a/b.rs", "pub fn x() {}\n"))),
        (
            "add nested capabilities.rs (only top-level one is exempt)",
            Box::new(|p| p.write("src/sub/capabilities.rs", "pub fn x() {}\n")),
        ),
        ("add empty file", Box::new(|p| p.write("src/empty.rs", ""))),
        ("remove lib.rs entry point", Box::new(|p| p.write("src/lib.rs", "pub fn first(a: &[u8; 4]) -> u8 { a[0] }\n"))),
    ];
    for (name, edit) in edits {
        let p = pure_pkg("stale");
        emit(&p);
        verify(&p).unwrap();
        edit(&p);
        assert!(verify(&p).is_err(), "edit '{name}' must invalidate the certificate");
    }
}

#[test]
fn deleting_or_renaming_a_source_file_invalidates_the_certificate() {
    let p = package("del", LIB, &pure_manifest("del"));
    p.write("src/extra.rs", "pub fn x() {}\n");
    emit(&p);
    verify(&p).unwrap();
    fs::rename(p.path("src/extra.rs"), p.path("src/renamed.rs")).unwrap();
    assert!(verify(&p).unwrap_err().contains("source_digest mismatch"));
    fs::remove_file(p.path("src/renamed.rs")).unwrap();
    assert!(verify(&p).unwrap_err().contains("source_digest mismatch"));
}

#[test]
fn stale_manifest_is_detected() {
    let p = pure_pkg("stale_man");
    emit(&p);
    p.write("required_capabilities.json", &pure_manifest("tamper-pure").replace("hardening", "hardening!"));
    let e = verify(&p).unwrap_err();
    assert!(e.contains("manifest_digest mismatch") || e.contains("capabilities module"), "{e}");
    // Adding a capability also drifts the generated module.
    p.write("required_capabilities.json", &manifest("tamper-pure", &[("stdout", "write")], &[]));
    assert!(verify(&p).is_err());
}

#[test]
fn edited_generated_module_is_detected() {
    let p = pure_pkg("capmod");
    emit(&p);
    let original = p.read("src/capabilities.rs");
    for edit in [
        format!("{original}\npub fn backdoor() {{ std::fs::read(\"/etc/passwd\").ok(); }}\n"),
        original.replace("PureCapabilities", "Renamed"),
        String::new(),
    ] {
        p.write("src/capabilities.rs", &edit);
        assert!(verify(&p).unwrap_err().contains("capabilities module"));
    }
    fs::remove_file(p.path("src/capabilities.rs")).unwrap();
    assert!(verify(&p).unwrap_err().contains("missing or differs"));
}

#[test]
fn violations_introduced_outside_the_digest_still_fail_the_analysis() {
    // build.rs and root files are not in source_digest (documented) but are analysed.
    let p = pure_pkg("outside");
    emit(&p);
    p.write("build.rs", "fn main() { std::process::Command::new(\"sh\"); }\n");
    let e = verify(&p).unwrap_err();
    assert!(e.contains("static capability analysis failed") || e.contains("violations"), "{e}");
    fs::remove_file(p.path("build.rs")).unwrap();
    verify(&p).unwrap();
    // Cargo.toml changes are not covered by the digest.
    p.write("Cargo.toml", "[package]\nname = \"x\"\n[dependencies]\nserde = \"1\"\n");
    verify(&p).unwrap();
}

#[test]
fn crlf_conversion_of_sources_and_manifest_keeps_the_certificate_valid() {
    // Cross-platform checkout stability (VER23a §3.1): CRLF is normalised before hashing.
    let p = pure_pkg("crlf");
    let multiline = pure_manifest("tamper-pure").replace(", ", ",\n");
    p.write("required_capabilities.json", &multiline);
    emit(&p);
    p.write("src/lib.rs", &LIB.replace('\n', "\r\n"));
    p.write("required_capabilities.json", &multiline.replace('\n', "\r\n"));
    verify(&p).unwrap();
    // A lone CR is content, not a line ending: it must change the digest.
    p.write("src/lib.rs", &LIB.replace('\n', "\r"));
    assert!(verify(&p).is_err());
}

#[test]
fn hidden_directories_are_outside_the_digest_and_the_scan() {
    let p = pure_pkg("hidden");
    emit(&p);
    p.write("src/.hidden/x.rs", "pub fn x() { std::fs::read(\"a\"); }\n");
    verify(&p).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinks_under_src_are_an_error_not_silently_skipped() {
    let p = pure_pkg("symlink");
    emit(&p);
    std::os::unix::fs::symlink(p.path("src/lib.rs"), p.path("src/link.rs")).unwrap();
    assert!(compute_source_digest(p.dir()).unwrap_err().contains("symlink"));
    assert!(verify(&p).is_err());
}

#[test]
fn digest_framing_is_injective() {
    // Moving bytes between a path and its content must change the digest
    // (the old `path || 0x00 || bytes` sketch would not have separated these).
    let a = Pkg::with("fr_a", &[("src/ab.rs", "c")]);
    let b = Pkg::with("fr_b", &[("src/a.rs", "bc")]);
    assert_ne!(compute_source_digest(a.dir()).unwrap(), compute_source_digest(b.dir()).unwrap());
    // Two files vs one concatenated file.
    let two = Pkg::with("fr_c", &[("src/a.rs", "x"), ("src/b.rs", "y")]);
    let one = Pkg::with("fr_d", &[("src/a.rs", "xy")]);
    assert_ne!(compute_source_digest(two.dir()).unwrap(), compute_source_digest(one.dir()).unwrap());
    // File order does not matter, only content and path.
    let p1 = Pkg::with("fr_e", &[("src/a.rs", "1"), ("src/b.rs", "2")]);
    let p2 = Pkg::with("fr_f", &[("src/b.rs", "2"), ("src/a.rs", "1")]);
    assert_eq!(compute_source_digest(p1.dir()).unwrap(), compute_source_digest(p2.dir()).unwrap());
    // Top-level capabilities.rs is excluded; a same-named nested file is not.
    let base = Pkg::with("fr_g", &[("src/lib.rs", "x")]);
    let d0 = compute_source_digest(base.dir()).unwrap();
    base.write("src/capabilities.rs", "anything");
    assert_eq!(compute_source_digest(base.dir()).unwrap(), d0);
    base.write("src/m/capabilities.rs", "anything");
    assert_ne!(compute_source_digest(base.dir()).unwrap(), d0);
}

#[test]
fn manifest_digest_is_over_normalised_bytes_and_empty_when_absent() {
    let p = Pkg::with("md", &[("required_capabilities.json", "a\r\nb")]);
    let q = Pkg::with("md2", &[("required_capabilities.json", "a\nb")]);
    assert_eq!(compute_manifest_digest(p.dir()).unwrap(), compute_manifest_digest(q.dir()).unwrap());
    let none = Pkg::empty("md3");
    assert_eq!(
        compute_manifest_digest(none.dir()).unwrap(),
        coding_adventures_ca_capability_analyzer::sha256_hex(b"")
    );
    assert_eq!(normalize_crlf(b"a\r\nb\rc\r\n\r\n"), b"a\nb\rc\n\n".to_vec());
    assert_eq!(normalize_crlf(b"\r"), b"\r".to_vec());
    assert_eq!(normalize_crlf(b""), Vec::<u8>::new());
}

#[test]
fn missing_source_dir_and_missing_cert_are_errors() {
    let p = Pkg::with("nosrc", &[("required_capabilities.json", &pure_manifest("nosrc"))]);
    assert!(compute_source_digest(p.dir()).unwrap_err().contains("does not exist"));
    let q = pure_pkg("nocert");
    assert!(verify_certificate(q.dir(), &q.path("missing.json")).unwrap_err().contains("does not exist"));
}

#[test]
fn oversized_certificate_is_rejected() {
    let p = pure_pkg("bigcert");
    p.write("cert.json", &" ".repeat(1024 * 1024 + 1));
    assert!(verify(&p).unwrap_err().contains("exceeds maximum"));
}

// ---------------------------------------------------------------------------
// Generation refuses violating packages
// ---------------------------------------------------------------------------

#[test]
fn generation_refuses_every_class_of_violation() {
    let cases: &[(&str, &str, &str)] = &[
        ("undeclared capability", "pub fn f() { std::fs::read(\"x\").ok(); }", "CAP001"),
        ("unexcepted unsafe", "pub fn f() { unsafe { } }", "CAP002"),
        ("unexcepted asm", "pub fn f() { core::arch::asm!(\"nop\"); }", "CAP002"),
        ("include!", "include!(\"x.rs\");", "CAP003"),
        ("path attr", "#[path = \"x.rs\"] mod x;", "CAP003"),
        ("unknown std", "pub fn f() { std::rt::lang_start(); }", "CAP003"),
        ("hidden alias", "use std::{fs, io}; pub fn f() { let _ = fs::read(\"x\"); }", "CAP001"),
    ];
    for (name, lib, code) in cases {
        let p = package("refuse", lib, &pure_manifest("refuse"));
        let e = generate_certificate(p.dir()).unwrap_err();
        assert!(e.contains("refusing to emit certificate"), "{name}: {e}");
        assert!(e.contains(code), "{name}: expected {code} in {e}");
    }
}

#[test]
fn generation_allows_waived_constructs_and_records_the_exception() {
    let p = package(
        "waived",
        "pub fn f() { unsafe { } }\n",
        &manifest("waived", &[], &["unsafe_code"]),
    );
    let c = generate_certificate(p.dir()).unwrap();
    assert_eq!(c.banned_construct_exceptions.len(), 1);
    assert_eq!(c.banned_construct_exceptions[0].construct, "unsafe_code");
    assert_eq!(c.label, "proved");
}

#[test]
fn no_manifest_package_name_comes_from_directory() {
    let p = Pkg::with("nomanifest", &[("src/lib.rs", "pub fn f() {}\n")]);
    coding_adventures_ca_capability_analyzer::write_module_to_disk(p.dir()).unwrap();
    let c = generate_certificate(p.dir()).unwrap();
    assert!(c.package.starts_with("rust/ca_hard_nomanifest"));
    fs::write(p.path("cert.json"), c.to_json_string()).unwrap();
    verify(&p).unwrap();
}

// ---------------------------------------------------------------------------
// JSON writer
// ---------------------------------------------------------------------------

#[test]
fn json_writer_escapes_everything_and_round_trips() {
    assert_eq!(json_str("a\"b\\c\nd\re\tf\u{8}g\u{c}h\u{1}i\u{1f}"), "\"a\\\"b\\\\c\\nd\\re\\tf\\bg\\fh\\u0001i\\u001f\"");
    assert_eq!(json_str("caf\u{e9} \u{1f600}"), "\"caf\u{e9} \u{1f600}\"");
    assert_eq!(json_str(""), "\"\"");

    // A manifest justification with hostile characters must survive manifest -> cert -> parse.
    let nasty = "ends with backslash \\\\ and \\\"quote\\\" and \\n newline and tab \\t and unicode \\u00e9";
    let m = format!(
        r#"{{"version": 1, "package": "rust/nasty", "capabilities": [], "justification": "Valid justification text.",
        "banned_construct_exceptions": [{{"construct": "unsafe_code", "language": "rust", "justification": "{nasty}"}}]}}"#
    );
    let p = package("nasty", "pub fn f() { unsafe { } }\n", &m);
    let c = generate_certificate(p.dir()).unwrap();
    let text = c.to_json_string();
    let parsed = parse_certificate(&text).unwrap();
    assert_eq!(parsed, c);
    assert!(parsed.banned_construct_exceptions[0].justification.contains('\\'));
    assert!(parsed.banned_construct_exceptions[0].justification.contains('\n'));
    // The raw text contains no unescaped control characters.
    assert!(!text.trim_end().chars().any(|ch| (ch as u32) < 0x20 && ch != '\n'));
    fs::write(p.path("cert.json"), &text).unwrap();
    verify(&p).unwrap();
}

#[test]
fn canonical_output_shape_is_stable() {
    let p = effectful_pkg("shape");
    let text = generate_certificate(p.dir()).unwrap().to_json_string();
    assert!(text.starts_with("{\n  \"schema_version\": \"ver24-effect-cert-v1\",\n"));
    assert!(text.ends_with("  \"label\": \"proved\"\n}\n"));
    // Field order is the schema order.
    let order = [
        "schema_version", "package", "source_digest", "manifest_digest", "declared_effects",
        "entry_points", "unconditional_roots", "banned_construct_exceptions", "label",
    ];
    let mut last = 0;
    for k in order {
        let at = text.find(&format!("\"{k}\"")).unwrap();
        assert!(at >= last, "{k} out of order");
        last = at;
    }
}

// ---------------------------------------------------------------------------
// Entry point discovery
// ---------------------------------------------------------------------------

fn sigs(lib: &str) -> Vec<(String, String)> {
    coding_adventures_ca_capability_analyzer::parser::public_fn_signatures(lib).unwrap()
}

#[test]
fn entry_point_signatures_are_bracket_aware() {
    let s = sigs("pub fn a<const N: usize>(x: [u8; N], y: &[[u8; 2]; 3]) -> [u8; N] { x }");
    assert_eq!(s[0].0, "a");
    assert_eq!(s[0].1, "pub fn a<const N: usize>(x: [u8; N], y: &[[u8; 2]; 3]) -> [u8; N]");
}

#[test]
fn entry_point_discovery_covers_modifiers_and_skips_restricted_visibility() {
    let s = sigs(
        r#"
        pub fn plain() {}
        pub const fn c() -> u8 { 1 }
        pub unsafe fn u() {}
        pub async fn a() {}
        pub extern "C" fn e() {}
        pub const unsafe extern "C" fn all() {}
        pub(crate) fn hidden1() {}
        pub(super) fn hidden2() {}
        pub(in crate::x) fn hidden3() {}
        fn private() {}
        pub struct NotAFn;
        pub fn generic<T: Clone, U>(t: T, u: U) -> T where T: Copy { t }
        pub fn decl_only(x: u8) -> u8;
        impl S { pub fn method(&self) {} fn inner(&self) {} }
        mod m { pub fn nested() {} }
        "#,
    );
    let names: Vec<&str> = s.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["plain", "c", "u", "a", "e", "all", "generic", "decl_only", "method", "nested"]
    );
    assert_eq!(s[6].1, "pub fn generic<T: Clone, U>(t: T, u: U) -> T where T: Copy");
    assert_eq!(s[7].1, "pub fn decl_only(x: u8) -> u8");
}

#[test]
fn entry_point_discovery_ignores_strings_comments_and_keeps_duplicates() {
    let s = sigs(
        r##"
        // pub fn in_comment() {}
        /* pub fn in_block() {} */
        const S: &str = "pub fn in_string() {}";
        const R: &str = r#"pub fn in_raw() {}"#;
        pub fn dup() {}
        impl A { pub fn dup(&self, x: u8) {} }
        "##,
    );
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].1, "pub fn dup()");
    assert_eq!(s[1].1, "pub fn dup(&self, x: u8)");
}

#[test]
fn entry_point_discovery_without_lib_rs_is_empty_and_unreadable_is_an_error() {
    let p = Pkg::with("nolib", &[("src/main.rs", "pub fn x() {}")]);
    assert!(discover_public_entry_points(p.dir()).unwrap().is_empty());
    let q = Pkg::with("badlib", &[("src/lib.rs", "pub fn x() { /* unterminated")]);
    assert!(discover_public_entry_points(q.dir()).is_err());
}
