"""Parser + typed-AST extraction tests for Twig."""

from __future__ import annotations

import pytest

from twig.ast_extract import extract_program
from twig.ast_nodes import (
    Apply,
    Begin,
    BoolLit,
    Define,
    If,
    IntLit,
    Lambda,
    Let,
    NilLit,
    Program,
    SymLit,
    VarRef,
)
from twig.parser import parse_twig


def _parse(source: str) -> Program:
    return extract_program(parse_twig(source))


# ---------------------------------------------------------------------------
# Atoms
# ---------------------------------------------------------------------------


def test_empty_program() -> None:
    assert _parse("").forms == []


def test_integer_literal() -> None:
    prog = _parse("42")
    assert prog.forms[0] == IntLit(value=42, line=1, column=1)


def test_negative_integer() -> None:
    prog = _parse("-7")
    assert isinstance(prog.forms[0], IntLit)
    assert prog.forms[0].value == -7


def test_boolean_literals() -> None:
    prog = _parse("#t #f")
    assert isinstance(prog.forms[0], BoolLit) and prog.forms[0].value is True
    assert isinstance(prog.forms[1], BoolLit) and prog.forms[1].value is False


def test_nil_literal() -> None:
    prog = _parse("nil")
    assert isinstance(prog.forms[0], NilLit)


def test_var_reference() -> None:
    prog = _parse("foo")
    assert isinstance(prog.forms[0], VarRef) and prog.forms[0].name == "foo"


def test_quoted_symbol_apostrophe() -> None:
    prog = _parse("'hello")
    assert isinstance(prog.forms[0], SymLit) and prog.forms[0].name == "hello"


def test_quoted_symbol_quote_form() -> None:
    prog = _parse("(quote hello)")
    assert isinstance(prog.forms[0], SymLit) and prog.forms[0].name == "hello"


# ---------------------------------------------------------------------------
# Compound forms
# ---------------------------------------------------------------------------


def test_if_form() -> None:
    prog = _parse("(if #t 1 2)")
    assert isinstance(prog.forms[0], If)
    assert isinstance(prog.forms[0].cond, BoolLit)
    assert isinstance(prog.forms[0].then_branch, IntLit)
    assert isinstance(prog.forms[0].else_branch, IntLit)


def test_let_form() -> None:
    prog = _parse("(let ((a 1) (b 2)) (+ a b))")
    let = prog.forms[0]
    assert isinstance(let, Let)
    assert [n for (n, _) in let.bindings] == ["a", "b"]
    assert len(let.body) == 1


def test_let_multiple_body_exprs() -> None:
    prog = _parse("(let ((x 5)) (print x) x)")
    let = prog.forms[0]
    assert isinstance(let, Let)
    assert len(let.body) == 2


def test_begin_form() -> None:
    prog = _parse("(begin 1 2 3)")
    assert isinstance(prog.forms[0], Begin)
    assert len(prog.forms[0].exprs) == 3


def test_lambda_form() -> None:
    prog = _parse("(lambda (x y) (+ x y))")
    lam = prog.forms[0]
    assert isinstance(lam, Lambda)
    assert lam.params == ["x", "y"]


def test_lambda_with_no_params() -> None:
    prog = _parse("(lambda () 42)")
    lam = prog.forms[0]
    assert isinstance(lam, Lambda) and lam.params == []


# ---------------------------------------------------------------------------
# Define
# ---------------------------------------------------------------------------


def test_define_value() -> None:
    prog = _parse("(define x 42)")
    d = prog.forms[0]
    assert isinstance(d, Define)
    assert d.name == "x"
    assert isinstance(d.expr, IntLit)


def test_define_function_sugar() -> None:
    prog = _parse("(define (f x y) (+ x y))")
    d = prog.forms[0]
    assert isinstance(d, Define)
    assert d.name == "f"
    assert isinstance(d.expr, Lambda)
    assert d.expr.params == ["x", "y"]


def test_define_function_multi_body() -> None:
    prog = _parse("(define (f x) (print x) x)")
    d = prog.forms[0]
    assert isinstance(d, Define) and isinstance(d.expr, Lambda)
    assert len(d.expr.body) == 2


# ---------------------------------------------------------------------------
# Application
# ---------------------------------------------------------------------------


def test_apply() -> None:
    prog = _parse("(+ 1 2 3)")
    app = prog.forms[0]
    assert isinstance(app, Apply)
    assert isinstance(app.fn, VarRef) and app.fn.name == "+"
    assert len(app.args) == 3


def test_apply_zero_args() -> None:
    prog = _parse("(thunk)")
    app = prog.forms[0]
    assert isinstance(app, Apply) and app.args == []


def test_apply_higher_order() -> None:
    """``((f x) y)`` — the function position is itself a call."""
    prog = _parse("((f x) y)")
    outer = prog.forms[0]
    assert isinstance(outer, Apply)
    assert isinstance(outer.fn, Apply)


# ---------------------------------------------------------------------------
# Comments
# ---------------------------------------------------------------------------


def test_comments_are_dropped() -> None:
    prog = _parse("; preamble\n42 ; trailing\n")
    assert isinstance(prog.forms[0], IntLit) and prog.forms[0].value == 42
    assert len(prog.forms) == 1


# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------


def test_unmatched_open_paren_raises() -> None:
    with pytest.raises(Exception):  # noqa: B017 — generic GrammarParseError
        _parse("(+ 1 2")


def test_unmatched_close_paren_raises() -> None:
    with pytest.raises(Exception):  # noqa: B017
        _parse("1 2)")


def test_define_value_with_extra_exprs_raises() -> None:
    """``(define x 1 2)`` is ambiguous — neither value nor function form."""
    from twig.errors import TwigParseError

    with pytest.raises(TwigParseError):
        _parse("(define x 1 2)")
