"""encoder.py -- Encode a single Intel 8008 instruction into bytes.

Intel 8008 Instruction Encoding Reference
------------------------------------------

The Intel 8008 (1972) is an 8-bit CPU with a 14-bit address bus.  Each
instruction is 1, 2, or 3 bytes.  The first byte encodes the opcode; for
2-byte instructions the second byte is an 8-bit immediate; for 3-byte
instructions bytes 2 and 3 encode a 14-bit address (low byte first, then
the high 6 bits in the high 6 bits of the third byte).

Register Codes
--------------

All register operands in the 8008 are encoded as 3-bit values:

    B=0, C=1, D=2, E=3, H=4, L=5, M=6 (memory at H:L), A=7

The register code appears in different bit positions depending on the
instruction group.

Address Encoding (3-byte instructions)
---------------------------------------

For JMP, CAL, and conditional jump/call instructions:

    Byte 2: lo(addr)  = addr & 0xFF          (low 8 bits)
    Byte 3: hi6(addr) = (addr >> 8) & 0x3F   (high 6 bits, in bits 7:2)

The full 14-bit address is reconstructed as:
    addr = (byte3 << 8) | byte2

Wait -- re-reading the spec:

    JMP a14:  0x44, lo(a14), hi6(a14)

Where hi6(a14) = (a14 >> 8) & 0x3F and the value is stored in bits [7:2]
of byte 3? No -- the spec says hi6(a14) directly, meaning the value IS
(a14 >> 8) & 0x3F placed in the low 6 bits of byte 3.

So for address 0x1234:
    byte 2 = 0x34  (low 8 bits)
    byte 3 = 0x12  (high 6 bits of 14-bit addr = 0x12 since 0x1234 >> 8 = 0x12)

This is the "little-endian" address encoding:
    addr = byte2 | (byte3 << 8)

hi(symbol) in MVI instructions
--------------------------------

The code generator emits ``hi(sym)`` and ``lo(sym)`` as operands to ``MVI``
instructions for loading 14-bit static variable addresses into H:L:

    hi(addr) = (addr >> 8) & 0x3F   (high 6 bits of 14-bit address)
    lo(addr) = addr & 0xFF           (low 8 bits)

These expressions are resolved in the encoder using the symbol table.

Instruction Groups
------------------

**Group 00** (opcode bits 7:6 = 00):
  MVI r, d8:  (r<<3) | 0x06, d8   (2 bytes)
  INR r:       r<<3                 (0x00..0x38 for B..A)
  DCR r:      (r<<3) | 0x01
  RLC:         0x02
  RRC:         0x0A
  RAL:         0x12
  RAR:         0x1A
  RFC / RET:   0x03   (return if carry false -- standard unconditional return)
  RTC:         0x07   (return if carry true)

Return instruction encoding: ``00 CCC T11``
  CCC = condition code (bits[5:3]): 0=CY, 1=Z, 2=S, 3=P
  T   = sense bit (bit 2): 0=if-false, 1=if-true
  bits[1:0] = 11 (identifies as return)

  RFC / RET:  0x03   (CCC=0, T=0, bits=00_000_011)
  RFZ:        0x0B   (CCC=1, T=0, bits=00_001_011)
  RFS:        0x13   (CCC=2, T=0, bits=00_010_011)
  RFP:        0x1B   (CCC=3, T=0, bits=00_011_011)
  RTC:        0x07   (CCC=0, T=1, bits=00_000_111)
  RTZ:        0x0F   (CCC=1, T=1, bits=00_001_111)
  RTS:        0x17   (CCC=2, T=1, bits=00_010_111)
  RTP:        0x1F   (CCC=3, T=1, bits=00_011_111)
  HLT:        0xFF   (alternate halt encoding)
  RLC:        0x02
  RRC:        0x0A
  RAL:        0x12
  RAR:        0x1A

**Group 01** (opcode bits 7:6 = 01):
  MOV dst, src:  0x40 | (dst<<3) | src
  IN p:          0x41 | (p<<3)   for p = 0..7
  JMP a14:       0x7C, lo, hi6   (unconditional jump)
  CAL a14:       0x7E, lo, hi6   (unconditional call)

  Conditional jump/call encoding: ``01 CCC T00`` (jump) / ``01 CCC T10`` (call)
    CCC = condition code: 0=CY, 1=Z, 2=S, 3=P
    T   = sense bit: 0=if-false, 1=if-true
    bits[1:0] = 00 for jump, 10 for call

  JFC a14:       0x40, lo, hi6   (jump if carry false)
  JTC a14:       0x44, lo, hi6   (jump if carry true)
  JFZ a14:       0x48, lo, hi6
  JTZ a14:       0x4C, lo, hi6
  JFS a14:       0x50, lo, hi6
  JTS a14:       0x54, lo, hi6
  JFP a14:       0x58, lo, hi6
  JTP a14:       0x5C, lo, hi6
  CFC a14:       0x42, lo, hi6   (call if carry false)
  CTC a14:       0x46, lo, hi6   (call if carry true)
  CFZ a14:       0x4A, lo, hi6
  CTZ a14:       0x4E, lo, hi6

**Group 10** (opcode bits 7:6 = 10) -- ALU register (all 1 byte):
  ADD r:  0x80 | r
  ADC r:  0x88 | r
  SUB r:  0x90 | r
  SBB r:  0x98 | r
  ANA r:  0xA0 | r
  XRA r:  0xA8 | r
  ORA r:  0xB0 | r
  CMP r:  0xB8 | r

**Group 11** (opcode bits 7:6 = 11) -- ALU immediate (2 bytes):
  Encoding: 11 OOO 100, d8
    OOO = operation code (bits[5:3] = ddd field)
    bits[2:0] = 100 (identifies as ALU immediate, sss=4)

  ADI d8:  0xC4, d8   (OOO=000: ADD immediate)
  ACI d8:  0xCC, d8   (OOO=001: ADD with Carry immediate)
  SUI d8:  0xD4, d8   (OOO=010: SUBtract immediate)
  SBI d8:  0xDC, d8   (OOO=011: SuBtract with borrow Immediate)
  ANI d8:  0xE4, d8   (OOO=100: AND immediate)
  XRI d8:  0xEC, d8   (OOO=101: XOR immediate)
  ORI d8:  0xF4, d8   (OOO=110: OR immediate)
  CPI d8:  0xFC, d8   (OOO=111: ComPare immediate)

**OUT p** (Group 00 extended, 1 byte):
  The simulator decodes OUT from group=00, sss=010, ddd>3 (i.e., ddd in 4..7).
  Port number is extracted as: port = (opcode >> 1) & 0x1F
  Formula: opcode = p << 1   (port number in bits[5:1], always even)

  Because sss=010 requires bit1=1 (odd opcode>>1), only ports where (p<<1)
  has sss=010 AND ddd>3 are actually handled by the simulator:
    OUT 17 = 0x22   (ddd=4, sss=010: simulator-compatible)
    OUT 21 = 0x2A   (ddd=5, sss=010: simulator-compatible)
  Ports 0-16, 18-20, 22-23 produce opcodes that may conflict with other
  instructions. This is a known simulator limitation.
  See intel8008_simulator for OUT port encoding discussion.

Public API:

    encode_instruction(mnemonic, operands, symbols, pc) -> bytes
    instruction_size(mnemonic, operands) -> int
    AssemblerError  (exception class)
"""

