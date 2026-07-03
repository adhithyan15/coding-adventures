#!/usr/bin/env python3
"""ADJ84 — the 0.5B LOCAL arm (the airgapped/HIPAA deployment shape, per ADJ79/81).

Deployment split: a capable model derives+compiles the rulebook OFFLINE; the tiny LOCAL model
does ONLY input-IR extraction against that fixed rulebook's declared slots; engine.py
adjudicates deterministically. Question: does the engine equalize a 0.5B with Haiku/Opus?

We give qwen2.5:0.5b the SAME compiled rulebooks (slot schema) used in the capable-model arms
and ask it to fill the slots with byte spans (or null). Then engine.adjudicate. A faithful 0.5B
extraction -> the engine reaches the same verdict as Haiku/Opus. The 0.5B can fail by (a)
malformed JSON, (b) hallucinating a slot value (byte-check catches), or (c) missing a present
slot (false INDETERMINATE).
"""
import json
import re
import urllib.request

import engine

GEN = "http://127.0.0.1:11434/api/generate"
ITEMS = {it["id"]: it for it in json.load(open("items.json"))["items"]}

# Compiled rulebooks (as a capable model would compile them OFFLINE), with the slot schema the
# 0.5B must fill. Slot names here MUST match the "when" conditions.
COMPILED = {
 "U6-overstay": {
  "slots": {"stay_days": "number of days the visitor stayed (compute from entry to departure dates)",
            "extension_obtained": "true if the text says an extension was obtained before the 90th day, false if it says none was, null if not stated"},
  "rb": {"rules": [
      {"id": "fine", "when": {"stay_days": ">90", "extension_obtained": "false"}, "then": "fine owed", "source_span": "Overstaying that limit triggers a fine", "provenance": "passage"},
      {"id": "waiver", "when": {"stay_days": ">90", "extension_obtained": "true"}, "then": "no fine", "source_span": "unless the visitor obtained an extension before the 90th day", "provenance": "passage"},
      {"id": "within", "when": {"stay_days": "<=90"}, "then": "no fine", "source_span": "Visitors may remain in the country for up to 90 days.", "provenance": "passage"}],
     "default": {"then": "no fine", "source_span": "", "provenance": "passage"}}},
 "U1-waterdamage": {
  "slots": {"cause_type": "the cause of the damage: one of sudden_accidental, gradual_seepage, maintenance_neglect, flood; null if the text does not state the cause",
            "flood_rider_purchased": "true/false whether the policyholder bought the flood rider; null if unknown"},
  "rb": {"rules": [
      {"id": "sudden", "when": {"cause_type": "sudden_accidental"}, "then": "covered", "source_span": "Brightline Insurance covers water damage caused by sudden and accidental events", "provenance": "passage"},
      {"id": "gradual", "when": {"cause_type": "gradual_seepage"}, "then": "excluded", "source_span": "Damage resulting from gradual seepage or long-term maintenance neglect is excluded.", "provenance": "passage"},
      {"id": "neglect", "when": {"cause_type": "maintenance_neglect"}, "then": "excluded", "source_span": "Damage resulting from gradual seepage or long-term maintenance neglect is excluded.", "provenance": "passage"},
      {"id": "flood_no", "when": {"cause_type": "flood", "flood_rider_purchased": "false"}, "then": "not_covered", "source_span": "Flood damage is covered only under a separate rider, which this policyholder has not purchased.", "provenance": "passage"}],
     "default": {"then": "not_covered", "source_span": "", "provenance": "passage"}}},
 "N1-reimburse": {
  "slots": {"service_type": "preventive_screening or standard_service",
            "panel_grade": "grade_a, grade_b, or none",
            "within_age_range": "true if the screening is within the panel's recommended age range (infer from the enrollee's age vs the recommended starting age), false otherwise, null if unknown",
            "network_status": "in_network or out_of_network"},
  "rb": {"rules": [
      {"id": "prev_inrange", "when": {"service_type": "preventive_screening", "panel_grade": "grade_b", "within_age_range": "true"}, "then": "100%", "source_span": "Preventive screenings that receive a Grade A or Grade B recommendation from the National Preventive Services Panel are reimbursed at 100%, with the annual deductible waived.", "provenance": "passage"},
      {"id": "prev_outside", "when": {"service_type": "preventive_screening", "within_age_range": "false"}, "then": "80%", "source_span": "However, a preventive screening ordered outside the panel's recommended age range is treated as a standard service and reverts to the 80% in-network rate.", "provenance": "passage"},
      {"id": "standard", "when": {"network_status": "in_network"}, "then": "80%", "source_span": "The MediCare-Plus plan reimburses most in-network medical services at 80% after the annual deductible is met.", "provenance": "passage"}],
     "default": {"then": "undetermined", "source_span": "", "provenance": "passage"}}},
 "N3-importduty": {
  "slots": {"order_value": "the dollar value of the order as a number",
            "category": "the category of goods, lowercase (e.g. books)"},
  "rb": {"rules": [
      {"id": "books", "when": {"category": "books"}, "then": "0%", "source_span": "except for books, which are always duty-free regardless of order value", "provenance": "passage"},
      {"id": "over", "when": {"order_value": ">=800"}, "then": "5%", "source_span": "Orders valued at or above $800 incur a 5% import duty", "provenance": "passage"},
      {"id": "under", "when": {"order_value": "<800"}, "then": "0%", "source_span": "Orders valued under $800 are exempt from import duty.", "provenance": "passage"}],
     "default": {"then": "undetermined", "source_span": "", "provenance": "passage"}}},
}


