package CodingAdventures::CpuPipeline;
# =============================================================================
# CodingAdventures::CpuPipeline — configurable N-stage CPU instruction pipeline
# =============================================================================
#
# All pipeline types live in a single .pm file, each in its own Perl package.
# This matches the existing repository convention (e.g. Cache.pm).
#
# Packages defined here:
#   CodingAdventures::CpuPipeline::Token           — instruction token
#   CodingAdventures::CpuPipeline::PipelineStage   — stage definition
#   CodingAdventures::CpuPipeline::PipelineConfig  — ordered stage list + width
#   CodingAdventures::CpuPipeline::HazardResponse  — hazard unit output
#   CodingAdventures::CpuPipeline::PipelineStats   — execution counters
#   CodingAdventures::CpuPipeline::Snapshot        — pipeline state at one cycle
#   CodingAdventures::CpuPipeline::Pipeline        — the engine

use strict;
use warnings;

our $VERSION = '0.01';

1;

# =============================================================================

package CodingAdventures::CpuPipeline::Token;
# -----------------------------------------------------------------------------
# A unit of work flowing through the pipeline.
#
# Each token represents one instruction.  It is ISA-agnostic — the ISA decoder
# fills in opcode/rs1/rs2/rd/immediate, but the pipeline only checks control
# flags (is_halt, is_bubble).
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {
        pc              => 0,
        raw_instruction => 0,
        opcode          => '',
        rs1             => -1,
        rs2             => -1,
        rd              => -1,
        immediate       => 0,
        reg_write       => 0,
        mem_read        => 0,
        mem_write       => 0,
        is_branch       => 0,
        is_halt         => 0,
        alu_result      => 0,
        mem_data        => 0,
        write_data      => 0,
        branch_taken    => 0,
        branch_target   => 0,
        is_bubble       => 0,
        stage_entered   => {},   # {stage_name -> cycle}
        forwarded_from  => '',
    }, $class;
}

sub new_bubble {
    my ($class) = @_;
    my $self = $class->new();
    $self->{is_bubble} = 1;
    $self->{opcode}    = 'BUBBLE';
    return $self;
}

sub to_string {
    my ($self) = @_;
    if ($self->{is_bubble}) {
        return sprintf 'Token[BUBBLE pc=0x%04X]', $self->{pc};
    }
    return sprintf 'Token[pc=0x%04X op=%s rd=%d rs1=%d rs2=%d halt=%s]',
        $self->{pc},
        ($self->{opcode} eq '' ? '?' : $self->{opcode}),
        $self->{rd}, $self->{rs1}, $self->{rs2},
        ($self->{is_halt} ? 'T' : 'F');
}

sub clone {
    my ($class_or_tok) = @_;
    # Can be called as Token->clone($tok) or $tok->clone()
    my $tok = ref($class_or_tok) ? $class_or_tok : $_[1];
    return undef unless defined $tok;
    my %copy = %$tok;
    $copy{stage_entered} = { %{ $tok->{stage_entered} } };
    return bless \%copy, ref($tok);
}

1;

# =============================================================================

package CodingAdventures::CpuPipeline::PipelineStage;
# -----------------------------------------------------------------------------
# Definition of a single pipeline stage.
# category: 'fetch' | 'decode' | 'execute' | 'memory' | 'writeback'
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class, $name, $description, $category) = @_;
    $category //= 'execute';
    return bless {
        name        => $name,
        description => $description // '',
        category    => $category,
    }, $class;
}

1;

# =============================================================================

package CodingAdventures::CpuPipeline::PipelineConfig;
# -----------------------------------------------------------------------------
# The full pipeline configuration: ordered list of PipelineStage objects
# and execution width (1 = scalar).
# -----------------------------------------------------------------------------

use strict;
use warnings;

my @VALID_CATEGORIES = qw(fetch decode execute memory writeback);

sub new {
    my ($class, $stages, $execution_width) = @_;
    $execution_width //= 1;
    return bless {
        stages          => $stages // [],
        execution_width => $execution_width,
    }, $class;
}

