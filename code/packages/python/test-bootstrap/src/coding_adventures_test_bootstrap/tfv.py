"""TFV v1 parser and serializer (TF01 §2).

Pure Python implementation of the Test Fixture Vocabulary (TFV) line format.
Zero external dependencies.
"""

from __future__ import annotations

import struct
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Any

HEADER = "tfv 1"
MAX_DEPTH = 64
MAX_INT_DIGITS = 1000
MAX_INPUT_BYTES = 1024 * 1024


def _parse_canonical_nat(s: str, name: str) -> int:
    """Parse a strict canonical non-negative integer (no sign, no leading zeros)."""
    if not s or not s.isdigit():
        raise ValueError(f"Invalid non-negative integer for {name}: {s!r}")
    if len(s) > 1 and s.startswith("0"):
        raise ValueError(f"Leading zero in {name}: {s!r}")
    if len(s) > MAX_INT_DIGITS:
        raise ValueError(f"{name} exceeds MAX_INT_DIGITS: {s!r}")
    return int(s)


def _parse_canonical_int(body: str) -> int:
    """Parse a strict canonical integer (no '+', no '_', no leading zero, no '-0')."""
    if not body:
        raise ValueError("Empty integer body")
    if body == "-0":
        raise ValueError("Invalid canonical integer: -0")
    if body.startswith("-"):
        rem = body[1:]
        if not rem or not rem.isdigit():
            raise ValueError(f"Invalid canonical negative integer: {body!r}")
        if rem == "0" or (len(rem) > 1 and rem.startswith("0")):
            raise ValueError(f"Leading zero in canonical negative integer: {body!r}")
        if len(rem) > MAX_INT_DIGITS:
            raise ValueError(f"Integer exceeds MAX_INT_DIGITS: {body!r}")
        return int(body)
    else:
        if not body.isdigit():
            raise ValueError(f"Invalid canonical integer: {body!r}")
        if len(body) > 1 and body.startswith("0"):
            raise ValueError(f"Leading zero in canonical integer: {body!r}")
        if len(body) > MAX_INT_DIGITS:
            raise ValueError(f"Integer exceeds MAX_INT_DIGITS: {body!r}")
        return int(body)


@dataclass(frozen=True)
class TFVRecord:
    """A single TFV record consisting of an identifier and named fields."""

    record_id: str
    fields: dict[str, Any]


def format_value(val: Any, depth: int = 0) -> str:
    """Format a Python value into canonical TFV prefix notation."""
    if depth > MAX_DEPTH:
        raise ValueError(f"Value nesting exceeds MAX_DEPTH ({MAX_DEPTH})")

    if val is None:
        return "N"
    if isinstance(val, bool):
        return "T" if val else "F"
    if isinstance(val, int):
        s = str(val)
        if len(s.lstrip("-")) > MAX_INT_DIGITS:
            raise ValueError(f"Integer exceeds MAX_INT_DIGITS ({MAX_INT_DIGITS})")
        return f"I{s}"
    if isinstance(val, float):
        packed = struct.pack(">d", val)
        return f"D{packed.hex().lower()}"
    if isinstance(val, (bytes, bytearray)):
        return f"X{bytes(val).hex().lower()}"
    if isinstance(val, str):
        return f"S{val.encode('utf-8').hex().lower()}"
    if isinstance(val, (list, tuple)):
        items = [format_value(item, depth + 1) for item in val]
        return f"L{len(val)}" + (" " + " ".join(items) if items else "")
    if isinstance(val, dict):
        pairs: list[str] = []
        for k, v in val.items():
            pairs.append(format_value(k, depth + 1))
            pairs.append(format_value(v, depth + 1))
        return f"M{len(val)}" + (" " + " ".join(pairs) if pairs else "")

    raise TypeError(f"Unsupported TFV value type: {type(val).__name__}")


