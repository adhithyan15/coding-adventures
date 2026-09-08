"""Contract tests for the independent OCAML06 capability gate."""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

import jsonschema

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/ocaml-capability-analyzer-v1"
PACKAGE_ROOT = REPO_ROOT / "code/packages/ocaml/ca-capability-analyzer"
WORKFLOW_PATH = REPO_ROOT / ".github/workflows/ci.yml"
REGISTRY_PATH = REPO_ROOT / "code/specs/data/ci-gates.json"
MANIFEST_SCHEMA_PATH = (
    REPO_ROOT / "code/specs/schemas/required_capabilities.schema.json"
)


class OcamlCapabilityGateTest(unittest.TestCase):
    def test_behavior_fixture_is_schema_valid(self) -> None:
        schema = json.loads(
            (FIXTURE_ROOT / "cases.schema.json").read_text(encoding="utf-8")
        )
        document = json.loads((FIXTURE_ROOT / "cases.json").read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator(schema).validate(document)
        ids = [case["id"] for case in document["cases"]]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertGreaterEqual(len(ids), 15)

    def test_manifest_schema_allows_ocaml_exceptions(self) -> None:
        schema = json.loads(MANIFEST_SCHEMA_PATH.read_text(encoding="utf-8"))
        languages = schema["$defs"]["banned_construct_exception"]["properties"][
            "language"
        ]["enum"]
        self.assertIn("ocaml", languages)
        manifest = json.loads(
            (PACKAGE_ROOT / "required_capabilities.json").read_text(encoding="utf-8")
        )
        jsonschema.Draft202012Validator(schema).validate(manifest)

    def test_package_pins_the_reviewed_dependencies(self) -> None:
        opam = (PACKAGE_ROOT / "coding-adventures-capability-analyzer.opam").read_text(
            encoding="utf-8"
        )
        for dependency, version in {
            "ocaml": "5.2.1",
            "dune": "3.17.2",
            "yojson": "3.0.0",
            "alcotest": "1.9.0",
            "bisect_ppx": "2.8.3",
            "ocamlformat": "0.27.0",
        }.items():
            self.assertRegex(
                opam, rf'"{re.escape(dependency)}"[^\n]*= "{re.escape(version)}"'
            )

    def test_ci_registry_routes_all_ocaml_analyzer_inputs(self) -> None:
        registry = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
        gate = registry["gates"]["contracts-capability-cage"]
        self.assertIn("ocaml/ca-capability-analyzer", gate["packages"])
        for path in (
            "code/specs/fixtures/ocaml-capability-analyzer-v1/**",
            "code/specs/OCAML06-capability-analyzer.md",
            "code/scripts/tests/test_ocaml_capability_gate.py",
            "code/packages/ocaml/ca-capability-analyzer/**",
        ):
            self.assertIn(path, gate["paths"])

    def test_workflow_has_an_independent_fail_closed_job(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        required_fragments = (
            "ocaml-capability-gate:",
            "name: OCaml capability gate",
            "needs.detect.outputs.run_contracts_capability_cage == 'true'",
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
            "ocaml/setup-ocaml@15d660006c1d3110d77c34b7faa3bddefe8b82f0",
            "ocaml-base-compiler.5.2.1",
            "opam-repository.git#ba8cc66eb9e5baae7ebc88cf77f4c488d63d87ff",
            "opam install . --deps-only --with-test --with-dev-setup",
            "opam exec -- dune build bin/main.exe",
            "find code/packages/ocaml code/programs/ocaml",
            'test "${#roots[@]}" -gt 0',
            '"$analyzer" --dir "$(dirname "$project")"',
        )
        for fragment in required_fragments:
            self.assertIn(fragment, workflow)


if __name__ == "__main__":
    unittest.main()
