"""Tests for Ed25519 digital signatures (RFC 8032).

These tests verify correctness using the official test vectors from
RFC 8032 Section 7.1, plus additional edge-case tests for field arithmetic,
point operations, encoding/decoding, and error handling.
"""

from __future__ import annotations

import pytest

from coding_adventures_ed25519 import generate_keypair, sign, verify
from coding_adventures_ed25519.ed25519 import (
    B,
    IDENTITY,
    L,
    SQRT_M1,
    d,
    field_inv,
    field_sqrt,
    p,
    point_add,
    point_decode,
    point_double,
    point_encode,
    scalar_mult,
)


# ═══════════════════════════════════════════════════════════════════════════════
# FIELD ARITHMETIC TESTS
# ═══════════════════════════════════════════════════════════════════════════════


class TestFieldArithmetic:
    """Tests for modular arithmetic in GF(2^255-19)."""

    def test_field_inv_basic(self) -> None:
        """Inverse of 3: 3 * inv(3) ≡ 1 (mod p)."""
        assert (3 * field_inv(3)) % p == 1

    def test_field_inv_large(self) -> None:
        """Inverse of a large number works correctly."""
        val = 2**200 + 37
        assert (val * field_inv(val)) % p == 1

    def test_field_inv_one(self) -> None:
        """Inverse of 1 is 1."""
        assert field_inv(1) == 1

    def test_sqrt_m1_squared(self) -> None:
        """SQRT_M1 squared should equal -1 mod p."""
        assert (SQRT_M1 * SQRT_M1) % p == (-1) % p

    def test_field_sqrt_perfect_square(self) -> None:
        """Square root of a perfect square should round-trip."""
        val = 42
        sq = (val * val) % p
        root = field_sqrt(sq)
        assert (root * root) % p == sq

    def test_field_sqrt_no_root(self) -> None:
        """Non-residue should raise ValueError."""
        # 2 is a quadratic non-residue mod p
        with pytest.raises(ValueError, match="not a quadratic residue"):
            field_sqrt(2)

    def test_field_sqrt_zero(self) -> None:
        """Square root of 0 is 0."""
        assert field_sqrt(0) == 0


# ═══════════════════════════════════════════════════════════════════════════════
# POINT OPERATION TESTS
# ═══════════════════════════════════════════════════════════════════════════════


class TestPointOperations:
    """Tests for curve point arithmetic."""

    def test_identity_add(self) -> None:
        """Adding the identity to B should give B."""
        result = point_add(IDENTITY, B)
        # Compare in affine coordinates
        enc = point_encode(result)
        assert enc == point_encode(B)

    def test_double_equals_add(self) -> None:
        """Doubling B should equal B + B."""
        doubled = point_double(B)
        added = point_add(B, B)
        assert point_encode(doubled) == point_encode(added)

    def test_scalar_mult_zero(self) -> None:
        """0 * B = identity."""
        result = scalar_mult(0, B)
        assert point_encode(result) == point_encode(IDENTITY)

    def test_scalar_mult_one(self) -> None:
        """1 * B = B."""
        result = scalar_mult(1, B)
        assert point_encode(result) == point_encode(B)

    def test_scalar_mult_two(self) -> None:
        """2 * B = B + B."""
        result = scalar_mult(2, B)
        expected = point_add(B, B)
        assert point_encode(result) == point_encode(expected)

    def test_scalar_mult_order(self) -> None:
        """L * B = identity (L is the subgroup order)."""
        result = scalar_mult(L, B)
        assert point_encode(result) == point_encode(IDENTITY)

    def test_base_point_on_curve(self) -> None:
        """The base point should satisfy the curve equation."""
        from coding_adventures_ed25519.ed25519 import B_x, B_y

        # -x^2 + y^2 = 1 + d*x^2*y^2
        lhs = (-B_x * B_x + B_y * B_y) % p
        rhs = (1 + d * B_x * B_x * B_y * B_y) % p
        assert lhs == rhs


# ═══════════════════════════════════════════════════════════════════════════════
# POINT ENCODING/DECODING TESTS
# ═══════════════════════════════════════════════════════════════════════════════


