//! The encoder on its own, with no evaluator: widths, lemmas, mutants, premises, labels, the
//! reference model and the sweep.  (The evaluator-facing checks are in `cross_checks.rs`.)

use bit_blaster::aig::Aig;
use bit_blaster::bv::BitVec;
use vos_model_sat::crosscheck::{decode_lemma_case, probes_affected_by};
use vos_model_sat::labels::{
    label_collapsed, label_for, label_token, CollapseReason, FORBIDDEN_WORDS, LABEL_LOWERED_HELPER,
    LABEL_N8, LABEL_TRIVIAL, NO_SHIPPED_PHRASE,
};
use vos_model_sat::lemmas::{lemma, lemma_circuit, LemmaId, UnknownLemma};
use vos_model_sat::probes::{all_probes, Kind};
use vos_model_sat::reference as r;
use vos_model_sat::sweep::{sweep, TooWide, MAX_SWEEP_INPUTS};
use vos_model_sat::{BadWidth, Encoder, Fault, Faulty, Mutant, Out, Ready, SchedModel, Val, MAX_N};

/// How many of the lemma's cases hold, and the total.
fn holds_count(model: &dyn SchedModel, l: LemmaId) -> (u64, u64) {
    let mut aig = Aig::new();
    let c = lemma_circuit(l, model, &mut aig);
    let (mut yes, mut all) = (0, 0);
    sweep(&aig, aig.num_inputs(), |_, v| {
        all += 1;
        yes += u64::from(v(c.holds));
    })
    .unwrap();
    (yes, all)
}

#[test]
fn widths_are_bounded() {
    assert_eq!(Encoder::new(0), Err(BadWidth(0)));
    assert_eq!(Encoder::new(MAX_N + 1), Err(BadWidth(65)));
    assert_eq!(Encoder::new(1).unwrap().n, 1);
    assert_eq!(Encoder::new(64).unwrap().n, 64);
    assert!(Mutant::new(LemmaId::ClosureAtM, 0).is_err());
    assert!(Mutant::new(LemmaId::ClosureAtM, 64).is_ok());
}

#[test]
fn lemma_names_round_trip_and_unknown_is_an_error() {
    for l in LemmaId::ALL {
        assert_eq!(LemmaId::from_name(l.name()), Ok(l));
    }
    assert_eq!(
        LemmaId::from_name("nope"),
        Err(UnknownLemma("nope".to_string()))
    );
    let mut aig = Aig::new();
    assert!(lemma("top_is_the_maximum", &Encoder { n: 4 }, &mut aig).is_ok());
    assert!(lemma("nope", &Encoder { n: 4 }, &mut aig).is_err());
    assert_eq!(LemmaId::ALL.len(), 10);
}

#[test]
fn honest_lemmas_hold_and_every_mutant_breaks_at_every_ladder_width_and_below() {
    // The SAT control's cheap precursor: the honest lemma is true on every case and the mutant
    // is false on some, for N = 2 ..= 10 (exhaustive) and N = 8.  (At N = 1 the `top` mutant
    // equals the base: nothing to prefer; the ladder starts at 8.)
    for n in 2..=10usize {
        for l in LemmaId::ALL {
            let (yes, all) = holds_count(&Encoder { n }, l);
            assert_eq!(yes, all, "{} at N = {n}", l.name());
            let (yes, all) = holds_count(&Mutant::new(l, n).unwrap(), l);
            assert!(yes < all, "mutant of {} survives at N = {n}", l.name());
        }
    }
}

#[test]
fn the_top_mutant_equals_the_base_at_n_1() {
    let (yes, all) = holds_count(
        &Mutant::new(LemmaId::TopIsTheMaximum, 1).unwrap(),
        LemmaId::TopIsTheMaximum,
    );
    assert_eq!(yes, all);
}

#[test]
fn the_ladder_widths_build_with_a_bounded_size() {
    for n in [8usize, 16, 32, 64] {
        for l in LemmaId::ALL {
            let mut aig = Aig::new();
            let c = lemma_circuit(l, &Encoder { n }, &mut aig);
            assert_eq!(c.inputs.len(), l.input_count(n));
            assert_eq!(aig.num_inputs(), c.inputs.len());
            assert!(
                aig.num_ands() < 200_000,
                "{} at {n}: {}",
                l.name(),
                aig.num_ands()
            );
            let mut aig2 = Aig::new();
            let m = lemma_circuit(l, &Mutant::new(l, n).unwrap(), &mut aig2);
            assert_eq!(m.inputs.len(), c.inputs.len());
        }
    }
}

