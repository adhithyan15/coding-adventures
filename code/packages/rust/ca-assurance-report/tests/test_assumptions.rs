//! VER13 section 6: every assumption rule AC-1..AC-14 has a falsification
//! test here.  Each test starts from a report the generator derived from the
//! real evidence (which passes every rule, see the first test), breaks exactly
//! one thing, and asserts the rule's stable diagnostic name.  A rule that can
//! only fire on a catalogue the loader would already refuse (AC-11's stale
//! attestation, a catalogue-side discharge) is exercised by editing the loaded
//! catalogue in memory: the report-side rule is defence in depth and must
//! still fire on its own.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use coding_adventures_ca_assurance_report::assumptions::{
    derive_exemption_assumptions, label_matches, remaining_by_layer, HOST_STACK, OWNED_RV32I_STACK,
};
use coding_adventures_ca_assurance_report::catalogue::{
    parse_pattern, CatalogueSet, DischargedBy, NonClaim, CATALOGUE_PATH, EVIDENCE_KINDS_PATH, NON_CLAIMS_PATH,
};
use coding_adventures_ca_assurance_report::evidence::ProofEntry;
use coding_adventures_ca_assurance_report::{
    check_assumptions, generate_ct_compare_report, generate_package_report, verify_evidence,
    AssumptionCheckError, AssumptionSummary, AssuranceReport, AuditProfile, DischargedAssumption, Evidence,
    EvidenceCheckError, WeaknessRegistry, WeaknessStatus,
};

fn rust_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

/// The real ct-compare evidence and the report derived from it.
fn ct() -> (AssuranceReport, Evidence) {
    let ev = Evidence::load(&rust_dir("ct-compare")).expect("ct-compare evidence loads");
    let r = generate_ct_compare_report("c0ffee", &WeaknessRegistry::embedded(), &ev).expect("derives");
    (r, ev)
}

/// The real gf256 evidence and the report derived from it.
fn gf() -> (AssuranceReport, Evidence) {
    let ev = Evidence::load(&rust_dir("gf256")).expect("gf256 evidence loads");
    let r = generate_package_report(
        "rust/gf256",
        "0.2.1",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        &ev,
        AuditProfile::FieldArithmetic,
    )
    .expect("derives");
    (r, ev)
}

fn check(r: &AssuranceReport, ev: &Evidence) -> Result<AssumptionSummary, AssumptionCheckError> {
    check_assumptions(r, ev, false)
}

/// Assert that `res` failed with diagnostic `name` and a message containing `needle`.
#[track_caller]
fn expect_err(res: Result<AssumptionSummary, AssumptionCheckError>, name: &str, needle: &str) {
    match res {
        Err(e) => {
            assert_eq!(e.code().1, name, "wrong rule: {e}");
            assert!(e.to_string().contains(needle), "{e} should mention {needle:?}");
            assert!(e.to_string().starts_with(e.code().0), "the diagnostic leads with the rule id: {e}");
        }
        Ok(s) => panic!("expected {name}, the report passed ({s:?})"),
    }
}

/// Move `id` from claim `i`'s assumptions to its `discharged` list.
fn discharge(r: &mut AssuranceReport, ev: &Evidence, i: usize, id: &str, evidence: &[&str], rung: &str) {
    let p = r.claims[i].proof.as_mut().unwrap();
    p.assumptions.retain(|a| a != id);
    p.discharged.push(DischargedAssumption {
        id: id.into(),
        rung: rung.into(),
        stack: p.stack.clone(),
        evidence: evidence.iter().map(|s| s.to_string()).collect(),
        pinned: BTreeMap::new(),
        scope: String::new(),
    });
    p.remaining_by_layer = remaining_by_layer(&ev.catalogue.catalogue, &p.assumptions);
}

/// Re-derive every exemption's list from the (edited) claims.
fn refresh_exemptions(r: &mut AssuranceReport, ev: &Evidence) {
    let claims = r.claims.clone();
    for x in &mut r.exemptions {
        x.assumptions = derive_exemption_assumptions(&ev.catalogue, &x.id, &claims).unwrap();
    }
}

/// A passing proof entry the in-memory evidence can cite.
fn add_proof(ev: &mut Evidence, id: &str) {
    ev.proofs.push(ProofEntry {
        id: id.into(),
        op: "eq".into(),
        function: "ct_eq".into(),
        kind: "test-only".into(),
        label: "proved".into(),
        verdict: "pass".into(),
        ..ProofEntry::default()
    });
}

/// Point evidence kind `kind` at the proof manifest with one pattern.
fn reroute_kind(ev: &mut Evidence, kind: &str, pattern: &str) {
    let k = ev.catalogue.kinds.kinds.iter_mut().find(|k| k.kind == kind).unwrap();
    k.manifest = "proof".into();
    k.patterns = vec![parse_pattern(pattern).unwrap()];
}

