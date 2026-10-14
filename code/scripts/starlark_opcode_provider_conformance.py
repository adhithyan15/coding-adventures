"""Validate STP01 compatibility data without importing or running a provider.

The literal oracle below is independently authored from STP01's normative
table. Neither corpus contents nor native output supplies an expected answer.
This checks a closed data contract; it does not qualify a compiler or VM.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = ROOT / "code/specs/fixtures/starlark-opcode-provider-v1/cases.json"
PROFILE = "standalone-provider-v1"
MAX_BYTES = 131072
MAX_DEPTH = 8

# Explicit bytes make accidental range extension or a missing member visible.
# The fixture was authored separately; do not replace this with fixture loading,
# enum introspection, a generated native snapshot, or a round-trip-only check.
_OPCODES = (
    (0x01, "LoadConst"),
    (0x02, "Pop"),
    (0x03, "Dup"),
    (0x04, "LoadNone"),
    (0x05, "LoadTrue"),
    (0x06, "LoadFalse"),
    (0x10, "StoreName"),
    (0x11, "LoadName"),
    (0x12, "StoreLocal"),
    (0x13, "LoadLocal"),
    (0x14, "StoreClosure"),
    (0x15, "LoadClosure"),
    (0x20, "Add"),
    (0x21, "Sub"),
    (0x22, "Mul"),
    (0x23, "Div"),
    (0x24, "FloorDiv"),
    (0x25, "Mod"),
    (0x26, "Power"),
    (0x27, "Negate"),
    (0x28, "BitAnd"),
    (0x29, "BitOr"),
    (0x2A, "BitXor"),
    (0x2B, "BitNot"),
    (0x2C, "LShift"),
    (0x2D, "RShift"),
    (0x30, "CmpEq"),
    (0x31, "CmpNe"),
    (0x32, "CmpLt"),
    (0x33, "CmpGt"),
    (0x34, "CmpLe"),
    (0x35, "CmpGe"),
    (0x36, "CmpIn"),
    (0x37, "CmpNotIn"),
    (0x38, "Not"),
    (0x40, "Jump"),
    (0x41, "JumpIfFalse"),
    (0x42, "JumpIfTrue"),
    (0x43, "JumpIfFalseOrPop"),
    (0x44, "JumpIfTrueOrPop"),
    (0x50, "MakeFunction"),
    (0x51, "CallFunction"),
    (0x52, "CallFunctionKw"),
    (0x53, "Return"),
    (0x60, "BuildList"),
    (0x61, "BuildDict"),
    (0x62, "BuildTuple"),
    (0x63, "ListAppend"),
    (0x64, "DictSet"),
    (0x70, "LoadSubscript"),
    (0x71, "StoreSubscript"),
    (0x72, "LoadAttr"),
    (0x73, "StoreAttr"),
    (0x74, "LoadSlice"),
    (0x80, "GetIter"),
    (0x81, "ForIter"),
    (0x82, "UnpackSequence"),
    (0x90, "LoadModule"),
    (0x91, "ImportFrom"),
    (0xA0, "Print"),
    (0xFF, "Halt"),
)
_CATEGORIES = {
    0x0: "stack",
    0x1: "variable",
    0x2: "arithmetic",
    0x3: "comparison",
    0x4: "control_flow",
    0x5: "function",
    0x6: "collection",
    0x7: "subscript_attribute",
    0x8: "iteration",
    0x9: "module",
    0xA: "io",
    0xF: "vm_control",
}
_OPERATORS = {
    "binary": (
        ("+", "Add"),
        ("-", "Sub"),
        ("*", "Mul"),
        ("/", "Div"),
        ("//", "FloorDiv"),
        ("%", "Mod"),
        ("**", "Power"),
        ("&", "BitAnd"),
        ("|", "BitOr"),
        ("^", "BitXor"),
        ("<<", "LShift"),
        (">>", "RShift"),
    ),
    "compare": (
        ("==", "CmpEq"),
        ("!=", "CmpNe"),
        ("<", "CmpLt"),
        (">", "CmpGt"),
        ("<=", "CmpLe"),
        (">=", "CmpGe"),
        ("in", "CmpIn"),
        ("not in", "CmpNotIn"),
    ),
    "augmented_assign": (
        ("+=", "Add"),
        ("-=", "Sub"),
        ("*=", "Mul"),
        ("/=", "Div"),
        ("//=", "FloorDiv"),
        ("%=", "Mod"),
        ("**=", "Power"),
        ("&=", "BitAnd"),
        ("|=", "BitOr"),
        ("^=", "BitXor"),
        ("<<=", "LShift"),
        (">>=", "RShift"),
    ),
    "unary": (("-", "Negate"), ("~", "BitNot")),
}
_UNKNOWN = (
    ("binary", "@"),
    ("binary", "++"),
    ("binary", "\uff0b"),
    ("compare", "IN"),
    ("compare", "not  in"),
    ("augmented_assign", ":="),
    ("unary", "+"),
    ("unary", "not"),
)


def _scan_depth(text: str) -> None:
    """Bound containers before JSON's recursive parser sees them.

    Brackets inside strings, including escaped quotes and backslashes, are
    data. Syntax validation itself belongs to the JSON decoder.
    """
    depth = 0
    quoted = False
    escaped = False
    for char in text:
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char in "{[":
            depth += 1
            if depth > MAX_DEPTH:
                raise ValueError("container depth exceeds eight")
        elif char in "}]":
            depth -= 1


def _guard_tree(value: Any) -> None:
    """Also protect callers passing decoded objects directly to validation."""
    pending = [(value, 1)]
    while pending:
        item, depth = pending.pop()
        if type(item) in (dict, list):
            if depth > MAX_DEPTH:
                raise ValueError("container depth exceeds eight")
            children = (
                list(item.keys()) + list(item.values()) if type(item) is dict else item
            )
            pending.extend((child, depth + 1) for child in children)
        elif isinstance(item, str):
            if any(0xD800 <= ord(char) <= 0xDFFF for char in item):
                raise ValueError("strings must contain Unicode scalar values")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object key")
        result[key] = value
    return result


def _integer(text: str) -> int:
    # Valid corpus integers are tiny. Bound conversion before calling int(),
    # independently of Python's optional process-wide integer digit limit.
    if len(text.lstrip("-")) > 10:
        raise ValueError("integer literal exceeds the corpus domain")
    return int(text)


def _reject_number(_text: str) -> NoReturn:
    raise ValueError(
        "fractional, exponent and nonfinite numbers are outside the corpus"
    )


def parse_json(raw: bytes) -> Any:
    """Decode one bounded, unambiguous UTF-8 document; no provider is invoked."""
    if type(raw) is not bytes or len(raw) > MAX_BYTES:
        raise ValueError("corpus must be at most 131072 UTF-8 bytes")
    text = raw.decode("utf-8")
    _scan_depth(text)
    value = json.loads(
        text,
        object_pairs_hook=_unique_object,
        parse_int=_integer,
        parse_float=_reject_number,
        parse_constant=_reject_number,
    )
    _guard_tree(value)
    return value


def load_corpus(path: Path) -> Any:
    # Reading limit+1 detects oversized input without an unbounded read.
    with path.open("rb") as stream:
        return parse_json(stream.read(MAX_BYTES + 1))


def _fields(value: Any, required: set[str], where: str) -> dict[str, Any]:
    if type(value) is not dict or set(value) != required:
        raise ValueError(f"{where}: fields must be exactly {sorted(required)}")
    return value


def _rows(value: Any, count: int, where: str) -> list[Any]:
    if type(value) is not list or len(value) != count:
        raise ValueError(f"{where}: expected exactly {count} ordered records")
    return value


def _text(value: Any, limit: int = 64) -> str:
    if type(value) is not str or not 1 <= len(value) <= limit:
        raise ValueError("label or operator has the wrong type or length")
    return value


def _byte(value: Any) -> int:
    if type(value) is not int or not 0 <= value <= 255:
        raise ValueError("byte must be a JSON integer in 0..255, excluding booleans")
    return value


def validate_document(document: Any) -> int:
    """Check every record against literal expectations, not mutual agreement."""
    _guard_tree(document)
    root = _fields(
        document,
        {
            "schema_version",
            "profile",
            "opcodes",
            "decode",
            "operators",
            "unknown_lookups",
        },
        "corpus",
    )
    if type(root["schema_version"]) is not int or root["schema_version"] != 1:
        raise ValueError("schema_version must be integer 1")
    if _text(root["profile"]) != PROFILE:
        raise ValueError("unsupported provider profile")

    assigned = dict(_OPCODES)
    for row, (byte, symbol) in zip(_rows(root["opcodes"], 61, "opcodes"), _OPCODES):
        row = _fields(row, {"byte", "symbol", "category"}, "opcode")
        actual = (_byte(row["byte"]), _text(row["symbol"]), _text(row["category"]))
        if actual != (byte, symbol, _CATEGORIES[byte >> 4]):
            raise ValueError("opcode disagrees with the STP01 literal oracle")

    for byte, row in enumerate(_rows(root["decode"], 256, "decode")):
        row = _fields(row, {"byte", "symbol", "category"}, "decode")
        symbol = _text(row["symbol"]) if row["symbol"] is not None else None
        category = _text(row["category"]) if row["category"] is not None else None
        # Membership comes first: a hole gets no category despite its nibble.
        expected = (
            (byte, assigned[byte], _CATEGORIES[byte >> 4])
            if byte in assigned
            else (byte, None, None)
        )
        if (_byte(row["byte"]), symbol, category) != expected:
            raise ValueError("decode disagrees with the STP01 literal oracle")

    groups = _fields(root["operators"], set(_OPERATORS), "operators")
    for group, expected in _OPERATORS.items():
        for row, pair in zip(_rows(groups[group], len(expected), group), expected):
            row = _fields(row, {"operator", "symbol"}, "operator")
            if (_text(row["operator"], 32), _text(row["symbol"])) != pair:
                raise ValueError("operator disagrees with the STP01 literal oracle")

    for row, pair in zip(
        _rows(root["unknown_lookups"], 8, "unknown_lookups"), _UNKNOWN
    ):
        row = _fields(row, {"group", "operator", "symbol"}, "unknown lookup")
        if (_text(row["group"]), _text(row["operator"], 32)) != pair or row[
            "symbol"
        ] is not None:
            raise ValueError("unknown lookup disagrees with the STP01 literal oracle")
    return 359


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_CORPUS)
    args = parser.parse_args(argv)
    try:
        count = validate_document(load_corpus(args.fixture))
    except (OSError, ValueError) as error:
        import sys

        print(f"STP01 invalid corpus: {error}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "profile": PROFILE,
                "records": count,
                "assigned": 61,
                "unassigned": 195,
                "operator_entries": 34,
                "native_adopters_checked": 0,
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
