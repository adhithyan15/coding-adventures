#!/usr/bin/perl
use strict;
use warnings;

# Tests for CodingAdventures::Intel8008GateLevel
#
# This test suite validates that the gate-level Intel 8008 simulator produces
# the same results as the behavioral simulator for every instruction type.
# Every arithmetic operation here routes through:
#
#   ripple_carry_adder() from CodingAdventures::Arithmetic
#     ← full_adder() chains
#       ← XOR/AND/OR gates from CodingAdventures::LogicGates
#
# The parity flag P uses: NOT(XORn(@bits)) — 7 XOR gates + NOT.
#
# Test structure:
#   - Unit tests for each submodule (Bits, ALU, Registers, Decoder, Stack)
#   - Integration tests for each instruction group
#   - Cross-validation: gate-level vs behavioral simulator

use Test2::V0;
use lib '../lib', '../../intel8008-simulator/lib';

use CodingAdventures::Intel8008GateLevel;
use CodingAdventures::Intel8008GateLevel::Bits qw(int_to_bits bits_to_int compute_parity);
use CodingAdventures::Intel8008GateLevel::ALU qw(
    alu_add alu_sub alu_and alu_or alu_xor
    alu_inr alu_dcr
    alu_rlc alu_rrc alu_ral alu_rar
    compute_flags
);
use CodingAdventures::Intel8008GateLevel::Registers qw(
    new_register_file read_reg write_reg reg_a hl_address
);
use CodingAdventures::Intel8008GateLevel::Decoder qw(decode);
use CodingAdventures::Intel8008GateLevel::Stack qw(
    new_stack push_stack pop_stack stack_pc set_pc
);

my $HAS_BSIM = eval { require CodingAdventures::Intel8008Simulator; 1 };

# ---------------------------------------------------------------------------
# Helper: create a fresh gate-level CPU and run a program
# ---------------------------------------------------------------------------
sub run_prog {
    my ($bytes, $max_steps) = @_;
    $max_steps //= 10_000;
    my $cpu = CodingAdventures::Intel8008GateLevel->new();
    my $traces = $cpu->run($bytes, $max_steps);
    return ($cpu, $traces);
}

# ===========================================================================
# Submodule: Bits
# ===========================================================================

subtest 'Bits — int_to_bits / bits_to_int' => sub {
    # LSB-first representation:  0b10110101 = 181
    #   bits[0]=1 (LSB), bits[7]=1 (MSB)
    my $bits = int_to_bits(0b10110101, 8);
    is(scalar @$bits, 8, 'int_to_bits returns 8 elements');
    is($bits->[0], 1, 'bit 0 (LSB) correct');
    is($bits->[1], 0, 'bit 1 correct');
    is($bits->[2], 1, 'bit 2 correct');
    is($bits->[3], 0, 'bit 3 correct');
    is($bits->[4], 1, 'bit 4 correct');
    is($bits->[5], 1, 'bit 5 correct');
    is($bits->[6], 0, 'bit 6 correct');
    is($bits->[7], 1, 'bit 7 (MSB) correct');

    is(bits_to_int($bits), 0b10110101, 'round-trip 181');
    is(bits_to_int(int_to_bits(0, 8)),   0,   'round-trip 0');
    is(bits_to_int(int_to_bits(255, 8)), 255, 'round-trip 255');
    is(bits_to_int(int_to_bits(1, 8)),   1,   'round-trip 1');
    is(bits_to_int(int_to_bits(128, 8)), 128, 'round-trip 128');
};

subtest 'Bits — compute_parity (NOT(XORn))' => sub {
    # Parity = 1 means EVEN number of 1-bits (8008 convention)
    # 0x00 = 0000_0000 → 0 ones → even → P=1
    my $zero_bits = int_to_bits(0, 8);
    is(compute_parity(@$zero_bits), 1, '0x00: 0 ones → even parity P=1');

    # 0x01 = 0000_0001 → 1 one → odd → P=0
    my $one_bits = int_to_bits(1, 8);
    is(compute_parity(@$one_bits), 0, '0x01: 1 one → odd parity P=0');

    # 0x03 = 0000_0011 → 2 ones → even → P=1
    my $three_bits = int_to_bits(3, 8);
    is(compute_parity(@$three_bits), 1, '0x03: 2 ones → even parity P=1');

    # 0xFF = 1111_1111 → 8 ones → even → P=1
    my $ff_bits = int_to_bits(0xFF, 8);
    is(compute_parity(@$ff_bits), 1, '0xFF: 8 ones → even parity P=1');

    # 0x80 = 1000_0000 → 1 one → odd → P=0
    my $eighty_bits = int_to_bits(0x80, 8);
    is(compute_parity(@$eighty_bits), 0, '0x80: 1 one → odd parity P=0');
};

# ===========================================================================
# Submodule: ALU
# ===========================================================================

