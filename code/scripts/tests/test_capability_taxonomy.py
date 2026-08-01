"""Conformance tests for the closed Spec 13 capability taxonomy."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

import jsonschema

REPO_ROOT = Path(__file__).resolve().parents[3]
SCHEMA_ROOT = REPO_ROOT / "code/specs/schemas"
FIXTURE_PATH = (
    REPO_ROOT / "code/specs/fixtures/capability-security-v1/taxonomy.json"
)


class CapabilityTaxonomyTest(unittest.TestCase):
    """Keep both manifests on the same exhaustive category/action contract."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.taxonomy = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
        cls.taxonomy_schema = cls.load_schema("capability_taxonomy.schema.json")
        cls.manifest_schemas = {
            "required": cls.load_schema("required_capabilities.schema.json"),
            "agent": cls.load_schema("agent_manifest.schema.json"),
        }

    @staticmethod
    def load_schema(name: str) -> dict[str, Any]:
        schema = json.loads((SCHEMA_ROOT / name).read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(schema)
        return schema

    def test_taxonomy_fixture_is_schema_valid_and_closed(self) -> None:
        jsonschema.Draft202012Validator(self.taxonomy_schema).validate(self.taxonomy)
        categories = self.taxonomy["categories"]
        actions = self.taxonomy["all_actions"]
        valid_count = sum(len(allowed) for allowed in categories.values())
        invalid_count = len(categories) * len(actions) - valid_count
        self.assertEqual(19, valid_count)
        self.assertEqual(93, invalid_count)
        self.assertEqual(self.taxonomy["expected_valid_pair_count"], valid_count)
        self.assertEqual(
            self.taxonomy["expected_invalid_cross_pair_count"], invalid_count
        )

    def test_both_manifest_schemas_accept_all_19_valid_pairs(self) -> None:
        for schema_name, schema in self.manifest_schemas.items():
            validator = jsonschema.Draft202012Validator(schema)
            for category, actions in self.taxonomy["categories"].items():
                for action in actions:
                    with self.subTest(
                        schema=schema_name, category=category, action=action
                    ):
                        errors = list(
                            validator.iter_errors(
                                self.manifest(schema_name, category, action)
                            )
                        )
                        self.assertEqual([], [error.message for error in errors])

    def test_both_manifest_schemas_reject_all_93_invalid_cross_pairs(self) -> None:
        for schema_name, schema in self.manifest_schemas.items():
            validator = jsonschema.Draft202012Validator(schema)
            for category, allowed in self.taxonomy["categories"].items():
                for action in self.taxonomy["all_actions"]:
                    if action in allowed:
                        continue
                    with self.subTest(
                        schema=schema_name, category=category, action=action
                    ):
                        errors = list(
                            validator.iter_errors(
                                self.manifest(schema_name, category, action)
                            )
                        )
                        self.assertTrue(errors, "invalid cross-pair must fail closed")

    def test_both_manifest_schemas_reject_unknown_vocabulary(self) -> None:
        for schema_name, schema in self.manifest_schemas.items():
            validator = jsonschema.Draft202012Validator(schema)
            for category, action in (("filesystem", "read"), ("fs", "destroy")):
                with self.subTest(
                    schema=schema_name, category=category, action=action
                ):
                    errors = list(
                        validator.iter_errors(
                            self.manifest(schema_name, category, action)
                        )
                    )
                    self.assertTrue(errors, "unknown vocabulary must fail closed")

    def test_repository_required_capabilities_use_only_valid_pairs(self) -> None:
        known_categories = set(self.taxonomy["categories"])
        known_actions = set(self.taxonomy["all_actions"])
        allowed = {
            (category, action)
            for category, actions in self.taxonomy["categories"].items()
            for action in actions
        }
        manifest_paths = sorted(REPO_ROOT.glob("code/**/required_capabilities.json"))
        self.assertGreater(len(manifest_paths), 300)
        for manifest_path in manifest_paths:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            # Some legacy build-dependency manifests use the same filename but
            # contain a JSON array, or use a legacy string-list capability
            # shape. They are not structured Spec 13 capability manifests.
            if not isinstance(manifest, dict):
                continue
            # A separate legacy class contains package metadata objects with
            # no capability declaration at all. Do not silently treat those
            # objects as schema-v1 empty manifests.
            if "capabilities" not in manifest:
                continue
            capabilities = manifest["capabilities"]
            if not isinstance(capabilities, list) or any(
                not isinstance(capability, dict) for capability in capabilities
            ):
                continue
            for index, capability in enumerate(capabilities):
                pair = (capability.get("category"), capability.get("action"))
                # Legacy vocabulary is separately inventoried for migration.
                # This regression gate proves there are no invalid cross-pairs
                # among manifests already using the current vocabulary.
                if pair[0] not in known_categories or pair[1] not in known_actions:
                    continue
                with self.subTest(
                    manifest=manifest_path.relative_to(REPO_ROOT),
                    capability=index,
                    pair=pair,
                ):
                    self.assertIn(pair, allowed)

    def test_camera_media_authority_free_manifest_is_schema_valid(self) -> None:
        manifest_path = (
            REPO_ROOT
            / "code/packages/rust/smart-home-camera-media/required_capabilities.json"
        )
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        validator = jsonschema.Draft202012Validator(
            self.manifest_schemas["required"]
        )
        validator.validate(manifest)
        self.assertEqual("rust/smart-home-camera-media", manifest["package"])
        self.assertEqual([], manifest["capabilities"])
        self.assertTrue(manifest["justification"].strip())

    @staticmethod
    def manifest(schema_name: str, category: str, action: str) -> dict[str, Any]:
        capability = {
            "category": category,
            "action": action,
            "target": "*",
            "justification": "Exhaustive taxonomy conformance fixture.",
        }
        if schema_name == "required":
            return {
                "version": 1,
                "package": "go/taxonomy-test",
                "capabilities": [capability],
                "justification": "Exercises one category and action pair.",
            }
        return {
            "version": 1,
            "agent": "taxonomy-test",
            "description": "Exercises one capability taxonomy pair.",
            "privilege_tier": 0,
            "channels": {"reads": [], "writes": []},
            "capabilities": [capability],
            "justification": "Exercises one category and action pair.",
        }


if __name__ == "__main__":
    unittest.main()
