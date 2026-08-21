"""Language-neutral raw RFC 1951 conformance and strict ZIP boundaries."""

from __future__ import annotations

import json
import random
import struct
import zlib
from pathlib import Path
from typing import Any, cast

import pytest

import coding_adventures_zip as zip_module
from coding_adventures_zip import (
    RAW_INFLATE_ERROR_CODES,
    RAW_INFLATE_MAX_OUTPUT,
    RawInflateError,
    ZipReader,
    crc32,
    raw_deflate,
    raw_inflate,
    raw_inflate_counted,
)
from coding_adventures_zip import _deflate_compress as compatibility_deflate
from coding_adventures_zip import _deflate_decompress as compatibility_inflate

FIXTURE_PATH = (
    Path(__file__).resolve().parents[4]
    / "specs"
    / "fixtures"
    / "zip-raw-rfc1951-v1"
    / "cases.json"
)
FIXTURE: dict[str, Any] = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))


def _from_hex(value: str) -> bytes:
    return bytes.fromhex(value)


def _expected_bytes(case: dict[str, Any]) -> bytes:
    output: dict[str, Any] = case["expected"]["output"]
    if "hex" in output:
        return _from_hex(cast(str, output["hex"]))
    return _from_hex(cast(str, output["repeat_hex"])) * cast(int, output["count"])


def _inflate_limit(case: dict[str, Any]) -> int:
    return cast(int, case.get("max_output", RAW_INFLATE_MAX_OUTPUT))


def test_closed_fixture_metadata() -> None:
    assert len(FIXTURE["cases"]) == 34
    assert FIXTURE["limits"] == {
        "default_max_output": RAW_INFLATE_MAX_OUTPUT,
        "hard_max_output": RAW_INFLATE_MAX_OUTPUT,
    }
    assert tuple(FIXTURE["error_ids"]) == RAW_INFLATE_ERROR_CODES


@pytest.mark.parametrize("case", FIXTURE["cases"], ids=lambda case: case["id"])
def test_closed_fixture(case: dict[str, Any]) -> None:
    expected: dict[str, Any] = case["expected"]
    operation = case["operation"]

    if operation == "inflate":
        encoded = _from_hex(case["input_hex"])
        result = raw_inflate_counted(encoded, max_output=_inflate_limit(case))
        assert result.output == _expected_bytes(case)
        assert result.bytes_consumed == expected["bytes_consumed"]
        assert raw_inflate(encoded, max_output=_inflate_limit(case)) == _expected_bytes(
            case
        )
    elif operation == "inflate-error":
        with pytest.raises(RawInflateError) as raised:
            raw_inflate_counted(
                _from_hex(case["input_hex"]), max_output=_inflate_limit(case)
            )
        assert raised.value.code == expected["error_id"]
        assert str(raised.value) == expected["error_id"]
        assert raised.value.args == (expected["error_id"],)
    elif operation == "deflate-interoperability":
        encoded = raw_deflate(_from_hex(case["input_hex"]))
        assert zlib.decompress(encoded, -zlib.MAX_WBITS) == _expected_bytes(case)
    elif operation == "crc32":
        checksum = int(case.get("initial_crc32_hex", "00000000"), 16)
        for chunk in case["chunks_hex"]:
            checksum = crc32(_from_hex(chunk), initial=checksum)
        assert f"{checksum:08x}" == expected["crc32_hex"]
    else:
        pytest.fail(f"unknown fixture operation: {operation}")


DYNAMIC = bytes.fromhex(
    "0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e"
)
DYNAMIC_OUTPUT = bytes.fromhex(
    "0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201"
)


def _raw_zip(
    name: str,
    compressed: bytes,
    uncompressed: bytes,
    *,
    declared_size: int | None = None,
) -> bytes:
    name_bytes = name.encode()
    size = len(uncompressed) if declared_size is None else declared_size
    checksum = crc32(uncompressed)
    local = (
        struct.pack(
            "<IHHHHHIIIHH",
            0x04034B50,
            20,
            0x0800,
            8,
            0,
            0,
            checksum,
            len(compressed),
            size,
            len(name_bytes),
            0,
        )
        + name_bytes
        + compressed
    )
    central_offset = len(local)
    central = (
        struct.pack(
            "<IHHHHHHIIIHHHHHII",
            0x02014B50,
            0x031E,
            20,
            0x0800,
            8,
            0,
            0,
            checksum,
            len(compressed),
            size,
            len(name_bytes),
            0,
            0,
            0,
            0,
            0,
            0,
        )
        + name_bytes
    )
    eocd = struct.pack(
        "<IHHHHIIH",
        0x06054B50,
        0,
        0,
        1,
        1,
        len(central),
        central_offset,
        0,
    )
    return local + central + eocd


def test_zip_reader_accepts_dynamic_raw_payload() -> None:
    reader = ZipReader(_raw_zip("dynamic.bin", DYNAMIC, DYNAMIC_OUTPUT))
    assert reader.read(reader.entries()[0]) == DYNAMIC_OUTPUT


def test_zip_reader_rejects_compressed_suffix_cavity() -> None:
    reader = ZipReader(_raw_zip("cavity.bin", DYNAMIC + b"\xde\xad", DYNAMIC_OUTPUT))
    with pytest.raises(ValueError, match="compressed payload contains trailing bytes"):
        reader.read(reader.entries()[0])


def test_zip_reader_rejects_declared_size_mismatch_without_trimming() -> None:
    reader = ZipReader(
        _raw_zip(
            "size.bin",
            DYNAMIC,
            DYNAMIC_OUTPUT,
            declared_size=len(DYNAMIC_OUTPUT) + 1,
        )
    )
    with pytest.raises(ValueError, match="uncompressed size does not match"):
        reader.read(reader.entries()[0])


def test_historical_deflate_wrappers_remain_compatible() -> None:
    expected = b"wrapper compatibility" * 16
    assert compatibility_inflate(compatibility_deflate(expected)) == expected


def test_full_32k_window_foreign_stream() -> None:
    prefix = bytes(((index * 73) + (index // 251)) & 0xFF for index in range(32768))
    expected = prefix + prefix
    compressor = zlib.compressobj(level=9, wbits=-zlib.MAX_WBITS)
    compressed = compressor.compress(expected) + compressor.flush()
    assert raw_inflate(compressed, max_output=len(expected)) == expected


def test_multi_megabyte_incompressible_deflate_uses_bounded_storage(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def reject_boxed_tokens(*args: object, **kwargs: object) -> None:
        raise AssertionError("large raw_deflate must not materialize LZSS tokens")

    monkeypatch.setattr(zip_module, "lzss_encode", reject_boxed_tokens)
    data = random.Random(0xC0DEC0DE).randbytes(2 * 1024 * 1024)
    compressed = raw_deflate(data)
    block_overhead = 5 * ((len(data) + 65_534) // 65_535)
    assert len(compressed) <= len(data) + block_overhead
    assert zlib.decompress(compressed, wbits=-zlib.MAX_WBITS) == data


@pytest.mark.parametrize("limit", [-1, RAW_INFLATE_MAX_OUTPUT + 1, 1.5, True])
def test_invalid_limit_fails_before_decoding(limit: object) -> None:
    with pytest.raises(RawInflateError) as raised:
        raw_inflate_counted(b"\x01\x00\x00\xff\xff", max_output=cast(Any, limit))
    assert raised.value.code == "invalid-output-limit"
