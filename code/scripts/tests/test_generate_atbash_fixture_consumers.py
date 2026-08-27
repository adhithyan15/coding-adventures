"""Test the dependency-free Atbash fixture-consumer generator."""

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

import generate_atbash_fixture_consumers as generator
from package_parity_report import IMPLEMENTATION_LANGUAGES


class AtbashFixtureConsumerGeneratorTests(unittest.TestCase):
    def test_target_roster_is_exactly_the_established_denominator(self) -> None:
        self.assertEqual(tuple(generator.TARGETS), IMPLEMENTATION_LANGUAGES)
        self.assertEqual(len(set(generator.TARGETS.values())), 15)

    def test_loads_the_exact_normative_atbash_roster(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)

        self.assertEqual(len(cases), 6)
        self.assertEqual(tuple(case["id"] for case in cases), generator.ATBASH_CASE_IDS)
        self.assertEqual({case["operation"] for case in cases}, {"atbash-transform"})
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
        nonfinite = valid.replace(b'"max_length": 20', b'"max_length": NaN', 1)

        with self.assertRaisesRegex(ValueError, "fixture-invalid-json"):
            generator.load_cases_bytes(duplicate)
        with self.assertRaisesRegex(ValueError, "fixture-invalid-json"):
            generator.load_cases_bytes(nonfinite)

    def test_loader_rejects_surrogates_depth_and_size_before_rendering(self) -> None:
        document = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        surrogate = copy.deepcopy(document)
        atbash = next(
            case
            for case in surrogate["cases"]
            if case["operation"] == "atbash-transform"
        )
        atbash["input"]["text"] = "\ud800"
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

    def test_file_loader_bounds_the_read_before_parsing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "oversized.json"
            with path.open("wb") as stream:
                stream.seek(generator.MAX_FIXTURE_BYTES)
                stream.write(b"x")

            with self.assertRaisesRegex(ValueError, "fixture-size-limit"):
                generator.load_cases(path)

    def test_rejects_wrong_rosters_and_source_unsafe_identifiers(self) -> None:
        baseline = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        mutations = [
            "classical-ciphers-v1-atbash-bad\nSystem.exit(1)",
            "1",
            "classical-ciphers-v1-atbash-class",
            "classical-ciphers-v1-atbash-unregistered",
        ]

        for case_id in mutations:
            with self.subTest(case_id=case_id):
                document = copy.deepcopy(baseline)
                atbash = next(
                    case
                    for case in document["cases"]
                    if case["operation"] == "atbash-transform"
                )
                atbash["id"] = case_id
                with self.assertRaisesRegex(ValueError, "fixture-invalid-case"):
                    generator.load_cases_bytes(
                        json.dumps(document, ensure_ascii=False).encode("utf-8")
                    )

    def test_loader_normalizes_malformed_operation_types(self) -> None:
        document = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        document["cases"][0]["operation"] = ["atbash-transform"]

        with self.assertRaisesRegex(ValueError, "fixture-invalid-case"):
            generator.load_cases_bytes(json.dumps(document).encode("utf-8"))

    def test_loader_enforces_closed_atbash_case_shape_and_scalar_bound(self) -> None:
        baseline = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        mutations = [
            ("empty", "input", "text", "A" * 8194),
            ("empty", "expected", "text", "A" * 8194),
            ("empty", "input", "key", "SECRET"),
            ("empty", "expected", "error_id", "atbash-error"),
        ]

        for suffix, section, field, value in mutations:
            with self.subTest(case=suffix, field=f"{section}.{field}"):
                document = copy.deepcopy(baseline)
                case = next(
                    case for case in document["cases"] if case["id"].endswith(suffix)
                )
                case[section][field] = value
                with self.assertRaisesRegex(ValueError, "fixture-invalid-case"):
                    generator.load_cases_bytes(
                        json.dumps(document, ensure_ascii=False).encode("utf-8")
                    )

    def test_all_lanes_encode_fixture_strings_without_interpolation(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        hostile = (
            '\x00\x01"\\${System.getenv("SECRET")} '
            '#{System.cmd("env", [])} $ENV{SECRET}'
        )
        mutated = copy.deepcopy(cases)
        mutated[0]["input"]["text"] = hostile
        mutated[0]["expected"]["text"] = hostile
        outputs = generator.render_all(mutated, digest)

        for language in generator.TARGETS:
            source = outputs[generator.TARGETS[language]]
            self.assertNotIn(hostile, source, language)
            if language == "dart":
                self.assertIn(r"\u0000", source, language)
                self.assertIn(r"\$", source, language)
            elif language == "rust":
                self.assertIn(r"\u{0}", source, language)
                self.assertIn(r"\u{24}", source, language)
            else:
                self.assertNotIn(r"\u0000", source, language)
                self.assertIn(str(ord("$")), source, language)

    def test_complete_expected_objects_drive_every_render(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        baseline = generator.render_all(cases, digest)
        mutated = copy.deepcopy(cases)
        mutated[0]["expected"]["text"] = "mutated-text"
        outputs = generator.render_all(mutated, digest)

        for relative_path in outputs:
            self.assertNotEqual(baseline[relative_path], outputs[relative_path])

    def test_check_reports_missing_and_oversized_outputs_without_writing(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        outputs = generator.render_all(cases, digest)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first_path = next(iter(outputs))
            target = root / first_path
            target.parent.mkdir(parents=True)
            with target.open("wb") as stream:
                stream.truncate(len(outputs[first_path].encode("utf-8")) + 1)

            failures = generator.check_outputs(outputs, root)

        self.assertIn(first_path.as_posix(), failures)
        self.assertEqual(
            len(failures), len(outputs) + len(generator.REGISTRATION_REQUIREMENTS)
        )

    def test_check_fails_closed_when_explicit_test_registration_drifts(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        outputs = generator.render_all(cases, digest)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            generator.write_outputs(outputs, root)
            for (
                relative_path,
                requirements,
            ) in generator.REGISTRATION_REQUIREMENTS.items():
                target = root / relative_path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("\n".join(requirements), encoding="utf-8")

            self.assertEqual(generator.check_outputs(outputs, root), [])

            for (
                relative_path,
                requirements,
            ) in generator.REGISTRATION_REQUIREMENTS.items():
                target = root / relative_path
                target.write_text("\n".join(requirements[1:]), encoding="utf-8")
                self.assertIn(
                    relative_path.as_posix(), generator.check_outputs(outputs, root)
                )
                target.write_text("\n".join(requirements), encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
