from __future__ import annotations

import copy
import hashlib
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(SCRIPTS_DIR))

import ocaml_toolchain_lock as toolchain


class OcamlToolchainRepositoryTests(unittest.TestCase):
    def test_checked_in_contract_is_valid(self) -> None:
        manifest = toolchain.validate_repository(REPO_ROOT)

        self.assertEqual(1, manifest["schema_version"])
        self.assertEqual(
            {"linux-x64", "macos-arm64", "windows-x64"},
            set(manifest["targets"]),
        )

    def test_fixture_inputs_are_byte_identical(self) -> None:
        library = (
            REPO_ROOT
            / "code/specs/fixtures/scaffold-generator/ocaml-library"
            / "coding-adventures-my-pkg.opam"
        )
        program = (
            REPO_ROOT
            / "code/specs/fixtures/scaffold-generator/ocaml-program"
            / "coding-adventures-my-pkg.opam"
        )

        self.assertEqual(library.read_bytes(), program.read_bytes())

    def test_workflow_uses_closed_matrix_and_pinned_identities(self) -> None:
        manifest = toolchain.load_manifest(REPO_ROOT)
        workflow = (REPO_ROOT / ".github/workflows/build-ocaml.yml").read_text(
            encoding="utf-8"
        )

        toolchain.validate_workflow_text(manifest, workflow)
        self.assertNotIn("continue-on-error:", workflow)
        self.assertNotIn("|| true", workflow)
        self.assertNotIn("secrets.", workflow)


class OcamlToolchainShapeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = toolchain.load_manifest(REPO_ROOT)

    def assert_rejected(self, mutator: object, message: str) -> None:
        document = copy.deepcopy(self.manifest)
        mutator(document)  # type: ignore[operator]
        with self.assertRaisesRegex(toolchain.ContractError, message):
            toolchain.validate_manifest_shape(document)

    def test_rejects_unknown_top_level_key(self) -> None:
        self.assert_rejected(
            lambda document: document.__setitem__("extra", True),
            "unknown keys",
        )

    def test_rejects_missing_direct_version(self) -> None:
        self.assert_rejected(
            lambda document: document["direct_versions"].pop("ocamlformat"),
            "direct_versions",
        )

    def test_rejects_malformed_action_commit(self) -> None:
        self.assert_rejected(
            lambda document: document["actions"].__setitem__("setup_ocaml", "v3"),
            "40 lowercase hexadecimal",
        )

    def test_rejects_unknown_target(self) -> None:
        self.assert_rejected(
            lambda document: document["targets"].__setitem__(
                "freebsd-x64",
                copy.deepcopy(document["targets"]["linux-x64"]),
            ),
            "targets",
        )

    def test_rejects_target_path_traversal(self) -> None:
        self.assert_rejected(
            lambda document: document["targets"]["linux-x64"].__setitem__(
                "lock_file", "../outside.opam.locked"
            ),
            "safe relative path",
        )

    def test_rejects_windows_compiler_on_unix(self) -> None:
        self.assert_rejected(
            lambda document: document["targets"]["linux-x64"].__setitem__(
                "windows_compiler", "mingw"
            ),
            "windows_compiler",
        )

    def test_rejects_missing_windows_compiler(self) -> None:
        self.assert_rejected(
            lambda document: document["targets"]["windows-x64"].__setitem__(
                "windows_compiler", None
            ),
            "windows_compiler",
        )

    def test_rejects_other_closed_shape_drift(self) -> None:
        cases = (
            (
                lambda document: document.__setitem__("schema_version", 2),
                "schema_version",
            ),
            (
                lambda document: document["direct_versions"].__setitem__(
                    "ocaml", "five"
                ),
                "semantic version",
            ),
            (
                lambda document: document["direct_versions"].__setitem__(
                    "ocaml", "5.2.2"
                ),
                "reviewed version",
            ),
            (
                lambda document: document["actions"].__setitem__("checkout", "0" * 40),
                "reviewed commit",
            ),
            (
                lambda document: document.__setitem__(
                    "opam_repository_commit", "0" * 40
                ),
                "reviewed commit",
            ),
            (
                lambda document: document.__setitem__(
                    "fixture_input_sha256", "not-a-digest"
                ),
                "SHA-256",
            ),
            (
                lambda document: document["targets"]["linux-x64"].__setitem__(
                    "runner_arch", "ARM64"
                ),
                "runner_arch",
            ),
            (
                lambda document: document["targets"]["linux-x64"].__setitem__(
                    "lock_file",
                    "windows-x64/coding-adventures-my-pkg.opam.locked",
                ),
                "target directory",
            ),
            (
                lambda document: document["targets"]["linux-x64"].__setitem__(
                    "receipt_file", "linux-x64/packages.txt"
                ),
                "target directory",
            ),
            (
                lambda document: document["targets"]["linux-x64"][
                    "runner_image"
                ].__setitem__("image_os", ""),
                "must be nonempty",
            ),
        )
        for mutate, message in cases:
            with self.subTest(message=message):
                self.assert_rejected(mutate, message)

    def test_rejects_non_object_manifest(self) -> None:
        with self.assertRaisesRegex(toolchain.ContractError, "must be an object"):
            toolchain.validate_manifest_shape([])


class OcamlToolchainWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = toolchain.load_manifest(REPO_ROOT)
        cls.workflow = (REPO_ROOT / ".github/workflows/build-ocaml.yml").read_text(
            encoding="utf-8"
        )

    def test_rejects_missing_contract_fragments(self) -> None:
        cases = (
            ("diff -u", "required fragment"),
            ("ubuntu-24.04", "runner label"),
            ("1.9.0", "exact version"),
        )
        for removed, message in cases:
            with (
                self.subTest(removed=removed),
                self.assertRaisesRegex(toolchain.ContractError, message),
            ):
                toolchain.validate_workflow_text(
                    self.manifest, self.workflow.replace(removed, "REMOVED")
                )

    def test_rejects_forbidden_workflow_constructs(self) -> None:
        for forbidden in (
            "continue-on-error: true",
            "run: command || true",
            "${{ secrets.TOKEN }}",
            "dune-cache: true",
            "opam-pin: true",
        ):
            with (
                self.subTest(forbidden=forbidden),
                self.assertRaisesRegex(toolchain.ContractError, "forbidden"),
            ):
                toolchain.validate_workflow_text(
                    self.manifest, f"{self.workflow}\n{forbidden}\n"
                )

    def test_rejects_unpinned_action_references(self) -> None:
        for reference in ("actions/example", "actions/example@v1"):
            with (
                self.subTest(reference=reference),
                self.assertRaisesRegex(toolchain.ContractError, "not commit-pinned"),
            ):
                toolchain.validate_workflow_text(
                    self.manifest,
                    f"{self.workflow}\n      uses: {reference}\n",
                )


class OcamlToolchainDigestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)
        self.fixture_root = self.root / "code/specs/fixtures/ocaml-toolchain"
        self.fixture_root.mkdir(parents=True)

        source_fixture_root = REPO_ROOT / "code/specs/fixtures/scaffold-generator"
        for kind in ("ocaml-library", "ocaml-program"):
            destination = self.root / "code/specs/fixtures/scaffold-generator" / kind
            destination.mkdir(parents=True)
            source = source_fixture_root / kind / "coding-adventures-my-pkg.opam"
            (destination / source.name).write_bytes(source.read_bytes())

        self.manifest = copy.deepcopy(toolchain.load_manifest(REPO_ROOT))
        for target in self.manifest["targets"].values():
            for path_key, digest_key in (
                ("lock_file", "lock_sha256"),
                ("receipt_file", "receipt_sha256"),
            ):
                path = self.fixture_root / target[path_key]
                path.parent.mkdir(parents=True, exist_ok=True)
                if path_key == "lock_file":
                    content = "\n".join(
                        f'"{name}" {{= "{version}"}}'
                        for name, version in self.manifest["direct_versions"].items()
                    ).encode("utf-8")
                else:
                    content = b"ocaml.5.2.1\n"
                path.write_bytes(content)
                target[digest_key] = hashlib.sha256(content).hexdigest()

        input_path = (
            self.root
            / "code/specs/fixtures/scaffold-generator/ocaml-library"
            / "coding-adventures-my-pkg.opam"
        )
        self.manifest["fixture_input_sha256"] = hashlib.sha256(
            input_path.read_bytes()
        ).hexdigest()
        (self.fixture_root / "toolchain-lock.json").write_text(
            json.dumps(self.manifest), encoding="utf-8"
        )

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_accepts_matching_regular_evidence(self) -> None:
        toolchain.validate_repository(self.root, check_workflow=False)

    def test_rejects_evidence_digest_mismatch(self) -> None:
        target = self.manifest["targets"]["linux-x64"]
        (self.fixture_root / target["receipt_file"]).write_text(
            "tampered\n", encoding="utf-8"
        )

        with self.assertRaisesRegex(toolchain.ContractError, "digest mismatch"):
            toolchain.validate_repository(self.root, check_workflow=False)

    def test_rejects_missing_evidence(self) -> None:
        target = self.manifest["targets"]["linux-x64"]
        (self.fixture_root / target["receipt_file"]).unlink()

        with self.assertRaisesRegex(toolchain.ContractError, "regular file"):
            toolchain.validate_repository(self.root, check_workflow=False)

    def test_rejects_different_fixture_inputs(self) -> None:
        program = (
            self.root
            / "code/specs/fixtures/scaffold-generator/ocaml-program"
            / "coding-adventures-my-pkg.opam"
        )
        program.write_text("different\n", encoding="utf-8")

        with self.assertRaisesRegex(toolchain.ContractError, "inputs differ"):
            toolchain.validate_repository(self.root, check_workflow=False)

    def test_rejects_lock_without_direct_dependency(self) -> None:
        target = self.manifest["targets"]["linux-x64"]
        lock = self.fixture_root / target["lock_file"]
        content = lock.read_text(encoding="utf-8").replace('"dune" {= "3.17.2"}', "")
        lock.write_bytes(content.encode("utf-8"))
        target["lock_sha256"] = hashlib.sha256(content.encode("utf-8")).hexdigest()
        (self.fixture_root / "toolchain-lock.json").write_text(
            json.dumps(self.manifest), encoding="utf-8"
        )

        with self.assertRaisesRegex(toolchain.ContractError, "omits exact"):
            toolchain.validate_repository(self.root, check_workflow=False)

    def test_rejects_symlinked_evidence(self) -> None:
        if sys.platform == "win32":
            self.skipTest("unprivileged Windows cannot reliably create symlinks")
        target = self.manifest["targets"]["linux-x64"]
        path = self.fixture_root / target["receipt_file"]
        path.unlink()
        path.symlink_to(self.fixture_root / target["lock_file"])

        with self.assertRaisesRegex(toolchain.ContractError, "regular file"):
            toolchain.validate_repository(self.root, check_workflow=False)


