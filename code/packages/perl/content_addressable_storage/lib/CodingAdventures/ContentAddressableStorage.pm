package CodingAdventures::ContentAddressableStorage;

# ============================================================================
# CodingAdventures::ContentAddressableStorage — Content-Addressable Storage
# ============================================================================
#
# Content-addressable storage (CAS) maps the *hash of content* to the content
# itself. The hash is simultaneously the address and an integrity check: if the
# bytes returned by the store don't hash to the key you requested, the data is
# corrupt. No separate checksum file or trust anchor is needed.
#
# Mental model
# ────────────
#
#   Traditional storage:   name  ──►  content   (name can lie; content can change)
#   Content-addressed:     hash  ──►  content   (hash is derived from content, cannot lie)
#
# Imagine a library where every book's call number *is* a fingerprint of the
# book's text. You can't file a different book under that number — the number
# would immediately be wrong. And if someone swaps pages, the fingerprint
# changes and the librarian knows before you even open the cover.
#
# How Git uses CAS
# ────────────────
# Git's entire history is built on this principle. Every blob (file snapshot),
# tree (directory listing), commit, and tag is stored by the SHA-1 hash of its
# serialized bytes. Two identical files share one object. Renaming a file creates
# zero new storage. History is an immutable DAG of hashes pointing to hashes.
#
# Architecture
# ────────────
#
#   ┌──────────────────────────────────────────────────┐
#   │  CodingAdventures::ContentAddressableStorage                            │
#   │  · put($data)              → $key_hex            │
#   │  · get($key_hex)           → $data               │
#   │  · exists($key_hex)        → bool                │
#   │  · find_by_prefix($hex)    → $key_hex            │
#   │  · inner()                 → BlobStore ref       │
#   └─────────────────┬────────────────────────────────┘
#                     │ uses
#            ┌────────┴──────────────────────────────┐
#            │                                       │
#   LocalDiskStore                       (custom backends)
#   root/XX/XXXXXX…
#
# Key representation
# ──────────────────
# Keys are 40-character lowercase hex strings throughout this Perl port, e.g.:
#
#   "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
#
# In the Rust implementation they are [u8; 20] (20 raw bytes). Hex strings are
# used here for readability and because they can be passed directly to the
# filesystem-backed LocalDiskStore without additional encoding.

use strict;
use warnings;
use utf8;

use CodingAdventures::Sha1;
use CodingAdventures::ContentAddressableStorage::BlobStore;
use CodingAdventures::ContentAddressableStorage::Error;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new($store) — wrap a BlobStore backend in a CAS.
#
# $store must be an instance of a class that extends BlobStore (or at minimum
# implements the four required methods: put, get, exists, keys_with_prefix).
#
# Example:
#
#   use CodingAdventures::ContentAddressableStorage;
#   use CodingAdventures::ContentAddressableStorage::LocalDiskStore;
#
#   my $store = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new('/tmp/my-cas');
#   my $cas   = CodingAdventures::ContentAddressableStorage->new($store);
#
#   my $key  = $cas->put("hello, world");
#   my $data = $cas->get($key);
# ---------------------------------------------------------------------------
sub new {
    my ($class, $store) = @_;
    die "CodingAdventures::ContentAddressableStorage->new() requires a BlobStore argument\n"
        unless defined $store;
    return bless { store => $store }, $class;
}

# ---------------------------------------------------------------------------
# inner() — access the underlying BlobStore
#
# Useful when you need backend-specific operations not exposed through the CAS
# interface, for example listing all keys for garbage collection or querying
# storage statistics.
# ---------------------------------------------------------------------------
sub inner { $_[0]->{store} }

# ---------------------------------------------------------------------------
# put($data) — hash content, store it, return the 40-char hex key
#
# The CAS computes the SHA-1 digest of $data and delegates storage to the
# underlying BlobStore. Because keys are derived deterministically from
# content, put() is idempotent: storing the same bytes twice returns the same
# key and does not create a second copy.
#
# Deduplication follows automatically: two callers who store identical bytes
# get back the same key, and the backend stores only one copy.
#
# Returns: a 40-character lowercase hex string (the SHA-1 hash of $data).
# Dies:    if the backend reports a storage error.
# ---------------------------------------------------------------------------
sub put {
    my ($self, $data) = @_;
    $data //= '';

    # Compute SHA-1: CodingAdventures::Sha1::digest() returns an arrayref of
    # 20 byte values. We convert to a 40-char hex string for use as the key.
    my $bytes   = CodingAdventures::Sha1::digest($data);
    my $key_hex = join('', map { sprintf('%02x', $_) } @{$bytes});

    # Delegate to the backend. BlobStore::put is required to be idempotent,
    # so we don't need a pre-check with exists() — that would add an extra
    # filesystem round-trip and introduce a TOCTOU race.
    eval { $self->{store}->put($key_hex, $data) };
    if ($@) {
        die $@;
    }

    return $key_hex;
}

# ---------------------------------------------------------------------------
# get($key_hex) — fetch content and verify its integrity
#
# Retrieves the bytes stored under $key_hex, then re-hashes them to confirm
# the store returned what it promised. If the hash of the returned bytes
# does not equal $key_hex, the store is corrupt: we die with a
# CasCorruptedError instead of silently returning wrong data.
#
# This integrity check is the core guarantee of CAS. Even if the filesystem
# has silently corrupted the file (bit rot, partial write, malicious edit),
# the mismatch is detected immediately on read.
#
# Returns: the raw byte string stored under $key_hex.
# Dies:    CasNotFoundError   if the key is not in the store.
#          CasCorruptedError  if stored bytes don't hash to $key_hex.
#          plain string error if the backend reports an I/O failure.
# ---------------------------------------------------------------------------
sub get {
    my ($self, $key_hex) = @_;

    my $data;
    eval { $data = $self->{store}->get($key_hex) };
    if ($@) {
        # Re-throw as CasNotFoundError. The backend (LocalDiskStore) dies with
        # a plain string when the file does not exist.
        die CodingAdventures::ContentAddressableStorage::Error::CasNotFoundError->new($key_hex);
    }

    # Integrity check: re-hash the returned bytes.
    my $actual_bytes = CodingAdventures::Sha1::digest($data);
    my $actual_hex   = join('', map { sprintf('%02x', $_) } @{$actual_bytes});

    unless ($actual_hex eq $key_hex) {
        die CodingAdventures::ContentAddressableStorage::Error::CasCorruptedError->new($key_hex);
    }

    return $data;
}

