"""Logic Gates — the foundation of all digital computing.

=== What is a logic gate? ===

A logic gate is the simplest possible decision-making element. It takes
one or two inputs, each either 0 or 1, and produces a single output
that is also 0 or 1. The output is entirely determined by the inputs —
there is no randomness, no hidden state, no memory.

In physical hardware, gates are built from transistors — tiny electronic
switches etched into silicon. A modern CPU contains billions of transistors
organized into billions of gates. But conceptually, every computation a
computer performs — from adding numbers to rendering video to running AI
models — ultimately reduces to combinations of these simple 0-or-1 operations.

This module implements the seven fundamental gates, proves that all of them
can be built from a single gate type (NAND), and provides multi-input variants.

=== Why only 0 and 1? ===

Computers use binary (base-2) because transistors are most reliable as
on/off switches. A transistor that is "on" (conducting electricity)
represents 1. A transistor that is "off" (blocking electricity) represents 0.
You could theoretically build a computer using base-3 or base-10, but the
error margins for distinguishing between voltage levels would make it
unreliable. Binary gives us two clean, easily distinguishable states.
"""

from functools import reduce


# ---------------------------------------------------------------------------
# Input validation
# ---------------------------------------------------------------------------
# Every gate checks that its inputs are valid binary values (0 or 1).
# We reject booleans (True/False) even though Python treats them as int
# subclasses, because allowing them would hide type confusion bugs.
# We also reject floats like 1.0 for the same reason.


def _validate_bit(value: int, name: str = "input") -> None:
    """Ensure a value is a binary bit: the integer 0 or the integer 1.

    We explicitly reject:
    - Booleans (True/False) — even though bool is a subclass of int in Python,
      accepting them silently would mask bugs where someone passes a comparison
      result instead of a bit value.
    - Floats (1.0, 0.0) — same reasoning; use int explicitly.
    - Integers outside {0, 1} — not valid binary digits.

    Example:
        >>> _validate_bit(0)     # OK
        >>> _validate_bit(1)     # OK
        >>> _validate_bit(2)     # raises ValueError
        >>> _validate_bit(True)  # raises TypeError
    """
    if not isinstance(value, int) or isinstance(value, bool):
        msg = f"{name} must be an int, got {type(value).__name__}"
        raise TypeError(msg)
    if value not in (0, 1):
        msg = f"{name} must be 0 or 1, got {value}"
        raise ValueError(msg)


# ===========================================================================
# THE FOUR FUNDAMENTAL GATES
# ===========================================================================
# These are the building blocks. NOT, AND, OR, and XOR are the four gates
# from which all other gates (and all of digital logic) can be constructed.
#
# Each gate is defined by its "truth table" — an exhaustive listing of
# every possible input combination and the corresponding output. Since each
# input can only be 0 or 1, a two-input gate has exactly 4 possible input
# combinations (2 × 2 = 4), making it easy to verify correctness.


def NOT(a: int) -> int:
    """The NOT gate (also called an "inverter").

    NOT is the simplest gate — it has one input and flips it.
    If the input is 0, the output is 1. If the input is 1, the output is 0.

    Think of it like a light switch: if the light is off (0), flipping the
    switch turns it on (1), and vice versa.

    Truth table:
        Input │ Output
        ──────┼───────
          0   │   1
          1   │   0

    Circuit symbol:
        a ──▷○── output
        (the small circle ○ means "invert")

    Example:
        >>> NOT(0)
        1
        >>> NOT(1)
        0
    """
    _validate_bit(a, "a")
    return 1 if a == 0 else 0


def AND(a: int, b: int) -> int:
    """The AND gate.

    AND takes two inputs and outputs 1 ONLY if BOTH inputs are 1.
    If either input is 0, the output is 0.

    Think of two switches wired in series (one after the other): electric
    current can only flow through if both switches are closed (both = 1).

    Truth table:
        A  B  │ Output
        ──────┼───────
        0  0  │   0      Neither is 1 → 0
        0  1  │   0      Only B is 1 → 0
        1  0  │   0      Only A is 1 → 0
        1  1  │   1      Both are 1  → 1  ✓

    Circuit symbol:
        a ──┐
            │D──── output
        b ──┘

    Example:
        >>> AND(1, 1)
        1
        >>> AND(1, 0)
        0
    """
    _validate_bit(a, "a")
    _validate_bit(b, "b")
    return 1 if a == 1 and b == 1 else 0


