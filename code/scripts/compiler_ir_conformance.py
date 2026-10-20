"""Validate BF03 core text/data fixtures without importing or executing native IR."""

from __future__ import annotations

import argparse
import copy
import json
import re
import sys
from pathlib import Path
from typing import Any, NoReturn

MAX_BYTES = 1048576
MAX_TEXT = 65536
MAX_ID = 2147483647
PROFILE = "core-text-v1"
DEFAULT_CORPUS = (
    Path(__file__).resolve().parents[1] / "specs/fixtures/compiler-ir-v1/cases.json"
)

# A separately authored literal matrix, never read from fixture or native enum.
# The operand alphabet describes data roles, not execution or native ordinals.
OPS = (
    ("LOAD_IMM", "constants", "register immediate"),
    ("LOAD_ADDR", "constants", "register label"),
    ("LOAD_BYTE", "memory", "register register register|immediate"),
    ("STORE_BYTE", "memory", "register register register|immediate"),
    ("LOAD_WORD", "memory", "register register register|immediate"),
    ("STORE_WORD", "memory", "register register register|immediate"),
    ("ADD", "arithmetic", "register register register"),
    ("ADD_IMM", "arithmetic", "register register immediate"),
    ("SUB", "arithmetic", "register register register"),
    ("AND", "arithmetic", "register register register"),
    ("AND_IMM", "arithmetic", "register register immediate"),
    ("CMP_EQ", "comparison", "register register register"),
    ("CMP_NE", "comparison", "register register register"),
    ("CMP_LT", "comparison", "register register register"),
    ("CMP_GT", "comparison", "register register register"),
    ("LABEL", "control_flow", "label"),
    ("JUMP", "control_flow", "label"),
    ("BRANCH_Z", "control_flow", "register label"),
    ("BRANCH_NZ", "control_flow", "register label"),
    ("CALL", "control_flow", "label"),
    ("RET", "control_flow", ""),
    ("SYSCALL", "system", "immediate"),
    ("HALT", "system", ""),
    ("NOP", "meta", ""),
    ("COMMENT", "meta", "text"),
)
SHAPES = {name: shape.split() for name, _, shape in OPS}
ROSTERS = {
    "programs": ["empty", "all_core", "sparse_ids", "numeric_bounds"],
    "invalid_programs": [
        "meta_id",
        "duplicate_id",
        "negative_id",
        "version_two",
        "register_over",
        "immediate_over",
        "wrong_arity",
        "wrong_tag",
        "register_label",
        "comment_hash",
        "comment_space",
        "duplicate_label",
        "duplicate_data",
        "extension_MUL",
        "extension_DIV",
        "extension_OR",
        "extension_OR_IMM",
        "extension_XOR",
        "extension_XOR_IMM",
        "extension_NOT",
    ],
    "invalid_texts": [
        "missing_version",
        "version_two",
        "duplicate_version",
        "missing_entry",
        "duplicate_entry",
        "late_data",
        "partial_id",
        "missing_id",
        "leading_zero",
        "plus_id",
        "float",
        "negative_zero",
        "wrong_case",
        "unknown_op",
        "crlf",
        "no_final_lf",
        "tab",
        "extra_blank",
        "padded_halt",
        "comment_only_id",
        "unknown_directive",
        "empty_text",
    ],
    "id_sequences": ["initial", "empty", "suffix", "last"],
}
SPARSE_GOLDEN = (
    ".version 1\n\n.entry _start\n\n_start:\n"
    "  LOAD_IMM   v0, -1  ; #42\n  ; note\n  NOP  ; #7\n  HALT  ; #99\n"
)
POSITIVE_TEXT = {
    "empty": ".version 1\n\n.entry _start\n",
    "sparse_ids": SPARSE_GOLDEN,
    "numeric_bounds": (
        ".version 1\n\n.data z 65535 255\n\n.data a 0 0\n\n.entry _start\n"
        "  LOAD_IMM   v65535, -2147483648  ; #2147483647\n"
        "  ADD_IMM    v0, v65535, 2147483647  ; #0\n  ; two  spaces\n"
    ),
    "all_core": (
        ".version 1\n\n.entry _start\n"
        "  LOAD_IMM   v0, -1  ; #0\n  LOAD_ADDR  v1, tape  ; #1\n"
        "  LOAD_BYTE  v2, v1, v0  ; #2\n  STORE_BYTE v2, v1, 0  ; #3\n"
        "  LOAD_WORD  v3, v1, 4  ; #4\n  STORE_WORD v3, v1, v0  ; #5\n"
        "  ADD        v0, v1, v2  ; #6\n  ADD_IMM    v0, v1, -2  ; #7\n"
        "  SUB        v0, v1, v2  ; #8\n  AND        v0, v1, v2  ; #9\n"
        "  AND_IMM    v0, v1, 255  ; #10\n  CMP_EQ     v0, v1, v2  ; #11\n"
        "  CMP_NE     v0, v1, v2  ; #12\n  CMP_LT     v0, v1, v2  ; #13\n"
        "  CMP_GT     v0, v1, v2  ; #14\n\n_start:\n"
        "  JUMP       absent  ; #16\n  BRANCH_Z   v0, _start  ; #17\n"
        "  BRANCH_NZ  v1, absent  ; #18\n  CALL       absent  ; #19\n"
        "  RET  ; #20\n  SYSCALL    0  ; #21\n  HALT  ; #22\n"
        "  NOP  ; #23\n  ; note\n"
    ),
}
NEGATIVE_TEXT = dict(
    zip(
        ROSTERS["invalid_texts"],
        (
            "\n.entry _start\n",
            ".version 2\n\n.entry _start\n",
            ".version 1\n.version 1\n\n.entry _start\n",
            ".version 1\n",
            ".version 1\n\n.entry a\n.entry b\n",
            ".version 1\n\n.entry a\n\n.data x 1 0\n",
            SPARSE_GOLDEN.replace("#42", "#42junk"),
            SPARSE_GOLDEN.replace("  ; #42", ""),
            SPARSE_GOLDEN.replace("#42", "#042"),
            SPARSE_GOLDEN.replace("#42", "#+42"),
            SPARSE_GOLDEN.replace("v0, -1", "v0, 1.0"),
            SPARSE_GOLDEN.replace("v0, -1", "v0, -0"),
            SPARSE_GOLDEN.replace("LOAD_IMM", "load_imm"),
            SPARSE_GOLDEN.replace("LOAD_IMM", "BOGUS"),
            SPARSE_GOLDEN.replace("\n", "\r\n"),
            SPARSE_GOLDEN[:-1],
            SPARSE_GOLDEN.replace("  NOP", "\tNOP"),
            SPARSE_GOLDEN + "\n",
            SPARSE_GOLDEN.replace("  HALT  ;", "  HALT         ;"),
            SPARSE_GOLDEN.replace("  ; note", "  ; #9"),
            SPARSE_GOLDEN + ".other 1\n",
            "",
        ),
        strict=True,
    )
)
ID_INPUTS = {"initial": (0, 3), "empty": (0, 0), "suffix": (42, 2), "last": (MAX_ID, 1)}


