from __future__ import annotations

import csv
import io
import sys
import unittest
from pathlib import Path


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import package_parity_report as parity  # noqa: E402


def package_path(language: str, package: str) -> str:
    return f"code/packages/{language}/{package}/src/source.txt"


class PackageIdentityTests(unittest.TestCase):
    def test_identity_ignores_case_and_punctuation(self) -> None:
        variants = ("directed-graph", "directed_graph", "Directed.Graph")
        self.assertEqual(
            {parity.package_identity(variant) for variant in variants},
            {"directedgraph"},
        )

    def test_display_normalizes_pascal_case_and_known_swift_names(self) -> None:
        self.assertEqual(
            parity.package_display_name("IrcServerNative"), "irc-server-native"
        )
        self.assertEqual(parity.package_display_name("Barcode1D"), "barcode-1d")
        self.assertEqual(parity.package_display_name("Mosaic.Flux"), "mosaic-flux")


class PackageInventoryTests(unittest.TestCase):
    def test_parser_uses_only_known_package_buckets(self) -> None:
        packages, unknown = parity.parse_package_paths(
            [
                package_path("rust", "heap"),
                package_path("python", "heap"),
                package_path("python", ".pytest_cache"),
                package_path("rust", "target"),
                package_path("mystery", "heap"),
                "code/programs/rust/not-a-package/src/main.rs",
            ]
        )

        self.assertEqual(set(packages["rust"]), {"heap"})
        self.assertEqual(set(packages["python"]), {"heap"})
        self.assertEqual(unknown, {"mystery"})

    def test_parser_reports_within_language_identity_collisions(self) -> None:
        paths = [
            package_path("ruby", "b-tree"),
            package_path("ruby", "b_tree"),
            package_path("rust", "b-tree"),
            package_path("python", "b-tree"),
        ]
        packages, unknown = parity.parse_package_paths(paths)
        report = parity.build_report(packages, unknown)

        self.assertEqual(len(report["collisions"]), 1)
        self.assertEqual(report["collisions"][0]["language"], "ruby")
        self.assertEqual(report["collisions"][0]["directories"], ["b-tree", "b_tree"])

    def test_report_builds_completion_bands_and_gap_lists(self) -> None:
        paths: list[str] = []
        for language in parity.IMPLEMENTATION_LANGUAGES:
            paths.append(package_path(language, "universal"))
            if language != "dart":
                paths.append(package_path(language, "near-complete"))
        paths.extend(
            [
                package_path("rust", "rust-only"),
                package_path("python", "python-only"),
                package_path("wasm", "universal"),
            ]
        )

        packages, unknown = parity.parse_package_paths(paths)
        report = parity.build_report(packages, unknown)
        coverage = {row["language"]: row for row in report["coverage"]}

        self.assertEqual(report["package_count"]["implementation_union"], 4)
        self.assertEqual(report["package_count"]["high_consensus"], 2)
        self.assertEqual(report["completion_bands"]["10-15"]["packages"], 2)
        self.assertEqual(report["completion_bands"]["10-15"]["missing_slots"], 1)
        self.assertEqual(coverage["dart"]["missing_high_consensus"], 1)
        self.assertEqual(
            coverage["dart"]["missing_high_consensus_packages"], ["near-complete"]
        )
        self.assertEqual(report["singleton_by_language"]["rust"], 1)
        self.assertEqual(report["singleton_by_language"]["python"], 1)

    def test_csv_is_a_complete_presence_matrix(self) -> None:
        packages, unknown = parity.parse_package_paths(
            [
                package_path("rust", "heap"),
                package_path("python", "heap"),
                package_path("starlark", "builtins"),
            ]
        )
        report = parity.build_report(packages, unknown)
        rows = list(csv.DictReader(io.StringIO(parity.render_csv(report))))

        self.assertEqual(len(rows), 2)
        by_package = {row["package"]: row for row in rows}
        self.assertEqual(by_package["heap"]["rust"], "1")
        self.assertEqual(by_package["heap"]["python"], "1")
        self.assertEqual(by_package["heap"]["dart"], "0")
        self.assertEqual(by_package["builtins"]["starlark"], "1")


if __name__ == "__main__":
    unittest.main()