subtest 'ALU — alu_add (via ripple_carry_adder)' => sub {
    # alu_add returns ($result, $carry_out, $flags_hashref)
    # 1 + 2 = 3, no carry
    my ($result, $cy, $flags) = alu_add(1, 2, 0);
    is($result,         3, 'add 1+2 = 3');
    is($flags->{carry}, 0, 'add 1+2: no carry');
    is($flags->{zero},  0, 'add 1+2: not zero');
    is($flags->{sign},  0, 'add 1+2: positive');

    # 255 + 1 = 256 → result 0 with carry
    ($result, $cy, $flags) = alu_add(255, 1, 0);
    is($result,         0, 'add 255+1 = 0 (overflow)');
    is($flags->{carry}, 1, 'add 255+1: carry set');
    is($flags->{zero},  1, 'add 255+1: zero set');

    # 100 + 100 = 200, MSB set → sign
    ($result, $cy, $flags) = alu_add(100, 100, 0);
    is($result,        200, 'add 100+100 = 200');
    is($flags->{sign},  1,  '200 has MSB set → sign flag');

    # ADC: add with carry_in=1
    ($result, $cy, $flags) = alu_add(5, 10, 1);
    is($result, 16, 'add 5+10+carry_in(1) = 16');

    # 0 + 0 → zero flag
    ($result, $cy, $flags) = alu_add(0, 0, 0);
    is($flags->{zero}, 1, '0+0: zero flag set');

    # Parity: 3 = 0000_0011 → 2 ones → even parity P=1
    ($result, $cy, $flags) = alu_add(1, 2, 0);
    is($flags->{parity}, 1, '1+2=3: even parity P=1');
};

subtest 'ALU — alu_sub (two\'s complement via gates)' => sub {
    # alu_sub returns ($result, $carry_out, $flags_hashref)
    # 5 - 3 = 2, no borrow
    my ($result, $cy, $flags) = alu_sub(5, 3, 0);
    is($result,         2, 'sub 5-3 = 2');
    is($flags->{carry}, 0, 'sub 5-3: no borrow (CY=0)');
    is($flags->{zero},  0, 'sub 5-3: not zero');

    # 3 - 5: borrow → CY=1
    ($result, $cy, $flags) = alu_sub(3, 5, 0);
    is($flags->{carry}, 1, 'sub 3-5: borrow set (CY=1)');

    # 5 - 5 = 0 → zero flag, no borrow
    ($result, $cy, $flags) = alu_sub(5, 5, 0);
    is($result,         0, 'sub 5-5 = 0');
    is($flags->{zero},  1, 'sub 5-5: zero flag set');
    is($flags->{carry}, 0, 'sub 5-5: no borrow');
};

subtest 'ALU — alu_and / alu_or / alu_xor (bitwise gates)' => sub {
    my ($result, $cy, $flags);

    # AND: 0b10101010 & 0b11001100 = 0b10001000 = 0x88
    ($result, $cy, $flags) = alu_and(0b10101010, 0b11001100);
    is($result, 0b10001000, 'AND 0xAA & 0xCC = 0x88');

    # OR: 0b10101010 | 0b01010101 = 0xFF
    ($result, $cy, $flags) = alu_or(0b10101010, 0b01010101);
    is($result, 0xFF, 'OR 0xAA | 0x55 = 0xFF');

    # XOR: 0xFF ^ 0xFF = 0 → zero flag
    ($result, $cy, $flags) = alu_xor(0xFF, 0xFF);
    is($result,        0, 'XOR 0xFF ^ 0xFF = 0');
    is($flags->{zero}, 1, 'XOR 0xFF^0xFF: zero flag');

    # XOR: 0b10101010 ^ 0b11001100 = 0b01100110 = 0x66
    ($result, $cy, $flags) = alu_xor(0b10101010, 0b11001100);
    is($result, 0b01100110, 'XOR 0xAA ^ 0xCC = 0x66');
};

subtest 'ALU — alu_inr / alu_dcr (carry preserved)' => sub {
    # alu_inr / alu_dcr return ($result, $flags_hashref) — only 2 values.
    # They use compute_flags_no_carry which preserves the old carry.
    my ($result, $flags);

    # INR: 5 → 6, carry preserved
    ($result, $flags) = alu_inr(5, 0);
    is($result,         6, 'inr 5 → 6');
    is($flags->{carry}, 0, 'inr: carry preserved (was 0)');

    ($result, $flags) = alu_inr(5, 1);
    is($result,         6, 'inr 5 → 6 (carry still 1)');
    is($flags->{carry}, 1, 'inr: carry preserved (was 1)');

    # INR: 255 → 0 (wraps), zero flag set, carry preserved
    ($result, $flags) = alu_inr(255, 0);
    is($result,        0, 'inr 255 → 0 (wrap)');
    is($flags->{zero}, 1, 'inr 255→0: zero flag');
    is($flags->{carry}, 0, 'inr 255→0: carry NOT updated');

    # DCR: 3 → 2, carry preserved
    ($result, $flags) = alu_dcr(3, 0);
    is($result, 2, 'dcr 3 → 2');
    is($flags->{carry}, 0, 'dcr: carry preserved');

    # DCR: 0 → 255 (wraps), sign set, carry preserved
    ($result, $flags) = alu_dcr(0, 0);
    is($result,        255, 'dcr 0 → 255 (wrap)');
    is($flags->{sign},  1,  'dcr 0→255: sign flag set');
    is($flags->{carry}, 0,  'dcr: carry NOT updated');
};

