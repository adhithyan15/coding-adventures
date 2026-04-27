"""
Tests for X25519 (RFC 7748)
============================

These test vectors come directly from RFC 7748 Section 6.1.
They are the canonical way to verify an X25519 implementation.
"""

from __future__ import annotations

import pytest
from coding_adventures_x25519 import (
    P,
    A24,
    cswap,
    decode_scalar,
    decode_u_coordinate,
    encode_u_coordinate,
    field_add,
    field_invert,
    field_mul,
    field_square,
    field_sub,
    generate_keypair,
    x25519,
    x25519_base,
)


# ============================================================================
# Field Arithmetic Tests
# ============================================================================


class TestFieldArithmetic:
    """Test the basic field operations over GF(2^255 - 19)."""

    def test_add_basic(self) -> None:
        """Simple addition in the field."""
        assert field_add(3, 5) == 8
        assert field_add(0, 0) == 0
        assert field_add(P - 1, 1) == 0  # Wraps around

    def test_add_wraps(self) -> None:
        """Addition that exceeds p wraps around."""
        assert field_add(P - 1, 2) == 1
        assert field_add(P - 1, P - 1) == P - 2

    def test_sub_basic(self) -> None:
        """Simple subtraction in the field."""
        assert field_sub(10, 3) == 7
        assert field_sub(0, 0) == 0
        assert field_sub(0, 1) == P - 1  # Wraps around

    def test_sub_wraps(self) -> None:
        """Subtraction below zero wraps around."""
        assert field_sub(3, 5) == P - 2

    def test_mul_basic(self) -> None:
        """Basic multiplication."""
        assert field_mul(3, 7) == 21
        assert field_mul(0, 12345) == 0
        assert field_mul(1, P - 1) == P - 1

    def test_mul_wraps(self) -> None:
        """Multiplication that exceeds p."""
        # (p-1) * 2 = 2p - 2 ≡ -2 ≡ p - 2
        assert field_mul(P - 1, 2) == P - 2

    def test_square_basic(self) -> None:
        """Squaring a field element."""
        assert field_square(3) == 9
        assert field_square(0) == 0
        assert field_square(1) == 1

    def test_square_consistency(self) -> None:
        """Squaring should match multiplication."""
        for val in [7, 42, P - 1, P - 2, 121666]:
            assert field_square(val) == field_mul(val, val)

    def test_invert_basic(self) -> None:
        """a * a^(-1) = 1 for non-zero elements."""
        for val in [1, 2, 3, 7, 42, 121666, P - 1]:
            inv = field_invert(val)
            assert field_mul(val, inv) == 1

    def test_invert_of_one(self) -> None:
        """The inverse of 1 is 1."""
        assert field_invert(1) == 1

    def test_a24_value(self) -> None:
        """Verify the a24 constant matches (486662 + 2) / 4."""
        assert A24 == 121666
        # Also verify: 4 * a24 - 2 = A (the curve parameter)
        assert 4 * A24 - 2 == 486662


# ============================================================================
# Conditional Swap Tests
# ============================================================================


class TestCswap:
    """Test constant-time conditional swap."""

    def test_no_swap(self) -> None:
        """swap=0 means no change."""
        a, b = cswap(0, 10, 20)
        assert a == 10
        assert b == 20

    def test_swap(self) -> None:
        """swap=1 means values are exchanged."""
        a, b = cswap(1, 10, 20)
        assert a == 20
        assert b == 10

    def test_swap_with_field_elements(self) -> None:
        """Works with large field elements."""
        x = P - 1
        y = 42
        a, b = cswap(1, x, y)
        assert a == 42
        assert b == P - 1

    def test_swap_same_values(self) -> None:
        """Swapping identical values is a no-op."""
        a, b = cswap(1, 7, 7)
        assert a == 7
        assert b == 7


# ============================================================================
# Encoding / Decoding Tests
# ============================================================================


