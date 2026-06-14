"""DecoderZ80 — combinational instruction decoder for the Zilog Z80.

=== How the real Z80 decoder works ===

The Z80 uses a PLA (Programmable Logic Array) rather than the simpler
AND/OR ROM decoder of the 8080. The PLA has product terms (AND rows)
and sum terms (OR columns). Each instruction maps to one or more product
terms that activate control signals.

In Python we model the primary group decode as AND/NOT gate logic
(matching the Z80 manual's structural description), then use pattern
matching for the individual instruction decode.

=== Opcode structure ===

Z80 opcodes are structured identically to Intel 8080 (for compatibility):

    Bit 7  Bit 6  |  Bit 5  Bit 4  Bit 3  |  Bit 2  Bit 1  Bit 0
    ────────────────────────────────────────────────────────────────
       group       |     dst / alu_op       |       src

Group decode (bits 7–6):
    00 → group_00: LD rr/misc (loads, increments, rotates, relative jumps)
    01 → group_01: LD r,r (register to register); 0x76 = HALT (HLT)
    10 → group_10: ALU A,r (ADD/ADC/SUB/SBC/AND/XOR/OR/CP)
    11 → group_11: misc (RET, JP, CALL, PUSH/POP, RST, immediate ALU)

=== Prefixed instructions ===

Four prefix bytes extend the instruction set:
  CB prefix (0xCB): rotate/shift/bit manipulation
  ED prefix (0xED): 16-bit arithmetic, block ops, IN/OUT variants, NEG
  DD prefix (0xDD): IX-indexed (replaces HL with IX+d in many instructions)
  FD prefix (0xFD): IY-indexed (replaces HL with IY+d)

DDCB and FDCB: two-byte prefix for bit ops on (IX+d)/(IY+d).

=== Gate-level group decode ===

Using the two MSBs as individual bits extracted via AND/NOT:

    bit7 = (opcode >> 7) & 1    (MSB)
    bit6 = (opcode >> 6) & 1

    is_group00 = AND(NOT(bit7), NOT(bit6))   # 00xxxxxx
    is_group01 = AND(NOT(bit7), bit6)         # 01xxxxxx
    is_group10 = AND(bit7, NOT(bit6))         # 10xxxxxx
    is_group11 = AND(bit7, bit6)              # 11xxxxxx

Each produces exactly one 1 and three 0s — a one-hot encoding.
Total: 2 NOT + 4 AND = 6 gates for the primary decode.

=== 8-bit register codes ===

    000 = B    001 = C    010 = D    011 = E
    100 = H    101 = L    110 = (HL) pseudo   111 = A

=== ALU operation codes (group_10, bits 5–3) ===

    000 = ADD A,r    001 = ADC A,r    010 = SUB r    011 = SBC A,r
    100 = AND r      101 = XOR r      110 = OR r     111 = CP r

These directly match the ALUZ80 operation codes.
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
    """
    return (value >> position) & 1


@dataclass(frozen=True)
class DecoderOutput:
    """Control signals produced by the Z80 combinational decoder.

    These are the electrical signals the Z80 decoder outputs simultaneously.
    In hardware they are wires; here they are fields.

    Fields
    ------
    op_group:       Primary group (0–3, from bits 7–6).
    dst:            Destination register code (bits 5–3), 0–7.
    src:            Source register code (bits 2–0), 0–7.
    alu_op:         ALU operation for group_10 (bits 5–3). Same as dst.
    reg_pair:       Register pair (bits 5–4), 0–3.
    is_halt:        True if opcode == 0x76 (HALT).
    is_memory_src:  True if src == 6 (read from (HL)).
    is_memory_dst:  True if dst == 6 (write to (HL)).
    extra_bytes:    Additional bytes to fetch (0, 1, or 2).
    prefix:         Prefix byte seen before this opcode (0 = none, 0xCB/0xED/0xDD/0xFD).
    opcode:         Raw opcode byte.
    """

    op_group: int       # 0–3
    dst: int            # 0–7
    src: int            # 0–7
    alu_op: int         # 0–7
    reg_pair: int       # 0–3 (bits 5–4)
    is_halt: bool
    is_memory_src: bool
    is_memory_dst: bool
    extra_bytes: int    # 0, 1, or 2
    prefix: int         # 0 if no prefix
    opcode: int         # raw opcode byte


