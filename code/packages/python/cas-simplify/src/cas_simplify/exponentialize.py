"""Convert trig/hyperbolic functions to exponential form and back.

:func:`exponentialize` — trig/hyp → complex exponential
---------------------------------------------------------

Rewrites standard circular and hyperbolic functions in terms of the
complex exponential ``Exp``.  Applied bottom-up so nested expressions
like ``cos(sin(x))`` are handled correctly.

Transformation table::

    sin(x)  = (exp(i·x) − exp(−i·x)) / (2·i)
    cos(x)  = (exp(i·x) + exp(−i·x)) / 2
    tan(x)  = −i · (exp(i·x) − exp(−i·x)) / (exp(i·x) + exp(−i·x))
    sinh(x) = (exp(x) − exp(−x)) / 2
    cosh(x) = (exp(x) + exp(−x)) / 2
    tanh(x) = (exp(x) − exp(−x)) / (exp(x) + exp(−x))

where ``i = ImaginaryUnit`` (the ``cas_complex`` sentinel value).

:func:`demoivre` — complex exponential → trig
----------------------------------------------

Applies De Moivre's theorem bottom-up::

    exp(b·i)     → cos(b) + i·sin(b)
    exp(a + b·i) → exp(a) · (cos(b) + i·sin(b))

Detection strategy
^^^^^^^^^^^^^^^^^^
The function inspects the argument of ``Exp(...)`` and tries to split
it into a real part ``a`` and an imaginary coefficient ``b`` (the
coefficient of ``ImaginaryUnit``).  Recognised shapes:

- ``ImaginaryUnit`` itself → real=None, imag=1
- ``Mul(ImaginaryUnit, b)`` or ``Mul(b, ImaginaryUnit)`` → real=None, imag=b
- ``Add(a, Mul(ImaginaryUnit, b))`` etc. → real=a, imag=b

If no imaginary part can be found the node is returned unchanged.

Notes
-----
Both functions use ``IRSymbol("ImaginaryUnit")`` directly rather than
importing from ``cas_complex`` to avoid a cross-package dependency.
This is the same symbol that ``cas_complex`` produces; equality between
``IRSymbol`` instances is purely by name.
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    COSH,
    DIV,
    EXP,
    MUL,
    NEG,
    SIN,
    SINH,
    SUB,
    TAN,
    TANH,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

# ImaginaryUnit — same string key as cas_complex; no import needed.
_I = IRSymbol("ImaginaryUnit")
_TWO = IRInteger(2)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def exponentialize(expr: IRNode) -> IRNode:
    """Rewrite trig/hyperbolic functions to exponential form.

    Applies transformations from the table in the module docstring in a
    single bottom-up pass.

    Parameters
    ----------
    expr:
        Symbolic IR expression to transform.

    Returns
    -------
    IRNode
        New expression with trig/hyp nodes replaced by Exp.  Atoms and
        heads not in the transformation table pass through unchanged.
    """
    if not isinstance(expr, IRApply):
        return expr
    new_args = tuple(exponentialize(a) for a in expr.args)
    node: IRApply = (
        IRApply(expr.head, new_args) if new_args != expr.args else expr
    )
    return _exp_node(node)


def demoivre(expr: IRNode) -> IRNode:
    """Apply De Moivre's theorem: exp(a + b·i) → exp(a)·(cos b + i·sin b).

    Applies bottom-up.  Only fires when ``Exp`` argument contains an
    ``ImaginaryUnit`` factor; all other nodes pass through unchanged.

    Parameters
    ----------
    expr:
        Symbolic IR expression to transform.

    Returns
    -------
    IRNode
        New expression with matching ``Exp`` nodes rewritten.
    """
    if not isinstance(expr, IRApply):
        return expr
    new_args = tuple(demoivre(a) for a in expr.args)
    node: IRApply = (
        IRApply(expr.head, new_args) if new_args != expr.args else expr
    )
    return _demoivre_node(node)


# ---------------------------------------------------------------------------
# exponentialize helpers
# ---------------------------------------------------------------------------


def _exp_node(expr: IRApply) -> IRNode:
    """Apply one exponentialize rule to a single node."""
    head = expr.head
    if len(expr.args) != 1:
        return expr
    x = expr.args[0]
    if head == SIN:
        return _sin_exp(x)
    if head == COS:
        return _cos_exp(x)
    if head == TAN:
        return _tan_exp(x)
    if head == SINH:
        return _sinh_exp(x)
    if head == COSH:
        return _cosh_exp(x)
    if head == TANH:
        return _tanh_exp(x)
    return expr


def _ix(x: IRNode) -> IRNode:
    """Build i·x."""
    return IRApply(MUL, (_I, x))


def _neg_ix(x: IRNode) -> IRNode:
    """Build (−i)·x = i·(−x)."""
    return IRApply(MUL, (_I, IRApply(NEG, (x,))))


def _sin_exp(x: IRNode) -> IRNode:
    """sin(x) = (exp(i·x) − exp(−i·x)) / (2·i)."""
    e_pos = IRApply(EXP, (_ix(x),))
    e_neg = IRApply(EXP, (_neg_ix(x),))
    numerator = IRApply(SUB, (e_pos, e_neg))
    denominator = IRApply(MUL, (_TWO, _I))
    return IRApply(DIV, (numerator, denominator))


def _cos_exp(x: IRNode) -> IRNode:
    """cos(x) = (exp(i·x) + exp(−i·x)) / 2."""
    e_pos = IRApply(EXP, (_ix(x),))
    e_neg = IRApply(EXP, (_neg_ix(x),))
    numerator = IRApply(ADD, (e_pos, e_neg))
    return IRApply(DIV, (numerator, _TWO))


def _tan_exp(x: IRNode) -> IRNode:
    """tan(x) = −i · (exp(i·x) − exp(−i·x)) / (exp(i·x) + exp(−i·x))."""
    e_pos = IRApply(EXP, (_ix(x),))
    e_neg = IRApply(EXP, (_neg_ix(x),))
    neg_i = IRApply(NEG, (_I,))
    numerator = IRApply(MUL, (neg_i, IRApply(SUB, (e_pos, e_neg))))
    denominator = IRApply(ADD, (e_pos, e_neg))
    return IRApply(DIV, (numerator, denominator))


def _sinh_exp(x: IRNode) -> IRNode:
    """sinh(x) = (exp(x) − exp(−x)) / 2."""
    e_pos = IRApply(EXP, (x,))
    e_neg = IRApply(EXP, (IRApply(NEG, (x,)),))
    return IRApply(DIV, (IRApply(SUB, (e_pos, e_neg)), _TWO))


def _cosh_exp(x: IRNode) -> IRNode:
    """cosh(x) = (exp(x) + exp(−x)) / 2."""
    e_pos = IRApply(EXP, (x,))
    e_neg = IRApply(EXP, (IRApply(NEG, (x,)),))
    return IRApply(DIV, (IRApply(ADD, (e_pos, e_neg)), _TWO))


def _tanh_exp(x: IRNode) -> IRNode:
    """tanh(x) = (exp(x) − exp(−x)) / (exp(x) + exp(−x))."""
    e_pos = IRApply(EXP, (x,))
    e_neg = IRApply(EXP, (IRApply(NEG, (x,)),))
    return IRApply(DIV, (IRApply(SUB, (e_pos, e_neg)), IRApply(ADD, (e_pos, e_neg))))


# ---------------------------------------------------------------------------
# demoivre helpers
# ---------------------------------------------------------------------------


def _demoivre_node(expr: IRApply) -> IRNode:
    """Apply De Moivre's theorem to a single Exp node."""
    if expr.head != EXP or len(expr.args) != 1:
        return expr

    real, imag = _split_real_imag(expr.args[0])
    if imag is None:
        # No imaginary component — leave unchanged.
        return expr

    # Build cos(b) + i·sin(b).
    cos_part: IRNode = IRApply(COS, (imag,))
    sin_part: IRNode = IRApply(SIN, (imag,))
    i_sin: IRNode = IRApply(MUL, (_I, sin_part))
    trig_sum: IRNode = IRApply(ADD, (cos_part, i_sin))

    if real is None:
        # Pure imaginary: exp(i·b) → cos(b) + i·sin(b).
        return trig_sum

    # Mixed: exp(a + i·b) → exp(a) · (cos(b) + i·sin(b)).
    return IRApply(MUL, (IRApply(EXP, (real,)), trig_sum))


