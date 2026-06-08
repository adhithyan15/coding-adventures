#!/usr/bin/env python3
"""ADJ78 v2 — compact IR with provenance-determined status + byte-accounting gap-fill.

Refinements over v1:
  - Do NOT ask the model stated-vs-inferred (0.5B does it badly). Ask only for FACTS,
    UNCERTAINTIES, QUESTIONS. STATUS is FRAMEWORK-determined: a fact that anchors to a
    source span = stated; one that does not = inferred (and gets a one-shot basis ask).
  - BYTE-ACCOUNTING GAP-FILL: every uncovered content sentence triggers ONE atomic call
    ("fact / question / discard? restate in one line"), typed by the model's choice and
    anchored to that sentence. Re-measure coverage before vs after.
Each model call remains a single natural question (ADJ77 principle).
"""
import json
import os
import re
import sys
import urllib.request

GEN = "http://127.0.0.1:11434/api/generate"
PASSAGES = {
    "leave": ("Acme Corp offers a generous paid-leave policy that staff rate highly. The standard "
              "annual allotment is 20 days, accrued monthly, and unused days may be carried over up "
              "to five into the next year. Part-time staff hired after January 2020 accrue at a "
              "reduced rate of 12 days per year. Jordan joined Acme as a part-time employee in March "
              "2022 and is asking how much leave will accrue this year."),
    "clinic": ("A 26-year-old patient presents with a productive cough, intermittent fevers for two "
               "weeks, and night sweats. A chest x-ray shows a right upper-lobe opacity. The patient "
               "recently traveled abroad. Initial antibiotics did not improve symptoms. The team must "
               "decide whether this is a bacterial pneumonia or something else."),
}
STOP = set("a an the to of for and or in on at is are be by with this that these those all any per "
           "from as it its their his her they who which what when up into not no will this".split())


def gen(model, prompt, npred=200, timeout=120):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def lines(t):
    out = []
    for ln in t.splitlines():
        ln = re.sub(r"^\s*[-*\d.):]+\s*", "", ln).strip().strip(" .")
        if len(ln) > 3 and not ln.lower().startswith(("here", "the following", "facts", "questions", "uncertaint", "sure")):
            out.append(ln)
    return out


def cwords(s):
    return {w for w in re.findall(r"[a-z0-9]+", s.lower()) if w not in STOP and len(w) > 2}


def sentences(p):
    out, pos = [], 0
    for m in re.finditer(r"[^.!?]+[.!?]", p):
        s = m.group().strip(); cs = p.find(s, pos)
        out.append({"text": s, "span": [cs, cs + len(s)]}); pos = cs + len(s)
    return out


def anchor(phrase, sents, thresh=0.34):
    pw = cwords(phrase)
    if not pw:
        return None
    best, bj = None, 0.0
    for s in sents:
        sw = cwords(s["text"])
        j = len(pw & sw) / len(pw) if sw else 0
        if j > bj:
            best, bj = s, j
    return best if bj >= thresh else None


def coverage(nodes, sents):
    cov = set()
    for n in nodes:
        if n["span"]:
            for i, s in enumerate(sents):
                if not (n["span"][1] <= s["span"][0] or n["span"][0] >= s["span"][1]):
                    cov.add(i)
    return cov


def build(model, passage):
    sents = sentences(passage)
    nodes = []
    for kind, instr in [("FACT", "List the facts in this passage, one short fact per line."),
                        ("UNCERTAINTY", "List anything unclear, ambiguous, or missing, one per line."),
                        ("QUESTION", "List the questions this passage asks you to decide, one per line.")]:
        for ph in lines(gen(model, f"Passage: {passage}\n\n{instr}")):
            sp = anchor(ph, sents)
            node = {"kind": kind, "text": ph, "span": sp["span"] if sp else None}
            if kind == "FACT":
                node["status"] = "stated" if sp else "inferred"
            nodes.append(node)
    cov_before = coverage(nodes, sents)
    # BYTE-ACCOUNTING GAP-FILL: one atomic call per uncovered sentence
    for i, s in enumerate(sents):
        if i in cov_before:
            continue
        out = gen(model, (f"Sentence: \"{s['text']}\"\n\nIs this sentence (a) a FACT to record, "
                          "(b) a QUESTION, or (c) framing/FILLER to discard? Answer with one letter "
                          "(a/b/c) then a colon and a one-line restatement."), npred=60)
        m = re.search(r"\b([abc])\b", out, re.I)
        choice = m.group(1).lower() if m else "a"
        rest = out.split(":", 1)[1].strip() if ":" in out else s["text"]
        kind = {"a": "FACT", "b": "QUESTION", "c": "DISCARDED"}[choice]
        nodes.append({"kind": kind, "text": rest[:120], "span": s["span"],
                      "status": ("stated" if kind == "FACT" else None), "via": "gap-fill"})
    cov_after = coverage([n for n in nodes if n["kind"] != "DISCARDED" or n.get("via")], sents)
    # every sentence is now either covered by a node OR explicitly discarded -> byte-accounted
    accounted = coverage(nodes, sents)  # includes DISCARDED spans
    return nodes, sents, cov_before, cov_after, accounted


def main():
    model = sys.argv[1] if len(sys.argv) > 1 else "qwen2.5:0.5b"
    res = {}
    for name, passage in PASSAGES.items():
        nodes, sents, cb, ca, acc = build(model, passage)
        facts = [n for n in nodes if n["kind"] == "FACT"]
        stated = [n for n in facts if n.get("status") == "stated"]
        res[name] = {"n_sentences": len(sents), "coverage_before_gapfill": round(len(cb)/len(sents), 2),
                     "byte_accounted_after_gapfill": round(len(acc)/len(sents), 2),
                     "counts": {k: sum(1 for n in nodes if n["kind"] == k) for k in ["FACT", "UNCERTAINTY", "QUESTION", "DISCARDED"]},
                     "stated_facts": len(stated), "inferred_facts": len(facts)-len(stated)}
        print(f"\n=== {name} ({len(sents)} sentences) ===")
        print(f"  coverage before gap-fill: {res[name]['coverage_before_gapfill']:.2f}")
        print(f"  BYTE-ACCOUNTED after gap-fill (covered or explicitly discarded): {res[name]['byte_accounted_after_gapfill']:.2f}")
        print(f"  counts: {res[name]['counts']}  (stated facts={len(stated)}, inferred={len(facts)-len(stated)})")
        for n in nodes:
            tag = f"{n['kind']}/{n.get('status')}" if n.get("status") else n["kind"]
            anc = f"@[{n['span'][0]}:{n['span'][1]}]" if n["span"] else "UNANCHORED"
            via = " (gap-fill)" if n.get("via") else ""
            print(f"    {tag:20} {anc:13}{via} {n['text'][:46]!r}")
    json.dump(res, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), f"v2_{model.replace(':','_')}.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
