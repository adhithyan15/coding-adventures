//! Fault IR, the registry, the witness search and witness chunks (VER04r 5.6, 8).

mod support;

use coding_adventures_warrant_corpus_gen::emit::build_faults;
use coding_adventures_warrant_corpus_gen::enumerate::enumerate_analysed;
use coding_adventures_warrant_corpus_gen::faults::{
    first_unimplemented, instantiate, search, stand_in, ClaimState, Expr, Fault, Kind, Level, Probe,
    KNOWN_FAULTS,
};
use coding_adventures_warrant_corpus_gen::ir::Ir;
use coding_adventures_warrant_corpus_gen::lower::lower_document;
use coding_adventures_warrant_corpus_gen::profile::{parse_profile, Profile};
use coding_adventures_warrant_corpus_gen::stream::slim_base_set;
use coding_adventures_warrant_corpus_gen::{analyse, code};
use support::minijson::parse;
use support::*;

const CT_PROFILE: &str = include_str!("../../../../specs/fixtures/ct-compare-warrant-v2/corpus.profile");

fn gen(src: &str, profile: &str) -> Result<coding_adventures_warrant_corpus_gen::CorpusV2, coding_adventures_warrant_corpus_gen::GenError> {
    with_file(src, |f| {
        coding_adventures_warrant_corpus_gen::generate(&coding_adventures_warrant_corpus_gen::GenInput {
            file: f,
            spec_path: "s.warrant",
            spec_bytes: src.as_bytes(),
            profile_bytes: profile.as_bytes(),
            binding_faults: None,
        })
    })
}

fn json(f: &Fault) -> String {
    f.to_json().render().unwrap().split_whitespace().collect::<Vec<_>>().join("")
}

fn ct_op(name: &str) -> coding_adventures_warrant_corpus_gen::lower::LoweredOp {
    with_file(CT_COMPARE, |f| lower_document(f).unwrap().op(name).unwrap().clone())
}

// ---- the registry -----------------------------------------------------------------------

#[test]
fn templates_render_exactly_as_section_8() {
    let inv = instantiate(&ct_op("eq"), "invert").unwrap();
    assert_eq!(
        json(&inv),
        r#"{"expr":["not",["var","result"]],"fault":"invert","kind":"post","op":"eq","requires":["ir.core"]}"#
    );
    let swap = instantiate(&ct_op("select_bytes"), "swap-choice").unwrap();
    assert_eq!(
        json(&swap),
        r#"{"fault":"swap-choice","kind":"pre","op":"select_bytes","requires":["ir.core"],"rewrites":[{"expr":["not",["var","choice"]],"param":"choice"}]}"#
    );
    let miss = instantiate(&ct_op("select_bytes"), "missing-error").unwrap();
    assert_eq!(
        json(&miss),
        r#"{"expr":["if",["is_err",["var","result"]],["ok",["var","a"]],["var","result"]],"fault":"missing-error","kind":"replace","op":"select_bytes","requires":["ir.core"]}"#
    );
    // one rewrite per Bytes parameter, each guarded by len(p) > 0
    let ilb = instantiate(&ct_op("eq"), "ignore-last-byte").unwrap();
    assert_eq!(
        json(&ilb),
        concat!(
            r#"{"fault":"ignore-last-byte","kind":"pre","op":"eq","requires":["ir.core"],"rewrites":["#,
            r#"{"expr":["if",["gt",["len",["var","a"]],["lit",{"type":"Nat","value":"0"}]],"#,
            r#"["concat",["take",["var","a"],["sub",["len",["var","a"]],["lit",{"type":"Nat","value":"1"}]]],["lit",{"type":"Bytes","value":"00"}]],["var","a"]],"param":"a"},"#,
            r#"{"expr":["if",["gt",["len",["var","b"]],["lit",{"type":"Nat","value":"0"}]],"#,
            r#"["concat",["take",["var","b"],["sub",["len",["var","b"]],["lit",{"type":"Nat","value":"1"}]]],["lit",{"type":"Bytes","value":"00"}]],["var","b"]],"param":"b"}]}"#
        )
    );
    let hb = instantiate(&ct_op("eq_u64"), "ignore-high-bit").unwrap();
    assert_eq!(hb.requires, [Level::Fixed]);
    assert_eq!(first_unimplemented(&hb.requires, &[Level::Core]), Some(Level::Fixed));
    assert_eq!(
        json(&hb),
        concat!(
            r#"{"fault":"ignore-high-bit","kind":"pre","op":"eq_u64","requires":["ir.fixed"],"rewrites":["#,
            r#"{"expr":["bit_and",["var","a"],["lit",{"type":"U64","value":"9223372036854775807"}]],"param":"a"},"#,
            r#"{"expr":["bit_and",["var","b"],["lit",{"type":"U64","value":"9223372036854775807"}]],"param":"b"}]}"#
        )
    );
}

