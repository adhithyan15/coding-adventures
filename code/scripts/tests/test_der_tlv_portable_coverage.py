"""Prove that every established lane consumes the closed DER-TLV profile."""

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
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/der-tlv-v1"
sys.path.insert(0, str(REPO_ROOT / "code/scripts"))
IMPLEMENTATION_LANGUAGES = importlib.import_module(
    "package_parity_report"
).IMPLEMENTATION_LANGUAGES

# language: (package spelling, source, test, builds, five production tokens)
EXPECTED: dict[str, tuple[str, str, str, list[str], list[str]]] = {
    "csharp": (
        "der-tlv",
        "DerTlv.cs",
        "tests/CodingAdventures.DerTlv.CSharp.Tests/PortableConformanceTests.cs",
        ["BUILD", "BUILD_windows"],
        ["DecodeOne", "DecodeExact", "Cursor", "Limits", "Error"],
    ),
    "dart": (
        "der-tlv",
        "lib/src/der_tlv.dart",
        "test/der_tlv_test.dart",
        ["BUILD", "BUILD_windows"],
        ["decodeOne", "decodeExact", "DerCursor", "DerLimits", "DerErrorKind"],
    ),
    "elixir": (
        "der_tlv",
        "lib/coding_adventures/der_tlv.ex",
        "test/der_tlv_test.exs",
        ["BUILD", "BUILD_windows"],
        ["decode_one", "decode_exact", "Cursor", "default_limits", "Error"],
    ),
    "fsharp": (
        "der-tlv",
        "DerTlv.fs",
        "tests/CodingAdventures.DerTlv.FSharp.Tests/PortableConformanceTests.fs",
        ["BUILD", "BUILD_windows"],
        ["decodeOne", "decodeExact", "DerCursor", "DerLimits", "DerError"],
    ),
    "go": (
        "der-tlv",
        "der_tlv.go",
        "der_tlv_test.go",
        ["BUILD"],
        ["DecodeOne", "DecodeExact", "Cursor", "Limits", "ErrorKind"],
    ),
    "haskell": (
        "der-tlv",
        "src/CodingAdventures/DerTlv.hs",
        "test/PortableConformanceSpec.hs",
        ["BUILD", "BUILD_windows"],
        ["decodeOne", "decodeExact", "DerCursor", "DerLimits", "DerErrorKind"],
    ),
    "java": (
        "der-tlv",
        "src/main/java/com/codingadventures/dertlv/DerTlv.java",
        "src/test/java/com/codingadventures/dertlv/DerTlvTest.java",
        ["BUILD", "BUILD_windows"],
        ["decodeOne", "decodeExact", "Cursor", "Limits", "Error"],
    ),
    "kotlin": (
        "der-tlv",
        "src/main/kotlin/com/codingadventures/dertlv/DerTlv.kt",
        "src/test/kotlin/com/codingadventures/dertlv/DerTlvTest.kt",
        ["BUILD", "BUILD_windows"],
        ["decodeOne", "decodeExact", "Cursor", "Limits", "DerError"],
    ),
    "lua": (
        "der_tlv",
        "src/coding_adventures/der_tlv/init.lua",
        "tests/test_der_tlv.lua",
        ["BUILD", "BUILD_windows"],
        ["decode_one", "decode_exact", "Cursor", "default_limits", "Error"],
    ),
    "perl": (
        "der-tlv",
        "lib/CodingAdventures/DerTlv.pm",
        "t/portable-conformance.t",
        ["BUILD", "BUILD_windows"],
        [
            "decode_one",
            "decode_exact",
            "CodingAdventures::DerTlv::Cursor",
            "default_limits",
            "CodingAdventures::DerTlv::Error",
        ],
    ),
    "python": (
        "der-tlv",
        "src/der_tlv/__init__.py",
        "tests/test_der_tlv.py",
        ["BUILD", "BUILD_windows"],
        ["decode_one", "decode_exact", "DerCursor", "DerLimits", "DerErrorKind"],
    ),
    "ruby": (
        "der_tlv",
        "lib/coding_adventures_der_tlv.rb",
        "test/test_der_tlv.rb",
        ["BUILD", "BUILD_windows"],
        ["decode_one", "decode_exact", "Cursor", "DEFAULT_LIMITS", "Error"],
    ),
    "rust": (
        "der-tlv",
        "src/lib.rs",
        "tests/portable_conformance.rs",
        ["BUILD"],
        ["decode_one", "decode_exact", "DerCursor", "DerLimits", "DerErrorKind"],
    ),
    "swift": (
        "der-tlv",
        "Sources/DerTlv/DerTlv.swift",
        "Tests/DerTlvTests/DerTlvTests.swift",
        ["BUILD", "BUILD_windows"],
        ["decodeOne", "decodeExact", "Cursor", "Limits", "FramingError"],
    ),
    "typescript": (
        "der-tlv",
        "src/index.ts",
        "tests/der-tlv.test.ts",
        ["BUILD", "BUILD_windows"],
        ["decodeOne", "decodeExact", "DerCursor", "DerLimits", "DerErrorKind"],
    ),
}


def load_json(path: Path) -> dict[str, Any]:
    """Load repository JSON without accepting duplicate keys."""

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
    surface_keys = ["decode_one", "decode_exact", "cursor", "limits", "error_taxonomy"]
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
    assert len(fixture["cases"]) == document["case_count"] == 54
    assert len(fixture["error_ids"]) == document["error_id_count"] == 17
    for consumer in document["consumers"]:
        package_root = consumer["package_root"]
        test_text = safe_repo_file(consumer["fixture_test"], package_root).read_text(
            "utf-8"
        )
        assert "der-tlv-v1" in test_text, consumer["language"]
        capabilities = load_json(
            safe_repo_file(consumer["capability_manifest"], package_root)
        )
        assert capabilities["capabilities"] == [], consumer["language"]


def check_closed_and_not_cross_wired() -> None:
    schema = load_json(FIXTURE_ROOT / "consumers.schema.json")
    document = deepcopy(consumers_document())
    document["consumers"].append(deepcopy(document["consumers"][0]))
    assert list(Draft202012Validator(schema).iter_errors(document))
    document = consumers_document()
    document["consumers"][0]["package_root"] = "code/packages/rust/der-tlv"
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
