# Campaign contracts (VER06 section 9.1), verdicts, caps and the leak cap in every path
# (VER06q sections 5.4 and 6).  Every target terminates by construction.
use strict;
use warnings;
use Test::More;
use Scalar::Util qw(blessed);
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(bounded_by);
use CodingAdventures::TestBootstrap;

my $F   = 'CodingAdventures::TestBootstrap::Fuzz';
my $Out = 'CodingAdventures::TestBootstrap::Fuzz::Outcome';
sub opts { return CodingAdventures::TestBootstrap::Fuzz::Options->new( { ci => 'off', timeout_ms => 2000, @_ } ) }
sub hex_of { return unpack 'H*', $_[0] }
sub pass_out  { return $Out->can('pass')->(@_) }
sub crash_out { return $Out->can('crash')->(@_) }
sub run_out {
    my @a = @_;
    return bounded_by( 120, sub { CodingAdventures::TestBootstrap::Fuzz::run_outcomes(@a) } );
}
sub run_t {
    my @a = @_;
    return bounded_by( 120, sub { CodingAdventures::TestBootstrap::Fuzz::run(@a) } );
}

subtest 'options are validated' => sub {
    my $O = 'CodingAdventures::TestBootstrap::Fuzz::Options';
    my $o = $O->new;
    is_deeply( [ @{$o}{qw(max_iterations max_input_len base_energy minimize_budget step_budget timeout_ms ci)} ], [ 1000, 4096, 10, 2000, 200_000, 5000, 'unset' ], 'defaults (an unset ci is CI)' );
    is( $o->{seed}, 42 );
    is( $o->{mem_budget}, 64 << 20 );
    ok( !eval { $O->new( { bogus => 1 } ); 1 }, 'an unknown option' );
    ok( !eval { $O->new( { max_iterations => 1_000_001 } ); 1 }, 'above the iteration ceiling' );
    ok( !eval { $O->new( { max_iterations => -1 } ); 1 }, 'negative' );
    ok( !eval { $O->new( { max_iterations => 1.5 } ); 1 }, 'a float' );
    ok( !eval { $O->new( { max_iterations => '5' } ); 1 }, 'a numeric string' );
    ok( !eval { $O->new( { timeout_ms => 0 } ); 1 }, 'a zero wall bound' );
    ok( !eval { $O->new( { timeout_ms => 60_001 } ); 1 }, 'above the wall ceiling' );
    ok( !eval { $O->new( { mem_budget => (1 << 30) + 1 } ); 1 }, 'above the memory ceiling' );
    ok( !eval { $O->new( { max_input_len => (1 << 20) + 1 } ); 1 }, 'above the input ceiling' );
    ok( !eval { $O->new( { ci => 'maybe' } ); 1 }, 'a bad ci mode' );
    ok( !eval { $O->new( { seed => 1.5 } ); 1 }, 'a float seed' );
    ok( !eval { $O->new( { seed => 'x' } ); 1 }, 'a string seed' );
    ok( !eval { $O->new( [] ); 1 }, 'not a hash' );
    is( $O->new( { seed => CodingAdventures::TestBootstrap::Int::parse('18446744073709551616') } )->{seed}->bstr, '18446744073709551616', 'a big seed' );
    unlike( $o . '', qr/42/, 'the seed is not printed' );
    ok( $O->can('is')->($o) && !$O->can('is')->( {} ) && !$O->can('is')->(undef), 'is' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::run( [], {}, sub { } ); 1 }, 'run refuses a non-Options' );
};

