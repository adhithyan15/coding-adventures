package CodingAdventures::PolynomialNative;

# ============================================================================
# CodingAdventures::PolynomialNative — Rust-backed polynomial arithmetic
# ============================================================================
#
# This module loads a Rust-compiled shared library via DynaLoader.
# The shared library (PolynomialNative.so) exports XSUBs (native subroutines)
# registered by boot_CodingAdventures__PolynomialNative.
#
# ## How DynaLoader works
#
# When Perl sees `use CodingAdventures::PolynomialNative;`, it:
#   1. Finds PolynomialNative.pm (this file) via @INC.
#   2. Calls `CodingAdventures::PolynomialNative->bootstrap("0.01")`.
#   3. DynaLoader finds PolynomialNative.so in auto/CodingAdventures/PolynomialNative/.
#   4. dlopen()'s it and calls boot_CodingAdventures__PolynomialNative.
#   5. The boot function registers normalize(), add(), etc. in this package.
#
# ## Usage
#
#   use CodingAdventures::PolynomialNative;
#
#   # Polynomials are array references: [coeff_of_x^0, coeff_of_x^1, ...]
#   my $a = [1.0, 2.0];       # 1 + 2x
#   my $b = [3.0, 4.0];       # 3 + 4x
#
#   my $sum = CodingAdventures::PolynomialNative::add($a, $b);   # [4.0, 6.0]
#   my $deg = CodingAdventures::PolynomialNative::degree($sum);  # 1

use strict;
use warnings;
use DynaLoader;

our $VERSION = '0.01';
our @ISA = ('DynaLoader');

# dl_load_flags: 0x01 = RTLD_GLOBAL — export symbols to other shared libraries.
# This is needed on some platforms for proper Perl embedding.
sub dl_load_flags { 0x01 }

# Bootstrap: trigger DynaLoader to find and load PolynomialNative.so.
__PACKAGE__->bootstrap($VERSION);

1;

__END__

=head1 NAME

CodingAdventures::PolynomialNative — Rust-backed polynomial arithmetic

=head1 SYNOPSIS

    use CodingAdventures::PolynomialNative qw();

    my $a = [1.0, 2.0, 3.0];   # 1 + 2x + 3x²
    my $b = [4.0, 5.0];         # 4 + 5x

    my $sum  = CodingAdventures::PolynomialNative::add($a, $b);
    my $prod = CodingAdventures::PolynomialNative::multiply($a, $b);
    my $val  = CodingAdventures::PolynomialNative::evaluate($a, 2.0);
    my $deg  = CodingAdventures::PolynomialNative::degree($a);

=head1 FUNCTIONS

=over 4

=item normalize($poly)

Strip trailing near-zero coefficients. Returns an arrayref.

=item degree($poly)

Return the degree (highest non-zero exponent index).

=item zero()

Return [0.0] — the additive identity polynomial.

=item one()

Return [1.0] — the multiplicative identity polynomial.

=item add($a, $b)

Add two polynomials. Returns an arrayref.

=item subtract($a, $b)

Subtract $b from $a. Returns an arrayref.

=item multiply($a, $b)

Multiply two polynomials. Result degree = deg($a) + deg($b). Returns arrayref.

=item evaluate($poly, $x)

Evaluate at point $x using Horner's method. Returns a number.

=back
