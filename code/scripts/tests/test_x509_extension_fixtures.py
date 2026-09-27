"""Validate and independently execute the generic X.509 Extension v1 corpus."""

from __future__ import annotations

import json
import unittest
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator
from referencing import Registry, Resource
from test_der_asn1_fixtures import (
    TypedFailure,
    expect_tag,
    limits_for,
    parse_oid,
    root_element,
    value_bytes,
)
from test_der_tlv_fixtures import FramingFailure, decode_one, load_json

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/x509-extension-v1"
ASN1_ROOT = REPO_ROOT / "code/specs/fixtures/der-asn1-v1"
TLV_ROOT = REPO_ROOT / "code/specs/fixtures/der-tlv-v1"

EXPECTED_ERRORS = [
    "structure",
    "missing-extension-id",
    "invalid-extension-id",
    "invalid-critical",
    "encoded-default-critical",
    "missing-extension-value",
    "invalid-extension-value",
    "trailing-element",
]


@dataclass(frozen=True)
class ExtensionFailure(Exception):
    error_id: str
    offset: int
    elements_read: int
    asn1_error_id: str | None = None
    framing_error_id: str | None = None


def schema_registry(
    asn1_schema: dict[str, Any], tlv_schema: dict[str, Any]
) -> Registry:
    asn1 = Resource.from_contents(asn1_schema)
    tlv = Resource.from_contents(tlv_schema)
    return Registry().with_resources(
        [
            ("https://coding-adventures.dev/schemas/der-asn1-v1.json", asn1),
            ("https://coding-adventures.dev/der-asn1-v1/schema.json", asn1),
            ("https://coding-adventures.dev/schemas/der-tlv-v1.json", tlv),
            ("https://coding-adventures.dev/der-tlv-v1/schema.json", tlv),
        ]
    )


def failure_result(error: ExtensionFailure) -> dict[str, Any]:
    result: dict[str, Any] = {
        "outcome": "error",
        "error_id": error.error_id,
        "offset": error.offset,
        "offset_scope": "extension-element",
        "elements_read": error.elements_read,
    }
    if error.asn1_error_id is not None:
        result["asn1_error_id"] = error.asn1_error_id
    if error.framing_error_id is not None:
        result["framing_error_id"] = error.framing_error_id
    return result


def map_typed(
    category: str, error: TypedFailure, elements_read: int, base_offset: int = 0
) -> ExtensionFailure:
    return ExtensionFailure(
        category,
        base_offset + error.offset,
        elements_read,
        error.error_id,
        error.framing_error_id,
    )


def read_child(
    contents: bytes,
    position: int,
    root_header_len: int,
    limits: dict[str, Any],
    elements_read: int,
    sibling_reads: int,
) -> tuple[Any, int, int, int]:
    if elements_read >= limits["max_total_elements"]:
        raise ExtensionFailure(
            "structure",
            root_header_len + position,
            elements_read,
            "element-limit-exceeded",
        )
    if sibling_reads >= limits["der"]["max_elements"]:
        raise ExtensionFailure(
            "structure",
            root_header_len + position,
            elements_read,
            "framing",
            "element-limit-exceeded",
        )
    try:
        child = decode_one(contents[position:], limits["der"])
    except FramingFailure as error:
        raise ExtensionFailure(
            "structure",
            root_header_len + position + error.offset,
            elements_read,
            "framing",
            error.error_id,
        ) from error
    return child, position + child.encoded_len, elements_read + 1, sibling_reads + 1


def decode_boolean(
    contents: bytes, child: Any, child_start: int, root_header_len: int
) -> bool:
    try:
        expect_tag(child, "universal", False, 1)
        value = value_bytes(contents[child_start:], child)
        if len(value) != 1:
            raise TypedFailure("invalid-boolean-length", child.header_len)
        if value not in (b"\x00", b"\xff"):
            raise TypedFailure("invalid-boolean-value", child.header_len)
        return value == b"\xff"
    except TypedFailure as error:
        raise map_typed(
            "invalid-critical",
            error,
            0,
            root_header_len + child_start,
        ) from error


