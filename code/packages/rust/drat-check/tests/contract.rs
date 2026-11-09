//! # Contract tests: one test per rule of VER11 section 3.4
//!
//! The bounded-exhaustive file (`exhaustive.rs`) proves the *semantics* on a
//! small universe. This file pins down everything that universe cannot
//! reach: the exact grammar of the two inputs (L1-L4, F1-F9, P1-P6), the
//! precise meaning of RAT and deletion on hand-built cases (C1-C6), and the
//! resource limits (C8). Every assertion names the rule it checks.

use drat_check::parse::{parse_formula, parse_proof, ParseError};
use drat_check::{check, check_with_limit, check_with_limits, Limits, Rejection, DEFAULT_MAX_VARS};

/// (x1 v x2)(x1 v -x2)(-x1 v x2)(-x1 v -x2): UNSAT, refuted by `1 0 / 0`.
const XOR: &str = "p cnf 2 4\n1 2 0\n1 -2 0\n-1 2 0\n-1 -2 0\n";

fn formula_err(text: &str) -> (usize, String) {
    match check(text, "0\n") {
        Err(Rejection::Parse(ParseError::Formula { line, message })) => (line, message),
        other => panic!("expected a formula parse error for {text:?}, got {other:?}"),
    }
}

fn proof_err(proof: &str) -> ParseError {
    match check(XOR, proof) {
        Err(Rejection::Parse(e)) => e,
        other => panic!("expected a proof parse error for {proof:?}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Formula grammar
// ---------------------------------------------------------------------------

#[test]
fn f1_header_rules() {
    for bad in [
        "p cnf\n",
        "p cnf 1\n",
        "p cnf -1 0\n",
        "p cnf 1 -1\n",
        "p cnf 1 1 1\n1 0\n",
        "p CNF 1 1\n1 0\n",
        "p cnf 1.0 1\n1 0\n",
        "p cnf 0x1 1\n1 0\n",
        "p cnf +1 1\n1 0\n",
        "p cnf 99999999999999999999 0\n",
        "p  \n",
    ] {
        let (_, msg) = formula_err(bad);
        assert!(msg.contains("malformed") || msg.contains("limit"), "{bad:?}: {msg}");
    }
    // a capital P is not a header line: the line is a clause line
    assert!(formula_err("P cnf 1 1\n1 0\n").1.contains("before"));
    // whitespace flexibility: leading blanks, tabs, runs of spaces.
    assert!(check("  p\tcnf   1  2 \n1 0\n-1 0\n", "0\n").is_ok());
    // second header, header after clauses, no header
    assert!(formula_err("p cnf 1 0\np cnf 1 0\n").1.contains("second"));
    assert!(formula_err("1 0\np cnf 1 1\n").1.contains("before"));
    assert!(formula_err("").1.contains("no `p cnf`"));
    assert!(formula_err("c only a comment\n\n").1.contains("no `p cnf`"));
    // a stray token on the header line of a *non*-header is a clause error.
    assert!(formula_err("p cnf 1 1\np 1 0\n").1.contains("second"));
}

#[test]
fn f1_header_variable_limit() {
    let at_limit = format!("p cnf {DEFAULT_MAX_VARS} 0\n");
    // Table allocation is ~60 MB; the proof "0" is not implied by nothing.
    assert_eq!(check(&at_limit, "0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    let over = format!("p cnf {} 0\n", DEFAULT_MAX_VARS + 1);
    assert!(formula_err(&over).1.contains("limit"));
    // explicit limits, including 0 and an absurdly large one (capped at 2^30,
    // so a header of 2^30+1 is still refused without allocating anything).
    assert!(check_with_limit("p cnf 0 0\n", "0\n", 0).is_err());
    assert!(matches!(
        check_with_limit("p cnf 1 1\n1 0\n", "0\n", 0),
        Err(Rejection::Parse(ParseError::Formula { .. }))
    ));
    let huge = format!("p cnf {} 0\n", (1usize << 30) + 1);
    assert!(matches!(
        check_with_limit(&huge, "0\n", usize::MAX),
        Err(Rejection::Parse(ParseError::Formula { .. }))
    ));
}

#[test]
fn f2_f3_f4_f5_clause_rules() {
    // F3: variable above the header, zero, overflow, i64::MIN
    assert!(formula_err("p cnf 2 1\n3 0\n").1.contains("outside"));
    assert!(formula_err("p cnf 2 1\n-3 0\n").1.contains("outside"));
    assert!(formula_err("p cnf 0 1\n1 0\n").1.contains("outside"));
    assert!(formula_err("p cnf 2 1\n99999999999999999999 0\n").1.contains("not an integer"));
    assert!(formula_err("p cnf 2 1\n-99999999999999999999 0\n").1.contains("not an integer"));
    assert!(formula_err("p cnf 2 1\n9223372036854775808 0\n").1.contains("not an integer"));
    assert!(formula_err("p cnf 2 1\n-9223372036854775808 0\n").1.contains("outside"));
    assert!(formula_err("p cnf 2 1\n9223372036854775807 0\n").1.contains("outside"));
    // L4 tokens that are not plain integers
    for tok in ["+1", "-0", "+0", "--1", "1.5", "0x1", "1e0", "1_0", "١", "−1", "1,", "x", "- 1"] {
        let text = format!("p cnf 2 1\n{tok} 0\n");
        let r = check(&text, "0\n");
        assert!(matches!(r, Err(Rejection::Parse(ParseError::Formula { .. }))), "{tok:?} -> {r:?}");
    }
    // F4 open clause
    assert!(formula_err("p cnf 2 1\n1 2").1.contains("terminating"));
    assert!(formula_err("p cnf 2 2\n1 0\n2").1.contains("terminating"));
    // F5 count both ways
    assert!(formula_err("p cnf 2 2\n1 0\n").1.contains("declares 2 clauses, found 1"));
    assert!(formula_err("p cnf 2 1\n1 0\n2 0\n").1.contains("declares 1 clauses, found 2"));
    assert!(formula_err("p cnf 2 0\n1 0\n").1.contains("declares 0 clauses, found 1"));
    // F2 layout freedom
    assert!(check("p cnf 2 4\n1 2 0 1 -2 0\n-1\n2 0\n-1\n-2\n0\n", "1 0\n0\n").is_ok());
    // leading zeros are allowed
    assert!(check("p cnf 2 4\n01 02 0\n001 -2 0\n-1 2 0\n-1 -002 0\n", "1 0\n0\n").is_ok());
}

#[test]
fn f2_empty_clause_in_formula() {
    let f = "p cnf 1 1\n0\n";
    assert!(check(f, "0\n").is_ok());
    // C6: an empty proof is never accepted, even here.
    assert_eq!(check(f, ""), Err(Rejection::NoEmptyClause));
    // deleting the formula's empty clause really removes it.
    assert_eq!(check(f, "d 0\n0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    assert!(check(f, "d 0\n0\n").is_err());
    // an empty clause among others
    assert!(check("p cnf 1 2\n1 0\n0\n", "0\n").is_ok());
}

#[test]
fn l1_l2_l3_lexical_rules() {
    // CRLF everywhere, tabs, trailing whitespace, blank lines.
    let f = XOR.replace('\n', "\r\n");
    assert!(check(&f, "1 0\r\n0\r\n").is_ok());
    assert!(check("p cnf 2 4\n\n \t\n1 2 0\n1 -2 0\n-1 2 0\n-1 -2 0\n\n", "\t1\t0 \n\n 0\n").is_ok());
    // L3: comment lines start with c after trimming, and only those.
    assert!(check("c hello\n  c indented\ncnf looks like a comment too\np cnf 2 4\n1 2 0\n1 -2 0\n-1 2 0\n-1 -2 0\n", "c x\n 1 0\ncx\n0\n").is_ok());
    // a 'c' after tokens is a bad token, in both inputs
    assert!(formula_err("p cnf 2 1\n1 0 c comment\n").1.contains("not an integer"));
    assert!(matches!(proof_err("1 0 c comment\n0\n"), ParseError::BadProofToken { line: 1, .. }));
    // a comment line cannot hide a clause terminator or split a clause
    assert!(check("p cnf 1 1\n1\nc hi\n0\n", "").is_err());
    // BOM and NUL are not whitespace
    assert!(matches!(
        check("\u{feff}p cnf 1 1\n1 0\n", "0\n"),
        Err(Rejection::Parse(ParseError::Formula { .. }))
    ));
    assert!(matches!(proof_err("\u{0}0\n"), ParseError::BadProofToken { .. }));
}

#[test]
fn f6_percent_terminator() {
    let f = "p cnf 1 2\n1 0\n-1 0\n%\n0\n";
    assert!(check(f, "0\n").is_ok());
    // Everything after % is dropped unread, even garbage.
    assert!(check("p cnf 1 2\n1 0\n-1 0\n%\nthis is not DIMACS \u{0} at all\n", "0\n").is_ok());
    // ... and a % cuts clauses off: the count check then fails.
    assert!(formula_err("p cnf 1 2\n1 0\n%\n-1 0\n").1.contains("declares 2 clauses, found 1"));
    // % must be alone on its line to count
    assert!(formula_err("p cnf 1 1\n1 0 %\n").1.contains("not an integer"));
    assert!(formula_err("%\np cnf 1 0\n").1.contains("no `p cnf`"));
    // % in a proof is just a bad token (P1)
    assert!(matches!(proof_err("%\n"), ParseError::BadProofToken { .. }));
}

#[test]
fn f7_f8_duplicates_and_tautologies_in_the_formula() {
    // duplicate literal clause acts as a unit
    assert!(check("p cnf 1 2\n1 1 0\n-1 -1 0\n", "0\n").is_ok());
    // tautology never forces anything
    assert!(check("p cnf 1 1\n1 -1 0\n", "0\n").is_err());
    // a tautological formula clause plus a duplicate: still SAT, no proof
    assert!(check("p cnf 2 3\n1 -1 0\n1 2 0\n1 2 0\n", "0\n").is_err());
    // F8: duplicate clauses are counted by deletion
    let dup = "p cnf 1 3\n1 0\n1 0\n-1 0\n";
    assert!(check(dup, "d 1 0\n0\n").is_ok(), "one copy of (1) survives a single deletion");
    assert!(check(dup, "d 1 0\nd 1 0\n0\n").is_err(), "both copies gone: SAT");
    assert_eq!(check(dup, "d 1 0\nd 1 0\nd 1 0\n"), Err(Rejection::DeleteAbsent { step: 2, line: 3 }));
}

// ---------------------------------------------------------------------------
// Proof grammar
// ---------------------------------------------------------------------------

#[test]
fn p1_p2_proof_layout() {
    // steps over lines, several per line, comments between
    assert!(check(XOR, "1\n0 0").is_ok());
    assert!(check(XOR, "1 0 0").is_ok());
    assert!(check(XOR, "1 0\nc mid\n0").is_ok());
    assert!(check(XOR, "1 0 d\n1\n0 1 0 0").is_ok());
    // d only first in a step
    for bad in ["1 d 0\n", "d d 1 0\n", "d1 0\n", "D 1 0\n", "dd 0\n", "1 0 d d 0\n"] {
        assert!(matches!(proof_err(bad), ParseError::BadProofToken { .. }), "{bad:?}");
    }
    // bad integers
    for tok in ["+1", "-0", "+0", "--1", "1.5", "0x1", "1e0", "99999999999999999999", "-99999999999999999999", "9223372036854775808", "x", "1a", "−1"] {
        match proof_err(&format!("{tok} 0\n")) {
            ParseError::BadProofToken { line: 1, token } => assert_eq!(token, tok),
            other => panic!("{tok:?}: {other:?}"),
        }
    }
}

#[test]
fn p3_p4_truncation_and_limits() {
    assert_eq!(proof_err("1 0\n2"), ParseError::Truncated { line: 2 });
    assert_eq!(proof_err("1 0\nd"), ParseError::Truncated { line: 2 });
    assert_eq!(proof_err("1 0\nd 1"), ParseError::Truncated { line: 2 });
    assert_eq!(proof_err("0\n\n\n1"), ParseError::Truncated { line: 4 });
    // truncation after the empty clause still rejects (P3 / D3)
    assert_eq!(proof_err("1 0\n0\n1 2"), ParseError::Truncated { line: 3 });
    // junk after the empty clause still rejects (P5 / D3)
    assert!(matches!(proof_err("1 0\n0\nfoo\n"), ParseError::BadProofToken { line: 3, .. }));
    // ... while well-formed steps after it are never checked
    assert!(check(XOR, "1 0\n0\n5 6 7 0\nd 9 0\n").is_ok());
    // P4
    let over = DEFAULT_MAX_VARS as i64 + 1;
    assert_eq!(
        proof_err(&format!("{over} 0\n")),
        ParseError::ProofVarOverLimit { line: 1, literal: over, limit: DEFAULT_MAX_VARS }
    );
    assert_eq!(
        proof_err(&format!("1 0\nd -{over} 0\n")),
        ParseError::ProofVarOverLimit { line: 2, literal: -over, limit: DEFAULT_MAX_VARS }
    );
    // exactly at the limit is accepted (and checkable: table is ~60 MB).
    let at = DEFAULT_MAX_VARS;
    assert!(check(XOR, &format!("{at} 0\n1 0\n0\n")).is_ok());
    // a deletion naming a huge (in-limit) variable allocates tables too, and
    // is then DeleteAbsent: D5.
    assert_eq!(check(XOR, &format!("d {at} 0\n")), Err(Rejection::DeleteAbsent { step: 0, line: 1 }));
    // parse_proof is public and obeys its own limit
    assert!(parse_proof("3 0\n", 2).is_err());
    assert!(parse_proof("2 0\n", 2).is_ok());
    assert!(parse_formula("p cnf 3 0\n", 2).is_err());
}

#[test]
fn p6_lines_and_steps_in_errors() {
    let r = check(XOR, "c c\n1 0\n\n d 1\n 3 0\n7 0\n0\n");
    // line 5 holds the terminating 0 of `d 1 3`, which is absent.
    assert_eq!(r, Err(Rejection::DeleteAbsent { step: 1, line: 5 }));
    // (2 -1) is RUP once (1) is in: 1 true forces 2, which the lemma denies.
    assert_eq!(check(XOR, "1 0\n\n\n2 -1 0\n"), Err(Rejection::NoEmptyClause));
    assert_eq!(check(XOR, "1 0\n\n\n-1 -2 -2 0\n3 0\n"), Err(Rejection::NoEmptyClause));
    let ok = check(XOR, "1 0\nc\n\n0\n").unwrap();
    assert_eq!((ok.lemmas, ok.deletions, ok.rat_lemmas, ok.empty_clause_line), (2, 0, 0, 4));
}

// ---------------------------------------------------------------------------
// RUP / RAT / deletion semantics
// ---------------------------------------------------------------------------

#[test]
fn c2_rup_cases() {
    // implication chain: 1, 1->2, 2->3, 3->4: lemma (4) is RUP.
    let f = "p cnf 4 4\n1 0\n-1 2 0\n-2 3 0\n-3 4 0\n";
    assert_eq!(check(f, "4 0\n"), Err(Rejection::NoEmptyClause));
    assert_eq!(check(f, "3 0\n"), Err(Rejection::NoEmptyClause));
    assert_eq!(check(f, "-4 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    // lemma literals order and repetition do not matter
    assert_eq!(check(f, "4 4 3 0\n"), Err(Rejection::NoEmptyClause));
    // a tautological lemma is RUP (and may be added)
    assert_eq!(check(f, "-4 4 0\n"), Err(Rejection::NoEmptyClause));
    // unit propagation needs *all* premises: drop the middle one
    let g = "p cnf 4 3\n1 0\n-1 2 0\n-3 4 0\n";
    assert_eq!(check(g, "3 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    // conflict through a unit clash alone: 1 and -1 both units
    assert!(check("p cnf 1 2\n1 0\n-1 0\n", "0\n").is_ok());
    // empty clause is not RUP in a SAT formula
    assert_eq!(check("p cnf 2 1\n1 2 0\n", "0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    // a clause with a duplicate literal propagates as a unit
    assert!(check("p cnf 2 3\n1 1 0\n-1 2 0\n-2 -2 0\n", "0\n").is_ok());
    // a satisfied-by-lemma clause does not conflict: lemma (1 2), db (-1 2)(1 -2)
    assert_eq!(check("p cnf 2 2\n-1 2 0\n1 -2 0\n", "1 2 0\n0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
}

#[test]
fn c3_rat_cases() {
    // docs example: db (-x a). Lemma (x) is not RAT; with unit (a) it is.
    let weak = "p cnf 2 1\n-1 2 0\n";
    assert_eq!(check(weak, "1 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    let strong = "p cnf 2 2\n-1 2 0\n2 0\n";
    assert_eq!(check(strong, "1 0\n"), Err(Rejection::NoEmptyClause)); // RAT accepted, no empty clause
    // the counters: RAT recorded exactly once
    // fresh variable x5: no clause contains -5, so (5 0) and (5 3 0) are RAT
    assert_eq!(check(XOR, "5 0\n"), Err(Rejection::NoEmptyClause));
    let ok = check(XOR, "5 0\n-5 1 0\n1 0\n0\n").unwrap();
    assert_eq!(ok.rat_lemmas, 1, "only (5) needs RAT; the rest are RUP: {ok:?}");
    // the *first* literal is the pivot, as written: (-1 5) pivots on -1.
    // db: (1 2) (1 -2) ... clauses containing +1 are partners of pivot -1.
    // lemma (-1 5): resolvents with (1 2): (-1 5 2) ; with (1 -2): (-1 5 -2).
    // Neither is RUP in {(1 2),(1 -2)} so it is rejected as written ...
    let g = "p cnf 2 2\n1 2 0\n1 -2 0\n";
    assert_eq!(check(g, "-1 5 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    // ... but with the fresh variable first the same clause is RAT.
    assert_eq!(check(g, "5 -1 0\n"), Err(Rejection::NoEmptyClause));
    // RAT is checked against the database BEFORE the lemma: a lemma cannot
    // justify itself. (x1) with partner (-1 2): resolvent (1 2) must be RUP
    // without (1) in the database.
    assert_eq!(check("p cnf 2 1\n-1 2 0\n", "1 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    // The pivot is the first literal only: lemma (2 1) pivots on 2, whose
    // partner (-2 1) gives the resolvent (2 1), the lemma itself, not RUP.
    let h = "p cnf 2 2\n-2 1 0\n-1 2 0\n";
    assert_eq!(check(h, "2 1 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    // RAT with a tautological resolvent is fine: pivot 1, partner (-1 -3 ...) etc.
    // lemma (1 3), partner (-1 -3): resolvent (1 3 -3) is a tautology: RUP.
    assert_eq!(check("p cnf 3 1\n-1 -3 0\n", "1 3 0\n"), Err(Rejection::NoEmptyClause));
    // RAT keeps SAT: after the RAT lemma, a *wrong* empty clause is refused.
    assert_eq!(check("p cnf 3 1\n-1 -3 0\n", "1 3 0\n0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    // the empty clause is never RAT, even with no clauses at all.
    assert_eq!(check("p cnf 0 0\n", "0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
}

#[test]
fn c3_rat_is_not_confused_by_deleted_partners() {
    // (-1 2) is a partner for lemma (1); delete it and (1) becomes RAT.
    let f = "p cnf 2 1\n-1 2 0\n";
    assert_eq!(check(f, "1 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    assert_eq!(check(f, "d -1 2 0\n1 0\n"), Err(Rejection::NoEmptyClause));
    // and a partner added by an earlier lemma counts as live: (-3 1) is RAT
    // (nothing contains +3), and then it blocks (3), whose resolvent (3 1) is
    // not RUP; deleting it first unblocks (3).
    let g = "p cnf 3 1\n1 2 0\n";
    assert_eq!(check(g, "-3 1 0\n3 0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    assert_eq!(check(g, "-3 1 0\nd -3 1 0\n3 0\n"), Err(Rejection::NoEmptyClause));
}

#[test]
fn c5_deletion_rules() {
    // set matching: permuted, duplicated literals
    // Without (1 2), the lemma (1) is neither RUP nor RAT any more.
    assert_eq!(check(XOR, "d 2 1 0\n1 0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    assert_eq!(check(XOR, "d 2 2 1 1 0\n1 0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    assert_eq!(check(XOR, "d 2 2 1 1 0\nd -2 1 0\n"), Err(Rejection::NoEmptyClause));
    // deletion of an absent clause, or a sub/super set
    assert_eq!(check(XOR, "d 1 0\n"), Err(Rejection::DeleteAbsent { step: 0, line: 1 }));
    assert_eq!(check(XOR, "d 1 2 -1 0\n"), Err(Rejection::DeleteAbsent { step: 0, line: 1 }));
    assert_eq!(check(XOR, "d 0\n"), Err(Rejection::DeleteAbsent { step: 0, line: 1 }));
    // a lemma may be deleted exactly as many times as it was added
    assert_eq!(check(XOR, "1 0\nd 1 0\nd 1 0\n"), Err(Rejection::DeleteAbsent { step: 2, line: 3 }));
    assert_eq!(check(XOR, "1 0\n1 0\nd 1 0\nd 1 0\n"), Err(Rejection::NoEmptyClause));
    // a deleted lemma really is gone: the empty clause needs (1) again
    assert!(check(XOR, "1 0\nd 1 0\n0\n").is_err());
    assert!(check(XOR, "1 0\n0\n").is_ok());
}

#[test]
fn c5_deleting_unit_and_reason_clauses_takes_effect() {
    // 1, 1->2, -2: UNSAT by UP alone.
    let f = "p cnf 2 3\n1 0\n-1 2 0\n-2 0\n";
    assert!(check(f, "0\n").is_ok());
    // delete the unit (1): propagation no longer reaches a conflict.
    assert_eq!(check(f, "d 1 0\n0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    // delete the unit (-2): likewise
    assert_eq!(check(f, "d -2 0\n0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    // delete the reason clause (-1 2)
    assert_eq!(check(f, "d -1 2 0\n0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    // derive the unit (2) first (RUP), delete its reason (-1 2) and the unit
    // (1) it came from: (2) alone still conflicts with (-2). Deletion of a
    // clause that was a reason must not leave stale propagation behind.
    assert!(check(f, "2 0\nd -1 2 0\nd 1 0\n0\n").is_ok());
    // ... and with (2) *also* deleted, nothing is left to conflict.
    assert_eq!(check(f, "2 0\nd -1 2 0\nd 1 0\nd 2 0\n0\n"), Err(Rejection::NotImplied { step: 4, line: 5 }));
    // re-adding a deleted unit (it is RAT-fine / RUP) restores propagation
    assert!(check(f, "d 1 0\n1 0\n0\n").is_err()); // (1) is not RUP once deleted
    assert!(check("p cnf 2 3\n1 0\n-1 2 0\n-2 0\n", "2 0\nd 1 0\nd -1 2 0\n0\n").is_ok());
}

#[test]
fn c6_termination_rules() {
    assert_eq!(check(XOR, ""), Err(Rejection::NoEmptyClause));
    assert_eq!(check(XOR, "c nothing\n\n"), Err(Rejection::NoEmptyClause));
    assert_eq!(check(XOR, "1 0\n"), Err(Rejection::NoEmptyClause));
    assert_eq!(check(XOR, "1 0\n2 0\n"), Err(Rejection::NoEmptyClause));
    assert_eq!(check(XOR, "1 0\n1 0\nd 1 0\n"), Err(Rejection::NoEmptyClause));
    // an empty proof for a formula that unit propagation already refutes
    assert_eq!(check("p cnf 1 2\n1 0\n-1 0\n", ""), Err(Rejection::NoEmptyClause));
    // the empty clause first, when it is already RUP
    assert!(check("p cnf 1 2\n1 0\n-1 0\n", "0\n").is_ok());
    // and when it is not
    assert_eq!(check(XOR, "0\n1 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
}

#[test]
fn c4_ordering_and_missing_premises() {
    // lemma order: (2) needs (1) first.
    // (1) is not unit-derivable from the four ternary clauses, but it is from
    // (1 2) and (1 -2), which are; and the empty clause needs (1).
    let g = "p cnf 4 6\n1 2 3 0\n1 2 -3 0\n1 -2 3 0\n1 -2 -3 0\n-1 4 0\n-1 -4 0\n";
    assert!(check(g, "1 2 0\n1 -2 0\n1 0\n0\n").is_ok());
    // premises must come first: (1) before its support, or 0 before (1)
    assert_eq!(check(g, "1 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    assert_eq!(check(g, "0\n1 2 0\n1 -2 0\n1 0\n"), Err(Rejection::NotImplied { step: 0, line: 1 }));
    assert_eq!(check(g, "1 2 0\n0\n"), Err(Rejection::NotImplied { step: 1, line: 2 }));
    // formula clause order does not matter
    let g2 = "p cnf 4 6\n-1 -4 0\n1 -2 -3 0\n-1 4 0\n1 -2 3 0\n1 2 -3 0\n1 2 3 0\n";
    assert!(check(g2, "1 2 0\n1 -2 0\n1 0\n0\n").is_ok());
}

// ---------------------------------------------------------------------------
// Resource limits
// ---------------------------------------------------------------------------

/// A valid proof that costs real work: pigeonhole(3 pigeons, 2 holes) with a
/// hand-written refutation (verified accepted below).
fn php32() -> (String, String) {
    // variables p_{i,h} = 2(i-1)+h : 1..6
    let f = "p cnf 6 9\n1 2 0\n3 4 0\n5 6 0\n-1 -3 0\n-1 -5 0\n-3 -5 0\n-2 -4 0\n-2 -6 0\n-4 -6 0\n";
    (f.to_string(), "-1 0\n-2 0\n0\n".to_string())
}

#[test]
fn c8_work_budget_sweep_never_accepts_more_than_unlimited() {
    let (f, p) = php32();
    let full = check(&f, &p).expect("php(3,2) refutation must verify");
    let mut accepted_over_budget = 0;
    let mut saw_limit = 0;
    for budget in 0..=full.work + 3 {
        let r = check_with_limits(&f, &p, Limits { max_work: budget, ..Limits::default() });
        match r {
            Ok(a) => {
                // same certificate, same counts as the unlimited run
                assert_eq!((a.lemmas, a.rat_lemmas, a.deletions), (full.lemmas, full.rat_lemmas, full.deletions));
                // C8: an acceptance never reports more work than the budget.
                if a.work > budget {
                    accepted_over_budget += 1;
                }
            }
            Err(Rejection::ResourceLimit { .. }) => saw_limit += 1,
            Err(other) => panic!("budget {budget}: a valid proof must only ever hit ResourceLimit, got {other:?}"),
        }
        // monotone: once a budget accepts, every larger budget accepts
        if r_is_ok(&f, &p, budget) {
            assert!(r_is_ok(&f, &p, budget + 1));
        }
    }
    assert!(saw_limit > 0, "tiny budgets must be refused");
    assert_eq!(accepted_over_budget, 0, "Accepted::work must never exceed Limits::max_work");
    println!("php32 full work={}", full.work);
    // budget 0 can never verify a non-trivial proof
    assert!(check_with_limits(&f, &p, Limits { max_work: 0, ..Limits::default() }).is_err());
}

fn r_is_ok(f: &str, p: &str, budget: u64) -> bool {
    check_with_limits(f, p, Limits { max_work: budget, ..Limits::default() }).is_ok()
}

/// Every budget, one after the other, on a proof whose only lemma needs the
/// RAT check with two partner clauses. Somewhere in the sweep the budget runs
/// out *while a resolvent is being built* (the charge `is_rat` makes before
/// calling `is_rup`); the verdict must then be `ResourceLimit`, never an
/// acceptance, and never `NotImplied` (the lemma is valid RAT).
#[test]
fn c8_budget_sweep_through_the_rat_check() {
    // x1 is RAT: partners (-1 2) and (-1 3) give resolvents (1 2), (1 3),
    // both RUP through the units 2 and 3. The formula is satisfiable.
    let f = "p cnf 3 4\n-1 2 0\n2 0\n-1 3 0\n3 0\n";
    let p = "1 0\n";
    assert_eq!(check_with_limits(f, p, Limits::default()), Err(Rejection::NoEmptyClause));
    let (mut limit, mut done) = (0, 0);
    for budget in 0..400 {
        match check_with_limits(f, p, Limits { max_work: budget, ..Limits::default() }) {
            Err(Rejection::ResourceLimit { step: 0, line: 1 }) => limit += 1,
            Err(Rejection::NoEmptyClause) => done += 1,
            other => panic!("budget {budget}: {other:?}"),
        }
    }
    assert!(limit > 10 && done > 10, "limit={limit} done={done}");
}

#[test]
fn c8_budget_on_invalid_proofs_still_rejects() {
    // A satisfiable formula: nothing is implied, so no budget may accept.
    let f = "p cnf 3 2\n1 2 0\n-1 3 0\n";
    for budget in [0u64, 1, 5, 50, 500, u64::MAX] {
        for proof in ["1 0\n", "1 0\n0\n", "0\n", "-3 0\n0\n"] {
            let r = check_with_limits(f, proof, Limits { max_work: budget, ..Limits::default() });
            assert!(r.is_err(), "budget {budget}, proof {proof:?}");
        }
    }
}

#[test]
fn c8_big_inputs_stay_inside_limits_and_finish() {
    // A 100k-variable implication chain, refuted by one lemma-free step:
    // x1, x1->x2, ..., x_{n-1}->x_n, -x_n. One RUP check walks the chain.
    let n = 100_000;
    let mut f = format!("p cnf {n} {}\n1 0\n-{n} 0\n", n + 1);
    for i in 1..n {
        f.push_str(&format!("-{i} {} 0\n", i + 1));
    }
    let ok = check(&f, "0\n").unwrap();
    assert!(ok.work >= n as u64);
    // the same chain under a budget smaller than the chain: ResourceLimit
    assert_eq!(
        check_with_limits(&f, "0\n", Limits { max_work: n as u64 / 2, ..Limits::default() }),
        Err(Rejection::ResourceLimit { step: 0, line: 1 })
    );
    // a single clause with 200k literals, in the proof
    let lits: Vec<String> = (1..=200_000).map(|i| i.to_string()).collect();
    let proof = format!("{} 0\n", lits.join(" "));
    // (no clause contains a negated literal, so it is vacuously RAT)
    assert_eq!(check_with_limit("p cnf 1 0\n", &proof, 1 << 20), Err(Rejection::NoEmptyClause));
}
