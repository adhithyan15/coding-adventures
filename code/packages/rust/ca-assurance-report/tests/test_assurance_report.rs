//! # Test Suite for ca-assurance-report (VER01 / VER15 / VER-P028)

#![deny(unsafe_code)]

mod common;

use coding_adventures_ca_assurance_report::checker::{
    verify_assurance_report, CheckError, CheckVerdict,
};
use coding_adventures_ca_assurance_report::{generate_ct_compare_report, AuditProfile};
use coding_adventures_ca_assurance_report::json::JsonValue;
use coding_adventures_ca_assurance_report::matrix::{
    build_weakness_matrix, AuditEvidence, PackageAuditContext, WeaknessStatus,
};
use coding_adventures_ca_assurance_report::Evidence;
use coding_adventures_ca_assurance_report::registry::WeaknessRegistry;
use coding_adventures_ca_assurance_report::report::AssuranceReport;

/// The real, checked-in evidence of `rust/ct-compare` (VER04h): the report is
/// derived from it, never written by hand.
fn real_evidence() -> Evidence {
    Evidence::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ct-compare"))
        .expect("ct-compare evidence manifests must load")
}

/// A fully live report has discharged every obligation, so no claim carries an
/// unchecked-row assumption any more (a `pass` verdict may not carry one).
/// Catalogue ids spell them `wir.bound.*-unchecked`.
fn discharge_unchecked(report: &mut AssuranceReport) {
    for c in &mut report.claims {
        if let Some(p) = c.proof.as_mut() {
            p.assumptions.retain(|a| !a.ends_with(":unchecked") && !a.ends_with("-unchecked"));
        }
    }
}

/// Evidence ids for tests that exercise the matrix builder directly.
fn evid() -> AuditEvidence {
    AuditEvidence {
        unsafe_exceptions: vec![],
        step_bound_ids: vec!["warrant/m/proof/op/steps-bound".into()],
        step_bound_label: "bounded(8)".into(),
        alloc_ids: vec!["bench/a".into()],
        ct_proof_ids: vec!["warrant/m/proof/op/constant-time".into()],
        ct_proof_label: "bounded(8)".into(),
        bench_ids: vec!["bench/a".into()],
        hw_counters: "not-live".into(),
        oob_ids: vec!["warrant/m/proof/op/constant-time".into()],
        oob_label: "bounded(8)".into(),
        // Profiles are data: the ct-compare adopter's own file supplies the static sentences.
        profile: AuditProfile::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ct-compare"))
            .expect("ct-compare profile loads"),
        all_ops_step_bounded: true,
        ..AuditEvidence::default()
    }
}

#[test]
fn test_registry_parsing_and_invariants() {
    let registry = WeaknessRegistry::embedded();
    assert_eq!(registry.version, 1);
    assert_eq!(registry.classes.len(), 26);

    let cwe_208 = registry.find_class(208).expect("CWE-208 must exist");
    assert_eq!(cwe_208.name, "Observable Timing Discrepancy");
    assert_eq!(cwe_208.family, "cryptography");
    assert_eq!(cwe_208.minimum_proof_label, "proved-to-gate-level");
    assert!(cwe_208.safe_subsets.is_empty());

    let cwe_416 = registry.find_class(416).expect("CWE-416 must exist");
    assert_eq!(cwe_416.name, "Use After Free");
    assert_eq!(cwe_416.family, "memory_safety");
    assert!(cwe_416.safe_subsets.contains(&"safe_rust".to_string()));

    let cwe_400 = registry.find_class(400).expect("CWE-400 must exist");
    assert_eq!(cwe_400.family, "parsing_dos");

    let cwe_78 = registry.find_class(78).expect("CWE-78 must exist");
    assert_eq!(cwe_78.family, "injection");

    let cwe_362 = registry.find_class(362).expect("CWE-362 must exist");
    assert_eq!(cwe_362.family, "concurrency");

    let cwe_1321 = registry.find_class(1321).expect("CWE-1321 must exist");
    assert_eq!(cwe_1321.family, "deserialization");
}

