#!/usr/bin/env python3

"""Regression tests for the generated HTML fixture stale-check manifest."""

from __future__ import annotations

from pathlib import Path
import unittest

import check_generated_html_fixtures as manifest


EXTERNAL_SOURCE_GENERATORS = {
    "generate_whatwg_entities_fixture.py",
}


class GeneratedHtmlFixtureManifestTest(unittest.TestCase):
    def test_default_manifest_covers_all_self_contained_lexer_generators(self) -> None:
        checked_scripts = scripts_in(manifest.default_checks())
        generator_scripts = {
            path.name
            for path in manifest.FIXTURE_DIR.glob("generate_whatwg_*_fixture.py")
        }
        self_contained_generators = generator_scripts - EXTERNAL_SOURCE_GENERATORS

        self.assertEqual(
            checked_scripts & self_contained_generators,
            self_contained_generators,
        )
        self.assertNotIn("generate_whatwg_entities_fixture.py", checked_scripts)

    def test_default_manifest_keeps_parser_and_html5lib_checks_visible(self) -> None:
        checked_scripts = scripts_in(manifest.default_checks())
        parser_generators = {
            path.name
            for path in manifest.PARSER_FIXTURE_DIR.glob("generate_whatwg_*_fixture.py")
        }

        self.assertIn("normalize_html5lib_fixtures.py", checked_scripts)
        self.assertIn("check_html5lib_tokenizer_coverage.py", checked_scripts)
        self.assertIn("check_whatwg_lexer_fixture_metadata.py", checked_scripts)
        self.assertIn("check_whatwg_lexer_rust_tests.py", checked_scripts)
        self.assertIn("check_whatwg_audit_manifest.py", checked_scripts)
        self.assertIn("check_html5lib_tree_construction_smoke.py", checked_scripts)
        self.assertIn("check_whatwg_audit_rust_tests.py", checked_scripts)
        self.assertEqual(
            checked_scripts & parser_generators,
            parser_generators,
        )

    def test_default_manifest_check_names_are_unique_and_use_check_mode(self) -> None:
        checks = manifest.default_checks()
        check_names = [check.name for check in checks]

        self.assertEqual(len(check_names), len(set(check_names)))
        for check in checks:
            self.assertIn("--check", check.command)


def scripts_in(checks: list[manifest.FixtureCheck]) -> set[str]:
    return {Path(check.command[0]).name for check in checks}


if __name__ == "__main__":
    unittest.main()
