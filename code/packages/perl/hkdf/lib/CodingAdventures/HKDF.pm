package CodingAdventures::HKDF;
# HKDF — HMAC-based Extract-and-Expand Key Derivation Function
# RFC 5869, implemented from scratch in Perl.
#
# What Is HKDF?
# ==============
# HKDF is a simple, well-analyzed key derivation function built on top of
# HMAC. It was designed by Hugo Krawczyk and published as RFC 5869 in 2010.
#
# HKDF is used in:
#   - TLS 1.3 (the primary key derivation mechanism)
#   - Signal Protocol (Double Ratchet key derivation)
#   - WireGuard VPN (handshake key expansion)
#   - Noise Protocol Framework
#   - IKEv2 (Internet Key Exchange)
#
# Why Do We Need a KDF?
# =====================
# Raw cryptographic keys often come from sources with uneven entropy:
#   - Diffie-Hellman shared secrets have algebraic structure (not uniform)
#   - Passwords have low entropy concentrated in certain bits
#   - Hardware RNGs may have bias in certain bit positions
#
# A KDF "extracts" the entropy from such sources into a uniformly random
# pseudorandom key (PRK), then "expands" that PRK into as many output
# bytes as needed — each cryptographically independent.
#
# The Two-Stage Design
# =====================
#
# Stage 1 — EXTRACT:
#   PRK = HMAC-Hash(salt, IKM)
#
#   The salt is used as the HMAC key and IKM as the message. This is
#   intentional — the salt acts as a randomness extractor. If no salt is
#   provided, HashLen zero bytes are used (per RFC 5869 Section 2.2).
#
# Stage 2 — EXPAND:
#   T(0) = ""  (empty string)
#   T(1) = HMAC-Hash(PRK, T(0) || info || 0x01)
#   T(2) = HMAC-Hash(PRK, T(1) || info || 0x02)
#   ...
#   T(N) = HMAC-Hash(PRK, T(N-1) || info || 0x0N)
#   OKM = first L bytes of T(1) || T(2) || ... || T(N)
#
#   The counter byte is a single octet (1 through 255), limiting the
#   maximum output to 255 * HashLen bytes.
#
# CRITICAL PERL NOTE — HMAC Return Type
# =======================================
# The CodingAdventures::HMAC functions (hmac_sha256, hmac_sha512) return
# an ARRAYREF of integers (0-255), NOT a binary string. We must convert
# with pack('C*', @{$arrayref}) before using the result as a byte string
# for concatenation in the expand loop.
#
# Hash Function Support
# ======================
#   SHA-256: HashLen = 32 bytes
#   SHA-512: HashLen = 64 bytes

use strict;
use warnings;
use bytes;        # Force byte semantics — critical for correct length/substr
                  # when handling binary key material with non-ASCII bytes.
use Exporter 'import';

use CodingAdventures::HMAC qw(hmac_sha256 hmac_sha512);

our $VERSION = '0.1.0';

our @EXPORT_OK = qw(
    hkdf_extract
    hkdf_expand
    hkdf
    hkdf_extract_hex
    hkdf_expand_hex
    hkdf_hex
);

# ---------------------------------------------------------------------------
# Hash configuration
# ---------------------------------------------------------------------------
# Each hash algorithm needs:
#   hmac_fn  — reference to the HMAC function (returns arrayref of bytes!)
#   hash_len — output digest length in bytes

my %HASH_CONFIG = (
    sha256 => {
        hmac_fn  => \&hmac_sha256,
        hash_len => 32,
    },
    sha512 => {
        hmac_fn  => \&hmac_sha512,
        hash_len => 64,
    },
);

# Look up hash configuration, dying for unsupported algorithms.
sub _get_config {
    my ($hash_name) = @_;
    my $config = $HASH_CONFIG{$hash_name};
    die "unsupported hash algorithm: $hash_name\n" unless $config;
    return $config;
}

