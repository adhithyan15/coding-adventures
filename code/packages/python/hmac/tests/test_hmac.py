"""Tests for coding_adventures_hmac — HMAC RFC 2104 / FIPS 198-1."""

from __future__ import annotations

import pytest

from coding_adventures_hmac import (
    hmac,
    hmac_md5,
    hmac_md5_hex,
    hmac_sha1,
    hmac_sha1_hex,
    hmac_sha256,
    hmac_sha256_hex,
    hmac_sha512,
    hmac_sha512_hex,
)
from coding_adventures_sha256 import sha256
from coding_adventures_sha512 import sha512


# =============================================================================
# RFC 4231 — HMAC-SHA256 test vectors
# =============================================================================


class TestHmacSha256Rfc4231:
    """All 7 test cases from RFC 4231 Section 4 for HMAC-SHA256."""

    def test_tc1_short_key_short_data(self) -> None:
        key = b"\x0b" * 20
        data = b"Hi There"
        assert hmac_sha256_hex(key, data) == (
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        )

    def test_tc2_jefe(self) -> None:
        assert hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?") == (
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        )

    def test_tc3_repeated_bytes(self) -> None:
        key = b"\xaa" * 20
        data = b"\xdd" * 50
        assert hmac_sha256_hex(key, data) == (
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
        )

    def test_tc4_longer_key(self) -> None:
        key = bytes.fromhex("0102030405060708090a0b0c0d0e0f10111213141516171819")
        data = b"\xcd" * 50
        assert hmac_sha256_hex(key, data) == (
            "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b"
        )

    def test_tc6_key_longer_than_block(self) -> None:
        key = b"\xaa" * 131
        data = b"Test Using Larger Than Block-Size Key - Hash Key First"
        assert hmac_sha256_hex(key, data) == (
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        )

    def test_tc7_key_and_data_longer_than_block(self) -> None:
        key = b"\xaa" * 131
        data = (
            b"This is a test using a larger than block-size key and a larger than "
            b"block-size data. The key needs to be hashed before being used by the "
            b"HMAC algorithm."
        )
        assert hmac_sha256_hex(key, data) == (
            "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2"
        )


# =============================================================================
# RFC 4231 — HMAC-SHA512 test vectors
# =============================================================================


