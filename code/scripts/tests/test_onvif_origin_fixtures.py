"""Validate the language-neutral ONVIF origin-policy fixture contract."""

from __future__ import annotations

import copy
import json
import unittest
from ipaddress import ip_address
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/smart-home-onvif-origin-v1"


class OnvifOriginFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((FIXTURE_ROOT / "schema.json").read_text(encoding="utf-8"))
        cls.cases = json.loads((FIXTURE_ROOT / "cases.json").read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(
            cls.schema, format_checker=FormatChecker()
        )

    def test_cases_match_schema_and_use_unique_ids(self) -> None:
        self.validator.validate(self.cases)
        case_ids = [case["id"] for case in self.cases["cases"]]
        self.assertEqual(len(case_ids), len(set(case_ids)))

    def test_operation_specific_fields_are_required(self) -> None:
        invalid = copy.deepcopy(self.cases)
        discovery = next(
            case for case in invalid["cases"] if case["operation"] == "discovery"
        )
        discovery["input"].pop("xaddrs")
        self.assertTrue(list(self.validator.iter_errors(invalid)))

    def test_empty_strings_and_malformed_addresses_are_rejected(self) -> None:
        empty = copy.deepcopy(self.cases)
        discovery = next(
            case for case in empty["cases"] if case["operation"] == "discovery"
        )
        discovery["input"]["probe_message_id"] = ""
        self.assertTrue(list(self.validator.iter_errors(empty)))

        malformed = copy.deepcopy(self.cases)
        discovery = next(
            case for case in malformed["cases"] if case["operation"] == "discovery"
        )
        discovery["input"]["sender_ip"] = "999.999.999.999"
        self.assertTrue(list(self.validator.iter_errors(malformed)))

    def test_expected_pinned_addresses_are_real_socket_addresses(self) -> None:
        for case in self.cases["cases"]:
            pinned = case["expected"].get("pinned_address")
            if pinned is None:
                continue
            if pinned.startswith("["):
                host, separator, port = pinned[1:].partition("]:")
            else:
                host, separator, port = pinned.rpartition(":")
            self.assertTrue(separator, case["id"])
            ip_address(host)
            self.assertGreaterEqual(int(port), 1, case["id"])
            self.assertLessEqual(int(port), 65535, case["id"])

    def test_acceptance_and_code_cannot_contradict_each_other(self) -> None:
        invalid = copy.deepcopy(self.cases)
        invalid["cases"][0]["expected"] = {"accepted": False, "code": "ok"}
        self.assertTrue(list(self.validator.iter_errors(invalid)))


if __name__ == "__main__":
    unittest.main()
