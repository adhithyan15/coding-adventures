"""Tests for coding_adventures_hkdf — RFC 5869 test vectors and edge cases."""

from __future__ import annotations

import pytest

from coding_adventures_hkdf import hkdf, hkdf_expand, hkdf_extract


# =============================================================================
# RFC 5869 Appendix A — Test Vectors
# =============================================================================


class TestRFC5869Vectors:
    """All three test cases from RFC 5869 Appendix A (SHA-256)."""

    # ── Test Case 1: Basic SHA-256 ──────────────────────────────────────────

    def test_case_1_extract(self) -> None:
        """TC1: Extract with 22-byte IKM and 13-byte salt."""
        ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        salt = bytes.fromhex("000102030405060708090a0b0c")
        prk = hkdf_extract(salt, ikm, hash="sha256")
        assert prk.hex() == (
            "077709362c2e32df0ddc3f0dc47bba63"
            "90b6c73bb50f9c3122ec844ad7c2b3e5"
        )

    def test_case_1_expand(self) -> None:
        """TC1: Expand to 42 bytes with 10-byte info."""
        prk = bytes.fromhex(
            "077709362c2e32df0ddc3f0dc47bba63"
            "90b6c73bb50f9c3122ec844ad7c2b3e5"
        )
        info = bytes.fromhex("f0f1f2f3f4f5f6f7f8f9")
        okm = hkdf_expand(prk, info, 42, hash="sha256")
        assert okm.hex() == (
            "3cb25f25faacd57a90434f64d0362f2a"
            "2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
            "34007208d5b887185865"
        )

    def test_case_1_combined(self) -> None:
        """TC1: Full extract-then-expand."""
        ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        salt = bytes.fromhex("000102030405060708090a0b0c")
        info = bytes.fromhex("f0f1f2f3f4f5f6f7f8f9")
        okm = hkdf(salt, ikm, info, 42, hash="sha256")
        assert okm.hex() == (
            "3cb25f25faacd57a90434f64d0362f2a"
            "2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
            "34007208d5b887185865"
        )

    # ── Test Case 2: Longer inputs ──────────────────────────────────────────

    def test_case_2_extract(self) -> None:
        """TC2: Extract with 80-byte IKM and 80-byte salt."""
        ikm = bytes(range(0x00, 0x50))   # 80 bytes: 0x00..0x4f
        salt = bytes(range(0x60, 0xB0))  # 80 bytes: 0x60..0xaf
        prk = hkdf_extract(salt, ikm, hash="sha256")
        assert prk.hex() == (
            "06a6b88c5853361a06104c9ceb35b45c"
            "ef760014904671014a193f40c15fc244"
        )

    def test_case_2_expand(self) -> None:
        """TC2: Expand to 82 bytes with 80-byte info."""
        prk = bytes.fromhex(
            "06a6b88c5853361a06104c9ceb35b45c"
            "ef760014904671014a193f40c15fc244"
        )
        info = bytes(range(0xB0, 0x100))  # 80 bytes: 0xb0..0xff
        okm = hkdf_expand(prk, info, 82, hash="sha256")
        assert okm.hex() == (
            "b11e398dc80327a1c8e7f78c596a4934"
            "4f012eda2d4efad8a050cc4c19afa97c"
            "59045a99cac7827271cb41c65e590e09"
            "da3275600c2f09b8367793a9aca3db71"
            "cc30c58179ec3e87c14c01d5c1f3434f"
            "1d87"
        )

    def test_case_2_combined(self) -> None:
        """TC2: Full extract-then-expand with long inputs."""
        ikm = bytes(range(0x00, 0x50))
        salt = bytes(range(0x60, 0xB0))
        info = bytes(range(0xB0, 0x100))
        okm = hkdf(salt, ikm, info, 82, hash="sha256")
        assert okm.hex() == (
            "b11e398dc80327a1c8e7f78c596a4934"
            "4f012eda2d4efad8a050cc4c19afa97c"
            "59045a99cac7827271cb41c65e590e09"
            "da3275600c2f09b8367793a9aca3db71"
            "cc30c58179ec3e87c14c01d5c1f3434f"
            "1d87"
        )

    # ── Test Case 3: Empty salt and info ────────────────────────────────────

    def test_case_3_extract(self) -> None:
        """TC3: Extract with empty salt (uses HashLen zero bytes)."""
        ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        prk = hkdf_extract(b"", ikm, hash="sha256")
        assert prk.hex() == (
            "19ef24a32c717b167f33a91d6f648bdf"
            "96596776afdb6377ac434c1c293ccb04"
        )

    def test_case_3_expand(self) -> None:
        """TC3: Expand with empty info."""
        prk = bytes.fromhex(
            "19ef24a32c717b167f33a91d6f648bdf"
            "96596776afdb6377ac434c1c293ccb04"
        )
        okm = hkdf_expand(prk, b"", 42, hash="sha256")
        assert okm.hex() == (
            "8da4e775a563c18f715f802a063c5a31"
            "b8a11f5c5ee1879ec3454e5f3c738d2d"
            "9d201395faa4b61a96c8"
        )

    def test_case_3_combined(self) -> None:
        """TC3: Full extract-then-expand with empty salt and info."""
        ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        okm = hkdf(b"", ikm, b"", 42, hash="sha256")
        assert okm.hex() == (
            "8da4e775a563c18f715f802a063c5a31"
            "b8a11f5c5ee1879ec3454e5f3c738d2d"
            "9d201395faa4b61a96c8"
        )


