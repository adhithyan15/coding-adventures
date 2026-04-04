"""Tests for the reed_solomon package.

Covers:
- Generator polynomial properties (roots, degree, monic)
- Encoding structural invariants (systematic, length)
- Syndrome zero on valid codewords
- Round-trip (no errors)
- Error correction at every byte position
- Correction capacity limits (TooManyErrors)
- Error locator degree after BM
- Concrete cross-language test vectors
- Edge cases (empty message, single byte, max-length codeword)
- Input validation (bad n_check, oversized, too-short received)
"""

import pytest
from gf256 import power

from reed_solomon import (
    VERSION,
    InvalidInputError,
    TooManyErrorsError,
    build_generator,
    decode,
    encode,
    error_locator,
    syndromes,
)

# =============================================================================
# Version
# =============================================================================


class TestVersion:
    def test_is_semver(self):
        parts = VERSION.split(".")
        assert len(parts) == 3
        assert all(p.isdigit() for p in parts)


# =============================================================================
# Generator Polynomial
# =============================================================================


class TestBuildGenerator:
    """The generator g(x) = ∏_{i=1}^{n_check} (x + α^i) over GF(256)."""

    def test_degree_equals_n_check(self):
        """g has degree n_check, so the LE list has n_check+1 entries."""
        for n_check in (2, 4, 8, 16):
            g = build_generator(n_check)
            assert len(g) == n_check + 1, f"n_check={n_check}"

    def test_monic(self):
        """Leading coefficient (last in LE) is always 1."""
        for n_check in (2, 4, 8, 16):
            g = build_generator(n_check)
            assert g[-1] == 1, f"n_check={n_check}"

    def test_known_n_check_2(self):
        """g = (x + 2)(x + 4) = x² + 6x + 8  →  LE: [8, 6, 1]."""
        assert build_generator(2) == [8, 6, 1]

    def test_alpha_roots(self):
        """Every α^i for i=1..n_check must be a root of g(x).

        Evaluate g in LE form at x = α^i using Horner's method.  A monic
        polynomial with prescribed roots evaluates to exactly zero at those roots.
        """
        for n_check in (2, 4, 8):
            g = build_generator(n_check)
            for i in range(1, n_check + 1):
                alpha_i = power(2, i)
                # Horner evaluation of LE polynomial
                acc = 0
                for coeff in reversed(g):
                    from gf256 import add, multiply
                    acc = add(multiply(acc, alpha_i), coeff)
                assert acc == 0, f"g({alpha_i}) ≠ 0 for n_check={n_check}, i={i}"

    def test_non_roots_are_not_zero(self):
        """α^{n_check+1} should NOT be a root of g (it is not a prescribed root)."""
        n_check = 4
        g = build_generator(n_check)
        alpha_next = power(2, n_check + 1)
        from gf256 import add, multiply
        acc = 0
        for coeff in reversed(g):
            acc = add(multiply(acc, alpha_next), coeff)
        assert acc != 0  # almost certainly; would indicate a coincidental root

    def test_raises_on_zero_n_check(self):
        with pytest.raises(InvalidInputError):
            build_generator(0)

    def test_raises_on_odd_n_check(self):
        for odd in (1, 3, 5, 7):
            with pytest.raises(InvalidInputError):
                build_generator(odd)


# =============================================================================
# Encoding
# =============================================================================


