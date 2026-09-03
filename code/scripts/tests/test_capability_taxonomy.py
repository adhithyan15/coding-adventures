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

    def test_agent_manifest_schema_versions_are_explicitly_evolved(self) -> None:
        validator = jsonschema.Draft202012Validator(self.manifest_schemas["agent"])
        legacy = self.manifest("agent", "net", "connect")
        validator.validate(legacy)

        current = dict(legacy)
        current["version"] = 2
        current["channels"] = {
            "reads": {},
            "writes": {"agent-output": 1},
        }
        validator.validate(current)

        missing = dict(current)
        missing["channels"] = {
            "reads": {},
            "writes": ["agent-output"],
        }
        self.assertTrue(list(validator.iter_errors(missing)))

        legacy_with_current_binding = dict(legacy)
        legacy_with_current_binding["channels"] = current["channels"]
        self.assertTrue(list(validator.iter_errors(legacy_with_current_binding)))

        # Schema v3 adds allowed_tools, and the published schema must mirror the
        # Rust gate in chief-of-staff-agent-manifest. The prose, README and
        # CHANGELOG were once evolved for v3 while this file was not, which left
        # a v3 manifest failing its own published contract -- and this schema is
        # what a reviewer, an editor, or a non-Rust consumer reads to learn what
        # a signed manifest authorizes.
        v3 = dict(current)
        v3["version"] = 3
        v3["allowed_tools"] = ["artifact.write", "context.append_entry"]
        validator.validate(v3)

        v3_no_tools = dict(v3)
        v3_no_tools["allowed_tools"] = []
        validator.validate(v3_no_tools)

        # Required at v3: "calls no tools" is declared, never defaulted into.
        v3_missing_tools = {k: v for k, v in v3.items() if k != "allowed_tools"}
        self.assertTrue(list(validator.iter_errors(v3_missing_tools)))

        # Earlier versions may not carry a tool surface, or a consumer trusting
        # `version` would be told something false about the signed bytes.
        for older in (1, 2):
            smuggled = dict(v3)
            smuggled["version"] = older
            if older == 1:
                smuggled["channels"] = legacy["channels"]
            self.assertTrue(list(validator.iter_errors(smuggled)))

        # A bare namespace names no tool and would invite prefix matching.
        for bad in ("artifact", "Artifact.create", "artifact..create", ".create", "artifact."):
            malformed = dict(v3)
            malformed["allowed_tools"] = [bad]
            self.assertTrue(
                list(validator.iter_errors(malformed)), f"must reject {bad!r}"
            )

        duplicated = dict(v3)
        duplicated["allowed_tools"] = ["artifact.write", "artifact.write"]
        self.assertTrue(list(validator.iter_errors(duplicated)))

        # Schema v4 adds tool_capabilities: D18D capability SCOPES, matched
        # against a ToolDefinition's required_capabilities. Colon-delimited
        # (smart_home:read) -- a different separator and a different namespace
        # from both tool identifiers and the OS capability triples.
        v4 = dict(v3)
        v4["version"] = 4
        v4["tool_capabilities"] = ["smart_home:read", "smart_home:write"]
        validator.validate(v4)

        v4_empty = dict(v4)
        v4_empty["tool_capabilities"] = []
        validator.validate(v4_empty)

        v4_missing = {k: v for k, v in v4.items() if k != "tool_capabilities"}
        self.assertTrue(list(validator.iter_errors(v4_missing)))

        # v3 carried allowed_tools but granted no tool capabilities.
        v3_smuggled = dict(v4)
        v3_smuggled["version"] = 3
        self.assertTrue(list(validator.iter_errors(v3_smuggled)))

        for bad in ("smart_home::read", ":read", "smart_home:", "smart home:read", ""):
            malformed = dict(v4)
            malformed["tool_capabilities"] = [bad]
            self.assertTrue(
                list(validator.iter_errors(malformed)), f"must reject {bad!r}"
            )

        v4_dup = dict(v4)
        v4_dup["tool_capabilities"] = ["smart_home:read", "smart_home:read"]
        self.assertTrue(list(validator.iter_errors(v4_dup)))

        invalid = dict(current)
        invalid["channels"] = {
            "reads": {},
            "writes": {"agent-output": 0},
        }
        self.assertTrue(list(validator.iter_errors(invalid)))

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
