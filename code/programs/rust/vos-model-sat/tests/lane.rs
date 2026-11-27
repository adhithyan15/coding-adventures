//! VOS-040b: the lane (solve, DRAT, control, witness, collapse, table) and its negative controls.
//! Every finding `MS01` to `MS10` has a test that makes it fire; the evaluator's answers are
//! computed once per test binary (`common::evidence`).

mod common;

use std::sync::OnceLock;

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv::BitVec;
use cdcl_sat::{Cnf, Var};
use common::{contract_dir, evidence, files};
use vos_model_sat::labels::{FORBIDDEN_WORDS, LABEL_TRIVIAL, NO_SHIPPED_PHRASE};
use vos_model_sat::lane::{
    analyse_with, case_number, check_with, first_difference, recorded_collapses, replay, Analysis,
    LaneConfig, LADDER,
};
use vos_model_sat::lemmas::LemmaId;
use vos_model_sat::model::{Encoder, SchedModel};
use vos_model_sat::mutants::{Fault, Faulty, Mutant};
use vos_model_sat::oracle::{load, ContractFiles};
use vos_model_sat::solve::{
    check_default, model_satisfies, propagation_refutes, solve_cnf, spec_states_budget, Budget,
    Verdict, ROUTE_MAX_CHECK_WORK, ROUTE_MAX_CONFLICTS,
};
use vos_model_sat::types::Ready;

fn fresh() -> ContractFiles {
    load(&contract_dir(), None).expect("the real contract loads")
}

fn real_cfg() -> LaneConfig<'static> {
    LaneConfig::real()
}

/// The real lane, once per binary.
fn real() -> &'static Analysis {
    static A: OnceLock<Analysis> = OnceLock::new();
    A.get_or_init(|| analyse_with(files(), &real_cfg(), Some(evidence())).unwrap())
}

fn codes(a: &Analysis) -> Vec<&'static str> {
    a.findings.iter().map(|f| f.code).collect()
}

// ---- the real run -------------------------------------------------------------------------

#[test]
fn the_real_lane_certifies_every_obligation() {
    let a = real();
    assert!(a.findings.is_empty(), "{:?}", a.findings);
    assert_eq!(a.rows.len(), 40, "ten lemmas times four widths");
    for r in &a.rows {
        assert!(LADDER.contains(&r.n()));
        assert!(r.conflicts() <= ROUTE_MAX_CONFLICTS);
        assert!(r.check_work() <= ROUTE_MAX_CHECK_WORK);
        assert!(r.check_work() > 0, "an accepted proof has work");
    }
    assert!(a.table.is_some());
    // 36 counted, closure_at_m (4 rows) trivial and not counted (VOS31 7.5).
    assert_eq!(a.rows.iter().filter(|r| r.counted()).count(), 36);
    assert!(a
        .report
        .last()
        .unwrap()
        .contains("36 obligations certified and counted, 4 trivial and not counted"));
}

#[test]
fn rows_follow_lemmas_part_order_then_n_ascending() {
    let a = real();
    let order: Vec<(&str, usize)> = a.rows.iter().map(|r| (r.lemma().name(), r.n())).collect();
    let mut want = Vec::new();
    for l in LemmaId::ALL {
        for n in LADDER {
            want.push((l.name(), n));
        }
    }
    assert_eq!(order, want);
}

#[test]
fn the_table_has_the_header_the_rows_and_the_trivial_line() {
    let t = real().table.as_ref().unwrap();
    let lines: Vec<&str> = t.lines().collect();
    assert_eq!(lines[0], "vos-sat 1");
    // The two digests are the ones lemmas.table already records for the same two files: the same
    // sha256 of the same text.
    let table = files().table.as_ref().unwrap();
    for key in ["contract-part", "lemmas-part"] {
        let want = table.lines().find(|l| l.starts_with(key)).unwrap();
        assert!(lines.contains(&want), "{key}");
    }
    assert_eq!(
        lines[3],
        "route-budget conflicts 50000 check-work 2147483648"
    );
    assert_eq!(lines.len(), 4 + 40 + 2);
    assert_eq!(lines[lines.len() - 2], "trivial closure_at_m");
    assert_eq!(lines[lines.len() - 1], "end");
    assert!(lines[4].starts_with("lemma closure_at_m n 8 label trivial_(inv_is_true) "));
    assert!(lines[4].ends_with(" verdict unsat collapsed constant-fold control sat premise none"));
}

#[test]
fn the_table_commits_no_solver_dependent_field() {
    let t = real().table.as_ref().unwrap();
    for w in [
        "conflicts ",
        "check_work",
        "check-work-used",
        "drat",
        "accepted",
    ] {
        // `route-budget conflicts 50000 check-work ...` is the one header that names them.
        for l in t.lines().filter(|l| !l.starts_with("route-budget")) {
            assert!(!l.contains(w), "{l}");
        }
    }
    // (That `render_table` ignores conflicts and check_work is a unit test in src/lane.rs.)
}

