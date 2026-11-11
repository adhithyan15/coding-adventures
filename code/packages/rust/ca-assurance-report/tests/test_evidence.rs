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
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(m)) if m.contains("src/**/*.rs")));

    // N2: the binding covers every src file, not only lib.rs: editing
    // src/capabilities.rs alone makes the evidence stale.
    let sb = Sandbox::new("stale-capabilities");
    let (report, _) = build(&sb);
    let caps = sb.read("src/capabilities.rs");
    sb.write("src/capabilities.rs", &format!("{caps}\n// edited after the proofs ran\n"));
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(m)) if m.contains("src/**/*.rs")));
    // A new src file is covered too.
    let sb = Sandbox::new("stale-new-file");
    let (report, _) = build(&sb);
    sb.write("src/extra.rs", "pub fn extra() {}\n");
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(_))));

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

/// Strip the `unsafe_code` exception from the certificate (keeping the key,
/// which the strict loader requires).
fn drop_unsafe_exception(sb: &Sandbox) {
    let cert = sb.read("effect_certificate.json");
    let start = cert.find("  \"banned_construct_exceptions\"").unwrap();
    let end = cert[start..].find("],\n").unwrap() + start + 3;
    sb.write(
        "effect_certificate.json",
        &format!("{}  \"banned_construct_exceptions\": [],\n{}", &cert[..start], &cert[end..]),
    );
}

