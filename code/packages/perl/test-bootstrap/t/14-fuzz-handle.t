# The per-execution handle, sentinels and the executor's isolation (VER06q section 5).
# Every target here terminates by construction or is stopped by the engine's own alarm, and
# every test runs under the `bounded_by` alarm of the helper as a second net.
use strict;
use warnings;
use Test::More;
use Scalar::Util qw(blessed refaddr reftype);
use Time::HiRes ();
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(bounded_by);
use CodingAdventures::TestBootstrap;

my $H = 'CodingAdventures::TestBootstrap::Fuzz::Handle';
my $SENT = 'CodingAdventures::TestBootstrap::Fuzz::Handle::Sentinel';
my $OBJ  = 'CodingAdventures::TestBootstrap::Fuzz::Handle::Object';

sub opts { return CodingAdventures::TestBootstrap::Fuzz::Options->new( { timeout_ms => 150, step_budget => 5, mem_budget => 100, @_ } ) }

# Runs `$target` once on `$data`; returns the outcome (and the edges).
sub once {
    my ( $target, $data, %o ) = @_;
    my ($exec) = CodingAdventures::TestBootstrap::Fuzz::Executor::make( $target, opts(%o) );
    my @r = bounded_by( 20, sub { $exec->( defined $data ? $data : 'x' ) } );
    return wantarray ? @r : $r[0];
}

sub sentinel_info { return CodingAdventures::TestBootstrap::Fuzz::Handle::sentinel_record( $_[0] ) }

subtest 'a handle is an empty scalar: its state is not in the object' => sub {
    my ( $h, $st ) = CodingAdventures::TestBootstrap::Fuzz::Handle::new( 5, 100 );
    is( reftype($h), 'SCALAR', 'a scalar reference' );
    is( blessed($h), $OBJ, 'of the target-facing class' );
    is( $$h, 0, 'holding nothing' );
    ok( !$h->can('retire') && !$h->can('new') && !$h->can('intact'), 'the machinery is not a method of the object' );
    $h->edge(7);
    $h->tick;
    is( $st->{steps}, 1, 'the counters live in the private state' );
    CodingAdventures::TestBootstrap::Fuzz::Handle::retire($st);
    $h->edge(8);
    is_deeply( CodingAdventures::TestBootstrap::Fuzz::Handle::edge_list($st), [7], 'a retired handle records nothing' );
};

subtest 'retained, forwarded and forged handles are inert' => sub {
    my $kept;
    my ( $out, $edges ) = once( sub { $kept = $_[1]; $_[1]->edge(3); return } );
    is( $out->{kind}, 'pass' );
    is_deeply( $edges, [3], 'the live handle recorded its edge' );
    ok( eval { $kept->edge(5); 1 }, 'edge on a retired handle is a no-op ...' );
    ( $out, $edges ) = once( sub { $_[1]->edge(4); return } );
    is_deeply( $edges, [4], '... and leaked nothing into the next execution' );
    for my $call ( [ 'tick', 1 ], [ 'tick', 0 ], [ 'reserve', 1 ], [ 'reserve', 0 ] ) {
        my ( $m, $arg ) = @$call;
        ok( !eval { $kept->$m($arg); 1 }, "$m($arg) on a retired handle dies" );
        my $err = $@;
        is( blessed($err), $SENT, '... with a sentinel object' );
        is_deeply( sentinel_info($err), [ 0, 'retired' ], '... the retired sentinel, found by refaddr' );
    }
    my $forged = bless \( my $x = 0 ), $OBJ;
    ok( eval { $forged->edge(9); 1 }, 'edge on a forged handle is a no-op' );
    ok( !eval { $forged->tick; 1 } && sentinel_info($@), 'tick on a forged handle dies with a sentinel' );
    ok( !eval { $forged->reserve(5); 1 } && sentinel_info($@), 'reserve on a forged handle dies with a sentinel' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Handle::tick( undef, 1 ); 1 } && sentinel_info($@), 'tick on undef dies with a sentinel' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Handle::tick( {}, 1 ); 1 } && sentinel_info($@), 'tick on a hash dies with a sentinel' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Handle::tick( bless( \( my $y = 0 ), 'Other' ), 1 ); 1 } && sentinel_info($@), 'a handle of another class is forged' );
    # a handle from an execution that is still running is not another execution's handle
    my $outer;
    ( $out ) = once( sub { $outer = $_[1]; return } );
    ( $out ) = once( sub { $outer->tick; return } );
    is_deeply( [ $out->{kind}, $out->{crash_kind} ], [ 'crash', 'stale-budget-sentinel' ], 'a stale handle used in a later execution is a stale-sentinel crash' );
};

