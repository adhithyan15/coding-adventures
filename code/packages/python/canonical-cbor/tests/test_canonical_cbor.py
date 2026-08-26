"""CBR01 portable and adversarial tests for the Python implementation."""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import pytest

from canonical_cbor import (
    ERROR_IDS,
    MAX_ENCODED_BYTES,
    MAX_NESTING_DEPTH,
    NULL,
    CborArray,
    CborBoolean,
    CborByteString,
    CborError,
    CborMap,
    CborMapEntry,
    CborNegative,
    CborTag,
    CborText,
    CborUnsigned,
    decode,
    encode_checked,
    encode_into_checked,
)

REPO_CODE = Path(__file__).resolve().parents[4]
FIXTURE_PATH = REPO_CODE / "specs" / "fixtures" / "canonical-cbor-v1" / "cases.json"
FIXTURE = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))


def _nested_array(depth: int) -> object:
    value: object = NULL
    for _ in range(depth):
        value = CborArray((value,))
    return value


def _nested_wire(depth: int) -> bytes:
    return bytes([0x81]) * depth + bytes([0xF6])


def _generated_value(specification: str) -> object:
    parts = specification.split(":")
    if len(parts) == 2 and parts[0] == "nested-array":
        return _nested_array(int(parts[1]))
    if len(parts) == 3 and parts[0] == "bytes-repeat":
        length = int(parts[1])
        byte = bytes.fromhex(parts[2])
        assert len(byte) == 1
        return CborByteString(byte * length)
    raise AssertionError(f"generated value escaped the closed grammar: {specification}")


def _generated_wire(specification: str) -> bytes:
    parts = specification.split(":")
    if len(parts) == 3 and parts[:2] == ["wire", "nested-array"]:
        return _nested_wire(int(parts[2]))
    if len(parts) == 4 and parts[:2] == ["wire", "bytes-repeat"]:
        length = int(parts[2])
        byte = bytes.fromhex(parts[3])
        assert len(byte) == 1
        return encode_checked(CborByteString(byte * length))
    raise AssertionError(f"generated wire escaped the closed grammar: {specification}")


def _fixture_decode_wire(specification: str) -> bytes:
    parts = specification.split(":")
    if len(parts) == 2 and parts[0] == "nested-array-wire":
        return _nested_wire(int(parts[1]))
    if ":" not in specification:
        return bytes.fromhex(specification)
    raise AssertionError(
        f"fixture decode input escaped the closed grammar: {specification}"
    )


def _assert_error(expected: str, operation: object) -> None:
    assert callable(operation)
    with pytest.raises(CborError) as caught:
        operation()
    assert caught.value.error_id == expected
    assert str(caught.value).startswith("canonical-cbor:")
    assert str(caught.value) == CborError(expected).args[0]


@pytest.mark.parametrize("case", FIXTURE["cases"], ids=lambda case: case["id"])
def test_language_neutral_fixture(case: dict[str, str]) -> None:
    """Execute every operation in the closed 55-case CBR01 corpus."""

    operation = case["operation"]
    if operation == "round-trip":
        wire = bytes.fromhex(case["input"])
        assert encode_checked(decode(wire)) == bytes.fromhex(case["expected"])
    elif operation == "decode-error":
        wire = _fixture_decode_wire(case["input"])
        _assert_error(case["expected"], lambda: decode(wire))
    elif operation == "encode-map":
        entries = tuple(
            CborMapEntry(decode(bytes.fromhex(key)), decode(bytes.fromhex(value)))
            for fragment in case["input"].split(";")
            for key, value in [fragment.split("=", 1)]
        )
        assert encode_checked(CborMap(entries)) == bytes.fromhex(case["expected"])
    elif operation == "generated-round-trip":
        value = _generated_value(case["input"])
        wire = _generated_wire(case["expected"])
        assert encode_checked(value) == wire
        assert encode_checked(decode(wire)) == wire
    elif operation == "encode-error":
        if case["input"] == "duplicate-map-key":
            value = CborMap(
                (
                    CborMapEntry(CborText("same"), CborUnsigned(1)),
                    CborMapEntry(CborText("same"), CborUnsigned(2)),
                )
            )
        else:
            value = _generated_value(case["input"])
        _assert_error(case["expected"], lambda: encode_checked(value))
    else:
        raise AssertionError(
            f"fixture operation escaped its closed grammar: {operation}"
        )