def OR(a: int, b: int) -> int:
    """The OR gate.

    OR takes two inputs and outputs 1 if EITHER input is 1 (or both).
    The output is 0 only when both inputs are 0.

    Think of two switches wired in parallel (side by side): current flows
    if either switch is closed.

    Truth table:
        A  B  │ Output
        ──────┼───────
        0  0  │   0      Neither is 1 → 0
        0  1  │   1      B is 1       → 1  ✓
        1  0  │   1      A is 1       → 1  ✓
        1  1  │   1      Both are 1   → 1  ✓

    Circuit symbol:
        a ──╲
             ╲──── output
        b ──╱

    Example:
        >>> OR(0, 0)
        0
        >>> OR(0, 1)
        1
    """
    _validate_bit(a, "a")
    _validate_bit(b, "b")
    return 1 if a == 1 or b == 1 else 0


def XOR(a: int, b: int) -> int:
    """The XOR gate (Exclusive OR).

    XOR outputs 1 if the inputs are DIFFERENT. Unlike OR, XOR outputs 0
    when both inputs are 1.

    The name "exclusive" means: one or the other, but NOT both.

    Truth table:
        A  B  │ Output
        ──────┼───────
        0  0  │   0      Same    → 0
        0  1  │   1      Different → 1  ✓
        1  0  │   1      Different → 1  ✓
        1  1  │   0      Same    → 0

    Why XOR matters for arithmetic:
        In binary addition, 1 + 1 = 10 (that's "one-zero" in binary, which
        equals 2 in decimal). The sum digit is 0 and the carry is 1.
        Notice that the sum digit (0) is exactly what XOR(1, 1) produces!

        0 + 0 = 0  →  XOR(0, 0) = 0  ✓
        0 + 1 = 1  →  XOR(0, 1) = 1  ✓
        1 + 0 = 1  →  XOR(1, 0) = 1  ✓
        1 + 1 = 0  →  XOR(1, 1) = 0  ✓  (carry the 1 separately)

        This is why XOR is the key gate in building adder circuits.

    Example:
        >>> XOR(1, 0)
        1
        >>> XOR(1, 1)
        0
    """
    _validate_bit(a, "a")
    _validate_bit(b, "b")
    return 1 if a != b else 0


# ===========================================================================
# COMPOSITE GATES
# ===========================================================================
# These gates are built by combining fundamental gates. They are included
# because they appear frequently in digital circuits and have useful properties.


def NAND(a: int, b: int) -> int:
    """The NAND gate (NOT AND).

    NAND is the inverse of AND: it outputs 1 in every case EXCEPT when both
    inputs are 1.

    Truth table:
        A  B  │ Output
        ──────┼───────
        0  0  │   1
        0  1  │   1
        1  0  │   1
        1  1  │   0      ← the only 0 output

    Why NAND is special — Functional Completeness:
        NAND has a remarkable property: you can build EVERY other gate using
        only NAND gates. This means if you had a factory that could only
        produce one type of gate, you'd pick NAND — because from NAND alone,
        you can construct NOT, AND, OR, XOR, and any other logic function.

        This property is called "functional completeness" and it's why real
        chip manufacturers often build entire processors from NAND gates —
        they're the cheapest and simplest to manufacture.

        See the nand_* functions below for proofs of how each gate is built
        from NAND.

    Implementation:
        NAND(a, b) = NOT(AND(a, b))

    Example:
        >>> NAND(1, 1)
        0
        >>> NAND(1, 0)
        1
    """
    return NOT(AND(a, b))


def NOR(a: int, b: int) -> int:
    """The NOR gate (NOT OR).

    NOR is the inverse of OR: it outputs 1 ONLY when both inputs are 0.

    Truth table:
        A  B  │ Output
        ──────┼───────
        0  0  │   1      ← the only 1 output
        0  1  │   0
        1  0  │   0
        1  1  │   0

    Like NAND, NOR is also functionally complete — you can build every
    other gate from NOR alone. (We don't demonstrate this here, but it's
    a fun exercise!)

    Implementation:
        NOR(a, b) = NOT(OR(a, b))

    Example:
        >>> NOR(0, 0)
        1
        >>> NOR(0, 1)
        0
    """
    return NOT(OR(a, b))


def XNOR(a: int, b: int) -> int:
    """The XNOR gate (Exclusive NOR, also called "equivalence gate").

    XNOR is the inverse of XOR: it outputs 1 when the inputs are the SAME.

    Truth table:
        A  B  │ Output
        ──────┼───────
        0  0  │   1      Same      → 1  ✓
        0  1  │   0      Different → 0
        1  0  │   0      Different → 0
        1  1  │   1      Same      → 1  ✓

    Use case:
        XNOR is used as an equality comparator. If you want to check whether
        two bits are equal, XNOR gives you the answer directly:
        XNOR(a, b) = 1 means a and b have the same value.

    Implementation:
        XNOR(a, b) = NOT(XOR(a, b))

    Example:
        >>> XNOR(1, 1)
        1
        >>> XNOR(1, 0)
        0
    """
    return NOT(XOR(a, b))