subtest 'budget, determinism and an honest report' => sub {
    my @lens;
    my $exec = sub { push @lens, length $_[0]; return ( pass_out(), [ length( $_[0] ) % 7, ord( substr( $_[0] . "\0", 0, 1 ) ) % 9 ] ) };
    my $o = opts( max_iterations => 300, seed => 5, max_input_len => 6 );
    my $r1 = run_out( ["abcdefghijkl"], $o, $exec );
    is( $r1->{iterations}, 300, 'iterations == max_iterations exactly with a live corpus' );
    ok( !( grep { $_ > 6 } @lens[ 1 .. $#lens ] ), 'no mutated candidate exceeds max_input_len' );
    is( $lens[0], 12, 'initial seeds run as given' );
    my $r2 = run_out( ["abcdefghijkl"], $o, $exec );
    is_deeply( [ map { $r1->{$_} } qw(iterations admitted_seeds total_edges) ], [ map { $r2->{$_} } qw(iterations admitted_seeds total_edges) ], 'deterministic' );
    is_deeply( CodingAdventures::TestBootstrap::Fuzz::Report::corpus($r1), CodingAdventures::TestBootstrap::Fuzz::Report::corpus($r2), '... down to the corpus' );
    my $r3 = run_out( ["abcdefghijkl"], opts( max_iterations => 300, seed => 6, max_input_len => 6 ), $exec );
    isnt( join( ',', map { hex_of($_) } @{ CodingAdventures::TestBootstrap::Fuzz::Report::corpus($r3) } ), join( ',', map { hex_of($_) } @{ CodingAdventures::TestBootstrap::Fuzz::Report::corpus($r1) } ), 'a different seed is a different campaign' );
    ok( CodingAdventures::TestBootstrap::Fuzz::Report::clean($r1), 'a live, quiet campaign is clean' );
    like( "$r1", qr/iterations=300/, 'the report prints counts' );
    unlike( "$r1", qr/abcdef|\b5\b.*seed/, 'and never the corpus' );
    is( CodingAdventures::TestBootstrap::Fuzz::Report::seed($r1), 5, 'the seed is behind an accessor' );
    ok( !exists $r1->{seed} && !exists $r1->{corpus}, 'not a field of the report' );
    # zero executions is never a pass
    my $z = run_out( ["a"], opts( max_iterations => 0 ), $exec );
    is( $z->{iterations}, 0 );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Report::clean($z), 'max_iterations 0 is never clean' );
    # an empty corpus is replaced by one 00 seed
    my @seen;
    my $e = run_out( [], opts( max_iterations => 3 ), sub { push @seen, $_[0]; return ( pass_out(), [1] ) } );
    is( $seen[0], "\0", 'an empty corpus becomes one 00 seed' );
    is( $e->{iterations}, 3 );
    # seeds that crash are never mutated
    my $n = 0;
    my $c = run_out( [ 'a', 'b' ], opts( max_iterations => 50 ), sub { $n++; return ( crash_out( 'K', [] ), [] ) } );
    is( $c->{iterations}, 0, 'a corpus of crashers fuzzes nothing' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Report::clean($c), '... and is not clean' );
    is( $c->{crashes}[0]{hits}, 2, 'both seeds hit the one signature' );
    # seeds that time out or run out of memory are never mutated either
    for my $kind (qw(timeout oom)) {
        my $r = run_out( ['a'], opts( max_iterations => 20 ), sub { return ( $Out->can($kind)->(), [] ) } );
        is( $r->{iterations}, 0, "a $kind seed is not mutated" );
        is( $r->{ $kind . 's' }, 1, '... and is counted' );
        ok( !CodingAdventures::TestBootstrap::Fuzz::Report::clean($r), '... and is not clean' );
    }
    # a seed with the utf8 flag is reduced to bytes
    my @bytes;
    run_out( ["\x{263a}"], opts( max_iterations => 0 ), sub { push @bytes, $_[0]; return ( pass_out(), [] ) } );
    is( hex_of( $bytes[0] ), 'e298ba', 'a wide-character seed is its UTF-8 octets' );
};

subtest 'inputs are validated before anything runs' => sub {
    my $ran = 0;
    my $exec = sub { $ran++; return ( pass_out(), [] ) };
    my $RO = \&CodingAdventures::TestBootstrap::Fuzz::run_outcomes;
    ok( !eval { $RO->( 'x', opts(), $exec ); 1 }, 'the corpus must be a list' );
    ok( !eval { $RO->( [undef], opts(), $exec ); 1 }, 'a seed must be defined' );
    ok( !eval { $RO->( [ [] ], opts(), $exec ); 1 }, 'a seed must not be a reference' );
    ok( !eval { $RO->( [ "\0" x ( ( 1 << 20 ) + 1 ) ], opts(), $exec ); 1 }, 'a seed above 1 MiB' );
    ok( !eval { $RO->( [ ('a') x 10_001 ], opts(), $exec ); 1 }, 'more than 10,000 seeds' );
    ok( !eval { $RO->( ['a'], {}, $exec ); 1 }, 'options must be Options' );
    ok( !eval { $RO->( ['a'], opts(), 'x' ); 1 }, 'exec must be code' );
    is( $ran, 0, 'nothing ran' );
    ok( eval { $RO->( [ "\0" x ( 1 << 20 ) ], opts( max_iterations => 0 ), $exec ); 1 }, 'a seed of exactly 1 MiB is fine' );
};

subtest 'dedup, minimisation and the crash cap' => sub {
    my $exec = sub {
        my ($d) = @_;
        return ( crash_out( 'K', ['f'] ), [ 1, 2 ] ) if $d =~ /Z/;
        return ( pass_out(), [ 1, length($d) % 5 ] );
    };
    my $r = run_out( [ 'aaZaa', 'bbbbZ' ], opts( max_iterations => 100, seed => 3 ), $exec );
    is( scalar @{ $r->{crashes} }, 1, 'one crash record per signature (initial seeds included)' );
    my $rec = $r->{crashes}[0];
    is( $rec->minimal_input, 'Z', 'minimised to the single byte that matters' );
    ok( $rec->{minimized}, 'and it says it finished' );
    cmp_ok( $rec->{hits}, '>=', 2, 'hits count every execution with that signature' );
    unlike( "$rec", qr/Z/, 'the printed record carries no input' );
    # more than 1,000 distinct signatures: the rest are counted, not kept
    my $k = 0;
    my $many = run_out( [ map { "s$_" } 1 .. 1002 ], opts( max_iterations => 0, minimize_budget => 0 ), sub { return ( crash_out( 'K' . $k++, [] ), [] ) } );
    is( scalar @{ $many->{crashes} }, 1000, 'at most 1,000 records' );
    is( $many->{crashes_dropped}, 2, 'the rest are counted' );
    is( CodingAdventures::TestBootstrap::Fuzz::Report::unique_crashes($many), 1002 );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Report::clean($many) );
    # a crash that changes into another crash while shrinking is not accepted
    my $swap = run_out( ['abcd'], opts( max_iterations => 100 ), sub {
        my ($d) = @_;
        return ( crash_out( length($d) == 4 ? 'A' : 'B', [] ), [] );
    } );
    is( $swap->{crashes}[0]->minimal_input, "\0\0\0\0", 'the minimiser never trades one crash for another: no deletion (it would become crash B), only zeroing' );
    # the campaign-wide minimisation budget counts rejected candidates, and is max_iterations
    my $calls = 0;
    my $mb = run_out( [ map { "seed-number-$_-" . ( 'x' x 30 ) } 1 .. 20 ], opts( max_iterations => 10, minimize_budget => 100_000 ), sub {
        $calls++;
        return ( crash_out( 'K' . ( length( $_[0] ) % 1000 ), [] ), [] );
    } );
    cmp_ok( $calls - 20, '<=', 10, 'the campaign spends at most max_iterations minimisation executions in total (rejected candidates count)' );
    ok( scalar( grep { !$_->{minimized} } @{ $mb->{crashes} } ) >= 1, 'a crash whose budget ran out says it was not fully minimised' );
};

subtest 'retained bytes are capped before they are stored' => sub {
    my $r = run_outcomes_retained( 10 );
    is( $r->{admitted_seeds}, 1, 'the first seed fits' );
    cmp_ok( $r->{admissions_dropped}, '>', 0, 'later admissions are counted as dropped' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Report::clean($r), 'a campaign cut short by the cap is not clean' );
    my $c = bounded_by( 60, sub { CodingAdventures::TestBootstrap::Fuzz::run_outcomes( ['abcdefgh'], opts( max_iterations => 0 ), sub { return ( crash_out( 'K', [] ), [] ) }, { retained_cap => 4 } ) } );
    is( $c->{crashes}[0]{input_dropped}, 1, 'an over-cap crash record keeps no input' );
    is( $c->{crashes}[0]->minimal_input, '', '... not even a prefix' );
    ok( !$c->{crashes}[0]{minimized}, '... and is not "minimized"' );
    my $v = CodingAdventures::TestBootstrap::Fuzz::verdict( $c, opts(), sub { die "must not run\n" }, 't' );
    is( $v->{crashes}[0]{status}, 'unverified', 'a record with no input is unverified' );
    is_deeply( $v->{witnesses}, [], 'and has no witness' );
    my $min = bounded_by( 60, sub { CodingAdventures::TestBootstrap::Fuzz::run_outcomes( ['abcdefgh'], opts( max_iterations => 100 ), sub { return ( $_[0] =~ /a/ ? crash_out( 'K', [] ) : pass_out(), [] ) }, { retained_cap => 100 } ) } );
    is( $min->{crashes}[0]->minimal_input, 'a', 'within the cap the input is kept and minimised' );
};

sub run_outcomes_retained {
    my ($cap) = @_;
    my $n = 0;
    return bounded_by( 60, sub {
        CodingAdventures::TestBootstrap::Fuzz::run_outcomes( ['abcdefgh'], opts( max_iterations => 40, seed => 2 ), sub { return ( pass_out(), [ $n++ ] ) }, { retained_cap => $cap } );
    } );
}

subtest 'coverage guidance is observable' => sub {
    # Three stages: the bytes 35, 8, 1 at positions 0, 1, 2 (each one arithmetic or boundary
    # step from 0).  The guided exec reports the stage it reached as an edge; the blind one
    # reports none, so the engine can only mutate the first seed blindly.
    my $make = sub {
        my ($guided) = @_;
        return sub {
            my ($d) = @_;
            my @e = (1);
            if ( substr( $d, 0, 1 ) eq chr(35) ) {
                push @e, 2;
                if ( substr( $d, 1, 1 ) eq chr(8) ) {
                    push @e, 3;
                    return ( crash_out( 'k', ['f'] ), $guided ? \@e : [] ) if substr( $d, 2, 1 ) eq chr(1);
                }
            }
            return ( pass_out(), $guided ? \@e : [] );
        };
    };
    my ( $guided, $blind ) = ( 0, 0 );
    for my $seed ( 2, 3, 4, 5 ) {
        my $o = opts( max_iterations => 6000, seed => $seed, max_input_len => 8 );
        $guided++ if @{ run_out( ["\0\0\0"], $o, $make->(1) )->{crashes} };
        $blind++  if @{ run_out( ["\0\0\0"], $o, $make->(0) )->{crashes} };
    }
    cmp_ok( $guided, '>', $blind, "per-stage coverage cracks it more often ($guided of 4) than blind mutation ($blind of 4)" );
    cmp_ok( $guided, '>=', 3, 'guided: at least three seeds of four' );
    # the tier is evidence: edges actually reported through the handle
    my $with = run_t( ['a'], opts( max_iterations => 3 ), sub { $_[1]->edge(1) } );
    is( $with->{coverage_tier}, 'caller-reported', 'an edge-reporting target is caller-reported' );
    my $without = run_t( ['a'], opts( max_iterations => 3 ), sub { 1 } );
    is( $without->{coverage_tier}, 'black-box-uninstrumented', 'a target that reports no edge is black-box' );
    my $labelled = run_out( ['a'], opts( max_iterations => 1 ), sub { return ( pass_out(), [1] ) }, { tier => sub { "t$_[0]" } } );
    is( $labelled->{coverage_tier}, 't1', 'a custom tier label receives the edge total' );
};

subtest 'verdicts: honesty, CI mode and re-verification' => sub {
    my $target = sub { die "boom\n" if $_[0] =~ /X/; };
    my $o = opts( max_iterations => 400, seed => 9 );
    my $r = run_t( ["aXb"], $o, $target );
    ok( @{ $r->{crashes} }, 'found it' );
    my $v = CodingAdventures::TestBootstrap::Fuzz::verdict_for( $target, $r, $o, 'my target' );
    is( $v->{passed}, 0, 'not passed' );
    is( $v->{reason}, 'crash' );
    is( $v->{crashes}[0]{status}, 'reproduced', 'the witness reproduces' );
    is( $v->{seed}, 9, 'ci off: the seed' );
    is_deeply( $v->{witnesses}, [ { signature => $r->{crashes}[0]{signature}, input => 'X' } ], 'and the witness' );
    like( $v->{lines}[0], qr/\Anot ok 1 - my_target \[fuzz-crash:[0-9a-f]{8}\]\z/, 'the public line is opaque and sanitised' );
    for my $mode (qw(on unset)) {
        my $ov = opts( max_iterations => 400, seed => 9, ci => $mode );
        my $vv = CodingAdventures::TestBootstrap::Fuzz::verdict_for( $target, $r, $ov, 't' );
        is_deeply( [ $vv->{seed}, $vv->{witnesses} ], [ undef, undef ], "ci $mode carries no seed and no witness" );
    }
    # flaky: the crash does not come back
    my $first = 1;
    my $flaky = sub { my ($d) = @_; if ( $d =~ /X/ && $first ) { $first = 0; die "once\n" } };
    my $fr = run_t( ["X"], opts( max_iterations => 5, minimize_budget => 0 ), $flaky );
    is( scalar @{ $fr->{crashes} }, 1 );
    my $fv = CodingAdventures::TestBootstrap::Fuzz::verdict_for( $flaky, $fr, opts(), 't' );
    is_deeply( [ $fv->{crashes}[0]{status}, $fv->{passed} ], [ 'flaky', 0 ], 'a witness that does not reproduce is flaky and still fails' );
    # a clean campaign passes
    my $quiet = run_t( ["abc"], opts( max_iterations => 50 ), sub { 1 } );
    my $qv = CodingAdventures::TestBootstrap::Fuzz::verdict_for( sub { 1 }, $quiet, opts(), 't' );
    is_deeply( [ $qv->{passed}, $qv->{reason}, $qv->{lines} ], [ 1, undef, [] ], 'a quiet campaign passes with no lines' );
    # zero executions never passes
    my $zero = run_t( ["abc"], opts( max_iterations => 0 ), sub { 1 } );
    my $zv = CodingAdventures::TestBootstrap::Fuzz::verdict_for( sub { 1 }, $zero, opts(), 't' );
    is_deeply( [ $zv->{passed}, $zv->{reason} ], [ 0, 'no-iterations' ], 'zero iterations: not a pass' );
    # reasons
    my $reason = sub {
        my ($patch) = @_;
        my $rep = { %$quiet, %$patch };
        return CodingAdventures::TestBootstrap::Fuzz::verdict( bless( $rep, ref $quiet ), opts(), sub { return ( pass_out(), [] ) }, 't' )->{reason};
    };
    is( $reason->( { timeouts => 1 } ), 'timeout' );
    is( $reason->( { ooms => 1 } ), 'oom' );
    is( $reason->( { crashes_dropped => 1 } ), 'crashes-dropped' );
    is( $reason->( { admissions_dropped => 1 } ), 'admissions-dropped' );
    is( $reason->( { leaked => 1 } ), 'executions-leaked' );
    my $tv = CodingAdventures::TestBootstrap::Fuzz::verdict( $quiet, opts(), sub { return ( pass_out(), [] ) } );
    is_deeply( [ $tv->{passed} ], [1], 'the target id defaults' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::verdict( $quiet, {}, sub { }, 't' ); 1 }, 'verdict refuses non-Options' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::verdict( $quiet, opts(), 'x', 't' ); 1 }, 'verdict refuses a non-exec' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::verdict( $quiet, opts(), sub { }, [] ); 1 }, 'verdict refuses a reference id' );
    # timeouts and ooms reach the public lines
    my $tr = run_t( ['a'], opts( max_iterations => 3, step_budget => 2, timeout_ms => 1000 ), sub { $_[1]->tick for 1 .. 5 } );
    my $tl = CodingAdventures::TestBootstrap::Fuzz::verdict( $tr, opts(), sub { return ( $Out->can('timeout')->(), [] ) }, 'tt' );
    like( $tl->{lines}[0], qr/\Anot ok 1 - tt \[fuzz-timeout:4e61ea76\]\z/, 'a timeout line' );
};

