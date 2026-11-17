//! Representation mode (VOS02 section 2.7, R6 rule 2 / CD07-R, D-GD2; backlog VOS-D061).
//!
//! The positive fixture `vos-fixture-rep` is a one-byte mask plus a round-robin pointer
//! (`Impl(mask, ptr)`), with the claim set VOS-012b will author.  Every negative control
//! here edits ONE thing in a copy of it and asserts the specific finding: a control that
//! merely "fails" could be failing for the wrong reason (VER04h section 6).

mod common;
use common::*;

const REP: &str = "vos-fixture-rep";
const IMPL: &str = "code/packages/rust/vos-fixture-rep/warrant/impl.part";
const TOML: &str = "code/packages/rust/vos-fixture-rep/vos.toml";

fn has(o: &vos_link::Outcome, code: &str, needle: &str) -> bool {
    o.findings
        .iter()
        .any(|f| f.code == code && f.message.contains(needle))
}

/// The findings of `gen-doc` on the fixture after one edit of `impl.part`.
fn mutated(label: &str, from: &str, to: &str) -> (Repo, vos_link::Outcome) {
    let r = Repo::with(label, REP);
    r.edit(IMPL, from, to);
    let g = r.gen(REP);
    (r, g)
}

/// Remove the claim `name` (its header line and its indented body) from the fixture.
fn without_claim(label: &str, name: &str) -> (Repo, vos_link::Outcome) {
    let r = Repo::with(label, REP);
    let text = r.read(IMPL);
    let head = format!("  claim {name}:\n");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("no claim {name}"));
    let body = start + head.len();
    // The body is the lines indented by four or more spaces.
    let end = text[body..]
        .split_inclusive('\n')
        .take_while(|l| l.starts_with("    "))
        .map(str::len)
        .sum::<usize>()
        + body;
    r.write(IMPL, format!("{}{}", &text[..start], &text[end..]));
    let g = r.gen(REP);
    (r, g)
}

// ---------------------------------------------------------------------------
// The positive fixture
// ---------------------------------------------------------------------------

#[test]
fn a_representation_crate_passes_gen_doc_and_check_doc() {
    let r = Repo::with("rep-ok", REP);
    let g = r.gen(REP);
    assert!(g.findings.is_empty(), "{}", show(&g));
    // Safety only: the representation claims are impl-own, not obligations (R4).
    assert_eq!(carried(&g), ["safety"]);
    let c = r.check(REP);
    assert!(c.findings.is_empty(), "{}", show(&c));
    assert_eq!(carried(&c), ["safety"]);
    assert_eq!(
        c.header,
        "crate vos-fixture-rep, kind production, mode representation, contract vos-sched-contract, obligations [safety]"
    );
}

#[test]
fn the_report_lists_every_representation_claim_with_its_claim_sha256() {
    let r = Repo::with("rep-claims", REP);
    let c = {
        r.gen(REP);
        r.check(REP)
    };
    let names: Vec<&str> = c.rep_claims.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "rep_init",
            "rep_alpha",
            "insert_rep",
            "remove_rep",
            "dispatch_rep",
            "dispatch_det"
        ]
    );
    for k in &c.rep_claims {
        assert!(vos_manifest::digest::is_hex64(&k.sha256), "{k:?}");
    }
    // Editing a claim changes its digest, which is what a lock pin will catch.
    let before = c.rep_claims[5].sha256.clone();
    r.edit(
        IMPL,
        "rr_pick(mask, bit_and(h, 7u8)) and",
        "rr_pick(mask, bit_and(h, 6u8)) and",
    );
    let g = r.gen(REP);
    assert!(g.findings.is_empty(), "{}", show(&g));
    assert_ne!(g.rep_claims[5].sha256, before);
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
        "run VOS_LINK_BLESS=1 cargo test to accept an intended change"
    );
}

