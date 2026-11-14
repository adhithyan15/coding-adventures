//! Audit profiles are data (VER04h section 9, item 22).
//!
//! The profile of a package is its own `assurance-profile.json`.  These tests pin
//! the fail-closed behaviour: a profile file that is edited, missing, claims
//! applicability the evidence contradicts, carries an unknown key, or declares a
//! decompressor's CWE-409 excluded is refused, by `--check` (`verify_evidence` and
//! `verify_rederivation`) or by the loader.

mod common;

use coding_adventures_ca_assurance_report::{
    generate_package_report, verify_evidence, verify_rederivation, AssuranceReport, AuditProfile, ProfileKind,
    Evidence, EvidenceCheckError as E, WeaknessRegistry,
};
use common::Sandbox;

const PROFILE: &str = "assurance-profile.json";

fn report(ev: &Evidence) -> AssuranceReport {
    generate_package_report("rust/ct-compare", "0.1.0", "c0ffee", &WeaknessRegistry::embedded(), ev)
        .expect("report derives")
}

fn registry() -> WeaknessRegistry {
    WeaknessRegistry::embedded()
}

#[test]
fn a_report_verifies_and_rederives_against_its_own_profile() {
    let sb = Sandbox::new("prof-ok");
    let ev = Evidence::load(&sb.pkg).unwrap();
    let r = report(&ev);
    assert_eq!(r.profile_sha256, ev.profile.sha256);
    verify_evidence(&r, &ev).unwrap();
    verify_rederivation(&r, &ev, &registry()).unwrap();
}

#[test]
fn an_edited_profile_file_is_stale_evidence() {
    let sb = Sandbox::new("prof-edit");
    let r = report(&Evidence::load(&sb.pkg).unwrap());
    // Same bytes length class, different wording: only the digest can notice.
    sb.edit(PROFILE, "arbitrary buffer lengths", "arbitrary  buffer lengths");
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&r, &ev), Err(E::StaleEvidence(m)) if m.contains("assurance-profile.json")));
}

#[test]
fn a_missing_profile_file_fails_the_load() {
    let sb = Sandbox::new("prof-missing");
    std::fs::remove_file(sb.pkg.join(PROFILE)).unwrap();
    let e = Evidence::load(&sb.pkg).unwrap_err();
    assert!(e.0.contains("assurance-profile.json"), "{}", e.0);
}

#[test]
fn an_unknown_key_is_refused_at_every_level() {
    let sb = Sandbox::new("prof-key");
    let good = sb.read(PROFILE);
    for (from, to) in [
        ("\"kind\"", "\"extra\": 1,\n  \"kind\""),
        ("\"rows\": {", "\"rows\": {\n    \"400\": {\"applicability\": \"excluded\", \"reason\": \"x\"},"),
        ("\"applicability\": \"excluded\",", "\"applicability\": \"excluded\", \"approved\": true,"),
    ] {
        let bytes = good.replacen(from, to, 1);
        assert_ne!(bytes, good, "{from}");
        let e = AuditProfile::from_bytes(bytes.as_bytes()).unwrap_err();
        assert!(e.0.contains("unknown key") || e.0.contains("cannot be declared"), "{from}: {}", e.0);
    }
}

#[test]
fn applicability_other_than_excluded_and_a_missing_row_are_refused() {
    let sb = Sandbox::new("prof-appl");
    let good = sb.read(PROFILE);
    let e = AuditProfile::from_bytes(good.replacen("\"excluded\"", "\"discharged\"", 1).as_bytes()).unwrap_err();
    assert!(e.0.contains("only declare `excluded`"), "{}", e.0);
    let no407 = good.replace("\"407\"", "\"4070\"");
    assert!(AuditProfile::from_bytes(no407.as_bytes()).is_err());
    let e = AuditProfile::from_bytes(good.replace("\"kind\": \"transform\"", "\"kind\": \"zip\"").as_bytes()).unwrap_err();
    assert!(e.0.contains("unknown kind"), "{}", e.0);
}

