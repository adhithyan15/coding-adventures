#!/usr/bin/perl
use strict;
use warnings;

# Tests for CodingAdventures::Intel8008Simulator
#
# We test every instruction group and category, plus integration programs.
# Target: 95%+ coverage of the Intel 8008 ISA.
#
# The Intel 8008 (April 1972) was the world's first commercial 8-bit
# microprocessor. This test suite exercises:
#   Group 1: Register operations (MOV, MVI, INR, DCR)
#   Group 2: ALU register (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP)
#   Group 3: ALU immediate (ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI)
#   Group 4: Rotates (RLC, RRC, RAL, RAR)
#   Group 5: Jumps (JMP, JFC/JTC, JFZ/JTZ, JFS/JTS, JFP/JTP)
#   Group 6: Calls (CAL, CFC/CTC, etc.)
#   Group 7: Returns (RET, RFC/RTC, etc.)
#   Group 8: Restart (RST 0–7)
#   Group 9: I/O (IN 0–7, OUT 0–23)
#   Group 10: HLT

use Test2::V0;
use lib '../lib';
use CodingAdventures::Intel8008Simulator;

# ---------------------------------------------------------------------------
# Helper: create a fresh CPU and run a program
# ---------------------------------------------------------------------------
sub run_prog {
    my ($bytes, $max_steps) = @_;
    $max_steps //= 10_000;
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    my $traces = $cpu->run($bytes, $max_steps);
    return ($cpu, $traces);
}

# ---------------------------------------------------------------------------
# Initialization
# ---------------------------------------------------------------------------

