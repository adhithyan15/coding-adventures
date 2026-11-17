//! The S3b acceptance test: the generator's enumeration and the independent oracle agree.
//!
//! `python/warrant-oracle` enumerates `.warrant` documents with its own evaluator, its own
//! domains and its own SHA-256 streams (VER04r section 10, decision [D3]; it shares no code
//! with this crate).  `tests/fixtures/dump_oracle_enumeration.py` prints what the oracle
//! computes, at `full` depth, as canonical JSON, and this test renders the same *parity
//! view* from this crate's [`enumerate_document`] and demands the two be **byte-identical**:
//! per suite the counts (`cases`, `discarded`, `expected_calls`, `wraps`), the descriptors
//! and label, the v1 and v2 digests, the slim set, and **every chunk line** (so the chunk
//! files S3c writes are the oracle's too).
//!
//! Documents: ct-compare at the default chunk size (all nine suites, 572,109 cases), and two
//! synthetic documents at `chunk_size 1024` (64 chunks per 65,536-ordinal suite, so the slim
//! stride is exercised) that between them cover wrapping `U8` arithmetic in `admit`, in
//! `expected` and in subject arguments, refinements on a prefix and on the last binder, a
//! pure guard, four calls per case, `Result`, `Nat`, `I8`, `U16`, `U32` and `U64` binders.
//!
//! The negative controls at the bottom prove the comparison can fail.

mod support;

use coding_adventures_warrant_corpus_gen::enumerate::{enumeration_view, Enumeration};
use coding_adventures_warrant_corpus_gen::enumerate_document;
use coding_adventures_warrant_corpus_gen::profile::{parse_profile, Profile};
use support::*;

const CT_FIXTURE: &str = include_str!("fixtures/ct-compare.oracle-enumeration.json");
const V1_FIXTURE: &str = include_str!("fixtures/enum_forms_v1.oracle-enumeration.json");
const V0_FIXTURE: &str = include_str!("fixtures/enum_forms_v0.oracle-enumeration.json");
const PROBE_FIXTURE: &str = include_str!("fixtures/enum_probe.oracle-enumeration.json");

fn run(src: &str, profile: &Profile) -> (Enumeration, String) {
    let e = with_file(src, |f| enumerate_document(f, profile).expect("enumerates"));
    let text = enumeration_view(&e, profile)
        .expect("view")
        .render()
        .expect("render");
    (e, text)
}

fn small() -> Profile {
    parse_profile(b"chunk_size 1024\n").unwrap()
}

#[test]
fn ct_compare_matches_the_oracle_byte_for_byte() {
    let (e, text) = run(CT_COMPARE, &Profile::default());
    assert_eq!(e.cases(), 572_109);
    assert_eq!(text, CT_FIXTURE);
}

#[test]
fn the_v1_synthetic_document_matches_the_oracle_byte_for_byte() {
    let (_, text) = run(ENUM_V1, &small());
    assert_eq!(text, V1_FIXTURE);
}

#[test]
fn the_v0_synthetic_document_matches_the_oracle_byte_for_byte() {
    let (_, text) = run(ENUM_V0, &Profile::default());
    assert_eq!(text, V0_FIXTURE);
}

#[test]
fn the_probe_document_matches_the_oracle_byte_for_byte() {
    // 13 suites over concat/take/drop, the connectives, ordering comparisons, exact Nat/Int
    // arithmetic, U16/U32/U64 wraps, Bytes literals, a model call in `admit` and wrapping pure
    // guards under `or` and `=>`: the constructs the other documents leave out
    let (e, text) = run(ENUM_PROBE, &small());
    assert_eq!(e.suites.len(), 13);
    assert_eq!(text, PROBE_FIXTURE);
}

#[test]
fn the_fixtures_are_canonical_json_with_one_final_newline_and_no_cr() {
    for (name, f) in [
        ("ct", CT_FIXTURE),
        ("v1", V1_FIXTURE),
        ("v0", V0_FIXTURE),
        ("probe", PROBE_FIXTURE),
    ] {
        assert!(!f.contains('\r'), "{name}");
        assert!(f.ends_with("}\n") && !f.ends_with("\n\n"), "{name}");
        assert!(f.is_ascii(), "{name}");
        assert!(
            f.contains("\"schema\": \"warrant-enumeration-parity-v0\""),
            "{name}"
        );
    }
}

// ---------------------------------------------------------------------------
// Negative controls: a wrong generator must lose parity
// ---------------------------------------------------------------------------

#[test]
fn a_different_chunk_size_loses_parity() {
    // The synthetic document at the default chunk size has one chunk per suite, not 64.
    let (_, text) = run(ENUM_V1, &Profile::default());
    assert_ne!(text, V1_FIXTURE);
}

#[test]
fn a_corrupted_snapshot_would_be_caught() {
    // flip one hex digit of one chunk line in the oracle snapshot
    let at = V1_FIXTURE.find("chunk_lines").unwrap();
    let digit = V1_FIXTURE[at..]
        .char_indices()
        .skip(40)
        .find(|(_, c)| c.is_ascii_hexdigit() && *c != 'f')
        .map(|(i, _)| at + i)
        .unwrap();
    let mut bytes = V1_FIXTURE.as_bytes().to_vec();
    bytes[digit] = if bytes[digit] == b'0' { b'1' } else { b'0' };
    let tampered = String::from_utf8(bytes).unwrap();
    let (_, text) = run(ENUM_V1, &small());
    assert_ne!(text, tampered);
    assert_eq!(text, V1_FIXTURE);
}

#[test]
fn the_view_carries_every_chunk_line_and_the_nine_v1_digest_pairs() {
    let (e, text) = run(CT_COMPARE, &Profile::default());
    let lines: usize = e.suites.iter().map(|s| s.chunks.len()).sum();
    // 2 + 1 + 1 + 1 + 1 + 1 + 4 + 4 + 3 chunks over the nine suites
    assert_eq!(lines, 18);
    assert_eq!(text.matches("v1_inputs_sha256").count(), 9);
    assert_eq!(text.matches("v1_expected_sha256").count(), 9);
}
