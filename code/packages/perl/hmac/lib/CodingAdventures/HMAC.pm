package CodingAdventures::HMAC;
# HMAC — Hash-based Message Authentication Code
# RFC 2104 / FIPS 198-1, implemented from scratch in Perl.
#
# What Is HMAC?
# =============
# HMAC takes a secret key and a message and produces a fixed-size
# authentication tag that proves two things simultaneously:
#
#   1. Integrity   — the message has not been altered.
#   2. Authenticity — the sender possesses the secret key.
#
# Unlike a plain hash (which anyone can compute from a known message), an
# HMAC tag cannot be forged without the key. HMAC is used in:
#
#   - TLS 1.2 PRF (key expansion and Pseudorandom Function)
#   - JWT "HS256" and "HS512" signature algorithms
#   - WPA2 four-way handshake (PBKDF2-HMAC-SHA1)
#   - TOTP/HOTP one-time passwords (RFC 6238 / 4226)
#   - AWS Signature Version 4 request signing
#
# Why Not hash(key . message)?
# =============================
# Naively prepending the key is vulnerable to the **length extension attack**
# on Merkle-Damgård hash functions (MD5, SHA-1, SHA-256, SHA-512).
#
# A Merkle-Damgård digest equals the hash function's internal state after
# the last block. An attacker who knows hash(key . msg) knows that state
# and can resume hashing — appending arbitrary bytes — without knowing `key`.
#
# HMAC defeats this with two nested hash calls under different derived keys:
#
#   HMAC(K, M) = H((K' XOR opad) . H((K' XOR ipad) . M))
#
# The outer hash treats the inner result as a new, opaque message.
# Resuming it without knowing K' XOR opad (which requires knowing K) is
# computationally infeasible.
#
# The ipad and opad Constants
# ============================
#   ipad = 0x36 = 0011_0110  (inner pad)
#   opad = 0x5C = 0101_1100  (outer pad)
#
# They differ in exactly 4 of 8 bits — maximum Hamming distance for single-byte
# values XOR'd with the same key byte — ensuring inner_key and outer_key are
# as different as possible despite sharing source K'.
#
# The Algorithm (RFC 2104 §2)
# ============================
#   1. Normalize K to exactly block_size bytes:
#        len(K) > block_size → K' = H(K), zero-pad to block_size
#        len(K) ≤ block_size → zero-pad to block_size
#   2. inner_key = K' XOR (0x36 × block_size)
#   3. outer_key = K' XOR (0x5C × block_size)
#   4. inner     = H(inner_key . message)
#   5. return      H(outer_key . inner)
#
# Block Sizes
# ===========
#   MD5 / SHA-1 / SHA-256: 64-byte blocks
#   SHA-512:               128-byte blocks (64-bit words, 1024-bit schedule)
#
# Note on Perl Byte Representation
# ==================================
# The hash functions in this monorepo (CodingAdventures::SHA256 etc.) take a
# Perl binary string and return an array-ref of integers 0-255.
# For concatenation we convert these array-refs back to binary strings using
# pack("C*", @{$bytes}).
#
# RFC 4231 Test Vector TC1 (HMAC-SHA256)
#   key = "\x0b" x 20
#   msg = "Hi There"
#   tag = "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

use strict;
use warnings;
use Exporter 'import';

use CodingAdventures::Md5;
use CodingAdventures::Sha1;
use CodingAdventures::SHA256;
use CodingAdventures::Sha512;

our $VERSION = '0.1.0';

our @EXPORT_OK = qw(
    hmac
    hmac_md5     hmac_md5_hex
    hmac_sha1    hmac_sha1_hex
    hmac_sha256  hmac_sha256_hex
    hmac_sha512  hmac_sha512_hex
);

# ipad = 0x36 (XOR'd with K' to form inner_key)
# opad = 0x5C (XOR'd with K' to form outer_key)
#
# Maximum Hamming distance between the two pads ensures that even when both
# are derived from the same key K', the inner and outer derived keys are
# as different as possible.
use constant IPAD => 0x36;
use constant OPAD => 0x5C;

