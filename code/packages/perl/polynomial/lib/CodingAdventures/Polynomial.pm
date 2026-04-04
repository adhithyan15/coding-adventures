package CodingAdventures::Polynomial;

# ============================================================================
# CodingAdventures::Polynomial — Single-variable polynomials over real numbers
# ============================================================================
#
# # What Is a Polynomial?
#
# A polynomial is a mathematical expression involving a variable (usually "x")
# with non-negative integer exponents. For example:
#
#   3x² + 2x + 1   is a degree-2 polynomial (a quadratic)
#   5x³ − x + 7    is a degree-3 polynomial (a cubic)
#   42             is a degree-0 polynomial (a constant)
#
# Polynomials are everywhere in computer science:
#   * Error-correcting codes (Reed-Solomon) represent data as polynomials
#   * Cryptography uses polynomials over finite fields (see GF256 package)
#   * Hash functions use polynomial rolling hashes
#   * Bezier curves are defined by cubic polynomials
#   * Compression algorithms use polynomial arithmetic
#
# # Our Representation
#
# We store a polynomial as an **array reference** (arrayref) where:
#
#   index 0 → coefficient of x⁰ (the constant term)
#   index 1 → coefficient of x¹
#   index 2 → coefficient of x²
#   ...and so on
#
# So the polynomial 3x² + 2x + 1 is stored as [1, 2, 3].
#
# Why index 0 = constant? This matches the mathematical convention that
# coefficient aᵢ belongs to the term aᵢ · xⁱ. The index IS the exponent.
#
# Example table:
#
#   Polynomial       | Array representation
#   -----------------+---------------------
#   1                | [1]
#   x                | [0, 1]
#   x + 2            | [2, 1]
#   3x² + 2x + 1     | [1, 2, 3]
#   x⁴               | [0, 0, 0, 0, 1]
#   0                | [0]
#
# # The Zero Polynomial
#
# The zero polynomial is special. We always represent it as [0]. This is the
# additive identity: p + 0 = p for any polynomial p.
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';

# Export nothing by default; callers opt-in to what they need.
our @EXPORT_OK = qw(
    normalize degree zero one
    add subtract multiply divmod_poly divide modulo
    evaluate gcd_poly
);

# ============================================================================
# normalize($poly_ref) → arrayref
#
# Remove trailing near-zero coefficients from the high-degree end.
#
# # Why normalize?
#
# After arithmetic operations, we often get extra zeros at the high end.
# For example, subtracting x² from x² gives [0, 0, 0], but that is really
# just [0] — the zero polynomial.  Trailing zeros confuse degree computation
# and slow down further operations.
#
# We use a tolerance of 1e-10 to handle floating-point imprecision. When
# working with exact integers (or GF256 values), coefficients will be exactly
# 0 or non-zero, so this threshold will never matter. When working with
# floating-point reals, tiny rounding errors like 1.23e-16 should be treated
# as zero.
#
# Algorithm:
#   1. Start from the highest-degree coefficient (last element of array).
#   2. Pop elements off the end while they are near-zero.
#   3. Never reduce below [0] — the zero polynomial always has length ≥ 1.
#
# @param $p   Arrayref of coefficients (index = degree)
# @return     New arrayref with trailing near-zeros removed
# ============================================================================
sub normalize {
    my ($p) = @_;
    my @coeffs = @$p;

    # Pop trailing near-zeros, but keep at least one element.
    while (@coeffs > 1 && abs($coeffs[-1]) < 1e-10) {
        pop @coeffs;
    }
    return \@coeffs;
}

