#!/usr/bin/env python3
"""Fail-closed Linux OCI capability preflight for trusted conformance.

This is deliberately separate from ``build_tool_conformance_execution``:
this module owns process APIs, while the authority and contract validator
remain process-free. The first Linux tranche validates immutable backend
identities, proves required rootless-Podman host capabilities, and constructs
the runner-owned invariant-probe container. It never decodes or executes a
fixture case.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat

# This is the deliberately isolated process-owning backend.
import subprocess  # nosec B404
import sys
import tempfile
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import build_tool_conformance as bootstrap

DEFAULT_FIXTURE_ROOT = bootstrap.DEFAULT_FIXTURE_ROOT
DEFAULT_IDENTITY_SCHEMA = DEFAULT_FIXTURE_ROOT / "linux-oci-backend.schema.json"
PODMAN_PATH = Path("/usr/bin/podman")
CRUN_PATH = Path("/usr/bin/crun")
PODMAN_COMMAND = "/usr/bin/podman"
CRUN_COMMAND = "/usr/bin/crun"
MAX_RUNTIME_OUTPUT_BYTES = 262_144
RUNTIME_TIMEOUT_SECONDS = 15.0
REQUIRED_CGROUP_CONTROLLERS = frozenset({"cpu", "memory", "pids"})


class LinuxOciUnavailable(RuntimeError):
    """A stable fail-closed backend capability failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


@dataclass(frozen=True)
class CommandResult:
    """Bounded result returned by a direct runtime command."""

    returncode: int
    stdout: bytes
    stderr: bytes


CommandRunner = Callable[[list[str], dict[str, str], float], CommandResult]
DigestReader = Callable[[Path], str]


def identity_sha256(raw: bytes) -> str:
    """Return the identity of the exact descriptor bytes."""

    return hashlib.sha256(raw).hexdigest()


def _mapping(value: object, *, code: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise LinuxOciUnavailable(code, "backend identity record is invalid")
    return value


def validate_identity(
    identity: dict[str, Any],
    schema: dict[str, Any] | None = None,
) -> None:
    """Validate the closed identity and cross-field digest bindings."""

    selected_schema = schema or bootstrap.load_document(DEFAULT_IDENTITY_SCHEMA)
    errors = bootstrap._schema_errors(identity, selected_schema)
    if errors:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_SCHEMA_INVALID",
            "Linux OCI backend identity does not match its closed schema",
        )
    image = _mapping(identity.get("image"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    manifest = image.get("manifest_sha256")
    reference = image.get("reference")
    if not isinstance(reference, str) or reference.rsplit("@sha256:", 1)[-1] != manifest:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_IDENTITY_MISMATCH",
            "image reference and manifest identity must match exactly",
        )
    shim = _mapping(identity.get("shim"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    probe = _mapping(identity.get("probe"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    if shim.get("path") == probe.get("path"):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_ARTIFACT_COLLISION",
            "shim and invariant probe must be distinct image artifacts",
        )


def load_identity(
    path: Path,
    *,
    schema_path: Path = DEFAULT_IDENTITY_SCHEMA,
) -> tuple[dict[str, Any], str]:
    """Load a bounded strict identity and return it with its raw digest."""

    try:
        with bootstrap._open_regular_no_follow(path) as source:
            raw = source.read(bootstrap.MAX_DOCUMENT_BYTES + 1)
    except (OSError, ValueError) as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_READ_FAILED",
            "Linux OCI backend identity could not be read",
        ) from error
    if len(raw) > bootstrap.MAX_DOCUMENT_BYTES:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_TOO_LARGE",
            "Linux OCI backend identity exceeds the document ceiling",
        )
    try:
        value = bootstrap.strict_load_bytes(raw)
    except bootstrap.ConformanceError as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_PARSE_FAILED",
            "Linux OCI backend identity is not strict JSON",
        ) from error
    if not isinstance(value, dict):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_SCHEMA_INVALID",
            "Linux OCI backend identity must be an object",
        )
    schema = bootstrap.load_document(schema_path)
    validate_identity(value, schema)
    return value, identity_sha256(raw)


def runtime_environment(state_root: Path) -> dict[str, str]:
    """Build the entire fixed environment for local Podman commands."""

    return {
        "HOME": str(state_root / "home"),
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "PATH": "/usr/sbin:/usr/bin:/sbin:/bin",
        "TZ": "UTC",
        "XDG_CONFIG_HOME": str(state_root / "config"),
        "XDG_RUNTIME_DIR": str(state_root / "runtime"),
    }


