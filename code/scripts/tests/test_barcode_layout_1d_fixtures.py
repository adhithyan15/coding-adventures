"""Validate the language-neutral barcode-layout-1d v1 corpus."""

from __future__ import annotations

import hashlib
import json
import math
import os
import runpy
import unittest
from itertools import pairwise
from pathlib import Path
from typing import Any, ClassVar

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/barcode-layout-1d-v1"
MAX_FIXTURE_BYTES = 131_072
MAX_FIXTURE_DEPTH = 8
MAX_SCHEMA_DEPTH = 24


class FixtureLoadError(ValueError):
    """Stable failure from the bounded test-only loader."""


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
        text = encoded.decode("utf-8")
    except UnicodeDecodeError as error:
        raise FixtureLoadError("fixture-invalid-json") from error
    depth = 0
    in_string = False
    escaped = False
    for character in text:
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
                raise ValueError("duplicate object key")
            result[key] = value
        return result

    try:
        return json.loads(
            encoded,
            object_pairs_hook=unique_object,
            parse_constant=lambda token: (_ for _ in ()).throw(ValueError(token)),
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


def _load(schema_encoded: bytes, document_encoded: bytes) -> dict[str, Any]:
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
    return document


class BarcodeLayoutFixtureTests(unittest.TestCase):
    document: ClassVar[dict[str, Any]]
    by_id: ClassVar[dict[str, dict[str, Any]]]

    @classmethod
    def setUpClass(cls) -> None:
        cls.schema_encoded = _read_bounded(FIXTURE_ROOT / "schema.json")
        cls.document_encoded = _read_bounded(FIXTURE_ROOT / "cases.json")
        cls.document = _load(cls.schema_encoded, cls.document_encoded)
        cls.by_id = {case["id"]: case for case in cls.document["cases"]}

    def test_envelope_limits_errors_and_operations_are_closed(self) -> None:
        self.assertEqual(
            {path.name for path in FIXTURE_ROOT.iterdir() if path.is_file()},
            {
                "CHANGELOG.md",
                "README.md",
                "cases.json",
                "generate_cases.py",
                "schema.json",
                "targets.json",
                "targets.schema.json",
            },
        )
        self.assertEqual(len(self.by_id), len(self.document["cases"]))
        self.assertEqual(
            self.document["limits"],
            {
                "max_pattern_scalars": 65_567,
                "max_runs": 40_979,
                "max_content_modules": 65_567,
                "max_quiet_zone_modules": 4_096,
                "max_symbols": 40_979,
                "max_label_scalars": 4_096,
                "max_metadata_entries": 64,
                "max_metadata_key_scalars": 128,
                "max_metadata_value_scalars": 4_096,
                "max_metadata_utf8_bytes": 65_536,
                "max_module_width": 8_192,
                "max_bar_height": 8_192,
                "max_color_scalars": 128,
                "max_cases": 64,
                "max_fixture_bytes": 131_072,
                "max_fixture_depth": 8,
            },
        )
        self.assertEqual(
            {case["operation"] for case in self.document["cases"]},
            {"expand-binary", "expand-width", "compute-layout", "project-scene"},
        )
        exercised = {
            case["expected"]["error"]
            for case in self.document["cases"]
            if "error" in case["expected"]
        }
        self.assertEqual(exercised, set(self.document["error_ids"]))

    def test_run_layout_and_scene_algebra_is_self_consistent(self) -> None:
        for case in self.document["cases"]:
            expected = case["expected"]
            if "runs" in expected:
                runs = expected["runs"]
                self.assertTrue(all(run["modules"] > 0 for run in runs), case["id"])
                for left, right in pairwise(runs):
                    self.assertNotEqual(left["color"], right["color"], case["id"])
                self.assertLessEqual(
                    sum(run["modules"] for run in runs),
                    self.document["limits"]["max_content_modules"],
                    case["id"],
                )
            if "layout" in expected:
                layout = expected["layout"]
                self.assertEqual(
                    layout["totalModules"],
                    layout["leftQuietZoneModules"]
                    + layout["contentModules"]
                    + layout["rightQuietZoneModules"],
                    case["id"],
                )
                for symbol in layout["symbolLayouts"]:
                    self.assertLess(
                        symbol["startModule"], symbol["endModule"], case["id"]
                    )
                    self.assertLessEqual(
                        symbol["endModule"], layout["contentModules"], case["id"]
                    )
            if "scene" in expected:
                scene = expected["scene"]
                metadata = scene["metadata"]
                self.assertEqual(
                    scene["width"], int(metadata["sceneWidthPx"]), case["id"]
                )
                self.assertEqual(
                    scene["height"], int(metadata["sceneHeightPx"]), case["id"]
                )
                for rectangle in scene["rectangles"]:
                    rect_metadata = rectangle["metadata"]
                    self.assertEqual(
                        rectangle["width"],
                        (
                            int(rect_metadata["moduleEnd"])
                            - int(rect_metadata["moduleStart"])
                        )
                        * int(metadata["moduleWidthPx"]),
                        case["id"],
                    )

    def test_large_run_digest_uses_the_normative_canonical_encoding(self) -> None:
        case = self.by_id["layout-v1-width-run-limit"]
        value = case["input"]
        count = value["repeat"]["count"]
        runs = [
            {
                "color": "bar" if index % 2 == 0 else "space",
                "modules": 1,
                "sourceLabel": value["sourceLabel"],
                "sourceIndex": value["sourceIndex"],
                "role": value["role"],
            }
            for index in range(count)
        ]
        encoded = json.dumps(
            runs, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode()
        digest = case["expected"]["runDigest"]
        self.assertEqual(digest["runCount"], count)
        self.assertEqual(digest["contentModules"], count)
        self.assertEqual(digest["firstRun"], runs[0])
        self.assertEqual(digest["lastRun"], runs[-1])
        self.assertEqual(digest["runsSha256"], hashlib.sha256(encoded).hexdigest())
        self.assertEqual(
            digest["runsSha256"],
            "6b5abd84e83b8cca52df0750f6a8ad78535e0a3056e553cea8faf00f217f4383",
        )

    def test_representative_policy_vectors_are_pinned(self) -> None:
        binary = self.by_id["layout-v1-binary-basic"]["expected"]["runs"]
        self.assertEqual(
            [(run["color"], run["modules"]) for run in binary],
            [("bar", 2), ("space", 1), ("bar", 1), ("space", 2)],
        )
        gap = self.by_id["layout-v1-layout-gap-span"]["expected"]["layout"]
        self.assertEqual(gap["symbolLayouts"][0]["endModule"], 4)
        scene = self.by_id["layout-v1-scene-default"]["expected"]["scene"]
        self.assertEqual((scene["width"], scene["height"]), (92, 120))
        self.assertEqual(scene["metadata"]["totalModules"], "23")
        metadata_limit = self.by_id["layout-v1-scene-metadata-entry-limit"]["expected"][
            "scene"
        ]["metadata"]
        self.assertEqual(len(metadata_limit), 74)

    def test_target_registry_is_complete_but_claims_no_conformance_yet(self) -> None:
        target_schema = _read_bounded(FIXTURE_ROOT / "targets.schema.json")
        targets = _load(target_schema, _read_bounded(FIXTURE_ROOT / "targets.json"))
        entries = targets["targets"]
        self.assertEqual(
            {entry["language"] for entry in entries},
            {
                "csharp",
                "fsharp",
                "go",
                "haskell",
                "perl",
                "python",
                "rust",
                "typescript",
            },
        )
        self.assertEqual(len(entries), 8)
        self.assertTrue(all(entry["status"] == "pending-adoption" for entry in entries))
        for entry in entries:
            self.assertTrue((REPO_ROOT / entry["package_root"]).is_dir())
            self.assertTrue((REPO_ROOT / entry["native_test_path"]).is_file())
            self.assertTrue(entry["known_divergences"])

        promoted = json.loads(json.dumps(targets))
        promoted["targets"][0]["status"] = "conformant"
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(target_schema, json.dumps(promoted).encode())

    def test_generator_is_byte_for_byte_clean(self) -> None:
        namespace = runpy.run_path(str(FIXTURE_ROOT / "generate_cases.py"))
        self.assertEqual(namespace["render_document"](), self.document_encoded)

    def test_bounded_loader_rejects_hostile_json_and_schema(self) -> None:
        with self.assertRaisesRegex(FixtureLoadError, "fixture-size-limit"):
            _load(self.schema_encoded, b" " * (MAX_FIXTURE_BYTES + 1))
        with self.assertRaisesRegex(FixtureLoadError, "fixture-depth-limit"):
            _load(self.schema_encoded, (b'{"x":' * 10) + b"0" + (b"}" * 10))
        duplicate = self.document_encoded.replace(
            b'"schema_version": 1,', b'"schema_version": 1, "schema_version": 1,', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-json"):
            _load(self.schema_encoded, duplicate)
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-json"):
            _load(self.schema_encoded, b'{"value": NaN}')
        surrogate = self.document_encoded.replace(
            b'"barcode-layout-1d-v1"', b'"\\ud800"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-invalid-scalar"):
            _load(self.schema_encoded, surrogate)
        external = self.schema_encoded.replace(
            b'"#/$defs/case"', b'"https://example.invalid/case.json"', 1
        )
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(external, self.document_encoded)

    def test_schema_couples_operations_inputs_and_outputs(self) -> None:
        missing_input = json.loads(self.document_encoded)
        missing_input["cases"][0]["input"] = {}
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(self.schema_encoded, json.dumps(missing_input).encode())

        wrong_output = json.loads(self.document_encoded)
        wrong_output["cases"][0]["expected"] = self.by_id["layout-v1-scene-default"][
            "expected"
        ]
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(self.schema_encoded, json.dumps(wrong_output).encode())

        invalid_success_run = json.loads(self.document_encoded)
        invalid_success_run["cases"][0]["expected"]["runs"][0]["modules"] = 0
        with self.assertRaisesRegex(FixtureLoadError, "fixture-schema-invalid"):
            _load(self.schema_encoded, json.dumps(invalid_success_run).encode())


if __name__ == "__main__":
    unittest.main()
