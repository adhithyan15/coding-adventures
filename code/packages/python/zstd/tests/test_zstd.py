"""Tests for the coding_adventures_zstd package.

Tests cover:
  - Round-trip correctness for various data shapes
  - RLE and multi-block behaviour
  - Error handling (bad magic, truncation, unsupported modes)
  - Compression ratio sanity checks
  - Internal helper correctness (FSE tables, bit I/O, literals section, etc.)
"""

from __future__ import annotations

import pytest

from coding_adventures_zstd import (
    LL_ACC_LOG,
    LL_CODES,
    LL_NORM,
    MAGIC,
    ML_ACC_LOG,
    ML_CODES,
    ML_NORM,
    OF_ACC_LOG,
    OF_NORM,
    _build_decode_table,
    _decode_literals_section,
    _decode_seq_count,
    _encode_literals_section,
    _encode_seq_count,
    _encode_sequences_section,
    _fse_decode_sym,
    _ll_to_code,
    _ml_to_code,
    _RevBitReader,
    _RevBitWriter,
    _Seq,
    compress,
    decompress,
)

# =============================================================================
# Helper
# =============================================================================


def rt(data: bytes) -> bytes:
    """Compress then decompress data, returning the result."""
    return decompress(compress(data))


# =============================================================================
# TC-1: Empty input round-trip
# =============================================================================


def test_tc1_empty() -> None:
    """An empty input produces a valid ZStd frame and round-trips to empty bytes."""
    compressed = compress(b"")
    result = decompress(compressed)
    assert result == b""
    # Frame must still have the magic bytes and a valid header.
    assert compressed[:4] == MAGIC.to_bytes(4, "little")


# =============================================================================
# TC-2: Single byte round-trip
# =============================================================================


def test_tc2_single_byte() -> None:
    """The smallest non-empty input (one byte) round-trips correctly."""
    for byte_val in [0x00, 0x42, 0xFF]:
        data = bytes([byte_val])
        assert rt(data) == data, f"failed for byte {byte_val:#04x}"


# =============================================================================
# TC-3: All 256 byte values round-trip
# =============================================================================


def test_tc3_all_bytes() -> None:
    """Every possible byte value 0x00-0xFF in order round-trips correctly.

    This exercises literal encoding of non-ASCII and zero bytes.
    """
    data = bytes(range(256))
    assert rt(data) == data


# =============================================================================
# TC-4: RLE block compression ratio
# =============================================================================


def test_tc4_rle_block() -> None:
    """1024 identical bytes should be detected as an RLE block.

    Expected compressed size:
      4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header) + 1 (RLE byte) = 17 bytes.
    """
    data = bytes([0x41]) * 1024  # 'A' * 1024
    compressed = compress(data)

    # Must round-trip correctly.
    assert decompress(compressed) == data

    # Must be well under 30 bytes (RLE encoding).
    assert len(compressed) < 30, (
        f"RLE of 1024 bytes compressed to {len(compressed)}, expected < 30"
    )


# =============================================================================
# TC-5: English prose compression ratio
# =============================================================================


def test_tc5_prose_compression() -> None:
    """Repeated English text must achieve >= 20% compression (output <= 80% input).

    Repeated text has strong LZ77 back-reference opportunities.
    """
    text = "the quick brown fox jumps over the lazy dog " * 25
    data = text.encode("ascii")
    compressed = compress(data)

    assert decompress(compressed) == data

    threshold = len(data) * 80 // 100
    assert len(compressed) < threshold, (
        f"prose: compressed {len(compressed)} bytes "
        f"(input {len(data)}), expected < {threshold} (80%)"
    )


# =============================================================================
# TC-6: Pseudo-random data (LCG) round-trip
# =============================================================================


def test_tc6_pseudo_random() -> None:
    """LCG pseudo-random bytes round-trip correctly regardless of block type.

    Random data has little structure, so LZ77 finds few matches.
    The compressor should fall back to raw blocks and still round-trip exactly.
    """
    seed = 42
    data = bytearray(512)
    for i in range(512):
        seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF
        data[i] = seed & 0xFF

    assert rt(bytes(data)) == bytes(data)


# =============================================================================
# TC-7: 200 KB single-byte run (multiple RLE blocks)
# =============================================================================


def test_tc7_multiblock_rle() -> None:
    """200 KB of identical bytes spans two 128 KB blocks; both should be RLE.

    Tests that the multi-block splitting logic works and produces correct output.
    """
    data = bytes([0x78]) * (200 * 1024)  # 200 KB of 'x'
    assert rt(data) == data


