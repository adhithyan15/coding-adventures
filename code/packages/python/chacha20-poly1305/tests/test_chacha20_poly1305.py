"""
Tests for ChaCha20-Poly1305 (RFC 8439)
=======================================

These tests verify the implementation against the official test vectors
from RFC 8439 (Sections 2.4.2, 2.5.2, and 2.8.2), plus additional
edge-case and property tests.
"""

from __future__ import annotations

import pytest

from coding_adventures_chacha20_poly1305 import (
    _chacha20_block,
    _constant_time_compare,
    _pad16,
    _quarter_round,
    _rotl32,
    aead_decrypt,
    aead_encrypt,
    chacha20_encrypt,
    poly1305_mac,
)

# ===================================================================
# Helper to convert hex strings to bytes
# ===================================================================

def hex2bytes(h: str) -> bytes:
    """Convert a hex string (with optional spaces/newlines) to bytes."""
    return bytes.fromhex(h.replace(" ", "").replace("\n", ""))


# ===================================================================
# RFC 8439 Test Vectors
# ===================================================================

# --- ChaCha20 (Section 2.4.2) ---
CHACHA20_KEY = hex2bytes(
    "000102030405060708090a0b0c0d0e0f"
    "101112131415161718191a1b1c1d1e1f"
)
CHACHA20_NONCE = hex2bytes("000000000000004a00000000")
CHACHA20_COUNTER = 1
CHACHA20_PLAINTEXT = (
    b"Ladies and Gentlemen of the class of '99: "
    b"If I could offer you only one tip for the future, "
    b"sunscreen would be it."
)
CHACHA20_EXPECTED_CT = hex2bytes(
    "6e2e359a2568f98041ba0728dd0d6981"
    "e97e7aec1d4360c20a27afccfd9fae0b"
    "f91b65c5524733ab8f593dabcd62b357"
    "1639d624e65152ab8f530c359f0861d8"
    "07ca0dbf500d6a6156a38e088a22b65e"
    "52bc514d16ccf806818ce91ab7793736"
    "5af90bbf74a35be6b40b8eedf2785e42"
    "874d"
)

# --- Poly1305 (Section 2.5.2) ---
POLY1305_KEY = hex2bytes(
    "85d6be7857556d337f4452fe42d506a8"
    "0103808afb0db2fd4abff6af4149f51b"
)
POLY1305_MESSAGE = b"Cryptographic Forum Research Group"
POLY1305_EXPECTED_TAG = hex2bytes("a8061dc1305136c6c22b8baf0c0127a9")

# --- AEAD (Section 2.8.2) ---
AEAD_KEY = hex2bytes(
    "808182838485868788898a8b8c8d8e8f"
    "909192939495969798999a9b9c9d9e9f"
)
AEAD_NONCE = hex2bytes("070000004041424344454647")
AEAD_AAD = hex2bytes("50515253c0c1c2c3c4c5c6c7")
AEAD_PLAINTEXT = (
    b"Ladies and Gentlemen of the class of '99: "
    b"If I could offer you only one tip for the future, "
    b"sunscreen would be it."
)
AEAD_EXPECTED_CT = hex2bytes(
    "d31a8d34648e60db7b86afbc53ef7ec2"
    "a4aded51296e08fea9e2b5a736ee62d6"
    "3dbea45e8ca9671282fafb69da92728b"
    "1a71de0a9e060b2905d6a5b67ecd3b36"
    "92ddbd7f2d778b8c9803aee328091b58"
    "fab324e4fad675945585808b4831d7bc"
    "3ff4def08e4b7a9de576d26586cec64b"
    "6116"
)
AEAD_EXPECTED_TAG = hex2bytes("1ae10b594f09e26a7e902ecbd0600691")


# ===================================================================
# Unit Tests: Low-level helpers
# ===================================================================

class TestRotl32:
    """Tests for the 32-bit left rotation helper."""

    def test_rotate_left_by_16(self) -> None:
        assert _rotl32(0xAABBCCDD, 16) == 0xCCDDAABB

    def test_rotate_left_by_0(self) -> None:
        assert _rotl32(0x12345678, 0) == 0x12345678

    def test_rotate_left_by_32(self) -> None:
        # Rotating by 32 bits is a full rotation -- should return original
        assert _rotl32(0x12345678, 32) == 0x12345678

    def test_rotate_left_by_1(self) -> None:
        # 0x80000000 rotated left 1 = 0x00000001
        assert _rotl32(0x80000000, 1) == 0x00000001

    def test_rotate_left_by_7(self) -> None:
        # 0x00000001 rotated left 7 = 0x00000080
        assert _rotl32(0x00000001, 7) == 0x00000080


