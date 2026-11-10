//! Security regressions for the package-agnostic report (VER04h section 9).
//!
//! M1: a manifest in which some operations are proven and others are
//! `not-live` / `unchecked` must not let the proven ones carry CWE-400 or
//! CWE-208 to a discharge or a proof citation, in the generator *and* in
//! `--check`.  L1: a `not-live` entry is an opt-in declared in
//! `evidence.bindings`.  L5: the package version comes from `[package]`.

mod common;

use common::Sandbox;

use coding_adventures_ca_assurance_report::evidence::package_version;
use coding_adventures_ca_assurance_report::json::JsonValue;
use coding_adventures_ca_assurance_report::{
    generate_ct_compare_report, verify_evidence, AssuranceReport, Evidence, EvidenceCheckError as E,
    MatrixError, WeaknessRegistry, WeaknessStatus,
};

const EQ_CT: &str = "warrant/ct_compare/proof/eq/constant-time";
const EQ_STEPS: &str = "warrant/ct_compare/proof/eq/steps-bound";

/// Rewrite one proof entry in the sandbox's proof manifest.
fn set_entry(sb: &Sandbox, id: &str, verdict: &str, label: &str, note: &str, drop_certs: bool) {
    let mut doc = JsonValue::parse(&sb.read("evidence/proof-manifest.json")).unwrap();
    let JsonValue::Object(top) = &mut doc else { panic!("object") };
    let JsonValue::Array(proofs) = top.get_mut("proofs").unwrap() else { panic!("array") };
    let mut hit = false;
    for p in proofs {
        let JsonValue::Object(o) = p else { panic!("entry") };
        if o.get("id").and_then(|v| v.as_str()) == Some(id) {
            o.insert("verdict".into(), JsonValue::String(verdict.into()));
            o.insert("label".into(), JsonValue::String(label.into()));
            o.insert("note".into(), JsonValue::String(note.into()));
            if drop_certs {
                o.insert("certificates".into(), JsonValue::Array(vec![]));
            }
            hit = true;
        }
    }
    assert!(hit, "{id} is in the manifest");
    sb.write("evidence/proof-manifest.json", &(doc.to_pretty_string(2) + "\n"));
}

fn not_live(sb: &Sandbox, id: &str) {
    set_entry(sb, id, "not-live", "not-live", "blocked for the test", true);
}

fn declare_eq_not_live(sb: &Sandbox) {
    sb.edit("evidence.bindings", "op eq export=ct_eq ", "op eq proofs_not_live=test-blocker export=ct_eq ");
}

/// `eq` loses both proofs; every other operation keeps its passing proofs and
/// the bench manifest still passes.
fn mixed(tag: &str) -> Sandbox {
    let sb = Sandbox::new(tag);
    not_live(&sb, EQ_CT);
    not_live(&sb, EQ_STEPS);
    declare_eq_not_live(&sb);
    sb
}

fn derive(sb: &Sandbox) -> (Result<AssuranceReport, MatrixError>, Evidence) {
    let ev = Evidence::load(&sb.pkg).expect("evidence loads");
    (generate_ct_compare_report("c0ffee", &WeaknessRegistry::embedded(), &ev), ev)
}

fn row(r: &AssuranceReport, cwe: u32) -> &coding_adventures_ca_assurance_report::WeaknessRow {
    r.weaknesses.iter().find(|w| w.cwe == cwe).unwrap()
}

