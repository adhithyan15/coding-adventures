use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CpuPipeline; 1 },
    'CodingAdventures::CpuPipeline loads' );

my $Token    = 'CodingAdventures::CpuPipeline::Token';
my $Config   = 'CodingAdventures::CpuPipeline::PipelineConfig';
my $Pipeline = 'CodingAdventures::CpuPipeline::Pipeline';
my $Hazard   = 'CodingAdventures::CpuPipeline::HazardResponse';
my $Stats    = 'CodingAdventures::CpuPipeline::PipelineStats';
my $Snap     = 'CodingAdventures::CpuPipeline::Snapshot';

# ============================================================================
# Helper: build a no-op 5-stage pipeline
# ============================================================================

sub noop_pipeline {
    my ($config) = @_;
    $config //= $Config->classic_5_stage();
    my $r = $Pipeline->new(
        $config,
        sub { 0 },         # fetch
        sub { $_[1] },     # decode (returns token unchanged)
        sub { $_[0] },     # execute
        sub { $_[0] },     # memory
        sub { },           # writeback
    );
    ok($r->{ok}, "pipeline created: $r->{err}") if !$r->{ok};
    return $r->{pipeline};
}

# ============================================================================
# Token tests
# ============================================================================

subtest 'Token basics' => sub {
    my $t = $Token->new();
    is($t->{pc},        0,  'pc defaults to 0');
    is($t->{opcode},    '', 'opcode defaults to empty');
    is($t->{rs1},       -1, 'rs1 defaults to -1');
    is($t->{is_bubble}, 0,  'is_bubble defaults to false');
    is($t->{reg_write}, 0,  'reg_write defaults to false');
};

subtest 'Token::new_bubble' => sub {
    my $b = $Token->new_bubble();
    is($b->{is_bubble}, 1,     'is_bubble is true');
    is($b->to_string(), '---', 'to_string returns ---');
};

subtest 'Token::to_string' => sub {
    my $t = $Token->new();
    $t->{opcode} = 'ADD';
    $t->{pc}     = 100;
    is($t->to_string(), 'ADD@100', 'named token shows opcode@pc');

    my $u = $Token->new();
    $u->{pc} = 200;
    is($u->to_string(), 'instr@200', 'undecoded token shows instr@pc');
};

subtest 'Token::clone' => sub {
    my $t = $Token->new();
    $t->{pc}     = 42;
    $t->{opcode} = 'SUB';
    $t->{stage_entered}{IF} = 5;

    my $c = $t->clone();
    is($c->{pc},               42,    'clone has same pc');
    is($c->{opcode},           'SUB', 'clone has same opcode');
    is($c->{stage_entered}{IF}, 5,    'clone has same stage_entered');

    $t->{pc} = 99;
    $t->{stage_entered}{IF} = 99;
    is($c->{pc},               42, 'mutating original does not affect clone');
    is($c->{stage_entered}{IF}, 5, 'stage_entered is deep copied');

    is(CodingAdventures::CpuPipeline::Token->clone(undef), undef, 'clone(undef) = undef');
};

# ============================================================================
# PipelineConfig tests
# ============================================================================

subtest 'PipelineConfig::classic_5_stage' => sub {
    my $cfg = $Config->classic_5_stage();
    is($cfg->num_stages(), 5,      '5 stages');
    is($cfg->{stages}[0]{name},    'IF',  'stage 0 = IF');
    is($cfg->{stages}[1]{name},    'ID',  'stage 1 = ID');
    is($cfg->{stages}[2]{name},    'EX',  'stage 2 = EX');
    is($cfg->{stages}[3]{name},    'MEM', 'stage 3 = MEM');
    is($cfg->{stages}[4]{name},    'WB',  'stage 4 = WB');

    is($cfg->{stages}[0]{category}, 'fetch',     'IF is fetch');
    is($cfg->{stages}[4]{category}, 'writeback', 'WB is writeback');
};

subtest 'PipelineConfig::deep_13_stage' => sub {
    my $cfg = $Config->deep_13_stage();
    is($cfg->num_stages(), 13, '13 stages');
};

subtest 'PipelineConfig::validate — too few stages' => sub {
    my $S = 'CodingAdventures::CpuPipeline::PipelineStage';
    my $cfg = CodingAdventures::CpuPipeline::PipelineConfig->new(
        [ $S->new('IF', 'only one', 'fetch') ]
    );
    my ($ok, $err) = $cfg->validate();
    is($ok, 0, 'validation fails');
    like($err, qr/at least 2/, 'error mentions at least 2');
};

subtest 'PipelineConfig::validate — duplicate name' => sub {
    my $S = 'CodingAdventures::CpuPipeline::PipelineStage';
    my $cfg = CodingAdventures::CpuPipeline::PipelineConfig->new([
        $S->new('IF', 'first', 'fetch'),
        $S->new('IF', 'dup',   'writeback'),
    ]);
    my ($ok, $err) = $cfg->validate();
    is($ok, 0, 'validation fails');
    like($err, qr/duplicate/, 'error mentions duplicate');
};

