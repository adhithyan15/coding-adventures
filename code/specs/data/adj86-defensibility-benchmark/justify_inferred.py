#!/usr/bin/env python3
"""ADJ86 — byte-cited justification check for INFERRED facts (the user's refinement / ADJ61).

For every inferred slot, the model must (1) point to the EXACT scenario bytes the inference is
drawn from, and (2) an adversarial check asks: considering ONLY the meaning of those bytes,
do they ENTAIL the inference, or is it a LEAP beyond what the bytes say? This separates a
grounded computation/restatement ("four months" -> "<1 year"; "emergency room" -> "emergency")
from a world-knowledge leap ("cardiologist" -> "specialist"; "$" -> "USD"; "member" ->
"covered"). Only LEAPs are genuine assumptions; ENTAILED inferences are grounded.

Stage 1 here just codegens the workflow over the 17 distinct inferred facts from the pilot.
Stage 2 (justify_eval) does the deterministic byte-anchor (basis must be verbatim) + tallies.

Usage: python justify_inferred.py pilot_v2_results.json   # -> justify_run.workflow.js + facts.json
"""
from __future__ import annotations

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ITEMS = {it["id"]: it for it in json.load(open(os.path.join(HERE, "items_pilot.json")))["items"]}


def main():
    rows = json.loads(open(sys.argv[1]).read())["result"]["results"]
    seen, facts = set(), []
    for r in rows:
        for name, sv in r["input_ir"].get("slots", {}).items():
            if sv.get("type") == "inferred" and sv.get("value") is not None and (r["id"], name) not in seen:
                seen.add((r["id"], name))
                facts.append({"id": r["id"], "slot": name, "value": sv["value"],
                              "scenario": ITEMS[r["id"]]["scenario"]})
    json.dump(facts, open(os.path.join(HERE, "facts.json"), "w"), indent=2)

    js = (
        "export const meta = { name: 'adj86-justify-inferred', description: 'Byte-cited entailment check for inferred facts.', phases: [{ title: 'Justify' }] }\n"
        f"const FACTS = {json.dumps(facts)}\n"
        "const SCHEMA = { type:'object', required:['basis_span','verdict','rationale'], properties:{\n"
        "  basis_span:{type:['string','null'],description:'the EXACT verbatim substring of the scenario the inference is drawn from, or null if nothing in the scenario supports it'},\n"
        "  verdict:{type:'string',enum:['ENTAILED','LEAP'],description:'ENTAILED iff the MEANING OF THE QUOTED BYTES ALONE establishes the inferred fact; LEAP if deriving it needs outside/world knowledge or an unstated assumption beyond those bytes'},\n"
        "  rationale:{type:'string'} } }\n"
        "const prompt = (f) => `You are an adversarial grounding checker. The SCENARIO below is the ONLY ground truth. A system INFERRED this fact:\\n  ${f.slot} = ${JSON.stringify(f.value)}\\n\\nSCENARIO: ${f.scenario}\\n\\n(1) Quote the EXACT verbatim bytes from the scenario that this inference is drawn from (basis_span), or null if nothing in the scenario supports it.\\n(2) Considering ONLY the meaning of those quoted bytes — explicitly NOT any outside/world knowledge — do they ENTAIL the inferred fact, or does deriving it require a LEAP beyond what the bytes literally establish? A restatement or a deterministic computation from the bytes (e.g. \\\"four months\\\" entails \\\"less than one year\\\") is ENTAILED. Needing a fact the bytes do not carry (e.g. that a cardiologist is a specialist, that \\\"$\\\" means USD, that \\\"member\\\" means covered) is a LEAP. Be strict.`\n"
        "const out = await parallel(FACTS.map((f) => () => agent(prompt(f), { phase:'Justify', label:`justify:${f.id}:${f.slot}`, agentType:'general-purpose', model:'opus', schema:SCHEMA }).then((v)=>({id:f.id, slot:f.slot, ...v}))))\n"
        "return { justifications: out.filter(Boolean) }\n"
    )
    open(os.path.join(HERE, "justify_run.workflow.js"), "w").write(js)
    print(f"wrote facts.json ({len(facts)} inferred facts) + justify_run.workflow.js")


if __name__ == "__main__":
    main()
