use strict;
use warnings;
use Test2::V0;

# Load dependencies from sibling directories via PERL5LIB
ok( eval { require CodingAdventures::CpuPipeline;  1 }, 'CpuPipeline loads' );
ok( eval { require CodingAdventures::CpuSimulator; 1 }, 'CpuSimulator loads' );
ok( eval { require CodingAdventures::Core;         1 }, 'CodingAdventures::Core loads' );

my $Core       = 'CodingAdventures::Core::Core';
my $CoreConfig = 'CodingAdventures::Core::CoreConfig';

# ============================================================================
# Minimal ISA decoder for testing
# ============================================================================
#
# Instruction encoding (simplified):
#   opcode = raw & 0xFF
#   0x00 = NOP
#   0xFF = HALT
#   0x01 = LOAD_IMM: rd = (raw >> 8) & 0xF, imm = (raw >> 16) & 0xFF
#   0x02 = ADD_IMM:  rd = (raw >> 8) & 0xF, rs1 = (raw >> 12) & 0xF, imm = (raw >> 16) & 0xFF

package NopDecoder;
sub new { bless {}, shift }
sub decode {
    my ($self, $raw, $token) = @_;
    my $opcode = $raw & 0xFF;
    if ($opcode == 0xFF) {
        $token->{opcode}  = 'HALT';
        $token->{is_halt} = 1;
    } elsif ($opcode == 0x01) {
        $token->{opcode}     = 'LOAD_IMM';
        $token->{rd}         = ($raw >> 8) & 0xF;
        $token->{immediate}  = ($raw >> 16) & 0xFF;
        $token->{reg_write}  = 1;
    } elsif ($opcode == 0x02) {
        $token->{opcode}      = 'ADD_IMM';
        $token->{rd}          = ($raw >> 8)  & 0xF;
        $token->{rs1}         = ($raw >> 12) & 0xF;
        $token->{immediate}   = ($raw >> 16) & 0xFF;
        $token->{reg_write}   = 1;
        $token->{source_regs} = [$token->{rs1}];
    } else {
        $token->{opcode} = 'NOP';
    }
    return $token;
}
sub execute {
    my ($self, $token, $rf) = @_;
    if ($token->{opcode} eq 'LOAD_IMM') {
        $token->{alu_result} = $token->{immediate};
        $token->{write_data} = $token->{immediate};
    } elsif ($token->{opcode} eq 'ADD_IMM') {
        my $v1 = $token->{rs1} >= 0 ? $rf->read($token->{rs1}) : 0;
        $token->{alu_result} = ($v1 + $token->{immediate}) & 0xFFFFFFFF;
        $token->{write_data} = $token->{alu_result};
    }
    return $token;
}
sub instruction_size { 4 }

package main;

# ============================================================================
# CoreConfig tests
# ============================================================================

subtest 'CoreConfig::simple' => sub {
    my $cfg = $CoreConfig->simple();
    is($cfg->{name},            'Simple', 'name=Simple');
    is($cfg->{pipeline_config}->num_stages(), 5,  '5 stages');
    is($cfg->{num_registers},   16,       '16 registers');
    is($cfg->{register_width},  32,       '32-bit');
};

subtest 'CoreConfig::performance' => sub {
    my $cfg = $CoreConfig->performance();
    is($cfg->{name}, 'Performance', 'name=Performance');
    is($cfg->{pipeline_config}->num_stages(), 13, '13 stages');
    is($cfg->{num_registers}, 31, '31 registers');
};

subtest 'CoreConfig::new defaults' => sub {
    my $cfg = $CoreConfig->new();
    is($cfg->{name}, 'Core', 'name=Core');
    is($cfg->{pipeline_config}->num_stages(), 5, '5 stages default');
};

# ============================================================================
# Core construction tests
# ============================================================================

subtest 'Core::new — success' => sub {
    my $result = $Core->new($CoreConfig->simple(), NopDecoder->new());
    is($result->{ok}, 1, 'ok=1');
    ok(defined $result->{core}, 'core defined');
};

subtest 'Core initial state' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    is($core->get_cycle(), 0, 'cycle=0');
    is($core->get_pc(),    0, 'pc=0');
    is($core->is_halted(), 0, 'not halted');
    for my $i (0..15) {
        is($core->read_register($i), 0, "R$i=0");
    }
};

