#!/usr/bin/env python3
"""ADJ77 — framework-level atomic reasoning scaffold for small local models.

Diagnosis (ADJ76): small models fail the monolithic byte-accounting contract via
multi-objective instruction overload, NOT truncation; free-form staging (ADJ74)
drifts on 1.5b. Solution under test: the FRAMEWORK owns the control flow and
aggregation; each MODEL call is a single atomic judgment a tiny model can do.

Pipeline (framework-controlled):
  1. SEGMENT       — deterministic sentence split (framework, no model)
  2. CASE FACTS    — 1 atomic model call: extract the case facts for the question
  3. PER-CLAUSE    — N atomic model calls: "does THIS clause apply to THESE facts,
                     and what value?" (single objective, single clause)
  4. RESOLVE       — deterministic: among applies+value clauses, pick the most
                     specific (max overlap with case facts); atomic tiebreak only
                     if needed. The model never juggles clauses; the engine
                     integrates.

Compared against bare (and ADJ74's monolithic/free-staged numbers).
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

# ---- robust scorer (from ADJ73) ----
ZERO = {"0", "none", "free", "exempt", "waived", "no", "cannot", "not eligible", "zero"}
NEG = re.compile(r"(\bno\b\s+\w+|\bnone\b|\bfree\b|\bexempt\b|\bwaived\b|\bcannot\b|"
                 r"\bnot\s+(eligible|applicable|charged|returnable)|\bzero\b|\$?\s*0(?:\.0+)?\b|\bdoes not\b)", re.I)


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


def gen(model, prompt, npred=200, timeout=120):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


STOP = set("a an the to of for and or in on at is are be by with this that these those "
           "all any each per from as it its their his her they who whom which what when "
           "however instead also may must can will would receive accrue apply applies "
           "rule statement passage question answer days dollars percent mph minutes year "
           "years day hours hour fee charge rate limit standard general".split())


def content_words(s):
    return {w for w in re.findall(r"[a-z0-9]+", s.lower()) if w not in STOP and len(w) > 2}


def bare(model, item):
    p = (f"Read the passage and answer the question.\n\nPassage: {item['passage']}\n\n"
         f"Question: {item['question']}\n\nEnd with a line exactly:\nFINAL ANSWER: <answer>")
    r = gen(model, p)
    m = list(re.finditer(r"final answer\s*:\s*(.+)", r, re.I))
    return m[-1].group(1).strip() if m else (r.strip().splitlines() or [""])[-1]


def scaffold(model, item):
    # 1. SEGMENT (deterministic)
    sents = [s.strip() for s in re.split(r"(?<=[.!?])\s+", item["passage"]) if s.strip()]
    # 2. CASE FACTS (1 atomic call)
    cf_prompt = (f"Passage: {item['passage']}\n\nQuestion: {item['question']}\n\n"
                 "In one short line, state only the specific facts about the subject of the "
                 "question (the entity/item/case being asked about). Do not state any rules.")
    case_facts = gen(model, cf_prompt, npred=60).strip().splitlines()[0] if True else ""
    cf_words = content_words(case_facts) | content_words(item["question"])
    # 3. PER-CLAUSE atomic applies+value
    applies = []
    for s in sents:
        q = (f"Case facts: {case_facts}\n\nStatement: \"{s}\"\n\n"
             "Does this statement state a RULE whose conditions are satisfied by the case facts? "
             "Reply on one line exactly:\nAPPLIES: yes or no | VALUE: <the value/outcome it specifies, or none>")
        out = gen(model, q, npred=60)
        a = re.search(r"applies\s*:\s*(yes|no)", out, re.I)
        v = re.search(r"value\s*:\s*(.+)", out, re.I)
        if a and a.group(1).lower() == "yes" and v:
            val = v.group(1).strip().strip("|").strip()
            spec = len(content_words(s) & cf_words)
            applies.append({"clause": s, "value": val, "spec": spec})
    # 4. RESOLVE (deterministic specificity; atomic tiebreak if needed)
    if not applies:
        return "none / not specified"
    applies.sort(key=lambda x: -x["spec"])
    if len(applies) == 1 or applies[0]["spec"] > applies[1]["spec"]:
        return applies[0]["value"]
    # tiebreak: one atomic call
    top = [x for x in applies if x["spec"] == applies[0]["spec"]][:3]
    opts = "\n".join(f"({chr(65+i)}) rule: \"{x['clause']}\" -> value: {x['value']}" for i, x in enumerate(top))
    tb = (f"Case facts: {case_facts}\n\nThese rules all match. Which ONE is the most specific "
          f"to the case (the exception that overrides the general rule)?\n{opts}\n\n"
          "Reply on one line exactly:\nCHOICE: <letter>")
    out = gen(model, tb, npred=30)
    c = re.search(r"choice\s*:\s*([a-z])", out, re.I)
    idx = (ord(c.group(1).upper()) - 65) if c else 0
    return top[idx]["value"] if 0 <= idx < len(top) else top[0]["value"]


def main():
    items = [i for i in json.load(open(ITEMS))["items"] if i["stratum"] == "PS"]
    models = sys.argv[1].split(",") if len(sys.argv) > 1 else ["qwen2.5:0.5b", "qwen2.5:1.5b", "qwen2.5:3b"]
    rows = []
    t0 = time.time()
    for model in models:
        for arm in ["bare", "scaffold"]:
            for it in items:
                ans = bare(model, it) if arm == "bare" else scaffold(model, it)
                cls = score(it, ans)
                rows.append({"model": model, "arm": arm, "id": it["id"], "answer": ans, "class": cls})
                print(f"{model} {arm:8} {it['id']}: {cls} ({ans[:38]})", flush=True)
    json.dump(rows, open(os.path.join(HERE, "results.json"), "w"), indent=2)

    def acc(model, arm):
        sub = [r for r in rows if r["model"] == model and r["arm"] == arm]
        return sum(1 for r in sub if r["class"] == "correct") / len(sub) if sub else None
    print("\n" + "=" * 60)
    print("PS accuracy: bare vs framework atomic scaffold")
    print("=" * 60)
    for m in models:
        print(f"  {m:14} bare={acc(m,'bare'):.2f}   scaffold={acc(m,'scaffold'):.2f}")
    print(f"\nelapsed {time.time()-t0:.0f}s")


if __name__ == "__main__":
    main()
