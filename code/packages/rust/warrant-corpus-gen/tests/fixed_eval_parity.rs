//! The S10c-2 acceptance test: the Rust generator and the independent oracle agree, byte for byte,
//! on the **evaluation** of every `ir.fixed` built-in and on the domains of [D21], [D25] and [D26]
//! (VER04r section 13, slice S10c, R9 to R13).
//!
//! `python/warrant-oracle` committed, in slice S10c-1, a synthetic document that uses all 35
//! built-ins at every one of the 8 fixed-width types, every case of the shift-count rule
//! (`fixed_eval_v1.warrant`), its `.profile` (`budget 2^20`, so the 16-bit binders are demoted to the
//! 16-bit `fx_edge`), and five committed dumps of what the oracle computes from them:
//!
//! | dump | schema | what this test reproduces it from |
//! |---|---|---|
//! | `.oracle-lowering.json` | `warrant-lowering-parity-v1` | [`view::oracle_view_v1`] of the lowered document |
//! | `.oracle-evaluation.json` | `warrant-evaluation-parity-v1` | the generated corpus (`corpus.json`, chunk files, `cases.tfv`) |
//! | `.oracle-faults.json` | `warrant-fault-parity-v1` | the same corpus under the explicitly pinned closed gate (`ir.core`, `GATE_CLOSED`) and the test-only **open** gate (`ir.core`, `ir.fixed`, `ir.tuple`) |
//! | `.oracle-values.json` | `warrant-fixed-values-parity-v1` | the generator's evaluator on every built-in at every width on edge values |
//! | `.oracle-domains.json` | `warrant-domain-parity-v1` | the resolved leaf domains, and the consumer-side descriptor check |
//!
//! The files are read from the oracle's directory (no copy), so an edit to any of them reruns this
//! test (the build tool's repository-input row `rust/warrant-corpus-gen`).  The chain is: oracle
//! source -> committed dump == this crate's output.  The dumps carry no generator name or version,
//! so the version bump of the flip does not move them.
//!
//! **Test cost.**  Nothing here enumerates an adopter: the document is the synthetic one (112
//! suites, at most 388,129 cases each), and the values table is about 9,800 evaluations.  (The
//! oracle's lesson `a-test-that-emits-an-adopter-becomes-a-full-enumeration`: wasm-leb128 is 46
//! million cases.)
//!
//! The negative controls at the bottom show each comparison can fail.

mod support;

use coding_adventures_warrant_corpus_gen::domains::{descriptor_size, describe};
use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::eval::{Program, Stats};
use coding_adventures_warrant_corpus_gen::faults::Level;
use coding_adventures_warrant_corpus_gen::ir::Ir;
use coding_adventures_warrant_corpus_gen::json::Json;
use coding_adventures_warrant_corpus_gen::lower::lower_document;
use coding_adventures_warrant_corpus_gen::value::Value;
use coding_adventures_warrant_corpus_gen::view::oracle_view_v1;
use coding_adventures_warrant_grammar::codegen::EvalValue;
use coding_adventures_warrant_grammar::domain::Domain;
use coding_adventures_warrant_grammar::fixed::FixedTy;
use std::sync::OnceLock;

use coding_adventures_warrant_corpus_gen::CorpusV2;
use support::minijson::parse;
use support::parity::*;
use support::*;

const WARRANT: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.warrant"
);
const PROFILE: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.profile"
);
const LOWERING: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-lowering.json"
);
const EVALUATION: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-evaluation.json"
);
const FAULTS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-faults.json"
);
const VALUES: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-values.json"
);
const DOMAINS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-domains.json"
);

/// The specification's path as written into `corpus.json`.
const SPEC: &str = "fixed_eval_v1.warrant";

/// The bound the oracle's `dump-lowering` costs under by default (`MAX_IR_COST`).
const LIMIT: u64 = 2_000;

/// The test-only **open** gate of the oracle (`parity_dumps.GATE_OPEN`): every level evaluated.
const GATE_OPEN: [Level; 3] = [Level::Core, Level::Fixed, Level::Tuple];

// ---- the lowering dump ---------------------------------------------------------------------------

fn lowering_text(src: &str) -> String {
    with_file(src, |f| {
        let l = lower_document(f).expect("lowers");
        oracle_view_v1(&l, LIMIT)
            .expect("view")
            .render()
            .expect("render")
    })
}

