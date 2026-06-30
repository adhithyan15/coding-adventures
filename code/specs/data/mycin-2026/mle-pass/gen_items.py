"""Generate the MLE-PASS multi-hop recall bank (items.json).

Each item is a TWO-HOP board question answered purely from grounded edges with zero
model calls: a clinical clue → disease (hop 1, an organ-system recall library) → answer
(hop 2). The engine joins the two grounded `relate` edges through a rule body (shared
`$D`), so the answer carries BOTH hops' byte-provenance.

Three groups (slice 2):
  • GENE_CHAINS  — hop 2 = `gene_defect` (clue → disease → gene).
  • INH_CHAINS   — hop 2 = `inheritance` (clue → disease → inheritance pattern): proves the
                   harness is generic over the second relation, not gene-specific.
  • ABSTAIN      — a real rule shape whose clue has NO grounded hop-1 edge: the engine binds
                   nothing and the item MUST abstain (never fabricate).

Nothing here is authored knowledge: every chain reuses already-grounded edges that shipped
in their own spider→provenance→adversarial-gate PRs. This generator only arranges them.
"""
import json

# (id, clue, hop1_lib, hop1_rel, disease, answer, clue_phrase, question_tail)
GENE_CHAINS = [
    ("mh-01", "leukocoria", "ophtho-edges.adj", "eye_finding_indicates",
     "retinoblastoma", "rb1", "a white pupillary reflex (leukocoria)", "a mutation in which gene"),
    ("mh-02", "kayser_fleischer_rings", "ophtho-edges.adj", "eye_finding_indicates",
     "wilson_disease", "atp7b", "Kayser-Fleischer corneal rings", "a mutation in which gene"),
    ("mh-03", "superior_lens_dislocation", "ophtho-edges.adj", "eye_finding_indicates",
     "marfan_syndrome", "fbn1", "upward (superior) lens dislocation", "a mutation in which gene"),
    ("mh-04", "caudate_nucleus", "neuro-edges.adj", "lesion_causes",
     "huntington_disease", "htt", "degeneration of the caudate nucleus", "a mutation in which gene"),
    ("mh-05", "type_i", "collagen-defect-edges.adj", "defect_causes",
     "osteogenesis_imperfecta", "col1a1", "a defect in type I collagen", "a mutation in which gene"),
    ("mh-06", "alpha_galactosidase_a", "enzyme-deficiency-edges.adj", "enzyme_deficiency_disease",
     "fabry_disease", "gla", "deficiency of alpha-galactosidase A", "a mutation in which gene"),
    ("mh-07", "glucocerebrosidase", "enzyme-deficiency-edges.adj", "enzyme_deficiency_disease",
     "gaucher_disease", "gba1", "deficiency of glucocerebrosidase", "a mutation in which gene"),
    # slice 2 — new gene chains over derm + two more enzyme deficiencies
    ("mh-08", "ash_leaf_spots", "derm-edges.adj", "skin_finding_in",
     "tuberous_sclerosis", "tsc1_tsc2", "ash-leaf (hypopigmented) macules", "a mutation in which gene"),
    ("mh-09", "sphingomyelinase", "enzyme-deficiency-edges.adj", "enzyme_deficiency_disease",
     "niemann_pick_disease", "smpd1", "deficiency of sphingomyelinase", "a mutation in which gene"),
    ("mh-10", "acid_alpha_glucosidase", "enzyme-deficiency-edges.adj", "enzyme_deficiency_disease",
     "pompe_disease", "gaa", "deficiency of acid alpha-glucosidase", "a mutation in which gene"),
]
GENE_POOL = ["rb1", "atp7b", "fbn1", "htt", "col1a1", "gla", "gba1", "hexa", "cftr",
             "dmpk", "fmr1", "pah", "hfe", "ube3a", "fxn", "tsc1_tsc2", "smpd1", "gaa"]

# hop 2 = inheritance (a different second relation; answer is a transmission pattern)
INH_CHAINS = [
    ("mh-11", "caudate_nucleus", "neuro-edges.adj", "lesion_causes",
     "huntington_disease", "autosomal_dominant", "degeneration of the caudate nucleus"),
    ("mh-12", "kayser_fleischer_rings", "ophtho-edges.adj", "eye_finding_indicates",
     "wilson_disease", "autosomal_recessive", "Kayser-Fleischer corneal rings"),
    ("mh-13", "superior_lens_dislocation", "ophtho-edges.adj", "eye_finding_indicates",
     "marfan_syndrome", "autosomal_dominant", "upward (superior) lens dislocation"),
]
INH_POOL = ["autosomal_dominant", "autosomal_recessive", "x_linked_recessive",
            "x_linked_dominant", "mitochondrial"]

