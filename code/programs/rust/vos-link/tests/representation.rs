//! The `representation` tier (VOS02 section 2.7, R6 rule 2; backlog VOS-D061).
//!
//! The tier is the CONTRACT's: `obligations-representation.part` holds eight claims over an
//! implementation-supplied `rep` and `hist`, and `vos-link` treats it like any other tier (a
//! digest, a block, `carries`).  There is no `mode` key and no claim-shape lint (two security
//! reviews showed a syntactic lint over authored claims does not converge); what is left to check
//! is that a production crate carries exactly one of `canonical` and `representation`, that
//! `any` carries neither, that `rep` and `hist` exist (the Warrant type check, CD04), and that a
//! tampered claim is a digest mismatch.  Each control asserts its own finding.

mod common;
use common::*;

const REP: &str = "vos-fixture-rep";
const BITMAP: &str = "vos-fixture-bitmap";
const ANY: &str = "vos-fixture-any";
const IMPL: &str = "code/packages/rust/vos-fixture-rep/warrant/impl.part";
const TOML: &str = "code/packages/rust/vos-fixture-rep/vos.toml";

fn has(o: &vos_link::Outcome, code: &str, needle: &str) -> bool {
    o.findings
        .iter()
        .any(|f| f.code == code && f.message.contains(needle))
}

/// The bytes of the contract's representation part, as copied into the test repository.
fn rep_part(r: &Repo) -> String {
    r.read("code/packages/rust/vos-sched-contract/warrant/obligations-representation.part")
}

/// `rep` and `hist` for a crate whose `Impl` is a mask and one more byte (or just a mask): the two
/// models the representation tier names, so that the block type-checks against the crate.
const BITMAP_REP: &str =
    "  model rep(m: Ready, h: U8) -> Impl = build(m)\n  model hist(s: Impl) -> U8 = 0u8\n";
const ANY_REP: &str = "  model rep(m: Ready, h: U8) -> Impl = Impl(build(m).mask, h)\n  model hist(s: Impl) -> U8 = s.choice\n";

// ---------------------------------------------------------------------------
// The positive fixture
// ---------------------------------------------------------------------------

#[test]
fn a_representation_crate_passes_and_prints_the_derived_mode() {
    let r = Repo::with("rep-ok", REP);
    let g = r.gen(REP);
    assert!(g.findings.is_empty(), "{}", show(&g));
    assert_eq!(carried(&g), ["representation", "safety"]);
    assert_eq!(
        g.header,
        "crate vos-fixture-rep, kind production, mode representation, contract vos-sched-contract, obligations [representation,safety]"
    );
    let c = r.check(REP);
    assert!(c.findings.is_empty(), "{}", show(&c));
    assert_eq!(carried(&c), ["representation", "safety"]);
    assert_eq!(c.header, g.header);
}

#[test]
fn the_representation_block_is_embedded_byte_for_byte_with_its_digest() {
    let r = Repo::with("rep-block", REP);
    r.gen(REP);
    let doc = r.doc(REP);
    let part = rep_part(&r);
    let head = doc
        .find("  -- vos02:obligations-begin tier=representation sha256=")
        .expect("a representation marker");
    // Sorted between policy and safety: the safety marker comes after it, in this document.
    assert!(doc.find("tier=safety").unwrap() > head);
    assert!(doc.contains(&part), "the part appears verbatim");
}

#[test]
fn the_document_is_the_golden_text() {
    let r = Repo::with("rep-golden", REP);
    r.gen(REP);
    let doc = r.doc(REP);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/expected/vos-fixture-rep.warrant");
    if std::env::var("VOS_LINK_BLESS").as_deref() == Ok("1") {
        assert!(
            std::env::var_os("CI").is_none(),
            "VOS_LINK_BLESS is refused when CI is set"
        );
        std::fs::write(&path, &doc).unwrap();
    }
    let golden = std::fs::read_to_string(&path).expect("the golden file");
    assert_eq!(
        doc, golden,
        "run VOS_LINK_BLESS=1 cargo test --locked to accept an intended change"
    );
}

#[test]
fn a_representation_carrier_gets_its_own_note_not_the_cone_note() {
    let r = Repo::with("rep-notes", REP);
    let g = r.gen(REP);
    assert_eq!(g.notes.len(), 1, "{:?}", g.notes);
    assert!(
        g.notes[0].starts_with("representation tier:"),
        "{:?}",
        g.notes
    );
    assert!(g.notes[0].contains("could truthfully carry it"));
    assert!(g.notes[0].contains("VOS-D030") && g.notes[0].contains("VOS-D060"));
    // A canonical crate keeps the cone note and prints no mode.
    let b = Repo::with("rep-notes-canon", BITMAP);
    let g = b.gen(BITMAP);
    assert!(g.notes[0].starts_with("R6 cone rule:"), "{:?}", g.notes);
    assert!(!g.header.contains("mode"), "{}", g.header);
}

