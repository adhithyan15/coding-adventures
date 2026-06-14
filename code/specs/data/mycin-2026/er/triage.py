#!/usr/bin/env python3
"""triage.py - map the warm-path output to an ER acuity + immediate actions.

MYCIN-2026 C3. The last deterministic step of the ER spine: take the differential
(leading diagnosis + whether the evidence is sufficient) and the decomposed
findings, and produce an Emergency Severity Index acuity (1 = resuscitation … 5 =
non-urgent) plus the immediate-action checklist - all from the grounded
`triage_rules.json`, at 0 model calls.

Precedence (most-acute wins):
  1. a RED-FLAG finding (e.g. an active seizure) escalates to resuscitation
     regardless of the differential - a high-risk presentation is emergent before
     the diagnosis is settled;
  2. otherwise the LEADING diagnosis's acuity, if the evidence is sufficient;
  3. otherwise (insufficient evidence / unknown leader) the undifferentiated
     default - urgent, not low-acuity, so a concerning presentation is not
     under-triaged.

Decision SUPPORT only: the acuity + actions are a grounded, overridable starting
checklist the triage nurse / physician reviews; never a replacement for judgment.
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
RULES = json.loads((HERE / "triage_rules.json").read_text())


def triage(leader: str | None, decision_type: str | None, findings: list[str]) -> dict:
    """Return the triage decision: acuity (1-5), label, immediate_actions, the
    rule that fired, and its source. `findings` are the kept "functor(value)"
    strings from the decomposition."""
    finding_set = set(findings)

    # 1. Red flags escalate regardless of the differential.
    for rf in RULES["red_flags"]:
        if rf["finding"] in finding_set:
            return {
                "acuity": rf["acuity"], "label": rf["label"],
                "immediate_actions": [rf["action"]],
                "time_target_min": 0,
                "rule": f"red_flag:{rf['finding']}",
                "source": rf["source"],
            }

    # 2. Sufficient-evidence leading diagnosis.
    dx = RULES["diagnosis_acuity"].get(leader or "")
    if dx is not None and decision_type != "insufficient_evidence":
        return {
            "acuity": dx["acuity"], "label": dx["label"],
            "immediate_actions": dx["immediate_actions"],
            "time_target_min": dx.get("time_target_min"),
            "rule": f"diagnosis:{leader}",
            "source": dx["source"],
        }

    # 3. Undifferentiated / insufficient evidence -> urgent default.
    d = RULES["insufficient_evidence"]
    return {
        "acuity": d["acuity"], "label": d["label"],
        "immediate_actions": d["immediate_actions"],
        "time_target_min": None,
        "rule": "insufficient_evidence",
        "source": d["source"],
    }
