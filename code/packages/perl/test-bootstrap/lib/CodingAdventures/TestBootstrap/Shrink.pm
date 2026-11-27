package CodingAdventures::TestBootstrap::Shrink;

# Integrated shrinking (VER05 4, VER05e 4, VER05p sections 1 and 5).
#
# A property that fails on [913, -4, 77, 0, 2] has told you nothing; one that
# fails on [0] has told you what is wrong.  Shrinking walks from the first failing
# candidate toward the smallest one that still fails.  It is "integrated" because
# the generator that made the value knows its own bounds, so a shrunk
# `Gens::integer(5, 10)` never becomes 0.
#
# The public shrinkers take a `$prop` code reference that returns the plain scalar
# 1 when the property HOLDS (the corpus convention) and return `($minimal, $steps)`,
# where steps counts ACCEPTED moves only.
#
# Integers here are `Int`s (native or big): every comparison and every piece of
# arithmetic goes through the Int module, and the bisection step is a FLOORED half
# (`-81` bisects to `-40`, not `-41`).

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Int    ();
use CodingAdventures::TestBootstrap::Hooks  ();
use CodingAdventures::TestBootstrap::Limits ();
use CodingAdventures::TestBootstrap::Budget ();
use CodingAdventures::TestBootstrap::Prng   ();
use CodingAdventures::TestBootstrap::Safe   ();

my $Int = 'CodingAdventures::TestBootstrap::Int';

use constant NOT_FALSIFYING => 'the initial value must falsify the property';

sub _in_bounds {
    my ( $value, $lo, $hi ) = @_;
    return ( !defined $lo || CodingAdventures::TestBootstrap::Int::le( $lo, $value ) ) && ( !defined $hi || CodingAdventures::TestBootstrap::Int::le( $value, $hi ) );
}

# A 0-based inclusive slice; empty when `$j < $i`.
sub _slice {
    my ( $a, $i, $j ) = @_;
    return [] if $j < $i;
    return [ @{$a}[ $i .. $j ] ];
}

sub string_to_bytes { return [ unpack 'C*', $_[0] ] }
sub bytes_to_string { return pack 'C*', @{ $_[0] } }

# Bisects then unit-steps `$current` toward `$target`.
#
# Each turn tries, in order,
#   1. the bisection candidate  current - floor(diff / 2)   when |diff| > 1,
#   2. the unit candidate       current -/+ 1 toward the target.
# A candidate is accepted only if inside [lo, hi] (undef = unbounded), different
# from current and still falsifying.  No acceptable candidate ends the walk.
#
# Worked example (corpus shrink_int/54321_le_100, property x <= 100):
# 54321 -> 27161 -> 13581 -> ... -> 101 in 15 accepted moves; 100 would satisfy the
# property, so neither candidate from 101 is accepted.
sub int_walk {
    my ( $current, $target, $lo, $hi, $falsifies, $budget ) = @_;
    my $steps = 0;
    while ( $steps < $budget && !CodingAdventures::TestBootstrap::Int::eq( $current, $target ) ) {
        my $diff  = CodingAdventures::TestBootstrap::Int::sub( $current, $target );
        my $moved = 0;
        if ( CodingAdventures::TestBootstrap::Int::cmp( CodingAdventures::TestBootstrap::Int::abs($diff), 1 ) > 0 ) {
            my $cand = $CodingAdventures::TestBootstrap::Hooks::H{bisect}->( $current, $diff );
            if ( !CodingAdventures::TestBootstrap::Int::eq( $cand, $current ) && _in_bounds( $cand, $lo, $hi ) && $falsifies->($cand) ) {
                ( $current, $moved ) = ( $cand, 1 );
                $steps++;
            }
        }
        if ( !$moved ) {
            my $cand = $CodingAdventures::TestBootstrap::Hooks::H{unit}->( $current, $diff );
            if ( !CodingAdventures::TestBootstrap::Int::eq( $cand, $current ) && _in_bounds( $cand, $lo, $hi ) && $falsifies->($cand) ) {
                ( $current, $moved ) = ( $cand, 1 );
                $steps++;
            }
        }
        last if !$moved;
    }
    return ( $current, $steps );
}

