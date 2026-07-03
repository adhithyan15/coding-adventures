#!/usr/bin/env python3
"""test_identify_bsi.py — guard bacteremia source→organism→cover. 0 model calls.

Asserts the source drives the organism differential, the significant set maps onto
the bacteremia formulary and derives a sensible empiric regimen, and that a severe
beta-lactam allergy with gram-negative needs ABSTAINS (no fabricated regimen).
Skips the CLI checks if adj-lang-cli is not built. CI runs this.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402
import identify_bsi as bsi  # noqa: E402


def test_cover_pure() -> None:
    """Generic set-cover: picks lowest-tier covering set; abstains when impossible."""
    drugs = {
        "broad": {"covers": ["gnb", "anaerobe"], "tier": 1, "contraindications": ["allergy"]},
        "narrow_a": {"covers": ["gnb"], "tier": 1, "contraindications": []},
        "narrow_b": {"covers": ["anaerobe"], "tier": 1, "contraindications": []},
    }
    # One broad drug (tier 1) beats two narrow (tier 1+1) by fewest-drugs tiebreak.
    assert bsi.min_cost_cover(drugs, ["gnb", "anaerobe"], set()) == ["broad"]
    # Exclude the broad one → must use the two narrow agents.
    cover = bsi.min_cost_cover(drugs, ["gnb", "anaerobe"], {"allergy"})
    assert cover is not None and set(cover) == {"narrow_a", "narrow_b"}, cover
    # No drug covers 'mrsa' → abstain.
    assert bsi.min_cost_cover(drugs, ["mrsa"], set()) is None


def test_token_validation() -> None:
    cli = decide_mod.find_cli()
    if cli is None:
        return
    try:
        bsi.run_differential(cli, {"infection_source": "urinary)\nobserve x("})
    except ValueError:
        return
    raise AssertionError("expected ValueError on an injection-shaped finding token")


def main() -> int:
    test_cover_pure()
    test_token_validation()
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_identify_bsi: PASS (pure checks); CLI checks SKIPPED (adj-lang-cli not built)")
        return 0

    # Urinary source → enteric GNB leads.
    ranked = bsi.run_differential(cli, {"infection_source": "urinary"})
    assert ranked[0]["hypothesis"] == "enteric_gnb", ranked[0]

    # Central line → S. aureus / CoNS dominate the top of the differential.
    ranked = bsi.run_differential(cli, {"infection_source": "intravascular_line"})
    top2 = {ranked[0]["hypothesis"], ranked[1]["hypothesis"]}
    assert top2 == {"s_aureus", "coag_neg_staph"}, top2

    # Intra-abdominal → piperacillin-tazobactam alone covers GNB + anaerobes + enterococcus.
    ranked = bsi.run_differential(cli, {"infection_source": "intraabdominal"})
    sig = bsi.significant_set(ranked)
    assert {"enteric_gnb", "anaerobes"} <= set(sig), sig
    cover = bsi.min_cost_cover(bsi.DRUGS, sig, set())
    assert cover == ["piperacillin_tazobactam"], cover

    # Same source + severe beta-lactam allergy → no GN coverage left → abstain.
    assert bsi.min_cost_cover(bsi.DRUGS, sig, {"betalactam_allergy_severe"}) is None

    print("test_identify_bsi: PASS (source drives the organism; pip-tazo covers the "
          "intra-abdominal triad; severe beta-lactam allergy abstains; 0 model calls)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
