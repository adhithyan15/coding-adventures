# Pinned values for the fuzz-v1 blind spots, cross-checked against the Ruby lane (VER06q
# section 9): signatures, TAP lines, energy, the rejection draw, the minimiser's candidate
# sequences and three whole campaigns.  See t/lib/FuzzPins.pm for provenance.
use strict;
use warnings;
use Test::More;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(bounded_by);
use FuzzPins;
use CodingAdventures::TestBootstrap;

my $Sig = 'CodingAdventures::TestBootstrap::Fuzz::Signature';
my $Out = 'CodingAdventures::TestBootstrap::Fuzz::Outcome';
sub unhex { return pack 'H*', $_[0] }
sub hex_of { return unpack 'H*', $_[0] }
sub I { return CodingAdventures::TestBootstrap::Int::parse( $_[0] ) }

subtest 'crash signatures (real sig_hash values)' => sub {
    for my $r (@FuzzPins::SIG) {
        is( CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( $r->[0], $r->[1] ), $r->[2], "$r->[0] @{$r->[1]} -> $r->[2]" );
    }
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( undef, [] ); 1 }, 'an undef kind is refused' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( 'k', 'x' ); 1 }, 'frames must be a list' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( 'k', [ [] ] ); 1 }, 'frames must be strings' );
    is( CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( 'k', ['x:y'] ), CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( 'k', [ 'x', 'y' ] ),
        'a colon is not escaped: one frame x:y and two frames x, y hash alike (VER06 9.4 item 6)' );
    ok( CodingAdventures::TestBootstrap::Fuzz::Signature::duplicate_crash( 'k', ['x:y'], 'k', [ 'x', 'y' ] ), '... so they are duplicates' );
};

