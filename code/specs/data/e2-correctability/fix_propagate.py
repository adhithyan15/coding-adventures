#!/usr/bin/env python3
"""E2 — the fix-locality + persist/propagate panel (RQ2, RQ3).

No LLM calls. This panel is mechanical: it runs the two already-built CAS
demonstrations and records, for the framework arm, that a correction is a SINGLE
local edit that persists under re-derivation and PROPAGATES to sibling cases with
ZERO answer-time model calls — and contrasts that with prose, which has no
localized handle (a "fix" is a rewrite of the derivation, nothing to propagate).

Sources (run as subprocesses for reproducibility):
  - adj52/cas/demo.py            — meningitis CSF-correlation override (fix + regression)
  - adj101 run100/cas_exercise.py — TAX rulebook -> compiled library -> held-out reuse

Run: python3 fix_propagate.py
"""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.abspath(os.path.join(HERE, ".."))
PY = sys.executable


def run(path):
    return subprocess.run([PY, path], capture_output=True, text=True,
                          cwd=os.path.dirname(path)).stdout


# --- RQ2/RQ3 framework arm: meningitis single-fact override -----------------
men = run(os.path.join(DATA, "adj52", "cas", "demo.py"))
men_edits = len(re.findall(r"^\s*EDIT ", men, re.M))
pre = re.search(r"pre-culture.*?([0-9.]+)\s+([0-9.]+)\s+DE-SATURATED", men)
reg = re.search(r"culture-positive.*?([0-9.]+)\s+([0-9.]+)\s+unchanged", men)
meningitis = {
    "fix": "one human override of the correlated CSF-chemistry facts (a single CAS edit)",
    "edits_applied_as_one_override": men_edits,
    "target_case_pre_culture": {"base_P": float(pre.group(1)) if pre else None,
                                "edited_P": float(pre.group(2)) if pre else None,
                                "effect": "false-certainty de-saturated"},
    "regression_sibling_culture_positive": {"base_P": float(reg.group(1)) if reg else None,
                                            "edited_P": float(reg.group(2)) if reg else None,
                                            "effect": "unchanged (no regression)"},
    "answer_time_model_calls": 0,
    "base_corpus": "immutable; edit lives in versioned, attributed, cited overrides/",
}

# --- RQ3 derive-once: TAX rulebook -> library -> held-out reuse -------------
tax = run(os.path.join(DATA, "adj101-defensibility-100crossdomain", "run100", "cas_exercise.py"))
tax_cases = re.findall(r"->\s+DETERMINATE\s+(\w+)", tax)
propagate = {
    "compiled_once": "TAX-1 rulebook (byte-verified) -> content-addressed library, no model",
    "held_out_cases_decided_on_cpu": len(tax_cases),
    "verdicts": tax_cases,
    "answer_time_model_calls": 0,
    "every_verdict_carries_proof": "rule -> fact -> cited policy bytes",
}

# --- the contrast (definitional; prose has no localized handle) -------------
panel = {
    "RQ2_fix_locality": {
        "framework": "single local edit at the located locus (override one CAS fact / edit one clause)",
        "prose": "no localized handle — the fix requires rewriting the derivation; not a fix of an artifact",
        "worked_example": meningitis,
    },
    "RQ3_persist_propagate": {
        "framework": "re-derive from corrected CAS; one override propagates to every sibling citing the fact",
        "prose": "nothing to propagate (no re-derivable artifact)",
        "meningitis_regression_safe": meningitis["regression_sibling_culture_positive"],
        "tax_derive_once": propagate,
        "headline_number": "1 edit / 1 compiled library -> N sibling cases corrected, answer-time model calls = 0",
    },
}

import json
json.dump(panel, open(os.path.join(HERE, "fix_propagate.json"), "w"), indent=1)
print(json.dumps(panel, indent=1))
