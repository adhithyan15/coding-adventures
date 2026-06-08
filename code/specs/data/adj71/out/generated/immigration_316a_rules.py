"""Generated ADJ71 rule library for ordinary 8 U.S.C. 1427(a) naturalization.

This module is generated from byte-cited source spans. It is intentionally
deterministic: no model calls, no network calls, and no wall-clock state.
"""

RULE_PROVENANCE = {
"constitutional_attachment": {
        "end": 1421,
        "id": "constitutional_attachment",
        "length": 67,
        "quote": "attached to the principles of the Constitution of the United States",
        "quote_sha256": "71f118b37d4bf4585203693de1552ccff4eb89f2aa692f4eb8dbc022801d7c5f",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 1354,
        "used_for": "requires attachment to constitutional principles"
},
"continuous_until_admission": {
        "end": 1234,
        "id": "continuous_until_admission",
        "length": 125,
        "quote": "has resided continuously within the United States from the date of the application up to the time of admission to citizenship",
        "quote_sha256": "cb77a3a78a34a49501161d07ce69088dc6d1d7adafc6a177759f91156420dc39",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 1109,
        "used_for": "requires continuous residence from filing through admission to citizenship"
},
"good_moral_character": {
        "end": 1352,
        "id": "good_moral_character",
        "length": 54,
        "quote": "has been and still is a person of good moral character",
        "quote_sha256": "0b2ef6aa1c8531130ee756551f8ac31f7a5e5bb80e1a7cc13693da24a79db503",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 1298,
        "used_for": "requires good moral character during the relevant periods"
},
"lpr_continuous_residence_5y": {
        "end": 768,
        "id": "lpr_continuous_residence_5y",
        "length": 205,
        "quote": "immediately preceding the date of filing his application for naturalization has resided continuously, after being lawfully admitted for permanent residence, within the United States for at least five years",
        "quote_sha256": "b15784a9d540c352b6aad52b3adedf48d8c6fcfbcbdf971982157a6688a209a2",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 563,
        "used_for": "requires LPR status and at least five years continuous residence before filing"
},
"physical_presence_half": {
        "end": 935,
        "id": "physical_presence_half",
        "length": 162,
        "quote": "during the five years immediately preceding the date of filing his application has been physically present therein for periods totaling at least half of that time",
        "quote_sha256": "e758d6dac33467f934a9834e329725036e3e0f363c4ca7c90518c6f1ad83ce5e",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 773,
        "used_for": "requires physical presence for at least half of the five-year period"
},
"scope_ordinary_path": {
        "end": 557,
        "id": "scope_ordinary_path",
        "length": 102,
        "quote": "No person, except as otherwise provided in this subchapter, shall be naturalized unless such applicant",
        "quote_sha256": "6829e27f09dedbdbfb2adc278f990ae98d42226dba6106242b5fcf326b380a76",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 455,
        "used_for": "ordinary naturalization rule applies only when no exception path is claimed"
},
"state_or_district_3m": {
        "end": 1103,
        "id": "state_or_district_3m",
        "length": 158,
        "quote": "has resided within the State or within the district of the Service in the United States in which the applicant filed the application for at least three months",
        "quote_sha256": "d845f4bf3812a8a5724c1d17d1dd7c939c8217ff0157055371756c7bc96a896c",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 945,
        "used_for": "requires at least three months residence in filing state or USCIS district"
},
"well_disposed": {
        "end": 1493,
        "id": "well_disposed",
        "length": 66,
        "quote": "well disposed to the good order and happiness of the United States",
        "quote_sha256": "4cff612580c1bfc09ba0de471c3a491d3c737de6147ad47aa24657e54f8b0f99",
        "source_hash": "ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41",
        "start": 1427,
        "used_for": "requires disposition to the good order and happiness of the United States"
}
}

REQUIREMENTS = [
    {
        "id": "ordinary_path_scope",
        "source_span_id": "scope_ordinary_path",
        "description": "no statutory exception path is claimed",
        "field": "no_exception_claimed",
        "expected": True,
    },
    {
        "id": "lpr_status",
        "source_span_id": "lpr_continuous_residence_5y",
        "description": "applicant has been lawfully admitted for permanent residence",
        "field": "lpr",
        "expected": True,
    },
    {
        "id": "continuous_residence_5y",
        "source_span_id": "lpr_continuous_residence_5y",
        "description": "continuous U.S. residence after LPR admission for at least five years before filing",
        "field": "continuous_residence_years",
        "minimum": 5.0,
    },
    {
        "id": "physical_presence_half_of_5y",
        "source_span_id": "physical_presence_half",
        "description": "physical presence for at least half of five years",
        "field": "physical_presence_months",
        "minimum": 30.0,
        "derived_arithmetic": "5 years * 12 months/year / 2 = 30 months",
    },
    {
        "id": "state_or_district_3m",
        "source_span_id": "state_or_district_3m",
        "description": "residence in filing state or USCIS district for at least three months",
        "field": "state_district_residence_months",
        "minimum": 3.0,
    },
    {
        "id": "continuous_until_admission",
        "source_span_id": "continuous_until_admission",
        "description": "continuous U.S. residence from application to admission to citizenship",
        "field": "continuous_until_admission",
        "expected": True,
    },
    {
        "id": "good_moral_character",
        "source_span_id": "good_moral_character",
        "description": "good moral character",
        "field": "good_moral_character",
        "expected": True,
    },
    {
        "id": "constitutional_attachment",
        "source_span_id": "constitutional_attachment",
        "description": "attachment to constitutional principles",
        "field": "attached_constitution",
        "expected": True,
    },
    {
        "id": "well_disposed",
        "source_span_id": "well_disposed",
        "description": "well disposed to the good order and happiness of the United States",
        "field": "well_disposed",
        "expected": True,
    },
]


def _evaluate_requirement(requirement, facts, fact_refs):
    field = requirement["field"]
    actual = facts[field]
    if "minimum" in requirement:
        passed = actual >= requirement["minimum"]
        expected = {">=": requirement["minimum"]}
    else:
        passed = actual == requirement["expected"]
        expected = requirement["expected"]
    return {
        "requirement_id": requirement["id"],
        "passed": passed,
        "description": requirement["description"],
        "field": field,
        "actual": actual,
        "expected": expected,
        "source_span": RULE_PROVENANCE[requirement["source_span_id"]],
        "fact_span": fact_refs[field],
        "derived_arithmetic": requirement.get("derived_arithmetic"),
    }


def evaluate_case(case):
    facts = case["facts"]
    fact_refs = case["fact_refs"]
    requirements = [
        _evaluate_requirement(requirement, facts, fact_refs)
        for requirement in REQUIREMENTS
    ]
    eligible = all(requirement["passed"] for requirement in requirements)
    return {
        "case_id": case["case_id"],
        "applicant": case["applicant"],
        "query": case["query"],
        "eligible": eligible,
        "failed_requirements": [
            requirement["requirement_id"]
            for requirement in requirements
            if not requirement["passed"]
        ],
        "proof_trace": requirements,
        "engine": "generated_python_rule_library",
        "answer_time_model_calls": 0,
    }

