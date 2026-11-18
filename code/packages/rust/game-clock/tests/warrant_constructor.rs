//! Executes GAME10's exact constructor-model claim against the exported Rust API.
//!
//! The Warrant evaluator runs only the pure model expression. The subject
//! operation is called independently by this test and normalized into the
//! evaluator's typed value representation before comparison. This finite and
//! seeded execution evidence is not a universal proof of the Warrant claim or
//! a source-level proof of the Rust implementation.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};

use coding_adventures_test_bootstrap::pbt::{
    check_property, gen_bounded_int, gen_u64, shrink_int, PbtOptions,
};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, Declaration, Expression, ParserLimits, QualifiedName, TypeExpr,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use game_clock::{
    new_clock, ClockError, GameClockConfig, GameClockState, HARD_MAX_DELTA_NS, HARD_MAX_FRAME_STEPS,
};

const WARRANT_SPEC: &str = include_str!("../../../../specs/game-clock.warrant");
const MODEL_CLAIM: &str = "new_clock_matches_model";
const PROPERTY_SEED: u64 = 0x4743_4c4b_5f50_5254;
const PROPERTY_CASES: usize = 1024;
const PROPERTY_SHRINK_STEPS: usize = 512;

/// Keep the bounded admission, layout, parse and checker steps explicit. The
/// callback runs while all source-backed AST data is still alive.
fn with_parsed_file<R>(
    source: &str,
    callback: impl FnOnce(
        Result<&coding_adventures_warrant_grammar::parser::WarrantFile<'_>, String>,
    ) -> R,
) -> R {
    let doc = match SourceDocument::admit(source.as_bytes(), SourceLimits::default()) {
        Ok(doc) => doc,
        Err(error) => return callback(Err(format!("source admission: {error:?}"))),
    };
    let layout = match capture_layout(&doc, LayoutLimits::default()) {
        Ok(layout) => layout,
        Err(error) => return callback(Err(format!("layout capture: {error:?}"))),
    };
    let file = match parse_document(&layout, &doc, ParserLimits::default()) {
        Ok(file) => file,
        Err(error) => return callback(Err(format!("Warrant parse: {error:?}"))),
    };
    callback(Ok(&file))
}

fn ungroup<'f, 'a>(mut expression: &'f Expression<'a>) -> &'f Expression<'a> {
    while let Expression::Grouped { inner, .. } = expression {
        expression = inner;
    }
    expression
}

fn has_only_segment(name: &QualifiedName<'_>, expected: &str) -> bool {
    name.segments.as_slice() == [expected]
}

fn is_bare_reference(expression: &Expression<'_>, expected: &str) -> bool {
    matches!(
        ungroup(expression),
        Expression::Reference {
            name,
            widths,
            args: None,
            ..
        } if widths.is_empty() && has_only_segment(name, expected)
    )
}

fn is_call_with_bare_binder(
    expression: &Expression<'_>,
    expected_call: &str,
    binder: &str,
) -> bool {
    matches!(
        ungroup(expression),
        Expression::Reference {
            name,
            widths,
            args: Some(args),
            ..
        } if widths.is_empty()
            && has_only_segment(name, expected_call)
            && args.len() == 1
            && is_bare_reference(&args[0], binder)
    )
}

