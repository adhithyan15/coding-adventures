#!/usr/bin/env python3
"""Independent stdlib checker for ``warrant-corpus-conformance-v1`` (VER04i 9.1).

What this is
============

Every lane's ``warrant-corpus`` runner must pass the same shared cases: a
strict JSON reader, a canonical writer, a tagged-value decoder, a containment
check and a label digest.  The cases live as data in
``code/specs/fixtures/warrant-corpus-conformance-v1/``.  *Data nobody
cross-checks is a rumour*, so this script re-derives every expected outcome
with **its own reference implementation**, which shares no code with any
lane (it imports nothing from ``coding_adventures_warrant_corpus`` and uses the
stdlib ``json`` module for nothing but its own sanity cross-check).

Two modes::

    python3 code/scripts/warrant_corpus_conformance.py            # check
    python3 code/scripts/warrant_corpus_conformance.py --write    # regenerate

``--write`` builds every file from the case table in this script (inputs and
the *intent* of each case), and ``check`` rebuilds the same bytes in memory and
requires the files on disk to be identical, then runs structural checks
(unique ids, closed reason set, index counts and digests, line endings).

The table states an **intent** for every case ("accept" or the reject reason)
*and* the reference implementation derives the outcome independently.  The
two must agree, which is double-entry bookkeeping: a typo in the table and a
bug in the reference are very unlikely to cancel out.

Reading order
=============

1. the reference JSON reader (bytes in, value or ``Reject`` out);
2. the reference canonical writer;
3. the reference tagged-value decoder and ``warrant-stream-v1`` encoder;
4. the reference lexical containment check;
5. the label digest;
6. the case tables;
7. building, writing and checking the fixture.
"""

from __future__ import annotations

import argparse
import hashlib
import ntpath
import posixpath
import re
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE = ROOT / "code/specs/fixtures/warrant-corpus-conformance-v1"
SCHEMA = "warrant-corpus-conformance-v1"

SAFE_MAX = 2**53 - 1
MAX_DEPTH = 256  # nested containers; the 257th is rejected (VER04i 9.1)
U64_MAX = 2**64 - 1
CANONICAL_TEXT_LIMIT = 2048  # larger renderings are pinned by digest alone

REASONS = frozenset(
    {
        "empty",
        "syntax",
        "literal",
        "utf8",
        "bom",
        "trailing-data",
        "control-char",
        "bad-escape",
        "duplicate-key",
        "unpaired-surrogate",
        "number-form",
        "unsafe-integer",
        "depth",
    }
)


class Reject(Exception):
    """The reference reader refused its input; ``reason`` says why."""

    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


# ---------------------------------------------------------------------------
# 1. The reference reader
# ---------------------------------------------------------------------------
#
# Step one is bytes -> code points, strictly.  UTF-8 is a prefix code: the first
# byte says how many continuation bytes follow, and each decoded value must be
# the *shortest* encoding of itself (otherwise 0xC0 0x80 would be a second way
# to spell NUL, which is how overlong encodings sneak past filters).
#
#   first byte   length   code points          notes
#   00..7F       1        U+0000..U+007F
#   C2..DF       2        U+0080..U+07FF       C0, C1 would be overlong
#   E0..EF       3        U+0800..U+FFFF       minus U+D800..U+DFFF
#   F0..F4       4        U+10000..U+10FFFF    F5.. would exceed U+10FFFF
def decode_utf8(data: bytes) -> list[int]:
    out: list[int] = []
    at = 0
    while at < len(data):
        first = data[at]
        if first < 0x80:
            out.append(first)
            at += 1
            continue
        if 0xC2 <= first <= 0xDF:
            extra, value, least = 1, first & 0x1F, 0x80
        elif 0xE0 <= first <= 0xEF:
            extra, value, least = 2, first & 0x0F, 0x800
        elif 0xF0 <= first <= 0xF4:
            extra, value, least = 3, first & 0x07, 0x10000
        else:
            raise Reject("utf8")  # stray continuation, C0/C1, F5..FF
        if at + extra >= len(data):
            raise Reject("utf8")  # truncated
        for step in range(1, extra + 1):
            byte = data[at + step]
            if not 0x80 <= byte <= 0xBF:
                raise Reject("utf8")
            value = (value << 6) | (byte & 0x3F)
        if value < least or 0xD800 <= value <= 0xDFFF or value > 0x10FFFF:
            raise Reject("utf8")  # overlong, encoded surrogate, out of range
        out.append(value)
        at += extra + 1
    return out


_WHITESPACE = {0x20, 0x09, 0x0A, 0x0D}
_SIMPLE_ESCAPES = {
    ord('"'): '"',
    ord("\\"): "\\",
    ord("/"): "/",
    ord("b"): "\b",
    ord("f"): "\f",
    ord("n"): "\n",
    ord("r"): "\r",
    ord("t"): "\t",
}
_HEX_DIGITS = {ord(c) for c in "0123456789abcdefABCDEF"}


class _Reader:
    """A recursive-descent parser over code points (see :func:`read_json`)."""

    def __init__(self, cps: list[int]) -> None:
        self.cps = cps
        self.at = 0

    # -- helpers ---------------------------------------------------------
    def peek(self) -> int | None:
        return self.cps[self.at] if self.at < len(self.cps) else None

    def space(self) -> None:
        while self.peek() in _WHITESPACE:
            self.at += 1

    def literal(self, word: str, value: Any) -> Any:
        for char in word:
            if self.peek() != ord(char):
                raise Reject("literal")
            self.at += 1
        return value

    # -- grammar ---------------------------------------------------------
    def value(self, depth: int) -> Any:
        self.space()
        cp = self.peek()
        if cp is None:
            raise Reject("syntax")
        char = chr(cp)
        if char == "{":
            return self.obj(depth)
        if char == "[":
            return self.arr(depth)
        if char == '"':
            return self.string()
        if char == "t":
            return self.literal("true", True)
        if char == "f":
            return self.literal("false", False)
        if char == "n":
            return self.literal("null", None)
        if char == "-" or "0" <= char <= "9":
            return self.number()
        if cp == 0xFEFF:
            raise Reject("bom")
        if char in "+.":
            raise Reject("number-form")
        if char.isalpha():  # NaN, Infinity, True, undefined ...
            raise Reject("literal")
        raise Reject("syntax")

    def number(self) -> int:
        start = self.at
        if self.peek() == ord("-"):
            self.at += 1
        cp = self.peek()
        if cp is not None and chr(cp) == "0":
            self.at += 1
            nxt = self.peek()
            if nxt is not None and ord("0") <= nxt <= ord("9"):
                raise Reject("number-form")  # leading zero
        elif cp is not None and ord("1") <= cp <= ord("9"):
            while self.peek() is not None and ord("0") <= self.peek() <= ord("9"):  # type: ignore[operator]
                self.at += 1
        elif cp is not None and chr(cp).isalpha():
            raise Reject("literal")  # -Infinity
        else:
            raise Reject("number-form")
        nxt = self.peek()
        if nxt is not None and chr(nxt) in ".eE":
            raise Reject("number-form")  # no fraction, no exponent, ever
        text = "".join(chr(c) for c in self.cps[start : self.at])
        if text == "-0":
            raise Reject("number-form")
        number = int(text)
        if abs(number) > SAFE_MAX:
            raise Reject("unsafe-integer")
        return number

    def hex4(self) -> int:
        digits = self.cps[self.at : self.at + 4]
        if len(digits) != 4 or any(d not in _HEX_DIGITS for d in digits):
            raise Reject("bad-escape")
        self.at += 4
        return int("".join(chr(d) for d in digits), 16)

    def string(self) -> str:
        self.at += 1  # opening quote
        out: list[str] = []
        while True:
            cp = self.peek()
            if cp is None:
                raise Reject("syntax")  # unterminated
            self.at += 1
            if cp == ord('"'):
                return "".join(out)
            if cp < 0x20:
                raise Reject("control-char")
            if cp != ord("\\"):
                out.append(chr(cp))
                continue
            esc = self.peek()
            if esc is None:
                raise Reject("bad-escape")
            self.at += 1
            if esc in _SIMPLE_ESCAPES:
                out.append(_SIMPLE_ESCAPES[esc])
            elif esc == ord("u"):
                unit = self.hex4()
                if 0xD800 <= unit <= 0xDBFF:
                    # A high surrogate needs its low half *immediately*.
                    if self.cps[self.at : self.at + 2] != [ord("\\"), ord("u")]:
                        raise Reject("unpaired-surrogate")
                    self.at += 2
                    low = self.hex4()
                    if not 0xDC00 <= low <= 0xDFFF:
                        raise Reject("unpaired-surrogate")
                    out.append(chr(0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)))
                elif 0xDC00 <= unit <= 0xDFFF:
                    raise Reject("unpaired-surrogate")
                else:
                    out.append(chr(unit))
            else:
                raise Reject("bad-escape")

    def arr(self, depth: int) -> list[Any]:
        if depth >= MAX_DEPTH:
            raise Reject("depth")
        self.at += 1
        items: list[Any] = []
        self.space()
        if self.peek() == ord("]"):
            self.at += 1
            return items
        while True:
            items.append(self.value(depth + 1))
            self.space()
            cp = self.peek()
            self.at += 1
            if cp == ord("]"):
                return items
            if cp != ord(","):
                raise Reject("syntax")

    def obj(self, depth: int) -> dict[str, Any]:
        if depth >= MAX_DEPTH:
            raise Reject("depth")
        self.at += 1
        result: dict[str, Any] = {}
        self.space()
        if self.peek() == ord("}"):
            self.at += 1
            return result
        while True:
            self.space()
            if self.peek() != ord('"'):
                raise Reject("syntax")
            key = self.string()
            if key in result:  # compared AFTER unescaping
                raise Reject("duplicate-key")
            self.space()
            if self.peek() != ord(":"):
                raise Reject("syntax")
            self.at += 1
            result[key] = self.value(depth + 1)
            self.space()
            cp = self.peek()
            self.at += 1
            if cp == ord("}"):
                return result
            if cp != ord(","):
                raise Reject("syntax")


