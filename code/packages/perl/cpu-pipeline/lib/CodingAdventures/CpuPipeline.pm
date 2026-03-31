package CodingAdventures::CpuPipeline;

# ============================================================================
# CodingAdventures::CpuPipeline — Configurable N-Stage CPU Pipeline
# ============================================================================
#
# A CPU pipeline allows a processor to overlap the execution of multiple
# instructions — while one instruction is executing, the next is being decoded,
# and the one after is being fetched. This is the same principle as a factory
# assembly line.
#
# ASSEMBLY LINE ANALOGY:
#
#   Imagine a car factory with 5 workstations. A new car chassis enters the
#   line every hour. Each workstation does ONE job and passes the work on:
#
#     Station 1: Weld the frame
#     Station 2: Install the engine
#     Station 3: Attach doors
#     Station 4: Paint
#     Station 5: Final inspection
#
#   While station 3 is attaching doors to Car A, station 2 is installing
#   the engine in Car B, station 1 is welding Car C. Three cars in flight
#   simultaneously, one car completed every hour.
#
# THE CLASSIC 5-STAGE RISC PIPELINE:
#
#   IF  (Instruction Fetch)  — read instruction bits from memory
#   ID  (Instruction Decode) — parse opcode, read register values
#   EX  (Execute)            — run the ALU or compute branch target
#   MEM (Memory Access)      — load from or store to data memory
#   WB  (Write Back)         — write result to the register file
#
#   Cycle:  1    2    3    4    5    6    7    8    9
#   Inst1: [IF] [ID] [EX] [ME] [WB]
#   Inst2:      [IF] [ID] [EX] [ME] [WB]
#   Inst3:           [IF] [ID] [EX] [ME] [WB]
#
# After the pipeline fills (cycle 5+), one instruction completes per cycle.
# This is 5x throughput improvement over a non-pipelined design!
#
# HAZARDS: Sometimes the pipeline must wait (STALL) or undo work (FLUSH):
#
#   Data hazard:    ADD R1, R2, R3 writes R1; next SUB reads R1 before WB
#   Control hazard: BEQ resolves in EX; IF and ID fetched the wrong next instr
#
# This module provides:
#
#   CpuPipeline::Token          — unit of work flowing through stages
#   CpuPipeline::PipelineStage  — stage definition (name, category)
#   CpuPipeline::PipelineConfig — stage configuration with validation
#   CpuPipeline::HazardResponse — stall/flush/forward control signals
#   CpuPipeline::PipelineStats  — IPC, CPI, performance counters
#   CpuPipeline::Snapshot       — point-in-time pipeline state
#   CpuPipeline::Pipeline       — the pipeline engine itself
#
# USAGE:
#
#   use CodingAdventures::CpuPipeline;
#   my $pkg = 'CodingAdventures::CpuPipeline';
#
#   my $config = $pkg->classic_5_stage();
#   my $p = CodingAdventures::CpuPipeline::Pipeline->new(
#       $config,
#       sub { my ($pc) = @_; return 0; },           # fetch
#       sub { my ($r, $t) = @_; return $t; },       # decode
#       sub { my ($t) = @_; return $t; },           # execute
#       sub { my ($t) = @_; return $t; },           # memory
#       sub { my ($t) = @_; },                      # writeback
#   );
#   $p->step() for 1..10;
#   printf "IPC: %.3f\n", $p->stats()->ipc();

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Token — a unit of work flowing through the pipeline
# ============================================================================

package CodingAdventures::CpuPipeline::Token;

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
        stage_entered   => {},
        forwarded_from  => '',
    }, $class;
}

sub new_bubble {
    my ($class) = @_;
    my $t = $class->new();
    $t->{is_bubble} = 1;
    return $t;
}

sub to_string {
    my ($self) = @_;
    return '---' if $self->{is_bubble};
    return "$self->{opcode}\@$self->{pc}" if $self->{opcode} ne '';
    return "instr\@$self->{pc}";
}

sub clone {
    my ($self) = @_;
    return undef unless defined $self;
    my %copy = %$self;
    $copy{stage_entered} = { %{ $self->{stage_entered} } };
    return bless \%copy, ref($self);
}

# ============================================================================
# PipelineStage — definition of one pipeline stage
# ============================================================================

package CodingAdventures::CpuPipeline::PipelineStage;

