"""Tests for the from-scratch BLAKE2b implementation.

The test vectors are cross-validated against Python's ``hashlib.blake2b``,
which wraps the reference implementation.  This is our authoritative
oracle: if our output matches stdlib across the cases below, our
implementation of RFC 7693 is correct.
"""

from __future__ import annotations

import hashlib

import pytest

from coding_adventures_blake2b import Blake2bHasher, blake2b, blake2b_hex

# ----------------------------------------------------------------------
# Core vectors -- fixed expected values checked into the test suite
# ----------------------------------------------------------------------


def test_empty_message_default_output() -> None:
    """Canonical BLAKE2b empty-input digest."""
    expected = (
        "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419"
        "d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
    )
    assert blake2b_hex(b"") == expected


def test_abc() -> None:
    """Canonical BLAKE2b 'abc' digest."""
    expected = (
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1"
        "7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
    )
    assert blake2b_hex(b"abc") == expected


def test_quick_brown_fox() -> None:
    """A common cross-library sanity vector."""
    expected = (
        "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673"
        "f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"
    )
    msg = b"The quick brown fox jumps over the lazy dog"
    assert blake2b_hex(msg) == expected


def test_truncated_digest() -> None:
    """digest_size=32 truncates the internal 64-byte state."""
    expected = (
        "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
    )
    assert blake2b_hex(b"", digest_size=32) == expected


def test_keyed_long_vector() -> None:
    """Keyed BLAKE2b across a 256-byte message with a 64-byte key."""
    key = bytes(range(1, 65))
    data = bytes(range(256))
    expected = (
        "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927"
        "ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3"
    )
    assert blake2b(data, key=key).hex() == expected


# ----------------------------------------------------------------------
# Cross-validation against hashlib.blake2b across message-length boundaries
# ----------------------------------------------------------------------


@pytest.mark.parametrize(
    "size",
    [0, 1, 63, 64, 65, 127, 128, 129, 255, 256, 257, 1024, 4096, 9999],
)
def test_matches_stdlib_across_sizes(size: int) -> None:
    """Match stdlib at every length near block boundaries plus large sizes.

    Block size is 128 bytes, so ``127``, ``128``, and ``129`` are the
    critical lengths that exercise the final-block flagging rule.
    """
    data = bytes((i * 7 + 3) & 0xFF for i in range(size))
    assert blake2b(data).hex() == hashlib.blake2b(data).hexdigest()


@pytest.mark.parametrize("digest_size", [1, 16, 20, 32, 48, 64])
def test_matches_stdlib_various_digest_sizes(digest_size: int) -> None:
    data = b"The quick brown fox jumps over the lazy dog"
    ours = blake2b(data, digest_size=digest_size)
    theirs = hashlib.blake2b(data, digest_size=digest_size).digest()
    assert ours == theirs
    assert len(ours) == digest_size


@pytest.mark.parametrize("key_len", [1, 16, 32, 64])
def test_matches_stdlib_keyed(key_len: int) -> None:
    key = bytes(range(1, key_len + 1))
    data = b"secret message body"
    ours = blake2b(data, key=key, digest_size=32)
    theirs = hashlib.blake2b(data, key=key, digest_size=32).digest()
    assert ours == theirs


def test_matches_stdlib_salt_and_personal() -> None:
    salt = bytes(range(16))
    personal = bytes(range(16, 32))
    data = b"parameterized hash"
    ours = blake2b(data, salt=salt, personal=personal)
    theirs = hashlib.blake2b(data, salt=salt, person=personal).digest()
    assert ours == theirs


# ----------------------------------------------------------------------
# Streaming behavior
# ----------------------------------------------------------------------


def test_streaming_matches_one_shot_single_chunk() -> None:
    h = Blake2bHasher()
    h.update(b"hello world")
    assert h.digest() == blake2b(b"hello world")


def test_streaming_matches_one_shot_many_chunks() -> None:
    """Feeding bytes one at a time must match the one-shot digest."""
    data = bytes(range(200))
    h = Blake2bHasher(digest_size=32)
    for b in data:
        h.update(bytes([b]))
    assert h.digest() == blake2b(data, digest_size=32)


def test_streaming_across_block_boundary() -> None:
    """Feed 127 bytes, then 2 bytes -- spans the 128-byte block boundary."""
    data = bytes(range(129))
    h = Blake2bHasher()
    h.update(data[:127])
    h.update(data[127:])
    assert h.digest() == blake2b(data)


def test_streaming_exact_block_then_more() -> None:
    """Update with exactly 128 bytes, then more -- the tricky final-flag case."""
    data = bytes(range(128)) + b"tail"
    h = Blake2bHasher()
    h.update(data[:128])
    h.update(data[128:])
    assert h.digest() == blake2b(data)


def test_digest_is_idempotent() -> None:
    """Calling digest() multiple times returns the same value."""
    h = Blake2bHasher()
    h.update(b"hello")
    first = h.digest()
    second = h.digest()
    assert first == second


def test_update_after_digest() -> None:
    """The hasher stays usable after digest() is called."""
    h = Blake2bHasher(digest_size=32)
    h.update(b"hello ")
    _ = h.digest()
    h.update(b"world")
    assert h.digest() == blake2b(b"hello world", digest_size=32)


def test_copy_is_independent() -> None:
    """copy() must produce a fully independent hasher."""
    h = Blake2bHasher()
    h.update(b"prefix ")
    clone = h.copy()
    h.update(b"path A")
    clone.update(b"path B")
    assert h.digest() == blake2b(b"prefix path A")
    assert clone.digest() == blake2b(b"prefix path B")


def test_update_returns_self_for_chaining() -> None:
    h = Blake2bHasher(digest_size=16)
    result = h.update(b"a").update(b"b").update(b"c")
    assert result is h
    assert h.digest() == blake2b(b"abc", digest_size=16)


# ----------------------------------------------------------------------
# Argument validation
# ----------------------------------------------------------------------


@pytest.mark.parametrize("bad", [0, -1, 65, 100])
def test_invalid_digest_size(bad: int) -> None:
    with pytest.raises(ValueError, match="digest_size"):
        blake2b(b"", digest_size=bad)


def test_key_too_long() -> None:
    with pytest.raises(ValueError, match="key length"):
        blake2b(b"", key=b"k" * 65)


def test_salt_wrong_length() -> None:
    with pytest.raises(ValueError, match="salt"):
        blake2b(b"", salt=b"short")


def test_personal_wrong_length() -> None:
    with pytest.raises(ValueError, match="personal"):
        blake2b(b"", personal=b"too-short")


def test_max_length_key_accepted() -> None:
    """A 64-byte key is the maximum allowed."""
    key = b"k" * 64
    ours = blake2b(b"x", key=key)
    theirs = hashlib.blake2b(b"x", key=key).digest()
    assert ours == theirs


# ----------------------------------------------------------------------
# hex_digest convenience
# ----------------------------------------------------------------------


def test_hex_digest_matches_digest_hex() -> None:
    h = Blake2bHasher()
    h.update(b"hex check")
    assert h.hex_digest() == h.digest().hex()


def test_blake2b_hex_wrapper() -> None:
    assert blake2b_hex(b"abc") == blake2b(b"abc").hex()
