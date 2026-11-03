package CodingAdventures::Rng;

# ============================================================================
# CodingAdventures::Rng — Three classic pseudorandom number generators
# ============================================================================
#
# This module implements LCG, Xorshift64, and PCG32 — three well-known PRNGs
# that together illustrate the evolution of random number generation from 1948
# to 2014.
#
# ## The Three Algorithms
#
#   LCG (Linear Congruential Generator, Knuth 1948)
#   ─────────────────────────────────────────────────
#   The simplest useful PRNG. State advances via:
#
#     state = (state × a + c) mod 2^64
#
#   Output is the upper 32 bits. Full period 2^64. Fast but consecutive
#   outputs are correlated.
#
#   Xorshift64 (Marsaglia 2003)
#   ────────────────────────────
#   Three XOR-shift operations on 64-bit state. No multiplication. Period
#   2^64 − 1. State 0 is a fixed point; seed 0 → replaced with 1.
#   Output is the lower 32 bits.
#
#   PCG32 (O'Neill 2014)
#   ──────────────────────
#   Same LCG recurrence plus XSH RR output permutation. Passes all known
#   statistical test suites with only 8 bytes of state.
#
# ## Perl Integer Arithmetic Notes
#
# Perl uses IEEE 754 doubles (53-bit mantissa) for its default "number" type.
# This is insufficient for 64-bit integer arithmetic: multiplying two 32-bit
# values can produce a 64-bit result that would lose precision in a double.
#
# Solution: We use Math::BigInt only where needed — specifically for the LCG
# multiply step. Once we have the lower 64 bits, we convert back to a native
# Perl integer (UV, unsigned integer) for all other operations.
#
# Actually, on 64-bit Perl builds, the native UV (unsigned value) IS 64 bits.
# We can use Perl's bitwise operators on native integers so long as we mask
# explicitly to prevent sign-extension issues:
#
#   & 0xFFFFFFFFFFFFFFFF  — keep low 64 bits (u64 mask)
#   & 0xFFFFFFFF          — keep low 32 bits (u32 mask)
#
# For multiplication: we use Math::BigInt to compute the full 128-bit product,
# then take the low 64 bits with ->band('0xFFFFFFFFFFFFFFFF').
#
# ## Classes
#
#   CodingAdventures::Rng::LCG          — Linear Congruential Generator
#   CodingAdventures::Rng::Xorshift64   — Marsaglia XOR-shift
#   CodingAdventures::Rng::PCG32        — Permuted Congruential Generator
#
# ## Usage
#
#   use CodingAdventures::Rng;
#
#   my $lcg = CodingAdventures::Rng::LCG->new(42);
#   print $lcg->next_u32();              # uint32
#   print $lcg->next_float();            # float in [0.0, 1.0)
#   print $lcg->next_int_in_range(1, 6); # integer in [1, 6]
#
# ============================================================================

use strict;
use warnings;
use Carp;
use Math::BigInt;

our $VERSION = '0.02';

# ============================================================================
# CONSTANTS
# ============================================================================

# These constants satisfy the Hull-Dobell theorem for full-period 2^64.
# Written as strings so Math::BigInt parses them without floating-point error.
use constant LCG_MULTIPLIER => '6364136223846793005';
use constant LCG_INCREMENT  => '1442695040888963407';

# FLOAT_DIV: divisor to convert a uint32 to [0.0, 1.0)
use constant FLOAT_DIV => 4_294_967_296.0;   # 2^32

# U64_MASK as a BigInt string
use constant U64_MASK_STR => '18446744073709551615';   # 0xFFFFFFFFFFFFFFFF
use constant U32_MASK      => 0xFFFFFFFF;

# ============================================================================
# INTERNAL HELPERS
# ============================================================================

# _bigint(n): convert a value to a Math::BigInt.
# Accepts plain integers, strings, or BigInt objects.
sub _bigint {
    my ($n) = @_;
    return Math::BigInt->new("$n");
}

# _u64_mask: BigInt mask for 64-bit truncation.
my $_U64_MASK = Math::BigInt->new(U64_MASK_STR);

# _lcg_advance(state_bigint): apply one LCG step, return new BigInt state.
#
#   new_state = (state × a + c) mod 2^64
#
# We use Math::BigInt for the multiply to avoid double precision loss, then
# mask to 64 bits.
sub _lcg_advance {
    my ($state) = @_;
    my $a = Math::BigInt->new(LCG_MULTIPLIER);
    my $c = Math::BigInt->new(LCG_INCREMENT);
    my $new = ($state * $a + $c)->band($_U64_MASK);
    return $new;
}

# _bigint_to_u32(bigint): extract the lower 32 bits as a plain Perl integer.
sub _bigint_to_u32 {
    my ($b) = @_;
    return ($b->band(U32_MASK))->numify();
}

