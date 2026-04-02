#!/usr/bin/perl
use strict;
use warnings;

# Tests for CodingAdventures::Intel4004GateLevel
#
# Every ALU operation routes through real gate functions from
# CodingAdventures::LogicGates and CodingAdventures::Arithmetic.
# We test the same ISA as the behavioral simulator but via gate-level circuits.
# Target: 95%+ coverage.

use Test2::V0;
use lib '../lib';
use lib '../../logic-gates/lib';
use lib '../../arithmetic/lib';
use CodingAdventures::Intel4004GateLevel;

# ---------------------------------------------------------------------------
# Helper: create a fresh CPU and run a program
# ---------------------------------------------------------------------------
sub run_prog {
    my ($bytes, $max_steps) = @_;
    $max_steps //= 10_000;
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    my $traces = $cpu->run($bytes, $max_steps);
    return ($cpu, $traces);
}

# ---------------------------------------------------------------------------
# Initialization
# ---------------------------------------------------------------------------

subtest 'initialization' => sub {
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    is($cpu->accumulator, 0, 'accumulator starts at 0');
    is($cpu->carry,       0, 'carry starts clear');
    is($cpu->pc,          0, 'PC starts at 0');
    is($cpu->halted,      0, 'not halted at start');
    for my $r (0..15) {
        is($cpu->get_register($r), 0, "register $r starts at 0");
    }
};

# ---------------------------------------------------------------------------
# Gate count
# ---------------------------------------------------------------------------

subtest 'gate_count' => sub {
    my $gc = CodingAdventures::Intel4004GateLevel::gate_count();
    ok($gc->{total} > 0,  'total gate count > 0');
    ok($gc->{alu} > 0,    'ALU gate count > 0');
    is($gc->{total}, 716, 'total gate count = 716');
};

# ---------------------------------------------------------------------------
# NOP and HLT
# ---------------------------------------------------------------------------

subtest 'NOP and HLT' => sub {
    my ($cpu, $traces) = run_prog([0x00, 0x01], 10);
    is(scalar @$traces,        2,     'two instructions executed');
    is($traces->[0]{mnemonic}, 'NOP', 'first is NOP');
    is($traces->[1]{mnemonic}, 'HLT', 'second is HLT');
    is($cpu->halted, 1, 'CPU halted after HLT');

    ok(dies { $cpu->step() }, 'step after halt dies');
};

# ---------------------------------------------------------------------------
# LDM (gate-level path: writes ACC via flip-flop)
# ---------------------------------------------------------------------------

subtest 'LDM' => sub {
    for my $n (0..15) {
        my ($cpu) = run_prog([0xD0 | $n, 0x01], 10);
        is($cpu->accumulator, $n, "LDM $n -> A=$n (via flip-flop)");
    }
};

# ---------------------------------------------------------------------------
# LD and XCH
# ---------------------------------------------------------------------------

subtest 'LD and XCH' => sub {
    # LD: load register into accumulator
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    $cpu->_write_reg(5, 9);
    $cpu->run([0xA5, 0x01], 10);
    is($cpu->accumulator, 9, 'LD R5 loads 9 into accumulator');

    # XCH: swap accumulator and register
    my ($cpu2) = run_prog([0xD7, 0xB2, 0x01], 10);  # LDM 7, XCH R2
    is($cpu2->accumulator,    0, 'XCH: A=0 after swap with R2=0');
    is($cpu2->get_register(2), 7, 'XCH: R2=7 after swap');
};

# ---------------------------------------------------------------------------
# INC via half-adder chain
# ---------------------------------------------------------------------------

subtest 'INC via half-adder' => sub {
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    $cpu->_write_reg(0, 6);
    $cpu->run([0x60, 0x01], 10);
    is($cpu->get_register(0), 7, 'INC R0: 6->7 via half-adder');

    # Wrap at 15
    my $cpu2 = CodingAdventures::Intel4004GateLevel->new();
    $cpu2->_write_reg(3, 15);
    $cpu2->run([0x63, 0x01], 10);
    is($cpu2->get_register(3), 0, 'INC R3: 15->0 (wrap via half-adder)');
    is($cpu2->carry,           0, 'INC does not affect carry');
};