def _negative_programs() -> dict[str, Any]:
    """Independent typed witnesses: never derive a negative from fixture data."""
    base = {
        "version": 1,
        "entry": "_start",
        "data": [],
        "instructions": [
            {
                "op": "LABEL",
                "operands": [{"kind": "label", "name": "_start"}],
                "id": -1,
            },
            {
                "op": "LOAD_IMM",
                "operands": [
                    {"kind": "register", "index": 0},
                    {"kind": "immediate", "value": -1},
                ],
                "id": 42,
            },
            {
                "op": "COMMENT",
                "operands": [{"kind": "text", "value": "note"}],
                "id": -1,
            },
            {"op": "NOP", "operands": [], "id": 7},
            {"op": "HALT", "operands": [], "id": 99},
        ],
    }
    results = {name: copy.deepcopy(base) for name in ROSTERS["invalid_programs"]}
    results["meta_id"]["instructions"][0]["id"] = 8
    results["duplicate_id"]["instructions"][-1]["id"] = 42
    results["negative_id"]["instructions"][1]["id"] = -1
    results["version_two"]["version"] = 2
    results["register_over"]["instructions"][1]["operands"][0]["index"] = 65536
    results["immediate_over"]["instructions"][1]["operands"][1]["value"] = 2147483648
    results["wrong_arity"]["instructions"][1]["operands"].pop()
    results["wrong_tag"]["instructions"][1]["operands"][1] = {
        "kind": "label",
        "name": "x",
    }
    results["register_label"]["entry"] = "v1"
    results["comment_hash"]["instructions"][2]["operands"][0]["value"] = "#lost"
    results["comment_space"]["instructions"][2]["operands"][0]["value"] = " note"
    results["duplicate_label"]["instructions"].append(
        copy.deepcopy(base["instructions"][0])
    )
    results["duplicate_data"]["data"] = [{"label": "x", "size": 0, "init": 0}] * 2
    for op in ("MUL", "DIV", "OR", "OR_IMM", "XOR", "XOR_IMM", "NOT"):
        results["extension_" + op]["instructions"][1]["op"] = op
    return results


