//! The S3c acceptance tests: ct-compare end to end, and the emitted bytes pinned.
//!
//! `generate` turns ct-compare into a v2 corpus: `corpus.json`, nine chunk files and
//! `cases.tfv`.  The tests pin it three ways, none of them a committed fixture (the committed
//! `ct-compare-warrant-v2/` directory is slice S3d, VER04r 16.3 [D10]):
//!
//! * the **eighteen v1 digests** are re-derived from the *emitted* `corpus.json` and compared
//!   with the v1 corpus, and every digest in `corpus.json` is re-derived from the emitted chunk
//!   files (a file and the corpus that names it cannot disagree);
//! * `cases.tfv` equals the v1 file byte for byte, the chunk files equal the independent
//!   oracle's `chunk_lines`, and the **eight control rows** are asserted in full: the three
//!   blind pairs, the `ir.fixed` `eq_u64/ignore-high-bit` (live under the shipped gate) and the witness chunk of each;
//! * the bytes of `corpus.json` are pinned by SHA-256 (with the generator version masked, so a
//!   version bump does not look like a behaviour change).
//!
//! The witness search is also checked against a **brute-force search written in this file**
//! with hand-coded fault semantics, so a pair mis-classified as witnessed (or blind) is caught.

mod support;

use std::sync::OnceLock;

use coding_adventures_sha256::{sha256, sha256_hex};
use coding_adventures_warrant_corpus_gen::emit::GENERATOR_VERSION;
use coding_adventures_warrant_corpus_gen::{code, generate, CorpusV2, GenInput};
use support::minijson::{parse, J};
use support::*;

const V1_CORPUS: &str =
    include_str!("../../../../specs/fixtures/ct-compare-warrant-v1/corpus.json");
const V1_TFV: &str = include_str!("../../../../specs/fixtures/ct-compare-warrant-v1/cases.tfv");
pub const CT_PROFILE: &str = include_str!("../../../../specs/fixtures/ct-compare-warrant-v2/corpus.profile");
const ORACLE_ENUM: &str = include_str!("fixtures/ct-compare.oracle-enumeration.json");

pub fn gen(src: &str, path: &str, profile: &str) -> Result<CorpusV2, coding_adventures_warrant_corpus_gen::GenError> {
    with_file(src, |f| {
        generate(&GenInput {
            file: f,
            spec_path: path,
            spec_bytes: src.as_bytes(),
            profile_bytes: profile.as_bytes(),
            binding_faults: None,
        })
    })
}

fn ct() -> &'static CorpusV2 {
    static C: OnceLock<CorpusV2> = OnceLock::new();
    C.get_or_init(|| gen(CT_COMPARE, "code/specs/ct-compare.warrant", CT_PROFILE).expect("ct-compare generates"))
}

fn corpus() -> J {
    parse(&ct().corpus_json).expect("corpus.json is strict JSON")
}

fn suite<'a>(c: &'a J, claim: &str) -> &'a J {
    c.get("suites").a().iter().find(|s| s.get("claim").s() == claim).expect("suite")
}

fn short(id: &str) -> &str {
    id.rsplit('/').next().unwrap()
}

// ---- the artifacts ------------------------------------------------------------------

#[test]
fn the_file_set_and_sizes_are_those_of_ct_compare() {
    let c = ct();
    let mut names: Vec<&str> = c.chunk_files.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "eq_agrees.txt", "eq_fixed_agrees_w0.txt", "eq_fixed_agrees_w1.txt",
            "eq_fixed_agrees_w16.txt", "eq_fixed_agrees_w2.txt", "eq_fixed_agrees_w4.txt",
            "eq_u64_agrees.txt", "select_picks.txt", "select_rejects_mismatch.txt"
        ]
    );
    assert_eq!(c.files().len(), 11);
    assert!(c.corpus_json.ends_with("}\n") && !c.corpus_json.contains('\r'));
    // The sizes in the report; a change here is a behaviour change to review.
    let masked = c.corpus_json.replace(&format!("\"version\": \"{GENERATOR_VERSION}\""), "\"version\": \"VERSION\"");
    assert_eq!(masked.len(), CORPUS_BYTES);
    assert_eq!(c.cases_tfv.len(), V1_TFV.len());
}

const CORPUS_BYTES: usize = 43780; // with the version masked as `VERSION` (43792 before the S10c flip: the `deferred` witness was 12 bytes longer)
const CORPUS_DIGEST: &str = "2ecd1225005e325cca8f86271170cb09f92aa5bb3d58344851d9f219b1a3f66c";

#[test]
fn corpus_json_is_pinned_by_digest_with_the_version_masked() {
    let masked = ct()
        .corpus_json
        .replace(&format!("\"version\": \"{GENERATOR_VERSION}\""), "\"version\": \"VERSION\"");
    assert_ne!(masked, ct().corpus_json, "the version is in the corpus");
    assert_eq!(sha256_hex(masked.as_bytes()), CORPUS_DIGEST);
}

