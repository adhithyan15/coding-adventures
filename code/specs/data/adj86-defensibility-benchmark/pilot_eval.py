#!/usr/bin/env python3
"""ADJ86 pilot evaluator (v2) — 2x2 {Haiku,Opus} x {bare,framework}, real engine.py.

Thesis: framework-assisted Haiku reaches plain-Opus defensibility. Reports, per cell,
accuracy (vs gold) and defensibility (framework = byte-accounting clean + engine-owned
verdict; bare = blind-audited verifiable-claim fraction), plus the underdetermined contrast.

Usage: python pilot_eval.py <workflow-results.json>
"""
from __future__ import annotations

import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
import provenance_engine as pe  # noqa: E402  — provenance-complete (rulebook + inferred-slot gates)

ITEMS = {it["id"]: it for it in json.load(open(os.path.join(HERE, "items_pilot.json")))["items"]}
ABSTAIN = ("cannot", "not stated", "not specified", "insufficient", "unknown", "indeterminate",
           "undetermined", "depends", "more information", "unable to", "does not state", "not enough")
# answer equivalences so scoring isn't a substring artifact (LAW6: engine "then" may be "duty-free")
EQUIV = {"0%": ("0%", "duty-free", "duty free", "0 %", "zero"), "75": ("75", "$75"),
         "required": ("required", "yes", "license is required")}


def equiv_match(gold_sub, text):
    text = str(text).lower()
    for alt in EQUIV.get(gold_sub, (gold_sub,)):
        if alt.lower() in text:
            return True
    return False


def fw_gold(item, res):
    gv = item["gold_verdict"]
    if gv == "INDETERMINATE":
        return res["verdict"] == "INDETERMINATE"
    return res["verdict"].startswith("DETERMINATE") and equiv_match(item.get("gold_answer_substring", ""), res["answer"])


def bare_score(item, ans):
    low = ans.lower()
    abstained = any(k in low for k in ABSTAIN)
    if item["gold_verdict"] == "INDETERMINATE":
        return abstained, abstained
    return equiv_match(item.get("gold_answer_substring", ""), ans), abstained


def main():
    res = json.loads(open(sys.argv[1]).read())
    res = res.get("result", res)
    rows = res["results"] if "results" in res else res

    cells = defaultdict(lambda: {"n": 0, "fw_acc": 0, "fw_byte": 0, "fw_halluc": 0, "fw_indet_on_ud": 0,
                                 "bare_acc": 0, "bare_def_sum": 0.0, "bare_fabricate_on_ud": 0, "ud_n": 0})
    detail = []
    for r in rows:
        item = ITEMS[r["id"]]
        m = r["model"]
        eng = pe.adjudicate(r["input_ir"], r["rulebook_ir"], item["scenario"], item["policy"], r.get("justifications", []))
        fw_ok = fw_gold(item, eng)
        byte_ok = eng["byte_accounting_ok"]
        halluc = eng.get("hallucinated_slots", [])
        au = r["audit"]
        bare_def = (au["claims_total"] - au["claims_unsupported"]) / au["claims_total"] if au["claims_total"] else 0.0
        bare_acc, abstained = bare_score(item, r["bare"]["answer"])

        c = cells[m]
        c["n"] += 1
        c["fw_acc"] += fw_ok
        c["fw_byte"] += byte_ok
        c["fw_halluc"] += bool(halluc)
        c["bare_acc"] += bare_acc
        c["bare_def_sum"] += bare_def
        if item["stratum"] == "underdetermined-baited":
            c["ud_n"] += 1
            c["fw_indet_on_ud"] += (eng["verdict"] == "INDETERMINATE")
            c["bare_fabricate_on_ud"] += (not abstained)
        detail.append({"id": r["id"], "model": m, "stratum": item["stratum"], "gold": item["gold_verdict"],
                       "fw_verdict": eng["verdict"], "fw_gold_ok": fw_ok, "byte_ok": byte_ok, "hallucinated": halluc,
                       "bare_acc": bare_acc, "bare_def": round(bare_def, 3), "bare_abstained": abstained})

    print("=" * 92)
    print("  ADJ86 PILOT v2 — 2x2: {Haiku, Opus} x {bare, framework}  (pipeline order fixed: IR -> rulebook-from-IR)")
    print("=" * 92)
    print(f"  {'cell':22} {'accuracy':>10} {'defensibility':>16}   notes")
    print("-" * 92)

    def line(label, acc, accn, deftext, note=""):
        print(f"  {label:22} {f'{acc}/{accn}':>10} {deftext:>16}   {note}")

    for m in ("haiku", "opus"):
        c = cells.get(m)
        if not c:
            continue
        n = c["n"]
        line(f"BARE {m}", c["bare_acc"], n, f"{c['bare_def_sum'] / n:.2f} frac",
             f"abstains on {c['ud_n'] - c['bare_fabricate_on_ud']}/{c['ud_n']} UD")
        line(f"FRAMEWORK {m}", c["fw_acc"], n, f"{c['fw_byte']}/{n} byte-clean",
             f"INDET on {c['fw_indet_on_ud']}/{c['ud_n']} UD; halluc {c['fw_halluc']}")
    print("\n  HEADLINE — does framework-Haiku reach plain-Opus / framework-Opus?")
    h, o = cells.get("haiku", {}), cells.get("opus", {})
    if h and o:
        n = h["n"]
        print(f"    accuracy:      bare-Haiku {h['bare_acc']}/{n}  ->  FW-Haiku {h['fw_acc']}/{n}   |  plain-Opus {o['bare_acc']}/{n}  FW-Opus {o['fw_acc']}/{n}")
        print(f"    bare-defensib: Haiku {h['bare_def_sum']/n:.2f}  Opus {o['bare_def_sum']/n:.2f}   (framework defensibility = byte-clean + engine proof, not prose)")
        print(f"    framework byte-clean: Haiku {h['fw_byte']}/{n}  Opus {o['fw_byte']}/{n}  (no hallucinated slots either model)")
    json.dump({"detail": detail, "cells": {k: dict(v) for k, v in cells.items()}},
              open(os.path.join(HERE, "pilot_eval_results.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