#[test]
fn closure_and_config_fold_on_the_honest_encoder() {
    // VOS31 7.5: the honest closure_at_m and config lemma are the constant true, which is why a
    // collapse reason and a mutant control exist.
    for l in [LemmaId::ClosureAtM, LemmaId::ConfigNIsTheIdRange] {
        let mut aig = Aig::new();
        let c = lemma_circuit(l, &Encoder { n: 64 }, &mut aig);
        assert!(c.holds.is_constant(), "{}", l.name());
    }
}

#[test]
fn premises_exist_exactly_for_the_implication_lemmas_and_are_satisfiable_and_not_always_true() {
    let mut with_premise = 0;
    for l in LemmaId::ALL {
        let mut aig = Aig::new();
        let c = lemma_circuit(l, &Encoder { n: 8 }, &mut aig);
        assert_eq!(c.premise.is_some(), l.has_premise(), "{}", l.name());
        if let Some(p) = c.premise {
            with_premise += 1;
            let (mut yes, mut no) = (0u32, 0u32);
            sweep(&aig, aig.num_inputs(), |_, v| {
                if v(p) {
                    yes += 1
                } else {
                    no += 1
                }
            })
            .unwrap();
            assert!(yes > 0, "{} premise is unsatisfiable: vacuous", l.name());
            assert!(no > 0 || l == LemmaId::PolicyImpliesSafety, "{}", l.name());
        }
    }
    assert_eq!(with_premise, 6);
}

#[test]
fn binder_shapes_match_the_domains_the_spec_states() {
    let sizes: Vec<usize> = LemmaId::ALL.iter().map(|l| l.input_count(8)).collect();
    assert_eq!(sizes, [16, 16, 16, 16, 16, 16, 16, 8, 16, 8]);
    assert!(
        LemmaId::PolicyImpliesSafety.has_ready_binders()
            && !LemmaId::PolicyImpliesSafety.has_id_binder()
    );
    assert!(
        !LemmaId::ConfigNIsTheIdRange.has_ready_binders()
            && LemmaId::ConfigNIsTheIdRange.has_id_binder()
    );
}

#[test]
fn decode_lemma_case_follows_the_numbering() {
    assert_eq!(
        decode_lemma_case(LemmaId::ClosureAtM, 8, 0x1234),
        (0x34, 0x12)
    );
    assert_eq!(
        decode_lemma_case(LemmaId::PolicyImpliesSafety, 8, 0x1FF),
        (0xFF, 0)
    );
    assert_eq!(
        decode_lemma_case(LemmaId::ConfigNIsTheIdRange, 8, 0x1AB),
        (0, 0xAB)
    );
    assert_eq!(
        decode_lemma_case(LemmaId::ClosureAtM, 3, 0b1111_1101),
        (0b101, 0b11111)
    );
}

#[test]
fn reference_model_spot_checks() {
    assert_eq!(r::mask(64), u64::MAX);
    assert_eq!(r::mask(3), 7);
    assert_eq!(r::top(8, 0b0101), Some(2));
    assert_eq!(r::top(8, 0), None);
    assert_eq!(r::top(4, 0b1_0000), None, "bits above N are not threads");
    assert_eq!(r::abs_insert(8, 0, 8), (r::OUT_OF_RANGE, 0));
    assert_eq!(r::abs_insert(8, 1, 0), (r::UNCHANGED, 1));
    assert_eq!(r::abs_insert(8, 0, 3), (r::CHANGED, 8));
    assert_eq!(r::abs_remove(8, 8, 3), (r::CHANGED, 0));
    assert_eq!(r::abs_remove(8, 0, 3), (r::UNCHANGED, 0));
    assert_eq!(r::abs_remove(8, 5, 9), (r::OUT_OF_RANGE, 5));
    assert!(r::valid_pick(8, 0, None));
    assert!(!r::valid_pick(8, 0, Some(1)));
    assert!(!r::valid_pick(8, 255, Some(8)));
    assert_eq!(r::pick_of(255), None);
    assert_eq!(r::pick_of(7), Some(7));
    assert_eq!(r::put(8, 1, 200, true), 1);
    assert_eq!(r::put(8, 1, 1, true), 3);
    assert!(r::subset(1, 3) && !r::subset(3, 1));
    assert!(r::safe_dispatch(8, 6, Some(2), 2));
    assert!(r::policy_dispatch(8, 6, Some(2), 2));
    assert!(!r::policy_dispatch(8, 6, Some(1), 4) && r::safe_dispatch(8, 6, Some(1), 4));
    for l in LemmaId::ALL {
        for m in 0..16u64 {
            for t in [0u8, 3, 4, 8, 254, 255] {
                assert!(r::lemma_holds(l, 4, m, t), "{} m={m} t={t}", l.name());
            }
        }
    }
}