def _prepare_state_root(state_root: Path) -> None:
    if not state_root.is_absolute():
        raise LinuxOciUnavailable(
            "LINUX_OCI_STATE_ROOT_INVALID",
            "runner-owned state root must be absolute",
        )
    try:
        root_status = state_root.lstat()
    except OSError as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_STATE_ROOT_INVALID",
            "runner-owned state root is unavailable",
        ) from error
    is_reparse = bool(
        getattr(root_status, "st_file_attributes", 0)
        & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    )
    if (
        not stat.S_ISDIR(root_status.st_mode)
        or stat.S_ISLNK(root_status.st_mode)
        or is_reparse
    ):
        raise LinuxOciUnavailable(
            "LINUX_OCI_STATE_ROOT_INVALID",
            "runner-owned state root is not a regular directory",
        )
    if os.name == "posix" and (
        root_status.st_uid != os.geteuid() or stat.S_IMODE(root_status.st_mode) & 0o077
    ):
        raise LinuxOciUnavailable(
            "LINUX_OCI_STATE_ROOT_INVALID",
            "runner-owned state root must be private to the invoking user",
        )
    for name in ("config", "home", "runtime", "runroot", "storage"):
        child = state_root / name
        try:
            child.mkdir(mode=0o700)
        except FileExistsError:
            child_status = child.lstat()
            child_reparse = bool(
                getattr(child_status, "st_file_attributes", 0)
                & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
            )
            child_not_private = os.name == "posix" and (
                child_status.st_uid != os.geteuid()
                or stat.S_IMODE(child_status.st_mode) & 0o077
            )
            if (
                not stat.S_ISDIR(child_status.st_mode)
                or stat.S_ISLNK(child_status.st_mode)
                or child_reparse
                or child_not_private
            ):
                raise LinuxOciUnavailable(
                    "LINUX_OCI_STATE_ROOT_INVALID",
                    "runner-owned state directory is invalid",
                )
        except OSError as error:
            raise LinuxOciUnavailable(
                "LINUX_OCI_STATE_ROOT_INVALID",
                "runner-owned state directory could not be created",
            ) from error


def _binary_digest(path: Path) -> str:
    """Hash one root-owned, non-privileged regular binary without link following."""

    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_BINARY_UNAVAILABLE",
            "required Linux OCI backend binary is unavailable",
        ) from error
    digest = hashlib.sha256()
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise LinuxOciUnavailable(
                "LINUX_OCI_BINARY_INVALID",
                "required Linux OCI backend binary is not regular",
            )
        if before.st_mode & (stat.S_ISUID | stat.S_ISGID):
            raise LinuxOciUnavailable(
                "LINUX_OCI_BINARY_INVALID",
                "privileged Linux OCI backend binaries are forbidden",
            )
        if hasattr(before, "st_uid") and before.st_uid != 0:
            raise LinuxOciUnavailable(
                "LINUX_OCI_BINARY_INVALID",
                "Linux OCI backend binaries must be root-owned",
            )
        while chunk := os.read(descriptor, 1024 * 1024):
            digest.update(chunk)
        after = os.fstat(descriptor)
        before_identity = (
            before.st_dev,
            before.st_ino,
            before.st_size,
            before.st_mtime_ns,
        )
        after_identity = (
            after.st_dev,
            after.st_ino,
            after.st_size,
            after.st_mtime_ns,
        )
        if before_identity != after_identity:
            raise LinuxOciUnavailable(
                "LINUX_OCI_BINARY_CHANGED",
                "Linux OCI backend binary changed while it was hashed",
            )
    finally:
        os.close(descriptor)
    return digest.hexdigest()


def _runtime_prefix(state_root: Path) -> list[str]:
    return [
        PODMAN_COMMAND,
        "--root",
        str(state_root / "storage"),
        "--runroot",
        str(state_root / "runroot"),
        "--runtime",
        CRUN_COMMAND,
        "--storage-driver",
        "vfs",
    ]


def _run_command(
    argv: list[str],
    environment: dict[str, str],
    timeout_seconds: float,
) -> CommandResult:
    """Run one fixed direct command and bound all captured runtime output."""

    try:
        completed = subprocess.run(
            argv,
            check=False,
            cwd=environment["HOME"],
            env=environment,
            input=b"",
            capture_output=True,
            timeout=timeout_seconds,
            # The direct argv is constructed only by this module.
            shell=False,  # nosec B603
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_UNAVAILABLE",
            "local Linux OCI runtime capability probe failed",
        ) from error
    if len(completed.stdout) + len(completed.stderr) > MAX_RUNTIME_OUTPUT_BYTES:
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_OUTPUT_LIMIT",
            "local Linux OCI runtime exceeded the preflight output ceiling",
        )
    return CommandResult(
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )


def _runtime_json(result: CommandResult) -> object:
    if result.returncode != 0:
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_UNAVAILABLE",
            "local Linux OCI runtime capability probe was not successful",
        )
    if len(result.stdout) > MAX_RUNTIME_OUTPUT_BYTES:
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_OUTPUT_LIMIT",
            "local Linux OCI runtime exceeded the preflight output ceiling",
        )
    try:
        decoded = result.stdout.decode("utf-8", errors="strict")

        def reject_duplicates(
            pairs: list[tuple[str, object]],
        ) -> dict[str, object]:
            value: dict[str, object] = {}
            for key, item in pairs:
                if key in value:
                    raise ValueError("duplicate runtime response key")
                value[key] = item
            return value

        return json.loads(decoded, object_pairs_hook=reject_duplicates)
    except (UnicodeDecodeError, ValueError) as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_RESPONSE_INVALID",
            "local Linux OCI runtime returned invalid capability data",
        ) from error


def validate_host_info(info_value: object, identity: Mapping[str, Any]) -> None:
    """Prove the required local rootless-Podman host capabilities."""

    info = _mapping(info_value, code="LINUX_OCI_RUNTIME_RESPONSE_INVALID")
    host = _mapping(info.get("host"), code="LINUX_OCI_RUNTIME_RESPONSE_INVALID")
    version = _mapping(info.get("version"), code="LINUX_OCI_RUNTIME_RESPONSE_INVALID")
    runtime = _mapping(identity.get("runtime"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    oci_runtime = _mapping(
        identity.get("oci_runtime"),
        code="LINUX_OCI_IDENTITY_SCHEMA_INVALID",
    )

    if host.get("serviceIsRemote") is not False:
        raise LinuxOciUnavailable(
            "LINUX_OCI_REMOTE_RUNTIME",
            "remote Podman is not a conforming Linux OCI backend",
        )
    security = _mapping(
        host.get("security"),
        code="LINUX_OCI_RUNTIME_RESPONSE_INVALID",
    )
    if security.get("rootless") is not True:
        raise LinuxOciUnavailable(
            "LINUX_OCI_ROOTFUL_RUNTIME",
            "rootful Podman is not a conforming Linux OCI backend",
        )
    if security.get("seccompEnabled") is not True:
        raise LinuxOciUnavailable(
            "LINUX_OCI_SECCOMP_REQUIRED",
            "seccomp is required for the Linux OCI backend",
        )
    if host.get("cgroupVersion") != "v2":
        raise LinuxOciUnavailable(
            "LINUX_OCI_CGROUP_V2_REQUIRED",
            "cgroup v2 is required for the Linux OCI backend",
        )
    controllers = host.get("cgroupControllers")
    if (
        not isinstance(controllers, list)
        or not REQUIRED_CGROUP_CONTROLLERS.issubset(
            item for item in controllers if isinstance(item, str)
        )
    ):
        raise LinuxOciUnavailable(
            "LINUX_OCI_CGROUP_CONTROLLERS_MISSING",
            "delegated cpu, memory, and pids controllers are required",
        )
    if host.get("cgroupManager") != "systemd":
        raise LinuxOciUnavailable(
            "LINUX_OCI_CGROUP_MANAGER_UNSUPPORTED",
            "rootless systemd cgroup delegation is required",
        )
    detected_oci_runtime = _mapping(
        host.get("ociRuntime"),
        code="LINUX_OCI_RUNTIME_RESPONSE_INVALID",
    )
    if (
        detected_oci_runtime.get("name") != oci_runtime.get("implementation")
        or detected_oci_runtime.get("path") != oci_runtime.get("path")
    ):
        raise LinuxOciUnavailable(
            "LINUX_OCI_CRUN_REQUIRED",
            "the reviewed local crun runtime is required",
        )
    if host.get("os") != "linux" or host.get("arch") != "amd64":
        raise LinuxOciUnavailable(
            "LINUX_OCI_HOST_PLATFORM_MISMATCH",
            "Linux amd64 is required for this OCI backend identity",
        )
    if version.get("Version") != runtime.get("version"):
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_VERSION_MISMATCH",
            "local Podman version does not match the reviewed identity",
        )


def validate_image_info(
    image_value: object,
    identity: Mapping[str, Any],
) -> None:
    """Prove the exact already-present image and reject writable volumes."""

    if not isinstance(image_value, list) or len(image_value) != 1:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_UNAVAILABLE",
            "exact reviewed OCI image is not present locally",
        )
    image_info = _mapping(
        image_value[0],
        code="LINUX_OCI_RUNTIME_RESPONSE_INVALID",
    )
    image_identity = _mapping(
        identity.get("image"),
        code="LINUX_OCI_IDENTITY_SCHEMA_INVALID",
    )
    actual_config = image_info.get("Id")
    if isinstance(actual_config, str) and actual_config.startswith("sha256:"):
        actual_config = actual_config.removeprefix("sha256:")
    if actual_config != image_identity.get("config_sha256"):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_CONFIG_MISMATCH",
            "local OCI image config identity does not match policy",
        )
    expected_manifest = f"sha256:{image_identity.get('manifest_sha256')}"
    if image_info.get("Digest") != expected_manifest:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_MANIFEST_MISMATCH",
            "local OCI image manifest identity does not match policy",
        )
    repo_digests = image_info.get("RepoDigests")
    if (
        not isinstance(repo_digests, list)
        or image_identity.get("reference") not in repo_digests
    ):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_REFERENCE_MISMATCH",
            "local OCI image does not retain the reviewed manifest reference",
        )
    if (
        image_info.get("Os") != image_identity.get("os")
        or image_info.get("Architecture") != image_identity.get("architecture")
    ):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_PLATFORM_MISMATCH",
            "local OCI image platform does not match policy",
        )
    config = _mapping(
        image_info.get("Config"),
        code="LINUX_OCI_RUNTIME_RESPONSE_INVALID",
    )
    if config.get("Volumes") not in (None, {}):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_VOLUMES_FORBIDDEN",
            "image-declared writable volumes are forbidden",
        )


