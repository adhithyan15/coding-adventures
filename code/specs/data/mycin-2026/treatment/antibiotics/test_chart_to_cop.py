#!/usr/bin/env python3
"""test_chart_to_cop.py — guard the chart→constraint compiler (CC-1).

Pure checks (no engine): the compiler maps chart facts to the right COP inputs, records
provenance for every constraint, and DISCARDS unmapped facts with a reason. Engine-gated
checks (if the adj-lang-cli is built): compiling the four canonical charts reproduces the
existing meningitis regimens — proving set-cover is the special case of the chart-driven
COP — and that a severe β-lactam allergy abstains (INFEASIBLE) exactly like
native_setcover, never inventing a regimen.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import chart_to_cop as cc  # noqa: E402
import decide as decide_mod  # noqa: E402


def test_scenario_selection_and_provenance():
    # Default adult → community scenario; every constraint carries its source fact + rule.
    cop = cc.compile_cop([cc.ChartFact("age_band", "adult", "45M")])
    assert cop.organisms and "n_meningitidis" in cop.organisms
    assert any(c["type"] == "coverage" for c in cop.constraints)
    assert all("from" in c and "rule" in c for c in cop.constraints)
    # Older adult OR immunocompromised → the over-50 set (adds listeria).
    older = cc.compile_cop([cc.ChartFact("age_band", "older_adult")])
    assert "listeria" in older.organisms
    immuno = cc.compile_cop([cc.ChartFact("immune_status", "immunocompromised")])
    assert "listeria" in immuno.organisms
    # Post-neurosurgical is most-specific → wins even alongside an age fact.
    neuro = cc.compile_cop([cc.ChartFact("age_band", "older_adult"),
                            cc.ChartFact("setting", "csf_shunt")])
    assert "pseudomonas" in neuro.organisms


def test_allergy_becomes_exclusion():
    cop = cc.compile_cop([cc.ChartFact("allergy", "penicillin", "anaphylaxis")])
    assert "betalactam_allergy_severe" in cop.exclusions
    assert any(c["type"] == "exclusion" for c in cop.constraints)


def test_culture_resistance_becomes_defeated_edge():
    cop = cc.compile_cop([cc.ChartFact("culture_resistance", "ceftriaxone:n_meningitidis", "S/I/R panel")])
    assert ("ceftriaxone", "n_meningitidis") in cop.defeated
    assert any(c["type"] == "defeated_edge" for c in cop.constraints)


def test_unmapped_fact_is_discarded_not_ignored():
    # No silent drops: a fact with no rule lands in discards WITH a reason.
    cop = cc.compile_cop([cc.ChartFact("age_band", "adult"),
                          cc.ChartFact("favorite_color", "blue")])
    assert any("favorite_color" in d["fact"] and d["reason"] for d in cop.discards)
    # An allergen with no grounded exclusion rule yet is also discarded (not excluded).
    cop2 = cc.compile_cop([cc.ChartFact("allergy", "sulfa")])
    assert not cop2.exclusions and any("sulfa" in d["fact"] for d in cop2.discards)


def test_engine_reproduces_existing_regimens():
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_chart_to_cop: PASS (pure compiler checks); engine portion SKIPPED (no cli)")
        return
    want = {
        "adult_community": {"ceftriaxone", "vancomycin"},
        "over_50_or_immunocompromised": {"ampicillin", "ceftriaxone", "vancomycin"},
        "post_neurosurgical_or_shunt": {"cefepime", "vancomycin"},
    }
    for name, regimen in want.items():
        r = cc.derive(cli, cc.CHARTS[name])
        assert r["regimen"] is not None and set(r["regimen"]) == regimen, (name, r["regimen"])
    # Severe β-lactam allergy → honest abstention (INFEASIBLE), never a fabricated regimen.
    alg = cc.derive(cli, cc.CHARTS["betalactam_allergic_adult"])
    assert alg["regimen"] is None and alg["outcome"] == "infeasible", alg
    assert alg["conflict"] is not None, "infeasible must name the conflicting constraint"
    print("test_chart_to_cop: PASS (compiler provenance + discards; engine reproduces the "
          "4 meningitis regimens as a special case; β-lactam allergy abstains; 0 model calls)")


def main() -> int:
    test_scenario_selection_and_provenance()
    test_allergy_becomes_exclusion()
    test_culture_resistance_becomes_defeated_edge()
    test_unmapped_fact_is_discarded_not_ignored()
    test_engine_reproduces_existing_regimens()
    return 0


if __name__ == "__main__":
    sys.exit(main())
