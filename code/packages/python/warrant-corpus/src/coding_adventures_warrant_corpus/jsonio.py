"""Bounded, strict JSON reading and canonical writing.

The corpus is *untrusted input* to the runner (VER04i section 6.2): a stale or
hand-edited corpus must fail closed, not be interpreted charitably.  So:

* files are size-capped **before** they are parsed;
* only regular files are opened (no symlink to somewhere surprising);
* duplicate object keys are an error (``json.loads`` would silently keep the
  last one, which is how two readers end up disagreeing about a document);
* ``NaN``/``Infinity`` are rejected (they are not JSON);
* a number must be an **integer** in the safe range ``+-(2**53 - 1)``: no
  fraction, no exponent, no ``-0``.  Python's ints are exact, but the corpus is
  shared with runtimes whose numbers are doubles, so every lane accepts the
  same set (VER04i 9.1); ``U64`` values travel as tagged decimal *text*;
* strings are well-formed Unicode: an unpaired surrogate escape (``"\\ud800"``)
  is an error.  ``json.loads`` happily builds a lone-surrogate ``str``, which
  cannot be encoded as UTF-8 and means different things to different readers;
* at most ``MAX_JSON_DEPTH`` containers nest (the 257th is refused);
* output is canonical: sorted keys, two-space indent, LF, trailing newline,
  so a regenerated file is byte-identical to the committed one.
"""

from __future__ import annotations

import hashlib
import json
import re
import stat
from pathlib import Path

from .errors import CorpusError, need

MAX_CORPUS_BYTES = 32 * 1024 * 1024
MAX_TEXT_BYTES = 8 * 1024 * 1024
MAX_JSON_DEPTH = 256  # nested containers; scalars do not count
SAFE_INTEGER_MAX = 2**53 - 1
_SURROGATE = re.compile("[\ud800-\udfff]")


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


def _reject_float(_text: str) -> object:
    raise CorpusError("malformed-json", "only integer numbers are allowed")


def _safe_integer(text: str) -> int:
    need(text != "-0", "malformed-json", "negative zero is not an integer")
    number = int(text)
    need(
        abs(number) <= SAFE_INTEGER_MAX,
        "malformed-json",
        "number is not a safe integer",
    )
    return number


def _check_shape(document: object) -> None:
    """Depth and well-formed-Unicode checks, iteratively (no recursion)."""
    stack: list[tuple[object, int]] = [(document, 0)]
    while stack:
        value, depth = stack.pop()
        if isinstance(value, str):
            need(
                not _SURROGATE.search(value),
                "malformed-json",
                "string has an unpaired surrogate",
            )
        elif isinstance(value, (dict, list)):
            need(depth < MAX_JSON_DEPTH, "malformed-json", "JSON nests too deeply")
            if isinstance(value, dict):
                for key, item in value.items():
                    need(
                        not _SURROGATE.search(key),
                        "malformed-json",
                        "key has an unpaired surrogate",
                    )
                    stack.append((item, depth + 1))
            else:
                stack.extend((item, depth + 1) for item in value)


def parse_json(data: bytes) -> object:
    """Parse UTF-8 JSON strictly (the VER04i 9.1 reader profile)."""
    try:
        text = data.decode("utf-8", errors="strict")
        document = json.loads(
            text,
            object_pairs_hook=_reject_duplicates,
            parse_constant=_reject_constant,
            parse_float=_reject_float,
            parse_int=_safe_integer,
        )
        _check_shape(document)
        return document
    except CorpusError:
        raise
    except (UnicodeDecodeError, ValueError, RecursionError) as error:
        raise CorpusError("malformed-json", "invalid or truncated JSON") from error


def render_json(document: object) -> str:
    """Canonical rendering: sorted keys, 2-space indent, LF, final newline."""
    return json.dumps(document, indent=2, sort_keys=True) + "\n"