#[test]
fn generation_is_deterministic() {
    let again = gen(CT_COMPARE, "code/specs/ct-compare.warrant", CT_PROFILE).unwrap();
    assert_eq!(&again, ct());
}

#[test]
fn cases_tfv_is_byte_identical_to_v1() {
    assert_eq!(ct().cases_tfv, V1_TFV);
}

#[test]
fn the_key_set_is_closed_sorted_and_has_no_null() {
    let c = corpus();
    assert_eq!(
        c.keys(),
        [
            "accepted_blind_faults", "assumes", "budget", "chunk_size", "controls", "document",
            "examples", "fault_models", "faults", "generator", "levels", "max_ir_cost",
            "model_order", "models", "module", "ops", "profile", "profile_sha256", "schema_version",
            "spec", "spec_sha256", "suites", "types"
        ]
    );
    assert_eq!(c.get("profile").s(), "warrant-corpus-v2");
    assert_eq!(c.get("schema_version").n(), 2);
    assert_eq!(c.get("module").s(), "ct_compare");
    assert_eq!(c.get("document").s(), "v0");
    assert_eq!(c.get("spec").s(), "code/specs/ct-compare.warrant");
    assert_eq!(c.get("spec_sha256").s(), sha256_hex(CT_COMPARE.as_bytes()));
    assert_eq!(c.get("profile_sha256").s(), sha256_hex(CT_PROFILE.as_bytes()));
    assert_eq!(c.get("generator").get("name").s(), "warrant-corpus-gen");
    assert_eq!(c.get("generator").get("version").s(), GENERATOR_VERSION);
    assert_eq!(c.get("budget").n(), 1 << 24);
    assert_eq!(c.get("chunk_size").n(), 65536);
    assert_eq!(c.get("max_ir_cost").n(), 2000);
    assert_eq!(c.get("levels").strs(), ["ir.core", "ir.fixed"]);
    assert!(c.get("fault_models").keys().is_empty());
    assert!(c.get("accepted_blind_faults").a().is_empty());
    assert!(c.get("assumes").a().is_empty());
    assert_eq!(c.get("model_order").strs(), ["same", "same_u64"]);
    assert_eq!(c.get("models").keys(), ["same", "same_u64"]);
    assert_eq!(
        c.get("types").get("SelectError").get("cases").strs(),
        ["CT_SELECT_LENGTH_MISMATCH"]
    );
    // every key of every object is sorted (the canonical writer), recursively
    fn sorted(j: &J) {
        match j {
            J::O(p) => {
                let k: Vec<&str> = p.iter().map(|(n, _)| n.as_str()).collect();
                let mut s = k.clone();
                s.sort_unstable();
                assert_eq!(k, s);
                p.iter().for_each(|(_, v)| sorted(v));
            }
            J::A(v) => v.iter().for_each(sorted),
            _ => {}
        }
    }
    sorted(&c);
}

#[test]
fn suite_rows_carry_the_1_4_key_set_with_conditional_keys_omitted() {
    let c = corpus();
    let base = [
        "admit", "binders", "body", "cases", "chunks", "chunks_sha256", "claim", "demoted",
        "discarded", "domain_label", "expected_calls", "id", "inputs_sha256", "ir_cost", "kind",
        "ops", "oracle", "product", "requires", "result_type", "slim_chunks",
        "slim_inputs_sha256", "stream", "wraps",
    ];
    for s in c.get("suites").a() {
        let mut want: Vec<&str> = base.to_vec();
        // all nine suites are reference suites with the v1 header
        want.extend(["expected_sha256", "slim_expected_sha256", "v1_expected_sha256", "v1_inputs_sha256"]);
        want.sort_unstable();
        let mut got = s.keys();
        got.sort_unstable();
        assert_eq!(got, want, "{}", s.get("claim").s());
        assert_eq!(s.get("oracle").get("kind").s(), "reference");
        assert_eq!(s.get("stream").s(), "warrant-stream-v1");
        assert_eq!(s.get("kind").s(), "exhaustive");
        assert_eq!(s.get("requires").strs(), ["ir.core"]);
    }
    // one binder row spelled out
    let b = suite(&c, "eq_fixed_agrees_w1").get("binders");
    assert_eq!(b.a()[0].a()[0].s(), "a");
    assert_eq!(b.a()[0].a()[1].get("type").s(), "Bytes<1>");
    assert!(b.a()[0].a()[1].get("domain").has("d"));
    // examples: `width` only where the call has a static width
    let ex = c.get("examples").a();
    assert_eq!(ex.len(), 12);
    assert!(!ct().corpus_json.contains("null"));
    assert!(!ex[0].has("width") && ex[4].get("width").n() == 0 && ex[5].get("width").n() == 16);
    assert_eq!(ex[7].get("expected").get("ok").s(), "00");
    assert_eq!(ex[9].get("expected").get("err").s(), "CT_SELECT_LENGTH_MISMATCH");
    assert_eq!(ex[11].get("args").a()[0].s(), "9223372036854775808");
    assert_eq!(ex[0].get("id").s(), "warrant/ct_compare/example/eq_empty");
}

