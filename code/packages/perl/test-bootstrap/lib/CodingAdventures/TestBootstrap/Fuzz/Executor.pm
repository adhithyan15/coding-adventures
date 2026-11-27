package CodingAdventures::TestBootstrap::Fuzz::Executor;

# One target execution through `Safe::call`, under a one-shot alarm (VER06q section 5).
#
# Perl cannot kill a running function.  What can interrupt Perl code is an exception thrown
# from a SIGNAL HANDLER, and signals are delivered SAFELY: the C handler only sets a flag and
# the Perl handler runs between ops.  So each execution:
#
#   1. gets a fresh Handle (inside-out state, see Handle.pm);
#   2. arms `Time::HiRes::alarm(timeout)` with `local $SIG{ALRM}` = the BUDGET HANDLER, which
#      marks the execution exhausted (first breach wins) and `die`s the steps sentinel, then
#      re-arms a short timer so a target that swallowed the sentinel is interrupted again;
#   3. runs `target(data, handle)` inside `Safe::call` (which classifies by `eval`, never by
#      `$@`), through a NAMED closure that installs a `__DIE__` recorder of the frames at the
#      die site (the error value itself is never touched);
#   4. disarms the alarm, RETIRES the handle, and only then judges the verdict: from the
#      sticky exhausted marker and from the counters against the budgets, so a target that
#      caught the sentinel and carried on is still a timeout or an oom.
#
# What is detected after the run (section 5.3): a replaced or cancelled alarm, a tampered
# engine (Handle.pm's subs), and a fork (the child exits at once).  What cannot be stopped:
# one long op (a huge `x`, a pathological regex), and a target that catches the sentinel in
# an unbounded loop.  Targets are trusted not to attack the harness (PT-F14: an
# out-of-process runner).

use strict;
use warnings;

use POSIX       ();
use Scalar::Util ();
use Sub::Util   ();
use Time::HiRes ();

use CodingAdventures::TestBootstrap::Safe             ();
use CodingAdventures::TestBootstrap::Fuzz::Limits     ();
use CodingAdventures::TestBootstrap::Fuzz::Handle     ();
use CodingAdventures::TestBootstrap::Fuzz::Outcome    ();
use CodingAdventures::TestBootstrap::Fuzz::Signature  ();

# --- captured at load (VER06q section 1)
my $safe_call      = \&CodingAdventures::TestBootstrap::Safe::call;
my $handle_new     = \&CodingAdventures::TestBootstrap::Fuzz::Handle::new;
my $handle_retire  = \&CodingAdventures::TestBootstrap::Fuzz::Handle::retire;
my $handle_breach  = \&CodingAdventures::TestBootstrap::Fuzz::Handle::breach;
my $handle_which   = \&CodingAdventures::TestBootstrap::Fuzz::Handle::exhausted_which;
my $handle_over    = \&CodingAdventures::TestBootstrap::Fuzz::Handle::over_budget;
my $handle_edges   = \&CodingAdventures::TestBootstrap::Fuzz::Handle::edge_list;
my $handle_record  = \&CodingAdventures::TestBootstrap::Fuzz::Handle::sentinel_record;
my $handle_intact  = \&CodingAdventures::TestBootstrap::Fuzz::Handle::intact;
my $handle_repair  = \&CodingAdventures::TestBootstrap::Fuzz::Handle::repair;
my $out_pass    = \&CodingAdventures::TestBootstrap::Fuzz::Outcome::pass;
my $out_timeout = \&CodingAdventures::TestBootstrap::Fuzz::Outcome::timeout;
my $out_oom     = \&CodingAdventures::TestBootstrap::Fuzz::Outcome::oom;
my $out_crash   = \&CodingAdventures::TestBootstrap::Fuzz::Outcome::crash;
my $norm_kind   = \&CodingAdventures::TestBootstrap::Fuzz::Signature::normalise_kind;
my $norm_frame  = \&CodingAdventures::TestBootstrap::Fuzz::Signature::normalise_frame;
my $collapse    = \&CodingAdventures::TestBootstrap::Fuzz::Signature::collapse;
my $refaddr  = \&Scalar::Util::refaddr;
my $blessed  = \&Scalar::Util::blessed;
my $reftype  = \&Scalar::Util::reftype;
my $now      = \&Time::HiRes::time;
my $set_name = \&Sub::Util::set_subname;
my $MAX_WALK    = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_WALK();
my $MAX_FRAMES  = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_FRAMES();
my $REARM       = CodingAdventures::TestBootstrap::Fuzz::Limits::REARM_SECONDS();
my $MARGIN      = CodingAdventures::TestBootstrap::Fuzz::Limits::ALARM_MARGIN();

