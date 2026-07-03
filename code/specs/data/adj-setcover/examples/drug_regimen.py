#!/usr/bin/env python3
"""drug_regimen.py — the medical domain through the SAME generic set-cover library.

A compact, self-contained empiric-meningitis formulary expressed as a generic
`SetCoverSpec` — to show the medical case is just one caller of the domain-agnostic
library (same constructs as `security_controls.py`). The grounded MYCIN formulary
lives at code/specs/data/mycin-2026/treatment/antibiotics/ and is richer; this is
the minimal illustration.

  - organisms      = requirements; drugs = elements (cost = preference tier)
  - combination    = vancomycin + ceftriaxone JOINTLY cover resistant pneumococcus,
                     which neither covers alone (the grounded n-ary fact)
  - defeater       = a culture showing resistance voids that drug's coverage edge
  - exclusion      = a severe beta-lactam allergy removes every beta-lactam

Run:  python3 drug_regimen.py
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from setcover import Combination, SetCoverSpec, find_cli, solve  # noqa: E402

COSTS = {"vancomycin": 1, "ceftriaxone": 1, "ampicillin": 1, "cefepime": 2, "moxifloxacin": 4}
COVERS = {
    "ceftriaxone": ["n_meningitidis"],
    "cefepime": ["n_meningitidis", "pseudomonas"],
    "ampicillin": ["listeria"],
    "moxifloxacin": ["n_meningitidis"],   # beta-lactam-sparing alternative
}
COMBINATIONS = [
    Combination(("vancomycin", "ceftriaxone"), "s_pneumoniae_resistant"),
    Combination(("vancomycin", "cefepime"), "s_pneumoniae_resistant"),
]
# Every beta-lactam carries the exclusion tag.
EXCLUDED_BY = {d: ["betalactam_allergy"] for d in ("ceftriaxone", "ampicillin", "cefepime")}


def scenario(title: str, requirements, exclusions, cli, cache) -> None:
    spec = SetCoverSpec(costs=COSTS, requirements=requirements, covers=COVERS,
                        combinations=COMBINATIONS, excluded_by=EXCLUDED_BY, exclusions=exclusions)
    res = solve(spec, cli=cli, cache_dir=cache)
    print("=" * 70 + f"\n{title}\n" + "=" * 70)
    print(f"  cover: {requirements}" + (f"  | exclusions: {exclusions}" if exclusions else ""))
    if res.selected is None:
        print(f"  NO regimen ({res.outcome}) -> escalate / specialist.")
        return
    print(f"  REGIMEN (cost {res.cost:.0f}): {' + '.join(res.selected)}"
          + ("   [cache hit]" if res.cached else ""))
    for c in res.used_combinations:
        print(f"    + combination {' + '.join(c.elements)} covers {c.covers}")


def main() -> int:
    cli = find_cli()
    if cli is None:
        print("drug_regimen: adj-lang-cli not built", file=sys.stderr)
        return 3
    cache = Path(__file__).resolve().parent / "_cache"
    scenario("1) Adult community (resistant pneumococcus + meningococcus)",
             ["s_pneumoniae_resistant", "n_meningitidis"], [], cli, cache)
    scenario("2) Severe beta-lactam allergy — beta-lactam combination unavailable",
             ["s_pneumoniae_resistant", "n_meningitidis"], ["betalactam_allergy"], cli, cache)
    print("\nSame generic library as the security-controls planner.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