// ---- the eighteen digests, from the emitted artifacts ---------------------------------

#[test]
fn the_eighteen_v1_digests_are_in_the_emitted_corpus() {
    let (c, v1) = (corpus(), parse(V1_CORPUS).unwrap());
    let mut n = 0;
    for old in v1.get("suites").a() {
        let new = suite(&c, old.get("claim").s());
        assert_eq!(new.get("v1_inputs_sha256").s(), old.get("inputs_sha256").s());
        assert_eq!(new.get("v1_expected_sha256").s(), old.get("expected_sha256").s());
        assert_eq!(new.get("cases").n(), old.get("cases").n());
        assert_eq!(new.get("discarded").n(), old.get("discarded").n());
        assert_eq!(new.get("expected_calls").n(), old.get("expected_calls").n());
        n += 2;
    }
    assert_eq!(n, 18);
}

fn raw(hex: &str) -> Vec<u8> {
    (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect()
}

fn framing(hexes: &[&str]) -> String {
    let mut buf = b"warrant-chunks-v1\0".to_vec();
    buf.extend_from_slice(&(hexes.len() as u32).to_le_bytes());
    for h in hexes {
        buf.extend(raw(h));
    }
    sha256_hex(&buf)
}

#[test]
fn every_digest_of_corpus_json_rederives_from_the_chunk_files() {
    let (c, files) = (corpus(), ct());
    let _ = sha256(b"");
    for (name, text) in &files.chunk_files {
        let claim = name.trim_end_matches(".txt");
        let s = suite(&c, claim);
        assert_eq!(s.get("chunks_sha256").s(), sha256_hex(text.as_bytes()), "{claim}");
        assert!(text.ends_with('\n') && !text.contains("\n\n") && !text.contains('\r'));
        let lines: Vec<Vec<&str>> = text.lines().map(|l| l.split(' ').collect()).collect();
        assert_eq!(lines.len() as u64, s.get("chunks").n());
        let (mut cases, mut first, mut calls) = (0u64, 0u64, 0u64);
        for (i, l) in lines.iter().enumerate() {
            assert_eq!(l.len(), 6, "reference suites carry an expected digest");
            assert_eq!(l[0].parse::<usize>().unwrap(), i);
            assert_eq!(l[2].parse::<u64>().unwrap(), first, "first_admitted is the prefix sum");
            let k: u64 = l[1].parse().unwrap();
            cases += k;
            first += k;
            calls += l[3].parse::<u64>().unwrap();
        }
        assert_eq!(cases, s.get("cases").n());
        assert_eq!(s.get("product").n() - cases, s.get("discarded").n());
        assert_eq!(calls, s.get("expected_calls").n());
        let ins: Vec<&str> = lines.iter().map(|l| l[4]).collect();
        let exs: Vec<&str> = lines.iter().map(|l| l[5]).collect();
        assert_eq!(s.get("inputs_sha256").s(), framing(&ins));
        assert_eq!(s.get("expected_sha256").s(), framing(&exs));
        let slim: Vec<usize> = s.get("slim_chunks").a().iter().map(|x| x.n() as usize).collect();
        assert_eq!(s.get("slim_inputs_sha256").s(), framing(&slim.iter().map(|i| ins[*i]).collect::<Vec<_>>()));
        assert_eq!(s.get("slim_expected_sha256").s(), framing(&slim.iter().map(|i| exs[*i]).collect::<Vec<_>>()));
    }
}

#[test]
fn chunk_files_equal_the_independent_oracles_chunk_lines() {
    let o = parse(ORACLE_ENUM).unwrap();
    for (name, text) in &ct().chunk_files {
        let claim = name.trim_end_matches(".txt");
        let want: Vec<&str> = o.get("suites").get(claim).get("chunk_lines").strs();
        assert_eq!(text.lines().collect::<Vec<_>>(), want, "{claim}");
    }
}

// ---- the eight controls ---------------------------------------------------------------

/// The shipped gate is open to `ir.fixed` and `ir.tuple` (the S10c flip) and, since S11c-2, to `ir.data`.
#[test]
fn the_shipped_gate_is_open_to_ir_fixed_ir_tuple_and_ir_data() {
    use coding_adventures_warrant_corpus_gen::faults::{Level, SHIPPED_GATE};
    assert_eq!(SHIPPED_GATE, [Level::Core, Level::Data, Level::Fixed, Level::Tuple]);
}

/// The closed gate (`ir.core` only) stays reachable through `generate_gated`: the `ir.fixed`
/// control is then `deferred`, and nothing but the controls differs from the shipped output.
#[test]
fn the_closed_gate_still_defers_the_ir_fixed_control() {
    use coding_adventures_warrant_corpus_gen::faults::Level;
    let closed = with_file(CT_COMPARE, |f| {
        coding_adventures_warrant_corpus_gen::generate_gated(
            &GenInput {
                file: f,
                spec_path: "code/specs/ct-compare.warrant",
                spec_bytes: CT_COMPARE.as_bytes(),
                profile_bytes: CT_PROFILE.as_bytes(),
                binding_faults: None,
            },
            &[Level::Core],
        )
    })
    .expect("ct-compare generates under the closed gate");
    let shipped = ct();
    assert_ne!(closed.corpus_json, shipped.corpus_json);
    assert!(closed.corpus_json.contains(r#""deferred": "ir.fixed""#));
    assert!(!shipped.corpus_json.contains(r#""deferred""#));
    assert_eq!(closed.chunk_files, shipped.chunk_files);
    assert_eq!(closed.cases_tfv, shipped.cases_tfv);
}

struct Row {
    id: &'static str,
    requires: &'static str,
    required: &'static [&'static str],
    blind: &'static [&'static str],
    /// `chunk N`, or the other two shapes
    witness: &'static [&'static str],
}

const W: [&str; 5] = [
    "eq_fixed_agrees_w0", "eq_fixed_agrees_w1", "eq_fixed_agrees_w2", "eq_fixed_agrees_w4",
    "eq_fixed_agrees_w16",
];

#[test]
fn the_eight_control_rows_have_the_right_states() {
    let c = corpus();
    let want = [
        Row { id: "eq/ignore-last-byte", requires: "ir.core", required: &["eq_agrees"], blind: &[], witness: &["chunk 0"] },
        Row { id: "eq/invert", requires: "ir.core", required: &["eq_agrees"], blind: &[], witness: &["chunk 0"] },
        // blind pair 1: Bytes<0> has no last byte
        Row { id: "eq_fixed/ignore-last-byte", requires: "ir.core", required: &W[1..], blind: &W[..1], witness: &["chunk 0"; 4] },
        Row { id: "eq_fixed/invert", requires: "ir.core", required: &W, blind: &[], witness: &["chunk 0"; 5] },
        // the ir.fixed control (D9): data in the fault row, and live since the flip (S10c, D29):
        // the shipped gate evaluates ir.fixed, so it has a witness chunk, not `deferred`
        // (the closed-gate test below pins the `deferred` shape)
        Row { id: "eq_u64/ignore-high-bit", requires: "ir.fixed", required: &["eq_u64_agrees"], blind: &[], witness: &["chunk 0"] },
        Row { id: "eq_u64/invert", requires: "ir.core", required: &["eq_u64_agrees"], blind: &[], witness: &["chunk 0"] },
        // blind pair 2: equal lengths never give Err
        Row { id: "select_bytes/missing-error", requires: "ir.core", required: &["select_rejects_mismatch"], blind: &["select_picks"], witness: &["chunk 0"] },
        // blind pair 3: a mismatch is Err whichever way choice flips
        Row { id: "select_bytes/swap-choice", requires: "ir.core", required: &["select_picks"], blind: &["select_rejects_mismatch"], witness: &["chunk 0"] },
    ];
    let rows = c.get("controls").a();
    assert_eq!(rows.len(), 8);
    for (r, w) in rows.iter().zip(&want) {
        let id = format!("warrant/ct_compare/negctl/exhaustive/{}", w.id);
        assert_eq!(r.get("id").s(), id);
        let (op, fault) = w.id.split_once('/').unwrap();
        assert_eq!((r.get("op").s(), r.get("fault").s()), (op, fault));
        assert_eq!(r.get("target_kind").s(), "exhaustive");
        assert_eq!(r.get("requires").strs(), [w.requires], "{id}");
        let req: Vec<&str> = r.get("required_claims").strs().into_iter().map(short).collect();
        let bl: Vec<&str> = r.get("blind_claims").strs().into_iter().map(short).collect();
        assert_eq!((req.as_slice(), bl.as_slice()), (w.required, w.blind), "{id}");
        let full = r.get("required_claims").strs();
        assert!(full.iter().all(|x| x.starts_with("warrant/ct_compare/exhaustive/")));
        assert!(!r.has("applies_to_forms"));
        let wit = r.get("slim_witness").a();
        assert_eq!(wit.len(), w.required.len(), "one witness entry per required claim");
        for ((e, claim), shape) in wit.iter().zip(&full).zip(w.witness) {
            assert_eq!(e.get("claim").s(), *claim);
            let got = if e.has("chunk") {
                format!("chunk {}", e.get("chunk").n())
            } else if e.has("deferred") {
                format!("deferred {}", e.get("deferred").s())
            } else {
                "unproven".to_string()
            };
            assert_eq!(&got, shape, "{id}");
            assert_eq!(e.keys().len(), 2);
        }
    }
    // the ir.fixed control is data in the fault row: the row says what it needs
    let f = c.get("faults").a().iter().find(|f| f.get("fault").s() == "ignore-high-bit").unwrap();
    assert_eq!(f.get("requires").strs(), ["ir.fixed"]);
    assert_eq!(f.get("kind").s(), "pre");
    let e = f.get("rewrites").a();
    assert_eq!((e[0].get("param").s(), e[1].get("param").s()), ("a", "b"));
    assert_eq!(e[0].get("expr").a()[0].s(), "bit_and");
}

#[test]
fn faults_are_sorted_by_op_then_name_with_the_right_kinds() {
    let c = corpus();
    let got: Vec<(String, String, String)> = c
        .get("faults")
        .a()
        .iter()
        .map(|f| (f.get("op").s().into(), f.get("fault").s().into(), f.get("kind").s().into()))
        .collect();
    let want = [
        ("eq", "ignore-last-byte", "pre"), ("eq", "invert", "post"),
        ("eq_fixed", "ignore-last-byte", "pre"), ("eq_fixed", "invert", "post"),
        ("eq_u64", "ignore-high-bit", "pre"), ("eq_u64", "invert", "post"),
        ("select_bytes", "missing-error", "replace"), ("select_bytes", "swap-choice", "pre"),
    ];
    assert_eq!(got.iter().map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str())).collect::<Vec<_>>(), want);
    // a fault row has `expr` xor `rewrites`
    for f in c.get("faults").a() {
        assert_ne!(f.has("expr"), f.has("rewrites"));
        assert!(!f.has("r"));
    }
    // the reserved name is `result`
    let inv = c.get("faults").a().iter().find(|f| f.get("fault").s() == "invert").unwrap();
    assert_eq!(inv.get("expr").a()[1].a()[1].s(), "result");
}

