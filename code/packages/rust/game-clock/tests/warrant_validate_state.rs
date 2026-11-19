//! Smoke-checks that the exact validator model is admitted and executes through
//! the native Warrant evaluator before building the broader assurance suite.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use coding_adventures_test_bootstrap::fuzz::{fuzz_target, FuzzOptions, FuzzOutcome};
use coding_adventures_test_bootstrap::pbt::{
    check_property, gen_bounded_int, gen_u64, shrink_int, PbtOptions,
};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, Declaration, Expression, ParserLimits,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_grammar::{
    checker::check_warrant,
    layout::{capture_layout, LayoutLimits},
};
use game_clock::{
    validate_state, ClockError, GameClockState, HARD_MAX_DELTA_NS, HARD_MAX_FRAME_STEPS,
};

#[path = "support/warrant_state.rs"]
mod shared_warrant;

const WARRANT_SPEC: &str = include_str!("../../../../specs/game-clock.warrant");
const PROPERTY_SEED: u64 = 0x4743_4c4b_5f56_414c;
const PROPERTY_CASES: usize = 2048;
const MAX_SHRINK_STEPS: usize = 512;

fn ungroup<'e, 's>(mut expression: &'e Expression<'s>) -> &'e Expression<'s> {
    while let Expression::Grouped { inner, .. } = expression {
        expression = inner;
    }
    expression
}

fn bare_call(expression: &Expression<'_>, function: &str, binder: &str) -> bool {
    matches!(
        ungroup(expression),
        Expression::Reference { name, widths, args: Some(args), .. }
            if widths.is_empty()
                && name.segments.as_slice() == [function]
                && args.len() == 1
                && matches!(
                    ungroup(&args[0]),
                    Expression::Reference { name, widths, args: None, .. }
                        if widths.is_empty() && name.segments.as_slice() == [binder]
                )
    )
}

fn validator_model_expression<'f, 's>(
    file: &'f coding_adventures_warrant_grammar::parser::WarrantFile<'s>,
) -> Result<(&'s str, &'f Expression<'s>), String> {
    let claims: Vec<_> = file
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Claim(claim) if claim.name == "validate_state_matches_model" => {
                Some(claim)
            }
            _ => None,
        })
        .collect();
    if claims.len() != 1 {
        return Err(format!(
            "expected one validator model claim, found {}",
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
        return Err("validator claim must be universally quantified".into());
    };
    if binders.len() != 1 {
        return Err(format!(
            "validator claim must have one binder, found {}",
            binders.len()
        ));
    }
    if binders[0].refinement.is_some() || binders[0].inout {
        return Err("validator binder must be bare, unrefined, and non-inout".into());
    }
    if !matches!(
        binders[0].param_type,
        coding_adventures_warrant_grammar::parser::TypeExpr::Named {
            name: "GameClockState",
            ..
        }
    ) {
        return Err("validator binder must be GameClockState".into());
    }
    let Expression::Binary {
        op: BinaryOp::Eq,
        left,
        right,
        ..
    } = ungroup(body)
    else {
        return Err("validator claim must be direct result equality".into());
    };
    if !bare_call(left, "validate_state", binders[0].name) {
        return Err("left side must call validate_state with the bare binder".into());
    }
    if !bare_call(right, "validate_state_model", binders[0].name) {
        return Err("right side must call validate_state_model with the same bare binder".into());
    }
    Ok((binders[0].name, right))
}

fn claim_gate_result(source: &str) -> Result<(), String> {
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default())
        .map_err(|error| format!("source admission failed: {error:?}"))?;
    let layout = capture_layout(&doc, LayoutLimits::default())
        .map_err(|error| format!("layout capture failed: {error:?}"))?;
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .map_err(|error| format!("Warrant parse failed: {error:?}"))?;
    validator_model_expression(&file).map(|_| ())
}

fn state_value(state: GameClockState) -> EvalValue {
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

fn decode_state_value(value: EvalValue) -> Result<GameClockState, String> {
    let EvalValue::Con {
        ty,
        index: 0,
        ctor,
        record: true,
        fields,
        ..
    } = value
    else {
        return Err("expected GameClockState record value".into());
    };
    if ty != "GameClockState" || ctor != "GameClockState" {
        return Err(format!("wrong record identity {ty}::{ctor}"));
    }
    if fields.len() != 10 {
        return Err(format!("expected ten state fields, found {}", fields.len()));
    }
    let mut fields = fields.into_iter();
    let tick_count = take_u64(&mut fields, "tick_count")?;
    let frame_count = take_u64(&mut fields, "frame_count")?;
    let elapsed_sim_ns = take_u64(&mut fields, "elapsed_sim_ns")?;
    let elapsed_wall_ns = take_u64(&mut fields, "elapsed_wall_ns")?;
    let accumulator_ns = take_u64(&mut fields, "accumulator_ns")?;
    let fixed_dt_ns = take_u64(&mut fields, "fixed_dt_ns")?;
    let max_frame_steps = match fields.next() {
        Some(EvalValue::Fixed(FixedTy::U32, value))
            if (0..=i128::from(u32::MAX)).contains(&value) =>
        {
            value as u32
        }
        _ => return Err("max_frame_steps must be U32 at field 6".into()),
    };
    let max_frame_delta_ns = take_u64(&mut fields, "max_frame_delta_ns")?;
    let total_dropped_ns = take_u64(&mut fields, "total_dropped_ns")?;
    let spiral_clamped_frames = take_u64(&mut fields, "spiral_clamped_frames")?;
    if fields.next().is_some() {
        return Err("unexpected trailing state field".into());
    }
    Ok(GameClockState {
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
    })
}

fn take_u64(fields: &mut impl Iterator<Item = EvalValue>, name: &str) -> Result<u64, String> {
    match fields.next() {
        Some(EvalValue::U64(value)) => Ok(value),
        _ => Err(format!("{name} must be U64 at its declared field position")),
    }
}

fn encode_state_bytes(input: GameClockState) -> Vec<u8> {
    let mut out = Vec::with_capacity(76);
    out.extend_from_slice(&input.tick_count.to_le_bytes());
    out.extend_from_slice(&input.frame_count.to_le_bytes());
    out.extend_from_slice(&input.elapsed_sim_ns.to_le_bytes());
    out.extend_from_slice(&input.elapsed_wall_ns.to_le_bytes());
    out.extend_from_slice(&input.accumulator_ns.to_le_bytes());
    out.extend_from_slice(&input.fixed_dt_ns.to_le_bytes());
    out.extend_from_slice(&input.max_frame_steps.to_le_bytes());
    out.extend_from_slice(&input.max_frame_delta_ns.to_le_bytes());
    out.extend_from_slice(&input.total_dropped_ns.to_le_bytes());
    out.extend_from_slice(&input.spiral_clamped_frames.to_le_bytes());
    out
}

fn decode_state_bytes(bytes: &[u8]) -> Result<GameClockState, String> {
    if bytes.len() != 76 {
        return Err(format!(
            "expected 76-byte state record, found {}",
            bytes.len()
        ));
    }
    let read_u64 = |offset: usize| {
        u64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .expect("checked record length"),
        )
    };
    Ok(GameClockState {
        tick_count: read_u64(0),
        frame_count: read_u64(8),
        elapsed_sim_ns: read_u64(16),
        elapsed_wall_ns: read_u64(24),
        accumulator_ns: read_u64(32),
        fixed_dt_ns: read_u64(40),
        max_frame_steps: u32::from_le_bytes(
            bytes[48..52].try_into().expect("checked record length"),
        ),
        max_frame_delta_ns: read_u64(52),
        total_dropped_ns: read_u64(60),
        spiral_clamped_frames: read_u64(68),
    })
}

