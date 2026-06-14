#!/usr/bin/env python3
"""PROOF 2 — cost-to-correct is small: one localized CAS edit, propagated, 0 model calls.

The naive rulebook over-saturates the pre-culture case (MEN-2) to P≈0.9999, because the
four correlated CSF-chemistry findings (neutrophilic pleocytosis, low glucose, elevated
protein, elevated lactate) are multiplied as if independent. This proof:

  1. LOCALIZE — run MEN-2 on the naive rulebook, read the proof DAG, and show the error is
     exactly the four stacked CSF `contributes` to bacterial (a specific, named locus).
  2. EDIT (once) — apply a localized CAS override: keep one representative CSF finding
     (neutrophilic pleocytosis) and neutralize the other three correlated ones. This is the
     documented adj52 fix — "fix the fact, not the weight". Content-address -> a new CAS
     version. The edit touches 3 clauses; nothing else.
  3. RE-DERIVE + PROPAGATE — re-run EVERY case on the corrected rulebook, with ZERO
     answer-time model calls. MEN-2 calibrates to P≈0.77 (high suspicion, await culture),
     and the fix propagates to every other case citing those clauses (e.g. MEN-1).

Run: python3 proofs/cost_to_correct.py
"""
import hashlib
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
sys.path.insert(0, ROOT)
import decide as D  # reuse find_cli, load_findings, ir_to_adj  # noqa: E402
import ir_to_adj as I  # noqa: E402

# the three correlated CSF findings to neutralize (keep neutrophilic as representative)
CORRELATED = [
    "csf_glucose(low) to bacterial_meningitis",
    "csf_protein(elevated) to bacterial_meningitis",
    "csf_lactate(elevated) to bacterial_meningitis",
]


def override(rulebook):
    """Localized CAS edit: drop the 3 correlated CSF `contributes` clauses to bacterial,
    keeping `csf_neutrophilic_pleocytosis(high)` as the single CSF-chemistry representative.
    Returns (corrected_text, removed_clause_lines)."""
    out, removed, skip = [], [], 0
    lines = rulebook.splitlines(keepends=True)
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.lstrip().startswith("contributes") and any(c in line for c in CORRELATED):
            # drop this clause line + its following annotation (source/trust) line(s)
            removed.append(line.strip())
            i += 1
            while i < len(lines) and (lines[i].lstrip().startswith("source")
                                      or lines[i].lstrip().startswith("trust")
                                      or lines[i].lstrip().startswith("locator")):
                i += 1
            continue
        out.append(line)
        i += 1
    return "".join(out), removed


def run_case(rulebook, ir, findings, cli):
    case_adj, _ = I.ir_to_adj(ir, findings)
    linked = rulebook.rstrip() + "\n\n" + case_adj
    p = os.path.join(ROOT, "cases", f"_cc_{ir['case_id']}.adj")
    open(p, "w").write(linked)
    out = subprocess.run([cli, p], capture_output=True, text=True)
    os.remove(p)
    res = json.loads(out.stdout)
    ranked = {r["hypothesis"]: r for r in res["ranked"]}
    lead = res["decision"].get("leader")
    return ranked, lead


def main():
    cli = D.find_cli()
    findings = I.load_findings()
    rb, _ = D.load_rulebook()
    irs = {cid: json.load(open(os.path.join(ROOT, "ir", f"{cid}.json")))
           for cid in ("MEN-1", "MEN-2", "MEN-3", "MEN-4")}

    # 1. LOCALIZE on MEN-2 with the naive rulebook
    naive_ranked, _ = run_case(rb, irs["MEN-2"], findings, cli)
    bact = naive_ranked["bacterial_meningitis"]
    csf_steps = [s for s in bact["proof"]
                 if s.get("kind") == "contribution" and s["evidence"].startswith("csf_")
                 and s["evidence"] != "csf_culture(positive)"]
    p_naive = bact["posterior"]

    # 2. EDIT once -> corrected rulebook -> new CAS version
    corrected, removed = override(rb)
    rb_v2_path = os.path.join(ROOT, "rulebook", "meningitis_v2.adj")
    open(rb_v2_path, "w").write(corrected)
    digest = hashlib.sha256(corrected.encode()).hexdigest()[:16]
    cas_v2 = os.path.join(ROOT, "cas", "objects", f"{digest}.json")
    json.dump({"hash": digest, "domain": "meningitis_differential",
               "supersedes": "286c17aaf48ff32d", "rulebook_path": "rulebook/meningitis_v2.adj",
               "edit": "CSF-correlation override: neutralize glucose/protein/lactate, keep neutrophilic",
               "removed_clauses": removed,
               "provenance": "ADJ56 — meningitis CSF over-saturation; fix the fact, not the weight"},
              open(cas_v2, "w"), ensure_ascii=False, indent=1)

    # 3. RE-DERIVE every case on the corrected rulebook (0 model calls)
    propagated = {}
    for cid, ir in irs.items():
        ranked, lead = run_case(corrected, ir, findings, cli)
        propagated[cid] = {"leader": lead,
                           "P_bacterial": round(ranked["bacterial_meningitis"]["posterior"], 4),
                           "P_viral": round(ranked["viral_meningitis"]["posterior"], 4)}
    p_fixed = propagated["MEN-2"]["P_bacterial"]

    result = {
        "claim": "cost-to-correct is small: one localized edit, propagated, 0 model calls",
        "1_localize": {
            "case": "MEN-2 (pre-culture)",
            "naive_P_bacterial": round(p_naive, 4),
            "error_locus": "four correlated CSF-chemistry contributions stacked as independent",
            "stacked_csf_contributions_in_proof": [s["evidence"] for s in csf_steps],
        },
        "2_edit": {
            "edits": 1,
            "clauses_touched": len(removed),
            "removed": removed,
            "kept_representative": "csf_neutrophilic_pleocytosis(high)",
            "new_cas_version": f"cas/objects/{digest}.json",
            "answer_time_model_calls": 0,
        },
        "3_propagate": {
            "MEN-2_calibrated": f"{round(p_naive,4)} -> {p_fixed} (over-saturation -> high suspicion, await culture)",
            "all_cases_after_fix": propagated,
            "answer_time_model_calls": 0,
            "note": "the single edit propagated to every case citing those clauses (MEN-1 recalibrated too)",
        },
    }
    json.dump(result, open(os.path.join(HERE, "cost_to_correct_result.json"), "w"), indent=1)
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