fn entry_mut<'a>(ev: &'a mut Evidence, id: &str) -> &'a mut coding_adventures_ca_assurance_report::catalogue::Assumption {
    ev.catalogue.catalogue.entries.iter_mut().find(|e| e.id == id).unwrap()
}

// ---- the untouched reports --------------------------------------------------

#[test]
fn the_real_reports_pass_every_rule_and_the_r0_list_grows() {
    let (r, ev) = ct();
    let summary = check(&r, &ev).expect("the derived ct-compare report passes AC-1..AC-14");
    assert!(summary.unused.is_empty(), "every ct-compare entry is used: {:?}", summary.unused);
    verify_evidence(&r, &ev).expect("and the full evidence cross-check");
    for c in &r.claims {
        let p = c.proof.as_ref().unwrap();
        // VER13 section 4.1: the list is longer than the four legacy strings,
        // and the four survive under their catalogue ids.
        assert!(p.assumptions.len() > 4, "{}: {}", c.id, p.assumptions.len());
        for legacy in ["compiler.preserves_timing", "memory.unaliased", "wir.bound.heap-unchecked", "wir.bound.depth-unchecked"] {
            assert!(p.assumptions.iter().any(|a| a == legacy), "{}: {legacy}", c.id);
        }
        for silent in ["wir.lowering.faithful", "wir.lowering.volatile-by-name", "wir.proof.obligation-binding", "spec.obs-model.complete", "mc.isa.timing-data-independent", "phys.silicon.realization"] {
            assert!(p.assumptions.iter().any(|a| a == silent), "{}: section 4.1 names {silent}", c.id);
        }
        assert_eq!(p.stack, HOST_STACK);
        assert_eq!(p.rung, "R0");
        assert!(p.discharged.is_empty(), "nothing is discharged yet (AS-001a)");
    }
    assert_eq!(r.catalogue.path, CATALOGUE_PATH);
    // gf256: no proof claims, so only the report scope; its one `*` entry with
    // proof labels is unused (AC-10 warning, not an error).
    let (g, gev) = gf();
    let s = check(&g, &gev).expect("the derived gf256 report passes");
    assert_eq!(s.unused, vec!["tcb.report.checker-correct".to_string()]);
    assert!(g.exemptions.iter().all(|x| x.assumptions.is_empty()), "not-live exemptions rest on no assumption");
}

