use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Clock;

# ============================================================================
# Tests for CodingAdventures::Clock
# ============================================================================
#
# We test the four components:
#   1. ClockEdge: construction and accessors
#   2. Clock:     tick, full_cycle, run, listeners, reset, period_ns
#   3. ClockDivider: frequency division
#   4. MultiPhaseClock: non-overlapping phase generation

# ============================================================================
# ClockEdge tests
# ============================================================================

subtest 'ClockEdge: construction and accessors' => sub {
    my $edge = CodingAdventures::Clock::ClockEdge->new(1, 1, 1, 0);
    ok(defined $edge, 'ClockEdge object created');
    is($edge->cycle,      1,  'cycle == 1');
    is($edge->value,      1,  'value == 1');
    is($edge->is_rising,  1,  'is_rising == 1');
    is($edge->is_falling, 0,  'is_falling == 0');
};

subtest 'ClockEdge: falling edge record' => sub {
    my $edge = CodingAdventures::Clock::ClockEdge->new(3, 0, 0, 1);
    is($edge->cycle,      3,  'cycle == 3');
    is($edge->value,      0,  'value == 0');
    is($edge->is_rising,  0,  'is_rising == 0');
    is($edge->is_falling, 1,  'is_falling == 1');
};

subtest 'ClockEdge: cycle 0 is valid (before first tick)' => sub {
    my $edge = CodingAdventures::Clock::ClockEdge->new(0, 0, 0, 0);
    is($edge->cycle, 0, 'cycle 0 is valid');
};

# ============================================================================
# Clock: initialization
# ============================================================================

subtest 'Clock: initial state' => sub {
    my $clk = CodingAdventures::Clock->new(1_000_000);
    is($clk->frequency_hz, 1_000_000, 'frequency_hz set correctly');
    is($clk->cycle,        0,         'initial cycle == 0');
    is($clk->value,        0,         'initial value == 0');
    is($clk->total_ticks,  0,         'initial total_ticks == 0');
};

# ============================================================================
# Clock: tick behavior
# ============================================================================

subtest 'Clock: first tick is rising edge' => sub {
    my $clk  = CodingAdventures::Clock->new(1_000_000);
    my $edge = $clk->tick;
    is($edge->is_rising,  1, 'first tick is rising');
    is($edge->is_falling, 0, 'first tick is not falling');
    is($edge->value,      1, 'value is 1 after rising edge');
    is($edge->cycle,      1, 'cycle becomes 1 on first rising edge');
    is($clk->total_ticks, 1, 'total_ticks == 1');
};

subtest 'Clock: second tick is falling edge' => sub {
    my $clk = CodingAdventures::Clock->new(1_000_000);
    $clk->tick;  # rising
    my $edge = $clk->tick;  # falling
    is($edge->is_rising,  0, 'second tick not rising');
    is($edge->is_falling, 1, 'second tick is falling');
    is($edge->value,      0, 'value is 0 after falling edge');
    is($edge->cycle,      1, 'still cycle 1 (falling belongs to same cycle)');
    is($clk->total_ticks, 2, 'total_ticks == 2');
};

subtest 'Clock: cycle count increments only on rising edges' => sub {
    my $clk = CodingAdventures::Clock->new(100);
    $clk->full_cycle for 1..5;
    is($clk->cycle,       5,  'after 5 full cycles, cycle == 5');
    is($clk->total_ticks, 10, 'after 5 full cycles, total_ticks == 10');
};

# ============================================================================
# Clock: full_cycle and run
# ============================================================================

subtest 'Clock: full_cycle returns rising then falling' => sub {
    my $clk    = CodingAdventures::Clock->new(1_000_000);
    my ($r, $f) = $clk->full_cycle;
    is($r->is_rising,  1, 'first return is rising edge');
    is($f->is_falling, 1, 'second return is falling edge');
    is($r->cycle, $f->cycle, 'both edges share the same cycle number');
};

subtest 'Clock: run(N) returns 2N edges' => sub {
    my $clk   = CodingAdventures::Clock->new(1_000_000);
    my $edges = $clk->run(5);
    is(scalar @$edges, 10, 'run(5) produces 10 edges');
    is($edges->[0]->is_rising,  1, 'edges[0] is rising');
    is($edges->[1]->is_falling, 1, 'edges[1] is falling');
    is($edges->[8]->is_rising,  1, 'edges[8] is rising (start of cycle 5)');
    is($edges->[9]->is_falling, 1, 'edges[9] is falling (end of cycle 5)');
};

# ============================================================================
# Clock: listeners (observer pattern)
# ============================================================================

subtest 'Clock: listener_count starts at 0' => sub {
    my $clk = CodingAdventures::Clock->new(1_000_000);
    is($clk->listener_count, 0, 'no listeners initially');
};

subtest 'Clock: register_listener increases count' => sub {
    my $clk = CodingAdventures::Clock->new(1_000_000);
    $clk->register_listener(sub {});
    is($clk->listener_count, 1, 'one listener after first register');
    $clk->register_listener(sub {});
    is($clk->listener_count, 2, 'two listeners after second register');
};

subtest 'Clock: listener receives ClockEdge on tick' => sub {
    my $clk = CodingAdventures::Clock->new(1_000_000);
    my @received;
    $clk->register_listener(sub { push @received, $_[0] });
    $clk->tick;
    $clk->tick;
    is(scalar @received, 2, 'listener called twice (once per tick)');
    is($received[0]->is_rising,  1, 'first edge delivered is rising');
    is($received[1]->is_falling, 1, 'second edge delivered is falling');
};

