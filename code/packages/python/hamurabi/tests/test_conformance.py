"""Shared corpus conformance tests against code/specs/fixtures/hamurabi-v1/cases.json.

Verifies 100% bit-exact parity across 10-year trajectories, intermediate states,
all 8 error arms, and authentic teletype report strings.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from hamurabi import (
    HamurabiError,
    HamurabiState,
    TurnOrder,
    format_report,
    new_game,
    parse_status,
    step_year,
    validate_order,
)

FIXTURES_PATH = (
    Path(__file__).parents[4] / "specs" / "fixtures" / "hamurabi-v1" / "cases.json"
)


def load_fixtures() -> list[dict]:
    with open(FIXTURES_PATH, encoding="utf-8") as f:
        return json.load(f)["cases"]


@pytest.mark.parametrize("case", load_fixtures(), ids=lambda c: c["id"])
def test_case_conformance(case: dict) -> None:
    """Replays every case from the canonical hamurabi-v1 corpus."""
    if "initial_state" in case:
        init_d = dict(case["initial_state"])
        init_d["status"] = parse_status(init_d["status"])
        state = HamurabiState(**init_d)
    else:
        state = new_game(case["seed"])

    if "turns" in case:
        for turn_data in case["turns"]:
            order_dict = turn_data["order"]
            order = TurnOrder(
                acres_delta=order_dict["acres_delta"],
                bushels_to_feed=order_dict["bushels_to_feed"],
                acres_to_plant=order_dict["acres_to_plant"],
            )

            if "expected_error" in turn_data:
                expected_err_name = turn_data["expected_error"]

                with pytest.raises(HamurabiError) as val_exc:
                    validate_order(state, order)
                assert (
                    val_exc.value.code == expected_err_name
                    or val_exc.value.__class__.__name__ == expected_err_name
                )

                with pytest.raises(HamurabiError) as step_exc:
                    step_year(state, order)
                assert (
                    step_exc.value.code == expected_err_name
                    or step_exc.value.__class__.__name__ == expected_err_name
                )
            else:
                validate_order(state, order)
                next_state = step_year(state, order)
                assert next_state is not state

                exp_state_dict = turn_data["expected_state"]
                actual_dict = next_state.to_dict()

                for key, exp_val in exp_state_dict.items():
                    actual_val = actual_dict[key]
                    assert actual_val == exp_val, (
                        f"Turn {turn_data.get('turn')} field '{key}' mismatch: "
                        f"expected {exp_val!r}, got {actual_val!r}"
                    )

                if "expected_report" in turn_data:
                    actual_report = format_report(next_state)
                    assert actual_report == turn_data["expected_report"]

                state = next_state
    elif "order" in case:
        order_dict = case["order"]
        order = TurnOrder(
            acres_delta=order_dict["acres_delta"],
            bushels_to_feed=order_dict["bushels_to_feed"],
            acres_to_plant=order_dict["acres_to_plant"],
        )
        expected_err_name = case["expected_error"]

        with pytest.raises(HamurabiError) as val_exc:
            validate_order(state, order)
        assert (
            val_exc.value.code == expected_err_name
            or val_exc.value.__class__.__name__ == expected_err_name
        )

        with pytest.raises(HamurabiError) as step_exc:
            step_year(state, order)
        assert (
            step_exc.value.code == expected_err_name
            or step_exc.value.__class__.__name__ == expected_err_name
        )
