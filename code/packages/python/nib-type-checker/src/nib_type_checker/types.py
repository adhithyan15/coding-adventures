"""Nib type system — NibType enum and type resolution helpers.

=============================================================================
WHAT IS A TYPE SYSTEM?
=============================================================================

A **type system** is a formal set of rules that assigns a *type* to every
expression in a program. Types serve two purposes:

1. **Safety**: They prevent meaningless operations. Adding a boolean to an
   integer is nonsense; the type checker catches this before it runs.

2. **Code generation**: They tell the backend how many bits to allocate for
   a variable, which machine instructions to emit, and how to pass arguments.

Nib's type system is deliberately minimal — just four types, matching the
Intel 4004's hardware capabilities exactly.

=============================================================================
THE FOUR NIB TYPES
=============================================================================

``u4`` — 4-bit unsigned integer
    Range 0–15. This is one 4004 register (e.g., R0). Arithmetic is done
    with the 4004's ADD instruction. Overflow is either wrapping (``+%``)
    or saturating (``+?``).

    Example: ``let x: u4 = 0xF;``

``u8`` — 8-bit unsigned integer
    Range 0–255. This is one 4004 register *pair* (e.g., P0 = R0/R1). The
    4004 has no native 8-bit ADD; we use two 4-bit ADDs with carry. Useful
    for counters, addresses, and byte-oriented I/O.

    Example: ``let count: u8 = 200;``

``bcd`` — Binary-Coded Decimal digit
    Range 0–9. Stored as 4 bits in one register, but interpreted as a
    decimal digit. The 4004 has a special DAA (Decimal Adjust Accumulator)
    instruction for BCD arithmetic — a relic of its calculator origins.

    BCD has a strict operator restriction: only ``+%`` (wrapping add with
    DAA adjustment) and ``-`` (subtraction) are legal. Plain ``+`` is
    banned because it would not emit DAA.

    The reason this restriction exists in the *type checker* (not the code
    generator) is that it is a *language-level* rule: the programmer must
    explicitly choose BCD-aware arithmetic, making it visible in the source
    code and checkable without any target knowledge.

    Example: ``let digit: bcd = 7;``

``bool`` — boolean
    0 (false) or 1 (true). The 4004 has no native boolean type; booleans
    are stored as a nibble containing 0 or 1. All conditionals (``if``,
    ``for`` bounds, ``||``, ``&&``, ``!``) require ``bool`` to prevent
    accidental use of integers as conditions.

    Example: ``let flag: bool = true;``

=============================================================================
TYPE SIZES
=============================================================================

The ``size_bytes`` property returns the storage cost of each type. The
code generator and backend validator use this to compute total static RAM
usage (the 4004 has only 160 bytes).

    u4   → 1 byte  (one nibble, but stored byte-aligned)
    u8   → 2 bytes (register pair)
    bcd  → 1 byte  (one nibble, byte-aligned)
    bool → 1 byte  (one nibble, byte-aligned)

=============================================================================
OPERATOR COMPATIBILITY
=============================================================================

Not all operators work with all types. The ``is_bcd_op_allowed`` helper
encodes the BCD operator restriction:

    Allowed for bcd:  +%  (WRAP_ADD) and  -  (MINUS)
    Forbidden for bcd: +  (PLUS), +?  (SAT_ADD), *  (STAR), /  (SLASH)

The ``is_numeric`` helper identifies types where integer arithmetic is
legal (u4, u8, bcd — but *not* bool, which only supports logical ops).

The ``types_are_compatible`` helper checks whether two types match for
an assignment or function-call argument check. Nib has *no implicit
widening*: you cannot assign a ``u4`` to a ``u8`` variable without an
explicit cast (future language feature). This strictness avoids the class
of bugs where a narrow value silently widens into a different numeric
representation.
"""

from __future__ import annotations

from enum import Enum


class NibType(Enum):
    """The four first-class types in the Nib language.

    Each variant corresponds to a specific hardware representation on the
    Intel 4004 microprocessor. The enum values are the literal strings used
    in source code — ``u4``, ``u8``, ``bcd``, ``bool`` — so they double as
    the canonical string representation.

    Examples
    --------
    >>> NibType.U4.value
    'u4'
    >>> NibType.U8.size_bytes
    2
    >>> NibType.BCD.size_bytes
    1
    >>> NibType.BOOL.value
    'bool'
    """

    U4 = "u4"
    U8 = "u8"
    BCD = "bcd"
    BOOL = "bool"

    @property
    def size_bytes(self) -> int:
        """Storage size in bytes on the Intel 4004.

        The 4004 allocates memory in bytes even though its registers are
        nibble-sized. ``u8`` needs two bytes (a register pair); all other
        types fit in one byte.

        Returns
        -------
        int
            2 for ``u8``; 1 for ``u4``, ``bcd``, and ``bool``.

        Examples
        --------
        >>> NibType.U8.size_bytes
        2
        >>> NibType.U4.size_bytes
        1
        """
        return 2 if self == NibType.U8 else 1


