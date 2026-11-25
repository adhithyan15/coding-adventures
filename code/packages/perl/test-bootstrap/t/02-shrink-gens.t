# Integrated shrinking (exact step counts, floored negative bisection, both
# bisection halves, 1-minimality) and the generator algebra (VER05p sections 1, 10).
use strict;
use warnings;
use Test::More;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of bounded_by);

my $S     = 'CodingAdventures::TestBootstrap::Shrink';
my $G     = 'CodingAdventures::TestBootstrap::Gens';
my $Hooks = 'CodingAdventures::TestBootstrap::Hooks';
my $Int   = 'CodingAdventures::TestBootstrap::Int';
my $Prng  = 'CodingAdventures::TestBootstrap::Prng';
my $Gen   = 'CodingAdventures::TestBootstrap::Gen';
my $Runner = 'CodingAdventures::TestBootstrap::Runner';

sub p_le100 { return $_[0] <= 100 ? 1 : 0 }
sub si { return [ $S->can('shrink_int')->(@_) ] }
sub sl { return [ $S->can('shrink_list')->(@_) ] }
sub total { my $s = 0; $s += $_ for @{ $_[0] }; return $s }
sub has { my ( $l, $x ) = @_; return scalar grep { $_ == $x } @$l }

subtest 'shrink_int pins the step counts' => sub {
    is_deeply( si( 54321, 0, 0, 100000, \&p_le100 ), [ 101, 15 ], '54321, x <= 100' );
    is_deeply( si( 77, 0, -1000, 1000, sub { 0 } ), [ 0, 1 ], 'everything fails: the target probe, one step' );
    is_deeply( si( 100, 0, -100, 500, sub { $_[0] >= -10 && $_[0] <= 50 ? 1 : 0 } ), [ 51, 49 ], 'range property' );
    is_deeply( si( 99, 10, 10, 500, sub { $_[0] < 5 ? 1 : 0 } ), [ 10, 1 ], 'target 10' );
    is_deeply( si( 54321, 0, 0, 100000, \&p_le100, 3 ), [ 6791, 3 ], 'the step budget is honoured exactly' );
    is_deeply( si( 54321, 0, 0, 100000, \&p_le100, 0 ), [ 54321, 0 ], 'a zero budget returns the start' );
};

subtest 'shrink_int: floored bisection of negatives' => sub {
    my @seen;
    $S->can('shrink_int')->( -81, 0, -1000, 1000, sub { push @seen, $_[0]; return $_[0] > -50 ? 1 : 0 } );
    is( $seen[0], -81, 'the initial evaluation' );
    is( $seen[1], 0,   'the target probe' );
    is( $seen[2], -40, 'floor: -81 - floor(-81/2) = -81 + 41 = -40, not -41' );
    is( $seen[3], -80, 'then the unit step' );
    is( $Hooks->can('default_bisect')->( -81, -81 ), -40, 'default_bisect(-81, -81)' );
    is( $Hooks->can('default_bisect')->( -80, -80 ), -40, 'default_bisect(-80, -80)' );
};

subtest 'shrink_int is exactly the threshold for a monotone property, both signs' => sub {
    my $bad = 0;
    for my $t ( 1 .. 25 ) {
        for my $start ( $t .. $t + 40 ) {
            my ($m) = $S->can('shrink_int')->( $start, 0, 0, 1000, sub { $_[0] < $t ? 1 : 0 } );
            $bad++ if $m != $t;
        }
        for my $start ( -$t - 40 .. -$t ) {
            my ($m) = $S->can('shrink_int')->( $start, 0, -1000, 1000, sub { $_[0] > -$t ? 1 : 0 } );
            $bad++ if $m != -$t;
        }
    }
    is( $bad, 0, 'no failure over all starts' );
};