# slice 3 — microbiology organism-ID chain (the original MYCIN domain), run in REVERSE:
# the clue is a *disease*, hop 1 is `causes(organism, disease)` traversed backwards to bind the
# causative organism (the middle entity), and hop 2 reads that organism's Gram stain or
# microscopic morphology. Both relations live in micro-edges.adj (imports de-dupe). This
# exercises the harness's `hop1_reverse` capability — a relation joined on its FIRST argument —
# and proves a two-hop chain need not run "left to right" through the edges.
# (id, disease, hop2_relation, answer | None for abstain, clue_phrase)
MICRO_CHAINS = [
    # disease → causative organism → Gram stain
    ("mh-16", "cholera", "gram_stain", "gram_negative", "cholera"),
    ("mh-17", "gonorrhea", "gram_stain", "gram_negative", "gonorrhea"),
    ("mh-18", "pertussis", "gram_stain", "gram_negative", "whooping cough (pertussis)"),
    ("mh-19", "anthrax", "gram_stain", "gram_positive", "anthrax"),
    ("mh-20", "listeriosis", "gram_stain", "gram_positive", "listeriosis"),
    ("mh-21", "pseudomembranous_colitis", "gram_stain", "gram_positive", "pseudomembranous colitis"),
    ("mh-22", "tetanus", "gram_stain", "gram_positive", "tetanus"),
    ("mh-23", "legionnaires_disease", "gram_stain", "gram_negative", "Legionnaires disease"),
    # disease → causative organism → microscopic morphology
    ("mh-24", "cholera", "morphology", "comma_shaped", "rice-water diarrhea of cholera"),
    ("mh-25", "peptic_ulcer_disease", "morphology", "spiral", "Helicobacter peptic ulcer disease"),
    ("mh-26", "gonorrhea", "morphology", "diplococci", "gonorrhea"),
    ("mh-27", "syphilis", "morphology", "spirochete", "syphilis"),
    ("mh-28", "meningitis", "morphology", "diplococci", "meningococcal meningitis"),
    ("mh-29", "pertussis", "morphology", "coccobacilli", "whooping cough (pertussis)"),
    # abstention — a disease whose causative organism is NOT grounded: MUST abstain.
    ("mh-30", "a_syndrome_with_no_grounded_organism", "gram_stain", None,
     "a syndrome whose causative organism is not in the grounded library"),
]
GRAM_POOL = ["gram_positive", "gram_negative", "acid_fast", "gram_variable", "poorly_staining"]
MORPH_POOL = ["cocci", "diplococci", "bacilli", "comma_shaped", "spiral", "spirochete",
              "coccobacilli"]
MICRO_POOLS = {"gram_stain": GRAM_POOL, "morphology": MORPH_POOL}
MICRO_TAIL = {
    "gram_stain": "shows which Gram-stain reaction on microscopy",
    "morphology": "has which microscopic morphology",
}

# abstention — real rule shape, ungrounded clue: the engine binds nothing → MUST abstain.
ABSTAIN = [
    ("mh-14", "a_clue_with_no_grounded_eye_edge", "ophtho-edges.adj", "eye_finding_indicates",
     "gene_defect", "genetics-edges.adj", GENE_POOL, "an eye finding not in the grounded library"),
    ("mh-15", "a_lesion_with_no_grounded_edge", "neuro-edges.adj", "lesion_causes",
     "gene_defect", "genetics-edges.adj", GENE_POOL, "a neuro lesion not in the grounded library"),
]

LETTERS = ["A", "B", "C", "D", "E"]


def options_for(answer, pool, idx):
    distractors = [g for g in pool if g != answer][:4]
    opts = distractors[:]
    pos = idx % 5
    opts.insert(pos, answer)
    opts = opts[:5]
    if opts[pos] != answer:
        opts[pos] = answer
    assert len(set(opts)) == 5, (answer, opts)
    return {LETTERS[i]: opts[i] for i in range(5)}, LETTERS[pos]