# =============================================================================
# TC-8: 300 KB repetitive text (multiple compressed blocks)
# =============================================================================


def test_tc8_multiblock_compressed() -> None:
    """300 KB of repetitive text exercises multi-block compressed output.

    Tests both block splitting and compressed block round-trips for large inputs.
    """
    # Mix of patterns that LZ77 can compress well but not RLE.
    unit = b"ABCDEFGHIJ" * 50 + b"ZYXWVUTSRQ" * 50
    data = unit * (300 * 1024 // len(unit) + 1)
    data = data[: 300 * 1024]  # exactly 300 KB

    result = rt(data)
    assert result == data


# =============================================================================
# TC-9: Bad magic -> ValueError
# =============================================================================


def test_tc9_bad_magic() -> None:
    """A frame with a wrong magic number must raise an exception."""
    bad_frame = b"\x00\x00\x00\x00" + b"\xE0" + b"\x00" * 12
    with pytest.raises((ValueError, Exception)):
        decompress(bad_frame)


# =============================================================================
# TC-10: Truncated input -> exception
# =============================================================================


def test_tc10_truncated_input() -> None:
    """Various truncated frames must raise exceptions (not silently succeed)."""
    # Truncated at different points.
    compressed = compress(b"hello world " * 20)
    for trunc_len in [1, 4, 5, 10, len(compressed) // 2]:
        with pytest.raises((ValueError, Exception)):
            decompress(compressed[:trunc_len])


# =============================================================================
# TC-11: RLE block decompression from raw frame bytes
# =============================================================================


def test_tc11_rle_from_raw_frame() -> None:
    """Manually construct a ZStd frame with a known RLE block and decompress it.

    This tests the decoder independently from the encoder's path.

    Frame layout:
      [0..3]  Magic = 0xFD2FB528 LE
      [4]     FHD = 0xE0 (Single_Segment=1, FCS=8 bytes)
      [5..12] FCS = 10 (u64 LE)
      [13..15] Block header: Last=1, Type=RLE(01), Size=10
               = (10 << 3) | (0b01 << 1) | 1 = 83 = 0x53 -> [0x53, 0x00, 0x00]
      [16]    RLE byte = 0xAA
    """
    rle_size = 10
    rle_byte = 0xAA

    frame = bytearray()
    frame.extend(MAGIC.to_bytes(4, "little"))    # magic
    frame.append(0xE0)                            # FHD: single-seg, 8-byte FCS
    frame.extend(rle_size.to_bytes(8, "little")) # FCS
    # Block header: size=rle_size, type=01 (RLE), last=1
    hdr = (rle_size << 3) | (0b01 << 1) | 1
    frame.extend(hdr.to_bytes(3, "little"))
    frame.append(rle_byte)

    result = decompress(bytes(frame))
    assert result == bytes([rle_byte]) * rle_size


# =============================================================================
# TC-12: Decompress frame with incompatible FSE modes -> error
# =============================================================================


def test_tc12_incompatible_fse_modes() -> None:
    """A compressed block with non-zero FSE modes must raise ValueError.

    Our decoder only supports Predefined mode (0x00 modes byte). Any other mode
    byte indicates an FSE_Compressed or Repeat table that we don't support.
    """
    # Construct a frame with a compressed block whose modes byte is non-zero.
    # We'll set LL mode = 2 (FSE_Compressed) which we don't support.
    modes_byte = 0b10_00_00_00  # LL mode = 2 (FSE_Compressed)

    # Build minimal compressed block data:
    #   literals: 1 byte (n=1, header=0x08), literal=0x41
    #   seq count: 0x01 (one sequence, but we won't provide valid seqs)
    #   modes_byte: non-zero
    #   (no valid bitstream follows — we just need to trigger the mode check)
    lit_header = bytes([0x08, 0x41])  # Raw literals, 1 byte: 'A'
    seq_count = bytes([0x01])         # 1 sequence
    block_content = lit_header + seq_count + bytes([modes_byte])

    # Build ZStd frame around this compressed block.
    frame = bytearray()
    frame.extend(MAGIC.to_bytes(4, "little"))
    frame.append(0xE0)  # FHD: single-seg, 8-byte FCS
    frame.extend((1).to_bytes(8, "little"))  # FCS = 1
    # Compressed block header: size=len(block_content), type=10, last=1
    hdr = (len(block_content) << 3) | (0b10 << 1) | 1
    frame.extend(hdr.to_bytes(3, "little"))
    frame.extend(block_content)

    with pytest.raises((ValueError, Exception)):
        decompress(bytes(frame))


# =============================================================================
# Additional round-trip tests
# =============================================================================


def test_hello_world() -> None:
    """'hello world' round-trips correctly."""
    assert rt(b"hello world") == b"hello world"


def test_all_zeros() -> None:
    """1000 zero bytes round-trip correctly (should be RLE)."""
    data = bytes(1000)
    result = rt(data)
    assert result == data


def test_all_0xff() -> None:
    """1000 0xFF bytes round-trip correctly (should be RLE)."""
    data = bytes([0xFF] * 1000)
    assert rt(data) == data


def test_binary_data() -> None:
    """Binary data with repeating pattern round-trips correctly."""
    data = bytes(i % 256 for i in range(300))
    assert rt(data) == data


def test_large_prose() -> None:
    """Large prose (>128 KB) spans multiple compressed blocks."""
    text = "the quick brown fox jumps over the lazy dog\n" * 3000
    data = text.encode("ascii")
    assert len(data) > 128 * 1024  # must span multiple blocks
    assert rt(data) == data


def test_repeated_pattern() -> None:
    """Repeating byte pattern round-trips correctly."""
    pattern = b"ABCDEFGH"
    data = (pattern * (3000 // len(pattern) + 1))[:3000]
    assert rt(data) == data


# =============================================================================
# Internal helper unit tests
# =============================================================================


class TestRevBitRoundtrip:
    """Verify that RevBitWriter and RevBitReader are perfect inverses."""

    def test_basic_roundtrip(self) -> None:
        """Write known bits and read them back in reverse write order."""
        bw = _RevBitWriter()
        bw.add_bits(0b101, 3)       # A — written first -> read last
        bw.add_bits(0b11001100, 8)  # B
        bw.add_bits(0b1, 1)         # C — written last -> read first
        bw.flush()
        buf = bw.finish()

        br = _RevBitReader(buf)
        assert br.read_bits(1) == 0b1        # C: last written, first read
        assert br.read_bits(8) == 0b11001100  # B
        assert br.read_bits(3) == 0b101       # A: first written, last read

    def test_zero_bits(self) -> None:
        """Writing 0 bits is a no-op."""
        bw = _RevBitWriter()
        bw.add_bits(0xFF, 0)  # no-op
        bw.add_bits(0b1010, 4)
        bw.flush()
        buf = bw.finish()

        br = _RevBitReader(buf)
        assert br.read_bits(0) == 0
        assert br.read_bits(4) == 0b1010

    def test_many_bits(self) -> None:
        """Writing 64 bits crosses byte boundaries correctly."""
        bw = _RevBitWriter()
        value = 0xDEAD_BEEF_CAFE_1234
        bw.add_bits(value, 64)
        bw.flush()
        buf = bw.finish()

        br = _RevBitReader(buf)
        recovered = br.read_bits(64)
        assert recovered == value & 0xFFFF_FFFF_FFFF_FFFF


class TestFSEDecodeTable:
    """Verify FSE decode table construction."""

    def test_ll_table_size(self) -> None:
        """LL decode table has exactly 2^LL_ACC_LOG entries."""
        tbl = _build_decode_table(LL_NORM, LL_ACC_LOG)
        assert len(tbl) == (1 << LL_ACC_LOG)

    def test_ll_symbols_valid(self) -> None:
        """Every slot in the LL decode table has a valid symbol."""
        tbl = _build_decode_table(LL_NORM, LL_ACC_LOG)
        for entry in tbl:
            assert 0 <= entry["sym"] < len(LL_NORM)

    def test_ml_table_size(self) -> None:
        """ML decode table has exactly 2^ML_ACC_LOG entries."""
        tbl = _build_decode_table(ML_NORM, ML_ACC_LOG)
        assert len(tbl) == (1 << ML_ACC_LOG)

    def test_of_table_size(self) -> None:
        """OF decode table has exactly 2^OF_ACC_LOG entries."""
        tbl = _build_decode_table(OF_NORM, OF_ACC_LOG)
        assert len(tbl) == (1 << OF_ACC_LOG)

    def test_nb_range(self) -> None:
        """nb field in each decode entry is non-negative and <= acc_log."""
        for norm, acc_log in [
            (LL_NORM, LL_ACC_LOG),
            (ML_NORM, ML_ACC_LOG),
            (OF_NORM, OF_ACC_LOG),
        ]:
            tbl = _build_decode_table(norm, acc_log)
            for entry in tbl:
                assert 0 <= entry["nb"] <= acc_log, (
                    f"nb={entry['nb']} out of range for acc_log={acc_log}"
                )


class TestLLMLCodes:
    """Verify ll_to_code and ml_to_code mappings."""

    def test_ll_identity_range(self) -> None:
        """LL values 0..15 map to codes 0..15 (identity mapping)."""
        for i in range(16):
            assert _ll_to_code(i) == i, f"LL code for {i}"

    def test_ml_identity_range(self) -> None:
        """ML values 3..34 map to codes 0..31."""
        for i in range(3, 35):
            assert _ml_to_code(i) == i - 3, f"ML code for {i}"

    def test_ll_grouped(self) -> None:
        """LL value 16 maps to code 16 (first grouped range)."""
        assert _ll_to_code(16) == 16
        assert _ll_to_code(17) == 16  # 17 = 16 + 1 extra bit

    def test_ml_grouped(self) -> None:
        """ML value 35 maps to code 32 (first grouped range)."""
        assert _ml_to_code(35) == 32
        assert _ml_to_code(36) == 32  # 36 = 35 + 1 extra bit


class TestLiteralsSection:
    """Verify literals section encode/decode symmetry."""

    def test_short_roundtrip(self) -> None:
        """Literals <= 31 bytes use a 1-byte header and round-trip correctly."""
        for n in [0, 1, 15, 31]:
            lits = bytes(range(n))
            enc = _encode_literals_section(lits)
            dec, consumed = _decode_literals_section(enc)
            assert dec == lits
            assert consumed == len(enc)

    def test_medium_roundtrip(self) -> None:
        """Literals 32..4095 bytes use a 2-byte header and round-trip."""
        for n in [32, 100, 256, 4095]:
            lits = bytes(i % 256 for i in range(n))
            enc = _encode_literals_section(lits)
            dec, consumed = _decode_literals_section(enc)
            assert dec == lits

    def test_large_roundtrip(self) -> None:
        """Literals > 4095 bytes use a 3-byte header and round-trip."""
        lits = bytes(i % 256 for i in range(5000))
        enc = _encode_literals_section(lits)
        dec, consumed = _decode_literals_section(enc)
        assert dec == lits

    def test_unsupported_type_raises(self) -> None:
        """Literals type != 0 raises ValueError."""
        bad = bytes([0x02])  # ltype = 2 (Huffman compressed)
        with pytest.raises(ValueError, match="unsupported literals type"):
            _decode_literals_section(bad)


class TestSeqCount:
    """Verify sequence count encode/decode symmetry."""

    @pytest.mark.parametrize(
        "count", [0, 1, 50, 127, 128, 200, 256, 300, 515, 1000, 0x7F7E]
    )
    def test_roundtrip(self, count: int) -> None:
        """Sequence count round-trips correctly, including multiples-of-256.

        The 2-byte encoding supports counts 128..32639 (0x7F7F).
        The 3-byte encoding handles larger counts.
        """
        enc = _encode_seq_count(count)
        dec, _ = _decode_seq_count(enc)
        assert dec == count, f"count {count}"

    def test_empty_raises(self) -> None:
        """Empty data raises ValueError."""
        with pytest.raises(ValueError, match="empty"):
            _decode_seq_count(b"")


class TestFSEEncodeDecode:
    """Verify FSE encode/decode symmetry on sequences."""

    def test_two_sequence_roundtrip(self) -> None:
        """Encoding two sequences and decoding them gives back the original values."""
        seqs = [
            _Seq(ll=2, ml=4, off=1),
            _Seq(ll=0, ml=3, off=2),
        ]
        bitstream = _encode_sequences_section(seqs)

        dt_ll = _build_decode_table(LL_NORM, LL_ACC_LOG)
        dt_ml = _build_decode_table(ML_NORM, ML_ACC_LOG)
        dt_of = _build_decode_table(OF_NORM, OF_ACC_LOG)

        br = _RevBitReader(bitstream)
        state_ll = br.read_bits(LL_ACC_LOG)
        state_ml = br.read_bits(ML_ACC_LOG)
        state_of = br.read_bits(OF_ACC_LOG)

        for i, expected in enumerate(seqs):
            ll_code, state_ll = _fse_decode_sym(state_ll, dt_ll, br)
            of_code, state_of = _fse_decode_sym(state_of, dt_of, br)
            ml_code, state_ml = _fse_decode_sym(state_ml, dt_ml, br)

            ll_base, ll_extra_bits = LL_CODES[ll_code]
            ml_base, ml_extra_bits = ML_CODES[ml_code]

            ll_dec = ll_base + br.read_bits(ll_extra_bits)
            ml_dec = ml_base + br.read_bits(ml_extra_bits)
            of_extra = br.read_bits(of_code)
            of_raw = (1 << of_code) | of_extra
            off_dec = of_raw - 3

            assert ll_dec == expected.ll, f"seq {i} LL"
            assert ml_dec == expected.ml, f"seq {i} ML"
            assert off_dec == expected.off, f"seq {i} OFF"

    def test_single_sequence_roundtrip(self) -> None:
        """Encoding one sequence and decoding gives back the original values."""
        seqs = [_Seq(ll=3, ml=5, off=2)]
        bitstream = _encode_sequences_section(seqs)

        dt_ll = _build_decode_table(LL_NORM, LL_ACC_LOG)
        dt_ml = _build_decode_table(ML_NORM, ML_ACC_LOG)
        dt_of = _build_decode_table(OF_NORM, OF_ACC_LOG)

        br = _RevBitReader(bitstream)
        state_ll = br.read_bits(LL_ACC_LOG)
        state_ml = br.read_bits(ML_ACC_LOG)
        state_of = br.read_bits(OF_ACC_LOG)

        ll_code, state_ll = _fse_decode_sym(state_ll, dt_ll, br)
        of_code, state_of = _fse_decode_sym(state_of, dt_of, br)
        ml_code, state_ml = _fse_decode_sym(state_ml, dt_ml, br)

        ll_base, ll_extra_bits = LL_CODES[ll_code]
        ml_base, ml_extra_bits = ML_CODES[ml_code]

        ll_dec = ll_base + br.read_bits(ll_extra_bits)
        ml_dec = ml_base + br.read_bits(ml_extra_bits)
        of_extra = br.read_bits(of_code)
        of_raw = (1 << of_code) | of_extra
        off_dec = of_raw - 3

        assert ll_dec == 3
        assert ml_dec == 5
        assert off_dec == 2


class TestWireFormat:
    """Test decompressor against manually constructed frames."""

    def test_raw_block_frame(self) -> None:
        """Manually constructed raw-block frame decompresses correctly.

        Frame layout:
          [0..3]  Magic LE
          [4]     FHD = 0x20 (Single_Segment=1, FCS=1byte)
          [5]     FCS = 5
          [6..8]  Block header: last=1, raw, size=5
          [9..13] b"hello"
        """
        frame = bytes([
            0x28, 0xB5, 0x2F, 0xFD,  # magic
            0x20,                      # FHD: Single_Segment=1, FCS=1byte
            0x05,                      # FCS = 5
            0x29, 0x00, 0x00,          # block: last=1, raw, size=5 (5<<3|1=41=0x29)
            ord("h"), ord("e"), ord("l"), ord("l"), ord("o"),
        ])
        assert decompress(frame) == b"hello"

    def test_rle_block_frame(self) -> None:
        """Manually constructed RLE block frame decompresses correctly."""
        rle_count = 8
        rle_byte = 0xBB
        frame = bytearray()
        frame.extend(MAGIC.to_bytes(4, "little"))
        frame.append(0xE0)  # FHD: single-seg, 8-byte FCS
        frame.extend(rle_count.to_bytes(8, "little"))
        hdr = (rle_count << 3) | (0b01 << 1) | 1  # RLE, last
        frame.extend(hdr.to_bytes(3, "little"))
        frame.append(rle_byte)
        assert decompress(bytes(frame)) == bytes([rle_byte] * rle_count)

    def test_wrong_magic(self) -> None:
        """Wrong magic number raises ValueError with informative message."""
        frame = b"\x00\x01\x02\x03" + b"\xE0" + b"\x00" * 12
        with pytest.raises(ValueError, match="bad magic"):
            decompress(frame)

    def test_too_short(self) -> None:
        """Frame shorter than 5 bytes raises ValueError."""
        with pytest.raises(ValueError, match="too short"):
            decompress(b"\x28\xB5\x2F")


class TestDeterminism:
    """Compression output must be deterministic."""

    def test_same_input_same_output(self) -> None:
        """Compressing the same data twice produces identical bytes."""
        data = b"hello, ZStd world! " * 50
        assert compress(data) == compress(data)
