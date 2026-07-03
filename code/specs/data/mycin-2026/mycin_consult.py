#!/usr/bin/env python3
"""mycin_consult.py — the interactive MYCIN consultation, end to end, 1976-faithful.

This is the program that ties MYCIN-2026 together the way the 1976 MYCIN actually
ran: an interactive dialogue that gathers findings, reaches a diagnosis, recommends
culture-directed therapy, and can explain WHY it asked and HOW it concluded — all
at 0 answer-time model calls (the only model touch is the upstream decompose; here
the inputs are clinician-entered data, exactly as MYCIN took them).

  [1] PRESENT ILLNESS   the findings known at intake
  [2] CONSULTATION      MYCIN's hallmark Q&A loop — value-of-information picks the
                        single most decision-relevant unobserved finding, asks for
                        it, records the answer, and RE-DERIVES; repeat until the
                        diagnosis is settled or no remaining question would change
                        it. Each question is justified by its VOI (the WHY).
  [3] DIAGNOSIS         the differential + the decision, with the contributing
                        evidence (the HOW — the proof, byte-cited in the rulebook).
  [4] THERAPY           empiric minimum-cost cover of the likely organisms, then
                        CULTURE-DIRECTED refinement: an in-vitro sensitivity
                        (resistant drug→organism) defeases that coverage edge and
                        the regimen RE-DERIVES around it.
  [5] REVIEW            every line grounded + overridable; the physician decides.

Two MYCIN behaviors this closes vs the rest of the stack: the interactive
consultation LOOP (we already had the VOI ranking; this drives the dialogue on top
of it) and LIVE sensitivity ingestion into therapy.

Usage:
  python3 mycin_consult.py [case]            # scripted (answers from the case oracle)
  python3 mycin_consult.py [case] --ask      # interactive (prompts you per question)
"""

from __future__ import annotations

import sys
from pathlib import Path

MYCIN = Path(__file__).resolve().parent
sys.path.insert(0, str(MYCIN / "warm"))
sys.path.insert(0, str(MYCIN / "treatment" / "antibiotics"))
import decide as decide_mod  # noqa: E402
import derive_regimen as reg  # noqa: E402  (formulary facts, for the WHY of each drug)
import ir_to_adj as ir_mod  # noqa: E402  (the closed-vocabulary domains)
import native_setcover as abx  # noqa: E402  (sensitivity-aware set-cover therapy)
import voi as voi_mod  # noqa: E402

# Stop asking once the diagnosis is determinate with enough corroborating evidence,
# or once no remaining question would move the differential, or after this many Qs.
EVIDENCE_STOP = 3
MAX_QUESTIONS = 10
EPS = 0.01  # a question whose |Δmargin| is below this won't change the call


# Scripted consultation cases: sparse intake + an ORACLE the clinician answers from
# (so the dialogue is deterministic + CI-testable), the empiric organism set, and an
# optional culture sensitivity that arrives after empiric therapy.
CASES = {
    "adult_bacterial": {
        "vignette": "Adult, acutely unwell: fever and neck stiffness at intake; "
                    "CSF studies pending.",
        "intake": ["fever(present)", "meningismus(present)"],
        "oracle": {  # the findings the clinician can supply WHEN ASKED
            "csf_gram_stain": "positive",
            "csf_neutrophilic_pleocytosis": "high",
            "csf_glucose": "low",
            "csf_protein": "high",
            "csf_lactate": "high",
            "serum_procalcitonin": "high",
            "csf_lymphocytic_pleocytosis": "normal",
        },
        "organisms": "over_50_or_immunocompromised",  # empiric cover incl. listeria
        # Culture comes back: the Listeria isolate is ampicillin-resistant → that
        # coverage edge is defeated and the regimen must re-derive.
        "sensitivities": [("ampicillin", "listeria")],
        "gold": "bacterial_meningitis",
    },
}


def margin(res: dict) -> float:
    post = sorted(res["posteriors"].values(), reverse=True)
    return post[0] - post[1] if len(post) >= 2 else post[0]


def ask_finding(functor: str, oracle: dict, interactive: bool) -> str | None:
    """Obtain the value of `functor` — interactively from the user, or from the
    case oracle. Returns the value, or None if the datum is unavailable."""
    if interactive:
        ans = input(f"    ? {functor} = ").strip()
        return ans or None
    return oracle.get(functor)


