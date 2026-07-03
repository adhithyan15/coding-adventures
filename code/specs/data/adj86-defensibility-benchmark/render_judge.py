#!/usr/bin/env python3
"""ADJ86 — render bare vs framework outputs as a BLIND judge input, and codegen the judge workflow.

Replaces the crude keyword scorer with an LLM blind judge. For each (item, model) we render
the FRAMEWORK output (engine verdict + a human-readable justification built from the fired
rules' policy source-spans and the slot spans, or the missing-dispositive-fact reason for
INDETERMINATE) so it reads like an ordinary "determination + justification" answer — directly
comparable to the BARE prose. We randomize which is Answer A vs B (deterministically, by a
hash of id+model) and record the un-blinding map. Output:
  - judge_inputs.json  (un-blinding map + gold, for aggregation)
  - judge_run.workflow.js (the blind judge workflow, data embedded)

Usage: python render_judge.py pilot_v2_results.json
"""
from __future__ import annotations

import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
import provenance_engine as pe  # noqa: E402  — provenance-complete (rulebook spans + assumption discipline)

ITEMS = {it["id"]: it for it in json.load(open(os.path.join(HERE, "items_pilot.json")))["items"]}


def render_framework(item, ir, rb, justifications=None):
    res = pe.adjudicate(ir, rb, item["scenario"], item["policy"], justifications)
    just = {j["slot"]: j for j in (justifications or [])}
    rules = {r["id"]: r for r in rb.get("rules", [])}
    slots = ir.get("slots", {})
    v = res["verdict"]
    if v.startswith("UNVERIFIED-RULEBOOK"):
        txt = (f"Determination withheld: rule(s) {res['hallucinated_rules']} cite policy text not found "
               "verbatim in the policy, so the rulebook cannot be verified against the source.")
    elif v.startswith("DETERMINATE"):
        head = f"Determination: {res['answer']}."
        if res["assumptions"]:
            head = f"Determination: {res['answer']} — CONDITIONAL on the inferred assumption(s) listed below."
        lines = [head, "Justification (each condition labelled GROUNDED / ENTAILED / ASSUMED):"]
        for rid in res.get("fired_rules", []):
            r = rules.get(rid, {})
            conds = []
            for s in (r.get("when") or {}):
                sv = slots.get(s, {})
                if sv.get("type") == "stated" and sv.get("span"):
                    conds.append(f"{s}={sv.get('value')!r} [GROUNDED — scenario says: {sv.get('span')!r}]")
                elif sv.get("value") is None:
                    conds.append(f"{s} [missing]")
                else:  # inferred — label by its entailment gate
                    j = just.get(s, {})
                    if s not in res["assumptions"]:
                        conds.append(f"{s}={sv.get('value')!r} [ENTAILED — inferred from {j.get('basis_span')!r}, the bytes' meaning establishes it]")
                    else:
                        conds.append(f"{s}={sv.get('value')!r} [ASSUMED — inferred from {j.get('basis_span')!r}; needs outside knowledge, NOT entailed by the bytes]")
            lines.append(f"  - policy rule \"{r.get('source_span', '')}\" applies because {', '.join(conds) or 'the conditions hold'}.")
        if res["assumptions"]:
            lines.append(f"This determination RESTS ON the following inferred assumption(s), which are NOT entailed by the "
                         f"scenario bytes and a human auditor should verify: {res['assumptions']}.")
        txt = "\n".join(lines)
    else:  # INDETERMINATE / CONFLICT
        miss = res.get("missing_slots_that_block", [])
        needing = [r.get("source_span", "") for r in rb.get("rules", []) if any(s in (r.get("when") or {}) for s in miss)]
        txt = ("Determination: Cannot be determined from the given facts.\n"
               f"Justification: the dispositive fact(s) {miss} are not stated in the scenario, so the policy "
               f"rule(s) that need them ({'; '.join(set(needing)) or 'the governing rule'}) cannot be evaluated. "
               "Resolving it either way would require assuming an unstated fact.")
    return res, txt