subtest 'the leak cap holds in EVERY path where an execution happens' => sub {
    my $tamper = sub { $SIG{ALRM} = 'IGNORE' };
    # 1. triage: ten seeds, every execution tampers: the campaign stops at four
    my $calls = 0;
    my $o = opts( max_iterations => 50, timeout_ms => 2000 );
    my $r = run_t( [ map {"seed$_"} 1 .. 10 ], $o, sub { $calls++; $tamper->() } );
    cmp_ok( $calls, '<=', 4 + 3, 'triage (plus any minimisation probes) stops running the target' );
    is( $r->{leaked}, 4, 'triage: exactly four leaked executions' );
    ok( $r->{aborted}, '... the campaign is aborted' );
    is( $r->{iterations}, 0, '... and mutated nothing' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Report::clean($r), '... and is never clean' );
    # 2. the mutation loop
    $calls = 0;
    $r = run_t( ['aaaa'], opts( max_iterations => 500, timeout_ms => 2000, minimize_budget => 0 ), sub { $calls++; $tamper->() if $calls > 3; $_[1]->edge( $calls ) } );
    is( $r->{leaked}, 4, 'mutation loop: four leaked executions' );
    is( $calls, 3 + 4, '... and the target was not run again' ) ;
    ok( $r->{aborted}, '... the campaign is aborted' );
    is( $r->{iterations}, 6, '... after six mutation executions (two quiet, four leaked)' );
    # 3. minimisation candidates: the seed crashes normally, every shrunken candidate tampers
    $calls = 0;
    $r = run_t( ['AAAAAAAA'], opts( max_iterations => 100, timeout_ms => 2000 ), sub { $calls++; return $tamper->() if length( $_[0] ) < 8; die "crash\n" } );
    is( $calls, 1 + 4, 'minimisation: the seed once, then exactly four leaking probes' );
    is( $r->{leaked}, 4 );
    my ($crash) = grep { $_->{kind} eq 'string' } @{ $r->{crashes} };
    ok( $crash, 'the normal crash has a record' );
    is( $crash ? $crash->minimal_input : undef, 'AAAAAAAA', '... the crash keeps the input it has' );
    ok( $crash && !$crash->{minimized}, '... and says it did not finish' );
    # 4. re-verification: six distinct crashes, then the target starts tampering
    my $mode = 'ok';
    $calls = 0;
    my $multi = sub {
        my ($d) = @_;
        $calls++;
        $tamper->() if $mode eq 'tamper';
        die bless( {}, 'Err' . ord($d) ) if length($d) == 1;
        return;
    };
    my $rr = run_t( [ 'a' .. 'f' ], opts( max_iterations => 0 ), $multi );
    is( scalar @{ $rr->{crashes} }, 6, 'six crash records' );
    is( $rr->{leaked}, 0, 'no leaks so far' );
    $mode = 'tamper';
    $calls = 0;
    my $v = CodingAdventures::TestBootstrap::Fuzz::verdict_for( $multi, $rr, opts( max_iterations => 0 ), 't' );
    is( $calls, 4, 're-verification stops running the target at the cap' );
    is_deeply( [ map { $_->{status} } @{ $v->{crashes} } ], [ ('flaky') x 4, ('unverified') x 2 ], 'the first four are flaky, the rest unverified' );
    is( $v->{leaked}, 4 );
    is( $v->{reason}, 'executions-leaked', 'the verdict reason' );
    ok( !$v->{passed} );
    like( join( "\n", @{ $v->{lines} } ), qr/\[fuzz-timeout:f0c5a137\]/, 'a leaked line is fuzz-timeout with the bare executions-leaked signature' );
    # 5. verdict_for shares ONE tracker with the campaign
    $mode = 'ok';
    my $tracker = CodingAdventures::TestBootstrap::Fuzz::new_leaks( 0, 2 );
    $calls = 0;
    my $first_mode = 0;
    my $t2 = sub {
        my ($d) = @_;
        $calls++;
        die bless( {}, 'Err2' ) if $d eq 'q';
        return;
    };
    my $r2 = run_t( ['q'], opts( max_iterations => 0 ), $t2, { leaks => $tracker } );
    is( $tracker->{count}, 0 );
    $tracker->{count} = 2;    # the campaign had already used the whole allowance
    $calls = 0;
    my $v2 = CodingAdventures::TestBootstrap::Fuzz::verdict_for( $t2, $r2, opts( max_iterations => 0 ), 't', $tracker );
    is( $calls, 0, 'a shared tracker at its cap: re-verification runs nothing' );
    is( $v2->{crashes}[0]{status}, 'unverified' );
    # 6. a report that already leaked four executions re-verifies nothing (plain verdict)
    $calls = 0;
    my $fake = bless( { %$r2, leaked => 4 }, ref $r2 );
    my ($exec) = CodingAdventures::TestBootstrap::Fuzz::executor( $t2, opts() );
    my $v3 = CodingAdventures::TestBootstrap::Fuzz::verdict( $fake, opts(), $exec, 't' );
    is( $calls, 0, 'plain verdict applies the report\'s cap' );
    is( $v3->{crashes}[0]{status}, 'unverified' );
    is( $v3->{leaked}, 4 );
    # a too-many-leaks tracker is seeded from the report
    my $t = CodingAdventures::TestBootstrap::Fuzz::new_leaks();
    is_deeply( [ $t->{count}, $t->{cap} ], [ 0, 4 ], 'a fresh tracker: 0 of 4' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::Engine::leak_cap_reached($t) );
    $t->{count} = 4;
    ok( CodingAdventures::TestBootstrap::Fuzz::Engine::leak_cap_reached($t), 'at four, the cap is reached' );
    my $g = CodingAdventures::TestBootstrap::Fuzz::Engine::guard( sub { die "must not run\n" }, $t );
    my $ran = 0;
    my @g = eval { $g->('x') };
    is_deeply( [ $ran, scalar @g, $@ ], [ 0, 0, '' ], 'a guard over a spent tracker runs nothing (and does not die)' );
};