class TestEncoding:
    """Test little-endian encoding and decoding of field elements."""

    def test_decode_u_coordinate_masks_high_bit(self) -> None:
        """Bit 255 (high bit of byte 31) is masked off."""
        u_bytes = b"\x00" * 31 + b"\xff"
        val = decode_u_coordinate(u_bytes)
        # 0xff with high bit masked = 0x7f = 127
        assert val == 127 << (31 * 8)

    def test_decode_u_coordinate_base_point(self) -> None:
        """The base point u=9 decodes correctly."""
        base = (9).to_bytes(32, byteorder="little")
        assert decode_u_coordinate(base) == 9

    def test_encode_decode_roundtrip(self) -> None:
        """Encoding then decoding recovers the original value."""
        for val in [0, 1, 9, 42, 2**200, P - 1]:
            encoded = encode_u_coordinate(val)
            decoded = decode_u_coordinate(encoded)
            assert decoded == val % P

    def test_encode_reduces_mod_p(self) -> None:
        """Values >= p are reduced before encoding."""
        encoded_p = encode_u_coordinate(P)
        encoded_0 = encode_u_coordinate(0)
        assert encoded_p == encoded_0

    def test_decode_scalar_clamps(self) -> None:
        """Verify scalar clamping sets/clears the right bits."""
        # All-ones input
        k = b"\xff" * 32
        val = decode_scalar(k)

        # Bit 0, 1, 2 must be 0 (lowest 3 bits cleared via &= 248)
        assert val & 0x07 == 0

        # Bit 255 must be 0 (high bit cleared via &= 127)
        assert val >> 255 == 0

        # Bit 254 must be 1 (set via |= 64)
        assert (val >> 254) & 1 == 1

    def test_decode_scalar_sets_bit_254(self) -> None:
        """Even an all-zeros scalar gets bit 254 set."""
        k = b"\x00" * 32
        val = decode_scalar(k)
        assert (val >> 254) & 1 == 1

    def test_invalid_length_u(self) -> None:
        """u-coordinate must be exactly 32 bytes."""
        with pytest.raises(ValueError, match="32 bytes"):
            decode_u_coordinate(b"\x00" * 31)

    def test_invalid_length_scalar(self) -> None:
        """Scalar must be exactly 32 bytes."""
        with pytest.raises(ValueError, match="32 bytes"):
            decode_scalar(b"\x00" * 33)


# ============================================================================
# RFC 7748 Section 6.1 — Test Vectors
# ============================================================================


class TestRFC7748Vectors:
    """Test against the official RFC 7748 test vectors.

    These are the gold standard: if your implementation produces these
    exact outputs, it is interoperable with every other X25519 implementation.
    """

    def test_vector_1(self) -> None:
        """RFC 7748 Section 6.1 — Test Vector 1."""
        scalar = bytes.fromhex(
            "a546e36bf0527c9d3b16154b82465edd"
            "62144c0ac1fc5a18506a2244ba449ac4"
        )
        u = bytes.fromhex(
            "e6db6867583030db3594c1a424b15f7c"
            "726624ec26b3353b10a903a6d0ab1c4c"
        )
        expected = bytes.fromhex(
            "c3da55379de9c6908e94ea4df28d084f"
            "32eccf03491c71f754b4075577a28552"
        )
        assert x25519(scalar, u) == expected

    def test_vector_2(self) -> None:
        """RFC 7748 Section 6.1 — Test Vector 2."""
        scalar = bytes.fromhex(
            "4b66e9d4d1b4673c5ad22691957d6af5"
            "c11b6421e0ea01d42ca4169e7918ba0d"
        )
        u = bytes.fromhex(
            "e5210f12786811d3f4b7959d0538ae2c"
            "31dbe7106fc03c3efc4cd549c715a493"
        )
        expected = bytes.fromhex(
            "95cbde9476e8907d7aade45cb4b873f8"
            "8b595a68799fa152e6f8f7647aac7957"
        )
        assert x25519(scalar, u) == expected


# ============================================================================
# Diffie-Hellman Key Agreement Tests
# ============================================================================