class DecoderZ80:
    """Combinational instruction decoder for the Zilog Z80.

    Maps an 8-bit opcode (with optional prefix context) to a DecoderOutput
    record using AND/NOT/OR gate functions for the primary classification.
    No state is held — every call is independent.

    Usage:
        >>> dec = DecoderZ80()
        >>> dec.decode_unprefixed(0x00)   # NOP
        DecoderOutput(op_group=0, ...)
        >>> dec.decode_unprefixed(0x76)   # HALT
        DecoderOutput(is_halt=True, ...)
        >>> dec.decode_unprefixed(0x80)   # ADD A, B
        DecoderOutput(op_group=2, alu_op=0, src=0, ...)
        >>> dec.decode_cb(0x07)           # CB RLC A
        DecoderOutput(op_group=0, src=7, ...)
    """

    def decode_unprefixed(self, opcode: int) -> DecoderOutput:
        """Decode a single unprefixed opcode byte into control signals.

        Args:
            opcode: 8-bit instruction opcode (0–255).

        Returns:
            DecoderOutput with all control signals.
        """
        b7 = _bit(opcode, 7)
        b6 = _bit(opcode, 6)
        b5 = _bit(opcode, 5)
        b4 = _bit(opcode, 4)
        b3 = _bit(opcode, 3)
        b2 = _bit(opcode, 2)
        b1 = _bit(opcode, 1)
        b0 = _bit(opcode, 0)

        # ── Group decode: AND/NOT tree on bits 7–6 ──────────────────────
        nb7 = NOT(b7)
        nb6 = NOT(b6)
        is_group00 = AND(nb7, nb6)   # 00
        is_group01 = AND(nb7, b6)    # 01
        is_group10 = AND(b7, nb6)    # 10
        is_group11 = AND(b7, b6)     # 11

        # Encode group as integer 0–3
        if is_group11:
            op_group = 3
        elif is_group10:
            op_group = 2
        elif is_group01:
            op_group = 1
        else:
            op_group = 0

        # ── Field extraction ─────────────────────────────────────────────
        dst = (b5 << 2) | (b4 << 1) | b3       # bits 5–3
        src = (b2 << 2) | (b1 << 1) | b0       # bits 2–0
        alu_op = dst                             # group_10: bits 5–3 = ALU op
        reg_pair = (b5 << 1) | b4               # bits 5–4

        # ── HLT detection: opcode == 0x76 ────────────────────────────────
        # 0x76 = 0b01110110 = group01 with dst=6 and src=6
        # HALT is literally "MOV M, M" in 8080 notation — repurposed
        is_halt_int = AND(
            is_group01,
            AND(AND(b5, b4), AND(NOT(b3), AND(b2, AND(b1, NOT(b0)))))
        )
        is_halt = bool(is_halt_int)

        # ── Memory operand detection ──────────────────────────────────────
        # (HL) pseudo-register: code 6 = 0b110
        is_mem_src_int = AND(b2, AND(b1, NOT(b0)))
        is_memory_src = bool(AND(is_mem_src_int, NOT(is_halt_int)))

        is_mem_dst_int = AND(b5, AND(b4, NOT(b3)))
        is_memory_dst = bool(AND(is_mem_dst_int, NOT(is_halt_int)))

        # ── Extra bytes needed ────────────────────────────────────────────
        extra_bytes = _count_extra_bytes_main(opcode, is_group00, is_group11)

        return DecoderOutput(
            op_group=op_group,
            dst=dst,
            src=src,
            alu_op=alu_op,
            reg_pair=reg_pair,
            is_halt=is_halt,
            is_memory_src=is_memory_src,
            is_memory_dst=is_memory_dst,
            extra_bytes=extra_bytes,
            prefix=0,
            opcode=opcode,
        )

    def decode_cb(self, opcode: int) -> DecoderOutput:
        """Decode a CB-prefixed opcode (rotate/shift/bit operations).

        CB opcodes are always 1 byte after the prefix. The format is:
            Bits 7–6: operation type (0=rotate/shift, 1=BIT, 2=RES, 3=SET)
            Bits 5–3: bit number (for BIT/RES/SET) or rotation type (0–7)
            Bits 2–0: register code (0=B, 1=C, ..., 6=(HL), 7=A)

        No memory operand for pure CB (DD/FD CB has (IX+d)/(IY+d)).

        Args:
            opcode: CB sub-opcode byte.

        Returns:
            DecoderOutput.
        """
        b7 = _bit(opcode, 7)
        b6 = _bit(opcode, 6)
        b5 = _bit(opcode, 5)
        b4 = _bit(opcode, 4)
        b3 = _bit(opcode, 3)
        b2 = _bit(opcode, 2)
        b1 = _bit(opcode, 1)
        b0 = _bit(opcode, 0)

        nb7 = NOT(b7)
        nb6 = NOT(b6)
        _is_group00 = AND(nb7, nb6)  # noqa: F841 — computed for gate-level completeness
        is_group01 = AND(nb7, b6)
        is_group10 = AND(b7, nb6)
        is_group11 = AND(b7, b6)

        if is_group11:
            op_group = 3
        elif is_group10:
            op_group = 2
        elif is_group01:
            op_group = 1
        else:
            op_group = 0

        dst = (b5 << 2) | (b4 << 1) | b3   # bit number for BIT/RES/SET
        src = (b2 << 2) | (b1 << 1) | b0   # register code
        alu_op = dst   # rotation type for group00

        # (HL) memory: src == 6
        is_mem_src_int = AND(b2, AND(b1, NOT(b0)))
        is_memory_src = bool(is_mem_src_int)
        is_memory_dst = is_memory_src  # CB ops read and write same register

        return DecoderOutput(
            op_group=op_group,
            dst=dst,
            src=src,
            alu_op=alu_op,
            reg_pair=0,
            is_halt=False,
            is_memory_src=is_memory_src,
            is_memory_dst=is_memory_dst,
            extra_bytes=0,   # CB sub-opcode is 1 byte, already fetched
            prefix=0xCB,
            opcode=opcode,
        )

    def decode_ed(self, opcode: int) -> DecoderOutput:
        """Decode an ED-prefixed opcode (extended instruction set).

        ED instructions include:
          - 16-bit arithmetic: ADC HL,rp / SBC HL,rp
          - Block ops: LDI/LDD/LDIR/LDDR, CPI/CPD/CPIR/CPDR
          - I/O block ops: INI/IND/INIR/INDR, OUTI/OUTD/OTIR/OTDR
          - Register loads: LD A,I / LD A,R / LD I,A / LD R,A
          - Interrupt: IM 0 / IM 1 / IM 2 / NEG / RETI / RETN
          - 16-bit indirect loads: LD rp,(nn) / LD (nn),rp
          - IN r,(C) / OUT (C),r

        Args:
            opcode: ED sub-opcode byte.

        Returns:
            DecoderOutput.
        """
        b5 = _bit(opcode, 5)
        b4 = _bit(opcode, 4)
        b3 = _bit(opcode, 3)
        b2 = _bit(opcode, 2)
        b1 = _bit(opcode, 1)
        b0 = _bit(opcode, 0)

        # For ED opcodes, extra_bytes depends on specific opcode
        extra_bytes = 0
        if opcode & 0xCF in (0x43, 0x4B):  # LD (nn),rp / LD rp,(nn)
            extra_bytes = 2

        reg_pair = (b5 << 1) | b4   # bits 5–4
        dst = (b5 << 2) | (b4 << 1) | b3
        src = (b2 << 2) | (b1 << 1) | b0

        return DecoderOutput(
            op_group=0,           # ED uses own dispatch
            dst=dst,
            src=src,
            alu_op=0,
            reg_pair=reg_pair,
            is_halt=False,
            is_memory_src=False,
            is_memory_dst=False,
            extra_bytes=extra_bytes,
            prefix=0xED,
            opcode=opcode,
        )

    def decode_dd_fd(self, prefix: int, opcode: int) -> DecoderOutput:
        """Decode a DD-prefixed (IX) or FD-prefixed (IY) opcode.

        DD/FD prefixed instructions mostly mirror the main instruction set
        with HL replaced by IX (DD) or IY (FD). Memory access via (IX+d)
        or (IY+d) requires an additional signed displacement byte.

        When DD/FD is followed by CB, we have a 4-byte instruction:
        DD CB d opcode or FD CB d opcode — decoded separately.

        Args:
            prefix:  0xDD (IX) or 0xFD (IY).
            opcode:  Sub-opcode byte.

        Returns:
            DecoderOutput.
        """
        extra_bytes = _count_extra_bytes_ddfd(opcode)

        b5 = _bit(opcode, 5)
        b4 = _bit(opcode, 4)
        b3 = _bit(opcode, 3)
        b2 = _bit(opcode, 2)
        b1 = _bit(opcode, 1)
        b0 = _bit(opcode, 0)
        b7 = _bit(opcode, 7)
        b6 = _bit(opcode, 6)

        nb7 = NOT(b7)
        nb6 = NOT(b6)
        is_group01 = AND(nb7, b6)
        is_group10 = AND(b7, nb6)
        is_group11 = AND(b7, b6)

        if is_group11:
            op_group = 3
        elif is_group10:
            op_group = 2
        elif is_group01:
            op_group = 1
        else:
            op_group = 0

        dst = (b5 << 2) | (b4 << 1) | b3
        src = (b2 << 2) | (b1 << 1) | b0
        reg_pair = (b5 << 1) | b4

        # For DD/FD, (IX/IY+d) is the memory operand (src or dst == 6)
        is_mem_src_int = AND(b2, AND(b1, NOT(b0)))
        is_memory_src = bool(is_mem_src_int)
        is_mem_dst_int = AND(b5, AND(b4, NOT(b3)))
        is_memory_dst = bool(is_mem_dst_int)

        return DecoderOutput(
            op_group=op_group,
            dst=dst,
            src=src,
            alu_op=dst,
            reg_pair=reg_pair,
            is_halt=opcode == 0x76,
            is_memory_src=is_memory_src,
            is_memory_dst=is_memory_dst,
            extra_bytes=extra_bytes,
            prefix=prefix,
            opcode=opcode,
        )