#[test]
fn collapse_reasons_on_the_real_lemmas_are_the_re_derived_ones() {
    let a = real();
    let collapsed: Vec<(&str, usize, &str)> = a
        .rows
        .iter()
        .filter_map(|r| r.collapsed().map(|c| (r.lemma().name(), r.n(), c.as_str())))
        .collect();
    // closure_at_m and config_n_is_the_id_range fold on the honest encoder (VOS31 5.2); so does
    // policy_admits_only_top (found by running it: VOS31 section 7.5 expected propagation-only).
    let names: std::collections::BTreeSet<&str> = collapsed.iter().map(|c| c.0).collect();
    assert_eq!(
        names.into_iter().collect::<Vec<_>>(),
        [
            "closure_at_m",
            "config_n_is_the_id_range",
            "policy_admits_only_top"
        ]
    );
    assert!(collapsed.iter().all(|c| c.2 == "constant-fold"));
    // A collapsed row is labelled so, and still carries the SAT-certified text of its width.
    let r = a
        .rows
        .iter()
        .find(|r| r.lemma() == LemmaId::PolicyAdmitsOnlyTop && r.n() == 16)
        .unwrap();
    assert!(
        r.label().ends_with("; collapsed (constant-fold)"),
        "{}",
        r.label()
    );
    assert!(r.premise_witnessed(), "a collapse is not a vacuous premise");
}

#[test]
fn the_premise_witnesses_are_exactly_the_six_implications_the_parser_reports() {
    let a = real();
    let witnessed: std::collections::BTreeSet<&str> = a
        .rows
        .iter()
        .filter(|r| r.premise_witnessed())
        .map(|r| r.lemma().name())
        .collect();
    assert_eq!(
        witnessed.into_iter().collect::<Vec<_>>(),
        [
            "insert_outside_pre_changes_nothing",
            "insert_then_remove_restores",
            "policy_admits_only_top",
            "policy_implies_safety",
            "remove_outside_pre_changes_nothing",
            "safe_dispatch_only_removes_the_choice",
        ]
    );
    let parsed = vos_model_sat::oracle::claims_of(&files().base_document()).unwrap();
    assert_eq!(parsed.len(), 10);
    for (name, implies) in parsed {
        assert_eq!(
            LemmaId::from_name(&name).unwrap().has_premise(),
            implies,
            "{name}"
        );
    }
}

// ---- the labels ---------------------------------------------------------------------------

#[test]
fn no_label_or_forbidden_word_is_printed_on_a_run_that_failed_every_proof() {
    // Every proof corrupted: nothing is certified, so no row, no report line, no table.
    let hook = |_: &str| "1 0\n".to_string();
    let c = real_cfg();
    let c = c.with_proof_hook(&hook);
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(a.findings.iter().any(|f| f.code == "MS04"));
    assert!(a.table.is_none() && a.rows.is_empty() && a.proofs.is_empty());
    for l in &a.report {
        assert!(!l.contains("SAT-certified"), "{l}");
    }
    // The UNSAT lemmas with a rejected proof print nothing; the report has no certified row.
    assert!(a.report.iter().all(|l| !l.contains("lemma, SAT")));
}

#[test]
fn a_failed_lemma_gets_no_label_while_the_others_still_report() {
    // top_is_the_maximum has a counterexample under the off-by-one top (MS03): that lemma gets
    // no line at all, never "SAT-certified"; the rest of the run still reports.
    let honest = |n: usize| -> Box<dyn SchedModel> {
        Box::new(Faulty {
            fault: Fault::TopOffByOne,
            n,
        })
    };
    let c = real_cfg();
    let c = c.with_honest(&honest);
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(a.findings.iter().any(|f| f.code == "MS03"));
    assert!(a.table.is_none());
    assert!(
        !a.report.iter().any(|l| l.starts_with("top_is_the_maximum")),
        "a failed lemma was reported as certified"
    );
}

#[test]
fn printed_text_never_uses_a_forbidden_word() {
    let a = real();
    let mut texts: Vec<String> = a.report.clone();
    texts.push(a.table.clone().unwrap());
    for t in texts {
        // The one honest phrase says no component has the width; remove it (and its table
        // spelling) before looking.
        let t = t
            .replace(NO_SHIPPED_PHRASE, "")
            .replace(&NO_SHIPPED_PHRASE.replace(' ', "_"), "");
        for w in FORBIDDEN_WORDS {
            assert!(!t.contains(w), "`{w}` in {t}");
        }
    }
    assert!(a.report.iter().any(|l| l.contains(LABEL_TRIVIAL)));
}

