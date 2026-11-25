package CodingAdventures::TestBootstrap::Gens;

# The generator algebra (VER05 3, VER05e 3, VER05p section 1).
#
# DRAW ORDER IS THE COMPATIBILITY CONTRACT between language lanes: the pbt-v1
# vectors pin exact outputs per seed, so reordering a draw breaks conformance.
# Nested generators are length-bounded per level, not in total
# (`list(list(g, 0, n), 0, n)` can produce n x n elements); shrinking, by
# contrast, shares one work budget across all levels.
#
# Values: integers are Ints (u8, u64 and i128 included; a u64 above 2**63 - 1 is a
# Math::BigInt), booleans are 1 and 0, bytes are byte strings, lists and tuples are
# array references.

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Int    ();
use CodingAdventures::TestBootstrap::Prng   ();
use CodingAdventures::TestBootstrap::Limits ();
use CodingAdventures::TestBootstrap::Gen    ();
use CodingAdventures::TestBootstrap::Safe   ();
use CodingAdventures::TestBootstrap::Shrink ();

my $Int    = 'CodingAdventures::TestBootstrap::Int';
my $Limits = 'CodingAdventures::TestBootstrap::Limits';

my $TWO127   = CodingAdventures::TestBootstrap::Int::pow2(127);
my $TWO128   = CodingAdventures::TestBootstrap::Int::pow2(128);
our $MIN_I128 = CodingAdventures::TestBootstrap::Int::neg($TWO127);
our $MAX_I128 = CodingAdventures::TestBootstrap::Int::sub( $TWO127, 1 );
our $MAX_U64  = CodingAdventures::TestBootstrap::Int::sub( CodingAdventures::TestBootstrap::Int::two64(), 1 );

sub _gen { return CodingAdventures::TestBootstrap::Gen::new(@_) }

# The in-range point nearest zero: the integer shrink target.
sub clamp_zero {
    my ( $lo, $hi ) = @_;
    return $lo if CodingAdventures::TestBootstrap::Int::sign($lo) > 0;
    return $hi if CodingAdventures::TestBootstrap::Int::sign($hi) < 0;
    return 0;
}

sub _int_shrinker {
    my ( $lo, $hi ) = @_;
    my $target = clamp_zero( $lo, $hi );
    return sub {
        my ( $value, $falsifies, $max_steps ) = @_;
        return CodingAdventures::TestBootstrap::Shrink::shrink_int_core( $value, $target, $lo, $hi, $falsifies, $max_steps );
    };
}

# Always generates `$value` and draws nothing.
sub constant {
    my ($value) = @_;
    return _gen( sub { return $value } );
}

# bounded(2) == 1.  Shrinks true to false.
sub bool {
    return _gen(
        sub { return $_[0]->bounded(2) == 1 ? 1 : 0 },
        sub {
            my ( $value, $falsifies, $max_steps ) = @_;
            return ( 0, 1 ) if $value && $max_steps > 0 && $falsifies->(0);
            return ( $value, 0 );
        }
    );
}

# [0, 255]: bounded(256).
sub u8 { return _gen( sub { return $_[0]->bounded(256) }, _int_shrinker( 0, 255 ) ) }

# A full 64-bit word: next_u64.
sub u64 { return _gen( sub { return $_[0]->next_u64 }, _int_shrinker( 0, $MAX_U64 ) ) }

# Two stream words, high first, read as two's complement:
#   words 0x8000...0 : 0   -> -2**127  (top bit set => subtract 2**128)
#   words 0x7fff...f : f.. ->  2**127 - 1
sub sample_i128 {
    my ($rng) = @_;
    my $hi = $rng->next_u64;
    my $lo = $rng->next_u64;
    my $v  = CodingAdventures::TestBootstrap::Int::add( CodingAdventures::TestBootstrap::Int::shl( $hi, 64 ), $lo );
    return CodingAdventures::TestBootstrap::Int::sub( $v, $TWO128 ) if CodingAdventures::TestBootstrap::Int::cmp( $v, $TWO127 ) >= 0;
    return $v;
}

