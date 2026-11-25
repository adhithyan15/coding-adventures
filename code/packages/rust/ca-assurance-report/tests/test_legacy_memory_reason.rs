//! Pins the legacy memory-reason shim in `verify_rederivation`.
//!
//! Reports generated before the memory-class wording was corrected carry the
//! sentence "... no FFI reachability ...".  `--check` accepts exactly that
//! sentence on a safe-Rust memory-class row and nothing else, so the
//! unregenerated reports keep passing without the shim becoming a loophole.

use std::path::Path;

use coding_adventures_ca_assurance_report::matrix::{
    LEGACY_SAFE_RUST_MEMORY_REASON, SAFE_RUST_MEMORY_REASON,
};
use coding_adventures_ca_assurance_report::{
    generate_package_report, verify_rederivation, AssuranceReport, Evidence, EvidenceCheckError as E,
    WeaknessRegistry, WeaknessRow, WeaknessStatus,
};

const MEMORY_CWES: [u32; 6] = [416, 787, 125, 415, 476, 190];

fn fixture() -> (AssuranceReport, Evidence) {
    // logic-gates is safe Rust with no unsafe exception, so its memory rows are excluded.
    let pkg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../logic-gates");
    let ev = Evidence::load(&pkg).unwrap();
    let r = generate_package_report("rust/logic-gates", "0.2.0", "c0ffee", &WeaknessRegistry::embedded(), &ev)
        .expect("report derives");
    (r, ev)
}

fn row(r: &mut AssuranceReport, cwe: u32) -> &mut WeaknessRow {
    r.weaknesses.iter_mut().find(|w| w.cwe == cwe).unwrap()
}

#[test]
fn the_fresh_wording_is_the_new_sentence() {
    let (r, ev) = fixture();
    verify_rederivation(&r, &ev, &WeaknessRegistry::embedded()).unwrap();
    for cwe in MEMORY_CWES {
        let w = r.weaknesses.iter().find(|w| w.cwe == cwe).unwrap();
        assert_eq!(w.reason, SAFE_RUST_MEMORY_REASON);
        assert_eq!(w.status, WeaknessStatus::Excluded);
    }
    assert_ne!(SAFE_RUST_MEMORY_REASON, LEGACY_SAFE_RUST_MEMORY_REASON);
}

#[test]
fn the_legacy_sentence_is_accepted_on_every_safe_rust_memory_row() {
    let (mut r, ev) = fixture();
    for cwe in MEMORY_CWES {
        row(&mut r, cwe).reason = LEGACY_SAFE_RUST_MEMORY_REASON.into();
    }
    verify_rederivation(&r, &ev, &WeaknessRegistry::embedded()).expect("legacy wording stays valid");
}

#[test]
fn the_legacy_sentence_is_refused_on_a_non_memory_row() {
    let (mut r, ev) = fixture();
    row(&mut r, 407).reason = LEGACY_SAFE_RUST_MEMORY_REASON.into();
    let e = verify_rederivation(&r, &ev, &WeaknessRegistry::embedded()).unwrap_err();
    assert!(matches!(&e, E::EvidenceMismatch(m) if m.contains("CWE-407") && m.contains("reason text")), "{e}");
}

#[test]
fn the_legacy_sentence_plus_any_other_changed_field_is_refused() {
    let reg = WeaknessRegistry::embedded();

    let (mut r, ev) = fixture();
    let w = row(&mut r, 416);
    w.reason = LEGACY_SAFE_RUST_MEMORY_REASON.into();
    w.status = WeaknessStatus::Discharged;
    assert!(verify_rederivation(&r, &ev, &reg).is_err(), "status");

    let (mut r, ev) = fixture();
    let w = row(&mut r, 787);
    w.reason = LEGACY_SAFE_RUST_MEMORY_REASON.into();
    w.evidence = vec!["effect-certificate::proved".into()];
    assert!(verify_rederivation(&r, &ev, &reg).is_err(), "evidence ids");

    let (mut r, ev) = fixture();
    let w = row(&mut r, 125);
    w.reason = LEGACY_SAFE_RUST_MEMORY_REASON.into();
    w.name = "renamed".into();
    assert!(verify_rederivation(&r, &ev, &reg).is_err(), "name");
}

#[test]
fn arbitrary_reason_text_is_refused_even_a_near_miss_of_the_legacy_sentence() {
    let reg = WeaknessRegistry::embedded();
    for text in [
        "a nicer sentence".to_string(),
        format!("{LEGACY_SAFE_RUST_MEMORY_REASON} "),
        LEGACY_SAFE_RUST_MEMORY_REASON.replace("safe Rust", "safe rust"),
    ] {
        let (mut r, ev) = fixture();
        row(&mut r, 476).reason = text.clone();
        let e = verify_rederivation(&r, &ev, &reg).unwrap_err();
        assert!(matches!(&e, E::EvidenceMismatch(m) if m.contains("reason text")), "{text}: {e}");
    }
}
