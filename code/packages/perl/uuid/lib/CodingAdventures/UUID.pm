package CodingAdventures::UUID;

# ============================================================================
# CodingAdventures::UUID — UUID v1/v3/v4/v5/v7 generation and parsing
# ============================================================================
#
# A UUID (Universally Unique Identifier) is a 128-bit label used to identify
# information in computer systems without central coordination. UUIDs are
# defined by RFC 4122 / ITU-T X.667. Their canonical textual form is:
#
#     xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
#     8        4    4    4    12  hex digits
#
# Where M is the "version" nibble (1-7) and N encodes the "variant":
#   10xx = RFC 4122 variant (most common; N ∈ {8,9,a,b})
#
# ## Version Overview
#
#   v1 — Time-based: encodes current time + random node (RFC 4122 §4.2)
#   v3 — MD5 name-based: deterministic from (namespace UUID + name string)
#   v4 — Random: 122 bits of randomness (RFC 4122 §4.4)
#   v5 — SHA-1 name-based: like v3 but using SHA-1 (preferred over v3)
#   v7 — Unix epoch time-sortable: 48-bit ms timestamp + random bits
#
# ## Test Vectors (RFC 4122 Appendix B / verified against Python uuid stdlib)
#
# These MUST match exactly:
#   v3(NAMESPACE_DNS, "www.example.com") = "5df41881-3aed-3515-88a7-2f4a814cf09e"
#   v5(NAMESPACE_DNS, "www.example.com") = "2ed6657d-e927-568b-95e1-2665a8aea6a2"
#
# ## Well-Known Namespace UUIDs (RFC 4122 §4.3)
#
#   NAMESPACE_DNS  = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
#   NAMESPACE_URL  = "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
#
# ## Implementation Notes
#
# Randomness: We use Perl's built-in rand() function. It is not
# cryptographically secure but is adequate for general-purpose unique IDs.
#
# v1 node: We generate a random 48-bit node with the multicast bit set,
# per RFC 4122 §4.5 (explicitly permitted as an alternative to real MACs).
#
# v7 time: We use time() for Unix epoch seconds and combine with a random
# sub-second component to approximate millisecond precision.
#
# ## Usage
#
#   use CodingAdventures::UUID qw(
#       generate_v4 generate_v1 generate_v3 generate_v5 generate_v7
#       parse validate nil_uuid
#   );
#
#   my $u = generate_v4();
#   my $u = generate_v5($UUID::NAMESPACE_DNS, "www.example.com");
#   # → "2ed6657d-e927-568b-95e3-af9f787f5a91"
#
#   validate("550e8400-e29b-41d4-a716-446655440000");  # true
#   my $info = parse($u);   # { version => 4, variant => "rfc4122", bytes => [...] }
#
# ============================================================================

use strict;
use warnings;

use CodingAdventures::Md5;
use CodingAdventures::Sha1;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    generate_v1 generate_v3 generate_v4 generate_v5 generate_v7
    parse validate nil_uuid
);

# ============================================================================
# Well-known namespace UUIDs (RFC 4122 §4.3)
# ============================================================================

our $NAMESPACE_DNS  = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
our $NAMESPACE_URL  = "6ba7b811-9dad-11d1-80b4-00c04fd430c8";
our $NAMESPACE_OID  = "6ba7b812-9dad-11d1-80b4-00c04fd430c8";
our $NAMESPACE_X500 = "6ba7b814-9dad-11d1-80b4-00c04fd430c8";

# ============================================================================
# Internal helpers
# ============================================================================

# _random_bytes($n) → list of $n random integers in [0, 255]
#
# Attempts to read from /dev/urandom (a cryptographically secure source
# available on Linux, macOS, and *BSD). Falls back to rand() if
# /dev/urandom is not available (e.g., Windows).
#
# NOTE: When using UUID v4 as security tokens (session IDs, CSRF tokens,
# password-reset links), ensure /dev/urandom is available. rand() is a
# predictable PRNG and is NOT suitable for security-sensitive UUID generation.
sub _random_bytes {
    my ($n) = @_;
    if (open my $fh, '<:raw', '/dev/urandom') {
        my $buf;
        if (read($fh, $buf, $n) == $n) {
            close $fh;
            return map { ord(substr($buf, $_, 1)) } 0 .. $n - 1;
        }
        close $fh;
    }
    # Fallback: rand() (not cryptographically secure)
    return map { int(rand(256)) } 1 .. $n;
}

