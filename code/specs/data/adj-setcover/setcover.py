#!/usr/bin/env python3
"""setcover.py — a DOMAIN-AGNOSTIC minimum-cost set-cover, solved by the ADJ engine.

This is the generic core under MYCIN's drug-regimen deriver, lifted out of medicine
so any domain can use it. You give it:

  - ELEMENTS (each with a cost + optional exclusion tags),
  - REQUIREMENTS to cover,
  - single-element COVERAGE edges (element → requirements it covers alone),
  - n-ary COMBINATIONS (a *subset* of elements that jointly covers a requirement
    that none covers alone — e.g. two drugs in synergy, or two security controls
    in defense-in-depth),
  - DEFEATERS (observed facts that VOID a specific coverage edge — e.g. a culture
    showing resistance, or a control with a known bypass),
  - active EXCLUSIONS (tags that remove elements — e.g. an allergy, or a policy ban),

and it emits an adj-lang integer program, solves it with `adj-lang-cli`
(`adj-constraint-solver`'s native min-cost set-cover), and returns the cheapest set
of elements covering every requirement — or reports that no cover exists.

It is **deterministic and pure**, so the result is CONTENT-ADDRESSED CACHED: the key
is a hash of the whole spec, so a recurring scenario is a cache hit, and editing any
fact (cost, edge, defeater, …) changes the hash and re-derives. The expensive solve
runs once per distinct input.

The n-ary combination is a k+1-clause AND-linearization (`y = AND(elements)`); the
engine (adj-constraint-solver ≥ 0.10) keeps it on the scalable SAT path. Defeasance
needs no engine support — a defeated edge is simply dropped before emitting.

This module is medicine-agnostic; see `examples/drug_regimen.py` and
`examples/security_controls.py` for two callers in different domains.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

# Element/requirement names become adj-lang identifiers, so they must be single
# lowercase tokens — validated before they ever reach the emitted program.
TOKEN_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")


@dataclass(frozen=True)
class Combination:
    """A set of elements that JOINTLY cover `covers` (n-ary; none covers it alone)."""
    elements: tuple[str, ...]
    covers: str


@dataclass
class SetCoverSpec:
    """A minimum-cost set-cover problem, domain-agnostic."""
    costs: dict[str, int]                         # element -> preference cost (≥ 0)
    requirements: list[str]                       # the things to cover
    covers: dict[str, list[str]]                  # element -> requirements it covers alone
    combinations: list[Combination] = field(default_factory=list)
    defeated: list[tuple[str, str]] = field(default_factory=list)  # (element, requirement) edges voided
    excluded_by: dict[str, list[str]] = field(default_factory=dict)  # element -> exclusion tags
    exclusions: list[str] = field(default_factory=list)  # active exclusion tags

    def canonical(self) -> str:
        """A canonical JSON serialization — the cache key derives from this, so it
        must be stable under dict/list ordering."""
        return json.dumps({
            "costs": dict(sorted(self.costs.items())),
            "requirements": sorted(self.requirements),
            "covers": {k: sorted(v) for k, v in sorted(self.covers.items())},
            "combinations": sorted([[sorted(c.elements), c.covers] for c in self.combinations]),
            "defeated": sorted([list(d) for d in self.defeated]),
            "excluded_by": {k: sorted(v) for k, v in sorted(self.excluded_by.items())},
            "exclusions": sorted(self.exclusions),
        }, separators=(",", ":"))

    def content_hash(self) -> str:
        return hashlib.sha256(self.canonical().encode()).hexdigest()[:16]


@dataclass
class SolveResult:
    selected: list[str] | None     # the chosen elements, or None if no cover exists
    cost: float | None
    outcome: str                   # "optimal" | "infeasible" | <solver outcome>
    used_combinations: list[Combination]
    cached: bool = False


def find_cli() -> Path | None:
    """Locate the built adj-lang-cli (PATH, then the workspace target dirs)."""
    p = shutil.which("adj-lang-cli")
    if p:
        return Path(p)
    here = Path(__file__).resolve()
    for parent in here.parents:
        for prof in ("debug", "release"):
            c = parent / "code/packages/rust/target" / prof / "adj-lang-cli"
            if c.exists():
                return c
    return None


def _validate(name: str, kind: str) -> str:
    if not TOKEN_RE.match(name):
        raise ValueError(f"unsafe {kind} name {name!r} (must match {TOKEN_RE.pattern})")
    return name


def candidates(spec: SetCoverSpec) -> list[str]:
    """Elements not removed by an active exclusion tag."""
    active = set(spec.exclusions)
    return [e for e in spec.costs
            if not (set(spec.excluded_by.get(e, [])) & active)]


def emit_program(spec: SetCoverSpec) -> tuple[str, dict[str, str], bool]:
    """Emit the adj-lang integer program. Returns (text, x_var → element, feasible)."""
    cands = candidates(spec)
    defeated = {tuple(d) for d in spec.defeated}
    for e in cands:
        _validate(e, "element")
    for r in spec.requirements:
        _validate(r, "requirement")

    lines: list[str] = []
    xvar = {e: f"x_{e}" for e in cands}
    for e in cands:
        _validate(xvar[e], "selector")
        lines.append(f"symbol {xvar[e]} : bool")

    # n-ary combinations: an aux `y = AND(members)`, only if every member is a
    # candidate and the combination's coverage isn't fully defeated.
    combo_cover: dict[str, list[str]] = {}
    for i, comb in enumerate(spec.combinations):
        if not all(m in xvar for m in comb.elements):
            continue
        if (("&".join(comb.elements), comb.covers)) in defeated:
            continue
        y = f"y_{i}"
        lines.append(f"symbol {y} : bool")
        for m in comb.elements:                    # y ≤ x_m  (all required)
            lines.append(f"constrain {y} <= {xvar[m]}")
        members = " + ".join(xvar[m] for m in comb.elements)   # y ≥ Σ − (k−1)
        lines.append(f"constrain {y} - ({members}) >= {1 - len(comb.elements)}")
        combo_cover.setdefault(comb.covers, []).append(y)

    feasible = True
    for r in spec.requirements:
        terms = [xvar[e] for e in cands
                 if r in spec.covers.get(e, []) and (e, r) not in defeated]
        terms += combo_cover.get(r, [])
        if terms:
            lines.append(f"constrain {' + '.join(terms)} >= 1   % cover {r}")
        else:
            feasible = False
            lines.append(f"constrain 0 >= 1   % UNCOVERABLE: {r}")

    obj_terms = []
    for e in cands:
        cost = spec.costs[e]
        if not isinstance(cost, int) or isinstance(cost, bool) or cost < 0:
            raise ValueError(f"unsafe cost {cost!r} for {e} (non-negative int required)")
        obj_terms.append(f"{cost} * {xvar[e]}")
    lines.append(f"minimize {' + '.join(obj_terms)}")
    return "\n".join(lines) + "\n", {v: e for e, v in xvar.items()}, feasible


def solve(spec: SetCoverSpec, cli: Path | None = None,
          cache_dir: Path | None = None) -> SolveResult:
    """Solve the min-cost set-cover. Content-addressed cached: a recurring spec is a
    cache hit; any change to the spec changes its hash and re-derives."""
    if cache_dir is not None:
        cache_dir = Path(cache_dir)
        cache_dir.mkdir(parents=True, exist_ok=True)
        cf = cache_dir / f"{spec.content_hash()}.json"
        if cf.exists():
            d = json.loads(cf.read_text())
            return SolveResult(d["selected"], d["cost"], d["outcome"],
                               [Combination(tuple(c[0]), c[1]) for c in d["used_combinations"]],
                               cached=True)

    if cli is None:
        cli = find_cli()
    if cli is None:
        raise RuntimeError("adj-lang-cli not built (cargo build -p adj-lang-cli)")

    program, var_to_elem, _ = emit_program(spec)
    fd, name = tempfile.mkstemp(suffix=".adj", prefix="_setcover_", dir=Path(__file__).resolve().parent)
    p = Path(name)
    try:
        with os.fdopen(fd, "w") as fh:
            fh.write(program)
        r = subprocess.run([str(cli), str(p)], capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"adj-lang-cli exited {r.returncode}: {r.stderr}")
        out = json.loads(r.stdout) if r.stdout else {}
    finally:
        p.unlink(missing_ok=True)

    opt = out.get("optimize", {})
    if opt.get("outcome") != "optimal":
        result = SolveResult(None, None, opt.get("outcome", "unknown"), [])
    else:
        chosen = sorted(var_to_elem[a["name"]] for a in opt.get("assignments", [])
                        if a["name"] in var_to_elem and abs(a["value"] - 1) < 1e-9)
        chosen_set = set(chosen)
        used = [c for c in spec.combinations if set(c.elements) <= chosen_set]
        result = SolveResult(chosen, opt.get("value"), "optimal", used)

    if cache_dir is not None:
        cf.write_text(json.dumps({
            "selected": result.selected, "cost": result.cost, "outcome": result.outcome,
            "used_combinations": [[list(c.elements), c.covers] for c in result.used_combinations],
        }, indent=2) + "\n")
    return result
