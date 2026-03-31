package CodingAdventures::Core;

# ============================================================================
# CodingAdventures::Core — CPU Core Integration Point
# ============================================================================
#
# The Core wires together all the CPU architecture sub-components into a
# complete, working processor. It is the integration point — it does not
# define new micro-architectural behavior, it connects the parts.
#
# WHAT THE CORE PROVIDES:
#
#   Pipeline          — manages instruction flow through stages (D04)
#   RegisterFile      — fast CPU operand storage (from cpu-simulator)
#   MemoryController  — backing memory access
#   ISA Decoder       — INJECTED from outside
#
# THE MOTHERBOARD ANALOGY:
#
#   The Core is like a motherboard. The CPU pipeline, caches, register file
#   are all separate components. The "core" is the specific combination:
#
#     Core "Simple":
#       Pipeline:      5-stage (IF, ID, EX, MEM, WB)
#       RegisterFile:  16 registers, 32-bit
#       Memory:        64KB
#
#     Core "Performance":
#       Pipeline:      13-stage (ARM Cortex-A78-inspired)
#       RegisterFile:  31 registers, 64-bit
#       Memory:        64KB
#
# ISA DECODER PROTOCOL:
#
#   The Core is ISA-agnostic. It delegates all instruction semantics to an
#   injected decoder object. The decoder must implement:
#
#     $decoder->decode($raw_instruction, $token)  → $decoded_token
#     $decoder->execute($token, $register_file)   → $result_token
#     $decoder->instruction_size()                → 4  (bytes)
#
#   This means the same Core works with ARM, RISC-V, or any custom ISA.
#
# USAGE:
#
#   my $result = CodingAdventures::Core::Core->new($config, $decoder);
#   die $result->{err} unless $result->{ok};
#   my $core = $result->{core};
#
#   $core->load_program(\@bytes, 0);
#   $core->run(1000);
#   printf "R0 = %d\n", $core->read_register(0);

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# MemoryController — wraps Memory with latency model
# ============================================================================

package CodingAdventures::Core::MemoryController;

use CodingAdventures::CpuSimulator;

sub new {
    my ($class, $size, $latency) = @_;
    $latency //= 100;
    return bless {
        memory  => CodingAdventures::CpuSimulator::Memory->new($size),
        size    => $size,
        latency => $latency,
    }, $class;
}

sub read_word  { return $_[0]->{memory}->read_word($_[1]) }
sub write_word { $_[0]->{memory}->write_word($_[1], $_[2]) }
sub load_program {
    my ($self, $bytes, $start) = @_;
    $self->{memory}->load_bytes($start, $bytes);
}

# ============================================================================
# CoreConfig — micro-architecture parameters
# ============================================================================

package CodingAdventures::Core::CoreConfig;

use CodingAdventures::CpuPipeline;

sub new {
    my ($class, %opts) = @_;
    return bless {
        name            => $opts{name}            // 'Core',
        pipeline_config => $opts{pipeline_config} // CodingAdventures::CpuPipeline::PipelineConfig->classic_5_stage(),
        num_registers   => $opts{num_registers}   // 16,
        register_width  => $opts{register_width}  // 32,
        memory_size     => $opts{memory_size}     // 65536,
        memory_latency  => $opts{memory_latency}  // 100,
    }, $class;
}

# simple() — 5-stage teaching core (equivalent to a 1980s microcontroller)
sub simple {
    return CodingAdventures::Core::CoreConfig->new(
        name            => 'Simple',
        pipeline_config => CodingAdventures::CpuPipeline::PipelineConfig->classic_5_stage(),
        num_registers   => 16,
        register_width  => 32,
        memory_size     => 65536,
        memory_latency  => 100,
    );
}

# performance() — 13-stage core (ARM Cortex-A78-inspired)
sub performance {
    return CodingAdventures::Core::CoreConfig->new(
        name            => 'Performance',
        pipeline_config => CodingAdventures::CpuPipeline::PipelineConfig->deep_13_stage(),
        num_registers   => 31,
        register_width  => 64,
        memory_size     => 65536,
        memory_latency  => 100,
    );
}

# ============================================================================
# CoreStats — execution statistics
# ============================================================================

package CodingAdventures::Core::CoreStats;

sub new {
    return bless {
        total_cycles           => 0,
        instructions_completed => 0,
        stall_cycles           => 0,
        flush_cycles           => 0,
    }, $_[0];
}

sub ipc {
    my ($self) = @_;
    return 0.0 if $self->{total_cycles} == 0;
    return $self->{instructions_completed} / $self->{total_cycles};
}

sub to_string {
    my ($self) = @_;
    return sprintf(
        'CoreStats{cycles=%d, completed=%d, IPC=%.3f, stalls=%d, flushes=%d}',
        $self->{total_cycles}, $self->{instructions_completed},
        $self->ipc(), $self->{stall_cycles}, $self->{flush_cycles}
    );
}

