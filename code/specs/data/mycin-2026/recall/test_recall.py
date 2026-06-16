#!/usr/bin/env python3
"""test_recall.py — REL-1 relational-recall prototype tests.

Proves the two worked vignettes from the spec resolve to the right enzyme with a
citation, that reverse lookup is free, and — most importantly — that the store
ABSTAINS (returns UNKNOWN) on an ungrounded disease instead of guessing. All
deterministic, 0 answer-time model calls.

Run:  python3 test_recall.py
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import recall  # noqa: E402


def _store() -> recall.RelationStore:
    return recall.parse_edges(HERE / "iem-edges.adj")


def test_parse_loads_all_edges() -> None:
    store = _store()
    # 12 diseases x 3 relations = 36 edges (REL-1 seed of 6 + REL-6 expansion of 6).
    assert len(store.edges) == 36, f"expected 36 edges, got {len(store.edges)}"
    # Every edge carries a citation (source) and a trust tier.
    assert all(e.source for e in store.edges), "every edge must carry a source citation"
    assert all(e.trust for e in store.edges), "every edge must carry a trust tier"


def test_forward_recall_tay_sachs_enzyme() -> None:
    store = _store()
    hits = store.query("deficient_in", ["tay_sachs", "$Enzyme"])
    assert len(hits) == 1, "exactly one enzyme is deficient in Tay-Sachs"
    assert hits[0].bindings["$Enzyme"] == "hexosaminidase_a"
    # The answer is PROVEN: the binding carries the edge + its citation.
    assert "hexosaminidase A" in hits[0].proof.source


def test_reverse_diagnostic_then_recall_shares_the_edge() -> None:
    store = _store()
    # The differential would bind the MAP disease; recall then runs the SAME goal.
    map_disease = "tay_sachs"
    enzyme = store.query("deficient_in", [map_disease, "$Enzyme"])[0].bindings["$Enzyme"]
    substrate = store.query("accumulates", [map_disease, "$S"])[0].bindings["$S"]
    pattern = store.query("inherited_as", [map_disease, "$P"])[0].bindings["$P"]
    assert (enzyme, substrate, pattern) == (
        "hexosaminidase_a",
        "gm2_ganglioside",
        "autosomal_recessive",
    )


def test_reverse_lookup_is_free() -> None:
    store = _store()
    # "Which disease lacks hexosaminidase A?" — same edge, variable on the other side.
    hits = store.query("deficient_in", ["$Disease", "hexosaminidase_a"])
    assert [h.bindings["$Disease"] for h in hits] == ["tay_sachs"]


def test_abstains_on_ungrounded_disease() -> None:
    store = _store()
    # Wilson disease is NOT in the graph — the store must abstain, not fabricate.
    # (Niemann-Pick was the original example but is now a covered REL-6 disease.)
    assert store.query("deficient_in", ["wilson_disease", "$Enzyme"]) == []
    out = store.ask("deficient_in", ["wilson_disease", "$Enzyme"])
    assert "UNKNOWN" in out and "abstaining" in out


def test_ground_atom_mismatch_does_not_match() -> None:
    store = _store()
    # A fully-ground goal with the WRONG enzyme must not match.
    assert store.query("deficient_in", ["tay_sachs", "glucocerebrosidase"]) == []
    # The CORRECT fully-ground goal matches (a yes/no recall, no variables).
    assert len(store.query("deficient_in", ["tay_sachs", "hexosaminidase_a"])) == 1


def test_repeated_variable_must_bind_consistently() -> None:
    # A goal reusing a variable twice only matches an edge whose args agree.
    store = recall.RelationStore(
        edges=[
            recall.Edge("same", ("x", "x"), source="s", trust="consensus"),
            recall.Edge("same", ("x", "y"), source="s", trust="consensus"),
        ]
    )
    hits = store.query("same", ["$A", "$A"])
    assert len(hits) == 1 and hits[0].bindings["$A"] == "x"


def _run() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"  FAIL  {t.__name__}: {exc}")
    print(f"\ntest_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