class TestQuarterRound:
    """Tests for the ChaCha20 quarter round function.

    RFC 8439 Section 2.1.1 provides a test vector for the quarter round.
    """

    def test_rfc_quarter_round(self) -> None:
        """RFC 8439 Section 2.1.1 quarter round test vector."""
        state = [
            0x879531E0, 0xC5ECF37D, 0x516461B1, 0xC9A62F8A,
            0x44C20EF3, 0x3390AF7F, 0xD9FC690B, 0x2A5F714C,
            0x53372767, 0xB00A5631, 0x974C541A, 0x359E9963,
            0x5C971061, 0x3D631689, 0x2098D9D6, 0x91DBD320,
        ]
        _quarter_round(state, 2, 7, 8, 13)
        assert state[2] == 0xBDB886DC
        assert state[7] == 0xCFACAFD2
        assert state[8] == 0xE46BEA80
        assert state[13] == 0xCCC07C79


class TestPad16:
    """Tests for the pad16 helper."""

    def test_no_padding_needed(self) -> None:
        assert _pad16(b"\x00" * 16) == b""

    def test_padding_needed(self) -> None:
        assert _pad16(b"\x00" * 10) == b"\x00" * 6

    def test_one_byte(self) -> None:
        assert _pad16(b"\x00") == b"\x00" * 15

    def test_empty(self) -> None:
        assert _pad16(b"") == b""


class TestConstantTimeCompare:
    """Tests for constant-time comparison."""

    def test_equal(self) -> None:
        assert _constant_time_compare(b"hello", b"hello") is True

    def test_not_equal(self) -> None:
        assert _constant_time_compare(b"hello", b"world") is False

    def test_different_lengths(self) -> None:
        assert _constant_time_compare(b"hello", b"hell") is False

    def test_empty(self) -> None:
        assert _constant_time_compare(b"", b"") is True


# ===================================================================
# ChaCha20 Tests
# ===================================================================

class TestChaCha20Block:
    """Tests for the ChaCha20 block function."""

    def test_rfc_section_2_3_2(self) -> None:
        """RFC 8439 Section 2.3.2: ChaCha20 block function test vector."""
        key = hex2bytes(
            "000102030405060708090a0b0c0d0e0f"
            "101112131415161718191a1b1c1d1e1f"
        )
        nonce = hex2bytes("000000090000004a00000000")
        counter = 1
        block = _chacha20_block(key, counter, nonce)

        # The expected output (first 16 bytes) from RFC 8439 Section 2.3.2:
        # 10 f1 e7 e4 d1 3b 59 15 50 0f dd 1f a3 20 71 c4
        assert block[0:4] == hex2bytes("10f1e7e4")
        assert len(block) == 64


