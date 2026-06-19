#!/usr/bin/env python3
"""run_chart_to_regimen.py — a raw FHIR chart → an antibiotic regimen + full audit trail.

MYCIN-2026 CC-7 (the "full chart drive-through", CHART-AS-CONSTRAINTS §7). This is the
TREATMENT counterpart to run_fhir.py (which drives a chart → *diagnosis*). It wires the three
already-built, already-tested stages into one end-to-end call:

    raw FHIR Bundle ──► deidentify ──► to_chartfacts ──► chart_to_cop.derive ──► regimen + audit
       (PHI in)         (HIPAA          (LOINC/SNOMED      (constraint program,
                         Safe Harbor)    → ChartFacts)       solved on the CPU)

The headline, end to end: a whole de-identified FHIR chart produces a treatment decision —
the regimen, OR an honest INFEASIBLE with the conflict set — at **0 answer-time model calls**
(every stage is deterministic: Safe-Harbor de-id, coded-fact mapping, and the constraint
solver). Nothing leaves the machine, and the result carries a *full audit trail*: what PHI was
removed, which chart facts mapped vs were discarded, every constraint with its grounded
provenance, and the wait-vs-treat / reimbursement decisions. Decision SUPPORT — the physician
reviews and overrides any grounded constraint.

The pieces already compose (no type translation): `deidentify` returns a clean Bundle dict,
`to_chartfacts` consumes it and returns `ChartFact`s, `derive` consumes those and returns the
decision. CC-7 is the orchestrator that joins them and assembles the audit trail, plus the
end-to-end drive-through tests.

Usage:  python3 run_chart_to_regimen.py samples/chart_with_phi_bundle.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "treatment" / "antibiotics"))
sys.path.insert(0, str(MYCIN / "warm"))
import chart_to_cop as cc  # noqa: E402  (derive: ChartFacts → regimen + provenance)
import decide as decide_mod  # noqa: E402  (find_cli)
import deidentify  # noqa: E402  (HIPAA Safe-Harbor de-identification)
import fhir_to_chartfacts as f2c  # noqa: E402  (de-identified Bundle → ChartFacts)


def chart_to_regimen(cli: Path, fhir_bundle: dict, disease: str = "meningitis",
                     as_of_year: int | None = None) -> dict:
    """End-to-end CC-7: a raw FHIR Bundle → a regimen decision + a full audit trail.

    Stages (all deterministic; 0 answer-time model calls):
      1. de-identify the bundle (HIPAA Safe Harbor) — strips PHI, keeps clinical codes/values;
      2. map the de-identified bundle → ChartFacts (LOINC/SNOMED lookups), recording discards;
      3. compile the ChartFacts → constraint program and solve → regimen OR INFEASIBLE(conflict).

    The returned dict is `chart_to_cop.derive`'s decision PLUS the upstream audit layers:
      - `deidentification`: the Safe-Harbor report (what classes of PHI were removed, dates
        generalized, ages capped, free-text labels kept);
      - `chart_discards`: chart resources that mapped to no constraint, each with a reason
        (the "no unaccounted bytes" discipline applied to the chart);
      - `answer_time_model_calls`: 0 (the whole drive-through is CPU-bound).
    `derive` itself supplies `regimen`/`outcome`/`conflict`/`timing`/`reimbursement` plus the
    per-constraint provenance trail."""
    clean_bundle, deident_report = deidentify.deidentify(fhir_bundle, as_of_year)
    facts, chart_discards = f2c.to_chartfacts(clean_bundle, as_of_year)
    decision = cc.derive(cli, facts, disease)
    # Assemble the full audit trail: the upstream de-id + mapping layers wrap the COP decision.
    decision["deidentification"] = deident_report
    decision["chart_discards"] = chart_discards
    decision["chart_facts"] = [{"kind": f.kind, "value": f.value, "span": f.span} for f in facts]
    decision["answer_time_model_calls"] = 0
    return decision


def _print_drivethrough(name: str, disease: str, res: dict) -> None:
    """Human-readable drive-through (presentation only; the data is `res`)."""
    bar = "=" * 78
    print(bar)
    print(f"FHIR CHART → REGIMEN: {name}   (disease={disease})")
    print(bar)
    dr = res["deidentification"]
    removed = ", ".join(f"{k}×{v}" for k, v in sorted(dr.get("removed", {}).items())) or "(none)"
    print("\n[1] DE-IDENTIFICATION (HIPAA Safe Harbor, one-way, 0 model calls):")
    print(f"    PHI removed: {removed}")
    print(f"    dates→year: {dr.get('dates_generalized', 0)}   age-capped: {dr.get('age_capped', 0)}"
          f"   free-text labels kept (audited): {dr.get('free_text_kept', 0)}")
    print("\n[2] CHART FACTS (deterministic LOINC/SNOMED mapping, 0 model calls):")
    for f in res["chart_facts"]:
        print(f"    {f['kind']:16s} = {f['value']:24s}  ⟵ {f['span']}")
    for d in res["chart_discards"]:
        print(f"    DISCARD: {d['fact']}  ({d['reason']})")
    print("\n[3] TREATMENT DECISION (constraint solver on the CPU, 0 model calls):")
    if res["regimen"]:
        print(f"    REGIMEN: {', '.join(res['regimen'])}   (cost {res.get('cost')})")
    else:
        print(f"    INFEASIBLE — honest abstention. conflict core: {res.get('conflict')}")
        print(f"    (dose-infeasible: {res.get('dose_infeasible')}; "
              f"contraindicated: {res.get('contraindicated')}; exclusions: {res.get('exclusions')})")
    t = res.get("timing") or {}
    print(f"    timing: {t.get('decision')} (delay_risk {t.get('delay_risk')})")
    if res.get("reimbursement"):
        rb = res["reimbursement"]
        print(f"    reimbursement: covered={rb.get('covered_regimen')} — {rb.get('note')}")
    print("\n" + bar)
    print("PHYSICIAN REVIEW — grounded + overridable; you make the call.")
    print(f"answer-time model calls: {res['answer_time_model_calls']}   |   "
          "chart data left the machine: none")


def run(bundle_path: str, disease: str = "meningitis", as_of_year: int = 2026) -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("run_chart_to_regimen: adj-lang-cli not built", file=sys.stderr)
        return 3
    bundle = json.loads(Path(bundle_path).read_text())
    res = chart_to_regimen(cli, bundle, disease=disease, as_of_year=as_of_year)
    _print_drivethrough(Path(bundle_path).name, disease, res)
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: run_chart_to_regimen.py <bundle.json> [disease] [as_of_year]", file=sys.stderr)
        return 2
    disease = argv[1] if len(argv) > 1 else "meningitis"
    as_of = int(argv[2]) if len(argv) > 2 else 2026
    return run(argv[0], disease, as_of)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
