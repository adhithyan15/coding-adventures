#!/usr/bin/env python3

"""Regression tests for the HTML fixture JSON schema checker."""

from __future__ import annotations

import unittest

import check_html_fixture_schemas as schema_check


class HtmlFixtureSchemasTest(unittest.TestCase):
    def test_checked_in_fixtures_match_schema_contracts(self) -> None:
        errors, stats = schema_check.check_fixture_schemas()

        self.assertEqual(errors, [])
        self.assertGreater(stats.fixture_count, 0)
        self.assertGreater(stats.case_count, 0)

    def test_parser_audit_cases_require_source_axis_and_reason(self) -> None:
        errors = schema_check.check_parser_audit_case(
            "html-parser/tests/fixtures/example.json",
            0,
            {
                "id": "example",
                "axis": "table",
                "reason": "foster parenting",
                "source": "tests1.dat:42",
            },
        )

        self.assertEqual(errors, [])

    def test_browser_readiness_cases_require_browser_expected_shape(self) -> None:
        errors = schema_check.check_browser_readiness_case(
            "html-parser/tests/fixtures/html-browser-readiness.json",
            0,
            {
                "id": "example",
                "input": "<title>x</title><p>body",
                "expected": {
                    "title": "x",
                    "base_href": None,
                    "body_text": "body",
                    "headings": [],
                    "links": [],
                    "images": [],
                    "forms": [],
                    "tables": [],
                },
            },
        )

        self.assertEqual(errors, [])

    def test_chunk_split_points_must_stay_inside_input(self) -> None:
        errors = schema_check.check_lexer_case(
            "html-lexer/tests/fixtures/whatwg-chunk-boundaries.json",
            schema_check.CHUNK_BOUNDARY_FIXTURE,
            0,
            {
                "id": "bad-split",
                "input": "abc",
                "split_points": [0, 4],
            },
        )

        self.assertIn("positions outside input length", "\n".join(errors))

    def test_numeric_reference_cases_use_numeric_schema(self) -> None:
        errors = schema_check.check_lexer_case(
            "html-lexer/tests/fixtures/whatwg-numeric-references.json",
            schema_check.NUMERIC_REFERENCE_FIXTURE,
            0,
            {
                "value": 60,
                "characters": "<",
                "codepoints": [60],
                "decimal": "&#60;",
                "decimal_missing_semicolon": "&#60",
                "hex": "&#x3C;",
                "hex_missing_semicolon": "&#x3C",
                "diagnostics": [],
            },
        )

        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
