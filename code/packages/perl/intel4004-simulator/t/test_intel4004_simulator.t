#!/usr/bin/perl
use strict;
use warnings;

# Tests for CodingAdventures::Intel4004Simulator
#
# We test every instruction category plus end-to-end programs.
# Target: 95%+ coverage of the Intel 4004 ISA.

use Test2::V0;
use lib '../lib';
use CodingAdventures::Intel4004Simulator;

# ---------------------------------------------------------------------------
# Helper: create a fresh CPU and run a program
# ---------------------------------------------------------------------------
sub run_prog {
    my ($bytes, $max_steps) = @_;
    $max_steps //= 10_000;
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    my $traces = $cpu->run($bytes, $max_steps);
    return ($cpu, $traces);
}

# ---------------------------------------------------------------------------
# Initialization
# ---------------------------------------------------------------------------

subtest 'initialization' => sub {
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    is($cpu->accumulator, 0, 'accumulator starts at 0');
    is($cpu->carry,       0, 'carry starts clear');
    is($cpu->pc,          0, 'PC starts at 0');
    is($cpu->halted,      0, 'not halted at start');
    for my $r (0..15) {
        is($cpu->get_register($r), 0, "register $r starts at 0");
    }
};

# ---------------------------------------------------------------------------
# NOP and HLT
# ---------------------------------------------------------------------------

subtest 'NOP and HLT' => sub {
    my ($cpu, $traces) = run_prog([0x00, 0x01], 10);
    is(scalar @$traces,   2,     'two instructions executed');
    is($traces->[0]{mnemonic}, 'NOP', 'first is NOP');
    is($traces->[1]{mnemonic}, 'HLT', 'second is HLT');
    is($cpu->halted, 1, 'CPU halted after HLT');

    # step() after halt must die
    ok(dies { $cpu->step() }, 'step after halt dies');
};

# ---------------------------------------------------------------------------
# LDM: Load immediate into accumulator
# ---------------------------------------------------------------------------

subtest 'LDM' => sub {
    for my $n (0..15) {
        my ($cpu) = run_prog([0xD0 | $n, 0x01], 10);
        is($cpu->accumulator, $n, "LDM $n sets A=$n");
    }

    my (undef, $traces) = run_prog([0xD5, 0x01], 10);
    is($traces->[0]{mnemonic},           'LDM 5', 'LDM 5 mnemonic');
    is($traces->[0]{accumulator_before}, 0,       'acc before = 0');
    is($traces->[0]{accumulator_after},  5,       'acc after = 5');
};

# ---------------------------------------------------------------------------
# LD: Load register into accumulator
# ---------------------------------------------------------------------------

subtest 'LD' => sub {
    for my $reg (0..15) {
        my $cpu = CodingAdventures::Intel4004Simulator->new();
        $cpu->{registers}[$reg] = 7;
        $cpu->run([0xA0 | $reg, 0x01], 10);
        is($cpu->accumulator, 7, "LD R$reg loads 7 into accumulator");
    }
};

# ---------------------------------------------------------------------------
# XCH: Exchange accumulator and register
# ---------------------------------------------------------------------------

subtest 'XCH' => sub {
    # LDM 5, XCH R0, HLT
    my ($cpu, $traces) = run_prog([0xD5, 0xB0, 0x01], 10);
    is($cpu->accumulator,   0, 'A = 0 after XCH (was 5, R0 was 0)');
    is($cpu->get_register(0), 5, 'R0 = 5 after XCH');

    is($traces->[1]{mnemonic},           'XCH R0', 'XCH mnemonic');
    is($traces->[1]{accumulator_before}, 5,         'acc before = 5');
    is($traces->[1]{accumulator_after},  0,         'acc after = 0');
};

# ---------------------------------------------------------------------------
# INC: Increment register
# ---------------------------------------------------------------------------

