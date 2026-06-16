#!/usr/bin/env python3
"""test_coag_edge_ground.py — REL-13 coagulation edge write-gate tests.

The renderer is shared (iem_edge_ground._edge_block, tested there); this pins the
coagulation gate's own wiring: the committed coag-edges.adj is gate-generated + up to
date, carries the coagulation vocabulary, and — crucially — RESOLVES the board-classic
binding queries through the SAME RelationStore engine the board harness uses (so this
domain is answerable the moment its filename is added to board_eval's EDGE_FILES).

Run:  python3 test_coag_edge_ground.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import coag_edge_ground as coag  # noqa: E402
import recall  # noqa: E402


def test_check_up_to_date_on_committed_file() -> None:
    r = subprocess.run(
        [sys.executable, str(HERE / "coag_edge_ground.py"), "--check"],
        capture_output=True, text=True,
    )
    assert r.returncode == 0, f"committed coag-edges.adj is stale: {r.stdout}{r.stderr}"
    assert "up to date" in r.stdout


def test_generated_file_carries_the_coag_vocabulary() -> None:
    text = (HERE / "coag-edges.adj").read_text()
    assert "dictionary coag_vocab" in text
    assert "define factor_deficiency : relation from disorder to factor" in text
    assert "define prolonged_test    : relation from disorder to test" in text
    assert "relate factor_deficiency(hemophilia_a, factor_viii)" in text


def _store() -> "recall.RelationStore":
    s = recall.RelationStore()
    s.edges.extend(recall.parse_edges(HERE / "coag-edges.adj").edges)
    return s


def test_recall_queries_resolve_through_the_engine() -> None:
    s = _store()
    # The three relations query one disorder three ways — all board-classic.
    cases = [
        ("factor_deficiency", "hemophilia_a", "factor_viii"),
        ("factor_deficiency", "hemophilia_b", "factor_ix"),
        ("factor_deficiency", "von_willebrand_disease", "von_willebrand_factor"),
        ("factor_deficiency", "hemophilia_c", "factor_xi"),
        ("coag_inheritance", "hemophilia_a", "x_linked_recessive"),
        ("coag_inheritance", "von_willebrand_disease", "autosomal_dominant"),
        ("coag_inheritance", "factor_vii_deficiency", "autosomal_recessive"),
        ("prolonged_test", "hemophilia_a", "aptt"),       # intrinsic pathway → aPTT
        ("prolonged_test", "factor_vii_deficiency", "pt"),  # extrinsic pathway → PT
        ("prolonged_test", "von_willebrand_disease", "bleeding_time"),
    ]
    for rel, subj, expected in cases:
        hits = s.query(rel, [subj, "$X"])
        assert hits, f"no binding for {rel}({subj}, $X)"
        assert hits[0].bindings.get("$X") == expected, f"{rel}({subj}) → {hits[0].bindings.get('$X')} != {expected}"


def test_uncovered_disorder_has_no_binding() -> None:
    # A disorder deliberately absent from the graph must NOT resolve — the harness will
    # abstain on it rather than fabricate. (Platelet disorders aren't in this factor graph.)
    s = _store()
    assert not s.query("factor_deficiency", ["bernard_soulier", "$X"])
    assert not s.query("factor_deficiency", ["glanzmann_thrombasthenia", "$X"])


def test_grounded_record_lifts_an_edge_to_authoritative() -> None:
    rec = {
        "spider_status": "grounded",
        "grounded": {"byte_quote": "Hemophilia A is caused by a deficiency of factor VIII.",
                     "resolved_url": "https://www.ncbi.nlm.nih.gov/books/NBK470265/"},
    }
    block, entry = coag.iem._edge_block(
        "factor_deficiency", "hemophilia_a", "factor_viii", "authored fallback", rec,
    )
    assert "trust authoritative" in block
    assert "factor VIII" in block
    assert entry["verdict"] == "ACCEPT" and entry["trust"] == "authoritative"


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
    print(f"\ntest_coag_edge_ground: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
