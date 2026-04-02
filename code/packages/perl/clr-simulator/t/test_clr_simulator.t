use strict;
use warnings;
use Test2::V0;
use lib 'lib';

use CodingAdventures::ClrSimulator;

# ============================================================================
# Helpers
# ============================================================================

sub run_program {
    my ($parts, %opts) = @_;
    my $code = CodingAdventures::ClrSimulator::assemble($parts);
    my $sim  = CodingAdventures::ClrSimulator->new();
    $sim->load($code, %opts);
    $sim->run();
    return $sim;
}

sub clr { 'CodingAdventures::ClrSimulator' }

# ============================================================================
# Module basics
# ============================================================================

subtest 'version' => sub {
    is($CodingAdventures::ClrSimulator::VERSION, '0.01', 'version is 0.01');
};

subtest 'opcode constants' => sub {
    is(CodingAdventures::ClrSimulator::NOP,       0x00, 'NOP');
    is(CodingAdventures::ClrSimulator::LDNULL,    0x01, 'LDNULL');
    is(CodingAdventures::ClrSimulator::RET,       0x2A, 'RET');
    is(CodingAdventures::ClrSimulator::ADD,       0x58, 'ADD');
    is(CodingAdventures::ClrSimulator::SUB,       0x59, 'SUB');
    is(CodingAdventures::ClrSimulator::MUL,       0x5A, 'MUL');
    is(CodingAdventures::ClrSimulator::DIV,       0x5B, 'DIV');
    is(CodingAdventures::ClrSimulator::PREFIX_FE, 0xFE, 'PREFIX_FE');
    is(CodingAdventures::ClrSimulator::CEQ_BYTE,  0x01, 'CEQ_BYTE');
    is(CodingAdventures::ClrSimulator::CGT_BYTE,  0x02, 'CGT_BYTE');
    is(CodingAdventures::ClrSimulator::CLT_BYTE,  0x04, 'CLT_BYTE');
};

# ============================================================================
# new() and load()
# ============================================================================

subtest 'new and load' => sub {
    my $sim = CodingAdventures::ClrSimulator->new();
    is(scalar @{$sim->{stack}},    0, 'stack empty');
    is($sim->{pc},                 0, 'pc=0');
    is($sim->{halted},             0, 'not halted');

    $sim->load([CodingAdventures::ClrSimulator::RET]);
    is(scalar @{$sim->{locals}}, 16, 'default 16 locals');

    $sim->load([CodingAdventures::ClrSimulator::RET], num_locals => 8);
    is(scalar @{$sim->{locals}}, 8, '8 locals when specified');
};

# ============================================================================
# NOP
# ============================================================================

subtest 'nop' => sub {
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::NOP,
                CodingAdventures::ClrSimulator::RET]);
    my $trace = $sim->step();
    is($trace->{opcode}, 'nop', 'opcode is nop');
    is($sim->{pc},       1,     'pc advanced');
    is(scalar @{$sim->{stack}}, 0, 'stack unchanged');
};

# ============================================================================
# ldnull
# ============================================================================

subtest 'ldnull' => sub {
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::LDNULL,
                CodingAdventures::ClrSimulator::RET]);
    my $trace = $sim->step();
    is($trace->{opcode}, 'ldnull', 'opcode is ldnull');
    is(scalar @{$sim->{stack}}, 1, 'one item on stack');
    is($sim->{stack}[-1], undef, 'top is null/undef');
};

# ============================================================================
# ldc.i4 short forms (0-8)
# ============================================================================

subtest 'ldc.i4 short forms' => sub {
    for my $n (0..8) {
        my $sim = CodingAdventures::ClrSimulator->new();
        $sim->load([CodingAdventures::ClrSimulator::LDC_I4_0 + $n,
                    CodingAdventures::ClrSimulator::RET]);
        $sim->step();
        is($sim->{stack}[-1], $n, "ldc.i4.$n pushes $n");
    }
};

# ============================================================================
# ldc.i4.s
# ============================================================================

