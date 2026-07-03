#!/usr/bin/env python3
"""ADJ74 — atomic staging for small models.

ADJ73 found a capability floor: small models (<=1.5b) fail the monolithic
justified-discards contract because the single giant instruction is too much.
Hypothesis (Adhithya): decompose the contract into ONE instruction per turn,
building context incrementally, and a weak model clears the floor.

Three arms, same buried-override items as ADJ73:
  bare        - just answer (1 call)
  monolithic  - full justified-discards contract in one prompt (1 call)
  staged      - 5 atomic turns building context (5 calls, /api/chat):
      T1 segment passage into statements (verbatim)
      T2 mark each [USE]/[DISCARD] for the question
      T3 justify every [DISCARD]
      T4 quote the exact words that determine the answer (byte-anchor)
      T5 commit FINAL ANSWER

Re-uses the style-robust scorer from ADJ73.
"""

import json
import os
import re
import sys
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ITEMS = os.path.join(HERE, "..", "adj73-omission-ablation", "items.json")
GEN = "http://127.0.0.1:11434/api/generate"
CHAT = "http://127.0.0.1:11434/api/chat"
MODELS = ["qwen2.5:0.5b", "qwen2.5:1.5b", "qwen2.5:3b"]

# ---- style-robust scorer (copied from ADJ73 rescore.py) ----
ZERO_TOKENS = {"0", "none", "free", "exempt", "waived", "no fine", "no tax",
               "no discount", "no fee", "no charge", "no return", "cannot",
               "not eligible", "no", "zero"}
NEG_CUE = re.compile(
    r"(\bno\b\s+\w+|\bnone\b|\bfree\b|\bexempt\b|\bwaived\b|\bcannot\b|"
    r"\bnot\s+(eligible|able|allowed|permitted|charged|applicable|returnable)|"
    r"\bzero\b|\$?\s*0(?:\.0+)?\b|\bdoes not\b|\bno further\b)", re.IGNORECASE)


def is_num(t):
    return bool(re.fullmatch(r"\d+(?:\.\d+)?", t))


def norm_nums(s):
    out = set()
    for m in re.findall(r"\d+(?:\.\d+)?", s):
        out.add(m)
        if float(m) == int(float(m)):
            out.add(str(int(float(m))))
    return out


def has_zero(tokens):
    return any(t.lower() in ZERO_TOKENS or t.lower().startswith("no ") for t in tokens)


def hit(answer, tokens):
    a = answer.lower(); anums = norm_nums(a)
    for t in tokens:
        if is_num(t):
            tn = str(int(float(t))) if float(t) == int(float(t)) else t
            if t in anums or tn in anums:
                return True
        elif t.lower() in a:
            return True
    return False


def score(item, answer):
    acc, trap = item.get("accept", []), item.get("trap", [])
    a, t = hit(answer, acc), hit(answer, trap)
    if not a and not t and NEG_CUE.search(answer or ""):
        if has_zero(acc):
            a = True
        elif has_zero(trap):
            t = True
    if item["stratum"] == "PS":
        return "correct" if a else ("skim" if t else "other")
    return "abstain" if a else ("fabricate" if t else "other")


def final_answer(text):
    m = list(re.finditer(r"final answer\s*:\s*(.+)", text, re.IGNORECASE))
    if m:
        return m[-1].group(1).strip()
    return (text.strip().splitlines() or [""])[-1].strip()


def gen(model, prompt, timeout=120):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def chat(model, messages, timeout=120):
    body = json.dumps({"model": model, "messages": messages, "stream": False,
                       "options": {"temperature": 0, "seed": 0}}).encode()
    req = urllib.request.Request(CHAT, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["message"]["content"]


BARE = ("Read the passage and answer the question.\n\nPassage: {p}\n\nQuestion: {q}\n\n"
        "End with a line exactly:\nFINAL ANSWER: <answer>")
MONO = ("Read the passage. List every clause; mark each [USE] or [DISCARD] for the question; "
        "for EVERY [DISCARD] give a specific reason it does not apply; then answer.\n\n"
        "Passage: {p}\n\nQuestion: {q}\n\nEnd with a line exactly:\nFINAL ANSWER: <answer>")
STAGES = [
    "Read this passage:\n\n{p}\n\nList every distinct statement in the passage, one per line, copied word-for-word.",
    "The question is: {q}\nFor each statement you listed, mark it [USE] or [DISCARD] for answering this question.",
    "For every statement you marked [DISCARD], state a specific reason why it does not apply to this question.",
    "Quote the exact words from the passage that determine the answer to the question.",
    "Now give the answer. End with a line exactly:\nFINAL ANSWER: <answer>",
]


def run_staged(model, item):
    msgs = []
    last = ""
    for i, stage in enumerate(STAGES):
        content = stage.format(p=item["passage"], q=item["question"])
        msgs.append({"role": "user", "content": content})
        last = chat(model, msgs)
        msgs.append({"role": "assistant", "content": last})
    return last


def main():
    items = json.load(open(ITEMS))["items"]
    models = sys.argv[1].split(",") if len(sys.argv) > 1 else MODELS
    rows = []
    t0 = time.time()
    n = 0
    total = len(models) * len(items) * 3
    for model in models:
        for item in items:
            for arm in ["bare", "monolithic", "staged"]:
                n += 1
                try:
                    if arm == "bare":
                        resp = gen(model, BARE.format(p=item["passage"], q=item["question"]))
                    elif arm == "monolithic":
                        resp = gen(model, MONO.format(p=item["passage"], q=item["question"]))
                    else:
                        resp = run_staged(model, item)
                    ans = final_answer(resp)
                    cls = score(item, ans)
                except Exception as e:  # noqa
                    resp, ans, cls = f"ERROR:{e}", "", "error"
                rows.append({"model": model, "arm": arm, "id": item["id"],
                             "stratum": item["stratum"], "final_answer": ans,
                             "class": cls, "raw": resp})
                print(f"[{n}/{total}] {model} {arm} {item['id']}: {cls} ({ans[:40]})", flush=True)
    json.dump(rows, open(os.path.join(HERE, "results.json"), "w"), indent=2)

    def rate(model, arm, stratum, cls):
        sub = [r for r in rows if r["model"] == model and r["arm"] == arm and r["stratum"] == stratum]
        return sum(1 for r in sub if r["class"] == cls) / len(sub) if sub else None

    f = lambda x: f"{x:.2f}" if x is not None else "  -"
    print("\n" + "=" * 78)
    print("PS accuracy (override-correct) | PS skim-rate")
    print("=" * 78)
    print(f"{'model':14} {'bare':>6} {'mono':>6} {'staged':>7}   {'skim_bare':>9} {'skim_mono':>9} {'skim_stg':>9}")
    for m in models:
        acc = [rate(m, a, "PS", "correct") for a in ["bare", "monolithic", "staged"]]
        sk = [rate(m, a, "PS", "skim") for a in ["bare", "monolithic", "staged"]]
        print(f"{m:14} {f(acc[0]):>6} {f(acc[1]):>6} {f(acc[2]):>7}   {f(sk[0]):>9} {f(sk[1]):>9} {f(sk[2]):>9}")
    print("\nAB accuracy (abstained):")
    for m in models:
        ab = [rate(m, a, "AB", "abstain") for a in ["bare", "monolithic", "staged"]]
        print(f"{m:14} bare={f(ab[0])} mono={f(ab[1])} staged={f(ab[2])}")
    print(f"\nelapsed {time.time()-t0:.0f}s, {total} units")


if __name__ == "__main__":
    main()