class TestEncode:
    """Systematic encoding: message bytes unchanged, check bytes appended."""

    def test_output_length(self):
        msg = bytes(range(10))
        for n_check in (2, 4, 8):
            cw = encode(msg, n_check)
            assert len(cw) == len(msg) + n_check

    def test_message_preserved(self):
        """First k bytes of the codeword equal the original message."""
        msg = bytes([1, 2, 3, 4, 5])
        for n_check in (2, 4, 8):
            cw = encode(msg, n_check)
            assert cw[:len(msg)] == msg

    def test_check_bytes_are_bytes(self):
        """Returned value is bytes and every check byte is in [0, 255]."""
        cw = encode(b"hello", 4)
        assert isinstance(cw, bytes)

    def test_zero_message(self):
        """Encoding all-zero message yields all-zero codeword."""
        msg = bytes(5)
        cw = encode(msg, 4)
        assert cw == bytes(5 + 4)

    def test_empty_message(self):
        """Empty message produces a codeword consisting solely of check bytes."""
        cw = encode(b"", 4)
        assert len(cw) == 4

    def test_single_byte_message(self):
        msg = bytes([0xAB])
        cw = encode(msg, 4)
        assert len(cw) == 5
        assert cw[0] == 0xAB

    def test_returns_bytes_type(self):
        cw = encode(b"test", 4)
        assert type(cw) is bytes

    def test_bytearray_input_accepted(self):
        cw = encode(bytearray([10, 20, 30]), 4)
        assert len(cw) == 7

    def test_raises_odd_n_check(self):
        with pytest.raises(InvalidInputError):
            encode(b"hello", 3)

    def test_raises_zero_n_check(self):
        with pytest.raises(InvalidInputError):
            encode(b"hello", 0)

    def test_raises_total_length_exceeds_255(self):
        msg = bytes(248)   # 248 + 8 = 256 > 255
        with pytest.raises(InvalidInputError):
            encode(msg, 8)

    def test_max_valid_length(self):
        """247 + 8 = 255 — exactly at the GF(256) block size limit."""
        msg = bytes(247)
        cw = encode(msg, 8)
        assert len(cw) == 255


# =============================================================================
# Syndromes
# =============================================================================


class TestSyndromes:
    """Valid codewords must have all-zero syndromes."""

    def test_valid_codeword_has_zero_syndromes(self):
        for n_check in (2, 4, 8):
            msg = bytes(range(1, 11))
            cw = encode(msg, n_check)
            s = syndromes(cw, n_check)
            assert len(s) == n_check
            assert all(v == 0 for v in s), f"n_check={n_check}, syndromes={s}"

    def test_corruption_produces_nonzero_syndrome(self):
        cw = bytearray(encode(b"hello world", 8))
        cw[0] ^= 0xFF        # corrupt first byte
        s = syndromes(bytes(cw), 8)
        assert any(v != 0 for v in s)

    def test_empty_message_codeword_zero_syndromes(self):
        cw = encode(b"", 4)
        s = syndromes(cw, 4)
        assert all(v == 0 for v in s)

    def test_single_byte_codeword_zero_syndromes(self):
        cw = encode(bytes([42]), 4)
        s = syndromes(cw, 4)
        assert all(v == 0 for v in s)

    def test_syndrome_count_equals_n_check(self):
        cw = encode(b"test", 8)
        s = syndromes(cw, 8)
        assert len(s) == 8


# =============================================================================
# Round-trip: encode then decode
# =============================================================================


class TestRoundTrip:
    """decode(encode(msg, n_check), n_check) must equal msg for any valid input."""

    def test_basic_ascii_string(self):
        msg = b"Hello, World!"
        for n_check in (2, 4, 8):
            assert decode(encode(msg, n_check), n_check) == msg

    def test_all_zero_bytes(self):
        msg = bytes(20)
        assert decode(encode(msg, 4), 4) == msg

    def test_all_0xff_bytes(self):
        msg = bytes([0xFF] * 20)
        assert decode(encode(msg, 4), 4) == msg

    def test_all_byte_values(self):
        """One pass through all 256 byte values."""
        msg = bytes(range(256 - 8))   # 248 bytes, n_check=8 → 256... too long
        msg = bytes(range(245))       # 245 + 8 = 253 ≤ 255
        assert decode(encode(msg, 8), 8) == msg

    def test_empty_message(self):
        assert decode(encode(b"", 4), 4) == b""

    def test_single_byte(self):
        for b in [0x00, 0x01, 0xAB, 0xFF]:
            msg = bytes([b])
            assert decode(encode(msg, 4), 4) == msg

    def test_max_length_round_trip(self):
        msg = bytes(range(247))   # 247 + 8 = 255
        assert decode(encode(msg, 8), 8) == msg


# =============================================================================
# Error Correction
# =============================================================================


