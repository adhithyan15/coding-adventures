package CodingAdventures::Scrypt;

# coding_adventures::scrypt -- scrypt (memory-hard password hashing)
# RFC 7914: "The scrypt Password-Based Key Derivation Function"
#
# == What Is scrypt? ==
#
# scrypt was designed by Colin Percival in 2009 to be a password-based key
# derivation function that is simultaneously hard on CPU *and* memory. This
# "memory-hard" property defeats hardware attackers (ASICs, FPGAs) that can
# build millions of SHA cores cheaply but cannot cheaply provision gigabytes
# of fast RAM for each core.
#
# Real-world uses:
#   - Litecoin: scrypt used as a proof-of-work function
#   - OpenSSL 1.1+: scryptenc_buf
#   - AWS Secrets Manager: scrypt-based key derivation
#   - Libsodium: crypto_pwhash_scryptsalsa208sha256
#
# == Algorithm Overview (RFC 7914 § 5) ==
#
# scrypt(P, S, N, r, p, dkLen):
#
#   1. B = PBKDF2-HMAC-SHA256(P, S, 1, p*128*r)     -- initial keying material
#   2. For each of the p blocks B[0..p-1]:
#        B[i] = scryptROMix(B[i], N, r)             -- memory-hard mix
#   3. DK = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)     -- final stretch
#
# Parameters:
#   P     -- password (may be empty; RFC 7914 vector 1 uses "")
#   S     -- salt
#   N     -- CPU/memory cost. Must be a power of 2 (e.g. 16384 for login use)
#   r     -- block size factor. Typically 8.
#   p     -- parallelisation factor. Typically 1.
#   dkLen -- desired output length in bytes
#
# Memory requirement: approximately 128 * r * N bytes.
# For N=16384, r=8: about 16 MB per invocation.
#
# == scryptROMix (RFC 7914 § 4) ==
#
# ROMix fills a "scratchpad" V of N blocks with pseudo-random data produced
# by the block mix, then reads them back in a pseudo-random order determined
# by the data itself (Integerify). Because the read order is data-dependent,
# an attacker cannot avoid storing all N blocks simultaneously.
#
# == BlockMix (RFC 7914 § 3) ==
#
# BlockMix(B, r) mixes a sequence of 2r 64-byte blocks using Salsa20/8 as
# the mixing function. It interleaves even and odd indexed outputs so that
# later stages depend on all earlier ones.
#
# == Salsa20/8 ==
#
# Salsa20/8 is a reduced-round (8 rounds) variant of the Salsa20 stream
# cipher. Each "round" consists of four quarter-rounds applied to columns,
# then four quarter-rounds applied to rows (the "column" and "row" rounds).
# A quarter-round QR(a, b, c, d) is:
#
#   b ^= rotl(a + d, 7)
#   c ^= rotl(b + a, 9)
#   d ^= rotl(c + b, 13)
#   a ^= rotl(d + c, 18)
#
# After 8 rounds, each word of the initial state is added back (modular) to
# produce the output. This is the "add-rotate-XOR" (ARX) construction used
# in ChaCha20, Salsa20, and many other primitives.
#
# == Why inline PBKDF2? ==
#
# RFC 7914 vector 1 uses an empty password (""). The CodingAdventures::PBKDF2
# package rejects empty passwords as a security measure. scrypt itself is the
# memory-hard layer that protects against brute force; its internal PBKDF2
# calls (with 1 iteration each) are purely for format conversion and do not
# need the empty-password guard. We implement a minimal inline PBKDF2 here
# that calls hmac_sha256 directly.
#
# == Perl uint32 Arithmetic ==
#
# Perl integers are at least 32 bits, typically 64 bits on modern hardware.
# When emulating 32-bit unsigned arithmetic we must mask after every add or
# rotate: ($a + $b) & 0xFFFFFFFF, (($x << $n) | ($x >> (32-$n))) & 0xFFFFFFFF.
# We do NOT use "use integer;" because that switches to signed arithmetic and
# interacts poorly with masking.
#
# == Binary Strings ==
#
# "use bytes;" makes length() and substr() operate on raw bytes (octets)
# rather than Unicode characters. All internal strings in this module are
# raw byte strings; pack/unpack handle encoding/decoding.