sub i128 { return _gen( \&sample_i128, _int_shrinker( $MIN_I128, $MAX_I128 ) ) }

sub _bounded_int {
    my ( $rng, $lo, $hi ) = @_;
    return CodingAdventures::TestBootstrap::Int::add( $lo, $rng->bounded_big( CodingAdventures::TestBootstrap::Int::add( CodingAdventures::TestBootstrap::Int::sub( $hi, $lo ), 1 ) ) );
}

# A uniform integer over the inclusive range [min, max], any width.  The offset from
# min is drawn by bounded_big (VER05 7.2).  Shrinks toward the in-range point nearest
# zero and never leaves [min, max].
sub integer {
    my ( $min, $max ) = @_;
    $min = CodingAdventures::TestBootstrap::Int::check( $min, 'min' );
    $max = CodingAdventures::TestBootstrap::Int::check( $max, 'max' );
    die "integer needs min <= max\n" if CodingAdventures::TestBootstrap::Int::cmp( $min, $max ) > 0;
    die "integer span wider than " . CodingAdventures::TestBootstrap::Limits::MAX_SPAN_BITS() . " bits\n"
      if CodingAdventures::TestBootstrap::Int::bit_length( CodingAdventures::TestBootstrap::Int::add( CodingAdventures::TestBootstrap::Int::sub( $max, $min ), 1 ) ) > CodingAdventures::TestBootstrap::Limits::MAX_SPAN_BITS();
    return _gen( sub { return _bounded_int( $_[0], $min, $max ) }, _int_shrinker( $min, $max ) );
}

sub _check_lengths {
    my ( $min_len, $max_len, $limit, $what ) = @_;
    $min_len = CodingAdventures::TestBootstrap::Int::check( $min_len, "$what min_len" );
    $max_len = CodingAdventures::TestBootstrap::Int::check( $max_len, "$what max_len" );
    die "$what needs 0 <= min_len <= max_len\n" if CodingAdventures::TestBootstrap::Int::is_big($min_len) || CodingAdventures::TestBootstrap::Int::is_big($max_len) || $min_len < 0 || $max_len < $min_len;
    die "$what max_len $max_len exceeds $limit\n" if $max_len > $limit;
    return ( $min_len, $max_len );
}

# A byte string: L = min_len + bounded(span), then L draws of bounded(256).
# Shrinks by sequence shrinking, respecting min_len.
sub bytes {
    my ( $min_len, $max_len ) = _check_lengths( @_[ 0, 1 ], CodingAdventures::TestBootstrap::Limits::MAX_INPUT_BYTES(), 'bytes' );
    my $span    = $max_len - $min_len + 1;
    my $sampler = sub {
        my ($rng) = @_;
        my $n = $min_len + $rng->bounded($span);
        my @list;
        push @list, $rng->bounded(256) for 1 .. $n;
        return CodingAdventures::TestBootstrap::Shrink::bytes_to_string( \@list );
    };
    my $shrinker = sub {
        my ( $value, $falsifies, $max_steps, $scope ) = @_;
        # The array conversion (and the string one on the way out) are copies too:
        # charge them first, and do nothing once the budget is spent.
        return ( $value, 0 ) if $max_steps == 0 || ( $scope && !$scope->charge_copy( 2 * length $value ) );
        my $wrapped = sub {
            return ( !$scope || $scope->charge_copy( scalar @{ $_[0] } ) ) && $falsifies->( CodingAdventures::TestBootstrap::Shrink::bytes_to_string( $_[0] ) );
        };
        my ( $list, $steps ) = CodingAdventures::TestBootstrap::Shrink::shrink_sequence(
            CodingAdventures::TestBootstrap::Shrink::string_to_bytes($value),
            $wrapped, $max_steps, $min_len, \&CodingAdventures::TestBootstrap::Shrink::byte_walk, $scope
        );
        return ( CodingAdventures::TestBootstrap::Shrink::bytes_to_string($list), $steps );
    };
    return _gen( $sampler, $shrinker );
}

