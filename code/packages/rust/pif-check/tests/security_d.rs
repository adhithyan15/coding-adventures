//! Security-review regressions for PIF-003d: the decoder refuses cyclic tables, the printer
//! never writes raw header text, one budget covers a whole file, the shadow check is linear and
//! `pif-pp goal` prints the digest a signer compares.

mod common;

use common::*;
use pif_check::cte::Dterm;
use pif_check::filemodel::{decode, DeclModel, FileModel, ModelError, FILE_BUDGET};
use pif_check::filetext::{parse_file, print_file};
use pif_check::pp_cli::run;
use pif_check::printer::print_term;
use std::time::{Duration, Instant};

const HEADER: &str = "pif 1\nlogic 1\nprofile native \"x\"\nfeatures\n";

/// Run `f` on a big-stack thread and fail if it does not finish within `secs`.
fn bounded<T: Send + 'static>(secs: u64, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let _ = tx.send(f());
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(secs))
        .expect("did not finish in time")
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

// ---- finding 9: backward-only operands ------------------------------------------------------

#[test]
fn finding_9_cyclic_and_forward_references_are_refused_quickly() {
    let forward = ModelError::Decode("forward reference", 5);
    // The confirmed crafted inputs.
    let self_app = format!("{HEADER}eapp 1 1 1\naxiom 0 [] 1\nend 2\n");
    let self_succ = format!("{HEADER}lsucc 1 1\nesort 1 1\naxiom 0 [] 1\nend 3\n");
    for (name, text) in [("eapp 1 1 1", &self_app), ("lsucc 1 1", &self_succ)] {
        let t = text.clone();
        let r = bounded(30, move || decode(&t));
        assert!(
            matches!(r, Err(ModelError::Decode("forward reference", _))),
            "{name}: {r:?}"
        );
    }
    assert_eq!(decode(&self_app).err().unwrap(), forward);
    // Other forward operands: a later expression, level, and name.
    for body in [
        "eapp 1 2 2\neapp 2 1 1\n",
        "esort 1 3\n",
        "lmax 1 1 0\n",
        "lparam 1 7\n",
        "econst 1 9 []\n",
        "elam 1 default 5 9 9\n",
        "elet 1 0 1 1 1\n",
        "evar 1 0\nepi 2 default 0 3 1\n",
        "evar 1 0\neapp 2 1 3\n",
        "evar 1 0\nelam 2 default 3 1 1\n",
    ] {
        let text = format!("{HEADER}{body}end 2\n");
        assert!(
            matches!(
                decode(&text),
                Err(ModelError::Decode("forward reference", _) | ModelError::Decode(_, _))
            ),
            "{body}"
        );
    }
    // The command line refuses them, with the reason on standard error, in bounded time.
    let t = Tmp::new();
    let p = t.put("cyclic.pif", self_app.as_bytes());
    let out = bounded(30, move || run(&s(&["file", &p])));
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("forward reference"), "{}", out.stderr);
    let p = t.put("cyclic2.pif", self_succ.as_bytes());
    assert_eq!(bounded(30, move || run(&s(&["check", &p]))).code, 1);
}

#[test]
fn finding_9_an_exponential_dag_is_stopped_by_the_step_budget() {
    // 60 doublings: 2^60 leaves as a tree, 62 records as a DAG. Backward-only, so it is valid
    // input; the budget (and the per-term cap) must stop the unfolding.
    let mut text = String::from(HEADER);
    text.push_str("evar 1 0\n");
    for i in 2..=61 {
        text.push_str(&format!("eapp {i} {} {}\n", i - 1, i - 1));
    }
    text.push_str("axiom 0 [] 61\n");
    text.push_str(&format!("end {}\n", 62));
    let r = bounded(30, move || decode(&text));
    assert_eq!(r.err(), Some(ModelError::TooLarge));
}

// ---- finding 10: file prints only after a round trip, header words are plain ----------------

#[test]
fn finding_10_header_words_cannot_carry_terminal_escapes() {
    for bad_id in [
        "na\u{1b}ive",
        "n\rative",
        "na\u{202e}tive",
        "Native",
        "n ative",
        "",
    ] {
        let text = format!("pif 1\nlogic 1\nprofile {bad_id} \"x\"\nfeatures\nend 0\n");
        assert!(decode(&text).is_err(), "{bad_id:?}");
    }
    for bad_feature in ["lits\u{1b}[31m", "li\u{202e}ts", "Lits", "li-ts"] {
        let text = format!("pif 1\nlogic 1\nprofile native \"x\"\nfeatures {bad_feature}\nend 0\n");
        assert!(decode(&text).is_err(), "{bad_feature:?}");
    }
    // A hand-built model with such words is refused by the printer too.
    let model = |id: &str, feature: &str| FileModel {
        profile: (id.into(), "label".into()),
        features: vec![feature.into()],
        imports: vec![],
        decls: vec![],
    };
    assert!(print_file(&model("native", "lits")).is_ok());
    assert!(print_file(&model("na\u{1b}", "lits")).is_err());
    assert!(print_file(&model("native", "li\u{202e}ts")).is_err());
    // A label with an escape character is printed quoted (as \u{1b}), never raw.
    let text = "pif 1\nlogic 1\nprofile native \"a\\u{1b}b\\u{202e}\"\nfeatures\nend 0\n";
    let printed = print_file(&decode(text).unwrap()).unwrap();
    assert!(
        printed.is_ascii() && !printed.bytes().any(|b| b < 0x20 && b != b'\n'),
        "{printed:?}"
    );
    // `pif-pp file` prints a file only after the same round trip as `check`, and its whole
    // output is printable ASCII.
    let t = Tmp::new();
    let path = t.put("prelude.pif", &prelude());
    let out = run(&s(&["file", &path]));
    assert_eq!(out.code, 0);
    assert!(out
        .stdout
        .bytes()
        .all(|b| b == b'\n' || (0x20..0x7f).contains(&b)));
    let path = t.put(
        "evil.pif",
        "pif 1\nlogic 1\nprofile na\u{1b}ive \"x\"\nfeatures\nend 0\n".as_bytes(),
    );
    let out = run(&s(&["file", &path]));
    assert_eq!(out.code, 1);
    assert!(out.stdout.is_empty());
}

