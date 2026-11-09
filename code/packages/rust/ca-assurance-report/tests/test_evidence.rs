//! VER04h section 9: the assurance gate must fail closed on evidence that does
//! not exist, disagrees with the manifests, or does not match its files.  Each
//! test corrupts one thing in a throw-away copy of the real evidence layout and
//! asserts the specific failure mode (plus one test that the untouched copy
//! passes, so the failures are caused by the corruption and by nothing else).

mod common;

use common::Sandbox;

use coding_adventures_ca_assurance_report::evidence::{label_rank, MEMORY_SAFETY_CWES};
use coding_adventures_ca_assurance_report::{
    derive_claims, generate_ct_compare_report, verify_evidence, AssuranceReport, Evidence,
    EvidenceCheckError as E, WeaknessRegistry, WeaknessStatus,
};
use coding_adventures_sha256::sha256_hex;

fn build(sb: &Sandbox) -> (AssuranceReport, Evidence) {
    let ev = Evidence::load(&sb.pkg).expect("evidence loads");
    let report = generate_ct_compare_report("c0ffee", &WeaknessRegistry::embedded(), &ev)
        .expect("report derives from evidence");
    (report, ev)
}

#[test]
fn untouched_evidence_verifies_and_counts_come_from_the_manifest() {
    let sb = Sandbox::new("ok");
    let (report, ev) = build(&sb);
    let unverified = verify_evidence(&report, &ev).expect("a derived report must verify");
    assert!(!unverified.is_empty(), "static attestations are surfaced, not hidden");

    // No literal survives: the numbers equal the manifest sums.
    let eq = report.claims.iter().find(|c| c.symbol == "ct_eq").unwrap();
    let want: u64 = ev.tests.iter().filter(|t| t.kind == "property" && t.op == "eq").map(|t| t.cases).sum();
    assert_eq!(eq.property.cases, want);
    assert_ne!(eq.property.cases, 10_000, "the old hard-coded literal is gone");
    assert_ne!(eq.fuzz.as_ref().unwrap().iterations, 100_000);
    assert!(!eq.proof.as_ref().unwrap().certificate.contains("..."));
    // Labels are the real bound, not the old `proved` / `bounded(4096)`.
    assert_eq!(eq.proof.as_ref().unwrap().label, "bounded(128)");
    let u64c = report.claims.iter().find(|c| c.symbol == "ct_eq_u64").unwrap();
    assert_eq!(u64c.proof.as_ref().unwrap().label, "proved");
    assert_eq!(derive_claims(&ev).unwrap().len(), 4);
}

#[test]
fn the_checked_in_report_matches_a_fresh_derivation() {
    let pkg = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ct-compare");
    let ev = Evidence::load(&pkg).unwrap();
    let text = std::fs::read_to_string(pkg.join("assurance_report.json")).unwrap();
    let on_disk = AssuranceReport::from_json_str(&text).unwrap();
    verify_evidence(&on_disk, &ev).expect("assurance_report.json must agree with the manifests");
    let fresh = generate_ct_compare_report(&on_disk.commit, &WeaknessRegistry::embedded(), &ev).unwrap();
    assert_eq!(fresh.to_json_string(), on_disk.to_json_string(), "regenerate assurance_report.json");
}

// ---- failure mode 1: evidence ids that no manifest contains ----------------

#[test]
fn unknown_evidence_ids_are_rejected_everywhere_they_can_appear() {
    let sb = Sandbox::new("unknown");
    let (report, ev) = build(&sb);
    let legacy = "ct-compare::tests::bench_instruction_counts::equal-retired-instructions";

    let mut r = report.clone();
    r.claims[0].evidence.push(legacy.into());
    assert_eq!(verify_evidence(&r, &ev), Err(E::UnknownEvidence(legacy.into())));

    let mut r = report.clone();
    r.weaknesses.iter_mut().find(|w| w.cwe == 400).unwrap().evidence.push("warrant-ir::linear-step-bound".into());
    assert!(matches!(verify_evidence(&r, &ev), Err(E::UnknownEvidence(id)) if id.contains("linear-step-bound")));

    let mut r = report.clone();
    r.exemptions[0].evidence = vec!["warrant::claims::secret-labels-verified".into()];
    assert!(matches!(verify_evidence(&r, &ev), Err(E::UnknownEvidence(_))));

    // A claim with no manifest entry behind it at all.
    let mut r = report.clone();
    let mut extra = r.claims[0].clone();
    extra.id = "ct_phantom_soundness".into();
    r.claims.push(extra);
    assert!(matches!(verify_evidence(&r, &ev), Err(E::UnknownEvidence(_))));
}