from __future__ import annotations

import re

# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class AssemblerError(Exception):
    """Raised when the assembler encounters an unrecoverable error.

    Examples:
    - Unknown mnemonic
    - Undefined label reference
    - Immediate value out of range (e.g. MVI A, 256 -- max is 255)
    - Address too large for 14-bit address space
    - Port number out of range
    """


# ---------------------------------------------------------------------------
# Register name helper
# ---------------------------------------------------------------------------

# Register encoding: B=0, C=1, D=2, E=3, H=4, L=5, M=6, A=7
_REG_CODES: dict[str, int] = {
    "B": 0, "C": 1, "D": 2, "E": 3, "H": 4, "L": 5, "M": 6, "A": 7,
}


def _parse_register(name: str) -> int:
    """Parse an 8008 register name into its 3-bit code (0–7).

    Valid register names: A, B, C, D, E, H, L, M.

    Args:
        name: Register name string, case-insensitive.

    Returns:
        3-bit integer code 0–7.

    Raises:
        AssemblerError: If the name is not a valid 8008 register.
    """
    upper = name.upper()
    if upper not in _REG_CODES:
        raise AssemblerError(
            f"Invalid 8008 register name: {name!r}. "
            f"Valid registers are: A, B, C, D, E, H, L, M"
        )
    return _REG_CODES[upper]


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


