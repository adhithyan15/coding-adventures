#!/usr/bin/env perl
# ============================================================================
# Tests for Ed25519 (RFC 8032)
# ============================================================================
# Test vectors verified against libsodium (PyNaCl) and the RFC 8032 appendix
# reference implementation.

use strict;
use warnings;
use Test2::V0;

use lib '../sha512/lib';
use CodingAdventures::Ed25519;

# ============================================================================
# RFC 8032 Test Vectors (verified against libsodium)
# ============================================================================

subtest 'Test 1: empty message' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my $expected_pub = "d75a980182b10ab7d54bfed3c964073a"
        . "0ee172f3daa62325af021a68f707511a";
    my $expected_sig = "e5564300c360ac729086e2cc806e828a"
        . "84877f1eb8e5d974d873e06522490155"
        . "5fb8821590a33bacc61e39701cf9b46b"
        . "d25bf5f0595bbe24655141438e7a100b";

    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    is(CodingAdventures::Ed25519::to_hex($pub), $expected_pub, "public key matches");

    my $sig = CodingAdventures::Ed25519::sign("", $sk);
    is(CodingAdventures::Ed25519::to_hex($sig), $expected_sig, "signature matches");

    ok(CodingAdventures::Ed25519::verify("", $sig, $pub), "signature verifies");
};

subtest 'Test 2: one byte (0x72)' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "4ccd089b28ff96da9db6c346ec114e0f"
        . "5b8a319f35aba624da8cf6ed4fb8a6fb"
    );
    my $expected_pub = "3d4017c3e843895a92b70aa74d1b7ebc"
        . "9c982ccf2ec4968cc0cd55f12af4660c";
    my $expected_sig = "92a009a9f0d4cab8720e820b5f642540"
        . "a2b27b5416503f8fb3762223ebdb69da"
        . "085ac1e43e15996e458f3613d0f11d8c"
        . "387b2eaeb4302aeeb00d291612bb0c00";
    my $message = CodingAdventures::Ed25519::from_hex("72");

    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    is(CodingAdventures::Ed25519::to_hex($pub), $expected_pub, "public key matches");

    my $sig = CodingAdventures::Ed25519::sign($message, $sk);
    is(CodingAdventures::Ed25519::to_hex($sig), $expected_sig, "signature matches");

    ok(CodingAdventures::Ed25519::verify($message, $sig, $pub), "signature verifies");
};

subtest 'Test 3: two bytes (0xaf82)' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "c5aa8df43f9f837bedb7442f31dcb7b1"
        . "66d38535076f094b85ce3a2e0b4458f7"
    );
    my $expected_pub = "fc51cd8e6218a1a38da47ed00230f058"
        . "0816ed13ba3303ac5deb911548908025";
    my $expected_sig = "6291d657deec24024827e69c3abe01a3"
        . "0ce548a284743a445e3680d7db5ac3ac"
        . "18ff9b538d16f290ae67f760984dc659"
        . "4a7c15e9716ed28dc027beceea1ec40a";
    my $message = CodingAdventures::Ed25519::from_hex("af82");

    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    is(CodingAdventures::Ed25519::to_hex($pub), $expected_pub, "public key matches");

    my $sig = CodingAdventures::Ed25519::sign($message, $sk);
    is(CodingAdventures::Ed25519::to_hex($sig), $expected_sig, "signature matches");

    ok(CodingAdventures::Ed25519::verify($message, $sig, $pub), "signature verifies");
};

# ============================================================================
# Verification Failure Tests
# ============================================================================

subtest 'rejects wrong message' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    my $sig = CodingAdventures::Ed25519::sign("hello", $sk);
    ok(!CodingAdventures::Ed25519::verify("world", $sig, $pub), "wrong message rejected");
};

subtest 'rejects wrong public key' => sub {
    my $seed1 = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my $seed2 = CodingAdventures::Ed25519::from_hex(
        "4ccd089b28ff96da9db6c346ec114e0f"
        . "5b8a319f35aba624da8cf6ed4fb8a6fb"
    );
    my (undef, $sk1) = CodingAdventures::Ed25519::generate_keypair($seed1);
    my ($pub2, undef) = CodingAdventures::Ed25519::generate_keypair($seed2);
    my $sig = CodingAdventures::Ed25519::sign("hello", $sk1);
    ok(!CodingAdventures::Ed25519::verify("hello", $sig, $pub2), "wrong key rejected");
};

subtest 'rejects tampered signature' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    my $sig = CodingAdventures::Ed25519::sign("hello", $sk);
    # Flip a bit in the first byte
    my $tampered = chr(ord(substr($sig, 0, 1)) ^ 1) . substr($sig, 1);
    ok(!CodingAdventures::Ed25519::verify("hello", $tampered, $pub), "tampered sig rejected");
};

subtest 'rejects invalid lengths' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    my $sig = CodingAdventures::Ed25519::sign("hello", $sk);
    ok(!CodingAdventures::Ed25519::verify("hello", "short", $pub), "short sig rejected");
    ok(!CodingAdventures::Ed25519::verify("hello", $sig, "short"), "short pubkey rejected");
};

# ============================================================================
# Key Generation Tests
# ============================================================================

subtest 'key generation properties' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);

    is(length($pub), 32, "public key is 32 bytes");
    is(length($sk), 64, "secret key is 64 bytes");
    is(substr($sk, 0, 32), $seed, "secret key starts with seed");
    is(substr($sk, 32, 32), $pub, "secret key ends with public key");
};

# ============================================================================
# Round-Trip Tests
# ============================================================================

subtest 'sign and verify round-trip' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my ($pub, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);

    # Empty message
    my $sig = CodingAdventures::Ed25519::sign("", $sk);
    ok(CodingAdventures::Ed25519::verify("", $sig, $pub), "empty message round-trip");

    # Short message
    $sig = CodingAdventures::Ed25519::sign("test", $sk);
    ok(CodingAdventures::Ed25519::verify("test", $sig, $pub), "short message round-trip");
};

subtest 'deterministic signatures' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my (undef, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    my $sig1 = CodingAdventures::Ed25519::sign("hello", $sk);
    my $sig2 = CodingAdventures::Ed25519::sign("hello", $sk);
    is($sig1, $sig2, "same message produces same signature");
};

subtest 'signature is 64 bytes' => sub {
    my $seed = CodingAdventures::Ed25519::from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        . "4449c5697b326919703bac031cae7f60"
    );
    my (undef, $sk) = CodingAdventures::Ed25519::generate_keypair($seed);
    my $sig = CodingAdventures::Ed25519::sign("hello", $sk);
    is(length($sig), 64, "signature is 64 bytes");
};

done_testing;
