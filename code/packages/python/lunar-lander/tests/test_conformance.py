"""Shared corpus conformance tests against cases.json.

Verifies 100% bit-exact parity across multi-turn trajectories, intermediate states,
ground touchdown contact time solver, fuel refunds, and all refusal/error arms.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from lunar_lander import (
    BurnCommand,
    LanderError,
    LanderState,
    step_burn,
    validate_burn,
)

FIXTURES_PATH = (
    Path(__file__).parents[4] / "specs" / "fixtures" / "lunar-lander-v1" / "cases.json"
)


def load_fixtures() -> list[dict]:
    with open(FIXTURES_PATH, encoding="utf-8") as f:
        data = json.load(f)
    assert data["version"] == 1
    assert data["suite"] == "lunar_lander_deterministic_flight"
    assert len(data["cases"]) == 39
    return data["cases"]


@pytest.mark.parametrize("case", load_fixtures(), ids=lambda c: c["id"])
def test_case_conformance(case: dict) -> None:
    """Replays every case from canonical lunar-lander-v1 corpus."""
    initial_state = LanderState(**case["initial_state"])

    if case.get("turns"):
        state = initial_state
        for turn_data in case["turns"]:
            cmd_data = turn_data["command"]
            cmd = BurnCommand(
                burn_rate_lbs_s=cmd_data["burn_rate_lbs_s"],
                duration_seconds=cmd_data["duration_seconds"],
            )

            # Verification: validate_burn must succeed
            validate_burn(state, cmd)

            # Step simulation
            next_state = step_burn(state, cmd)
            assert next_state is not state

            exp_state = turn_data["expected_state"]
            actual = next_state.to_dict()

            for key, exp_val in exp_state.items():
                actual_val = actual[key]
                t_num = turn_data.get("turn")
                assert actual_val == exp_val, (
                    f"Case {case['id']} turn {t_num} '{key}' mismatch: "
                    f"expected {exp_val!r}, got {actual_val!r}"
                )

            state = next_state
    elif case.get("command"):
        cmd_data = case["command"]
        cmd = BurnCommand(
            burn_rate_lbs_s=cmd_data["burn_rate_lbs_s"],
            duration_seconds=cmd_data["duration_seconds"],
        )
        expected_err = case["expected_error"]

        with pytest.raises(LanderError) as val_exc:
            validate_burn(initial_state, cmd)
        assert (
            val_exc.value == expected_err
            or val_exc.value.code == expected_err
            or val_exc.value.__class__.__name__ == expected_err
        ), f"Case {case['id']} validate_burn error mismatch"

        with pytest.raises(LanderError) as step_exc:
            step_burn(initial_state, cmd)
        assert (
            step_exc.value == expected_err
            or step_exc.value.code == expected_err
            or step_exc.value.__class__.__name__ == expected_err
        ), f"Case {case['id']} step_burn error mismatch"
    else:
        pytest.fail(f"Case {case['id']} has neither turns nor command")