subtest 'INC' => sub {
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{registers}[3] = 5;
    $cpu->run([0x63, 0x01], 10);    # INC R3, HLT
    is($cpu->get_register(3), 6, 'INC R3: 5->6');

    # Modulo 16 wrap
    my $cpu2 = CodingAdventures::Intel4004Simulator->new();
    $cpu2->{registers}[0] = 15;
    $cpu2->run([0x60, 0x01], 10);   # INC R0
    is($cpu2->get_register(0), 0, 'INC R0 wraps 15->0');
    is($cpu2->carry, 0, 'INC does not affect carry');
};

# ---------------------------------------------------------------------------
# ADD: Add register to accumulator with carry
# ---------------------------------------------------------------------------

subtest 'ADD' => sub {
    # 3 + 4 = 7, no carry
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{registers}[2] = 4;
    $cpu->run([0xD3, 0x82, 0x01], 10);   # LDM 3, ADD R2, HLT
    is($cpu->accumulator, 7, 'ADD: 3+4=7');
    is($cpu->carry,       0, 'no carry on 3+4');

    # 9 + 9 = 18, overflow -> carry
    my $cpu2 = CodingAdventures::Intel4004Simulator->new();
    $cpu2->{registers}[0] = 9;
    $cpu2->run([0xD9, 0x80, 0x01], 10);   # LDM 9, ADD R0
    is($cpu2->accumulator, 2, 'ADD: 9+9=18 -> A=2 (mod 16)');
    is($cpu2->carry,       1, 'carry set on overflow');

    # ADD uses carry_in
    my $cpu3 = CodingAdventures::Intel4004Simulator->new();
    $cpu3->{registers}[0] = 3;
    $cpu3->{carry} = 1;
    $cpu3->run([0xD5, 0x80, 0x01], 10);   # LDM 5, ADD R0 (carry=1)
    is($cpu3->accumulator, 9, 'ADD with carry_in: 5+3+1=9');
};

# ---------------------------------------------------------------------------
# SUB: Subtract register from accumulator
# ---------------------------------------------------------------------------

subtest 'SUB' => sub {
    # 7 - 3 = 4, carry=1 (no borrow)
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{registers}[1] = 3;
    $cpu->run([0xD7, 0x91, 0x01], 10);   # LDM 7, SUB R1
    is($cpu->accumulator, 4, 'SUB: 7-3=4');
    is($cpu->carry,       1, 'carry=1 (no borrow) after 7-3');

    # 3 - 7 = -4 -> 12 in 4-bit, carry=0 (borrow occurred)
    my $cpu2 = CodingAdventures::Intel4004Simulator->new();
    $cpu2->{registers}[0] = 7;
    $cpu2->run([0xD3, 0x90, 0x01], 10);   # LDM 3, SUB R0
    is($cpu2->accumulator, 12, 'SUB: 3-7 wraps to 12');
    is($cpu2->carry,        0, 'carry=0 (borrow) after 3-7');
};

# ---------------------------------------------------------------------------
# CLB, CLC, STC, CMC
# ---------------------------------------------------------------------------

subtest 'carry ops' => sub {
    # CLC clears carry
    my ($cpu) = run_prog([0xDA, 0xF1, 0x01], 10);  # LDM 10, CLC, HLT
    is($cpu->carry, 0, 'CLC clears carry');

    # STC sets carry
    my ($cpu2) = run_prog([0xFA, 0x01], 10);
    is($cpu2->carry, 1, 'STC sets carry');

    # CMC toggles carry
    my ($cpu3) = run_prog([0xFA, 0xF3, 0x01], 10);  # STC, CMC, HLT
    is($cpu3->carry, 0, 'CMC clears after STC');
    my ($cpu4) = run_prog([0xF1, 0xF3, 0x01], 10);  # CLC, CMC, HLT
    is($cpu4->carry, 1, 'CMC sets after CLC');

    # CLB clears both A and carry
    my $cpu5 = CodingAdventures::Intel4004Simulator->new();
    $cpu5->{carry} = 1;
    $cpu5->run([0xD7, 0xF0, 0x01], 10);  # LDM 7, CLB, HLT
    is($cpu5->accumulator, 0, 'CLB clears accumulator');
    is($cpu5->carry,       0, 'CLB clears carry');
};