def _count_extra_bytes_main(opcode: int, is_group00: int, is_group11: int) -> int:
    """Count extra bytes for unprefixed Z80 instructions.

    Z80 instruction lengths (excluding prefix bytes):
        1 byte:  register-register, ALU register, single-byte control
        2 bytes: LD r,n, LD (IX+d),n, relative jumps (JR), immediate ALU
        3 bytes: LD rp,nn, LD A,(nn), CALL nn, JP nn, conditional J/CALL

    Returns 0, 1, or 2.
    """
    if is_group00:
        # LXI/LD rp, nn: bits 3–0 == 0001
        if (opcode & 0x0F) == 0x01:
            return 2
        # LD r, n (MVI): bits 2–0 == 110 (src=6), various dst
        if (opcode & 0x07) == 0x06 and opcode != 0x76:
            return 1
        # LD A,(nn), LD (nn),A, LD HL,(nn), LD (nn),HL
        if opcode in (0x3A, 0x32, 0x2A, 0x22):
            return 2
        # JR e (relative jump): always 2 bytes
        if opcode == 0x18:
            return 1
        # JR cc, e (conditional relative): 0x20, 0x28, 0x30, 0x38
        if opcode in (0x20, 0x28, 0x30, 0x38):
            return 1
        # DJNZ: 2 bytes
        if opcode == 0x10:
            return 1
        return 0

    if is_group11:
        # JP nn, CALL nn: 3 bytes
        if opcode in (0xC3, 0xCD):
            return 2
        # Conditional JP cc, nn: opcode & 0xC7 == 0xC2
        if (opcode & 0xC7) == 0xC2:
            return 2
        # Conditional CALL cc, nn: opcode & 0xC7 == 0xC4
        if (opcode & 0xC7) == 0xC4:
            return 2
        # IN A,(n) and OUT (n),A: 2 bytes
        if opcode in (0xDB, 0xD3):
            return 1
        # ALU immediate (ADD A,n, etc.): opcode & 0xC7 == 0xC6
        if (opcode & 0xC7) == 0xC6:
            return 1
        return 0

    # Group 01 (LD r,r) and Group 10 (ALU r): always 1 byte
    return 0