def _equal(actual: Any, expected: Any) -> bool:
    """JSON equality must not equate a boolean with an integer witness."""
    if type(actual) is not type(expected):
        return False
    if type(actual) is dict:
        return actual.keys() == expected.keys() and all(
            _equal(actual[k], expected[k]) for k in actual
        )
    if type(actual) is list:
        return len(actual) == len(expected) and all(
            _equal(a, b) for a, b in zip(actual, expected)
        )
    return actual == expected


def _tree(value: Any) -> None:
    """Cap direct-object callers too: cycles cannot bypass the work/depth budget."""
    pending = [(value, 1)]
    visited = string_bytes = 0
    while pending:
        item, depth = pending.pop()
        visited += 1
        if visited > 100000:
            raise ValueError("decoded tree exceeds work limit")
        if type(item) in (dict, list):
            if depth > 16 or len(item) > 100000:
                raise ValueError("decoded container exceeds depth/work limit")
            children = (
                list(item.keys()) + list(item.values()) if type(item) is dict else item
            )
            pending.extend((child, depth + 1) for child in children)
        elif type(item) is str:
            # UTF-8 strict rejects surrogate code points even on direct callers.
            if len(item) > MAX_BYTES - string_bytes:
                raise ValueError("decoded strings exceed byte limit")
            string_bytes += len(item.encode("utf-8"))
            if string_bytes > MAX_BYTES:
                raise ValueError("decoded strings exceed byte limit")


def _unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate decoded JSON key")
        result[key] = value
    return result


def _json_int(token: str) -> int:
    if len(token.lstrip("-")) > 10:
        raise ValueError("integer token exceeds ten digits")
    return int(token)


def _not_integer(_token: str) -> NoReturn:
    raise ValueError("float/exponent/nonfinite JSON numbers are outside the profile")


def parse_json(raw: bytes) -> Any:
    """Read one unambiguous scalar-only JSON value, bounded before recursion."""
    if type(raw) is not bytes or len(raw) > MAX_BYTES:
        raise ValueError("corpus exceeds byte limit or is not bytes")
    text = raw.decode("utf-8")
    depth = 0
    quoted = escaped = False
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
        elif char in "[{":
            depth += 1
            if depth > 16:
                raise ValueError("JSON depth exceeds sixteen")
        elif char in "]}":
            depth -= 1
    value = json.loads(
        text,
        object_pairs_hook=_unique,
        parse_int=_json_int,
        parse_float=_not_integer,
        parse_constant=_not_integer,
    )
    _tree(value)
    return value


def load_corpus(path: Path) -> Any:
    with path.open("rb") as stream:
        return parse_json(stream.read(MAX_BYTES + 1))


