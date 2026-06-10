#!/usr/bin/env python3
"""ADJ101 — provenance-gated program executor (the deterministic core of the program-emission track).

The framework arm, on a computational item, does NOT compute in the model's forward pass. It emits:

  emission = {
    "facts":   { name: {"magnitude": <num>, "unit": <str>, "polarity": "affirmed"|"denied",
                        "type": "stated"|"inferred", "span": <verbatim source bytes>,
                        "basis_span": <str>, "entailment": "ENTAILED"|"LEAP"} , ... },
    "discarded": [ {"span": <source bytes>, "reason": <why this quantity is irrelevant>} , ... ],
    "program": "<python that computes RESULT using ONLY facts[...] values + tool libraries>"
  }

This module enforces byte-provenance on the program's inputs and runs it deterministically:

  1. JUSTIFICATION gate — every fact is stated(span) or inferred(basis_span + ENTAILED). A fact with
     no provenance is a fabrication. (No fact inferred/used without justification.)
  2. COVERAGE gate — every quantity-bearing span in the SOURCE is either represented by a fact (its
     span) or listed in `discarded(reason)`. A source quantity that is neither is a silent drop.
     (No fact dropped without justification.)
  3. NO-MAGIC-NUMBERS — the emitted program may not hard-code numeric quantities; every numeric input
     must arrive via facts[...]. Small structural constants (0,1,2,10,100, etc.) are allowed; any other
     literal is an un-provenanced input. (Closes the "launder a number as a code literal" hole.)
  4. EXECUTE — run the program in a subprocess with a timeout, `facts` injected, capture RESULT.

NOTE on sandboxing: the program is model-emitted code. We run it in a separate process with a wall
timeout. In this benchmark the emitter is our own model-under-test on math/chem problems (not an
adversary), so process isolation + timeout is the appropriate guard; a hostile setting would need a
seccomp/container sandbox. We do not pass network creds or args to the child.
"""
from __future__ import annotations

import ast
import json
import os
import re
import subprocess
import sys
import tempfile

# Structural constants a program may use without provenance (exponents, bases, rounding, etc.).
ALLOWED_LITERALS = {0, 1, 2, 3, 4, 5, 10, 100, 1000, 60, 360, 3600, 24, 12, 0.5, 2.0}


def _norm(s: str) -> str:
    return re.sub(r"\s+", " ", (s or "")).strip().lower()


def check_justification(facts: dict) -> list:
    """Return the list of fact names whose provenance is missing (fabrications)."""
    bad = []
    for name, f in facts.items():
        typ = f.get("type")
        if typ == "stated" and f.get("span"):
            continue
        if typ == "inferred" and f.get("basis_span") and f.get("entailment") == "ENTAILED":
            continue
        bad.append(name)
    return bad


# A quantity in the source = a number (optionally with a unit/word). Coarse but effective for coverage.
_QTY = re.compile(r"-?\d[\d,]*(?:\.\d+)?")


def source_quantities(source: str) -> list:
    """Distinct numeric tokens appearing in the source text (the things that must be accounted for)."""
    return sorted({m.group(0).replace(",", "") for m in _QTY.finditer(source or "")})


def check_coverage(quantity_spans: list, facts: dict, discarded: list) -> list:
    """Declared source quantity PHRASES neither represented by a fact nor explicitly discarded.

    Span/phrase-based (not bare-number): the item declares the quantity-bearing phrases the solver
    must account for; each must be matched by a fact's span/basis (used) or a discarded span. This is
    robust to unit-exponent noise (the '2' in 'm/s^2' is not a quantity) and makes the benchmark's
    coverage expectation explicit and auditable.
    """
    used = [_norm(f.get("span") or f.get("basis_span") or "") for f in facts.values()]
    dropped = [_norm(d.get("span") or "") for d in (discarded or [])]
    accounted = [s for s in used + dropped if s]
    missing = []
    for span in (quantity_spans or []):
        s = _norm(span)
        if not any(s in a or a in s for a in accounted):
            missing.append(span)
    return missing