# ============================================================================
# degree($poly_ref) → integer
#
# The degree of a polynomial is the highest power of x with a non-zero
# coefficient.
#
# Examples:
#   degree([0])       → 0   (the zero polynomial is degree 0 by convention)
#   degree([1])       → 0   (a non-zero constant has degree 0)
#   degree([0, 1])    → 1   (x has degree 1)
#   degree([1, 2, 3]) → 2   (3x² + 2x + 1 has degree 2)
#
# Note: the degree of the zero polynomial is sometimes defined as -∞ or -1,
# but we use 0 for simplicity (it is the convention in many coding-theory
# implementations).
#
# @param $p   Arrayref of coefficients
# @return     Non-negative integer degree
# ============================================================================
sub degree {
    my ($p) = @_;
    my $norm = normalize($p);
    return scalar(@$norm) - 1;
}

# ============================================================================
# zero() → [0]
#
# The additive identity for polynomial arithmetic.
# p + zero() = p for all p.
# ============================================================================
sub zero { return [0] }

# ============================================================================
# one() → [1]
#
# The multiplicative identity for polynomial arithmetic.
# p * one() = p for all p.
# ============================================================================
sub one { return [1] }

# ============================================================================
# add($a, $b) → arrayref
#
# Add two polynomials coefficient by coefficient.
#
# If the polynomials have different lengths, the shorter one is conceptually
# padded with zeros at the high end.
#
#   (3x² + 2x + 1) + (4x + 5)
#   = [1, 2, 3] + [5, 4]
#   = [1+5, 2+4, 3+0]
#   = [6, 6, 3]
#   = 3x² + 6x + 6
#
# Time complexity: O(max(deg(a), deg(b)))
#
# @param $a   Arrayref of coefficients
# @param $b   Arrayref of coefficients
# @return     Normalized arrayref of the sum
# ============================================================================
sub add {
    my ($a, $b) = @_;
    my @ra = @$a;
    my @rb = @$b;

    # Make both arrays the same length by padding the shorter with zeros.
    while (@ra < @rb) { push @ra, 0 }
    while (@rb < @ra) { push @rb, 0 }

    my @result = map { $ra[$_] + $rb[$_] } 0 .. $#ra;
    return normalize(\@result);
}

# ============================================================================
# subtract($a, $b) → arrayref
#
# Subtract polynomial $b from polynomial $a.
#
#   (3x² + 2x + 1) − (3x² + x)
#   = [1, 2, 3] − [0, 1, 3]
#   = [1, 1, 0]
#   → normalize → [1, 1]
#   = x + 1
#
# Note that over GF(2) — a field with characteristic 2 — subtraction and
# addition are the same operation. This package works over the reals, so
# subtraction genuinely differs from addition.
#
# @param $a   Arrayref (minuend)
# @param $b   Arrayref (subtrahend)
# @return     Normalized arrayref of the difference
# ============================================================================
sub subtract {
    my ($a, $b) = @_;
    my @ra = @$a;
    my @rb = @$b;

    while (@ra < @rb) { push @ra, 0 }
    while (@rb < @ra) { push @rb, 0 }

    my @result = map { $ra[$_] - $rb[$_] } 0 .. $#ra;
    return normalize(\@result);
}

# ============================================================================
# multiply($a, $b) → arrayref
#
# Multiply two polynomials using the distributive law.
#
# # How Polynomial Multiplication Works
#
# When we multiply two polynomials, every term in $a multiplies every term
# in $b. The resulting coefficient of xᵏ is the sum of all products aᵢ · bⱼ
# where i + j = k.
#
# This is exactly like integer multiplication by hand, but without carrying:
#
#   (x + 2)(x + 3) = x·x + x·3 + 2·x + 2·3
#                  = x² + 3x + 2x + 6
#                  = x² + 5x + 6
#
# In array terms:
#   [2, 1] * [3, 1] = [6, 5, 1]
#
# The output array has length deg(a) + deg(b) + 1.
#
# Algorithm (O(n·m) naive):
#   For each coefficient a[i], multiply it by every b[j] and accumulate into
#   result[i+j].
#
# @param $a   Arrayref of coefficients
# @param $b   Arrayref of coefficients
# @return     Normalized arrayref of the product
# ============================================================================
sub multiply {
    my ($a, $b) = @_;
    my @ra = @$a;
    my @rb = @$b;

    # Pre-allocate result array with zeros.
    my $result_len = @ra + @rb - 1;
    my @result = (0) x $result_len;

    for my $i (0 .. $#ra) {
        for my $j (0 .. $#rb) {
            $result[$i + $j] += $ra[$i] * $rb[$j];
        }
    }
    return normalize(\@result);
}