def _fields(value: Any, names: str) -> dict[str, Any]:
    if type(value) is not dict or set(value) != set(names.split()):
        raise ValueError(f"expected closed fields: {names}")
    return value


def _array(value: Any, maximum: int) -> list[Any]:
    if type(value) is not list or len(value) > maximum:
        raise ValueError("array type/count exceeds profile")
    return value


def _int(value: Any, low: int, high: int) -> int:
    if type(value) is not int or not low <= value <= high:
        raise ValueError(f"expected integer {low}..{high}")
    return value


def _label(value: Any) -> str:
    if type(value) is not str or not re.fullmatch(
        r"[A-Za-z_][A-Za-z0-9_]{0,63}", value
    ):
        raise ValueError("invalid ASCII label")
    if re.fullmatch(r"v[0-9]+", value):
        raise ValueError("label collides with register syntax")
    return value


def _operand(value: Any) -> str:
    if type(value) is not dict or type(value.get("kind")) is not str:
        raise ValueError("operand needs a known tag")
    kind = value["kind"]
    member = {
        "register": "index",
        "immediate": "value",
        "label": "name",
        "text": "value",
    }.get(kind)
    if member is None:
        raise ValueError("unknown operand tag")
    _fields(value, f"kind {member}")
    data = value[member]
    if kind == "register":
        _int(data, 0, 65535)
    elif kind == "immediate":
        _int(data, -2147483648, 2147483647)
    elif kind == "label":
        _label(data)
    elif (
        type(data) is not str
        or not 1 <= len(data) <= 256
        or data.strip(" ") != data
        or data.startswith("#")
        or any(not 32 <= ord(c) <= 126 or c in ";:" for c in data)
    ):
        raise ValueError("invalid unescaped COMMENT payload")
    return kind


def validate_program(program: Any) -> None:
    """Validate typed records independently of text; no label resolution occurs."""
    _tree(program)
    p = _fields(program, "version entry data instructions")
    _int(p["version"], 1, 1)
    _label(p["entry"])
    data_labels: set[str] = set()
    for row in _array(p["data"], 32):
        _fields(row, "label size init")
        label = _label(row["label"])
        if label in data_labels:
            raise ValueError("duplicate data label")
        data_labels.add(label)
        _int(row["size"], 0, 65535)
        _int(row["init"], 0, 255)
    identities: set[int] = set()
    labels: set[str] = set()
    for row in _array(p["instructions"], 256):
        _fields(row, "op operands id")
        op = row["op"]
        if type(op) is not str or op not in SHAPES:
            raise ValueError("operation is outside core25")
        operands = _array(row["operands"], 3)
        tags = [_operand(operand) for operand in operands]
        shape = SHAPES[op]
        if len(tags) != len(shape) or any(
            tag not in allowed.split("|") for tag, allowed in zip(tags, shape)
        ):
            raise ValueError("wrong operand arity or role")
        if op in ("LABEL", "COMMENT"):
            _int(row["id"], -1, -1)
        else:
            identity = _int(row["id"], 0, MAX_ID)
            if identity in identities:
                raise ValueError("duplicate ordinary ID")
            identities.add(identity)
        if op == "LABEL":
            label = operands[0]["name"]
            if label in labels:
                raise ValueError("duplicate instruction label")
            labels.add(label)


def _render(program: dict[str, Any]) -> str:
    """The canonical grammar is explicit, including zero-operand spacing."""
    lines = [".version 1"]
    for row in program["data"]:
        lines.extend(("", f".data {row['label']} {row['size']} {row['init']}"))
    lines.extend(("", f".entry {program['entry']}"))
    for row in program["instructions"]:
        op, operands = row["op"], row["operands"]
        if op == "LABEL":
            lines.extend(("", operands[0]["name"] + ":"))
        elif op == "COMMENT":
            lines.append("  ; " + operands[0]["value"])
        else:
            spellings = [
                "v" + str(o["index"])
                if o["kind"] == "register"
                else str(o["value"])
                if o["kind"] == "immediate"
                else o["name"]
                for o in operands
            ]
            body = op.ljust(11) + ", ".join(spellings) if operands else op
            lines.append(f"  {body}  ; #{row['id']}")
    result = "\n".join(lines) + "\n"
    if len(result.encode("utf-8")) > MAX_TEXT:
        raise ValueError("printed text exceeds byte limit")
    return result


