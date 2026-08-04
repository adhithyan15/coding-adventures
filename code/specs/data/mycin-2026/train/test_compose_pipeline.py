#!/usr/bin/env python3
"""test_compose_pipeline.py — guard the whole-paper compose driver (AC-3).

Pure structural checks always run (the paragraph/citation inventory needs no binaries). The full
four-stage assertion — derive across paragraphs, MULTI-snapshot verify, cross-paragraph explain —
runs when the adj-lang-cli / adj-verify binaries are built, and skips otherwise.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compose_pipeline as cp  # noqa: E402
import gen_argument_data as gad  # noqa: E402

_HAVE_BINS = gad.CLI.exists() and gad.VERIFY.exists()
_PAPER = cp.DEFAULT_PAPER  # the committed AC-2 composition/ example


def test_the_committed_paper_inventory_is_pure_and_multi_paragraph():
    """Stage 1 (EMIT) runs with no binaries: the committed paper has ≥2 paragraphs (a genuine
    multi-paragraph composition) and one composed .adj whose citation count is counted."""
    assert _PAPER.is_dir(), f"committed paper dir missing: {_PAPER}"
    report = cp.run_paper(_PAPER)
    assert len(report["paragraphs"]) >= 2, "a whole-paper example must span ≥2 paragraphs"
    assert report["citations"] >= len(report["paragraphs"]), "each paragraph contributes citations"
    # Each paragraph's snapshot is distinct (they are different source texts).
    assert len(set(report["paragraphs"].values())) == len(report["paragraphs"])


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
def test_full_whole_paper_pipeline_multi_snapshot():
    """With the binaries built, the whole-paper pipeline proves the emission path end to end:
    the composed .adj DERIVES the paper thesis by chaining across paragraphs, adj-verify
    byte-anchors EVERY citation across ALL the paragraph snapshots (the multi-snapshot proof), and
    --explain renders the cross-paragraph chain with per-paragraph provenance."""
    report = cp.run_paper(_PAPER)
    assert report["ran_cli"] is True

    # Stage 2 — DERIVE across paragraphs (a non-empty recall answer).
    assert '"abstained":false' in (report["derived"] or ""), "the paper thesis must derive"

    # Stage 3 — MULTI-snapshot VERIFY: every citation anchored across all paragraph snapshots.
    assert report["verified"] is True
    assert report["quotes_verified"] == report["citations"], \
        "every citation must byte-anchor across the paragraph snapshots"

    # Stage 4 — EXPLAIN: the cross-paragraph chain, with ≥2 DISTINCT paragraph provenances (proof
    # that the chain genuinely spans paragraphs, not one source).
    ex = report["explained"] or ""
    assert ex.startswith("Argument for "), f"--explain must render an argument section:\n{ex}"
    assert "<= inference" in ex and "premise " in ex
    provenances = {name for name in report["paragraphs"] if f'source "{name}"' in ex}
    assert len(provenances) >= 2, \
        f"the chain must carry ≥2 distinct paragraph provenances, saw {provenances}"


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
def test_pipeline_is_deterministic():
    """The whole-paper flow is deterministic — two runs produce identical reports."""
    assert cp.run_paper(_PAPER) == cp.run_paper(_PAPER)