#[test]
fn a_reserved_marker_or_control_text_in_a_reason_is_refused() {
    let sb = Sandbox::new("prof-reason");
    let good = sb.read(PROFILE);
    for bad in ["[attestation not machine-checked by x]", "two\\nlines", "caf\u{e9}"] {
        let e = AuditProfile::from_bytes(good.replacen("arbitrary buffer lengths", bad, 1).as_bytes());
        assert!(e.is_err(), "{bad:?}");
    }
}

#[test]
fn a_decompressor_cannot_declare_cwe_409_excluded() {
    let sb = Sandbox::new("prof-409");
    let good = sb.read(PROFILE).replace("\"transform\"", "\"decompressor\"");
    let with409 = good.replacen(
        "\"rows\": {",
        "\"rows\": {\n    \"409\": {\"applicability\": \"excluded\", \"reason\": \"no decompression here\"},",
        1,
    );
    let e = AuditProfile::from_bytes(with409.as_bytes()).unwrap_err();
    assert!(e.0.contains("decompressor") && e.0.contains("CWE-409"), "{}", e.0);
}

#[test]
fn a_profile_claiming_what_the_evidence_contradicts_cannot_generate_a_report() {
    // CWE-407 excluded, but one operation has no `steps` bound row.
    let sb = Sandbox::new("prof-contra");
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    ev.spec.ops[0].has_steps_bound = false;
    let e = generate_package_report("rust/ct-compare", "0.1.0", "c", &registry(), &ev).unwrap_err();
    assert!(e.to_string().contains("CWE-407") && e.to_string().contains("steps"), "{e}");
    // CWE-326/327 excluded, but a `rand:` capability is declared.
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    ev.cert.declared_effects.push("rand:bytes:*".into());
    let e = generate_package_report("rust/ct-compare", "0.1.0", "c", &registry(), &ev).unwrap_err();
    assert!(e.to_string().contains("rand"), "{e}");
    // CWE-409 declared excluded for a document that bounds an output.
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    let with409 = sb.read(PROFILE).replacen(
        "\"rows\": {",
        "\"rows\": {\n    \"409\": {\"applicability\": \"excluded\", \"reason\": \"no decompression here\"},",
        1,
    );
    ev.profile = AuditProfile::from_bytes(with409.as_bytes()).unwrap();
    ev.spec.ops[0].has_output_bound = true;
    let e = generate_package_report("rust/ct-compare", "0.1.0", "c", &registry(), &ev).unwrap_err();
    assert!(e.to_string().contains("CWE-409") && e.to_string().contains("output"), "{e}");
}

#[test]
fn edited_reason_text_and_another_profile_fail_the_rederivation() {
    let sb = Sandbox::new("prof-rederive");
    let ev = Evidence::load(&sb.pkg).unwrap();
    let mut r = report(&ev);
    // AR-001: prose edited in the report, digests untouched.
    r.weaknesses.iter_mut().find(|w| w.cwe == 407).unwrap().reason = "a nicer sentence".into();
    verify_evidence(&r, &ev).expect("structured facts alone do not notice prose");
    let e = verify_rederivation(&r, &ev, &registry()).unwrap_err();
    assert!(matches!(&e, E::EvidenceMismatch(m) if m.contains("CWE-407") && m.contains("reason text")), "{e}");

    // A report generated under another profile, with that profile's real digest and name.
    let other = Sandbox::new("prof-other");
    let other_ev = {
        let o = std::fs::read_to_string(other.pkg.join(PROFILE)).unwrap();
        let mut e = Evidence::load(&other.pkg).unwrap();
        e.profile = AuditProfile::from_bytes(o.replace("arbitrary buffer lengths", "other words").as_bytes()).unwrap();
        e
    };
    let r2 = report(&other_ev);
    let e = verify_rederivation(&r2, &ev, &registry()).unwrap_err();
    assert!(matches!(e, E::EvidenceMismatch(_)));
}

