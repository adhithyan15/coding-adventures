"""Validate every explicit Haskell capability manifest against the shared schema."""

from __future__ import annotations

import json
import unittest
from pathlib import Path

import jsonschema

REPO_ROOT = Path(__file__).resolve().parents[3]
SCHEMA_PATH = REPO_ROOT / "code/specs/schemas/required_capabilities.schema.json"
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/scaffold-generator"


class HaskellRequiredCapabilitiesTest(unittest.TestCase):
    """Keep existing and generated Haskell manifests on the schema-v1 contract."""

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
            path
            for root in roots
            for path in root.glob("*/required_capabilities.json")
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