// ---- the witness search against a brute force written here ----------------------------

/// The hand-coded semantics of one (claim, fault): `Some(f)` maps the case's binder values to
/// `(expected, faulted)`.
mod brute {
    use super::*;
    use coding_adventures_warrant_corpus_gen::domains::resolve_claim;
    use coding_adventures_warrant_corpus_gen::eval::{Program, Stats};
    use coding_adventures_warrant_corpus_gen::lower::lower_document;
    use coding_adventures_warrant_corpus_gen::value::Value;
    use coding_adventures_warrant_grammar::domain::DomainEnv;

    pub type Case = Vec<Value>;
    pub type Sem = fn(&Case) -> (Value, Value);

    fn b(v: &Value) -> Vec<u8> {
        v.as_bytes().unwrap().to_vec()
    }
    fn ok(x: Vec<u8>) -> Value {
        Value::Ok(Box::new(Value::bytes(&x)))
    }
    fn err() -> Value {
        Value::Err("CT_SELECT_LENGTH_MISMATCH".into())
    }
    fn ignore_last(x: &[u8]) -> Vec<u8> {
        let mut y = x.to_vec();
        if let Some(l) = y.last_mut() {
            *l = 0;
        }
        y
    }

    pub fn invert_eq(c: &Case) -> (Value, Value) {
        let e = b(&c[0]) == b(&c[1]);
        (Value::Bool(e), Value::Bool(!e))
    }
    pub fn ignore_last_eq(c: &Case) -> (Value, Value) {
        (
            Value::Bool(b(&c[0]) == b(&c[1])),
            Value::Bool(ignore_last(&b(&c[0])) == ignore_last(&b(&c[1]))),
        )
    }
    pub fn invert_u64(c: &Case) -> (Value, Value) {
        let e = c[0] == c[1];
        (Value::Bool(e), Value::Bool(!e))
    }
    pub fn swap_choice(c: &Case) -> (Value, Value) {
        let ch = c[2].as_bool().unwrap();
        // the stand-in: the claim's reference at the flipped choice
        (select_ref(c, ch), select_ref(c, !ch))
    }
    fn select_ref(c: &Case, ch: bool) -> Value {
        // the references of the two claims: `Ok(if choice then a else b)` and the constant Err
        if b(&c[0]).len() == b(&c[1]).len() {
            ok(if ch { b(&c[0]) } else { b(&c[1]) })
        } else {
            err()
        }
    }
    pub fn missing_error(c: &Case) -> (Value, Value) {
        let e = select_ref(c, c[2].as_bool().unwrap());
        let f = match &e {
            Value::Err(_) => ok(b(&c[0])),
            other => other.clone(),
        };
        (e, f)
    }

