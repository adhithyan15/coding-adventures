//! # CWE-409 (decompression bomb) is derived, never excluded, for a decompressor
//!
//! The weakness matrix used to print one row for CWE-409 whatever the package:
//! *excluded*, "package implements no decompression or archive decoding
//! routines".  True for a LEB128 codec or a logic-gate library, false for
//! `rust/lzw`, whose `decompress` turns a 15 KB stream into 40 MB (SEC-002) and
//! is the first package of this repository that decompresses hostile input.  A
//! report that excluded the class for it would be a false assurance.
//!
//! The row is now derived from the evidence, and fails closed:
//!
//! ```text
//!   profile does not decompress  AND  document declares no `output` row
//!                                                      -> excluded (unchanged)
//!   otherwise, every operation with an `output` row has
//!     * a `heap` row beside it,
//!     * passing exhaustive + property + fuzz `<op>/output-bound` suites,
//!     * no observed peak over its bound                -> discharged (bounded)
//!   otherwise                                          -> open row: no report
//! ```
//!
//! The deciding facts are read from the Warrant document and the test manifest,
//! so neither the profile flag nor the report can talk the matrix into an
//! exclusion: an `output` row applies the class under any profile, and the
//! `lossless-compression` profile applies it even to a document with no row.
//! `--check` re-derives the same facts, so a report cannot exclude the class or
//! claim a discharge the manifest does not support.
//!
//! The end-to-end half, on the real `rust/lzw` manifests, is `test_lzw_report.rs`.

use std::path::{Path, PathBuf};

use coding_adventures_ca_assurance_report::evidence::DecompressionBounds;
use coding_adventures_ca_assurance_report::{
    build_weakness_matrix, generate_package_report, verify_evidence, AuditEvidence, AuditProfile,
    Evidence, PackageAuditContext, WeaknessRegistry, WeaknessRow, WeaknessStatus,
};

fn pkg_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

const SUITES: [&str; 3] = [
    "warrant/m/exhaustive/decompress/output-bound",
    "warrant/m/property/decompress/output-bound",
    "warrant/m/fuzz/decompress/output-bound",
];

/// A clean decompressor: one operation, its three suites, an observed peak
/// inside the bound.
fn clean_bounds() -> DecompressionBounds {
    DecompressionBounds {
        ops: vec!["decompress".into()],
        ids: SUITES.iter().map(|s| s.to_string()).collect(),
        unproven: vec![],
        cases: 21_000,
        max_observed_heap: 4_000_000,
        bound_at_max: 67_000_000,
    }
}

fn row409(profile: AuditProfile, d: DecompressionBounds) -> Result<WeaknessRow, String> {
    let ctx = PackageAuditContext {
        package_id: "rust/example".into(),
        is_safe_rust: true,
        declared_capabilities: vec![],
        has_wir_timing_proof: false,
        has_instruction_benchmarks: false,
        has_zero_alloc_proof: false,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: AuditEvidence {
            // Acknowledge the other open rows so only CWE-409 decides the outcome.
            step_not_live: vec!["warrant/m/proof/op/steps-bound".into()],
            step_unproven: vec!["warrant/m/proof/op/steps-bound".into()],
            not_live_note: "front end refused".into(),
            no_secret_operand_ids: vec!["warrant-spec::all-operands-public".into()],
            profile,
            decompression: d,
            ..AuditEvidence::default()
        },
    };
    build_weakness_matrix(&WeaknessRegistry::embedded(), &ctx)
        .map_err(|e| e.to_string())
        .map(|rows| rows.into_iter().find(|r| r.cwe == 409).expect("CWE-409 row"))
}

// ---------------------------------------------------------------------------
// The matrix rule: pass cases
// ---------------------------------------------------------------------------

#[test]
fn a_package_that_does_not_decompress_keeps_the_exclusion() {
    for profile in [
        AuditProfile::Comparison,
        AuditProfile::FieldArithmetic,
        AuditProfile::VarintCodec,
        AuditProfile::BooleanLogic,
    ] {
        let row = row409(profile, DecompressionBounds::default()).expect("excluded is a row");
        assert_eq!(row.status, WeaknessStatus::Excluded, "{profile:?}");
        assert!(row.reason.contains("no decompression"), "{}", row.reason);
        assert_eq!(row.evidence, vec!["unverified::ast-audit::no-compression-routines".to_string()]);
    }
}

