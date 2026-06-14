#!/usr/bin/env python3
"""test_treatment.py - guard the treatment-as-constraint layer. 0 model calls.

Asserts the SAME constraint solver that diagnoses also makes the treatment
decision, and that cost+time can override the argmax probability:
  - cost break-even solves to p* = cost_treat/cost_miss;
  - waiting for culture within the door-to-antibiotic deadline is UNSAT;
  - at a probability where viral is MORE probable (P(bacterial) < 0.5) but above
    p*, the action is still "treat empirically now" (the override);
  - below p*, treatment is withheld.
Skips cleanly without adj-lang-cli.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import treat as treat_mod  # noqa: E402
sys.path.insert(0, str(HERE.parent / "warm"))
import decide as decide_mod  # noqa: E402


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_treatment: SKIPPED (adj-lang-cli not built)")
        return 0

    # cost break-even = cost_treat / cost_miss.
    p_star = treat_mod.cost_breakeven(cli)
    assert abs(p_star - treat_mod.COST_TREAT / treat_mod.COST_MISS) < 1e-9, p_star

    # waiting for culture within the deadline is infeasible (solver proves it).
    timing = treat_mod.can_wait_for_culture(cli)
    assert timing.get("outcome") == "unsat", timing
    assert timing.get("core"), timing

    # OVERRIDE: viral more probable (P=0.30) but above p* -> treat anyway.
    over = treat_mod.recommend("synthetic", cli, p_override=0.30)
    assert over["answer_time_model_calls"] == 0
    assert over["most_probable_dx"] == "viral_meningitis"
    assert over["action"].startswith("TREAT")
    assert over["overrides_argmax_probability"] is True, over

    # Below p* -> withhold.
    low = treat_mod.recommend("synthetic", cli, p_override=p_star / 2)
    assert not low["treat_threshold_met"], low
    assert low["action"].startswith("WITHHOLD"), low

    print(f"test_treatment: PASS (p*={p_star}; wait-for-culture UNSAT core={timing['core']}; "
          f"P=0.30 viral-more-probable -> TREAT overrides argmax; P<p* -> withhold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
