#!/usr/bin/env python3
"""Phase 2: assemble the GROUNDED PE corpus from the forward-grounding spider
output. Every admitted likelihood ratio carries its byte-anchored provenance
chain to primary data; ungroundable links are kept as explicit data-gaps (no
invented number), not silently dropped.

Input:  provenance/pe/grounding-results.json  (the Phase-1 workflow result)
Output: provenance/pe/pe-corpus.json          (the case-blind grounded corpus)
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
res = json.loads((HERE / "grounding-results.json").read_text())
per = res["per_finding"] if "per_finding" in res else res.get("result", {}).get("per_finding", [])

prior = None
findings = []
for r in per:
    node = {
        "id": r["id"],
        "lr": r.get("computed_lr", 0) or 0,
        "verdict": r.get("verdict", "unknown"),
        "grounded": r.get("verdict") == "grounded",
        "provenance": {
            "study": (r.get("primary_data") or {}).get("study", ""),
            "values": (r.get("primary_data") or {}).get("values", ""),
            "byte_quote": (r.get("primary_data") or {}).get("byte_quote", ""),
            "formula": r.get("lr_formula", ""),
            "n": (r.get("primary_data") or {}).get("n", ""),
            "population": (r.get("primary_data") or {}).get("population", ""),
        },
        "note": r.get("note", ""),
    }
    if r["id"] == "f0":  # the prior
        prior = node
    else:
        findings.append(node)

# attach finding/state labels from the skeleton
skel = {f["id"]: f for f in json.loads((HERE / "findings.json").read_text())}
for n in findings:
    s = skel.get(n["id"], {})
    n["finding"] = s.get("finding", "")
    n["state"] = s.get("state", "")
if prior:
    prior["finding"] = "prior"
    prior["value"] = prior["lr"]  # for the prior, computed_lr holds the prevalence

corpus = {
    "domain": "pulmonary_embolism",
    "target": "diagnosis(pulmonary_embolism)",
    "built": "case-blind, provenance-first (Phase 1 forward grounding)",
    "invariant": "every LR carries a byte-anchored provenance chain to primary data; ungroundable links are explicit data-gaps, never invented numbers",
    "prior": prior,
    "findings": findings,
}
# Canonical home: the corpus is a first-class, growing PRODUCT under corpus/,
# distinct from the provenance/ experiment scripts that produced it.
CORPUS_OUT = HERE.parent.parent / "corpus" / "pulmonary_embolism" / "corpus.json"
CORPUS_OUT.parent.mkdir(parents=True, exist_ok=True)
CORPUS_OUT.write_text(json.dumps(corpus, indent=2))
(HERE / "pe-corpus.json").write_text(json.dumps(corpus, indent=2))  # experiment-local copy

g = sum(1 for n in findings if n["grounded"])
print(f"PE corpus assembled: prior {'GROUNDED' if (prior and prior['grounded']) else 'ungrounded'} "
      f"= {prior['lr'] if prior else '?'}; {g}/{len(findings)} finding LRs grounded")
print(f"\n{'id':4s} {'finding(state)':46s} {'LR':>7}  {'verdict':14s} study")
for n in [prior] + findings if prior else findings:
    if not n:
        continue
    fs = f"{n.get('finding','')}({n.get('state','')})" if n.get("state") else n.get("finding", "")
    print(f"{n['id']:4s} {fs:46s} {n['lr']:>7}  {n['verdict']:14s} {n['provenance']['study'][:42]}")
