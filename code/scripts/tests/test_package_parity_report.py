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
            package_path("mystery", "b-tree"),
        ]
        packages, unknown = parity.parse_package_paths(paths)
        report = parity.build_report(packages, unknown)
        markdown = parity.render_markdown(report)

        self.assertEqual(len(report["collisions"]), 1)
        self.assertEqual(report["collisions"][0]["language"], "ruby")
        self.assertEqual(report["collisions"][0]["directories"], ["b-tree", "b_tree"])
        self.assertIn("`ruby/b-tree`: `b-tree`, `b_tree`", markdown)
        self.assertIn("Unclassified package buckets: `mystery`", markdown)

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

    def test_ocaml_is_known_but_excluded_from_established_denominator(self) -> None:
        paths = [
            package_path(language, "universal")
            for language in parity.IMPLEMENTATION_LANGUAGES
        ]
        paths.extend(
            [
                package_path("ocaml", "universal"),
                package_path("ocaml", "ocaml-only"),
            ]
        )

        packages, unknown = parity.parse_package_paths(paths)
        report = parity.build_report(packages, unknown)
        by_package = {row["package"]: row for row in report["package_frequency"]}
        ocaml_summary = next(
            row for row in report["special_buckets"] if row["language"] == "ocaml"
        )

        self.assertEqual(unknown, set())
        self.assertEqual(report["schema_version"], 3)
        self.assertIn("ocaml", report["bucket_classes"]["emerging_implementation"])
        self.assertEqual(report["package_count"]["established_languages"], 15)
        self.assertEqual(report["package_count"]["implementation_union"], 1)
        self.assertEqual(
            report["package_count"]["implementation_package_slots"],
            len(parity.IMPLEMENTATION_LANGUAGES),
        )
        self.assertEqual(report["completion_bands"]["10-15"]["packages"], 1)
        self.assertEqual(report["completion_bands"]["10-15"]["missing_slots"], 0)
        self.assertEqual(report["completion_bands"]["5-9"]["packages"], 0)
        self.assertEqual(report["completion_bands"]["2-4"]["packages"], 0)
        self.assertEqual(report["completion_bands"]["1"]["packages"], 0)
        self.assertEqual(
            sum(band["packages"] for band in report["completion_bands"].values()),
            report["package_count"]["implementation_union"],
        )
        self.assertEqual(by_package["universal"]["language_count"], 15)
        self.assertIn("ocaml", by_package["universal"]["languages"])
        self.assertEqual(by_package["ocaml-only"]["language_count"], 0)
        self.assertEqual(by_package["ocaml-only"]["implementation_languages"], [])
        self.assertEqual(ocaml_summary["class"], "emerging_implementation")
        self.assertEqual(ocaml_summary["present"], 2)
        markdown = parity.render_markdown(report)
        self.assertIn("| ocaml | emerging_implementation | 2 |", markdown)

        csv_rows = {
            row["package"]: row
            for row in csv.DictReader(io.StringIO(parity.render_csv(report)))
        }
        self.assertEqual(csv_rows["universal"]["ocaml"], "1")
        self.assertEqual(csv_rows["ocaml-only"]["ocaml"], "1")

    def test_completion_band_tracks_the_established_denominator(self) -> None:
        self.assertEqual(parity.completion_band_labels(15)[0], "10-15")
        self.assertEqual(parity.completion_band_labels(16)[0], "10-16")
        self.assertEqual(parity.completion_band(16, 16), "10-16")
        self.assertEqual(parity.completion_band(9, 16), "5-9")
        self.assertEqual(parity.completion_band(4, 16), "2-4")
        self.assertEqual(parity.completion_band(1, 16), "1")
        with self.assertRaises(ValueError):
            parity.completion_band_labels(9)
        with self.assertRaises(ValueError):
            parity.completion_band(17, 16)
        with self.assertRaises(ValueError):
            parity.completion_band(0, 16)

    def test_bucket_classes_are_disjoint(self) -> None:
        with self.assertRaisesRegex(ValueError, "exactly one class: ocaml"):
            parity.classified_buckets(
                {
                    "implementation": ("ocaml",),
                    "emerging_implementation": ("ocaml",),
                }
            )

    def test_markdown_uses_reported_denominator_and_band_order(self) -> None:
        paths = [
            package_path(language, "universal")
            for language in parity.IMPLEMENTATION_LANGUAGES
        ]
        packages, unknown = parity.parse_package_paths(paths)
        report = parity.build_report(packages, unknown)

        markdown = parity.render_markdown(report)

        self.assertIn("| Established implementation languages | 15 |", markdown)
        self.assertIn("| Present In | Packages | Missing Slots To All 15 |", markdown)
        self.assertIn("| 10-15 languages | 1 | 0 |", markdown)

    def test_csv_is_a_complete_presence_matrix(self) -> None:
        packages, unknown = parity.parse_package_paths(
            [
                package_path("rust", "heap"),
                package_path("python", "heap"),
                package_path("starlark", "builtins"),
            ]
        )
        report = parity.build_report(packages, unknown)
        reader = csv.DictReader(io.StringIO(parity.render_csv(report)))
        rows = list(reader)

        self.assertEqual(reader.fieldnames, ["package", *parity.ALL_BUCKETS])
        self.assertEqual(len(rows), 2)
        by_package = {row["package"]: row for row in rows}
        self.assertEqual(by_package["heap"]["rust"], "1")
        self.assertEqual(by_package["heap"]["python"], "1")
        self.assertEqual(by_package["heap"]["dart"], "0")
        self.assertEqual(by_package["builtins"]["starlark"], "1")
        self.assertEqual(by_package["builtins"]["ocaml"], "0")


if __name__ == "__main__":
    unittest.main()
