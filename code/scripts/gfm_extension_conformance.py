#!/usr/bin/env python3
"""Validate the closed portable GFM extension corpus without running a parser."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = ROOT / "code/specs/fixtures/gfm-extension-v1/cases.json"
PROFILE = "gfm-extension-v1"
MAX_BYTES = 131_072
MAX_JSON_DEPTH = 16
LIMITS = {
    "max_source_bytes": 8_192,
    "max_source_lines": 128,
    "max_source_line_bytes": 1_024,
    "max_ast_nodes": 256,
    "max_ast_depth": 16,
    "max_ast_text_bytes": 8_192,
    "max_html_bytes": 32_768,
}
CASE_IDS = (
    "strike-basic",
    "strike-strong-child",
    "strike-single-tilde",
    "task-checked-lower",
    "task-checked-upper",
    "task-unchecked",
    "task-like-paragraph",
    "table-alignments",
    "table-inline-strike",
    "table-short-row",
    "table-long-row",
    "table-missing-delimiter",
    "fence-named-info",
    "fence-anonymous-empty",
    "fence-inert-mermaid",
    "mixed",
)
CASE_DIGESTS = {
    "strike-basic": "e39f364fea90480a0522c310679bfb67569de9377085a93611f3ec2a8ce292d1",
    "strike-strong-child": "84e9eac966fbe8ce17d9da0dae9de547c40f6726784f0f0fe75463eb4b300440",
    "strike-single-tilde": "cc8bb0ebdd99c5d6cca72bfac7a1e40b932908b79172669aadb210a1e32a41b2",
    "task-checked-lower": "27583e35b3b8a58f3e6cc6d8a1f6dbacfb4479b193e384497584f6e4dba4ad38",
    "task-checked-upper": "a72803e6d354185c3ed23046ce4818e433b16767e57275a620aa583f47808940",
    "task-unchecked": "c2eb6d4dfdf0eb68b9991c09e91dad8b522ed2914f06dd8ef54829bc49307ad4",
    "task-like-paragraph": "2c0c6babbd9aa03e0444d5851ff49dd1db885fdd76066e4b42bab14feecb5bfb",
    "table-alignments": "b855bd39d08ba1ba9b7ae1b31295cd179bd28fbf5e011d7135b98165e785bff5",
    "table-inline-strike": "275efe4d1c459f46d4ec1020cec58b13ad974e93f94e5e497b91b19cd06b69a9",
    "table-short-row": "86c4dd4974cd68576c933390660cbbfc09a5779c264402d10d0af5f5c1264422",
    "table-long-row": "4e098f5772e2da397a8660e70b8707f075d3e215301687d294a851c80006b6b7",
    "table-missing-delimiter": "cbedabca34d583e3314eb484266018f13e57f1e8de94f0368f27a84d49add3b5",
    "fence-named-info": "febd690c39e4decbe7d95daec0e806fc2c9616f04935043bd9331d6891561dd5",
    "fence-anonymous-empty": "74c6f7afc60b8f57f9906d3735e909910cb41d5c119ea941aef57b5c2d2150d3",
    "fence-inert-mermaid": "a55a458e13b50c3d8239f83c3bb63883804f05640f31ea13497a8e8c0eaba5f7",
    "mixed": "a6e650fa70b4ca5bfa67ec36ee9d161571585cb380b2a4781e863f9dcfe71262",
}


def _scan_depth(text: str) -> None:
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
            if depth > MAX_JSON_DEPTH:
                raise ValueError("JSON container depth exceeds sixteen")
        elif char in "}]":
            depth -= 1


def _guard_tree(value: Any) -> None:
    pending = [(value, 1)]
    while pending:
        item, depth = pending.pop()
        if type(item) in (dict, list):
            if depth > MAX_JSON_DEPTH:
                raise ValueError("JSON container depth exceeds sixteen")
            children = (
                list(item.keys()) + list(item.values()) if type(item) is dict else item
            )
            pending.extend((child, depth + 1) for child in children)
        elif isinstance(item, str) and any(
            0xD800 <= ord(character) <= 0xDFFF for character in item
        ):
            raise ValueError("strings must contain Unicode scalar values")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object key")
        result[key] = value
    return result


def _integer(text: str) -> int:
    if len(text.lstrip("-")) > 10:
        raise ValueError("integer literal exceeds the corpus domain")
    return int(text)


def _reject_number(_text: str) -> NoReturn:
    raise ValueError(
        "fractional, exponent and nonfinite numbers are outside the corpus"
    )


def parse_json(raw: bytes) -> Any:
    if type(raw) is not bytes or len(raw) > MAX_BYTES:
        raise ValueError("corpus must be at most 131072 UTF-8 bytes")
    text = raw.decode("utf-8")
    if text.startswith("\ufeff"):
        raise ValueError("corpus must not start with a byte-order mark")
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
    with path.open("rb") as stream:
        return parse_json(stream.read(MAX_BYTES + 1))


def _fields(value: Any, required: set[str], where: str) -> dict[str, Any]:
    if type(value) is not dict or set(value) != required:
        raise ValueError(f"{where}: fields must be exactly {sorted(required)}")
    return value


def _list(value: Any, where: str, *, minimum: int = 0, maximum: int) -> list[Any]:
    if type(value) is not list or not minimum <= len(value) <= maximum:
        raise ValueError(f"{where}: expected {minimum}..{maximum} entries")
    return value


def _text(value: Any, where: str, maximum_bytes: int, *, empty: bool = True) -> str:
    if type(value) is not str or (not empty and value == ""):
        raise ValueError(f"{where}: expected text")
    if len(value.encode("utf-8")) > maximum_bytes:
        raise ValueError(f"{where}: text exceeds {maximum_bytes} UTF-8 bytes")
    return value


def _validate_inline(node: Any, state: dict[str, int], depth: int, where: str) -> None:
    _count_node(state, depth)
    if type(node) is not dict or type(node.get("type")) is not str:
        raise ValueError(f"{where}: inline node must have a string type")
    kind = node["type"]
    if kind == "text":
        item = _fields(node, {"type", "value"}, where)
        _text(item["value"], f"{where}.value", LIMITS["max_ast_text_bytes"])
    elif kind in {"strikethrough", "strong"}:
        item = _fields(node, {"type", "children"}, where)
        for index, child in enumerate(_list(item["children"], where, maximum=64)):
            _validate_inline(child, state, depth + 1, f"{where}.children[{index}]")
    elif kind == "soft_break":
        _fields(node, {"type"}, where)
    else:
        raise ValueError(f"{where}: unsupported inline node {kind!r}")


def _count_node(state: dict[str, int], depth: int) -> None:
    state["nodes"] += 1
    if state["nodes"] > LIMITS["max_ast_nodes"]:
        raise ValueError("AST node count exceeds 256")
    if depth > LIMITS["max_ast_depth"]:
        raise ValueError("AST depth exceeds sixteen")


def _validate_paragraph(
    node: Any, state: dict[str, int], depth: int, where: str
) -> None:
    _count_node(state, depth)
    item = _fields(node, {"type", "children"}, where)
    if item["type"] != "paragraph":
        raise ValueError(f"{where}: expected paragraph")
    for index, child in enumerate(_list(item["children"], where, maximum=64)):
        _validate_inline(child, state, depth + 1, f"{where}.children[{index}]")


def _validate_table(
    node: dict[str, Any], state: dict[str, int], depth: int, where: str
) -> None:
    align = _list(node["align"], f"{where}.align", minimum=1, maximum=16)
    if any(value not in (None, "left", "center", "right") for value in align):
        raise ValueError(f"{where}: invalid alignment")
    rows = _list(node["children"], f"{where}.children", minimum=2, maximum=32)
    for row_index, row_value in enumerate(rows):
        _count_node(state, depth + 1)
        row = _fields(row_value, {"type", "isHeader", "children"}, f"{where}.row")
        if row["type"] != "table_row" or type(row["isHeader"]) is not bool:
            raise ValueError(f"{where}: invalid table row")
        if row["isHeader"] != (row_index == 0):
            raise ValueError(f"{where}: only the first row is a header")
        cells = _list(row["children"], f"{where}.row.children", minimum=1, maximum=16)
        if len(cells) != len(align):
            raise ValueError(f"{where}: row width must equal alignment width")
        for cell_index, cell_value in enumerate(cells):
            _count_node(state, depth + 2)
            cell = _fields(cell_value, {"type", "children"}, f"{where}.cell")
            if cell["type"] != "table_cell":
                raise ValueError(f"{where}: invalid table cell")
            for inline_index, inline in enumerate(
                _list(cell["children"], f"{where}.cell.children", maximum=64)
            ):
                _validate_inline(
                    inline,
                    state,
                    depth + 3,
                    f"{where}.cells[{cell_index}].children[{inline_index}]",
                )


def _validate_block(node: Any, state: dict[str, int], depth: int, where: str) -> None:
    if type(node) is not dict or type(node.get("type")) is not str:
        raise ValueError(f"{where}: block node must have a string type")
    kind = node["type"]
    if kind == "paragraph":
        _validate_paragraph(node, state, depth, where)
    elif kind == "list":
        _count_node(state, depth)
        item = _fields(node, {"type", "ordered", "start", "tight", "children"}, where)
        if (
            item["ordered"] is not False
            or item["start"] is not None
            or type(item["tight"]) is not bool
        ):
            raise ValueError(
                f"{where}: v1 task lists are tightness-tagged unordered lists"
            )
        for index, child_value in enumerate(
            _list(item["children"], f"{where}.children", minimum=1, maximum=32)
        ):
            _count_node(state, depth + 1)
            child = _fields(
                child_value, {"type", "checked", "children"}, f"{where}.task"
            )
            if child["type"] != "task_item" or type(child["checked"]) is not bool:
                raise ValueError(f"{where}: list child must be a task item")
            for block_index, block in enumerate(
                _list(
                    child["children"], f"{where}.task.children", minimum=1, maximum=16
                )
            ):
                _validate_paragraph(
                    block,
                    state,
                    depth + 2,
                    f"{where}.children[{index}].children[{block_index}]",
                )
    elif kind == "table":
        _count_node(state, depth)
        item = _fields(node, {"type", "align", "children"}, where)
        _validate_table(item, state, depth, where)
    elif kind == "fenced_block":
        _count_node(state, depth)
        item = _fields(node, {"type", "name", "info", "value"}, where)
        value = _text(item["value"], f"{where}.value", LIMITS["max_ast_text_bytes"])
        if value and not value.endswith("\n"):
            raise ValueError(f"{where}: nonempty fence payload must end in LF")
        if item["name"] is None or item["info"] is None:
            if item["name"] is not None or item["info"] is not None:
                raise ValueError(f"{where}: fence name and info nullability must agree")
        else:
            name = _text(item["name"], f"{where}.name", 128, empty=False)
            info = _text(item["info"], f"{where}.info", 1024, empty=False)
            if not info.split() or name != info.split()[0]:
                raise ValueError(f"{where}: fence name must be the first info token")
    else:
        raise ValueError(f"{where}: unsupported block node {kind!r}")


def _validate_ast(value: Any, where: str) -> None:
    document = _fields(value, {"type", "children"}, where)
    if document["type"] != "document":
        raise ValueError(f"{where}: expected document root")
    state = {"nodes": 0}
    _count_node(state, 1)
    for index, child in enumerate(
        _list(document["children"], f"{where}.children", maximum=32)
    ):
        _validate_block(child, state, 2, f"{where}.children[{index}]")


def _case_digest(case: dict[str, Any]) -> str:
    payload = {name: case[name] for name in ("source", "expected_ast", "expected_html")}
    raw = json.dumps(
        payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(raw).hexdigest()


def validate_document(document: Any) -> int:
    _guard_tree(document)
    root = _fields(document, {"schema_version", "profile", "limits", "cases"}, "corpus")
    if type(root["schema_version"]) is not int or root["schema_version"] != 1:
        raise ValueError("schema_version must be integer 1")
    if root["profile"] != PROFILE:
        raise ValueError(f"profile must be {PROFILE!r}")
    if root["limits"] != LIMITS:
        raise ValueError("limits must equal the published v1 limits")
    cases = _list(root["cases"], "cases", minimum=16, maximum=16)
    ids: list[str] = []
    for index, case_value in enumerate(cases):
        case = _fields(
            case_value,
            {"id", "source", "expected_ast", "expected_html"},
            f"cases[{index}]",
        )
        case_id = _text(case["id"], f"cases[{index}].id", 64, empty=False)
        ids.append(case_id)
        source = _text(
            case["source"], f"cases[{index}].source", LIMITS["max_source_bytes"]
        )
        lines = source.splitlines(keepends=True)
        if len(lines) > LIMITS["max_source_lines"]:
            raise ValueError(f"cases[{index}]: source has too many lines")
        if any(
            len(line.encode("utf-8")) > LIMITS["max_source_line_bytes"]
            for line in lines
        ):
            raise ValueError(f"cases[{index}]: source line is too long")
        _validate_ast(case["expected_ast"], f"cases[{index}].expected_ast")
        _text(
            case["expected_html"],
            f"cases[{index}].expected_html",
            LIMITS["max_html_bytes"],
        )
        if CASE_DIGESTS.get(case_id) != _case_digest(case):
            raise ValueError(f"cases[{index}]: normative literal digest changed")
    if tuple(ids) != CASE_IDS:
        raise ValueError("case IDs and order must equal the published v1 roster")
    return len(cases)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_CORPUS)
    args = parser.parse_args(argv)
    count = validate_document(load_corpus(args.fixture))
    print(f"validated {count} portable GFM extension cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
