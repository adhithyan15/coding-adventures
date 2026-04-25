"""Literal-equality matching."""

from __future__ import annotations

from symbolic_ir import ADD, IRApply, IRInteger, IRSymbol

from cas_pattern_matching import match


def test_integer_matches_itself() -> None:
    assert match(IRInteger(5), IRInteger(5)) is not None


def test_integer_does_not_match_different_integer() -> None:
    assert match(IRInteger(5), IRInteger(6)) is None


def test_symbol_matches_itself() -> None:
    assert match(IRSymbol("x"), IRSymbol("x")) is not None


def test_compound_matches_compound() -> None:
    a = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    b = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    assert match(a, b) is not None


def test_compound_different_head_no_match() -> None:
    from symbolic_ir import MUL

    a = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    b = IRApply(MUL, (IRSymbol("x"), IRInteger(1)))
    assert match(a, b) is None


def test_compound_different_args_no_match() -> None:
    a = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
    b = IRApply(ADD, (IRSymbol("x"), IRInteger(2)))
    assert match(a, b) is None
