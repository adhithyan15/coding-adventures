package CodingAdventures::CtCompare;

# ============================================================================
# CodingAdventures::CtCompare -- constant-time comparison and selection (CT01)
# ============================================================================
#
# Why this exists
# ---------------
# A secret comparison that stops at the first differing byte tells an attacker
# how long the matching prefix was.  Controlling one operand (a MAC tag, a
# session token) they can recover the other one byte at a time by timing guesses.
# The defence is to look at EVERY byte regardless of the data, fold the
# differences into one accumulator, and inspect the accumulator exactly once at
# the end:
#
#     a   = 41 42 43 44          a   = 41 42 43 44
#     b   = 41 42 43 44          b   = 41 FF 43 44
#     xor = 00 00 00 00          xor = 00 BD 00 00   (all four bytes were read)
#     acc = 0x00  -> equal       acc = 0xBD          -> different
#
# This is the Perl lane of CT01 (code/specs/CT01-constant-time-compare.md).
#
# What this module promises -- and what it does not
# -------------------------------------------------
# The source has no content-dependent early return, branch, subscript or loop
# bound; work depends only on the (public) lengths.  It does NOT promise
# wall-clock constant time.  Perl is an interpreter: every op is dispatched
# through the runloop, scalars are reference counted and reallocated as they
# grow, a string's bytes may be copied or shared copy-on-write, and the
# interpreter's own branches and allocator are invisible to this code.  Secrets
# also linger in freed memory; Perl offers no way to wipe them.  Treat the
# module as protection against ALGORITHMIC timing leaks only.
#
# How the selection bit is received
# ---------------------------------
# Perl has no branch-free way to turn a boolean into a number (`$b ? 1 : 0` is a
# branch on the secret), so, like the Go, Ruby and Elixir lanes, the selection
# bit is an INTEGER that must be exactly 0 (pick right) or 1 (pick left), and
# every equality has a bit-returning variant so no caller ever converts a
# boolean with a conditional:
#
#     my $picked = ct_select_bytes($a, $b, ct_eq_bit($x, $y));
#
# Bitwise operators and strings
# -----------------------------------------------------------------------------
# In classic Perl `^` and `|` act on STRINGS bytewise when both operands are
# strings -- exactly the trap that makes "12" ^ "34" a different thing from
# 12 ^ 34.  `use feature 'bitwise'` (Perl 5.28) makes ^ | & ~ strictly numeric,
# so every value below is turned into an integer first and the operators can
# never silently change meaning.
# ============================================================================

use strict;
use warnings;
use v5.28;
use feature 'bitwise';
no warnings 'experimental::bitwise';

use Carp qw(croak);
use Exporter 'import';

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(
    ct_eq ct_eq_fixed ct_select_bytes ct_eq_u64
    ct_eq_bit ct_eq_fixed_bit ct_eq_u64_bit
);

# ----------------------------------------------------------------------------
# Input validation.  These look at types, ranges and lengths only -- never at
# whether two inputs match -- so they reveal nothing about valid secrets.
# ----------------------------------------------------------------------------

# Return the byte values of a byte-string input as a plain list of integers.
# Accepts a byte string (characters 0..255; a string holding wider characters
# is rejected) or an unblessed array reference of integers 0..255.  Each value
# is read exactly once, so nothing can change between validation and use.
sub _as_bytes {
    my ($value, $name) = @_;
    croak "$name must be a byte string or an array reference of byte values"
        if !defined $value;
    my $kind = ref $value;
    if ($kind eq q{}) {
        my $copy = "$value";
        croak "$name contains characters above 0xFF"
            unless utf8::downgrade($copy, 1);
        return unpack 'C*', $copy;
    }
    if ($kind eq 'ARRAY') {
        my @snapshot = @{$value};
        my $index = 0;
        for my $element (@snapshot) {
            # Only plain integers 0..255: a float, a string such as "9e1" or a
            # reference must not be silently truncated into a different byte.
            croak "$name\[$index] must be an integer from 0 to 255"
                unless defined $element
                && !ref $element
                && $element =~ /\A[0-9]{1,3}\z/
                && $element <= 255;
            $index++;
        }
        return map { 0 + $_ } @snapshot;
    }
    croak "$name must be a byte string or an array reference of byte values";
}

