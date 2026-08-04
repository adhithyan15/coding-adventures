from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import learning_coverage_report as learning  # noqa: E402
import package_parity_report as parity  # noqa: E402


def empty_packages() -> dict[str, dict[str, set[str]]]:
    return {bucket: {} for bucket in parity.ALL_BUCKETS}


def add_package(
    packages: dict[str, dict[str, set[str]]],
    name: str,
    languages: tuple[str, ...],
) -> None:
    identity = parity.package_identity(name)
    for language in languages:
        packages[language][identity] = {name}


class LearningDocumentTests(unittest.TestCase):
    def test_discovers_annotations_and_ignores_generated_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            learning_dir = root / "code" / "learning"
            learning_dir.mkdir(parents=True)
            (learning_dir / "trees.md").write_text(
                "<!-- learning-concepts: b-tree, trie -->\n# Trees\n",
                encoding="utf-8",
            )
            (learning_dir / "COVERAGE.md").write_text(
                "generated package names", encoding="utf-8"
            )

            documents = learning.discover_learning_documents(root)

        self.assertEqual(len(documents), 1)
        self.assertEqual(
            documents[0]["annotations"],
            {parity.package_identity("b-tree"), parity.package_identity("trie")},
        )


class LearningCoverageTests(unittest.TestCase):
    def test_classifies_all_four_coverage_states(self) -> None:
        packages = empty_packages()
        add_package(packages, "dedicated", parity.IMPLEMENTATION_LANGUAGES[:10])
        add_package(packages, "related", parity.IMPLEMENTATION_LANGUAGES[:5])
        add_package(packages, "index-only", parity.IMPLEMENTATION_LANGUAGES[:2])
        add_package(packages, "missing", parity.IMPLEMENTATION_LANGUAGES[:1])
        documents = [
            {
                "path": "code/learning/topic.md",
                "is_index": False,
                "stem_identity": parity.package_identity("topic"),
                "annotations": {parity.package_identity("dedicated")},
                "search_text": "a related concept.",
            },
            {
                "path": "code/learning/README.md",
                "is_index": True,
                "stem_identity": parity.package_identity("README"),
                "annotations": set(),
                "search_text": "index only.",
            },
        ]

        report = learning.build_learning_report(packages, set(), documents)
        rows = {row["package"]: row for row in report["concepts"]}

        self.assertEqual(rows["dedicated"]["status"], "dedicated")
        self.assertEqual(rows["related"]["status"], "related")
        self.assertEqual(rows["index-only"]["status"], "index-only")
        self.assertEqual(rows["missing"]["status"], "missing")
        self.assertEqual(rows["dedicated"]["priority"], "P0")
        self.assertEqual(rows["related"]["priority"], "P1")
        self.assertEqual(rows["index-only"]["priority"], "P2")
        self.assertEqual(rows["missing"]["priority"], "P3")

    def test_assigns_domain_specific_rules_before_generic_parser_rule(self) -> None:
        self.assertEqual(learning.domain_for("sql-parser"), "sql-storage")
        self.assertEqual(
            learning.domain_for("huffman-compression"), "compression-encoding"
        )
        self.assertEqual(learning.domain_for("recursive-descent-parser"), "language-tooling")
        self.assertEqual(learning.domain_for("csharp-parser"), "language-tooling")
        self.assertEqual(learning.domain_for("uuid"), "other")

    def test_mentions_require_concept_boundaries(self) -> None:
        documents = [{
            "path": "code/learning/example.md",
            "is_index": False,
            "stem_identity": "",
            "annotations": set(),
            "search_text": "A language guide mentions algorithms.",
        }]

        self.assertEqual(learning._mentioning_paths(documents, {"go"}), [])
        self.assertEqual(
            learning._mentioning_paths(documents, {"algorithm"}),
            [],
        )
        self.assertEqual(
            learning._mentioning_paths(documents, {"algorithms"}),
            ["code/learning/example.md"],
        )

    def test_markdown_contains_every_actionable_concept(self) -> None:
        report = {
            "summary": {
                "concepts": 2,
                "documents": 0,
                "dedicated": 0,
                "related": 0,
                "index-only": 1,
                "missing": 1,
            },
            "priority_summary": {
                priority: {
                    "dedicated": 0,
                    "related": 0,
                    "index-only": int(priority == "P1"),
                    "missing": int(priority == "P0"),
                }
                for priority in learning.PRIORITY_ORDER
            },
            "concepts": [
                {
                    "package": "alpha",
                    "language_count": 12,
                    "priority": "P0",
                    "domain": "other",
                    "status": "missing",
                },
                {
                    "package": "beta",
                    "language_count": 7,
                    "priority": "P1",
                    "domain": "other",
                    "status": "index-only",
                },
            ],
        }

        markdown = learning.render_markdown(report)

        self.assertIn("| `alpha` | 12 | missing |", markdown)
        self.assertIn("| `beta` | 7 | index-only |", markdown)


if __name__ == "__main__":
    unittest.main()
