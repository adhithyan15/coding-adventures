from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(SCRIPTS_DIR))

import ocaml_toolchain_lock as toolchain  # noqa: E402


class OcamlToolchainRepositoryTests(unittest.TestCase):
    def test_checked_in_contract_is_valid(self) -> None:
        manifest = toolchain.validate_repository(REPO_ROOT)

        self.assertEqual(1, manifest["schema_version"])
        self.assertEqual(
            {"linux-x64", "macos-x64", "windows-x64"},
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
        workflow = (
            REPO_ROOT / ".github/workflows/build-ocaml.yml"
        ).read_text(encoding="utf-8")

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
            lambda document: document["actions"].__setitem__(
                "setup_ocaml", "v3"
            ),
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


class OcamlToolchainDigestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)
        self.fixture_root = (
            self.root / "code/specs/fixtures/ocaml-toolchain"
        )
        self.fixture_root.mkdir(parents=True)

        source_fixture_root = (
            REPO_ROOT / "code/specs/fixtures/scaffold-generator"
        )
        for kind in ("ocaml-library", "ocaml-program"):
            destination = (
                self.root
                / "code/specs/fixtures/scaffold-generator"
                / kind
            )
            destination.mkdir(parents=True)
            source = (
                source_fixture_root
                / kind
                / "coding-adventures-my-pkg.opam"
            )
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
                        for name, version in self.manifest[
                            "direct_versions"
                        ].items()
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

    @mock.patch("ocaml_toolchain_lock.subprocess.run")
    def test_runtime_probe_executes_closed_commands(
        self, run: mock.Mock
    ) -> None:
        outputs = (
            "2.5.2",
            "5.2.1",
            "3.17.2",
            "1.9.0",
            "2.8.3",
            "0.27.0",
        )
        run.side_effect = [
            mock.Mock(returncode=0, stdout=output, stderr="")
            for output in outputs
        ]

        toolchain.validate_runtime(toolchain.load_manifest(REPO_ROOT))

        self.assertEqual(6, run.call_count)
        for call in run.call_args_list:
            self.assertTrue(call.kwargs["check"])
            self.assertFalse(call.kwargs["shell"])


if __name__ == "__main__":
    unittest.main()