subtest 'edges: reduction and validation' => sub {
    my ( $out, $edges ) = once( sub { my $h = $_[1]; $h->edge($_) for ( 1, 65537, -1, 70000, CodingAdventures::TestBootstrap::Int::parse('18446744073709551615'), 3 ); return } );
    is_deeply( $edges, [ 1, 3, 4464, 65535 ], 'reduced mod 65,536, unique, ascending' );
    ( $out ) = once( sub { $_[1]->edge(1.5) } );
    is_deeply( [ $out->{kind}, $out->{crash_kind} ], [ 'crash', 'string' ], 'a float edge id is a crash of the target' );
    my $id = CodingAdventures::TestBootstrap::Fuzz::Handle::edge_id('entered');
    is( $id, CodingAdventures::TestBootstrap::Prng::fnv1a64_word('entered') & 0xFFFF, 'edge_id is the low 16 bits of fnv1a64' );
    is( $H->edge_id('entered'), $id, 'also callable as a class method' );
    my ($hh) = CodingAdventures::TestBootstrap::Fuzz::Handle::new( 1, 1 );
    is( $hh->edge_id('entered'), $id, 'and as a method of the handle the target holds' );
    my ( $exec, $calls ) = CodingAdventures::TestBootstrap::Fuzz::Executor::make( sub { $_[1]->edge(1); $_[1]->edge(1) }, opts() );
    $exec->('a');
    $exec->('b');
    is( $calls->(), 4, 'the edge-call counter is evidence for the coverage tier' );
};

