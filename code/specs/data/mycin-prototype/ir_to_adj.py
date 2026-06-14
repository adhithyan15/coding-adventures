#!/usr/bin/env python3
"""ir_to_adj — DETERMINISTIC compiler: typed IR -> a case adj-lang program.

This is the frontend half of "IR -> ADJ program". It is **model-free**: given the
(gated) typed IR for a case, it emits one `observe <term>` line per accepted finding,
having first validated every term against the standard dictionary (the same
enforcement dict_lint applies to the rulebook). The rulebook supplies the `?` queries,
so the case program is just the observations — concatenated with the CAS rulebook at
decide time. No model reasoning happens here; the diagnosis is the engine's.

Which findings become `observe` lines:
  * type == "stated"  -> always (byte-anchored to the vignette).
  * type == "inferred" -> only if the adversarial inference read did NOT rule it a
    majority-LEAP (a surfaced over-read is dropped, not silently asserted).
Findings whose term is not in the dictionary are rejected (raises) — the shared
vocabulary is enforced, so a case can never `observe` a term the rulebook can't see.

Usage:  python3 ir_to_adj.py <gated_ir.json>  ->  prints the case .adj to stdout
        (or imported: ir_to_adj(ir, findings_dict) -> (adj_text, dropped))
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load_findings():
    d = json.load(open(os.path.join(HERE, "dictionary.json")))
    return {f["functor"]: set(f["value_domain"]) for f in d["findings"]}


def _parse_term(term):
    """'csf_glucose(low)' -> ('csf_glucose', 'low'); bare atom -> (atom, None)."""
    term = term.strip()
    if "(" in term and term.endswith(")"):
        functor, value = term[:-1].split("(", 1)
        return functor.strip(), value.strip()
    return term, None


def ir_to_adj(ir, findings_dict):
    """Return (adj_text, dropped). `dropped` lists findings excluded as LEAP."""
    leap = {j["term"] for j in ir.get("inference_justifications", [])
            if j.get("verdict") == "LEAP"}
    lines, dropped = [], []
    seen = set()
    for f in ir.get("findings", []):
        term = f["term"].replace(" ", "")
        functor, value = _parse_term(term)
        if functor not in findings_dict or value not in findings_dict[functor]:
            raise ValueError(f"{ir.get('case_id')}: term '{term}' is not in the dictionary "
                             f"(enforcement: rulebook and case must share one vocabulary)")
        if f.get("polarity") == "denied":
            continue  # negation is carried by the value (normal/absent/negative), not by observe
        if f.get("type") == "inferred" and term in leap:
            dropped.append(term)       # surfaced over-read: do not assert it
            continue
        if term in seen:
            continue
        seen.add(term)
        lines.append(f"observe {functor}({value})")
    adj = (f"% case {ir.get('case_id')} — observations only; the rulebook supplies the queries\n"
           + "\n".join(lines) + "\n")
    return adj, dropped


def main(path):
    ir = json.load(open(path))
    adj, dropped = ir_to_adj(ir, load_findings())
    if dropped:
        sys.stderr.write(f"dropped LEAP findings: {dropped}\n")
    sys.stdout.write(adj)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: python3 ir_to_adj.py <gated_ir.json>", file=sys.stderr)
        sys.exit(2)
    main(sys.argv[1])
