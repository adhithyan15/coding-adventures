"""Validate the language-neutral barcode-symbologies v1 corpus."""

from __future__ import annotations

import hashlib
import json
import math
import os
import subprocess
import sys
import unittest
from pathlib import Path
from typing import Any, ClassVar

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/barcode-symbologies-v1"
MAX_FIXTURE_BYTES = 131_072
MAX_FIXTURE_DEPTH = 8
MAX_SCHEMA_DEPTH = 24


class FixtureLoadError(ValueError):
    """Stable failure from the bounded fixture loader."""

    def __init__(self, error_id: str) -> None:
        self.error_id = error_id
        super().__init__(error_id)


def _read_bounded(path: Path) -> bytes:
    with path.open("rb") as source:
        if os.fstat(source.fileno()).st_size > MAX_FIXTURE_BYTES:
            raise FixtureLoadError("fixture-size-limit")
        encoded = source.read(MAX_FIXTURE_BYTES + 1)
    if len(encoded) > MAX_FIXTURE_BYTES:
        raise FixtureLoadError("fixture-size-limit")
    return encoded


def _depth_preflight(encoded: bytes, limit: int) -> None:
    try:
        source = encoded.decode("utf-8")
    except UnicodeDecodeError as error:
        raise FixtureLoadError("fixture-invalid-json") from error
    depth = 0
    in_string = False
    escaped = False
    for character in source:
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
        elif character == '"':
            in_string = True
        elif character in "[{":
            depth += 1
            if depth > limit:
                raise FixtureLoadError("fixture-depth-limit")
        elif character in "]}":
            depth -= 1


def _strict_json(encoded: bytes) -> object:
    def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON object name")
            result[key] = value
        return result

    def reject_constant(_token: str) -> object:
        raise ValueError("non-standard JSON number")

    try:
        return json.loads(
            encoded,
            object_pairs_hook=unique_object,
            parse_constant=reject_constant,
        )
    except (
        UnicodeDecodeError,
        json.JSONDecodeError,
        RecursionError,
        ValueError,
    ) as error:
        raise FixtureLoadError("fixture-invalid-json") from error


def _validate_tree(value: object, depth_limit: int) -> None:
    stack: list[tuple[object, int]] = [(value, 0)]
    while stack:
        current, depth = stack.pop()
        if depth > depth_limit:
            raise FixtureLoadError("fixture-depth-limit")
        if isinstance(current, str):
            if any(0xD800 <= ord(character) <= 0xDFFF for character in current):
                raise FixtureLoadError("fixture-invalid-scalar")
        elif isinstance(current, float) and not math.isfinite(current):
            raise FixtureLoadError("fixture-invalid-json")
        elif isinstance(current, list):
            stack.extend((item, depth + 1) for item in current)
        elif isinstance(current, dict):
            stack.extend((key, depth + 1) for key in current)
            stack.extend((item, depth + 1) for item in current.values())


def _validate_local_refs(schema: object) -> None:
    stack = [schema]
    while stack:
        current = stack.pop()
        if isinstance(current, list):
            stack.extend(current)
        elif isinstance(current, dict):
            for key, value in current.items():
                if key in {"$ref", "$dynamicRef"} and (
                    not isinstance(value, str) or not value.startswith("#/")
                ):
                    raise FixtureLoadError("fixture-schema-invalid")
                stack.append(value)


