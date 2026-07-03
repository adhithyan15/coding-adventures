#!/usr/bin/env python3

"""Regression tests for the HTML fixture format registry checker."""

from __future__ import annotations

import unittest

import check_html_fixture_format_registry as format_check


class HtmlFixtureFormatRegistryTest(unittest.TestCase):
    def test_checked_in_fixture_formats_match_registry(self) -> None:
        errors, stats = format_check.check_fixture_format_registry()

        self.assertEqual(errors, [])
        self.assertGreater(stats.registered_count, 0)
        self.assertEqual(stats.registered_count, stats.format_fixture_count)

    def test_registry_paths_are_unique(self) -> None:
        paths = [entry.relative_path for entry in format_check.FORMAT_REGISTRY]

        self.assertEqual(len(paths), len(set(paths)))

    def test_registry_categories_are_explicit(self) -> None:
        categories = {entry.category for entry in format_check.FORMAT_REGISTRY}

        self.assertEqual(
            categories,
            {
                "lexer-chunk-boundary",
                "lexer-entities",
                "lexer-input-stream",
                "lexer-numeric-reference",
                "lexer-token",
                "parser-audit",
                "parser-browser-content-tree",
                "parser-browser-readiness",
                "parser-browser-render-tree",
            },
        )

    def test_category_contract_reports_missing_category_field(self) -> None:
        errors = format_check.check_category_contract(
            "html-lexer/tests/fixtures/example.json",
            "lexer-input-stream",
            {
                "format": "whatwg-html-input-stream-preprocessing/v1",
                "cases": [{"id": "missing-normalized", "input": "x"}],
            },
        )

        self.assertIn("normalized", "\n".join(errors))


if __name__ == "__main__":
    unittest.main()
