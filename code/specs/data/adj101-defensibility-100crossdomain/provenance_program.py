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


def _norm_formula(s: str) -> str:
    """Normalize a formula/equation string for faithfulness comparison: x**3 == x^3, 2*x == 2x, no
    spaces, drop a trailing '=0'. A notation transform (math -> python syntax) is not a mis-extraction."""
    t = _norm(s).replace("**", "^").replace("*", "").replace(" ", "")
    return t[:-2] if t.endswith("=0") else t


def check_justification(facts: dict) -> dict:
    """Split facts by provenance quality:
      - fabrications: NO provenance at all (stated w/o span, or inferred w/o basis_span).
      - surfaced_assumptions: inferred WITH a basis but the gate said LEAP (not entailed by the
        bytes). These are NOT fabrications — they are the audit working: the assumption is surfaced
        and flagged for a human to verify/override (the provenance-engine assumption discipline).
    Fully grounded iff stated(span) or inferred(basis + ENTAILED)."""
    fabrications, assumptions = [], []
    for name, f in facts.items():
        typ = f.get("type")
        if typ == "stated" and f.get("span"):
            continue
        if typ == "inferred" and f.get("basis_span"):
            if f.get("entailment") == "ENTAILED":
                continue
            assumptions.append(name)   # surfaced, basis-backed assumption (auditable)
            continue
        fabrications.append(name)      # no provenance -> genuine fabrication
    return {"fabrications": fabrications, "surfaced_assumptions": assumptions}


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


# Unit-scale factors: a typed magnitude is faithful to its span modulo a common unit conversion —
# e.g. "4%" -> 0.04 (x100), SI prefixes (km<->m x1000). This is REQUIRED because the IR is unit-typed:
# the whole point of typing is that 4% becomes the fraction 0.04, so the raw "4" still lives in the span.
_UNIT_SCALES = (1.0, 100.0, 0.01, 1000.0, 0.001, 60.0, 3600.0)


def check_faithfulness(facts: dict) -> list:
    """Stated NUMERIC facts whose magnitude does not match their own cited span (modulo a unit
    conversion) — a localized mis-extraction. Non-numeric data facts (SMILES, equations, labels) are
    NOT quantities and are skipped (their faithfulness is the string appearing in the span). This is
    the workhorse of localizability: a value that contradicts its cited bytes is caught at that fact."""
    bad = []
    for name, f in facts.items():
        if f.get("type") != "stated":
            continue
        mag, span = f.get("magnitude"), f.get("span")
        if mag is None or not span:
            continue
        try:
            magf = float(mag)
        except (TypeError, ValueError):
            # non-numeric datum (e.g. a SMILES/equation string): faithful iff it appears in the span,
            # modulo notation (x**3 vs x^3, 2*x vs 2x, spacing) — a syntax transform is not a mis-extraction.
            if isinstance(mag, str) and _norm_formula(mag) and _norm_formula(mag) not in _norm_formula(span):
                bad.append(name)
            continue
        span_vals = _numeric_values(span)
        if not span_vals:
            continue  # span carries no number to check against (e.g. a worded quantity)
        if not any(abs(magf * k - v) < 1e-6 or abs(magf / k - v) < 1e-6
                   for v in span_vals for k in _UNIT_SCALES):
            bad.append(name)
    return bad


def _numeric_values(text: str) -> set:
    out = set()
    for m in _QTY.finditer(text or ""):
        try:
            out.add(float(m.group(0).replace(",", "")))
        except ValueError:
            pass
    return out


def override_facts(emission: dict, overrides: dict) -> dict:
    """Return a corrected emission with named facts' magnitudes overridden — the 'fix the fact, not
    the weight' move. Re-running adjudicate_program on the result re-derives with ZERO model calls."""
    facts = json.loads(json.dumps(emission.get("facts", {})))  # deep copy
    for name, new_mag in overrides.items():
        if name in facts:
            facts[name]["magnitude"] = new_mag
            facts[name].setdefault("_override", {})["from"] = emission["facts"][name].get("magnitude")
            facts[name]["_override"]["to"] = new_mag
    return {**emission, "facts": facts}


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
    just = check_justification(facts)
    fabrications = just["fabrications"]              # facts with NO provenance at all
    surfaced_assumptions = just["surfaced_assumptions"]  # inferred+basis but LEAP — auditable, verify
    unfaithful = check_faithfulness(facts)           # facts whose value contradicts their cited span
    missing_coverage = check_coverage(quantity_spans, facts, emission.get("discarded", []))
    magic = magic_numbers(emission.get("program", ""))
    run = execute(emission.get("program", ""), facts)
    result = run["result"]

    # Correctness is INFORMATIONAL in the rescored paradigm, not the target. The target is whether the
    # trail is auditable and the error, when present, is localized + correctable.
    correct = None
    if gold is not None and isinstance(result, (int, float)):
        correct = abs(result - gold) <= max(tolerance, 1e-9)

    # The error locus: where an auditor should look, in order. A mis-extracted fact (value != cited
    # bytes) or a fabrication is fixed first; a surfaced assumption is verified/overridden; an exec
    # error points at the program.
    error_locus = {
        "unfaithful_facts": unfaithful,
        "fabrications": fabrications,
        "surfaced_assumptions": surfaced_assumptions,
        "exec_error": run["stderr"] or None,
    }
    # AUDITABLE = nothing un-provenanced slipped through: no fabrication, no value-vs-span
    # contradiction, no silently dropped quantity, no laundered magic number. A SURFACED ASSUMPTION
    # does NOT break auditability — it is the audit working (flagged for human verification).
    auditable = not (fabrications or unfaithful or missing_coverage or magic)
    return {
        "result": result,
        "exec_ok": run["exec_ok"],
        "stderr": run["stderr"],
        # provenance / auditability signals (the PRIMARY axis)
        "fabrications": fabrications,
        "unfaithful_facts": unfaithful,
        "surfaced_assumptions": surfaced_assumptions,
        "missing_coverage": missing_coverage,
        "magic_numbers": magic,
        "auditable": auditable,
        "error_locus": error_locus,          # when wrong, this names where to look + override
        # correctness is reported but SECONDARY
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
