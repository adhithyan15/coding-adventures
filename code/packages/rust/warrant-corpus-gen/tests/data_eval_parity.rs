//! The S11c-2 acceptance test: the generator reproduces, byte for byte, every dump the oracle committed for
//! `ir.data` **evaluation** (VER04r section 13, slice S11c; 16.14 [D47] to [D54], R27 to R39; 16.17 R67; 16.19
//! R94; 16.20 R95 to R99; Appendix A.2; sections 2.3 to 2.5, 5.1, 5.3, 8.2).
//!
//! `python/warrant-oracle` committed, in slice S11c-1, a synthetic document (`data_eval_v1.warrant`: 9 records,
//! 4 variants, 60 models, 68 claims, 23 examples), its `.profile` (`budget 2^20`, twelve data faults) and six
//! dumps in `tests/fixtures/data-eval-v1/`:
//!
//! | dump | schema | what this test reproduces it from |
//! |---|---|---|
//! | `.oracle-lowering.json` | `warrant-lowering-parity-v1` | [`oracle_view_v1`] of the lowered document, with the `types` rows |
//! | `.oracle-evaluation.json` | `warrant-evaluation-parity-v1` | the generated corpus under the gate this slice opens |
//! | `.oracle-faults.json` | `warrant-fault-parity-v1` | the same corpus under the gate **before** `ir.data` (every data fault `deferred`) and the one with it |
//! | `.oracle-values.json` | `warrant-data-values-parity-v1` | the generator's JSON, stream and TFV writers on 28 values, and a consumer's reading of 23 JSON forms it must reject |
//! | `.oracle-domains.json` | `warrant-data-domain-parity-v1` | the resolved descriptors, the budget demotions with their paths [D54] and the consumer-side descriptor check |
//! | `.oracle-refusals.json` | `warrant-data-faults-refusals-v1` | 16 refused signatures and the 11 `typing` rows of R67, R94, R96 to R98 |
//!
//! The files are read from the oracle's directory (no copy), so an edit to any of them reruns this test (the
//! build tool's repository-input row `rust/warrant-corpus-gen`, `warrantDataEvalFixture`).  The dumps carry no
//! generator name or version.  The negative controls at the bottom show each comparison can fail.

mod support;

use support::minijson::{parse, J};
use support::parity::*;
use support::*;

const WARRANT: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.warrant"
);
const PROFILE: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.profile"
);
const LOWERING: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-lowering.json"
);
const EVALUATION: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-evaluation.json"
);
const FAULTS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-faults.json"
);

/// The specification's path as written into `corpus.json`.
const SPEC: &str = "data_eval_v1.warrant";

/// The gate of the oracle before this slice (`parity_dumps.GATE_BEFORE_DATA`): every `ir.data` fault is
/// `deferred`.
const GATE_BEFORE_DATA: [Level; 3] = [Level::Core, Level::Fixed, Level::Tuple];
/// The gate this slice opens (`parity_dumps.GATE_DATA`).
const GATE_DATA: [Level; 4] = [Level::Core, Level::Data, Level::Fixed, Level::Tuple];

use coding_adventures_warrant_corpus_gen::domains::{claim_domains, describe, descriptor_size, resolve_claim_in};
use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::fault_types::{fault_unify, FaultTyper};
use coding_adventures_warrant_corpus_gen::faults::{instantiate_with, Level};
use coding_adventures_warrant_corpus_gen::ir::{Arith, Ir, Pat};
use coding_adventures_warrant_corpus_gen::json::Json;
use coding_adventures_warrant_corpus_gen::lower::{lower_document, Lowered};
use coding_adventures_warrant_corpus_gen::profile::parse_profile;
use coding_adventures_warrant_corpus_gen::stream::encode_in;
use coding_adventures_warrant_corpus_gen::tfv::{value_json, value_tfv};
use coding_adventures_warrant_corpus_gen::types::{Decls, Ty, Width};
use coding_adventures_warrant_corpus_gen::value::Value;
use coding_adventures_warrant_corpus_gen::view::oracle_view_v1;
use coding_adventures_warrant_corpus_gen::CorpusV2;
use coding_adventures_warrant_grammar::domain::{Domain, DomainEnv};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use std::rc::Rc;
use std::sync::OnceLock;

const VALUES: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-values.json"
);
const DOMAINS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-domains.json"
);
const REFUSALS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-refusals.json"
);

/// The bound the oracle's `dump-lowering` costs under by default (`MAX_IR_COST`).
const LIMIT: u64 = 2_000;

// ---- the generated corpora ---------------------------------------------------------------------------

fn before() -> &'static CorpusV2 {
    static C: OnceLock<CorpusV2> = OnceLock::new();
    C.get_or_init(|| generated(WARRANT, SPEC, PROFILE, &GATE_BEFORE_DATA).expect("generates (before ir.data)"))
}

fn opened() -> &'static CorpusV2 {
    static C: OnceLock<CorpusV2> = OnceLock::new();
    C.get_or_init(|| generated(WARRANT, SPEC, PROFILE, &GATE_DATA).expect("generates (ir.data open)"))
}

// ---- the lowering dump ---------------------------------------------------------------------------------

fn lowering_text(src: &str) -> String {
    with_file(src, |f| {
        let l = lower_document(f).expect("lowers");
        oracle_view_v1(&l, LIMIT).expect("view").render().expect("render")
    })
}

#[test]
fn the_lowering_dump_is_reproduced_with_its_types_rows() {
    let text = lowering_text(WARRANT);
    assert!(text == LOWERING, "{}", first_difference(&text, LOWERING));
}

// ---- the evaluation dump -----------------------------------------------------------------------------

#[test]
fn the_evaluation_dump_is_reproduced() {
    let text = evaluation_view(opened()).render().expect("render");
    assert!(text == EVALUATION, "{}", first_difference(&text, EVALUATION));
}

// ---- the fault dump ----------------------------------------------------------------------------------

fn faults_view(closed: &CorpusV2, open: &CorpusV2) -> Json {
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
        ("gate_shipped", Json::strs(GATE_BEFORE_DATA.iter().map(|l| l.name()))),
        ("gate_on", Json::strs(GATE_DATA.iter().map(|l| l.name()))),
    ])
}