# _bytes_to_uuid_string(@bytes) → UUID string
#
# Takes exactly 16 integer bytes [0..255] and formats them as a standard
# UUID string: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
#
# Groups: bytes 0-3, 4-5, 6-7, 8-9, 10-15 (0-indexed)
sub _bytes_to_uuid_string {
    my @b = @_;  # 16 bytes
    return sprintf(
        "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
        $b[0],  $b[1],  $b[2],  $b[3],
        $b[4],  $b[5],
        $b[6],  $b[7],
        $b[8],  $b[9],
        $b[10], $b[11], $b[12], $b[13], $b[14], $b[15]
    );
}

# _set_version_and_variant(\@bytes, $version)
#
# Applies RFC 4122 version and variant bits to a 16-byte array (in-place).
#
# Version (nibble in byte 6, high bits):
#   byte[6] = (byte[6] & 0x0f) | ($version << 4)
#
# Variant (byte 8, high 2 bits must be 10):
#   byte[8] = (byte[8] & 0x3f) | 0x80
#
# This follows the RFC 4122 UUID field layout:
#   time_low(4) | time_mid(2) | time_hi_and_version(2) |
#   clock_seq_hi_and_reserved(1) | clock_seq_low(1) | node(6)
sub _set_version_and_variant {
    my ($bytes, $version) = @_;
    # byte index 6: time_hi_and_version — top nibble = version
    $bytes->[6] = ($bytes->[6] & 0x0f) | ($version << 4);
    # byte index 8: clock_seq_hi_and_reserved — top 2 bits = "10"
    $bytes->[8] = ($bytes->[8] & 0x3f) | 0x80;
}

# _parse_uuid_to_bytes($uuid_str) → @bytes or ()
#
# Converts "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" to a 16-element list of
# integers. Returns an empty list if the string is not a valid UUID.
sub _parse_uuid_to_bytes {
    my ($uuid_str) = @_;
    (my $hex = $uuid_str) =~ s/-//g;
    return () unless length($hex) == 32;
    return () if $hex =~ /[^0-9a-fA-F]/;
    return map { hex(substr($hex, $_ * 2, 2)) } 0 .. 15;
}

# ============================================================================
# nil_uuid
# ============================================================================

# nil_uuid() → "00000000-0000-0000-0000-000000000000"
#
# The nil UUID consists of all zero bits. RFC 4122 §4.1.7 defines it as a
# special-case UUID meaning "no UUID" or "uninitialized".
sub nil_uuid {
    return "00000000-0000-0000-0000-000000000000";
}

# ============================================================================
# validate
# ============================================================================

# validate($uuid_str) → 1 (true) or '' (false)
#
# Checks that $uuid_str is in canonical UUID format:
#   8 hex digits - 4 hex digits - 4 hex digits - 4 hex digits - 12 hex digits
# Case-insensitive. Does NOT check version or variant bits — use parse() for that.
sub validate {
    my ($uuid_str) = @_;
    return '' unless defined $uuid_str && !ref($uuid_str);
    return $uuid_str =~ /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/ ? 1 : '';
}

# ============================================================================
# parse
# ============================================================================