class TestChaCha20Encrypt:
    """Tests for the ChaCha20 stream cipher."""

    def test_rfc_section_2_4_2(self) -> None:
        """RFC 8439 Section 2.4.2: Full encryption test vector."""
        ct = chacha20_encrypt(
            CHACHA20_PLAINTEXT, CHACHA20_KEY, CHACHA20_NONCE, CHACHA20_COUNTER,
        )
        assert ct == CHACHA20_EXPECTED_CT

    def test_decrypt_is_encrypt(self) -> None:
        """ChaCha20 is symmetric: encrypt(encrypt(pt)) = pt."""
        ct = chacha20_encrypt(
            CHACHA20_PLAINTEXT, CHACHA20_KEY, CHACHA20_NONCE, CHACHA20_COUNTER,
        )
        pt = chacha20_encrypt(
            ct, CHACHA20_KEY, CHACHA20_NONCE, CHACHA20_COUNTER,
        )
        assert pt == CHACHA20_PLAINTEXT

    def test_empty_plaintext(self) -> None:
        """Encrypting empty data returns empty data."""
        ct = chacha20_encrypt(b"", CHACHA20_KEY, CHACHA20_NONCE, 0)
        assert ct == b""

    def test_single_byte(self) -> None:
        """Encrypting one byte works correctly."""
        ct = chacha20_encrypt(b"\x00", CHACHA20_KEY, CHACHA20_NONCE, 0)
        assert len(ct) == 1
        # Decrypting should give back the original
        pt = chacha20_encrypt(ct, CHACHA20_KEY, CHACHA20_NONCE, 0)
        assert pt == b"\x00"

    def test_exactly_64_bytes(self) -> None:
        """A plaintext that is exactly one block (64 bytes) long."""
        data = bytes(range(64))
        ct = chacha20_encrypt(data, CHACHA20_KEY, CHACHA20_NONCE, 0)
        pt = chacha20_encrypt(ct, CHACHA20_KEY, CHACHA20_NONCE, 0)
        assert pt == data

    def test_multi_block(self) -> None:
        """A plaintext spanning multiple 64-byte blocks."""
        data = bytes(range(256)) * 2  # 512 bytes = 8 blocks
        ct = chacha20_encrypt(data, CHACHA20_KEY, CHACHA20_NONCE, 0)
        pt = chacha20_encrypt(ct, CHACHA20_KEY, CHACHA20_NONCE, 0)
        assert pt == data

    def test_invalid_key_length(self) -> None:
        with pytest.raises(ValueError, match="Key must be 32 bytes"):
            chacha20_encrypt(b"hello", b"short", CHACHA20_NONCE, 0)

    def test_invalid_nonce_length(self) -> None:
        with pytest.raises(ValueError, match="Nonce must be 12 bytes"):
            chacha20_encrypt(b"hello", CHACHA20_KEY, b"short", 0)


# ===================================================================
# Poly1305 Tests
# ===================================================================

class TestPoly1305:
    """Tests for the Poly1305 MAC."""

    def test_rfc_section_2_5_2(self) -> None:
        """RFC 8439 Section 2.5.2: Poly1305 test vector."""
        tag = poly1305_mac(POLY1305_MESSAGE, POLY1305_KEY)
        assert tag == POLY1305_EXPECTED_TAG

    def test_empty_message(self) -> None:
        """MAC of empty message should still produce a valid 16-byte tag."""
        tag = poly1305_mac(b"", POLY1305_KEY)
        assert len(tag) == 16

    def test_single_byte_message(self) -> None:
        """MAC of a single byte should produce a valid tag."""
        tag = poly1305_mac(b"\x00", POLY1305_KEY)
        assert len(tag) == 16

    def test_exactly_16_bytes(self) -> None:
        """Message that is exactly one Poly1305 block."""
        tag = poly1305_mac(b"\x00" * 16, POLY1305_KEY)
        assert len(tag) == 16

    def test_different_messages_different_tags(self) -> None:
        """Different messages with same key produce different tags."""
        tag1 = poly1305_mac(b"hello", POLY1305_KEY)
        tag2 = poly1305_mac(b"world", POLY1305_KEY)
        assert tag1 != tag2

    def test_different_keys_different_tags(self) -> None:
        """Same message with different keys produces different tags."""
        key2 = bytes(range(32))
        tag1 = poly1305_mac(b"hello", POLY1305_KEY)
        tag2 = poly1305_mac(b"hello", key2)
        assert tag1 != tag2

    def test_invalid_key_length(self) -> None:
        with pytest.raises(ValueError, match="Poly1305 key must be 32 bytes"):
            poly1305_mac(b"hello", b"short")


# ===================================================================
# AEAD Tests
# ===================================================================

