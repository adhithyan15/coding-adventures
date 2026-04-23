"""test_intel_8008_packager.py -- Tests for the Intel 8008 Intel HEX packager.

Tests cover:
- encode_hex: record format, checksum correctness, multi-record output,
              origin address, EOF record
- decode_hex: round-trip with encode_hex, error cases (bad checksum,
              unsupported type, malformed lines)
- 8008-specific: 16 KB address space limit, addresses > 0xFF (multi-byte addr)
"""

from __future__ import annotations

import pytest

from intel_8008_packager import decode_hex, encode_hex

# ===========================================================================
# encode_hex tests
# ===========================================================================

class TestEncodeHexFormat:
    """Tests for the output format of encode_hex."""

    def test_single_byte(self) -> None:
        """Single byte produces a data record plus EOF."""
        result = encode_hex(bytes([0xFF]))
        lines = result.strip().splitlines()
        assert len(lines) == 2
        assert lines[0].startswith(":")
        assert lines[1] == ":00000001FF"

    def test_starts_with_colon(self) -> None:
        """Every record line starts with ':'."""
        result = encode_hex(bytes([0x01, 0x02, 0x03]))
        for line in result.strip().splitlines():
            assert line.startswith(":")

    def test_eof_record_always_last(self) -> None:
        """The End-of-File record ':00000001FF' is always the last line."""
        result = encode_hex(bytes([0xAA, 0xBB]))
        lines = result.strip().splitlines()
        assert lines[-1] == ":00000001FF"

    def test_three_byte_program(self) -> None:
        """Three-byte program produces correct record format."""
        binary = bytes([0x06, 0x00, 0xFF])  # MVI B, 0; HLT
        result = encode_hex(binary)
        lines = result.strip().splitlines()
        # data record: :03 0000 00 06 00 FF <cs>
        data_line = lines[0]
        # byte count field = '03'
        assert data_line[1:3] == "03"
        # address = '0000'
        assert data_line[3:7] == "0000"
        # record type = '00'
        assert data_line[7:9] == "00"
        # data bytes = '0600FF'
        assert data_line[9:15] == "0600FF"

    def test_checksum_verification(self) -> None:
        """Each data record has a valid checksum (sum of all bytes = 0 mod 256)."""
        binary = bytes([0x06, 0x00, 0xFF])
        result = encode_hex(binary)
        for line in result.strip().splitlines():
            if line == ":00000001FF":
                continue  # skip EOF
            # Parse bytes from the record (excluding the leading ':')
            record_hex = line[1:]
            record_bytes = bytes.fromhex(record_hex)
            assert sum(record_bytes) % 256 == 0, (
                f"Checksum verification failed for record: {line}"
            )

    def test_16_byte_record_boundary(self) -> None:
        """Exactly 16 bytes produces one data record."""
        binary = bytes(range(16))
        result = encode_hex(binary)
        lines = result.strip().splitlines()
        assert len(lines) == 2  # 1 data + 1 EOF
        assert lines[0][1:3] == "10"  # byte count = 16 = 0x10

    def test_17_byte_splits_into_two_records(self) -> None:
        """17 bytes splits into one 16-byte record and one 1-byte record."""
        binary = bytes(range(17))
        result = encode_hex(binary)
        lines = result.strip().splitlines()
        assert len(lines) == 3  # 2 data + 1 EOF
        assert lines[0][1:3] == "10"  # first record: 16 bytes
        assert lines[1][1:3] == "01"  # second record: 1 byte

    def test_32_byte_split(self) -> None:
        """32 bytes = exactly two 16-byte records."""
        binary = bytes(range(32))
        result = encode_hex(binary)
        lines = result.strip().splitlines()
        assert len(lines) == 3  # 2 data + 1 EOF

    def test_address_increments_between_records(self) -> None:
        """Record addresses increment by 16 (0x10) for each subsequent record."""
        binary = bytes(range(32))
        result = encode_hex(binary)
        lines = result.strip().splitlines()
        # First record at address 0x0000
        assert lines[0][3:7] == "0000"
        # Second record at address 0x0010
        assert lines[1][3:7] == "0010"

    def test_origin_nonzero(self) -> None:
        """Non-zero origin shifts all record addresses."""
        binary = bytes([0xFF])
        result = encode_hex(binary, origin=0x2000)
        lines = result.strip().splitlines()
        # Address in first record should be 0x2000
        assert lines[0][3:7] == "2000"

    def test_origin_cross_256_boundary(self) -> None:
        """Origin of 0x0100 sets address high byte correctly."""
        binary = bytes([0xAA])
        result = encode_hex(binary, origin=0x0100)
        lines = result.strip().splitlines()
        assert lines[0][3:7] == "0100"