def print_program(program: Any) -> str:
    validate_program(program)
    return _render(program)


def _decimal(token: str) -> int:
    if len(token) > 11 or not re.fullmatch(r"0|-?[1-9][0-9]*", token):
        raise ValueError("noncanonical decimal token")
    return int(token)


def parse_text(text: str) -> dict[str, Any]:
    """Parse canonical data text only; re-rendering rejects spacing aliases."""
    if (
        type(text) is not str
        or len(text) > MAX_TEXT
        or len(text.encode("utf-8")) > MAX_TEXT
    ):
        raise ValueError("text type/byte limit")
    if not text.isascii() or "\r" in text or not text.endswith("\n"):
        raise ValueError("text must be ASCII/LF with final LF")
    lines = text[:-1].split("\n")
    if lines[0] != ".version 1":
        raise ValueError("first line must declare supported version1")
    cursor = 1
    data = []
    while (
        cursor + 1 < len(lines)
        and lines[cursor] == ""
        and lines[cursor + 1].startswith(".data ")
    ):
        parts = lines[cursor + 1].split(" ")
        if len(parts) != 4 or len(data) >= 32:
            raise ValueError("data syntax/count")
        data.append(
            {"label": parts[1], "size": _decimal(parts[2]), "init": _decimal(parts[3])}
        )
        cursor += 2
    if (
        cursor + 1 >= len(lines)
        or lines[cursor] != ""
        or not lines[cursor + 1].startswith(".entry ")
    ):
        raise ValueError("entry directive missing or misplaced")
    program = {
        "version": 1,
        "entry": lines[cursor + 1][7:],
        "data": data,
        "instructions": [],
    }
    cursor += 2
    while cursor < len(lines):
        if len(program["instructions"]) >= 256:
            raise ValueError("instruction count")
        line = lines[cursor]
        if line == "":
            cursor += 1
            if cursor >= len(lines) or not lines[cursor].endswith(":"):
                raise ValueError("blank line must precede a LABEL")
            row = {
                "op": "LABEL",
                "operands": [{"kind": "label", "name": lines[cursor][:-1]}],
                "id": -1,
            }
        elif line.startswith("  ; "):
            row = {
                "op": "COMMENT",
                "operands": [{"kind": "text", "value": line[4:]}],
                "id": -1,
            }
        else:
            match = re.fullmatch(r"  ([A-Z_]+)(.*?)  ; #([0-9]+)", line)
            if match is None:
                raise ValueError("ordinary instruction syntax/ID")
            name, tail, identity = match.groups()
            operands = []
            if tail.strip():
                for token in tail.strip().split(", "):
                    if re.fullmatch(r"v[0-9]+", token):
                        operands.append(
                            {"kind": "register", "index": _decimal(token[1:])}
                        )
                    elif re.fullmatch(r"[-+0-9].*", token):
                        operands.append({"kind": "immediate", "value": _decimal(token)})
                    else:
                        operands.append({"kind": "label", "name": token})
            row = {"op": name, "operands": operands, "id": _decimal(identity)}
        program["instructions"].append(row)
        cursor += 1
    validate_program(program)
    if _render(program) != text:
        raise ValueError("text is not the canonical byte representation")
    return program


def id_sequence(start: int, count: int) -> tuple[list[int], int]:
    _int(start, 0, MAX_ID)
    _int(count, 0, 256)
    if count and start + count - 1 > MAX_ID:
        raise ValueError("ID allocation exhausted")
    return list(range(start, start + count)), start + count


def _must_reject(function: Any, value: Any) -> None:
    try:
        function(value)
    except ValueError:
        return
    raise ValueError("negative fixture unexpectedly accepted")


