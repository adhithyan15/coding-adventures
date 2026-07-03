#!/usr/bin/env python3
"""test_identify.py — guard organism identification → cover → dose. 0 model calls.

Asserts the organism-identification differential ranks the right pathogen from
the Gram-stain morphology, keeps the epidemiologically-live organisms in play,
and that the significant set maps onto the formulary and DERIVES a regimen.
Skips cleanly if adj-lang-cli is not built (mirrors test_warm.py). CI runs this.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402
import identify as ident  # noqa: E402


def test_significant_set_pure() -> None:
    """The leader is always kept; others only if share ≥ threshold (no CLI needed)."""
    ranked = [
        {"hypothesis": "a", "posterior": 0.9, "normalized_share": 0.6},
        {"hypothesis": "b", "posterior": 0.5, "normalized_share": 0.3},
        {"hypothesis": "c", "posterior": 0.1, "normalized_share": 0.05},
    ]
    sig = [r["hypothesis"] for r in ident.significant_set(ranked)]
    assert sig == ["a", "b"], sig
    # Even a low-share leader is kept (we never cover nothing).
    lone = [{"hypothesis": "x", "posterior": 0.2, "normalized_share": 0.02}]
    assert [r["hypothesis"] for r in ident.significant_set(lone)] == ["x"]
    assert ident.significant_set([]) == []


def test_mapping_targets_exist_in_formulary() -> None:
    """Every mapped formulary token must be a real organism the formulary covers."""
    import derive_regimen as reg
    coverable = set().union(*(d["covers"] for d in reg.DRUGS.values()))
    coverable |= {c["covers"] for c in reg.COMBINATIONS}
    for org, tok in ident.ORG_TO_FORMULARY.items():
        assert tok in coverable, f"{org} -> {tok} not coverable by any drug/combination"


def main() -> int:
    test_significant_set_pure()
    test_mapping_targets_exist_in_formulary()
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_identify: PASS (pure checks); CLI checks SKIPPED (adj-lang-cli not built)")
        return 0

    # Gram-positive diplococci → pneumococcus leads decisively.
    ranked = ident.run_differential(cli, {"csf_gram_morphology": "gram_positive_diplococci",
                                           "age_band": "adult"})
    assert ranked[0]["hypothesis"] == "s_pneumoniae", ranked[0]

    # Gram-negative diplococci + rash + crowding → meningococcus leads.
    ranked = ident.run_differential(cli, {"csf_gram_morphology": "gram_negative_diplococci",
                                          "petechial_rash": "present",
                                          "crowding_exposure": "present",
                                          "age_band": "infant_child"})
    assert ranked[0]["hypothesis"] == "n_meningitidis", ranked[0]

    # Older + immunocompromised + pneumococcal stain → Listeria stays in the significant set.
    ranked = ident.run_differential(cli, {"csf_gram_morphology": "gram_positive_diplococci",
                                          "age_band": "older_adult",
                                          "immunocompromised": "present"})
    sig = [r["hypothesis"] for r in ident.significant_set(ranked)]
    assert "s_pneumoniae" in sig and "listeria" in sig, sig

    # That set maps onto the formulary and derives a (non-empty) regimen.
    import derive_regimen as reg
    organisms = [ident.ORG_TO_FORMULARY[n] for n in sig if n in ident.ORG_TO_FORMULARY]
    cover = reg.min_cost_cover(reg.candidates(set()), organisms)
    assert cover and "ampicillin" in cover, (organisms, cover)  # Listeria coverage present

    print("test_identify: PASS (morphology IDs the pathogen; epidemiology keeps "
          "Listeria in play; identify→cover→dose derives the IDSA regimen; 0 model calls)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
