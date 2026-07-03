#!/usr/bin/env python3
"""treat.py - the treatment decision as a SOLVED constraint problem. 0 model calls.

MYCIN-2026 (treatment layer). The differential gives P(bacterial); the clinical
ACTION - treat empirically now, or wait for definitive culture - is NOT the
argmax-posterior. Medicine treats fast and cheap: missing bacterial meningitis is
catastrophic, empirical antibiotics are cheap and low-harm, and there is a hard
door-to-antibiotic deadline. So the decision is a constraint problem the SAME
solver (adj-lang `symbol`/`constrain`/`solve`/`check`) solves:

  (A) COST break-even - solve for the probability p* at which treating equals
      waiting:  p* * cost_miss = cost_treat  ->  treat iff P(bacterial) >= p*.
      With cost_miss >> cost_treat, p* is tiny: you treat even at low probability.

  (B) TIME feasibility - check whether WAITING for a definitive culture result can
      satisfy the door-to-antibiotic deadline:  culture_hours <= deadline_hours.
      Culture takes ~48 h, the deadline is ~1 h, so this is UNSAT (with an IIS
      core) - the solver PROVES you cannot defer the decision; it must be made now
      on current evidence.

This resolves the honest paradox from the diagnostic layer: a pre-culture case can
be *more probably* viral (by base rate) yet the cost+time-constrained decision is
"treat empirically for bacterial NOW" - because you act on COST and TIME, not on
the argmax probability. The recommendation cites the solved p* and the binding
time constraint, so it is auditable end to end.

Usage:  python3 treat.py <case_id>     (reads ir/<case_id>.json; runs the warm decide)
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

MYCIN = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402

IR_DIR = MYCIN / "ir"

# Clinical cost/time parameters (utility units; documented, editable - they are
# the policy, and like the rulebook they are auditable inputs, not magic numbers).
COST_MISS = 100   # harm of MISSING bacterial meningitis (death/disability) - large
COST_TREAT = 1    # harm/cost of empirical antibiotics if not bacterial - small
DEADLINE_HOURS = 1    # door-to-antibiotic target (IDSA: antibiotics within ~1 h)
CULTURE_HOURS = 48    # time to a definitive CSF culture result


def run_check_or_solve(cli: Path, program: str, key: str) -> dict:
    p = MYCIN / "treatment" / "_tmp_policy.adj"
    p.write_text(program)
    try:
        r = subprocess.run([str(cli), str(p)], capture_output=True, text=True)
        assert r.returncode == 0, r.stderr
        return json.loads(r.stdout).get(key, {})
    finally:
        p.unlink(missing_ok=True)


def cost_breakeven(cli: Path) -> float:
    """Solve p* : p* * cost_miss = cost_treat (the treat-iff threshold)."""
    prog = (f"symbol p_star : scalar\nobserve cost_miss({COST_MISS})\n"
            f"observe cost_treat({COST_TREAT})\n"
            f"constrain p_star * cost_miss = cost_treat\nsolve for {{ p_star }}\n")
    out = run_check_or_solve(cli, prog, "solve")
    for a in out.get("assignments", []):
        if a["name"] == "p_star":
            return float(a["value"])
    raise RuntimeError(f"break-even did not solve: {out}")


def can_wait_for_culture(cli: Path) -> dict:
    """Check culture_hours <= deadline_hours. UNSAT => cannot wait (must act now)."""
    prog = (f"symbol wait_strategy : scalar\nobserve culture_hours({CULTURE_HOURS})\n"
            f"observe deadline_hours({DEADLINE_HOURS})\n"
            f"constrain wait_strategy = culture_hours\n"
            f"constrain wait_strategy <= deadline_hours\ncheck\n")
    return run_check_or_solve(cli, prog, "check")


def recommend(case_id: str, cli: Path, p_override: float | None = None) -> dict:
    # 1. the differential (warm path, 0 model calls) - or an explicit posterior to
    # evaluate the policy at any P(bacterial) (e.g. a calibrated value where viral
    # is more probable, to show the cost+time override).
    if p_override is not None:
        p_bacterial = p_override
        most_probable = "bacterial_meningitis" if p_bacterial >= 0.5 else "viral_meningitis"
    else:
        ir = json.loads((IR_DIR / f"{case_id}.json").read_text())
        observe_adj, _, _ = ir_mod.ir_to_adj(ir, ir_mod.load_domains())
        dec = decide_mod.decide(case_id, observe_adj, cli)
        p_bacterial = dec["posteriors"].get("bacterial_meningitis", 0.0)
        most_probable = dec["leader"]

    # 2. the SOLVED treatment constraints.
    p_star = cost_breakeven(cli)
    timing = can_wait_for_culture(cli)
    must_act_now = timing.get("outcome") == "unsat"   # cannot defer past the deadline
    cost_says_treat = p_bacterial >= p_star

    treat = cost_says_treat   # treat iff probability clears the cost break-even
    action = "TREAT EMPIRICALLY NOW" if treat else "WITHHOLD; observe / await results"
    # The interesting case: probability favors viral, but cost+time force treatment.
    overrides_argmax = treat and most_probable != "bacterial_meningitis"

    return {
        "case_id": case_id,
        "answer_time_model_calls": 0,
        "p_bacterial": round(p_bacterial, 4),
        "most_probable_dx": most_probable,
        "cost_breakeven_p_star": p_star,
        "treat_threshold_met": cost_says_treat,
        "time_constraint": {"can_wait_for_culture": not must_act_now,
                            "check": timing, "deadline_hours": DEADLINE_HOURS,
                            "culture_hours": CULTURE_HOURS},
        "must_decide_now": must_act_now,
        "action": action,
        "overrides_argmax_probability": overrides_argmax,
        "justification": (
            f"P(bacterial)={p_bacterial:.4f} >= cost break-even p*={p_star} "
            f"(missing bacterial costs {COST_MISS}x empirical treatment), and waiting "
            f"for culture ({CULTURE_HOURS} h) violates the door-to-antibiotic deadline "
            f"({DEADLINE_HOURS} h) [time constraint UNSAT, IIS core {timing.get('core')}] "
            f"-> act now." if treat else
            f"P(bacterial)={p_bacterial:.4f} < cost break-even p*={p_star}; treatment not indicated."),
    }


def main(argv: list[str]) -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("treat: adj-lang-cli not built (cargo build -p adj-lang-cli)", file=sys.stderr)
        return 3
    p_override = None
    if "--p" in argv:
        p_override = float(argv[argv.index("--p") + 1])
    case_id = next((a for a in argv if not a.startswith("--") and a != str(p_override)),
                   "case_preculture_ambiguous")
    rec = recommend(case_id, cli, p_override)
    print(json.dumps(rec, indent=2))
    print(f"\n  {rec['case_id']}: most-probable dx = {rec['most_probable_dx']} "
          f"(P(bacterial)={rec['p_bacterial']}) -> ACTION: {rec['action']}")
    if rec["overrides_argmax_probability"]:
        print("  ^ the cost+time constraints OVERRIDE the argmax probability: "
              "viral is more probable, but you treat for bacterial now.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
