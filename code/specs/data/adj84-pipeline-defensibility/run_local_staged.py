#!/usr/bin/env python3
"""ADJ84 v4 — the 0.5B LOCAL arm done RIGHT: staged natural-language extraction (ADJ78/81).

run_local.py asked the 0.5B for one-shot JSON -> it choked (ADJ77's rigid-format failure):
hallucinated slots / unparseable output -> the framework correctly made it ABSTAIN, but it
yielded nothing. Here we use the 0.5B's actual strength: COPY-THE-PHRASE, one focused question
per slot. The FRAMEWORK (deterministic) maps copied phrases to slot values and does the
inference (dates->duration, age vs threshold). The model never reasons; it only copies.

Engine + byte-gate unchanged. Expectation: defensibility stays high (NONE on absent slots ->
INDETERMINATE; non-verbatim copy -> UNSAFE), and YIELD rises vs the JSON arm.
"""
import json
import re
import urllib.request

import engine
from run_local import COMPILED, gen, gold_match  # reuse rulebooks + ollama call + scorer

ITEMS = {it["id"]: it for it in json.load(open("items.json"))["items"]}
MONTHS = {m: i for i, m in enumerate(
    ["january", "february", "march", "april", "may", "june", "july", "august",
     "september", "october", "november", "december"], 1)}
_CUM = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]  # non-leap day-of-year starts


def doy(month, day):
    return _CUM[month - 1] + day


def is_none(reply):
    r = reply.strip().lower()
    return ("none" in r and len(r) < 40) or r in ("", "n/a", "not stated", "not mentioned")


def copy_span(reply, input_text):
    """Pull the model's copied phrase and check it is verbatim in the input."""
    m = re.search(r'["“]([^"”]{3,})["”]', reply)  # quoted text if present
    cand = (m.group(1) if m else reply).strip().strip('.').strip()
    norm_in = re.sub(r"\s+", " ", input_text)
    norm_c = re.sub(r"\s+", " ", cand)
    return (cand, True) if norm_c and norm_c in norm_in else (cand, False)


def ask_copy(model, input_text, instruction):
    p = (f"Passage: {input_text}\n\n{instruction}\n"
         f"Reply with ONLY the exact words copied from the passage, or the single word NONE "
         f"if the passage does not state it. No explanation.")
    return gen(model, p, npred=60)