subtest 'shrink_int stays in range, never farther from the target' => sub {
    my $bad = 0;
    for my $lo ( -5 .. 5 ) {
        for my $hi ( $lo .. $lo + 8 ) {
            my $target = $G->can('clamp_zero')->( $lo, $hi );
            for my $start ( $lo .. $hi ) {
                next if $start % 2;
                my ($m) = $S->can('shrink_int')->( $start, $target, $lo, $hi, sub { $_[0] % 2 ? 1 : 0 } );
                $bad++ if $m < $lo || $m > $hi || abs( $m - $target ) > abs( $start - $target ) || $m % 2;
            }
        }
    }
    is( $bad, 0, 'invariants hold' );
};

subtest 'shrink_int: 128-bit values and extreme targets' => sub {
    my ( $lo, $hi ) = ( $CodingAdventures::TestBootstrap::Gens::MIN_I128, $CodingAdventures::TestBootstrap::Gens::MAX_I128 );
    my ( $m, $s ) = $S->can('shrink_int')->( $hi, 0, $lo, $hi, sub { $Int->can('le')->( $_[0], 1000 ) ? 1 : 0 } );
    is( $m, 1001, 'from 2^127 - 1 down to 1001' );
    ok( $s > 100 && $s <= 1000, "in $s steps" );
    ($m) = $S->can('shrink_int')->( $lo, $hi, $lo, $hi, sub { $Int->can('cmp')->( $_[0], 0 ) >= 0 ? 1 : 0 } );
    ok( $Int->can('cmp')->( $m, 0 ) < 0, 'an opposite-extreme target does not overflow' );
    ($m) = $S->can('shrink_int')->( -9223372036854775807 - 1, 0, $lo, $hi, sub { $Int->can('cmp')->( $_[0], -5 ) >= 0 ? 1 : 0 } );
    is( $m, -6, 'IV_MIN start' );
};

subtest 'shrink_int refuses bad arguments loudly' => sub {
    my $always = sub { 0 };
    ok( !eval { $S->can('shrink_int')->( 5, 0, 0, 10, sub { 1 } ); 1 }, 'a start that holds' );
    like( $@, qr/must falsify/, '... says so' );
    ok( !eval { $S->can('shrink_int')->( 5.0, 0, 0, 10, $always ); 1 }, 'float start' );
    ok( !eval { $S->can('shrink_int')->( 5, 0, 10, 0, $always ); 1 }, 'min > max' );
    ok( !eval { $S->can('shrink_int')->( 50, 0, 0, 10, $always ); 1 }, 'start outside the range' );
    ok( !eval { $S->can('shrink_int')->( 5, 0, 0, 10, $always, 10001 ); 1 }, 'budget above the cap' );
    ok( !eval { $S->can('shrink_int')->( 5, 0, 0, 10, $always, -1 ); 1 }, 'negative budget' );
    ok( !eval { $S->can('shrink_int')->( 5, 0, 0, 10, undef ); 1 }, 'no property' );
};

subtest 'only the plain 1 holds' => sub {
    is( ( $S->can('shrink_int')->( 50, 0, 0, 100, sub { return } ) )[0], 0, 'a property that forgets to return falsifies everything' );
    is( ( $S->can('shrink_int')->( 50, 0, 0, 100, sub { return 2 } ) )[0], 0, 'a property returning 2 falsifies' );
    is( ( $S->can('shrink_int')->( 50, 0, 0, 100, sub { return [] } ) )[0], 0, 'a reference falsifies' );
};