fn bytes_word(bytes: &[u8], offset: usize) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let mut word = [0u8; 8];
    for (index, byte) in word.iter_mut().enumerate() {
        *byte = bytes.get(offset + index).copied().unwrap_or(0);
    }
    u64::from_le_bytes(word)
}

/// Maps mutated bytes to a well-typed, semantically valid public state. This
/// target exercises the valid-state lane directly; malformed shape testing is
/// a separate byte-decoder campaign below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FuzzStratum {
    Valid,
    ZeroDt,
    OversizedDt,
    InvalidSteps,
    InvalidDelta,
    InvalidAccumulator,
    ProductOverflow,
    ElapsedMismatch,
    ZeroOutranksCorruption,
    CorruptionOutranksOverflow,
    OverflowOutranksMismatch,
    FullWidth,
}

impl FuzzStratum {
    const ALL: [Self; 12] = [
        Self::Valid,
        Self::ZeroDt,
        Self::OversizedDt,
        Self::InvalidSteps,
        Self::InvalidDelta,
        Self::InvalidAccumulator,
        Self::ProductOverflow,
        Self::ElapsedMismatch,
        Self::ZeroOutranksCorruption,
        Self::CorruptionOutranksOverflow,
        Self::OverflowOutranksMismatch,
        Self::FullWidth,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Valid => 0,
            Self::ZeroDt => 1,
            Self::OversizedDt => 2,
            Self::InvalidSteps => 3,
            Self::InvalidDelta => 4,
            Self::InvalidAccumulator => 5,
            Self::ProductOverflow => 6,
            Self::ElapsedMismatch => 7,
            Self::ZeroOutranksCorruption => 8,
            Self::CorruptionOutranksOverflow => 9,
            Self::OverflowOutranksMismatch => 10,
            Self::FullWidth => 11,
        }
    }
}

/// Maps every mutated byte sequence to one deterministic, full-width typed
/// state in an explicit semantic stratum. This is intentionally distinct from
/// malformed 76-byte transport testing below.
fn typed_state_from_bytes(bytes: &[u8]) -> (GameClockState, FuzzStratum) {
    let selector = bytes_word(bytes, 40) as usize % FuzzStratum::ALL.len();
    let stratum = FuzzStratum::ALL[selector];
    let a = bytes_word(bytes, 0);
    let b = bytes_word(bytes, 8);
    let c = bytes_word(bytes, 16);
    let d = bytes_word(bytes, 24);
    let e = bytes_word(bytes, 32);
    let ordinary_dt = a % HARD_MAX_DELTA_NS + 1;
    let tick_limit = u64::MAX / ordinary_dt;
    let safe_tick = if tick_limit == u64::MAX {
        b
    } else {
        b % (tick_limit + 1)
    };
    let safe_elapsed = (u128::from(safe_tick) * u128::from(ordinary_dt)) as u64;
    let mut input = match stratum {
        FuzzStratum::Valid => state(
            safe_tick,
            safe_elapsed,
            ordinary_dt,
            (c % u64::from(HARD_MAX_FRAME_STEPS) + 1) as u32,
            ordinary_dt + d % (HARD_MAX_DELTA_NS - ordinary_dt + 1),
            e % ordinary_dt,
        ),
        FuzzStratum::ZeroDt => state(b, c, 0, d as u32, e, a),
        FuzzStratum::OversizedDt => state(b, c, u64::MAX, 1, 0, 0),
        FuzzStratum::InvalidSteps => state(
            b,
            c,
            ordinary_dt,
            [0, HARD_MAX_FRAME_STEPS + 1, u32::MAX][(d % 3) as usize],
            ordinary_dt,
            0,
        ),
        FuzzStratum::InvalidDelta => state(0, 0, ordinary_dt, 1, c % ordinary_dt, 0),
        FuzzStratum::InvalidAccumulator => state(
            0,
            0,
            ordinary_dt,
            1,
            ordinary_dt,
            ordinary_dt + a % (u64::MAX - ordinary_dt + 1),
        ),
        FuzzStratum::ProductOverflow => state(
            u64::MAX,
            c,
            2 + a % (HARD_MAX_DELTA_NS - 1),
            1,
            2 + a % (HARD_MAX_DELTA_NS - 1),
            0,
        ),
        FuzzStratum::ElapsedMismatch => state(
            safe_tick,
            safe_elapsed.wrapping_add(1),
            ordinary_dt,
            1,
            ordinary_dt,
            0,
        ),
        FuzzStratum::ZeroOutranksCorruption => state(u64::MAX, c, 0, 0, 0, u64::MAX),
        FuzzStratum::CorruptionOutranksOverflow => state(u64::MAX, c, 2, 0, 0, 2),
        FuzzStratum::OverflowOutranksMismatch => state(u64::MAX, 0, 2, 1, 2, 0),
        FuzzStratum::FullWidth => arbitrary_state(&mut SplitMix64::new(a ^ b ^ c ^ d ^ e)),
    };
    input.frame_count = a;
    input.elapsed_wall_ns = b;
    input.total_dropped_ns = c;
    input.spiral_clamped_frames = d;
    (input, stratum)
}