sub validate {
    my ($class_or_self) = @_;
    my $self = ref($class_or_self) ? $class_or_self : $_[1];
    return (0, 'config is nil') unless defined $self;
    my $stages = $self->{stages};
    return (0, 'stages must be a non-empty list')
        unless ref $stages eq 'ARRAY' && @$stages;
    my %valid = map { $_ => 1 } @VALID_CATEGORIES;
    for my $i (0 .. $#$stages) {
        my $s = $stages->[$i];
        return (0, "stage $i has no name")
            unless defined $s->{name} && length $s->{name};
        return (0, "stage '$s->{name}' has unknown category '$s->{category}'")
            unless $valid{ $s->{category} // '' };
    }
    return (0, 'execution_width must be >= 1')
        unless defined $self->{execution_width} && $self->{execution_width} >= 1;
    return (1, undef);
}

sub num_stages {
    my ($self) = @_;
    return scalar @{ $self->{stages} };
}

sub classic_5_stage {
    my ($class) = @_;
    my $PS = 'CodingAdventures::CpuPipeline::PipelineStage';
    return $class->new([
        $PS->new('IF',  'Instruction Fetch',  'fetch'),
        $PS->new('ID',  'Instruction Decode', 'decode'),
        $PS->new('EX',  'Execute',            'execute'),
        $PS->new('MEM', 'Memory Access',      'memory'),
        $PS->new('WB',  'Write Back',         'writeback'),
    ], 1);
}

sub deep_13_stage {
    my ($class) = @_;
    my $PS = 'CodingAdventures::CpuPipeline::PipelineStage';
    return $class->new([
        $PS->new('IF1',  'Fetch 1 - TLB lookup',       'fetch'),
        $PS->new('IF2',  'Fetch 2 - cache read',        'fetch'),
        $PS->new('IF3',  'Fetch 3 - align/buffer',      'fetch'),
        $PS->new('ID1',  'Decode 1 - pre-decode',       'decode'),
        $PS->new('ID2',  'Decode 2 - full decode',      'decode'),
        $PS->new('ID3',  'Decode 3 - register read',    'decode'),
        $PS->new('EX1',  'Execute 1 - ALU',             'execute'),
        $PS->new('EX2',  'Execute 2 - shift/multiply',  'execute'),
        $PS->new('EX3',  'Execute 3 - result select',   'execute'),
        $PS->new('MEM1', 'Memory 1 - address calc',     'memory'),
        $PS->new('MEM2', 'Memory 2 - cache access',     'memory'),
        $PS->new('MEM3', 'Memory 3 - data align',       'memory'),
        $PS->new('WB',   'Write Back',                  'writeback'),
    ], 1);
}

1;

# =============================================================================

package CodingAdventures::CpuPipeline::HazardResponse;
# -----------------------------------------------------------------------------
# What the hazard detection callback returns to the pipeline.
# action: 'none' | 'stall' | 'flush' | 'forward_from_ex' | 'forward_from_mem'
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        action         => $opts{action}         // 'none',
        stall_stages   => $opts{stall_stages}   // 0,
        flush_count    => $opts{flush_count}    // 0,
        redirect_pc    => $opts{redirect_pc}    // 0,
        forward_value  => $opts{forward_value}  // 0,
        forward_source => $opts{forward_source} // '',
    }, $class;
}

1;

# =============================================================================

package CodingAdventures::CpuPipeline::PipelineStats;
# -----------------------------------------------------------------------------
# Execution counters.  ipc() and cpi() are computed on demand.
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {
        total_cycles           => 0,
        instructions_completed => 0,
        stall_cycles           => 0,
        flush_cycles           => 0,
        bubble_cycles          => 0,
    }, $class;
}

sub ipc {
    my ($self) = @_;
    return 0.0 unless $self->{total_cycles};
    return $self->{instructions_completed} / $self->{total_cycles};
}

sub cpi {
    my ($self) = @_;
    return 0.0 unless $self->{instructions_completed};
    return $self->{total_cycles} / $self->{instructions_completed};
}

sub to_string {
    my ($self) = @_;
    return sprintf 'PipelineStats{cycles=%d instr=%d stalls=%d flushes=%d ipc=%.3f}',
        $self->{total_cycles}, $self->{instructions_completed},
        $self->{stall_cycles}, $self->{flush_cycles}, $self->ipc();
}

1;

# =============================================================================

package CodingAdventures::CpuPipeline::Snapshot;
# -----------------------------------------------------------------------------
# Complete pipeline state at one clock cycle.
# stages is a hashref {stage_name => Token or undef}
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        cycle    => $opts{cycle}    // 0,
        stages   => $opts{stages}   // {},
        stalled  => $opts{stalled}  // 0,
        flushing => $opts{flushing} // 0,
        pc       => $opts{pc}       // 0,
    }, $class;
}

1;

# =============================================================================

package CodingAdventures::CpuPipeline::Pipeline;
# -----------------------------------------------------------------------------
# The N-stage pipeline engine.
#
# new() returns a hashref {ok=>1, pipeline=>$self} or {ok=>0, err=>"reason"}.
# step() returns a Snapshot.
# run($max_cycles) returns a PipelineStats.
# -----------------------------------------------------------------------------

use strict;
use warnings;
use Scalar::Util qw(weaken);

my $Token    = 'CodingAdventures::CpuPipeline::Token';
my $Config   = 'CodingAdventures::CpuPipeline::PipelineConfig';
my $Hazard   = 'CodingAdventures::CpuPipeline::HazardResponse';
my $Stats    = 'CodingAdventures::CpuPipeline::PipelineStats';
my $Snap     = 'CodingAdventures::CpuPipeline::Snapshot';

