package CodingAdventures::X25519;

# ============================================================================
# X25519: Elliptic Curve Diffie-Hellman on Curve25519 (RFC 7748)
# ============================================================================
#
# This module implements X25519, a key agreement protocol based on the
# Montgomery form of Curve25519. It was designed by Daniel Bernstein and
# is used in TLS 1.3, Signal Protocol, WireGuard, and many other systems.
#
# The algorithm performs scalar multiplication on an elliptic curve using
# only the x-coordinate (called the "u-coordinate" in Montgomery form).
# This simplification makes the algorithm both fast and resistant to
# certain side-channel attacks.
#
# All arithmetic operates in the prime field GF(2^255 - 19).
# We use Perl's Math::BigInt (a core module) for arbitrary-precision integers.
# ============================================================================

use strict;
use warnings;
use Math::BigInt;
use Exporter 'import';

our $VERSION = '0.01';
our @EXPORT_OK = qw(x25519 x25519_base generate_keypair);

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# The prime p = 2^255 - 19. This is the modulus for all field arithmetic.
# Bernstein chose this prime because it's the largest prime less than 2^255,
# and its proximity to a power of 2 enables fast modular reduction.
my $P = Math::BigInt->new(2)->bpow(255)->bsub(19);

# The constant a24 = 121665 = (A - 2) / 4 where A = 486662 is the curve
# parameter. This constant appears in the Montgomery ladder's differential
# addition formula.
#
# Note: RFC 7748 states a24 = 121666 = (A+2)/4, but the Montgomery ladder
# formula z_2 = E * (AA + a24 * E) actually requires (A-2)/4 = 121665
# to produce correct results. The RFC's own test vectors confirm 121665.
my $A24 = Math::BigInt->new(121665);

# The base point u-coordinate. For Curve25519, this is simply 9.
# This point generates the prime-order subgroup of the curve.
my $BASE_POINT = Math::BigInt->new(9);

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

# x25519($scalar, $u_point) -> $result
#
# Perform X25519 scalar multiplication: compute scalar * u_point.
# Both $scalar and $u_point are 32-byte strings.
# Returns a 32-byte string result.
#
# Dies if the result is all-zeros (degenerate input point).
sub x25519 {
    my ($scalar, $u_point) = @_;

    die "scalar must be 32 bytes" unless length($scalar) == 32;
    die "u_point must be 32 bytes" unless length($u_point) == 32;

    # Step 1: Clamp the scalar.
    # Clamping serves three critical purposes:
    #   - Clear low 3 bits: makes scalar divisible by 8 (the cofactor),
    #     preventing small-subgroup attacks.
    #   - Clear bit 255: keeps scalar under 2^255.
    #   - Set bit 254: ensures the Montgomery ladder always processes exactly
    #     255 iterations, preventing timing side-channels.
    my $k = _clamp_scalar($scalar);

    # Step 2: Decode the u-coordinate from little-endian bytes.
    # We mask the high bit (bit 255) to zero per RFC 7748.
    my $u = _decode_u_coordinate($u_point);

    # Step 3: Run the Montgomery ladder.
    my $result = _montgomery_ladder($k, $u);

    # Step 4: Encode the result as 32 little-endian bytes.
    my $output = _encode_u_coordinate($result);

    # Step 5: Reject all-zeros output (indicates degenerate input).
    die "X25519 produced all-zeros output (low-order input point)"
        if $output eq ("\x00" x 32);

    return $output;
}

# x25519_base($scalar) -> $public_key
#
# Compute the X25519 public key by multiplying the standard base point (u=9).
# $scalar is a 32-byte string (the private key).
# Returns a 32-byte string (the public key).
sub x25519_base {
    my ($scalar) = @_;
    return x25519($scalar, _encode_u_coordinate($BASE_POINT));
}

# generate_keypair($private_key) -> ($private_key, $public_key)
#
# Generate a public key from a private key. Returns both as a list.
sub generate_keypair {
    my ($private_key) = @_;
    return ($private_key, x25519_base($private_key));
}

