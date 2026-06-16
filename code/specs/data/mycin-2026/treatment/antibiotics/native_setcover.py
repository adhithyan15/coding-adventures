#!/usr/bin/env python3
"""native_setcover.py — derive the regimen as a NATIVE engine integer program.

derive_regimen.py computes the minimum-cost set-cover in Python (an exhaustive
subset search wrapping the CLI). This module instead **emits the set-cover as an
adj-lang integer program** and lets the engine's integer optimizer
(adj-constraint-solver ≥ 0.8) solve it — so the regimen is a first-class,
proof-carrying engine result with the same audit trail the diagnosis has, not a
Python loop. The Python `min_cost_cover` becomes a thin emitter; the solving moves
into the reusable substrate (any future domain that needs set-cover gets it free).

The encoding (all variables boolean, `x ∈ {0,1}`):

  - one selector `x_<drug>` per candidate drug;
  - each grounded COMBINATION rule (e.g. vancomycin + ceftriaxone covers resistant
    pneumococcus, which neither covers alone) is an AND of its members, linearized
    with an auxiliary boolean `y` and the standard `y ≤ a, y ≤ b, y ≥ a+b−1`;
  - one covering constraint per organism: `Σ (drugs covering it) + Σ (combos
    covering it) ≥ 1`;
  - objective: `minimize Σ tier_d · x_d` (combinations add no cost beyond their
    member drugs, already counted).

An organism with no possible coverer yields an infeasible program — the engine
reports it (honest abstention), exactly like the Python deriver returning None.

Usage:  python3 native_setcover.py    (runs the demo + cross-checks vs Python)
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402  (find_cli)
import derive_regimen as reg  # noqa: E402  (the grounded formulary + Python set-cover)

TOKEN_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")


def _sym(prefix: str, *parts: str) -> str:
    name = "_".join((prefix, *parts))
    if not TOKEN_RE.match(name):
        raise ValueError(f"unsafe symbol {name!r}")
    return name


def _safe_reason(reason: str) -> str:
    """Sanitize an exclusion reason before it goes into a `%` line-comment in the emitted
    program — keep only an alnum/space/_-: subset (no newline can escape the comment),
    bounded length. Never interpolate a reason string into the program unvalidated."""
    return re.sub(r"[^A-Za-z0-9 _:-]", "", reason)[:80]


def emit_program(organisms: list[str], exclusions: set[str],
                 defeated: set[tuple[str, str]] = frozenset(),
                 weights: tuple[int, int] = reg.DEFAULT_WEIGHTS,
                 forced_zero: dict[str, str] | None = None) -> tuple[str, dict, bool]:
    """Emit the adj-lang integer program for this cover. Returns (program text,
    {x_var → drug}, feasible?) — feasible is False if some organism has no coverer
    (the program is then trivially infeasible and the engine will say so).

    `defeated` is the set of (drug, organism) coverage edges VOIDED by an observed
    culture sensitivity — an in-vitro result that the isolate is resistant to that
    drug. A defeated edge is dropped before the cover is built, so the regimen
    re-derives around it (MYCIN's "culture-directed" refinement of empiric therapy).
    A combination is defeated for an organism if any of its members is resistant to
    that organism (the synergy rationale is undercut).

    `forced_zero` maps a drug → the REASON it is unavailable for this patient, and every
    such drug is pinned out by an EXPLICIT engine constraint `x_d <= 0   % excluded (reason)`
    — NOT silently dropped from the candidate set in Python. This unifies every exclusion
    family as auditable constraints visible in the emitted program:
      - dose-infeasible (CC-2): no safe-and-effective dose under the chart's renal/interaction
        risks (the efficacy floor exceeds the toxicity ceiling);
      - contraindicated (CC-3): e.g. a drug contraindicated in pregnancy;
      - step-therapy (CC-6): a payer won't reimburse the drug until a prerequisite is tried
        (the `x_Y ≤ tried_X` precedence with the known-untried `tried_X = 0` folded in).
    A forced-zero drug keeps its selector variable (so the reason is on the record and any
    covering combination it belongs to is correctly disabled via `y <= x_d`); the constraint,
    not its absence, removes it — so the engine, not Python, owns the exclusion + infeasibility."""
    forced_zero = forced_zero or {}
    cands = list(reg.candidates(exclusions))
    lines: list[str] = []
    xvar = {d: _sym("x", d) for d in cands}
    var_to_drug = {v: d for d, v in xvar.items()}
    for v in xvar.values():
        lines.append(f"symbol {v} : bool")

    # Linearize each applicable combination rule into an AND auxiliary.
    combo_cover: dict[str, list[str]] = {}  # organism → [aux y vars]
    for rule in reg.COMBINATIONS:
        ds = rule["drugs"]
        if not all(d in xvar for d in ds):
            continue  # a member is excluded → the combination is unavailable
        if any((d, rule["covers"]) in defeated for d in ds):
            continue  # a resistant member breaks the combination's coverage
        y = _sym("y", *ds)
        lines.append(f"symbol {y} : bool")
        for d in ds:  # y ≤ x_d  for each member
            lines.append(f"constrain {y} <= {xvar[d]}")
        # y ≥ Σ x_d − (k − 1)   →   y is 1 only when every member is selected
        lines.append(f"constrain {y} - ({' + '.join(xvar[d] for d in ds)}) >= {1 - len(ds)}")
        combo_cover.setdefault(rule["covers"], []).append(y)

    feasible = True
    for org in organisms:
        # Validate before interpolating: an organism name reaches the emitted
        # program text (even inside a `%` line-comment, which a newline would
        # escape). Uphold the closed-vocabulary contract — never interpolate a
        # field from the formulary unvalidated, even across the CAS trust boundary.
        if not TOKEN_RE.match(org):
            raise ValueError(f"unsafe organism token {org!r} (must match {TOKEN_RE.pattern})")
        # A drug covers `org` unless a culture sensitivity defeated that edge.
        terms = [xvar[d] for d in cands
                 if org in reg.DRUGS[d]["covers"] and (d, org) not in defeated]
        terms += combo_cover.get(org, [])
        if terms:
            lines.append(f"constrain {' + '.join(terms)} >= 1   % cover {org}")
        else:
            feasible = False  # no drug or combination covers this organism
            lines.append(f"constrain 0 >= 1   % UNCOVERABLE: {org}")

    # Exclusions as EXPLICIT constraints: every forced-zero drug is pinned out by
    # `constrain x_d <= 0`, with its reason in the comment — so dose-infeasibility (CC-2),
    # contraindication (CC-3), and step-therapy (CC-6) are all auditable in the program and
    # the resulting infeasibility is the engine's verdict, not a Python pre-filter.
    for d in cands:
        if d in forced_zero:
            lines.append(f"constrain {xvar[d]} <= 0   % excluded ({_safe_reason(forced_zero[d])})")

    # CC-4 objective: minimize Σ (w_cost·tier + w_tox·side_effects)·x_d. The coefficient
    # is a non-negative integer (validated below), so this stays in the engine's INTEGER
    # optimizer. weights=(1,0) reproduces the historical tier-only objective exactly.
    w_cost, w_tox = weights
    for w in (w_cost, w_tox):
        if not isinstance(w, int) or isinstance(w, bool) or w < 0:
            raise ValueError(f"unsafe objective weight {w!r} (must be a non-negative int)")
    obj_terms = []
    for d in cands:
        tier = reg.DRUGS[d]["tier"]
        tox = reg.DRUGS[d].get("side_effects", 0)
        for fld, val in (("tier", tier), ("side_effects", tox)):
            if not isinstance(val, int) or isinstance(val, bool) or val < 0:
                raise ValueError(f"unsafe {fld} {val!r} for {d} (must be a non-negative int)")
        coeff = w_cost * tier + w_tox * tox
        obj_terms.append(f"{coeff} * {xvar[d]}")
    lines.append(f"minimize {' + '.join(obj_terms)}")
    return "\n".join(lines) + "\n", var_to_drug, feasible


def solve(cli: Path, organisms: list[str], exclusions: set[str],
          defeated: set[tuple[str, str]] = frozenset(),
          weights: tuple[int, int] = reg.DEFAULT_WEIGHTS,
          forced_zero: dict[str, str] | None = None) -> dict:
    """Run the emitted program through the engine; return the engine's regimen.
    `defeated` carries culture-sensitivity results (resistant drug→organism edges);
    `weights`=(w_cost, w_tox) is the CC-4 cost+side-effect objective blend (default (1,0));
    `forced_zero` maps drug→reason for every drug pinned out by an explicit `x_d <= 0`
    constraint (dose-infeasible / contraindicated / step-therapy)."""
    program, var_to_drug, _ = emit_program(organisms, exclusions, defeated, weights, forced_zero)
    fd, name = tempfile.mkstemp(suffix=".adj", prefix="_tmp_native_", dir=HERE)
    p = Path(name)
    try:
        with os.fdopen(fd, "w") as fh:
            fh.write(program)
        r = subprocess.run([str(cli), str(p)], capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"adj-lang-cli exited {r.returncode}: {r.stderr}")
        try:
            out = json.loads(r.stdout) if r.stdout else {}
        except json.JSONDecodeError as e:
            raise RuntimeError(f"adj-lang-cli emitted non-JSON output: {e}") from e
    finally:
        p.unlink(missing_ok=True)
    opt = out.get("optimize", {})
    if opt.get("outcome") != "optimal":
        return {"regimen": None, "outcome": opt.get("outcome"), "iis": opt.get("core")}
    chosen = sorted(
        var_to_drug[a["name"]]
        for a in opt.get("assignments", [])
        if a["name"] in var_to_drug and abs(a["value"] - 1) < 1e-9
    )
    # CC-4 objective breakdown — recovered from the chosen drugs for provenance: the
    # cost (Σ tier) and side-effect (Σ side_effects) components that sum (under `weights`)
    # to the engine's reported objective value. `cost` stays the engine's optimal value
    # (back-compatible: under default weights it is exactly Σ tier, as before).
    w_cost, w_tox = weights
    cost_component = sum(reg.DRUGS[d]["tier"] for d in chosen)
    tox_component = sum(reg.DRUGS[d].get("side_effects", 0) for d in chosen)
    return {"regimen": chosen, "outcome": "optimal", "cost": opt.get("value"),
            "binding": opt.get("binding"),
            "objective": {"weights": {"w_cost": w_cost, "w_tox": w_tox},
                          "cost": cost_component, "side_effects": tox_component,
                          "total": w_cost * cost_component + w_tox * tox_component}}


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("native_setcover: adj-lang-cli not built", file=sys.stderr)
        return 3
    print(f"formulary: {reg.FORMULARY_SOURCE}")
    print("solving each regimen as a NATIVE engine integer program "
          "(adj-constraint-solver integer optimizer)\n")
    scenarios = [
        ("Adult community", reg.SCENARIOS["adult_community"], set()),
        ("Over-50 / immunocompromised", reg.SCENARIOS["over_50_or_immunocompromised"], set()),
        ("Post-neurosurgical", reg.SCENARIOS["post_neurosurgical_or_shunt"], set()),
        ("Severe beta-lactam allergy (adult community)",
         reg.SCENARIOS["adult_community"], {"betalactam_allergy_severe"}),
    ]
    ok = True
    for title, organisms, excl in scenarios:
        res = solve(cli, organisms, excl)
        # Cross-check against the Python set-cover: the engine must agree on COST.
        py = reg.min_cost_cover(reg.candidates(excl), organisms)
        py_cost = sum(reg.DRUGS[d]["tier"] for d in py) if py else None
        print("=" * 74 + f"\n{title}\n  organisms: {organisms}")
        if excl:
            print(f"  exclusions: {sorted(excl)}")
        if res["regimen"] is None:
            print(f"  ENGINE: NO REGIMEN ({res['outcome']}) -> escalate / specialist"
                  + (f" [IIS {res['iis']}]" if res.get("iis") else ""))
            agree = py is None
        else:
            print(f"  ENGINE regimen: {' + '.join(res['regimen'])}  (cost {res['cost']:.0f})")
            agree = py is not None and abs(py_cost - res["cost"]) < 1e-9
        print(f"  Python set-cover cost: {py_cost}  ->  "
              + ("AGREE" if agree else "*** DISAGREE ***"))
        ok = ok and agree
        print()
    print("engine vs Python: " + ("all agree — native set-cover verified" if ok else "MISMATCH"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