def test_fixture_contract_and_static_error_taxonomy() -> None:
    assert FIXTURE["profile"] == "rfc8949-section-4.2.3-length-first"
    assert FIXTURE["limits"] == {
        "max_nesting_depth": MAX_NESTING_DEPTH,
        "max_encoded_bytes": MAX_ENCODED_BYTES,
    }
    assert tuple(FIXTURE["error_ids"]) == ERROR_IDS
    assert len(FIXTURE["cases"]) == 55
    messages = set()
    for error_id in ERROR_IDS:
        message = str(CborError(error_id))
        assert message.startswith("canonical-cbor:")
        messages.add(message)
    assert len(messages) == 14
    with pytest.raises(ValueError, match="unknown error identifier"):
        CborError("not-in-the-contract")


def test_complete_value_model_and_uint64_domain() -> None:
    value = CborArray(
        (
            CborUnsigned((1 << 64) - 1),
            CborNegative((1 << 64) - 1),
            CborByteString(b"\x00\xff"),
            CborText("snowman: \u2603"),
            CborMap((CborMapEntry(CborUnsigned(0), CborBoolean(False)),)),
            CborTag((1 << 64) - 1, CborBoolean(True)),
            NULL,
        )
    )
    assert decode(encode_checked(value)) == value

    for constructor in (CborUnsigned, CborNegative):
        with pytest.raises(ValueError, match="unsigned 64-bit"):
            constructor(-1)
        with pytest.raises(ValueError, match="unsigned 64-bit"):
            constructor(1 << 64)
        with pytest.raises(TypeError, match="integer"):
            constructor(True)
    with pytest.raises(ValueError, match="unsigned 64-bit"):
        CborTag(-1, NULL)
    with pytest.raises(TypeError, match="CborValue"):
        CborTag(0, object())  # type: ignore[arg-type]


def test_values_defensively_own_bytes_and_collections() -> None:
    source = bytearray(b"abc")
    byte_string = CborByteString(source)
    source[0] = ord("z")
    assert byte_string.value == b"abc"

    values = [CborUnsigned(1)]
    array = CborArray(values)
    values.append(CborUnsigned(2))
    assert array.items == (CborUnsigned(1),)

    entries = [CborMapEntry(CborUnsigned(0), NULL)]
    mapping = CborMap(entries)
    entries.clear()
    assert mapping.entries == (CborMapEntry(CborUnsigned(0), NULL),)


def test_constructor_and_runtime_boundaries_are_exact() -> None:
    with pytest.raises(TypeError, match="byte string"):
        CborByteString("abc")  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="text"):
        CborText(b"abc")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="Unicode scalar"):
        CborText("\ud800")
    with pytest.raises(TypeError, match="CborValue"):
        CborArray((object(),))  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="CborValue"):
        CborArray(1)
    with pytest.raises(TypeError, match="CborMapEntry"):
        CborMap((object(),))  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="CborMapEntry"):
        CborMap(1)

    class BombIterable:
        def __iter__(self) -> object:
            raise AssertionError("constructor consumed hostile iterable")

    with pytest.raises(TypeError, match="list or tuple"):
        CborArray(BombIterable())
    with pytest.raises(TypeError, match="list or tuple"):
        CborMap(BombIterable())
    with pytest.raises(TypeError, match="CborValue"):
        CborMapEntry(object(), NULL)  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="boolean"):
        CborBoolean(1)  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="exact CborValue"):
        encode_checked(object())  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="bytes-like"):
        decode("00")  # type: ignore[arg-type]


def test_checked_append_is_atomic_on_every_failure_path() -> None:
    destination = bytearray(b"prefix")
    encode_into_checked(CborUnsigned(24), destination)
    assert destination == b"prefix\x18\x18"

    invalid = _nested_array(MAX_NESTING_DEPTH + 1)
    before = bytes(destination)
    _assert_error("encode-too-deep", lambda: encode_into_checked(invalid, destination))
    assert bytes(destination) == before

    duplicate = CborMap(
        (
            CborMapEntry(CborUnsigned(0), NULL),
            CborMapEntry(CborUnsigned(0), CborBoolean(True)),
        )
    )
    _assert_error(
        "duplicate-map-key", lambda: encode_into_checked(duplicate, destination)
    )
    assert bytes(destination) == before
    with pytest.raises(TypeError, match="bytearray"):
        encode_into_checked(NULL, [])  # type: ignore[arg-type]

    class HostileBytearray(bytearray):
        def extend(self, value: object) -> None:
            self[0] = ord("X")
            super().extend(bytes(value)[:1])  # type: ignore[arg-type]
            raise MemoryError("synthetic host allocation failure")

    hostile = HostileBytearray(b"kept")
    with pytest.raises(TypeError, match="exact bytearray"):
        encode_into_checked(CborUnsigned(24), hostile)
    assert hostile == b"kept"