# ---------------------------------------------------------------------------
# Scalar Clamping
# ---------------------------------------------------------------------------
# The scalar (private key) is "clamped" before use. This is a REQUIRED part
# of the X25519 specification, not an optimization.
#
# Three modifications to the 32-byte scalar:
#   byte[0]  &= 248  — Clear bits 0, 1, 2 (make divisible by cofactor 8)
#   byte[31] &= 127  — Clear bit 255 (keep under 2^255)
#   byte[31] |= 64   — Set bit 254 (constant-time: always 255 ladder steps)
#
# Why divisible by 8? Curve25519 has cofactor h=8. Multiplying by a multiple
# of 8 "kills" any small-subgroup component of the input point.
#
# Why set bit 254? Guarantees the Montgomery ladder always starts from a
# known '1' bit, making execution time independent of the secret scalar.

sub _clamp_scalar {
    my ($bytes) = @_;
    my @k = unpack("C*", $bytes);

    $k[0]  &= 248;
    $k[31] &= 127;
    $k[31] |= 64;

    return _decode_le(\@k);
}

# ---------------------------------------------------------------------------
# U-Coordinate Encoding / Decoding
# ---------------------------------------------------------------------------

# Decode a u-coordinate from 32 little-endian bytes.
# Per RFC 7748, the high bit (bit 255) is masked to zero.
sub _decode_u_coordinate {
    my ($bytes) = @_;
    my @u = unpack("C*", $bytes);
    $u[31] &= 127;    # mask bit 255
    return _decode_le(\@u);
}

# Encode an integer as a 32-byte little-endian string.
sub _encode_u_coordinate {
    my ($n) = @_;
    $n = $n->copy()->bmod($P);
    return _encode_le($n, 32);
}

# ---------------------------------------------------------------------------
# Little-Endian Byte Conversion
# ---------------------------------------------------------------------------

# Decode a byte array (little-endian) into a Math::BigInt.
# Little-endian means byte[0] is least significant.
sub _decode_le {
    my ($bytes_ref) = @_;
    my $result = Math::BigInt->new(0);
    for my $i (0 .. $#$bytes_ref) {
        if ($bytes_ref->[$i] != 0) {
            $result->badd(
                Math::BigInt->new($bytes_ref->[$i])->blsft(8 * $i)
            );
        }
    }
    return $result;
}

# Encode a Math::BigInt as a byte string of given length, little-endian.
sub _encode_le {
    my ($n, $len) = @_;
    my $result = '';
    my $val = $n->copy();
    for my $i (0 .. $len - 1) {
        my $byte = $val->copy()->band(Math::BigInt->new(255));
        $result .= chr($byte->numify());
        $val->brsft(8);
    }
    return $result;
}

# ---------------------------------------------------------------------------
# Montgomery Ladder
# ---------------------------------------------------------------------------
# The Montgomery ladder computes scalar multiplication on an elliptic curve
# using only the x-coordinate. Invented by Peter Montgomery in 1987.
#
# Key insight: we maintain two points whose difference is always the base
# point. This "differential" relationship lets us add and double points
# using only x-coordinates (no y needed).
#
# For each bit of the scalar (from MSB to LSB):
#   - bit=0: double the first point, add the two points
#   - bit=1: double the second point, add the two points
#
# The conditional swap (cswap) trick handles both cases uniformly.
#
# Points use projective coordinates (X, Z) where affine x = X/Z.
# This avoids expensive inversions during the ladder — only one at the end.

