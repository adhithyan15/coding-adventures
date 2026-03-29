package CodingAdventures::Clock;

# ============================================================================
# CodingAdventures::Clock — Pure-Perl clock/oscillator simulation
# ============================================================================
#
# Every sequential circuit in a computer -- flip-flops, registers, counters,
# CPU pipeline stages -- is driven by a clock signal. The clock is a square
# wave that alternates between 0 and 1:
#
#     +--+  +--+  +--+  +--+
#     |  |  |  |  |  |  |  |
#  ---+  +--+  +--+  +--+  +--
#
# On each rising edge (0->1), flip-flops capture their inputs. This is what
# makes synchronous digital logic work: everything happens in lockstep.
#
# In real hardware:
#   - CPU clock: 3-5 GHz (3-5 billion cycles per second)
#   - GPU clock: 1-2 GHz
#   - Memory (DDR5): 4-8 GHz
#
# A single clock cycle has two halves:
#   Tick 0: value goes 0 -> 1  (RISING EDGE)   <- flip-flops capture here
#   Tick 1: value goes 1 -> 0  (FALLING EDGE)  <- DDR memory uses this too
#
# "DDR" (Double Data Rate) memory uses BOTH edges, which is why DDR5-6400
# actually runs at 3200 MHz but achieves 6400 MT/s (mega-transfers/second).
#
# This package is part of the coding-adventures monorepo, Layer 8.

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# ClockEdge — an immutable record of one clock transition
# ============================================================================
#
# Every time the clock ticks, it produces a ClockEdge. An edge captures:
#   - Which cycle we are in (starts at 1 on first rising edge)
#   - The current signal level after the transition (0 or 1)
#   - Whether this was a rising edge (0->1)
#   - Whether this was a falling edge (1->0)
#
# Think of a logic analyzer trace: it records the signal level at each
# moment and annotates rising vs falling transitions.

package CodingAdventures::Clock::ClockEdge;

# new($cycle, $value, $is_rising, $is_falling)
#
# Creates a new ClockEdge record.
#
# Parameters:
#   $cycle      — which cycle (integer >= 0)
#   $value      — signal level after transition (0 or 1)
#   $is_rising  — boolean: was this a 0->1 transition?
#   $is_falling — boolean: was this a 1->0 transition?
sub new {
    my ($class, $cycle, $value, $is_rising, $is_falling) = @_;

    # Validate: cycle must be a non-negative integer
    die "cycle must be a non-negative integer, got '$cycle'"
        unless defined $cycle && $cycle =~ /^\d+$/;

    # Validate: value must be exactly 0 or 1
    die "value must be 0 or 1, got '$value'"
        unless defined $value && ($value == 0 || $value == 1);

    return bless {
        cycle      => $cycle,
        value      => $value,
        is_rising  => $is_rising  ? 1 : 0,
        is_falling => $is_falling ? 1 : 0,
    }, $class;
}

# Accessors for the edge fields
sub cycle      { $_[0]->{cycle}      }
sub value      { $_[0]->{value}      }
sub is_rising  { $_[0]->{is_rising}  }
sub is_falling { $_[0]->{is_falling} }

# ============================================================================
# Clock — the main square-wave generator
# ============================================================================
#
# The Clock maintains:
#   - A cycle count (increments on each rising edge, so every two ticks)
#   - The current signal value (0 or 1, toggling each tick)
#   - A list of "listeners" — callbacks invoked on every tick
#
# The observer pattern (listeners) lets components react to the clock
# without polling. In real hardware, the clock wire is physically connected
# to every flip-flop's clock input pin. We simulate this by calling each
# listener function after each tick.
#
# Example:
#
#   my $clk = CodingAdventures::Clock->new(1_000_000);  # 1 MHz
#   my $edge = $clk->tick;  # $edge->is_rising == 1, cycle == 1
#   $edge = $clk->tick;     # $edge->is_falling == 1, cycle == 1
#   $edge = $clk->tick;     # $edge->is_rising == 1, cycle == 2

package CodingAdventures::Clock;

# new($frequency_hz)
#
# Creates a new Clock with the given frequency in Hz.
#
# The clock starts at value 0 (low), cycle 0, with no ticks elapsed.
# This is the state of a real oscillator before it starts oscillating.
#
# Parameters:
#   $frequency_hz — clock frequency in Hz (must be a positive integer)
sub new {
    my ($class, $frequency_hz) = @_;

    # Validate: frequency must be a positive integer
    die "frequency_hz must be a positive integer, got '$frequency_hz'"
        unless defined $frequency_hz
            && $frequency_hz =~ /^\d+$/
            && $frequency_hz > 0;

    return bless {
        frequency_hz  => $frequency_hz,
        cycle         => 0,    # Current cycle count (0 before first tick)
        value         => 0,    # Current signal level (starts low)
        _total_ticks  => 0,    # Total half-cycles elapsed
        _listeners    => [],   # Registered edge callbacks
    }, $class;
}

