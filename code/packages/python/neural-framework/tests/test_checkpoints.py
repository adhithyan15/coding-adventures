"""NN40: the same hostile documents must reach each native loader unchanged.

Accepted controls prevent an always-rejecting loader from earning conformance.
The corpus describes parsed JSON, not runtime-specific text parsing behavior.
"""

import json
from pathlib import Path

import pytest

from neural_framework import DenseNetwork

CORPUS = json.loads(
    (
        Path(__file__).resolve().parents[4] / "specs/fixtures/neural-framework-v1/checkpoints.json"
    ).read_text(encoding="utf-8")
)


def test_corpus_inventory():
    assert set(CORPUS) == {"contract", "cases"}
    assert CORPUS["contract"] == "NN40"
    assert len(CORPUS["cases"]) == 55
    assert len({case["id"] for case in CORPUS["cases"]}) == 55
    assert sum(case["accept"] for case in CORPUS["cases"]) == 3
    for case in CORPUS["cases"]:
        assert set(case) == {"id", "accept", "checkpoint"}
        assert isinstance(case["id"], str) and case["id"]
        assert type(case["accept"]) is bool
    # NN43 pins the shared numeric-null boundary, not merely a larger count.
    by_id = {case["id"]: case for case in CORPUS["cases"]}
    for case_id in (
        "null-weight-cell",
        "null-weight-cell-mixed",
        "null-bias-cell",
        "null-bias-cell-mixed",
        "null-weight-cell-later-layer",
        "null-bias-cell-later-layer",
    ):
        assert by_id[case_id]["accept"] is False


@pytest.mark.parametrize("case", CORPUS["cases"], ids=lambda case: case["id"])
def test_shared_checkpoint_decision(case):
    if case["accept"]:
        model = DenseNetwork.from_checkpoint(case["checkpoint"])
        saved = model.to_checkpoint()
        assert DenseNetwork.from_checkpoint(saved).to_checkpoint() == saved
        assert saved == case["checkpoint"]
    else:
        with pytest.raises(ValueError):
            DenseNetwork.from_checkpoint(case["checkpoint"])