# ============================================================================
# Memory and program loading
# ============================================================================

subtest 'Core::load_program stores bytes' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0xEF, 0xBE, 0xAD, 0xDE], 0);
    is($core->read_memory_word(0), 0xDEADBEEF, 'loaded 0xDEADBEEF');
};

subtest 'Core::load_program sets PC' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0x100);
    is($core->get_pc(), 0x100, 'PC=0x100 after load');
};

subtest 'Core memory round-trip' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->write_memory_word(0x20, 0xABCDEF01);
    is($core->read_memory_word(0x20), 0xABCDEF01, 'memory round-trip');
};

# ============================================================================
# step() and run()
# ============================================================================

subtest 'Core::step increments cycle' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    $core->step();
    is($core->get_cycle(), 1, 'cycle=1 after step');
    $core->step();
    is($core->get_cycle(), 2, 'cycle=2 after second step');
};

subtest 'Core::step returns snapshot' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    my $snap = $core->step();
    is($snap->{cycle}, 1, 'snapshot cycle=1');
};

subtest 'Core::run executes multiple cycles' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    $core->run(10);
    is($core->get_cycle(), 10, 'cycle=10 after run(10)');
};

subtest 'Core::run returns CoreStats' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    my $stats = $core->run(10);
    is($stats->{total_cycles}, 10, 'total_cycles=10');
    ok($stats->ipc() > 0, 'IPC > 0');
};

# ============================================================================
# Register access via instruction execution
# ============================================================================

subtest 'Core::write_register/read_register round-trip' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->write_register(3, 999);
    is($core->read_register(3), 999, 'register round-trip');
};

subtest 'LOAD_IMM instruction writes register after WB' => sub {
    # Encode: opcode=0x01, rd=2, immediate=77
    # raw = (77 << 16) | (2 << 8) | 0x01 = 0x004D0201
    my $raw = (77 << 16) | (2 << 8) | 0x01;
    my @bytes;
    push @bytes, $raw & 0xFF;
    push @bytes, ($raw >> 8)  & 0xFF;
    push @bytes, ($raw >> 16) & 0xFF;
    push @bytes, ($raw >> 24) & 0xFF;
    push @bytes, (0, 0, 0, 0);  # padding

    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program(\@bytes, 0);
    $core->run(6);  # 5 stages + 1 extra
    is($core->read_register(2), 77, 'R2=77 after LOAD_IMM 77');
};

# ============================================================================
# Halt detection
# ============================================================================

subtest 'Core halts when HALT reaches WB' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0xFF, 0, 0, 0], 0);
    for (1..10) { $core->step(); last if $core->is_halted(); }
    is($core->is_halted(), 1, 'halted=1 after HALT');
};

subtest 'Core::run stops at max_cycles without halt' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    $core->run(5);
    is($core->get_cycle(), 5, 'stopped at max_cycles=5');
    is($core->is_halted(), 0, 'not halted');
};

# ============================================================================
# Statistics
# ============================================================================

subtest 'CoreStats total_cycles' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    $core->run(20);
    is($core->get_stats()->{total_cycles}, 20, 'total_cycles=20');
};

subtest 'CoreStats IPC approaches 1.0 for NOPs' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    $core->run(50);
    my $ipc = $core->get_stats()->ipc();
    ok($ipc > 0.8, "IPC > 0.8, got $ipc");
};

subtest 'CoreStats::to_string is non-empty' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->run(5);
    my $s = $core->get_stats()->to_string();
    like($s, qr/IPC/, 'to_string contains IPC');
};

# ============================================================================
# Trace
# ============================================================================

subtest 'Core::get_trace returns one snapshot per step' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    $core->step() for 1..3;
    my $trace = $core->get_trace();
    is(scalar(@$trace), 3, '3 snapshots');
};

subtest 'Core trace snapshots have sequential cycles' => sub {
    my $core = $Core->new($CoreConfig->simple(), NopDecoder->new())->{core};
    $core->load_program([0,0,0,0], 0);
    $core->step() for 1..4;
    my $trace = $core->get_trace();
    for my $i (0..$#$trace) {
        is($trace->[$i]{cycle}, $i + 1, "trace[$i].cycle = " . ($i+1));
    }
};

done_testing;