# ---------------------------------------------------------------------------
# hmac($hash_fn, $block_size, $key, $message) -> $bytes_aref
#
# Generic HMAC over any hash function.
#
# Parameters:
#   $hash_fn    — code ref: ($binary_string) -> $bytes_aref (arrayref of ints)
#   $block_size — internal block size in bytes (64 or 128)
#   $key        — binary string, any length
#   $message    — binary string, any length
#
# Returns an array-ref of unsigned bytes (integers 0-255).
# ---------------------------------------------------------------------------
sub hmac {
    my ($hash_fn, $block_size, $key, $message) = @_;

    # Step 1 — normalize key to exactly $block_size bytes
    my $key_prime = _normalize_key($hash_fn, $block_size, $key);

    # Step 2 — derive inner and outer padded keys
    my $inner_key = _xor_fill($key_prime, IPAD);
    my $outer_key = _xor_fill($key_prime, OPAD);

    # Step 3 — inner hash: H(inner_key || message)
    my $inner_bytes = $hash_fn->($inner_key . $message);
    my $inner_str   = pack("C*", @{$inner_bytes});

    # Step 4 — outer hash: H(outer_key || inner)
    return $hash_fn->($outer_key . $inner_str);
}

# ---------------------------------------------------------------------------
# Named variants
# ---------------------------------------------------------------------------

# HMAC-MD5: returns arrayref of 16 bytes (RFC 2202).
#
# HMAC-MD5 remains secure as a MAC even though MD5 is broken for collision
# resistance — MAC security and collision resistance are different properties.
sub hmac_md5 {
    my ($key, $message) = @_;
    return hmac(sub { CodingAdventures::Md5::digest($_[0]) }, 64, $key, $message);
}

# HMAC-SHA1: returns arrayref of 20 bytes (RFC 2202).
#
# Used in WPA2 (PBKDF2-HMAC-SHA1), older TLS/SSH, and TOTP/HOTP.
sub hmac_sha1 {
    my ($key, $message) = @_;
    return hmac(sub { CodingAdventures::Sha1::digest($_[0]) }, 64, $key, $message);
}

# HMAC-SHA256: returns arrayref of 32 bytes (RFC 4231).
#
# The modern default for TLS 1.3, JWT HS256, and AWS Signature V4.
sub hmac_sha256 {
    my ($key, $message) = @_;
    return hmac(sub { CodingAdventures::SHA256::sha256($_[0]) }, 64, $key, $message);
}

# HMAC-SHA512: returns arrayref of 64 bytes (RFC 4231).
#
# Used in JWT HS512. SHA-512 uses 128-byte blocks, so key normalization
# and ipad/opad arrays are 128 bytes wide.
sub hmac_sha512 {
    my ($key, $message) = @_;
    return hmac(sub { CodingAdventures::Sha512::digest($_[0]) }, 128, $key, $message);
}

# ---------------------------------------------------------------------------
# Hex-string variants
# ---------------------------------------------------------------------------

sub hmac_md5_hex    { join('', map { sprintf('%02x', $_) } @{ hmac_md5(@_) }) }
sub hmac_sha1_hex   { join('', map { sprintf('%02x', $_) } @{ hmac_sha1(@_) }) }
sub hmac_sha256_hex { join('', map { sprintf('%02x', $_) } @{ hmac_sha256(@_) }) }
sub hmac_sha512_hex { join('', map { sprintf('%02x', $_) } @{ hmac_sha512(@_) }) }

# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------

# Normalize key to exactly $block_size bytes (binary string).
# If len(key) > block_size, hash it first (converts array-ref to string).
# Zero-pad on the right to block_size.
sub _normalize_key {
    my ($hash_fn, $block_size, $key) = @_;
    if (length($key) > $block_size) {
        my $hashed = $hash_fn->($key);
        $key = pack("C*", @{$hashed});
    }
    # Zero-pad to block_size
    if (length($key) < $block_size) {
        $key .= "\x00" x ($block_size - length($key));
    }
    return $key;
}

# XOR every byte of a binary string with constant $fill.
# Returns a new binary string of the same length.
sub _xor_fill {
    my ($s, $fill) = @_;
    return join('', map { chr(ord($_) ^ $fill) } split(//, $s));
}

1;
