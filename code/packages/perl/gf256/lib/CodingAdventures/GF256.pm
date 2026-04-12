package CodingAdventures::GF256;

# ============================================================================
# CodingAdventures::GF256 — Arithmetic in the Galois Field GF(2⁸)
# ============================================================================
#
# # What Is a Finite Field?
#
# A *field* is a set of elements with two operations — addition and
# multiplication — that obey all the usual algebraic laws:
#
#   * Commutativity:   a + b = b + a,   a·b = b·a
#   * Associativity:   (a+b)+c = a+(b+c), etc.
#   * Distributivity: a·(b+c) = a·b + a·c
#   * Identities:      a + 0 = a,  a·1 = a
#   * Inverses:        every a ≠ 0 has an inverse a⁻¹ such that a·a⁻¹ = 1
#
# A *finite* field (also called a Galois Field, after Évariste Galois, who
# died in a duel aged 20) has exactly q elements, where q = pⁿ for some
# prime p.
#
# GF(2⁸) has exactly 256 elements — which is why it fits perfectly in a
# byte. Every element can be stored as a single uint8_t in C.
#
# # Why GF(2⁸)?
#
# GF(2⁸) is the workhorse of byte-oriented cryptography and coding theory:
#
#   * AES (Advanced Encryption Standard) — all S-box and MixColumns operations
#     are GF(2⁸) arithmetic.
#   * Reed-Solomon error-correcting codes — used in QR codes, CDs, DVDs,
#     RAID-6, and spacecraft telemetry.
#   * CRC polynomials — defined as polynomial arithmetic over GF(2).
#
# # The Elements
#
# GF(2⁸) elements are polynomials over GF(2) (coefficients 0 or 1) of
# degree ≤ 7. There are exactly 2⁸ = 256 such polynomials:
#
#   a₇x⁷ + a₆x⁶ + a₅x⁵ + a₄x⁴ + a₃x³ + a₂x² + a₁x + a₀
#
# where each aᵢ ∈ {0, 1}. We represent them as integers 0–255, using
# bit i for coefficient aᵢ. For example:
#
#   0b00001101 = 13 represents x³ + x² + 1
#   0b00000011 = 3  represents x + 1
#
# # Addition: XOR
#
# Since coefficients are in GF(2), addition is modulo 2: 0+0=0, 1+0=1,
# 0+1=1, 1+1=0 (no carry!). In binary, this is exactly XOR:
#
#   (x³+x²+1) + (x+1) = x³ + x² + x + 0 = x³+x²+x
#   Binary: 0b00001101 XOR 0b00000011 = 0b00001110 = 14
#
# Because addition is XOR, subtraction is IDENTICAL to addition:
#   a − b = a + b in GF(2⁸) (since -1 = 1 in GF(2))
#
# # Multiplication: The Primitive Polynomial
#
# Multiplying two polynomials of degree ≤ 7 can give a degree ≤ 14 result,
# which no longer fits in a byte. We need to reduce modulo an *irreducible*
# polynomial of degree 8 — one that cannot be factored.
#
# We use the primitive polynomial (also used by AES):
#
#   p(x) = x⁸ + x⁴ + x³ + x + 1
#
# In binary: 100011011₂ = 0x11D
#
# "Primitive" means that there is a generator element g such that g⁰, g¹,
# g², ..., g²⁵⁴ enumerate ALL 255 non-zero field elements. We use g = 2
# (i.e., the polynomial x).
#
# # LOG and ALOG Tables
#
# Computing gⁱ modulo p(x) at runtime for each multiplication is expensive.
# Instead, we pre-compute two lookup tables at module load time:
#
#   ALOG[i] = gⁱ mod p(x)  (anti-log, or exponentiation table)
#   LOG[a]  = i  such that gⁱ = a  (discrete logarithm)
#
# With these tables, multiplication becomes:
#
#   a · b = ALOG[(LOG[a] + LOG[b]) mod 255]   (for a,b ≠ 0)
#
# And division:
#
#   a / b = ALOG[(LOG[a] − LOG[b]) mod 255]
#
# This converts O(degree) polynomial multiplication into O(1) table lookups!
#
# Note: the ALOG table is indexed 0..254 for the 255 non-zero elements.
# To allow addition of two indices without range checks, we often extend it
# to 0..509, but here we use `% 255` instead.
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';

our @EXPORT_OK = qw(add subtract multiply divide power inverse);

# ============================================================================
# Constants
# ============================================================================

# The additive identity (zero element of the field).
our $ZERO = 0;

# The multiplicative identity (one element of the field).
our $ONE = 1;

