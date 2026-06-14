"""Laplace transform computation: laplace_transform(f, t, s) → IRNode.

This module implements the top-level Laplace transform algorithm:

1. **Linearity (Add)**: Break sums apart.
   L{f(t) + g(t)} = L{f(t)} + L{g(t)}

2. **Linearity (constant multiple in Mul)**: Pull out constants.
   L{c · f(t)} = c · L{f(t)},  where c does not depend on t.

3. **Table lookup**: Consult the transform table in ``table.py`` to
   find a closed-form result for the reduced expression.

4. **Fall-through**: If no pattern matches, return the unevaluated IR
   node ``Laplace(f, t, s)``. This is the standard CAS behaviour — it is
   better to return an honest "I don't know" than a wrong answer.

Design note on "unevaluated" expressions
-----------------------------------------
In every CAS (MACSYMA, Mathematica, Maple), when an operation cannot be
completed symbolically, the system returns the *unevaluated* form of the
expression — e.g. ``laplace(unknown_fn(t), t, s)`` remains as-is rather
than crashing. This is what makes CAS systems composable: partial results
can be simplified further as more information becomes available.
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    MUL,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

from cas_laplace.heads import LAPLACE
from cas_laplace.table import _extract_coeff_and_fn, table_lookup


def laplace_transform(
    f_ir: IRNode,
    t_sym: IRSymbol,
    s_sym: IRSymbol,
) -> IRNode:
    """Compute the Laplace transform of ``f_ir`` with respect to ``t_sym``.

    Returns the transform as an IR expression in ``s_sym``, or the
    unevaluated ``Laplace(f, t, s)`` if no pattern matches.

    Algorithm
    ---------
    1. If ``f_ir`` is an ``Add(a, b)``, apply linearity recursively:
       L{a + b} = L{a} + L{b}.
    2. If ``f_ir`` is a ``Mul(c, g)`` where ``c`` is constant w.r.t. ``t``,
       apply linearity: L{c·g} = c · L{g}.
    3. Consult the transform table.
    4. Fall through to the unevaluated form.

    Parameters
    ----------
    f_ir:
        The IR expression for f(t) — the function to transform.
    t_sym:
        The time variable, e.g. ``IRSymbol("t")``.
    s_sym:
        The complex frequency variable, e.g. ``IRSymbol("s")``.

    Returns
    -------
    IRNode
        The transform F(s), or ``IRApply(LAPLACE, (f_ir, t_sym, s_sym))``
        if the transform is not in the table.

    Examples
    --------
    The constant function f(t) = 1 transforms to 1/s::

        laplace_transform(IRInteger(1), t, s) == Div(1, s)

    A sum is decomposed by linearity::

        laplace_transform(Add(sin(t), cos(t)), t, s)
        == Add(laplace(sin(t)), laplace(cos(t)))
    """
    # ------------------------------------------------------------------
    # Step 1: Linearity over addition.
    # L{f + g} = L{f} + L{g}
    # ------------------------------------------------------------------
    if (
        isinstance(f_ir, IRApply)
        and isinstance(f_ir.head, IRSymbol)
        and f_ir.head.name == "Add"
        and len(f_ir.args) == 2
    ):
        lf = laplace_transform(f_ir.args[0], t_sym, s_sym)
        lg = laplace_transform(f_ir.args[1], t_sym, s_sym)
        return IRApply(ADD, (lf, lg))

    # ------------------------------------------------------------------
    # Step 2: Linearity over constant multiplication.
    # L{c · f} = c · L{f},  where c is constant w.r.t. t.
    # ------------------------------------------------------------------
    if (
        isinstance(f_ir, IRApply)
        and isinstance(f_ir.head, IRSymbol)
        and f_ir.head.name == "Mul"
        and len(f_ir.args) == 2
    ):
        coeff, fn = _extract_coeff_and_fn(f_ir, t_sym)
        # Only apply linearity if we actually found a constant factor
        # (i.e. coeff is not just IRInteger(1) from "no extraction").
        if not (isinstance(coeff, IRInteger) and coeff.value == 1):
            lf = laplace_transform(fn, t_sym, s_sym)
            return IRApply(MUL, (coeff, lf))

    # ------------------------------------------------------------------
    # Step 3: Table lookup.
    # ------------------------------------------------------------------
    result = table_lookup(f_ir, t_sym, s_sym)
    if result is not None:
        return result

    # ------------------------------------------------------------------
    # Step 4: Fall-through — return unevaluated Laplace(f, t, s).
    # ------------------------------------------------------------------
    return IRApply(LAPLACE, (f_ir, t_sym, s_sym))
