"""Free-variable analysis for Twig lambdas.

When a ``lambda`` appears inside a function body it may reference
names that are *neither* parameters of the lambda *nor* top-level
``define``-bound globals.  Those names need to be captured at
closure-construction time and stashed inside the closure object so
they're available when the closure is later applied.

This module computes that set.  ``free_vars(lam, ...)`` returns the
ordered list of names — order is significant because the compiler
emits them as the leading parameters of the gensym'd IIR function
the lambda compiles to, and as the leading arguments to
``call_builtin "make_closure"`` at the lambda site.

Algorithm
---------
A standard set-based traversal:

* ``names_used(expr)`` — the set of names referenced anywhere in
  ``expr``, ignoring ``define``s (top-level defines never appear
  inside expressions in TW00).
* ``free_vars(lam, globals)`` — start from ``names_used(body)``,
  subtract the lambda's own ``params``, subtract ``globals``
  (top-level names visible everywhere), subtract names introduced
  by inner ``let`` bindings or nested lambda parameters before they
  are read.

The traversal is purely structural — no environment threading
needed beyond a single "currently-bound" set passed down recursive
calls.  Output is a deterministic list (insertion order via a
``dict`` so closures get stable shapes across runs).
"""

from __future__ import annotations

from twig.ast_nodes import (
    Apply,
    Begin,
    BoolLit,
    Expr,
    If,
    IntLit,
    Lambda,
    Let,
    NilLit,
    SymLit,
    VarRef,
)


def free_vars(lam: Lambda, globals_: set[str]) -> list[str]:
    """Return the free variables of ``lam``, in stable order.

    ``globals_`` is the set of top-level ``define``-bound names; any
    reference to one of these does *not* count as free (they're
    looked up by name at apply time, not captured).
    """
    bound: set[str] = set(lam.params)
    found: dict[str, None] = {}
    for expr in lam.body:
        _walk(expr, bound, globals_, found)
    return list(found.keys())


def _walk(
    expr: Expr,
    bound: set[str],
    globals_: set[str],
    found: dict[str, None],
) -> None:
    """Recurse into ``expr``, adding any free name into ``found``.

    ``bound`` is the set of names already bound at this point —
    parameters of any enclosing lambda plus any ``let`` bindings
    we've descended into.  ``globals_`` is the immutable top-level
    set.  ``found`` is the result accumulator (a dict to preserve
    insertion order without allocating a list per recursive call).
    """
    if isinstance(expr, VarRef):
        # Module-qualified names (e.g. ``host/write-byte``) are
        # resolved at the call site against the module's export
        # table — they are *never* closure-captured.  Short-circuit
        # before the bound/globals check so they never appear in the
        # free-variable list.
        #
        # We check that the slash is *internal* (not bare ``"/"`` —
        # the arithmetic division operator — and not a leading/trailing
        # slash), i.e. ``idx > 0 and idx < len - 1``.
        _slash = expr.name.find("/")
        if _slash > 0 and _slash < len(expr.name) - 1:
            return
        # A reference is "free" iff it's neither bound here nor a
        # global.  Builtins (``+``, ``cons``, …) live alongside
        # user globals in ``globals_`` — the wrapper passes them
        # in.  This avoids special-casing every builtin name here.
        if expr.name not in bound and expr.name not in globals_:
            found[expr.name] = None
        return

    if isinstance(expr, IntLit | BoolLit | NilLit | SymLit):
        return  # literals contain no references

    if isinstance(expr, If):
        _walk(expr.cond, bound, globals_, found)
        _walk(expr.then_branch, bound, globals_, found)
        _walk(expr.else_branch, bound, globals_, found)
        return

    if isinstance(expr, Begin):
        for e in expr.exprs:
            _walk(e, bound, globals_, found)
        return

    if isinstance(expr, Let):
        # Scheme-style ``let``: bindings see the *outer* scope.  The
        # body sees ``bound ∪ {names from bindings}``.  We handle
        # that by walking each binding RHS in the outer scope, then
        # extending ``bound`` for the body.
        for _name, rhs in expr.bindings:
            _walk(rhs, bound, globals_, found)
        body_bound = bound | {n for (n, _) in expr.bindings}
        for e in expr.body:
            _walk(e, body_bound, globals_, found)
        return

    if isinstance(expr, Lambda):
        # An inner lambda introduces its own parameters as bound
        # names for its body.  Free variables of the *inner* body
        # that aren't params of either lambda *and* aren't globals
        # become free variables of the *outer* lambda too — they
        # bubble up.
        inner_bound = bound | set(expr.params)
        for e in expr.body:
            _walk(e, inner_bound, globals_, found)
        return

    if isinstance(expr, Apply):
        _walk(expr.fn, bound, globals_, found)
        for arg in expr.args:
            _walk(arg, bound, globals_, found)
        return

    raise TypeError(f"unhandled expression type: {type(expr).__name__}")
