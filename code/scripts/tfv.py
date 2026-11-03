#!/usr/bin/env python3
"""TFV v1: the line-oriented corpus transport of TF01 section 2.

Corpora are written as readable JSON (`cases.json`) and compiled into TFV
(`cases.tfv`), a format any language can parse in a few dozen lines with no
JSON library. This module is the Python reference for both directions and is
deliberately standard-library only, so it can serve as an *independent*
oracle for every language lane.

THE VALUE MODEL
===============

A value is a tagged tuple, one tag per TFV kind:

    ("null",)                ("bool", True)          ("int", -12)
    ("f64", 0x3ff0000000000000)                      ("bytes", b"\\x00\\xff")
    ("text", "hello")        ("list", [v, ...])      ("map", [(k, v), ...])

`f64` holds the raw 64-bit IEEE-754 pattern, so -0.0 and every NaN payload
survive exactly.

JSON SHORTHANDS IN cases.json
=============================

A JSON string is text, `true`/`false` are booleans, `null` is null, and a JSON
array is a list. Everything else is an object with an explicit `kind`:

    {"kind": "int", "value": "18446744073709551615"}  # decimal string: no float loss
    {"kind": "f64", "bits": "7ff8000000000000"}
    {"kind": "bytes", "hex": "00ff"}
    {"kind": "map", "entries": [[key, value], ...]}

A bare JSON number is rejected on purpose: JSON numbers are floats in many
parsers, and a corpus must never depend on that.
"""

from __future__ import annotations

import re
from typing import Any

Value = tuple
Record = tuple[str, list[tuple[str, Value]]]

HEADER = "tfv 1"
MAX_DEPTH = 64  # lists and maps nest at most this deep (TF01 section 2.2)
MAX_INT_DIGITS = 1000  # an integer has at most this many digits (TF01 section 2.2)
_ID = re.compile(r"[A-Za-z0-9._/-]+")
_FIELD = re.compile(r"[a-z][a-z0-9_]*")
_INT = re.compile(rf"-?(0|[1-9][0-9]{{0,{MAX_INT_DIGITS - 1}}})")
_HEX = re.compile(r"(?:[0-9a-f]{2})*")
_COUNT = re.compile(r"0|[1-9][0-9]*")


class TfvError(ValueError):
    """A TFV text or a JSON corpus value broke the TF01 rules."""


# ---------------------------------------------------------------------------
# JSON -> value
# ---------------------------------------------------------------------------
def from_json(node: Any, depth: int = 0) -> Value:
    if node is None:
        return ("null",)
    if node is True or node is False:
        return ("bool", node)
    if isinstance(node, str):
        try:
            node.encode("utf-8")
        except UnicodeEncodeError as error:
            raise TfvError("text must be valid Unicode (no lone surrogates)") from error
        return ("text", node)
    is_container = isinstance(node, list) or (
        isinstance(node, dict) and node.get("kind") == "map"
    )
    if is_container and depth >= MAX_DEPTH:
        raise TfvError(f"lists and maps nest at most {MAX_DEPTH} deep")
    if isinstance(node, list):
        return ("list", [from_json(item, depth + 1) for item in node])
    if not isinstance(node, dict) or "kind" not in node:
        raise TfvError(f"not a TFV JSON value (bare numbers are not allowed): {node!r}")
    kind = node["kind"]
    if not isinstance(kind, str):
        raise TfvError(f"kind must be a string: {kind!r}")
    expected_keys = {
        "int": {"kind", "value"},
        "f64": {"kind", "bits"},
        "bytes": {"kind", "hex"},
        "map": {"kind", "entries"},
    }.get(kind)
    if expected_keys is None:
        raise TfvError(f"unknown kind {kind!r}")
    if set(node) != expected_keys:
        raise TfvError(f"{kind} value must have exactly {sorted(expected_keys)}")
    if kind == "int":
        text = node["value"]
        if not isinstance(text, str) or not _INT.fullmatch(text) or text == "-0":
            raise TfvError(f"int value must be a canonical decimal string: {text!r}")
        return ("int", int(text))
    if kind == "f64":
        bits = node["bits"]
        if not isinstance(bits, str) or not re.fullmatch(r"[0-9a-f]{16}", bits):
            raise TfvError(f"f64 bits must be 16 lowercase hex digits: {bits!r}")
        return ("f64", int(bits, 16))
    if kind == "bytes":
        text = node["hex"]
        if not isinstance(text, str) or not _HEX.fullmatch(text):
            raise TfvError(f"bytes hex must be lowercase and even-length: {text!r}")
        return ("bytes", bytes.fromhex(text))
    entries = node["entries"]
    if not isinstance(entries, list) or any(
        not isinstance(pair, list) or len(pair) != 2 for pair in entries
    ):
        raise TfvError("map entries must be [key, value] pairs")
    pairs = [(from_json(k, depth + 1), from_json(v, depth + 1)) for k, v in entries]
    _require_distinct_keys(pairs)
    return ("map", pairs)


