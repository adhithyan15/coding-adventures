use strict;
use warnings;
use Test2::V0;
use lib '../lib';
use CodingAdventures::CpuPipeline;

my $Token    = 'CodingAdventures::CpuPipeline::Token';
my $Config   = 'CodingAdventures::CpuPipeline::PipelineConfig';
my $Hazard   = 'CodingAdventures::CpuPipeline::HazardResponse';
my $Stats    = 'CodingAdventures::CpuPipeline::PipelineStats';
my $Pipeline = 'CodingAdventures::CpuPipeline::Pipeline';

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

sub make_memory {
    my (%opts) = @_;
    my $halt_at = $opts{halt_at} // 0;
    my @mem = (0x01) x 256;
    $mem[$halt_at] = 0xFF;
    return \@mem;
}

sub make_pipeline {
    my (%opts) = @_;
    my $mem    = $opts{memory} // make_memory(halt_at => 100);
    my $config = $opts{config} // $Config->classic_5_stage();

    my $fetch_fn = sub {
        my ($pc) = @_;
        return $mem->[$pc] // 0;
    };
    my $decode_fn = sub {
        my ($raw, $tok) = @_;
        if ($raw == 0xFF) {
            $tok->{opcode}   = 'HALT';
            $tok->{is_halt}  = 1;
        } else {
            $tok->{opcode} = 'NOP';
        }
        return $tok;
    };
    my $execute_fn   = sub { $_[0] };
    my $memory_fn    = sub { $_[0] };
    my $writeback_fn = sub { };

    my $result = $Pipeline->new($config, $fetch_fn, $decode_fn,
        $execute_fn, $memory_fn, $writeback_fn);
    die "Pipeline.new failed: $result->{err}" unless $result->{ok};
    return $result->{pipeline};
}

# ---------------------------------------------------------------------------
# Token tests
# ---------------------------------------------------------------------------

subtest 'Token::new' => sub {
    my $tok = $Token->new();
    is $tok->{opcode},    '',  'opcode empty';
    is $tok->{pc},        0,   'pc 0';
    is $tok->{rs1},       -1,  'rs1 -1';
    is $tok->{is_bubble}, 0,   'not bubble';
    is $tok->{is_halt},   0,   'not halt';
};

subtest 'Token::new_bubble' => sub {
    my $b = $Token->new_bubble();
    is $b->{is_bubble}, 1,        'is bubble';
    is $b->{opcode},    'BUBBLE', 'opcode BUBBLE';
};

subtest 'Token::to_string' => sub {
    my $tok = $Token->new();
    $tok->{pc}     = 0x10;
    $tok->{opcode} = 'ADD';
    my $s = $tok->to_string();
    like $s, qr/ADD/,  'contains opcode';
    like $s, qr/0010/, 'contains pc hex';
};

subtest 'Token::clone deep-copies stage_entered' => sub {
    my $tok = $Token->new();
    $tok->{stage_entered}{IF} = 3;
    my $c = $Token->clone($tok);
    $c->{stage_entered}{ID} = 4;
    ok !exists $tok->{stage_entered}{ID}, 'original not modified';
};

subtest 'Token::clone undef' => sub {
    is $Token->clone(undef), undef, 'clone(undef) returns undef';
};

# ---------------------------------------------------------------------------
# PipelineConfig tests
# ---------------------------------------------------------------------------

subtest 'PipelineConfig::validate valid' => sub {
    my ($ok, $err) = $Config->validate($Config->classic_5_stage());
    is $ok, 1, 'ok';
    ok !defined $err, 'no error';
};

subtest 'PipelineConfig::validate nil' => sub {
    my ($ok, $err) = $Config->validate(undef);
    is $ok, 0, 'not ok';
    ok defined $err, 'has error';
};

subtest 'PipelineConfig::validate empty stages' => sub {
    my $cfg = $Config->new([], 1);
    my ($ok, $err) = $Config->validate($cfg);
    is $ok, 0, 'not ok';
};

subtest 'PipelineConfig::classic_5_stage num_stages' => sub {
    my $cfg = $Config->classic_5_stage();
    is $cfg->num_stages(), 5, '5 stages';
};

subtest 'PipelineConfig::deep_13_stage' => sub {
    my $cfg = $Config->deep_13_stage();
    is $cfg->num_stages(), 13, '13 stages';
    my ($ok) = $Config->validate($cfg);
    is $ok, 1, 'valid';
};

# ---------------------------------------------------------------------------
# Pipeline::new tests
# ---------------------------------------------------------------------------

subtest 'Pipeline::new success' => sub {
    my $p = make_pipeline();
    ok defined $p, 'pipeline created';
};