#[test]
fn removing_the_unsafe_exception_restores_the_exclusion() {
    // The attestation is derived from the certificate *and* the source scan, so
    // the same evidence set supports "no unsafe" exactly when neither the
    // certificate declares an exception nor the code contains `unsafe`.
    let sb = Sandbox::new("no-unsafe");
    drop_unsafe_exception(&sb);
    let old = sb.src_digest();
    let src = sb.read("src/lib.rs").replace("unsafe", "safe_");
    sb.write("src/lib.rs", &src);
    let new = sb.src_digest();
    for m in ["evidence/test-manifest.json", "evidence/proof-manifest.json", "evidence/bench-manifest.json"] {
        let t = sb.read(m).replace(&old, &new);
        sb.write(m, &t);
    }
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(ev.cert.unsafe_exceptions.is_empty() && ev.unsafe_source_files.is_empty());
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

// ===========================================================================
// Security review fixes B1-B9.  Each test below fails against the code as it
// was before the fix (the comment names the old behaviour).
// ===========================================================================

use coding_adventures_ca_assurance_report::evidence::{
    certificate_kind, has_unsafe_token, is_unsafe_bearing, read_capped, scan_unsafe_sources,
    MAX_CERTIFICATE_BYTES, MAX_MANIFEST_BYTES, TRIVIAL_MARK,
};
use coding_adventures_ca_assurance_report::json::JsonValue;
use coding_adventures_ca_assurance_report::{verify_assurance_report, CheckError};

/// Replace the text of one manifest entry: `edit(entry_text) -> new_entry_text`.
/// An entry is the `{ ... }` object holding `"id": "<id>"` (entries are flat, so
/// the nearest enclosing braces delimit it).
fn edit_entry(sb: &Sandbox, manifest: &str, id: &str, edit: &dyn Fn(&str) -> String) {
    let text = sb.read(manifest);
    let at = text.find(&format!("\"id\": \"{id}\"")).unwrap_or_else(|| panic!("{id} not in {manifest}"));
    let start = text[..at].rfind("\n    {\n").unwrap() + 1;
    let end = at + text[at..].find("\n    }").unwrap() + 6;
    let new = edit(&text[start..end]);
    sb.write(manifest, &format!("{}{}{}", &text[..start], new, &text[end..]));
}

// ---- B1: duplicate claim ids ------------------------------------------------

#[test]
fn b1_duplicate_claim_ids_are_rejected() {
    let sb = Sandbox::new("b1");
    let (report, ev) = build(&sb);
    // An exact duplicate used to pass: `find` only ever compared the first copy.
    let mut r = report.clone();
    r.claims.push(r.claims[0].clone());
    assert!(matches!(verify_evidence(&r, &ev), Err(E::DuplicateId(m)) if m.contains("ct_eq")));
    // A *tampered* second copy used to hide behind the genuine first one.
    let mut r = report.clone();
    let mut evil = r.claims[0].clone();
    evil.property.cases = 1;
    evil.proof.as_mut().unwrap().label = "proved".into();
    r.claims.push(evil);
    assert!(matches!(verify_evidence(&r, &ev), Err(E::DuplicateId(_))));
    // The claim count must equal the derived count, whatever the ids.
    let mut r = report.clone();
    r.claims.pop();
    assert!(verify_evidence(&r, &ev).is_err());
    assert!(verify_evidence(&report, &ev).is_ok());
}

// ---- B2: --check validates exhaustive status, proof labels, E3/E5 per op -------

#[test]
fn b2_checker_requires_exhaustive_pass_ranked_labels_and_no_unchecked_on_pass() {
    let registry = WeaknessRegistry::embedded();
    let sb = Sandbox::new("b2");
    let (report, _) = build(&sb);
    let mut r = report.clone();
    r.claims[0].exhaustive.as_mut().unwrap().status = "fail".into();
    assert!(matches!(verify_assurance_report(&r, &registry), Err(CheckError::FailingClaim(m)) if m.contains("exhaustive")));
    for bad in ["unknown", "falsified", "unchecked", "bounded(x)", ""] {
        let mut r = report.clone();
        r.claims[0].proof.as_mut().unwrap().label = bad.into();
        assert!(
            matches!(verify_assurance_report(&r, &registry), Err(CheckError::FailingClaim(m)) if m.contains("rank")),
            "label {bad:?}"
        );
    }
    // A `pass` verdict cannot carry an unchecked obligation as an assumption.
    let mut r = report.clone();
    for w in &mut r.weaknesses {
        if w.status == WeaknessStatus::Exemption {
            w.status = WeaknessStatus::Discharged;
            w.approved = None;
            w.evidence = vec!["gate-level-proof".into()];
        }
    }
    r.claims[0].proof.as_mut().unwrap().label = "proved-to-gate-level".into();
    r.exemptions.clear();
    r.rungs.e2 = "pass".into();
    r.verdict = "pass".into();
    // Unchecked rows are cited by catalogue id (`wir.bound.heap-unchecked`).
    assert!(r.claims.iter().any(|c| c.proof.as_ref().unwrap().assumptions.iter().any(|a| a.ends_with("-unchecked"))));
    assert!(matches!(verify_assurance_report(&r, &registry), Err(CheckError::FailingClaim(m)) if m.contains("unchecked")));
}

#[test]
fn b2_rungs_e3_and_e5_are_judged_per_operation_with_ranked_labels() {
    // E3 used to pool: eq_u64 losing every exhaustive entry still left some
    // exhaustive entries somewhere, so the rung stayed `pass`.
    let sb = Sandbox::new("b2-e3");
    assert_eq!(Evidence::load(&sb.pkg).unwrap().rungs().e3, "pass");
    let t = sb.read("evidence/test-manifest.json");
    let mut out = String::new();
    let mut rest = t.as_str();
    while let Some(i) = rest.find("\"id\": \"warrant/ct_compare/exhaustive/eq_u64") {
        let (head, tail) = rest.split_at(i);
        // The entry's `"op"` line follows its `"kind"` line (keys are sorted).
        let op_at = tail.find("\"op\": \"eq_u64\"").unwrap();
        out.push_str(head);
        out.push_str(&tail[..op_at]);
        out.push_str("\"op\": \"eq\"");
        rest = &tail[op_at + "\"op\": \"eq_u64\"".len()..];
    }
    out.push_str(rest);
    sb.write("evidence/test-manifest.json", &out);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(ev.tests.iter().any(|t| t.kind == "exhaustive"), "other ops still have exhaustive entries");
    assert_eq!(ev.rungs().e3, "fail");

    // E5 used to ask only for verdict `pass`; an unranked label must not count.
    let sb = Sandbox::new("b2-e5");
    assert_eq!(Evidence::load(&sb.pkg).unwrap().rungs().e5, "pass");
    sb.edit("evidence/proof-manifest.json", "\"label\": \"bounded(128)\"", "\"label\": \"unknown\"");
    assert_eq!(Evidence::load(&sb.pkg).unwrap().rungs().e5, "fail");
}

// ---- B3: effect certificate parsing and the source cross-check -----------------

#[test]
fn b3_malformed_effect_certificates_are_rejected_not_read_as_clean() {
    for (tag, cert) in [
        ("empty-object", "{}".to_string()),
        ("not-object", "[]".to_string()),
        ("label-number", r#"{"label": 7, "declared_effects": [], "banned_construct_exceptions": []}"#.into()),
        ("effects-type", r#"{"label": "proved", "declared_effects": "none", "banned_construct_exceptions": []}"#.into()),
        ("effects-entry", r#"{"label": "proved", "declared_effects": [1], "banned_construct_exceptions": []}"#.into()),
        ("exceptions-missing", r#"{"label": "proved", "declared_effects": []}"#.into()),
        ("exceptions-type", r#"{"label": "proved", "declared_effects": [], "banned_construct_exceptions": {}}"#.into()),
        ("exception-entry", r#"{"label": "proved", "declared_effects": [], "banned_construct_exceptions": ["unsafe"]}"#.into()),
        ("exception-construct", r#"{"label": "proved", "declared_effects": [], "banned_construct_exceptions": [{"justification": "x"}]}"#.into()),
        ("exception-why", r#"{"label": "proved", "declared_effects": [], "banned_construct_exceptions": [{"construct": "unsafe_code", "justification": 1}]}"#.into()),
    ] {
        let sb = Sandbox::new(&format!("b3-{tag}"));
        sb.write("effect_certificate.json", &cert);
        assert!(Evidence::load(&sb.pkg).is_err(), "{tag} must not load");
    }
}

#[test]
fn b3_any_unsafe_or_ffi_construct_counts_as_unsafe_bearing() {
    for c in ["unsafe_code", "Unsafe_Code", "UNSAFE-block", "ffi_call", "extern-FFI", "unsafe"] {
        assert!(is_unsafe_bearing(c), "{c}");
    }
    for c in ["panic", "todo_macro", "dyn_trait"] {
        assert!(!is_unsafe_bearing(c), "{c}");
    }
    let sb = Sandbox::new("b3-names");
    let cert = sb.read("effect_certificate.json").replace("\"unsafe_code\"", "\"Unsafe_Code\"");
    sb.write("effect_certificate.json", &cert);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert_eq!(ev.cert.unsafe_exceptions.len(), 1, "case must not hide the exception");
    assert!(!ev.attestation_ids().contains("analyzer::no-unsafe-or-ffi"));
}

#[test]
fn b3_source_with_unsafe_needs_a_declared_exception() {
    // Dropping the exception while the code still has `unsafe` used to produce a
    // clean `analyzer::no-unsafe-or-ffi` attestation.
    let sb = Sandbox::new("b3-src");
    drop_unsafe_exception(&sb);
    let e = Evidence::load(&sb.pkg).unwrap_err();
    assert!(e.0.contains("unsafe") && e.0.contains("lib.rs"), "{e}");
    // The same cross-check guards verify_evidence for an in-memory Evidence.
    let sb = Sandbox::new("b3-src2");
    let (report, mut ev) = build(&sb);
    ev.cert.unsafe_exceptions.clear();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::UnsafeContradiction(m)) if m.contains("lib.rs")));
    // Unsafe hiding in a submodule is found too.
    let sb = Sandbox::new("b3-submodule");
    drop_unsafe_exception(&sb);
    let clean = sb.read("src/lib.rs").replace("unsafe", "safe_");
    sb.write("src/lib.rs", &clean);
    std::fs::create_dir_all(sb.pkg.join("src/inner")).unwrap();
    std::fs::write(sb.pkg.join("src/inner/deep.rs"), "pub fn f() { unsafe { } }\n").unwrap();
    assert_eq!(scan_unsafe_sources(&sb.pkg).unwrap(), vec!["inner/deep.rs".to_string()]);
}

#[test]
fn b3_unsafe_scanner_ignores_comments_strings_and_lookalikes() {
    assert!(has_unsafe_token("fn f() { unsafe { x() } }"));
    assert!(has_unsafe_token("unsafe fn g() {}"));
    assert!(has_unsafe_token("unsafe impl Send for T {}"));
    assert!(has_unsafe_token("#[unsafe(no_mangle)] fn h() {}"));
    assert!(has_unsafe_token("/* c */ let a = 'x'; unsafe {}"));
    assert!(has_unsafe_token("let s = \"q\"; unsafe {}"));
    assert!(!has_unsafe_token("// unsafe in a comment\nfn f() {}"));
    assert!(!has_unsafe_token("/* unsafe /* nested unsafe */ still comment */ fn f() {}"));
    assert!(!has_unsafe_token("let s = \"unsafe \\\" unsafe\";"));
    assert!(!has_unsafe_token("let s = r#\"unsafe \" unsafe\"#;"));
    assert!(!has_unsafe_token("let s = b\"unsafe\";"));
    assert!(!has_unsafe_token("#![deny(unsafe_code)]\nfn unsafe_ish() {}\nlet is_unsafe = 1;"));
    assert!(!has_unsafe_token("fn f<'a>(x: &'a str) -> &'a str { x } let c = '\\''; let d = 'u';"));
}

#[test]
fn n1_escaped_quote_char_literal_does_not_desync_the_lexer() {
    // The audit reproducer: `'\''` used to swallow up to the next `'`, which
    // flipped string state so the later `unsafe` was read as inside a string.
    assert!(has_unsafe_token("fn f(){ g('\\'','\"'); unsafe { } }\nconst S:&str=\"x\";"));
    assert!(has_unsafe_token("let a = '\\\\'; let b = '\"'; unsafe {}"));
    assert!(has_unsafe_token("let a = '\\u{1F600}'; let b = '\\x41'; let c = '\\0'; unsafe {}"));
    assert!(has_unsafe_token("let a = b'\\''; let s = \"x\"; unsafe {}"));
    assert!(has_unsafe_token("let a = b'\\''; let q = '\"'; unsafe {}"));
    assert!(has_unsafe_token("let a = b'x'; let s = b\"q\\\"\"; unsafe {}"));
}

#[test]
fn n1_lifetimes_and_labels_next_to_quotes() {
    assert!(has_unsafe_token("fn f<'a>(x: &'a str) -> &'static str { \"s\"; unsafe {} }"));
    assert!(has_unsafe_token("'outer: loop { break 'outer; } let s = \"x\"; unsafe {}"));
    assert!(has_unsafe_token("fn f<'a, 'b>(x: &'a u8, y: &'b u8) { let c = '\"'; unsafe {} }"));
    assert!(!has_unsafe_token("fn f<'a>(x: &'a str) { let s = \"unsafe\"; }"));
    assert!(!has_unsafe_token("'outer: loop { let _ = \"unsafe\"; break 'outer; }"));
}

#[test]
fn n1_comments_raw_strings_and_byte_strings() {
    assert!(!has_unsafe_token("/* a /* unsafe */ unsafe */ fn f() {}"));
    assert!(has_unsafe_token("/* a /* b */ c */ unsafe {}"));
    assert!(has_unsafe_token("/* /* */ */ unsafe {}"));
    assert!(!has_unsafe_token("let s = r##\"unsafe \"# still \"## ;"));
    assert!(has_unsafe_token("let s = r##\"unsafe \"# still \"##; unsafe {}"));
    assert!(!has_unsafe_token("let s = br#\"unsafe\"#;"));
    assert!(has_unsafe_token("let s = br###\"a\"##\"###; unsafe {}"));
    assert!(!has_unsafe_token("let s = b\"unsafe \\\" unsafe\";"));
    assert!(has_unsafe_token("let s = b\"x\"; unsafe {}"));
    assert!(!has_unsafe_token("let s = r\"unsafe\";"));
}

#[test]
fn n1_lexer_fails_closed_on_untokenisable_input() {
    // None of these contains the word `unsafe`, yet each must count as unsafe
    // because the lexer cannot positively tokenise the file.
    for bad in [
        "let s = \"never closed",
        "/* never closed /* nested */",
        "let s = r#\"never closed\";",
        "let c = '",
        "let c = '\\",
        "let c = '\\q';",
        "let c = '\\u{zz}';",
        "let c = '\\x4';",
        "let c = ''';",
        "let c = b'ab';",
        "let c = b'\\';",
    ] {
        assert!(has_unsafe_token(bad), "{bad:?} must fail closed");
    }
    // And well-formed input without the keyword is still clean.
    assert!(!has_unsafe_token("fn f<'a>(x: &'a str) { let c = '\\''; let s = \"y\"; }"));
}

#[test]
fn n3_hostile_counts_fail_closed() {
    // Present-but-invalid counts are errors, not clamped to 0 or wrapped.
    for bad in ["-5", "9007199254740994", "1.5", "\"7\"", "1e300", "true"] {
        let sb = Sandbox::new("hostile-count");
        sb.edit("evidence/test-manifest.json", "\"cases\": 1,", &format!("\"cases\": {bad},"));
        let e = Evidence::load(&sb.pkg).err().unwrap_or_else(|| panic!("{bad} must be refused"));
        assert!(e.to_string().contains("cases"), "{bad}: {e}");
    }
    // Totals are summed with checked arithmetic: counts that would wrap u64
    // make claim derivation fail instead of reporting a small total.
    let sb = Sandbox::new("overflow-sum");
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    let mut dup = ev.tests.iter().find(|t| t.kind == "property").unwrap().clone();
    let op = dup.op.clone();
    dup.id = "warrant/test/dup".into();
    ev.tests.push(dup);
    for t in ev.tests.iter_mut().filter(|t| t.kind == "property" && t.op == op) {
        t.cases = u64::MAX / 2 + 1;
    }
    let e = derive_claims(&ev).unwrap_err();
    assert!(e.to_string().contains("overflow"), "{e}");
}

// ---- B4: proof certificates bound to obligations ----------------------------------

#[test]
fn b4a_a_passing_obligation_must_cite_a_certificate() {
    // Emptying one entry's list used to be invisible: other entries of the same
    // operation still contributed certificates to the digest.
    let sb = Sandbox::new("b4a");
    let (report, _) = build(&sb);
    edit_entry(&sb, "evidence/proof-manifest.json", "warrant/ct_compare/proof/eq/steps-bound", &|entry| {
        let start = entry.find("\"certificates\": [").unwrap();
        let end = start + entry[start..].find(']').unwrap() + 1;
        format!("{}\"certificates\": []{}", &entry[..start], &entry[end..])
    });
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::BadCertificate(m)) if m.contains("no certificate")));
}

#[test]
fn b4b_trivial_certificates_are_marked_and_the_mark_is_required() {
    let sb = Sandbox::new("b4b");
    let (report, ev) = build(&sb);
    let eq = report.claims.iter().find(|c| c.symbol == "ct_eq").unwrap();
    assert_eq!(certificate_kind(&ev, "eq"), TRIVIAL_MARK, "ct_eq's miter collapses to a constant");
    assert_eq!(eq.proof.as_ref().unwrap().certificate_kind, TRIVIAL_MARK);
    assert!(report.to_json_string().contains("\"certificate_kind\": \"trivial-miter\""));
    // Presenting it as an ordinary miter (or saying nothing) fails --check.
    for presented in ["miter", ""] {
        let mut r = report.clone();
        r.claims.iter_mut().find(|c| c.symbol == "ct_eq").unwrap().proof.as_mut().unwrap().certificate_kind =
            presented.into();
        assert!(
            matches!(verify_evidence(&r, &ev), Err(E::TrivialCertificateUnmarked(m)) if m.contains("trivial-miter")),
            "{presented:?}"
        );
    }
    // The marking survives a JSON round trip, and an unknown kind is refused at load.
    let again = AssuranceReport::from_json_str(&report.to_json_string()).unwrap();
    assert_eq!(again, report);
    sb.edit("evidence/proof-manifest.json", "\"kind\": \"trivial-miter-collapsed\"", "\"kind\": \"mystery\"");
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("unknown certificate kind"));
}

#[test]
fn b4c_every_discharged_obligation_carries_a_well_formed_obligation_digest() {
    let sb = Sandbox::new("b4c");
    let ev = Evidence::load(&sb.pkg).unwrap();
    let checked: Vec<_> = ev
        .proofs
        .iter()
        .filter(|p| (p.kind == "constant-time" || p.kind == "steps-bound") && p.verdict == "pass")
        .collect();
    assert!(checked.len() >= 8);
    for p in &checked {
        assert_eq!(p.obligation_sha256.len(), 64, "{}", p.id);
    }
    // Absent, or malformed, fails --check.
    for (tag, bad) in [("zz", "zz"), ("upper", "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789"), ("empty", "")] {
        let sb = Sandbox::new(&format!("b4c-{tag}"));
        let (report, ev) = build(&sb);
        let good = ev.proofs.iter().find(|p| p.id.ends_with("eq/constant-time")).unwrap().obligation_sha256.clone();
        sb.edit("evidence/proof-manifest.json", &good, bad);
        let ev = Evidence::load(&sb.pkg).unwrap();
        assert!(
            matches!(verify_evidence(&report, &ev), Err(E::BadCertificate(m)) if m.contains("obligation_sha256")),
            "{tag}"
        );
    }
}

// ---- B5: test and bench manifests are checked for source staleness ----------------

#[test]
fn b5_test_and_bench_manifests_must_match_the_source() {
    for (file, who) in [("evidence/test-manifest.json", "test manifest"), ("evidence/bench-manifest.json", "bench manifest")] {
        let sb = Sandbox::new(&format!("b5-{who}").replace(' ', "-"));
        let (report, ev) = build(&sb);
        assert!(sb.read(file).contains(&ev.source_sha256), "{file} records source_sha256");
        let t = sb.read(file).replace(&ev.source_sha256, &"0".repeat(64));
        sb.write(file, &t);
        let ev = Evidence::load(&sb.pkg).unwrap();
        assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(m)) if m.contains(who)), "{who}");
    }
    // A manifest that predates the field (empty hash) is stale as well.
    let sb = Sandbox::new("b5-missing");
    let (report, ev) = build(&sb);
    let t = sb.read("evidence/test-manifest.json").replace(&format!("  \"source_sha256\": \"{}\",\n", ev.source_sha256), "");
    sb.write("evidence/test-manifest.json", &t);
    let ev = Evidence::load(&sb.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(_))));
}