sub new {
    my ($class, $config, $fetch_fn, $decode_fn, $execute_fn, $memory_fn, $writeback_fn) = @_;
    my ($ok, $err) = $Config->validate($config);
    return { ok => 0, err => $err } unless $ok;

    my $n = $config->num_stages();
    my @stages = (undef) x $n;

    my $self = bless {
        config       => $config,
        stages       => \@stages,
        pc           => 0,
        cycle        => 0,
        halted       => 0,
        stats        => $Stats->new(),
        history      => [],      # arrayref of Snapshot (chronological)
        fetch_fn     => $fetch_fn,
        decode_fn    => $decode_fn,
        execute_fn   => $execute_fn,
        memory_fn    => $memory_fn,
        writeback_fn => $writeback_fn,
        hazard_fn    => undef,
        predict_fn   => undef,
    }, $class;

    return { ok => 1, pipeline => $self };
}

sub set_hazard_fn  { $_[0]->{hazard_fn}  = $_[1] }
sub set_predict_fn { $_[0]->{predict_fn} = $_[1] }
sub set_pc         { $_[0]->{pc}         = $_[1] }
sub get_pc         { $_[0]->{pc} }
sub is_halted      { $_[0]->{halted} }
sub get_cycle      { $_[0]->{cycle} }
sub get_stats      { $_[0]->{stats} }
sub get_config     { $_[0]->{config} }

sub get_trace {
    my ($self) = @_;
    return [ @{ $self->{history} } ];
}

sub snapshot {
    my ($self) = @_;
    return $self->_build_snapshot(0, 0);
}

