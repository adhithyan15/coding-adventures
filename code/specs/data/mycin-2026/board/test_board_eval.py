#!/usr/bin/env python3
"""test_board_eval.py — REL-5 board-eval scoreboard tests.

Pins the defensibility contract: on covered items the harness answers correctly
WITH a proof; on deliberately-uncovered items it abstains; it NEVER fabricates
(wrong == 0). Also pins that grounded-coverage tracks the edges' trust tier — the
live number a grounding PR moves (today 0%, all authored-debt).

Run:  python3 test_board_eval.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import board_eval as be  # noqa: E402


def _card():
    items = json.loads((HERE / "items.json").read_text())["items"]
    return be.score(items), items


def test_never_fabricates() -> None:
    # Holds with OR without the engine: absent the CLI every engine-backed item
    # abstains (UNKNOWN), so wrong stays 0 — the harness never fabricates a fallback.
    card, _ = _card()
    assert card.summary()["wrong"] == 0, "a wrong answer is a fabrication — the one hard failure"


def test_covered_items_answer_correctly_with_a_proof() -> None:
    if not be.cli_available():
        return  # recall is answered by the native engine now; skip in a Python-only env
    card, _ = _card()
    by_id = {r.item_id: r for r in card.results}
    assert by_id["tay_sachs_enzyme"].outcome == "correct"
    assert by_id["tay_sachs_enzyme"].answer == "hexosaminidase_a"
    # A correct recall answer carries the citing edge's trust tier (its proof).
    assert by_id["tay_sachs_enzyme"].trust is not None
    # The bank spans TWELVE recall domains: 18 IEM + 8 vitamin + 8 anemia + 8 endocrine
    # + 11 coagulation (REL-13) + 13 microbiology (MICRO) + 9 pharmacology (PHARM) + 7
    # immunology (IMMUNO) + 7 genetics (GENETICS) + 3 rheumatology (RHEUM) + 3 oncology
    # (ONCO) + 3 histology (HISTO) = 98 covered recall items, all over one merged store.
    # MICRO/PHARM/IMMUNO/GENETICS/RHEUM/ONCO/HISTO are ADJ-ONLY domains: their *-edges.adj
    # are hand-authored libraries carrying byte-provenance inline (no gate/JSON/manifest).
    # RHEUM/ONCO/HISTO are Tier-2 organ-system domains where a subject binds several
    # answers, so the board scores the top binding per subject (the libraries + their
    # tests cover all edges).
    recall_correct = [r for r in card.results if r.outcome == "correct" and r.tactic == "recall"]
    assert len(recall_correct) == 126
    assert by_id["fabry_enzyme"].outcome == "correct"               # IEM
    assert by_id["thiamine_disease"].answer == "beriberi"           # vitamin
    assert by_id["ida_mcv"].answer == "microcytic"                  # anemia
    assert by_id["cortisol_gland"].answer == "adrenal_cortex"       # endocrine (REL-12)
    assert by_id["adh_def"].answer == "central_diabetes_insipidus"
    assert by_id["hemophilia_a_factor"].answer == "factor_viii"     # coagulation (REL-13)
    assert by_id["vwd_test"].answer == "bleeding_time"              # coag — prolonged_test relation
    assert by_id["micro_saureus_gram"].answer == "gram_positive"   # microbiology (MICRO)
    assert by_id["micro_vibrio_causes"].answer == "cholera"        # micro — causes relation
    assert by_id["micro_saureus_gram"].trust == "authoritative"    # ADJ-only edge, grounded inline
    assert by_id["pharm_metformin_class"].answer == "biguanide"    # pharmacology (PHARM)
    assert by_id["pharm_opioid_antidote"].answer == "naloxone"     # pharm — antidote_for relation
    assert by_id["pharm_metformin_class"].trust == "authoritative"  # ADJ-only edge, grounded inline
    assert by_id["immuno_as_hla"].answer == "hla_b27"             # immunology (IMMUNO)
    assert by_id["immuno_type1_mediator"].answer == "ige"         # immuno — mediated_by relation
    assert by_id["immuno_as_hla"].trust == "authoritative"        # ADJ-only edge, grounded inline
    assert by_id["genetics_hd_repeat"].answer == "cag"           # genetics (GENETICS)
    assert by_id["genetics_pws_imprint"].answer == "paternal"    # genetics — imprinting relation
    assert by_id["genetics_hd_gene"].answer == "htt"             # gene_defect shared w/ IMMUNO, disjoint subj
    assert by_id["genetics_hd_repeat"].trust == "authoritative"   # ADJ-only edge, grounded inline
    assert by_id["rheum_sle_ab"].answer == "anti_dsdna"          # rheumatology (RHEUM, Tier-2)
    assert by_id["rheum_gpa_ab"].answer == "pr3"                 # rheum — autoantibody association
    assert by_id["rheum_sle_ab"].trust == "authoritative"        # ADJ-only edge, grounded inline
    assert by_id["rheum_ra_ab"].answer == "anti_ccp"            # primary-source backfill: RA→anti-CCP
    assert by_id["rheum_sjo_ab"].answer == "anti_ro"           # primary-source backfill: Sjögren→anti-Ro/SSA
    assert by_id["rheum_ra_ab"].trust == "authoritative"        # previously deferred, now grounded
    assert by_id["onco_ovarian_marker"].answer == "ca_125"       # oncology (ONCO, Tier-2)
    assert by_id["onco_hcc_marker"].answer == "alpha_fetoprotein"  # onco — tumor_marker relation
    assert by_id["onco_ovarian_marker"].trust == "authoritative"  # ADJ-only edge, grounded inline
    assert by_id["histo_rs_cond"].answer == "hodgkin_lymphoma"   # histology (HISTO, Tier-2)
    assert by_id["histo_heinz_cond"].answer == "g6pd_deficiency"  # histo — seen_in (finding->condition)
    assert by_id["histo_rs_cond"].trust == "authoritative"       # ADJ-only edge, grounded inline
    assert by_id["histo_auer_cond"].answer == "acute_promyelocytic_leukemia"  # primary-source backfill
    assert by_id["histo_psammoma_cond"].answer == "meningioma"   # primary-source backfill (psammoma→meningioma)
    assert by_id["cardio_mr_lesion"].answer == "mitral_regurgitation"  # cardiology (CARDIO, Tier-2)
    assert by_id["cardio_as_lesion"].answer == "aortic_stenosis"  # cardio — murmur_indicates (murmur->lesion)
    assert by_id["cardio_mr_lesion"].trust == "authoritative"    # ADJ-only edge, grounded inline
    assert by_id["neuro_broca_def"].answer == "nonfluent_aphasia"  # neurology (NEURO, Tier-2)
    assert by_id["neuro_stn_def"].answer == "hemiballismus"      # neuro — lesion_causes (site->deficit)
    assert by_id["neuro_broca_def"].trust == "authoritative"     # ADJ-only edge, grounded inline
    assert by_id["gi_celiac_dx"].answer == "celiac_disease"      # gastroenterology (GI, Tier-2)
    assert by_id["gi_hirsch_dx"].answer == "hirschsprung_disease"  # gi — biopsy_finding_in (finding->dx)
    assert by_id["gi_celiac_dx"].trust == "authoritative"        # ADJ-only edge, grounded inline
    assert by_id["derm_em_dx"].answer == "erythema_multiforme"   # dermatology (DERM, Tier-2)
    assert by_id["derm_psoriasis_dx"].answer == "psoriasis"      # derm — skin_finding_in (finding->dx)
    assert by_id["derm_em_dx"].trust == "authoritative"          # ADJ-only edge, grounded inline
    assert by_id["resp_silica_dx"].answer == "silicosis"         # respiratory (RESP, Tier-2)
    assert by_id["resp_asbestos_dx"].answer == "asbestosis"      # resp — inhalation_causes (exposure->dz)
    assert by_id["resp_silica_dx"].trust == "authoritative"      # ADJ-only edge, grounded inline
    assert by_id["resp_coal_dx"].answer == "coal_workers_pneumoconiosis"  # primary-source-first: no deferral
    assert by_id["resp_coal_dx"].trust == "authoritative"        # grounded from CDC/NIOSH (cdc.gov), not StatPearls


def test_uncovered_items_abstain_not_fabricate() -> None:
    card, _ = _card()
    by_id = {r.item_id: r for r in card.results}
    # Diseases genuinely absent from the graph must abstain, not fabricate.
    assert by_id["wilson_disease_enzyme"].outcome == "abstained"
    assert by_id["wilson_disease_enzyme"].answer is None
    assert by_id["menkes_enzyme"].outcome == "abstained"
    # A platelet disorder isn't in the coagulation-FACTOR graph → abstain, never fabricate.
    assert by_id["bernard_soulier_factor"].outcome == "abstained"


def test_defensibility_is_full() -> None:
    if not be.cli_available():
        return  # accuracy-on-attempted needs the engine to attempt; skip Python-only
    card, _ = _card()
    # Every item is either correct-with-proof or an honest abstention.
    assert card.summary()["defensibility"] == 1.0
    assert card.summary()["accuracy_on_attempted"] == 1.0


def test_grounded_coverage_is_the_live_grounding_number() -> None:
    if not be.cli_available():
        return  # recall (and its grounding signal) is answered by the native engine
    card, _ = _card()
    s = card.summary()
    # REL-13b spider-grounded the coagulation domain, and REL-14 re-grounded its lone
    # holdout (factor_deficiency__factor_vii_deficiency): the original byte_quote pinned
    # the disorder's onset/category but not the deficiency-OF-factor identity, so verify
    # landed at direction_only. REL-14 found a stronger source whose verbatim span self-
    # contains the relation ("Factor VII deficiency is a bleeding disorder characterized
    # by a lack in the production of factor VII"), lifting it to authoritative. The ADJ-only
    # domains MICRO (13 microbiology), PHARM (9 pharmacology), IMMUNO (7 immunology),
    # GENETICS (7 genetics), RHEUM (3 scored rheumatology), ONCO (3 scored oncology), and
    # HISTO (3 scored histology) edges — all spider-grounded to NCBI StatPearls byte-stable
    # spans — add 45 more authoritative recall answers. 97 of the 98 recall answers across
    # all TWELVE domains now cite an authoritative edge: grounded-coverage 99%. ONE holdout
    # stays consensus + FLAG (direction_only) — the adversarial verify could not pin it
    # verbatim, so the framework declines to claim grounding it cannot defend, by design:
    # cortisol_def (endocrine, deficiency_syndrome__cortisol — the only verbatim spans frame
    # cortisol deficiency as a consequence/feature of Addison disease, not the named-syndrome identity).
    assert s["grounded_coverage"] == round(125 / 126, 4)   # 0.9921
    assert s["grounded_correct"] == 125
    by_id = {r.item_id: r for r in card.results}
    assert by_id["tay_sachs_enzyme"].trust == "authoritative"      # IEM
    assert by_id["ida_mcv"].trust == "authoritative"               # anemia
    assert by_id["cortisol_gland"].trust == "authoritative"        # endocrine, grounded (REL-12b)
    assert by_id["hemophilia_a_factor"].trust == "authoritative"   # coagulation, grounded (REL-13b)
    assert by_id["factor7_def_factor"].trust == "authoritative"    # coagulation, re-grounded (REL-14)
    assert by_id["cortisol_def"].trust == "consensus"              # endocrine direction_only holdout


def test_gate_exit_code_zero_when_no_fabrication() -> None:
    assert be.main(["--quiet"]) == 0


# ---- REL-7: differential tactic ----

def test_score_differential_logic() -> None:
    # determinate → commits to the leader: correct iff it matches gold.
    det = {"type": "determinate", "leader": "bacterial_meningitis"}
    assert be.score_differential(det, "bacterial_meningitis") == ("correct", "bacterial_meningitis")
    assert be.score_differential(det, "viral_meningitis") == ("wrong", "bacterial_meningitis")
    # committing a leader when the gold is ABSTAIN is a fabrication (wrong).
    assert be.score_differential(det, "ABSTAIN")[0] == "wrong"
    # kickback / empty → the engine declined to commit: abstain (correct vs ABSTAIN).
    kick = {"type": "kickback", "leader": "bacterial_meningitis", "runner_up": "viral_meningitis"}
    assert be.score_differential(kick, "ABSTAIN") == ("abstained", None)
    assert be.score_differential({"type": "empty"}, "bacterial_meningitis") == ("abstained", None)
    # No decision (CLI unavailable) → abstain, never fabricate.
    assert be.score_differential(None, "bacterial_meningitis") == ("abstained", None)


def test_differential_items_never_fabricate() -> None:
    # Regardless of whether the CLI is built, differential items are correct or
    # abstained — never wrong. (When the binary is absent they abstain.)
    card, _ = _card_with_diff()
    diff = [r for r in card.results if r.tactic == "differential"]
    assert diff, "the bank has differential items"
    assert all(r.outcome in ("correct", "abstained") for r in diff)


def test_differential_runs_natively_when_cli_present() -> None:
    if not be.cli_available():
        return  # skip: Python-only environment without the built Rust CLI
    card, _ = _card_with_diff()
    by_id = {r.item_id: r for r in card.results}
    # Strong CSF → the engine commits to bacterial; equivocal CSF → it abstains.
    assert by_id["meningitis_bacterial_dx"].outcome == "correct"
    assert by_id["meningitis_bacterial_dx"].answer == "bacterial_meningitis"
    assert by_id["meningitis_equivocal_dx"].outcome == "abstained"


# ---- F2: management tactic (chart-as-constraints) ----

def test_score_management_logic() -> None:
    # A regimen matching gold is correct.
    res = {"regimen": ["ceftriaxone", "vancomycin"], "outcome": "optimal"}
    assert be.score_management(res, ["vancomycin", "ceftriaxone"])[0] == "correct"  # order-insensitive
    # A different regimen than gold is wrong.
    assert be.score_management(res, ["ampicillin"])[0] == "wrong"
    # INFEASIBLE when the chart's constraints conflict and gold is "INFEASIBLE" → correct.
    inf = {"regimen": None, "outcome": "infeasible", "conflict": [0]}
    assert be.score_management(inf, "INFEASIBLE") == ("correct", "INFEASIBLE")
    # Fabricating a regimen when the chart should be INFEASIBLE → wrong.
    assert be.score_management(res, "INFEASIBLE")[0] == "wrong"
    # INFEASIBLE when a regimen was expected → honest abstention (declined, not wrong).
    assert be.score_management(inf, ["ceftriaxone"]) == ("abstained", "INFEASIBLE")
    # No result (CLI unavailable) → abstain, never fabricate.
    assert be.score_management(None, ["ceftriaxone"]) == ("abstained", None)


def test_management_items_never_fabricate() -> None:
    card, _ = _card_with_diff()
    mgmt = [r for r in card.results if r.tactic == "management"]
    assert mgmt, "the bank has management items"
    assert all(r.outcome in ("correct", "abstained") for r in mgmt)


def test_management_runs_the_constraint_engine_when_cli_present() -> None:
    if not be.cli_available():
        return  # skip: Python-only environment without the built Rust CLI
    card, _ = _card_with_diff()
    by_id = {r.item_id: r for r in card.results}
    # The chart-as-constraints engine solves a regimen, and proves INFEASIBLE when a
    # β-lactam allergy conflicts with the only covering drugs (constraints made unsat).
    assert by_id["mgmt_adult_community"].outcome == "correct"
    assert by_id["mgmt_adult_community"].answer == "ceftriaxone+vancomycin"
    assert by_id["mgmt_betalactam_allergic"].outcome == "correct"
    assert by_id["mgmt_betalactam_allergic"].answer == "INFEASIBLE"


def _card_with_diff():
    import json
    items = json.loads((HERE / "items.json").read_text())["items"]
    return be.score(items), items


def _run() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"  FAIL  {t.__name__}: {exc}")
    print(f"\ntest_board_eval: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