# parse($uuid_str) → \%info  or  (undef, $error_string)
#
# Parses a UUID string and returns a hashref with:
#
#   version  => integer (0-8, or 0 for nil UUID)
#   variant  => string ("rfc4122", "reserved_microsoft", "reserved_future", "ncs")
#   bytes    => arrayref of 16 integers [0..255]
#
# Returns (undef, "error message") in list context on failure,
# or croaks in scalar context.
sub parse {
    my ($uuid_str) = @_;
    unless (validate($uuid_str)) {
        return (undef, "invalid UUID format: " . (defined $uuid_str ? $uuid_str : 'undef'));
    }

    my @bytes = _parse_uuid_to_bytes($uuid_str);
    unless (@bytes == 16) {
        return (undef, "could not parse UUID bytes");
    }

    # Version: top nibble of byte 6 (time_hi_and_version)
    my $version = ($bytes[6] >> 4) & 0x0f;

    # Variant: top bits of byte 8 (clock_seq_hi_and_reserved)
    # 0xxxxxxx = NCS
    # 10xxxxxx = RFC 4122
    # 110xxxxx = Microsoft
    # 111xxxxx = Reserved
    my $b8      = $bytes[8];
    my $variant;
    if    (($b8 & 0x80) == 0x00) { $variant = "ncs";                 }
    elsif (($b8 & 0xc0) == 0x80) { $variant = "rfc4122";             }
    elsif (($b8 & 0xe0) == 0xc0) { $variant = "reserved_microsoft";  }
    else                          { $variant = "reserved_future";     }

    return {
        version => $version,
        variant => $variant,
        bytes   => \@bytes,
    };
}

# ============================================================================
# generate_v4
# ============================================================================

# generate_v4() → UUID string
#
# ## Algorithm
#
# RFC 4122 §4.4 specifies:
#   1. Set all 128 bits to pseudo-random values.
#   2. Set version: bits 15-12 of byte 6 = 0100 (4).
#   3. Set variant: bits 7-6 of byte 8 = 10 (RFC 4122).
#
# This leaves 122 bits of randomness (128 - 4 version bits - 2 variant bits).
#
# ## Uniqueness Probability
#
# The birthday-paradox collision probability for n v4 UUIDs is approximately:
#
#     P ≈ n² / (2 × 2^122) ≈ n² / 10.6 × 10^36
#
# You would need to generate ~10^18 UUIDs before the collision probability
# exceeds 1 in a billion.
sub generate_v4 {
    my @bytes = _random_bytes(16);
    _set_version_and_variant(\@bytes, 4);
    return _bytes_to_uuid_string(@bytes);
}

# ============================================================================
# generate_v1
# ============================================================================

# generate_v1() → UUID string
#
# ## Algorithm
#
# UUID v1 encodes the current Gregorian time + node identifier:
#
#   time:      60-bit count of 100ns intervals since 15 October 1582
#   clock_seq: 14-bit counter for sub-100ns resolution and clock resets
#   node:      48-bit MAC address (or random, per RFC 4122 §4.5)
#
# ## Timestamp Layout
#
# The 60-bit timestamp is spread across three UUID fields:
#   time_low   (32 bits): bits 0-31 of timestamp
#   time_mid   (16 bits): bits 32-47 of timestamp
#   time_hi    (12 bits): bits 48-59 of timestamp + version nibble
#
# ## Simplifications
#
# We use time() for seconds and rand() for sub-second resolution.
# Node is random with the multicast bit set (RFC 4122 §4.5 permits this).
sub generate_v1 {
    # UUID epoch offset: 100ns intervals from 15 Oct 1582 to Unix epoch
    # = 12219292800 seconds × 10^7 intervals/second = 122192928000000000
    # Perl uses floating point here; for large integers we need care.
    # We work in integer arithmetic.
    my $UUID_EPOCH_OFFSET = 122192928000000000;

    my $unix_sec   = time();
    my $sub_sec    = int(rand(10_000_000));  # random 100ns intervals within current second
    my $timestamp  = $UUID_EPOCH_OFFSET + $unix_sec * 10_000_000 + $sub_sec;

    # Extract timestamp fields (Perl integers are 64-bit on modern systems)
    # Mask to avoid sign issues with the >> operator on some platforms.
    my $time_low = $timestamp & 0xFFFFFFFF;
    my $time_mid = ($timestamp >> 32) & 0xFFFF;
    my $time_hi  = ($timestamp >> 48) & 0x0FFF;

    # Random 14-bit clock sequence
    my $clock_seq = int(rand(0x4000));  # 0..16383

    # Random 48-bit node with multicast bit set (locally administered)
    my @node = _random_bytes(6);
    $node[0] |= 0x01;  # set multicast bit

    # Assemble 16 bytes (big-endian time fields)
    my @bytes = (
        # time_low (4 bytes, big-endian)
        ($time_low >> 24) & 0xFF,
        ($time_low >> 16) & 0xFF,
        ($time_low >>  8) & 0xFF,
         $time_low        & 0xFF,
        # time_mid (2 bytes, big-endian)
        ($time_mid >>  8) & 0xFF,
         $time_mid        & 0xFF,
        # time_hi_and_version: version = 1 in top nibble
        0x10 | (($time_hi >> 8) & 0x0F),
         $time_hi          & 0xFF,
        # clock_seq_hi_and_reserved: variant bits 10xxxxxx
        0x80 | (($clock_seq >> 8) & 0x3F),
         $clock_seq         & 0xFF,
        # node (6 bytes)
        @node,
    );

    return _bytes_to_uuid_string(@bytes);
}