#[test]
fn probes_are_well_formed() {
    let probes = all_probes();
    let mut names: Vec<&str> = probes.iter().map(|p| p.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), probes.len(), "probe names are unique");
    for p in &probes {
        let bits = p.input_count(8);
        assert!((1..=18).contains(&bits), "{} has {bits} inputs", p.name);
        assert_eq!(p.decode(8, 0).len(), p.binders.len());
        let claim = p.claim_text();
        assert!(claim.contains(&format!("claim probe_{}:", p.name)));
        // A Ready binder is eight Bool names on the Warrant side.
        let readies = p.binders.iter().filter(|b| b.kind == Kind::Ready).count();
        assert!(p.warrant_binders().matches(": Bool").count() >= 8 * readies);
    }
    // Relation probes: 2^18 cases, the documented slice.
    let rel = probes.iter().find(|p| p.name == "safe_dispatch").unwrap();
    assert_eq!(rel.input_count(8), 18);
}

#[test]
fn probe_circuits_read_back_as_the_reference_on_a_known_case() {
    let probes = all_probes();
    let top = probes.iter().find(|p| p.name == "top").unwrap();
    let mut aig = Aig::new();
    let out = top.circuit(&Encoder { n: 8 }, &mut aig);
    let mut seen = Vec::new();
    sweep(&aig, 8, |case, v| {
        if case == 0 || case == 0b0100_0100 || case == 0xFF {
            seen.push(out.read(v));
        }
    })
    .unwrap();
    assert_eq!(
        seen,
        [Val::Pick(None), Val::Pick(Some(6)), Val::Pick(Some(7))]
    );
    assert!(matches!(out, Out::Pick(_)));
}

#[test]
fn probes_affected_by_names_callers() {
    assert_eq!(probes_affected_by("top"), ["top", "policy_dispatch"]);
    assert!(probes_affected_by("valid_pick").contains(&"safe_dispatch"));
    assert!(probes_affected_by("after").contains(&"policy_dispatch"));
    assert!(probes_affected_by("pre_insert").contains(&"pre_insert"));
    assert!(probes_affected_by("inv").contains(&"inv"));
    assert!(probes_affected_by("abs_remove").contains(&"abs_remove_upd"));
    assert!(probes_affected_by("policy_dispatch").contains(&"policy_dispatch"));
    assert!(probes_affected_by("abs_insert").contains(&"abs_insert_ready"));
    assert!(probes_affected_by("nothing").is_empty());
    // Every mutant's replaced model is one this table knows.
    for l in LemmaId::ALL {
        assert!(
            !probes_affected_by(l.mutant_model()).is_empty(),
            "{}",
            l.name()
        );
    }
}

#[test]
fn faults_each_change_something_observable_at_n_8() {
    for fault in Fault::ALL {
        let f = Faulty { fault, n: 8 };
        let broke = LemmaId::ALL.iter().any(|&l| {
            let (yes, all) = holds_count(&f, l);
            yes < all
        });
        assert!(broke, "{fault:?} breaks no lemma at all");
    }
}

#[test]
fn pick_equality_ignores_the_id_of_idle() {
    let m = Encoder { n: 8 };
    let mut aig = Aig::new();
    let a = vos_model_sat::PickEnc {
        idle: aig.input(),
        id: BitVec::input(&mut aig, 8),
    };
    let b = vos_model_sat::PickEnc {
        idle: aig.input(),
        id: BitVec::input(&mut aig, 8),
    };
    let eq = m.pick_eq(&mut aig, &a, &b);
    // idle, idle, ids differ: equal.  idle vs run: unequal.  run, run, same id: equal.
    let word = |ia: bool, ida: u8, ib: bool, idb: u8| -> Vec<bool> {
        let mut v = vec![ia];
        v.extend((0..8).map(|i| ida >> i & 1 == 1));
        v.push(ib);
        v.extend((0..8).map(|i| idb >> i & 1 == 1));
        v
    };
    assert!(aig.eval(eq, &word(true, 1, true, 2)));
    assert!(!aig.eval(eq, &word(true, 1, false, 1)));
    assert!(aig.eval(eq, &word(false, 5, false, 5)));
    assert!(!aig.eval(eq, &word(false, 5, false, 6)));
}

