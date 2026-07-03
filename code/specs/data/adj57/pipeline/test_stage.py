#!/usr/bin/env python3
"""Tests for the universal stage contract (stage.py). Run: python test_stage.py"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import stage  # noqa: E402


def test_text_full_partition_is_clean():
    cov = stage.gate_text("decompose", "HR 120. Glaucoma.", [
        {"text": "HR 120.", "kind": "used", "produced": "heart_rate(120)"},
        {"text": " ", "kind": "discard", "reason": "whitespace"},
        {"text": "Glaucoma.", "kind": "used", "produced": "glaucoma(present)"}])
    assert cov.covered and cov.clean
    assert cov.n_used == 2 and cov.n_discard == 1


def test_text_lossy_partition_is_caught():
    cov = stage.gate_text("decompose", "HR 120. Glaucoma.", [
        {"text": "HR 120.", "kind": "used", "produced": "heart_rate(120)"}])  # drops " Glaucoma."
    assert not cov.covered
    assert cov.detail["first_divergence"] == 7


def test_text_used_without_citation_is_unclean():
    cov = stage.gate_text("decompose", "X", [{"text": "X", "kind": "used"}])  # no produced
    assert cov.covered and not cov.clean


def test_elements_silent_drop_is_caught():
    cov = stage.gate_elements("derive", ["a", "b"], used=[{"id": "a", "produced": "link"}], discards=[])
    assert not cov.covered
    assert cov.detail["missing"] == ["b"]


def test_elements_full_accounting_is_clean():
    cov = stage.gate_elements("derive", ["a", "b"],
                              used=[{"id": "a", "produced": "link"}],
                              discards=[{"id": "b", "reason": "unrelated comorbidity"}])
    assert cov.covered and cov.clean


def test_elements_discard_without_reason_is_unclean():
    cov = stage.gate_elements("derive", ["a"], used=[], discards=[{"id": "a"}])  # no reason
    assert cov.covered and not cov.clean


def test_partition_by_used_fills_gaps():
    text = "Background. Sensitivity was 0.92 here. End."
    segs = stage.partition_text_by_used(text, [{"text": "Sensitivity was 0.92", "produced": "LR"}], "context")
    assert "".join(s["text"] for s in segs) == text  # reconstructs exactly
    assert sum(s["kind"] == "used" for s in segs) == 1
    assert sum(s["kind"] == "discard" for s in segs) == 2


def test_partition_by_used_flags_nonverbatim_quote():
    segs = stage.partition_text_by_used("real text", [{"text": "NOT PRESENT", "produced": "x"}], "ctx")
    assert any("BROKEN" in s.get("reason", "") for s in segs)


def test_trail_ok_only_when_all_clean():
    t = stage.Trail()
    t.record(stage.gate_text("a", "X", [{"text": "X", "kind": "used", "produced": "p"}]))
    assert t.ok()
    t.record(stage.gate_elements("b", ["x"], used=[], discards=[]))  # hole
    assert not t.ok()
    assert any("b:" in h for h in t.holes())


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print(f"  ok  {fn.__name__}")
    print(f"\n{len(fns)} tests passed.")