#[test]
fn only_the_one_function_in_lane_rs_chooses_a_label() {
    // VOS31 11 / VOS-040a INFO: "SAT-certified" is emitted only after a DRAT-checked UNSAT and a
    // passing control.  The label constructors may be CALLED nowhere but `certified_label`,
    // which `obligation` reaches only after both gates.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let needles = ["label_for(", "label_collapsed(", "LABEL_N8"];
    for e in std::fs::read_dir(&src).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&p).unwrap();
        if name == "labels.rs" {
            continue;
        }
        if name == "lane.rs" {
            // The in-crate unit tests (after `#[cfg(test)]`) are not production code.
            let text = text.split("#[cfg(test)]").next().unwrap().to_string();
            let start = text.find("fn certified_label").unwrap();
            let end = start + text[start..].find("\n}\n").unwrap();
            let outside = format!("{}{}", &text[..start], &text[end..]);
            for n in needles {
                assert!(text[start..end].contains(n) || n == "LABEL_N8");
                assert!(!outside.contains(n), "{n} used outside certified_label");
            }
            // And `certified_label` has one caller: the end of `obligation`.
            assert_eq!(outside.matches("certified_label(").count(), 1);
        } else {
            for n in needles {
                assert!(!text.contains(n), "{n} used in {name}");
            }
        }
    }
}

// ---- the route budget ---------------------------------------------------------------------

fn vos30() -> String {
    let code = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();
    std::fs::read_to_string(code.join("specs").join("VOS30-proof-route-and-capacity.md")).unwrap()
}

#[test]
fn route_budget_is_the_one_vos30_states() {
    assert_eq!(ROUTE_MAX_CONFLICTS, 50_000);
    assert_eq!(ROUTE_MAX_CHECK_WORK, 1 << 31);
    spec_states_budget(&vos30(), Budget::ROUTE).unwrap();
}

#[test]
fn a_wrong_budget_constant_is_caught_against_vos30() {
    let spec = vos30();
    for wrong in [
        Budget {
            max_conflicts: 50_001,
            ..Budget::ROUTE
        },
        Budget {
            max_conflicts: 100_000,
            ..Budget::ROUTE
        },
        Budget {
            max_check_work: 1 << 30,
            ..Budget::ROUTE
        },
        Budget {
            max_check_work: 1 << 32,
            ..Budget::ROUTE
        },
        Budget {
            max_check_work: 2_000_000_000,
            ..Budget::ROUTE
        },
    ] {
        assert!(spec_states_budget(&spec, wrong).is_err(), "{wrong:?}");
    }
    // A spec that changed: the section says other numbers.
    let changed = spec
        .replace("**50,000 conflicts**", "**60,000 conflicts**")
        .replace("**2^31**", "**2^30**");
    assert!(spec_states_budget(&changed, Budget::ROUTE).is_err());
    assert!(spec_states_budget("no such section", Budget::ROUTE).is_err());
    // The text outside 8.6.1 does not count: remove the heading's section.
    let moved = spec.replace("#### 8.6.1 The route budget", "#### 8.6.1 Something else");
    assert!(spec_states_budget(&moved, Budget::ROUTE).is_err());
}

#[test]
fn an_over_budget_obligation_is_ms05() {
    // Too few conflicts: top_is_the_maximum at N = 8 needs 57.
    let c = real_cfg().with_budget(Budget {
        max_conflicts: 5,
        ..Budget::ROUTE
    });
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(codes(&a).contains(&"MS05"), "{:?}", a.findings);
    assert!(a.table.is_none());
    // Too little checker work.
    let c = real_cfg().with_budget(Budget {
        max_check_work: 100,
        ..Budget::ROUTE
    });
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(
        a.findings
            .iter()
            .any(|f| f.code == "MS05" && f.message.contains("check_work")),
        "{:?}",
        a.findings
    );
}

// ---- the findings -------------------------------------------------------------------------

#[test]
fn ms01_vos_toml_must_opt_in() {
    let c = LaneConfig::real();
    // The real contract's vos.toml carries `sat = "model"` (VOS-040c), so it passes MS01.
    assert!(real().findings.is_empty(), "{:?}", real().findings);
    // A missing key, a wrong value, a duplicate, a non-string and a missing file are MS01 too;
    // the right value (with a comment) is not.  The parser is the kit's own.
    let mut f = fresh();
    for toml in [
        "class = \"K\"
",
        "class = \"K\"
sat = \"none\"
",
        "class = \"K\"
sat = \"models\"
",
        "sat = \"model\"
sat = \"model\"
",
        "sat = true
",
        "# sat = \"model\"
",
        "unknown = 1
sat = \"model\"
",
    ] {
        f.vos_toml = Some(toml.to_string());
        let a = analyse_with(&f, &c, Some(evidence())).unwrap();
        assert_eq!(codes(&a), ["MS01"], "{toml:?}");
        assert!(a.table.is_none());
    }
    f.vos_toml = None;
    let a = analyse_with(&f, &c, Some(evidence())).unwrap();
    assert!(a.findings[0].message.contains("not found"));
    f.vos_toml = Some(
        "class = \"K\"
sat = \"model\" # opt in
"
        .to_string(),
    );
    assert!(analyse_with(&f, &c, Some(evidence()))
        .unwrap()
        .findings
        .is_empty());
}

