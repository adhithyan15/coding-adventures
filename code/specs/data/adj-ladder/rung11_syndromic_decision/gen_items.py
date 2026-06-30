"""Generate rung-11 (syndromic decision) items.json for the ADJ-LADDER.

Rung 11 is the first DECISION rung that is also genuinely MULTI-HOP: it composes the
rule-derived compound finding of rung-3 with the competing-differential ranking of rung-6, so
the engine must (1) DERIVE which clinical *syndrome* the patient's findings satisfy, then
(2) RANK the candidate diseases and pick the leader. That is two reasoning hops — recognise the
pattern, then decide — rather than a single arithmetic evaluation (the quantitative band, rungs
7–10) or a single-pattern decision (rung-3).

The shape of one item:

  prior P for d1 … d5                              # equal priors — the decision is evidence-driven
  contributes L_syn  from syndrome_1 to d1         # the WHOLE syndrome argues for the gold disease
  contributes L_flash from flashy to d_anchor      # a single flashy finding argues for a RIVAL
  rule { head: syndrome_1 when: f_a, f_b }         # the syndrome fires ONLY if BOTH findings hold
  observe f_a                                       # both syndrome findings are present …
  observe f_b
  observe flashy                                    # … and so is the rival's one flashy finding
  ? d1 … ? d5

The engine fires `syndrome_1` only because BOTH `f_a` and `f_b` are observed (a missing half
would leave it unfired and contributing nothing — the rung-3 mechanism), multiplies the gold
disease's prior by `L_syn`, multiplies the rival's prior by the single `L_flash`, and
`decision_leader` returns the unique maximum. With `L_syn > L_flash > 1` the fully-satisfied
syndrome wins — but a reader who ANCHORS on the lone flashy finding (a higher single-finding
likelihood than any one of the syndrome's parts) picks the rival. That anchoring trap is the
point: the diagnosis is the one supported by the COMPLETE pattern, not the loudest single clue.

No engine or harness change: this reuses `rule { head … when … }` (rung-3) and the
`decision_leader` extractor (rung-6), and the ladder's synthetic priors/LRs (the ADJ-LADDER is a
capability benchmark, not the grounded CAS, so authored numbers are fine — only the no-result-
literal contamination gate applies: every prior and LR in the program appears verbatim in the
stem, and the answer is a disease NAME, never a number).

Contamination-safe: the only literals in the program are the priors and LRs, all of which the
stem prints; the gold answer is an option label (a disease name), so nothing numeric leaks. Gold
letter rotates A–E by index; every item's unique leader is asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# Each scenario: a fully-satisfied SYNDROME (two findings → gold disease) competing against a
# RIVAL supported by one flashy finding, plus three prior-only diseases. All five disease names
# are distinct (the MCQ options). Fields:
#   (syndrome_name, gold_disease, (finding_a, finding_b), rival_disease, flashy_finding,
#    [three prior-only diseases], human phrasing of the two syndrome findings + the flashy one)
SCENARIOS = [
    ("dermatomyositis_pattern", "dermatomyositis",
     ("gottron_papules", "heliotrope_rash"), "systemic_lupus", "malar_rash",
     ["psoriasis", "rosacea", "contact_dermatitis"],
     "Gottron papules and a heliotrope rash", "a malar rash"),
    ("infective_endocarditis_pattern", "infective_endocarditis",
     ("janeway_lesions", "roth_spots"), "rheumatic_fever", "migratory_polyarthritis",
     ["pericarditis", "myocarditis", "atrial_myxoma"],
     "Janeway lesions and Roth spots", "a migratory polyarthritis"),
    ("hemochromatosis_pattern", "hereditary_hemochromatosis",
     ("bronze_skin", "restrictive_cardiomyopathy"), "diabetes_mellitus", "polyuria",
     ["wilson_disease", "addison_disease", "porphyria"],
     "bronze skin and a restrictive cardiomyopathy", "polyuria"),
    ("tuberous_sclerosis_pattern", "tuberous_sclerosis",
     ("ash_leaf_macules", "facial_angiofibromas"), "neurofibromatosis", "cafe_au_lait_spots",
     ["sturge_weber", "von_hippel_lindau", "ataxia_telangiectasia"],
     "ash-leaf macules and facial angiofibromas", "café-au-lait spots"),
    ("serotonin_syndrome_pattern", "serotonin_syndrome",
     ("clonus", "hyperreflexia"), "neuroleptic_malignant_syndrome", "lead_pipe_rigidity",
     ["malignant_hyperthermia", "anticholinergic_toxicity", "thyroid_storm"],
     "clonus and hyperreflexia", "lead-pipe rigidity"),
    ("kawasaki_pattern", "kawasaki_disease",
     ("strawberry_tongue", "cervical_lymphadenopathy"), "scarlet_fever", "sandpaper_rash",
     ["measles", "stevens_johnson", "toxic_shock_syndrome"],
     "a strawberry tongue and cervical lymphadenopathy", "a sandpaper rash"),
    ("addison_pattern", "addison_disease",
     ("skin_hyperpigmentation", "hyponatremia_hyperkalemia"), "hypothyroidism", "cold_intolerance",
     ["cushing_syndrome", "conn_syndrome", "pheochromocytoma"],
     "skin hyperpigmentation with hyponatremia and hyperkalemia", "cold intolerance"),
    ("hyperthyroid_pattern", "graves_disease",
     ("exophthalmos", "pretibial_myxedema"), "toxic_multinodular_goiter", "heat_intolerance",
     ["hashimoto_thyroiditis", "thyroid_adenoma", "subacute_thyroiditis"],
     "exophthalmos and pretibial myxedema", "heat intolerance"),
    ("wernicke_pattern", "wernicke_encephalopathy",
     ("ophthalmoplegia", "gait_ataxia"), "cerebellar_stroke", "dysmetria",
     ["normal_pressure_hydrocephalus", "vestibular_neuritis", "cobalamin_deficiency"],
     "ophthalmoplegia and gait ataxia", "dysmetria"),
    ("hsp_pattern", "henoch_schonlein_purpura",
     ("palpable_purpura", "abdominal_pain_with_arthralgia"), "immune_thrombocytopenia", "petechiae",
     ["hemolytic_uremic_syndrome", "meningococcemia", "polyarteritis_nodosa"],
     "palpable purpura with abdominal pain and arthralgia", "petechiae"),
    ("cushing_pattern", "cushing_syndrome",
     ("moon_facies", "purple_abdominal_striae"), "metabolic_syndrome", "central_obesity",
     ["acromegaly", "hypothyroidism", "polycystic_ovary_syndrome"],
     "moon facies and purple abdominal striae", "central obesity"),
    ("carcinoid_pattern", "carcinoid_syndrome",
     ("episodic_flushing", "secretory_diarrhea"), "vipoma", "watery_diarrhea",
     ["zollinger_ellison", "mastocytosis", "pheochromocytoma"],
     "episodic flushing and secretory diarrhea", "watery diarrhea"),
]

# Two LR variants per scenario (gold syndrome strictly beats the flashy rival in both) → 24 items.
# (L_syn, L_flash) with L_syn > L_flash > 1.
VARIANTS = [(12, 6), (15, 8)]


def build():
    items = []
    idx = 0
    for scen in SCENARIOS:
        (syndrome, gold, (fa, fb), rival, flashy, priors_only, syn_phrase, flashy_phrase) = scen
        for (l_syn, l_flash) in VARIANTS:
            diseases = [gold, rival, *priors_only]  # five distinct diseases
            assert len(set(diseases)) == 5, diseases
            prior = "0.2"
            # Unnormalised posteriors: gold = 0.2*l_syn, rival = 0.2*l_flash, others = 0.2.
            posterior = {d: 0.2 for d in diseases}
            posterior[gold] = 0.2 * l_syn
            posterior[rival] = 0.2 * l_flash
            leader = max(posterior, key=posterior.get)
            assert leader == gold, (gold, posterior)
            assert sum(1 for d in diseases if posterior[d] == posterior[gold]) == 1, scen

            gold_pos = idx % 5
            opts = [d for d in diseases if d != gold]
            opts.insert(gold_pos, gold)
            opts = opts[:5]
            if opts[gold_pos] != gold:
                opts[gold_pos] = gold
            assert len(set(opts)) == 5, opts
            options = {LETTERS[i]: opts[i] for i in range(5)}

            prog = (
                "".join(f"prior {prior} for {d}\n" for d in diseases)
                + f"contributes {l_syn} from {syndrome} to {gold}\n"
                + f"contributes {l_flash} from {flashy} to {rival}\n"
                + f"rule {{ head: {syndrome} when: {fa}, {fb} }}\n"
                + f"observe {fa}\n"
                + f"observe {fb}\n"
                + f"observe {flashy}\n"
                + "".join(f"? {d}\n" for d in diseases)
            )
            stem = (
                f"Five diagnoses are equally likely a priori (each prior {prior}). The complete "
                f"syndrome of {syn_phrase} raises the likelihood of {gold.replace('_', ' ')} "
                f"{l_syn}-fold, while {flashy_phrase} alone raises the likelihood of "
                f"{rival.replace('_', ' ')} {l_flash}-fold. The patient has {syn_phrase}, and also "
                f"{flashy_phrase}. Which single diagnosis is most likely?"
            )
            items.append({
                "id": f"r11sd-{idx + 1:02d}",
                "qtype": "syndromic_decision",
                "stem": stem,
                "program": prog,
                "answer_from": {"type": "decision_leader", "structural_weights": False},
                "options": options,
                "gold_letter": LETTERS[gold_pos],
            })
            idx += 1
    return {
        "description": (
            "ADJ-LADDER rung 11 — syndromic decision: the first rung that is BOTH a decision and "
            "genuinely multi-hop. Each item gives five equally-likely diagnoses; the engine must "
            "first DERIVE which clinical syndrome the patient satisfies (a `rule` head that fires "
            "only when BOTH of its findings are observed — the rung-3 mechanism), then RANK the "
            "diagnoses by posterior and pick the leader (the rung-6 decision_leader extractor). The "
            "fully-satisfied syndrome (likelihood L_syn) beats a rival supported by a single flashy "
            "finding (L_flash < L_syn) — the anchoring trap: the diagnosis is the one backed by the "
            "COMPLETE pattern, not the loudest lone clue. No engine/harness change (reuses rule + "
            "decision_leader); synthetic priors/LRs (the ladder is a capability benchmark, not the "
            "grounded CAS). Contamination-safe: every prior and LR is printed in the stem and the "
            "answer is a disease name, so no result literal leaks; gold rotates A–E; every item's "
            "unique leader is asserted at build."
        ),
        "items": items,
    }


if __name__ == "__main__":
    doc = build()
    with open("items.json", "w") as f:
        json.dump(doc, f, indent=2)
        f.write("\n")
    print("wrote items.json:", len(doc["items"]), "items")
    for it in doc["items"]:
        print(it["id"], it["qtype"], "gold", it["gold_letter"], "=",
              it["options"][it["gold_letter"]])
