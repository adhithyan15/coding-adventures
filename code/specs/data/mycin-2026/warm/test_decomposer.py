#!/usr/bin/env python3
"""test_decomposer.py - guard local-first backend selection + the live entry point.

Runs with NO model: a stub `gen` is injected so the decompose path is exercised
deterministically. Backend availability is checked structurally (mlx unavailable
without config; selection raises when nothing is available). CI runs this.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decomposer as dc  # noqa: E402

CANNED = (
    '{"findings": ['
    '{"term": "fever(present)", "span": "fever", "type": "stated", "polarity": "affirmed"},'
    '{"term": "meningismus(present)", "span": "neck stiffness", "type": "stated", "polarity": "affirmed"}'
    '], "discard": [], "inference_justifications": []}'
)


def test_decompose_text_with_injected_backend() -> None:
    """The live entry point works with any `gen`, no real model required."""
    ir = dc.decompose_text("72M with fever and neck stiffness", gen=lambda _p: CANNED)
    assert ir["case_id"] == "live"
    terms = {f["term"] if isinstance(f, dict) else f for f in ir["findings"]}
    assert "fever(present)" in terms, ir["findings"]
    assert "meningismus(present)" in terms, ir["findings"]


def test_decompose_tolerates_unparseable_output() -> None:
    """A backend that emits junk yields an empty-but-valid IR (abstain, not crash)."""
    ir = dc.decompose_text("anything", gen=lambda _p: "not json at all")
    assert ir["findings"] == [] and ir["discard"] == []


def test_mlx_backend_unavailable_without_config() -> None:
    """No MYCIN_MLX_MODEL set -> the MLX backend reports unavailable (not an error)."""
    saved = os.environ.pop("MYCIN_MLX_MODEL", None)
    try:
        assert dc.mlx_backend() is None
    finally:
        if saved is not None:
            os.environ["MYCIN_MLX_MODEL"] = saved


def test_select_backend_raises_when_none_available(monkeypatch=None) -> None:
    """Selection never silently degrades to nothing - it raises with guidance."""
    orig_mlx, orig_oll = dc.mlx_backend, dc.ollama_backend
    dc.mlx_backend = lambda: None
    dc.ollama_backend = lambda: None
    try:
        raised = False
        try:
            dc.select_backend()
        except RuntimeError as e:
            raised = True
            assert "no local decomposer backend" in str(e)
        assert raised, "expected RuntimeError when no backend is available"
    finally:
        dc.mlx_backend, dc.ollama_backend = orig_mlx, orig_oll


def test_load_domains_shape() -> None:
    domains = dc.load_domains()
    assert isinstance(domains, dict) and domains, "dictionary functors -> value domains"
    assert all(isinstance(v, list) for v in domains.values())


def main() -> int:
    test_decompose_text_with_injected_backend()
    test_decompose_tolerates_unparseable_output()
    test_mlx_backend_unavailable_without_config()
    test_select_backend_raises_when_none_available()
    test_load_domains_shape()
    print("test_decomposer: PASS (backend selection + live decompose_text; no model required)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
