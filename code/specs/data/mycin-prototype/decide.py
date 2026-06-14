#!/usr/bin/env python3
"""decide — the WARM-PATH decision step. DETERMINISTIC, zero answer-time model calls.

For each case: take the typed IR (decompose) + the adversarial read (inference + discard
verdicts), apply the gate (drop adversarial-LEAP findings; surface wrongly-dropped
discards), compile the gated IR to a case `.adj` (ir_to_adj), CONCATENATE it with the
CAS rulebook (the linking), and run the adj-lang-cli — the CPU reasoner — to get the
differential + proof DAG. The only thing this step invokes is the engine binary: no
agent, no model. We assert `answer_time_model_calls == 0`.

Run:  python3 decide.py   (after decompose -> ir/, adversarial_read -> advread/)
Outputs: cases/<id>.linked.adj (rulebook + case), decide_results.json.
"""
import glob
import json
import os
import shutil
import subprocess
import sys

import ir_to_adj as I

HERE = os.path.dirname(os.path.abspath(__file__))
CAS = os.path.join(HERE, "cas", "objects")
IR = os.path.join(HERE, "ir")
ADV = os.path.join(HERE, "advread")
CASES = os.path.join(HERE, "cases")


def find_cli():
    """Locate the adj-lang-cli binary, building it if necessary."""
    p = shutil.which("adj-lang-cli")
    if p:
        return p
    for cand in (
        os.path.join(HERE, "..", "..", "..", "packages", "rust", "target", "debug", "adj-lang-cli"),
        os.path.join(HERE, "..", "..", "..", "packages", "rust", "target", "release", "adj-lang-cli"),
    ):
        cand = os.path.abspath(cand)
        if os.path.exists(cand):
            return cand
    raise SystemExit("adj-lang-cli not found — run `cargo build -p adj-lang-cli` in code/packages/rust")


def load_rulebook():
    """Return (rulebook_text, cas_hash) from the gated CAS library object."""
    objs = sorted(glob.glob(os.path.join(CAS, "*.json")))
    if not objs:
        raise SystemExit("no CAS object — run cas_write_gate.py prep && commit")
    obj = json.load(open(objs[-1]))
    rb_path = os.path.join(HERE, obj["rulebook_path"])
    return open(rb_path).read(), obj["hash"]


def gated_ir(ir, adv):
    """Mark adversarial-LEAP inferred findings as LEAP (so ir_to_adj drops them);
    return (gated_ir, recovered_discards)."""
    leaps = set((adv or {}).get("inference_leaps", []))
    ij = {j["term"]: dict(j) for j in ir.get("inference_justifications", [])}
    for t in leaps:
        ij.setdefault(t, {"term": t, "basis_span": "", "verdict": "LEAP"})["verdict"] = "LEAP"
    g = {**ir, "inference_justifications": list(ij.values())}
    return g, (adv or {}).get("discard_load_bearing", [])


def decide_case(cid, rulebook, findings, cli):
    ir = json.load(open(os.path.join(IR, f"{cid}.json")))
    adv_path = os.path.join(ADV, f"{cid}.json")
    adv = json.load(open(adv_path)) if os.path.exists(adv_path) else {}
    gir, recovered = gated_ir(ir, adv)
    case_adj, dropped = I.ir_to_adj(gir, findings)
    linked = rulebook.rstrip() + "\n\n" + case_adj
    linked_path = os.path.join(CASES, f"{cid}.linked.adj")
    open(linked_path, "w").write(linked)

    out = subprocess.run([cli, linked_path], capture_output=True, text=True)
    result = json.loads(out.stdout) if out.stdout.strip().startswith("{") else {"error": out.stderr}
    dec = dict(result.get("decision", {}))
    leader = dec.get("leader")

    # Evidence-sufficiency guard (abstain, don't fabricate): if the leader's proof
    # fired ONLY a prior — no observed finding contributed — the "decision" rests on
    # base rates alone, which is not a defensible commitment. Override to abstain.
    ranked = {r["hypothesis"]: r for r in result.get("ranked", [])}
    lead_proof = ranked.get(leader, {}).get("proof", [])
    n_evidence = sum(1 for s in lead_proof if s.get("kind") in ("contribution", "interaction"))
    if leader is not None and n_evidence == 0:
        dec = {"type": "insufficient_evidence", "leader": leader,
               "reason": "no observed finding contributed — decision would rest on the prior alone"}

    return {
        "case_id": cid,
        "answer_time_model_calls": 0,           # decide invokes only the engine binary
        "decision": dec,
        "leader": leader,
        "n_evidence_for_leader": n_evidence,
        "posteriors": {r["hypothesis"]: round(r["posterior"], 4) for r in result.get("ranked", [])},
        "dropped_leap_findings": dropped,
        "recovered_discards": recovered,        # discard read: wrongly-dropped findings surfaced
        "linked_program": f"cases/{cid}.linked.adj",
    }


def main():
    cli = find_cli()
    rulebook, cas_hash = load_rulebook()
    findings = I.load_findings()
    gold = {c["id"]: c["gold"] for c in json.load(open(os.path.join(CASES, "cases.json")))["cases"]}
    ids = sorted(os.path.splitext(os.path.basename(p))[0] for p in glob.glob(os.path.join(IR, "*.json")))

    rows = []
    for cid in ids:
        r = decide_case(cid, rulebook, findings, cli)
        g = gold.get(cid)
        d = r["decision"]
        # match: determinate leader == gold; abstain (kickback / insufficient_evidence)
        # is correct iff gold is indeterminate.
        if d.get("type") in ("kickback", "insufficient_evidence"):
            r["match"] = (g == "indeterminate")
        else:
            r["match"] = (r["leader"] == g)
        r["gold"] = g
        rows.append(r)

    summary = {
        "cas_library": cas_hash,
        "answer_time_model_calls_total": sum(r["answer_time_model_calls"] for r in rows),
        "decisions": {r["case_id"]: f"{r['decision'].get('type')} -> {r.get('leader')} "
                                    f"(P={r['posteriors'].get(r.get('leader'), '-')}) [gold {r['gold']}]"
                      for r in rows},
        "recovered_discards": {r["case_id"]: r["recovered_discards"] for r in rows if r["recovered_discards"]},
    }
    json.dump({"summary": summary, "rows": rows},
              open(os.path.join(HERE, "decide_results.json"), "w"), indent=1)
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