def run_attempt(
    data: bytes, root: Any, limits: dict[str, Any], elements_read: int
) -> dict[str, Any]:
    try:
        try:
            expect_tag(root, "universal", True, 16)
            if 1 >= limits["max_depth"]:
                raise TypedFailure("depth-limit-exceeded", 0)
        except TypedFailure as error:
            raise map_typed("structure", error, elements_read) from error

        contents = value_bytes(data, root)
        position = 0
        sibling_reads = 0
        if not contents:
            raise ExtensionFailure(
                "missing-extension-id", root.header_len, elements_read
            )
        extension_id_start = position
        extension_id, position, elements_read, sibling_reads = read_child(
            contents, position, root.header_len, limits, elements_read, sibling_reads
        )
        try:
            expect_tag(extension_id, "universal", False, 6)
        except TypedFailure as error:
            raise map_typed(
                "invalid-extension-id",
                error,
                elements_read,
                root.header_len + extension_id_start,
            ) from error
        try:
            arcs = parse_oid(
                value_bytes(contents[extension_id_start:], extension_id),
                root.header_len + extension_id_start + extension_id.header_len,
                limits["max_oid_arcs"],
            )
        except TypedFailure as error:
            raise map_typed(
                "invalid-extension-id", error, elements_read
            ) from error

        if position == len(contents):
            raise ExtensionFailure(
                "missing-extension-value",
                root.header_len + position,
                elements_read,
            )
        second_start = position
        second, position, elements_read, sibling_reads = read_child(
            contents, position, root.header_len, limits, elements_read, sibling_reads
        )
        critical = False
        if second.number == 1:
            try:
                critical = decode_boolean(
                    contents, second, second_start, root.header_len
                )
            except ExtensionFailure as error:
                raise ExtensionFailure(
                    error.error_id,
                    error.offset,
                    elements_read,
                    error.asn1_error_id,
                    error.framing_error_id,
                ) from error
            if not critical:
                raise ExtensionFailure(
                    "encoded-default-critical",
                    root.header_len + second_start,
                    elements_read,
                )
            if position == len(contents):
                raise ExtensionFailure(
                    "missing-extension-value",
                    root.header_len + position,
                    elements_read,
                )
            extension_value_start = position
            extension_value, position, elements_read, sibling_reads = read_child(
                contents, position, root.header_len, limits, elements_read, sibling_reads
            )
        else:
            extension_value_start = second_start
            extension_value = second

        try:
            expect_tag(extension_value, "universal", False, 4)
        except TypedFailure as error:
            raise map_typed(
                "invalid-extension-value",
                error,
                elements_read,
                root.header_len + extension_value_start,
            ) from error
        extension_value_bytes = value_bytes(
            contents[extension_value_start:], extension_value
        )

        if position != len(contents):
            trailing_start = position
            _, position, elements_read, sibling_reads = read_child(
                contents, position, root.header_len, limits, elements_read, sibling_reads
            )
            raise ExtensionFailure(
                "trailing-element",
                root.header_len + trailing_start,
                elements_read,
            )

        return {
            "outcome": "value",
            "extension_id_arcs_decimal": [str(arc) for arc in arcs],
            "critical": critical,
            "extension_value_hex": extension_value_bytes.hex(),
            "elements_read": elements_read,
        }
    except ExtensionFailure as error:
        return failure_result(error)


def run_case(
    asn1_document: dict[str, Any], case: dict[str, Any]
) -> dict[str, Any]:
    data, root, limits, elements_read = root_element(asn1_document, case)
    if case["operation"] == "decode-extension":
        return run_attempt(data, root, limits, elements_read)
    events: list[dict[str, Any]] = []
    for action in case["actions"]:
        assert action == "decode"
        event = run_attempt(data, root, limits, elements_read)
        events.append(event)
        elements_read = event["elements_read"]
    return {"outcome": "script", "events": events}


class X509ExtensionFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load_json(FIXTURE_ROOT / "schema.json")
        cls.document = load_json(FIXTURE_ROOT / "cases.json")
        cls.asn1_schema = load_json(ASN1_ROOT / "schema.json")
        cls.asn1_document = load_json(ASN1_ROOT / "cases.json")
        cls.tlv_schema = load_json(TLV_ROOT / "schema.json")

    def test_schema_is_closed_and_document_validates(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        Draft202012Validator(
            self.schema,
            registry=schema_registry(self.asn1_schema, self.tlv_schema),
        ).validate(self.document)

    def test_profile_ids_operation_and_taxonomy_are_closed(self) -> None:
        self.assertEqual(self.document["schema_version"], 1)
        self.assertEqual(self.document["profile"], "rfc5280-x509-extension-v1")
        self.assertEqual(self.document["error_ids"], EXPECTED_ERRORS)
        self.assertGreaterEqual(len(self.document["cases"]), 30)
        ids = [case["id"] for case in self.document["cases"]]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(
            {case["operation"] for case in self.document["cases"]},
            {"decode-extension", "extension-script"},
        )

    def test_nested_error_projection_is_exact(self) -> None:
        for case in self.document["cases"]:
            expected = case["expected"]
            results = (
                expected["events"]
                if expected["outcome"] == "script"
                else [expected]
            )
            for result in results:
                if result["outcome"] != "error":
                    continue
                category = result["error_id"]
                if category in {
                    "structure",
                    "invalid-extension-id",
                    "invalid-critical",
                    "invalid-extension-value",
                }:
                    self.assertIn("asn1_error_id", result, case["id"])
                else:
                    self.assertNotIn("asn1_error_id", result, case["id"])
                if result.get("asn1_error_id") == "framing":
                    self.assertIn("framing_error_id", result, case["id"])
                else:
                    self.assertNotIn("framing_error_id", result, case["id"])

    def test_independent_oracle_matches_every_case(self) -> None:
        for case in self.document["cases"]:
            with self.subTest(case=case["id"]):
                self.assertEqual(
                    run_case(self.asn1_document, case), case["expected"]
                )

    def test_redacted_payload_never_enters_projection(self) -> None:
        for case in self.document["cases"]:
            hostile = case.get("redacted_input_hex")
            if hostile is None:
                continue
            result = run_case(self.asn1_document, case)
            self.assertNotIn(hostile, json.dumps(result))


if __name__ == "__main__":
    unittest.main()
