#!/usr/bin/env python3
"""Build and verify the ADJ standard-library provenance CAS.

The tool is deliberately offline. A controlled spider captures response bytes;
this program stores those exact bytes, creates a first-class fetch receipt,
validates complete byte accounting, and projects selected snapshots into the
flat hash directory consumed by ``adj-verify``.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import os
import re
import stat
import tempfile
from collections.abc import Iterable
from datetime import datetime
from pathlib import Path, PurePosixPath
from typing import Any
from urllib.parse import urlparse

DEFAULT_ROOT = Path("code/specs/data/adj-stdlib-provenance/cas")
DEFAULT_MANIFEST = Path("code/specs/data/adj-stdlib-provenance/manifest.json")
DEFAULT_SCHEMA = Path("code/specs/data/adj-stdlib-provenance/manifest.schema.json")
MAX_OBJECT_BYTES = 64 * 1024 * 1024
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
OBJECT_KINDS = {
    "fetch_receipt",
    "input_receipt",
    "provenance_bundle",
    "raw_source",
    "rendered_text",
    "source_ir",
    "text_transform",
}
RECEIPT_HEADERS = {
    "content-encoding",
    "content-language",
    "content-length",
    "content-type",
    "etag",
    "last-modified",
}


class ProvenanceError(ValueError):
    """A stable, user-actionable provenance validation failure."""


def canonical_json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _is_link_or_reparse(info: os.stat_result) -> bool:
    attributes = getattr(info, "st_file_attributes", 0)
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    return stat.S_ISLNK(info.st_mode) or bool(attributes & reparse)


def _reject_link_components(path: Path, *, allow_missing_leaf: bool = False) -> None:
    absolute = Path(os.path.abspath(path))
    current = Path(absolute.anchor)
    parts = absolute.parts[1:] if absolute.anchor else absolute.parts
    for index, part in enumerate(parts):
        current /= part
        try:
            info = current.lstat()
        except FileNotFoundError:
            if allow_missing_leaf:
                return
            raise ProvenanceError(f"missing path component: {current}") from None
        if _is_link_or_reparse(info):
            raise ProvenanceError(f"refusing link or reparse point: {current}")


def _read_regular_file(path: Path, *, limit: int = MAX_OBJECT_BYTES) -> bytes:
    _reject_link_components(path)
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except FileNotFoundError as error:
        raise ProvenanceError(f"missing file: {path}") from error
    try:
        before = os.fstat(descriptor)
        if _is_link_or_reparse(before) or not stat.S_ISREG(before.st_mode):
            raise ProvenanceError(f"refusing non-regular file: {path}")
        if before.st_size > limit:
            raise ProvenanceError(f"file exceeds {limit} byte limit: {path}")
        chunks: list[bytes] = []
        remaining = limit + 1
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        data = b"".join(chunks)
        after = os.fstat(descriptor)
        identity_before = (
            before.st_dev,
            before.st_ino,
            before.st_size,
            before.st_mtime_ns,
        )
        identity_after = (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
        if identity_before != identity_after or len(data) != after.st_size:
            raise ProvenanceError(f"file changed while it was read: {path}")
        if len(data) > limit:
            raise ProvenanceError(f"file exceeds {limit} byte limit: {path}")
    finally:
        os.close(descriptor)
    _reject_link_components(path)
    return data


def _ensure_real_directory(path: Path) -> None:
    if path.exists():
        _reject_link_components(path)
    else:
        _reject_link_components(path, allow_missing_leaf=True)
    path.mkdir(parents=True, exist_ok=True)
    _reject_link_components(path)
    info = path.lstat()
    if _is_link_or_reparse(info) or not stat.S_ISDIR(info.st_mode):
        raise ProvenanceError(f"refusing non-directory CAS path: {path}")


def _write_exclusive(path: Path, data: bytes) -> None:
    _ensure_real_directory(path.parent)
    if path.exists():
        if _read_regular_file(path, limit=len(data)) != data:
            raise ProvenanceError(f"refusing to overwrite existing bytes: {path}")
        return
    descriptor, temporary_name = tempfile.mkstemp(prefix=".cas-", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError:
            if _read_regular_file(path, limit=len(data)) != data:
                raise ProvenanceError(
                    f"concurrent CAS write disagrees at {path}"
                ) from None
        if _read_regular_file(path, limit=len(data)) != data:
            raise ProvenanceError(f"published bytes do not re-read exactly: {path}")
    finally:
        temporary.unlink(missing_ok=True)


def _write_atomic(path: Path, data: bytes) -> None:
    _ensure_real_directory(path.parent)
    if path.exists():
        _reject_link_components(path)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".index-", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _require_hash(value: Any, field: str) -> str:
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        raise ProvenanceError(f"{field} must be a lowercase SHA-256")
    return value


def _is_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _require_nonempty(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ProvenanceError(f"{field} must be a non-empty string")
    return value


def _require_https_locator(value: Any, field: str) -> str:
    locator = _require_nonempty(value, field)
    parsed = urlparse(locator)
    if (
        parsed.scheme != "https"
        or not parsed.netloc
        or parsed.username
        or parsed.password
    ):
        raise ProvenanceError(f"{field} must be an absolute credential-free HTTPS URL")
    return locator


def _require_utc_timestamp(value: Any, field: str) -> str:
    timestamp = _require_nonempty(value, field)
    if not timestamp.endswith("Z"):
        raise ProvenanceError(f"{field} must be an ISO-8601 UTC timestamp ending in Z")
    try:
        datetime.fromisoformat(timestamp[:-1] + "+00:00")
    except ValueError as error:
        raise ProvenanceError(f"{field} must be an ISO-8601 UTC timestamp") from error
    return timestamp


class Cas:
    """SHA-256 fanout CAS with a checked, deterministic index."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.objects = root / "objects"
        self.index_path = root / "index.json"
        self.index: dict[str, dict[str, Any]] = {}

    def object_path(self, digest: str) -> Path:
        _require_hash(digest, "object hash")
        return self.objects / digest[:2] / digest[2:]

    def load(self) -> None:
        if not self.index_path.exists():
            self.index = {}
            return
        value = json.loads(_read_regular_file(self.index_path).decode("utf-8"))
        if not isinstance(value, dict):
            raise ProvenanceError("CAS index must be a JSON object")
        self.index = value

    def put(
        self,
        data: bytes,
        *,
        kind: str,
        label: str,
        links: Iterable[str] = (),
    ) -> str:
        if kind not in OBJECT_KINDS:
            raise ProvenanceError(f"unsupported CAS object kind: {kind}")
        _require_nonempty(label, "object label")
        if len(data) > MAX_OBJECT_BYTES:
            raise ProvenanceError(f"object exceeds {MAX_OBJECT_BYTES} byte limit")
        digest = sha256_bytes(data)
        normalized_links = sorted(
            {_require_hash(link, "object link") for link in links}
        )
        path = self.object_path(digest)
        if path.exists():
            existing = _read_regular_file(path)
            if existing != data:
                raise ProvenanceError(f"CAS collision or drift at {digest}")
        else:
            _ensure_real_directory(self.root)
            _ensure_real_directory(self.objects)
            _ensure_real_directory(path.parent)
            _write_exclusive(path, data)
        record = {
            "kinds": [kind],
            "links": normalized_links,
            "path": path.relative_to(self.root).as_posix(),
            "sha256": digest,
            "size": len(data),
        }
        prior = self.index.get(digest)
        if prior is not None:
            record["kinds"] = sorted(set(prior.get("kinds", [])) | {kind})
            record["links"] = sorted(
                set(prior.get("links", [])) | set(normalized_links)
            )
            stable_fields = {key: record[key] for key in ("path", "sha256", "size")}
            if any(prior.get(key) != value for key, value in stable_fields.items()):
                raise ProvenanceError(
                    f"CAS metadata disagrees for existing object {digest}"
                )
        self.index[digest] = record
        return digest

    def put_json(
        self,
        value: Any,
        *,
        kind: str,
        label: str,
        links: Iterable[str] = (),
    ) -> str:
        return self.put(
            canonical_json_bytes(value), kind=kind, label=label, links=links
        )

    def write_index(self) -> None:
        _ensure_real_directory(self.root)
        _write_atomic(self.index_path, canonical_json_bytes(self.index))