#[test]
fn m1_mixed_not_live_manifest_discharges_neither_cwe_400_nor_cwe_208() {
    let sb = mixed("m1-mixed");
    let (report, ev) = derive(&sb);
    let report = report.expect("a mixed manifest still derives a report");
    for cwe in [400, 208] {
        let w = row(&report, cwe);
        assert_eq!(w.status, WeaknessStatus::Exemption, "CWE-{cwe} must not be discharged");
        assert_eq!(w.approved, Some(false), "CWE-{cwe} is an unapproved exemption");
        assert!(
            w.evidence.iter().all(|e| !e.contains("/proof/")),
            "CWE-{cwe} must not cite the passing proofs of the other operations: {:?}",
            w.evidence
        );
        assert!(w.reason.contains("blocked for the test"), "CWE-{cwe} quotes the blocker: {}", w.reason);
    }
    // The honest report verifies.
    verify_evidence(&report, &ev).expect("the generator's own mixed report verifies");
    // weakest() considers every obligation, not only the live ones.
    let ctx = ev.audit_context("rust/ct-compare");
    assert_eq!(ctx.evidence.ct_proof_label, "not-live");
    assert_eq!(ctx.evidence.step_bound_label, "not-live");
    assert!(!ctx.has_wir_timing_proof, "one unproven operation withdraws the timing-proof flag");
    assert!(!ctx.evidence.step_unproven.is_empty() && !ctx.evidence.ct_unproven.is_empty());
}

#[test]
fn m1_check_rejects_a_mixed_report_that_claims_a_cwe_400_discharge() {
    let sb = mixed("m1-check400");
    let (report, ev) = derive(&sb);
    let mut report = report.unwrap();
    // Forge the discharge a generator without the fix would have written:
    // it cites the *other* operations' passing step-bound proofs and the bench.
    let cited: Vec<String> = ev
        .proofs
        .iter()
        .filter(|p| p.kind == "steps-bound" && p.verdict == "pass")
        .map(|p| p.id.clone())
        .chain(ev.bench.iter().map(|(id, _)| id.clone()))
        .collect();
    assert!(!cited.is_empty());
    let w = report.weaknesses.iter_mut().find(|w| w.cwe == 400).unwrap();
    w.status = WeaknessStatus::Discharged;
    w.approved = None;
    w.reason = "discharged for bounded inputs: forged".into();
    w.evidence = cited;
    report.exemptions.retain(|x| x.cwe != 400);
    match verify_evidence(&report, &ev) {
        Err(E::EvidenceMismatch(m)) => assert!(m.contains("CWE-400") && m.contains("not proven"), "{m}"),
        other => panic!("a forged CWE-400 discharge must fail --check, got {other:?}"),
    }
}

#[test]
fn m1_check_rejects_a_mixed_report_that_cites_wir_proofs_for_cwe_208() {
    let sb = mixed("m1-check208");
    let (report, ev) = derive(&sb);
    let report = report.unwrap();
    let cited: Vec<String> = ev
        .proofs
        .iter()
        .filter(|p| p.kind == "constant-time" && p.verdict == "pass")
        .map(|p| p.id.clone())
        .collect();
    assert!(!cited.is_empty());
    // As an unapproved exemption (the shape main's code produced)...
    let mut forged = report.clone();
    forged.weaknesses.iter_mut().find(|w| w.cwe == 208).unwrap().evidence = cited.clone();
    match verify_evidence(&forged, &ev) {
        Err(E::EvidenceMismatch(m)) => assert!(m.contains("CWE-208"), "{m}"),
        other => panic!("CWE-208 citing WIR proofs next to an unproven operation must fail: {other:?}"),
    }
    // ...and as a discharge.
    let mut forged = report;
    let w = forged.weaknesses.iter_mut().find(|w| w.cwe == 208).unwrap();
    w.status = WeaknessStatus::Discharged;
    w.evidence = cited;
    assert!(matches!(verify_evidence(&forged, &ev), Err(E::EvidenceMismatch(_))));
}

#[test]
fn m1_unchecked_steps_or_ct_entries_block_discharge_exactly_as_before() {
    // `unchecked` is not a proof either: the other operations' passes must
    // not discharge the row.  With no not-live entry to acknowledge, the row
    // is open (the generator refuses), as on main.
    for (id, what) in [(EQ_STEPS, "CWE-400"), (EQ_CT, "CWE-208")] {
        let sb = Sandbox::new("m1-unchecked");
        set_entry(&sb, id, "unchecked", "unchecked", "the pilot prover does not track this resource", true);
        let (report, _) = derive(&sb);
        match report {
            Err(MatrixError::OpenRow(m)) => assert!(m.contains(what), "{m}"),
            Ok(r) => panic!("{what}: unchecked entry produced a report: {:?}", row(&r, 400).status),
            Err(e) => panic!("{e}"),
        }
    }
}

