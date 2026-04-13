package CodingAdventures::ChaCha20Poly1305;

# ============================================================================
# CodingAdventures::ChaCha20Poly1305 — RFC 8439 Authenticated Encryption
# ============================================================================
#
# This module implements ChaCha20-Poly1305, an authenticated encryption
# scheme combining the ChaCha20 stream cipher with the Poly1305 MAC.
# Designed by Daniel J. Bernstein and standardized in RFC 8439.
#
# Three components:
#
#   1. **ChaCha20** — A stream cipher using only ARX operations (Add, Rotate,
#      XOR). No lookup tables, no timing side-channels. Fast in pure software.
#
#   2. **Poly1305** — A one-time message authentication code. Given a one-time
#      key, it produces a 16-byte tag that proves the message wasn't tampered.
#
#   3. **AEAD** — The combined construction from RFC 8439 Section 2.8 that
#      provides both confidentiality (encryption) and integrity (authentication).
#
# Why ChaCha20 over AES?
# ~~~~~~~~~~~~~~~~~~~~~~
# AES is fast only with hardware AES-NI instructions. Without them (common on
# mobile), AES is slow and vulnerable to cache-timing attacks. ChaCha20 uses
# only add/rotate/XOR — constant-time on every CPU.
#
# Used in: TLS 1.3, WireGuard, SSH, Chrome/Android HTTPS.
#
# IMPORTANT: Educational implementation. Use a vetted library for real crypto.
# ============================================================================

use strict;
use warnings;
use Math::BigInt;

our $VERSION = '0.01';

# ============================================================================
# Constants
# ============================================================================
#
# The ChaCha20 state matrix starts with four magic constants that spell
# "expand 32-byte k" in ASCII (little-endian 32-bit words):
#
#   0x61707865 = "expa"
#   0x3320646e = "nd 3"
#   0x79622d32 = "2-by"
#   0x6b206574 = "te k"
# ============================================================================

use constant MASK32 => 0xFFFFFFFF;
use constant {
    CONST0 => 0x61707865,
    CONST1 => 0x3320646e,
    CONST2 => 0x79622d32,
    CONST3 => 0x6b206574,
};

# ============================================================================
# 32-bit Arithmetic Helpers
# ============================================================================
#
# Perl integers can be 64-bit, so we mask to 32 bits after additions.
# The left-rotate is: ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
# ============================================================================

## add32($a, $b) — add with 32-bit wrapping
sub add32 { ($_[0] + $_[1]) & MASK32 }

## rotl32($x, $n) — left rotate a 32-bit value by n bits
sub rotl32 { (($_[0] << $_[1]) | ($_[0] >> (32 - $_[1]))) & MASK32 }

## le32($bytes, $offset) — read little-endian 32-bit uint from byte string
sub le32 {
    unpack('V', substr($_[0], $_[1], 4))
}

## to_le32($n) — encode 32-bit uint as 4-byte little-endian string
sub to_le32 { pack('V', $_[0]) }

## to_le64($n) — encode 64-bit uint as 8-byte little-endian string
sub to_le64 { pack('Q<', $_[0]) }

# ============================================================================
# ChaCha20 Quarter Round
# ============================================================================
#
# The quarter round is the core mixing operation. It operates on four
# 32-bit words using the ARX (Add-Rotate-XOR) pattern:
#
#   a += b; d ^= a; d <<<= 16
#   c += d; b ^= c; b <<<= 12
#   a += b; d ^= a; d <<<= 8
#   c += d; b ^= c; b <<<= 7
#
# The rotation constants (16, 12, 8, 7) maximize diffusion.
# ============================================================================

## quarter_round(\@state, $a, $b, $c, $d) — in-place quarter round
sub quarter_round {
    my ($s, $a, $b, $c, $d) = @_;

    $s->[$a] = add32($s->[$a], $s->[$b]);
    $s->[$d] = rotl32($s->[$d] ^ $s->[$a], 16);

    $s->[$c] = add32($s->[$c], $s->[$d]);
    $s->[$b] = rotl32($s->[$b] ^ $s->[$c], 12);

    $s->[$a] = add32($s->[$a], $s->[$b]);
    $s->[$d] = rotl32($s->[$d] ^ $s->[$a],  8);

    $s->[$c] = add32($s->[$c], $s->[$d]);
    $s->[$b] = rotl32($s->[$b] ^ $s->[$c],  7);
}

