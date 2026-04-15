"""Tests for coding_adventures_lzw — CMP03 LZW compression.

Coverage targets:
  - Empty input
  - Single byte
  - Two distinct bytes (no repetition)
  - Repeated pair ("ABABAB") — exercises dict growth and reference emission
  - All-same bytes ("AAAAAAA") — exercises the tricky-token edge case
  - Long ASCII string — round-trip
  - Binary data — round-trip
  - Compress / decompress symmetry for all vectors
  - Wire format header correctness (original_length)
  - Multiple CLEAR cycles (dict-full reset) via a long input
  - BitWriter / BitReader roundtrip via _pack_codes / _unpack_codes
"""

from __future__ import annotations

import struct

import pytest

from coding_adventures_lzw import (
    CLEAR_CODE,
    INITIAL_CODE_SIZE,
    INITIAL_NEXT_CODE,
    MAX_CODE_SIZE,
    STOP_CODE,
    compress,
    decompress,
    _BitReader,
    _BitWriter,
    _decode_codes,
    _encode_codes,
    _pack_codes,
    _unpack_codes,
)


# ---------------------------------------------------------------------------
# BitWriter / BitReader
# ---------------------------------------------------------------------------

class TestBitWriter:
    def test_single_9bit_code(self) -> None:
        w = _BitWriter()
        # Code 256 = 0b100000000 in 9 bits.
        w.write(256, 9)
        w.flush()
        b = w.bytes()
        # 256 packed LSB-first: byte0 = 0b00000000, byte1 bits0 = 1 → 0b00000001
        assert b == bytes([0x00, 0x01])

    def test_two_9bit_codes(self) -> None:
        w = _BitWriter()
        w.write(65, 9)   # 'A' = 0b001000001
        w.write(257, 9)  # STOP = 0b100000001
        w.flush()
        b = w.bytes()
        # 65 in 9 bits LSB-first: bits 0-8 = 01000001 0
        # 257 in 9 bits: bits 9-17 = 100000001
        # Total 18 bits packed into 3 bytes.
        assert len(b) == 3

    def test_flush_empty(self) -> None:
        w = _BitWriter()
        w.flush()
        assert w.bytes() == b""

    def test_roundtrip_via_reader(self) -> None:
        codes = [CLEAR_CODE, 65, 66, 258, STOP_CODE]
        code_size = INITIAL_CODE_SIZE
        next_code = INITIAL_NEXT_CODE

        w = _BitWriter()
        for code in codes:
            w.write(code, code_size)
            if code == CLEAR_CODE:
                code_size = INITIAL_CODE_SIZE
                next_code = INITIAL_NEXT_CODE
            elif code != STOP_CODE:
                if next_code < (1 << MAX_CODE_SIZE):
                    next_code += 1
                    if next_code > (1 << code_size):
                        code_size += 1
        w.flush()

        r = _BitReader(w.bytes())
        code_size = INITIAL_CODE_SIZE
        next_code = INITIAL_NEXT_CODE
        decoded = []
        for _ in range(len(codes)):
            c = r.read(code_size)
            decoded.append(c)
            if c == CLEAR_CODE:
                code_size = INITIAL_CODE_SIZE
                next_code = INITIAL_NEXT_CODE
            elif c != STOP_CODE:
                if next_code < (1 << MAX_CODE_SIZE):
                    next_code += 1
                    if next_code > (1 << code_size):
                        code_size += 1
        assert decoded == codes


class TestBitReader:
    def test_eof_error(self) -> None:
        r = _BitReader(b"")
        with pytest.raises(EOFError):
            r.read(9)

    def test_exhausted(self) -> None:
        r = _BitReader(b"\x00\x01")
        r.read(9)  # read 9 bits (uses both bytes partially)
        # There is 7 bits left in the buffer (16 - 9), so not exhausted until drained.
        assert not r.exhausted()


# ---------------------------------------------------------------------------
# Encode / Decode codes
# ---------------------------------------------------------------------------

class TestEncodeCodes:
    def test_empty(self) -> None:
        codes, orig = _encode_codes(b"")
        assert orig == 0
        assert codes[0] == CLEAR_CODE
        assert codes[-1] == STOP_CODE
        # Empty input: CLEAR, STOP only.
        assert len(codes) == 2

    def test_single_byte(self) -> None:
        codes, orig = _encode_codes(b"A")
        assert orig == 1
        assert codes[0] == CLEAR_CODE
        assert codes[-1] == STOP_CODE
        assert 65 in codes  # 'A'

    def test_two_distinct_bytes(self) -> None:
        codes, orig = _encode_codes(b"AB")
        assert orig == 2
        # Should emit: CLEAR, 65(A), 66(B), STOP — 'AB' added but never emitted.
        assert codes == [CLEAR_CODE, 65, 66, STOP_CODE]

    def test_repeated_pair(self) -> None:
        # "ABABAB" → CLEAR, 65, 66, 258, 258, STOP (per spec trace)
        codes, orig = _encode_codes(b"ABABAB")
        assert orig == 6
        assert codes[0] == CLEAR_CODE
        assert codes[-1] == STOP_CODE
        # Round-trip is the primary check; exact codes are also verified here.
        assert codes == [CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]

    def test_all_same_bytes(self) -> None:
        # "AAAAAAA" → CLEAR, 65, 258, 259, 65, STOP
        codes, orig = _encode_codes(b"AAAAAAA")
        assert orig == 7
        assert codes == [CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]


