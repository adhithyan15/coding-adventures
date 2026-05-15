"""Decoder6502 — combinational instruction decoder for the MOS 6502.

=== How the real 6502 decoder works ===

The 6502 uses a PLA (Programmable Logic Array) for instruction decode.
The PLA has product terms (AND rows) and sum terms (OR columns).  Each
opcode activates one or more product terms that drive the control ROM,
which then generates the microcode timing signals.

Chuck Peddle (lead designer) describes the PLA as the heart of the
chip: 130 AND terms × 21 product lines, driving 21 control signals.

In Python we model the primary group decode using AND/NOT gate logic
(matching the structural description in the 6502 programmer's reference),
then use a lookup table for the complete opcode → (mnemonic, mode) mapping.

=== Opcode structure ===

The 6502 opcode byte has a semi-regular structure called the "aaa bbb cc"
encoding (from Stall's 6502 reference):

    Bits 7–5  (aaa): operation within the class
    Bits 4–2  (bbb): addressing mode selector
    Bits 1–0  (cc) : instruction class

Class codes:
    01 → ALU group (ORA, AND, EOR, ADC, STA, LDA, CMP, SBC)
    10 → shift/load group (ASL, ROL, LSR, ROR, STX, LDX, DEC, INC)
    00 → miscellaneous (BIT, JMP, STY, LDY, CPY, CPX, branches, flags)

=== Gate-level group decode ===

Using the two LSBs as individual bits:
    cc_bit0 = (opcode >> 0) & 1
    cc_bit1 = (opcode >> 1) & 1

    is_class01 = AND(cc_bit0, NOT(cc_bit1))
    is_class10 = AND(NOT(cc_bit0), cc_bit1)
    is_class00 = AND(NOT(cc_bit0), NOT(cc_bit1))
    is_class11 = AND(cc_bit0, cc_bit1)  — mostly illegal in NMOS

This is a 2-to-4 decoder: 2 NOT + 4 AND = 6 gates.

=== Addressing mode codes (bbb field for class 01) ===

For cc=01 (ALU group):
    000 = (ind,X)  [INX]
    001 = zp       [ZP]
    010 = #imm     [IMM]
    011 = abs      [ABS]
    100 = (ind),Y  [INY]
    101 = zp,X     [ZPX]
    110 = abs,Y    [ABY]
    111 = abs,X    [ABX]

=== Branch instruction pattern ===

All 8 conditional branches follow the pattern xxy10000 where:
    xx = flag selector (00=N, 01=V, 10=C, 11=Z)
    y  = expected flag value (0=clear, 1=set)

This is decoded by the PLA as: bit4 = 0, bit3 = 0 for branches
(effectively all even-column instructions with bit0=0 in the top 4).
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT


def _bit(value: int, position: int) -> int:
    """Extract a single bit from an integer (models opcode bus wire).

    Args:
        value:    The opcode byte (0–255).
        position: Bit position (0 = LSB, 7 = MSB).

    Returns:
        0 or 1.

    Examples:
        >>> _bit(0b10110100, 7)
        1
        >>> _bit(0b10110100, 0)
        0
    """
    return (value >> position) & 1


# ── Addressing mode constants (shared with simulator.py) ─────────────────────

IMM = 0    # Immediate:          #$nn
ZP  = 1    # Zero Page:          $nn
ZPX = 2    # Zero Page,X:        $nn,X
ZPY = 3    # Zero Page,Y:        $nn,Y
ABS = 4    # Absolute:           $nnnn
ABX = 5    # Absolute,X:         $nnnn,X
ABY = 6    # Absolute,Y:         $nnnn,Y
INX = 7    # (Indirect,X):       ($nn,X)
INY = 8    # (Indirect),Y:       ($nn),Y
IMP = 9    # Implied
ACC = 10   # Accumulator
REL = 11   # Relative (branches)
IND = 12   # Absolute Indirect   (JMP only)

# Mnemonic for addressing mode (for display)
_MODE_NAMES: dict[int, str] = {
    IMM: "IMM", ZP: "ZP", ZPX: "ZPX", ZPY: "ZPY",
    ABS: "ABS", ABX: "ABX", ABY: "ABY",
    INX: "INX", INY: "INY",
    IMP: "IMP", ACC: "ACC", REL: "REL", IND: "IND",
}


@dataclass(frozen=True)
class DecodedInstruction:
    """Result of decoding a single opcode byte.

    Fields:
        opcode:   The raw opcode byte (0–255).
        mnemonic: Instruction name (e.g., "LDA", "ADC", "BEQ").
        mode:     Addressing mode code (one of the module-level constants).
        mode_name: Human-readable mode name (e.g., "ABS", "ZPX").
    """

    opcode: int
    mnemonic: str
    mode: int
    mode_name: str


# ── Full 151-opcode lookup table ─────────────────────────────────────────────
# All official NMOS 6502 opcodes.
# Format: opcode → (mnemonic, mode_constant)

_OPTABLE: dict[int, tuple[str, int]] = {
    # BRK / NOP
    0x00: ("BRK", IMP),
    0xEA: ("NOP", IMP),

    # LDA — load accumulator
    0xA9: ("LDA", IMM), 0xA5: ("LDA", ZP),  0xB5: ("LDA", ZPX),
    0xAD: ("LDA", ABS), 0xBD: ("LDA", ABX), 0xB9: ("LDA", ABY),
    0xA1: ("LDA", INX), 0xB1: ("LDA", INY),

    # LDX — load X
    0xA2: ("LDX", IMM), 0xA6: ("LDX", ZP),  0xB6: ("LDX", ZPY),
    0xAE: ("LDX", ABS), 0xBE: ("LDX", ABY),

    # LDY — load Y
    0xA0: ("LDY", IMM), 0xA4: ("LDY", ZP),  0xB4: ("LDY", ZPX),
    0xAC: ("LDY", ABS), 0xBC: ("LDY", ABX),

    # STA — store accumulator
    0x85: ("STA", ZP),  0x95: ("STA", ZPX), 0x8D: ("STA", ABS),
    0x9D: ("STA", ABX), 0x99: ("STA", ABY), 0x81: ("STA", INX),
    0x91: ("STA", INY),

    # STX — store X
    0x86: ("STX", ZP), 0x96: ("STX", ZPY), 0x8E: ("STX", ABS),

    # STY — store Y
    0x84: ("STY", ZP), 0x94: ("STY", ZPX), 0x8C: ("STY", ABS),

    # Register transfers (all implied)
    0xAA: ("TAX", IMP), 0xA8: ("TAY", IMP),
    0x8A: ("TXA", IMP), 0x98: ("TYA", IMP),
    0xBA: ("TSX", IMP), 0x9A: ("TXS", IMP),

    # Stack operations
    0x48: ("PHA", IMP), 0x68: ("PLA", IMP),
    0x08: ("PHP", IMP), 0x28: ("PLP", IMP),

    # ADC — add with carry
    0x69: ("ADC", IMM), 0x65: ("ADC", ZP),  0x75: ("ADC", ZPX),
    0x6D: ("ADC", ABS), 0x7D: ("ADC", ABX), 0x79: ("ADC", ABY),
    0x61: ("ADC", INX), 0x71: ("ADC", INY),

    # SBC — subtract with carry (inverted borrow)
    0xE9: ("SBC", IMM), 0xE5: ("SBC", ZP),  0xF5: ("SBC", ZPX),
    0xED: ("SBC", ABS), 0xFD: ("SBC", ABX), 0xF9: ("SBC", ABY),
    0xE1: ("SBC", INX), 0xF1: ("SBC", INY),

    # AND — bitwise AND
    0x29: ("AND", IMM), 0x25: ("AND", ZP),  0x35: ("AND", ZPX),
    0x2D: ("AND", ABS), 0x3D: ("AND", ABX), 0x39: ("AND", ABY),
    0x21: ("AND", INX), 0x31: ("AND", INY),

    # ORA — bitwise OR
    0x09: ("ORA", IMM), 0x05: ("ORA", ZP),  0x15: ("ORA", ZPX),
    0x0D: ("ORA", ABS), 0x1D: ("ORA", ABX), 0x19: ("ORA", ABY),
    0x01: ("ORA", INX), 0x11: ("ORA", INY),

    # EOR — bitwise exclusive OR
    0x49: ("EOR", IMM), 0x45: ("EOR", ZP),  0x55: ("EOR", ZPX),
    0x4D: ("EOR", ABS), 0x5D: ("EOR", ABX), 0x59: ("EOR", ABY),
    0x41: ("EOR", INX), 0x51: ("EOR", INY),

    # BIT — bit test
    0x24: ("BIT", ZP), 0x2C: ("BIT", ABS),

    # INC — increment memory
    0xE6: ("INC", ZP), 0xF6: ("INC", ZPX),
    0xEE: ("INC", ABS), 0xFE: ("INC", ABX),

    # INX / INY — increment index registers
    0xE8: ("INX", IMP), 0xC8: ("INY", IMP),

    # DEC — decrement memory
    0xC6: ("DEC", ZP), 0xD6: ("DEC", ZPX),
    0xCE: ("DEC", ABS), 0xDE: ("DEC", ABX),

    # DEX / DEY — decrement index registers
    0xCA: ("DEX", IMP), 0x88: ("DEY", IMP),

    # ASL — arithmetic shift left
    0x0A: ("ASL", ACC), 0x06: ("ASL", ZP),  0x16: ("ASL", ZPX),
    0x0E: ("ASL", ABS), 0x1E: ("ASL", ABX),

    # LSR — logical shift right
    0x4A: ("LSR", ACC), 0x46: ("LSR", ZP),  0x56: ("LSR", ZPX),
    0x4E: ("LSR", ABS), 0x5E: ("LSR", ABX),

    # ROL — rotate left through carry
    0x2A: ("ROL", ACC), 0x26: ("ROL", ZP),  0x36: ("ROL", ZPX),
    0x2E: ("ROL", ABS), 0x3E: ("ROL", ABX),

    # ROR — rotate right through carry
    0x6A: ("ROR", ACC), 0x66: ("ROR", ZP),  0x76: ("ROR", ZPX),
    0x6E: ("ROR", ABS), 0x7E: ("ROR", ABX),

    # CMP — compare accumulator
    0xC9: ("CMP", IMM), 0xC5: ("CMP", ZP),  0xD5: ("CMP", ZPX),
    0xCD: ("CMP", ABS), 0xDD: ("CMP", ABX), 0xD9: ("CMP", ABY),
    0xC1: ("CMP", INX), 0xD1: ("CMP", INY),

    # CPX — compare X register
    0xE0: ("CPX", IMM), 0xE4: ("CPX", ZP), 0xEC: ("CPX", ABS),

    # CPY — compare Y register
    0xC0: ("CPY", IMM), 0xC4: ("CPY", ZP), 0xCC: ("CPY", ABS),

    # Branches (all relative mode)
    0x90: ("BCC", REL), 0xB0: ("BCS", REL),
    0xF0: ("BEQ", REL), 0xD0: ("BNE", REL),
    0x10: ("BPL", REL), 0x30: ("BMI", REL),
    0x50: ("BVC", REL), 0x70: ("BVS", REL),

    # Jumps and subroutines
    0x4C: ("JMP", ABS), 0x6C: ("JMP", IND),
    0x20: ("JSR", ABS), 0x60: ("RTS", IMP), 0x40: ("RTI", IMP),

    # Flag instructions (all implied)
    0x18: ("CLC", IMP), 0x38: ("SEC", IMP),
    0xD8: ("CLD", IMP), 0xF8: ("SED", IMP),
    0x58: ("CLI", IMP), 0x78: ("SEI", IMP),
    0xB8: ("CLV", IMP),
}


class Decoder6502:
    """Combinational instruction decoder for the MOS 6502.

    Models the PLA-based decode logic of the real chip using AND/NOT
    gates for the primary group decode, with a lookup table for the
    full instruction set.

    The real 6502 PLA has ~130 AND product terms and 21 output lines.
    We represent the lookup table as the "programmed" PLA output.

    Usage::

        >>> dec = Decoder6502()
        >>> dec.decode(0xA9)
        DecodedInstruction(opcode=169, mnemonic='LDA', mode=0, mode_name='IMM')
        >>> dec.decode(0xBD)
        DecodedInstruction(opcode=189, mnemonic='LDA', mode=5, mode_name='ABX')
    """

    def decode(self, opcode: int) -> DecodedInstruction:
        """Decode a single opcode byte into mnemonic and addressing mode.

        Gate-level group detect (AND/NOT on cc bits) plus PLA lookup.

        The cc bits (opcode[1:0]) are extracted via AND gates:
          cc_bit0 = AND((opcode >> 0) & 1, 1)  — wire tap
          cc_bit1 = AND((opcode >> 1) & 1, 1)  — wire tap

        Primary group classification (2-to-4 decoder):
          class01 = AND(cc_bit0, NOT(cc_bit1))  — ALU group
          class10 = AND(NOT(cc_bit0), cc_bit1)  — shift/load group
          class00 = AND(NOT(cc_bit0), NOT(cc_bit1))  — misc group
          class11 = AND(cc_bit0, cc_bit1)       — mostly illegal

        The final decode is a lookup into the 151-entry opcode table,
        which represents the PLA's programmed product terms.

        Args:
            opcode: Instruction byte (0–255).

        Returns:
            DecodedInstruction with mnemonic, mode, and mode_name.

        Raises:
            ValueError: If the opcode is not in the legal 6502 instruction set.

        Examples:
            >>> Decoder6502().decode(0xA9)
            DecodedInstruction(opcode=169, mnemonic='LDA', mode=0, mode_name='IMM')
        """
        # ── Gate-level group decode (AND/NOT on cc bits) ──────────────────
        cc_bit0 = AND(_bit(opcode, 0), 1)   # Bit 0 of opcode
        cc_bit1 = AND(_bit(opcode, 1), 1)   # Bit 1 of opcode

        # 2-to-4 decoder: one-hot group signals
        _class01 = AND(cc_bit0, NOT(cc_bit1))    # ALU group
        _class10 = AND(NOT(cc_bit0), cc_bit1)    # Shift/store group
        _class00 = AND(NOT(cc_bit0), NOT(cc_bit1))  # Misc group
        _class11 = AND(cc_bit0, cc_bit1)         # Mostly illegal

        # ── PLA lookup (programmed product terms) ─────────────────────────
        if opcode not in _OPTABLE:
            raise ValueError(
                f"Illegal 6502 opcode {opcode:#04x} — not in NMOS instruction set"
            )

        mnemonic, mode = _OPTABLE[opcode]
        return DecodedInstruction(
            opcode=opcode,
            mnemonic=mnemonic,
            mode=mode,
            mode_name=_MODE_NAMES[mode],
        )

    def is_branch(self, opcode: int) -> bool:
        """Detect whether opcode is a conditional branch instruction.

        Branch opcodes all follow the pattern 0bxxy10000 (bit 4 = 1,
        bits 3,2,1,0 = 0000).  This is decoded by checking:
          bit4 = (opcode >> 4) & 1  → must be 1
          AND with pattern check via NOT/AND gates.

        Branches: BPL BMI BVC BVS BCC BCS BNE BEQ
        Opcodes:  $10 $30 $50 $70 $90 $B0 $D0 $F0

        Args:
            opcode: Instruction byte.

        Returns:
            True if this is a branch instruction.
        """
        # All branch opcodes share the pattern: low nibble = 0x00
        # and bit 4 = 0 of high nibble is 1 (high nibble is odd).
        low_nibble_zero = AND(
            AND(NOT(_bit(opcode, 0)), NOT(_bit(opcode, 1))),
            AND(NOT(_bit(opcode, 2)), NOT(_bit(opcode, 3))),
        )
        bit4_set = AND(_bit(opcode, 4), 1)
        return bool(AND(low_nibble_zero, bit4_set))

    def is_legal(self, opcode: int) -> bool:
        """Return True if the opcode is in the legal NMOS 6502 instruction set.

        Args:
            opcode: Instruction byte (0–255).

        Returns:
            True if legal, False if illegal/undocumented.
        """
        return opcode in _OPTABLE
