#!/usr/bin/env python3
"""test_gen_argument_data.py — guard the prose→ARGUMENT decomposer gold generator (AD-2).

Two layers:

  PURE (always run, no binaries): the deterministic gold-builder's guarantees — every
  citation's byte offset points at the verbatim quote, the snapshot is the paragraph's
  SHA-256, every gold span is a real substring of the paragraph (span-faithfulness), the
  emitted `.adj` is well-formed, the training row matches the §3.2 schema, and a quote that
  is NOT in the paragraph raises the fabrication guard `SpanNotFound`.

  GATE (skipped if adj-lang-cli / adj-verify are not built): each seed's gold `.adj` passes
  the three-part correctness gate — it COMPILES, `adj-lang-cli` DERIVES its thesis, and
  `adj-verify --snapshots` BYTE-ANCHORS every citation. This is the same discipline
  test_gen_data.py applies to the closed-vocab shape, extended to the open-vocab argument.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import gen_argument_data as g  # noqa: E402


# ---------------------------------------------------------------------------
# PURE — the deterministic builder's byte-provenance guarantees.
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("spec", g.SEED, ids=[s["id"] for s in g.SEED])
def test_every_citation_offset_points_at_the_verbatim_quote(spec):
    """The core byte-provenance invariant: for every premise/inference, the recorded
    `at <offset>` is exactly where its quote begins in the paragraph, and the bytes there
    are the quote verbatim. If this holds, adj-verify's byte-anchor can only pass."""
    sb = g.source_bytes_for(spec)
    adj_text, hexhash = g.build_argument_adj(spec, sb)
    assert hexhash == hashlib.sha256(sb).hexdigest(), "snapshot must be the paragraph SHA-256"
    for item in spec["premises"] + spec["inferences"]:
        quote = item["quote"].encode("utf-8")
        off = sb.find(quote)
        assert off >= 0, f"{spec['id']}: quote not in paragraph: {item['quote']!r}"
        assert sb[off:off + len(quote)] == quote, "bytes at the offset must be the quote"
        # and the emitted program records that exact offset + hash.
        assert f'at {off} snapshot "{hexhash}"' in adj_text


@pytest.mark.parametrize("spec", g.SEED, ids=[s["id"] for s in g.SEED])
def test_gold_spans_are_faithful_substrings(spec):
    """Span-faithfulness (the spec §4 metric): every gold premise/inference/discard span is
    a verbatim substring of the paragraph — never a paraphrase the model would have to
    invent."""
    source = g.source_bytes_for(spec).decode("utf-8")
    row = g.to_training_row(spec, source)
    gold = row["gold"]
    for p in gold["premises"]:
        assert p["span"] in source, f"premise span not verbatim: {p['span']!r}"
    for i in gold["inferences"]:
        assert i["span"] in source, f"inference span not verbatim: {i['span']!r}"
    for d in gold["discard"]:
        assert d["span"] in source, f"discard span not verbatim: {d['span']!r}"


@pytest.mark.parametrize("spec", g.SEED, ids=[s["id"] for s in g.SEED])
def test_emitted_adj_is_well_formed(spec):
    """The `.adj` opens the named argument, carries one line per premise/inference, and ends
    with the `? thesis` query."""
    sb = g.source_bytes_for(spec)
    adj_text, _ = g.build_argument_adj(spec, sb)
    assert adj_text.startswith(f'argument {spec["name"]} {{')
    assert adj_text.rstrip().endswith(f'? {spec["thesis"]}')
    for p in spec["premises"]:
        assert f'premise {p["name"]} : {p["kind"]} {p["term"]} ' in adj_text
    for i in spec["inferences"]:
        refs = ", ".join(i["from"])
        assert f'infer {i["name"]} : {i["connective"]} conclude {i["conclusion"]} from {refs} ' in adj_text


@pytest.mark.parametrize("spec", g.SEED, ids=[s["id"] for s in g.SEED])
def test_training_row_matches_schema(spec):
    """The JSONL row carries the §3.2 fields; it round-trips through JSON; and the gold has at
    least one premise, one inference, and a thesis (a real argument, not an empty shell)."""
    source = g.source_bytes_for(spec).decode("utf-8")
    row = g.to_training_row(spec, source)
    assert json.loads(json.dumps(row)) == row  # JSON-clean
    assert row["shape"] == "argument"
    assert row["note"] == source
    gold = row["gold"]
    assert gold["premises"] and gold["inferences"] and gold["thesis"]
    for p in gold["premises"]:
        assert set(p) == {"name", "kind", "term", "span", "type"}
    for i in gold["inferences"]:
        assert set(i) == {"name", "connective", "conclusion", "from", "span", "type"}


def test_open_vocab_spans_three_domains():
    """The seed proves generalization: the predicates/entities are NOT a fixed vocabulary —
    the seed covers at least three distinct domains with disjoint predicate sets."""
    domains = {s["domain"] for s in g.SEED}
    assert len(domains) >= 3, f"seed must span ≥3 domains for open-vocab evidence: {domains}"
    functors = set()
    for s in g.SEED:
        for p in s["premises"]:
            functors.add(p["term"].split("(", 1)[0])
    # materials-science, epidemiology, astronomy predicates don't overlap.
    assert len(functors) >= 6, f"open-vocab: expected many distinct predicates, got {functors}"


def test_fabricated_quote_raises_span_not_found():
    """The fabrication guard: a citation whose quote is not a verbatim slice of the paragraph
    is refused at build time — the builder never emits a citation adj-verify would reject."""
    spec = {
        "name": "bogus", "doc": "d", "trust": "authoritative",
        "premises": [{"name": "p1", "kind": "extracted", "term": "foo(bar)",
                      "quote": "this phrase is absent from the source"}],
        "inferences": [], "thesis": "foo($X)",
    }
    with pytest.raises(g.SpanNotFound):
        g.build_argument_adj(spec, b"the source says something entirely different")


# ---------------------------------------------------------------------------
# GATE — the three-part correctness check (needs the built binaries).
# ---------------------------------------------------------------------------

_HAVE_BINS = g.CLI.exists() and g.VERIFY.exists()


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
@pytest.mark.parametrize("spec", g.SEED, ids=[s["id"] for s in g.SEED])
def test_seed_passes_the_three_part_gate(spec):
    """Each seed's gold `.adj` COMPILES, DERIVES its thesis, and BYTE-ANCHORS every citation."""
    sb = g.source_bytes_for(spec)
    adj_text, _ = g.build_argument_adj(spec, sb)
    res = g.verify_gold(adj_text, sb)
    assert res["derive_ok"], f"{spec['id']}: must compile + run"
    assert spec["expect"] in res["derive_stdout"], f"{spec['id']}: thesis must derive ({spec['expect']})"
    assert res["verified"] is True, f"{spec['id']}: adj-verify must pass"
    assert res["quotes_verified"] == g.total_citations(spec), \
        f"{spec['id']}: every citation must byte-anchor"