def build_fetch_receipt(
    *,
    locator: str,
    final_locator: str,
    retrieved_at: str,
    status: int,
    media_type: str,
    body_sha256: str,
    body_size: int,
    headers: dict[str, str] | None = None,
) -> dict[str, Any]:
    _require_https_locator(locator, "receipt.locator")
    _require_https_locator(final_locator, "receipt.final_locator")
    _require_utc_timestamp(retrieved_at, "receipt.retrieved_at")
    if not _is_integer(status) or not 100 <= status <= 599:
        raise ProvenanceError(
            "receipt.status must be an HTTP status from 100 through 599"
        )
    _require_nonempty(media_type, "receipt.media_type")
    _require_hash(body_sha256, "receipt.body_sha256")
    if not _is_integer(body_size) or body_size < 0 or body_size > MAX_OBJECT_BYTES:
        raise ProvenanceError("receipt.body_size is outside the supported range")
    normalized_headers: dict[str, str] = {}
    for raw_name, raw_value in (headers or {}).items():
        name = raw_name.strip().lower()
        if name not in RECEIPT_HEADERS:
            raise ProvenanceError(f"receipt header is not allow-listed: {raw_name}")
        normalized_headers[name] = _require_nonempty(
            raw_value, f"receipt.headers.{name}"
        ).strip()
    return {
        "body_sha256": body_sha256,
        "body_size": body_size,
        "final_locator": final_locator,
        "headers": dict(sorted(normalized_headers.items())),
        "kind": "fetch_receipt",
        "locator": locator,
        "media_type": media_type,
        "retrieved_at": retrieved_at,
        "status": status,
    }


def _require_repo_path(value: Any, field: str) -> str:
    repo_path = _require_nonempty(value, field)
    path = PurePosixPath(repo_path)
    if path.is_absolute() or ".." in path.parts or path.as_posix() != repo_path:
        raise ProvenanceError(f"{field} must be a normalized repository-relative path")
    return repo_path


def git_blob_sha1(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data, usedforsecurity=False).hexdigest()


def build_input_receipt(
    *,
    repo_path: str,
    captured_at: str,
    body_sha256: str,
    body_size: int,
    body_git_sha1: str,
) -> dict[str, Any]:
    _require_repo_path(repo_path, "receipt.repo_path")
    _require_utc_timestamp(captured_at, "receipt.captured_at")
    _require_hash(body_sha256, "receipt.body_sha256")
    if not _is_integer(body_size) or body_size < 0 or body_size > MAX_OBJECT_BYTES:
        raise ProvenanceError("receipt.body_size is outside the supported range")
    if not isinstance(body_git_sha1, str) or not re.fullmatch(
        r"[0-9a-f]{40}", body_git_sha1
    ):
        raise ProvenanceError("receipt.body_git_sha1 must be a lowercase Git SHA-1")
    return {
        "body_git_sha1": body_git_sha1,
        "body_sha256": body_sha256,
        "body_size": body_size,
        "captured_at": captured_at,
        "kind": "input_receipt",
        "repo_path": repo_path,
    }


def _validate_claim(
    claim: Any, source: bytes, segment_start: int, segment_end: int
) -> None:
    if not isinstance(claim, dict) or set(claim) != {
        "claim_id",
        "end",
        "quote",
        "quote_sha256",
        "start",
    }:
        raise ProvenanceError("represented claim must have the exact claim schema")
    _require_nonempty(claim["claim_id"], "claim.claim_id")
    start = claim["start"]
    end = claim["end"]
    if (
        not _is_integer(start)
        or not _is_integer(end)
        or start < segment_start
        or end <= start
        or end > segment_end
    ):
        raise ProvenanceError("claim byte range must be inside its represented segment")
    quote = _require_nonempty(claim["quote"], "claim.quote")
    quote_hash = _require_hash(claim["quote_sha256"], "claim.quote_sha256")
    cited = source[start:end]
    try:
        cited_text = cited.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ProvenanceError("claim byte range is not valid UTF-8") from error
    if quote != cited_text or quote_hash != sha256_bytes(cited):
        raise ProvenanceError(
            "claim quote or quote hash disagrees with cited source bytes"
        )


def validate_segments(segments: Any, source: bytes) -> None:
    source_size = len(source)
    if not isinstance(segments, list) or (source_size > 0 and not segments):
        raise ProvenanceError("source_ir.segments must account for the complete source")
    cursor = 0
    for index, segment in enumerate(segments):
        prefix = f"source_ir.segments[{index}]"
        if not isinstance(segment, dict):
            raise ProvenanceError(f"{prefix} must be an object")
        start = segment.get("start")
        end = segment.get("end")
        if not _is_integer(start) or not _is_integer(end):
            raise ProvenanceError(f"{prefix} start/end must be integers")
        if start != cursor or end <= start or end > source_size:
            raise ProvenanceError(
                f"{prefix} does not continue the exact byte partition"
            )
        disposition = segment.get("disposition")
        if disposition == "represented":
            if set(segment) != {"claims", "disposition", "end", "start"}:
                raise ProvenanceError(
                    f"{prefix} has unknown or missing represented fields"
                )
            claims = segment.get("claims")
            if not isinstance(claims, list) or not claims:
                raise ProvenanceError(
                    f"{prefix}.claims must contain represented IR claims"
                )
            for claim in claims:
                _validate_claim(claim, source, start, end)
            claim_cursor = start
            for claim in sorted(claims, key=lambda item: (item["start"], item["end"])):
                if claim["start"] > claim_cursor:
                    raise ProvenanceError(
                        f"{prefix} has represented bytes outside all claim ranges"
                    )
                claim_cursor = max(claim_cursor, claim["end"])
            if claim_cursor != end:
                raise ProvenanceError(
                    f"{prefix} has represented bytes outside all claim ranges"
                )
        elif disposition == "discarded":
            if set(segment) != {"disposition", "end", "reason", "start"}:
                raise ProvenanceError(
                    f"{prefix} has unknown or missing discarded fields"
                )
            _require_nonempty(segment.get("reason"), f"{prefix}.reason")
        else:
            raise ProvenanceError(
                f"{prefix}.disposition must be represented or discarded"
            )
        cursor = end
    if cursor != source_size:
        raise ProvenanceError("source_ir.segments leave trailing bytes unaccounted")