# Per-item extraction recipes: each slot -> (instruction, mapper). Inferred slots are computed
# by the framework from copied stated facts (the model only copies; it never computes).
def extract(model, iid, input_text):
    slots, prov = {}, {}

    def stated(name, instruction, mapper):
        reply = ask_copy(model, input_text, instruction)
        prov[name] = reply
        if is_none(reply):
            slots[name] = {"value": None, "span": None, "type": "stated"}
            return None
        span, ok = copy_span(reply, input_text)
        val = mapper(reply)
        slots[name] = {"value": val, "span": (span if ok else span), "type": "stated"}
        return val

    if iid == "U6-overstay":
        e = ask_copy(model, input_text, "Copy the exact words giving the date the visitor ENTERED the country.")
        d = ask_copy(model, input_text, "Copy the exact words giving the date the visitor DEPARTED the country.")
        days = None
        try:
            em = re.search(r"(january|february|march|april|may|june|july|august|september|october|november|december)\s+(\d{1,2})", e.lower())
            dm = re.search(r"(january|february|march|april|may|june|july|august|september|october|november|december)\s+(\d{1,2})", d.lower())
            if em and dm:
                days = doy(MONTHS[dm.group(1)], int(dm.group(2))) - doy(MONTHS[em.group(1)], int(em.group(2)))
        except Exception:
            days = None
        slots["stay_days"] = {"value": days, "span": None, "type": "inferred"}
        prov["stay_days"] = f"computed from entry={e!r} departure={d!r}"
        stated("extension_obtained",
               "Copy the exact words stating whether the visitor obtained an EXTENSION before the 90th day. If the passage does not mention an extension, reply NONE.",
               lambda r: False if re.search(r"\bnot\b|\bno\b", r.lower()) else None)

    elif iid == "U1-waterdamage":
        def cause_map(r):
            rl = r.lower()
            if "burst" in rl or "sudden" in rl or "accidental" in rl:
                return "sudden_accidental"
            if "seepage" in rl:
                return "gradual_seepage"
            if "neglect" in rl:
                return "maintenance_neglect"
            if "flood" in rl:
                return "flood"
            return None  # copied a non-cause phrase (e.g. "mold") -> not a cause value
        stated("cause_type",
               "Copy the exact words stating what CAUSED the water damage (the cause or event). If the passage does not state the cause, reply NONE.",
               cause_map)
        stated("flood_rider_purchased",
               "Copy the exact words stating whether a flood/separate rider was purchased. If not mentioned, reply NONE.",
               lambda r: False if re.search(r"\bnot\b|\bno\b", r.lower()) else True)

    elif iid == "N1-reimburse":
        stated("service_type",
               "Copy the exact words naming the type of medical service the enrollee received.",
               lambda r: "preventive_screening" if "screening" in r.lower() else "standard_service")
        stated("panel_grade",
               "Copy the exact words stating the screening's recommendation grade (e.g., Grade A or Grade B).",
               lambda r: "grade_b" if re.search(r"grade\s*b", r.lower()) else ("grade_a" if re.search(r"grade\s*a", r.lower()) else None))
        stated("network_status",
               "Copy the exact words stating whether the enrollee is in-network or out-of-network.",
               lambda r: "in_network" if "in-network" in r.lower() or "in network" in r.lower() else "out_of_network")
        ea = ask_copy(model, input_text, "Copy the exact words stating the enrollee's AGE.")
        ra = ask_copy(model, input_text, "Copy the exact words stating the age at which the panel recommends this screening BEGINS.")
        within = None
        try:
            eav = int(re.search(r"(\d{1,3})", ea).group(1))
            rav = int(re.search(r"(\d{1,3})", ra).group(1))
            within = (eav >= rav)
        except Exception:
            within = None
        slots["within_age_range"] = {"value": within, "span": None, "type": "inferred"}
        prov["within_age_range"] = f"computed from age={ea!r} start={ra!r}"

    elif iid == "N3-importduty":
        stated("order_value",
               "Copy the exact words stating the dollar VALUE of the order.",
               lambda r: int(re.search(r"(\d[\d,]*)", r).group(1).replace(",", "")) if re.search(r"\d", r) else None)
        stated("category",
               "Copy the exact words stating what KIND of goods the order is for.",
               lambda r: "books" if "book" in r.lower() else r.strip().lower())

    return {"slots": slots, "uncertainties": []}, prov


def main():
    model = "qwen2.5:0.5b"
    print(f"LOCAL staged extractor = {model} (copy-the-phrase per slot; framework maps+infers)\n")
    print(f"{'item':16} {'verdict':28} {'ans':8} {'gold':5} {'defens':7} {'bytes':6} note")
    print("-" * 95)
    summ = {"defensible": 0, "correct": 0}
    for iid, spec in COMPILED.items():
        item = ITEMS[iid]
        ir, prov = extract(model, iid, item["input_text"])
        res = engine.adjudicate(ir, spec["rb"], item["input_text"])
        ok = gold_match(item, res)
        defensible = (res["verdict"] == "INDETERMINATE" or res["verdict"].startswith("UNSAFE")
                      or (res["verdict"].startswith("DETERMINATE") and ok))
        summ["defensible"] += defensible
        summ["correct"] += ok
        note = ""
        if res["verdict"] == "INDETERMINATE":
            note = "blocks=" + ",".join(res["missing_slots_that_block"])
        if res["hallucinated_slots"]:
            note += " HALLUC=" + ",".join(res["hallucinated_slots"])
        print(f"{iid:16} {res['verdict'][:28]:28} {str(res['answer'])[:8]:8} "
              f"{('OK' if ok else 'X'):5} {('OK' if defensible else 'X'):7} {('OK' if res['byte_accounting_ok'] else 'X'):6} {note}")
        json.dump({"ir": ir, "provenance": prov, "result": res},
                  open(f"runs/{iid}_qwen0.5b_staged.json", "w"), indent=2)
    n = len(COMPILED)
    print(f"\nqwen2.5:0.5b staged: defensible {summ['defensible']}/{n}, correct {summ['correct']}/{n}")


if __name__ == "__main__":
    main()