subtest 'ldc.i4.s' => sub {
    my $sim = run_program([
        [CodingAdventures::ClrSimulator::LDC_I4_S, 42],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 42, 'positive value');

    $sim = run_program([
        [CodingAdventures::ClrSimulator::LDC_I4_S, 246],  # -10 signed
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], -10, 'negative value -10');

    $sim = run_program([
        [CodingAdventures::ClrSimulator::LDC_I4_S, 255],  # -1 signed
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], -1, '-1 from 0xFF');
};

# ============================================================================
# ldc.i4 (32-bit)
# ============================================================================

subtest 'ldc.i4 32-bit' => sub {
    my $bytes = CodingAdventures::ClrSimulator::encode_ldc_i4(100000);
    push @$bytes, CodingAdventures::ClrSimulator::RET;
    my $sim = run_program([$bytes]);
    is($sim->{stack}[-1], 100000, 'large positive value');

    $bytes = CodingAdventures::ClrSimulator::encode_ldc_i4(-100000);
    push @$bytes, CodingAdventures::ClrSimulator::RET;
    $sim = run_program([$bytes]);
    is($sim->{stack}[-1], -100000, 'large negative value');
};

# ============================================================================
# encode_ldc_i4
# ============================================================================

subtest 'encode_ldc_i4' => sub {
    my $b = CodingAdventures::ClrSimulator::encode_ldc_i4(0);
    is(scalar @$b, 1, '0 → 1 byte');
    is($b->[0], CodingAdventures::ClrSimulator::LDC_I4_0, '0 → LDC_I4_0');

    $b = CodingAdventures::ClrSimulator::encode_ldc_i4(8);
    is(scalar @$b, 1, '8 → 1 byte');
    is($b->[0], CodingAdventures::ClrSimulator::LDC_I4_8, '8 → LDC_I4_8');

    $b = CodingAdventures::ClrSimulator::encode_ldc_i4(9);
    is(scalar @$b, 2, '9 → 2 bytes (ldc.i4.s)');
    is($b->[0], CodingAdventures::ClrSimulator::LDC_I4_S, '9 → LDC_I4_S');

    $b = CodingAdventures::ClrSimulator::encode_ldc_i4(-1);
    is(scalar @$b, 2, '-1 → 2 bytes');

    $b = CodingAdventures::ClrSimulator::encode_ldc_i4(50000);
    is(scalar @$b, 5, '50000 → 5 bytes');
    is($b->[0], CodingAdventures::ClrSimulator::LDC_I4, '50000 → LDC_I4');
};

# ============================================================================
# stloc / ldloc short forms
# ============================================================================

subtest 'stloc and ldloc short forms' => sub {
    for my $slot (0..3) {
        my $sim = run_program([
            CodingAdventures::ClrSimulator::encode_ldc_i4(10 + $slot),
            [CodingAdventures::ClrSimulator::STLOC_0 + $slot],
            [CodingAdventures::ClrSimulator::LDLOC_0 + $slot],
            [CodingAdventures::ClrSimulator::RET],
        ]);
        is($sim->{stack}[-1],     10 + $slot, "stack top = ${\(10+$slot)}");
        is($sim->{locals}[$slot], 10 + $slot, "locals[$slot] = ${\(10+$slot)}");
    }
};

# ============================================================================
# stloc.s / ldloc.s
# ============================================================================

subtest 'stloc.s / ldloc.s' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(77),
        [CodingAdventures::ClrSimulator::STLOC_S, 5],
        [CodingAdventures::ClrSimulator::LDLOC_S, 5],
        [CodingAdventures::ClrSimulator::RET],
    ], num_locals => 16);
    is($sim->{stack}[-1], 77,  'stack top = 77');
    is($sim->{locals}[5], 77,  'locals[5] = 77');
};

# ============================================================================
# Arithmetic
# ============================================================================

subtest 'add' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(3),
        CodingAdventures::ClrSimulator::encode_ldc_i4(4),
        [CodingAdventures::ClrSimulator::ADD],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 7, '3 + 4 = 7');
};

subtest 'sub' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(10),
        CodingAdventures::ClrSimulator::encode_ldc_i4(3),
        [CodingAdventures::ClrSimulator::SUB],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 7, '10 - 3 = 7');
};

