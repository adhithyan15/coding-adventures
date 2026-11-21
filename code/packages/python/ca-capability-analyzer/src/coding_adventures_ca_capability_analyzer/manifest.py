"""Manifest loader and validator for required_capabilities.json (VER08/Spec 13).

Pure Python with zero external dependencies.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from coding_adventures_ca_capability_analyzer.types import (
    CAP_DYNAMIC_EVAL,
    CAP_ENV_READ,
    CAP_ENV_WRITE,
    CAP_FFI_UNSAFE,
    CAP_FS_READ,
    CAP_FS_WRITE,
    CAP_NET_CLIENT,
    CAP_NET_SERVER,
    CAP_PROCESS_EXEC,
    CAP_RAND_READ,
    CAP_REGEX_RE,
    CAP_THREAD_SPAWN,
    CAP_TIME_READ,
    CAP_TIME_SLEEP,
    Manifest,
)

MAX_MANIFEST_SIZE_BYTES = 1024 * 1024
MAX_MANIFEST_ITEMS = 4096
MAX_CAP_TOKEN_LEN = 128

_STRING_SYNONYMS: dict[str, str] = {
    "time:clock": CAP_TIME_READ,
    "random:entropy": CAP_RAND_READ,
    "process:spawn": CAP_PROCESS_EXEC,
    "net:socket": CAP_NET_CLIENT,
}


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """JSON object_pairs_hook that rejects duplicate keys (CWE-20)."""
    out: dict[str, Any] = {}
    for key, value in pairs:
        if key in out:
            raise ValueError(f"Duplicate JSON key in manifest: {key!r}")
        out[key] = value
    return out


def normalize_capability(raw: Any) -> str | None:
    """Normalize raw capability or schema dict into canonical capability."""
    if isinstance(raw, str):
        if len(raw) > MAX_CAP_TOKEN_LEN:
            raise ValueError(
                f"Capability string exceeds max length ({MAX_CAP_TOKEN_LEN})"
            )
        cleaned = raw.strip()
        if cleaned.endswith(":*"):
            cleaned = cleaned[:-2]
        # Normalize category synonyms
        if cleaned.startswith("proc:"):
            cleaned = "process:" + cleaned[5:]
        return _STRING_SYNONYMS.get(cleaned, cleaned)

    if isinstance(raw, dict):
        cat = raw.get("category", "")
        action = raw.get("action", "")
        if not isinstance(cat, str) or not isinstance(action, str):
            raise ValueError("Capability category and action must be strings")
        if len(cat) > MAX_CAP_TOKEN_LEN or len(action) > MAX_CAP_TOKEN_LEN:
            raise ValueError(
                f"Capability category/action exceeds max length ({MAX_CAP_TOKEN_LEN})"
            )

        if cat == "fs":
            if action in ("read", "list"):
                return CAP_FS_READ
            if action in ("write", "create", "delete"):
                return CAP_FS_WRITE
        elif cat == "net":
            if action in ("connect", "dns"):
                return CAP_NET_CLIENT
            if action == "listen":
                return CAP_NET_SERVER
        elif cat in ("proc", "process"):
            if action in ("exec", "fork", "signal"):
                return CAP_PROCESS_EXEC
        elif cat == "ffi":
            if action in ("call", "load", "unsafe"):
                return CAP_FFI_UNSAFE
        elif cat == "env":
            if action == "read":
                return CAP_ENV_READ
            if action == "write":
                return CAP_ENV_WRITE
        elif cat == "time":
            if action in ("read", "clock"):
                return CAP_TIME_READ
            if action == "sleep":
                return CAP_TIME_SLEEP
        elif cat in ("rand", "random") and action in ("read", "entropy"):
            return CAP_RAND_READ
        elif cat == "thread" and action in ("spawn", "message"):
            return CAP_THREAD_SPAWN
        elif cat == "dynamic" and action == "eval":
            return CAP_DYNAMIC_EVAL
        elif cat == "regex" and action == "re":
            return CAP_REGEX_RE

        # Fallback to cat:action
        return f"{cat}:{action}"

    return None


def load_manifest(
    manifest_path: Path | str, expected_root: Path | None = None
) -> Manifest:
    """Load and parse required_capabilities.json with fail-closed semantics."""
    p = Path(manifest_path)
    if p.is_symlink():
        raise ValueError(f"Manifest cannot be a symlink: {manifest_path}")
    if not p.is_file():
        # Absence means pure computation (default deny)
        return Manifest(
            package="",
            version=1,
            capabilities=frozenset(),
            justification="Pure computation baseline (default deny)",
            is_explicit=False,
        )

    resolved = p.resolve(strict=True)
    if expected_root is not None and not resolved.is_relative_to(
        expected_root.resolve(strict=True)
    ):
        msg = f"Manifest path '{manifest_path}' escapes expected root '{expected_root}'"
        raise ValueError(msg)

    size = resolved.stat().st_size
    if size > MAX_MANIFEST_SIZE_BYTES:
        raise ValueError(
            f"Manifest size ({size} bytes) exceeds limit ({MAX_MANIFEST_SIZE_BYTES})"
        )

    with resolved.open("r", encoding="utf-8") as f:
        data = json.load(f, object_pairs_hook=_reject_duplicate_keys)

    if not isinstance(data, dict):
        raise ValueError("Manifest root must be a JSON object")

    raw_version = data.get("version", 1)
    if isinstance(raw_version, bool) or not isinstance(raw_version, int):
        raise ValueError("Manifest 'version' must be an integer")
    if raw_version < 1:
        raise ValueError(f"Manifest 'version' must be >= 1, got {raw_version}")
    version = raw_version

    raw_caps = data.get("capabilities", [])
    if not isinstance(raw_caps, list):
        raise ValueError("Manifest 'capabilities' must be a list")
    if len(raw_caps) > MAX_MANIFEST_ITEMS:
        raise ValueError(
            f"Manifest 'capabilities' exceeds max items ({MAX_MANIFEST_ITEMS})"
        )

    norm_caps: set[str] = set()
    for item in raw_caps:
        norm = normalize_capability(item)
        if norm:
            norm_caps.add(norm)

    return Manifest(
        package=str(data.get("package", "")),
        version=version,
        capabilities=frozenset(norm_caps),
        justification=str(data.get("justification", "")),
        is_explicit=True,
    )
