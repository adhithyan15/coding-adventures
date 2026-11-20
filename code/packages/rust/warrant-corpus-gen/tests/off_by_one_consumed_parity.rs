//! The S10o-2 acceptance test: the generator reproduces, byte for byte, the oracle's committed proof
//! of the fault `off-by-one-consumed` (VER04r 8.2, 16.10 [D36]).
//!
//! `python/warrant-oracle` committed, in slice S10o-1, a synthetic document, its `.profile` and two
//! dumps in `tests/fixtures/off-by-one-consumed-v1/`:
//!
//! | dump | schema | what this test reproduces it from |
//! |---|---|---|
//! | `.oracle-faults.json` | `warrant-fault-parity-v1` | the fault rows, and the controls under the explicit closed gate (`GATE_CLOSED`) and the open gate (`GATE_OPEN`) |
//! | `.oracle-refusals.json` | `warrant-faults-refusals-v1` | each refused signature: the generator code `fault` and the oracle's reason |
//!
//! The files are read from the oracle's directory (no copy).  A last test checks the wasm-leb128
//! `Result<(U64,Nat),E>` template against the compact JSON of the spec.

mod support;

use coding_adventures_warrant_corpus_gen::error::code;
use coding_adventures_warrant_corpus_gen::faults::instantiate;
use coding_adventures_warrant_corpus_gen::json::Json;
use coding_adventures_warrant_corpus_gen::lower::lower_document;
use support::minijson::parse;
use support::parity::*;
use support::*;

const WARRANT: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.warrant"
);
const PROFILE: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.profile"
);
const FAULTS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.oracle-faults.json"
);
const REFUSALS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.oracle-refusals.json"
);
const SPEC_TEXT: &str = include_str!("../../../../specs/VER04r-warrant-corpus-v2.md");

/// The specification's path as written into `corpus.json`.
const SPEC: &str = "off_by_one_consumed_v1.warrant";

/// `warrant-fault-parity-v1`: the fault rows, the controls under the closed gate and under the open
/// gate (the oracle's `parity_dumps.faults_view`, the gates named explicitly).
fn faults_view() -> Json {
    let closed = generated(WARRANT, SPEC, PROFILE, &GATE_CLOSED).expect("generates (closed gate)");
    let open = generated(WARRANT, SPEC, PROFILE, &GATE_OPEN).expect("generates (open gate)");
    let s = parse(&closed.corpus_json).expect("parses");
    let g = parse(&open.corpus_json).expect("parses");
    assert_eq!(s.get("faults"), g.get("faults"), "the fault rows never depend on the gate");
    Json::obj([
        ("schema", Json::str("warrant-fault-parity-v1")),
        ("module", to_json(s.get("module"))),
        ("faults", to_json(s.get("faults"))),
        ("controls_shipped", to_json(s.get("controls"))),
        ("controls_gate_on", to_json(g.get("controls"))),
        ("accepted_blind_faults", to_json(s.get("accepted_blind_faults"))),
        ("levels_shipped", to_json(s.get("levels"))),
        ("levels_gate_on", to_json(g.get("levels"))),
        ("gate_shipped", Json::strs(GATE_CLOSED.iter().map(|l| l.name()))),
        ("gate_on", Json::strs(GATE_OPEN.iter().map(|l| l.name()))),
    ])
}

#[test]
fn the_fault_rows_and_controls_match_the_oracle_fault_dump_byte_for_byte() {
    let text = faults_view().render().expect("render");
    assert!(text == FAULTS, "{}", first_difference(&text, FAULTS));
}

#[test]
fn the_dump_is_not_vacuous() {
    assert!(FAULTS.contains("\"warrant-fault-parity-v1\""));
    assert!(FAULTS.matches("\"fault\": \"off-by-one-consumed\"").count() >= 8);
    // the blind control on `dec_u64`'s `Err` claim and both gates
    assert!(FAULTS.contains("\"controls_shipped\"") && FAULTS.contains("\"controls_gate_on\""));
}

/// The contract boilerplate for the operation `f` of a refusal document, so the S2 checker reaches the
/// lowering: `f` has one parameter, named `param`.
fn contracts(param: &str) -> String {
    format!(
        "  assume platform:\n    none\n  assume environment:\n    none\n  labels f:\n    {param} secret\n  input f.{param}:\n    hostile\n    ensures no_panic, terminates\n  bound f:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n"
    )
}