# ---------------------------------------------------------------------------
# exists($key_hex) — check presence without fetching the blob
#
# Returns 1 if an object with this key is stored, 0 if not.
# Dies on backend error.
# ---------------------------------------------------------------------------
sub exists {
    my ($self, $key_hex) = @_;
    return $self->{store}->exists($key_hex);
}

# ---------------------------------------------------------------------------
# find_by_prefix($hex_prefix) — resolve an abbreviated hex string to a full key
#
# This is the "git show a3f4" feature: the user supplies a short hex prefix and
# we find the unique stored key that starts with those bytes.
#
# $hex_prefix rules:
#   - Must be non-empty and contain only [0-9a-fA-F].
#     Violation → CasInvalidPrefixError.
#   - Odd-length strings are treated as nibble-aligned: "a3f" means the prefix
#     bytes [0xa3, 0xf0] — the trailing nibble is the high nibble of a byte.
#     So "a3f" matches any key starting with 0xa3, 0xf_.
#   - Can be 1–40 characters.
#
# The prefix is decoded to a raw byte string and passed to the backend's
# keys_with_prefix() method. The CAS then enforces uniqueness:
#   - 0 matches → CasPrefixNotFoundError
#   - 1 match   → return that key
#   - 2+ matches → CasAmbiguousPrefixError
#
# Returns: the unique 40-char hex key matching the prefix.
# Dies:    CasInvalidPrefixError, CasPrefixNotFoundError, CasAmbiguousPrefixError
# ---------------------------------------------------------------------------
sub find_by_prefix {
    my ($self, $hex_prefix) = @_;
    $hex_prefix //= '';

    # Validate: non-empty and all hex characters.
    if (length($hex_prefix) == 0 || $hex_prefix =~ /[^0-9a-fA-F]/) {
        die CodingAdventures::ContentAddressableStorage::Error::CasInvalidPrefixError->new($hex_prefix);
    }

    # Odd-length hex prefix handling — the core of correct nibble matching.
    #
    # A 7-char prefix like "1bafb97" means: match any key whose hex starts
    # with the nibbles 1, b, a, f, b, 9, 7.  That is:
    #   - first 3 bytes exactly [0x1b, 0xaf, 0xb9]
    #   - high nibble of the 4th byte == 7  (i.e., 4th byte is 0x70..0x7f)
    #
    # The naive approach of padding "1bafb97" → "1bafb970" and then passing
    # 4 bytes to keys_with_prefix() would only match keys starting with exactly
    # [0x1b, 0xaf, 0xb9, 0x70] — too strict.  A key "1bafb97a..." would not match.
    #
    # Correct approach:
    #   1. Pass only the COMPLETE bytes (floor(len/2)) to keys_with_prefix.
    #   2. Filter the returned candidates by the trailing nibble.
    my $is_odd = (length($hex_prefix) % 2 == 1);
    my $trailing_nibble_val = 0;
    my $complete_hex = $hex_prefix;

    if ($is_odd) {
        $trailing_nibble_val = hex(substr($hex_prefix, -1));       # 0-15
        $complete_hex        = substr($hex_prefix, 0, length($hex_prefix) - 1);
    }

    # Pack the complete hex pairs into raw bytes.
    my $prefix_bytes = '';
    while ($complete_hex =~ /([0-9a-fA-F]{2})/g) {
        $prefix_bytes .= chr(hex($1));
    }

    # Ask the backend for all keys matching the complete-byte prefix.
    my $matches;
    if ($is_odd && length($prefix_bytes) == 0) {
        # 1-nibble prefix: scan all 16 possible first bytes (0xN0 through 0xNf).
        # For example, "a" should match keys in buckets a0/, a1/, …, af/.
        my @all;
        for my $lo (0 .. 15) {
            my $first_byte = ($trailing_nibble_val << 4) | $lo;
            my $m = $self->{store}->keys_with_prefix(chr($first_byte));
            push @all, @{$m};
        }
        $matches = \@all;
    } else {
        $matches = $self->{store}->keys_with_prefix($prefix_bytes);

        if ($is_odd) {
            # Filter: keep only keys where the (2*n)-th char of the 40-char
            # hex key equals the trailing nibble.
            # keys_with_prefix returns 40-char lowercase hex strings.
            my $n          = length($prefix_bytes);   # number of complete bytes
            my $nibble_pos = $n * 2;                  # 0-indexed char in hex key
            my $expected   = lc(sprintf('%x', $trailing_nibble_val));
            $matches = [ grep { substr($_, $nibble_pos, 1) eq $expected } @{$matches} ];
        }
    }

    my @sorted = sort @{$matches};

    my $count = scalar @sorted;

    if ($count == 0) {
        die CodingAdventures::ContentAddressableStorage::Error::CasPrefixNotFoundError->new($hex_prefix);
    } elsif ($count == 1) {
        return $sorted[0];
    } else {
        die CodingAdventures::ContentAddressableStorage::Error::CasAmbiguousPrefixError->new($hex_prefix);
    }
}

1;