# ---------------------------------------------------------------------------
# ADD via ripple-carry adder
# ---------------------------------------------------------------------------

subtest 'ADD via ripple-carry adder' => sub {
    # 3 + 4 = 7, no carry
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    $cpu->run([0xD3, 0x01], 10);  # LDM 3, HLT
    $cpu->reset();
    $cpu->_write_reg(2, 4);
    $cpu->run([0xD3, 0x82, 0x01], 10);   # LDM 3, ADD R2
    is($cpu->accumulator, 7, 'ADD: 3+4=7 via ripple-carry');
    is($cpu->carry,       0, 'no carry on 3+4');

    # 9 + 9 = 18 -> A=2, carry=1
    my $cpu2 = CodingAdventures::Intel4004GateLevel->new();
    $cpu2->_write_reg(0, 9);
    $cpu2->run([0xD9, 0x80, 0x01], 10);
    is($cpu2->accumulator, 2, 'ADD: 9+9=18 -> A=2 (gate-level overflow)');
    is($cpu2->carry,       1, 'carry set on overflow');

    # ADD uses carry_in (stored in flip-flop)
    my $cpu3 = CodingAdventures::Intel4004GateLevel->new();
    $cpu3->_write_reg(0, 3);
    $cpu3->_write_carry(1);
    $cpu3->run([0xD5, 0x80, 0x01], 10);   # LDM 5, ADD R0 (carry=1)
    is($cpu3->accumulator, 9, 'ADD with carry_in via gate: 5+3+1=9');
};

# ---------------------------------------------------------------------------
# SUB via gate NOT + ripple-carry adder
# ---------------------------------------------------------------------------

subtest 'SUB via NOT + ripple-carry' => sub {
    # 7 - 3 = 4, carry=1 (no borrow)
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    $cpu->_write_reg(1, 3);
    $cpu->run([0xD7, 0x91, 0x01], 10);   # LDM 7, SUB R1
    is($cpu->accumulator, 4, 'SUB: 7-3=4 (NOT+add)');
    is($cpu->carry,       1, 'carry=1 (no borrow) after 7-3');

    # 3 - 7 wraps to 12, carry=0 (borrow)
    my $cpu2 = CodingAdventures::Intel4004GateLevel->new();
    $cpu2->_write_reg(0, 7);
    $cpu2->run([0xD3, 0x90, 0x01], 10);
    is($cpu2->accumulator, 12, 'SUB: 3-7 wraps to 12');
    is($cpu2->carry,        0, 'carry=0 (borrow) after 3-7');
};

# ---------------------------------------------------------------------------
# Carry ops (CLB, CLC, STC, CMC via NOT gate)
# ---------------------------------------------------------------------------

subtest 'carry ops via gates' => sub {
    my ($cpu) = run_prog([0xFA, 0xF1, 0x01], 10);  # STC, CLC
    is($cpu->carry, 0, 'CLC clears carry');

    my ($cpu2) = run_prog([0xFA, 0x01], 10);
    is($cpu2->carry, 1, 'STC sets carry');

    # CMC uses NOT gate
    my ($cpu3) = run_prog([0xFA, 0xF3, 0x01], 10);  # STC, CMC
    is($cpu3->carry, 0, 'CMC (NOT gate): 1->0');
    my ($cpu4) = run_prog([0xF1, 0xF3, 0x01], 10);  # CLC, CMC
    is($cpu4->carry, 1, 'CMC (NOT gate): 0->1');
};

# ---------------------------------------------------------------------------
# IAC via gate_add (carry_in=1)
# ---------------------------------------------------------------------------

subtest 'IAC via gate_add' => sub {
    my ($cpu) = run_prog([0xD5, 0xF2, 0x01], 10);   # LDM 5, IAC
    is($cpu->accumulator, 6, 'IAC: 5->6 via gate_add');

    my ($cpu2) = run_prog([0xDF, 0xF2, 0x01], 10);  # LDM 15, IAC
    is($cpu2->accumulator, 0, 'IAC: 15->0 overflow');
    is($cpu2->carry,       1, 'IAC: carry set on overflow');
};

