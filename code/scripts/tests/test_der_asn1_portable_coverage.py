"""Prove that every established lane consumes typed DER value v1 fixtures."""

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
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/der-asn1-v1"
sys.path.insert(0, str(REPO_ROOT / "code/scripts"))
IMPLEMENTATION_LANGUAGES = importlib.import_module(
    "package_parity_report"
).IMPLEMENTATION_LANGUAGES

# language: package spelling, source, test, builds, six production tokens
EXPECTED: dict[str, tuple[str, str, str, list[str], list[str]]] = {
    "csharp": (
        "der-asn1",
        "DerAsn1.cs",
        "tests/CodingAdventures.DerAsn1.CSharp.Tests/PortableConformanceTests.cs",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "DecodeBoolean",
        ],
    ),
    "dart": (
        "der-asn1",
        "lib/src/der_asn1.dart",
        "test/der_asn1_test.dart",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decodeBoolean",
        ],
    ),
    "elixir": (
        "der_asn1",
        "lib/coding_adventures/der_asn1.ex",
        "test/der_asn1_test.exs",
        ["BUILD", "BUILD_windows"],
        ["Decoder", "Element", "Cursor", "default_limits", "Error", "decode_boolean"],
    ),
    "fsharp": (
        "der-asn1",
        "DerAsn1.fs",
        "tests/CodingAdventures.DerAsn1.FSharp.Tests/PortableConformanceTests.fs",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1Error",
            "decodeBoolean",
        ],
    ),
    "go": (
        "der-asn1",
        "der_asn1.go",
        "der_asn1_test.go",
        ["BUILD"],
        [
            "ASN1Decoder",
            "ASN1Element",
            "ASN1Cursor",
            "ASN1Limits",
            "ErrorKind",
            "DecodeBoolean",
        ],
    ),
    "haskell": (
        "der-asn1",
        "src/CodingAdventures/DerAsn1.hs",
        "test/PortableConformanceSpec.hs",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decodeBoolean",
        ],
    ),
    "java": (
        "der-asn1",
        "src/main/java/com/codingadventures/derasn1/DerAsn1.java",
        "src/test/java/com/codingadventures/derasn1/DerAsn1Test.java",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decodeBoolean",
        ],
    ),
    "kotlin": (
        "der-asn1",
        "src/main/kotlin/com/codingadventures/derasn1/DerAsn1.kt",
        "src/test/kotlin/com/codingadventures/derasn1/DerAsn1Test.kt",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decodeBoolean",
        ],
    ),
    "lua": (
        "der_asn1",
        "src/coding_adventures/der_asn1/init.lua",
        "tests/test_der_asn1.lua",
        ["BUILD", "BUILD_windows"],
        ["Decoder", "Element", "Cursor", "default_limits", "Error", "decode_boolean"],
    ),
    "perl": (
        "der-asn1",
        "lib/CodingAdventures/DerAsn1.pm",
        "t/portable-conformance.t",
        ["BUILD", "BUILD_windows"],
        [
            "CodingAdventures::DerAsn1::Decoder",
            "CodingAdventures::DerAsn1::Element",
            "CodingAdventures::DerAsn1::Cursor",
            "default_limits",
            "CodingAdventures::DerAsn1::Error",
            "decode_boolean",
        ],
    ),
    "python": (
        "der-asn1",
        "src/der_asn1/__init__.py",
        "tests/test_der_asn1.py",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decode_boolean",
        ],
    ),
    "ruby": (
        "der_asn1",
        "lib/coding_adventures_der_asn1.rb",
        "test/test_der_asn1.rb",
        ["BUILD", "BUILD_windows"],
        ["Decoder", "Element", "Cursor", "DEFAULT_LIMITS", "Error", "decode_boolean"],
    ),
    "rust": (
        "der-asn1",
        "src/lib.rs",
        "tests/portable_conformance.rs",
        ["BUILD"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decode_boolean",
        ],
    ),
    "swift": (
        "der-asn1",
        "Sources/DerAsn1/DerAsn1.swift",
        "Tests/DerAsn1Tests/DerAsn1Tests.swift",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decodeBoolean",
        ],
    ),
    "typescript": (
        "der-asn1",
        "src/index.ts",
        "tests/der-asn1.test.ts",
        ["BUILD", "BUILD_windows"],
        [
            "Asn1Decoder",
            "Asn1Element",
            "Asn1Cursor",
            "Asn1Limits",
            "Asn1ErrorKind",
            "decodeBoolean",
        ],
    ),
}


