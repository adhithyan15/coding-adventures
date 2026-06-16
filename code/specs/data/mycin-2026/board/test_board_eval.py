#!/usr/bin/env python3
"""test_board_eval.py — REL-5 board-eval scoreboard tests.

Pins the defensibility contract: on covered items the harness answers correctly
WITH a proof; on deliberately-uncovered items it abstains; it NEVER fabricates
(wrong == 0). Also pins that grounded-coverage tracks the edges' trust tier — the
live number a grounding PR moves (today 0%, all authored-debt).

Run:  python3 test_board_eval.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import board_eval as be  # noqa: E402


def _card():
    items = json.loads((HERE / "items.json").read_text())["items"]
    store = be.recall.parse_edges(be.RECALL / "iem-edges.adj")
    return be.score(items, store), items


def test_never_fabricates() -> None:
    card, _ = _card()
    assert card.summary()["wrong"] == 0, "a wrong answer is a fabrication — the one hard failure"


def test_covered_items_answer_correctly_with_a_proof() -> None:
    card, _ = _card()
    by_id = {r.item_id: r for r in card.results}
    assert by_id["tay_sachs_enzyme"].outcome == "correct"
    assert by_id["tay_sachs_enzyme"].answer == "hexosaminidase_a"
    # A correct recall answer carries the citing edge's trust tier (its proof).
    assert by_id["tay_sachs_enzyme"].trust is not None
    assert sum(1 for r in card.results if r.outcome == "correct") == 10


def test_uncovered_items_abstain_not_fabricate() -> None:
    card, _ = _card()
    by_id = {r.item_id: r for r in card.results}
    assert by_id["niemann_pick_enzyme"].outcome == "abstained"
    assert by_id["niemann_pick_enzyme"].answer is None
    assert by_id["fabry_enzyme"].outcome == "abstained"


def test_defensibility_is_full() -> None:
    card, _ = _card()
    # Every item is either correct-with-proof or an honest abstention.
    assert card.summary()["defensibility"] == 1.0
    assert card.summary()["accuracy_on_attempted"] == 1.0


def test_grounded_coverage_is_the_live_grounding_number() -> None:
    card, _ = _card()
    s = card.summary()
    # After the REL-4b spider, 16/18 IEM edges are spider-grounded (authoritative);
    # 2 stayed direction_only → consensus (the adversarial verify could not confirm
    # byte-stability). Of the 10 correct board items, 9 cite an authoritative edge —
    # the lone holdout is lesch_nyhan_enzyme (deficient_in__lesch_nyhan, direction_only).
    # This number IS the grounding progress: it moved 0% → 90% with no harness change.
    assert s["grounded_coverage"] == 0.9
    assert s["grounded_correct"] == 9
    by_id = {r.item_id: r for r in card.results}
    assert by_id["lesch_nyhan_enzyme"].trust == "consensus"
    assert by_id["tay_sachs_enzyme"].trust == "authoritative"


def test_gate_exit_code_zero_when_no_fabrication() -> None:
    assert be.main(["--quiet"]) == 0


def _run() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"  FAIL  {t.__name__}: {exc}")
    print(f"\ntest_board_eval: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