subtest 'PipelineConfig::validate — no fetch stage' => sub {
    my $S = 'CodingAdventures::CpuPipeline::PipelineStage';
    my $cfg = CodingAdventures::CpuPipeline::PipelineConfig->new([
        $S->new('EX', 'execute',   'execute'),
        $S->new('WB', 'writeback', 'writeback'),
    ]);
    my ($ok, $err) = $cfg->validate();
    is($ok, 0, 'validation fails');
    like($err, qr/fetch/, 'error mentions fetch');
};

subtest 'PipelineConfig::validate — valid config' => sub {
    my $cfg = $Config->classic_5_stage();
    my ($ok, $err) = $cfg->validate();
    is($ok, 1, 'valid config passes');
    ok(!defined $err, 'no error message');
};

# ============================================================================
# Pipeline construction tests
# ============================================================================

subtest 'Pipeline::new — success' => sub {
    my $r = $Pipeline->new(
        $Config->classic_5_stage(),
        sub { 0 }, sub { $_[1] }, sub { $_[0] }, sub { $_[0] }, sub { },
    );
    is($r->{ok}, 1, 'ok=1');
    ok(defined $r->{pipeline}, 'pipeline defined');
};

subtest 'Pipeline::new — invalid config' => sub {
    my $S = 'CodingAdventures::CpuPipeline::PipelineStage';
    my $bad = CodingAdventures::CpuPipeline::PipelineConfig->new(
        [ $S->new('IF', 'only', 'fetch') ]
    );
    my $r = $Pipeline->new($bad, sub {}, sub {}, sub {}, sub {}, sub {});
    is($r->{ok}, 0, 'ok=0 for invalid config');
    ok(defined $r->{err}, 'err is set');
};

subtest 'Pipeline initial state' => sub {
    my $p = noop_pipeline();
    is($p->get_cycle(), 0, 'cycle starts at 0');
    is($p->get_pc(),    0, 'pc starts at 0');
    is($p->is_halted(), 0, 'not halted');
};

# ============================================================================
# Basic step() tests
# ============================================================================

subtest 'Pipeline::step advances cycle' => sub {
    my $p = noop_pipeline();
    $p->step(); is($p->get_cycle(), 1, 'cycle=1 after step 1');
    $p->step(); is($p->get_cycle(), 2, 'cycle=2 after step 2');
};

subtest 'Pipeline::step advances PC by 4' => sub {
    my $p = noop_pipeline();
    $p->set_pc(0);
    $p->step();
    is($p->get_pc(), 4, 'PC=4 after first step');
    $p->step();
    is($p->get_pc(), 8, 'PC=8 after second step');
};

subtest 'Pipeline::step returns Snapshot with correct cycle' => sub {
    my $p    = noop_pipeline();
    my $snap = $p->step();
    is($snap->{cycle}, 1, 'snapshot cycle=1');
    $snap = $p->step();
    is($snap->{cycle}, 2, 'snapshot cycle=2');
};

subtest 'Pipeline::step uses custom predict_fn' => sub {
    my $p = noop_pipeline();
    $p->set_predict_fn(sub { $_[0] + 2 });
    $p->set_pc(0);
    $p->step();
    is($p->get_pc(), 2, 'PC=2 with +2 predictor');
};

subtest 'Pipeline does not advance when halted' => sub {
    my $p = noop_pipeline();
    $p->{halted} = 1;
    $p->step();
    is($p->get_cycle(), 0, 'cycle unchanged when halted');
};

# ============================================================================
# Halt propagation
# ============================================================================

subtest 'Halt instruction propagates through pipeline' => sub {
    my $r = $Pipeline->new(
        $Config->classic_5_stage(),
        sub { 0 },
        sub { my ($raw, $t) = @_; $t->{opcode} = 'HALT'; $t->{is_halt} = 1; $t },
        sub { $_[0] },
        sub { $_[0] },
        sub { },
    );
    my $p = $r->{pipeline};
    for (1..6) { $p->step(); last if $p->is_halted(); }
    is($p->is_halted(), 1, 'pipeline halted after HALT reaches WB');
};

# ============================================================================
# Statistics tests
# ============================================================================

subtest 'PipelineStats::ipc after run' => sub {
    my $p = noop_pipeline();
    $p->run(100);
    my $stats = $p->stats();
    is($stats->{total_cycles}, 100, 'total_cycles=100');
    # 100 cycles - 4 fill cycles = 96 completions
    is($stats->{instructions_completed}, 96, '96 instructions completed');
    my $ipc = $stats->ipc();
    ok($ipc > 0.9, "IPC > 0.9, got $ipc");
};

