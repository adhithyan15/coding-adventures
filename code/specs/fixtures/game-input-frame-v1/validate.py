"""Validate GAME input's independently authored, language-neutral v1 vectors."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

FIXTURE = Path(__file__).resolve().parent / "cases.json"
MAX_U64 = (1 << 64) - 1
MAX_U32 = (1 << 32) - 1
MAX_BUTTONS = 64
MAX_AXES = 16
MAX_CHANGES = 256
AXIS_MIN = -32767
AXIS_MAX = 32767
CASE_IDS = {
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
}


class FixtureError(ValueError):
    """A malformed fixture or an expectation that disagrees with the model."""


def exact_keys(value: Any, expected: set[str], label: str) -> dict[str, Any]:
    if type(value) is not dict or set(value) != expected:
        raise FixtureError(f"{label} must have exactly {sorted(expected)}")
    return value


def integer(value: Any, low: int, high: int, label: str) -> int:
    if type(value) is not int or not low <= value <= high:
        raise FixtureError(f"{label} must be an integer in {low}..={high}")
    return value


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Keep JSON member names unique instead of silently taking the last one."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise FixtureError(f"duplicate object key {key!r}")
        result[key] = value
    return result


def u64_string(value: Any, label: str) -> int:
    if type(value) is not str or re.fullmatch(r"0|[1-9][0-9]*", value, re.ASCII) is None:
        raise FixtureError(f"{label} must be a canonical ASCII decimal u64 string")
    parsed = int(value)
    if parsed > MAX_U64:
        raise FixtureError(f"{label} exceeds u64")
    return parsed


def parse_case(raw: Any) -> dict[str, Any]:
    case = exact_keys(raw, {"id", "input", "expected"}, "case")
    if type(case["id"]) is not str or not case["id"]:
        raise FixtureError("case.id must be a nonempty string")
    source = exact_keys(
        case["input"], {"player_id", "tick", "state", "changes"}, f"{case['id']}.input"
    )
    player = integer(source["player_id"], 0, MAX_U32, f"{case['id']}.player_id")
    tick = u64_string(source["tick"], f"{case['id']}.tick")
    state = exact_keys(source["state"], {"held_buttons", "axes"}, f"{case['id']}.state")
    held = u64_string(state["held_buttons"], f"{case['id']}.held_buttons")
    if type(state["axes"]) is not list or len(state["axes"]) != MAX_AXES:
        raise FixtureError(f"{case['id']}.state.axes must contain exactly 16 values")
    axes = [integer(value, -32768, 32767, f"{case['id']}.state.axes[{i}]") for i, value in enumerate(state["axes"])]
    if type(source["changes"]) is not list:
        raise FixtureError(f"{case['id']}.changes must be an array")
    changes: list[dict[str, Any]] = []
    for index, change in enumerate(source["changes"]):
        if type(change) is not dict or change.get("kind") not in {"button", "axis"}:
            raise FixtureError(f"{case['id']}.changes[{index}] has an unknown kind")
        if change["kind"] == "button":
            change = exact_keys(change, {"kind", "slot", "down"}, f"{case['id']}.changes[{index}]")
            slot = integer(change["slot"], 0, 255, f"{case['id']}.changes[{index}].slot")
            if type(change["down"]) is not bool:
                raise FixtureError(f"{case['id']}.changes[{index}].down must be a boolean")
            changes.append({"kind": "button", "slot": slot, "down": change["down"]})
        else:
            change = exact_keys(change, {"kind", "slot", "value"}, f"{case['id']}.changes[{index}]")
            slot = integer(change["slot"], 0, 255, f"{case['id']}.changes[{index}].slot")
            value = integer(change["value"], -32768, 32767, f"{case['id']}.changes[{index}].value")
            changes.append({"kind": "axis", "slot": slot, "value": value})
    return {"id": case["id"], "player": player, "tick": tick, "held": held, "axes": axes, "changes": changes, "expected": case["expected"]}


def model(case: dict[str, Any]) -> dict[str, Any]:
    """Independent per-action level/count reference, not the production reducer."""
    if len(case["changes"]) > MAX_CHANGES:
        return {"error": {"code": "TooManyChanges"}}
    axes = case["axes"][:]
    for slot, value in enumerate(axes):
        if not AXIS_MIN <= value <= AXIS_MAX:
            return {"error": {"code": "InvalidPreviousAxisValue", "axis_slot": slot, "value": value}}

    # This reference stores one level and two transition-presence counters for
    # each action. It does not use the implementation's bit-mask update path.
    held_levels = [bool(case["held"] & (1 << slot)) for slot in range(MAX_BUTTONS)]
    rise_counts = [0] * MAX_BUTTONS
    fall_counts = [0] * MAX_BUTTONS
    for index, change in enumerate(case["changes"]):
        slot = change["slot"]
        if change["kind"] == "button":
            if slot >= MAX_BUTTONS:
                return {"error": {"code": "InvalidButtonActionId", "change_index": index, "action_id": slot}}
            was_down = held_levels[slot]
            now_down = change["down"]
            if now_down and not was_down:
                rise_counts[slot] += 1
            elif was_down and not now_down:
                fall_counts[slot] += 1
            held_levels[slot] = now_down
        else:
            if slot >= MAX_AXES:
                return {"error": {"code": "InvalidAxisActionId", "change_index": index, "action_id": slot}}
            value = change["value"]
            if not AXIS_MIN <= value <= AXIS_MAX:
                return {"error": {"code": "InvalidAxisValue", "change_index": index, "axis_slot": slot, "value": value}}
            axes[slot] = value
    held = sum(1 << slot for slot, down in enumerate(held_levels) if down)
    pressed = sum(1 << slot for slot, count in enumerate(rise_counts) if count > 0)
    released = sum(1 << slot for slot, count in enumerate(fall_counts) if count > 0)
    return {
        "frame": {
            "player_id": case["player"],
            "tick": str(case["tick"]),
            "held_buttons": str(held),
            "pressed_buttons": str(pressed),
            "released_buttons": str(released),
            "axes": axes,
        }
    }


def validate_expected(value: Any, case_id: str) -> dict[str, Any]:
    """Type-check every expected field before Python's permissive equality."""
    if type(value) is not dict:
        raise FixtureError(f"{case_id}.expected must be an object")
    if set(value) == {"frame"}:
        frame = exact_keys(
            value["frame"],
            {"player_id", "tick", "held_buttons", "pressed_buttons", "released_buttons", "axes"},
            f"{case_id}.expected.frame",
        )
        integer(frame["player_id"], 0, MAX_U32, f"{case_id}.expected.frame.player_id")
        u64_string(frame["tick"], f"{case_id}.expected.frame.tick")
        for name in ("held_buttons", "pressed_buttons", "released_buttons"):
            u64_string(frame[name], f"{case_id}.expected.frame.{name}")
        if type(frame["axes"]) is not list or len(frame["axes"]) != MAX_AXES:
            raise FixtureError(f"{case_id}.expected.frame.axes must contain exactly 16 values")
        for index, axis in enumerate(frame["axes"]):
            integer(axis, AXIS_MIN, AXIS_MAX, f"{case_id}.expected.frame.axes[{index}]")
        return {"frame": frame}

    if set(value) != {"error"} or type(value["error"]) is not dict:
        raise FixtureError(f"{case_id}.expected must contain exactly frame or error")
    error = value["error"]
    if type(error.get("code")) is not str:
        raise FixtureError(f"{case_id}.expected.error.code must be a string")
    code = error["code"]
    shapes = {
        "TooManyChanges": {"code"},
        "InvalidPreviousAxisValue": {"code", "axis_slot", "value"},
        "InvalidButtonActionId": {"code", "change_index", "action_id"},
        "InvalidAxisActionId": {"code", "change_index", "action_id"},
        "InvalidAxisValue": {"code", "change_index", "axis_slot", "value"},
    }
    if code not in shapes or set(error) != shapes[code]:
        raise FixtureError(f"{case_id}.expected.error has an unknown code or wrong fields")
    if code == "InvalidPreviousAxisValue":
        integer(error["axis_slot"], 0, MAX_AXES - 1, f"{case_id}.expected.error.axis_slot")
        integer(error["value"], -32768, 32767, f"{case_id}.expected.error.value")
    elif code in {"InvalidButtonActionId", "InvalidAxisActionId"}:
        integer(error["change_index"], 0, 255, f"{case_id}.expected.error.change_index")
        minimum = MAX_BUTTONS if code == "InvalidButtonActionId" else MAX_AXES
        integer(error["action_id"], minimum, 255, f"{case_id}.expected.error.action_id")
    elif code == "InvalidAxisValue":
        integer(error["change_index"], 0, 255, f"{case_id}.expected.error.change_index")
        integer(error["axis_slot"], 0, MAX_AXES - 1, f"{case_id}.expected.error.axis_slot")
        integer(error["value"], -32768, 32767, f"{case_id}.expected.error.value")
    return {"error": error}