# ---------------------------------------------------------------------------
# IAC: Increment accumulator
# ---------------------------------------------------------------------------

subtest 'IAC' => sub {
    my ($cpu) = run_prog([0xD5, 0xF2, 0x01], 10);  # LDM 5, IAC, HLT
    is($cpu->accumulator, 6, 'IAC: 5->6');
    is($cpu->carry,       0, 'no carry on IAC 5->6');

    # Overflow
    my ($cpu2) = run_prog([0xDF, 0xF2, 0x01], 10);  # LDM 15, IAC
    is($cpu2->accumulator, 0, 'IAC: 15->0 (overflow)');
    is($cpu2->carry,       1, 'carry set on IAC overflow');
};

# ---------------------------------------------------------------------------
# DAC: Decrement accumulator
# ---------------------------------------------------------------------------

subtest 'DAC' => sub {
    my ($cpu) = run_prog([0xD5, 0xF8, 0x01], 10);  # LDM 5, DAC
    is($cpu->accumulator, 4, 'DAC: 5->4');
    is($cpu->carry,       1, 'carry=1 (no borrow) on DAC from 5');

    # Underflow
    my ($cpu2) = run_prog([0xD0, 0xF8, 0x01], 10);  # LDM 0, DAC
    is($cpu2->accumulator, 15, 'DAC: 0->15 (underflow)');
    is($cpu2->carry,        0, 'carry=0 (borrow) on DAC from 0');
};

# ---------------------------------------------------------------------------
# CMA: Complement accumulator
# ---------------------------------------------------------------------------

subtest 'CMA' => sub {
    my ($cpu) = run_prog([0xD5, 0xF4, 0x01], 10);  # LDM 5 (0101), CMA
    is($cpu->accumulator, 10, 'CMA: ~5 (0101) = 10 (1010)');

    my ($cpu2) = run_prog([0xD0, 0xF4, 0x01], 10);  # LDM 0, CMA
    is($cpu2->accumulator, 15, 'CMA: ~0 = 15');
};

# ---------------------------------------------------------------------------
# RAL, RAR: Rotate through carry
# ---------------------------------------------------------------------------

subtest 'RAL RAR' => sub {
    # RAL: A=6 (0110), carry=0 -> A=12 (1100), carry=0
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{carry} = 0;
    $cpu->run([0xD6, 0xF5, 0x01], 10);   # LDM 6, RAL
    is($cpu->accumulator, 12, 'RAL: 6 -> 12 (shift left)');
    is($cpu->carry,        0, 'RAL: new carry = old bit 3 of 0110 = 0');

    # RAL: A=9 (1001), carry=1 -> A=3 (0011), carry=1
    my $cpu2 = CodingAdventures::Intel4004Simulator->new();
    $cpu2->{carry} = 1;
    $cpu2->run([0xD9, 0xF5, 0x01], 10);  # LDM 9 (1001), RAL
    is($cpu2->accumulator, 3, 'RAL: 9+carry_in=1 -> (1001<<1)|1 = 0011 = 3');
    is($cpu2->carry,       1, 'RAL: new carry = old bit 3 of 1001 = 1');

    # RAR: A=6 (0110), carry=0 -> A=3 (0011), carry=0
    my $cpu3 = CodingAdventures::Intel4004Simulator->new();
    $cpu3->{carry} = 0;
    $cpu3->run([0xD6, 0xF6, 0x01], 10);  # LDM 6, RAR
    is($cpu3->accumulator, 3, 'RAR: 6 -> 3 (shift right)');
    is($cpu3->carry,       0, 'RAR: new carry = bit 0 of 0110 = 0');

    # RAR: A=9 (1001), carry=1 -> A=12 (1100), carry=1
    my $cpu4 = CodingAdventures::Intel4004Simulator->new();
    $cpu4->{carry} = 1;
    $cpu4->run([0xD9, 0xF6, 0x01], 10);  # LDM 9, RAR
    is($cpu4->accumulator, 12, 'RAR: 9+carry_in=1 -> (1001>>1)|(1<<3) = 1100 = 12');
    is($cpu4->carry,        1, 'RAR: new carry = bit 0 of 1001 = 1');
};