#[test]
fn ms01_n_and_lemma_count_are_bounded() {
    let ladder = [8usize, 65];
    let c = real_cfg().with_ladder(&ladder);
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert_eq!(codes(&a), ["MS01"]);
    let zero = [0usize];
    let c = real_cfg().with_ladder(&zero);
    assert_eq!(
        codes(&analyse_with(files(), &c, Some(evidence())).unwrap()),
        ["MS01"]
    );
    // 65 claims: over the limit of 64 (CK03).
    let mut f = fresh();
    f.lemmas_text = (0..65)
        .map(|i| format!("  claim c{i}:\n    forall b: Bool. (b == b)\n"))
        .collect();
    let a = analyse_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert_eq!(codes(&a), ["MS01"]);
    assert!(a.findings[0].message.contains("65 claims"));
}

#[test]
fn ms02_an_encoder_that_disagrees_with_the_evaluator() {
    // The contract text says t < 9u8 in pre_insert; the encoder and the mutants are unchanged.
    let mut f = fresh();
    let needle = "model pre_insert(m: Ready, t: U8) -> Bool = t < 8u8";
    assert!(f.contract_text.contains(needle));
    f.contract_text = f.contract_text.replace(
        needle,
        "model pre_insert(m: Ready, t: U8) -> Bool = t < 9u8",
    );
    let a = analyse_with(&f, &real_cfg(), None).unwrap();
    assert!(codes(&a).contains(&"MS02"), "{:?}", a.findings);
    assert!(a.table.is_none());
}

#[test]
fn ms03_a_lemma_that_is_not_unsat() {
    for fault in [Fault::TopOffByOne, Fault::PreInsertLe, Fault::PutOnlySets] {
        let honest = move |n: usize| -> Box<dyn SchedModel> { Box::new(Faulty { fault, n }) };
        let c = real_cfg();
        let c = c.with_honest(&honest);
        let a = analyse_with(files(), &c, Some(evidence())).unwrap();
        assert!(
            a.findings
                .iter()
                .any(|f| f.code == "MS03" && f.message.contains("counterexample")),
            "{fault:?}: {:?}",
            a.findings
        );
        assert!(a.table.is_none() && a.rows.is_empty());
    }
}

#[test]
fn ms04_a_proof_the_default_limits_reject() {
    for bad in ["1 0\n", "", "this is not drat", "d 1 2 0\n"] {
        let hook = move |_: &str| bad.to_string();
        let c = real_cfg();
        let c = c.with_proof_hook(&hook);
        let a = analyse_with(files(), &c, Some(evidence())).unwrap();
        assert!(
            a.findings
                .iter()
                .any(|f| f.code == "MS04" && f.message.contains("default limits")),
            "{bad:?}: {:?}",
            a.findings
        );
    }
    // The checker itself, on a real obligation: the genuine proof is accepted, a truncated one
    // is not.
    let mut cnf = Cnf::new(0);
    let x = Var::from_index(0);
    cnf.add_clause(&[x.positive()]);
    cnf.add_clause(&[x.negative()]);
    let out = solve_cnf(&cnf, 100, true);
    assert_eq!(out.verdict, Verdict::Unsat);
    let good = check_default(&cnf, out.drat.as_ref().unwrap());
    assert!(good.accepted && good.work.is_some());
    let bad = check_default(&cnf, "");
    assert!(!bad.accepted && bad.reason.is_some() && bad.work.is_none());
}