fn result_value(result: Result<(), ClockError>) -> EvalValue {
    EvalValue::Result(match result {
        Ok(()) => Ok(Box::new(EvalValue::Bool(true))),
        Err(error) => Err((
            "ClockError".into(),
            match error {
                ClockError::ZeroFixedTimestep => "ZeroFixedTimestep",
                ClockError::InvalidMaxFrameSteps => "InvalidMaxFrameSteps",
                ClockError::InvalidMaxFrameDelta => "InvalidMaxFrameDelta",
                ClockError::CorruptedStateInvariant => "CorruptedStateInvariant",
                ClockError::TimestampOverflow => "TimestampOverflow",
            }
            .into(),
        )),
    })
}

fn state(
    tick_count: u64,
    elapsed_sim_ns: u64,
    fixed_dt_ns: u64,
    max_frame_steps: u32,
    max_frame_delta_ns: u64,
    accumulator_ns: u64,
) -> GameClockState {
    GameClockState {
        tick_count,
        frame_count: 0,
        elapsed_sim_ns,
        elapsed_wall_ns: 0,
        accumulator_ns,
        fixed_dt_ns,
        max_frame_steps,
        max_frame_delta_ns,
        total_dropped_ns: 0,
        spiral_clamped_frames: 0,
    }
}

fn oracle(input: GameClockState) -> Result<(), ClockError> {
    if input.fixed_dt_ns == 0 {
        return Err(ClockError::ZeroFixedTimestep);
    }
    if input.fixed_dt_ns > HARD_MAX_DELTA_NS {
        return Err(ClockError::CorruptedStateInvariant);
    }
    if input.max_frame_steps == 0 || input.max_frame_steps > HARD_MAX_FRAME_STEPS {
        return Err(ClockError::CorruptedStateInvariant);
    }
    if input.max_frame_delta_ns < input.fixed_dt_ns || input.max_frame_delta_ns > HARD_MAX_DELTA_NS
    {
        return Err(ClockError::CorruptedStateInvariant);
    }
    if input.accumulator_ns >= input.fixed_dt_ns {
        return Err(ClockError::CorruptedStateInvariant);
    }
    let product = u128::from(input.tick_count) * u128::from(input.fixed_dt_ns);
    if product > u128::from(u64::MAX) {
        return Err(ClockError::TimestampOverflow);
    }
    if input.elapsed_sim_ns != product as u64 {
        return Err(ClockError::CorruptedStateInvariant);
    }
    Ok(())
}

fn deliberately_broken_oracle(input: GameClockState) -> Result<(), ClockError> {
    // Fault control: accidentally erases the accumulator guard after the
    // preceding configuration guards. This must disagree on a reachable input.
    let mut input = input;
    if input.fixed_dt_ns > 0
        && input.fixed_dt_ns <= HARD_MAX_DELTA_NS
        && input.max_frame_steps > 0
        && input.max_frame_steps <= HARD_MAX_FRAME_STEPS
        && input.max_frame_delta_ns >= input.fixed_dt_ns
        && input.max_frame_delta_ns <= HARD_MAX_DELTA_NS
        && input.accumulator_ns >= input.fixed_dt_ns
    {
        input.accumulator_ns = 0;
    }
    oracle(input)
}

fn eval_model(
    file: &coding_adventures_warrant_grammar::parser::WarrantFile<'_>,
    binder: &str,
    model: &Expression<'_>,
    input: GameClockState,
) -> Result<EvalValue, String> {
    let mut environment = HashMap::new();
    let encoded = state_value(input);
    if decode_state_value(encoded.clone())? != input {
        return Err("ten-field model adapter did not round-trip".into());
    }
    environment.insert(binder, encoded);
    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    evaluator.reset_work();
    evaluator
        .eval(model, &environment)
        .map_err(|error| format!("Warrant evaluator error: {error:?}"))
}

