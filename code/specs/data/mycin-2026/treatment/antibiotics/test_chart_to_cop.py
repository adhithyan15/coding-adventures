#!/usr/bin/env python3
"""test_chart_to_cop.py — guard the chart→constraint compiler (CC-1).

Pure checks (no engine): the compiler maps chart facts to the right COP inputs, records
provenance for every constraint, and DISCARDS unmapped facts with a reason. Engine-gated
checks (if the adj-lang-cli is built): compiling the four canonical charts reproduces the
existing meningitis regimens — proving set-cover is the special case of the chart-driven
COP — and that a severe β-lactam allergy abstains (INFEASIBLE) exactly like
native_setcover, never inventing a regimen.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import chart_to_cop as cc  # noqa: E402
import decide as decide_mod  # noqa: E402


def test_scenario_selection_and_provenance():
    # Default adult → community scenario; every constraint carries its source fact + rule.
    cop = cc.compile_cop([cc.ChartFact("age_band", "adult", "45M")])
    assert cop.organisms and "n_meningitidis" in cop.organisms
    assert any(c["type"] == "coverage" for c in cop.constraints)
    assert all("from" in c and "rule" in c for c in cop.constraints)
    # Older adult OR immunocompromised → the over-50 set (adds listeria).
    older = cc.compile_cop([cc.ChartFact("age_band", "older_adult")])
    assert "listeria" in older.organisms
    immuno = cc.compile_cop([cc.ChartFact("immune_status", "immunocompromised")])
    assert "listeria" in immuno.organisms
    # Post-neurosurgical is most-specific → wins even alongside an age fact.
    neuro = cc.compile_cop([cc.ChartFact("age_band", "older_adult"),
                            cc.ChartFact("setting", "csf_shunt")])
    assert "pseudomonas" in neuro.organisms


def test_allergy_activates_a_context():
    # CC-3b: a penicillin allergy activates the "penicillin_allergy" CONTEXT (the engine then
    # derives the side-chain-scoped exclusions in derive()) — it no longer adds a blanket token.
    cop = cc.compile_cop([cc.ChartFact("allergy", "penicillin", "rash")])
    assert cop.active_contexts == {"penicillin_allergy"}
    assert not cop.exclusions
    ctx = [c for c in cop.constraints if c["type"] == "context" and c["detail"] == "penicillin_allergy"]
    assert len(ctx) == 1 and ctx[0]["from"] == "allergy=penicillin"


def test_culture_resistance_becomes_defeated_edge():
    cop = cc.compile_cop([cc.ChartFact("culture_resistance", "ceftriaxone:n_meningitidis", "S/I/R panel")])
    assert ("ceftriaxone", "n_meningitidis") in cop.defeated
    assert any(c["type"] == "defeated_edge" for c in cop.constraints)


def test_dose_risk_facts_become_dose_constraints():
    # renal + interaction facts compile into dose-ceiling risks + provenance.
    cop = cc.compile_cop([cc.ChartFact("renal_status", "renal_severe", "eGFR 12"),
                          cc.ChartFact("interaction", "nephrotoxin_interaction", "tacrolimus"),
                          cc.ChartFact("weight", "80")])
    assert cop.risks == {"renal_severe", "nephrotoxin_interaction"}, cop.risks
    assert cop.weight == 80.0
    assert sum(c["type"] == "dose_risk" for c in cop.constraints) == 2
    # an unrecognized renal/interaction value is discarded, not silently turned into a risk.
    bad = cc.compile_cop([cc.ChartFact("interaction", "qt_additive")])
    assert not bad.risks and any("qt_additive" in d["fact"] for d in bad.discards)


def test_hepatic_renal_conjunction_caps_ceftriaxone():
    # CC-2b (ceftriaxone FDA label): hepatic dysfunction ALONE needs no dose adjustment, but
    # combined hepatic + significant renal impairment caps the dose. The compiler must record
    # a hepatic risk yet emit the grounded `hepatorenal` token ONLY when renal is also present.
    hep = cc.compile_cop([cc.ChartFact("hepatic_status", "hepatic_severe", "cirrhosis")])
    assert "hepatic_severe" in hep.risks and "hepatorenal" not in hep.risks, hep.risks
    both = cc.compile_cop([cc.ChartFact("hepatic_status", "hepatic_severe"),
                           cc.ChartFact("renal_status", "renal_moderate")])
    assert "hepatorenal" in both.risks, both.risks
    assert any(c.get("from") == "hepatic_status+renal_status" for c in both.constraints)
    # an unrecognized hepatic value is discarded, not silently turned into a risk.
    bad = cc.compile_cop([cc.ChartFact("hepatic_status", "hepatic_mild")])
    assert not bad.risks and any("hepatic_mild" in d["fact"] for d in bad.discards)

    cli = decide_mod.find_cli()
    if cli is None:
        return
    # End-to-end: the conjunction shrinks ceftriaxone's ceiling but it stays FEASIBLE — a
    # dose-ADJUSTMENT, not a fabricated INFEASIBLE — so the standard regimen survives.
    r = cc.derive(cli, [cc.ChartFact("age_band", "adult"),
                        cc.ChartFact("hepatic_status", "hepatic_severe"),
                        cc.ChartFact("renal_status", "renal_moderate")])
    assert r["regimen"] and set(r["regimen"]) == {"ceftriaxone", "vancomycin"}, r
    assert "ceftriaxone" not in r["dose_infeasible"], r["dose_infeasible"]
    assert any(c.get("from") == "hepatic_status+renal_status" for c in r["constraints"]), r


def test_dose_infeasibility_folds_into_the_cover():
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # Control: no dose risks → vancomycin is dosable → the standard regimen stands.
    base = cc.derive(cli, [cc.ChartFact("age_band", "adult")])
    assert base["regimen"] and "vancomycin" in base["regimen"] and not base["dose_infeasible"]
    # Severe renal + a concurrent nephrotoxin push vancomycin's ceiling below its efficacy
    # floor → no safe+effective dose → excluded from the cover → honest abstention (the
    # combination that covers resistant pneumococcus needs vancomycin), never a toxic dose.
    risky = cc.derive(cli, [cc.ChartFact("age_band", "adult"),
                            cc.ChartFact("renal_status", "renal_severe"),
                            cc.ChartFact("interaction", "nephrotoxin_interaction")])
    assert "vancomycin" in risky["dose_infeasible"], risky["dose_infeasible"]
    assert risky["regimen"] is None and risky["outcome"] == "infeasible", risky
    assert any(c["type"] == "dose_infeasible" for c in risky["constraints"])


def test_pregnancy_activates_a_clinical_context():
    # CC-3 (ADJ-native): compile_cop no longer decides WHICH drugs are contraindicated —
    # that is the engine's job (see test_pregnancy_engine_behaviour). A pregnancy fact only
    # activates the "pregnancy" clinical CONTEXT, recorded as a provenance constraint.
    cop = cc.compile_cop([cc.ChartFact("pregnancy", "present", "28wk")])
    assert cop.active_contexts == {"pregnancy"}
    assert not cop.contraindicated  # populated by the engine in derive(), not here
    ctx = [c for c in cop.constraints if c["type"] == "context"]
    assert len(ctx) == 1 and ctx[0]["from"] == "pregnancy=present" and ctx[0]["detail"] == "pregnancy"
    # A non-'present' pregnancy value activates no context and is discarded (no silent effect).
    cop2 = cc.compile_cop([cc.ChartFact("pregnancy", "unknown")])
    assert not cop2.active_contexts and any("pregnancy" in d["fact"] for d in cop2.discards)


def test_pregnancy_engine_behaviour():
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # Pregnant adult, no allergy → the standard regimen stands (vanc/ceftriaxone aren't
    # pregnancy-contraindicated); moxifloxacin/tmp_smx are flagged contraindicated.
    ok = cc.derive(cli, [cc.ChartFact("age_band", "adult"), cc.ChartFact("pregnancy", "present")])
    assert ok["regimen"] and {"moxifloxacin", "tmp_smx"} <= set(ok["contraindicated"])
    # CC-3b: pregnant + PENICILLIN allergy is now FEASIBLE — a penicillin allergy excludes only
    # penicillins (ampicillin), NOT 3rd-gen cephalosporins, so vancomycin + ceftriaxone stands
    # (ceftriaxone cross-reactivity <1%). ampicillin + the pregnancy drugs are contraindicated.
    pcn = cc.derive(cli, [cc.ChartFact("age_band", "adult"), cc.ChartFact("pregnancy", "present"),
                          cc.ChartFact("allergy", "penicillin")])
    assert pcn["regimen"] and "ceftriaxone" in pcn["regimen"], pcn
    assert {"ampicillin", "moxifloxacin", "tmp_smx"} <= set(pcn["contraindicated"])
    # Pregnant + an UNSPECIFIED whole-class β-lactam allergy → penicillins/cephalosporins/
    # carbapenems all out (only aztreonam survives, which can't cover S. pneumoniae) AND the
    # fluoroquinolone/TMP-SMX alternatives are pregnancy-contraindicated → honest abstention.
    none = cc.derive(cli, [cc.ChartFact("age_band", "adult"), cc.ChartFact("pregnancy", "present"),
                           cc.ChartFact("allergy", "betalactam")])
    assert none["regimen"] is None and none["outcome"] == "infeasible" and none["conflict"] is not None


def test_objective_priority_sets_the_cost_side_effect_weights():
    # CC-4: an objective_priority chart fact selects the (w_cost, w_tox) blend the
    # set-cover minimizes, with provenance. Default (no such fact) stays tier-only (1,0).
    default = cc.compile_cop([cc.ChartFact("age_band", "adult")])
    assert default.weights == (1, 0)
    low_tox = cc.compile_cop([cc.ChartFact("age_band", "adult"),
                              cc.ChartFact("objective_priority", "low_toxicity", "frail, polypharmacy")])
    assert low_tox.weights == (1, 3)
    assert any(c["type"] == "objective" for c in low_tox.constraints)
    # An unknown priority applies nothing and is discarded (no silent default change).
    bad = cc.compile_cop([cc.ChartFact("objective_priority", "cheapest_please")])
    assert bad.weights == (1, 0) and any("objective_priority" in d["fact"] for d in bad.discards)


def test_objective_breakdown_surfaced_with_chart_weights():
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # derive() surfaces the CC-4 objective breakdown, carrying the chart's weights; the
    # total is internally consistent (w_cost·cost + w_tox·side_effects) for any priority.
    r = cc.derive(cli, [cc.ChartFact("setting", "post_neurosurgical"),
                        cc.ChartFact("objective_priority", "low_toxicity")])
    ob = r["objective"]
    assert ob is not None and ob["weights"] == {"w_cost": 1, "w_tox": 3}, ob
    assert ob["total"] == 1 * ob["cost"] + 3 * ob["side_effects"], ob


def test_decide_timing_decision_table():
    # CC-5 (§4) ADJ-NATIVE: the wait-vs-treat-now DECISION is now derived by the engine from
    # the timing.adj precedence ladder (decide_timing wraps timing.derive_timing). Same
    # outcomes as the retired Python if/elif, keyed by (disease acuity, culture, clinical).
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # Time-critical disease (meningitis) → empiric now, high delay_risk, with the GROUNDED
    # time-criticality basis (CC-5b): a verbatim guideline quote + qualitative treat target,
    # NOT the retired (unsupported) ≤60-min numeric threshold.
    t = cc.decide_timing(cli, "meningitis", "pending", "stable")
    assert t["decision"] == "treat_now_empiric" and t["delay_risk"] == "high", t
    assert t["threshold"]["treat_target"] == "as_soon_as_possible", t
    assert t["threshold"]["trust"] == "authoritative", t
    assert "as soon as possible" in t["threshold"]["source"], t  # verbatim guideline quote rides on it
    assert "treat_within_min" not in t["threshold"], "the unsupported ≤60-min overclaim is retired"
    assert t["standing"] == "authoritative"  # the time-critical rule governed
    # A critical patient forces empiric-now even for a routine-acuity disease.
    assert cc.decide_timing(cli, "cellulitis", "pending", "critical")["decision"] == "treat_now_empiric"
    # Stable + non-time-critical + culture pending → awaiting the culture is defensible.
    aw = cc.decide_timing(cli, "cellulitis", "pending", "stable")
    assert aw["decision"] == "await_culture" and aw["delay_risk"] == "low", aw
    # Culture already back → targeted, the wait question is moot.
    assert cc.decide_timing(cli, "meningitis", "resulted", "stable")["decision"] == "targeted_culture_directed"
    # No timing info on a routine disease → conservative treat-now (don't gamble), moderate risk.
    none = cc.decide_timing(cli, "cellulitis", "", "")
    assert none["decision"] == "treat_now_empiric" and none["delay_risk"] == "moderate", none


def test_timing_facts_compile_and_surface_in_derive():
    # culture_status / clinical_status compile into timing inputs with provenance.
    cop = cc.compile_cop([cc.ChartFact("culture_status", "pending", "cultures sent"),
                          cc.ChartFact("clinical_status", "critical", "obtunded, hypotensive")])
    assert cop.culture_status == "pending" and cop.clinical_status == "critical"
    assert sum(c["type"] == "timing_input" for c in cop.constraints) == 2
    # Unrecognized values are discarded, never silently set.
    bad = cc.compile_cop([cc.ChartFact("culture_status", "maybe")])
    assert not bad.culture_status and any("culture_status" in d["fact"] for d in bad.discards)
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # derive() surfaces the timing decision; meningitis is time-critical → empiric now.
    r = cc.derive(cli, [cc.ChartFact("age_band", "adult"), cc.ChartFact("culture_status", "pending")])
    assert r["timing"]["decision"] == "treat_now_empiric" and r["timing"]["delay_risk"] == "high"


def test_step_therapy_facts_compile_and_reimbursement_blocked_logic():
    # CC-6: a step_therapy rule "restricted:prerequisite" + a prior_failed drug compile
    # into the payer-policy COP inputs with provenance.
    cop = cc.compile_cop([cc.ChartFact("step_therapy", "cefepime:meropenem", "payer policy"),
                          cc.ChartFact("prior_failed", "ampicillin", "failed amp last week")])
    assert ("cefepime", "meropenem") in cop.step_therapy
    assert "ampicillin" in cop.tried
    assert any(c["type"] == "step_therapy" for c in cop.constraints)
    assert any(c["type"] == "prior_treatment" for c in cop.constraints)
    bad = cc.compile_cop([cc.ChartFact("step_therapy", "no_colon_here")])
    assert not bad.step_therapy and any("step_therapy" in d["fact"] for d in bad.discards)
    # The precedence x_Y ≤ tried_X is now DERIVED BY THE ENGINE (step_therapy.adj, NAF) —
    # covered by test_step_therapy.py and the engine-gated test_dual_* path below.


def test_dual_clinical_vs_reimbursement_regimen():
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # post-neurosurgical: clinical optimum uses cefepime. A payer step-therapy rule
    # "cefepime needs meropenem tried first" (meropenem NOT tried) → the reimbursement-
    # covered regimen drops cefepime and differs; the clinical regimen is unchanged.
    r = cc.derive(cli, [cc.ChartFact("setting", "post_neurosurgical"),
                        cc.ChartFact("step_therapy", "cefepime:meropenem", "payer policy")])
    assert "cefepime" in r["regimen"]                       # clinical optimum keeps cefepime
    rb = r["reimbursement"]
    assert rb is not None and rb["blocked"] == ["cefepime"] and rb["differs_from_clinical"]
    assert "cefepime" not in (rb["covered_regimen"] or [])  # payer-covered regimen drops it
    assert rb["covered_regimen"] is not None and rb["note"]
    # Once meropenem has been tried/failed, the prerequisite is satisfied → no divergence.
    r2 = cc.derive(cli, [cc.ChartFact("setting", "post_neurosurgical"),
                         cc.ChartFact("step_therapy", "cefepime:meropenem"),
                         cc.ChartFact("prior_failed", "meropenem")])
    assert not r2["reimbursement"]["differs_from_clinical"]
    # No payer rules → no reimbursement block at all (additive, opt-in).
    assert cc.derive(cli, [cc.ChartFact("age_band", "adult")])["reimbursement"] is None


def test_step_therapy_can_be_reimbursement_infeasible_distinct_from_clinical():
    cli = decide_mod.find_cli()
    if cli is None:
        return
    # A step-therapy rule that blocks a CLINICALLY-FORCED drug (vancomycin is the only
    # resistant-pneumococcus coverer) → a clinically valid regimen exists, but it is
    # reimbursement-INFEASIBLE → surfaced distinctly for physician override / appeal.
    r = cc.derive(cli, [cc.ChartFact("age_band", "adult"),
                        cc.ChartFact("step_therapy", "vancomycin:meropenem")])
    assert r["regimen"] is not None                          # clinically feasible
    rb = r["reimbursement"]
    assert rb["covered_regimen"] is None and rb["covered_outcome"] == "infeasible"
    assert rb["differs_from_clinical"] and "appeal" in rb["note"]


def test_unmapped_fact_is_discarded_not_ignored():
    # No silent drops: a fact with no rule lands in discards WITH a reason.
    cop = cc.compile_cop([cc.ChartFact("age_band", "adult"),
                          cc.ChartFact("favorite_color", "blue")])
    assert any("favorite_color" in d["fact"] and d["reason"] for d in cop.discards)
    # An allergen with no grounded allergy context yet is also discarded (no context activated).
    cop2 = cc.compile_cop([cc.ChartFact("allergy", "sulfa")])
    assert not cop2.active_contexts and any("sulfa" in d["fact"] for d in cop2.discards)


def test_engine_reproduces_existing_regimens():
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_chart_to_cop: PASS (pure compiler checks); engine portion SKIPPED (no cli)")
        return
    want = {
        "adult_community": {"ceftriaxone", "vancomycin"},
        "over_50_or_immunocompromised": {"ampicillin", "ceftriaxone", "vancomycin"},
        "post_neurosurgical_or_shunt": {"cefepime", "vancomycin"},
        # CC-3b: a penicillin allergy keeps the 3rd-gen cephalosporin (cross-reactivity <1%),
        # so the regimen is unchanged from adult_community — vancomycin + ceftriaxone.
        "penicillin_allergic_adult": {"ceftriaxone", "vancomycin"},
    }
    for name, regimen in want.items():
        r = cc.derive(cli, cc.CHARTS[name])
        assert r["regimen"] is not None and set(r["regimen"]) == regimen, (name, r["regimen"])
    # An UNSPECIFIED whole-class β-lactam allergy → honest abstention (INFEASIBLE), never a
    # fabricated regimen (only aztreonam survives, which can't cover S. pneumoniae).
    alg = cc.derive(cli, cc.CHARTS["betalactam_allergic_adult"])
    assert alg["regimen"] is None and alg["outcome"] == "infeasible", alg
    assert alg["conflict"] is not None, "infeasible must name the conflicting constraint"
    print("test_chart_to_cop: PASS (compiler provenance + discards; engine reproduces the "
          "4 meningitis regimens as a special case; β-lactam allergy abstains; 0 model calls)")


def main() -> int:
    test_scenario_selection_and_provenance()
    test_allergy_activates_a_context()
    test_culture_resistance_becomes_defeated_edge()
    test_dose_risk_facts_become_dose_constraints()
    test_hepatic_renal_conjunction_caps_ceftriaxone()
    test_objective_priority_sets_the_cost_side_effect_weights()
    test_objective_breakdown_surfaced_with_chart_weights()
    test_decide_timing_decision_table()
    test_timing_facts_compile_and_surface_in_derive()
    test_step_therapy_facts_compile_and_reimbursement_blocked_logic()
    test_dual_clinical_vs_reimbursement_regimen()
    test_step_therapy_can_be_reimbursement_infeasible_distinct_from_clinical()
    test_dose_infeasibility_folds_into_the_cover()
    test_pregnancy_activates_a_clinical_context()
    test_pregnancy_engine_behaviour()
    test_unmapped_fact_is_discarded_not_ignored()
    test_engine_reproduces_existing_regimens()
    return 0


if __name__ == "__main__":
    sys.exit(main())