# ---------------------------------------------------------------------------
# DAC via gate_add + NOT(1)
# ---------------------------------------------------------------------------

subtest 'DAC' => sub {
    my ($cpu) = run_prog([0xD5, 0xF8, 0x01], 10);  # LDM 5, DAC
    is($cpu->accumulator, 4, 'DAC: 5->4');
    is($cpu->carry,       1, 'DAC: carry=1 (no borrow) from 5');

    my ($cpu2) = run_prog([0xD0, 0xF8, 0x01], 10); # LDM 0, DAC
    is($cpu2->accumulator, 15, 'DAC: 0->15 (underflow)');
    is($cpu2->carry,        0, 'DAC: carry=0 (borrow) from 0');
};

# ---------------------------------------------------------------------------
# CMA via NOT gates
# ---------------------------------------------------------------------------

subtest 'CMA via NOT gates' => sub {
    my ($cpu) = run_prog([0xD5, 0xF4, 0x01], 10);  # LDM 5, CMA
    is($cpu->accumulator, 10, 'CMA: ~5 = 10 (via NOT gates)');

    my ($cpu2) = run_prog([0xDF, 0xF4, 0x01], 10);
    is($cpu2->accumulator, 0, 'CMA: ~15 = 0');
};

# ---------------------------------------------------------------------------
# RAL and RAR
# ---------------------------------------------------------------------------

subtest 'RAL and RAR' => sub {
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    $cpu->_write_carry(0);
    $cpu->run([0xD6, 0xF5, 0x01], 10);  # LDM 6 (0110), RAL
    is($cpu->accumulator, 12, 'RAL: 6->12 (shift left, carry=0 in)');
    is($cpu->carry,        0, 'RAL: new carry=0 (bit3 of 0110)');

    my $cpu2 = CodingAdventures::Intel4004GateLevel->new();
    $cpu2->_write_carry(0);
    $cpu2->run([0xD6, 0xF6, 0x01], 10);  # LDM 6 (0110), RAR
    is($cpu2->accumulator, 3, 'RAR: 6->3 (shift right, carry=0 in)');
    is($cpu2->carry,       0, 'RAR: new carry=0 (bit0 of 0110)');
};

# ---------------------------------------------------------------------------
# DAA via gate_add
# ---------------------------------------------------------------------------

subtest 'DAA via gate_add' => sub {
    my ($cpu) = run_prog([0xD7, 0xFB, 0x01], 10);  # LDM 7, DAA (no adjust)
    is($cpu->accumulator, 7, 'DAA: A=7, no adjustment');

    my ($cpu2) = run_prog([0xDA, 0xFB, 0x01], 10);  # LDM 10, DAA
    is($cpu2->accumulator, 0, 'DAA: 10+6=16 -> A=0 overflow');
    is($cpu2->carry,       1, 'DAA: carry set on overflow');
};

# ---------------------------------------------------------------------------
# KBP
# ---------------------------------------------------------------------------

subtest 'KBP' => sub {
    my %expected = (0=>0, 1=>1, 2=>2, 4=>3, 8=>4);
    for my $input (sort keys %expected) {
        my $cpu = CodingAdventures::Intel4004GateLevel->new();
        $cpu->run([0xD0|$input, 0xFC, 0x01], 10);
        is($cpu->accumulator, $expected{$input}, "KBP: $input -> $expected{$input}");
    }
    my ($cpu) = run_prog([0xD3, 0xFC, 0x01], 10);
    is($cpu->accumulator, 15, 'KBP: 3 -> 15 (error)');
};

# ---------------------------------------------------------------------------
# WRM and RDM (RAM via flip-flop states)
# ---------------------------------------------------------------------------

subtest 'WRM and RDM via flip-flops' => sub {
    # FIM P0,0x00; SRC P0; LDM 7; WRM; LDM 0; RDM; HLT
    my ($cpu) = run_prog([0x20,0x00, 0x21, 0xD7, 0xE0, 0xD0, 0xE9, 0x01], 10);
    is($cpu->accumulator, 7, 'RDM reads back value written by WRM (flip-flop)');
};

# ---------------------------------------------------------------------------
# JUN
# ---------------------------------------------------------------------------

