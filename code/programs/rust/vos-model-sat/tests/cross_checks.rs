//! The three cross-checks on the real `sched` contract (VOS31 section 6), and their negative
//! controls (section 6.4).  Every positive check here compares millions of cases; every
//! negative control shows the same check FAILING on an encoder or an input that is wrong.

mod common;

use common::{evidence, EXPECTED_MUTANT_FAILED};
use vos_model_sat::crosscheck::{
    check_against_reference, check_all_widths, check_evaluator_side, check_function, check_lemma,
    check_reference_vs_evaluator, check_table_counts, encoder_failed, honest_at, real_encoders,
    table_failed_counts, Evidence, Report, EVALUATOR_N,
};
use vos_model_sat::oracle::{DocResults, LemmaFailures};
use vos_model_sat::probes::all_probes;
use vos_model_sat::{Encoder, Fault, Faulty, LemmaId, Mutant, SchedModel, Val};

// ---- positive: the real encoder agrees ----------------------------------------------------

#[test]
fn a_lemma_check_base_every_lemma_whole_domain() {
    let ev = evidence();
    for lemma in LemmaId::ALL {
        let e = &ev.base.lemmas[lemma.name()];
        assert_eq!(e.failed(), 0, "{} holds on the real contract", lemma.name());
        let r = check_lemma(&Encoder { n: 8 }, lemma, e);
        assert!(r.is_clean(), "{}: {r:?}", lemma.name());
        assert_eq!(r.compared, e.product);
    }
}

#[test]
fn a_lemma_check_mutants_failing_sets_equal_and_counts_match_the_table() {
    let ev = evidence();
    let table = ev.table.as_deref().expect("lemmas.table is committed");
    let from_table = table_failed_counts(table);
    for (i, lemma) in LemmaId::ALL.into_iter().enumerate() {
        let (l, results) = &ev.mutants[i];
        assert_eq!(*l, lemma);
        let e = &results.lemmas[lemma.name()];
        let mutant = Mutant::new(lemma, 8).unwrap();
        // The SETS are equal, case by case ...
        let r = check_lemma(&mutant, lemma, e);
        assert!(r.is_clean(), "{}: {r:?}", lemma.name());
        // ... and the count is the one lemmas.table and VOS31 6.1 state.
        assert_eq!(e.failed(), EXPECTED_MUTANT_FAILED[i], "{}", lemma.name());
        assert_eq!(encoder_failed(&mutant, lemma), EXPECTED_MUTANT_FAILED[i]);
        assert_eq!(from_table[lemma.name()], EXPECTED_MUTANT_FAILED[i]);
        assert_eq!(mutant.replaced_model(), lemma.mutant_model());
    }
}

#[test]
fn b_function_check_every_model_function_whole_domain() {
    let ev = evidence();
    let enc = Encoder { n: 8 };
    let probes = all_probes();
    let mut total = 0;
    for p in &probes {
        let r = check_function(&enc, p, &ev.base.probes[p.name]);
        assert!(r.is_clean(), "{}: {r:?}", p.name);
        total += r.compared;
    }
    assert!(total > 1_000_000, "{total}");
}

#[test]
fn the_evaluator_side_as_a_whole_is_clean() {
    let (base, mutant) = real_encoders();
    let r = check_evaluator_side(evidence(), &base, &mutant);
    assert!(r.is_clean(), "{r:?}");
}

#[test]
fn c_width_check_n_1_to_10_exhaustive() {
    let r = check_all_widths(&honest_at);
    assert!(r.is_clean(), "{r:?}");
    assert!(r.compared > 10_000_000, "{}", r.compared);
}

#[test]
fn c_reference_equals_the_evaluator_at_n_8() {
    let r = check_reference_vs_evaluator(&evidence().base);
    assert!(r.is_clean(), "{r:?}");
}

// ---- negative controls: (a), (b) and (c) EACH catch EACH wrong encoder --------------------

