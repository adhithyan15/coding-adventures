from __future__ import annotations

import ctypes
import errno
import hashlib
import json
import os
import stat
import struct
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPT_ROOT.parents[1]
FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1"
sys.path.insert(0, str(SCRIPT_ROOT))

import build_tool_conformance as bootstrap
import build_tool_conformance_authority as authority
import build_tool_conformance_capability_broker as broker

SOURCE_COMMIT = "a" * 40
SOURCE_TREE = "b" * 40
ZERO_SHA256 = "0" * 64


def valid_identity() -> dict[str, object]:
    manifest = "1" * 64
    return {
        "schema_version": 1,
        "backend_kind": "linux_oci",
        "platform": "linux",
        "architecture": "amd64",
        "runtime": {
            "implementation": "podman",
            "path": "/usr/bin/podman",
            "version": "5.1.2",
            "sha256": "2" * 64,
        },
        "oci_runtime": {
            "implementation": "crun",
            "path": "/usr/bin/crun",
            "sha256": "3" * 64,
        },
        "conmon": {
            "implementation": "conmon",
            "path": "/usr/bin/conmon",
            "sha256": "8" * 64,
        },
        "image": {
            "reference": f"example.invalid/build-tool@sha256:{manifest}",
            "manifest_sha256": manifest,
            "config_sha256": "4" * 64,
            "os": "linux",
            "architecture": "amd64",
        },
        "seccomp_profile_sha256": "5" * 64,
        "shim": {
            "path": "/opt/conformance/shim",
            "sha256": "6" * 64,
        },
        "probe": {
            "path": "/opt/conformance/probe",
            "sha256": "7" * 64,
        },
    }


