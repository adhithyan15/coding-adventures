#!/usr/bin/env python3

"""Regression tests for the HTML fixture case identity checker."""

from __future__ import annotations

import unittest

import check_html_fixture_case_ids as case_id_check


class HtmlFixtureCaseIdsTest(unittest.TestCase):
    def test_checked_in_fixtures_have_valid_case_identities(self) -> None:
        errors, stats = case_id_check.check_fixture_case_ids()

        self.assertEqual(errors, [])
        self.assertGreater(stats.fixture_count, 0)
        self.assertGreater(stats.case_count, 0)

    def test_numeric_reference_fixture_uses_integer_values_as_identities(self) -> None:
        numeric_fixture = (
            case_id_check.FIXTURE_DIR / case_id_check.NUMERIC_REFERENCE_FIXTURE
        )
        numeric_cases = case_id_check.read_json_object(numeric_fixture, [])["cases"]

        self.assertTrue(all("id" not in case for case in numeric_cases))
        self.assertTrue(all(isinstance(case["value"], int) for case in numeric_cases))

    def test_parser_audit_counts_match_case_axes(self) -> None:
        audit_fixture = (
            case_id_check.PARSER_FIXTURE_DIR / "whatwg-table-audit.json"
        )
        audit_data = case_id_check.read_json_object(audit_fixture, [])
        audit_cases = audit_data["cases"]

        self.assertEqual(audit_data["case_count"], len(audit_cases))
        self.assertEqual(
            case_id_check.check_parser_audit_counts(
                "html-parser/tests/fixtures/whatwg-table-audit.json",
                audit_data,
                audit_cases,
            ),
            [],
        )


if __name__ == "__main__":
    unittest.main()