subtest 'mul' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(6),
        CodingAdventures::ClrSimulator::encode_ldc_i4(7),
        [CodingAdventures::ClrSimulator::MUL],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 42, '6 * 7 = 42');
};

subtest 'div' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(10),
        CodingAdventures::ClrSimulator::encode_ldc_i4(3),
        [CodingAdventures::ClrSimulator::DIV],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 3, '10 / 3 = 3 (truncated)');

    # Negative truncation toward zero: -7 / 2 = -3 (not -4)
    $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(-7),
        CodingAdventures::ClrSimulator::encode_ldc_i4(2),
        [CodingAdventures::ClrSimulator::DIV],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], -3, '-7 / 2 = -3 (truncate toward zero)');

    # Division by zero
    my $s2 = CodingAdventures::ClrSimulator->new();
    $s2->load(CodingAdventures::ClrSimulator::assemble([
        CodingAdventures::ClrSimulator::encode_ldc_i4(5),
        CodingAdventures::ClrSimulator::encode_ldc_i4(0),
        [CodingAdventures::ClrSimulator::DIV],
        [CodingAdventures::ClrSimulator::RET],
    ]));
    $s2->step(); $s2->step();  # push 5, push 0
    ok(dies { $s2->step() }, 'division by zero raises');
};

# ============================================================================
# Compare instructions (two-byte 0xFE prefix)
# ============================================================================

subtest 'ceq (0xFE 0x01)' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(5),
        CodingAdventures::ClrSimulator::encode_ldc_i4(5),
        [CodingAdventures::ClrSimulator::PREFIX_FE, CodingAdventures::ClrSimulator::CEQ_BYTE],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 1, 'equal → 1');

    $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(3),
        CodingAdventures::ClrSimulator::encode_ldc_i4(5),
        [CodingAdventures::ClrSimulator::PREFIX_FE, CodingAdventures::ClrSimulator::CEQ_BYTE],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 0, 'unequal → 0');
};

subtest 'cgt (0xFE 0x02)' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(10),
        CodingAdventures::ClrSimulator::encode_ldc_i4(3),
        [CodingAdventures::ClrSimulator::PREFIX_FE, CodingAdventures::ClrSimulator::CGT_BYTE],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 1, '10 > 3 → 1');

    $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(3),
        CodingAdventures::ClrSimulator::encode_ldc_i4(10),
        [CodingAdventures::ClrSimulator::PREFIX_FE, CodingAdventures::ClrSimulator::CGT_BYTE],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 0, '3 > 10 → 0');
};