#[test]
fn cited_evidence_that_failed_is_rejected() {
    let sb = Sandbox::new("failing");
    let (report, _) = build(&sb);
    sb.edit(
        "evidence/test-manifest.json",
        "\"id\": \"warrant/ct_compare/fuzz/eq_agrees\",\n      \"iterations\"",
        "\"id\": \"warrant/ct_compare/fuzz/eq_agrees\",\n      \"iterations\"",
    );
    let text = sb.read("evidence/test-manifest.json");
    // Flip the verdict of exactly the fuzz entry for eq_agrees.
    let marker = "\"id\": \"warrant/ct_compare/fuzz/eq_agrees\"";
    let at = text.find(marker).unwrap();
    let verdict_at = at + text[at..].find("\"verdict\": \"pass\"").unwrap();
    let mut flipped = text.clone();
    flipped.replace_range(verdict_at..verdict_at + 17, "\"verdict\": \"fail\"");
    sb.write("evidence/test-manifest.json", &flipped);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::FailingEvidence(m)) if m.contains("fuzz/eq_agrees")));
    // And a report cannot be generated from failing evidence that passes the gate.
    let regenerated = generate_ct_compare_report("x", &WeaknessRegistry::embedded(), &ev).unwrap();
    let eq = regenerated.claims.iter().find(|c| c.symbol == "ct_eq").unwrap();
    assert_eq!(eq.fuzz.as_ref().unwrap().status, "fail");
}

// ---- failure mode 2: counts and labels disagree with the manifests ---------

#[test]
fn counts_labels_and_digests_must_equal_the_manifest() {
    let sb = Sandbox::new("mismatch");
    let (report, ev) = build(&sb);
    let mutate = |f: &dyn Fn(&mut AssuranceReport)| {
        let mut r = report.clone();
        f(&mut r);
        verify_evidence(&r, &ev)
    };
    let is_mismatch = |r: Result<Vec<String>, E>, what: &str| match r {
        Err(E::EvidenceMismatch(m)) => assert!(m.contains(what), "{m}"),
        other => panic!("expected mismatch on {what}, got {other:?}"),
    };
    is_mismatch(mutate(&|r| r.claims[0].property.cases = 10_000), "property cases");
    is_mismatch(mutate(&|r| r.claims[0].fuzz.as_mut().unwrap().iterations = 100_000), "fuzz");
    is_mismatch(mutate(&|r| r.claims[0].exhaustive.as_mut().unwrap().cases += 1), "exhaustive");
    is_mismatch(mutate(&|r| r.claims[0].proof.as_mut().unwrap().label = "proved".into()), "proof");
    is_mismatch(mutate(&|r| r.claims[0].proof.as_mut().unwrap().label = "bounded(4096)".into()), "proof");
    is_mismatch(
        mutate(&|r| r.claims[0].proof.as_mut().unwrap().certificate = "drat:sha256:e2a4c386892...".into()),
        "proof",
    );
    is_mismatch(mutate(&|r| r.claims[0].example = "fail".into()), "example");
    is_mismatch(mutate(&|r| r.claims[0].evidence.pop().map(|_| ()).unwrap_or(())), "evidence ids");
    is_mismatch(mutate(&|r| r.rungs.e4 = "fail".into()), "rungs");
    // The correct values are accepted.
    assert!(mutate(&|_| ()).is_ok());
}

// ---- failure mode 3: certificates --------------------------------------------

fn first_certificate(ev: &Evidence) -> String {
    ev.proofs.iter().flat_map(|p| p.certificates.iter()).next().unwrap().id.clone()
}