# ============================================================================
# Core — the integration point
# ============================================================================
#
# Internal architecture:
#
#   The Core holds a Pipeline, a RegisterFile, and a MemoryController.
#   The Pipeline's callbacks are closures over the Core's state hashrefs.
#   Since Perl passes hashrefs by reference, callbacks can read/write
#   the register file and memory directly.
#
#   ┌──────────────────────────────────────────────────────────────┐
#   │  Core                                                         │
#   │  ┌───────────┐ fetch_fn  ┌──────────────────────────────┐   │
#   │  │ Memory    │←─────────→│ Pipeline (CpuPipeline::*)    │   │
#   │  │ Controller│ memory_fn │                              │   │
#   │  └───────────┘           │  IF → ID → EX → MEM → WB   │   │
#   │                          └──────────────────────────────┘   │
#   │  ┌───────────┐ writeback_fn          ↑                       │
#   │  │ Register  │←─────────────────────┘                       │
#   │  │ File      │← decode_fn / execute_fn                       │
#   │  └───────────┘           ↑                                   │
#   │  ┌───────────┐           │                                   │
#   │  │ ISA       │───────────┘  (injected)                       │
#   │  │ Decoder   │                                               │
#   │  └───────────┘                                               │
#   └──────────────────────────────────────────────────────────────┘

package CodingAdventures::Core::Core;

use CodingAdventures::CpuPipeline;
use CodingAdventures::CpuSimulator;

sub new {
    my ($class, $config, $decoder) = @_;

    # 1. Register file
    my $reg_file = CodingAdventures::CpuSimulator::RegisterFile->new(
        $config->{num_registers}, $config->{register_width}
    );

    # 2. Memory controller
    my $mem_ctrl = CodingAdventures::Core::MemoryController->new(
        $config->{memory_size}, $config->{memory_latency}
    );

    # 3. Pipeline callbacks (closures over $reg_file and $mem_ctrl by reference)
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

    # 4. Create the pipeline
    my $pipe_result = CodingAdventures::CpuPipeline::Pipeline->new(
        $config->{pipeline_config},
        $fetch_fn, $decode_fn, $execute_fn, $memory_fn, $writeback_fn,
    );

    return { ok => 0, err => $pipe_result->{err} } unless $pipe_result->{ok};

    my $pipeline = $pipe_result->{pipeline};

    # Set predict_fn using decoder's instruction_size
    $pipeline->set_predict_fn(sub {
        my ($pc) = @_;
        return $pc + $decoder->instruction_size();
    });

    my $state = {
        reg_file => $reg_file,
        mem_ctrl => $mem_ctrl,
        cycle    => 0,
        stats    => CodingAdventures::Core::CoreStats->new(),
    };

    my $core = bless {
        config   => $config,
        decoder  => $decoder,
        pipeline => $pipeline,
        state    => $state,
    }, $class;

    return { ok => 1, core => $core };
}

sub load_program {
    my ($self, $bytes, $start_address) = @_;
    $start_address //= 0;
    $self->{state}{mem_ctrl}->load_program($bytes, $start_address);
    $self->{pipeline}->set_pc($start_address);
}

sub step {
    my ($self) = @_;
    my $snap = $self->{pipeline}->step();
    $self->{state}{cycle}++;

    my $ps = $self->{pipeline}->stats();
    my $st = $self->{state}{stats};
    $st->{total_cycles}           = $ps->{total_cycles};
    $st->{instructions_completed} = $ps->{instructions_completed};
    $st->{stall_cycles}           = $ps->{stall_cycles};
    $st->{flush_cycles}           = $ps->{flush_cycles};

    return $snap;
}

sub run {
    my ($self, $max_cycles) = @_;
    $max_cycles //= 10_000;
    while (!$self->{pipeline}->is_halted() && $self->{state}{cycle} < $max_cycles) {
        $self->step();
    }
    return $self->{state}{stats};
}

sub is_halted      { return $_[0]->{pipeline}->is_halted() }
sub get_cycle      { return $_[0]->{state}{cycle} }
sub get_stats      { return $_[0]->{state}{stats} }
sub get_pc         { return $_[0]->{pipeline}->get_pc() }
sub get_trace      { return $_[0]->{pipeline}->get_trace() }

sub read_register  {
    my ($self, $index) = @_;
    return $self->{state}{reg_file}->read($index);
}

sub write_register {
    my ($self, $index, $value) = @_;
    $self->{state}{reg_file}->write($index, $value);
}

sub read_memory_word {
    my ($self, $address) = @_;
    return $self->{state}{mem_ctrl}->read_word($address);
}

sub write_memory_word {
    my ($self, $address, $value) = @_;
    $self->{state}{mem_ctrl}->write_word($address, $value);
}

# ============================================================================
# Top-level package
# ============================================================================

package CodingAdventures::Core;

1;
__END__

=head1 NAME

CodingAdventures::Core - CPU core integrating pipeline, register file, and memory

=head1 SYNOPSIS

    use CodingAdventures::Core;
    use CodingAdventures::CpuPipeline;
    use CodingAdventures::CpuSimulator;

    # Build a minimal NOP decoder
    my $decoder = {
        decode          => sub { my ($r, $t) = @_; $t->{opcode} = 'NOP'; $t },
        execute         => sub { $_[1] },
        instruction_size => sub { 4 },
    };
    bless $decoder, 'MyDecoder';
    sub MyDecoder::decode          { $_[0]->{decode}->($_[1], $_[2]) }
    sub MyDecoder::execute         { $_[0]->{execute}->($_[1], $_[2]) }
    sub MyDecoder::instruction_size { 4 }

    my $result = CodingAdventures::Core::Core->new(
        CodingAdventures::Core::CoreConfig->simple(), $decoder
    );
    my $core = $result->{core};
    $core->load_program([(0) x 64], 0);
    $core->run(10);
    printf "IPC: %.3f\n", $core->get_stats()->ipc();

=cut
