#!/usr/bin/env python3
"""test_anemia_edge_ground.py — REL-11 anemia edge write-gate tests.

The per-edge renderer is shared (iem_edge_ground._edge_block, tested there); this
pins the anemia gate's own wiring: the committed anemia-edges.adj is gate-generated +
up to date and carries the anemia vocabulary.

Run:  python3 test_anemia_edge_ground.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import anemia_edge_ground as ane  # noqa: E402


def test_check_up_to_date_on_committed_file() -> None:
    r = subprocess.run(
        [sys.executable, str(HERE / "anemia_edge_ground.py"), "--check"],
        capture_output=True, text=True,
    )
    assert r.returncode == 0, f"committed anemia-edges.adj is stale: {r.stdout}{r.stderr}"
    assert "up to date" in r.stdout


def test_generated_file_carries_the_anemia_vocabulary() -> None:
    text = (HERE / "anemia-edges.adj").read_text()
    assert "dictionary anemia_vocab" in text
    assert "define has_mcv         : relation from anemia to mcv_class" in text
    assert "relate has_mcv(iron_deficiency_anemia, microcytic)" in text


def test_grounded_record_lifts_an_edge_to_authoritative() -> None:
    rec = {
        "spider_status": "grounded",
        "grounded": {"byte_quote": "Iron deficiency anemia is a microcytic anemia.",
                     "resolved_url": "https://www.ncbi.nlm.nih.gov/books/NBK448065/"},
    }
    block, entry = ane.iem._edge_block(
        "has_mcv", "iron_deficiency_anemia", "microcytic", "authored fallback", rec,
    )
    assert "trust authoritative" in block
    assert "microcytic anemia" in block
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
    print(f"\ntest_anemia_edge_ground: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
