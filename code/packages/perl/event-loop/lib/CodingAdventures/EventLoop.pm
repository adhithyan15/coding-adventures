package CodingAdventures::EventLoop;

# ============================================================================
# CodingAdventures::EventLoop — Event emitter and tick-based scheduler
# ============================================================================
#
# An event loop is the heartbeat of any interactive application.  It provides
# two complementary patterns for reacting to things that happen:
#
# ## Pattern 1: Push-Based Event Emitter (Observer Pattern)
#
# Producers emit named events; consumers register handlers.  The two sides
# never need to know about each other — the event loop is the intermediary.
#
#   $loop->on("damage", sub { my $data = shift; $hp -= $data->{amount} });
#   $loop->emit("damage", { amount => 10 });
#
# ## Pattern 2: Pull-Based Tick Scheduler (Game Loop Pattern)
#
# A fixed-timestep loop calls all tick handlers on every "tick", passing
# the elapsed time (delta_time) since the last tick.  This is the standard
# pattern for games, simulations, and any real-time system.
#
#   $loop->on_tick(sub { my $dt = shift; $pos += $vel * $dt });
#   $loop->run(60, 1/60);    # 60 frames at 60 Hz
#
# ## Why One Package for Both?
#
# Real applications mix the two: a game runs a fixed-timestep tick loop
# (pull) while keyboard/mouse input arrives as named events (push).  Keeping
# both in one module provides a unified interface.
#
# ## Usage
#
#   use CodingAdventures::EventLoop;
#
#   my $loop = CodingAdventures::EventLoop->new();
#
#   $loop->on("greet", sub { print "Hello, $_[0]{name}\n" });
#   $loop->once("startup", sub { print "Started\n" });
#
#   $loop->emit("startup", {});
#   $loop->emit("startup", {});   # once handler does NOT fire again
#   $loop->emit("greet",  { name => "World" });
#
#   $loop->on_tick(sub { my $dt = shift; ... });
#   $loop->run(10, 0.016);        # 10 ticks at ~60 Hz
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Constructor
# ============================================================================

=head2 new()

Create a new EventLoop with no handlers registered.

  my $loop = CodingAdventures::EventLoop->new();

=cut

sub new {
    my ($class) = @_;
    return bless {
        # _handlers: hashref of event_name => [ {fn => sub, once => bool}, … ]
        _handlers      => {},
        # _tick_handlers: arrayref of {fn => sub, once => bool}
        _tick_handlers => [],
        elapsed_time   => 0.0,
        tick_count     => 0,
    }, $class;
}

# ============================================================================
# on — Register a persistent event handler
# ============================================================================

=head2 on($event_name, $callback)

Register C<$callback> to be called every time C<$event_name> is emitted.

  $loop->on("click", sub { my $data = shift; ... });

=cut

sub on {
    my ($self, $event_name, $callback) = @_;
    $self->{_handlers}{$event_name} //= [];
    push @{$self->{_handlers}{$event_name}}, { fn => $callback, once => 0 };
}

# ============================================================================
# once — Register a one-shot handler
# ============================================================================

=head2 once($event_name, $callback)

Register C<$callback> to be called exactly once for C<$event_name>,
then automatically removed.

  $loop->once("init", sub { print "Initialised!\n" });

=cut

sub once {
    my ($self, $event_name, $callback) = @_;
    $self->{_handlers}{$event_name} //= [];
    push @{$self->{_handlers}{$event_name}}, { fn => $callback, once => 1 };
}

# ============================================================================
# off — Remove handlers
# ============================================================================

=head2 off($event_name [, $callback])

Remove event handlers.

  $loop->off("click");            # remove ALL handlers for "click"
  $loop->off("click", $handler); # remove only this specific handler

When removing a specific callback, only the first matching occurrence is
removed (same behaviour as JavaScript's removeEventListener).

=cut

sub off {
    my ($self, $event_name, $callback) = @_;
    return unless exists $self->{_handlers}{$event_name};

    if (!defined $callback) {
        # Remove all handlers for this event.
        $self->{_handlers}{$event_name} = [];
    }
    else {
        # Remove the first handler whose fn matches.
        my $handlers = $self->{_handlers}{$event_name};
        for my $i (0..$#$handlers) {
            if ($handlers->[$i]{fn} == $callback) {
                splice @$handlers, $i, 1;
                last;
            }
        }
    }
}

# ============================================================================
# emit — Fire an event
# ============================================================================

=head2 emit($event_name, $data)

Fire all handlers registered for C<$event_name>, passing C<$data> to each.

  $loop->emit("damage", { amount => 10, type => "fire" });

Handlers are called in registration order.  One-shot handlers (registered
with C<once>) are removed before being called, so re-emitting the same
event from within a handler will not trigger them again.

=cut

sub emit {
    my ($self, $event_name, $data) = @_;
    my $handlers = $self->{_handlers}{$event_name};
    return unless $handlers && @$handlers;

    # Snapshot: iterate a copy so that adding/removing handlers during
    # this emit does not affect the current iteration.
    my @snapshot = @$handlers;

    # Remove once-handlers from the live list before calling any of them.
    for my $h (@snapshot) {
        if ($h->{once}) {
            $self->off($event_name, $h->{fn});
        }
    }

    # Call all handlers in the snapshot.
    for my $h (@snapshot) {
        $h->{fn}->($data);
    }
}

# ============================================================================
# on_tick — Register a tick handler
# ============================================================================

=head2 on_tick($callback)

Register C<$callback> to be called on every C<tick()>.

  $loop->on_tick(sub { my $dt = shift; $position += $velocity * $dt });

=cut

sub on_tick {
    my ($self, $callback) = @_;
    push @{$self->{_tick_handlers}}, { fn => $callback, once => 0 };
}

# ============================================================================
# tick — Advance time by one step
# ============================================================================

=head2 tick([$delta_time])

Advance the loop by one time step (default 1.0).  All tick handlers receive
C<$delta_time> as their argument.  Updates C<elapsed_time> and C<tick_count>.

  $loop->tick(1/60);    # one frame at 60 Hz

=cut

sub tick {
    my ($self, $delta_time) = @_;
    $delta_time //= 1.0;

    # Snapshot for safe iteration.
    my @snapshot = @{$self->{_tick_handlers}};
    for my $h (@snapshot) {
        $h->{fn}->($delta_time);
    }

    $self->{elapsed_time} += $delta_time;
    $self->{tick_count}++;
}

# ============================================================================
# run — Execute n_ticks ticks
# ============================================================================

=head2 run([$n_ticks [, $delta_time]])

Run C<$n_ticks> ticks (default 1) each of duration C<$delta_time> (default 1.0).

  $loop->run(60, 1/60);    # simulate one second at 60 Hz

=cut

sub run {
    my ($self, $n_ticks, $delta_time) = @_;
    $n_ticks    //= 1;
    $delta_time //= 1.0;
    for (1..$n_ticks) {
        $self->tick($delta_time);
    }
}

# ============================================================================
# step — Execute one tick
# ============================================================================

=head2 step([$delta_time])

Convenience alias for C<< $loop->run(1, $delta_time) >>.

=cut

sub step {
    my ($self, $delta_time) = @_;
    $self->tick($delta_time // 1.0);
}

1;

__END__

=head1 NAME

CodingAdventures::EventLoop - Event emitter and tick-based scheduler

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::EventLoop;
  my $loop = CodingAdventures::EventLoop->new();
  $loop->on("ev", sub { ... });
  $loop->emit("ev", { value => 42 });
  $loop->on_tick(sub { my $dt = shift; ... });
  $loop->run(10, 0.016);

=head1 LICENSE

MIT
