//! Shared real-parser/checker/evaluator adapter for the validate-state test targets.

use std::collections::HashMap;

use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::parser::parse_document;
use coding_adventures_warrant_grammar::parser::{Declaration, Expression, ParserLimits, TypeExpr};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_grammar::{
    checker::check_warrant,
    layout::{capture_layout, LayoutLimits},
};
use game_clock::{ClockError, GameClockState};

const WARRANT_SPEC: &str = include_str!("../../../../../specs/game-clock.warrant");

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

fn model_expression<'f, 's>(
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
            "expected one validator claim, found {}",
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
        return Err("validator claim is not universal".into());
    };
    if binders.len() != 1
        || binders[0].refinement.is_some()
        || binders[0].inout
        || !matches!(
            binders[0].param_type,
            TypeExpr::Named {
                name: "GameClockState",
                ..
            }
        )
    {
        return Err("validator claim binder is not one bare GameClockState".into());
    }
    let Expression::Binary {
        op: coding_adventures_warrant_grammar::parser::BinaryOp::Eq,
        left,
        right,
        ..
    } = ungroup(body)
    else {
        return Err("validator claim is not direct result equality".into());
    };
    let binder = binders[0].name;
    if !bare_call(left, "validate_state", binder)
        || !bare_call(right, "validate_state_model", binder)
    {
        return Err("validator claim subjects do not use the same bare binder".into());
    }
    Ok((binder, right))
}

fn state_value(state: GameClockState) -> EvalValue {
    EvalValue::Con {
        ty: "GameClockState".into(),
        index: 0,
        ctor: "GameClockState".into(),
        record: true,
        fields: vec![
            EvalValue::U64(state.tick_count),
            EvalValue::U64(state.frame_count),
            EvalValue::U64(state.elapsed_sim_ns),
            EvalValue::U64(state.elapsed_wall_ns),
            EvalValue::U64(state.accumulator_ns),
            EvalValue::U64(state.fixed_dt_ns),
            EvalValue::Fixed(FixedTy::U32, i128::from(state.max_frame_steps)),
            EvalValue::U64(state.max_frame_delta_ns),
            EvalValue::U64(state.total_dropped_ns),
            EvalValue::U64(state.spiral_clamped_frames),
        ],
    }
}

pub fn evaluate_validator(state: GameClockState) -> Result<EvalValue, String> {
    let doc = SourceDocument::admit(WARRANT_SPEC.as_bytes(), SourceLimits::default())
        .map_err(|error| format!("Warrant admission: {error:?}"))?;
    let layout = capture_layout(&doc, LayoutLimits::default())
        .map_err(|error| format!("Warrant layout: {error:?}"))?;
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .map_err(|error| format!("Warrant parse: {error:?}"))?;
    check_warrant(&file).map_err(|error| format!("Warrant typecheck: {error:?}"))?;
    let (binder, model) = model_expression(&file)?;
    let mut environment = HashMap::new();
    environment.insert(binder, state_value(state));
    let mut evaluator = Evaluator::new(&file, CodegenLimits::default());
    evaluator.reset_work();
    evaluator
        .eval(model, &environment)
        .map_err(|error| format!("Warrant evaluator: {error:?}"))
}

pub fn expected_value(result: Result<(), ClockError>) -> EvalValue {
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
