package CodingAdventures::Core;
# =============================================================================
# CodingAdventures::Core — complete CPU core
# =============================================================================
#
# Packages:
#   CodingAdventures::Core::MemoryController — Memory wrapper with latency
#   CodingAdventures::Core::CoreConfig       — micro-architecture parameters
#   CodingAdventures::Core::CoreStats        — aggregate statistics
#   CodingAdventures::Core::Core             — the wired-together processor

use strict;
use warnings;

our $VERSION = '0.01';

1;

# =============================================================================

package CodingAdventures::Core::MemoryController;
# -----------------------------------------------------------------------------
# Thin wrapper around CodingAdventures::CpuSimulator::Memory with a stored
# latency value (for metadata; actual simulation is single-cycle).
# -----------------------------------------------------------------------------

use strict;
use warnings;
use CodingAdventures::CpuSimulator;

sub new {
    my ($class, $size, $latency) = @_;
    $size    //= 65536;
    $latency //= 100;
    return bless {
        memory  => CodingAdventures::CpuSimulator::Memory->new($size),
        latency => $latency,
    }, $class;
}

sub read_word  { $_[0]->{memory}->read_word($_[1]) }
sub write_word { $_[0]->{memory}->write_word($_[1], $_[2]) }

sub load_program {
    my ($self, $bytes, $start) = @_;
    $start //= 0;
    $self->{memory}->load_bytes($start, $bytes);
}

1;

# =============================================================================

package CodingAdventures::Core::CoreConfig;
# -----------------------------------------------------------------------------
# Micro-architecture parameters.
# -----------------------------------------------------------------------------

use strict;
use warnings;
use CodingAdventures::CpuPipeline;

sub new {
    my ($class, %opts) = @_;
    return bless {
        name           => $opts{name}           // 'Default',
        pipeline       => $opts{pipeline}       // CodingAdventures::CpuPipeline::PipelineConfig->classic_5_stage(),
        num_registers  => $opts{num_registers}  // 16,
        register_width => $opts{register_width} // 32,
        memory_size    => $opts{memory_size}    // 65536,
        memory_latency => $opts{memory_latency} // 100,
    }, $class;
}

sub simple {
    my ($class) = @_;
    return $class->new(
        name           => 'Simple',
        pipeline       => CodingAdventures::CpuPipeline::PipelineConfig->classic_5_stage(),
        num_registers  => 16,
        register_width => 32,
        memory_size    => 65536,
        memory_latency => 1,
    );
}

sub performance {
    my ($class) = @_;
    return $class->new(
        name           => 'Performance',
        pipeline       => CodingAdventures::CpuPipeline::PipelineConfig->deep_13_stage(),
        num_registers  => 31,
        register_width => 64,
        memory_size    => 262144,
        memory_latency => 100,
    );
}

1;

# =============================================================================

package CodingAdventures::Core::CoreStats;
# -----------------------------------------------------------------------------
# Aggregate statistics.
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        instructions_completed => $opts{instructions_completed} // 0,
        total_cycles           => $opts{total_cycles}           // 0,
        pipeline_stats         => $opts{pipeline_stats}         // undef,
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
    return sprintf 'CoreStats{instr=%d cycles=%d ipc=%.3f cpi=%.3f}',
        $self->{instructions_completed}, $self->{total_cycles},
        $self->ipc(), $self->cpi();
}

1;

# =============================================================================

package CodingAdventures::Core::Core;
# -----------------------------------------------------------------------------
# The wired-together processor core.
#
# The ISA decoder is injected at construction.  It must implement:
#   $decoder->decode($raw, $token)       → $token
#   $decoder->execute($token, $reg_file) → $token
#   $decoder->instruction_size()         → int
# -----------------------------------------------------------------------------

use strict;
use warnings;
use CodingAdventures::CpuPipeline;
use CodingAdventures::CpuSimulator;

my $Pipeline   = 'CodingAdventures::CpuPipeline::Pipeline';
my $PCfg       = 'CodingAdventures::CpuPipeline::PipelineConfig';
my $MemCtrl    = 'CodingAdventures::Core::MemoryController';
my $RegFile    = 'CodingAdventures::CpuSimulator::RegisterFile';
my $CoreStats  = 'CodingAdventures::Core::CoreStats';