# ---------------------------------------------------------------------------
# hkdf_extract($salt, $ikm, $hash) -> $prk (binary string)
#
# HKDF-Extract (RFC 5869 Section 2.2)
#
# Condenses input keying material into a fixed-length pseudorandom key.
#
#   PRK = HMAC-Hash(salt, IKM)
#
# The salt is used as the HMAC key. If salt is empty or undef, we use
# HashLen zero bytes as specified by RFC 5869.
#
# IMPORTANT: hmac_sha256/hmac_sha512 return an arrayref of byte integers.
# We pack it into a binary string before returning.
#
# Parameters:
#   $salt — binary string (or empty/undef for default zero-byte salt)
#   $ikm  — input keying material (binary string)
#   $hash — "sha256" or "sha512" (default: "sha256")
#
# Returns: PRK as a binary string (HashLen bytes)
# ---------------------------------------------------------------------------
sub hkdf_extract {
    my ($salt, $ikm, $hash) = @_;
    $hash //= 'sha256';
    my $config = _get_config($hash);

    # RFC 5869 Section 2.2: "if not provided, [salt] is set to a string
    # of HashLen zeros."
    my $effective_salt = $salt;
    if (!defined($effective_salt) || length($effective_salt) == 0) {
        $effective_salt = "\x00" x $config->{hash_len};
    }

    # Call HMAC — returns arrayref of bytes. Pack to binary string.
    my $prk_bytes = $config->{hmac_fn}->($effective_salt, $ikm);
    return pack('C*', @{$prk_bytes});
}

# ---------------------------------------------------------------------------
# hkdf_expand($prk, $info, $length, $hash) -> $okm (binary string)
#
# HKDF-Expand (RFC 5869 Section 2.3)
#
# Generates arbitrary-length output from a fixed-length PRK.
#
#   T(0) = ""
#   T(i) = HMAC-Hash(PRK, T(i-1) || info || byte(i))
#   OKM  = first L bytes of T(1) || T(2) || ... || T(N)
#
# Each T(i) chains the previous output with info and a 1-byte counter.
# The counter is 1-indexed and limited to a single octet (max 255).
#
# IMPORTANT: HMAC returns arrayref of bytes — must pack before concat.
#
# Parameters:
#   $prk    — pseudorandom key from extract (binary string)
#   $info   — context/application info (binary string, can be empty)
#   $length — desired output length in bytes (1..255*HashLen)
#   $hash   — "sha256" or "sha512" (default: "sha256")
#
# Returns: OKM as a binary string (exactly $length bytes)
# ---------------------------------------------------------------------------
sub hkdf_expand {
    my ($prk, $info, $length, $hash) = @_;
    $hash //= 'sha256';
    $info //= '';
    my $config = _get_config($hash);

    # Validate output length.
    my $max_length = 255 * $config->{hash_len};
    die "HKDF expand length must be > 0\n" if $length <= 0;
    die sprintf(
        "HKDF expand length %d exceeds maximum %d (255 * %d)\n",
        $length, $max_length, $config->{hash_len}
    ) if $length > $max_length;

    # Number of HMAC iterations: ceil(L / HashLen)
    my $n = int(($length + $config->{hash_len} - 1) / $config->{hash_len});

    # Iterative expansion.
    # T(0) is the empty string. Each T(i) = HMAC(PRK, T(i-1) || info || i).
    my $t_prev = '';       # T(0) = empty
    my $okm    = '';       # accumulated output

    for my $i (1 .. $n) {
        # Build the HMAC message: previous T block + info + counter byte
        my $message = $t_prev . $info . chr($i);

        # HMAC returns arrayref — pack to binary string
        my $t_bytes = $config->{hmac_fn}->($prk, $message);
        $t_prev = pack('C*', @{$t_bytes});
        $okm .= $t_prev;
    }

    # Truncate to exactly the requested length.
    return substr($okm, 0, $length);
}

# ---------------------------------------------------------------------------
# hkdf($salt, $ikm, $info, $length, $hash) -> $okm (binary string)
#
# Combined HKDF — extract then expand in one call.
#
#   OKM = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)
#
# Parameters:
#   $salt   — binary string (or empty/undef for default zero salt)
#   $ikm    — input keying material
#   $info   — context info for expansion
#   $length — desired output length
#   $hash   — "sha256" or "sha512" (default: "sha256")
#
# Returns: OKM as a binary string
# ---------------------------------------------------------------------------
sub hkdf {
    my ($salt, $ikm, $info, $length, $hash) = @_;
    $hash //= 'sha256';
    my $prk = hkdf_extract($salt, $ikm, $hash);
    return hkdf_expand($prk, $info, $length, $hash);
}

# ---------------------------------------------------------------------------
# Hex convenience functions
# ---------------------------------------------------------------------------

sub hkdf_extract_hex {
    return unpack('H*', hkdf_extract(@_));
}

sub hkdf_expand_hex {
    return unpack('H*', hkdf_expand(@_));
}

sub hkdf_hex {
    return unpack('H*', hkdf(@_));
}

1;