#[test]
fn l1_not_live_needs_a_declaration_in_evidence_bindings() {
    // Manifest-only `not-live`: no declaration, no load.
    let sb = Sandbox::new("l1-undeclared");
    not_live(&sb, EQ_CT);
    let e = Evidence::load(&sb.pkg).unwrap_err().0;
    assert!(e.contains("proofs_not_live"), "{e}");

    // Declared for a different operation: still refused.
    let sb = Sandbox::new("l1-other-op");
    not_live(&sb, EQ_CT);
    sb.edit("evidence.bindings", "op eq_u64 export=ct_eq_u64 ", "op eq_u64 proofs_not_live=x export=ct_eq_u64 ");
    assert!(Evidence::load(&sb.pkg).is_err());

    // No bindings file at all: refused.
    let sb = Sandbox::new("l1-no-file");
    not_live(&sb, EQ_CT);
    std::fs::remove_file(sb.pkg.join("evidence.bindings")).unwrap();
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("proofs_not_live"));

    // Declared: loads.
    let sb = Sandbox::new("l1-declared");
    not_live(&sb, EQ_CT);
    declare_eq_not_live(&sb);
    Evidence::load(&sb.pkg).expect("a declared not-live entry loads");
}

#[test]
fn l1_a_not_live_entry_must_be_well_formed() {
    let load = |tweak: &dyn Fn(&Sandbox)| {
        let sb = Sandbox::new("l1-shape");
        declare_eq_not_live(&sb);
        tweak(&sb);
        Evidence::load(&sb.pkg).map(|_| ()).map_err(|e| e.0)
    };
    // Keeps its certificate.
    let e = load(&|sb| set_entry(sb, EQ_CT, "not-live", "not-live", "blocked", false)).unwrap_err();
    assert!(e.contains("certificate"), "{e}");
    // Carries a proof label.
    let e = load(&|sb| set_entry(sb, EQ_CT, "not-live", "proved", "blocked", true)).unwrap_err();
    assert!(e.contains("label"), "{e}");
    // No blocker recorded.
    let e = load(&|sb| set_entry(sb, EQ_CT, "not-live", "not-live", "  ", true)).unwrap_err();
    assert!(e.contains("note"), "{e}");
    // The well-formed entry is accepted.
    load(&|sb| set_entry(sb, EQ_CT, "not-live", "not-live", "blocked", true)).unwrap();
}

#[test]
fn l5_package_version_comes_from_the_package_table_only() {
    let toml = "[workspace.package]\nversion = \"9.9.9\"\n\n[dependencies]\nfoo = \"1\"\n\n[dependencies.bar]\nversion = \"7.7.7\"\n\n[package]\nname = \"x\"\n# version = \"8.8.8\"\nversion = \"1.2.3\" # trailing\n\n[dev-dependencies.baz]\nversion = \"6.6.6\"\n";
    assert_eq!(package_version(toml).unwrap(), "1.2.3");
    // A `version` that only appears outside [package] is not the package's.
    assert!(package_version("[dependencies.bar]\nversion = \"7.7.7\"\n[package]\nname = \"x\"\n").is_err());
    assert!(package_version("[workspace.package]\nversion = \"9.9.9\"\n").is_err());
    // Inherited versions are refused rather than guessed.
    assert!(package_version("[package]\nname = \"x\"\nversion.workspace = true\n").is_err());
    assert!(package_version("[package]\nversion = 3\n").is_err());
    assert!(package_version("[package]\nversion = \"\"\n").is_err());
    // The checked-in manifests report their real versions.
    let ct = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ct-compare/Cargo.toml")).unwrap();
    assert_eq!(package_version(&ct).unwrap(), "0.1.0");
}