subtest 'PipelineStats::ipc is 0 before any cycles' => sub {
    my $p     = noop_pipeline();
    my $stats = $p->stats();
    is($stats->ipc(), 0.0, 'IPC=0 before cycles');
};

subtest 'PipelineStats::cpi is 0 before completions' => sub {
    my $p = noop_pipeline();
    $p->step() for 1..3;
    my $stats = $p->stats();
    is($stats->{instructions_completed}, 0, '0 completions in 3 cycles');
    is($stats->cpi(), 0.0, 'CPI=0');
};

subtest 'PipelineStats::to_string is non-empty' => sub {
    my $p = noop_pipeline();
    $p->run(10);
    my $s = $p->stats()->to_string();
    like($s, qr/IPC/, 'to_string contains IPC');
};

# ============================================================================
# Stall tests
# ============================================================================

subtest 'Stall increments stall_cycles' => sub {
    my $stall_count = 0;
    my $r = $Pipeline->new(
        $Config->classic_5_stage(),
        sub { 0 }, sub { $_[1] }, sub { $_[0] }, sub { $_[0] }, sub { },
    );
    my $p = $r->{pipeline};
    $p->set_hazard_fn(sub {
        if ($stall_count < 2) {
            $stall_count++;
            return $Hazard->new(action => 'stall');
        }
        return $Hazard->new(action => 'none');
    });
    $p->run(5);
    is($p->stats()->{stall_cycles}, 2, '2 stall cycles recorded');
};

subtest 'Stall snapshot.stalled is true' => sub {
    my $triggered = 0;
    my $r = $Pipeline->new(
        $Config->classic_5_stage(),
        sub { 0 }, sub { $_[1] }, sub { $_[0] }, sub { $_[0] }, sub { },
    );
    my $p = $r->{pipeline};
    $p->set_hazard_fn(sub {
        if (!$triggered) { $triggered = 1; return $Hazard->new(action => 'stall') }
        return $Hazard->new(action => 'none');
    });
    my $snap = $p->step();
    is($snap->{stalled}, 1, 'snapshot.stalled=1 on stall cycle');
};

# ============================================================================
# Flush tests
# ============================================================================

subtest 'Flush increments flush_cycles' => sub {
    my $flush_count = 0;
    my $r = $Pipeline->new(
        $Config->classic_5_stage(),
        sub { 0 }, sub { $_[1] }, sub { $_[0] }, sub { $_[0] }, sub { },
    );
    my $p = $r->{pipeline};
    $p->set_hazard_fn(sub {
        if ($flush_count < 3) {
            $flush_count++;
            return $Hazard->new(action => 'flush', redirect_pc => 0);
        }
        return $Hazard->new(action => 'none');
    });
    $p->run(6);
    is($p->stats()->{flush_cycles}, 3, '3 flush cycles recorded');
};

subtest 'Flush redirects PC' => sub {
    my $flushed = 0;
    my $r = $Pipeline->new(
        $Config->classic_5_stage(),
        sub { 0 }, sub { $_[1] }, sub { $_[0] }, sub { $_[0] }, sub { },
    );
    my $p = $r->{pipeline};
    $p->set_hazard_fn(sub {
        if (!$flushed) { $flushed = 1; return $Hazard->new(action => 'flush', redirect_pc => 100) }
        return $Hazard->new(action => 'none');
    });
    $p->step();
    is($p->get_pc(), 104, 'PC redirected to 100+4=104 after flush');
};

subtest 'Flush snapshot.flushing is true' => sub {
    my $triggered = 0;
    my $r = $Pipeline->new(
        $Config->classic_5_stage(),
        sub { 0 }, sub { $_[1] }, sub { $_[0] }, sub { $_[0] }, sub { },
    );
    my $p = $r->{pipeline};
    $p->set_hazard_fn(sub {
        if (!$triggered) { $triggered = 1; return $Hazard->new(action => 'flush', redirect_pc => 0) }
        return $Hazard->new(action => 'none');
    });
    my $snap = $p->step();
    is($snap->{flushing}, 1, 'snapshot.flushing=1 during flush');
};

# ============================================================================
# Trace tests
# ============================================================================

subtest 'get_trace returns one snapshot per step' => sub {
    my $p = noop_pipeline();
    $p->step() for 1..5;
    my $trace = $p->get_trace();
    is(scalar(@$trace), 5, '5 snapshots in trace');
};

subtest 'get_trace snapshots have sequential cycle numbers' => sub {
    my $p = noop_pipeline();
    $p->step() for 1..4;
    my $trace = $p->get_trace();
    for my $i (0 .. $#$trace) {
        is($trace->[$i]{cycle}, $i + 1, "trace[$i].cycle = " . ($i + 1));
    }
};

subtest 'snapshot() to_string contains cycle info' => sub {
    my $p    = noop_pipeline();
    my $snap = $p->step();
    like($snap->to_string(), qr/cycle 1/, 'to_string mentions cycle 1');
};

done_testing;
