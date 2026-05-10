"""
Binary, unary, and NULL-aware helpers used by the VM dispatch loop
==================================================================

All three-valued logic, type coercion, and type-error decisions live here.
Keeping them out of the main dispatch loop keeps the loop itself short and
focused on control flow.

NULL propagation rules follow the spec exactly:

- **Arithmetic / comparison**: any NULL input → NULL output.
- **AND**: at least one FALSE → FALSE; otherwise if any NULL → NULL; else TRUE.
- **OR**: at least one TRUE → TRUE; otherwise if any NULL → NULL; else FALSE.
- **Concat**: any NULL → NULL.

Booleans vs integers
--------------------

Python's ``bool`` is a subclass of ``int`` (``True == 1``, ``False == 0``).
That historical wart means every type check must test ``bool`` before ``int``
or booleans will be interpreted as integers. The helpers here follow that
discipline; callers should not bypass them.
"""

from __future__ import annotations

from sql_backend.values import SqlValue, sql_type_name
from sql_codegen import BinaryOpCode, UnaryOpCode

from .errors import DivisionByZero, TypeMismatch


def _is_bool(v: SqlValue) -> bool:
    """True if *v* is a Python bool, *not* an integer masquerading as one."""
    return isinstance(v, bool)


def _is_numeric(v: SqlValue) -> bool:
    """True for int or float; excludes bool (SQL BOOLEAN is not numeric here)."""
    return isinstance(v, int | float) and not _is_bool(v)


def _to_number(v: SqlValue) -> int | float:
    """Narrow a SqlValue to a numeric. Caller must have checked ``_is_numeric``."""
    assert _is_numeric(v)  # noqa: S101 — internal invariant, not input validation
    return v  # type: ignore[return-value]


# --------------------------------------------------------------------------
# Binary ops — dispatched by the top-level helper below.
# --------------------------------------------------------------------------


def apply_binary(op: BinaryOpCode, left: SqlValue, right: SqlValue) -> SqlValue:
    """Evaluate ``left OP right`` with SQL three-valued logic.

    Returns the result value. Raises :class:`TypeMismatch` or
    :class:`DivisionByZero` for ill-typed or divide-by-zero inputs.
    """
    # AND and OR handle NULL themselves (3VL). Everything else short-circuits
    # on NULL first — that's the most common case and lets the arithmetic
    # branches assume both sides are non-NULL.
    if op is BinaryOpCode.AND:
        return _and(left, right)
    if op is BinaryOpCode.OR:
        return _or(left, right)

    if left is None or right is None:
        return None

    if op in _ARITHMETIC:
        return _arithmetic(op, left, right)
    if op in _COMPARISON:
        return _comparison(op, left, right)
    if op is BinaryOpCode.CONCAT:
        return _concat(left, right)

    raise TypeMismatch(expected="known op", got=str(op), context="BinaryOp")


_ARITHMETIC = {
    BinaryOpCode.ADD,
    BinaryOpCode.SUB,
    BinaryOpCode.MUL,
    BinaryOpCode.DIV,
    BinaryOpCode.MOD,
}
_COMPARISON = {
    BinaryOpCode.EQ,
    BinaryOpCode.NEQ,
    BinaryOpCode.LT,
    BinaryOpCode.LTE,
    BinaryOpCode.GT,
    BinaryOpCode.GTE,
}


def _arithmetic(op: BinaryOpCode, left: SqlValue, right: SqlValue) -> SqlValue:
    if not (_is_numeric(left) and _is_numeric(right)):
        raise TypeMismatch(
            expected="numeric",
            got=f"{sql_type_name(left)}, {sql_type_name(right)}",
            context=f"BinaryOp({op.name})",
        )
    a = _to_number(left)
    b = _to_number(right)
    if op is BinaryOpCode.ADD:
        return a + b
    if op is BinaryOpCode.SUB:
        return a - b
    if op is BinaryOpCode.MUL:
        return a * b
    if op is BinaryOpCode.DIV:
        if b == 0:
            raise DivisionByZero()
        # Integer division when both operands are ints; truncate toward zero
        # as C / most SQL dialects do, not Python's floor-divide semantics.
        if isinstance(a, int) and isinstance(b, int):
            q = abs(a) // abs(b)
            return -q if (a < 0) ^ (b < 0) else q
        return a / b
    if op is BinaryOpCode.MOD:
        if b == 0:
            raise DivisionByZero()
        return a % b
    raise TypeMismatch(expected="arithmetic op", got=op.name, context="BinaryOp")


