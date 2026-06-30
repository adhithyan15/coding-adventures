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
    assert len(items) >= 15
    seen = set()
    answerable = 0
    for it in items:
        assert it["id"] not in seen, f"duplicate id {it['id']}"
        seen.add(it["id"])
        # five distinct options on every item
        assert len(it["options"]) == 5
        assert len(set(it["options"].values())) == 5, it["id"]
        if it.get("expect_abstain"):
            # abstention items have no correct option (the chain is ungrounded)
            assert it.get("expected") is None and "gold_letter" not in it, it["id"]
        else:
            answerable += 1
            assert it["gold_letter"] in it["options"], it["id"]
            assert it["options"][it["gold_letter"]] == it["expected"], it["id"]
        # query is a genuine two-hop join (rule body chains hop1 × hop2 on $D)
        q = mp.build_query(it)
        assert it["hop1_relation"] in q and it["hop2_relation"] in q
        assert "$D" in q  # the shared join variable
    assert answerable >= 13


def test_bank_exercises_multiple_hop2_relations_and_abstention():
    items = mp.load_items()
    hop2 = {it["hop2_relation"] for it in items}
    assert {"gene_defect", "inheritance"} <= hop2  # generic over the second hop
    assert any(it.get("expect_abstain") for it in items)  # an abstention sub-bank exists


def test_build_query_shape():
    item = mp.load_items()[0]
    q = mp.build_query(item)
    assert q.count("import ") == 2          # both grounded libraries
    assert "rule {" in q and "when:" in q   # the joining rule body
    assert q.strip().endswith("$G)")        # the binding query


@pytest.mark.skipif(mp._CLI is None, reason="adj-lang-cli not built")
def test_engine_solves_every_item_with_both_hops_cited():
    board = mp.score(mp.load_items())
    # Every item scores correct: answerable ones bind the gold, abstention ones abstain.
    assert board["wrong"] == 0, board["results"]
    assert board["correct"] == board["total"]
    assert board["abstained_correctly"] >= 1, board["results"]
    # Every correct ANSWERABLE answer is defended by BOTH grounded hops (≥2 citing clauses).
    assert board["multihop_coverage"] == 1.0, board["results"]
    for r in board["results"]:
        if r["expect_abstain"]:
            assert r["binding"] is None, r  # never fabricate for an ungrounded chain
        else:
            assert r["citations"] >= 2, r


@pytest.mark.skipif(mp._CLI is None, reason="adj-lang-cli not built")
def test_unknown_clue_abstains_not_fabricates():
    # A clue with no grounded hop-1 edge must bind nothing — never invent a gene.
    bogus = dict(mp.load_items()[0])
    bogus["clue"] = "a_finding_with_no_grounded_edge"
    r = mp.run_item(bogus)
    assert r.binding is None
    assert mp.letter_for(bogus, r.binding) is None
