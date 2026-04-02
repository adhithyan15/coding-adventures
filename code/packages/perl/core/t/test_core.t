use strict;
use warnings;
use Test2::V0;
use lib '../lib';
use lib '../../cpu-pipeline/lib';
use lib '../../cpu-simulator/lib';
use CodingAdventures::Core;
use CodingAdventures::CpuPipeline;
use CodingAdventures::CpuSimulator;

my $CoreCls    = 'CodingAdventures::Core::Core';
my $CoreConfig = 'CodingAdventures::Core::CoreConfig';
my $CoreStats  = 'CodingAdventures::Core::CoreStats';

# ---------------------------------------------------------------------------
# Minimal ISA decoder
# Opcodes (low byte):
#   0x00 = NOP
#   0xFF = HALT
#   0x01 = LOAD_IMM: rd=2, imm=77 → R2 = 77
#   0x02 = ADD_IMM:  rd=3, imm=10 → R3 = R3 + 10
# ---------------------------------------------------------------------------

package NopDecoder;

sub new { bless {}, $_[0] }

sub decode {
    my ($self, $raw, $token) = @_;
    my $op = $raw & 0xFF;
    if ($op == 0xFF) {
        $token->{opcode}   = 'HALT';
        $token->{is_halt}  = 1;
    } elsif ($op == 0x01) {
        $token->{opcode}    = 'LOAD_IMM';
        $token->{rd}        = 2;
        $token->{immediate} = 77;
        $token->{reg_write} = 1;
    } elsif ($op == 0x02) {
        $token->{opcode}    = 'ADD_IMM';
        $token->{rd}        = 3;
        $token->{rs1}       = 3;
        $token->{immediate} = 10;
        $token->{reg_write} = 1;
    } else {
        $token->{opcode} = 'NOP';
    }
    return $token;
}

sub execute {
    my ($self, $token, $reg_file) = @_;
    if ($token->{opcode} eq 'LOAD_IMM') {
        $token->{alu_result} = $token->{immediate};
        $token->{write_data} = $token->{immediate};
    } elsif ($token->{opcode} eq 'ADD_IMM') {
        my $src = $token->{rs1} >= 0 ? $reg_file->read($token->{rs1}) : 0;
        $token->{alu_result} = $src + $token->{immediate};
        $token->{write_data} = $token->{alu_result};
    }
    return $token;
}

sub instruction_size { 4 }

package main;

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

sub make_core {
    my (%opts) = @_;
    my $config  = $opts{config}  // $CoreConfig->simple();
    my $decoder = $opts{decoder} // NopDecoder->new();
    my $result  = $CoreCls->new($config, $decoder);
    die "Core.new failed: $result->{err}" unless $result->{ok};
    return $result->{core};
}

# ---------------------------------------------------------------------------
# CoreConfig presets
# ---------------------------------------------------------------------------

subtest 'CoreConfig::simple' => sub {
    my $cfg = $CoreConfig->simple();
    is $cfg->{name},          'Simple', 'name';
    is $cfg->{num_registers}, 16,       'num_registers';
    is $cfg->{register_width}, 32,      'register_width';
    is $cfg->{memory_size},   65536,    'memory_size';
};

subtest 'CoreConfig::performance' => sub {
    my $cfg = $CoreConfig->performance();
    is $cfg->{name},          'Performance', 'name';
    is $cfg->{num_registers}, 31,            'num_registers';
};

# ---------------------------------------------------------------------------
# Core construction
# ---------------------------------------------------------------------------

subtest 'Core::new succeeds with valid config+decoder' => sub {
    my $r = $CoreCls->new($CoreConfig->simple(), NopDecoder->new());
    is $r->{ok}, 1, 'ok';
    ok defined $r->{core}, 'core defined';
};

subtest 'Core::new nil config' => sub {
    my $r = $CoreCls->new(undef, NopDecoder->new());
    is $r->{ok}, 0, 'not ok';
    ok defined $r->{err}, 'has error';
};

subtest 'Core::new nil decoder' => sub {
    my $r = $CoreCls->new($CoreConfig->simple(), undef);
    is $r->{ok}, 0, 'not ok';
};

subtest 'Core starts not halted at cycle 0' => sub {
    my $c = make_core();
    is $c->is_halted(), 0, 'not halted';
    is $c->get_cycle(), 0, 'cycle 0';
};

