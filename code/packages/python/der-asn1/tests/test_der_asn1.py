"""Language-neutral portable conformance for typed DER values."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, cast

import pytest
from der_tlv import DerLimits

from der_asn1 import (
    Asn1Decoder,
    Asn1Element,
    Asn1Error,
    Asn1Limits,
    decode_bit_string,
    decode_boolean,
    decode_ia5_string,
    decode_implicit_ia5_string,
    decode_implicit_object_identifier,
    decode_implicit_octet_string,
    decode_integer,
    decode_null,
    decode_object_identifier,
    decode_octet_string,
)

FIXTURE_ROOT = Path(__file__).resolve().parents[4] / "specs/fixtures"
ASN1_FIXTURE = FIXTURE_ROOT / "der-asn1-v1/cases.json"
TLV_FIXTURE = FIXTURE_ROOT / "der-tlv-v1/cases.json"


def load(path: Path) -> dict[str, Any]:
    return cast(dict[str, Any], json.loads(path.read_text("utf-8")))


def materialize(segments: list[dict[str, Any]]) -> bytes:
    result = bytearray()
    for segment in segments:
        if "hex" in segment:
            result.extend(bytes.fromhex(segment["hex"]))
        else:
            result.extend(bytes.fromhex(segment["repeat_hex"]) * segment["count"])
    return bytes(result)


def configured_limits(document: dict[str, Any], case: dict[str, Any]) -> Asn1Limits:
    values = json.loads(json.dumps(document["defaults"]))
    override = case.get("limits", {})
    values.update({key: value for key, value in override.items() if key != "der"})
    values["der"].update(override.get("der", {}))
    if values["der"]["max_value_len"] == "host-max":
        values["der"]["max_value_len"] = sys.maxsize
    return Asn1Limits(der=DerLimits(**values.pop("der")), **values)


def tag(element: Asn1Element) -> dict[str, Any]:
    return {
        "class": element.tag.class_.value,
        "constructed": element.tag.constructed,
        "number": element.tag.number,
    }


def failure(error: Asn1Error, scope: str = "operation-input") -> dict[str, Any]:
    result: dict[str, Any] = {
        "outcome": "error",
        "error_id": error.kind.value,
        "offset": error.offset,
        "offset_scope": scope,
    }
    if error.framing_kind is not None:
        result["framing_error_id"] = error.framing_kind.value
    return result


def verify_upstream_exact(
    document: dict[str, Any], upstream: dict[str, Any], case: dict[str, Any]
) -> dict[str, str]:
    upstream_case = next(
        item for item in upstream["cases"] if item["id"] == case["der_tlv_case_id"]
    )
    data = materialize(upstream_case["input"])
    tlv_values = dict(upstream["defaults"])
    tlv_values.update(upstream_case.get("limits", {}))
    if tlv_values["max_value_len"] == "host-max":
        tlv_values["max_value_len"] = sys.maxsize
    decoder = Asn1Decoder(Asn1Limits(der=DerLimits(**tlv_values)))
    try:
        element = decoder.decode_exact(data)
        expected = upstream_case["expected"]
        assert expected["outcome"] == "element"
        assert tag(element) == expected["tag"]
        assert len(element.header) == expected["header_len"]
        assert len(element.encoded) == expected["encoded_len"]
        assert expected["remainder_offset"] == len(data)
    except Asn1Error as error:
        expected = upstream_case["expected"]
        assert expected["outcome"] == "error"
        assert error.kind.value == "framing"
        assert error.framing_kind is not None
        assert error.framing_kind.value == expected["error_id"]
        assert error.offset == expected["offset"]
    return {"outcome": "upstream"}


def primitive_result(
    operation: str,
    element: Asn1Element,
    limits: Asn1Limits,
    tag_number: int | None,
) -> dict[str, Any]:
    if operation == "decode-boolean":
        return {
            "outcome": "value",
            "boolean": decode_boolean(element),
            "elements_read": 1,
        }
    if operation in ("decode-integer", "integer-to-u64"):
        integer = decode_integer(element)
        result: dict[str, Any] = {
            "outcome": "value",
            "signed_hex": integer.signed_bytes.hex(),
            "negative": integer.is_negative,
        }
        if operation == "integer-to-u64":
            result["u64_decimal"] = str(integer.to_u64())
        return result
    if operation == "decode-bit-string":
        bit_string = decode_bit_string(element)
        return {
            "outcome": "value",
            "bytes_hex": bit_string.bytes.hex(),
            "unused_bits": bit_string.unused_bits,
            "bit_length": bit_string.bit_length,
        }
    if operation == "decode-octet-string":
        return {"outcome": "value", "bytes_hex": decode_octet_string(element).hex()}
    if operation == "decode-implicit-octet-string":
        assert tag_number is not None
        value = decode_implicit_octet_string(element, tag_number)
        return {"outcome": "value", "bytes_hex": value.hex()}
    if operation == "decode-ia5-string":
        return {"outcome": "value", "text": decode_ia5_string(element)}
    if operation == "decode-implicit-ia5-string":
        assert tag_number is not None
        return {
            "outcome": "value",
            "text": decode_implicit_ia5_string(element, tag_number),
        }
    if operation == "decode-null":
        decode_null(element)
        return {"outcome": "value"}
    if operation == "decode-object-identifier":
        oid = decode_object_identifier(element, limits)
    elif operation == "decode-implicit-object-identifier":
        assert tag_number is not None
        oid = decode_implicit_object_identifier(element, tag_number, limits)
    else:
        raise AssertionError(operation)
    return {
        "outcome": "value",
        "bytes_hex": oid.encoded.hex(),
        "arcs_decimal": [str(arc) for arc in oid.arcs],
        "arc_count": oid.arc_count,
    }


def cursor_result(
    case: dict[str, Any], decoder: Asn1Decoder, root: Asn1Element
) -> dict[str, Any]:
    cursor = decoder.sequence(root)
    original_length = len(cursor.remaining)
    events: list[dict[str, Any]] = []
    for action in case["actions"]:
        if action == "finish":
            try:
                cursor.finish()
                events.append({"outcome": "finished"})
            except Asn1Error as error:
                events.append(failure(error, "container-value"))
            continue
        active = decoder
        if action == "read-with-different-limits":
            active = Asn1Decoder(
                Asn1Limits(max_total_elements=decoder.limits.max_total_elements + 1)
            )
        try:
            child = cursor.read(active)
            events.append(
                {"outcome": "end"}
                if child is None
                else {"outcome": "value", "tag": tag(child), "depth": child.depth}
            )
        except Asn1Error as error:
            events.append(failure(error, "container-value"))
    return {
        "outcome": "value",
        "elements_read": decoder.elements_read,
        "remaining_offset": original_length - len(cursor.remaining),
        "events": events,
    }


def run_case(
    document: dict[str, Any], upstream: dict[str, Any], case: dict[str, Any]
) -> dict[str, Any]:
    if "der_tlv_case_id" in case:
        return verify_upstream_exact(document, upstream, case)
    limits = configured_limits(document, case)
    decoder = Asn1Decoder(limits)
    try:
        root = decoder.decode_exact(materialize(case["input"]))
        operation = case["operation"]
        if operation == "decode-exact":
            return {
                "outcome": "value",
                "tag": tag(root),
                "header_hex": root.header.hex(),
                "value_hex": root.value.hex(),
                "encoded_hex": root.encoded.hex(),
                "depth": root.depth,
                "elements_read": decoder.elements_read,
            }
        if operation == "cursor-script":
            return cursor_result(case, decoder, root)
        if operation in ("sequence", "set"):
            cursor = (
                decoder.sequence(root) if operation == "sequence" else decoder.set(root)
            )
            return {
                "outcome": "value",
                "elements_read": decoder.elements_read,
                "remaining_offset": len(root.value) - len(cursor.remaining),
            }
        if operation == "explicit":
            child = decoder.explicit(root, case["tag_number"])
            return {
                "outcome": "value",
                "tag": tag(child),
                "value_hex": child.value.hex(),
                "depth": child.depth,
                "elements_read": decoder.elements_read,
            }
        return primitive_result(operation, root, limits, case.get("tag_number"))
    except Asn1Error as error:
        scope = (
            "container-value"
            if case["operation"] == "explicit" and error.kind.value == "framing"
            else "operation-input"
        )
        return failure(error, scope)


DOCUMENT = load(ASN1_FIXTURE)
UPSTREAM = load(TLV_FIXTURE)


@pytest.mark.parametrize("case", DOCUMENT["cases"], ids=lambda case: case["id"])
def test_portable_conformance(case: dict[str, Any]) -> None:
    actual = run_case(DOCUMENT, UPSTREAM, case)
    assert actual == case["expected"]
    if redacted := case.get("redacted_input_hex"):
        assert redacted not in json.dumps(actual)


def test_limit_validation_and_oid_equality() -> None:
    with pytest.raises(ValueError):
        Asn1Limits(max_depth=-1)
    decoder = Asn1Decoder()
    oid = decode_object_identifier(decoder.decode_exact(bytes.fromhex("06032a0304")))
    assert oid.equals([1, 2, 3, 4])
    assert not oid.equals([1, 2, 3])