def build_source_ir(
    *, source_sha256: str, source: bytes, segments: list[dict[str, Any]]
) -> dict[str, Any]:
    _require_hash(source_sha256, "source_ir.source_sha256")
    if len(source) > MAX_OBJECT_BYTES:
        raise ProvenanceError("source_ir source is outside the supported range")
    validate_segments(segments, source)
    return {
        "kind": "source_ir",
        "segments": segments,
        "source_sha256": source_sha256,
        "source_size": len(source),
    }


def build_text_transform(
    *,
    source_sha256: str,
    source: bytes,
    result_sha256: str,
    result: bytes,
    operations: list[dict[str, Any]],
) -> dict[str, Any]:
    _require_hash(source_sha256, "transform.source_sha256")
    _require_hash(result_sha256, "transform.result_sha256")
    if not isinstance(operations, list) or not operations:
        raise ProvenanceError("transform.operations must not be empty")
    result_cursor = 0
    source_cursor = 0
    for index, operation in enumerate(operations):
        prefix = f"transform.operations[{index}]"
        if not isinstance(operation, dict) or set(operation) != {
            "operation",
            "result_end",
            "result_start",
            "source_end",
            "source_start",
        }:
            raise ProvenanceError(f"{prefix} must have the exact operation schema")
        source_start = operation["source_start"]
        source_end = operation["source_end"]
        result_start = operation["result_start"]
        result_end = operation["result_end"]
        if not all(
            _is_integer(value)
            for value in (source_start, source_end, result_start, result_end)
        ):
            raise ProvenanceError(f"{prefix} byte ranges must be integers")
        if (
            source_start < source_cursor
            or source_end <= source_start
            or source_end > len(source)
            or result_start != result_cursor
            or result_end <= result_start
            or result_end > len(result)
        ):
            raise ProvenanceError(f"{prefix} has a non-canonical byte mapping")
        source_slice = source[source_start:source_end]
        operation_name = operation["operation"]
        if operation_name == "copy":
            expected = source_slice
        elif operation_name == "html_entity_decode":
            try:
                expected = html.unescape(source_slice.decode("utf-8")).encode("utf-8")
            except UnicodeDecodeError as error:
                raise ProvenanceError(f"{prefix} source is not UTF-8") from error
        else:
            raise ProvenanceError(f"{prefix}.operation is unsupported")
        if expected != result[result_start:result_end]:
            raise ProvenanceError(f"{prefix} does not reproduce the result bytes")
        source_cursor = source_end
        result_cursor = result_end
    if result_cursor != len(result):
        raise ProvenanceError("transform operations leave result bytes unaccounted")
    return {
        "kind": "text_transform",
        "operations": operations,
        "result_sha256": result_sha256,
        "result_size": len(result),
        "source_sha256": source_sha256,
        "source_size": len(source),
    }


def _json_object(cas: Cas, digest: str, expected_kind: str) -> dict[str, Any]:
    if digest not in cas.index:
        raise ProvenanceError(f"missing CAS object {digest}")
    record = cas.index[digest]
    if expected_kind not in record["kinds"]:
        raise ProvenanceError(f"{digest} must be a {expected_kind} object")
    data = _read_regular_file(cas.object_path(digest))
    try:
        value = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProvenanceError(f"{digest} is not valid UTF-8 JSON") from error
    if not isinstance(value, dict):
        raise ProvenanceError(f"{digest} JSON payload must be an object")
    if data != canonical_json_bytes(value):
        raise ProvenanceError(f"{digest} JSON payload is not canonical")
    return value


def _validate_index(cas: Cas) -> None:
    if not cas.index_path.exists():
        raise ProvenanceError("missing CAS index")
    if _read_regular_file(cas.index_path) != canonical_json_bytes(cas.index):
        raise ProvenanceError("CAS index is not canonical JSON")
    for digest, record in sorted(cas.index.items()):
        _require_hash(digest, "CAS index key")
        if not isinstance(record, dict):
            raise ProvenanceError(f"CAS record {digest} must be an object")
        if set(record) != {"kinds", "links", "path", "sha256", "size"}:
            raise ProvenanceError(f"CAS record {digest} has unknown or missing fields")
        if record.get("sha256") != digest:
            raise ProvenanceError(f"CAS record {digest} repeats the wrong hash")
        kinds = record.get("kinds")
        if (
            not isinstance(kinds, list)
            or kinds != sorted(set(kinds))
            or not kinds
            or any(kind not in OBJECT_KINDS for kind in kinds)
        ):
            raise ProvenanceError(f"CAS record {digest}.kinds are invalid")
        expected_path = PurePosixPath("objects") / digest[:2] / digest[2:]
        if record.get("path") != expected_path.as_posix():
            raise ProvenanceError(f"CAS record {digest} has a non-canonical path")
        links = record.get("links")
        if not isinstance(links, list) or links != sorted(set(links)):
            raise ProvenanceError(
                f"CAS record {digest}.links must be sorted and unique"
            )
        for link in links:
            _require_hash(link, f"CAS record {digest} link")
            if link not in cas.index:
                raise ProvenanceError(
                    f"CAS record {digest} links missing object {link}"
                )
        data = _read_regular_file(cas.object_path(digest))
        if not _is_integer(record.get("size")) or record.get("size") != len(data):
            raise ProvenanceError(f"CAS record {digest} has the wrong size")
        if sha256_bytes(data) != digest:
            raise ProvenanceError(f"CAS object {digest} does not rehash to its key")
    if cas.objects.exists():
        expected_files = {record["path"] for record in cas.index.values()}
        for path in cas.objects.rglob("*"):
            info = path.lstat()
            if _is_link_or_reparse(info):
                raise ProvenanceError(
                    f"link or reparse point inside CAS objects: {path.relative_to(cas.root)}"
                )
            if stat.S_ISDIR(info.st_mode):
                relative_directory = path.relative_to(cas.objects)
                if len(relative_directory.parts) != 1 or not re.fullmatch(
                    r"[0-9a-f]{2}", relative_directory.name
                ):
                    raise ProvenanceError(
                        f"non-canonical CAS directory: {path.relative_to(cas.root)}"
                    )
                continue
            relative = path.relative_to(cas.root).as_posix()
            if relative not in expected_files:
                raise ProvenanceError(
                    f"unindexed CAS object: {path.relative_to(cas.root)}"
                )


