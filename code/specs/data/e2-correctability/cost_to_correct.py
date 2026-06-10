#!/usr/bin/env python3
"""E2 — the cost-to-correct model: the cost should be paid ONCE and not recur
over the same facts.

This is the reframed E2 headline. The localize panel (localize_results.json) is a
NULL: a strong reviewer finds the error about equally well in the framework trail
and in plain prose. That null is not a weakness — it CLEARS THE GROUND. Since
*finding* the error costs the same in both arms, the entire cost-to-correct
difference lives in what happens AFTER you find it: fix, propagate, and recurrence.

Cost-to-correct, decomposed (per a load-bearing fact F that is wrong):
    find  : locate the error.                    ~equal both arms (the localize null)
    fix   : change the artifact.                 framework = edit 1 fact; prose = rewrite 1 answer
    prop  : the M-1 OTHER current cases citing F. framework = 0 (re-derive); prose = M-1 rewrites
    recur : the G FUTURE cases citing F.          framework = 0 (F is installed); prose = G re-errors

    framework total  =  1            (paid once, O(1), non-recurring)
    prose total      =  M + G        (recurring, O(M+G), unbounded in time)

Plain prose has NO PERSISTENCE LAYER: each answer is stateless, so a correction to
one answer does nothing for the next case, and a fresh case confidently re-asserts
the same false fact. The framework writes the correction once into the CAS; every
dependent case — present and future — inherits it for free.

This script instantiates the model with the framework-arm data already on disk
(fix_propagate.json). The prose-arm recurrence is shown empirically in
recurring_cost/ (a separate, small run) and folded in here when present.

Run: python3 cost_to_correct.py
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
fp = json.load(open(os.path.join(HERE, "fix_propagate.json")))


def cost_curve(M, G, name):
    """Cumulative cost-to-correct as more cases share fact F.
    framework: 1 forever (the one-time derivation/override). prose: 1 per case (find+fix),
    and every future case re-incurs the error."""
    return {
        "scenario": name,
        "current_cases_sharing_fact_M": M,
        "future_cases_G": G,
        "framework_total_corrections": 1,
        "framework_answer_time_model_calls": 0,
        "prose_total_corrections": M + G,
        "prose_persists": False,
        "asymmetry": f"framework O(1) vs prose O({M}+{G})={M + G}; ratio grows without bound in G",
    }


# instantiate with the two real framework-arm demonstrations
men = fp["RQ2_fix_locality"]["worked_example"]
tax = fp["RQ3_persist_propagate"]["tax_derive_once"]

model = {
    "thesis": "cost-to-correct should be paid ONCE and not recur over the same facts",
    "why_the_localize_null_matters": ("finding the error costs ~the same in both arms (see "
        "localize_results.json), so the ENTIRE cost-to-correct difference is fix+propagate+recurrence"),
    "cost_decomposition": {
        "find": "≈ equal (localize null)",
        "fix": {"framework": "edit 1 fact", "prose": "rewrite 1 answer"},
        "propagate_M_minus_1_current": {"framework": 0, "prose": "M-1 rewrites"},
        "recurrence_G_future": {"framework": 0, "prose": "G re-errors (no persistence)"},
        "total": {"framework": "O(1) paid once", "prose": "O(M+G) recurring"},
    },
    "framework_arm_evidence": {
        "meningitis_single_override": {
            "edits": men["edits_applied_as_one_override"],
            "target_corrected": f"{men['target_case_pre_culture']['base_P']} -> {men['target_case_pre_culture']['edited_P']}",
            "regression_sibling": f"{men['regression_sibling_culture_positive']['base_P']} -> {men['regression_sibling_culture_positive']['edited_P']} (unchanged)",
            "answer_time_model_calls": men["answer_time_model_calls"],
        },
        "tax_derive_once": {
            "compiled_once": tax["compiled_once"],
            "held_out_cases_decided": tax["held_out_cases_decided_on_cpu"],
            "answer_time_model_calls": tax["answer_time_model_calls"],
        },
    },
    "cost_curves": [
        cost_curve(3, 10, "TAX filing-threshold fact shared across cases"),
        cost_curve(2, 10, "meningitis CSF-correlation fact shared across cases"),
    ],
    "prose_arm_recurrence": "see recurring_cost/ (empirical: plain Claude re-commits the same "
                            "fact-error across fresh sibling cases; correction does not persist)",
}

# fold in the empirical prose recurrence if it has been run
emp = os.path.join(HERE, "recurring_cost", "recurrence_results.json")
if os.path.exists(emp):
    model["prose_arm_recurrence_empirical_haiku"] = json.load(open(emp))["summary"]
weak = os.path.join(HERE, "recurring_cost", "recurrence_weak.json")
if os.path.exists(weak):
    w = json.load(open(weak))["summary"]
    model["prose_arm_recurrence_empirical_weak_models"] = w["prose_arm_by_model"]
    model["recurrence_capability_graded_finding"] = w["finding"]

json.dump(model, open(os.path.join(HERE, "cost_to_correct.json"), "w"), indent=1)
print(json.dumps(model, indent=1))