def read_json(data: bytes) -> Any:
    """Parse ``data`` under the VER04i 9.1 reader profile, or raise Reject."""
    cps = decode_utf8(data)
    if cps and cps[0] == 0xFEFF:
        raise Reject("bom")
    reader = _Reader(cps)
    reader.space()
    if reader.peek() is None:
        raise Reject("empty")
    value = reader.value(0)
    reader.space()
    if reader.peek() is not None:
        raise Reject("trailing-data")
    return value


# ---------------------------------------------------------------------------
# 2. The reference canonical writer
# ---------------------------------------------------------------------------
#
# Rules (VER04i 9.1): keys sorted by code point; two-space indent; ``,`` LF
# between members; ``: `` after a key; ``[]``/``{}`` when empty; every char
# outside U+0020..U+007E escaped (lowercase hex; astral = surrogate pair).
_SHORT = {'"': '\\"', "\\": "\\\\", "\b": "\\b", "\f": "\\f", "\n": "\\n", "\r": "\\r", "\t": "\\t"}


def _escape(text: str) -> str:
    out = ['"']
    for char in text:
        if char in _SHORT:
            out.append(_SHORT[char])
        elif 0x20 <= ord(char) <= 0x7E:
            out.append(char)
        elif ord(char) <= 0xFFFF:
            out.append(f"\\u{ord(char):04x}")
        else:
            high = 0xD800 + ((ord(char) - 0x10000) >> 10)
            low = 0xDC00 + ((ord(char) - 0x10000) & 0x3FF)
            out.append(f"\\u{high:04x}\\u{low:04x}")
    out.append('"')
    return "".join(out)


def render_json(value: Any, level: int = 0) -> str:
    """Canonical text of ``value`` (no trailing newline; see :func:`canonical`)."""
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return _escape(value)
    pad, inner = "  " * level, "  " * (level + 1)
    if isinstance(value, list):
        if not value:
            return "[]"
        body = ",\n".join(inner + render_json(item, level + 1) for item in value)
        return f"[\n{body}\n{pad}]"
    if not value:
        return "{}"
    body = ",\n".join(
        f"{inner}{_escape(key)}: {render_json(value[key], level + 1)}"
        for key in sorted(value)  # str ordering IS code point ordering
    )
    return f"{{\n{body}\n{pad}}}"


def canonical(value: Any) -> str:
    """The canonical document: the rendering plus one final LF."""
    return render_json(value) + "\n"


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# ---------------------------------------------------------------------------
# 3. Tagged values and the warrant-stream-v1 encoding (VER04i 2.6, 2.7)
# ---------------------------------------------------------------------------
class ValueReject(Exception):
    """A well-formed JSON value that is not a valid tagged Warrant value."""


_DECIMAL = re.compile(r"0|[1-9][0-9]*")
_HEXTEXT = re.compile(r"(?:[0-9a-f]{2})*")
_BYTES_T = re.compile(r"Bytes(?:<([0-9]+)>)?")
_UPPER = {"U8": 255, "U64": U64_MAX, "Nat": U64_MAX}


def _split_result(text: str) -> tuple[str, str]:
    inner = text[len("Result<") : -1]
    depth = 0
    for index, char in enumerate(inner):
        depth += char == "<"
        depth -= char == ">"
        if char == "," and depth == 0:
            return inner[:index], inner[index + 1 :]
    raise ValueReject("type")


def _u32(number: int) -> bytes:
    return number.to_bytes(4, "little")


def decode_and_encode(raw: Any, type_text: str, errors: dict[str, list[str]]) -> bytes:
    """Decode ``raw`` as ``type_text`` and return its stream bytes."""
    if type_text == "Bool":
        if type(raw) is not bool:
            raise ValueReject("Bool must be a JSON boolean")
        return b"\x01" if raw else b"\x00"
    if type_text.startswith("Result<"):
        ok_type, error_type = _split_result(type_text)
        if not (type(raw) is dict and set(raw) == {"type", "value"} and raw["type"] == type_text):
            raise ValueReject("Result must be tagged")
        body = raw["value"]
        if not (type(body) is dict and len(body) == 1):
            raise ValueReject("Result body")
        if "ok" in body:
            return b"\x50" + decode_and_encode(body["ok"], ok_type, errors)
        variant = body.get("err")
        if not (type(variant) is str and variant in errors.get(error_type, [])):
            raise ValueReject("undeclared error variant")
        name = variant.encode("utf-8")
        return b"\x51" + _u32(len(name)) + name
    if not (type(raw) is dict and set(raw) == {"type", "value"} and raw["type"] == type_text):
        raise ValueReject("tagged object expected")
    text = raw["value"]
    match = _BYTES_T.fullmatch(type_text)
    if match:
        if not (type(text) is str and _HEXTEXT.fullmatch(text)):
            raise ValueReject("Bytes must be lowercase even-length hex")
        data = bytes.fromhex(text)
        if match.group(1) is not None and len(data) != int(match.group(1)):
            raise ValueReject("Bytes width")
        return b"\x20" + _u32(len(data)) + data
    if type_text in _UPPER:
        if not (type(text) is str and _DECIMAL.fullmatch(text)):
            raise ValueReject("canonical decimal text expected")
        number = int(text)
        if number > _UPPER[type_text]:
            raise ValueReject("out of range")
        return b"\x10" + number.to_bytes(8, "little")
    raise ValueReject("type")


# ---------------------------------------------------------------------------
# 4. The reference lexical containment check
# ---------------------------------------------------------------------------
#
# A containment check answers "is CHILD at or beneath ROOT?" *by text*, after
# normalising "." and "..".  It must never fall for:
#
# * another drive / share (no common root at all);
# * a ".." that merely begins a name ("..foo" is an ordinary directory);
# * a string prefix that is not a path prefix ("/rr" vs "/r");
# * a ".." at the root (it stays at the root: there is no parent of "/").
#
# ``split`` returns (anchor, components) or None for a device-namespace path,
# which is never inside anything (and nothing is inside it).
def _walk(parts: list[str]) -> list[str]:
    out: list[str] = []
    for part in parts:
        if part in ("", "."):
            continue
        if part == "..":
            if out:
                out.pop()
            continue
        out.append(part)
    return out


def split_path(flavor: str, path: str) -> tuple[str, list[str]] | None:
    if flavor == "posix":
        if not path.startswith("/"):
            raise ValueError("fixture paths are absolute")
        return "/", _walk(path.split("/"))
    text = path.replace("/", "\\")
    if text.startswith("\\\\?\\") or text.startswith("\\\\.\\"):
        return None
    if text.startswith("\\\\"):
        pieces = text[2:].split("\\")
        if len(pieces) < 2 or not pieces[0] or not pieces[1]:
            raise ValueError("malformed UNC path")
        return "\\\\" + pieces[0].lower() + "\\" + pieces[1].lower(), _walk(
            [p.lower() for p in pieces[2:]]
        )
    if len(text) >= 3 and text[0].isalpha() and text[1] == ":" and text[2] == "\\":
        return text[:2].lower(), _walk([p.lower() for p in text[3:].split("\\")])
    raise ValueError("fixture paths are absolute")


def contains(flavor: str, root: str, child: str) -> dict[str, bool]:
    """``{"inside": ..., "same": ...}`` for ``child`` against ``root``."""
    a, b = split_path(flavor, root), split_path(flavor, child)
    if a is None or b is None:
        return {"inside": False, "same": False}
    inside = a[0] == b[0] and b[1][: len(a[1])] == a[1]
    return {"inside": inside, "same": inside and a[1] == b[1]}