def _reachable(cas: Cas, roots: Iterable[str]) -> set[str]:
    seen: set[str] = set()
    pending = list(roots)
    while pending:
        digest = pending.pop()
        if digest in seen:
            continue
        if digest not in cas.index:
            raise ProvenanceError(f"manifest references missing CAS object {digest}")
        seen.add(digest)
        pending.extend(cas.index[digest]["links"])
    return seen


def _validate_manifest_schema(schema_path: Path, manifest: dict[str, Any]) -> None:
    try:
        from jsonschema import Draft202012Validator
        from jsonschema.exceptions import SchemaError
    except ImportError as error:
        raise ProvenanceError(
            "jsonschema is required to validate the provenance manifest"
        ) from error
    schema = json.loads(_read_regular_file(schema_path).decode("utf-8"))
    try:
        Draft202012Validator.check_schema(schema)
    except SchemaError as error:
        raise ProvenanceError(
            f"invalid provenance manifest schema: {error.message}"
        ) from error
    failures = sorted(
        ".".join(str(part) for part in issue.absolute_path) + ": " + issue.message
        for issue in Draft202012Validator(schema).iter_errors(manifest)
    )
    if failures:
        raise ProvenanceError("manifest JSON Schema: " + failures[0].lstrip(": "))


def _validate_cas_schemas(schema_path: Path, cas: Cas) -> None:
    from jsonschema import Draft202012Validator

    schema = json.loads(_read_regular_file(schema_path).decode("utf-8"))
    definitions = schema.get("$defs", {})
    schema_kinds = {
        "fetch_receipt",
        "input_receipt",
        "provenance_bundle",
        "source_ir",
        "text_transform",
    }
    for digest, record in sorted(cas.index.items()):
        for kind in sorted(set(record["kinds"]) & schema_kinds):
            value = _json_object(cas, digest, kind)
            validator = Draft202012Validator(
                {
                    "$defs": definitions,
                    "$ref": f"#/$defs/{kind}",
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                }
            )
            failures = sorted(
                ".".join(str(part) for part in issue.absolute_path)
                + ": "
                + issue.message
                for issue in validator.iter_errors(value)
            )
            if failures:
                raise ProvenanceError(
                    f"CAS {kind} {digest} JSON Schema: {failures[0].lstrip(': ')}"
                )


def _validate_source_ir(cas: Cas, digest: str, source_hash: str) -> dict[str, Any]:
    source = _read_regular_file(cas.object_path(source_hash))
    value = _json_object(cas, digest, "source_ir")
    normalized = build_source_ir(
        source_sha256=source_hash,
        source=source,
        segments=value.get("segments"),
    )
    if value != normalized:
        raise ProvenanceError(f"source IR {digest} has unknown or inconsistent fields")
    if cas.index[digest]["links"] != [source_hash]:
        raise ProvenanceError(f"source IR {digest} must link only to its source bytes")
    return value


def _claims_by_id(ir: dict[str, Any]) -> dict[str, dict[str, Any]]:
    claims: dict[str, dict[str, Any]] = {}
    for segment in ir["segments"]:
        for claim in segment.get("claims", []):
            claim_id = claim["claim_id"]
            if claim_id in claims:
                raise ProvenanceError(f"duplicate claim ID in source IR: {claim_id}")
            claims[claim_id] = claim
    return claims


def _range_is_represented(start: int, end: int, ir: dict[str, Any]) -> bool:
    cursor = start
    for segment in ir["segments"]:
        if segment["disposition"] != "represented" or segment["end"] <= cursor:
            continue
        if segment["start"] > cursor:
            return False
        cursor = min(end, segment["end"])
        if cursor == end:
            return True
    return False


