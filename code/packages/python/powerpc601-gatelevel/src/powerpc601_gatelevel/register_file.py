"""register_file.py — Gate-level 32-bit register file for the PowerPC 601.

The register file stores all programmer-visible state as bit lists:
  - 32 GPRs (GPR0–GPR31), each 32 bits
  - LR  (Link Register), 32 bits
  - CTR (Count Register), 32 bits
  - XER (Fixed-Point Exception Register), 32 bits
  - CR  (Condition Register), 32 bits
  - CIA (Current Instruction Address / PC), 32 bits

Gate-level storage
──────────────────
Each register is stored as a list[int] of 32 bits (a "register flip-flop bank").
On a real chip, each bit would be stored in a D flip-flop; reads are
combinational (the flip-flop output drives the bus) and writes are clocked
(the D input is loaded on the rising edge).

Here we model this as list[int] containing 0 or 1 values.  The external API
converts to/from Python integers for the instruction-level interface.

CIA increment
─────────────
The CIA is incremented by 4 on each instruction fetch.  We use add_32bit
(which routes through ripple_carry_adder) for this increment.

Why 4?  PowerPC instructions are 32-bit (4 bytes) fixed-width, stored
big-endian.  Incrementing the 32-bit CIA by 4 moves to the next instruction.

CR (Condition Register) layout
──────────────────────────────
CR is a 32-bit register divided into 8 × 4-bit fields:
  CR0 = bits [31:28] (MSB nibble in Python int = bit 31 to bit 28)
  CR7 = bits [3:0]   (LSB nibble)

Each field contains [LT, GT, EQ, SO] from high to low bit:
  Field bit 3 (MSB of nibble) = LT
  Field bit 2                  = GT
  Field bit 1                  = EQ
  Field bit 0 (LSB of nibble)  = SO

CR bit BI (0-indexed from MSB, per PowerPC spec) = Python bit weight 2^(31-BI).

For CR field n: the nibble occupies CR[31-4n : 28-4n] (Python bit weights).
  Field 0 (CR0): bits 31..28
  Field 1 (CR1): bits 27..24
  ...
  Field 7 (CR7): bits  3..0
"""

from __future__ import annotations

from logic_gates import AND, NOT, OR

from .bits import add_32bit, bits_to_int, int_to_bits

# Register bank size
_NUM_GPRS: int = 32
_REG_BITS: int = 32


