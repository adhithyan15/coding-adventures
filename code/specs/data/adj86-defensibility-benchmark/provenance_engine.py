#!/usr/bin/env python3
"""ADJ86 — provenance-complete engine: extend byte-accounting to BOTH legs of the pipeline.

The ADJ84 engine byte-checks only the INPUT (stated slots vs scenario). Two provenance gaps
let "invented conditions" through (this is what the ADJ86 v2 blind judge caught):

  (A) RULEBOOK source_spans were never verified against the POLICY — the engine never even
      received the policy text. A rule could cite a span that isn't in the policy.
  (B) INFERRED slots (span=null) are EXEMPT from the byte check by design — you cannot
      verbatim-ground a derived fact. So a model can launder an assumption (e.g.
      "cardiologist -> specialist", "apartment -> necessity") through an inferred slot, have a
      rule condition on it, and reach a confident DETERMINATE verdict with the assumption
      INVISIBLE. Byte provenance allowed it because inference is *permitted* to be ungrounded;
      the bug is that the engine then TRUSTED it silently.

This wrapper closes both, on the project's own principle — a verdict is only as grounded as
its weakest dispositive link:

  (A) verify_rule_spans: every rule.source_span must be verbatim in the policy (whitespace/
      case-normalized). Unverifiable rules are DROPPED (cannot fire) and reported as
      hallucinated_rules; if any exist, byte_accounting_ok is False.
  (B) assumption discipline: any DISPOSITIVE condition (a fired rule's `when` slot) that rests
      on an inferred / span-less slot is surfaced as an explicit ASSUMPTION. A DETERMINATE
      verdict that depends on one is relabelled `DETERMINATE(assumes: ...)`, and
      `fully_grounded` is False. The determination is still made (we don't forbid inference —
      the project gates on representation, not interpretation) but the assumption is now
      auditable instead of laundered.
"""
from __future__ import annotations

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "adj84-pipeline-defensibility"))
import engine as base  # noqa: E402  — the ADJ84 deterministic engine (reused unchanged)


def _norm(s):
    return re.sub(r"\s+", " ", (s or "")).strip().lower()


def verify_rule_spans(rules, policy):
    """(clean_rules, hallucinated_rule_ids, report) — drop rules whose source_span isn't in policy."""
    pol = _norm(policy)
    clean, hallucinated, report = [], [], {}
    for r in rules:
        span = _norm(r.get("source_span"))
        rid = r.get("id", r.get("then", "?"))
        if span and span in pol:
            report[rid] = "OK"
            clean.append(r)
        else:
            report[rid] = f"HALLUCINATED_RULE: source_span not verbatim in policy ({r.get('source_span','')!r:.50})"
            hallucinated.append(rid)
    return clean, hallucinated, report


def adjudicate(input_ir, rulebook_ir, scenario, policy, justifications=None):
    """justifications: optional {slot_name: {"verdict": "ENTAILED"|"LEAP", "basis_span": str}}.
    With it, an inferred dispositive slot counts as GROUNDED iff its gate said ENTAILED with a
    verbatim basis; otherwise it is a surfaced ASSUMPTION. Without it, every inferred dispositive
    slot is treated as an assumption (the conservative pre-gate behaviour)."""
    rules = rulebook_ir.get("rules", [])
    clean_rules, hallucinated_rules, rule_report = verify_rule_spans(rules, policy)
    # adjudicate using ONLY the policy-verified rules (invented rules cannot fire)
    rb_clean = {**rulebook_ir, "rules": clean_rules}
    res = base.adjudicate(input_ir, rb_clean, scenario)

    # (B) assumption discipline: which fired-rule conditions rest on inferred slots that are
    # NOT entailed by their cited bytes? Those are the genuine, surfaced assumptions.
    just = {j.get("slot", k): j for k, j in (justifications or {}).items()} if isinstance(justifications, dict) \
        else {j["slot"]: j for j in (justifications or [])}
    scen = _norm(scenario)
    slots = input_ir.get("slots", {})
    fired = set(res.get("fired_rules", []))
    assumptions = []
    for r in clean_rules:
        rid = r.get("id", r.get("then"))
        if rid not in fired:
            continue
        for slot_name in (r.get("when") or {}):
            sv = slots.get(slot_name, {})
            if sv.get("value") is None:
                continue  # missing slot -> handled structurally by INDETERMINATE
            if sv.get("type") == "stated" and sv.get("span") not in (None, ""):
                continue  # grounded by verbatim scenario bytes
            # inferred dispositive slot: grounded iff its gate said ENTAILED with a verbatim basis
            j = just.get(slot_name)
            basis_ok = bool(j and j.get("basis_span") and _norm(j["basis_span"]) in scen)
            if not (j and j.get("verdict") == "ENTAILED" and basis_ok):
                assumptions.append(slot_name)
    assumptions = sorted(set(assumptions))

    verdict = res["verdict"]
    if verdict.startswith("DETERMINATE") and assumptions:
        verdict = f"{verdict}(assumes:{','.join(assumptions)})"
    byte_ok = res["byte_accounting_ok"] and not hallucinated_rules
    if hallucinated_rules and not verdict.startswith("UNSAFE"):
        verdict = f"UNVERIFIED-RULEBOOK({verdict})"

    return {
        **res,
        "verdict": verdict,
        "byte_accounting_ok": byte_ok,
        "fully_grounded": byte_ok and not assumptions,
        "assumptions": assumptions,
        "hallucinated_rules": hallucinated_rules,
        "rule_span_report": rule_report,
    }


if __name__ == "__main__":
    # demo: an inferred dispositive slot is surfaced, not laundered.
    ir = {"slots": {"provider_type": {"value": "cardiologist", "span": "cardiologist", "type": "stated"},
                    "provider_is_specialist": {"value": True, "span": None, "type": "inferred"},
                    "deductible_met": {"value": True, "span": "deductible is fully met", "type": "stated"}},
          "uncertainties": []}
    rb = {"rules": [{"id": "r70", "when": {"provider_is_specialist": "true", "deductible_met": "true"},
                     "then": "70%", "source_span": "in-network specialist visits at 70% after the annual deductible is met"}]}
    policy = "The HealthFirst plan reimburses in-network specialist visits at 70% after the annual deductible is met."
    scen = "A member whose deductible is fully met saw a cardiologist."
    import json
    out = adjudicate(ir, rb, scen, policy)
    print("verdict:", out["verdict"])
    print("fully_grounded:", out["fully_grounded"], "| assumptions:", out["assumptions"])
    print("rule_span_report:", json.dumps(out["rule_span_report"]))