def load_json(path: Path) -> dict[str, Any]:
    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    return json.loads(path.read_text("utf-8"), object_pairs_hook=reject_duplicates)


def consumers_document() -> dict[str, Any]:
    return load_json(FIXTURE_ROOT / "consumers.json")


def safe_repo_file(value: str, package_root: str) -> Path:
    pure = PurePosixPath(value)
    assert not pure.is_absolute()
    assert ".." not in pure.parts
    assert pure.as_posix() == value
    assert value.startswith(package_root + "/")
    target = (REPO_ROOT / value).resolve()
    assert target.is_relative_to((REPO_ROOT / package_root).resolve())
    assert target.is_file(), value
    return target


def check_registry_schema_and_denominator() -> None:
    schema = load_json(FIXTURE_ROOT / "consumers.schema.json")
    document = consumers_document()
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(document)
    lanes = [consumer["language"] for consumer in document["consumers"]]
    assert lanes == list(IMPLEMENTATION_LANGUAGES)
    assert lanes == list(EXPECTED)
    assert document["established_lane_count"] == len(lanes) == 15


def check_paths_builds_and_surface() -> None:
    surface_keys = [
        "decoder",
        "element",
        "cursor",
        "limits",
        "error_taxonomy",
        "typed_values",
    ]
    for consumer in consumers_document()["consumers"]:
        language = consumer["language"]
        spelling, source, test, builds, tokens = EXPECTED[language]
        package_root = f"code/packages/{language}/{spelling}"
        assert consumer["package_root"] == package_root
        assert consumer["api_source"] == f"{package_root}/{source}"
        assert consumer["fixture_test"] == f"{package_root}/{test}"
        assert consumer["build_files"] == [f"{package_root}/{item}" for item in builds]
        assert (
            consumer["capability_manifest"]
            == f"{package_root}/required_capabilities.json"
        )
        assert list(consumer["surface"]) == surface_keys
        assert list(consumer["surface"].values()) == tokens
        source_text = safe_repo_file(consumer["api_source"], package_root).read_text(
            "utf-8"
        )
        for token in tokens:
            assert token in source_text, f"{language} production source lost {token}"
        safe_repo_file(consumer["fixture_test"], package_root)
        for build in consumer["build_files"]:
            safe_repo_file(build, package_root)


def check_fixture_and_capabilities() -> None:
    document = consumers_document()
    fixture = load_json(REPO_ROOT / document["fixture"])
    capability_schema = load_json(
        REPO_ROOT / "code/specs/schemas/required_capabilities.schema.json"
    )
    validator = Draft202012Validator(capability_schema)
    assert len(fixture["cases"]) == document["case_count"] == 122
    assert len(fixture["error_ids"]) == document["error_id_count"] == 22
    for consumer in document["consumers"]:
        package_root = consumer["package_root"]
        test_text = safe_repo_file(consumer["fixture_test"], package_root).read_text(
            "utf-8"
        )
        assert "der-asn1-v1" in test_text
        assert "cases.json" in test_text
        capabilities = load_json(
            safe_repo_file(consumer["capability_manifest"], package_root)
        )
        validator.validate(capabilities)
        assert capabilities["version"] == 1
        expected_package = package_root.removeprefix("code/packages/").replace(
            "der_asn1", "der-asn1"
        )
        assert capabilities["package"] == expected_package
        assert capabilities["capabilities"] == []


def check_closed_and_not_cross_wired() -> None:
    schema = load_json(FIXTURE_ROOT / "consumers.schema.json")
    document = deepcopy(consumers_document())
    document["consumers"].append(deepcopy(document["consumers"][0]))
    assert list(Draft202012Validator(schema).iter_errors(document))
    document = consumers_document()
    document["consumers"][0]["package_root"] = "code/packages/rust/der-asn1"
    with unittest.TestCase().assertRaises(AssertionError):
        safe_repo_file(
            document["consumers"][0]["api_source"],
            document["consumers"][0]["package_root"],
        )


class PortableCoverageTests(unittest.TestCase):
    def test_registry_schema_and_denominator(self) -> None:
        check_registry_schema_and_denominator()

    def test_paths_builds_and_surface(self) -> None:
        check_paths_builds_and_surface()

    def test_fixture_and_capabilities(self) -> None:
        check_fixture_and_capabilities()

    def test_closed_and_not_cross_wired(self) -> None:
        check_closed_and_not_cross_wired()


if __name__ == "__main__":
    unittest.main()