def _component_record(provenance: str, path: str, raw: bytes) -> dict[str, object]:
    return {
        "provenance": provenance,
        "path": path,
        "byte_length": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def write_broker_authority_fixture(root: Path) -> dict[str, object]:
    repository = root / "repository"
    bundle_root = root / "authority"
    repository.mkdir()
    bundle_root.mkdir()
    repository_sources: dict[str, bytes] = {}
    copied = {
        "authority_bundle_schema": (
            FIXTURE_ROOT / "execution-capability-broker-authority.schema.json"
        ),
        "execution_policy_schema": FIXTURE_ROOT / "execution-policy.schema.json",
        "execution_policy": FIXTURE_ROOT / "execution-policy.json",
        "linux_backend_identity_schema": FIXTURE_ROOT / "linux-oci-backend.schema.json",
        "preflight_import_manifest": (
            FIXTURE_ROOT / "preflight-broker-backend-imports.json"
        ),
        "capability_broker_schema": (
            FIXTURE_ROOT / "linux-capability-preflight-broker.schema.json"
        ),
        "capability_broker_manifest": (
            FIXTURE_ROOT / "linux-capability-preflight-broker.json"
        ),
    }
    placeholders = {
        "bootstrap_runner": b"# process-free bootstrap\n",
        "authority_verifier": b"# process-free authority verifier\n",
        "preflight_loader": b"# exact loader\n",
        "linux_preflight_backend": b"# process-free broker consumer\n",
        "capability_broker": b"# protected capability broker\n",
    }
    for role, relative in authority.BROKER_REPOSITORY_COMPONENT_PATHS.items():
        raw = (
            copied[role].read_bytes()
            if role in copied
            else placeholders[role]
        )
        target = repository / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
        repository_sources[role] = raw

    identity_raw = json.dumps(
        valid_identity(),
        sort_keys=True,
        separators=(",", ":"),
    ).encode()
    identity_path = bundle_root / authority.BUNDLE_COMPONENT_PATHS[
        "linux_backend_identity"
    ]
    identity_path.write_bytes(identity_raw)
    components = {
        role: _component_record(
            "repository",
            authority.BROKER_REPOSITORY_COMPONENT_PATHS[role],
            repository_sources[role],
        )
        for role in authority.BROKER_REPOSITORY_COMPONENT_PATHS
    }
    components["linux_backend_identity"] = _component_record(
        "bundle",
        authority.BUNDLE_COMPONENT_PATHS["linux_backend_identity"],
        identity_raw,
    )
    bundle = {
        "schema_version": 1,
        "purpose": "build-tool-trusted-authority",
        "authorization_scope": "linux_capability_preflight_broker_v1",
        "repository": "github.com/adhithyan15/coding-adventures",
        "conformance_revision": "v1",
        "platform": "linux",
        "architecture": "amd64",
        "source": {
            "git_object_format": "sha1",
            "commit_oid": SOURCE_COMMIT,
            "tree_oid": SOURCE_TREE,
        },
        "components": components,
    }
    raw_bundle = json.dumps(bundle, sort_keys=True, separators=(",", ":")).encode()
    bundle_path = bundle_root / "authority.json"
    bundle_path.write_bytes(raw_bundle)
    return {
        "repository": repository,
        "bundle": bundle,
        "bundle_path": bundle_path,
        "digest": authority.broker_authority_bundle_sha256(raw_bundle),
        "raw": raw_bundle,
    }


class CapabilityBrokerAuthorityTests(unittest.TestCase):
    def test_authority_schema_is_closed_and_domain_separated(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_broker_authority_fixture(Path(directory))
            schema = bootstrap.strict_load_bytes(
                (
                    FIXTURE_ROOT
                    / "execution-capability-broker-authority.schema.json"
                ).read_bytes()
            )
            bootstrap._validate_schema(
                fixture["bundle"],
                schema,
                "TEST_SCHEMA_INVALID",
            )
            raw = fixture["raw"]
            manual = hashlib.sha256(
                authority.BROKER_AUTHORITY_DOMAIN
                + struct.pack(">Q", len(raw))
                + raw
            ).hexdigest()
            self.assertEqual(
                authority.broker_authority_bundle_sha256(raw),
                manual,
            )
            self.assertNotEqual(manual, authority.loader_authority_bundle_sha256(raw))
            self.assertNotEqual(manual, authority.authority_bundle_sha256(raw))

    def test_authority_retains_exact_thirteen_role_profile(self) -> None:
        if os.name != "posix":
            self.skipTest("handle-relative authority traversal is Linux-only")
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_broker_authority_fixture(Path(directory))
            approved = authority.authorize_capability_broker(
                fixture["bundle_path"],
                approved_digest=fixture["digest"],
                expected_commit_oid=SOURCE_COMMIT,
                expected_tree_oid=SOURCE_TREE,
                repository_root=fixture["repository"],
            )
        self.assertEqual(
            set(approved.components),
            set(authority.BROKER_COMPONENT_ROLES),
        )
        self.assertEqual(
            approved.behavior["profile"],
            "linux_capability_preflight_broker_v1",
        )
        self.assertEqual(
            approved.import_manifest["required_exports"],
            [
                "CommandResult",
                "LinuxOciUnavailable",
                "preflight_brokered",
                "preflight_prevalidated",
            ],
        )

    def test_role_swap_is_rejected_before_component_read(self) -> None:
        if os.name != "posix":
            self.skipTest("handle-relative authority traversal is Linux-only")
        with tempfile.TemporaryDirectory() as directory:
            fixture = write_broker_authority_fixture(Path(directory))
            bundle = fixture["bundle"]
            components = bundle["components"]
            components["capability_broker"]["path"] = components[
                "linux_preflight_backend"
            ]["path"]
            raw = json.dumps(bundle, sort_keys=True, separators=(",", ":")).encode()
            fixture["bundle_path"].write_bytes(raw)
            with self.assertRaises(bootstrap.ConformanceError) as caught:
                authority.authorize_capability_broker(
                    fixture["bundle_path"],
                    approved_digest=authority.broker_authority_bundle_sha256(raw),
                    expected_commit_oid=SOURCE_COMMIT,
                    expected_tree_oid=SOURCE_TREE,
                    repository_root=fixture["repository"],
                )
        self.assertEqual(caught.exception.code, "BROKER_AUTHORITY_COMPONENT_ROLE_INVALID")


class CapabilityBrokerManifestTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.raw = (
            FIXTURE_ROOT / "linux-capability-preflight-broker.json"
        ).read_bytes()
        cls.schema_raw = (
            FIXTURE_ROOT / "linux-capability-preflight-broker.schema.json"
        ).read_bytes()
        cls.manifest = broker.parse_behavior_manifest(cls.raw, cls.schema_raw)

    def test_manifest_closes_two_operations_and_limits(self) -> None:
        self.assertEqual(
            [item["id"] for item in self.manifest["operations"]],
            ["runtime_version", "image_inspect"],
        )
        execution = self.manifest["execution"]
        self.assertEqual(execution["timeout_ms"], 15_000)
        self.assertEqual(execution["combined_output_bytes"], 262_144)
        self.assertEqual(
            execution["ambient_exec"],
            {
                "kind": "landlock_path_beneath",
                "minimum_abi": 1,
                "handled_access": "execute",
                "coverage": "pathname_backed_executables",
                "allowed_role": "runtime",
                "default": "deny",
            },
        )
        self.assertEqual(execution["cleanup"]["cleanup_timeout_ms"], 5_000)

    def test_any_behavior_mutation_fails_exact_schema(self) -> None:
        changed = json.loads(self.raw)
        changed["execution"]["timeout_ms"] = 15_001
        with self.assertRaises(broker.BrokerUnavailable) as caught:
            broker.parse_behavior_manifest(
                json.dumps(changed, separators=(",", ":")).encode(),
                self.schema_raw,
            )
        self.assertEqual(caught.exception.code, "BROKER_BEHAVIOR_SCHEMA_INVALID")

    def test_rendered_operations_use_only_retained_descriptors(self) -> None:
        state = {
            "config": 20,
            "home": 21,
            "runtime": 22,
            "runroot": 23,
            "storage": 24,
        }
        rendered = broker.render_operations(
            self.manifest,
            valid_identity(),
            runtime_fd=10,
            oci_runtime_fd=11,
            state_fds=state,
        )
        info, inspect = rendered
        storage_common = [
            "/usr/bin/podman",
            "--remote=false",
            "--root",
            "/proc/self/fd/24",
            "--runroot",
            "/proc/self/fd/23",
            "--runtime",
            "/usr/bin/crun",
            "--conmon",
            "/usr/bin/conmon",
            "--storage-driver",
            "vfs",
        ]
        self.assertEqual(
            info.argv,
            [
                "/usr/bin/podman",
                "--remote=false",
                "version",
                "--format",
                "json",
            ],
        )
        self.assertEqual(
            inspect.argv,
            [
                *storage_common,
                "image",
                "inspect",
                "--format",
                "json",
                f"sha256:{valid_identity()['image']['config_sha256']}",
            ],
        )
        self.assertEqual(info.executable_fd, 10)
        self.assertEqual(info.cwd_fd, 21)
        self.assertEqual(
            info.environment,
            {
                "HOME": "/proc/self/fd/21",
                "LANG": "C.UTF-8",
                "LC_ALL": "C.UTF-8",
                "PATH": "/nonexistent",
                "TZ": "UTC",
                "XDG_CONFIG_HOME": "/proc/self/fd/20",
                "XDG_RUNTIME_DIR": "/proc/self/fd/22",
            },
        )
        self.assertNotIn(str(REPO_ROOT), " ".join(info.argv))

    def test_unknown_operation_or_missing_descriptor_fails_closed(self) -> None:
        changed = json.loads(self.raw)
        changed["operations"][0]["id"] = "runtime_shell"
        with self.assertRaises(broker.BrokerUnavailable):
            broker.render_operations(
                changed,
                valid_identity(),
                runtime_fd=10,
                oci_runtime_fd=11,
                state_fds={},
            )

    def test_matching_mutated_schema_cannot_expand_command_grammar(self) -> None:
        changed = json.loads(self.raw)
        changed["operations"][0]["argv"] = [
            {"kind": "identity_ref", "path": "runtime.path"},
            {"kind": "literal", "value": "run"},
            {"kind": "literal", "value": "attacker/image"},
        ]
        schema = json.loads(self.schema_raw)
        schema["const"] = changed
        parsed = broker.parse_behavior_manifest(
            json.dumps(changed, separators=(",", ":")).encode(),
            json.dumps(schema, separators=(",", ":")).encode(),
        )
        with self.assertRaises(broker.BrokerUnavailable) as caught:
            broker.render_operations(
                parsed,
                valid_identity(),
                runtime_fd=10,
                oci_runtime_fd=11,
                state_fds={
                    "config": 20,
                    "home": 21,
                    "runtime": 22,
                    "runroot": 23,
                    "storage": 24,
                },
            )
        self.assertEqual(caught.exception.code, "BROKER_BEHAVIOR_INVALID")

    def test_worker_protocol_cannot_accept_backend_or_claim_authority(self) -> None:
        destinations = {
            action.dest
            for action in broker._worker_parser()._actions
        }
        self.assertEqual(
            set(broker.WORKER_COMPONENTS),
            {"broker", "identity", "behavior_schema", "behavior"},
        )
        self.assertNotIn("backend_fd", destinations)
        self.assertNotIn("loader_fd", destinations)
        self.assertNotIn("authority_sha256", destinations)
        self.assertNotIn("source_commit", destinations)

    def test_exact_sealed_loader_and_backend_modules_execute(self) -> None:
        backend = broker._load_exact_backend(
            (SCRIPT_ROOT / "build_tool_conformance_backend_loader.py").read_bytes(),
            (SCRIPT_ROOT / "build_tool_conformance_linux_oci.py").read_bytes(),
            (
                FIXTURE_ROOT / "preflight-broker-backend-imports.json"
            ).read_bytes(),
        )
        self.assertIn("CommandResult", backend)
        self.assertIn("LinuxOciUnavailable", backend)
        self.assertIn("preflight_brokered", backend)
        self.assertNotIn("subprocess", backend)
        identity = valid_identity()
        image = identity["image"]
        assert isinstance(image, dict)
        result = backend["CommandResult"]
        summary = backend["preflight_brokered"](
            identity,
            runtime_info=result(
                0,
                json.dumps(
                    {
                        "Client": {
                            "Version": "5.1.2",
                            "Os": "linux",
                            "OsArch": "linux/amd64",
                        }
                    }
                ).encode(),
                b"",
            ),
            image_inspect=result(
                0,
                json.dumps(
                    [
                        {
                            "Id": image["config_sha256"],
                            "Digest": f"sha256:{image['manifest_sha256']}",
                            "RepoDigests": [image["reference"]],
                            "Os": "linux",
                            "Architecture": "amd64",
                            "Config": {"Volumes": None},
                        }
                    ]
                ).encode(),
                b"",
            ),
            platform_name="linux",
            effective_uid=1000,
        )
        self.assertEqual(summary["status"], "available")


class CapabilityBrokerHelperTests(unittest.TestCase):
    def test_binary_policy_requires_immutable_root_owned_executable(self) -> None:
        safe = os.stat_result(
            (
                stat.S_IFREG | 0o755,
                1,
                2,
                1,
                0,
                0,
                1,
                0,
                0,
                0,
            )
        )
        broker.validate_binary_status(safe)
        for mode in (
            stat.S_IFREG | 0o775,
            stat.S_IFREG | stat.S_ISUID | 0o755,
            stat.S_IFDIR | 0o755,
            stat.S_IFREG | 0o644,
        ):
            invalid = os.stat_result(
                (mode, 1, 2, 1, 0, 0, 1, 0, 0, 0)
            )
            with self.assertRaises(broker.BrokerUnavailable):
                broker.validate_binary_status(invalid)

    def test_stable_binary_identity_includes_ctime(self) -> None:
        first = broker.binary_stat_identity(
            type(
                "Status",
                (),
                {
                    "st_dev": 1,
                    "st_ino": 2,
                    "st_size": 3,
                    "st_mtime_ns": 4,
                    "st_ctime_ns": 5,
                    "st_mode": stat.S_IFREG | 0o755,
                    "st_uid": 0,
                    "st_nlink": 1,
                },
            )()
        )
        second = (*first[:-1], first[-1] + 1)
        self.assertNotEqual(first, second)

    def test_landlock_allows_only_the_exact_retained_runtime_inode(self) -> None:
        self.assertEqual(
            ctypes.sizeof(broker._LandlockPathBeneathAttr),
            12,
        )
        calls: list[tuple[int, tuple[object, ...]]] = []

        def syscall(number: int, *arguments: object) -> int:
            calls.append((number, arguments))
            return {
                broker.SYS_LANDLOCK_CREATE_RULESET: (1, 71),
                broker.SYS_LANDLOCK_ADD_RULE: (0,),
                broker.SYS_LANDLOCK_RESTRICT_SELF: (0,),
            }[number][sum(1 for selected, _ in calls if selected == number) - 1]

        with (
            mock.patch.object(broker.sys, "platform", "linux"),
            mock.patch.object(broker, "_linux_syscall", side_effect=syscall),
            mock.patch.object(broker, "_prctl") as prctl,
            mock.patch.object(broker.os, "close") as close,
        ):
            broker.install_execute_landlock(51)

        self.assertEqual(
            [number for number, _arguments in calls],
            [
                broker.SYS_LANDLOCK_CREATE_RULESET,
                broker.SYS_LANDLOCK_CREATE_RULESET,
                broker.SYS_LANDLOCK_ADD_RULE,
                broker.SYS_LANDLOCK_RESTRICT_SELF,
            ],
        )
        path_rule = calls[2][1][2]._obj  # type: ignore[attr-defined]
        self.assertEqual(path_rule.parent_fd, 51)
        self.assertEqual(
            path_rule.allowed_access,
            broker.LANDLOCK_ACCESS_FS_EXECUTE,
        )
        prctl.assert_called_once_with(broker.PR_SET_NO_NEW_PRIVS, 1)
        close.assert_called_once_with(71)

    def test_landlock_is_mandatory(self) -> None:
        with (
            mock.patch.object(broker.sys, "platform", "linux"),
            mock.patch.object(broker, "_linux_syscall", return_value=0),
            self.assertRaises(broker.BrokerUnavailable) as caught,
        ):
            broker.install_execute_landlock(51)
        self.assertEqual(caught.exception.code, "BROKER_EXEC_SANDBOX_UNAVAILABLE")

    def test_landlock_enforces_exact_retained_fd_exec_on_linux(self) -> None:
        if not (
            sys.platform.startswith("linux")
            and hasattr(os, "fork")
            and os.execve in os.supports_fd
        ):
            self.skipTest("Landlock retained-FD execution requires Linux")
        allowed_fd = os.open("/bin/true", os.O_RDONLY | os.O_CLOEXEC)
        denied_fd = os.open("/bin/false", os.O_RDONLY | os.O_CLOEXEC)
        try:
            allowed_pid = os.fork()
            if allowed_pid == 0:
                try:
                    broker.install_execute_landlock(allowed_fd)
                    os.execve(allowed_fd, ["/bin/true"], {"PATH": "/nonexistent"})
                except BaseException:  # noqa: BLE001 - child test process
                    os._exit(125)
            _selected, allowed_status = os.waitpid(allowed_pid, 0)
            self.assertTrue(os.WIFEXITED(allowed_status))
            self.assertEqual(os.WEXITSTATUS(allowed_status), 0)

            pathname_pid = os.fork()
            if pathname_pid == 0:
                try:
                    broker.install_execute_landlock(allowed_fd)
                    os.execve("/bin/true", ["/bin/true"], {"PATH": "/nonexistent"})
                except BaseException:  # noqa: BLE001 - child test process
                    os._exit(124)
            _selected, pathname_status = os.waitpid(pathname_pid, 0)
            self.assertTrue(os.WIFEXITED(pathname_status))
            self.assertEqual(os.WEXITSTATUS(pathname_status), 0)

            denied_pid = os.fork()
            if denied_pid == 0:
                try:
                    broker.install_execute_landlock(allowed_fd)
                    os.execve(denied_fd, ["/bin/false"], {"PATH": "/nonexistent"})
                except OSError as error:
                    os._exit(42 if error.errno == errno.EACCES else 41)
                except BaseException:  # noqa: BLE001 - child test process
                    os._exit(40)
                os._exit(39)
            _selected, denied_status = os.waitpid(denied_pid, 0)
            self.assertTrue(os.WIFEXITED(denied_status))
            self.assertEqual(os.WEXITSTATUS(denied_status), 42)
        finally:
            os.close(allowed_fd)
            os.close(denied_fd)

    def test_verified_binary_is_retained_after_exact_hash(self) -> None:
        if not hasattr(os, "pread"):
            self.skipTest("descriptor-relative binary hashing is POSIX-only")
        raw = b"approved runtime"
        status = type(
            "Status",
            (),
            {
                "st_dev": 1,
                "st_ino": 2,
                "st_size": len(raw),
                "st_mtime_ns": 3,
                "st_ctime_ns": 4,
                "st_mode": stat.S_IFREG | 0o755,
                "st_uid": 0,
                "st_nlink": 1,
            },
        )()
        for path in ("/usr/bin/podman", "/usr/bin/crun", "/usr/bin/conmon"):
            with (
                mock.patch.object(broker.os, "open", return_value=51),
                mock.patch.object(broker.os, "fstat", side_effect=[status, status]),
                mock.patch.object(broker.os, "pread", return_value=raw),
                mock.patch.object(broker.os, "close") as close,
            ):
                descriptor = broker.open_verified_binary(
                    path,
                    hashlib.sha256(raw).hexdigest(),
                )
            self.assertEqual(descriptor, 51)
            close.assert_not_called()

    def test_private_state_children_are_created_handle_relative(self) -> None:
        if os.name != "posix":
            self.skipTest("directory-FD state creation is Linux-only")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            root.chmod(0o700)
            storage = root / "storage"
            storage.mkdir(mode=0o700)
            root_fd = os.open(
                root,
                os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC,
            )
            children: dict[str, int] = {}
            try:
                children = broker.open_private_state_children(root_fd)
                self.assertEqual(set(children), set(broker.STATE_CHILDREN))
                self.assertEqual(
                    sorted(item.name for item in root.iterdir()),
                    sorted(broker.STATE_CHILDREN),
                )
            finally:
                for descriptor in children.values():
                    os.close(descriptor)
                os.close(root_fd)

    def test_streaming_reads_both_pipes_without_reaping_pid(self) -> None:
        if not (
            hasattr(os, "fork")
            and hasattr(os, "waitid")
            and hasattr(os, "WNOWAIT")
        ):
            self.skipTest("fork/waitid streaming is Linux-only")
        stdout_read, stdout_write = os.pipe()
        stderr_read, stderr_write = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(stdout_read)
            os.close(stderr_read)
            os.write(stdout_write, b"out")
            os.write(stderr_write, b"err")
            os.close(stdout_write)
            os.close(stderr_write)
            os._exit(0)
        os.close(stdout_write)
        os.close(stderr_write)
        result = broker._stream_child(
            pid,
            stdout_read,
            stderr_read,
            timeout_seconds=1.0,
            output_limit=16,
        )
        self.assertEqual(result, broker.CommandResult(0, b"out", b"err"))
        selected, _status = os.waitpid(pid, os.WNOHANG)
        self.assertEqual(selected, pid)

    def test_streaming_timeout_rejects_partial_output(self) -> None:
        if not (
            hasattr(os, "fork")
            and hasattr(os, "waitid")
            and hasattr(os, "WNOWAIT")
        ):
            self.skipTest("fork/waitid streaming is Linux-only")
        stdout_read, stdout_write = os.pipe()
        stderr_read, stderr_write = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(stdout_read)
            os.close(stderr_read)
            os.write(stdout_write, b"partial")
            time.sleep(1)
            os._exit(0)
        os.close(stdout_write)
        os.close(stderr_write)
        try:
            with self.assertRaises(broker.BrokerUnavailable) as caught:
                broker._stream_child(
                    pid,
                    stdout_read,
                    stderr_read,
                    timeout_seconds=0.01,
                    output_limit=16,
                )
            self.assertEqual(caught.exception.code, "BROKER_RUNTIME_TIMEOUT")
        finally:
            os.kill(pid, 9)
            os.waitpid(pid, 0)

    def test_combined_output_accumulator_rejects_partial_result(self) -> None:
        output = broker.OutputAccumulator(limit=5)
        output.add_stdout(b"abc")
        output.add_stderr(b"de")
        self.assertEqual(output.stdout, b"abc")
        self.assertEqual(output.stderr, b"de")
        with self.assertRaises(broker.BrokerUnavailable) as caught:
            output.add_stdout(b"f")
        self.assertEqual(caught.exception.code, "BROKER_RUNTIME_OUTPUT_LIMIT")
        self.assertEqual(output.stdout, b"")
        self.assertEqual(output.stderr, b"")

    def test_internal_worker_protocol_round_trips_only_bounded_results(self) -> None:
        payload = broker._worker_success_payload(
            [
                broker.CommandResult(0, b'{"host":{}}', b""),
                broker.CommandResult(1, b"", b"missing"),
            ]
        )
        raw = json.dumps(payload, separators=(",", ":")).encode()
        results, error = broker._parse_worker_protocol(
            raw,
            returncode=0,
            output_limit=64,
        )
        self.assertIsNone(error)
        self.assertIsNotNone(results)
        assert results is not None
        self.assertEqual(results["runtime_version"].stdout, b'{"host":{}}')
        self.assertEqual(results["image_inspect"].stderr, b"missing")
        self.assertNotIn("authority_sha256", payload)
        self.assertNotIn("conformance_status", payload)

    def test_internal_worker_error_cannot_claim_authority(self) -> None:
        payload = broker._worker_error_payload("BROKER_FAILED", "closed failure")
        results, error = broker._parse_worker_protocol(
            json.dumps(payload, separators=(",", ":")).encode(),
            returncode=1,
            output_limit=64,
        )
        self.assertIsNone(results)
        self.assertIsNotNone(error)
        assert error is not None
        self.assertEqual(error.code, "BROKER_FAILED")
        self.assertNotIn("authorization_scope", payload)

    def test_internal_worker_protocol_rejects_bad_encoding_and_overflow(self) -> None:
        payload = broker._worker_success_payload(
            [
                broker.CommandResult(0, b"abcd", b""),
                broker.CommandResult(0, b"", b""),
            ]
        )
        payload["results"]["runtime_version"]["stdout_base64"] = "%%%%"
        with self.assertRaises(broker.BrokerUnavailable):
            broker._parse_worker_protocol(
                json.dumps(payload, separators=(",", ":")).encode(),
                returncode=0,
                output_limit=4,
            )
        payload = broker._worker_success_payload(
            [
                broker.CommandResult(0, b"abcde", b""),
                broker.CommandResult(0, b"", b""),
            ]
        )
        with self.assertRaises(broker.BrokerUnavailable):
            broker._parse_worker_protocol(
                json.dumps(payload, separators=(",", ":")).encode(),
                returncode=0,
                output_limit=4,
            )

    def test_cgroup_empty_parser_is_exact(self) -> None:
        self.assertTrue(broker.cgroup_is_empty(b"populated 0\nfrozen 0\n"))
        self.assertFalse(broker.cgroup_is_empty(b"populated 1\nfrozen 0\n"))
        for invalid in (b"", b"populated 00\n", b"populated 0\npopulated 1\n"):
            with self.assertRaises(broker.BrokerUnavailable):
                broker.cgroup_is_empty(invalid)

    def test_kernel_seccomp_probe_requires_closed_action_set(self) -> None:
        status = os.stat_result(
            (
                stat.S_IFREG | 0o444,
                1,
                2,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
            )
        )
        with (
            mock.patch.object(broker.os, "open", return_value=51),
            mock.patch.object(broker.os, "fstat", return_value=status),
            mock.patch.object(
                broker.os,
                "read",
                return_value=b"kill_process kill_thread trap errno trace log allow\n",
            ),
            mock.patch.object(broker.os, "close"),
        ):
            broker.validate_kernel_seccomp()
        with (
            mock.patch.object(broker.os, "open", return_value=51),
            mock.patch.object(broker.os, "fstat", return_value=status),
            mock.patch.object(broker.os, "read", return_value=b"errno allow\n"),
            mock.patch.object(broker.os, "close"),
            self.assertRaises(broker.BrokerUnavailable) as caught,
        ):
            broker.validate_kernel_seccomp()
        self.assertEqual(caught.exception.code, "BROKER_SECCOMP_UNAVAILABLE")

    def test_cleanup_kills_group_proves_empty_and_removes_cgroup(self) -> None:
        if not hasattr(os, "killpg"):
            self.skipTest("process-group cleanup is POSIX-only")
        cgroup = broker.CgroupHandle(root_fd=10, child_fd=11, name="known")
        with (
            mock.patch.object(broker, "_write_small_at") as write,
            mock.patch.object(
                broker,
                "_read_small_at",
                return_value=b"populated 0\nfrozen 0\n",
            ),
            mock.patch.object(broker, "_reap_adopted_children"),
            mock.patch.object(broker.os, "killpg") as kill_group,
            mock.patch.object(broker.os, "close") as close,
            mock.patch.object(broker.os, "rmdir") as remove,
        ):
            broker.cleanup_command_cgroup(
                cgroup,
                process_group=123,
                timeout_seconds=0.1,
            )
        write.assert_called_once_with(11, "cgroup.kill", b"1\n")
        kill_group.assert_called_once_with(123, 9)
        close.assert_called_once_with(11)
        remove.assert_called_once_with("known", dir_fd=10)

    def test_backend_manifest_has_no_process_import(self) -> None:
        manifest = json.loads(
            (FIXTURE_ROOT / "preflight-broker-backend-imports.json").read_bytes()
        )
        self.assertNotIn("subprocess", manifest["imports"])
        self.assertIn("preflight_brokered", manifest["required_exports"])


if __name__ == "__main__":
    unittest.main()