# ---------------------------------------------------------------------------
# TCC: Transfer carry to accumulator, clear carry
# ---------------------------------------------------------------------------

subtest 'TCC' => sub {
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{carry} = 1;
    $cpu->run([0xF7, 0x01], 10);
    is($cpu->accumulator, 1, 'TCC: carry=1 -> A=1');
    is($cpu->carry,       0, 'TCC: carry cleared');

    my $cpu2 = CodingAdventures::Intel4004Simulator->new();
    $cpu2->{carry} = 0;
    $cpu2->run([0xF7, 0x01], 10);
    is($cpu2->accumulator, 0, 'TCC: carry=0 -> A=0');
};

# ---------------------------------------------------------------------------
# TCS: Transfer carry subtract
# ---------------------------------------------------------------------------

subtest 'TCS' => sub {
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{carry} = 1;
    $cpu->run([0xF9, 0x01], 10);
    is($cpu->accumulator, 10, 'TCS: carry=1 -> A=10');
    is($cpu->carry,        0, 'TCS: carry cleared');

    my $cpu2 = CodingAdventures::Intel4004Simulator->new();
    $cpu2->{carry} = 0;
    $cpu2->run([0xF9, 0x01], 10);
    is($cpu2->accumulator, 9, 'TCS: carry=0 -> A=9');
};

# ---------------------------------------------------------------------------
# DAA: Decimal adjust accumulator
# ---------------------------------------------------------------------------

subtest 'DAA' => sub {
    # A=7: no adjustment needed (7 <= 9 and carry=0)
    my ($cpu) = run_prog([0xD7, 0xFB, 0x01], 10);
    is($cpu->accumulator, 7, 'DAA: A=7, no carry -> no change');

    # A=10: > 9 so add 6 -> 16 -> A=0, carry=1
    my ($cpu2) = run_prog([0xDA, 0xFB, 0x01], 10);
    is($cpu2->accumulator, 0, 'DAA: A=10 -> add 6 -> 16 -> A=0');
    is($cpu2->carry,       1, 'DAA: overflow sets carry');

    # A=3 with carry=1 -> add 6 -> 9, carry unchanged (no new overflow)
    my $cpu3 = CodingAdventures::Intel4004Simulator->new();
    $cpu3->{carry} = 1;
    $cpu3->run([0xD3, 0xFB, 0x01], 10);
    is($cpu3->accumulator, 9, 'DAA: A=3 with carry -> add 6 -> 9');
    is($cpu3->carry,       1, 'DAA: carry preserved when no new overflow');
};

# ---------------------------------------------------------------------------
# KBP: Keyboard process
# ---------------------------------------------------------------------------

subtest 'KBP' => sub {
    my %kbp = (0=>0, 1=>1, 2=>2, 4=>3, 8=>4);
    for my $input (keys %kbp) {
        my $expected = $kbp{$input};
        my $cpu = CodingAdventures::Intel4004Simulator->new();
        $cpu->run([0xD0|$input, 0xFC, 0x01], 10);
        is($cpu->accumulator, $expected, "KBP: $input -> $expected");
    }
    # Invalid input -> 15
    my ($cpu) = run_prog([0xD3, 0xFC, 0x01], 10);  # LDM 3, KBP
    is($cpu->accumulator, 15, 'KBP: 3 -> 15 (error)');
};

# ---------------------------------------------------------------------------
# DCL: Designate command line (RAM bank select)
# ---------------------------------------------------------------------------

