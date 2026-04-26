"""Blank — anonymous wildcard, with and without head constraint."""

from __future__ import annotations

from symbolic_ir import ADD, IRApply, IRInteger, IRSymbol

from cas_pattern_matching import Blank, match


def test_blank_matches_anything_integer() -> None:
    assert match(Blank(), IRInteger(42)) is not None


def test_blank_matches_anything_symbol() -> None:
    assert match(Blank(), IRSymbol("x")) is not None


def test_blank_matches_anything_compound() -> None:
    expr = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    assert match(Blank(), expr) is not None


def test_blank_with_integer_head_matches_int() -> None:
    assert match(Blank("Integer"), IRInteger(5)) is not None


def test_blank_with_integer_head_rejects_symbol() -> None:
    assert match(Blank("Integer"), IRSymbol("x")) is None


def test_blank_with_add_head_matches_add() -> None:
    expr = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    assert match(Blank("Add"), expr) is not None


def test_blank_with_add_head_rejects_symbol() -> None:
    assert match(Blank("Add"), IRSymbol("x")) is None
