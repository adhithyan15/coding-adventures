//! Helpers shared by the oracle parity tests (`tuple_parity.rs`, `fixed_eval_parity.rs`): the
//! oracle's `warrant-evaluation-parity-v1` view of a generated corpus, a readable first difference,
//! and generation under a chosen fault gate.

#![allow(dead_code)]

use coding_adventures_warrant_corpus_gen::error::GenError;
use coding_adventures_warrant_corpus_gen::faults::Level;
use coding_adventures_warrant_corpus_gen::json::Json;
use coding_adventures_warrant_corpus_gen::{generate_gated, CorpusV2, GenInput};

use super::minijson::{parse, J};
use super::with_file;

/// The **closed** fault gate: only `ir.core` is evaluated (the oracle's `parity_dumps.GATE_CLOSED`).
///
/// The parity dumps pin their gate explicitly instead of taking the default of `generate`
/// (`SHIPPED_GATE`).  The dumps record what the oracle computes under a *named* gate; when the
/// atomic flip opens the shipped gate to `ir.fixed` and `ir.tuple`, only `SHIPPED_GATE` and the
/// fixtures that record it move, and these closed-gate views stay byte for byte what they were.
pub const GATE_CLOSED: [Level; 1] = [Level::Core];

/// The first line where two texts differ, with a little context, for a readable failure.
pub fn first_difference(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}: rust `{x}` vs oracle `{y}`", i + 1);
        }
    }
    format!(
        "lengths differ: {} vs {} lines",
        a.lines().count(),
        b.lines().count()
    )
}

pub fn generated(
    src: &str,
    spec_path: &str,
    profile: &str,
    gate: &[Level],
) -> Result<CorpusV2, GenError> {
    with_file(src, |f| {
        let input = GenInput {
            file: f,
            spec_path,
            spec_bytes: src.as_bytes(),
            profile_bytes: profile.as_bytes(),
            binding_faults: None,
        };
        generate_gated(&input, gate)
    })
}

pub fn to_json(j: &J) -> Json {
    match j {
        J::B(b) => Json::Bool(*b),
        J::N(n) => Json::Int(*n),
        J::S(s) => Json::str(s.clone()),
        J::A(items) => Json::Arr(items.iter().map(to_json).collect()),
        J::O(pairs) => Json::Obj(pairs.iter().map(|(k, v)| (k.clone(), to_json(v))).collect()),
        J::Null => panic!("a v2 corpus holds no null"),
    }
}

/// The suite-row keys of the evaluation view (the oracle's `parity_dumps._SUITE_KEYS`).
const SUITE_KEYS: [&str; 23] = [
    "id",
    "claim",
    "kind",
    "requires",
    "binders",
    "demoted",
    "domain_label",
    "stream",
    "result_type",
    "product",
    "chunks",
    "cases",
    "discarded",
    "expected_calls",
    "wraps",
    "ir_cost",
    "slim_chunks",
    "slim_inputs_sha256",
    "slim_expected_sha256",
    "inputs_sha256",
    "expected_sha256",
    "v1_inputs_sha256",
    "v1_expected_sha256",
];

/// `warrant-evaluation-parity-v1`: what the generator evaluates (the oracle's
/// `parity_dumps.evaluation_view`).
pub fn evaluation_view(c: &CorpusV2) -> Json {
    let corpus = parse(&c.corpus_json).expect("corpus.json parses");
    let mut suites = Vec::new();
    for row in corpus.get("suites").a() {
        let claim = row.get("claim").s();
        let mut view: Vec<(String, Json)> = SUITE_KEYS
            .iter()
            .filter(|k| row.has(k))
            .map(|k| (k.to_string(), to_json(row.get(k))))
            .collect();
        let file = format!("{}.txt", claim.replace('/', "."));
        let text = &c
            .chunk_files
            .iter()
            .find(|(n, _)| *n == file)
            .unwrap_or_else(|| panic!("no chunk file for {claim}"))
            .1;
        view.push((
            "chunk_lines".into(),
            Json::arr(text.lines().map(Json::str)),
        ));
        suites.push((claim.to_string(), Json::Obj(view)));
    }
    let ops = match corpus.get("ops") {
        J::O(pairs) => Json::Obj(
            pairs
                .iter()
                .map(|(n, row)| (n.clone(), to_json(row.get("requires"))))
                .collect(),
        ),
        _ => panic!("ops is an object"),
    };
    let mut out = vec![
        ("schema".to_string(), Json::str("warrant-evaluation-parity-v1")),
        ("module".to_string(), to_json(corpus.get("module"))),
        ("document".to_string(), to_json(corpus.get("document"))),
        ("chunk_size".to_string(), to_json(corpus.get("chunk_size"))),
        ("budget".to_string(), to_json(corpus.get("budget"))),
        ("max_ir_cost".to_string(), to_json(corpus.get("max_ir_cost"))),
        ("levels".to_string(), to_json(corpus.get("levels"))),
        ("ops".to_string(), ops),
        ("suites".to_string(), Json::Obj(suites)),
        ("examples".to_string(), to_json(corpus.get("examples"))),
        ("cases_tfv".to_string(), Json::str(c.cases_tfv.clone())),
    ];
    if corpus.has("omitted_examples") {
        out.push((
            "omitted_examples".to_string(),
            to_json(corpus.get("omitted_examples")),
        ));
    }
    Json::Obj(out)
}

