"""Tests for the coding_adventures_zstd package.

Tests cover:
  - Round-trip correctness for various data shapes
  - RLE and multi-block behaviour
  - Error handling (bad magic, truncation, unsupported modes)
  - Compression ratio sanity checks
  - Internal helper correctness (FSE tables, bit I/O, literals section, etc.)
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

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
    _decompress_block,
    _encode_literals_section,
    _encode_seq_count,
    _encode_sequences_section,
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
        "count", [0, 1, 50, 127, 128, 200, 256, 300, 515, 1000, 0x7EFF, 0x7F00, 40000]
    )
    def test_roundtrip(self, count: int) -> None:
        """Sequence count round-trips correctly, including multiples-of-256
        and the 2-byte/3-byte boundary (0x7EFF is the last 2-byte value,
        0x7F00 is the first 3-byte value).
        """
        enc = _encode_seq_count(count)
        dec, _ = _decode_seq_count(enc)
        assert dec == count, f"count {count}"

    def test_empty_raises(self) -> None:
        """Empty data raises ValueError."""
        with pytest.raises(ValueError, match="empty"):
            _decode_seq_count(b"")

    @pytest.mark.parametrize(
        ("count", "expected"),
        [
            (200, bytes([0x80, 0xC8])),
            (300, bytes([0x81, 0x2C])),
            (515, bytes([0x82, 0x03])),
            (768, bytes([0x83, 0x00])),
        ],
    )
    def test_wire_bytes_exact(self, count: int, expected: bytes) -> None:
        """Exact wire-byte assertions for the 2-byte Number_of_Sequences
        form, cross-checked against the real zstd wire format (RFC 8878
        §3.1.1.3.1: byte0 = 0x80 | (count >> 8), byte1 = count & 0xFF — NO
        additive offset).

        An earlier revision of this codec applied a spurious "-0x80"
        adjustment before splitting into bytes. That version would have
        encoded 200 as [0x80, 0x48] instead of the correct [0x80, 0xC8] —
        internally self-consistent (its own decoder undid the same
        offset) but rejected by the real `zstd` CLI as corrupt. See
        lessons.md Lesson 95/96.
        """
        assert _encode_seq_count(count) == expected, f"count {count}"


def _decode_seqs_reference(bitstream: bytes, seqs: list) -> list[tuple[int, int, int]]:
    """Decode a sequences-section bitstream using the CORRECT (RFC 8878 /
    real-`zstd`-verified) peek/read/update ordering.

    This mirrors ``_decompress_block``'s sequence loop exactly, but as a
    standalone helper so unit tests can exercise the low-level FSE
    primitives directly without constructing a full compressed block.
    Duplicated here (rather than imported) so a regression in
    ``_decompress_block`` doesn't silently make this helper "agree" with a
    broken implementation — this is the independent spec-shaped version.

    Returns:
        List of (ll, ml, off) tuples, one per sequence, in decode order.
    """
    dt_ll = _build_decode_table(LL_NORM, LL_ACC_LOG)
    dt_ml = _build_decode_table(ML_NORM, ML_ACC_LOG)
    dt_of = _build_decode_table(OF_NORM, OF_ACC_LOG)

    br = _RevBitReader(bitstream)
    # Initial states are read in order LL, OF, ML (RFC 8878 §3.1.1.3.2.1.2).
    state_ll = br.read_bits(LL_ACC_LOG)
    state_of = br.read_bits(OF_ACC_LOG)
    state_ml = br.read_bits(ML_ACC_LOG)

    n = len(seqs)
    results = []
    for i in range(n):
        # Step 1: peek symbols (no bits consumed).
        ll_entry = dt_ll[state_ll]
        ml_entry = dt_ml[state_ml]
        of_entry = dt_of[state_of]
        ll_code = ll_entry["sym"]
        ml_code = ml_entry["sym"]
        of_code = of_entry["sym"]

        ll_base, ll_extra_bits = LL_CODES[ll_code]
        ml_base, ml_extra_bits = ML_CODES[ml_code]

        # Step 2: extra bits, order OF, ML, LL.
        of_extra = br.read_bits(of_code)
        of_raw = (1 << of_code) | of_extra
        off_dec = of_raw - 3
        ml_dec = ml_base + br.read_bits(ml_extra_bits)
        ll_dec = ll_base + br.read_bits(ll_extra_bits)

        # Step 3: update states, order LL, ML, OF — skipped for the last seq.
        if i != n - 1:
            state_ll = ll_entry["base"] + br.read_bits(ll_entry["nb"])
            state_ml = ml_entry["base"] + br.read_bits(ml_entry["nb"])
            state_of = of_entry["base"] + br.read_bits(of_entry["nb"])

        results.append((ll_dec, ml_dec, off_dec))
    return results


class TestFSEEncodeDecode:
    """Verify FSE encode/decode symmetry on sequences."""

    def test_two_sequence_roundtrip(self) -> None:
        """Encoding two sequences and decoding them gives back the original values."""
        seqs = [
            _Seq(ll=2, ml=4, off=1),
            _Seq(ll=0, ml=3, off=2),
        ]
        bitstream = _encode_sequences_section(seqs)
        decoded = _decode_seqs_reference(bitstream, seqs)

        for i, (expected, (ll_dec, ml_dec, off_dec)) in enumerate(
            zip(seqs, decoded, strict=True)
        ):
            assert ll_dec == expected.ll, f"seq {i} LL"
            assert ml_dec == expected.ml, f"seq {i} ML"
            assert off_dec == expected.off, f"seq {i} OFF"

    def test_single_sequence_roundtrip(self) -> None:
        """Encoding one sequence and decoding gives back the original values."""
        seqs = [_Seq(ll=3, ml=5, off=2)]
        bitstream = _encode_sequences_section(seqs)
        (ll_dec, ml_dec, off_dec), = _decode_seqs_reference(bitstream, seqs)

        assert ll_dec == 3
        assert ml_dec == 5
        assert off_dec == 2

    def test_many_sequence_roundtrip(self) -> None:
        """A longer sequence list (exercising the last-sequence special case
        across multiple non-last sequences too) round-trips correctly."""
        seqs = [
            _Seq(ll=0, ml=3, off=1),
            _Seq(ll=5, ml=10, off=100),
            _Seq(ll=1, ml=3, off=4),
            _Seq(ll=20, ml=40, off=2000),
            _Seq(ll=0, ml=3, off=1),
        ]
        bitstream = _encode_sequences_section(seqs)
        decoded = _decode_seqs_reference(bitstream, seqs)

        for i, (expected, (ll_dec, ml_dec, off_dec)) in enumerate(
            zip(seqs, decoded, strict=True)
        ):
            assert ll_dec == expected.ll, f"seq {i} LL"
            assert ml_dec == expected.ml, f"seq {i} ML"
            assert off_dec == expected.off, f"seq {i} OFF"


def _apply_seq_offsets(lits: bytes, seqs: list[tuple[int, int, int]]) -> bytes:
    """Independently apply a list of ALREADY-RESOLVED (ll, ml, offset)
    triples to ``lits``, mirroring the trivial "emit literals, then copy
    ml bytes from offset positions back" step of ``_decompress_block``'s
    main loop.

    This is intentionally the ONLY piece of decode logic duplicated here —
    it is not what TestRepeatOffsetDecode below is testing (that logic
    predates this fix and is already covered by the FSE round-trip tests
    above). What IS being tested is which `offset` value the decoder's new
    Repeated_Offset (R1/R2/R3) selector logic computes for each sequence —
    this helper just gives an independent way to turn "the offsets we
    expect, by hand, per RFC 8878 / the reference decoder" into expected
    OUTPUT BYTES, so the test can assert on `bytes(out)` instead of
    re-deriving byte-for-byte content by hand (error-prone for 30+ bytes).
    """
    out = bytearray()
    lit_pos = 0
    for ll, ml, offset in seqs:
        out.extend(lits[lit_pos:lit_pos + ll])
        lit_pos += ll
        copy_start = len(out) - offset
        for i in range(ml):
            out.append(out[copy_start + i])
    return bytes(out)


class TestRepeatOffsetDecode:
    """Repeated-Offset (R1/R2/R3) sequence decoding — RFC 8878
    §3.1.1.3.2.1.1 — lessons.md Lesson 98.

    This package's own ENCODER never emits Offset_Value <= 3 (every LZSS
    match offset is coded explicitly, ``raw_off = seq.off + 3 >= 4`` since
    the minimum real match offset is 1), so the normal ``_Seq(off=...)``
    API can't directly produce a repeat-offset code — a real back-reference
    offset can never be small enough. To exercise the DECODER's
    repeat-offset branch in isolation (independent of the real `zstd` CLI —
    see TestCliInterop below for the end-to-end proof), these tests pass
    ``_Seq.off`` values of 0, -1, and -2. Those are NOT real offsets (no
    encoder would ever produce them from actual LZ77 matches) — they are a
    deliberate trick that exploits `_encode_sequences_section`'s own
    ``raw_off = seq.off + 3`` formula (the SAME formula the decoder inverts)
    to land on a chosen (of_code, extra_bits) pair, i.e. a chosen
    Offset_Value in {1, 2, 3}, letting the test drive the decoder's new
    selector logic directly:

        seq.off = -2  →  raw_off = 1  →  of_code=0, extra=0  →  Offset_Value=1
        seq.off = -1  →  raw_off = 2  →  of_code=1, extra=0  →  Offset_Value=2
        seq.off =  0  →  raw_off = 3  →  of_code=1, extra=1  →  Offset_Value=3

    Combined with a chosen literal length (0 selects "ll_is_zero=True"; a
    non-zero LL code 1..15 selects "ll_is_zero=False"), every one of the
    four repeat-offset selectors (RFC 8878's "ll_is_zero + Offset_Value - 1"
    shift, 0..3) is independently reachable and asserted below.
    """

    def test_all_four_selectors_and_frame_scoped_registers(self) -> None:
        """Six sequences in one block: two explicit offsets seed R1/R2/R3
        away from their frame-default [1, 4, 8], then four repeat-offset
        sequences — one per selector (0, 1, 2, 3) — read and rotate them.

        Expected offsets and register transitions below were derived BY
        HAND from RFC 8878 §3.1.1.3.2.1.1 and cross-checked against the
        literal `ZSTD_decodeSequence` reference C source (fetched from
        github.com/facebook/zstd, not recalled from memory) before writing
        this test — see the module comment above `_decompress_block` in
        __init__.py for the full derivation. `_apply_seq_offsets` then
        turns those hand-derived (ll, ml, offset) triples into expected
        OUTPUT BYTES independently of `_decompress_block`, so this test
        catches a wrong offset/register computation as either a byte
        mismatch or (if the wrong offset is 0 or out of bounds) a
        ValueError, not a silently-passing false positive.
        """
        lits = bytes(range(18))  # 18 arbitrary, distinct literal bytes

        # (ll, ml, off) as fed to the ENCODER. `off` is a REAL offset for
        # the two explicit sequences (A, B) and a repeat-offset "selector
        # trick" value (see class docstring) for the other four.
        seqs = [
            _Seq(ll=5, ml=3, off=3),    # A: explicit -> offset 3
            _Seq(ll=12, ml=3, off=20),  # B: explicit -> offset 20
            _Seq(ll=0, ml=3, off=0),    # C: selector 3 (ll=0, Offset_Value=3)
            _Seq(ll=1, ml=3, off=-2),   # D: selector 0 (ll>0, Offset_Value=1)
            _Seq(ll=0, ml=3, off=-2),   # E: selector 1 (ll=0, Offset_Value=1)
            _Seq(ll=0, ml=3, off=-1),   # F: selector 2 (ll=0, Offset_Value=2)
        ]

        block = bytearray()
        block.extend(_encode_literals_section(lits))
        block.extend(_encode_seq_count(len(seqs)))
        block.append(0x00)  # symbol compression modes: all Predefined
        block.extend(_encode_sequences_section(seqs))

        out = bytearray()
        rep = [1, 4, 8]  # frame defaults (RFC 8878 §3.1.1.3.2.1.1)
        _decompress_block(bytes(block), out, rep)

        # Hand-derived (ll, ml, resolved_offset) per sequence:
        #   A explicit(3):    R=[1,4,8]  -> R=[3,1,4]
        #   B explicit(20):   R=[3,1,4]  -> R=[20,3,1]
        #   C selector3: offset=R1-1=19; full rotate -> R=[19,20,3]
        #   D selector0: offset=R1=19;   no rotation  -> R=[19,20,3]
        #   E selector1: offset=R2=20;   R1<->R2 swap -> R=[20,19,3]
        #   F selector2: offset=R3=3;    full rotate  -> R=[3,20,19]
        expected = _apply_seq_offsets(
            lits,
            [
                (5, 3, 3),
                (12, 3, 20),
                (0, 3, 19),
                (1, 3, 19),
                (0, 3, 20),
                (0, 3, 3),
            ],
        )
        assert bytes(out) == expected
        assert rep == [3, 20, 19]

    def test_repeat_offset_persists_across_blocks_in_same_frame(self) -> None:
        """The R1/R2/R3 registers are FRAME-scoped, not block-scoped: a
        Compressed block's sequences must be able to reuse an offset an
        EARLIER block in the same frame established, via the same `rep`
        list threaded through both calls (mirroring what `decompress()`
        does across the block loop).

        Block 1's one explicit-offset(7) sequence rotates the frame
        defaults [1, 4, 8] to [7, 1, 4] (R1=7, R2=1, R3=4 — note R2 is now
        1, the OLD R1, not the original default 4; a naive "R2 is still
        its startup default" assumption would be wrong here, which is
        exactly why this test picks a selector-1 (R2) reference for block
        2 rather than a selector-0 (R1) one). Block 2's sequence has
        ll=0 and an Offset_Value of 1 (the ``off=-2`` selector trick, see
        the class docstring) -> selector 1 -> "use R2" = 1.
        """
        lits1 = b"0123456789"
        seqs1 = [_Seq(ll=10, ml=3, off=7)]  # explicit offset 7
        block1 = bytearray()
        block1.extend(_encode_literals_section(lits1))
        block1.extend(_encode_seq_count(len(seqs1)))
        block1.append(0x00)
        block1.extend(_encode_sequences_section(seqs1))

        lits2 = b""
        seqs2 = [_Seq(ll=0, ml=3, off=-2)]  # selector 1 -> R2
        block2 = bytearray()
        block2.extend(_encode_literals_section(lits2))
        block2.extend(_encode_seq_count(len(seqs2)))
        block2.append(0x00)
        block2.extend(_encode_sequences_section(seqs2))

        out = bytearray()
        rep = [1, 4, 8]
        _decompress_block(bytes(block1), out, rep)
        assert bytes(out) == _apply_seq_offsets(lits1, [(10, 3, 7)])
        assert bytes(out) == b"0123456789345"
        assert rep == [7, 1, 4]  # explicit offset 7 rotated in; R2 = old R1

        _decompress_block(bytes(block2), out, rep)
        # Block 2 has no literals of its own; its sequence copies 3 bytes
        # at offset R2=1 from the TAIL of block 1's output (cross-block
        # back-reference — only possible because `out` and `rep` both
        # persist across the two _decompress_block calls, exactly as
        # decompress() threads them across a frame's block loop).
        # offset=1 with ml=3 self-overlaps: copy_start = 13-1 = 12, and
        # each copied byte becomes readable by the next copy in the same
        # loop, so it repeats out[12] ('5') three times, not a 3-byte slice.
        assert bytes(out) == b"0123456789345" + b"555"
        assert bytes(out) == b"0123456789345555"
        assert rep == [1, 7, 4]  # selector 1: R1<->R2 swap, R3 untouched


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


# =============================================================================
# TC-9 (spec): Cross-language / interoperability with the real `zstd` CLI
# =============================================================================
#
# code/specs/CMP07-zstd.md ("Test Cases" section) defines TC-9 as: compress
# with the real `zstd` CLI and decompress with ours, AND compress with ours
# and decompress with the real `zstd -d` CLI — both directions must round-
# trip exactly. This was previously MISSING from this package's test suite
# (the numeric labels TC-1..TC-12 further up this file predate alignment
# with the shared cross-language spec and cover a different, python-specific
# set of scenarios — they are not renumbered here to avoid unrelated churn).
#
# Why this matters: purely-internal round-trip tests (compress-then-
# decompress using only this package's own code) can pass even when the
# wire format is flat-out wrong, because a self-consistent bug in the
# encoder is invisible to a decoder built from the same wrong assumptions.
# That is exactly what happened here — see lessons.md Lesson 95/96 and the
# sibling `java/zstd` (#9780) and `rust/zstd` audits. Three real bugs were
# found ONLY by decompressing our output with the actual `zstd` binary:
#
#   1. FSE table-spread algorithm used a fabricated two-pass split instead
#      of the real single-pass algorithm (_build_decode_table /
#      _build_encode_sym).
#   2. Per-sequence FSE field order was wrong: symbols must be PEEKED (no
#      bits consumed) before any bits are read, extra bits are read in
#      order OF/ML/LL, state updates happen in order LL/ML/OF, and the
#      state update is skipped entirely for a block's last sequence
#      (_encode_sequences_section / _decompress_block).
#   3. The Number_of_Sequences 2-byte wire encoding applied a spurious
#      -0x80/+0x80 offset not present in the real format
#      (_encode_seq_count / _decode_seq_count) — this one hid behind the
#      1-byte encoding for any input producing < 128 LZ77 sequences, so it
#      only surfaced on inputs large/repetitive enough to cross that
#      boundary.
#
# All three were internally self-consistent (our decoder mirrored our own
# encoder) and therefore invisible to every pre-existing round-trip test in
# this file — the real `zstd` CLI was the only thing that caught them.
#
# Gracefully skipped (not failed) when `zstd` isn't on PATH, matching the
# convention used by the java/zstd and rust/zstd sibling packages.

_ZSTD_CLI = shutil.which("zstd")
_ZSTD_SKIP_REASON = "real `zstd` CLI not found on PATH"


def _zstd_cli_decompress(compressed: bytes, tmp_path: Path) -> bytes:
    """Decompress ``compressed`` bytes using the real `zstd -d` CLI."""
    in_path = tmp_path / "cli_in.zst"
    out_path = tmp_path / "cli_out.bin"
    in_path.write_bytes(compressed)
    result = subprocess.run(
        [_ZSTD_CLI, "-d", "-f", str(in_path), "-o", str(out_path)],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert result.returncode == 0, (
        f"real `zstd -d` rejected our compressed output: {result.stderr.strip()}"
    )
    return out_path.read_bytes()


def _zstd_cli_compress(data: bytes, tmp_path: Path) -> bytes:
    """Compress ``data`` bytes using the real `zstd` CLI (default settings:
    includes a trailing content checksum, per real zstd's default)."""
    in_path = tmp_path / "cli_orig.bin"
    out_path = tmp_path / "cli_orig.zst"
    in_path.write_bytes(data)
    result = subprocess.run(
        [_ZSTD_CLI, "-f", str(in_path), "-o", str(out_path)],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert result.returncode == 0, (
        f"real `zstd` CLI failed to compress: {result.stderr.strip()}"
    )
    return out_path.read_bytes()


@pytest.mark.skipif(_ZSTD_CLI is None, reason=_ZSTD_SKIP_REASON)
class TestCliInterop:
    """TC-9 (spec): real `zstd` CLI interoperability, both directions."""

    def test_our_compress_real_decompress_spec_text(self, tmp_path: Path) -> None:
        """The exact TC-9 spec payload: our compress(), decompressed by the
        real `zstd -d` CLI, must match byte-for-byte."""
        text = "the quick brown fox jumps over the lazy dog " * 25
        data = text.encode("utf-8")
        compressed = compress(data)
        assert _zstd_cli_decompress(compressed, tmp_path) == data

    def test_real_compress_our_decompress_spec_text(self, tmp_path: Path) -> None:
        """The exact TC-9 spec payload, compressed by the real `zstd` CLI
        (which by default appends a content checksum), decompressed by
        ours, must match byte-for-byte. Exercises the FHD
        Content_Checksum_Flag (bit 2) skip-logic in decompress()."""
        text = "the quick brown fox jumps over the lazy dog " * 25
        data = text.encode("utf-8")
        compressed = _zstd_cli_compress(data, tmp_path)
        assert decompress(compressed) == data

    @pytest.mark.parametrize(
        "data",
        [
            b"",
            b"\x42",
            bytes(range(256)),
            b"A" * 1024,
            bytes(i % 251 for i in range(4000)),
        ],
        ids=["empty", "single_byte", "all_bytes", "rle", "binary_pattern"],
    )
    def test_our_compress_real_decompress_various(
        self, data: bytes, tmp_path: Path
    ) -> None:
        """A spread of TC-1..TC-6-shaped payloads, all verified against the
        real `zstd -d` CLI rather than only our own decompress()."""
        compressed = compress(data)
        assert _zstd_cli_decompress(compressed, tmp_path) == data

    def test_our_compress_real_decompress_crosses_seq_count_boundary(
        self, tmp_path: Path
    ) -> None:
        """Regression test for the Number_of_Sequences wire-encoding bug
        (see the module comment above, bug 3): pick an input whose LZ77 pass
        produces >= 128 sequences, forcing the 2-byte Number_of_Sequences
        encoding. Before the fix, this crossed silently in our own
        round-trip tests but was rejected by the real `zstd` CLI with
        "Data corruption detected"."""
        data = b"the quick brown fox jumps over the lazy dog " * 800
        compressed = compress(data)
        assert _zstd_cli_decompress(compressed, tmp_path) == data

    def test_our_compress_real_decompress_multiblock(self, tmp_path: Path) -> None:
        """A >128 KB input (multiple compressed blocks) round-trips through
        the real `zstd -d` CLI.

        Sized just past the 128 KB block boundary (not e.g. 300 KB like the
        internal-only TC-8 test above) to keep LZSS-pass runtime bounded —
        see lessons.md Lesson 92 (LZSS/LZ77 passes are CPU-heavy and CI
        runners run ~25x slower than local); this still forces exactly 2
        blocks, which is what the multi-block wire format needs exercised.
        """
        unit = b"ABCDEFGHIJ" * 50 + b"ZYXWVUTSRQ" * 50
        size = 132 * 1024  # 128 KB + a bit, forces exactly 2 blocks
        data = (unit * (size // len(unit) + 1))[:size]
        compressed = compress(data)
        assert _zstd_cli_decompress(compressed, tmp_path) == data

    def test_real_compress_our_decompress_binary(self, tmp_path: Path) -> None:
        """Binary data compressed by the real `zstd` CLI, decompressed by
        ours."""
        data = bytes(i % 251 for i in range(4000))
        compressed = _zstd_cli_compress(data, tmp_path)
        assert decompress(compressed) == data

    def test_real_compress_our_decompress_repeat_offset_rle(
        self, tmp_path: Path
    ) -> None:
        """Regression test for lessons.md Lesson 98: `b"A" * 500` is exactly
        the shape of input where the real `zstd` CLI's encoder reaches for a
        Repeat_Offset (R1/R2/R3) shortcut — e.g. two literal bytes "AA"
        followed by one long match with Offset_Value=1 ("reuse R1", whose
        frame-default value is 1) — rather than this package's own RLE
        block type. Before the Lesson 98 fix, this package's decoder didn't
        understand Offset_Value <= 3 as a repeat reference and either raised
        ValueError (an explicit reject, added as defence-in-depth after an
        earlier revision crashed with an uncaught IndexError on the same
        input) or, before that guard existed, corrupted output. It must now
        decode to the exact original bytes.
        """
        data = b"A" * 500
        compressed = _zstd_cli_compress(data, tmp_path)
        assert decompress(compressed) == data

    def test_real_compress_our_decompress_repeat_offset_periodic(
        self, tmp_path: Path
    ) -> None:
        """Periodic data with a distinctive, non-trivial period gives the
        real `zstd` CLI's encoder many back-to-back LZ77 matches at the
        SAME distance (one full period apart) — the textbook case for
        repeatedly reusing Repeated_Offset1 rather than re-encoding the
        same explicit offset every time. Regression test for lessons.md
        Lesson 98, using a period (53 bytes) chosen to avoid degenerating
        into a single-byte RLE pattern (see test above) while still being
        short enough, repeated enough times, to make repeat-offset reuse
        very likely.
        """
        unit = bytes((i * 37 + 11) % 256 for i in range(53))
        data = unit * 200  # 10,600 bytes; period 53 forces repeated matches
        compressed = _zstd_cli_compress(data, tmp_path)
        assert decompress(compressed) == data

    def test_real_compress_our_decompress_still_rejects_huffman_literals(
        self, tmp_path: Path
    ) -> None:
        """Huffman-coded literals remain outside this educational codec's
        supported subset (see code/specs/CMP07-zstd.md's 'Educational
        Simplification' table: 'Literals: Huffman or raw' -> 'Raw only').
        This is UNCHANGED by the Lesson 98 repeat-offset fix — that fix is
        decode-only support for a SEQUENCES-section feature (Offset_Value
        <= 3), unrelated to the LITERALS-section encoding. High-entropy
        data compressible enough for `zstd` to bother with Huffman, but
        with enough symbol variety to make Huffman worthwhile over raw,
        should still raise a clear ValueError rather than crash or silently
        misdecode literals as if they were raw.
        """
        # A skewed-but-varied byte distribution: real zstd's heuristics
        # reliably pick Huffman-coded literals for this shape over raw.
        data = bytes((i * i) % 191 for i in range(20000))
        compressed = _zstd_cli_compress(data, tmp_path)
        try:
            result = decompress(compressed)
        except ValueError:
            return  # expected: Huffman literals correctly rejected
        # If the real CLI didn't actually choose Huffman literals for this
        # input (heuristics can vary by zstd version), the least we require
        # is that decode is still byte-exact — never silent corruption.
        assert result == data