# ---------------------------------------------------------------------------
# load_program / step / run
# ---------------------------------------------------------------------------

subtest 'Core::load_program loads memory' => sub {
    my $c = make_core();
    $c->load_program([0xFF, 0, 0, 0], 0);
    my $w = $c->read_memory_word(0);
    is $w & 0xFF, 0xFF, 'HALT byte at address 0';
};

subtest 'Core::step advances cycle' => sub {
    my $c = make_core();
    $c->load_program([0x00, 0,0,0, 0xFF, 0,0,0], 0);
    $c->step();
    is $c->get_cycle(), 1, 'cycle 1';
};

subtest 'Core::step returns snapshot' => sub {
    my $c = make_core();
    $c->load_program([0x00, 0,0,0], 0);
    my $snap = $c->step();
    is $snap->{cycle}, 1, 'snap cycle 1';
};

subtest 'Core::step does not advance when halted' => sub {
    my $c = make_core();
    $c->load_program([0xFF, 0,0,0], 0);
    $c->step() for 1..10;
    my $after = $c->get_cycle();
    $c->step();
    is $c->get_cycle(), $after, 'cycle unchanged';
};

subtest 'Core::run — halts after HALT reaches WB' => sub {
    my $c = make_core();
    $c->load_program([0xFF, 0,0,0], 0);
    $c->run(20);
    is $c->is_halted(), 1, 'halted';
};

subtest 'Core::run — NOP+NOP+HALT' => sub {
    my $c = make_core();
    $c->load_program([0x00,0,0,0, 0x00,0,0,0, 0xFF,0,0,0], 0);
    $c->run(30);
    is $c->is_halted(), 1, 'halted';
};

# ---------------------------------------------------------------------------
# Register access
# ---------------------------------------------------------------------------

subtest 'register access' => sub {
    my $c = make_core();
    is $c->read_register(0), 0, 'R0 = 0 initially';
    $c->write_register(5, 0xBEEF);
    is $c->read_register(5), 0xBEEF, 'R5 = 0xBEEF after write';
};

# ---------------------------------------------------------------------------
# LOAD_IMM execution end-to-end
# ---------------------------------------------------------------------------

subtest 'LOAD_IMM writes R2 = 77' => sub {
    my $c = make_core();
    # opcode 0x01 = LOAD_IMM (R2 ← 77), then HALT
    $c->load_program([0x01,0,0,0, 0xFF,0,0,0], 0);
    $c->run(20);
    is $c->is_halted(),     1,  'halted';
    is $c->read_register(2), 77, 'R2 = 77';
};

# ---------------------------------------------------------------------------
# Stats
# ---------------------------------------------------------------------------

subtest 'CoreStats ipc/cpi' => sub {
    my $s = $CoreStats->new(instructions_completed => 10, total_cycles => 20);
    is $s->ipc(), 0.5, 'ipc 0.5';
    is $s->cpi(), 2.0, 'cpi 2.0';
};

subtest 'CoreStats ipc 0 when no cycles' => sub {
    my $s = $CoreStats->new();
    is $s->ipc(), 0.0, 'ipc 0';
};

subtest 'get_stats after run' => sub {
    my $c = make_core();
    $c->load_program([0xFF, 0,0,0], 0);
    $c->run(20);
    my $stats = $c->get_stats();
    ok $stats->{total_cycles} > 0,           'total_cycles > 0';
    ok $stats->{instructions_completed} >= 1, 'at least 1 completed';
};

# ---------------------------------------------------------------------------
# Trace
# ---------------------------------------------------------------------------

subtest 'get_trace returns one snapshot per step' => sub {
    my $c = make_core();
    $c->load_program([0x00,0,0,0, 0xFF,0,0,0], 0);
    $c->step(); $c->step(); $c->step();
    my $trace = $c->get_trace();
    is scalar @$trace, 3, '3 trace entries';
    is $trace->[0]{cycle}, 1, 'first cycle = 1';
};

# ---------------------------------------------------------------------------
# Performance config
# ---------------------------------------------------------------------------

subtest 'Core with performance config (13-stage)' => sub {
    my $c = make_core(config => $CoreConfig->performance());
    $c->load_program([0xFF, 0,0,0], 0);
    $c->run(40);
    is $c->is_halted(), 1, 'halted';
};

done_testing;
