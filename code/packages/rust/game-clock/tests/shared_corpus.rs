//! Durable public-API consumer for the language-neutral GAME10 v1 corpus.
//!
//! The fixture's expected values are parsed as data and compared with public API
//! results. No expected value is derived from the implementation under test.

use coding_adventures_bounded_json::{parse, JsonNumber, JsonValue};
use game_clock::{
    advance_frame, new_clock, validate_state, ClockError, ClockStepResult, GameClockConfig,
    GameClockState,
};

const CORPUS: &str = include_str!("../../../../specs/fixtures/game-clock-v1/cases.json");

fn object<'a>(value: &'a JsonValue, context: &str) -> Result<&'a [(String, JsonValue)], String> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(format!("{context}: expected object")),
    }
}

fn field<'a>(value: &'a JsonValue, name: &str) -> Result<&'a JsonValue, String> {
    let fields = object(value, "field lookup")?;
    let mut found = fields.iter().filter(|(key, _)| key == name);
    let value = found
        .next()
        .map(|(_, value)| value)
        .ok_or_else(|| format!("missing field {name}"))?;
    if found.next().is_some() {
        return Err(format!("duplicate field {name}"));
    }
    Ok(value)
}

fn field_mut<'a>(value: &'a mut JsonValue, name: &str) -> Result<&'a mut JsonValue, String> {
    let fields = match value {
        JsonValue::Object(fields) => fields,
        _ => return Err("field lookup: expected object".into()),
    };
    let mut found = fields.iter_mut().filter(|(key, _)| key == name);
    let value = found
        .next()
        .map(|(_, value)| value)
        .ok_or_else(|| format!("missing field {name}"))?;
    if found.next().is_some() {
        return Err(format!("duplicate field {name}"));
    }
    Ok(value)
}

fn string(value: &JsonValue, context: &str) -> Result<String, String> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(format!("{context}: expected string")),
    }
}

fn u64_string(value: &JsonValue, context: &str) -> Result<u64, String> {
    let text = string(value, context)?;
    let parsed = text
        .parse::<u64>()
        .map_err(|_| format!("{context}: invalid u64 decimal"))?;
    if parsed.to_string() != text {
        return Err(format!("{context}: non-canonical u64 decimal"));
    }
    Ok(parsed)
}

fn u32_number(value: &JsonValue, context: &str) -> Result<u32, String> {
    match value {
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => {
            u32::try_from(*value).map_err(|_| format!("{context}: integer exceeds u32"))
        }
        _ => Err(format!("{context}: expected a non-negative JSON integer")),
    }
}

fn read_config(value: &JsonValue) -> Result<GameClockConfig, String> {
    Ok(GameClockConfig {
        fixed_dt_ns: u64_string(field(value, "fixed_dt_ns")?, "fixed_dt_ns")?,
        max_frame_steps: u32_number(field(value, "max_frame_steps")?, "max_frame_steps")?,
        max_frame_delta_ns: u64_string(field(value, "max_frame_delta_ns")?, "max_frame_delta_ns")?,
    })
}

fn read_state(value: &JsonValue) -> Result<GameClockState, String> {
    Ok(GameClockState {
        tick_count: u64_string(field(value, "tick_count")?, "tick_count")?,
        frame_count: u64_string(field(value, "frame_count")?, "frame_count")?,
        elapsed_sim_ns: u64_string(field(value, "elapsed_sim_ns")?, "elapsed_sim_ns")?,
        elapsed_wall_ns: u64_string(field(value, "elapsed_wall_ns")?, "elapsed_wall_ns")?,
        accumulator_ns: u64_string(field(value, "accumulator_ns")?, "accumulator_ns")?,
        fixed_dt_ns: u64_string(field(value, "fixed_dt_ns")?, "fixed_dt_ns")?,
        max_frame_steps: u32_number(field(value, "max_frame_steps")?, "max_frame_steps")?,
        max_frame_delta_ns: u64_string(field(value, "max_frame_delta_ns")?, "max_frame_delta_ns")?,
        total_dropped_ns: u64_string(field(value, "total_dropped_ns")?, "total_dropped_ns")?,
        spiral_clamped_frames: u64_string(
            field(value, "spiral_clamped_frames")?,
            "spiral_clamped_frames",
        )?,
    })
}