class RegisterFilePPC:
    """Gate-level 32-bit register file for the PowerPC 601.

    Stores 32 GPRs plus LR, CTR, XER, CR, CIA as lists of 32 bits (LSB-first).
    All read/write operations convert between integers and bit lists.

    GPR0 special case:
      When rA=0 in load/store effective-address computation, the EA base is 0
      (not GPR[0]'s value).  However, GPR[0] can hold any value for arithmetic
      instructions.  This special case is handled in the simulator (decoder/
      instruction execution), not here.  The register file stores GPR[0] normally.

    Example
    ───────
    >>> rf = RegisterFilePPC()
    >>> rf.write_gpr(3, 42)
    >>> rf.read_gpr(3)
    42
    >>> rf.write_lr(0xDEAD)
    >>> rf.read_lr()
    57005
    """

    def __init__(self) -> None:
        # 32 GPRs × 32 bits each, all initialized to 0 (LSB-first bit lists)
        self._gprs: list[list[int]] = [
            [0] * _REG_BITS for _ in range(_NUM_GPRS)
        ]
        # Special-purpose registers stored as 32-bit lists
        self._lr:  list[int] = [0] * _REG_BITS   # Link Register
        self._ctr: list[int] = [0] * _REG_BITS   # Count Register
        self._xer: list[int] = [0] * _REG_BITS   # Fixed-Point Exception Register
        self._cr:  list[int] = [0] * _REG_BITS   # Condition Register
        self._cia: list[int] = [0] * _REG_BITS   # Current Instruction Address

    # ── GPR access ──────────────────────────────────────────────────────────────

    def read_gpr(self, n: int) -> int:
        """Read GPR n as a 32-bit unsigned integer.

        Parameters
        ──────────
        n : register number 0–31
        """
        return bits_to_int(self._gprs[n])

    def write_gpr(self, n: int, value: int) -> None:
        """Write a 32-bit value to GPR n.

        Parameters
        ──────────
        n     : register number 0–31
        value : 32-bit unsigned integer to store
        """
        self._gprs[n] = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── LR access ───────────────────────────────────────────────────────────────

    def read_lr(self) -> int:
        """Read the Link Register as a 32-bit unsigned integer."""
        return bits_to_int(self._lr)

    def write_lr(self, value: int) -> None:
        """Write the Link Register."""
        self._lr = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── CTR access ──────────────────────────────────────────────────────────────

    def read_ctr(self) -> int:
        """Read the Count Register as a 32-bit unsigned integer."""
        return bits_to_int(self._ctr)

    def write_ctr(self, value: int) -> None:
        """Write the Count Register."""
        self._ctr = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── XER access ──────────────────────────────────────────────────────────────

    def read_xer(self) -> int:
        """Read the XER register as a 32-bit unsigned integer."""
        return bits_to_int(self._xer)

    def write_xer(self, value: int) -> None:
        """Write the XER register."""
        self._xer = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── CR access ───────────────────────────────────────────────────────────────

    def read_cr(self) -> int:
        """Read the Condition Register as a 32-bit unsigned integer."""
        return bits_to_int(self._cr)

    def write_cr(self, value: int) -> None:
        """Write the Condition Register."""
        self._cr = int_to_bits(value & 0xFFFF_FFFF, 32)

    # ── CIA access ──────────────────────────────────────────────────────────────

    def read_cia(self) -> int:
        """Read the Current Instruction Address as a 32-bit unsigned integer."""
        return bits_to_int(self._cia)

    def write_cia(self, value: int) -> None:
        """Write the Current Instruction Address."""
        self._cia = int_to_bits(value & 0xFFFF_FFFF, 32)

    def increment_cia(self, by: int = 4) -> None:
        """Increment CIA by `by` bytes using gate-level add_32bit.

        Standard PowerPC instruction advance: by=4 (32-bit fixed-width instructions).

        The add routes through ripple_carry_adder (32 full adders), so this
        is a genuine gate-level operation.

        Parameters
        ──────────
        by : byte count to add to CIA (normally 4 for next instruction)
        """
        old_cia = bits_to_int(self._cia)
        new_cia, _carry, _ov = add_32bit(old_cia, by, 0)
        self._cia = int_to_bits(new_cia & 0xFFFF_FFFF, 32)

    # ── CR field helpers ─────────────────────────────────────────────────────────

    def set_cr_field(self, field: int, lt: int, gt: int, eq: int, so: int) -> None:
        """Set one 4-bit CR field using gate-level OR/AND operations.

        Each CR field (0=CR0, 7=CR7) occupies 4 bits:
          MSB: LT (less-than)
          ...
          LSB: SO (summary overflow)

        The field occupies bit positions 31-4*field down to 28-4*field
        in the 32-bit CR integer (Python bit weights).

        Gate-level update:
          1. Build a mask: 0xF shifted to the field position
          2. AND the current CR with NOT(mask) to clear the field
          3. OR in the new nibble shifted to the correct position

        Parameters
        ──────────
        field : CR field number 0–7 (0=CR0 occupies MSB nibble)
        lt, gt, eq, so : individual condition bits (0 or 1 each)
        """
        # Nibble value: [LT, GT, EQ, SO] from bit 3 to bit 0 of the nibble
        # LT is the MSB of each nibble (bit 3 within the nibble)
        nibble = (lt << 3) | (gt << 2) | (eq << 1) | so
        # Bit position of the nibble's LSB within the 32-bit CR:
        # Field 0 → shift=28, Field 1 → shift=24, ... Field 7 → shift=0
        shift = 28 - field * 4  # bookkeeping address arithmetic
        mask = 0xF << shift

        # Gate-level: clear field then set
        cr_val = bits_to_int(self._cr)
        # AND current CR with NOT(mask) — clear the 4-bit slot
        cr_bits = int_to_bits(cr_val, 32)
        mask_bits = int_to_bits(mask, 32)
        not_mask_bits = [NOT(m) for m in mask_bits]
        cleared_bits = [AND(c, nm) for c, nm in zip(cr_bits, not_mask_bits, strict=True)]

        # OR in the new nibble
        nibble_val = nibble << shift
        nibble_bits = int_to_bits(nibble_val, 32)
        new_cr_bits = [OR(c, n) for c, n in zip(cleared_bits, nibble_bits, strict=True)]

        self._cr = new_cr_bits

    def get_cr_bit(self, bi: int) -> int:
        """Get CR bit BI (0-indexed from MSB, per PowerPC encoding convention).

        In PowerPC, BI=0 means CR bit 31 (the MSB in Python's representation),
        BI=1 means bit 30, etc.  BI=31 means bit 0.

        This maps:  Python bit weight = 2^(31 - BI)
        In our LSB-first bit list: index = 31 - BI

        Gate-level extraction: AND the bit with 1 to isolate it.

        Parameters
        ──────────
        bi : CR bit index (0-indexed from MSB), 0–31

        Returns
        ───────
        0 or 1
        """
        cr_bits = self._cr  # LSB-first bit list
        # BI=0 → MSB of CR = bit[31] in our LSB-first list
        bit_index = 31 - bi  # bookkeeping index arithmetic
        return AND(cr_bits[bit_index], 1)

    # ── Snapshot interface ───────────────────────────────────────────────────────

    def get_gprs_tuple(self) -> tuple[int, ...]:
        """Return all 32 GPR values as a tuple (for state snapshot)."""
        return tuple(bits_to_int(self._gprs[i]) for i in range(_NUM_GPRS))

    def reset(self) -> None:
        """Reset all registers to zero."""
        self._gprs = [[0] * _REG_BITS for _ in range(_NUM_GPRS)]
        self._lr  = [0] * _REG_BITS
        self._ctr = [0] * _REG_BITS
        self._xer = [0] * _REG_BITS
        self._cr  = [0] * _REG_BITS
        self._cia = [0] * _REG_BITS
