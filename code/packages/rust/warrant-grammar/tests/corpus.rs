//! Regeneration and bounded-profile checks for the portable Warrant corpus.

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::corpus::generate_corpus;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const SPEC: &str = include_str!("../../../../specs/ct-compare.warrant");
const CORPUS: &str = include_str!("../../../../specs/fixtures/ct-compare-warrant-v1/corpus.json");
const TFV: &str = include_str!("../../../../specs/fixtures/ct-compare-warrant-v1/cases.tfv");

fn compile(spec: &str) -> (String, String) {
    let admitted = SourceDocument::admit(spec.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&admitted, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &admitted, ParserLimits::default()).unwrap();
    check_warrant(&file).unwrap();
    generate_corpus(&file, spec.as_bytes()).unwrap()
}

#[test]
fn checked_in_corpus_and_examples_regenerate_byte_for_byte() {
    let generated = compile(SPEC);
    assert_eq!(
        generated.0, CORPUS,
        "run gen_corpus after changing the spec"
    );
    assert_eq!(generated.1, TFV, "run gen_corpus after changing the spec");
}

#[test]
fn supported_profile_pins_counts_digests_and_separates_the_oracle() {
    assert_eq!(CORPUS.matches("\"kind\":\"exhaustive\"").count(), 9);
    assert!(CORPUS.contains("\"cases\":120409"));
    assert!(CORPUS.contains("\"cases\":139818"));
    assert!(CORPUS.contains("\"domain_text\":\"Bytes[347] x Bytes[347]\""));
    assert!(CORPUS.contains("\"expected_calls\":120409"));
    assert!(CORPUS.contains(
        "\"inputs_sha256\":\"73b983e125ba39dc8c311f407423a8f40cfcf2738e65151cdf177e36c384bc5b\""
    ));
    assert!(CORPUS.contains(
        "\"expected_sha256\":\"a243a089a7ad008af8f29fca5504a92c042db32cca3e70953d91e749baf9d856\""
    ));
    assert!(CORPUS.contains("\"lhs\":[\"subject_op\",\"eq\""));
    assert!(CORPUS.contains("\"body\":[\"eq\",[\"var\",\"a\"],[\"var\",\"b\"]]"));
    assert!(!CORPUS.contains("\"body\":[\"subject_op\""));
    assert!(CORPUS.contains("\"eq_fixed\":{\"params\":[[\"a\",\"Bytes<N>\"],[\"b\",\"Bytes<N>\"]],\"ret\":\"Bool\",\"width_params\":[\"N\"]}"));
    assert!(CORPUS.contains("\"select_bytes\":{\"params\":[[\"a\",\"Bytes\"],[\"b\",\"Bytes\"],[\"choice\",\"Bool\"]],\"ret\":\"Result<Bytes,SelectError>\",\"fresh\":true}"));
}

#[test]
fn v1_and_unsupported_claim_shapes_fail_closed() {
    let v1 = SPEC.replacen("warrant ct_compare", "warrant ct_compare v1", 1);
    let admitted = SourceDocument::admit(v1.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&admitted, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &admitted, ParserLimits::default()).unwrap();
    assert!(generate_corpus(&file, v1.as_bytes())
        .unwrap_err()
        .0
        .contains("only warrant ct_compare v0"));

    let unsupported = SPEC.replace("eq(a, b) == same(a, b)", "eq(a, b) == eq(b, a)");
    let admitted = SourceDocument::admit(unsupported.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&admitted, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &admitted, ParserLimits::default()).unwrap();
    check_warrant(&file).unwrap();
    assert!(generate_corpus(&file, unsupported.as_bytes())
        .unwrap_err()
        .0
        .contains("exactly one subject side"));
}

#[test]
fn faulted_operations_without_claim_targets_fail_closed() {
    let start = SPEC.find("  claim eq_fixed_agrees_w0:").unwrap();
    let end = SPEC.find("  claim select_picks:").unwrap();
    let without_fixed_claims = format!("{}{}", &SPEC[..start], &SPEC[end..]);
    let admitted =
        SourceDocument::admit(without_fixed_claims.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&admitted, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &admitted, ParserLimits::default()).unwrap();
    check_warrant(&file).unwrap();
    assert!(generate_corpus(&file, without_fixed_claims.as_bytes())
        .unwrap_err()
        .0
        .contains("has no claim target"));
}
