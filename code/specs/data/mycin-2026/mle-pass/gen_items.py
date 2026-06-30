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
            "fabricate). Nothing authored: every edge reuses an already-grounded, "
            "spider+adversarially-gated fact. Engine: every answerable item correct with both "
            "hops cited; every abstention item abstains; zero model calls."
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