sub new {
    my ($class, $name, $description, $category) = @_;
    return bless {
        name        => $name,
        description => $description,
        category    => $category // 'execute',
    }, $class;
}

# ============================================================================
# PipelineConfig — the full pipeline configuration
# ============================================================================

package CodingAdventures::CpuPipeline::PipelineConfig;

sub new {
    my ($class, $stages, $execution_width) = @_;
    return bless {
        stages          => $stages          // [],
        execution_width => $execution_width // 1,
    }, $class;
}

sub num_stages { return scalar @{ $_[0]->{stages} } }

# classic_5_stage — the textbook RISC pipeline (IF/ID/EX/MEM/WB)
#
# This pipeline was popularized by the MIPS R2000 in 1985. Every computer
# architecture textbook uses it as the reference design.
sub classic_5_stage {
    my ($class_or_self) = @_;
    my $s = 'CodingAdventures::CpuPipeline::PipelineStage';
    return CodingAdventures::CpuPipeline::PipelineConfig->new([
        $s->new('IF',  'Instruction Fetch',  'fetch'),
        $s->new('ID',  'Instruction Decode', 'decode'),
        $s->new('EX',  'Execute',            'execute'),
        $s->new('MEM', 'Memory Access',      'memory'),
        $s->new('WB',  'Write Back',         'writeback'),
    ], 1);
}

# deep_13_stage — inspired by ARM Cortex-A78 (2020)
#
# Splitting classic stages into sub-stages allows higher clock frequencies.
# Tradeoff: branch misprediction now costs ~10 cycles instead of 2.
sub deep_13_stage {
    my ($class_or_self) = @_;
    my $s = 'CodingAdventures::CpuPipeline::PipelineStage';
    return CodingAdventures::CpuPipeline::PipelineConfig->new([
        $s->new('IF1',  'Fetch 1 - TLB lookup',      'fetch'),
        $s->new('IF2',  'Fetch 2 - cache read',       'fetch'),
        $s->new('IF3',  'Fetch 3 - align/buffer',     'fetch'),
        $s->new('ID1',  'Decode 1 - pre-decode',      'decode'),
        $s->new('ID2',  'Decode 2 - full decode',     'decode'),
        $s->new('ID3',  'Decode 3 - register read',   'decode'),
        $s->new('EX1',  'Execute 1 - ALU',            'execute'),
        $s->new('EX2',  'Execute 2 - shift/multiply', 'execute'),
        $s->new('EX3',  'Execute 3 - result select',  'execute'),
        $s->new('MEM1', 'Memory 1 - address calc',    'memory'),
        $s->new('MEM2', 'Memory 2 - cache access',    'memory'),
        $s->new('MEM3', 'Memory 3 - data align',      'memory'),
        $s->new('WB',   'Write Back',                 'writeback'),
    ], 1);
}

sub validate {
    my ($self) = @_;
    my $n = $self->num_stages();
    if ($n < 2) {
        return (0, "pipeline must have at least 2 stages, got $n");
    }
    if ($self->{execution_width} < 1) {
        return (0, "execution_width must be >= 1");
    }
    # Check unique names
    my %seen;
    for my $stage (@{ $self->{stages} }) {
        if ($seen{$stage->{name}}++) {
            return (0, "duplicate stage name: \"$stage->{name}\"");
        }
    }
    # Check required categories
    my %cats = map { $_->{category} => 1 } @{ $self->{stages} };
    return (0, 'pipeline must have at least one fetch stage')
        unless $cats{fetch};
    return (0, 'pipeline must have at least one writeback stage')
        unless $cats{writeback};
    return (1, undef);
}

# ============================================================================
# HazardResponse — what the hazard detector tells the pipeline to do
# ============================================================================
#
# Priority: flush > stall > forward_from_ex > forward_from_mem > none

package CodingAdventures::CpuPipeline::HazardResponse;

sub new {
    my ($class, %opts) = @_;
    return bless {
        action         => $opts{action}         // 'none',
        forward_value  => $opts{forward_value}  // 0,
        forward_source => $opts{forward_source} // '',
        stall_stages   => $opts{stall_stages}   // 0,
        flush_count    => $opts{flush_count}    // 0,
        redirect_pc    => $opts{redirect_pc}    // 0,
    }, $class;
}

# ============================================================================
# PipelineStats — performance counters
# ============================================================================

package CodingAdventures::CpuPipeline::PipelineStats;

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
    return 0.0 if $self->{total_cycles} == 0;
    return $self->{instructions_completed} / $self->{total_cycles};
}