# The primitive polynomial x^8 + x^4 + x^3 + x + 1, as an integer.
# Bit 8 = x^8, bit 4 = x^4, bit 3 = x^3, bit 1 = x, bit 0 = 1.
# We keep bit 8 here so we can detect overflow past degree 7.
our $PRIMITIVE_POLYNOMIAL = 0x11D;   # = 285 = 0b100011101

# ============================================================================
# Pre-computed LOG and ALOG Tables
# ============================================================================
#
# These are package-level arrays, filled once when the module is first loaded.
# Using `our` makes them accessible from tests for verification.
#
# ALOG[i] — the field element g^i, where g = 2 (the polynomial x).
# LOG[a]  — the discrete log base g of element a (only defined for a ≠ 0).
#
# Index range:
#   ALOG: 0..254 (255 entries for the 255 non-zero elements)
#   LOG:  0..255 (256 entries; LOG[0] is meaningless, left as 0)

our @ALOG = (0) x 256;   # anti-logarithm (exponentiation) table
our @LOG  = (0) x 256;   # logarithm table

# Build the tables at module load time using a BEGIN block.
# We compute g^0 = 1, g^1 = 2, g^2 = 4, g^3 = 8, ...
# When a result would exceed 0xFF (degree > 7), we XOR with the primitive
# polynomial (minus its x^8 term, i.e. 0x1D = 0b00011101) to reduce it.
#
# Why XOR with 0x1D and not 0x11D?  Because the x^8 term in the reduced
# value is already represented by the overflow bit — when we XOR with 0x11D,
# the bit-8 and the overflow bit cancel (both are 1), leaving an 8-bit value.
# Equivalently: reduction mod p(x) where p(x) = x^8 + lower means:
#   if result >= 256: result = result XOR 0x11D
# (the XOR clears bit 8 and applies the lower terms of the primitive poly)
#
# This is the standard "peasant multiplication" reduction step.

{
    my $val = 1;
    for my $i (0 .. 254) {
        $ALOG[$i] = $val;
        $LOG[$val] = $i;
        $val <<= 1;                 # multiply by x (shift left by 1)
        if ($val & 0x100) {         # if degree exceeded 7 (bit 8 set)
            $val ^= $PRIMITIVE_POLYNOMIAL;  # reduce mod p(x)
            $val &= 0xFF;           # keep only 8 bits
        }
    }
    # Ensure ALOG[255] = 1: g^255 = g^0 = 1 in a field of order 255.
    # The loop above sets ALOG[0] = 1 and generates the full cycle.
    # For safe addition of indices, we also set ALOG[255] = ALOG[0].
    $ALOG[255] = $ALOG[0];
}

# ============================================================================
# add($a, $b) → integer in 0..255
#
# Addition in GF(2⁸) is bitwise XOR.
#
# Why XOR? Because GF(2⁸) elements are polynomials with coefficients in
# GF(2) = {0, 1}. Adding two GF(2) numbers is addition modulo 2 — which is
# exactly XOR.
#
# Truth table for coefficient addition in GF(2):
#
#   a | b | a+b (mod 2)
#   --|---|------------
#   0 | 0 |     0
#   0 | 1 |     1
#   1 | 0 |     1
#   1 | 1 |     0    ← 1+1=0, NOT 2 (the field has characteristic 2)
#
# As a consequence:
#   * addition is its own inverse: a + a = 0 for all a
#   * subtraction = addition: a − b = a + b
#   * the result is always in 0..255 — no reduction needed
#
# @param $a   Field element (integer 0..255)
# @param $b   Field element (integer 0..255)
# @return     $a XOR $b (integer 0..255)
# ============================================================================
sub add {
    my ($a, $b) = @_;
    return $a ^ $b;
}

# ============================================================================
# subtract($a, $b) → integer in 0..255
#
# Subtraction in GF(2⁸) is identical to addition.
#
# In any field of characteristic 2 (meaning 1+1=0), the additive inverse
# of every element a is a itself: −a = a.  Therefore a−b = a+(−b) = a+b = a XOR b.
#
# This is not a quirk — it's a mathematical fact that makes GF(2⁸) very
# efficient for hardware: addition and subtraction share the same XOR circuit.
# ============================================================================
sub subtract {
    my ($a, $b) = @_;
    return $a ^ $b;
}

