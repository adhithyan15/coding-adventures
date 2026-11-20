//! Deterministic admission properties and a bounded source-input fuzz run.
//!
//! This suite complements the proof tests. It generates actual source variation
//! around the admitted helpers, while a fixed refusal census keeps the boundary
//! cases reproducible. Fuzz outcome edges are explicit adapter-result markers;
//! they are not native line or branch coverage.

use coding_adventures_test_bootstrap::fuzz::{
    fuzz_target, CoverageBitmap, FuzzOptions, FuzzOutcome,
};
use coding_adventures_test_bootstrap::pbt::{check_property, PbtOptions};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_ir::{BinaryOp, Program, WirExpr, WirTerminator};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use typescript_warrant_ir::{lower_typescript_bytes, BridgeError, MAX_SOURCE_BYTES, MAX_TOKENS};

const SOURCE: &[u8] = include_bytes!("fixtures/digital.ts");
const PROPERTY_SEED: u64 = 0x5453_5749_525f_5032;
const PROPERTY_CASES: usize = 256;
const PROPERTY_STRATA: usize = 13;
// This campaign samples zero or one wrapper to keep its generated corpus
// small; it is not the bridge's parenthesis admission limit. The parser and
// adapter byte/token/AST budgets bound larger inputs.
const PROPERTY_PARENS: usize = 2;
const PROPERTY_COMMENT_STYLES: usize = 4;
const PROPERTY_OPERATOR_CHOICES: usize = 4;
const PROPERTY_SHRINK_STEPS: usize = 128;
const FUZZ_SEED: u64 = 0x5453_5749_525f_4632;
const FUZZ_ITERATIONS: usize = 256;

#[derive(Clone, Debug)]
struct AdmissionCase {
    stratum: usize,
    paren_depth: usize,
    comment_style: usize,
    operator_choice: usize,
    source: Vec<u8>,
    should_admit: bool,
}

/// Choose a bounded contract stratum, then vary its actual source spelling.
fn generated_case(rng: &mut SplitMix64) -> AdmissionCase {
    let stratum = rng.bounded(PROPERTY_STRATA as u64) as usize;
    let paren_depth = rng.bounded(PROPERTY_PARENS as u64) as usize;
    let comment_style = rng.bounded(PROPERTY_COMMENT_STYLES as u64) as usize;
    let operator_choice = rng.bounded(PROPERTY_OPERATOR_CHOICES as u64) as usize;
    admission_case(stratum, paren_depth, comment_style, operator_choice)
}

fn admission_case(
    stratum: usize,
    paren_depth: usize,
    comment_style: usize,
    operator_choice: usize,
) -> AdmissionCase {
    let source = std::str::from_utf8(SOURCE).expect("checked fixture bytes are UTF-8");
    let (mut text, should_admit) = match stratum {
        0 => (source.to_owned(), true),
        1 => (source.replacen("mask &", "((mask)) &", 1), true),
        2 => (
            source.replacen("1n << BigInt(slot)", "1n << (BigInt(slot))", 1),
            true,
        ),
        3 => (source.replacen("!== 0n", "=== 0n", 1), true),
        4 => (source.replacen("return mask |", "return mask &", 1), true),
        5 => (
            source.replacen("1n << BigInt(slot)", "2n << BigInt(slot)", 1),
            false,
        ),
        6 => (
            source.replacen("return mask |", "return mask << BigInt(slot)", 1),
            false,
        ),
        7 => (
            source.replacen("return mask & ~(1n << BigInt(slot));", "return ~mask;", 1),
            false,
        ),
        8 => (source.replacen("return (mask", "return\n  (mask", 1), false),
        9 => (source.replacen("mask: bigint", "mask: 'bigint'", 1), false),
        10 => (source.replacen("(mask &", "(\"mask\" &", 1), false),
        11 => (
            source.replacen("1n << BigInt(slot)", "1n << slot", 1),
            false,
        ),
        12 => (format!("{source}\nconst trailing = 0n;"), false),
        _ => panic!("unknown admission stratum {stratum}"),
    };

    // Transparent wrappers exercise real expression-node nesting without
    // changing the helper's operands. The maximum depth stays below the parser
    // rule cap and the adapter's fixed input budgets.
    let slot = wrap("slot", paren_depth);
    text = text.replace("BigInt(slot)", &format!("BigInt({slot})"));
    // This variation places grouping inside the intrinsic call; the source
    // adapter lowers transparent parentheses wherever the AST permits them.

    // Comments are trivia; Unicode appears only inside comments. All selected
    // comments are same-line so they do not trigger JavaScript's restricted
    // return production.
    match comment_style {
        0 => {}
        1 => text = text.replace("return ", "return /* λ */ "),
        2 => text = text.replace("mask &", "mask /* μ */ &"),
        3 => text = text.replace("BigInt(", "BigInt(/* slot π */"),
        _ => unreachable!(),
    }

    // The three choices cover distinct actual operator positions. They remain
    // admitted mutations and are checked as source variants, not normalized to
    // the expected formula by the adapter.
    match operator_choice {
        0 => {}
        1 => {
            if text.contains("!== 0n") {
                text = text.replacen("!== 0n", "=== 0n", 1);
            } else if text.contains("=== 0n") {
                text = text.replacen("=== 0n", "!== 0n", 1);
            }
        }
        2 => {
            if text.contains("mask & (") {
                text = text.replacen("mask & (", "mask | (", 1);
            } else if text.contains("mask | (") {
                text = text.replacen("mask | (", "mask & (", 1);
            }
        }
        3 => {
            if text.contains("return mask |") {
                text = text.replacen("return mask |", "return mask &", 1);
            } else if text.contains("return mask &") {
                text = text.replacen("return mask &", "return mask |", 1);
            }
        }
        _ => unreachable!(),
    }

    AdmissionCase {
        stratum,
        paren_depth,
        comment_style,
        operator_choice,
        source: text.into_bytes(),
        should_admit,
    }
}

