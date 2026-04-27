use strict;
use warnings;
use Test2::V0;

# ============================================================================
# 03-blob-store-abstract.t — BlobStore abstract method enforcement
# ============================================================================
#
# These tests verify that calling any of the four abstract methods on the
# BlobStore base class directly (without a concrete subclass) results in a
# clear die() with an "abstract method" message.
#
# This is the Perl equivalent of testing that a Rust trait's required methods
# are not callable on the trait object itself. In Perl, the base class provides
# default implementations that die with a helpful message, ensuring subclasses
# that forget to implement a method fail loudly.

ok( eval { require CodingAdventures::ContentAddressableStorage::BlobStore; 1 }, 'BlobStore loads' );

my $store = CodingAdventures::ContentAddressableStorage::BlobStore->new();

# ---------------------------------------------------------------------------
# put() on base class dies with "abstract method"
# ---------------------------------------------------------------------------
subtest 'put() on base class dies' => sub {
    ok(
        dies { $store->put('a3f4b2c1d0', 'data') },
        'put() on BlobStore base class dies'
    );
    my $err;
    eval { $store->put('a3f4b2c1d0', 'data') };
    $err = $@;
    like( $err, qr/abstract method/i, 'error mentions "abstract method"' );
};

# ---------------------------------------------------------------------------
# get() on base class dies with "abstract method"
# ---------------------------------------------------------------------------
subtest 'get() on base class dies' => sub {
    ok(
        dies { $store->get('a3f4b2c1d0') },
        'get() on BlobStore base class dies'
    );
    my $err;
    eval { $store->get('a3f4b2c1d0') };
    $err = $@;
    like( $err, qr/abstract method/i, 'error mentions "abstract method"' );
};

# ---------------------------------------------------------------------------
# exists() on base class dies with "abstract method"
# ---------------------------------------------------------------------------
subtest 'exists() on base class dies' => sub {
    ok(
        dies { $store->exists('a3f4b2c1d0') },
        'exists() on BlobStore base class dies'
    );
    my $err;
    eval { $store->exists('a3f4b2c1d0') };
    $err = $@;
    like( $err, qr/abstract method/i, 'error mentions "abstract method"' );
};

# ---------------------------------------------------------------------------
# keys_with_prefix() on base class dies with "abstract method"
# ---------------------------------------------------------------------------
subtest 'keys_with_prefix() on base class dies' => sub {
    ok(
        dies { $store->keys_with_prefix("\xa3") },
        'keys_with_prefix() on BlobStore base class dies'
    );
    my $err;
    eval { $store->keys_with_prefix("\xa3") };
    $err = $@;
    like( $err, qr/abstract method/i, 'error mentions "abstract method"' );
};

# ---------------------------------------------------------------------------
# A properly implemented subclass does NOT die
#
# To confirm the base class is not broken — only the abstract guards — we
# create a minimal in-memory backend and verify it works end-to-end.
# ---------------------------------------------------------------------------
subtest 'concrete subclass works without die' => sub {
    # Inline a tiny in-memory BlobStore for this test only.
    package MemStore;
    our @ISA = ('CodingAdventures::ContentAddressableStorage::BlobStore');

    # Use a package variable rather than a lexical so that named subs
    # (put, get, etc.) can reference it without triggering Perl's
    # "Variable '%storage' is not available" closure warning.
    our %storage;

    sub new { bless {}, shift }
    sub put    { my ($s, $k, $d) = @_; $storage{$k} = $d }
    sub get    { my ($s, $k) = @_;     die "not found\n" unless exists $storage{$k}; $storage{$k} }
    sub exists { my ($s, $k) = @_;     exists $storage{$k} ? 1 : 0 }
    sub keys_with_prefix {
        my ($s, $pfx) = @_;
        my $pfx_hex = join('', map { sprintf('%02x', ord($_)) } split(//, $pfx));
        [ grep { substr($_, 0, length($pfx_hex)) eq $pfx_hex } keys %storage ]
    }

    package main;

    my $mem = MemStore->new;

    ok( lives { $mem->put('abcd' x 10, 'test data') }, 'concrete put() does not die' );
    ok( lives { $mem->get('abcd' x 10) },               'concrete get() does not die' );
    ok( lives { $mem->exists('abcd' x 10) },             'concrete exists() does not die' );
    ok( lives { $mem->keys_with_prefix("\xab") },        'concrete keys_with_prefix() does not die' );
};

done_testing;
