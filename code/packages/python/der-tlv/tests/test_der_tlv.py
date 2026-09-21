"""Portable and API-focused tests for DER TLV framing."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, cast

import pytest

from der_tlv import (
    DerCursor,
    DerElement,
    DerError,
    DerLimits,
    decode_exact,
    decode_one,
)

FIXTURE = Path(__file__).resolve().parents[4] / "specs/fixtures/der-tlv-v1/cases.json"


def corpus() -> dict[str, Any]:
    return cast(dict[str, Any], json.loads(FIXTURE.read_text("utf-8")))


def materialize(segments: list[dict[str, Any]]) -> bytes:
    result = bytearray()
    for segment in segments:
        if "hex" in segment:
            result.extend(bytes.fromhex(segment["hex"]))
        else:
            result.extend(bytes.fromhex(segment["repeat_hex"]) * segment["count"])
    return bytes(result)


def limits(document: dict[str, Any], case: dict[str, Any]) -> DerLimits:
    values = dict(document["defaults"])
    values.update(case.get("limits", {}))
    if values["max_value_len"] == "host-max":
        values["max_value_len"] = sys.maxsize
    return DerLimits(**values)


def element_projection(element: DerElement, element_offset: int) -> dict[str, Any]:
    return {
        "outcome": "element",
        "element_offset": element_offset,
        "tag": {
            "class": element.tag.class_.value,
            "constructed": element.tag.constructed,
            "number": element.tag.number,
        },
        "header_len": len(element.header),
        "encoded_len": len(element.encoded),
        "remainder_offset": element_offset + len(element.encoded),
    }


def error_projection(error: DerError) -> dict[str, Any]:
    return {"outcome": "error", "error_id": error.kind.value, "offset": error.offset}


def run_decode(
    case: dict[str, Any], data: bytes, configured: DerLimits
) -> dict[str, Any]:
    try:
        if case["operation"] == "decode-one":
            element, remainder = decode_one(data, configured)
            projection = element_projection(element, 0)
            assert projection["remainder_offset"] == len(data) - len(remainder)
            return projection
        return element_projection(decode_exact(data, configured), 0)
    except DerError as error:
        return error_projection(error)


def run_cursor(
    case: dict[str, Any], data: bytes, configured: DerLimits
) -> dict[str, Any]:
    cursor = DerCursor(data, configured)
    events: list[dict[str, Any]] = []
    for action in case["actions"]:
        if action == "finish":
            try:
                cursor.finish()
                events.append({"outcome": "finished"})
            except DerError as error:
                events.append(error_projection(error))
            continue
        offset = len(data) - len(cursor.remaining)
        try:
            element = cursor.read()
            events.append(
                {"outcome": "end"}
                if element is None
                else element_projection(element, offset)
            )
        except DerError as error:
            events.append(error_projection(error))
    return {
        "events": events,
        "elements_read": cursor.elements_read,
        "remaining_offset": len(data) - len(cursor.remaining),
    }


@pytest.mark.parametrize("case", corpus()["cases"], ids=lambda case: case["id"])
def test_portable_conformance(case: dict[str, Any]) -> None:
    document = corpus()
    data = materialize(case["input"])
    configured = limits(document, case)
    actual = (
        run_cursor(case, data, configured)
        if case["operation"] == "cursor"
        else run_decode(case, data, configured)
    )
    assert actual == case["expected"]
    if redacted := case.get("redacted_input_hex"):
        assert redacted not in json.dumps(actual)


def test_views_share_the_callers_mutable_buffer() -> None:
    data = bytearray([0x04, 0x01, 0x2A])
    element = decode_exact(data)
    data[2] = 0x7F
    assert element.value.tobytes() == b"\x7f"


def test_negative_limits_and_wide_tag_limit_are_rejected() -> None:
    with pytest.raises(ValueError):
        DerLimits(max_elements=-1)
    with pytest.raises(ValueError):
        DerLimits(max_tag_number=0x1_0000_0000)