# Matches hi(symbol_name) or lo(symbol_name)
_HI_LO_RE = re.compile(r"^(hi|lo)\(([A-Za-z_][A-Za-z0-9_]*)\)$", re.IGNORECASE)


def _resolve_operand(operand: str, symbols: dict[str, int], pc: int) -> int:
    """Resolve an operand to an integer -- numeric literal, ``$``, label, or hi()/lo().

    Handles four kinds of operand:
    - ``$``           -- the current program counter
    - ``0x...``       -- hexadecimal literal
    - decimal digit   -- decimal literal
    - ``hi(sym)``     -- high 6 bits of sym's 14-bit address: (addr >> 8) & 0x3F
    - ``lo(sym)``     -- low 8 bits of sym's address: addr & 0xFF
    - identifier      -- a label reference from the symbol table

    Args:
        operand: Raw operand string from the lexer.
        symbols: Symbol table mapping label names to addresses.
        pc:      Current program counter (address of current instruction).

    Returns:
        Resolved integer address / value.

    Raises:
        AssemblerError: If it's a label not in ``symbols`` or an invalid expression.
    """
    if operand == "$":
        return pc

    # hi(sym) / lo(sym) expressions for 14-bit address halves
    m = _HI_LO_RE.match(operand)
    if m:
        kind = m.group(1).lower()
        sym = m.group(2)
        if sym not in symbols:
            raise AssemblerError(f"Undefined label in {operand!r}: {sym!r}")
        addr = symbols[sym]
        if kind == "hi":
            # High 6 bits of the 14-bit address.
            # These go into H register.  hi(addr) = (addr >> 8) & 0x3F
            return (addr >> 8) & 0x3F
        else:
            # Low 8 bits.  lo(addr) = addr & 0xFF
            return addr & 0xFF

    # Numeric literals
    if operand.lower().startswith("0x") or operand.lstrip("-").isdigit():
        return _parse_immediate(operand)

    # Must be a label reference.
    if operand not in symbols:
        raise AssemblerError(f"Undefined label: {operand!r}")
    return symbols[operand]


# ---------------------------------------------------------------------------
# Fixed one-byte instructions (no operands)
# ---------------------------------------------------------------------------

# Return instruction encoding: 00 CCC T11
#   CCC = condition code (bits[5:3]): 0=CY, 1=Z, 2=S, 3=P
#   T   = sense bit (bit 2): 0=if-false (RF*), 1=if-true (RT*)
#   bits[1:0] = 11
#
# Truth table:
#   RFC = 00_000_0_11 = 0x03   (Carry False → carry=0 → always returns in practice)
#   RTC = 00_000_1_11 = 0x07   (Carry True)
#   RFZ = 00_001_0_11 = 0x0B   (Zero  False)
#   RTZ = 00_001_1_11 = 0x0F   (Zero  True)
#   RFS = 00_010_0_11 = 0x13   (Sign  False)
#   RTS = 00_010_1_11 = 0x17   (Sign  True)
#   RFP = 00_011_0_11 = 0x1B   (Parity False)
#   RTP = 00_011_1_11 = 0x1F   (Parity True)
#
# Historical note: early versions of this assembler had RFC=0x07 (actually RTC),
# RTC=0x0F (actually RTZ), etc. — all "true-sense" returns were shifted by one
# condition code. Fixed to match the simulator's 00_CCC_T_11 decoding.
_FIXED_OPCODES: dict[str, int] = {
    # Rotations (Group 00)
    "RLC": 0x02,
    "RRC": 0x0A,
    "RAL": 0x12,
    "RAR": 0x1A,
    # Conditional returns (Group 00): encoding 00 CCC T11
    # RFC = Return if Carry False = unconditional return in practice (CY is 0 after ALU)
    "RFC": 0x03,
    "RET": 0x03,   # synonym for RFC in the 8008 codegen
    "RFZ": 0x0B,
    "RFS": 0x13,
    "RFP": 0x1B,
    "RTC": 0x07,
    "RTZ": 0x0F,
    "RTS": 0x17,
    "RTP": 0x1F,
    # Halt
    "HLT": 0xFF,
}