subtest 'Pipeline::new invalid config' => sub {
    my $cfg = $Config->new([], 1);
    my $result = $Pipeline->new($cfg, sub{}, sub{$_[1]}, sub{$_[0]}, sub{$_[0]}, sub{});
    is $result->{ok}, 0, 'not ok';
    ok defined $result->{err}, 'has error';
};

subtest 'Pipeline starts at cycle 0 not halted' => sub {
    my $p = make_pipeline(memory => make_memory(halt_at => 100));
    is $p->get_cycle(),  0, 'cycle 0';
    is $p->is_halted(), 0, 'not halted';
};

# ---------------------------------------------------------------------------
# Pipeline::step tests
# ---------------------------------------------------------------------------

subtest 'step advances cycle' => sub {
    my $p = make_pipeline();
    $p->step();
    is $p->get_cycle(), 1, 'cycle 1';
    $p->step();
    is $p->get_cycle(), 2, 'cycle 2';
};

subtest 'step returns snapshot with correct cycle' => sub {
    my $p = make_pipeline();
    my $snap = $p->step();
    is $snap->{cycle}, 1, 'snap cycle 1';
};

subtest 'halts after HALT reaches WB' => sub {
    my @mem = (0) x 256;
    $mem[0] = 0xFF;
    my $p = make_pipeline(memory => \@mem);
    for (1..10) { $p->step() unless $p->is_halted() }
    is $p->is_halted(), 1, 'halted';
};

subtest 'does not advance cycle when halted' => sub {
    my @mem = (0) x 256;
    $mem[0] = 0xFF;
    my $p = make_pipeline(memory => \@mem);
    for (1..10) { $p->step() }
    my $c = $p->get_cycle();
    $p->step();
    is $p->get_cycle(), $c, 'cycle unchanged after halt';
};

# ---------------------------------------------------------------------------
# Stats
# ---------------------------------------------------------------------------

subtest 'PipelineStats ipc 0 with no cycles' => sub {
    my $s = $Stats->new();
    is $s->ipc(), 0.0, 'ipc 0';
};

subtest 'PipelineStats cpi 0 with no instructions' => sub {
    my $s = $Stats->new();
    is $s->cpi(), 0.0, 'cpi 0';
};

subtest 'instructions_completed increments on retire' => sub {
    my @mem = (0x01) x 256;
    $mem[0] = 0xFF;
    my $p = make_pipeline(memory => \@mem);
    $p->run(20);
    ok $p->get_stats()->{instructions_completed} >= 1, 'at least 1 completed';
};

# ---------------------------------------------------------------------------
# Stall via hazard_fn
# ---------------------------------------------------------------------------

subtest 'stall increments stall_cycles' => sub {
    my $p = make_pipeline();
    my $calls = 0;
    $p->set_hazard_fn(sub {
        $calls++;
        return $calls == 2
            ? $Hazard->new(action => 'stall')
            : $Hazard->new(action => 'none');
    });
    $p->step(); $p->step();
    is $p->get_stats()->{stall_cycles}, 1, 'stall_cycles = 1';
};

subtest 'flush increments flush_cycles' => sub {
    my $p = make_pipeline();
    my $calls = 0;
    $p->set_hazard_fn(sub {
        $calls++;
        return $calls == 2
            ? $Hazard->new(action => 'flush', flush_count => 2, redirect_pc => 0x10)
            : $Hazard->new(action => 'none');
    });
    $p->step(); $p->step();
    is $p->get_stats()->{flush_cycles}, 1, 'flush_cycles = 1';
};

# ---------------------------------------------------------------------------
# Trace
# ---------------------------------------------------------------------------

subtest 'get_trace returns one entry per step' => sub {
    my $p = make_pipeline();
    $p->step(); $p->step(); $p->step();
    my $trace = $p->get_trace();
    is scalar @$trace, 3, '3 trace entries';
    is $trace->[0]{cycle}, 1, 'first entry cycle 1';
    is $trace->[2]{cycle}, 3, 'last entry cycle 3';
};

# ---------------------------------------------------------------------------
# predict_fn
# ---------------------------------------------------------------------------

subtest 'predict_fn overrides PC advance' => sub {
    my @fetched;
    my $p = make_pipeline();
    my $orig_fetch = $p->{fetch_fn};
    $p->{fetch_fn} = sub { push @fetched, $_[0]; return 0x01 };
    $p->set_predict_fn(sub { return 100 });
    $p->step();
    is $fetched[0], 0, 'first fetch at PC 0';
};

# ---------------------------------------------------------------------------
# 13-stage pipeline
# ---------------------------------------------------------------------------

subtest '13-stage pipeline runs without error' => sub {
    my @mem = (0x01) x 256;
    $mem[0] = 0xFF;
    my $p = make_pipeline(
        memory => \@mem,
        config => $Config->deep_13_stage(),
    );
    $p->run(30);
    is $p->is_halted(), 1, 'halted';
};

done_testing;