use strict;
use warnings;
use bytes;            # length/substr operate on raw bytes
use Exporter 'import';
use POSIX qw(ceil);
use CodingAdventures::HMAC qw(hmac);
use CodingAdventures::SHA256;

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(scrypt scrypt_hex);

# ──────────────────────────────────────────────────────────────────────────────
# Salsa20/8 primitives
# ──────────────────────────────────────────────────────────────────────────────

# _rotl32: rotate a 32-bit unsigned integer $x left by $n bits.
#
# Visual diagram of rotl($x, 7) on a 32-bit word:
#
#   Before: [b31 b30 b29 ... b08 b07 | b06 b05 ... b01 b00]
#   After:  [b24 b23 b22 ... b01 b00 | b31 b30 ... b26 b25]
#            ^--- left shift by 7 ---^  ^--- right shift by 25 ---^
#
# Masking with 0xFFFFFFFF discards any bits above position 31 that Perl's
# 64-bit integers would otherwise retain.
sub _rotl32 {
    my ($x, $n) = @_;
    return (($x << $n) | ($x >> (32 - $n))) & 0xFFFFFFFF;
}

# _salsa20_8: apply 8 rounds of Salsa20 to a 64-byte block.
#
# The input $s is a 64-byte binary string containing 16 little-endian
# 32-bit words. We unpack them into @x (the working state) and keep a
# copy @z (the initial state). After 8 rounds we add @z back to @x
# word-by-word (mod 2^32) — this is the "add initial state" step that
# prevents the function from being trivially invertible.
#
# Round structure (8 rounds = 4 iterations of "column round + row round"):
#
#   Column round quarter-rounds operate on the column layout:
#     0  4  8 12
#     1  5  9 13
#     2  6 10 14
#     3  7 11 15
#
#   Row round quarter-rounds operate on the row layout:
#     0  1  2  3
#     5  6  7  4
#    10 11  8  9
#    15 12 13 14
#
# The quarter-round QR(a, b, c, d) follows Salsa20 specification:
#   b ^= rotl(a + d, 7)
#   c ^= rotl(b + a, 9)
#   d ^= rotl(c + b, 13)
#   a ^= rotl(d + c, 18)
#
# Note: scrypt uses Salsa20/8 (8 rounds), NOT Salsa20/20. The /8 variant
# trades security margin for speed, which is acceptable inside a
# memory-hard construction.
sub _salsa20_8 {
    my ($s) = @_;

    # Unpack the 64-byte block as 16 little-endian 32-bit words.
    # "V16" means: unpack 16 unsigned 32-bit integers, little-endian.
    my @x = unpack("V16", $s);
    my @z = @x;    # save initial state for final add

    # Quarter-round closure captures @x by closure reference.
    # Arguments are the indices (a, b, c, d) into @x.
    my $qr = sub {
        my ($a, $b, $c, $d) = @_;
        $x[$b] = ($x[$b] ^ _rotl32(($x[$a] + $x[$d]) & 0xFFFFFFFF,  7)) & 0xFFFFFFFF;
        $x[$c] = ($x[$c] ^ _rotl32(($x[$b] + $x[$a]) & 0xFFFFFFFF,  9)) & 0xFFFFFFFF;
        $x[$d] = ($x[$d] ^ _rotl32(($x[$c] + $x[$b]) & 0xFFFFFFFF, 13)) & 0xFFFFFFFF;
        $x[$a] = ($x[$a] ^ _rotl32(($x[$d] + $x[$c]) & 0xFFFFFFFF, 18)) & 0xFFFFFFFF;
    };

    # 4 double-rounds = 8 rounds total.
    for (1..4) {
        # Column rounds: each quarter-round targets one column.
        $qr->(0, 4,  8, 12);
        $qr->(5, 9, 13,  1);
        $qr->(10, 14,  2,  6);
        $qr->(15,  3,  7, 11);

        # Row rounds: each quarter-round targets one row.
        $qr->( 0,  1,  2,  3);
        $qr->( 5,  6,  7,  4);
        $qr->(10, 11,  8,  9);
        $qr->(15, 12, 13, 14);
    }

    # Add initial state back, mod 2^32, then pack back to bytes.
    my @result = map { ($x[$_] + $z[$_]) & 0xFFFFFFFF } 0..15;
    return pack("V16", @result);
}

