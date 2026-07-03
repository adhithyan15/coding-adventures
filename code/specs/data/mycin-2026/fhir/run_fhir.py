#!/usr/bin/env python3
"""run_fhir.py - a FHIR chart export -> diagnosis. Coded charts: 0 model calls.

MYCIN-2026 D1. Runs the warm path off a real chart shape (an HL7 FHIR Bundle):

    FHIR Bundle ──► extract ──► coded findings (0 model calls) ─┐
                            └► narrative text → decompose_text  ─┴► ir_to_adj → decide
                                                  (1 local call, only if needed)

The headline: when the chart's labs/problems are CODED (the common case for an
EHR export), the whole pipeline runs at **0 model calls** - the structured data
maps straight to typed findings and the CPU engine diagnoses. Free-text narrative
(an HPI) still goes through the on-device decomposer. Either way, nothing leaves
the machine. Decision SUPPORT: the differential + the chart's allergies/meds are a
grounded, overridable trail the physician reviews.

Usage:  python3 run_fhir.py samples/meningitis_bundle.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(MYCIN / "warm"))
sys.path.insert(0, str(HERE))
import decide as decide_mod  # noqa: E402
import fhir_ingest as fhir  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402


def chart_to_ir(chart: dict, cli) -> tuple[dict, int]:
    """Build the decomposed IR from a chart. Coded findings are added directly (0
    model calls); a free-text narrative is decomposed on-device (1 call). Returns
    (ir, model_calls_used)."""
    findings = [{"term": t, "type": "stated", "polarity": "affirmed"} for t in chart["findings"]]
    model_calls = 0
    narrative = chart.get("narrative", "").strip()
    if narrative:
        try:
            import decomposer as dc
            _, gen = dc.select_backend()
            nir = dc.decompose_text(narrative, gen=gen)
            findings += nir.get("findings", [])
            model_calls = 1
        except Exception as e:  # noqa: BLE001 - no backend -> coded findings only
            print(f"  (narrative not decomposed: {e}; using coded findings only)", file=sys.stderr)
    return {"case_id": "fhir", "findings": findings, "discard": [],
            "inference_justifications": []}, model_calls


def run(bundle_path: str) -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("run_fhir: adj-lang-cli not built", file=sys.stderr)
        return 3
    bundle = json.loads(Path(bundle_path).read_text())
    chart = fhir.extract(bundle)

    bar = "=" * 78
    print(bar)
    print(f"FHIR CHART: {Path(bundle_path).name}  (gender={chart['demographics'].get('gender')})")
    print(bar)
    print("\n[1] CODED FINDINGS (deterministic from LOINC/SNOMED, 0 model calls):")
    print(f"    {', '.join(chart['findings']) or '(none coded)'}")
    if chart["unmapped"]:
        print(f"    unmapped coded resources (surfaced, not guessed): {chart['unmapped']}")
    if chart["narrative"]:
        print(f"    free-text narrative -> on-device decompose: {chart['narrative'][:80]}...")
    if chart["allergies"]:
        print(f"    ALLERGIES (carry to therapy): {chart['allergies']}")
    if chart["medications"]:
        print(f"    current meds: {chart['medications']}")

    ir, model_calls = chart_to_ir(chart, cli)
    domains = ir_mod.load_domains()
    observe_adj, kept, dropped = ir_mod.ir_to_adj(ir, domains)
    if not kept:
        print("\n[2] No findings mapped -> the engine abstains.")
        return 0
    res = decide_mod.decide("fhir", observe_adj, cli)
    print(f"\n[2] DIFFERENTIAL (decompose model calls: {model_calls}; answer-time: 0):")
    for hyp, p in sorted(res["posteriors"].items(), key=lambda kv: -kv[1]):
        lead = "  <- leading" if hyp == res["leader"] else ""
        print(f"    {hyp:24s} P = {p:.4f}{lead}")
    print(f"    decision: {res['decision'].get('type')}")

    print("\n" + bar)
    print("PHYSICIAN REVIEW - grounded + overridable; you make the call.")
    print(f"total model calls: {model_calls}   |   chart data left the machine: none")
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: run_fhir.py <bundle.json>", file=sys.stderr)
        return 2
    return run(argv[0])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