subtest 'format_tap_failure: cross-lane pins from the Ruby lane' => sub {
    for my $r (@FuzzPins::TAP) {
        my ( $n, $hex, $want ) = @$r;
        my $target = unhex($hex);
        is( CodingAdventures::TestBootstrap::Fuzz::Signature::format_tap_failure( $n, $target, 'fuzz-crash', '3a8f19b2' ), $want, 'target ' . ( length($hex) > 24 ? substr( $hex, 0, 24 ) . '...' : $hex ) );
    }
    my $F = \&CodingAdventures::TestBootstrap::Fuzz::Signature::format_tap_failure;
    is( $F->( 1, "a\0b", 'fuzz-crash', '3a8f19b2' ), 'not ok 1 - a_b [fuzz-crash:3a8f19b2]', 'an embedded NUL' );
    is( $F->( 1, "caf\x{e9}", 'fuzz-crash', '3a8f19b2' ), 'not ok 1 - caf_ [fuzz-crash:3a8f19b2]', 'a latin-1 character string is one byte' );
    is( $F->( 1, "a\x{263a}", 'fuzz-crash', '3a8f19b2' ), 'not ok 1 - a___ [fuzz-crash:3a8f19b2]', 'a wide character is its three UTF-8 octets' );
    my $line = $F->( 1, "x\nnot ok 2 - forged\r# SKIP", 'fuzz-oom', '00000000' );
    unlike( $line, qr/[\r\n]/, 'no argument can add a TAP line' );
    unlike( $line, qr/ # /, 'and none can start a directive' );
    for my $bad ( [ 'fuzz-bogus', '3a8f19b2' ], [ 'fuzz-crash', '3A8F19B2' ], [ 'fuzz-crash', '3a8f19b' ], [ 'fuzz-crash', "3a8f19b2\n" ], [ 'fuzz-crash', undef ], [ undef, '3a8f19b2' ] ) {
        ok( !eval { $F->( 1, 't', @$bad ); 1 }, 'refused: ' . join( ' ', map { defined $_ ? $_ =~ s/\n/\\n/r : 'undef' } @$bad ) );
    }
    for my $n ( 0, 2147483648, 1.0, '1', undef, -1, I('9223372036854775808') ) {
        ok( !eval { $F->( $n, 't', 'fuzz-crash', '3a8f19b2' ); 1 }, 'a bad test number is refused: ' . ( defined $n ? "$n" : 'undef' ) );
    }
    ok( !eval { $F->( 1, [], 'fuzz-crash', '3a8f19b2' ); 1 }, 'a reference target is refused' );
    is( $F->( 2147483647, 't', 'fuzz-crash', '3a8f19b2' ), 'not ok 2147483647 - t [fuzz-crash:3a8f19b2]', 'the largest test number' );
    for my $id (qw(fuzz-crash fuzz-timeout fuzz-oom fuzz-leak fuzz-panic)) {
        ok( CodingAdventures::TestBootstrap::Fuzz::FailureIds::is_fuzz_failure_id($id), "$id is registered" );
    }
    ok( !CodingAdventures::TestBootstrap::Fuzz::FailureIds::is_fuzz_failure_id($_), 'not registered: ' . ( defined $_ ? $_ : 'undef' ) ) for ( 'fuzz-arbitrary-error', '', undef, 'Fuzz-crash' );
    ok( !CodingAdventures::TestBootstrap::Fuzz::FailureIds::is_fuzz_failure_id( ['fuzz-crash'] ), 'a reference is not registered' );
};

subtest 'tap_failures: crashes, then timeouts, ooms and leaked executions (bare-kind signatures)' => sub {
    my $rec = sub { return { signature => $_[0] } };
    my $report = { crashes => [ $rec->('aaaaaaaa'), $rec->('bbbbbbbb') ], timeouts => 2, ooms => 1, leaked => 3 };
    my $lines = CodingAdventures::TestBootstrap::Fuzz::Signature::tap_failures( $report, 'my/target', 5 );
    is_deeply( $lines, [
        'not ok 5 - my/target [fuzz-crash:aaaaaaaa]',
        'not ok 6 - my/target [fuzz-crash:bbbbbbbb]',
        'not ok 7 - my/target [fuzz-timeout:4e61ea76]',
        'not ok 8 - my/target [fuzz-oom:7146a410]',
        'not ok 9 - my/target [fuzz-timeout:f0c5a137]',
    ], 'in order, numbered from first_test' );
    is_deeply( CodingAdventures::TestBootstrap::Fuzz::Signature::tap_failures( { crashes => [], timeouts => 0, ooms => 0, leaked => 0 }, 't' ), [], 'a clean report has no lines' );
    is_deeply( CodingAdventures::TestBootstrap::Fuzz::Signature::tap_failures( { crashes => [], timeouts => 0, ooms => 0 }, 't', 1 ), [], 'a missing leaked count is zero' );
};

subtest 'frame and kind normalisation' => sub {
    my $nf = \&CodingAdventures::TestBootstrap::Fuzz::Signature::normalise_frame;
    is( $nf->( '/a/b/c.pl', 12 ), 'c.pl#12' );
    is( $nf->( 'C:\\x\\y.pl', 3 ), 'y.pl#3', 'a Windows path' );
    is( $nf->( '(eval 41)', 7 ), 'eval#7', 'the eval number is not part of the frame' );
    is( $nf->( "we ird:\n.pl", 1 ), 'we_ird__.pl#1', 'no colon, no space, no newline' );
    is( $nf->( 'a.pl', 'x:y' ), 'a.pl#0', 'a non-numeric line is 0' );
    is( length( $nf->( 'z' x 500 . '.pl', 1 ) ), 64 + 2, 'the basename is capped at 64' );
    my $nk = \&CodingAdventures::TestBootstrap::Fuzz::Signature::normalise_kind;
    is( $nk->('Foo::Bar'), 'Foo__Bar' );
    is( $nk->( 'k' x 100 ), 'k' x 64, 'capped at 64' );
    is( $nk->(undef), '<anonymous>' );
    is( $nk->( [] ), '<anonymous>' );
    my $c = CodingAdventures::TestBootstrap::Fuzz::Signature::collapse( [ 'a', 'a', 'b', 'b', 'a' ] );
    is_deeply( $c, [ 'a', 'b', 'a' ], 'consecutive duplicates collapse' );
    $c = CodingAdventures::TestBootstrap::Fuzz::Signature::collapse( [ map {"f$_"} 1 .. 100 ] );
    is( scalar @$c, 64, 'the innermost 64 survive' );
    is( $c->[0], 'f1', 'the innermost first' );
};

subtest 'calculate_energy: rows from the Ruby lane, including exact ties' => sub {
    for my $r (@FuzzPins::ENERGY) {
        is( CodingAdventures::TestBootstrap::Fuzz::Bitmap::calculate_energy( @{ $r->[0] } ), $r->[1], join( ',', @{ $r->[0] } ) . " -> $r->[1]" );
    }
    my $e = CodingAdventures::TestBootstrap::Fuzz::Bitmap::calculate_energy( 10, 4, 3, 6 );
    like( $e, qr/\A[0-9]+\z/, 'an integer' );
};

subtest 'the bitmap: reduction, admission, listing' => sub {
    my $reduce = \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::reduce;
    is( $reduce->(0), 0 );
    is( $reduce->(-1), 65535, '-1 is 65535' );
    is( $reduce->(65536), 0 );
    is( $reduce->(65537), 1 );
    is( $reduce->( I('-9223372036854775808') ), 0, 'the most negative int' );
    is( $reduce->( I('18446744073709551615') ), 65535, 'a big is reduced by mod' );
    is( $reduce->( I('-18446744073709551617') ), 65535, 'a negative big' );
    ok( !eval { $reduce->(1.5); 1 }, 'a float is refused' );
    ok( !eval { $reduce->('7'); 1 }, 'a numeric string is refused' );
    my $bm = CodingAdventures::TestBootstrap::Fuzz::Bitmap::new();
    my ( $new, $n ) = $bm->admit( [ 1, 2, 65537, 3 ] );
    is_deeply( [ $new, $n ], [ 1, 3 ], '65537 collides with 1' );
    is( $bm->count, 3 );
    ok( $bm->has(65538), 'has reduces too' );
    ok( !$bm->has(9) );
    ( $new, $n ) = $bm->admit( [ 1, 2 ] );
    is_deeply( [ $new, $n ], [ 0, 0 ], 'nothing new' );
    $bm->record(7);
    $bm->record(7);
    is_deeply( $bm->covered_edges, [ 1, 2, 3, 7 ], 'ascending' );
    like( "$bm", qr/edges=4/ );
    $bm->reset;
    is( $bm->count, 0, 'reset' );
    my $ca = \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::corpus_admit;
    is_deeply( [ $ca->( [ 1, 2, 3 ], [ 2, 4 ] ) ], [ 1, 1 ], 'one novel edge' );
    is_deeply( [ $ca->( [ 1, 2, 3 ], [ 1, 2 ] ) ], [ 0, 0 ] );
    is_deeply( [ $ca->( [], [ 5, 5, 5 ] ) ], [ 1, 1 ], 'unique novel edges' );
    is_deeply( [ $ca->( [ 1 ], [ 65537 ] ) ], [ 1, 1 ], 'admission is over the UNREDUCED ids' );
    is_deeply( [ $ca->( [ I('18446744073709551615') ], [ I('18446744073709551615'), I('18446744073709551614') ] ) ], [ 1, 1 ], 'bigs are keyed by their decimal text' );
    ok( !eval { $ca->( [1], [1.5] ); 1 }, 'a float edge is refused' );
};

subtest 'the rejection draw in bounded sampling (the corpus never reaches it)' => sub {
    my $n63 = I('9223372036854775809');
    my $t = CodingAdventures::TestBootstrap::Prng::threshold( CodingAdventures::TestBootstrap::Int::to_u64($n63) );
    is( $t, 9223372036854775807, '2^63 + 1: the threshold is 2^63 - 1, so about half the raw words are rejected' );
    ok( CodingAdventures::TestBootstrap::Prng::new(3)->next_word < $t, 'seed 3: the first raw word is BELOW the threshold, so the first draw is redrawn' );
    ok( CodingAdventures::TestBootstrap::Prng::new(4)->next_word < $t, 'seed 4: the same' );
    ok( !( CodingAdventures::TestBootstrap::Prng::new(0)->next_word < $t ), 'seed 0 is not rejected' );
    for my $r (@FuzzPins::DRAWS) {
        my ( $seed, $n, $want ) = @$r;
        my $rng = CodingAdventures::TestBootstrap::Prng::new( I($seed) );
        my @got = map { CodingAdventures::TestBootstrap::Int::tostring( $rng->bounded( I($n) ) ) } 1 .. 5;
        is_deeply( \@got, $want, "seed $seed, n = $n" );
    }
    my $first = join ',', map { CodingAdventures::TestBootstrap::Int::tostring( CodingAdventures::TestBootstrap::Prng::new($_)->next_u64 ) } 0, 1, 2, 42;
    is( $first, '16294208416658607535,10451216379200822465,10905525725756348110,13679457532755275413', 'the first words of seeds 0, 1, 2, 42' );
    # the engine draws through the sabotage seam: a shifted stream changes mutate_sequence
    my $base = CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence( "\0" x 8, 42, 4 );
    is( hex_of($base), '03ff00000000bf080000' );
    my $shifted = CodingAdventures::TestBootstrap::Hooks::with(
        { new_rng => sub { my $r = CodingAdventures::TestBootstrap::Prng::new( $_[0] ); $r->{state} = CodingAdventures::TestBootstrap::Prng::add64( $r->{state}, 1 ); return $r } },
        sub { CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence( "\0" x 8, 42, 4 ) } );
    isnt( $shifted, $base, 'a shifted stream changes the result' );
};

subtest 'the minimiser\'s candidate sequences (bisection halves), from the Ruby lane' => sub {
    my $SIG = CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature( 'k', [] );
    my $run = sub {
        my ( $hex, $budget, $crashes ) = @_;
        my @seen;
        my $test = sub { push @seen, hex_of( $_[0] ); return $crashes->( $_[0] ) ? $Out->can('crash')->( 'k', [] ) : $Out->can('pass')->() };
        my ( $out, $spent, $finished ) = CodingAdventures::TestBootstrap::Fuzz::Engine::minimize( unhex($hex), $SIG, $test, $budget );
        return ( \@seen, hex_of($out), $spent, $finished );
    };
    my ( $seen, $out, $spent, $fin ) = $run->( '004142434445464748', 1000, sub { index( $_[0], 'A' ) >= 0 } );
    is_deeply( $seen, [qw(4445464748 0041424348 00414243 4243 0041 41), '', '00'], 'a 9-byte input whose crash needs byte 41: halves 4, 2, 1 then the zeroing pass' );
    is_deeply( [ $out, $spent, $fin ], [ '41', 8, 1 ] );
    ( $seen, $out, $spent, $fin ) = $run->( '41', 1000, sub {1} );
    is_deeply( [ $seen, $out, $spent, $fin ], [ [''], '', 1, 1 ], 'a 1-byte crasher reaches the empty input (the first chunk is max(1, len/2))' );
    ( $seen, $out, $spent, $fin ) = $run->( '0a0b0c', 1000, sub { length( $_[0] ) >= 1 && ord( $_[0] ) == 0x0a } );
    is_deeply( [ $seen, $out, $spent, $fin ], [ [qw(0b0c 0a0c 0a 00)], '0a', 4, 1 ], 'a 3-byte input that needs its first byte: deletion then zeroing' );
    ( $seen, $out, $spent, $fin ) = $run->( '0a0b0c0d0e0f1011', 4, sub { length( $_[0] ) >= 1 && ord( $_[0] ) == 0x0a } );
    is_deeply( [ $seen, $out, $spent, $fin ], [ [qw(0e0f1011 0a0b0c0d 0c0d 0a0b)], '0a0b', 4, 0 ], 'a budget of 4 cuts the minimisation short and says so' );
    ( $seen, $out, $spent, $fin ) = $run->( '', 1000, sub {1} );
    is_deeply( [ $seen, $out, $spent, $fin ], [ [], '', 0, 1 ], 'an empty input executes nothing' );
    ( $seen, $out, $spent, $fin ) = $run->( '010203040506', 3, sub {0} );
    is_deeply( [ scalar @$seen, $spent, $fin ], [ 3, 3, 0 ], 'a rejected candidate counts against the budget just like an accepted one' );
    my @tried;
    ($out) = CodingAdventures::TestBootstrap::Fuzz::Engine::minimize( "\1\2\3", $SIG, sub { push @tried, 1; return $Out->can('crash')->( 'other', [] ) }, 100 );
    is( $out, "\1\2\3", 'a different crash is never accepted: the signature must match exactly' );
    ok( scalar @tried, '... but it was tried' );
    my $calls = 0;
    ( $out, $spent, $fin ) = CodingAdventures::TestBootstrap::Fuzz::Engine::minimize( "\1\2\3\4", $SIG, sub { $calls++; return undef }, 100 );
    is_deeply( [ $calls, $out, $spent, $fin ], [ 1, "\1\2\3\4", 1, 0 ], 'a test that returns nothing (the campaign was aborted) stops the minimiser' );
    $calls = 0;
    ( $out, $spent, $fin ) = CodingAdventures::TestBootstrap::Fuzz::Engine::minimize( "\1", $SIG, sub { $calls++; return $_[0] eq '' ? $Out->can('pass')->() : undef }, 100 );
    is_deeply( [ $calls, $fin ], [ 2, 0 ], '... in the zeroing pass too' );
    my $T = sub { $Out->can('pass')->() };
    my $mci = \&CodingAdventures::TestBootstrap::Fuzz::Engine::minimize_crashing_input;
    is( $mci->( "\1", $SIG, 5, $T ), "\1" );
    is( $mci->( "\1", $SIG, undef, $T ), "\1", 'the default budget' );
    ok( !eval { $mci->( "\1", $SIG, 5 ); 1 }, 'a test function is required' );
    ok( !eval { $mci->( undef, $SIG, 5, $T ); 1 }, 'data must be a string' );
    ok( !eval { $mci->( "\1", $SIG, -1, $T ); 1 }, 'a negative budget' );
    ok( !eval { $mci->( "\1", $SIG, 100001, $T ); 1 }, 'a budget above the ceiling' );
    ok( !eval { $mci->( "\1", $SIG, 1.5, $T ); 1 }, 'a fractional budget' );
    ok( !eval { $mci->( "x" x ( ( 1 << 20 ) + 1 ), $SIG, 5, $T ); 1 }, 'data above 1 MiB' );
};

subtest 'whole campaigns, driven by the same deterministic target as the Ruby lane' => sub {
    # edges from the length, the first byte and a long input; a timeout at 14 bytes, an oom when
    # the byte sum is 5 mod 97, crash "k" (frame "f") when the sum is 0 mod 31, crash "j" when
    # the second byte is 0x42 (identical in the Ruby script that produced the pins)
    my $exec = sub {
        my ($data) = @_;
        my $sum = 0;
        $sum += ord for split //, $data;
        my @edges = ( length($data) % 5, 10 + ( length($data) ? ord($data) : 0 ) % 6 );
        push @edges, 20 if length($data) > 5;
        return ( $Out->can('timeout')->(), \@edges ) if length($data) >= 14;
        return ( $Out->can('oom')->(), \@edges ) if $sum % 97 == 5;
        return ( $Out->can('crash')->( 'k', ['f'] ), \@edges ) if length($data) > 0 && $sum % 31 == 0;
        return ( $Out->can('crash')->( 'j', [] ), \@edges ) if length($data) >= 2 && ord( substr( $data, 1, 1 ) ) == 0x42;
        return ( $Out->can('pass')->(), \@edges );
    };
    my $i = 0;
    for my $camp (@FuzzPins::CAMPAIGNS) {
        $i++;
        my $o = CodingAdventures::TestBootstrap::Fuzz::Options->new( { max_iterations => $camp->{iters}, seed => I( $camp->{seed} ), max_input_len => $camp->{mil}, base_energy => 10, minimize_budget => 500 } );
        my $r = bounded_by( 60, sub { CodingAdventures::TestBootstrap::Fuzz::Engine::run_outcomes( [ map { unhex($_) } @{ $camp->{corpus} } ], $o, $exec ) } );
        is_deeply(
            [ @{$r}{qw(iterations admitted_seeds total_edges timeouts ooms crashes_dropped)} ],
            [ @{$camp}{qw(iterations admitted edges timeouts ooms dropped)} ],
            "campaign $i (seed $camp->{seed}): the counts are the Ruby lane's"
        );
        is( scalar @{ $r->{crashes} }, scalar @{ $camp->{crashes} }, "campaign $i: the same number of crash records" );
        for my $j ( 0 .. $#{ $camp->{crashes} } ) {
            my $rec = $r->{crashes}[$j];
            my $c   = $camp->{crashes}[$j];
            is_deeply( [ $rec->{signature}, $rec->{kind}, $rec->{hits}, hex_of( $rec->minimal_input ), $rec->{minimized} ], $c, "campaign $i: crash record $j" );
        }
        is_deeply( [ map { hex_of($_) } @{ CodingAdventures::TestBootstrap::Fuzz::Report::corpus($r) } ], $camp->{seeds}, "campaign $i: the admitted seeds, in order" );
    }
};

done_testing;