    /// The lowest admitted ordinal where `sem` differs, or `None`.
    pub fn lowest(src: &str, claim: &str, sem: Sem) -> Option<u64> {
        with_file(src, |f| {
            let lowered = lower_document(f).unwrap();
            let program = Program::new(&lowered.models);
            let env = DomainEnv::new(f);
            let suite = lowered.suites.iter().find(|s| s.claim == claim).unwrap();
            let d = resolve_claim(f, &env, suite, 1 << 24).unwrap();
            let sizes: Vec<u64> = d.binders.iter().map(|b| b.size).collect();
            let mut st = Stats::default();
            for ordinal in 0..d.product {
                let (mut rest, mut idx) = (ordinal, vec![0u64; sizes.len()]);
                for j in (0..sizes.len()).rev() {
                    idx[j] = rest % sizes[j];
                    rest /= sizes[j];
                }
                let case: Case = d.binders.iter().zip(&idx).map(|(b, i)| b.value(*i).unwrap()).collect();
                let vars: Vec<(&str, Value)> =
                    d.binders.iter().map(|b| b.name.as_str()).zip(case.iter().cloned()).collect();
                if !program.eval(&suite.admit, &vars, &mut st).unwrap().as_bool().unwrap() {
                    continue;
                }
                let (e, f) = sem(&case);
                if e != f {
                    return Some(ordinal);
                }
            }
            None
        })
    }
}