fn wrap(expression: &str, depth: usize) -> String {
    format!("{}{}{}", "(".repeat(depth), expression, ")".repeat(depth))
}

fn property_holds(case: &AdmissionCase) -> bool {
    match lower_typescript_bytes(&case.source) {
        Ok(_) => case.should_admit,
        Err(_) => !case.should_admit,
    }
}

/// Reduce generated spelling while preserving the same failure predicate.
fn shrink_admission_case(
    case: AdmissionCase,
    property: &dyn Fn(&AdmissionCase) -> bool,
) -> (AdmissionCase, usize) {
    let candidates = std::iter::once(admission_case(case.stratum, 0, 0, 0))
        .chain((0..case.paren_depth).map(|depth| {
            admission_case(
                case.stratum,
                depth,
                case.comment_style,
                case.operator_choice,
            )
        }))
        .chain((0..case.comment_style).map(|style| {
            admission_case(case.stratum, case.paren_depth, style, case.operator_choice)
        }))
        .chain((0..case.operator_choice).map(|choice| {
            admission_case(case.stratum, case.paren_depth, case.comment_style, choice)
        }))
        .chain((0..case.stratum).map(|stratum| admission_case(stratum, 0, 0, 0)));
    let mut steps = 0;
    for candidate in candidates {
        steps += 1;
        if !property(&candidate) {
            return (candidate, steps.min(PROPERTY_SHRINK_STEPS));
        }
        if steps >= PROPERTY_SHRINK_STEPS {
            break;
        }
    }
    (case, 0)
}

#[test]
fn seeded_admission_properties_generate_syntax_and_refusal_cases() {
    let mut census_rng = SplitMix64::new(PROPERTY_SEED);
    let generated: Vec<_> = (0..PROPERTY_CASES)
        .map(|_| generated_case(&mut census_rng))
        .collect();
    let mut strata = [0usize; PROPERTY_STRATA];
    let mut paren_counts = [0usize; PROPERTY_PARENS];
    let mut comment_counts = [0usize; PROPERTY_COMMENT_STYLES];
    let mut operator_counts = [0usize; PROPERTY_OPERATOR_CHOICES];
    let mut unique_sources = std::collections::HashSet::new();
    for case in &generated {
        strata[case.stratum] += 1;
        paren_counts[case.paren_depth] += 1;
        comment_counts[case.comment_style] += 1;
        operator_counts[case.operator_choice] += 1;
        unique_sources.insert(case.source.clone());
        let result = lower_typescript_bytes(&case.source);
        assert_eq!(
            result.is_ok(),
            case.should_admit,
            "generated admission mismatch: stratum={}, paren_depth={}, comment_style={}, operator_choice={}, result={result:?}",
            case.stratum,
            case.paren_depth,
            case.comment_style,
            case.operator_choice
        );
    }
    assert!(
        strata.iter().all(|count| *count > 0),
        "every refusal stratum must run: {strata:?}"
    );
    assert!(
        unique_sources.len() > PROPERTY_CASES / 2,
        "generator must produce varied source, got {} unique inputs",
        unique_sources.len()
    );

    let options = PbtOptions {
        num_tests: PROPERTY_CASES,
        max_retries: 64,
        max_shrink_steps: PROPERTY_SHRINK_STEPS,
        is_ci: false,
    };
    let verdict = check_property(
        PROPERTY_SEED,
        &options,
        |rng| Ok(generated_case(rng)),
        property_holds,
        shrink_admission_case,
    );
    assert_eq!(verdict, Verdict::Pass, "seed={PROPERTY_SEED:#018x}");
    eprintln!(
        "TS-WIR admission PBT: seed={PROPERTY_SEED:#018x}, cases={PROPERTY_CASES}, unique_sources={}, strata={strata:?}, paren_depths={paren_counts:?}, comments={comment_counts:?}, operator_choices={operator_counts:?}, shrink_steps={PROPERTY_SHRINK_STEPS}, verdict=Pass",
        unique_sources.len()
    );
}