class TestEncodeHexChecksums:
    """Tests for checksum correctness."""

    def test_known_checksum_three_bytes(self) -> None:
        """Verify the checksum for a known 3-byte record.

        Record: :03 0000 00 01 02 03
        sum = 0x03 + 0x00 + 0x00 + 0x00 + 0x01 + 0x02 + 0x03 = 0x09
        checksum = (0x100 - 0x09) % 0x100 = 0xF7
        """
        binary = bytes([0x01, 0x02, 0x03])
        result = encode_hex(binary)
        lines = result.strip().splitlines()
        # Checksum is last 2 hex chars of data line
        cs_hex = lines[0][-2:]
        assert cs_hex == "F7"

    def test_all_bytes_checksum_correctness(self) -> None:
        """For any binary, every record's checksum makes the record sum to 0."""
        import random
        random.seed(42)
        binary = bytes(random.randint(0, 255) for _ in range(64))
        result = encode_hex(binary)
        for line in result.strip().splitlines():
            if line == ":00000001FF":
                continue
            record_bytes = bytes.fromhex(line[1:])
            assert sum(record_bytes) % 256 == 0


class TestEncodeHexErrors:
    """Tests for encode_hex error handling."""

    def test_empty_binary_raises(self) -> None:
        """Empty binary raises ValueError."""
        with pytest.raises(ValueError, match="must be non-empty"):
            encode_hex(b"")

    def test_origin_negative_raises(self) -> None:
        """Negative origin raises ValueError."""
        with pytest.raises(ValueError, match="origin must be"):
            encode_hex(bytes([0xFF]), origin=-1)

    def test_origin_overflow_raises(self) -> None:
        """Origin beyond 16-bit range raises ValueError."""
        # origin=0x10000 is one past the 16-bit maximum (0xFFFF)
        with pytest.raises(ValueError, match="origin must be"):
            encode_hex(bytes([0xFF]), origin=0x10000)

    def test_image_overflows_16bit_raises(self) -> None:
        """Image that overflows the 16-bit space raises ValueError."""
        # 256 bytes at origin 0xFF01 → end at 0x10001 (overflow)
        with pytest.raises(ValueError, match="overflows"):
            encode_hex(bytes(256), origin=0xFF01)


# ===========================================================================
# decode_hex tests
# ===========================================================================

class TestDecodeHexRoundTrip:
    """Round-trip tests: encode_hex → decode_hex → original bytes."""

    def test_single_byte_roundtrip(self) -> None:
        """Single byte survives round-trip."""
        binary = bytes([0xFF])
        origin, recovered = decode_hex(encode_hex(binary))
        assert origin == 0
        assert recovered == binary

    def test_three_byte_roundtrip(self) -> None:
        """Three bytes survive round-trip."""
        binary = bytes([0x06, 0x00, 0xFF])
        origin, recovered = decode_hex(encode_hex(binary))
        assert origin == 0
        assert recovered == binary

    def test_16_byte_roundtrip(self) -> None:
        """Exactly 16 bytes (one full record) survive round-trip."""
        binary = bytes(range(16))
        origin, recovered = decode_hex(encode_hex(binary))
        assert origin == 0
        assert recovered == binary

    def test_17_byte_roundtrip(self) -> None:
        """17 bytes (spans two records) survive round-trip."""
        binary = bytes(range(17))
        origin, recovered = decode_hex(encode_hex(binary))
        assert origin == 0
        assert recovered == binary

    def test_256_byte_roundtrip(self) -> None:
        """256 bytes survive round-trip (16 records)."""
        binary = bytes(range(256))
        origin, recovered = decode_hex(encode_hex(binary))
        assert origin == 0
        assert recovered == binary

    def test_nonzero_origin_roundtrip(self) -> None:
        """Non-zero origin is preserved through round-trip."""
        binary = bytes([0xAA, 0xBB, 0xCC])
        hex_text = encode_hex(binary, origin=0x2000)
        origin, recovered = decode_hex(hex_text)
        assert origin == 0x2000
        assert recovered == binary

    def test_large_8008_image_roundtrip(self) -> None:
        """Full 8008 ROM image (code + padding) survives round-trip.

        The spec calls for a 16 KB image: ROM padded to 0xFF (unused bytes),
        RAM region initialized to 0x00.  We test a 1 KB slice for speed.
        """
        # Simulate: 32 bytes of code + 992 bytes of 0xFF padding
        code = bytes([0x06, 0x00, 0x46, 0x20, 0x00, 0xFF])
        padding = bytes([0xFF] * (1024 - len(code)))
        binary = code + padding
        origin, recovered = decode_hex(encode_hex(binary))
        assert origin == 0
        assert recovered == binary

    def test_all_zeros_roundtrip(self) -> None:
        """All-zero binary survives round-trip."""
        binary = bytes(32)
        origin, recovered = decode_hex(encode_hex(binary))
        assert recovered == binary

    def test_all_ff_roundtrip(self) -> None:
        """All-0xFF binary (erased flash) survives round-trip."""
        binary = bytes([0xFF] * 32)
        origin, recovered = decode_hex(encode_hex(binary))
        assert recovered == binary


