#!/usr/bin/env python3
"""ADJ79 — rulebook-side compact IR, closed-book, on a small model.

Same 3-kind IR as ADJ78 (FACT/UNCERTAINTY/QUESTION) but for a DERIVED RULEBOOK:
  - rule-FACT  : a conditional rule "condition -> outcome"
  - basis      : recursive sub-node giving the rule's source FROM MODEL MEMORY,
                 tagged authenticity=claimed_from_model_memory (ADJ70 lower-trust),
                 because a 0.5B cannot web-search -- honest provenance class.
  - UNCERTAINTY: where the rules vary / the model is unsure
  - QUESTION   : what must be asked about a specific case to apply the rules

Each model call is a single natural question (ADJ77 principle); the framework owns
typing + recursion + authenticity tagging.
"""
import json
import os
import re
import sys
import urllib.request

GEN = "http://127.0.0.1:11434/api/generate"
DOMAINS = {
    "voting": "the requirements for a person to be eligible to vote in United States federal elections",
    "library": "the rules a public library uses to decide overdue book fines",
}


def gen(model, prompt, npred=220, timeout=120):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def lines(t):
    out = []
    for ln in t.splitlines():
        ln = re.sub(r"^\s*[-*\d.):]+\s*", "", ln).strip().strip(" .")
        if len(ln) > 4 and not ln.lower().startswith(("here", "the following", "rules", "sure", "certainly")):
            out.append(ln)
    return out


def build_rulebook_ir(model, domain):
    nodes = []
    # 1. RULES (conditional FACTs), closed-book
    rules_out = gen(model, (f"From your own knowledge, list the rules that determine {domain}. "
                            "One rule per line. Write each rule as: condition -> outcome."))
    rules = lines(rules_out)
    for r in rules:
        # 2. BASIS (recursive, closed-book) -- one atomic natural ask per rule
        basis = gen(model, (f"Rule: \"{r}\"\n\nIn one short line, state the source or authority this "
                            "rule comes from, as best you recall. If you are not sure of a specific "
                            "source, say 'general knowledge'."), npred=50).strip().splitlines()
        basis = basis[0] if basis else "general knowledge"
        nodes.append({"kind": "FACT", "subtype": "rule", "text": r,
                      "basis": basis, "authenticity": "claimed_from_model_memory"})
    # 3. UNCERTAINTIES
    for u in lines(gen(model, (f"About the rules for {domain}, list anything that varies by "
                               "jurisdiction, has exceptions, or that you are unsure about. One per line."))):
        nodes.append({"kind": "UNCERTAINTY", "text": u})
    # 4. QUESTIONS (what to ask about a specific case)
    for q in lines(gen(model, (f"To apply the rules for {domain} to a specific person/case, what "
                               "questions would you need answered? One question per line."))):
        nodes.append({"kind": "QUESTION", "text": q})
    return nodes


def main():
    model = sys.argv[1] if len(sys.argv) > 1 else "qwen2.5:0.5b"
    res = {}
    for name, domain in DOMAINS.items():
        nodes = build_rulebook_ir(model, domain)
        rules = [n for n in nodes if n.get("subtype") == "rule"]
        # crude conditional-structure check: does the rule express a condition->outcome?
        conditional = sum(1 for r in rules if re.search(r"->|→|\bif\b|\bwhen\b|\bmust\b|\brequire", r["text"], re.I))
        res[name] = {"counts": {k: sum(1 for n in nodes if n["kind"] == k) for k in ["FACT", "UNCERTAINTY", "QUESTION"]},
                     "n_rules": len(rules), "conditional_rules": conditional,
                     "all_rule_provenance": "claimed_from_model_memory"}
        print(f"\n=== {name}: {domain[:50]}... ===")
        print(f"  counts={res[name]['counts']}  conditional-structured rules={conditional}/{len(rules)}")
        print(f"  (all rule provenance = claimed_from_model_memory; needs spider-grounding on an online/capable model)")
        for n in nodes:
            if n.get("subtype") == "rule":
                print(f"    RULE: {n['text'][:60]!r}  <= basis: {n['basis'][:34]!r}")
            else:
                print(f"    {n['kind']}: {n['text'][:60]!r}")
    json.dump(res, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), f"rb_{model.replace(':','_')}.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
