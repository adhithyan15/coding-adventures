//! The evaluator oracle's refusals, `run_cross_checks` on a tampered contract, and the CLI.
//! Every refusal is a message, never a panic (VOS31 section 9, "untrusted input").

mod common;

use std::process::Command;

use coding_adventures_warrant_grammar::codegen::EvalValue;
use common::contract_dir;
use vos_model_sat::oracle::{
    load, on_big_stack, run_document, to_val, with_probe_claims, ContractFiles, CASE_BUDGET,
};
use vos_model_sat::probes::{all_probes, Ty};
use vos_model_sat::{run_cross_checks, Val};

fn fresh() -> ContractFiles {
    load(&contract_dir(), None).expect("the real contract loads")
}

#[test]
fn load_refuses_a_missing_directory_and_a_dotdot_path() {
    let missing = contract_dir().join("no-such-crate");
    assert!(load(&missing, None).is_err());
    let dotdot = contract_dir().join("..").join("vos-sched-contract");
    let e = load(&dotdot, None).unwrap_err();
    assert!(e.contains(".."), "{e}");
}

#[test]
fn the_documents_are_the_ones_check_contract_evaluates() {
    let f = fresh();
    let base = f.base_document();
    assert!(base.starts_with("warrant "));
    assert!(base.contains("model cfg_N() -> U8 = 8u8"));
    assert!(base.contains("claim closure_at_m"));
    // The mutant replaces ONE model by span: `top` differs, everything else is the base.
    let m = f.mutant_document("top_is_the_maximum").unwrap();
    assert_ne!(m, base);
    assert!(m.contains("if m.p0 then Run(0u8) else (if m.p6"));
    assert_eq!(f.mutants.len(), 10);
    assert_eq!(f.contract.config.len(), 1);
}

#[test]
fn mutant_document_refusals() {
    let mut f = fresh();
    assert!(f
        .mutant_document("no_such_lemma")
        .unwrap_err()
        .contains("no mutant part"));
    // Two declarations: not exactly one model.
    let two = "  model inv(m: Ready) -> Bool = true\n  model init_m() -> Ready = empty()\n";
    f.mutants[0].1 = two.to_string();
    assert!(f
        .mutant_document("closure_at_m")
        .unwrap_err()
        .contains("not exactly one"));
    // A claim, not a model.
    f.mutants[0].1 = "  claim c: forall b: Bool. (b == b)\n".to_string();
    let e = f.mutant_document("closure_at_m").unwrap_err();
    assert!(e.contains("mutant of closure_at_m"), "{e}");
    // A model with no body (a bound operation stand-in).
    f.mutants[0].1 = "  model inv(m: Ready) -> Bool
"
    .to_string();
    let e = f.mutant_document("closure_at_m").unwrap_err();
    assert!(e.contains("mutant of closure_at_m"), "{e}");
    // A model the contract does not declare.
    f.mutants[0].1 = "  model nothing_like_it(m: Ready) -> Bool = true\n".to_string();
    assert!(f
        .mutant_document("closure_at_m")
        .unwrap_err()
        .contains("not in contract.part"));
    // Text that does not parse.
    f.mutants[0].1 = "  model inv(((\n".to_string();
    assert!(f.mutant_document("closure_at_m").is_err());
    // A broken contract text.
    let mut g = fresh();
    g.contract_text = "  model (((\n".to_string();
    assert!(g
        .mutant_document("closure_at_m")
        .unwrap_err()
        .contains("contract.part"));
}

fn doc_with(extra: &str) -> String {
    format!("{}{extra}", fresh().base_document())
}

#[test]
fn run_document_refuses_what_it_cannot_enumerate() {
    // A claim that is not a forall.
    let d = doc_with("  claim plain:\n    inv(empty())\n");
    assert!(run_document(&d, &["plain"], &[])
        .unwrap_err()
        .contains("not a `forall`"));
    // Three U8 binders: 2^24 cases, over the budget.
    let d = doc_with("  claim big:\n    forall a: U8, b: U8, c: U8. (a == a)\n");
    let e = run_document(&d, &["big"], &[]).unwrap_err();
    assert!(e.contains("budget") && CASE_BUDGET == 400_000, "{e}");
    // A binder type that is neither Bool nor U8.
    let d = doc_with("  claim wide:\n    forall a: U16. (a == a)\n");
    assert!(run_document(&d, &["wide"], &[])
        .unwrap_err()
        .contains("not Bool or U8"));
    // A refinement guard.
    let d = doc_with("  claim guarded:\n    forall a: U8 where a < 4u8. (a == a)\n");
    let e = run_document(&d, &["guarded"], &[]);
    assert!(e.is_err(), "{e:?}");
    // No such claim.
    assert!(run_document(&doc_with(""), &["absent"], &[])
        .unwrap_err()
        .contains("no claim named"));
    // A probe whose body is not `a == a`.
    let d = doc_with("  claim probe_odd:\n    forall a: Bool. a\n");
    let odd = vos_model_sat::probes::Probe {
        name: "odd",
        ..all_probes()
            .into_iter()
            .find(|p| p.name == "pick_of")
            .unwrap()
    };
    assert!(run_document(&d, &[], &[odd])
        .unwrap_err()
        .contains("a == a"));
    // A document that does not parse, and one that does not type-check.
    assert!(run_document("not a warrant document", &[], &[]).is_err());
    let d = doc_with("  claim ill:\n    forall a: Bool. (a == 3u8)\n");
    assert!(run_document(&d, &[], &[])
        .unwrap_err()
        .contains("type check"));
}

