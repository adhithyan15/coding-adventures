#!/usr/bin/env python3
"""Verify external build-tool authority for a future Linux preflight.

Authority validation is deliberately process-free. This tranche does not
import or invoke the process-owning backend: a later exact-byte loader must
prove that the code used for capability inspection is the retained approved
artifact and its approved import closure. The v1 scope cannot authorize an
invariant probe, fixture decode, container, adapter, or execution case.
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import os
import stat
import struct
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Any

import build_tool_conformance as bootstrap

AUTHORITY_DOMAIN = (
    b"coding-adventures/build-tool-authority/linux-capability-preflight/v1\0"
)
LOADER_AUTHORITY_DOMAIN = (
    b"coding-adventures/build-tool-authority/linux-capability-preflight-loader/v1\0"
)
MAX_AUTHORITY_BUNDLE_BYTES = bootstrap.MAX_DOCUMENT_BYTES
MAX_AUTHORITY_COMPONENT_BYTES = 16_777_216
MAX_AUTHORITY_TOTAL_BYTES = 67_108_864
EMPTY_SHA256 = hashlib.sha256(b"").hexdigest()
DEFAULT_AUTHORITY_SCHEMA = (
    bootstrap.DEFAULT_FIXTURE_ROOT / "execution-authority.schema.json"
)

REPOSITORY_COMPONENT_PATHS = MappingProxyType(
    {
        "authority_bundle_schema": (
            "code/specs/fixtures/build-tool-v1/execution-authority.schema.json"
        ),
        "execution_policy_schema": (
            "code/specs/fixtures/build-tool-v1/execution-policy.schema.json"
        ),
        "execution_policy": ("code/specs/fixtures/build-tool-v1/execution-policy.json"),
        "linux_backend_identity_schema": (
            "code/specs/fixtures/build-tool-v1/linux-oci-backend.schema.json"
        ),
        "bootstrap_runner": "code/scripts/build_tool_conformance.py",
        "authority_verifier": ("code/scripts/build_tool_conformance_authority.py"),
        "linux_preflight_backend": ("code/scripts/build_tool_conformance_linux_oci.py"),
    }
)
BUNDLE_COMPONENT_PATHS = MappingProxyType(
    {"linux_backend_identity": "linux-oci-backend.json"}
)
COMPONENT_ROLES = (*REPOSITORY_COMPONENT_PATHS, *BUNDLE_COMPONENT_PATHS)
LOADER_REPOSITORY_COMPONENT_PATHS = MappingProxyType(
    {
        "authority_bundle_schema": (
            "code/specs/fixtures/build-tool-v1/"
            "execution-preflight-loader-authority.schema.json"
        ),
        "execution_policy_schema": (
            "code/specs/fixtures/build-tool-v1/execution-policy.schema.json"
        ),
        "execution_policy": ("code/specs/fixtures/build-tool-v1/execution-policy.json"),
        "linux_backend_identity_schema": (
            "code/specs/fixtures/build-tool-v1/linux-oci-backend.schema.json"
        ),
        "bootstrap_runner": "code/scripts/build_tool_conformance.py",
        "authority_verifier": ("code/scripts/build_tool_conformance_authority.py"),
        "preflight_loader": ("code/scripts/build_tool_conformance_backend_loader.py"),
        "linux_preflight_backend": ("code/scripts/build_tool_conformance_linux_oci.py"),
        "preflight_import_manifest": (
            "code/specs/fixtures/build-tool-v1/preflight-imports.json"
        ),
    }
)
LOADER_COMPONENT_ROLES = (
    *LOADER_REPOSITORY_COMPONENT_PATHS,
    *BUNDLE_COMPONENT_PATHS,
)
GIT_OID_LENGTH = 40


@dataclass(frozen=True)
class PreflightAuthority:
    """Retained exact bytes and typed values approved for one preflight."""

    bundle_digest: str
    bundle: dict[str, Any]
    components: Mapping[str, bytes]
    policy: dict[str, Any]
    identity: dict[str, Any]
    identity_schema: dict[str, Any]


@dataclass(frozen=True)
class LoaderAuthority:
    """Retained exact bytes approved only for isolated loadability."""

    bundle_digest: str
    bundle: dict[str, Any]
    components: Mapping[str, bytes]
    policy: dict[str, Any]
    identity: dict[str, Any]


def authority_bundle_sha256(raw: bytes) -> str:
    """Digest exact bundle bytes with scope and length separation."""

    digest = hashlib.sha256()
    digest.update(AUTHORITY_DOMAIN)
    digest.update(struct.pack(">Q", len(raw)))
    digest.update(raw)
    return digest.hexdigest()


def loader_authority_bundle_sha256(raw: bytes) -> str:
    """Digest exact loader-authority bytes in their separate scope."""

    digest = hashlib.sha256()
    digest.update(LOADER_AUTHORITY_DOMAIN)
    digest.update(struct.pack(">Q", len(raw)))
    digest.update(raw)
    return digest.hexdigest()


def _is_sha256(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _is_git_oid(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == GIT_OID_LENGTH
        and all(character in "0123456789abcdef" for character in value)
    )


def _file_identity(value: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_size,
        value.st_mtime_ns,
        value.st_nlink,
    )


def _read_bound_regular(
    path: Path,
    *,
    prefix: str,
    label: str,
    max_bytes: int,
) -> bytes:
    """Read one stable, singly linked, bounded regular file without follow."""

    try:
        initial = path.lstat()
        initial_reparse = bool(
            getattr(initial, "st_file_attributes", 0)
            & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
        )
        if (
            not stat.S_ISREG(initial.st_mode)
            or stat.S_ISLNK(initial.st_mode)
            or initial_reparse
            or initial.st_nlink != 1
        ):
            raise bootstrap.ConformanceError(
                f"{prefix}_FILE_INVALID",
                f"{label} must be one regular, non-reparse, singly linked file",
            )
        with bootstrap._open_regular_no_follow(path) as source:
            before = os.fstat(source.fileno())
            is_reparse = bool(
                getattr(before, "st_file_attributes", 0)
                & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
            )
            if not stat.S_ISREG(before.st_mode) or is_reparse or before.st_nlink != 1:
                raise bootstrap.ConformanceError(
                    f"{prefix}_FILE_INVALID",
                    f"{label} must be one regular, non-reparse, singly linked file",
                )
            raw = source.read(max_bytes + 1)
            after = os.fstat(source.fileno())
    except bootstrap.ConformanceError:
        raise
    except (OSError, ValueError) as error:
        raise bootstrap.ConformanceError(
            f"{prefix}_READ_FAILED",
            f"{label} could not be read without following a link",
        ) from error

    if len(raw) > max_bytes:
        raise bootstrap.ConformanceError(
            f"{prefix}_TOO_LARGE",
            f"{label} exceeds its byte ceiling",
        )
    if (
        _file_identity(initial) != _file_identity(before)
        or _file_identity(before) != _file_identity(after)
        or before.st_size != len(raw)
    ):
        raise bootstrap.ConformanceError(
            f"{prefix}_CHANGED",
            f"{label} changed during its authority read",
        )
    return raw


def _open_absolute_directory(path: Path) -> int:
    """Open an absolute directory chain without following any component."""

    if (
        os.name != "posix"
        or not path.is_absolute()
        or any(part in {".", ".."} for part in path.parts[1:])
    ):
        raise bootstrap.ConformanceError(
            "LOADER_AUTHORITY_PLATFORM_UNSUPPORTED",
            "atomic loader authority traversal requires Linux",
        )
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_DIRECTORY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    descriptor: int | None = None
    try:
        descriptor = os.open("/", flags)
        for part in path.parts[1:]:
            child = os.open(part, flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
            status = os.fstat(descriptor)
            if not stat.S_ISDIR(status.st_mode):
                raise OSError("authority directory component is not a directory")
        return descriptor
    except (OSError, ValueError) as error:
        if descriptor is not None:
            os.close(descriptor)
        raise bootstrap.ConformanceError(
            "LOADER_AUTHORITY_ROOT_INVALID",
            "loader authority root could not be retained without following links",
        ) from error
    except BaseException:
        if descriptor is not None:
            os.close(descriptor)
        raise


def _read_bound_regular_at(
    directory: int,
    relative_path: str,
    *,
    label: str,
    max_bytes: int,
) -> bytes:
    """Read a retained-root-relative file without path re-resolution."""

    parts = relative_path.split("/")
    if (
        not parts
        or any(not part or part in {".", ".."} for part in parts)
        or "\\" in relative_path
    ):
        raise bootstrap.ConformanceError(
            "LOADER_AUTHORITY_COMPONENT_ROLE_INVALID",
            f"{label} has an invalid fixed path",
        )
    directory_flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_DIRECTORY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    current = os.dup(directory)
    file_descriptor: int | None = None
    try:
        for part in parts[:-1]:
            child = os.open(part, directory_flags, dir_fd=current)
            os.close(current)
            current = child
        file_descriptor = os.open(
            parts[-1],
            os.O_RDONLY
            | getattr(os, "O_CLOEXEC", 0)
            | getattr(os, "O_NOFOLLOW", 0)
            | getattr(os, "O_NONBLOCK", 0),
            dir_fd=current,
        )
        before = os.fstat(file_descriptor)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
            raise OSError("authority component is not singly linked and regular")
        chunks: list[bytes] = []
        total = 0
        while chunk := os.read(file_descriptor, min(1_048_576, max_bytes + 1 - total)):
            chunks.append(chunk)
            total += len(chunk)
            if total > max_bytes:
                raise bootstrap.ConformanceError(
                    "LOADER_AUTHORITY_COMPONENT_TOO_LARGE",
                    f"{label} exceeds its byte ceiling",
                )
        after = os.fstat(file_descriptor)
        if _file_identity(before) != _file_identity(after) or before.st_size != total:
            raise bootstrap.ConformanceError(
                "LOADER_AUTHORITY_COMPONENT_CHANGED",
                f"{label} changed during its authority read",
            )
        return b"".join(chunks)
    except bootstrap.ConformanceError:
        raise
    except (OSError, ValueError) as error:
        raise bootstrap.ConformanceError(
            "LOADER_AUTHORITY_COMPONENT_READ_FAILED",
            f"{label} could not be read from its retained root",
        ) from error
    finally:
        if file_descriptor is not None:
            os.close(file_descriptor)
        os.close(current)


def _read_component_bytes(
    *,
    repository_root: Path,
    bundle_path: Path,
    role: str,
    record: Mapping[str, Any],
) -> bytes:
    """Resolve one closed role and retain its exact approved bytes."""

    if role in REPOSITORY_COMPONENT_PATHS:
        expected_provenance = "repository"
        expected_path = REPOSITORY_COMPONENT_PATHS[role]
        base = repository_root
    elif role in BUNDLE_COMPONENT_PATHS:
        expected_provenance = "bundle"
        expected_path = BUNDLE_COMPONENT_PATHS[role]
        base = bundle_path.parent
    else:
        raise bootstrap.ConformanceError(
            "AUTHORITY_COMPONENT_ROLE_INVALID",
            "authority bundle contains an unknown component role",
        )
    if (
        record.get("provenance") != expected_provenance
        or record.get("path") != expected_path
        or bootstrap.portable_path_error(expected_path)
    ):
        raise bootstrap.ConformanceError(
            "AUTHORITY_COMPONENT_ROLE_INVALID",
            f"authority component {role} does not match its fixed role",
        )
    return _read_bound_regular(
        base / expected_path,
        prefix="AUTHORITY_COMPONENT",
        label=f"authority component {role}",
        max_bytes=MAX_AUTHORITY_COMPONENT_BYTES,
    )


def _source_matches(
    bundle: Mapping[str, Any],
    expected_commit_oid: str,
    expected_tree_oid: str,
) -> bool:
    source = bundle.get("source")
    return (
        isinstance(source, Mapping)
        and source.get("git_object_format") == "sha1"
        and hmac.compare_digest(
            str(source.get("commit_oid", "")),
            expected_commit_oid,
        )
        and hmac.compare_digest(
            str(source.get("tree_oid", "")),
            expected_tree_oid,
        )
    )


def _validate_bundle_profile(bundle: Mapping[str, Any]) -> None:
    expected = {
        "schema_version": 1,
        "purpose": "build-tool-trusted-authority",
        "authorization_scope": "linux_capability_preflight_v1",
        "repository": "github.com/adhithyan15/coding-adventures",
        "conformance_revision": "v1",
        "platform": "linux",
        "architecture": "amd64",
    }
    if any(bundle.get(key) != value for key, value in expected.items()):
        raise bootstrap.ConformanceError(
            "AUTHORITY_PROFILE_INVALID",
            "authority bundle is not the closed Linux capability-preflight profile",
        )
    components = bundle.get("components")
    if not isinstance(components, Mapping) or set(components) != set(COMPONENT_ROLES):
        raise bootstrap.ConformanceError(
            "AUTHORITY_COMPONENT_ROLE_INVALID",
            "authority bundle must contain exactly the fixed preflight roles",
        )


def _validate_preflight_policy(
    policy: dict[str, Any],
    *,
    conformance_revision: str,
) -> None:
    expected_backends = (
        ("darwin", "macos_isolated"),
        ("linux", "linux_oci"),
        ("windows", "windows_appcontainer"),
    )
    actual_backends = policy.get("backends")
    valid_backends = (
        isinstance(actual_backends, list)
        and len(actual_backends) == len(expected_backends)
        and all(
            isinstance(item, Mapping)
            and item.get("platform") == platform
            and item.get("kind") == kind
            and item.get("status") == "unavailable"
            and item.get("identity_sha256") is None
            for item, (platform, kind) in zip(
                actual_backends,
                expected_backends,
                strict=True,
            )
        )
    )
    if (
        policy.get("conformance_revision") != conformance_revision
        or policy.get("enabled") is not False
        or policy.get("execution_corpus_sha256") != EMPTY_SHA256
        or policy.get("adapters") != []
        or not valid_backends
    ):
        raise bootstrap.ConformanceError(
            "AUTHORITY_POLICY_PROFILE_INVALID",
            "capability preflight requires the disabled empty execution profile",
        )


def _validate_identity_semantics(identity: dict[str, Any]) -> None:
    image = identity.get("image")
    shim = identity.get("shim")
    probe = identity.get("probe")
    if not isinstance(image, Mapping):
        raise bootstrap.ConformanceError(
            "AUTHORITY_IDENTITY_SCHEMA_INVALID",
            "Linux backend image identity is invalid",
        )
    reference = image.get("reference")
    manifest = image.get("manifest_sha256")
    if (
        not isinstance(reference, str)
        or not isinstance(manifest, str)
        or reference.rsplit("@sha256:", 1)[-1] != manifest
    ):
        raise bootstrap.ConformanceError(
            "AUTHORITY_IMAGE_IDENTITY_MISMATCH",
            "image reference and manifest identity must match exactly",
        )
    if not isinstance(shim, Mapping) or not isinstance(probe, Mapping):
        raise bootstrap.ConformanceError(
            "AUTHORITY_IDENTITY_SCHEMA_INVALID",
            "Linux backend image artifact identity is invalid",
        )
    if shim.get("path") == probe.get("path") or shim.get("sha256") == probe.get(
        "sha256"
    ):
        raise bootstrap.ConformanceError(
            "AUTHORITY_IMAGE_ARTIFACT_COLLISION",
            "shim and invariant probe must have distinct identities",
        )


def _strict_component_document(
    raw: bytes,
    *,
    code: str,
) -> dict[str, Any]:
    try:
        return bootstrap.strict_load_bytes(
            raw,
            max_bytes=MAX_AUTHORITY_COMPONENT_BYTES,
        )
    except bootstrap.ConformanceError as error:
        raise bootstrap.ConformanceError(
            code,
            "authority component is not a strict bounded JSON object",
        ) from error


def authorize_preflight(
    bundle_path: Path,
    *,
    approved_digest: str,
    expected_commit_oid: str,
    expected_tree_oid: str,
    repository_root: Path = bootstrap.REPO_ROOT,
) -> PreflightAuthority:
    """Prove one external bundle for capability inspection only."""

    if not _is_sha256(approved_digest):
        raise bootstrap.ConformanceError(
            "AUTHORITY_DIGEST_INVALID",
            "approved authority SHA-256 must be 64 lowercase hexadecimal digits",
        )
    if not _is_git_oid(expected_commit_oid) or not _is_git_oid(expected_tree_oid):
        raise bootstrap.ConformanceError(
            "AUTHORITY_SOURCE_ID_INVALID",
            "expected source commit and tree must be full lowercase SHA-1 identities",
        )

    selected_bundle = bundle_path.absolute()
    selected_repository = repository_root.absolute()
    raw_bundle = _read_bound_regular(
        selected_bundle,
        prefix="AUTHORITY_BUNDLE",
        label="authority bundle",
        max_bytes=MAX_AUTHORITY_BUNDLE_BYTES,
    )
    actual_digest = authority_bundle_sha256(raw_bundle)
    if not hmac.compare_digest(actual_digest, approved_digest):
        raise bootstrap.ConformanceError(
            "AUTHORITY_DIGEST_MISMATCH",
            "authority bundle does not match the out-of-band approval",
        )
    bundle = bootstrap.strict_load_bytes(
        raw_bundle,
        max_bytes=MAX_AUTHORITY_BUNDLE_BYTES,
    )
    if not _source_matches(bundle, expected_commit_oid, expected_tree_oid):
        raise bootstrap.ConformanceError(
            "AUTHORITY_SOURCE_MISMATCH",
            "authority source does not match protected commit and tree identities",
        )
    _validate_bundle_profile(bundle)

    authority_schema_path = (
        selected_repository / REPOSITORY_COMPONENT_PATHS["authority_bundle_schema"]
    )
    authority_schema_raw = _read_bound_regular(
        authority_schema_path,
        prefix="AUTHORITY_COMPONENT",
        label="authority component authority_bundle_schema",
        max_bytes=MAX_AUTHORITY_COMPONENT_BYTES,
    )
    authority_schema = _strict_component_document(
        authority_schema_raw,
        code="AUTHORITY_BUNDLE_SCHEMA_INVALID",
    )
    bootstrap._validate_schema(
        bundle,
        authority_schema,
        "AUTHORITY_BUNDLE_SCHEMA_INVALID",
    )

    components_value = bundle.get("components")
    if not isinstance(components_value, Mapping):
        raise bootstrap.ConformanceError(
            "AUTHORITY_BUNDLE_SCHEMA_INVALID",
            "authority components must be a closed object",
        )
    retained: dict[str, bytes] = {}
    total_bytes = 0
    for role in COMPONENT_ROLES:
        record = components_value.get(role)
        if not isinstance(record, Mapping):
            raise bootstrap.ConformanceError(
                "AUTHORITY_COMPONENT_ROLE_INVALID",
                f"authority component {role} is missing",
            )
        if role == "authority_bundle_schema" and (
            record.get("provenance") != "repository"
            or record.get("path")
            != REPOSITORY_COMPONENT_PATHS["authority_bundle_schema"]
        ):
            raise bootstrap.ConformanceError(
                "AUTHORITY_COMPONENT_ROLE_INVALID",
                "authority schema component does not match its fixed role",
            )
        raw = (
            authority_schema_raw
            if role == "authority_bundle_schema"
            else _read_component_bytes(
                repository_root=selected_repository,
                bundle_path=selected_bundle,
                role=role,
                record=record,
            )
        )
        total_bytes += len(raw)
        if total_bytes > MAX_AUTHORITY_TOTAL_BYTES:
            raise bootstrap.ConformanceError(
                "AUTHORITY_COMPONENT_TOTAL_TOO_LARGE",
                "authority components exceed their aggregate byte ceiling",
            )
        if record.get("byte_length") != len(raw):
            raise bootstrap.ConformanceError(
                "AUTHORITY_COMPONENT_LENGTH_MISMATCH",
                f"authority component {role} has the wrong byte length",
            )
        expected_sha256 = record.get("sha256")
        if not isinstance(expected_sha256, str) or not hmac.compare_digest(
            hashlib.sha256(raw).hexdigest(),
            expected_sha256,
        ):
            raise bootstrap.ConformanceError(
                "AUTHORITY_COMPONENT_DIGEST_MISMATCH",
                f"authority component {role} has the wrong SHA-256",
            )
        retained[role] = raw

    policy_schema = _strict_component_document(
        retained["execution_policy_schema"],
        code="AUTHORITY_POLICY_SCHEMA_INVALID",
    )
    policy = _strict_component_document(
        retained["execution_policy"],
        code="AUTHORITY_POLICY_INVALID",
    )
    bootstrap._validate_schema(
        policy,
        policy_schema,
        "AUTHORITY_POLICY_SCHEMA_INVALID",
    )
    _validate_preflight_policy(
        policy,
        conformance_revision=bundle["conformance_revision"],
    )

    identity_schema = _strict_component_document(
        retained["linux_backend_identity_schema"],
        code="AUTHORITY_IDENTITY_SCHEMA_INVALID",
    )
    identity = _strict_component_document(
        retained["linux_backend_identity"],
        code="AUTHORITY_IDENTITY_INVALID",
    )
    bootstrap._validate_schema(
        identity,
        identity_schema,
        "AUTHORITY_IDENTITY_SCHEMA_INVALID",
    )
    _validate_identity_semantics(identity)
    return PreflightAuthority(
        bundle_digest=actual_digest,
        bundle=bundle,
        components=MappingProxyType(retained),
        policy=policy,
        identity=identity,
        identity_schema=identity_schema,
    )


def authorize_backend_loader(
    bundle_path: Path,
    *,
    approved_digest: str,
    expected_commit_oid: str,
    expected_tree_oid: str,
    repository_root: Path = bootstrap.REPO_ROOT,
) -> LoaderAuthority:
    """Authorize exact bytes for isolated backend loadability only."""

    if not _is_sha256(approved_digest):
        raise bootstrap.ConformanceError(
            "LOADER_AUTHORITY_DIGEST_INVALID",
            "approved loader-authority SHA-256 must be lowercase hexadecimal",
        )
    if not _is_git_oid(expected_commit_oid) or not _is_git_oid(expected_tree_oid):
        raise bootstrap.ConformanceError(
            "LOADER_AUTHORITY_SOURCE_ID_INVALID",
            "expected source commit and tree must be full lowercase SHA-1 identities",
        )
    selected_bundle = bundle_path.absolute()
    selected_repository = repository_root.absolute()
    repository_descriptor: int | None = None
    bundle_descriptor: int | None = None
    try:
        repository_descriptor = _open_absolute_directory(selected_repository)
        bundle_descriptor = _open_absolute_directory(selected_bundle.parent)
        raw_bundle = _read_bound_regular_at(
            bundle_descriptor,
            selected_bundle.name,
            label="loader authority bundle",
            max_bytes=MAX_AUTHORITY_BUNDLE_BYTES,
        )
        actual_digest = loader_authority_bundle_sha256(raw_bundle)
        if not hmac.compare_digest(actual_digest, approved_digest):
            raise bootstrap.ConformanceError(
                "LOADER_AUTHORITY_DIGEST_MISMATCH",
                "loader authority does not match the out-of-band approval",
            )
        bundle = bootstrap.strict_load_bytes(
            raw_bundle,
            max_bytes=MAX_AUTHORITY_BUNDLE_BYTES,
        )
        if not _source_matches(bundle, expected_commit_oid, expected_tree_oid):
            raise bootstrap.ConformanceError(
                "LOADER_AUTHORITY_SOURCE_MISMATCH",
                "loader authority source does not match protected identities",
            )
        expected_profile = {
            "schema_version": 1,
            "purpose": "build-tool-trusted-authority",
            "authorization_scope": "linux_capability_preflight_loader_v1",
            "repository": "github.com/adhithyan15/coding-adventures",
            "conformance_revision": "v1",
            "platform": "linux",
            "architecture": "amd64",
        }
        components_value = bundle.get("components")
        if (
            any(bundle.get(key) != value for key, value in expected_profile.items())
            or not isinstance(components_value, Mapping)
            or set(components_value) != set(LOADER_COMPONENT_ROLES)
        ):
            raise bootstrap.ConformanceError(
                "LOADER_AUTHORITY_PROFILE_INVALID",
                "authority is not the closed exact-loader profile",
            )

        retained: dict[str, bytes] = {}
        total_bytes = 0
        for role in LOADER_COMPONENT_ROLES:
            record = components_value.get(role)
            if not isinstance(record, Mapping):
                raise bootstrap.ConformanceError(
                    "LOADER_AUTHORITY_COMPONENT_ROLE_INVALID",
                    f"loader authority component {role} is missing",
                )
            if role in LOADER_REPOSITORY_COMPONENT_PATHS:
                expected_provenance = "repository"
                expected_path = LOADER_REPOSITORY_COMPONENT_PATHS[role]
                root_descriptor = repository_descriptor
            else:
                expected_provenance = "bundle"
                expected_path = BUNDLE_COMPONENT_PATHS[role]
                root_descriptor = bundle_descriptor
            if (
                record.get("provenance") != expected_provenance
                or record.get("path") != expected_path
                or bootstrap.portable_path_error(expected_path)
            ):
                raise bootstrap.ConformanceError(
                    "LOADER_AUTHORITY_COMPONENT_ROLE_INVALID",
                    f"loader authority component {role} does not match its role",
                )
            raw = _read_bound_regular_at(
                root_descriptor,
                expected_path,
                label=f"loader authority component {role}",
                max_bytes=MAX_AUTHORITY_COMPONENT_BYTES,
            )
            total_bytes += len(raw)
            if total_bytes > MAX_AUTHORITY_TOTAL_BYTES:
                raise bootstrap.ConformanceError(
                    "LOADER_AUTHORITY_COMPONENT_TOTAL_TOO_LARGE",
                    "loader authority components exceed their aggregate ceiling",
                )
            if record.get("byte_length") != len(raw):
                raise bootstrap.ConformanceError(
                    "LOADER_AUTHORITY_COMPONENT_LENGTH_MISMATCH",
                    f"loader authority component {role} has the wrong length",
                )
            digest = record.get("sha256")
            if not isinstance(digest, str) or not hmac.compare_digest(
                hashlib.sha256(raw).hexdigest(), digest
            ):
                raise bootstrap.ConformanceError(
                    "LOADER_AUTHORITY_COMPONENT_DIGEST_MISMATCH",
                    f"loader authority component {role} has the wrong digest",
                )
            retained[role] = raw

        authority_schema = _strict_component_document(
            retained["authority_bundle_schema"],
            code="LOADER_AUTHORITY_BUNDLE_SCHEMA_INVALID",
        )
        bootstrap._validate_schema(
            bundle,
            authority_schema,
            "LOADER_AUTHORITY_BUNDLE_SCHEMA_INVALID",
        )
        policy_schema = _strict_component_document(
            retained["execution_policy_schema"],
            code="LOADER_AUTHORITY_POLICY_SCHEMA_INVALID",
        )
        policy = _strict_component_document(
            retained["execution_policy"],
            code="LOADER_AUTHORITY_POLICY_INVALID",
        )
        bootstrap._validate_schema(
            policy,
            policy_schema,
            "LOADER_AUTHORITY_POLICY_SCHEMA_INVALID",
        )
        _validate_preflight_policy(
            policy,
            conformance_revision=bundle["conformance_revision"],
        )
        identity_schema = _strict_component_document(
            retained["linux_backend_identity_schema"],
            code="LOADER_AUTHORITY_IDENTITY_SCHEMA_INVALID",
        )
        identity = _strict_component_document(
            retained["linux_backend_identity"],
            code="LOADER_AUTHORITY_IDENTITY_INVALID",
        )
        bootstrap._validate_schema(
            identity,
            identity_schema,
            "LOADER_AUTHORITY_IDENTITY_SCHEMA_INVALID",
        )
        _validate_identity_semantics(identity)
        manifest = _strict_component_document(
            retained["preflight_import_manifest"],
            code="LOADER_AUTHORITY_IMPORT_MANIFEST_INVALID",
        )
        if (
            set(manifest) != {"schema_version", "module", "imports", "required_exports"}
            or manifest.get("schema_version") != 1
            or manifest.get("module") != "build_tool_conformance_linux_oci"
            or manifest.get("required_exports")
            != [
                "CommandResult",
                "LinuxOciUnavailable",
                "preflight_prevalidated",
            ]
        ):
            raise bootstrap.ConformanceError(
                "LOADER_AUTHORITY_IMPORT_MANIFEST_INVALID",
                "loader authority import manifest is outside the v1 profile",
            )
        return LoaderAuthority(
            bundle_digest=actual_digest,
            bundle=bundle,
            components=MappingProxyType(retained),
            policy=policy,
            identity=identity,
        )
    finally:
        if repository_descriptor is not None:
            os.close(repository_descriptor)
        if bundle_descriptor is not None:
            os.close(bundle_descriptor)


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Verify external build-tool capability-preflight authority."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    selected = subparsers.add_parser(
        "validate-authority",
        help="Validate exact authority without importing a process backend.",
    )
    selected.add_argument("--authority-bundle", type=Path, required=True)
    selected.add_argument("--approved-authority-sha256", required=True)
    selected.add_argument("--source-commit", required=True)
    selected.add_argument("--source-tree", required=True)
    selected.add_argument(
        "--repository-root",
        type=Path,
        default=bootstrap.REPO_ROOT,
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    try:
        arguments = parser.parse_args(argv)
    except SystemExit as error:
        return int(error.code)
    try:
        keywords = {
            "approved_digest": arguments.approved_authority_sha256,
            "expected_commit_oid": arguments.source_commit,
            "expected_tree_oid": arguments.source_tree,
            "repository_root": arguments.repository_root,
        }
        approved = authorize_preflight(
            arguments.authority_bundle,
            **keywords,
        )
        output = {
            "schema_version": 1,
            "authorization_scope": approved.bundle["authorization_scope"],
            "authority_sha256": approved.bundle_digest,
            "status": "valid",
            "conformance_status": "not-run",
        }
        exit_code = 0
    except bootstrap.ConformanceError as error:
        print(
            json.dumps(
                {
                    "code": error.code,
                    "message": error.message,
                    "status": "error",
                },
                sort_keys=True,
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 2
    print(json.dumps(output, sort_keys=True, separators=(",", ":")))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
