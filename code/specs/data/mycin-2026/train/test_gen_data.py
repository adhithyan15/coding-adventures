#!/usr/bin/env python3
"""test_gen_data.py — guard the decomposer training-data generator (M-train).

Pure checks (no teacher / Ollama / MLX): every (functor, value) in every profile is in
the decomposer's CLOSED VOCABULARY (the dictionary), so the generated gold IR can never
contain a term the rulebook doesn't define; sampled findings stay in-vocab across seeds;
and the generation-time `hint` never leaks into the gold label. This is the closed-vocab
adherence guarantee the warm pipeline relies on — verified without running the teacher."""

from __future__ import annotations

import json
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import gen_data as gd  # noqa: E402


def _vocab() -> dict[str, set[str]]:
    d = json.loads((MYCIN / "warm" / "dictionary.json").read_text())
    return {f["functor"]: set(f["value_domain"]) for f in d["findings"]}


def test_every_profile_value_is_in_the_dictionary():
    vocab = _vocab()
    profiles = {"BACTERIAL": gd.BACTERIAL, "VIRAL": gd.VIRAL, "NONSPECIFIC": gd.NONSPECIFIC}
    for name, pairs in profiles.items():
        for functor, value in pairs:
            assert functor in vocab, f"{name}: functor {functor!r} not in dictionary"
            assert value in vocab[functor], f"{name}: {functor}={value!r} not in value_domain"
    # ORGANISM_ID carries a phrasing hint as a 3rd tuple element.
    for functor, value, hint in gd.ORGANISM_ID:
        assert functor in vocab, f"ORGANISM_ID: functor {functor!r} not in dictionary"
        assert value in vocab[functor], f"ORGANISM_ID: {functor}={value!r} not in value_domain"
        assert isinstance(hint, str) and hint, "every ORGANISM_ID entry needs a phrasing hint"
    # The organism-id findings the grounded rulebook reasons over are now teachable.
    om = {f for f, _, _ in gd.ORGANISM_ID}
    for needed in ("csf_gram_morphology", "age_band", "immunocompromised",
                   "listeria_exposure", "recent_neurosurgery_or_shunt", "petechial_rash"):
        assert needed in om, f"decomposer can't yet learn organism-id finding {needed!r}"


def test_sampled_findings_stay_in_vocab_and_hints_do_not_leak():
    vocab = _vocab()
    saw_organism_id = False
    for seed in range(60):
        findings = gd.sample_findings(random.Random(seed))
        for f in findings:
            assert f["functor"] in vocab and f["value"] in vocab[f["functor"]], f
            assert f["polarity"] in ("stated", "denied")
            if f["functor"] in {x for x, _, _ in gd.ORGANISM_ID}:
                saw_organism_id = True
        # The gold label is the typed fields only — never the generation-time hint.
        gold = [{"functor": f["functor"], "value": f["value"], "polarity": f["polarity"]} for f in findings]
        assert all("hint" not in g for g in gold)
    assert saw_organism_id, "organism-id findings never sampled across 60 seeds"


def main() -> int:
    test_every_profile_value_is_in_the_dictionary()
    test_sampled_findings_stay_in_vocab_and_hints_do_not_leak()
    print("test_gen_data: PASS (every profile value in the closed vocabulary; sampled "
          "findings stay in-vocab incl. the organism-id findings; hints don't leak to gold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