fn chunk_of(c: &J, claim: &str, control: &str) -> Option<u64> {
    let row = c
        .get("controls")
        .a()
        .iter()
        .find(|r| r.get("id").s().ends_with(control))
        .unwrap();
    row.get("slim_witness")
        .a()
        .iter()
        .find(|e| short(e.get("claim").s()) == claim)
        .and_then(|e| e.has("chunk").then(|| e.get("chunk").n()))
}

#[test]
fn the_witness_search_agrees_with_a_brute_force_on_every_pair() {
    use brute::*;
    let c = corpus();
    type P = (&'static str, &'static str, Sem, bool);
    let pairs: Vec<P> = vec![
        ("eq_agrees", "eq/ignore-last-byte", ignore_last_eq, true),
        ("eq_agrees", "eq/invert", invert_eq, true),
        ("eq_fixed_agrees_w0", "eq_fixed/ignore-last-byte", ignore_last_eq, false),
        ("eq_fixed_agrees_w1", "eq_fixed/ignore-last-byte", ignore_last_eq, true),
        ("eq_fixed_agrees_w2", "eq_fixed/ignore-last-byte", ignore_last_eq, true),
        ("eq_fixed_agrees_w4", "eq_fixed/ignore-last-byte", ignore_last_eq, true),
        ("eq_fixed_agrees_w16", "eq_fixed/ignore-last-byte", ignore_last_eq, true),
        ("eq_fixed_agrees_w0", "eq_fixed/invert", invert_eq, true),
        ("eq_fixed_agrees_w16", "eq_fixed/invert", invert_eq, true),
        ("eq_u64_agrees", "eq_u64/invert", invert_u64, true),
        ("select_picks", "select_bytes/swap-choice", swap_choice, true),
        ("select_rejects_mismatch", "select_bytes/swap-choice", swap_choice, false),
        ("select_picks", "select_bytes/missing-error", missing_error, false),
        ("select_rejects_mismatch", "select_bytes/missing-error", missing_error, true),
    ];
    for (claim, control, sem, witnessed) in pairs {
        let brute = lowest(CT_COMPARE, claim, sem);
        assert_eq!(brute.is_some(), witnessed, "brute force: {claim} {control}");
        let got = chunk_of(&c, claim, control);
        assert_eq!(got, brute.map(|o| o / 65536), "{claim} {control}");
        // blind claims are listed in blind_claims, witnessed ones in required_claims
        let row = c.get("controls").a().iter().find(|r| r.get("id").s().ends_with(control)).unwrap();
        let in_blind = row.get("blind_claims").strs().into_iter().any(|x| short(x) == claim);
        assert_eq!(in_blind, !witnessed, "{claim} {control}");
    }
}

// ---- seeded-wrong: faults that cannot be seen -------------------------------------------

const W0_ONLY: &str = "warrant w0only
  op eq_fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool
  model same(a: Bytes, b: Bytes) -> Bool = a == b
  assume platform:
    none
  assume environment:
    none
  labels eq_fixed:
    a.contents secret
    a.length public
    b.contents secret
    b.length public
  input eq_fixed.a:
    hostile
    ensures no_panic, terminates
  input eq_fixed.b:
    hostile
    ensures no_panic, terminates
  bound eq_fixed:
    steps <= 64 * len(a) + 512
    heap <= 2 * len(a)
    depth <= 32
  claim w0:
    forall a: Bytes<0>, b: Bytes<0>. eq_fixed<0>(a, b) == same(a, b)
";