class _TFVParser:
    def __init__(self, tokens: list[str]) -> None:
        self.tokens = tokens
        self.pos = 0

    def has_more(self) -> bool:
        return self.pos < len(self.tokens)

    def next_token(self) -> str:
        if self.pos >= len(self.tokens):
            raise ValueError("Unexpected end of tokens in TFV stream")
        tok = self.tokens[self.pos]
        self.pos += 1
        return tok

    def parse_value(self, depth: int = 0) -> Any:
        if depth > MAX_DEPTH:
            raise ValueError(f"Value nesting exceeds MAX_DEPTH ({MAX_DEPTH})")

        tok = self.next_token()
        if not tok:
            raise ValueError("Empty token in TFV stream")

        tag = tok[0]
        body = tok[1:]

        if tag == "N":
            if body:
                raise ValueError(f"Unexpected body for tag N: {tok}")
            return None
        if tag == "T":
            if body:
                raise ValueError(f"Unexpected body for tag T: {tok}")
            return True
        if tag == "F":
            if body:
                raise ValueError(f"Unexpected body for tag F: {tok}")
            return False
        if tag == "I":
            return _parse_canonical_int(body)
        if tag == "D":
            if len(body) != 16:
                raise ValueError(f"Float must have exactly 16 hex chars: {tok}")
            return struct.unpack(">d", bytes.fromhex(body))[0]
        if tag == "X":
            return bytes.fromhex(body)
        if tag == "S":
            return bytes.fromhex(body).decode("utf-8")
        if tag == "L":
            count = _parse_canonical_nat(body, "L count")
            return [self.parse_value(depth + 1) for _ in range(count)]
        if tag == "M":
            count = _parse_canonical_nat(body, "M count")
            out: dict[Any, Any] = {}
            for _ in range(count):
                k = self.parse_value(depth + 1)
                v = self.parse_value(depth + 1)
                out[k] = v
            return out

        raise ValueError(f"Unknown TFV tag: {tag} in token {tok}")


def format_record(record: TFVRecord) -> str:
    """Format a single TFV record into its line string."""
    if not record.record_id or any(c.isspace() for c in record.record_id):
        msg = (
            "Invalid record_id: must not be empty or "
            f"contain whitespace: {record.record_id!r}"
        )
        raise ValueError(msg)
    tokens = [record.record_id, f"R{len(record.fields)}"]
    for k, v in record.fields.items():
        if not k or any(c.isspace() for c in k):
            raise ValueError(
                f"Invalid field key: must not be empty or contain whitespace: {k!r}"
            )
        tokens.append(f"K{k}")
        tokens.append(format_value(v))
    return " ".join(tokens)


def format_tfv(records: Sequence[TFVRecord]) -> str:
    """Format a sequence of records into complete TFV document."""
    lines = [HEADER]
    for r in records:
        lines.append(format_record(r))
    return "\n".join(lines) + "\n"


def parse_tfv(text: str) -> list[TFVRecord]:
    """Parse complete TFV text document into records."""
    encoded = text.encode("utf-8")
    if len(encoded) > MAX_INPUT_BYTES:
        msg = (
            f"TFV input size ({len(encoded)} bytes) "
            f"exceeds MAX_INPUT_BYTES ({MAX_INPUT_BYTES})"
        )
        raise ValueError(msg)
    lines = [ln.strip() for ln in text.strip().splitlines() if ln.strip()]
    if not lines:
        return []
    if lines[0] != HEADER:
        raise ValueError(f"Invalid TFV header: expected '{HEADER}', got '{lines[0]}'")

    records: list[TFVRecord] = []
    for line in lines[1:]:
        tokens = line.split(" ")
        if len(tokens) < 2:
            raise ValueError(f"Malformed TFV record line: {line}")
        record_id = tokens[0]
        if not record_id or any(c.isspace() for c in record_id):
            raise ValueError(f"Invalid record_id: {record_id!r}")
        field_count_tok = tokens[1]
        if not field_count_tok.startswith("R"):
            raise ValueError(
                f"Expected R<count> field indicator, got {field_count_tok}"
            )
        field_count = _parse_canonical_nat(field_count_tok[1:], "R field count")

        parser = _TFVParser(tokens[2:])
        fields: dict[str, Any] = {}
        for _ in range(field_count):
            k_tok = parser.next_token()
            if not k_tok.startswith("K"):
                raise ValueError(f"Expected K<name> field key, got {k_tok}")
            key_name = k_tok[1:]
            if not key_name or any(c.isspace() for c in key_name):
                raise ValueError(f"Invalid field key: {key_name!r}")
            val = parser.parse_value()
            fields[key_name] = val

        if parser.has_more():
            raise ValueError("Trailing tokens in TFV record line")

        records.append(TFVRecord(record_id=record_id, fields=fields))

    return records
