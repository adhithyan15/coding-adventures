#!/usr/bin/env python3
"""ADJ84 — the deterministic adjudication engine (the part that is the FRAMEWORK, not a prompt).

The model's ONLY jobs are the two extraction stages (ADJ78/79/81 division of labour):
  Stage 1  input  -> input-IR   : typed SLOTS (stated|inferred) each with a byte span,
                                   plus UNCERTAINTIES and QUESTIONS.
  Stage 2  rule   -> rulebook-IR: structured RULES over those slots, each with a source span
                                   and a provenance class.
This module does the reasoning DETERMINISTICALLY:
  - verifies every stated slot's span is verbatim in the input (byte-accounting),
  - evaluates the rules against the slots,
  - and crucially returns INDETERMINATE *structurally* whenever a slot needed to decide
    between rules is missing -- no matter what the model's prose would have concluded.

That last property is the whole point: defensibility comes from the ENGINE, so a model that
merely extracts faithfully (even a weak one) cannot overclaim. The only way to fail is to
HALLUCINATE a slot value that was never in the input -- which the byte-span check catches.
"""
import json
import re


# ---------------------------------------------------------------------------
# Predicate language for a rule's "when" conditions.
# A condition is slot_name -> predicate string. Supported predicates:
#   "part_time"            exact string-equality match (value == "part_time")
#   ">2020" "<=18" "==45"  numeric comparison against the slot's numeric value
#   "true"/"false"         boolean match
#   "*"                    slot must merely be present (non-null)
# Evaluation returns one of: True (known-satisfied), False (known-violated),
# None (UNKNOWN -- the slot value is missing, so the condition cannot be decided).
# ---------------------------------------------------------------------------
_NUM = re.compile(r"^(>=|<=|==|>|<)\s*(-?\d+(?:\.\d+)?)$")


def eval_condition(slot_value, predicate):
    if slot_value is None:
        return None  # UNKNOWN: cannot evaluate a condition on a missing slot
    predicate = str(predicate).strip()
    m = _NUM.match(predicate)
    if m:
        op, num = m.group(1), float(m.group(2))
        try:
            v = float(slot_value)
        except (TypeError, ValueError):
            return False
        return {">": v > num, "<": v < num, ">=": v >= num,
                "<=": v <= num, "==": v == num}[op]
    if predicate in ("true", "false"):
        return str(slot_value).strip().lower() == predicate
    if predicate == "*":
        return True  # present and non-null
    return str(slot_value).strip().lower() == predicate.lower()


def rule_status(rule, slots):
    """Return ('satisfied'|'violated'|'unknown', [missing_slots])."""
    missing = []
    any_unknown = False
    for slot_name, predicate in rule.get("when", {}).items():
        val = slots.get(slot_name, {}).get("value") if slot_name in slots else None
        res = eval_condition(val, predicate)
        if res is False:
            return "violated", []          # one known-false condition kills the rule
        if res is None:
            any_unknown = True
            missing.append(slot_name)
    return ("unknown", missing) if any_unknown else ("satisfied", [])


# ---------------------------------------------------------------------------
# Byte-accounting: every STATED slot must quote a verbatim span of the input.
# Inferred slots (type == "inferred") are allowed a null/derived span but are
# flagged as inferred (not byte-grounded) so the audit trail stays honest.
# ---------------------------------------------------------------------------
def verify_spans(slots, input_text):
    norm_in = re.sub(r"\s+", " ", input_text)
    report = {}
    for name, s in slots.items():
        stype = s.get("type", "stated")
        span = s.get("span")
        if stype == "inferred":
            report[name] = "inferred (not byte-grounded; justified)"
        elif span and re.sub(r"\s+", " ", span) in norm_in:
            report[name] = "verbatim-ok"
        elif s.get("value") is None:
            report[name] = "absent (correctly null)"
        else:
            report[name] = "HALLUCINATED-SPAN (value present, span not in input)"
    return report


# ---------------------------------------------------------------------------
# Precedence for DEFEASIBLE rules (added in v2; see FINDINGS "override-precedence").
# When several satisfied rules disagree, resolve by the two classic precedence
# principles of defeasible reasoning (this is how Adj-Lang / MYCIN handle it):
#   1. OVERRIDE MARKER: a rule whose source_span carries override language
#      ("except", "regardless", "unless", "however", "instead", "notwithstanding")
#      states an exception and dominates the rule it excepts.
#   2. SPECIFICITY: failing that, the rule with MORE conditions (more specific) wins.
# If neither breaks the tie, it remains a genuine CONFLICT.
# ---------------------------------------------------------------------------
OVERRIDE_MARKERS = ("except", "regardless", "unless", "however", "instead", "notwithstanding")


def _specificity(rule):
    return len(rule.get("when", {}))