# ---------------------------------------------------------------------------
# 5. The label digest (VER04i 8, 9.1)
# ---------------------------------------------------------------------------
def label_digest(manifest: dict[str, Any]) -> str:
    """SHA-256 of the canonical manifest with ``toolchain.version`` blanked."""
    stable = dict(manifest)
    stable["toolchain"] = {"name": manifest["toolchain"]["name"], "version": ""}
    return sha256_hex(canonical(stable).encode("utf-8"))


# ---------------------------------------------------------------------------
# 6. The case tables
# ---------------------------------------------------------------------------
# JSON cases: (slug, summary, input bytes, intent) where intent is None for
# "accept" or the reject reason.  The reference derives the outcome; the table
# only states what we *meant*.
def _nest_arrays(count: int, leaf: bytes = b"") -> bytes:
    return b"[" * count + leaf + b"]" * count


def _nest_objects(count: int) -> bytes:
    return b'{"a":' * (count - 1) + b"{}" + b"}" * (count - 1)


def _alternating(count: int) -> bytes:
    """``count`` containers, alternating array / object, innermost empty."""
    text = b"[]" if (count - 1) % 2 == 0 else b"{}"
    for index in range(count - 2, -1, -1):
        text = b"[" + text + b"]" if index % 2 == 0 else b'{"a":' + text + b"}"
    return text


JsonCase = tuple[str, str, bytes, "str | None"]