# Transforms generated values.  Drops shrinking: the function has no inverse.
sub map {
    my ( $gen, $fn ) = @_;
    die "a code reference is required\n" if ref $fn ne 'CODE';
    return _gen( sub { return $fn->( $gen->sample( $_[0], $_[1] ) ) } );
}

# The predicate runs through Safe::call in SCALAR context (as a property does); a
# thrown value is rethrown to the caller, a non-local exit is an exhausted error.
sub _pred_holds {
    my ( $pred, $v ) = @_;
    my ( $st, $r ) = CodingAdventures::TestBootstrap::Safe::call( $pred, $v );
    die $r if $st eq 'died';
    die CodingAdventures::TestBootstrap::Gen::exhausted_error('a filter predicate left through a non-local exit') if $st eq 'escaped';
    return CodingAdventures::TestBootstrap::Safe::holds($r);
}

# Keeps only values for which `$pred` returns the plain 1.  After exactly
# `$max_retries` rejected samples sampling fails with an exhausted error.  Shrinking
# is preserved: a candidate failing the predicate is simply not a valid input.
sub filter {
    my ( $gen, $pred, $max_retries ) = @_;
    die "a predicate code reference is required\n" if ref $pred ne 'CODE';
    $max_retries = CodingAdventures::TestBootstrap::Limits::DEFAULT_MAX_RETRIES() if !defined $max_retries;
    CodingAdventures::TestBootstrap::Limits::check_range( $max_retries, 1, CodingAdventures::TestBootstrap::Limits::MAX_RETRIES(), 'max_retries' );
    my $sampler = sub {
        my ( $rng, $scope ) = @_;
        # Every retry inside a check is charged, so stacked filters (each retrying
        # 10,000 times) cannot multiply uncharged work: in a NESTED check a retry is
        # an attempt charged up the chain; in a top-level check it is one element of
        # work against the root's 64M allowance.
        my $nested = $scope && $scope->nested;
        for ( 1 .. $max_retries ) {
            if ($scope) {
                my $ok = $nested ? $scope->charge(1) : $scope->charge_copy(1);
                die CodingAdventures::TestBootstrap::Gen::exhausted_error('the enclosing shrink budget was spent while filtering') if !$ok;
            }
            my $v = $gen->sample( $rng, $scope );
            return $v if _pred_holds( $pred, $v );
        }
        die CodingAdventures::TestBootstrap::Gen::exhausted_error("filter rejected $max_retries samples");
    };
    my $shrinker = sub {
        my ( $value, $falsifies, $max_steps, $scope ) = @_;
        # Charge BEFORE the user predicate runs: it is property code and may be costly.
        return $gen->shrink_nested(
            $value,
            sub {
                return ( !$scope || $scope->charge(1) ) && _pred_holds( $pred, $_[0] ) && $falsifies->( $_[0] );
            },
            $max_steps, $scope
        );
    };
    return _gen( $sampler, $shrinker );
}

# Dependent generation: sample, then sample from the generator `$fn` returns.
sub flat_map {
    my ( $gen, $fn ) = @_;
    die "a code reference is required\n" if ref $fn ne 'CODE';
    return _gen( sub { return $fn->( $gen->sample( $_[0], $_[1] ) )->sample( $_[0], $_[1] ) } );
}

# Picks one generator uniformly (bounded(n), then that arm's draws).
sub one_of {
    my ($gens_in) = @_;
    my $arms = CodingAdventures::TestBootstrap::Safe::array_copy( $gens_in, 'one_of' );
    die "one_of needs at least one generator\n" if !@$arms;
    return _gen( sub { return $arms->[ $_[0]->bounded( scalar @$arms ) ]->sample( $_[0], $_[1] ) } );
}

