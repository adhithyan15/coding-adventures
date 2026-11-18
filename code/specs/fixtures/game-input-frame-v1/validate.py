"""Validate GAME11's independently authored, language-neutral v1 vectors."""

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
    """Independent, small-array reference for the public reducer contract."""
    if len(case["changes"]) > MAX_CHANGES:
        return {"error": {"code": "TooManyChanges"}}
    axes = case["axes"][:]
    for slot, value in enumerate(axes):
        if not AXIS_MIN <= value <= AXIS_MAX:
            return {"error": {"code": "InvalidPreviousAxisValue", "axis_slot": slot, "value": value}}

    held = case["held"]
    pressed = 0
    released = 0
    for index, change in enumerate(case["changes"]):
        slot = change["slot"]
        if change["kind"] == "button":
            if slot >= MAX_BUTTONS:
                return {"error": {"code": "InvalidButtonActionId", "change_index": index, "action_id": slot}}
            bit = 1 << slot
            was_down = bool(held & bit)
            if change["down"] and not was_down:
                pressed |= bit
            elif not change["down"] and was_down:
                released |= bit
            if change["down"]:
                held |= bit
            else:
                held &= ~bit
        else:
            if slot >= MAX_AXES:
                return {"error": {"code": "InvalidAxisActionId", "change_index": index, "action_id": slot}}
            value = change["value"]
            if not AXIS_MIN <= value <= AXIS_MAX:
                return {"error": {"code": "InvalidAxisValue", "change_index": index, "axis_slot": slot, "value": value}}
            axes[slot] = value
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


def validate() -> None:
    # Read and pin the exact bytes before any JSON decoding or normalization.
    raw_bytes = FIXTURE.read_bytes()
    actual_digest = hashlib.sha256(raw_bytes).hexdigest()
    if actual_digest != EXPECTED_SHA256:
        raise FixtureError(f"cases.json SHA-256 mismatch: {actual_digest}")
    document = json.loads(raw_bytes)
    exact_keys(document, {"version", "suite", "cases"}, "document")
    if document["version"] != 1 or document["suite"] != "game-input-frame-v1":
        raise FixtureError("unsupported fixture version or suite")
    if type(document["cases"]) is not list:
        raise FixtureError("cases must be an array")
    parsed = [parse_case(case) for case in document["cases"]]
    ids = [case["id"] for case in parsed]
    if len(ids) != len(set(ids)) or set(ids) != CASE_IDS:
        raise FixtureError(f"case ID set mismatch: missing={sorted(CASE_IDS - set(ids))}, extra={sorted(set(ids) - CASE_IDS)}")
    for case, original in zip(parsed, document["cases"], strict=True):
        raw_expected = original["expected"]
        if type(raw_expected) is not dict:
            raise FixtureError(f"{case['id']}.expected must be an object")
        expected = exact_keys(raw_expected, {"frame"} if "frame" in raw_expected else {"error"}, f"{case['id']}.expected")
        actual = model(case)
        if actual != expected:
            raise FixtureError(f"{case['id']} expected {expected!r}, independent model gives {actual!r}")
    print(f"game-input-frame-v1: {len(parsed)} exact cases; sha256={actual_digest}")


EXPECTED_SHA256 = "8db2c0fdc9bb6ab93785f3bd6d5dc52183b67682ca0e8d693fcdbfaa30aff785"


if __name__ == "__main__":
    try:
        validate()
    except (FixtureError, json.JSONDecodeError, OSError) as error:
        print(f"game-input-frame-v1: {error}", file=sys.stderr)
        raise SystemExit(1) from error
