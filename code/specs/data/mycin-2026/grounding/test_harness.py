#!/usr/bin/env python3
"""test_harness.py — guard the generic grounding harness + source verification.

Pure (no spider/engine): the shared gate verdict, source-object content addressing,
and the citation-verification — a cited quote that IS in the decomposed source
verifies; one that is NOT fails ('the source doesn't say what was implied').
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402


def src() -> h.SourceObject:
    return h.SourceObject(
        source_id="pubmed:15509818",
        title="van de Beek, NEJM 2004",
        resolved_url="https://pubmed.ncbi.nlm.nih.gov/15509818/",
        claims=[h.SourceClaim(id="c1",
                              text="S. pneumoniae 51% and N. meningitidis 37% of episodes",
                              byte_quote="The most common pathogens were Streptococcus pneumoniae "
                                         "(51 percent of episodes) and Neisseria meningitidis (37 percent).")],
        cites=[],
    )


def test_gate_verdicts() -> None:
    assert h.gate("grounded") == ("ACCEPT", "authoritative")
    for s in ("direction_only", "refuted", "ungrounded", "missing"):
        assert h.gate(s) == ("FLAG", "inferred")


def test_source_object_content_addressing() -> None:
    a, b = src(), src()
    assert a.content_hash() == b.content_hash(), "same content → same hash"
    # changing a claim quote changes the hash (tamper-evident)
    b.claims = [h.SourceClaim(id="c1", text="x", byte_quote="different quote")]
    assert a.content_hash() != b.content_hash()


def test_citation_verified_when_quote_in_source() -> None:
    # The fact's cited quote IS present in the decomposed source → verified.
    v = h.verify_citation("Streptococcus pneumoniae (51 percent of episodes)", src())
    assert v["verified"] and v["matched_claim"] == "c1", v
    # Whitespace/case differences don't break it.
    v2 = h.verify_citation("streptococcus pneumoniae  (51 PERCENT of episodes)", src())
    assert v2["verified"], v2


def test_citation_robust_to_markdown_and_entities() -> None:
    # A web fetch renders the SAME bytes with Markdown emphasis / HTML entities.
    # The fact quotes plain prose; verification must see through the markup.
    so = h.SourceObject(
        source_id="s", title="t", resolved_url="u",
        claims=[h.SourceClaim(id="c1", text="x",
                              byte_quote="_S. pneumoniae_ was predominant (P&lt;0.001)")])
    assert h.verify_citation("S. pneumoniae was predominant (P<0.001)", so)["verified"]
    # En/em-dash and "--" are the same range to a reader: "9–23" must match "9--23".
    so2 = h.SourceObject(source_id="s", title="t", resolved_url="u",
                         claims=[h.SourceClaim(id="c1", text="x",
                                 byte_quote="occurred 9--23 times more frequently in dormitories")])
    assert h.verify_citation("occurred 9–23 times more frequently in dormitories", so2)["verified"]


def test_composite_quote_fragment_coverage() -> None:
    # A citation stitched with "…": the load-bearing first span is in the source, a
    # bundled-context span is NOT. Result: not fully verified, but core_verified.
    so = h.SourceObject(
        source_id="s", title="t", resolved_url="u",
        claims=[h.SourceClaim(id="c1", text="x", byte_quote="Listeria monocytogenes (182/2974; 6%)")])
    v = h.verify_citation("Listeria monocytogenes (182/2974; 6%) ... Mortality was 32%", so)
    assert not v["verified"] and v["core_verified"], v
    assert v["fragments_matched"] == 1 and v["fragments_total"] == 2, v
    assert "over-reach" in v["reason"]
    # If every fragment IS present, the whole composite verifies.
    so.claims.append(h.SourceClaim(id="c2", text="m", byte_quote="Mortality was 32% in this cohort"))
    assert h.verify_citation("Listeria monocytogenes (182/2974; 6%) ... Mortality was 32%", so)["verified"]


def test_citation_unverified_when_quote_absent() -> None:
    # A quote the source does NOT contain → unverified ('does the source say it?').
    v = h.verify_citation("Listeria caused 80 percent of cases", src())
    assert not v["verified"] and v["matched_claim"] is None, v
    assert "does the source say" in v["reason"].lower() or "not" in v["reason"].lower()
    assert not h.verify_citation("", src())["verified"]


def test_roundtrip_cas(tmp=Path("/tmp/_mycin_src_test")) -> None:
    import shutil
    orig = h.SOURCES_DIR
    h.SOURCES_DIR = tmp
    try:
        shutil.rmtree(tmp, ignore_errors=True)
        hh = h.write_source_object(src())
        loaded = h.load_source_object(hh)
        assert loaded is not None and loaded.claims[0].id == "c1"
        assert h.verify_citation("(51 percent of episodes)", loaded)["verified"]
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
        h.SOURCES_DIR = orig


def main() -> int:
    test_gate_verdicts()
    test_source_object_content_addressing()
    test_citation_verified_when_quote_in_source()
    test_citation_robust_to_markdown_and_entities()
    test_composite_quote_fragment_coverage()
    test_citation_unverified_when_quote_absent()
    test_roundtrip_cas()
    print("test_harness: PASS (gate; source content-addressing; citation verified iff the "
          "quote is genuinely in the decomposed source; CAS round-trip)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
