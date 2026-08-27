"""Test the dependency-free Vigenere fixture-consumer generator."""

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

import generate_vigenere_fixture_consumers as generator  # noqa: E402
from package_parity_report import IMPLEMENTATION_LANGUAGES  # noqa: E402


class VigenereFixtureConsumerGeneratorTests(unittest.TestCase):
    def test_target_roster_is_exactly_the_established_denominator(self) -> None:
        self.assertEqual(tuple(generator.TARGETS), IMPLEMENTATION_LANGUAGES)
        self.assertEqual(len(set(generator.TARGETS.values())), 15)

    def test_loads_every_normative_vigenere_case(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)

        self.assertEqual(len(cases), 26)
        self.assertEqual(
            {case["operation"] for case in cases},
            {
                "vigenere-encrypt",
                "vigenere-decrypt",
                "vigenere-find-key-length",
                "vigenere-find-key",
                "vigenere-break",
            },
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
        nonfinite = valid.replace(b'"max_length": 20', b'"max_length": NaN', 1)

        with self.assertRaisesRegex(ValueError, "fixture-invalid-json"):
            generator.load_cases_bytes(duplicate)
        with self.assertRaisesRegex(ValueError, "fixture-invalid-json"):
            generator.load_cases_bytes(nonfinite)

    def test_loader_rejects_surrogates_depth_and_size_before_rendering(self) -> None:
        document = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        surrogate = copy.deepcopy(document)
        vigenere = next(
            case
            for case in surrogate["cases"]
            if case["operation"] == "vigenere-encrypt"
        )
        vigenere["input"]["text"] = "\ud800"
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

    def test_rejects_source_unsafe_identifiers(self) -> None:
        for case_id in (
            "classical-ciphers-v1-vigenere-bad\nSystem.exit(1)",
            "1",
            "classical-ciphers-v1-vigenere-class",
        ):
            with self.subTest(case_id=case_id):
                document = json.loads(
                    generator.FIXTURE_PATH.read_text(encoding="utf-8")
                )
                vigenere = next(
                    case
                    for case in document["cases"]
                    if case["operation"] == "vigenere-encrypt"
                )
                vigenere["id"] = case_id

                with self.assertRaisesRegex(ValueError, "fixture-invalid-case"):
                    generator.load_cases_bytes(
                        json.dumps(document, ensure_ascii=False).encode("utf-8")
                    )

    def test_loader_normalizes_malformed_operation_types(self) -> None:
        document = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        document["cases"][0]["operation"] = ["vigenere-encrypt"]

        with self.assertRaisesRegex(ValueError, "fixture-invalid-case"):
            generator.load_cases_bytes(json.dumps(document).encode("utf-8"))

    def test_loader_enforces_closed_vigenere_schema_bounds(self) -> None:
        baseline = json.loads(generator.FIXTURE_PATH.read_text(encoding="utf-8"))
        mutations = [
            ("analysis-preflight-limit", "input", "repeat_count", 8194),
            ("long-find-key", "input", "ciphertext", "A" * 8194),
            ("standard-encrypt", "input", "key", "A" * 42),
            ("smallest-ic-tie", "input", "max_length", 42),
            ("smallest-ic-tie", "expected", "key_length", 41),
            ("empty-groups-are-a", "expected", "key", "lowercase"),
            ("long-break", "expected", "plaintext", "A" * 8194),
            (
                "analysis-preflight-limit",
                "expected",
                "error_id",
                "vigenere-analysis-limit-mutated",
            ),
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

    def test_all_lanes_encode_fixture_strings_as_scalars(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        hostile = (
            '\x00\x01"\\${System.getenv("SECRET")} '
            '#{System.cmd("env", [])} $ENV{SECRET}'
        )
        cases = copy.deepcopy(cases)
        transform = next(
            case for case in cases if case["operation"] == "vigenere-encrypt"
        )
        transform["input"]["text"] = hostile
        outputs = generator.render_all(cases, digest)

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

    def test_repeat_descriptors_and_error_ids_are_fixture_owned(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        cases = copy.deepcopy(cases)
        limit = next(
            case for case in cases if case["id"].endswith("analysis-preflight-limit")
        )
        baseline = generator.render_all(cases, digest)
        limit["input"] = {
            "repeat_scalar": "\U0010ffff",
            "repeat_count": 8192,
            "max_length": 20,
        }

        outputs = generator.render_all(cases, digest)

        for relative_path, source in outputs.items():
            self.assertNotEqual(baseline[relative_path], source, relative_path)
            self.assertIn("8192", source, relative_path)
            self.assertTrue(
                "\U0010ffff" in source
                or "1114111" in source
                or r"\u{10ffff}" in source,
                relative_path,
            )
            self.assertIn("vigenere-analysis-limit", source, relative_path)

    def test_negative_renderers_cannot_accept_nonthrowing_calls(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        outputs = generator.render_all(cases, digest)
        dart = outputs[generator.TARGETS["dart"]]
        typescript = outputs[generator.TARGETS["typescript"]]

        self.assertIn("expect(caught, isNotNull);", dart)
        self.assertNotIn("fail('expected fixture error')", dart)
        self.assertIn("expect(caught).toBeDefined();", typescript)
        self.assertNotIn("fixture-did-not-throw", typescript)

    def test_complete_expected_objects_drive_every_render(self) -> None:
        cases, digest = generator.load_cases(generator.FIXTURE_PATH)
        baseline = generator.render_all(cases, digest)
        mutations = [
            ("vigenere-standard-encrypt", "text", "mutated-text"),
            ("vigenere-smallest-ic-tie", "key_length", 3),
            ("vigenere-empty-groups-are-a", "key", "BBB"),
            ("vigenere-long-break", "key", "MUTANT"),
            ("vigenere-long-break", "plaintext", "mutated-plaintext"),
        ]

        for suffix, field, value in mutations:
            mutated = copy.deepcopy(cases)
            case = next(case for case in mutated if case["id"].endswith(suffix))
            case["expected"][field] = value
            outputs = generator.render_all(mutated, digest)
            for relative_path in outputs:
                self.assertNotEqual(
                    baseline[relative_path],
                    outputs[relative_path],
                    f"{relative_path}: {suffix}.{field}",
                )

    def test_check_reports_missing_and_changed_outputs_without_writing(self) -> None:
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
        self.assertEqual(len(failures), 15)


if __name__ == "__main__":
    unittest.main()