#[test]
fn ms06_a_control_that_is_unsat_or_does_not_replay() {
    // The "mutant" is the honest encoder: its negation is UNSAT, so the lemma proved nothing.
    let mutant = |_: LemmaId, n: usize| -> Box<dyn SchedModel> { Box::new(Encoder { n }) };
    let c = real_cfg();
    let c = c.with_mutant(&mutant);
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(
        a.findings
            .iter()
            .any(|f| f.code == "MS06" && f.message.contains("UNSAT")),
        "{:?}",
        a.findings
    );
    assert!(a.table.is_none());

    // A control that is SAT but is NOT the mutant of the part: at N = 8 its counterexample must be
    // a failing case of the mutant document in the Warrant evaluator, and is not.
    let wrong = |l: LemmaId, n: usize| -> Box<dyn SchedModel> {
        if l == LemmaId::InsertThenRemoveRestores {
            Box::new(Faulty {
                fault: Fault::PutOnlySets,
                n,
            })
        } else {
            Box::new(Mutant { lemma: l, n })
        }
    };
    let c = real_cfg();
    let c = c.with_mutant(&wrong);
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(
        a.findings.iter().any(|f| f.code == "MS06"
            && f.message
                .contains("not a failing case of the mutant document")),
        "{:?}",
        a.findings
    );
    // Only N = 8 is replayed through the evaluator; the other widths of that lemma pass.
    assert_eq!(
        a.findings.iter().filter(|f| f.code == "MS06").count(),
        1,
        "{:?}",
        a.findings
    );

    // An over-budget control is MS06 as well.
    let c = real_cfg().with_budget(Budget {
        max_conflicts: 0,
        ..Budget::ROUTE
    });
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(a.findings.iter().any(|f| f.code == "MS06"));
}

#[test]
fn replay_rejects_a_model_that_is_not_a_counterexample() {
    // not (x and y): a model of the negation has x and y both true.
    let mut aig = Aig::new();
    let (x, y) = (aig.input(), aig.input());
    let holds = !aig.and(x, y);
    let mut cnf = Cnf::new(0);
    let mut ts = bit_blaster::tseitin::Tseitin::new();
    ts.assert_true(&aig, !holds, &mut cnf);
    let Verdict::Sat(m) = solve_cnf(&cnf, 100, false).verdict else {
        panic!("sat expected")
    };
    let inputs = replay(&aig, &ts, &cnf, &m, holds, false).unwrap();
    assert_eq!(inputs, [true, true]);
    assert_eq!(case_number(&inputs), 3);
    // A model that does not satisfy the CNF.
    let bad = vec![false; m.len()];
    assert!(replay(&aig, &ts, &cnf, &bad, holds, false)
        .unwrap_err()
        .contains("does not satisfy"));
    // A model that satisfies the CNF but whose circuit inputs say otherwise: expect the edge
    // true when it is false.
    assert!(replay(&aig, &ts, &cnf, &m, holds, true)
        .unwrap_err()
        .contains("does not replay"));
    assert!(model_satisfies(&cnf, &m));
}

struct PreAlwaysTrue(Encoder);

impl SchedModel for PreAlwaysTrue {
    fn n(&self) -> usize {
        self.0.n
    }
    fn pre_insert(&self, _: &mut Aig, _: &Ready, _: &BitVec) -> Edge {
        Edge::constant(true)
    }
}

#[test]
fn ms07_a_vacuous_premise_is_not_witnessed() {
    // pre_insert is true everywhere, so `not pre_insert` (the premise of
    // insert_outside_pre_changes_nothing) is unsatisfiable: the implication is vacuous.
    let honest = |n: usize| -> Box<dyn SchedModel> { Box::new(PreAlwaysTrue(Encoder { n })) };
    let c = real_cfg();
    let c = c.with_honest(&honest);
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert!(
        a.findings
            .iter()
            .any(|f| f.code == "MS07" && f.message.contains("vacuous")),
        "{:?}",
        a.findings
    );
    assert!(a.table.is_none());
}

#[test]
fn ms07_the_parser_not_the_encoder_decides_which_lemmas_are_implications() {
    // lemmas.part now says config_n_is_the_id_range is an implication; the encoder has no premise
    // for it, so the encoder cannot declare itself exempt.
    let mut f = fresh();
    let old = "forall t: U8. pre_insert(empty(), t) == (t < cfg_N())";
    assert!(f.lemmas_text.contains(old));
    f.lemmas_text = f.lemmas_text.replace(
        old,
        "forall t: U8. (t < 255u8) => (pre_insert(empty(), t) == (t < cfg_N()))",
    );
    let a = analyse_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert!(
        a.findings
            .iter()
            .any(|f| f.code == "MS07" && f.message.contains("encoder has no premise")),
        "{:?}",
        a.findings
    );
    // And the converse: an encoder premise for a claim the parser says is not an implication.
    let mut g = fresh();
    let flat = "not pre_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t) =>";
    assert!(g.lemmas_text.contains(flat));
    g.lemmas_text = g.lemmas_text.replacen(
        flat,
        "not pre_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t) and true and",
        1,
    );
    let a = analyse_with(&g, &real_cfg(), Some(evidence())).unwrap();
    assert!(
        a.findings
            .iter()
            .any(|f| f.code == "MS07" && f.message.contains("not an implication")),
        "{:?}",
        a.findings
    );
}

