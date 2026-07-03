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

# --- batch 2: completeness over nominal strength ------------------------------------------------
# A 3-finding GOLD syndrome (a classic clinical triad) competes against a RIVAL whose 2-finding
# syndrome is only PARTIALLY present — one of its two findings is observed, so the rival rule never
# fires and contributes nothing, leaving the rival at its prior. Crucially the rival's syndrome is
# nominally STRONGER (L_rival > L_gold): a reader who assumes the half-seen rival pattern is "close
# enough" picks it, but pattern completeness — not nominal strength — decides. The engine fires only
# the fully-satisfied gold triad. Three prior-only diseases fill the five options.
# (gold_syndrome, gold, (f1, f2, f3), rival_syndrome, rival, (g1, g2_unobserved), [2 prior-only],
#  triad_phrase, rival_phrase, partial_phrase)
THREE_FINDING_SCENARIOS = [
    ("charcot_triad", "ascending_cholangitis",
     ("fever", "jaundice", "right_upper_quadrant_pain"),
     "cholecystitis_pattern", "acute_cholecystitis", ("murphy_sign", "gallstones_on_ultrasound"),
     ["viral_hepatitis", "acute_pancreatitis", "liver_abscess"],
     "fever, jaundice, and right-upper-quadrant pain", "a positive Murphy sign with gallstones",
     "a positive Murphy sign"),
    ("beck_triad", "cardiac_tamponade",
     ("hypotension", "jugular_venous_distension", "muffled_heart_sounds"),
     "tension_pneumothorax_pattern", "tension_pneumothorax",
     ("tracheal_deviation", "absent_breath_sounds"),
     ["massive_pulmonary_embolism", "cardiogenic_shock", "constrictive_pericarditis"],
     "hypotension, jugular venous distension, and muffled heart sounds",
     "tracheal deviation with absent breath sounds", "tracheal deviation"),
    ("hus_triad", "hemolytic_uremic_syndrome",
     ("hemolytic_anemia", "thrombocytopenia", "acute_kidney_injury"),
     "ttp_pattern", "thrombotic_thrombocytopenic_purpura", ("fever", "fluctuating_neurologic_signs"),
     ["disseminated_intravascular_coagulation", "immune_thrombocytopenia", "evans_syndrome"],
     "hemolytic anemia, thrombocytopenia, and acute kidney injury",
     "fever with fluctuating neurologic signs", "fever"),
    ("cushing_reflex", "raised_intracranial_pressure",
     ("hypertension", "bradycardia", "irregular_respiration"),
     "brainstem_stroke_pattern", "brainstem_stroke", ("crossed_sensory_loss", "vertigo"),
     ["hypertensive_emergency", "bacterial_meningitis", "subarachnoid_hemorrhage"],
     "hypertension, bradycardia, and an irregular respiratory pattern",
     "crossed sensory loss with vertigo", "vertigo"),
    ("reactive_arthritis_triad", "reactive_arthritis",
     ("conjunctivitis", "urethritis", "asymmetric_oligoarthritis"),
     "psoriatic_pattern", "psoriatic_arthritis", ("nail_pitting", "dactylitis"),
     ["ankylosing_spondylitis", "gouty_arthritis", "septic_arthritis"],
     "conjunctivitis, urethritis, and an asymmetric oligoarthritis", "nail pitting with dactylitis",
     "nail pitting"),
    ("wernicke_triad", "wernicke_encephalopathy",
     ("ophthalmoplegia", "gait_ataxia", "confusion"),
     "cerebellar_stroke_pattern", "cerebellar_stroke", ("limb_dysmetria", "nystagmus"),
     ["normal_pressure_hydrocephalus", "vestibular_neuritis", "multiple_sclerosis"],
     "ophthalmoplegia, gait ataxia, and confusion", "limb dysmetria with nystagmus",
     "nystagmus"),
]
# (L_gold, L_rival) with L_rival > L_gold > 1: the rival's FULL syndrome is nominally stronger, but
# it is incomplete so it never fires; the gold triad (which IS complete) wins regardless.
VARIANTS3 = [(10, 15), (12, 18)]