JSON_CASES: list[JsonCase] = [
    # --- plain documents --------------------------------------------------
    ("scalar-null", "a bare null is a document", b"null", None),
    ("scalar-true", "a bare true is a document", b"true", None),
    ("scalar-false", "a bare false is a document", b"false", None),
    ("scalar-zero", "zero", b"0", None),
    ("scalar-negative", "a negative integer", b"-7", None),
    ("scalar-empty-string", "the empty string", b'""', None),
    ("empty-array", "an empty array renders as []", b"[]", None),
    ("empty-object", "an empty object renders as {}", b"{}", None),
    ("whitespace-all-four", "space, tab, CR and LF are all whitespace", b" \t\r\n[ \t\r\n1 \t\r\n] \t\r\n", None),
    ("crlf-between-tokens", "CRLF input becomes LF output", b'{\r\n"a":\r\n1\r\n}\r\n', None),
    ("nested-document", "indentation and sorting of a mixed document", b'{"c":[1,[2,3],{"d":null}],"b":{},"a":[]}', None),
    ("nested-objects", "two-space indent per level", b'{"a":{"b":{"c":[true,false]}}}', None),
    ("tagged-u64-text", "a tagged U64 is decimal TEXT and passes through untouched", b'{"type":"U64","value":"18446744073709551615"}', None),
    ("large-number-as-string", "digits inside a string are not a number", b'"9007199254740993"', None),
    # --- integers ----------------------------------------------------------
    ("int-max-safe", "2^53 - 1 is the largest accepted integer", b"9007199254740991", None),
    ("int-min-safe", "-(2^53 - 1) is the smallest accepted integer", b"-9007199254740991", None),
    ("int-array", "negative, zero and positive integers", b"[-1,0,1]", None),
    ("int-2^53", "2^53 is refused, not rounded", b"9007199254740992", "unsafe-integer"),
    ("int-neg-2^53", "-2^53 is refused", b"-9007199254740992", "unsafe-integer"),
    ("int-2^53+1", "2^53 + 1 would silently round to 2^53 in a double", b"9007199254740993", "unsafe-integer"),
    ("int-i64-max", "2^63 - 1", b"9223372036854775807", "unsafe-integer"),
    ("int-i64-overflow", "2^63", b"9223372036854775808", "unsafe-integer"),
    ("int-i64-min", "-2^63", b"-9223372036854775808", "unsafe-integer"),
    ("int-u64-max", "2^64 - 1 as a JSON number (U64 travels as text)", b"18446744073709551615", "unsafe-integer"),
    ("int-u64-overflow", "2^64", b"18446744073709551616", "unsafe-integer"),
    ("int-400-digits", "an integer of 401 digits", b"1" + b"0" * 400, "unsafe-integer"),
    ("int-unsafe-in-array", "an unsafe integer nested in an array", b"[1,9007199254740992]", "unsafe-integer"),
    ("int-unsafe-in-object", "an unsafe integer nested in an object", b'{"count":9007199254740993}', "unsafe-integer"),
    ("tagged-u64-as-number", "a U64 written as a JSON number is refused", b'{"type":"U64","value":18446744073709551615}', "unsafe-integer"),
    # --- numbers that are not integers ---------------------------------------
    ("num-minus-zero", "-0 is refused (it has no integer spelling)", b"-0", "number-form"),
    ("num-minus-zero-float", "-0.0", b"-0.0", "number-form"),
    ("num-fraction", "1.0 is a fraction even though it equals 1", b"1.0", "number-form"),
    ("num-fraction-half", "0.5", b"0.5", "number-form"),
    ("num-fraction-nested", "a fraction nested in an object", b'{"a":1.5}', "number-form"),
    ("num-fraction-in-array", "a fraction nested in an array", b"[1.0]", "number-form"),
    ("num-exponent", "1e2 is an exponent even though it equals 100", b"1e2", "number-form"),
    ("num-exponent-plus", "1E+2", b"1E+2", "number-form"),
    ("num-exponent-minus", "1e-2", b"1e-2", "number-form"),
    ("num-zero-exponent", "0e0", b"0e0", "number-form"),
    ("num-huge-exponent", "1e400 overflows a double to infinity", b"1e400", "number-form"),
    ("num-fraction-exponent", "1.5e3", b"1.5e3", "number-form"),
    ("num-trailing-dot", "1. has no digits after the point", b"1.", "number-form"),
    ("num-leading-dot", ".5 has no integer part", b".5", "number-form"),
    ("num-leading-plus", "+1", b"+1", "number-form"),
    ("num-leading-zero", "01", b"01", "number-form"),
    ("num-leading-zero-negative", "-01", b"-01", "number-form"),
    ("num-double-zero", "00", b"00", "number-form"),
    ("num-bare-minus", "a lone minus sign", b"-", "number-form"),
    ("num-minus-space", "- 1", b"- 1", "number-form"),
    ("num-double-minus", "--1", b"--1", "number-form"),
    ("num-bare-exponent", "1e with no digits", b"1e", "number-form"),
    ("num-hex", "0x10 reads as 0 then trailing data", b"0x10", "trailing-data"),
    ("num-underscore", "1_000 reads as 1 then trailing data", b"1_000", "trailing-data"),
    # --- literals --------------------------------------------------------------
    ("literal-true-truncated", "tru", b"tru", "literal"),
    ("literal-null-truncated", "nul", b"nul", "literal"),
    ("literal-capital-true", "True is not JSON", b"True", "literal"),
    ("literal-capital-null", "NULL is not JSON", b"NULL", "literal"),
    ("literal-nan", "NaN is not JSON", b"NaN", "literal"),
    ("literal-infinity", "Infinity is not JSON", b"Infinity", "literal"),
    ("literal-neg-infinity", "-Infinity is not JSON", b"-Infinity", "literal"),
    ("literal-undefined", "undefined is not JSON", b"undefined", "literal"),
    # --- empty and malformed structure ------------------------------------------
    ("empty-input", "no bytes at all", b"", "empty"),
    ("whitespace-only", "only whitespace", b" \n\t", "empty"),
    ("open-object", "an unclosed object", b"{", "syntax"),
    ("open-array", "an unclosed array", b"[", "syntax"),
    ("key-only", "a key with no colon", b'{"a"', "syntax"),
    ("key-colon-only", "a key and colon with no value", b'{"a":', "syntax"),
    ("value-no-close", "a pair with no closing brace", b'{"a":1', "syntax"),
    ("array-no-close", "an element with no closing bracket", b"[1", "syntax"),
    ("trailing-comma-array", "[1,]", b"[1,]", "syntax"),
    ("trailing-comma-object", '{"a":1,}', b'{"a":1,}', "syntax"),
    ("leading-comma-array", "[,1]", b"[,1]", "syntax"),
    ("missing-comma-array", "[1 2]", b"[1 2]", "syntax"),
    ("missing-colon", '{"a" 1}', b'{"a" 1}', "syntax"),
    ("unquoted-key", "{a:1}", b"{a:1}", "syntax"),
    ("single-quoted-key", "{'a':1}", b"{'a':1}", "syntax"),
    ("number-key", "{1:2}", b"{1:2}", "syntax"),
    ("unterminated-string", 'an opening quote with no end', b'"abc', "syntax"),
    ("unterminated-string-in-array", 'an unterminated string inside an array', b'["a"', "syntax"),
    ("comment-block", "/* */ comments are not JSON", b"/*x*/1", "syntax"),
    ("comment-line", "// comments are not JSON", b"//x\n1", "syntax"),
    ("comment-in-array", "a comment between elements", b"[1,/*x*/2]", "syntax"),
    ("stray-close-bracket", "]", b"]", "syntax"),
    ("stray-close-brace", "}", b"}", "syntax"),
    ("stray-colon", ":", b":", "syntax"),
    ("stray-comma", ",", b",", "syntax"),
    ("whitespace-form-feed", "form feed is not JSON whitespace", b"\x0c1", "syntax"),
    ("whitespace-vertical-tab", "vertical tab is not JSON whitespace", b"\x0b1", "syntax"),
    ("whitespace-nbsp", "U+00A0 is not JSON whitespace", "\u00a01".encode("utf-8"), "syntax"),
    ("whitespace-line-separator", "U+2028 is not JSON whitespace", "\u20281".encode("utf-8"), "syntax"),
    # --- trailing data ------------------------------------------------------------
    ("trailing-letter", "garbage after an object", b"{} x", "trailing-data"),
    ("trailing-second-object", "two objects", b"{}{}", "trailing-data"),
    ("trailing-bracket", "an extra closing bracket", b"[1]]", "trailing-data"),
    ("trailing-second-number", "two numbers", b"1 2", "trailing-data"),
    ("trailing-second-null", "two nulls", b"null null", "trailing-data"),
    ("trailing-nul-byte", "a NUL byte after the document", b"{}\x00", "trailing-data"),
    ("trailing-after-string", "a letter after a string", b'"a"b', "trailing-data"),
    ("trailing-comma-top", "a comma after the document", b"1,", "trailing-data"),
    # --- byte order marks ---------------------------------------------------------------
    ("bom-before-object", "a UTF-8 BOM before the document", b"\xef\xbb\xbf{}", "bom"),
    ("bom-alone", "a BOM and nothing else", b"\xef\xbb\xbf", "bom"),
    ("bom-after-whitespace", "a BOM after leading whitespace", b" \xef\xbb\xbf{}", "bom"),
    ("bom-utf16-le", "a UTF-16 LE document is not UTF-8", b"\xff\xfe{\x00}\x00", "utf8"),
    ("bom-utf16-be", "a UTF-16 BE document is not UTF-8", b"\xfe\xff\x00{\x00}", "utf8"),
    ("bom-inside-string", "U+FEFF inside a string is just a character", b'"\xef\xbb\xbf"', None),
    # --- strings: escapes -----------------------------------------------------------------
    ("string-short-escapes", "every short escape, and the solidus", rb'"\"\\\/\b\f\n\r\t"', None),
    ("string-solidus-escape", r"\/ is read as / and written as /", rb'"a\/b"', None),
    ("string-quote-escape", 'an escaped quote stays escaped', rb'"a\"b"', None),
    ("string-bmp-escapes", r"\u escapes for BMP characters", rb'"\u00e9\u20ac"', None),
    ("string-bmp-raw-quoted", "raw UTF-8 inside quotes renders as escapes", b'"' + "\u00e9\u20ac".encode("utf-8") + b'"', None),
    ("string-astral-raw", "a raw astral character renders as a surrogate pair", b'"' + "\U0001f600".encode("utf-8") + b'"', None),
    ("string-astral-escaped-pair", "an escaped surrogate pair is one character", rb'"\ud83d\ude00"', None),
    ("string-astral-escaped-pair-upper", "uppercase hex in a pair, rendered lowercase", rb'"\uD83D\uDE00"', None),
    ("string-escape-upper-hex", r"\u00FF renders as \u00ff", rb'"\u00FF"', None),
    ("string-escape-nul", r"\u0000 is allowed as an escape", rb'"\u0000"', None),
    ("string-escape-unit-separator", r"\u001f is allowed as an escape", rb'"\u001f"', None),
    ("string-escape-del", r"\u007f renders as \u007f", rb'"\u007f"', None),
    ("string-raw-del", "a raw DEL byte is allowed in a string and renders as \\u007f", b'"\x7f"', None),
    ("string-raw-space-tilde", "raw space and tilde are the printable edges", b'" ~"', None),
    ("string-noncharacter-escape", r"\ufffe is a noncharacter but well-formed", rb'"\ufffe"', None),
    ("string-raw-ffff", "a raw U+FFFF", b'"\xef\xbf\xbf"', None),
    ("string-raw-10ffff", "a raw U+10FFFF", b'"\xf4\x8f\xbf\xbf"', None),
    ("string-raw-line-separator", "a raw U+2028 is data inside a string", b'"\xe2\x80\xa8"', None),
    ("string-utf8-boundaries", "the first and last code point of each UTF-8 length", b'"' + "".join(chr(c) for c in (0x80, 0x7FF, 0x800, 0xD7FF, 0xE000, 0xFFFF, 0x10000, 0x10FFFF)).encode("utf-8") + b'"', None),
    ("key-escapes", "keys are escaped and sorted by code point", b'{"\xc3\xa9":3,"\\n":1,"\\u0000":2}', None),
    # --- strings: control characters and bad escapes -----------------------------------------
    ("control-nul-raw", "a raw NUL in a string", b'"\x00"', "control-char"),
    ("control-soh-raw", "a raw U+0001 in a string", b'"\x01"', "control-char"),
    ("control-tab-raw", "a raw tab in a string", b'"\t"', "control-char"),
    ("control-lf-raw", "a raw LF in a string", b'"\n"', "control-char"),
    ("control-cr-raw", "a raw CR in a string", b'"\r"', "control-char"),
    ("control-us-raw", "a raw U+001F in a string", b'"\x1f"', "control-char"),
    ("control-tab-in-key", "a raw tab in a key", b'{"a\tb":1}', "control-char"),
    ("control-lf-in-key", "a raw LF in a key", b'{"a\nb":1}', "control-char"),
    ("escape-hex-x", r"\x41 is not a JSON escape", rb'"\x41"', "bad-escape"),
    ("escape-bell", r"\a is not a JSON escape", rb'"\a"', "bad-escape"),
    ("escape-single-quote", r"\' is not a JSON escape", b'"\\\'"', "bad-escape"),
    ("escape-capital-u", r"\U0041 is not a JSON escape", rb'"\U0041"', "bad-escape"),
    ("escape-short-hex", r"\u12 has two hex digits", rb'"\u12"', "bad-escape"),
    ("escape-bad-hex", r"\u12G4 has a non-hex digit", rb'"\u12G4"', "bad-escape"),
    ("escape-no-hex", r"\u with nothing after it", rb'"\u"', "bad-escape"),
    ("escape-space", r"backslash space", b'"\\ "', "bad-escape"),
    ("escape-zero", r"\0 is not a JSON escape", rb'"\0"', "bad-escape"),
    ("escape-backslash-at-end", "a lone backslash at the end of input", b'"\\', "bad-escape"),
    # --- strings: surrogate escapes ----------------------------------------------------------
    ("surrogate-high-alone", r"a lone high surrogate \ud800", rb'"\ud800"', "unpaired-surrogate"),
    ("surrogate-high-max-alone", r"a lone high surrogate \udbff", rb'"\udbff"', "unpaired-surrogate"),
    ("surrogate-low-alone", r"a lone low surrogate \udc00", rb'"\udc00"', "unpaired-surrogate"),
    ("surrogate-low-max-alone", r"a lone low surrogate \udfff", rb'"\udfff"', "unpaired-surrogate"),
    ("surrogate-high-upper-hex", r"a lone high surrogate \uD800 in upper-case hex", rb'"\uD800"', "unpaired-surrogate"),
    ("surrogate-high-then-bmp", "a high surrogate then an ordinary escape", rb'"\ud800\u0041"', "unpaired-surrogate"),
    ("surrogate-high-then-raw", "a high surrogate then a raw character", rb'"\ud800x"', "unpaired-surrogate"),
    ("surrogate-high-then-high", "two high surrogates", rb'"\ud800\ud800"', "unpaired-surrogate"),
    ("surrogate-low-then-high", "a pair in the wrong order", rb'"\udc00\ud800"', "unpaired-surrogate"),
    ("surrogate-pair-then-high", "a good pair then a lone high", rb'"\ud83d\ude00\ud83d"', "unpaired-surrogate"),
    ("surrogate-low-then-pair", "a lone low then a good pair", rb'"\ude00\ud83d\ude00"', "unpaired-surrogate"),
    ("surrogate-high-then-pair", "a lone high then a good pair", rb'"\ud800\ud83d\ude00"', "unpaired-surrogate"),
    ("surrogate-high-at-end", "a lone high surrogate after text", rb'"a\ud800"', "unpaired-surrogate"),
    ("surrogate-high-then-newline-escape", r"a high surrogate then \n", rb'"\ud800\n"', "unpaired-surrogate"),
    ("surrogate-in-key", "a lone surrogate in a KEY", rb'{"\ud800":1}', "unpaired-surrogate"),
    ("surrogate-in-nested-value", "a lone surrogate in a nested value", rb'{"a":["\udc00"]}', "unpaired-surrogate"),
    ("surrogate-pair-boundaries", "the lowest and highest valid pairs", rb'"\ud800\udc00\udbff\udfff"', None),
    # --- invalid UTF-8 -------------------------------------------------------------------------
    ("utf8-continuation-alone", "a stray continuation byte 0x80", b'"\x80"', "utf8"),
    ("utf8-continuation-bf", "a stray continuation byte 0xBF", b'"\xbf"', "utf8"),
    ("utf8-ff", "the byte 0xFF", b'"\xff"', "utf8"),
    ("utf8-fe", "the byte 0xFE", b'"\xfe"', "utf8"),
    ("utf8-overlong-nul", "C0 80: an overlong NUL", b'"\xc0\x80"', "utf8"),
    ("utf8-overlong-slash", "C0 AF: an overlong solidus", b'"\xc0\xaf"', "utf8"),
    ("utf8-overlong-c1", "C1 BF: an overlong U+007F", b'"\xc1\xbf"', "utf8"),
    ("utf8-overlong-3-byte", "E0 80 80: an overlong 3-byte NUL", b'"\xe0\x80\x80"', "utf8"),
    ("utf8-overlong-3-byte-max", "E0 9F BF: an overlong 3-byte U+07FF", b'"\xe0\x9f\xbf"', "utf8"),
    ("utf8-overlong-4-byte", "F0 80 80 80: an overlong 4-byte NUL", b'"\xf0\x80\x80\x80"', "utf8"),
    ("utf8-overlong-4-byte-max", "F0 8F BF BF: an overlong 4-byte U+FFFF", b'"\xf0\x8f\xbf\xbf"', "utf8"),
    ("utf8-encoded-surrogate-low", "ED A0 80: U+D800 encoded directly", b'"\xed\xa0\x80"', "utf8"),
    ("utf8-encoded-surrogate-high", "ED BF BF: U+DFFF encoded directly", b'"\xed\xbf\xbf"', "utf8"),
    ("utf8-cesu-pair", "a surrogate pair encoded as two 3-byte sequences (CESU-8)", b'"\xed\xa0\xbd\xed\xb8\x80"', "utf8"),
    ("utf8-above-max", "F4 90 80 80: U+110000", b'"\xf4\x90\x80\x80"', "utf8"),
    ("utf8-f5", "F5 80 80 80: a lead byte above F4", b'"\xf5\x80\x80\x80"', "utf8"),
    ("utf8-five-byte", "F8 88 80 80 80: a 5-byte form", b'"\xf8\x88\x80\x80\x80"', "utf8"),
    ("utf8-truncated-2-byte", "a 2-byte lead cut off by the closing quote", b'"\xc3"', "utf8"),
    ("utf8-truncated-3-byte", "a 3-byte sequence with one continuation", b'"\xe2\x82"', "utf8"),
    ("utf8-truncated-4-byte", "a 4-byte sequence with two continuations", b'"\xf0\x9f\x98"', "utf8"),
    ("utf8-truncated-at-end", "a 2-byte lead as the last byte of the input", b'"\xc3', "utf8"),
    ("utf8-in-key", "invalid UTF-8 in a key", b'{"\xff":1}', "utf8"),
    ("utf8-outside-string", "invalid UTF-8 where a token should be", b"\xff", "utf8"),
    ("utf8-in-array", "invalid UTF-8 between array elements", b"[\xff]", "utf8"),
    ("utf8-continuation-after-valid", "a valid character then a stray continuation", b'"\xc3\xa9\x80"', "utf8"),
    # --- duplicate keys -----------------------------------------------------------------------------
    ("dup-top-level", "the same key twice, different values", b'{"a":1,"a":2}', "duplicate-key"),
    ("dup-same-value", "the same key twice, same value", b'{"a":1,"a":1}', "duplicate-key"),
    ("dup-three-keys", "a duplicate that is not adjacent", b'{"a":1,"b":2,"a":3}', "duplicate-key"),
    ("dup-after-container", "a duplicate after a container value", b'{"a":{"x":1},"a":2}', "duplicate-key"),
    ("dup-nested-object", "a duplicate inside a nested object", b'{"o":{"k":1,"k":2}}', "duplicate-key"),
    ("dup-in-array", "a duplicate inside an object inside an array", b'[{"x":{"k":1,"k":1}}]', "duplicate-key"),
    ("dup-deep", "a duplicate four levels down", b'{"a":{"b":{"c":{"d":1,"d":2}}}}', "duplicate-key"),
    ("dup-escaped-equivalent", r'"a" and "\u0061" are the same key', b'{"a":1,"\\u0061":2}', "duplicate-key"),
    ("dup-escaped-capital", r'"\u0041" and "A" are the same key', b'{"\\u0041":1,"A":2}', "duplicate-key"),
    ("dup-empty-key", "the empty key twice", b'{"":1,"":2}', "duplicate-key"),
    ("dup-proto", "a duplicated __proto__ key", b'{"__proto__":1,"__proto__":2}', "duplicate-key"),
    ("dup-astral-equivalent", "a raw astral key and its escaped pair are the same key", b'{"' + "\U0001f600".encode("utf-8") + b'":1,"\\ud83d\\ude00":2}', "duplicate-key"),
    ("dup-absent-different-case", "keys differing only in case are different keys", b'{"a":1,"A":2}', None),
    ("dup-absent-different-objects", "the same key in two sibling objects is fine", b'[{"a":1},{"a":2}]', None),
    ("dup-absent-nested-same-name", "a nested key may reuse its parent's name", b'{"a":{"a":1}}', None),
    ("dup-absent-empty-key-once", "one empty key is fine", b'{"":1}', None),
    # --- depth ----------------------------------------------------------------------------------------------
    ("depth-256-arrays", "256 nested arrays are accepted", _nest_arrays(256), None),
    ("depth-257-arrays", "257 nested arrays are refused", _nest_arrays(257), "depth"),
    ("depth-256-objects", "256 nested objects are accepted", _nest_objects(256), None),
    ("depth-257-objects", "257 nested objects are refused", _nest_objects(257), "depth"),
    ("depth-256-arrays-scalar-leaf", "a scalar inside 256 arrays does not add a level", _nest_arrays(256, b"1"), None),
    ("depth-257-arrays-scalar-leaf", "a scalar inside 257 arrays", _nest_arrays(257, b"1"), "depth"),
    ("depth-256-alternating", "256 alternating arrays and objects", _alternating(256), None),
    ("depth-257-alternating", "257 alternating arrays and objects", _alternating(257), "depth"),
    # --- keys that are special in some runtimes ---------------------------------------------------------------
    ("key-proto", "__proto__ is an ordinary key", b'{"__proto__":1}', None),
    ("key-proto-object", "__proto__ holding an object must not reach a prototype", b'{"__proto__":{"polluted":true}}', None),
    ("key-proto-in-array", "__proto__ inside an array element", b'[{"__proto__":[]}]', None),
    ("key-constructor", "constructor is an ordinary key", b'{"constructor":1}', None),
    ("key-to-string", "toString is an ordinary key", b'{"toString":1}', None),
    ("key-has-own-property", "hasOwnProperty is an ordinary key", b'{"hasOwnProperty":1}', None),
    ("key-value-of", "valueOf is an ordinary key", b'{"valueOf":1}', None),
    ("key-prototype", "prototype is an ordinary key", b'{"prototype":1}', None),
    ("key-specials-sorted", "special keys sort like any other key", b'{"toString":1,"constructor":2,"__proto__":3,"a":4,"valueOf":5,"hasOwnProperty":6}', None),
    ("key-integer-like-sorted", "integer-like keys sort as text (10 before 2), not as numbers", b'{"10":1,"9":2,"2":3,"1":4,"0":5}', None),
    ("key-order-ascii", "uppercase sorts before underscore before lowercase", b'{"b":1,"a":2,"B":3,"_":4,"Z":5}', None),
    ("key-order-prefix", "a key sorts before its extensions", b'{"ab":1,"a":2,"b":3}', None),
    ("key-order-latin1", "U+00E9 sorts after z", '{"\u00e9":1,"z":2}'.encode("utf-8"), None),
    ("key-order-bmp-vs-astral", "U+FFFF sorts BEFORE U+1F600 (UTF-16 order would reverse them)", b'{"\\ud83d\\ude00":1,"\\uffff":2}', None),
    ("key-order-private-use-vs-astral", "U+E000 sorts before U+10000", b'{"\\ud800\\udc00":1,"\\ue000":2}', None),
]