#[test]
fn the_reason_text_is_the_profiles_and_still_carries_the_unverified_marker() {
    let sb = Sandbox::new("prof-words");
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    let bytes = sb.read(PROFILE).replacen("arbitrary buffer lengths", "bespoke wording", 1);
    ev.profile = AuditProfile::from_bytes(bytes.as_bytes()).unwrap();
    let r = report(&ev);
    let row = r.weaknesses.iter().find(|w| w.cwe == 326).unwrap();
    assert!(row.reason.starts_with("bespoke wording"), "{}", row.reason);
    assert!(row.reason.ends_with("[attestation not machine-checked by any tool]"), "{}", row.reason);
}

// ---- the BUILD pin binds the kind as well as the name -----------------------
//
// `--profile NAME` alone pins only a label; `name` and `kind` are both author data, so an author
// could keep `name: lossless-compression`, flip `kind` to `transform`, regenerate, and pass.
// `--profile-kind` binds the kind that matters to the security rule.

fn run(sb: &Sandbox, args: &[&str]) -> (bool, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_ca-assurance-report-rs"))
        .current_dir(&sb.pkg)
        .args(args)
        .output()
        .unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout))
}

const PINS: [&str; 4] = ["--profile", "lossless-compression", "--profile-kind", "decompressor"];

fn with(base: &[&str], extra: &[&str]) -> Vec<String> {
    base.iter().chain(extra).map(|s| s.to_string()).collect()
}

fn run_s(sb: &Sandbox, args: Vec<String>) -> (bool, String) {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    run(sb, &a)
}

#[test]
fn verify_pin_holds_name_and_kind_to_the_file() {
    let p = AuditProfile::load(&Sandbox::for_package("pin-lib", "lzw").pkg).unwrap();
    assert!(p.verify_pin(None, None).is_ok());
    assert!(p.verify_pin(Some("lossless-compression"), Some(ProfileKind::Decompressor)).is_ok());
    assert!(p.verify_pin(Some("other"), None).is_err());
    let e = p.verify_pin(None, Some(ProfileKind::Transform)).unwrap_err();
    assert!(e.0.contains("kind") && e.0.contains("decompressor"), "{}", e.0);
}

#[test]
fn lzw_profile_edited_to_transform_with_the_name_kept_fails_check_under_the_build_pin() {
    let sb = Sandbox::for_package("pin-cli", "lzw");
    let honest = with(&["--generate", "--package-dir", ".", "--commit", "c0ffee"], &PINS);
    assert!(run_s(&sb, honest).0, "honest generation under the pins");
    let check = with(&["--check", "assurance_report.json", "--allow-unverified"], &PINS);
    let (ok, out) = run_s(&sb, check.clone());
    assert!(ok, "honest check under the pins: {out}");

    // The author flips the kind, keeps the name, and regenerates with only the old name pin.
    sb.edit(PROFILE, "\"kind\": \"decompressor\"", "\"kind\": \"transform\"");
    let regen = ["--generate", "--package-dir", ".", "--commit", "c0ffee", "--profile", "lossless-compression"];
    assert!(run(&sb, &regen).0, "the old name-only pin is satisfied: that was the hole");
    let (ok, out) = run_s(&sb, check);
    assert!(!ok && out.contains("kind"), "the BUILD pin must refuse it: {out}");
    // Generation under the kind pin refuses too.
    let (ok, out) = run_s(&sb, with(&["--generate", "--package-dir", ".", "--commit", "c0ffee"], &PINS));
    assert!(!ok && out.contains("kind"), "{out}");
}

#[test]
fn a_kind_mismatch_fails_check_in_either_direction() {
    let sb = Sandbox::for_package("pin-kind", "lzw");
    assert!(run_s(&sb, with(&["--generate", "--package-dir", ".", "--commit", "c0ffee"], &PINS)).0);
    let (ok, out) = run(&sb, &["--check", "assurance_report.json", "--allow-unverified", "--profile-kind", "transform"]);
    assert!(!ok && out.contains("kind"), "{out}");
    let (ok, out) = run(&sb, &["--check", "assurance_report.json", "--allow-unverified", "--profile-kind", "bogus"]);
    assert!(!ok, "an unknown kind is a usage error: {out}");
}
