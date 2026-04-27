"""Tests for the Intel HEX encoder/decoder.

=== What We're Testing ===

The Intel HEX format is a 50-year-old standard used by every EPROM programmer
on the market.  Our encoder must produce byte-perfect output that any
programmer (hardware or software) will accept.

Key properties we verify:

1. Checksum correctness — the sum of all record bytes (including checksum)
   must equal 0 mod 256.  A single wrong bit causes the programmer to reject
   the file.

2. Record structure — correct byte count, 16-bit address, record type fields.

3. Record splitting — binaries larger than 16 bytes must be split into
   multiple records at the correct address boundaries.

4. EOF record — every valid Intel HEX file must end with ``:00000001FF``.

5. Round-trip fidelity — ``decode_hex(encode_hex(binary)) == binary`` for
   all inputs.

6. Error handling — malformed input raises ``ValueError`` with a helpful
   message.
"""

from __future__ import annotations

import pytest

from intel_4004_packager.hex_encoder import decode_hex, encode_hex


# ---------------------------------------------------------------------------
# Checksum correctness
# ---------------------------------------------------------------------------


class TestChecksum:
    """Verify that every record's checksum satisfies the Intel HEX invariant.

    The invariant: sum of (byte_count + addr_hi + addr_lo + rec_type +
    all data bytes + checksum) ≡ 0 (mod 256).
    """

    def _verify_record(self, line: str) -> None:
        """Parse one ``:...`` line and assert its checksum is valid."""
        assert line.startswith(":"), f"missing colon: {line!r}"
        record_bytes = bytes.fromhex(line[1:].strip())
        # All bytes including checksum must sum to 0 mod 256
        assert sum(record_bytes) % 256 == 0, (
            f"checksum invariant failed for {line!r}: "
            f"sum={sum(record_bytes):#04x}"
        )

    def test_single_byte(self) -> None:
        hex_text = encode_hex(bytes([0x01]))
        for line in hex_text.splitlines():
            if line:
                self._verify_record(line)

    def test_full_16_byte_record(self) -> None:
        binary = bytes(range(16))
        hex_text = encode_hex(binary)
        for line in hex_text.splitlines():
            if line:
                self._verify_record(line)

    def test_17_bytes_splits_into_two_records(self) -> None:
        binary = bytes(range(17))
        hex_text = encode_hex(binary)
        data_lines = [l for l in hex_text.splitlines() if l and not l.startswith(":00")]
        assert len(data_lines) == 2
        for line in hex_text.splitlines():
            if line:
                self._verify_record(line)

    def test_256_bytes_all_checksums_valid(self) -> None:
        binary = bytes(range(256))
        hex_text = encode_hex(binary)
        for line in hex_text.splitlines():
            if line:
                self._verify_record(line)

    def test_all_zeros(self) -> None:
        binary = bytes(16)
        hex_text = encode_hex(binary)
        for line in hex_text.splitlines():
            if line:
                self._verify_record(line)

    def test_all_0xff(self) -> None:
        binary = bytes([0xFF] * 16)
        hex_text = encode_hex(binary)
        for line in hex_text.splitlines():
            if line:
                self._verify_record(line)


# ---------------------------------------------------------------------------
# EOF record
# ---------------------------------------------------------------------------


class TestEofRecord:
    """Every Intel HEX file must end with ``:00000001FF``."""

    def test_eof_record_present(self) -> None:
        hex_text = encode_hex(bytes([0x00]))
        assert hex_text.strip().endswith(":00000001FF")

    def test_eof_record_is_last_line(self) -> None:
        hex_text = encode_hex(bytes(range(48)))
        non_empty_lines = [l for l in hex_text.splitlines() if l.strip()]
        assert non_empty_lines[-1] == ":00000001FF"

    def test_eof_record_checksum(self) -> None:
        # :00000001FF — verify the checksum manually
        # bytes: 0x00 0x00 0x00 0x01 0xFF
        # sum = 0x100 = 0 mod 256 ✓
        record_bytes = bytes.fromhex("00000001FF")
        assert sum(record_bytes) % 256 == 0


# ---------------------------------------------------------------------------
# Record structure
# ---------------------------------------------------------------------------


