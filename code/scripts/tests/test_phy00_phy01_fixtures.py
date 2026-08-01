from __future__ import annotations

import base64
import copy
import json
import math
import re
import unittest
from collections.abc import Iterator
from decimal import Decimal
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = ROOT / "code" / "specs" / "fixtures" / "phy00-phy01-v1"
SCHEMA_PATH = FIXTURE_ROOT / "schema.json"
CASE_ROOT = FIXTURE_ROOT / "cases"
GENERATED_DART_PATH = FIXTURE_ROOT / "dart" / "generated_cases.dart"
MAX_SAFE_INTEGER = 9_007_199_254_740_991
MAX_TOLERANCE = 1e-10
EXPECTED_CASE_FILES = {"trig.json", "wave.json"}
EXPECTED_SUITES = {"phy00-trig", "phy01-wave"}


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_non_finite(value: str) -> None:
    raise ValueError(f"non-finite JSON number: {value}")


def _validate_json_value(value: Any, depth: int = 0) -> None:
    if depth > 32:
        raise ValueError("JSON nesting exceeds 32 levels")
    if isinstance(value, str):
        if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
            raise ValueError("unpaired Unicode surrogate")
        return
    if value is None or isinstance(value, bool):
        return
    if isinstance(value, int):
        if not -MAX_SAFE_INTEGER <= value <= MAX_SAFE_INTEGER:
            raise ValueError("integer outside interoperable range")
        return
    if isinstance(value, float):
        raise TypeError("floating-point JSON values are forbidden")
    if isinstance(value, list):
        for item in value:
            _validate_json_value(item, depth + 1)
        return
    if isinstance(value, dict):
        for key, item in value.items():
            _validate_json_value(key, depth + 1)
            _validate_json_value(item, depth + 1)
        return
    raise ValueError(f"unsupported JSON value: {type(value).__name__}")


def strict_loads(raw: bytes, max_bytes: int = 1_000_000) -> dict[str, Any]:
    if len(raw) > max_bytes:
        raise ValueError("JSON input exceeds byte limit")
    if raw.startswith(b"\xef\xbb\xbf"):
        raise ValueError("UTF-8 BOM is forbidden")
    text = raw.decode("utf-8", errors="strict")
    value = json.loads(
        text,
        object_pairs_hook=_reject_duplicate_pairs,
        parse_constant=_reject_non_finite,
    )
    _validate_json_value(value)
    if not isinstance(value, dict):
        raise TypeError("top-level JSON value must be an object")
    return value


def load_json(path: Path) -> dict[str, Any]:
    return strict_loads(path.read_bytes())


def validate_document_set(paths: list[Path], documents: list[dict[str, Any]]) -> None:
    names = {path.name for path in paths}
    if len(paths) != len(EXPECTED_CASE_FILES) or names != EXPECTED_CASE_FILES:
        raise ValueError("case file set must be exactly trig.json and wave.json")
    suites = [document.get("suite") for document in documents]
    if len(suites) != len(EXPECTED_SUITES) or set(suites) != EXPECTED_SUITES:
        raise ValueError("case documents must contain exactly one document per suite")


def iter_scalars(value: Any) -> Iterator[dict[str, Any]]:
    if isinstance(value, dict):
        kind = value.get("kind")
        if kind in {"finite", "positive-infinity", "negative-infinity", "nan"}:
            yield value
            return
        for item in value.values():
            yield from iter_scalars(item)
    elif isinstance(value, list):
        for item in value:
            yield from iter_scalars(item)


def decode_scalar(value: dict[str, Any]) -> float:
    kind = value["kind"]
    if kind == "finite":
        decoded = float(value["decimal"])
        if not math.isfinite(decoded):
            raise ValueError("finite decimal is outside binary64 range")
        if decoded == 0.0 and not Decimal(value["decimal"]).is_zero():
            raise ValueError("nonzero finite decimal underflows binary64")
        return decoded
    if kind == "positive-infinity":
        return math.inf
    if kind == "negative-infinity":
        return -math.inf
    if kind == "nan":
        return math.nan
    raise ValueError(f"unknown scalar kind: {kind}")


def compare_value(
    actual: float,
    expected_scalar: dict[str, Any],
    comparison: dict[str, Any],
) -> bool:
    expected = decode_scalar(expected_scalar)
    kind = comparison["kind"]
    if kind == "exact":
        if math.isnan(expected):
            return math.isnan(actual)
        if expected == 0.0:
            return actual == 0.0 and math.copysign(1.0, actual) == math.copysign(
                1.0, expected
            )
        return actual == expected

    tolerance = decode_tolerance(comparison["tolerance"])
    if not math.isfinite(actual) or not math.isfinite(expected):
        return False
    error = abs(actual - expected)
    if kind == "absolute":
        return error <= tolerance
    if kind == "relative":
        return error <= tolerance * abs(expected)
    raise ValueError(f"unknown comparison kind: {kind}")


