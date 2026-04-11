package CodingAdventures::PBKDF2;

# coding_adventures::pbkdf2 -- PBKDF2 (Password-Based Key Derivation Function 2)
# RFC 8018 (formerly RFC 2898 / PKCS#5 v2.1)
#
# == What Is PBKDF2? ==
#
# PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
# function (PRF) -- typically HMAC -- c times per output block. The iteration
# count c is the tunable cost: every brute-force guess requires the same c PRF
# calls as the original derivation.
#
# Real-world uses:
#   - WPA2 Wi-Fi: PBKDF2-HMAC-SHA1, 4096 iterations
#   - Django: PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
#   - macOS Keychain: PBKDF2-HMAC-SHA256
#
# == Algorithm (RFC 8018 § 5.2) ==
#
#   DK = T_1 || T_2 || ... (first dk_len bytes)
#
#   T_i = U_1 XOR U_2 XOR ... XOR U_c
#
#   U_1 = PRF(password, salt . INT_32_BE(i))
#   U_j = PRF(password, U_{j-1})   for j = 2..c
#
# INT_32_BE(i) encodes the block counter as a 4-byte big-endian integer.
# In Perl: pack("N", $i) produces this encoding.
#
# == Note on HMAC Return Types ==
#
# The CodingAdventures::HMAC functions return array references of integers
# (byte values). PBKDF2 needs to XOR byte strings, so we convert using
# pack("C*", @$arrayref) to get a raw binary string.
#
# == Security Notes ==
#
# OWASP 2023 minimum iteration counts:
#   - HMAC-SHA256: 600,000
#   - HMAC-SHA1:   1,300,000
#
# For new systems prefer Argon2id (memory-hard, resists GPU attacks).

use strict;
use warnings;
use bytes;            # force length/substr to operate on raw bytes
use Exporter 'import';
use POSIX qw(ceil);   # ceil() for block-count computation

use CodingAdventures::HMAC qw(hmac hmac_sha1 hmac_sha256 hmac_sha512);
# CodingAdventures::Sha1, ::SHA256, and ::Sha512 are loaded transitively by
# CodingAdventures::HMAC above. We call their hash functions directly in the
# allow_empty_password path to bypass the empty-key guard in the named
# hmac_sha* wrappers.

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(
    pbkdf2_hmac_sha1   pbkdf2_hmac_sha1_hex
    pbkdf2_hmac_sha256 pbkdf2_hmac_sha256_hex
    pbkdf2_hmac_sha512 pbkdf2_hmac_sha512_hex
);

# ──────────────────────────────────────────────────────────────────────────────
# Helpers
# ──────────────────────────────────────────────────────────────────────────────

# _arrayref_to_str: convert an arrayref of byte integers to a raw binary string.
# The HMAC functions in this monorepo return [ 0..255, ... ] — pack turns those
# into a byte string that Perl string operations can work with.
sub _arrayref_to_str {
    my ($aref) = @_;
    return pack("C*", @$aref);
}

# _xor_str: XOR two equal-length binary strings, returning a new string.
sub _xor_str {
    my ($a, $b) = @_;
    my $len = length($a);
    my $result = $a ^ $b;   # Perl's ^ operator XORs strings character by character
    return substr($result, 0, $len);
}