# ============================================================================
# Internal: name-based UUID (shared by v3 and v5)
# ============================================================================

# _name_based_uuid(\@hash_bytes, $version) → UUID string
#
# Common logic for v3 (MD5) and v5 (SHA-1):
#   1. Take the first 16 bytes of the hash output.
#   2. Apply version and variant bits.
#   3. Format as UUID string.
#
# SHA-1 produces 20 bytes; we discard the last 4 as specified by RFC 4122 §4.3.
sub _name_based_uuid {
    my ($hash_bytes, $version) = @_;
    my @bytes = @{$hash_bytes}[0..15];  # first 16 bytes only
    _set_version_and_variant(\@bytes, $version);
    return _bytes_to_uuid_string(@bytes);
}

# ============================================================================
# generate_v3
# ============================================================================

# generate_v3($namespace_uuid_str, $name) → UUID string
#                                          or (undef, $error)
#
# ## Algorithm (RFC 4122 §4.3)
#
#   1. Convert namespace UUID string to 16 bytes.
#   2. Concatenate: input = namespace_bytes . name_bytes (UTF-8 raw bytes)
#   3. Hash with MD5: 16-byte digest
#   4. Set version bits to 0011 (3) in byte 6
#   5. Set variant bits to 10xx in byte 8
#   6. Format as UUID string
#
# ## Determinism
#
# The same (namespace, name) pair always produces the same UUID.
# This is useful for stable identifiers derived from well-known names.
#
# ## Test Vector (RFC 4122 Appendix B)
#
#   generate_v3(NAMESPACE_DNS, "www.example.com")
#   → "5df41881-3aed-3515-88a7-2f4a814cf09e"
sub generate_v3 {
    my ($namespace_uuid_str, $name) = @_;

    my @ns_bytes = _parse_uuid_to_bytes($namespace_uuid_str);
    unless (@ns_bytes == 16) {
        return (undef, "generate_v3: invalid namespace UUID: $namespace_uuid_str");
    }

    # Build input: namespace bytes as a raw string + name string
    my $input = pack("C*", @ns_bytes) . $name;

    # Compute MD5 hash → arrayref of 16 bytes
    my $hash_bytes = CodingAdventures::Md5->digest($input);

    return _name_based_uuid($hash_bytes, 3);
}

# ============================================================================
# generate_v5
# ============================================================================

# generate_v5($namespace_uuid_str, $name) → UUID string
#                                          or (undef, $error)
#
# ## Algorithm (RFC 4122 §4.3)
#
# Identical to v3 but uses SHA-1 instead of MD5:
#   1. Convert namespace UUID string to 16 bytes.
#   2. Concatenate: input = namespace_bytes . name_bytes
#   3. Hash with SHA-1: 20-byte digest (use only first 16 bytes)
#   4. Set version bits to 0101 (5) in byte 6
#   5. Set variant bits to 10xx in byte 8
#   6. Format as UUID string
#
# ## v5 vs v3
#
# v5 is preferred for new applications because:
#   - SHA-1 provides a larger hash space than MD5
#   - MD5 has theoretical collision vulnerabilities
#
# ## Test Vector (verified against Python uuid.uuid5() reference implementation)
#
#   generate_v5(NAMESPACE_DNS, "www.example.com")
#   → "2ed6657d-e927-568b-95e1-2665a8aea6a2"
sub generate_v5 {
    my ($namespace_uuid_str, $name) = @_;

    my @ns_bytes = _parse_uuid_to_bytes($namespace_uuid_str);
    unless (@ns_bytes == 16) {
        return (undef, "generate_v5: invalid namespace UUID: $namespace_uuid_str");
    }

    # Build input: namespace bytes as raw string + name string
    my $input = pack("C*", @ns_bytes) . $name;

    # Compute SHA-1 hash → arrayref of 20 bytes; we use only the first 16
    my $hash_bytes = CodingAdventures::Sha1->digest($input);

    return _name_based_uuid($hash_bytes, 5);
}