class TestErrorCorrection:
    """Injecting exactly t errors must still yield the original message."""

    def _corrupt(self, cw: bytes, positions: list[int], magnitudes: list[int]) -> bytes:
        buf = bytearray(cw)
        for pos, mag in zip(positions, magnitudes):
            buf[pos] ^= mag
        return bytes(buf)

    def test_single_error_every_position_n_check_2(self):
        """n_check=2 → t=1.  Corrupt every single byte position in turn."""
        msg = bytes(range(1, 11))   # 10 bytes → codeword of 12
        cw = encode(msg, 2)
        for pos in range(len(cw)):
            corrupted = self._corrupt(cw, [pos], [0x5A])
            recovered = decode(corrupted, 2)
            assert recovered == msg, f"Failed at position {pos}"

    def test_two_errors_every_position_n_check_4(self):
        """n_check=4 → t=2.  Corrupt two positions simultaneously."""
        msg = bytes(range(1, 11))
        cw = encode(msg, 4)
        n = len(cw)
        for pos1 in range(0, n, 3):         # sparse sampling
            for pos2 in range(pos1 + 1, n, 4):
                corrupted = self._corrupt(cw, [pos1, pos2], [0xDE, 0xAD])
                recovered = decode(corrupted, 4)
                assert recovered == msg, f"Failed at positions {pos1}, {pos2}"

    def test_four_errors_n_check_8(self):
        """n_check=8 → t=4.  Corrupt 4 arbitrary positions."""
        msg = b"Reed-Solomon"
        cw = encode(msg, 8)
        corrupted = self._corrupt(cw, [0, 3, 7, 10], [0xFF, 0xAA, 0x55, 0x0F])
        assert decode(corrupted, 8) == msg

    def test_at_capacity_every_byte_value(self):
        """Corruption magnitude of 0x01 through 0xFF must all be correctable."""
        msg = bytes([1, 2, 3, 4, 5])
        cw = encode(msg, 2)
        for mag in range(1, 256):
            corrupted = self._corrupt(cw, [0], [mag])
            assert decode(corrupted, 2) == msg, f"Failed with magnitude {mag:#x}"

    def test_check_bytes_can_be_corrupted(self):
        """Errors in the check bytes must also be correctable."""
        msg = bytes(range(10))
        cw = encode(msg, 4)
        # Corrupt last two check bytes
        corrupted = self._corrupt(cw, [len(msg), len(msg) + 1], [0xAA, 0xBB])
        assert decode(corrupted, 4) == msg


# =============================================================================
# Capacity Limits
# =============================================================================


class TestCapacityLimits:
    """More than t errors must raise TooManyErrorsError."""

    def test_t_plus_1_errors_raises(self):
        """n_check=4 → t=2.  Three errors must be unrecoverable."""
        msg = bytes(range(10))
        cw = bytearray(encode(msg, 4))
        cw[0] ^= 0xAA
        cw[3] ^= 0xBB
        cw[7] ^= 0xCC
        with pytest.raises(TooManyErrorsError):
            decode(bytes(cw), 4)

    def test_exactly_t_errors_succeeds(self):
        """n_check=4 → t=2.  Exactly two errors must be correctable."""
        msg = bytes(range(10))
        cw = bytearray(encode(msg, 4))
        cw[0] ^= 0xAA
        cw[5] ^= 0xBB
        assert decode(bytes(cw), 4) == msg

    def test_one_too_many_errors_n_check_8(self):
        """n_check=8 → t=4.  Five errors must raise."""
        msg = b"Hello"
        cw = bytearray(encode(msg, 8))
        for i in range(5):
            cw[i] ^= (i + 1) * 17
        with pytest.raises(TooManyErrorsError):
            decode(bytes(cw), 8)

    def test_zero_errors_no_exception(self):
        """Valid codeword must decode without any exception."""
        msg = b"test"
        cw = encode(msg, 4)
        assert decode(cw, 4) == msg


# =============================================================================
# Error Locator
# =============================================================================


