#!/usr/bin/env python3
"""identify_bsi.py — bacteremia: identify the organism from the SOURCE, then cover it.

MYCIN-2026 A2. Bacteremia was MYCIN's primary domain, and its signature move was
to reason from the portal of entry: a urinary source implies enteric gram-negative
rods; a central line implies skin flora (CoNS, S. aureus, Candida); an
intra-abdominal source implies gram-negatives + anaerobes + Enterococcus; and so
on. This is the SAME machinery as the meningitis vertical (A1) pointed at a
different site — proving the substrate generalizes across infection sites with no
new engine:

    source + host factors ──► source-id differential ──► significant set ──► set-cover ──► regimen

The set-cover here is GENERIC (no CSF-penetration filter — this is bloodstream,
not CNS): minimum preference-cost set of drugs whose union covers every organism
still in play. 0 answer-time model calls.

Usage:  python3 identify_bsi.py    (runs the demo scenarios)
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from itertools import combinations
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402  (find_cli only)

FORMULARY = json.loads((HERE / "bsi-formulary.json").read_text())
DRUGS = FORMULARY["drugs"]

# Leader is always kept; others kept if still materially in play.
IN_PLAY_SHARE = 0.12
# A finding term/value is a single lowercase identifier (closed vocabulary). We
# validate before composing the .adj program so an externally-sourced finding can
# never inject extra rulebook directives (same boundary discipline as decide.py).
TOKEN_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")


def run_differential(cli: Path, findings: dict[str, str]) -> list[dict]:
    """Compose a case (import the rulebook + observe findings), run the CLI, return
    the ranked organisms. The case file is created with mkstemp (O_EXCL, 0600,
    unpredictable name) next to source-id.adj so the relative import resolves."""
    lines = ['import "source-id.adj"']
    for f, v in findings.items():
        if not (TOKEN_RE.match(f) and TOKEN_RE.match(v)):
            raise ValueError(f"unsafe finding token {f!r}={v!r} (must match {TOKEN_RE.pattern})")
        lines.append(f"observe {f}({v})")
    fd, name = tempfile.mkstemp(suffix=".adj", prefix="_tmp_bsi_", dir=HERE)
    case = Path(name)
    try:
        with os.fdopen(fd, "w") as fh:
            fh.write("\n".join(lines) + "\n")
        r = subprocess.run([str(cli), str(case)], capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"adj-lang-cli exited {r.returncode}: {r.stderr}")
        out = json.loads(r.stdout)
    finally:
        case.unlink(missing_ok=True)
    return out.get("ranked", [])


def significant_set(ranked: list[dict]) -> list[str]:
    """Leader + every organism still materially in play (share ≥ threshold)."""
    if not ranked:
        return []
    keep = [ranked[0]["hypothesis"]]
    keep += [r["hypothesis"] for r in ranked[1:] if r.get("normalized_share", 0) >= IN_PLAY_SHARE]
    return keep


def min_cost_cover(drugs: dict, organisms: list[str], exclusions: set[str]) -> list[str] | None:
    """GENERIC minimum preference-cost (then fewest-drug) set-cover. Candidates are
    drugs not contraindicated for this patient; returns None if no set covers all
    organisms. Tiny formulary → exhaustive subset search is instant. (B1 will lift
    this into the engine as a native, proof-DAG-producing `select` construct.)"""
    cands = [d for d, f in drugs.items() if not (set(f.get("contraindications", [])) & exclusions)]
    need = set(organisms)
    best, best_key = None, None
    for k in range(1, len(cands) + 1):
        for combo in combinations(cands, k):
            cov = set().union(*(drugs[d]["covers"] for d in combo)) if combo else set()
            if need <= cov:
                key = (sum(drugs[d]["tier"] for d in combo), len(combo))
                if best_key is None or key < best_key:
                    best, best_key = list(combo), key
    return best


def identify_and_treat(cli: Path, title: str, findings: dict[str, str],
                       exclusions: set[str]) -> None:
    print("=" * 78 + f"\n{title}\n" + "=" * 78)
    print(f"  findings: {', '.join(f'{k}={v}' for k, v in findings.items())}")
    ranked = run_differential(cli, findings)
    print("  ORGANISM DIFFERENTIAL (which bloodstream pathogen?):")
    for r in ranked[:6]:
        mark = "  <- leading" if r is ranked[0] else ""
        print(f"    {r['hypothesis']:18s} P={r['posterior']:.3f}  share={r['normalized_share']:.3f}{mark}")
    sig = significant_set(ranked)
    print(f"  SIGNIFICANT SET (cover these): {sig}")
    cover = min_cost_cover(DRUGS, sig, exclusions)
    if cover is None:
        print(f"  NO REGIMEN covers {sig} under exclusions {sorted(exclusions)} -> escalate / specialist.\n")
        return
    print(f"  DERIVED EMPIRIC REGIMEN: {' + '.join(cover)}")
    for d in cover:
        covered = sorted(set(DRUGS[d]["covers"]) & set(sig))
        print(f"    - {d:24s} covers {covered}")
    print()


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("identify_bsi: adj-lang-cli not built", file=sys.stderr)
        return 3
    print("formulary: bsi-formulary.json (authored-illustrative; not yet spider-grounded)\n")

    identify_and_treat(cli, "1) Urinary source (urosepsis)",
                       {"infection_source": "urinary"}, set())
    identify_and_treat(cli, "2) Central-line-associated bloodstream infection",
                       {"infection_source": "intravascular_line"}, set())
    identify_and_treat(cli, "3) Intra-abdominal source (polymicrobial)",
                       {"infection_source": "intraabdominal"}, set())
    identify_and_treat(cli, "4) Skin/soft-tissue source in an injection drug user",
                       {"infection_source": "skin_soft_tissue", "injection_drug_use": "present"}, set())
    identify_and_treat(cli, "5) Febrile neutropenia, source unknown — anti-pseudomonal needed",
                       {"infection_source": "unknown", "neutropenia": "present"}, set())
    identify_and_treat(cli, "6) Intra-abdominal source, SEVERE beta-lactam allergy — cover re-derives",
                       {"infection_source": "intraabdominal"}, {"betalactam_allergy_severe"})
    return 0


if __name__ == "__main__":
    sys.exit(main())