subtest 'DCL' => sub {
    for my $bank (0..3) {
        my $cpu = CodingAdventures::Intel4004Simulator->new();
        $cpu->run([0xD0|$bank, 0xFD, 0x01], 10);
        is($cpu->{ram_bank}, $bank, "DCL: A=$bank selects bank $bank");
    }
    # A=4-7 get masked to 2 bits (& 0x3) after being checked > 3
    my ($cpu) = run_prog([0xD5, 0xFD, 0x01], 10);  # A=5 & 3 = 1
    is($cpu->{ram_bank}, 1, 'DCL: A=5 -> bank 1 (5&3=1)');
};

# ---------------------------------------------------------------------------
# JUN: Unconditional jump
# ---------------------------------------------------------------------------

subtest 'JUN' => sub {
    # JUN 0x010 — jump to address 16; place HLT there
    # Bytes: 0x40, 0x10 (JUN 0x010), then padding, then HLT at addr 16
    my @prog = (0) x 4096;
    $prog[0] = 0x40;  # JUN high nibble = 0, target high = 0
    $prog[1] = 0x10;  # target low = 0x10
    $prog[0x10] = 0x01;  # HLT at 0x010

    my ($cpu) = run_prog(\@prog, 100);
    is($cpu->pc, 0x011, 'JUN: PC lands after HLT at 0x10');
    is($cpu->halted, 1, 'JUN: execution reaches HLT');
};

# ---------------------------------------------------------------------------
# JCN: Conditional jump
# ---------------------------------------------------------------------------

subtest 'JCN' => sub {
    # JCN with cond=4 (test zero): A=0 so branch taken
    # 0x14 = JCN cond=4
    # Target page relative: addr 0 + 2 = page 0; target byte = 0x05
    # So target = 0x005
    my @prog = (0) x 4096;
    $prog[0] = 0x14;   # JCN cond=4 (test zero)
    $prog[1] = 0x05;   # page-relative target = 5
    $prog[2] = 0xD7;   # LDM 7 (should be skipped if jump taken)
    $prog[3] = 0x01;   # HLT
    $prog[5] = 0x01;   # HLT at target

    my ($cpu, $traces) = run_prog(\@prog, 100);
    is($cpu->accumulator, 0, 'JCN cond=4 taken: LDM 7 was skipped');
    is($cpu->halted, 1, 'JCN halted at target');

    # JCN with cond=4 but A!=0: branch NOT taken
    my @prog2 = (0) x 4096;
    $prog2[0] = 0xD7;  # LDM 7 (A=7, not zero)
    $prog2[1] = 0x14;  # JCN cond=4
    $prog2[2] = 0x06;  # page-relative target = 6 (not reached)
    $prog2[3] = 0xD5;  # LDM 5
    $prog2[4] = 0x01;  # HLT
    $prog2[6] = 0x01;  # HLT at target (should not be reached)

    my ($cpu2) = run_prog(\@prog2, 100);
    is($cpu2->accumulator, 5, 'JCN cond=4 not taken when A!=0');

    # JCN with invert bit (0x8) and test carry (0x2): carry=0, test_carry=false,
    # invert -> condition = true, so jump taken
    my @prog3 = (0) x 4096;
    $prog3[0] = 0x1A;  # JCN cond=0xA (invert+test_carry)
    $prog3[1] = 0x05;  # target = 5
    $prog3[2] = 0xD7;  # LDM 7 (skipped)
    $prog3[3] = 0x01;
    $prog3[5] = 0x01;  # HLT at target

    my ($cpu3) = run_prog(\@prog3, 100);
    is($cpu3->accumulator, 0, 'JCN inverted: carry=0 -> inverted -> taken');
};

# ---------------------------------------------------------------------------
# JMS and BBL: Subroutine call and return
# ---------------------------------------------------------------------------

