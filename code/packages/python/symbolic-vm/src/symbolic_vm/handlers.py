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
    BLOCK,
    COS,
    COSH,
    DEFINE,
    DIV,
    EQUAL,
    EXP,
    FOR_EACH,
    FOR_RANGE,
    GREATER,
    GREATER_EQUAL,
    IF,
    INV,
    LESS,
    LESS_EQUAL,
    LIST,
    LOG,
    MUL,
    NEG,
    NOT,
    NOT_EQUAL,
    OR,
    POW,
    RETURN,
    SIN,
    SINH,
    SQRT,
    SUB,
    TAN,
    TANH,
    WHILE,
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
# Control flow (Phase G — MACSYMA grammar extensions)
#
# Five new heads implement structured programming in the VM:
#
#   While(condition, body)
#   ForRange(var, start, step, end, body)
#   ForEach(var, list, body)
#   Block(locals_list, stmt1, …, stmtN)
#   Return(value)
#
# All of While/ForRange/ForEach/Block are "held heads" — their args are
# NOT pre-evaluated by the VM before dispatch. Each handler manually
# evaluates the parts it needs at the right time (condition before each
# iteration, body on each iteration, etc.).
#
# Return is NOT a held head; the VM evaluates its single argument to
# produce the return value, then calls the handler, which raises
# _ReturnSignal. The control-flow handlers (While/ForRange/ForEach/Block)
# catch _ReturnSignal and return its payload.
# ---------------------------------------------------------------------------


class _ReturnSignal(BaseException):
    """Raised by the ``Return`` handler to unwind a Block/loop early.

    Inherits from ``BaseException`` rather than ``Exception`` so that
    ``except Exception`` clauses in user code (if any) don't swallow it
    accidentally. The VM handlers catch it explicitly.
    """

    def __init__(self, value: IRNode) -> None:
        super().__init__()
        self.value = value


def while_(simplify: bool) -> Handler:  # noqa: ARG001
    """Build a While handler.

    Evaluates ``condition`` before each iteration; exits when it is
    falsy or indeterminate (symbolic).  Evaluates ``body`` on each
    iteration and returns the last body value, or ``False`` if the loop
    never executes (condition was false from the start).

    Both ``condition`` and ``body`` are held — the handler calls
    ``vm.eval`` on them directly so they are re-evaluated each time
    round the loop rather than evaluated once before dispatch.
    """

    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 2:
            raise TypeError(
                f"While expects 2 arguments, got {len(expr.args)}"
            )
        cond_template, body_template = expr.args
        result: IRNode = FALSE
        try:
            while True:
                cond = vm.eval(cond_template)
                t = _is_truthy(cond)
                if t is False:
                    break
                if t is None:
                    # Symbolic condition — cannot determine loop count.
                    # Return the While expression as-is (unevaluated).
                    return expr
                result = vm.eval(body_template)
        except _ReturnSignal as sig:
            return sig.value
        return result

    return handler


def for_range_(simplify: bool) -> Handler:  # noqa: ARG001
    """Build a ForRange handler.

    Implements ``for var: start step step thru end do body``.
    Evaluates ``start``, ``step``, and ``end`` once; iterates while
    ``var <= end`` (for positive step), binding ``var`` to successive
    values.  Restores the previous binding of ``var`` on exit (even if
    a ``Return`` fires).

    All five arguments are held — see :data:`~symbolic_vm.backends._HELD_HEADS`.
    """

    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 5:
            raise TypeError(
                f"ForRange expects 5 arguments, got {len(expr.args)}"
            )
        var_sym, start_tpl, step_tpl, end_tpl, body_tpl = expr.args
        if not isinstance(var_sym, IRSymbol):
            raise TypeError(
                f"ForRange: first argument must be a symbol, got {var_sym!r}"
            )
        start = vm.eval(start_tpl)
        step = vm.eval(step_tpl)
        end = vm.eval(end_tpl)

        v_start = to_number(start)
        v_step = to_number(step)
        v_end = to_number(end)

        if v_start is None or v_step is None or v_end is None:
            # Symbolic bounds — leave the expression unevaluated.
            return expr

        # Save the loop variable's old binding so we can restore it on
        # exit.  If the variable was unbound before the loop, we unbind
        # it again after.
        old = vm.backend.lookup(var_sym.name)
        result: IRNode = FALSE
        try:
            i_val = v_start
            while (v_step > 0 and i_val <= v_end) or (v_step < 0 and i_val >= v_end):
                vm.backend.bind(var_sym.name, from_number(i_val))
                try:
                    result = vm.eval(body_tpl)
                except _ReturnSignal as sig:
                    return sig.value
                i_val += v_step
        finally:
            if old is None:
                vm.backend.unbind(var_sym.name)
            else:
                vm.backend.bind(var_sym.name, old)
        return result

    return handler