sub cpi {
    my ($self) = @_;
    return 0.0 if $self->{instructions_completed} == 0;
    return $self->{total_cycles} / $self->{instructions_completed};
}

sub to_string {
    my ($self) = @_;
    return sprintf(
        'PipelineStats{cycles=%d, completed=%d, IPC=%.3f, CPI=%.3f, stalls=%d, flushes=%d, bubbles=%d}',
        $self->{total_cycles}, $self->{instructions_completed},
        $self->ipc(), $self->cpi(),
        $self->{stall_cycles}, $self->{flush_cycles}, $self->{bubble_cycles}
    );
}

# ============================================================================
# Snapshot — point-in-time pipeline state
# ============================================================================

package CodingAdventures::CpuPipeline::Snapshot;

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

sub to_string {
    my ($self) = @_;
    return sprintf('[cycle %d] PC=%d stalled=%d flushing=%d',
        $self->{cycle}, $self->{pc}, $self->{stalled}, $self->{flushing});
}

# ============================================================================
# Pipeline — the configurable N-stage pipeline engine
# ============================================================================
#
# The pipeline is a list of N slots (one per stage). Each slot holds a Token
# (or undef for empty). On each step():
#
#   1. Query hazard callback
#   2. Compute next stage contents (shift/stall/flush)
#   3. Run stage callbacks (decode, execute, memory, writeback)
#   4. Count bubbles, retire last stage
#   5. Record snapshot
#
# All transitions happen "simultaneously" — we compute the full next state
# before committing, matching real hardware clock-edge behavior.

package CodingAdventures::CpuPipeline::Pipeline;