#[test]
fn run_document_reads_a_probe_it_cannot_convert_as_an_error() {
    // The probe says it is a Ready but the Warrant expression is a Bool: refused, with the case.
    let d = with_probe_claims(&fresh().base_document(), &all_probes());
    let wrong = vos_model_sat::probes::Probe {
        ty: Ty::Ready,
        ..all_probes()
            .into_iter()
            .find(|p| p.name == "pre_insert")
            .unwrap()
    };
    let e = run_document(&d, &[], &[wrong]).unwrap_err();
    assert!(e.contains("unexpected value"), "{e}");
}

#[test]
fn to_val_rejects_a_mismatched_value() {
    assert_eq!(
        to_val(&EvalValue::Bool(true), Ty::Bool),
        Some(Val::Bool(true))
    );
    assert_eq!(to_val(&EvalValue::Bool(true), Ty::Ready), None);
    assert_eq!(to_val(&EvalValue::U8(1), Ty::Pick), None);
    let con = |ctor: &str, index: usize, fields: Vec<EvalValue>| EvalValue::Con {
        ty: "T".to_string(),
        index,
        ctor: ctor.to_string(),
        record: false,
        fields,
    };
    assert_eq!(
        to_val(&con("Idle", 0, vec![]), Ty::Pick),
        Some(Val::Pick(None))
    );
    assert_eq!(
        to_val(&con("Run", 1, vec![EvalValue::U8(9)]), Ty::Pick),
        Some(Val::Pick(Some(9)))
    );
    assert_eq!(to_val(&con("Other", 1, vec![]), Ty::Pick), None);
    assert_eq!(to_val(&con("Run", 1, vec![]), Ty::Pick), None);
    assert_eq!(
        to_val(&con("OutOfRange", 2, vec![]), Ty::Upd),
        Some(Val::Upd(2))
    );
    assert_eq!(
        to_val(
            &con(
                "R",
                0,
                vec![
                    EvalValue::Bool(true),
                    EvalValue::Bool(false),
                    EvalValue::Bool(true)
                ]
            ),
            Ty::Ready
        ),
        Some(Val::Ready(5))
    );
    assert_eq!(
        to_val(&con("R", 0, vec![EvalValue::U8(1)]), Ty::Ready),
        None
    );
}

#[test]
fn big_stack_runner_returns_the_value() {
    assert_eq!(on_big_stack(|| 7).unwrap(), 7);
}

#[test]
fn negative_control_a_tampered_contract_fails_the_whole_cross_check() {
    // The contract text now says `t < 9u8` in pre_insert while the encoder (and every mutant
    // part) is unchanged: the evaluator and the encoder disagree and the run is not ok.
    let mut f = fresh();
    let needle = "model pre_insert(m: Ready, t: U8) -> Bool = t < 8u8";
    assert!(f.contract_text.contains(needle));
    f.contract_text = f.contract_text.replace(
        needle,
        "model pre_insert(m: Ready, t: U8) -> Bool = t < 9u8",
    );
    let (lines, ok) = run_cross_checks(&f).unwrap();
    assert!(!ok, "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("FAILED")));
}

#[test]
fn run_cross_checks_reports_an_unreadable_mutant_as_an_error() {
    let mut f = fresh();
    f.mutants.retain(|(s, _)| s != "closure_at_m");
    assert!(run_cross_checks(&f).is_err());
}

// ---- the command line ---------------------------------------------------------------------

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_vos-model-sat"))
        .args(args)
        .output()
        .expect("the binary runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn cli_usage_errors_are_exit_2() {
    assert_eq!(run(&[]).0, 2);
    assert_eq!(run(&["solve"]).0, 2);
    assert_eq!(run(&["cross-check"]).0, 2);
    assert_eq!(run(&["cross-check", "a", "b"]).0, 2);
    assert_eq!(run(&["cross-check", "a", "--root"]).0, 2);
    let (code, _, err) = run(&["cross-check", "no-such-dir"]);
    assert_eq!(code, 2);
    assert!(err.contains("no verdict"), "{err}");
}

#[test]
fn cli_cross_check_on_the_real_contract_is_exit_0() {
    let dir = contract_dir();
    let (code, out, err) = run(&["cross-check", dir.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("0 disagreements"));
    assert!(!out.contains("FAILED"));
}
