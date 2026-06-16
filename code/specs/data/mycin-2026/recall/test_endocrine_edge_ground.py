#!/usr/bin/env python3
"""test_endocrine_edge_ground.py — REL-12 endocrine edge write-gate tests.

The renderer is shared (iem_edge_ground._edge_block, tested there); this pins the
endocrine gate's own wiring: the committed endocrine-edges.adj is gate-generated + up
to date and carries the endocrine vocabulary.

Run:  python3 test_endocrine_edge_ground.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import endocrine_edge_ground as endo  # noqa: E402


def test_check_up_to_date_on_committed_file() -> None:
    r = subprocess.run(
        [sys.executable, str(HERE / "endocrine_edge_ground.py"), "--check"],
        capture_output=True, text=True,
    )
    assert r.returncode == 0, f"committed endocrine-edges.adj is stale: {r.stdout}{r.stderr}"
    assert "up to date" in r.stdout


def test_generated_file_carries_the_endocrine_vocabulary() -> None:
    text = (HERE / "endocrine-edges.adj").read_text()
    assert "dictionary endocrine_vocab" in text
    assert "define secreted_by         : relation from hormone to gland" in text
    assert "relate secreted_by(cortisol, adrenal_cortex)" in text


def test_grounded_record_lifts_an_edge_to_authoritative() -> None:
    rec = {
        "spider_status": "grounded",
        "grounded": {"byte_quote": "Cortisol is secreted by the adrenal cortex.",
                     "resolved_url": "https://www.ncbi.nlm.nih.gov/books/NBK538239/"},
    }
    block, entry = endo.iem._edge_block(
        "secreted_by", "cortisol", "adrenal_cortex", "authored fallback", rec,
    )
    assert "trust authoritative" in block
    assert "adrenal cortex" in block
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
    print(f"\ntest_endocrine_edge_ground: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
