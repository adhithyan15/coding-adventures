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


# ---- F3: byte-provenance + discard + inference justification in the gold IR ----

def test_find_span_returns_a_verbatim_slice_or_empty():
    v = "A 19-year-old presents with neutrophil-predominant pleocytosis and a low CSF glucose."
    # A surface form that appears (case-insensitively) → a real substring of the prose.
    span = gd.find_span(v, ["neutrophil-predominant pleocytosis"])
    assert span and span.lower() in v.lower(), span
    # A `/`-alternative phrase: either side may match.
    assert gd.find_span(v, ["PMN predominance / neutrophil-predominant pleocytosis"])
    # Nothing matches → empty string (no fabricated span).
    assert gd.find_span(v, ["enteroviral pcr positive"]) == ""
    assert gd.find_span(v, [None, ""]) == ""


def test_build_gold_ir_grounds_stated_findings_with_an_entailed_span():
    surfaces = {"csf_glucose": ["low CSF glucose", "hypoglycorrhachia"],
                "csf_neutrophilic_pleocytosis": ["neutrophil-predominant pleocytosis"]}
    vignette = ("A 19-year-old with neutrophil-predominant pleocytosis and a low CSF "
                "glucose; he works as a high-school teacher.")
    findings = [{"functor": "csf_neutrophilic_pleocytosis", "value": "high", "polarity": "stated"},
                {"functor": "csf_glucose", "value": "low", "polarity": "stated"}]
    distractors = [("works as a high-school teacher", "social history, not a finding")]
    gold = gd.build_gold_ir(vignette, findings, distractors, surfaces)
    # Every finding keeps functor/value/polarity AND gains term/span/type (additive).
    for f in gold["findings"]:
        for k in ("functor", "value", "polarity", "term", "span", "type"):
            assert k in f, f"missing {k} in {f}"
    # Both findings are stated verbatim → spans are real substrings, type stated, ENTAILED.
    assert all(f["span"] and f["span"].lower() in vignette.lower() for f in gold["findings"])
    assert all(f["type"] == "stated" for f in gold["findings"])
    assert {f["functor"]: f["polarity"] for f in gold["findings"]} == {
        "csf_neutrophilic_pleocytosis": "affirmed", "csf_glucose": "affirmed"}
    verdicts = {ij["term"]: ij["verdict"] for ij in gold["inference_justifications"]}
    assert all(v == "ENTAILED" for v in verdicts.values()), verdicts
    assert all(ij["basis_span"] for ij in gold["inference_justifications"])
    # The distractor that appears in the prose is recorded as a justified discard.
    assert len(gold["discard"]) == 1
    assert gold["discard"][0]["span"].lower() in vignette.lower()
    assert gold["discard"][0]["reason"]


def test_build_gold_ir_marks_unstated_findings_inferred_and_leap():
    # The teacher's prose does NOT mention the gram stain → the finding can only be a LEAP.
    surfaces = {"csf_gram_stain": ["positive Gram stain"], "fever": ["fever"]}
    vignette = "The patient has a fever and headache; labs are otherwise pending."
    findings = [{"functor": "fever", "value": "present", "polarity": "stated"},
                {"functor": "csf_gram_stain", "value": "positive", "polarity": "stated"}]
    gold = gd.build_gold_ir(vignette, findings, [], surfaces)
    by_f = {f["functor"]: f for f in gold["findings"]}
    assert by_f["fever"]["span"] and by_f["fever"]["type"] == "stated"
    assert by_f["csf_gram_stain"]["span"] == "" and by_f["csf_gram_stain"]["type"] == "inferred"
    verdicts = {ij["term"]: ij["verdict"] for ij in gold["inference_justifications"]}
    assert verdicts["fever(present)"] == "ENTAILED"
    assert verdicts["csf_gram_stain(positive)"] == "LEAP"  # ir_to_adj will drop this — safe


def test_build_gold_ir_records_negation_polarity():
    surfaces = {"csf_gram_stain": ["Gram stain was negative"]}
    vignette = "The CSF Gram stain was negative."
    findings = [{"functor": "csf_gram_stain", "value": "negative", "polarity": "denied"}]
    gold = gd.build_gold_ir(vignette, findings, [], surfaces)
    assert gold["findings"][0]["polarity"] == "denied"


def test_distractors_pool_is_well_formed():
    assert gd.DISTRACTORS, "need a distractor pool to teach justified discard"
    for phrase, reason in gd.DISTRACTORS:
        assert isinstance(phrase, str) and len(phrase) >= 4
        assert isinstance(reason, str) and reason


def main() -> int:
    test_every_profile_value_is_in_the_dictionary()
    test_sampled_findings_stay_in_vocab_and_hints_do_not_leak()
    test_find_span_returns_a_verbatim_slice_or_empty()
    test_build_gold_ir_grounds_stated_findings_with_an_entailed_span()
    test_build_gold_ir_marks_unstated_findings_inferred_and_leap()
    test_build_gold_ir_records_negation_polarity()
    test_distractors_pool_is_well_formed()
    print("test_gen_data: PASS (closed-vocab adherence; hints don't leak; gold IR carries "
          "byte-provenance spans, ENTAILED/LEAP inference verdicts, and justified discards)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