# =============================================================================
# Edge Cases
# =============================================================================


class TestEdgeCases:
    """Edge cases beyond the RFC test vectors."""

    def test_expand_length_exactly_hash_len(self) -> None:
        """When L = HashLen, only one HMAC block is needed (N = 1)."""
        prk = bytes.fromhex(
            "077709362c2e32df0ddc3f0dc47bba63"
            "90b6c73bb50f9c3122ec844ad7c2b3e5"
        )
        okm = hkdf_expand(prk, b"test", 32, hash="sha256")
        assert len(okm) == 32

    def test_expand_length_one_byte(self) -> None:
        """Minimum valid output: 1 byte."""
        prk = b"\x01" * 32
        okm = hkdf_expand(prk, b"", 1, hash="sha256")
        assert len(okm) == 1

    def test_expand_max_length_sha256(self) -> None:
        """Maximum valid output: 255 * 32 = 8160 bytes for SHA-256."""
        prk = b"\x01" * 32
        okm = hkdf_expand(prk, b"", 255 * 32, hash="sha256")
        assert len(okm) == 8160

    def test_expand_exceeds_max_length(self) -> None:
        """Output length > 255 * HashLen must raise ValueError."""
        prk = b"\x01" * 32
        with pytest.raises(ValueError, match="exceeds maximum"):
            hkdf_expand(prk, b"", 255 * 32 + 1, hash="sha256")

    def test_expand_zero_length(self) -> None:
        """Zero output length must raise ValueError."""
        prk = b"\x01" * 32
        with pytest.raises(ValueError, match="must be positive"):
            hkdf_expand(prk, b"", 0, hash="sha256")

    def test_expand_negative_length(self) -> None:
        """Negative output length must raise ValueError."""
        prk = b"\x01" * 32
        with pytest.raises(ValueError, match="must be positive"):
            hkdf_expand(prk, b"", -1, hash="sha256")

    def test_unsupported_hash(self) -> None:
        """Unsupported hash algorithm must raise ValueError."""
        with pytest.raises(ValueError, match="Unsupported hash"):
            hkdf_extract(b"salt", b"ikm", hash="md5")

    def test_unsupported_hash_expand(self) -> None:
        """Unsupported hash in expand must raise ValueError."""
        with pytest.raises(ValueError, match="Unsupported hash"):
            hkdf_expand(b"\x01" * 32, b"", 32, hash="sha384")

    def test_sha512_basic(self) -> None:
        """SHA-512 variant produces 64-byte PRK and correct-length OKM."""
        ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        salt = bytes.fromhex("000102030405060708090a0b0c")
        prk = hkdf_extract(salt, ikm, hash="sha512")
        assert len(prk) == 64
        okm = hkdf_expand(prk, b"info", 64, hash="sha512")
        assert len(okm) == 64

    def test_sha512_empty_salt(self) -> None:
        """SHA-512 with empty salt uses 64 zero bytes as the HMAC key."""
        ikm = b"\xab" * 32
        prk = hkdf_extract(b"", ikm, hash="sha512")
        assert len(prk) == 64

    def test_sha512_max_length(self) -> None:
        """SHA-512 maximum output: 255 * 64 = 16320 bytes."""
        prk = b"\x01" * 64
        okm = hkdf_expand(prk, b"", 255 * 64, hash="sha512")
        assert len(okm) == 16320

    def test_sha512_exceeds_max(self) -> None:
        """SHA-512: 255 * 64 + 1 must raise ValueError."""
        prk = b"\x01" * 64
        with pytest.raises(ValueError, match="exceeds maximum"):
            hkdf_expand(prk, b"", 255 * 64 + 1, hash="sha512")

    def test_different_info_produces_different_okm(self) -> None:
        """The info parameter acts as a domain separator."""
        prk = b"\x01" * 32
        okm1 = hkdf_expand(prk, b"purpose-a", 32)
        okm2 = hkdf_expand(prk, b"purpose-b", 32)
        assert okm1 != okm2

    def test_different_salt_produces_different_prk(self) -> None:
        """Different salts produce different PRKs from the same IKM."""
        ikm = b"\x01" * 32
        prk1 = hkdf_extract(b"salt-1", ikm)
        prk2 = hkdf_extract(b"salt-2", ikm)
        assert prk1 != prk2

    def test_deterministic(self) -> None:
        """Same inputs always produce the same output."""
        okm1 = hkdf(b"salt", b"ikm", b"info", 42)
        okm2 = hkdf(b"salt", b"ikm", b"info", 42)
        assert okm1 == okm2

    def test_round_trip_extract_expand(self) -> None:
        """Combined hkdf() equals manual extract then expand."""
        salt = b"my-salt"
        ikm = b"my-input-keying-material"
        info = b"my-context"
        length = 48

        combined = hkdf(salt, ikm, info, length)
        prk = hkdf_extract(salt, ikm)
        manual = hkdf_expand(prk, info, length)
        assert combined == manual

    def test_default_hash_is_sha256(self) -> None:
        """Omitting the hash parameter defaults to SHA-256."""
        ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        salt = bytes.fromhex("000102030405060708090a0b0c")
        # Should match TC1 PRK
        prk = hkdf_extract(salt, ikm)
        assert prk.hex() == (
            "077709362c2e32df0ddc3f0dc47bba63"
            "90b6c73bb50f9c3122ec844ad7c2b3e5"
        )