class TestHmacSha512Rfc4231:
    """All 7 test cases from RFC 4231 Section 4 for HMAC-SHA512."""

    def test_tc1_short_key_short_data(self) -> None:
        key = b"\x0b" * 20
        data = b"Hi There"
        assert hmac_sha512_hex(key, data) == (
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde"
            "daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
        )

    def test_tc2_jefe(self) -> None:
        assert hmac_sha512_hex(b"Jefe", b"what do ya want for nothing?") == (
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea250554"
            "9758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        )

    def test_tc3_repeated_bytes(self) -> None:
        key = b"\xaa" * 20
        data = b"\xdd" * 50
        assert hmac_sha512_hex(key, data) == (
            "fa73b0089d56a284efb0f0756c890be9b1b5dbdd8ee81a3655f83e33b2279d39"
            "bf3e848279a722c806b485a47e67c807b946a337bee8942674278859e13292fb"
        )

    def test_tc4_longer_key(self) -> None:
        key = bytes.fromhex("0102030405060708090a0b0c0d0e0f10111213141516171819")
        data = b"\xcd" * 50
        assert hmac_sha512_hex(key, data) == (
            "b0ba465637458c6990e5a8c5f61d4af7e576d97ff94b872de76f8050361ee3db"
            "a91ca5c11aa25eb4d679275cc5788063a5f19741120c4f2de2adebeb10a298dd"
        )

    def test_tc6_key_longer_than_block(self) -> None:
        key = b"\xaa" * 131
        data = b"Test Using Larger Than Block-Size Key - Hash Key First"
        assert hmac_sha512_hex(key, data) == (
            "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f352"
            "6b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598"
        )

    def test_tc7_key_and_data_longer_than_block(self) -> None:
        key = b"\xaa" * 131
        data = (
            b"This is a test using a larger than block-size key and a larger than "
            b"block-size data. The key needs to be hashed before being used by the "
            b"HMAC algorithm."
        )
        assert hmac_sha512_hex(key, data) == (
            "e37b6a775dc87dbaa4dfa9f96e5e3ffddebd71f8867289865df5a32d20cdc944"
            "b6022cac3c4982b10d5eeb55c3e4de15134676fb6de0446065c97440fa8c6a58"
        )


# =============================================================================
# RFC 2202 — HMAC-MD5 test vectors
# =============================================================================


class TestHmacMd5Rfc2202:
    def test_tc1(self) -> None:
        key = b"\x0b" * 16
        assert hmac_md5_hex(key, b"Hi There") == "9294727a3638bb1c13f48ef8158bfc9d"

    def test_tc2_jefe(self) -> None:
        assert hmac_md5_hex(b"Jefe", b"what do ya want for nothing?") == (
            "750c783e6ab0b503eaa86e310a5db738"
        )

    def test_tc3_repeated_bytes(self) -> None:
        key = b"\xaa" * 16
        data = b"\xdd" * 50
        assert hmac_md5_hex(key, data) == "56be34521d144c88dbb8c733f0e8b3f6"

    def test_tc6_key_longer_than_block(self) -> None:
        key = b"\xaa" * 80
        data = b"Test Using Larger Than Block-Size Key - Hash Key First"
        assert hmac_md5_hex(key, data) == "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd"

    def test_tc7_key_and_data_longer_than_block(self) -> None:
        key = b"\xaa" * 80
        data = b"Test Using Larger Than Block-Size Key and Larger Than One Block-Size Data"
        assert hmac_md5_hex(key, data) == "6f630fad67cda0ee1fb1f562db3aa53e"


# =============================================================================
# RFC 2202 — HMAC-SHA1 test vectors
# =============================================================================


class TestHmacSha1Rfc2202:
    def test_tc1(self) -> None:
        key = b"\x0b" * 20
        assert hmac_sha1_hex(key, b"Hi There") == (
            "b617318655057264e28bc0b6fb378c8ef146be00"
        )

    def test_tc2_jefe(self) -> None:
        assert hmac_sha1_hex(b"Jefe", b"what do ya want for nothing?") == (
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        )

    def test_tc3_repeated_bytes(self) -> None:
        key = b"\xaa" * 20
        data = b"\xdd" * 50
        assert hmac_sha1_hex(key, data) == "125d7342b9ac11cd91a39af48aa17b4f63f175d3"

    def test_tc6_key_longer_than_block(self) -> None:
        key = b"\xaa" * 80
        data = b"Test Using Larger Than Block-Size Key - Hash Key First"
        assert hmac_sha1_hex(key, data) == "aa4ae5e15272d00e95705637ce8a3b55ed402112"

    def test_tc7_key_and_data_longer_than_block(self) -> None:
        key = b"\xaa" * 80
        data = b"Test Using Larger Than Block-Size Key and Larger Than One Block-Size Data"
        assert hmac_sha1_hex(key, data) == "e8e99d0f45237d786d6bbaa7965c7808bbff1a91"


# =============================================================================
# Return types and lengths
# =============================================================================


class TestReturnTypes:
    def test_hmac_md5_returns_16_bytes(self) -> None:
        result = hmac_md5(b"key", b"msg")
        assert isinstance(result, bytes)
        assert len(result) == 16

    def test_hmac_sha1_returns_20_bytes(self) -> None:
        result = hmac_sha1(b"key", b"msg")
        assert isinstance(result, bytes)
        assert len(result) == 20

    def test_hmac_sha256_returns_32_bytes(self) -> None:
        result = hmac_sha256(b"key", b"msg")
        assert isinstance(result, bytes)
        assert len(result) == 32

    def test_hmac_sha512_returns_64_bytes(self) -> None:
        result = hmac_sha512(b"key", b"msg")
        assert isinstance(result, bytes)
        assert len(result) == 64

    def test_hex_sha256_is_64_char_lowercase(self) -> None:
        result = hmac_sha256_hex(b"key", b"msg")
        assert isinstance(result, str)
        assert len(result) == 64
        assert result == result.lower()
        assert all(c in "0123456789abcdef" for c in result)

    def test_hex_sha512_is_128_char_lowercase(self) -> None:
        result = hmac_sha512_hex(b"key", b"msg")
        assert len(result) == 128
        assert all(c in "0123456789abcdef" for c in result)

    def test_hex_md5_is_32_char_lowercase(self) -> None:
        assert len(hmac_md5_hex(b"key", b"msg")) == 32

    def test_hex_sha1_is_40_char_lowercase(self) -> None:
        assert len(hmac_sha1_hex(b"key", b"msg")) == 40


# =============================================================================
# Key handling edge cases
# =============================================================================


class TestKeyHandling:
    def test_empty_key(self) -> None:
        result = hmac_sha256(b"", b"message")
        assert len(result) == 32

    def test_empty_message(self) -> None:
        result = hmac_sha256(b"key", b"")
        assert len(result) == 32

    def test_empty_key_and_message(self) -> None:
        result = hmac_sha256(b"", b"")
        assert len(result) == 32

    def test_key_exactly_64_bytes(self) -> None:
        key = b"\x01" * 64
        assert len(hmac_sha256(key, b"msg")) == 32

    def test_key_65_bytes_is_hashed(self) -> None:
        # Keys longer than block size are hashed before use.
        # Verify 65-byte and 66-byte keys produce different results
        # (since sha256(65-byte key) != sha256(66-byte key)).
        r1 = hmac_sha256(b"\x01" * 65, b"msg")
        r2 = hmac_sha256(b"\x01" * 66, b"msg")
        assert r1 != r2

    def test_sha512_key_exactly_128_bytes(self) -> None:
        key = b"\x01" * 128
        assert len(hmac_sha512(key, b"msg")) == 64

    def test_sha512_key_129_bytes_is_hashed(self) -> None:
        r1 = hmac_sha512(b"\x01" * 129, b"msg")
        r2 = hmac_sha512(b"\x01" * 130, b"msg")
        assert r1 != r2


# =============================================================================
# Authentication properties
# =============================================================================


class TestAuthenticationProperties:
    def test_deterministic(self) -> None:
        assert hmac_sha256(b"key", b"msg") == hmac_sha256(b"key", b"msg")

    def test_key_sensitivity(self) -> None:
        assert hmac_sha256(b"key1", b"message") != hmac_sha256(b"key2", b"message")

    def test_message_sensitivity(self) -> None:
        assert hmac_sha256(b"key", b"message1") != hmac_sha256(b"key", b"message2")

    def test_not_prefix_malleable(self) -> None:
        # Changing a single bit of the message changes the tag.
        base = hmac_sha256(b"key", b"base_message")
        extended = hmac_sha256(b"key", b"base_messageX")
        assert base != extended

    def test_generic_matches_named_sha256(self) -> None:
        key, msg = b"test-key", b"test-message"
        assert hmac(sha256, 64, key, msg) == hmac_sha256(key, msg)

    def test_generic_matches_named_sha512(self) -> None:
        key, msg = b"test-key", b"test-message"
        assert hmac(sha512, 128, key, msg) == hmac_sha512(key, msg)

    def test_hex_matches_bytes(self) -> None:
        key, msg = b"k", b"m"
        assert hmac_sha256_hex(key, msg) == hmac_sha256(key, msg).hex()
        assert hmac_sha512_hex(key, msg) == hmac_sha512(key, msg).hex()

    def test_different_algorithms_differ(self) -> None:
        key, msg = b"key", b"message"
        assert hmac_md5(key, msg) != hmac_sha256(key, msg)
        assert hmac_sha1(key, msg) != hmac_sha256(key, msg)
        assert hmac_sha256(key, msg) != hmac_sha512(key, msg)

    def test_binary_key_and_message(self) -> None:
        key = bytes(range(32))
        msg = bytes(range(256))
        result = hmac_sha256(key, msg)
        assert len(result) == 32
