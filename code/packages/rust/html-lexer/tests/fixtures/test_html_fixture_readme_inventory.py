#!/usr/bin/env python3

"""Regression tests for the HTML fixture README inventory check."""

from __future__ import annotations

import unittest

import check_html_fixture_readme_inventory as readme_check


class HtmlFixtureReadmeInventoryTest(unittest.TestCase):
    def test_readme_inventory_is_current(self) -> None:
        errors, _stats = readme_check.check_readme_inventory()

        self.assertEqual(errors, [])

    def test_inventory_covers_fixture_data_and_user_facing_scripts(self) -> None:
        artifacts = readme_check.readme_artifacts()
        artifact_names = {artifact_name for _readme_path, artifact_name in artifacts}

        self.assertIn("whatwg-attribute-boundaries.json", artifact_names)
        self.assertIn("whatwg-text-mode-boundaries.json", artifact_names)
        self.assertIn("html5lib-tree-construction-smoke.dat", artifact_names)
        self.assertIn("check_html_fixture_readme_inventory.py", artifact_names)
        self.assertIn("audit_html5lib_coverage.py", artifact_names)
        self.assertNotIn("generated_fixture_io.py", artifact_names)
        self.assertNotIn("test_html_fixture_readme_inventory.py", artifact_names)

    def test_inventory_targets_are_sorted_and_unique(self) -> None:
        artifacts = readme_check.readme_artifacts()

        self.assertEqual(artifacts, sorted(artifacts, key=lambda item: (str(item[0]), item[1])))
        self.assertEqual(len(artifacts), len(set(artifacts)))


if __name__ == "__main__":
    unittest.main()