class TestPointEncoding:
    """Tests for point compression and decompression."""

    def test_encode_decode_base_point(self) -> None:
        """Encoding and decoding the base point should round-trip."""
        encoded = point_encode(B)
        decoded = point_decode(encoded)
        assert point_encode(decoded) == encoded

    def test_encode_decode_identity(self) -> None:
        """The identity (0, 1) encodes as y=1 with sign bit 0."""
        encoded = point_encode(IDENTITY)
        decoded = point_decode(encoded)
        assert point_encode(decoded) == encoded

    def test_decode_invalid_length(self) -> None:
        """Decoding non-32-byte data should fail."""
        with pytest.raises(ValueError, match="32 bytes"):
            point_decode(b"\x00" * 31)

    def test_decode_y_out_of_range(self) -> None:
        """y >= p should be rejected."""
        # Set y = p (which is 2^255 - 19, encoded as little-endian)
        y_bytes = bytearray(p.to_bytes(32, "little"))
        # Clear the sign bit (high bit of byte[31])
        y_bytes[31] &= 0x7F
        # p is 2^255 - 19. In 32 LE bytes, byte[31] has bit 7 = 0 already
        # since p < 2^255. But y_bytes represents p which is >= p, so reject.
        with pytest.raises(ValueError):
            point_decode(bytes(y_bytes))

    def test_encode_decode_double_base(self) -> None:
        """2*B should encode and decode correctly."""
        double_b = scalar_mult(2, B)
        encoded = point_encode(double_b)
        decoded = point_decode(encoded)
        assert point_encode(decoded) == encoded


# ═══════════════════════════════════════════════════════════════════════════════
# RFC 8032 TEST VECTORS
# ═══════════════════════════════════════════════════════════════════════════════


