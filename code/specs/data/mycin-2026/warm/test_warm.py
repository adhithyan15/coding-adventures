#!/usr/bin/env python3
"""test_warm.py - guard the warm path: the committed IRs re-decide at 0 model calls.

The decompose step (decompose.py) ran the model ONCE per case and committed the
IRs (ir/*.json). This test re-runs the DETERMINISTIC half (ir_to_adj -> decide)
and asserts every case still reaches its gold diagnosis with
answer_time_model_calls_total == 0 - i.e. the diagnosis is a pure function of the
decomposition + the grounded CAS rulebook, with no model in the answer loop. CI
runs the same. Skips cleanly if adj-lang-cli is not built.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decide as decide_mod  # noqa: E402
import run_warm  # noqa: E402


def main() -> int:
    if decide_mod.find_cli() is None:
        print("test_warm: SKIPPED (adj-lang-cli not built)")
        return 0
    rc = run_warm.main()
    summary = json.loads((ROOT / "warm" / "decide-results.json").read_text())
    assert summary["answer_time_model_calls_total"] == 0, summary
    assert summary["wrong"] == 0, f"warm path regressed: {summary}"
    assert summary["correct"] >= 4, f"expected >=4 correct, got {summary['correct']}"
    print("test_warm: PASS "
          f"({summary['correct']} correct, {summary['abstained']} abstained, "
          f"0 wrong, 0 answer-time model calls)")
    return rc


if __name__ == "__main__":
    sys.exit(main())