# The target probe followed by the walk (VER05 4.2).
sub shrink_int_core {
    my ( $initial, $target, $lo, $hi, $falsifies, $max_steps ) = @_;
    return ( $initial, 0 ) if $max_steps == 0;    # no budget, no shrinking
    # the target itself fails: cannot do better
    return ( $target, 1 )
      if _in_bounds( $target, $lo, $hi ) && !CodingAdventures::TestBootstrap::Int::eq( $target, $initial ) && $falsifies->($target);
    return int_walk( $initial, $target, $lo, $hi, $falsifies, $max_steps );
}

# A byte value shrunk toward 0 by the integer walk.
sub byte_walk {
    my ( $value, $falsifies, $budget ) = @_;
    return int_walk( $value, 0, 0, 255, $falsifies, $budget );
}

# Shrinks a falsifying sequence (an array reference) (VER05 4.3).  `$falsifies`
# always receives a FRESH array it may keep or mutate; the ELEMENTS are not cloned.
#
#   probe        min_len == 0: does the empty list fail?  Then it is the answer,
#                one step.  (min_len > 0: probe the length-min_len prefix and go on.)
#   bisection    keep the left half, else the right half, while one fails.
#   deletion     single elements, right to left.
#   element-wise left to right via `$shrink_elem->($value, $falsifies, $steps,
#                $scope)` (undef means none).
#
# Worked example (corpus list_sum_le_20, [10 5 8 4]): the empty list and both halves
# satisfy the property; deleting index 4 leaves [10 5 8] (sum 23, still failing);
# element-wise 10 -> 8 gives the 1-minimal [8 5 8].
#
# Every candidate is CHARGED (its copy) to the budget tree BEFORE it is built; once
# refused, every loop ends.  When max_steps == 0 or the budget is already spent the
# input is returned as is (it may be the same array).  `$scope` is joined; with no
# scope a fresh root is used for the call.
sub shrink_sequence {
    my ( $initial, $falsifies, $max_steps, $min_len, $shrink_elem, $scope ) = @_;
    return ( $initial, 0 ) if $max_steps == 0;
    my $own;
    if ( !defined $scope ) {
        $own   = CodingAdventures::TestBootstrap::Budget::open_child( undef, $max_steps );
        $scope = $own->scope;
    }
    my $size = scalar @$initial;
    return ( $initial, 0 ) if !$scope->charge_copy($size);
    my $current = [@$initial];
    my $steps   = 0;

    if ( @$current > $min_len && $scope->charge_copy($min_len) && $falsifies->( _slice( $current, 0, $min_len - 1 ) ) ) {
        return ( [], 1 ) if $min_len == 0;
        $current = _slice( $current, 0, $min_len - 1 );
        $steps   = 1;
    }

    my $floor = $min_len > 1 ? $min_len : 1;

    # Phase 1: bisection (left half first, then right half).
    while ( @$current > $floor && $steps < $max_steps ) {
        my $mid = int( @$current / 2 );
        if ( $mid >= $min_len && $scope->charge_copy($mid) && $falsifies->( _slice( $current, 0, $mid - 1 ) ) ) {
            $current = _slice( $current, 0, $mid - 1 );
            $steps++;
        }
        else {
            my $right_len = @$current - $mid;
            if ( $right_len >= $min_len && $scope->charge_copy($right_len) && $falsifies->( _slice( $current, $mid, $#$current ) ) ) {
                $current = _slice( $current, $mid, $#$current );
                $steps++;
            }
            else {
                last;
            }
        }
    }

    # Phase 2: single-element deletion, right to left.
    if ( !$CodingAdventures::TestBootstrap::Hooks::H{skip_deletion} ) {
        my $i = $#$current;
        while ( $i >= 0 && @$current > $floor && $steps < $max_steps && $scope->room ) {
            last if !$scope->charge_copy( @$current - 1 );    # charge BEFORE copying
            my @cand = @$current;
            splice @cand, $i, 1;
            if ( $falsifies->( \@cand ) ) {
                $current = \@cand;
                $steps++;
                $i = $#$current if $i > $#$current;
            }
            else {
                $i--;
            }
        }
    }

    # Phase 3: element-wise, left to right.
    if ($shrink_elem) {
        my $count = @$current;
        for my $index ( 0 .. $count - 1 ) {
            my $remaining = $max_steps - $steps;
            last if $remaining <= 0 || !$scope->room;
            my $at           = $index;
            my $with_element = sub {
                my ($candidate) = @_;
                return 0 if !$scope->charge_copy( scalar @$current );
                my @probe = @$current;
                $probe[$at] = $candidate;
                return $falsifies->( \@probe );
            };
            my ( $value, $used ) = $shrink_elem->( $current->[$index], $with_element, $remaining, $scope );
            if ( $used > 0 ) {
                $current->[$index] = $value;
                $steps += $used;
            }
        }
    }

    return ( $current, $steps );
}

sub _check_prop {
    die "a property code reference is required\n" if ref $_[0] ne 'CODE';
}

# Calls a property from a PUBLIC entry point: a thrown value propagates to the
# caller, a non-local exit is an error.
sub _call_prop {
    my ( $prop, $v, $scope ) = @_;
    my ( $st, $val ) = CodingAdventures::TestBootstrap::Safe::call( $prop, $v, $scope );
    die $val if $st eq 'died';
    die "the property left through a non-local exit\n" if $st eq 'escaped';
    return $val;
}

sub _holds { return CodingAdventures::TestBootstrap::Safe::holds( $_[0] ) }

# Shrinks a falsifying integer toward `$target` inside [lo, hi] (the corpus calls
# these min and max).  `$prop->($x)` is the plain 1 when the property holds.  Dies
# when `$initial` satisfies it, for lo > hi, an initial outside the bounds or a
# budget outside [0, MAX_SHRINK_STEPS].  Returns `($minimal, $steps)`.
sub shrink_int {
    my ( $initial, $target, $lo, $hi, $prop, $max_steps, $parent ) = @_;
    _check_prop($prop);
    $max_steps = 1000 if !defined $max_steps;
    $initial = CodingAdventures::TestBootstrap::Int::check( $initial, 'initial' );
    $target  = CodingAdventures::TestBootstrap::Int::check( $target,  'target' );
    $lo      = CodingAdventures::TestBootstrap::Int::check( $lo,      'min' );
    $hi      = CodingAdventures::TestBootstrap::Int::check( $hi,      'max' );
    CodingAdventures::TestBootstrap::Limits::check_budget($max_steps);
    die "min exceeds max\n" if CodingAdventures::TestBootstrap::Int::cmp( $lo, $hi ) > 0;
    die "initial value must lie within [min, max]\n" if !_in_bounds( $initial, $lo, $hi );
    my $own   = CodingAdventures::TestBootstrap::Budget::open_child( $parent, $max_steps );
    my $scope = $own->scope;
    local $CodingAdventures::TestBootstrap::Budget::CURRENT = $scope;
    return ( $initial, 0 ) if !$scope->charge(1);    # starved by an enclosing shrink
    die NOT_FALSIFYING . "\n" if _holds( _call_prop( $prop, $initial, $scope ) );
    my $falsifies = sub { return $scope->charge(1) && !_holds( _call_prop( $prop, $_[0], $scope ) ) };
    return shrink_int_core( $initial, $target, $lo, $hi, $falsifies, $max_steps );
}

# Shrinks a falsifying list of integers: empty probe, halves, single deletion, then
# each element toward `$elem_target` (unbounded).
sub shrink_list {
    my ( $initial, $elem_target, $prop, $max_steps, $parent ) = @_;
    _check_prop($prop);
    $max_steps = 1000 if !defined $max_steps;
    die "initial must be a list\n" if ref $initial ne 'ARRAY';
    $elem_target = CodingAdventures::TestBootstrap::Int::check( $elem_target, 'elem_target' );
    my @items = map { CodingAdventures::TestBootstrap::Int::check( $_, 'element' ) } @$initial;
    CodingAdventures::TestBootstrap::Limits::check_budget($max_steps);
    my $own   = CodingAdventures::TestBootstrap::Budget::open_child( $parent, $max_steps );
    my $scope = $own->scope;
    local $CodingAdventures::TestBootstrap::Budget::CURRENT = $scope;
    # charged before the first copies and for the initial evaluation
    return ( $initial, 0 ) if !$scope->charge_copy( 2 * @items ) || !$scope->charge(1);
    die NOT_FALSIFYING . "\n" if _holds( _call_prop( $prop, [@items], $scope ) );
    my $walk      = sub { return int_walk( $_[0], $elem_target, undef, undef, $_[1], $_[2] ) };
    my $falsifies = sub { return $scope->charge(1) && !_holds( _call_prop( $prop, $_[0], $scope ) ) };
    return shrink_sequence( \@items, $falsifies, $max_steps, 0, $walk, $scope );
}

# Shrinks falsifying bytes (a byte string): sequence shrinking over byte values
# toward 0x00.
sub shrink_bytes {
    my ( $initial, $prop, $max_steps, $parent ) = @_;
    _check_prop($prop);
    $max_steps = 1000 if !defined $max_steps;
    die "initial must be a string\n" if !defined $initial || ref $initial;
    CodingAdventures::TestBootstrap::Limits::check_budget($max_steps);
    my $own   = CodingAdventures::TestBootstrap::Budget::open_child( $parent, $max_steps );
    my $scope = $own->scope;
    local $CodingAdventures::TestBootstrap::Budget::CURRENT = $scope;
    # the copy, the array conversion and the string conversion back
    return ( $initial, 0 ) if !$scope->charge_copy( 3 * length $initial ) || !$scope->charge(1);
    die NOT_FALSIFYING . "\n" if _holds( _call_prop( $prop, $initial, $scope ) );
    # each attempt also converts its candidate back to a string: charge that copy too
    my $falsifies = sub {
        return $scope->charge_copy( scalar @{ $_[0] } ) && $scope->charge(1) && !_holds( _call_prop( $prop, bytes_to_string( $_[0] ), $scope ) );
    };
    my ( $list, $steps ) = shrink_sequence( string_to_bytes($initial), $falsifies, $max_steps, 0, \&byte_walk, $scope );
    return ( bytes_to_string($list), $steps );
}

# Replays a failure from nothing but `$seed`: draw up to 1000 values bounded(1000);
# the first that falsifies is the initial failure, then shrunk toward 0 over
# [0, 1000].  Same seed, same pair, forever.  Returns `($initial_fail, $minimal)`.
sub simulate_replay {
    my ( $seed, $prop, $parent ) = @_;
    _check_prop($prop);
    my $own   = CodingAdventures::TestBootstrap::Budget::open_child( $parent, 1000 );
    my $scope = $own->scope;
    local $CodingAdventures::TestBootstrap::Budget::CURRENT = $scope;
    my $rng   = CodingAdventures::TestBootstrap::Prng::new_rng($seed);
    my $first;
    for ( 1 .. 1000 ) {
        last if !$scope->charge(1);
        my $v = $rng->bounded(1000);
        if ( !_holds( _call_prop( $prop, $v, $scope ) ) ) {
            $first = $v;
            last;
        }
    }
    die "property was not falsified within 1000 draws\n" if !defined $first;
    my ($minimal) = shrink_int( $first, 0, 0, 1000, $prop, 1000, $scope );
    return ( $first, $minimal );
}

1;