def _load_fixture(schema_encoded: bytes, document_encoded: bytes) -> dict[str, Any]:
    if max(len(schema_encoded), len(document_encoded)) > MAX_FIXTURE_BYTES:
        raise FixtureLoadError("fixture-size-limit")
    _depth_preflight(schema_encoded, MAX_SCHEMA_DEPTH)
    _depth_preflight(document_encoded, MAX_FIXTURE_DEPTH)
    schema = _strict_json(schema_encoded)
    document = _strict_json(document_encoded)
    if not isinstance(schema, dict) or not isinstance(document, dict):
        raise FixtureLoadError("fixture-schema-invalid")
    _validate_tree(schema, MAX_SCHEMA_DEPTH)
    _validate_tree(document, MAX_FIXTURE_DEPTH)
    _validate_local_refs(schema)
    try:
        Draft202012Validator.check_schema(schema)
        Draft202012Validator(schema).validate(document)
    except Exception as error:
        raise FixtureLoadError("fixture-schema-invalid") from error
    ids = [
        case.get("id") for case in document.get("cases", []) if isinstance(case, dict)
    ]
    if len(ids) != len(set(ids)):
        raise FixtureLoadError("fixture-duplicate-id")
    return document


class BarcodeSymbologyFixtureTests(unittest.TestCase):
    document: ClassVar[dict[str, Any]]
    by_id: ClassVar[dict[str, dict[str, Any]]]

    @classmethod
    def setUpClass(cls) -> None:
        cls.schema_encoded = _read_bounded(FIXTURE_ROOT / "schema.json")
        cls.document_encoded = _read_bounded(FIXTURE_ROOT / "cases.json")
        cls.document = _load_fixture(cls.schema_encoded, cls.document_encoded)
        cls.by_id = {case["id"]: case for case in cls.document["cases"]}

    def test_envelope_ids_and_coverage_are_closed(self) -> None:
        self.assertEqual(
            {path.name for path in FIXTURE_ROOT.iterdir() if path.is_file()},
            {
                "CHANGELOG.md",
                "README.md",
                "cases.json",
                "consumers.json",
                "consumers.schema.json",
                "generate_cases.py",
                "schema.json",
            },
        )
        cases = self.document["cases"]
        self.assertEqual(len(self.by_id), len(cases))
        self.assertEqual(
            self.document["limits"],
            {
                "max_input_scalars": 4096,
                "max_cases": 96,
                "max_fixture_bytes": 131_072,
                "max_fixture_depth": 8,
                "max_module_bits": 65_567,
                "max_runs": 40_979,
            },
        )
        self.assertEqual(
            self.document["error_ids"],
            [
                "input-too-long",
                "invalid-length",
                "invalid-character",
                "reserved-character",
                "invalid-guard",
                "guard-conflict",
                "invalid-check-digit",
            ],
        )
        self.assertEqual(
            {case["symbology"] for case in cases},
            {"itf", "code39", "codabar", "code128-b", "upc-a", "ean-13"},
        )
        exercised_errors = {
            case["expected"]["error"] for case in cases if "error" in case["expected"]
        }
        self.assertEqual(exercised_errors, set(self.document["error_ids"]))

    def test_exact_outputs_have_canonical_runs_and_digests(self) -> None:
        for case in self.document["cases"]:
            expected = case["expected"]
            if "modules" not in expected:
                continue
            modules = expected["modules"]
            self.assertEqual(expected["module_count"], len(modules), case["id"])
            self.assertEqual(
                expected["module_sha256"],
                hashlib.sha256(modules.encode("ascii")).hexdigest(),
                case["id"],
            )
            bit = "1"
            rebuilt: list[str] = []
            for length in expected["run_lengths"]:
                rebuilt.append(bit * length)
                bit = "0" if bit == "1" else "1"
            self.assertEqual("".join(rebuilt), modules, case["id"])

    def test_boundary_cases_use_compact_digest_results(self) -> None:
        boundary_results = {
            "barcode-v1-itf-limit": (
                "1d05a1711752d58cd7b1a0fc3b865510186533adc6b73b84fba762884acfa52d",
                36873,
                "8b6082f6457e993cd94fbfa69695e4642a611709dd743e8ec708a83cd4ad8923",
                20487,
                "bcbea6d6d8120c3e3810fe31a0fce95e20dabe5eb3a4993cd6c42a64a194358c",
            ),
            "barcode-v1-code39-limit": (
                "6896d9ea3f73a4434f5832bc65714e7d066f177373f36f34dc8a6f735daa41b1",
                65567,
                "72f9b8c37f97c6b087956fea6dfe23068bf0074ad7307ea4ccb292e198c178b5",
                40979,
                "51223c9cdcdd76b7dffc84569affceda616a971182427c1a4b4bae5a88b9b157",
            ),
            "barcode-v1-codabar-limit": (
                "0baf2aa0e9193c46259b8a7535fd0e4d72f175d21725e68357c699bffe0ca42b",
                40981,
                "bdfa55560b4c7702b71fabc7ee8f3b6daf9352a7f96a1034d04760edebc7696d",
                32783,
                "b099e19f83cf933005c74263d44c6998f2beda6c574adcd49254acdda826d14b",
            ),
            "barcode-v1-code128-b-limit": (
                "6896d9ea3f73a4434f5832bc65714e7d066f177373f36f34dc8a6f735daa41b1",
                45091,
                "1e7c8653046a853b5432243d4ea38659083faafc66695641788a51ba335864ee",
                24595,
                "f4f55266c5126ea8d527113bc930badf37968294530f103d2c0e07f17b452b37",
            ),
        }
        for case_id, pinned in boundary_results.items():
            expected = self.by_id[case_id]["expected"]
            self.assertNotIn("modules", expected)
            self.assertEqual(
                (
                    expected["normalized_sha256"],
                    expected["module_count"],
                    expected["module_sha256"],
                    expected["run_count"],
                    expected["run_lengths_sha256"],
                ),
                pinned,
            )

    def test_representative_published_vectors_are_pinned(self) -> None:
        self.assertEqual(
            self.by_id["barcode-v1-itf-12"]["expected"]["modules"],
            "101011101000101011100011101",
        )
        self.assertEqual(
            self.by_id["barcode-v1-code39-a"]["expected"]["modules"],
            "10001011101110101110101000101110100010111011101",
        )
        self.assertEqual(
            self.by_id["barcode-v1-codabar-zero"]["expected"]["modules"],
            "1011001001010101001101011001001",
        )
        self.assertEqual(
            self.by_id["barcode-v1-code128-b-empty"]["expected"]["symbol_values"],
            [104, 1, 106],
        )
        self.assertEqual(
            self.by_id["barcode-v1-upc-a-computed"]["expected"]["modules"],
            "10100011010111101010111100011010001101000110101010110110011101001100110101110010011101101100101",
        )
        ean = self.by_id["barcode-v1-ean-13-computed"]["expected"]
        self.assertEqual(ean["left_parity"], "LGLLGG")
        self.assertEqual(
            ean["modules"],
            "10100011010100111010111101111010001001011001101010100001010000101000010111010010000101100110101",
        )

    def test_code39_and_code128_policy_decisions_are_explicit(self) -> None:
        code39_empty = self.by_id["barcode-v1-code39-empty"]["expected"]
        self.assertEqual(code39_empty["normalized"], "")
        self.assertEqual(code39_empty["module_count"], 31)
        code128_empty = self.by_id["barcode-v1-code128-b-empty"]["expected"]
        self.assertEqual(code128_empty["checksum"], 1)
        self.assertEqual(code128_empty["module_count"], 35)
        self.assertEqual(
            self.by_id["barcode-v1-code39-reserved-star"]["expected"],
            {"error": "reserved-character"},
        )

        code128_values = {
            value
            for case in self.document["cases"]
            if case["symbology"] == "code128-b" and "symbol_values" in case["expected"]
            for value in case["expected"]["symbol_values"]
        }
        self.assertEqual(code128_values, set(range(103)) | {104, 106})

    def test_all_required_symbol_tables_are_exercised(self) -> None:
        successes = [
            case for case in self.document["cases"] if "normalized" in case["expected"]
        ]

        def normalized_characters(symbology: str) -> set[str]:
            return {
                character
                for case in successes
                if case["symbology"] == symbology
                for character in case["expected"]["normalized"]
            }

        self.assertEqual(normalized_characters("itf"), set("0123456789"))
        self.assertEqual(
            normalized_characters("code39") | {"*"},
            set("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ -.$/+%*"),
        )
        self.assertEqual(normalized_characters("codabar"), set("0123456789-$:/.+ABCD"))

        upc_left: set[str] = set()
        upc_right: set[str] = set()
        ean_tables = {"L": set(), "G": set(), "R": set()}
        for case in successes:
            normalized = case["expected"]["normalized"]
            if case["symbology"] == "upc-a":
                upc_left.update(normalized[:6])
                upc_right.update(normalized[6:])
            elif case["symbology"] == "ean-13":
                parity = case["expected"]["left_parity"]
                for digit, table in zip(normalized[1:7], parity, strict=True):
                    ean_tables[table].add(digit)
                ean_tables["R"].update(normalized[7:])
        all_digits = set("0123456789")
        self.assertEqual(upc_left, all_digits)
        self.assertEqual(upc_right, all_digits)
        self.assertEqual(ean_tables, {table: all_digits for table in "LGR"})

    def test_generator_is_byte_for_byte_clean(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(FIXTURE_ROOT / "generate_cases.py"), "--check"],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)

    def test_bounded_loader_rejects_hostile_json_and_schema(self) -> None:
        with self.assertRaisesRegex(FixtureLoadError, "fixture-size-limit"):
            _load_fixture(self.schema_encoded, b" " * (MAX_FIXTURE_BYTES + 1))
        with self.assertRaisesRegex(FixtureLoadError, "fixture-depth-limit"):
            _load_fixture(self.schema_encoded, (b'{"x":' * 10) + b"0" + (b"}" * 10))
        duplicate = self.document_encoded.replace(
            b'"schema_version": 1,',
            b'"schema_version": 1, "schema_version": 1,',
            1,
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-json"):
            _load_fixture(self.schema_encoded, duplicate)
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-json"):
            _load_fixture(self.schema_encoded, b'{"value": NaN}')
        surrogate = self.document_encoded.replace(
            b'"barcode-symbologies-v1"', b'"\\ud800"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-scalar"):
            _load_fixture(self.schema_encoded, surrogate)
        external_schema = self.schema_encoded.replace(
            b'"#/$defs/case"', b'"https://example.invalid/case.json"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load_fixture(external_schema, self.document_encoded)
        nonfinite_schema = self.schema_encoded.replace(
            b"{", b'{"x-overflow": 1e999999,', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-json"):
            _load_fixture(nonfinite_schema, self.document_encoded)
        surrogate_schema = self.schema_encoded.replace(
            b'"Barcode symbologies v1 conformance corpus"', b'"\\ud800"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-scalar"):
            _load_fixture(surrogate_schema, self.document_encoded)
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load_fixture(self.schema_encoded, b"{}")
        hash_with_lf = self.document_encoded.replace(
            b"8b6082f6457e993cd94fbfa69695e4642a611709dd743e8ec708a83cd4ad8923",
            b"8b6082f6457e993cd94fbfa69695e4642a611709dd743e8ec708a83cd4ad8923\\n",
            1,
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load_fixture(self.schema_encoded, hash_with_lf)
        id_with_cr = self.document_encoded.replace(
            b'"barcode-v1-itf-12"', b'"barcode-v1-itf-12\\r"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load_fixture(self.schema_encoded, id_with_cr)
        impossible_normalizations = {
            "barcode-v1-itf-12": "1" * 4098,
            "barcode-v1-upc-a-computed": "0" * 13,
            "barcode-v1-ean-13-computed": "ABCDEFGHIJKLM",
        }
        for case_id, normalized in impossible_normalizations.items():
            mutated = json.loads(self.document_encoded)
            next(case for case in mutated["cases"] if case["id"] == case_id)[
                "expected"
            ]["normalized"] = normalized
            with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
                _load_fixture(
                    self.schema_encoded,
                    json.dumps(mutated, ensure_ascii=False).encode("utf-8"),
                )


if __name__ == "__main__":
    unittest.main()