#[test]
fn bounded_source_fuzz_campaign_reports_all_target_calls() {
    let mut seeds = vec![SOURCE.to_vec()];
    for stratum in 1..PROPERTY_STRATA {
        let variant = (stratum * 3) % PROPERTY_PARENS;
        seeds.push(
            admission_case(
                stratum,
                variant,
                stratum % PROPERTY_COMMENT_STYLES,
                stratum % PROPERTY_OPERATOR_CHOICES,
            )
            .source,
        );
    }
    seeds.push(vec![0xff, 0xfe, 0xfd]);
    seeds.push(vec![b' '; MAX_SOURCE_BYTES + 1]);
    seeds.push(vec![b';'; MAX_TOKENS + 8]);
    let seed_count = seeds.len();
    let target_calls = AtomicUsize::new(0);

    let options = FuzzOptions {
        max_iterations: FUZZ_ITERATIONS,
        seed: FUZZ_SEED,
        max_input_len: MAX_SOURCE_BYTES,
        base_energy: 2,
    };
    let report = fuzz_target(&seeds, &options, |input, outcome_trace| {
        target_calls.fetch_add(1, Ordering::Relaxed);
        // These edge IDs describe adapter outcomes; they are not native code
        // coverage and do not claim to count compiler or Rust source branches.
        match catch_unwind(AssertUnwindSafe(|| lower_typescript_bytes(input))) {
            Ok(Ok(program)) => {
                outcome_trace.record_edge(1);
                record_lowered_shape(&program, outcome_trace);
                FuzzOutcome::Pass
            }
            Ok(Err(error)) => {
                outcome_trace.record_edge(10 + error_class(&error));
                FuzzOutcome::Pass
            }
            Err(_) => FuzzOutcome::Crash {
                kind: "typescript-warrant-ir-panic".to_string(),
                frames: vec!["lower_typescript_bytes".to_string()],
            },
        }
    });
    let observed_calls = target_calls.load(Ordering::Relaxed);
    assert!(
        report.is_clean(),
        "fuzz report: iterations={}, crashes={}, timeouts={}, ooms={}",
        report.iterations,
        report.crashes.len(),
        report.timeouts,
        report.ooms
    );
    assert_eq!(report.iterations, FUZZ_ITERATIONS);
    assert_eq!(
        observed_calls,
        seed_count + report.iterations,
        "target calls include initial corpus triage and mutation iterations"
    );
    assert!(
        report.admitted_seeds > 0,
        "at least the baseline source must be admitted"
    );
    assert!(
        report.total_edges >= 4,
        "source, helper, and refusal outcomes must register distinct adapter-outcome paths"
    );
    eprintln!(
        "TS-WIR source fuzz: seed={FUZZ_SEED:#018x}, seed_calls={seed_count}, mutation_iterations={}, total_target_calls={observed_calls}, admitted_seeds={}, adapter_outcome_edges={}, crashes={}, timeouts={}, ooms={}",
        report.iterations,
        report.admitted_seeds,
        report.total_edges,
        report.crashes.len(),
        report.timeouts,
        report.ooms,
    );
}

fn error_class(error: &BridgeError) -> usize {
    match error {
        BridgeError::SourceTooLarge { .. } => 1,
        BridgeError::InvalidUtf8 => 2,
        BridgeError::TooManyTokens { .. } => 3,
        BridgeError::Parse(_) => 4,
        BridgeError::TooManyAstNodes { .. } => 5,
        BridgeError::UnsupportedAst(_) => 6,
        BridgeError::TooManyWirNodes { .. } => 7,
        BridgeError::TypeCheck(_) => 8,
        BridgeError::InvalidNotContext => 9,
        BridgeError::RestrictedProductionLineTerminator => 10,
    }
}

fn record_lowered_shape(program: &Program, outcome_trace: &mut CoverageBitmap) {
    outcome_trace.record_edge(2 + program.functions.len());
    for function in &program.functions {
        if let Some(block) = function.blocks.first() {
            if let WirTerminator::Return(Some(expression)) = &block.terminator {
                record_expression_outcomes(expression, outcome_trace);
            }
        }
    }
}

fn record_expression_outcomes(expression: &WirExpr, outcome_trace: &mut CoverageBitmap) {
    match expression {
        WirExpr::Binary { op, lhs, rhs } => {
            let edge = match op {
                BinaryOp::BitAnd => 30,
                BinaryOp::BitOr => 31,
                BinaryOp::Shl => 32,
                BinaryOp::Eq => 33,
                BinaryOp::Ne => 34,
                _ => 35,
            };
            outcome_trace.record_edge(edge);
            record_expression_outcomes(lhs, outcome_trace);
            record_expression_outcomes(rhs, outcome_trace);
        }
        WirExpr::Unary { expr, .. } => {
            outcome_trace.record_edge(36);
            record_expression_outcomes(expr, outcome_trace);
        }
        WirExpr::ZExt { expr, .. } | WirExpr::Trunc { expr, .. } => {
            outcome_trace.record_edge(37);
            record_expression_outcomes(expr, outcome_trace);
        }
        WirExpr::ConstInt { .. } => outcome_trace.record_edge(38),
        WirExpr::Var(_) => outcome_trace.record_edge(39),
        _ => outcome_trace.record_edge(40),
    }
}
