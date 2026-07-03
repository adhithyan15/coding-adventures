#!/usr/bin/env python3
"""Reproduce the CAS edit-override-propagate-regression loop end to end.

Demonstrates "fix the fact, not the weight" on the real ADJ55 bacterial-meningitis
corpus and ADJ56's documented CSF over-saturation: a human overrides the
correlated CSF-chemistry claims, the over-confident case de-saturates, and a
regression check confirms the dispositive (culture-positive) case is unchanged.

Run: python demo.py
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CORPUS = HERE.parent / "corpus" / "bacterial_meningitis" / "corpus.json"
EVAL = HERE.parent / "corpus" / "eval.py"
OVERRIDES = HERE / "overrides" / "meningitis-csf-correlation.json"
EFFECTIVE = HERE / "effective-meningitis.json"
CASES = {
    "pre-culture (Gram/culture pending)": HERE / "cases" / "meningitis-preculture.json",
    "culture-positive (regression check)": HERE.parent / "provenance" / "meningitis" / "case.json",
}


def final_p(corpus: Path, case: Path) -> float:
    out = subprocess.run(
        [sys.executable, str(EVAL), str(corpus), str(case), "grounded"],
        capture_output=True, text=True,
    ).stdout
    m = re.search(r">>> P\(.*\) = ([0-9.]+)", out)
    return float(m.group(1)) if m else float("nan")


def main() -> None:
    print("STEP 1 — apply the human override (edit the correlated CSF-chemistry facts):")
    subprocess.run([sys.executable, str(HERE / "override.py"), str(CORPUS), str(OVERRIDES), str(EFFECTIVE)])
    print("\nSTEP 2 — re-run + regression check (base corpus vs human-edited effective corpus):\n")
    print(f"  {'case':38s} {'base P':>8s} {'edited P':>9s}   outcome")
    for name, case in CASES.items():
        b = final_p(CORPUS, case)
        e = final_p(EFFECTIVE, case)
        outcome = "DE-SATURATED (false-certainty fixed)" if e < b - 0.05 else "unchanged (no regression)"
        print(f"  {name:38s} {b:8.4f} {e:9.4f}   {outcome}")
    print("\nThe base corpus is immutable; the edit lives in overrides/ (versioned, attributed,")
    print("cited) and in the effective corpus's per-node provenance.override. Fix the fact, not the weight.")


if __name__ == "__main__":
    main()