def gen(model, prompt, npred=400, timeout=180):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def stage_b_prompt(input_text, question, slots):
    lines = "\n".join(f'  - "{k}": {v}' for k, v in slots.items())
    skel = ", ".join(f'"{k}": {{"value": null, "span": null, "type": "stated"}}' for k in slots)
    return (f"Extract facts from the TEXT as JSON. Do not solve the problem.\n\n"
            f"TEXT: {input_text}\n\nQUESTION: {question}\n\n"
            f"Fill these slots (use null if the TEXT does not state the value; DO NOT guess):\n{lines}\n\n"
            f'Output ONLY this JSON, filling values and verbatim spans:\n'
            f'{{"slots": {{{skel}}}}}\n')


def gold_match(item, res):
    gv = item["gold_verdict"]
    if gv == "INDETERMINATE":
        return res["verdict"] == "INDETERMINATE"
    if not res["verdict"].startswith("DETERMINATE"):
        return False
    return item.get("gold_answer_substring", "") in str(res["answer"])


def main():
    model = "qwen2.5:0.5b"
    print(f"LOCAL extraction model = {model} (rulebook compiled offline; engine adjudicates)\n")
    print(f"{'item':16} {'verdict':28} {'ans':8} {'gold':5} {'defens':7} {'bytes':6} note")
    print("-" * 95)
    for iid, spec in COMPILED.items():
        item = ITEMS[iid]
        raw = gen(model, stage_b_prompt(item["input_text"], item["question"], spec["slots"]), npred=800)
        try:
            ir = engine._extract_json(raw)
            if "slots" not in ir:
                ir = {"slots": ir}
        except Exception as e:
            # malformed IR is itself an unreliable extraction -> the framework refuses (defensible)
            print(f"{iid:16} {'PARSE-FAIL->ABSTAIN':28} {'-':8} {'X':5} {'OK(abstain)':7} {'-':6} {str(e)[:24]}")
            continue
        res = engine.adjudicate(ir, spec["rb"], item["input_text"])
        ok = gold_match(item, res)
        # defensible = never a confidently-wrong grounded answer. UNSAFE/INDETERMINATE abstentions count as defensible.
        defensible = (res["verdict"] in ("INDETERMINATE",) or res["verdict"].startswith("UNSAFE")
                      or (res["verdict"].startswith("DETERMINATE") and ok))
        note = ""
        if res["verdict"] == "INDETERMINATE":
            note = "blocks=" + ",".join(res["missing_slots_that_block"])
        if res["hallucinated_slots"]:
            note += " HALLUC=" + ",".join(res["hallucinated_slots"])
        print(f"{iid:16} {res['verdict'][:28]:28} {str(res['answer'])[:8]:8} "
              f"{('OK' if ok else 'X'):5} {('OK' if defensible else 'X'):7} {('OK' if res['byte_accounting_ok'] else 'X'):6} {note}")
        json.dump({"raw": raw, "parsed_ir": ir, "result": res},
                  open(f"runs/{iid}_qwen0.5b_local.json", "w"), indent=2)


if __name__ == "__main__":
    main()