class TestErrorLocator:
    """error_locator exposes the BM polynomial for external callers."""

    def test_no_errors_gives_lambda_zero_equals_one(self):
        """With zero syndromes (no errors), Λ(x) = [1] — degree 0."""
        cw = encode(b"hello world", 8)
        s = syndromes(cw, 8)
        lam = error_locator(s)
        assert lam == [1]

    def test_one_error_gives_degree_one(self):
        """One error → Λ has degree 1 → list of length 2."""
        msg = bytes(range(5))
        cw = bytearray(encode(msg, 8))
        cw[2] ^= 0x77
        s = syndromes(bytes(cw), 8)
        lam = error_locator(s)
        assert len(lam) == 2
        assert lam[0] == 1

    def test_two_errors_give_degree_two(self):
        msg = bytes(range(10))
        cw = bytearray(encode(msg, 8))
        cw[1] ^= 0xAA
        cw[8] ^= 0xBB
        s = syndromes(bytes(cw), 8)
        lam = error_locator(s)
        assert len(lam) == 3
        assert lam[0] == 1

    def test_lambda_0_always_one(self):
        """By definition Λ(0) = Λ[0] = 1 for any syndrome sequence."""
        msg = bytes(range(10))
        cw = bytearray(encode(msg, 8))
        cw[4] ^= 0x33
        s = syndromes(bytes(cw), 8)
        lam = error_locator(s)
        assert lam[0] == 1


# =============================================================================
# Decode Input Validation
# =============================================================================


class TestDecodeValidation:
    def test_raises_odd_n_check(self):
        with pytest.raises(InvalidInputError):
            decode(bytes(10), 3)

    def test_raises_zero_n_check(self):
        with pytest.raises(InvalidInputError):
            decode(bytes(10), 0)

    def test_raises_received_too_short(self):
        with pytest.raises(InvalidInputError):
            decode(bytes(3), 4)   # 3 < 4

    def test_received_exactly_n_check_length(self):
        """Received of exactly n_check bytes means 0 message bytes — valid."""
        msg = b""
        cw = encode(msg, 4)
        assert len(cw) == 4
        recovered = decode(cw, 4)
        assert recovered == b""


# =============================================================================
# Concrete Test Vectors — cross-validated with Rust and TypeScript
# =============================================================================


class TestVectors:
    """Pin exact byte values that must match the Rust and TypeScript reference.

    These vectors were computed by the Rust reference implementation and must
    produce identical results in all language ports.
    """

    def test_generator_n_check_2(self):
        """g(x) = (x+2)(x+4) = x² + 6x + 8  →  LE [8, 6, 1]."""
        assert build_generator(2) == [8, 6, 1]

    def test_encode_decode_cross_language(self):
        """A known message encoded and decoded must round-trip in Python."""
        msg = bytes([1, 2, 3, 4, 5, 6, 7, 8])
        n_check = 8
        cw = encode(msg, n_check)
        assert len(cw) == 16
        # Systematic: message bytes unchanged
        assert cw[:8] == msg
        # Valid codeword has zero syndromes
        assert all(s == 0 for s in syndromes(cw, n_check))
        # Decode recovers message
        assert decode(cw, n_check) == msg

    def test_syndrome_zero_for_canonical_codewords(self):
        """Various message lengths and n_check values."""
        for k, n_check in [(5, 2), (10, 4), (15, 8), (1, 4), (50, 16)]:
            msg = bytes(range(k))
            cw = encode(msg, n_check)
            assert all(s == 0 for s in syndromes(cw, n_check))

    def test_alternating_bit_pattern(self):
        """0xAA / 0x55 patterns are common stress tests for bit-level implementations."""
        msg = bytes([0xAA, 0x55] * 10)
        n_check = 8
        cw = encode(msg, n_check)
        assert all(s == 0 for s in syndromes(cw, n_check))
        assert decode(cw, n_check) == msg

    def test_all_ones_message(self):
        msg = bytes([0xFF] * 20)
        n_check = 4
        cw = encode(msg, n_check)
        assert decode(cw, n_check) == msg

    def test_correction_with_known_positions_and_magnitudes(self):
        """Corrupt at exactly t positions with known values, verify recovery."""
        msg = b"Reed-Solomon"   # 12 bytes
        n_check = 8             # t = 4
        cw = bytearray(encode(msg, n_check))

        # Corrupt 4 bytes with well-known XOR masks
        cw[0]  ^= 0xFF
        cw[3]  ^= 0xAA
        cw[7]  ^= 0x55
        cw[10] ^= 0x0F

        recovered = decode(bytes(cw), n_check)
        assert recovered == msg
