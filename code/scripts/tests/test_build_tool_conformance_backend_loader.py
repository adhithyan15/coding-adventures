from __future__ import annotations

import hashlib
import json
import os
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_conformance as bootstrap
import build_tool_conformance_authority as authority
import build_tool_conformance_backend_loader as loader

REPO_ROOT = SCRIPTS_DIR.parents[1]
FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1"
BACKEND_PATH = SCRIPTS_DIR / "build_tool_conformance_linux_oci.py"
LOADER_PATH = SCRIPTS_DIR / "build_tool_conformance_backend_loader.py"


def identity_bytes() -> bytes:
    manifest = "4" * 64
    value = {
        "schema_version": 1,
        "backend_kind": "linux_oci",
        "platform": "linux",
        "architecture": "amd64",
        "runtime": {
            "implementation": "podman",
            "path": "/usr/bin/podman",
            "version": "5.8.3",
            "linkage": "static",
            "sha256": "1" * 64,
        },
        "oci_runtime": {
            "implementation": "crun",
            "path": "/usr/bin/crun",
            "sha256": "2" * 64,
        },
        "conmon": {
            "implementation": "conmon",
            "path": "/usr/bin/conmon",
            "sha256": "8" * 64,
        },
        "image": {
            "reference": f"localhost/build-tool@sha256:{manifest}",
            "manifest_sha256": manifest,
            "config_sha256": "5" * 64,
            "os": "linux",
            "architecture": "amd64",
        },
        "seccomp_profile_sha256": "6" * 64,
        "shim": {"path": "/opt/conformance/shim", "sha256": "7" * 64},
        "probe": {"path": "/opt/conformance/probe", "sha256": "8" * 64},
    }
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode() + b"\n"


def source_bytes(body: str = "") -> bytes:
    return (
        "from __future__ import annotations\n"
        "from dataclasses import dataclass\n"
        "\n"
        "class LinuxOciUnavailable(RuntimeError):\n"
        "    def __init__(self, code: str, message: str) -> None:\n"
        "        super().__init__(message)\n"
        "        self.code = code\n"
        "        self.message = message\n"
        "\n"
        "@dataclass(frozen=True)\n"
        "class CommandResult:\n"
        "    returncode: int\n"
        "    stdout: bytes\n"
        "    stderr: bytes\n"
        "\n"
        "def preflight_brokered(\n"
        "    identity,\n"
        "    *,\n"
        "    runtime_info,\n"
        "    image_inspect,\n"
        "    platform_name=None,\n"
        "    effective_uid=None,\n"
        "):\n"
        "    raise AssertionError('loadability validation must not call preflight')\n"
        f"{body}"
    ).encode()


def manifest_bytes(imports: list[str]) -> bytes:
    value = {
        "schema_version": 1,
        "module": "build_tool_conformance_linux_oci",
        "imports": imports,
        "required_exports": [
            "CommandResult",
            "LinuxOciUnavailable",
            "preflight_brokered",
        ],
    }
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode() + b"\n"