# Accessors for public fields
sub frequency_hz { $_[0]->{frequency_hz} }
sub cycle        { $_[0]->{cycle}        }
sub value        { $_[0]->{value}        }
sub total_ticks  { $_[0]->{_total_ticks} }

# tick()
#
# Advances one half-cycle and returns the ClockEdge that occurred.
#
# The clock alternates like a toggle switch:
#   - If currently 0, goes to 1 (rising edge, new cycle starts)
#   - If currently 1, goes to 0 (falling edge, cycle ends)
#
# After toggling, all registered listeners are called with the edge record.
#
# Returns: a CodingAdventures::Clock::ClockEdge object
sub tick {
    my ($self) = @_;

    # Save the old value so we can determine edge direction.
    # This is the same logic as a toggle flip-flop: Q_next = NOT Q.
    my $old_value = $self->{value};
    $self->{value} = 1 - $self->{value};    # Toggle between 0 and 1
    $self->{_total_ticks}++;

    # Detect edge direction:
    #   Rising  = 0 -> 1 transition
    #   Falling = 1 -> 0 transition
    my $is_rising  = ($old_value == 0 && $self->{value} == 1) ? 1 : 0;
    my $is_falling = ($old_value == 1 && $self->{value} == 0) ? 1 : 0;

    # Cycle count increments on each rising edge.
    # So cycle 1 starts at the first rising edge, cycle 2 at the second, etc.
    # The falling edge belongs to the same cycle as the preceding rising edge.
    if ($is_rising) {
        $self->{cycle}++;
    }

    my $edge = CodingAdventures::Clock::ClockEdge->new(
        $self->{cycle}, $self->{value}, $is_rising, $is_falling
    );

    # Notify all listeners — the observer pattern.
    # In real hardware, the clock signal propagates to every connected
    # component simultaneously. We simulate this sequentially.
    for my $listener (@{ $self->{_listeners} }) {
        $listener->($edge);
    }

    return $edge;
}

# full_cycle()
#
# Executes one complete cycle (rising edge + falling edge).
#
# A full cycle is two ticks:
#   1. Rising edge (0 -> 1): the "active" half
#   2. Falling edge (1 -> 0): the "idle" half
#
# Returns: ($rising_edge, $falling_edge) — two ClockEdge objects
sub full_cycle {
    my ($self) = @_;
    my $rising  = $self->tick;
    my $falling = $self->tick;
    return ($rising, $falling);
}

# run($cycles)
#
# Executes N complete cycles and returns all edges.
#
# Since each cycle has two edges, running N cycles produces 2N edges.
# Useful for simulation: "run the clock for 100 cycles and collect what happened."
#
# Parameters:
#   $cycles — number of complete cycles to run (positive integer)
#
# Returns: arrayref of ClockEdge objects (length = 2 * $cycles)
sub run {
    my ($self, $cycles) = @_;

    die "cycles must be a positive integer, got '$cycles'"
        unless defined $cycles && $cycles =~ /^\d+$/ && $cycles > 0;

    my @edges;
    for (1 .. $cycles) {
        my ($r, $f) = $self->full_cycle;
        push @edges, $r, $f;
    }
    return \@edges;
}

# register_listener($callback)
#
# Adds a function to be called on every clock edge.
#
# In real hardware, this is like connecting a wire from the clock to
# a component's clock input pin. The listener receives a ClockEdge on
# every tick.
#
# Parameters:
#   $callback — a code reference that accepts one ClockEdge argument
sub register_listener {
    my ($self, $callback) = @_;

    die "listener must be a code reference"
        unless ref($callback) eq 'CODE';

    push @{ $self->{_listeners} }, $callback;
    return;
}

# listener_count()
#
# Returns the number of registered listeners.
sub listener_count {
    my ($self) = @_;
    return scalar @{ $self->{_listeners} };
}

# unregister_listener($index)
#
# Removes a previously registered listener by 1-based index.
#
# Parameters:
#   $index — 1-based position in the listener list
sub unregister_listener {
    my ($self, $index) = @_;

    die "index must be a positive integer, got '$index'"
        unless defined $index && $index =~ /^\d+$/ && $index > 0;

    my $count = scalar @{ $self->{_listeners} };
    die "listener index $index out of range [1, $count]"
        if $index > $count;

    # splice removes one element at the 0-based position ($index - 1)
    splice @{ $self->{_listeners} }, $index - 1, 1;
    return 1;
}

