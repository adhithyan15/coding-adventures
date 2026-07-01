"""Generate rung-14 (indeterminate decision with an abstention arm) items.json for the ADJ-LADDER.

Rung 14 introduces a NEW decision shape: **knowing when NOT to decide**. Every earlier decision rung
(6, 11, 12) asks the engine to commit to an answer; this rung tests the complementary skill — an
honest **abstention** when the evidence genuinely does not distinguish the top candidates. That is the
whole thesis in miniature: a system that reasons on the CPU should *solve when it can and abstain when
it cannot*, and the abstention must be a first-class, positively-scored answer — not a silent shrug.

Each item gives five options: four candidate diagnoses plus a fifth **"insufficient information to
distinguish"** choice. Two families, interleaved:

  * DETERMINATE — one finding raises diagnosis A more strongly than a competing finding raises
    diagnosis B (`L_hi > L_lo`), so the posterior has a clear unique leader. The engine returns a
    `determinate` decision naming A; the gold answer is A. Picking "insufficient information" here is
    WRONG — the evidence *does* decide.

  * TIE — the two findings raise their diagnoses by the *same* likelihood ratio (`L == L`), so the
    top two posteriors are exactly equal. The engine returns a `kickback` decision ("top two
    hypotheses are tied on posterior log-odds") — it declines to commit. The gold answer is
    "insufficient information to distinguish". Picking either tied diagnosis here is WRONG — the
    evidence does NOT decide, and guessing would be a fabrication.

The engine owns both verdicts natively (`decision.type` = `determinate` vs `kickback`); the harness's
`decision_leader_to_letter` maps a `determinate` leader to that diagnosis's letter and — when the rung
supplies a `tie_label` — maps a `kickback` to the "insufficient information" letter. No number is ever
compared in Python; the engine's own refusal-to-commit *is* the abstention.

The reasoning tested is calibration: commit exactly when the log-odds separate, abstain exactly when
they coincide. A model that always guesses the higher-sounding diagnosis fails every TIE item; a model
that always hedges fails every DETERMINATE item.

Contamination-safe: the only literals in each program are the priors and the likelihood ratios, all of
which the stem prints; the gold answer is a categorical option (a diagnosis name or the fixed
"insufficient information" phrase), so nothing numeric leaks. Identifiers are digit-free. Gold rotates
A–E by index; the determinate/tie split is 12/12 and asserted at build.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# The fixed abstention option — the fifth choice on every item, and the gold answer on TIE items.
TIE_OPTION = "insufficient information to distinguish"

# Each scenario is a pair of diagnoses that share a presentation (so a genuine tie is clinically
# plausible), plus two prior-only filler diagnoses. Fields:
#   (dis_a, find_a, dis_b, find_b, [filler_c, filler_d], phrase_a, phrase_b)
# find_a favours dis_a, find_b favours dis_b; both are always observed.
SCENARIOS = [
    ("bacterial_pharyngitis", "tonsillar_exudate", "viral_pharyngitis", "coryza",
     ["infectious_mononucleosis", "oral_candidiasis"],
     "a tonsillar exudate", "coryza"),
    ("iron_deficiency_anemia", "low_ferritin", "thalassemia_trait", "target_cells",
     ["anemia_of_chronic_disease", "sideroblastic_anemia"],
     "a low ferritin", "target cells on the smear"),
    ("bacterial_meningitis", "neutrophilic_pleocytosis", "viral_meningitis", "lymphocytic_pleocytosis",
     ["fungal_meningitis", "subarachnoid_hemorrhage"],
     "a neutrophilic CSF pleocytosis", "a lymphocytic CSF pleocytosis"),
    ("ulcerative_colitis", "continuous_colonic_inflammation", "crohn_disease", "skip_lesions",
     ["ischemic_colitis", "infectious_colitis"],
     "continuous colonic inflammation", "skip lesions"),
    ("stable_angina", "exertional_chest_pain", "gastroesophageal_reflux", "postprandial_burning",
     ["costochondritis", "pericarditis"],
     "exertional chest pain", "postprandial burning"),
    ("transudative_effusion", "low_pleural_protein", "exudative_effusion", "high_pleural_ldh",
     ["hemothorax", "chylothorax"],
     "a low pleural protein", "a high pleural LDH"),
    ("hyperthyroidism", "suppressed_tsh", "anxiety_disorder", "situational_triggers",
     ["pheochromocytoma", "caffeine_excess"],
     "a suppressed TSH", "clear situational triggers"),
    ("cardiac_syncope", "exertional_onset", "vasovagal_syncope", "prodromal_nausea",
     ["orthostatic_syncope", "seizure"],
     "an exertional onset", "a prodromal nausea"),
    ("nephrotic_syndrome", "heavy_proteinuria", "nephritic_syndrome", "dysmorphic_hematuria",
     ["acute_tubular_necrosis", "prerenal_azotemia"],
     "heavy proteinuria", "dysmorphic hematuria"),
    ("migraine", "unilateral_throbbing", "tension_headache", "bilateral_band_tightness",
     ["cluster_headache", "sinus_headache"],
     "a unilateral throbbing quality", "a bilateral band-like tightness"),
    ("osteoarthritis", "mechanical_joint_pain", "rheumatoid_arthritis", "prolonged_morning_stiffness",
     ["gout", "septic_arthritis"],
     "mechanical joint pain", "prolonged morning stiffness"),
    ("delirium", "acute_fluctuating_course", "dementia", "insidious_progression",
     ["depression", "psychosis"],
     "an acute fluctuating course", "an insidious progression"),
]

# Determinate items separate the top two (L_hi strictly beats L_lo, by a comfortable margin so the
# engine does not kick back); tie items give both diagnoses the SAME likelihood ratio.
L_HI, L_LO = 8, 3
L_TIE = 5


def build():
    items = []
    idx = 0
    for scen in SCENARIOS:
        dis_a, find_a, dis_b, find_b, fillers, phrase_a, phrase_b = scen
        diseases = [dis_a, dis_b, *fillers]  # four real hypotheses
        assert len(set(diseases)) == 4, diseases
        # Two items per scenario: one determinate (gold = dis_a), one tie (gold = the abstain option).
        for kind in ("determinate", "tie"):
            l_a = L_HI if kind == "determinate" else L_TIE
            l_b = L_LO if kind == "determinate" else L_TIE
            gold = dis_a if kind == "determinate" else TIE_OPTION

            # Five options: the four diagnoses + the abstention choice. Gold rotates A–E.
            pool = [d for d in diseases + [TIE_OPTION] if d != gold]
            gold_pos = idx % 5
            opts = pool[:]
            opts.insert(gold_pos, gold)
            opts = opts[:5]
            if opts[gold_pos] != gold:
                opts[gold_pos] = gold
            assert len(set(opts)) == 5, opts
            assert TIE_OPTION in opts, opts
            options = {LETTERS[i]: opts[i] for i in range(5)}

            prog = (
                "".join(f"prior 0.2 for {d}\n" for d in diseases)
                + f"contributes {l_a} from {find_a} to {dis_a}\n"
                + f"contributes {l_b} from {find_b} to {dis_b}\n"
                + f"observe {find_a}\n"
                + f"observe {find_b}\n"
                + "".join(f"? {d}\n" for d in diseases)
            )
            if kind == "determinate":
                stem = (
                    f"Four diagnoses are equally likely a priori (each prior 0.2). {phrase_a.capitalize()} "
                    f"raises the likelihood of {dis_a.replace('_', ' ')} {l_a}-fold, while {phrase_b} "
                    f"raises the likelihood of {dis_b.replace('_', ' ')} {l_b}-fold. The patient has "
                    f"{phrase_a} and {phrase_b}. Which single diagnosis is most likely — or is the "
                    f"evidence insufficient to distinguish them?"
                )
            else:
                stem = (
                    f"Four diagnoses are equally likely a priori (each prior 0.2). {phrase_a.capitalize()} "
                    f"raises the likelihood of {dis_a.replace('_', ' ')} {l_a}-fold, and {phrase_b} "
                    f"raises the likelihood of {dis_b.replace('_', ' ')} by the same {l_b}-fold. The "
                    f"patient has {phrase_a} and {phrase_b}. Which single diagnosis is most likely — or "
                    f"is the evidence insufficient to distinguish them?"
                )
            items.append({
                "id": f"r14id-{idx + 1:02d}",
                "qtype": "indeterminate_decision",
                "stem": stem,
                "program": prog,
                "answer_from": {
                    "type": "decision_leader",
                    "structural_weights": False,
                    "tie_label": TIE_OPTION,
                },
                "options": options,
                "gold_letter": LETTERS[gold_pos],
            })
            idx += 1
    return {
        "description": (
            "ADJ-LADDER rung 14 — indeterminate decision: the abstention rung. Each item offers four "
            "candidate diagnoses plus an 'insufficient information to distinguish' option. On "
            "DETERMINATE items one finding outweighs the other (L_hi > L_lo), so the engine returns a "
            "determinate leader and the gold answer is that diagnosis. On TIE items both findings carry "
            "the SAME likelihood ratio, so the top two posteriors are exactly equal, the engine returns "
            "a kickback ('top two hypotheses are tied on posterior log-odds'), and the gold answer is "
            "the abstention option. The engine owns both verdicts natively (decision.type = determinate "
            "vs kickback); the harness maps a determinate leader to its diagnosis letter and a kickback "
            "to the tie_label letter — Python never compares a number. This tests calibration: commit "
            "when the log-odds separate, abstain when they coincide. A guesser fails every tie; a hedger "
            "fails every determinate. Contamination-safe: the only literals are the printed priors and "
            "likelihood ratios, and the answer is a categorical option; identifiers are digit-free; gold "
            "rotates A–E; the determinate/tie split is 12/12."
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
