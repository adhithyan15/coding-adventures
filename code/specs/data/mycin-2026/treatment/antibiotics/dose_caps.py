#!/usr/bin/env python3
"""dose_caps.py — DERIVE conjunctive dose caps by running the ADJ rulebook.

This is the runtime half of the ADJ-native dose-cap refactor (CC-2b).  Where
`dose_caps_build.py` GENERATES the grounded rulebook (`dose_caps.adj`), this module RUNS it:
given the patient's active risk tokens (read off the chart — `hepatic_severe`,
`renal_moderate`, …), it asks the engine which COMPOUND risks hold and which drugs are
dose-capped, returning both with their grounding.

    derive_dose_caps(cli, {"hepatic_severe", "renal_moderate"})
        → (
            {"hepatorenal"},                                   # derived compound risks
            {"ceftriaxone": {"risk": "hepatorenal",            # capped drugs + grounding
                             "source": "<FDA byte-quote>",
                             "locator": "https://…", "trust": "authoritative"}}
          )

The conjunction is the ENGINE'S, not Python's: we append the patient's `active_risk` facts and
the binding queries `? derived_risk($R)` / `? dose_capped($D, $R)` to the rulebook, run
`adj-lang-cli`, and read the bindings.  Zero model calls — pure SLD over the grounded graph.
A single active risk (hepatic only, or renal only) derives NOTHING, faithfully reflecting that
hepatic impairment alone needs no dose adjustment.

SECURITY (trust boundary).  A risk token can originate from a model-decomposed chart, so it is
semi-untrusted and is interpolated into an `.adj` program we then execute.  We reject any token
that is not a single lower-snake-case word (`^[a-z][a-z0-9_]*$`) BEFORE it reaches the program
text — this closes off `.adj` injection (a token like `renal_moderate)\n? evil(` could
otherwise smuggle clauses).  The temp program is written inside this directory and removed in a
`finally`.  (Mirrors contraindications.py's CONTEXT_RE discipline.)
"""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
RULEBOOK = HERE / "dose_caps.adj"

# A risk token is a closed-vocabulary symbol.  Anything else is refused before it can reach the
# generated program (injection guard) — see the module docstring.
RISK_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")


def _safe_risk(tok: str) -> str:
    if not RISK_RE.match(tok):
        raise ValueError(f"unsafe risk token {tok!r} (must match {RISK_RE.pattern})")
    return tok


def derive_dose_caps(cli: Path, active_risks) -> tuple[set[str], dict[str, dict]]:
    """Run the dose-cap rulebook under the patient's `active_risks` and return
    (derived_risks, caps):
      * derived_risks — the set of COMPOUND risk tokens the engine derives as holding
        (e.g. {"hepatorenal"}); fold these into the COP's risk set so the dose-window
        penalty fires.
      * caps — {drug: {risk, source, locator, trust}} for every drug the engine derives as
        dose-capped, each carrying the grounded byte-quote from the cap fact it joined.
    An empty/falsy `active_risks` short-circuits to (set(), {}) without invoking the engine."""
    toks = sorted({_safe_risk(r) for r in active_risks})
    if not toks:
        return set(), {}

    program = RULEBOOK.read_text() + "\n"
    program += "".join(f"relate active_risk({t})\n" for t in toks)
    program += "? derived_risk($R)\n"
    program += "? dose_capped($D, $R)\n"

    # Write the case program beside the rulebook (a tempfile in HERE), run, then remove it.
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

    derived_risks: set[str] = set()
    caps: dict[str, dict] = {}
    for rec in out.get("recall", []):
        q = rec.get("query", "")
        if q.startswith("derived_risk("):
            for ans in rec.get("answers", []):
                r_tok = ans.get("bindings", {}).get("R")
                if r_tok:
                    derived_risks.add(r_tok)
        elif q.startswith("dose_capped("):
            for ans in rec.get("answers", []):
                b = ans.get("bindings", {})
                drug, risk = b.get("D"), b.get("R")
                if not drug or not risk:
                    continue
                # The grounding lives on the dose_capped_under fact the derivation joined;
                # surface the first cited clause that carries a source (the grounded quote).
                cite = next((c for c in ans.get("citations", []) if c.get("source")), {})
                caps.setdefault(drug, {"risk": risk, "source": cite.get("source", ""),
                                       "locator": cite.get("locator"),
                                       "trust": cite.get("trust", "unattributed")})
    return derived_risks, caps


if __name__ == "__main__":  # tiny demo
    import sys
    sys.path.insert(0, str(HERE.parent.parent / "warm"))
    import decide as decide_mod  # noqa: E402

    cli = decide_mod.find_cli()
    if cli is None:
        print("dose_caps: adj-lang-cli not built", file=sys.stderr)
        raise SystemExit(3)
    risks_dr, caps = derive_dose_caps(cli, {"hepatic_severe", "renal_moderate"})
    print(json.dumps({"derived_risks": sorted(risks_dr), "caps": caps}, indent=2, ensure_ascii=False))