# ===========================================================================
# NAND-DERIVED GATES — Proving Functional Completeness
# ===========================================================================
# The functions below prove that NAND is functionally complete by building
# NOT, AND, OR, and XOR using ONLY the NAND gate. No other gate is used.
#
# This is not just an academic exercise. In real chip manufacturing, the
# ability to build everything from one gate type dramatically simplifies
# the fabrication process. The first commercially successful logic family
# (TTL 7400 series, introduced in 1966) was built around NAND gates.
#
# For each derived gate, we show:
# 1. The construction formula
# 2. A circuit diagram showing how NAND gates are wired
# 3. A proof by truth table that it matches the original gate


def nand_not(a: int) -> int:
    """NOT built entirely from NAND gates.

    Construction:
        NOT(a) = NAND(a, a)

    Why this works:
        NAND outputs 0 only when both inputs are 1.
        If we feed the same value to both inputs:
        - NAND(0, 0) = 1  (neither is 1, so NOT 0 = 1 ✓)
        - NAND(1, 1) = 0  (both are 1, so NOT 1 = 0 ✓)

    Circuit:
        a ──┬──┐
            │  │D──○── output
            └──┘
        (both inputs of the NAND come from the same wire)

    Example:
        >>> nand_not(0)
        1
        >>> nand_not(1)
        0
    """
    return NAND(a, a)


def nand_and(a: int, b: int) -> int:
    """AND built entirely from NAND gates.

    Construction:
        AND(a, b) = NOT(NAND(a, b)) = NAND(NAND(a, b), NAND(a, b))

    Why this works:
        NAND is the opposite of AND. So if we invert NAND's output (using
        our nand_not trick above), we get AND back.

    Circuit (2 NAND gates):
        a ──┐
            │D──○──┬──┐
        b ──┘      │  │D──○── output
                   └──┘
        Gate 1: NAND(a, b)
        Gate 2: NAND(result, result) = NOT(result) = AND(a, b)

    Example:
        >>> nand_and(1, 1)
        1
        >>> nand_and(1, 0)
        0
    """
    return nand_not(NAND(a, b))


def nand_or(a: int, b: int) -> int:
    """OR built entirely from NAND gates.

    Construction:
        OR(a, b) = NAND(NOT(a), NOT(b)) = NAND(NAND(a,a), NAND(b,b))

    Why this works (De Morgan's Law):
        De Morgan's Law states: NOT(A AND B) = (NOT A) OR (NOT B)
        Rearranging: A OR B = NOT(NOT(A) AND NOT(B)) = NAND(NOT(A), NOT(B))

        This is a fundamental identity in Boolean algebra, discovered by
        Augustus De Morgan in the 1800s — long before electronic computers
        existed!

    Circuit (3 NAND gates):
        a ──┬──┐
            │  │D──○──┐
            └──┘      │
                      │D──○── output
        b ──┬──┐      │
            │  │D──○──┘
            └──┘
        Gate 1: NAND(a, a) = NOT(a)
        Gate 2: NAND(b, b) = NOT(b)
        Gate 3: NAND(NOT(a), NOT(b)) = OR(a, b)

    Example:
        >>> nand_or(0, 1)
        1
        >>> nand_or(0, 0)
        0
    """
    return NAND(nand_not(a), nand_not(b))


def nand_xor(a: int, b: int) -> int:
    """XOR built entirely from NAND gates.

    Construction:
        Let N = NAND(a, b)
        XOR(a, b) = NAND(NAND(a, N), NAND(b, N))

    Why this works:
        This is the most complex NAND construction. It uses 4 NAND gates.
        The intermediate value N = NAND(a, b) is reused twice, which is
        why XOR is more "expensive" in hardware than AND or OR.

        Proof by truth table:
        a=0, b=0: N=NAND(0,0)=1, NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0 ✓
        a=0, b=1: N=NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0, NAND(1,0)=1 ✓
        a=1, b=0: N=NAND(1,0)=1, NAND(1,1)=0, NAND(0,1)=1, NAND(0,1)=1 ✓
        a=1, b=1: N=NAND(1,1)=0, NAND(1,0)=1, NAND(1,0)=1, NAND(1,1)=0 ✓

    Circuit (4 NAND gates):
        a ──┬────────┐
            │        │D──○── wire1 ──┐
            │   ┌──┐ │               │D──○── output
            │   │  │D──○── N ──┐     │
        b ──┼───┘              │     │
            │                  │D──○─┘
            └──────────────────┘
                              wire2

        Gate 1: N = NAND(a, b)
        Gate 2: wire1 = NAND(a, N)
        Gate 3: wire2 = NAND(b, N)
        Gate 4: output = NAND(wire1, wire2)

    Example:
        >>> nand_xor(1, 0)
        1
        >>> nand_xor(1, 1)
        0
    """
    nand_ab = NAND(a, b)
    return NAND(NAND(a, nand_ab), NAND(b, nand_ab))


