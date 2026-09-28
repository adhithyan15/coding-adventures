from __future__ import annotations

import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/barcode-symbologies-v1"
ESTABLISHED_ITF_LANGUAGES = {
    "csharp",
    "elixir",
    "fsharp",
    "go",
    "haskell",
    "lua",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
}


class BarcodeSymbologyConsumerRegistryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads(
            (FIXTURE_ROOT / "consumers.schema.json").read_text(encoding="utf-8")
        )
        cls.registry = json.loads(
            (FIXTURE_ROOT / "consumers.json").read_text(encoding="utf-8")
        )

    def test_registry_matches_closed_schema(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        Draft202012Validator(self.schema).validate(self.registry)

    def test_itf_registry_covers_each_existing_lane_once(self) -> None:
        itf = [
            consumer
            for consumer in self.registry["consumers"]
            if consumer["symbology"] == "itf"
        ]
        self.assertEqual(len(itf), len(ESTABLISHED_ITF_LANGUAGES))
        self.assertEqual({consumer["language"] for consumer in itf}, ESTABLISHED_ITF_LANGUAGES)
        self.assertEqual(
            len({(consumer["symbology"], consumer["language"]) for consumer in itf}),
            len(itf),
        )

    def test_registered_package_and_test_paths_exist(self) -> None:
        for consumer in self.registry["consumers"]:
            with self.subTest(consumer=consumer):
                self.assertTrue((REPO_ROOT / consumer["package_root"]).is_dir())
                self.assertTrue((REPO_ROOT / consumer["test_path"]).is_file())


if __name__ == "__main__":
    unittest.main()