/// Require one precise, unguarded universal equality and return its bare
/// binder name and pure-model RHS. AST spans are deliberately ignored.
fn inspect_constructor_claim<'f, 'a>(
    file: &'f coding_adventures_warrant_grammar::parser::WarrantFile<'a>,
) -> Result<(&'a str, &'f Expression<'a>), String> {
    let claims: Vec<_> = file
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Claim(claim) if claim.name == MODEL_CLAIM => Some(claim),
            _ => None,
        })
        .collect();
    if claims.len() != 1 {
        return Err(format!(
            "expected exactly one `{MODEL_CLAIM}` claim, found {}",
            claims.len()
        ));
    }

    let Expression::Quantified {
        is_forall: true,
        binders,
        body,
        ..
    } = ungroup(&claims[0].body)
    else {
        return Err(format!("`{MODEL_CLAIM}` is not a universal quantifier"));
    };
    if binders.len() != 1 {
        return Err(format!(
            "`{MODEL_CLAIM}` must have exactly one binder, found {}",
            binders.len()
        ));
    }
    let binder = &binders[0];
    if binder.refinement.is_some() || binder.inout {
        return Err(format!(
            "`{MODEL_CLAIM}` binder must be unrefined and non-inout"
        ));
    }
    if !matches!(
        &binder.param_type,
        TypeExpr::Named {
            name: "GameClockConfig",
            ..
        }
    ) {
        return Err(format!(
            "`{MODEL_CLAIM}` binder must have type GameClockConfig"
        ));
    }

    let Expression::Binary {
        op: BinaryOp::Eq,
        left,
        right,
        ..
    } = ungroup(body)
    else {
        return Err(format!("`{MODEL_CLAIM}` body must be a direct equality"));
    };
    if !is_call_with_bare_binder(left, "new_clock", binder.name) {
        return Err(format!(
            "`{MODEL_CLAIM}` left side must call new_clock with its bare binder"
        ));
    }
    if !is_call_with_bare_binder(right, "new_clock_model", binder.name) {
        return Err(format!(
            "`{MODEL_CLAIM}` right side must call new_clock_model with the same bare binder"
        ));
    }
    Ok((binder.name, right))
}

fn config_value(config: GameClockConfig) -> EvalValue {
    EvalValue::Con {
        ty: "GameClockConfig".to_string(),
        index: 0,
        ctor: "GameClockConfig".to_string(),
        record: true,
        fields: vec![
            EvalValue::U64(config.fixed_dt_ns),
            EvalValue::Fixed(FixedTy::U32, i128::from(config.max_frame_steps)),
            EvalValue::U64(config.max_frame_delta_ns),
        ],
    }
}

fn state_value(state: GameClockState) -> EvalValue {
    // Keep this destructuring exhaustive: a new Rust field must force an
    // explicit decision about the portable Warrant record before this test
    // still compiles.
    let GameClockState {
        tick_count,
        frame_count,
        elapsed_sim_ns,
        elapsed_wall_ns,
        accumulator_ns,
        fixed_dt_ns,
        max_frame_steps,
        max_frame_delta_ns,
        total_dropped_ns,
        spiral_clamped_frames,
    } = state;
    EvalValue::Con {
        ty: "GameClockState".to_string(),
        index: 0,
        ctor: "GameClockState".to_string(),
        record: true,
        fields: vec![
            EvalValue::U64(tick_count),
            EvalValue::U64(frame_count),
            EvalValue::U64(elapsed_sim_ns),
            EvalValue::U64(elapsed_wall_ns),
            EvalValue::U64(accumulator_ns),
            EvalValue::U64(fixed_dt_ns),
            EvalValue::Fixed(FixedTy::U32, i128::from(max_frame_steps)),
            EvalValue::U64(max_frame_delta_ns),
            EvalValue::U64(total_dropped_ns),
            EvalValue::U64(spiral_clamped_frames),
        ],
    }
}

fn error_variant(error: ClockError) -> &'static str {
    match error {
        ClockError::ZeroFixedTimestep => "ZeroFixedTimestep",
        ClockError::InvalidMaxFrameSteps => "InvalidMaxFrameSteps",
        ClockError::InvalidMaxFrameDelta => "InvalidMaxFrameDelta",
        ClockError::CorruptedStateInvariant => "CorruptedStateInvariant",
        ClockError::TimestampOverflow => "TimestampOverflow",
    }
}

fn result_value(result: Result<GameClockState, ClockError>) -> EvalValue {
    EvalValue::Result(match result {
        Ok(state) => Ok(Box::new(state_value(state))),
        Err(error) => Err(("ClockError".to_string(), error_variant(error).to_string())),
    })
}

#[derive(Clone, Copy, Debug)]
enum SubjectAdapter {
    Native,
    AlwaysErr,
    SwappedErrorPrecedence,
    WrongStateField,
}