# ============================================================================
# multiply($a, $b) → integer in 0..255
#
# Multiplication in GF(2⁸) via the LOG/ALOG lookup tables.
#
# # The Trick: Logarithms Turn Multiplication Into Addition
#
# In ordinary arithmetic, logarithms turn multiplication into addition:
#   log(a · b) = log(a) + log(b)
#
# The same trick works in GF(2⁸), where "log" is the discrete logarithm
# base g (our generator g = 2):
#   a · b = g^(LOG[a] + LOG[b])  = ALOG[(LOG[a] + LOG[b]) mod 255]
#
# Steps:
#   1. Look up the discrete logs: i = LOG[a], j = LOG[b]
#   2. Add the exponents: k = (i + j) mod 255
#   3. Look up the result: ALOG[k]
#
# Special case: if either operand is 0, the product is 0.
# (0 has no discrete log since g^i is never 0 for any i.)
#
# @param $a   Field element (integer 0..255)
# @param $b   Field element (integer 0..255)
# @return     a·b in GF(2⁸) (integer 0..255)
# ============================================================================
sub multiply {
    my ($a, $b) = @_;
    return 0 if $a == 0 || $b == 0;
    return $ALOG[ ($LOG[$a] + $LOG[$b]) % 255 ];
}

# ============================================================================
# divide($a, $b) → integer in 0..255
#
# Division in GF(2⁸) using the LOG/ALOG tables.
#
# Analogously to multiplication:
#   a / b = g^(LOG[a] − LOG[b])
#
# We take the modulus 255 to handle wrap-around:
#   (LOG[a] - LOG[b]) mod 255 — in Perl, the '%' operator on positive
#   integers works as expected. To avoid negative results, we add 255:
#   (LOG[a] - LOG[b] + 255) % 255
#
# Special case: 0 / b = 0 for any non-zero b.
# Dividing by 0 is undefined — we die with an error message.
#
# @param $a   Dividend (integer 0..255)
# @param $b   Divisor (integer 1..255; 0 causes die)
# @return     a/b in GF(2⁸) (integer 0..255)
# ============================================================================
sub divide {
    my ($a, $b) = @_;
    die "division by zero in GF256" if $b == 0;
    return 0 if $a == 0;
    return $ALOG[ ($LOG[$a] - $LOG[$b] + 255) % 255 ];
}

# ============================================================================
# power($base, $exp) → integer in 0..255
#
# Raise a field element to an integer power.
#
# Using the ALOG table:
#   base^exp = ALOG[(LOG[base] * exp) mod 255]
#
# Special cases:
#   * exp = 0: anything^0 = 1 (including 0^0 = 1 by convention here)
#   * base = 0 and exp > 0: 0^k = 0
#
# Negative exponents could be handled as:
#   base^(−k) = (base^(−1))^k = ALOG[(255 − LOG[base]) * k mod 255]
# but we do not support them here — pass non-negative integers only.
#
# @param $base   Field element (integer 0..255)
# @param $exp    Non-negative integer exponent
# @return        base^exp in GF(2⁸)
# ============================================================================
sub power {
    my ($base, $exp) = @_;
    return 1 if $exp == 0;         # a^0 = 1 for all a (including a=0 by convention)
    return 0 if $base == 0;        # 0^k = 0 for k > 0
    return $ALOG[ ($LOG[$base] * $exp) % 255 ];
}

# ============================================================================
# inverse($a) → integer in 1..255
#
# The multiplicative inverse of a field element.
#
# a⁻¹ is the unique element such that a · a⁻¹ = 1.
#
# Using the log table:
#   a⁻¹ = g^(−LOG[a]) = g^(255 − LOG[a])
#
# Because g^255 = g^0 = 1, subtracting LOG[a] from 255 gives the inverse.
#
# The zero element has no multiplicative inverse (there is no x such that
# 0 · x = 1), so we die with an error.
#
# @param $a   Field element (integer 1..255)
# @return     a⁻¹ in GF(2⁸) (integer 1..255)
# ============================================================================
sub inverse {
    my ($a) = @_;
    die "inverse of zero in GF256" if $a == 0;
    return $ALOG[ 255 - $LOG[$a] ];
}


# ============================================================================
# CodingAdventures::GF256::Field — parameterizable field factory
# ============================================================================
#
# The functions above are fixed to the Reed-Solomon polynomial 0x11D.
# AES uses a different primitive polynomial: 0x11B.
# GF256::Field accepts any primitive polynomial and builds its own independent
# log/antilog tables.
#
# Usage:
#   use CodingAdventures::GF256::Field;
#
#   my $aes = CodingAdventures::GF256::Field->new(0x11B);
#   $aes->multiply(0x53, 0x8C);   # => 1   (AES GF(2^8) inverses)
#   $aes->multiply(0x57, 0x83);   # => 0xC1 (FIPS 197 Appendix B)
#
# Backward compatibility: the module-level functions (add, subtract, multiply,
# divide, power, inverse) remain unchanged and still use 0x11D.
# ============================================================================

package CodingAdventures::GF256::Field;

use strict;
use warnings;