#[test]
fn the_fault_dump_is_reproduced_under_both_gates() {
    let text = faults_view(before(), opened()).render().expect("render");
    assert!(text == FAULTS, "{}", first_difference(&text, FAULTS));
}

// ---- types and values ------------------------------------------------------------------------------------

/// Run `f` on a document lowered and its declarations (`Decls` is reference counted: no `static`).
fn with_decls<R>(src: &str, f: impl FnOnce(&Lowered, &Decls) -> R) -> R {
    with_file(src, |file| {
        let l = lower_document(file).expect("lowers");
        let d = Decls::new(&l.types);
        f(&l, &d)
    })
}

/// A type from its canonical spelling (VER04r 2.1), the declared names being `names`.
fn ty(text: &str) -> Ty {
    fn go(s: &[u8], i: &mut usize) -> Ty {
        if s[*i] == b'(' {
            *i += 1;
            let mut parts = vec![go(s, i)];
            while s[*i] == b',' {
                *i += 1;
                parts.push(go(s, i));
            }
            assert_eq!(s[*i], b')');
            *i += 1;
            return Ty::Tuple(parts);
        }
        let start = *i;
        while *i < s.len() && (s[*i].is_ascii_alphanumeric() || s[*i] == b'_') {
            *i += 1;
        }
        let head = std::str::from_utf8(&s[start..*i]).unwrap().to_string();
        if *i < s.len() && s[*i] == b'<' {
            *i += 1;
            if head == "Bytes" {
                let w = *i;
                while s[*i].is_ascii_digit() {
                    *i += 1;
                }
                let n: u64 = std::str::from_utf8(&s[w..*i]).unwrap().parse().unwrap();
                assert_eq!(s[*i], b'>');
                *i += 1;
                return Ty::bytes_n(n);
            }
            let mut args = vec![go(s, i)];
            while s[*i] == b',' {
                *i += 1;
                args.push(go(s, i));
            }
            assert_eq!(s[*i], b'>');
            *i += 1;
            return match head.as_str() {
                "Option" => Ty::Option(Box::new(args.remove(0))),
                "Result" => {
                    let Ty::Named(e) = args.remove(1) else { panic!("an error name") };
                    Ty::Result(Box::new(args.remove(0)), e)
                }
                other => panic!("{other}"),
            };
        }
        match head.as_str() {
            "Bool" => Ty::Bool,
            "Nat" => Ty::Nat,
            "Int" => Ty::Int,
            "Bytes" => Ty::BYTES,
            n => match FixedTy::from_name(n) {
                Some(t) => Ty::Fixed(t),
                None => Ty::Named(n.to_string()),
            },
        }
    }
    let mut i = 0;
    let t = go(text.as_bytes(), &mut i);
    assert_eq!(i, text.len(), "{text}");
    t
}

fn int(n: i128) -> Value {
    Value::Int(n)
}
fn none() -> Value {
    Value::Opt(None)
}
fn some(v: Value) -> Value {
    Value::Opt(Some(Box::new(v)))
}
fn tup(v: Vec<Value>) -> Value {
    Value::Tuple(v)
}
fn con(name: &str, v: Vec<Value>) -> Value {
    Value::Con(Rc::from(name), v)
}
fn bytes(b: &[u8]) -> Value {
    Value::bytes(b)
}
fn ok(v: Value) -> Value {
    Value::Ok(Box::new(v))
}
fn t() -> Value {
    Value::Bool(true)
}
fn f() -> Value {
    Value::Bool(false)
}

/// The 28 values of the oracle's table (`eval_dumps.DATA_VALUES`): a record is its constructor named like it.
fn data_values() -> Vec<(&'static str, Value)> {
    vec![
        ("Option<U8>", none()),
        ("Option<U8>", some(int(7))),
        ("Option<Bool>", some(f())),
        ("Option<Option<Bool>>", some(none())),
        ("Option<Option<Bool>>", some(some(t()))),
        ("Option<(Bool,Nat)>", some(tup(vec![t(), int(4)]))),
        ("Option<Bytes>", some(bytes(&[1, 255]))),
        ("Point", con("Point", vec![int(1), int(2)])),
        ("Point", con("Point", vec![int(255), int(0)])),
        ("Wrapped", con("Wrapped", vec![con("Point", vec![int(1), int(2)]), t()])),
        ("Slot", con("Slot", vec![some(int(9)), tup(vec![t(), int(2)])])),
        ("Slot", con("Slot", vec![none(), tup(vec![f(), int(0)])])),
        ("Wide", con("Wide", vec![int(65535), int(3)])),
        ("Sum", con("Sum", vec![bytes(&[1]), int(3), t()])),
        ("Holder", con("Holder", vec![bytes(&[0, 255]), tup(vec![bytes(&[255]), t()])])),
        ("Shape", con("Circle", vec![int(3)])),
        ("Shape", con("Rect", vec![int(3), int(4)])),
        ("Shape", con("Dot", vec![])),
        ("Dir", con("Both", vec![t(), f()])),
        ("Msg", con("Short", vec![])),
        ("Msg", con("Done", vec![int(5), none()])),
        ("Msg", con("Done", vec![int(5), some(int(65535))])),
        ("Msg", con("Opt", vec![some(int(1))])),
        ("Result<Sum,DataError>", ok(con("Sum", vec![bytes(b""), int(0), f()]))),
        ("Result<Sum,DataError>", Value::Err(Rc::from("Bad"))),
        ("Result<Option<U8>,DataError>", ok(none())),
        ("Result<Option<U8>,DataError>", ok(some(int(7)))),
        ("Result<Dir,DataError>", ok(con("Down", vec![t()]))),
    ]
}