subtest 'JMS and BBL' => sub {
    # JMS 0x006 from addr 0 -> push return=2, jump to 6
    # At 6: BBL 5 -> pop return, A=5, jump back to 2
    # At 2: HLT
    my @prog = (0) x 4096;
    $prog[0] = 0x50;   # JMS high nibble=0, target=0x006
    $prog[1] = 0x06;
    $prog[2] = 0x01;   # HLT (return point)
    $prog[6] = 0xC5;   # BBL 5

    my ($cpu, $traces) = run_prog(\@prog, 100);
    is($cpu->accumulator, 5, 'BBL loads return value 5 into A');
    is($cpu->halted,      1, 'execution returns and hits HLT');
    ok(scalar(grep { $_->{mnemonic} =~ /JMS/ } @$traces) > 0, 'JMS in traces');
    ok(scalar(grep { $_->{mnemonic} =~ /BBL/ } @$traces) > 0, 'BBL in traces');

    # Nested calls (stack depth = 3)
    my @prog2 = (0) x 4096;
    # addr 0: JMS -> 0x010
    $prog2[0] = 0x50; $prog2[1] = 0x10;
    # addr 2: HLT
    $prog2[2] = 0x01;
    # addr 0x10: JMS -> 0x020
    $prog2[0x10] = 0x50; $prog2[0x11] = 0x20;
    # addr 0x12: BBL 1
    $prog2[0x12] = 0xC1;
    # addr 0x20: BBL 2
    $prog2[0x20] = 0xC2;

    my ($cpu2) = run_prog(\@prog2, 100);
    is($cpu2->halted, 1, 'nested JMS/BBL returns correctly');
    is($cpu2->accumulator, 1, 'outer BBL sets A=1');
};

# ---------------------------------------------------------------------------
# ISZ: Increment and skip if zero
# ---------------------------------------------------------------------------

subtest 'ISZ' => sub {
    # ISZ R0 with R0=0xE (14): increments to 15, not zero, jumps back
    # Use a loop: R0 starts at 0xE (14), loop 2 times until 0
    # ISZ R0, addr -> jump target = page | addr
    # addr 0: ISZ R0, 0x00 (jump to addr 0 if != 0)
    # addr 2: HLT
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{registers}[0] = 0xE;  # start at 14; will wrap to 0 after 2 increments
    $cpu->run([0x70, 0x00, 0x01], 10);   # ISZ R0, target=0; HLT
    # R0: 14->15 (jump to 0), 15->0 (no jump, fall through to HLT)
    is($cpu->get_register(0), 0, 'ISZ: R0 wrapped to 0');
    is($cpu->halted,          1, 'ISZ: fell through to HLT when R0=0');
};

# ---------------------------------------------------------------------------
# FIM: Fetch immediate into register pair
# ---------------------------------------------------------------------------

subtest 'FIM' => sub {
    # FIM P0, 0xAB -> R0=0xA, R1=0xB
    my ($cpu) = run_prog([0x20, 0xAB, 0x01], 10);
    is($cpu->get_register(0), 0xA, 'FIM P0: R0=high nibble');
    is($cpu->get_register(1), 0xB, 'FIM P0: R1=low nibble');

    # FIM P3, 0x37 -> R6=3, R7=7
    my ($cpu2) = run_prog([0x26, 0x37, 0x01], 10);
    is($cpu2->get_register(6), 3, 'FIM P3: R6=3');
    is($cpu2->get_register(7), 7, 'FIM P3: R7=7');
};

# ---------------------------------------------------------------------------
# SRC: Send register control (set RAM address)
# ---------------------------------------------------------------------------

subtest 'SRC' => sub {
    # SRC P0 with P0 = 0x23 -> ram_register = (0x2 % 4) = 2, ram_character = 3
    my $cpu = CodingAdventures::Intel4004Simulator->new();
    $cpu->{registers}[0] = 2;   # R0 high = 2
    $cpu->{registers}[1] = 3;   # R1 low = 3
    $cpu->run([0x21, 0x01], 10);   # SRC P0, HLT
    is($cpu->{ram_register},  2, 'SRC: ram_register set to 2');
    is($cpu->{ram_character}, 3, 'SRC: ram_character set to 3');
};

# ---------------------------------------------------------------------------
# WRM, RDM: Write/read RAM main character
# ---------------------------------------------------------------------------