def validate_document(document: Any) -> dict[str, Any]:
    """Check schema, complete roster, literal matrix and independent goldens."""
    _tree(document)
    root = _fields(
        document,
        "schema_version profile opcodes programs invalid_programs invalid_texts id_sequences",
    )
    _int(root["schema_version"], 1, 1)
    if root["profile"] != PROFILE:
        raise ValueError("unsupported core profile")
    rows = _array(root["opcodes"], 25)
    if len(rows) != 25:
        raise ValueError("core table must contain25 operations")
    for row, (name, category, shape) in zip(rows, OPS):
        _fields(row, "name category operands")
        if row != {"name": name, "category": category, "operands": shape.split()}:
            raise ValueError("opcode table disagrees with literal BF03 oracle")
    total_cases = sum(len(_array(root[key], 128)) for key in ROSTERS)
    if total_cases > 128:
        raise ValueError("too many cases")
    # Count both positive and rejected typed payloads before semantic dispatch.
    programs = [
        row.get("program")
        for key in ("programs", "invalid_programs")
        for row in root[key]
        if type(row) is dict
    ]
    instructions = declarations = operands = 0
    for program in programs:
        if type(program) is dict:
            instructions += len(_array(program.get("instructions"), 256))
            declarations += len(_array(program.get("data"), 32))
            for row in program["instructions"]:
                if type(row) is dict:
                    operands += len(_array(row.get("operands"), 3))
    if instructions > 4096 or declarations > 512 or operands > 12288:
        raise ValueError("aggregate typed work budget")
    negative_programs = _negative_programs()
    for key, roster in ROSTERS.items():
        rows = root[key]
        if [row.get("name") if type(row) is dict else None for row in rows] != roster:
            raise ValueError("case roster omitted, reordered or duplicated")
        for row in rows:
            fields = {
                "programs": "name program text",
                "invalid_programs": "name program",
                "invalid_texts": "name text",
                "id_sequences": "name start count values next",
            }[key]
            _fields(row, fields)
            if key == "programs":
                if row["text"] != POSITIVE_TEXT[row["name"]]:
                    raise ValueError("named positive witness changed")
                if (
                    print_program(row["program"]) != row["text"]
                    or parse_text(row["text"]) != row["program"]
                ):
                    raise ValueError("program golden mismatch")
            elif key == "invalid_programs":
                if not _equal(row["program"], negative_programs[row["name"]]):
                    raise ValueError("named negative typed witness changed")
                _must_reject(validate_program, row["program"])
            elif key == "invalid_texts":
                if (
                    type(row["text"]) is not str
                    or row["text"] != NEGATIVE_TEXT[row["name"]]
                ):
                    raise ValueError("named negative text witness changed")
                _must_reject(parse_text, row["text"])
            else:
                if not _equal(
                    [row["start"], row["count"]], list(ID_INPUTS[row["name"]])
                ):
                    raise ValueError("named generator input witness changed")
                values, next_counter = id_sequence(row["start"], row["count"])
                for identity in _array(row["values"], 256):
                    _int(identity, 0, MAX_ID)
                _int(row["next"], 0, MAX_ID + 1)
                if row["values"] != values or row["next"] != next_counter:
                    raise ValueError("ID golden mismatch")
    if root["programs"][2]["text"] != SPARSE_GOLDEN:
        raise ValueError("sparse identity witness disagrees with literal spec golden")
    if [row["op"] for row in root["programs"][1]["program"]["instructions"]] != [
        name for name, _, _ in OPS
    ]:
        raise ValueError("all-core program must exercise each operation in table order")
    return {
        "profile": PROFILE,
        "opcodes": 25,
        "cases": total_cases,
        "native_adopters_checked": 0,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_CORPUS)
    args = parser.parse_args(argv)
    try:
        summary = validate_document(load_corpus(args.fixture))
    except (OSError, ValueError) as error:
        print(f"BF03 invalid corpus: {error}", file=sys.stderr)
        return 1
    print(json.dumps(summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