def build():
    items = []
    # --- gene chains (hop2 = gene_defect) ---
    for idx, (iid, clue, lib, rel, disease, gene, phrase, tail) in enumerate(GENE_CHAINS):
        options, gold = options_for(gene, GENE_POOL, idx)
        items.append({
            "id": iid, "qtype": "multi_hop_recall",
            "stem": f"A patient has {phrase}. That finding points to a single disease; "
                    f"the disease is caused by {tail}?",
            "hop1_lib": lib, "hop1_relation": rel,
            "hop2_lib": "genetics-edges.adj", "hop2_relation": "gene_defect",
            "clue": clue, "expected": gene, "options": options, "gold_letter": gold,
        })
    # --- inheritance chains (hop2 = inheritance) ---
    for idx, (iid, clue, lib, rel, disease, patt, phrase) in enumerate(INH_CHAINS):
        options, gold = options_for(patt, INH_POOL, idx)
        items.append({
            "id": iid, "qtype": "multi_hop_recall",
            "stem": f"A patient has {phrase}. That finding points to a single disease; "
                    f"that disease is inherited in which pattern?",
            "hop1_lib": lib, "hop1_relation": rel,
            "hop2_lib": "genetics-edges.adj", "hop2_relation": "inheritance",
            "clue": clue, "expected": patt, "options": options, "gold_letter": gold,
        })
    # --- microbiology organism-ID chain (slice 3): reverse hop1 + Gram stain / morphology ---
    for idx, (iid, disease, hop2_rel, answer, phrase) in enumerate(MICRO_CHAINS):
        pool = MICRO_POOLS[hop2_rel]
        item = {
            "id": iid, "qtype": "multi_hop_recall",
            "hop1_lib": "micro-edges.adj", "hop1_relation": "causes", "hop1_reverse": True,
            "hop2_lib": "micro-edges.adj", "hop2_relation": hop2_rel,
            "clue": disease,
        }
        if answer is None:
            # ungrounded disease — no causative organism is grounded, so the chain MUST abstain.
            item.update({
                "stem": f"A patient has {phrase}. The organism that causes it {MICRO_TAIL[hop2_rel]}? "
                        f"(If the causative organism is not grounded, abstain.)",
                "expected": None, "expect_abstain": True,
                "options": {LETTERS[i]: pool[i] for i in range(5)},
            })
        else:
            options, gold = options_for(answer, pool, idx)
            item.update({
                "stem": f"A patient is diagnosed with {phrase}. The organism that causes it "
                        f"{MICRO_TAIL[hop2_rel]}?",
                "expected": answer, "options": options, "gold_letter": gold,
            })
        items.append(item)
    # --- abstention (ungrounded clue: MUST abstain) ---
    for (iid, clue, lib, rel, hop2_rel, hop2_lib, pool, phrase) in ABSTAIN:
        # plausible distractors, but NO correct answer is reachable (the chain is ungrounded).
        options = {LETTERS[i]: pool[i] for i in range(5)}
        items.append({
            "id": iid, "qtype": "multi_hop_recall",
            "stem": f"A patient has {phrase}. The disease it indicates is caused by a mutation "
                    f"in which gene? (If the chain is not grounded, abstain.)",
            "hop1_lib": lib, "hop1_relation": rel,
            "hop2_lib": hop2_lib, "hop2_relation": hop2_rel,
            "clue": clue, "expected": None, "expect_abstain": True, "options": options,
        })
    return {
        "description": (
            "MLE-PASS multi-hop recall bank — TWO-HOP board questions answered purely from "
            "grounded edges with zero model calls. A clinical clue → disease (hop 1, an "
            "organ-system recall library) → answer (hop 2) is joined on the shared disease "
            "through an adj-lang rule body; the engine's SLD resolver returns the binding with "
            "BOTH hops' byte-provenance, and the harness maps it to the printed options. "
            "Slice 2 adds: more clue→disease→gene chains (derm, niemann-pick, pompe); a second "
            "relation as hop 2 — inheritance (clue→disease→inheritance pattern), proving the "
            "harness is generic over the second hop, not gene-specific; and an ABSTENTION "
            "sub-bank whose clue has no grounded hop-1 edge, which MUST abstain (never "
            "fabricate). Slice 3 adds the microbiology organism-ID chain (the original MYCIN "
            "domain) run in REVERSE: the clue is a disease, hop 1 is causes(organism, disease) "
            "traversed backwards to bind the causative organism, and hop 2 reads that organism's "
            "Gram stain or microscopic morphology — proving a two-hop chain need not run left to "
            "right through the edges, and that both hops can share one library. Nothing authored: "
            "every edge reuses an already-grounded, spider+adversarially-gated fact. Engine: every "
            "answerable item correct with both hops cited; every abstention item abstains; zero "
            "model calls."
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
        tag = "ABSTAIN" if it.get("expect_abstain") else f"{it['expected']} ({it['gold_letter']})"
        print(it["id"], it["hop2_relation"], it["clue"], "→", tag)
