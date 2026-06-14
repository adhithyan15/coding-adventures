"""Analyze an ADJ52 pipeline run: cross-tabs of correctness vs blind-judge wins,
and the framework's posterior-saturation distribution.

Usage: python analyze_run.py <run-output.json> [summary-out.json]

The input is the Workflow result JSON ({"result": {"tally", "per_case"}}) or the
bare result object. Emits headline cross-tabs to stdout and, if a second path is
given, a trimmed per-case summary JSON (dropping the verbose perturbations and
rationale text) for committing alongside the writeup.
"""

from __future__ import annotations

import json
import re
import statistics
import sys
from typing import Any


def load_per_case(path: str) -> list[dict[str, Any]]:
    with open(path, encoding="utf-8") as fh:
        doc: dict[str, Any] = json.load(fh)
    result = doc.get("result", doc)
    per_case: list[dict[str, Any]] = result.get("per_case", [])
    return per_case


def posterior_of(fw_top: str) -> float | None:
    """fw_top looks like 'diagnosis(x) @ 0.9967'; pull the trailing float."""
    m = re.search(r"@\s*([0-9]*\.?[0-9]+)", fw_top or "")
    return float(m.group(1)) if m else None


def main() -> None:
    if len(sys.argv) < 2:
        print("usage: python analyze_run.py <run-output.json> [summary-out.json]")
        sys.exit(2)
    per_case = load_per_case(sys.argv[1])
    n = len(per_case)

    def correct(v: Any) -> bool:
        return v == "correct"

    both_correct = sum(1 for r in per_case if correct(r.get("framework_correct")) and correct(r.get("plain_correct")))
    only_fw = sum(1 for r in per_case if correct(r.get("framework_correct")) and not correct(r.get("plain_correct")))
    only_plain = sum(1 for r in per_case if not correct(r.get("framework_correct")) and correct(r.get("plain_correct")))
    neither = sum(1 for r in per_case if not correct(r.get("framework_correct")) and not correct(r.get("plain_correct")))

    fw_won = sum(1 for r in per_case if r.get("framework_won"))
    plain_won = sum(1 for r in per_case if r.get("plain_won"))
    tie = sum(1 for r in per_case if r.get("winner") == "tie")

    # The framework's value niche: it won AND it was correct where plain was not.
    fw_won_and_caught_error = sum(
        1 for r in per_case
        if r.get("framework_won") and correct(r.get("framework_correct")) and not correct(r.get("plain_correct"))
    )
    # Cases both correct yet plain still won (lost on calibration/defensibility, not correctness).
    lost_while_both_correct = sum(
        1 for r in per_case
        if r.get("plain_won") and correct(r.get("framework_correct")) and correct(r.get("plain_correct"))
    )

    posteriors = [p for r in per_case if (p := posterior_of(r.get("fw_top", ""))) is not None]
    sat_99 = sum(1 for p in posteriors if p >= 0.99)
    sat_999 = sum(1 for p in posteriors if p >= 0.999)
    median_post = statistics.median(posteriors) if posteriors else float("nan")

    print(f"cases: {n}")
    print(f"correctness cross-tab: both={both_correct} only_fw={only_fw} only_plain={only_plain} neither={neither}")
    print(f"win cross-tab: framework={fw_won} plain={plain_won} tie={tie}")
    print(f"framework won AND was correct where plain was wrong (the niche): {fw_won_and_caught_error}")
    print(f"plain won while BOTH were correct (framework lost on calibration, not correctness): {lost_while_both_correct}")
    print(f"framework top-posterior saturation: >=0.99: {sat_99}/{len(posteriors)}  >=0.999: {sat_999}/{len(posteriors)}  median: {median_post:.4f}")

    if len(sys.argv) >= 3:
        summary = [
            {
                "id": r.get("id"),
                "domain": r.get("fw_domain"),
                "fw_top": r.get("fw_top"),
                "fw_next_step": (r.get("fw_next_step") or "")[:120],
                "winner": r.get("winner"),
                "framework_correct": r.get("framework_correct"),
                "plain_correct": r.get("plain_correct"),
                "diagnosis_unchanged": r.get("diagnosis_unchanged"),
            }
            for r in per_case
        ]
        with open(sys.argv[2], "w", encoding="utf-8") as fh:
            json.dump(summary, fh, indent=2)
        print(f"wrote trimmed summary: {sys.argv[2]} ({len(summary)} cases)")


if __name__ == "__main__":
    main()
