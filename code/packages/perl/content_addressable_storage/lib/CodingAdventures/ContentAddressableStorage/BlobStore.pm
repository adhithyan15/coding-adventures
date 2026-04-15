package CodingAdventures::ContentAddressableStorage::BlobStore;

# ============================================================================
# CodingAdventures::ContentAddressableStorage::BlobStore — abstract base class for CAS backends
# ============================================================================
#
# In Rust, BlobStore is a `trait` — a contract that any type implementing it
# must fulfil. Perl has no native trait system, but we simulate it with a base
# class that provides default implementations for all required methods that
# immediately die with "abstract method". Any subclass that forgets to override
# a required method will get a clear error at runtime rather than a cryptic
# "undefined subroutine" message.
#
# The four required methods, matching the Rust trait exactly:
#
#   put($key_hex, $data)          — persist $data under $key_hex
#   get($key_hex)                 — retrieve bytes stored under $key_hex
#   exists($key_hex)              — check presence without fetching data
#   keys_with_prefix($prefix_hex) — list all keys matching a byte prefix
#
# Keys are passed as 40-character lowercase hex strings throughout this Perl
# implementation. In the Rust implementation they are [u8; 20] (20 raw bytes).
# Hex strings are chosen here because they are:
#   1. Human-readable (useful for debugging and logging)
#   2. Directly usable as filesystem path components
#   3. Easy to compare, sort, and prefix-match with Perl string operations
#
# The tradeoff is a small encoding/decoding cost at the storage layer, which
# is negligible compared to the I/O overhead of reading and writing blobs.
#
# How to implement a new backend:
#
#   package MyBackend;
#   our @ISA = ('CodingAdventures::ContentAddressableStorage::BlobStore');
#
#   sub put    { my ($self, $key_hex, $data) = @_; ... }
#   sub get    { my ($self, $key_hex) = @_; ... }
#   sub exists { my ($self, $key_hex) = @_; ... }
#   sub keys_with_prefix { my ($self, $prefix_hex) = @_; ... }

use strict;
use warnings;
use utf8;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new() — base constructor
#
# Subclasses typically call SUPER::new or write their own constructor.
# This one exists so `BlobStore->new` doesn't crash; in practice you always
# instantiate a concrete subclass like LocalDiskStore.
# ---------------------------------------------------------------------------
sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

# ---------------------------------------------------------------------------
# put($key_hex, $data)
#
# Store $data (a raw byte string) under the 40-char hex key $key_hex.
#
# Contract:
#   - Idempotent: storing the same key twice is not an error. The CAS layer
#     guarantees that the same content always maps to the same key, so if a
#     key already exists the stored bytes are definitionally identical.
#   - Returns nothing on success.
#   - Dies on backend failure.
# ---------------------------------------------------------------------------
sub put {
    my ($self, $key_hex, $data) = @_;
    die ref($self) . "->put() is an abstract method — override in subclass\n";
}

# ---------------------------------------------------------------------------
# get($key_hex)
#
# Retrieve the byte string stored under $key_hex.
#
# Contract:
#   - Returns the raw bytes as a Perl scalar string.
#   - Dies (with a plain string error, not a blessed exception) if the key
#     does not exist or if a backend error occurs.
#   - Does NOT verify the hash — integrity checking is the CAS layer's job.
# ---------------------------------------------------------------------------
sub get {
    my ($self, $key_hex) = @_;
    die ref($self) . "->get() is an abstract method — override in subclass\n";
}

# ---------------------------------------------------------------------------
# exists($key_hex)
#
# Check whether a key is present without fetching the blob.
#
# Contract:
#   - Returns 1 if the key exists, 0 if not.
#   - Dies on backend failure.
# ---------------------------------------------------------------------------
sub exists {
    my ($self, $key_hex) = @_;
    die ref($self) . "->exists() is an abstract method — override in subclass\n";
}

# ---------------------------------------------------------------------------
# keys_with_prefix($prefix_bytes)
#
# Return an arrayref of 40-char hex key strings whose leading bytes match
# $prefix_bytes (a raw byte string, not hex).
#
# This is the engine behind abbreviated-hash lookup. The caller supplies a
# decoded byte prefix (e.g. "\xa3\xf4") and the store returns all full keys
# that start with those bytes. The CAS layer then checks for uniqueness.
#
# Contract:
#   - Returns an arrayref (possibly empty).
#   - The returned strings are 40-char lowercase hex.
#   - Dies on backend failure.
# ---------------------------------------------------------------------------
sub keys_with_prefix {
    my ($self, $prefix_bytes) = @_;
    die ref($self) . "->keys_with_prefix() is an abstract method — override in subclass\n";
}

1;
