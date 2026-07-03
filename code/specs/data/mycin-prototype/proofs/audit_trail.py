#!/usr/bin/env python3
"""PROOF 3 — the audit trail is easy to follow: render the proof DAG as plain markdown.

For each case, re-run the engine and render the leader's proof DAG as a human-readable
table: prior -> each contribution (its evidence, log-LR, the running posterior after it,
the cited source + trust tier) -> final posterior. A reviewer can audit the decision line
by line without re-running the model. Writes proofs/audit_trail.md.

Run: python3 proofs/audit_trail.py
"""
import json
import math
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
sys.path.insert(0, ROOT)
import decide as D  # noqa: E402
import ir_to_adj as I  # noqa: E402


def sigmoid(x):
    return 1 / (1 + math.exp(-x)) if x >= 0 else math.exp(x) / (1 + math.exp(x))


def render(cid, rb, findings, cli):
    ir = json.load(open(os.path.join(ROOT, "ir", f"{cid}.json")))
    case_adj, _ = I.ir_to_adj(ir, findings)
    p = os.path.join(ROOT, "cases", f"_at_{cid}.adj")
    open(p, "w").write(rb.rstrip() + "\n\n" + case_adj)
    out = subprocess.run([cli, p], capture_output=True, text=True)
    os.remove(p)
    res = json.loads(out.stdout)
    lead = res["decision"].get("leader")
    r = {x["hypothesis"]: x for x in res["ranked"]}[lead]

    lines = [f"### {cid} — leader: `{lead}`  (decision: {res['decision'].get('type')})", "",
             "| step | evidence | log-LR | running P | cited source | trust |",
             "|---|---|---:|---:|---|---|"]
    run = 0.0
    for s in r["proof"]:
        run += s["logit"]
        ev = s.get("evidence", "—") if isinstance(s.get("evidence"), str) else ", ".join(s.get("evidence", []))
        src = (s.get("source") or "")[:48]
        lines.append(f"| {s['kind']} | {ev} | {s['logit']:+.3f} | {sigmoid(run):.4f} | {src} | {s.get('trust','')} |")
    lines.append(f"\n**Final P({lead}) = {r['posterior']:.4f}** — every step traces to a cited "
                 f"source clause; no model was consulted to produce or to audit this.\n")
    return "\n".join(lines)


def main():
    cli = D.find_cli()
    findings = I.load_findings()
    rb, cas_hash = D.load_rulebook()
    out = ["# MYCIN-2026 — worked audit trails (proof DAGs)",
           "",
           f"Rendered from the content-addressed CAS library `{cas_hash}` by re-running the engine. "
           "Each row is one fired clause; the running P is the posterior after applying it. A reviewer "
           "audits the decision line by line — the trail is the decision.", ""]
    for cid in ("MEN-1", "MEN-3"):
        out.append(render(cid, rb, findings, cli))
    open(os.path.join(HERE, "audit_trail.md"), "w").write("\n".join(out) + "\n")
    print("wrote proofs/audit_trail.md")


if __name__ == "__main__":
    main()