def _require_distinct_keys(pairs: list[tuple[Value, Value]]) -> None:
    for i, (key, _) in enumerate(pairs):
        for other, _ in pairs[i + 1 :]:
            if values_equal(key, other):
                raise TfvError("map has two deep-equal keys")


# ---------------------------------------------------------------------------
# value -> TFV tokens
# ---------------------------------------------------------------------------
def encode(value: Value) -> list[str]:
    kind = value[0]
    if kind == "null":
        return ["N"]
    if kind == "bool":
        return ["T" if value[1] else "F"]
    if kind == "int":
        return [f"I{value[1]}"]
    if kind == "f64":
        return [f"D{value[1]:016x}"]
    if kind == "bytes":
        return ["X" + value[1].hex()]
    if kind == "text":
        return ["S" + value[1].encode("utf-8").hex()]
    if kind == "list":
        out = [f"L{len(value[1])}"]
        for item in value[1]:
            out += encode(item)
        return out
    if kind == "map":
        out = [f"M{len(value[1])}"]
        for key, item in value[1]:
            out += encode(key) + encode(item)
        return out
    raise TfvError(f"unknown kind {kind!r}")


def encode_record(record_id: str, fields: list[tuple[str, Value]]) -> str:
    if not _ID.fullmatch(record_id):
        raise TfvError(f"bad record id {record_id!r}")
    names = [name for name, _ in fields]
    if len(set(names)) != len(names) or not all(_FIELD.fullmatch(n) for n in names):
        raise TfvError(f"{record_id}: bad or duplicate field names {names}")
    tokens = [record_id, f"R{len(fields)}"]
    for name, value in fields:
        tokens += [f"K{name}"] + encode(value)
    return " ".join(tokens)


def encode_file(records: list[Record]) -> str:
    ids = [record_id for record_id, _ in records]
    if len(set(ids)) != len(ids):
        raise TfvError("duplicate record id")
    return "\n".join([HEADER] + [encode_record(i, f) for i, f in records]) + "\n"


# ---------------------------------------------------------------------------
# TFV text -> records (strict)
# ---------------------------------------------------------------------------
def parse_file(text: str) -> list[Record]:
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    if not lines or lines[0] != HEADER:
        raise TfvError("first line must be exactly 'tfv 1'")
    records: list[Record] = []
    seen: set[str] = set()
    for number, line in enumerate(lines[1:], start=2):
        if line == "" or line.startswith("#"):
            continue
        record = _parse_record(line, number)
        if record[0] in seen:
            raise TfvError(f"line {number}: duplicate record id {record[0]!r}")
        seen.add(record[0])
        records.append(record)
    return records


def _parse_record(line: str, number: int) -> Record:
    if "\r" in line:
        raise TfvError(f"line {number}: carriage return")
    tokens = line.split(" ")
    if any(token == "" for token in tokens):
        raise TfvError(
            f"line {number}: empty token (double, leading or trailing space)"
        )
    if len(tokens) < 2 or not _ID.fullmatch(tokens[0]):
        raise TfvError(f"line {number}: bad record id")
    record_id, head = tokens[0], tokens[1]
    if not head.startswith("R") or not _COUNT.fullmatch(head[1:]):
        raise TfvError(f"line {number}: expected R<count>")
    position = 2
    fields: list[tuple[str, Value]] = []
    for _ in range(_count(head[1:], number)):
        if position >= len(tokens):
            raise TfvError(f"line {number}: record ends early")
        key = tokens[position]
        if not key.startswith("K") or not _FIELD.fullmatch(key[1:]):
            raise TfvError(f"line {number}: expected K<field>, got {key!r}")
        if any(name == key[1:] for name, _ in fields):
            raise TfvError(f"line {number}: duplicate field {key[1:]!r}")
        value, position = _parse_value(tokens, position + 1, number, 0)
        fields.append((key[1:], value))
    if position != len(tokens):
        raise TfvError(f"line {number}: trailing tokens")
    return record_id, fields