#[test]
fn only_the_representation_lint_note_is_printed_for_a_representation_crate() {
    let r = Repo::with("rep-notes", REP);
    let g = r.gen(REP);
    assert_eq!(g.notes.len(), 1, "{:?}", g.notes);
    assert!(g.notes[0].starts_with("CD07-R:"), "{:?}", g.notes);
    assert!(g.notes[0].contains("mislabelled oracle"));
    assert!(g.notes[0].contains("VOS-D060") && g.notes[0].contains("VOS-D030"));
}

#[test]
fn an_explicit_canonical_mode_is_the_default_and_prints_no_mode() {
    let r = Repo::with("rep-explicit", "vos-fixture-bitmap");
    r.write(
        "code/packages/rust/vos-fixture-bitmap/vos.toml",
        "kind = \"production\"\nmode = \"canonical\"\ncontract = \"vos-sched-contract\"\nobligations = [\"canonical\", \"policy\", \"safety\"]\n",
    );
    let g = r.gen("vos-fixture-bitmap");
    assert!(g.findings.is_empty(), "{}", show(&g));
    assert!(!g.header.contains("mode"), "{}", g.header);
    assert!(g.rep_claims.is_empty());
}

// ---------------------------------------------------------------------------
// CD07-R: each negative control, with its specific finding
// ---------------------------------------------------------------------------