// ---- B6: the spec path ----------------------------------------------------------------

#[test]
fn b6_spec_paths_must_be_relative_pinned_and_consistent() {
    let spec = "\"spec\": \"code/specs/ct-compare.warrant\"";
    for (tag, bad) in [
        ("absolute", "\"spec\": \"/etc/passwd\""),
        ("dotdot", "\"spec\": \"code/specs/../specs/ct-compare.warrant\""),
        ("leading-dotdot", "\"spec\": \"../../code/specs/ct-compare.warrant\""),
        ("backslash", "\"spec\": \"code\\\\specs\\\\ct-compare.warrant\""),
        ("empty", "\"spec\": \"\""),
        ("other-doc", "\"spec\": \"code/specs/other.warrant\""),
    ] {
        for file in ["evidence/test-manifest.json", "evidence/proof-manifest.json"] {
            let sb = Sandbox::new(&format!("b6-{tag}-{}", &file[9..13]));
            // Make the redirected document exist so only the policy can refuse it.
            let _ = std::fs::copy(sb.spec(), sb.root.join("code/specs/other.warrant"));
            sb.edit(file, spec, bad);
            let e = Evidence::load(&sb.pkg).unwrap_err();
            assert!(e.0.contains("spec"), "{tag}/{file}: {e}");
        }
    }
    // Both manifests must describe the same module as the spec.
    for file in ["evidence/test-manifest.json", "evidence/proof-manifest.json", "evidence/bench-manifest.json"] {
        let sb = Sandbox::new(&format!("b6-module-{}", &file[9..13]));
        sb.edit(file, "\"module\": \"ct_compare\"", "\"module\": \"other\"");
        assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("module"), "{file}");
    }
}

