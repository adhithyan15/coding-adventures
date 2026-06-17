#!/usr/bin/env python3
"""contraindications.py — DERIVE the contraindicated drugs by running the ADJ rulebook.

This is the runtime half of the ADJ-native contraindication refactor.  Where
`contraindications_build.py` GENERATES the grounded rulebook (`contraindications.adj`), this
module RUNS it: given the patient's active clinical contexts (read off the chart), it asks
the engine which drugs are contraindicated and returns the answer with its grounding.

    derive_contraindications(cli, {"pregnancy"})
        → {"moxifloxacin": {"context": "pregnancy", "source": "<FDA byte-quote>",
                            "locator": "https://…", "trust": "authoritative"},
           "tmp_smx":      {"context": "pregnancy", "source": "…", …}}

The reasoning is the ENGINE'S, not Python's: we append the patient's `active_context` facts
and the binding query `? contraindicated($D, $C)` to the rulebook, run `adj-lang-cli`, and
read the bindings.  Zero model calls — pure SLD over the grounded knowledge graph.

SECURITY (trust boundary).  A context token can originate from a model-decomposed chart, so
it is semi-untrusted and is interpolated into an `.adj` program we then execute.  We reject
any token that is not a single lower-snake-case word (`^[a-z][a-z0-9_]*$`) BEFORE it reaches
the program text — this closes off `.adj` injection (a token like `pregnancy)\n? evil(` could
otherwise smuggle clauses).  The temp program is written inside this directory and removed
in a `finally`.  (Mirrors warm/decide.py's CASE_ID_RE discipline.)
"""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
RULEBOOK = HERE / "contraindications.adj"

# A context token is a closed-vocabulary symbol.  Anything else is refused before it can
# reach the generated program (injection guard) — see the module docstring.
CONTEXT_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")


def _safe_context(ctx: str) -> str:
    if not CONTEXT_RE.match(ctx):
        raise ValueError(f"unsafe context token {ctx!r} (must match {CONTEXT_RE.pattern})")
    return ctx


def derive_contraindications(cli: Path, contexts) -> dict[str, dict]:
    """Run the contraindication rulebook under the patient's active `contexts` and return
    {drug: {context, source, locator, trust}} for every drug the engine derives as
    contraindicated.  An empty/falsy `contexts` short-circuits to {} without invoking the
    engine (no active context → nothing contraindicated)."""
    ctx = sorted({_safe_context(c) for c in contexts})
    if not ctx:
        return {}

    program = RULEBOOK.read_text() + "\n"
    program += "".join(f"relate active_context({c})\n" for c in ctx)
    program += "? contraindicated($D, $C)\n"

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

    derived: dict[str, dict] = {}
    for rec in out.get("recall", []):
        if not rec.get("query", "").startswith("contraindicated("):
            continue
        for ans in rec.get("answers", []):
            b = ans.get("bindings", {})
            drug, context = b.get("D"), b.get("C")
            if not drug or not context:
                continue
            # The grounding lives on the contraindication fact the derivation joined; surface
            # the first cited clause that carries a source (the grounded byte-quote).
            cite = next((c for c in ans.get("citations", []) if c.get("source")), {})
            # Keep the strongest (first) provenance if a drug is hit by multiple contexts.
            derived.setdefault(drug, {"context": context, "source": cite.get("source", ""),
                                      "locator": cite.get("locator"),
                                      "trust": cite.get("trust", "unattributed")})
    return derived


if __name__ == "__main__":  # tiny demo
    import sys
    sys.path.insert(0, str(HERE.parent.parent / "warm"))
    import decide as decide_mod  # noqa: E402

    cli = decide_mod.find_cli()
    if cli is None:
        print("contraindications: adj-lang-cli not built", file=sys.stderr)
        raise SystemExit(3)
    print(json.dumps(derive_contraindications(cli, {"pregnancy"}), indent=2, ensure_ascii=False))
