#!/usr/bin/env python3
"""ADJ73 omission ablation. For each (model x condition x item) calls Ollama at
temperature 0, parses the committed FINAL ANSWER, and scores.

PS (present-but-skimmed): correct = override answer; skim = general-rule trap.
AB (absent): correct = abstain; fabricate = supplies an uncovered value.

Conditions differ ONLY in the prompt:
  bare      - just answer
  coverage  - list clauses, [USE]/[DISCARD], NO reason for discards
  justified - list clauses, [USE]/[DISCARD], every [DISCARD] needs a reason
"""

import json
import os
import re
import sys
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
OLLAMA = "http://127.0.0.1:11434/api/generate"
MODELS = ["qwen2.5:1.5b", "qwen2.5:3b", "gemma4:latest", "llama3.1:8b"]

PROMPTS = {
    "bare": (
        "Read the passage and answer the question.\n\n"
        "Passage: {passage}\n\nQuestion: {question}\n\n"
        "End your response with a line exactly of the form:\nFINAL ANSWER: <your answer>"
    ),
    "coverage": (
        "Read the passage. First list every clause/statement in the passage; for each, "
        "mark [USE] or [DISCARD] for answering the question. You do NOT need to explain "
        "discards. Then answer.\n\n"
        "Passage: {passage}\n\nQuestion: {question}\n\n"
        "End your response with a line exactly of the form:\nFINAL ANSWER: <your answer>"
    ),
    "justified": (
        "Read the passage. First list every clause/statement in the passage; for each, "
        "mark [USE] or [DISCARD] for answering the question. For EVERY clause you mark "
        "[DISCARD], you MUST give a specific reason why that clause does not apply to "
        "this question. Then answer.\n\n"
        "Passage: {passage}\n\nQuestion: {question}\n\n"
        "End your response with a line exactly of the form:\nFINAL ANSWER: <your answer>"
    ),
}


def call(model, prompt, timeout=180):
    body = json.dumps({
        "model": model, "prompt": prompt, "stream": False,
        "options": {"temperature": 0, "seed": 0},
    }).encode()
    req = urllib.request.Request(OLLAMA, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def final_answer(text):
    m = list(re.finditer(r"final answer\s*:\s*(.+)", text, re.IGNORECASE))
    if m:
        return m[-1].group(1).strip()
    return text.strip().splitlines()[-1] if text.strip() else ""


def nums(s):
    return set(re.findall(r"\d+(?:\.\d+)?", s))


def is_num(t):
    return bool(re.fullmatch(r"\d+(?:\.\d+)?", t))


def hit(answer, tokens):
    a = answer.lower()
    anums = nums(a)
    for t in tokens:
        if is_num(t):
            if t in anums:
                return True
        elif t.lower() in a:
            return True
    return False


def score(item, answer):
    acc, trap = item.get("accept", []), item.get("trap", [])
    a_hit, t_hit = hit(answer, acc), hit(answer, trap)
    if item["stratum"] == "PS":
        if a_hit:
            return "correct"
        if t_hit:
            return "skim"
        return "other"
    else:  # AB
        if a_hit:  # accept list = abstain markers
            return "abstain"
        if t_hit:
            return "fabricate"
        return "other"


def main():
    items = json.load(open(os.path.join(HERE, "items.json")))["items"]
    models = sys.argv[1].split(",") if len(sys.argv) > 1 else MODELS
    rows = []
    total = len(models) * len(PROMPTS) * len(items)
    n = 0
    t0 = time.time()
    for model in models:
        for cond, tmpl in PROMPTS.items():
            for item in items:
                n += 1
                prompt = tmpl.format(passage=item["passage"], question=item["question"])
                try:
                    resp = call(model, prompt)
                    ans = final_answer(resp)
                    cls = score(item, ans)
                except Exception as e:  # noqa
                    resp, ans, cls = f"ERROR: {e}", "", "error"
                rows.append({
                    "model": model, "condition": cond, "id": item["id"],
                    "stratum": item["stratum"], "final_answer": ans, "class": cls,
                    "raw": resp,
                })
                print(f"[{n}/{total}] {model} {cond} {item['id']}: {cls}  ({ans[:50]})",
                      flush=True)
    json.dump(rows, open(os.path.join(HERE, "results_raw.json"), "w"), indent=2)

    # Summary
    def acc(model, cond, stratum):
        sub = [r for r in rows if r["model"] == model and r["condition"] == cond and r["stratum"] == stratum]
        if not sub:
            return None
        good = "correct" if stratum == "PS" else "abstain"
        return sum(1 for r in sub if r["class"] == good) / len(sub)

    print("\n" + "=" * 78)
    print("ACCURACY  (PS = override-correct ; AB = abstained)")
    print("=" * 78)
    print(f"{'model':16} {'stratum':8} {'bare':>8} {'coverage':>10} {'justified':>10}")
    for model in models:
        for stratum in ["PS", "AB"]:
            b, c, j = acc(model, "bare", stratum), acc(model, "coverage", stratum), acc(model, "justified", stratum)
            fmt = lambda x: f"{x:.2f}" if x is not None else "  -"
            print(f"{model:16} {stratum:8} {fmt(b):>8} {fmt(c):>10} {fmt(j):>10}")

    # PS skim-rate + AB fabricate-rate
    print("\nPS skim-trap rate (lower=better) / AB fabricate rate (lower=better):")
    for model in models:
        for cond in PROMPTS:
            ps = [r for r in rows if r["model"] == model and r["condition"] == cond and r["stratum"] == "PS"]
            ab = [r for r in rows if r["model"] == model and r["condition"] == cond and r["stratum"] == "AB"]
            skim = sum(1 for r in ps if r["class"] == "skim") / len(ps) if ps else 0
            fab = sum(1 for r in ab if r["class"] == "fabricate") / len(ab) if ab else 0
            print(f"  {model:16} {cond:10} skim={skim:.2f}  fabricate={fab:.2f}")

    print(f"\nelapsed {time.time()-t0:.0f}s, {total} generations")
    json.dump(rows, open(os.path.join(HERE, "results_raw.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
