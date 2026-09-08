"""Contract tests for the independent OCAML06 capability gate."""

from __future__ import annotations

import importlib.util
import json
import os
import re
import tempfile
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


def load_coverage_gate():
    path = PACKAGE_ROOT / "test/check_coverage.py"
    spec = importlib.util.spec_from_file_location("ocaml_coverage_gate", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


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
        packaged = json.loads(
            (
                PACKAGE_ROOT / "test/fixtures/ocaml-capability-analyzer-v1/cases.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(packaged, document)

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

    def test_numeric_coverage_gate_is_fail_closed(self) -> None:
        gate = load_coverage_gate()
        summary = (
            "Coverage: 95/100 (95.00%)\n"
            "File 'src/coding_adventures_capability_analyzer.ml': "
            "95/100 (95.00%)\n"
        )
        self.assertEqual(
            gate.source_percentage(
                summary, "src/coding_adventures_capability_analyzer.ml"
            ),
            95.0,
        )
        with self.assertRaises(ValueError):
            gate.source_percentage(summary, "src/missing.ml")
        self.assertEqual(
            gate.require_minimum(
                summary, "src/coding_adventures_capability_analyzer.ml", 95.0
            ),
            95.0,
        )
        with self.assertRaises(ValueError):
            gate.require_minimum(
                summary, "src/coding_adventures_capability_analyzer.ml", 95.01
            )

    def test_numeric_coverage_gate_accepts_bisect_per_file_output(self) -> None:
        gate = load_coverage_gate()
        summary = (
            " 97.25 %   389/400   src/coding_adventures_capability_analyzer.ml\n"
            "100.00 %     8/8     bin/main.ml\n"
            " 97.31 %   397/408   Project coverage\n"
        )
        self.assertEqual(
            gate.source_percentage(
                summary, "src/coding_adventures_capability_analyzer.ml"
            ),
            97.25,
        )
        self.assertEqual(gate.source_percentage(summary, "bin/main.ml"), 100.0)

    def test_coverage_command_aggregates_every_process_file(self) -> None:
        gate = load_coverage_gate()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "bisect-a.coverage").touch()
            (root / "bisect-b.coverage").touch()
            previous = Path.cwd()
            try:
                os.chdir(root)
                command = gate.coverage_command(
                    "bisect*.coverage", ["src/library.ml", "bin/main.ml"]
                )
            finally:
                os.chdir(previous)
        self.assertEqual(command.count("--expect"), 2)
        self.assertTrue(command[-2].endswith("bisect-a.coverage"))
        self.assertTrue(command[-1].endswith("bisect-b.coverage"))

    def test_cobertura_diagnostics_report_uncovered_source_lines(self) -> None:
        gate = load_coverage_gate()
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "coverage.xml"
            report.write_text(
                '<coverage><packages><package><classes>'
                '<class filename="src/library.ml"><lines>'
                '<line number="10" hits="1"/><line number="11" hits="0"/>'
                '<line number="19" hits="0"/>'
                '</lines></class></classes></package></packages></coverage>',
                encoding="utf-8",
            )
            self.assertEqual(gate.uncovered_lines(report, "src/library.ml"), [11, 19])

    def test_cobertura_command_aggregates_every_process_file(self) -> None:
        gate = load_coverage_gate()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "bisect-a.coverage").touch()
            (root / "bisect-b.coverage").touch()
            previous = Path.cwd()
            try:
                os.chdir(root)
                command = gate.cobertura_command(
                    "bisect*.coverage", ["src/library.ml"], root / "coverage.xml"
                )
            finally:
                os.chdir(previous)
        self.assertIn("cobertura", command)
        self.assertTrue(command[-2].endswith("bisect-a.coverage"))
        self.assertTrue(command[-1].endswith("bisect-b.coverage"))

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

        job_gate = registry["gates"]["ocaml-capability-gate"]
        self.assertEqual(job_gate["scope"], "job")
        self.assertIn("dune-workspace", job_gate["paths"])
        self.assertIn("**/dune-workspace", job_gate["paths"])
        self.assertIn("code/packages/ocaml/**", job_gate["paths"])
        self.assertIn("code/programs/ocaml/**", job_gate["paths"])

    def test_workflow_has_an_independent_fail_closed_job(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        required_fragments = (
            "ocaml-capability-gate:",
            "name: OCaml capability gate",
            "run_ocaml_capability_gate: ${{ steps.detect.outputs.run_ocaml_capability_gate }}",
            "needs.detect.outputs.run_ocaml_capability_gate == 'true'",
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
            "ocaml/setup-ocaml@15d660006c1d3110d77c34b7faa3bddefe8b82f0",
            "ocaml-base-compiler.5.2.1",
            "opam-repository.git#ba8cc66eb9e5baae7ebc88cf77f4c488d63d87ff",
            "opam install . --deps-only --with-test --with-dev-setup",
            "dune runtest --force --instrument-with bisect_ppx",
            '--coverage-glob "bisect*.coverage"',
            "--source bin/main.ml",
            "test/check_coverage.py",
            "--minimum 95",
            "opam exec -- dune build bin/main.exe",
            "find code/packages/ocaml code/programs/ocaml",
            'find . -name dune-workspace -print -quit',
            'test "${#roots[@]}" -gt 0',
            '"$analyzer" --dir "$(dirname "$project")"',
            "needs: [detect, contracts, ocaml-capability-gate,",
            "OCAML_CAPABILITY_RESULT: ${{ needs['ocaml-capability-gate'].result }}",
            '"$OCAML_CAPABILITY_RESULT"',
        )
        for fragment in required_fragments:
            self.assertIn(fragment, workflow)


if __name__ == "__main__":
    unittest.main()
