"""Prove that every established ZIP lane consumes the closed raw profile."""

from __future__ import annotations

import importlib
import json
import sys
import unittest
from copy import deepcopy
from pathlib import Path, PurePosixPath
from typing import Any

from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/zip-raw-rfc1951-v1"
sys.path.insert(0, str(REPO_ROOT / "code/scripts"))
IMPLEMENTATION_LANGUAGES = importlib.import_module(
    "package_parity_report"
).IMPLEMENTATION_LANGUAGES

EXPECTED_CONSUMERS: dict[str, dict[str, object]] = {
    "csharp": {
        "api_source": "code/packages/csharp/zip/RawRfc1951.cs",
        "fixture_test": "code/packages/csharp/zip/tests/CodingAdventures.Zip.Tests/PortableConformanceTests.cs",
        "build_files": [
            "code/packages/csharp/zip/BUILD",
            "code/packages/csharp/zip/BUILD_windows",
        ],
        "surface": [
            "RawDeflate",
            "RawInflate",
            "RawInflateCounted",
            "MaxOutput",
            "ErrorCodes",
        ],
    },
    "dart": {
        "api_source": "code/packages/dart/zip/lib/coding_adventures_zip.dart",
        "fixture_test": "code/packages/dart/zip/test/portable_conformance_test.dart",
        "build_files": ["code/packages/dart/zip/BUILD"],
        "surface": [
            "rawDeflate",
            "rawInflate",
            "rawInflateCounted",
            "rawInflateMaxOutput",
            "RawInflateError",
        ],
    },
    "elixir": {
        "api_source": "code/packages/elixir/zip/lib/coding_adventures/zip.ex",
        "fixture_test": "code/packages/elixir/zip/test/portable_conformance_test.exs",
        "build_files": ["code/packages/elixir/zip/BUILD"],
        "surface": [
            "raw_deflate",
            "raw_inflate",
            "raw_inflate_counted",
            "raw_inflate_max_output",
            "raw_inflate_error_codes",
        ],
    },
    "fsharp": {
        "api_source": "code/packages/fsharp/zip/Zip.fs",
        "fixture_test": "code/packages/fsharp/zip/tests/CodingAdventures.Zip.Tests/PortableConformanceTests.fs",
        "build_files": [
            "code/packages/fsharp/zip/BUILD",
            "code/packages/fsharp/zip/BUILD_windows",
        ],
        "surface": [
            "rawDeflate",
            "rawInflate",
            "rawInflateCounted",
            "maxOutput",
            "errorCodes",
        ],
    },
    "go": {
        "api_source": "code/packages/go/zip/zip.go",
        "fixture_test": "code/packages/go/zip/portable_conformance_test.go",
        "build_files": ["code/packages/go/zip/BUILD"],
        "surface": [
            "RawDeflate",
            "RawInflate",
            "RawInflateCounted",
            "RawInflateMaxOutput",
            "RawInflateErrorCode",
        ],
    },
    "haskell": {
        "api_source": "code/packages/haskell/zip/src/Zip.hs",
        "fixture_test": "code/packages/haskell/zip/test/PortableConformanceSpec.hs",
        "build_files": [
            "code/packages/haskell/zip/BUILD",
            "code/packages/haskell/zip/BUILD_windows",
        ],
        "surface": [
            "rawDeflate",
            "rawInflate",
            "rawInflateCounted",
            "rawInflateMaxOutput",
            "rawInflateErrorCodes",
        ],
    },
    "java": {
        "api_source": "code/packages/java/zip/src/main/java/com/codingadventures/zip/RawRfc1951.java",
        "fixture_test": "code/packages/java/zip/src/test/java/com/codingadventures/zip/RawRfc1951ConformanceTest.java",
        "build_files": [
            "code/packages/java/zip/BUILD",
            "code/packages/java/zip/BUILD_windows",
        ],
        "surface": [
            "rawDeflate",
            "rawInflate",
            "rawInflateCounted",
            "MAX_OUTPUT",
            "ERROR_CODES",
        ],
    },
    "kotlin": {
        "api_source": "code/packages/kotlin/zip/src/main/kotlin/com/codingadventures/zip/RawRfc1951.kt",
        "fixture_test": "code/packages/kotlin/zip/src/test/kotlin/com/codingadventures/zip/RawRfc1951ConformanceTest.kt",
        "build_files": [
            "code/packages/kotlin/zip/BUILD",
            "code/packages/kotlin/zip/BUILD_windows",
        ],
        "surface": [
            "rawDeflate",
            "rawInflate",
            "rawInflateCounted",
            "MAX_RAW_OUTPUT",
            "RAW_INFLATE_ERROR_CODES",
        ],
    },
    "lua": {
        "api_source": "code/packages/lua/zip/src/coding_adventures/zip/init.lua",
        "fixture_test": "code/packages/lua/zip/tests/test_portable_conformance.lua",
        "build_files": [
            "code/packages/lua/zip/BUILD",
            "code/packages/lua/zip/BUILD_windows",
        ],
        "surface": [
            "raw_deflate",
            "raw_inflate",
            "raw_inflate_counted",
            "RAW_INFLATE_MAX_OUTPUT",
            "RAW_INFLATE_ERROR_CODES",
        ],
    },
    "perl": {
        "api_source": "code/packages/perl/zip/lib/CodingAdventures/Zip.pm",
        "fixture_test": "code/packages/perl/zip/t/02-portable-conformance.t",
        "build_files": [
            "code/packages/perl/zip/BUILD",
            "code/packages/perl/zip/BUILD_windows",
        ],
        "surface": [
            "raw_deflate",
            "raw_inflate",
            "raw_inflate_counted",
            "RAW_INFLATE_MAX_OUTPUT",
            "raw_inflate_error_codes",
        ],
    },
    "python": {
        "api_source": "code/packages/python/zip/src/coding_adventures_zip/__init__.py",
        "fixture_test": "code/packages/python/zip/tests/test_portable_conformance.py",
        "build_files": [
            "code/packages/python/zip/BUILD",
            "code/packages/python/zip/BUILD_windows",
        ],
        "surface": [
            "raw_deflate",
            "raw_inflate",
            "raw_inflate_counted",
            "RAW_INFLATE_MAX_OUTPUT",
            "RAW_INFLATE_ERROR_CODES",
        ],
    },
    "ruby": {
        "api_source": "code/packages/ruby/zip/lib/coding_adventures_zip.rb",
        "fixture_test": "code/packages/ruby/zip/test/test_portable_conformance.rb",
        "build_files": ["code/packages/ruby/zip/BUILD"],
        "surface": [
            "raw_deflate",
            "raw_inflate",
            "raw_inflate_counted",
            "RAW_INFLATE_MAX_OUTPUT",
            "RAW_INFLATE_ERROR_CODES",
        ],
    },
    "rust": {
        "api_source": "code/packages/rust/zip/src/lib.rs",
        "fixture_test": "code/packages/rust/zip/tests/portable_conformance.rs",
        "build_files": ["code/packages/rust/zip/BUILD"],
        "surface": [
            "raw_deflate",
            "raw_inflate",
            "raw_inflate_counted",
            "RAW_INFLATE_MAX_OUTPUT",
            "RawInflateErrorCode",
        ],
    },
    "swift": {
        "api_source": "code/packages/swift/zip/Sources/Zip/Zip.swift",
        "fixture_test": "code/packages/swift/zip/Tests/ZipTests/PortableConformanceTests.swift",
        "build_files": [
            "code/packages/swift/zip/BUILD",
            "code/packages/swift/zip/BUILD_windows",
        ],
        "surface": [
            "rawDeflate",
            "rawInflate",
            "rawInflateCounted",
            "rawInflateMaxOutput",
            "rawInflateErrorCodes",
        ],
    },
    "typescript": {
        "api_source": "code/packages/typescript/zip/src/zip.ts",
        "fixture_test": "code/packages/typescript/zip/tests/portable-conformance.test.ts",
        "build_files": ["code/packages/typescript/zip/BUILD"],
        "surface": [
            "rawDeflate",
            "rawInflate",
            "rawInflateCounted",
            "RAW_INFLATE_MAX_OUTPUT",
            "RawInflateErrorCode",
        ],
    },
}