#[test]
fn a_decompressor_with_complete_bound_evidence_is_discharged_and_cites_the_suites() {
    let row = row409(AuditProfile::LosslessCompression, clean_bounds()).expect("derived");
    assert_eq!(row.status, WeaknessStatus::Discharged);
    assert_eq!(row.approved, None);
    assert_eq!(row.evidence, clean_bounds().ids, "cites exactly the output-bound suites");
    assert!(row.reason.starts_with("discharged for bounded inputs"), "{}", row.reason);
    assert!(row.reason.contains("not proved for every input"), "the discharge is bounded, and says so");
    assert!(row.reason.contains("4000000") && row.reason.contains("67000000"), "{}", row.reason);
    assert!(!row.reason.contains("no decompression"), "never the exclusion sentence");
}

#[test]
fn an_output_row_applies_the_class_under_any_profile() {
    // The wrong profile cannot hide the amplifier: the document's own `output`
    // row is enough, and with clean evidence the row is the discharge.
    let row = row409(AuditProfile::Comparison, clean_bounds()).expect("derived");
    assert_eq!(row.status, WeaknessStatus::Discharged);
    // ... and with incomplete evidence it is open, not excluded.
    let mut d = clean_bounds();
    d.unproven.push("decompress: no fuzz suite for decompress/output-bound".into());
    assert!(row409(AuditProfile::Comparison, d).is_err());
}

// ---------------------------------------------------------------------------
// The matrix rule: fail-closed cases
// ---------------------------------------------------------------------------

#[test]
fn the_decompression_profile_is_never_excluded_even_without_an_output_row() {
    let err = row409(AuditProfile::LosslessCompression, DecompressionBounds::default())
        .expect_err("a decompressor whose document bounds no output is an open row");
    assert!(err.contains("CWE-409") && err.contains("no `output` bound row"), "{err}");
}

#[test]
fn any_unproven_fact_leaves_the_row_open() {
    for what in [
        "decompress: no heap bound row beside the output row",
        "decompress: no exhaustive suite for decompress/output-bound",
        "decompress: warrant/m/fuzz/decompress/output-bound is fail",
        "decompress: warrant/m/property/decompress/output-bound peaked at 70000000 bytes over its 67000000-byte bound",
        "decompress: no entry was observed against the heap row",
    ] {
        let mut d = clean_bounds();
        d.unproven.push(what.into());
        let err = row409(AuditProfile::LosslessCompression, d).expect_err(what);
        assert!(err.contains("CWE-409") && err.contains(what), "{err}");
    }
}

// ---------------------------------------------------------------------------
// The profile, and the packages that keep the exclusion
// ---------------------------------------------------------------------------

#[test]
fn the_lossless_compression_profile_parses_and_unknown_profiles_list_it() {
    assert_eq!("lossless-compression".parse::<AuditProfile>(), Ok(AuditProfile::LosslessCompression));
    assert!(AuditProfile::LosslessCompression.decompresses());
    assert!(!AuditProfile::Comparison.decompresses());
    let err = "zip".parse::<AuditProfile>().unwrap_err();
    assert!(err.contains("lossless-compression"), "{err}");
}

#[test]
fn check_accepts_the_old_exclusion_for_a_package_with_no_output_row() {
    // The four earlier adopters keep their reports byte for byte: their
    // documents declare no `output` row, so the exclusion is still true.
    for (name, profile, version) in [
        ("wasm-leb128", AuditProfile::VarintCodec, "0.2.0"),
        ("logic-gates", AuditProfile::BooleanLogic, "0.1.0"),
    ] {
        let ev = Evidence::load(&pkg_dir(name)).unwrap();
        assert!(ev.decompression_bounds().ops.is_empty(), "{name} has no `output` row");
        let report = generate_package_report(
            &format!("rust/{name}"),
            version,
            "c0ffee",
            &WeaknessRegistry::embedded(),
            &ev,
            profile,
        )
        .unwrap();
        let r409 = report.weaknesses.iter().find(|w| w.cwe == 409).unwrap();
        assert_eq!(r409.status, WeaknessStatus::Excluded, "{name}");
        verify_evidence(&report, &ev).expect("the exclusion verifies where it is true");
    }
}
