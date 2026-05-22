#!/usr/bin/env python3

"""Regression tests for the HTML fixture Python script compile inventory."""

from __future__ import annotations

import unittest

import check_html_fixture_scripts_compile as script_check


class HtmlFixtureScriptsCompileTest(unittest.TestCase):
    def test_inventory_matches_discovered_scripts(self) -> None:
        discovered_scripts = [
            script_check.relative_script(path)
            for path in script_check.fixture_scripts()
        ]

        self.assertEqual(script_check.read_inventory(), discovered_scripts)

    def test_inventory_covers_lexer_and_parser_fixture_scripts(self) -> None:
        inventory = script_check.read_inventory()

        self.assertIn(
            "html-lexer/tests/fixtures/check_html_fixture_scripts_compile.py",
            inventory,
        )
        self.assertIn(
            "html-parser/tests/fixtures/check_whatwg_audit_manifest.py",
            inventory,
        )
        self.assertTrue(
            any(path.startswith("html-lexer/tests/fixtures/") for path in inventory)
        )
        self.assertTrue(
            any(path.startswith("html-parser/tests/fixtures/") for path in inventory)
        )

    def test_inventory_is_sorted_and_unique(self) -> None:
        inventory = script_check.read_inventory()

        self.assertEqual(inventory, sorted(inventory))
        self.assertEqual(len(inventory), len(set(inventory)))


if __name__ == "__main__":
    unittest.main()