class TestRecordStructure:
    """Verify field layout of encoded records."""

    def test_single_byte_record_layout(self) -> None:
        # One data byte at address 0: :01 0000 00 XX CC
        hex_text = encode_hex(bytes([0xD5]))
        first_line = hex_text.splitlines()[0]
        # :01 = 1 data byte
        assert first_line[1:3] == "01", f"byte count wrong: {first_line}"
        # 0000 = address 0
        assert first_line[3:7] == "0000", f"address wrong: {first_line}"
        # 00 = data record type
        assert first_line[7:9] == "00", f"record type wrong: {first_line}"
        # D5 = the data byte
        assert first_line[9:11] == "D5", f"data byte wrong: {first_line}"

    def test_16_byte_record_has_byte_count_10(self) -> None:
        # 16 decimal = 0x10 hex
        binary = bytes(range(16))
        first_line = encode_hex(binary).splitlines()[0]
        assert first_line[1:3] == "10"

    def test_partial_last_record(self) -> None:
        # 18 bytes: first record = 16 bytes, second = 2 bytes
        binary = bytes(range(18))
        lines = [l for l in encode_hex(binary).splitlines() if l and not l.startswith(":00")]
        assert lines[0][1:3] == "10"  # first = 16 bytes
        assert lines[1][1:3] == "02"  # second = 2 bytes

    def test_addresses_are_sequential(self) -> None:
        binary = bytes(range(32))
        lines = [l for l in encode_hex(binary).splitlines() if l and not l.startswith(":00")]
        # First record: address 0x0000
        assert lines[0][3:7] == "0000"
        # Second record: address 0x0010 (16 decimal)
        assert lines[1][3:7] == "0010"

    def test_origin_offset_applied(self) -> None:
        binary = bytes([0xAB, 0xCD])
        hex_text = encode_hex(binary, origin=0x0100)
        first_line = hex_text.splitlines()[0]
        assert first_line[3:7] == "0100", f"expected address 0100, got {first_line[3:7]}"

    def test_origin_0x200(self) -> None:
        binary = bytes(range(16))
        hex_text = encode_hex(binary, origin=0x0200)
        first_line = hex_text.splitlines()[0]
        assert first_line[3:7] == "0200"

    def test_record_type_is_00_for_data(self) -> None:
        lines = [l for l in encode_hex(bytes([0x01])).splitlines() if l and not l.startswith(":00")]
        assert lines[0][7:9] == "00"

    def test_record_type_is_01_for_eof(self) -> None:
        hex_text = encode_hex(bytes([0x01]))
        eof_line = hex_text.strip().split("\n")[-1]
        assert eof_line[7:9] == "01"


# ---------------------------------------------------------------------------
# Round-trip fidelity
# ---------------------------------------------------------------------------


class TestRoundTrip:
    """encode_hex → decode_hex must be lossless."""

    def test_single_byte(self) -> None:
        binary = bytes([0xD5])
        origin, decoded = decode_hex(encode_hex(binary))
        assert origin == 0
        assert decoded == binary

    def test_two_bytes(self) -> None:
        binary = bytes([0xD5, 0x01])
        _, decoded = decode_hex(encode_hex(binary))
        assert decoded == binary

    def test_all_256_values(self) -> None:
        binary = bytes(range(256))
        _, decoded = decode_hex(encode_hex(binary))
        assert decoded == binary

    def test_with_nonzero_origin(self) -> None:
        binary = bytes([0xAB, 0xCD, 0xEF])
        origin, decoded = decode_hex(encode_hex(binary, origin=0x0300))
        assert origin == 0x0300
        assert decoded == binary

    def test_4kb_rom(self) -> None:
        # Maximum 4004 ROM size
        binary = bytes(i % 256 for i in range(4096))
        _, decoded = decode_hex(encode_hex(binary))
        assert decoded == binary

    def test_known_ldr5_hlt(self) -> None:
        # LDM 5 = 0xD5, HLT = 0x01
        binary = bytes([0xD5, 0x01])
        _, decoded = decode_hex(encode_hex(binary))
        assert decoded == binary


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


class TestEncodeErrors:
    def test_empty_binary_raises(self) -> None:
        with pytest.raises(ValueError, match="non-empty"):
            encode_hex(b"")

    def test_negative_origin_raises(self) -> None:
        with pytest.raises(ValueError, match="origin"):
            encode_hex(bytes([0x00]), origin=-1)

    def test_origin_too_large_raises(self) -> None:
        with pytest.raises(ValueError, match="origin"):
            encode_hex(bytes([0x00]), origin=0x10000)

    def test_overflow_raises(self) -> None:
        # 4097 bytes starting at 0xFFFF — overflows 16-bit space
        with pytest.raises(ValueError, match="overflow"):
            encode_hex(bytes(100), origin=0xFFFF)


class TestDecodeErrors:
    def test_missing_colon_raises(self) -> None:
        with pytest.raises(ValueError, match="expected ':'"):
            decode_hex("020000000000D5012A\n:00000001FF\n")

    def test_bad_hex_chars_raises(self) -> None:
        with pytest.raises(ValueError, match="invalid hex"):
            decode_hex(":0200000Z0000D5012A\n")

    def test_bad_checksum_raises(self) -> None:
        # Corrupt the last byte (checksum) of a valid record
        hex_text = encode_hex(bytes([0xD5, 0x01]))
        lines = hex_text.splitlines()
        # Flip the last two hex chars of the first line
        bad_line = lines[0][:-2] + "00"
        bad_hex = bad_line + "\n" + "\n".join(lines[1:]) + "\n"
        with pytest.raises(ValueError, match="checksum"):
            decode_hex(bad_hex)

    def test_empty_string_returns_empty(self) -> None:
        origin, data = decode_hex("")
        assert origin == 0
        assert data == b""