class TestRFC8032Vectors:
    """Official test vectors from RFC 8032 Section 7.1."""

    def test_vector_1_empty_message(self) -> None:
        """Test vector 1: empty message."""
        seed = bytes.fromhex(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
        )
        expected_pub = bytes.fromhex(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        )
        expected_sig = bytes.fromhex(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155"
            "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        )
        message = b""

        pub, sec = generate_keypair(seed)
        assert pub == expected_pub

        sig = sign(message, sec)
        assert sig == expected_sig

        assert verify(message, sig, pub) is True

    def test_vector_2_one_byte(self) -> None:
        """Test vector 2: single byte message (0x72)."""
        seed = bytes.fromhex(
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"
        )
        expected_pub = bytes.fromhex(
            "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"
        )
        expected_sig = bytes.fromhex(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da"
            "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
        )
        message = bytes.fromhex("72")

        pub, sec = generate_keypair(seed)
        assert pub == expected_pub

        sig = sign(message, sec)
        assert sig == expected_sig

        assert verify(message, sig, pub) is True

    def test_vector_3_two_bytes(self) -> None:
        """Test vector 3: two byte message (0xaf82)."""
        seed = bytes.fromhex(
            "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7"
        )
        expected_pub = bytes.fromhex(
            "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025"
        )
        expected_sig = bytes.fromhex(
            "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac"
            "18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a"
        )
        message = bytes.fromhex("af82")

        pub, sec = generate_keypair(seed)
        assert pub == expected_pub

        sig = sign(message, sec)
        assert sig == expected_sig

        assert verify(message, sig, pub) is True

    def test_vector_4_1023_bytes(self) -> None:
        """Test vector 4: 1023-byte message."""
        seed = bytes.fromhex(
            "f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5"
        )
        expected_pub = bytes.fromhex(
            "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e"
        )
        expected_sig = bytes.fromhex(
            "d686294b743c6760c6a78a2c4c2fc76115c2600b8f083acde59e7cee32578c0f"
            "59ea4219ab9b5896795e4e2b87a30270aa0e3099eee944e9e67a1b22df41ff07"
        )
        message = bytes.fromhex(
            "08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98"
            "fa6e264bf09efe12ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d8"
            "79de7c0046dc4996d9e773f4bc9efe5738829adb26c81b37c93a1b270b20329d"
            "658675fc6ea534e0810a4432826bf58c941efb65d57a338bbd2e26640f89ffbc"
            "1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe"
            "ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e"
            "06203b33890cc9b832f79ef80560ccb9a39ce767967ed628c6ad573cb116dbef"
            "fefd75499da96bd68a8a97b928a8bbc103b6621fcde2beca1231d206be6cd9ec"
            "7aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53f1d8ac70bdcb7ed"
            "185ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb"
            "2d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc2"
            "4554119a831a9aad6079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f27"
            "088d78b7e883c8759d2c4f5c65adb7553878ad575f9fad878e80a0c9ba63bcbc"
            "c2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2cf65bd92dd597b0"
            "7e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128bab"
            "27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51ad"
            "dd3b75a2b1ae0784504df543af8969be3ea7082ff7fc9888c144da2af58429ec"
            "96031dbcad3dad9af0dcbaaaf268cb8fcffead94f3c7ca495e056a9b47acdb75"
            "1fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301651339e22904cc8c4"
            "2f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8c"
            "a61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34dff"
            "7310fdc82aebfd904b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08d"
            "78ff4ef5d63653a65040cf9bfd4aca7984a74d37145986780fc0b16ac451649d"
            "e6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a3ca8e1b939ae49e48"
            "8acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a32"
            "ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e6"
            "aed3fb0f9cd71a43dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5fb"
            "93246f6b1116398a346f1a641f3b041e989f7914f90cc2c7fff357876e506b50"
            "d334ba77c225bc307ba537152f3f1610e4eafe595f6d9d90d11faa933a15ef13"
            "69546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380db"
            "2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c0"
            "618983f8741c5ef68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d00"
        )

        pub, sec = generate_keypair(seed)
        assert pub == expected_pub

        sig = sign(message, sec)
        assert sig == expected_sig

        assert verify(message, sig, pub) is True


# ═══════════════════════════════════════════════════════════════════════════════
# VERIFICATION EDGE CASES
# ═══════════════════════════════════════════════════════════════════════════════


class TestVerificationEdgeCases:
    """Tests that verification correctly rejects invalid inputs."""

    def _make_keypair(self) -> tuple[bytes, bytes]:
        """Helper: generate a fixed keypair for testing."""
        seed = bytes.fromhex(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
        )
        return generate_keypair(seed)

    def test_wrong_message(self) -> None:
        """Signature should not verify for a different message."""
        pub, sec = self._make_keypair()
        sig = sign(b"hello", sec)
        assert verify(b"world", sig, pub) is False

    def test_wrong_public_key(self) -> None:
        """Signature should not verify with a different public key."""
        pub1, sec1 = self._make_keypair()
        seed2 = bytes.fromhex(
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"
        )
        pub2, _sec2 = generate_keypair(seed2)
        sig = sign(b"hello", sec1)
        assert verify(b"hello", sig, pub2) is False

    def test_tampered_signature_r(self) -> None:
        """Flipping a bit in R should invalidate the signature."""
        pub, sec = self._make_keypair()
        sig = sign(b"hello", sec)
        tampered = bytearray(sig)
        tampered[0] ^= 1  # flip one bit in R
        assert verify(b"hello", bytes(tampered), pub) is False

    def test_tampered_signature_s(self) -> None:
        """Flipping a bit in S should invalidate the signature."""
        pub, sec = self._make_keypair()
        sig = sign(b"hello", sec)
        tampered = bytearray(sig)
        tampered[32] ^= 1  # flip one bit in S
        assert verify(b"hello", bytes(tampered), pub) is False

    def test_invalid_signature_length(self) -> None:
        """Signature of wrong length should return False."""
        pub, _sec = self._make_keypair()
        assert verify(b"hello", b"\x00" * 63, pub) is False
        assert verify(b"hello", b"\x00" * 65, pub) is False

    def test_invalid_public_key_length(self) -> None:
        """Public key of wrong length should return False."""
        _pub, sec = self._make_keypair()
        sig = sign(b"hello", sec)
        assert verify(b"hello", sig, b"\x00" * 31) is False

    def test_s_out_of_range(self) -> None:
        """S >= L should be rejected."""
        pub, sec = self._make_keypair()
        sig = sign(b"hello", sec)
        # Replace S with L (which is too large)
        tampered = sig[:32] + L.to_bytes(32, "little")
        assert verify(b"hello", tampered, pub) is False


# ═══════════════════════════════════════════════════════════════════════════════
# KEY GENERATION EDGE CASES
# ═══════════════════════════════════════════════════════════════════════════════


class TestKeyGeneration:
    """Tests for key generation edge cases."""

    def test_invalid_seed_length(self) -> None:
        """Seed must be exactly 32 bytes."""
        with pytest.raises(ValueError, match="32 bytes"):
            generate_keypair(b"\x00" * 31)
        with pytest.raises(ValueError, match="32 bytes"):
            generate_keypair(b"\x00" * 33)

    def test_invalid_secret_key_length(self) -> None:
        """Secret key must be exactly 64 bytes."""
        with pytest.raises(ValueError, match="64 bytes"):
            sign(b"hello", b"\x00" * 63)

    def test_deterministic(self) -> None:
        """Same seed should always produce the same keypair."""
        seed = bytes(range(32))
        pub1, sec1 = generate_keypair(seed)
        pub2, sec2 = generate_keypair(seed)
        assert pub1 == pub2
        assert sec1 == sec2

    def test_sign_deterministic(self) -> None:
        """Same message + key should always produce the same signature."""
        seed = bytes(range(32))
        _pub, sec = generate_keypair(seed)
        sig1 = sign(b"hello", sec)
        sig2 = sign(b"hello", sec)
        assert sig1 == sig2

    def test_secret_key_format(self) -> None:
        """Secret key should be seed || public_key."""
        seed = bytes(range(32))
        pub, sec = generate_keypair(seed)
        assert sec[:32] == seed
        assert sec[32:] == pub