# A weighted choice among `[weight, gen]` pairs: pick = bounded(total); arms are
# walked in order subtracting each weight, so a zero-weight arm can never be
# selected.  Empty, negative weight, zero total or total above 2**32 is refused.
sub frequency {
    my ($arms_in) = @_;
    my $in = CodingAdventures::TestBootstrap::Safe::array_copy( $arms_in, 'frequency' );
    die "frequency needs at least one arm\n" if !@$in;
    my ( $total, @owned ) = (0);
    for my $pair (@$in) {
        my $p = CodingAdventures::TestBootstrap::Safe::array_copy( $pair, 'a frequency arm' );
        my ( $weight, $gen ) = @$p;
        $weight = CodingAdventures::TestBootstrap::Limits::check_range( $weight, 0, CodingAdventures::TestBootstrap::Limits::MAX_FREQUENCY_TOTAL(), 'frequency weight' );
        $total += $weight;
        die "frequency total weight exceeds 2^32\n" if $total > CodingAdventures::TestBootstrap::Limits::MAX_FREQUENCY_TOTAL();
        push @owned, [ $weight, $gen ];
    }
    die "frequency total weight must be > 0\n" if $total == 0;
    return _gen(
        sub {
            my ( $rng, $scope ) = @_;
            my $pick = $rng->bounded($total);
            for my $arm (@owned) {
                return $arm->[1]->sample( $rng, $scope ) if $pick < $arm->[0];
                $pick -= $arm->[0];
            }
            die "unreachable: pick < total\n";
        }
    );
}

# A list: L = min_len + bounded(span), then L elements in order.  Shrinks by
# sequence shrinking with the element generator's shrinker for the element-wise
# phase.
sub list {
    my ( $elem, $min_len_in, $max_len_in ) = @_;
    my ( $min_len, $max_len ) = _check_lengths( $min_len_in, $max_len_in, CodingAdventures::TestBootstrap::Limits::MAX_LIST_LEN(), 'list' );
    my $span    = $max_len - $min_len + 1;
    my $sampler = sub {
        my ( $rng, $scope ) = @_;
        my $n = $min_len + $rng->bounded($span);
        my @out;
        push @out, $elem->sample( $rng, $scope ) for 1 .. $n;
        return \@out;
    };
    my $shrinker = sub {
        my ( $value, $falsifies, $max_steps, $scope ) = @_;
        my $shrink_elem = sub { return $elem->shrink_nested( $_[0], $_[1], $_[2], $_[3] ) };
        return CodingAdventures::TestBootstrap::Shrink::shrink_sequence( $value, $falsifies, $max_steps, $min_len, $shrink_elem, $scope );
    };
    return _gen( $sampler, $shrinker );
}

# One shrink-wide evaluation budget for a tuple: the repeat-until-no-progress loop
# calls each component shrinker once per pass, so without a shared counter the work
# is quadratic in max_steps.
sub _global_budget {
    my ( $falsifies, $max_steps ) = @_;
    my $budget = CodingAdventures::TestBootstrap::Limits::attempt_budget($max_steps);
    my $spent  = 0;
    return sub { $spent++; return $spent <= $budget && $falsifies->( $_[0] ) };
}