#[test]
fn ms08_a_collapse_that_does_not_re_derive() {
    // The committed table records a collapse that the regenerated obligation does not have.
    let table = real().table.clone().unwrap();
    let mut f = fresh();
    let line = table
        .lines()
        .find(|l| l.starts_with("lemma top_is_the_maximum n 8 "))
        .unwrap();
    let tampered = table.replace(
        line,
        &line.replace("collapsed no", "collapsed propagation-only"),
    );
    f.sat_table = Some(tampered);
    let a = check_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert!(codes(&a).contains(&"MS08"), "{:?}", a.findings);
    assert!(codes(&a).contains(&"MS09"), "stale as well");
    // A reason outside the closed enum can never re-derive.
    let tampered = table.replace(
        line,
        &line.replace("collapsed no", "collapsed frame-untouched"),
    );
    f.sat_table = Some(tampered);
    let a = check_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert!(a
        .findings
        .iter()
        .any(|f| f.code == "MS08" && f.message.contains("frame-untouched")));
    // A recorded reason that DOES hold is not a finding.
    f.sat_table = Some(table.clone());
    assert!(check_with(&f, &real_cfg(), Some(evidence()))
        .unwrap()
        .findings
        .is_empty());
    assert_eq!(recorded_collapses(&table).len(), 12);
}

#[test]
fn ms08_a_refutation_with_no_conflict_and_no_re_derivable_reason() {
    // An encoder whose lemma is UNSAT at once but neither folds in the AIG nor falls to the
    // propagator would be a collapse nobody can explain.  The honest lane has none: every row
    // with 0 conflicts re-derives.
    for r in &real().rows {
        assert!(
            r.conflicts() > 0 || r.collapsed().is_some(),
            "{} {}",
            r.lemma().name(),
            r.n()
        );
    }
}

#[test]
fn ms09_the_table_is_absent_stale_or_has_another_budget() {
    let table = real().table.clone().unwrap();
    let mut f = fresh();
    f.sat_table = None;
    let a = check_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert_eq!(codes(&a), ["MS09"]);
    assert!(a.findings[0].message.contains("absent"));
    // Current.
    f.sat_table = Some(table.clone());
    assert!(check_with(&f, &real_cfg(), Some(evidence()))
        .unwrap()
        .findings
        .is_empty());
    // A tampered digest.
    let mut bytes = table.clone().into_bytes();
    let at = table.find("contract-part ").unwrap() + 14;
    bytes[at] = if bytes[at] == b'0' { b'1' } else { b'0' };
    f.sat_table = Some(String::from_utf8(bytes).unwrap());
    let a = check_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert_eq!(codes(&a), ["MS09"]);
    assert!(a.findings[0].message.contains("line 2"), "{:?}", a.findings);
    // A table written under another budget constant.
    f.sat_table = Some(table.replace("conflicts 50000", "conflicts 60000"));
    let a = check_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert!(a.findings[0].message.contains("line 4"), "{:?}", a.findings);
    // A dropped row, an extra trailing line.
    f.sat_table = Some(table.replace("trivial closure_at_m\n", ""));
    assert_eq!(
        codes(&check_with(&f, &real_cfg(), Some(evidence())).unwrap()),
        ["MS09"]
    );
    f.sat_table = Some(format!("{table}extra\n"));
    assert_eq!(
        codes(&check_with(&f, &real_cfg(), Some(evidence())).unwrap()),
        ["MS09"]
    );
    // A table that is fine except for CRLF line endings is still different bytes.
    f.sat_table = Some(table.replace('\n', "\r\n"));
    assert_eq!(
        codes(&check_with(&f, &real_cfg(), Some(evidence())).unwrap()),
        ["MS09"]
    );
}

#[test]
fn first_difference_names_the_line() {
    assert_eq!(
        first_difference("a\nb\n", "a\nc\n"),
        "line 2: committed `b`, regenerated `c`"
    );
    assert_eq!(
        first_difference("a\n", "a\nb\n"),
        "line 2: committed `<end of file>`, regenerated `b`"
    );
    assert!(first_difference("a\n", "a").contains("trailing newline"));
}

#[test]
fn ms10_a_lemma_with_no_row_and_none_ran() {
    // A claim the encoder does not know.
    let mut f = fresh();
    f.lemmas_text
        .push_str("  claim extra_lemma:\n    forall b: Bool. (b == b)\n");
    let a = analyse_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert!(a
        .findings
        .iter()
        .any(|f| f.code == "MS10" && f.message.contains("extra_lemma")));
    // A lemma of the encoder that lemmas.part dropped, and an empty lemmas.part: none ran.
    let mut g = fresh();
    g.lemmas_text.clear();
    let a = analyse_with(&g, &real_cfg(), Some(evidence())).unwrap();
    assert!(a.findings.iter().any(|f| f.message.contains("none ran")));
    assert!(a.table.is_none());
    let only_ten = a.findings.iter().filter(|f| f.code == "MS10").count();
    assert_eq!(
        only_ten,
        1 + 10,
        "none ran, plus each encoder lemma missing"
    );
}