#[test]
fn a_fault_no_case_can_observe_fails_generation() {
    // `ignore-last-byte` on Bytes<0>: the fault is never applied, so the only candidate is blind
    let e = gen(W0_ONLY, "w.warrant", "fault eq_fixed ignore-last-byte\n").unwrap_err();
    assert_eq!(e.code, code::FAULT, "{e}");
    assert_eq!(e.at, "eq_fixed/ignore-last-byte");
    assert!(e.message.contains("blind"), "{e}");
    // the same document with a fault that IS observable generates, and w0 is a witnessed claim
    let ok = gen(W0_ONLY, "w.warrant", "fault eq_fixed invert\n").unwrap();
    let c = parse(&ok.corpus_json).unwrap();
    assert_eq!(c.get("controls").a()[0].get("blind_claims").a().len(), 0);
    assert_eq!(c.get("levels").strs(), ["ir.core"]);
}

#[test]
fn a_fault_on_an_op_no_claim_calls_fails_generation() {
    let doc = CT_COMPARE.to_string();
    // eq_u64 has a claim; `eq` has one too: drop every claim of `eq`
    let cut = doc.replace("  claim eq_agrees:\n    forall a: Bytes, b: Bytes. eq(a, b) == same(a, b)\n", "");
    assert_ne!(cut, doc);
    let e = gen(&cut, "x.warrant", "fault eq invert\n").unwrap_err();
    assert_eq!(e.code, code::FAULT);
    assert!(e.message.contains("no claim calls"), "{e}");
}

// ---- examples: signed values, and a value that does not fit ------------------------------

#[test]
fn examples_write_signed_values_and_refuse_values_outside_their_type() {
    let ops = ["op s8(a: Bytes, b: Bytes) -> I8", "op n(a: Bytes, b: Bytes) -> Nat"];
    let ok = doc_ops(" v1", &ops, "  example neg:\n    s8(b\"\", b\"00\") == -5i8\n  example big:\n    n(b\"\", b\"\") == 18446744073709551615\n");
    let c = gen(&ok, "s.warrant", "").expect("generates");
    assert_eq!(
        c.cases_tfv,
        "tfv 1\nwarrant/synth/example/neg R4 Kop S7338 Ka X Kb X00 Kexpect I-5\nwarrant/synth/example/big R4 Kop S6e Ka X Kb X Kexpect I18446744073709551615\n"
    );
    let j = parse(&c.corpus_json).unwrap();
    assert_eq!(j.get("examples").a()[0].get("expected").s(), "-5");
    assert!(c.chunk_files.is_empty());
    let bad = doc_ops(" v1", &ops, "  example big:\n    n(b\"\", b\"\") == 18446744073709551615 + 1\n");
    let e = gen(&bad, "s.warrant", "").unwrap_err();
    assert_eq!(e.code, code::W0701, "{e}");
    assert_eq!(e.at, "big");
}

// ---- accept_blind ([D15], VER04r 16.8) --------------------------------------------------

const ACCEPT_ID: &str = "warrant/w0only/negctl/exhaustive/eq_fixed/ignore-last-byte";

#[test]
fn an_all_blind_fault_named_by_accept_blind_generates_and_is_recorded() {
    let profile = "fault eq_fixed ignore-last-byte\nfault eq_fixed invert\naccept_blind eq_fixed ignore-last-byte\n";
    let out = gen(W0_ONLY, "w.warrant", profile).unwrap();
    let c = parse(&out.corpus_json).unwrap();
    assert_eq!(c.get("accepted_blind_faults").strs(), [ACCEPT_ID]);
    // the control row is still emitted: nothing required, the one candidate blind, no witness
    let rows = c.get("controls").a();
    assert_eq!(rows.len(), 2);
    let row = rows.iter().find(|r| r.get("id").s() == ACCEPT_ID).unwrap();
    assert!(row.get("required_claims").a().is_empty());
    assert_eq!(row.get("blind_claims").strs(), ["warrant/w0only/exhaustive/w0"]);
    assert!(row.get("slim_witness").a().is_empty());
    // the fault row is still listed, and the witnessed fault is not accepted
    assert_eq!(c.get("faults").a().len(), 2);
    let other = rows.iter().find(|r| r.get("fault").s() == "invert").unwrap();
    assert_eq!(other.get("required_claims").a().len(), 1);
    // the same profile without the accept line still fails closed
    let e = gen(W0_ONLY, "w.warrant", "fault eq_fixed ignore-last-byte\n").unwrap_err();
    assert_eq!(e.code, code::FAULT);
    assert!(e.message.contains("accept_blind"), "{e}");
}

