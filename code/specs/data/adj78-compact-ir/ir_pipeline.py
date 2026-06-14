#!/usr/bin/env python3
"""ADJ78 — compact IR (FACT / UNCERTAINTY / QUESTION) a 0.5B can build in steps.

Core trick (from ADJ77): the model emits NATURAL content one kind at a time; the
FRAMEWORK assigns type (by step), provenance (string-match back to source), and
structure (accumulation). No schema, no offsets asked of the model.

Compares STEPWISE (4 natural steps + framework typing/provenance/coverage) vs a
MONOLITHIC one-shot "extract facts/uncertainties/questions as a structured list".
Metrics: coverage of source sentences, anchor-rate of stated facts (hallucination
guard), node counts.
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


def gen(model, prompt, npred=200, timeout=120):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def lines(text):
    out = []
    for ln in text.splitlines():
        ln = re.sub(r"^\s*[-*\d.):]+\s*", "", ln).strip()  # strip bullets/numbering
        ln = ln.strip(" .")
        if len(ln) > 3 and not ln.lower().startswith(("here", "the following", "facts", "questions", "uncertaint")):
            out.append(ln)
    return out


STOP = set("a an the to of for and or in on at is are be by with this that these those all any "
           "per from as it its their his her they who which what when up into not no".split())


def cwords(s):
    return {w for w in re.findall(r"[a-z0-9]+", s.lower()) if w not in STOP and len(w) > 2}


def sentences(passage):
    out = []
    pos = 0
    for m in re.finditer(r"[^.!?]+[.!?]", passage):
        s = m.group().strip()
        cs = passage.find(s, pos)
        out.append({"text": s, "span": [cs, cs + len(s)]})
        pos = cs + len(s)
    return out


def anchor(phrase, sents, thresh=0.34):
    pw = cwords(phrase)
    if not pw:
        return None, 0.0
    best, bestj = None, 0.0
    for s in sents:
        sw = cwords(s["text"])
        if not sw:
            continue
        j = len(pw & sw) / len(pw)  # fraction of phrase words found in the sentence
        if j > bestj:
            best, bestj = s, j
    return (best, bestj) if bestj >= thresh else (None, bestj)


def build_ir(model, passage):
    sents = sentences(passage)
    steps = [
        ("FACT", "stated", "List the facts that the passage states directly, one short fact per line, in plain words."),
        ("FACT", "inferred", "List anything you can reasonably infer that the passage does NOT state directly, one per line."),
        ("UNCERTAINTY", None, "List anything that is unclear, ambiguous, or missing from the passage, one per line."),
        ("QUESTION", None, "List the questions the passage is implicitly asking you to answer or decide, one per line."),
    ]
    nodes = []
    for kind, status, instr in steps:
        out = gen(model, f"Passage: {passage}\n\n{instr}", npred=200)
        for ph in lines(out):
            sp, conf = anchor(ph, sents)
            node = {"kind": kind, "status": status, "text": ph,
                    "span": sp["span"] if sp else None, "anchor_conf": round(conf, 2)}
            # hallucination guard: a STATED fact with no anchor -> demote to inferred
            if kind == "FACT" and status == "stated" and sp is None:
                node["status"] = "inferred(demoted: unanchored)"
            nodes.append(node)
    # coverage: which content sentences are touched by some anchored node?
    covered = set()
    for n in nodes:
        if n["span"]:
            for i, s in enumerate(sents):
                if not (n["span"][1] <= s["span"][0] or n["span"][0] >= s["span"][1]):
                    covered.add(i)
    return nodes, sents, covered


def monolithic_ir(model, passage):
    out = gen(model, (f"Passage: {passage}\n\nExtract a structured list with three sections: "
                      "FACTS, UNCERTAINTIES, QUESTIONS. Under each, list items one per line."), npred=400)
    return lines(out)


def main():
    model = sys.argv[1] if len(sys.argv) > 1 else "qwen2.5:0.5b"
    allrows = {}
    for name, passage in PASSAGES.items():
        nodes, sents, covered = build_ir(model, passage)
        facts = [n for n in nodes if n["kind"] == "FACT"]
        stated = [n for n in facts if n["status"] == "stated"]
        anchored_stated = [n for n in stated if n["span"]]
        cov = len(covered) / len(sents) if sents else 0
        anchor_rate = len(anchored_stated) / len(stated) if stated else float("nan")
        mono = monolithic_ir(model, passage)
        allrows[name] = {"nodes": nodes, "n_sentences": len(sents), "coverage": cov,
                         "stated_anchor_rate": anchor_rate,
                         "counts": {k: sum(1 for n in nodes if n["kind"] == k) for k in ["FACT", "UNCERTAINTY", "QUESTION"]},
                         "monolithic_lines": len(mono)}
        print(f"\n=== {name} ({len(sents)} sentences) ===")
        print(f"  STEPWISE: {allrows[name]['counts']}  coverage={cov:.2f}  stated-anchor-rate={anchor_rate:.2f}")
        print(f"  MONOLITHIC one-shot: {len(mono)} total lines (no typing/provenance)")
        for n in nodes[:8]:
            tag = f"{n['kind']}/{n['status']}" if n['status'] else n['kind']
            anc = f"@[{n['span'][0]}:{n['span'][1]}]" if n['span'] else "UNANCHORED"
            print(f"    {tag:28} {anc:14} {n['text'][:48]!r}")
    json.dump(allrows, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), f"ir_{model.replace(':','_')}.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
