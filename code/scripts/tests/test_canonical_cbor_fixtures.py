"""Validate the language-neutral canonical-CBOR v1 fixture contract."""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path
from typing import Any, ClassVar

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/canonical-cbor-v1"
HEX_RE = re.compile(r"^(?:[0-9a-f]{2})*$")
GENERATED_RE = re.compile(
    r"^(?:nested-array:[0-9]+|bytes-repeat:[0-9]+:[0-9a-f]{2})$"
)
WIRE_RE = re.compile(
    r"^wire:(?:nested-array:[0-9]+|bytes-repeat:[0-9]+:[0-9a-f]{2})$"
)


def render_projection(document: dict[str, object]) -> str:
    """Render the dependency-free C projection consumed by native tests."""

    lines = [
        "#ifndef CA_CANONICAL_CBOR_V1_VECTORS_H",
        "#define CA_CANONICAL_CBOR_V1_VECTORS_H",
        "",
        "#include <stddef.h>",
        "",
        "typedef struct {",
        "    const char *id;",
        "    const char *operation;",
        "    const char *input;",
        "    const char *expected;",
        "} CanonicalCborVector;",
        "",
        "static const CanonicalCborVector CANONICAL_CBOR_V1_VECTORS[] = {",
    ]
    cases = document["cases"]
    if not isinstance(cases, list):
        raise TypeError("fixture cases must be a list")
    for case in cases:
        if not isinstance(case, dict):
            raise TypeError("fixture case must be an object")
        fields = [case[name] for name in ("id", "operation", "input", "expected")]
        if any(not isinstance(field, str) or '"' in field or chr(92) in field for field in fields):
            raise AssertionError("fixture projection fields must be plain C strings")
        lines.append(
            "    {" + ", ".join(f'"{field}"' for field in fields) + "},"
        )
    lines.extend(
        [
            "};",
            "",
            "#define CANONICAL_CBOR_V1_VECTOR_COUNT " + chr(92),
            "    (sizeof(CANONICAL_CBOR_V1_VECTORS) / sizeof(CANONICAL_CBOR_V1_VECTORS[0]))",
            "",
            "#endif",
            "",
        ]
    )
    return "\n".join(lines)


class CanonicalCborFixtureTests(unittest.TestCase):
    schema: ClassVar[dict[str, Any]]
    document: ClassVar[dict[str, Any]]
    validator: ClassVar[Any]

    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((FIXTURE_ROOT / "schema.json").read_text("utf-8"))
        cls.document = json.loads((FIXTURE_ROOT / "cases.json").read_text("utf-8"))
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def test_schema_ids_profile_and_limits_are_closed(self) -> None:
        self.validator.validate(self.document)
        ids = [case["id"] for case in self.document["cases"]]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(
            self.document["limits"],
            {"max_nesting_depth": 128, "max_encoded_bytes": 1_048_576},
        )
        self.assertEqual(
            {case["operation"] for case in self.document["cases"]},
            {
                "round-trip",
                "decode-error",
                "encode-map",
                "generated-round-trip",
                "encode-error",
            },
        )

    def test_wire_hex_and_generated_case_grammars_are_exact(self) -> None:
        for case in self.document["cases"]:
            operation = case["operation"]
            input_value = case["input"]
            expected = case["expected"]
            if operation == "round-trip":
                self.assertRegex(input_value, HEX_RE, case["id"])
                self.assertRegex(expected, HEX_RE, case["id"])
            elif operation == "encode-map":
                fragments = input_value.split(";")
                self.assertGreaterEqual(len(fragments), 2, case["id"])
                for fragment in fragments:
                    key, separator, value = fragment.partition("=")
                    self.assertEqual(separator, "=", case["id"])
                    self.assertRegex(key, HEX_RE, case["id"])
                    self.assertRegex(value, HEX_RE, case["id"])
                self.assertRegex(expected, HEX_RE, case["id"])
            elif operation == "generated-round-trip":
                self.assertRegex(input_value, GENERATED_RE, case["id"])
                self.assertRegex(expected, WIRE_RE, case["id"])
            elif operation == "decode-error":
                if input_value.startswith("nested-array-wire:"):
                    self.assertRegex(input_value, r"^nested-array-wire:[0-9]+$")
                else:
                    self.assertRegex(input_value, HEX_RE, case["id"])
            elif operation == "encode-error":
                self.assertTrue(
                    input_value == "duplicate-map-key"
                    or GENERATED_RE.fullmatch(input_value),
                    case["id"],
                )

    def test_all_errors_are_stable_payload_blind_identifiers(self) -> None:
        expected_ids = {
            "unexpected-eof",
            "trailing-bytes",
            "reserved",
            "indefinite",
            "non-minimal-integer",
            "invalid-utf8",
            "non-canonical-map-order",
            "unsupported-simple",
            "float-not-supported",
            "too-deep",
            "length-too-large",
            "duplicate-map-key",
            "encode-too-deep",
            "encode-too-large",
        }
        self.assertEqual(set(self.document["error_ids"]), expected_ids)
        exercised = {
            case["expected"]
            for case in self.document["cases"]
            if case["operation"] in {"decode-error", "encode-error"}
        }
        self.assertEqual(exercised, expected_ids)
        for error_id in expected_ids:
            self.assertRegex(error_id, r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

    def test_projection_is_byte_for_byte_generated_from_json(self) -> None:
        actual = (FIXTURE_ROOT / "canonical_cbor_vectors.h").read_text("utf-8")
        self.assertEqual(actual, render_projection(self.document))


if __name__ == "__main__":
    unittest.main()