# _bigint_shr(bigint, n): logical right-shift by n bits, return BigInt.
sub _bigint_shr {
    my ($b, $n) = @_;
    return $b->copy()->brsft($n);
}

# _rotr32(v, rot): rotate 32-bit value v right by rot positions.
#
# A rotation wraps bits that fall off the right back onto the left. For 32-bit:
#
#   rotr32(v, r) = (v >> r) | (v << (32 - r)), masked to 32 bits
#
# v and rot are plain Perl integers here (u32 values).
sub _rotr32 {
    my ($v, $rot) = @_;
    $rot = $rot & 31;   # ensure rotation in [0, 31]
    return (($v >> $rot) | (($v << (32 - $rot)) & U32_MASK)) & U32_MASK;
}

# _rejection_sample(next_u32_fn, min, max): a uniform integer in [min, max]
# (spec rng.md, "Rejection Sampling", v2), identical in every language.
#
# span = max - min + 1 lies between 1 and 2^64. Naïve r % span over-samples
# low values whenever the number of possible draws is not a multiple of span.
# Example: span = 3, 2^32 = 4294967296 = 3 * 1431655765 + 1, so offset 0
# would be drawn once more than 1 and 2. Rejecting the `threshold` lowest
# draws removes exactly the excess. Three regimes, chosen by span alone:
#
#   span <= 2^32    draw 32 bits,  threshold = (2^32 - span) % span
#   span <  2^64    draw 64 bits,  threshold = (2^64 - span) % span
#   span == 2^64    the whole signed 64-bit domain: every u64 is valid
#
# The 32-bit regime fits native integers exactly. Wider spans use
# Math::BigInt: Perl's native arithmetic silently turns into floating point
# when a 64-bit integer operation overflows, which would change the answer.
#
# Bounds must be integers (native, decimal strings or Math::BigInt) inside the
# signed 64-bit domain, with min <= max; anything else croaks before any draw.
# v1 masked the span to 32 bits and drew 32 bits for every span, so a range
# wider than 2^32 could never return more than 2^32 - 1 above min.
my $_I64_MIN = Math::BigInt->new('-9223372036854775808');
my $_I64_MAX = Math::BigInt->new('9223372036854775807');
my $_TWO_32  = Math::BigInt->new('4294967296');
my $_TWO_64  = Math::BigInt->new('18446744073709551616');

sub _rejection_sample {
    my ($next_u32_fn, $min, $max) = @_;
    # Each bound is stringified exactly once, and that one string is both
    # checked and parsed: an object with "" overloading could otherwise pass
    # the check with one string and be parsed from another.
    my @text;
    for my $bound ($min, $max) {
        my $string = defined $bound ? "$bound" : '';
        # Two checks, because a float's string form is rounded to 15
        # significant digits: 0.9999999999999999 prints as "1" and would pass
        # the pattern alone. The second compares the number with its own
        # integer part, which differs for any non-integer float. (Comparing
        # with Math::BigInt does not work: its constructor stringifies too.)
        Carp::croak('next_int_in_range bounds must be integers')
            unless $string =~ /\A-?[0-9]+\z/
            && (ref($bound) || $bound == int($bound));
        push @text, $string;
    }
    my ($lo, $hi) = map { Math::BigInt->new($_) } @text;
    Carp::croak("next_int_in_range needs signed 64-bit bounds with min <= max, got [$lo, $hi]")
        if $lo->is_nan || $hi->is_nan
        || $lo < $_I64_MIN || $hi > $_I64_MAX || $lo > $hi;

    my $span = $hi - $lo + 1;
    my $next_u64 = sub {
        my $h = Math::BigInt->new($next_u32_fn->());
        return $h->blsft(32)->bior($next_u32_fn->());
    };
    if ($span <= $_TWO_32) {
        my $s = $span->numify;
        my $threshold = (4294967296 - $s) % $s;
        while (1) {
            my $r = $next_u32_fn->();
            return ($lo + ($r % $s))->numify if $r >= $threshold;
        }
    }
    if ($span < $_TWO_64) {
        my $threshold = ($_TWO_64 - $span) % $span;
        while (1) {
            my $r = $next_u64->();
            return ($lo + ($r % $span))->numify if $r >= $threshold;
        }
    }
    # Only the whole domain is left; anything else is a bug, not a draw.
    Carp::croak("next_int_in_range: unexpected span $span") unless $span == $_TWO_64;
    return ($lo + $next_u64->())->numify;
}

# ============================================================================
# LCG — Linear Congruential Generator
# ============================================================================

package CodingAdventures::Rng::LCG;