# ===========================================================================
# MULTI-INPUT GATES
# ===========================================================================
# In practice, you often need to AND or OR more than two values together.
# For example, "are ALL four conditions true?" requires a 4-input AND.
#
# Multi-input gates work by chaining 2-input gates. For AND:
#   AND_N(a, b, c, d) = AND(AND(AND(a, b), c), d)
#
# Python's `reduce` function does exactly this: it takes a list and
# repeatedly applies a 2-argument function from left to right.


def AND_N(*inputs: int) -> int:
    """AND with N inputs. Returns 1 only if ALL inputs are 1.

    This chains 2-input AND gates together using reduce:
        AND_N(a, b, c, d) = AND(AND(AND(a, b), c), d)

    In hardware, this would be a chain of AND gates:
        a ──┐
            │D── r1 ──┐
        b ──┘         │D── r2 ──┐
                 c ───┘         │D── output
                          d ───┘

    Example:
        >>> AND_N(1, 1, 1, 1)
        1
        >>> AND_N(1, 1, 0, 1)
        0
    """
    if len(inputs) < 2:
        msg = "AND_N requires at least 2 inputs"
        raise ValueError(msg)
    return reduce(AND, inputs)


def OR_N(*inputs: int) -> int:
    """OR with N inputs. Returns 1 if ANY input is 1.

    This chains 2-input OR gates together using reduce:
        OR_N(a, b, c, d) = OR(OR(OR(a, b), c), d)

    Example:
        >>> OR_N(0, 0, 0, 0)
        0
        >>> OR_N(0, 0, 1, 0)
        1
    """
    if len(inputs) < 2:
        msg = "OR_N requires at least 2 inputs"
        raise ValueError(msg)
    return reduce(OR, inputs)


def XOR_N(*bits: int) -> int:
    """N-input XOR gate — reduces a sequence of bits via XOR (parity checker).

    XOR_N(a, b, c, d) = XOR(XOR(XOR(a, b), c), d)

    Returns 1 if an odd number of inputs are 1 (odd parity).
    Returns 0 if an even number of inputs are 1 (even parity).

    This is how the 8008's parity flag is computed in hardware: a chain
    of XOR gates reduces 8 bits to a single parity bit. The result tells
    you whether the number of 1-bits in the byte is odd (XOR_N=1) or
    even (XOR_N=0).

    For the 8008 P flag: P = NOT(XOR_N(*result_bits))
    (P=1 means even parity — even number of 1-bits in the result)

    Truth table examples:
        XOR_N(0, 0) = 0   (0 ones → even parity → 0)
        XOR_N(1, 0) = 1   (1 one  → odd parity  → 1)
        XOR_N(1, 1) = 0   (2 ones → even parity → 0)
        XOR_N(1, 1, 1) = 1 (3 ones → odd parity → 1)

    Unlike AND_N and OR_N, XOR_N accepts 0 or 1 inputs:
        XOR_N()    → 0  (zero ones is even parity, XOR identity is 0)
        XOR_N(a,)  → a  (single input passes through unchanged)

    This mirrors the mathematical definition: XOR is the addition
    operation in GF(2) (the Galois Field with two elements {0, 1}).
    XOR_N over N bits computes the sum modulo 2 of those N bits.

    Hardware note: for 8 bits, a balanced binary XOR tree uses 7 gates:
        level 1: XOR(b0,b1), XOR(b2,b3), XOR(b4,b5), XOR(b6,b7) — 4 gates
        level 2: XOR(^0^1, ^2^3), XOR(^4^5, ^6^7)               — 2 gates
        level 3: XOR(^^01^^23, ^^45^^67)                          — 1 gate
    Total: 7 XOR gates for an 8-bit parity tree.

    Args:
        *bits: Any number of bit values (each must be 0 or 1).

    Returns:
        0 or 1.

    Example:
        >>> XOR_N(0, 0, 0, 0, 0, 0, 1, 1)
        0
        >>> XOR_N(0, 0, 0, 0, 0, 0, 0, 1)
        1
    """
    for i, b in enumerate(bits):
        _validate_bit(b, f"bit[{i}]")
    if not bits:
        return 0
    return reduce(XOR, bits)