# ---------------------------------------------------------------------------
# step() — one clock cycle
# ---------------------------------------------------------------------------
sub step {
    my ($self) = @_;
    return $self->snapshot() if $self->{halted};

    $self->{cycle}++;
    $self->{stats}{total_cycles}++;

    my $n      = $self->{config}->num_stages();
    my $config = $self->{config};

    # Phase 1: hazard detection
    my $hazard;
    if ($self->{hazard_fn}) {
        $hazard = $self->{hazard_fn}->($self->{stages});
    } else {
        $hazard = $Hazard->new(action => 'none');
    }

    # Phase 2 & 3: compute and commit next state
    my ($stalled, $flushing) = (0, 0);
    my $action = $hazard->{action};

    if ($action eq 'flush') {
        $flushing = 1;
        $self->_apply_flush($hazard, $n);
    } elsif ($action eq 'stall') {
        $stalled = 1;
        $self->_apply_stall($hazard, $n);
    } else {
        if ($action eq 'forward_from_ex' || $action eq 'forward_from_mem') {
            $self->_apply_forwarding($hazard);
        }
        $self->_shift_stages($n);
    }

    # Phase 4: fire stage callbacks
    for my $i (reverse 0 .. $n - 1) {
        my $tok   = $self->{stages}[$i];
        next unless defined $tok && !$tok->{is_bubble};
        my $stage = $config->{stages}[$i];

        # Record when this token entered this stage
        $tok->{stage_entered}{$stage->{name}} //= $self->{cycle};

        my $cat = $stage->{category};
        if ($cat eq 'decode') {
            if ($tok->{opcode} eq '') {
                my $decoded = $self->{decode_fn}->($tok->{raw_instruction}, $tok);
                $self->{stages}[$i] = $decoded;
            }
        } elsif ($cat eq 'execute') {
            if (($tok->{stage_entered}{$stage->{name}} // 0) == $self->{cycle}) {
                my $executed = $self->{execute_fn}->($tok);
                $self->{stages}[$i] = $executed;
            }
        } elsif ($cat eq 'memory') {
            if (($tok->{stage_entered}{$stage->{name}} // 0) == $self->{cycle}) {
                my $result = $self->{memory_fn}->($tok);
                $self->{stages}[$i] = $result;
            }
        }
    }

    # Phase 5: retire last stage (writeback)
    my $last_tok = $self->{stages}[$n - 1];
    if (defined $last_tok && !$last_tok->{is_bubble}) {
        $self->{writeback_fn}->($last_tok);
        $self->{stats}{instructions_completed}++;
        $self->{halted} = 1 if $last_tok->{is_halt};
    }

    # Count bubble cycles
    for my $tok (@{ $self->{stages} }) {
        $self->{stats}{bubble_cycles}++ if defined $tok && $tok->{is_bubble};
    }

    # Phase 6: snapshot
    my $snap = $self->_build_snapshot($stalled, $flushing);
    push @{ $self->{history} }, $snap;
    return $snap;
}

sub run {
    my ($self, $max_cycles) = @_;
    $max_cycles //= 10_000;
    while (!$self->{halted} && $self->{cycle} < $max_cycles) {
        $self->step();
    }
    return $self->{stats};
}

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

sub _fetch_instruction {
    my ($self) = @_;
    my $tok = $Token->new();
    $tok->{pc}              = $self->{pc};
    $tok->{raw_instruction} = $self->{fetch_fn}->($self->{pc});
    $tok->{stage_entered}{ $self->{config}{stages}[0]{name} } = $self->{cycle};
    return $tok;
}

sub _advance_pc {
    my ($self) = @_;
    if ($self->{predict_fn}) {
        $self->{pc} = $self->{predict_fn}->($self->{pc});
    } else {
        $self->{pc} += 4;
    }
}

sub _determine_flush_count {
    my ($self, $hazard, $n) = @_;
    return [List::Util::min($hazard->{flush_count}, $n)]->[0]
        if $hazard->{flush_count} > 0;
    my $stages = $self->{config}{stages};
    for my $i (0 .. $#$stages) {
        if ($stages->[$i]{category} eq 'execute') {
            return $i > 0 ? ($i < $n ? $i : $n) : 1;
        }
    }
    return 1;
}

sub _determine_stall_point {
    my ($self, $hazard, $n) = @_;
    if ($hazard->{stall_stages} > 0) {
        my $v = $hazard->{stall_stages};
        return $v < $n ? $v : $n;
    }
    my $stages = $self->{config}{stages};
    for my $i (0 .. $#$stages) {
        if ($stages->[$i]{category} eq 'execute') {
            my $pt = $i + 1;   # 1-based insertion point
            return $pt < $n ? $pt : $n - 1;
        }
    }
    return $n > 1 ? 1 : 0;
}

sub _apply_flush {
    my ($self, $hazard, $n) = @_;
    $self->{stats}{flush_cycles}++;
    my $flush_count = $self->_determine_flush_count($hazard, $n);
    my @next;

    for my $i (0 .. $n - 1) {
        if ($i < $flush_count) {
            my $b = $Token->new_bubble();
            $b->{stage_entered}{ $self->{config}{stages}[$i]{name} } = $self->{cycle};
            $next[$i] = $b;
        } elsif ($i > $flush_count) {
            $next[$i] = $self->{stages}[$i - 1];
        } else {
            my $b = $Token->new_bubble();
            $b->{stage_entered}{ $self->{config}{stages}[$i]{name} } = $self->{cycle};
            $next[$i] = $b;
        }
    }

    $self->{pc} = $hazard->{redirect_pc};
    my $tok = $self->_fetch_instruction();
    $next[0] = $tok;
    $self->_advance_pc();
    $self->{stages} = \@next;
}

sub _apply_stall {
    my ($self, $hazard, $n) = @_;
    $self->{stats}{stall_cycles}++;
    my $sp = $self->_determine_stall_point($hazard, $n);
    my @next;

    for my $i (0 .. $n - 1) {
        if ($i > $sp) {
            $next[$i] = $self->{stages}[$i - 1];
        } elsif ($i == $sp) {
            my $b = $Token->new_bubble();
            $b->{stage_entered}{ $self->{config}{stages}[$i]{name} } = $self->{cycle};
            $next[$i] = $b;
        } else {
            $next[$i] = $self->{stages}[$i];
        }
    }
    $self->{stages} = \@next;
}

sub _apply_forwarding {
    my ($self, $hazard) = @_;
    my $stages = $self->{config}{stages};
    for my $i (0 .. $#$stages) {
        my $tok = $self->{stages}[$i];
        next unless defined $tok && !$tok->{is_bubble};
        if ($stages->[$i]{category} eq 'decode') {
            $tok->{alu_result}     = $hazard->{forward_value};
            $tok->{forwarded_from} = $hazard->{forward_source};
        }
    }
}

sub _shift_stages {
    my ($self, $n) = @_;
    my @next = (undef) x $n;
    for my $i (1 .. $n - 1) {
        $next[$i] = $self->{stages}[$i - 1];
    }
    my $tok = $self->_fetch_instruction();
    $next[0] = $tok;
    $self->_advance_pc();
    $self->{stages} = \@next;
}

sub _build_snapshot {
    my ($self, $stalled, $flushing) = @_;
    my %stage_map;
    my $stages = $self->{config}{stages};
    for my $i (0 .. $#$stages) {
        my $tok = $self->{stages}[$i];
        if (defined $tok) {
            $stage_map{ $stages->[$i]{name} } = $Token->clone($tok);
        }
    }
    return $Snap->new(
        cycle    => $self->{cycle},
        stages   => \%stage_map,
        stalled  => $stalled,
        flushing => $flushing,
        pc       => $self->{pc},
    );
}

1;