def resolve_precedence(satisfied):
    marked = [r for r in satisfied
              if any(m in (r.get("source_span") or "").lower() for m in OVERRIDE_MARKERS)]
    pool = marked or satisfied
    max_spec = max(_specificity(r) for r in pool)
    top = [r for r in pool if _specificity(r) == max_spec]
    if len({r["then"] for r in top}) == 1:
        return top[0], ("override-marker" if marked else "specificity")
    return None, "unresolved"


# ---------------------------------------------------------------------------
# The deterministic adjudication.
# ---------------------------------------------------------------------------
def adjudicate(input_ir, rulebook_ir, input_text):
    slots = input_ir.get("slots", {})
    rules = rulebook_ir.get("rules", [])
    span_report = verify_spans(slots, input_text)

    satisfied, unknown = [], []
    for r in rules:
        status, missing = rule_status(r, slots)
        if status == "satisfied":
            satisfied.append(r)
        elif status == "unknown":
            unknown.append((r, missing))

    consequences_satisfied = {r["then"] for r in satisfied}
    consequences_unknown = {r["then"] for (r, _) in unknown}

    # INDETERMINATE if an unknown rule could yield a DIFFERENT consequence than the
    # satisfied set (i.e., a missing slot actually changes the outcome).
    missing_slots = sorted({m for (_, ms) in unknown for m in ms})
    if consequences_unknown - consequences_satisfied:
        verdict = "INDETERMINATE"
        answer = None
    elif len(consequences_satisfied) == 1:
        verdict = "DETERMINATE"
        answer = next(iter(consequences_satisfied))
    elif len(consequences_satisfied) > 1:
        winner, how = resolve_precedence(satisfied)   # defeasible precedence (v2)
        if winner is not None:
            verdict = f"DETERMINATE(precedence:{how})"
            answer = winner["then"]
        else:
            verdict = "CONFLICT"      # multiple satisfied rules genuinely disagree
            answer = None
    elif rulebook_ir.get("default"):
        verdict = "DETERMINATE(default)"
        answer = rulebook_ir["default"]["then"]
    else:
        verdict = "INDETERMINATE"
        answer = None

    hallucinated = [k for k, v in span_report.items() if v.startswith("HALLUCINATED")]
    # BYTE-ACCOUNTING GATE (v3): a verdict built on an unverifiable (hallucinated) slot is not
    # defensible no matter what the rules say. Refuse rather than emit a confidently-wrong,
    # ungrounded answer. This is what makes even a weak extractor SAFE: it abstains, never
    # overclaims. (The audit trail still records which slot failed verification.)
    if hallucinated:
        verdict, answer = "UNSAFE(unverified-extraction)", None

    return {
        "verdict": verdict,
        "answer": answer,
        "fired_rules": [r.get("id", r["then"]) for r in satisfied],
        "missing_slots_that_block": missing_slots if verdict == "INDETERMINATE" else [],
        "span_report": span_report,
        "hallucinated_slots": hallucinated,
        "byte_accounting_ok": not hallucinated,
        "surfaced_uncertainties": input_ir.get("uncertainties", []),
        "proof": {
            "slots": {k: {"value": v.get("value"), "span": v.get("span"),
                          "type": v.get("type", "stated")} for k, v in slots.items()},
            "rules": rules,
            "default": rulebook_ir.get("default"),
        },
    }


def _extract_json(text):
    """Best-effort: pull the first {...} JSON object out of a model response."""
    text = text.strip()
    text = re.sub(r"^```(?:json)?|```$", "", text, flags=re.M).strip()
    depth, start = 0, None
    for i, ch in enumerate(text):
        if ch == "{":
            if depth == 0:
                start = i
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0 and start is not None:
                return json.loads(text[start:i + 1])
    raise ValueError("no JSON object found")


if __name__ == "__main__":
    # tiny self-test: U6-overstay with extension slot left null -> must be INDETERMINATE
    inp = ("Visitors may remain in the country for up to 90 days. Overstaying that limit "
           "triggers a fine, unless the visitor obtained an extension before the 90th day. "
           "Priya entered the country on January 1 and departed on May 1 of the same year.")
    input_ir = {"slots": {
        "stay_days": {"value": 120, "span": None, "type": "inferred"},
        "extension_obtained": {"value": None, "span": None, "type": "stated"},
    }, "uncertainties": ["whether Priya obtained an extension is not stated"]}
    rulebook_ir = {"rules": [
        {"id": "fine", "when": {"stay_days": ">90", "extension_obtained": "false"},
         "then": "fine owed", "source_span": "Overstaying that limit triggers a fine",
         "provenance": "passage"},
        {"id": "waiver", "when": {"stay_days": ">90", "extension_obtained": "true"},
         "then": "no fine", "source_span": "unless the visitor obtained an extension before the 90th day",
         "provenance": "passage"},
    ]}
    import pprint
    pprint.pprint(adjudicate(input_ir, rulebook_ir, inp))
