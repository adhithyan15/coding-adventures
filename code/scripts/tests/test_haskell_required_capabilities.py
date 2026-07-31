"""Validate scaffold capability manifests against the shared schema."""

from __future__ import annotations

import json
import unittest
from pathlib import Path

import jsonschema

REPO_ROOT = Path(__file__).resolve().parents[3]
SCHEMA_PATH = REPO_ROOT / "code/specs/schemas/required_capabilities.schema.json"
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/scaffold-generator"


class RequiredCapabilitiesTest(unittest.TestCase):
    """Keep existing and generated manifests on the schema-v1 contract."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(cls.schema)
        cls.validator = jsonschema.Draft202012Validator(cls.schema)

    def test_all_explicit_haskell_manifests_are_schema_valid(self) -> None:
        roots = (
            REPO_ROOT / "code/packages/haskell",
            REPO_ROOT / "code/programs/haskell",
        )
        paths = sorted(
            path for root in roots for path in root.glob("*/required_capabilities.json")
        )
        self.assertTrue(paths, "expected at least one explicit Haskell manifest")

        for path in paths:
            with self.subTest(path=path.relative_to(REPO_ROOT)):
                document = json.loads(path.read_text(encoding="utf-8"))
                self.assert_schema_valid(document)
                self.assertEqual(
                    f"haskell/{path.parent.name}",
                    document["package"],
                    "manifest package identity must match its on-disk directory",
                )

    def test_language_neutral_scaffold_fixtures_are_schema_valid(self) -> None:
        paths = sorted(FIXTURE_ROOT.glob("haskell_*_required_capabilities.json"))
        self.assertEqual(2, len(paths), "expected Haskell library and program fixtures")

        for path in paths:
            with self.subTest(path=path.relative_to(REPO_ROOT)):
                document = json.loads(path.read_text(encoding="utf-8"))
                self.assert_schema_valid(document)

    def test_ocaml_scaffold_fixtures_are_schema_valid(self) -> None:
        paths = sorted(FIXTURE_ROOT.glob("ocaml-*/required_capabilities.json"))
        self.assertEqual(2, len(paths), "expected OCaml library and program fixtures")

        for path in paths:
            with self.subTest(path=path.relative_to(REPO_ROOT)):
                document = json.loads(path.read_text(encoding="utf-8"))
                self.assert_schema_valid(document)
                self.assertEqual("ocaml/my-pkg", document["package"])

    def test_dart_scaffold_fixtures_are_schema_valid(self) -> None:
        paths = sorted(FIXTURE_ROOT.glob("dart_*_required_capabilities.json"))
        self.assertEqual(2, len(paths), "expected Dart library and program fixtures")

        expected_packages = {"dart/my-pkg", "dart/build-helper"}
        actual_packages: set[str] = set()
        for path in paths:
            with self.subTest(path=path.relative_to(REPO_ROOT)):
                document = json.loads(path.read_text(encoding="utf-8"))
                self.assert_schema_valid(document)
                actual_packages.add(document["package"])

        self.assertEqual(expected_packages, actual_packages)

    def test_dart_generator_declares_its_runtime_authority(self) -> None:
        path = (
            REPO_ROOT
            / "code/programs/dart/scaffold-generator/required_capabilities.json"
        )
        document = json.loads(path.read_text(encoding="utf-8"))
        self.assert_schema_valid(document)
        self.assertEqual(
            {
                "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
                "version": 1,
                "package": "dart/scaffold-generator",
                "capabilities": [
                    {
                        "category": "fs",
                        "action": "read",
                        "target": "scaffold-generator.json",
                        "justification": "Loads the checked-in CLI Builder specification before parsing scaffold arguments.",
                    },
                    {
                        "category": "fs",
                        "action": "read",
                        "target": "../../../../code",
                        "justification": "Confirms the fixed repository root before resolving any generated output path.",
                    },
                    {
                        "category": "fs",
                        "action": "read",
                        "target": "../../../../lessons.md",
                        "justification": "Confirms the fixed repository root before resolving any generated output path.",
                    },
                    {
                        "category": "fs",
                        "action": "read",
                        "target": "../../../packages/dart/*",
                        "justification": "Checks Dart package directory metadata while resolving dependencies and refusing an existing library target.",
                    },
                    {
                        "category": "fs",
                        "action": "read",
                        "target": "../../../packages/dart/*/pubspec.yaml",
                        "justification": "Reads Dart package metadata to validate dependencies and compute their transitive order.",
                    },
                    {
                        "category": "fs",
                        "action": "read",
                        "target": "../*",
                        "justification": "Checks Dart program directory metadata while resolving dependencies and refusing an existing program target.",
                    },
                    {
                        "category": "fs",
                        "action": "read",
                        "target": "../*/pubspec.yaml",
                        "justification": "Reads Dart program metadata when a generated target depends on a sibling program.",
                    },
                    {
                        "category": "fs",
                        "action": "create",
                        "target": "../../../packages/dart/*/**",
                        "justification": "Creates the selected Dart library scaffold and its standard nested directories.",
                    },
                    {
                        "category": "fs",
                        "action": "create",
                        "target": "../*/**",
                        "justification": "Creates the selected Dart program scaffold and its standard nested directories.",
                    },
                    {
                        "category": "fs",
                        "action": "write",
                        "target": "../../../packages/dart/*/**",
                        "justification": "Writes the reviewed library scaffold files beneath the selected target directory.",
                    },
                    {
                        "category": "fs",
                        "action": "write",
                        "target": "../*/**",
                        "justification": "Writes the reviewed program scaffold files beneath the selected target directory.",
                    },
                    {
                        "category": "time",
                        "action": "read",
                        "target": "*",
                        "justification": "Reads the current date for the generated package changelog entry.",
                    },
                    {
                        "category": "stdout",
                        "action": "write",
                        "target": "*",
                        "justification": "Reports dry-run previews, successful output paths, and argument or validation errors.",
                    },
                ],
                "justification": "The scaffold generator reads repository metadata, creates and writes one selected Dart target, dates its changelog, and reports through standard output or standard error. It uses no network, subprocess, environment, FFI, or stdin authority.",
            },
            document,
        )

    def assert_schema_valid(self, document: object) -> None:
        errors = sorted(
            self.validator.iter_errors(document),
            key=lambda error: list(error.absolute_path),
        )
        self.assertEqual(
            [],
            [error.message for error in errors],
            "document must validate against the shared Draft 2020-12 schema",
        )


if __name__ == "__main__":
    unittest.main()