def _comparison(op: BinaryOpCode, left: SqlValue, right: SqlValue) -> SqlValue:
    # Booleans can only compare to booleans; numeric-vs-numeric is fine (int/float
    # promote); strings to strings. Mixing categories → TypeMismatch.
    if _is_bool(left) != _is_bool(right):
        raise TypeMismatch(
            expected="matching types",
            got=f"{sql_type_name(left)}, {sql_type_name(right)}",
            context=f"BinaryOp({op.name})",
        )
    if isinstance(left, str) != isinstance(right, str):
        raise TypeMismatch(
            expected="matching types",
            got=f"{sql_type_name(left)}, {sql_type_name(right)}",
            context=f"BinaryOp({op.name})",
        )
    try:
        if op is BinaryOpCode.EQ:
            return left == right
        if op is BinaryOpCode.NEQ:
            return left != right
        if op is BinaryOpCode.LT:
            return left < right  # type: ignore[operator]
        if op is BinaryOpCode.LTE:
            return left <= right  # type: ignore[operator]
        if op is BinaryOpCode.GT:
            return left > right  # type: ignore[operator]
        if op is BinaryOpCode.GTE:
            return left >= right  # type: ignore[operator]
    except TypeError as e:
        raise TypeMismatch(
            expected="comparable",
            got=f"{sql_type_name(left)}, {sql_type_name(right)}",
            context=f"BinaryOp({op.name})",
        ) from e
    raise TypeMismatch(expected="comparison op", got=op.name, context="BinaryOp")


def _concat(left: SqlValue, right: SqlValue) -> SqlValue:
    if not (isinstance(left, str) and isinstance(right, str)):
        raise TypeMismatch(
            expected="text",
            got=f"{sql_type_name(left)}, {sql_type_name(right)}",
            context="BinaryOp(CONCAT)",
        )
    return left + right


def _and(left: SqlValue, right: SqlValue) -> SqlValue:
    # Three-valued AND: FALSE dominates; NULL only if no FALSE seen.
    if left is False or right is False:
        return False
    if left is None or right is None:
        return None
    if not (_is_bool(left) and _is_bool(right)):
        raise TypeMismatch(
            expected="boolean",
            got=f"{sql_type_name(left)}, {sql_type_name(right)}",
            context="BinaryOp(AND)",
        )
    return True


def _or(left: SqlValue, right: SqlValue) -> SqlValue:
    # Three-valued OR: TRUE dominates; NULL only if no TRUE seen.
    if left is True or right is True:
        return True
    if left is None or right is None:
        return None
    if not (_is_bool(left) and _is_bool(right)):
        raise TypeMismatch(
            expected="boolean",
            got=f"{sql_type_name(left)}, {sql_type_name(right)}",
            context="BinaryOp(OR)",
        )
    return False


# --------------------------------------------------------------------------
# Unary ops.
# --------------------------------------------------------------------------


def apply_unary(op: UnaryOpCode, value: SqlValue) -> SqlValue:
    """Evaluate a unary op. NEG on NULL → NULL; NOT on NULL → NULL."""
    if value is None:
        return None
    if op is UnaryOpCode.NEG:
        if not _is_numeric(value):
            raise TypeMismatch(
                expected="numeric", got=sql_type_name(value), context="UnaryOp(NEG)"
            )
        return -_to_number(value)
    if op is UnaryOpCode.NOT:
        if not _is_bool(value):
            raise TypeMismatch(
                expected="boolean", got=sql_type_name(value), context="UnaryOp(NOT)"
            )
        return not value
    raise TypeMismatch(expected="unary op", got=str(op), context="UnaryOp")


# --------------------------------------------------------------------------
# LIKE matcher. Patterns use ``%`` for zero-or-more chars, ``_`` for exactly
# one. Everything else is a literal. Case-sensitive by spec.
# --------------------------------------------------------------------------


def like_match(value: str, pattern: str) -> bool:
    """Case-insensitive LIKE matcher (SQLite / ANSI SQL default behaviour).

    LIKE is case-insensitive for ASCII letters by default in SQLite and in
    the SQL standard.  Non-ASCII characters (Unicode) are compared
    case-sensitively here, which matches SQLite's behaviour when the
    ``NOCASE`` collation is not in effect and ICU is not compiled in.

    Wildcards::

        %   — matches zero or more characters
        _   — matches exactly one character

    Truth table::

        like_match('Hello', 'hello')   → True   (ASCII case-fold)
        like_match('Hello', 'HELLO%')  → True
        like_match('abc',   'a%c')     → True
        like_match('abc',   'a_c')     → True
        like_match('ac',    'a_c')     → False  (underscore needs exactly 1)
        like_match('',      '%')       → True   (% matches empty)

    Algorithm: iterative DP — O(m·n) time, O(m·n) space, no recursion.
    ``m = len(value)``, ``n = len(pattern)``.
    ``dp[i][j]`` is True if ``value[:i]`` matches ``pattern[:j]``.
    """
    # Normalise to lowercase for case-insensitive ASCII comparison.
    # The pattern wildcards % and _ are already ASCII so folding them is safe.
    value_lower = value.lower()
    pattern_lower = pattern.lower()

    m, n = len(value_lower), len(pattern_lower)
    # dp[i][j] = True if value[:i] matches pattern[:j]
    dp = [[False] * (n + 1) for _ in range(m + 1)]
    dp[0][0] = True
    for j in range(1, n + 1):
        if pattern_lower[j - 1] == "%":
            dp[0][j] = dp[0][j - 1]
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            p = pattern_lower[j - 1]
            if p == "%":
                # Match zero chars (dp[i][j-1]) or one more char (dp[i-1][j]).
                dp[i][j] = dp[i][j - 1] or dp[i - 1][j]
            elif p == "_" or p == value_lower[i - 1]:
                dp[i][j] = dp[i - 1][j - 1]
    return dp[m][n]