def test_encoder_revalidates_frozen_values_at_the_public_boundary() -> None:
    unsigned = CborUnsigned(1)
    object.__setattr__(unsigned, "value", -1)
    with pytest.raises(ValueError, match="unsigned 64-bit"):
        encode_checked(unsigned)

    array = CborArray((NULL,))
    object.__setattr__(array, "items", (object(),))
    with pytest.raises(TypeError, match="CborValue"):
        encode_checked(array)

    byte_string = CborByteString(b"safe")
    object.__setattr__(byte_string, "value", bytearray(b"unsafe"))
    with pytest.raises(TypeError, match="invalid byte string"):
        encode_checked(byte_string)

    tag = CborTag(1, NULL)
    object.__setattr__(tag, "value", object())
    with pytest.raises(TypeError, match="CborValue"):
        encode_checked(tag)

    boolean = CborBoolean(True)
    object.__setattr__(boolean, "value", 1)
    with pytest.raises(TypeError, match="boolean"):
        encode_checked(boolean)

    mapping = CborMap((CborMapEntry(CborUnsigned(0), NULL),))
    object.__setattr__(mapping, "entries", (object(),))
    with pytest.raises(TypeError, match="CborMapEntry"):
        encode_checked(mapping)

    entry = CborMapEntry(CborUnsigned(0), NULL)
    object.__setattr__(entry, "value", object())
    mapping = CborMap((CborMapEntry(CborUnsigned(0), NULL),))
    object.__setattr__(mapping, "entries", (entry,))
    with pytest.raises(TypeError, match="CborValue"):
        encode_checked(mapping)


def test_subclasses_cannot_override_value_invariants() -> None:
    class PretendUnsigned(CborUnsigned):
        pass

    with pytest.raises(TypeError, match="exact CborValue"):
        encode_checked(PretendUnsigned(1))


def test_length_first_map_order_uses_complete_unsigned_key_bytes() -> None:
    value = CborMap(
        (
            CborMapEntry(CborText("b"), CborUnsigned(0)),
            CborMapEntry(CborUnsigned(24), CborUnsigned(1)),
            CborMapEntry(CborText(""), CborUnsigned(2)),
            CborMapEntry(CborUnsigned(0), CborUnsigned(3)),
            CborMapEntry(CborText("a"), CborUnsigned(4)),
        )
    )
    assert encode_checked(value).hex() == "a500036002181801616104616200"


def test_text_and_length_preflight_cover_every_reachable_header_width() -> None:
    text = CborText("ASCII, caf\u00e9, emoji \U0001f600")
    assert decode(encode_checked(text)) == text
    for length in (24, 256, 65_536):
        value = CborByteString(b"x" * length)
        assert decode(encode_checked(value)) == value


def test_decode_accepts_bytes_like_inputs_and_returns_immutable_payloads() -> None:
    source = bytearray.fromhex("43010203")
    decoded = decode(memoryview(source))
    source[:] = b"\x00" * len(source)
    assert decoded == CborByteString(b"\x01\x02\x03")
    with pytest.raises(TypeError):
        decoded.value[0] = 9  # type: ignore[index,union-attr]


def test_invalid_utf8_error_chain_cannot_retain_the_rejected_payload() -> None:
    with pytest.raises(CborError) as caught:
        decode(bytes.fromhex("63ff6162"))
    error = caught.value
    assert error.error_id == "invalid-utf8"
    assert error.__cause__ is None
    assert error.__context__ is None
    rendered = "".join(
        traceback.format_exception(type(error), error, error.__traceback__)
    )
    assert "0xff" not in rendered
    assert "position" not in rendered
    assert "b'\\xffab'" not in rendered


def test_fixture_generated_grammar_rejects_suffixes() -> None:
    with pytest.raises(AssertionError, match="closed grammar"):
        _generated_value("nested-array:1:ignored")
    with pytest.raises(AssertionError, match="closed grammar"):
        _generated_wire("wire:nested-array:1:ignored")
    with pytest.raises(AssertionError, match="closed grammar"):
        _fixture_decode_wire("nested-array-wire:129:ignored")