def _count_extra_bytes_ddfd(opcode: int) -> int:
    """Count extra bytes for DD/FD-prefixed instructions.

    Most DD/FD instructions mirror the main set.
    Instructions using (IX+d)/(IY+d) need an extra displacement byte.
    LD IX,nn / ADD IX,rp / LD IX,(nn) / LD (nn),IX need 2 extra bytes.
    """
    # LD IX, nn  / LD IY, nn
    if opcode == 0x21:
        return 2
    # LD IX, (nn) / LD IY, (nn)
    if opcode == 0x2A:
        return 2
    # LD (nn), IX / LD (nn), IY
    if opcode == 0x22:
        return 2
    # LD (IX+d), n  — needs displacement + immediate
    if opcode == 0x36:
        return 2
    # ALU ops with (IX+d) / (IY+d): 1 extra byte (displacement)
    if 0x86 <= opcode <= 0xBE and (opcode & 0x07) == 0x06:
        return 1
    # LD r, (IX+d) or LD (IX+d), r: 1 extra byte (displacement)
    if (
        0x40 <= opcode <= 0x7F
        and opcode != 0x76
        and ((opcode & 0x07) == 0x06 or ((opcode >> 3) & 0x07) == 0x06)
    ):
        return 1
    # INC (IX+d) / DEC (IX+d): 1 extra byte
    if opcode in (0x34, 0x35):
        return 1
    # Single-byte mirror of main instructions (NOP, RET, etc.)
    return 0