def consult(cli, case: dict, interactive: bool = False) -> dict:
    bar = "=" * 78
    print(bar + f"\nMYCIN-2026 CONSULTATION\n  {case['vignette']}\n" + bar)

    # The closed vocabulary — every answer is validated against it before it can
    # reach the engine (an answer from input()/the oracle is otherwise untrusted;
    # this upholds the same gate ir_to_adj enforces on the decompose path).
    domains = ir_mod.load_domains()  # functor -> set(legal values)
    observed = list(case["intake"])
    asked = {t.split("(")[0] for t in observed}
    dialogue = []
    print(f"\n[1] PRESENT ILLNESS: {', '.join(observed)}")

    print("\n[2] CONSULTATION  (value-of-information drives the questions; 0 model calls)")
    res = None
    for _ in range(MAX_QUESTIONS):
        observe_adj = "".join(f"observe {t}\n" for t in observed)
        res = decide_mod.decide("session", observe_adj, cli)
        rows = voi_mod.voi("session", observe_adj, set(observed), cli)
        # Settled? determinate with enough evidence, or nothing left that would move it.
        next_q = next((r for r in rows if r["order"].split("(")[0] not in asked), None)
        determinate = res["decision"].get("type") == "determinate"
        if determinate and (res["n_evidence_for_leader"] >= EVIDENCE_STOP
                            or next_q is None or abs(next_q["margin_delta"]) < EPS):
            break
        if next_q is None:
            break
        functor = next_q["order"].split("(")[0]
        asked.add(functor)
        print(f"  ? order {functor:30s}  (would shift the margin by "
              f"{next_q['margin_delta']:+.3f} — most decision-relevant)")
        ans = ask_finding(functor, case["oracle"], interactive)
        if ans is None:
            print("      → unavailable; noted, moving on")
            continue
        # Closed-vocabulary gate: only a recognized functor(value) reaches the engine.
        if functor not in domains or ans not in domains[functor]:
            print(f"      → '{ans}' is not a recognized value for {functor}; ignored")
            continue
        observed.append(f"{functor}({ans})")
        dialogue.append({"functor": functor, "value": ans, "voi": next_q["margin_delta"]})
        nr = decide_mod.decide("session", "".join(f"observe {t}\n" for t in observed), cli)
        print(f"      → {functor} = {ans}   (leader now {nr['leader']}, "
              f"P={max(nr['posteriors'].values()):.3f})")

    leader = res["leader"]
    dtype = res["decision"].get("type")
    print("\n[3] DIAGNOSIS  (0 answer-time model calls — engine over the grounded rulebook)")
    for hyp, p in sorted(res["posteriors"].items(), key=lambda kv: -kv[1]):
        print(f"    {hyp:24s} P = {p:.4f}{'   <- leading' if hyp == leader else ''}")
    print(f"    decision: {dtype}  | corroborating findings for the leader: "
          f"{res['n_evidence_for_leader']}  | questions asked: {len(dialogue)}")

    if leader != "bacterial_meningitis" or dtype == "insufficient_evidence":
        print("\n[4] THERAPY: no empiric antibacterial therapy indicated by the leading "
              f"diagnosis ({leader}); reassess / await data. (Abstains, never fabricates.)")
        print("\n" + bar + "\nPHYSICIAN REVIEW — grounded + overridable; you make the call.")
        return {"leader": leader, "dialogue": dialogue, "regimen": None}

    organisms = reg.SCENARIOS[case["organisms"]]
    print(f"\n[4] THERAPY  (minimum-cost cover; 0 model calls)\n    likely organisms: {organisms}")
    empiric = abx.solve(cli, organisms, set())
    _show_regimen("    EMPIRIC regimen", empiric, organisms, set())

    sens = [tuple(s) for s in case.get("sensitivities", [])]
    final = empiric
    if sens:
        print(f"\n    CULTURE BACK — sensitivities (resistant): {sens}")
        final = abx.solve(cli, organisms, set(), defeated=set(sens))
        _show_regimen("    CULTURE-DIRECTED regimen (coverage re-derived)", final, organisms, set(sens))

    print("\n" + bar)
    print("PHYSICIAN REVIEW — every question, finding, and drug above is grounded + "
          "overridable; you make the call.")
    print("answer-time model calls across the whole consultation: 0")
    return {"leader": leader, "dialogue": dialogue, "regimen": final.get("regimen"),
            "empiric": empiric.get("regimen")}


def _show_regimen(label: str, res: dict, organisms: list[str], defeated: set) -> None:
    if res.get("regimen") is None:
        print(f"{label}: NO regimen covers {organisms} "
              + (f"given resistance {sorted(defeated)} " if defeated else "")
              + f"-> escalate / specialist [{res.get('outcome')}].")
        return
    print(f"{label}: {' + '.join(res['regimen'])}  (cost {res.get('cost'):.0f})")
    for d in res["regimen"]:
        covered = sorted(o for o in organisms
                         if o in reg.DRUGS.get(d, {}).get("covers", []) and (d, o) not in defeated)
        why = f"covers {covered}" if covered else "combination partner"
        print(f"      - {d:13s} {why}")


def main(argv: list[str]) -> int:
    interactive = "--ask" in argv
    names = [a for a in argv if not a.startswith("--")]
    case_name = names[0] if names else "adult_bacterial"
    if case_name not in CASES:
        print(f"unknown case {case_name!r}; choices: {list(CASES)}", file=sys.stderr)
        return 2
    cli = decide_mod.find_cli()
    if cli is None:
        print("mycin_consult: adj-lang-cli not built", file=sys.stderr)
        return 3
    consult(cli, CASES[case_name], interactive=interactive)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