fn json_string(value: u64) -> JsonValue {
    JsonValue::String(value.to_string())
}

fn json_u32(value: u32) -> JsonValue {
    JsonValue::Number(JsonNumber::Integer(i64::from(value)))
}

fn config_json(value: GameClockConfig) -> JsonValue {
    JsonValue::Object(vec![
        ("fixed_dt_ns".into(), json_string(value.fixed_dt_ns)),
        ("max_frame_steps".into(), json_u32(value.max_frame_steps)),
        (
            "max_frame_delta_ns".into(),
            json_string(value.max_frame_delta_ns),
        ),
    ])
}

fn state_json(value: GameClockState) -> JsonValue {
    JsonValue::Object(vec![
        ("tick_count".into(), json_string(value.tick_count)),
        ("frame_count".into(), json_string(value.frame_count)),
        ("elapsed_sim_ns".into(), json_string(value.elapsed_sim_ns)),
        ("elapsed_wall_ns".into(), json_string(value.elapsed_wall_ns)),
        ("accumulator_ns".into(), json_string(value.accumulator_ns)),
        ("fixed_dt_ns".into(), json_string(value.fixed_dt_ns)),
        ("max_frame_steps".into(), json_u32(value.max_frame_steps)),
        (
            "max_frame_delta_ns".into(),
            json_string(value.max_frame_delta_ns),
        ),
        (
            "total_dropped_ns".into(),
            json_string(value.total_dropped_ns),
        ),
        (
            "spiral_clamped_frames".into(),
            json_string(value.spiral_clamped_frames),
        ),
    ])
}

fn step_json(value: ClockStepResult) -> JsonValue {
    JsonValue::Object(vec![
        ("state".into(), state_json(value.state)),
        ("steps_to_run".into(), json_u32(value.steps_to_run)),
        ("alpha_milli".into(), json_u32(value.alpha_milli)),
        ("alpha_q16".into(), json_u32(value.alpha_q16)),
        (
            "spiral_of_death_clamped".into(),
            JsonValue::Bool(value.spiral_of_death_clamped),
        ),
        ("dropped_ns".into(), json_string(value.dropped_ns)),
    ])
}

fn error_name(error: ClockError) -> &'static str {
    match error {
        ClockError::ZeroFixedTimestep => "ZeroFixedTimestep",
        ClockError::InvalidMaxFrameSteps => "InvalidMaxFrameSteps",
        ClockError::InvalidMaxFrameDelta => "InvalidMaxFrameDelta",
        ClockError::CorruptedStateInvariant => "CorruptedStateInvariant",
        ClockError::TimestampOverflow => "TimestampOverflow",
    }
}

fn error_json(error: ClockError) -> JsonValue {
    JsonValue::Object(vec![(
        "error".into(),
        JsonValue::String(error_name(error).into()),
    )])
}

fn success_json(value: JsonValue) -> JsonValue {
    JsonValue::Object(vec![("ok".into(), value)])
}

fn semantically_equal(left: &JsonValue, right: &JsonValue) -> bool {
    match (left, right) {
        (JsonValue::Object(left), JsonValue::Object(right)) => {
            if left.len() != right.len() {
                return false;
            }
            let unique = |fields: &[(String, JsonValue)]| {
                let mut names = std::collections::HashSet::new();
                fields.iter().all(|(name, _)| names.insert(name))
            };
            if !unique(left) || !unique(right) {
                return false;
            }
            left.iter().all(|(name, value)| {
                right
                    .iter()
                    .find(|(other_name, _)| other_name == name)
                    .is_some_and(|(_, other_value)| semantically_equal(value, other_value))
            })
        }
        (JsonValue::Array(left), JsonValue::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| semantically_equal(left, right))
        }
        _ => left == right,
    }
}

