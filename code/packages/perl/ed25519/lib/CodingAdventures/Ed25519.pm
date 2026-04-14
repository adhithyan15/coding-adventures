package CodingAdventures::Ed25519;

# ============================================================================
# Ed25519: Digital Signatures on the Edwards Curve (RFC 8032)
# ============================================================================
#
# Ed25519 is an elliptic curve digital signature algorithm (EdDSA) designed by
# Daniel J. Bernstein et al.  It uses the twisted Edwards curve:
#
#     -x^2 + y^2 = 1 + d*x^2*y^2    (mod p)
#
# where p = 2^255 - 19.  Ed25519 provides:
#   - 32-byte public keys and 64-byte signatures
#   - 128-bit security level
#   - Deterministic signatures (no random nonce needed)
#   - Fast signing and verification
#
# ARCHITECTURE
# ============
# Perl has Math::BigInt in core, giving us arbitrary-precision integers.
# We use that for all field and scalar arithmetic.
#
# EXTENDED COORDINATES
# ====================
# Points on the curve are represented as (X, Y, Z, T) where:
#   x = X/Z,  y = Y/Z,  T = X*Y/Z
#
# The identity point is (0, 1, 1, 0) -- affine (0, 1).
# The unified addition formula works for all point pairs including
# doubling, adding the identity, and adding inverses.
# ============================================================================

use strict;
use warnings;
use Math::BigInt;

our $VERSION = '0.01';

# ============================================================================
# PART 1: CURVE CONSTANTS
# ============================================================================
# All constants are Math::BigInt objects for consistent arithmetic.

# The prime field: p = 2^255 - 19
my $P = Math::BigInt->new(2)->bpow(255)->bsub(19);

# The curve parameter d = -121665/121666 mod p
# Pre-computed: d = -121665 * inverse(121666, p) mod p
my $D = Math::BigInt->new("37095705934669439343138083508754565189542113879843219016388785533085940283555");

# The group order: L = 2^252 + 27742317777372353535851937790883648493
my $L = Math::BigInt->new(2)->bpow(252)->badd("27742317777372353535851937790883648493");

# sqrt(-1) mod p, used in the square root computation
my $SQRT_M1 = Math::BigInt->new("19681161376707505956807079304988542015446066515923890162744021073123829784752");

# Base point B coordinates
# B_y = 4/5 mod p = 4 * inverse(5, p) mod p
my $B_Y = Math::BigInt->new(4)->bmul(Math::BigInt->new(5)->bmodpow($P - 2, $P))->bmod($P);

# B_x is recovered from B_y using the curve equation.
# Pre-computed for efficiency.
my $B_X = Math::BigInt->new("15112221349535400772501151409588531511454012693041857206046113283949847762202");

# ============================================================================
# PART 2: FIELD ARITHMETIC (mod p)
# ============================================================================
# We wrap Math::BigInt operations to keep everything in the field.

sub _field_add {
    my ($a, $b) = @_;
    return ($a + $b) % $P;
}

sub _field_sub {
    my ($a, $b) = @_;
    return ($a - $b + $P) % $P;
}

sub _field_mul {
    my ($a, $b) = @_;
    return ($a * $b) % $P;
}

sub _field_sq {
    my ($a) = @_;
    return ($a * $a) % $P;
}

sub _field_neg {
    my ($a) = @_;
    return ($P - $a) % $P;
}

# Field inversion: a^(p-2) mod p  (Fermat's little theorem)
sub _field_inv {
    my ($a) = @_;
    return $a->copy()->bmodpow($P - 2, $P);
}

# Field square root for p = 5 (mod 8):
#   candidate = a^((p+3)/8) mod p
#   If candidate^2 == a:  return candidate
#   If candidate^2 == -a: return candidate * sqrt(-1) mod p
#   Else: no square root
sub _field_sqrt {
    my ($a) = @_;
    my $exp = ($P + 3) / 8;
    my $candidate = $a->copy()->bmodpow($exp, $P);
    my $check = _field_sq($candidate);

    if ($check == $a % $P) {
        return $candidate;
    }
    if ($check == _field_neg($a)) {
        return _field_mul($candidate, $SQRT_M1);
    }
    return undef;  # No square root
}

