//! Public-API consumer for the independently authored GAME input frame corpus.
//!
//! The vectors carry expected outputs as data. This adapter decodes those
//! expectations, runs the public reducer, and compares every public field and
//! every error field. A fixed byte digest and exact case-ID set keep a dropped
//! or substituted vector from looking like a successful partial run.

use coding_adventures_bounded_json::{parse, JsonNumber, JsonValue};
use coding_adventures_sha256::sha256_hex;
use game_input_frame::{
    reduce_frame, ActionChange, ActionFrame, AxisActionId, ButtonActionId, FrameError, InputState,
    PlayerId,
};

const CORPUS_BYTES: &[u8] =
    include_bytes!("../../../../specs/fixtures/game-input-frame-v1/cases.json");
const CORPUS: &str = include_str!("../../../../specs/fixtures/game-input-frame-v1/cases.json");
const CORPUS_SHA256: &str = "8db2c0fdc9bb6ab93785f3bd6d5dc52183b67682ca0e8d693fcdbfaa30aff785";
const EXPECTED_CASE_IDS: [&str; 21] = [
    "empty_frame_preserves_levels_and_clears_edges",
    "press_from_up",
    "release_from_down",
    "duplicate_down_is_noop",
    "duplicate_up_is_noop",
    "same_tick_tap_sets_both_edges",
    "repeated_taps_are_presence_masks",
    "down_up_down_finishes_held",
    "axis_update_and_persistence",
    "axis_last_change_wins",
    "button_slot_63_and_full_width_tick",
    "u32_player_max_and_axis_lower_bound",
    "too_many_precedes_invalid_state_and_event",
    "lowest_invalid_prior_axis_wins",
    "invalid_button_slot",
    "invalid_axis_slot_precedes_axis_value",
    "invalid_axis_value",
    "first_invalid_change_wins",
    "accept_255_changes",
    "accept_256_changes",
    "reject_257_changes",
];

#[derive(Debug)]
struct Case {
    id: String,
    previous: InputState,
    tick: u64,
    changes: Vec<ActionChange>,
    expected: Result<ActionFrame, FrameError>,
}

fn object<'a>(value: &'a JsonValue, context: &str) -> Result<&'a [(String, JsonValue)], String> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(format!("{context}: expected object")),
    }
}

fn field<'a>(value: &'a JsonValue, name: &str, context: &str) -> Result<&'a JsonValue, String> {
    let fields = object(value, context)?;
    let mut found = fields.iter().filter(|(key, _)| key == name);
    let result = found
        .next()
        .map(|(_, value)| value)
        .ok_or_else(|| format!("{context}: missing field {name}"))?;
    if found.next().is_some() {
        return Err(format!("{context}: duplicate field {name}"));
    }
    Ok(result)
}

fn exact_fields(value: &JsonValue, expected: &[&str], context: &str) -> Result<(), String> {
    let fields = object(value, context)?;
    let names: Vec<_> = fields.iter().map(|(name, _)| name.as_str()).collect();
    if names.len() != expected.len()
        || names.iter().any(|name| !expected.contains(name))
        || expected
            .iter()
            .any(|name| names.iter().filter(|got| *got == name).count() != 1)
    {
        return Err(format!(
            "{context}: fields {names:?}, expected {expected:?}"
        ));
    }
    Ok(())
}

fn string(value: &JsonValue, context: &str) -> Result<String, String> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(format!("{context}: expected string")),
    }
}

fn integer(value: &JsonValue, low: i64, high: i64, context: &str) -> Result<i64, String> {
    match value {
        JsonValue::Number(JsonNumber::Integer(value)) if (low..=high).contains(value) => Ok(*value),
        _ => Err(format!(
            "{context}: expected exact integer in {low}..={high}"
        )),
    }
}

fn u64_string(value: &JsonValue, context: &str) -> Result<u64, String> {
    let raw = string(value, context)?;
    let parsed = raw
        .parse::<u64>()
        .map_err(|_| format!("{context}: invalid u64 decimal"))?;
    if parsed.to_string() != raw {
        return Err(format!("{context}: non-canonical u64 decimal"));
    }
    Ok(parsed)
}

fn axes(value: &JsonValue, context: &str) -> Result<[i16; 16], String> {
    let entries = match value {
        JsonValue::Array(entries) if entries.len() == 16 => entries,
        _ => return Err(format!("{context}: expected exactly 16 axes")),
    };
    let mut result = [0_i16; 16];
    for (slot, raw) in entries.iter().enumerate() {
        result[slot] = integer(raw, i16::MIN.into(), i16::MAX.into(), context)? as i16;
    }
    Ok(result)
}

