#!/usr/bin/env python3
"""test_run_chart_to_regimen.py — the CC-7 full-chart drive-through, end to end.

Drives RAW FHIR Bundles (PHI in) through deidentify → to_chartfacts → chart_to_cop.derive in a
single `chart_to_regimen` call and asserts: (a) PHI is stripped and the de-identification report
is part of the audit trail; (b) the constraint solver returns the right decision — a regimen for
a straightforward chart, an honest INFEASIBLE+conflict for a complex one; (c) every stage is
deterministic (answer_time_model_calls == 0). Engine-gated on `find_cli()` (pure-mapping
assertions still run without the CLI); mirrors test_deidentify.py's two-tier pattern.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "treatment" / "antibiotics"))
sys.path.insert(0, str(HERE.parent / "warm"))
import decide as decide_mod  # noqa: E402
import deidentify  # noqa: E402
import fhir_to_chartfacts as f2c  # noqa: E402
import run_chart_to_regimen as rc  # noqa: E402

FEASIBLE = HERE / "samples" / "chart_feasible_adult_bundle.json"
INFEASIBLE = HERE / "samples" / "chart_with_phi_bundle.json"


def _bundle(p: Path) -> dict:
    return json.loads(p.read_text())


def test_deidentification_strips_phi_before_anything_downstream():
    # Pure (no CLI): the raw bundle carries PHI; de-id removes it and the mapping never sees it.
    raw = _bundle(FEASIBLE)
    clean, report = deidentify.deidentify(raw, as_of_year=2026)
    blob = json.dumps(clean)
    for phi in ("Okafor", "Daniel", "MRN-7720145", "312-555-0148", "Maple Court"):
        assert phi not in blob, f"PHI {phi!r} survived de-identification: leak"
    assert report["removed"], "the de-identification report records what was removed"
    # The clinical signal the COP needs DOES survive (codes/values).
    facts, _ = f2c.to_chartfacts(clean, as_of_year=2026)
    kinds = {f.kind for f in facts}
    assert "age_band" in kinds and "weight" in kinds, kinds


def test_feasible_chart_drives_through_to_a_regimen():
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_run_chart_to_regimen: PASS (de-id+mapping); engine SKIPPED (no cli)")
        return
    res = rc.chart_to_regimen(cli, _bundle(FEASIBLE), as_of_year=2026)
    # A straightforward adult → the standard community empiric regimen is feasible. The min-cost
    # set-cover reports a solved cover as `optimal` (vs `infeasible` for the abstention case).
    assert res["outcome"] == "optimal" and res["regimen"], res
    assert {"vancomycin", "ceftriaxone"} <= set(res["regimen"]), res["regimen"]
    # The full audit trail is assembled on the decision.
    assert res["deidentification"]["removed"], "de-id report present"
    assert any(f["kind"] == "age_band" for f in res["chart_facts"]), res["chart_facts"]
    assert res["answer_time_model_calls"] == 0
    # Time-critical disease → treat-now (CC-5 grounded acuity).
    assert res["timing"]["decision"] == "treat_now_empiric", res["timing"]


def test_complex_chart_drives_through_to_honest_abstention():
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # Penicillin allergy + ESRD + a nephrotoxin: vancomycin is undosable and ampicillin is
    # contraindicated → no regimen covers the organisms safely → INFEASIBLE with a conflict core.
    res = rc.chart_to_regimen(cli, _bundle(INFEASIBLE), as_of_year=2026)
    assert res["outcome"] == "infeasible" and res["regimen"] is None, res
    assert res["conflict"], "an INFEASIBLE drive-through surfaces the minimal conflict set"
    assert "vancomycin" in res["dose_infeasible"], res["dose_infeasible"]
    assert "ampicillin" in res["contraindicated"], res["contraindicated"]
    # Still fully audited + CPU-bound even when it abstains.
    assert res["deidentification"]["removed"]
    assert res["answer_time_model_calls"] == 0


def test_chart_facts_and_discards_are_carried_for_audit():
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # The drive-through carries the mapped facts AND any unmapped resources (with a reason) —
    # the "no unaccounted bytes" discipline applied to the chart.
    res = rc.chart_to_regimen(cli, _bundle(INFEASIBLE), as_of_year=2026)
    assert isinstance(res["chart_facts"], list) and res["chart_facts"]
    assert isinstance(res["chart_discards"], list)  # may be empty, but always present


if __name__ == "__main__":
    test_deidentification_strips_phi_before_anything_downstream()
    test_feasible_chart_drives_through_to_a_regimen()
    test_complex_chart_drives_through_to_honest_abstention()
    test_chart_facts_and_discards_are_carried_for_audit()
    print("all CC-7 drive-through tests passed")
