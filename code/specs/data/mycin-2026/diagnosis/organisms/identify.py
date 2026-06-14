#!/usr/bin/env python3
"""identify.py — IDENTIFY the organism, then COVER it. 0 model calls.

This is the step 1976 MYCIN did before recommending therapy, and the join that
makes the whole vertical end-to-end:

    findings ──► organism-id differential ──► significant-organism set ──► set-cover
                 (which bacterium?)            (what must be covered)       (the regimen)
                 diagnosis/organisms/          probability-thresholded      treatment/
                 organism-id.adj               + epidemiology-in-play        antibiotics/

The organism-identification rulebook ranks the specific pathogens from the
Gram-stain morphology (near-decisive) and the epidemiology (age band, immune
status, exposures). The *significant set* is the leader plus every organism still
materially in play (share ≥ threshold) — exactly MYCIN's "cover the organisms
that could plausibly be there." That set is mapped onto the formulary's organism
vocabulary and handed to the minimum-cost set-cover deriver (derive_regimen.py),
so the regimen is DERIVED from the identification, not hard-coded.

Why "still in play" matters clinically: an older / immunocompromised patient whose
CSF Gram stain shows pneumococcus still gets empiric Listeria coverage, because the
epidemiology keeps Listeria a live possibility the stain did not rule out. The
differential surfaces that; the set-cover acts on it.

Usage:  python3 identify.py        (runs the demo scenarios)
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
ABX = MYCIN / "treatment" / "antibiotics"
WARM = MYCIN / "warm"
sys.path.insert(0, str(ABX))
sys.path.insert(0, str(WARM))
import decide as decide_mod  # noqa: E402  (find_cli)
import derive_regimen as reg  # noqa: E402  (formulary set-cover + dose windows)

# Threshold (normalized share across the differential) above which an organism is
# kept "in play" and must be covered empirically. The leader is always included.
IN_PLAY_SHARE = 0.12

# A finding term/value is a single lowercase identifier (matches the closed
# vocabulary). We validate before composing the .adj program so an externally-
# sourced finding can never inject extra rulebook directives — the same boundary
# discipline decide.py applies to case_id (CASE_ID_RE).
TOKEN_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")

# Map an organism hypothesis (organism-vocab) → the formulary's organism token
# (treatment/antibiotics/formulary.json). Empiric pneumococcus is treated as
# resistant-until-proven-susceptible (the reason vancomycin is added empirically).
ORG_TO_FORMULARY = {
    "s_pneumoniae": "s_pneumoniae_resistant",
    "n_meningitidis": "n_meningitidis",
    "listeria": "listeria",
    "h_influenzae": "h_influenzae",
    "gram_negative_bacilli": "gram_negative",
    "s_aureus": "mrsa",
    # group_b_strep has no empiric token in this meningitis formulary yet — the
    # deriver flags it rather than silently dropping it (honest abstention).
}


def run_differential(cli: Path, findings: dict[str, str]) -> list[dict]:
    """Compose a case (import the rulebook + observe the findings), run the CLI,
    return the ranked organisms. The case file is written INTO the rulebook's
    directory so the relative `import "organism-id.adj"` resolves, then removed."""
    lines = ['import "organism-id.adj"']
    for f, v in findings.items():
        if not (TOKEN_RE.match(f) and TOKEN_RE.match(v)):
            raise ValueError(f"unsafe finding token {f!r}={v!r} (must match {TOKEN_RE.pattern})")
        lines.append(f"observe {f}({v})")
    # The case file must sit next to organism-id.adj for the relative import to
    # resolve. Create it with mkstemp (O_EXCL, mode 0600, unpredictable name) so a
    # predictable-name symlink/clobber/race can't redirect the write.
    fd, name = tempfile.mkstemp(suffix=".adj", prefix="_tmp_identify_", dir=HERE)
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


def significant_set(ranked: list[dict]) -> list[dict]:
    """The leader plus every organism still materially in play (share ≥ threshold)."""
    if not ranked:
        return []
    keep = [ranked[0]]
    keep += [r for r in ranked[1:] if r.get("normalized_share", 0) >= IN_PLAY_SHARE]
    return keep


def identify_and_treat(cli: Path, title: str, findings: dict[str, str],
                       exclusions: set[str], risks: set[str], weight: float) -> None:
    print("=" * 78 + f"\n{title}\n" + "=" * 78)
    print(f"  findings: {', '.join(f'{k}={v}' for k, v in findings.items())}")
    ranked = run_differential(cli, findings)

    print("  ORGANISM DIFFERENTIAL (which bacterium?):")
    for r in ranked:
        mark = "  <- leading" if r is ranked[0] else ""
        print(f"    {r['hypothesis']:22s} P={r['posterior']:.3f}  share={r['normalized_share']:.3f}{mark}")

    sig = significant_set(ranked)
    sig_names = [r["hypothesis"] for r in sig]
    print(f"  SIGNIFICANT SET (cover these): {sig_names}")

    # Map to formulary organism tokens; flag any organism with no empiric token.
    organisms, unmapped = [], []
    for name in sig_names:
        tok = ORG_TO_FORMULARY.get(name)
        (organisms.append(tok) if tok else unmapped.append(name))
    if unmapped:
        print(f"  *** NOT in this formulary (no empiric mapping): {unmapped} -> "
              f"would need a formulary extension; flagging rather than guessing. ***")

    if not organisms:
        print("  NO mappable organism to cover -> escalate / specialist.\n")
        return

    cover = reg.min_cost_cover(reg.candidates(exclusions), organisms)
    if cover is None:
        print(f"  NO REGIMEN covers {organisms} under exclusions {sorted(exclusions)} "
              f"-> escalate / specialist.\n")
        return
    print(f"  DERIVED REGIMEN (covers {organisms}): {' + '.join(cover)}")
    single = set().union(*(reg.DRUGS[d]["covers"] for d in cover)) if cover else set()
    for rule in reg.COMBINATIONS:
        if set(rule["drugs"]) <= set(cover) and rule["covers"] in set(organisms) - single:
            print(f"    + COMBINATION {' + '.join(rule['drugs'])} covers {rule['covers']}")
    for d in cover:
        w = reg.dose_window(cli, d, weight, risks)
        if w["feasible"]:
            print(f"    - {d:13s} dose {w['floor_per_kg']}-{w['ceiling_per_kg']} mg/kg "
                  f"-> {w['mg_range']} (risks: {w['active_risks'] or 'none'})")
        else:
            print(f"    - {d:13s} *** DOSE UNSAT [IIS {w['iis']}] -> switch / adjust ***")
    print()


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("identify: adj-lang-cli not built", file=sys.stderr)
        return 3
    print(f"formulary: {reg.FORMULARY_SOURCE}\n")

    identify_and_treat(
        cli, "1) Adult, CSF Gram stain: gram-positive diplococci",
        {"csf_gram_morphology": "gram_positive_diplococci", "age_band": "adult"},
        set(), set(), 70)

    identify_and_treat(
        cli, "2) Older + immunocompromised, GP diplococci — Listeria STAYS in play",
        {"csf_gram_morphology": "gram_positive_diplococci", "age_band": "older_adult",
         "immunocompromised": "present"},
        set(), set(), 70)

    identify_and_treat(
        cli, "3) Young adult, GN diplococci + petechial rash + dormitory",
        {"csf_gram_morphology": "gram_negative_diplococci", "age_band": "infant_child",
         "petechial_rash": "present", "crowding_exposure": "present"},
        set(), set(), 70)

    identify_and_treat(
        cli, "4) Post-neurosurgical, GP cocci in clusters — staphylococcal",
        {"csf_gram_morphology": "gram_positive_cocci_clusters",
         "recent_neurosurgery_or_shunt": "present", "age_band": "adult"},
        set(), set(), 70)

    identify_and_treat(
        cli, "5) Neonate, no organisms on Gram stain — epidemiology only",
        {"csf_gram_morphology": "none_seen", "age_band": "neonate"},
        set(), set(), 4)

    return 0


if __name__ == "__main__":
    sys.exit(main())