use constant {
    STALE           => 'stale-budget-sentinel',
    ALARM_TAMPERED  => 'alarm-tampered',
    ENGINE_TAMPERED => 'engine-tampered',
    ESCAPED         => 'escaped',
};

# Whole-second alarm, rounded UP (the platform fallback).  Returns the seconds left on the
# previous timer.
sub _int_alarm {
    my $s = $_[0];
    return CORE::alarm( $s <= 0 ? 0 : int($s) + ( $s > int($s) ? 1 : 0 ) );
}

# `Time::HiRes::alarm` where the platform has it, whole seconds (rounded UP) otherwise.
my $HAVE_HIRES = eval { Time::HiRes::alarm(0); 1 } ? 1 : 0;
my $hires_alarm = \&Time::HiRes::alarm;
my $int_alarm   = \&_int_alarm;

# A TEST SEAM (like Fuzz::Hooks): true forces the whole-second fallback so its tamper
# arithmetic can be exercised on a platform that has Time::HiRes::alarm.
our $FORCE_INTEGER_ALARM = 0;

# The directory of the engine's own files: a frame inside it is plumbing, never "the
# target".  Empty (nothing excluded) when this file was loaded without a directory part.
my $ENGINE_DIR = do { my $d = __FILE__; $d =~ s{[^/\\]*\z}{}; $d };
my $BODY_NAME  = __PACKAGE__ . '::target_body';

sub _is_engine_file { return ( length $ENGINE_DIR && index( $_[0], $ENGINE_DIR ) == 0 ) ? 1 : 0 }

# The `__DIE__` recorder of one execution.  It runs at the die site, before unwinding, and
# stores the frames (`basename#line`, innermost first) up to the named body closure.  It
# reads NO argument: the error is never touched.  The LAST die before the error escapes wins.
sub _recorder {
    my ($st) = @_;
    return sub {
        my @frames;
        for my $i ( 0 .. $MAX_WALK - 1 ) {
            my @c = caller($i);
            last if !@c || ( defined $c[3] && $c[3] eq $BODY_NAME );
            next if _is_engine_file( defined $c[1] ? $c[1] : '' );
            my $f = $norm_frame->( $c[1], $c[2] );
            push @frames, $f if !@frames || $frames[-1] ne $f;
            last if @frames >= $MAX_FRAMES;
        }
        $st->{frames} = \@frames;
        return;
    };
}

# The kind of an error value, from `ref`, `blessed` and `reftype` ONLY: nothing is
# stringified, numified, compared or called.
sub _kind_of {
    my ($err) = @_;
    return 'undef'  if !defined $err;
    return 'string' if !ref $err;
    my $class = $blessed->($err);
    return $norm_kind->($class) if defined $class;
    return $norm_kind->( $reftype->($err) );
}