fn compare_all(
    file: &coding_adventures_warrant_grammar::parser::WarrantFile<'_>,
    binder: &str,
    model: &Expression<'_>,
    input: GameClockState,
) -> Result<(), String> {
    let expected = result_value(oracle(input));
    let native = result_value(validate_state(&input));
    let evaluated = eval_model(file, binder, model, input)?;
    if native != expected || evaluated != expected {
        return Err(format!(
            "validator disagreement for {input:?}: native={native:?}, Warrant={evaluated:?}, oracle={expected:?}"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum InputStratum {
    FullWidth,
    Valid,
    ZeroDt,
    OversizedDt,
    InvalidSteps,
    InvalidDelta,
    InvalidAccumulator,
    ProductOverflow,
    ElapsedMismatch,
}

impl InputStratum {
    const ALL: [Self; 9] = [
        Self::FullWidth,
        Self::Valid,
        Self::ZeroDt,
        Self::OversizedDt,
        Self::InvalidSteps,
        Self::InvalidDelta,
        Self::InvalidAccumulator,
        Self::ProductOverflow,
        Self::ElapsedMismatch,
    ];

    const fn index(self) -> usize {
        match self {
            Self::FullWidth => 0,
            Self::Valid => 1,
            Self::ZeroDt => 2,
            Self::OversizedDt => 3,
            Self::InvalidSteps => 4,
            Self::InvalidDelta => 5,
            Self::InvalidAccumulator => 6,
            Self::ProductOverflow => 7,
            Self::ElapsedMismatch => 8,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PropertyCase {
    state: GameClockState,
    stratum: InputStratum,
}

fn bounded(rng: &mut SplitMix64, min: u64, max: u64) -> u64 {
    gen_bounded_int(rng, i128::from(min), i128::from(max)) as u64
}

fn arbitrary_state(rng: &mut SplitMix64) -> GameClockState {
    GameClockState {
        tick_count: gen_u64(rng),
        frame_count: gen_u64(rng),
        elapsed_sim_ns: gen_u64(rng),
        elapsed_wall_ns: gen_u64(rng),
        accumulator_ns: gen_u64(rng),
        fixed_dt_ns: gen_u64(rng),
        max_frame_steps: rng.next_u64() as u32,
        max_frame_delta_ns: gen_u64(rng),
        total_dropped_ns: gen_u64(rng),
        spiral_clamped_frames: gen_u64(rng),
    }
}

fn gen_case(rng: &mut SplitMix64) -> PropertyCase {
    let stratum = InputStratum::ALL[rng.bounded(InputStratum::ALL.len() as u64) as usize];
    let mut input = match stratum {
        InputStratum::FullWidth => arbitrary_state(rng),
        InputStratum::Valid => {
            let fixed_dt_ns = bounded(rng, 1, HARD_MAX_DELTA_NS);
            let tick_count = bounded(rng, 0, u64::MAX / fixed_dt_ns);
            let max_frame_delta_ns = bounded(rng, fixed_dt_ns, HARD_MAX_DELTA_NS);
            state(
                tick_count,
                tick_count * fixed_dt_ns,
                fixed_dt_ns,
                bounded(rng, 1, u64::from(HARD_MAX_FRAME_STEPS)) as u32,
                max_frame_delta_ns,
                bounded(rng, 0, fixed_dt_ns - 1),
            )
        }
        InputStratum::ZeroDt => state(gen_u64(rng), gen_u64(rng), 0, 0, gen_u64(rng), gen_u64(rng)),
        InputStratum::OversizedDt => {
            let fixed_dt_ns = bounded(rng, HARD_MAX_DELTA_NS + 1, u64::MAX);
            state(gen_u64(rng), gen_u64(rng), fixed_dt_ns, 0, 0, 0)
        }
        InputStratum::InvalidSteps => {
            let fixed_dt_ns = bounded(rng, 1, HARD_MAX_DELTA_NS);
            state(
                gen_u64(rng),
                gen_u64(rng),
                fixed_dt_ns,
                [0, HARD_MAX_FRAME_STEPS + 1, u32::MAX][rng.bounded(3) as usize],
                fixed_dt_ns,
                0,
            )
        }
        InputStratum::InvalidDelta => {
            let fixed_dt_ns = bounded(rng, 1, HARD_MAX_DELTA_NS);
            let max_frame_delta_ns = if rng.bounded(2) == 0 {
                bounded(rng, 0, fixed_dt_ns - 1)
            } else {
                bounded(rng, HARD_MAX_DELTA_NS + 1, u64::MAX)
            };
            state(0, 0, fixed_dt_ns, 1, max_frame_delta_ns, 0)
        }
        InputStratum::InvalidAccumulator => {
            let fixed_dt_ns = bounded(rng, 1, HARD_MAX_DELTA_NS);
            state(0, 0, fixed_dt_ns, 1, fixed_dt_ns, fixed_dt_ns)
        }
        InputStratum::ProductOverflow => {
            let fixed_dt_ns = bounded(rng, 2, HARD_MAX_DELTA_NS);
            state(u64::MAX, 0, fixed_dt_ns, 1, fixed_dt_ns, 0)
        }
        InputStratum::ElapsedMismatch => {
            let fixed_dt_ns = bounded(rng, 2, HARD_MAX_DELTA_NS);
            state(1, 0, fixed_dt_ns, 1, fixed_dt_ns, 0)
        }
    };
    // Vary ignored fields in every stratum, not only arbitrary full-width
    // records, so targeted valid/error inputs exercise all ten mapped fields.
    input.frame_count = gen_u64(rng);
    input.elapsed_wall_ns = gen_u64(rng);
    input.total_dropped_ns = gen_u64(rng);
    input.spiral_clamped_frames = gen_u64(rng);
    PropertyCase {
        state: input,
        stratum,
    }
}

fn shrink_case(
    initial: PropertyCase,
    property: &dyn Fn(&PropertyCase) -> bool,
) -> (PropertyCase, usize) {
    let mut current = initial;
    let mut shrink_steps = 0;
    macro_rules! shrink_field {
        ($field:ident, $max:expr) => {{
            let outcome = shrink_int(
                i128::from(current.state.$field),
                0,
                0,
                i128::from($max),
                |candidate| {
                    property(&PropertyCase {
                        state: GameClockState {
                            $field: candidate as _,
                            ..current.state
                        },
                        ..current
                    })
                },
                MAX_SHRINK_STEPS,
            );
            current.state.$field = outcome.0 as _;
            shrink_steps += outcome.1;
        }};
    }
    shrink_field!(tick_count, u64::MAX);
    shrink_field!(elapsed_sim_ns, u64::MAX);
    shrink_field!(accumulator_ns, u64::MAX);
    shrink_field!(fixed_dt_ns, u64::MAX);
    shrink_field!(max_frame_steps, u32::MAX);
    shrink_field!(max_frame_delta_ns, u64::MAX);
    (current, shrink_steps)
}

#[test]
fn checked_mul_validator_model_evaluates_and_matches_public_api_smoke_cases() {
    let doc = SourceDocument::admit(WARRANT_SPEC.as_bytes(), SourceLimits::default())
        .expect("admit Warrant source");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("capture Warrant layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse Warrant");
    check_warrant(&file).expect("typecheck Warrant model and direct validator claim");
    let (binder, model) = validator_model_expression(&file).expect("exact validator claim shape");

    let cases = [
        ("valid", state(3, 6, 2, 8, 250_000_000, 1)),
        ("zero fixed timestep", state(0, 0, 0, 8, 250_000_000, 0)),
        ("product overflow", state(u64::MAX, 0, 2, 8, 250_000_000, 0)),
        ("elapsed mismatch", state(3, 5, 2, 8, 250_000_000, 0)),
        (
            "zero outranks other invalid fields",
            state(u64::MAX, 0, 0, 0, 0, u64::MAX),
        ),
    ];

    for (label, input) in cases {
        let expected = result_value(validate_state(&input));
        let mut environment = HashMap::new();
        environment.insert(binder, state_value(input));
        let actual = Evaluator::new(&file, CodegenLimits::default())
            .eval(model, &environment)
            .unwrap_or_else(|error| panic!("{label}: Warrant evaluator failed: {error:?}"));
        assert_eq!(actual, expected, "{label}: model/API mismatch");
    }
}

#[test]
fn ten_field_evaluator_adapter_round_trips_and_rejects_malformed_records() {
    let original = GameClockState {
        tick_count: 11,
        frame_count: 22,
        elapsed_sim_ns: 33,
        elapsed_wall_ns: 44,
        accumulator_ns: 55,
        fixed_dt_ns: 66,
        max_frame_steps: 77,
        max_frame_delta_ns: 88,
        total_dropped_ns: 99,
        spiral_clamped_frames: 111,
    };
    let encoded = state_value(original);
    assert_eq!(decode_state_value(encoded.clone()), Ok(original));

    let EvalValue::Con {
        ty,
        index,
        ctor,
        record,
        fields,
    } = encoded
    else {
        panic!("state adapter must emit a record")
    };
    let rebuild = |new_ty: &str,
                   new_index: usize,
                   new_ctor: &str,
                   new_record: bool,
                   fields: Vec<EvalValue>| {
        EvalValue::Con {
            ty: new_ty.to_string(),
            index: new_index,
            ctor: new_ctor.to_string(),
            record: new_record,
            fields,
        }
    };
    let mut missing = fields.clone();
    missing.pop();
    let mut extra = fields.clone();
    extra.push(EvalValue::U64(0));
    let mut wrong_kind = fields.clone();
    wrong_kind[0] = EvalValue::Bool(true);
    let mut wrong_step_type = fields.clone();
    wrong_step_type[6] = EvalValue::U64(8);
    for malformed in [
        rebuild(&ty, index, &ctor, record, missing),
        rebuild(&ty, index, &ctor, record, extra),
        rebuild(&ty, index, &ctor, record, wrong_kind),
        rebuild(&ty, index, &ctor, record, wrong_step_type),
        rebuild("Other", index, &ctor, record, fields.clone()),
        rebuild(&ty, index + 1, &ctor, record, fields.clone()),
        rebuild(&ty, index, "Other", record, fields.clone()),
        rebuild(&ty, index, &ctor, false, fields.clone()),
    ] {
        assert!(
            decode_state_value(malformed).is_err(),
            "malformed record admitted"
        );
    }
    assert_eq!(
        decode_state_bytes(&encode_state_bytes(original)),
        Ok(original),
        "76-byte transport preserves all ten distinct field sentinels"
    );
    let bytes = encode_state_bytes(original);
    assert_eq!(u64::from_le_bytes(bytes[0..8].try_into().unwrap()), 11);
    assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 22);
    assert_eq!(u64::from_le_bytes(bytes[16..24].try_into().unwrap()), 33);
    assert_eq!(u64::from_le_bytes(bytes[24..32].try_into().unwrap()), 44);
    assert_eq!(u64::from_le_bytes(bytes[32..40].try_into().unwrap()), 55);
    assert_eq!(u64::from_le_bytes(bytes[40..48].try_into().unwrap()), 66);
    assert_eq!(u32::from_le_bytes(bytes[48..52].try_into().unwrap()), 77);
    assert_eq!(u64::from_le_bytes(bytes[52..60].try_into().unwrap()), 88);
    assert_eq!(u64::from_le_bytes(bytes[60..68].try_into().unwrap()), 99);
    assert_eq!(u64::from_le_bytes(bytes[68..76].try_into().unwrap()), 111);
}

#[test]
fn shared_sat_replay_adapter_matches_warrant_on_exact_branch_results() {
    for input in [
        state(0, 0, 1, 1, 1, 0),
        state(0, 0, 0, 0, 0, u64::MAX),
        state(0, 0, HARD_MAX_DELTA_NS + 1, 1, 0, 0),
        state(u64::MAX, 0, 2, 0, 2, 0),
        state(u64::MAX, 0, 2, 1, 2, 0),
        state(3, 5, 2, 1, 2, 0),
    ] {
        let public = validate_state(&input);
        assert_eq!(
            shared_warrant::evaluate_validator(input).unwrap(),
            shared_warrant::expected_value(public),
            "shared Warrant replay mismatch for {input:?}"
        );
    }
}

#[test]
fn validator_claim_ast_gate_rejects_missing_guarded_reversed_and_rebound_claims() {
    let claim = "  claim validate_state_matches_model:\n    forall s: GameClockState. validate_state(s) == validate_state_model(s)\n";
    assert_eq!(
        WARRANT_SPEC.matches(claim).count(),
        1,
        "fixture claim text is unique"
    );
    assert!(claim_gate_result(WARRANT_SPEC).is_ok());

    let mutations = [
        ("missing claim", WARRANT_SPEC.replace(claim, "")),
        (
            "guarded equality",
            WARRANT_SPEC.replace(
                "forall s: GameClockState. validate_state(s) == validate_state_model(s)",
                "forall s: GameClockState. if true then validate_state(s) == validate_state_model(s) else true",
            ),
        ),
        (
            "reversed operands",
            WARRANT_SPEC.replace(
                "validate_state(s) == validate_state_model(s)",
                "validate_state_model(s) == validate_state(s)",
            ),
        ),
        (
            "wrong binder reference",
            WARRANT_SPEC.replace(
                "forall s: GameClockState. validate_state(s) == validate_state_model(s)",
                "forall s: GameClockState. validate_state(s) == validate_state_model(other)",
            ),
        ),
        (
            "wrong subject",
            WARRANT_SPEC.replace(
                "validate_state(s) == validate_state_model(s)",
                "advance_frame(s, 0u64) == validate_state_model(s)",
            ),
        ),
        (
            "extra binder",
            WARRANT_SPEC.replace(
                "forall s: GameClockState. validate_state(s) == validate_state_model(s)",
                "forall s: GameClockState, n: U64. validate_state(s) == validate_state_model(s)",
            ),
        ),
        (
            "duplicate bypass claim",
            WARRANT_SPEC.replace(claim, &format!("{claim}{claim}")),
        ),
    ];
    for (label, source) in mutations {
        assert!(
            !claim_gate_result(&source).is_ok(),
            "claim AST gate admitted {label}"
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    Ok,
    ZeroFixedTimestep,
    CorruptedStateInvariant,
    TimestampOverflow,
}

fn outcome(result: Result<(), ClockError>) -> Outcome {
    match result {
        Ok(()) => Outcome::Ok,
        Err(ClockError::ZeroFixedTimestep) => Outcome::ZeroFixedTimestep,
        Err(ClockError::CorruptedStateInvariant) => Outcome::CorruptedStateInvariant,
        Err(ClockError::TimestampOverflow) => Outcome::TimestampOverflow,
        Err(other) => panic!("validate_state produced impossible config error: {other:?}"),
    }
}

fn outcome_index(result: Result<(), ClockError>) -> usize {
    match outcome(result) {
        Outcome::Ok => 0,
        Outcome::ZeroFixedTimestep => 1,
        Outcome::CorruptedStateInvariant => 2,
        Outcome::TimestampOverflow => 3,
    }
}

#[test]
fn validator_boundary_vectors_match_warrant_public_api_and_independent_u128_oracle() {
    let doc = SourceDocument::admit(WARRANT_SPEC.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
    check_warrant(&file).unwrap();
    let (binder, model) = validator_model_expression(&file).expect("exact validator claim shape");
    let max_tick_for_two = u64::MAX / 2;
    let cases = [
        state(0, 0, 1, 1, 1, 0),
        state(1, 2, 2, 1, 2, 0),
        state(u64::MAX, u64::MAX, 1, 1, 1, 0),
        state(max_tick_for_two, u64::MAX - 1, 2, 1, 2, 0),
        state(max_tick_for_two + 1, 0, 2, 1, 2, 0),
        state(3, 5, 2, 1, 2, 0),
        state(0, 0, 0, 0, 0, u64::MAX),
        state(u64::MAX, 0, 0, 0, 0, u64::MAX),
        state(u64::MAX, 0, 2, 0, 0, 2),
        state(u64::MAX, 0, 2, 1, 1, 0),
        state(u64::MAX, 0, 2, 1, u64::MAX, 0),
        state(0, 0, 2, 1, 2, 2),
        state(0, 0, 10_000_000_000, 1024, 10_000_000_000, 9_999_999_999),
        state(0, 0, 10_000_000_001, 1, 10_000_000_001, 0),
    ];
    let mut outcomes = BTreeMap::new();
    for (index, input) in cases.into_iter().enumerate() {
        let before = input;
        compare_all(&file, binder, model, input)
            .unwrap_or_else(|error| panic!("branch vector {index}, {input:?}: {error}"));
        assert_eq!(input, before, "validator modified an input field");
        *outcomes
            .entry(outcome(validate_state(&input)))
            .or_insert(0usize) += 1;
    }
    assert!(outcomes.contains_key(&Outcome::Ok));
    assert!(outcomes.contains_key(&Outcome::ZeroFixedTimestep));
    assert!(outcomes.contains_key(&Outcome::CorruptedStateInvariant));
    assert!(outcomes.contains_key(&Outcome::TimestampOverflow));
    assert_eq!(outcomes.values().sum::<usize>(), cases.len());

    let mut boundary_cases = Vec::new();
    for dt in [1, 2, 3, HARD_MAX_DELTA_NS] {
        let quotient = u64::MAX / dt;
        for tick in [quotient.saturating_sub(1), quotient] {
            boundary_cases.push(state(
                tick,
                (u128::from(tick) * u128::from(dt)) as u64,
                dt,
                1,
                dt,
                0,
            ));
        }
        if quotient < u64::MAX {
            let tick = quotient + 1;
            boundary_cases.push(state(tick, 0, dt, 1, dt, 0));
        }
        let tick = quotient.saturating_sub(1);
        let product = (u128::from(tick) * u128::from(dt)) as u64;
        if product > 0 {
            boundary_cases.push(state(tick, product - 1, dt, 1, dt, 0));
        }
        if product < u64::MAX {
            boundary_cases.push(state(tick, product + 1, dt, 1, dt, 0));
        }
    }
    for dt in [0, HARD_MAX_DELTA_NS + 1, u64::MAX] {
        boundary_cases.push(state(0, 0, dt, 1, dt, 0));
    }
    for steps in [
        0,
        1,
        HARD_MAX_FRAME_STEPS,
        HARD_MAX_FRAME_STEPS + 1,
        u32::MAX,
    ] {
        boundary_cases.push(state(0, 0, 1, steps, 1, 0));
    }
    for (dt, delta) in [
        (1, 0),
        (1, 1),
        (HARD_MAX_DELTA_NS, HARD_MAX_DELTA_NS),
        (HARD_MAX_DELTA_NS, HARD_MAX_DELTA_NS + 1),
    ] {
        boundary_cases.push(state(0, 0, dt, 1, delta, 0));
    }
    for accumulator in [0, 1, HARD_MAX_DELTA_NS - 1, HARD_MAX_DELTA_NS] {
        let dt = HARD_MAX_DELTA_NS;
        boundary_cases.push(state(0, 0, dt, 1, dt, accumulator));
    }
    let mut boundary_outcomes = [0usize; 4];
    for (index, input) in boundary_cases.iter().copied().enumerate() {
        compare_all(&file, binder, model, input)
            .unwrap_or_else(|error| panic!("boundary threshold {index}, {input:?}: {error}"));
        boundary_outcomes[outcome_index(validate_state(&input))] += 1;
    }
    println!(
        "validator threshold vectors={} outcomes={boundary_outcomes:?}",
        boundary_cases.len()
    );

    // Each ignored counter is independently changed to zero, ordinary, and
    // maximum values. The six read fields each receive an isolated mutation.
    let valid = state(7, 7 * 3, 3, 8, 250_000_000, 2);
    type IgnoredFieldMutator = (&'static str, fn(GameClockState, u64) -> GameClockState);
    let ignored_fields: [IgnoredFieldMutator; 4] = [
        ("frame_count", |mut s, value| {
            s.frame_count = value;
            s
        }),
        ("elapsed_wall_ns", |mut s, value| {
            s.elapsed_wall_ns = value;
            s
        }),
        ("total_dropped_ns", |mut s, value| {
            s.total_dropped_ns = value;
            s
        }),
        ("spiral_clamped_frames", |mut s, value| {
            s.spiral_clamped_frames = value;
            s
        }),
    ];
    for (name, mutate) in ignored_fields {
        for value in [0, 17, u64::MAX] {
            let changed = mutate(valid, value);
            compare_all(&file, binder, model, changed).unwrap_or_else(|error| {
                panic!("ignored-counter metamorphic {name}={value}: {error}")
            });
            assert_eq!(validate_state(&changed), Ok(()));
        }
    }
    for (name, changed) in [
        (
            "tick_count",
            GameClockState {
                tick_count: 8,
                ..valid
            },
        ),
        (
            "elapsed_sim_ns",
            GameClockState {
                elapsed_sim_ns: 22,
                ..valid
            },
        ),
        (
            "accumulator_ns",
            GameClockState {
                accumulator_ns: 3,
                ..valid
            },
        ),
        (
            "fixed_dt_ns",
            GameClockState {
                fixed_dt_ns: 0,
                ..valid
            },
        ),
        (
            "max_frame_steps",
            GameClockState {
                max_frame_steps: 0,
                ..valid
            },
        ),
        (
            "max_frame_delta_ns",
            GameClockState {
                max_frame_delta_ns: 2,
                ..valid
            },
        ),
    ] {
        compare_all(&file, binder, model, changed)
            .unwrap_or_else(|error| panic!("read-field mutation {name}: {error}"));
        assert_ne!(
            validate_state(&changed),
            Ok(()),
            "{name} mutation was not observed"
        );
    }
    // Keep check_warrant's full validation result alive in this test and make
    // accidental removal of the checker an explicit failure.
    println!(
        "validator branch vectors={} outcomes={outcomes:?}",
        cases.len()
    );
}

#[test]
fn validator_model_matches_seeded_full_width_strata() {
    let doc = SourceDocument::admit(WARRANT_SPEC.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
    check_warrant(&file).unwrap();
    let (binder, model) = validator_model_expression(&file).expect("exact validator claim shape");
    let stratum_counts = Mutex::new([0usize; 9]);
    let outcome_counts = Mutex::new([0usize; 4]);
    let property = |case: &PropertyCase| {
        stratum_counts.lock().unwrap()[case.stratum.index()] += 1;
        match compare_all(&file, binder, model, case.state) {
            Ok(()) => {
                outcome_counts.lock().unwrap()[outcome_index(validate_state(&case.state))] += 1;
                true
            }
            Err(_) => false,
        }
    };
    let options = PbtOptions {
        num_tests: PROPERTY_CASES,
        max_retries: 1,
        max_shrink_steps: MAX_SHRINK_STEPS,
        is_ci: false,
    };
    let verdict = check_property(
        PROPERTY_SEED,
        &options,
        |rng| Ok(gen_case(rng)),
        property,
        |case, property| shrink_case(case, property),
    );
    assert_eq!(verdict, Verdict::Pass, "seeded validator PBT: {verdict:?}");
    let strata = *stratum_counts.lock().unwrap();
    assert_eq!(strata.iter().sum::<usize>(), PROPERTY_CASES);
    for (index, count) in strata.iter().enumerate() {
        assert!(
            *count > 0,
            "input stratum {:?} not reached",
            InputStratum::ALL[index]
        );
    }
    let outcomes = *outcome_counts.lock().unwrap();
    assert_eq!(outcomes.iter().sum::<usize>(), PROPERTY_CASES);
    for (index, count) in outcomes.iter().enumerate() {
        assert!(*count > 0, "outcome stratum {index} not reached");
    }
    println!(
        "validator PBT seed=0x{PROPERTY_SEED:016x} cases={PROPERTY_CASES} strata={strata:?} outcomes={outcomes:?}"
    );
}

#[test]
fn faulty_accumulator_predicate_has_a_smaller_shrink_control() {
    let initial = PropertyCase {
        state: state(0, 0, 10, 1, 10, 10),
        stratum: InputStratum::InvalidAccumulator,
    };
    let bad_property =
        |case: &PropertyCase| deliberately_broken_oracle(case.state) == validate_state(&case.state);
    assert!(
        !bad_property(&initial),
        "fault control must start as a counterexample"
    );
    let (smaller, attempts) = shrink_case(initial, &bad_property);
    assert!(attempts > 0, "shrink control must attempt reductions");
    assert!(
        !bad_property(&smaller),
        "shrunk witness must preserve the fault"
    );
    let score = |s: GameClockState| {
        u128::from(s.tick_count)
            + u128::from(s.elapsed_sim_ns)
            + u128::from(s.accumulator_ns)
            + u128::from(s.fixed_dt_ns)
            + u128::from(s.max_frame_steps)
            + u128::from(s.max_frame_delta_ns)
    };
    assert!(score(smaller.state) < score(initial.state));
    assert_eq!(
        oracle(smaller.state),
        Err(ClockError::CorruptedStateInvariant)
    );
    assert_ne!(
        deliberately_broken_oracle(smaller.state),
        oracle(smaller.state)
    );
    println!(
        "shrink negative control initial={:?} minimal={:?} attempts={attempts}",
        initial.state, smaller.state
    );
}

#[test]
fn ver06_fuzzes_typed_states_across_semantic_strata_through_all_three_paths() {
    let doc = SourceDocument::admit(WARRANT_SPEC.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
    check_warrant(&file).unwrap();
    let (binder, model) = validator_model_expression(&file).expect("exact validator claim shape");

    const SEED: u64 = 0x4743_4c4b_5f54_5950;
    let (max_input_state, _) = typed_state_from_bytes(&[u8::MAX; 48]);
    assert!(compare_all(&file, binder, model, max_input_state).is_ok());
    let mut dt_one_and_max_tick = vec![0u8; 48];
    dt_one_and_max_tick[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut dt_cap = vec![0u8; 48];
    dt_cap[40..48].copy_from_slice(&(FuzzStratum::Valid.index() as u64).to_le_bytes());
    let corpus = vec![vec![0; 48], vec![0xff; 48], dt_one_and_max_tick, dt_cap];
    let options = FuzzOptions {
        max_iterations: 1024,
        seed: SEED,
        max_input_len: 256,
        base_energy: 4,
    };
    let invocations = Cell::new(0usize);
    let strata = Cell::new([0usize; 12]);
    let outcomes = Cell::new([0usize; 4]);
    let precedence = Cell::new([0usize; 3]);
    let report = fuzz_target(&corpus, &options, |bytes, coverage| {
        invocations.set(invocations.get() + 1);
        let (typed, stratum) = typed_state_from_bytes(bytes);
        if let Err(error) = compare_all(&file, binder, model, typed) {
            return FuzzOutcome::Crash {
                kind: "typed-state-disagreement".into(),
                frames: vec![error],
            };
        }
        strata.update(|mut counts| {
            counts[stratum.index()] += 1;
            counts
        });
        let result_index = outcome_index(validate_state(&typed));
        outcomes.update(|mut counts| {
            counts[result_index] += 1;
            counts
        });
        match stratum {
            FuzzStratum::ZeroOutranksCorruption if result_index == 1 => {
                precedence.update(|mut counts| {
                    counts[0] += 1;
                    counts
                });
            }
            FuzzStratum::CorruptionOutranksOverflow if result_index == 2 => {
                precedence.update(|mut counts| {
                    counts[1] += 1;
                    counts
                });
            }
            FuzzStratum::OverflowOutranksMismatch if result_index == 3 => {
                precedence.update(|mut counts| {
                    counts[2] += 1;
                    counts
                });
            }
            _ => {}
        }
        // This is the engine's manual feedback channel, not compiler branch
        // coverage. It records both intended input and observed result class.
        coverage.record_edge(0x730 + stratum.index());
        coverage.record_edge(0x750 + result_index);
        FuzzOutcome::Pass
    });
    assert!(
        report.is_clean(),
        "typed-strata VER06 fuzz report: {report:?}"
    );
    assert_eq!(report.iterations, options.max_iterations);
    assert_eq!(invocations.get(), corpus.len() + options.max_iterations);
    let strata = strata.get();
    let outcomes = outcomes.get();
    assert_eq!(strata.iter().sum::<usize>(), invocations.get());
    assert_eq!(outcomes.iter().sum::<usize>(), invocations.get());
    for (index, count) in strata.iter().enumerate() {
        assert!(*count > 0, "typed fuzz stratum {index} not reached");
    }
    for (index, count) in outcomes.iter().enumerate() {
        assert!(*count > 0, "typed fuzz result class {index} not reached");
    }
    assert!(precedence.get().iter().all(|count| *count > 0));
    println!(
        "typed-strata VER06 seed={SEED:#018x} mutations={} callbacks_including_seeds={} seed_count={} strata={strata:?} outcomes={outcomes:?} precedence={:?} (manual feedback, not native branches) edges={}",
        report.iterations,
        invocations.get(),
        corpus.len(),
        precedence.get(),
        report.total_edges
    );
}

#[test]
fn ver06_malformed_state_transport_is_separate_from_typed_api_fuzz() {
    let doc = SourceDocument::admit(WARRANT_SPEC.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
    check_warrant(&file).unwrap();
    let (binder, model) = validator_model_expression(&file).expect("exact validator claim shape");

    const SEED: u64 = 0x4743_4c4b_5f42_4e44;
    let corpus = vec![
        encode_state_bytes(state(0, 0, 1, 1, 1, 0)),
        encode_state_bytes(state(0, 0, 0, 0, 0, 0)),
        encode_state_bytes(state(0, 0, 2, 0, 2, 0)),
        encode_state_bytes(state(u64::MAX, 0, 2, 1, 2, 0)),
        vec![],
        vec![0; 75],
        vec![0; 77],
    ];
    let options = FuzzOptions {
        max_iterations: 2048,
        seed: SEED,
        max_input_len: 256,
        base_energy: 4,
    };
    let invocations = Cell::new(0usize);
    let accepted_shape = Cell::new(0usize);
    let refused_shape = Cell::new(0usize);
    let outcomes = Cell::new([0usize; 4]);
    let report = fuzz_target(&corpus, &options, |bytes, coverage| {
        invocations.set(invocations.get() + 1);
        let input = match decode_state_bytes(bytes) {
            Ok(input) => {
                accepted_shape.set(accepted_shape.get() + 1);
                input
            }
            Err(_) => {
                refused_shape.set(refused_shape.get() + 1);
                coverage.record_edge(0x740);
                return FuzzOutcome::Pass;
            }
        };
        if let Err(error) = compare_all(&file, binder, model, input) {
            return FuzzOutcome::Crash {
                kind: "decoded-state-disagreement".into(),
                frames: vec![error],
            };
        }
        outcomes.update(|mut counts| {
            counts[outcome_index(validate_state(&input))] += 1;
            counts
        });
        coverage.record_edge(0x750 + outcome_index(validate_state(&input)));
        FuzzOutcome::Pass
    });
    assert!(report.is_clean(), "boundary VER06 fuzz report: {report:?}");
    assert_eq!(report.iterations, options.max_iterations);
    assert_eq!(invocations.get(), corpus.len() + options.max_iterations);
    assert!(
        accepted_shape.get() > 0,
        "76-byte typed records were reached"
    );
    assert!(
        refused_shape.get() > 0,
        "malformed byte lengths were refused"
    );
    assert_eq!(outcomes.get().iter().sum::<usize>(), accepted_shape.get());
    for (index, count) in outcomes.get().iter().enumerate() {
        assert!(
            *count > 0,
            "decoded semantic result stratum {index} not reached"
        );
    }
    println!(
        "boundary VER06 seed={SEED:#018x} mutations={} invocations_including_seeds={} seed_count={} exact_record_calls={} malformed_shape_refusals={} result_outcomes={:?} (manual feedback, not native branches) edges={}",
        report.iterations,
        invocations.get(),
        corpus.len(),
        accepted_shape.get(),
        refused_shape.get(),
        outcomes.get(),
        report.total_edges
    );
}