def _validate_source_entry(
    cas: Cas, source: Any, prefix: str
) -> tuple[
    set[str],
    dict[str, dict[str, dict[str, Any]]],
    dict[str, tuple[str, str]],
]:
    if not isinstance(source, dict) or set(source) != {
        "raw_source_sha256",
        "receipt_sha256",
        "representations",
        "source_ir_sha256",
    }:
        raise ProvenanceError(f"{prefix} must have the exact source schema")
    raw_hash = _require_hash(source["raw_source_sha256"], f"{prefix}.raw_source_sha256")
    receipt_hash = _require_hash(source["receipt_sha256"], f"{prefix}.receipt_sha256")
    ir_hash = _require_hash(source["source_ir_sha256"], f"{prefix}.source_ir_sha256")
    if "raw_source" not in cas.index.get(raw_hash, {}).get("kinds", []):
        raise ProvenanceError(f"{prefix} raw source is missing or has the wrong kind")
    if cas.index[raw_hash]["links"]:
        raise ProvenanceError(f"{prefix} raw source must not carry graph links")
    raw = _read_regular_file(cas.object_path(raw_hash))
    receipt_kinds = set(cas.index.get(receipt_hash, {}).get("kinds", []))
    if "fetch_receipt" in receipt_kinds:
        receipt = _json_object(cas, receipt_hash, "fetch_receipt")
        normalized_receipt = build_fetch_receipt(
            locator=receipt.get("locator"),
            final_locator=receipt.get("final_locator"),
            retrieved_at=receipt.get("retrieved_at"),
            status=receipt.get("status"),
            media_type=receipt.get("media_type"),
            body_sha256=receipt.get("body_sha256"),
            body_size=receipt.get("body_size"),
            headers=receipt.get("headers"),
        )
        if not 200 <= receipt["status"] <= 299:
            raise ProvenanceError(
                f"{prefix} unsuccessful receipt cannot ground a bundle"
            )
        is_external_authority = True
    elif "input_receipt" in receipt_kinds:
        receipt = _json_object(cas, receipt_hash, "input_receipt")
        normalized_receipt = build_input_receipt(
            repo_path=receipt.get("repo_path"),
            captured_at=receipt.get("captured_at"),
            body_sha256=receipt.get("body_sha256"),
            body_size=receipt.get("body_size"),
            body_git_sha1=receipt.get("body_git_sha1"),
        )
        if receipt["body_git_sha1"] != git_blob_sha1(raw):
            raise ProvenanceError(
                f"{prefix} input receipt has the wrong Git blob SHA-1"
            )
        is_external_authority = False
    else:
        raise ProvenanceError(f"{prefix} receipt has an unsupported kind")
    if receipt != normalized_receipt:
        raise ProvenanceError(f"{prefix} receipt has unknown or inconsistent fields")
    if receipt["body_sha256"] != raw_hash or receipt["body_size"] != len(raw):
        raise ProvenanceError(f"{prefix} receipt does not identify its exact raw bytes")
    if cas.index[receipt_hash]["links"] != [raw_hash]:
        raise ProvenanceError(f"{prefix} receipt must link only to its raw bytes")
    raw_ir = _validate_source_ir(cas, ir_hash, raw_hash)
    raw_claims = _claims_by_id(raw_ir)
    claims = {ir_hash: raw_claims}
    authorities = {raw_hash: (raw_hash, receipt_hash)}
    links = {raw_hash, receipt_hash, ir_hash}
    representations = source["representations"]
    if not isinstance(representations, list):
        raise ProvenanceError(f"{prefix}.representations must be an array")
    for index, representation in enumerate(representations):
        item_prefix = f"{prefix}.representations[{index}]"
        if not isinstance(representation, dict) or set(representation) != {
            "rendered_text_sha256",
            "source_ir_sha256",
            "transform_sha256",
        }:
            raise ProvenanceError(
                f"{item_prefix} must have the exact representation schema"
            )
        text_hash = _require_hash(
            representation["rendered_text_sha256"],
            f"{item_prefix}.rendered_text_sha256",
        )
        text_ir_hash = _require_hash(
            representation["source_ir_sha256"], f"{item_prefix}.source_ir_sha256"
        )
        transform_hash = _require_hash(
            representation["transform_sha256"], f"{item_prefix}.transform_sha256"
        )
        if "rendered_text" not in cas.index.get(text_hash, {}).get("kinds", []):
            raise ProvenanceError(f"{item_prefix} rendered text has the wrong kind")
        if cas.index[text_hash]["links"] != [raw_hash]:
            raise ProvenanceError(
                f"{item_prefix} rendered text must link to its raw source"
            )
        rendered = _read_regular_file(cas.object_path(text_hash))
        transform = _json_object(cas, transform_hash, "text_transform")
        normalized_transform = build_text_transform(
            source_sha256=raw_hash,
            source=raw,
            result_sha256=text_hash,
            result=rendered,
            operations=transform.get("operations"),
        )
        if transform != normalized_transform:
            raise ProvenanceError(f"{item_prefix} transform has inconsistent fields")
        for operation in transform["operations"]:
            if not _range_is_represented(
                operation["source_start"], operation["source_end"], raw_ir
            ):
                raise ProvenanceError(
                    f"{item_prefix} transform consumes raw bytes not represented by source IR"
                )
        if cas.index[transform_hash]["links"] != sorted([raw_hash, text_hash]):
            raise ProvenanceError(f"{item_prefix} transform links are inconsistent")
        text_ir = _validate_source_ir(cas, text_ir_hash, text_hash)
        text_claims = _claims_by_id(text_ir)
        missing_raw_claims = sorted(set(text_claims) - set(raw_claims))
        if missing_raw_claims:
            raise ProvenanceError(
                f"{item_prefix} rendered claims lack raw claims: "
                f"{', '.join(missing_raw_claims)}"
            )
        for claim_id, text_claim in text_claims.items():
            raw_claim = raw_claims[claim_id]
            claim_cursor = text_claim["start"]
            for operation in transform["operations"]:
                if operation["result_end"] <= claim_cursor:
                    continue
                if (
                    operation["result_start"] != claim_cursor
                    or operation["result_end"] > text_claim["end"]
                    or operation["source_start"] < raw_claim["start"]
                    or operation["source_end"] > raw_claim["end"]
                ):
                    raise ProvenanceError(
                        f"{item_prefix} transform does not map claim {claim_id} "
                        "to its corresponding raw claim bytes"
                    )
                claim_cursor = operation["result_end"]
                if claim_cursor == text_claim["end"]:
                    break
            if claim_cursor != text_claim["end"]:
                raise ProvenanceError(
                    f"{item_prefix} transform leaves claim {claim_id} bytes unmapped"
                )
        claims[text_ir_hash] = text_claims
        if is_external_authority:
            authorities[text_hash] = (raw_hash, receipt_hash)
        links.update((text_hash, text_ir_hash, transform_hash))
    return links, claims, authorities