def for_each_(simplify: bool) -> Handler:  # noqa: ARG001
    """Build a ForEach handler.

    Implements ``for var in list do body``.  Evaluates the list once,
    then evaluates ``body`` for each element, binding ``var`` in turn.
    Restores ``var``'s previous binding on exit.

    Returns ``False`` if the list is empty (matching MACSYMA semantics).
    """

    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 3:
            raise TypeError(
                f"ForEach expects 3 arguments, got {len(expr.args)}"
            )
        var_sym, list_tpl, body_tpl = expr.args
        if not isinstance(var_sym, IRSymbol):
            raise TypeError(
                f"ForEach: first argument must be a symbol, got {var_sym!r}"
            )
        list_val = vm.eval(list_tpl)
        if not (isinstance(list_val, IRApply) and list_val.head == LIST):
            # Not a concrete list — leave unevaluated.
            return expr

        old = vm.backend.lookup(var_sym.name)
        result: IRNode = FALSE
        try:
            for elem in list_val.args:
                vm.backend.bind(var_sym.name, elem)
                try:
                    result = vm.eval(body_tpl)
                except _ReturnSignal as sig:
                    return sig.value
        finally:
            if old is None:
                vm.backend.unbind(var_sym.name)
            else:
                vm.backend.bind(var_sym.name, old)
        return result

    return handler


def block_(simplify: bool) -> Handler:  # noqa: ARG001
    """Build a Block handler.

    Implements ``block([locals], stmt1, stmt2, …)``.

    The first argument must be an ``IRApply(LIST, …)`` — the locals
    declaration.  Each element of that list is either:

    - ``IRSymbol(name)`` — declare ``name``, initialize to ``False``.
    - ``IRApply(ASSIGN, sym, rhs)`` — declare ``sym``, initialize to
      ``vm.eval(rhs)``.

    Statements (args 1…N) are evaluated in order; the return value is
    the value of the last statement.  A ``Return(value)`` anywhere
    inside the block (including inside nested While/For loops) exits
    immediately and returns ``value``.

    All bindings for local variables are saved on entry and restored on
    exit — even if a ``Return`` or an exception fires.
    """

    def handler(vm, expr: IRApply) -> IRNode:
        if not expr.args:
            return FALSE

        # The first argument is always the locals list (possibly empty).
        # The compiler always prepends ``List()`` so this invariant holds.
        first = expr.args[0]
        if isinstance(first, IRApply) and first.head == LIST:
            locals_node = first
            stmts = expr.args[1:]
        else:
            # Defensive: treat everything as statements, no locals.
            locals_node = IRApply(LIST, ())
            stmts = expr.args

        # Process locals: save old bindings, install new ones.
        saved: dict[str, IRNode | None] = {}
        for local in locals_node.args:
            if isinstance(local, IRSymbol):
                name = local.name
                saved[name] = vm.backend.lookup(name)
                vm.backend.bind(name, FALSE)
            elif (
                isinstance(local, IRApply)
                and local.head == ASSIGN
                and len(local.args) == 2
                and isinstance(local.args[0], IRSymbol)
            ):
                name = local.args[0].name
                saved[name] = vm.backend.lookup(name)
                # Evaluate the initializer in the enclosing scope
                # (before the local binding shadows it).
                vm.backend.bind(name, vm.eval(local.args[1]))
            else:
                raise TypeError(
                    f"Block: invalid local declaration: {local!r}"
                )

        # Execute statements in order, capturing return value.
        result: IRNode = FALSE
        try:
            for stmt in stmts:
                result = vm.eval(stmt)
        except _ReturnSignal as sig:
            result = sig.value
        finally:
            # Restore all saved bindings regardless of how we exited.
            for name, old_val in saved.items():
                if old_val is None:
                    vm.backend.unbind(name)
                else:
                    vm.backend.bind(name, old_val)

        return result

    return handler


def return_(_simplify: bool) -> Handler:
    """Build a Return handler.

    ``Return`` is NOT a held head — the VM evaluates its single argument
    before calling this handler, so ``expr.args[0]`` is already the
    evaluated return value.  The handler raises :class:`_ReturnSignal`
    which unwinds through enclosing While/ForRange/ForEach/Block handlers.
    """

    def handler(_vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 1:
            raise TypeError(
                f"Return expects 1 argument, got {len(expr.args)}"
            )
        raise _ReturnSignal(expr.args[0])

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
        # Control flow (Phase G)
        WHILE.name: while_(simplify),
        FOR_RANGE.name: for_range_(simplify),
        FOR_EACH.name: for_each_(simplify),
        BLOCK.name: block_(simplify),
        RETURN.name: return_(simplify),
    }


def _head_name(expr: IRApply) -> str:
    return expr.head.name if isinstance(expr.head, IRSymbol) else "?"