def _count(body: str, number: int) -> int:
    # Rust parses a count into usize and rejects overflow; 20 digits is the
    # most a 64-bit count can have. Checking length first keeps a huge count
    # from ever reaching int(), whose digit limit would raise a plain
    # ValueError instead of TfvError.
    if len(body) > 20:
        raise TfvError(f"line {number}: count too large")
    return int(body)


def _parse_value(
    tokens: list[str], position: int, number: int, depth: int
) -> tuple[Value, int]:
    if position >= len(tokens):
        raise TfvError(f"line {number}: value missing")
    token = tokens[position]
    tag, body = token[0], token[1:]
    if tag in "NTF" and body == "":
        return (
            {"N": ("null",), "T": ("bool", True), "F": ("bool", False)}[tag],
            position + 1,
        )
    if tag == "I" and _INT.fullmatch(body) and body != "-0":
        return ("int", int(body)), position + 1
    if tag == "D" and re.fullmatch(r"[0-9a-f]{16}", body):
        return ("f64", int(body, 16)), position + 1
    if tag == "X" and _HEX.fullmatch(body):
        return ("bytes", bytes.fromhex(body)), position + 1
    if tag == "S" and _HEX.fullmatch(body):
        try:
            return ("text", bytes.fromhex(body).decode("utf-8")), position + 1
        except UnicodeDecodeError as error:
            raise TfvError(f"line {number}: text is not valid UTF-8") from error
    if tag in "LM" and _COUNT.fullmatch(body):
        if depth >= MAX_DEPTH:
            raise TfvError(
                f"line {number}: lists and maps nest at most {MAX_DEPTH} deep"
            )
        count, position = _count(body, number), position + 1
        if tag == "L":
            items = []
            for _ in range(count):
                item, position = _parse_value(tokens, position, number, depth + 1)
                items.append(item)
            return ("list", items), position
        pairs = []
        for _ in range(count):
            key, position = _parse_value(tokens, position, number, depth + 1)
            item, position = _parse_value(tokens, position, number, depth + 1)
            pairs.append((key, item))
        try:
            _require_distinct_keys(pairs)
        except TfvError as error:
            raise TfvError(f"line {number}: {error}") from error
        return ("map", pairs), position
    raise TfvError(f"line {number}: malformed token {token!r}")


# ---------------------------------------------------------------------------
# deep equality (TF01 section 3.1), the reference semantics
# ---------------------------------------------------------------------------
_NAN_CANON = 0x7FF8000000000000


def _canon_f64(bits: int) -> int:
    exponent = (bits >> 52) & 0x7FF
    mantissa = bits & ((1 << 52) - 1)
    return _NAN_CANON if exponent == 0x7FF and mantissa != 0 else bits


def values_equal(a: Value, b: Value) -> bool:
    if a[0] != b[0]:
        return False
    kind = a[0]
    if kind == "null":
        return True
    if kind in ("bool", "int", "bytes", "text"):
        return a[1] == b[1]
    if kind == "f64":
        return _canon_f64(a[1]) == _canon_f64(b[1])
    if kind == "list":
        return len(a[1]) == len(b[1]) and all(
            values_equal(x, y) for x, y in zip(a[1], b[1])
        )
    if kind == "map":
        if len(a[1]) != len(b[1]):
            return False
        for key, item in a[1]:
            match = [v for k, v in b[1] if values_equal(key, k)]
            if len(match) != 1 or not values_equal(item, match[0]):
                return False
        return True
    raise TfvError(f"unknown kind {kind!r}")
