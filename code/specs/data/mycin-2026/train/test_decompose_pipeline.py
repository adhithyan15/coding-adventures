#!/usr/bin/env python3
"""test_decompose_pipeline.py — guard the decompose→emit→verify→explain pipeline (AD-5).

Pure structural checks always run (stage 1, EMIT, needs no binaries). The full four-stage
assertion runs when the adj-lang-cli / adj-verify binaries are built, and skips otherwise — the
same graceful degradation the rest of the AD scaffold uses.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import decompose_pipeline as dp  # noqa: E402
import gen_argument_data as gad  # noqa: E402

_HAVE_BINS = gad.CLI.exists() and gad.VERIFY.exists()


@pytest.mark.parametrize("spec", gad.SEED, ids=[s["id"] for s in gad.SEED])
def test_emit_stage_is_pure_and_well_formed(spec):
    """Stage 1 (EMIT) runs with no binaries: it produces a well-formed .adj for every seed."""
    report = dp.run_pipeline(spec) if not _HAVE_BINS else None
    # Emit is pure; verify it directly regardless of binaries.
    sb = gad.source_bytes_for(spec)
    adj_text, _ = gad.build_argument_adj(spec, sb)
    assert adj_text.startswith(f'argument {spec["name"]} {{')
    assert adj_text.rstrip().endswith(f'? {spec["thesis"]}')
    if report is not None:
        assert report["emitted"] == adj_text and report["ran_cli"] is False


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
@pytest.mark.parametrize("spec", gad.SEED, ids=[s["id"] for s in gad.SEED])
def test_full_four_stage_pipeline(spec):
    """With the binaries built, the pipeline proves the emission path end to end for every seed:
    it emits an .adj that DERIVES the thesis, BYTE-ANCHORS every citation, and whose --explain
    renders the premises → connective → conclusion chain."""
    report = dp.run_pipeline(spec)
    assert report["ran_cli"] is True

    # Stage 2 — DERIVE: the paragraph's thesis is reached (the expected bound value appears).
    assert spec["expect"] in (report["derived"] or ""), f"{spec['id']}: thesis must derive"

    # Stage 3 — VERIFY: every citation byte-anchored.
    assert report["verified"] is True
    assert report["quotes_verified"] == gad.total_citations(spec)

    # Stage 4 — EXPLAIN: the chain renders as premises → connective → conclusion.
    ex = report["explained"] or ""
    assert ex.startswith("Argument for "), f"{spec['id']}: --explain must render an argument section"
    assert "<= inference" in ex, "the conclusion(s) render as inference (connective) steps"
    assert "premise " in ex, "the grounded premises render as premise lines"
    # The derived conclusion value appears in the rendered chain (not the open query variable).
    assert spec["expect"] in ex, f"{spec['id']}: the derived conclusion shows in the chain"


@pytest.mark.skipif(not _HAVE_BINS, reason="adj-lang-cli / adj-verify not built")
def test_pipeline_is_deterministic():
    """The whole flow is deterministic — two runs of a seed produce identical reports."""
    spec = gad.SEED[0]
    a, b = dp.run_pipeline(spec), dp.run_pipeline(spec)
    assert a == b, "the pipeline must be byte-identical across runs"
