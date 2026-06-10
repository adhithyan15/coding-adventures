#!/usr/bin/env python3
"""E2 correctability — assemble the wrong-answer cell set and apply the
format-confound guard.

We reuse the solutions ADJ99 already generated (same solver, same question, four
arms). For E2 we take the matched **both-arms-wrong** items so every case has a
real error to localize in *each* arm, and the comparison is the artifact, not
which arm happened to be right.

The format-confound guard (the methodological upgrade over ADJ96): we strip the
*citation chrome* that let a regex identify the arm with 100% accuracy in ADJ99,
while preserving the *stepwise decomposition* that is the framework's actual
treatment. Then we measure the residual regex separability and report it openly.

Outputs:
  cells/cell_NNNN.json  — blind, exactly what the auditor sees (normalized only)
  items_e2.json         — private map (cell -> arm/scale/gold/raw), never shown
  leak_check.json       — deterministic regex arm-separation on normalized text

Run: python3 build_items.py
"""
import glob
import json
import os
import re
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ADJ99 = os.path.abspath(os.path.join(HERE, "..", "adj99-hle100-run"))
CELLS_DIR = os.path.join(HERE, "cells")
ORACLE_DIR = os.path.join(HERE, "oracle")
os.makedirs(CELLS_DIR, exist_ok=True)
os.makedirs(ORACLE_DIR, exist_ok=True)

# --- format normalizer (copied verbatim from adj99 rescore_prep.py so importing
#     it does not re-run that module's file-writing side effects) ---------------
SECTION_HEADERS = [
    "RETRIEVED FACTS (CAS):", "RETRIEVED FACTS:", "RETRIEVED FACTS",
    "REASONING CHAIN:", "REASONING CHAIN", "REASONING:", "ANSWER:", "CONCLUSION:",
]


def _strip_balanced(text, opener):
    """Remove every `opener ... )` span, honoring nested parentheses."""
    out, i, n, olen = [], 0, len(text), len(opener)
    while i < n:
        if text.startswith(opener, i):
            depth, j = 0, i
            while j < n:
                if text[j] == "(":
                    depth += 1
                elif text[j] == ")":
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
            i = j
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


def normalize(reasoning, answer):
    """Style-neutral `REASONING: ... \n\nCONCLUSION: ...` rendering. Strips
    citation chrome (src parentheticals, [cites] markers, [n] fact enumerators,
    ALL-CAPS section headers) but keeps the stepwise `(1) (2)` decomposition."""
    t = reasoning or ""
    t = _strip_balanced(t, "(src:")
    t = re.sub(r"\[cites:[^\]]*\]", "", t)
    t = re.sub(r"(?m)^\s*\[\d+\]\s*", "", t)
    t = "\n".join(ln for ln in t.splitlines() if ln.strip() not in SECTION_HEADERS)
    t = re.sub(r"\n{3,}", "\n\n", t).strip()
    a = re.sub(r"^\s*ANSWER:\s*", "", (answer or "").strip())
    return "REASONING:\n{}\n\nCONCLUSION:\n{}".format(t if t else "(none provided)", a)


# --- load ADJ99 solutions ----------------------------------------------------
items_meta = {i["id"]: i for i in json.load(open(os.path.join(ADJ99, "items_100.json")))}
items99 = []
for bf in sorted(f for f in glob.glob(os.path.join(ADJ99, "batches", "batch_*.json"))
                 if "degraded" not in f):
    items99 += json.load(open(bf))["result"]["items"]


def reasoning_of(arm, d):
    return d.get("trail") if arm.startswith("fw-") else d.get("work")


def valid(d):
    a, r = str(d.get("answer", "")), str(reasoning_of("fw-", d) if "trail" in d else d.get("work", ""))
    return not (a.startswith("[agent-error]") or (d.get("trail") == "[agent-error]"))


def both_wrong(it, scale):
    fw, pl = it["arms"].get(f"fw-{scale}"), it["arms"].get(f"plain-{scale}")
    if not (fw and pl):
        return False
    okfw = not str(fw.get("answer", "")).startswith("[agent-error]") and fw.get("trail") != "[agent-error]"
    okpl = not str(pl.get("answer", "")).startswith("[agent-error]") and str(pl.get("work", "")) != "[agent-error]"
    return okfw and okpl and fw.get("accuracy") == "incorrect" and pl.get("accuracy") == "incorrect"


