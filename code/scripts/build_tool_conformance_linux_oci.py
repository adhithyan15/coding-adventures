#!/usr/bin/env python3
"""Fail-closed Linux OCI capability preflight for trusted conformance.

This is deliberately separate from ``build_tool_conformance_execution``.
It validates immutable backend identities, consumes only bounded results from
the separately authorized capability broker, and constructs the runner-owned
invariant-probe container request without invoking it. It never owns a process,
decodes a fixture case, or executes a container.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

DEFAULT_IDENTITY_SCHEMA = Path("linux-oci-backend.schema.json")
MAX_DOCUMENT_BYTES = 2_000_000
PODMAN_COMMAND = "/usr/bin/podman"
CRUN_COMMAND = "/usr/bin/crun"
MAX_RUNTIME_OUTPUT_BYTES = 262_144
MAX_RUNTIME_JSON_DEPTH = 64


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


def identity_sha256(raw: bytes) -> str:
    """Return the identity of the exact descriptor bytes."""

    return hashlib.sha256(raw).hexdigest()


def _mapping(value: object, *, code: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise LinuxOciUnavailable(code, "backend identity record is invalid")
    return value


def _is_sha256(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _is_version(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) <= 32
        and len(value.split(".")) == 3
        and all(part and part.isascii() and part.isdigit() for part in value.split("."))
    )


def _is_image_path(value: object) -> bool:
    allowed = frozenset(
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"
    )
    return (
        isinstance(value, str)
        and 2 <= len(value) <= 256
        and value.startswith("/")
        and all(segment and set(segment) <= allowed for segment in value[1:].split("/"))
    )


def _is_image_reference(value: object) -> bool:
    if not isinstance(value, str) or len(value) > 512:
        return False
    prefix, separator, digest = value.rpartition("@sha256:")
    allowed = frozenset("abcdefghijklmnopqrstuvwxyz0123456789._/-")
    return (
        separator == "@sha256:"
        and bool(prefix)
        and prefix[0] in frozenset("abcdefghijklmnopqrstuvwxyz0123456789")
        and set(prefix) <= allowed
        and _is_sha256(digest)
    )


def validate_identity(
    identity: dict[str, Any],
    schema: dict[str, Any] | None = None,
) -> None:
    """Validate the closed identity and cross-field digest bindings."""

    del schema
    exact_keys = {
        "schema_version",
        "backend_kind",
        "platform",
        "architecture",
        "runtime",
        "oci_runtime",
        "conmon",
        "image",
        "seccomp_profile_sha256",
        "shim",
        "probe",
    }
    valid = (
        set(identity) == exact_keys
        and type(identity.get("schema_version")) is int
        and identity.get("schema_version") == 1
        and identity.get("backend_kind") == "linux_oci"
        and identity.get("platform") == "linux"
        and identity.get("architecture") == "amd64"
        and isinstance(identity.get("runtime"), dict)
        and set(identity["runtime"])
        == {"implementation", "path", "version", "linkage", "sha256"}
        and identity["runtime"].get("implementation") == "podman"
        and identity["runtime"].get("path") == "/usr/bin/podman"
        and identity["runtime"].get("linkage") == "static"
        and _is_version(identity["runtime"].get("version"))
        and _is_sha256(identity["runtime"].get("sha256"))
        and isinstance(identity.get("oci_runtime"), dict)
        and set(identity["oci_runtime"]) == {"implementation", "path", "sha256"}
        and identity["oci_runtime"].get("implementation") == "crun"
        and identity["oci_runtime"].get("path") == "/usr/bin/crun"
        and _is_sha256(identity["oci_runtime"].get("sha256"))
        and isinstance(identity.get("conmon"), dict)
        and set(identity["conmon"]) == {"implementation", "path", "sha256"}
        and identity["conmon"].get("implementation") == "conmon"
        and identity["conmon"].get("path") == "/usr/bin/conmon"
        and _is_sha256(identity["conmon"].get("sha256"))
        and isinstance(identity.get("image"), dict)
        and set(identity["image"])
        == {
            "reference",
            "manifest_sha256",
            "config_sha256",
            "os",
            "architecture",
        }
        and identity["image"].get("os") == "linux"
        and identity["image"].get("architecture") == "amd64"
        and _is_image_reference(identity["image"].get("reference"))
        and _is_sha256(identity["image"].get("manifest_sha256"))
        and _is_sha256(identity["image"].get("config_sha256"))
        and isinstance(identity.get("shim"), dict)
        and set(identity["shim"]) == {"path", "sha256"}
        and _is_image_path(identity["shim"].get("path"))
        and _is_sha256(identity["shim"].get("sha256"))
        and isinstance(identity.get("probe"), dict)
        and set(identity["probe"]) == {"path", "sha256"}
        and _is_image_path(identity["probe"].get("path"))
        and _is_sha256(identity["probe"].get("sha256"))
        and _is_sha256(identity.get("seccomp_profile_sha256"))
    )
    if not valid:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_SCHEMA_INVALID",
            "Linux OCI backend identity does not match its closed schema",
        )
    image = _mapping(identity.get("image"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    manifest = image.get("manifest_sha256")
    reference = image.get("reference")
    if (
        not isinstance(reference, str)
        or reference.rsplit("@sha256:", 1)[-1] != manifest
    ):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IMAGE_IDENTITY_MISMATCH",
            "image reference and manifest identity must match exactly",
        )
    shim = _mapping(identity.get("shim"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    probe = _mapping(identity.get("probe"), code="LINUX_OCI_IDENTITY_SCHEMA_INVALID")
    if shim.get("path") == probe.get("path") or shim.get("sha256") == probe.get(
        "sha256"
    ):
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

    del schema_path
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(
            os,
            "O_NOFOLLOW",
            0,
        )
    )
    try:
        descriptor = os.open(path, flags)
        try:
            status = os.fstat(descriptor)
            if not stat.S_ISREG(status.st_mode):
                raise OSError("identity is not regular")
            chunks: list[bytes] = []
            total = 0
            while chunk := os.read(
                descriptor,
                min(1_048_576, MAX_DOCUMENT_BYTES + 1 - total),
            ):
                chunks.append(chunk)
                total += len(chunk)
                if total > MAX_DOCUMENT_BYTES:
                    break
            after = os.fstat(descriptor)
            before_identity = (
                status.st_dev,
                status.st_ino,
                status.st_size,
                status.st_mtime_ns,
            )
            after_identity = (
                after.st_dev,
                after.st_ino,
                after.st_size,
                after.st_mtime_ns,
            )
            if before_identity != after_identity:
                raise OSError("identity changed while it was read")
            raw = b"".join(chunks)
            if total <= MAX_DOCUMENT_BYTES and before_identity[2] != len(raw):
                raise OSError("identity length changed while it was read")
        finally:
            os.close(descriptor)
    except (OSError, ValueError) as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_READ_FAILED",
            "Linux OCI backend identity could not be read",
        ) from error
    if len(raw) > MAX_DOCUMENT_BYTES:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_TOO_LARGE",
            "Linux OCI backend identity exceeds the document ceiling",
        )
    try:

        def reject_duplicates(
            pairs: list[tuple[str, object]],
        ) -> dict[str, object]:
            value: dict[str, object] = {}
            for key, item in pairs:
                if key in value:
                    raise ValueError("duplicate identity key")
                value[key] = item
            return value

        value = json.loads(
            raw.decode("utf-8", errors="strict"),
            object_pairs_hook=reject_duplicates,
        )
    except (UnicodeDecodeError, ValueError) as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_PARSE_FAILED",
            "Linux OCI backend identity is not strict JSON",
        ) from error
    if not isinstance(value, dict):
        raise LinuxOciUnavailable(
            "LINUX_OCI_IDENTITY_PARSE_FAILED",
            "Linux OCI backend identity is not a strict JSON object",
        )
    validate_identity(value)
    return value, identity_sha256(raw)


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


def _reject_excessive_json_nesting(value: str) -> None:
    depth = 0
    in_string = False
    escaped = False
    for character in value:
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
        elif character == '"':
            in_string = True
        elif character in "[{":
            depth += 1
            if depth > MAX_RUNTIME_JSON_DEPTH:
                raise ValueError("runtime response nesting exceeds the ceiling")
        elif character in "]}":
            depth -= 1
            if depth < 0:
                raise ValueError("runtime response has unbalanced nesting")


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
        _reject_excessive_json_nesting(decoded)

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
    except (RecursionError, UnicodeDecodeError, ValueError) as error:
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_RESPONSE_INVALID",
            "local Linux OCI runtime returned invalid capability data",
        ) from error


def validate_runtime_version(
    version_value: object,
    identity: Mapping[str, Any],
) -> None:
    """Validate the closed local-only Podman version response."""

    version = _mapping(
        version_value,
        code="LINUX_OCI_RUNTIME_RESPONSE_INVALID",
    )
    client = _mapping(
        version.get("Client"),
        code="LINUX_OCI_RUNTIME_RESPONSE_INVALID",
    )
    runtime = _mapping(
        identity.get("runtime"),
        code="LINUX_OCI_IDENTITY_SCHEMA_INVALID",
    )
    if "Server" in version and version.get("Server") is not None:
        raise LinuxOciUnavailable(
            "LINUX_OCI_REMOTE_RUNTIME",
            "remote Podman is not a conforming Linux OCI backend",
        )
    if client.get("Version") != runtime.get("version"):
        raise LinuxOciUnavailable(
            "LINUX_OCI_RUNTIME_VERSION_MISMATCH",
            "local Podman version does not match the reviewed identity",
        )
    if client.get("Os") != "linux" or client.get("OsArch") != "linux/amd64":
        raise LinuxOciUnavailable(
            "LINUX_OCI_HOST_PLATFORM_MISMATCH",
            "Linux amd64 is required for this OCI backend identity",
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
    if image_info.get("Os") != image_identity.get("os") or image_info.get(
        "Architecture"
    ) != image_identity.get("architecture"):
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


def preflight_brokered(
    identity: dict[str, Any],
    *,
    runtime_info: CommandResult,
    image_inspect: CommandResult,
    platform_name: str | None = None,
    effective_uid: int | None = None,
) -> dict[str, Any]:
    """Validate capability data returned by the exact protected broker."""

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
    version = _runtime_json(runtime_info)
    validate_runtime_version(version, identity)
    image_details = _runtime_json(image_inspect)
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


def preflight_prevalidated(
    identity: dict[str, Any],
    *,
    runtime_info: CommandResult,
    image_inspect: CommandResult,
    platform_name: str | None = None,
    effective_uid: int | None = None,
) -> dict[str, Any]:
    """Consume mandatory broker results for an already validated identity."""

    return preflight_brokered(
        identity,
        runtime_info=runtime_info,
        image_inspect=image_inspect,
        platform_name=platform_name,
        effective_uid=effective_uid,
    )


def preflight(
    identity: dict[str, Any],
    *,
    runtime_info: CommandResult,
    image_inspect: CommandResult,
    identity_schema: dict[str, Any] | None = None,
    platform_name: str | None = None,
    effective_uid: int | None = None,
) -> dict[str, Any]:
    """Validate an identity, then consume mandatory broker results."""

    validate_identity(identity, identity_schema)
    return preflight_prevalidated(
        identity,
        runtime_info=runtime_info,
        image_inspect=image_inspect,
        platform_name=platform_name,
        effective_uid=effective_uid,
    )


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
        parser.parse_args(argv)
        error = LinuxOciUnavailable(
            "LINUX_OCI_AUTHORITY_REQUIRED",
            "use build_tool_conformance_authority.py with out-of-band approval",
        )
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