# Samples each generator left to right (an array) and shrinks one coordinate at a
# time, in repeated passes until a pass makes no progress.
sub tuple {
    my ($gens_in) = @_;
    my $arms = CodingAdventures::TestBootstrap::Safe::array_copy( $gens_in, 'tuple' );
    die "tuple needs at least one generator\n" if !@$arms;
    die "tuple arity exceeds " . CodingAdventures::TestBootstrap::Limits::MAX_TUPLE_ARITY() . "\n" if @$arms > CodingAdventures::TestBootstrap::Limits::MAX_TUPLE_ARITY();
    my $sampler = sub {
        my ( $rng, $scope ) = @_;
        return [ map { $_->sample( $rng, $scope ) } @$arms ];
    };
    my $shrinker = sub {
        my ( $value, $raw_falsifies, $max_steps, $scope ) = @_;
        my $falsifies = _global_budget( $raw_falsifies, $max_steps );
        my @current   = @{$value}[ 0 .. $#$arms ];
        my $steps     = 0;
        my $progress  = 1;
        while ( $progress && $steps < $max_steps ) {
            $progress = 0;
            for my $index ( 0 .. $#$arms ) {
                my $remaining = $max_steps - $steps;
                next if $remaining <= 0;
                my $slot = sub {
                    my @probe = @current;
                    $probe[$index] = $_[0];
                    return $falsifies->( \@probe );
                };
                my ( $shrunk, $used ) = $arms->[$index]->shrink_nested( $current[$index], $slot, $remaining, $scope );
                if ( $used > 0 ) {
                    $current[$index] = $shrunk;
                    $steps += $used;
                    $progress = 1;
                }
            }
        }
        return ( \@current, $steps );
    };
    return _gen( $sampler, $shrinker );
}

# -- corpus entry points (pbt-v1) -------------------------------------------

# `$count` primitives of `$type_name` ("bool", "u8", "u64", "i128") from `$seed`.
sub gen_primitive {
    my ( $seed, $type_name, $count ) = @_;
    $count = CodingAdventures::TestBootstrap::Limits::check_count($count);
    my $rng = CodingAdventures::TestBootstrap::Prng::new_rng($seed);
    my $sampler;
    if    ( $type_name eq 'bool' ) { $sampler = sub { return $_[0]->bounded(2) == 1 ? 1 : 0 } }
    elsif ( $type_name eq 'u8' )   { $sampler = sub { return $_[0]->bounded(256) } }
    elsif ( $type_name eq 'u64' )  { $sampler = sub { return $_[0]->next_u64 } }
    elsif ( $type_name eq 'i128' ) { $sampler = \&sample_i128 }
    else                           { die "unknown primitive type\n" }
    return [ map { $sampler->($rng) } 1 .. $count ];
}

# `$count` uniform integers over [lo, hi] from `$seed`.
sub gen_bounded {
    my ( $seed, $lo, $hi, $count ) = @_;
    $lo = CodingAdventures::TestBootstrap::Int::check( $lo, 'min' );
    $hi = CodingAdventures::TestBootstrap::Int::check( $hi, 'max' );
    die "need lo <= hi\n" if CodingAdventures::TestBootstrap::Int::cmp( $lo, $hi ) > 0;
    $count = CodingAdventures::TestBootstrap::Limits::check_count($count);
    # Each draw is about as wide as the span; count x width bounds what the returned
    # list can hold (a 1,000-digit span x 10**6 draws would not).
    my $width = int( ( CodingAdventures::TestBootstrap::Int::bit_length( CodingAdventures::TestBootstrap::Int::sub( $hi, $lo ) ) + 7 ) / 8 );
    $width = 1 if $width < 1;
    die "count $count x $width-byte span exceeds the generation budget\n" if $count * $width > CodingAdventures::TestBootstrap::Limits::MAX_GEN_BYTES_TOTAL();
    die "span wider than " . CodingAdventures::TestBootstrap::Limits::MAX_SPAN_BITS() . " bits\n"
      if CodingAdventures::TestBootstrap::Int::bit_length( CodingAdventures::TestBootstrap::Int::add( CodingAdventures::TestBootstrap::Int::sub( $hi, $lo ), 1 ) ) > CodingAdventures::TestBootstrap::Limits::MAX_SPAN_BITS();
    my $rng = CodingAdventures::TestBootstrap::Prng::new_rng($seed);
    return [ map { _bounded_int( $rng, $lo, $hi ) } 1 .. $count ];
}

# `$count` byte strings with length in [min_len, max_len] from `$seed`.
sub gen_bytes {
    my ( $seed, $min_len_in, $max_len_in, $count ) = @_;
    my ( $min_len, $max_len ) = _check_lengths( $min_len_in, $max_len_in, CodingAdventures::TestBootstrap::Limits::MAX_INPUT_BYTES(), 'gen_bytes' );
    $count = CodingAdventures::TestBootstrap::Limits::check_count($count);    # applies even when max_len == 0
    die "count $count x max_len $max_len exceeds the total of " . CodingAdventures::TestBootstrap::Limits::MAX_GEN_BYTES_TOTAL() . " bytes\n"
      if $max_len > 0 && $count > int( CodingAdventures::TestBootstrap::Limits::MAX_GEN_BYTES_TOTAL() / $max_len );
    my $gen = bytes( $min_len, $max_len );
    my $rng = CodingAdventures::TestBootstrap::Prng::new_rng($seed);
    return [ map { $gen->sample($rng) } 1 .. $count ];
}

1;
