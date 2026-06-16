#!/usr/bin/env python3
"""test_vitamin_edge_ground.py — REL-10 vitamin edge write-gate tests.

The per-edge rendering (_edge_block) is shared with iem_edge_ground.py and tested
there; this pins the vitamin gate's own wiring: the committed vitamin-edges.adj is
gate-generated + up to date, carries the vitamin vocabulary, and a grounded fixture
record lifts an edge to authoritative.

Run:  python3 test_vitamin_edge_ground.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import vitamin_edge_ground as vit  # noqa: E402


def test_check_up_to_date_on_committed_file() -> None:
    r = subprocess.run(
        [sys.executable, str(HERE / "vitamin_edge_ground.py"), "--check"],
        capture_output=True, text=True,
    )
    assert r.returncode == 0, f"committed vitamin-edges.adj is stale: {r.stdout}{r.stderr}"
    assert "up to date" in r.stdout


def test_generated_file_carries_the_vitamin_vocabulary() -> None:
    text = (HERE / "vitamin-edges.adj").read_text()
    assert "dictionary vitamin_vocab" in text
    assert "define deficiency_causes : relation from vitamin to disease" in text
    assert "relate deficiency_causes(thiamine, beriberi)" in text


def test_grounded_record_lifts_an_edge_to_authoritative() -> None:
    # The shared renderer: a grounded record supplies the byte-quote + url and lifts
    # trust to authoritative (vitamin edges use the same _edge_block as IEM).
    rec = {
        "spider_status": "grounded",
        "grounded": {"byte_quote": "Thiamine deficiency causes beriberi.",
                     "resolved_url": "https://www.ncbi.nlm.nih.gov/books/NBK537204/"},
    }
    block, entry = vit.iem._edge_block(
        "deficiency_causes", "thiamine", "beriberi",
        "authored fallback", rec,
    )
    assert "trust authoritative" in block
    assert "Thiamine deficiency causes beriberi." in block
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
    print(f"\ntest_vitamin_edge_ground: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
