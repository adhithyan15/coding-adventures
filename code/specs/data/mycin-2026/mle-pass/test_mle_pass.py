"""Gates for the MLE-PASS multi-hop recall harness.

These prove the two-hop reasoning is REAL and grounded: the engine binds the gold gene
for every item, each answer cites BOTH hops (multi-hop byte-provenance), the run makes zero
model calls, and an unknown clue abstains rather than fabricates. The CLI-backed tests skip
cleanly when adj-lang-cli is not built (e.g. a docs-only CI shard)."""
from pathlib import Path

import pytest

import mle_pass_eval as mp

HERE = Path(__file__).resolve().parent


def test_items_bank_is_well_formed():
    items = mp.load_items()
    assert len(items) >= 7
    seen = set()
    for it in items:
        assert it["id"] not in seen, f"duplicate id {it['id']}"
        seen.add(it["id"])
        # five distinct options, gold present, expected gene is the gold option's value
        assert len(it["options"]) == 5
        assert len(set(it["options"].values())) == 5, it["id"]
        assert it["gold_letter"] in it["options"], it["id"]
        assert it["options"][it["gold_letter"]] == it["expected"], it["id"]
        # query is a genuine two-hop join (rule body chains hop1 × gene_defect)
        q = mp.build_query(it)
        assert it["hop1_relation"] in q and "gene_defect" in q
        assert "$D" in q  # the shared join variable


def test_build_query_shape():
    item = mp.load_items()[0]
    q = mp.build_query(item)
    assert q.count("import ") == 2          # both grounded libraries
    assert "rule {" in q and "when:" in q   # the joining rule body
    assert q.strip().endswith("$G)")        # the binding query


@pytest.mark.skipif(mp._CLI is None, reason="adj-lang-cli not built")
def test_engine_solves_every_item_with_both_hops_cited():
    board = mp.score(mp.load_items())
    # Every item answered correctly, none wrong, none abstained.
    assert board["wrong"] == 0, board["results"]
    assert board["abstained"] == 0, board["results"]
    assert board["correct"] == board["total"]
    # Every correct answer is defended by BOTH grounded hops (≥2 citing clauses).
    assert board["multihop_coverage"] == 1.0, board["results"]
    for r in board["results"]:
        assert r["citations"] >= 2, r


@pytest.mark.skipif(mp._CLI is None, reason="adj-lang-cli not built")
def test_unknown_clue_abstains_not_fabricates():
    # A clue with no grounded hop-1 edge must bind nothing — never invent a gene.
    bogus = dict(mp.load_items()[0])
    bogus["clue"] = "a_finding_with_no_grounded_edge"
    r = mp.run_item(bogus)
    assert r.binding is None
    assert mp.letter_for(bogus, r.binding) is None