# --- batch 3: combined likelihood wins a true tie-break -----------------------------------------
# Two competing syndromes BOTH fully fire (all their findings observed), so this is a real
# tie-break among satisfied patterns — unlike batch-2, where the loser never fired. The GOLD
# disease is supported by a 2-finding syndrome (LR `l_gold`) AND a second, independent corroborating
# finding (LR `l_extra`); the engine MULTIPLIES the two, so its combined likelihood is
# `l_gold * l_extra`. The RIVAL disease is supported by a single, nominally STRONGER syndrome (LR
# `l_rival`), with `l_rival > l_gold` and `l_rival > l_extra` but `l_gold * l_extra > l_rival`. Both
# rules fire; the engine ranks by the product of every fired likelihood, so the gold disease — whose
# two independent pieces of evidence COMBINE to outweigh the rival's one louder clue — wins. The
# trap: comparing the single strongest syndrome (the rival's) picks the rival; the diagnosis is the
# one with the greater TOTAL weight of independent evidence.
# (gold_syndrome, gold, (g1, g2), extra_finding, rival_syndrome, rival, (r1, r2), [2 prior-only],
#  gold_syn_phrase, gold_extra_phrase, rival_syn_phrase)
COMBINED_LR_SCENARIOS = [
    ("endocarditis_pattern", "infective_endocarditis", ("new_regurgitant_murmur", "janeway_lesions"),
     "splinter_hemorrhages", "rheumatic_pattern", "rheumatic_fever",
     ("migratory_polyarthritis", "subcutaneous_nodules"),
     ["pericarditis", "atrial_myxoma"],
     "a new regurgitant murmur with Janeway lesions", "splinter hemorrhages",
     "a migratory polyarthritis with subcutaneous nodules"),
    ("lupus_pattern", "systemic_lupus", ("malar_rash", "photosensitivity"), "oral_ulcers",
     "dermatomyositis_pattern", "dermatomyositis", ("gottron_papules", "heliotrope_rash"),
     ["psoriasis", "lichen_planus"],
     "a malar rash with photosensitivity", "oral ulcers",
     "Gottron papules with a heliotrope rash"),
    ("sarcoid_pattern", "sarcoidosis", ("bilateral_hilar_adenopathy", "erythema_nodosum"),
     "elevated_ace_level", "tuberculosis_pattern", "tuberculosis",
     ("night_sweats", "apical_cavitation"),
     ["lymphoma", "histoplasmosis"],
     "bilateral hilar adenopathy with erythema nodosum", "an elevated ACE level",
     "night sweats with apical cavitation"),
    ("hemochromatosis_pattern", "hereditary_hemochromatosis", ("bronze_skin", "restrictive_cardiomyopathy"),
     "elevated_ferritin", "diabetes_pattern", "diabetes_mellitus", ("polyuria", "polydipsia"),
     ["wilson_disease", "porphyria"],
     "bronze skin with a restrictive cardiomyopathy", "an elevated ferritin",
     "polyuria with polydipsia"),
    ("carcinoid_pattern", "carcinoid_syndrome", ("episodic_flushing", "secretory_diarrhea"),
     "right_sided_heart_murmur", "pheochromocytoma_pattern", "pheochromocytoma",
     ("paroxysmal_hypertension", "palpitations"),
     ["vipoma", "mastocytosis"],
     "episodic flushing with secretory diarrhea", "a right-sided heart murmur",
     "paroxysmal hypertension with palpitations"),
    ("myeloma_pattern", "multiple_myeloma", ("lytic_bone_lesions", "hypercalcemia"), "rouleaux_formation",
     "waldenstrom_pattern", "waldenstrom_macroglobulinemia", ("serum_hyperviscosity", "lymphadenopathy"),
     ["monoclonal_gammopathy", "amyloidosis"],
     "lytic bone lesions with hypercalcemia", "rouleaux formation on the smear",
     "serum hyperviscosity with lymphadenopathy"),
]
# (l_gold, l_extra, l_rival): l_gold*l_extra > l_rival > l_gold and > l_extra, so the rival's single
# syndrome is nominally stronger than either of the gold disease's two pieces, yet their PRODUCT wins.
VARIANTS_COMBINED = [(8, 3, 15), (6, 4, 20)]

