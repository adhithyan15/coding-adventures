#!/usr/bin/env perl

# ============================================================================
# X25519 Test Suite
# ============================================================================
# Tests against all RFC 7748 test vectors, including the 1000-iteration test.
# ============================================================================

use strict;
use warnings;
use Test2::V0;
use CodingAdventures::X25519 qw(x25519 x25519_base generate_keypair);

# Helper: convert hex string to binary
sub hex_to_bytes {
    my ($hex) = @_;
    return pack("H*", $hex);
}

# Helper: convert binary to hex string
sub bytes_to_hex {
    my ($bytes) = @_;
    return unpack("H*", $bytes);
}

# ---------------------------------------------------------------------------
# RFC 7748 Test Vector 1
# ---------------------------------------------------------------------------
subtest 'RFC 7748 test vector 1' => sub {
    my $scalar = hex_to_bytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    my $u      = hex_to_bytes("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
    my $expected = "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552";

    my $result = x25519($scalar, $u);
    is(bytes_to_hex($result), $expected, "test vector 1 matches");
};

# ---------------------------------------------------------------------------
# RFC 7748 Test Vector 2
# ---------------------------------------------------------------------------
subtest 'RFC 7748 test vector 2' => sub {
    my $scalar = hex_to_bytes("4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d");
    my $u      = hex_to_bytes("e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493");
    my $expected = "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957";

    my $result = x25519($scalar, $u);
    is(bytes_to_hex($result), $expected, "test vector 2 matches");
};

# ---------------------------------------------------------------------------
# Base Point Multiplication — Alice
# ---------------------------------------------------------------------------
subtest 'base point multiplication (Alice)' => sub {
    my $alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    my $expected = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a";

    my $result = x25519_base($alice_private);
    is(bytes_to_hex($result), $expected, "Alice's public key is correct");
};

# ---------------------------------------------------------------------------
# Base Point Multiplication — Bob
# ---------------------------------------------------------------------------
subtest 'base point multiplication (Bob)' => sub {
    my $bob_private = hex_to_bytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    my $expected = "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f";

    my $result = x25519_base($bob_private);
    is(bytes_to_hex($result), $expected, "Bob's public key is correct");
};

# ---------------------------------------------------------------------------
# Diffie-Hellman Shared Secret
# ---------------------------------------------------------------------------
subtest 'Diffie-Hellman shared secret' => sub {
    my $alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    my $bob_private   = hex_to_bytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    my $expected = "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742";

    my $alice_public = x25519_base($alice_private);
    my $bob_public   = x25519_base($bob_private);

    my $shared_alice = x25519($alice_private, $bob_public);
    my $shared_bob   = x25519($bob_private, $alice_public);

    is(bytes_to_hex($shared_alice), $expected, "Alice's shared secret matches");
    is(bytes_to_hex($shared_bob),   $expected, "Bob's shared secret matches");
    is($shared_alice, $shared_bob, "Both parties compute the same secret");
};

# ---------------------------------------------------------------------------
# generate_keypair
# ---------------------------------------------------------------------------
subtest 'generate_keypair' => sub {
    my $alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    my $expected_public = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a";

    my ($priv, $pub) = generate_keypair($alice_private);

    is($priv, $alice_private, "private key is returned unchanged");
    is(bytes_to_hex($pub), $expected_public, "public key is derived correctly");
};

# ---------------------------------------------------------------------------
# Iterated Test — 1 Iteration
# ---------------------------------------------------------------------------
subtest 'iterated test (1 iteration)' => sub {
    # Start with k = u = 9 as 32-byte LE
    my $nine = "\x09" . ("\x00" x 31);
    my $expected = "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079";

    my $result = x25519($nine, $nine);
    is(bytes_to_hex($result), $expected, "1 iteration from k=u=9 is correct");
};

# ---------------------------------------------------------------------------
# Iterated Test — 1000 Iterations
# ---------------------------------------------------------------------------
subtest 'iterated test (1000 iterations)' => sub {
    my $k = "\x09" . ("\x00" x 31);
    my $u = "\x09" . ("\x00" x 31);
    my $expected = "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51";

    for my $i (1 .. 1000) {
        my $new_k = x25519($k, $u);
        $u = $k;
        $k = $new_k;
    }

    is(bytes_to_hex($k), $expected, "1000 iterations from k=u=9 is correct");
};

# ---------------------------------------------------------------------------
# Input Validation
# ---------------------------------------------------------------------------
subtest 'input validation' => sub {
    like(
        dies { x25519("\x01\x02\x03", "\x00" x 32) },
        qr/scalar must be 32 bytes/,
        "rejects short scalar"
    );

    like(
        dies { x25519("\x00" x 32, "\x01\x02\x03") },
        qr/u_point must be 32 bytes/,
        "rejects short u_point"
    );
};

done_testing();
