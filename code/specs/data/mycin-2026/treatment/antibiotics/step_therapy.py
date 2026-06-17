#!/usr/bin/env python3
"""step_therapy.py — DERIVE the reimbursement-blocked drugs by running the ADJ rule.

The runtime half of the step-therapy ADJ-native refactor.  `step_therapy.adj` holds the
durable, domain-neutral precedence rule (negation-as-failure); this module RUNS it under a
patient's per-case payer facts and returns which drugs the payer will not reimburse.

    derive_blocked(cli,
                   step_therapy={("cefepime", "meropenem")},   # (restricted Y, prerequisite X)
                   tried={"vancomycin"})                        # drugs already tried/failed
        → {"cefepime"}        # cefepime needs meropenem tried first; it has not been → blocked

This replaces chart_to_cop's Python `reimbursement_blocked()` set-difference: the precedence
`x_Y ≤ tried_X` is now derived by the ENGINE via the rule
`reimbursement_blocked($Y) when: requires_prerequisite($Y,$X), not already_tried($X)` — 0
model calls, pure SLD + negation-as-failure over the per-case facts.

SECURITY (trust boundary).  Drug tokens can originate from a model-decomposed chart, so they
are semi-untrusted and are interpolated into an `.adj` program we then execute.  Every token
is checked against `^[a-z][a-z0-9_]*$` BEFORE it reaches the program text (closes off `.adj`
clause injection).  The temp program is written beside the rulebook and removed in a
`finally`.  (Mirrors contraindications.py / warm/decide.py.)
"""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
RULEBOOK = HERE / "step_therapy.adj"

# A drug token is a closed-vocabulary symbol — anything else is refused before it can reach
# the generated program (injection guard).
DRUG_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")


def _safe_drug(d: str) -> str:
    if not DRUG_RE.match(d):
        raise ValueError(f"unsafe drug token {d!r} (must match {DRUG_RE.pattern})")
    return d


def derive_blocked(cli: Path, step_therapy, tried) -> set[str]:
    """Return the set of drugs the payer will not reimburse because a step-therapy
    prerequisite is unmet, DERIVED by the engine from `step_therapy` (an iterable of
    (restricted, prerequisite) pairs) and `tried` (drugs already tried).  An empty
    `step_therapy` short-circuits to the empty set without invoking the engine."""
    pairs = sorted({(_safe_drug(y), _safe_drug(x)) for y, x in step_therapy})
    if not pairs:
        return set()
    done = sorted({_safe_drug(d) for d in tried})

    program = RULEBOOK.read_text() + "\n"
    program += "".join(f"relate requires_prerequisite({y}, {x})\n" for y, x in pairs)
    program += "".join(f"relate already_tried({d})\n" for d in done)
    program += "? reimbursement_blocked($Y)\n"

    tmp = tempfile.NamedTemporaryFile("w", suffix=".adj", dir=HERE, delete=False)
    try:
        tmp.write(program)
        tmp.close()
        r = subprocess.run([str(cli), tmp.name], capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"adj-lang-cli exited {r.returncode}: {r.stderr}")
        out = json.loads(r.stdout)
    finally:
        Path(tmp.name).unlink(missing_ok=True)

    blocked: set[str] = set()
    for rec in out.get("recall", []):
        if not rec.get("query", "").startswith("reimbursement_blocked("):
            continue
        for ans in rec.get("answers", []):
            y = ans.get("bindings", {}).get("Y")
            if y:
                blocked.add(y)
    return blocked


if __name__ == "__main__":  # tiny demo
    import sys
    sys.path.insert(0, str(HERE.parent.parent / "warm"))
    import decide as decide_mod  # noqa: E402

    cli = decide_mod.find_cli()
    if cli is None:
        print("step_therapy: adj-lang-cli not built", file=sys.stderr)
        raise SystemExit(3)
    print(sorted(derive_blocked(cli, {("cefepime", "meropenem"), ("linezolid", "vancomycin")},
                                {"vancomycin"})))
