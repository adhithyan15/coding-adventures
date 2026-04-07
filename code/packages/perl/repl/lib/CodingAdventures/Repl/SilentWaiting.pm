package CodingAdventures::Repl::SilentWaiting;

# ============================================================================
# CodingAdventures::Repl::SilentWaiting — Built-in no-op waiting handler
# ============================================================================
#
# # SilentWaiting: the "do nothing" implementation
#
# SilentWaiting is the default Waiting implementation. It is completely silent:
# it does not print anything, does not animate anything, does not record
# timings. Every method is a no-op.
#
# # When should you use SilentWaiting?
#
# SilentWaiting is appropriate in three scenarios:
#
#   1. TESTING — Tests inject I/O via function references and capture output
#      in string buffers. Any spinner or progress display would pollute the
#      captured output and make assertions harder.
#
#   2. PIPED OUTPUT — When the REPL's output is being piped to a file or
#      another program (e.g., `myrepl < script.txt > output.txt`), terminal
#      animations are meaningless and would corrupt the output stream.
#
#   3. FAST LANGUAGES — If every expression evaluates in under a millisecond,
#      there is nothing to wait for. A spinner that flashes for 0 ms is worse
#      than no spinner at all.
#
# # Design: the Null Object Pattern
#
# SilentWaiting is an example of the "Null Object" design pattern: instead
# of checking `if defined $waiting` everywhere and branching on whether a
# waiting handler was provided, we always require a waiting handler and
# provide SilentWaiting as the safe default.
#
# This eliminates an entire class of null-pointer-style bugs. The calling
# code is simpler because it never has to ask "is there a waiting handler?"
# — there always is one.
#
# Null Object is widely used in Java (NullOutputStream, NullLogger), Go
# (io.Discard, log.New(ioutil.Discard, ...)), and Ruby (Logger::NullLogger).
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# new() → SilentWaiting instance
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# ----------------------------------------------------------------------------
# start() → undef
#
# No-op. Returns undef as the state (there is no state to track).
# ----------------------------------------------------------------------------
sub start {
    return undef;
}

# ----------------------------------------------------------------------------
# tick($state) → $state
#
# No-op. Returns the state unchanged.
# ----------------------------------------------------------------------------
sub tick {
    my ($self, $state) = @_;
    return $state;
}

# ----------------------------------------------------------------------------
# tick_ms() → 100
#
# Return a sensible default poll interval even though we never actually poll.
# 100 ms is 10 times per second — smooth enough for most animations, low
# enough not to burn CPU.
# ----------------------------------------------------------------------------
sub tick_ms {
    return 100;
}

# ----------------------------------------------------------------------------
# stop($state) → void
#
# No-op. Nothing to clean up.
# ----------------------------------------------------------------------------
sub stop {
    my ($self, $state) = @_;
    return;
}

1;

__END__

=head1 NAME

CodingAdventures::Repl::SilentWaiting - No-op waiting handler for the REPL framework

=head1 SYNOPSIS

    use CodingAdventures::Repl::SilentWaiting;

    my $w = CodingAdventures::Repl::SilentWaiting->new();
    my $state = $w->start();
    # ... eval runs ...
    $w->stop($state);

=head1 DESCRIPTION

A Waiting implementation that does absolutely nothing. Useful for tests,
piped I/O, and fast expressions that need no progress display.

All methods are no-ops. C<tick_ms()> returns C<100>.

Implements the Null Object pattern — always safe to use as the default.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
