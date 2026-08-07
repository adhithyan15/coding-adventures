"""Validate the language-neutral WebSocket core fixture contract."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/websocket-core-v1"


class WebSocketCoreFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((FIXTURE_ROOT / "schema.json").read_text("utf-8"))
        cls.document = json.loads((FIXTURE_ROOT / "cases.json").read_text("utf-8"))
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def test_cases_match_schema_and_have_unique_ids(self) -> None:
        self.validator.validate(self.document)
        case_ids = [case["id"] for case in self.document["cases"]]
        self.assertEqual(len(case_ids), len(set(case_ids)))

    def test_every_portable_operation_has_positive_and_adversarial_cases(self) -> None:
        cases = self.document["cases"]
        expected_operations = {
            "derive_accept",
            "build_client_request",
            "validate_client_response",
            "accept_server_request",
            "encode_frame",
            "decode_frames",
            "assemble_messages",
        }
        self.assertEqual({case["operation"] for case in cases}, expected_operations)

        for operation in expected_operations - {"derive_accept"}:
            operation_cases = [case for case in cases if case["operation"] == operation]
            self.assertTrue(
                any("error" not in case["expected"] for case in operation_cases),
                f"{operation} needs a positive case",
            )
            self.assertTrue(
                any("error" in case["expected"] for case in operation_cases),
                f"{operation} needs an adversarial case",
            )

    def test_operation_specific_fields_are_required(self) -> None:
        invalid = copy.deepcopy(self.document)
        decode = next(
            case for case in invalid["cases"] if case["operation"] == "decode_frames"
        )
        decode["input"].pop("max_frame_payload")
        self.assertTrue(list(self.validator.iter_errors(invalid)))

    def test_error_expectations_require_payload_free_diagnostics(self) -> None:
        error_cases = [
            case for case in self.document["cases"] if "error" in case["expected"]
        ]
        self.assertGreaterEqual(len(error_cases), 20)
        for case in error_cases:
            diagnostic = case["expected"]["diagnostic"]
            self.assertTrue(diagnostic.startswith("websocket: "), case["id"])
            for secret in ("payload", "nonce", "mask key", "header value"):
                self.assertNotIn(secret, diagnostic, case["id"])

    def test_hex_fields_are_lowercase_and_byte_aligned(self) -> None:
        def visit(value: object, field: str = "") -> None:
            if isinstance(value, dict):
                for key, child in value.items():
                    visit(child, key)
            elif isinstance(value, list):
                for child in value:
                    visit(child, field)
            elif field.endswith("_hex") and value is not None:
                self.assertIsInstance(value, str)
                self.assertEqual(value, value.lower())
                self.assertEqual(len(value) % 2, 0)

        visit(self.document)


if __name__ == "__main__":
    unittest.main()
