"""Validate the language-neutral GAME10 fixed-step clock vectors."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any

MAX_U64 = (1 << 64) - 1
MAX_U32 = (1 << 32) - 1
HARD_MAX_STEPS = 1024
HARD_MAX_DELTA_NS = 10_000_000_000
DEFAULT_MAX_DELTA_NS = 250_000_000
ERRORS = {
    "ZeroFixedTimestep",
    "InvalidMaxFrameSteps",
    "InvalidMaxFrameDelta",
    "CorruptedStateInvariant",
    "TimestampOverflow",
}
CONFIG_KEYS = {"fixed_dt_ns", "max_frame_steps", "max_frame_delta_ns"}
STATE_KEYS = {
    "tick_count",
    "frame_count",
    "elapsed_sim_ns",
    "elapsed_wall_ns",
    "accumulator_ns",
    "fixed_dt_ns",
    "max_frame_steps",
    "max_frame_delta_ns",
    "total_dropped_ns",
    "spiral_clamped_frames",
}


class ClockFailure(Exception):
    def __init__(self, code: str):
        if code not in ERRORS:
            raise AssertionError(f"unknown model error {code}")
        self.code = code


def fail(code: str) -> None:
    raise ClockFailure(code)


def keys(obj: Any, expected: set[str], label: str) -> dict[str, Any]:
    if type(obj) is not dict or set(obj) != expected:
        raise ValueError(f"{label} must contain exactly {sorted(expected)}")
    return obj


def u64(value: Any, label: str) -> int:
    if (
        type(value) is not str
        or re.fullmatch(r"0|[1-9][0-9]*", value, re.ASCII) is None
    ):
        raise ValueError(f"{label} must be a canonical ASCII u64 string")
    number = int(value)
    if number > MAX_U64:
        raise ValueError(f"{label} exceeds u64")
    return number


def u32(value: Any, label: str) -> int:
    if type(value) is not int or value < 0 or value > MAX_U32:
        raise ValueError(f"{label} must be a JSON u32 integer")
    return value


def config_in(value: Any, label: str = "config") -> dict[str, int]:
    cfg = keys(value, CONFIG_KEYS, label)
    return {
        "fixed_dt_ns": u64(cfg["fixed_dt_ns"], f"{label}.fixed_dt_ns"),
        "max_frame_steps": u32(cfg["max_frame_steps"], f"{label}.max_frame_steps"),
        "max_frame_delta_ns": u64(
            cfg["max_frame_delta_ns"], f"{label}.max_frame_delta_ns"
        ),
    }


def state_in(value: Any, label: str = "state") -> dict[str, int]:
    raw = keys(value, STATE_KEYS, label)
    out = {
        key: u64(raw[key], f"{label}.{key}") for key in STATE_KEYS - {"max_frame_steps"}
    }
    out["max_frame_steps"] = u32(raw["max_frame_steps"], f"{label}.max_frame_steps")
    return out


def config_out(cfg: dict[str, int]) -> dict[str, Any]:
    return {
        "fixed_dt_ns": str(cfg["fixed_dt_ns"]),
        "max_frame_steps": cfg["max_frame_steps"],
        "max_frame_delta_ns": str(cfg["max_frame_delta_ns"]),
    }


def state_out(state: dict[str, int]) -> dict[str, Any]:
    return {
        key: (state[key] if key == "max_frame_steps" else str(state[key]))
        for key in STATE_KEYS
    }


def valid_config(cfg: dict[str, int]) -> None:
    dt = cfg["fixed_dt_ns"]
    if dt == 0:
        fail("ZeroFixedTimestep")
    if dt > HARD_MAX_DELTA_NS:
        fail("InvalidMaxFrameDelta")
    steps = cfg["max_frame_steps"]
    if steps == 0 or steps > HARD_MAX_STEPS:
        fail("InvalidMaxFrameSteps")
    delta = cfg["max_frame_delta_ns"]
    if delta < dt or delta > HARD_MAX_DELTA_NS:
        fail("InvalidMaxFrameDelta")


def make_clock(cfg: dict[str, int]) -> dict[str, int]:
    valid_config(cfg)
    return {
        "tick_count": 0,
        "frame_count": 0,
        "elapsed_sim_ns": 0,
        "elapsed_wall_ns": 0,
        "accumulator_ns": 0,
        "fixed_dt_ns": cfg["fixed_dt_ns"],
        "max_frame_steps": cfg["max_frame_steps"],
        "max_frame_delta_ns": cfg["max_frame_delta_ns"],
        "total_dropped_ns": 0,
        "spiral_clamped_frames": 0,
    }


def validate_state(state: dict[str, int]) -> None:
    dt = state["fixed_dt_ns"]
    if dt == 0:
        fail("ZeroFixedTimestep")
    if dt > HARD_MAX_DELTA_NS:
        fail("CorruptedStateInvariant")
    steps = state["max_frame_steps"]
    if steps == 0 or steps > HARD_MAX_STEPS:
        fail("CorruptedStateInvariant")
    delta = state["max_frame_delta_ns"]
    if delta < dt or delta > HARD_MAX_DELTA_NS:
        fail("CorruptedStateInvariant")
    if state["accumulator_ns"] >= dt:
        fail("CorruptedStateInvariant")
    sim = state["tick_count"] * dt
    if sim > MAX_U64:
        fail("TimestampOverflow")
    if state["elapsed_sim_ns"] != sim:
        fail("CorruptedStateInvariant")


def checked_add(left: int, right: int) -> int:
    value = left + right
    if value > MAX_U64:
        fail("TimestampOverflow")
    return value


def advance(state: dict[str, int], raw_delta: int) -> dict[str, Any]:
    validate_state(state)
    dt = state["fixed_dt_ns"]
    clamped = min(raw_delta, state["max_frame_delta_ns"])
    wall_drop = raw_delta - clamped
    remaining = state["accumulator_ns"] + clamped
    if remaining > MAX_U64:
        fail("TimestampOverflow")

    # Execute bounded ticks by subtraction. Drain remaining whole ticks afterward.
    steps = 0
    while steps < state["max_frame_steps"] and remaining >= dt:
        remaining -= dt
        steps += 1
    spiral = remaining >= dt
    spiral_drop = 0
    if spiral:
        excess_steps = remaining // dt
        spiral_drop = excess_steps * dt
        remaining -= spiral_drop

    tick_count = checked_add(state["tick_count"], steps)
    frame_count = checked_add(state["frame_count"], 1)
    sim_advance = steps * dt
    if sim_advance > MAX_U64:
        fail("TimestampOverflow")
    elapsed_sim = checked_add(state["elapsed_sim_ns"], sim_advance)
    elapsed_wall = checked_add(state["elapsed_wall_ns"], clamped)
    dropped = checked_add(wall_drop, spiral_drop)
    total_dropped = checked_add(state["total_dropped_ns"], dropped)
    spiral_frames = checked_add(state["spiral_clamped_frames"], int(spiral))

    next_state = {
        "tick_count": tick_count,
        "frame_count": frame_count,
        "elapsed_sim_ns": elapsed_sim,
        "elapsed_wall_ns": elapsed_wall,
        "accumulator_ns": remaining,
        "fixed_dt_ns": dt,
        "max_frame_steps": state["max_frame_steps"],
        "max_frame_delta_ns": state["max_frame_delta_ns"],
        "total_dropped_ns": total_dropped,
        "spiral_clamped_frames": spiral_frames,
    }
    alpha_milli = remaining * 1000 // dt
    alpha_q16 = remaining * 65536 // dt
    dropped_this_frame = dropped
    if (
        state["accumulator_ns"] + raw_delta
        != steps * dt + dropped_this_frame + remaining
    ):
        raise AssertionError("model violated per-frame conservation")
    return {
        "state": state_out(next_state),
        "steps_to_run": steps,
        "alpha_milli": alpha_milli,
        "alpha_q16": alpha_q16,
        "spiral_of_death_clamped": spiral,
        "dropped_ns": str(dropped_this_frame),
    }


def expected_for(case: dict[str, Any]) -> dict[str, Any]:
    op = case["op"]
    inp = case["input"]
    if op == "standard_60hz":
        keys(inp, set(), f"{case['id']}.input")
        return {
            "ok": {
                "config": config_out(
                    {
                        "fixed_dt_ns": 16_666_667,
                        "max_frame_steps": 8,
                        "max_frame_delta_ns": DEFAULT_MAX_DELTA_NS,
                    }
                )
            }
        }
    if op == "from_hz":
        keys(inp, {"hz", "max_frame_steps"}, f"{case['id']}.input")
        hz = u32(inp["hz"], f"{case['id']}.input.hz")
        steps = u32(inp["max_frame_steps"], f"{case['id']}.input.max_frame_steps")
        try:
            if hz == 0:
                fail("ZeroFixedTimestep")
            if steps == 0 or steps > HARD_MAX_STEPS:
                fail("InvalidMaxFrameSteps")
            dt = 1_000_000_000 // hz
            if dt == 0:
                fail("ZeroFixedTimestep")
            if dt > HARD_MAX_DELTA_NS:
                fail("InvalidMaxFrameDelta")
            cfg = {
                "fixed_dt_ns": dt,
                "max_frame_steps": steps,
                "max_frame_delta_ns": max(DEFAULT_MAX_DELTA_NS, dt),
            }
            valid_config(cfg)
            return {"ok": {"config": config_out(cfg)}}
        except ClockFailure as error:
            return {"error": error.code}
    if op == "new_clock":
        keys(inp, {"config"}, f"{case['id']}.input")
        cfg = config_in(inp["config"], f"{case['id']}.input.config")
        try:
            return {"ok": {"state": state_out(make_clock(cfg))}}
        except ClockFailure as error:
            return {"error": error.code}
    if op == "validate_state":
        keys(inp, {"state"}, f"{case['id']}.input")
        state = state_in(inp["state"], f"{case['id']}.input.state")
        try:
            validate_state(state)
            return {"ok": True}
        except ClockFailure as error:
            return {"error": error.code}
    if op == "advance_frame":
        keys(inp, {"state", "raw_frame_delta_ns"}, f"{case['id']}.input")
        state = state_in(inp["state"], f"{case['id']}.input.state")
        raw = u64(inp["raw_frame_delta_ns"], f"{case['id']}.input.raw_frame_delta_ns")
        try:
            return {"ok": advance(state, raw)}
        except ClockFailure as error:
            return {"error": error.code}
    raise ValueError(f"{case['id']}: unsupported operation {op!r}")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def main() -> int:
    path = Path(__file__).with_name("cases.json")
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys
        )
        keys(document, {"version", "suite", "cases"}, "corpus")
        if type(document["version"]) is not int or document["version"] != 1:
            raise ValueError("corpus version must be integer 1")
        if document["suite"] != "game_clock_portable_v1":
            raise ValueError("unexpected suite name")
        cases = document["cases"]
        if type(cases) is not list or not cases:
            raise ValueError("cases must be a non-empty array")
        seen: set[str] = set()
        outcomes = {"ok": 0, "error": 0}
        operations: set[str] = set()
        for index, case in enumerate(cases):
            keys(case, {"id", "op", "input", "expected"}, f"cases[{index}]")
            case_id = case["id"]
            if type(case_id) is not str or not case_id or case_id in seen:
                raise ValueError(f"cases[{index}] has an empty or duplicate id")
            seen.add(case_id)
            op = case["op"]
            if type(op) is not str:
                raise ValueError(f"{case_id}: op must be a string")
            operations.add(op)
            actual = expected_for(case)
            encoded_actual = json.dumps(
                actual, sort_keys=True, separators=(",", ":"), allow_nan=False
            )
            encoded_expected = json.dumps(
                case["expected"], sort_keys=True, separators=(",", ":"), allow_nan=False
            )
            if encoded_actual != encoded_expected:
                raise ValueError(
                    f"{case_id}: expected {encoded_expected}, model produced {encoded_actual}"
                )
            outcomes["error" if "error" in actual else "ok"] += 1
        required = {
            "standard_60hz",
            "from_hz",
            "new_clock",
            "validate_state",
            "advance_frame",
        }
        if not required.issubset(operations):
            raise ValueError(f"missing operations: {sorted(required - operations)}")
        if not outcomes["ok"] or not outcomes["error"]:
            raise ValueError("corpus needs both successful and rejected vectors")
    except (OSError, json.JSONDecodeError, ValueError, ClockFailure) as error:
        print(f"GAME10 fixture validation failed: {error}", file=sys.stderr)
        return 1
    print(
        f"GAME10 fixture valid: {len(cases)} cases, {outcomes['ok']} accepted, {outcomes['error']} rejected"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
