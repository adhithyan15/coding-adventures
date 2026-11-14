"""Bounded, strict JSON reading and canonical writing.

The corpus is *untrusted input* to the runner (VER04i section 6.2): a stale or
hand-edited corpus must fail closed, not be interpreted charitably.  So:

* files are size-capped **before** they are parsed;
* only regular files are opened (no symlink to somewhere surprising);
* duplicate object keys are an error (``json.loads`` would silently keep the
  last one, which is how two readers end up disagreeing about a document);
* ``NaN``/``Infinity`` are rejected (they are not JSON);
* output is canonical: sorted keys, two-space indent, LF, trailing newline,
  so a regenerated file is byte-identical to the committed one.
"""

from __future__ import annotations

import hashlib
import json
import stat
from pathlib import Path

from .errors import CorpusError, need

MAX_CORPUS_BYTES = 32 * 1024 * 1024
MAX_TEXT_BYTES = 8 * 1024 * 1024


def read_bytes(path: Path, limit: int) -> bytes:
    """Read a regular file of at most ``limit`` bytes, or raise."""
    try:
        info = path.lstat()
        need(stat.S_ISREG(info.st_mode), "io", f"{path.name} is not a regular file")
        need(info.st_size <= limit, "input-limit", f"{path.name} exceeds {limit} bytes")
        with path.open("rb") as handle:
            data = handle.read(limit + 1)
    except OSError as error:
        raise CorpusError("io", f"cannot read {path.name}") from error
    need(len(data) <= limit, "input-limit", f"{path.name} exceeds {limit} bytes")
    return data


def sha256_hex(data: bytes) -> str:
    """Lowercase hex SHA-256 of ``data``."""
    return hashlib.sha256(data).hexdigest()


def _reject_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        need(key not in result, "malformed-json", "duplicate object key")
        result[key] = value
    return result


def _reject_constant(_name: str) -> object:
    raise CorpusError("malformed-json", "non-finite number")


def parse_json(data: bytes) -> object:
    """Parse UTF-8 JSON strictly (no duplicate keys, no NaN)."""
    try:
        text = data.decode("utf-8", errors="strict")
        return json.loads(
            text,
            object_pairs_hook=_reject_duplicates,
            parse_constant=_reject_constant,
        )
    except CorpusError:
        raise
    except (UnicodeDecodeError, ValueError, RecursionError) as error:
        raise CorpusError("malformed-json", "invalid or truncated JSON") from error


def render_json(document: object) -> str:
    """Canonical rendering: sorted keys, 2-space indent, LF, final newline."""
    return json.dumps(document, indent=2, sort_keys=True) + "\n"
