"""Validate and execute the CR01-CR03 language-neutral fixture corpus."""

from __future__ import annotations

import copy
import json
import math
import unittest
from pathlib import Path
from typing import Any, ClassVar

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/classical-ciphers-v1"


class ConformanceError(ValueError):
    """A payload-blind error produced by the semantic oracle."""

    def __init__(self, error_id: str) -> None:
        self.error_id = error_id
        super().__init__(error_id)


def _has_surrogate(value: str) -> bool:
    return any(0xD800 <= ord(character) <= 0xDFFF for character in value)


def _walk_strings(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [text for item in value for text in _walk_strings(item)]
    if isinstance(value, dict):
        return [
            text
            for key, item in value.items()
            for text in (*_walk_strings(key), *_walk_strings(item))
        ]
    return []


def _validate_document(document: dict[str, Any], encoded_size: int) -> None:
    limits = document["limits"]
    if encoded_size > limits["max_fixture_bytes"]:
        raise ConformanceError("fixture-size-limit")
    ids = [case["id"] for case in document["cases"]]
    if len(ids) != len(set(ids)):
        raise ConformanceError("fixture-duplicate-id")
    if any(_has_surrogate(text) for text in _walk_strings(document)):
        raise ConformanceError("fixture-invalid-scalar")


def _atbash(text: str) -> str:
    transformed: list[str] = []
    for character in text:
        code_point = ord(character)
        if 0x41 <= code_point <= 0x5A:
            transformed.append(chr(0x5A - (code_point - 0x41)))
        elif 0x61 <= code_point <= 0x7A:
            transformed.append(chr(0x7A - (code_point - 0x61)))
        else:
            transformed.append(character)
    return "".join(transformed)


def _scytale_key(text: str, key: int) -> None:
    if key < 2 or key > len(text):
        raise ConformanceError("scytale-invalid-key")


def _scytale_encrypt(text: str, key: int) -> str:
    if not text:
        return ""
    _scytale_key(text, key)
    rows = math.ceil(len(text) / key)
    padded = list(text) + [" "] * (rows * key - len(text))
    return "".join(
        padded[row * key + column] for column in range(key) for row in range(rows)
    )


def _scytale_decrypt(text: str, key: int) -> str:
    if not text:
        return ""
    _scytale_key(text, key)
    rows = math.ceil(len(text) / key)
    columns = [text[start : start + rows] for start in range(0, len(text), rows)]
    plaintext = "".join(
        columns[column][row]
        for row in range(rows)
        for column in range(len(columns))
        if row < len(columns[column])
    )
    return plaintext.rstrip(" ")


def _scytale_brute_force(text: str, limit: int) -> list[dict[str, object]]:
    if len(text) > limit:
        raise ConformanceError("scytale-brute-force-limit")
    return [
        {"key": key, "text": _scytale_decrypt(text, key)}
        for key in range(2, len(text) // 2 + 1)
    ]


def _ascii_letter(code_point: int) -> bool:
    return 0x41 <= code_point <= 0x5A or 0x61 <= code_point <= 0x7A


def _key_shifts(key: str) -> list[int]:
    if not key or any(not _ascii_letter(ord(character)) for character in key):
        raise ConformanceError("vigenere-invalid-key")
    return [(ord(character.upper()) - 0x41) for character in key]


def _vigenere_transform(text: str, key: str, direction: int) -> str:
    shifts = _key_shifts(key)
    key_index = 0
    output: list[str] = []
    for character in text:
        code_point = ord(character)
        if _ascii_letter(code_point):
            base = 0x61 if code_point >= 0x61 else 0x41
            shift = direction * shifts[key_index % len(shifts)]
            output.append(chr(base + (code_point - base + shift) % 26))
            key_index += 1
        else:
            output.append(character)
    return "".join(output)


def _ascii_upper(text: str) -> list[int]:
    return [
        ord(character.upper()) for character in text if _ascii_letter(ord(character))
    ]


def _position_group(letters: list[int], key_length: int, position: int) -> list[int]:
    return letters[position::key_length]


def _index_of_coincidence(letters: list[int]) -> float:
    if len(letters) < 2:
        return 0.0
    counts = [0] * 26
    for letter in letters:
        counts[letter - 0x41] += 1
    numerator = sum(count * (count - 1) for count in counts)
    return numerator / (len(letters) * (len(letters) - 1))


def _find_key_length(
    ciphertext: str,
    max_length: int,
    limits: dict[str, int],
    analysis: dict[str, object],
) -> int:
    if max_length > limits["max_vigenere_key_length"]:
        raise ConformanceError("vigenere-key-length-limit")
    if len(ciphertext) > limits["max_vigenere_analysis_scalars"]:
        raise ConformanceError("vigenere-analysis-limit")
    letters = _ascii_upper(ciphertext)
    candidate_limit = min(max_length, len(letters) // 2)
    if len(letters) < 2 or candidate_limit < 2:
        return int(analysis["insufficient_signal_key_length"])
    scores: list[tuple[int, float]] = []
    for key_length in range(2, candidate_limit + 1):
        groups = [
            _position_group(letters, key_length, position)
            for position in range(key_length)
        ]
        valid_groups = [group for group in groups if len(group) > 1]
        average = sum(_index_of_coincidence(group) for group in valid_groups) / len(
            valid_groups
        )
        scores.append((key_length, average))
    best_score = max(score for _, score in scores)
    if best_score <= 0.0:
        return int(analysis["insufficient_signal_key_length"])
    threshold = best_score * float(analysis["ic_near_max_ratio"])
    return next(key_length for key_length, score in scores if score >= threshold)


def _find_key(
    ciphertext: str,
    key_length: int,
    limits: dict[str, int],
    analysis: dict[str, object],
) -> str:
    if key_length <= 0:
        return ""
    if key_length > limits["max_vigenere_key_length"]:
        raise ConformanceError("vigenere-key-length-limit")
    if len(ciphertext) > limits["max_vigenere_analysis_scalars"]:
        raise ConformanceError("vigenere-analysis-limit")
    frequencies = [float(value) for value in analysis["english_frequencies"]]  # type: ignore[arg-type]
    letters = _ascii_upper(ciphertext)
    recovered: list[str] = []
    for position in range(key_length):
        group = _position_group(letters, key_length, position)
        if not group:
            recovered.append(str(analysis["empty_group_key_letter"]))
            continue
        best_shift = 0
        best_score = math.inf
        for shift in range(26):
            counts = [0] * 26
            for letter in group:
                counts[(letter - 0x41 - shift) % 26] += 1
            total = len(group)
            score = sum(
                (counts[index] - total * frequencies[index]) ** 2
                / (total * frequencies[index])
                for index in range(26)
            )
            if score < best_score:
                best_score = score
                best_shift = shift
        recovered.append(chr(0x41 + best_shift))
    return "".join(recovered)


def _break_cipher(
    ciphertext: str,
    limits: dict[str, int],
    analysis: dict[str, object],
) -> dict[str, str]:
    if len(ciphertext) > limits["max_vigenere_analysis_scalars"]:
        raise ConformanceError("vigenere-analysis-limit")
    key_length = _find_key_length(ciphertext, 20, limits, analysis)
    key = _find_key(ciphertext, key_length, limits, analysis)
    return {"key": key, "plaintext": _vigenere_transform(ciphertext, key, -1)}


def _resolve_text(case_input: dict[str, object], field: str) -> str:
    direct = case_input.get(field)
    if isinstance(direct, str):
        return direct
    scalar = case_input["repeat_scalar"]
    count = case_input["repeat_count"]
    if not isinstance(scalar, str) or not isinstance(count, int):
        raise TypeError("closed schema guarantees one scalar and an integer count")
    return scalar * count


def _execute(case: dict[str, Any], document: dict[str, Any]) -> dict[str, object]:
    operation = case["operation"]
    case_input = case["input"]
    limits = document["limits"]
    analysis = document["analysis"]
    try:
        if operation == "atbash-transform":
            return {"text": _atbash(case_input["text"])}
        if operation == "scytale-encrypt":
            return {"text": _scytale_encrypt(case_input["text"], case_input["key"])}
        if operation == "scytale-decrypt":
            return {"text": _scytale_decrypt(case_input["text"], case_input["key"])}
        if operation == "scytale-brute-force":
            text = _resolve_text(case_input, "text")
            return {
                "candidates": _scytale_brute_force(
                    text, limits["max_scytale_brute_force_scalars"]
                )
            }
        if operation in {"vigenere-encrypt", "vigenere-decrypt"}:
            direction = 1 if operation == "vigenere-encrypt" else -1
            return {
                "text": _vigenere_transform(
                    case_input["text"], case_input["key"], direction
                )
            }
        if operation == "vigenere-find-key-length":
            ciphertext = _resolve_text(case_input, "ciphertext")
            return {
                "key_length": _find_key_length(
                    ciphertext, case_input["max_length"], limits, analysis
                )
            }
        if operation == "vigenere-find-key":
            ciphertext = _resolve_text(case_input, "ciphertext")
            return {
                "key": _find_key(ciphertext, case_input["key_length"], limits, analysis)
            }
        if operation == "vigenere-break":
            ciphertext = _resolve_text(case_input, "ciphertext")
            return _break_cipher(ciphertext, limits, analysis)
    except ConformanceError as error:
        return {"error_id": error.error_id}
    raise AssertionError(f"unhandled closed operation: {operation}")


class ClassicalCipherFixtureTests(unittest.TestCase):
    schema: ClassVar[dict[str, Any]]
    document: ClassVar[dict[str, Any]]
    validator: ClassVar[Any]
    encoded: ClassVar[bytes]

    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((FIXTURE_ROOT / "schema.json").read_text("utf-8"))
        cls.encoded = (FIXTURE_ROOT / "cases.json").read_bytes()
        cls.document = json.loads(cls.encoded)
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def test_schema_profile_limits_and_ids_are_closed(self) -> None:
        self.validator.validate(self.document)
        _validate_document(self.document, len(self.encoded))
        ids = [case["id"] for case in self.document["cases"]]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertLessEqual(len(ids), self.document["limits"]["max_cases"])
        self.assertEqual(
            {case["operation"] for case in self.document["cases"]},
            {
                "atbash-transform",
                "scytale-encrypt",
                "scytale-decrypt",
                "scytale-brute-force",
                "vigenere-encrypt",
                "vigenere-decrypt",
                "vigenere-find-key-length",
                "vigenere-find-key",
                "vigenere-break",
            },
        )

    def test_analysis_constants_are_exact(self) -> None:
        analysis = self.document["analysis"]
        self.assertEqual(analysis["ic_near_max_ratio"], 0.9)
        self.assertEqual(analysis["key_length_tie_break"], "smallest-near-maximum")
        self.assertEqual(analysis["chi_squared_tie_break"], "smallest-shift")
        self.assertEqual(analysis["insufficient_signal_key_length"], 1)
        self.assertEqual(analysis["empty_group_key_letter"], "A")
        self.assertFalse(analysis["shorten_repeating_key"])
        self.assertEqual(len(analysis["english_frequencies"]), 26)

    def test_stable_errors_are_complete_and_payload_blind(self) -> None:
        self.assertEqual(
            set(self.document["operation_error_ids"]),
            {
                "scytale-invalid-key",
                "scytale-brute-force-limit",
                "vigenere-invalid-key",
                "vigenere-analysis-limit",
                "vigenere-key-length-limit",
            },
        )
        self.assertEqual(
            set(self.document["validation_error_ids"]),
            {"fixture-duplicate-id", "fixture-invalid-scalar", "fixture-size-limit"},
        )
        exercised = {
            case["expected"]["error_id"]
            for case in self.document["cases"]
            if "error_id" in case["expected"]
        }
        self.assertEqual(exercised, set(self.document["operation_error_ids"]))
        for error_id in (
            *self.document["operation_error_ids"],
            *self.document["validation_error_ids"],
        ):
            self.assertRegex(error_id, r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

    def test_fixture_validation_rejects_duplicate_ids_surrogates_and_size(self) -> None:
        duplicate = copy.deepcopy(self.document)
        duplicate["cases"][1]["id"] = duplicate["cases"][0]["id"]
        with self.assertRaisesRegex(ConformanceError, "^fixture-duplicate-id$"):
            _validate_document(duplicate, len(self.encoded))

        surrogate = copy.deepcopy(self.document)
        surrogate["cases"][0]["input"]["text"] = chr(0xD800)
        with self.assertRaisesRegex(ConformanceError, "^fixture-invalid-scalar$"):
            _validate_document(surrogate, len(self.encoded))

        with self.assertRaisesRegex(ConformanceError, "^fixture-size-limit$"):
            _validate_document(
                self.document, self.document["limits"]["max_fixture_bytes"] + 1
            )

    def test_semantic_oracle_matches_every_case(self) -> None:
        for case in self.document["cases"]:
            with self.subTest(case=case["id"]):
                self.assertEqual(_execute(case, self.document), case["expected"])


if __name__ == "__main__":
    unittest.main()