#[test]
fn test_weakness_matrix_generation_safe_rust() {
    let registry = WeaknessRegistry::embedded();
    let ctx = PackageAuditContext {
        package_id: "rust/ct-compare".into(),
        is_safe_rust: true,
        declared_capabilities: vec![],
        has_wir_timing_proof: true,
        has_instruction_benchmarks: true,
        has_zero_alloc_proof: true,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: evid(),
    };

    let matrix = build_weakness_matrix(&registry, &ctx).expect("matrix generation must succeed");
    assert_eq!(matrix.len(), 26);

    let row_416 = matrix.iter().find(|r| r.cwe == 416).unwrap();
    assert_eq!(row_416.status, WeaknessStatus::Excluded);
    assert!(row_416.reason.contains("safe Rust"));

    let row_208 = matrix.iter().find(|r| r.cwe == 208).unwrap();
    assert_eq!(row_208.status, WeaknessStatus::Exemption);
    assert_eq!(row_208.approved, Some(false));
    assert!(row_208.reason.contains("gate-level"));

    let row_400 = matrix.iter().find(|r| r.cwe == 400).unwrap();
    assert_eq!(row_400.status, WeaknessStatus::Discharged);
}

#[test]
fn test_ct_compare_report_generation_and_self_check() {
    let registry = WeaknessRegistry::embedded();
    let report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence())
        .expect("report generation must succeed");

    assert_eq!(report.package, "rust/ct-compare");
    assert_eq!(report.verdict, "report-only");
    assert_eq!(report.rungs.e2, "not-live");
    assert_eq!(report.weaknesses.len(), 26);
    // CWE-208 plus the four memory-safety classes the declared unsafe exception
    // leaves without evidence (AF-22): every one unapproved.
    assert_eq!(report.exemptions.len(), 5);
    assert!(report.exemptions.iter().all(|e| !e.approved));
    assert!(report.exemptions.iter().any(|e| e.cwe == 208));
    assert!(report.public_symbols_without_claims.is_empty());

    let verdict = verify_assurance_report(&report, &registry).expect("verification must succeed");
    match verdict {
        CheckVerdict::ReportOnly(reason) => {
            assert!(reason.contains("not-live"));
            assert!(reason.contains("unapproved"));
        }
        CheckVerdict::Pass => panic!("expected ReportOnly verdict"),
    }
}