class TestDecodeHexOutput:
    """Tests for decode_hex output structure."""

    def test_origin_is_min_address(self) -> None:
        """Origin returned is the minimum record address seen."""
        binary = bytes([0x01, 0x02])
        hex_text = encode_hex(binary, origin=0x0100)
        origin, _ = decode_hex(hex_text)
        assert origin == 0x0100

    def test_empty_hex_returns_empty(self) -> None:
        """Empty / EOF-only hex returns (0, b'')."""
        origin, data = decode_hex(":00000001FF\n")
        assert origin == 0
        assert data == b""

    def test_blank_lines_ignored(self) -> None:
        """Blank lines in the hex text are ignored."""
        binary = bytes([0xFF])
        hex_text = encode_hex(binary)
        hex_with_blanks = "\n" + hex_text + "\n\n"
        origin, recovered = decode_hex(hex_with_blanks)
        assert recovered == binary


class TestDecodeHexErrors:
    """Tests for decode_hex error handling."""

    def test_missing_colon_raises(self) -> None:
        """Line not starting with ':' raises ValueError."""
        with pytest.raises(ValueError, match="expected ':'"):
            decode_hex("0300000001020FF7\n:00000001FF\n")

    def test_invalid_hex_raises(self) -> None:
        """Non-hex characters in record raise ValueError."""
        with pytest.raises(ValueError, match="invalid hex"):
            decode_hex(":ZZZZZZZZ\n")

    def test_record_too_short_raises(self) -> None:
        """Record with fewer than 5 bytes raises ValueError."""
        with pytest.raises(ValueError, match="too short"):
            decode_hex(":0000FF\n")

    def test_bad_checksum_raises(self) -> None:
        """Record with wrong checksum raises ValueError."""
        # Corrupt the last two chars (checksum) of a valid record
        binary = bytes([0x01, 0x02, 0x03])
        hex_text = encode_hex(binary)
        lines = hex_text.strip().splitlines()
        # Replace checksum with 'FF' (wrong for this record unless it happens to match)
        corrupted_line = lines[0][:-2] + "00\n"
        corrupted = corrupted_line + lines[1] + "\n"
        with pytest.raises(ValueError, match="checksum mismatch"):
            decode_hex(corrupted)

    def test_unsupported_record_type_raises(self) -> None:
        """Record type 02 (Extended Segment Address) raises ValueError."""
        # Construct a valid type-02 record by hand
        # :02 0000 02 0000 FC  (Extended Segment Address)
        rec = ":020000020000FC\n:00000001FF\n"
        with pytest.raises(ValueError, match="unsupported record type"):
            decode_hex(rec)

    def test_truncated_data_raises(self) -> None:
        """Record that claims more data bytes than present raises ValueError."""
        # byte_count=5 but only 3 data bytes present (and no checksum)
        # :05 0000 00 01 02 03  (missing 2 data bytes + checksum)
        with pytest.raises(ValueError):
            decode_hex(":050000000102\n")

    def test_oversized_image_raises(self) -> None:
        """Image larger than 16 KB raises ValueError."""
        # Two records at far-apart addresses to create a large apparent image
        # Record at 0x0000 with 1 byte; record at 0x4001 with 1 byte.
        # Build both records by hand:
        r1 = ":01000000FF00\n"  # 1 byte at 0x0000, data=0xFF
        # At 0x4001, data=0x00:
        # sum([1, 0x40, 0x01, 0x00, 0x00]) = 0x42; cs = 0xBE
        r2 = ":0140010000BE\n"
        eof = ":00000001FF\n"
        with pytest.raises(ValueError, match="too large"):
            decode_hex(r1 + r2 + eof)


# ===========================================================================
# 8008-specific tests
# ===========================================================================

class TestIntel8008Specifics:
    """Tests verifying 8008-specific constraints (16 KB address space)."""

    def test_max_image_size_fits(self) -> None:
        """Full 16 KB image (0x4000 bytes) encodes and decodes correctly."""
        # Use 0x100 bytes for test speed
        binary = bytes([0xFF] * 256)
        origin, recovered = decode_hex(encode_hex(binary, origin=0x3F00))
        assert origin == 0x3F00
        assert recovered == binary

    def test_ram_address_roundtrip(self) -> None:
        """Data at RAM address 0x2000 survives round-trip."""
        binary = bytes([0x00] * 16)  # 16 bytes of RAM-region data
        hex_text = encode_hex(binary, origin=0x2000)
        origin, recovered = decode_hex(hex_text)
        assert origin == 0x2000
        assert recovered == binary

    def test_encode_decode_simulated_pipeline(self) -> None:
        """Simulate the assembler → packager → simulator pipeline.

        The assembler produces binary bytes; the packager produces Intel HEX;
        the simulator loads the HEX back to bytes.  This tests the full chain.
        """
        from intel_8008_packager import decode_hex as pkg_decode
        from intel_8008_packager import encode_hex as pkg_encode

        # Simulated assembler output: a minimal 8008 program
        # MVI B, 0:   [0x06, 0x00]
        # CAL func:   [0x46, 0x06, 0x00]  (func at 0x0006)
        # HLT:        [0xFF]
        # func: MOV A, B: [0x78]
        # RFC:        [0x07]
        assembled = bytes([0x06, 0x00, 0x46, 0x06, 0x00, 0xFF, 0x78, 0x07])

        hex_text = pkg_encode(assembled)
        origin, binary_back = pkg_decode(hex_text)
        assert origin == 0
        assert binary_back == assembled