def _validate_bundle(
    cas: Cas,
    digest: str,
    *,
    visiting: set[str],
    validated: set[str],
    snapshots: set[str],
    bundle_claims: dict[str, set[str]],
    bundle_inputs: dict[str, str],
) -> None:
    if digest in validated:
        return
    if digest in visiting:
        raise ProvenanceError(f"provenance bundle dependency cycle at {digest}")
    visiting.add(digest)
    bundle = _json_object(cas, digest, "provenance_bundle")
    if set(bundle) != {
        "bundle_id",
        "clauses",
        "dependencies",
        "input",
        "kind",
        "library",
        "sources",
    }:
        raise ProvenanceError(f"bundle {digest} has unknown or missing fields")
    if bundle["kind"] != "provenance_bundle":
        raise ProvenanceError(f"bundle {digest} payload has the wrong kind")
    _require_nonempty(bundle["bundle_id"], "bundle.bundle_id")
    _require_nonempty(bundle["library"], "bundle.library")
    dependencies = bundle["dependencies"]
    if not isinstance(dependencies, list) or dependencies != sorted(set(dependencies)):
        raise ProvenanceError(f"bundle {digest} dependencies must be sorted and unique")
    for dependency in dependencies:
        _require_hash(dependency, "bundle dependency")
        _validate_bundle(
            cas,
            dependency,
            visiting=visiting,
            validated=validated,
            snapshots=snapshots,
            bundle_claims=bundle_claims,
            bundle_inputs=bundle_inputs,
        )
    sources = bundle["sources"]
    if not isinstance(sources, list) or not sources:
        raise ProvenanceError(f"bundle {digest} must contain sources")
    expected_links: set[str] = set(dependencies)
    claims: dict[str, dict[str, dict[str, Any]]] = {}
    authorities: dict[str, tuple[str, str]] = {}
    source_identities: set[tuple[str, str, str]] = set()
    source_by_identity: dict[tuple[str, str, str], dict[str, Any]] = {}
    for index, source in enumerate(sources):
        links, source_claims, source_authorities = _validate_source_entry(
            cas, source, f"bundle.sources[{index}]"
        )
        expected_links.update(links)
        identity = (
            source["raw_source_sha256"],
            source["receipt_sha256"],
            source["source_ir_sha256"],
        )
        source_identities.add(identity)
        source_by_identity[identity] = source
        for ir_hash, ir_claims in source_claims.items():
            if ir_hash in claims:
                raise ProvenanceError(f"bundle {digest} repeats source IR {ir_hash}")
            claims[ir_hash] = ir_claims
        for snapshot_hash, authority in source_authorities.items():
            if snapshot_hash in authorities and authorities[snapshot_hash] != authority:
                raise ProvenanceError(
                    f"bundle {digest} gives snapshot {snapshot_hash} conflicting authorities"
                )
            authorities[snapshot_hash] = authority
    input_binding = bundle["input"]
    if not isinstance(input_binding, dict) or set(input_binding) != {
        "raw_source_sha256",
        "receipt_sha256",
        "source_ir_sha256",
    }:
        raise ProvenanceError(f"bundle {digest} input must have the exact schema")
    input_identity = (
        _require_hash(
            input_binding["raw_source_sha256"], "bundle.input.raw_source_sha256"
        ),
        _require_hash(input_binding["receipt_sha256"], "bundle.input.receipt_sha256"),
        _require_hash(
            input_binding["source_ir_sha256"], "bundle.input.source_ir_sha256"
        ),
    )
    if input_identity not in source_identities:
        raise ProvenanceError(f"bundle {digest} input is absent from its source graph")
    if source_by_identity[input_identity]["representations"]:
        raise ProvenanceError(f"bundle {digest} input must use its exact raw bytes")
    input_raw_hash, input_receipt_hash, input_ir_hash = input_identity
    input_receipt = _json_object(cas, input_receipt_hash, "input_receipt")
    if input_receipt["repo_path"] != bundle["library"]:
        raise ProvenanceError(f"bundle {digest} input path disagrees with its library")
    if cas.index[input_ir_hash]["links"] != [input_raw_hash]:
        raise ProvenanceError(
            f"bundle {digest} input IR does not describe its input bytes"
        )
    prior_input = bundle_inputs.get(bundle["library"])
    if prior_input is not None and prior_input != input_raw_hash:
        raise ProvenanceError(
            f"bundles disagree on input bytes for {bundle['library']}"
        )
    bundle_inputs[bundle["library"]] = input_raw_hash
    clauses = bundle["clauses"]
    if not isinstance(clauses, list) or not clauses:
        raise ProvenanceError(f"bundle {digest} must contain clauses")
    seen_clause_ids: set[str] = set()
    for index, clause in enumerate(clauses):
        prefix = f"bundle.clauses[{index}]"
        if not isinstance(clause, dict) or set(clause) != {
            "claim_id",
            "end",
            "input_claim",
            "locator",
            "quote",
            "quote_sha256",
            "resolution",
            "snapshot_sha256",
            "source_ir_sha256",
            "start",
        }:
            raise ProvenanceError(f"{prefix} must have the exact clause schema")
        claim_id = _require_nonempty(clause["claim_id"], f"{prefix}.claim_id")
        if claim_id in seen_clause_ids:
            raise ProvenanceError(f"bundle {digest} repeats clause {claim_id}")
        seen_clause_ids.add(claim_id)
        snapshot = _require_hash(clause["snapshot_sha256"], f"{prefix}.snapshot_sha256")
        ir_hash = _require_hash(
            clause["source_ir_sha256"], f"{prefix}.source_ir_sha256"
        )
        if snapshot not in cas.index or ir_hash not in claims:
            raise ProvenanceError(f"{prefix} points outside the bundle source graph")
        if cas.index[ir_hash]["links"] != [snapshot]:
            raise ProvenanceError(f"{prefix} source IR does not describe its snapshot")
        expected_claim = claims[ir_hash].get(claim_id)
        clause_claim = {
            key: clause[key]
            for key in ("claim_id", "end", "quote", "quote_sha256", "start")
        }
        if expected_claim != clause_claim:
            raise ProvenanceError(f"{prefix} disagrees with its byte-verified IR claim")
        input_claim = clause["input_claim"]
        if not isinstance(input_claim, dict) or set(input_claim) != {
            "end",
            "quote",
            "quote_sha256",
            "start",
        }:
            raise ProvenanceError(f"{prefix}.input_claim has the wrong schema")
        expected_input_claim = claims.get(input_ir_hash, {}).get(claim_id)
        normalized_input_claim = (
            {key: expected_input_claim[key] for key in input_claim}
            if expected_input_claim is not None
            else None
        )
        if input_claim != normalized_input_claim:
            raise ProvenanceError(
                f"{prefix} input claim disagrees with the decomposed ADJ bytes"
            )
        locator = _require_nonempty(clause["locator"], f"{prefix}.locator")
        if f'locator "{locator}"' not in input_claim["quote"]:
            raise ProvenanceError(
                f"{prefix} locator is absent from its ADJ input claim"
            )
        resolution = clause["resolution"]
        if not isinstance(resolution, dict):
            raise ProvenanceError(f"{prefix}.resolution must be an object")
        if resolution.get("kind") == "accepted_root":
            if set(resolution) != {
                "authority_receipt_sha256",
                "authority_source_sha256",
                "classification",
                "kind",
                "reason",
            }:
                raise ProvenanceError(
                    f"{prefix} accepted-root resolution has unknown fields"
                )
            if resolution["classification"] not in {
                "accepted_fact",
                "accepted_law",
                "primary_definition",
                "primary_measurement",
            }:
                raise ProvenanceError(
                    f"{prefix} accepted-root classification is unsupported"
                )
            _require_nonempty(resolution["reason"], f"{prefix}.resolution.reason")
            authority = (
                _require_hash(
                    resolution["authority_source_sha256"],
                    f"{prefix}.resolution.authority_source_sha256",
                ),
                _require_hash(
                    resolution["authority_receipt_sha256"],
                    f"{prefix}.resolution.authority_receipt_sha256",
                ),
            )
            if authorities.get(snapshot) != authority:
                raise ProvenanceError(
                    f"{prefix} accepted root does not name the snapshot authority"
                )
            receipt_kinds = cas.index[authority[1]]["kinds"]
            if "fetch_receipt" in receipt_kinds:
                authority_locator = _json_object(cas, authority[1], "fetch_receipt")[
                    "locator"
                ]
            else:
                authority_path = _json_object(cas, authority[1], "input_receipt")[
                    "repo_path"
                ]
                authority_locator = f"repo://{authority_path}"
            if locator != authority_locator:
                raise ProvenanceError(
                    f"{prefix} ADJ locator disagrees with its authority receipt"
                )
            if authority[0] == input_raw_hash:
                raise ProvenanceError(
                    f"{prefix} cannot use its own code bytes as an accepted root"
                )
        elif resolution.get("kind") == "dependency":
            if set(resolution) != {"bundle_sha256", "claim_id", "kind"}:
                raise ProvenanceError(
                    f"{prefix} dependency resolution has unknown fields"
                )
            dependency = _require_hash(
                resolution["bundle_sha256"], f"{prefix}.resolution.bundle_sha256"
            )
            if dependency not in dependencies:
                raise ProvenanceError(
                    f"{prefix} dependency is absent from bundle dependencies"
                )
            dependency_claim = _require_nonempty(
                resolution["claim_id"], f"{prefix}.resolution.claim_id"
            )
            if dependency_claim not in bundle_claims[dependency]:
                raise ProvenanceError(
                    f"{prefix} dependency does not export claim {dependency_claim}"
                )
        else:
            raise ProvenanceError(f"{prefix}.resolution kind is unsupported")
        expected_links.update((snapshot, ir_hash))
        snapshots.add(snapshot)
    if cas.index[digest]["links"] != sorted(expected_links):
        raise ProvenanceError(
            f"bundle {digest} CAS links disagree with its payload graph"
        )
    visiting.remove(digest)
    bundle_claims[digest] = seen_clause_ids
    validated.add(digest)