# ALU register operations (Group 10): op r = base_opcode | reg_code
_ALU_REG_BASE: dict[str, int] = {
    "ADD": 0x80,
    "ADC": 0x88,
    "SUB": 0x90,
    "SBB": 0x98,
    "ANA": 0xA0,
    "XRA": 0xA8,
    "ORA": 0xB0,
    "CMP": 0xB8,
}

# ALU immediate operations (Group 11): op d8 = [opcode, d8]
# Encoding: 11 OOO 100  where OOO = operation (0=ADD, 1=ADC, 2=SUB, 3=SBB,
#                                               4=ANA, 5=XRA, 6=ORA, 7=CMP)
# sss = 100 = 4 distinguishes these from MOV (group=11 is otherwise unused).
#
# Historical note: early versions had ADI=0x04, ACI=0x0C, etc. (group=00 not
# group=11), causing ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI to be decoded as INR
# or unknown instructions instead of ALU-immediate.
_ALU_IMM_OPCODES: dict[str, int] = {
    "ADI": 0xC4,   # 11_000_100
    "ACI": 0xCC,   # 11_001_100
    "SUI": 0xD4,   # 11_010_100
    "SBI": 0xDC,   # 11_011_100
    "ANI": 0xE4,   # 11_100_100
    "XRI": 0xEC,   # 11_101_100
    "ORI": 0xF4,   # 11_110_100
    "CPI": 0xFC,   # 11_111_100
}

# 3-byte jump/call instructions: mnemonic → first opcode byte
# Address bytes follow: lo8(addr), hi6(addr)
#
# Unconditional jump/call are special opcodes (ddd=7, sense=1):
#   JMP = 0x7C = 01_111_100   (unconditional jump)
#   CAL = 0x7E = 01_111_110   (unconditional call)
#
# Conditional jumps: 01 CCC T00   (bits[1:0] = 00)
# Conditional calls: 01 CCC T10   (bits[1:0] = 10)
#   CCC = condition code: 0=CY, 1=Z, 2=S, 3=P
#   T   = sense bit (bit 2): 0=if-false (JF*/CF*), 1=if-true (JT*/CT*)
#
# Historical note: early versions had JMP=0x44 (actually JTC), CAL=0x46
# (actually CTC). All "true-sense" jumps/calls were wrong because the sense
# bit (T) was not accounted for. Fixed to match simulator's 01_CCC_T_{00,10}
# decoding. JMP=0x7C and CAL=0x7E are validated against simulator test cases.
_JUMP_CALL_OPCODES: dict[str, int] = {
    # Unconditional (special opcodes, not the 01_CCC_T_xx pattern)
    "JMP": 0x7C,   # 01_111_100 — simulator hardcodes opcode==0x7C as JMP
    "CAL": 0x7E,   # 01_111_110 — simulator hardcodes opcode==0x7E as CAL
    # Conditional jumps (01 CCC T00): bits[1:0]=00, sense in bit 2
    "JFC": 0x40,   # CCC=0, T=0 — jump if carry false
    "JTC": 0x44,   # CCC=0, T=1 — jump if carry true
    "JFZ": 0x48,   # CCC=1, T=0
    "JTZ": 0x4C,   # CCC=1, T=1
    "JFS": 0x50,   # CCC=2, T=0
    "JTS": 0x54,   # CCC=2, T=1
    "JFP": 0x58,   # CCC=3, T=0
    "JTP": 0x5C,   # CCC=3, T=1
    # Conditional calls (01 CCC T10): bits[1:0]=10, sense in bit 2
    "CFC": 0x42,   # CCC=0, T=0 — call if carry false
    "CTC": 0x46,   # CCC=0, T=1 — call if carry true
    "CFZ": 0x4A,   # CCC=1, T=0
    "CTZ": 0x4E,   # CCC=1, T=1
    "CFS": 0x52,   # CCC=2, T=0
    "CTS": 0x56,   # CCC=2, T=1
    "CFP": 0x5A,   # CCC=3, T=0
    "CTP": 0x5E,   # CCC=3, T=1
}


