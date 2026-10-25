#!/usr/bin/env python3
"""Validate the closed CommonMark 0.31.2 foundation without native code."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[2]
FIXTURE_ROOT = ROOT / "code/specs/fixtures/commonmark-0.31.2"
DEFAULT_CORPUS = FIXTURE_ROOT / "spec.json"
DEFAULT_RENDERER = FIXTURE_ROOT / "renderer-cases.json"
DEFAULT_PROVENANCE = FIXTURE_ROOT / "provenance.json"

MAX_CORPUS_BYTES = 262_144
MAX_RENDERER_BYTES = 131_072
MAX_PROVENANCE_BYTES = 8_192
MAX_JSON_DEPTH = 32
MAX_TREE_ITEMS = 32_768
MAX_TREE_TEXT_BYTES = 524_288

OFFICIAL_RAW_BYTES = 140_487
OFFICIAL_RAW_SHA256 = "d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20"
OFFICIAL_SEMANTIC_SHA256 = (
    "0153ea34b32feb95ffca06fd6c5b543673aa803aabfbef719192b6e8be122d57"
)
RENDERER_SEMANTIC_SHA256 = (
    "fa211cec61ae33764ad388e1a6ee4f03b94c6c87665c0b426a5dbe3b1b711828"
)

RENDERER_LIMITS = {
    "max_ast_nodes": 256,
    "max_ast_depth": 32,
    "max_children": 64,
    "max_ast_text_bytes": 8_192,
    "max_html_bytes": 32_768,
}
RENDERER_CASE_IDS = (
    "empty-document",
    "headings-all-levels",
    "paragraph-escaping-and-breaks",
    "inline-emphasis-strong-code",
    "code-blocks",
    "nested-blockquote",
    "tight-unordered-list",
    "loose-ordered-list-start",
    "thematic-break",
    "link-image-attributes",
    "dangerous-destinations",
    "url-and-email-autolinks",
    "raw-html-enabled",
    "raw-html-suppressed",
    "non-html-raw-skipped",
    "mixed-block-order",
)
TE00_NODE_TYPES = {
    "document",
    "heading",
    "paragraph",
    "code_block",
    "blockquote",
    "list",
    "list_item",
    "thematic_break",
    "raw_block",
    "text",
    "emphasis",
    "strong",
    "code_span",
    "link",
    "image",
    "autolink",
    "raw_inline",
    "hard_break",
    "soft_break",
}
PROVENANCE = {
    "schema_version": 1,
    "profile": "commonmark-0.31.2",
    "source_url": "https://spec.commonmark.org/0.31.2/spec.json",
    "specification_url": "https://spec.commonmark.org/0.31.2/",
    "version": "0.31.2",
    "published": "2024-01-28",
    "author": "John MacFarlane",
    "license": {
        "spdx": "CC-BY-SA-4.0",
        "url": "https://creativecommons.org/licenses/by-sa/4.0/",
    },
    "artifact": {
        "path": "spec.json",
        "bytes": OFFICIAL_RAW_BYTES,
        "sha256": OFFICIAL_RAW_SHA256,
        "examples": 652,
    },
}


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _semantic_sha256(value: Any) -> str:
    raw = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return _sha256(raw)


def _scan_depth(text: str) -> None:
    depth = 0
    quoted = False
    escaped = False
    for character in text:
        if quoted:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                quoted = False
        elif character == '"':
            quoted = True
        elif character in "[{":
            depth += 1
            if depth > MAX_JSON_DEPTH:
                raise ValueError("JSON container depth exceeds 32")
        elif character in "]}":
            depth -= 1


def _guard_tree(value: Any) -> None:
    """Bound direct decoded callers before walking their complete object graph."""

    pending = [(value, 1)]
    seen_containers: set[int] = set()
    visited = 0
    text_bytes = 0
    while pending:
        item, depth = pending.pop()
        visited += 1
        if type(item) in (dict, list):
            if depth > MAX_JSON_DEPTH:
                raise ValueError("JSON container depth exceeds 32")
            identity = id(item)
            if identity in seen_containers:
                raise ValueError("decoded object graph must be a tree")
            seen_containers.add(identity)
            child_count = len(item) * 2 if type(item) is dict else len(item)
            if visited + len(pending) + child_count > MAX_TREE_ITEMS:
                raise ValueError("decoded object graph exceeds 32768 items")
            if type(item) is dict:
                for key, child in item.items():
                    pending.append((key, depth + 1))
                    pending.append((child, depth + 1))
            else:
                pending.extend((child, depth + 1) for child in item)
        elif isinstance(item, str):
            if len(item) > MAX_TREE_TEXT_BYTES - text_bytes:
                raise ValueError("decoded object graph text exceeds 524288 UTF-8 bytes")
            encoded = item.encode("utf-8", errors="surrogatepass")
            if len(encoded) > MAX_TREE_TEXT_BYTES - text_bytes:
                raise ValueError("decoded object graph text exceeds 524288 UTF-8 bytes")
            text_bytes += len(encoded)
            if any(0xD800 <= ord(character) <= 0xDFFF for character in item):
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
        raise ValueError("integer literal exceeds the fixture domain")
    return int(text)


def _reject_number(_text: str) -> NoReturn:
    raise ValueError("fractional, exponent and nonfinite numbers are not admitted")


def parse_json(raw: bytes, maximum_bytes: int) -> Any:
    if type(raw) is not bytes or type(maximum_bytes) is not int or maximum_bytes < 1:
        raise ValueError("parse_json requires bytes and a positive integer ceiling")
    if len(raw) > maximum_bytes:
        raise ValueError(f"JSON transport exceeds {maximum_bytes} bytes")
    text = raw.decode("utf-8")
    if text.startswith("\ufeff"):
        raise ValueError("JSON transport must not start with a byte-order mark")
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


def _read_bounded(path: Path, maximum_bytes: int) -> bytes:
    with path.open("rb") as stream:
        raw = stream.read(maximum_bytes + 1)
    if len(raw) > maximum_bytes:
        raise ValueError(f"{path.name} exceeds {maximum_bytes} bytes")
    return raw


def load_corpus(path: Path = DEFAULT_CORPUS) -> Any:
    raw = _read_bounded(path, MAX_CORPUS_BYTES)
    if len(raw) != OFFICIAL_RAW_BYTES or _sha256(raw) != OFFICIAL_RAW_SHA256:
        raise ValueError("official CommonMark corpus bytes or digest changed")
    return parse_json(raw, MAX_CORPUS_BYTES)


def load_renderer(path: Path = DEFAULT_RENDERER) -> Any:
    return parse_json(_read_bounded(path, MAX_RENDERER_BYTES), MAX_RENDERER_BYTES)


def load_provenance(path: Path = DEFAULT_PROVENANCE) -> Any:
    return parse_json(_read_bounded(path, MAX_PROVENANCE_BYTES), MAX_PROVENANCE_BYTES)


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


def _int(value: Any, where: str, minimum: int, maximum: int) -> int:
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError(f"{where}: expected integer {minimum}..{maximum}")
    return value


def validate_corpus(document: Any) -> int:
    _guard_tree(document)
    examples = _list(document, "corpus", minimum=652, maximum=652)
    previous_end = 0
    for index, value in enumerate(examples):
        where = f"examples[{index}]"
        example = _fields(
            value,
            {"markdown", "html", "example", "start_line", "end_line", "section"},
            where,
        )
        _text(example["markdown"], f"{where}.markdown", 8_192)
        _text(example["html"], f"{where}.html", 32_768)
        number = _int(example["example"], f"{where}.example", 1, 652)
        if number != index + 1:
            raise ValueError(f"{where}: example number must equal array position")
        start = _int(example["start_line"], f"{where}.start_line", 1, 100_000)
        end = _int(example["end_line"], f"{where}.end_line", 2, 100_000)
        if start >= end:
            raise ValueError(f"{where}: source line range must increase")
        if start <= previous_end:
            raise ValueError(f"{where}: source line ranges must not overlap")
        previous_end = end
        _text(example["section"], f"{where}.section", 128, empty=False)
    if _semantic_sha256(examples) != OFFICIAL_SEMANTIC_SHA256:
        raise ValueError("official CommonMark semantic digest changed")
    return len(examples)


def validate_provenance(document: Any) -> int:
    _guard_tree(document)
    root = _fields(
        document,
        {
            "schema_version",
            "profile",
            "source_url",
            "specification_url",
            "version",
            "published",
            "author",
            "license",
            "artifact",
        },
        "provenance",
    )
    _int(root["schema_version"], "provenance.schema_version", 1, 1)
    for name in (
        "profile",
        "source_url",
        "specification_url",
        "version",
        "published",
        "author",
    ):
        _text(root[name], f"provenance.{name}", 256, empty=False)
    license_record = _fields(root["license"], {"spdx", "url"}, "license")
    _text(license_record["spdx"], "license.spdx", 64, empty=False)
    _text(license_record["url"], "license.url", 256, empty=False)
    artifact = _fields(
        root["artifact"], {"path", "bytes", "sha256", "examples"}, "artifact"
    )
    _text(artifact["path"], "artifact.path", 64, empty=False)
    _int(artifact["bytes"], "artifact.bytes", 1, MAX_CORPUS_BYTES)
    _text(artifact["sha256"], "artifact.sha256", 64, empty=False)
    _int(artifact["examples"], "artifact.examples", 1, 652)
    if root != PROVENANCE:
        raise ValueError("provenance must equal the published CommonMark record")
    return 1


def _count_node(state: dict[str, Any], depth: int, kind: str) -> None:
    state["nodes"] += 1
    state["kinds"].add(kind)
    if state["nodes"] > RENDERER_LIMITS["max_ast_nodes"]:
        raise ValueError("AST node count exceeds 256")
    if depth > RENDERER_LIMITS["max_ast_depth"]:
        raise ValueError("AST depth exceeds 32")


def _children(value: Any, where: str, *, minimum: int = 0) -> list[Any]:
    return _list(
        value,
        where,
        minimum=minimum,
        maximum=RENDERER_LIMITS["max_children"],
    )


def _node(value: Any, where: str) -> tuple[dict[str, Any], str]:
    if type(value) is not dict or type(value.get("type")) is not str:
        raise ValueError(f"{where}: node must have a string type")
    return value, value["type"]


def _validate_inline(
    value: Any,
    state: dict[str, Any],
    depth: int,
    where: str,
    *,
    inside_link: bool = False,
) -> None:
    node, kind = _node(value, where)
    _count_node(state, depth, kind)
    if kind in {"text", "code_span"}:
        item = _fields(node, {"type", "value"}, where)
        _text(
            item["value"],
            f"{where}.value",
            RENDERER_LIMITS["max_ast_text_bytes"],
        )
    elif kind in {"emphasis", "strong"}:
        item = _fields(node, {"type", "children"}, where)
        for index, child in enumerate(_children(item["children"], where)):
            _validate_inline(
                child,
                state,
                depth + 1,
                f"{where}.children[{index}]",
                inside_link=inside_link,
            )
    elif kind == "link":
        if inside_link:
            raise ValueError(f"{where}: links must not nest")
        item = _fields(node, {"type", "destination", "title", "children"}, where)
        _text(
            item["destination"],
            f"{where}.destination",
            RENDERER_LIMITS["max_ast_text_bytes"],
        )
        if item["title"] is not None:
            _text(
                item["title"],
                f"{where}.title",
                RENDERER_LIMITS["max_ast_text_bytes"],
            )
        for index, child in enumerate(_children(item["children"], where)):
            _validate_inline(
                child,
                state,
                depth + 1,
                f"{where}.children[{index}]",
                inside_link=True,
            )
    elif kind == "image":
        item = _fields(node, {"type", "destination", "title", "alt"}, where)
        for name in ("destination", "alt"):
            _text(
                item[name],
                f"{where}.{name}",
                RENDERER_LIMITS["max_ast_text_bytes"],
            )
        if item["title"] is not None:
            _text(
                item["title"],
                f"{where}.title",
                RENDERER_LIMITS["max_ast_text_bytes"],
            )
    elif kind == "autolink":
        item = _fields(node, {"type", "destination", "isEmail"}, where)
        _text(
            item["destination"],
            f"{where}.destination",
            RENDERER_LIMITS["max_ast_text_bytes"],
        )
        if type(item["isEmail"]) is not bool:
            raise ValueError(f"{where}.isEmail: expected boolean")
    elif kind == "raw_inline":
        item = _fields(node, {"type", "format", "value"}, where)
        _text(item["format"], f"{where}.format", 64, empty=False)
        _text(
            item["value"],
            f"{where}.value",
            RENDERER_LIMITS["max_ast_text_bytes"],
        )
    elif kind in {"hard_break", "soft_break"}:
        _fields(node, {"type"}, where)
    else:
        raise ValueError(f"{where}: unsupported inline node {kind!r}")


def _validate_block(value: Any, state: dict[str, Any], depth: int, where: str) -> None:
    node, kind = _node(value, where)
    _count_node(state, depth, kind)
    if kind == "heading":
        item = _fields(node, {"type", "level", "children"}, where)
        _int(item["level"], f"{where}.level", 1, 6)
        for index, child in enumerate(_children(item["children"], where)):
            _validate_inline(child, state, depth + 1, f"{where}.children[{index}]")
    elif kind == "paragraph":
        item = _fields(node, {"type", "children"}, where)
        for index, child in enumerate(_children(item["children"], where)):
            _validate_inline(child, state, depth + 1, f"{where}.children[{index}]")
    elif kind == "code_block":
        item = _fields(node, {"type", "language", "value"}, where)
        if item["language"] is not None:
            _text(item["language"], f"{where}.language", 128, empty=False)
        _text(
            item["value"],
            f"{where}.value",
            RENDERER_LIMITS["max_ast_text_bytes"],
        )
    elif kind == "blockquote":
        item = _fields(node, {"type", "children"}, where)
        for index, child in enumerate(_children(item["children"], where)):
            _validate_block(child, state, depth + 1, f"{where}.children[{index}]")
    elif kind == "list":
        item = _fields(node, {"type", "ordered", "start", "tight", "children"}, where)
        if type(item["ordered"]) is not bool or type(item["tight"]) is not bool:
            raise ValueError(f"{where}: list flags must be boolean")
        if item["ordered"]:
            _int(item["start"], f"{where}.start", 0, 999_999_999)
        elif item["start"] is not None:
            raise ValueError(f"{where}: unordered list start must be null")
        for index, child in enumerate(_children(item["children"], where, minimum=1)):
            if type(child) is not dict or child.get("type") != "list_item":
                raise ValueError(f"{where}: list children must be list_item nodes")
            _validate_block(child, state, depth + 1, f"{where}.children[{index}]")
    elif kind == "list_item":
        item = _fields(node, {"type", "children"}, where)
        for index, child in enumerate(_children(item["children"], where)):
            _validate_block(child, state, depth + 1, f"{where}.children[{index}]")
    elif kind == "thematic_break":
        _fields(node, {"type"}, where)
    elif kind == "raw_block":
        item = _fields(node, {"type", "format", "value"}, where)
        _text(item["format"], f"{where}.format", 64, empty=False)
        _text(
            item["value"],
            f"{where}.value",
            RENDERER_LIMITS["max_ast_text_bytes"],
        )
    else:
        raise ValueError(f"{where}: unsupported block node {kind!r}")


def _validate_document(value: Any, where: str) -> set[str]:
    document = _fields(value, {"type", "children"}, where)
    if document["type"] != "document":
        raise ValueError(f"{where}: root type must be document")
    state: dict[str, Any] = {"nodes": 0, "kinds": set()}
    _count_node(state, 1, "document")
    for index, child in enumerate(_children(document["children"], where)):
        _validate_block(child, state, 2, f"{where}.children[{index}]")
    return state["kinds"]


def validate_renderer(document: Any) -> int:
    _guard_tree(document)
    root = _fields(
        document, {"schema_version", "profile", "limits", "cases"}, "renderer"
    )
    _int(root["schema_version"], "renderer.schema_version", 1, 1)
    if root["profile"] != "commonmark-renderer-v1":
        raise ValueError("renderer profile must be 'commonmark-renderer-v1'")
    limits = _fields(root["limits"], set(RENDERER_LIMITS), "renderer.limits")
    for name, expected in RENDERER_LIMITS.items():
        _int(limits[name], f"renderer.limits.{name}", expected, expected)
    cases = _list(root["cases"], "renderer.cases", minimum=16, maximum=16)
    ids: list[str] = []
    kinds: set[str] = set()
    for index, value in enumerate(cases):
        where = f"renderer.cases[{index}]"
        case = _fields(
            value, {"id", "suppress_raw_html", "ast", "expected_html"}, where
        )
        ids.append(_text(case["id"], f"{where}.id", 64, empty=False))
        if type(case["suppress_raw_html"]) is not bool:
            raise ValueError(f"{where}.suppress_raw_html: expected boolean")
        kinds.update(_validate_document(case["ast"], f"{where}.ast"))
        _text(
            case["expected_html"],
            f"{where}.expected_html",
            RENDERER_LIMITS["max_html_bytes"],
        )
    if tuple(ids) != RENDERER_CASE_IDS:
        raise ValueError("renderer case IDs and order changed")
    if kinds != TE00_NODE_TYPES:
        raise ValueError("renderer cases must cover every TE00 node type")
    if _semantic_sha256(root) != RENDERER_SEMANTIC_SHA256:
        raise ValueError("renderer normative literals changed")
    return len(cases)


def validate_all(
    corpus_path: Path = DEFAULT_CORPUS,
    renderer_path: Path = DEFAULT_RENDERER,
    provenance_path: Path = DEFAULT_PROVENANCE,
) -> tuple[int, int]:
    provenance = load_provenance(provenance_path)
    corpus = load_corpus(corpus_path)
    renderer = load_renderer(renderer_path)
    validate_provenance(provenance)
    return validate_corpus(corpus), validate_renderer(renderer)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--renderer", type=Path, default=DEFAULT_RENDERER)
    parser.add_argument("--provenance", type=Path, default=DEFAULT_PROVENANCE)
    args = parser.parse_args(argv)
    examples, renderer_cases = validate_all(args.corpus, args.renderer, args.provenance)
    print(
        f"validated {examples} CommonMark 0.31.2 examples "
        f"and {renderer_cases} renderer cases"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