class TestDiffieHellman:
    """Test the complete Diffie-Hellman key exchange flow."""

    def test_alice_public_key(self) -> None:
        """Alice's public key from her private key."""
        alice_private = bytes.fromhex(
            "77076d0a7318a57d3c16c17251b26645"
            "df4c2f87ebc0992ab177fba51db92c2a"
        )
        alice_public_expected = bytes.fromhex(
            "8520f0098930a754748b7ddcb43ef75a"
            "0dbf3a0d26381af4eba4a98eaa9b4e6a"
        )
        assert x25519_base(alice_private) == alice_public_expected

    def test_bob_public_key(self) -> None:
        """Bob's public key from his private key."""
        bob_private = bytes.fromhex(
            "5dab087e624a8a4b79e17f8b83800ee6"
            "6f3bb1292618b6fd1c2f8b27ff88e0eb"
        )
        bob_public_expected = bytes.fromhex(
            "de9edb7d7b7dc1b4d35b61c2ece43537"
            "3f8343c85b78674dadfc7e146f882b4f"
        )
        assert x25519_base(bob_private) == bob_public_expected

    def test_shared_secret(self) -> None:
        """Alice and Bob derive the same shared secret.

        This is the whole point of Diffie-Hellman:
            x25519(alice_private, bob_public) == x25519(bob_private, alice_public)
        """
        alice_private = bytes.fromhex(
            "77076d0a7318a57d3c16c17251b26645"
            "df4c2f87ebc0992ab177fba51db92c2a"
        )
        bob_private = bytes.fromhex(
            "5dab087e624a8a4b79e17f8b83800ee6"
            "6f3bb1292618b6fd1c2f8b27ff88e0eb"
        )
        alice_public = x25519_base(alice_private)
        bob_public = x25519_base(bob_private)

        shared_ab = x25519(alice_private, bob_public)
        shared_ba = x25519(bob_private, alice_public)

        expected = bytes.fromhex(
            "4a5d9d5ba4ce2de1728e3bf480350f25"
            "e07e21c947d19e3376f09b3c1e161742"
        )
        assert shared_ab == expected
        assert shared_ba == expected

    def test_generate_keypair_is_x25519_base(self) -> None:
        """generate_keypair is just an alias for x25519_base."""
        alice_private = bytes.fromhex(
            "77076d0a7318a57d3c16c17251b26645"
            "df4c2f87ebc0992ab177fba51db92c2a"
        )
        assert generate_keypair(alice_private) == x25519_base(alice_private)


# ============================================================================
# Iterated Test Vector (RFC 7748 Section 6.1)
# ============================================================================


class TestIterated:
    """Test the iterated X25519 computation from RFC 7748.

    Starting from k = u = 9 (as 32-byte LE), repeatedly compute:
        k, u = x25519(k, u), k

    This catches subtle bugs that single-vector tests might miss.
    """

    def test_1_iteration(self) -> None:
        """After 1 iteration."""
        k = (9).to_bytes(32, byteorder="little")
        u = (9).to_bytes(32, byteorder="little")

        new_k = x25519(k, u)

        expected = bytes.fromhex(
            "422c8e7a6227d7bca1350b3e2bb7279f"
            "7897b87bb6854b783c60e80311ae3079"
        )
        assert new_k == expected

    def test_1000_iterations(self) -> None:
        """After 1000 iterations.

        This takes a few seconds in Python but is important for catching
        accumulation errors in the field arithmetic.
        """
        k = (9).to_bytes(32, byteorder="little")
        u = (9).to_bytes(32, byteorder="little")

        for _ in range(1000):
            k, u = x25519(k, u), k

        expected = bytes.fromhex(
            "684cf59ba83309552800ef566f2f4d3c"
            "1c3887c49360e3875f2eb94d99532c51"
        )
        assert k == expected

    # def test_1000000_iterations(self) -> None:
    #     """After 1,000,000 iterations.
    #
    #     WARNING: This takes a VERY long time in pure Python (hours).
    #     Uncomment only for thorough verification.
    #
    #     Expected output:
    #       7c3911e0ab2586fd864497297e575e6f3bc601c0883c30df5f4dd2d24f665424
    #     """
    #     k = (9).to_bytes(32, byteorder="little")
    #     u = (9).to_bytes(32, byteorder="little")
    #     for _ in range(1_000_000):
    #         k, u = x25519(k, u), k
    #     expected = bytes.fromhex(
    #         "7c3911e0ab2586fd864497297e575e6f"
    #         "3bc601c0883c30df5f4dd2d24f665424"
    #     )
    #     assert k == expected


# ============================================================================
# Edge Cases
# ============================================================================


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_base_point_is_nine(self) -> None:
        """The base point constant is u=9 in little-endian."""
        from coding_adventures_x25519 import BASE_POINT

        assert BASE_POINT == b"\x09" + b"\x00" * 31

    def test_p_value(self) -> None:
        """Verify p = 2^255 - 19."""
        assert P == 2**255 - 19

    def test_field_identity_elements(self) -> None:
        """0 is additive identity, 1 is multiplicative identity."""
        assert field_add(42, 0) == 42
        assert field_mul(42, 1) == 42

    def test_field_negation(self) -> None:
        """a + (-a) = 0 in the field."""
        a = 12345
        neg_a = field_sub(0, a)
        assert field_add(a, neg_a) == 0

    def test_field_distributive(self) -> None:
        """a * (b + c) = a*b + a*c  — the distributive law."""
        a, b, c = 123, 456, 789
        lhs = field_mul(a, field_add(b, c))
        rhs = field_add(field_mul(a, b), field_mul(a, c))
        assert lhs == rhs
