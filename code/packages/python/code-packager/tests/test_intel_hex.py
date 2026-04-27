"""Tests for IntelHexPackager."""

from __future__ import annotations

import pytest

from code_packager import CodeArtifact, IntelHexPackager, Target, UnsupportedTargetError


def _decode_hex(hex_bytes: bytes) -> bytes:
    """Parse Intel HEX bytes back to binary (for round-trip tests)."""
    result = bytearray()
    for line in hex_bytes.decode("ascii").splitlines():
        line = line.strip()
        if not line.startswith(":"):
            continue
        byte_count = int(line[1:3], 16)
        record_type = int(line[7:9], 16)
        if record_type == 0x00:  # data record
            data_hex = line[9:9 + byte_count * 2]
            result.extend(bytes.fromhex(data_hex))
    return bytes(result)


class TestIntelHexPackager:
    def setup_method(self):
        self.p = IntelHexPackager()

    def test_round_trip(self, hex_artifact):
        result = self.p.pack(hex_artifact)
        decoded = _decode_hex(result)
        assert decoded == hex_artifact.native_bytes

    def test_returns_bytes(self, hex_artifact):
        result = self.p.pack(hex_artifact)
        assert isinstance(result, bytes)

    def test_starts_with_colon(self, hex_artifact):
        result = self.p.pack(hex_artifact)
        assert result.startswith(b":")

    def test_ends_with_eof_record(self, hex_artifact):
        result = self.p.pack(hex_artifact)
        text = result.decode("ascii")
        assert ":00000001FF" in text

    def test_intel_8008_target(self):
        a = CodeArtifact(
            native_bytes=b"\x01\x02\x03",
            entry_point=0,
            target=Target.intel_8008(),
        )
        result = self.p.pack(a)
        assert _decode_hex(result) == b"\x01\x02\x03"

    def test_origin_metadata(self):
        a = CodeArtifact(
            native_bytes=b"\xAB",
            entry_point=0,
            target=Target.intel_4004(),
            metadata={"origin": 0x100},
        )
        result = self.p.pack(a)
        text = result.decode("ascii")
        # The address field in the first data record should reflect origin 0x100
        first_record = [l for l in text.splitlines() if l.startswith(":")][0]
        addr = int(first_record[3:7], 16)
        assert addr == 0x100

    def test_file_extension(self):
        assert self.p.file_extension(Target.intel_4004()) == ".hex"

    def test_wrong_format_raises(self):
        a = CodeArtifact(
            native_bytes=b"\x90",
            entry_point=0,
            target=Target.raw(),
        )
        with pytest.raises(UnsupportedTargetError):
            self.p.pack(a)

    def test_single_byte(self):
        a = CodeArtifact(native_bytes=b"\xFF", entry_point=0, target=Target.intel_4004())
        result = self.p.pack(a)
        assert b":00000001FF" in result
        decoded = _decode_hex(result)
        assert decoded == b"\xFF"