subtest 'JUN' => sub {
    my @prog = (0) x 4096;
    $prog[0] = 0x40; $prog[1] = 0x10;  # JUN 0x010
    $prog[0x10] = 0x01;                  # HLT at 0x10
    my ($cpu) = run_prog(\@prog, 100);
    is($cpu->halted, 1, 'JUN: reaches HLT at target');
};

# ---------------------------------------------------------------------------
# JCN
# ---------------------------------------------------------------------------

subtest 'JCN' => sub {
    # cond=4 (test zero), A=0 -> jump taken
    my @prog = (0) x 4096;
    $prog[0] = 0x14; $prog[1] = 0x05;  # JCN cond=4, target=5
    $prog[2] = 0xD7; $prog[3] = 0x01;  # LDM 7, HLT (skipped)
    $prog[5] = 0x01;                    # HLT at target
    my ($cpu) = run_prog(\@prog, 100);
    is($cpu->accumulator, 0, 'JCN taken: LDM 7 skipped');
};

# ---------------------------------------------------------------------------
# JMS and BBL (stack via flip-flops)
# ---------------------------------------------------------------------------

subtest 'JMS and BBL via flip-flop stack' => sub {
    my @prog = (0) x 4096;
    $prog[0] = 0x50; $prog[1] = 0x06;  # JMS 0x006
    $prog[2] = 0x01;                    # HLT (return point)
    $prog[6] = 0xC5;                    # BBL 5

    my ($cpu) = run_prog(\@prog, 100);
    is($cpu->accumulator, 5, 'BBL: A=5 after return via flip-flop stack');
    is($cpu->halted,      1, 'returned to HLT correctly');
};

# ---------------------------------------------------------------------------
# ISZ via half-adder chain
# ---------------------------------------------------------------------------

subtest 'ISZ via half-adder' => sub {
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    $cpu->_write_reg(0, 0xE);  # 14; wraps to 0 after 2 increments
    $cpu->run([0x70, 0x00, 0x01], 10);  # ISZ R0,0; HLT
    is($cpu->get_register(0), 0, 'ISZ: R0 wrapped to 0');
    is($cpu->halted,          1, 'ISZ: fell through to HLT');
};

# ---------------------------------------------------------------------------
# Reset
# ---------------------------------------------------------------------------

subtest 'reset' => sub {
    my $cpu = CodingAdventures::Intel4004GateLevel->new();
    $cpu->run([0xD7, 0xFA, 0x01], 10);  # LDM 7, STC, HLT
    is($cpu->accumulator, 7, 'before reset A=7');
    is($cpu->carry,       1, 'before reset carry=1');
    $cpu->reset();
    is($cpu->accumulator, 0, 'after reset A=0');
    is($cpu->carry,       0, 'after reset carry=0');
    is($cpu->pc,          0, 'after reset PC=0');
    is($cpu->halted,      0, 'after reset not halted');
};

# ---------------------------------------------------------------------------
# Integration: 1 + 2 = 3 via gate-level adder
# ---------------------------------------------------------------------------

subtest 'integration: 1+2 via gate-level adder' => sub {
    # LDM 1, XCH R0, LDM 2, ADD R0, HLT
    my ($cpu) = run_prog([0xD1, 0xB0, 0xD2, 0x80, 0x01], 10);
    is($cpu->accumulator, 3, '1 + 2 = 3 via ripple-carry adder');
    is($cpu->carry,       0, 'no carry on 1+2');
};

# ---------------------------------------------------------------------------
# Integration: 3 x 4 via ISZ loop (gate-level half-adder increment)
# ---------------------------------------------------------------------------

subtest 'integration: 3x4 via ISZ loop (gate-level)' => sub {
    # Same loop as behavioral test, but every increment goes through half-adder
    my ($cpu) = run_prog([
        0x20, 0xC0,   # FIM P0: R0=12, R1=0
        0xD3,         # LDM 3
        0x81,         # ADD R1
        0xB1,         # XCH R1
        0x70, 0x02,   # ISZ R0, target=2
        0xA1,         # LD R1
        0x01,         # HLT
    ], 100);
    is($cpu->accumulator, 12, '3 x 4 = 12 via ISZ/half-adder loop');
};

done_testing();
