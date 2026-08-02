from __future__ import annotations

import importlib
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

manifest_module = importlib.import_module("adj_stdlib_manifest")


def evidence(path: str, *, sourced: bool = True, pinned: bool = False) -> dict:
    return {
        "path": path,
        "test_reference": True,
        "source_envelope": sourced,
        "pinned_quote": pinned,
    }


def valid_manifest(path: str) -> dict:
    return {
        "schema_version": 1,
        "manifest_id": "test.curriculum.v1",
        "coverage_roots": [
            {
                "id": "test.root",
                "title": "Test root",
                "version": "v1",
                "locator": "https://example.test/root",
                "status": "declared",
                "retrieved_at": None,
                "cas_hash": None,
            }
        ],
        "objectives": [
            {
                "id": "test.objective",
                "title": "Test objective",
                "band": "K-2",
                "domain": "mathematics",
                "competency": "compute",
                "coverage_roots": ["test.root"],
                "standards": [],
                "prerequisites": [],
                "libraries": [path],
                "modalities": ["text"],
                "source_cas_hashes": [],
                "benchmark_paths": [],
                "status": {
                    "implementation": "present",
                    "provenance": "source_labeled",
                    "tests": "present",
                    "benchmark": "missing",
                    "crosswalk": "unmapped",
                },
            }
        ],
    }


class AdjStdlibManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.library = "code/specs/data/adj-formula-stdlib/arithmetic/add.adj"
        self.evidence = {self.library: evidence(self.library)}

    def validate(self, value: dict) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            return manifest_module.validate_manifest(
                Path(directory), value, self.evidence, provenance_bundles={}
            )

    def test_accepts_a_source_labeled_seed_objective(self) -> None:
        self.assertEqual(self.validate(valid_manifest(self.library)), [])

    def test_rejects_duplicate_ids_and_unknown_prerequisites(self) -> None:
        value = valid_manifest(self.library)
        duplicate = deepcopy(value["objectives"][0])
        duplicate["prerequisites"] = ["missing.objective"]
        value["objectives"].append(duplicate)

        errors = self.validate(value)

        self.assertIn("duplicate objective id: test.objective", errors)
        self.assertIn(
            "test.objective references unknown prerequisite: missing.objective", errors
        )

    def test_rejects_prerequisite_cycles(self) -> None:
        value = valid_manifest(self.library)
        value["objectives"][0]["prerequisites"] = ["test.second"]
        second = deepcopy(value["objectives"][0])
        second["id"] = "test.second"
        second["prerequisites"] = ["test.objective"]
        value["objectives"].append(second)

        errors = self.validate(value)

        self.assertTrue(
            any(error.startswith("prerequisite cycle:") for error in errors)
        )

    def test_rejects_unknown_or_unsafe_library_paths(self) -> None:
        value = valid_manifest("../outside.adj")

        errors = self.validate(value)

        self.assertTrue(any("unsafe library path" in error for error in errors))

    def test_rejects_provenance_claims_stronger_than_evidence(self) -> None:
        value = valid_manifest(self.library)
        value["objectives"][0]["status"]["provenance"] = "fully_verified"

        errors = self.validate(value)

        self.assertTrue(any("claims byte pins" in error for error in errors))
        self.assertTrue(any("no source_cas_hashes" in error for error in errors))
        self.assertTrue(any("no provenance_bundle_hashes" in error for error in errors))

    def test_provenance_bundle_must_be_verified_and_match_a_library(self) -> None:
        value = valid_manifest(self.library)
        bundle_hash = "a" * 64
        value["objectives"][0]["provenance_bundle_hashes"] = [bundle_hash]

        missing = manifest_module.validate_manifest(
            Path("."), value, self.evidence, provenance_bundles={}
        )
        mismatched = manifest_module.validate_manifest(
            Path("."),
            value,
            self.evidence,
            provenance_bundles={bundle_hash: {"library": "code/other.adj"}},
        )

        self.assertTrue(
            any("unverified provenance bundle" in error for error in missing)
        )
        self.assertTrue(
            any("belongs to unlisted library" in error for error in mismatched)
        )

    def test_source_hashes_must_equal_resolved_bundle_sources(self) -> None:
        value = valid_manifest(self.library)
        bundle_hash = "a" * 64
        value["objectives"][0]["provenance_bundle_hashes"] = [bundle_hash]
        value["objectives"][0]["source_cas_hashes"] = ["b" * 64]
        bundles = {
            bundle_hash: {
                "library": self.library,
                "sources": [{"raw_source_sha256": "c" * 64}],
            }
        }

        errors = manifest_module.validate_manifest(
            Path("."), value, self.evidence, provenance_bundles=bundles
        )

        self.assertTrue(
            any("disagree with resolved bundle sources" in e for e in errors)
        )

    def test_rejects_mapped_and_held_out_claims_without_artifacts(self) -> None:
        value = valid_manifest(self.library)
        status = value["objectives"][0]["status"]
        status["crosswalk"] = "mapped"
        status["benchmark"] = "held_out"

        errors = self.validate(value)

        self.assertTrue(any("names no standards" in error for error in errors))
        self.assertTrue(any("names no benchmark_paths" in error for error in errors))

    def test_json_schema_validation_reports_instance_errors(self) -> None:
        schema = {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["schema_version"],
            "properties": {"schema_version": {"const": 1}},
        }

        errors = manifest_module.validate_json_schema(schema, {"schema_version": 2})

        self.assertTrue(any("1 was expected" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
