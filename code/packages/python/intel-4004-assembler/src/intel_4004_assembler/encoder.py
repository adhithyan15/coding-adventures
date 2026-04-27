"""encoder.py -- Encode a single Intel 4004 instruction into bytes.

Intel 4004 Instruction Encoding Reference
------------------------------------------

The Intel 4004 has an 8-bit instruction word (sometimes two bytes).

Bit layout of the first byte: [OPH | OPL] where OPH is the high nibble
(bits 7 to 4) and OPL is the low nibble (bits 3 to 0).

Full opcode table (46 instructions):

One-byte instructions (fixed opcodes):

    NOP  = 0x00
    HLT  = 0x01  (simulator-only, not in original 4004)
    SRC Pp = 0x20 | (pair_index * 2 + 1)   e.g. P0 -> 0x21, P1 -> 0x23
    FIN Pp = 0x30 | (pair_index * 2)        e.g. P0 -> 0x30, P1 -> 0x32
    JIN Pp = 0x30 | (pair_index * 2 + 1)   e.g. P0 -> 0x31, P1 -> 0x33
    INC Rn = 0x60 | n   (n = register index 0..15)
    ADD Rn = 0x80 | n
    SUB Rn = 0x90 | n
    LD  Rn = 0xA0 | n
    XCH Rn = 0xB0 | n
    BBL k  = 0xC0 | k   (k = 0..15, return value)
    LDM k  = 0xD0 | k   (k = 0..15, load immediate into ACC)
    WRM = 0xE0,  WMP = 0xE1,  WRR = 0xE2
    WR0 = 0xE4,  WR1 = 0xE5,  WR2 = 0xE6,  WR3 = 0xE7
    SBM = 0xE8,  RDM = 0xE9,  RDR = 0xEA,  ADM = 0xEB
    RD0 = 0xEC,  RD1 = 0xED,  RD2 = 0xEE,  RD3 = 0xEF
    CLB = 0xF0,  CLC = 0xF1,  IAC = 0xF2,  CMC = 0xF3
    CMA = 0xF4,  RAL = 0xF5,  RAR = 0xF6,  TCC = 0xF7
    DAC = 0xF8,  TCS = 0xF9,  STC = 0xFA,  DAA = 0xFB
    KBP = 0xFC,  DCL = 0xFD

Two-byte instructions:

    JCN cond, addr12   -> [0x10 | cond, addr & 0xFF]
        cond nibble bits: bit3=test_carry, bit2=test_zero,
                          bit1=test_sign, bit0=invert condition
    FIM Pp, d8         -> [0x20 | (pair_index * 2), d8]
    JUN addr12         -> [0x40 | (addr >> 8), addr & 0xFF]
    JMS addr12         -> [0x50 | (addr >> 8), addr & 0xFF]
    ISZ Rn, addr8      -> [0x70 | n, addr8]

Register and pair names:

    R0 to R15 -> register index 0 to 15
    P0 to P7  -> pair index 0 to 7
    (P0 = R0:R1,  P1 = R2:R3,  P2 = R4:R5, ...)

ADD_IMM pseudo-instruction:

    ADD_IMM Rd, Rs, k  is a pseudo-instruction from the code generator.
    It expands to two real instructions: LDM k, ADD Rs.
    This computes ACC = k + Rs.

Byte-size calculation:

    instruction_size(mnemonic, operands) -> 0, 1, or 2
    Used in Pass 1 to advance the program counter without encoding.

Public API:

    encode_instruction(mnemonic, operands, symbols, pc) -> bytes
    instruction_size(mnemonic, operands) -> int
    AssemblerError  (exception class)
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class AssemblerError(Exception):
    """Raised when the assembler encounters an unrecoverable error.

    Examples:
    - Unknown mnemonic
    - Undefined label reference
    - Immediate value out of range (e.g. LDM 16 -- max is 15)
    - Address too large for instruction encoding
    """


# ---------------------------------------------------------------------------
# Register/pair name helpers
# ---------------------------------------------------------------------------

def _parse_register(name: str) -> int:
    """Parse a register name like ``R0``..``R15`` into an index 0..15.

    Args:
        name: Register name string, e.g. ``"R5"``.

    Returns:
        Integer index 0--15.

    Raises:
        AssemblerError: If the name is not a valid register.
    """
    upper = name.upper()
    if upper.startswith("R"):
        try:
            idx = int(upper[1:])
        except ValueError:
            raise AssemblerError(f"Invalid register name: {name!r}") from None
        if 0 <= idx <= 15:
            return idx
    raise AssemblerError(f"Invalid register name: {name!r}")


def _parse_pair(name: str) -> int:
    """Parse a register pair name like ``P0``..``P7`` into an index 0..7.

    Register pairs:
      P0 = R0:R1,  P1 = R2:R3,  P2 = R4:R5,  P3 = R6:R7
      P4 = R8:R9,  P5 = R10:R11, P6 = R12:R13, P7 = R14:R15

    Args:
        name: Pair name string, e.g. ``"P1"``.

    Returns:
        Integer index 0--7.

    Raises:
        AssemblerError: If the name is not a valid pair.
    """
    upper = name.upper()
    if upper.startswith("P"):
        try:
            idx = int(upper[1:])
        except ValueError:
            raise AssemblerError(f"Invalid register pair name: {name!r}") from None
        if 0 <= idx <= 7:
            return idx
    raise AssemblerError(f"Invalid register pair name: {name!r}")


def _parse_immediate(value: str) -> int:
    """Parse a numeric literal (decimal or hex with "0x" prefix).

    Args:
        value: String like ``"5"``, ``"0x4F"``, ``"255"``.

    Returns:
        Integer value.

    Raises:
        AssemblerError: If the string is not a valid numeric literal.
    """
    try:
        if value.lower().startswith("0x"):
            return int(value, 16)
        return int(value, 10)
    except ValueError:
        raise AssemblerError(f"Invalid numeric literal: {value!r}") from None


def _resolve_operand(operand: str, symbols: dict[str, int], pc: int) -> int:
    """Resolve an operand to an integer -- numeric literal, ``$``, or label.

    ``$`` means "the current program counter" -- used to write self-loops.

    Args:
        operand: Raw operand string from the lexer.
        symbols: Symbol table mapping label names to addresses.
        pc:      Current program counter (address of current instruction).

    Returns:
        Resolved integer address / value.

    Raises:
        AssemblerError: If it's a label not in ``symbols``.
    """
    if operand == "$":
        return pc
    if operand.lower().startswith("0x") or operand.lstrip("-").isdigit():
        return _parse_immediate(operand)
    # Must be a label reference.
    if operand not in symbols:
        raise AssemblerError(f"Undefined label: {operand!r}")
    return symbols[operand]


# ---------------------------------------------------------------------------
# One-byte fixed-opcode instructions (no operands)
# ---------------------------------------------------------------------------

# Maps mnemonic -> byte value for all zero-operand instructions.
_FIXED_OPCODES: dict[str, int] = {
    "NOP": 0x00,
    "HLT": 0x01,  # Simulator-only -- not in original 4004; maps to 0x01
    "WRM": 0xE0,
    "WMP": 0xE1,
    "WRR": 0xE2,
    "WR0": 0xE4,
    "WR1": 0xE5,
    "WR2": 0xE6,
    "WR3": 0xE7,
    "SBM": 0xE8,
    "RDM": 0xE9,
    "RDR": 0xEA,
    "ADM": 0xEB,
    "RD0": 0xEC,
    "RD1": 0xED,
    "RD2": 0xEE,
    "RD3": 0xEF,
    "CLB": 0xF0,
    "CLC": 0xF1,
    "IAC": 0xF2,
    "CMC": 0xF3,
    "CMA": 0xF4,
    "RAL": 0xF5,
    "RAR": 0xF6,
    "TCC": 0xF7,
    "DAC": 0xF8,
    "TCS": 0xF9,
    "STC": 0xFA,
    "DAA": 0xFB,
    "KBP": 0xFC,
    "DCL": 0xFD,
}


# ---------------------------------------------------------------------------
# Instruction size (needed by Pass 1 before symbols are known)
# ---------------------------------------------------------------------------

def instruction_size(mnemonic: str, operands: tuple[str, ...]) -> int:  # noqa: ANN001
    """Return the encoded byte size of an instruction.

    This is called during **Pass 1** when labels are being collected.
    We must know sizes *before* we know addresses, so this function
    must not look up the symbol table.

    Sizes:
      - Most single-register / I/O / no-operand instructions: **1 byte**
      - JCN, FIM, JUN, JMS, ISZ:                              **2 bytes**
      - ADD_IMM macro (LDM k + ADD Rn):                       **2 bytes**
      - FIN, JIN, SRC:                                        **1 byte**

    Args:
        mnemonic:  Uppercased opcode string.
        operands:  Tuple of operand strings (used for ADD_IMM only).

    Returns:
        Byte count: 1 or 2.

    Raises:
        AssemblerError: If the mnemonic is completely unknown.
    """
    if mnemonic in _FIXED_OPCODES:
        return 1
    if mnemonic in ("INC", "ADD", "SUB", "LD", "XCH", "BBL", "LDM"):
        return 1
    if mnemonic in ("SRC", "FIN", "JIN"):
        return 1
    if mnemonic in ("JCN", "FIM", "JUN", "JMS", "ISZ"):
        return 2
    if mnemonic == "ADD_IMM":
        # Expands to: LDM k (1) + ADD Rn (1) = 2 bytes
        return 2
    if mnemonic == "ORG":
        # Directive -- no bytes emitted
        return 0
    raise AssemblerError(f"Unknown mnemonic: {mnemonic!r}")


# ---------------------------------------------------------------------------
# Main encoder
# ---------------------------------------------------------------------------

def encode_instruction(
    mnemonic: str,
    operands: tuple[str, ...],
    symbols: dict[str, int],
    pc: int,
) -> bytes:
    """Encode one instruction into its binary representation.

    This is the heart of Pass 2.  For each instruction we:
    1. Validate operand count.
    2. Resolve any label/``$`` references via ``symbols``.
    3. Range-check immediates.
    4. Build and return the byte sequence.

    Args:
        mnemonic:  Uppercased opcode, e.g. ``"JUN"``.
        operands:  Tuple of operand strings from the lexer.
        symbols:   Symbol table built by Pass 1 (label -> address).
        pc:        Address of *this* instruction (used for ``$`` and
                   for JCN address verification).

    Returns:
        A ``bytes`` object containing 1--2 encoded bytes.

    Raises:
        AssemblerError: On any encoding error (unknown mnemonic,
                        undefined label, out-of-range value, etc.).

    Examples::

        encode_instruction("NOP", (), {}, 0)    # -> b'\\x00'
        encode_instruction("LDM", ("5",), {}, 0) # -> b'\\xD5'
        encode_instruction("JUN", ("0x042",), {}, 0) # -> b'\\x40\\x42'
    """
    # --- Zero-operand fixed instructions ----------------------------------------
    if mnemonic in _FIXED_OPCODES:
        _expect_operands(mnemonic, operands, 0)
        return bytes([_FIXED_OPCODES[mnemonic]])

    # --- ORG directive (emits nothing) ------------------------------------------
    if mnemonic == "ORG":
        return b""

    # --- LDM k   (load 4-bit immediate into ACC) --------------------------------
    if mnemonic == "LDM":
        _expect_operands(mnemonic, operands, 1)
        k = _resolve_operand(operands[0], symbols, pc)
        _check_range(mnemonic, k, 0, 15)
        return bytes([0xD0 | k])

    # --- BBL k   (branch back and load k into ACC -- return instruction) ---------
    if mnemonic == "BBL":
        _expect_operands(mnemonic, operands, 1)
        k = _resolve_operand(operands[0], symbols, pc)
        _check_range(mnemonic, k, 0, 15)
        return bytes([0xC0 | k])

    # --- INC Rn  (increment register) -------------------------------------------
    if mnemonic == "INC":
        _expect_operands(mnemonic, operands, 1)
        n = _parse_register(operands[0])
        return bytes([0x60 | n])

    # --- ADD Rn  (ACC = ACC + Rn + carry) ----------------------------------------
    if mnemonic == "ADD":
        _expect_operands(mnemonic, operands, 1)
        n = _parse_register(operands[0])
        return bytes([0x80 | n])

    # --- SUB Rn  (ACC = ACC − Rn − borrow) ---------------------------------------
    if mnemonic == "SUB":
        _expect_operands(mnemonic, operands, 1)
        n = _parse_register(operands[0])
        return bytes([0x90 | n])

    # --- LD  Rn  (load Rn into ACC) ----------------------------------------------
    if mnemonic == "LD":
        _expect_operands(mnemonic, operands, 1)
        n = _parse_register(operands[0])
        return bytes([0xA0 | n])

    # --- XCH Rn  (exchange ACC with Rn) ------------------------------------------
    if mnemonic == "XCH":
        _expect_operands(mnemonic, operands, 1)
        n = _parse_register(operands[0])
        return bytes([0xB0 | n])

    # --- SRC Pp  (set RAM character address from pair Pp) -----------------------
    #   Encoding: 0x2n+1 where n = 2*p
    #   P0->0x21, P1->0x23, P2->0x25, P3->0x27, ...
    if mnemonic == "SRC":
        _expect_operands(mnemonic, operands, 1)
        p = _parse_pair(operands[0])
        return bytes([0x20 | (2 * p + 1)])

    # --- FIN Pp  (fetch indirect: load (R0:R1) -> Pp) ---------------------------
    #   Encoding: 0x3n where n = 2*p
    #   P0->0x30, P1->0x32, P2->0x34, ...
    if mnemonic == "FIN":
        _expect_operands(mnemonic, operands, 1)
        p = _parse_pair(operands[0])
        return bytes([0x30 | (2 * p)])

    # --- JIN Pp  (jump indirect via pair Pp) ------------------------------------
    #   Encoding: 0x3n+1 where n = 2*p
    #   P0->0x31, P1->0x33, P2->0x35, ...
    if mnemonic == "JIN":
        _expect_operands(mnemonic, operands, 1)
        p = _parse_pair(operands[0])
        return bytes([0x30 | (2 * p + 1)])

    # --- FIM Pp, d8  (fetch immediate: load 8-bit value into pair Pp) ----------
    #   Byte 1: 0x2n where n = 2*p  -> P0->0x20, P1->0x22, ...
    #   Byte 2: the 8-bit immediate value
    if mnemonic == "FIM":
        _expect_operands(mnemonic, operands, 2)
        p = _parse_pair(operands[0])
        d8 = _resolve_operand(operands[1], symbols, pc)
        _check_range(mnemonic, d8, 0, 255)
        return bytes([0x20 | (2 * p), d8])

    # --- JCN cond, addr12  (conditional jump) -----------------------------------
    #   Byte 1: 0x1c  (c = condition nibble, 0..15)
    #   Byte 2: low 8 bits of target address
    #
    #   Condition nibble bits:
    #     bit 3 = test carry
    #     bit 2 = test zero
    #     bit 1 = test sign (negative)
    #     bit 0 = invert condition
    #
    #   The Intel 4004 JCN instruction only has an 8-bit address field.
    #   The high nibble of the target must be the same as the high nibble
    #   of the instruction that follows JCN (i.e. within the same 256-byte page).
    if mnemonic == "JCN":
        _expect_operands(mnemonic, operands, 2)
        cond = _resolve_operand(operands[0], symbols, pc)
        _check_range(mnemonic + " condition", cond, 0, 15)
        addr = _resolve_operand(operands[1], symbols, pc)
        _check_range(mnemonic + " address", addr, 0, 0xFFF)
        addr8 = addr & 0xFF
        return bytes([0x10 | cond, addr8])

    # --- JUN addr12  (unconditional jump) ----------------------------------------
    #   Byte 1: 0x4a  (a = high nibble of addr, bits 11--8)
    #   Byte 2: 0xbc  (bc = low byte of addr, bits 7--0)
    if mnemonic == "JUN":
        _expect_operands(mnemonic, operands, 1)
        addr = _resolve_operand(operands[0], symbols, pc)
        _check_range(mnemonic, addr, 0, 0xFFF)
        high_nibble = (addr >> 8) & 0xF
        low_byte = addr & 0xFF
        return bytes([0x40 | high_nibble, low_byte])

    # --- JMS addr12  (jump to subroutine) ----------------------------------------
    #   Same encoding as JUN but with high nibble 0x5.
    #   Byte 1: 0x5a, Byte 2: 0xbc
    if mnemonic == "JMS":
        _expect_operands(mnemonic, operands, 1)
        addr = _resolve_operand(operands[0], symbols, pc)
        _check_range(mnemonic, addr, 0, 0xFFF)
        high_nibble = (addr >> 8) & 0xF
        low_byte = addr & 0xFF
        return bytes([0x50 | high_nibble, low_byte])

    # --- ISZ Rn, addr8  (increment Rn; jump if not zero) -------------------------
    #   Byte 1: 0x7n (n = register index)
    #   Byte 2: 8-bit address to jump to if Rn ≠ 0 after increment
    if mnemonic == "ISZ":
        _expect_operands(mnemonic, operands, 2)
        n = _parse_register(operands[0])
        addr = _resolve_operand(operands[1], symbols, pc)
        _check_range(mnemonic + " address", addr, 0, 0xFF)
        return bytes([0x70 | n, addr & 0xFF])

    # --- ADD_IMM Rd, Rs, k  (pseudo-instruction: ACC = Rs + k) ------------------
    #   Expands to: LDM k, ADD Rs
    #   The destination register Rd is ignored at encoding time -- the
    #   result ends up in ACC, and the caller (codegen) follows with XCH Rd.
    if mnemonic == "ADD_IMM":
        _expect_operands(mnemonic, operands, 3)
        n = _parse_register(operands[1])  # source register
        k = _resolve_operand(operands[2], symbols, pc)
        _check_range(mnemonic + " immediate", k, 0, 15)
        # LDM k puts k into ACC; ADD Rn computes ACC = ACC + Rn + carry.
        return bytes([0xD0 | k, 0x80 | n])

    raise AssemblerError(f"Unknown mnemonic: {mnemonic!r}")


# ---------------------------------------------------------------------------
# Internal validation helpers
# ---------------------------------------------------------------------------

def _expect_operands(mnemonic: str, operands: tuple[str, ...], count: int) -> None:
    """Raise AssemblerError if operand count doesn't match expected count.

    Args:
        mnemonic:  Instruction name (for the error message).
        operands:  The actual operand tuple.
        count:     Expected number of operands.

    Raises:
        AssemblerError: If ``len(operands) != count``.
    """
    if len(operands) != count:
        raise AssemblerError(
            f"{mnemonic} expects {count} operand(s), got {len(operands)}: "
            f"{operands!r}"
        )


def _check_range(name: str, value: int, lo: int, hi: int) -> None:
    """Raise AssemblerError if value is outside [lo, hi].

    Args:
        name:  Descriptive name for the value (for the error message).
        value: The integer to check.
        lo:    Minimum allowed value (inclusive).
        hi:    Maximum allowed value (inclusive).

    Raises:
        AssemblerError: If ``value < lo or value > hi``.
    """
    if not (lo <= value <= hi):
        raise AssemblerError(
            f"{name} value {value} (0x{value:X}) is out of range "
            f"[{lo}, {hi}] (0x{lo:X}..0x{hi:X})"
        )