#[test]
fn the_checked_in_reports_pin_the_catalogue_on_disk() {
    for name in ["ct-compare", "gf256"] {
        let dir = rust_dir(name);
        let ev = Evidence::load(&dir).unwrap();
        let r = AssuranceReport::from_json_str(&std::fs::read_to_string(dir.join("assurance_report.json")).unwrap()).unwrap();
        check(&r, &ev).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn every_assumption_string_the_sources_emit_resolves_in_the_catalogue() {
    // AS-001a acceptance 1: literals in evidence.rs / matrix.rs / lib.rs that
    // name assumptions or attestations resolve to a catalogue id or alias.
    let set = CatalogueSet::load_from_root(&repo_root()).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut seen = 0;
    for f in ["evidence.rs", "matrix.rs", "lib.rs"] {
        let text = std::fs::read_to_string(src.join(f)).unwrap();
        for lit in text.split('"').skip(1).step_by(2) {
            let names_assumption = (lit.starts_with("unverified::") && lit.len() > "unverified::".len())
                || lit == "compiler.preserves_timing"
                || lit == "memory.unaliased";
            if names_assumption {
                seen += 1;
                assert!(set.catalogue.resolve(lit).is_some(), "{f}: {lit:?} is not in the catalogue");
            }
        }
    }
    assert!(seen >= 11, "found only {seen} literals");
    for legacy in ["heap-bound:unchecked", "depth-bound:unchecked"] {
        assert!(set.catalogue.by_alias(legacy).is_some(), "{legacy}");
    }
}

// ---- AC-14 CatalogueDrift ---------------------------------------------------

#[test]
fn ac14_a_stale_pin_is_catalogue_drift() {
    let (r, ev) = ct();
    for field in 0..3 {
        let mut bad = r.clone();
        match field {
            0 => bad.catalogue.sha256 = "0".repeat(64),
            1 => bad.catalogue.evidence_kinds_sha256 = "0".repeat(64),
            _ => bad.catalogue.path = "code/assumptions/catalogue-v2.json".into(),
        }
        expect_err(check(&bad, &ev), "CatalogueDrift", "regenerate");
    }
}

#[test]
fn ac14_editing_the_catalogue_on_disk_invalidates_a_pinned_report() {
    let sb = common::Sandbox::new("ac14-edit");
    let ev = Evidence::load(&sb.pkg).unwrap();
    let r = generate_ct_compare_report("c0ffee", &WeaknessRegistry::embedded(), &ev).unwrap();
    check(&r, &ev).unwrap();
    // A semantically empty edit still changes the digest: the pin is on bytes.
    let path = sb.assumptions("catalogue-v1.json");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push('\n');
    std::fs::write(&path, text).unwrap();
    let ev2 = Evidence::load(&sb.pkg).unwrap();
    expect_err(check(&r, &ev2), "CatalogueDrift", "sha256");
    // ...and the full gate reports it as an assumption-rule failure.
    match verify_evidence(&r, &ev2) {
        Err(EvidenceCheckError::Assumption(AssumptionCheckError::CatalogueDrift(_))) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn ac14_missing_oversized_or_malformed_catalogue_files_fail_closed() {
    // Missing: no catalogue above the package means no evidence at all.
    let sb = common::Sandbox::new("ac14-missing");
    std::fs::remove_file(sb.assumptions("evidence-kinds.json")).unwrap();
    let e = Evidence::load(&sb.pkg).unwrap_err();
    assert!(e.0.contains("evidence-kinds.json"), "{e}");
    let sb = common::Sandbox::new("ac14-gone");
    std::fs::remove_dir_all(sb.root.join("code/assumptions")).unwrap();
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("not found"));
    // Oversized: refused while reading, before parsing.
    let sb = common::Sandbox::new("ac14-big");
    std::fs::write(sb.assumptions("non-claims.json"), vec![b' '; 64 * 1024 + 1]).unwrap();
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("limit"));

    // Malformed bytes, one breakage at a time.
    let root = repo_root();
    let cat = std::fs::read_to_string(root.join(CATALOGUE_PATH)).unwrap();
    let kinds = std::fs::read_to_string(root.join(EVIDENCE_KINDS_PATH)).unwrap();
    let nc = std::fs::read_to_string(root.join(NON_CLAIMS_PATH)).unwrap();
    let load = |c: &str, k: &str, n: &str| CatalogueSet::from_bytes(c.as_bytes(), k.as_bytes(), n.as_bytes());
    load(&cat, &kinds, &nc).expect("the committed files load");
    let once = |s: &str, from: &str, to: &str| {
        assert!(s.contains(from), "{from}");
        s.replacen(from, to, 1)
    };
    let cases: Vec<(&str, String, String, String)> = vec![
        ("schema", once(&cat, "ver13-assumption-catalogue-v1", "v0"), kinds.clone(), nc.clone()),
        ("duplicate key", once(&cat, "\"legacy\": false,", "\"legacy\": false, \"legacy\": true,"), kinds.clone(), nc.clone()),
        ("unknown field", once(&cat, "\"legacy\": false,", "\"legacy\": false, \"extra\": 1,"), kinds.clone(), nc.clone()),
        ("layer", once(&cat, "\"layer\": \"spec\"", "\"layer\": \"kernel\""), kinds.clone(), nc.clone()),
        ("unknown id", once(&cat, "\"depends_on\": [\n        \"wir.lowering.faithful\"", "\"depends_on\": [\n        \"no.such.id\""), kinds.clone(), nc.clone()),
        ("cycle", once(&cat, "\"depends_on\": [],\n      \"discharge\": {\n        \"evidence_kinds\": [\n          \"differential-run\"", "\"depends_on\": [\"wir.lowering.faithful\"],\n      \"discharge\": {\n        \"evidence_kinds\": [\n          \"differential-run\""), kinds.clone(), nc.clone()),
        ("kind not defined", cat.clone(), once(&kinds, "\"kind\": \"miri-run\"", "\"kind\": \"miri-run-renamed\""), nc.clone()),
        ("overlap", cat.clone(), once(&kinds, "\"miri/*/*\"", "\"warrant/*/proof/*/steps-bound\""), nc.clone()),
        ("namespace", cat.clone(), once(&kinds, "\"miri/*/*\"", "\"warrant/*/miri/*\""), nc.clone()),
        ("non-claim package", cat.clone(), kinds.clone(), once(&nc, "rust/gf256", "*")),
    ];
    for (what, c, k, n) in cases {
        let e = load(&c, &k, &n).expect_err(what);
        assert!(e.to_string().starts_with("AC-14 MalformedCatalogue"), "{what}: {e}");
    }
}

#[test]
fn ac14_a_package_cannot_shadow_the_repository_catalogue() {
    // A package that ships its own `code/assumptions/` (here: three files that
    // are not even valid catalogues) is never checked against it.
    let sb = common::Sandbox::new("ac14-shadow");
    let shadow = sb.pkg.join("code/assumptions");
    std::fs::create_dir_all(&shadow).unwrap();
    for f in ["catalogue-v1.json", "evidence-kinds.json", "non-claims.json"] {
        std::fs::write(shadow.join(f), "{\"schema\": \"shadow\"}").unwrap();
    }
    // `locate` refuses a catalogue at or inside the package outright.
    let e = CatalogueSet::locate(&sb.pkg).unwrap_err();
    assert!(e.to_string().starts_with("AC-14 MalformedCatalogue") && e.0.contains("ships its own"), "{e}");
    // Evidence::load reads the catalogue at the Warrant document's root, which
    // is above the package, so the shadow is never read.
    let ev = Evidence::load(&sb.pkg).expect("the repository catalogue is used");
    let real = CatalogueSet::load_from_root(&sb.root).unwrap();
    assert_eq!(ev.catalogue.catalogue_sha256, real.catalogue_sha256);
    // A package that also ships the spec tree would put that root *at* the
    // package: refused, never loaded.
    std::fs::create_dir_all(sb.pkg.join("code/specs")).unwrap();
    std::fs::copy(sb.spec(), sb.pkg.join("code/specs/ct-compare.warrant")).unwrap();
    let e = Evidence::load(&sb.pkg).unwrap_err();
    assert!(e.0.contains("strict ancestor"), "{e}");
    let e = CatalogueSet::load_for_package(&sb.pkg, &sb.pkg).unwrap_err();
    assert!(e.0.contains("strict ancestor"), "{e}");
    let e = CatalogueSet::load_for_package(&sb.pkg.join("code"), &sb.pkg).unwrap_err();
    assert!(e.0.contains("strict ancestor"), "{e}");
}

#[test]
fn ac14_the_evidence_kind_pattern_count_is_capped() {
    use coding_adventures_ca_assurance_report::catalogue::{parse_evidence_kinds, MAX_TOTAL_PATTERNS};
    // 17 kinds x 256 patterns = 4352 > 4096: refused before the overlap check.
    let rows: Vec<String> = (0..17)
        .map(|k| {
            let pats: Vec<String> = (0..256).map(|i| format!("\"ns{k}/p{i}\"")).collect();
            format!(
                "{{\"kind\": \"k{k}\", \"manifest\": \"planned\", \"patterns\": [{}], \"verdict\": \"pass\", \"per_run\": false, \"description\": \"d\"}}",
                pats.join(", ")
            )
        })
        .collect();
    let doc = format!("{{\"schema\": \"ver13-evidence-kinds-v1\", \"kinds\": [{}]}}", rows.join(", "));
    let e = parse_evidence_kinds(doc.as_bytes()).unwrap_err();
    assert!(e.0.contains(&format!("more than {MAX_TOTAL_PATTERNS} patterns")), "{e}");
}

// ---- AC-1 UnknownAssumption -------------------------------------------------

#[test]
fn ac1_unknown_aliased_or_repeated_ids_are_rejected_in_every_list() {
    let (r, ev) = ct();
    let mut bad = r.clone();
    bad.claims[0].proof.as_mut().unwrap().assumptions.push("no.such.assumption".into());
    expect_err(check(&bad, &ev), "UnknownAssumption", "no.such.assumption");

    // An alias is accepted only as an `unverified::` evidence id, never in a list.
    let mut bad = r.clone();
    let p = bad.claims[0].proof.as_mut().unwrap();
    p.assumptions.retain(|a| a != "wir.bound.heap-unchecked");
    p.assumptions.push("heap-bound:unchecked".into());
    expect_err(check(&bad, &ev), "UnknownAssumption", "alias of `wir.bound.heap-unchecked`");

    let mut bad = r.clone();
    let first = bad.claims[1].proof.as_ref().unwrap().assumptions[0].clone();
    bad.claims[1].proof.as_mut().unwrap().assumptions.push(first);
    expect_err(check(&bad, &ev), "UnknownAssumption", "listed twice");

    let mut bad = r.clone();
    bad.assumptions.push("gov.unknown".into());
    expect_err(check(&bad, &ev), "UnknownAssumption", "report assumptions");

    let mut bad = r.clone();
    bad.exemptions[0].assumptions.push("src.nope".into());
    expect_err(check(&bad, &ev), "UnknownAssumption", "exemption");

    let mut bad = r.clone();
    discharge(&mut bad, &ev, 0, "made.up", &["x"], "R1");
    expect_err(check(&bad, &ev), "UnknownAssumption", "discharged");

    // Listed both ways at once.
    let mut bad = r.clone();
    discharge(&mut bad, &ev, 0, "memory.unaliased", &["x"], "R1");
    bad.claims[0].proof.as_mut().unwrap().assumptions.push("memory.unaliased".into());
    expect_err(check(&bad, &ev), "UnknownAssumption", "both");
}

// ---- AC-11 StaleAttestation -------------------------------------------------

#[test]
fn ac11_an_unverified_id_needs_a_still_attested_entry() {
    let (r, mut ev) = ct();
    let mut bad = r.clone();
    bad.weaknesses[0].evidence.push("unverified::ast-audit::not-in-catalogue".into());
    expect_err(check(&bad, &ev), "UnknownAssumption", "no catalogue alias");
    // The loader refuses an attestation whose status moved on, so the
    // report-side rule is exercised on an in-memory catalogue.
    entry_mut(&mut ev, "att.ast-audit.no-protocol-sinks").status = "evidenced".into();
    expect_err(check(&r, &ev), "StaleAttestation", "unverified::ast-audit::no-protocol-sinks");
}

// ---- AC-2 EmptyAssumptions --------------------------------------------------

#[test]
fn ac2_a_proved_label_never_has_an_empty_list() {
    let (r, ev) = ct();
    for label in ["bounded(128)", "proved", "proved-to-gate-level"] {
        let mut bad = r.clone();
        let p = bad.claims[3].proof.as_mut().unwrap();
        p.label = label.into();
        p.assumptions.clear();
        p.remaining_by_layer.clear();
        expect_err(check(&bad, &ev), "EmptyAssumptions", label);
    }
    assert!(label_matches("bounded(k)", "bounded(128)"));
    assert!(!label_matches("bounded(k)", "bounded(x)"));
    assert!(!label_matches("proved", "proved-to-isa"));
}

// ---- AC-7 StackMismatch -----------------------------------------------------

#[test]
fn ac7_discharge_evidence_must_be_for_the_claims_stack() {
    let (r, ev) = ct();
    let mut bad = r.clone();
    bad.claims[0].proof.as_mut().unwrap().stack = "host-x86_64".into();
    expect_err(check(&bad, &ev), "StackMismatch", "unknown stack");

    let mut bad = r.clone();
    discharge(&mut bad, &ev, 0, "memory.unaliased", &["warrant/ct_compare/proof/eq/alias-split"], "R0+");
    bad.claims[0].proof.as_mut().unwrap().discharged[0].stack = OWNED_RV32I_STACK.into();
    expect_err(check(&bad, &ev), "StackMismatch", "owned-rv32i");
}

// ---- AC-8 FloorDischarged ---------------------------------------------------

#[test]
fn ac8_floor_and_declared_entries_cannot_be_discharged() {
    let (r, ev) = ct();
    for id in ["phys.silicon.realization", "spec.obs-model.public-length", "cc.caller-context"] {
        let mut bad = r.clone();
        discharge(&mut bad, &ev, 0, id, &["gate/x/structural/y"], "R4");
        expect_err(check(&bad, &ev), "FloorDischarged", id);
    }
}

// ---- AC-6 PartialUmbrella ---------------------------------------------------

#[test]
fn ac6_an_umbrella_is_discharged_only_with_every_refinement() {
    let (r, ev) = ct();
    let mut bad = r.clone();
    discharge(&mut bad, &ev, 0, "compiler.preserves_timing", &["catalogue/refinements/all"], "R2");
    expect_err(check(&bad, &ev), "PartialUmbrella", "compiler.preserves_timing");
}

// ---- AC-3 UnevidencedDischarge ----------------------------------------------

#[test]
fn ac3_a_discharge_needs_passing_evidence_of_an_accepted_kind() {
    let (r, mut ev) = ct();
    let heap = "wir.bound.heap-unchecked";
    let cases: [(&[&str], &str, &str); 6] = [
        (&[], "R0+", "no evidence"),
        (&["not/a/known/kind"], "R0+", "matches no evidence kind"),
        // Right kind, but the manifest records it `unchecked`.
        (&["warrant/ct_compare/proof/eq/heap-bound"], "R0+", "verdict unchecked"),
        // A passing proof of the wrong kind cannot discharge it.
        (&["warrant/ct_compare/proof/eq/steps-bound"], "R0+", "cannot discharge"),
        // Right kind, absent from every manifest.
        (&["warrant/ct_compare/proof/nope/heap-bound"], "R0+", "absent"),
        // Below the catalogue's required rung (R0 is "no discharge").
        (&["warrant/ct_compare/proof/eq/heap-bound"], "R0", "required rung"),
    ];
    for (evidence, rung, needle) in cases {
        let mut bad = r.clone();
        discharge(&mut bad, &ev, 0, heap, evidence, rung);
        expect_err(check(&bad, &ev), "UnevidencedDischarge", needle);
    }
    // A planned kind has no manifest yet, so it can never be satisfied.
    let mut bad = r.clone();
    discharge(&mut bad, &ev, 0, "memory.unaliased", &["miri/ct_compare/tests"], "R0+");
    expect_err(check(&bad, &ev), "UnevidencedDischarge", "planned");

    // The positive control: once the heap bound *passes* in the manifest, the
    // same discharge is accepted (in every claim, so the report stays complete).
    for p in ev.proofs.iter_mut().filter(|p| p.kind == "heap-bound") {
        p.verdict = "pass".into();
    }
    let mut good = r.clone();
    for (i, op) in ["eq", "eq_fixed", "select_bytes", "eq_u64"].iter().enumerate() {
        let id = format!("warrant/ct_compare/proof/{op}/heap-bound");
        discharge(&mut good, &ev, i, heap, &[id.as_str()], "R0+");
    }
    check(&good, &ev).expect("a discharge backed by a passing manifest entry");

    // A catalogue-side discharge must match the manifest digest.
    let (r, mut ev) = ct();
    let e = entry_mut(&mut ev, "os.platform.linux-only");
    e.status = "discharged".into();
    e.discharged_by = vec![DischargedBy {
        evidence: "warrant/ct_compare/negctl/example/eq/invert".into(),
        digest: format!("sha256:{}", "0".repeat(64)),
        stack: HOST_STACK.into(),
        pinned: BTreeMap::new(),
    }];
    e.discharge.as_mut().unwrap().evidence_kinds = vec!["negative-control".into()];
    expect_err(check(&r, &ev), "UnevidencedDischarge", "digest");
}

// ---- AC-9 PinMismatch -------------------------------------------------------

#[test]
fn ac9_per_run_evidence_needs_the_same_pinned_build() {
    let (r, mut ev) = ct();
    // Pretend the proof manifest held an IR-taint run (kind `ir-taint-analysis`
    // is per-run), so the discharge gets past AC-3 and reaches AC-9.
    reroute_kind(&mut ev, "ir-taint-analysis", "warrant/*/proof/*/ir-taint");
    let mut ids = Vec::new();
    for (op, f) in [("eq", "ct_eq"), ("eq_fixed", "ct_eq_fixed"), ("select_bytes", "ct_select_bytes"), ("eq_u64", "ct_eq_u64")] {
        let id = format!("warrant/ct_compare/proof/{op}/ir-taint");
        add_proof(&mut ev, &id);
        ev.proofs.last_mut().unwrap().op = op.into();
        ev.proofs.last_mut().unwrap().function = f.into();
        ids.push(id);
    }
    let target = "cc.llvm-ir.no-secret-branch";
    let tuple: BTreeMap<String, String> = [
        ("toolchain", "rustc 1.99.0"),
        ("target", "x86_64-unknown-linux-gnu"),
        ("flags", "-C opt-level=3"),
        ("artefact_sha256", "ab"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let build = |ev: &Evidence, pinned: &BTreeMap<String, String>, scope: &str| {
        let mut out = r.clone();
        for (i, id) in ids.iter().enumerate() {
            discharge(&mut out, ev, i, target, &[id.as_str()], "R1");
            let d = out.claims[i].proof.as_mut().unwrap().discharged.last_mut().unwrap();
            d.pinned = pinned.clone();
            d.scope = scope.into();
        }
        refresh_exemptions(&mut out, ev);
        out
    };
    expect_err(check(&build(&ev, &BTreeMap::new(), ""), &ev), "PinMismatch", "without a pinned build tuple");
    expect_err(check(&build(&ev, &tuple, "for build ab"), &ev), "PinMismatch", "records no build tuple");
    let mut other = tuple.clone();
    other.insert("toolchain".into(), "rustc 1.98.0".into());
    for id in &ids {
        ev.pinned.insert(id.clone(), other.clone());
    }
    expect_err(check(&build(&ev, &tuple, "for build ab"), &ev), "PinMismatch", "regenerate the evidence");
    for id in &ids {
        ev.pinned.insert(id.clone(), tuple.clone());
    }
    expect_err(check(&build(&ev, &tuple, "for every build"), &ev), "PinMismatch", "for build ab");
    // Positive control, and AS-003a acceptance 5: the CWE-208 exemption's
    // derived list loses the discharged id.
    let good = build(&ev, &tuple, "for build ab");
    check(&good, &ev).expect("a pinned, matching per-run discharge");
    let x = good.exemptions.iter().find(|x| x.cwe == 208).unwrap();
    assert!(!x.assumptions.iter().any(|a| a == target));
    assert!(r.exemptions.iter().find(|x| x.cwe == 208).unwrap().assumptions.iter().any(|a| a == target));
}

// ---- AC-4 SilentlyDroppedAssumption -----------------------------------------

#[test]
fn ac4_an_applicable_or_reachable_assumption_cannot_be_dropped() {
    let (r, ev) = ct();
    // Applies by `applies_to` (label, kinds, package).
    for (claim, id) in [(0, "spec.secret-labels.complete"), (2, "compiler.preserves_timing"), (3, "memory.unaliased")] {
        let mut bad = r.clone();
        let p = bad.claims[claim].proof.as_mut().unwrap();
        p.assumptions.retain(|a| a != id);
        p.remaining_by_layer = remaining_by_layer(&ev.catalogue.catalogue, &p.assumptions);
        refresh_exemptions(&mut bad, &ev);
        expect_err(check(&bad, &ev), "SilentlyDroppedAssumption", id);
    }
    // Reachable only through `depends_on` (a weakness-row entry the volatile
    // lowering depends on).
    let mut bad = r.clone();
    let p = bad.claims[0].proof.as_mut().unwrap();
    p.assumptions.retain(|a| a != "src.rust.unsafe-region.sound");
    p.remaining_by_layer = remaining_by_layer(&ev.catalogue.catalogue, &p.assumptions);
    expect_err(check(&bad, &ev), "SilentlyDroppedAssumption", "src.rust.unsafe-region.sound");
    // The report scope.
    let mut bad = r.clone();
    bad.assumptions.retain(|a| a != "gov.approval.blocked");
    expect_err(check(&bad, &ev), "SilentlyDroppedAssumption", "report-scope");
    // A cited attestation is relied upon even where the catalogue scopes it
    // to another package (gf256 cites ct-compare-scoped attestations).
    let (g, gev) = gf();
    let mut bad = g.clone();
    bad.assumptions.retain(|a| a != "att.ast-audit.no-deserialization");
    expect_err(check(&bad, &gev), "SilentlyDroppedAssumption", "att.ast-audit.no-deserialization");
    // And the full gate names the rule.
    let mut bad = r.clone();
    bad.assumptions.retain(|a| a != "gov.approval.blocked");
    let e = verify_evidence(&bad, &ev).unwrap_err();
    assert!(e.to_string().starts_with("AC-4 SilentlyDroppedAssumption"), "{e}");
}

// ---- AC-5 LabelOutranksAssumptions ------------------------------------------

#[test]
fn ac5_a_label_cannot_outrank_its_undischarged_assumptions() {
    let (r, ev) = ct();
    for label in ["proved-to-gate-level", "proved-to-isa", "proved-per-run"] {
        let mut bad = r.clone();
        bad.claims[0].proof.as_mut().unwrap().label = label.into();
        expect_err(check(&bad, &ev), "LabelOutranksAssumptions", label);
    }
    let mut bad = r.clone();
    bad.claims[0].proof.as_mut().unwrap().remaining_by_layer.insert("wir".into(), 1);
    expect_err(check(&bad, &ev), "LabelOutranksAssumptions", "remaining_by_layer");
    let mut bad = r.clone();
    bad.claims[0].proof.as_mut().unwrap().rung = "R9".into();
    expect_err(check(&bad, &ev), "LabelOutranksAssumptions", "rung");
}

// ---- AC-12 NonClaimReported -------------------------------------------------

#[test]
fn ac12_a_declared_non_claim_is_never_reported_as_proved() {
    let (g, gev) = gf();
    let mut bad = g.clone();
    let w = bad.weaknesses.iter_mut().find(|w| w.cwe == 208).unwrap();
    w.status = WeaknessStatus::Discharged;
    expect_err(check(&bad, &gev), "NonClaimReported", "MA01");

    let mut gev2 = gev.clone();
    let p = gev2.proofs.iter_mut().find(|p| p.kind == "constant-time").unwrap();
    p.verdict = "pass".into();
    p.label = "proved".into();
    expect_err(check(&g, &gev2), "NonClaimReported", "retire the non-claim");

    // ct-compare *does* prove constant time; a non-claim would contradict it.
    let (r, mut ev) = ct();
    ev.catalogue.non_claims.push(NonClaim {
        package: "rust/ct-compare".into(),
        cwe: 208,
        claim: "constant-time".into(),
        source: "code/specs/CT01-constant-time-compare.md".into(),
    });
    expect_err(check(&r, &ev), "NonClaimReported", "passes");
}

// ---- AC-13 ExemptionAssumptionMismatch --------------------------------------

#[test]
fn ac13_exemptions_list_exactly_what_they_derive_from() {
    let (r, ev) = ct();
    let gate = r.exemptions.iter().position(|x| x.id == "EXEMPT-CWE-208-GATE-LEVEL").unwrap();
    assert!(r.exemptions[gate].assumptions.iter().any(|a| a == "wir.cost.step-model"));
    assert!(r.exemptions[gate].assumptions.iter().all(|a| {
        let e = ev.catalogue.catalogue.get(a).unwrap();
        ["cc", "mc", "uarch", "gate", "phys"].contains(&e.layer.as_str()) || a == "wir.cost.step-model"
    }));
    let unsafe_ = r.exemptions.iter().find(|x| x.id == "EXEMPT-CWE-416-UNSAFE-EXCEPTION").unwrap();
    assert_eq!(unsafe_.assumptions, vec!["src.rust.unsafe-region.sound", "wir.lowering.volatile-by-name"]);

    let mut bad = r.clone();
    bad.exemptions[gate].assumptions.retain(|a| a != "phys.silicon.realization");
    expect_err(check(&bad, &ev), "ExemptionAssumptionMismatch", "phys.silicon.realization");
    let mut bad = r.clone();
    bad.exemptions[gate].assumptions.push("spec.obs-model.complete".into());
    expect_err(check(&bad, &ev), "ExemptionAssumptionMismatch", "spec.obs-model.complete");
    let mut bad = r.clone();
    bad.exemptions[0].id = "EXEMPT-CWE-79-HANDWAVE".into();
    expect_err(check(&bad, &ev), "ExemptionAssumptionMismatch", "no derivation rule");
}

// ---- AC-10 UnusedAssumption --------------------------------------------------

#[test]
fn ac10_an_entry_no_claim_uses_is_flagged_and_fails_under_strict() {
    let (_, mut ev) = ct();
    // Narrow one leaf entry to a label no ct-compare claim carries.
    entry_mut(&mut ev, "uarch.shared-resources.none").applies_to.labels = vec!["proved-to-isa".into()];
    let r = generate_ct_compare_report("c0ffee", &WeaknessRegistry::embedded(), &ev).unwrap();
    let s = check(&r, &ev).expect("a warning, not an error");
    assert_eq!(s.unused, vec!["uarch.shared-resources.none".to_string()]);
    match check_assumptions(&r, &ev, true) {
        Err(e) => {
            assert_eq!(e.code(), ("AC-10", "UnusedAssumption"));
            assert!(e.to_string().contains("uarch.shared-resources.none"));
        }
        Ok(_) => panic!("--strict must fail on an unused entry"),
    }
}

#[test]
fn cli_strict_turns_the_ac10_warning_into_a_failure() {
    let bin = env!("CARGO_BIN_EXE_ca-assurance-report-rs");
    let report = rust_dir("gf256").join("assurance_report.json");
    let out = std::process::Command::new(bin)
        .args(["--check", report.to_str().unwrap(), "--allow-unverified"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("warning: AC-10 UnusedAssumption: tcb.report.checker-correct"), "{stdout}");
    let out = std::process::Command::new(bin)
        .args(["--check", report.to_str().unwrap(), "--allow-unverified", "--strict"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("AC-10 UnusedAssumption"));
}

// ---- report parsing ---------------------------------------------------------

#[test]
fn reports_without_the_ver13_fields_are_refused_not_defaulted() {
    let (r, _) = ct();
    let good = r.to_json_string();
    assert_eq!(AssuranceReport::from_json_str(&good).unwrap(), r, "round trip");
    let cut = |from: &str, to: &str| {
        assert!(good.contains(from), "{from}");
        AssuranceReport::from_json_str(&good.replacen(from, to, 1))
    };
    assert!(cut("\"catalogue\": {", "\"catalogue_gone\": {").is_err(), "missing pin");
    assert!(cut("\"stack\": \"host-rustc\"", "\"stack\": 7").is_err(), "mistyped stack");
    assert!(cut("\"assumptions\": [\n        \"cc.caller-context\"", "\"assumptions\": [\n        1").is_err(), "non-string id");
    assert!(cut("\"discharged\": []", "\"discharged\": [{\"id\": \"x\"}]").is_err(), "incomplete discharge");
    assert!(cut("\"remaining_by_layer\": {", "\"remaining_by_layer\": {\"wir\": -1, ").is_err(), "negative count");
}

#[test]
fn evidence_ids_classify_into_exactly_one_kind() {
    let set = CatalogueSet::load_from_root(&repo_root()).unwrap();
    let k = |id: &str| set.kinds.classify(id).unwrap().map(|k| k.kind.clone());
    assert_eq!(k("warrant/ct_compare/negctl/example/eq/invert").as_deref(), Some("negative-control"));
    assert_eq!(k("warrant/ct_compare/proof/eq/steps-bound").as_deref(), Some("steps-bound-obligation"));
    assert_eq!(k("warrant/ct_compare/proofneg/eq/secret-branch").as_deref(), Some("non-collapsed-negative-control"));
    assert_eq!(k("cc/ct_compare/ir-taint/eq").as_deref(), Some("ir-taint-analysis"));
    assert_eq!(k("bench/ct_eq/16_bytes"), None, "a plain benchmark proves no assumption away");
    assert!(set.kinds.get("ir-taint-analysis").unwrap().per_run);
    assert!(parse_pattern("*/x").is_err() && parse_pattern("a//b").is_err() && parse_pattern("a/b c").is_err());
}