# reset()
#
# Restores the clock to its initial state.
#
# Sets value back to 0, cycle count to 0, total ticks to 0.
# Listeners are preserved -- only the timing state is reset.
# This is like hitting the reset button on an oscillator.
sub reset {
    my ($self) = @_;
    $self->{cycle}        = 0;
    $self->{value}        = 0;
    $self->{_total_ticks} = 0;
    return;
}

# period_ns()
#
# Returns the clock period in nanoseconds.
#
# The period is the time for one complete cycle (rising + falling edge).
# For a 1 GHz clock, period = 1 ns. For 1 MHz, period = 1000 ns.
#
# Formula: period_ns = 1_000_000_000 / frequency_hz
#
# Returns: period in nanoseconds (floating point)
sub period_ns {
    my ($self) = @_;
    return 1_000_000_000 / $self->{frequency_hz};
}

# ============================================================================
# ClockDivider — frequency division
# ============================================================================
#
# In hardware, clock dividers generate slower clocks from a fast master clock.
# Example: a 1 GHz CPU clock divided by 4 gives a 250 MHz bus clock.
#
# How it works:
#   - Count rising edges from the source clock
#   - Every $divisor rising edges, generate one full cycle on the output
#
# Real-world uses:
#   - CPU-to-bus clock ratio (CPU at 4 GHz, bus at 1 GHz)
#   - USB clock derivation from system clock
#   - Audio sample rate generation
#
# Example:
#
#   my $master = CodingAdventures::Clock->new(1_000_000);  # 1 MHz
#   my $div    = CodingAdventures::Clock::ClockDivider->new($master, 4);
#   # div->output runs at 250 kHz
#   $master->run(4);   # after 4 rising edges, output has one full cycle

package CodingAdventures::Clock::ClockDivider;

# new($source, $divisor)
#
# Creates a clock divider that produces a slower clock from the source.
#
# The divisor must be >= 2 (dividing by 1 is a no-op and likely a bug).
# The output clock frequency = floor(source.frequency_hz / divisor).
#
# The divider automatically registers as a listener on the source clock.
#
# Parameters:
#   $source  — a CodingAdventures::Clock instance
#   $divisor — division factor (integer >= 2)
sub new {
    my ($class, $source, $divisor) = @_;

    # Duck-type check: source must have a tick() method
    die "source must be a Clock instance (needs tick method)"
        unless ref($source) && $source->can('tick');

    die "divisor must be an integer >= 2, got '$divisor'"
        unless defined $divisor
            && $divisor =~ /^\d+$/
            && $divisor >= 2;

    my $output_freq = int($source->frequency_hz / $divisor);
    # Ensure output frequency is at least 1 Hz
    $output_freq = 1 if $output_freq < 1;

    my $self = bless {
        source   => $source,
        divisor  => $divisor,
        output   => CodingAdventures::Clock->new($output_freq),
        _counter => 0,
    }, $class;

    # Register ourselves as a listener. Every source tick calls our handler.
    $source->register_listener(sub { $self->_on_edge(@_) });

    return $self;
}

# Accessors
sub source  { $_[0]->{source}  }
sub divisor { $_[0]->{divisor} }
sub output  { $_[0]->{output}  }

# _on_edge($edge)
#
# Internal handler called on every source clock edge.
#
# We only count rising edges. When we have counted $divisor rising edges,
# we generate one complete output cycle (rising + falling).
# This is exactly how a hardware frequency divider works.
sub _on_edge {
    my ($self, $edge) = @_;

    if ($edge->is_rising) {
        $self->{_counter}++;
        if ($self->{_counter} >= $self->{divisor}) {
            $self->{_counter} = 0;
            $self->{output}->tick;    # rising edge on output
            $self->{output}->tick;    # falling edge on output
        }
    }
    return;
}

# ============================================================================
# MultiPhaseClock — non-overlapping phase generation
# ============================================================================
#
# Used in CPU pipelines where different stages need offset clocks.
# A 4-phase clock generates 4 non-overlapping clock signals, each
# active for 1/4 of the master cycle.
#
# Timing diagram for a 4-phase clock:
#
#   Source:   _|^|_|^|_|^|_|^|_
#   Phase 0:  _|^|___|___|___|_
#   Phase 1:  _|___|^|___|___|_
#   Phase 2:  _|___|___|^|___|_
#   Phase 3:  _|___|___|___|^|_
#
# On each rising edge of the source, exactly ONE phase is active (1)
# and all others are inactive (0). The active phase rotates round-robin.
#
# Real-world uses:
#   - Classic RISC pipelines (fetch, decode, execute, writeback)
#   - DRAM refresh timing
#   - Multiplexed bus access