#[test]
fn accepted_blind_faults_are_sorted_by_control_id_whatever_the_line_order() {
    let fixture = include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus.profile");
    // the committed profile lists flag before eq_fixed in both its fault and accept lines
    let a = parse(&gen_fixture(fixture).corpus_json).unwrap();
    assert_eq!(
        a.get("accepted_blind_faults").strs(),
        [
            "warrant/blindacc/negctl/exhaustive/eq_fixed/ignore-last-byte",
            "warrant/blindacc/negctl/exhaustive/flag/swap-choice",
        ]
    );
    let reversed = "fault eq_fixed invert\nfault flag invert\nfault eq_fixed ignore-last-byte\n\
                    fault flag swap-choice\naccept_blind eq_fixed ignore-last-byte\naccept_blind flag swap-choice\n";
    let b = parse(&gen_fixture(reversed).corpus_json).unwrap();
    assert_eq!(a.get("accepted_blind_faults").strs(), b.get("accepted_blind_faults").strs());
    assert_eq!(a.get("controls"), b.get("controls"));
}

#[test]
fn accept_blind_naming_a_fault_a_claim_can_see_is_a_profile_error() {
    // `invert` is witnessed at chunk 0: accepting it would excuse nothing (a stale profile)
    let e = gen(
        W0_ONLY,
        "w.warrant",
        "fault eq_fixed invert\naccept_blind eq_fixed invert\n",
    )
    .unwrap_err();
    assert_eq!(e.code, code::PROFILE, "{e}");
    assert_eq!(e.at, "eq_fixed/invert");
    assert!(e.message.contains("not all-blind"), "{e}");
    // ... also next to a fault that is genuinely all-blind and accepted
    let e = gen(
        W0_ONLY,
        "w.warrant",
        "fault eq_fixed invert\nfault eq_fixed ignore-last-byte\n\
         accept_blind eq_fixed ignore-last-byte\naccept_blind eq_fixed invert\n",
    )
    .unwrap_err();
    assert_eq!(e.code, code::PROFILE, "{e}");
    // an unproven claim is not blind either: ct-compare's predicate-free `eq` claim is
    // witnessed, and a fault with only unproven candidates has required claims
    let e = gen(CT_COMPARE, "x.warrant", &format!("{CT_PROFILE}accept_blind eq invert\n")).unwrap_err();
    assert_eq!(e.code, code::PROFILE, "{e}");
}

#[test]
fn accept_blind_without_a_fault_line_or_repeated_is_a_profile_error() {
    for profile in [
        "accept_blind eq_fixed ignore-last-byte\n",
        "fault eq_fixed invert\naccept_blind eq_fixed ignore-last-byte\n",
        "fault eq_fixed ignore-last-byte\naccept_blind eq_fixed ignore-last-byte\naccept_blind eq_fixed ignore-last-byte\n",
    ] {
        let e = gen(W0_ONLY, "w.warrant", profile).unwrap_err();
        assert_eq!(e.code, code::PROFILE, "{profile:?}");
    }
}

#[test]
fn accept_blind_does_not_excuse_a_fault_no_claim_calls() {
    let doc = CT_COMPARE.to_string();
    let cut = doc.replace("  claim eq_agrees:\n    forall a: Bytes, b: Bytes. eq(a, b) == same(a, b)\n", "");
    assert_ne!(cut, doc);
    let e = gen(&cut, "x.warrant", "fault eq invert\naccept_blind eq invert\n").unwrap_err();
    assert_eq!(e.code, code::FAULT, "{e}");
    assert!(e.message.contains("no claim calls"), "{e}");
}

#[test]
fn a_whitespace_only_profile_line_is_refused_whatever_follows() {
    for line in [" ", "\t", "\x0b"] {
        let e = gen(CT_COMPARE, "x.warrant", &format!("{CT_PROFILE}{line}\n")).unwrap_err();
        assert_eq!(e.code, code::PROFILE, "{line:?}");
    }
}

fn gen_fixture(profile: &str) -> CorpusV2 {
    let spec = include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/blind.warrant");
    gen(
        spec,
        "code/packages/python/warrant-oracle/tests/fixtures/accept-blind-v2/blind.warrant",
        profile,
    )
    .expect("the accept-blind fixture generates")
}

/// The committed accept-blind fixture is exactly what this generator writes (and what the
/// oracle's `verify-v2` accepts: `tests/test_accept_blind.py` in `python/warrant-oracle`).
#[test]
fn the_committed_accept_blind_fixture_is_what_the_generator_writes() {
    let profile = include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus.profile");
    let out = gen_fixture(profile);
    assert_eq!(
        out.corpus_json,
        include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus/corpus.json")
    );
    assert_eq!(
        out.cases_tfv,
        include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus/cases.tfv")
    );
    let committed: [(&str, &str); 2] = [
        (
            "flag_ignores_choice.txt",
            include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus/chunks/flag_ignores_choice.txt"),
        ),
        (
            "w0.txt",
            include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus/chunks/w0.txt"),
        ),
    ];
    let mut have: Vec<(&str, &str)> = out.chunk_files.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    have.sort_unstable();
    assert_eq!(have, committed);
}
