"""Prove that every established lane consumes generic X.509 Extension v1."""

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
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/x509-extension-v1"
sys.path.insert(0, str(REPO_ROOT / "code/scripts"))
IMPLEMENTATION_LANGUAGES = importlib.import_module(
    "package_parity_report"
).IMPLEMENTATION_LANGUAGES

PACKAGE_SPELLINGS = {
    "csharp": "x509-extension",
    "dart": "x509-extension",
    "elixir": "x509_extension",
    "fsharp": "x509-extension",
    "go": "x509-extension",
    "haskell": "x509-extension",
    "java": "x509-extension",
    "kotlin": "x509-extension",
    "lua": "x509_extension",
    "perl": "x509-extension",
    "python": "x509-extension",
    "ruby": "x509_extension",
    "rust": "x509-extension",
    "swift": "x509-extension",
    "typescript": "x509-extension",
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
    assert lanes == list(PACKAGE_SPELLINGS)
    assert document["established_lane_count"] == len(lanes) == 15


def check_paths_surface_and_metadata() -> None:
    for consumer in consumers_document()["consumers"]:
        language = consumer["language"]
        package_root = (
            f"code/packages/{language}/{PACKAGE_SPELLINGS[language]}"
        )
        assert consumer["package_root"] == package_root
        paths = [
            consumer["api_source"],
            consumer["fixture_test"],
            *consumer["build_files"],
            consumer["capability_manifest"],
            *consumer["dependency_metadata"],
            consumer["readme"],
            consumer["changelog"],
        ]
        for path in paths:
            safe_repo_file(path, package_root)
        source_text = safe_repo_file(
            consumer["api_source"], package_root
        ).read_text("utf-8")
        for token in consumer["surface"].values():
            assert token in source_text, f"{language} production source lost {token}"


def check_fixture_tests_and_capabilities() -> None:
    document = consumers_document()
    fixture = load_json(REPO_ROOT / document["fixture"])
    capability_schema = load_json(
        REPO_ROOT / "code/specs/schemas/required_capabilities.schema.json"
    )
    validator = Draft202012Validator(capability_schema)
    assert len(fixture["cases"]) == document["case_count"] == 48
    assert len(fixture["error_ids"]) == document["error_id_count"] == 8
    for consumer in document["consumers"]:
        package_root = consumer["package_root"]
        test_text = safe_repo_file(
            consumer["fixture_test"], package_root
        ).read_text("utf-8")
        assert "x509-extension-v1" in test_text
        assert "cases.json" in test_text
        capabilities = load_json(
            safe_repo_file(consumer["capability_manifest"], package_root)
        )
        validator.validate(capabilities)
        assert capabilities["version"] == 1
        assert capabilities["capabilities"] == []
        expected_package = package_root.removeprefix("code/packages/").replace(
            "x509_extension", "x509-extension"
        )
        assert capabilities["package"] == expected_package


def check_closed_and_not_cross_wired() -> None:
    schema = load_json(FIXTURE_ROOT / "consumers.schema.json")
    document = deepcopy(consumers_document())
    document["consumers"].append(deepcopy(document["consumers"][0]))
    assert list(Draft202012Validator(schema).iter_errors(document))
    document = deepcopy(consumers_document())
    document["consumers"][0]["package_root"] = "code/packages/rust/x509-extension"
    with unittest.TestCase().assertRaises(AssertionError):
        safe_repo_file(
            document["consumers"][0]["api_source"],
            document["consumers"][0]["package_root"],
        )


class PortableCoverageTests(unittest.TestCase):
    def test_registry_schema_and_denominator(self) -> None:
        check_registry_schema_and_denominator()

    def test_paths_surface_and_metadata(self) -> None:
        check_paths_surface_and_metadata()

    def test_fixture_tests_and_capabilities(self) -> None:
        check_fixture_tests_and_capabilities()

    def test_closed_and_not_cross_wired(self) -> None:
        check_closed_and_not_cross_wired()


if __name__ == "__main__":
    unittest.main()
