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


def main() -> int:
    test_emit_is_well_formed()
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_native_setcover: PASS (emit checks); CLI checks SKIPPED (adj-lang-cli not built)")
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

    print("test_native_setcover: PASS (engine integer optimizer agrees with the "
          "Python set-cover on cost; abstains via Infeasible; 0 model calls)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