def validate_repository(
    cas_root: Path,
    manifest_path: Path,
    schema_path: Path | None = None,
    workspace_root: Path | None = None,
) -> dict[str, Any]:
    cas = Cas(cas_root)
    cas.load()
    _validate_index(cas)
    manifest_bytes = _read_regular_file(manifest_path)
    manifest = json.loads(manifest_bytes.decode("utf-8"))
    if not isinstance(manifest, dict) or manifest.get("schema_version") != 1:
        raise ProvenanceError("provenance manifest schema_version must equal 1")
    if manifest_bytes != canonical_json_bytes(manifest):
        raise ProvenanceError("provenance manifest is not canonical JSON")
    if schema_path is not None:
        _validate_manifest_schema(schema_path, manifest)
        _validate_cas_schemas(schema_path, cas)
    if set(manifest) != {"algorithm", "bundle_hashes", "manifest_id", "schema_version"}:
        raise ProvenanceError("provenance manifest has unknown or missing fields")
    _require_nonempty(manifest["manifest_id"], "manifest_id")
    if manifest["algorithm"] != "sha256":
        raise ProvenanceError("provenance manifest algorithm must equal sha256")
    bundles = manifest["bundle_hashes"]
    if not isinstance(bundles, list) or bundles != sorted(set(bundles)):
        raise ProvenanceError("bundle_hashes must be sorted and unique")
    snapshots: set[str] = set()
    validated: set[str] = set()
    bundle_claims: dict[str, set[str]] = {}
    bundle_inputs: dict[str, str] = {}
    for bundle in bundles:
        _validate_bundle(
            cas,
            _require_hash(bundle, "bundle hash"),
            visiting=set(),
            validated=validated,
            snapshots=snapshots,
            bundle_claims=bundle_claims,
            bundle_inputs=bundle_inputs,
        )
    effective_workspace_root = workspace_root or manifest_path.parent
    for repo_path, expected_hash in sorted(bundle_inputs.items()):
        input_bytes = _read_regular_file(
            effective_workspace_root / PurePosixPath(repo_path)
        )
        if sha256_bytes(input_bytes) != expected_hash:
            raise ProvenanceError(
                f"workspace input bytes disagree with bundle for {repo_path}"
            )
    reachable = _reachable(cas, bundles)
    unreferenced = sorted(set(cas.index) - reachable)
    if unreferenced:
        raise ProvenanceError(f"unreferenced CAS objects: {', '.join(unreferenced)}")
    return {
        "bundles": len(validated),
        "objects": len(cas.index),
        "snapshot_hashes": sorted(snapshots),
        "snapshots": len(snapshots),
        "valid": True,
    }


def project_snapshots(
    cas_root: Path,
    manifest_path: Path,
    output: Path,
    schema_path: Path | None = None,
    workspace_root: Path | None = None,
) -> dict[str, Any]:
    result = validate_repository(
        cas_root, manifest_path, schema_path, workspace_root=workspace_root
    )
    cas = Cas(cas_root)
    cas.load()
    _ensure_real_directory(output)
    for child in output.iterdir():
        if child.name not in result["snapshot_hashes"]:
            raise ProvenanceError(
                f"projection directory contains unexpected entry: {child.name}"
            )
    for digest in result["snapshot_hashes"]:
        destination = output / digest
        source = cas.object_path(digest)
        source_bytes = _read_regular_file(source)
        _write_exclusive(destination, source_bytes)
        if sha256_bytes(_read_regular_file(destination)) != digest:
            raise ProvenanceError(f"projection does not rehash to {digest}")
    return {
        **result,
        "output": str(output),
        "projected": len(result["snapshot_hashes"]),
    }


def _load_headers(path: Path | None) -> dict[str, str]:
    if path is None:
        return {}
    value = json.loads(_read_regular_file(path).decode("utf-8"))
    if not isinstance(value, dict) or not all(
        isinstance(key, str) and isinstance(item, str) for key, item in value.items()
    ):
        raise ProvenanceError(
            "headers file must contain a string-to-string JSON object"
        )
    return value


def _resolve(root: Path, value: Path) -> Path:
    return value if value.is_absolute() else root / value


