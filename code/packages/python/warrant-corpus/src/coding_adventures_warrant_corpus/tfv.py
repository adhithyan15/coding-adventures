"""A minimal TFV v1 reader (TF01 section 2) for ``cases.tfv``.

TFV is the repository's language-neutral line format.  This reader is written
from the format's rules, not copied from ``code/scripts/tfv.py``, so a bug in
the reference script cannot hide a bug here (and vice versa).  It accepts only
the value kinds the Warrant corpus uses and rejects the rest.

A record is one line::

    <id> R<count> K<name> <value> K<name> <value> ...

Values are space-separated tokens::

    N            null            T / F           booleans
    I<decimal>   integer         X<hex>          bytes ("X" alone is empty)
    S<hex>       UTF-8 text      L<n> v1 .. vn   list
    M<n> k1 v1 .. kn vn          map

Because lists and maps announce their length, the parser is a small recursive
descent; it never has to guess where a value ends.
"""

from __future__ import annotations

import re

from .errors import CorpusError, need

HEADER = "tfv 1"
MAX_DEPTH = 64
_ID = re.compile(r"[A-Za-z0-9._/@#-]+")
_FIELD = re.compile(r"[a-z][a-z0-9_]*")
_INT = re.compile(r"-?(?:0|[1-9][0-9]*)")
_HEX = re.compile(r"(?:[0-9a-f]{2})*")
_COUNT = re.compile(r"0|[1-9][0-9]*")

# ("bool", b) ("int", n) ("bytes", b) ("text", s) ("list", [...]) ("map", [...])
Value = tuple


def _value(tokens: list[str], at: int, depth: int) -> tuple[Value, int]:
    need(at < len(tokens), "tfv", "record ends inside a value")
    need(depth <= MAX_DEPTH, "tfv", "value nests too deeply")
    token = tokens[at]
    tag, body = token[0], token[1:]
    if tag == "N" and not body:
        return ("null",), at + 1
    if tag == "T" and not body:
        return ("bool", True), at + 1
    if tag == "F" and not body:
        return ("bool", False), at + 1
    if tag == "I":
        need(_INT.fullmatch(body) is not None and body != "-0", "tfv", "bad integer")
        return ("int", int(body)), at + 1
    if tag == "X":
        need(_HEX.fullmatch(body) is not None, "tfv", "bad bytes")
        return ("bytes", bytes.fromhex(body)), at + 1
    if tag == "S":
        need(_HEX.fullmatch(body) is not None, "tfv", "bad text")
        try:
            return ("text", bytes.fromhex(body).decode("utf-8")), at + 1
        except UnicodeDecodeError as error:
            raise CorpusError("tfv", "text is not valid UTF-8") from error
    if tag in ("L", "M"):
        need(_COUNT.fullmatch(body) is not None, "tfv", "bad container length")
        count = int(body)
        need(count <= len(tokens), "tfv", "container length exceeds the record")
        at += 1
        if tag == "L":
            items = []
            for _ in range(count):
                item, at = _value(tokens, at, depth + 1)
                items.append(item)
            return ("list", items), at
        pairs = []
        for _ in range(count):
            key, at = _value(tokens, at, depth + 1)
            item, at = _value(tokens, at, depth + 1)
            pairs.append((key, item))
        return ("map", pairs), at
    raise CorpusError("tfv", "unsupported TFV value kind")


def parse_tfv(text: str) -> list[tuple[str, dict[str, Value]]]:
    """Parse a whole ``cases.tfv`` into ``[(record id, {field: value})]``."""
    lines = text.split("\n")
    need(lines and lines[0] == HEADER, "tfv", "missing `tfv 1` header")
    need(lines[-1] == "", "tfv", "TFV must end with a newline")
    records: list[tuple[str, dict[str, Value]]] = []
    seen: set[str] = set()
    for line in lines[1:-1]:
        need(line != "" and "\r" not in line, "tfv", "blank or CR line in TFV")
        tokens = line.split(" ")
        need(len(tokens) >= 2, "tfv", "record is too short")
        record_id = tokens[0]
        need(_ID.fullmatch(record_id) is not None, "tfv", "bad record id")
        need(record_id not in seen, "tfv", "duplicate record id")
        seen.add(record_id)
        need(
            tokens[1].startswith("R") and _COUNT.fullmatch(tokens[1][1:]) is not None,
            "tfv",
            "bad field count",
        )
        count = int(tokens[1][1:])
        fields: dict[str, Value] = {}
        at = 2
        for _ in range(count):
            need(
                at < len(tokens) and tokens[at].startswith("K"),
                "tfv",
                "expected a field key",
            )
            name = tokens[at][1:]
            need(_FIELD.fullmatch(name) is not None, "tfv", "bad field name")
            need(name not in fields, "tfv", "duplicate field")
            fields[name], at = _value(tokens, at + 1, 0)
        need(at == len(tokens), "tfv", "trailing tokens after the last field")
        records.append((record_id, fields))
    return records