# --- batch 4: combined likelihood beats TWO firing rivals (3-way tie-break) ----------------------
# Batch 3 pitted the gold disease's combined evidence against ONE firing rival. Batch 4 raises the
# bar: the same GOLD disease — a 2-finding syndrome (LR `l_gold`) PLUS an independent corroborating
# finding (LR `l_extra`), whose product the engine forms as `l_gold * l_extra` — now competes against
# TWO rivals that BOTH fully fire, each backed by a single 2-finding syndrome (LRs `l_ra`, `l_rb`).
# Each rival's single syndrome is nominally stronger than either of the gold's two pieces
# (`l_ra`,`l_rb` both `> l_gold` and `> l_extra`), yet the gold's PRODUCT beats both
# (`l_gold * l_extra > l_ra` and `> l_rb`). Three syndrome rules fire; `decision_leader` ranks by the
# product of every fired likelihood and returns the gold disease. The trap deepens: there are now two
# louder single clues to anchor on, and the reader must recognise that the gold's two independent
# pieces of evidence COMBINE to outweigh either. Two prior-only diseases fill the five options.
# (gold_syn, gold, (g1, g2), extra, ra_syn, ra, (ra1, ra2), rb_syn, rb, (rb1, rb2), [2 prior-only],
#  gold_syn_phrase, gold_extra_phrase, ra_syn_phrase, rb_syn_phrase)
TWO_RIVAL_SCENARIOS = [
    ("endocarditis_pattern", "infective_endocarditis", ("new_regurgitant_murmur", "janeway_lesions"),
     "splinter_hemorrhages",
     "lupus_pattern", "systemic_lupus", ("malar_rash", "photosensitivity"),
     "rheumatic_pattern", "rheumatic_fever", ("migratory_polyarthritis", "subcutaneous_nodules"),
     ["pericarditis", "atrial_myxoma"],
     "a new regurgitant murmur with Janeway lesions", "splinter hemorrhages",
     "a malar rash with photosensitivity", "a migratory polyarthritis with subcutaneous nodules"),
    ("sarcoid_pattern", "sarcoidosis", ("bilateral_hilar_adenopathy", "erythema_nodosum"),
     "elevated_ace_level",
     "tuberculosis_pattern", "tuberculosis", ("drenching_night_sweats", "apical_cavitation"),
     "lymphoma_pattern", "lymphoma", ("bulky_mediastinal_mass", "generalized_pruritus"),
     ["histoplasmosis", "silicosis"],
     "bilateral hilar adenopathy with erythema nodosum", "an elevated ACE level",
     "drenching night sweats with apical cavitation",
     "a bulky mediastinal mass with generalized pruritus"),
    ("carcinoid_pattern", "carcinoid_syndrome", ("episodic_flushing", "secretory_diarrhea"),
     "right_sided_heart_murmur",
     "pheochromocytoma_pattern", "pheochromocytoma", ("paroxysmal_hypertension", "palpitations"),
     "vipoma_pattern", "vipoma", ("watery_diarrhea", "hypokalemia"),
     ["mastocytosis", "zollinger_ellison"],
     "episodic flushing with secretory diarrhea", "a right-sided heart murmur",
     "paroxysmal hypertension with palpitations", "a watery diarrhea with hypokalemia"),
    ("myeloma_pattern", "multiple_myeloma", ("lytic_bone_lesions", "hypercalcemia"),
     "rouleaux_formation",
     "waldenstrom_pattern", "waldenstrom_macroglobulinemia", ("serum_hyperviscosity", "lymphadenopathy"),
     "amyloid_pattern", "primary_amyloidosis", ("nephrotic_proteinuria", "macroglossia"),
     ["monoclonal_gammopathy", "chronic_lymphocytic_leukemia"],
     "lytic bone lesions with hypercalcemia", "rouleaux formation on the smear",
     "serum hyperviscosity with lymphadenopathy", "nephrotic-range proteinuria with macroglossia"),
    ("hemochromatosis_pattern", "hereditary_hemochromatosis", ("bronze_skin", "restrictive_cardiomyopathy"),
     "elevated_ferritin",
     "wilson_pattern", "wilson_disease", ("kayser_fleischer_rings", "resting_tremor"),
     "diabetes_pattern", "diabetes_mellitus", ("polyuria", "polydipsia"),
     ["porphyria", "addison_disease"],
     "bronze skin with a restrictive cardiomyopathy", "an elevated ferritin",
     "Kayser-Fleischer rings with a resting tremor", "polyuria with polydipsia"),
    ("dermatomyositis_pattern", "dermatomyositis", ("gottron_papules", "heliotrope_rash"),
     "proximal_muscle_weakness",
     "sclerosis_pattern", "systemic_sclerosis", ("sclerodactyly", "raynaud_phenomenon"),
     "psoriatic_pattern", "psoriatic_arthritis", ("nail_pitting", "dactylitis"),
     ["lichen_planus", "rosacea"],
     "Gottron papules with a heliotrope rash", "proximal muscle weakness",
     "sclerodactyly with Raynaud phenomenon", "nail pitting with dactylitis"),
]
# (l_gold, l_extra, l_ra, l_rb): l_gold*l_extra > l_ra and > l_rb, while each rival LR exceeds
# l_gold and l_extra individually. The two rivals differ (distinct posteriors), neither ties gold.
VARIANTS_TWO_RIVAL = [(8, 3, 15, 20), (6, 5, 18, 25)]


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
    # --- batch 2: a complete 3-finding triad beats a nominally-stronger but INCOMPLETE rival -----
    for scen in THREE_FINDING_SCENARIOS:
        (gold_syn, gold, (f1, f2, f3), rival_syn, rival, (g1, g2), priors_only,
         triad_phrase, rival_phrase, partial_phrase) = scen
        for (l_gold, l_rival) in VARIANTS3:
            diseases = [gold, rival, *priors_only]  # five distinct diseases
            assert len(set(diseases)) == 5, diseases
            prior = "0.2"
            # gold's triad is fully observed → fires (×l_gold). The rival's 2-finding syndrome has
            # only g1 observed (g2 absent) → does NOT fire → stays at prior. Others stay at prior.
            posterior = {d: 0.2 for d in diseases}
            posterior[gold] = 0.2 * l_gold
            leader = max(posterior, key=posterior.get)
            assert leader == gold, (gold, posterior)
            assert sum(1 for d in diseases if posterior[d] == posterior[gold]) == 1, scen
            assert l_rival > l_gold > 1, scen  # rival nominally stronger, yet incomplete → loses

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
                + f"contributes {l_gold} from {gold_syn} to {gold}\n"
                + f"contributes {l_rival} from {rival_syn} to {rival}\n"
                + f"rule {{ head: {gold_syn} when: {f1}, {f2}, {f3} }}\n"
                + f"rule {{ head: {rival_syn} when: {g1}, {g2} }}\n"
                + f"observe {f1}\n"
                + f"observe {f2}\n"
                + f"observe {f3}\n"
                + f"observe {g1}\n"  # only ONE of the rival syndrome's two findings → it never fires
                + "".join(f"? {d}\n" for d in diseases)
            )
            stem = (
                f"Five diagnoses are equally likely a priori (each prior {prior}). The complete triad "
                f"of {triad_phrase} raises the likelihood of {gold.replace('_', ' ')} {l_gold}-fold. "
                f"The full syndrome of {rival_phrase} would raise {rival.replace('_', ' ')} "
                f"{l_rival}-fold — a stronger association — but it requires both of its findings. The "
                f"patient has {triad_phrase}, and {partial_phrase} but not the rest of that syndrome. "
                f"Which single diagnosis is most likely?"
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
    # --- batch 3: combined likelihood wins a true tie-break between two FIRING syndromes ---------
    for scen in COMBINED_LR_SCENARIOS:
        (gold_syn, gold, (g1, g2), extra, rival_syn, rival, (r1, r2), priors_only,
         gold_syn_phrase, gold_extra_phrase, rival_syn_phrase) = scen
        for (l_gold, l_extra, l_rival) in VARIANTS_COMBINED:
            diseases = [gold, rival, *priors_only]  # four here; a fifth prior-only is added below
            # Pad to five distinct diseases with a neutral, digit-free distractor per scenario slot.
            filler = ["idiopathic_condition", "reactive_process", "paraneoplastic_syndrome"]
            for f in filler:
                if len(diseases) >= 5:
                    break
                if f not in diseases:
                    diseases.append(f)
            diseases = diseases[:5]
            assert len(set(diseases)) == 5, diseases
            prior = "0.2"
            # gold's two independent contributions MULTIPLY (l_gold * l_extra); the rival fires one
            # nominally-stronger syndrome (l_rival). Both rules fire fully.
            posterior = {d: 0.2 for d in diseases}
            posterior[gold] = 0.2 * l_gold * l_extra
            posterior[rival] = 0.2 * l_rival
            leader = max(posterior, key=posterior.get)
            assert leader == gold, (gold, posterior)
            assert sum(1 for d in diseases if posterior[d] == posterior[gold]) == 1, scen
            assert l_gold * l_extra > l_rival > l_gold and l_rival > l_extra, scen

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
                + f"contributes {l_gold} from {gold_syn} to {gold}\n"
                + f"contributes {l_extra} from {extra} to {gold}\n"
                + f"contributes {l_rival} from {rival_syn} to {rival}\n"
                + f"rule {{ head: {gold_syn} when: {g1}, {g2} }}\n"
                + f"rule {{ head: {rival_syn} when: {r1}, {r2} }}\n"
                + f"observe {g1}\n"
                + f"observe {g2}\n"
                + f"observe {extra}\n"
                + f"observe {r1}\n"
                + f"observe {r2}\n"
                + "".join(f"? {d}\n" for d in diseases)
            )
            stem = (
                f"Five diagnoses are equally likely a priori (each prior {prior}). The syndrome of "
                f"{gold_syn_phrase} raises the likelihood of {gold.replace('_', ' ')} {l_gold}-fold, "
                f"and {gold_extra_phrase} independently raises it a further {l_extra}-fold. The "
                f"syndrome of {rival_syn_phrase} raises {rival.replace('_', ' ')} {l_rival}-fold — a "
                f"stronger single association than either finding of the first. The patient has "
                f"{gold_syn_phrase}, {gold_extra_phrase}, and {rival_syn_phrase}. Which single "
                f"diagnosis is most likely?"
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
    # --- batch 4: combined likelihood beats TWO firing rivals (3-way tie-break) ------------------
    for scen in TWO_RIVAL_SCENARIOS:
        (gold_syn, gold, (g1, g2), extra, ra_syn, ra, (ra1, ra2), rb_syn, rb, (rb1, rb2),
         priors_only, gold_syn_phrase, gold_extra_phrase, ra_syn_phrase, rb_syn_phrase) = scen
        for (l_gold, l_extra, l_ra, l_rb) in VARIANTS_TWO_RIVAL:
            diseases = [gold, ra, rb, *priors_only]  # gold + two rivals + two prior-only = five
            assert len(set(diseases)) == 5, diseases
            prior = "0.2"
            # gold's two independent contributions MULTIPLY (l_gold * l_extra); BOTH rivals fire one
            # nominally-stronger syndrome each (l_ra, l_rb). All three syndrome rules fire fully.
            posterior = {d: 0.2 for d in diseases}
            posterior[gold] = 0.2 * l_gold * l_extra
            posterior[ra] = 0.2 * l_ra
            posterior[rb] = 0.2 * l_rb
            leader = max(posterior, key=posterior.get)
            assert leader == gold, (gold, posterior)
            assert sum(1 for d in diseases if posterior[d] == posterior[gold]) == 1, scen
            # each rival is nominally stronger than either of the gold's two pieces, yet the product wins
            assert l_gold * l_extra > l_ra and l_gold * l_extra > l_rb, scen
            assert l_ra > l_gold and l_ra > l_extra, scen
            assert l_rb > l_gold and l_rb > l_extra, scen

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
                + f"contributes {l_gold} from {gold_syn} to {gold}\n"
                + f"contributes {l_extra} from {extra} to {gold}\n"
                + f"contributes {l_ra} from {ra_syn} to {ra}\n"
                + f"contributes {l_rb} from {rb_syn} to {rb}\n"
                + f"rule {{ head: {gold_syn} when: {g1}, {g2} }}\n"
                + f"rule {{ head: {ra_syn} when: {ra1}, {ra2} }}\n"
                + f"rule {{ head: {rb_syn} when: {rb1}, {rb2} }}\n"
                + f"observe {g1}\n"
                + f"observe {g2}\n"
                + f"observe {extra}\n"
                + f"observe {ra1}\n"
                + f"observe {ra2}\n"
                + f"observe {rb1}\n"
                + f"observe {rb2}\n"
                + "".join(f"? {d}\n" for d in diseases)
            )
            stem = (
                f"Five diagnoses are equally likely a priori (each prior {prior}). The syndrome of "
                f"{gold_syn_phrase} raises the likelihood of {gold.replace('_', ' ')} {l_gold}-fold, "
                f"and {gold_extra_phrase} independently raises it a further {l_extra}-fold. Two rival "
                f"syndromes are each a stronger single association: {ra_syn_phrase} raises "
                f"{ra.replace('_', ' ')} {l_ra}-fold, and {rb_syn_phrase} raises "
                f"{rb.replace('_', ' ')} {l_rb}-fold. The patient has {gold_syn_phrase}, "
                f"{gold_extra_phrase}, {ra_syn_phrase}, and {rb_syn_phrase}. Which single diagnosis is "
                f"most likely?"
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
            "unique leader is asserted at build. Batch 2 adds the completeness-over-strength variant: "
            "a complete 3-finding triad competes against a rival whose 2-finding syndrome is only "
            "partially present (one finding observed) and is nominally STRONGER — yet the rival rule "
            "never fires, so the fully-satisfied triad wins. Pattern completeness, not nominal "
            "association strength, decides. Batch 3 adds the combined-likelihood tie-break: two "
            "syndromes both fully fire, but the gold disease carries a 2-finding syndrome PLUS an "
            "independent corroborating finding whose product (the engine multiplies contributions to "
            "one hypothesis) beats the rival's single nominally-stronger syndrome. Batch 4 raises it "
            "to a 3-way tie-break: the same gold product must outweigh TWO rivals that both fully fire, "
            "each backed by a single syndrome nominally stronger than either of the gold's two pieces."
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