#[test]
fn certificate_files_must_exist_match_their_hash_and_check() {
    // Missing file.
    let sb = Sandbox::new("cert-missing");
    let (report, ev) = build(&sb);
    let id = first_certificate(&ev);
    std::fs::remove_file(sb.pkg.join(format!("evidence/proofs/{id}.drat"))).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::BadCertificate(m)) if m.contains("unreadable")));

    // Tampered DRAT: the contents no longer hash to the id.
    let sb = Sandbox::new("cert-tamper");
    let (report, ev) = build(&sb);
    let id = first_certificate(&ev);
    let path = sb.pkg.join(format!("evidence/proofs/{id}.drat"));
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("0\n");
    std::fs::write(&path, text).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::BadCertificate(m)) if m.contains("hash")));

    // Hash-consistent but not a refutation: the DRAT checker must still reject.
    let sb = Sandbox::new("cert-bogus");
    let (_, ev) = build(&sb);
    let old = first_certificate(&ev);
    let cnf = "p cnf 2 1\n1 2 0\n"; // satisfiable: no refutation can exist
    let drat = "";
    let mut bytes = cnf.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend_from_slice(drat.as_bytes());
    let new = sha256_hex(&bytes);
    std::fs::write(sb.pkg.join(format!("evidence/proofs/{new}.cnf")), cnf).unwrap();
    std::fs::write(sb.pkg.join(format!("evidence/proofs/{new}.drat")), drat).unwrap();
    let manifest = sb.read("evidence/proof-manifest.json").replace(&old, &new);
    sb.write("evidence/proof-manifest.json", &manifest);
    let ev = Evidence::load(&sb.pkg).unwrap();
    let report = generate_ct_compare_report("x", &WeaknessRegistry::embedded(), &ev).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::BadCertificate(m)) if m.contains("DRAT checker rejected")));
}

// ---- failure mode 4: Warrant declarations without evidence -------------------

#[test]
fn a_warrant_claim_without_evidence_is_uncovered() {
    // A claim is added to the spec and both manifests are re-stamped with the
    // new spec hash (as a careless regeneration would): coverage still fails.
    let sb = Sandbox::new("uncovered-claim");
    let (report, _) = build(&sb);
    let spec = std::fs::read_to_string(sb.spec()).unwrap();
    let new_spec = format!("{spec}\n  claim eq_reflexive:\n    forall a: Bytes. eq(a, a) == true\n");
    std::fs::write(sb.spec(), &new_spec).unwrap();
    let (old_sha, new_sha) = (sha256_hex(spec.as_bytes()), sha256_hex(new_spec.as_bytes()));
    for m in ["evidence/test-manifest.json", "evidence/proof-manifest.json"] {
        let t = sb.read(m).replace(&old_sha, &new_sha);
        sb.write(m, &t);
    }
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::UncoveredClaim(m)) if m.contains("eq_reflexive")));
}

#[test]
fn a_missing_suite_or_proof_entry_is_uncovered() {
    // Remove a whole manifest entry by renaming its id to something unrelated.
    let sb = Sandbox::new("uncovered-entry");
    let (report, _) = build(&sb);
    sb.edit("evidence/test-manifest.json", "warrant/ct_compare/exhaustive/select_picks", "warrant/ct_compare/exhaustive/renamed");
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::UncoveredClaim(m)) if m.contains("select_picks")));

    let sb = Sandbox::new("uncovered-example");
    let (report, _) = build(&sb);
    sb.edit("evidence/test-manifest.json", "warrant/ct_compare/example/u64_diff", "warrant/ct_compare/example/renamed");
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::UncoveredClaim(m)) if m.contains("u64_diff")));

    // `secret` labels with no constant-time obligation recorded.
    let sb = Sandbox::new("uncovered-proof");
    let (report, _) = build(&sb);
    let t = sb.read("evidence/proof-manifest.json").replace("\"kind\": \"constant-time\"", "\"kind\": \"renamed\"");
    sb.write("evidence/proof-manifest.json", &t);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::UncoveredClaim(m)) if m.contains("constant-time")));

    // A report that omits the claim for an operation.
    let sb = Sandbox::new("uncovered-report");
    let (mut report, ev) = build(&sb);
    report.claims.retain(|c| c.symbol != "ct_eq_u64");
    assert!(matches!(verify_evidence(&report, &ev), Err(E::UncoveredClaim(m)) if m.contains("eq_u64")));
}