subtest 'clt (0xFE 0x04)' => sub {
    my $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(2),
        CodingAdventures::ClrSimulator::encode_ldc_i4(8),
        [CodingAdventures::ClrSimulator::PREFIX_FE, CodingAdventures::ClrSimulator::CLT_BYTE],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 1, '2 < 8 → 1');

    $sim = run_program([
        CodingAdventures::ClrSimulator::encode_ldc_i4(8),
        CodingAdventures::ClrSimulator::encode_ldc_i4(2),
        [CodingAdventures::ClrSimulator::PREFIX_FE, CodingAdventures::ClrSimulator::CLT_BYTE],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    is($sim->{stack}[-1], 0, '8 < 2 → 0');
};

# ============================================================================
# Branch instructions
# ============================================================================

subtest 'br.s unconditional branch' => sub {
    # PC: 0=LDC_I4_1, 1=BR_S +1, 3=LDC_I4_2 (skip), 4=LDC_I4_3, 5=RET
    my $code = [
        CodingAdventures::ClrSimulator::LDC_I4_1,   # pc 0
        CodingAdventures::ClrSimulator::BR_S, 1,    # pc 1: jump pc=4
        CodingAdventures::ClrSimulator::LDC_I4_2,   # pc 3: skipped
        CodingAdventures::ClrSimulator::LDC_I4_3,   # pc 4
        CodingAdventures::ClrSimulator::RET,
    ];
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load($code);
    $sim->run();
    is($sim->{stack}[0], 1, 'first stack item = 1');
    is($sim->{stack}[1], 3, 'second stack item = 3 (2 was skipped)');
};

subtest 'brfalse.s branches on 0' => sub {
    # push 0, brfalse.s +2, push 42 (skipped, 2-byte ldc.i4.s), push 99, ret → [99]
    # encode_ldc_i4(42) produces 2 bytes (ldc.i4.s + operand), so offset must be 2
    my $code = CodingAdventures::ClrSimulator::assemble([
        CodingAdventures::ClrSimulator::encode_ldc_i4(0),
        [CodingAdventures::ClrSimulator::BRFALSE_S, 2],
        CodingAdventures::ClrSimulator::encode_ldc_i4(42),
        CodingAdventures::ClrSimulator::encode_ldc_i4(99),
        [CodingAdventures::ClrSimulator::RET],
    ]);
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load($code);
    $sim->run();
    is($sim->{stack}[-1], 99, 'stack top = 99 (42 skipped)');
    is(scalar @{$sim->{stack}}, 1, 'only one item on stack');
};

subtest 'brfalse.s does not branch on non-zero' => sub {
    my $code = CodingAdventures::ClrSimulator::assemble([
        CodingAdventures::ClrSimulator::encode_ldc_i4(1),
        [CodingAdventures::ClrSimulator::BRFALSE_S, 2],
        CodingAdventures::ClrSimulator::encode_ldc_i4(42),
        CodingAdventures::ClrSimulator::encode_ldc_i4(99),
        [CodingAdventures::ClrSimulator::RET],
    ]);
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load($code);
    $sim->run();
    is($sim->{stack}[0], 42, 'stack[0] = 42');
    is($sim->{stack}[1], 99, 'stack[1] = 99');
};

subtest 'brfalse.s treats null as false' => sub {
    my $code = [
        CodingAdventures::ClrSimulator::LDNULL,
        CodingAdventures::ClrSimulator::BRFALSE_S, 1,
        CodingAdventures::ClrSimulator::LDC_I4_1,
        CodingAdventures::ClrSimulator::LDC_I4_2,
        CodingAdventures::ClrSimulator::RET,
    ];
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load($code);
    $sim->run();
    is($sim->{stack}[-1], 2, 'null treated as false → branch taken');
};

subtest 'brtrue.s branches on non-zero' => sub {
    # encode_ldc_i4(42) produces 2 bytes, so offset must be 2 to skip it
    my $code = CodingAdventures::ClrSimulator::assemble([
        CodingAdventures::ClrSimulator::encode_ldc_i4(1),
        [CodingAdventures::ClrSimulator::BRTRUE_S, 2],
        CodingAdventures::ClrSimulator::encode_ldc_i4(42),
        CodingAdventures::ClrSimulator::encode_ldc_i4(99),
        [CodingAdventures::ClrSimulator::RET],
    ]);
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load($code);
    $sim->run();
    is($sim->{stack}[-1], 99, 'brtrue.s taken for 1');
};

subtest 'brtrue.s does not branch on 0' => sub {
    my $code = CodingAdventures::ClrSimulator::assemble([
        CodingAdventures::ClrSimulator::encode_ldc_i4(0),
        [CodingAdventures::ClrSimulator::BRTRUE_S, 2],
        CodingAdventures::ClrSimulator::encode_ldc_i4(42),
        CodingAdventures::ClrSimulator::encode_ldc_i4(99),
        [CodingAdventures::ClrSimulator::RET],
    ]);
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load($code);
    $sim->run();
    is($sim->{stack}[0], 42, 'stack[0] = 42 (branch not taken)');
};

# ============================================================================
# ret
# ============================================================================

subtest 'ret halts the simulator' => sub {
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::RET]);
    $sim->step();
    ok($sim->{halted}, 'halted after ret');
    ok(dies { $sim->step() }, 'step after halt raises');
};

# ============================================================================
# Error conditions
# ============================================================================

