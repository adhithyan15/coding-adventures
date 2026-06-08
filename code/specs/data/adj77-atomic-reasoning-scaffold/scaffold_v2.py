#!/usr/bin/env python3
"""ADJ77 scaffold-v2: NATURAL atomic steps (no rigid tags).

Diagnosis from v1: the 0.5b reasons fine in OPEN natural form (the case-facts call
accidentally solved items) but chokes on STRUCTURED tagged output ("APPLIES: yes |
VALUE:" -> "NO"). So v2 uses two NATURAL calls, no schema:
  1. FOCUS  — "describe the specific category/characteristics of the subject"
              (directs attention to the subject's distinguishing attributes)
  2. ANSWER — re-ask with the focus prepended + an instruction that an exception
              overrides the general rule.
Framework parses only FINAL ANSWER (lenient). Test: does this fix the NUMERIC
present-but-skimmed items (where bare skims), not just default to 'none'?
"""
import json
import os
import re
import sys
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from scaffold import score, gen  # reuse robust scorer + ollama call

ITEMS = os.path.join(HERE, "..", "adj73-omission-ablation", "items.json")


def fa(t):
    m = list(re.finditer(r"final answer\s*:\s*(.+)", t, re.I))
    return m[-1].group(1).strip() if m else (t.strip().splitlines() or [""])[-1]


def bare(model, it):
    p = (f"Read the passage and answer the question.\n\nPassage: {it['passage']}\n\n"
         f"Question: {it['question']}\n\nEnd with a line exactly:\nFINAL ANSWER: <answer>")
    return fa(gen(model, p, npred=150))


def v2(model, it):
    focus_prompt = (f"Passage: {it['passage']}\n\nQuestion: {it['question']}\n\n"
                    "In one line, describe only the specific category or characteristics of the "
                    "SUBJECT of the question (e.g. its type, status, or group). Do not answer the "
                    "question yet.")
    focus = gen(model, focus_prompt, npred=50).strip().splitlines()[0]
    answer_prompt = (f"{it['passage']}\n\nThe subject of the question has these characteristics: {focus}\n\n"
                     "An exception or special rule for that kind of subject OVERRIDES the general rule. "
                     "Use the rule that specifically applies to this subject.\n"
                     f"Question: {it['question']}\n\nEnd with a line exactly:\nFINAL ANSWER: <answer>")
    return fa(gen(model, answer_prompt, npred=150)), focus


def main():
    items = [i for i in json.load(open(ITEMS))["items"] if i["stratum"] == "PS"]
    model = sys.argv[1] if len(sys.argv) > 1 else "qwen2.5:0.5b"
    nb = nv = 0
    rows = []
    for it in items:
        b = bare(model, it); bc = score(it, b)
        v, focus = v2(model, it); vc = score(it, v)
        nb += bc == "correct"; nv += vc == "correct"
        rows.append({"id": it["id"], "accept": it["accept"], "bare": b, "bare_cls": bc,
                     "v2": v, "v2_cls": vc, "focus": focus})
        print(f"{it['id']}: bare={bc:7} v2={vc:7} | accept={it['accept']} | v2_ans={v[:32]!r}", flush=True)
    json.dump(rows, open(os.path.join(HERE, f"v2_{model.replace(':','_')}.json"), "w"), indent=2)
    print(f"\n{model}  PS accuracy: bare={nb}/{len(items)}={nb/len(items):.2f}  v2={nv}/{len(items)}={nv/len(items):.2f}")
    # numeric-only subset (where 'none' default can't accidentally score)
    numeric = [i for i in range(len(items)) if all(re.fullmatch(r'\d+(?:\.\d+)?', a) for a in items[i]['accept'])]
    if numeric:
        nbn = sum(1 for i in numeric if rows[i]['bare_cls'] == 'correct')
        nvn = sum(1 for i in numeric if rows[i]['v2_cls'] == 'correct')
        print(f"  NUMERIC-only ({len(numeric)} items): bare={nbn}/{len(numeric)}  v2={nvn}/{len(numeric)}")


if __name__ == "__main__":
    main()
