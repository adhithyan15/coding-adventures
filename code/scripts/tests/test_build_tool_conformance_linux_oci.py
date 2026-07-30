from __future__ import annotations

import copy
import hashlib
import inspect
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_conformance as bootstrap
import build_tool_conformance_linux_oci as linux_oci

FIXTURE_ROOT = bootstrap.DEFAULT_FIXTURE_ROOT


def backend_identity() -> dict[str, object]:
    return {
        "schema_version": 1,
        "backend_kind": "linux_oci",
        "platform": "linux",
        "architecture": "amd64",
        "runtime": {
            "implementation": "podman",
            "path": "/usr/bin/podman",
            "version": "4.9.3",
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
            "reference": f"ghcr.io/coding-adventures/build-tool-probe@sha256:{'3' * 64}",
            "manifest_sha256": "3" * 64,
            "config_sha256": "4" * 64,
            "os": "linux",
            "architecture": "amd64",
        },
        "seccomp_profile_sha256": "5" * 64,
        "shim": {
            "path": "/usr/local/bin/build-tool-conformance-shim",
            "sha256": "6" * 64,
        },
        "probe": {
            "path": "/usr/local/bin/build-tool-conformance-probe",
            "sha256": "7" * 64,
        },
    }


def image_info() -> list[dict[str, object]]:
    identity = backend_identity()
    image = identity["image"]
    assert isinstance(image, dict)
    return [
        {
            "Id": image["config_sha256"],
            "Digest": f"sha256:{image['manifest_sha256']}",
            "RepoDigests": [image["reference"]],
            "Os": "linux",
            "Architecture": "amd64",
            "Config": {
                "Volumes": None,
            },
        }
    ]


def runtime_version() -> dict[str, object]:
    return {
        "Client": {
            "Version": "4.9.3",
            "Os": "linux",
            "OsArch": "linux/amd64",
        }
    }


class LinuxOciBackendTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = bootstrap.load_document(
            FIXTURE_ROOT / "linux-oci-backend.schema.json"
        )

    def test_identity_schema_is_closed_and_semantics_bind_manifest(self) -> None:
        identity = backend_identity()
        self.assertEqual(bootstrap._schema_errors(identity, self.schema), [])
        linux_oci.validate_identity(identity, self.schema)

        unknown = copy.deepcopy(identity)
        unknown["runtime"]["arguments"] = ["--fixture-controlled"]  # type: ignore[index]
        self.assertTrue(bootstrap._schema_errors(unknown, self.schema))
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci.validate_identity(unknown, self.schema)
        self.assertEqual(
            raised.exception.code,
            "LINUX_OCI_IDENTITY_SCHEMA_INVALID",
        )

        for field, mutate in (
            (
                "boolean-schema-version",
                lambda value: value.__setitem__("schema_version", True),
            ),
            (
                "version",
                lambda value: value["runtime"].__setitem__("version", "latest"),
            ),
            (
                "reference",
                lambda value: value["image"].__setitem__(
                    "reference",
                    f"UPPER/build@sha256:{'3' * 64}",
                ),
            ),
            (
                "artifact-path",
                lambda value: value["probe"].__setitem__("path", "/bad//probe"),
            ),
            (
                "conmon-path",
                lambda value: value["conmon"].__setitem__(
                    "path",
                    "/usr/local/bin/conmon",
                ),
            ),
        ):
            invalid = copy.deepcopy(identity)
            mutate(invalid)  # type: ignore[arg-type]
            with (
                self.subTest(field=field),
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                linux_oci.validate_identity(invalid, self.schema)
            self.assertEqual(
                raised.exception.code,
                "LINUX_OCI_IDENTITY_SCHEMA_INVALID",
            )

        mismatch = copy.deepcopy(identity)
        mismatch["image"]["manifest_sha256"] = "8" * 64  # type: ignore[index]
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci.validate_identity(mismatch, self.schema)
        self.assertEqual(raised.exception.code, "LINUX_OCI_IMAGE_IDENTITY_MISMATCH")

        collision = copy.deepcopy(identity)
        collision["probe"]["path"] = collision["shim"]["path"]  # type: ignore[index]
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci.validate_identity(collision, self.schema)
        self.assertEqual(
            raised.exception.code,
            "LINUX_OCI_IMAGE_ARTIFACT_COLLISION",
        )

    def test_identity_loader_hashes_exact_bytes_and_rejects_invalid_input(self) -> None:
        raw = json.dumps(
            backend_identity(),
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "identity.json"
            path.write_bytes(raw)
            loaded, digest = linux_oci.load_identity(path)
            self.assertEqual(loaded, backend_identity())
            self.assertEqual(digest, hashlib.sha256(raw).hexdigest())

            path.write_bytes(b"[]")
            with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
                linux_oci.load_identity(path)
            self.assertEqual(
                raised.exception.code,
                "LINUX_OCI_IDENTITY_PARSE_FAILED",
            )

            path.write_bytes(b'{"schema_version":1}')
            with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
                linux_oci.load_identity(path)
            self.assertEqual(
                raised.exception.code,
                "LINUX_OCI_IDENTITY_SCHEMA_INVALID",
            )

            path.write_bytes(b'{"schema_version":1,"schema_version":1}')
            with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
                linux_oci.load_identity(path)
            self.assertEqual(
                raised.exception.code,
                "LINUX_OCI_IDENTITY_PARSE_FAILED",
            )

        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci.load_identity(Path("definitely-missing-identity.json"))
        self.assertEqual(raised.exception.code, "LINUX_OCI_IDENTITY_READ_FAILED")

    def test_backend_has_no_process_binary_or_state_authority(self) -> None:
        for forbidden in (
            "subprocess",
            "runtime_environment",
            "_prepare_state_root",
            "_binary_digest",
            "_run_command",
        ):
            self.assertFalse(hasattr(linux_oci, forbidden))
        signature = inspect.signature(linux_oci.preflight_brokered)
        self.assertIs(signature.parameters["runtime_info"].default, inspect.Parameter.empty)
        self.assertIs(
            signature.parameters["image_inspect"].default,
            inspect.Parameter.empty,
        )

    def test_image_preflight_requires_exact_local_identity_and_no_volumes(self) -> None:
        identity = backend_identity()
        linux_oci.validate_image_info(image_info(), identity)

        mutations = [
            ("Id", "9" * 64, "LINUX_OCI_IMAGE_CONFIG_MISMATCH"),
            ("Digest", f"sha256:{'9' * 64}", "LINUX_OCI_IMAGE_MANIFEST_MISMATCH"),
            ("Os", "windows", "LINUX_OCI_IMAGE_PLATFORM_MISMATCH"),
            ("Architecture", "arm64", "LINUX_OCI_IMAGE_PLATFORM_MISMATCH"),
        ]
        for field, value, code in mutations:
            mutated = copy.deepcopy(image_info())
            mutated[0][field] = value
            with (
                self.subTest(code=code),
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                linux_oci.validate_image_info(mutated, identity)
            self.assertEqual(raised.exception.code, code)

        volumes = copy.deepcopy(image_info())
        volumes[0]["Config"]["Volumes"] = {"/data": {}}  # type: ignore[index]
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci.validate_image_info(volumes, identity)
        self.assertEqual(raised.exception.code, "LINUX_OCI_IMAGE_VOLUMES_FORBIDDEN")

        missing = []
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci.validate_image_info(missing, identity)
        self.assertEqual(raised.exception.code, "LINUX_OCI_IMAGE_UNAVAILABLE")

        reference = copy.deepcopy(image_info())
        reference[0]["RepoDigests"] = []
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci.validate_image_info(reference, identity)
        self.assertEqual(raised.exception.code, "LINUX_OCI_IMAGE_REFERENCE_MISMATCH")

    def test_runtime_response_rejects_duplicate_security_keys(self) -> None:
        response = linux_oci.CommandResult(
            returncode=0,
            stdout=b'{"host":{"security":{"rootless":true,"rootless":false}}}',
            stderr=b"",
        )
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci._runtime_json(response)
        self.assertEqual(
            raised.exception.code,
            "LINUX_OCI_RUNTIME_RESPONSE_INVALID",
        )

        for result, code in (
            (
                linux_oci.CommandResult(returncode=1, stdout=b"", stderr=b"private"),
                "LINUX_OCI_RUNTIME_UNAVAILABLE",
            ),
            (
                linux_oci.CommandResult(returncode=0, stdout=b"{", stderr=b""),
                "LINUX_OCI_RUNTIME_RESPONSE_INVALID",
            ),
            (
                linux_oci.CommandResult(
                    returncode=0,
                    stdout=b"[" * 2_000 + b"]" * 2_000,
                    stderr=b"",
                ),
                "LINUX_OCI_RUNTIME_RESPONSE_INVALID",
            ),
            (
                linux_oci.CommandResult(
                    returncode=0,
                    stdout=b"x" * (linux_oci.MAX_RUNTIME_OUTPUT_BYTES + 1),
                    stderr=b"",
                ),
                "LINUX_OCI_RUNTIME_OUTPUT_LIMIT",
            ),
        ):
            with (
                self.subTest(code=code),
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                linux_oci._runtime_json(result)
            self.assertEqual(raised.exception.code, code)

    def test_preflight_requires_both_broker_results(self) -> None:
        with self.assertRaises(TypeError):
            linux_oci.preflight_brokered(  # type: ignore[call-arg]
                backend_identity(),
                runtime_info=linux_oci.CommandResult(0, b"{}", b""),
            )

    def test_brokered_preflight_consumes_closed_local_version_result(self) -> None:
        summary = linux_oci.preflight_brokered(
            backend_identity(),
            runtime_info=linux_oci.CommandResult(
                0,
                json.dumps(runtime_version()).encode(),
                b"",
            ),
            image_inspect=linux_oci.CommandResult(
                0,
                json.dumps(image_info()).encode(),
                b"",
            ),
            platform_name="linux",
            effective_uid=1000,
        )
        self.assertEqual(summary["status"], "available")
        client = runtime_version()["Client"]
        assert isinstance(client, dict)
        for changed, code in (
            (
                {"Client": {**client, "Version": "9.9.9"}},
                "LINUX_OCI_RUNTIME_VERSION_MISMATCH",
            ),
            (
                {"Client": client, "Server": {}},
                "LINUX_OCI_REMOTE_RUNTIME",
            ),
            (
                {"Client": {**client, "Os": "windows"}},
                "LINUX_OCI_HOST_PLATFORM_MISMATCH",
            ),
            (
                {"Client": {**client, "OsArch": "linux/arm64"}},
                "LINUX_OCI_HOST_PLATFORM_MISMATCH",
            ),
        ):
            with (
                self.subTest(code=code),
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                linux_oci.preflight_brokered(
                    backend_identity(),
                    runtime_info=linux_oci.CommandResult(
                        0,
                        json.dumps(changed).encode(),
                        b"",
                    ),
                    image_inspect=linux_oci.CommandResult(
                        0,
                        json.dumps(image_info()).encode(),
                        b"",
                    ),
                    platform_name="linux",
                    effective_uid=1000,
                )
            self.assertEqual(raised.exception.code, code)

    def test_probe_command_is_fixed_direct_argv_with_one_bounded_tmpfs(self) -> None:
        identity = backend_identity()
        limits = {
            "wall_time_ms": 1000,
            "output_bytes": 4096,
            "workspace_bytes": 8192,
            "process_count": 3,
            "memory_mib": 64,
            "cpu_time_ms": 1000,
        }
        with tempfile.TemporaryDirectory() as directory:
            state_root = Path(directory)
            command = linux_oci.build_probe_create_argv(
                identity,
                limits=limits,
                state_root=state_root,
                cidfile=state_root / "container.cid",
                seccomp_profile=state_root / "seccomp.json",
            )

        expected_image = f"sha256:{'4' * 64}"
        self.assertEqual(command[0], "/usr/bin/podman")
        self.assertIn("--pull=never", command)
        self.assertIn("--userns=nomap", command)
        self.assertIn("--network=none", command)
        self.assertIn("--pid=private", command)
        self.assertIn("--ipc=none", command)
        self.assertIn("--uts=private", command)
        self.assertIn("--cgroupns=private", command)
        self.assertIn("--read-only=true", command)
        self.assertIn("--read-only-tmpfs=false", command)
        self.assertIn("--image-volume=ignore", command)
        self.assertIn("--cap-drop=ALL", command)
        self.assertIn("--security-opt=no-new-privileges=true", command)
        self.assertIn("--cgroup-conf=memory.swap.max=0", command)
        self.assertIn(
            "--tmpfs=/sandbox:rw,nosuid,nodev,noexec,size=8192,mode=0700", command
        )
        self.assertIn("--log-driver=none", command)
        self.assertIn("--unsetenv-all", command)
        self.assertEqual(command[-1], expected_image)
        self.assertNotIn(identity["image"]["reference"], command)  # type: ignore[index]
        self.assertFalse(any(value == "--mount" for value in command))
        self.assertNotIn("--passwd=false", command)
        self.assertFalse(any("fixture" in value for value in command))

    def test_zero_workspace_or_output_fails_closed(self) -> None:
        identity = backend_identity()
        base_limits = {
            "wall_time_ms": 1000,
            "output_bytes": 1,
            "workspace_bytes": 1,
            "process_count": 1,
            "memory_mib": 64,
            "cpu_time_ms": 1000,
        }
        for field, code in (
            ("workspace_bytes", "LINUX_OCI_ZERO_WORKSPACE_UNSUPPORTED"),
            ("output_bytes", "LINUX_OCI_ZERO_OUTPUT_UNSUPPORTED"),
        ):
            limits = dict(base_limits)
            limits[field] = 0
            with (
                self.subTest(field=field),
                tempfile.TemporaryDirectory() as directory,
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                root = Path(directory)
                linux_oci.build_probe_create_argv(
                    identity,
                    limits=limits,
                    state_root=root,
                    cidfile=root / "container.cid",
                    seccomp_profile=root / "seccomp.json",
                )
            self.assertEqual(raised.exception.code, code)

        for field in ("process_count", "memory_mib"):
            limits = dict(base_limits)
            limits[field] = 0
            with (
                self.subTest(field=field),
                tempfile.TemporaryDirectory() as directory,
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                root = Path(directory)
                linux_oci.build_probe_create_argv(
                    identity,
                    limits=limits,
                    state_root=root,
                    cidfile=root / "container.cid",
                    seccomp_profile=root / "seccomp.json",
                )
            self.assertEqual(raised.exception.code, "LINUX_OCI_LIMIT_INVALID")

        with (
            tempfile.TemporaryDirectory() as directory,
            self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
        ):
            root = Path(directory)
            linux_oci.build_probe_create_argv(
                identity,
                limits=base_limits,
                state_root=root,
                cidfile=root / "container.cid",
                seccomp_profile=root.parent / "outside-seccomp.json",
            )
        self.assertEqual(raised.exception.code, "LINUX_OCI_RUNNER_PATH_INVALID")

    def test_preflight_consumes_only_bounded_broker_results(self) -> None:
        identity = backend_identity()
        summary = linux_oci.preflight(
            identity,
            runtime_info=linux_oci.CommandResult(
                returncode=0,
                stdout=json.dumps(runtime_version()).encode("utf-8"),
                stderr=b"",
            ),
            image_inspect=linux_oci.CommandResult(
                returncode=0,
                stdout=json.dumps(image_info()).encode("utf-8"),
                stderr=b"",
            ),
            platform_name="linux",
            effective_uid=1000,
        )

        self.assertEqual(summary["status"], "available")

    def test_preflight_rejects_platform_and_root_before_decoding_results(
        self,
    ) -> None:
        identity = backend_identity()
        poison = linux_oci.CommandResult(0, b"{", b"")
        for platform_name, uid, code in (
            ("win32", 1000, "LINUX_OCI_PLATFORM_UNSUPPORTED"),
            ("linux", 0, "LINUX_OCI_ROOT_USER_FORBIDDEN"),
        ):
            with (
                self.subTest(code=code),
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                linux_oci.preflight(
                    identity,
                    runtime_info=poison,
                    image_inspect=poison,
                    platform_name=platform_name,
                    effective_uid=uid,
                )
            self.assertEqual(raised.exception.code, code)

    def test_direct_cli_requires_external_authority_before_identity_read(self) -> None:
        stdout = io.StringIO()
        with (
            mock.patch.object(linux_oci, "load_identity") as identity_reader,
            mock.patch.object(linux_oci, "preflight") as preflight,
            redirect_stdout(stdout),
        ):
            exit_code = linux_oci.main(["--identity", "identity.json"])
        self.assertEqual(exit_code, 1)
        output = json.loads(stdout.getvalue())
        self.assertEqual(
            output["diagnostics"][0]["code"],
            "LINUX_OCI_AUTHORITY_REQUIRED",
        )
        identity_reader.assert_not_called()
        preflight.assert_not_called()

    def test_identity_digest_is_raw_byte_stable(self) -> None:
        raw = json.dumps(
            backend_identity(),
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        self.assertEqual(
            linux_oci.identity_sha256(raw),
            hashlib.sha256(raw).hexdigest(),
        )


if __name__ == "__main__":
    unittest.main()