# Tagged cases: (slug, summary, type, errors, input text, intent) where intent
# is None (accept), "value", or "malformed-json".
TaggedCase = tuple[str, str, str, "dict[str, list[str]] | None", str, "str | None"]


def _tag(type_text: str, value_json: str) -> str:
    return '{"type":"%s","value":%s}' % (type_text, value_json)


SELECT = {"SelectError": ["CT_SELECT_LENGTH_MISMATCH"]}
RES_B = "Result<Bytes,SelectError>"

TAGGED_CASES: list[TaggedCase] = [
    ("bool-true", "a Bool is a bare JSON boolean", "Bool", None, "true", None),
    ("bool-false", "false", "Bool", None, "false", None),
    ("bool-string", "the string \"true\" is not a Bool", "Bool", None, '"true"', "value"),
    ("bool-number", "1 is not a Bool", "Bool", None, "1", "value"),
    ("bool-tagged", "a Bool is not tagged", "Bool", None, _tag("Bool", "true"), "value"),
    ("u8-zero", "U8 lower bound", "U8", None, _tag("U8", '"0"'), None),
    ("u8-max", "U8 upper bound", "U8", None, _tag("U8", '"255"'), None),
    ("u8-overflow", "U8 above 255", "U8", None, _tag("U8", '"256"'), "value"),
    ("u8-negative", "U8 below 0", "U8", None, _tag("U8", '"-1"'), "value"),
    ("u64-zero", "U64 lower bound", "U64", None, _tag("U64", '"0"'), None),
    ("u64-one", "U64 one", "U64", None, _tag("U64", '"1"'), None),
    ("u64-2^53", "U64 2^53 is exact as text", "U64", None, _tag("U64", '"9007199254740992"'), None),
    ("u64-2^53+1", "U64 2^53 + 1 is exact as text (a double would round it)", "U64", None, _tag("U64", '"9007199254740993"'), None),
    ("u64-2^63", "U64 2^63", "U64", None, _tag("U64", '"9223372036854775808"'), None),
    ("u64-max", "U64 upper bound", "U64", None, _tag("U64", '"18446744073709551615"'), None),
    ("u64-overflow", "U64 above 2^64 - 1", "U64", None, _tag("U64", '"18446744073709551616"'), "value"),
    ("u64-huge", "U64 with 30 digits", "U64", None, _tag("U64", '"' + "9" * 30 + '"'), "value"),
    ("u64-leading-zero", "leading zero", "U64", None, _tag("U64", '"01"'), "value"),
    ("u64-double-zero", "00", "U64", None, _tag("U64", '"00"'), "value"),
    ("u64-plus", "+1", "U64", None, _tag("U64", '"+1"'), "value"),
    ("u64-minus-zero", "-0", "U64", None, _tag("U64", '"-0"'), "value"),
    ("u64-empty", "empty text", "U64", None, _tag("U64", '""'), "value"),
    ("u64-leading-space", "leading space", "U64", None, _tag("U64", '" 1"'), "value"),
    ("u64-trailing-space", "trailing space", "U64", None, _tag("U64", '"1 "'), "value"),
    ("u64-trailing-newline", "trailing newline", "U64", None, _tag("U64", '"1\\n"'), "value"),
    ("u64-hex", "0x10", "U64", None, _tag("U64", '"0x10"'), "value"),
    ("u64-exponent", "1e3", "U64", None, _tag("U64", '"1e3"'), "value"),
    ("u64-fraction", "1.0", "U64", None, _tag("U64", '"1.0"'), "value"),
    ("u64-arabic-digits", "non-ASCII decimal digits", "U64", None, _tag("U64", '"\\u0661\\u0662"'), "value"),
    ("u64-small-number", "a JSON number where text is required", "U64", None, _tag("U64", "5"), "value"),
    ("u64-null", "null where text is required", "U64", None, _tag("U64", "null"), "value"),
    ("u64-array", "an array where text is required", "U64", None, _tag("U64", '["1"]'), "value"),
    ("u64-bare-string", "an untagged string", "U64", None, '"1"', "value"),
    ("u64-wrong-tag", "a U8 tag on a U64", "U64", None, _tag("U8", '"1"'), "value"),
    ("u64-missing-type", "no type key", "U64", None, '{"value":"1"}', "value"),
    ("u64-extra-key", "an extra key", "U64", None, '{"type":"U64","value":"1","x":1}', "value"),
    ("u64-key-order", "key order does not matter", "U64", None, '{"value":"7","type":"U64"}', None),
    ("u64-json-number-above-2^53", "a U64 as a JSON number above 2^53 never reaches the decoder", "U64", None, _tag("U64", "9007199254740993"), "malformed-json"),
    ("u64-json-exponent", "a U64 as 1e3", "U64", None, _tag("U64", "1e3"), "malformed-json"),
    ("u64-duplicate-key", "a duplicate key in the tagged object", "U64", None, '{"type":"U64","value":"1","value":"2"}', "malformed-json"),
    ("u64-lone-surrogate-text", "a lone surrogate escape in the text", "U64", None, _tag("U64", '"\\ud800"'), "malformed-json"),
    ("nat-zero", "Nat lower bound", "Nat", None, _tag("Nat", '"0"'), None),
    ("nat-max", "Nat upper bound (the checked width is 64 bits)", "Nat", None, _tag("Nat", '"18446744073709551615"'), None),
    ("nat-overflow", "Nat above the checked width", "Nat", None, _tag("Nat", '"18446744073709551616"'), "value"),
    ("bytes-empty", "the empty byte string", "Bytes", None, _tag("Bytes", '""'), None),
    ("bytes-ascii", "bytes 00 and ff", "Bytes", None, _tag("Bytes", '"00ff"'), None),
    ("bytes-uppercase", "uppercase hex is refused", "Bytes", None, _tag("Bytes", '"FF"'), "value"),
    ("bytes-mixed-case", "mixed-case hex is refused", "Bytes", None, _tag("Bytes", '"aB"'), "value"),
    ("bytes-odd-length", "an odd number of hex digits", "Bytes", None, _tag("Bytes", '"abc"'), "value"),
    ("bytes-non-hex", "a non-hex digit", "Bytes", None, _tag("Bytes", '"zz"'), "value"),
    ("bytes-space", "a space inside the hex", "Bytes", None, _tag("Bytes", '"00 ff"'), "value"),
    ("bytes-prefix", "a 0x prefix", "Bytes", None, _tag("Bytes", '"0x00"'), "value"),
    ("bytes-number", "a number instead of text", "Bytes", None, _tag("Bytes", "0"), "value"),
    ("bytes-fixed-ok", "Bytes<2> with two bytes", "Bytes<2>", None, _tag("Bytes<2>", '"0102"'), None),
    ("bytes-fixed-short", "Bytes<2> with one byte", "Bytes<2>", None, _tag("Bytes<2>", '"01"'), "value"),
    ("bytes-fixed-long", "Bytes<2> with three bytes", "Bytes<2>", None, _tag("Bytes<2>", '"010203"'), "value"),
    ("bytes-fixed-zero", "Bytes<0> with no bytes", "Bytes<0>", None, _tag("Bytes<0>", '""'), None),
    ("bytes-fixed-zero-nonempty", "Bytes<0> with a byte", "Bytes<0>", None, _tag("Bytes<0>", '"00"'), "value"),
    ("result-ok-bytes", "Ok holding Bytes", RES_B, SELECT, _tag(RES_B, '{"ok":' + _tag("Bytes", '"0aff"') + "}"), None),
    ("result-ok-bytes-empty", "Ok holding the empty Bytes", RES_B, SELECT, _tag(RES_B, '{"ok":' + _tag("Bytes", '""') + "}"), None),
    ("result-err-declared", "Err with a declared variant", RES_B, SELECT, _tag(RES_B, '{"err":"CT_SELECT_LENGTH_MISMATCH"}'), None),
    ("result-err-undeclared", "Err with a variant the module did not declare", RES_B, SELECT, _tag(RES_B, '{"err":"OTHER"}'), "value"),
    ("result-err-no-errors-known", "Err when no error type is declared", RES_B, None, _tag(RES_B, '{"err":"CT_SELECT_LENGTH_MISMATCH"}'), "value"),
    ("result-err-number", "Err with a non-text variant", RES_B, SELECT, _tag(RES_B, '{"err":1}'), "value"),
    ("result-both-keys", "ok and err together", RES_B, SELECT, _tag(RES_B, '{"ok":' + _tag("Bytes", '""') + ',"err":"CT_SELECT_LENGTH_MISMATCH"}'), "value"),
    ("result-empty-body", "neither ok nor err", RES_B, SELECT, _tag(RES_B, "{}"), "value"),
    ("result-wrong-tag", "the wrong outer type tag", RES_B, SELECT, _tag("Result<Bool,SelectError>", '{"ok":true}'), "value"),
    ("result-body-not-object", "a body that is not an object", RES_B, SELECT, _tag(RES_B, '"ok"'), "value"),
    ("result-ok-bool", "Ok holding a bare Bool", "Result<Bool,SelectError>", SELECT, _tag("Result<Bool,SelectError>", '{"ok":true}'), None),
    ("result-ok-wrong-inner", "Ok holding the wrong inner shape", RES_B, SELECT, _tag(RES_B, '{"ok":true}'), "value"),
]