sub _montgomery_ladder {
    my ($k, $u) = @_;

    # Initial state:
    #   (x_2, z_2) = (1, 0) — point at infinity (identity element)
    #   (x_3, z_3) = (u, 1) — the input base point
    #   x_1 = u — saved for differential addition formula
    my $x_1 = $u->copy();
    my $x_2 = Math::BigInt->new(1);
    my $z_2 = Math::BigInt->new(0);
    my $x_3 = $u->copy();
    my $z_3 = Math::BigInt->new(1);
    my $swap = 0;

    # Process bits 254 down to 0 (255 iterations).
    # Start at 254 because bit 255 is always 0 (cleared by clamping)
    # and bit 254 is always 1 (set by clamping).
    for (my $t = 254; $t >= 0; $t--) {
        # Extract bit t of the scalar
        my $k_t = $k->copy()->brsft($t)->band(Math::BigInt->new(1))->numify();

        # XOR with accumulated swap
        $swap ^= $k_t;

        # Conditional swap based on accumulated swap value
        if ($swap) {
            ($x_2, $x_3) = ($x_3, $x_2);
            ($z_2, $z_3) = ($z_3, $z_2);
        }
        $swap = $k_t;

        # ---------------------------------------------------------------
        # Combined doubling and differential addition
        # ---------------------------------------------------------------

        # A = x_2 + z_2, B = x_2 - z_2
        my $A  = _field_add($x_2, $z_2);
        my $AA = _field_mul($A, $A);
        my $B  = _field_sub($x_2, $z_2);
        my $BB = _field_mul($B, $B);

        # E = AA - BB = (X+Z)^2 - (X-Z)^2 = 4*X*Z
        my $E  = _field_sub($AA, $BB);

        # C = x_3 + z_3, D = x_3 - z_3
        my $C  = _field_add($x_3, $z_3);
        my $D  = _field_sub($x_3, $z_3);

        # Cross-multiply for differential addition
        my $DA = _field_mul($D, $A);
        my $CB = _field_mul($C, $B);

        # New x_3 = (DA + CB)^2
        my $da_plus_cb = _field_add($DA, $CB);
        $x_3 = _field_mul($da_plus_cb, $da_plus_cb);

        # New z_3 = x_1 * (DA - CB)^2
        my $da_minus_cb = _field_sub($DA, $CB);
        $z_3 = _field_mul($x_1, _field_mul($da_minus_cb, $da_minus_cb));

        # New x_2 = AA * BB (doubling: x result)
        $x_2 = _field_mul($AA, $BB);

        # New z_2 = E * (AA + a24 * E)
        # The a24 term comes from the curve parameter: a24 = (A-2)/4 = 121665
        $z_2 = _field_mul($E, _field_add($AA, _field_mul($A24, $E)));
    }

    # Final conditional swap
    if ($swap) {
        ($x_2, $x_3) = ($x_3, $x_2);
        ($z_2, $z_3) = ($z_3, $z_2);
    }

    # Convert projective to affine: result = x_2 * z_2^(p-2) mod p
    # Fermat's little theorem: z^(p-2) ≡ z^(-1) (mod p) for prime p.
    my $z_inv = $z_2->copy()->bmodpow($P - 2, $P);
    return _field_mul($x_2, $z_inv);
}

# ---------------------------------------------------------------------------
# Field Arithmetic over GF(2^255 - 19)
# ---------------------------------------------------------------------------
# All operations are modulo p = 2^255 - 19.
# We use Math::BigInt (Perl core) for arbitrary-precision arithmetic.
# Math::BigInt is slow but correct — perfectly fine for educational use.

# Field addition: (a + b) mod p
sub _field_add {
    my ($a, $b) = @_;
    return $a->copy()->badd($b)->bmod($P);
}

# Field subtraction: (a - b) mod p
sub _field_sub {
    my ($a, $b) = @_;
    return $a->copy()->badd($P)->bsub($b)->bmod($P);
}

# Field multiplication: (a * b) mod p
sub _field_mul {
    my ($a, $b) = @_;
    return $a->copy()->bmul($b)->bmod($P);
}

1;

__END__

=head1 NAME

CodingAdventures::X25519 - Pure Perl X25519 (RFC 7748) Diffie-Hellman

=head1 SYNOPSIS

    use CodingAdventures::X25519 qw(x25519 x25519_base generate_keypair);

    # Generate a keypair (private key should be 32 random bytes)
    my $private_key = ...; # 32 random bytes
    my $public_key = x25519_base($private_key);

    # Diffie-Hellman key exchange
    my $shared_secret = x25519($my_private, $their_public);

    # Or generate both at once
    my ($priv, $pub) = generate_keypair($private_key);

=head1 DESCRIPTION

This module implements X25519 (RFC 7748), elliptic curve Diffie-Hellman on
Curve25519. All field arithmetic over GF(2^255-19) is implemented from
scratch using Math::BigInt (a Perl core module).

=head1 FUNCTIONS

=over 4

=item x25519($scalar, $u_point)

Compute scalar * u_point. Both are 32-byte strings. Returns 32-byte string.

=item x25519_base($scalar)

Compute scalar * base_point (u=9). Returns 32-byte public key.

=item generate_keypair($private_key)

Returns ($private_key, $public_key).

=back

=cut