# ============================================================================
# generate_v7
# ============================================================================

# generate_v7() → UUID string
#
# ## Algorithm (draft-ietf-uuidrev-rfc4122bis)
#
# UUID v7 is designed to be lexicographically sortable by creation time:
#
# Bit layout (128 bits total):
#   bits 127-80 (48 bits): unix_ts_ms — Unix timestamp in milliseconds
#   bits 79-76  ( 4 bits): ver — 0111 (version 7)
#   bits 75-64  (12 bits): rand_a — random
#   bits 63-62  ( 2 bits): var — 10 (RFC 4122 variant)
#   bits 61-0   (62 bits): rand_b — random
#
# Byte layout:
#   bytes  0-5: unix_ts_ms (big-endian)
#   bytes  6-7: ver(4) | rand_a(12)
#   bytes  8-15: var(2) | rand_b(62)
#
# ## Advantages over v1
#
# v1 fragments the timestamp across three fields that are NOT in sort order.
# v7 puts the full millisecond timestamp in the most-significant bytes, so
# UUIDs sort chronologically by both byte comparison and string comparison.
sub generate_v7 {
    # Unix timestamp in milliseconds (approximate; Perl's time() is seconds)
    my $seconds   = time();
    my $sub_ms    = int(rand(1000));  # random 0..999 for sub-second ms component
    my $unix_ms   = $seconds * 1000 + $sub_ms;

    # Clamp to 48-bit range
    $unix_ms &= 0xFFFFFFFFFFFF;

    # 10 random bytes for rand_a (12 bits) and rand_b (62 bits)
    my @rand = _random_bytes(10);

    # Assemble 16 bytes
    my @bytes = (
        # Bytes 0-5: unix_ts_ms (48 bits, big-endian)
        ($unix_ms >> 40) & 0xFF,
        ($unix_ms >> 32) & 0xFF,
        ($unix_ms >> 24) & 0xFF,
        ($unix_ms >> 16) & 0xFF,
        ($unix_ms >>  8) & 0xFF,
         $unix_ms        & 0xFF,
        # Bytes 6-7: version nibble (0111 = 7) | rand_a (12 bits)
        0x70 | ($rand[0] & 0x0F),
        $rand[1],
        # Bytes 8-15: variant bits (10xxxxxx) | rand_b (62 bits)
        0x80 | ($rand[2] & 0x3F),
        $rand[3],
        $rand[4], $rand[5], $rand[6], $rand[7], $rand[8], $rand[9],
    );

    return _bytes_to_uuid_string(@bytes);
}

1;

__END__

=head1 NAME

CodingAdventures::UUID - UUID v1/v3/v4/v5/v7 generation and parsing

=head1 SYNOPSIS

    use CodingAdventures::UUID qw(
        generate_v4 generate_v5 generate_v3
        generate_v1 generate_v7
        parse validate nil_uuid
    );

    my $u = generate_v4();
    my $u = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
    # → "2ed6657d-e927-568b-95e3-af9f787f5a91"

    validate("550e8400-e29b-41d4-a716-446655440000");  # true
    my $info = parse($u);  # { version => 5, variant => "rfc4122", bytes => [...] }
    nil_uuid();  # "00000000-0000-0000-0000-000000000000"

=head1 DESCRIPTION

Pure Perl UUID library implementing RFC 4122 versions 1, 3, 4, 5 and the
draft UUIDv7 specification. Depends on CodingAdventures::Md5 (for v3) and
CodingAdventures::Sha1 (for v5).

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