# ============================================================================
# divmod_poly($dividend, $divisor) → ($quotient_ref, $remainder_ref)
#
# Polynomial long division: divide $dividend by $divisor, producing a
# quotient and a remainder such that:
#
#   dividend = quotient * divisor + remainder
#
# where deg(remainder) < deg(divisor).
#
# # The Algorithm: Polynomial Long Division
#
# This mirrors grade-school long division exactly, just with polynomials
# instead of integers.
#
# Example: Divide x³ − 2x² − 4 by x − 3
#
#   Step 1: Compare leading terms: x³ ÷ x = x²
#   Step 2: Multiply: x²(x − 3) = x³ − 3x²
#   Step 3: Subtract: (x³ − 2x² − 4) − (x³ − 3x²) = x² − 4
#   Step 4: Repeat with x² − 4 as the new dividend.
#   Step 5: x² ÷ x = x. Multiply: x(x − 3) = x² − 3x.
#   Step 6: Subtract: (x² − 4) − (x² − 3x) = 3x − 4
#   Step 7: 3x ÷ x = 3. Multiply: 3(x − 3) = 3x − 9.
#   Step 8: Subtract: (3x − 4) − (3x − 9) = 5
#   Step 9: deg(5) = 0 < deg(x − 3) = 1, so stop.
#
#   Quotient: x² + x + 3
#   Remainder: 5
#   Verification: (x² + x + 3)(x − 3) + 5 = x³ − 2x² − 4 ✓
#
# This function dies with "division by zero polynomial" if $divisor is
# the zero polynomial, since division by zero is undefined.
#
# @param $dividend   Arrayref
# @param $divisor    Arrayref (must not be zero polynomial)
# @return            ($quotient_ref, $remainder_ref) as a two-element list
# ============================================================================
sub divmod_poly {
    my ($dividend, $divisor) = @_;

    my $norm_div = normalize($divisor);

    # Guard: cannot divide by the zero polynomial.
    die "division by zero polynomial"
        if @$norm_div == 1 && abs($norm_div->[0]) < 1e-10;

    my @rem     = @{ normalize($dividend) };
    my @divisor = @$norm_div;
    my $deg_div = scalar(@divisor) - 1;

    # Build an empty quotient (length = deg(dividend) - deg(divisor) + 1).
    my $deg_rem = scalar(@rem) - 1;
    if ($deg_rem < $deg_div) {
        # Dividend has smaller degree than divisor: quotient = 0, remainder = dividend.
        return (zero(), normalize(\@rem));
    }

    my $quot_len = $deg_rem - $deg_div + 1;
    my @quot     = (0) x $quot_len;

    # Leading coefficient of divisor (we divide by this each step).
    my $lead_div = $divisor[-1];

    # Perform the long division, working from the highest degree down.
    for my $i (reverse 0 .. $quot_len - 1) {
        # The current leading term of the remainder.
        my $lead_rem = $rem[$deg_div + $i];

        # Quotient coefficient at position $i.
        my $q = $lead_rem / $lead_div;
        $quot[$i] = $q;

        # Subtract q * divisor from the remainder, aligned at position $i.
        for my $j (0 .. $deg_div) {
            $rem[$i + $j] -= $q * $divisor[$j];
        }
    }

    return (normalize(\@quot), normalize(\@rem));
}