#[test]
fn step_zero_the_grammar_checker_accepts_the_oracle_fixture() {
    // The generator's front end (`check_warrant`, run by every lowering) accepts every line of the
    // committed document: the fixture needs no regeneration.
    with_file(WARRANT, |f| {
        lower_document(f).expect("check_warrant and the lowering accept the fixture");
    });
}

#[test]
fn the_fixed_eval_document_lowers_to_the_oracle_dump_byte_for_byte() {
    let text = lowering_text(WARRANT);
    assert!(text == LOWERING, "{}", first_difference(&text, LOWERING));
}

// ---- the evaluation and fault dumps ----------------------------------------------------------

fn evaluation_text(src: &str, profile: &str) -> String {
    evaluation_view(&generated(src, SPEC, profile, &GATE_CLOSED).expect("generates"))
        .render()
        .expect("render")
}

/// The committed document generated once under the shipped gate and once under the open gate: the
/// tests below share them (a generation enumerates 112 suites, so each is paid for once).
fn shipped() -> &'static CorpusV2 {
    static C: OnceLock<CorpusV2> = OnceLock::new();
    C.get_or_init(|| generated(WARRANT, SPEC, PROFILE, &GATE_CLOSED).expect("generates (closed gate)"))
}

fn open() -> &'static CorpusV2 {
    static C: OnceLock<CorpusV2> = OnceLock::new();
    C.get_or_init(|| {
        generated(WARRANT, SPEC, PROFILE, &GATE_OPEN).expect("generates (open gate)")
    })
}

#[test]
fn the_fixed_eval_corpus_matches_the_oracle_evaluation_dump_byte_for_byte() {
    let text = evaluation_view(shipped()).render().expect("render");
    assert!(text == EVALUATION, "{}", first_difference(&text, EVALUATION));
}

/// `warrant-fault-parity-v1` of the `"fixed"` fixture: the fault rows and the controls under the
/// shipped gate and under the open gate (the oracle's `parity_dumps.faults_view(.., "fixed")`).
fn faults_view(shipped: &CorpusV2, open: &CorpusV2) -> Json {
    let s = parse(&shipped.corpus_json).expect("parses");
    let g = parse(&open.corpus_json).expect("parses");
    assert_eq!(
        s.get("faults"),
        g.get("faults"),
        "the fault rows never depend on the gate"
    );
    Json::obj([
        ("schema", Json::str("warrant-fault-parity-v1")),
        ("module", to_json(s.get("module"))),
        ("faults", to_json(s.get("faults"))),
        ("controls_shipped", to_json(s.get("controls"))),
        ("controls_gate_on", to_json(g.get("controls"))),
        (
            "accepted_blind_faults",
            to_json(s.get("accepted_blind_faults")),
        ),
        ("levels_shipped", to_json(s.get("levels"))),
        ("levels_gate_on", to_json(g.get("levels"))),
        ("gate_shipped", Json::strs(["ir.core"])),
        ("gate_on", Json::strs(GATE_OPEN.iter().map(|l| l.name()))),
    ])
}

#[test]
fn the_fault_rows_and_controls_match_the_oracle_fault_dump_byte_for_byte() {
    let text = faults_view(shipped(), open()).render().expect("render");
    assert!(text == FAULTS, "{}", first_difference(&text, FAULTS));
}

// ---- the values dump ---------------------------------------------------------------------------

const TYPES: [FixedTy; 8] = [
    FixedTy::U8,
    FixedTy::U16,
    FixedTy::U32,
    FixedTy::U64,
    FixedTy::I8,
    FixedTy::I16,
    FixedTy::I32,
    FixedTy::I64,
];

/// The edge values of a type: its extremes and their neighbours, 0, 1, the middle and a bit pattern.
fn sample(t: FixedTy) -> Vec<i128> {
    let (lo, hi) = (t.min(), t.max());
    let pattern = (0..t.bytes()).fold(0u128, |p, _| (p << 8) | 0xa5);
    let mut values = vec![lo, hi, hi - 1, 0, 1, hi >> 1, t.canon(pattern)];
    if t.signed() {
        values.extend([lo + 1, -1]);
    } else {
        values.push(2);
    }
    values.sort_unstable();
    values.dedup();
    values
}

/// The shift-count rows of VER04j 3.8: `0 1 w-1 w w+1 2w 255 65535 65536`.
fn counts(w: u64) -> Vec<u64> {
    let mut v = vec![0, 1, w - 1, w, w + 1, 2 * w, 255, 65535, 65536];
    v.sort_unstable();
    v.dedup();
    v
}