# ============================================================================
# ChaCha20 Block Function
# ============================================================================
#
# The state is a 4x4 matrix of 32-bit words:
#
#    [ CONST0  CONST1  CONST2  CONST3 ]   ← magic constants
#    [ key[0]  key[1]  key[2]  key[3] ]   ← first half of key
#    [ key[4]  key[5]  key[6]  key[7] ]   ← second half of key
#    [ counter nonce0  nonce1  nonce2 ]   ← counter + nonce
#
# 20 rounds = 10 × (column round + diagonal round).
# After the rounds, the original state is added back (feed-forward).
# Output: 64-byte keystream block.
# ============================================================================

## chacha20_block($key, $nonce, $counter) → 64-byte string
sub chacha20_block {
    my ($key, $nonce, $counter) = @_;

    # Initialize the 16-word state.
    my @state = (
        CONST0, CONST1, CONST2, CONST3,
        le32($key,  0), le32($key,  4), le32($key,  8), le32($key, 12),
        le32($key, 16), le32($key, 20), le32($key, 24), le32($key, 28),
        $counter & MASK32,
        le32($nonce, 0), le32($nonce, 4), le32($nonce, 8),
    );

    # Save a copy for the feed-forward.
    my @initial = @state;

    # 20 rounds: 10 iterations of column round + diagonal round.
    #
    # Column rounds mix within columns of the 4x4 matrix:
    #   QR(0,4, 8,12)  QR(1,5, 9,13)  QR(2,6,10,14)  QR(3,7,11,15)
    #
    # Diagonal rounds mix across diagonals:
    #   QR(0,5,10,15)  QR(1,6,11,12)  QR(2,7, 8,13)  QR(3,4, 9,14)
    for (1 .. 10) {
        # Column round
        quarter_round(\@state, 0, 4,  8, 12);
        quarter_round(\@state, 1, 5,  9, 13);
        quarter_round(\@state, 2, 6, 10, 14);
        quarter_round(\@state, 3, 7, 11, 15);
        # Diagonal round
        quarter_round(\@state, 0, 5, 10, 15);
        quarter_round(\@state, 1, 6, 11, 12);
        quarter_round(\@state, 2, 7,  8, 13);
        quarter_round(\@state, 3, 4,  9, 14);
    }

    # Feed-forward: add the original state to prevent inversion.
    my $output = '';
    for my $i (0 .. 15) {
        $output .= to_le32(add32($state[$i], $initial[$i]));
    }

    return $output;
}

# ============================================================================
# ChaCha20 Stream Cipher
# ============================================================================
#
# Encrypts by XOR-ing the plaintext with a keystream generated 64 bytes at a
# time. Since XOR is its own inverse, decryption is the same operation.
# ============================================================================

## chacha20_encrypt($data, $key, $nonce, $counter) → encrypted bytes
sub chacha20_encrypt {
    my ($data, $key, $nonce, $counter) = @_;
    $counter //= 0;

    die "Key must be 32 bytes" unless length($key) == 32;
    die "Nonce must be 12 bytes" unless length($nonce) == 12;

    my $result = '';
    my $data_len = length($data);

    for (my $offset = 0; $offset < $data_len; $offset += 64) {
        my $block = chacha20_block($key, $nonce, $counter);
        $counter++;

        my $chunk_len = ($data_len - $offset < 64) ? ($data_len - $offset) : 64;
        for my $i (0 .. $chunk_len - 1) {
            my $p = ord(substr($data, $offset + $i, 1));
            my $k = ord(substr($block, $i, 1));
            $result .= chr($p ^ $k);
        }
    }

    return $result;
}

