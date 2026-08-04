#!/usr/bin/env python3

"""Regression tests for split WPT/html5lib conformance source discovery."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from subprocess import run

import audit_html5lib_coverage as audit


class HtmlConformanceCoverageAuditTest(unittest.TestCase):
    def test_resolves_revision_from_nested_checkout_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="html-conformance-audit-") as temp_dir:
            root = Path(temp_dir)
            nested = root / "html" / "syntax" / "parsing" / "resources"
            nested.mkdir(parents=True)
            run(["git", "init", "--quiet", str(root)], check=True)
            run(["git", "-C", str(root), "config", "user.name", "Audit Test"], check=True)
            run(
                ["git", "-C", str(root), "config", "user.email", "audit@example.com"],
                check=True,
            )
            (nested / "sample.dat").write_text("#data\n\n#errors\n#document\n")
            run(["git", "-C", str(root), "add", "."], check=True)
            run(["git", "-C", str(root), "commit", "--quiet", "-m", "fixture"], check=True)

            revision = audit.git_revision(nested)

            self.assertRegex(revision, r"^[0-9a-f]{40}$")

    def test_resolves_current_wpt_and_html5lib_layouts(self) -> None:
        with tempfile.TemporaryDirectory(prefix="html-conformance-audit-") as temp_dir:
            root = Path(temp_dir)
            html5lib_root = root / "html5lib-tests"
            (html5lib_root / "tokenizer").mkdir(parents=True)
            wpt_root = root / "wpt"
            resources = wpt_root / "html" / "syntax" / "parsing" / "resources"
            resources.mkdir(parents=True)
            (resources / "sample.dat").write_text("#data\n\n#errors\n#document\n")

            resolved_html5lib = audit.resolve_html5lib_root(str(root))
            resolved_tree, source = audit.resolve_upstream_tree_path(
                str(wpt_root),
                resolved_html5lib,
            )

            self.assertEqual(resolved_html5lib, html5lib_root.resolve())
            self.assertEqual(resolved_tree, resources.resolve())
            self.assertEqual(source, "wpt/html/syntax/parsing/resources")

    def test_keeps_legacy_html5lib_tree_layout_compatible(self) -> None:
        with tempfile.TemporaryDirectory(prefix="html-conformance-audit-") as temp_dir:
            html5lib_root = Path(temp_dir) / "html5lib-tests"
            (html5lib_root / "tokenizer").mkdir(parents=True)
            tree_construction = html5lib_root / "tree-construction"
            tree_construction.mkdir()

            resolved_tree, source = audit.resolve_upstream_tree_path(
                None,
                html5lib_root,
            )

            self.assertEqual(resolved_tree, tree_construction)
            self.assertEqual(source, "html5lib-tests/tree-construction")

    def test_requires_wpt_when_current_html5lib_has_no_tree_tests(self) -> None:
        with tempfile.TemporaryDirectory(prefix="html-conformance-audit-") as temp_dir:
            html5lib_root = Path(temp_dir) / "html5lib-tests"
            (html5lib_root / "tokenizer").mkdir(parents=True)

            with self.assertRaisesRegex(
                SystemExit,
                "Provide WPT via --wpt-root or WPT_ROOT",
            ):
                audit.resolve_upstream_tree_path(None, html5lib_root)


if __name__ == "__main__":
    unittest.main()