subtest 'WRM and RDM' => sub {
    # SRC P0 (addr 0,0), WRM writes A=7 to ram[0][0][0], RDM reads it back
    # FIM P0, 0x00 -> R0=0, R1=0
    # SRC P0
    # LDM 7
    # WRM
    # LDM 0  (clear A)
    # RDM    (read back)
    # HLT
    my ($cpu) = run_prog([0x20,0x00, 0x21, 0xD7, 0xE0, 0xD0, 0xE9, 0x01], 10);
    is($cpu->accumulator, 7, 'RDM reads back value written by WRM');
};

# ---------------------------------------------------------------------------
# WRR, RDR: Write/read ROM I/O port
# ---------------------------------------------------------------------------

subtest 'WRR and RDR' => sub {
    # LDM 9, WRR (write 9 to rom_port), LDM 0, RDR (read back), HLT
    my ($cpu) = run_prog([0xD9, 0xE2, 0xD0, 0xEA, 0x01], 10);
    is($cpu->accumulator, 9, 'RDR reads back value written by WRR');
};

# ---------------------------------------------------------------------------
# WR0-WR3, RD0-RD3: RAM status characters
# ---------------------------------------------------------------------------

subtest 'status characters' => sub {
    # FIM P0, 0x00; SRC P0; LDM 5; WR0; LDM 0; RD0; HLT
    my ($cpu) = run_prog([0x20,0x00, 0x21, 0xD5, 0xE4, 0xD0, 0xEC, 0x01], 10);
    is($cpu->accumulator, 5, 'WR0/RD0 round-trip works');

    # WR3/RD3 (0xE7/0xEF)
    my ($cpu2) = run_prog([0x20,0x00, 0x21, 0xD3, 0xE7, 0xD0, 0xEF, 0x01], 10);
    is($cpu2->accumulator, 3, 'WR3/RD3 round-trip works');
};

# ---------------------------------------------------------------------------
# ADM, SBM: RAM arithmetic
# ---------------------------------------------------------------------------

subtest 'ADM and SBM' => sub {
    # Write 4 to ram[0][0][0], then ADM (A=3+4=7 no carry)
    # FIM P0,0x00; SRC P0; LDM 4; WRM; LDM 3; ADM; HLT
    my ($cpu) = run_prog([0x20,0x00, 0x21, 0xD4, 0xE0, 0xD3, 0xEB, 0x01], 10);
    is($cpu->accumulator, 7, 'ADM: 3 + 4 = 7');
    is($cpu->carry,       0, 'ADM: no carry');

    # Write 9 to ram[0][0][0], A=9, SBM: 9 - 9 = 0
    # FIM P0,0x00; SRC P0; LDM 9; WRM; LDM 9; STC; SBM; HLT
    my ($cpu2) = run_prog([0x20,0x00, 0x21, 0xD9, 0xE0, 0xD9, 0xFA, 0xE8, 0x01], 10);
    is($cpu2->accumulator, 0, 'SBM: 9-9=0 with carry=1 (no borrow)');
};

# ---------------------------------------------------------------------------
# WMP: Write accumulator to RAM output port
# ---------------------------------------------------------------------------

subtest 'WMP' => sub {
    # FIM P0,0; SRC P0; LDM 7; WMP; HLT
    my ($cpu) = run_prog([0x20,0x00, 0x21, 0xD7, 0xE1, 0x01], 10);
    is($cpu->{ram_output}[0], 7, 'WMP writes to ram_output for current bank');
};

# ---------------------------------------------------------------------------
# FIN: Fetch indirect from ROM
# ---------------------------------------------------------------------------

