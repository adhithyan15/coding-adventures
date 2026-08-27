"""Test the dependency-free Scytale fixture-consumer generator."""

# ruff: noqa: E402

from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import generate_scytale_fixture_consumers as generator
from package_parity_report import IMPLEMENTATION_LANGUAGES


class ScytaleFixtureConsumerGeneratorTests(unittest.TestCase):
    def test_target_roster_is_exactly_the_established_denominator(self) -> None:
        self.assertEqual(tuple(generator.TARGETS), IMPLEMENTATION_LANGUAGES)
        self.assertEqual(len(set(generator.TARGETS.values())), 15)

    def test_loads_every_normative_scytale_case(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)

        self.assertEqual(len(cases), 18)
        self.assertEqual(
            {case["operation"] for case in cases},
            {"scytale-encrypt", "scytale-decrypt", "scytale-brute-force"},
        )
        self.assertEqual(
            digest,
            hashlib.sha256(generator.FIXTURE_PATH.read_bytes()).hexdigest(),
        )

    def test_every_render_contains_digest_and_every_case_id(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        outputs = generator.render_all(cases, digest)

        self.assertEqual(set(outputs), set(generator.TARGETS.values()))
        for relative_path, source in outputs.items():
            self.assertTrue(source.endswith("\n"), relative_path)
            self.assertIn(digest, source, relative_path)
            self.assertNotIn(str(generator.REPO_ROOT), source, relative_path)
            for case in cases:
                self.assertIn(case["id"], source, relative_path)

    def test_strict_loader_rejects_duplicate_names_and_nonfinite_numbers(self) -> None:
        valid = generator.FIXTURE_PATH.read_bytes()
        duplicate = valid.replace(
            b'"schema_version": 1,',
            b'"schema_version": 1, "schema_version": 1,',
            1,
        )
        nonfinite = valid.replace(b'"key": 3', b'"key": NaN', 1)

        with self.assertRaisesRegex(ValueError, "fixture-invalid-json"):
            generator.load_cases_bytes(duplicate)
        with self.assertRaisesRegex(ValueError, "fixture-invalid-json"):
            generator.load_cases_bytes(nonfinite)

    def test_loader_rejects_surrogates_depth_and_size_before_rendering(self) -> None:
        document = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        surrogate = copy.deepcopy(document)
        surrogate["cases"][0]["input"]["text"] = "\ud800"
        too_deep = b'{"a":' * 9 + b"null" + b"}" * 9
        too_large = b" " * (generator.MAX_FIXTURE_BYTES + 1)

        with self.assertRaisesRegex(ValueError, "fixture-invalid-scalar"):
            generator.load_cases_bytes(
                json.dumps(surrogate, ensure_ascii=True).encode("utf-8")
            )
        with self.assertRaisesRegex(ValueError, "fixture-depth-limit"):
            generator.load_cases_bytes(too_deep)
        with self.assertRaisesRegex(ValueError, "fixture-size-limit"):
            generator.load_cases_bytes(too_large)

    def test_check_reports_missing_and_changed_outputs_without_writing(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        outputs = generator.render_all(cases, digest)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first_path = next(iter(outputs))
            target = root / first_path
            target.parent.mkdir(parents=True)
            target.write_text("stale\n", encoding="utf-8")

            failures = generator.check_outputs(outputs, root)

        self.assertIn(first_path.as_posix(), failures)
        self.assertEqual(len(failures), 15)


if __name__ == "__main__":
    unittest.main()
