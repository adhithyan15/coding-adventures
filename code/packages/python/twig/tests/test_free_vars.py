"""Free-variable analysis tests."""

from __future__ import annotations

from twig.ast_extract import extract_program
from twig.ast_nodes import Define, Lambda
from twig.free_vars import free_vars
from twig.parser import parse_twig


def _first_lambda(source: str) -> Lambda:
    """Extract the first ``Lambda`` AST node found by walking the program.

    Used by tests to grab the inner lambda inside ``(define (f ...) ...)``
    or similar wrappings.
    """
    prog = extract_program(parse_twig(source))
    for form in prog.forms:
        if isinstance(form, Lambda):
            return form
        if isinstance(form, Define) and isinstance(form.expr, Lambda):
            # The function body is a lambda; the *inner* lambda we want
            # is whatever lambda the body returns/contains.  Walk it.
            return _walk_for_lambda(form.expr)
    raise AssertionError("no lambda in program")


def _walk_for_lambda(lam: Lambda) -> Lambda:
    """Find the first nested lambda inside ``lam``'s body, or return
    ``lam`` if it has no nested lambdas (so simple lambdas can still
    be retrieved)."""
    # Look one level deep; tests use simple shapes.
    for expr in lam.body:
        if isinstance(expr, Lambda):
            return expr
    return lam


def test_lambda_with_no_free_vars() -> None:
    lam = _first_lambda("(lambda (x) (+ x 1))")
    assert free_vars(lam, globals_={"+"}) == []


def test_lambda_captures_outer_param() -> None:
    """``(define (adder n) (lambda (x) (+ x n)))`` — the inner
    lambda captures ``n``."""
    lam = _first_lambda("(define (adder n) (lambda (x) (+ x n)))")
    assert free_vars(lam, globals_={"+"}) == ["n"]


def test_globals_are_not_captured() -> None:
    """References to top-level defines and builtins do not become
    captures."""
    lam = _first_lambda("(define (mkthunk) (lambda () (+ x 1)))")
    captures = free_vars(lam, globals_={"+", "x"})
    assert captures == []


def test_let_bound_names_dont_escape() -> None:
    src = "(define (f) (lambda () (let ((y 1)) y)))"
    lam = _first_lambda(src)
    assert free_vars(lam, globals_=set()) == []


def test_let_rhs_in_outer_scope() -> None:
    """``(let ((y x)) y)`` — ``x`` in the RHS is in outer scope, so
    the enclosing lambda must capture it."""
    src = "(define (mk) (lambda () (let ((y x)) y)))"
    lam = _first_lambda(src)
    captures = free_vars(lam, globals_=set())
    assert captures == ["x"]


def test_nested_lambdas_bubble_captures_up() -> None:
    """An inner lambda's free variable must also be free in the
    *outer* lambda when it isn't bound there either."""
    src = """
    (define (mk)
      (lambda (a)
        (lambda (b) (+ a b zzz))))
    """
    # Grab the outer lambda inside ``mk``'s define.
    prog = extract_program(parse_twig(src))
    outer_def = prog.forms[0]
    assert isinstance(outer_def, Define)
    outer_lam = outer_def.expr  # the (lambda (a) ...) directly
    assert isinstance(outer_lam, Lambda)
    captures = free_vars(outer_lam, globals_={"+"})
    # ``a`` is a param of outer_lam, ``b`` is a param of inner.
    # ``zzz`` is unbound everywhere → bubbles up.
    assert captures == ["zzz"]


def test_capture_order_is_stable() -> None:
    """Captures preserve first-encounter order — important because
    the compiler emits them as the leading parameters of the
    gensym'd IIR function.  Unstable order would change the
    function signature run-to-run."""
    src = "(define (mk) (lambda () (+ a b c a b)))"
    lam = _first_lambda(src)
    assert free_vars(lam, globals_={"+"}) == ["a", "b", "c"]


def test_if_branches_walked() -> None:
    """``If`` is handled — free vars in cond / then / else all bubble up."""
    src = "(define (mk) (lambda () (if a b c)))"
    lam = _first_lambda(src)
    assert free_vars(lam, globals_=set()) == ["a", "b", "c"]


def test_begin_walks_every_expr() -> None:
    src = "(define (mk) (lambda () (begin a b c)))"
    lam = _first_lambda(src)
    assert free_vars(lam, globals_=set()) == ["a", "b", "c"]


def test_let_body_uses_extended_scope() -> None:
    """Names introduced by ``let`` are bound in the body, so they
    don't escape as captures."""
    src = "(define (mk) (lambda () (let ((y 5)) (+ y z))))"
    lam = _first_lambda(src)
    assert free_vars(lam, globals_={"+"}) == ["z"]


def test_literals_have_no_free_vars() -> None:
    src = "(define (mk) (lambda () 42))"
    lam = _first_lambda(src)
    assert free_vars(lam, globals_=set()) == []


def test_unhandled_node_type_raises() -> None:
    """Defensive: ``_walk`` rejects an AST shape it doesn't know."""
    import pytest

    from twig.free_vars import _walk

    with pytest.raises(TypeError):
        _walk("not-an-expr", set(), set(), {})
