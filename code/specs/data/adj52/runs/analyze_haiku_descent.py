"""Cross-tabs for the Haiku-descent run (blind-Haiku vs framework-no-engine-Haiku).

Usage: python analyze_haiku_descent.py <run-output.json> [summary-out.json]
Emits headline cross-tabs to stdout; if a second path is given, writes a trimmed
per-case summary (dropping the large prose/ir/rulebook/conclusion blobs).
"""

from __future__ import annotations

import json
import sys
from typing import Any


def load_per_case(path: str) -> list[dict[str, Any]]:
    with open(path, encoding="utf-8") as fh:
        doc: dict[str, Any] = json.load(fh)
    result = doc.get("result", doc)
    return result.get("per_case", [])


def main() -> None:
    if len(sys.argv) < 2:
        print("usage: python analyze_haiku_descent.py <run-output.json> [summary-out.json]")
        sys.exit(2)
    pc = load_per_case(sys.argv[1])
    n = len(pc)

    def c(v: Any) -> bool:
        return v == "correct"

    fw_correct = sum(1 for r in pc if c(r.get("framework_correct")))
    bl_correct = sum(1 for r in pc if c(r.get("blind_correct")))
    both = sum(1 for r in pc if c(r.get("framework_correct")) and c(r.get("blind_correct")))
    only_fw = sum(1 for r in pc if c(r.get("framework_correct")) and not c(r.get("blind_correct")))
    only_bl = sum(1 for r in pc if not c(r.get("framework_correct")) and c(r.get("blind_correct")))
    neither = sum(1 for r in pc if not c(r.get("framework_correct")) and not c(r.get("blind_correct")))

    fw_won = sum(1 for r in pc if r.get("framework_won"))
    bl_won = sum(1 for r in pc if r.get("blind_won"))
    tie = sum(1 for r in pc if r.get("winner") == "tie")

    fw_won_correct = sum(1 for r in pc if r.get("framework_won") and c(r.get("framework_correct")))
    fw_won_neither = sum(1 for r in pc if r.get("framework_won") and not c(r.get("framework_correct")) and not c(r.get("blind_correct")))
    fw_won_but_blind_was_right = sum(1 for r in pc if r.get("framework_won") and not c(r.get("framework_correct")) and c(r.get("blind_correct")))
    bl_won_but_fw_was_right = sum(1 for r in pc if r.get("blind_won") and not c(r.get("blind_correct")) and c(r.get("framework_correct")))

    dx_unchanged = sum(1 for r in pc if r.get("diagnosis_unchanged"))

    print(f"cases: {n}")
    print(f"correctness: framework {fw_correct} vs blind {bl_correct}  (both {both}, only_framework {only_fw}, only_blind {only_bl}, neither {neither})")
    print(f"wins: framework {fw_won}, blind {bl_won}, tie {tie}")
    print(f"  framework won AND was correct: {fw_won_correct}")
    print(f"  framework won while NEITHER was correct (defensibility win on a missed case): {fw_won_neither}")
    print(f"  framework won but BLIND was the correct one (judge preferred wrong-but-defensible): {fw_won_but_blind_was_right}")
    print(f"  blind won but FRAMEWORK was the correct one: {bl_won_but_fw_was_right}")
    print(f"perturbation integrity: diagnosis_unchanged {dx_unchanged}/{n}")

    if len(sys.argv) >= 3:
        summary = [
            {
                "id": r.get("id"),
                "source_url": r.get("source_url"),
                "framework_top": r.get("framework_top"),
                "framework_next_step": (r.get("framework_next_step") or "")[:140],
                "framework_confidence": r.get("framework_confidence"),
                "winner": r.get("winner"),
                "framework_correct": r.get("framework_correct"),
                "blind_correct": r.get("blind_correct"),
                "diagnosis_unchanged": r.get("diagnosis_unchanged"),
            }
            for r in pc
        ]
        with open(sys.argv[2], "w", encoding="utf-8") as fh:
            json.dump(summary, fh, indent=2)
        print(f"wrote trimmed summary: {sys.argv[2]} ({len(summary)} cases)")


if __name__ == "__main__":
    main()