# ============================================================================
# divide($a, $b) → arrayref
#
# Polynomial division returning only the quotient.
#
# Equivalent to (divmod_poly($a, $b))[0].
# See divmod_poly for the algorithm details.
#
# @param $a   Dividend arrayref
# @param $b   Divisor arrayref
# @return     Quotient arrayref
# ============================================================================
sub divide {
    my ($a, $b) = @_;
    my ($q, $_r) = divmod_poly($a, $b);
    return $q;
}

# ============================================================================
# modulo($a, $b) → arrayref
#
# Polynomial division returning only the remainder.
#
# Equivalent to (divmod_poly($a, $b))[1].
#
# This is the key operation for modular polynomial arithmetic — the
# foundation of Reed-Solomon codes and GF(2⁸) field arithmetic.
#
# In GF(2⁸), all arithmetic is done modulo the irreducible polynomial
# x⁸ + x⁴ + x³ + x + 1 (represented in binary as 0x11D). The modulo
# operation ensures results always stay within the field.
#
# @param $a   Dividend (the value to reduce)
# @param $b   Modulus (the irreducible polynomial)
# @return     Remainder arrayref, always with degree < degree($b)
# ============================================================================
sub modulo {
    my ($a, $b) = @_;
    my ($_q, $r) = divmod_poly($a, $b);
    return $r;
}

# ============================================================================
# evaluate($poly, $x) → number
#
# Evaluate the polynomial at a specific value of x.
#
# # Horner's Method
#
# A naive implementation would compute each xⁿ separately:
#
#   result = a₀ + a₁·x + a₂·x² + a₃·x³
#
# This requires n multiplications for the powers PLUS n multiplications for
# scaling PLUS n additions = O(n²) work.
#
# Horner's method factors the polynomial cleverly:
#
#   a₀ + x(a₁ + x(a₂ + x·a₃))
#
# This is O(n) — just n multiplications and n additions — the optimal
# algorithm for polynomial evaluation. It was discovered by William George
# Horner in 1819 (though Qin Jiushao described it in 1247!).
#
# Algorithm: Start with the highest-degree coefficient and repeatedly
# multiply by x, then add the next lower coefficient.
#
# Example: Evaluate 3x² + 2x + 1 at x = 2:
#   Coefficients (high to low): 3, 2, 1
#   Step 1: acc = 3
#   Step 2: acc = 3*2 + 2 = 8
#   Step 3: acc = 8*2 + 1 = 17
#   Result: 17  (verify: 3·4 + 2·2 + 1 = 12 + 4 + 1 = 17 ✓)
#
# @param $poly   Arrayref of coefficients (index 0 = constant term)
# @param $x      The value to substitute for the variable
# @return        The scalar result
# ============================================================================
sub evaluate {
    my ($poly, $x) = @_;
    my $norm = normalize($poly);

    # Start with the leading coefficient and work down using Horner's rule.
    my $acc = $norm->[-1];
    for my $i (reverse 0 .. $#$norm - 1) {
        $acc = $acc * $x + $norm->[$i];
    }
    return $acc;
}

