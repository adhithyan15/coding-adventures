#!/usr/bin/env python3
"""test_native_setcover.py — the native engine set-cover agrees with the Python one.

Guards that the regimen derived by the adj-lang integer optimizer (the engine)
matches the Python exhaustive set-cover on cost, across the meningitis scenarios,
and that the engine abstains (Infeasible) when no regimen exists. Skips the CLI
checks if adj-lang-cli is not built. 0 answer-time model calls.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402
import derive_regimen as reg  # noqa: E402
import native_setcover as ns  # noqa: E402


def test_emit_is_well_formed() -> None:
    """The emitted program declares a bool selector per candidate + an objective."""
    prog, var_to_drug, feasible = ns.emit_program(
        reg.SCENARIOS["adult_community"], set())
    assert feasible
    assert "minimize" in prog
    assert all(":" in line for line in prog.splitlines() if line.startswith("symbol"))
    # resistant pneumococcus is covered only by a combination → an AND auxiliary.
    assert "symbol y_" in prog, prog


def test_default_weights_reproduce_tier_only_objective() -> None:
    """CC-4: the default objective weight (1, 0) emits exactly the historical
    `minimize Σ tier·x` — toxicity is inert until a policy raises w_tox, so every
    existing consumer is unchanged. The objective coefficient of each drug == its tier."""
    prog_default, _, _ = ns.emit_program(reg.SCENARIOS["adult_community"], set())
    prog_explicit, _, _ = ns.emit_program(reg.SCENARIOS["adult_community"], set(), weights=(1, 0))
    assert prog_default == prog_explicit
    obj = [ln for ln in prog_default.splitlines() if ln.startswith("minimize")][0]
    for d, v in {"vancomycin": "x_vancomycin", "moxifloxacin": "x_moxifloxacin"}.items():
        assert f"{reg.DRUGS[d]['tier']} * {v}" in obj, obj


def test_side_effect_weights_loaded() -> None:
    """CC-4: every candidate drug carries a non-negative integer side_effects weight
    (the authored-debt toxicity layer), so the cost+side-effect objective is defined."""
    for d in reg.DRUGS:
        se = reg.DRUGS[d].get("side_effects")
        assert isinstance(se, int) and not isinstance(se, bool) and se >= 0, (d, se)


def main() -> int:
    test_emit_is_well_formed()
    test_default_weights_reproduce_tier_only_objective()
    test_side_effect_weights_loaded()
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_native_setcover: PASS (emit + CC-4 weight checks); "
              "CLI checks SKIPPED (adj-lang-cli not built)")
        return 0

    # Adult community: engine derives the combination regimen at cost 2.
    res = ns.solve(cli, reg.SCENARIOS["adult_community"], set())
    assert res["outcome"] == "optimal" and abs(res["cost"] - 2) < 1e-9, res
    assert set(res["regimen"]) == {"vancomycin", "ceftriaxone"}, res

    # Engine cost must equal the Python set-cover cost on every scenario.
    for organisms, excl in [
        (reg.SCENARIOS["adult_community"], set()),
        (reg.SCENARIOS["over_50_or_immunocompromised"], set()),
        (reg.SCENARIOS["post_neurosurgical_or_shunt"], set()),
    ]:
        eng = ns.solve(cli, organisms, excl)
        py = reg.min_cost_cover(reg.candidates(excl), organisms)
        py_cost = sum(reg.DRUGS[d]["tier"] for d in py)
        assert abs(eng["cost"] - py_cost) < 1e-9, (organisms, eng, py_cost)

    # Severe beta-lactam allergy → no β-lactam-free combination covers resistant
    # pneumococcus → the engine reports Infeasible (honest abstention).
    out = ns.solve(cli, reg.SCENARIOS["adult_community"], {"betalactam_allergy_severe"})
    assert out["regimen"] is None and out["outcome"] == "infeasible", out

    # ---- CC-4: cost + side-effect multi-objective ----
    # `pseudomonas` has three coverers with anti-correlated cost/toxicity:
    # cefepime (tier 2, se 2), meropenem (tier 3, se 2), aztreonam (tier 4, se 1).
    # Under the cost objective the cheapest tier wins (cefepime); raise w_tox and the
    # pricier-but-safer agent wins (aztreonam) — the regimen FLIPS on the objective.
    cost = ns.solve(cli, ["pseudomonas"], set(), weights=(1, 0))
    assert cost["regimen"] == ["cefepime"], cost
    assert cost["objective"]["total"] == cost["objective"]["cost"], cost      # w_tox=0 → total is cost
    low_tox = ns.solve(cli, ["pseudomonas"], set(), weights=(1, 3))
    assert low_tox["regimen"] == ["aztreonam"], low_tox
    # The breakdown is internally consistent: total == w_cost·cost + w_tox·side_effects,
    # and it equals the engine's reported optimal objective value.
    ob = low_tox["objective"]
    assert ob["total"] == 1 * ob["cost"] + 3 * ob["side_effects"], ob
    assert abs(low_tox["cost"] - ob["total"]) < 1e-9, low_tox

    # The engine must agree with the Python weighted set-cover under EVERY weight blend
    # (the multi-objective invariant, not just tier-only).
    for organisms in (["pseudomonas"], reg.SCENARIOS["post_neurosurgical_or_shunt"],
                      reg.SCENARIOS["adult_community"]):
        for w in ((1, 0), (1, 1), (1, 3)):
            eng = ns.solve(cli, organisms, set(), weights=w)
            py = reg.min_cost_cover(reg.candidates(set()), organisms, w)
            py_obj = sum(reg.drug_weight(d, w) for d in py)
            assert abs(eng["objective"]["total"] - py_obj) < 1e-9, (organisms, w, eng, py_obj)

    print("test_native_setcover: PASS (engine integer optimizer agrees with the Python "
          "set-cover under every cost/side-effect weight blend; objective flips cefepime→"
          "aztreonam under w_tox; abstains via Infeasible; 0 model calls)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