#[test]
fn negative_control_a_catches_every_fault_on_the_base_document() {
    let ev = evidence();
    for fault in Fault::ALL {
        let f = Faulty { fault, n: 8 };
        let mut r = Report::default();
        for lemma in LemmaId::ALL {
            r.merge(check_lemma(&f, lemma, &ev.base.lemmas[lemma.name()]));
        }
        assert!(r.disagreements > 0, "(a) missed {fault:?}");
    }
}

#[test]
fn negative_control_b_catches_every_fault() {
    let ev = evidence();
    for fault in Fault::ALL {
        let f = Faulty { fault, n: 8 };
        let mut r = Report::default();
        for p in all_probes() {
            r.merge(check_function(&f, &p, &ev.base.probes[p.name]));
        }
        assert!(r.disagreements > 0, "(b) missed {fault:?}");
    }
}

#[test]
fn negative_control_c_catches_every_fault() {
    for fault in Fault::ALL {
        let r = check_against_reference(&Faulty { fault, n: 8 });
        assert!(r.disagreements > 0, "(c) missed {fault:?}");
        // At another width too, not only at 8.
        let r3 = check_against_reference(&Faulty { fault, n: 3 });
        assert!(r3.disagreements > 0, "(c) missed {fault:?} at N = 3");
    }
}

#[test]
fn negative_control_the_two_faults_named_by_the_spec() {
    // VOS31 6.4 names these two: `top` off by one, and `<=` for `<` in `pre_insert`.
    let ev = evidence();
    let top = Faulty {
        fault: Fault::TopOffByOne,
        n: 8,
    };
    let le = Faulty {
        fault: Fault::PreInsertLe,
        n: 8,
    };
    let top_probe = all_probes().into_iter().find(|p| p.name == "top").unwrap();
    let pre_probe = all_probes()
        .into_iter()
        .find(|p| p.name == "pre_insert")
        .unwrap();
    assert!(!check_function(&top, &top_probe, &ev.base.probes["top"]).is_clean());
    assert!(!check_function(&le, &pre_probe, &ev.base.probes["pre_insert"]).is_clean());
    // The lemma check sees `top` through top_is_the_maximum ...
    let l = LemmaId::TopIsTheMaximum;
    assert!(!check_lemma(&top, l, &ev.base.lemmas[l.name()]).is_clean());
    // ... and `<=` through the config lemma (the other lemmas stay vacuously true).
    let c = LemmaId::ConfigNIsTheIdRange;
    assert!(!check_lemma(&le, c, &ev.base.lemmas[c.name()]).is_clean());
}

#[test]
fn negative_control_a_wrong_mutant_is_caught() {
    // Using the HONEST model as a lemma's "mutant" makes the lemma fail nowhere, while the
    // evaluator fails on the lemma's real set.
    let (base, _) = real_encoders();
    let wrong = |_l: LemmaId| -> Box<dyn SchedModel> { Box::new(Encoder { n: 8 }) };
    let r = check_evaluator_side(evidence(), &base, &wrong);
    assert!(!r.is_clean());
    assert!(r.disagreements > 100_000, "{}", r.disagreements);
}

#[test]
fn negative_control_equal_counts_but_different_sets_is_caught() {
    // The evaluator's failing set with one failing case and one passing case swapped has the
    // same SIZE; only a set comparison sees it (VOS31 6.1: "not just equal in size").
    let l = LemmaId::TopIsTheMaximum;
    let (_, results) = &evidence().mutants[4];
    let real = &results.lemmas[l.name()];
    let mut swapped = real.clone();
    let a = swapped.failing.iter().position(|&f| f).unwrap();
    let b = swapped.failing.iter().position(|&f| !f).unwrap();
    swapped.failing[a] = false;
    swapped.failing[b] = true;
    assert_eq!(swapped.failed(), real.failed());
    let r = check_lemma(&Mutant::new(l, 8).unwrap(), l, &swapped);
    assert_eq!(r.disagreements, 2);
    assert_eq!(r.samples.len(), 2);
}