# Containment cases: (slug, summary, flavor, root, child, inside, same).
ContainmentCase = tuple[str, str, str, str, str, bool, bool]
P, W = "posix", "win32"
CONTAINMENT_CASES: list[ContainmentCase] = [
    ("posix-child", "a nested child", P, "/repo", "/repo/a/b", True, False),
    ("posix-root-itself", "the root is inside itself", P, "/repo", "/repo", True, True),
    ("posix-child-trailing-slash", "the root with a trailing slash", P, "/repo", "/repo/", True, True),
    ("posix-root-trailing-slash", "a root written with a trailing slash", P, "/repo/", "/repo/x", True, False),
    ("posix-sibling-prefix", "/repo2 shares a string prefix but is not inside /repo", P, "/repo", "/repo2", False, False),
    ("posix-sibling-prefix-deep", "/repo-evil/x shares a string prefix", P, "/repo", "/repo-evil/x", False, False),
    ("posix-parent", "the parent directory", P, "/repo/sub", "/repo", False, False),
    ("posix-dotdot-child", "/repo/.. is the parent of the root", P, "/repo", "/repo/..", False, False),
    ("posix-dotdot-escape", "/repo/../other escapes", P, "/repo", "/repo/../other", False, False),
    ("posix-dotdot-return", "/repo/../repo/x comes back in", P, "/repo", "/repo/../repo/x", True, False),
    ("posix-dotdot-deep-escape", "a deep path that climbs out", P, "/repo", "/repo/a/../../etc", False, False),
    ("posix-dotdot-prefix-name", "..foo is an ordinary directory name", P, "/repo", "/repo/..foo", True, False),
    ("posix-dotdot-prefix-name-nested", "..foo/bar is inside", P, "/repo", "/repo/..foo/bar", True, False),
    ("posix-three-dots-name", "... is an ordinary directory name", P, "/repo", "/repo/...", True, False),
    ("posix-dot-and-double-slash", ". and // components are dropped", P, "/repo", "/repo/./a//b", True, False),
    ("posix-other-tree", "an unrelated tree", P, "/repo", "/other", False, False),
    ("posix-filesystem-root-child", "the filesystem root is not inside /repo", P, "/repo", "/", False, False),
    ("posix-root-is-slash", "everything is inside /", P, "/", "/anything/at/all", True, False),
    ("posix-root-is-slash-same", "/ is inside /", P, "/", "/", True, True),
    ("posix-case-sensitive", "posix compares case-sensitively", P, "/repo", "/Repo/x", False, False),
    ("posix-backslash-is-a-name", "a backslash is an ordinary character on posix", P, "/repo", "/repo/a\\..\\b", True, False),
    ("posix-dotdot-at-root-clamps", "/.. is / (there is no parent of the root)", P, "/repo", "/../repo/x", True, False),
    ("posix-root-normalised", "the root is normalised too", P, "/a/b/../c", "/a/c/d", True, False),
    ("win32-child", "a nested child", W, "D:\\repo", "D:\\repo\\x", True, False),
    ("win32-root-itself", "the root is inside itself", W, "D:\\repo", "D:\\repo", True, True),
    ("win32-cross-drive", "C: is another drive: path.relative would return an ABSOLUTE path", W, "D:\\repo", "C:\\tmp\\x", False, False),
    ("win32-cross-drive-same-path", "the same path on another drive", W, "D:\\repo", "C:\\repo\\x", False, False),
    ("win32-cross-drive-root-child", "another drive's root", W, "D:\\repo", "C:\\", False, False),
    ("win32-sibling-prefix", "D:\\repo2 is not inside D:\\repo", W, "D:\\repo", "D:\\repo2", False, False),
    ("win32-dotdot-child", "D:\\repo\\.. is the parent", W, "D:\\repo", "D:\\repo\\..", False, False),
    ("win32-dotdot-escape", "D:\\repo\\..\\other escapes", W, "D:\\repo", "D:\\repo\\..\\other", False, False),
    ("win32-dotdot-return", "a path that leaves and re-enters", W, "D:\\repo", "D:\\repo\\..\\REPO\\x", True, False),
    ("win32-dotdot-prefix-name", "..foo is an ordinary directory name", W, "D:\\repo", "D:\\repo\\..foo", True, False),
    ("win32-dotdot-at-root-clamps", "D:\\.. is D:\\", W, "D:\\repo", "D:\\..\\repo\\x", True, False),
    ("win32-dotdot-climbs-out-of-two", "two levels up", W, "D:\\repo", "D:\\repo\\a\\..\\..", False, False),
    ("win32-case-insensitive", "drive letter and names compare case-insensitively", W, "D:\\repo", "d:\\REPO\\x", True, False),
    ("win32-forward-slashes", "/ is a separator too", W, "D:\\repo", "D:/repo/x", True, False),
    ("win32-root-trailing-backslash", "a root written with a trailing backslash", W, "D:\\repo\\", "D:\\repo\\x", True, False),
    ("win32-drive-root", "everything on D: is inside D:\\", W, "D:\\", "D:\\x", True, False),
    ("win32-drive-root-same", "D:\\ is inside D:\\", W, "D:\\", "D:\\", True, True),
    ("win32-drive-root-other-drive", "E:\\x is not inside D:\\", W, "D:\\", "E:\\x", False, False),
    ("win32-unc-child", "a child of a UNC root", W, "\\\\server\\share\\repo", "\\\\server\\share\\repo\\x", True, False),
    ("win32-unc-root-itself", "a UNC root is inside itself", W, "\\\\server\\share\\repo", "\\\\server\\share\\repo", True, True),
    ("win32-unc-other-share", "another share on the same server", W, "\\\\server\\share\\repo", "\\\\server\\other\\repo\\x", False, False),
    ("win32-unc-other-server", "another server", W, "\\\\server\\share\\repo", "\\\\server2\\share\\repo\\x", False, False),
    ("win32-unc-case-insensitive", "server, share and names compare case-insensitively", W, "\\\\server\\share\\repo", "\\\\SERVER\\Share\\Repo\\x", True, False),
    ("win32-unc-share-root", "a child of the share root", W, "\\\\server\\share", "\\\\server\\share\\a", True, False),
    ("win32-unc-sibling-prefix", "repo2 is not inside repo on a share", W, "\\\\server\\share\\repo", "\\\\server\\share\\repo2", False, False),
    ("win32-unc-dotdot-clamps-to-share", ".. cannot climb above the share", W, "\\\\server\\share\\repo", "\\\\server\\share\\..\\..\\x", False, False),
    ("win32-drive-root-unc-child", "a UNC path is never inside a drive root", W, "D:\\repo", "\\\\server\\share\\repo", False, False),
    ("win32-unc-root-drive-child", "a drive path is never inside a UNC root", W, "\\\\server\\share\\repo", "D:\\repo\\x", False, False),
    ("win32-device-namespace-question", "a \\\\?\\ device path is never inside an ordinary root", W, "D:\\repo", "\\\\?\\D:\\repo\\x", False, False),
    ("win32-device-namespace-dot", "a \\\\.\\ device path is never inside an ordinary root", W, "D:\\repo", "\\\\.\\D:\\repo", False, False),
]