def validate_bytes(raw_bytes: bytes, expected_sha256: str, *, announce: bool = True) -> None:
    """Validate one exact byte payload; tests pass its own digest explicitly."""
    actual_digest = hashlib.sha256(raw_bytes).hexdigest()
    if actual_digest != expected_sha256:
        raise FixtureError(f"cases.json SHA-256 mismatch: {actual_digest}")
    document = json.loads(raw_bytes, object_pairs_hook=unique_object)
    exact_keys(document, {"version", "suite", "cases"}, "document")
    if type(document["version"]) is not int or document["version"] != 1 or document["suite"] != "game-input-frame-v1":
        raise FixtureError("unsupported fixture version or suite")
    if type(document["cases"]) is not list:
        raise FixtureError("cases must be an array")
    parsed = [parse_case(case) for case in document["cases"]]
    ids = [case["id"] for case in parsed]
    if len(ids) != len(set(ids)) or set(ids) != CASE_IDS:
        raise FixtureError(f"case ID set mismatch: missing={sorted(CASE_IDS - set(ids))}, extra={sorted(set(ids) - CASE_IDS)}")
    for case, original in zip(parsed, document["cases"], strict=True):
        expected = validate_expected(original["expected"], case["id"])
        actual = model(case)
        if actual != expected:
            raise FixtureError(f"{case['id']} expected {expected!r}, independent model gives {actual!r}")
    if announce:
        print(f"game-input-frame-v1: {len(parsed)} exact cases; sha256={actual_digest}")


def validate() -> None:
    # Read and pin the exact canonical bytes before any JSON decoding.
    validate_bytes(FIXTURE.read_bytes(), EXPECTED_SHA256)


EXPECTED_SHA256 = "8db2c0fdc9bb6ab93785f3bd6d5dc52183b67682ca0e8d693fcdbfaa30aff785"


if __name__ == "__main__":
    try:
        validate()
    except (FixtureError, json.JSONDecodeError, OSError) as error:
        print(f"game-input-frame-v1: {error}", file=sys.stderr)
        raise SystemExit(1) from error