#[test]
fn negative_control_a_wrong_domain_size_is_a_disagreement() {
    let l = LemmaId::ClosureAtM;
    let short = LemmaFailures {
        product: 256,
        failing: vec![false; 256],
    };
    let r = check_lemma(&Encoder { n: 8 }, l, &short);
    assert!(!r.is_clean());
    assert_eq!(r.compared, 1);
}

fn clone_results() -> DocResults {
    let b = &evidence().base;
    DocResults {
        lemmas: b.lemmas.clone(),
        probes: b.probes.clone(),
    }
}

#[test]
fn negative_control_tampered_function_values_and_reference() {
    let ev = evidence();
    let probe = all_probes().into_iter().find(|p| p.name == "top").unwrap();
    let mut vals = ev.base.probes["top"].clone();
    vals[3] = Val::Pick(Some(200));
    assert!(!check_function(&Encoder { n: 8 }, &probe, &vals).is_clean());
    vals.pop();
    let r = check_function(&Encoder { n: 8 }, &probe, &vals);
    assert_eq!(
        r.compared, 1,
        "a short evaluator answer is one disagreement"
    );
    // The reference against a tampered evaluator.
    let mut tampered = clone_results();
    tampered.probes.get_mut("mem").unwrap()[5] = Val::Bool(false);
    tampered.lemmas.get_mut("closure_at_m").unwrap().failing[7] = true;
    let r = check_reference_vs_evaluator(&tampered);
    assert_eq!(r.disagreements, 2, "{r:?}");
    // Missing entries are disagreements, not silent passes.
    let mut gone = clone_results();
    gone.probes.remove("top");
    gone.lemmas.remove("config_n_is_the_id_range");
    assert_eq!(check_reference_vs_evaluator(&gone).disagreements, 2);
}

#[test]
fn negative_control_the_table_counts() {
    let table = evidence().table.clone().unwrap();
    let l = LemmaId::InsertThenRemoveRestores;
    assert!(check_table_counts(&table, l, 448, 448).is_clean());
    // Either side off: a disagreement naming it.
    let r = check_table_counts(&table, l, 449, 448);
    assert_eq!(r.disagreements, 1);
    assert!(r.samples[0].detail.contains("evaluator"));
    let r = check_table_counts(&table, l, 448, 3);
    assert!(r.samples[0].detail.contains("encoder"));
    // A table with the row removed.
    let without: String = table
        .lines()
        .filter(|ln| !ln.starts_with("lemma insert_then_remove_restores"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!check_table_counts(&without, l, 448, 448).is_clean());
    // A table that does not parse yields no counts at all.
    assert!(table_failed_counts("lemma x failed notanumber\nlemma").is_empty());
}

#[test]
fn negative_control_a_check_that_compared_nothing_is_not_clean() {
    assert!(!Report::default().is_clean());
}

#[test]
fn negative_control_missing_evaluator_results() {
    let (base, mutant) = real_encoders();
    let ev = evidence();
    let mut empty = Evidence {
        base: DocResults::default(),
        mutants: vec![(LemmaId::ClosureAtM, DocResults::default())],
        table: None,
    };
    let r = check_evaluator_side(&empty, &base, &mutant);
    // 10 lemmas and every probe missing from the base, one lemma missing from the mutant.
    assert_eq!(r.disagreements as usize, 10 + all_probes().len() + 1);
    // A mutant result with its lemma but no table to compare with.
    empty.mutants[0].1 = DocResults {
        lemmas: ev.mutants[0].1.lemmas.clone(),
        probes: Default::default(),
    };
    let r = check_evaluator_side(&empty, &base, &mutant);
    assert!(r.samples.len() == 5 && r.disagreements as usize > 10 + all_probes().len());
}

#[test]
fn the_evaluator_n_is_the_contract_n() {
    assert_eq!(EVALUATOR_N, 8);
}