fn checked_case(
    case: &JsonValue,
    adapter: fn(&JsonValue) -> Result<JsonValue, String>,
) -> Result<(), String> {
    let id = string(field(case, "id")?, "case id")?;
    let expected = field(case, "expected")?;
    let actual = adapter(case).map_err(|error| format!("{id}: adapter failed: {error}"))?;
    if semantically_equal(&actual, expected) {
        Ok(())
    } else {
        Err(format!(
            "{id}: actual {actual:?} differs from expected {expected:?}"
        ))
    }
}

fn public_api_adapter(case: &JsonValue) -> Result<JsonValue, String> {
    let operation = string(field(case, "op")?, "operation")?;
    let input = field(case, "input")?;
    let output = match operation.as_str() {
        "standard_60hz" => success_json(JsonValue::Object(vec![(
            "config".into(),
            config_json(GameClockConfig::standard_60hz()),
        )])),
        "from_hz" => {
            let hz = u32_number(field(input, "hz")?, "hz")?;
            let max_steps = u32_number(field(input, "max_frame_steps")?, "max_frame_steps")?;
            match GameClockConfig::from_hz(hz, max_steps) {
                Ok(config) => success_json(JsonValue::Object(vec![(
                    "config".into(),
                    config_json(config),
                )])),
                Err(error) => error_json(error),
            }
        }
        "new_clock" => match new_clock(read_config(field(input, "config")?)?) {
            Ok(state) => success_json(JsonValue::Object(vec![("state".into(), state_json(state))])),
            Err(error) => error_json(error),
        },
        "validate_state" => {
            let state = read_state(field(input, "state")?)?;
            match validate_state(&state) {
                Ok(()) => success_json(JsonValue::Bool(true)),
                Err(error) => error_json(error),
            }
        }
        "advance_frame" => {
            let initial_state = read_state(field(input, "state")?)?;
            let delta = u64_string(field(input, "raw_frame_delta_ns")?, "raw_frame_delta_ns")?;
            match advance_frame(&initial_state, delta) {
                Ok(result) => {
                    // The public mutating wrapper must be atomic on error and agree on success.
                    let mut mutated = initial_state;
                    let wrapped = mutated.advance(delta).map_err(|error| error_name(error))?;
                    if wrapped != result || mutated != result.state {
                        return Err("mutating wrapper disagrees with advance_frame".into());
                    }
                    success_json(step_json(result))
                }
                Err(error) => {
                    let mut mutated = initial_state;
                    let wrapped_error = mutated.advance(delta).expect_err("same input must fail");
                    if wrapped_error != error || mutated != initial_state {
                        return Err("failed mutating advance changed its state or error".into());
                    }
                    error_json(error)
                }
            }
        }
        unknown => return Err(format!("unknown operation {unknown}")),
    };
    Ok(output)
}

fn all_cases() -> Vec<JsonValue> {
    let root = parse(CORPUS).expect("checked-in GAME10 fixture must be valid JSON");
    assert_eq!(
        u32_number(field(&root, "version").unwrap(), "version").unwrap(),
        1
    );
    assert_eq!(
        string(field(&root, "suite").unwrap(), "suite").unwrap(),
        "game_clock_portable_v1"
    );
    match field(&root, "cases").unwrap() {
        JsonValue::Array(cases) => cases.clone(),
        _ => panic!("cases must be an array"),
    }
}

#[test]
fn every_shared_v1_case_matches_the_public_api() {
    let cases = all_cases();
    let mut ids = std::collections::HashSet::new();
    let mut operations = std::collections::HashSet::new();
    for case in &cases {
        assert!(ids.insert(string(field(case, "id").unwrap(), "id").unwrap()));
        operations.insert(string(field(case, "op").unwrap(), "op").unwrap());
        checked_case(case, public_api_adapter).unwrap();
    }
    assert_eq!(cases.len(), 23, "the v1 corpus must be fully consumed");
    assert_eq!(
        operations,
        [
            "standard_60hz".to_string(),
            "from_hz".to_string(),
            "new_clock".to_string(),
            "validate_state".to_string(),
            "advance_frame".to_string(),
        ]
        .into_iter()
        .collect()
    );
}