class OcamlToolchainRuntimeTests(unittest.TestCase):
    def test_exact_tool_versions_accept_expected_outputs(self) -> None:
        manifest = toolchain.load_manifest(REPO_ROOT)
        toolchain.validate_tool_version_outputs(
            manifest,
            {
                "opam": "2.5.2\n",
                "ocaml": "The OCaml toplevel, version 5.2.1\n",
                "dune": "3.17.2\n",
                "alcotest": "1.9.0\n",
                "bisect_ppx": "2.8.3\n",
                "ocamlformat": "0.27.0\n",
            },
        )

    def test_exact_tool_versions_reject_drift(self) -> None:
        manifest = toolchain.load_manifest(REPO_ROOT)
        with self.assertRaisesRegex(toolchain.ContractError, "dune"):
            toolchain.validate_tool_version_outputs(
                manifest,
                {
                    "opam": "2.5.2",
                    "ocaml": "5.2.1",
                    "dune": "3.18.0",
                    "alcotest": "1.9.0",
                    "bisect_ppx": "2.8.3",
                    "ocamlformat": "0.27.0",
                },
            )

    def test_exact_tool_versions_reject_missing_probe(self) -> None:
        manifest = toolchain.load_manifest(REPO_ROOT)
        with self.assertRaisesRegex(toolchain.ContractError, "output keys"):
            toolchain.validate_tool_version_outputs(manifest, {})

    @mock.patch("ocaml_toolchain_lock.subprocess.run")
    def test_runtime_probe_executes_closed_commands(self, run: mock.Mock) -> None:
        outputs = (
            "2.5.2",
            "5.2.1",
            "3.17.2",
            "1.9.0",
            "2.8.3",
            "0.27.0",
        )
        run.side_effect = [
            mock.Mock(returncode=0, stdout=output, stderr="") for output in outputs
        ]

        toolchain.validate_runtime(toolchain.load_manifest(REPO_ROOT))

        self.assertEqual(6, run.call_count)
        for call in run.call_args_list:
            self.assertTrue(call.kwargs["check"])
            self.assertFalse(call.kwargs["shell"])


class OcamlToolchainLoadingAndCliTests(unittest.TestCase):
    def test_load_manifest_rejects_missing_and_invalid_json(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(toolchain.ContractError, "missing manifest"):
                toolchain.load_manifest(root)

            path = root / toolchain.MANIFEST_RELATIVE_PATH
            path.parent.mkdir(parents=True)
            path.write_text("{", encoding="utf-8")
            with self.assertRaisesRegex(toolchain.ContractError, "cannot read"):
                toolchain.load_manifest(root)

    def test_cli_validate_repository_passes(self) -> None:
        output = io.StringIO()
        with redirect_stdout(output):
            result = toolchain.main(
                ["validate-repository", "--repo-root", str(REPO_ROOT)]
            )

        self.assertEqual(0, result)
        self.assertIn("passed", output.getvalue())

    @mock.patch("ocaml_toolchain_lock.validate_runtime")
    def test_cli_validate_runtime_passes(self, validate_runtime: mock.Mock) -> None:
        output = io.StringIO()
        with redirect_stdout(output):
            result = toolchain.main(["validate-runtime", "--repo-root", str(REPO_ROOT)])

        self.assertEqual(0, result)
        validate_runtime.assert_called_once()

    @mock.patch(
        "ocaml_toolchain_lock.validate_repository",
        side_effect=toolchain.ContractError("broken"),
    )
    def test_cli_reports_contract_failure(self, _validate: mock.Mock) -> None:
        error = io.StringIO()
        with redirect_stderr(error):
            result = toolchain.main(
                ["validate-repository", "--repo-root", str(REPO_ROOT)]
            )

        self.assertEqual(1, result)
        self.assertIn("broken", error.getvalue())


if __name__ == "__main__":
    unittest.main()