subtest 'ALU — rotate operations' => sub {
    my ($new_a, $new_cy);

    # RLC (rotate left, bit7 → carry and bit0): 0b10000000 → 0b00000001, carry=1
    ($new_a, $new_cy) = alu_rlc(0b10000000);
    is($new_a,  0b00000001, 'RLC 0x80 → 0x01');
    is($new_cy, 1,          'RLC 0x80: carry=1');

    ($new_a, $new_cy) = alu_rlc(0b00000001);
    is($new_a,  0b00000010, 'RLC 0x01 → 0x02');
    is($new_cy, 0,          'RLC 0x01: carry=0');

    # RRC (rotate right, bit0 → carry and bit7): 0b00000001 → 0b10000000, carry=1
    ($new_a, $new_cy) = alu_rrc(0b00000001);
    is($new_a,  0b10000000, 'RRC 0x01 → 0x80');
    is($new_cy, 1,          'RRC 0x01: carry=1');

    ($new_a, $new_cy) = alu_rrc(0b10000000);
    is($new_a,  0b01000000, 'RRC 0x80 → 0x40');
    is($new_cy, 0,          'RRC 0x80: carry=0');

    # RAL (rotate left through carry): 0b10000000, cy=0 → 0b00000000, carry=1
    ($new_a, $new_cy) = alu_ral(0b10000000, 0);
    is($new_a,  0b00000000, 'RAL 0x80 cy=0 → 0x00');
    is($new_cy, 1,          'RAL 0x80: carry=1');

    # RAL: 0b01000000, cy=1 → 0b10000001, carry=0
    ($new_a, $new_cy) = alu_ral(0b01000000, 1);
    is($new_a,  0b10000001, 'RAL 0x40 cy=1 → 0x81');
    is($new_cy, 0,          'RAL 0x40 cy=1: carry=0');

    # RAR (rotate right through carry): 0b00000001, cy=0 → 0b00000000, carry=1
    ($new_a, $new_cy) = alu_rar(0b00000001, 0);
    is($new_a,  0b00000000, 'RAR 0x01 cy=0 → 0x00');
    is($new_cy, 1,          'RAR 0x01: carry=1');

    # RAR: 0b00000010, cy=1 → 0b10000001, carry=0
    ($new_a, $new_cy) = alu_rar(0b00000010, 1);
    is($new_a,  0b10000001, 'RAR 0x02 cy=1 → 0x81');
    is($new_cy, 0,          'RAR 0x02 cy=1: carry=0');
};

# ===========================================================================
# Submodule: Registers
# ===========================================================================

subtest 'Registers — flip-flop register file' => sub {
    my $file = new_register_file();

    # All registers start at 0
    for my $i (0..5, 7) {
        is(read_reg($file, $i), 0, "register $i starts at 0");
    }

    # Write and read back
    write_reg($file, 7, 0x42);  # A = 0x42
    is(read_reg($file, 7), 0x42, 'write/read A = 0x42');

    write_reg($file, 0, 0xAB);  # B = 0xAB
    is(read_reg($file, 0), 0xAB, 'write/read B = 0xAB');

    # Value wraps to 8 bits
    write_reg($file, 1, 0x1FF);  # C: overflow → 0xFF
    is(read_reg($file, 1), 0xFF, 'write 0x1FF → 0xFF (8-bit wrap)');

    # HL address computation: H=0x05, L=0x06 → addr = (5 & 0x3F) << 8 | 6 = 0x0506
    write_reg($file, 4, 0x05);   # H
    write_reg($file, 5, 0x06);   # L
    is(hl_address($file), 0x0506, 'HL address: H=0x05, L=0x06 → 0x0506');

    # H only uses 6 bits: H=0xFF → masked to 0x3F
    write_reg($file, 4, 0xFF);
    write_reg($file, 5, 0x00);
    is(hl_address($file), 0x3F00, 'HL address: H=0xFF masked to 0x3F');

    # Attempting to read/write M (index 6) raises an error
    ok(dies { read_reg($file, 6) },  'read M (index 6) dies');
    ok(dies { write_reg($file, 6, 0) }, 'write M (index 6) dies');
};

# ===========================================================================
# Submodule: Decoder
# ===========================================================================

subtest 'Decoder — combinational gate trees' => sub {
    # HLT: 0x76
    my $ctrl = decode(0x76);
    is($ctrl->{is_hlt}, 1, '0x76 → is_hlt');

    # HLT: 0xFF
    $ctrl = decode(0xFF);
    is($ctrl->{is_hlt}, 1, '0xFF → is_hlt');

    # ADD B: 0x80 = 10 000 000 → group_10, ddd=0, sss=0
    $ctrl = decode(0x80);
    is($ctrl->{group_10}, 1, '0x80 ADD B → group_10');
    is($ctrl->{ddd},      0, '0x80 ADD B → ddd=0 (ADD)');
    is($ctrl->{sss},      0, '0x80 ADD B → sss=0 (reg B)');
    is($ctrl->{group_00}, 0, '0x80: group_00 not set');
    is($ctrl->{group_01}, 0, '0x80: group_01 not set');
    is($ctrl->{group_11}, 0, '0x80: group_11 not set');

    # MVI A: 0x3E = 00 111 110 → group_00, ddd=7, sss=6
    $ctrl = decode(0x3E);
    is($ctrl->{group_00}, 1, '0x3E MVI A → group_00');
    is($ctrl->{is_mvi},   1, '0x3E → is_mvi');
    is($ctrl->{ddd},      7, '0x3E → ddd=7 (A)');

    # INR B: 0x00 = 00 000 000 → group_00, sss=0
    $ctrl = decode(0x00);
    is($ctrl->{is_inr}, 1, '0x00 → is_inr');

    # DCR A: 0x39 = 00 111 001 → group_00, sss=1
    $ctrl = decode(0x39);
    is($ctrl->{is_dcr}, 1, '0x39 → is_dcr');

    # MOV A, B: 0x78 = 01 111 000 → group_01
    $ctrl = decode(0x78);
    is($ctrl->{group_01}, 1, '0x78 MOV A,B → group_01');
    is($ctrl->{ddd},      7, '0x78 → ddd=7 (dest A)');
    is($ctrl->{sss},      0, '0x78 → sss=0 (src B)');

    # Instruction byte counts
    is(decode(0x06)->{instr_bytes}, 2, 'MVI B (0x06): 2 bytes');
    is(decode(0x3E)->{instr_bytes}, 2, 'MVI A (0x3E): 2 bytes');
    is(decode(0xC4)->{instr_bytes}, 2, 'ADI (0xC4): 2 bytes');
    is(decode(0x7C)->{instr_bytes}, 3, 'JMP (0x7C): 3 bytes');
    is(decode(0x7E)->{instr_bytes}, 3, 'CAL (0x7E): 3 bytes');
    is(decode(0x40)->{instr_bytes}, 3, 'JFC (0x40): 3 bytes');
    is(decode(0x78)->{instr_bytes}, 1, 'MOV A,B (0x78): 1 byte');
    is(decode(0x80)->{instr_bytes}, 1, 'ADD B (0x80): 1 byte');
};