# ---------------------------------------------------------------------------
# Instruction size (needed by Pass 1 before symbols are known)
# ---------------------------------------------------------------------------

def instruction_size(mnemonic: str, operands: tuple[str, ...]) -> int:  # noqa: ANN001
    """Return the encoded byte size of an instruction.

    This is called during **Pass 1** when labels are being collected.
    We must know sizes *before* we know addresses, so this function
    must not look up the symbol table.

    Sizes (from the OCT01 spec):
      - Fixed single-byte instructions (RFC/RET/RLC/RRC/RAL/RAR/HLT/etc.): **1 byte**
      - ALU register ops (ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP) + register: **1 byte**
      - MOV dst, src:                                                       **1 byte**
      - IN p, OUT p:                                                         **1 byte**
      - INR r, DCR r:                                                        **1 byte**
      - MVI r, d8 + ALU immediate (ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI):      **2 bytes**
      - JMP/CAL and all conditional jumps/calls:                             **3 bytes**

    Args:
        mnemonic:  Uppercased opcode string.
        operands:  Tuple of operand strings (unused, but kept for API symmetry).

    Returns:
        Byte count: 1, 2, or 3.

    Raises:
        AssemblerError: If the mnemonic is completely unknown.
    """
    if mnemonic in _FIXED_OPCODES:
        return 1
    if mnemonic in _ALU_REG_BASE:
        return 1
    if mnemonic in _ALU_IMM_OPCODES:
        return 2
    if mnemonic in _JUMP_CALL_OPCODES:
        return 3
    if mnemonic == "MOV":
        return 1
    if mnemonic == "MVI":
        return 2
    if mnemonic in ("INR", "DCR"):
        return 1
    if mnemonic in ("IN", "OUT"):
        return 1
    if mnemonic == "RST":
        return 1
    if mnemonic == "ORG":
        # Directive -- no bytes emitted.
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
    """Encode one Intel 8008 instruction into its binary representation.

    This is the heart of Pass 2.  For each instruction we:
    1. Validate operand count.
    2. Resolve any label/``$``/hi()/lo() references via ``symbols``.
    3. Range-check immediates.
    4. Build and return the byte sequence.

    Args:
        mnemonic:  Uppercased opcode, e.g. ``"MVI"``.
        operands:  Tuple of operand strings from the lexer.
        symbols:   Symbol table built by Pass 1 (label -> address).
        pc:        Address of *this* instruction (used for ``$``).

    Returns:
        A ``bytes`` object containing 1–3 encoded bytes.

    Raises:
        AssemblerError: On any encoding error (unknown mnemonic,
                        undefined label, out-of-range value, etc.).

    Examples::

        encode_instruction("HLT", (), {}, 0)                  # -> b'\\xff'
        encode_instruction("MVI", ("B", "42"), {}, 0)         # -> b'\\x06\\x2a'
        encode_instruction("MOV", ("A", "B"), {}, 0)          # -> b'\\x78'
        encode_instruction("ADD", ("C",), {}, 0)              # -> b'\\x81'
        encode_instruction("JMP", ("0x000a",), {}, 0)         # -> b'\\x7c\\x0a\\x00'
    """
    # --- ORG directive (emits nothing) -----------------------------------------
    if mnemonic == "ORG":
        return b""

    # --- Fixed one-byte instructions (no operands) -----------------------------
    if mnemonic in _FIXED_OPCODES:
        _expect_operands(mnemonic, operands, 0)
        return bytes([_FIXED_OPCODES[mnemonic]])

    # --- MOV dst, src  (Group 01: 0x40 | dst<<3 | src) -------------------------
    #
    # The MOV instruction copies one register to another.  Both dst and src are
    # 3-bit register codes.  The opcode is:
    #   0x40 | (dst_code << 3) | src_code
    #
    # Note: MOV M, M (dst=6, src=6) is an alternative encoding of HLT on the
    # real 8008.  The assembler accepts it but does not special-case it -- the
    # value 0x76 is emitted and the simulator treats it as HLT.
    #
    # Example: MOV A, B = 0x40 | (7<<3) | 0 = 0x40 | 0x38 | 0 = 0x78
    if mnemonic == "MOV":
        _expect_operands(mnemonic, operands, 2)
        dst = _parse_register(operands[0])
        src = _parse_register(operands[1])
        return bytes([0x40 | (dst << 3) | src])

    # --- MVI r, d8  (Group 00: (r<<3) | 0x06, d8) ------------------------------
    #
    # Move Immediate: load an 8-bit constant into register r.
    # The first opcode byte encodes the destination register in bits 5:3:
    #   (r_code << 3) | 0x06
    #
    # Example: MVI B, 42 → [0x06, 0x2A]   (0<<3)|0x06 = 0x06; 42 = 0x2A
    # Example: MVI H, 0x20 → [0x26, 0x20]  (4<<3)|0x06 = 0x26
    #
    # The operand may be a plain immediate (42, 0x2A) or an expression:
    #   hi(symbol_name) = (symbol_addr >> 8) & 0x3F
    #   lo(symbol_name) = symbol_addr & 0xFF
    # These are resolved by _resolve_operand.
    if mnemonic == "MVI":
        _expect_operands(mnemonic, operands, 2)
        r = _parse_register(operands[0])
        d8 = _resolve_operand(operands[1], symbols, pc)
        _check_range(mnemonic + " immediate", d8, 0, 255)
        opcode = (r << 3) | 0x06
        return bytes([opcode, d8])

    # --- INR r  (Group 00: r<<3) ------------------------------------------------
    #
    # Increment register r.  Does NOT affect the CY (carry) flag.
    # Encoding: r_code << 3
    # Example: INR B → 0x00  (0<<3 = 0)
    # Example: INR D → 0x10  (2<<3 = 16 = 0x10)
    if mnemonic == "INR":
        _expect_operands(mnemonic, operands, 1)
        r = _parse_register(operands[0])
        return bytes([r << 3])

    # --- DCR r  (Group 00: r<<3 | 0x01) ----------------------------------------
    #
    # Decrement register r.  Does NOT affect the CY flag.
    # Encoding: (r_code << 3) | 0x01
    # Example: DCR B → 0x01  ((0<<3) | 1 = 1)
    # Example: DCR C → 0x09  ((1<<3) | 1 = 9 = 0x09)
    if mnemonic == "DCR":
        _expect_operands(mnemonic, operands, 1)
        r = _parse_register(operands[0])
        return bytes([(r << 3) | 0x01])

    # --- RST n  (Group 00: n<<3 | 0x05) ----------------------------------------
    #
    # Restart: push PC onto the stack and jump to address n×8 (page 0).
    # Encoding: (n << 3) | 0x05
    # n must be 0–7.
    if mnemonic == "RST":
        _expect_operands(mnemonic, operands, 1)
        n = _resolve_operand(operands[0], symbols, pc)
        _check_range("RST n", n, 0, 7)
        return bytes([(n << 3) | 0x05])

    # --- ALU register operations (Group 10) -------------------------------------
    #
    # All of: ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP
    # Encoding: base_opcode | reg_code
    # Example: ADD B → 0x80 | 0 = 0x80
    # Example: CMP C → 0xB8 | 1 = 0xB9
    # Example: ORA A → 0xB0 | 7 = 0xB7
    if mnemonic in _ALU_REG_BASE:
        _expect_operands(mnemonic, operands, 1)
        r = _parse_register(operands[0])
        return bytes([_ALU_REG_BASE[mnemonic] | r])

    # --- ALU immediate operations (Group 11) ------------------------------------
    #
    # All of: ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI
    # 2-byte instruction: [opcode_byte, d8]
    # Example: ADI 5 → [0x04, 0x05]
    # Example: CPI 0 → [0x3C, 0x00]
    if mnemonic in _ALU_IMM_OPCODES:
        _expect_operands(mnemonic, operands, 1)
        d8 = _resolve_operand(operands[0], symbols, pc)
        _check_range(mnemonic + " immediate", d8, 0, 255)
        return bytes([_ALU_IMM_OPCODES[mnemonic], d8])

    # --- IN p  (Group 01: 0x41 | p<<3) ------------------------------------------
    #
    # Read 8-bit input from port p (p = 0–7) into the accumulator.
    # Encoding: 0x41 | (p << 3)
    # p=0 → 0x41, p=1 → 0x49, p=2 → 0x51, ..., p=7 → 0x79
    if mnemonic == "IN":
        _expect_operands(mnemonic, operands, 1)
        p = _resolve_operand(operands[0], symbols, pc)
        _check_range("IN port", p, 0, 7)
        return bytes([0x41 | (p << 3)])

    # --- OUT p  (Group 00 extended: p << 1) ------------------------------------
    #
    # Write accumulator to output port p (p = 0–23).
    # Encoding: opcode = p << 1
    # The simulator detects OUT via: group=00, sss=010, ddd>3,
    # with port=(opcode>>1)&0x1F.  Only ports 17 and 21 produce opcodes that
    # satisfy both sss=010 AND ddd>3 AND port in 0..23:
    #   OUT 17 = 0x22   (port 17 << 1; sss=010, ddd=4)
    #   OUT 21 = 0x2A   (port 21 << 1; sss=010, ddd=5)
    # Other port numbers produce opcodes that conflict with rotate, MVI, or
    # INR/DCR instructions and are not correctly decoded by the simulator.
    # This is a known simulator limitation.
    if mnemonic == "OUT":
        _expect_operands(mnemonic, operands, 1)
        p = _resolve_operand(operands[0], symbols, pc)
        _check_range("OUT port", p, 0, 23)
        return bytes([p << 1])

    # --- 3-byte jump and call instructions (Group 01) ---------------------------
    #
    # JMP, CAL, and all conditional jumps/calls (JFC, JTC, JFZ, JTZ, etc.).
    # Format: [opcode_byte, lo8(addr), hi6(addr)]
    # Where:
    #   lo8(addr)  = addr & 0xFF          (low 8 bits)
    #   hi6(addr)  = (addr >> 8) & 0x3F  (high 6 bits of 14-bit address)
    #
    # The full 14-bit address is reconstructed by the CPU as:
    #   addr = (byte3 << 8) | byte2
    #   (byte3 contains the hi6 value, byte2 contains lo8)
    #
    # Example: JMP 0x1234 → [0x44, 0x34, 0x12]
    # Example: JTZ 0x000A → [0x68, 0x0A, 0x00]
    if mnemonic in _JUMP_CALL_OPCODES:
        _expect_operands(mnemonic, operands, 1)
        addr = _resolve_operand(operands[0], symbols, pc)
        _check_range(mnemonic + " address", addr, 0, 0x3FFF)
        lo8 = addr & 0xFF
        hi6 = (addr >> 8) & 0x3F
        return bytes([_JUMP_CALL_OPCODES[mnemonic], lo8, hi6])

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