# ============================================================================
# PART 3: POINT OPERATIONS (Extended Coordinates)
# ============================================================================
# A point is represented as [$X, $Y, $Z, $T].

sub _point_identity {
    return [
        Math::BigInt->new(0),
        Math::BigInt->new(1),
        Math::BigInt->new(1),
        Math::BigInt->new(0),
    ];
}

# ---------------------------------------------------------------------------
# Point Addition (unified formula for twisted Edwards a = -1)
# ---------------------------------------------------------------------------
# A = X1*X2,  B = Y1*Y2,  C = T1*d*T2,  D = Z1*Z2
# E = (X1+Y1)*(X2+Y2) - A - B
# F = D - C,  G = D + C,  H = B + A
# X3 = E*F,  Y3 = G*H,  T3 = E*H,  Z3 = F*G

sub _point_add {
    my ($p1, $p2) = @_;
    my ($X1, $Y1, $Z1, $T1) = @$p1;
    my ($X2, $Y2, $Z2, $T2) = @$p2;

    my $A = _field_mul($X1, $X2);
    my $B = _field_mul($Y1, $Y2);
    my $C = _field_mul(_field_mul($T1, $D), $T2);
    my $DD = _field_mul($Z1, $Z2);

    my $E = _field_sub(
        _field_mul(_field_add($X1, $Y1), _field_add($X2, $Y2)),
        _field_add($A, $B)
    );
    my $F = _field_sub($DD, $C);
    my $G = _field_add($DD, $C);
    my $H = _field_add($B, $A);

    return [
        _field_mul($E, $F),
        _field_mul($G, $H),
        _field_mul($F, $G),
        _field_mul($E, $H),
    ];
}

# ---------------------------------------------------------------------------
# Point Doubling
# ---------------------------------------------------------------------------
# A = X1^2,  B = Y1^2,  C = 2*Z1^2,  D = -A
# E = (X1+Y1)^2 - A - B,  G = D + B,  F = G - C,  H = D - B
# X3 = E*F,  Y3 = G*H,  T3 = E*H,  Z3 = F*G

sub _point_double {
    my ($pt) = @_;
    my ($X1, $Y1, $Z1, $T1) = @$pt;

    my $A = _field_sq($X1);
    my $B = _field_sq($Y1);
    my $C = _field_mul(Math::BigInt->new(2), _field_sq($Z1));
    my $DD = _field_neg($A);

    my $E = _field_sub(_field_sq(_field_add($X1, $Y1)), _field_add($A, $B));
    my $G = _field_add($DD, $B);
    my $F = _field_sub($G, $C);
    my $H = _field_sub($DD, $B);

    return [
        _field_mul($E, $F),
        _field_mul($G, $H),
        _field_mul($F, $G),
        _field_mul($E, $H),
    ];
}

# ---------------------------------------------------------------------------
# Scalar Multiplication: double-and-add, high-to-low bit scanning
# ---------------------------------------------------------------------------

sub _scalar_mult {
    my ($scalar, $point) = @_;
    my $result = _point_identity();
    my $bits = length($scalar->as_bin()) - 2;  # strip '0b' prefix

    for my $i (reverse 0 .. $bits - 1) {
        $result = _point_double($result);
        if ($scalar->copy()->brsft($i)->band(1)->is_one()) {
            $result = _point_add($result, $point);
        }
    }

    return $result;
}

# ============================================================================
# PART 4: POINT ENCODING/DECODING (RFC 8032 Section 5.1.2)
# ============================================================================

