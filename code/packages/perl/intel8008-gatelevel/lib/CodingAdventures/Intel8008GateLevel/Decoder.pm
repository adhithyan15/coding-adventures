package CodingAdventures::Intel8008GateLevel::Decoder;

# ============================================================================
# Decoder.pm — Instruction Decoder (Combinational Gate Logic)
# ============================================================================
#
# The instruction decoder takes the 8-bit opcode and produces control signals
# that drive the rest of the CPU for one instruction cycle.
#
# The decoder is PURELY COMBINATIONAL — it has no state, no clock. It is
# the "what do I do with this instruction?" question answered entirely by
# AND/OR/NOT gate trees that pattern-match the opcode bits.
#
# ## Why Combinational?
#
# In a real CPU, the instruction decoder must produce all control signals
# simultaneously within one clock cycle. There's no time for sequential
# evaluation. Everything fans out in parallel from the 8 opcode input bits.
#
# ## Gate Tree Structure
#
# Level 1 — Decode the 2-bit major group (bits 7–6):
#
#   group_00 = AND(NOT(b7), NOT(b6))    → INR/DCR/MVI/Rotates/RET/RST/OUT
#   group_01 = AND(NOT(b7),     b6 )    → MOV, IN, Jumps, Calls, HLT
#   group_10 = AND(    b7,  NOT(b6))    → ALU register ops
#   group_11 = AND(    b7,      b6 )    → ALU immediate
#
# Level 2 — Decode the sub-type within each group using bits 5–0.
#
# ## Opcode Fields
#
# The 8008 opcode has three named fields:
#
#   bit[7:6] = group (major instruction class)
#   bit[5:3] = DDD  (destination register or ALU operation select)
#   bit[2:0] = SSS  (source register or sub-operation)
#
# These are decoded using AND combinations of the 8 input bits.
#
# ## Example: Detecting ADD B (opcode 0x80 = 10 000 000)
#
#   b7=1, b6=0 → group_10 = AND(b7, NOT(b6)) = AND(1,1) = 1   ✓ ALU register
#   b5=0, b4=0, b3=0 → is_add = AND(NOT(b5), NOT(b4), NOT(b3)) = 1  ✓
#   b2=0, b1=0, b0=0 → reg_src = 0 (register B)                ✓

use strict;
use warnings;

use CodingAdventures::LogicGates qw(AND OR NOT);

use Exporter 'import';
our @EXPORT_OK = qw(decode);