// ---- the solver half ----------------------------------------------------------------------

fn pigeonhole(n: usize) -> Cnf {
    let var = |p: usize, h: usize| Var::from_index(p * n + h);
    let mut c = Cnf::new(0);
    for p in 0..=n {
        let lits: Vec<_> = (0..n).map(|h| var(p, h).positive()).collect();
        c.add_clause(&lits);
    }
    for h in 0..n {
        for p in 0..=n {
            for q in p + 1..=n {
                c.add_clause(&[var(p, h).negative(), var(q, h).negative()]);
            }
        }
    }
    c
}

#[test]
fn solve_certifies_a_hard_unsat_and_stops_at_the_conflict_bound() {
    let cnf = pigeonhole(5);
    let out = solve_cnf(&cnf, 50_000, true);
    assert_eq!(out.verdict, Verdict::Unsat);
    assert!(out.conflicts > 0);
    assert!(check_default(&cnf, out.drat.as_ref().unwrap()).accepted);
    // Without a proof request there is no proof.
    assert!(solve_cnf(&cnf, 50_000, false).drat.is_none());
    // Over the bound: Unknown, and no proof.
    let hard = pigeonhole(9);
    let out = solve_cnf(&hard, 10, true);
    assert_eq!(out.verdict, Verdict::Unknown);
    assert!(out.drat.is_none());
}

#[test]
fn sat_models_are_replayed_by_a_plain_evaluator() {
    let mut c = Cnf::new(0);
    let (x, y) = (Var::from_index(0), Var::from_index(1));
    c.add_clause(&[x.positive(), y.positive()]);
    c.add_clause(&[x.negative()]);
    let Verdict::Sat(m) = solve_cnf(&c, 100, false).verdict else {
        panic!()
    };
    assert!(model_satisfies(&c, &m));
    let mut bad = m.clone();
    bad[1] = false;
    assert!(!model_satisfies(&c, &bad));
    // A short model reads the missing variables as false.
    assert!(!model_satisfies(&c, &[]));
}

#[test]
fn propagation_refutes_only_what_unit_propagation_alone_refutes() {
    let (x, y, z) = (Var::from_index(0), Var::from_index(1), Var::from_index(2));
    // x; -x v y; -y: refuted by propagation.
    let mut c = Cnf::new(0);
    c.add_clause(&[x.positive()]);
    c.add_clause(&[x.negative(), y.positive()]);
    c.add_clause(&[y.negative()]);
    assert!(propagation_refutes(&c));
    // The empty clause.
    let mut e = Cnf::new(1);
    e.add_clause(&[]);
    assert!(propagation_refutes(&e));
    // Contradictory units found while draining the queue.
    let mut u = Cnf::new(0);
    u.add_clause(&[x.positive()]);
    u.add_clause(&[x.negative()]);
    assert!(propagation_refutes(&u));
    // Pigeonhole is UNSAT but needs search: propagation alone does not refute it.
    assert!(!propagation_refutes(&pigeonhole(3)));
    // A satisfiable formula is never refuted.
    let mut s = Cnf::new(0);
    s.add_clause(&[x.positive(), y.positive()]);
    s.add_clause(&[y.negative(), z.positive()]);
    assert!(!propagation_refutes(&s));
    // Propagation reaches a conflict through a chain: x; -x v y; -y v z; -z.
    let mut chain = Cnf::new(0);
    chain.add_clause(&[x.positive()]);
    chain.add_clause(&[x.negative(), y.positive()]);
    chain.add_clause(&[y.negative(), z.positive()]);
    chain.add_clause(&[z.negative()]);
    assert!(propagation_refutes(&chain));
    // The empty formula.
    assert!(!propagation_refutes(&Cnf::new(0)));
}

#[test]
fn propagation_agrees_with_the_constant_fold_on_the_real_collapses() {
    // For each collapsed row the CNF really is trivial: the propagator refutes it too.
    for r in real().rows.iter().filter(|r| r.collapsed().is_some()) {
        assert_eq!(r.vars(), 1, "{} {}", r.lemma().name(), r.n());
        assert_eq!(r.clauses(), 2);
    }
}

// ---- the review fixes ---------------------------------------------------------------------

#[test]
fn ms10_a_duplicate_claim_name_is_refused() {
    let mut f = fresh();
    f.lemmas_text
        .push_str("  claim closure_at_m:\n    forall b: Bool. (b == b)\n");
    let a = analyse_with(&f, &real_cfg(), Some(evidence())).unwrap();
    assert!(
        a.findings
            .iter()
            .any(|x| x.code == "MS10" && x.message.contains("more than once")),
        "{:?}",
        a.findings
    );
    assert!(a.table.is_none() && a.rows.is_empty());
}

