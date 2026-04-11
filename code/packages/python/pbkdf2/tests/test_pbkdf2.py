"""Tests for PBKDF2 — RFC 8018 / RFC 6070 / RFC 7914."""

from __future__ import annotations

import pytest

from coding_adventures_pbkdf2 import (
    pbkdf2_hmac_sha1,
    pbkdf2_hmac_sha1_hex,
    pbkdf2_hmac_sha256,
    pbkdf2_hmac_sha256_hex,
    pbkdf2_hmac_sha512,
    pbkdf2_hmac_sha512_hex,
)


# ──────────────────────────────────────────────────────────────────────────────
# RFC 6070 test vectors — PBKDF2-HMAC-SHA1
# ──────────────────────────────────────────────────────────────────────────────

class TestRfc6070Sha1:
    """Official RFC 6070 PBKDF2-HMAC-SHA1 test vectors.

    These cover single-iteration (no XOR accumulation), multi-iteration,
    long password+salt, and null bytes inside password/salt.
    """

    def test_vector_1_c1(self) -> None:
        dk = pbkdf2_hmac_sha1(b"password", b"salt", 1, 20)
        assert dk.hex() == "0c60c80f961f0e71f3a9b524af6012062fe037a6"

    def test_vector_2_c4096(self) -> None:
        dk = pbkdf2_hmac_sha1(b"password", b"salt", 4096, 20)
        assert dk.hex() == "4b007901b765489abead49d926f721d065a429c1"

    def test_vector_3_long_password_salt(self) -> None:
        dk = pbkdf2_hmac_sha1(
            b"passwordPASSWORDpassword",
            b"saltSALTsaltSALTsaltSALTsaltSALTsalt",
            4096,
            25,
        )
        assert dk.hex() == "3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038"

    def test_vector_4_null_bytes(self) -> None:
        # Null bytes in password and salt are valid — PBKDF2 is binary-safe.
        dk = pbkdf2_hmac_sha1(b"pass\x00word", b"sa\x00lt", 4096, 16)
        assert dk.hex() == "56fa6aa75548099dcc37d7f03425e0c3"


# ──────────────────────────────────────────────────────────────────────────────
# RFC 7914 test vector — PBKDF2-HMAC-SHA256
# ──────────────────────────────────────────────────────────────────────────────

class TestRfc7914Sha256:
    """RFC 7914 Appendix B — PBKDF2-HMAC-SHA256."""

    def test_vector_1_c1_64bytes(self) -> None:
        dk = pbkdf2_hmac_sha256(b"passwd", b"salt", 1, 64)
        expected = (
            "55ac046e56e3089fec1691c22544b605"
            "f94185216dde0465e68b9d57c20dacbc"
            "49ca9cccf179b645991664b39d77ef31"
            "7c71b845b1e30bd509112041d3a19783"
        )
        assert dk.hex() == expected

    def test_sha256_output_length(self) -> None:
        dk = pbkdf2_hmac_sha256(b"key", b"salt", 1, 32)
        assert len(dk) == 32

    def test_sha256_custom_key_length(self) -> None:
        # Requesting fewer bytes than h_len truncates correctly.
        short = pbkdf2_hmac_sha256(b"key", b"salt", 1, 16)
        full = pbkdf2_hmac_sha256(b"key", b"salt", 1, 32)
        assert short == full[:16]

    def test_sha256_multi_block(self) -> None:
        # dkLen=64 for SHA-256 (h_len=32) requires 2 blocks.
        dk = pbkdf2_hmac_sha256(b"password", b"salt", 1, 64)
        assert len(dk) == 64
        # First 32 bytes must match the single-block result.
        dk_32 = pbkdf2_hmac_sha256(b"password", b"salt", 1, 32)
        assert dk[:32] == dk_32


# ──────────────────────────────────────────────────────────────────────────────
# SHA-512 sanity checks
# ──────────────────────────────────────────────────────────────────────────────

