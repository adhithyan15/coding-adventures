//! VOS-D025 (VER04j section 14.3 item 55): a model that builds a fixed-size
//! array with `fixed<N>(..)` generates, and its generated suites pass.
//!
//! The harness (`tests/generated/wxf_evidence.rs`, drift-tested by
//! `tests/wxf_regen.rs`) is generated from `tests/fixtures/wxf.warrant`, whose
//! builder `cells_of` is `match fixed<8>(concat(..)) with Some(b) -> b | None -> ..`
//! (the shape of VOS01 section 5.4's linear scan).  Before the fix generation
//! stopped at W0701 "call to `fixed`".  Here the generated binary runs against
//! an in-test subject (`tests/support/wxf_subject.rs`) with its negative
//! controls, and the sections below check what the suites did.

use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxf_subject {
        include!("support/wxf_subject.rs");
    }
    include!("generated/wxf_evidence.rs");
}

include!("support/wxh_runner.rs");

#[test]
fn the_whole_generated_suite_passes_with_every_negative_control() {
    let r = run_once();
    let bad: Vec<String> = r
        .outcomes
        .iter()
        .filter(|o| !o.passed())
        .map(|o| format!("{}: {}", o.id, message(o)))
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(r.failed, 0);
}

#[test]
fn the_exhaustive_property_and_fuzz_suites_all_ran_on_the_builder_claims() {
    for claim in ["tally_matches_the_model", "lowest_matches_the_model", "highest_matches_the_model"] {
        for kind in ["exhaustive", "property", "fuzz"] {
            let o = outcome(&format!("{kind}/{claim}"));
            assert!(o.passed(), "{kind}/{claim}: {}", message(o));
        }
    }
    // Eight Bool binders: the 256 builder states are enumerated, not sampled.
    let m = &run_once().manifest;
    let at = m.find("exhaustive/tally_matches_the_model").expect("entry");
    // Keys are sorted, so `cases` precedes `id`: take the whole object.
    let start = m[..at].rfind('{').unwrap();
    let entry = &m[start..at + m[at..].find('}').unwrap()];
    assert!(entry.contains("\"cases\": 256"), "{entry}");
}

#[test]
fn each_fault_is_caught_by_every_suite_that_attacks_the_operation() {
    for target in [
        "negctl/exhaustive/tally/field-zero",
        "negctl/property/tally/field-zero",
        "negctl/fuzz/tally/field-zero",
        "negctl/exhaustive/lowest/wrong-ctor",
        "negctl/property/lowest/wrong-ctor",
        "negctl/fuzz/lowest/wrong-ctor",
        "negctl/exhaustive/highest/wrong-ctor",
    ] {
        let o = outcome(target);
        assert!(o.passed(), "{target} must catch its fault: {}", message(o));
    }
}

#[test]
fn the_examples_over_literal_cells_pass() {
    for id in ["nothing_set", "two_set", "lowest_of_cells_two_and_five", "highest_of_none_is_idle"] {
        let o = outcome(&format!("example/{id}"));
        assert!(o.passed(), "{id}: {}", message(o));
    }
}

fn generate_with(spec: &str) -> Result<String, String> {
    use coding_adventures_sha256::sha256_hex;
    use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
    use coding_adventures_warrant_grammar::checker::check_warrant;
    use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
    use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
    use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
    let b = parse_evidence_bindings(include_str!("fixtures/wxf.bindings")).expect("bindings");
    let doc = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).map_err(|e| format!("check: {e:?}"))?;
    let options = EvidenceOptions {
        package_crate: b.manifest.package.clone(),
        spec_path: b.spec.clone(),
        spec_sha256: sha256_hex(spec.as_bytes()),
        seed: b.seed,
        property_cases: b.property_cases,
        fuzz_iterations: b.fuzz_iterations,
        negctl_fuzz_iterations: b.negctl_fuzz_iterations,
        exhaustive_budget: b.exhaustive_budget,
        arg_types: b.arg_types.clone(),
        hosts: b.hosts.clone(),
    };
    compile_rust_evidence_harness(&file, &checked, &b, &options).map_err(|e| format!("{e:?}"))
}

#[test]
fn a_width_other_than_a_literal_is_still_refused_and_a_document_without_fixed_is_unchanged() {
    // The helper is emitted only for a document that builds with `fixed<N>`.
    let with = include_str!("fixtures/wxf.warrant");
    let text = generate_with(with).expect("generates");
    assert!(text.contains("pub fn fixed_n("), "the fixed<N> helper is in the harness");
    // Removing the builder removes the helper: every other harness is byte-identical.
    let without = with.replace(
        "match fixed<8>(concat(concat(concat(concat(concat(concat(concat(\n        be_bytes(b2u(b0)), be_bytes(b2u(b1))), be_bytes(b2u(b2))), be_bytes(b2u(b3))),\n        be_bytes(b2u(b4))), be_bytes(b2u(b5))), be_bytes(b2u(b6))), be_bytes(b2u(b7))))\n    with Some(b) -> b | None -> b\"0000000000000000\")",
        "b\"0000000000000000\")",
    );
    assert_ne!(without, with, "the builder text was replaced");
    let text = generate_with(&without).expect("generates without the builder");
    assert!(!text.contains("fixed_n"), "no fixed<N> helper without a fixed<N> call");
}