# ──────────────────────────────────────────────────────────────────────────────
# XOR helpers
# ──────────────────────────────────────────────────────────────────────────────

# _xor64: XOR two 64-byte binary strings.
#
# Perl's built-in ^ operator, when applied to two strings of equal length,
# XORs them byte-by-byte and returns a new string of the same length.
# This is exactly what we need for the Salsa20/8 input preparation.
sub _xor64 {
    my ($a, $b) = @_;
    return $a ^ $b;
}

# _xor_blocks: XOR two arrays of 64-byte strings, element-wise.
#
# Both $a and $b are arrayrefs of equal length where each element is a
# 64-byte block. Returns an arrayref of XORed blocks.
sub _xor_blocks {
    my ($a, $b) = @_;
    return [ map { $a->[$_] ^ $b->[$_] } 0..$#$a ];
}

# ──────────────────────────────────────────────────────────────────────────────
# BlockMix (RFC 7914 § 3)
# ──────────────────────────────────────────────────────────────────────────────

# _block_mix: mix a sequence of 2r 64-byte blocks using Salsa20/8.
#
# Input:  $blocks — arrayref of 2r 64-byte binary strings
#         $r      — block size parameter
#
# Algorithm (RFC 7914 § 3):
#   X = B[2r-1]                           (carry block)
#   For i = 0..2r-1:
#     X = Salsa20/8(X XOR B[i])
#     Y[i] = X
#   Output = [Y[0], Y[2], Y[4], ..., Y[2r-2], Y[1], Y[3], ..., Y[2r-1]]
#
# The even/odd shuffle in the output is what gives BlockMix its avalanche
# property: the first half of the output depends on even-indexed inputs,
# the second half on odd-indexed inputs, but XOR with the carry X ensures
# every output block depends on all prior inputs.
sub _block_mix {
    my ($blocks, $r) = @_;
    my $two_r = 2 * $r;

    # Start with the last block as the carry.
    my $x = $blocks->[$two_r - 1];

    my @y;
    for my $i (0 .. $two_r - 1) {
        $x    = _salsa20_8(_xor64($x, $blocks->[$i]));
        $y[$i] = $x;
    }

    # Interleave: even-indexed blocks first, then odd-indexed blocks.
    # This is equivalent to the "even columns then odd columns" layout
    # that the Salsa20 quarter-round naturally produces.
    my @out;
    for my $i (0 .. $r - 1) { push @out, $y[2 * $i];     }
    for my $i (0 .. $r - 1) { push @out, $y[2 * $i + 1]; }

    return \@out;
}

# ──────────────────────────────────────────────────────────────────────────────
# Integerify (RFC 7914 § 4)
# ──────────────────────────────────────────────────────────────────────────────

# _integerify: extract an integer from the last block of the working set.
#
# RFC 7914 defines Integerify(X) as the first 8 bytes of the last 64-byte
# block, interpreted as a little-endian 64-bit integer. Because N <= 2^20 in
# our validated range, the low 32 bits are always sufficient to determine the
# index mod N. We unpack just the first 4 bytes as a little-endian uint32.
#
# This pseudo-random index is what makes ROMix a "read-only memory" function:
# which scratchpad slot to XOR in is determined by the current state, so an
# attacker cannot pre-compute the access pattern without materialising all N
# blocks.
sub _integerify {
    my ($x) = @_;
    my $last = $x->[-1];

    # "V" = unpack a single unsigned 32-bit little-endian integer from the
    # first 4 bytes of $last.
    my ($lo) = unpack("V", $last);
    return $lo;
}

# ──────────────────────────────────────────────────────────────────────────────
# ROMix (RFC 7914 § 4)
# ──────────────────────────────────────────────────────────────────────────────

# _ro_mix: the memory-hard core of scrypt.
#
# Input:  $b_bytes — a binary string of 128*r bytes
#         $n       — scratchpad size (must be a power of 2)
#         $r       — block size parameter
#
# Algorithm (RFC 7914 § 4):
#   X = B                              (working state: 2r blocks of 64 bytes)
#
#   Phase 1 — fill scratchpad:
#     For i = 0..N-1:
#       V[i] = X                       (record current state)
#       X    = BlockMix(X, r)          (mix to next state)
#
#   Phase 2 — pseudo-random reads:
#     For i = 0..N-1:
#       j = Integerify(X) mod N        (data-dependent index into V)
#       X = BlockMix(X XOR V[j], r)   (mix with a recorded state)
#
#   Output: pack X back to bytes
#
# The scratchpad V has N entries, each being 2r*64 bytes. Total memory:
# 128 * r * N bytes.
#
# Why N must be a power of 2: the mod N operation in phase 2 needs to produce
# a uniform distribution; any non-power-of-2 modulus would bias the selection.
sub _ro_mix {
    my ($b_bytes, $n, $r) = @_;
    my $two_r = 2 * $r;

    # Split the flat byte string into an array of 64-byte blocks.
    my @x = map { substr($b_bytes, $_ * 64, 64) } 0 .. $two_r - 1;

    # Phase 1: fill scratchpad.
    my @v;
    for my $i (0 .. $n - 1) {
        $v[$i] = [@x];                          # snapshot (copy of array)
        my $mixed = _block_mix(\@x, $r);
        @x = @$mixed;
    }

    # Phase 2: pseudo-random reads and mixes.
    for my $i (0 .. $n - 1) {
        my $j     = _integerify(\@x) % $n;
        my $xored = _xor_blocks(\@x, $v[$j]);
        my $mixed = _block_mix($xored, $r);
        @x = @$mixed;
    }

    # Reassemble 64-byte blocks back into a flat byte string.
    return join('', @x);
}

# ──────────────────────────────────────────────────────────────────────────────
# Inline PBKDF2-HMAC-SHA256 (no empty-password guard)
# ──────────────────────────────────────────────────────────────────────────────

# _pbkdf2_sha256_raw: minimal PBKDF2-HMAC-SHA256 for internal scrypt use.
#
# This implements RFC 8018 § 5.2 using CodingAdventures::HMAC::hmac_sha256 as
# the PRF. It intentionally omits the empty-password check that
# CodingAdventures::PBKDF2 enforces, because RFC 7914 vector 1 calls scrypt
# with password="" and the memory-hard ROMix layer provides the security.
#
# The function is always called with $iterations = 1 by scrypt itself, so the
# inner loop over j runs 0 times and the cost is exactly $num_blocks HMAC
# evaluations.
#
# The internal HMAC-SHA256 PRF uses the generic hmac() function from
# CodingAdventures::HMAC, which does NOT have the empty-key guard. We pass
# a coderef that calls CodingAdventures::SHA256::sha256 directly.
# Block size for SHA-256 is 64 bytes.
#
# hmac() returns an arrayref of byte integers [0..255].
# pack("C*", @$arrayref) converts that to a raw binary string.
#
# Perl's built-in ^ on two equal-length strings XORs them byte-by-byte.

my $_sha256_prf = sub {
    my ($key, $msg) = @_;
    return pack("C*", @{ hmac(sub { CodingAdventures::SHA256::sha256($_[0]) }, 64, $key, $msg) });
};

sub _pbkdf2_sha256_raw {
    my ($password, $salt, $iterations, $key_length) = @_;

    my $h_len      = 32;    # SHA-256 output is 32 bytes
    my $num_blocks = ceil($key_length / $h_len);
    my $dk         = '';

    for my $i (1 .. $num_blocks) {
        # Seed = salt || INT(i), where INT(i) is big-endian 4-byte block counter.
        # pack("N", $i) encodes $i as unsigned 32-bit big-endian.
        my $seed = $salt . pack("N", $i);

        # U_1 = HMAC-SHA256(password, seed)
        my $u = $_sha256_prf->($password, $seed);
        my $t = $u;

        # U_j = HMAC-SHA256(password, U_{j-1}), XOR into accumulator t.
        for my $j (2 .. $iterations) {
            $u = $_sha256_prf->($password, $u);
            $t = $t ^ $u;
        }

        $dk .= $t;
    }

    return substr($dk, 0, $key_length);
}

# ──────────────────────────────────────────────────────────────────────────────
# Public API
# ──────────────────────────────────────────────────────────────────────────────

# scrypt: derive a key from a password using the scrypt algorithm.
#
# Parameters:
#   $password — secret string (may be empty; RFC 7914 vector 1 uses "")
#   $salt     — random per-credential salt (may be empty in test vectors)
#   $n        — CPU/memory cost factor. Must be a power of 2, >= 2, <= 2^20.
#               Practical values: 2^14 (16384) for interactive login,
#               2^20 (1048576) for offline/bulk key derivation.
#   $r        — block size. Must be >= 1. Typically 8.
#               Larger r increases memory bandwidth requirements.
#   $p        — parallelism. Must be >= 1. Typically 1.
#               Larger p increases CPU time without increasing memory.
#   $dk_len   — desired key length in bytes (1..2^20).
#
# Returns: raw binary string of $dk_len bytes.
#
# Memory usage: approximately 128 * $r * $n bytes.
# CPU cost:     approximately 2 * $n * $r BlockMix calls per block, times $p.
#
# Security guidance:
#   Interactive logins: N=2^14, r=8, p=1 (16 MB, ~100 ms on modern hardware)
#   Bulk storage:       N=2^20, r=8, p=1 (1 GB, several seconds)
sub scrypt {
    my ($password, $salt, $n, $r, $p, $dk_len) = @_;

    # Validate parameters per RFC 7914 § 2.
    die "scrypt password must be defined\n" unless defined($password);
    die "scrypt salt must be defined\n"     unless defined($salt);
    die "scrypt N must be a power of 2 and >= 2\n"
        unless defined($n) && $n >= 2 && ($n & ($n - 1)) == 0;
    die "scrypt N must not exceed 2^20\n"
        unless $n <= 2**20;
    die "scrypt r must be a positive integer\n"
        unless defined($r) && $r >= 1;
    die "scrypt p must be a positive integer\n"
        unless defined($p) && $p >= 1;
    die "scrypt dk_len must be between 1 and 2^20\n"
        unless defined($dk_len) && $dk_len >= 1 && $dk_len <= 2**20;
    die "scrypt p * r exceeds limit\n"
        unless $p * $r <= 2**30;

    # Step 1: derive p*128*r bytes of initial keying material from the
    # password and user-supplied salt. These p blocks of 128*r bytes will
    # each be independently mixed by ROMix.
    my $b = _pbkdf2_sha256_raw($password, $salt, 1, $p * 128 * $r);

    # Step 2: apply ROMix to each of the p blocks independently.
    # ROMix is the memory-hard core; each call processes 128*r bytes and
    # requires N scratchpad slots of the same size.
    my @mixed_parts;
    for my $i (0 .. $p - 1) {
        my $chunk = substr($b, $i * 128 * $r, 128 * $r);
        push @mixed_parts, _ro_mix($chunk, $n, $r);
    }
    my $b_mixed = join('', @mixed_parts);

    # Step 3: extract the final key from the mixed blocks using PBKDF2.
    # The $b_mixed now plays the role of the salt; the password is fed in
    # again so that an attacker who only knows the mixed output (but not the
    # original password) still cannot recover the key.
    return _pbkdf2_sha256_raw($password, $b_mixed, 1, $dk_len);
}

# scrypt_hex: like scrypt, but returns a lowercase hex string.
#
# Useful for storing derived keys in text-based databases or config files.
# Each raw byte maps to exactly two hex digits.
sub scrypt_hex {
    return join('', map { sprintf('%02x', ord($_)) } split(//, scrypt(@_)));
}

1;