def stratified(scale, per_cat):
    """Deterministic: both-wrong items, sorted by id, first `per_cat` per category."""
    by_cat = defaultdict(list)
    for it in sorted(items99, key=lambda x: x["id"]):
        if both_wrong(it, scale):
            by_cat[it["category"]].append(it)
    chosen = []
    for cat in sorted(by_cat):
        chosen += by_cat[cat][:per_cat]
    return chosen


cells = []
idx = 0


def add_cells(it, scale):
    global idx
    for arm in (f"fw-{scale}", f"plain-{scale}"):
        d = it["arms"][arm]
        ans = d.get("answer", "")
        raw = reasoning_of(arm, d) or ""
        cells.append({
            "idx": idx, "item_id": it["id"], "category": it["category"],
            "scale": scale, "arm": "framework" if arm.startswith("fw-") else "plain",
            "question": items_meta[it["id"]]["question"], "gold": items_meta[it["id"]]["answer"],
            "answer": ans, "reasoning_raw": raw, "normalized": normalize(raw, ans),
        })
        idx += 1


for it in stratified("haiku", 4):   # primary: 4/category both-wrong Haiku items
    add_cells(it, "haiku")
HAIKU_ITEMS = idx // 2
for it in stratified("opus", 2):    # robustness: 2/category both-wrong Opus items
    add_cells(it, "opus")

# --- emit blind cell files (only what the auditor may see) -------------------
for c in cells:
    blind = {"idx": c["idx"], "category": c["category"],
             "question": c["question"], "normalized": c["normalized"]}
    json.dump(blind, open(os.path.join(CELLS_DIR, "cell_%04d.json" % c["idx"]), "w"),
              ensure_ascii=False, indent=1)
    # gold-aware oracle input (NEVER shown to the blind auditor; read by the
    # post-audit oracle+score stage only).
    orc = {"idx": c["idx"], "question": c["question"], "gold": c["gold"],
           "answer": c["answer"], "reasoning_raw": c["reasoning_raw"]}
    json.dump(orc, open(os.path.join(ORACLE_DIR, "cell_%04d.json" % c["idx"]), "w"),
              ensure_ascii=False, indent=1)
json.dump(cells, open(os.path.join(HERE, "items_e2.json"), "w"), ensure_ascii=False, indent=1)

# --- leak-check: can a regex separate arms on the NORMALIZED text? -----------
# Chrome tells (should be gone). Structure tells (the treatment; may remain).
CHROME = ["RETRIEVED FACTS", "REASONING CHAIN", "[cites", "(src:"]
STRUCT = re.compile(r"(?m)^\s*\(\d+\)\s")  # `(1) ` stepwise markers


def chrome_tell(t):
    return any(m in t for m in CHROME)


def struct_tell(t):
    return bool(STRUCT.search(t))


def classifier_acc(feature):
    """Predict framework iff feature present; accuracy vs true arm."""
    ok = tot = 0
    for c in cells:
        pred = "framework" if feature(c["normalized"]) else "plain"
        ok += (pred == c["arm"])
        tot += 1
    return ok / tot if tot else 0.0


leak = {
    "n_cells": len(cells),
    "haiku_items_both_wrong_selected": HAIKU_ITEMS,
    "opus_items_both_wrong_selected": (idx // 2) - HAIKU_ITEMS,
    "chrome_tell_present_on_normalized": sum(chrome_tell(c["normalized"]) for c in cells),
    "regex_acc_chrome": round(classifier_acc(chrome_tell), 4),
    "regex_acc_structure": round(classifier_acc(struct_tell), 4),
    "note": ("chrome accuracy ~0.5 confirms the citation-format confound is removed; any "
             "structure accuracy above chance is the stepwise-decomposition TREATMENT, not "
             "chrome — reported openly, not hidden."),
}
json.dump(leak, open(os.path.join(HERE, "leak_check.json"), "w"), indent=1)

print("cells:", len(cells), "| haiku both-wrong items:", HAIKU_ITEMS,
      "| opus both-wrong items:", (idx // 2) - HAIKU_ITEMS)
print("leak-check:", json.dumps(leak, indent=1))
