#!/usr/bin/env python3
"""test_iem_edge_ground.py — REL-4 IEM edge write-gate tests.

Exercises the gate's per-edge rendering directly (no file writes), plus a `--check`
on the committed file. Proves: an ungrounded/refuted edge stays consensus + FLAG
(authored-debt visible), a grounded edge lifts to `trust authoritative` carrying the
grounded byte-quote + URL, an untrusted quote is escaped for the adj-lang literal,
and the committed iem-edges.adj is gate-generated + up to date.

Run:  python3 test_iem_edge_ground.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import iem_edge_ground as g  # noqa: E402

AUTHORED = "Tay-Sachs disease results from deficiency of the enzyme hexosaminidase A (HEXA)."


def test_ungrounded_edge_is_consensus_flagged() -> None:
    block, entry = g._edge_block("deficient_in", "tay_sachs", "hexosaminidase_a", AUTHORED, None)
    assert "trust consensus" in block
    assert "% [FLAG: pending]" in block
    assert AUTHORED in block          # the authored source is preserved, not dropped
    assert entry["verdict"] == "FLAG"
    assert entry["trust"] == "consensus"


def test_grounded_edge_is_authoritative_with_quote_and_locator() -> None:
    rec = {
        "spider_status": "grounded",
        "grounded": {
            "byte_quote": "OMIM #272800: Tay-Sachs disease is caused by HEXA deficiency.",
            "resolved_url": "https://omim.org/entry/272800",
        },
    }
    block, entry = g._edge_block("deficient_in", "tay_sachs", "hexosaminidase_a", AUTHORED, rec)
    assert "trust authoritative" in block
    assert "OMIM #272800" in block                  # the GROUNDED quote, not the authored one
    assert 'locator "https://omim.org/entry/272800"' in block
    assert "% [FLAG" not in block                    # grounded → no flag
    assert entry["verdict"] == "ACCEPT"
    assert entry["trust"] == "authoritative"


def test_refuted_edge_stays_consensus_flagged() -> None:
    rec = {"spider_status": "refuted", "grounded": {"byte_quote": "x", "resolved_url": "u"}}
    block, entry = g._edge_block("deficient_in", "tay_sachs", "hexosaminidase_a", AUTHORED, rec)
    # A refuted grounding never silently replaces the authored value — kept + flagged.
    assert "trust consensus" in block
    assert "% [FLAG: refuted]" in block
    assert entry["verdict"] == "FLAG"


def test_untrusted_quote_is_escaped_for_the_adj_literal() -> None:
    # A spider quote containing a double-quote (or trailing backslash) must not break
    # the `source "..."` literal and corrupt the parse of following clauses.
    rec = {
        "spider_status": "grounded",
        "grounded": {"byte_quote": 'he said "deficiency" \\', "resolved_url": "u"},
    }
    block, _ = g._edge_block("deficient_in", "tay_sachs", "hexosaminidase_a", AUTHORED, rec)
    # No RAW double-quote inside the source value; backslash doubled.
    assert '\\"deficiency\\"' in block
    assert block.count('source "') == 1


def test_check_up_to_date_on_committed_file() -> None:
    # The committed iem-edges.adj must be exactly what the gate generates (it is
    # gate-owned, not hand-edited) — so --check passes with no grounding JSON.
    r = subprocess.run(
        [sys.executable, str(HERE / "iem_edge_ground.py"), "--check"],
        capture_output=True, text=True,
    )
    assert r.returncode == 0, f"committed iem-edges.adj is stale: {r.stdout}{r.stderr}"
    assert "up to date" in r.stdout


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
    print(f"\ntest_iem_edge_ground: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run())