# decode — decode an opcode byte into control signals.
#
# @param $opcode   8-bit integer (the fetched instruction byte)
# @return          Hashref of control signals
sub decode {
    my ($opcode) = @_;

    # -----------------------------------------------------------------------
    # Extract individual bits (wire the opcode bus into named signals)
    # -----------------------------------------------------------------------
    my @bit;
    for my $i (0..7) {
        $bit[$i] = ($opcode >> $i) & 1;
    }
    # Aliases for readability (hardware would name these signals on the bus)
    my ($b0, $b1, $b2, $b3, $b4, $b5, $b6, $b7) = @bit;

    # -----------------------------------------------------------------------
    # Level 1: Decode major group (bits 7–6)
    # -----------------------------------------------------------------------
    my $not_b7 = NOT($b7);
    my $not_b6 = NOT($b6);

    my $group_00 = AND($not_b7, $not_b6);  # bits[7:6] = 00
    my $group_01 = AND($not_b7, $b6    );  # bits[7:6] = 01
    my $group_10 = AND($b7,     $not_b6);  # bits[7:6] = 10
    my $group_11 = AND($b7,     $b6    );  # bits[7:6] = 11

    # -----------------------------------------------------------------------
    # Decode DDD (bits 5–3) and SSS (bits 2–0) fields
    # -----------------------------------------------------------------------
    my $ddd = ($opcode >> 3) & 7;
    my $sss = $opcode & 7;

    my $not_b5 = NOT($b5);
    my $not_b4 = NOT($b4);
    my $not_b3 = NOT($b3);
    my $not_b2 = NOT($b2);
    my $not_b1 = NOT($b1);
    my $not_b0 = NOT($b0);

    # -----------------------------------------------------------------------
    # Level 2a: Group 00 (INR, DCR, MVI, Rotates, RET, RST, OUT)
    # -----------------------------------------------------------------------

    # INR: group=00, sss=000 (bits[2:0]=000)
    my $is_inr = AND($group_00,
                     AND(AND($not_b2, $not_b1), $not_b0));

    # DCR: group=00, sss=001 (bits[2:0]=001)
    my $is_dcr = AND($group_00,
                     AND(AND($not_b2, $not_b1), $b0));

    # Rotate: group=00, sss=010, bit5=0 (DDD bit2=0, i.e., b5=0)
    my $is_rot = AND($group_00,
                     AND(AND($not_b2, $b1), $not_b0));
    $is_rot = AND($is_rot, $not_b5);  # bit5=0 distinguishes rotate from OUT

    # MVI: group=00, sss=110
    my $is_mvi = AND($group_00,
                     AND(AND($b2, $b1), $not_b0));

    # RET family: group=00, sss=011 or 111 (bits[1:0]=11)
    my $is_ret_family = AND($group_00, AND($b1, $b0));

    # RST: group=00, sss=101 (bits[2:0]=101)
    my $is_rst = AND($group_00,
                     AND(AND($b2, $not_b1), $b0));

    # OUT: group=00, sss=010, bit5=1 (DDD bit2=1, i.e., b5=1)
    my $is_out = AND($group_00,
                     AND(AND($not_b2, $b1), $not_b0));
    $is_out = AND($is_out, $b5);

    # -----------------------------------------------------------------------
    # Level 2b: Group 01 (MOV, IN, HLT, Jumps, Calls)
    # -----------------------------------------------------------------------

    # HLT: opcode 0x76 = 01 110 110
    my $is_hlt = ($opcode == 0x76 || $opcode == 0xFF) ? 1 : 0;

    # IN: group=01, sss=001
    my $is_in_flag = AND($group_01,
                         AND(AND($not_b2, $not_b1), $b0));

    # Jump/Call detection: group=01, ddd<=3, sss ∈ {000,100,010,110}
    # OR (group=01, ddd=7, sss ∈ {100,110})
    # Simplified: we just pass ddd and sss to the caller.

    # MOV: group=01, not IN, not HLT, not jump/call
    # (Handled by the CPU based on decoded group+sss+ddd)

    # -----------------------------------------------------------------------
    # Level 2c: Group 10 (ALU register operations: 10 OOO SSS)
    # -----------------------------------------------------------------------

    # ALU op from DDD field (bits 5–3):
    my $alu_op = $ddd;  # 0=ADD,1=ADC,2=SUB,3=SBB,4=ANA,5=XRA,6=ORA,7=CMP

    # -----------------------------------------------------------------------
    # Level 2d: Group 11 (ALU immediate: 11 OOO 100)
    # -----------------------------------------------------------------------

    # ALU immediate: group=11, sss=100
    my $is_alu_imm = AND($group_11,
                         AND(AND($b2, $not_b1), $not_b0));

    # -----------------------------------------------------------------------
    # Assemble the control signal record
    # -----------------------------------------------------------------------
    return {
        # Raw decoded fields
        group    => ($opcode >> 6) & 3,
        ddd      => $ddd,
        sss      => $sss,

        # Group decode (one-hot)
        group_00 => $group_00,
        group_01 => $group_01,
        group_10 => $group_10,
        group_11 => $group_11,

        # Instruction type signals
        is_hlt   => $is_hlt,
        is_inr   => $is_inr,
        is_dcr   => $is_dcr,
        is_rot   => $is_rot,
        is_mvi   => $is_mvi,
        is_ret   => $is_ret_family,
        is_rst   => $is_rst,
        is_out   => $is_out,
        is_in    => $is_in_flag,
        is_alu_r => $group_10,      # ALU register operation
        is_alu_i => $is_alu_imm,    # ALU immediate

        # Instruction byte count (needed for PC advance)
        instr_bytes => _instr_bytes($opcode),

        # T bit and CCC for jump/call/return (sense and condition)
        # T = (sss >> 2) & 1 for jumps and calls
        # CCC = ddd for jumps and calls
        t_bit    => ($sss >> 2) & 1,
        ccc      => $ddd & 3,

        # Unconditional variants (ddd=7 with specific sss)
        is_jmp   => ($opcode == 0x7C) ? 1 : 0,
        is_cal   => ($opcode == 0x7E) ? 1 : 0,
        is_ret_u => ($opcode == 0x3F) ? 1 : 0,
    };
}

# Determine instruction byte count from opcode.
# 1 byte: most instructions
# 2 bytes: MVI (00DDD110) or ALU-immediate (11OOO100)
# 3 bytes: jumps/calls (group=01, specific ddd/sss)
sub _instr_bytes {
    my ($opcode) = @_;
    my $group = ($opcode >> 6) & 3;
    my $ddd   = ($opcode >> 3) & 7;
    my $sss   = $opcode & 7;

    # 2-byte: MVI
    return 2 if $group == 0b00 && $sss == 0b110;

    # 2-byte: ALU immediate
    return 2 if $group == 0b11 && $sss == 0b100;

    # 3-byte: conditional jumps/calls
    if ($group == 0b01) {
        my $is_jcc_sss = ($sss == 0b000 || $sss == 0b100 ||
                          $sss == 0b010 || $sss == 0b110);
        return 3 if $ddd <= 3 && $is_jcc_sss;
        return 3 if $ddd == 7 && ($sss == 0b100 || $sss == 0b110);
    }

    return 1;
}

1;
