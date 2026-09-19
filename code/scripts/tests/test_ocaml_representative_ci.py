"""Contract tests for OCAML07 representative-package execution CI."""

from __future__ import annotations

import copy
import io
import sys
import tarfile
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT_ROOT = REPO_ROOT / "code/scripts"
sys.path.insert(0, str(SCRIPT_ROOT))

import ocaml_representative_ci as representative


class ManifestAndRepositoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = representative.load_manifest(REPO_ROOT)
        cls.workflow = (REPO_ROOT / representative.WORKFLOW_RELATIVE_PATH).read_text(
            encoding="utf-8"
        )

    def test_checked_repository_passes(self) -> None:
        validated = representative.validate_repository(REPO_ROOT)
        self.assertEqual(1, validated["schema_version"])

    def test_manifest_is_closed(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["unexpected"] = True
        with self.assertRaisesRegex(representative.ContractError, "top-level keys"):
            representative.validate_manifest_shape(document)

    def test_package_order_and_dependencies_are_exact(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["packages"] = list(reversed(document["packages"]))
        with self.assertRaisesRegex(
            representative.ContractError, "package (order|logic-gates)"
        ):
            representative.validate_manifest_shape(document)

        document = copy.deepcopy(self.manifest)
        document["packages"][3]["local_dependencies"] = ["directed-graph"]
        with self.assertRaisesRegex(representative.ContractError, "dependencies"):
            representative.validate_manifest_shape(document)

    def test_target_and_toolchain_contracts_are_exact(self) -> None:
        for mutation, message in (
            (("targets", "linux-x64", "runner"), "targets"),
            (("versions", "odoc"), "versions"),
            (("actions", "checkout"), "actions"),
        ):
            document = copy.deepcopy(self.manifest)
            parent = document
            for key in mutation[:-1]:
                parent = parent[key]
            parent[mutation[-1]] = "drift"
            with self.subTest(mutation=mutation), self.assertRaisesRegex(
                representative.ContractError, message
            ):
                representative.validate_manifest_shape(document)

    def test_workflow_rejects_permission_action_and_masking_drift(self) -> None:
        mutations = (
            self.workflow.replace("contents: read", "contents: write", 1),
            self.workflow.replace("actions/checkout@", "attacker/checkout@", 1),
            self.workflow.replace("fail-fast: false", "fail-fast: true", 1),
            self.workflow.replace("set -euo pipefail", "set -euo pipefail\n          true || true", 1),
            self.workflow + "\n# ${{ secrets.REPOSITORY_TOKEN }}\n",
        )
        for index, workflow in enumerate(mutations):
            with self.subTest(case=index), self.assertRaises(
                representative.ContractError
            ):
                    representative.validate_workflow_policy_text(workflow)


class CoverageTests(unittest.TestCase):
    def test_numeric_coverage_requires_every_source_and_minimum(self) -> None:
        summary = (
            "Coverage summary\n"
            " 97.35 %   367/377   src/coding_adventures_logic_gates.ml\n"
        )
        result = representative.validate_coverage_summary(
            summary, ["src/coding_adventures_logic_gates.ml"], 9500
        )
        self.assertEqual(9735, result["src/coding_adventures_logic_gates.ml"])

        with self.assertRaisesRegex(representative.ContractError, "below"):
            representative.validate_coverage_summary(
                summary.replace("97.35", "94.99"),
                ["src/coding_adventures_logic_gates.ml"],
                9500,
            )

        with self.assertRaisesRegex(representative.ContractError, "missing"):
            representative.validate_coverage_summary(summary, ["src/missing.ml"], 9500)

    def test_coverage_rejects_duplicates_and_zero_instrumented_lines(self) -> None:
        duplicate = (
            " 97.00 % 97/100 src/a.ml\n"
            " 97.00 % 97/100 src/a.ml\n"
        )
        with self.assertRaisesRegex(representative.ContractError, "duplicate"):
            representative.validate_coverage_summary(duplicate, ["src/a.ml"], 9500)
        with self.assertRaisesRegex(representative.ContractError, "instrumented"):
            representative.validate_coverage_summary(
                "100.00 % 0/0 src/a.ml\n", ["src/a.ml"], 9500
            )

    def test_coverage_normalizes_windows_source_paths(self) -> None:
        result = representative.validate_coverage_summary(
            " 96.64 % 290/300 src\\coding_adventures_state_machine.ml\n",
            ["src/coding_adventures_state_machine.ml"],
            9500,
        )
        self.assertEqual(9664, result["src/coding_adventures_state_machine.ml"])


class ArchiveTests(unittest.TestCase):
    def _archive(self, members: list[tuple[str, bytes, str]]) -> Path:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "package.tar.gz"
        with tarfile.open(path, "w:gz") as archive:
            for name, content, kind in members:
                info = tarfile.TarInfo(name)
                if kind == "symlink":
                    info.type = tarfile.SYMTYPE
                    info.linkname = "outside"
                    archive.addfile(info)
                else:
                    info.size = len(content)
                    archive.addfile(info, io.BytesIO(content))
        return path

    def test_archive_members_are_single_root_regular_paths(self) -> None:
        path = self._archive(
            [
                ("coding-adventures-sample-0.1.0/README.md", b"ok", "file"),
                ("coding-adventures-sample-0.1.0/src/main.ml", b"let x = 1", "file"),
            ]
        )
        members = representative.validate_archive_members(
            path,
            "coding-adventures-sample-0.1.0",
            {"README.md", "src/main.ml"},
        )
        self.assertEqual({"README.md", "src/main.ml"}, members)

    def test_archive_rejects_traversal_links_duplicates_and_generated_files(self) -> None:
        cases = (
            [("../escape", b"x", "file")],
            [("root/link", b"", "symlink")],
            [("root/README", b"a", "file"), ("root/README", b"b", "file")],
            [("root/_build/output", b"x", "file")],
        )
        for index, members in enumerate(cases):
            with self.subTest(case=index), self.assertRaises(
                representative.ContractError
            ):
                representative.validate_archive_members(
                    self._archive(members), "root", {"README"}
                )


class EvidenceTests(unittest.TestCase):
    def test_evidence_requires_every_package_and_both_downstream_receipts(self) -> None:
        manifest = representative.load_manifest(REPO_ROOT)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = [
                "installed-packages.txt",
                "toolchain.txt",
                "source-archives/SHA256SUMS",
                "downstream/representative.json",
                "downstream/transitive-leaf.json",
            ]
            for package in manifest["packages"]:
                package_id = package["id"]
                files.extend(
                    [
                        f"coverage/{package_id}/coverage-summary.txt",
                        f"coverage/{package_id}/bisect0001.coverage",
                        f"documentation/{package_id}.log",
                        f"documentation/{package_id}/index.html",
                        f"analyzer/{package_id}.txt",
                        (
                            "source-archives/"
                            f"coding-adventures-{package_id}-{package['version']}.tar.gz"
                        ),
                    ]
                )
            for relative in files:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("evidence\n", encoding="utf-8")
            representative.validate_evidence(manifest, root)
            (root / "analyzer/state-machine.txt").unlink()
            with self.assertRaisesRegex(
                representative.ContractError, "analyzer/state-machine"
            ):
                representative.validate_evidence(manifest, root)


class CliTests(unittest.TestCase):
    def test_cli_validate_repository_passes(self) -> None:
        output = io.StringIO()
        with redirect_stdout(output):
            result = representative.main(
                ["validate-repository", "--repo-root", str(REPO_ROOT)]
            )
        self.assertEqual(0, result)
        self.assertIn("OCAML07", output.getvalue())

    def test_cli_reports_contract_failure(self) -> None:
        error = io.StringIO()
        with redirect_stderr(error):
            result = representative.main(
                ["validate-coverage", "--source", "src/a.ml", "--minimum", "9500"]
            )
        self.assertEqual(1, result)
        self.assertIn("OCAML07", error.getvalue())


if __name__ == "__main__":
    unittest.main()