# ============================================================================
# gcd_poly($a, $b) → arrayref
#
# Compute the Greatest Common Divisor (GCD) of two polynomials.
#
# # The Euclidean Algorithm for Polynomials
#
# The same Euclidean algorithm that computes GCD for integers works for
# polynomials too! The key insight is:
#
#   gcd(a, b) = gcd(b, a mod b)
#
# This is because any common divisor of a and b is also a common divisor
# of b and (a mod b), and vice versa.
#
# We stop when b becomes the zero polynomial. At that point, a holds the GCD.
#
# Example: gcd(x² − 1, x − 1)
#   Round 1: divide x² − 1 by x − 1
#     x² − 1 = (x + 1)(x − 1) + 0
#     remainder = 0 → stop
#   GCD = x − 1 (makes sense: x − 1 divides both x² − 1 and x − 1)
#
# Example: gcd(x³ − x, x² − 1)
#   x³ − x = x(x² − 1) + (x − x) ... actually:
#   x³ − x = x·(x² − 1) + (x − x + 0) ← let me redo:
#   x³ − x divided by x² − 1:
#     Leading term: x³ / x² = x
#     x·(x² − 1) = x³ − x  → remainder = 0
#   GCD = x² − 1
#
# # Monic Result
#
# A polynomial GCD is defined up to a constant multiple (just like how
# gcd(6, 4) could be 2 or −2 — we always pick positive). For polynomials
# we make the result **monic** (leading coefficient = 1) by dividing through.
#
# @param $a   Arrayref of coefficients
# @param $b   Arrayref of coefficients
# @return     Monic GCD arrayref
# ============================================================================
sub gcd_poly {
    my ($a, $b) = @_;
    my $pa = normalize($a);
    my $pb = normalize($b);

    # Euclidean algorithm: keep replacing (a, b) with (b, a mod b)
    # until b is the zero polynomial.
    while (!( @$pb == 1 && abs($pb->[0]) < 1e-10 )) {
        my ($_q, $r) = divmod_poly($pa, $pb);
        $pa = $pb;
        $pb = $r;
    }

    # $pa now holds the GCD. Make it monic (leading coefficient = 1).
    my @coeffs   = @$pa;
    my $lead     = $coeffs[-1];

    # If the lead is near-zero we have a degenerate case: return one().
    if (abs($lead) < 1e-10) { return one() }

    my @monic = map { $_ / $lead } @coeffs;
    return normalize(\@monic);
}

1;

__END__

=head1 NAME

CodingAdventures::Polynomial - Single-variable polynomials over the reals

=head1 SYNOPSIS

    use CodingAdventures::Polynomial qw(
        zero one degree normalize
        add subtract multiply divide modulo divmod_poly
        evaluate gcd_poly
    );

    # Build the polynomial 3x^2 + 2x + 1
    my $p = [1, 2, 3];

    # Degree
    say degree($p);          # 2

    # Evaluate at x=2: 3*4 + 2*2 + 1 = 17
    say evaluate($p, 2);     # 17

    # Arithmetic
    my $q = [5, 4];          # 4x + 5
    my $sum = add($p, $q);   # 3x^2 + 6x + 6

    # Long division
    my ($quot, $rem) = divmod_poly($p, $q);

    # GCD
    my $g = gcd_poly([0, -1, 0, 1], [-1, 0, 1]);  # x-1

=head1 DESCRIPTION

Implements standard polynomial arithmetic over the real numbers.
Polynomials are represented as array references where index i holds
the coefficient of x^i (index 0 = constant term).

All results are normalized (trailing near-zero coefficients removed).

=head1 FUNCTIONS

=over 4

=item C<normalize($p)>

Remove trailing near-zero coefficients (threshold 1e-10). Always returns
at least C<[0]>.

=item C<degree($p)>

Return the degree (highest power with non-zero coefficient) of polynomial $p.

=item C<zero()>

Return C<[0]>, the additive identity.

=item C<one()>

Return C<[1]>, the multiplicative identity.

=item C<add($a, $b)>

Add two polynomials and return the normalized result.

=item C<subtract($a, $b)>

Subtract $b from $a and return the normalized result.

=item C<multiply($a, $b)>

Multiply two polynomials using the distributive law.

=item C<divmod_poly($a, $b)>

Polynomial long division. Returns C<($quotient, $remainder)>. Dies with
C<"division by zero polynomial"> if $b is the zero polynomial.

=item C<divide($a, $b)>

Return only the quotient from divmod_poly.

=item C<modulo($a, $b)>

Return only the remainder from divmod_poly.

=item C<evaluate($poly, $x)>

Evaluate the polynomial at a numeric value $x using Horner's method.

=item C<gcd_poly($a, $b)>

Compute the monic GCD using the polynomial Euclidean algorithm.

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