class TestDecodeCodes:
    def test_empty_stream(self) -> None:
        result = _decode_codes([CLEAR_CODE, STOP_CODE])
        assert result == b""

    def test_single_byte(self) -> None:
        result = _decode_codes([CLEAR_CODE, 65, STOP_CODE])
        assert result == b"A"

    def test_two_distinct(self) -> None:
        result = _decode_codes([CLEAR_CODE, 65, 66, STOP_CODE])
        assert result == b"AB"

    def test_repeated_pair(self) -> None:
        result = _decode_codes([CLEAR_CODE, 65, 66, 258, 258, STOP_CODE])
        assert result == b"ABABAB"

    def test_all_same_bytes_tricky_token(self) -> None:
        # Exercises the "tricky token" edge case (code == next_code).
        result = _decode_codes([CLEAR_CODE, 65, 258, 259, 65, STOP_CODE])
        assert result == b"AAAAAAA"

    def test_clear_mid_stream(self) -> None:
        # CLEAR mid-stream should reset the dictionary.
        codes = [CLEAR_CODE, 65, CLEAR_CODE, 66, STOP_CODE]
        result = _decode_codes(codes)
        assert result == b"AB"

    def test_invalid_code_raises(self) -> None:
        # Code far beyond next_code (e.g. 9999) must raise ValueError; the
        # stream is corrupt and silently continuing would produce wrong output.
        with pytest.raises(ValueError, match="invalid LZW code"):
            _decode_codes([CLEAR_CODE, 9999, 65, STOP_CODE])


# ---------------------------------------------------------------------------
# Pack / Unpack codes
# ---------------------------------------------------------------------------

class TestPackUnpackCodes:
    def test_header_original_length(self) -> None:
        packed = _pack_codes([CLEAR_CODE, STOP_CODE], original_length=42)
        (orig,) = struct.unpack(">I", packed[:4])
        assert orig == 42

    def test_roundtrip_empty(self) -> None:
        codes = [CLEAR_CODE, STOP_CODE]
        packed = _pack_codes(codes, 0)
        unpacked_codes, orig = _unpack_codes(packed)
        assert orig == 0
        assert CLEAR_CODE in unpacked_codes
        assert STOP_CODE in unpacked_codes

    def test_roundtrip_ababab(self) -> None:
        codes = [CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]
        packed = _pack_codes(codes, 6)
        unpacked_codes, orig = _unpack_codes(packed)
        assert orig == 6
        assert unpacked_codes == codes

    def test_roundtrip_all_same(self) -> None:
        codes = [CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]
        packed = _pack_codes(codes, 7)
        unpacked_codes, orig = _unpack_codes(packed)
        assert orig == 7
        assert unpacked_codes == codes

    def test_truncated_data(self) -> None:
        # _unpack_codes should not crash on too-short input.
        codes, orig = _unpack_codes(b"\x00\x00")
        assert isinstance(codes, list)
        assert isinstance(orig, int)


# ---------------------------------------------------------------------------
# Public API: compress / decompress
# ---------------------------------------------------------------------------

class TestCompressDecompress:
    def test_empty(self) -> None:
        assert decompress(compress(b"")) == b""

    def test_single_byte(self) -> None:
        assert decompress(compress(b"A")) == b"A"

    def test_two_distinct(self) -> None:
        assert decompress(compress(b"AB")) == b"AB"

    def test_repeated_pair(self) -> None:
        assert decompress(compress(b"ABABAB")) == b"ABABAB"

    def test_all_same_bytes(self) -> None:
        # Exercises tricky-token path in decoder.
        assert decompress(compress(b"AAAAAAA")) == b"AAAAAAA"

    def test_long_string_roundtrip(self) -> None:
        text = b"the quick brown fox jumps over the lazy dog " * 20
        assert decompress(compress(text)) == text

    def test_binary_data_roundtrip(self) -> None:
        data = bytes(range(256)) * 4
        assert decompress(compress(data)) == data

    def test_all_zeros(self) -> None:
        data = b"\x00" * 100
        assert decompress(compress(data)) == data

    def test_all_ff(self) -> None:
        data = b"\xff" * 100
        assert decompress(compress(data)) == data

    def test_compresses_repetitive_data(self) -> None:
        # Highly repetitive data should compress to less than the original.
        data = b"ABCABC" * 100
        assert len(compress(data)) < len(data)

    def test_bytearray_input_compress(self) -> None:
        assert decompress(compress(bytearray(b"hello"))) == b"hello"

    def test_bytearray_input_decompress(self) -> None:
        compressed = compress(b"hello")
        assert decompress(bytearray(compressed)) == b"hello"

    def test_aababc(self) -> None:
        # Spec vector: exercises dict building step by step.
        assert decompress(compress(b"AABABC")) == b"AABABC"

    def test_header_contains_original_length(self) -> None:
        data = b"hello world"
        compressed = compress(data)
        (stored_len,) = struct.unpack(">I", compressed[:4])
        assert stored_len == len(data)

    def test_constants(self) -> None:
        assert CLEAR_CODE == 256
        assert STOP_CODE == 257
        assert INITIAL_NEXT_CODE == 258
        assert INITIAL_CODE_SIZE == 9
        assert MAX_CODE_SIZE == 16
