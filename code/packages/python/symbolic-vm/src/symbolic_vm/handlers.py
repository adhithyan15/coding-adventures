"""Shared head handlers.

These functions power both :class:`~symbolic_vm.backends.StrictBackend`
and :class:`~symbolic_vm.backends.SymbolicBackend`. The only difference
is what happens when an operation can't fold numerically:

- Strict handlers raise :class:`TypeError` — strict mode refuses to
  leave an expression half-evaluated.
- Symbolic handlers return the expression as-is — that's the whole
  point of Mathematica-style evaluation.

To avoid two near-identical handler tables, each handler accepts a
``simplify`` flag. When ``True`` the handler applies identity/zero
laws and returns a reduced form; when ``False`` it folds purely
numeric cases and raises on anything symbolic.

Handler signatures all match :class:`~symbolic_vm.backend.Handler`:
``(vm, expr) -> IRNode``.
"""

from __future__ import annotations

import math
from collections.abc import Mapping

from symbolic_ir import (
    ACOS,
    ACOSH,
    ADD,
    AND,
    ASIN,
    ASINH,
    ASSIGN,
    ATAN,
    ATANH,
    COS,
    COSH,
    DEFINE,
    DIV,
    EQUAL,
    EXP,
    GREATER,
    GREATER_EQUAL,
    IF,
    INV,
    LESS,
    LESS_EQUAL,
    LOG,
    MUL,
    NEG,
    NOT,
    NOT_EQUAL,
    OR,
    POW,
    SIN,
    SINH,
    SQRT,
    SUB,
    TAN,
    TANH,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

from symbolic_vm.backend import Handler
from symbolic_vm.numeric import Numeric, from_number, is_one, is_zero, to_number

# ---------------------------------------------------------------------------
# Booleans — stored as the symbols ``True`` / ``False`` to align with the
# MACSYMA compiler's output for the ``true`` / ``false`` keywords.
# ---------------------------------------------------------------------------

TRUE = IRSymbol("True")
FALSE = IRSymbol("False")
ONE = IRInteger(1)
ZERO = IRInteger(0)


def _bool(value: bool) -> IRNode:
    return TRUE if value else FALSE


def _is_truthy(node: IRNode) -> bool | None:
    """Return the Python truthiness of ``node`` if it's a boolean literal."""
    if node == TRUE:
        return True
    if node == FALSE:
        return False
    return None


# ---------------------------------------------------------------------------
# Arithmetic — binary Add/Sub/Mul/Div/Pow and unary Neg/Inv.
# ---------------------------------------------------------------------------


def add(simplify: bool) -> Handler:
    """Build an Add handler. See module docstring for the ``simplify`` flag."""

    def handler(_vm, expr: IRApply) -> IRNode:
        a, b = _binary_args(expr)
        va, vb = to_number(a), to_number(b)
        if va is not None and vb is not None:
            return from_number(va + vb)
        if not simplify:
            raise TypeError(f"Add requires numeric arguments: {expr}")
        # Identity: x + 0 → x, 0 + x → x.
        if is_zero(va):
            return b
        if is_zero(vb):
            return a
        return expr

    return handler


def sub(simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        a, b = _binary_args(expr)
        va, vb = to_number(a), to_number(b)
        if va is not None and vb is not None:
            return from_number(va - vb)
        if not simplify:
            raise TypeError(f"Sub requires numeric arguments: {expr}")
        # x - 0 → x. (0 - x does NOT simplify to -x here; that's a
        # separate algebraic rewrite and keeping it explicit is fine.)
        if is_zero(vb):
            return a
        return expr

    return handler


def mul(simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        a, b = _binary_args(expr)
        va, vb = to_number(a), to_number(b)
        if va is not None and vb is not None:
            return from_number(va * vb)
        if not simplify:
            raise TypeError(f"Mul requires numeric arguments: {expr}")
        # Absorbing zero wins over everything, even an unknown symbol.
        if is_zero(va) or is_zero(vb):
            return ZERO
        if is_one(va):
            return b
        if is_one(vb):
            return a
        return expr

    return handler


def div(simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        a, b = _binary_args(expr)
        va, vb = to_number(a), to_number(b)
        if va is not None and vb is not None:
            if vb == 0:
                raise ZeroDivisionError(f"division by zero: {expr}")
            return from_number(va / vb)
        if not simplify:
            raise TypeError(f"Div requires numeric arguments: {expr}")
        if is_zero(va):
            return ZERO
        if is_one(vb):
            return a
        return expr

    return handler


def pow_(simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        base, exponent = _binary_args(expr)
        vb, ve = to_number(base), to_number(exponent)
        if vb is not None and ve is not None:
            return from_number(_pow_numeric(vb, ve))
        if not simplify:
            raise TypeError(f"Pow requires numeric arguments: {expr}")
        # Standard algebraic identities.
        if is_zero(ve):
            return ONE            # x^0 → 1
        if is_one(ve):
            return base           # x^1 → x
        if is_zero(vb):
            return ZERO           # 0^n → 0 (for n != 0, covered above)
        if is_one(vb):
            return ONE            # 1^n → 1
        return expr

    return handler


def neg(simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        (a,) = expr.args
        va = to_number(a)
        if va is not None:
            return from_number(-va)
        if not simplify:
            raise TypeError(f"Neg requires a numeric argument: {expr}")
        # -(-x) → x.
        if isinstance(a, IRApply) and a.head == NEG and len(a.args) == 1:
            return a.args[0]
        return expr

    return handler


def inv(simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        (a,) = expr.args
        va = to_number(a)
        if va is not None:
            if va == 0:
                raise ZeroDivisionError(f"inverse of zero: {expr}")
            return from_number(1 / va)
        if not simplify:
            raise TypeError(f"Inv requires a numeric argument: {expr}")
        return expr

    return handler


def _pow_numeric(base: Numeric, exponent: Numeric) -> Numeric:
    """Raise ``base`` to ``exponent``, keeping exactness where possible."""
    # Fraction ** integer stays exact. Fraction ** non-integer goes to
    # float via math.pow. Any float involvement poisons the result to float.
    from fractions import Fraction

    if isinstance(base, Fraction) and isinstance(exponent, Fraction):
        if exponent.denominator == 1:
            # ``Fraction ** int`` is supported directly and stays exact.
            return base ** exponent.numerator
        return math.pow(float(base), float(exponent))
    return math.pow(float(base), float(exponent))


def _binary_args(expr: IRApply) -> tuple[IRNode, IRNode]:
    if len(expr.args) != 2:
        raise TypeError(
            f"{_head_name(expr)} expects 2 arguments, got {len(expr.args)}"
        )
    a, b = expr.args
    return a, b


# ---------------------------------------------------------------------------
# Elementary functions — ``Sin``, ``Cos``, ``Exp``, ``Log``, ``Sqrt``.
#
# These only fold when the argument is numeric. In symbolic mode they
# leave symbolic arguments alone (``Sin(x)`` stays ``Sin(x)``). A few
# exact identities — ``Sin(0) = 0``, ``Cos(0) = 1``, ``Exp(0) = 1``,
# ``Log(1) = 0`` — are applied first so the obvious cases don't
# silently become floats.
# ---------------------------------------------------------------------------


def _elementary(
    name: str,
    numeric_fn,
    exact_identities: Mapping[Numeric, IRNode],
    simplify: bool,
) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        (a,) = expr.args
        va = to_number(a)
        if va is not None:
            if va in exact_identities:
                return exact_identities[va]
            return from_number(numeric_fn(float(va)))
        if not simplify:
            raise TypeError(f"{name} requires a numeric argument: {expr}")
        return expr

    return handler


def sin(simplify: bool) -> Handler:
    return _elementary("Sin", math.sin, {0: ZERO}, simplify)


def cos(simplify: bool) -> Handler:
    return _elementary("Cos", math.cos, {0: ONE}, simplify)


def tan(simplify: bool) -> Handler:
    return _elementary("Tan", math.tan, {0: ZERO}, simplify)


def exp(simplify: bool) -> Handler:
    return _elementary("Exp", math.exp, {0: ONE}, simplify)


def log(simplify: bool) -> Handler:
    return _elementary("Log", math.log, {1: ZERO}, simplify)


def sqrt(simplify: bool) -> Handler:
    return _elementary("Sqrt", math.sqrt, {0: ZERO, 1: ONE}, simplify)


def atan(simplify: bool) -> Handler:
    return _elementary("Atan", math.atan, {0: ZERO}, simplify)


def asin(simplify: bool) -> Handler:
    return _elementary("Asin", math.asin, {0: ZERO}, simplify)


def acos(simplify: bool) -> Handler:
    return _elementary("Acos", math.acos, {}, simplify)


def sinh(simplify: bool) -> Handler:
    return _elementary("Sinh", math.sinh, {0: ZERO}, simplify)


def cosh(simplify: bool) -> Handler:
    return _elementary("Cosh", math.cosh, {0: ONE}, simplify)


def tanh(simplify: bool) -> Handler:
    return _elementary("Tanh", math.tanh, {0: ZERO}, simplify)


def asinh(simplify: bool) -> Handler:
    return _elementary("Asinh", math.asinh, {0: ZERO}, simplify)


def acosh(simplify: bool) -> Handler:
    return _elementary("Acosh", math.acosh, {1: ZERO}, simplify)


def atanh(simplify: bool) -> Handler:
    return _elementary("Atanh", math.atanh, {0: ZERO}, simplify)


# ---------------------------------------------------------------------------
# Comparisons
# ---------------------------------------------------------------------------


def _comparison(op, simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        a, b = _binary_args(expr)
        va, vb = to_number(a), to_number(b)
        if va is not None and vb is not None:
            return _bool(op(va, vb))
        # Structural equality catches ``x = x``.
        if op is _eq and a == b:
            return TRUE
        if op is _neq and a == b:
            return FALSE
        if not simplify:
            raise TypeError(f"comparison requires numeric arguments: {expr}")
        return expr

    return handler


def _eq(a: Numeric, b: Numeric) -> bool:
    return a == b


def _neq(a: Numeric, b: Numeric) -> bool:
    return a != b


def _lt(a: Numeric, b: Numeric) -> bool:
    return a < b


def _gt(a: Numeric, b: Numeric) -> bool:
    return a > b


def _leq(a: Numeric, b: Numeric) -> bool:
    return a <= b


def _geq(a: Numeric, b: Numeric) -> bool:
    return a >= b


def equal(simplify: bool) -> Handler:
    return _comparison(_eq, simplify)


def not_equal(simplify: bool) -> Handler:
    return _comparison(_neq, simplify)


def less(simplify: bool) -> Handler:
    return _comparison(_lt, simplify)


def greater(simplify: bool) -> Handler:
    return _comparison(_gt, simplify)


def less_equal(simplify: bool) -> Handler:
    return _comparison(_leq, simplify)


def greater_equal(simplify: bool) -> Handler:
    return _comparison(_geq, simplify)


# ---------------------------------------------------------------------------
# Logic — variadic And/Or and unary Not.
# ---------------------------------------------------------------------------


def and_(_simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        remaining: list[IRNode] = []
        for a in expr.args:
            t = _is_truthy(a)
            if t is False:
                return FALSE            # short-circuit
            if t is None:
                remaining.append(a)
            # True drops out — it's the identity for ``and``.
        if not remaining:
            return TRUE
        if len(remaining) == 1:
            return remaining[0]
        return IRApply(AND, tuple(remaining))

    return handler


def or_(_simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        remaining: list[IRNode] = []
        for a in expr.args:
            t = _is_truthy(a)
            if t is True:
                return TRUE
            if t is None:
                remaining.append(a)
        if not remaining:
            return FALSE
        if len(remaining) == 1:
            return remaining[0]
        return IRApply(OR, tuple(remaining))

    return handler


def not_(_simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        (a,) = expr.args
        t = _is_truthy(a)
        if t is True:
            return FALSE
        if t is False:
            return TRUE
        return expr

    return handler


# ---------------------------------------------------------------------------
# ``If`` — a held head. Evaluate the predicate first; based on the result,
# evaluate one branch. The other branch is thrown away unevaluated.
# ---------------------------------------------------------------------------


def if_(_simplify: bool) -> Handler:
    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) not in (2, 3):
            raise TypeError(
                f"If expects 2 or 3 arguments, got {len(expr.args)}"
            )
        predicate = vm.eval(expr.args[0])
        t = _is_truthy(predicate)
        if t is True:
            return vm.eval(expr.args[1])
        if t is False:
            if len(expr.args) == 3:
                return vm.eval(expr.args[2])
            return FALSE
        # Predicate didn't reduce to a boolean. Leave as-is.
        return IRApply(expr.head, (predicate, *expr.args[1:]))

    return handler


# ---------------------------------------------------------------------------
# Binding — ``Assign`` and ``Define``.
#
# ``Assign(x, rhs)``: eagerly evaluate ``rhs``, bind ``x``, return the value.
# ``Define(name, List(params), body)``: store the whole IRApply under ``name``;
# the VM recognizes that shape when looking up a function call's head.
# ---------------------------------------------------------------------------


def assign(_simplify: bool) -> Handler:
    def handler(vm, expr: IRApply) -> IRNode:
        lhs, rhs = _binary_args(expr)
        if not isinstance(lhs, IRSymbol):
            raise TypeError(f"Assign lhs must be a symbol, got {lhs!r}")
        value = vm.eval(rhs)
        vm.backend.bind(lhs.name, value)
        return value

    return handler


def define(_simplify: bool) -> Handler:
    def handler(vm, expr: IRApply) -> IRNode:
        name, _params, _body = expr.args
        if not isinstance(name, IRSymbol):
            raise TypeError(f"Define name must be a symbol, got {name!r}")
        # Store the entire ``Define(...)`` record under the name, so
        # the VM's function-call path can find and apply it later.
        vm.backend.bind(name.name, expr)
        return name

    return handler


# ---------------------------------------------------------------------------
# ``List`` — a passthrough handler so lists appear as themselves.
# ---------------------------------------------------------------------------


def list_(_simplify: bool) -> Handler:
    def handler(_vm, expr: IRApply) -> IRNode:
        return expr

    return handler


# ---------------------------------------------------------------------------
# Shared builder — construct the handler table for a backend.
# ---------------------------------------------------------------------------


def build_handler_table(simplify: bool) -> dict[str, Handler]:
    """Produce the full handler table for a backend.

    ``simplify=False`` yields a numeric-only evaluator (StrictBackend).
    ``simplify=True`` yields a symbolic evaluator that applies algebraic
    identities and leaves irreducible subexpressions alone.

    Note the deliberate absence of ``D`` here — derivatives live in
    :mod:`symbolic_vm.derivative` and are added by the SymbolicBackend.
    Strict mode doesn't know how to differentiate a symbol (there's
    nothing to differentiate *with respect to* when no free variables
    exist), so StrictBackend simply doesn't install a D handler, and
    the VM's ``on_unknown_head`` fallback raises.
    """
    return {
        ADD.name: add(simplify),
        SUB.name: sub(simplify),
        MUL.name: mul(simplify),
        DIV.name: div(simplify),
        POW.name: pow_(simplify),
        NEG.name: neg(simplify),
        INV.name: inv(simplify),
        SIN.name: sin(simplify),
        COS.name: cos(simplify),
        TAN.name: tan(simplify),
        EXP.name: exp(simplify),
        LOG.name: log(simplify),
        SQRT.name: sqrt(simplify),
        ATAN.name: atan(simplify),
        ASIN.name: asin(simplify),
        ACOS.name: acos(simplify),
        SINH.name: sinh(simplify),
        COSH.name: cosh(simplify),
        TANH.name: tanh(simplify),
        ASINH.name: asinh(simplify),
        ACOSH.name: acosh(simplify),
        ATANH.name: atanh(simplify),
        EQUAL.name: equal(simplify),
        NOT_EQUAL.name: not_equal(simplify),
        LESS.name: less(simplify),
        GREATER.name: greater(simplify),
        LESS_EQUAL.name: less_equal(simplify),
        GREATER_EQUAL.name: greater_equal(simplify),
        AND.name: and_(simplify),
        OR.name: or_(simplify),
        NOT.name: not_(simplify),
        IF.name: if_(simplify),
        ASSIGN.name: assign(simplify),
        DEFINE.name: define(simplify),
        "List": list_(simplify),
    }


def _head_name(expr: IRApply) -> str:
    return expr.head.name if isinstance(expr.head, IRSymbol) else "?"