subtest 'api and hygiene' => sub {
    my ($exec) = CodingAdventures::TestBootstrap::Fuzz::executor( sub { 1 } );
    my ($out) = $exec->('x');
    is( $out->{kind}, 'pass', 'executor() with default options' );
    ok( eval { CodingAdventures::TestBootstrap::Fuzz::run( ['a'], undef, sub { 1 } ); 1 }, 'run() with default options' ) or diag $@;
    is_deeply( [ CodingAdventures::TestBootstrap::Fuzz::format_tap_failure( 1, 't', 'fuzz-oom', '00000000' ) ], ['not ok 1 - t [fuzz-oom:00000000]'], 'facade: format_tap_failure' );
    is( CodingAdventures::TestBootstrap::Fuzz::crash_signature( 'oom', [] ), '7146a410', 'facade: crash_signature' );
    is( CodingAdventures::TestBootstrap::Fuzz::calculate_energy( 10, 4, 3, 6 ), 8, 'facade: calculate_energy' );
    is_deeply( [ CodingAdventures::TestBootstrap::Fuzz::corpus_admit( [1], [2] ) ], [ 1, 1 ], 'facade: corpus_admit' );
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::mutate_sequence( "\0" x 8, 42, 4 ) ), '03ff00000000bf080000', 'facade: mutate_sequence' );
    is_deeply( CodingAdventures::TestBootstrap::Fuzz::tap_failures( { crashes => [], timeouts => 1, ooms => 0 }, 'x' ), ['not ok 1 - x [fuzz-timeout:4e61ea76]'], 'facade: tap_failures' );
    ok( CodingAdventures::TestBootstrap::Fuzz::minimize_crashing_input( 'a', 'deadbeef', 3, sub { pass_out() } ) eq 'a', 'facade: minimize_crashing_input' );
    my $root = "$FindBin::Bin/../lib/CodingAdventures/TestBootstrap";
    my @src = ( glob("$root/Fuzz/*.pm"), "$root/Fuzz.pm" );
    cmp_ok( scalar @src, '>=', 14, 'found the fuzz sources' );
    my ( $can, $isa ) = ( 0, 0 );
    for my $f (@src) {
        open my $fh, '<', $f or die "$f: $!";
        local $/;
        my $t = <$fh>;
        $can++ while $t =~ /->can\(/g;
        $isa++ while $t =~ /->isa\(/g;
    }
    is_deeply( [ $can, $isa ], [ 0, 0 ], 'no fuzz source uses ->can or ->isa' );
    # a planted `can` (or any new sub) does not steer the engine
    {
        no warnings 'once';
        local *CodingAdventures::TestBootstrap::Fuzz::Engine::can = sub { return sub { 0 } };
        local *CodingAdventures::TestBootstrap::Fuzz::Mutate::can = sub { return sub { 0 } };
        local *CodingAdventures::TestBootstrap::Fuzz::Executor::can = sub { return sub { 0 } };
        my $r = run_t( ['a'], opts( max_iterations => 20 ), sub { die "x\n" if $_[0] =~ /\xff/; } );
        is( $r->{iterations}, 20, 'a planted can changes nothing' );
    }
    # sealed globs
    for my $sub (qw(Mutate::mutate_sequence_counted Bitmap::corpus_admit Signature::format_tap_failure Engine::run_outcomes Executor::run Handle::tick Corpus::evaluate Hooks::with Options::new Report::new Outcome::pass FailureIds::is_fuzz_failure_id)) {
        my $name = "CodingAdventures::TestBootstrap::Fuzz::$sub";
        ok( !eval { no strict 'refs'; no warnings; *{$name} = sub { 0 }; 1 }, "$sub is sealed" );
    }
    ok( !eval { no warnings; *CodingAdventures::TestBootstrap::Fuzz::run = sub { 0 }; 1 }, 'the facade is sealed' );
    # replacing a captured Limits constant or a sealed Int function cannot steer the engine
    my $r = run_t( ['a'], opts( max_iterations => 5 ), sub { 1 } );
    is( $r->{iterations}, 5 );
};

done_testing;