#[test]
fn a_representation_document_that_carries_canonical_is_rejected() {
    // `obligations` lists only safety, so the canonical block is not embedded; the claims
    // arrive through impl.part (a claim moved between blocks is still carried, NC17).
    let r = Repo::with("rep-canon", REP);
    let canon = r.read("code/packages/rust/vos-sched-contract/warrant/obligations-canonical.part");
    r.write(IMPL, format!("{}{canon}", r.read(IMPL)));
    let g = r.gen(REP);
    assert!(
        has(&g, "CD07", "carries a `canonical` obligation"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_representation_document_that_carries_an_any_obligation_is_rejected() {
    let r = Repo::with("rep-any-block", REP);
    let any = r.read("code/packages/rust/vos-sched-contract/warrant/obligations-any.part");
    r.write(IMPL, format!("{}{any}", r.read(IMPL)));
    let g = r.gen(REP);
    // The `any` claims name an oracle field `Impl` lacks, so the Warrant checker may
    // refuse first; either way the document does not pass.
    assert!(
        has(&g, "CD07", "carries an `any` obligation") || codes(&g).contains(&"CD04"),
        "{}",
        show(&g)
    );
    assert!(!g.findings.is_empty());
}

#[test]
fn a_crate_named_any_may_not_declare_a_mode() {
    let r = Repo::new("rep-named-any");
    common::copy_dir(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/impls")
            .join(REP),
        &r.pkg("vos-fixture-rep-any"),
    );
    let g = r.gen("vos-fixture-rep-any");
    assert!(has(&g, "CD07", "a crate named `-any`"), "{}", show(&g));
}

#[test]
fn a_rep_with_a_constant_field_is_rejected() {
    let (_r, g) = mutated(
        "rep-const",
        "Impl(mask, bit_and(h, 7u8))",
        "Impl(mask, 0u8)",
    );
    assert!(
        has(
            &g,
            "CD07",
            "field(s) 1 of `Impl` do not depend on any parameter of `rep`"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn a_rep_that_ignores_an_argument_by_cancelling_it_is_rejected() {
    let (_r, g) = mutated(
        "rep-cancel",
        "Impl(mask, bit_and(h, 7u8))",
        "Impl(mask, h - h)",
    );
    assert!(
        has(&g, "CD07", "field(s) 1 of `Impl` do not depend"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_missing_rep_model_is_rejected() {
    let (_r, g) = mutated(
        "rep-no-rep",
        "model rep(mask: U8, h: U8) -> Impl = Impl(mask, bit_and(h, 7u8))",
        "model rep_unused(mask: U8, h: U8) -> Impl = Impl(mask, bit_and(h, 7u8))",
    );
    // Every claim also fails to type-check (`rep` is unbound): the checker refuses first.
    assert!(codes(&g).contains(&"CD04"), "{}", show(&g));
}

#[test]
fn a_rep_that_is_not_a_constructor_application_is_rejected() {
    let (_r, g) = mutated(
        "rep-helper",
        "model rep(mask: U8, h: U8) -> Impl = Impl(mask, bit_and(h, 7u8))",
        "model rep_raw(mask: U8, h: U8) -> Impl = Impl(mask, bit_and(h, 7u8))\n  model rep(mask: U8, h: U8) -> Impl = rep_raw(mask, h)",
    );
    assert!(
        has(
            &g,
            "CD07",
            "`rep` is not an `Impl(..)` constructor application"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn a_missing_op_rep_claim_is_rejected_for_each_op() {
    for op in ["insert", "remove", "dispatch"] {
        let (_r, g) = without_claim(&format!("rep-no-{op}"), &format!("{op}_rep"));
        assert!(
            has(&g, "CD07", &format!("no claim `{op}_rep`")),
            "{op}: {}",
            show(&g)
        );
    }
}

#[test]
fn a_missing_rep_init_or_rep_alpha_claim_is_rejected() {
    for name in ["rep_init", "rep_alpha"] {
        let (_r, g) = without_claim(&format!("rep-no-{name}"), name);
        assert!(
            has(&g, "CD07", &format!("no claim `{name}`")),
            "{name}: {}",
            show(&g)
        );
    }
}

#[test]
fn an_op_applied_to_build_inside_a_representation_claim_is_rejected() {
    let (_r, g) = mutated(
        "rep-build",
        "insert_out(rep(mask, h), arg_t(h)) ==",
        "insert_out(build(dec(alpha_w(rep(mask, h)))), arg_t(h)) ==",
    );
    assert!(
        has(&g, "CD07", "claim `insert_rep` mentions `build`"),
        "{}",
        show(&g)
    );
    assert!(
        has(
            &g,
            "CD07",
            "applies the operation `insert_out` to a state that is not `rep(..)`"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn an_op_applied_to_a_variable_state_is_rejected() {
    // The quantified binder `s` is not `rep(..)`: a claim about "any state" is not a claim
    // about the representation.  (An image of rep is fine; a bare variable is not.)
    let (_r, g) = mutated(
        "rep-var",
        "claim rep_alpha:\n    forall mask: U8, h: U8. alpha_w(rep(mask, h)) == AbsW(mask)",
        "claim rep_alpha:\n    forall mask: U8, h: U8. alpha_w(Impl(mask, h)) == AbsW(mask)",
    );
    assert!(
        has(
            &g,
            "CD07",
            "claim `rep_alpha` applies the operation `alpha_w` to a state that is not `rep(..)`"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn an_image_of_rep_is_an_accepted_state_argument() {
    // The positive fixture already nests `alpha_w(insert_img(rep(..), t))`; this pins the
    // reason: only a call to an `_img` operation (or `rep`) is accepted as the state.
    let (_r, g) = mutated(
        "rep-img-ok",
        "dec(alpha_w(insert_img(rep(mask, h), arg_t(h)))) ==",
        "dec(alpha_w(remove_img(insert_img(rep(mask, h), arg_t(h)), arg_t(h)))) ==",
    );
    assert!(g.findings.is_empty(), "{}", show(&g));
}

#[test]
fn a_det_claim_whose_right_hand_side_calls_an_op_is_rejected() {
    let (_r, g) = mutated(
        "rep-det-rhs",
        "dispatch_out(rep(mask, h)) == rr_pick(mask, bit_and(h, 7u8)) and",
        "dispatch_out(rep(mask, h)) == dispatch_out(rep(mask, h)) and",
    );
    assert!(
        has(&g, "CD07", "determinism claim `dispatch_det` calls the operation `dispatch_out` on the right of `==`"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_det_claim_that_calls_an_op_deep_in_its_right_hand_side_is_rejected() {
    // The op is nested under a model call and a projection: the walk is not a prefix test.
    let (_r, g) = mutated(
        "rep-det-deep",
        "dispatch_img(rep(mask, h)) == rr_next(mask, bit_and(h, 7u8)))",
        "dispatch_img(rep(mask, h)) == rr_next(mask, bit_and(h, 7u8)) and\n      dispatch_out(rep(mask, h)) == (if has_bit(mask, 0u8) then dispatch_out(rep(mask, h)) else Idle))",
    );
    assert!(
        has(
            &g,
            "CD07",
            "calls the operation `dispatch_out` on the right of `==`"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn a_representation_crate_without_a_det_claim_is_rejected() {
    let (_r, g) = without_claim("rep-no-det", "dispatch_det");
    assert!(has(&g, "CD07", "no `<op>_det` claim"), "{}", show(&g));
}

#[test]
fn a_rep_claim_that_is_not_det_does_not_count_as_a_det_claim() {
    let (_r, g) = mutated(
        "rep-rename-det",
        "claim dispatch_det:",
        "claim dispatch_pin:",
    );
    assert!(has(&g, "CD07", "no `<op>_det` claim"), "{}", show(&g));
}

#[test]
fn an_empty_off_cone_set_says_to_declare_canonical() {
    // `build` sets the pointer from the abstract state too: nothing is history.
    let (_r, g) = mutated(
        "rep-empty-offcone",
        "b2m(m.p7, 7)), 0u8))",
        "b2m(m.p7, 7)), b2m(m.p0, 0)))",
    );
    assert!(has(&g, "CD07", "declare canonical"), "{}", show(&g));
    assert!(
        has(&g, "CD07", "representation mode is for history only"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_build_that_cannot_be_decided_fails_closed_in_representation_mode() {
    // `build` becomes a bare call of a helper: not an `Impl(..)` application, so the off-cone
    // set cannot be found structurally and the rule fails closed.
    let r = Repo::with("rep-build-notctor", REP);
    let text = r.read(IMPL);
    let start = text.find("  model build(m: Ready) -> Impl = (").unwrap();
    let end = text[start..].find("  model inv_impl").unwrap() + start;
    let body = text[start..end].replacen("build(", "build_core(", 1);
    r.write(
        IMPL,
        format!(
            "{}{}  model build(m: Ready) -> Impl = build_core(m)\n{}",
            &text[..start],
            body,
            &text[end..]
        ),
    );
    let g = r.gen(REP);
    assert!(
        has(
            &g,
            "CD07",
            "`build` model is not an `Impl(..)` constructor application"
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn the_cone_walks_fail_closed_when_they_run_out_of_budget() {
    let r = Repo::with("rep-budget", REP);
    r.gen(REP);
    let loaded = match vos_link::inputs::load(&r.pkg(REP), None).unwrap() {
        vos_link::inputs::Load::Ready(l) => *l,
        vos_link::inputs::Load::Findings(f) => panic!("{f:?}"),
    };
    let doc = std::fs::read(r.path(&format!("code/specs/{REP}.warrant"))).unwrap();
    let v = vos_link::check::verify_with_budget(&loaded, &doc, 10).unwrap();
    assert!(
        v.findings
            .iter()
            .any(|f| f.code == "CD07" && f.message.contains("`build` cone walk exhausted")),
        "{:?}",
        v.findings
    );
    assert!(
        v.findings
            .iter()
            .any(|f| f.code == "CD07" && f.message.contains("`rep` cone walk exhausted")),
        "{:?}",
        v.findings
    );
    // With the real budget the same document passes.
    let ok = vos_link::check::verify(&loaded, &doc).unwrap();
    assert!(ok.findings.is_empty(), "{:?}", ok.findings);
}

#[test]
fn a_reference_crate_may_declare_representation_and_then_owes_the_claims() {
    let r = Repo::with("rep-reference", "vos-fixture-ref");
    r.write(
        "code/packages/rust/vos-fixture-ref/vos.toml",
        "kind = \"reference\"\nmode = \"representation\"\ncontract = \"vos-sched-contract\"\nobligations = [\"safety\"]\n",
    );
    let g = r.gen("vos-fixture-ref");
    assert!(has(&g, "CD07", "no claim `rep_init`"), "{}", show(&g));
    assert!(
        g.header.contains("kind reference, mode representation"),
        "{}",
        g.header
    );
}

// ---------------------------------------------------------------------------
// GD05: the vos.toml key
// ---------------------------------------------------------------------------

#[test]
fn mode_on_a_kind_any_crate_is_rejected() {
    let r = Repo::with("rep-mode-any", "vos-fixture-any");
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        "kind = \"any\"\nmode = \"representation\"\ncontract = \"vos-sched-contract\"\nobligations = [\"any\", \"safety\"]\n",
    );
    let g = r.gen("vos-fixture-any");
    assert!(has(&g, "GD05", "`mode` on kind = \"any\""), "{}", show(&g));
    // Even the default spelling is refused: an `any` crate has no mode at all.
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        "kind = \"any\"\nmode = \"canonical\"\ncontract = \"vos-sched-contract\"\nobligations = [\"any\", \"safety\"]\n",
    );
    let g = r.gen("vos-fixture-any");
    assert!(has(&g, "GD05", "`mode` on kind = \"any\""), "{}", show(&g));
    assert!(!r.path("code/specs/vos-fixture-any.warrant").exists());
}

#[test]
fn mode_on_a_contract_package_is_rejected() {
    let r = Repo::with("rep-mode-contract", REP);
    r.edit(
        "code/packages/rust/vos-sched-contract/vos.toml",
        "kind = \"contract\"",
        "kind = \"contract\"\nmode = \"representation\"",
    );
    let g = r.gen(REP);
    assert!(has(&g, "GD05", "a contract has none"), "{}", show(&g));
}

#[test]
fn canonical_in_obligations_under_representation_is_rejected() {
    let r = Repo::with("rep-oblig-canon", REP);
    r.edit(
        TOML,
        "obligations = [\"safety\"]",
        "obligations = [\"canonical\", \"safety\"]",
    );
    let g = r.gen(REP);
    assert!(
        has(
            &g,
            "GD05",
            "obligations contains `canonical` under mode = \"representation\""
        ),
        "{}",
        show(&g)
    );
}

#[test]
fn an_unknown_mode_value_is_rejected() {
    for bad in ["sideways", "Representation", "none"] {
        let r = Repo::with("rep-bad-mode", REP);
        r.edit(
            TOML,
            "mode = \"representation\"",
            &format!("mode = \"{bad}\""),
        );
        let g = r.gen(REP);
        assert!(
            has(&g, "GD05", &format!("mode \"{bad}\" is not a mode")),
            "{bad}: {}",
            show(&g)
        );
    }
}

#[test]
fn mode_with_the_wrong_type_or_twice_is_a_vos_toml_error() {
    let r = Repo::with("rep-mode-type", REP);
    r.edit(TOML, "mode = \"representation\"", "mode = 3");
    let g = r.gen(REP);
    assert!(has(&g, "GD05", "wrong type"), "{}", show(&g));
    r.edit(
        TOML,
        "mode = 3",
        "mode = \"representation\"\nmode = \"representation\"",
    );
    let g = r.gen(REP);
    assert!(has(&g, "GD05", "duplicate key"), "{}", show(&g));
}

// ---------------------------------------------------------------------------
// check-doc, with the document on disk
// ---------------------------------------------------------------------------

#[test]
fn check_doc_catches_a_claim_edited_in_the_generated_document() {
    // A hand edit that weakens `dispatch_det` in the document is CD08 (regeneration) and the
    // lint sees the edited text too.
    let r = Repo::with("rep-doc-edit", REP);
    r.gen(REP);
    r.edit(
        &format!("code/specs/{REP}.warrant"),
        "dispatch_out(rep(mask, h)) == rr_pick(mask, bit_and(h, 7u8)) and",
        "dispatch_out(rep(mask, h)) == dispatch_out(rep(mask, h)) and",
    );
    let c = r.check(REP);
    assert!(
        has(&c, "CD07", "determinism claim `dispatch_det` calls"),
        "{}",
        show(&c)
    );
    assert!(codes(&c).contains(&"CD08"), "{}", show(&c));
}
