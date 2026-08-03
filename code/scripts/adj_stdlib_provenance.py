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
import subprocess
import tempfile
import threading
import xml.etree.ElementTree as ET
from collections.abc import Iterable, Sequence
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
    "execution_witness",
    "fetch_receipt",
    "formula_derivation",
    "formula_parser_inventory",
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


def _mathml_to_infix(source: bytes, prefix: str) -> bytes:
    upper_source = source.upper()
    if b"<!DOCTYPE" in upper_source or b"<!ENTITY" in upper_source:
        raise ProvenanceError(f"{prefix} MathML declarations are forbidden")
    try:
        root = ET.fromstring(source.decode("utf-8"))
    except (UnicodeDecodeError, ET.ParseError) as error:
        raise ProvenanceError(f"{prefix} source is not valid UTF-8 MathML") from error

    def render(node: ET.Element) -> str:
        tag = node.tag.rsplit("}", 1)[-1]
        children = list(node)
        for child in children:
            if child.tail is not None and child.tail.strip():
                raise ProvenanceError(f"{prefix} MathML contains mixed tail text")
        if tag == "math":
            if node.text is not None and node.text.strip():
                raise ProvenanceError(f"{prefix} MathML contains mixed root text")
            if len(children) != 1:
                raise ProvenanceError(f"{prefix} MathML math root is ambiguous")
            return render(children[0])
        if tag == "semantics":
            if node.text is not None and node.text.strip():
                raise ProvenanceError(f"{prefix} MathML contains mixed semantics text")
            if not children:
                raise ProvenanceError(f"{prefix} MathML semantics is empty")
            alternatives = [render(child) for child in children]
            if any(value != alternatives[0] for value in alternatives[1:]):
                raise ProvenanceError(f"{prefix} MathML semantic branches disagree")
            return alternatives[0]
        if tag in {"mrow", "annotation-xml"}:
            if node.text is not None and node.text.strip():
                raise ProvenanceError(f"{prefix} MathML contains mixed container text")
            return "".join(render(child) for child in children)
        if tag in {"mi", "mn", "mo"}:
            if children or node.text is None:
                raise ProvenanceError(f"{prefix} MathML token is not canonical")
            return node.text.replace("×", "*")
        if tag == "mfrac":
            if len(children) != 2:
                raise ProvenanceError(
                    f"{prefix} MathML fraction must have two operands"
                )
            return f"({render(children[0])}/{render(children[1])})"
        raise ProvenanceError(f"{prefix} MathML element {tag!r} is unsupported")

    return render(root).encode("utf-8")


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
    explicit_source_partition = any(
        isinstance(operation, dict) and operation.get("operation") == "discard"
        for operation in operations
    )
    result_cursor = 0
    source_cursor = 0
    for index, operation in enumerate(operations):
        prefix = f"transform.operations[{index}]"
        if not isinstance(operation, dict):
            raise ProvenanceError(f"{prefix} must be an object")
        operation_name = operation.get("operation")
        expected_fields = {
            "operation",
            "result_end",
            "result_start",
            "source_end",
            "source_start",
        }
        if operation_name == "discard":
            expected_fields.update(("claim_id", "reason"))
        if set(operation) != expected_fields:
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
        invalid_mapping = (
            source_start < source_cursor
            or (
                explicit_source_partition
                and index > 0
                and source_start != source_cursor
            )
            or source_end <= source_start
            or source_end > len(source)
            or result_start != result_cursor
            or result_end > len(result)
        )
        if operation_name == "discard":
            invalid_mapping = invalid_mapping or result_end != result_start
        else:
            invalid_mapping = invalid_mapping or result_end <= result_start
        if invalid_mapping:
            raise ProvenanceError(f"{prefix} has a non-canonical byte mapping")
        source_slice = source[source_start:source_end]
        if operation_name == "copy":
            expected = source_slice
        elif operation_name == "html_entity_decode":
            try:
                expected = html.unescape(source_slice.decode("utf-8")).encode("utf-8")
            except UnicodeDecodeError as error:
                raise ProvenanceError(f"{prefix} source is not UTF-8") from error
        elif operation_name == "mathml_to_infix":
            expected = _mathml_to_infix(source_slice, prefix)
        elif operation_name == "discard":
            _require_nonempty(operation["claim_id"], f"{prefix}.claim_id")
            _require_nonempty(operation["reason"], f"{prefix}.reason")
            expected = b""
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