// ---- failure mode 5: stale evidence -------------------------------------------

#[test]
fn manifests_must_describe_the_spec_and_source_on_disk() {
    let sb = Sandbox::new("stale-spec");
    let (report, _) = build(&sb);
    let spec = std::fs::read_to_string(sb.spec()).unwrap();
    std::fs::write(sb.spec(), format!("{spec}\n")).unwrap();
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(m)) if m.contains("spec")));

    let sb = Sandbox::new("stale-source");
    let (report, _) = build(&sb);
    let src = sb.read("src/lib.rs");
    sb.write("src/lib.rs", &format!("{src}\n// edited after the proofs ran\n"));
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(m)) if m.contains("src/lib.rs")));

    // Only the proof manifest was produced from a different spec.
    let sb = Sandbox::new("stale-proof-spec");
    let (report, ev0) = build(&sb);
    let t = sb.read("evidence/proof-manifest.json").replace(&ev0.spec.sha256, &"0".repeat(64));
    sb.write("evidence/proof-manifest.json", &t);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(m)) if m.contains("proof manifest")));
}

// ---- failure mode 6 (AF-22): "no unsafe" next to a declared exception ----------

#[test]
fn a_no_unsafe_claim_contradicts_a_declared_unsafe_exception() {
    let sb = Sandbox::new("unsafe");
    let (report, ev) = build(&sb);
    assert!(!ev.cert.unsafe_exceptions.is_empty(), "ct-compare declares an unsafe_code exception");
    // The derived report never excludes a memory-safety class on that basis.
    for w in report.weaknesses.iter().filter(|w| MEMORY_SAFETY_CWES.contains(&w.cwe)) {
        assert_ne!(w.status, WeaknessStatus::Excluded, "CWE-{}", w.cwe);
        assert!(!w.evidence.contains(&"analyzer::no-unsafe-or-ffi".to_string()));
    }
    // The old report's claim is rejected with the dedicated error.
    let mut lie = report.clone();
    let row = lie.weaknesses.iter_mut().find(|w| w.cwe == 416).unwrap();
    row.status = WeaknessStatus::Excluded;
    row.approved = None;
    row.reason = "safe Rust: static analyzer proved no unsafe blocks and no FFI reachability".into();
    row.evidence = vec!["analyzer::no-unsafe-or-ffi".into(), "effect-certificate::proved".into()];
    lie.exemptions.retain(|e| e.cwe != 416);
    assert!(matches!(verify_evidence(&lie, &ev), Err(E::UnsafeContradiction(m)) if m.contains("CWE-416")));
    // Even with innocuous evidence ids, an Excluded memory-safety row is a lie.
    let mut lie2 = report.clone();
    let row = lie2.weaknesses.iter_mut().find(|w| w.cwe == 125).unwrap();
    row.status = WeaknessStatus::Excluded;
    row.evidence = vec!["effect-certificate::proved".into()];
    assert!(matches!(verify_evidence(&lie2, &ev), Err(E::UnsafeContradiction(_))));
}

#[test]
fn removing_the_unsafe_exception_restores_the_exclusion() {
    // The attestation is derived from the certificate, so the same evidence
    // set supports "no unsafe" exactly when no exception is declared.
    let sb = Sandbox::new("no-unsafe");
    let cert = sb.read("effect_certificate.json");
    let start = cert.find("  \"banned_construct_exceptions\"").unwrap();
    let end = cert[start..].find("],\n").unwrap() + start + 3;
    sb.write("effect_certificate.json", &format!("{}{}", &cert[..start], &cert[end..]));
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(ev.cert.unsafe_exceptions.is_empty());
    assert!(ev.attestation_ids().contains("analyzer::no-unsafe-or-ffi"));
    let report = generate_ct_compare_report("x", &WeaknessRegistry::embedded(), &ev).unwrap();
    assert_eq!(report.weaknesses.iter().find(|w| w.cwe == 416).unwrap().status, WeaknessStatus::Excluded);
    verify_evidence(&report, &ev).unwrap();
}

// ---- generation fails closed as well --------------------------------------------

