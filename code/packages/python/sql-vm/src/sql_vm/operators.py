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

from .errors import TypeMismatch


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

    # IS [NOT] DISTINCT FROM — NULL-safe comparisons that always return a bool,
    # never NULL.  They must be handled *before* the general NULL short-circuit
    # because they need to see NULL operands directly.
    if op is BinaryOpCode.IS_DISTINCT_FROM:
        # Two values are "distinct" when they differ or exactly one is NULL.
        # Truth table (never returns NULL):
        #   NULL IS DISTINCT FROM NULL  → FALSE  (both null = same)
        #   NULL IS DISTINCT FROM 1     → TRUE   (one null, one not)
        #   1    IS DISTINCT FROM 2     → TRUE   (different values)
        #   1    IS DISTINCT FROM 1     → FALSE  (equal values)
        if left is None and right is None:
            return False
        if left is None or right is None:
            return True
        return left != right
    if op is BinaryOpCode.IS_NOT_DISTINCT_FROM:
        # Two values are "not distinct" when they are equal or both NULL.
        # NULL-safe equality — equivalent to Python ``left == right`` where
        # None == None.
        #   NULL IS NOT DISTINCT FROM NULL  → TRUE
        #   NULL IS NOT DISTINCT FROM 1     → FALSE
        #   1    IS NOT DISTINCT FROM 1     → TRUE
        #   1    IS NOT DISTINCT FROM 2     → FALSE
        if left is None and right is None:
            return True
        if left is None or right is None:
            return False
        return left == right

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
        # SQLite returns NULL for ``x / 0`` rather than raising — matches its
        # philosophy that arithmetic errors yield NULL.  Previously we raised
        # DivisionByZero, which mini-sqlite surfaced as OperationalError.
        if b == 0:
            return None
        # Integer division when both operands are ints; truncate toward zero
        # as C / most SQL dialects do, not Python's floor-divide semantics.
        if isinstance(a, int) and isinstance(b, int):
            q = abs(a) // abs(b)
            return -q if (a < 0) ^ (b < 0) else q
        return a / b
    if op is BinaryOpCode.MOD:
        if b == 0:
            # SQLite returns NULL for x % 0 rather than raising an error.
            return None
        # SQLite's ``%`` is C-style ``fmod`` *with an integer cast first*:
        # if either operand is float, both are truncated toward zero and
        # the result is the integer modulo cast back to float.  This is
        # different from the ``mod()`` scalar function (which uses true
        # fmod).  Examples:
        #
        #     7   %  3   → 1
        #    -7   %  3   → -1   (sign follows dividend, not Python's 2)
        #     7   % -3   → 1
        #     7.5 %  2.0 → 1.0  (truncate to 7 % 2, then cast back)
        #    15.5 %  4.5 → 3.0  (truncate to 15 % 4)
        is_float = isinstance(a, float) or isinstance(b, float)
        ia, ib = int(a), int(b)
        if ib == 0:
            # Both operands truncate to 0-modulus (e.g. ``1.5 % 0.5`` →
            # ``int(0.5) = 0``).  Mirror SQLite's NULL-on-error policy.
            return None
        magnitude = abs(ia) % abs(ib)
        result = -magnitude if ia < 0 else magnitude
        return float(result) if is_float else result
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


def _truthiness(v: SqlValue) -> bool | None:
    """Coerce a SqlValue to a SQL truth value.

    SQLite has no separate BOOLEAN type at the storage level: integers
    and floats double as booleans, with zero meaning FALSE and every
    other numeric value (including negative numbers and floats) meaning
    TRUE.  Python ``bool`` is a subtype of ``int`` and round-trips
    naturally.  NULL coerces to ``None``.

    Strings are deliberately ambiguous here — SQL would coerce ``'1'``
    via NUMERIC affinity to TRUE, but the AND/OR operator never sees a
    raw string in well-typed code, so we conservatively return ``None``
    and let the caller raise a TypeMismatch.
    """
    if v is None:
        return None
    if isinstance(v, bool):
        return v
    if isinstance(v, (int, float)):
        return bool(v)
    return None


def _and(left: SqlValue, right: SqlValue) -> SqlValue:
    # Three-valued AND: FALSE dominates; NULL only if no FALSE seen.
    # Operands are coerced via SQLite's numeric truth rule — ``1 AND 0``
    # must yield ``0`` (FALSE), not raise TypeMismatch and definitely
    # not silently produce NULL (the bug that lived here until now).
    lt = _truthiness(left)
    rt = _truthiness(right)
    if lt is False or rt is False:
        return False
    if lt is None or rt is None:
        # Either NULL or non-coercible — see _truthiness for the string case.
        if not (isinstance(left, (bool, int, float)) or left is None) \
                or not (isinstance(right, (bool, int, float)) or right is None):
            raise TypeMismatch(
                expected="boolean",
                got=f"{sql_type_name(left)}, {sql_type_name(right)}",
                context="BinaryOp(AND)",
            )
        return None
    return True