def load_json(path: Path) -> dict[str, Any]:
    """Load one repository-owned JSON document without accepting duplicates."""

    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        document: dict[str, Any] = {}
        for key, value in pairs:
            if key in document:
                raise ValueError(f"duplicate JSON key: {key}")
            document[key] = value
        return document

    return json.loads(path.read_text("utf-8"), object_pairs_hook=reject_duplicates)


def consumers_document() -> dict[str, Any]:
    return load_json(FIXTURE_ROOT / "consumers.json")


def safe_repo_file(value: str, package_root: str) -> Path:
    """Resolve a canonical package-local path and reject traversal or aliases."""

    pure = PurePosixPath(value)
    assert not pure.is_absolute()
    assert ".." not in pure.parts
    assert pure.as_posix() == value
    assert value.startswith(package_root + "/")
    target = (REPO_ROOT / value).resolve()
    assert target.is_relative_to((REPO_ROOT / package_root).resolve())
    assert target.is_file()
    return target


def check_consumer_registry_matches_closed_schema_and_denominator() -> None:
    schema = load_json(FIXTURE_ROOT / "consumers.schema.json")
    document = consumers_document()
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(document)

    lanes = [consumer["language"] for consumer in document["consumers"]]
    assert lanes == list(IMPLEMENTATION_LANGUAGES)
    assert lanes == list(EXPECTED_CONSUMERS)
    assert document["established_lane_count"] == len(lanes)