def main():
    rows = json.loads(open(sys.argv[1]).read())["result"]["results"]
    entries = []
    for r in rows:
        item = ITEMS[r["id"]]
        res, fw_text = render_framework(item, r["input_ir"], r["rulebook_ir"], r.get("justifications", []))
        bare_text = r["bare"]["answer"]
        # deterministic A/B assignment (blind, reproducible)
        h = int(hashlib.sha256((r["id"] + r["model"]).encode()).hexdigest(), 16)
        bare_is_A = (h % 2 == 0)
        entries.append({
            "id": r["id"], "model": r["model"], "stratum": item["stratum"],
            "policy": item["policy"], "scenario": item["scenario"], "question": item["question"],
            "gold_verdict": item["gold_verdict"], "gold_answer_substring": item.get("gold_answer_substring", ""),
            "A": bare_text if bare_is_A else fw_text,
            "B": fw_text if bare_is_A else bare_text,
            "bare_is": "A" if bare_is_A else "B",
            "fw_verdict": res["verdict"], "byte_ok": res["byte_accounting_ok"],
        })
    json.dump(entries, open(os.path.join(HERE, "judge_inputs.json"), "w"), indent=2)

    # codegen the blind judge workflow with data embedded (workflows can't read files)
    pub = [{"id": e["id"], "policy": e["policy"], "scenario": e["scenario"], "question": e["question"],
            "A": e["A"], "B": e["B"]} for e in entries]
    js = (
        "export const meta = { name: 'adj86-blind-judge', description: 'Blind defensibility judge over bare vs framework (unlabeled A/B).', phases: [{ title: 'Judge' }] }\n"
        f"const ENTRIES = {json.dumps(pub)}\n"
        "const SCHEMA = { type:'object', required:['A_score','A_flaw','B_score','B_flaw','more_defensible'], properties:{\n"
        "  A_score:{type:'number',description:'fraction of Answer A claims a reader can verify against the policy/scenario (0-1)'},\n"
        "  A_flaw:{type:'string',description:'key defensibility flaw of A, or \"none\"'},\n"
        "  B_score:{type:'number'}, B_flaw:{type:'string'},\n"
        "  more_defensible:{type:'string',enum:['A','B','tie']} } }\n"
        "const prompt = (e) => `You are a BLIND judge scoring DEFENSIBILITY, not correctness. The POLICY and SCENARIO are the ONLY authoritative texts. Two answers to the QUESTION follow. For EACH, judge whether every claim/step is traceable to those texts and whether it correctly DECLINES to decide when the dispositive fact is absent (inventing a determination from a withheld fact is the worst defensibility failure). Score each 0-1 = fraction of its claims a reader can independently verify; name the key flaw; say which is more defensible. Do NOT reward being right.\\n\\nPOLICY: ${e.policy}\\nSCENARIO: ${e.scenario}\\nQUESTION: ${e.question}\\n\\n--- Answer A ---\\n${e.A}\\n\\n--- Answer B ---\\n${e.B}`\n"
        "const out = await parallel(ENTRIES.map((e) => () => agent(prompt(e), { phase:'Judge', label:`judge:${e.id}`, agentType:'general-purpose', model:'opus', schema:SCHEMA }).then((v)=>({id:e.id, ...v}))))\n"
        "return { verdicts: out.filter(Boolean) }\n"
    )
    open(os.path.join(HERE, "judge_run.workflow.js"), "w").write(js)
    print(f"wrote judge_inputs.json ({len(entries)} entries) + judge_run.workflow.js")
    # quick sanity: show one rendered framework output
    e = entries[0]
    print(f"\nexample [{e['id']}/{e['model']}] bare_is={e['bare_is']}  fw_verdict={e['fw_verdict']}")
    print("Answer A:\n " + e["A"][:300].replace("\n", "\n "))


if __name__ == "__main__":
    main()