# An unsigned 64-bit value: a non-negative decimal integer no larger than
# 18446744073709551615.  Perl's UV type holds exactly these 64 bits.
sub _as_u64 {
    my ($value, $name) = @_;
    croak "$name must be an unsigned 64-bit integer"
        unless defined $value
        && !ref $value
        && $value =~ /\A(?:0|[1-9][0-9]*)\z/
        && (length $value < 20
            || (length $value == 20 && $value le '18446744073709551615'));
    return 0 + $value;
}

# ----------------------------------------------------------------------------
# Equality
# ----------------------------------------------------------------------------

# OR together the XOR of every byte pair.  Both lists have the same length.
sub _difference {
    my ($left, $right) = @_;
    my $acc = 0;
    for my $i (0 .. $#{$left}) {
        $acc |= $left->[$i] ^ $right->[$i];
    }
    return $acc;
}

# 1 when the byte strings are equal, else 0.  A length mismatch is 0 and reads
# neither input's contents; lengths are public.
#
# For acc in 0..255, acc - 1 is -1 exactly when acc == 0.  Perl casts a
# negative operand of >> to an unsigned 64-bit word, so -1 becomes all ones and
# (-1 >> 8) & 1 == 1, while for acc in 1..255 the value acc-1 is 0..254 and
# (acc-1) >> 8 == 0.  No comparison, no conditional.
sub ct_eq_bit {
    my ($left, $right) = @_;
    my @l = _as_bytes($left,  'left');
    my @r = _as_bytes($right, 'right');
    return 0 unless @l == @r;
    my $acc = _difference(\@l, \@r);
    return (($acc - 1) >> 8) & 1;
}

# Fixed-width companion: Perl has no fixed-size array type, so this is the same
# operation, kept so call sites read the same in every CT01 lane.
sub ct_eq_fixed_bit { return ct_eq_bit(@_) }

# Boolean equality without early exit.  The boolean is the PUBLIC result.
sub ct_eq       { return ct_eq_bit(@_) == 1 }
sub ct_eq_fixed { return ct_eq_bit(@_) == 1 }

# ----------------------------------------------------------------------------
# Selection
# ----------------------------------------------------------------------------

# Select $left when $choice is 1 and $right when it is 0, returning a fresh
# byte string.  With mask = 0xFF (choice 1) or 0x00 (choice 0):
#
#     out = right ^ ((left ^ right) & mask)
#
# is `left` or `right` by the same instructions either way.  The choice must be
# an integer scalar whose bits other than bit 0 are clear; the test
# (choice & ~1) != 0 is one computation that gives the same answer for both
# valid values, so a valid call does identical work whichever bit is secret.
sub ct_select_bytes {
    my ($left, $right, $choice) = @_;
    my @l = _as_bytes($left,  'left');
    my @r = _as_bytes($right, 'right');
    croak 'CT_SELECT_LENGTH_MISMATCH: ct_select_bytes requires equal-length byte strings'
        unless @l == @r;
    croak 'ct_select_bytes requires choice to be the integer 0 or 1'
        unless defined $choice && !ref $choice && $choice =~ /\A-?[0-9]{1,18}\z/;
    my $bit = 0 + $choice;
    croak 'ct_select_bytes requires choice to be the integer 0 or 1'
        if ($bit & ~1) != 0;

    my $mask = (-$bit) & 0xFF;    # 0 -> 0x00, 1 -> 0xFF
    my @out;
    for my $i (0 .. $#l) {
        push @out, $r[$i] ^ (($l[$i] ^ $r[$i]) & $mask);
    }
    return pack 'C*', @out;
}

# ----------------------------------------------------------------------------
# Unsigned 64-bit equality
# ----------------------------------------------------------------------------

# d = left XOR right is zero exactly when the values are equal.  OR-fold all 64
# bits into bit 0 with a fixed sequence of shifts (32, 16, 8, 4, 2, 1), then
# flip it: the result is 1 exactly when d was 0.  (The two's-complement trick
# d | -d is avoided because unary minus on a Perl UV above 2**63 yields a
# floating point number.)  Perl's >> on an unsigned value is a logical shift.
sub ct_eq_u64_bit {
    my ($left, $right) = @_;
    my $l = _as_u64($left,  'left');
    my $r = _as_u64($right, 'right');
    my $d = $l ^ $r;
    $d |= $d >> 32;
    $d |= $d >> 16;
    $d |= $d >> 8;
    $d |= $d >> 4;
    $d |= $d >> 2;
    $d |= $d >> 1;
    return ($d & 1) ^ 1;
}

sub ct_eq_u64 { return ct_eq_u64_bit(@_) == 1 }

1;