subtest 'initialization' => sub {
    my $cpu = CodingAdventures::Intel8008Simulator->new();
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

# ---------------------------------------------------------------------------
# HLT
# ---------------------------------------------------------------------------

subtest 'HLT (0x76)' => sub {
    my ($cpu, $traces) = run_prog([0x76], 10);
    is(scalar @$traces, 1,     'one instruction executed');
    is($traces->[0]{mnemonic}, 'HLT', 'instruction is HLT');
    is($cpu->halted, 1, 'CPU halted after 0x76');

    ok(dies { $cpu->step() }, 'step after halt dies');
};

subtest 'HLT (0xFF)' => sub {
    my ($cpu, $traces) = run_prog([0xFF], 10);
    is($cpu->halted, 1, 'CPU halted after 0xFF');
};

# ---------------------------------------------------------------------------
# MVI — Move Immediate (2 bytes: 00 DDD 110, data)
# ---------------------------------------------------------------------------

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

subtest 'MVI M (memory indirect)' => sub {
    # Set H=0x00, L=0x40, then MVI M, 0xAB → writes 0xAB to address 0x0040
    my ($cpu) = run_prog([
        0x26, 0x00,   # MVI H, 0x00
        0x2E, 0x40,   # MVI L, 0x40
        0x36, 0xAB,   # MVI M, 0xAB
        0x76,         # HLT
    ]);
    is($cpu->{memory}[0x0040], 0xAB, 'MVI M writes to memory at [H:L]');
};

subtest 'MVI flags not affected' => sub {
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{flags} = { carry => 1, zero => 1, sign => 1, parity => 1 };
    $cpu->run([0x3E, 0x00, 0x76]);  # MVI A, 0x00
    # MVI should NOT update flags
    is($cpu->flags->{carry},  1, 'MVI does not clear carry');
    is($cpu->flags->{zero},   1, 'MVI does not update zero flag');
    is($cpu->flags->{sign},   1, 'MVI does not update sign flag');
    is($cpu->flags->{parity}, 1, 'MVI does not update parity flag');
};

# ---------------------------------------------------------------------------
# MOV — Register to Register Transfer (1 byte: 01 DDD SSS)
# ---------------------------------------------------------------------------

subtest 'MOV register to register' => sub {
    # MVI B, 0x55; MOV A, B; HLT → A = 0x55
    my ($cpu) = run_prog([0x06, 0x55, 0x78, 0x76]);
    is($cpu->a, 0x55, 'MOV A, B copies B to A');

    # MVI A, 0x33; MOV B, A; HLT → B = 0x33
    ($cpu) = run_prog([0x3E, 0x33, 0x47, 0x76]);
    is($cpu->b, 0x33, 'MOV B, A copies A to B');

    # MOV H, L after loading both
    ($cpu) = run_prog([
        0x26, 0x11,   # MVI H, 0x11
        0x2E, 0x22,   # MVI L, 0x22
        0x65,         # MOV H, L  (0x65 = 01 100 101)
        0x76,         # HLT
    ]);
    is($cpu->h, 0x22, 'MOV H, L copies L to H');
};

subtest 'MOV with memory (M)' => sub {
    # Set H:L = 0x0050, MVI M, 0xBC, then MOV H, M (0x66) → H = 0xBC
    # Note: MOV A, M (0x7E) conflicts with CAL (unconditional call).
    # MOV H, M (0x66) is available: ddd=4(H), sss=6(M) → group=01, ddd=4, sss=110.
    my ($cpu) = run_prog([
        0x26, 0x00,   # MVI H, 0x00
        0x2E, 0x50,   # MVI L, 0x50
        0x36, 0xBC,   # MVI M, 0xBC   (writes to 0x0050)
        0x66,         # MOV H, M      (H ← mem[0x0050] = 0xBC)
        0x76,
    ]);
    is($cpu->h, 0xBC, 'MOV H, M reads from memory at [H:L]');

    # MOV M, A stores A at [H:L]  (0x77 = 01 110 111 = MOV M, A)
    ($cpu) = run_prog([
        0x3E, 0xDE,   # MVI A, 0xDE
        0x26, 0x00,   # MVI H, 0x00
        0x2E, 0x60,   # MVI L, 0x60
        0x77,         # MOV M, A      (writes to 0x0060)
        0x76,
    ]);
    is($cpu->{memory}[0x0060], 0xDE, 'MOV M, A writes A to memory at [H:L]');
};

# ---------------------------------------------------------------------------
# INR — Increment Register (1 byte: 00 DDD 000)
# ---------------------------------------------------------------------------

subtest 'INR' => sub {
    # INR A — starts at 0, should become 1
    my ($cpu) = run_prog([0x38, 0x76]);  # 0x38 = 00 111 000 = INR A
    is($cpu->a, 1, 'INR A increments A from 0 to 1');

    # INR wraps 0xFF → 0x00
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0xFF;
    $cpu->run([0x38, 0x76]);
    is($cpu->a, 0x00, 'INR A wraps 0xFF to 0x00');
    is($cpu->flags->{zero}, 1, 'INR sets Z when wraps to 0');

    # INR B  (0x00 = 00 000 000)
    ($cpu) = run_prog([0x00, 0x76]);  # INR B (B starts at 0, becomes 1)
    is($cpu->b, 1, 'INR B increments B');

    # INR does NOT update carry
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{flags}{carry} = 1;
    $cpu->run([0x00, 0x76]);  # INR B
    is($cpu->flags->{carry}, 1, 'INR preserves carry flag');
};

subtest 'INR parity and sign flags' => sub {
    # MVI A, 0x01 then INR A → A=0x02; 0x02=00000010 has 1 one → P=0 (odd)
    my ($cpu) = run_prog([0x3E, 0x01, 0x38, 0x76]);
    is($cpu->a, 0x02, 'INR A: 1 → 2');
    is($cpu->flags->{parity}, 0, 'INR: parity odd for 0x02');
    is($cpu->flags->{sign},   0, 'INR: sign clear for 0x02');

    # MVI A, 0x02 then INR A → A=0x03; 0x03=00000011 has 2 ones → P=1 (even)
    ($cpu) = run_prog([0x3E, 0x02, 0x38, 0x76]);
    is($cpu->flags->{parity}, 1, 'INR: parity even for 0x03 (2 ones)');

    # MVI A, 0x7F then INR A → A=0x80; sign bit set
    ($cpu) = run_prog([0x3E, 0x7F, 0x38, 0x76]);
    is($cpu->flags->{sign}, 1, 'INR: sign set when bit 7 becomes 1');
};

# ---------------------------------------------------------------------------
# DCR — Decrement Register (1 byte: 00 DDD 001)
# ---------------------------------------------------------------------------

subtest 'DCR' => sub {
    # MVI A, 5; DCR A → A=4
    my ($cpu) = run_prog([0x3E, 0x05, 0x39, 0x76]);  # 0x39 = 00 111 001 = DCR A
    is($cpu->a, 4, 'DCR A decrements A from 5 to 4');

    # DCR wraps 0x00 → 0xFF
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->run([0x39, 0x76]);  # A starts at 0
    is($cpu->a, 0xFF, 'DCR A wraps 0x00 to 0xFF');
    is($cpu->flags->{sign}, 1, 'DCR: sign set after wrap to 0xFF');

    # DCR does NOT update carry
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{flags}{carry} = 1;
    $cpu->run([0x39, 0x76]);
    is($cpu->flags->{carry}, 1, 'DCR preserves carry flag');
};

# ---------------------------------------------------------------------------
# ADD — Add register to A (1 byte: 10 000 SSS)
# ---------------------------------------------------------------------------

subtest 'ADD register' => sub {
    # MVI B, 1; MVI A, 2; ADD B; HLT → A = 3
    my ($cpu) = run_prog([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
    is($cpu->a, 3, 'ADD B: A = 2 + 1 = 3');
    is($cpu->flags->{carry}, 0, 'ADD: no carry for 2+1');
    is($cpu->flags->{zero},  0, 'ADD: zero clear for 3');

    # Overflow: MVI A, 0xFF; ADD A (A + A = 0x1FE → A=0xFE, CY=1)
    # But first let's do: MVI B, 1; MVI A, 0xFF; ADD B → 0x100 → A=0, CY=1, Z=1
    ($cpu) = run_prog([0x06, 0x01, 0x3E, 0xFF, 0x80, 0x76]);
    is($cpu->a,              0,  'ADD overflow: A wraps to 0');
    is($cpu->flags->{carry}, 1,  'ADD overflow: carry set');
    is($cpu->flags->{zero},  1,  'ADD overflow: zero set');

    # Parity: 0x01 + 0x02 = 0x03 = 00000011 (2 ones → even parity)
    ($cpu) = run_prog([0x06, 0x02, 0x3E, 0x01, 0x80, 0x76]);
    is($cpu->flags->{parity}, 1, 'ADD: parity=1 (even) for 0x03');
};

# ---------------------------------------------------------------------------
# ADC — Add with carry (10 001 SSS)
# ---------------------------------------------------------------------------

subtest 'ADC register' => sub {
    # Set carry=1, MVI B, 1, MVI A, 2; ADC B → A = 2+1+1=4
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{flags}{carry} = 1;
    $cpu->run([0x06, 0x01, 0x3E, 0x02, 0x88, 0x76]);  # 0x88 = 10 001 000 = ADC B
    is($cpu->a, 4, 'ADC B with carry=1: 2+1+1=4');

    # No carry
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->run([0x06, 0x01, 0x3E, 0x02, 0x88, 0x76]);
    is($cpu->a, 3, 'ADC B with carry=0: 2+1+0=3');
};

# ---------------------------------------------------------------------------
# SUB — Subtract register from A (10 010 SSS)
# ---------------------------------------------------------------------------

subtest 'SUB register' => sub {
    # MVI A, 5; MVI B, 3; SUB B → A = 2, CY=0 (no borrow)
    my ($cpu) = run_prog([0x3E, 0x05, 0x06, 0x03, 0x90, 0x76]);  # 0x90 = 10 010 000
    is($cpu->a,              2, 'SUB B: 5-3=2');
    is($cpu->flags->{carry}, 0, 'SUB: CY=0 when no borrow');
    is($cpu->flags->{zero},  0, 'SUB: Z=0 for result 2');

    # MVI A, 3; MVI B, 5; SUB B → A = -2 = 0xFE, CY=1 (borrow)
    ($cpu) = run_prog([0x3E, 0x03, 0x06, 0x05, 0x90, 0x76]);
    is($cpu->a,              0xFE, 'SUB B: 3-5=0xFE (borrow)');
    is($cpu->flags->{carry}, 1,    'SUB: CY=1 when borrow occurs');
    is($cpu->flags->{sign},  1,    'SUB: S=1 for negative result');

    # A - A = 0, Z=1
    ($cpu) = run_prog([0x3E, 0x05, 0x97, 0x76]);  # 0x97 = 10 010 111 = SUB A
    is($cpu->a,             0, 'SUB A: A-A=0');
    is($cpu->flags->{zero}, 1, 'SUB A: zero flag set');
};

# ---------------------------------------------------------------------------
# SBB — Subtract with borrow (10 011 SSS)
# ---------------------------------------------------------------------------

subtest 'SBB register' => sub {
    # A=5, B=2, CY=1: SBB B → 5-2-1=2
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{flags}{carry} = 1;
    $cpu->run([0x3E, 0x05, 0x06, 0x02, 0x98, 0x76]);  # 0x98 = SBB B
    is($cpu->a, 2, 'SBB B with carry=1: 5-2-1=2');

    # No carry
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->run([0x3E, 0x05, 0x06, 0x02, 0x98, 0x76]);
    is($cpu->a, 3, 'SBB B with carry=0: 5-2-0=3');
};

# ---------------------------------------------------------------------------
# ANA — AND (10 100 SSS) — always clears carry
# ---------------------------------------------------------------------------

subtest 'ANA register' => sub {
    # A=0xFF, B=0x0F; ANA B → A=0x0F, CY=0
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{flags}{carry} = 1;  # Verify ANA clears carry
    $cpu->run([0x3E, 0xFF, 0x06, 0x0F, 0xA0, 0x76]);  # 0xA0 = 10 100 000 = ANA B
    is($cpu->a,              0x0F, 'ANA B: 0xFF & 0x0F = 0x0F');
    is($cpu->flags->{carry}, 0,    'ANA clears carry');

    # ANA A — AND with itself (useful to zero carry, set flags based on A)
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{flags}{carry} = 1;
    $cpu->run([0x3E, 0xA7, 0xA7, 0x76]);  # 0xA7 = ANA A
    is($cpu->a,              0xA7, 'ANA A: A unchanged');
    is($cpu->flags->{carry}, 0,    'ANA A clears carry');
};

# ---------------------------------------------------------------------------
# XRA — XOR (10 101 SSS) — always clears carry
# ---------------------------------------------------------------------------

subtest 'XRA register' => sub {
    # A=0xFF, B=0x0F; XRA B → A=0xF0, CY=0
    my ($cpu) = run_prog([0x3E, 0xFF, 0x06, 0x0F, 0xA8, 0x76]);  # 0xA8 = XRA B
    is($cpu->a,              0xF0, 'XRA B: 0xFF ^ 0x0F = 0xF0');
    is($cpu->flags->{carry}, 0,    'XRA clears carry');

    # XRA A — XOR with itself = 0 (clears accumulator, clears carry)
    ($cpu) = run_prog([0x3E, 0x55, 0xAF, 0x76]);  # 0xAF = XRA A
    is($cpu->a,             0, 'XRA A: A ^ A = 0');
    is($cpu->flags->{zero}, 1, 'XRA A: zero flag set');
};

# ---------------------------------------------------------------------------
# ORA — OR (10 110 SSS) — always clears carry
# ---------------------------------------------------------------------------

subtest 'ORA register' => sub {
    # A=0x0F, B=0xF0; ORA B → A=0xFF
    my ($cpu) = run_prog([0x3E, 0x0F, 0x06, 0xF0, 0xB0, 0x76]);  # 0xB0 = ORA B
    is($cpu->a,              0xFF, 'ORA B: 0x0F | 0xF0 = 0xFF');
    is($cpu->flags->{carry}, 0,    'ORA clears carry');
};

# ---------------------------------------------------------------------------
# CMP — Compare (10 111 SSS) — sets flags, A unchanged
# ---------------------------------------------------------------------------

subtest 'CMP register' => sub {
    # A=5, B=5; CMP B → Z=1, A unchanged
    my ($cpu) = run_prog([0x3E, 0x05, 0x06, 0x05, 0xB8, 0x76]);  # 0xB8 = CMP B
    is($cpu->a,             5, 'CMP: A unchanged');
    is($cpu->flags->{zero}, 1, 'CMP equal: Z=1');

    # A=5, B=3; CMP B → Z=0, CY=0 (no borrow)
    ($cpu) = run_prog([0x3E, 0x05, 0x06, 0x03, 0xB8, 0x76]);
    is($cpu->flags->{zero},  0, 'CMP A>B: Z=0');
    is($cpu->flags->{carry}, 0, 'CMP A>B: no borrow');

    # CMP A with itself → always Z=1
    ($cpu) = run_prog([0x3E, 0x42, 0xBF, 0x76]);  # 0xBF = CMP A
    is($cpu->flags->{zero}, 1, 'CMP A,A: always zero');
};

# ---------------------------------------------------------------------------
# ALU immediate instructions (11 OOO 100, data)
# ---------------------------------------------------------------------------

subtest 'ADI (add immediate)' => sub {
    # MVI A, 0x02; ADI 0x05 → A = 7  (0xC4 = 11 000 100)
    my ($cpu) = run_prog([0x3E, 0x02, 0xC4, 0x05, 0x76]);
    is($cpu->a, 7, 'ADI: 2 + 5 = 7');

    # ADI overflow
    ($cpu) = run_prog([0x3E, 0xFF, 0xC4, 0x01, 0x76]);
    is($cpu->a,              0,  'ADI overflow: 0xFF+1=0');
    is($cpu->flags->{carry}, 1,  'ADI overflow: carry set');
};

subtest 'SUI (subtract immediate)' => sub {
    # MVI A, 0x0A; SUI 0x03 → A = 7  (0xD4 = 11 010 100)
    my ($cpu) = run_prog([0x3E, 0x0A, 0xD4, 0x03, 0x76]);
    is($cpu->a, 7, 'SUI: 10 - 3 = 7');
};

subtest 'ANI (AND immediate)' => sub {
    # MVI A, 0xFF; ANI 0x0F → A = 0x0F  (0xE4 = 11 100 100)
    my ($cpu) = run_prog([0x3E, 0xFF, 0xE4, 0x0F, 0x76]);
    is($cpu->a, 0x0F, 'ANI: 0xFF & 0x0F = 0x0F');
};

subtest 'XRI (XOR immediate)' => sub {
    # MVI A, 0xFF; XRI 0xFF → A = 0x00  (0xEC = 11 101 100)
    my ($cpu) = run_prog([0x3E, 0xFF, 0xEC, 0xFF, 0x76]);
    is($cpu->a,             0, 'XRI: 0xFF ^ 0xFF = 0');
    is($cpu->flags->{zero}, 1, 'XRI: zero flag set');
};

subtest 'ORI (OR immediate)' => sub {
    # MVI A, 0x0F; ORI 0xF0 → A = 0xFF  (0xF4 = 11 110 100)
    my ($cpu) = run_prog([0x3E, 0x0F, 0xF4, 0xF0, 0x76]);
    is($cpu->a, 0xFF, 'ORI: 0x0F | 0xF0 = 0xFF');
};

subtest 'CPI (compare immediate)' => sub {
    # MVI A, 0x0A; CPI 0x0A → Z=1, A unchanged  (0xFC = 11 111 100)
    my ($cpu) = run_prog([0x3E, 0x0A, 0xFC, 0x0A, 0x76]);
    is($cpu->a,             0x0A, 'CPI: A unchanged');
    is($cpu->flags->{zero}, 1,    'CPI equal: Z=1');
};

# ---------------------------------------------------------------------------
# Rotate instructions
# ---------------------------------------------------------------------------

subtest 'RLC (0x02) — rotate left circular' => sub {
    # A = 0x80 (10000000): RLC → A = 0x01 (00000001), CY=1
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x80;
    $cpu->run([0x02, 0x76]);
    is($cpu->a,              0x01, 'RLC: 0x80 → 0x01 (bit 7 wraps to bit 0)');
    is($cpu->flags->{carry}, 1,    'RLC: CY=1 (old bit 7)');

    # A = 0x01: RLC → A = 0x02, CY=0
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x01;
    $cpu->run([0x02, 0x76]);
    is($cpu->a,              0x02, 'RLC: 0x01 → 0x02');
    is($cpu->flags->{carry}, 0,    'RLC: CY=0 (old bit 7 was 0)');
};

subtest 'RRC (0x0A) — rotate right circular' => sub {
    # A = 0x01 (00000001): RRC → A = 0x80 (10000000), CY=1
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x01;
    $cpu->run([0x0A, 0x76]);
    is($cpu->a,              0x80, 'RRC: 0x01 → 0x80 (bit 0 wraps to bit 7)');
    is($cpu->flags->{carry}, 1,    'RRC: CY=1 (old bit 0)');

    # A = 0x02: RRC → A = 0x01, CY=0
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x02;
    $cpu->run([0x0A, 0x76]);
    is($cpu->a,              0x01, 'RRC: 0x02 → 0x01');
    is($cpu->flags->{carry}, 0,    'RRC: CY=0');
};

subtest 'RAL (0x12) — rotate left through carry' => sub {
    # A=0x80, CY=0: RAL → new_A = (0x80 << 1)|0 = 0x00, new_CY = 1
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x80;
    $cpu->{flags}{carry} = 0;
    $cpu->run([0x12, 0x76]);
    is($cpu->a,              0x00, 'RAL: 0x80 with CY=0 → 0x00');
    is($cpu->flags->{carry}, 1,    'RAL: new CY = old bit 7 = 1');

    # A=0x40, CY=1: RAL → new_A = (0x40 << 1)|1 = 0x81, new_CY = 0
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x40;
    $cpu->{flags}{carry} = 1;
    $cpu->run([0x12, 0x76]);
    is($cpu->a,              0x81, 'RAL: 0x40 with CY=1 → 0x81');
    is($cpu->flags->{carry}, 0,    'RAL: new CY = old bit 7 = 0');
};

subtest 'RAR (0x1A) — rotate right through carry' => sub {
    # A=0x01, CY=0: RAR → new_A = (0 << 7)|(0x01 >> 1) = 0x00, new_CY = 1
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x01;
    $cpu->{flags}{carry} = 0;
    $cpu->run([0x1A, 0x76]);
    is($cpu->a,              0x00, 'RAR: 0x01 with CY=0 → 0x00');
    is($cpu->flags->{carry}, 1,    'RAR: new CY = old bit 0 = 1');

    # A=0x02, CY=1: RAR → new_A = (1 << 7)|(0x02 >> 1) = 0x81, new_CY = 0
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[7] = 0x02;
    $cpu->{flags}{carry} = 1;
    $cpu->run([0x1A, 0x76]);
    is($cpu->a,              0x81, 'RAR: 0x02 with CY=1 → 0x81');
    is($cpu->flags->{carry}, 0,    'RAR: new CY = old bit 0 = 0');
};

# ---------------------------------------------------------------------------
# JMP — Unconditional Jump (0x7C, lo, hi)
# ---------------------------------------------------------------------------

subtest 'JMP unconditional' => sub {
    # JMP to 0x0006, then HLT at 0x0006, MVI A, 0x42 at 0x0003 (should be skipped)
    #   0x00: 0x7C 0x06 0x00  (JMP 0x0006)
    #   0x03: 0x3E 0x42        (MVI A, 0x42) — skipped
    #   0x06: 0x76             (HLT)
    my @prog = (0) x 16;
    $prog[0] = 0x7C; $prog[1] = 0x06; $prog[2] = 0x00;  # JMP 0x0006
    $prog[3] = 0x3E; $prog[4] = 0x42;                   # MVI A, 0x42 (skipped)
    $prog[6] = 0x76;                                      # HLT
    my ($cpu) = run_prog(\@prog);
    is($cpu->a,    0,    'JMP skips over MVI A, 0x42');
    is($cpu->pc,   7,    'PC is at 7 after HLT (past the HLT instruction)');
    is($cpu->halted, 1,  'CPU halted');
};

# ---------------------------------------------------------------------------
# Conditional Jumps
# ---------------------------------------------------------------------------

subtest 'JFC / JTC (carry jumps)' => sub {
    # JFC: jump if carry=0. Start with CY=0, jump over MVI A, 0xFF.
    #   0x00: 0x40 0x06 0x00  (JFC 0x0006) — should jump since CY=0
    #   0x03: 0x3E 0xFF        (MVI A, 0xFF) — skipped
    #   0x06: 0x76             (HLT)
    my @prog = (0) x 16;
    $prog[0] = 0x40; $prog[1] = 0x06; $prog[2] = 0x00;
    $prog[3] = 0x3E; $prog[4] = 0xFF;
    $prog[6] = 0x76;
    my ($cpu) = run_prog(\@prog);
    is($cpu->a, 0, 'JFC jumps when CY=0 (skips MVI)');

    # JTC: jump if carry=1. Start with CY=0, should NOT jump.
    @prog = (0) x 16;
    $prog[0] = 0x44; $prog[1] = 0x06; $prog[2] = 0x00;  # JTC 0x0006
    $prog[3] = 0x3E; $prog[4] = 0xFF;                    # MVI A, 0xFF (NOT skipped)
    $prog[5] = 0x76;                                      # HLT
    ($cpu) = run_prog(\@prog);
    is($cpu->a, 0xFF, 'JTC does not jump when CY=0 (falls through to MVI)');
};

subtest 'JFZ / JTZ (zero jumps)' => sub {
    # JTZ: jump if Z=1. Load A=0, then subtract 0 to set Z.
    # ADD 0 and check: instead, use SUB A to set Z=1.
    # MVI A, 0x05; SUB A → A=0, Z=1; JTZ target → should jump
    #   0x00: 0x3E 0x05        MVI A, 5
    #   0x02: 0x97             SUB A (A-A=0, Z=1)
    #   0x03: 0x4C 0x08 0x00   JTZ 0x0008
    #   0x06: 0x3E 0xFF        MVI A, 0xFF (skipped)
    #   0x08: 0x76             HLT
    my @prog = (0) x 16;
    $prog[0] = 0x3E; $prog[1] = 0x05;
    $prog[2] = 0x97;
    $prog[3] = 0x4C; $prog[4] = 0x08; $prog[5] = 0x00;
    $prog[6] = 0x3E; $prog[7] = 0xFF;
    $prog[8] = 0x76;
    my ($cpu) = run_prog(\@prog);
    is($cpu->a, 0, 'JTZ jumps when Z=1 (skips MVI A, 0xFF)');
};

# ---------------------------------------------------------------------------
# CAL / RET — Subroutine call and return
# ---------------------------------------------------------------------------

subtest 'CAL and RET' => sub {
    # Program:
    #   0x00: 0x7E 0x06 0x00   CAL 0x0006  (call subroutine)
    #   0x03: 0x3E 0x42        MVI A, 0x42  (after return)
    #   0x05: 0x76             HLT
    #   0x06: 0x3E 0x10        MVI A, 0x10  (subroutine body)
    #   0x08: 0x3F             RET
    my @prog = (0) x 16;
    $prog[0] = 0x7E; $prog[1] = 0x06; $prog[2] = 0x00;  # CAL 0x0006
    $prog[3] = 0x3E; $prog[4] = 0x42;                   # MVI A, 0x42
    $prog[5] = 0x76;                                     # HLT
    $prog[6] = 0x3E; $prog[7] = 0x10;                   # MVI A, 0x10 (sub body)
    $prog[8] = 0x3F;                                     # RET
    my ($cpu) = run_prog(\@prog);
    # After RET, the subroutine returns to 0x03, then MVI A, 0x42, then HLT
    is($cpu->a,    0x42, 'CAL/RET: A=0x42 after call-return sequence');
    is($cpu->halted, 1,  'CPU halted after sequence');
};

# ---------------------------------------------------------------------------
# RST — Restart (1-byte call to fixed address)
# ---------------------------------------------------------------------------

subtest 'RST instructions' => sub {
    # RST 1: 0x0D = 00 001 101 — calls to address 0x0008
    # Place HLT at 0x0008, check that PC ends up there
    my @prog = (0) x 32;
    $prog[0]  = 0x0D;   # RST 1 → calls 0x0008
    $prog[8]  = 0x76;   # HLT at 0x0008
    my ($cpu) = run_prog(\@prog);
    is($cpu->halted, 1, 'RST 1 jumps to 0x0008 and halts');

    # RST 0: 0x05 = 00 000 101 — calls to address 0x0000
    # Careful: RST 0 at address 0 would loop; put something at a different address
    # and use RST 7 → 0x0038
    @prog = (0) x 64;
    $prog[0]    = 0x3D;  # RST 7 → calls 0x0038
    $prog[0x38] = 0x76;  # HLT at 0x0038
    ($cpu) = run_prog(\@prog);
    is($cpu->halted, 1, 'RST 7 jumps to 0x0038 and halts');
};

# ---------------------------------------------------------------------------
# IN / OUT — I/O port instructions
# ---------------------------------------------------------------------------

subtest 'IN (read input port)' => sub {
    # IN 0: 0x41 = 01 000 001
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->set_input_port(0, 0xAB);
    $cpu->run([0x41, 0x76]);
    is($cpu->a, 0xAB, 'IN 0 reads port 0 into A');

    # IN 3: 0x59 = 01 011 001
    $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->set_input_port(3, 0xCD);
    $cpu->run([0x59, 0x76]);
    is($cpu->a, 0xCD, 'IN 3 reads port 3 into A');
};

subtest 'OUT (write output port)' => sub {
    # OUT 4: 0x22 = 00 100 010 — unambiguous OUT (ddd=4, sss=010)
    # MVI A, 0x55; OUT 4 (0x22); HLT → output_port[4] = 0x55
    my ($cpu) = run_prog([0x3E, 0x55, 0x22, 0x76]);
    is($cpu->get_output_port(4), 0x55, 'OUT 4 (0x22): writes A to port 4');

    # OUT 7: 0x3A = 00 111 010 — unambiguous OUT (ddd=7, sss=010)
    ($cpu) = run_prog([0x3E, 0xAA, 0x3A, 0x76]);
    is($cpu->get_output_port(7), 0xAA, 'OUT 7 (0x3A): writes A to port 7');

    # Note: OUT 0-3 (0x02, 0x0A, 0x12, 0x1A) overlap with RLC/RRC/RAL/RAR.
    # The real 8008 hardware uses I/O control bus signals to distinguish them.
    # A software simulator treats those opcodes as rotate instructions.
};

# ---------------------------------------------------------------------------
# Stack depth tracking
# ---------------------------------------------------------------------------

subtest 'stack depth' => sub {
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    is($cpu->stack_depth, 0, 'stack depth starts at 0');

    # After one call, stack depth = 1
    $cpu->{memory}[0] = 0x7E; $cpu->{memory}[1] = 0x06; $cpu->{memory}[2] = 0x00;
    $cpu->{memory}[3] = 0x76;  # HLT (return address)
    $cpu->{memory}[6] = 0x3F;  # RET at the subroutine
    $cpu->{memory}[7] = 0x76;  # HLT after return

    $cpu->step();  # CAL 0x0006
    is($cpu->stack_depth, 1, 'stack depth 1 after CAL');
    $cpu->step();  # RET
    is($cpu->stack_depth, 0, 'stack depth 0 after RET');
};

# ---------------------------------------------------------------------------
# HL address computation
# ---------------------------------------------------------------------------

subtest 'hl_address' => sub {
    my $cpu = CodingAdventures::Intel8008Simulator->new();
    $cpu->{regs}[4] = 0x10;  # H = 0x10 — only low 6 bits used: 0x10 & 0x3F = 0x10
    $cpu->{regs}[5] = 0x20;  # L = 0x20
    is($cpu->hl_address, (0x10 << 8) | 0x20, 'hl_address = (H & 0x3F) << 8 | L');

    # Test that H high 2 bits are ignored
    $cpu->{regs}[4] = 0xFF;  # H = 0xFF → 0xFF & 0x3F = 0x3F
    $cpu->{regs}[5] = 0x00;
    is($cpu->hl_address, 0x3F00, 'hl_address masks H to 6 bits');
};

# ---------------------------------------------------------------------------
# Integration: compute 1+2=3
# ---------------------------------------------------------------------------

subtest 'integration: 1+2=3' => sub {
    # MVI B, 1; MVI A, 2; ADD B; HLT
    my ($cpu) = run_prog([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
    is($cpu->a, 3, '1 + 2 = 3');
    is($cpu->flags->{zero},  0, 'Z=0 (result is not zero)');
    is($cpu->flags->{sign},  0, 'S=0 (positive result)');
    is($cpu->flags->{carry}, 0, 'CY=0 (no overflow)');
    # 3 = 0b00000011 has 2 ones → even parity → P=1
    is($cpu->flags->{parity}, 1, 'P=1 (even parity: 2 ones in 0x03)');
};

# ---------------------------------------------------------------------------
# Integration: memory operations
# ---------------------------------------------------------------------------

subtest 'integration: memory store/load' => sub {
    # Store 0x42 at memory address 0x0010 via MOV M, A.
    # Then load it back into A via a two-step: MOV L, M (read memory into L),
    # then ADD with A=0 (using ANA A then ORA L to copy L→A without conflict).
    # Actually simpler: use INR + ALU to verify the value was stored, or
    # use MOV L, M (0x6E) then MOV A, L (0x7D) — both are valid!
    #
    # Valid MOV to A (ddd=7) without conflicts:
    #   0x78=MOV A,B  0x7A=MOV A,D  0x7B=MOV A,E  0x7D=MOV A,L  0x7F=MOV A,A
    # MOV A,L = 0x7D = 01 111 101 ✓ (not a jump/call)
    #
    # Plan: store 0x42 at 0x0010, read it back into L (via MOV L,M = 0x6E),
    # then load A = L (via MOV A, L = 0x7D).
    my ($cpu) = run_prog([
        0x26, 0x00,   # MVI H, 0x00
        0x2E, 0x10,   # MVI L, 0x10      (L=0x10; H:L=0x0010)
        0x3E, 0x42,   # MVI A, 0x42
        0x77,         # MOV M, A          (store 0x42 at 0x0010)
        0x47,         # MOV B, A          (B = 0x42)
        0x3E, 0x00,   # MVI A, 0x00       (clear A)
        0x26, 0x00,   # MVI H, 0x00       (restore H=0)
        0x6E,         # MOV L, M          (L ← mem[H:L=0x0010] = 0x42)
        0x7D,         # MOV A, L          (A ← L = 0x42)
        0x76,
    ]);
    is($cpu->a,              0x42, 'A loaded back from memory via L');
    is($cpu->b,              0x42, 'B also has 0x42');
    is($cpu->{memory}[0x10], 0x42, 'memory[0x10] = 0x42');
};

# ---------------------------------------------------------------------------
# Integration: counting loop (DCR + conditional jump)
# ---------------------------------------------------------------------------

subtest 'integration: countdown loop' => sub {
    # Count B from 3 down to 0:
    #   0x00: MVI B, 3      (0x06, 0x03)
    #   0x02: DCR B         (0x01)
    #   0x03: JFZ 0x0002    (0x48, 0x02, 0x00)  — jump if Z=0 (not zero yet)
    #   0x06: HLT           (0x76)
    my ($cpu) = run_prog([
        0x06, 0x03,         # MVI B, 3
        0x01,               # DCR B
        0x48, 0x02, 0x00,   # JFZ 0x0002 (loop while B != 0)
        0x76,               # HLT
    ]);
    is($cpu->b,             0, 'loop counted B down to 0');
    is($cpu->flags->{zero}, 1, 'Z=1 when B reached 0');
};

# ---------------------------------------------------------------------------
# Reset
# ---------------------------------------------------------------------------

subtest 'reset' => sub {
    my ($cpu) = run_prog([0x3E, 0x42, 0x76]);
    is($cpu->a, 0x42, 'A is 0x42 after program');
    $cpu->reset();
    is($cpu->a,    0, 'A cleared after reset');
    is($cpu->pc,   0, 'PC cleared after reset');
    is($cpu->halted, 0, 'not halted after reset');
};

done_testing();