use strict;
use warnings;
use Math::BigInt;

# LCG::new(seed): constructor. Any seed (including 0) is valid.
#
# We store state as a Math::BigInt to avoid precision loss in the LCG
# multiply step, which can produce values up to ~2^128 before masking.
sub new {
    my ($class, $seed) = @_;
    $seed //= 0;
    Carp::croak("seed must be a non-negative integer") unless defined $seed && $seed =~ /^\d+$/;
    return bless {
        _state => Math::BigInt->new("$seed"),
    }, $class;
}

# next_u32(): advance state and return upper 32 bits as a plain Perl integer.
#
#   new_state = (state × a + c) mod 2^64
#   output    = new_state >> 32    (upper 32 bits)
sub next_u32 {
    my ($self) = @_;
    $self->{_state} = CodingAdventures::Rng::_lcg_advance($self->{_state});
    # Upper 32 bits = state >> 32
    my $upper = $self->{_state}->copy()->brsft(32);
    return ($upper->band(CodingAdventures::Rng::U32_MASK))->numify();
}

# next_u64(): combine two consecutive next_u32 calls: (hi << 32) | lo.
# Returns a Math::BigInt (may exceed 53-bit float precision).
sub next_u64 {
    my ($self) = @_;
    my $hi = Math::BigInt->new($self->next_u32());
    my $lo = Math::BigInt->new($self->next_u32());
    return $hi->blsft(32)->bior($lo);
}

# next_float(): float in [0.0, 1.0).
sub next_float {
    my ($self) = @_;
    return $self->next_u32() / CodingAdventures::Rng::FLOAT_DIV;
}

# next_int_in_range(min, max): uniform integer in [min, max] inclusive.
sub next_int_in_range {
    my ($self, $min, $max) = @_;
    return CodingAdventures::Rng::_rejection_sample(
        sub { $self->next_u32() },
        $min, $max
    );
}

# ============================================================================
# Xorshift64 — Marsaglia XOR-Shift Generator
# ============================================================================

package CodingAdventures::Rng::Xorshift64;

use strict;
use warnings;
use Math::BigInt;

# The XOR-shift constants (13, 7, 17) were found by Marsaglia's exhaustive
# search. Each shift scatters bits left untouched by the others.
#
# State is stored as a Math::BigInt to handle 64-bit shift-and-xor correctly.
# Note: Perl's native >> is arithmetic (sign-extends), so we must use BigInt
# for the right-shift on a 64-bit value.

sub new {
    my ($class, $seed) = @_;
    $seed //= 0;
    Carp::croak("seed must be a non-negative integer") unless defined $seed && $seed =~ /^\d+$/;
    $seed = 1 if $seed == 0;   # seed 0 is a fixed point; replace with 1
    return bless {
        _state => Math::BigInt->new("$seed"),
    }, $class;
}

# next_u32(): apply three XOR-shifts; return lower 32 bits.
#
#   x ^= x << 13
#   x ^= x >> 7
#   x ^= x << 17
#   output = x & 0xFFFFFFFF
#
# IMPORTANT: Math::BigInt operations (bxor, band, blsft, brsft) modify
# objects IN PLACE and return the same reference. We must work with a
# fresh copy of state so we never mutate $self->{_state} mid-step, and
# we must save the new state before extracting the return value (the
# u32 mask would corrupt the stored state otherwise).
sub next_u32 {
    my ($self) = @_;
    my $U64 = Math::BigInt->new(CodingAdventures::Rng::U64_MASK_STR);

    # Work on a copy so the in-place operations don't corrupt _state early.
    my $x = $self->{_state}->copy();

    # x ^= x << 13  (mask to 64 bits to simulate u64 overflow)
    $x->bxor($x->copy()->blsft(13)->band($U64));
    # x ^= x >> 7   (logical / unsigned right-shift on positive BigInt)
    $x->bxor($x->copy()->brsft(7));
    # x ^= x << 17  (mask to 64 bits)
    $x->bxor($x->copy()->blsft(17)->band($U64));
    # Keep only the low 64 bits in state
    $x->band($U64);

    # Save new state (full 64 bits) BEFORE masking to 32 for output.
    $self->{_state} = $x->copy();

    # Return lower 32 bits as a plain Perl integer.
    return ($x->band(CodingAdventures::Rng::U32_MASK))->numify();
}

# next_u64(): two consecutive next_u32 calls, composed into 64 bits.
sub next_u64 {
    my ($self) = @_;
    my $hi = Math::BigInt->new($self->next_u32());
    my $lo = Math::BigInt->new($self->next_u32());
    return $hi->blsft(32)->bior($lo);
}

# next_float(): float in [0.0, 1.0).
sub next_float {
    my ($self) = @_;
    return $self->next_u32() / CodingAdventures::Rng::FLOAT_DIV;
}