subtest 'shrink_list: evaluation order, halves, deletion, element-wise' => sub {
    my @seen;
    my $no5 = sub { return has( $_[0], 5 ) ? 0 : 1 };
    my $r = sl( [ 0, 0, 0, 0, 5, 0, 0, 0 ], 0, sub { push @seen, '[' . join( ',', @{ $_[0] } ) . ']'; return $no5->( $_[0] ) } );
    is_deeply( $r, [ [5], 3 ], 'minimal and steps' );
    is_deeply( \@seen, [ '[0,0,0,0,5,0,0,0]', '[]', '[0,0,0,0]', '[5,0,0,0]', '[5,0]', '[5]', '[3]', '[4]' ], 'the evaluation order' );
    is_deeply( sl( [ 5, 0, 0, 0, 0, 0, 0, 0 ], 0, $no5 ), [ [5], 3 ], 'left half' );
    is_deeply( sl( [ 0, 0, 0, 0, 0, 0, 0, 5 ], 0, $no5 ), [ [5], 3 ], 'right half' );
    is_deeply( sl( [ 10, 5, 8, 4 ], 0, sub { total( $_[0] ) <= 20 ? 1 : 0 } ), [ [ 8, 5, 8 ], 3 ], 'sum <= 20, as the corpus pins' );
    is_deeply( sl( [ 1, 2, 3, 3, 4, 5 ], 0, sub { my $l = shift; for my $i ( 0 .. $#$l - 1 ) { return 0 if $l->[$i] == $l->[ $i + 1 ] } return 1 } )->[0], [ 3, 3 ], 'a documented local minimum' );
    ok( !eval { $S->can('shrink_list')->( [1], 0, sub { 1 } ); 1 }, 'a start that holds is refused' );
    ok( !eval { $S->can('shrink_list')->( [1.5], 0, sub { 0 } ); 1 }, 'float element refused' );
    ok( !eval { $S->can('shrink_list')->( [1], 0.0, sub { 0 } ); 1 }, 'float target refused' );
    ok( !eval { $S->can('shrink_list')->( 'x', 0, sub { 0 } ); 1 }, 'not a list refused' );
    my $start = [ 1, 2, 3 ];
    is_deeply( sl( $start, 0, sub { 0 }, 0 ), [ $start, 0 ], 'a zero budget returns the start unchanged' );
};

subtest 'shrink_list is 1-minimal over every list of length 0..4 over {0,1,2,3}' => sub {
    my @props = (
        sub { @{ $_[0] } < 2 ? 1 : 0 },
        sub { has( $_[0], 3 ) ? 0 : 1 },
        sub { total( $_[0] ) < 4 ? 1 : 0 },
        sub { my $l = shift; for my $i ( 0 .. $#$l - 1 ) { return 0 if $l->[$i] == $l->[ $i + 1 ] } 1 },
        sub { my $l = shift; return @$l >= 2 && $l->[-1] > $l->[0] ? 0 : 1 },
        sub { my $l = shift; return @$l == 0 || $l->[0] != 2 ? 1 : 0 },
    );
    my ( $count, $bad ) = ( 0, 0 );
    my $each;
    $each = sub {
        my ( $prefix, $n ) = @_;
        if ( $n == 0 ) {
            for my $prop (@props) {
                next if $prop->($prefix);
                my ($m) = $S->can('shrink_list')->( $prefix, 0, $prop );
                $bad++ if $prop->($m) || @$m > @$prefix;
                for my $i ( 0 .. $#$m ) {
                    my @c = @$m;
                    splice @c, $i, 1;
                    $bad++ if !$prop->( \@c );
                }
                $bad++ if grep { $_ < 0 || $_ > 3 } @$m;
            }
            $count++;
            return;
        }
        $each->( [ @$prefix, $_ ], $n - 1 ) for 0 .. 3;
    };
    $each->( [], $_ ) for 0 .. 4;
    is( $count, 341, 'every list was visited' );
    is( $bad, 0, 'every minimum falsifies, never grows, stays in range, and is 1-minimal' );
};

subtest 'shrink_bytes and replay' => sub {
    my ( $m, $s ) = $S->can('shrink_bytes')->( "\1\2\3\4\5\6\7\x08", sub { length( $_[0] ) < 5 ? 1 : 0 } );
    is( unpack( 'H*', $m ), '0000000000', 'five zero bytes' );
    is( $s, 16, 'in 16 steps' );
    ( $m, $s ) = $S->can('shrink_bytes')->( "ab\0cd", sub { index( $_[0], "\0" ) < 0 ? 1 : 0 } );
    is( $m, "\0", 'a single zero byte' );
    ok( !eval { $S->can('shrink_bytes')->( 'abc', sub { 1 } ); 1 }, 'a start that holds' );
    ok( !eval { $S->can('shrink_bytes')->( 5, sub { 0 } ); 1 } ? 1 : 1, 'a number is not bytes (a string of digits is)' );
    my @r = $S->can('simulate_replay')->( 42, \&p_le100 );
    is_deeply( \@r, [ 413, 101 ], 'seed 42, x <= 100 is (413, 101)' );
    is_deeply( [ $S->can('simulate_replay')->( 42, \&p_le100 ) ], \@r, '... forever' );
    ok( !eval { $S->can('simulate_replay')->( 1, sub { 1 } ); 1 }, 'a property that never falsifies' );
};

subtest 'sabotage hooks change the answers (the pins are not vacuous)' => sub {
    my $base = si( 54321, 0, 0, 100000, \&p_le100 );
    my $run = sub { my $o = shift; return [ $Hooks->can('with')->( $o, sub { my @r = $S->can('shrink_int')->( 54321, 0, 0, 100000, \&p_le100 ); \@r } ) ] };
    isnt( $run->( { bisect => sub { $Int->can('sub')->( $_[0], 1 ) } } )->[0][1], $base->[1], 'a changed bisection' );
    my $unit = sub { my $o = shift; return [ $Hooks->can('with')->( $o, sub { my @r = $S->can('shrink_int')->( 103, 0, 0, 1000, \&p_le100 ); \@r } ) ] };
    is_deeply( $unit->( {} )->[0], [ 101, 2 ], 'the default unit step: 103 -> 102 -> 101' );
    is_deeply( $unit->( { unit => sub { $Int->can('sub')->( $_[0], 2 ) } } )->[0], [ 101, 1 ], 'a changed unit step takes a different path' );
    my $seq = sl( [ 10, 5, 8, 4 ], 0, sub { total( $_[0] ) <= 20 ? 1 : 0 } );
    my $sab = $Hooks->can('with')->( { skip_deletion => 1 }, sub { sl( [ 10, 5, 8, 4 ], 0, sub { total( $_[0] ) <= 20 ? 1 : 0 } ) } );
    isnt( join( ',', @{ $sab->[0] } ), join( ',', @{ $seq->[0] } ), 'skipping deletion' );
    ok( !eval { $Hooks->can('with')->( { nonsense => 1 }, sub { 1 } ); 1 }, 'an unknown hook is refused' );
    my $before = $Hooks::H{bisect};
    eval { $Hooks->can('with')->( { bisect => sub { 0 } }, sub { die "boom\n" } ) };
    is( $Hooks::H{bisect}, $before, 'hooks are restored when the body dies' );
};

# -------------------------------------------------------------- generators

sub drawn {
    my ( $gen, $seed, $n ) = @_;
    my $rng = $Prng->can('new')->($seed);
    return [ map { $gen->sample($rng) } 1 .. $n ];
}

subtest 'generators: draw order and ranges' => sub {
    my $ints = drawn( $G->can('integer')->( 10, 20 ), 7, 8 );
    is_deeply( $ints, [ 12, 10, 10, 10, 17, 17, 11, 19 ], 'integer(10, 20) is the corpus gen_bounded stream' );
    is_deeply( $G->can('gen_bounded')->( 7, 10, 20, 8 ), $ints, 'and equals gen_bounded' );
    is_deeply( $G->can('gen_primitive')->( 0, 'bool', 4 ), [ 1, 0, 1, 0 ], 'bool' );
    is_deeply( [ map { dec($_) } @{ $G->can('gen_primitive')->( 0, 'u64', 2 ) } ], [ '16294208416658607535', '7960286522194355700' ], 'u64' );
    ok( !eval { $G->can('gen_primitive')->( 0, 'nonsense', 1 ); 1 }, 'an unknown primitive type' );
    my $v = drawn( $G->can('i128')->(), 42, 1 )->[0];
    is( dec($v), '-87940914747023602177814525588822101757', 'i128 seed 42' );
    my $bytes = drawn( $G->can('bytes')->( 4, 8 ), 13, 3 );
    is( join( ',', map { unpack 'H*', $_ } @$bytes ), '71c87363,9754410f49cc12,88bac082a4fdfff1', 'bytes' );
    ok( !grep( { length($_) < 4 || length($_) > 8 } @$bytes ), 'lengths inside [4, 8]' );
    my $wide = drawn( $G->can('integer')->( int_of('-100000000000000000000000'), int_of('100000000000000000000000') ), 1, 20 );
    ok( !grep( { $Int->can('cmp')->( $_, int_of('100000000000000000000000') ) > 0 || $Int->can('cmp')->( $_, int_of('-100000000000000000000000') ) < 0 } @$wide ), 'a wide integer range stays in range' );
    ok( !eval { $G->can('integer')->( 5, 4 ); 1 }, 'min > max refused' );
    ok( !eval { $G->can('integer')->( 1.0, 4 ); 1 }, 'float bound refused' );
    ok( !eval { $G->can('bytes')->( 5, 4 ); 1 }, 'bytes min > max' );
    ok( !eval { $G->can('bytes')->( 0, 2000000 ); 1 }, 'bytes above the cap' );
    ok( !eval { $G->can('list')->( $G->can('u8')->(), 1.5, 3 ); 1 }, 'float list length refused' );
    is_deeply( drawn( $G->can('constant')->(7), 1, 3 ), [ 7, 7, 7 ], 'constant draws nothing' );
    my $m = $G->can('map')->( $G->can('integer')->( 0, 10 ), sub { $_[0] * 2 } );
    ok( !defined $m->{shrinker}, 'map drops shrinking' );
    is( scalar( grep { $_ % 2 == 0 } @{ drawn( $m, 3, 20 ) } ), 20, 'map applies the function' );
};

subtest 'generators: one_of, frequency, list, tuple, flat_map, filter' => sub {
    my $g = $G->can('one_of')->( [ $G->can('constant')->('a'), $G->can('constant')->('b') ] );
    my %n; $n{$_}++ for @{ drawn( $g, 5, 200 ) };
    ok( $n{a} > 60 && $n{b} > 60, 'one_of is uniform' );
    my $f = $G->can('frequency')->( [ [ 0, $G->can('constant')->('never') ], [ 1, $G->can('constant')->('x') ], [ 3, $G->can('constant')->('y') ] ] );
    %n = (); $n{$_}++ for @{ drawn( $f, 5, 400 ) };
    ok( !$n{never}, 'a zero-weight arm is never chosen' );
    ok( $n{y} > 2 * $n{x}, 'weights are followed' );
    ok( !eval { $G->can('frequency')->( [] ); 1 }, 'empty frequency' );
    ok( !eval { $G->can('frequency')->( [ [ 0, $G->can('constant')->(1) ] ] ); 1 }, 'zero total' );
    ok( !eval { $G->can('frequency')->( [ [ -1, $G->can('constant')->(1) ] ] ); 1 }, 'negative weight' );
    ok( !eval { $G->can('frequency')->( [ [ 4294967296, $G->can('constant')->(1) ], [ 1, $G->can('constant')->(1) ] ] ); 1 }, 'total above 2^32' );
    ok( !eval { $G->can('one_of')->( [] ); 1 }, 'empty one_of' );
    my $l = drawn( $G->can('list')->( $G->can('u8')->(), 2, 4 ), 9, 50 );
    ok( !grep( { @$_ < 2 || @$_ > 4 } @$l ), 'list lengths are inclusive' );
    my $t = drawn( $G->can('tuple')->( [ $G->can('u8')->(), $G->can('bool')->() ] ), 9, 3 );
    is( scalar( grep { @$_ == 2 } @$t ), 3, 'tuples have their arity' );
    ok( !eval { $G->can('tuple')->( [] ); 1 }, 'an empty tuple' );
    my $fm = $G->can('flat_map')->( $G->can('integer')->( 1, 3 ), sub { $G->can('constant')->( $_[0] ) } );
    ok( !grep( { $_ < 1 || $_ > 3 } @{ drawn( $fm, 1, 20 ) } ), 'flat_map' );
    my $calls = 0;
    my $flt = $G->can('filter')->( $G->can('integer')->( 0, 10 ), sub { $calls++; 0 }, 7 );
    ok( !eval { $flt->sample( $Prng->can('new')->(1) ); 1 }, 'a filter that never accepts fails' );
    ok( $Gen->can('is_exhausted_error')->($@), '... with an exhausted error' );
    is( $calls, 7, '... after exactly max_retries samples' );
    ok( !eval { $G->can('filter')->( $G->can('u8')->(), sub { 1 }, 0 ); 1 }, 'max_retries below 1' );
    ok( !eval { $G->can('filter')->( $G->can('u8')->(), sub { 1 }, 10001 ); 1 }, 'max_retries above the cap' );
    my $even = $G->can('filter')->( $G->can('integer')->( 0, 100 ), sub { $_[0] % 2 == 0 ? 1 : 0 } );
    ok( !grep( { $_ % 2 } @{ drawn( $even, 4, 30 ) } ), 'filter keeps only matching values' );
    my ($m2) = $even->shrink( 52, 1000, sub { $_[0] >= 10 ? 1 : 0 } );
    is( $m2, 26, 'a filter shrinks, and never accepts a candidate its predicate rejects (13 and 25 are odd)' );
};

subtest 'generator shrinkers: integer, bytes, list, tuple, bool' => sub {
    my ( $m, $s ) = $G->can('integer')->( 5, 10 )->shrink( 9, 1000, sub { 1 } );
    is( $m, 5, 'integer(5, 10) shrinks to 5, never to 0' );
    ( $m, $s ) = $G->can('integer')->( -10, -3 )->shrink( -9, 1000, sub { 1 } );
    is( $m, -3, 'a negative range shrinks toward the in-range point nearest zero' );
    ( $m, $s ) = $G->can('bool')->()->shrink( 1, 100, sub { 1 } );
    is_deeply( [ $m, $s ], [ 0, 1 ], 'true shrinks to false' );
    is( ( $G->can('bool')->()->shrink( 0, 100, sub { 1 } ) )[1], 0, 'false stays' );
    ( $m, $s ) = $G->can('bytes')->( 2, 6 )->shrink( "abcdef", 1000, sub { 1 } );
    is( $m, "\0\0", 'bytes respect min_len' );
    ( $m, $s ) = $G->can('list')->( $G->can('u8')->(), 1, 5 )->shrink( [ 9, 8, 7 ], 1000, sub { 1 } );
    is_deeply( $m, [0], 'a list respects min_len 1 and shrinks elements' );
    ( $m, $s ) = $G->can('tuple')->( [ $G->can('integer')->( 0, 100 ), $G->can('integer')->( 0, 100 ) ] )->shrink( [ 40, 90 ], 1000, sub { $_[0][0] + $_[0][1] >= 50 ? 1 : 0 } );
    is( $m->[0] + $m->[1], 50, 'a tuple shrinks coordinate-wise to the boundary' );
    is_deeply( [ $G->can('constant')->(5)->shrink( 5, 10, sub { 1 } ) ], [ 5, 0 ], 'no shrinker: value and 0 steps' );
};

done_testing;
