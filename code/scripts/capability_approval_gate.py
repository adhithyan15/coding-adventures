#!/usr/bin/env python3
"""Verify signed Spec 13 capability-profile changes against trusted base data.

The script is intended to be executed from the pull-request base revision.
Candidate manifests and approvals are read as git blobs and are always treated
as untrusted bytes. The only signing implementation reused here is
``release_gate.py`` from the same trusted base revision.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import os
import re
import shlex
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from types import ModuleType
from typing import Any

from jsonschema import Draft202012Validator
from referencing import Registry, Resource


REQUIRED_SCHEMA_PATH = "code/specs/schemas/required_capabilities.schema.json"
APPROVAL_SCHEMA_PATH = "code/specs/schemas/capability-approval.schema.json"
RELEASE_GATE_PATH = "code/scripts/release_gate.py"
MANIFEST_PREFIXES = ("code/packages/", "code/programs/")
APPROVAL_PREFIX = "code/specs/capability-approvals/"
SIGNATURE_NAMESPACE = "coding-adventures-capabilities"
MAX_MANIFEST_BYTES = 256 * 1024
MAX_APPROVAL_BYTES = 256 * 1024
MAX_SIGNATURE_BYTES = 64 * 1024
HEX_OBJECT_ID = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
MANIFEST_PATH_PATTERN = re.compile(
    r"^code/(?:packages|programs)/[a-z0-9_-]+/[a-z0-9][a-z0-9._-]*/required_capabilities\.json$"
)


class GateError(Exception):
    """A stable, fail-closed diagnostic with an operator-facing code."""

    def __init__(self, code: str, message: str) -> None:
        self.code = code
        self.message = message
        super().__init__(f"{code}: {message}")


@dataclass(frozen=True)
class Profile:
    status: str
    digest: str | None
    capabilities: list[dict[str, Any]] | None
    exceptions: list[dict[str, Any]] | None
    package: str | None


def _no_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GateError("DUPLICATE_JSON_MEMBER", f"duplicate JSON member {key!r}")
        result[key] = value
    return result


def strict_json_loads(raw: bytes, label: str) -> Any:
    """Parse UTF-8 JSON while rejecting duplicate object keys."""
    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=_no_duplicate_pairs)
    except UnicodeDecodeError as exc:
        raise GateError("INVALID_JSON", f"{label} is not UTF-8") from exc
    except json.JSONDecodeError as exc:
        raise GateError("INVALID_JSON", f"{label} is invalid JSON at byte {exc.pos}") from exc
    except RecursionError as exc:
        raise GateError("INVALID_JSON", f"{label} exceeds the supported JSON nesting depth") from exc


def canonical_json_bytes(document: Any) -> bytes:
    """Return the one byte representation accepted for signed approval JSON."""
    try:
        return (
            json.dumps(
                document,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
                allow_nan=False,
            ).encode("utf-8")
            + b"\n"
        )
    except (TypeError, ValueError) as exc:
        raise GateError("INVALID_JSON", "approval cannot be represented as canonical JSON") from exc


def _git(root: str, *args: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    result = subprocess.run(
        ["git", *args],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if check and result.returncode != 0:
        message = result.stderr.decode("utf-8", "replace").strip()
        raise GateError("GIT", message or f"git {' '.join(args[:2])} failed")
    return result


def git_blob(
    root: str,
    revision: str,
    path: str,
    *,
    max_size: int = MAX_APPROVAL_BYTES,
) -> bytes | None:
    """Read one bounded Git blob; only a confirmed missing path means absent."""
    object_name = f"{revision}:{path}"
    kind = _git(root, "cat-file", "-t", object_name, check=False)
    if kind.returncode != 0:
        message = kind.stderr.decode("utf-8", "replace").strip()
        if re.fullmatch(
            r"fatal: path '.+' (?:does not exist in|exists on disk, but not in) '.+'",
            message,
        ):
            return None
        raise GateError("GIT_READ", message or f"cannot inspect {path} at {revision}")
    if kind.stdout.strip() != b"blob":
        raise GateError("GIT_OBJECT_TYPE", f"{path} at {revision} is not a blob")
    size_result = _git(root, "cat-file", "-s", object_name)
    try:
        size = int(size_result.stdout.strip())
    except ValueError as exc:
        raise GateError("GIT_READ", f"git returned an invalid object size for {path}") from exc
    if size < 0 or size > max_size:
        raise GateError("INPUT_TOO_LARGE", f"{path} exceeds the {max_size}-byte input limit")
    return _git(root, "cat-file", "blob", object_name).stdout


def _schema_validator(
    schema_bytes: bytes,
    required_schema_bytes: bytes | None = None,
) -> Draft202012Validator:
    schema = strict_json_loads(schema_bytes, "trusted schema")
    try:
        Draft202012Validator.check_schema(schema)
        if required_schema_bytes is None:
            return Draft202012Validator(schema)
        required_schema = strict_json_loads(required_schema_bytes, "trusted required-capabilities schema")
        Draft202012Validator.check_schema(required_schema)
        registry = Registry().with_resource(
            required_schema["$id"], Resource.from_contents(required_schema)
        )
        return Draft202012Validator(schema, registry=registry)
    except (KeyError, TypeError, ValueError) as exc:
        raise GateError("TRUSTED_SCHEMA", f"trusted schema is invalid: {exc}") from exc


def _validation_message(validator: Draft202012Validator, document: Any, label: str) -> str | None:
    try:
        errors = sorted(validator.iter_errors(document), key=lambda error: tuple(str(p) for p in error.absolute_path))
    except RecursionError:
        return f"{label} exceeds the supported JSON nesting depth"
    if not errors:
        return None
    error = errors[0]
    pointer = "/" + "/".join(str(part).replace("~", "~0").replace("/", "~1") for part in error.absolute_path)
    return f"{label} fails schema at {pointer}: {error.message}"


def _profile(raw: bytes | None, validator: Draft202012Validator, label: str) -> Profile:
    if raw is None:
        return Profile("absent", None, [], [], None)
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) > MAX_MANIFEST_BYTES:
        return Profile("invalid", digest, None, None, None)
    try:
        document = strict_json_loads(raw, label)
    except GateError:
        return Profile("invalid", digest, None, None, None)
    if _validation_message(validator, document, label) is not None:
        return Profile("invalid", digest, None, None, None)
    return Profile(
        "valid",
        digest,
        document["capabilities"],
        document.get("banned_construct_exceptions", []),
        document["package"],
    )


def _manifest_path(path: str) -> PurePosixPath:
    parsed = PurePosixPath(path)
    if (
        MANIFEST_PATH_PATTERN.fullmatch(path) is None
        or parsed.is_absolute()
        or ".." in parsed.parts
        or "\\" in path
        or parsed.name != "required_capabilities.json"
        or not any(path.startswith(prefix) for prefix in MANIFEST_PREFIXES)
        or len(parsed.parts) != 5
    ):
        raise GateError("MANIFEST_PATH", f"unsupported capability manifest path {path!r}")
    return parsed


def _package_root(path: PurePosixPath) -> str:
    return PurePosixPath(*path.parts[:4]).as_posix()


def _package_name(path: PurePosixPath) -> str:
    return f"{path.parts[2]}/{path.parts[3]}"


def approval_path_for_manifest(manifest_path: str) -> str:
    parsed = _manifest_path(manifest_path)
    relative = PurePosixPath(*parsed.parts[1:-1])
    return PurePosixPath(APPROVAL_PREFIX, *relative.parts).with_suffix(".json").as_posix()


def _changed_paths(root: str, base_commit: str, head_commit: str, pathspec: str) -> set[str]:
    result = _git(
        root,
        "diff",
        "--no-renames",
        "--name-only",
        "-z",
        base_commit,
        head_commit,
        "--",
        pathspec,
    )
    return {path.decode("utf-8") for path in result.stdout.split(b"\0") if path}


def _tree_oid(root: str, revision: str, package_root: str) -> str | None:
    result = _git(root, "ls-tree", "-z", revision, "--", package_root)
    records = [record for record in result.stdout.split(b"\0") if record]
    if not records:
        return None
    if len(records) != 1 or b"\t" not in records[0]:
        raise GateError("PACKAGE_TREE", f"git returned an ambiguous package tree entry for {package_root}")
    metadata, raw_path = records[0].split(b"\t", 1)
    fields = metadata.split()
    if raw_path.decode("utf-8", "strict") != package_root or len(fields) != 3:
        raise GateError("PACKAGE_TREE", f"git returned a malformed package tree entry for {package_root}")
    mode, object_type, raw_oid = fields
    if mode != b"040000" or object_type != b"tree":
        raise GateError("PACKAGE_TREE", f"{package_root} exists but is not a package directory tree")
    oid = raw_oid.decode("ascii", "strict")
    if not HEX_OBJECT_ID.fullmatch(oid):
        raise GateError("PACKAGE_TREE", f"git returned an invalid package tree id for {package_root}")
    return oid


def _requires_approval(old: Profile, new: Profile) -> bool:
    if old.status == "invalid":
        # An invalid base grants no trustworthy semantic baseline. Any edit,
        # including deleting the malformed file, must bind its raw digest to
        # an authorized repair decision.
        return new.status in {"valid", "absent"}
    if new.status == "invalid":
        return False
    old_capabilities = old.capabilities or []
    new_capabilities = new.capabilities or []
    old_exceptions = old.exceptions or []
    new_exceptions = new.exceptions or []
    return (
        old_capabilities != new_capabilities
        and bool(old_capabilities or new_capabilities)
    ) or (
        old_exceptions != new_exceptions
        and bool(old_exceptions or new_exceptions)
    )


def _profile_payload_fields(profile: Profile, prefix: str) -> dict[str, Any]:
    return {
        f"{prefix}_profile_status": profile.status,
        f"{prefix}_manifest_sha256": profile.digest,
        f"{prefix}_capabilities": profile.capabilities,
        f"{prefix}_banned_construct_exceptions": profile.exceptions,
    }


def _expected_approval(
    *,
    manifest_path: str,
    package: str,
    base_ref: str,
    base_commit: str,
    package_tree: str | None,
    old: Profile,
    new: Profile,
    reason: str,
    approved_by: str,
    approved_at: str,
) -> dict[str, Any]:
    return {
        "version": 1,
        "type": "capability_escalation",
        "manifest_path": manifest_path,
        "package": package,
        "base_ref": base_ref,
        "base_commit": base_commit,
        "package_tree": package_tree,
        **_profile_payload_fields(old, "previous"),
        "new_profile_status": new.status,
        "new_manifest_sha256": new.digest,
        "new_capabilities": new.capabilities,
        "new_banned_construct_exceptions": new.exceptions,
        "reason": reason,
        "approved_by": approved_by,
        "approved_at": approved_at,
    }


def _check_payload_matches(
    approval: dict[str, Any],
    *,
    manifest_path: str,
    package: str,
    base_ref: str,
    base_commit: str,
    package_tree: str | None,
    old: Profile,
    new: Profile,
) -> None:
    expected = _expected_approval(
        manifest_path=manifest_path,
        package=package,
        base_ref=base_ref,
        base_commit=base_commit,
        package_tree=package_tree,
        old=old,
        new=new,
        reason=approval["reason"],
        approved_by=approval["approved_by"],
        approved_at=approval["approved_at"],
    )
    if approval != expected:
        raise GateError("STALE_APPROVAL", "approval does not exactly match the base and candidate package profiles")


def _has_namespace(entry: dict[str, Any], namespace: str) -> bool:
    options = entry.get("options", "")
    match = re.search(r'(?:^|,)namespaces="([^"]*)"(?:,|$)', options)
    if match is None:
        return False
    return namespace in match.group(1).split(",")


def _authorized_signer(entries: list[dict[str, Any]], principal: str) -> dict[str, Any] | None:
    """Select a trusted principal entry that explicitly grants this namespace."""
    return next(
        (
            entry
            for entry in entries
            if entry.get("principal") == principal
            and _has_namespace(entry, SIGNATURE_NAMESPACE)
        ),
        None,
    )


def _load_trusted_release_gate(root: str, base_commit: str) -> ModuleType:
    raw = git_blob(root, base_commit, RELEASE_GATE_PATH)
    if raw is None:
        raise GateError("TRUSTED_VERIFIER", f"{RELEASE_GATE_PATH} is absent from PR base")
    with tempfile.TemporaryDirectory(prefix="capability-gate-trusted-") as temp:
        path = PurePosixPath(temp, "release_gate.py")
        # The imported module keeps its code object after the temporary source
        # is removed; no candidate-tree Python path is used for this import.
        from pathlib import Path

        trusted_path = Path(str(path))
        trusted_path.write_bytes(raw)
        spec = importlib.util.spec_from_file_location("_trusted_release_gate", trusted_path)
        if spec is None or spec.loader is None:
            raise GateError("TRUSTED_VERIFIER", "could not load release gate from PR base")
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        spec.loader.exec_module(module)
        return module


def _load_trusted_schemas(root: str, base_commit: str) -> tuple[Draft202012Validator, Draft202012Validator]:
    required_raw = git_blob(root, base_commit, REQUIRED_SCHEMA_PATH)
    approval_raw = git_blob(root, base_commit, APPROVAL_SCHEMA_PATH)
    if required_raw is None or approval_raw is None:
        raise GateError("TRUSTED_SCHEMA", "required capability approval schemas are absent from PR base")
    return _schema_validator(required_raw), _schema_validator(approval_raw, required_raw)


def _validate_profile_package(profile: Profile, path: PurePosixPath, label: str) -> None:
    if profile.status == "valid" and profile.package != _package_name(path):
        raise GateError("PACKAGE_MISMATCH", f"{label} package identity does not match its path")


def _load_candidate_approval(
    root: str,
    head_commit: str,
    approval_path: str,
    approval_validator: Draft202012Validator,
) -> tuple[dict[str, Any], str]:
    raw = git_blob(root, head_commit, approval_path)
    signature_raw = git_blob(
        root, head_commit, approval_path + ".sig", max_size=MAX_SIGNATURE_BYTES
    )
    if raw is None or signature_raw is None:
        raise GateError("MISSING_APPROVAL", f"expected approval JSON and detached signature at {approval_path}")
    if len(raw) > MAX_APPROVAL_BYTES or len(signature_raw) > MAX_SIGNATURE_BYTES:
        raise GateError("INPUT_TOO_LARGE", "approval JSON or signature exceeds the size limit")
    approval = strict_json_loads(raw, approval_path)
    if not isinstance(approval, dict):
        raise GateError("INVALID_APPROVAL", "approval payload must be a JSON object")
    if raw != canonical_json_bytes(approval):
        raise GateError("NONCANONICAL_APPROVAL", "approval bytes are not canonical UTF-8 JSON")
    message = _validation_message(approval_validator, approval, approval_path)
    if message:
        raise GateError("INVALID_APPROVAL", message)
    timestamp = approval["approved_at"]
    try:
        parsed_timestamp = dt.datetime.fromisoformat(timestamp[:-1] + "+00:00")
    except ValueError as exc:
        raise GateError("INVALID_APPROVAL", "approved_at is not a valid UTC timestamp") from exc
    if parsed_timestamp.utcoffset() != dt.timedelta(0):
        raise GateError("INVALID_APPROVAL", "approved_at must be expressed in UTC")
    try:
        signature = signature_raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise GateError("INVALID_SIGNATURE", "detached signature is not UTF-8") from exc
    return approval, signature


def _proposal_instructions(
    *,
    manifest_path: str,
    approval_path: str,
    base_ref: str,
    base_commit: str,
    head_commit: str,
) -> str:
    command = [
        "python3",
        "code/scripts/capability_approval_gate.py",
        "propose",
        "--repo-root",
        ".",
        "--manifest-path",
        manifest_path,
        "--base-ref",
        base_ref,
        "--base-commit",
        base_commit,
        "--head-commit",
        head_commit,
        "--reason",
        "REPLACE WITH REVIEWED CAPABILITY JUSTIFICATION",
        "--output",
        approval_path,
    ]
    return " ".join(shlex.quote(part) for part in command)


def verify_changes(
    repo_root: str,
    *,
    base_ref: str,
    base_commit: str,
    head_commit: str,
    trusted_release_gate: ModuleType | None = None,
    trusted_schema_validators: tuple[Draft202012Validator, Draft202012Validator] | None = None,
) -> list[str]:
    """Verify changed profiles. Returns a concise list of verified packages."""
    if not HEX_OBJECT_ID.fullmatch(base_commit) or not HEX_OBJECT_ID.fullmatch(head_commit):
        raise GateError("REVISION", "base and head must be full Git object IDs")
    _git(repo_root, "cat-file", "-e", f"{base_commit}^{{commit}}")
    _git(repo_root, "cat-file", "-e", f"{head_commit}^{{commit}}")
    if not base_ref or "\n" in base_ref or "\0" in base_ref:
        raise GateError("BASE_REF", "invalid base ref")

    if trusted_schema_validators is None:
        required_validator, approval_validator = _load_trusted_schemas(repo_root, base_commit)
    else:
        required_validator, approval_validator = trusted_schema_validators

    changed = _changed_paths(repo_root, base_commit, head_commit, "code/packages")
    changed |= _changed_paths(repo_root, base_commit, head_commit, "code/programs")
    manifest_paths = sorted(
        path
        for path in changed
        if path.endswith("/required_capabilities.json")
        and len(PurePosixPath(path).parts) == 5
        and any(path.startswith(prefix) for prefix in MANIFEST_PREFIXES)
    )
    changed_approval_paths = _changed_paths(
        repo_root, base_commit, head_commit, APPROVAL_PREFIX.rstrip("/")
    )
    expected_approval_paths: set[str] = set()
    verified: list[str] = []
    pending: list[tuple[str, str, Profile, Profile, str | None]] = []

    for path in manifest_paths:
        parsed = _manifest_path(path)
        package = _package_name(parsed)
        base_raw = git_blob(repo_root, base_commit, path, max_size=MAX_MANIFEST_BYTES)
        head_raw = git_blob(repo_root, head_commit, path, max_size=MAX_MANIFEST_BYTES)
        if base_raw == head_raw:
            continue
        old = _profile(base_raw, required_validator, f"base {path}")
        new = _profile(head_raw, required_validator, f"candidate {path}")
        if new.status == "invalid":
            raise GateError("INVALID_CANDIDATE_MANIFEST", f"candidate {path} is not a valid schema-v1 manifest")
        _validate_profile_package(old, parsed, f"base {path}")
        _validate_profile_package(new, parsed, f"candidate {path}")
        package_root = _package_root(parsed)
        package_tree = _tree_oid(repo_root, head_commit, package_root)
        if new.status == "valid" and package_tree is None:
            raise GateError("PACKAGE_TREE", f"candidate manifest exists but package tree {package_root} is absent")
        if _requires_approval(old, new):
            approval_path = approval_path_for_manifest(path)
            expected_approval_paths.update({approval_path, approval_path + ".sig"})
            pending.append((path, package, old, new, package_tree))

    unexpected = changed_approval_paths - expected_approval_paths
    if unexpected:
        unexpected_list = ", ".join(sorted(unexpected))
        raise GateError("ORPHAN_APPROVAL", f"approval changes do not match a changed security-relevant profile: {unexpected_list}")

    approvals: list[tuple[str, str, dict[str, Any], str]] = []
    for path, package, old, new, package_tree in pending:
        approval_path = approval_path_for_manifest(path)
        try:
            approval, signature = _load_candidate_approval(
                repo_root, head_commit, approval_path, approval_validator
            )
        except GateError as exc:
            if exc.code == "MISSING_APPROVAL":
                command = _proposal_instructions(
                    manifest_path=path,
                    approval_path=approval_path,
                    base_ref=base_ref,
                    base_commit=base_commit,
                    head_commit=head_commit,
                )
                raise GateError(
                    exc.code,
                    f"{exc.message}. Create an unsigned proposal with: {command}",
                ) from exc
            raise
        _check_payload_matches(
            approval,
            manifest_path=path,
            package=package,
            base_ref=base_ref,
            base_commit=base_commit,
            package_tree=package_tree,
            old=old,
            new=new,
        )
        approvals.append((package, approval_path, approval, signature))

    if pending and trusted_release_gate is None:
        trusted_release_gate = _load_trusted_release_gate(repo_root, base_commit)

    signers_text: str | None = None
    signer_entries: list[dict[str, Any]] = []
    if pending:
        try:
            from pathlib import Path

            signers_text, signer_entries = trusted_release_gate.verify_signers_chain(  # type: ignore[union-attr]
                Path(repo_root), base_commit
            )
        except Exception as exc:  # release_gate exposes specific GateError details
            message = getattr(exc, "message", str(exc))
            code = getattr(exc, "code", "NO_TRUSTED_SIGNER")
            if "no signer has been registered" in message or "no key" in message or "no key" in code.lower():
                code = "NO_TRUSTED_SIGNER"
            raise GateError(code, message) from exc
        if not signer_entries:
            raise GateError("NO_TRUSTED_SIGNER", "trusted base signer registry contains no keys")

    for package, approval_path, approval, signature in approvals:
        principal = approval["approved_by"]
        signer = _authorized_signer(signer_entries, principal)
        if signer is None:
            raise GateError(
                "UNAUTHORIZED_SIGNER",
                f"{principal!r} is not authorized for namespace {SIGNATURE_NAMESPACE!r} at PR base",
            )
        try:
            verified_principal = trusted_release_gate.verify_signed_file(  # type: ignore[union-attr]
                signers_text,
                canonical_json_bytes(approval),
                signature,
                SIGNATURE_NAMESPACE,
            )
        except Exception as exc:
            raise GateError(
                getattr(exc, "code", "INVALID_SIGNATURE"),
                getattr(exc, "message", str(exc)),
            ) from exc
        if verified_principal != principal:
            raise GateError("PRINCIPAL_MISMATCH", "SSH signature principal does not match approved_by")
        verified.append(package)

    return verified


def propose_approval(
    repo_root: str,
    *,
    manifest_path: str,
    base_ref: str,
    base_commit: str,
    head_commit: str,
    reason: str,
    approved_by: str = "UNSIGNED-PROPOSAL",
    approved_at: str = "1970-01-01T00:00:00Z",
) -> bytes:
    """Build canonical unsigned approval bytes for human review and signing."""
    parsed = _manifest_path(manifest_path)
    required_raw = git_blob(repo_root, base_commit, REQUIRED_SCHEMA_PATH, max_size=MAX_MANIFEST_BYTES)
    if required_raw is None:
        raise GateError("TRUSTED_SCHEMA", "required-capabilities schema is absent from PR base")
    validator = _schema_validator(required_raw)
    old = _profile(git_blob(repo_root, base_commit, manifest_path), validator, f"base {manifest_path}")
    new = _profile(git_blob(repo_root, head_commit, manifest_path), validator, f"candidate {manifest_path}")
    if new.status == "invalid":
        raise GateError("INVALID_CANDIDATE_MANIFEST", "candidate manifest is not schema-valid")
    _validate_profile_package(old, parsed, f"base {manifest_path}")
    _validate_profile_package(new, parsed, f"candidate {manifest_path}")
    if not _requires_approval(old, new):
        raise GateError("NO_APPROVAL_REQUIRED", "this profile change does not require a signed approval")
    package_tree = _tree_oid(repo_root, head_commit, _package_root(parsed))
    if new.status == "valid" and package_tree is None:
        raise GateError("PACKAGE_TREE", "candidate package tree is absent")
    document = _expected_approval(
        manifest_path=manifest_path,
        package=_package_name(parsed),
        base_ref=base_ref,
        base_commit=base_commit,
        package_tree=package_tree,
        old=old,
        new=new,
        reason=reason,
        approved_by=approved_by,
        approved_at=approved_at,
    )
    payload = canonical_json_bytes(document)
    approval_schema_raw = git_blob(
        repo_root, base_commit, APPROVAL_SCHEMA_PATH, max_size=MAX_APPROVAL_BYTES
    )
    if approval_schema_raw is None:
        raise GateError("TRUSTED_SCHEMA", "capability approval schema is absent from PR base")
    approval_validator = _schema_validator(approval_schema_raw, required_raw)
    validation_error = _validation_message(approval_validator, document, "unsigned proposal")
    if validation_error:
        raise GateError("INVALID_PROPOSAL", validation_error)
    return payload


def safe_write_repo_file(repo_root: str, relative_path: str, payload: bytes) -> str:
    """Create a new repository-relative file without following symlinks."""
    parsed = PurePosixPath(relative_path)
    if (
        parsed.is_absolute()
        or ".." in parsed.parts
        or "." in parsed.parts
        or "\\" in relative_path
        or not parsed.parts
    ):
        raise GateError("PROPOSAL_PATH", "proposal output must be a clean repository-relative path")
    if not hasattr(os, "O_NOFOLLOW") or not hasattr(os, "O_DIRECTORY"):
        raise GateError("PROPOSAL_OUTPUT", "safe proposal creation requires no-follow directory support")
    root = Path(repo_root).resolve(strict=True)
    directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    root_fd = os.open(root, directory_flags)
    parent_fd = root_fd
    opened_fds = [root_fd]
    try:
        for component in parsed.parts[:-1]:
            try:
                child_fd = os.open(component, directory_flags, dir_fd=parent_fd)
            except FileNotFoundError:
                os.mkdir(component, mode=0o755, dir_fd=parent_fd)
                child_fd = os.open(component, directory_flags, dir_fd=parent_fd)
            opened_fds.append(child_fd)
            parent_fd = child_fd
        leaf_flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW
        file_fd = os.open(parsed.parts[-1], leaf_flags, 0o600, dir_fd=parent_fd)
        try:
            remaining = memoryview(payload)
            while remaining:
                written = os.write(file_fd, remaining)
                if written <= 0:
                    raise OSError("short write while creating proposal")
                remaining = remaining[written:]
            os.fsync(file_fd)
        finally:
            os.close(file_fd)
        return parsed.as_posix()
    except GateError:
        raise
    except OSError as exc:
        raise GateError("PROPOSAL_OUTPUT", f"cannot safely create {relative_path}: {exc}") from exc
    finally:
        for fd in reversed(opened_fds):
            os.close(fd)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    verify = subparsers.add_parser("verify", help="verify changed capability profiles")
    verify.add_argument("--repo-root", required=True)
    verify.add_argument("--base-ref", required=True)
    verify.add_argument("--base-commit", required=True)
    verify.add_argument("--head-commit", required=True)
    propose = subparsers.add_parser("propose", help="write unsigned approval payload bytes")
    propose.add_argument("--repo-root", required=True)
    propose.add_argument("--manifest-path", required=True)
    propose.add_argument("--base-ref", required=True)
    propose.add_argument("--base-commit", required=True)
    propose.add_argument("--head-commit", required=True)
    propose.add_argument("--reason", required=True)
    propose.add_argument("--approved-by", default="UNSIGNED-PROPOSAL")
    propose.add_argument("--approved-at", default="1970-01-01T00:00:00Z")
    propose.add_argument("--output", required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "verify":
            packages = verify_changes(
                args.repo_root,
                base_ref=args.base_ref,
                base_commit=args.base_commit,
                head_commit=args.head_commit,
            )
            if packages:
                print("Verified signed capability profiles: " + ", ".join(sorted(packages)))
            else:
                print("No changed nonempty capability profile requires a signed approval.")
            return 0
        proposal = propose_approval(
            args.repo_root,
            manifest_path=args.manifest_path,
            base_ref=args.base_ref,
            base_commit=args.base_commit,
            head_commit=args.head_commit,
            reason=args.reason,
            approved_by=args.approved_by,
            approved_at=args.approved_at,
        )
        output = Path(safe_write_repo_file(args.repo_root, args.output, proposal))
        print(f"Unsigned proposal written to {output}")
        print(
            "After owner review and signer bootstrap, sign the exact file with: "
            + shlex.join(["ssh-keygen", "-Y", "sign", "-f", "<HARDWARE_KEY_PATH>", "-n", SIGNATURE_NAMESPACE, str(output)])
        )
        print("No signature was created.")
        return 0
    except GateError as exc:
        print(f"::error title=Capability approval gate [{exc.code}]::{exc.message}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