def write_loader_authority_fixture(root: Path) -> dict[str, object]:
    repository = root / "repository"
    bundle_root = root / "authority"
    bundle_root.mkdir(parents=True)
    components: dict[str, dict[str, object]] = {}
    for role, relative in authority.LOADER_REPOSITORY_COMPONENT_PATHS.items():
        raw = (REPO_ROOT / relative).read_bytes()
        target = repository / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
        components[role] = {
            "provenance": "repository",
            "path": relative,
            "byte_length": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
    raw_identity = identity_bytes()
    (bundle_root / "linux-oci-backend.json").write_bytes(raw_identity)
    components["linux_backend_identity"] = {
        "provenance": "bundle",
        "path": "linux-oci-backend.json",
        "byte_length": len(raw_identity),
        "sha256": hashlib.sha256(raw_identity).hexdigest(),
    }
    bundle = {
        "schema_version": 1,
        "purpose": "build-tool-trusted-authority",
        "authorization_scope": "linux_capability_preflight_loader_v1",
        "repository": "github.com/adhithyan15/coding-adventures",
        "conformance_revision": "v1",
        "platform": "linux",
        "architecture": "amd64",
        "source": {
            "git_object_format": "sha1",
            "commit_oid": "a" * 40,
            "tree_oid": "b" * 40,
        },
        "components": components,
    }
    raw_bundle = (
        json.dumps(bundle, sort_keys=True, separators=(",", ":")).encode() + b"\n"
    )
    bundle_path = bundle_root / "authority.json"
    bundle_path.write_bytes(raw_bundle)
    return {
        "repository": repository,
        "bundle": bundle,
        "bundle_path": bundle_path,
        "digest": authority.loader_authority_bundle_sha256(raw_bundle),
    }


class ImportManifestTests(unittest.TestCase):
    def test_loader_authority_schema_is_closed_and_domain_separated(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_loader_authority_fixture(Path(directory))
            schema = bootstrap.load_document(
                FIXTURE_ROOT / "execution-preflight-loader-authority.schema.json"
            )
            self.assertEqual(
                bootstrap._schema_errors(fixture["bundle"], schema),
                [],
            )
            raw = fixture["bundle_path"].read_bytes()  # type: ignore[union-attr]
        manual = hashlib.sha256(
            authority.LOADER_AUTHORITY_DOMAIN + struct.pack(">Q", len(raw)) + raw
        ).hexdigest()
        self.assertEqual(fixture["digest"], manual)
        self.assertNotEqual(
            authority.authority_bundle_sha256(raw),
            manual,
        )

    def test_repository_backend_matches_closed_manifest(self) -> None:
        manifest = loader.parse_import_manifest(
            (FIXTURE_ROOT / "preflight-imports.json").read_bytes()
        )
        self.assertEqual(
            loader.source_imports(BACKEND_PATH.read_bytes()),
            frozenset(manifest.imports),
        )
        self.assertIsNone(
            loader._validate_backend_structure(BACKEND_PATH.read_bytes(), manifest)
        )

    def test_manifest_rejects_duplicates_unknown_fields_and_bad_exports(self) -> None:
        valid = json.loads(manifest_bytes(["__future__", "dataclasses"]))
        mutations = []
        duplicate = dict(valid)
        duplicate["imports"] = ["dataclasses", "dataclasses"]
        mutations.append(duplicate)
        extra = dict(valid)
        extra["ambient"] = True
        mutations.append(extra)
        exports = dict(valid)
        exports["required_exports"] = ["preflight"]
        mutations.append(exports)
        third_party = dict(valid)
        third_party["imports"] = ["jsonschema"]
        mutations.append(third_party)
        for value in mutations:
            with (
                self.subTest(value=value),
                self.assertRaises(loader.LoaderUnavailable),
            ):
                loader.parse_import_manifest(json.dumps(value).encode())

    def test_undeclared_and_dynamic_imports_fail_before_execution(self) -> None:
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader.validate_source_closure(
                b"import os\nimport subprocess\n",
                loader.parse_import_manifest(manifest_bytes(["os"])),
            )
        self.assertEqual(raised.exception.code, "LOADER_IMPORT_CLOSURE_MISMATCH")

        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader.validate_source_closure(
                b"__import__('os')\n",
                loader.parse_import_manifest(manifest_bytes([])),
            )
        self.assertEqual(raised.exception.code, "LOADER_DYNAMIC_IMPORT_FORBIDDEN")

        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader.validate_source_closure(
                b"from os import *\n",
                loader.parse_import_manifest(manifest_bytes(["os"])),
            )
        self.assertEqual(raised.exception.code, "LOADER_WILDCARD_IMPORT_FORBIDDEN")

    def test_backend_definitions_load_without_invoking_preflight(self) -> None:
        manifest = loader.parse_import_manifest(
            manifest_bytes(["__future__", "dataclasses"])
        )
        self.assertIsNone(loader._validate_backend_structure(source_bytes(), manifest))

        dangerous = (
            b"import subprocess\n"
            b"subprocess.run(['/usr/bin/podman', 'info'])\n"
            b"class LinuxOciUnavailable(RuntimeError):\n"
            b"    pass\n"
            b"class CommandResult:\n"
            b"    pass\n"
            b"def preflight_brokered(identity, *, state_root):\n"
            b"    return None\n"
        )
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader._validate_backend_structure(
                dangerous,
                loader.parse_import_manifest(manifest_bytes(["subprocess"])),
            )
        self.assertEqual(
            raised.exception.code,
            "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
        )

        for imports, dangerous_source in (
            (
                ["sys"],
                b"import sys\nsys.modules['subprocess'].run(['podman'])\n",
            ),
            (
                ["pathlib"],
                b"import pathlib\npathlib.os.system('podman info')\n",
            ),
        ):
            with (
                self.subTest(imports=imports),
                self.assertRaises(loader.LoaderUnavailable) as raised,
            ):
                loader.validate_source_closure(
                    dangerous_source,
                    loader.parse_import_manifest(manifest_bytes(imports)),
                )
            self.assertEqual(
                raised.exception.code,
                "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
            )

    def test_required_backend_interface_is_closed(self) -> None:
        invalid = (
            b"class LinuxOciUnavailable(RuntimeError):\n"
            b"    pass\n"
            b"class CommandResult:\n"
            b"    pass\n"
        )
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader._validate_backend_structure(
                invalid,
                loader.parse_import_manifest(manifest_bytes([])),
            )
        self.assertEqual(raised.exception.code, "LOADER_BACKEND_INTERFACE_INVALID")

        stable_source = source_bytes()
        legacy_alias = source_bytes(
            "\ndef preflight_prevalidated(\n"
            "    identity, *, runtime_info, image_inspect,\n"
            "    platform_name=None, effective_uid=None,\n"
            "):\n"
            "    return preflight_brokered(\n"
            "        identity,\n"
            "        runtime_info=runtime_info,\n"
            "        image_inspect=image_inspect,\n"
            "        platform_name=platform_name,\n"
            "        effective_uid=effective_uid,\n"
            "    )\n"
        )
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader._validate_backend_structure(
                legacy_alias,
                loader.parse_import_manifest(
                    manifest_bytes(["__future__", "dataclasses"])
                ),
            )
        self.assertEqual(raised.exception.code, "LOADER_BACKEND_INTERFACE_INVALID")

        for name, malformed in (
            (
                "error-fields",
                stable_source.replace(
                    b"    def __init__(self, code: str, message: str) -> None:\n"
                    b"        super().__init__(message)\n"
                    b"        self.code = code\n"
                    b"        self.message = message\n",
                    b"    pass\n",
                ),
            ),
            (
                "command-field-type",
                stable_source.replace(b"    stdout: bytes\n", b"    stdout: str\n"),
            ),
        ):
            with (
                self.subTest(name=name),
                self.assertRaises(loader.LoaderUnavailable) as raised,
            ):
                loader._validate_backend_structure(
                    malformed,
                    loader.parse_import_manifest(
                        manifest_bytes(["__future__", "dataclasses"])
                    ),
                )
            self.assertEqual(
                raised.exception.code,
                "LOADER_BACKEND_INTERFACE_INVALID",
            )

        required_keywords = (
            b"from dataclasses import dataclass\n"
            b"class LinuxOciUnavailable(RuntimeError):\n"
            b"    def __init__(self, code, message):\n"
            b"        self.code = code\n"
            b"        self.message = message\n"
            b"@dataclass(frozen=True)\n"
            b"class CommandResult:\n"
            b"    returncode: int\n"
            b"    stdout: bytes\n"
            b"    stderr: bytes\n"
            b"def preflight_brokered(\n"
            b"    identity, *, state_root, command_runner, binary_digest,\n"
            b"    platform_name, effective_uid\n"
            b"):\n"
            b"    pass\n"
        )
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader._validate_backend_structure(
                required_keywords,
                loader.parse_import_manifest(manifest_bytes(["dataclasses"])),
            )
        self.assertEqual(raised.exception.code, "LOADER_BACKEND_INTERFACE_INVALID")

        extra_subscript_field = source_bytes().replace(
            b"    stderr: bytes\n",
            b"    stderr: bytes\n    extra: list[int]\n",
        )
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader._validate_backend_structure(
                extra_subscript_field,
                loader.parse_import_manifest(
                    manifest_bytes(["__future__", "dataclasses"])
                ),
            )
        self.assertEqual(raised.exception.code, "LOADER_BACKEND_INTERFACE_INVALID")

        ambiguous = (
            b"class LinuxOciUnavailable(RuntimeError):\n"
            b"    pass\n"
            b"class CommandResult:\n"
            b"    pass\n"
            b"def preflight_brokered(identity, required_extra, *, state_root):\n"
            b"    pass\n"
        )
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader._validate_backend_structure(
                ambiguous,
                loader.parse_import_manifest(manifest_bytes([])),
            )
        self.assertEqual(raised.exception.code, "LOADER_BACKEND_INTERFACE_INVALID")


class LoaderAuthorityLogicTests(unittest.TestCase):
    def _authorize_with_retained_bytes(
        self,
        fixture: dict[str, object],
    ) -> authority.LoaderAuthority:
        repository = fixture["repository"]
        bundle_path = fixture["bundle_path"]
        assert isinstance(repository, Path)
        assert isinstance(bundle_path, Path)

        def retained_read(
            descriptor: int,
            relative: str,
            *,
            label: str,
            max_bytes: int,
        ) -> bytes:
            del label, max_bytes
            if descriptor == 101:
                return (repository / relative).read_bytes()
            return (bundle_path.parent / relative).read_bytes()

        with (
            mock.patch.object(
                authority,
                "_open_absolute_directory",
                side_effect=[101, 202],
            ),
            mock.patch.object(
                authority,
                "_read_bound_regular_at",
                side_effect=retained_read,
            ),
            mock.patch.object(authority.os, "close"),
        ):
            return authority.authorize_backend_loader(
                bundle_path,
                approved_digest=fixture["digest"],  # type: ignore[arg-type]
                expected_commit_oid="a" * 40,
                expected_tree_oid="b" * 40,
                repository_root=repository,
            )

    def test_loader_profile_authorizes_exact_roles_only(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_loader_authority_fixture(Path(directory))
            approved = self._authorize_with_retained_bytes(fixture)
        self.assertEqual(
            set(approved.components),
            set(authority.LOADER_COMPONENT_ROLES),
        )
        self.assertEqual(
            approved.bundle["authorization_scope"],
            "linux_capability_preflight_loader_v1",
        )
        self.assertFalse(approved.policy["enabled"])

    def test_old_scope_cannot_authorize_loader(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_loader_authority_fixture(Path(directory))
            bundle = fixture["bundle"]
            bundle_path = fixture["bundle_path"]
            assert isinstance(bundle, dict)
            assert isinstance(bundle_path, Path)
            bundle["authorization_scope"] = "linux_capability_preflight_v1"
            raw = (
                json.dumps(bundle, sort_keys=True, separators=(",", ":")).encode()
                + b"\n"
            )
            bundle_path.write_bytes(raw)
            fixture["digest"] = authority.loader_authority_bundle_sha256(raw)
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                self._authorize_with_retained_bytes(fixture)
        self.assertEqual(
            raised.exception.code,
            "LOADER_AUTHORITY_PROFILE_INVALID",
        )


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux sealed memfd contract")
class IsolatedLoaderTests(unittest.TestCase):
    def test_loader_authority_retains_exact_closed_components(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_loader_authority_fixture(Path(directory))
            approved = authority.authorize_backend_loader(
                fixture["bundle_path"],  # type: ignore[arg-type]
                approved_digest=fixture["digest"],  # type: ignore[arg-type]
                expected_commit_oid="a" * 40,
                expected_tree_oid="b" * 40,
                repository_root=fixture["repository"],  # type: ignore[arg-type]
            )
        self.assertEqual(
            set(approved.components),
            set(authority.LOADER_COMPONENT_ROLES),
        )
        self.assertEqual(
            approved.bundle["authorization_scope"],
            "linux_capability_preflight_loader_v1",
        )

    def test_intermediate_symlink_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_loader_authority_fixture(Path(directory))
            repository = fixture["repository"]
            assert isinstance(repository, Path)
            scripts = repository / "code" / "scripts"
            retained = repository / "code" / "scripts-retained"
            scripts.rename(retained)
            scripts.symlink_to(retained, target_is_directory=True)
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                authority.authorize_backend_loader(
                    fixture["bundle_path"],  # type: ignore[arg-type]
                    approved_digest=fixture["digest"],  # type: ignore[arg-type]
                    expected_commit_oid="a" * 40,
                    expected_tree_oid="b" * 40,
                    repository_root=repository,
                )
        self.assertEqual(
            raised.exception.code,
            "LOADER_AUTHORITY_COMPONENT_READ_FAILED",
        )

    def test_final_fifo_is_rejected_without_blocking(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_loader_authority_fixture(Path(directory))
            repository = fixture["repository"]
            assert isinstance(repository, Path)
            loader_path = (
                repository
                / authority.LOADER_REPOSITORY_COMPONENT_PATHS["preflight_loader"]
            )
            loader_path.unlink()
            os.mkfifo(loader_path)
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                authority.authorize_backend_loader(
                    fixture["bundle_path"],  # type: ignore[arg-type]
                    approved_digest=fixture["digest"],  # type: ignore[arg-type]
                    expected_commit_oid="a" * 40,
                    expected_tree_oid="b" * 40,
                    repository_root=repository,
                )
        self.assertEqual(
            raised.exception.code,
            "LOADER_AUTHORITY_COMPONENT_READ_FAILED",
        )

    def test_missing_root_has_stable_error(self) -> None:
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            authority._open_absolute_directory(
                Path("/definitely-missing-loader-authority-root")
            )
        self.assertEqual(
            raised.exception.code,
            "LOADER_AUTHORITY_ROOT_INVALID",
        )

    def test_exact_backend_loads_without_calling_preflight(self) -> None:
        receipt = loader.validate_exact_backend(
            loader_source=LOADER_PATH.read_bytes(),
            backend_source=source_bytes(),
            import_manifest=manifest_bytes(["__future__", "dataclasses"]),
            identity=identity_bytes(),
        )
        self.assertEqual(receipt["status"], "loadable")
        self.assertEqual(receipt["conformance_status"], "not-run")
        self.assertEqual(receipt["authorization_scope"], "loadability-only")

    def test_poisoned_python_environment_is_not_authority(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            poison = Path(directory)
            (poison / "sitecustomize.py").write_text(
                "raise RuntimeError('ambient sitecustomize ran')\n",
                encoding="utf-8",
            )
            old_pythonpath = os.environ.get("PYTHONPATH")
            os.environ["PYTHONPATH"] = str(poison)
            try:
                receipt = loader.validate_exact_backend(
                    loader_source=LOADER_PATH.read_bytes(),
                    backend_source=source_bytes(),
                    import_manifest=manifest_bytes(["__future__", "dataclasses"]),
                    identity=identity_bytes(),
                )
            finally:
                if old_pythonpath is None:
                    os.environ.pop("PYTHONPATH", None)
                else:
                    os.environ["PYTHONPATH"] = old_pythonpath
        self.assertEqual(receipt["status"], "loadable")

    def test_import_time_process_attempt_is_blocked(self) -> None:
        dangerous = (
            b"import subprocess\n"
            b"subprocess.run(['/usr/bin/podman', 'info'])\n"
            b"class LinuxOciUnavailable(RuntimeError):\n"
            b"    pass\n"
            b"class CommandResult:\n"
            b"    pass\n"
            b"def preflight_brokered(identity, *, state_root):\n"
            b"    return None\n"
        )
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader.validate_exact_backend(
                loader_source=LOADER_PATH.read_bytes(),
                backend_source=dangerous,
                import_manifest=manifest_bytes(["subprocess"]),
                identity=identity_bytes(),
            )
        self.assertEqual(
            raised.exception.code,
            "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
        )

    def test_sealed_input_cannot_be_mutated_after_worker_start(self) -> None:
        descriptor = loader._sealed_memfd("test-input", b"approved")
        try:
            with self.assertRaises(OSError):
                os.pwrite(descriptor, b"changed", 0)
            self.assertEqual(os.pread(descriptor, 8, 0), b"approved")
        finally:
            os.close(descriptor)

    def test_worker_output_is_streamed_to_a_hard_combined_cap(self) -> None:
        flood_loader = b"import os\nos.write(1, b'x' * 70000)\n"
        with self.assertRaises(loader.LoaderUnavailable) as raised:
            loader.validate_exact_backend(
                loader_source=flood_loader,
                backend_source=source_bytes(),
                import_manifest=manifest_bytes(["__future__", "dataclasses"]),
                identity=identity_bytes(),
            )
        self.assertEqual(raised.exception.code, "LOADER_WORKER_OUTPUT_LIMIT")


if __name__ == "__main__":
    unittest.main()