def _run_formula_inventory(
    parser_command: Sequence[str], source_path: Path
) -> dict[str, Any]:
    if not parser_command:
        raise ProvenanceError("formula inventory parser command must not be empty")
    try:
        process = subprocess.Popen(
            [*parser_command, os.fspath(source_path)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as error:
        raise ProvenanceError(
            f"formula inventory parser failed to run: {error}"
        ) from error
    stdout = bytearray()
    stderr = bytearray()
    overflow = threading.Event()

    def drain(stream: Any, output: bytearray, limit: int) -> None:
        while True:
            chunk = stream.read(65536)
            if not chunk:
                return
            remaining = limit - len(output)
            output.extend(chunk[:remaining])
            if len(chunk) > remaining:
                overflow.set()
                process.kill()
                return

    stdout_thread = threading.Thread(
        target=drain,
        args=(process.stdout, stdout, MAX_OBJECT_BYTES),
        daemon=True,
    )
    stderr_thread = threading.Thread(
        target=drain,
        args=(process.stderr, stderr, 4096),
        daemon=True,
    )
    stdout_thread.start()
    stderr_thread.start()
    try:
        returncode = process.wait(timeout=60)
    except subprocess.TimeoutExpired as error:
        process.kill()
        process.wait()
        raise ProvenanceError(
            "formula inventory parser timed out after 60 seconds"
        ) from error
    finally:
        stdout_thread.join()
        stderr_thread.join()
        process.stdout.close()
        process.stderr.close()
    if overflow.is_set():
        raise ProvenanceError("formula inventory parser output exceeds byte limit")
    if returncode != 0:
        detail = bytes(stderr).decode("utf-8", errors="replace").strip()
        raise ProvenanceError(f"formula inventory parser exited {returncode}: {detail}")
    try:
        value = json.loads(bytes(stdout).decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProvenanceError(
            "formula inventory parser did not emit UTF-8 JSON"
        ) from error
    if not isinstance(value, dict):
        raise ProvenanceError("formula inventory parser output must be an object")
    if bytes(stdout) != canonical_json_bytes(value):
        raise ProvenanceError("formula inventory parser output is not canonical JSON")
    return value


def _validate_formula_inventory_value(
    value: Any, source_hash: str, source: bytes
) -> None:
    if not isinstance(value, dict) or set(value) != {
        "formulas",
        "kind",
        "parser_contract",
        "schema_version",
        "scope",
        "source_sha256",
        "source_size",
    }:
        raise ProvenanceError("formula parser inventory has unknown or missing fields")
    if (
        value["kind"] != "formula_parser_inventory"
        or value["parser_contract"] != "adj-lang/formula_source_map/v1"
        or not _is_integer(value["schema_version"])
        or value["schema_version"] != 1
        or value["scope"] != "source_file"
        or _require_hash(
            value["source_sha256"], "formula parser inventory source_sha256"
        )
        != source_hash
        or not _is_integer(value["source_size"])
        or value["source_size"] != len(source)
    ):
        raise ProvenanceError(
            "formula parser inventory contract or source binding disagrees"
        )
    formulas = value["formulas"]
    if not isinstance(formulas, list):
        raise ProvenanceError("formula parser inventory formulas must be an array")
    previous_end = 0
    seen_names: set[str] = set()
    for index, formula in enumerate(formulas):
        prefix = f"formula parser inventory formulas[{index}]"
        if not isinstance(formula, dict) or set(formula) != {
            "body",
            "declaration",
            "formula",
            "formulabook",
            "parameters",
            "step_count",
        }:
            raise ProvenanceError(f"{prefix} has unknown or missing fields")
        formula_name = _require_nonempty(formula["formula"], f"{prefix}.formula")
        if formula_name in seen_names:
            raise ProvenanceError(
                f"formula parser inventory repeats formula name {formula_name}"
            )
        seen_names.add(formula_name)
        _require_nonempty(formula["formulabook"], f"{prefix}.formulabook")
        parameters = formula["parameters"]
        if not isinstance(parameters, list) or any(
            not isinstance(parameter, str) or not parameter for parameter in parameters
        ):
            raise ProvenanceError(f"{prefix}.parameters must contain non-empty strings")
        if not _is_integer(formula["step_count"]) or formula["step_count"] < 0:
            raise ProvenanceError(f"{prefix}.step_count must be a non-negative integer")
        spans: dict[str, tuple[int, int]] = {}
        for span_name in ("body", "declaration"):
            span = formula[span_name]
            if not isinstance(span, dict) or set(span) != {"end", "sha256", "start"}:
                raise ProvenanceError(f"{prefix}.{span_name} has the wrong schema")
            start = span["start"]
            end = span["end"]
            if (
                not _is_integer(start)
                or not _is_integer(end)
                or start < 0
                or end <= start
                or end > len(source)
            ):
                raise ProvenanceError(f"{prefix}.{span_name} is outside source bytes")
            if _require_hash(
                span["sha256"], f"{prefix}.{span_name}.sha256"
            ) != sha256_bytes(source[start:end]):
                raise ProvenanceError(f"{prefix}.{span_name} byte hash disagrees")
            spans[span_name] = (start, end)
        body_start, body_end = spans["body"]
        declaration_start, declaration_end = spans["declaration"]
        if not (
            declaration_start <= body_start
            and body_end <= declaration_end
            and declaration_start >= previous_end
        ):
            raise ProvenanceError(
                f"{prefix} body/declaration containment or parser order disagrees"
            )
        previous_end = declaration_end


def put_formula_parser_inventory(
    cas: Cas,
    source_hash: str,
    parser_command: Sequence[str],
    *,
    label: str,
) -> str:
    source_hash = _require_hash(source_hash, "formula inventory source hash")
    if "raw_source" not in cas.index.get(source_hash, {}).get("kinds", []):
        raise ProvenanceError("formula inventory source is not raw_source bytes")
    source = _read_regular_file(cas.object_path(source_hash))
    inventory = _run_formula_inventory(parser_command, cas.object_path(source_hash))
    _validate_formula_inventory_value(inventory, source_hash, source)
    return cas.put_json(
        inventory,
        kind="formula_parser_inventory",
        label=label,
        links=[source_hash],
    )


def _validate_formula_inventory(
    cas: Cas,
    digest: str,
    source_hash: str,
    input_claims: dict[str, dict[str, Any]],
    parser_command: Sequence[str] | None,
) -> dict[str, Any]:
    source = _read_regular_file(cas.object_path(source_hash))
    value = _json_object(cas, digest, "formula_parser_inventory")
    _validate_formula_inventory_value(value, source_hash, source)
    if cas.index[digest]["links"] != [source_hash]:
        raise ProvenanceError(
            "formula parser inventory must link only to its source bytes"
        )
    if parser_command is None:
        raise ProvenanceError(
            "formula inventory replay requires --formula-inventory-binary"
        )
    replayed = _run_formula_inventory(parser_command, cas.object_path(source_hash))
    if replayed != value:
        raise ProvenanceError(
            "stored formula parser inventory disagrees with parser replay"
        )
    selected_claims: set[str] = set()
    for index, formula in enumerate(value["formulas"]):
        declaration = formula["declaration"]
        enclosing = [
            claim_id
            for claim_id, claim in input_claims.items()
            if claim["start"] <= declaration["start"]
            and declaration["end"] <= claim["end"]
        ]
        if len(enclosing) != 1:
            raise ProvenanceError(
                f"formula parser inventory formulas[{index}] declaration must be "
                "enclosed by exactly one input IR claim"
            )
        if enclosing[0] in selected_claims:
            raise ProvenanceError(
                f"input IR claim {enclosing[0]} cannot ground more than one formula"
            )
        selected_claims.add(enclosing[0])
    return value


def _run_json_command(
    command: Sequence[str], arguments: Sequence[str], *, label: str
) -> dict[str, Any]:
    if not command:
        raise ProvenanceError(f"{label} command must not be empty")
    try:
        process = subprocess.Popen(
            [*command, *arguments],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as error:
        raise ProvenanceError(f"{label} failed to run: {error}") from error
    stdout = bytearray()
    stderr = bytearray()
    overflow = threading.Event()

    def drain(stream: Any, output: bytearray, limit: int) -> None:
        while True:
            chunk = stream.read(65536)
            if not chunk:
                return
            remaining = limit - len(output)
            output.extend(chunk[:remaining])
            if len(chunk) > remaining:
                overflow.set()
                process.kill()
                return

    threads = (
        threading.Thread(
            target=drain,
            args=(process.stdout, stdout, MAX_OBJECT_BYTES),
            daemon=True,
        ),
        threading.Thread(
            target=drain, args=(process.stderr, stderr, 4096), daemon=True
        ),
    )
    for thread in threads:
        thread.start()
    try:
        returncode = process.wait(timeout=60)
    except subprocess.TimeoutExpired as error:
        process.kill()
        process.wait()
        raise ProvenanceError(f"{label} timed out after 60 seconds") from error
    finally:
        for thread in threads:
            thread.join()
        process.stdout.close()
        process.stderr.close()
    if overflow.is_set():
        raise ProvenanceError(f"{label} output exceeds byte limit")
    if returncode != 0:
        detail = bytes(stderr).decode("utf-8", errors="replace").strip()
        raise ProvenanceError(f"{label} exited {returncode}: {detail}")
    try:
        value = json.loads(bytes(stdout).decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProvenanceError(f"{label} did not emit UTF-8 JSON") from error
    if not isinstance(value, dict):
        raise ProvenanceError(f"{label} output must be an object")
    if bytes(stdout) != canonical_json_bytes(value):
        raise ProvenanceError(f"{label} output is not canonical JSON")
    return value


def _bundle_closure(cas: Cas, root: dict[str, Any]) -> list[tuple[str | None, dict[str, Any]]]:
    closure: list[tuple[str | None, dict[str, Any]]] = [(None, root)]
    seen: set[str] = set()
    pending = list(root["dependencies"])
    while pending:
        digest = pending.pop()
        if digest in seen:
            continue
        seen.add(digest)
        bundle = _json_object(cas, digest, "provenance_bundle")
        closure.append((digest, bundle))
        pending.extend(bundle["dependencies"])
    return closure


def _direct_formula_inventory(
    cas: Cas, query_bundle: dict[str, Any]
) -> tuple[str, str, dict[str, Any]]:
    dependencies = query_bundle.get("dependencies")
    if not isinstance(dependencies, list) or len(dependencies) != 1:
        raise ProvenanceError("execution witness requires exactly one formula dependency")
    bundle_hash = _require_hash(dependencies[0], "execution witness formula bundle")
    bundle = _json_object(cas, bundle_hash, "provenance_bundle")
    inventory_hash = _require_hash(
        bundle.get("formula_inventory_sha256"),
        "execution witness formula inventory",
    )
    inventory = _json_object(cas, inventory_hash, "formula_parser_inventory")
    return bundle_hash, inventory_hash, inventory


def _materialize_formula_audit(
    cas: Cas,
    query_bundle: dict[str, Any],
    audit_command: Sequence[str],
) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="adj-formula-audit-") as directory:
        root = Path(directory)
        snapshots = root / "snapshots"
        _ensure_real_directory(snapshots)
        for _digest, bundle in _bundle_closure(cas, query_bundle):
            library = PurePosixPath(bundle["library"])
            if (
                library.is_absolute()
                or ".." in library.parts
                or library.as_posix() != bundle["library"]
                or not library.parts
                or library.parts[0] != "code"
            ):
                raise ProvenanceError("formula audit library path escapes workspace")
            source_hash = _require_hash(
                bundle["input"]["raw_source_sha256"], "formula audit input source"
            )
            destination = root / Path(*library.parts)
            _write_exclusive(destination, _read_regular_file(cas.object_path(source_hash)))
            for clause in bundle["clauses"]:
                snapshot_hash = _require_hash(
                    clause["snapshot_sha256"], "formula audit snapshot"
                )
                _write_exclusive(
                    snapshots / snapshot_hash,
                    _read_regular_file(cas.object_path(snapshot_hash)),
                )
        query_path = root / Path(*PurePosixPath(query_bundle["library"]).parts)
        audit = _run_json_command(
            audit_command,
            ["--snapshots", os.fspath(snapshots), os.fspath(query_path)],
            label="formula audit",
        )
    if set(audit) != {
        "contract",
        "derivations",
        "imports",
        "kind",
        "root_source_sha256",
        "schema_version",
    } or (
        audit["contract"] != "adj-lang/formula_audit/v1"
        or audit["kind"] != "formula_execution_audit"
        or audit["schema_version"] != 1
        or audit["root_source_sha256"]
        != query_bundle["input"]["raw_source_sha256"]
        or not isinstance(audit["derivations"], list)
        or not isinstance(audit["imports"], list)
    ):
        raise ProvenanceError("formula audit contract or query binding disagrees")
    return audit


def _enclosing_claim(
    cas: Cas, source_hash: str, source_ir_hash: str, span: dict[str, Any], label: str
) -> tuple[str, dict[str, Any]]:
    ir = _validate_source_ir(cas, source_ir_hash, source_hash)
    matches = [
        (claim_id, claim)
        for claim_id, claim in _claims_by_id(ir).items()
        if claim["start"] <= span.get("start", -1)
        and span.get("end", -1) <= claim["end"]
    ]
    if len(matches) != 1:
        raise ProvenanceError(f"{label} must be enclosed by exactly one IR claim")
    return matches[0]


def _execution_graph(
    cas: Cas, query_bundle: dict[str, Any]
) -> tuple[dict[str, tuple[str | None, dict[str, Any]]], list[tuple[str, dict[str, Any]]]]:
    by_source: dict[str, tuple[str | None, dict[str, Any]]] = {}
    formula_bundles: list[tuple[str, dict[str, Any]]] = []
    for digest, bundle in _bundle_closure(cas, query_bundle):
        source_hash = _require_hash(
            bundle["input"]["raw_source_sha256"], "execution graph source"
        )
        if source_hash in by_source:
            raise ProvenanceError("execution graph repeats one program source")
        by_source[source_hash] = (digest, bundle)
        if digest is not None and "formula_inventory_sha256" in bundle:
            formula_bundles.append((digest, bundle))
    return by_source, formula_bundles


def _formula_reference(
    cas: Cas,
    identity: Any,
    formula_bundles: list[tuple[str, dict[str, Any]]],
) -> tuple[dict[str, Any], set[str]]:
    if not isinstance(identity, dict) or set(identity) != {
        "body",
        "declaration",
        "formulabook",
        "name",
        "parameters",
        "source_sha256",
    }:
        raise ProvenanceError("formula audit export identity has the wrong schema")
    source_hash = _require_hash(identity["source_sha256"], "formula identity source")
    matches: list[tuple[str, dict[str, Any], str, dict[str, Any]]] = []
    for bundle_hash, bundle in formula_bundles:
        if bundle["input"]["raw_source_sha256"] != source_hash:
            continue
        inventory_hash = bundle["formula_inventory_sha256"]
        inventory = _json_object(cas, inventory_hash, "formula_parser_inventory")
        for formula in inventory["formulas"]:
            if (
                formula["body"] == identity["body"]
                and formula["declaration"] == identity["declaration"]
                and formula["formulabook"] == identity["formulabook"]
                and formula["formula"] == identity["name"]
                and formula["parameters"] == identity["parameters"]
            ):
                matches.append((bundle_hash, bundle, inventory_hash, formula))
    if len(matches) != 1:
        raise ProvenanceError("formula audit identity must resolve to exactly one export")
    bundle_hash, bundle, inventory_hash, _formula = matches[0]
    source_ir_hash = bundle["input"]["source_ir_sha256"]
    claim_id, _claim = _enclosing_claim(
        cas, source_hash, source_ir_hash, identity["declaration"], "formula declaration"
    )
    clauses = [clause for clause in bundle["clauses"] if clause["claim_id"] == claim_id]
    if len(clauses) != 1:
        raise ProvenanceError("formula export must resolve to exactly one bundle clause")
    clause = clauses[0]
    reference = {
        "body": identity["body"],
        "bundle_sha256": bundle_hash,
        "claim_id": claim_id,
        "declaration": identity["declaration"],
        "formula": identity["name"],
        "formulabook": identity["formulabook"],
        "inventory_sha256": inventory_hash,
        "parameters": identity["parameters"],
        "snapshot_sha256": clause["snapshot_sha256"],
        "source_ir_sha256": source_ir_hash,
        "source_sha256": source_hash,
    }
    return reference, {
        bundle_hash,
        inventory_hash,
        source_hash,
        source_ir_hash,
        clause["snapshot_sha256"],
        clause["source_ir_sha256"],
    }


def _question_reference(
    cas: Cas, query_bundle: dict[str, Any], question: Any
) -> tuple[dict[str, Any], set[str]]:
    if not isinstance(question, dict) or set(question) != {
        "declaration",
        "name",
        "source_sha256",
    }:
        raise ProvenanceError("formula audit question has the wrong schema")
    source_hash = query_bundle["input"]["raw_source_sha256"]
    source_ir_hash = query_bundle["input"]["source_ir_sha256"]
    if question["source_sha256"] != source_hash:
        raise ProvenanceError("formula audit question names the wrong source")
    claim_id, _claim = _enclosing_claim(
        cas,
        source_hash,
        source_ir_hash,
        question["declaration"],
        "formula question",
    )
    return {
        **question,
        "claim_id": claim_id,
        "source_ir_sha256": source_ir_hash,
    }, {source_hash, source_ir_hash}


def _input_reference(
    query_bundle: dict[str, Any], identity: Any
) -> tuple[dict[str, Any], set[str]]:
    if not isinstance(identity, dict) or set(identity) != {"provenance", "term"}:
        raise ProvenanceError("formula audit input identity has the wrong schema")
    provenance = identity["provenance"]
    quote = provenance.get("quote") if isinstance(provenance, dict) else None
    if not isinstance(quote, dict):
        raise ProvenanceError("formula audit input has no byte quote identity")
    matches = [
        clause
        for clause in query_bundle["clauses"]
        if clause["snapshot_sha256"] == quote.get("snapshot_sha256")
        and clause["start"] == quote.get("byte_offset")
        and clause["end"] - clause["start"] == quote.get("byte_len")
        and clause["quote_sha256"] == quote.get("text_sha256")
    ]
    if len(matches) != 1:
        raise ProvenanceError("formula audit input must resolve to exactly one query claim")
    clause = matches[0]
    return {
        "claim_id": clause["claim_id"],
        "identity": identity,
        "source_ir_sha256": clause["source_ir_sha256"],
    }, {clause["snapshot_sha256"], clause["source_ir_sha256"]}


def _normalized_formula_evidence(
    cas: Cas, query_bundle: dict[str, Any], audit: dict[str, Any]
) -> list[tuple[dict[str, Any], set[str], dict[str, Any], set[str]]]:
    by_source, formula_bundles = _execution_graph(cas, query_bundle)
    imports: list[dict[str, Any]] = []
    import_links: set[str] = set()
    for item in audit["imports"]:
        if not isinstance(item, dict) or set(item) != {
            "declaration",
            "imported_source_sha256",
            "importer_source_sha256",
            "literal",
        }:
            raise ProvenanceError("formula audit import has the wrong schema")
        importer = by_source.get(item["importer_source_sha256"])
        imported = by_source.get(item["imported_source_sha256"])
        if importer is None or imported is None or imported[0] is None:
            raise ProvenanceError("formula audit import escapes the CAS bundle graph")
        importer_ir = importer[1]["input"]["source_ir_sha256"]
        claim_id, _claim = _enclosing_claim(
            cas,
            item["importer_source_sha256"],
            importer_ir,
            item["declaration"],
            "formula import",
        )
        imports.append(
            {
                **item,
                "claim_id": claim_id,
                "imported_bundle_sha256": imported[0],
                "importer_source_ir_sha256": importer_ir,
            }
        )
        import_links.update(
            {
                item["importer_source_sha256"],
                importer_ir,
                item["imported_source_sha256"],
                imported[1]["input"]["source_ir_sha256"],
                imported[0],
            }
        )
    imports.sort(key=lambda item: (item["importer_source_sha256"], item["declaration"]["start"]))

    normalized = []
    seen_exports: set[bytes] = set()
    for item in audit["derivations"]:
        if not isinstance(item, dict) or set(item) != {
            "export",
            "formula_sequence",
            "inputs",
            "plan",
            "question",
            "result",
            "tree",
            "verification",
        }:
            raise ProvenanceError("formula audit derivation has the wrong schema")
        export, export_links = _formula_reference(cas, item["export"], formula_bundles)
        export_key = canonical_json_bytes(export)
        if export_key in seen_exports:
            raise ProvenanceError("formula audit repeats one export derivation")
        seen_exports.add(export_key)
        sequence = []
        sequence_links: set[str] = set()
        for identity in item["formula_sequence"]:
            reference, links = _formula_reference(cas, identity, formula_bundles)
            sequence.append(reference)
            sequence_links.update(links)
        if not sequence or sequence[0] != export:
            raise ProvenanceError("formula audit sequence does not begin with its export")
        question, question_links = _question_reference(cas, query_bundle, item["question"])
        inputs = []
        input_links: set[str] = set()
        for identity in item["inputs"]:
            reference, links = _input_reference(query_bundle, identity)
            inputs.append(reference)
            input_links.update(links)
        verification = item["verification"]
        if (
            not isinstance(verification, dict)
            or verification.get("fully_verified") is not True
            or verification.get("passed") is not True
        ):
            raise ProvenanceError("formula audit computation is not fully verified")
        formula_checks = verification.get("formula_quotes")
        input_checks = verification.get("input_quotes")
        if not isinstance(formula_checks, list) or not isinstance(input_checks, list):
            raise ProvenanceError("formula audit verification lists are malformed")
        normalized_formula_checks = []
        for check in formula_checks:
            if not isinstance(check, dict) or set(check) != {
                "identity",
                "provenance",
                "quote",
            }:
                raise ProvenanceError("formula audit formula quote has the wrong schema")
            reference, links = _formula_reference(cas, check["identity"], formula_bundles)
            if check["quote"].get("status") != "verified":
                raise ProvenanceError("formula audit formula quote is not verified")
            normalized_formula_checks.append(
                {
                    "identity": reference,
                    "provenance": check["provenance"],
                    "quote": check["quote"],
                }
            )
            sequence_links.update(links)
        normalized_input_checks = []
        for check in input_checks:
            if not isinstance(check, dict) or set(check) != {"identity", "quote"}:
                raise ProvenanceError("formula audit input quote has the wrong schema")
            reference, links = _input_reference(query_bundle, check["identity"])
            if check["quote"].get("status") != "verified":
                raise ProvenanceError("formula audit input quote is not verified")
            normalized_input_checks.append({"identity": reference, "quote": check["quote"]})
            input_links.update(links)
        if [check["identity"] for check in normalized_formula_checks] != sequence:
            raise ProvenanceError("formula audit formula statuses disagree with its sequence")
        if [check["identity"] for check in normalized_input_checks] != inputs:
            raise ProvenanceError("formula audit input statuses disagree with consumed inputs")
        derivation = {
            "contract": "adj-lang/formula_derivation/v1",
            "export": export,
            "formula_sequence": sequence,
            "imports": imports,
            "kind": "formula_derivation",
            "plan": item["plan"],
            "question": question,
            "schema_version": 1,
        }
        derivation_links = export_links | sequence_links | import_links | question_links
        witness = {
            "contract": "adj-lang/formula_execution/v1",
            "export": export,
            "formula_sequence": sequence,
            "inputs": inputs,
            "kind": "execution_witness",
            "question": question,
            "result": item["result"],
            "schema_version": 1,
            "tree": item["tree"],
            "verification": {
                **verification,
                "formula_quotes": normalized_formula_checks,
                "input_quotes": normalized_input_checks,
            },
        }
        witness_links = sequence_links | question_links | input_links
        normalized.append((derivation, derivation_links, witness, witness_links))
    direct_bundle_hash, inventory_hash, inventory = _direct_formula_inventory(cas, query_bundle)
    direct_refs = []
    for formula in inventory["formulas"]:
        identity = {
            "body": formula["body"],
            "declaration": formula["declaration"],
            "formulabook": formula["formulabook"],
            "name": formula["formula"],
            "parameters": formula["parameters"],
            "source_sha256": inventory["source_sha256"],
        }
        reference, _links = _formula_reference(cas, identity, formula_bundles)
        if reference["bundle_sha256"] != direct_bundle_hash or reference["inventory_sha256"] != inventory_hash:
            raise ProvenanceError("direct parser export resolves outside its formula bundle")
        direct_refs.append(canonical_json_bytes(reference))
    if sorted(direct_refs) != sorted(seen_exports):
        raise ProvenanceError("parsed exports and replayed derivations disagree")
    return normalized


def put_formula_execution_evidence(
    cas: Cas,
    query_bundle: dict[str, Any],
    audit_command: Sequence[str],
    *,
    label: str,
) -> tuple[list[str], list[str]]:
    audit = _materialize_formula_audit(cas, query_bundle, audit_command)
    derivation_hashes: list[str] = []
    witness_hashes: list[str] = []
    for derivation, derivation_links, witness, witness_links in _normalized_formula_evidence(
        cas, query_bundle, audit
    ):
        derivation_hash = cas.put_json(
            derivation,
            kind="formula_derivation",
            label=f"{label} {derivation['export']['formula']} derivation",
            links=derivation_links,
        )
        witness["formula_derivation_sha256"] = derivation_hash
        witness_hash = cas.put_json(
            witness,
            kind="execution_witness",
            label=f"{label} {derivation['export']['formula']} witness",
            links=witness_links | {derivation_hash},
        )
        derivation_hashes.append(derivation_hash)
        witness_hashes.append(witness_hash)
    return sorted(derivation_hashes), sorted(witness_hashes)


def _validate_formula_execution_evidence(
    cas: Cas,
    query_bundle: dict[str, Any],
    audit_command: Sequence[str] | None,
) -> set[str]:
    if audit_command is None:
        raise ProvenanceError("formula evidence replay requires --formula-audit-binary")
    stored_derivations = query_bundle.get("formula_derivation_sha256s")
    stored_witnesses = query_bundle.get("execution_witness_sha256s")
    if (
        not isinstance(stored_derivations, list)
        or stored_derivations != sorted(set(stored_derivations))
        or not stored_derivations
        or not isinstance(stored_witnesses, list)
        or stored_witnesses != sorted(set(stored_witnesses))
        or not stored_witnesses
    ):
        raise ProvenanceError("formula evidence hashes must be non-empty sorted unique arrays")
    audit = _materialize_formula_audit(cas, query_bundle, audit_command)
    expected = _normalized_formula_evidence(cas, query_bundle, audit)
    expected_derivations: list[str] = []
    expected_witnesses: list[str] = []
    expected_links: set[str] = set()
    for derivation, derivation_links, witness, witness_links in expected:
        derivation_hash = sha256_bytes(canonical_json_bytes(derivation))
        expected_derivations.append(derivation_hash)
        stored_derivation = _json_object(cas, derivation_hash, "formula_derivation")
        if stored_derivation != derivation or cas.index[derivation_hash]["links"] != sorted(derivation_links):
            raise ProvenanceError("stored formula derivation disagrees with replay")
        witness["formula_derivation_sha256"] = derivation_hash
        witness_hash = sha256_bytes(canonical_json_bytes(witness))
        expected_witnesses.append(witness_hash)
        stored_witness = _json_object(cas, witness_hash, "execution_witness")
        if stored_witness != witness or cas.index[witness_hash]["links"] != sorted(
            witness_links | {derivation_hash}
        ):
            raise ProvenanceError("stored execution witness disagrees with replay")
        expected_links.update((derivation_hash, witness_hash))
    if sorted(expected_derivations) != stored_derivations or sorted(expected_witnesses) != stored_witnesses:
        raise ProvenanceError("stored formula evidence set disagrees with replay")
    return expected_links


def _registered_manifest(
    cas: Cas,
    manifest_bytes: bytes,
    registrations: dict[str, str],
    expected_manifest_id: str,
    *,
    allow_replacements: bool = False,
    require_existing: bool = False,
    expected_current: dict[str, str] | None = None,
) -> tuple[dict[str, Any], list[str]]:
    if not isinstance(registrations, dict) or not registrations:
        raise ProvenanceError("bundle registrations must be a non-empty object")
    try:
        manifest = json.loads(manifest_bytes.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProvenanceError("provenance manifest must be UTF-8 JSON") from error
    if not isinstance(manifest, dict) or set(manifest) != {
        "algorithm",
        "bundle_hashes",
        "manifest_id",
        "schema_version",
    }:
        raise ProvenanceError("provenance manifest has unknown or missing fields")
    if manifest_bytes != canonical_json_bytes(manifest):
        raise ProvenanceError("provenance manifest is not canonical JSON")
    if (
        not _is_integer(manifest["schema_version"])
        or manifest["schema_version"] != 1
        or manifest["algorithm"] != "sha256"
    ):
        raise ProvenanceError("provenance manifest uses an unsupported contract")
    manifest_id = _require_nonempty(manifest["manifest_id"], "manifest_id")
    if manifest_id != _require_nonempty(expected_manifest_id, "expected_manifest_id"):
        raise ProvenanceError(
            f"provenance manifest_id {manifest_id} does not match {expected_manifest_id}"
        )
    roots = manifest["bundle_hashes"]
    if not isinstance(roots, list):
        raise ProvenanceError("bundle_hashes must be sorted and unique")
    normalized_roots = [
        _require_hash(value, f"bundle_hashes[{index}]")
        for index, value in enumerate(roots)
    ]
    if normalized_roots != sorted(set(normalized_roots)):
        raise ProvenanceError("bundle_hashes must be sorted and unique")

    by_id: dict[str, str] = {}
    for digest in normalized_roots:
        bundle = _json_object(cas, digest, "provenance_bundle")
        bundle_id = _require_nonempty(bundle.get("bundle_id"), "bundle.bundle_id")
        previous = by_id.get(bundle_id)
        if previous is not None and previous != digest:
            raise ProvenanceError(
                f"manifest registers bundle_id {bundle_id} more than once"
            )
        by_id[bundle_id] = digest

    for bundle_id, expected_digest in (expected_current or {}).items():
        normalized_id = _require_nonempty(bundle_id, "expected bundle_id")
        normalized_digest = _require_hash(
            expected_digest, f"expected current root {normalized_id}"
        )
        actual_digest = by_id.get(normalized_id)
        if actual_digest != normalized_digest:
            raise ProvenanceError(
                f"stale root replacement for {normalized_id}: expected "
                f"{normalized_digest}, found {actual_digest or 'unregistered'}"
            )

    for expected_id, digest_value in registrations.items():
        bundle_id = _require_nonempty(expected_id, "registration bundle_id")
        digest = _require_hash(digest_value, f"registration {bundle_id}")
        bundle = _json_object(cas, digest, "provenance_bundle")
        actual_id = _require_nonempty(bundle.get("bundle_id"), "bundle.bundle_id")
        if actual_id != bundle_id:
            raise ProvenanceError(
                f"registration {bundle_id} points to bundle_id {actual_id}"
            )
        previous = by_id.get(bundle_id)
        if require_existing and previous is None:
            raise ProvenanceError(f"cannot replace unregistered bundle_id {bundle_id}")
        if previous is not None and previous != digest and not allow_replacements:
            raise ProvenanceError(
                f"refusing to replace registered bundle_id {bundle_id}; "
                "use an explicit root-replacement migration"
            )
        by_id[bundle_id] = digest

    registered = sorted(by_id.values())
    manifest["bundle_hashes"] = registered
    return manifest, registered


class CasRootLock:
    """Cross-platform OS lock released automatically when the process exits."""

    def __init__(self, cas_root: Path, *, blocking: bool = True) -> None:
        self.path = cas_root.resolve() / "lock"
        self.blocking = blocking
        self.descriptor: int | None = None

    def __enter__(self):
        _ensure_real_directory(self.path.parent)
        _reject_link_components(self.path, allow_missing_leaf=True)
        flags = os.O_CREAT | os.O_RDWR | getattr(os, "O_BINARY", 0)
        descriptor = os.open(self.path, flags, 0o600)
        try:
            if os.fstat(descriptor).st_size == 0:
                if os.write(descriptor, b"\0") != 1:
                    raise ProvenanceError("CAS lock initialization was incomplete")
                os.fsync(descriptor)
            os.lseek(descriptor, 0, os.SEEK_SET)
        except Exception:
            os.close(descriptor)
            raise
        try:
            if os.name == "nt":
                import msvcrt

                mode = msvcrt.LK_LOCK if self.blocking else msvcrt.LK_NBLCK
                msvcrt.locking(descriptor, mode, 1)
            else:
                import fcntl

                mode = fcntl.LOCK_EX
                if not self.blocking:
                    mode |= fcntl.LOCK_NB
                fcntl.flock(descriptor, mode)
        except (BlockingIOError, OSError) as error:
            os.close(descriptor)
            raise ProvenanceError(
                f"another provenance operation holds the CAS lock for {self.path}"
            ) from error
        self.descriptor = descriptor
        return self

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> bool:
        descriptor = self.descriptor
        self.descriptor = None
        if descriptor is None:
            return False
        try:
            os.lseek(descriptor, 0, os.SEEK_SET)
            if os.name == "nt":
                import msvcrt

                msvcrt.locking(descriptor, msvcrt.LK_UNLCK, 1)
            else:
                import fcntl

                fcntl.flock(descriptor, fcntl.LOCK_UN)
        finally:
            os.close(descriptor)
        return False


class CasMutationTransaction:
    """Serialize one CAS mutation and remove newly materialized bytes on failure."""

    def __init__(self, cas_root: Path, *, blocking: bool = True) -> None:
        self.cas = Cas(cas_root)
        self._lock = CasRootLock(cas_root, blocking=blocking)
        self._baseline_index: bytes | None = None
        self._baseline_objects: set[Path] = set()
        self._entered = False
        self._committed = False

    def __enter__(self):
        self._lock.__enter__()
        try:
            self.cas.load()
            if self.cas.index_path.exists():
                _validate_index(self.cas)
                self._baseline_index = _read_regular_file(self.cas.index_path)
            elif self.cas.index:
                raise ProvenanceError("CAS index loaded without an index file")
            self._baseline_objects = {
                path.resolve() for path in self.cas.objects.rglob("*") if path.is_file()
            }
            self._entered = True
            return self
        except Exception:
            self._lock.__exit__(None, None, None)
            raise

    def commit(self) -> None:
        if not self._entered or self._committed:
            raise ProvenanceError("CAS mutation transaction is not open")
        self.cas.write_index()
        self._committed = True

    def _rollback(self) -> None:
        if not self._entered:
            return
        if self._baseline_index is None:
            if self.cas.index_path.exists():
                _reject_link_components(self.cas.index_path)
                self.cas.index_path.unlink()
        else:
            _write_atomic(self.cas.index_path, self._baseline_index)
        if self.cas.objects.exists():
            for path in sorted(self.cas.objects.rglob("*"), reverse=True):
                if not path.is_file() or path.resolve() in self._baseline_objects:
                    continue
                data = _read_regular_file(path)
                relative = path.relative_to(self.cas.objects)
                if len(relative.parts) != 2:
                    raise ProvenanceError(
                        f"refusing rollback of non-canonical CAS path {path}"
                    )
                digest = relative.parts[0] + relative.parts[1]
                if sha256_bytes(data) != digest:
                    raise ProvenanceError(
                        f"refusing rollback of mismatched CAS object {path}"
                    )
                path.unlink()
                try:
                    path.parent.rmdir()
                except OSError:
                    pass
        self.cas.load()

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> bool:
        try:
            if exc_type is not None or not self._committed:
                self._rollback()
            if exc_type is None and not self._committed:
                raise ProvenanceError("CAS mutation transaction exited without commit")
        finally:
            self._lock.__exit__(exc_type, exc, traceback)
            self._entered = False
        return False


class BundleRegistrationTransaction(CasMutationTransaction):
    """Validate and publish a generator's CAS index and manifest root update."""

    def __init__(
        self,
        cas_root: Path,
        manifest_path: Path,
        *,
        expected_manifest_id: str,
        schema_path: Path | None = None,
        workspace_root: Path | None = None,
        formula_inventory_command: Sequence[str] | None = None,
        formula_audit_command: Sequence[str] | None = None,
        allow_unwitnessed_baseline: bool = False,
    ) -> None:
        super().__init__(cas_root, blocking=False)
        self.manifest_path = manifest_path
        self.expected_manifest_id = expected_manifest_id
        self.schema_path = schema_path
        self.workspace_root = workspace_root
        self.formula_inventory_command = formula_inventory_command
        self.formula_audit_command = formula_audit_command
        self.allow_unwitnessed_baseline = allow_unwitnessed_baseline
        self._baseline_manifest = b""

    def __enter__(self):
        try:
            super().__enter__()
            _validate_repository_unlocked(
                self.cas.root,
                self.manifest_path,
                self.schema_path,
                workspace_root=self.workspace_root,
                formula_inventory_command=self.formula_inventory_command,
                formula_audit_command=self.formula_audit_command,
                _allow_unwitnessed=self.allow_unwitnessed_baseline,
            )
            self._baseline_manifest = _read_regular_file(self.manifest_path)
            return self
        except Exception:
            super().__exit__(Exception, None, None)
            raise

    def commit(self, registrations: dict[str, str]) -> list[str]:
        if not self._entered or self._committed:
            raise ProvenanceError("registration transaction is not open")
        manifest, registered = _registered_manifest(
            self.cas,
            self._baseline_manifest,
            registrations,
            self.expected_manifest_id,
        )
        try:
            self.cas.write_index()
            _write_atomic(self.manifest_path, canonical_json_bytes(manifest))
            _validate_repository_unlocked(
                self.cas.root,
                self.manifest_path,
                self.schema_path,
                workspace_root=self.workspace_root,
                formula_inventory_command=self.formula_inventory_command,
                formula_audit_command=self.formula_audit_command,
            )
        except Exception:
            self._rollback()
            raise
        self._committed = True
        return registered

    def _rollback(self) -> None:
        if not self._entered:
            return
        if self._baseline_manifest:
            _write_atomic(self.manifest_path, self._baseline_manifest)
        super()._rollback()


class BundleRootReplacementTransaction(BundleRegistrationTransaction):
    """Explicitly replace owned roots and prune only newly unreachable objects."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self._baseline_digests: set[str] = set()
        self._pruned_digests: list[str] = []
        self._prune_backup: tempfile.TemporaryDirectory[str] | None = None

    def __enter__(self):
        super().__enter__()
        self._baseline_digests = set(self.cas.index)
        return self

    def _stage_prune(self, digests: set[str]) -> None:
        backup = tempfile.TemporaryDirectory(prefix="adj-provenance-prune-")
        backup_root = Path(backup.name)
        try:
            for digest in sorted(digests):
                data = _read_regular_file(self.cas.object_path(digest))
                if sha256_bytes(data) != digest:
                    raise ProvenanceError(
                        f"refusing to prune mismatched CAS object {digest}"
                    )
                _write_exclusive(backup_root / digest, data)
            for digest in sorted(digests):
                path = self.cas.object_path(digest)
                _reject_link_components(path)
                self._pruned_digests.append(digest)
                path.unlink()
                try:
                    path.parent.rmdir()
                except OSError:
                    pass
        except Exception:
            self._prune_backup = backup
            self._restore_pruned()
            raise
        self._prune_backup = backup

    def _restore_pruned(self) -> None:
        if self._prune_backup is None:
            return
        backup_root = Path(self._prune_backup.name)
        for digest in self._pruned_digests:
            destination = self.cas.object_path(digest)
            if destination.exists():
                if sha256_bytes(_read_regular_file(destination)) != digest:
                    raise ProvenanceError(
                        f"cannot restore over mismatched CAS object {digest}"
                    )
                continue
            _write_exclusive(destination, _read_regular_file(backup_root / digest))
        self._pruned_digests.clear()

    def commit(self, registrations: dict[str, str]) -> list[str]:
        del registrations
        raise ProvenanceError(
            "root replacement transactions require replace_roots with expected hashes"
        )

    def replace_roots(self, replacements: dict[str, dict[str, str]]) -> dict[str, Any]:
        if not self._entered or self._committed:
            raise ProvenanceError("root replacement transaction is not open")
        if not isinstance(replacements, dict) or not replacements:
            raise ProvenanceError("root replacements must be a non-empty object")
        expected_current: dict[str, str] = {}
        new_roots: dict[str, str] = {}
        for bundle_id, replacement in replacements.items():
            normalized_id = _require_nonempty(bundle_id, "replacement bundle_id")
            if not isinstance(replacement, dict) or set(replacement) != {
                "expected_old_sha256",
                "new_sha256",
            }:
                raise ProvenanceError(
                    f"replacement {normalized_id} must name expected_old_sha256 "
                    "and new_sha256"
                )
            expected_current[normalized_id] = _require_hash(
                replacement["expected_old_sha256"],
                f"replacement {normalized_id}.expected_old_sha256",
            )
            new_roots[normalized_id] = _require_hash(
                replacement["new_sha256"],
                f"replacement {normalized_id}.new_sha256",
            )
        manifest, registered = _registered_manifest(
            self.cas,
            self._baseline_manifest,
            new_roots,
            self.expected_manifest_id,
            allow_replacements=True,
            require_existing=True,
            expected_current=expected_current,
        )
        try:
            self.cas.write_index()
            with tempfile.TemporaryDirectory(
                prefix="adj-provenance-candidate-"
            ) as candidate_directory:
                candidate_manifest = Path(candidate_directory) / "manifest.json"
                _write_exclusive(candidate_manifest, canonical_json_bytes(manifest))
                _validate_repository_unlocked(
                    self.cas.root,
                    candidate_manifest,
                    self.schema_path,
                    workspace_root=self.workspace_root or self.manifest_path.parent,
                    formula_inventory_command=self.formula_inventory_command,
                    formula_audit_command=self.formula_audit_command,
                    _allow_unreferenced=True,
                )
            reachable = _reachable(self.cas, registered)
            unreachable = set(self.cas.index) - reachable
            staged_strays = unreachable - self._baseline_digests
            if staged_strays:
                raise ProvenanceError(
                    "root replacement staged unreferenced new objects: "
                    + ", ".join(sorted(staged_strays))
                )
            self._stage_prune(unreachable)
            self.cas.index = {
                digest: record
                for digest, record in self.cas.index.items()
                if digest in reachable
            }
            self.cas.write_index()
            _write_atomic(self.manifest_path, canonical_json_bytes(manifest))
            _validate_repository_unlocked(
                self.cas.root,
                self.manifest_path,
                self.schema_path,
                workspace_root=self.workspace_root,
                formula_inventory_command=self.formula_inventory_command,
                formula_audit_command=self.formula_audit_command,
            )
        except Exception:
            self._rollback()
            raise
        self._committed = True
        pruned = sorted(unreachable)
        if self._prune_backup is not None:
            try:
                self._prune_backup.cleanup()
            except OSError:
                pass
            self._prune_backup = None
        return {"bundle_hashes": registered, "pruned_sha256s": pruned}

    def _rollback(self) -> None:
        self._restore_pruned()
        super()._rollback()
        if self._prune_backup is not None:
            try:
                self._prune_backup.cleanup()
            except OSError:
                pass
            self._prune_backup = None


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
        "execution_witness",
        "fetch_receipt",
        "formula_derivation",
        "formula_parser_inventory",
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


def _transform_operations_for_claim(
    operations: list[dict[str, Any]], claim_id: str, text_claim: dict[str, Any]
) -> list[dict[str, Any]]:
    return [
        operation
        for operation in operations
        if (operation["operation"] == "discard" and operation["claim_id"] == claim_id)
        or (
            operation["operation"] != "discard"
            and operation["result_end"] > text_claim["start"]
            and operation["result_start"] < text_claim["end"]
        )
    ]


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
        discard_claims = {
            operation["claim_id"]
            for operation in transform["operations"]
            if operation["operation"] == "discard"
        }
        unknown_discard_claims = sorted(
            discard_claims - (set(text_claims) & set(raw_claims))
        )
        if unknown_discard_claims:
            raise ProvenanceError(
                f"{item_prefix} transform discards name unknown claims: "
                f"{', '.join(unknown_discard_claims)}"
            )
        for claim_id, text_claim in text_claims.items():
            raw_claim = raw_claims[claim_id]
            if discard_claims:
                claim_operations = _transform_operations_for_claim(
                    transform["operations"], claim_id, text_claim
                )
                if not claim_operations or (
                    raw_claim["start"] != claim_operations[0]["source_start"]
                    or raw_claim["end"] != claim_operations[-1]["source_end"]
                ):
                    raise ProvenanceError(
                        f"{item_prefix} explicit transform partition does not "
                        f"account for every raw claim byte in {claim_id}"
                    )
            claim_cursor = text_claim["start"]
            for operation in transform["operations"]:
                if operation["operation"] == "discard":
                    if operation["claim_id"] != claim_id:
                        continue
                    if (
                        operation["source_start"] < raw_claim["start"]
                        or operation["source_end"] > raw_claim["end"]
                    ):
                        raise ProvenanceError(
                            f"{item_prefix} transform discard for claim {claim_id} "
                            "escapes its corresponding raw claim bytes"
                        )
                    continue
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
    bundle_ids: dict[str, str],
    bundle_inputs: dict[str, str],
    formula_inventory_command: Sequence[str] | None,
    formula_audit_command: Sequence[str] | None,
) -> None:
    if digest in validated:
        return
    if digest in visiting:
        raise ProvenanceError(f"provenance bundle dependency cycle at {digest}")
    visiting.add(digest)
    bundle = _json_object(cas, digest, "provenance_bundle")
    required_fields = {
        "bundle_id",
        "clauses",
        "dependencies",
        "input",
        "kind",
        "library",
        "sources",
    }
    optional_fields = {
        "execution_witness_sha256s",
        "formula_derivation_sha256s",
        "formula_inventory_sha256",
    }
    if not required_fields <= set(bundle) or not set(bundle) <= (
        required_fields | optional_fields
    ):
        raise ProvenanceError(f"bundle {digest} has unknown or missing fields")
    if bundle["kind"] != "provenance_bundle":
        raise ProvenanceError(f"bundle {digest} payload has the wrong kind")
    bundle_id = _require_nonempty(bundle["bundle_id"], "bundle.bundle_id")
    prior_digest = bundle_ids.get(bundle_id)
    if prior_digest is not None and prior_digest != digest:
        raise ProvenanceError(
            f"bundle_id {bundle_id} resolves to both {prior_digest} and {digest}"
        )
    bundle_ids[bundle_id] = digest
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
            bundle_ids=bundle_ids,
            bundle_inputs=bundle_inputs,
            formula_inventory_command=formula_inventory_command,
            formula_audit_command=formula_audit_command,
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
    if "formula_inventory_sha256" in bundle:
        inventory_hash = _require_hash(
            bundle["formula_inventory_sha256"], "bundle.formula_inventory_sha256"
        )
        if "formula_parser_inventory" not in cas.index.get(inventory_hash, {}).get(
            "kinds", []
        ):
            raise ProvenanceError(
                f"bundle {digest} formula inventory is missing or has the wrong kind"
            )
        _validate_formula_inventory(
            cas,
            inventory_hash,
            input_raw_hash,
            claims[input_ir_hash],
            formula_inventory_command,
        )
        expected_links.add(inventory_hash)
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
    evidence_fields = {
        "execution_witness_sha256s",
        "formula_derivation_sha256s",
    }
    if evidence_fields & set(bundle):
        if not evidence_fields <= set(bundle):
            raise ProvenanceError(
                f"bundle {digest} must bind derivations and witnesses together"
            )
        expected_links.update(
            _validate_formula_execution_evidence(
                cas, bundle, formula_audit_command
            )
        )
    if cas.index[digest]["links"] != sorted(expected_links):
        raise ProvenanceError(
            f"bundle {digest} CAS links disagree with its payload graph"
        )
    visiting.remove(digest)
    bundle_claims[digest] = seen_clause_ids
    validated.add(digest)


def _validate_repository_unlocked(
    cas_root: Path,
    manifest_path: Path,
    schema_path: Path | None = None,
    workspace_root: Path | None = None,
    formula_inventory_command: Sequence[str] | None = None,
    formula_audit_command: Sequence[str] | None = None,
    _allow_unreferenced: bool = False,
    _allow_unwitnessed: bool = False,
) -> dict[str, Any]:
    cas = Cas(cas_root)
    cas.load()
    _validate_index(cas)
    manifest_bytes = _read_regular_file(manifest_path)
    manifest = json.loads(manifest_bytes.decode("utf-8"))
    if (
        not isinstance(manifest, dict)
        or not _is_integer(manifest.get("schema_version"))
        or manifest.get("schema_version") != 1
    ):
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
    if not isinstance(bundles, list):
        raise ProvenanceError("bundle_hashes must be sorted and unique")
    normalized_bundles = [
        _require_hash(value, f"bundle_hashes[{index}]")
        for index, value in enumerate(bundles)
    ]
    if normalized_bundles != sorted(set(normalized_bundles)):
        raise ProvenanceError("bundle_hashes must be sorted and unique")
    snapshots: set[str] = set()
    validated: set[str] = set()
    bundle_claims: dict[str, set[str]] = {}
    bundle_ids: dict[str, str] = {}
    bundle_inputs: dict[str, str] = {}
    for bundle in normalized_bundles:
        _validate_bundle(
            cas,
            _require_hash(bundle, "bundle hash"),
            visiting=set(),
            validated=validated,
            snapshots=snapshots,
            bundle_claims=bundle_claims,
            bundle_ids=bundle_ids,
            bundle_inputs=bundle_inputs,
            formula_inventory_command=formula_inventory_command,
            formula_audit_command=formula_audit_command,
        )
    if not _allow_unwitnessed:
        inventoried = {
            digest
            for digest in validated
            if "formula_inventory_sha256"
            in _json_object(cas, digest, "provenance_bundle")
        }
        witnessed = {
            _json_object(cas, digest, "provenance_bundle")["dependencies"][0]
            for digest in validated
            if "execution_witness_sha256s"
            in _json_object(cas, digest, "provenance_bundle")
        }
        if inventoried != witnessed:
            missing = sorted(inventoried - witnessed)
            extra = sorted(witnessed - inventoried)
            raise ProvenanceError(
                "formula execution witness coverage disagrees; missing="
                + ",".join(missing)
                + " extra="
                + ",".join(extra)
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
    reachable = _reachable(cas, normalized_bundles)
    unreferenced = sorted(set(cas.index) - reachable)
    if unreferenced and not _allow_unreferenced:
        raise ProvenanceError(f"unreferenced CAS objects: {', '.join(unreferenced)}")
    return {
        "bundles": len(validated),
        "objects": len(cas.index),
        "snapshot_hashes": sorted(snapshots),
        "snapshots": len(snapshots),
        "valid": True,
    }


def validate_repository(
    cas_root: Path,
    manifest_path: Path,
    schema_path: Path | None = None,
    workspace_root: Path | None = None,
    formula_inventory_command: Sequence[str] | None = None,
    formula_audit_command: Sequence[str] | None = None,
) -> dict[str, Any]:
    with CasRootLock(cas_root):
        return _validate_repository_unlocked(
            cas_root,
            manifest_path,
            schema_path,
            workspace_root=workspace_root,
            formula_inventory_command=formula_inventory_command,
            formula_audit_command=formula_audit_command,
        )


def project_snapshots(
    cas_root: Path,
    manifest_path: Path,
    output: Path,
    schema_path: Path | None = None,
    workspace_root: Path | None = None,
    formula_inventory_command: Sequence[str] | None = None,
    formula_audit_command: Sequence[str] | None = None,
) -> dict[str, Any]:
    with CasRootLock(cas_root):
        result = _validate_repository_unlocked(
            cas_root,
            manifest_path,
            schema_path,
            workspace_root=workspace_root,
            formula_inventory_command=formula_inventory_command,
            formula_audit_command=formula_audit_command,
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
    links.update(bundle.get("execution_witness_sha256s", []))
    links.update(bundle.get("formula_derivation_sha256s", []))
    if bundle.get("formula_inventory_sha256") is not None:
        links.add(bundle["formula_inventory_sha256"])
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
    verify_parser.add_argument("--formula-inventory-binary", type=Path)
    verify_parser.add_argument("--formula-audit-binary", type=Path)

    project_parser = subparsers.add_parser("project")
    project_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    project_parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    project_parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    project_parser.add_argument("--output", type=Path, required=True)
    project_parser.add_argument("--formula-inventory-binary", type=Path)
    project_parser.add_argument("--formula-audit-binary", type=Path)

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

    formula_parser = subparsers.add_parser("put-formula-inventory")
    formula_parser.add_argument("--cas", type=Path, default=DEFAULT_ROOT)
    formula_parser.add_argument("--source", required=True)
    formula_parser.add_argument("--formula-inventory-binary", type=Path, required=True)
    formula_parser.add_argument("--label", required=True)

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
                formula_inventory_command=(
                    [os.fspath(_resolve(repo_root, args.formula_inventory_binary))]
                    if args.formula_inventory_binary is not None
                    else None
                ),
                formula_audit_command=(
                    [os.fspath(_resolve(repo_root, args.formula_audit_binary))]
                    if args.formula_audit_binary is not None
                    else None
                ),
            )
        elif args.command == "project":
            result = project_snapshots(
                _resolve(repo_root, args.cas),
                _resolve(repo_root, args.manifest),
                _resolve(repo_root, args.output),
                _resolve(repo_root, args.schema),
                workspace_root=repo_root,
                formula_inventory_command=(
                    [os.fspath(_resolve(repo_root, args.formula_inventory_binary))]
                    if args.formula_inventory_binary is not None
                    else None
                ),
                formula_audit_command=(
                    [os.fspath(_resolve(repo_root, args.formula_audit_binary))]
                    if args.formula_audit_binary is not None
                    else None
                ),
            )
        elif args.command == "capture":
            with CasMutationTransaction(_resolve(repo_root, args.cas)) as transaction:
                cas = transaction.cas
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
                transaction.commit()
            result = {"raw_source_sha256": raw_hash, "receipt_sha256": receipt_hash}
        elif args.command == "capture-input":
            with CasMutationTransaction(_resolve(repo_root, args.cas)) as transaction:
                cas = transaction.cas
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
                transaction.commit()
            result = {"raw_source_sha256": raw_hash, "receipt_sha256": receipt_hash}
        elif args.command == "put-rendered":
            with CasMutationTransaction(_resolve(repo_root, args.cas)) as transaction:
                cas = transaction.cas
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
                transaction.commit()
            result = {"rendered_text_sha256": rendered_hash}
        elif args.command == "put-ir":
            with CasMutationTransaction(_resolve(repo_root, args.cas)) as transaction:
                cas = transaction.cas
                source_hash = _require_hash(args.source, "source hash")
                if not {
                    "raw_source",
                    "rendered_text",
                }.intersection(cas.index.get(source_hash, {}).get("kinds", [])):
                    raise ProvenanceError("source hash is not source bytes in the CAS")
                segments = json.loads(
                    _read_regular_file(_resolve(repo_root, args.segments)).decode(
                        "utf-8"
                    )
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
                transaction.commit()
            result = {"source_ir_sha256": ir_hash}
        elif args.command == "put-formula-inventory":
            with CasMutationTransaction(_resolve(repo_root, args.cas)) as transaction:
                cas = transaction.cas
                inventory_hash = put_formula_parser_inventory(
                    cas,
                    args.source,
                    [os.fspath(_resolve(repo_root, args.formula_inventory_binary))],
                    label=args.label,
                )
                transaction.commit()
            result = {"formula_inventory_sha256": inventory_hash}
        elif args.command == "put-transform":
            with CasMutationTransaction(_resolve(repo_root, args.cas)) as transaction:
                cas = transaction.cas
                source_hash = _require_hash(args.source, "source hash")
                result_hash = _require_hash(args.result, "result hash")
                if "raw_source" not in cas.index.get(source_hash, {}).get("kinds", []):
                    raise ProvenanceError("transform source is not raw_source bytes")
                if "rendered_text" not in cas.index.get(result_hash, {}).get(
                    "kinds", []
                ):
                    raise ProvenanceError("transform result is not rendered_text bytes")
                operations = json.loads(
                    _read_regular_file(_resolve(repo_root, args.operations)).decode(
                        "utf-8"
                    )
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
                transaction.commit()
            result = {"transform_sha256": transform_hash}
        else:
            with CasMutationTransaction(_resolve(repo_root, args.cas)) as transaction:
                cas = transaction.cas
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
                transaction.commit()
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