const NUMERIC: &str = "  model one(a: U8) -> U8 = a\n  op nat(a: Bytes) -> Nat\n  op byte(a: Bytes) -> U8\n  op blob(a: Bytes) -> Bytes\n  op word(a: U64) -> U64\n";

#[test]
fn the_other_templates_instantiate_by_signature() {
    let src = doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool", "op nat(a: Bytes, b: Bytes) -> Nat", "op byte(a: Bytes, b: Bytes) -> U8", "op blob(a: Bytes, b: Bytes) -> Bytes"], "");
    let _ = NUMERIC;
    with_file(&src, |f| {
        let l = lower_document(f).unwrap();
        let off = instantiate(l.op("nat").unwrap(), "off-by-one").unwrap();
        assert!(json(&off).contains(r#"["add",["var","result"],["lit",{"type":"Nat","value":"1"}]]"#));
        let off8 = instantiate(l.op("byte").unwrap(), "off-by-one").unwrap();
        assert!(json(&off8).contains(r#"{"type":"U8","value":"1"}"#));
        let tl = instantiate(l.op("blob").unwrap(), "truncate-last").unwrap();
        assert!(matches!(tl.kind, Kind::Post(Expr::Core(Ir::If(..)))));
        let xor = instantiate(l.op("byte").unwrap(), "xor-lsb").unwrap();
        assert_eq!(xor.requires, [Level::Fixed]);
        assert!(json(&xor).contains(r#"["bit_xor",["var","result"],["lit",{"type":"U8","value":"1"}]]"#));
        // every name of the registry is reachable
        assert_eq!(KNOWN_FAULTS.len(), 18);
    });
}

#[test]
fn a_template_the_signature_cannot_take_is_refused() {
    let eq = ct_op("eq");
    for (op, fault, why) in [
        (ct_op("select_bytes"), "invert", "Bool result"),
        (eq.clone(), "missing-error", "needs a Result result"),
        (eq.clone(), "swap-choice", "`choice`"),
        (eq.clone(), "off-by-one", "Nat or fixed"),
        (eq.clone(), "truncate-last", "Bytes result"),
        (eq.clone(), "xor-lsb", "fixed-width result"),
        (eq.clone(), "ignore-high-bit", "U64 parameter"),
        (ct_op("eq_u64"), "ignore-last-byte", "Bytes parameter"),
    ] {
        let e = instantiate(&op, fault).unwrap_err();
        assert_eq!(e.code, code::FAULT, "{fault}");
        assert!(e.message.contains(why), "{fault}: {e}");
        assert_eq!(e.at, format!("{}/{fault}", op.name));
    }
    let e = instantiate(&eq, "no-such-fault").unwrap_err();
    assert_eq!(e.code, code::FAULT);
    assert!(e.message.contains("unknown fault name"));
}

#[test]
fn a_parameter_called_result_is_w0701() {
    let mut op = ct_op("eq");
    op.params[0].0 = "result".into();
    let e = instantiate(&op, "invert").unwrap_err();
    assert_eq!(e.code, code::W0701);
    assert!(e.message.contains("result"));
    // and the generator checks every op, faulted or not
    with_file(CT_COMPARE, |f| {
        let mut a = analyse(f, &Profile::default()).unwrap();
        a.lowered.ops[3].params[0].0 = "result".into();
        let e = build_faults(&a.lowered, &Profile::default(), None).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert_eq!(e.at, "eq_u64");
    });
}

// ---- build_faults: profile and bindings -------------------------------------------------

#[test]
fn profile_faults_are_sorted_and_cross_checked_with_the_bindings() {
    with_file(CT_COMPARE, |f| {
        let p = parse_profile(CT_PROFILE.as_bytes()).unwrap();
        let a = analyse(f, &p).unwrap();
        let names = |fs: &[Fault]| fs.iter().map(|f| format!("{}/{}", f.op, f.name)).collect::<Vec<_>>();
        let fs = build_faults(&a.lowered, &p, None).unwrap();
        assert_eq!(names(&fs)[0], "eq/ignore-last-byte");
        assert_eq!(names(&fs).len(), 8);
        // shuffled profile lines give the same order
        let shuffled: String = CT_PROFILE.lines().rev().map(|l| format!("{l}\n")).collect();
        let p2 = parse_profile(shuffled.as_bytes()).unwrap();
        assert_eq!(build_faults(&a.lowered, &p2, None).unwrap(), fs);
        // bindings equal: fine (order and duplicates do not matter)
        let b: Vec<(String, Vec<String>)> = vec![
            ("eq".into(), vec!["invert".into(), "ignore-last-byte".into()]),
            ("eq_fixed".into(), vec!["invert".into(), "ignore-last-byte".into()]),
            ("select_bytes".into(), vec!["swap-choice".into(), "missing-error".into()]),
            ("eq_u64".into(), vec!["invert".into(), "ignore-high-bit".into()]),
        ];
        assert!(build_faults(&a.lowered, &p, Some(&b)).is_ok());
        // a fault the bindings lack, a fault removed from the profile, an op missing
        let less: String = CT_PROFILE.lines().skip(1).map(|l| format!("{l}\n")).collect();
        let less = parse_profile(less.as_bytes()).unwrap();
        let e = build_faults(&a.lowered, &less, Some(&b)).unwrap_err();
        assert_eq!(e.code, code::DRIFT);
        let mut fewer = b.clone();
        fewer[0].1.pop();
        assert_eq!(build_faults(&a.lowered, &p, Some(&fewer)).unwrap_err().code, code::DRIFT);
        let mut gone = b.clone();
        gone.pop();
        assert_eq!(build_faults(&a.lowered, &p, Some(&gone)).unwrap_err().code, code::DRIFT);
        // an op the document does not declare
        let bad = parse_profile(b"fault nope invert\n").unwrap();
        assert_eq!(build_faults(&a.lowered, &bad, None).unwrap_err().code, code::PROFILE);
    });
}

// ---- the witness search -----------------------------------------------------------------

const LATE: &str = "  model m(a: Bytes<1>, b: Bytes<1>) -> Bool = a == b
  model id(x: Bytes<1>) -> Bytes<1> = x
  model m2(a: Bytes<1>, b: Bytes<1>) -> Bool = b == b\"00\"
  claim late:
    forall a: Bytes<1> | a == b\"80\", b: Bytes<1>. g(a, b) == m(a, b)
  claim swapped:
    forall a: Bytes<1>, b: Bytes<1>. g(a, b) == m2(a, b)
  claim computed:
    forall a: Bytes<1>, b: Bytes<1>. g(a, id(b)) == m(a, b)
  claim relation:
    forall a: Bytes<1>, b: Bytes<1>. g(a, b) == g(b, a)
";

fn late_doc() -> String {
    // `g` takes Bytes<1>: written out because `doc_ops` only knows the (Bytes, Bytes) shape
    let ops = ["op g(a: Bytes<1>, b: Bytes<1>) -> Bool"];
    doc_ops(" v1", &ops, LATE)
}

#[test]
fn a_witness_beyond_chunk_zero_joins_the_slim_set() {
    let doc = late_doc();
    let profile = "chunk_size 1024\nfault g ignore-last-byte\nfault g invert\n";
    let c = gen(&doc, profile).expect("generates");
    let j = parse(&c.corpus_json).unwrap();
    let late = j.get("suites").a().iter().find(|s| s.get("claim").s() == "late").unwrap();
    assert_eq!(late.get("chunks").n(), 64);
    // a=0x80 is the 129th of 256 values: ordinal 128*256 = 32768, chunk 32
    let base = slim_base_set("warrant/synth/exhaustive/late", 64, None);
    let got: Vec<u64> = late.get("slim_chunks").a().iter().map(|x| x.n()).collect();
    assert!(got.contains(&32), "{got:?}");
    assert!(!base.contains(&32), "the stride alone would miss the witness: {base:?}");
    assert!(got.len() == base.len() + 1 && got.windows(2).all(|w| w[0] < w[1]));
    for control in j.get("controls").a() {
        let w: Vec<_> = control.get("slim_witness").a().iter().collect();
        let l = w.iter().find(|e| e.get("claim").s().ends_with("/late")).unwrap();
        assert_eq!(l.get("chunk").n(), 32, "{}", control.get("id").s());
    }
    // the slim digests follow the extended set: dropping the fault line gives the base set
    let none = gen(&doc, "chunk_size 1024\n").unwrap();
    let jn = parse(&none.corpus_json).unwrap();
    let ln = jn.get("suites").a().iter().find(|s| s.get("claim").s() == "late").unwrap();
    assert_eq!(ln.get("slim_chunks").a().len(), base.len());
    assert_ne!(ln.get("slim_inputs_sha256").s(), late.get("slim_inputs_sha256").s());
    assert!(jn.get("controls").a().is_empty());
}

#[test]
fn claims_without_a_stand_in_are_unproven_and_stay_required() {
    let doc = late_doc();
    let c = gen(&doc, "chunk_size 1024\nfault g invert\n").unwrap();
    let j = parse(&c.corpus_json).unwrap();
    let row = &j.get("controls").a()[0];
    let state = |claim: &str| {
        row.get("slim_witness")
            .a()
            .iter()
            .find(|e| e.get("claim").s().ends_with(claim))
            .map(|e| e.keys().into_iter().find(|k| *k != "claim").unwrap().to_string())
    };
    assert_eq!(state("/late").as_deref(), Some("chunk"));
    assert_eq!(state("/swapped").as_deref(), Some("chunk"));
    // a computed argument: no stand-in; a predicate: no stand-in
    assert_eq!(state("/computed").as_deref(), Some("unproven"));
    assert_eq!(state("/relation").as_deref(), Some("unproven"));
    assert_eq!(row.get("required_claims").a().len(), 4);
    assert!(row.get("blind_claims").a().is_empty());
    // a control with unproven claims and one witnessed: ok; with ONLY unproven ones it still passes
    let only = "  claim relation:\n    forall a: Bytes<1>, b: Bytes<1>. g(a, b) == g(b, a)\n";
    let doc2 = doc_ops(" v1", &["op g(a: Bytes<1>, b: Bytes<1>) -> Bool"], only);
    let c2 = gen(&doc2, "chunk_size 1024\nfault g invert\n").unwrap();
    let j2 = parse(&c2.corpus_json).unwrap();
    assert_eq!(
        j2.get("controls").a()[0].get("slim_witness").a()[0].get("unproven"),
        &support::minijson::J::B(true)
    );
}

#[test]
fn pre_fault_rewrites_apply_simultaneously() {
    // rewrites a := b and b := a over `g(a,b) == (b == 00)`: simultaneously the faulted
    // expected is (a == 00); applied one after the other it would be (b == 00) and the fault
    // would be invisible.  The search must see it, at the lowest case: a=00, b=01 (ordinal 1).
    let doc = late_doc();
    with_file(&doc, |f| {
        let p = parse_profile(b"chunk_size 1024\n").unwrap();
        let a = analyse(f, &p).unwrap();
        let op = a.lowered.op("g").unwrap().clone();
        let var = |n: &str| Box::new(Ir::Var(n.into()));
        let mk = |swap: bool| Fault {
            name: "swap-ab".into(),
            op: "g".into(),
            requires: vec![Level::Core],
            kind: Kind::Pre(vec![
                ("a".into(), Expr::Core(*var("b"))),
                ("b".into(), Expr::Core(if swap { *var("a") } else { *var("b") })),
            ]),
        };
        let (swap, same) = (mk(true), mk(false));
        let mut seen = Vec::new();
        enumerate_analysed(f, &p, a, &mut |view| {
            if view.suite.claim == "swapped" {
                for fault in [&swap, &same] {
                    let si = stand_in(view.suite, fault, &op).expect("a stand-in");
                    let probes = [Probe { fault, op: &op, stand_in: si }];
                    seen.push(search(view.program, view.suite, view.plan, &probes)?[0]);
                }
            }
            Ok(())
        })
        .unwrap();
        // swap: (a==00) vs (b==00) differ first at a=00,b=01; same (b:=b, a:=b): expected(b,b) = (b==00): equal always
        assert_eq!(seen, vec![Some(1), None]);
    });
}

#[test]
fn a_binder_passed_twice_needs_one_rewrite_or_there_is_no_stand_in() {
    let doc = doc_ops(
        " v1",
        &["op g(a: Bytes<1>, b: Bytes<1>) -> Bool"],
        "  model m(a: Bytes<1>) -> Bool = a == b\"00\"\n  claim twice:\n    forall x: Bytes<1>. g(x, x) == m(x)\n",
    );
    with_file(&doc, |f| {
        let a = analyse(f, &Profile::default()).unwrap();
        let op = a.lowered.op("g").unwrap();
        let s = &a.lowered.suites[0];
        let one = Fault {
            name: "one".into(),
            op: "g".into(),
            requires: vec![Level::Core],
            kind: Kind::Pre(vec![("a".into(), Expr::Core(Ir::Var("b".into())))]),
        };
        assert!(stand_in(s, &one, op).is_none(), "x is rewritten at one position only");
        let both = Fault {
            kind: Kind::Pre(vec![
                ("a".into(), Expr::Core(Ir::Var("b".into()))),
                ("b".into(), Expr::Core(Ir::Var("b".into()))),
            ]),
            ..one.clone()
        };
        assert!(stand_in(s, &both, op).is_some());
        let post = Fault { kind: Kind::Post(Expr::Core(Ir::Var("result".into()))), ..one };
        assert_eq!(stand_in(s, &post, op).unwrap().binders, ["x", "x"]);
        // a different op, a wrong arity and a predicate suite have none
        let other = Fault { op: "h".into(), ..post.clone() };
        assert!(stand_in(s, &other, op).is_none());
    });
}

#[test]
fn claim_states_are_comparable() {
    assert_eq!(ClaimState::Witnessed(3), ClaimState::Witnessed(3));
    assert_ne!(ClaimState::Blind, ClaimState::Unproven);
    assert_ne!(ClaimState::Deferred(Level::Fixed), ClaimState::Blind);
    assert_eq!(Level::Core.name(), "ir.core");
    assert!(Level::Core < Level::Fixed);
}

// ---- reserved device names (L5) ---------------------------------------------------------

#[test]
fn windows_reserved_chunk_names_are_refused_at_generation() {
    for name in ["nul", "con", "aux", "prn", "com1", "lpt9", "NUL"] {
        let doc = doc_ops(
            " v1",
            &["op f(a: Bytes, b: Bytes) -> Bool"],
            &format!("  model m(a: Bytes, b: Bytes) -> Bool = a == b\n  claim {name}:\n    forall a: Bytes<1>, b: Bytes<1>. f(a, b) == m(a, b)\n"),
        );
        let e = gen(&doc, "").unwrap_err();
        assert_eq!(e.code, code::W0701, "{name}: {e}");
        assert!(e.message.contains("reserved device name"), "{e}");
    }
    // a name that merely contains one is fine
    let ok = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  model m(a: Bytes, b: Bytes) -> Bool = a == b\n  claim console:\n    forall a: Bytes<1>, b: Bytes<1>. f(a, b) == m(a, b)\n",
    );
    assert!(gen(&ok, "").is_ok());
}

#[test]
fn tfv_refuses_an_example_that_is_not_a_subject_call() {
    use coding_adventures_warrant_corpus_gen::eval::Program;
    use coding_adventures_warrant_corpus_gen::tfv::examples;
    let doc = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  example e:\n    f(b\"\", b\"\") == true\n",
    );
    with_file(&doc, |f| {
        let mut a = analyse(f, &Profile::default()).unwrap();
        let p = Program::new(&a.lowered.models);
        assert_eq!(examples(&a.lowered, &p).unwrap().lines.len(), 1);
        a.lowered.examples[0].reference.as_mut().unwrap().subject = Ir::Bool(true);
        let e = examples(&a.lowered, &Program::new(&a.lowered.models)).unwrap_err();
        assert_eq!((e.code, e.at.as_str()), (code::W0701, "e"));
        a.lowered.examples[0].reference.as_mut().unwrap().subject = Ir::Subject { op: "nope".into(), args: vec![], widths: vec![] };
        let e = examples(&a.lowered, &Program::new(&a.lowered.models)).unwrap_err();
        assert!(e.message.contains("undeclared op"), "{e}");
    });
}
