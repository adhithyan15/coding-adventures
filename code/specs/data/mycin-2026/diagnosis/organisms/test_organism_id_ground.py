#!/usr/bin/env python3
"""test_organism_id_ground.py — guard the organism-id write gate (debt retirement).

Pure checks of the gate logic (no spider/engine): the source-value parser and the
ACCEPT/FLAG verdicts per spider status. If the grounding file + CLI are present, an
integration check that the regenerated rulebook still runs the differential.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import organism_id_ground as g  # noqa: E402


def test_parse_proportion() -> None:
    assert g.parse_proportion("S. pneumoniae caused 51% of episodes (proportion ~0.51)") == 0.51
    assert g.parse_proportion("~0.139 (13.9%) of cases") == 0.139
    assert g.parse_proportion("~0.06 (6%; 182/2974 episodes)") == 0.06
    assert g.parse_proportion("13.9%") == 0.139
    assert g.parse_proportion("definitional") is None
    assert g.parse_proportion("") is None


def test_gate_verdicts() -> None:
    assert g.gate("grounded") == ("ACCEPT", "authoritative")
    assert g.gate("direction_only") == ("FLAG", "inferred")
    assert g.gate("refuted") == ("FLAG", "inferred")
    assert g.gate("ungrounded") == ("FLAG", "inferred")
    assert g.gate("missing") == ("FLAG", "inferred")


def test_parse_proportion_rejects_out_of_range() -> None:
    # Untrusted spider text claiming an impossible proportion → None (→ fallback).
    assert g.parse_proportion("proportion ~9.99 of cases") is None
    assert g.parse_proportion("5000%") is None
    assert g.parse_proportion("0%") == 0.0


def test_cite_escapes_for_adj_lang_string() -> None:
    assert "PENDING" in g.cite(None)
    # A quote is escaped (\"), not stripped, so the citation is faithful + parseable.
    assert g.cite({"grounded": {"source_title": 'A "quoted" study'}}) == r'A \"quoted\" study'
    # A trailing backslash must NOT be able to escape the closing string quote.
    out = g.cite({"grounded": {"source_title": "ends with backslash\\"}})
    assert out.endswith("\\\\"), out  # the lone backslash is doubled (escaped)
    # Control chars collapsed to spaces (no breaking out of the line).
    assert "\n" not in g.cite({"grounded": {"source_title": "line1\nline2"}})


def main() -> int:
    test_parse_proportion()
    test_parse_proportion_rejects_out_of_range()
    test_gate_verdicts()
    test_cite_escapes_for_adj_lang_string()

    if not g.GROUNDING.exists():
        print("test_organism_id_ground: PASS (gate logic); grounding file not present "
              "(run the spider) — regeneration check SKIPPED")
        return 0
    # The regenerated rulebook is byte-identical to what --check expects.
    rc = g.build(check=True)
    assert rc == 0, "organism-id.adj is out of date vs the grounding (run organism_id_ground.py)"
    print("test_organism_id_ground: PASS (parser + verdicts; regenerated rulebook up to date)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