subtest 'FIN' => sub {
    # P0 = 0x10 (address 16 on same page), ROM[16] = 0xAB
    # FIM P0,0x10; FIN P1; HLT
    # P1 = ROM[page|P0] = ROM[0x010] = 0xAB -> R2=0xA, R3=0xB
    my @prog = (0) x 4096;
    $prog[0] = 0x20; $prog[1] = 0x10;   # FIM P0, 0x10
    $prog[2] = 0x32;                     # FIN P1
    $prog[3] = 0x01;                     # HLT
    $prog[0x10] = 0xAB;                  # ROM data at addr 16

    my ($cpu) = run_prog(\@prog, 100);
    is($cpu->get_register(2), 0xA, 'FIN P1: R2=high nibble of ROM byte');
    is($cpu->get_register(3), 0xB, 'FIN P1: R3=low nibble of ROM byte');
};

# ---------------------------------------------------------------------------
# JIN: Jump indirect via register pair
# ---------------------------------------------------------------------------

subtest 'JIN' => sub {
    # FIM P0,0x08; JIN P0; (skip 5 bytes); HLT at addr 8
    my @prog = (0) x 4096;
    $prog[0] = 0x20; $prog[1] = 0x08;  # FIM P0, 0x08
    $prog[2] = 0x31;                    # JIN P0 -> PC = page|0x08 = 0x008
    $prog[3] = 0xD7;                    # LDM 7 (should be skipped)
    $prog[4] = 0x01;
    $prog[8] = 0x01;                    # HLT at target

    my ($cpu) = run_prog(\@prog, 100);
    is($cpu->accumulator, 0, 'JIN: LDM 7 was skipped');
    is($cpu->halted,      1, 'JIN: jumped to HLT');
};

# ---------------------------------------------------------------------------
# Reset
# ---------------------------------------------------------------------------

subtest 'reset' => sub {
    my $cpu = CodingAdventures::Intel4004Simulator->new();
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
# Integration: 1 + 2 = 3
# ---------------------------------------------------------------------------

subtest 'integration: 1+2' => sub {
    # LDM 1, XCH R0, LDM 2, ADD R0, HLT
    my ($cpu) = run_prog([0xD1, 0xB0, 0xD2, 0x80, 0x01], 10);
    is($cpu->accumulator, 3, '1 + 2 = 3');
    is($cpu->carry,       0, 'no carry on 1+2');
};

# ---------------------------------------------------------------------------
# Integration: 3 x 4 via ISZ loop
# ---------------------------------------------------------------------------

subtest 'integration: 3x4 via ISZ loop' => sub {
    # Compute 3 * 4 by adding 3 four times.
    # R0 = loop counter (starts at 0xC = 12 = -4 in 4-bit), R1 = accumulator
    # Loop:
    #   ISZ R0, loop_target  <- increment R0; if R0 != 0 jump back to loop
    #   (fall through when R0 == 0 after 4 iterations)
    # After loop: A = R1 = 3*4 = 12
    #
    # Encoding:
    #   addr 0: FIM P0, 0xC0   (R0=12=0xC, R1=0)     [0x20, 0xC0]
    #   addr 2: FIM P1, 0x00   (R2=0, R3=0)           [0x22, 0x00]
    #   addr 4: LDM 3          (A=3)                   [0xD3]
    #   addr 5: ADD R1         (A = A + R1)            [0x81]
    #   addr 6: XCH R1         (R1 = A, A = old R1)   [0xB1]
    #   addr 7: ISZ R0, 0x04   (if R0!=0, jump to 4)  [0x70, 0x04]
    #   addr 9: LD R1          (A = R1 = result)       [0xA1]
    #   addr 10: HLT                                   [0x01]

    my ($cpu) = run_prog([
        0x20, 0xC0,   # FIM P0: R0=12, R1=0
        0xD3,         # LDM 3
        0x81,         # ADD R1 (A = 3 + R1, carry=0)
        0xB1,         # XCH R1 (R1=A, A=old R1)
        0x70, 0x02,   # ISZ R0, target=2 (page=0, target = 0x002)
        0xA1,         # LD R1 (A = result)
        0x01,         # HLT
    ], 100);

    is($cpu->accumulator, 12, '3 x 4 = 12 via ISZ loop');
};

done_testing();