// ---- B7: duplicate ids, duplicate JSON keys, attestation collisions ------------------

#[test]
fn b7_duplicate_and_reserved_ids_are_rejected() {
    // Within a manifest.
    let sb = Sandbox::new("b7-dup-test");
    sb.edit("evidence/test-manifest.json", "warrant/ct_compare/example/eq_one_diff", "warrant/ct_compare/example/eq_empty");
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("duplicate evidence id"));
    // Across manifests: a bench entry reusing a test id.
    let sb = Sandbox::new("b7-dup-cross");
    sb.edit("evidence/bench-manifest.json", "bench/ct_eq/16_bytes", "warrant/ct_compare/example/eq_empty");
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("duplicate evidence id"));
    // A manifest entry may not take an attestation id, nor an `unverified::` one.
    for reserved in ["analyzer::no-unsafe-or-ffi", "effect-certificate::proved", "capability-audit::fs-empty", "unverified::anything"] {
        let sb = Sandbox::new("b7-reserved");
        sb.edit("evidence/bench-manifest.json", "bench/ct_eq/16_bytes", reserved);
        assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("reserved"), "{reserved}");
    }
}

#[test]
fn b7_duplicate_json_keys_are_rejected() {
    assert!(JsonValue::parse(r#"{"a": 1, "a": 2}"#).unwrap_err().contains("duplicate key"));
    assert!(JsonValue::parse(r#"{"o": {"k": 1, "k": 1}}"#).is_err());
    assert!(JsonValue::parse(r#"{"a": 1, "b": 2}"#).is_ok());
    // The second `verdict` would have silently won, flipping a failure to a pass.
    let sb = Sandbox::new("b7-json");
    sb.edit("evidence/test-manifest.json", "\"verdict\": \"pass\"", "\"verdict\": \"fail\", \"verdict\": \"pass\"");
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("duplicate key"));
}

// ---- B8: the no-unsafe contradiction is judged on structured fields ---------------------

#[test]
fn b8_rewording_the_reason_cannot_hide_a_no_unsafe_claim() {
    let sb = Sandbox::new("b8");
    let (report, ev) = build(&sb);
    // Old check keyed on the substring "no unsafe blocks": reworded, a discharged
    // CWE-416 resting on the no-unsafe basis sailed through.
    let mut lie = report.clone();
    let row = lie.weaknesses.iter_mut().find(|w| w.cwe == 416).unwrap();
    row.status = WeaknessStatus::Discharged;
    row.approved = None;
    row.reason = "memory safe by construction".into();
    row.evidence = vec!["effect-certificate::proved".into()];
    lie.exemptions.retain(|e| e.cwe != 416);
    assert!(matches!(verify_evidence(&lie, &ev), Err(E::UnsafeContradiction(m)) if m.contains("CWE-416")));
    // 125/787 are dischargeable only on a proof-manifest id.
    let mut lie = report.clone();
    let row = lie.weaknesses.iter_mut().find(|w| w.cwe == 125).unwrap();
    row.evidence = vec!["effect-certificate::proved".into()];
    assert!(matches!(verify_evidence(&lie, &ev), Err(E::UnsafeContradiction(m)) if m.contains("CWE-125")));
    // Free text no longer matters in either direction: a harmless row whose
    // reason mentions the phrase is not a contradiction.
    let mut fine = report.clone();
    fine.weaknesses.iter_mut().find(|w| w.cwe == 400).unwrap().reason.push_str(" (there are no unsafe blocks in the harness)");
    verify_evidence(&fine, &ev).unwrap();
}

// ---- B9: size caps -------------------------------------------------------------------------

#[test]
fn b9_oversized_inputs_are_refused() {
    assert_eq!(MAX_MANIFEST_BYTES, 16 * 1024 * 1024);
    assert_eq!(MAX_CERTIFICATE_BYTES, 64 * 1024 * 1024);
    let dir = std::env::temp_dir().join(format!("ver04h-b9-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("f");
    std::fs::write(&f, vec![b'x'; 100]).unwrap();
    assert_eq!(read_capped(&f, 100).unwrap().len(), 100, "exactly the cap is allowed");
    assert!(read_capped(&f, 99).unwrap_err().0.contains("limit"));
    let _ = std::fs::remove_dir_all(&dir);

    // End to end: a manifest one byte over the cap is refused before parsing.
    let sb = Sandbox::new("b9");
    sb.write("evidence/bench-manifest.json", &" ".repeat(MAX_MANIFEST_BYTES as usize + 1));
    assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("limit"));
}