def decode_tolerance(decimal: str) -> float:
    tolerance = float(decimal)
    if not math.isfinite(tolerance):
        raise ValueError("tolerance is outside binary64 range")
    if tolerance <= 0.0:
        raise ValueError("tolerance must remain positive in binary64")
    if tolerance > MAX_TOLERANCE:
        raise ValueError("tolerance exceeds the PHY00/PHY01 maximum")
    return tolerance


def canonical_base64(document: dict[str, Any]) -> str:
    canonical = json.dumps(
        document,
        ensure_ascii=True,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("ascii")
    return base64.b64encode(canonical).decode("ascii")


def generated_dart_constant(source: str, name: str) -> str:
    match = re.search(
        rf"const String {re.escape(name)} =\s*((?:\s*'[A-Za-z0-9+/=]*')+);",
        source,
    )
    if match is None:
        raise AssertionError(f"missing generated Dart constant: {name}")
    return "".join(re.findall(r"'([A-Za-z0-9+/=]*)'", match.group(1)))


def reference_trig(case: dict[str, Any]) -> float:
    operation = case["operation"]
    inputs = case["input"]
    if operation == "constant":
        return math.pi
    if operation == "atan2":
        result = math.atan2(decode_scalar(inputs["y"]), decode_scalar(inputs["x"]))
        return math.pi if result == -math.pi else result
    if operation == "radians":
        return math.radians(decode_scalar(inputs["degrees"]))
    if operation == "degrees":
        return math.degrees(decode_scalar(inputs["radians"]))

    value = decode_scalar(inputs["x"])
    if operation == "sin":
        return math.sin(value)
    if operation == "cos":
        return math.cos(value)
    if operation == "tan":
        sine = math.sin(value)
        cosine = math.cos(value)
        if abs(cosine) < 1e-15:
            return 1e308 if sine > 0.0 else -1e308
        return sine / cosine
    if operation == "sqrt":
        if value < 0.0:
            raise ValueError("invalid-argument")
        return math.sqrt(value)
    if operation == "atan":
        return math.atan(value)
    raise ValueError(f"unknown PHY00 operation: {operation}")


def decode_wave(case: dict[str, Any]) -> tuple[float, float, float]:
    parameters = case["input"]["wave"]
    amplitude = decode_scalar(parameters["amplitude"])
    frequency = decode_scalar(parameters["frequency"])
    phase = decode_scalar(parameters["phase"])
    if not math.isfinite(amplitude) or amplitude < 0.0:
        raise ValueError("invalid-argument")
    if not math.isfinite(frequency) or frequency <= 0.0:
        raise ValueError("invalid-argument")
    if not math.isfinite(2.0 * math.pi * frequency):
        raise ValueError("invalid-argument")
    if not math.isfinite(phase):
        raise ValueError("invalid-argument")
    return amplitude, frequency, phase


def reference_wave(case: dict[str, Any]) -> float | None:
    amplitude, frequency, phase = decode_wave(case)
    operation = case["operation"]
    if operation == "construct":
        return None
    if operation == "period":
        return 1.0 / frequency
    if operation == "angular-frequency":
        return 2.0 * math.pi * frequency
    if operation != "evaluate":
        raise ValueError(f"unknown PHY01 operation: {operation}")

    time = decode_scalar(case["input"]["time"])
    if not math.isfinite(time):
        raise ValueError("invalid-argument")
    if amplitude == 0.0:
        return 0.0
    period = 1.0 / frequency
    reduced_time = time if math.isinf(period) else math.fmod(time, period)
    reduced_phase = math.fmod(phase, 2.0 * math.pi)
    angle = 2.0 * math.pi * (frequency * reduced_time) + reduced_phase
    unit_value = math.sin(angle)
    if unit_value >= 1.0:
        return amplitude
    if unit_value <= -1.0:
        return -amplitude
    return amplitude * unit_value


class Phy00Phy01FixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load_json(SCHEMA_PATH)
        cls.case_paths = sorted(CASE_ROOT.glob("*.json"))
        cls.documents = [load_json(path) for path in cls.case_paths]
        validate_document_set(cls.case_paths, cls.documents)
        cls.cases = [case for document in cls.documents for case in document["cases"]]

    def test_schema_is_draft_2020_12_and_closed(self) -> None:
        self.assertEqual(
            self.schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema",
        )
        self.assertFalse(self.schema["additionalProperties"])
        self.assertEqual(self.schema["properties"]["schema_version"], {"const": 1})
        self.assertEqual(
            set(self.schema["required"]),
            {"schema_version", "suite", "summary", "cases"},
        )

    def test_checked_in_documents_are_formally_valid(self) -> None:
        import jsonschema

        jsonschema.Draft202012Validator.check_schema(self.schema)
        validator = jsonschema.Draft202012Validator(self.schema)
        for document in self.documents:
            errors = sorted(
                validator.iter_errors(document), key=lambda error: list(error.path)
            )
            self.assertEqual(
                [],
                errors,
                "\n".join(f"{list(error.path)}: {error.message}" for error in errors),
            )

    def test_case_identities_and_suite_membership_are_closed(self) -> None:
        ids = [case["id"] for case in self.cases]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(
            {document["schema_version"] for document in self.documents}, {1}
        )
        self.assertEqual(
            {document["suite"] for document in self.documents},
            {"phy00-trig", "phy01-wave"},
        )
        for document in self.documents:
            prefix = "phy00/" if document["suite"] == "phy00-trig" else "phy01/"
            self.assertTrue(
                all(case["id"].startswith(prefix) for case in document["cases"])
            )

    def test_document_set_rejects_extra_files_and_duplicate_suites(self) -> None:
        with self.assertRaisesRegex(ValueError, "file set"):
            validate_document_set(
                [*self.case_paths, CASE_ROOT / "extra.json"],
                [*self.documents, copy.deepcopy(self.documents[0])],
            )
        duplicate_suite = [
            copy.deepcopy(self.documents[0]),
            copy.deepcopy(self.documents[0]),
        ]
        with self.assertRaisesRegex(ValueError, "one document per suite"):
            validate_document_set(self.case_paths, duplicate_suite)

    def test_required_boundary_cases_are_present(self) -> None:
        required = {
            "phy00/tan/positive-pole",
            "phy00/atan2/negative-x-negative-zero",
            "phy00/sqrt/tiny-normal",
            "phy00/sqrt/minimum-subnormal",
            "phy00/sqrt/maximum-finite",
            "phy00/sqrt/negative-zero",
            "phy00/sqrt/positive-infinity",
            "phy00/sqrt/nan",
            "phy01/construct/angular-frequency-overflow",
            "phy01/period/minimum-subnormal-frequency",
            "phy01/evaluate/periodic-next-cycle",
            "phy01/evaluate/nan-time",
            "phy01/evaluate/zero-amplitude-extreme",
            "phy01/evaluate/extreme-finite-bounded",
            "phy01/evaluate/subnormal-period-overflow",
        }
        self.assertTrue(required.issubset({case["id"] for case in self.cases}))

    def test_tagged_scalars_decode_to_the_claimed_binary64_class(self) -> None:
        scalars = [
            scalar for document in self.documents for scalar in iter_scalars(document)
        ]
        self.assertTrue(
            any(scalar == {"kind": "finite", "decimal": "-0"} for scalar in scalars)
        )
        self.assertTrue(any(scalar["kind"] == "nan" for scalar in scalars))
        self.assertTrue(
            any(scalar["kind"] == "positive-infinity" for scalar in scalars)
        )
        for scalar in scalars:
            decoded = decode_scalar(scalar)
            if scalar["kind"] == "finite":
                self.assertTrue(math.isfinite(decoded))
                if scalar["decimal"] == "-0":
                    self.assertEqual(math.copysign(1.0, decoded), -1.0)

    def test_tolerances_are_finite_positive_and_bounded(self) -> None:
        for case in self.cases:
            comparison = case["expected"].get("comparison")
            if comparison is not None and "tolerance" in comparison:
                self.assertLessEqual(
                    decode_tolerance(comparison["tolerance"]), MAX_TOLERANCE
                )

    def test_outcome_kinds_are_operation_appropriate(self) -> None:
        for case in self.cases:
            outcome = case["expected"]["outcome"]
            if outcome == "accepted":
                self.assertEqual(case["operation"], "construct")
            if outcome == "property":
                self.assertEqual(case["operation"], "evaluate")
                self.assertIn(
                    case["id"],
                    {
                        "phy01/evaluate/extreme-finite-bounded",
                        "phy01/evaluate/subnormal-period-overflow",
                    },
                )

    def test_reference_calculation_agrees_with_every_case(self) -> None:
        for case in self.cases:
            expected = case["expected"]
            try:
                actual = (
                    reference_trig(case)
                    if case["id"].startswith("phy00/")
                    else reference_wave(case)
                )
            except ValueError as error:
                self.assertEqual(expected["outcome"], "error", case["id"])
                self.assertEqual(str(error), expected["error_code"], case["id"])
                continue

            self.assertNotEqual(expected["outcome"], "error", case["id"])
            if expected["outcome"] == "accepted":
                self.assertIsNone(actual, case["id"])
            elif expected["outcome"] == "value":
                self.assertIsInstance(actual, float, case["id"])
                self.assertTrue(
                    compare_value(actual, expected["value"], expected["comparison"]),
                    case["id"],
                )
            else:
                self.assertEqual(expected["outcome"], "property", case["id"])
                self.assertIsInstance(actual, float, case["id"])
                self.assertTrue(math.isfinite(actual), case["id"])
                amplitude = decode_scalar(case["input"]["wave"]["amplitude"])
                self.assertLessEqual(abs(actual), amplitude, case["id"])

    def test_schema_rejects_cross_suite_and_unknown_fields(self) -> None:
        import jsonschema

        validator = jsonschema.Draft202012Validator(self.schema)
        wrong_suite = copy.deepcopy(self.documents[0])
        wrong_suite["suite"] = "phy01-wave"
        self.assertTrue(list(validator.iter_errors(wrong_suite)))

        unknown = copy.deepcopy(self.documents[0])
        unknown["unexpected"] = True
        self.assertTrue(list(validator.iter_errors(unknown)))

    def test_schema_rejects_operation_inappropriate_outcomes(self) -> None:
        import jsonschema

        validator = jsonschema.Draft202012Validator(self.schema)
        trig_document = next(
            document for document in self.documents if document["suite"] == "phy00-trig"
        )
        wave_document = next(
            document for document in self.documents if document["suite"] == "phy01-wave"
        )

        accepted_trig = copy.deepcopy(trig_document)
        accepted_trig["cases"][0]["expected"] = {"outcome": "accepted"}
        self.assertTrue(list(validator.iter_errors(accepted_trig)))

        property_construct = copy.deepcopy(wave_document)
        property_construct["cases"][0]["expected"] = {
            "outcome": "property",
            "predicate": "finite-absolute-not-greater-than-amplitude",
        }
        self.assertTrue(list(validator.iter_errors(property_construct)))

        approximate_infinity = copy.deepcopy(trig_document)
        infinity_case = next(
            case
            for case in approximate_infinity["cases"]
            if case["id"] == "phy00/sqrt/positive-infinity"
        )
        infinity_case["expected"]["comparison"] = {
            "kind": "absolute",
            "tolerance": "1e-10",
        }
        self.assertTrue(list(validator.iter_errors(approximate_infinity)))

    def test_binary64_semantics_reject_overflow_and_underflow(self) -> None:
        with self.assertRaisesRegex(ValueError, "outside binary64 range"):
            decode_scalar({"kind": "finite", "decimal": "1e999"})
        with self.assertRaisesRegex(ValueError, "underflows binary64"):
            decode_scalar({"kind": "finite", "decimal": "1e-999"})
        with self.assertRaisesRegex(ValueError, "outside binary64 range"):
            decode_tolerance("1e999")
        with self.assertRaisesRegex(ValueError, "remain positive"):
            decode_tolerance("1e-999")
        with self.assertRaisesRegex(ValueError, "exceeds"):
            decode_tolerance("1e-9")

    def test_generated_dart_representation_matches_strict_documents(self) -> None:
        source = GENERATED_DART_PATH.read_text(encoding="utf-8")
        by_suite = {document["suite"]: document for document in self.documents}
        self.assertEqual(
            generated_dart_constant(source, "phy00TrigFixtureBase64"),
            canonical_base64(by_suite["phy00-trig"]),
        )
        self.assertEqual(
            generated_dart_constant(source, "phy01WaveFixtureBase64"),
            canonical_base64(by_suite["phy01-wave"]),
        )

    def test_strict_loader_rejects_ambiguous_json(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            strict_loads(b'{"schema_version":1,"schema_version":1}')
        with self.assertRaisesRegex(ValueError, "UTF-8 BOM"):
            strict_loads(b"\xef\xbb\xbf{}")
        with self.assertRaisesRegex(TypeError, "floating-point JSON"):
            strict_loads(b'{"value":1.5}')
        with self.assertRaisesRegex(ValueError, "non-finite JSON number"):
            strict_loads(b'{"value":NaN}')


if __name__ == "__main__":
    unittest.main()