class TestSha512:
    def test_output_length(self) -> None:
        dk = pbkdf2_hmac_sha512(b"secret", b"nacl", 1, 64)
        assert len(dk) == 64

    def test_truncation(self) -> None:
        short = pbkdf2_hmac_sha512(b"secret", b"nacl", 1, 32)
        full = pbkdf2_hmac_sha512(b"secret", b"nacl", 1, 64)
        assert short == full[:32]

    def test_multi_block(self) -> None:
        # 128 bytes = 2 blocks of 64.
        dk = pbkdf2_hmac_sha512(b"key", b"salt", 1, 128)
        assert len(dk) == 128


# ──────────────────────────────────────────────────────────────────────────────
# Hex variants
# ──────────────────────────────────────────────────────────────────────────────

class TestHexVariants:
    def test_sha1_hex_matches_bytes(self) -> None:
        raw = pbkdf2_hmac_sha1(b"password", b"salt", 1, 20)
        assert pbkdf2_hmac_sha1_hex(b"password", b"salt", 1, 20) == raw.hex()

    def test_sha256_hex_rfc6070(self) -> None:
        assert pbkdf2_hmac_sha1_hex(b"password", b"salt", 1, 20) == (
            "0c60c80f961f0e71f3a9b524af6012062fe037a6"
        )

    def test_sha256_hex_matches_bytes(self) -> None:
        raw = pbkdf2_hmac_sha256(b"passwd", b"salt", 1, 32)
        assert pbkdf2_hmac_sha256_hex(b"passwd", b"salt", 1, 32) == raw.hex()

    def test_sha512_hex_matches_bytes(self) -> None:
        raw = pbkdf2_hmac_sha512(b"secret", b"nacl", 1, 64)
        assert pbkdf2_hmac_sha512_hex(b"secret", b"nacl", 1, 64) == raw.hex()


# ──────────────────────────────────────────────────────────────────────────────
# Security and validation
# ──────────────────────────────────────────────────────────────────────────────

class TestValidation:
    def test_empty_password_raises(self) -> None:
        with pytest.raises(ValueError, match="password must not be empty"):
            pbkdf2_hmac_sha256(b"", b"salt", 1, 32)

    def test_empty_password_sha1_raises(self) -> None:
        with pytest.raises(ValueError, match="password must not be empty"):
            pbkdf2_hmac_sha1(b"", b"salt", 1, 20)

    def test_zero_iterations_raises(self) -> None:
        with pytest.raises(ValueError, match="iterations must be positive"):
            pbkdf2_hmac_sha256(b"pw", b"salt", 0, 32)

    def test_negative_iterations_raises(self) -> None:
        with pytest.raises(ValueError, match="iterations must be positive"):
            pbkdf2_hmac_sha256(b"pw", b"salt", -1, 32)

    def test_zero_key_length_raises(self) -> None:
        with pytest.raises(ValueError, match="key_length must be positive"):
            pbkdf2_hmac_sha256(b"pw", b"salt", 1, 0)

    def test_empty_salt_allowed(self) -> None:
        # RFC 8018 does not forbid empty salt (though it is not recommended).
        dk = pbkdf2_hmac_sha256(b"password", b"", 1, 32)
        assert len(dk) == 32

    def test_deterministic(self) -> None:
        a = pbkdf2_hmac_sha256(b"secret", b"nacl", 100, 32)
        b = pbkdf2_hmac_sha256(b"secret", b"nacl", 100, 32)
        assert a == b

    def test_different_salts_different_output(self) -> None:
        a = pbkdf2_hmac_sha256(b"password", b"salt1", 1, 32)
        b = pbkdf2_hmac_sha256(b"password", b"salt2", 1, 32)
        assert a != b

    def test_different_passwords_different_output(self) -> None:
        a = pbkdf2_hmac_sha256(b"password1", b"salt", 1, 32)
        b = pbkdf2_hmac_sha256(b"password2", b"salt", 1, 32)
        assert a != b

    def test_different_iterations_different_output(self) -> None:
        a = pbkdf2_hmac_sha256(b"password", b"salt", 1, 32)
        b = pbkdf2_hmac_sha256(b"password", b"salt", 2, 32)
        assert a != b