subtest 'Clock: unregister_listener removes listener' => sub {
    my $clk = CodingAdventures::Clock->new(1_000_000);
    my @called;
    $clk->register_listener(sub { push @called, 'A' });
    $clk->register_listener(sub { push @called, 'B' });
    $clk->tick;
    is(scalar @called, 2, 'both listeners called before unregister');
    @called = ();
    $clk->unregister_listener(1);  # remove listener at index 1
    $clk->tick;
    is(scalar @called, 1, 'only one listener called after unregister');
    is($called[0], 'B', 'surviving listener is B');
};

# ============================================================================
# Clock: reset and period_ns
# ============================================================================

subtest 'Clock: reset restores initial state (listeners preserved)' => sub {
    my $clk = CodingAdventures::Clock->new(1_000_000);
    $clk->register_listener(sub {});
    $clk->run(10);
    $clk->reset;
    is($clk->cycle,        0, 'cycle reset to 0');
    is($clk->value,        0, 'value reset to 0');
    is($clk->total_ticks,  0, 'total_ticks reset to 0');
    is($clk->listener_count, 1, 'listener preserved after reset');
};

subtest 'Clock: period_ns = 1e9 / frequency_hz' => sub {
    my $clk1 = CodingAdventures::Clock->new(1_000_000_000);   # 1 GHz
    ok(abs($clk1->period_ns - 1.0) < 0.001, '1 GHz clock has 1 ns period');

    my $clk2 = CodingAdventures::Clock->new(1_000_000);       # 1 MHz
    ok(abs($clk2->period_ns - 1000.0) < 0.001, '1 MHz clock has 1000 ns period');

    my $clk3 = CodingAdventures::Clock->new(1_000);           # 1 kHz
    ok(abs($clk3->period_ns - 1_000_000.0) < 0.001, '1 kHz clock has 1e6 ns period');
};

# ============================================================================
# ClockDivider tests
# ============================================================================

subtest 'ClockDivider: basic divide-by-4' => sub {
    my $master = CodingAdventures::Clock->new(1_000_000);
    my $div    = CodingAdventures::Clock::ClockDivider->new($master, 4);
    is($div->divisor, 4, 'divisor stored correctly');
    # 4 full source cycles → 1 output cycle
    $master->run(4);
    is($div->output->cycle, 1, 'output clock completes 1 cycle after 4 source cycles');
};

subtest 'ClockDivider: output frequency = source / divisor' => sub {
    my $master = CodingAdventures::Clock->new(1_000_000);
    my $div    = CodingAdventures::Clock::ClockDivider->new($master, 4);
    is($div->output->frequency_hz, 250_000, 'output is 250 kHz (1 MHz / 4)');
};

subtest 'ClockDivider: divide-by-2' => sub {
    my $master = CodingAdventures::Clock->new(100);
    my $div    = CodingAdventures::Clock::ClockDivider->new($master, 2);
    $master->run(10);
    is($div->output->cycle, 5, 'divide-by-2: 10 source cycles → 5 output cycles');
};

# ============================================================================
# MultiPhaseClock tests
# ============================================================================

subtest 'MultiPhaseClock: all phases start at 0' => sub {
    my $master = CodingAdventures::Clock->new(1_000_000);
    my $mp     = CodingAdventures::Clock::MultiPhaseClock->new($master, 4);
    is($mp->get_phase($_), 0, "phase $_ starts at 0") for 0..3;
};

subtest 'MultiPhaseClock: phase 0 activates on first rising edge' => sub {
    my $master = CodingAdventures::Clock->new(1_000_000);
    my $mp     = CodingAdventures::Clock::MultiPhaseClock->new($master, 4);
    $master->tick;  # rising edge → phase 0 activates
    is($mp->get_phase(0), 1, 'phase 0 is active');
    is($mp->get_phase(1), 0, 'phase 1 is inactive');
    is($mp->get_phase(2), 0, 'phase 2 is inactive');
    is($mp->get_phase(3), 0, 'phase 3 is inactive');
};

subtest 'MultiPhaseClock: phases rotate on successive rising edges' => sub {
    my $master = CodingAdventures::Clock->new(1_000_000);
    my $mp     = CodingAdventures::Clock::MultiPhaseClock->new($master, 4);

    $master->tick;  # rising → phase 0
    is($mp->get_phase(0), 1, '1st rising: phase 0 active');

    $master->tick;  # falling
    $master->tick;  # rising → phase 1
    is($mp->get_phase(1), 1, '2nd rising: phase 1 active');
    is($mp->get_phase(0), 0, '2nd rising: phase 0 inactive');

    $master->tick; $master->tick;  # falling, rising → phase 2
    is($mp->get_phase(2), 1, '3rd rising: phase 2 active');

    $master->tick; $master->tick;  # falling, rising → phase 3
    is($mp->get_phase(3), 1, '4th rising: phase 3 active');
};

subtest 'MultiPhaseClock: wraps around to phase 0 after N phases' => sub {
    my $master = CodingAdventures::Clock->new(1_000_000);
    my $mp     = CodingAdventures::Clock::MultiPhaseClock->new($master, 2);

    $master->tick;  # rising → phase 0
    is($mp->get_phase(0), 1, 'phase 0 on 1st rising');
    $master->tick;  # falling
    $master->tick;  # rising → phase 1
    is($mp->get_phase(1), 1, 'phase 1 on 2nd rising');
    $master->tick;  # falling
    $master->tick;  # rising → back to phase 0
    is($mp->get_phase(0), 1, 'phase 0 again on 3rd rising (wraparound)');
    is($mp->get_phase(1), 0, 'phase 1 inactive after wraparound');
};

done_testing;