fn call_subject(
    config: GameClockConfig,
    adapter: SubjectAdapter,
) -> Result<GameClockState, ClockError> {
    match adapter {
        SubjectAdapter::Native => new_clock(config),
        // Deliberate fault-injection controls. These are not source proofs.
        SubjectAdapter::AlwaysErr => Err(ClockError::ZeroFixedTimestep),
        SubjectAdapter::SwappedErrorPrecedence => {
            if config.fixed_dt_ns > HARD_MAX_DELTA_NS
                && (config.max_frame_steps == 0 || config.max_frame_steps > HARD_MAX_FRAME_STEPS)
            {
                Err(ClockError::InvalidMaxFrameSteps)
            } else {
                new_clock(config)
            }
        }
        SubjectAdapter::WrongStateField => new_clock(config).map(|mut state| {
            state.tick_count = 1;
            state
        }),
    }
}

fn compare_case(
    file: &coding_adventures_warrant_grammar::parser::WarrantFile<'_>,
    binder: &str,
    model: &Expression<'_>,
    config: GameClockConfig,
    adapter: SubjectAdapter,
) -> Result<(), String> {
    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    let mut environment = HashMap::new();
    environment.insert(binder, config_value(config));
    evaluator.reset_work();
    let expected = evaluator
        .eval(model, &environment)
        .map_err(|error| format!("model evaluation failed: {error:?}; config={config:?}"))?;
    let actual = result_value(call_subject(config, adapter));
    if actual != expected {
        return Err(format!(
            "constructor mismatch: config={config:?}, model={expected:?}, subject={actual:?}, adapter={adapter:?}"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum ConstructorOutcome {
    Ok,
    ZeroFixedTimestep,
    InvalidMaxFrameSteps,
    InvalidMaxFrameDelta,
}

fn outcome(value: &EvalValue) -> ConstructorOutcome {
    match value {
        EvalValue::Result(Ok(state)) => {
            assert!(matches!(
                state.as_ref(),
                EvalValue::Con {
                    ty,
                    index: 0,
                    ctor,
                    record: true,
                    fields,
                } if ty == "GameClockState" && ctor == "GameClockState" && fields.len() == 10
            ));
            ConstructorOutcome::Ok
        }
        EvalValue::Result(Err((error_ty, variant))) if error_ty == "ClockError" => {
            match variant.as_str() {
                "ZeroFixedTimestep" => ConstructorOutcome::ZeroFixedTimestep,
                "InvalidMaxFrameSteps" => ConstructorOutcome::InvalidMaxFrameSteps,
                "InvalidMaxFrameDelta" => ConstructorOutcome::InvalidMaxFrameDelta,
                other => panic!("unexpected constructor error variant {other}"),
            }
        }
        other => panic!("unexpected model result value: {other:?}"),
    }
}

#[test]
fn constructor_model_exactly_matches_all_648_boundary_configs() {
    with_parsed_file(WARRANT_SPEC, |parsed| {
        let file = parsed.expect("admit, layout and parse game-clock.warrant");
        check_warrant(file).expect("check game-clock.warrant");
        let (binder, model) = inspect_constructor_claim(file).expect("exact model claim shape");
        let times = [
            0,
            1,
            2,
            16_666_666,
            16_666_667,
            9_999_999_999,
            10_000_000_000,
            10_000_000_001,
            u64::MAX,
        ];
        let steps = [0, 1, 2, 8, 1023, 1024, 1025, u32::MAX];
        let mut seen = HashSet::with_capacity(648);
        let mut counts = BTreeMap::new();

        for fixed_dt_ns in times {
            for max_frame_steps in steps {
                for max_frame_delta_ns in times {
                    let config = GameClockConfig {
                        fixed_dt_ns,
                        max_frame_steps,
                        max_frame_delta_ns,
                    };
                    assert!(seen.insert((fixed_dt_ns, max_frame_steps, max_frame_delta_ns)));
                    compare_case(file, binder, model, config, SubjectAdapter::Native)
                        .unwrap_or_else(|error| panic!("boundary input {config:?}: {error}"));

                    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
                    let mut environment = HashMap::new();
                    environment.insert(binder, config_value(config));
                    evaluator.reset_work();
                    let value = evaluator
                        .eval(model, &environment)
                        .unwrap_or_else(|error| panic!("model failed at {config:?}: {error:?}"));
                    *counts.entry(outcome(&value)).or_insert(0usize) += 1;
                }
            }
        }

        assert_eq!(seen.len(), 648, "every distinct boundary triple must run");
        assert_eq!(counts.get(&ConstructorOutcome::Ok), Some(&105));
        assert_eq!(
            counts.get(&ConstructorOutcome::ZeroFixedTimestep),
            Some(&72)
        );
        assert_eq!(
            counts.get(&ConstructorOutcome::InvalidMaxFrameSteps),
            Some(&162)
        );
        assert_eq!(
            counts.get(&ConstructorOutcome::InvalidMaxFrameDelta),
            Some(&309)
        );
        assert_eq!(counts.values().sum::<usize>(), 648);
        println!("constructor boundary cases=648 outcomes={counts:?}");
    });
}

#[derive(Clone, Copy, Debug)]
enum InputStratum {
    FullWidth,
    Valid,
    ZeroDt,
    OversizedDt,
    InvalidSteps,
    InvalidDelta,
}

impl InputStratum {
    const ALL: [Self; 6] = [
        Self::FullWidth,
        Self::Valid,
        Self::ZeroDt,
        Self::OversizedDt,
        Self::InvalidSteps,
        Self::InvalidDelta,
    ];

    const fn index(self) -> usize {
        match self {
            Self::FullWidth => 0,
            Self::Valid => 1,
            Self::ZeroDt => 2,
            Self::OversizedDt => 3,
            Self::InvalidSteps => 4,
            Self::InvalidDelta => 5,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PropertyCase {
    config: GameClockConfig,
    stratum: InputStratum,
}

fn bounded(rng: &mut SplitMix64, min: u64, max: u64) -> u64 {
    gen_bounded_int(rng, i128::from(min), i128::from(max)) as u64
}

fn gen_property_case(rng: &mut SplitMix64) -> PropertyCase {
    let stratum = InputStratum::ALL[rng.bounded(InputStratum::ALL.len() as u64) as usize];
    let config = match stratum {
        InputStratum::FullWidth => GameClockConfig {
            fixed_dt_ns: gen_u64(rng),
            max_frame_steps: rng.next_u64() as u32,
            max_frame_delta_ns: gen_u64(rng),
        },
        InputStratum::Valid => {
            let fixed_dt_ns = bounded(rng, 1, HARD_MAX_DELTA_NS);
            GameClockConfig {
                fixed_dt_ns,
                max_frame_steps: bounded(rng, 1, u64::from(HARD_MAX_FRAME_STEPS)) as u32,
                max_frame_delta_ns: bounded(rng, fixed_dt_ns, HARD_MAX_DELTA_NS),
            }
        }
        InputStratum::ZeroDt => GameClockConfig {
            fixed_dt_ns: 0,
            max_frame_steps: rng.next_u64() as u32,
            max_frame_delta_ns: gen_u64(rng),
        },
        InputStratum::OversizedDt => GameClockConfig {
            fixed_dt_ns: bounded(rng, HARD_MAX_DELTA_NS + 1, u64::MAX),
            max_frame_steps: rng.next_u64() as u32,
            max_frame_delta_ns: gen_u64(rng),
        },
        InputStratum::InvalidSteps => GameClockConfig {
            fixed_dt_ns: bounded(rng, 1, HARD_MAX_DELTA_NS),
            max_frame_steps: [0, HARD_MAX_FRAME_STEPS + 1, u32::MAX][rng.bounded(3) as usize],
            max_frame_delta_ns: gen_u64(rng),
        },
        InputStratum::InvalidDelta => {
            let fixed_dt_ns = bounded(rng, 1, HARD_MAX_DELTA_NS);
            GameClockConfig {
                fixed_dt_ns,
                max_frame_steps: bounded(rng, 1, u64::from(HARD_MAX_FRAME_STEPS)) as u32,
                max_frame_delta_ns: if rng.bounded(2) == 0 {
                    bounded(rng, 0, fixed_dt_ns - 1)
                } else {
                    bounded(rng, HARD_MAX_DELTA_NS + 1, u64::MAX)
                },
            }
        }
    };
    PropertyCase { config, stratum }
}

fn shrink_property_case(
    initial: PropertyCase,
    property: &dyn Fn(&PropertyCase) -> bool,
) -> (PropertyCase, usize) {
    let mut current = initial;
    let mut total_steps = 0;
    let dt = shrink_int(
        i128::from(current.config.fixed_dt_ns),
        0,
        0,
        i128::from(u64::MAX),
        |candidate| {
            property(&PropertyCase {
                config: GameClockConfig {
                    fixed_dt_ns: candidate as u64,
                    ..current.config
                },
                ..current
            })
        },
        PROPERTY_SHRINK_STEPS,
    );
    current.config.fixed_dt_ns = dt.0 as u64;
    total_steps += dt.1;

    let step_count = shrink_int(
        i128::from(current.config.max_frame_steps),
        0,
        0,
        i128::from(u32::MAX),
        |candidate| {
            property(&PropertyCase {
                config: GameClockConfig {
                    max_frame_steps: candidate as u32,
                    ..current.config
                },
                ..current
            })
        },
        PROPERTY_SHRINK_STEPS,
    );
    current.config.max_frame_steps = step_count.0 as u32;
    total_steps += step_count.1;

    let delta = shrink_int(
        i128::from(current.config.max_frame_delta_ns),
        0,
        0,
        i128::from(u64::MAX),
        |candidate| {
            property(&PropertyCase {
                config: GameClockConfig {
                    max_frame_delta_ns: candidate as u64,
                    ..current.config
                },
                ..current
            })
        },
        PROPERTY_SHRINK_STEPS,
    );
    current.config.max_frame_delta_ns = delta.0 as u64;
    total_steps += delta.1;
    (current, total_steps)
}

#[test]
fn constructor_model_matches_seeded_stratified_full_width_properties() {
    with_parsed_file(WARRANT_SPEC, |parsed| {
        let file = parsed.expect("admit, layout and parse game-clock.warrant");
        check_warrant(file).expect("check game-clock.warrant");
        let (binder, model) = inspect_constructor_claim(file).expect("exact model claim shape");
        let stratum_counts = Cell::new([0usize; 6]);
        let mut outcome_counts = BTreeMap::new();
        let options = PbtOptions {
            num_tests: PROPERTY_CASES,
            max_retries: 1,
            max_shrink_steps: PROPERTY_SHRINK_STEPS,
            is_ci: false,
        };
        let property = |case: &PropertyCase| {
            compare_case(file, binder, model, case.config, SubjectAdapter::Native).is_ok()
        };
        let verdict = check_property(
            PROPERTY_SEED,
            &options,
            |rng| {
                let case = gen_property_case(rng);
                stratum_counts.update(|mut counts| {
                    counts[case.stratum.index()] += 1;
                    counts
                });
                Ok(case)
            },
            property,
            |case, property| shrink_property_case(case, property),
        );
        assert_eq!(
            verdict,
            Verdict::Pass,
            "seeded constructor PBT: {verdict:?}"
        );
        let strata = stratum_counts.get();
        assert_eq!(strata.iter().sum::<usize>(), PROPERTY_CASES);
        for (index, count) in strata.iter().enumerate() {
            assert!(
                *count > 0,
                "stratum {:?} was not generated",
                InputStratum::ALL[index]
            );
        }

        // Record the model's exact outcome mix over the generated cases. These
        // counts describe this seed and generator only, not the full domain.
        let mut rng = SplitMix64::new(PROPERTY_SEED);
        for _ in 0..PROPERTY_CASES {
            let case = gen_property_case(&mut rng);
            let mut evaluator = Evaluator::new(file, CodegenLimits::default());
            let mut environment = HashMap::new();
            environment.insert(binder, config_value(case.config));
            evaluator.reset_work();
            let value = evaluator
                .eval(model, &environment)
                .unwrap_or_else(|error| panic!("property input {:?}: {error:?}", case.config));
            *outcome_counts.entry(outcome(&value)).or_insert(0usize) += 1;
        }
        assert_eq!(outcome_counts.values().sum::<usize>(), PROPERTY_CASES);
        println!(
            "constructor property seed=0x{PROPERTY_SEED:016x} cases={PROPERTY_CASES} strata={strata:?} outcomes={outcome_counts:?}"
        );
    });
}

#[test]
fn constructor_controls_reject_three_same_path_subject_faults() {
    with_parsed_file(WARRANT_SPEC, |parsed| {
        let file = parsed.expect("admit, layout and parse game-clock.warrant");
        check_warrant(file).expect("check game-clock.warrant");
        let (binder, model) = inspect_constructor_claim(file).expect("exact model claim shape");

        let valid = GameClockConfig {
            fixed_dt_ns: 16_666_667,
            max_frame_steps: 8,
            max_frame_delta_ns: 250_000_000,
        };
        let error_precedence = GameClockConfig {
            fixed_dt_ns: HARD_MAX_DELTA_NS + 1,
            max_frame_steps: 0,
            max_frame_delta_ns: 0,
        };
        for (config, adapter, expected_case) in [
            (valid, SubjectAdapter::AlwaysErr, "always-Err valid config"),
            (
                error_precedence,
                SubjectAdapter::SwappedErrorPrecedence,
                "fixed-dt-before-step validation precedence",
            ),
            (
                valid,
                SubjectAdapter::WrongStateField,
                "tick_count changed from zero",
            ),
        ] {
            compare_case(file, binder, model, config, SubjectAdapter::Native)
                .unwrap_or_else(|error| panic!("native baseline {expected_case}: {error}"));
            let result = compare_case(file, binder, model, config, adapter);
            assert!(
                result.is_err(),
                "fault injection `{expected_case}` escaped the same model-comparison path"
            );
            let detail = result.expect_err("fault control must be rejected");
            assert!(
                detail.contains("constructor mismatch"),
                "fault control reached infrastructure failure rather than a model/subject mismatch: {detail}"
            );
            assert!(
                detail.contains(&format!("config={config:?}")),
                "mismatch omitted its concrete counterexample: {detail}"
            );
            assert!(
                detail.contains(&format!("adapter={adapter:?}")),
                "mismatch omitted its fault adapter: {detail}"
            );
            println!("fault-control {expected_case}: rejected ({detail})");
        }
    });
}

#[test]
fn constructor_claim_shape_fails_closed_when_tampered() {
    const CLAIM: &str = "  claim new_clock_matches_model:\n    forall c: GameClockConfig. new_clock(c) == new_clock_model(c)\n";
    let missing = WARRANT_SPEC.replace(CLAIM, "");
    assert_tamper_rejected_at_shape(
        &missing,
        "expected exactly one `new_clock_matches_model` claim, found 0",
    );

    let guarded = WARRANT_SPEC.replace(
        "forall c: GameClockConfig. new_clock(c) == new_clock_model(c)",
        "forall c: GameClockConfig | c.max_frame_steps > 0u32. new_clock(c) == new_clock_model(c)",
    );
    assert_tamper_rejected_at_shape(&guarded, "binder must be unrefined and non-inout");

    let reversed = WARRANT_SPEC.replace(
        "new_clock(c) == new_clock_model(c)",
        "new_clock_model(c) == new_clock(c)",
    );
    assert_tamper_rejected_at_shape(
        &reversed,
        "left side must call new_clock with its bare binder",
    );
}

fn assert_tamper_rejected_at_shape(source: &str, expected_reason: &str) {
    with_parsed_file(source, |parsed| {
        let file = parsed.expect("tampered document must remain admitted and parseable");
        check_warrant(file).expect("tampered claim must pass the Warrant checker");
        let error = inspect_constructor_claim(file)
            .expect_err("targeted AST-shape validation must reject the tamper");
        assert!(
            error.contains(expected_reason),
            "tamper failed at the wrong boundary: expected `{expected_reason}`, got `{error}`"
        );
    });
}

#[test]
fn property_shrinker_reduces_a_known_counterexample() {
    let initial = PropertyCase {
        config: GameClockConfig {
            fixed_dt_ns: 100,
            max_frame_steps: 7,
            max_frame_delta_ns: 250,
        },
        stratum: InputStratum::Valid,
    };
    let threshold_fault = |case: &PropertyCase| case.config.fixed_dt_ns <= 10;
    assert!(!threshold_fault(&initial));
    let (shrunk, steps) = shrink_property_case(initial, &threshold_fault);
    assert!(steps > 0, "the integrated shrinker must do work");
    assert!(
        !threshold_fault(&shrunk),
        "shrink result must preserve falsification"
    );
    assert!(shrunk.config.fixed_dt_ns < initial.config.fixed_dt_ns);
    println!("shrinker control: {initial:?} -> {shrunk:?} in {steps} steps");
}