#[test]
fn the_cone_rule_is_skipped_for_a_representation_carrier() {
    // `build` sets the pointer to a constant: outside the cone, which is the point.  The same
    // `build` in a canonical crate is a CD07 finding (negative_controls: the oracle crates).
    let r = Repo::with("rep-cone", REP);
    let g = r.gen(REP);
    assert!(!has(&g, "CD07", "cone"), "{}", show(&g));
    assert!(g.findings.is_empty());
}

#[test]
fn a_mode_key_no_longer_exists() {
    // The mode is derived from the tier carried: a `mode` line is an unknown key, so two
    // author-written values can never disagree.
    let r = Repo::with("rep-no-mode-key", REP);
    r.edit(
        TOML,
        "kind = \"production\"",
        "kind = \"production\"\nmode = \"representation\"",
    );
    let g = r.gen(REP);
    assert!(has(&g, "GD05", "unknown key"), "{}", show(&g));
}

// ---------------------------------------------------------------------------
// R6 rule 2: exactly one of canonical and representation, by digest
// ---------------------------------------------------------------------------

#[test]
fn a_production_crate_carrying_both_canonical_and_representation_is_rejected() {
    let r = Repo::with("rep-both", BITMAP);
    let p = "code/packages/rust/vos-fixture-bitmap/warrant/impl.part";
    r.write(p, format!("{}{BITMAP_REP}", r.read(p)));
    r.write(
        "code/packages/rust/vos-fixture-bitmap/vos.toml",
        "kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"canonical\", \"policy\", \"representation\", \"safety\"]\n",
    );
    let g = r.gen(BITMAP);
    assert!(
        has(
            &g,
            "CD07",
            "carries both the `canonical` and the `representation` obligations"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn a_production_crate_carrying_neither_is_rejected() {
    let r = Repo::with("rep-neither", REP);
    r.edit(
        TOML,
        "obligations = [\"representation\", \"safety\"]",
        "obligations = [\"safety\"]",
    );
    let g = r.gen(REP);
    assert!(
        has(
            &g,
            "CD07",
            "does not carry the `canonical` obligations (R6), nor the `representation` obligations"
        ),
        "{}",
        show(&g)
    );
    assert!(
        !r.path(&format!("code/specs/{REP}.warrant")).exists(),
        "nothing is written"
    );
}

#[test]
fn a_production_crate_carrying_representation_and_any_is_rejected() {
    // Carries `any` and `representation` (the any fixture's `Impl(mask, choice)` has the oracle the
    // `any` block names, and `rep`/`hist` are given to it), but declares `production`.
    let r = Repo::with("rep-plus-any", ANY);
    let p = "code/packages/rust/vos-fixture-any/warrant/impl.part";
    r.write(p, format!("{}{ANY_REP}", r.read(p)));
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        "kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"any\", \"representation\", \"safety\"]\n",
    );
    let g = r.gen(ANY);
    assert!(
        has(
            &g,
            "CD07",
            "kind production: the document carries an `any` obligation"
        ),
        "{}",
        show(&g)
    );
    // and it is not "exactly one of canonical, representation" that is wrong: representation is carried.
    assert!(!has(&g, "CD07", "carries both"), "{}", show(&g));
}

#[test]
fn an_any_crate_carrying_representation_is_rejected() {
    let r = Repo::with("any-rep", ANY);
    let p = "code/packages/rust/vos-fixture-any/warrant/impl.part";
    r.write(p, format!("{}{ANY_REP}", r.read(p)));
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        "kind = \"any\"\ncontract = \"vos-sched-contract\"\nobligations = [\"any\", \"representation\", \"safety\"]\n",
    );
    let g = r.gen(ANY);
    assert!(
        has(
            &g,
            "CD07",
            "kind any: the document carries a `representation` claim"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn a_crate_named_any_may_not_be_production_even_with_the_representation_tier() {
    let r = Repo::new("rep-named-any");
    common::copy_dir(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/impls")
            .join(REP),
        &r.pkg("vos-fixture-rep-any"),
    );
    let g = r.gen("vos-fixture-rep-any");
    assert!(
        has(
            &g,
            "CD07",
            "a crate named `-any` declares kind = \"production\""
        ),
        "{}",
        show(&g)
    );
}

// ---------------------------------------------------------------------------
// CD04: rep and hist exist, because the block references them
// ---------------------------------------------------------------------------

#[test]
fn a_representation_crate_without_rep_or_hist_fails_the_type_check() {
    let r = Repo::with("rep-no-rep", REP);
    r.edit(
        IMPL,
        "  model rep(m: Ready, h: U8) -> Impl = Impl(build(m).mask, h)\n",
        "",
    );
    let g = r.gen(REP);
    assert!(
        has(&g, "CD04", "warrant-grammar refused the document"),
        "{}",
        show(&g)
    );
    let r = Repo::with("rep-no-hist", REP);
    r.edit(IMPL, "  model hist(s: Impl) -> U8 = s.ptr\n", "");
    let g = r.gen(REP);
    assert!(
        has(&g, "CD04", "warrant-grammar refused the document"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_rep_or_hist_with_the_wrong_signature_fails_the_type_check() {
    // hist must take an Impl and answer a U8; rep must take (Ready, U8).
    let (label, from, to) = (
        "rep-bad-hist",
        "model hist(s: Impl) -> U8 = s.ptr",
        "model hist(s: Impl) -> Bool = s.ptr < 8u8",
    );
    let r = Repo::with(label, REP);
    r.edit(IMPL, from, to);
    let g = r.gen(REP);
    assert!(has(&g, "CD04", "refused"), "{}", show(&g));
    let r = Repo::with("rep-bad-rep", REP);
    r.edit(
        IMPL,
        "model rep(m: Ready, h: U8) -> Impl = Impl(build(m).mask, h)",
        "model rep(m: Ready, h: Bool) -> Impl = build(m)",
    );
    let g = r.gen(REP);
    assert!(has(&g, "CD04", "refused"), "{}", show(&g));
}

// ---------------------------------------------------------------------------
// Digest controls: a tampered or weakened representation claim
// ---------------------------------------------------------------------------

#[test]
fn a_representation_claim_edited_in_the_document_is_a_digest_mismatch() {
    let r = Repo::with("rep-tamper-doc", REP);
    r.gen(REP);
    r.edit(
        &format!("code/specs/{REP}.warrant"),
        "claim dispatch_rep_post:",
        "claim dispatch_rep_post:  -- weakened",
    );
    let c = r.check(REP);
    assert!(
        has(&c, "CD02", "obligations block `representation`"),
        "{}",
        show(&c)
    );
    assert!(
        has(&c, "CD03", "obligations block `representation`"),
        "{}",
        show(&c)
    );
    assert!(codes(&c).contains(&"CD08"), "{}", show(&c));
}

#[test]
fn a_tampered_claim_with_the_marker_digest_recomputed_is_still_not_the_contracts() {
    // The attacker rewrites a claim to a tautology AND fixes the block's digest: CD02 is silent;
    // the copy no longer equals the contract's part (CD03) and the claim has another
    // claim_sha256, so the tier is only partly carried (CD06).
    let r = Repo::with("rep-tamper-sha", REP);
    r.gen(REP);
    let doc = r.doc(REP);
    let begin = doc
        .find("  -- vos02:obligations-begin tier=representation")
        .unwrap();
    let body_start = doc[begin..].find('\n').unwrap() + begin + 1;
    let body_end = body_start
        + doc[body_start..]
            .find("  -- vos02:obligations-end\n")
            .unwrap();
    let old = &doc[body_start..body_end];
    let new = old.replacen(
        "dec(alpha_w(rep(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), bit_and(k, 7u8)))) == ready_of(b0, b1, b2, b3, b4, b5, b6, b7)",
        "true",
        1,
    );
    assert_ne!(old, new);
    let forged = format!(
        "{}  -- vos02:obligations-begin tier=representation sha256={}\n{}{}",
        &doc[..begin],
        vos_manifest::digest::sha256_hex(new.as_bytes()),
        new,
        &doc[body_end..]
    );
    r.write(&format!("code/specs/{REP}.warrant"), forged);
    let c = r.check(REP);
    let got = codes(&c);
    assert!(!got.contains(&"CD02"), "{}", show(&c));
    assert!(
        has(&c, "CD03", "obligations block `representation`"),
        "{}",
        show(&c)
    );
    assert!(
        has(&c, "CD06", "tier `representation` is only partly carried"),
        "{}",
        show(&c)
    );
}

#[test]
fn a_weaker_claim_with_a_representation_obligations_name_is_cd05() {
    // The crate does not list the tier, so the real block is not embedded; its own `rep_alpha`
    // has the name of an obligation and another claim_sha256 (NC2).
    let r = Repo::with("rep-weak", REP);
    r.edit(
        TOML,
        "obligations = [\"representation\", \"safety\"]",
        "obligations = [\"safety\"]",
    );
    r.write(
        IMPL,
        format!(
            "{}  claim rep_alpha:\n    forall b: Bool. b == b\n",
            r.read(IMPL)
        ),
    );
    let g = r.gen(REP);
    assert!(
        has(
            &g,
            "CD05",
            "claim `rep_alpha` has the name of the representation obligation"
        ),
        "{}",
        show(&g)
    );
}