# ---------------------------------------------------------------------------
# Type resolution helpers
# ---------------------------------------------------------------------------


def parse_type_name(name: str) -> NibType | None:
    """Convert a source-level type name string to a ``NibType``.

    The Nib type names are ``u4``, ``u8``, ``bcd``, and ``bool``.  These are
    NAME tokens in the AST (not keywords), so this helper converts the token
    value to the corresponding enum variant.

    Parameters
    ----------
    name:
        The string value of a type token (e.g., ``"u4"``).

    Returns
    -------
    NibType | None
        The matching ``NibType``, or ``None`` if the name is not a valid
        Nib type.

    Examples
    --------
    >>> parse_type_name("u4")
    <NibType.U4: 'u4'>
    >>> parse_type_name("bool")
    <NibType.BOOL: 'bool'>
    >>> parse_type_name("int") is None
    True
    """
    mapping: dict[str, NibType] = {
        "u4": NibType.U4,
        "u8": NibType.U8,
        "bcd": NibType.BCD,
        "bool": NibType.BOOL,
    }
    return mapping.get(name)


def types_are_compatible(lhs: NibType, rhs: NibType) -> bool:
    """Return True if ``rhs`` can be assigned to a location typed ``lhs``.

    Nib has **no implicit widening**. The types must match exactly. This
    prevents the class of subtle bugs where, say, a ``u4`` (0–15) silently
    widens into a ``u8`` (0–255) and the programmer forgets to handle the
    extended range.

    In a future version of Nib, explicit ``as u8`` casts may be added. For
    now, every assignment and every function-call argument must match exactly.

    Parameters
    ----------
    lhs:
        The declared/expected type (left-hand side of assignment, or the
        declared parameter type for a function call).
    rhs:
        The inferred type of the expression being assigned or passed.

    Returns
    -------
    bool
        True if ``lhs == rhs``; False otherwise.

    Examples
    --------
    >>> types_are_compatible(NibType.U4, NibType.U4)
    True
    >>> types_are_compatible(NibType.U4, NibType.U8)
    False
    >>> types_are_compatible(NibType.BOOL, NibType.BOOL)
    True
    """
    return lhs == rhs


def is_bcd_op_allowed(operator_value: str) -> bool:
    """Return True if ``operator_value`` is legal for BCD operands.

    BCD arithmetic on the Intel 4004 uses the DAA (Decimal Adjust
    Accumulator) instruction, which only exists for addition. Subtraction
    works by complementing, adding, and re-complementing — but *only* if
    the compiler knows it needs to. Multiplication and division do not have
    BCD equivalents on the 4004 at all.

    The allowed operators for BCD:

    +  ``+%`` (``WRAP_ADD``) — modular BCD add with DAA correction. On the
       4004, this emits ADD + DAA (or ADD + LD for single-digit result).
    +  ``-`` (``MINUS``) — BCD subtraction via ten's complement.

    The forbidden operators for BCD:

    -  ``+`` (bare PLUS) — would not emit DAA; produces raw binary sum.
    -  ``+?`` (SAT_ADD) — saturating; not meaningful for decimal digits.
    -  ``*`` (STAR) — not supported in Nib v1 at all.
    -  ``/`` (SLASH) — not supported in Nib v1 at all.

    Parameters
    ----------
    operator_value:
        The literal text of the operator token (e.g., ``"+%"``, ``"-"``,
        ``"+"``, ``"*"``).

    Returns
    -------
    bool
        True only for ``"+%"`` and ``"-"``.

    Examples
    --------
    >>> is_bcd_op_allowed("+%")
    True
    >>> is_bcd_op_allowed("-")
    True
    >>> is_bcd_op_allowed("+")
    False
    >>> is_bcd_op_allowed("+?")
    False
    """
    return operator_value in {"+%", "-"}


def is_numeric(t: NibType) -> bool:
    """Return True if ``t`` supports numeric arithmetic operators.

    ``bool`` only supports logical operators (``&&``, ``||``, ``!``).
    The numeric types (``u4``, ``u8``, ``bcd``) support arithmetic
    operators, though ``bcd`` has additional restrictions (see
    ``is_bcd_op_allowed``).

    Parameters
    ----------
    t:
        The type to check.

    Returns
    -------
    bool
        True for ``u4``, ``u8``, ``bcd``; False for ``bool``.

    Examples
    --------
    >>> is_numeric(NibType.U4)
    True
    >>> is_numeric(NibType.BCD)
    True
    >>> is_numeric(NibType.BOOL)
    False
    """
    return t in {NibType.U4, NibType.U8, NibType.BCD}