def _bundle_declared_links(bundle: dict[str, Any]) -> list[str]:
    links = set(bundle.get("dependencies", []))
    for source in bundle.get("sources", []):
        links.update(
            (
                source.get("raw_source_sha256"),
                source.get("receipt_sha256"),
                source.get("source_ir_sha256"),
            )
        )
        for representation in source.get("representations", []):
            links.update(
                (
                    representation.get("rendered_text_sha256"),
                    representation.get("source_ir_sha256"),
                    representation.get("transform_sha256"),
                )
            )
    for clause in bundle.get("clauses", []):
        links.update((clause.get("snapshot_sha256"), clause.get("source_ir_sha256")))
    return sorted(_require_hash(link, "bundle link") for link in links)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    verify_parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    verify_parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)

    project_parser = subparsers.add_parser("project")
    project_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    project_parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    project_parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    project_parser.add_argument("--output", type=Path, required=True)

    capture_parser = subparsers.add_parser("capture")
    capture_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    capture_parser.add_argument("--body", type=Path, required=True)
    capture_parser.add_argument("--locator", required=True)
    capture_parser.add_argument("--final-locator")
    capture_parser.add_argument("--retrieved-at", required=True)
    capture_parser.add_argument("--status", type=int, required=True)
    capture_parser.add_argument("--media-type", required=True)
    capture_parser.add_argument("--headers", type=Path)
    capture_parser.add_argument("--label", required=True)

    input_parser = subparsers.add_parser("capture-input")
    input_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    input_parser.add_argument("--body", type=Path, required=True)
    input_parser.add_argument("--repo-path", required=True)
    input_parser.add_argument("--captured-at", required=True)
    input_parser.add_argument("--label", required=True)

    rendered_parser = subparsers.add_parser("put-rendered")
    rendered_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    rendered_parser.add_argument("--source", required=True)
    rendered_parser.add_argument("--body", type=Path, required=True)
    rendered_parser.add_argument("--label", required=True)

    transform_parser = subparsers.add_parser("put-transform")
    transform_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    transform_parser.add_argument("--source", required=True)
    transform_parser.add_argument("--result", required=True)
    transform_parser.add_argument("--operations", type=Path, required=True)
    transform_parser.add_argument("--label", required=True)

    ir_parser = subparsers.add_parser("put-ir")
    ir_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    ir_parser.add_argument("--source", required=True)
    ir_parser.add_argument("--segments", type=Path, required=True)
    ir_parser.add_argument("--label", required=True)

    bundle_parser = subparsers.add_parser("put-bundle")
    bundle_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    bundle_parser.add_argument("--bundle", type=Path, required=True)
    bundle_parser.add_argument("--label", required=True)

    args = parser.parse_args()
    repo_root = args.repo_root.resolve()
    try:
        if args.command == "verify":
            result = validate_repository(
                _resolve(repo_root, args.cas),
                _resolve(repo_root, args.manifest),
                _resolve(repo_root, args.schema),
                workspace_root=repo_root,
            )
        elif args.command == "project":
            result = project_snapshots(
                _resolve(repo_root, args.cas),
                _resolve(repo_root, args.manifest),
                _resolve(repo_root, args.output),
                _resolve(repo_root, args.schema),
                workspace_root=repo_root,
            )
        elif args.command == "capture":
            cas = Cas(_resolve(repo_root, args.cas))
            cas.load()
            body = _read_regular_file(_resolve(repo_root, args.body))
            raw_hash = cas.put(body, kind="raw_source", label=args.label)
            receipt = build_fetch_receipt(
                locator=args.locator,
                final_locator=args.final_locator or args.locator,
                retrieved_at=args.retrieved_at,
                status=args.status,
                media_type=args.media_type,
                body_sha256=raw_hash,
                body_size=len(body),
                headers=_load_headers(
                    _resolve(repo_root, args.headers)
                    if args.headers is not None
                    else None
                ),
            )
            receipt_hash = cas.put_json(
                receipt,
                kind="fetch_receipt",
                label=f"fetch receipt: {args.label}",
                links=[raw_hash],
            )
            cas.write_index()
            result = {"raw_source_sha256": raw_hash, "receipt_sha256": receipt_hash}
        elif args.command == "capture-input":
            cas = Cas(_resolve(repo_root, args.cas))
            cas.load()
            repo_path = _require_repo_path(args.repo_path, "receipt.repo_path")
            body_path = _resolve(repo_root, args.body)
            expected_path = repo_root / PurePosixPath(repo_path)
            if os.path.abspath(body_path) != os.path.abspath(expected_path):
                raise ProvenanceError(
                    "capture-input body must be the file named by --repo-path"
                )
            body = _read_regular_file(body_path)
            raw_hash = cas.put(body, kind="raw_source", label=args.label)
            receipt = build_input_receipt(
                repo_path=repo_path,
                captured_at=args.captured_at,
                body_sha256=raw_hash,
                body_size=len(body),
                body_git_sha1=git_blob_sha1(body),
            )
            receipt_hash = cas.put_json(
                receipt,
                kind="input_receipt",
                label=f"input receipt: {args.label}",
                links=[raw_hash],
            )
            cas.write_index()
            result = {"raw_source_sha256": raw_hash, "receipt_sha256": receipt_hash}
        elif args.command == "put-rendered":
            cas = Cas(_resolve(repo_root, args.cas))
            cas.load()
            source_hash = _require_hash(args.source, "source hash")
            if "raw_source" not in cas.index.get(source_hash, {}).get("kinds", []):
                raise ProvenanceError("source hash is not a raw_source CAS object")
            rendered = _read_regular_file(_resolve(repo_root, args.body))
            rendered_hash = cas.put(
                rendered,
                kind="rendered_text",
                label=args.label,
                links=[source_hash],
            )
            cas.write_index()
            result = {"rendered_text_sha256": rendered_hash}
        elif args.command == "put-ir":
            cas = Cas(_resolve(repo_root, args.cas))
            cas.load()
            source_hash = _require_hash(args.source, "source hash")
            if not {
                "raw_source",
                "rendered_text",
            }.intersection(cas.index.get(source_hash, {}).get("kinds", [])):
                raise ProvenanceError("source hash is not source bytes in the CAS")
            segments = json.loads(
                _read_regular_file(_resolve(repo_root, args.segments)).decode("utf-8")
            )
            source_bytes = _read_regular_file(cas.object_path(source_hash))
            ir = build_source_ir(
                source_sha256=source_hash,
                source=source_bytes,
                segments=segments,
            )
            ir_hash = cas.put_json(
                ir, kind="source_ir", label=args.label, links=[source_hash]
            )
            cas.write_index()
            result = {"source_ir_sha256": ir_hash}
        elif args.command == "put-transform":
            cas = Cas(_resolve(repo_root, args.cas))
            cas.load()
            source_hash = _require_hash(args.source, "source hash")
            result_hash = _require_hash(args.result, "result hash")
            if "raw_source" not in cas.index.get(source_hash, {}).get("kinds", []):
                raise ProvenanceError("transform source is not raw_source bytes")
            if "rendered_text" not in cas.index.get(result_hash, {}).get("kinds", []):
                raise ProvenanceError("transform result is not rendered_text bytes")
            operations = json.loads(
                _read_regular_file(_resolve(repo_root, args.operations)).decode("utf-8")
            )
            transform = build_text_transform(
                source_sha256=source_hash,
                source=_read_regular_file(cas.object_path(source_hash)),
                result_sha256=result_hash,
                result=_read_regular_file(cas.object_path(result_hash)),
                operations=operations,
            )
            transform_hash = cas.put_json(
                transform,
                kind="text_transform",
                label=args.label,
                links=[source_hash, result_hash],
            )
            cas.write_index()
            result = {"transform_sha256": transform_hash}
        else:
            cas = Cas(_resolve(repo_root, args.cas))
            cas.load()
            bundle = json.loads(
                _read_regular_file(_resolve(repo_root, args.bundle)).decode("utf-8")
            )
            if (
                not isinstance(bundle, dict)
                or bundle.get("kind") != "provenance_bundle"
            ):
                raise ProvenanceError(
                    "bundle file must contain a provenance_bundle object"
                )
            bundle_hash = cas.put_json(
                bundle,
                kind="provenance_bundle",
                label=args.label,
                links=_bundle_declared_links(bundle),
            )
            cas.write_index()
            result = {"bundle_sha256": bundle_hash}
    except (
        OSError,
        UnicodeDecodeError,
        json.JSONDecodeError,
        ProvenanceError,
    ) as error:
        print(
            json.dumps({"error": str(error), "valid": False}, indent=2, sort_keys=True)
        )
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