fn always_error_adapter(_case: &JsonValue) -> Result<JsonValue, String> {
    Ok(error_json(ClockError::ZeroFixedTimestep))
}

fn reverse_object_members(value: &mut JsonValue) {
    match value {
        JsonValue::Object(fields) => {
            fields.reverse();
            for (_, value) in fields {
                reverse_object_members(value);
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                reverse_object_members(value);
            }
        }
        _ => {}
    }
}

#[test]
fn corpus_checker_rejects_negative_controls() {
    let cases = all_cases();
    let success = cases
        .iter()
        .find(|case| matches!(field(case, "expected").unwrap(), JsonValue::Object(fields) if fields.iter().any(|(name, _)| name == "ok")))
        .unwrap();
    assert!(checked_case(success, always_error_adapter).is_err());

    let mut reordered_expected = success.clone();
    reverse_object_members(field_mut(&mut reordered_expected, "expected").unwrap());
    assert!(checked_case(&reordered_expected, public_api_adapter).is_ok());

    let mut duplicate_expected = success.clone();
    let JsonValue::Object(expected_fields) =
        field_mut(&mut duplicate_expected, "expected").unwrap()
    else {
        panic!("expected object")
    };
    let JsonValue::Object(ok_fields) = &mut expected_fields
        .iter_mut()
        .find(|(name, _)| name == "ok")
        .unwrap()
        .1
    else {
        panic!("ok object")
    };
    let JsonValue::Object(config_fields) = &mut ok_fields
        .iter_mut()
        .find(|(name, _)| name == "config")
        .unwrap()
        .1
    else {
        panic!("config object")
    };
    // Keep the member count fixed but duplicate one name, matching the parsed
    // shape an ambiguous JSON object could otherwise create.
    config_fields[1].0 = "fixed_dt_ns".into();
    assert!(checked_case(&duplicate_expected, public_api_adapter).is_err());

    let mut unknown_operation = success.clone();
    let JsonValue::Object(case_fields) = &mut unknown_operation else {
        panic!("case object")
    };
    case_fields
        .iter_mut()
        .find(|(name, _)| name == "op")
        .unwrap()
        .1 = JsonValue::String("advance_frame_v10".into());
    assert!(checked_case(&unknown_operation, public_api_adapter).is_err());

    let mut changed_success = success.clone();
    let JsonValue::Object(case_fields) = &mut changed_success else {
        panic!("case object")
    };
    let expected = case_fields
        .iter_mut()
        .find(|(name, _)| name == "expected")
        .unwrap();
    let JsonValue::Object(expected_fields) = &mut expected.1 else {
        panic!("expected object")
    };
    let ok = expected_fields
        .iter_mut()
        .find(|(name, _)| name == "ok")
        .unwrap();
    let JsonValue::Object(ok_fields) = &mut ok.1 else {
        panic!("ok object")
    };
    let config = ok_fields
        .iter_mut()
        .find(|(name, _)| name == "config")
        .unwrap();
    let JsonValue::Object(config_fields) = &mut config.1 else {
        panic!("config object")
    };
    config_fields
        .iter_mut()
        .find(|(name, _)| name == "fixed_dt_ns")
        .unwrap()
        .1 = JsonValue::String("1".into());
    assert!(checked_case(&changed_success, public_api_adapter).is_err());

    let mut changed_error = cases
        .iter()
        .find(|case| matches!(field(case, "expected").unwrap(), JsonValue::Object(fields) if fields.iter().any(|(name, _)| name == "error")))
        .unwrap()
        .clone();
    let JsonValue::Object(case_fields) = &mut changed_error else {
        panic!("case object")
    };
    let expected = case_fields
        .iter_mut()
        .find(|(name, _)| name == "expected")
        .unwrap();
    let JsonValue::Object(expected_fields) = &mut expected.1 else {
        panic!("expected object")
    };
    expected_fields
        .iter_mut()
        .find(|(name, _)| name == "error")
        .unwrap()
        .1 = JsonValue::String("TimestampOverflow".into());
    assert!(checked_case(&changed_error, public_api_adapter).is_err());
}