#[test]
fn generation_refuses_incomplete_or_failing_evidence() {
    let registry = WeaknessRegistry::embedded();
    // Missing manifests.
    let sb = Sandbox::new("gen-missing");
    std::fs::remove_file(sb.pkg.join("evidence/proof-manifest.json")).unwrap();
    assert!(Evidence::load(&sb.pkg).is_err());
    std::fs::write(sb.pkg.join("evidence/proof-manifest.json"), "{ not json").unwrap();
    assert!(Evidence::load(&sb.pkg).is_err());
    std::fs::write(sb.pkg.join("evidence/proof-manifest.json"), "{\"schema\": \"other\"}").unwrap();
    assert!(Evidence::load(&sb.pkg).unwrap_err().to_string().contains("schema"));

    // An operation whose obligations vanished.
    let sb = Sandbox::new("gen-noproof");
    let t = sb.read("evidence/proof-manifest.json").replace("\"op\": \"eq_u64\"", "\"op\": \"gone\"");
    sb.write("evidence/proof-manifest.json", &t);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(generate_ct_compare_report("x", &registry, &ev).is_err());

    // A failing bench manifest removes the zero-allocation evidence: CWE-400 is open.
    let sb = Sandbox::new("gen-bench");
    let t = sb.read("evidence/bench-manifest.json").replace("\"verdict\": \"pass\"", "\"verdict\": \"fail\"");
    sb.write("evidence/bench-manifest.json", &t);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(!ev.audit_context("rust/ct-compare").has_zero_alloc_proof);
    assert!(generate_ct_compare_report("x", &registry, &ev).is_err());

    // A failing proof leaves the CT flag false and the label unranked.
    let sb = Sandbox::new("gen-proof-fail");
    let t = sb.read("evidence/proof-manifest.json").replacen("\"verdict\": \"pass\"", "\"verdict\": \"fail\"", 1);
    sb.write("evidence/proof-manifest.json", &t);
    let ev = Evidence::load(&sb.pkg).unwrap();
    let derived = derive_claims(&ev).unwrap();
    assert!(derived.iter().any(|c| matches!(c.proof.as_ref().unwrap().label.as_str(), "unknown" | "falsified")));
}

#[test]
fn proof_labels_are_ranked_by_what_they_cover() {
    assert_eq!(label_rank("proved"), Some(u64::MAX));
    assert_eq!(label_rank("bounded(128)"), Some(128));
    assert!(label_rank("bounded(8)") < label_rank("bounded(128)"));
    assert!(label_rank("bounded(128)") < label_rank("proved"));
    for bad in ["unknown", "falsified", "unchecked", "bounded(x)", "bounded(", ""] {
        assert_eq!(label_rank(bad), None, "{bad}");
    }
}

#[test]
fn context_flags_come_from_manifests() {
    let sb = Sandbox::new("flags");
    let ev = Evidence::load(&sb.pkg).unwrap();
    let ctx = ev.audit_context("rust/ct-compare");
    assert!(ctx.has_pbt_coverage && ctx.has_fuzz_coverage && ctx.has_wir_timing_proof);
    assert!(!ctx.is_safe_rust, "the unsafe exception must flow into the context");
    assert_eq!(ctx.evidence.hw_counters, ev.hw_instruction_counters);
    assert!(ctx.evidence.ct_proof_ids.iter().all(|id| id.contains("/proof/")));
    // Flip every fuzz verdict: the flag must drop.
    let t = sb.read("evidence/test-manifest.json");
    let mut out = String::new();
    let mut rest = t.as_str();
    while let Some(i) = rest.find("\"kind\": \"fuzz\"") {
        let (head, tail) = rest.split_at(i);
        out.push_str(head);
        let v = tail.find("\"verdict\": \"pass\"").unwrap();
        out.push_str(&tail[..v]);
        out.push_str("\"verdict\": \"fail\"");
        rest = &tail[v + 17..];
    }
    out.push_str(rest);
    sb.write("evidence/test-manifest.json", &out);
    let ctx = Evidence::load(&sb.pkg).unwrap().audit_context("rust/ct-compare");
    assert!(!ctx.has_fuzz_coverage);
    assert!(ctx.has_pbt_coverage);
}