def magic_numbers(program: str) -> list:
    """Numeric literals in the program that are not allowed structural constants -> un-provenanced inputs."""
    out = []
    try:
        tree = ast.parse(program)
    except SyntaxError:
        return ["<syntax-error>"]
    for node in ast.walk(tree):
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)) and not isinstance(node.value, bool):
            if node.value not in ALLOWED_LITERALS:
                out.append(node.value)
    return out


def execute(program: str, facts: dict, timeout: float = 8.0) -> dict:
    """Run the emitted program in a subprocess with `facts` injected; capture RESULT."""
    harness = (
        "import json, sys, math\n"
        "facts = json.loads(sys.argv[1])\n"
        "RESULT = None\n"
        + program + "\n"
        "print(json.dumps({'RESULT': RESULT}))\n"
    )
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as fh:
        fh.write(harness)
        path = fh.name
    try:
        proc = subprocess.run(
            [sys.executable, path, json.dumps(facts)],
            capture_output=True, text=True, timeout=timeout,
            env={"PATH": os.environ.get("PATH", "")},  # no inherited creds/tokens
        )
        if proc.returncode != 0:
            return {"exec_ok": False, "result": None, "stderr": proc.stderr.strip()[:500]}
        last = [ln for ln in proc.stdout.splitlines() if ln.strip()][-1]
        return {"exec_ok": True, "result": json.loads(last)["RESULT"], "stderr": ""}
    except subprocess.TimeoutExpired:
        return {"exec_ok": False, "result": None, "stderr": "timeout"}
    except Exception as e:  # noqa: BLE001 — surface any harness failure as a typed result
        return {"exec_ok": False, "result": None, "stderr": f"{type(e).__name__}: {e}"[:500]}
    finally:
        os.unlink(path)


def adjudicate_program(quantity_spans: list, emission: dict, gold=None, tolerance: float = 0.0) -> dict:
    """Full gate + execute. Returns the auditable verdict for one computational item.

    quantity_spans: the item's declared quantity-bearing phrases (used or discarded must cover them).
    """
    facts = emission.get("facts", {})
    fabrications = check_justification(facts)
    missing_coverage = check_coverage(quantity_spans, facts, emission.get("discarded", []))
    magic = magic_numbers(emission.get("program", ""))
    run = execute(emission.get("program", ""), facts)
    result = run["result"]
    correct = None
    if gold is not None and isinstance(result, (int, float)):
        correct = abs(result - gold) <= max(tolerance, 1e-9)
    return {
        "result": result,
        "exec_ok": run["exec_ok"],
        "stderr": run["stderr"],
        "fabrications": fabrications,            # facts lacking provenance
        "missing_coverage": missing_coverage,    # source quantities silently dropped
        "magic_numbers": magic,                  # un-provenanced numeric literals in the program
        "provenance_clean": not fabrications and not missing_coverage and not magic,
        "correct": correct,
    }


if __name__ == "__main__":
    # Smoke: PHYS1 done right — distractor discarded, no magic numbers, correct answer.
    src = ("A projectile is launched from ground level at a speed of 20 m/s at an angle of 30 degrees "
           "above the horizontal. Take g = 9.8 m/s^2. The launch pad is 2 meters wide.")
    emission = {
        "facts": {
            "v0": {"magnitude": 20, "unit": "m/s", "polarity": "affirmed", "type": "stated", "span": "20 m/s"},
            "angle_deg": {"magnitude": 30, "unit": "degree", "polarity": "affirmed", "type": "stated", "span": "30 degrees"},
            "g": {"magnitude": 9.8, "unit": "m/s^2", "polarity": "affirmed", "type": "stated", "span": "9.8 m/s^2"},
        },
        "discarded": [{"span": "2 meters wide", "reason": "pad width does not affect vertical max height"}],
        "program": (
            "import sympy as sp\n"
            "v0 = facts['v0']['magnitude']\n"
            "ang = sp.rad(facts['angle_deg']['magnitude'])\n"
            "g = facts['g']['magnitude']\n"
            "RESULT = float((v0*sp.sin(ang))**2/(2*g))\n"
        ),
    }
    spans = ["20 m/s", "30 degrees", "9.8 m/s^2", "2 meters wide"]
    out = adjudicate_program(spans, emission, gold=5.102, tolerance=0.02)
    print(json.dumps(out, indent=1))