# ============================================================================
# Poly1305 Message Authentication Code
# ============================================================================
#
# Poly1305 authenticates a message using polynomial evaluation modulo a prime
# p = 2^130 - 5. It is a "one-time" MAC — the key must never be reused.
#
# Algorithm:
#   1. Split the 32-byte key into r (16 bytes, clamped) and s (16 bytes).
#   2. For each 16-byte block of the message:
#      a. Read the block as a little-endian integer.
#      b. Append a 0x01 byte (the "sentinel" — adds 2^(8*block_len)).
#      c. acc = ((acc + block_with_sentinel) * r) mod (2^130 - 5)
#   3. tag = (acc + s) mod 2^128
#
# Big Integer Arithmetic:
#   Poly1305 requires 130-bit arithmetic. We use Perl's Math::BigInt (core
#   module) for the modular arithmetic. While slower than hand-optimized
#   limb arithmetic, it is correct and educational.
# ============================================================================

## The prime p = 2^130 - 5
my $P = Math::BigInt->new(2)->bpow(130)->bsub(5);

## The modulus for the final tag: 2^128
my $MOD128 = Math::BigInt->new(2)->bpow(128);

## clamp_r($r_bytes) → clamped 16-byte string
#
# Forces specific bits of r to zero per RFC 8439 Section 2.5.
# 0-indexed: bytes 3,7,11,15 AND 0x0F; bytes 4,8,12 AND 0xFC.
sub clamp_r {
    my ($r) = @_;
    my @bytes = unpack('C*', $r);

    # Clamp (0-indexed byte positions):
    $bytes[3]  &= 0x0F;
    $bytes[4]  &= 0xFC;
    $bytes[7]  &= 0x0F;
    $bytes[8]  &= 0xFC;
    $bytes[11] &= 0x0F;
    $bytes[12] &= 0xFC;
    $bytes[15] &= 0x0F;

    return pack('C*', @bytes);
}

## bytes_to_bigint($bytes) → Math::BigInt
#
# Convert a little-endian byte string to a Math::BigInt.
sub bytes_to_bigint {
    my ($bytes) = @_;
    my $n = Math::BigInt->new(0);
    my @b = unpack('C*', $bytes);

    # Process bytes from most-significant to least (reverse of little-endian).
    for my $i (reverse 0 .. $#b) {
        $n->blsft(8);
        $n->badd($b[$i]);
    }
    return $n;
}

## bigint_to_bytes16($n) → 16-byte string
#
# Convert a Math::BigInt to a 16-byte little-endian string.
# Only the lower 128 bits are kept (implicit mod 2^128).
sub bigint_to_bytes16 {
    my ($n) = @_;
    my $val = $n->copy();
    my @bytes;

    for (1 .. 16) {
        my $byte = $val->copy()->band(Math::BigInt->new(0xFF));
        push @bytes, $byte->numify();
        $val->brsft(8);
    }

    return pack('C*', @bytes);
}

## poly1305_mac($message, $key) → 16-byte tag
sub poly1305_mac {
    my ($message, $key) = @_;
    die "Poly1305 key must be 32 bytes" unless length($key) == 32;

    # Split the key: first 16 bytes = r (clamped), last 16 = s.
    my $r_bytes = clamp_r(substr($key, 0, 16));
    my $s_bytes = substr($key, 16, 16);

    my $r = bytes_to_bigint($r_bytes);
    my $s = bytes_to_bigint($s_bytes);
    my $acc = Math::BigInt->new(0);

    my $msg_len = length($message);

    # Process in 16-byte blocks.
    for (my $i = 0; $i < $msg_len; $i += 16) {
        my $block_len = ($msg_len - $i < 16) ? ($msg_len - $i) : 16;
        my $block = substr($message, $i, $block_len);

        # Append the 0x01 sentinel byte.
        my $n = bytes_to_bigint($block . "\x01");

        # acc = ((acc + n) * r) mod p
        $acc->badd($n);
        $acc->bmul($r);
        $acc->bmod($P);
    }

    # tag = (acc + s) mod 2^128
    $acc->badd($s);
    $acc->bmod($MOD128);

    return bigint_to_bytes16($acc);
}