fn show(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Bytes(b) => format!("x{}", hex(b)),
        Value::Opt(None) => "none".into(),
        Value::Opt(Some(inner)) => format!("some {}", show(inner)),
        other => panic!("a value outside the row grammar: {other:?}"),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn var(n: &str) -> Ir {
    Ir::Var(n.to_string())
}

/// An `ir.fixed` node as the lowering builds it: `ty` is the static type argument, `at` the width
/// of a built-in that has none.  `rename` is the negative control: a wrong evaluator reads
/// `from` as `to`.
fn node(
    tag: &'static str,
    ty: Option<FixedTy>,
    at: Option<FixedTy>,
    args: Vec<Ir>,
    rename: Option<(&str, &'static str)>,
) -> Ir {
    let tag = match rename {
        Some((from, to)) if from == tag => to,
        _ => tag,
    };
    Ir::Fixed { tag, ty, at, args }
}

struct Rows<'a> {
    program: Program<'a>,
    out: Vec<String>,
}

impl Rows<'_> {
    /// One evaluation: `label operands = result`, operands in the order of `vars`.
    fn row(&mut self, label: &str, ir: &Ir, vars: &[(&str, Value)]) {
        let value = self
            .program
            .eval(ir, vars, &mut Stats::default())
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let shown: Vec<String> = vars.iter().map(|(_, v)| show(v)).collect();
        self.out.push(if shown.is_empty() {
            format!("{label} = {}", show(&value))
        } else {
            format!("{label} {} = {}", shown.join(" "), show(&value))
        });
    }
}