package CodingAdventures::Clock::MultiPhaseClock;

# new($source, $phases)
#
# Creates a multi-phase clock from a source clock.
#
# Parameters:
#   $source — a CodingAdventures::Clock instance
#   $phases — number of phases (integer >= 2)
sub new {
    my ($class, $source, $phases) = @_;

    die "source must be a Clock instance (needs tick method)"
        unless ref($source) && $source->can('tick');

    die "phases must be an integer >= 2, got '$phases'"
        unless defined $phases
            && $phases =~ /^\d+$/
            && $phases >= 2;

    my $self = bless {
        source        => $source,
        phases        => $phases,
        active_phase  => 0,                    # 0-based index of the active phase
        _phase_values => [(0) x $phases],      # All phases start inactive (0)
    }, $class;

    # Register as a listener on the source clock.
    $source->register_listener(sub { $self->_on_edge(@_) });

    return $self;
}

# Accessors
sub source       { $_[0]->{source}       }
sub phases       { $_[0]->{phases}       }
sub active_phase { $_[0]->{active_phase} }

# get_phase($index)
#
# Returns the current value (0 or 1) of phase $index.
#
# The phase index is 0-based (0 through phases-1).
#
# Parameters:
#   $index — 0-based phase index
sub get_phase {
    my ($self, $index) = @_;

    die "phase index must be a non-negative integer, got '$index'"
        unless defined $index && $index =~ /^\d+$/;

    die "phase index $index out of range [0, " . ($self->{phases} - 1) . "]"
        if $index >= $self->{phases};

    return $self->{_phase_values}[$index];
}

# _on_edge($edge)
#
# Internal handler called on every source clock edge.
#
# On rising edges only, we rotate the active phase. Exactly one phase
# is high at any time -- the "non-overlapping" property that prevents
# pipeline hazards.
sub _on_edge {
    my ($self, $edge) = @_;

    if ($edge->is_rising) {
        # Reset all phases to 0 (inactive)
        $self->{_phase_values} = [(0) x $self->{phases}];

        # Activate the current phase
        $self->{_phase_values}[$self->{active_phase}] = 1;

        # Rotate to next phase using modular arithmetic
        # This creates the round-robin pattern: 0, 1, 2, ..., N-1, 0, 1, ...
        $self->{active_phase} = ($self->{active_phase} + 1) % $self->{phases};
    }
    return;
}

# ============================================================================
# Module exports
# ============================================================================
#
# Re-export sub-classes through the main namespace for convenience.

package CodingAdventures::Clock;

# These allow users to do:
#   CodingAdventures::Clock::ClockDivider->new(...)
#   CodingAdventures::Clock::MultiPhaseClock->new(...)
# (which already works since they're defined in this file)

1;

__END__

=head1 NAME

CodingAdventures::Clock - Pure-Perl clock/oscillator simulation

=head1 SYNOPSIS

    use CodingAdventures::Clock;

    # Create a 1 MHz clock
    my $clk = CodingAdventures::Clock->new(1_000_000);

    # Tick once (rising edge)
    my $edge = $clk->tick;
    print $edge->is_rising;   # 1
    print $edge->cycle;       # 1
    print $edge->value;       # 1

    # Run 10 complete cycles
    my $edges = $clk->run(10);  # 20 edges total

    # Register a listener
    $clk->register_listener(sub {
        my ($edge) = @_;
        print "tick! cycle=${\$edge->cycle} val=${\$edge->value}\n";
    });

    # Clock divider: divide 1 MHz by 4 to get 250 kHz
    my $div = CodingAdventures::Clock::ClockDivider->new($clk, 4);
    # $div->output is a Clock running at 250 kHz

    # Multi-phase clock: 4 non-overlapping phases
    my $mp = CodingAdventures::Clock::MultiPhaseClock->new($clk, 4);
    $clk->tick;
    print $mp->get_phase(0);  # 1 (phase 0 is active)

=head1 DESCRIPTION

Simulates a digital clock signal at the bit level. Models rising and
falling edges, clock dividers (frequency division), and multi-phase clocks
(non-overlapping phase generation for pipeline stages).

This is Layer 8 of the coding-adventures computing stack, built on top of
logic gates and used by sequential circuits (registers, counters, CPUs).

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