fn read_case(value: &JsonValue) -> Result<Case, String> {
    exact_fields(value, &["id", "input", "expected"], "case")?;
    let id = string(field(value, "id", "case")?, "case.id")?;
    let input = field(value, "input", &id)?;
    exact_fields(
        input,
        &["player_id", "tick", "state", "changes"],
        &format!("{id}.input"),
    )?;
    let player_id = integer(
        field(input, "player_id", &id)?,
        0,
        u32::MAX.into(),
        &format!("{id}.player_id"),
    )? as u32;
    let tick = u64_string(field(input, "tick", &id)?, &format!("{id}.tick"))?;
    let state = field(input, "state", &id)?;
    exact_fields(state, &["held_buttons", "axes"], &format!("{id}.state"))?;
    let held_buttons = u64_string(
        field(state, "held_buttons", &id)?,
        &format!("{id}.state.held_buttons"),
    )?;
    let prior_axes = axes(field(state, "axes", &id)?, &format!("{id}.state.axes"))?;
    let previous = InputState {
        player_id: PlayerId::new(player_id),
        held_buttons,
        axes: prior_axes,
    };
    let raw_changes = match field(input, "changes", &id)? {
        JsonValue::Array(values) => values,
        _ => return Err(format!("{id}.changes: expected array")),
    };
    let mut changes = Vec::with_capacity(raw_changes.len());
    for (index, change) in raw_changes.iter().enumerate() {
        let kind = string(
            field(change, "kind", &format!("{id}.changes[{index}]"))?,
            "change.kind",
        )?;
        match kind.as_str() {
            "button" => {
                exact_fields(
                    change,
                    &["kind", "slot", "down"],
                    &format!("{id}.changes[{index}]"),
                )?;
                let slot = integer(
                    field(change, "slot", &id)?,
                    0,
                    u8::MAX.into(),
                    "button slot",
                )? as u8;
                let down = match field(change, "down", &id)? {
                    JsonValue::Bool(value) => *value,
                    _ => return Err(format!("{id}.changes[{index}].down: expected boolean")),
                };
                changes.push(ActionChange::Button {
                    action_id: ButtonActionId::new(slot),
                    down,
                });
            }
            "axis" => {
                exact_fields(
                    change,
                    &["kind", "slot", "value"],
                    &format!("{id}.changes[{index}]"),
                )?;
                let slot =
                    integer(field(change, "slot", &id)?, 0, u8::MAX.into(), "axis slot")? as u8;
                let value = integer(
                    field(change, "value", &id)?,
                    i16::MIN.into(),
                    i16::MAX.into(),
                    "axis value",
                )? as i16;
                changes.push(ActionChange::Axis {
                    action_id: AxisActionId::new(slot),
                    value,
                });
            }
            other => return Err(format!("{id}.changes[{index}]: unknown kind {other}")),
        }
    }
    let expected = read_expected(field(value, "expected", &id)?, &id)?;
    Ok(Case {
        id,
        previous,
        tick,
        changes,
        expected,
    })
}

fn read_expected(value: &JsonValue, id: &str) -> Result<Result<ActionFrame, FrameError>, String> {
    exact_fields(value, &["frame"], &format!("{id}.expected"))
        .or_else(|_| exact_fields(value, &["error"], &format!("{id}.expected")))?;
    if let Ok(frame) = field(value, "frame", &format!("{id}.expected")) {
        exact_fields(
            frame,
            &[
                "player_id",
                "tick",
                "held_buttons",
                "pressed_buttons",
                "released_buttons",
                "axes",
            ],
            &format!("{id}.expected.frame"),
        )?;
        let player = integer(
            field(frame, "player_id", id)?,
            0,
            u32::MAX.into(),
            "expected player",
        )? as u32;
        let tick = u64_string(field(frame, "tick", id)?, "expected tick")?;
        let held = u64_string(field(frame, "held_buttons", id)?, "expected held")?;
        let pressed = u64_string(field(frame, "pressed_buttons", id)?, "expected pressed")?;
        let released = u64_string(field(frame, "released_buttons", id)?, "expected released")?;
        let axes = axes(field(frame, "axes", id)?, "expected axes")?;
        if axes.contains(&i16::MIN) {
            return Err(format!(
                "{id}.expected.frame.axes: i16::MIN is outside the v1 axis domain"
            ));
        }
        return Ok(Ok(ActionFrame {
            player_id: PlayerId::new(player),
            tick,
            held_buttons: held,
            pressed_buttons: pressed,
            released_buttons: released,
            axes,
        }));
    }
    let error = field(value, "error", &format!("{id}.expected"))?;
    let code = string(field(error, "code", id)?, "expected error code")?;
    let parsed = match code.as_str() {
        "TooManyChanges" => {
            exact_fields(error, &["code"], "expected TooManyChanges")?;
            FrameError::TooManyChanges
        }
        "InvalidPreviousAxisValue" => {
            exact_fields(
                error,
                &["code", "axis_slot", "value"],
                "expected prior-axis error",
            )?;
            FrameError::InvalidPreviousAxisValue {
                axis_slot: integer(field(error, "axis_slot", id)?, 0, 15, "expected axis slot")?
                    as u8,
                value: integer(
                    field(error, "value", id)?,
                    i16::MIN.into(),
                    i16::MAX.into(),
                    "expected axis value",
                )? as i16,
            }
        }
        "InvalidButtonActionId" => {
            exact_fields(
                error,
                &["code", "change_index", "action_id"],
                "expected button-id error",
            )?;
            FrameError::InvalidButtonActionId {
                change_index: integer(
                    field(error, "change_index", id)?,
                    0,
                    u8::MAX.into(),
                    "expected change index",
                )? as u8,
                action_id: integer(
                    field(error, "action_id", id)?,
                    64,
                    u8::MAX.into(),
                    "expected button id",
                )? as u8,
            }
        }
        "InvalidAxisActionId" => {
            exact_fields(
                error,
                &["code", "change_index", "action_id"],
                "expected axis-id error",
            )?;
            FrameError::InvalidAxisActionId {
                change_index: integer(
                    field(error, "change_index", id)?,
                    0,
                    u8::MAX.into(),
                    "expected change index",
                )? as u8,
                action_id: integer(
                    field(error, "action_id", id)?,
                    16,
                    u8::MAX.into(),
                    "expected axis id",
                )? as u8,
            }
        }
        "InvalidAxisValue" => {
            exact_fields(
                error,
                &["code", "change_index", "axis_slot", "value"],
                "expected axis-value error",
            )?;
            FrameError::InvalidAxisValue {
                change_index: integer(
                    field(error, "change_index", id)?,
                    0,
                    u8::MAX.into(),
                    "expected change index",
                )? as u8,
                axis_slot: integer(field(error, "axis_slot", id)?, 0, 15, "expected axis slot")?
                    as u8,
                value: integer(
                    field(error, "value", id)?,
                    i16::MIN.into(),
                    i16::MAX.into(),
                    "expected axis value",
                )? as i16,
            }
        }
        _ => return Err(format!("{id}.expected.error: unknown code {code}")),
    };
    Ok(Err(parsed))
}