# ============================================================================
# AEAD: Authenticated Encryption with Associated Data (RFC 8439 §2.8)
# ============================================================================
#
# Encryption:
#   1. poly_key = first 32 bytes of ChaCha20(key, nonce, counter=0)
#   2. ciphertext = ChaCha20(plaintext, key, nonce, counter=1)
#   3. mac_data = AAD || pad16(AAD) || CT || pad16(CT) ||
#                 le64(len(AAD)) || le64(len(CT))
#   4. tag = Poly1305(poly_key, mac_data)
#
# Decryption:
#   1. Verify the tag FIRST (don't release unauthenticated plaintext).
#   2. If valid, decrypt with ChaCha20(ciphertext, key, nonce, counter=1).
# ============================================================================

## pad16($data) → zero-padding to 16-byte boundary
sub pad16 {
    my ($data) = @_;
    my $r = length($data) % 16;
    return ($r == 0) ? '' : ("\0" x (16 - $r));
}

## build_mac_data($aad, $ciphertext) → MAC input per RFC 8439
sub build_mac_data {
    my ($aad, $ct) = @_;
    return $aad . pad16($aad)
         . $ct  . pad16($ct)
         . to_le64(length($aad))
         . to_le64(length($ct));
}

## constant_time_equal($a, $b) → boolean
#
# Compare two byte strings in constant time to prevent timing attacks.
sub constant_time_equal {
    my ($a, $b) = @_;
    return 0 if length($a) != length($b);
    my $diff = 0;
    for my $i (0 .. length($a) - 1) {
        $diff |= ord(substr($a, $i, 1)) ^ ord(substr($b, $i, 1));
    }
    return $diff == 0;
}

## aead_encrypt($plaintext, $key, $nonce, $aad) → ($ciphertext, $tag)
sub aead_encrypt {
    my ($plaintext, $key, $nonce, $aad) = @_;
    $aad //= '';

    die "Key must be 32 bytes"   unless length($key) == 32;
    die "Nonce must be 12 bytes" unless length($nonce) == 12;

    # Step 1: Generate the Poly1305 one-time key from counter=0.
    my $poly_key = substr(
        chacha20_encrypt("\0" x 32, $key, $nonce, 0),
        0, 32
    );

    # Step 2: Encrypt with counter=1.
    my $ct = chacha20_encrypt($plaintext, $key, $nonce, 1);

    # Step 3: Compute the authentication tag.
    my $mac_data = build_mac_data($aad, $ct);
    my $tag = poly1305_mac($mac_data, $poly_key);

    return ($ct, $tag);
}

## aead_decrypt($ciphertext, $key, $nonce, $aad, $tag) → $plaintext or undef
sub aead_decrypt {
    my ($ct, $key, $nonce, $aad, $tag) = @_;
    $aad //= '';

    die "Key must be 32 bytes"   unless length($key) == 32;
    die "Nonce must be 12 bytes" unless length($nonce) == 12;
    die "Tag must be 16 bytes"   unless length($tag) == 16;

    # Step 1: Generate the Poly1305 one-time key.
    my $poly_key = substr(
        chacha20_encrypt("\0" x 32, $key, $nonce, 0),
        0, 32
    );

    # Step 2: Verify the tag BEFORE decrypting.
    my $mac_data = build_mac_data($aad, $ct);
    my $expected_tag = poly1305_mac($mac_data, $poly_key);

    unless (constant_time_equal($tag, $expected_tag)) {
        return (undef, "authentication failed");
    }

    # Step 3: Decrypt.
    my $plaintext = chacha20_encrypt($ct, $key, $nonce, 1);
    return ($plaintext, undef);
}

1;

__END__

=head1 NAME

CodingAdventures::ChaCha20Poly1305 - ChaCha20-Poly1305 AEAD (RFC 8439)

=head1 SYNOPSIS

    use CodingAdventures::ChaCha20Poly1305;

    # Stream cipher
    my $ct = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        $plaintext, $key_32, $nonce_12, $counter
    );

    # MAC
    my $tag = CodingAdventures::ChaCha20Poly1305::poly1305_mac(
        $message, $key_32
    );

    # AEAD
    my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        $plaintext, $key_32, $nonce_12, $aad
    );

    my ($pt, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $ct, $key_32, $nonce_12, $aad, $tag
    );

=head1 DESCRIPTION

Educational implementation of ChaCha20-Poly1305 authenticated encryption.
Do not use for real cryptography.

=cut