# Label-digest cases: (slug, summary, manifest, same_as, differs_from).
BASE_MANIFEST: dict[str, Any] = {
    "schema": "warrant-lane-evidence-v1",
    "package": "python/ct-compare",
    "module": "ct_compare",
    "toolchain": {"name": "python", "version": "3.13.1"},
    "runner": {"src_sha256": "0" * 64},
    "tests": {"examples": 12, "exhaustive": 9},
}


def _variant(**changes: Any) -> dict[str, Any]:
    manifest = {k: (dict(v) if isinstance(v, dict) else v) for k, v in BASE_MANIFEST.items()}
    for key, value in changes.items():
        manifest[key] = value
    return manifest


LabelCase = tuple[str, str, dict[str, Any], "str | None", "str | None"]
LABEL_CASES: list[LabelCase] = [
    ("python-base", "the reference manifest", _variant(), None, None),
    ("python-other-version", "toolchain.version does not change the digest", _variant(toolchain={"name": "python", "version": "3.12.0"}), "python-base", None),
    ("python-empty-version", "an empty toolchain.version is the same digest", _variant(toolchain={"name": "python", "version": ""}), "python-base", None),
    ("node-name", "toolchain.name IS part of the digest (never a literal)", _variant(toolchain={"name": "node", "version": "22.1.0"}), None, "python-base"),
    ("node-name-other-version", "same name, another version, same digest", _variant(toolchain={"name": "node", "version": "20.0.0"}), "node-name", None),
    ("name-case", "toolchain.name compares exactly", _variant(toolchain={"name": "Python", "version": "3.13.1"}), None, "python-base"),
    ("name-empty", "an empty toolchain.name still counts", _variant(toolchain={"name": "", "version": "3.13.1"}), None, "python-base"),
    ("key-order", "key order in the input is irrelevant", {"tests": {"exhaustive": 9, "examples": 12}, "runner": {"src_sha256": "0" * 64}, "toolchain": {"version": "9", "name": "python"}, "module": "ct_compare", "package": "python/ct-compare", "schema": "warrant-lane-evidence-v1"}, "python-base", None),
    ("package-changes", "another package is another digest", _variant(package="typescript/ct-compare"), None, "python-base"),
    ("runner-digest-changes", "another runner digest is another digest", _variant(runner={"src_sha256": "1" * 64}), None, "python-base"),
    ("non-ascii", "non-ASCII text is rendered as lowercase \\u escapes before hashing", _variant(package="python/caf\u00e9-\U0001f600"), None, "python-base"),
    ("special-keys", "__proto__ and integer-like keys survive into the digest", _variant(extra={"__proto__": {"x": 1}, "10": 1, "9": 2}), None, "python-base"),
    ("extra-field", "an extra field changes the digest", _variant(extra=1), None, "python-base"),
]