class TestAEAD:
    """Tests for the AEAD authenticated encryption/decryption."""

    def test_rfc_section_2_8_2_encrypt(self) -> None:
        """RFC 8439 Section 2.8.2: AEAD encryption test vector."""
        ct, tag = aead_encrypt(AEAD_PLAINTEXT, AEAD_KEY, AEAD_NONCE, AEAD_AAD)
        assert ct == AEAD_EXPECTED_CT
        assert tag == AEAD_EXPECTED_TAG

    def test_rfc_section_2_8_2_decrypt(self) -> None:
        """RFC 8439 Section 2.8.2: AEAD decryption test vector."""
        pt = aead_decrypt(
            AEAD_EXPECTED_CT, AEAD_KEY, AEAD_NONCE, AEAD_AAD, AEAD_EXPECTED_TAG,
        )
        assert pt == AEAD_PLAINTEXT

    def test_roundtrip(self) -> None:
        """Encrypt then decrypt should return original plaintext."""
        plaintext = b"Hello, ChaCha20-Poly1305!"
        key = bytes(range(32))
        nonce = bytes(range(12))
        aad = b"additional data"

        ct, tag = aead_encrypt(plaintext, key, nonce, aad)
        pt = aead_decrypt(ct, key, nonce, aad, tag)
        assert pt == plaintext

    def test_empty_plaintext(self) -> None:
        """AEAD with empty plaintext (authentication only)."""
        key = bytes(range(32))
        nonce = bytes(range(12))
        aad = b"just authenticate this"

        ct, tag = aead_encrypt(b"", key, nonce, aad)
        assert ct == b""
        assert len(tag) == 16

        pt = aead_decrypt(ct, key, nonce, aad, tag)
        assert pt == b""

    def test_empty_aad(self) -> None:
        """AEAD with no additional authenticated data."""
        key = bytes(range(32))
        nonce = bytes(range(12))

        ct, tag = aead_encrypt(b"secret", key, nonce, b"")
        pt = aead_decrypt(ct, key, nonce, b"", tag)
        assert pt == b"secret"

    def test_tampered_ciphertext_fails(self) -> None:
        """Modifying even one bit of ciphertext should fail authentication."""
        key = bytes(range(32))
        nonce = bytes(range(12))
        aad = b"aad"

        ct, tag = aead_encrypt(b"secret message", key, nonce, aad)

        # Flip one bit in the ciphertext
        tampered = bytearray(ct)
        tampered[0] ^= 0x01
        tampered = bytes(tampered)

        with pytest.raises(ValueError, match="tag mismatch"):
            aead_decrypt(tampered, key, nonce, aad, tag)

    def test_tampered_tag_fails(self) -> None:
        """A wrong tag should fail authentication."""
        key = bytes(range(32))
        nonce = bytes(range(12))
        aad = b"aad"

        ct, tag = aead_encrypt(b"secret message", key, nonce, aad)

        bad_tag = bytes(16)  # all zeros
        with pytest.raises(ValueError, match="tag mismatch"):
            aead_decrypt(ct, key, nonce, aad, bad_tag)

    def test_wrong_aad_fails(self) -> None:
        """Using different AAD for decrypt should fail authentication."""
        key = bytes(range(32))
        nonce = bytes(range(12))

        ct, tag = aead_encrypt(b"secret", key, nonce, b"correct aad")

        with pytest.raises(ValueError, match="tag mismatch"):
            aead_decrypt(ct, key, nonce, b"wrong aad", tag)

    def test_wrong_key_fails(self) -> None:
        """Using a different key for decrypt should fail authentication."""
        key1 = bytes(range(32))
        key2 = bytes(range(1, 33))
        nonce = bytes(range(12))

        ct, tag = aead_encrypt(b"secret", key1, nonce, b"aad")

        with pytest.raises(ValueError, match="tag mismatch"):
            aead_decrypt(ct, key2, nonce, b"aad", tag)

    def test_wrong_nonce_fails(self) -> None:
        """Using a different nonce for decrypt should fail authentication."""
        key = bytes(range(32))
        nonce1 = bytes(range(12))
        nonce2 = bytes(range(1, 13))

        ct, tag = aead_encrypt(b"secret", key, nonce1, b"aad")

        with pytest.raises(ValueError, match="tag mismatch"):
            aead_decrypt(ct, key, nonce2, b"aad", tag)

    def test_invalid_key_length_encrypt(self) -> None:
        with pytest.raises(ValueError, match="Key must be 32 bytes"):
            aead_encrypt(b"hello", b"short", bytes(12))

    def test_invalid_nonce_length_encrypt(self) -> None:
        with pytest.raises(ValueError, match="Nonce must be 12 bytes"):
            aead_encrypt(b"hello", bytes(32), b"short")

    def test_invalid_tag_length_decrypt(self) -> None:
        with pytest.raises(ValueError, match="Tag must be 16 bytes"):
            aead_decrypt(b"hello", bytes(32), bytes(12), b"", b"short")

    def test_large_plaintext(self) -> None:
        """Test with a plaintext spanning many blocks."""
        key = bytes(range(32))
        nonce = bytes(range(12))
        plaintext = b"A" * 1024  # 16 blocks

        ct, tag = aead_encrypt(plaintext, key, nonce, b"")
        pt = aead_decrypt(ct, key, nonce, b"", tag)
        assert pt == plaintext