# Runs `target(data, handle)` once.  Returns ($outcome, \@edges, $edge_calls); the edges are
# those of THIS execution, read after the handle was retired.
sub run {
    my ( $target, $data, $o ) = @_;
    die "target must be a code reference\n" if !defined $target || ( $reftype->($target) // '' ) ne 'CODE';
    die "data must be a string\n" if !defined $data || ref $data;
    my ( $h, $st ) = $handle_new->( $o->{step_budget}, $o->{mem_budget} );
    my $pid    = $$;
    my $fallback = ( !$HAVE_HIRES || $FORCE_INTEGER_ALARM ) ? 1 : 0;
    my $alarm  = $fallback ? $int_alarm : $hires_alarm;
    my $secs   = $o->{timeout_ms} / 1000;
    my $active = 0;
    my $fired  = 0;
    my $handler = sub {
        return if !$active;    # the execution is over: a late delivery is a no-op
        $fired = 1;
        $handle_breach->( $st, 'steps' );
        $alarm->($REARM);
        die $st->{exhausted};
    };
    my $recorder = _recorder($st);
    my $body = $set_name->( $BODY_NAME, sub { local $SIG{__DIE__} = $recorder; return $target->( $data, $h ) } );

    my ( $status, $value ) = ( ESCAPED, undef );
    my ( $outer, $span, $start, $elapsed, $remaining, $handler_ok );
    {
        local $@;
        local $SIG{ALRM} = $handler;
        $outer = $alarm->(0);    # an alarm the caller had pending
        $span  = ( $outer > 0 && $outer < $secs ) ? $outer : $secs;
        # the span the timer REALLY has: the fallback rounds it up to whole seconds, and every
        # comparison below uses that, not the fractional wish
        $span  = int($span) + ( $span > int($span) ? 1 : 0 ) if $fallback;
        $start = $now->();
        $alarm->($span);
        $active = 1;
        eval {
            ( $status, $value ) = $safe_call->($body);
            $active = 0;
            1;
        };
        # (INFO) a re-armed timer may fire between leaving the eval and clearing the flag: retry
        # inside an eval until the flag is down, so a late delivery can never die outside one.
        until ( !$active ) { eval { $active = 0; 1 } }
        $remaining = $alarm->(0);
        $elapsed   = $now->() - $start;
        my $cur = $SIG{ALRM};
        $handler_ok = ( defined $cur && ref $cur && $refaddr->($cur) == $refaddr->($handler) ) ? 1 : 0;
    }
    # The caller's own alarm survives: re-armed with what it had left.
    $alarm->( ( $outer - $elapsed ) > 0.001 ? $outer - $elapsed : 0.001 ) if $outer > 0;

    # A forked child of the target has returned into the engine: it must not run a campaign.
    POSIX::_exit(86) if $$ != $pid;

    # (1) was the alarm left alone?  Only a REPLACED handler, or a run that overran its
    # deadline while the timer was cancelled or re-armed, is tampering.  A timer found at 0
    # that never fired is the honest `eval { local $SIG{ALRM} = ...; alarm N; ...; alarm 0 }`
    # idiom (the target suspended our wall bound while its own alarm was active); it is fine
    # when the run stayed within the deadline.
    my $tampered = !$handler_ok;
    if ( !$tampered && !$fired && $elapsed > $span + $MARGIN ) {
        my $expected = $span - $elapsed;
        $tampered = 1 if $remaining <= 0 || $remaining > $expected + $MARGIN;
    }
    # (2) is the engine as it was loaded?
    my $engine_ok = $handle_intact->();
    $handle_repair->() if !$engine_ok;

    # (3) retire, then (4) read: nothing a stale reference does can change this verdict
    $handle_retire->($st);
    my $escaped = ( $tampered || !$engine_ok ) ? 1 : 0;
    my $which   = $handle_which->($st) // $handle_over->($st);
    my $outcome;
    if ($which) {
        $outcome = $which eq 'steps' ? $out_timeout->($escaped) : $out_oom->($escaped);
    }
    elsif ($tampered)    { $outcome = $out_crash->( ALARM_TAMPERED, [], 1 ) }
    elsif ( !$engine_ok ) { $outcome = $out_crash->( ENGINE_TAMPERED, [], 1 ) }
    elsif ( $status eq 'ok' ) { $outcome = $out_pass->(0) }
    else {
        my $frames = $status eq 'died' ? $collapse->( $st->{frames} || [] ) : [];
        if ( $status eq 'died' ) {
            # A budget sentinel that surfaces while this execution is NOT exhausted is stale
            # (retained from another execution, or the retired sentinel of a retired
            # handle): a crash, never a timeout.
            $outcome = $handle_record->($value) ? $out_crash->( STALE, $frames, 0 ) : $out_crash->( _kind_of($value), $frames, 0 );
        }
        else {
            $outcome = $out_crash->( ESCAPED, $frames, 0 );
        }
    }
    return ( $outcome, $handle_edges->($st), $st->{edge_calls} );
}

# The outcome-level executor for `target`: `exec(data) -> (outcome, edges)`, plus a function
# that reports how many times the target has called `edge` so far (the coverage tier is
# evidence, not a flag).
sub make {
    my ( $target, $options ) = @_;
    die "target must be a code reference\n" if !defined $target || ( $reftype->($target) // '' ) ne 'CODE';
    my $calls = 0;
    my $exec  = sub {
        my ( $outcome, $edges, $n ) = run( $target, $_[0], $options );
        $calls += $n;
        return ( $outcome, $edges );
    };
    return ( $exec, sub { return $calls } );
}

1;
