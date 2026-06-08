#!/usr/bin/env python3
"""ADJ76 — WHY does qwen2.5:0.5b crap out on the monolithic contract?

(1) Failure taxonomy of the existing ADJ74 monolithic 0.5b PS raws.
(2) Truncation control: re-run monolithic at increasing token budgets; if accuracy
    stays ~0, the failure is cognitive (instruction overload), not a token-budget
    artifact.
"""
import json
import os
import re
import urllib.request
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
items = {i["id"]: i for i in json.load(open(os.path.join(HERE, "..", "adj73-omission-ablation", "items.json")))["items"]}
rows = json.load(open(os.path.join(HERE, "..", "adj74-atomic-staging", "results.json")))
mono = [r for r in rows if r["model"] == "qwen2.5:0.5b" and r["arm"] == "monolithic" and r["stratum"] == "PS"]


def classify(r):
    fa, raw = r["final_answer"], r["raw"]
    if r["class"] == "correct":
        return "correct"
    if re.search(r"\[?(DISCARD|USE|FIXED|KEEP)\]?", fa, re.I) and not re.search(r"\d", fa):
        return "answered_with_tag"
    if "final answer" not in raw.lower():
        return "no_final_answer"
    if r["class"] == "skim":
        return "applied_general_rule(skim)"
    return "other_nonanswer"


tax = Counter(classify(r) for r in mono)
invented = sum(1 for r in mono if re.search(r"\[(FIXED|KEEP|RELEVANT|IGNORE|NOTE)\]", r["raw"], re.I))
print("=== monolithic 0.5b PS failure taxonomy (n=12) ===")
for k, v in tax.most_common():
    print(f"  {k}: {v}")
print(f"  (outputs that invented a non-spec tag like [FIXED]: {invented})")

GEN = "http://127.0.0.1:11434/api/generate"
MONO = ("Read the passage. List every clause; mark each [USE] or [DISCARD] for the question; "
        "for EVERY [DISCARD] give a specific reason it does not apply; then answer.\n\n"
        "Passage: {p}\n\nQuestion: {q}\n\nEnd with a line exactly:\nFINAL ANSWER: <answer>")
ZERO = {"0", "none", "free", "exempt", "waived", "no", "cannot", "not eligible", "zero"}
NEG = re.compile(r"(\bno\b\s+\w+|\bnone\b|\bfree\b|\bexempt\b|\bwaived\b|\bcannot\b|"
                 r"\bnot\s+(eligible|applicable|charged)|\bzero\b|\$?\s*0(?:\.0+)?\b|\bdoes not\b)", re.I)


def nums(s):
    o = set()
    for m in re.findall(r"\d+(?:\.\d+)?", s):
        o.add(m)
        if float(m) == int(float(m)):
            o.add(str(int(float(m))))
    return o


def hit(a, toks):
    al = a.lower(); an = nums(al)
    for t in toks:
        if re.fullmatch(r"\d+(?:\.\d+)?", t):
            tn = str(int(float(t))) if float(t) == int(float(t)) else t
            if t in an or tn in an:
                return True
        elif t.lower() in al:
            return True
    return False


def score(it, a):
    acc, tr = it.get("accept", []), it.get("trap", [])
    ah, th = hit(a, acc), hit(a, tr)
    if not ah and not th and NEG.search(a or ""):
        if any(x.lower() in ZERO or x.lower().startswith("no ") for x in acc):
            ah = True
        elif any(x.lower() in ZERO or x.lower().startswith("no ") for x in tr):
            th = True
    return "correct" if ah else ("skim" if th else "other")


def fa(t):
    m = list(re.finditer(r"final answer\s*:\s*(.+)", t, re.I))
    return m[-1].group(1).strip() if m else (t.strip().splitlines() or [""])[-1]


def gen(prompt, npred):
    body = json.dumps({"model": "qwen2.5:0.5b", "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.loads(r.read())["response"]


print("\n=== truncation control: monolithic 0.5b PS at increasing token budget ===")
for npred in [128, 512, 1024]:
    correct = 0
    for r in mono:
        it = items[r["id"]]
        resp = gen(MONO.format(p=it["passage"], q=it["question"]), npred)
        if score(it, fa(resp)) == "correct":
            correct += 1
    print(f"  num_predict={npred:4}: PS accuracy = {correct/len(mono):.2f}  ({correct}/{len(mono)})")
