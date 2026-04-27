"""Instruction decoder for the Intel 8008 gate-level simulator.

=== What is an instruction decoder? ===

The instruction decoder is a combinational logic circuit — it has no memory
and produces outputs purely as a function of its inputs. Given the 8 bits
of an opcode byte, it generates a set of control signals that tell the rest
of the CPU what to do during that instruction cycle.

In the real Intel 8008, the decoder was implemented as a tree of AND, OR,
and NOT gates that pattern-matched on the opcode bits. No lookup table, no
software switch statement — just wire connections and transistors.

=== How opcode bits map to instructions ===

The 8008 uses a structured encoding:
    bits[7:6] = major group (00, 01, 10, 11)
    bits[5:3] = destination register or ALU operation
    bits[2:0] = source register or sub-operation

We decode using AND/OR/NOT gates on these fields, matching how the real
hardware would implement it.

=== Gate cost of the decoder ===

A 2-to-4 decoder (for bits[7:6]) requires 4 AND gates + 2 NOT gates = 6 gates.
The full 8008 decoder with all conditions and control signals requires
approximately 50–100 gates. This is smaller than the ALU but still significant.
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT, OR


@dataclass
class DecoderOutput:
    """Control signals produced by the instruction decoder.

    These signals drive the rest of the CPU during one instruction cycle.
    All fields are 0 or 1 (gate outputs).

    The decoder takes the 8-bit opcode and produces these control signals,
    which determine:
    - Which ALU operation to perform
    - Which registers to read/write
    - Whether to update flags
    - Whether to jump or call or return
    - Whether to access memory
    - Whether to halt
    """

    # Instruction type flags (mutually exclusive groups)
    is_mov: int         # 1 = MOV D,S (group 01, rest)
    is_mvi: int         # 1 = MVI D,d (group 00, sss=110)
    is_inr: int         # 1 = INR D   (group 00, sss=000)
    is_dcr: int         # 1 = DCR D   (group 00, sss=001)
    is_alu_reg: int     # 1 = ADD/SUB/etc. with register source (group 10)
    is_alu_imm: int     # 1 = ADI/SUI/etc. with immediate (group 11, sss=100)
    is_rotate: int      # 1 = RLC/RRC/RAL/RAR
    is_jump: int        # 1 = conditional or unconditional jump (3-byte)
    is_call: int        # 1 = conditional or unconditional call (3-byte)
    is_ret: int         # 1 = conditional or unconditional return
    is_rst: int         # 1 = RST n (1-byte call to n*8)
    is_in: int          # 1 = IN port
    is_out: int         # 1 = OUT port
    is_halt: int        # 1 = HLT (0x76 or 0xFF)

    # Operand fields
    alu_op: int         # 0-7: ADD=0,ADC=1,SUB=2,SBB=3,ANA=4,XRA=5,ORA=6,CMP=7
    reg_dst: int        # 0-7: destination register index
    reg_src: int        # 0-7: source register index
    rotate_type: int    # 0=RLC, 1=RRC, 2=RAL, 3=RAR

    # Condition for conditional jump/call/return
    cond_code: int      # 0=CY, 1=Z, 2=S, 3=P
    cond_sense: int     # 0=if-false/clear, 1=if-true/set
    unconditional: int  # 1 = always take the jump/call/return

    # RST/IN/OUT port
    port_or_rst: int    # RST n: target = n*8; IN/OUT: port number

    # Instruction length
    instruction_bytes: int  # 1, 2, or 3


def decode(opcode: int) -> DecoderOutput:
    """Decode an 8008 opcode into control signals using AND/OR/NOT gates.

    This function mirrors the combinational gate network in the real chip.
    All decisions are made via gate operations on the individual opcode bits.

    The decoder does NOT fetch additional bytes — it only looks at the
    opcode itself. The CPU is responsible for fetching operand bytes based
    on the instruction_bytes field.

    Args:
        opcode: 8-bit instruction byte (0x00–0xFF).

    Returns:
        DecoderOutput with all control signals set.
    """
    # Extract individual bits (LSB first to match hardware convention)
    # b[0] = bit 0 (LSB), b[7] = bit 7 (MSB)
    b = [(opcode >> i) & 1 for i in range(8)]

    # --- Major group decode (bits 7–6) ---
    # Use AND/NOT to decode which of the 4 major groups this opcode belongs to.
    # group_00 = NOT(b7) AND NOT(b6)
    # group_01 = NOT(b7) AND b6
    # group_10 = b7 AND NOT(b6)
    # group_11 = b7 AND b6
    group_00 = AND(NOT(b[7]), NOT(b[6]))
    group_01 = AND(NOT(b[7]),     b[6])
    group_10 = AND(    b[7],  NOT(b[6]))
    group_11 = AND(    b[7],      b[6])

    # --- Register fields ---
    # bits[5:3] = DDD (destination/operation), bits[2:0] = SSS (source/sub-op)
    # These are the raw 3-bit fields from the opcode
    ddd = (opcode >> 3) & 0x7
    sss = opcode & 0x7

    # --- Special opcode detection ---
    # HLT has two encodings that must be caught before general dispatch
    is_hlt_76 = 1 if opcode == 0x76 else 0
    is_hlt_ff = 1 if opcode == 0xFF else 0
    is_halt = OR(is_hlt_76, is_hlt_ff)

    # JMP unconditional: 0x7C = 01 111 100
    is_jmp = 1 if opcode == 0x7C else 0
    # CAL unconditional: 0x7E = 01 111 110
    is_cal = 1 if opcode == 0x7E else 0

    # --- Group 00 decoding ---
    # sss bits for group 00 determine the instruction:
    sss_000 = AND(NOT(b[2]), AND(NOT(b[1]), NOT(b[0])))  # sss = 000
    sss_001 = AND(NOT(b[2]), AND(NOT(b[1]),     b[0]))   # sss = 001
    sss_010 = AND(NOT(b[2]), AND(    b[1],  NOT(b[0])))  # sss = 010
    sss_101 = AND(    b[2],  AND(NOT(b[1]),     b[0]))   # sss = 101
    sss_110 = AND(    b[2],  AND(    b[1],  NOT(b[0])))  # sss = 110
    sss_011 = AND(NOT(b[2]), AND(    b[1],      b[0]))   # sss = 011
    sss_111 = AND(    b[2],  AND(    b[1],      b[0]))   # sss = 111

    # INR: group=00, sss=000
    is_inr = AND(group_00, sss_000)
    # DCR: group=00, sss=001
    is_dcr = AND(group_00, sss_001)
    # MVI: group=00, sss=110 (2-byte)
    is_mvi = AND(group_00, sss_110)

    # Rotate: group=00, sss=010, ddd in {0,1,2,3}
    # ddd < 4 means bit[5] of opcode = 0 (ddd's MSB = b[5])
    is_rotate_candidate = AND(group_00, sss_010)
    ddd_lt4 = NOT(b[5])  # ddd < 4 iff bit[5] = 0
    is_rotate = AND(is_rotate_candidate, ddd_lt4)

    # OUT: group=00, sss=010, ddd >= 4 (ddd's MSB = b[5] = 1)
    is_out = AND(is_rotate_candidate, b[5])

    # RST: group=00, sss=101
    is_rst = AND(group_00, sss_101)

    # Return: group=00, (sss=011 or sss=111)
    is_ret_candidate = AND(group_00, OR(sss_011, sss_111))

    # --- Group 01 decoding ---
    # IN: group=01, sss=001 exactly
    is_in = AND(group_01, sss_001)

    # Conditional jump: group=01, ddd<=3, sss&3=00 (sss in {000, 100})
    # ddd <= 3 means b[5]=0
    sss_bot2_eq00 = AND(NOT(b[1]), NOT(b[0]))
    is_cond_jump = AND(AND(group_01, ddd_lt4), sss_bot2_eq00)
    # Exclude HLT (0x76): HLT has group=01, ddd=6≥4, so ddd_lt4=0 already excludes it

    # Conditional call: group=01, ddd<=3, sss&3=10 (sss in {010, 110})
    sss_bot2_eq10 = AND(b[1], NOT(b[0]))
    is_cond_call = AND(AND(group_01, ddd_lt4), sss_bot2_eq10)

    # Jump (any): unconditional JMP or conditional jump
    is_jump = OR(is_jmp, is_cond_jump)
    # Call (any): unconditional CAL or conditional call
    is_call = OR(is_cal, is_cond_call)
    # Return (any): with or without condition
    is_ret = is_ret_candidate

    # MOV: group=01, not (HLT or JMP or CAL or IN or cond_jump or cond_call)
    # = all remaining group=01 opcodes
    is_mov_candidate = AND(group_01,
                           NOT(OR(is_halt,
                               OR(is_jmp,
                               OR(is_cal,
                               OR(is_in,
                               OR(is_cond_jump, is_cond_call)))))))

    # Need to handle the case where ddd>=4 with sss&3 in {00,10} → MOV
    # These are caught by is_mov_candidate since ddd_lt4=0 excludes them from jump/call
    is_mov = is_mov_candidate

    # --- Group 10 decoding: ALU register ---
    is_alu_reg = group_10

    # --- Group 11 decoding: ALU immediate and special ---
    # ALU imm: group=11, sss=100
    sss_100 = AND(NOT(b[2]), AND(    b[1],  NOT(b[0])))  # Wait: 100 = b[2]=1, b[1]=0, b[0]=0
    # Correction: 100 in binary (sss=4): bit2=1, bit1=0, bit0=0
    sss_4 = AND(b[2], AND(NOT(b[1]), NOT(b[0])))
    is_alu_imm = AND(group_11, sss_4)

    # --- ALU operation code (bits[5:3] = ddd) ---
    # Same field is used for both ALU reg (group 10) and ALU imm (group 11)
    alu_op = ddd

    # --- Condition code and sense for conditional branches ---
    # For jumps/calls: CCC = ddd (bits[5:3]), T = bit[2] of sss
    # For returns: same encoding in the opcode
    cond_code = ddd & 0x3  # low 2 bits of ddd (0=CY, 1=Z, 2=S, 3=P)
    cond_sense = (sss >> 2) & 1  # bit[2] of sss = T (0=if-false, 1=if-true)

    # Unconditional: JMP (opcode=0x7C) or CAL (opcode=0x7E) or RET (ddd=7, sss=7)
    is_unconditional = OR(is_jmp, OR(is_cal, 1 if (ddd == 7 and sss == 7) else 0))

    # --- RST vector and IN/OUT port ---
    # RST: target = ddd * 8 (ddd = bits[5:3] = AAA in spec)
    # IN/OUT port: ddd = port number
    port_or_rst = ddd

    # --- Rotate type (bits[4:3] = ddd[1:0]) ---
    # 0=RLC, 1=RRC, 2=RAL, 3=RAR (only the low 2 bits of ddd matter)
    rotate_type = ddd & 0x3

    # --- Instruction length ---
    if is_mvi or is_alu_imm:
        instruction_bytes = 2
    elif is_jump or is_call:
        instruction_bytes = 3
    else:
        instruction_bytes = 1

    return DecoderOutput(
        is_mov=is_mov,
        is_mvi=is_mvi,
        is_inr=is_inr,
        is_dcr=is_dcr,
        is_alu_reg=is_alu_reg,
        is_alu_imm=is_alu_imm,
        is_rotate=is_rotate,
        is_jump=is_jump,
        is_call=is_call,
        is_ret=is_ret,
        is_rst=is_rst,
        is_in=is_in,
        is_out=is_out,
        is_halt=is_halt,
        alu_op=alu_op,
        reg_dst=ddd,
        reg_src=sss,
        rotate_type=rotate_type,
        cond_code=cond_code,
        cond_sense=cond_sense,
        unconditional=is_unconditional,
        port_or_rst=port_or_rst,
        instruction_bytes=instruction_bytes,
    )
