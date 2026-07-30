from __future__ import annotations

import copy
import hashlib
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
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


def host_info() -> dict[str, object]:
    return {
        "host": {
            "arch": "amd64",
            "os": "linux",
            "cgroupManager": "systemd",
            "cgroupVersion": "v2",
            "cgroupControllers": ["cpu", "io", "memory", "pids"],
            "serviceIsRemote": False,
            "ociRuntime": {
                "name": "crun",
                "path": "/usr/bin/crun",
            },
            "security": {
                "rootless": True,
                "seccompEnabled": True,
            },
        },
        "version": {
            "Version": "4.9.3",
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

    def test_fixed_environment_drops_host_authority(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            state_root = Path(directory)
            environment = linux_oci.runtime_environment(state_root)
        self.assertEqual(
            environment,
            {
                "HOME": str(state_root / "home"),
                "LANG": "C.UTF-8",
                "LC_ALL": "C.UTF-8",
                "PATH": "/usr/sbin:/usr/bin:/sbin:/bin",
                "TZ": "UTC",
                "XDG_CONFIG_HOME": str(state_root / "config"),
                "XDG_RUNTIME_DIR": str(state_root / "runtime"),
            },
        )
        for forbidden in (
            "CONTAINER_HOST",
            "DOCKER_HOST",
            "GITHUB_TOKEN",
            "HTTP_PROXY",
            "SSH_AUTH_SOCK",
        ):
            self.assertNotIn(forbidden, environment)

    def test_state_root_must_be_runner_owned_and_absolute(self) -> None:
        with self.assertRaises(linux_oci.LinuxOciUnavailable) as raised:
            linux_oci._prepare_state_root(Path("relative-state"))
        self.assertEqual(raised.exception.code, "LINUX_OCI_STATE_ROOT_INVALID")

    def test_host_preflight_requires_every_containment_primitive(self) -> None:
        identity = backend_identity()
        linux_oci.validate_host_info(host_info(), identity)

        mutations = [
            (("host", "serviceIsRemote"), True, "LINUX_OCI_REMOTE_RUNTIME"),
            (("host", "security", "rootless"), False, "LINUX_OCI_ROOTFUL_RUNTIME"),
            (("host", "cgroupVersion"), "v1", "LINUX_OCI_CGROUP_V2_REQUIRED"),
            (
                ("host", "cgroupControllers"),
                ["memory", "pids"],
                "LINUX_OCI_CGROUP_CONTROLLERS_MISSING",
            ),
            (
                ("host", "ociRuntime", "name"),
                "runc",
                "LINUX_OCI_CRUN_REQUIRED",
            ),
            (
                ("host", "security", "seccompEnabled"),
                False,
                "LINUX_OCI_SECCOMP_REQUIRED",
            ),
            (
                ("host", "cgroupManager"),
                "cgroupfs",
                "LINUX_OCI_CGROUP_MANAGER_UNSUPPORTED",
            ),
            (
                ("host", "arch"),
                "arm64",
                "LINUX_OCI_HOST_PLATFORM_MISMATCH",
            ),
            (
                ("version", "Version"),
                "5.8.3",
                "LINUX_OCI_RUNTIME_VERSION_MISMATCH",
            ),
        ]
        for path, value, code in mutations:
            mutated = copy.deepcopy(host_info())
            cursor = mutated
            for part in path[:-1]:
                cursor = cursor[part]  # type: ignore[assignment,index]
            cursor[path[-1]] = value  # type: ignore[index]
            with (
                self.subTest(code=code),
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                linux_oci.validate_host_info(mutated, identity)
            self.assertEqual(raised.exception.code, code)

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

    def test_direct_runtime_wrapper_bounds_output_and_redacts_failures(self) -> None:
        completed = SimpleNamespace(returncode=0, stdout=b"{}", stderr=b"")
        with mock.patch.object(linux_oci.subprocess, "run", return_value=completed) as run:
            result = linux_oci._run_command(
                ["/usr/bin/podman", "info"],
                {"HOME": "/private"},
                1.0,
            )
        self.assertEqual(result.stdout, b"{}")
        self.assertFalse(run.call_args.kwargs["shell"])
        self.assertEqual(run.call_args.args[0], ["/usr/bin/podman", "info"])

        oversized = SimpleNamespace(
            returncode=0,
            stdout=b"x" * (linux_oci.MAX_RUNTIME_OUTPUT_BYTES + 1),
            stderr=b"",
        )
        with (
            mock.patch.object(linux_oci.subprocess, "run", return_value=oversized),
            self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
        ):
            linux_oci._run_command(
                ["/usr/bin/podman", "info"],
                {"HOME": "/private"},
                1.0,
            )
        self.assertEqual(raised.exception.code, "LINUX_OCI_RUNTIME_OUTPUT_LIMIT")

        with (
            mock.patch.object(
                linux_oci.subprocess,
                "run",
                side_effect=OSError("secret host detail"),
            ),
            self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
        ):
            linux_oci._run_command(
                ["/usr/bin/podman", "info"],
                {"HOME": "/private"},
                1.0,
            )
        self.assertEqual(raised.exception.code, "LINUX_OCI_RUNTIME_UNAVAILABLE")
        self.assertNotIn("secret", raised.exception.message)

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
        self.assertIn("--tmpfs=/sandbox:rw,nosuid,nodev,noexec,size=8192,mode=0700", command)
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

    def test_preflight_uses_only_info_and_exact_image_inspect(self) -> None:
        identity = backend_identity()
        calls: list[tuple[list[str], dict[str, str], float]] = []

        def fake_runner(
            argv: list[str],
            environment: dict[str, str],
            timeout_seconds: float,
        ) -> linux_oci.CommandResult:
            calls.append((argv, environment, timeout_seconds))
            payload: object
            if "info" in argv:
                payload = host_info()
            else:
                payload = image_info()
            return linux_oci.CommandResult(
                returncode=0,
                stdout=json.dumps(payload).encode("utf-8"),
                stderr=b"",
            )

        digests = {
            Path("/usr/bin/podman"): "1" * 64,
            Path("/usr/bin/crun"): "2" * 64,
        }
        with tempfile.TemporaryDirectory() as directory:
            summary = linux_oci.preflight(
                identity,
                state_root=Path(directory),
                command_runner=fake_runner,
                binary_digest=lambda path: digests[path],
                platform_name="linux",
                effective_uid=1000,
            )

        self.assertEqual(summary["status"], "available")
        self.assertEqual(len(calls), 2)
        self.assertEqual(calls[0][0][-3:], ["info", "--format", "json"])
        self.assertEqual(
            calls[1][0][-5:],
            ["image", "inspect", "--format", "json", f"sha256:{'4' * 64}"],
        )
        self.assertTrue(all("--pull" not in argument for call in calls for argument in call[0]))
        self.assertTrue(all("create" not in call[0] for call in calls))

    def test_preflight_rejects_platform_root_and_binary_mismatch_before_spawn(
        self,
    ) -> None:
        identity = backend_identity()
        for platform_name, uid, digest, code in (
            ("win32", 1000, "1" * 64, "LINUX_OCI_PLATFORM_UNSUPPORTED"),
            ("linux", 0, "1" * 64, "LINUX_OCI_ROOT_USER_FORBIDDEN"),
            ("linux", 1000, "9" * 64, "LINUX_OCI_RUNTIME_IDENTITY_MISMATCH"),
        ):
            calls = 0

            def forbidden_runner(
                argv: list[str],
                environment: dict[str, str],
                timeout_seconds: float,
            ) -> linux_oci.CommandResult:
                del argv, environment, timeout_seconds
                nonlocal calls
                calls += 1
                raise AssertionError("runtime must not be invoked")

            with (
                self.subTest(code=code),
                tempfile.TemporaryDirectory() as directory,
                self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
            ):
                linux_oci.preflight(
                    identity,
                    state_root=Path(directory),
                    command_runner=forbidden_runner,
                    binary_digest=lambda _path, selected=digest: selected,
                    platform_name=platform_name,
                    effective_uid=uid,
                )
            self.assertEqual(raised.exception.code, code)
            self.assertEqual(calls, 0)

        digests = {
            Path("/usr/bin/podman"): "1" * 64,
            Path("/usr/bin/crun"): "9" * 64,
        }
        with (
            tempfile.TemporaryDirectory() as directory,
            self.assertRaises(linux_oci.LinuxOciUnavailable) as raised,
        ):
            linux_oci.preflight(
                identity,
                state_root=Path(directory),
                command_runner=forbidden_runner,
                binary_digest=lambda path: digests[path],
                platform_name="linux",
                effective_uid=1000,
            )
        self.assertEqual(raised.exception.code, "LINUX_OCI_CRUN_IDENTITY_MISMATCH")

    def test_cli_reports_stable_available_and_unavailable_results(self) -> None:
        identity = backend_identity()
        available = {
            "schema_version": 1,
            "backend_kind": "linux_oci",
            "status": "available",
            "conformance_status": "not-run",
        }
        stdout = io.StringIO()
        with (
            mock.patch.object(
                linux_oci,
                "load_identity",
                return_value=(identity, "a" * 64),
            ),
            mock.patch.object(linux_oci, "preflight", return_value=available),
            redirect_stdout(stdout),
        ):
            exit_code = linux_oci.main(["--identity", "identity.json"])
        self.assertEqual(exit_code, 0)
        output = json.loads(stdout.getvalue())
        self.assertEqual(output["identity_sha256"], "a" * 64)

        stdout = io.StringIO()
        unavailable = linux_oci.LinuxOciUnavailable(
            "LINUX_OCI_PLATFORM_UNSUPPORTED",
            "Linux OCI backend preflight requires Linux",
        )
        with (
            mock.patch.object(
                linux_oci,
                "load_identity",
                side_effect=unavailable,
            ),
            redirect_stdout(stdout),
        ):
            exit_code = linux_oci.main(["--identity", "identity.json"])
        self.assertEqual(exit_code, 1)
        output = json.loads(stdout.getvalue())
        self.assertEqual(
            output["diagnostics"][0]["code"],
            "LINUX_OCI_PLATFORM_UNSUPPORTED",
        )

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