# Encode a point as 32 bytes: y in little-endian, with the sign of x
# stored in the high bit of byte 31.
sub _point_encode {
    my ($pt) = @_;
    my ($X, $Y, $Z, $T) = @$pt;

    # Convert to affine: x = X/Z, y = Y/Z
    my $z_inv = _field_inv($Z);
    my $x_aff = _field_mul($X, $z_inv);
    my $y_aff = _field_mul($Y, $z_inv);

    # Encode y as 32 bytes LE
    my @y_bytes = _bigint_to_le_bytes($y_aff, 32);

    # Set the high bit of byte 31 (0-indexed) to the low bit of x
    $y_bytes[31] |= (($x_aff % 2) << 7);

    return pack("C*", @y_bytes);
}

# Decode a 32-byte encoded point.
# Returns the point in extended coordinates, or undef on failure.
sub _point_decode {
    my ($encoded) = @_;
    return undef unless length($encoded) == 32;

    my @bytes = unpack("C*", $encoded);

    # Extract the sign bit of x from the high bit of byte 31
    my $x_sign = ($bytes[31] >> 7) & 1;

    # Clear the sign bit to get y
    $bytes[31] &= 0x7F;
    my $y = _le_bytes_to_bigint(\@bytes);

    # Check y < p
    return undef if $y >= $P;

    # Compute x^2 = (y^2 - 1) * inv(d*y^2 + 1)
    my $y2 = _field_sq($y);
    my $num = _field_sub($y2, Math::BigInt->new(1));
    my $den = _field_add(_field_mul($D, $y2), Math::BigInt->new(1));
    my $den_inv = _field_inv($den);
    my $x2 = _field_mul($num, $den_inv);

    # If x^2 = 0 and sign bit is 1, invalid
    if ($x2->is_zero()) {
        return undef if $x_sign;
        return [
            Math::BigInt->new(0),
            $y->copy(),
            Math::BigInt->new(1),
            Math::BigInt->new(0),
        ];
    }

    # Compute x = sqrt(x^2)
    my $x = _field_sqrt($x2);
    return undef unless defined $x;

    # Ensure the sign matches
    if (($x % 2) != $x_sign) {
        $x = _field_neg($x);
    }

    return [
        $x,
        $y->copy(),
        Math::BigInt->new(1),
        _field_mul($x, $y),
    ];
}

# ============================================================================
# PART 5: BYTE/BIGINT CONVERSION HELPERS
# ============================================================================

# Convert a Math::BigInt to a little-endian byte array of the given length.
sub _bigint_to_le_bytes {
    my ($n, $len) = @_;
    my @bytes;
    my $tmp = $n->copy();
    for my $i (0 .. $len - 1) {
        push @bytes, ($tmp % 256)->numify();
        $tmp->brsft(8);
    }
    return @bytes;
}

# Convert a little-endian byte array ref to a Math::BigInt.
sub _le_bytes_to_bigint {
    my ($bytes_ref) = @_;
    my $result = Math::BigInt->new(0);
    for my $i (reverse 0 .. $#$bytes_ref) {
        $result->blsft(8);
        $result->badd($bytes_ref->[$i]);
    }
    return $result;
}

# ============================================================================
# PART 6: SHA-512 INTEGRATION
# ============================================================================

use CodingAdventures::Sha512;

# SHA-512 returns a byte array ref. We convert to a binary string for
# concatenation and to a Math::BigInt for arithmetic.
sub _sha512 {
    my ($data) = @_;
    my $hash_bytes = CodingAdventures::Sha512::digest($data);
    return pack("C*", @$hash_bytes);
}

sub _sha512_to_bigint {
    my ($data) = @_;
    my $hash_bytes = CodingAdventures::Sha512::digest($data);
    return _le_bytes_to_bigint($hash_bytes);
}

# ============================================================================
# PART 7: PUBLIC API
# ============================================================================

# Build the base point B in extended coordinates.
my $B = [
    $B_X->copy(),
    $B_Y->copy(),
    Math::BigInt->new(1),
    _field_mul($B_X, $B_Y),
];

# ---------------------------------------------------------------------------
# generate_keypair($seed) -> ($public_key, $secret_key)
# ---------------------------------------------------------------------------
# Takes a 32-byte seed string. Returns:
#   $public_key: 32-byte encoded point
#   $secret_key: 64-byte string (seed || public_key)