fn cases() -> Vec<Case> {
    assert_eq!(
        sha256_hex(CORPUS_BYTES),
        CORPUS_SHA256,
        "canonical bytes changed"
    );
    let parsed = parse(CORPUS).expect("canonical fixture parses as bounded JSON");
    exact_fields(&parsed, &["version", "suite", "cases"], "corpus").unwrap();
    assert_eq!(
        integer(
            field(&parsed, "version", "corpus").unwrap(),
            1,
            1,
            "version"
        )
        .unwrap(),
        1
    );
    assert_eq!(
        string(field(&parsed, "suite", "corpus").unwrap(), "suite").unwrap(),
        "game-input-frame-v1"
    );
    let rows = match field(&parsed, "cases", "corpus").unwrap() {
        JsonValue::Array(rows) => rows,
        _ => panic!("cases must be an array"),
    };
    let cases: Vec<_> = rows.iter().map(|row| read_case(row).unwrap()).collect();
    let mut ids: Vec<_> = cases.iter().map(|case| case.id.as_str()).collect();
    ids.sort_unstable();
    let mut expected = EXPECTED_CASE_IDS.to_vec();
    expected.sort_unstable();
    assert_eq!(ids, expected, "every exact case ID must appear once");
    cases
}

fn matches_with(
    case: &Case,
    reducer: impl Fn(&InputState, u64, &[ActionChange]) -> Result<ActionFrame, FrameError>,
) -> bool {
    reducer(&case.previous, case.tick, &case.changes) == case.expected
}

#[test]
fn every_independent_case_matches_the_public_reducer() {
    for case in cases() {
        assert!(
            matches_with(&case, reduce_frame),
            "case {} diverged",
            case.id
        );
    }
}

#[test]
fn same_case_comparison_rejects_three_named_semantic_mutants() {
    let all = cases();
    let tap = all
        .iter()
        .find(|case| case.id == "same_tick_tap_sets_both_edges")
        .unwrap();
    let high_bit = all
        .iter()
        .find(|case| case.id == "button_slot_63_and_full_width_tick")
        .unwrap();

    // A reducer that refuses every input fails on this successful canonical case.
    assert!(!matches_with(tap, |_, _, _| Err(
        FrameError::TooManyChanges
    )));

    // This endpoint-only mutant deliberately forgets the down/up pulse.
    assert!(!matches_with(tap, |previous, tick, changes| {
        let mut frame = reduce_frame(previous, tick, changes)?;
        frame.pressed_buttons = frame.held_buttons & !previous.held_buttons;
        frame.released_buttons = previous.held_buttons & !frame.held_buttons;
        Ok(frame)
    }));

    // This mask-width mutant drops the most significant public action slot.
    assert!(!matches_with(high_bit, |previous, tick, changes| {
        let mut frame = reduce_frame(previous, tick, changes)?;
        frame.held_buttons &= u64::MAX >> 1;
        frame.pressed_buttons &= u64::MAX >> 1;
        frame.released_buttons &= u64::MAX >> 1;
        Ok(frame)
    }));
}