/// The JSON a consumer must refuse (`eval_dumps.DATA_REJECTED`), as text.
const REJECTED_JSON: [(&str, &str); 23] = [
    ("Option<U8>", r#"{"none": false}"#),
    ("Option<U8>", r#"{}"#),
    ("Option<U8>", r#"{"none": true, "some": "1"}"#),
    ("Option<U8>", r#"{"some": 1}"#),
    ("Option<U8>", r#"{"some": "256"}"#),
    ("Option<U8>", "null"),
    ("Option<U8>", r#""none""#),
    ("Point", r#"["1"]"#),
    ("Point", r#"["1", "2", "3"]"#),
    ("Point", r#"{"x": "1", "y": "2"}"#),
    ("Point", r#"["1", "256"]"#),
    ("Wrapped", r#"[["1", "2"], 1]"#),
    ("Shape", r#"{"ctor": "Circle"}"#),
    ("Shape", r#"{"ctor": "Circle", "fields": ["3"], "extra": 1}"#),
    ("Shape", r#"{"ctor": "Square", "fields": ["3"]}"#),
    ("Shape", r#"{"ctor": "Circle", "fields": []}"#),
    ("Shape", r#"{"ctor": "Circle", "fields": ["3", "4"]}"#),
    ("Shape", r#"{"ctor": "Circle", "fields": [3]}"#),
    ("Shape", r#"{"ctor": "Dot", "fields": ["0"]}"#),
    ("Shape", r#"["Circle", ["3"]]"#),
    ("Msg", r#"{"ctor": "Done", "fields": ["5", {"some": "65536"}]}"#),
    ("Result<Sum,DataError>", r#"{"ok": ["", "0"]}"#),
    ("Result<Sum,DataError>", r#"{"err": "Missing"}"#),
];

/// A consumer's reading of the JSON of a value of type `ty` (VER04r 2.3; the oracle's kit reader
/// `conformance.decode_value`): the reader's message when the value does not fit.  This is the consumer side
/// the dump pins; the generator's writer ([`value_json`]) is the producer side.
fn decode(raw: &J, ty: &Ty, decls: &Decls, errors: &[(String, Vec<String>)]) -> Result<(), String> {
    let refuse = |why: &str| Err(why.to_string());
    match ty {
        Ty::Bool => match raw {
            J::B(_) => Ok(()),
            _ => refuse("a Bool is a JSON boolean"),
        },
        Ty::Nat | Ty::Fixed(_) => {
            let J::S(text) = raw else {
                return refuse("an integer value is decimal text");
            };
            let signed = ty.fixed().is_some_and(|f| f.signed());
            let body = text.strip_prefix('-').unwrap_or(text);
            if text.starts_with('-') && !signed {
                return refuse("a negative value for an unsigned type");
            }
            if body.is_empty() || !body.bytes().all(|c| c.is_ascii_digit()) || body.len() > 20 {
                return refuse("malformed integer text");
            }
            if body.len() > 1 && body.starts_with('0') {
                return refuse("a leading zero in an integer");
            }
            if text == "-0" {
                return refuse("negative zero");
            }
            let v: i128 = text.parse().unwrap();
            match ty.fixed() {
                Some(t) if v < t.min() || v > t.max() => refuse("an integer value is out of range for its type"),
                None if !(0..1i128 << 64).contains(&v) => refuse("a Nat value is in 0..2^64-1"),
                _ => Ok(()),
            }
        }
        Ty::Bytes(_) => match raw {
            J::S(h) if h.len() % 2 == 0 && h.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) => Ok(()),
            _ => refuse("Bytes is lowercase even-length hex"),
        },
        Ty::Result(ok, err) => {
            let J::O(pairs) = raw else {
                return refuse("a Result has exactly one key");
            };
            if pairs.len() != 1 {
                return refuse("a Result has exactly one key");
            }
            if raw.has("ok") {
                return decode(raw.get("ok"), ok, decls, errors);
            }
            let cases = errors.iter().find(|(n, _)| n == err).map(|(_, c)| c);
            match raw.get_opt("err") {
                Some(J::S(case)) if cases.is_none_or(|c| c.contains(case)) => Ok(()),
                _ => refuse("an err case is a name declared in the error type"),
            }
        }
        Ty::Tuple(parts) => decode_parts(raw, parts, decls, errors),
        Ty::Named(name) if decls.is_record(name) => {
            let fields = decls.fields(name, name).unwrap();
            decode_parts(raw, fields, decls, errors)
        }
        Ty::Option(inner) => match raw {
            J::O(p) if p.len() == 1 && p[0].0 == "none" && p[0].1 == J::B(true) => Ok(()),
            J::O(p) if p.len() == 1 && p[0].0 == "some" => decode(&p[0].1, inner, decls, errors),
            _ => refuse("an Option is {none: true} or {some: v}"),
        },
        Ty::Named(name) => {
            let J::O(p) = raw else {
                return refuse("a variant has exactly the keys ctor and fields");
            };
            let mut keys: Vec<&str> = p.iter().map(|(k, _)| k.as_str()).collect();
            keys.sort_unstable();
            if keys != ["ctor", "fields"] {
                return refuse("a variant has exactly the keys ctor and fields");
            }
            let (J::S(ctor), J::A(items)) = (raw.get("ctor"), raw.get("fields")) else {
                return refuse("an undeclared constructor");
            };
            let Some(declared) = decls.fields(name, ctor) else {
                return refuse("an undeclared constructor");
            };
            if declared.len() != items.len() {
                return refuse("a constructor's field count");
            }
            items
                .iter()
                .zip(declared)
                .try_for_each(|(x, t)| decode(x, t, decls, errors))
        }
        _ => refuse("a value of this type cannot be decoded"),
    }
}

fn decode_parts(raw: &J, parts: &[Ty], decls: &Decls, errors: &[(String, Vec<String>)]) -> Result<(), String> {
    match raw {
        J::A(items) if items.len() == parts.len() => items
            .iter()
            .zip(parts)
            .try_for_each(|(x, t)| decode(x, t, decls, errors)),
        _ => Err("a tuple or record is an array of its arity".to_string()),
    }
}

/// The oracle's `\"json\": null` has no form in this crate's [`Json`] (a v2 corpus holds no null): a marker the
/// rendered text swaps for the token.
const NULL_MARK: &str = "@@null@@";

fn raw_json(text: &str) -> Json {
    if text == "null" {
        return Json::str(NULL_MARK);
    }
    to_json(&parse(text).expect("parses"))
}

fn values_view() -> String {
    with_decls(WARRANT, |l, d| {
        let errors: Vec<(String, Vec<String>)> = l.errors.iter().map(|e| (e.name.clone(), e.cases.clone())).collect();
        let mut rows = Vec::new();
        for (text, v) in data_values() {
            let t = ty(text);
            let json = value_json(d, &t, &v).expect("json");
            // the consumer's reader (an independent reading of 2.3) reads the producer's JSON back
            let reread = parse(&json.render().unwrap().replace('\n', " ")).expect("parses");
            assert_eq!(decode(&reread, &t, d, &errors), Ok(()), "{text}");
            let mut stream = Vec::new();
            encode_in(d, &t, &v, &mut stream).expect("stream");
            rows.push(Json::obj([
                ("type", Json::str(text)),
                ("json", json),
                ("stream", Json::str(stream.iter().map(|b| format!("{b:02x}")).collect::<String>())),
                ("tfv", Json::str(value_tfv(d, &t, &v).expect("tfv"))),
            ]));
        }
        let mut rejected = Vec::new();
        for (text, raw) in REJECTED_JSON {
            let parsed = if raw == "null" { J::Null } else { parse(raw).expect("parses") };
            let why = decode(&parsed, &ty(text), d, &errors).expect_err(raw);
            rejected.push(Json::obj([
                ("type", Json::str(text)),
                ("json", raw_json(raw)),
                ("oracle_code", Json::str("value")),
                ("message", Json::str(why)),
            ]));
        }
        Json::obj([
            ("schema", Json::str("warrant-data-values-parity-v1")),
            (
                "format",
                Json::str(
                    "type, json (section 2.3), stream (section 2.4, hex), tfv (section 2.5); a record is an array, an Option \
                     {none:true} or {some:v}, a variant {ctor, fields}; stream 40/41 Option, 64 record, 65 variant",
                ),
            ),
            ("values", Json::Arr(rows)),
            ("rejected", Json::Arr(rejected)),
        ])
        .render()
        .expect("render")
        .replace(&format!("\"{NULL_MARK}\""), "null")
    })
}

#[test]
fn the_values_dump_is_reproduced() {
    let text = values_view();
    assert!(text == VALUES, "{}", first_difference(&text, VALUES));
}

// ---- the domains dump ------------------------------------------------------------------------------------

const DATA_DOMAINS: [&str; 12] = [
    "Option<Bool>",
    "Option<Option<Bool>>",
    "Option<(Bool,Nat)>",
    "Duo",
    "Tri",
    "Dir",
    "Mode",
    "Shape",
    "Point",
    "Slot",
    "Result<Option<Bool>,DataError>",
    "Result<Dir,DataError>",
];

const DATA_DEMOTIONS: [&[&str]; 6] = [
    &["Wide"],
    &["Msg"],
    &["Option<U16>", "Nat"],
    &["Nat", "U16"],
    &["Result<Wide,DataError>"],
    &["Dir", "Slot"],
];

/// The document with one extra claim per row of the domain tables: the S2 resolver reads binder types from a
/// claim, so a table row is a claim `forall x: T. x == x and op_mk_dot(true) == Dot` (a claim must call an operation).
fn domain_doc() -> String {
    let mut src = WARRANT.to_string();
    for (i, text) in DATA_DOMAINS.iter().enumerate() {
        src.push_str(&format!("  claim dd_leaf_{i}:\n    forall x: {text}. x == x and op_mk_dot(true) == Dot\n"));
    }
    for (i, row) in DATA_DEMOTIONS.iter().enumerate() {
        let binders: Vec<String> = row.iter().enumerate().map(|(j, t)| format!("b{j}: {t}")).collect();
        src.push_str(&format!("  claim dd_demote_{i}:\n    forall {}. b0 == b0 and op_mk_dot(true) == Dot\n", binders.join(", ")));
    }
    src
}

/// The domain before the budget demoted anything: a demoted 16-bit leaf (`FxEdge`) is its full `Fx` again.
fn undemoted(d: &Domain) -> Domain {
    match d {
        Domain::FxEdge(f) => Domain::Fx(*f),
        Domain::Tuple(parts) => Domain::Tuple(parts.iter().map(undemoted).collect()),
        Domain::Opt(inner) => Domain::Opt(Box::new(undemoted(inner))),
        Domain::Res(inner, errs, name) => Domain::Res(Box::new(undemoted(inner)), errs.clone(), name.clone()),
        Domain::Data { name, ctors, record } => Domain::Data {
            name: name.clone(),
            ctors: ctors
                .iter()
                .map(|(c, fields)| (c.clone(), fields.iter().map(undemoted).collect()))
                .collect(),
            record: *record,
        },
        other => other.clone(),
    }
}

fn value_of(d: &Decls, t: &Ty, dom: &Domain, i: u64) -> Json {
    let v = Value::from_eval(&dom.nth(i).expect("in range")).expect("converts");
    value_json(d, t, &v).expect("json")
}

fn values_of(d: &Decls, t: &Ty, dom: &Domain, idx: impl IntoIterator<Item = u64>) -> Json {
    Json::arr(idx.into_iter().map(|i| value_of(d, t, dom, i)))
}

fn domains_view() -> String {
    let src = domain_doc();
    with_file(&src, |file| {
        let l = lower_document(file).expect("lowers");
        let d = Decls::new(&l.types);
        let env = DomainEnv::new(file);
        let budget = parse_profile(PROFILE.as_bytes()).expect("profile").budget;
        let mut leaves = Vec::new();
        for (i, text) in DATA_DOMAINS.iter().enumerate() {
            let (doms, _) = claim_domains(file, &env, &format!("dd_leaf_{i}"), u64::MAX).expect("resolves");
            let (descriptor, label, size) = describe(&doms[0], 1).expect("describes");
            let t = ty(text);
            let mut row = vec![
                ("descriptor", descriptor),
                ("label", Json::str(label.name())),
                ("size", Json::Int(size)),
            ];
            if size <= 16 {
                row.push(("values", values_of(&d, &t, &doms[0], 0..size)));
            } else {
                row.push(("head", values_of(&d, &t, &doms[0], 0..4)));
                row.push(("tail", values_of(&d, &t, &doms[0], size - 4..size)));
            }
            leaves.push((text.to_string(), Json::obj(row)));
        }
        let mut demotions = Vec::new();
        for (i, row) in DATA_DEMOTIONS.iter().enumerate() {
            let claim = format!("dd_demote_{i}");
            let (before, _) = claim_domains(file, &env, &claim, u64::MAX).expect("resolves");
            let suite = l.suites.iter().find(|s| s.claim == claim).expect("suite");
            let r = resolve_claim_in(file, &env, suite, budget, &d).expect("demotes");
            demotions.push(Json::obj([
                ("binders", Json::strs(row.iter().copied())),
                ("budget", Json::Int(budget)),
                (
                    "sizes_before",
                    Json::arr(before.iter().map(|b| Json::Int(undemoted(b).domain_size().expect("size")))),
                ),
                ("demoted", Json::strs(r.demoted.iter())),
                ("descriptors", Json::arr(r.binders.iter().map(|b| b.descriptor.clone()))),
                ("sizes", Json::arr(r.binders.iter().map(|b| Json::Int(b.size)))),
            ]));
        }
        Json::obj([
            ("schema", Json::str("warrant-data-domain-parity-v1")),
            ("domains", Json::Obj(leaves)),
            ("demotions", Json::Arr(demotions)),
            ("consumer", Json::arr(consumer_descriptors().iter().map(verdict))),
        ])
        .render()
        .expect("render")
    })
}

fn node(kind: &str, size: u64, extra: Vec<(&str, Json)>) -> Json {
    let mut row = vec![("d", Json::str(kind)), ("size", Json::Int(size))];
    row.extend(extra);
    Json::obj(row)
}

fn boolean() -> Json {
    node("bool", 2, vec![])
}

fn option(of: Json, size: u64) -> Json {
    node("option", size, vec![("of", of)])
}

fn variant(ctors: Vec<(&str, Vec<Json>)>, size: u64) -> Json {
    node(
        "variant",
        size,
        vec![(
            "ctors",
            Json::arr(ctors.into_iter().map(|(n, f)| Json::arr([Json::str(n), Json::Arr(f)]))),
        )],
    )
}

/// The descriptors a consumer must accept, then reject (`eval_dumps.DATA_ACCEPTED`, `DATA_REJECTED_DESCRIPTORS`).
fn consumer_descriptors() -> Vec<Json> {
    let u8_all = || node("u8_all", 256, vec![]);
    let nat = || node("nat_0_64", 65, vec![]);
    let edge16 = || node("fx_edge", 281, vec![("t", Json::str("U16"))]);
    let dir = || {
        variant(
            vec![("Up", vec![]), ("Down", vec![boolean()]), ("Both", vec![boolean(), boolean()])],
            7,
        )
    };
    let accepted = vec![
        option(boolean(), 3),
        node("tuple", 65536, vec![("of", Json::arr([u8_all(), u8_all()]))]),
        dir(),
        variant(
            vec![("Short", vec![]), ("Done", vec![nat(), option(edge16(), 282)])],
            18331,
        ),
        node(
            "result",
            5,
            vec![("ok", option(boolean(), 3)), ("errs", Json::strs(["Bad", "Worse"]))],
        ),
    ];
    let rejected = vec![
        option(boolean(), 2),
        node("option", 3, vec![]),
        node("option", 3, vec![("of", boolean()), ("extra", Json::Int(1))]),
        node("tuple", 1, vec![("of", Json::Arr(vec![]))]),
        node("tuple", 3, vec![("of", Json::arr([boolean()]))]),
        node("variant", 0, vec![("ctors", Json::Arr(vec![]))]),
        variant(vec![("Up", vec![]), ("Down", vec![boolean()])], 2),
        node(
            "variant",
            2,
            vec![(
                "ctors",
                Json::arr([
                    Json::arr([Json::str("Up"), Json::Arr(vec![])]),
                    Json::arr([Json::str("Down")]),
                ]),
            )],
        ),
        variant(
            vec![
                ("Up", vec![]),
                ("Down", vec![node("fx_edge", 256, vec![("t", Json::str("U8"))])]),
            ],
            257,
        ),
        node("variant", 1, vec![("of", Json::Arr(vec![]))]),
        node("result", 2, vec![("ok", boolean()), ("errs", Json::strs(["Bad"]))]),
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
            ("oracle_code", Json::str(if e.code == code::LIMIT { "limit" } else { "generation" })),
            ("message", Json::str(e.message)),
        ]),
    }
}

#[test]
fn the_domains_dump_is_reproduced_with_its_demotion_paths() {
    let text = domains_view();
    assert!(text == DOMAINS, "{}", first_difference(&text, DOMAINS));
}

// ---- the refusals dump -----------------------------------------------------------------------------------

/// The declarations every refusal document starts from (`make_data_eval._BASE`).
const BASE: &str = "warrant refusals v1\n  error E = A | B\n  record Point(x: U8, y: U8)\n  record NoNum(a: Bool, b: Bytes<2>)\n  \
record Wide(w: U16, n: Nat)\n  record Holder(d: Bytes, pair: (Bytes,Bool))\n  record WithDir(a: U8, d: Dir)\n  \
variant Dir = Up | Down(f: Bool)\n";

/// `(name, fault, signature line)` of `make_data_eval.FAULT_REFUSALS`.
const FAULT_REFUSALS: [(&str, &str, &str); 16] = [
    ("field-zero/a-scalar-result", "field-zero", "  op f(a: U8) -> U8\n"),
    ("field-zero/a-bytes-result", "field-zero", "  op f(a: U8) -> Bytes\n"),
    ("field-zero/a-tuple-result", "field-zero", "  op f(a: U8) -> (U8,Nat)\n"),
    ("field-zero/a-variant-result", "field-zero", "  op f(a: U8) -> Dir\n"),
    ("field-zero/a-record-with-no-numeric-field", "field-zero", "  op f(a: U8) -> NoNum\n"),
    ("field-zero/an-ok-record-with-no-numeric-field", "field-zero", "  op f(a: U8) -> Result<NoNum,E>\n"),
    ("field-zero/an-option-of-a-record", "field-zero", "  op f(a: U8) -> Option<Point>\n"),
    ("field-zero/a-result-of-a-scalar", "field-zero", "  op f(a: U8) -> Result<U8,E>\n"),
    ("field-zero/a-result-of-a-variant", "field-zero", "  op f(a: U8) -> Result<Dir,E>\n"),
    ("field-zero/a-result-of-an-option-of-a-record", "field-zero", "  op f(a: U8) -> Result<Option<Point>,E>\n"),
    (
        "field-zero/a-parameter-named-ok-payload-in-the-ok-form",
        "field-zero",
        "  op f(ok_payload: U8) -> Result<Point,E>\n",
    ),
    (
        "missing-error/a-variant-with-no-parameter-of-its-type",
        "missing-error",
        "  op f(a: U8) -> Result<Dir,E>\n",
    ),
    (
        "missing-error/a-record-field-that-is-a-variant-with-no-parameter",
        "missing-error",
        "  op f(a: U8) -> Result<WithDir,E>\n",
    ),
    (
        "missing-error/a-result-payload-that-is-a-result",
        "missing-error",
        "  op f(a: U8) -> Result<Result<U8,E>,E>\n",
    ),
    ("missing-error/a-non-result-result", "missing-error", "  op f(a: U8) -> Point\n"),
    ("wrong-ctor/is-not-in-the-registry", "wrong-ctor", "  op f(a: U8) -> Result<Dir,E>\n"),
];

/// The contract boilerplate for operation `f`, so the checker reaches the lowering.
fn contracts(params: &[String]) -> String {
    let mut c = String::from("  assume platform:\n    none\n  assume environment:\n    none\n");
    if !params.is_empty() {
        c.push_str("  labels f:\n");
        for p in params {
            c.push_str(&format!("    {p} secret\n"));
        }
        for p in params {
            c.push_str(&format!("  input f.{p}:\n    hostile\n    ensures no_panic, terminates\n"));
        }
    }
    c.push_str("  bound f:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    c
}

fn param_names(source: &str) -> Vec<String> {
    let sig = source.split("op f").nth(1).expect("an operation f");
    let sig = &sig[sig.find('(').expect("(") + 1..];
    let sig = &sig[..sig.find(')').expect(")")];
    sig.split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.split(':').next().expect("name").trim().to_string())
        .collect()
}

/// `f` of a refusal document lowered, with the document's declarations.
fn with_op<R>(
    source: &str,
    run: impl FnOnce(&coding_adventures_warrant_corpus_gen::lower::LoweredOp, &Lowered, &Decls) -> R,
) -> R {
    let src = format!("{source}{}", contracts(&param_names(source)));
    with_decls(&src, |l, d| run(l.op("f").expect("op f"), l, d))
}

fn refusals_view() -> String {
    let mut rows = Vec::new();
    for (name, fault, line) in FAULT_REFUSALS {
        let source = format!("{BASE}{line}");
        let (c, message) = with_op(&source, |op, l, d| {
            let e = instantiate_with(op, fault, &l.errors, d).expect_err(name);
            (e.code, e.message)
        });
        assert_eq!(c, code::FAULT, "{name}");
        let why = message
            .strip_prefix("the fault cannot be instantiated on this operation: ")
            .unwrap_or_else(|| panic!("{name}: {message}"));
        rows.push(Json::obj([
            ("name", Json::str(name)),
            ("fault", Json::str(fault)),
            ("oracle_code", Json::str("generation")),
            ("message", Json::str(format!("fault: {why}"))),
            ("source", Json::str(source)),
        ]));
    }
    let mut typing = Vec::new();
    for (name, line, expr, _accepted) in typing_rows() {
        let source = format!("{BASE}  record Pair2(p: (Bytes<2>,Bool))\n{line}");
        let outcome = with_op(&source, |op, _, d| {
            let mut env: Vec<(String, Ty)> = op.params.clone();
            env.push(("result".to_string(), op.ret.clone()));
            let typer = FaultTyper { decls: d, op: &op.name, branch_check: true };
            typer
                .typed(&expr, &env, Some(&op.ret), "expression")
                .map(|got| fault_unify(&got, &op.ret).expect("a common type").name())
        });
        let mut row = vec![("name", Json::str(name)), ("source", Json::str(source)), ("expr", expr.to_json())];
        match outcome {
            Ok(shown) => {
                row.push(("accepted", Json::Bool(true)));
                row.push(("type", Json::str(shown)));
            }
            Err(e) => {
                // the oracle's `generation` is this crate's `fault`, its `ir` this crate's `ir`; a refusal of the
                // R94 comparison carries the oracle's `fault: ` prefix
                let (c, m) = if e.code == code::FAULT {
                    ("generation", format!("fault: {}", e.message))
                } else {
                    assert_eq!(e.code, code::IR, "{name}");
                    ("ir", e.message)
                };
                row.push(("oracle_code", Json::str(c)));
                row.push(("message", Json::str(m)));
            }
        }
        typing.push(Json::obj(row));
    }
    Json::obj([
        ("schema", Json::str("warrant-data-faults-refusals-v1")),
        ("refusals", Json::Arr(rows)),
        ("typing", Json::Arr(typing)),
    ])
    .render()
    .expect("render")
}

fn lit_bytes(hex: &str) -> Ir {
    Ir::Bytes(hex.to_string())
}

fn lit_nat(n: i128) -> Ir {
    Ir::Int { ty: Ty::Nat, value: n }
}

fn result_var() -> Ir {
    Ir::Var("result".to_string())
}

/// `if is_err(result) then ok(payload) else result`
fn if_ok(payload: Ir) -> Ir {
    Ir::If(
        Box::new(Ir::IsErr(Box::new(result_var()))),
        Box::new(Ir::Ok(Box::new(payload))),
        Box::new(result_var()),
    )
}

/// `(name, signature line, expression, accepted)` of `make_data_eval.TYPING`.
fn typing_rows() -> Vec<(&'static str, &'static str, Ir, bool)> {
    let pair = |hex: &str| Ir::Tuple(vec![lit_bytes(hex), Ir::Bool(false)]);
    vec![
        (
            "r67/a-tuple-payload-with-a-plain-bytes-zero",
            "  op f(a: Nat) -> Result<(Bytes,Bool),E>\n",
            if_ok(pair("")),
            true,
        ),
        (
            "r67/a-one-byte-literal-in-a-bytes-2-component",
            "  op f(a: Nat) -> Result<(Bytes<2>,Bool),E>\n",
            if_ok(pair("01")),
            false,
        ),
        (
            "r67/a-two-byte-literal-in-a-bytes-2-component",
            "  op f(a: Nat) -> Result<(Bytes<2>,Bool),E>\n",
            if_ok(pair("0102")),
            true,
        ),
        (
            "r67/a-record-field-that-is-a-tuple-with-a-plain-bytes-zero",
            "  op f(a: Nat) -> Result<Holder,E>\n",
            if_ok(Ir::Ctor { name: "Holder".into(), args: vec![lit_bytes(""), pair("")] }),
            true,
        ),
        (
            "r67/a-one-byte-literal-in-a-record-field-of-bytes-2",
            "  op f(a: Nat) -> Result<Pair2,E>\n",
            if_ok(Ir::Ctor { name: "Pair2".into(), args: vec![pair("01")] }),
            false,
        ),
        (
            "r94/plain-bytes-against-a-declared-bytes-2",
            "  op f(a: Nat) -> Bytes<2>\n",
            Ir::Take(Box::new(result_var()), Box::new(lit_nat(1))),
            true,
        ),
        (
            "r94/a-bool-against-a-declared-bytes-result",
            "  op f(a: Nat) -> Bytes\n",
            Ir::Bool(true),
            false,
        ),
        (
            "r94/a-bytes-against-a-declared-u8-result",
            "  op f(a: Nat) -> U8\n",
            lit_bytes("00"),
            false,
        ),
        (
            "r94/a-bool-ok-in-the-else-branch-of-a-u8-result",
            "  op f(a: Nat) -> Result<U8,E>\n",
            Ir::If(
                Box::new(Ir::IsErr(Box::new(result_var()))),
                Box::new(result_var()),
                Box::new(Ir::Ok(Box::new(Ir::Bool(true)))),
            ),
            false,
        ),
        (
            "r94/a-bool-ok-in-the-second-match-arm-of-a-u8-result",
            "  op f(a: Nat) -> Result<U8,E>\n",
            Ir::Match {
                scrutinee: Box::new(result_var()),
                arms: vec![
                    (Pat::Ok(Box::new(Pat::Bind("p".into()))), Ir::Ok(Box::new(Ir::Var("p".into())))),
                    (Pat::ErrAny, Ir::Ok(Box::new(Ir::Bool(true)))),
                ],
                on_result: true,
            },
            false,
        ),
        (
            "r94/an-int-expression-against-a-declared-nat-result",
            "  op f(a: Nat) -> Nat\n",
            Ir::Arith(Arith::Add, Ty::Int, Box::new(result_var()), Box::new(lit_nat(1))),
            true,
        ),
    ]
}

#[test]
fn the_refusals_dump_is_reproduced_typing_rows_included() {
    let text = refusals_view();
    assert!(text == REFUSALS, "{}", first_difference(&text, REFUSALS));
}

// ---- the dumps are what they say ------------------------------------------------------------------------

#[test]
fn the_dumps_are_canonical_json_of_their_schemas() {
    for (name, d, schema) in [
        ("lowering", LOWERING, "warrant-lowering-parity-v1"),
        ("evaluation", EVALUATION, "warrant-evaluation-parity-v1"),
        ("faults", FAULTS, "warrant-fault-parity-v1"),
        ("values", VALUES, "warrant-data-values-parity-v1"),
        ("domains", DOMAINS, "warrant-data-domain-parity-v1"),
        ("refusals", REFUSALS, "warrant-data-faults-refusals-v1"),
    ] {
        assert!(!d.contains('\r'), "{name}");
        assert!(d.ends_with("}\n") && !d.ends_with("\n\n"), "{name}");
        assert!(d.is_ascii(), "{name}");
        assert!(d.contains(&format!("\"schema\": \"{schema}\"")), "{name}");
    }
}

#[test]
fn the_fixture_is_not_vacuous() {
    let dump = parse(EVALUATION).expect("parses");
    let J::O(pairs) = dump.get("suites") else { panic!("suites is an object") };
    assert_eq!(pairs.len(), 68);
    let (mut v1, mut v2) = (0, 0);
    for (_, s) in pairs {
        match s.get("stream").s() {
            "warrant-stream-v1" => v1 += 1,
            "warrant-stream-v2" => v2 += 1,
            other => panic!("{other}"),
        }
    }
    assert_eq!((v1, v2), (29, 39), "29 suites on the v1 header (Option<U8> included), 39 on v2");
    assert_eq!(dump.get("examples").a().len(), 23);
    // the three demotion paths of section 5.3 [D54] are in the dumps
    for path in ["\"1\"", "\"0.0\"", "\"0.Done.1.some\""] {
        assert!(EVALUATION.contains(path), "{path}");
    }
    assert_eq!(parse(FAULTS).expect("parses").get("faults").a().len(), 12);
}

#[test]
fn the_open_gate_witnesses_what_the_gate_before_it_defers() {
    let (b, o) = (&before().corpus_json, &opened().corpus_json);
    assert!(b.contains(r#""deferred": "ir.data""#));
    assert!(!o.contains(r#""deferred": "ir.data""#));
    // all three states under the open gate: witnessed, unproven (a computed argument), blind (an Err-only claim)
    assert!(o.contains(r#""chunk": 0"#) && o.contains(r#""unproven": true"#));
    assert!(o.contains("c_adder_err_only") && o.contains(r#""blind_claims""#));
    // nothing but the controls differ: no suite, chunk or example moves with the gate
    assert_eq!(before().chunk_files, opened().chunk_files);
    assert_eq!(before().cases_tfv, opened().cases_tfv);
}

// ---- negative controls -----------------------------------------------------------------------------------

#[test]
fn a_changed_record_changes_the_evaluation_dump() {
    // `Point(a, b)` built as `Point(b, a)`: the expected digest of c_mk_point moves, its inputs do not
    let mutated = WARRANT.replacen(
        "model mk_point(a: U8, b: U8) -> Point = Point(a, b)",
        "model mk_point(a: U8, b: U8) -> Point = Point(b, a)",
        1,
    );
    assert_ne!(mutated, WARRANT);
    let c = generated(&mutated, SPEC, PROFILE, &GATE_DATA).expect("generates");
    let text = evaluation_view(&c).render().unwrap();
    assert_ne!(text, EVALUATION);
    let (was, now) = (parse(EVALUATION).unwrap(), parse(&text).unwrap());
    let (b, a) = (was.get("suites").get("c_mk_point"), now.get("suites").get("c_mk_point"));
    assert_ne!(b.get("expected_sha256"), a.get("expected_sha256"));
    assert_eq!(b.get("inputs_sha256"), a.get("inputs_sha256"));
}

#[test]
fn a_changed_fault_line_changes_the_fault_dump() {
    let mutated = PROFILE.replacen("fault op_mk_point field-zero\n", "", 1);
    assert_ne!(mutated, PROFILE);
    let text = faults_view(
        &generated(WARRANT, SPEC, &mutated, &GATE_BEFORE_DATA).unwrap(),
        &generated(WARRANT, SPEC, &mutated, &GATE_DATA).unwrap(),
    )
    .render()
    .unwrap();
    assert_ne!(text, FAULTS);
    assert!(!text.contains("op_mk_point/field-zero") && FAULTS.contains("op_mk_point/field-zero"));
}

#[test]
fn a_wrong_value_writer_loses_values_parity() {
    let right = values_view();
    assert_eq!(right, VALUES);
    // the stream tag of an Option's `some` is 41 and the TFV of a variant `L2 S<ctor> L<k> ..`
    assert_ne!(right.replacen("\"stream\": \"4100\"", "\"stream\": \"4000\"", 1), VALUES);
    assert_ne!(right.replacen("L2 S4f7074", "L1 S4f7074", 1), VALUES);
}

#[test]
fn a_wrong_demotion_path_loses_domains_parity() {
    assert_ne!(domains_view().replacen("\"0.Done.1.some\"", "\"0.1.some\"", 1), DOMAINS);
}

#[test]
fn a_lenient_typer_loses_refusals_parity() {
    // a comparison that accepted the one-byte literal in a `Bytes<2>` component would move the row
    let right = refusals_view();
    assert_eq!(right, REFUSALS);
    assert_ne!(right.replacen("\"oracle_code\": \"generation\"", "\"accepted\": true", 1), REFUSALS);
}

#[test]
fn the_type_language_of_the_typing_rows_has_a_named_width() {
    assert_eq!(Ty::Bytes(Some(Width::Lit(2))).name(), "Bytes<2>");
    let e: Option<GenError> = None;
    assert!(e.is_none());
}

// ---- the oracle accepts a generator-written corpus (the end-to-end check of S11c-2) ------------------------

/// Write `c` under a fresh temporary directory and return it.
fn write_corpus(c: &CorpusV2) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wcg-data-eval-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("chunks")).expect("temp dir");
    for (path, text) in c.files() {
        std::fs::write(dir.join(path), text).expect("write");
    }
    dir
}

/// A path the oracle can compare with `--repo-root`: canonical, and without Windows' verbatim prefix.
///
/// On Windows `canonicalize` returns `\\?\D:\a\...`, while Python's `Path.resolve()` of a path that was
/// written `D:\a\...\..\..` returns `D:\a\...`.  Passing one of each makes the oracle's "is the `.warrant`
/// inside the repository root" check fail (`relative_to` sees two different drives), so every path the test
/// hands the oracle goes through this one function.  On Linux and macOS there is no prefix and it is a plain
/// `canonicalize`.  A path that does not exist (no oracle tree beside the crate) is returned as written, so
/// the test still reaches its "skipped" branch.
fn plain(path: &std::path::Path) -> std::path::PathBuf {
    let Ok(canon) = path.canonicalize() else {
        return path.to_path_buf();
    };
    match canon.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) => std::path::PathBuf::from(rest),
        None => canon,
    }
}

/// The independent oracle's `verify-v2` accepts the corpus this generator writes from the `data-eval-v1`
/// document: every digest, chunk line, descriptor, example (JSON and TFV) and control row of 68 suites and
/// 1,839,544 cases is re-derived by the second implementation and agrees.  The test needs `python3` and the
/// oracle's source tree beside this crate (the repository checkout); where either is missing it says so and
/// passes, because the byte parity above is the gate that cannot be skipped.  The same check by hand:
///
/// ```text
/// warrant-corpus-gen --warrant <fixture>/data_eval_v1.warrant --profile <fixture>/data_eval_v1.profile --out /tmp/c
/// (cd code/packages/python/warrant-oracle && PYTHONPATH=src python3 -m coding_adventures_warrant_oracle \
///    verify-v2 /tmp/c --warrant <fixture>/data_eval_v1.warrant --profile <fixture>/data_eval_v1.profile --repo-root <root>)
/// ```
#[test]
fn the_oracles_verify_v2_accepts_a_generator_written_corpus() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../../../..");
    let oracle = root.join("code/packages/python/warrant-oracle");
    let fixture = oracle.join("tests/fixtures/data-eval-v1");
    let spec = "code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.warrant";
    let corpus = generated(WARRANT, spec, PROFILE, &GATE_DATA).expect("generates");
    let dir = write_corpus(&corpus);
    let ran = std::process::Command::new("python3")
        .current_dir(&oracle)
        .env("PYTHONPATH", oracle.join("src"))
        .args(["-m", "coding_adventures_warrant_oracle", "verify-v2"])
        .arg(&dir)
        .arg("--warrant")
        .arg(plain(&fixture.join("data_eval_v1.warrant")))
        .arg("--profile")
        .arg(plain(&fixture.join("data_eval_v1.profile")))
        .arg("--repo-root")
        .arg(plain(&root))
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    match ran {
        Err(e) => eprintln!("skipped: python3 cannot run ({e}); the byte parity tests are the gate"),
        Ok(r) if !oracle.join("src").is_dir() => {
            let _ = r;
            eprintln!("skipped: no oracle source tree")
        }
        Ok(r) => {
            let out = String::from_utf8_lossy(&r.stdout).into_owned();
            let err = String::from_utf8_lossy(&r.stderr).into_owned();
            assert!(r.status.success(), "verify-v2 refused the corpus: {out} {err}");
            assert!(out.contains("ok: 1839544 cases across 68 suites"), "{out}");
        }
    }
}
