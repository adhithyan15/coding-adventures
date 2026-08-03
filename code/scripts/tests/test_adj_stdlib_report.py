from __future__ import annotations

import importlib
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

stdlib = importlib.import_module("adj_stdlib_report")


class AdjStdlibReportTests(unittest.TestCase):
    def make_root(self, directory: str) -> Path:
        root = Path(directory)
        for _, relative in stdlib.COLLECTIONS:
            (root / relative).mkdir(parents=True)
        (root / "code" / "packages" / "rust" / "adj-lang-cli" / "tests").mkdir(
            parents=True
        )
        return root

    def test_measures_source_envelope_query_test_and_byte_pin(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self.make_root(directory)
            library = root / "code/specs/data/adj-facts-stdlib/mathematics/counting.adj"
            library.parent.mkdir(parents=True, exist_ok=True)
            digest = "a" * 64
            library.write_text(
                "table count {\n"
                "  columns item, value\n"
                "  row (one, 1)\n"
                f'  quote "one" at 0 snapshot "{digest}"\n'
                '  source "one"\n'
                '  locator "https://example.test/source"\n'
                "  trust authoritative\n"
                "}\n",
                encoding="utf-8",
            )
            (library.parent / "count-recall.query.adj").write_text(
                'import "counting.adj"\n? count(one, $V)\n', encoding="utf-8"
            )
            test = root / "code/packages/rust/adj-lang-cli/tests/counting_e2e.rs"
            test.write_text(
                "// adj-facts-stdlib/mathematics/counting.adj\n", encoding="utf-8"
            )

            report = stdlib.build_report(root)
            row = next(item for item in report["libraries"] if item["content_library"])

        self.assertTrue(row["query_companion"])
        self.assertTrue(row["test_reference"])
        self.assertTrue(row["source_envelope"])
        self.assertTrue(row["pinned_quote"])
        self.assertTrue(row["pin_syntax"])
        self.assertFalse(row["cas_resolvable"])
        self.assertFalse(row["byte_verified"])
        self.assertEqual(row["counts"]["tables"], 1)
        self.assertEqual(report["summary"]["pin_syntax_libraries"], 1)
        self.assertEqual(report["summary"]["byte_pinned_libraries"], 0)

    def test_report_forwards_formula_inventory_command(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self.make_root(directory)
            command = ["trusted-parser"]
            with mock.patch.object(
                stdlib.adj_stdlib_provenance,
                "_validate_repository_unlocked",
                side_effect=stdlib.adj_stdlib_provenance.ProvenanceError(
                    "injected replay stop"
                ),
            ) as validate:
                report = stdlib.build_report(root, formula_inventory_command=command)

        self.assertEqual(report["scope"]["provenance_error"], "injected replay stop")
        self.assertEqual(
            validate.call_args.kwargs["formula_inventory_command"], command
        )

    def test_requires_a_complete_source_envelope_for_every_clause(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self.make_root(directory)
            library = root / "code/specs/data/adj-facts-stdlib/science/two.adj"
            library.parent.mkdir(parents=True, exist_ok=True)
            library.write_text(
                "table first {\n"
                "  columns item, value\n"
                "  row (a, 1)\n"
                '  source "a"\n'
                '  locator "https://example.test/a"\n'
                "  trust authoritative\n"
                "}\n"
                "table second {\n"
                "  columns item, value\n"
                "  row (b, 2)\n"
                '  source "b"\n'
                "  trust authoritative\n"
                "}\n",
                encoding="utf-8",
            )

            report = stdlib.build_report(root)
            row = next(item for item in report["libraries"] if item["content_library"])

        self.assertFalse(row["source_envelope"])
        self.assertEqual(
            report["gaps"]["missing_source_envelope"],
            ["code/specs/data/adj-facts-stdlib/science/two.adj"],
        )

    def test_byte_verification_matches_exact_quote_range_and_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self.make_root(directory)
            collection_root = root / stdlib.COLLECTIONS[0][1]
            library = collection_root / "mathematics/counting.adj"
            library.parent.mkdir(parents=True, exist_ok=True)
            digest = "a" * 64
            library.write_text(
                "table count {\n"
                "  columns item, value\n"
                "  row (one, 1)\n"
                f'  quote "one" at 0 snapshot "{digest}"\n'
                '  source "one"\n'
                '  locator "https://example.test/source"\n'
                "  trust authoritative\n"
                "}\n",
                encoding="utf-8",
            )
            repo_path = library.relative_to(root).as_posix()
            forged = stdlib.inspect_library(
                root,
                "facts",
                collection_root,
                library,
                [],
                set(),
                {repo_path: {"b" * 64: {("different", 0, 9, digest)}}},
            )
            matched = stdlib.inspect_library(
                root,
                "facts",
                collection_root,
                library,
                [],
                set(),
                {repo_path: {"b" * 64: {("one", 0, 3, digest)}}},
            )

        self.assertTrue(forged["cas_resolvable"])
        self.assertFalse(forged["byte_verified"])
        self.assertTrue(matched["byte_verified"])

    def test_two_pins_on_one_clause_do_not_cover_an_unpinned_clause(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self.make_root(directory)
            collection_root = root / stdlib.COLLECTIONS[0][1]
            library = collection_root / "mathematics/two.adj"
            library.parent.mkdir(parents=True, exist_ok=True)
            digest = "a" * 64
            library.write_text(
                "table first {\n"
                "  columns item, value\n"
                "  row (one, 1)\n"
                f'  quote "one" at 0 snapshot "{digest}"\n'
                f'  quote "uno" at 10 snapshot "{digest}"\n'
                "}\n"
                "table second {\n"
                "  columns item, value\n"
                "  row (two, 2)\n"
                "}\n",
                encoding="utf-8",
            )
            repo_path = library.relative_to(root).as_posix()
            row = stdlib.inspect_library(
                root,
                "facts",
                collection_root,
                library,
                [],
                set(),
                {
                    repo_path: {
                        "b" * 64: {
                            ("one", 0, 3, digest),
                            ("uno", 10, 13, digest),
                        }
                    }
                },
            )

        self.assertFalse(row["pin_syntax"])
        self.assertFalse(row["byte_verified"])

    def test_pin_after_closing_brace_is_not_attached_to_previous_clause(self) -> None:
        digest = "a" * 64
        text = (
            "table first {\n"
            f'  quote "one" at 0 snapshot "{digest}"\n'
            "}\n"
            f'quote "detached" at 10 snapshot "{digest}"\n'
            "table second {\n"
            "}\n"
        )

        clauses = stdlib._clause_pin_evidence(text)

        self.assertEqual(clauses, [{("one", 0, 3, digest)}, set()])

    def test_query_comments_do_not_count_as_imports(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self.make_root(directory)
            library = root / "code/specs/data/adj-facts-stdlib/math/counting.adj"
            library.parent.mkdir(parents=True, exist_ok=True)
            library.write_text(
                'relate counts(one, 1) source "one" trust authoritative\n',
                encoding="utf-8",
            )
            (library.parent / "other.query.adj").write_text(
                '% import "counting.adj" is only an example\n? other($Value)\n',
                encoding="utf-8",
            )

            report = stdlib.build_report(root)
            row = next(item for item in report["libraries"] if item["content_library"])

        self.assertFalse(row["query_companion"])
        self.assertEqual(
            report["gaps"]["missing_query_companion"],
            [library.relative_to(root).as_posix()],
        )

    def test_excludes_consumers_from_content_gap_denominators(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self.make_root(directory)
            consumer = root / "code/specs/data/mycin-2026/recall/example-case.adj"
            consumer.write_text(
                'import "edges.adj"\n? treats(example, $Treatment)\n',
                encoding="utf-8",
            )

            report = stdlib.build_report(root)

        recall = report["collections"]["medical-recall"]
        self.assertEqual(recall["adj_files"], 1)
        self.assertEqual(recall["content_libraries"], 0)
        self.assertEqual(recall["consumer_programs"], 1)
        self.assertEqual(report["gaps"]["missing_source_envelope"], [])

    def test_markdown_states_limits_and_lists_small_gaps(self) -> None:
        report = {
            "collections": {
                "facts": {
                    "content_libraries": 1,
                    "clauses": 1,
                    "query_companions": 0,
                    "test_references": 1,
                    "source_envelopes": 1,
                    "byte_pinned_libraries": 0,
                }
            },
            "domains": {
                "facts/mathematics": {
                    "libraries": 1,
                    "clauses": 1,
                    "queries": 0,
                    "tests": 1,
                    "source_envelopes": 1,
                    "byte_pins": 0,
                }
            },
            "gaps": {
                "missing_query_companion": ["code/example.adj"],
                "missing_test_reference": [],
                "missing_source_envelope": [],
                "missing_byte_pin": ["code/example.adj"],
            },
        }

        markdown = stdlib.render_markdown(report)

        self.assertIn("structural inventory", markdown.lower())
        self.assertIn("`code/example.adj`", markdown)
        self.assertIn("cannot measure curriculum coverage", markdown)


if __name__ == "__main__":
    unittest.main()
