"""Validate and independently execute the typed DER value v1 corpus."""

from __future__ import annotations

import json
import unittest
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator
from referencing import Registry, Resource
from test_der_tlv_fixtures import (
    Element,
    FramingFailure,
    decode_one,
    load_json,
    materialize,
    run_decode_case,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/der-asn1-v1"
TLV_ROOT = REPO_ROOT / "code/specs/fixtures/der-tlv-v1"

EXPECTED_ERRORS = [
    "framing",
    "unexpected-tag",
    "decoder-limit-mismatch",
    "depth-limit-exceeded",
    "element-limit-exceeded",
    "invalid-boolean-length",
    "invalid-boolean-value",
    "empty-integer",
    "non-minimal-integer",
    "negative-integer",
    "integer-overflow",
    "missing-unused-bit-count",
    "invalid-unused-bit-count",
    "non-zero-bit-padding",
    "bit-length-overflow",
    "non-empty-null",
    "non-ascii-ia5-string",
    "empty-object-identifier",
    "unterminated-object-identifier",
    "non-minimal-object-identifier",
    "object-identifier-overflow",
    "oid-arc-limit-exceeded",
]


@dataclass(frozen=True)
class TypedFailure(Exception):
    error_id: str
    offset: int
    scope: str = "operation-input"
    framing_error_id: str | None = None


def schema_registry(tlv_schema: dict[str, Any]) -> Registry:
    """Register both the canonical and relative-resolution upstream names."""

    resource = Resource.from_contents(tlv_schema)
    return Registry().with_resources(
        [
            ("https://coding-adventures.dev/schemas/der-tlv-v1.json", resource),
            ("https://coding-adventures.dev/der-tlv-v1/schema.json", resource),
        ]
    )


def limits_for(document: dict[str, Any], case: dict[str, Any]) -> dict[str, Any]:
    limits = json.loads(json.dumps(document["defaults"]))
    override = case.get("limits", {})
    limits.update({key: value for key, value in override.items() if key != "der"})
    limits["der"].update(override.get("der", {}))
    return limits


def root_element(
    document: dict[str, Any], case: dict[str, Any]
) -> tuple[bytes, Element, dict[str, Any], int]:
    data = materialize(case["input"])
    limits = limits_for(document, case)
    if limits["max_depth"] == 0:
        raise TypedFailure("depth-limit-exceeded", 0)
    if limits["max_total_elements"] == 0:
        raise TypedFailure("element-limit-exceeded", 0)
    try:
        element = decode_one(data, limits["der"])
        if element.encoded_len != len(data):
            raise FramingFailure("trailing-data", element.encoded_len)
    except FramingFailure as error:
        raise TypedFailure(
            "framing", error.offset, framing_error_id=error.error_id
        ) from error
    return data, element, limits, 1


def tag_result(element: Element) -> dict[str, Any]:
    return {
        "class": element.class_name,
        "constructed": element.constructed,
        "number": element.number,
    }


def expect_tag(
    element: Element, class_name: str, constructed: bool, number: int
) -> None:
    if (
        element.class_name != class_name
        or element.constructed != constructed
        or element.number != number
    ):
        raise TypedFailure("unexpected-tag", 0)


def value_bytes(data: bytes, element: Element) -> bytes:
    return data[element.header_len : element.encoded_len]


def parse_oid(encoded: bytes, value_offset: int, max_arcs: int) -> list[int]:
    if not encoded:
        raise TypedFailure("empty-object-identifier", value_offset)

    def subidentifier(start: int) -> tuple[int, int]:
        if encoded[start] == 0x80:
            raise TypedFailure("non-minimal-object-identifier", value_offset + start)
        value = 0
        offset = start
        while True:
            if offset >= len(encoded):
                raise TypedFailure(
                    "unterminated-object-identifier", value_offset + offset
                )
            octet = encoded[offset]
            candidate = value * 128 + (octet & 0x7F)
            if candidate > 0xFFFF_FFFF_FFFF_FFFF:
                raise TypedFailure("object-identifier-overflow", value_offset + offset)
            value = candidate
            offset += 1
            if octet & 0x80 == 0:
                return value, offset

    combined, offset = subidentifier(0)
    if combined < 40:
        arcs = [0, combined]
    elif combined < 80:
        arcs = [1, combined - 40]
    else:
        arcs = [2, combined - 80]
    if len(arcs) > max_arcs:
        raise TypedFailure("oid-arc-limit-exceeded", value_offset)
    while offset < len(encoded):
        arc_start = offset
        arc, offset = subidentifier(offset)
        arcs.append(arc)
        if len(arcs) > max_arcs:
            raise TypedFailure("oid-arc-limit-exceeded", value_offset + arc_start)
    return arcs


def value_result(**values: Any) -> dict[str, Any]:
    return {"outcome": "value", **values}


def failure_result(error: TypedFailure) -> dict[str, Any]:
    result: dict[str, Any] = {
        "outcome": "error",
        "error_id": error.error_id,
        "offset": error.offset,
        "offset_scope": error.scope,
    }
    if error.framing_error_id is not None:
        result["framing_error_id"] = error.framing_error_id
    return result


def typed_primitive_result(
    operation: str,
    data: bytes,
    element: Element,
    limits: dict[str, Any],
    tag_number: int | None,
) -> dict[str, Any]:
    value = value_bytes(data, element)
    value_offset = element.header_len
    universal_tags = {
        "decode-boolean": 1,
        "decode-integer": 2,
        "integer-to-u64": 2,
        "decode-bit-string": 3,
        "decode-octet-string": 4,
        "decode-null": 5,
        "decode-object-identifier": 6,
        "decode-ia5-string": 22,
    }
    if operation.startswith("decode-implicit-"):
        assert tag_number is not None
        expect_tag(element, "context-specific", False, tag_number)
    else:
        expect_tag(element, "universal", False, universal_tags[operation])

    if operation == "decode-boolean":
        if len(value) != 1:
            raise TypedFailure("invalid-boolean-length", value_offset)
        if value not in (b"\x00", b"\xff"):
            raise TypedFailure("invalid-boolean-value", value_offset)
        return value_result(boolean=value == b"\xff", elements_read=1)

    if operation in ("decode-integer", "integer-to-u64"):
        if not value:
            raise TypedFailure("empty-integer", value_offset)
        if len(value) > 1 and (
            (value[0] == 0 and value[1] & 0x80 == 0)
            or (value[0] == 0xFF and value[1] & 0x80 != 0)
        ):
            raise TypedFailure("non-minimal-integer", value_offset)
        negative = bool(value[0] & 0x80)
        result = value_result(signed_hex=value.hex(), negative=negative)
        if operation == "integer-to-u64":
            if negative:
                raise TypedFailure("negative-integer", value_offset)
            magnitude = value[1:] if value[0] == 0 else value
            if len(magnitude) > 8:
                raise TypedFailure("integer-overflow", value_offset)
            result["u64_decimal"] = str(int.from_bytes(magnitude, "big"))
        return result

    if operation == "decode-bit-string":
        if not value:
            raise TypedFailure("missing-unused-bit-count", value_offset)
        unused = value[0]
        payload = value[1:]
        if unused > 7 or (not payload and unused != 0):
            raise TypedFailure("invalid-unused-bit-count", value_offset)
        if unused and payload[-1] & ((1 << unused) - 1):
            raise TypedFailure("non-zero-bit-padding", value_offset + len(value) - 1)
        return value_result(
            bytes_hex=payload.hex(),
            unused_bits=unused,
            bit_length=len(payload) * 8 - unused,
        )

    if operation in ("decode-octet-string", "decode-implicit-octet-string"):
        return value_result(bytes_hex=value.hex())

    if operation in ("decode-ia5-string", "decode-implicit-ia5-string"):
        for offset, octet in enumerate(value):
            if octet > 0x7F:
                raise TypedFailure("non-ascii-ia5-string", value_offset + offset)
        return value_result(text=value.decode("ascii"))

    if operation == "decode-null":
        if value:
            raise TypedFailure("non-empty-null", value_offset)
        return value_result()

    if operation in (
        "decode-object-identifier",
        "decode-implicit-object-identifier",
    ):
        arcs = parse_oid(value, value_offset, limits["max_oid_arcs"])
        return value_result(
            bytes_hex=value.hex(),
            arcs_decimal=[str(arc) for arc in arcs],
            arc_count=len(arcs),
        )
    raise AssertionError(operation)


def container_result(
    document: dict[str, Any],
    case: dict[str, Any],
    data: bytes,
    root: Element,
    limits: dict[str, Any],
    elements_read: int,
) -> dict[str, Any]:
    operation = case["operation"]
    if operation == "sequence":
        expect_tag(root, "universal", True, 16)
    elif operation == "set":
        expect_tag(root, "universal", True, 17)
    else:
        expect_tag(root, "context-specific", True, case["tag_number"])
    if 1 >= limits["max_depth"]:
        raise TypedFailure("depth-limit-exceeded", 0)
    contents = value_bytes(data, root)
    if operation in ("sequence", "set"):
        return value_result(elements_read=elements_read, remaining_offset=0)
    if elements_read >= limits["max_total_elements"]:
        raise TypedFailure("element-limit-exceeded", root.header_len)
    try:
        child = decode_one(contents, limits["der"])
        if child.encoded_len != len(contents):
            raise FramingFailure("trailing-data", child.encoded_len)
    except FramingFailure as error:
        raise TypedFailure(
            "framing",
            error.offset,
            scope="container-value",
            framing_error_id=error.error_id,
        ) from error
    child_value = contents[child.header_len : child.encoded_len]
    return value_result(
        tag=tag_result(child),
        value_hex=child_value.hex(),
        depth=1,
        elements_read=elements_read + 1,
    )


def cursor_result(
    case: dict[str, Any], data: bytes, root: Element, limits: dict[str, Any]
) -> dict[str, Any]:
    expect_tag(root, "universal", True, 16)
    if 1 >= limits["max_depth"]:
        raise TypedFailure("depth-limit-exceeded", 0)
    contents = value_bytes(data, root)
    offset = 0
    elements_read = 1
    events: list[dict[str, Any]] = []
    for action in case["actions"]:
        if action == "finish":
            if offset == len(contents):
                events.append({"outcome": "finished"})
            else:
                events.append(
                    failure_result(
                        TypedFailure(
                            "framing", offset, "container-value", "trailing-data"
                        )
                    )
                )
            continue
        if action == "read-with-different-limits":
            events.append(
                failure_result(
                    TypedFailure("decoder-limit-mismatch", 0, "container-value")
                )
            )
            continue
        if offset == len(contents):
            events.append({"outcome": "end"})
            continue
        if elements_read >= limits["max_total_elements"]:
            events.append(
                failure_result(
                    TypedFailure("element-limit-exceeded", 0, "container-value")
                )
            )
            continue
        try:
            child = decode_one(contents[offset:], limits["der"])
        except FramingFailure as error:
            events.append(
                failure_result(
                    TypedFailure(
                        "framing",
                        offset + error.offset,
                        "container-value",
                        error.error_id,
                    )
                )
            )
            continue
        offset += child.encoded_len
        elements_read += 1
        events.append(value_result(tag=tag_result(child), depth=1))
    return value_result(
        elements_read=elements_read, remaining_offset=offset, events=events
    )


def run_case(
    document: dict[str, Any], upstream: dict[str, Any], case: dict[str, Any]
) -> dict[str, Any]:
    if "der_tlv_case_id" in case:
        upstream_case = next(
            item for item in upstream["cases"] if item["id"] == case["der_tlv_case_id"]
        )
        return (
            {"outcome": "upstream"}
            if run_decode_case(upstream, upstream_case) == upstream_case["expected"]
            else {}
        )
    try:
        data, root, limits, elements_read = root_element(document, case)
        operation = case["operation"]
        if operation == "decode-exact":
            return value_result(
                tag=tag_result(root),
                header_hex=data[: root.header_len].hex(),
                value_hex=value_bytes(data, root).hex(),
                encoded_hex=data[: root.encoded_len].hex(),
                depth=0,
                elements_read=elements_read,
            )
        if operation in ("sequence", "set", "explicit"):
            return container_result(document, case, data, root, limits, elements_read)
        if operation == "cursor-script":
            return cursor_result(case, data, root, limits)
        return typed_primitive_result(
            operation, data, root, limits, case.get("tag_number")
        )
    except TypedFailure as error:
        return failure_result(error)


class DerAsn1FixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load_json(FIXTURE_ROOT / "schema.json")
        cls.document = load_json(FIXTURE_ROOT / "cases.json")
        cls.tlv_schema = load_json(TLV_ROOT / "schema.json")
        cls.upstream = load_json(TLV_ROOT / "cases.json")

    def test_schema_is_closed_and_document_validates(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        Draft202012Validator(
            self.schema, registry=schema_registry(self.tlv_schema)
        ).validate(self.document)

    def test_profile_ids_operations_and_taxonomy_are_closed(self) -> None:
        self.assertEqual(self.document["schema_version"], 1)
        self.assertEqual(self.document["profile"], "x690-der-asn1-values-v1")
        self.assertEqual(self.document["error_ids"], EXPECTED_ERRORS)
        cases = self.document["cases"]
        self.assertGreaterEqual(len(cases), 90)
        ids = [case["id"] for case in cases]
        self.assertEqual(len(ids), len(set(ids)))
        operations = {case["operation"] for case in cases}
        self.assertEqual(
            operations,
            {
                "decode-exact",
                "decode-boolean",
                "decode-integer",
                "integer-to-u64",
                "decode-bit-string",
                "decode-octet-string",
                "decode-ia5-string",
                "decode-null",
                "decode-object-identifier",
                "decode-implicit-octet-string",
                "decode-implicit-ia5-string",
                "decode-implicit-object-identifier",
                "sequence",
                "set",
                "explicit",
                "cursor-script",
            },
        )

    def test_all_exact_tlv_cases_are_reused_once(self) -> None:
        expected = {
            case["id"]
            for case in self.upstream["cases"]
            if case["operation"] == "decode-exact"
        }
        references = [
            case["der_tlv_case_id"]
            for case in self.document["cases"]
            if "der_tlv_case_id" in case
        ]
        self.assertEqual(set(references), expected)
        self.assertEqual(len(references), len(set(references)))
        self.assertEqual(len(references), len(expected))

    def test_independent_oracle_matches_every_case(self) -> None:
        for case in self.document["cases"]:
            with self.subTest(case=case["id"]):
                self.assertEqual(
                    run_case(self.document, self.upstream, case), case["expected"]
                )

    def test_redacted_semantic_case_does_not_project_payload(self) -> None:
        case = next(
            item
            for item in self.document["cases"]
            if item["id"] == "der-asn1-v1-ia5-nonascii-last"
        )
        result = run_case(self.document, self.upstream, case)
        self.assertNotIn(case["redacted_input_hex"], json.dumps(result))


if __name__ == "__main__":
    unittest.main()