#[test]
fn ready_helpers() {
    let mut aig = Aig::new();
    let rd = Ready::input(&mut aig, 5);
    assert_eq!(rd.width(), 5);
    assert_eq!(aig.num_inputs(), 5);
    assert_eq!(rd.bit(2), rd.0.bits[2]);
}

#[test]
fn sweep_refuses_a_wrong_or_too_wide_input_count() {
    let mut aig = Aig::new();
    aig.input();
    assert_eq!(sweep(&aig, 2, |_, _| {}), Err(TooWide(2)));
    let mut wide = Aig::new();
    for _ in 0..=MAX_SWEEP_INPUTS {
        wide.input();
    }
    assert_eq!(
        sweep(&wide, MAX_SWEEP_INPUTS + 1, |_, _| {}),
        Err(TooWide(MAX_SWEEP_INPUTS + 1))
    );
    // A sweep visits every case once, in order, including a partial last batch.
    let mut three = Aig::new();
    for _ in 0..3 {
        three.input();
    }
    let mut seen = Vec::new();
    sweep(&three, 3, |c, _| seen.push(c)).unwrap();
    assert_eq!(seen, (0..8).collect::<Vec<u64>>());
}

// ---- labels (VOS31 section 11): exact strings ---------------------------------------------

#[test]
fn labels_are_the_exact_strings_of_the_spec() {
    assert_eq!(
        LABEL_N8,
        "lemma, SAT-certified at N = 8; encoder checked against the evaluator on the whole domain"
    );
    assert_eq!(label_for(8), LABEL_N8);
    for n in [16usize, 32, 64] {
        assert_eq!(
            label_for(n),
            format!("lemma family member, SAT-certified at N = {n}; the encoder is checked against the evaluator at N = 8 only; no shipped component has this width")
        );
    }
    assert_eq!(LABEL_TRIVIAL, "trivial (inv is true)");
    assert_eq!(LABEL_LOWERED_HELPER, "lowered helper");
    assert_eq!(
        label_collapsed(8, CollapseReason::ConstantFold),
        format!("{LABEL_N8}; collapsed (constant-fold)")
    );
    assert_eq!(label_token("a b c"), "a_b_c");
    assert!(!label_token(&label_for(16)).contains(' '));
}

#[test]
fn no_label_contains_a_forbidden_word() {
    let mut labels = vec![
        LABEL_N8.to_string(),
        LABEL_TRIVIAL.to_string(),
        LABEL_LOWERED_HELPER.to_string(),
        label_collapsed(8, CollapseReason::PropagationOnly),
        label_collapsed(64, CollapseReason::ConstantFold),
    ];
    labels.extend([8usize, 16, 32, 64].map(label_for));
    for l in labels {
        // The one honest phrase of the wider rows says no component HAS this width; it is the
        // only place "shipped" may appear.
        let scrubbed = l.replace(NO_SHIPPED_PHRASE, "");
        for w in FORBIDDEN_WORDS {
            assert!(!scrubbed.contains(w), "label {l:?} contains {w:?}");
        }
    }
}

#[test]
fn collapse_reasons_are_a_closed_enum() {
    // VOS31 7.5 (Q7): exactly these two spellings, round-tripping, and nothing else parses.
    assert_eq!(CollapseReason::ConstantFold.as_str(), "constant-fold");
    assert_eq!(CollapseReason::PropagationOnly.as_str(), "propagation-only");
    for r in CollapseReason::ALL {
        assert_eq!(CollapseReason::parse(r.as_str()), Some(r));
        assert!(label_collapsed(16, r).ends_with(&format!("collapsed ({})", r.as_str())));
    }
    for bad in [
        "",
        "proved",
        "verified",
        "identical-structure",
        "frame-untouched",
        "Constant-Fold",
    ] {
        assert_eq!(CollapseReason::parse(bad), None, "{bad}");
    }
}