#[test]
fn test_sabotage_non_live_rung_marked_as_pass() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Sabotage: attempt to mark verdict as pass while E2 is not-live
    report.verdict = "pass".into();

    let err = verify_assurance_report(&report, &registry).unwrap_err();
    match err {
        CheckError::IllegalPassVerdict(msg) => {
            assert!(msg.contains("not-live") || msg.contains("unapproved"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn test_sabotage_unapproved_exemption_marked_as_pass() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Make all rungs pass, but leave exemption unapproved
    report.rungs.e2 = "pass".into();
    report.verdict = "pass".into();

    let err = verify_assurance_report(&report, &registry).unwrap_err();
    match err {
        CheckError::IllegalPassVerdict(msg) => {
            assert!(msg.contains("unapproved exemptions") || msg.contains("unapproved"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn test_sabotage_missing_weakness_row() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Sabotage: omit CWE-787 from the matrix
    report.weaknesses.retain(|r| r.cwe != 787);

    let err = verify_assurance_report(&report, &registry).unwrap_err();
    match err {
        CheckError::IncompleteMatrix(msg) => {
            assert!(msg.contains("787"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn test_sabotage_duplicate_weakness_row() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Sabotage: duplicate first row
    let dup = report.weaknesses[0].clone();
    report.weaknesses.push(dup);

    let err = verify_assurance_report(&report, &registry).unwrap_err();
    match err {
        CheckError::IncompleteMatrix(msg) => {
            assert!(msg.contains("duplicate"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn test_sabotage_unauthorized_cwe208_exclusion() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Sabotage: illegally mark CWE-208 as excluded
    for w in &mut report.weaknesses {
        if w.cwe == 208 {
            w.status = WeaknessStatus::Excluded;
            w.reason = "falsified exclusion".into();
        }
    }

    let err = verify_assurance_report(&report, &registry).unwrap_err();
    match err {
        CheckError::InvalidExclusion(msg) => {
            assert!(msg.contains("208"));
            assert!(msg.contains("no safe subset"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn test_sabotage_uncovered_public_symbols() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Sabotage: add an exported symbol without claims
    report
        .public_symbols_without_claims
        .push("unclaimed_secret_fn".into());

    let err = verify_assurance_report(&report, &registry).unwrap_err();
    match err {
        CheckError::UncoveredSymbols(syms) => {
            assert_eq!(syms, vec!["unclaimed_secret_fn"]);
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn test_sabotage_invalid_package_format() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Sabotage: bad package prefix
    report.package = "npm/ct-compare".into();

    let err = verify_assurance_report(&report, &registry).unwrap_err();
    match err {
        CheckError::InvalidPackage(msg) => {
            assert!(msg.contains("rust/"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn test_json_parsing_and_pretty_printing() {
    let json_str = r#"{
  "name": "testing",
  "count": 42,
  "negative": -10,
  "fraction": 3.1415,
  "flag": true,
  "escaped": "hello \"world\"\n\t\\",
  "nested": {
    "items": [1, 2, 3]
  }
}"#;

    let val = JsonValue::parse(json_str).expect("valid JSON must parse");
    assert_eq!(val.get("name").and_then(|v| v.as_str()), Some("testing"));
    assert_eq!(val.get("count").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(val.get("negative").and_then(|v| v.as_i64()), Some(-10));
    assert_eq!(val.get("flag").and_then(|v| v.as_bool()), Some(true));

    let pretty = val.to_pretty_string(2);
    let val2 = JsonValue::parse(&pretty).expect("pretty-printed JSON must parse");
    assert_eq!(val, val2);

    // Negative JSON parsing tests
    assert!(JsonValue::parse("{ bad }").is_err());
    assert!(JsonValue::parse("[1, 2,").is_err());
    assert!(JsonValue::parse(r#" "unterminated "#).is_err());
}

#[test]
fn test_roundtrip_serialization() {
    let registry = WeaknessRegistry::embedded();
    let report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    let json_str = report.to_json_string();

    let deserialized =
        AssuranceReport::from_json_str(&json_str).expect("deserialization must succeed");
    assert_eq!(report, deserialized);
}

#[test]
fn test_cli_execution_and_flags() {
    let bin = env!("CARGO_BIN_EXE_ca-assurance-report-rs");

    // 1. --help
    let out = std::process::Command::new(bin)
        .arg("--help")
        .output()
        .expect("binary must run");
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stderr);
    assert!(help.contains("Usage:"));

    // 2. Unknown option fails with exit code 2
    let out = std::process::Command::new(bin)
        .arg("--unknown-flag")
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(2));

    // 3. Missing argument to --package-dir fails with exit code 2
    let out = std::process::Command::new(bin)
        .arg("--package-dir")
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(2));

    // 4. Missing argument to --check fails with exit code 2
    let out = std::process::Command::new(bin)
        .arg("--check")
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(2));

    // 5. Non-existent file to --check fails with exit code 2
    let out = std::process::Command::new(bin)
        .args(["--check", "/no/such/assurance_report.json"])
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(2));

    // 6. Generating needs evidence manifests: an empty directory is an error.
    let empty = std::env::temp_dir().join(format!("test_ca_assurance_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&empty);
    let out = std::process::Command::new(bin)
        .args(["--package-dir", empty.to_str().unwrap(), "--generate"])
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&empty);

    // 7. Generate from a copy of the real evidence, then check it.
    let sandbox = common::Sandbox::new("cli");
    let tmp = sandbox.pkg.clone();
    let out = std::process::Command::new(bin)
        .args([
            "--package-dir",
            tmp.to_str().unwrap(),
            "--generate",
            "--commit",
            "test_commit_sha",
        ])
        .output()
        .expect("binary must run");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Generated assurance report"));

    let report_file = tmp.join("assurance_report.json");
    assert!(report_file.is_file());

    // The derived report cites `unverified::` attestations: refused by default.
    let out = std::process::Command::new(bin)
        .args(["--check", report_file.to_str().unwrap()])
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--allow-unverified") && stderr.contains("unverified::"), "{stderr}");
    // ... and accepted, visibly, with the explicit flag.
    let out = std::process::Command::new(bin)
        .args(["--check", report_file.to_str().unwrap(), "--allow-unverified"])
        .output()
        .expect("binary must run");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ASSURANCE VERDICT: REPORT-ONLY"));
    assert!(stdout.contains("accepted by --allow-unverified"));
    // Every accepted id is named, and the flag is marked temporary (WD-008).
    assert!(stdout.contains("WD-008"), "{stdout}");
    let named = stdout.lines().filter(|l| l.trim_start().starts_with("accepted unverified: unverified::")).count();
    let announced: usize = stdout
        .lines()
        .find_map(|l| l.strip_prefix("note: ")?.split(' ').next()?.parse().ok())
        .expect("count in the note");
    assert!(announced >= 1 && named == announced, "named {named} of {announced}:
{stdout}");

    // 8. The same report fails closed once its evidence is corrupted.
    sandbox.edit("evidence/test-manifest.json", r#""verdict": "pass""#, r#""verdict": "fail""#);
    let out = std::process::Command::new(bin)
        .args(["--check", report_file.to_str().unwrap(), "--allow-unverified"])
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("ASSURANCE VERIFICATION FAILED"));

    // 9. Missing manifests make --check fail rather than skip the cross-check.
    let _ = std::fs::remove_file(tmp.join("evidence/bench-manifest.json"));
    let out = std::process::Command::new(bin)
        .args(["--check", report_file.to_str().unwrap()])
        .output()
        .expect("binary must run");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn test_full_pass_verdict_when_all_rungs_pass() {
    let registry = WeaknessRegistry::embedded();
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // Discharge every exemption (CWE-208 with proof down to gate level; the
    // unsafe-exception classes with evidence of some tool's making).
    for w in &mut report.weaknesses {
        if w.status == WeaknessStatus::Exemption {
            w.status = WeaknessStatus::Discharged;
            w.approved = None;
            w.reason = "proved down to gate-level netlist in Busicom pilot".into();
            w.evidence = vec!["gate-level-proof".into()];
        }
    }
    report.claims[0].proof.as_mut().unwrap().label = "proved-to-gate-level".into();
    discharge_unchecked(&mut report);
    report.exemptions.clear();
    report.rungs.e2 = "pass".into();
    report.verdict = "pass".into();

    let verdict = verify_assurance_report(&report, &registry).expect("should pass verification");
    assert_eq!(verdict, CheckVerdict::Pass);
}

#[test]
fn test_registry_validation_errors() {
    // 1. Not an object
    assert!(WeaknessRegistry::load_from_str("[]").is_err());

    // 2. Unsupported version
    assert!(WeaknessRegistry::load_from_str(r#"{"version": 2, "classes": []}"#).is_err());

    // 3. Duplicate CWE
    let duplicate_json = r#"{
      "version": 1,
      "classes": [
        {"cwe": 208, "name": "A", "family": "cryptography"},
        {"cwe": 208, "name": "B", "family": "cryptography"}
      ]
    }"#;
    assert!(WeaknessRegistry::load_from_str(duplicate_json).is_err());

    // 4. Missing required class fields
    let missing_field_json = r#"{
      "version": 1,
      "classes": [
        {"name": "No CWE"}
      ]
    }"#;
    assert!(WeaknessRegistry::load_from_str(missing_field_json).is_err());
}

#[test]
fn test_json_edge_cases_and_coverage() {
    use coding_adventures_ca_assurance_report::json::JsonValue;

    // Test numbers: decimal, negative, exponential
    let val = JsonValue::parse(r#"{"exp": 1e2, "float": 0.5, "empty_arr": [], "empty_obj": {}}"#).unwrap();
    assert_eq!(val.get("exp").and_then(|v| v.as_i64()), Some(100));
    assert_eq!(val.get("float").and_then(|v| v.as_i64()), None);
    assert_eq!(val.get("empty_arr").and_then(|v| v.as_array()), Some(&[][..]));
    assert!(val.get("empty_obj").and_then(|v| v.as_object()).unwrap().is_empty());

    // Display trait
    let display_str = format!("{}", val);
    assert!(display_str.contains("exp"));

    // Trailing chars
    assert!(JsonValue::parse(r#"{"a": 1} trailing"#).is_err());

    // Unicode escapes
    let unicode = JsonValue::parse(r#""\u0041\u0042""#).unwrap();
    assert_eq!(unicode.as_str(), Some("AB"));

    // Bounded depth limit (max 64) prevents stack overflow DoS
    let deep_json = "[".repeat(70) + &"]".repeat(70);
    assert!(JsonValue::parse(&deep_json).is_err());
}

#[test]
fn test_display_and_error_formatting() {
    use coding_adventures_ca_assurance_report::checker::CheckError;
    use coding_adventures_ca_assurance_report::matrix::MatrixError;
    use coding_adventures_ca_assurance_report::registry::RegistryError;
    use coding_adventures_ca_assurance_report::report::ReportError;

    let err = CheckError::InvalidPackage("bad".into());
    assert!(format!("{err}").contains("bad"));
    let err = CheckError::IncompleteMatrix("inc".into());
    assert!(format!("{err}").contains("inc"));
    let err = CheckError::InvalidExclusion("inv".into());
    assert!(format!("{err}").contains("inv"));
    let err = CheckError::OpenRow("open".into());
    assert!(format!("{err}").contains("open"));
    let err = CheckError::IllegalPassVerdict("pass".into());
    assert!(format!("{err}").contains("pass"));
    let err = CheckError::UncoveredSymbols(vec!["sym".into()]);
    assert!(format!("{err}").contains("sym"));
    let err = CheckError::InvalidRungStatus("broken".into());
    assert!(format!("{err}").contains("broken"));
    let err = CheckError::FailingClaim("failing".into());
    assert!(format!("{err}").contains("failing"));
    let err = CheckError::MissingEvidence("evidence".into());
    assert!(format!("{err}").contains("evidence"));
    let err = CheckError::MismatchedExemption("mismatch".into());
    assert!(format!("{err}").contains("mismatch"));
    let err = CheckError::CapabilitiesViolation("cap_viol".into());
    assert!(format!("{err}").contains("cap_viol"));
    let err = CheckError::CorruptReport("corrupt".into());
    assert!(format!("{err}").contains("corrupt"));

    let err = RegistryError::Io("io".into());
    assert!(format!("{err}").contains("io"));
    let err = RegistryError::JsonParse("json".into());
    assert!(format!("{err}").contains("json"));
    let err = RegistryError::Validation("val".into());
    assert!(format!("{err}").contains("val"));

    let err = MatrixError::MissingClass(999);
    assert!(format!("{err}").contains("999"));
    let err = MatrixError::InvalidExclusion("inv".into());
    assert!(format!("{err}").contains("inv"));
    let err = MatrixError::OpenRow("open".into());
    assert!(format!("{err}").contains("open"));

    let err = ReportError::JsonParse("jp".into());
    assert!(format!("{err}").contains("jp"));
    let err = ReportError::Validation("val".into());
    assert!(format!("{err}").contains("val"));
}

#[test]
fn test_checker_edge_cases_and_rejections() {
    let registry = WeaknessRegistry::embedded();
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 26);

    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();

    // 1. Empty reason for excluded row
    for w in &mut report.weaknesses {
        if w.cwe == 78 {
            w.reason = "".into();
        }
    }
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::InvalidExclusion(_))
    ));

    // 2. Unknown CWE in report
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.weaknesses[0].cwe = 99999;
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::IncompleteMatrix(_))
    ));

    // 3. Unknown verdict
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.rungs.e2 = "pass".into();
    for e in &mut report.exemptions {
        e.approved = true;
    }
    for w in report.weaknesses.iter_mut().filter(|w| w.status == WeaknessStatus::Exemption) {
        w.approved = Some(true);
    }
    report.verdict = "unrecognized-verdict".into();
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::CorruptReport(_))
    ));

    // 4. Enforced report-only mode when all rungs pass
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.rungs.e2 = "pass".into();
    report.exemptions.clear();
    for w in report.weaknesses.iter_mut().filter(|w| w.status == WeaknessStatus::Exemption) {
        w.status = WeaknessStatus::Discharged;
    }
    report.claims[0].proof.as_mut().unwrap().label = "proved-to-gate-level".into();
    report.verdict = "report-only".into();
    let verdict = verify_assurance_report(&report, &registry).unwrap();
    assert_eq!(verdict, CheckVerdict::ReportOnly("enforced report-only mode".into()));

    // 5. Invalid rung status
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.rungs.e1 = "fail".into();
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::InvalidRungStatus(_))
    ));

    // 6. Failing claim property or fuzz
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.claims[0].property.status = "fail".into();
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::FailingClaim(_))
    ));

    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.claims[0].fuzz.as_mut().unwrap().status = "crash".into();
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::FailingClaim(_))
    ));

    // 7. Missing evidence on discharged row
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    let row_400 = report.weaknesses.iter_mut().find(|w| w.cwe == 400).unwrap();
    row_400.evidence.clear();
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::MissingEvidence(_))
    ));

    // 8. Discharged CWE-208 without gate-level proof label
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    let row_208 = report.weaknesses.iter_mut().find(|w| w.cwe == 208).unwrap();
    row_208.status = WeaknessStatus::Discharged;
    row_208.evidence = vec!["wir-proof-only".into()];
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::MissingEvidence(_))
    ));

    // 9. Mismatched exemption between matrix and exemptions list
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.exemptions[0].approved = true; // matrix says false
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::MismatchedExemption(_))
    ));

    // 10. Capabilities analyzer failure when verdict is pass
    let mut report = generate_ct_compare_report("a1a722f32c5", &registry, &real_evidence()).unwrap();
    report.rungs.e2 = "pass".into();
    report.exemptions.clear();
    for w in report.weaknesses.iter_mut().filter(|w| w.status == WeaknessStatus::Exemption) {
        w.status = WeaknessStatus::Discharged;
    }
    report.claims[0].proof.as_mut().unwrap().label = "proved-to-gate-level".into();
    discharge_unchecked(&mut report);
    report.verdict = "pass".into();
    report.capabilities.analyzer = "fail".into();
    assert!(matches!(
        verify_assurance_report(&report, &registry),
        Err(CheckError::CapabilitiesViolation(_))
    ));
}

#[test]
fn test_matrix_builder_violations() {
    let registry = WeaknessRegistry::embedded();

    // A declared unsafe exception no longer yields an "excluded" memory-safety
    // row: OOB read/write are discharged from bounds-checked proofs, the rest
    // become unapproved exemptions (AF-22).
    let mut unsafe_ev = evid();
    unsafe_ev.unsafe_exceptions = vec!["volatile barrier".into()];
    let ctx = PackageAuditContext {
        package_id: "rust/ct-compare".into(),
        is_safe_rust: false,
        declared_capabilities: vec![],
        has_wir_timing_proof: true,
        has_instruction_benchmarks: true,
        has_zero_alloc_proof: true,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: unsafe_ev.clone(),
    };
    let rows = build_weakness_matrix(&registry, &ctx).unwrap();
    for cwe in [416, 415, 476, 190] {
        let row = rows.iter().find(|r| r.cwe == cwe).unwrap();
        assert_eq!(row.status, WeaknessStatus::Exemption, "CWE-{cwe}");
        assert_eq!(row.approved, Some(false));
        assert!(!row.reason.contains("no unsafe blocks"));
    }
    for cwe in [125, 787] {
        let row = rows.iter().find(|r| r.cwe == cwe).unwrap();
        assert_eq!(row.status, WeaknessStatus::Discharged, "CWE-{cwe}");
        assert!(row.evidence.contains(&"warrant/m/proof/op/constant-time".to_string()));
    }
    // Without bounds-checked proofs the OOB classes fall back to exemptions too.
    let mut no_oob = unsafe_ev;
    no_oob.oob_ids.clear();
    let ctx = PackageAuditContext { evidence: no_oob, ..ctx };
    let rows = build_weakness_matrix(&registry, &ctx).unwrap();
    assert_eq!(rows.iter().find(|r| r.cwe == 125).unwrap().status, WeaknessStatus::Exemption);

    // Undeclared / unhandled capability triggers open row error
    let ctx = PackageAuditContext {
        package_id: "rust/ct-compare".into(),
        is_safe_rust: true,
        declared_capabilities: vec!["proc:exec:*".into()],
        has_wir_timing_proof: true,
        has_instruction_benchmarks: true,
        has_zero_alloc_proof: true,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: evid(),
    };
    assert!(build_weakness_matrix(&registry, &ctx).is_err());

    // No step-bound evidence ids: CWE-400 cannot be discharged on a bare flag.
    let ctx = PackageAuditContext {
        package_id: "rust/ct-compare".into(),
        is_safe_rust: true,
        declared_capabilities: vec![],
        has_wir_timing_proof: true,
        has_instruction_benchmarks: true,
        has_zero_alloc_proof: true,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: AuditEvidence::default(),
    };
    assert!(build_weakness_matrix(&registry, &ctx).is_err());

    // Missing zero alloc proof
    let ctx = PackageAuditContext {
        package_id: "rust/ct-compare".into(),
        is_safe_rust: true,
        declared_capabilities: vec![],
        has_wir_timing_proof: true,
        has_instruction_benchmarks: true,
        has_zero_alloc_proof: false,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: evid(),
    };
    assert!(build_weakness_matrix(&registry, &ctx).is_err());

    // Missing timing proof
    let ctx = PackageAuditContext {
        package_id: "rust/ct-compare".into(),
        is_safe_rust: true,
        declared_capabilities: vec![],
        has_wir_timing_proof: false,
        has_instruction_benchmarks: true,
        has_zero_alloc_proof: true,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: evid(),
    };
    assert!(build_weakness_matrix(&registry, &ctx).is_err());

    // WeaknessStatus parsing
    assert_eq!("excluded".parse::<WeaknessStatus>(), Ok(WeaknessStatus::Excluded));
    assert_eq!("discharged".parse::<WeaknessStatus>(), Ok(WeaknessStatus::Discharged));
    assert_eq!("exemption".parse::<WeaknessStatus>(), Ok(WeaknessStatus::Exemption));
    assert!("other".parse::<WeaknessStatus>().is_err());
}

#[test]
fn test_report_deserialization_failures() {
    assert!(AssuranceReport::from_json_str("[]").is_err());
    assert!(AssuranceReport::from_json_str("{}").is_err());
    assert!(AssuranceReport::from_json_str(r#"{"package": "rust/foo"}"#).is_err());
    assert!(AssuranceReport::from_json_str(r#"{"package": "rust/foo", "capabilities": {}}"#).is_err());
}