def _split_real_imag(arg: IRNode) -> tuple[IRNode | None, IRNode | None]:
    """Split an expression into (real_part, imaginary_coefficient).

    Returns ``(real, imag)`` where ``imag`` is the coefficient of
    ``ImaginaryUnit``.  Either component may be ``None`` (absent).

    Recognised patterns::

        ImaginaryUnit           → (None, 1)
        Mul(i, b)               → (None, b)
        Mul(b, i)               → (None, b)
        Add(a, Mul(i, b), …)    → (a, b)
        Add(Mul(i, b), a, …)    → (a, b)

    If no imaginary component is found, returns ``(arg, None)``.
    """
    # Pattern: bare ImaginaryUnit.
    if arg == _I:
        return (None, IRInteger(1))

    # Pattern: Mul containing exactly one ImaginaryUnit.
    if isinstance(arg, IRApply) and arg.head == MUL:
        coeff = _extract_i_from_mul(arg)
        if coeff is not None:
            # Entire arg is i * coeff.
            return (None, coeff)

    # Pattern: Add containing one or more imaginary terms.
    if isinstance(arg, IRApply) and arg.head == ADD:
        real_terms: list[IRNode] = []
        imag_coeff: IRNode | None = None

        for term in arg.args:
            if term == _I:
                if imag_coeff is None:
                    imag_coeff = IRInteger(1)
                else:
                    # More than one bare i — can't decompose cleanly.
                    return (arg, None)
            else:
                i_part = _extract_i_from_term(term)
                if i_part is not None:
                    if imag_coeff is None:
                        imag_coeff = i_part
                    else:
                        # Two imaginary terms — ambiguous; bail out.
                        return (arg, None)
                else:
                    real_terms.append(term)

        if imag_coeff is None:
            return (arg, None)  # No imaginary component found.

        if not real_terms:
            return (None, imag_coeff)

        real: IRNode = (
            real_terms[0]
            if len(real_terms) == 1
            else IRApply(ADD, tuple(real_terms))
        )
        return (real, imag_coeff)

    return (arg, None)


def _extract_i_from_mul(mul_node: IRApply) -> IRNode | None:
    """If mul_node = Mul(i, b, …), return b (the non-i product), else None."""
    args = mul_node.args
    i_positions = [j for j, a in enumerate(args) if a == _I]
    if len(i_positions) != 1:
        return None
    i_idx = i_positions[0]
    rest = [a for j, a in enumerate(args) if j != i_idx]
    if not rest:
        return IRInteger(1)
    if len(rest) == 1:
        return rest[0]
    return IRApply(MUL, tuple(rest))


def _extract_i_from_term(term: IRNode) -> IRNode | None:
    """Return the non-i part of a term that contains ImaginaryUnit, or None."""
    if term == _I:
        return IRInteger(1)
    if isinstance(term, IRApply) and term.head == MUL:
        return _extract_i_from_mul(term)
    return None