def _require_runner_path(path: Path, state_root: Path, label: str) -> str:
    if not path.is_absolute():
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNNER_PATH_INVALID",
            f"runner-owned {label} path must be absolute",
        )
    try:
        path.relative_to(state_root)
    except ValueError as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNNER_PATH_INVALID",
            f"runner-owned {label} path escapes the private state root",
        ) from error
    return str(path)


def build_probe_create_argv(
    identity: Mapping[str, Any],
    *,
    limits: Mapping[str, int],
    state_root: Path,
    cidfile: Path,
    seccomp_profile: Path,
) -> list[str]:
    """Construct the exact non-executing invariant-probe container request."""

    workspace_bytes = limits.get("workspace_bytes")
    output_bytes = limits.get("output_bytes")
    if not isinstance(workspace_bytes, int) or workspace_bytes <= 0:
        raise LinuxOciUnavailable(
            "LINUX_OCI_ZERO_WORKSPACE_UNSUPPORTED",
            "Linux OCI tmpfs workspace must have a positive hard ceiling",
        )
    if not isinstance(output_bytes, int) or output_bytes <= 0:
        raise LinuxOciUnavailable(
            "LINUX_OCI_ZERO_OUTPUT_UNSUPPORTED",
            "Linux OCI probe output must have a positive hard ceiling",
        )
    process_count = limits.get("process_count")
    memory_mib = limits.get("memory_mib")
    if not isinstance(process_count, int) or process_count <= 0:
        raise LinuxOciUnavailable(
            "LINUX_OCI_LIMIT_INVALID",
            "Linux OCI task ceiling must be positive",
        )
    if not isinstance(memory_mib, int) or memory_mib <= 0:
        raise LinuxOciUnavailable(
            "LINUX_OCI_LIMIT_INVALID",
            "Linux OCI memory ceiling must be positive",
        )

    cidfile_value = _require_runner_path(cidfile, state_root, "container id")
    seccomp_value = _require_runner_path(
        seccomp_profile,
        state_root,
        "seccomp profile",
    )
    image = _mapping(identity.get("image"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    probe = _mapping(identity.get("probe"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    image_id = f"sha256:{image.get('config_sha256')}"
    memory_bytes = memory_mib * 1024 * 1024

    return [
        *_runtime_prefix(state_root),
        "create",
        "--pull=never",
        "--platform=linux/amd64",
        f"--cidfile={cidfile_value}",
        "--userns=nomap",
        "--user=65532:65532",
        "--network=none",
        "--pid=private",
        "--ipc=none",
        "--uts=private",
        "--cgroupns=private",
        "--cgroups=enabled",
        "--read-only=true",
        "--read-only-tmpfs=false",
        "--image-volume=ignore",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges=true",
        f"--security-opt=seccomp={seccomp_value}",
        f"--pids-limit={process_count}",
        f"--memory={memory_bytes}b",
        "--cgroup-conf=memory.swap.max=0",
        "--cpus=1.0",
        f"--tmpfs=/sandbox:rw,nosuid,nodev,noexec,size={workspace_bytes},mode=0700",
        "--ulimit=core=0:0",
        "--ulimit=nofile=64:64",
        "--log-driver=none",
        "--restart=no",
        "--stop-signal=SIGKILL",
        "--stop-timeout=0",
        "--no-healthcheck",
        "--systemd=false",
        "--http-proxy=false",
        "--no-hosts",
        "--hostname=conformance",
        "--umask=077",
        "--unsetenv-all",
        "--env=HOME=/sandbox/home",
        "--env=TMPDIR=/sandbox/tmp",
        "--env=LANG=C.UTF-8",
        "--env=LC_ALL=C.UTF-8",
        "--env=PATH=/usr/local/bin:/usr/bin:/bin",
        "--env=TZ=UTC",
        "--workdir=/sandbox",
        f"--entrypoint={probe.get('path')}",
        image_id,
    ]


def preflight(
    identity: dict[str, Any],
    *,
    state_root: Path,
    command_runner: CommandRunner = _run_command,
    binary_digest: DigestReader = _binary_digest,
    platform_name: str | None = None,
    effective_uid: int | None = None,
) -> dict[str, Any]:
    """Prove host and image capabilities without creating a container."""

    validate_identity(identity)
    selected_platform = platform_name or sys.platform
    if not selected_platform.startswith("linux"):
        raise LinuxOciUnavailable(
            "LINUX_OCI_PLATFORM_UNSUPPORTED",
            "Linux OCI backend preflight requires Linux",
        )
    uid = effective_uid if effective_uid is not None else os.geteuid()
    if uid == 0:
        raise LinuxOciUnavailable(
            "LINUX_OCI_ROOT_USER_FORBIDDEN",
            "Linux OCI backend preflight must run as a non-root user",
        )
    runtime = _mapping(identity["runtime"], code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    oci_runtime = _mapping(
        identity["oci_runtime"],
        code="LINUX_OCI_IDENTITY_SCHEMA_INVALID",
    )
    if binary_digest(PODMAN_PATH) != runtime.get("sha256"):
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_IDENTITY_MISMATCH",
            "local Podman binary does not match the reviewed identity",
        )
    if binary_digest(CRUN_PATH) != oci_runtime.get("sha256"):
        raise LinuxOciUnavailable(
            "LINUX_OCI_CRUN_IDENTITY_MISMATCH",
            "local crun binary does not match the reviewed identity",
        )

    _prepare_state_root(state_root)
    environment = runtime_environment(state_root)
    prefix = _runtime_prefix(state_root)
    info = _runtime_json(
        command_runner(
            [*prefix, "info", "--format", "json"],
            environment,
            RUNTIME_TIMEOUT_SECONDS,
        )
    )
    validate_host_info(info, identity)

    image = _mapping(identity["image"], code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    image_id = f"sha256:{image.get('config_sha256')}"
    image_details = _runtime_json(
        command_runner(
            [*prefix, "image", "inspect", "--format", "json", image_id],
            environment,
            RUNTIME_TIMEOUT_SECONDS,
        )
    )
    validate_image_info(image_details, identity)
    return {
        "schema_version": 1,
        "backend_kind": "linux_oci",
        "platform": "linux",
        "architecture": "amd64",
        "runtime": "rootless-podman",
        "status": "available",
        "conformance_status": "not-run",
    }


def _unavailable_result(error: LinuxOciUnavailable) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "backend_kind": "linux_oci",
        "status": "unavailable",
        "conformance_status": "non-passing",
        "diagnostics": [
            {
                "code": error.code,
                "severity": "error",
                "message": error.message,
            }
        ],
    }


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Fail-closed Linux OCI trusted-execution capability preflight."
    )
    parser.add_argument("--identity", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    try:
        arguments = parser.parse_args(argv)
        identity, digest = load_identity(arguments.identity)
        with tempfile.TemporaryDirectory(prefix="btconf-linux-oci-") as directory:
            result = preflight(identity, state_root=Path(directory))
        result["identity_sha256"] = digest
        print(json.dumps(result, sort_keys=True, separators=(",", ":")))
        return 0
    except LinuxOciUnavailable as error:
        print(
            json.dumps(
                _unavailable_result(error),
                sort_keys=True,
                separators=(",", ":"),
            )
        )
        return 1
    except SystemExit as error:
        return int(error.code)


if __name__ == "__main__":
    raise SystemExit(main())