subtest 'sticky exhaustion, both directions, judged after retirement' => sub {
    my @log;
    my ( $out ) = once( sub {
        my $h = $_[1];
        eval { $h->tick for 1 .. 10; 1 };
        push @log, ( blessed($@) // 'none' );
        push @log, ( eval { $h->reserve(0); 1 } ? 'reserve(0) lived' : 'reserve(0) died' );
        push @log, ( eval { $h->reserve(1); 1 } ? 'reserve(1) lived' : 'reserve(1) died' );
        push @log, ( eval { $h->tick(0); 1 } ? 'tick(0) lived' : 'tick(0) died' );
        push @log, ( eval { $h->tick; 1 } ? 'tick lived' : 'tick died' );
        return 'swallowed it and returned normally';
    }, undef, step_budget => 5 );
    is( $out->{kind}, 'timeout', 'a steps breach the target swallowed is still a timeout' );
    is_deeply( \@log, [ $SENT, 'reserve(0) died', 'reserve(1) died', 'tick(0) died', 'tick died' ], 'after a steps breach reserve(0) and reserve(1) die' );

    @log = ();
    ( $out ) = once( sub {
        my $h = $_[1];
        eval { $h->reserve(1000); 1 };
        push @log, ( blessed($@) // 'none' );
        push @log, ( eval { $h->tick(0); 1 } ? 'tick(0) lived' : 'tick(0) died' );
        push @log, ( eval { $h->tick; 1 } ? 'tick lived' : 'tick died' );
        push @log, ( eval { $h->reserve(0); 1 } ? 'reserve(0) lived' : 'reserve(0) died' );
        die "something ordinary\n";
    }, undef, mem_budget => 100 );
    is( $out->{kind}, 'oom', 'a memory breach followed by an ordinary death is still an oom' );
    is_deeply( \@log, [ $SENT, 'tick(0) died', 'tick died', 'reserve(0) died' ], 'after a memory breach tick(0) dies' );

    ( $out ) = once( sub { no warnings 'exiting'; my $h = $_[1]; eval { $h->tick for 1 .. 10 }; last } );
    is( $out->{kind}, 'timeout', 'a breach followed by a non-local exit is still a timeout' );

    # the exact accounting
    ( $out ) = once( sub { $_[1]->tick for 1 .. 5; return }, undef, step_budget => 5 );
    is( $out->{kind}, 'pass', 'exactly step_budget ticks pass' );
    ( $out ) = once( sub { $_[1]->tick for 1 .. 6; return }, undef, step_budget => 5 );
    is( $out->{kind}, 'timeout', 'one more is a timeout' );
    ( $out ) = once( sub { $_[1]->tick(5); return }, undef, step_budget => 5 );
    is( $out->{kind}, 'pass', 'tick(5) is five units' );
    ( $out ) = once( sub { $_[1]->tick(6); return }, undef, step_budget => 5 );
    is( $out->{kind}, 'timeout', 'tick(6) is six' );
    ( $out ) = once( sub { $_[1]->tick( CodingAdventures::TestBootstrap::Int::parse('99999999999999999999999') ); return } );
    is( $out->{kind}, 'timeout', 'a big tick is a breach' );
    ( $out ) = once( sub { $_[1]->tick(9223372036854775807); return } );
    is( $out->{kind}, 'timeout', 'a tick of the largest int does not overflow into a float' );
    ( $out ) = once( sub { $_[1]->tick(-5); $_[1]->tick(0); return }, undef, step_budget => 1 );
    is( $out->{kind}, 'pass', 'a non-positive tick counts nothing' );
    ( $out ) = once( sub { $_[1]->reserve(100); return }, undef, mem_budget => 100 );
    is( $out->{kind}, 'pass', 'reserving exactly mem_budget passes' );
    ( $out ) = once( sub { $_[1]->reserve(101); return }, undef, mem_budget => 100 );
    is( $out->{kind}, 'oom', 'one more is an oom' );
    ( $out ) = once( sub { $_[1]->reserve(60); $_[1]->reserve(41); return }, undef, mem_budget => 100 );
    is( $out->{kind}, 'oom', 'reservations accumulate' );
    ( $out ) = once( sub { $_[1]->reserve(-5); $_[1]->reserve(0); return }, undef, mem_budget => 1 );
    is( $out->{kind}, 'pass', 'a non-positive reserve reserves nothing' );
    ( $out ) = once( sub { $_[1]->reserve( CodingAdventures::TestBootstrap::Int::parse('99999999999999999999999') ); return } );
    is( $out->{kind}, 'oom', 'a big reserve is a breach' );
    ( $out ) = once( sub { $_[1]->reserve(1.5) } );
    is( $out->{kind}, 'crash', 'a float reserve is an error of the target' );
    ( $out ) = once( sub { $_[1]->tick('x') } );
    is( $out->{kind}, 'crash', 'a string tick is an error of the target' );
};

subtest 'the verdict also comes from the counters, not only from the sticky marker' => sub {
    my $clear = \&CodingAdventures::TestBootstrap::Fuzz::Handle::_test_clear_exhausted;
    my $cleared;
    my ($out) = once( sub { my $h = $_[1]; eval { $h->tick for 1 .. 10 }; $cleared = $clear->($h); return 'carried on' }, undef, step_budget => 5 );
    ok( $cleared, 'the seam cleared a live handle\'s marker' );
    is( $out->{kind}, 'timeout', 'a steps breach whose marker was cleared is still a timeout' );
    ($out) = once( sub { my $h = $_[1]; eval { $h->reserve(1000) }; $clear->($h); return 'carried on' }, undef, mem_budget => 100 );
    is( $out->{kind}, 'oom', 'a memory breach whose marker was cleared is still an oom' );
    ok( !$clear->( bless \( my $x = 0 ), $OBJ ), 'the seam does nothing to a forged handle' );
};

subtest 'sentinels are recognised by refaddr, never by message' => sub {
    my ($out) = once( sub { die bless( \( my $x = 0 ), $SENT ) } );
    is_deeply( [ $out->{kind}, $out->{crash_kind} ], [ 'crash', CodingAdventures::TestBootstrap::Fuzz::Signature::normalise_kind($SENT) ], 'a forged sentinel is an ordinary crash (of its class), never a timeout' );
    my $kept;
    ( $out ) = once( sub { eval { $_[1]->tick for 1 .. 10 }; $kept = $@; return } );
    is( $out->{kind}, 'timeout' );
    is( blessed($kept), $SENT, 'a sentinel was issued' );
    is_deeply( sentinel_info($kept)->[1], 'steps', 'registered as a steps sentinel' );
    ( $out ) = once( sub { die $kept } );
    is_deeply( [ $out->{kind}, $out->{crash_kind} ], [ 'crash', 'stale-budget-sentinel' ], 'a sentinel kept from an earlier execution is a stale-sentinel crash' );
    # a message that looks like a budget error means nothing
    ( $out ) = once( sub { die "fuzz budget exhausted\n" } );
    is( $out->{kind}, 'crash', 'a message that sounds like exhaustion is a crash' );
    # the registry is cleaned with the sentinel: only live sentinels are kept
    my $addr = refaddr($kept);
    undef $kept;
    ok( !defined sentinel_info( bless \( my $z = 0 ), $SENT ), 'a fresh object is not in the registry' );
};

subtest 'wall bound: the alarm stops a runaway loop' => sub {
    my $t0 = Time::HiRes::time();
    my ($out) = once( sub { 1 while 1 }, undef, timeout_ms => 100 );
    is( $out->{kind}, 'timeout', 'a runaway loop is a timeout' );
    cmp_ok( Time::HiRes::time() - $t0, '<', 5, '... promptly' );
    if ( $^O ne 'MSWin32' ) {    # (Windows signals do not interrupt a blocking wait)
        ($out) = once( sub { select( undef, undef, undef, 20 ) }, undef, timeout_ms => 100 );
        is( $out->{kind}, 'timeout', 'a long sleep is interrupted too' );
    }
    my $after = 0;
    ($out) = once( sub { eval { 1 while 1 }; $after++; return 'it carried on after swallowing the sentinel' }, undef, timeout_ms => 100 );
    is( $out->{kind}, 'timeout', 'a target that swallowed the alarm sentinel is still a timeout' );
    is( $after, 1, '... and it did carry on' );
    ($out) = once( sub { my $n = 0; while (1) { eval { 1 while 1 }; last if ++$n >= 3 } return }, undef, timeout_ms => 100 );
    is( $out->{kind}, 'timeout', 'the timer re-arms: a target that swallows the sentinel three times is interrupted three times' );
    ($out) = once( sub { return 1 }, undef, timeout_ms => 100 );
    is( $out->{kind}, 'pass', 'a quick target passes' );
    ok( !$out->{escaped}, '... unescaped' );
    my ( $exec ) = CodingAdventures::TestBootstrap::Fuzz::Executor::make( sub { select( undef, undef, undef, 0.4 ) }, opts( timeout_ms => 100 ) );
    bounded_by( 20, sub { $exec->('a') } );
    select( undef, undef, undef, 0.4 );    # a late ALRM must not kill or confuse the next test
    pass('no stray alarm after the run');
};

subtest 'the caller\'s own alarm survives an execution' => sub {
    my $left;
    {
        local $SIG{ALRM} = sub { die "outer alarm fired\n" };
        alarm 30;
        my ($exec) = CodingAdventures::TestBootstrap::Fuzz::Executor::make( sub { 1 }, opts() );
        $exec->('x');    # (not through `once`: its bounded_by helper owns the alarm)
        $left = alarm 0;
    }
    cmp_ok( $left, '>=', 25, 'the outer alarm still has about 30 seconds' );
    # an outer deadline that is nearer than ours bounds the execution
    my $err;
    {
        local $SIG{ALRM} = sub { die "outer alarm fired\n" };
        Time::HiRes::alarm(0.2);
        my ($exec) = CodingAdventures::TestBootstrap::Fuzz::Executor::make( sub { 1 while 1 }, opts( timeout_ms => 20_000 ) );
        my ($o) = $exec->('x');
        is( $o->{kind}, 'timeout', 'the nearer outer deadline ended the execution as a timeout' );
        eval { select( undef, undef, undef, 2 ); 1 };    # the outer alarm is re-armed and fires shortly after
        $err = $@;
        alarm 0;
    }
    is( $err, "outer alarm fired\n", 'and the outer alarm then fired in its own handler' );
};

subtest 'tampering with the alarm is detected, never a pass' => sub {
    my ($out) = once( sub { $SIG{ALRM} = 'IGNORE'; return 1 }, undef, timeout_ms => 2000 );
    is_deeply( [ $out->{kind}, $out->{crash_kind}, $out->{escaped} ], [ 'crash', 'alarm-tampered', 1 ], 'a replaced handler: a tampered, leaked execution' );
    ($out) = once( sub { $SIG{ALRM} = sub { 1 }; return 1 }, undef, timeout_ms => 2000 );
    is( $out->{crash_kind}, 'alarm-tampered', 'a replaced handler (a different sub)' );
    ($out) = once( sub { Time::HiRes::alarm(0); return 1 }, undef, timeout_ms => 2000 );
    is_deeply( [ $out->{kind}, $out->{escaped} ], [ 'pass', 0 ], 'a cancelled timer within the deadline is not tampering' );
    ($out) = once( sub { Time::HiRes::alarm(60); return 1 }, undef, timeout_ms => 2000 );
    is_deeply( [ $out->{kind}, $out->{escaped} ], [ 'pass', 0 ], 'a timer re-armed for later, within the deadline, is not tampering' );
    ($out) = once( sub { Time::HiRes::alarm(60); select( undef, undef, undef, 0.45 ); return 1 }, undef, timeout_ms => 100 );
    is_deeply( [ $out->{crash_kind}, $out->{escaped} ], [ 'alarm-tampered', 1 ], 'a timer re-armed for later that let the run overrun is tampering' );
    ($out) = once( sub { Time::HiRes::alarm(0); select( undef, undef, undef, 0.4 ); return 1 }, undef, timeout_ms => 100 );
    is( $out->{crash_kind}, 'alarm-tampered', 'a cancelled timer that let the run pass its deadline' );
    ($out) = once( sub { $SIG{ALRM} = 'IGNORE'; eval { $_[1]->tick for 1 .. 10 }; return 1 }, undef, timeout_ms => 2000 );
    is_deeply( [ $out->{kind}, $out->{escaped} ], [ 'timeout', 1 ], 'a tampered execution that also broke its budget is a leaked timeout' );
    ($out) = once( sub { local $SIG{ALRM} = 'IGNORE'; return 1 } );
    is( $out->{kind}, 'pass', 'a `local` handler restored before returning is not detected (the documented residual)' );
};

subtest 'the honest alarm idiom is not tampering' => sub {
    my $idiom = sub {
        my $r = eval { local $SIG{ALRM} = sub { die "my alarm\n" }; alarm 2; my $x = 1 + 1; alarm 0; 1 };
        return $r;
    };
    my ($out) = once( $idiom, undef, timeout_ms => 2000 );
    is_deeply( [ $out->{kind}, $out->{escaped} ], [ 'pass', 0 ], 'pass, not escaped' );
    my $r = bounded_by( 60, sub { CodingAdventures::TestBootstrap::Fuzz::run( ['a'], opts( max_iterations => 30, timeout_ms => 2000, step_budget => 100 ), $idiom ) } );
    is_deeply( [ $r->{leaked}, $r->{aborted}, $r->{iterations} ], [ 0, 0, 30 ], 'repeated runs do not abort the campaign' );
    ok( CodingAdventures::TestBootstrap::Fuzz::Report::clean($r), '... and the campaign is clean' );
    # the target's own alarm fires into the target's own handler
    my $fired = 0;
    ($out) = once( sub { local $SIG{ALRM} = sub { $fired++ }; Time::HiRes::alarm(0.05); select( undef, undef, undef, 0.3 ); 1 }, undef, timeout_ms => 5000 );
    is( $fired, 1, 'its own alarm ran its own handler' );
    is( $out->{kind}, 'pass', 'and that is not tampering either' );
    # a replaced handler is still tampering
    ($out) = once( sub { $SIG{ALRM} = sub { 1 }; alarm 0; return 1 }, undef, timeout_ms => 2000 );
    is_deeply( [ $out->{crash_kind}, $out->{escaped} ], [ 'alarm-tampered', 1 ], 'a replaced handler is tampered' );
    # cancelling the alarm and then overrunning the deadline is caught, never a pass
    ($out) = once( sub { alarm 0; select( undef, undef, undef, 0.5 ); 1 }, undef, timeout_ms => 100 );
    isnt( $out->{kind}, 'pass', 'cancel + overrun is not a pass' );
    is( $out->{crash_kind}, 'alarm-tampered', '... it is tampered' );
};

subtest 'the whole-second fallback (no Time::HiRes::alarm) is honest too' => sub {
    local $CodingAdventures::TestBootstrap::Fuzz::Executor::FORCE_INTEGER_ALARM = 1;
    my ($out) = once( sub { 1 }, undef, timeout_ms => 150 );
    is_deeply( [ $out->{kind}, $out->{escaped} ], [ 'pass', 0 ], 'an honest execution is not tampered (a 150 ms wish is a 1 s timer)' );
    my $r = bounded_by( 60, sub { CodingAdventures::TestBootstrap::Fuzz::run( ['a'], opts( max_iterations => 20, timeout_ms => 150 ), sub { 1 } ) } );
    is_deeply( [ $r->{leaked}, $r->{iterations} ], [ 0, 20 ], 'twenty honest executions do not leak' );
    ($out) = once( sub { 1 while 1 }, undef, timeout_ms => 150 );
    is( $out->{kind}, 'timeout', 'a runaway loop is still stopped (after the rounded-up second)' );
    ($out) = once( sub { $SIG{ALRM} = 'IGNORE'; 1 }, undef, timeout_ms => 150 );
    is( $out->{crash_kind}, 'alarm-tampered', 'a replaced handler is tampered' );
    ($out) = once( sub { alarm 0; select( undef, undef, undef, 1.6 ); 1 }, undef, timeout_ms => 150 );
    is( $out->{crash_kind}, 'alarm-tampered', 'cancel + overrun of the rounded deadline is tampered' );
    my $left;
    {
        local $SIG{ALRM} = sub { die "outer\n" };
        alarm 30;
        my ($exec) = CodingAdventures::TestBootstrap::Fuzz::Executor::make( sub { 1 }, opts() );
        $exec->('x');
        $left = alarm 0;
    }
    cmp_ok( $left, '>=', 25, 'the caller\'s alarm still survives' );
};

subtest 'tampering with the engine is detected and repaired' => sub {
    my ($out) = once( sub {
        no strict 'refs';
        no warnings;
        delete ${"${OBJ}::"}{tick};
        *{"${OBJ}::tick"} = sub { return };
        return 1;
    } );
    is_deeply( [ $out->{crash_kind}, $out->{escaped} ], [ 'engine-tampered', 1 ], 'a replaced method: engine-tampered, leaked' );
    ok( $H->can('intact') && CodingAdventures::TestBootstrap::Fuzz::Handle::intact(), 'the next check finds the engine as loaded (it was repaired)' );
    ($out) = once( sub { $_[1]->tick for 1 .. 10; return }, undef, step_budget => 5 );
    is( $out->{kind}, 'timeout', 'and the repaired tick counts again' );
    ($out) = once( sub { no strict 'refs'; push @{"${OBJ}::ISA"}, 'Foo'; return 1 } );
    is( $out->{crash_kind}, 'engine-tampered', 'a planted @ISA' );
    ok( CodingAdventures::TestBootstrap::Fuzz::Handle::intact(), '... repaired' );
    ($out) = once( sub { no strict 'refs'; *{"${OBJ}::extra"} = sub { 1 }; return 1 } );
    is( $out->{crash_kind}, 'engine-tampered', 'a planted new sub' );
    ok( !defined &{"${OBJ}::extra"}, '... removed by the repair' );
    ($out) = once( sub { no strict 'refs'; no warnings; delete ${"${H}::"}{breach}; return 1 } );
    is( $out->{crash_kind}, 'engine-tampered', 'a deleted engine sub' );
    ok( CodingAdventures::TestBootstrap::Fuzz::Handle::intact(), '... restored' );
    ($out) = once( sub { 1 } );
    is( $out->{kind}, 'pass', 'the engine works afterwards' );
    # a sealed glob cannot be reassigned
    ok( !eval { no warnings; no strict 'refs'; *{"${H}::tick"} = sub { }; 1 }, 'a sealed glob refuses assignment' );
    ok( !eval { no warnings; no strict 'refs'; local *{"CodingAdventures::TestBootstrap::Fuzz::Mutate::splice_bytes"} = sub { }; 1 }, 'and `local`' );
};

subtest 'a forked child that returns into the engine exits at once' => sub {
    plan skip_all => 'fork is emulated with threads on Windows' if $^O eq 'MSWin32';
    my $status;
    my ($out) = once( sub {
        my $kid = fork;
        die "fork failed\n" if !defined $kid;
        return if $kid == 0;    # the child returns into the engine: it must exit with 86
        waitpid( $kid, 0 );
        $status = $? >> 8;
        return;
    } );
    is( $out->{kind}, 'pass', 'the parent is unaffected' );
    is( $status, 86, 'the child exited with status 86 (no END blocks, no second campaign)' );
};

subtest 'error values are classified by ref, blessed and reftype only' => sub {
    $::TOUCHED = 0;
    {
        package HostileErr;
        use overload
          '""'   => sub { $::TOUCHED++; die "stringified\n" },
          'eq'   => sub { $::TOUCHED++; die "eq\n" },
          'bool' => sub { $::TOUCHED++; die "bool\n" },
          '0+'   => sub { $::TOUCHED++; die "numified\n" },
          '=='   => sub { $::TOUCHED++; die "==\n" },
          '@{}'  => sub { $::TOUCHED++; die "array\n" },
          '%{}'  => sub { $::TOUCHED++; die "hash\n" },
          '&{}'  => sub { $::TOUCHED++; die "code\n" },
          fallback => 0;
    }
    {
        package Some::Weird::Class;
    }
    my %kind;
    for my $c (
        [ string => sub { die "just a message\n" } ],
        [ hostile => sub { die bless [], 'HostileErr' } ],
        [ plain_hash => sub { die { code => 5 } } ],
        [ plain_array => sub { die [1] } ],
        [ plain_code => sub { die sub { 1 } } ],
        [ scalar_ref => sub { die \"x" } ],
        [ blessed => sub { die bless {}, 'Some::Weird::Class' } ],
        [ long_class => sub { die bless {}, 'K' x 500 } ],
        [ evil_class => sub { die bless {}, "Evil\nnot ok 1 - forged" } ],
    ) {
        my ($out) = once( $c->[1] );
        $kind{ $c->[0] } = $out->{crash_kind};
        is( $out->{kind}, 'crash', "$c->[0]: a crash" );
    }
    is( $::TOUCHED, 0, 'no overload of the error value ran' );
    is( $kind{string}, 'string' );
    is( $kind{hostile}, 'HostileErr' );
    is( $kind{plain_hash}, 'HASH' );
    is( $kind{plain_array}, 'ARRAY' );
    is( $kind{plain_code}, 'CODE' );
    is( $kind{scalar_ref}, 'SCALAR' );
    is( $kind{blessed}, 'Some__Weird__Class', 'a class name is normalised: no colon' );
    is( $kind{long_class}, 'K' x 64, 'a class name is capped at 64' );
    is( $kind{evil_class}, 'Evil_not_ok_1_-_forged', 'a hostile class name cannot add a line' );
    # a tied $@-like value cannot run code either: die with a tied-scalar reference
    {
        package TiedBomb;
        sub TIESCALAR { bless [], shift }
        sub FETCH { $::TOUCHED++; die "fetch\n" }
    }
    tie my $bomb, 'TiedBomb';
    my ($out) = once( sub { die \$bomb } );
    is_deeply( [ $out->{kind}, $out->{crash_kind} ], [ 'crash', 'SCALAR' ], 'a reference to a tied scalar is a SCALAR crash' );
    is( $::TOUCHED, 0, 'and no FETCH ran' );
    # the same place, different messages: one crash
    my @sigs;
    for my $msg ( 'one', 'two', "multi\nline" ) {
        my ($o) = once( sub { die "$msg\n" } );
        push @sigs, CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( $o->{crash_kind}, $o->{frames} );
    }
    ok( $sigs[0] eq $sigs[1] && $sigs[1] eq $sigs[2], 'the same die site is one crash whatever the message' );
};

subtest 'frames: the die site and its callers, up to the target body' => sub {
    my $inner = sub { die "x\n" };    # the die site
    my $outer = sub { $inner->() };
    my $path = sub { $outer->() };
    my ($out) = once($path);
    my @f = @{ $out->{frames} };
    ok( @f >= 3, 'the die site and two callers' ) or diag "@f";
    like( $f[0], qr/\A14-fuzz-handle\.t#\d+\z/, 'basename#line' );
    ok( !( grep { /:/ } @f ), 'no frame contains a colon' );
    ok( !( grep { /Executor|Handle\.pm|Safe\.pm/ } @f ), 'engine plumbing is not part of the signature' );
    my ($o2) = once($path);
    is_deeply( $o2->{frames}, \@f, 'stable across runs' );
    ($o2) = once( sub { $inner->() } );
    isnt( join( ',', @{ $o2->{frames} } ), join( ',', @f ), 'a different call path is a different signature' );
    # recursion collapses
    my $deep;
    $deep = sub { no warnings "recursion"; my $n = shift; return $n ? $deep->( $n - 1 ) : die "deep\n" };
    ($o2) = once( sub { $deep->(200) } );
    cmp_ok( scalar @{ $o2->{frames} }, '<=', 6, 'a recursive crash collapses its consecutive duplicate frames' );
    # a target that installs its own die handler hides its frames: the bare kind
    ($o2) = once( sub { local $SIG{__DIE__} = sub { }; die "hidden\n" } );
    is_deeply( $o2->{frames}, [], 'the target\'s own __DIE__ handler hides frames (documented)' );
    # a die inside an eval inside the target does not leak frames into a pass
    ($o2) = once( sub { eval { die "inner\n" }; return 1 } );
    is( $o2->{kind}, 'pass', 'a caught die is not a crash' );
    # eval-string frames are normalised
    ($o2) = once( sub { eval "die 'x'" or die $@ } );
    ok( !( grep { /\(eval|[ :]/ } @{ $o2->{frames} } ), 'no (eval N) in frames' );
};

subtest 'escapes: last and next out of the target' => sub {
    my ($out) = once( sub { no warnings 'exiting'; last } );
    is_deeply( [ $out->{kind}, $out->{crash_kind} ], [ 'crash', 'escaped' ], 'a `last` out of the target is an escaped crash' );
    ($out) = once( sub { no warnings 'exiting'; next } );
    is( $out->{crash_kind}, 'escaped', 'so is `next`' );
    ($out) = once( sub { no warnings 'exiting'; for my $i (1) { ( sub { last } )->() } return 1 } );
    is( $out->{kind}, 'pass', 'a `last` aimed at the target\'s own loop is the target\'s business' );
};

subtest 'the executor and the guard refuse what is not theirs' => sub {
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Executor::make( 'x', opts() ); 1 }, 'a target must be code' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Executor::run( undef, 'x', opts() ); 1 }, 'run refuses a non-code target' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Executor::run( sub { }, undef, opts() ); 1 }, 'run refuses undef data' );
    my $o = opts();
    my $leaks = CodingAdventures::TestBootstrap::Fuzz::Engine::new_leaks();
    my $g = CodingAdventures::TestBootstrap::Fuzz::Engine::guard( sub { return ( { kind => 'pass', escaped => 0 }, [] ) }, $leaks );
    ok( !eval { $g->('x'); 1 }, 'an exec that fabricates an outcome is refused' );
    my $real = CodingAdventures::TestBootstrap::Fuzz::Outcome::pass();
    ok( CodingAdventures::TestBootstrap::Fuzz::Outcome::is($real), 'a built outcome is recognised' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Outcome::is( bless { %$real }, ref $real ), 'a copy is not' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Outcome::is( { kind => 'pass' } ), 'a hash is not' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Outcome::is(undef), 'undef is not' );
    like( "$real", qr/Outcome pass/, 'an outcome prints its kind only' );
    like( CodingAdventures::TestBootstrap::Fuzz::Outcome::crash( 'K', [] ) . '', qr/crash K/ );
    ok( CodingAdventures::TestBootstrap::Fuzz::Outcome::is_pass($real) && !CodingAdventures::TestBootstrap::Fuzz::Outcome::is_crash($real) );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Outcome::is_pass(undef) );
};

done_testing;