# ===========================================================================
# Submodule: Stack
# ===========================================================================

subtest 'Stack — 8-level push-down' => sub {
    my $stack = new_stack();

    # Initial state: PC = 0, depth = 0
    is(stack_pc($stack), 0, 'initial PC = 0');
    is($stack->{depth},  0, 'initial depth = 0');

    # Set PC directly
    set_pc($stack, 0x1234);
    is(stack_pc($stack), 0x1234 & 0x3FFF, 'set_pc works');

    # Push: CALL to 0x0100
    set_pc($stack, 0x0050);  # current PC
    push_stack($stack, 0x0100);
    is(stack_pc($stack),         0x0100, 'after push: PC = target');
    is($stack->{entries}[1],     0x0050, 'after push: old PC saved at [1]');
    is($stack->{depth},          1,      'depth = 1 after push');

    # Pop: RETURN
    pop_stack($stack);
    is(stack_pc($stack), 0x0050, 'after pop: PC restored');
    is($stack->{depth},  0,      'depth = 0 after pop');

    # Nested pushes
    set_pc($stack, 0x0001);
    push_stack($stack, 0x0100);  # depth 1
    push_stack($stack, 0x0200);  # depth 2
    push_stack($stack, 0x0300);  # depth 3
    is(stack_pc($stack),     0x0300, 'nested push: PC = deepest target');
    is($stack->{entries}[1], 0x0200, 'level 1 saved');
    is($stack->{entries}[2], 0x0100, 'level 2 saved');
    is($stack->{entries}[3], 0x0001, 'level 3: original PC');
    is($stack->{depth},      3,      'depth = 3 after 3 pushes');

    pop_stack($stack);
    is(stack_pc($stack), 0x0200, 'pop 1: returned to 0x0200');
    pop_stack($stack);
    is(stack_pc($stack), 0x0100, 'pop 2: returned to 0x0100');
    pop_stack($stack);
    is(stack_pc($stack), 0x0001, 'pop 3: returned to original');
};

# ===========================================================================
# Integration: Gate-Level CPU — Initialization
# ===========================================================================

subtest 'initialization' => sub {
    my $cpu = CodingAdventures::Intel8008GateLevel->new();
    is($cpu->a,      0, 'A starts at 0');
    is($cpu->b,      0, 'B starts at 0');
    is($cpu->c,      0, 'C starts at 0');
    is($cpu->d,      0, 'D starts at 0');
    is($cpu->e,      0, 'E starts at 0');
    is($cpu->h,      0, 'H starts at 0');
    is($cpu->l,      0, 'L starts at 0');
    is($cpu->pc,     0, 'PC starts at 0');
    is($cpu->halted, 0, 'not halted at start');
    is($cpu->flags->{carry},  0, 'carry starts clear');
    is($cpu->flags->{zero},   0, 'zero starts clear');
    is($cpu->flags->{sign},   0, 'sign starts clear');
    is($cpu->flags->{parity}, 0, 'parity starts clear');
};

# ===========================================================================
# Integration: HLT
# ===========================================================================

subtest 'HLT (0x76)' => sub {
    my ($cpu, $traces) = run_prog([0x76], 10);
    is(scalar @$traces, 1,    'one instruction executed');
    is($traces->[0]{mnemonic}, 'HLT', 'mnemonic is HLT');
    is($cpu->halted, 1, 'CPU halted after 0x76');
    ok(dies { $cpu->step() }, 'step after halt dies');
};

subtest 'HLT (0xFF)' => sub {
    my ($cpu) = run_prog([0xFF], 10);
    is($cpu->halted, 1, 'CPU halted after 0xFF');
};

# ===========================================================================
# Integration: MVI — all registers
# ===========================================================================

subtest 'MVI register' => sub {
    # MVI A, 0x42  (0x3E 0x42)
    my ($cpu) = run_prog([0x3E, 0x42, 0x76]);
    is($cpu->a, 0x42, 'MVI A loads accumulator');

    # MVI B, 0x01
    ($cpu) = run_prog([0x06, 0x01, 0x76]);
    is($cpu->b, 0x01, 'MVI B loads register B');

    # MVI C, 0x02
    ($cpu) = run_prog([0x0E, 0x02, 0x76]);
    is($cpu->c, 0x02, 'MVI C loads register C');

    # MVI D, 0x03
    ($cpu) = run_prog([0x16, 0x03, 0x76]);
    is($cpu->d, 0x03, 'MVI D loads register D');

    # MVI E, 0x04
    ($cpu) = run_prog([0x1E, 0x04, 0x76]);
    is($cpu->e, 0x04, 'MVI E loads register E');

    # MVI H, 0x05
    ($cpu) = run_prog([0x26, 0x05, 0x76]);
    is($cpu->h, 0x05, 'MVI H loads register H');

    # MVI L, 0x06
    ($cpu) = run_prog([0x2E, 0x06, 0x76]);
    is($cpu->l, 0x06, 'MVI L loads register L');
};

