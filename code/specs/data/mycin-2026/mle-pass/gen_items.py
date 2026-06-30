"""Generate the MLE-PASS multi-hop recall bank (items.json).

Each item is a TWO-HOP board question answered purely from grounded edges with zero
model calls: a clinical clue → disease (hop 1, an organ-system recall library) → gene
(hop 2, the genetics library). The engine joins the two grounded `relate` edges through
a rule body (shared `$D`), so the answer carries BOTH hops' byte-provenance.

Nothing here is authored knowledge: every chain reuses already-grounded edges that
shipped in their own spider→provenance→adversarial-gate PRs. This generator only
arranges existing edges into MCQs.
"""
import json

# (id, clue, hop1_lib, hop1_rel, disease, gene, clue_phrase)
CHAINS = [
    ("mh-01", "leukocoria", "ophtho-edges.adj", "eye_finding_indicates",
     "retinoblastoma", "rb1", "a white pupillary reflex (leukocoria)"),
    ("mh-02", "kayser_fleischer_rings", "ophtho-edges.adj", "eye_finding_indicates",
     "wilson_disease", "atp7b", "Kayser-Fleischer corneal rings"),
    ("mh-03", "superior_lens_dislocation", "ophtho-edges.adj", "eye_finding_indicates",
     "marfan_syndrome", "fbn1", "upward (superior) lens dislocation"),
    ("mh-04", "caudate_nucleus", "neuro-edges.adj", "lesion_causes",
     "huntington_disease", "htt", "degeneration of the caudate nucleus"),
    ("mh-05", "type_i", "collagen-defect-edges.adj", "defect_causes",
     "osteogenesis_imperfecta", "col1a1", "a defect in type I collagen"),
    ("mh-06", "alpha_galactosidase_a", "enzyme-deficiency-edges.adj", "enzyme_deficiency_disease",
     "fabry_disease", "gla", "deficiency of alpha-galactosidase A"),
    ("mh-07", "glucocerebrosidase", "enzyme-deficiency-edges.adj", "enzyme_deficiency_disease",
     "gaucher_disease", "gba1", "deficiency of glucocerebrosidase"),
]

# Distractor pool: real genes from the genetics library (so every option is a plausible gene).
GENE_POOL = ["rb1", "atp7b", "fbn1", "htt", "col1a1", "gla", "gba1", "hexa", "cftr",
             "dmpk", "fmr1", "pah", "hfe", "ube3a", "fxn"]


def build():
    items = []
    letters = ["A", "B", "C", "D", "E"]
    for idx, (iid, clue, lib, rel, disease, gene, phrase) in enumerate(CHAINS):
        # five distinct gene options: the gold + four distractors from the pool.
        distractors = [g for g in GENE_POOL if g != gene][:4]
        opts = distractors[:]
        pos = idx % 5
        opts.insert(pos, gene)
        opts = opts[:5]
        # guarantee the gold sits at `pos` and options are distinct
        if opts[pos] != gene:
            opts[pos] = gene
        assert len(set(opts)) == 5, (iid, opts)
        options = {letters[i]: opts[i] for i in range(5)}
        gold_letter = letters[opts.index(gene)]
        stem = (
            f"A patient has {phrase}. That finding points to a single disease; "
            f"the disease is caused by a mutation in which gene?"
        )
        items.append({
            "id": iid,
            "qtype": "multi_hop_recall",
            "stem": stem,
            "hop1_lib": lib,
            "hop1_relation": rel,
            "hop2_lib": "genetics-edges.adj",
            "hop2_relation": "gene_defect",
            "clue": clue,
            "expected": gene,
            "options": options,
            "gold_letter": gold_letter,
        })
    return {
        "description": (
            "MLE-PASS multi-hop recall bank — TWO-HOP board questions answered purely from "
            "grounded edges with zero model calls. Each item chains a clinical clue → disease "
            "(hop 1, an organ-system recall library: ophtho / neuro / collagen / enzyme-deficiency) "
            "→ gene (hop 2, the genetics library) by joining two grounded `relate` edges through a "
            "rule body (shared disease variable). The engine resolves the join (SLD) and returns the "
            "gene binding carrying BOTH hops' byte-provenance; the harness maps the binding to the "
            "printed gene options. Nothing is authored: every edge reuses an already-grounded, "
            "spider+adversarially-gated fact. This is the first slice of the MLE-PASS harness — the "
            "multi-hop reasoning step past the single-hop recall rungs, toward board coverage."
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
        print(it["id"], it["clue"], "→", it["expected"], it["gold_letter"], it["options"])
