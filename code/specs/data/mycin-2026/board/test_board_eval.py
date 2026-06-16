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
    store = be.load_store()
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
    # The bank spans THREE recall domains: 18 IEM + 8 vitamin (REL-10) + 8 anemia
    # (REL-11) = 34 covered recall items, all over one merged store.
    recall_correct = [r for r in card.results if r.outcome == "correct" and r.tactic == "recall"]
    assert len(recall_correct) == 34
    assert by_id["fabry_enzyme"].outcome == "correct"          # IEM
    assert by_id["thiamine_disease"].answer == "beriberi"      # vitamin
    assert by_id["ida_mcv"].answer == "microcytic"             # anemia (REL-11)
    assert by_id["b12_anemia_finding"].answer == "hypersegmented_neutrophils"


def test_uncovered_items_abstain_not_fabricate() -> None:
    card, _ = _card()
    by_id = {r.item_id: r for r in card.results}
    # Diseases genuinely absent from the graph must abstain, not fabricate.
    assert by_id["wilson_disease_enzyme"].outcome == "abstained"
    assert by_id["wilson_disease_enzyme"].answer is None
    assert by_id["menkes_enzyme"].outcome == "abstained"


def test_defensibility_is_full() -> None:
    card, _ = _card()
    # Every item is either correct-with-proof or an honest abstention.
    assert card.summary()["defensibility"] == 1.0
    assert card.summary()["accuracy_on_attempted"] == 1.0


def test_grounded_coverage_is_the_live_grounding_number() -> None:
    card, _ = _card()
    s = card.summary()
    # All THREE recall domains are now spider-grounded: IEM (REL-8) + vitamin (REL-10b)
    # + anemia (REL-11b). So every one of the 34 recall answers cites an authoritative
    # edge: grounded-coverage 100%. Expansion added debt domain by domain; grounding
    # retired it each time — the same one number across the whole campaign.
    assert s["grounded_coverage"] == 1.0
    assert s["grounded_correct"] == 34
    by_id = {r.item_id: r for r in card.results}
    assert by_id["tay_sachs_enzyme"].trust == "authoritative"      # IEM
    assert by_id["thiamine_disease"].trust == "authoritative"      # vitamin
    assert by_id["ida_mcv"].trust == "authoritative"               # anemia (REL-11b)


def test_gate_exit_code_zero_when_no_fabrication() -> None:
    assert be.main(["--quiet"]) == 0


# ---- REL-7: differential tactic ----

def test_score_differential_logic() -> None:
    # determinate → commits to the leader: correct iff it matches gold.
    det = {"type": "determinate", "leader": "bacterial_meningitis"}
    assert be.score_differential(det, "bacterial_meningitis") == ("correct", "bacterial_meningitis")
    assert be.score_differential(det, "viral_meningitis") == ("wrong", "bacterial_meningitis")
    # committing a leader when the gold is ABSTAIN is a fabrication (wrong).
    assert be.score_differential(det, "ABSTAIN")[0] == "wrong"
    # kickback / empty → the engine declined to commit: abstain (correct vs ABSTAIN).
    kick = {"type": "kickback", "leader": "bacterial_meningitis", "runner_up": "viral_meningitis"}
    assert be.score_differential(kick, "ABSTAIN") == ("abstained", None)
    assert be.score_differential({"type": "empty"}, "bacterial_meningitis") == ("abstained", None)
    # No decision (CLI unavailable) → abstain, never fabricate.
    assert be.score_differential(None, "bacterial_meningitis") == ("abstained", None)


def test_differential_items_never_fabricate() -> None:
    # Regardless of whether the CLI is built, differential items are correct or
    # abstained — never wrong. (When the binary is absent they abstain.)
    card, _ = _card_with_diff()
    diff = [r for r in card.results if r.tactic == "differential"]
    assert diff, "the bank has differential items"
    assert all(r.outcome in ("correct", "abstained") for r in diff)


def test_differential_runs_natively_when_cli_present() -> None:
    if not be.cli_available():
        return  # skip: Python-only environment without the built Rust CLI
    card, _ = _card_with_diff()
    by_id = {r.item_id: r for r in card.results}
    # Strong CSF → the engine commits to bacterial; equivocal CSF → it abstains.
    assert by_id["meningitis_bacterial_dx"].outcome == "correct"
    assert by_id["meningitis_bacterial_dx"].answer == "bacterial_meningitis"
    assert by_id["meningitis_equivocal_dx"].outcome == "abstained"


def _card_with_diff():
    import json
    items = json.loads((HERE / "items.json").read_text())["items"]
    store = be.load_store()
    return be.score(items, store), items


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