# ---------------------------------------------------------------------------
# 7. Building, writing and checking the fixture
# ---------------------------------------------------------------------------
def _expect_json(data: bytes, intent: str | None, slug: str) -> dict[str, Any]:
    """Run the reference reader and writer; insist it matches the intent."""
    try:
        value = read_json(data)
    except Reject as error:
        if intent != error.reason:
            raise SystemExit(f"json/{slug}: intended {intent!r}, reference says reject {error.reason!r}")
        return {"outcome": "reject", "error": "malformed-json", "reason": error.reason}
    if intent is not None:
        raise SystemExit(f"json/{slug}: intended reject {intent!r}, reference accepts")
    text = canonical(value)
    expect: dict[str, Any] = {"outcome": "accept", "canonical_sha256": sha256_hex(text.encode("utf-8"))}
    if len(text.encode("utf-8")) <= CANONICAL_TEXT_LIMIT:
        expect["canonical"] = text
    return expect


def build_cases() -> dict[str, list[dict[str, Any]]]:
    """Every category's case list, derived through the reference implementation."""
    json_cases = [
        {"id": f"json/{slug}", "summary": summary, "input_hex": data.hex(), "expect": _expect_json(data, intent, slug)}
        for slug, summary, data, intent in JSON_CASES
    ]

    tagged_cases = []
    for slug, summary, type_text, errors, text, intent in TAGGED_CASES:
        data = text.encode("utf-8")
        try:
            raw = read_json(data)
        except Reject:
            outcome: dict[str, Any] = {"outcome": "reject", "error": "malformed-json"}
            got = "malformed-json"
        else:
            try:
                stream = decode_and_encode(raw, type_text, errors or {})
            except ValueReject:
                outcome = {"outcome": "reject", "error": "value"}
                got = "value"
            else:
                outcome = {"outcome": "accept", "stream_hex": stream.hex(), "stream_sha256": sha256_hex(stream)}
                got = None
        if got != intent:
            raise SystemExit(f"tagged/{slug}: intended {intent!r}, reference says {got!r}")
        case: dict[str, Any] = {"id": f"tagged/{slug}", "summary": summary, "type": type_text}
        if errors is not None:
            case["errors"] = errors
        case["input_hex"] = data.hex()
        case["expect"] = outcome
        tagged_cases.append(case)

    containment_cases = []
    for slug, summary, flavor, root, child, inside, same in CONTAINMENT_CASES:
        got_verdict = contains(flavor, root, child)
        if got_verdict != {"inside": inside, "same": same}:
            raise SystemExit(f"containment/{slug}: intended {(inside, same)}, reference says {got_verdict}")
        containment_cases.append(
            {"id": f"containment/{slug}", "summary": summary, "flavor": flavor, "root": root, "child": child, "expect": got_verdict}
        )

    digests: dict[str, str] = {}
    label_cases = []
    for slug, summary, manifest, same_as, differs_from in LABEL_CASES:
        digest = label_digest(manifest)
        digests[slug] = digest
        case = {"id": f"label-digest/{slug}", "summary": summary, "manifest": manifest, "sha256": digest}
        if same_as is not None:
            if digests[same_as] != digest:
                raise SystemExit(f"label-digest/{slug}: expected the digest of {same_as}")
            case["same_as"] = f"label-digest/{same_as}"
        if differs_from is not None:
            if digests[differs_from] == digest:
                raise SystemExit(f"label-digest/{slug}: expected a digest different from {differs_from}")
            case["differs_from"] = f"label-digest/{differs_from}"
        label_cases.append(case)

    return {
        "json": json_cases,
        "tagged": tagged_cases,
        "containment": containment_cases,
        "label-digest": label_cases,
    }


def build_files() -> dict[str, str]:
    """The exact text of every fixture file, keyed by file name."""
    files: dict[str, str] = {}
    index_rows = []
    for category, cases in build_cases().items():
        text = canonical({"schema": SCHEMA, "category": category, "cases": cases})
        name = f"{category}.json"
        files[name] = text
        index_rows.append(
            {"category": category, "path": name, "cases": len(cases), "sha256": sha256_hex(text.encode("utf-8"))}
        )
    files["index.json"] = canonical(
        {"schema": SCHEMA, "total_cases": sum(r["cases"] for r in index_rows), "files": index_rows}
    )
    files[".gitattributes"] = (
        "# Every byte of this fixture is part of its contract: never convert line endings.\n"
        "* -text\n"
    )
    return files


def structural_problems(files: dict[str, str]) -> list[str]:
    """Checks that do not depend on the case tables (ids, reasons, ASCII ...)."""
    problems: list[str] = []
    seen: set[str] = set()
    for name, text in files.items():
        if name == ".gitattributes":
            continue
        if "\r" in text or not text.endswith("\n") or not text.isascii():
            problems.append(f"{name}: must be LF-only ASCII ending in a newline")
        document = read_json(text.encode("utf-8"))
        if name == "index.json":
            continue
        for case in document["cases"]:
            if case["id"] in seen:
                problems.append(f"duplicate case id {case['id']}")
            seen.add(case["id"])
            if not case["id"].startswith(document["category"] + "/"):
                problems.append(f"{case['id']}: id must start with its category")
            reason = case.get("expect", {}).get("reason")
            if reason is not None and reason not in REASONS:
                problems.append(f"{case['id']}: reason {reason!r} is not in the closed set")
    return problems


def second_opinions() -> list[str]:
    """Cross-check the reference against the stdlib where the two must agree.

    * the UTF-8 decoder against ``bytes.decode`` over every table input;
    * the containment reference against ``PurePosixPath`` / ``PureWindowsPath``
      after ``normpath`` (except device paths, which the stdlib treats
      differently by design, and ``..`` at a UNC share root).
    """
    problems: list[str] = []
    for slug, _summary, data, _intent in JSON_CASES:
        try:
            reference: str | None = "".join(map(chr, decode_utf8(data)))
        except Reject:
            reference = None
        try:
            stdlib: str | None = data.decode("utf-8")
        except UnicodeDecodeError:
            stdlib = None
        if reference != stdlib:
            problems.append(f"json/{slug}: UTF-8 decoder disagrees with the stdlib")
    for slug, _s, flavor, root, child, inside, same in CONTAINMENT_CASES:
        if "\\\\?\\" in child or "\\\\.\\" in child or (flavor == "win32" and child.startswith("\\\\")):
            continue
        if flavor == "posix":
            pure, norm = PurePosixPath, posixpath.normpath
        else:
            pure, norm = PureWindowsPath, ntpath.normpath
        base, target = pure(norm(root)), pure(norm(child))
        stdlib_inside = target == base or base in target.parents
        if stdlib_inside != inside or (target == base) != same:
            problems.append(f"containment/{slug}: pathlib disagrees with the reference")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--write", action="store_true", help="regenerate the fixture files")
    args = parser.parse_args(argv)

    files = build_files()
    problems = structural_problems(files) + second_opinions()
    if args.write:
        args.fixture.mkdir(parents=True, exist_ok=True)
        for name, text in files.items():
            (args.fixture / name).write_bytes(text.encode("utf-8"))
        print(f"wrote {len(files)} files to {args.fixture}")
    else:
        for name, text in files.items():
            path = args.fixture / name
            if not path.is_file():
                problems.append(f"{name}: missing")
            elif path.read_bytes() != text.encode("utf-8"):
                problems.append(f"{name}: differs from the reference derivation (run with --write)")
        present = {p.name for p in args.fixture.iterdir()} if args.fixture.is_dir() else set()
        for extra in sorted(present - set(files) - {"README.md"}):
            problems.append(f"{extra}: not part of the fixture")
    for problem in problems:
        print(f"not ok {problem}")
    if problems:
        return 1
    total = sum(len(c) for c in build_cases().values())
    print(f"ok warrant-corpus-conformance-v1: {total} cases agree with the reference implementation")
    return 0


if __name__ == "__main__":
    sys.exit(main())
