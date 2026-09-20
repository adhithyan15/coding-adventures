"""Validate the closed, language-neutral DER TLV v1 fixture corpus."""

from __future__ import annotations

import json
import sys
import unittest
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/der-tlv-v1"
EXPECTED_ERRORS = [
    "empty-input",
    "truncated-high-tag",
    "truncated-length",
    "truncated-value",
    "end-of-contents",
    "non-minimal-tag",
    "tag-overflow",
    "indefinite-length",
    "reserved-length",
    "non-minimal-length",
    "length-too-wide",
    "length-host-overflow",
    "input-limit-exceeded",
    "value-limit-exceeded",
    "element-limit-exceeded",
    "tag-limit-exceeded",
    "trailing-data",
]


def load_json(path: Path) -> dict[str, Any]:
    """Load JSON while rejecting duplicate keys."""

    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    return json.loads(path.read_text("utf-8"), object_pairs_hook=reject_duplicates)


def materialize(segments: list[dict[str, Any]]) -> bytes:
    output = bytearray()
    for segment in segments:
        if "hex" in segment:
            output.extend(bytes.fromhex(segment["hex"]))
        else:
            output.extend(bytes.fromhex(segment["repeat_hex"]) * segment["count"])
    assert len(output) <= 1_048_576
    return bytes(output)


class FramingFailure(Exception):
    def __init__(self, error_id: str, offset: int) -> None:
        super().__init__(error_id, offset)
        self.error_id = error_id
        self.offset = offset


@dataclass(frozen=True)
class Element:
    element_offset: int
    class_name: str
    constructed: bool
    number: int
    header_len: int
    encoded_len: int


def case_limits(document: dict[str, Any], case: dict[str, Any]) -> dict[str, int]:
    limits = dict(document["defaults"])
    limits.update(case.get("limits", {}))
    if limits["max_value_len"] == "host-max":
        limits["max_value_len"] = sys.maxsize
    return limits


def decode_one(data: bytes, limits: dict[str, int], base: int = 0) -> Element:
    if len(data) > limits["max_input_len"]:
        raise FramingFailure("input-limit-exceeded", base)
    if not data:
        raise FramingFailure("empty-input", base)

    first = data[0]
    classes = ["universal", "application", "context-specific", "private"]
    class_name = classes[first >> 6]
    constructed = bool(first & 0x20)
    low = first & 0x1F
    if low != 0x1F:
        number = low
        identifier_len = 1
        if number > limits["max_tag_number"]:
            raise FramingFailure("tag-limit-exceeded", base)
    else:
        number = 0
        index = 1
        while True:
            if index >= len(data):
                raise FramingFailure("truncated-high-tag", base + index)
            octet = data[index]
            payload = octet & 0x7F
            if index == 1 and payload == 0:
                raise FramingFailure("non-minimal-tag", base + index)
            candidate = number * 128 + payload
            if candidate > 0xFFFF_FFFF:
                raise FramingFailure("tag-overflow", base + index)
            number = candidate
            if number > limits["max_tag_number"]:
                raise FramingFailure("tag-limit-exceeded", base + index)
            index += 1
            if not octet & 0x80:
                break
        if number < 31:
            raise FramingFailure("non-minimal-tag", base)
        identifier_len = index

    if class_name == "universal" and number == 0:
        raise FramingFailure("end-of-contents", base)

    length_offset = base + identifier_len
    if identifier_len >= len(data):
        raise FramingFailure("truncated-length", length_offset)
    first_length = data[identifier_len]
    if first_length < 0x80:
        value_len = first_length
        length_len = 1
    else:
        if first_length == 0x80:
            raise FramingFailure("indefinite-length", length_offset)
        if first_length == 0xFF:
            raise FramingFailure("reserved-length", length_offset)
        count = first_length & 0x7F
        if count > 8:
            raise FramingFailure("length-too-wide", length_offset)
        start = identifier_len + 1
        end = start + count
        if end > len(data):
            raise FramingFailure("truncated-length", base + len(data))
        octets = data[start:end]
        if octets[0] == 0:
            raise FramingFailure("non-minimal-length", base + start)
        value_len = int.from_bytes(octets, "big")
        if value_len < 128:
            raise FramingFailure("non-minimal-length", length_offset)
        if value_len > sys.maxsize:
            raise FramingFailure("length-host-overflow", length_offset)
        length_len = 1 + count

    if value_len > limits["max_value_len"]:
        raise FramingFailure("value-limit-exceeded", length_offset)
    header_len = identifier_len + length_len
    if value_len > sys.maxsize - header_len:
        raise FramingFailure("length-host-overflow", length_offset)
    encoded_len = header_len + value_len
    if encoded_len > len(data):
        raise FramingFailure("truncated-value", base + len(data))
    return Element(
        element_offset=base,
        class_name=class_name,
        constructed=constructed,
        number=number,
        header_len=header_len,
        encoded_len=encoded_len,
    )


