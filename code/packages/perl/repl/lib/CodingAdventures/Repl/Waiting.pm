package CodingAdventures::Repl::Waiting;

# ============================================================================
# CodingAdventures::Repl::Waiting — The Waiting interface
# ============================================================================
#
# # What is a "Waiting" handler?
#
# When a REPL evaluates a long-running expression, the user stares at a blank
# screen. Without feedback, they have no way to know whether the REPL is
# thinking hard, crashed, or stuck in an infinite loop.
#
# A Waiting object is responsible for providing that feedback. It could:
#
#   * Print a spinning cursor: |, /, -, \, |, /…
#   * Show a progress bar
#   * Print elapsed time
#   * Do nothing at all (for tests or when output is piped)
#
# The framework calls the waiting object around every eval:
#
#   my $state = $waiting->start();
#   # ... eval runs ...
#   $waiting->stop($state);
#
# The framework also calls tick() repeatedly while polling for completion.
# This is relevant in asynchronous implementations; for synchronous ones
# (like ours — see below) the loop still calls tick() in case a future
# subclass wants to render an animation.
#
# # Synchronous vs. Asynchronous Eval
#
# ## Why not use Perl threads for async eval?
#
# Perl threads (C<use threads>) are notionally available, but they come with
# heavy caveats:
#
#   1. Many CPAN modules are not thread-safe (they use global state).
#   2. Perl threads are NOT system threads — each thread gets a FULL COPY of
#      the interpreter. This is expensive for large programs.
#   3. C<Thread::Queue>, which would make inter-thread communication clean,
#      is a core module since 5.12 but its behaviour around non-scalar values
#      (e.g., arrayrefs) varies by Perl version.
#   4. On some platforms (older macOS, some BSDs) thread support is compiled
#      out entirely.
#
# ## Our choice: synchronous eval
#
# We evaluate expressions synchronously, just like Lua's REPL implementation
# in this same codebase. The trade-off is honest:
#
#   ADVANTAGE:   Zero complexity. Works on every platform. No thread bugs.
#   DISADVANTAGE: An infinite loop in user code hangs the REPL.
#
# The user can always press Ctrl-C to send SIGINT, which Perl's default
# signal handler will honour.
#
# ## The Waiting interface design with synchronous eval
#
# Since eval runs synchronously, the interaction is simply:
#
#   $state = $waiting->start();   # start animation / record start time
#   # ... eval() runs; no ticking during eval in sync mode ...
#   $waiting->stop($state);       # stop animation / print elapsed time
#
# The tick() / tick_ms() methods are kept in the interface so that a future
# async subclass can be dropped in without changing the call sites.
#
# # The Interface Contract
#
# Implement four methods:
#
#   start()           → $state   (called before eval; returns opaque state)
#   tick($state)      → $state   (called repeatedly while waiting; may update display)
#   tick_ms()         → integer  (milliseconds between tick() calls)
#   stop($state)      → void     (called after eval; clean up display)
#
# The $state value is whatever your implementation needs to carry between
# calls — a start timestamp, a spinner frame index, etc. Return undef if
# you have no state to track.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# new() → Waiting instance
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# ----------------------------------------------------------------------------
# start() → $state
#
# Called immediately before eval() is invoked.
#
# Use this to record a start timestamp, print an opening animation frame,
# or allocate any resources needed during the wait.
#
# @return   Opaque $state value passed to tick() and stop().
# ----------------------------------------------------------------------------
sub start {
    return undef;
}

# ----------------------------------------------------------------------------
# tick($state) → $state
#
# Called repeatedly while the REPL is waiting for eval to finish.
#
# In synchronous mode, this is called ZERO times (eval blocks the main
# thread). In a future async implementation it would be called every
# tick_ms() milliseconds to update a spinner animation.
#
# @param $state   The value returned by start() (or the previous tick()).
# @return         New state (can be the same object/value, or a new one).
# ----------------------------------------------------------------------------
sub tick {
    my ($self, $state) = @_;
    return $state;
}

# ----------------------------------------------------------------------------
# tick_ms() → integer
#
# How long (in milliseconds) to sleep between tick() calls.
#
# Smaller values give smoother animations but more CPU usage. A spinner only
# needs to update ~8 times per second (125 ms) to look smooth to the eye.
#
# In synchronous mode, this value is informational only and is not actually
# used by the framework.
#
# @return   Milliseconds (integer >= 0).
# ----------------------------------------------------------------------------
sub tick_ms {
    return 100;
}

# ----------------------------------------------------------------------------
# stop($state) → void
#
# Called immediately after eval() returns (successfully or with an error).
#
# Use this to erase the spinner from the terminal, print elapsed time,
# or free resources allocated in start().
#
# @param $state   The value returned by start() or the last tick().
# ----------------------------------------------------------------------------
sub stop {
    my ($self, $state) = @_;
    return;
}

1;

__END__

=head1 NAME

CodingAdventures::Repl::Waiting - Waiting/animation interface for the REPL framework

=head1 SYNOPSIS

    package MyWaiting;
    use parent 'CodingAdventures::Repl::Waiting';

    sub start   { return time() }          # record start time
    sub tick    { my ($self, $t) = @_; print "\r" . (time() - $t) . "s…"; return $t }
    sub tick_ms { return 250 }
    sub stop    { print "\r           \r" } # erase the elapsed counter

=head1 DESCRIPTION

Duck-typing interface. The REPL framework calls C<start()> before evaluating
each expression and C<stop($state)> after.  Implement C<tick($state)> if you
want animation during async evaluation.

=head2 Methods

=over 4

=item C<start()> — called before eval; returns opaque state

=item C<tick($state)> — called during wait; returns new state

=item C<tick_ms()> — milliseconds between ticks

=item C<stop($state)> — called after eval; clean up

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