// ---- finding 11: one budget for the whole file ----------------------------------------------

#[test]
fn finding_11_the_budget_is_per_file_not_per_term() {
    // One shared DAG with 2^19 leaves (under the per-term cap of 2^20 nodes) used by many
    // declarations: each is fine alone, the file as a whole is not.
    let mut text = String::from(HEADER);
    text.push_str("evar 1 0\n");
    for i in 2..=20 {
        text.push_str(&format!("eapp {i} {} {}\n", i - 1, i - 1));
    }
    let decls = 40;
    for _ in 0..decls {
        text.push_str("axiom 0 [] 20\n");
    }
    text.push_str(&format!("end {}\n", 20 + decls));
    let one = text
        .replace(&"axiom 0 [] 20\n".repeat(decls), "axiom 0 [] 20\n")
        .replace(&format!("end {}", 20 + decls), "end 21");
    let (many, single) = (text.clone(), one.clone());
    // A single such declaration decodes (it fits the budget); forty of them do not.
    let r = bounded(60, move || decode(&single).map(|m| m.decls.len()));
    assert_eq!(r, Ok(1));
    let r = bounded(60, move || decode(&many));
    assert_eq!(r.err(), Some(ModelError::TooLarge));
    assert!(FILE_BUDGET < decls * (1 << 19));
    // The printed text and the text to parse back are bounded as a whole too.
    let huge = format!(
        "pifpp 1\nprofile native \"x\"\nfeatures\n{}",
        "axiom A : Sort 0\n".repeat(2_100_000)
    );
    assert_eq!(parse_file(&huge).err(), Some(ModelError::TooLarge));
}

// ---- finding 12: the shadow check is linear ---------------------------------------------------

#[test]
fn finding_12_thirty_thousand_binders_print_quickly() {
    let r = bounded(60, || {
        let start = Instant::now();
        let n = 30_000;
        // forall (b0 : T), forall (b1 : T), ... all distinct and unused.
        let mut cte = String::new();
        for i in 0..n {
            cte.push_str(&format!("Pd(<\"b{i}\">,C<\"T\">[],"));
        }
        cte.push_str("C<\"T\">[]");
        cte.push_str(&")".repeat(n));
        let t = Dterm::parse(&cte).unwrap();
        let text = print_term(&t).unwrap();
        // The same names repeated and unused (allowed), and repeated and used (refused).
        let mut rep = String::new();
        for _ in 0..n {
            rep.push_str("Pd(<\"a\">,C<\"T\">[],");
        }
        rep.push_str("C<\"T\">[]");
        rep.push_str(&")".repeat(n));
        let unused = print_term(&Dterm::parse(&rep).unwrap());
        // The innermost body `v0` uses the innermost binder, and 29,999 outer ones share its name.
        let mut used = rep.clone();
        let at = used.rfind("C<\"T\">[]").unwrap();
        used.replace_range(at..at + "C<\"T\">[]".len(), "v0");
        let used = print_term(&Dterm::parse(&used).unwrap());
        (text.len(), unused.is_ok(), used.is_err(), start.elapsed())
    });
    assert!(r.0 > 30_000);
    assert!(r.1, "unused repeated binders are allowed");
    assert!(r.2, "a used binder that shadows is refused");
    assert!(r.3 < Duration::from_secs(30), "took {:?}", r.3);
}

// ---- finding 13: pif-pp goal prints the digest ----------------------------------------------

#[test]
fn finding_13_pif_pp_goal_prints_the_goal_digest() {
    let fixture = include_str!("../../lean-kernel/tests/fixtures/worked-example.pif");
    let model = decode(fixture).unwrap();
    let Some(DeclModel::Theorem { ty, .. }) = model
        .decls
        .iter()
        .find(|d| matches!(d, DeclModel::Theorem { .. }))
    else {
        panic!()
    };
    let out = run(&s(&["goal", &ty.display().unwrap()]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let mut lines = out.stdout.lines();
    assert!(lines.next().unwrap().starts_with("forall (a : Bool)"));
    assert_eq!(
        lines.next().unwrap(),
        format!("goal-digest {WORKED_DIGEST}")
    );
    assert!(lines.next().is_none());
}