def projected_element(element: Element) -> dict[str, Any]:
    return {
        "outcome": "element",
        "element_offset": element.element_offset,
        "tag": {
            "class": element.class_name,
            "constructed": element.constructed,
            "number": element.number,
        },
        "header_len": element.header_len,
        "encoded_len": element.encoded_len,
        "remainder_offset": element.element_offset + element.encoded_len,
    }


def projected_error(error: FramingFailure) -> dict[str, Any]:
    return {"outcome": "error", "error_id": error.error_id, "offset": error.offset}


def run_decode_case(document: dict[str, Any], case: dict[str, Any]) -> dict[str, Any]:
    data = materialize(case["input"])
    limits = case_limits(document, case)
    try:
        element = decode_one(data, limits)
        if case["operation"] == "decode-exact" and element.encoded_len != len(data):
            raise FramingFailure("trailing-data", element.encoded_len)
        return projected_element(element)
    except FramingFailure as error:
        return projected_error(error)


def run_cursor_case(document: dict[str, Any], case: dict[str, Any]) -> dict[str, Any]:
    data = materialize(case["input"])
    limits = case_limits(document, case)
    if len(data) > limits["max_input_len"]:
        raise FramingFailure("input-limit-exceeded", 0)
    offset = 0
    elements_read = 0
    events: list[dict[str, Any]] = []
    for action in case["actions"]:
        if action == "finish":
            if offset == len(data):
                events.append({"outcome": "finished"})
            else:
                events.append(projected_error(FramingFailure("trailing-data", offset)))
            continue
        if offset == len(data):
            events.append({"outcome": "end"})
            continue
        if elements_read >= limits["max_elements"]:
            events.append(
                projected_error(FramingFailure("element-limit-exceeded", offset))
            )
            continue
        try:
            element = decode_one(data[offset:], limits, offset)
        except FramingFailure as error:
            events.append(projected_error(error))
            continue
        offset += element.encoded_len
        elements_read += 1
        events.append(projected_element(element))
    return {
        "events": events,
        "elements_read": elements_read,
        "remaining_offset": offset,
    }


class DerTlvFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load_json(FIXTURE_ROOT / "schema.json")
        cls.document = load_json(FIXTURE_ROOT / "cases.json")

    def test_schema_is_closed_and_document_validates(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        Draft202012Validator(self.schema).validate(self.document)

    def test_profile_is_complete_and_ids_are_unique(self) -> None:
        self.assertEqual(self.document["schema_version"], 1)
        self.assertEqual(self.document["profile"], "x690-der-tlv-framing-v1")
        self.assertEqual(self.document["error_ids"], EXPECTED_ERRORS)
        cases = self.document["cases"]
        self.assertGreaterEqual(len(cases), 50)
        ids = [case["id"] for case in cases]
        self.assertEqual(len(ids), len(set(ids)))

    def test_independent_oracle_matches_every_normalized_case(self) -> None:
        for case in self.document["cases"]:
            with self.subTest(case=case["id"]):
                if case["operation"] == "cursor":
                    actual = run_cursor_case(self.document, case)
                else:
                    actual = run_decode_case(self.document, case)
                self.assertEqual(actual, case["expected"])

    def test_redaction_case_names_payload_without_putting_it_in_error(self) -> None:
        case = next(
            case
            for case in self.document["cases"]
            if case["id"] == "der-tlv-v1-redacted-hostile-input"
        )
        actual = run_decode_case(self.document, case)
        self.assertNotIn(case["redacted_input_hex"], json.dumps(actual))


if __name__ == "__main__":
    unittest.main()
