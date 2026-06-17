#!/usr/bin/env python3
"""timing.py — DERIVE the wait-vs-treat-now decision by running the ADJ precedence ladder.

The runtime half of the CC-5 ADJ-native refactor. `timing.adj` holds the wait-vs-treat
decision as a defeasible-precedence ladder (`functional timing(_)` + `priority:` tiers); this
module RUNS it under a case's culture/clinical status + the disease's acuity and reads the
engine's *governing* answer — replacing the Python if/elif `decide_timing`.

    derive_timing(cli, culture_status="pending", clinical_status="stable",
                  disease_acuity="time_critical")
        → {"decision": "treat_now_empiric", "delay_risk": "high", "standing": "authoritative",
           "governing": True, ...}

The DECISION is the engine's (`? timing($D)`, 0 model calls); `delay_risk` is read off the
*governing tier* — treat-now via the authoritative (time-critical/unstable) rule is high risk,
treat-now via the default fallback is only moderate, await is low, targeted is none. The
reasoning lives in the language; Python only asserts the inputs and maps the verdict to its
human-readable risk label.

SECURITY: the three inputs can originate from a model-decomposed chart, so they are
semi-untrusted and interpolated into an `.adj` program. Each is checked against a closed
vocabulary (`^[a-z][a-z0-9_]*$` + an allow-list) BEFORE it reaches the program text. The temp
program is written beside the rulebook and removed in a `finally` (mirrors contraindications.py
/ step_therapy.py).
"""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
RULEBOOK = HERE / "timing.adj"

_TOKEN_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")
# Closed vocabularies for the three inputs (defence in depth on top of the regex).
_CULTURE = {"pending", "resulted", "unknown"}
_CLINICAL = {"critical", "unstable", "stable", "unknown"}
_ACUITY = {"time_critical", "routine"}

# delay_risk is a property of the governing (decision, tier): treat-now forced by the
# authoritative time-critical/unstable rule is HIGH; the conservative default fallback is only
# MODERATE; awaiting is LOW; a resulted culture moots the wait (NONE).
_DELAY_RISK = {
    ("targeted_culture_directed", None): "none",
    ("await_culture", None): "low",
    ("treat_now_empiric", "authoritative"): "high",
    ("treat_now_empiric", "default"): "moderate",
}


def _safe(token: str, allowed: set[str], kind: str) -> str:
    if not _TOKEN_RE.match(token) or token not in allowed:
        raise ValueError(f"unsafe/unknown {kind} {token!r} (allowed: {sorted(allowed)})")
    return token


def derive_timing(cli: Path, culture_status: str, clinical_status: str,
                  disease_acuity: str) -> dict:
    """Run the timing precedence ladder and return the engine-derived decision + delay_risk.
    Inputs default to the conservative unknowns when a chart did not specify them."""
    culture = _safe(culture_status or "unknown", _CULTURE, "culture_status")
    clinical = _safe(clinical_status or "unknown", _CLINICAL, "clinical_status")
    acuity = _safe(disease_acuity or "routine", _ACUITY, "disease_acuity")

    program = RULEBOOK.read_text() + "\n"
    program += f"relate culture_status({culture})\n"
    program += f"relate clinical_status({clinical})\n"
    program += f"relate disease_acuity({acuity})\n"
    program += "relate case_active(yes)\n"  # always-present marker the default rule gates on
    program += "? timing($D)\n"

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

    # Read the governing answer for the `timing(...)` query.
    decision, standing, has_conflict = None, None, False
    for rec in out.get("governing", []):
        if not rec.get("query", "").startswith("timing("):
            continue
        has_conflict = rec.get("has_conflict", False)
        for ans in rec.get("answers", []):
            if ans.get("status") == "governing":
                decision = ans.get("bindings", {}).get("D")
                standing = ans.get("standing")
    # delay_risk keyed by (decision, tier) — None tier-key for decisions whose risk is fixed.
    risk_key = (decision, standing) if (decision, standing) in _DELAY_RISK else (decision, None)
    delay_risk = _DELAY_RISK.get(risk_key, "moderate")
    return {
        "decision": decision,
        "delay_risk": delay_risk,
        "standing": standing,
        "governing": decision is not None and not has_conflict,
        "culture_status": culture,
        "clinical_status": clinical,
        "disease_acuity": acuity,
    }


if __name__ == "__main__":  # tiny demo
    import sys
    sys.path.insert(0, str(HERE.parent.parent / "warm"))
    import decide as decide_mod  # noqa: E402

    cli = decide_mod.find_cli()
    if cli is None:
        print("timing: adj-lang-cli not built", file=sys.stderr)
        raise SystemExit(3)
    for c, cl, a in [("resulted", "stable", "routine"), ("pending", "stable", "time_critical"),
                     ("pending", "stable", "routine"), ("unknown", "stable", "routine")]:
        print(c, cl, a, "->", derive_timing(cli, c, cl, a))
