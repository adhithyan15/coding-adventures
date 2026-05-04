"""State snapshot for the Manchester Baby (SSEM, 1948).

──────────────────────────────────────────────────────────────────────────────
THE MACHINE IN A NUTSHELL
──────────────────────────────────────────────────────────────────────────────

The Manchester Baby was beautifully simple: the *entire computer* fits in just
two registers and a 32-word memory.

  A   — Accumulator (32-bit, two's complement)
        The only register the programmer can read or write directly.
        All arithmetic flows through A.

  CI  — Control Instruction counter (≈ program counter, 5 bits)
        Points to the line number currently being executed.
        Only values 0–31 are meaningful (the store has exactly 32 lines).
        The hardware starts CI at 31 (= −1 mod 32) so the first increment
        brings it to 0, making line 0 the first instruction to run.

  Store — 32 × 32-bit words (the Williams-tube "memory")
        Both code and data live here together (von Neumann architecture).
        Words are 32-bit two's-complement integers, stored in the snapshot
        as *unsigned* Python ints (0 … 2³²−1).  Use ``acc_signed`` when you
        need the signed interpretation.

  Halted — bool
        True after the STP instruction executes.  The simulator will refuse
        to step() on a halted machine.

──────────────────────────────────────────────────────────────────────────────
TWO'S COMPLEMENT QUICK REFERENCE
──────────────────────────────────────────────────────────────────────────────

The Baby uses 32-bit two's complement throughout:

    Unsigned range:  0x00000000 … 0xFFFFFFFF  (0 … 4 294 967 295)
    Signed range:    0x80000000 … 0x7FFFFFFF  (−2 147 483 648 … 2 147 483 647)

    To convert an unsigned 32-bit value to signed:
        if value >= 0x80000000: signed = value - 0x100000000
        else:                   signed = value

    This is exactly what ``acc_signed`` computes via Python's ctypes trick.

The simulator stores all values as *unsigned* Python ints (no negatives) and
applies ``& 0xFFFFFFFF`` after arithmetic.  This keeps the arithmetic simple
and avoids Python's arbitrary-precision integers accidentally "going negative"
mid-calculation.

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from dataclasses import dataclass

# Width constants — document the hardware constraints in one place.
_WORD_MASK = 0xFFFFFFFF   # 32-bit unsigned mask
_STORE_SIZE = 32          # exactly 32 lines of store
_CI_MASK = 0x1F           # CI is 5 bits (0–31)


@dataclass(frozen=True)
class BabyState:
    """Immutable snapshot of the Manchester Baby's complete internal state.

    Frozen (immutable) so that:
      - Tests can safely store snapshots mid-execution without risk of
        the simulator later mutating the values.
      - ``ExecutionResult.final_state`` is truly final.

    All numeric fields hold *unsigned* Python ints so they compare cleanly
    to hex literals in test assertions.  Use ``acc_signed`` when you need
    the signed interpretation of the accumulator.

    Attributes
    ----------
    store :
        Tuple of exactly 32 unsigned 32-bit ints representing the 32 words
        of Williams-tube memory.  Both program instructions and data values
        live here (von Neumann architecture).

        Example: ``state.store[3]`` is the word at line 3.

    accumulator :
        The A register as an unsigned 32-bit int (0 … 0xFFFFFFFF).
        This is the result of the most recent LDN or SUB instruction, or
        the value written by the programmer's STO sequence.

    ci :
        The Control Instruction counter — the line number of the
        *most recently executed* instruction (0–31).  After step() returns,
        ``store[ci]`` holds the instruction that just ran.

    halted :
        True after STP executes; the simulator will raise RuntimeError on
        any further step() call.

    Examples
    --------
    >>> state = BabyState(
    ...     store=tuple([0] * 32),
    ...     accumulator=0xFFFFFFD6,   # −42 in two's complement
    ...     ci=4,
    ...     halted=True,
    ... )
    >>> state.acc_signed
    -42
    >>> state.present_instruction
    0
    """

    store: tuple[int, ...]       # 32 unsigned 32-bit words
    accumulator: int             # unsigned 32-bit
    ci: int                      # 0–31 (word address of last executed instr)
    halted: bool

    # ------------------------------------------------------------------
    # Helper properties
    # ------------------------------------------------------------------

    @property
    def acc_signed(self) -> int:
        """Return the accumulator as a *signed* Python int (−2³¹ … 2³¹−1).

        The raw ``accumulator`` field is stored unsigned (0 … 0xFFFFFFFF).
        This property converts it to the signed interpretation the programmer
        sees.

        Two's complement rule:
          - If bit 31 is 0 (accumulator < 0x80000000): the value is positive,
            no conversion needed.
          - If bit 31 is 1 (accumulator ≥ 0x80000000): the value is negative;
            subtract 2³² to get the signed result.

        Examples
        --------
        >>> s = tuple([0] * 32)
        >>> BabyState(store=s, accumulator=0, ci=0, halted=False).acc_signed
        0
        >>> BabyState(store=s, accumulator=42, ci=0, halted=False).acc_signed
        42
        >>> BabyState(store=s, accumulator=0xFFFFFFD6, ci=0, halted=False).acc_signed
        -42
        >>> BabyState(store=s, accumulator=0x80000000, ci=0, halted=False).acc_signed
        -2147483648
        >>> BabyState(store=s, accumulator=0x7FFFFFFF, ci=0, halted=False).acc_signed
        2147483647
        """
        a = self.accumulator
        if a >= 0x80000000:
            return a - 0x100000000
        return a

    @property
    def present_instruction(self) -> int:
        """Return the raw 32-bit word at the current CI position.

        This is the instruction word most recently executed (or about to be
        executed if inspected between steps).  Useful for disassembly and
        debugging.

        Example
        -------
        >>> # LDN 5 encodes as (0b010 << 13) | 5 = 0x4005
        >>> store = [0] * 32
        >>> store[2] = 0x4005
        >>> s = BabyState(store=tuple(store), accumulator=0, ci=2, halted=False)
        >>> hex(s.present_instruction)
        '0x4005'
        """
        return self.store[self.ci]
