#!/usr/bin/env python3
"""ADJ84 runner — adjudicate every (item, model) pipeline through engine.py and tabulate.

Embeds the ACTUAL Stage-A (rulebook-IR) and Stage-B (input-IR) outputs the Haiku/Opus
sub-agents produced (transcripts in the session / runs/*.json for U6). The engine is the
SAME for all; only the model-produced IR changes. Compares the engine verdict to gold.
"""
import json
import engine

ITEMS = {it["id"]: it for it in json.load(open("items.json"))["items"]}

# --- Stage outputs as produced by the sub-agents (faithful transcription) ---
RUNS = {
 ("U6-overstay", "haiku"): {
  "rb": {"rules": [{"id": "fine", "when": {"days_in_country": ">90", "extension_obtained": "false"}, "then": "fine owed", "source_span": "Overstaying that limit triggers a fine", "provenance": "passage"}],
         "default": {"then": "no fine", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"days_in_country": {"value": 120, "span": "entered the country on January 1 and departed on May 1 of the same year", "type": "inferred"},
                   "extension_obtained": {"value": None, "span": None, "type": "stated"}},
         "uncertainties": ["Whether Priya obtained a stay extension before the 90th day"]}},
 ("U6-overstay", "opus"): {
  "rb": {"rules": [{"id": "fine", "when": {"days_stayed": ">90", "ext": "false"}, "then": "fine owed", "source_span": "Overstaying that limit triggers a fine", "provenance": "passage"},
                   {"id": "waiver", "when": {"days_stayed": ">90", "ext": "true"}, "then": "no fine", "source_span": "unless the visitor obtained an extension before the 90th day", "provenance": "passage"},
                   {"id": "within", "when": {"days_stayed": "<=90"}, "then": "no fine", "source_span": "Visitors may remain in the country for up to 90 days.", "provenance": "passage"}],
         "default": {"then": "no fine", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"days_stayed": {"value": 120, "span": "Priya entered the country on January 1 and departed on May 1 of the same year.", "type": "inferred"},
                   "ext": {"value": None, "span": None, "type": "stated"}},
         "uncertainties": ["The TEXT does not state whether Priya obtained an extension before the 90th day."]}},

 ("U1-waterdamage", "haiku"): {
  "rb": {"rules": [{"id": "sudden", "when": {"cause_mechanism": "sudden_and_accidental"}, "then": "covered", "source_span": "Brightline Insurance covers water damage caused by sudden and accidental events, such as a burst pipe.", "provenance": "passage"},
                   {"id": "gradual", "when": {"cause_mechanism": "gradual_or_neglect"}, "then": "denied", "source_span": "Damage resulting from gradual seepage or long-term maintenance neglect is excluded.", "provenance": "passage"},
                   {"id": "flood", "when": {"damage_type": "flood", "flood_rider_purchased": "false"}, "then": "denied", "source_span": "Flood damage is covered only under a separate rider, which this policyholder has not purchased.", "provenance": "passage"}],
         "default": {"then": "denied", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"damage_type": {"value": "mold", "span": "warped flooring and mold beneath a bathroom", "type": "stated"},
                   "cause_mechanism": {"value": None, "span": None, "type": "inferred"},
                   "flood_rider_purchased": {"value": False, "span": "which this policyholder has not purchased", "type": "stated"}},
         "uncertainties": ["the TEXT does not specify the root cause mechanism for this claim"]}},
 ("U1-waterdamage", "opus"): {
  "rb": {"rules": [{"id": "sudden", "when": {"is_water_damage": "true", "cause_type": "sudden_accidental"}, "then": "covered", "source_span": "Brightline Insurance covers water damage caused by sudden and accidental events, such as a burst pipe.", "provenance": "passage"},
                   {"id": "gradual", "when": {"cause_type": "gradual_seepage"}, "then": "excluded", "source_span": "Damage resulting from gradual seepage or long-term maintenance neglect is excluded.", "provenance": "passage"},
                   {"id": "neglect", "when": {"cause_type": "maintenance_neglect"}, "then": "excluded", "source_span": "Damage resulting from gradual seepage or long-term maintenance neglect is excluded.", "provenance": "passage"},
                   {"id": "flood_rider", "when": {"cause_type": "flood", "flood_rider_purchased": "true"}, "then": "covered", "source_span": "Flood damage is covered only under a separate rider, which this policyholder has not purchased.", "provenance": "passage"},
                   {"id": "flood_norider", "when": {"cause_type": "flood", "flood_rider_purchased": "false"}, "then": "not_covered", "source_span": "Flood damage is covered only under a separate rider, which this policyholder has not purchased.", "provenance": "passage"}],
         "default": {"then": "not_covered", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"cause_type": {"value": None, "span": None, "type": "inferred"},
                   "is_water_damage": {"value": True, "span": "warped flooring and mold beneath a bathroom", "type": "inferred"},
                   "flood_rider_purchased": {"value": False, "span": "which this policyholder has not purchased", "type": "stated"}},
         "uncertainties": ["the text does not state the underlying cause of the water damage"]}},

 ("N1-reimburse", "haiku"): {
  "rb": {"rules": [{"id": "prev_ab_inrange", "when": {"service_type": "preventive_screening", "screening_grade": "*", "network_status": "in_network", "recommended_age_range": "true"}, "then": "100%", "source_span": "Preventive screenings that receive a Grade A or Grade B recommendation from the National Preventive Services Panel are reimbursed at 100%, with the annual deductible waived.", "provenance": "passage"},
                   {"id": "prev_outside", "when": {"service_type": "preventive_screening", "recommended_age_range": "false"}, "then": "80%", "source_span": "However, a preventive screening ordered outside the panel's recommended age range is treated as a standard service and reverts to the 80% in-network rate.", "provenance": "passage"},
                   {"id": "standard", "when": {"service_type": "standard_service", "network_status": "in_network", "deductible_met": "true"}, "then": "80%", "source_span": "The MediCare-Plus plan reimburses most in-network medical services at 80% after the annual deductible is met.", "provenance": "passage"}],
         "default": {"then": "undetermined", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"network_status": {"value": "in_network", "span": "in-network enrollee", "type": "stated"},
                   "service_type": {"value": "preventive_screening", "span": "preventive screening", "type": "inferred"},
                   "screening_grade": {"value": "grade_b", "span": "a colorectal cancer screening that carries a Grade B recommendation", "type": "stated"},
                   "deductible_met": {"value": None, "span": None, "type": "stated"},
                   "age": {"value": 45, "span": "A 45-year-old in-network enrollee", "type": "stated"},
                   "recommended_age_range": {"value": True, "span": "The panel recommends colorectal cancer screening beginning at age 45. A 45-year-old in-network enrollee received a colorectal cancer screening", "type": "inferred"}},
         "uncertainties": []}},
 ("N1-reimburse", "opus"): {
  "rb": {"rules": [{"id": "prev_outside", "when": {"service_type": "preventive_screening", "within_recommended_age_range": "false", "network_status": "in_network"}, "then": "80%", "source_span": "However, a preventive screening ordered outside the panel's recommended age range is treated as a standard service and reverts to the 80% in-network rate.", "provenance": "passage"},
                   {"id": "prev_a", "when": {"service_type": "preventive_screening", "within_recommended_age_range": "true", "panel_recommendation_grade": "grade_a"}, "then": "100%", "source_span": "Preventive screenings that receive a Grade A or Grade B recommendation ... are reimbursed at 100%, with the annual deductible waived.", "provenance": "passage"},
                   {"id": "prev_b", "when": {"service_type": "preventive_screening", "within_recommended_age_range": "true", "panel_recommendation_grade": "grade_b"}, "then": "100%", "source_span": "Preventive screenings that receive a Grade A or Grade B recommendation ... are reimbursed at 100%, with the annual deductible waived.", "provenance": "passage"},
                   {"id": "standard_innet", "when": {"network_status": "in_network"}, "then": "80%", "source_span": "The MediCare-Plus plan reimburses most in-network medical services at 80% after the annual deductible is met.", "provenance": "passage"}],
         "default": {"then": "undetermined", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"network_status": {"value": "in_network", "span": "A 45-year-old in-network enrollee received a colorectal cancer screening", "type": "stated"},
                   "service_type": {"value": "preventive_screening", "span": "received a colorectal cancer screening that carries a Grade B recommendation", "type": "stated"},
                   "panel_recommendation_grade": {"value": "grade_b", "span": "a colorectal cancer screening that carries a Grade B recommendation", "type": "stated"},
                   "within_recommended_age_range": {"value": True, "span": "The panel recommends colorectal cancer screening beginning at age 45. A 45-year-old in-network enrollee received a colorectal cancer screening", "type": "inferred"},
                   "deductible_met": {"value": None, "span": None, "type": "stated"}},
         "uncertainties": []}},

 ("N3-importduty", "haiku"): {
  "rb": {"rules": [{"id": "books", "when": {"product_category": "books"}, "then": "0%", "source_span": "books, which are always duty-free regardless of order value", "provenance": "passage"},
                   {"id": "high", "when": {"order_value": ">=800", "product_category": "*"}, "then": "5%", "source_span": "Orders valued at or above $800 incur a 5% import duty", "provenance": "passage"},
                   {"id": "low", "when": {"order_value": "<800"}, "then": "0%", "source_span": "Orders valued under $800 are exempt from import duty", "provenance": "passage"}],
         "default": {"then": "undetermined", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"order_value": {"value": 950, "span": "$950 worth of books", "type": "stated"},
                   "product_category": {"value": "books", "span": "order for $950 worth of books", "type": "stated"}},
         "uncertainties": []}},
 ("N3-importduty", "opus"): {
  "rb": {"rules": [{"id": "books", "when": {"category": "books"}, "then": "0%", "source_span": "except for books, which are always duty-free regardless of order value", "provenance": "passage"},
                   {"id": "under", "when": {"order_value": "<800"}, "then": "0%", "source_span": "Orders valued under $800 are exempt from import duty.", "provenance": "passage"},
                   {"id": "over", "when": {"order_value": ">=800"}, "then": "5%", "source_span": "Orders valued at or above $800 incur a 5% import duty", "provenance": "passage"}],
         "default": {"then": "undetermined", "source_span": "", "provenance": "passage"}},
  "ir": {"slots": {"order_value": {"value": 950, "span": "an order for $950 worth of books", "type": "stated"},
                   "category": {"value": "books", "span": "$950 worth of books", "type": "stated"}},
         "uncertainties": []}},
}


def gold_match(item, res):
    gv = item["gold_verdict"]
    if gv == "INDETERMINATE":
        return res["verdict"] == "INDETERMINATE"
    # DETERMINATE: verdict determinate AND answer contains gold substring
    if not res["verdict"].startswith("DETERMINATE"):
        return False
    sub = item.get("gold_answer_substring", "")
    return sub in str(res["answer"])


print(f"{'item':16} {'model':6} {'verdict':22} {'answer':10} {'gold_ok':7} {'bytes_ok':8} blocks/halluc")
print("-" * 95)
for (iid, model), data in RUNS.items():
    item = ITEMS[iid]
    res = engine.adjudicate(data["ir"], data["rb"], item["input_text"])
    ok = gold_match(item, res)
    note = ""
    if res["verdict"] == "INDETERMINATE":
        note = "blocks=" + ",".join(res["missing_slots_that_block"])
    if res["hallucinated_slots"]:
        note += " HALLUC=" + ",".join(res["hallucinated_slots"])
    print(f"{iid:16} {model:6} {res['verdict']:22} {str(res['answer'])[:10]:10} "
          f"{('OK' if ok else 'X'):7} {('OK' if res['byte_accounting_ok'] else 'X'):8} {note}")