/// Every row of the values table, in the oracle's fixed order (type, then built-in, then operands),
/// computed by this crate's evaluator over `Ir::Fixed` trees.
fn value_rows(rename: Option<(&str, &'static str)>) -> Vec<String> {
    let mut rows = Rows {
        program: Program::new(&[]),
        out: Vec::new(),
    };
    let n = |tag, ty, at, args| node(tag, ty, at, args, rename);
    let int = |x: i128| Value::Int(x);
    for t in TYPES {
        let name = t.name();
        let (lo, hi) = (t.min(), t.max());
        let values = sample(t);
        rows.out.push(format!("max_value {name} = {hi}"));
        rows.out.push(format!("min_value {name} = {lo}"));
        let mut binary = vec![
            "bit_and",
            "bit_or",
            "bit_xor",
            "checked_add",
            "checked_sub",
            "checked_mul",
        ];
        binary.extend(if t.signed() {
            ["div_trunc", "rem_trunc"]
        } else {
            ["div", "rem"]
        });
        for tag in binary {
            for &a in &values {
                for &b in &values {
                    let ir = n(tag, None, Some(t), vec![var("a"), var("b")]);
                    rows.row(&format!("{tag} {name}"), &ir, &[("a", int(a)), ("b", int(b))]);
                }
            }
        }
        let mut unary = vec![
            "bit_not", "popcount", "clz", "ctz", "be_bytes", "le_bytes", "to_int",
        ];
        if !t.signed() {
            unary.push("to_nat");
        }
        for tag in unary {
            for &a in &values {
                // to_int / to_nat are width-free; the rest work at `t`
                let at = (tag != "to_int" && tag != "to_nat").then_some(t);
                let ir = n(tag, None, at, vec![var("a")]);
                rows.row(&format!("{tag} {name}"), &ir, &[("a", int(a))]);
            }
        }
        let mut shifts = vec!["shl", "shr", "rotl", "rotr", "bit"];
        if t.signed() {
            shifts.push("sar");
        }
        for tag in shifts {
            for &a in &values {
                for c in counts(u64::from(t.bits())) {
                    let ir = n(tag, None, Some(t), vec![var("a"), var("n")]);
                    rows.row(
                        &format!("{tag} {name}"),
                        &ir,
                        &[("a", int(a)), ("n", int(i128::from(c)))],
                    );
                }
            }
        }
        for target in TYPES {
            let inside = target.min() <= lo && hi <= target.max();
            let mut tags = vec!["wrap", "try"];
            if inside {
                tags.push("widen");
            }
            for tag in tags {
                for &a in &values {
                    let ir = n(tag, Some(target), None, vec![var("a")]);
                    rows.row(
                        &format!("{tag} {name}->{}", target.name()),
                        &ir,
                        &[("a", int(a))],
                    );
                }
            }
        }
        let width = t.bytes() as usize;
        let mut octets: Vec<Vec<u8>> = [0u32, 0x80, 0xF8]
            .iter()
            .map(|k| (0..width as u32).map(|x| ((k + x) & 0xFF) as u8).collect())
            .collect();
        octets.extend(values.iter().map(|&a| t.be_bytes_of_pattern(a)));
        for tag in ["from_be", "from_le"] {
            for b in &octets {
                let ir = n(tag, Some(t), None, vec![var("b")]);
                rows.row(&format!("{tag} {name}"), &ir, &[("b", Value::bytes(b))]);
            }
        }
        // unwrap_or: the default is used exactly when the producer is none
        for (a, b) in [(hi, 1), (hi, 0), (lo, 1), (0, 0)] {
            for d in [0, hi] {
                let ir = n(
                    "unwrap_or",
                    None,
                    None,
                    vec![
                        n("checked_add", None, Some(t), vec![var("a"), var("b")]),
                        var("d"),
                    ],
                );
                rows.row(
                    &format!("unwrap_or_checked_add {name}"),
                    &ir,
                    &[("a", int(a)), ("b", int(b)), ("d", int(d))],
                );
            }
        }
    }
    let ints: [i128; 7] = [-(1 << 70), -7, -1, 0, 1, 7, 1 << 70];
    let nats: [i128; 4] = [0, 1, 7, (1 << 64) - 1];
    for tag in ["div", "rem", "imin", "imax"] {
        for (ta, va) in [("Nat", &nats[..]), ("Int", &ints[..])] {
            for (tb, vb) in [("Nat", &nats[..]), ("Int", &ints[..])] {
                for &a in va {
                    for &b in vb {
                        let ir = n(tag, None, None, vec![var("a"), var("b")]);
                        rows.row(
                            &format!("{tag} {ta},{tb}"),
                            &ir,
                            &[("a", int(a)), ("b", int(b))],
                        );
                    }
                }
            }
        }
    }
    for data in [&b""[..], &b"\x00"[..], &b"\x01\x02\x03"[..]] {
        for index in [-1i128, 0, 1, 2, 3, 1 << 70] {
            for kind in ["Int", "Nat"] {
                if kind == "Nat" && index < 0 {
                    continue;
                }
                let ir = n("at", None, None, vec![var("b"), var("i")]);
                rows.row(
                    &format!("at Bytes,{kind}"),
                    &ir,
                    &[("b", Value::bytes(data)), ("i", int(index))],
                );
            }
        }
    }
    rows.out
}

/// The two-complement pattern of `a` as big-endian bytes (the oracle's `(a & mask).to_bytes`).
trait Pattern {
    fn be_bytes_of_pattern(self, a: i128) -> Vec<u8>;
}
impl Pattern for FixedTy {
    fn be_bytes_of_pattern(self, a: i128) -> Vec<u8> {
        let p = self.pattern(a);
        (0..self.bytes()).rev().map(|i| (p >> (8 * i)) as u8).collect()
    }
}

fn values_view(rename: Option<(&str, &'static str)>) -> Json {
    Json::obj([
        ("schema", Json::str("warrant-fixed-values-parity-v1")),
        (
            "format",
            Json::str(
                "tag type operands = result; integers decimal, Bool true/false, Bytes x<hex>, Option none|some <v>; \
                 a conversion's type is S->D; an Int/Nat operand pair is written Int,Nat",
            ),
        ),
        (
            "samples",
            Json::obj(TYPES.iter().map(|t| {
                (
                    t.name(),
                    Json::arr(sample(*t).iter().map(|v| Json::str(v.to_string()))),
                )
            })),
        ),
        (
            "counts",
            Json::obj([FixedTy::U8, FixedTy::U16, FixedTy::U32, FixedTy::U64].map(|t| {
                (
                    t.bits().to_string(),
                    Json::arr(counts(u64::from(t.bits())).into_iter().map(Json::Int)),
                )
            })),
        ),
        ("rows", Json::arr(value_rows(rename).iter().map(Json::str))),
    ])
}

#[test]
fn every_built_in_at_every_width_matches_the_oracle_values_table_byte_for_byte() {
    let text = values_view(None).render().expect("render");
    assert!(text == VALUES, "{}", first_difference(&text, VALUES));
}

// ---- the domains dump ------------------------------------------------------------------------

fn show_eval(v: &EvalValue) -> String {
    match v {
        EvalValue::Fixed(_, n) | EvalValue::Int(n) => n.to_string(),
        EvalValue::U8(n) => n.to_string(),
        EvalValue::U64(n) | EvalValue::Nat(n) => n.to_string(),
        EvalValue::Bytes(b) => hex(b),
        other => panic!("a leaf value: {other:?}"),
    }
}

/// One leaf of the domain table: its descriptor, label and size, and either every value (`full`) or
/// the first and the last four.
fn leaf(d: &Domain, full: bool) -> Json {
    let (descriptor, label, size) = describe(d, 1).expect("a leaf describes");
    let values: Vec<String> = d.iter().map(|v| show_eval(&v)).collect();
    assert_eq!(values.len() as u64, size);
    let mut row = vec![
        ("descriptor", descriptor),
        ("label", Json::str(label.name())),
        ("size", Json::Int(size)),
    ];
    if full {
        row.push(("values", Json::strs(&values)));
    } else {
        row.push(("head", Json::strs(&values[..4])));
        row.push(("tail", Json::strs(&values[values.len() - 4..])));
    }
    Json::obj(row)
}

fn desc(kind: &str, size: u64, t: Option<&str>, extra: Option<(&str, u64)>) -> Json {
    let mut row = vec![("d", Json::str(kind)), ("size", Json::Int(size))];
    if let Some(t) = t {
        row.push(("t", Json::str(t)));
    }
    if let Some((k, v)) = extra {
        row.push((k, Json::Int(v)));
    }
    Json::obj(row)
}

/// The descriptors a consumer must accept, then reject (the oracle's `ACCEPTED` and `REJECTED`).
fn consumer_descriptors() -> Vec<Json> {
    let accepted = vec![
        desc("fx_edge", 281, Some("U16"), None),
        desc("fx_edge", 527, Some("I16"), None),
        desc("fx_edge", 559, Some("I32"), None),
        desc("fx_edge", 623, Some("I64"), None),
        desc("nat_shift", 73, None, None),
        Json::obj([
            ("d", Json::str("tuple")),
            (
                "of",
                Json::arr([desc("nat_shift", 73, None, None), desc("bool", 2, None, None)]),
            ),
            ("size", Json::Int(146)),
        ]),
    ];
    let rejected = vec![
        desc("fx_edge", 256, Some("U8"), None),
        desc("fx_edge", 256, Some("I8"), None),
        desc("fx_edge", 377, Some("U64"), None),
        desc("fx_edge", 377, Some("I16"), None),
        desc("fx_edge", 377, Some("I64"), None),
        desc("fx_edge", 314, Some("U32"), None),
        desc("fx_full", 256, Some("U8"), None),
        desc("fx_full", 4_294_967_296, Some("U32"), None),
        desc("nat_shift", 72, None, None),
        desc("nat_shift", 73, None, Some(("k", 64))),
        desc("nat_count", 73, None, Some(("k", 64))),
    ];
    accepted.into_iter().chain(rejected).collect()
}

fn verdict(d: &Json) -> Json {
    match descriptor_size(d, 1) {
        Ok(size) => Json::obj([
            ("descriptor", d.clone()),
            ("accepted", Json::Bool(true)),
            ("size", Json::Int(size)),
        ]),
        Err(e) => Json::obj([
            ("descriptor", d.clone()),
            (
                "oracle_code",
                Json::str(if e.code == code::LIMIT { "limit" } else { "generation" }),
            ),
            ("message", Json::str(e.message)),
        ]),
    }
}

fn domains_view() -> Json {
    let leaves = Json::obj([
        ("fx_edge:U16", leaf(&Domain::FxEdge(FixedTy::U16), true)),
        ("fx_edge:U32", leaf(&Domain::Fx(FixedTy::U32), true)),
        ("fx_edge:I16", leaf(&Domain::FxEdge(FixedTy::I16), true)),
        ("fx_edge:I32", leaf(&Domain::Fx(FixedTy::I32), true)),
        ("fx_edge:I64", leaf(&Domain::Fx(FixedTy::I64), true)),
        ("nat_shift", leaf(&Domain::NatCount, true)),
        ("nat_0_64", leaf(&Domain::Nat, false)),
        ("u64_edge", leaf(&Domain::Fx(FixedTy::U64), true)),
        ("fx_full:I8", leaf(&Domain::Fx(FixedTy::I8), false)),
        ("fx_full:U16", leaf(&Domain::Fx(FixedTy::U16), false)),
        ("fx_full:I16", leaf(&Domain::Fx(FixedTy::I16), false)),
        ("bytes_n:2", leaf(&Domain::BytesN(2), true)),
    ]);
    Json::obj([
        ("schema", Json::str("warrant-domain-parity-v1")),
        ("leaves", leaves),
        (
            "consumer",
            Json::arr(consumer_descriptors().iter().map(verdict)),
        ),
    ])
}

#[test]
fn the_leaf_domains_and_the_consumer_check_match_the_oracle_domain_table_byte_for_byte() {
    let text = domains_view().render().expect("render");
    assert!(text == DOMAINS, "{}", first_difference(&text, DOMAINS));
}

// ---- the dumps are canonical ------------------------------------------------------------------

#[test]
fn the_dumps_are_canonical_json_of_their_schemas() {
    for (name, d, schema) in [
        ("lowering", LOWERING, "warrant-lowering-parity-v1"),
        ("evaluation", EVALUATION, "warrant-evaluation-parity-v1"),
        ("faults", FAULTS, "warrant-fault-parity-v1"),
        ("values", VALUES, "warrant-fixed-values-parity-v1"),
        ("domains", DOMAINS, "warrant-domain-parity-v1"),
    ] {
        assert!(!d.contains('\r'), "{name}");
        assert!(d.ends_with("}\n") && !d.ends_with("\n\n"), "{name}");
        assert!(d.is_ascii(), "{name}");
        assert!(d.contains(&format!("\"schema\": \"{schema}\"")), "{name}");
    }
}

#[test]
fn the_values_table_is_not_vacuous() {
    let rows = value_rows(None);
    assert!(rows.len() > 9_000, "{}", rows.len());
    // every one of the 35 built-ins has rows
    for tag in coding_adventures_warrant_corpus_gen::ir::FIXED_TAGS {
        assert!(
            rows.iter().any(|r| r.starts_with(&format!("{tag} "))
                || r.starts_with("unwrap_or_checked_add") && tag == "unwrap_or"),
            "{tag}"
        );
    }
    // the cases a host's native shift or division gets wrong, as the oracle pins them
    for row in [
        "shl U8 1 8 = 0",
        "sar I8 -128 9 = -1",
        "bit U64 1 64 = false",
        "rotl U8 127 9 = 254",
        "div_trunc I8 -128 -1 = none",
        "rem_trunc I8 -128 -1 = none",
        "div_trunc I8 -128 0 = none",
        "checked_mul U64 18446744073709551615 18446744073709551615 = none",
        "wrap U8->I8 255 = -1",
        "div Int,Nat -7 7 = some -1",
        "rem Int,Nat -7 7 = some 0",
    ] {
        assert!(rows.iter().any(|r| r == row), "{row}");
    }
}

// ---- the evaluation dump holds the S10c domains ------------------------------------------------

/// The descriptor of binder `binder` of the suite of claim `claim`, as the committed dump has it.
fn dumped_descriptor(claim: &str, binder: usize) -> String {
    let dump = parse(EVALUATION).expect("the evaluation dump parses");
    let suite = dump.get("suites").get(claim);
    let b = &suite.get("binders").a()[binder];
    to_json(b.a()[1].get("domain"))
        .render()
        .unwrap()
        .split_whitespace()
        .collect()
}

#[test]
fn the_shift_count_rule_selects_exactly_where_the_oracle_does() {
    // R10, from the generated corpus itself and the committed dump together
    let corpus = parse(&shipped().corpus_json).unwrap();
    let selected = [
        "c_sel_shl",
        "c_sel_shr",
        "c_sel_sar",
        "c_sel_rotl",
        "c_sel_rotr",
        "c_sel_bit",
        "c_sel_in_tuple",
        "c_sel_in_subject_arg",
        "c_sel_parenthesised",
        "c_sel_and_admit",
        "c_sel_in_arm",
        "c_sh_u8",
        "c_sh_i64",
    ];
    let kept = ["c_nosel_expr", "c_nosel_model", "c_nosel_admit"];
    let nat_binder = |claim: &str| -> String {
        let suite = corpus.get("suites").a().iter().find(|s| s.get("claim").s() == claim);
        let suite = suite.unwrap_or_else(|| panic!("no suite {claim}"));
        let b = suite
            .get("binders")
            .a()
            .iter()
            .find(|b| b.a()[1].get("type").s() == "Nat")
            .unwrap_or_else(|| panic!("{claim} has no Nat binder"));
        b.a()[1].get("domain").get("d").s().to_string()
    };
    for claim in selected {
        assert_eq!(nat_binder(claim), "nat_shift", "{claim}");
    }
    for claim in kept {
        assert_eq!(nat_binder(claim), "nat_0_64", "{claim}");
    }
    // the parenthesised row is the one the grammar crate used to miss
    assert_eq!(
        dumped_descriptor("c_sel_parenthesised", 1),
        r#"{"d":"nat_shift","size":73}"#
    );
    // and in a document with two Nat binders only the count selects
    assert_eq!(
        dumped_descriptor("c_sel_one_of_two", 0),
        r#"{"d":"nat_shift","size":73}"#
    );
    assert_eq!(
        dumped_descriptor("c_sel_one_of_two", 1),
        r#"{"d":"nat_0_64","size":65}"#
    );
}

#[test]
fn the_sixteen_bit_binders_are_demoted_to_fx_edge_by_the_budget_alone() {
    let corpus = parse(&shipped().corpus_json).unwrap();
    let mut demoted = 0;
    for s in corpus.get("suites").a() {
        for (index, b) in s.get("binders").a().iter().enumerate() {
            let domain = b.a()[1].get("domain");
            if domain.get("d").s() == "fx_edge" {
                // [D25]: 16-bit only by demotion; 8-bit never (the resolver cannot make one)
                let t = domain.get("t").s();
                assert!(["U16", "I16", "U32", "I32", "I64"].contains(&t), "{t}");
                if t.ends_with("16") {
                    assert!(
                        // `demoted` lists the binder indices the budget demoted, as decimal text
                        s.get("demoted").a().iter().any(|d| d.s() == index.to_string()),
                        "{}: a 16-bit fx_edge that is not a recorded demotion",
                        s.get("claim").s()
                    );
                    demoted += 1;
                }
            }
        }
    }
    assert!(demoted > 5, "{demoted}");
    // an 8-bit fx_edge descriptor has no form
    assert!(describe(&Domain::FxEdge(FixedTy::I8), 1).is_err());
}

#[test]
fn a_nat_that_is_the_count_of_a_shift_never_selects_through_an_admit() {
    // the admit never selects: the claim's `n` stays `nat_0_64`
    assert_eq!(
        dumped_descriptor("c_nosel_admit", 1),
        r#"{"d":"nat_0_64","size":65}"#
    );
    // c_sel_and_admit counts in the body as well, so it selects
    assert_eq!(
        dumped_descriptor("c_sel_and_admit", 1),
        r#"{"d":"nat_shift","size":73}"#
    );
}

// ---- the gate stays closed ----------------------------------------------------------------------

#[test]
fn the_shipped_gate_defers_every_ir_fixed_fault_and_the_open_gate_searches_it() {
    let (shipped, open) = (shipped(), open());
    assert_ne!(shipped.corpus_json, open.corpus_json);
    assert!(shipped.corpus_json.contains(r#""deferred": "ir.fixed""#));
    assert!(!open.corpus_json.contains(r#""deferred": "ir.fixed""#));
    assert!(!open.corpus_json.contains(r#""deferred": "ir.tuple""#));
    // nothing but the controls differ: no suite, chunk or example moves with the gate
    assert_eq!(shipped.chunk_files, open.chunk_files);
    assert_eq!(shipped.cases_tfv, open.cases_tfv);
    // the open gate finds witnesses and, for the tuple-and-fixed pair, also a blind control would
    // show: every fault of this document has one
    assert!(open.corpus_json.contains(r#""chunk": 0"#));
}

#[test]
fn generate_accepts_an_ir_fixed_document_and_still_refuses_a_v0_one() {
    assert!(shipped().corpus_json.contains("ir.fixed"));
    // the built-in names are version scoped: a v0 document keeps the level refusal
    let v0 = "warrant v0doc\n  op f(a: U8) -> U8\n  model m(a: U8) -> U8 = bit_not(a)\n  claim c:\n    forall a: U8. f(a) == m(a)\n";
    let e: GenError = with_file(v0, |f| lower_document(f).expect_err("refused"));
    assert!(e.code == code::LEVEL || e.code == code::CHECK, "{e}");
}

// ---- negative controls --------------------------------------------------------------------------

#[test]
fn a_mis_lowered_built_in_loses_lowering_parity() {
    let mutated = WARRANT.replacen("bit_and(a, b)", "bit_or(a, b)", 1);
    assert_ne!(mutated, WARRANT);
    assert_ne!(lowering_text(&mutated), LOWERING);
}

#[test]
fn a_changed_built_in_changes_the_evaluation_dump() {
    // `bit_and` evaluated as `bit_or` in the U8 reference: every expected digest of c_bin_u8 moves
    let mutated = WARRANT.replacen(
        "model bin_u8(a: U8, b: U8) -> (U8,U8,U8,U8,U8,U8) = (bit_and(a, b),",
        "model bin_u8(a: U8, b: U8) -> (U8,U8,U8,U8,U8,U8) = (bit_or(a, b),",
        1,
    );
    assert_ne!(mutated, WARRANT);
    let text = evaluation_text(&mutated, PROFILE);
    assert_ne!(text, EVALUATION);
    let before = parse(EVALUATION).unwrap();
    let after = parse(&text).unwrap();
    let (b, a) = (
        before.get("suites").get("c_bin_u8"),
        after.get("suites").get("c_bin_u8"),
    );
    assert_ne!(b.get("expected_sha256"), a.get("expected_sha256"));
    assert_eq!(b.get("inputs_sha256"), a.get("inputs_sha256"));
}

#[test]
fn an_expression_count_loses_the_nat_shift_domain() {
    // shl(a, (n)) -> shl(a, len(take(..., n))): the count is now an expression of `n`, so `n` keeps
    // nat_0_64 (the same expression c_nosel_expr uses)
    let mutated = WARRANT.replacen("shl(a, (n))", "shl(a, len(take(b\"0102030405\", n)))", 1);
    assert_ne!(mutated, WARRANT);
    let mutated_corpus = generated(&mutated, SPEC, PROFILE, &GATE_CLOSED).unwrap();
    assert_ne!(evaluation_view(&mutated_corpus).render().unwrap(), EVALUATION);
    let corpus = parse(&mutated_corpus.corpus_json).unwrap();
    let suite = corpus
        .get("suites")
        .a()
        .iter()
        .find(|s| s.get("claim").s() == "c_sel_parenthesised")
        .unwrap();
    assert_eq!(suite.get("binders").a()[1].a()[1].get("domain").get("d").s(), "nat_0_64");
}

#[test]
fn a_changed_profile_changes_the_fault_dump() {
    let mutated = PROFILE.replacen("fault op_x_u8 xor-lsb\n", "", 1);
    assert_ne!(mutated, PROFILE);
    let text = faults_view(
        &generated(WARRANT, SPEC, &mutated, &GATE_CLOSED).unwrap(),
        &generated(WARRANT, SPEC, &mutated, &GATE_OPEN).unwrap(),
    )
    .render()
    .unwrap();
    assert_ne!(text, FAULTS);
    assert!(!text.contains("op_x_u8/xor-lsb"));
    assert!(FAULTS.contains("op_x_u8/xor-lsb"));
}

#[test]
fn a_wrong_evaluator_loses_values_parity() {
    // `sar` read as `shr`: the sign no longer fills, so the rows of every signed type move
    let wrong = values_view(Some(("sar", "shr"))).render().unwrap();
    assert_ne!(wrong, VALUES);
    // and `rotl` as `rotr`
    let wrong = values_view(Some(("rotl", "rotr"))).render().unwrap();
    assert_ne!(wrong, VALUES);
    // only the named rows move: the same table with no rename is the oracle's
    assert_eq!(values_view(None).render().unwrap(), VALUES);
}

#[test]
fn a_wrong_domain_loses_domains_parity() {
    let mut text = domains_view().render().unwrap();
    assert_eq!(text, DOMAINS);
    // the withdrawn 377-value signed set would shorten the I64 list
    text = text.replacen("\"size\": 623", "\"size\": 377", 1);
    assert_ne!(text, DOMAINS);
    // a consumer that accepted an 8-bit fx_edge would change a verdict
    let lenient = Json::obj([("d", Json::str("fx_edge")), ("t", Json::str("U8")), ("size", Json::Int(256))]);
    assert!(descriptor_size(&lenient, 1).is_err());
}

#[test]
fn the_types_of_the_document_are_all_eight() {
    // not a vacuous fixture: every fixed-width type is a binder type of some suite
    let corpus = parse(&shipped().corpus_json).unwrap();
    let mut seen: Vec<String> = Vec::new();
    for s in corpus.get("suites").a() {
        for b in s.get("binders").a() {
            seen.push(b.a()[1].get("type").s().to_string());
        }
    }
    for t in TYPES {
        assert!(seen.iter().any(|s| s == t.name()), "{}", t.name());
    }
}