# _to_hex: convert a binary string to a lowercase hex string.
sub _to_hex {
    my ($s) = @_;
    return join('', map { sprintf('%02x', ord($_)) } split(//, $s));
}

# ──────────────────────────────────────────────────────────────────────────────
# Core loop
# ──────────────────────────────────────────────────────────────────────────────

# _pbkdf2: generic PBKDF2 loop.
#
# $prf:                coderef: ($key, $msg) -> raw byte string of length $h_len
# $h_len:              output byte length of $prf
# $password:           secret being stretched (raw byte string)
# $salt:               unique random value per credential (raw byte string)
# $iterations:         number of PRF calls per block
# $key_length:         number of derived bytes
# $allow_empty_password: optional boolean (default 0). When true, an empty
#                      password is permitted. Used by scrypt, which wraps
#                      PBKDF2 for format conversion only; the memory-hard
#                      ROMix layer provides the actual brute-force resistance.
sub _pbkdf2 {
    my ($prf, $h_len, $password, $salt, $iterations, $key_length, $allow_empty_password) = @_;
    $allow_empty_password //= 0;

    die "PBKDF2 password must not be empty\n"
        unless defined($password) && (length($password) > 0 || $allow_empty_password);

    # The upper bounds (2**31 for iterations, 2**20 for key_length) prevent
    # an attacker from supplying arbitrarily large values that would cause
    # unbounded loops or massive memory allocation.
    die "PBKDF2 iterations must be a positive integer\n"
        unless defined($iterations) && $iterations =~ /^\d+$/ && $iterations > 0 && $iterations <= 2**31;

    die "PBKDF2 key_length must be a positive integer\n"
        unless defined($key_length) && $key_length =~ /^\d+$/ && $key_length > 0 && $key_length <= 2**20;

    my $num_blocks = ceil($key_length / $h_len);
    my $dk         = '';

    for my $i (1 .. $num_blocks) {
        # Seed = salt || INT_32_BE(i)
        # pack("N", $i) encodes $i as a 4-byte big-endian unsigned integer.
        my $seed = $salt . pack("N", $i);

        # U_1 = PRF(password, seed)
        my $u = $prf->($password, $seed);

        # t accumulates the XOR of all U values.
        my $t = $u;

        # U_j = PRF(password, U_{j-1}), XOR into t.
        for my $j (2 .. $iterations) {
            $u = $prf->($password, $u);
            $t = _xor_str($t, $u);
        }

        $dk .= $t;
    }

    return substr($dk, 0, $key_length);
}

# ──────────────────────────────────────────────────────────────────────────────
# Public API — concrete PRF variants
# ──────────────────────────────────────────────────────────────────────────────

# PBKDF2 with HMAC-SHA1 as the PRF.
# hLen = 20 bytes. Used in WPA2 (4096 iterations).
# For new systems prefer pbkdf2_hmac_sha256.
#
# RFC 6070 test vector:
#   pbkdf2_hmac_sha1_hex("password", "salt", 1, 20)
#   => "0c60c80f961f0e71f3a9b524af6012062fe037a6"
#
# $allow_empty_password: optional (default 0). Pass 1 to permit empty passwords.
# When set, the PRF uses the low-level hmac() to bypass the empty-key guard in
# hmac_sha1(), because an empty password becomes an empty HMAC key. The caller
# (e.g. scrypt) is responsible for ensuring brute-force resistance at a higher
# level.
sub pbkdf2_hmac_sha1 {
    my ($password, $salt, $iterations, $key_length, $allow_empty_password) = @_;
    my $prf;
    if ($allow_empty_password) {
        # Bypass the hmac_sha1() empty-key guard by calling the generic hmac()
        # directly with the SHA-1 hash function (block size 64 bytes).
        $prf = sub {
            my ($key, $msg) = @_;
            return _arrayref_to_str(hmac(sub { CodingAdventures::Sha1::digest($_[0]) }, 64, $key, $msg));
        };
    } else {
        $prf = sub {
            my ($key, $msg) = @_;
            return _arrayref_to_str(hmac_sha1($key, $msg));
        };
    }
    return _pbkdf2($prf, 20, $password, $salt, $iterations, $key_length, $allow_empty_password);
}

# PBKDF2 with HMAC-SHA256 as the PRF.
# hLen = 32 bytes. Recommended for new systems (OWASP 2023: >= 600,000 iterations).
#
# $allow_empty_password: optional (default 0). Pass 1 to permit empty passwords.
# This is used by the scrypt package, which calls PBKDF2 with 1 iteration purely
# for format conversion; the memory-hard ROMix layer provides brute-force resistance.
# When set, the PRF uses the low-level hmac() to bypass the empty-key guard in
# hmac_sha256().
sub pbkdf2_hmac_sha256 {
    my ($password, $salt, $iterations, $key_length, $allow_empty_password) = @_;
    my $prf;
    if ($allow_empty_password) {
        # Bypass the hmac_sha256() empty-key guard by calling the generic hmac()
        # directly with the SHA-256 hash function (block size 64 bytes).
        $prf = sub {
            my ($key, $msg) = @_;
            return _arrayref_to_str(hmac(sub { CodingAdventures::SHA256::sha256($_[0]) }, 64, $key, $msg));
        };
    } else {
        $prf = sub {
            my ($key, $msg) = @_;
            return _arrayref_to_str(hmac_sha256($key, $msg));
        };
    }
    return _pbkdf2($prf, 32, $password, $salt, $iterations, $key_length, $allow_empty_password);
}

# PBKDF2 with HMAC-SHA512 as the PRF.
# hLen = 64 bytes. Suitable for high-security applications.
#
# $allow_empty_password: optional (default 0). Pass 1 to permit empty passwords.
# When set, the PRF uses the low-level hmac() to bypass the empty-key guard in
# hmac_sha512().
sub pbkdf2_hmac_sha512 {
    my ($password, $salt, $iterations, $key_length, $allow_empty_password) = @_;
    my $prf;
    if ($allow_empty_password) {
        # Bypass the hmac_sha512() empty-key guard by calling the generic hmac()
        # directly with the SHA-512 hash function (block size 128 bytes).
        $prf = sub {
            my ($key, $msg) = @_;
            return _arrayref_to_str(hmac(sub { CodingAdventures::Sha512::digest($_[0]) }, 128, $key, $msg));
        };
    } else {
        $prf = sub {
            my ($key, $msg) = @_;
            return _arrayref_to_str(hmac_sha512($key, $msg));
        };
    }
    return _pbkdf2($prf, 64, $password, $salt, $iterations, $key_length, $allow_empty_password);
}

# ──────────────────────────────────────────────────────────────────────────────
# Hex variants
# ──────────────────────────────────────────────────────────────────────────────

sub pbkdf2_hmac_sha1_hex   { _to_hex(pbkdf2_hmac_sha1(@_))   }
sub pbkdf2_hmac_sha256_hex { _to_hex(pbkdf2_hmac_sha256(@_)) }
sub pbkdf2_hmac_sha512_hex { _to_hex(pbkdf2_hmac_sha512(@_)) }

1;