def check_registry_pins_package_paths_builds_and_public_surface() -> None:
    for consumer in consumers_document()["consumers"]:
        language = consumer["language"]
        expected = EXPECTED_CONSUMERS[language]
        package_root = f"code/packages/{language}/zip"
        assert consumer["package_root"] == package_root
        assert consumer["api_source"] == expected["api_source"]
        assert consumer["fixture_test"] == expected["fixture_test"]
        assert consumer["build_files"] == expected["build_files"]
        assert (
            consumer["capability_manifest"]
            == f"{package_root}/required_capabilities.json"
        )

        source = safe_repo_file(consumer["api_source"], package_root).read_text("utf-8")
        expected_surface = expected["surface"]
        assert list(consumer["surface"].values()) == expected_surface
        for token in expected_surface:
            assert token in source, f"{language} production source lost {token}"
        safe_repo_file(consumer["fixture_test"], package_root)
        for build_file in consumer["build_files"]:
            safe_repo_file(build_file, package_root)


def check_every_lane_loads_the_same_closed_fixture_and_declares_no_authority() -> None:
    fixture = consumers_document()["fixture"]
    cases = load_json(REPO_ROOT / fixture)
    assert len(cases["cases"]) == consumers_document()["case_count"] == 34
    assert len(cases["error_ids"]) == consumers_document()["error_id_count"] == 14

    for consumer in consumers_document()["consumers"]:
        package_root = consumer["package_root"]
        test_text = safe_repo_file(consumer["fixture_test"], package_root).read_text(
            "utf-8"
        )
        assert "zip-raw-rfc1951-v1" in test_text
        capabilities = load_json(
            safe_repo_file(consumer["capability_manifest"], package_root)
        )
        assert capabilities["capabilities"] == [], consumer["language"]


def check_schema_rejects_a_sixteenth_or_cross_wired_consumer() -> None:
    schema = load_json(FIXTURE_ROOT / "consumers.schema.json")
    document = deepcopy(consumers_document())
    document["consumers"].append(deepcopy(document["consumers"][0]))
    assert list(Draft202012Validator(schema).iter_errors(document))

    document = consumers_document()
    document["consumers"][0]["package_root"] = "code/packages/rust/zip"
    try:
        safe_repo_file(
            document["consumers"][0]["api_source"],
            document["consumers"][0]["package_root"],
        )
    except AssertionError:
        pass
    else:
        raise AssertionError("cross-wired consumer unexpectedly resolved")


class PortableCoverageTests(unittest.TestCase):
    """Exercise the closure registry through the CI unittest front door."""

    def test_consumer_registry_matches_closed_schema_and_denominator(self) -> None:
        check_consumer_registry_matches_closed_schema_and_denominator()

    def test_registry_pins_package_paths_builds_and_public_surface(self) -> None:
        check_registry_pins_package_paths_builds_and_public_surface()

    def test_every_lane_loads_same_fixture_and_declares_no_authority(self) -> None:
        check_every_lane_loads_the_same_closed_fixture_and_declares_no_authority()

    def test_schema_rejects_sixteenth_or_cross_wired_consumer(self) -> None:
        check_schema_rejects_a_sixteenth_or_cross_wired_consumer()