sub generate_keypair {
    my ($seed) = @_;
    die "seed must be 32 bytes" unless length($seed) == 32;

    my $h = _sha512($seed);
    my @h_bytes = unpack("C*", $h);

    # Clamp the first 32 bytes
    $h_bytes[0] &= 248;     # Clear bits 0,1,2
    $h_bytes[31] &= 127;    # Clear bit 255
    $h_bytes[31] |= 64;     # Set bit 254

    my @scalar_bytes = @h_bytes[0..31];
    my $a = _le_bytes_to_bigint(\@scalar_bytes);

    # A = a * B
    my $A = _scalar_mult($a, $B);
    my $public_key = _point_encode($A);

    # Secret key = seed || public_key
    my $secret_key = $seed . $public_key;

    return ($public_key, $secret_key);
}

# ---------------------------------------------------------------------------
# sign($message, $secret_key) -> $signature
# ---------------------------------------------------------------------------
# Creates a 64-byte deterministic signature.

sub sign {
    my ($message, $secret_key) = @_;
    die "secret_key must be 64 bytes" unless length($secret_key) == 64;

    my $seed = substr($secret_key, 0, 32);
    my $public_key = substr($secret_key, 32, 32);

    # Re-derive the scalar and prefix from the seed
    my $h = _sha512($seed);
    my @h_bytes = unpack("C*", $h);

    $h_bytes[0] &= 248;
    $h_bytes[31] &= 127;
    $h_bytes[31] |= 64;

    my @scalar_bytes = @h_bytes[0..31];
    my $a = _le_bytes_to_bigint(\@scalar_bytes);

    # prefix = last 32 bytes of SHA-512(seed)
    my $prefix = pack("C*", @h_bytes[32..63]);

    # r = SHA-512(prefix || message) mod L
    my $r_hash = _sha512_to_bigint($prefix . $message);
    my $r = $r_hash % $L;

    # R = r * B
    my $R_point = _scalar_mult($r, $B);
    my $R_enc = _point_encode($R_point);

    # k = SHA-512(R || A || message) mod L
    my $k_hash = _sha512_to_bigint($R_enc . $public_key . $message);
    my $k = $k_hash % $L;

    # S = (r + k * a) mod L
    my $S = ($r + $k * $a) % $L;

    # Encode S as 32 bytes LE
    my @s_bytes = _bigint_to_le_bytes($S, 32);
    my $S_enc = pack("C*", @s_bytes);

    return $R_enc . $S_enc;
}

# ---------------------------------------------------------------------------
# verify($message, $signature, $public_key) -> boolean
# ---------------------------------------------------------------------------

sub verify {
    my ($message, $signature, $public_key) = @_;

    return 0 unless length($signature) == 64 && length($public_key) == 32;

    my $R_enc = substr($signature, 0, 32);
    my $S_enc = substr($signature, 32, 32);

    # Decode R and A
    my $R = _point_decode($R_enc);
    return 0 unless defined $R;

    my $A = _point_decode($public_key);
    return 0 unless defined $A;

    # Decode S as a scalar
    my @s_bytes = unpack("C*", $S_enc);
    my $S = _le_bytes_to_bigint(\@s_bytes);

    # Check S < L (malleability check)
    return 0 if $S >= $L;

    # k = SHA-512(R || A || message) mod L
    my $k_hash = _sha512_to_bigint($R_enc . $public_key . $message);
    my $k = $k_hash % $L;

    # Verify: S * B == R + k * A
    my $lhs = _scalar_mult($S, $B);
    my $rhs = _point_add($R, _scalar_mult($k, $A));

    # Compare by encoding both to 32 bytes
    return _point_encode($lhs) eq _point_encode($rhs) ? 1 : 0;
}

# ---------------------------------------------------------------------------
# Hex Utilities
# ---------------------------------------------------------------------------

sub from_hex {
    my ($hex) = @_;
    return pack("H*", $hex);
}

sub to_hex {
    my ($bin) = @_;
    return unpack("H*", $bin);
}

1;