# next_int_in_range(min, max): uniform integer in [min, max] inclusive.
sub next_int_in_range {
    my ($self, $min, $max) = @_;
    return CodingAdventures::Rng::_rejection_sample(
        sub { $self->next_u32() },
        $min, $max
    );
}

# ============================================================================
# PCG32 — Permuted Congruential Generator
# ============================================================================

package CodingAdventures::Rng::PCG32;

use strict;
use warnings;
use Math::BigInt;

# PCG32 uses the LCG recurrence and then applies XSH RR (XOR-Shift High /
# Random Rotate) to the old state before advancing.
#
# XSH RR step by step (using old_state before the LCG advance):
#   xorshifted = ((old >> 18) ^ old) >> 27   — mix bits down
#   rot        = old >> 59                   — 5-bit rotation amount (0-31)
#   output     = rotr32(xorshifted, rot)     — scatter bits
#
# Initialization (initseq warm-up — same as Go reference):
#   1. Start with state = 0, increment = LCG_INCREMENT.
#   2. Advance once (stirs in the increment).
#   3. Add seed to state.
#   4. Advance once more (scatters seed bits).

sub new {
    my ($class, $seed) = @_;
    $seed //= 0;
    Carp::croak("seed must be a non-negative integer") unless defined $seed && $seed =~ /^\d+$/;
    my $inc = Math::BigInt->new(CodingAdventures::Rng::LCG_INCREMENT);
    my $state = Math::BigInt->new(0);

    # Warm-up step 1
    $state = CodingAdventures::Rng::_lcg_advance($state);
    # Warm-up step 2: add seed
    my $U64 = Math::BigInt->new(CodingAdventures::Rng::U64_MASK_STR);
    $state = ($state + Math::BigInt->new("$seed"))->band($U64);
    # Warm-up step 3
    $state = CodingAdventures::Rng::_lcg_advance($state);

    return bless {
        _state     => $state,
        _increment => $inc,
    }, $class;
}

# next_u32(): advance PCG32 state and return XSH RR permuted 32-bit output.
sub next_u32 {
    my ($self) = @_;
    my $old = $self->{_state};
    my $U64 = Math::BigInt->new(CodingAdventures::Rng::U64_MASK_STR);

    # Advance the LCG
    $self->{_state} = CodingAdventures::Rng::_lcg_advance($old);

    # XSH RR permutation on old state:
    # Step 1: xorshifted = ((old >> 18) ^ old) >> 27
    my $xorshifted_full = $old->copy()->brsft(18)->bxor($old)->brsft(27);
    my $xorshifted = ($xorshifted_full->band(CodingAdventures::Rng::U32_MASK))->numify();

    # Step 2: rotation amount = old >> 59  (top 5 bits, value 0-31)
    my $rot = ($old->copy()->brsft(59))->numify();

    # Step 3: rotate right
    return CodingAdventures::Rng::_rotr32($xorshifted, $rot);
}

# next_u64(): two consecutive next_u32 calls.
sub next_u64 {
    my ($self) = @_;
    my $hi = Math::BigInt->new($self->next_u32());
    my $lo = Math::BigInt->new($self->next_u32());
    return $hi->blsft(32)->bior($lo);
}

# next_float(): float in [0.0, 1.0).
sub next_float {
    my ($self) = @_;
    return $self->next_u32() / CodingAdventures::Rng::FLOAT_DIV;
}

# next_int_in_range(min, max): uniform integer in [min, max] inclusive.
sub next_int_in_range {
    my ($self, $min, $max) = @_;
    return CodingAdventures::Rng::_rejection_sample(
        sub { $self->next_u32() },
        $min, $max
    );
}

# ============================================================================

package CodingAdventures::Rng;

1;

__END__

=head1 NAME

CodingAdventures::Rng - Three classic pseudorandom number generators

=head1 SYNOPSIS

    use CodingAdventures::Rng;

    my $lcg = CodingAdventures::Rng::LCG->new(42);
    print $lcg->next_u32();               # uint32
    print $lcg->next_float();             # float in [0.0, 1.0)
    print $lcg->next_int_in_range(1, 6);  # integer in [1, 6]

    my $xs  = CodingAdventures::Rng::Xorshift64->new(42);
    my $pcg = CodingAdventures::Rng::PCG32->new(42);

=head1 DESCRIPTION

Implements LCG (Knuth 1948), Xorshift64 (Marsaglia 2003), and PCG32
(O'Neill 2014). All three expose the same four-method API:
next_u32, next_u64, next_float, next_int_in_range.

Uses Math::BigInt for 64-bit multiply to avoid IEEE 754 precision loss.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