#[test]
fn dimacs_and_drat_are_kept_only_when_asked() {
    assert!(
        real().proofs.is_empty(),
        "the default run keeps no proof text"
    );
    let c = real_cfg().keep_proofs();
    let a = analyse_with(files(), &c, Some(evidence())).unwrap();
    assert_eq!(a.proofs.len(), 40);
    assert!(a
        .proofs
        .iter()
        .all(|(_, cnf, drat)| !cnf.is_empty() && !drat.is_empty()));
}

#[test]
fn the_deleted_bridge_flag_is_wired_into_no_build_workflow_or_toml_file() {
    // VOS-040c deleted the 040b `--allow-missing-sat-key` bridge; this guards against its
    // reintroduction: CI fails if a BUILD file, a workflow or a .toml spells the flag.  `git grep --untracked` from the repository root.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .to_path_buf();
    let needle = ["allow-missing", "sat-key"].join("-");
    let out = std::process::Command::new("git")
        .current_dir(&root)
        .args(["grep", "--untracked", "-l", "-e", &needle, "--"])
        .args(["*BUILD*", "*.toml", "*.yml", "*.yaml"])
        .output();
    let Ok(out) = out else {
        eprintln!("git is not available: the bridge-flag scan is skipped");
        return;
    };
    // git grep exits 1 with no output when nothing matches, 128 on an error (not a repository).
    if out.status.code() == Some(128) {
        eprintln!("not a git checkout: the bridge-flag scan is skipped");
        return;
    }
    let hits = String::from_utf8_lossy(&out.stdout);
    assert!(
        hits.trim().is_empty(),
        "the bridge flag is passed by: {hits}"
    );
}

#[test]
fn the_budget_pin_is_anchored_to_exact_unique_lines() {
    let spec = vos30();
    // The first anchor line quoted twice in the section: no longer unique.
    let line = "**50,000 conflicts** (8.3 allows 100,000) **and** `check_work` is at most **2^31**";
    assert!(spec.contains(line));
    let twice = spec.replacen(line, &format!("{line}\n{line}"), 1);
    assert!(spec_states_budget(&twice, Budget::ROUTE).is_err());
    // The numbers only mentioned in passing in the section are not enough.
    let reworded = spec.replacen(
        line,
        "the bound is stated in prose: **50,000 conflicts** and **2^31** appear here only",
        1,
    );
    assert!(spec_states_budget(&reworded, Budget::ROUTE).is_err());
    // The second anchor.
    let broken = spec.replacen(
        "These two constants (50,000 conflicts, 2^31 check work)",
        "These two constants (50,000 conflicts, 2^30 check work)",
        1,
    );
    assert!(spec_states_budget(&broken, Budget::ROUTE).is_err());
    let not_power = Budget {
        max_check_work: 3_000_000_000,
        ..Budget::ROUTE
    };
    assert!(spec_states_budget(&spec, not_power).is_err());
}

#[test]
fn the_kit_and_the_lane_agree_on_the_ladder() {
    // `vos-link` cannot depend on the lane (the Cargo edge runs the other way), so CK11 keeps
    // its own copy of the ladder; this is the test that ties the two.
    let lane: Vec<u64> = LADDER.iter().map(|&n| n as u64).collect();
    assert_eq!(lane, vos_link::sat_lane::SAT_LADDER);
    assert_eq!(
        vos_model_sat::labels::LABEL_TRIVIAL.replace(' ', "_"),
        vos_link::sat_lane::SAT_TRIVIAL_LABEL,
        "the trivial label token CK11 does not count"
    );
}

#[test]
fn the_committed_table_of_the_real_contract_is_current_and_ck11_accepts_it() {
    // The committed `warrant/lemmas.sat` is what `gen` writes now (MS09 would say otherwise)...
    let a = check_with(files(), &real_cfg(), Some(evidence())).unwrap();
    assert!(a.findings.is_empty(), "{:?}", a.findings);
    // ...and the kit's files-only rule accepts it with 36 counted rows and 4 trivial ones.
    let table = std::fs::read_to_string(contract_dir().join("warrant").join("lemmas.sat")).unwrap();
    let claims: Vec<String> = vos_model_sat::oracle::claims_of(&files().base_document())
        .unwrap()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    let (bad, facts) = vos_link::sat_lane::judge_sat_table(
        &table,
        &claims,
        &coding_adventures_sha256::sha256_hex(files().contract_text.as_bytes()),
        &coding_adventures_sha256::sha256_hex(files().lemmas_text.as_bytes()),
    );
    assert!(bad.is_empty(), "{bad:?}");
    assert_eq!((facts.rows, facts.counted), (40, 36));
}