# ----------------------------------------------------------------------------
# new($polynomial) — construct a field for the given primitive polynomial.
#
# Builds LOG and ALOG tables using the same algorithm as the module-level code.
# The tables are stored as array references inside the blessed hash.
#
# @param $polynomial  The irreducible polynomial as an integer (e.g. 0x11B for AES,
#                     0x11D for Reed-Solomon).
# @return             A blessed GF256::Field object.
# ----------------------------------------------------------------------------
sub new {
    my ($class, $polynomial) = @_;

    my @alog = (0) x 256;
    my @log  = (0) x 256;

    my $val = 1;
    for my $i (0 .. 254) {
        $alog[$i] = $val;
        $log[$val] = $i;
        $val <<= 1;
        if ($val & 0x100) {
            $val ^= $polynomial;
            $val &= 0xFF;
        }
    }
    $alog[255] = $alog[0];    # g^255 = g^0 = 1

    return bless {
        polynomial => $polynomial,
        alog       => \@alog,
        log        => \@log,
    }, $class;
}

# The primitive polynomial this field was built with.
sub polynomial { $_[0]->{polynomial} }

# Add two field elements: a XOR b (characteristic 2; polynomial-independent).
sub add {
    my ($self, $a, $b) = @_;
    return $a ^ $b;
}

# Subtract two field elements: same as add in GF(2^8).
sub subtract {
    my ($self, $a, $b) = @_;
    return $a ^ $b;
}

# Multiply two field elements using this field's log/antilog tables.
sub multiply {
    my ($self, $a, $b) = @_;
    return 0 if $a == 0 || $b == 0;
    return $self->{alog}[ ($self->{log}[$a] + $self->{log}[$b]) % 255 ];
}

# Divide a by b. Dies if b is 0.
sub divide {
    my ($self, $a, $b) = @_;
    die "GF256::Field: division by zero" if $b == 0;
    return 0 if $a == 0;
    return $self->{alog}[ ($self->{log}[$a] - $self->{log}[$b] + 255) % 255 ];
}

# Raise base to a non-negative integer power.
sub power {
    my ($self, $base, $exp) = @_;
    return 1 if $exp == 0;
    return 0 if $base == 0;
    return $self->{alog}[ ($self->{log}[$base] * $exp) % 255 ];
}

# Return the multiplicative inverse of a. Dies if a is 0.
sub inverse {
    my ($self, $a) = @_;
    die "GF256::Field: zero has no multiplicative inverse" if $a == 0;
    return $self->{alog}[ 255 - $self->{log}[$a] ];
}

package CodingAdventures::GF256;

1;

__END__

=head1 NAME

CodingAdventures::GF256 - Arithmetic in the Galois Field GF(2^8)

=head1 SYNOPSIS

    use CodingAdventures::GF256 qw(add subtract multiply divide power inverse);

    # Addition is XOR
    my $sum = add(0x53, 0xCA);   # 0x53 XOR 0xCA = 0x99

    # Multiplication via LOG/ALOG tables
    my $prod = multiply(0x53, 0xCA);

    # Division
    my $quot = divide(0x53, 0xCA);

    # Power
    my $p = power(2, 8);    # g^8 mod primitive polynomial

    # Inverse
    my $inv = inverse(0x53);  # 0x53 * inverse(0x53) == 1

=head1 DESCRIPTION

Implements arithmetic in GF(2^8), the Galois Field with 256 elements.
Elements are integers 0..255, interpreted as polynomials over GF(2)
reduced modulo the primitive polynomial x^8 + x^4 + x^3 + x + 1 (0x11D).

LOG and ALOG lookup tables are built at module load time for O(1)
multiplication and division.

=head1 CONSTANTS

=over 4

=item C<$ZERO>

The additive identity: 0.

=item C<$ONE>

The multiplicative identity: 1.

=item C<$PRIMITIVE_POLYNOMIAL>

The irreducible polynomial used for reduction: 0x11D = 285.

=back

=head1 FUNCTIONS

=over 4

=item C<add($a, $b)>

Bitwise XOR. Never fails.

=item C<subtract($a, $b)>

Same as add. In GF(2^8), subtraction equals addition.

=item C<multiply($a, $b)>

GF(2^8) multiplication via LOG/ALOG tables. Returns 0 if either argument is 0.

=item C<divide($a, $b)>

GF(2^8) division. Dies C<"division by zero in GF256"> if C<$b == 0>.

=item C<power($base, $exp)>

Raise C<$base> to the C<$exp> power. C<$exp> must be a non-negative integer.

=item C<inverse($a)>

Multiplicative inverse. Dies C<"inverse of zero in GF256"> if C<$a == 0>.

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