# ===========================================================================
# Integration: MOV — register-to-register transfer
# ===========================================================================

subtest 'MOV register' => sub {
    # MVI B, 0x5A; MOV A, B (0x78); HLT
    my ($cpu) = run_prog([0x06, 0x5A, 0x78, 0x76]);
    is($cpu->a, 0x5A, 'MOV A, B copies B→A');

    # MVI A, 0x33; MOV B, A (0x47); HLT
    ($cpu) = run_prog([0x3E, 0x33, 0x47, 0x76]);
    is($cpu->b, 0x33, 'MOV B, A copies A→B');

    # MVI A, 0x11; MOV C, A (0x4F); MOV D, A (0x57); HLT
    # Note: MOV D, C = 0x51 is actually IN 2 (all 01_xxx_001 are IN instructions)
    # Use MOV D, A (0x57 = 01_010_111) instead since sss=7(A) is not conflicted.
    ($cpu) = run_prog([0x3E, 0x11, 0x4F, 0x57, 0x76]);
    is($cpu->c, 0x11, 'MOV C, A works');
    is($cpu->d, 0x11, 'MOV D, A works');
};

# ===========================================================================
# Integration: Memory via M pseudo-register
# ===========================================================================

subtest 'MOV with M (memory)' => sub {
    # Write 0xAB to memory[0x0100]:
    #   MVI H, 0x01 ; MVI L, 0x00 ; MVI M, 0xAB (writes to memory[0x0100])
    # Then read back via L:
    #   MOV L, M (0x6E) ; MOV A, L (0x7D) ; HLT
    my ($cpu) = run_prog([
        0x26, 0x01,   # MVI H, 0x01
        0x2E, 0x00,   # MVI L, 0x00
        0x36, 0xAB,   # MVI M, 0xAB → memory[0x0100] = 0xAB
        0x6E,         # MOV L, M    → L = memory[0x0100] = 0xAB
        0x7D,         # MOV A, L    → A = 0xAB
        0x76,         # HLT
    ]);
    is($cpu->a, 0xAB, 'MOV A via M read: A = 0xAB');
    is($cpu->memory->[0x0100], 0xAB, 'memory[0x0100] = 0xAB after MVI M');
};

# ===========================================================================
# Integration: INR / DCR
# ===========================================================================

subtest 'INR / DCR' => sub {
    # INR A: MVI A, 0x41; INR A (0x38); HLT
    my ($cpu) = run_prog([0x3E, 0x41, 0x38, 0x76]);
    is($cpu->a, 0x42, 'INR A: 0x41 → 0x42');

    # DCR A: MVI A, 0x43; DCR A (0x39); HLT
    ($cpu) = run_prog([0x3E, 0x43, 0x39, 0x76]);
    is($cpu->a, 0x42, 'DCR A: 0x43 → 0x42');

    # INR wraps: 0xFF → 0x00, zero flag set, carry NOT changed
    ($cpu) = run_prog([0x3E, 0xFF, 0x38, 0x76]);
    is($cpu->a,                0, 'INR 0xFF → 0');
    is($cpu->flags->{zero},    1, 'INR wrap: zero flag');
    is($cpu->flags->{carry},   0, 'INR wrap: carry NOT set');

    # DCR wraps: 0x00 → 0xFF, sign flag
    ($cpu) = run_prog([0x3E, 0x00, 0x39, 0x76]);
    is($cpu->a,                0xFF, 'DCR 0x00 → 0xFF');
    is($cpu->flags->{sign},    1,    'DCR wrap: sign flag');
    is($cpu->flags->{carry},   0,    'DCR wrap: carry NOT set');
};

# ===========================================================================
# Integration: ALU register ops (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP)
# ===========================================================================