# new($config, $decoder) → {ok=>1, core=>$self} | {ok=>0, err=>"reason"}
sub new {
    my ($class, $config, $decoder) = @_;
    return { ok => 0, err => 'config is nil'  } unless defined $config;
    return { ok => 0, err => 'decoder is nil' } unless defined $decoder;

    # 1. Shared mutable state — captured by reference in all closures
    my $reg_file = $RegFile->new($config->{num_registers}, $config->{register_width});
    my $mem_ctrl = $MemCtrl->new($config->{memory_size}, $config->{memory_latency});

    # 2. Define the five pipeline callbacks.
    #    Each closure captures $reg_file and $mem_ctrl by reference.
    #    Since Perl scalars are references when blessed, this is safe.

    my $fetch_fn = sub {
        my ($pc) = @_;
        return $mem_ctrl->read_word($pc);
    };

    my $decode_fn = sub {
        my ($raw, $token) = @_;
        return $decoder->decode($raw, $token);
    };

    my $execute_fn = sub {
        my ($token) = @_;
        return $decoder->execute($token, $reg_file);
    };

    my $memory_fn = sub {
        my ($token) = @_;
        if ($token->{mem_read}) {
            my $data = $mem_ctrl->read_word($token->{alu_result});
            $token->{mem_data}   = $data;
            $token->{write_data} = $data;
        } elsif ($token->{mem_write}) {
            $mem_ctrl->write_word($token->{alu_result}, $token->{write_data});
        }
        return $token;
    };

    my $writeback_fn = sub {
        my ($token) = @_;
        if ($token->{reg_write} && $token->{rd} >= 0) {
            $reg_file->write($token->{rd}, $token->{write_data});
        }
    };

    # 3. Build the pipeline
    my $pipeline_config = $config->{pipeline} || $PCfg->classic_5_stage();
    my $pipeline_result = $Pipeline->new(
        $pipeline_config,
        $fetch_fn, $decode_fn, $execute_fn, $memory_fn, $writeback_fn,
    );
    return { ok => 0, err => $pipeline_result->{err} } unless $pipeline_result->{ok};

    # 4. Set predict function
    my $p = $pipeline_result->{pipeline};
    my $instr_size = $decoder->instruction_size();
    $p->set_predict_fn(sub { $_[0] + $instr_size });

    # 5. Assemble the Core
    my $self = bless {
        config   => $config,
        decoder  => $decoder,
        pipeline => $p,
        reg_file => $reg_file,
        mem_ctrl => $mem_ctrl,
        cycle    => 0,
        halted   => 0,
    }, $class;

    return { ok => 1, core => $self };
}

sub load_program {
    my ($self, $bytes, $start_address) = @_;
    $start_address //= 0;
    $self->{mem_ctrl}->load_program($bytes, $start_address);
    $self->{pipeline}->set_pc($start_address);
}

sub step {
    my ($self) = @_;
    return $self->{pipeline}->snapshot() if $self->{halted};
    $self->{cycle}++;
    my $snap = $self->{pipeline}->step();
    $self->{halted} = 1 if $self->{pipeline}->is_halted();
    return $snap;
}

sub run {
    my ($self, $max_cycles) = @_;
    $max_cycles //= 100_000;
    while (!$self->{halted} && $self->{cycle} < $max_cycles) {
        $self->step();
    }
    return $self->get_stats();
}

sub is_halted  { $_[0]->{halted} }
sub get_cycle  { $_[0]->{cycle} }

sub get_stats {
    my ($self) = @_;
    my $ps = $self->{pipeline}->get_stats();
    return $CoreStats->new(
        instructions_completed => $ps->{instructions_completed},
        total_cycles           => $ps->{total_cycles},
        pipeline_stats         => $ps,
    );
}

sub read_register    { $_[0]->{reg_file}->read($_[1]) }
sub write_register   { $_[0]->{reg_file}->write($_[1], $_[2]) }
sub read_memory_word  { $_[0]->{mem_ctrl}->read_word($_[1]) }
sub write_memory_word { $_[0]->{mem_ctrl}->write_word($_[1], $_[2]) }
sub get_trace        { $_[0]->{pipeline}->get_trace() }

1;