def _or(left: SqlValue, right: SqlValue) -> SqlValue:
    # Three-valued OR: TRUE dominates; NULL only if no TRUE seen.
    # See ``_and`` for the numeric-truthiness rationale.
    lt = _truthiness(left)
    rt = _truthiness(right)
    if lt is True or rt is True:
        return True
    if lt is None or rt is None:
        if not (isinstance(left, (bool, int, float)) or left is None) \
                or not (isinstance(right, (bool, int, float)) or right is None):
            raise TypeMismatch(
                expected="boolean",
                got=f"{sql_type_name(left)}, {sql_type_name(right)}",
                context="BinaryOp(OR)",
            )
        return None
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
        # Same coercion rule as the binary AND/OR path: any non-NULL numeric
        # is a valid truth operand for NOT.  SQLite has no separate BOOLEAN
        # storage class, so ``NOT 0`` must give ``1`` and ``NOT 5`` must
        # give ``0``.  Strings (which have no defined truth value in this
        # registry) still raise TypeMismatch.
        truth = _truthiness(value)
        if truth is None:
            raise TypeMismatch(
                expected="boolean", got=sql_type_name(value), context="UnaryOp(NOT)"
            )
        return not truth
    raise TypeMismatch(expected="unary op", got=str(op), context="UnaryOp")


# --------------------------------------------------------------------------
# LIKE matcher. Patterns use ``%`` for zero-or-more chars, ``_`` for exactly
# one. Everything else is a literal. Case-sensitive by spec.
# --------------------------------------------------------------------------


def like_match(value: str, pattern: str, escape: str | None = None) -> bool:
    """Case-insensitive LIKE matcher (SQLite / ANSI SQL default behaviour).

    LIKE is case-insensitive for ASCII letters by default in SQLite and in
    the SQL standard.  Non-ASCII characters (Unicode) are compared
    case-sensitively here, which matches SQLite's behaviour when the
    ``NOCASE`` collation is not in effect and ICU is not compiled in.

    Wildcards::

        %   — matches zero or more characters
        _   — matches exactly one character

    If *escape* is provided (a single-character string), it disables the
    wildcard meaning of the following character in the pattern.  For
    example ``LIKE 'a\\_b' ESCAPE '\\'`` matches the literal string ``"a_b"``.
    An escape character at the end of the pattern, or followed by some
    character other than ``%``, ``_``, or the escape character itself, is
    a syntax error in SQLite; here we tolerate it by treating both the
    escape character and the next character as literal.

    Truth table::

        like_match('Hello', 'hello')                  → True
        like_match('abc',   'a%c')                    → True
        like_match('ac',    'a_c')                    → False
        like_match('a_b',   'a\\\\_b', escape='\\\\')   → True
        like_match('axb',   'a\\\\_b', escape='\\\\')   → False

    Algorithm: the pattern is first tokenised into a list of three kinds of
    tokens — ``('star',)``, ``('one',)``, and ``('lit', c)`` — that
    collapse each escape+char pair into a single literal token.  Then a
    standard wildcard-matching DP runs in O(m·k) time where m=len(value)
    and k=number of tokens.
    """
    # Normalise to lowercase for case-insensitive ASCII comparison.
    value_lower = value.lower()
    pattern_lower = pattern.lower()
    esc_lower = escape.lower() if (escape is not None and len(escape) == 1) else None

    # Tokenise the pattern.  Each token is either:
    #   ('star',)       — % wildcard, matches zero or more chars
    #   ('one',)        — _ wildcard, matches exactly one char
    #   ('lit',   c)    — match this exact character
    tokens: list[tuple[str, str]] = []
    j = 0
    while j < len(pattern_lower):
        c = pattern_lower[j]
        # Escape character followed by another character: collapse the pair
        # into a single literal token for the following character.
        if esc_lower is not None and c == esc_lower and j + 1 < len(pattern_lower):
            tokens.append(("lit", pattern_lower[j + 1]))
            j += 2
            continue
        if c == "%":
            # Collapse consecutive %s into a single star token to keep the DP
            # state space small for adversarial patterns like '%%%%%a'.
            if not tokens or tokens[-1] != ("star", ""):
                tokens.append(("star", ""))
            j += 1
            continue
        if c == "_":
            tokens.append(("one", ""))
            j += 1
            continue
        tokens.append(("lit", c))
        j += 1

    # Standard wildcard-matching DP over (value_position, token_position).
    # dp[i][k] = True if value[:i] matches tokens[:k].
    m, k_max = len(value_lower), len(tokens)
    dp = [[False] * (k_max + 1) for _ in range(m + 1)]
    dp[0][0] = True
    # Empty value matches a leading run of stars (and only stars).
    for k in range(1, k_max + 1):
        if tokens[k - 1][0] == "star":
            dp[0][k] = dp[0][k - 1]
    for i in range(1, m + 1):
        vi = value_lower[i - 1]
        for k in range(1, k_max + 1):
            kind, lit = tokens[k - 1]
            if kind == "star":
                # Either consume zero value chars (dp[i][k-1]) or one more
                # value char while staying on the star (dp[i-1][k]).
                dp[i][k] = dp[i][k - 1] or dp[i - 1][k]
            elif kind == "one":
                dp[i][k] = dp[i - 1][k - 1]
            else:  # kind == "lit"
                if vi == lit:
                    dp[i][k] = dp[i - 1][k - 1]
    return dp[m][k_max]