subtest 'error conditions' => sub {
    # Stack underflow
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::ADD, CodingAdventures::ClrSimulator::RET]);
    ok(dies { $sim->step() }, 'stack underflow raises');

    # PC out of range
    $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([]);
    ok(dies { $sim->step() }, 'PC past end raises');

    # Unknown opcode
    $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([0xFF]);
    ok(dies { $sim->step() }, 'unknown opcode raises');

    # Incomplete two-byte opcode
    $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::LDC_I4_1,
                CodingAdventures::ClrSimulator::LDC_I4_1,
                CodingAdventures::ClrSimulator::PREFIX_FE]);
    $sim->step(); $sim->step();
    ok(dies { $sim->step() }, 'incomplete 0xFE opcode raises');

    # Unknown 0xFE subopcode
    $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::LDC_I4_1,
                CodingAdventures::ClrSimulator::LDC_I4_1,
                CodingAdventures::ClrSimulator::PREFIX_FE, 0xFF]);
    $sim->step(); $sim->step();
    ok(dies { $sim->step() }, 'unknown 0xFE subopcode raises');

    # Uninitialized local (ldloc.0)
    $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::LDLOC_0, CodingAdventures::ClrSimulator::RET]);
    ok(dies { $sim->step() }, 'uninitialized local raises');

    # Uninitialized local (ldloc.s)
    $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::LDLOC_S, 5, CodingAdventures::ClrSimulator::RET]);
    ok(dies { $sim->step() }, 'uninitialized local.s raises');
};

# ============================================================================
# run() and traces
# ============================================================================

subtest 'run returns traces' => sub {
    my $sim = CodingAdventures::ClrSimulator->new();
    my $code = CodingAdventures::ClrSimulator::assemble([
        CodingAdventures::ClrSimulator::encode_ldc_i4(5),
        CodingAdventures::ClrSimulator::encode_ldc_i4(3),
        [CodingAdventures::ClrSimulator::ADD],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    $sim->load($code);
    my $traces = $sim->run();
    is(scalar @$traces, 4, '4 traces for 4 instructions');
    is($sim->{stack}[-1], 8, '5 + 3 = 8');
};

subtest 'run stops at max_steps' => sub {
    # Infinite loop: br.s 0xFE (offset=-2)
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::BR_S, 0xFE]);
    my $traces = $sim->run(max_steps => 5);
    is(scalar @$traces, 5, 'stopped at 5 steps');
    ok(!$sim->{halted}, 'still not halted');
};

subtest 'trace structure' => sub {
    my $sim = CodingAdventures::ClrSimulator->new();
    $sim->load([CodingAdventures::ClrSimulator::LDC_I4_3,
                CodingAdventures::ClrSimulator::RET]);
    my $trace = $sim->step();
    is($trace->{pc},       0,         'pc=0');
    is($trace->{opcode},   'ldc.i4.3','opcode');
    is(scalar @{$trace->{stack_before}}, 0, 'empty stack before');
    is($trace->{stack_after}[0],         3, 'stack after has 3');
    like($trace->{description}, qr/push 3/, 'description mentions push');
};

# ============================================================================
# Integration: x = 1 + 2
# ============================================================================

subtest 'integration: x = 1 + 2' => sub {
    my $sim = run_program([
        [CodingAdventures::ClrSimulator::LDC_I4_1],
        [CodingAdventures::ClrSimulator::LDC_I4_2],
        [CodingAdventures::ClrSimulator::ADD],
        [CodingAdventures::ClrSimulator::STLOC_0],
        [CodingAdventures::ClrSimulator::RET],
    ]);
    ok($sim->{halted}, 'halted');
    is($sim->{locals}[0], 3, 'locals[0] = 3');
};

# ============================================================================
# assemble helper
# ============================================================================

subtest 'assemble' => sub {
    my $result = CodingAdventures::ClrSimulator::assemble([
        [0x10, 0x20],
        [0x30],
        [0x40],
    ]);
    is($result->[0], 0x10, 'byte 0');
    is($result->[1], 0x20, 'byte 1');
    is($result->[2], 0x30, 'byte 2');
    is($result->[3], 0x40, 'byte 3');
};

done_testing();