sub new {
    my ($class, $config, $fetch_fn, $decode_fn, $execute_fn, $memory_fn, $writeback_fn) = @_;

    my ($ok, $err) = $config->validate();
    unless ($ok) {
        return { ok => 0, err => $err };
    }

    my @stages = (undef) x $config->num_stages();

    my $self = bless {
        config       => $config,
        stages       => \@stages,
        pc           => 0,
        cycle        => 0,
        halted       => 0,
        stats        => CodingAdventures::CpuPipeline::PipelineStats->new(),
        history      => [],
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
sub get_pc         { return $_[0]->{pc} }
sub is_halted      { return $_[0]->{halted} }
sub get_cycle      { return $_[0]->{cycle} }
sub stats          { return $_[0]->{stats} }
sub get_config     { return $_[0]->{config} }

sub get_trace {
    my ($self) = @_;
    return [ reverse @{ $self->{history} } ];
}

sub stage_contents {
    my ($self, $name) = @_;
    my @stage_defs = @{ $self->{config}{stages} };
    for my $i (0 .. $#stage_defs) {
        if ($stage_defs[$i]{name} eq $name) {
            return $self->{stages}[$i];
        }
    }
    return undef;
}

sub snapshot {
    my ($self) = @_;
    return $self->_take_snapshot();
}

# step() — advance the pipeline by one clock cycle
sub step {
    my ($self) = @_;
    return $self->_take_snapshot() if $self->{halted};

    $self->{cycle}++;
    $self->{stats}{total_cycles}++;

    my $num_stages = $self->{config}->num_stages();

    # Phase 1: Query hazard detector
    # Pass next_preview (post-shift view) so the hazard detector sees what
    # stages will look like after a normal advance — not the pre-shift state.
    my $hazard;
    if ($self->{hazard_fn}) {
        my @next_preview;
        $next_preview[0] = undef;  # stage 1: new instruction not yet fetched
        for my $i (1 .. $num_stages - 1) {
            $next_preview[$i] = $self->{stages}[$i - 1];
        }
        $hazard = $self->{hazard_fn}->(\@next_preview);
    } else {
        $hazard = CodingAdventures::CpuPipeline::HazardResponse->new(action => 'none');
    }

    # Phase 2: Compute next state
    my ($stalled, $flushing) = (0, 0);
    my $action = $hazard->{action};

    if ($action eq 'flush') {
        $self->_apply_flush($hazard, $num_stages);
        $flushing = 1;
    } elsif ($action eq 'stall') {
        $self->_apply_stall($hazard, $num_stages);
        $stalled = 1;
    } else {
        $self->_apply_forwarding($hazard) if $action eq 'forward_from_ex' || $action eq 'forward_from_mem';
        $self->_shift_stages($num_stages);
    }

    # Phase 4: Run stage callbacks
    $self->_execute_stage_callbacks($num_stages);

    # Phase 4b: Count bubble cycles
    for my $tok (@{ $self->{stages} }) {
        $self->{stats}{bubble_cycles}++ if defined $tok && $tok->{is_bubble};
    }

    # Phase 4c: Retire last stage
    $self->_retire_last_stage($num_stages);

    # Phase 5: Snapshot
    my $snap = CodingAdventures::CpuPipeline::Snapshot->new(
        cycle    => $self->{cycle},
        stages   => $self->_build_stage_map(),
        stalled  => $stalled,
        flushing => $flushing,
        pc       => $self->{pc},
    );
    unshift @{ $self->{history} }, $snap;  # newest first

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

# Internal helpers

sub _determine_flush_count {
    my ($self, $hazard, $num_stages) = @_;
    if ($hazard->{flush_count} && $hazard->{flush_count} > 0) {
        return $hazard->{flush_count} < $num_stages ? $hazard->{flush_count} : $num_stages;
    }
    my $idx = 0;
    my @stages = @{ $self->{config}{stages} };
    for my $i (0 .. $#stages) {
        if ($stages[$i]{category} eq 'execute') { $idx = $i; last }
    }
    my $fc = $idx > 0 ? $idx : 1;
    return $fc < $num_stages ? $fc : $num_stages;
}

sub _determine_stall_point {
    my ($self, $hazard, $num_stages) = @_;
    if ($hazard->{stall_stages} && $hazard->{stall_stages} > 0) {
        my $s = $hazard->{stall_stages};
        return $s < $num_stages - 1 ? $s : $num_stages - 1;
    }
    my $idx = 1;
    my @stages = @{ $self->{config}{stages} };
    for my $i (0 .. $#stages) {
        if ($stages[$i]{category} eq 'execute') { $idx = $i; last }
    }
    return $idx < $num_stages - 1 ? $idx : $num_stages - 1;
}

sub _apply_flush {
    my ($self, $hazard, $num_stages) = @_;
    $self->{stats}{flush_cycles}++;

    my $flush_count = $self->_determine_flush_count($hazard, $num_stages);
    my @old   = @{ $self->{stages} };
    my @new;

    for my $i (0 .. $num_stages - 1) {
        if ($i < $flush_count) {
            my $b = CodingAdventures::CpuPipeline::Token->new_bubble();
            $b->{stage_entered}{ $self->{config}{stages}[$i]{name} } = $self->{cycle};
            push @new, $b;
        } elsif ($i == $flush_count) {
            my $b = CodingAdventures::CpuPipeline::Token->new_bubble();
            $b->{stage_entered}{ $self->{config}{stages}[$i]{name} } = $self->{cycle};
            push @new, $b;
        } else {
            push @new, $old[$i - 1];
        }
    }

    $self->{pc} = $hazard->{redirect_pc};
    my $tok = $self->_fetch_new_instruction();
    $new[0] = $tok;
    $self->_advance_pc();
    $self->{stages} = \@new;
}

sub _apply_stall {
    my ($self, $hazard, $num_stages) = @_;
    $self->{stats}{stall_cycles}++;

    my $stall_point = $self->_determine_stall_point($hazard, $num_stages);
    my @old = @{ $self->{stages} };
    my @new;

    for my $i (0 .. $num_stages - 1) {
        if ($i > $stall_point) {
            push @new, $old[$i - 1];
        } elsif ($i == $stall_point) {
            my $b = CodingAdventures::CpuPipeline::Token->new_bubble();
            $b->{stage_entered}{ $self->{config}{stages}[$i]{name} } = $self->{cycle};
            push @new, $b;
        } else {
            push @new, $old[$i];
        }
    }
    $self->{stages} = \@new;
}

sub _apply_forwarding {
    my ($self, $hazard) = @_;
    my @stage_defs = @{ $self->{config}{stages} };
    for my $i (0 .. $#stage_defs) {
        my $tok = $self->{stages}[$i];
        if ($stage_defs[$i]{category} eq 'decode' && defined $tok && !$tok->{is_bubble}) {
            $tok->{alu_result}     = $hazard->{forward_value};
            $tok->{forwarded_from} = $hazard->{forward_source};
        }
    }
}

sub _shift_stages {
    my ($self, $num_stages) = @_;
    my @old = @{ $self->{stages} };
    my @new = (undef) x $num_stages;
    for my $i (1 .. $num_stages - 1) {
        $new[$i] = $old[$i - 1];
    }
    my $tok = $self->_fetch_new_instruction();
    $new[0] = $tok;
    $self->_advance_pc();
    $self->{stages} = \@new;
}

sub _fetch_new_instruction {
    my ($self) = @_;
    my $tok = CodingAdventures::CpuPipeline::Token->new();
    $tok->{pc}              = $self->{pc};
    $tok->{raw_instruction} = $self->{fetch_fn}->($self->{pc});
    my $stage_name = $self->{config}{stages}[0]{name};
    $tok->{stage_entered}{$stage_name} = $self->{cycle};
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

sub _execute_stage_callbacks {
    my ($self, $num_stages) = @_;
    my @stage_defs = @{ $self->{config}{stages} };

    for my $i (reverse 0 .. $num_stages - 1) {
        my $tok = $self->{stages}[$i];
        next unless defined $tok && !$tok->{is_bubble};

        my $stage_def = $stage_defs[$i];
        unless (exists $tok->{stage_entered}{ $stage_def->{name} }) {
            $tok->{stage_entered}{ $stage_def->{name} } = $self->{cycle};
        }

        my $cat = $stage_def->{category};
        if ($cat eq 'fetch') {
            # Already handled in _fetch_new_instruction
        } elsif ($cat eq 'decode') {
            if ($tok->{opcode} eq '') {
                my $decoded = $self->{decode_fn}->($tok->{raw_instruction}, $tok);
                $self->{stages}[$i] = $decoded;
            }
        } elsif ($cat eq 'execute') {
            if (($tok->{stage_entered}{ $stage_def->{name} } // 0) == $self->{cycle}) {
                my $result = $self->{execute_fn}->($tok);
                $self->{stages}[$i] = $result;
            }
        } elsif ($cat eq 'memory') {
            if (($tok->{stage_entered}{ $stage_def->{name} } // 0) == $self->{cycle}) {
                my $result = $self->{memory_fn}->($tok);
                $self->{stages}[$i] = $result;
            }
        }
        # writeback handled in _retire_last_stage
    }
}

sub _retire_last_stage {
    my ($self, $num_stages) = @_;
    my $last_tok = $self->{stages}[$num_stages - 1];
    if (defined $last_tok && !$last_tok->{is_bubble}) {
        $self->{writeback_fn}->($last_tok);
        $self->{stats}{instructions_completed}++;
        $self->{halted} = 1 if $last_tok->{is_halt};
    }
}

sub _take_snapshot {
    my ($self) = @_;
    return CodingAdventures::CpuPipeline::Snapshot->new(
        cycle  => $self->{cycle},
        stages => $self->_build_stage_map(),
        pc     => $self->{pc},
    );
}

sub _build_stage_map {
    my ($self) = @_;
    my %map;
    my @stage_defs = @{ $self->{config}{stages} };
    for my $i (0 .. $#stage_defs) {
        my $tok = $self->{stages}[$i];
        $map{ $stage_defs[$i]{name} } = $tok->clone() if defined $tok;
    }
    return \%map;
}

# ============================================================================
# Re-export convenience methods on the top-level package
# ============================================================================

package CodingAdventures::CpuPipeline;

sub classic_5_stage {
    return CodingAdventures::CpuPipeline::PipelineConfig->classic_5_stage();
}

sub deep_13_stage {
    return CodingAdventures::CpuPipeline::PipelineConfig->deep_13_stage();
}

1;
__END__

=head1 NAME

CodingAdventures::CpuPipeline - Configurable N-stage CPU instruction pipeline

=head1 SYNOPSIS

    use CodingAdventures::CpuPipeline;

    my $config = CodingAdventures::CpuPipeline->classic_5_stage();
    my $result = CodingAdventures::CpuPipeline::Pipeline->new(
        $config,
        sub { 0 },        # fetch
        sub { $_[1] },    # decode
        sub { $_[0] },    # execute
        sub { $_[0] },    # memory
        sub { },          # writeback
    );
    my $p = $result->{pipeline};
    $p->run(10);
    printf "IPC: %.3f\n", $p->stats()->ipc();

=head1 DESCRIPTION

A configurable N-stage CPU instruction pipeline simulator.

=cut