subtest 'ADD register (via ripple_carry_adder)' => sub {
    # MVI B, 1; MVI A, 2; ADD B (0x80); HLT
    my ($cpu) = run_prog([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
    is($cpu->a,              3, 'ADD B: A = 2+1 = 3');
    is($cpu->flags->{carry}, 0, 'ADD 2+1: no carry');

    # Overflow: MVI A, 0xFF; MVI B, 0x01; ADD B → A=0, carry=1
    ($cpu) = run_prog([0x3E, 0xFF, 0x06, 0x01, 0x80, 0x76]);
    is($cpu->a,              0, 'ADD overflow → A=0');
    is($cpu->flags->{carry}, 1, 'ADD overflow: carry set');
    is($cpu->flags->{zero},  1, 'ADD overflow: zero set');
};

subtest 'SUB register (via two\'s complement gates)' => sub {
    # MVI B, 3; MVI A, 5; SUB B (0x90); HLT → A=2
    my ($cpu) = run_prog([0x06, 0x03, 0x3E, 0x05, 0x90, 0x76]);
    is($cpu->a,              2, 'SUB B: A = 5-3 = 2');
    is($cpu->flags->{carry}, 0, 'SUB 5-3: no borrow');

    # Borrow: MVI A, 3; MVI B, 5; SUB B → CY=1
    ($cpu) = run_prog([0x06, 0x05, 0x3E, 0x03, 0x90, 0x76]);
    is($cpu->flags->{carry}, 1, 'SUB 3-5: borrow set');
};

subtest 'ANA / XRA / ORA (bitwise gate ops)' => sub {
    # ANA: 0b10101010 & 0b11001100 = 0b10001000
    my ($cpu) = run_prog([0x3E, 0b10101010, 0x06, 0b11001100, 0xA0, 0x76]);
    is($cpu->a, 0b10001000, 'ANA B: 0xAA & 0xCC = 0x88');

    # XRA: 0xFF ^ 0xFF = 0 → zero flag
    ($cpu) = run_prog([0x3E, 0xFF, 0x06, 0xFF, 0xA8, 0x76]);
    is($cpu->a,              0, 'XRA B: 0xFF ^ 0xFF = 0');
    is($cpu->flags->{zero},  1, 'XRA: zero flag');

    # ORA: 0b10101010 | 0b01010101 = 0xFF
    ($cpu) = run_prog([0x3E, 0b10101010, 0x06, 0b01010101, 0xB0, 0x76]);
    is($cpu->a, 0xFF, 'ORA B: 0xAA | 0x55 = 0xFF');
};

subtest 'CMP register (compare, no result stored)' => sub {
    # MVI A, 5; MVI B, 5; CMP B (0xB8); HLT → A unchanged, zero flag
    my ($cpu) = run_prog([0x3E, 0x05, 0x06, 0x05, 0xB8, 0x76]);
    is($cpu->a,             5, 'CMP B: A unchanged = 5');
    is($cpu->flags->{zero}, 1, 'CMP 5==5: zero flag set');

    # MVI A, 3; MVI B, 5; CMP B → borrow (unsigned 3 < 5)
    ($cpu) = run_prog([0x3E, 0x03, 0x06, 0x05, 0xB8, 0x76]);
    is($cpu->a,              3, 'CMP B: A unchanged = 3');
    is($cpu->flags->{carry}, 1, 'CMP 3<5: carry (borrow) set');
};

# ===========================================================================
# Integration: ALU immediate ops
# ===========================================================================

subtest 'ALU immediate (ADI, SUI, ANI, XRI, ORI, CPI)' => sub {
    # ADI 0x05: MVI A, 0x10; ADI 0x05 (0xC4, 0x05); HLT
    my ($cpu) = run_prog([0x3E, 0x10, 0xC4, 0x05, 0x76]);
    is($cpu->a, 0x15, 'ADI 0x05: A = 0x10 + 0x05 = 0x15');

    # SUI 0x03: MVI A, 0x10; SUI 0x03 (0xD4, 0x03); HLT
    ($cpu) = run_prog([0x3E, 0x10, 0xD4, 0x03, 0x76]);
    is($cpu->a, 0x0D, 'SUI 0x03: A = 0x10 - 0x03 = 0x0D');

    # ANI: MVI A, 0xFF; ANI 0x0F (0xE4, 0x0F); HLT
    ($cpu) = run_prog([0x3E, 0xFF, 0xE4, 0x0F, 0x76]);
    is($cpu->a, 0x0F, 'ANI 0x0F: A = 0xFF & 0x0F = 0x0F');

    # XRI: MVI A, 0xFF; XRI 0xFF (0xEC, 0xFF); HLT → A=0
    ($cpu) = run_prog([0x3E, 0xFF, 0xEC, 0xFF, 0x76]);
    is($cpu->a,             0, 'XRI 0xFF: A = 0xFF ^ 0xFF = 0');
    is($cpu->flags->{zero}, 1, 'XRI: zero flag');

    # ORI: MVI A, 0x0F; ORI 0xF0 (0xF4, 0xF0); HLT → A=0xFF
    ($cpu) = run_prog([0x3E, 0x0F, 0xF4, 0xF0, 0x76]);
    is($cpu->a, 0xFF, 'ORI 0xF0: A = 0x0F | 0xF0 = 0xFF');

    # CPI: MVI A, 0x42; CPI 0x42 (0xFC, 0x42); HLT → zero flag, A unchanged
    ($cpu) = run_prog([0x3E, 0x42, 0xFC, 0x42, 0x76]);
    is($cpu->a,             0x42, 'CPI: A unchanged');
    is($cpu->flags->{zero},    1, 'CPI 0x42==0x42: zero flag');
};

# ===========================================================================
# Integration: Rotates (RLC, RRC, RAL, RAR)
# ===========================================================================

subtest 'Rotates' => sub {
    # RLC (opcode 0x02): MVI A, 0x80; RLC; HLT → A=0x01, carry=1
    my ($cpu) = run_prog([0x3E, 0x80, 0x02, 0x76]);
    is($cpu->a,              0x01, 'RLC 0x80 → 0x01');
    is($cpu->flags->{carry}, 1,    'RLC 0x80: carry=1');

    # RRC (opcode 0x0A): MVI A, 0x01; RRC; HLT → A=0x80, carry=1
    ($cpu) = run_prog([0x3E, 0x01, 0x0A, 0x76]);
    is($cpu->a,              0x80, 'RRC 0x01 → 0x80');
    is($cpu->flags->{carry}, 1,    'RRC 0x01: carry=1');

    # RAL (opcode 0x12): MVI A, 0x80; RAL (carry=0); HLT → A=0x00, carry=1
    ($cpu) = run_prog([0x3E, 0x80, 0x12, 0x76]);
    is($cpu->a,              0x00, 'RAL 0x80 cy=0 → 0x00');
    is($cpu->flags->{carry}, 1,    'RAL 0x80: carry=1');

    # RAR (opcode 0x1A): MVI A, 0x01; RAR (carry=0); HLT → A=0x00, carry=1
    ($cpu) = run_prog([0x3E, 0x01, 0x1A, 0x76]);
    is($cpu->a,              0x00, 'RAR 0x01 cy=0 → 0x00');
    is($cpu->flags->{carry}, 1,    'RAR 0x01: carry=1');
};

# ===========================================================================
# Integration: JMP and conditional jumps
# ===========================================================================

subtest 'JMP (unconditional)' => sub {
    # JMP 0x0006; MVI A, 0xFF (skipped); MVI A, 0x42; HLT
    # Program layout:
    #   0x0000: JMP 0x0006 → bytes: 0x7C, 0x06, 0x00
    #   0x0003: MVI A, 0xFF (0x3E, 0xFF) ← skipped
    #   0x0005: HLT (0x76) ← skipped
    #   0x0006: MVI A, 0x42 (0x3E, 0x42)
    #   0x0008: HLT (0x76)
    my ($cpu) = run_prog([0x7C, 0x06, 0x00, 0x3E, 0xFF, 0x76, 0x3E, 0x42, 0x76]);
    is($cpu->a, 0x42, 'JMP skips over MVI A, 0xFF and reaches MVI A, 0x42');
};

subtest 'JFC/JTC — conditional jump on carry' => sub {
    # JFC 0x0006 (0x40, lo, hi): jump if carry=0
    # MVI A, 0x01; MVI B, 0x01; SUB B → A=0, carry=0
    # JFC 0x000A → taken (carry=0)
    # MVI A, 0xFF (at 0x0008) ← skipped
    # HLT (at 0x000A)
    my @prog;
    # 0x0000: MVI A, 0x01
    push @prog, 0x3E, 0x01;
    # 0x0002: MVI B, 0x01
    push @prog, 0x06, 0x01;
    # 0x0004: SUB B (0x90) → A=0, carry=0
    push @prog, 0x90;
    # 0x0005: JFC 0x000A (0x40, 0x0A, 0x00)
    push @prog, 0x40, 0x0A, 0x00;
    # 0x0008: MVI A, 0xFF (skipped)
    push @prog, 0x3E, 0xFF;
    # 0x000A: HLT
    push @prog, 0x76;

    my ($cpu) = run_prog(\@prog);
    is($cpu->a,             0, 'JFC: A=0 (MVI A,0xFF was skipped)');
    is($cpu->flags->{zero}, 1, 'JFC: zero flag set after SUB');
};

# ===========================================================================
# Integration: CALL and RET
# ===========================================================================

subtest 'CAL and RET (unconditional)' => sub {
    # CAL 0x0006 (0x7E, lo, hi); MVI A, 0xFF; HLT
    # Subroutine at 0x0006: MVI A, 0x42; RET (0x3F); HLT
    my @prog;
    # 0x0000: CAL 0x0006
    push @prog, 0x7E, 0x06, 0x00;
    # 0x0003: MVI A, 0xFF (should not run — CAL returns to here, then next)
    push @prog, 0x3E, 0xFF;
    # 0x0005: HLT
    push @prog, 0x76;
    # 0x0006: MVI A, 0x42
    push @prog, 0x3E, 0x42;
    # 0x0008: RET (0x3F)
    push @prog, 0x3F;

    my ($cpu, $traces) = run_prog(\@prog);
    # After CAL+RET, execution returns to 0x0003 (after the 3-byte CAL)
    # Then runs MVI A, 0xFF, then HLT
    is($cpu->a, 0xFF, 'CAL/RET: subroutine ran MVI A, 0x42, then return runs MVI A, 0xFF');
    is($cpu->halted, 1, 'CPU halted after full call/return sequence');
};

# ===========================================================================
# Integration: RST
# ===========================================================================

subtest 'RST' => sub {
    # RST 1 = opcode 00_001_101 = 0x0D; jumps to (1 << 3) = 0x0008
    # Program at 0: RST 1 (0x0D); MVI A, 0xFF (skipped because RST jumps away)
    # Subroutine at 0x0008: MVI A, 0x42; HLT
    my @prog = (0) x 16;
    $prog[0] = 0x0D;          # RST 1 (00 001 101 = 0x0D)
    $prog[1] = 0x3E;          # MVI A, 0xFF (skipped — RST jumped away)
    $prog[2] = 0xFF;
    $prog[3] = 0x76;          # HLT (also skipped)
    $prog[8] = 0x3E;          # MVI A, 0x42 at RST 1 target (0x0008)
    $prog[9] = 0x42;
    $prog[10] = 0x76;         # HLT

    my ($cpu) = run_prog(\@prog);
    is($cpu->a, 0x42, 'RST 1 (0x0D) jumps to 0x0008 and runs MVI A, 0x42');
};

# ===========================================================================
# Integration: IN / OUT
# ===========================================================================

subtest 'IN / OUT' => sub {
    # IN 3: read from input port 3 → A
    # OUT 4: write A to output port 4  (opcode 0x22)
    my $cpu = CodingAdventures::Intel8008GateLevel->new();
    $cpu->set_input_port(3, 0x99);
    # IN 3 = group=01, ddd=3, sss=001 → opcode: 01_011_001 = 0x59
    $cpu->run([0x59, 0x22, 0x76], 10);
    is($cpu->a,                  0x99, 'IN 3: A = port 3 value');
    is($cpu->get_output_port(4), 0x99, 'OUT 4: port 4 received A');
};

# ===========================================================================
# Integration: Flags — zero, sign, parity, carry
# ===========================================================================

subtest 'Flag computation (gate-level)' => sub {
    # Zero flag: MVI A, 0xFF; INR A → A=0, zero=1
    my ($cpu) = run_prog([0x3E, 0xFF, 0x38, 0x76]);
    is($cpu->flags->{zero}, 1, 'zero flag after INR 0xFF → 0');

    # Sign flag: MVI A, 0x7F; INR A → A=0x80, sign=1
    ($cpu) = run_prog([0x3E, 0x7F, 0x38, 0x76]);
    is($cpu->flags->{sign}, 1, 'sign flag after INR 0x7F → 0x80');

    # Parity flag: 0x03 = 0000_0011 → 2 ones → even parity P=1
    # ADI 0x02: MVI A,0x01; ADI 0x02 → A=0x03
    ($cpu) = run_prog([0x3E, 0x01, 0xC4, 0x02, 0x76]);
    is($cpu->flags->{parity}, 1, 'parity=1 for 0x03 (even parity)');

    # Parity flag: 0x01 = 0000_0001 → 1 one → odd parity P=0
    ($cpu) = run_prog([0x3E, 0x01, 0x76]);
    # After MVI, flags are not updated. Use ADI 0 to force flag recompute.
    ($cpu) = run_prog([0x3E, 0x00, 0xC4, 0x01, 0x76]);
    is($cpu->flags->{parity}, 0, 'parity=0 for 0x01 (odd parity)');
};

# ===========================================================================
# Integration: Counter loop (multi-instruction program)
# ===========================================================================

subtest 'Counter loop — multi-instruction program' => sub {
    # Count from 5 down to 0 using DCR and conditional jump:
    #   0x0000: MVI A, 5      (0x3E, 0x05)
    #   0x0002: DCR A (0x39) ← loop top
    #   0x0003: JFZ 0x0002 (0x48, 0x02, 0x00)  ← jump if zero=0 (not zero yet)
    #   0x0006: HLT (0x76)
    my ($cpu) = run_prog([0x3E, 0x05, 0x39, 0x48, 0x02, 0x00, 0x76]);
    is($cpu->a,             0, 'counter loop: A = 0 after counting down from 5');
    is($cpu->flags->{zero}, 1, 'counter loop: zero flag set at end');
    is($cpu->halted,        1, 'counter loop: CPU halted');
};

# ===========================================================================
# Cross-validation: gate-level vs behavioral simulator
# ===========================================================================

SKIP: {
    skip "CodingAdventures::Intel8008Simulator not available", 3 unless $HAS_BSIM;

    subtest 'Cross-validation: ADD program' => sub {
        # MVI B,1; MVI A,2; ADD B; HLT
        my $program = [0x06, 0x01, 0x3E, 0x02, 0x80, 0x76];

        my $bsim = CodingAdventures::Intel8008Simulator->new();
        my $gsim = CodingAdventures::Intel8008GateLevel->new();

        my $btraces = $bsim->run($program, 100);
        my $gtraces = $gsim->run($program, 100);

        is(scalar @$gtraces, scalar @$btraces, 'same number of trace steps');
        is($gsim->a, $bsim->a, 'register A matches');
        is($gsim->flags->{carry},  $bsim->flags->{carry},  'carry flag matches');
        is($gsim->flags->{zero},   $bsim->flags->{zero},   'zero flag matches');
        is($gsim->flags->{sign},   $bsim->flags->{sign},   'sign flag matches');
        is($gsim->flags->{parity}, $bsim->flags->{parity}, 'parity flag matches');
    };

    subtest 'Cross-validation: subtract and loop' => sub {
        # MVI A,0x10; MVI B,0x05; SUB B; ADI 0x01; HLT
        my $program = [0x3E, 0x10, 0x06, 0x05, 0x90, 0xC4, 0x01, 0x76];

        my $bsim = CodingAdventures::Intel8008Simulator->new();
        my $gsim = CodingAdventures::Intel8008GateLevel->new();

        $bsim->run($program, 100);
        $gsim->run($program, 100);

        is($gsim->a, $bsim->a, 'register A matches after SUB+ADI');
        is($gsim->flags->{carry},  $bsim->flags->{carry},  'carry matches');
        is($gsim->flags->{zero},   $bsim->flags->{zero},   'zero matches');
        is($gsim->flags->{sign},   $bsim->flags->{sign},   'sign matches');
        is($gsim->flags->{parity}, $bsim->flags->{parity}, 'parity matches');
    };

    subtest 'Cross-validation: all-flags program' => sub {
        # Program that exercises carry, zero, sign, parity across multiple ops:
        # MVI A, 0xFF; MVI B, 0x01; ADD B (overflow → carry, zero)
        # ADI 0x01 (A=1, odd parity)
        # MVI A, 0x7F; INR A (A=0x80, sign, even parity)
        # HLT
        my $program = [
            0x3E, 0xFF,   # MVI A, 0xFF
            0x06, 0x01,   # MVI B, 0x01
            0x80,         # ADD B → A=0, carry=1, zero=1
            0xC4, 0x01,   # ADI 0x01 → A=1
            0x3E, 0x7F,   # MVI A, 0x7F (MVI doesn't set flags)
            0x38,         # INR A → A=0x80, sign=1
            0x76,         # HLT
        ];

        my $bsim = CodingAdventures::Intel8008Simulator->new();
        my $gsim = CodingAdventures::Intel8008GateLevel->new();

        $bsim->run($program, 100);
        $gsim->run($program, 100);

        is($gsim->a, $bsim->a, 'A matches');
        is($gsim->b, $bsim->b, 'B matches');
        is($gsim->flags->{carry},  $bsim->flags->{carry},  'carry matches');
        is($gsim->flags->{zero},   $bsim->flags->{zero},   'zero matches');
        is($gsim->flags->{sign},   $bsim->flags->{sign},   'sign matches');
        is($gsim->flags->{parity}, $bsim->flags->{parity}, 'parity matches');
    };
}

done_testing();