/// The operation `f` of a refusal document (one parameter), lowered, or the lowering's refusal.
fn lowered_op(
    source: &str,
) -> Result<
    coding_adventures_warrant_corpus_gen::lower::LoweredOp,
    coding_adventures_warrant_corpus_gen::error::GenError,
> {
    let param = source
        .split("op f(")
        .nth(1)
        .and_then(|t| t.split(':').next())
        .expect("a one-parameter operation f");
    let src = format!("{source}{}", contracts(param));
    with_file(&src, |f| lower_document(f).map(|l| l.op("f").expect("op f").clone()))
}

fn refusal_op(source: &str) -> coding_adventures_warrant_corpus_gen::lower::LoweredOp {
    lowered_op(source).expect("lowers")
}

/// The reason after the oracle's `fault: ` prefix.
fn why(message: &str) -> &str {
    message.strip_prefix("fault: ").expect("the oracle's message starts `fault: `")
}

#[test]
fn every_refused_signature_has_the_generator_code_fault_and_the_oracles_reason() {
    let dump = parse(REFUSALS).expect("the refusals dump parses");
    assert_eq!(dump.get("schema").s(), "warrant-faults-refusals-v1");
    assert_eq!(dump.get("fault").s(), "off-by-one-consumed");
    let rows = dump.get("refusals").a();
    assert_eq!(rows.len(), 11);
    for r in rows {
        let name = r.get("name").s();
        assert_eq!(r.get("oracle_code").s(), "generation", "{name}");
        // S11b: an `Option` signature lowers (`ir.data`, [D48]), so every row, `an-option-result`
        // included, reaches the fault and is refused with the `fault` code of section 8.2 and the
        // oracle's reason (before S11b the generator's lowering stopped that one row first).
        let op = lowered_op(r.get("source").s()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let e = instantiate(&op, "off-by-one-consumed").expect_err(name);
        assert_eq!(e.code, code::FAULT, "{name}: {e}");
        assert_eq!(
            e.message,
            format!(
                "the fault cannot be instantiated on this operation: {}",
                why(r.get("message").s())
            ),
            "{name}: generator vs oracle `{}`",
            r.get("message").s()
        );
    }
}

#[test]
fn the_ok_payload_clash_is_refused_only_in_the_ok_form() {
    let bare = refusal_op("warrant r v1\n  error E = A | B\n  op f(ok_payload: U8) -> (U8,Nat)\n");
    assert!(instantiate(&bare, "off-by-one-consumed").is_ok());
    let ok = refusal_op("warrant r v1\n  error E = A | B\n  op f(ok_payload: U8) -> Result<(U8,Nat),E>\n");
    assert_eq!(instantiate(&ok, "off-by-one-consumed").unwrap_err().code, code::FAULT);
    // Refusal order: a non-numeric second component is reported before the `ok_payload` clash, so
    // the message is the numeric one, not the clash one.
    let both =
        refusal_op("warrant r v1\n  error E = A | B\n  op f(ok_payload: U8) -> Result<(U8,Bool),E>\n");
    let e = instantiate(&both, "off-by-one-consumed").unwrap_err();
    assert_eq!(e.code, code::FAULT, "{e}");
    assert_eq!(
        e.message,
        "the fault cannot be instantiated on this operation: \
         off-by-one-consumed needs a numeric second component"
    );
}

#[test]
fn the_wasm_leb128_template_is_the_compact_json_of_the_spec() {
    let marker = "**`off-by-one-consumed`: exact form";
    let tail = &SPEC_TEXT[SPEC_TEXT.find(marker).expect("the spec has the section")..];
    let block = &tail[tail.find("```json\n").expect("the compact JSON block") + 8..];
    let expected = block[..block.find("\n```").expect("the block ends")].trim();
    let op = refusal_op("warrant r v1\n  error E = A | B\n  op f(a: U8) -> Result<(U64,Nat),E>\n");
    let f = instantiate(&op, "off-by-one-consumed").expect("instantiates");
    let got: String = f.to_json().render().expect("render").split_whitespace().collect();
    assert!(got.contains(expected), "template `{got}` vs spec `{expected}`");
    assert_eq!(f.requires.len(), 2);
}
