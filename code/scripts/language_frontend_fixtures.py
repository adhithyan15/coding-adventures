"""Validate the authority-free language-frontend fixture corpus.

The manifest is deliberately a checksum ledger, not a loader for production
lexers and parsers. Native packages embed canonical grammars at build time and
consume only the inert cases during tests.
"""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "code/specs/fixtures/language-frontends-v1"
FAMILIES = (
    "csharp",
    "excel",
    "java",
    "javascript",
    "lattice",
    "python",
    "ruby",
    "sql",
    "starlark",
    "typescript",
    "verilog",
    "vhdl",
)
KINDS = ("tokens", "grammar")
IDENTIFIER = re.compile(r"[a-z0-9][a-z0-9._/-]*\Z", re.ASCII)
SHA256 = re.compile(r"[0-9a-f]{64}\Z", re.ASCII)
TOKEN_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\Z", re.ASCII)
MAX_GRAMMARS = 256
MAX_CASES = 256
MAX_AST_DEPTH = 128
MAX_AST_ENTRIES = 10000
MAX_SCHEMA_BYTES = 64 * 1024
MAX_MANIFEST_BYTES = 256 * 1024
MAX_CASE_BYTES = 2 * 1024 * 1024


def _closed(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise ValueError(f"{label} has unknown or missing fields")
    return value


def _relative_grammar_path(
    root: Path, path: Any, family: str, version: str, kind: str
) -> Path:
    if not isinstance(path, str) or not path.isascii() or "\\" in path:
        raise ValueError("grammar path is not canonical ASCII")
    grammar_root = (
        "ecmascript" if family == "javascript" and version != "javascript" else family
    )
    if grammar_root == "ecmascript" and not version.startswith("es"):
        raise ValueError("non-ECMAScript JavaScript edition")
    expected = f"code/grammars/{grammar_root}/{version}.{kind}"
    if path != expected:
        raise ValueError("grammar path does not match family, version, and kind")
    target = root / path
    if not target.resolve().is_relative_to(root.resolve()):
        raise ValueError("grammar path escapes repository")
    if target.is_symlink() or not target.is_file():
        raise ValueError("grammar file missing or linked")
    return target


def validate_grammars(root: Path, document: dict[str, Any]) -> dict[str, int]:
    """Pin every canonical token/parser grammar path and exact byte digest."""
    _closed(document, {"schema_version", "kind", "grammars"}, "manifest")
    if document["schema_version"] != 1 or document["kind"] != "grammars":
        raise ValueError("unsupported grammar manifest")
    entries = document["grammars"]
    if not isinstance(entries, list) or not 24 <= len(entries) <= MAX_GRAMMARS:
        raise ValueError("grammar roster size")

    seen: set[str] = set()
    versions: dict[tuple[str, str], set[str]] = {}
    for entry in entries:
        _closed(entry, {"family", "version", "kind", "path", "sha256"}, "grammar entry")
        family, version, kind = entry["family"], entry["version"], entry["kind"]
        if family not in FAMILIES or not isinstance(version, str) or not version:
            raise ValueError("unknown grammar family or version")
        if not re.fullmatch(r"[a-z0-9][a-z0-9.]*", version, re.ASCII):
            raise ValueError("non-canonical grammar version")
        if (
            kind not in KINDS
            or not isinstance(entry["sha256"], str)
            or not SHA256.fullmatch(entry["sha256"])
        ):
            raise ValueError("invalid grammar kind or digest")
        target = _relative_grammar_path(root, entry["path"], family, version, kind)
        if entry["path"] in seen:
            raise ValueError("duplicate grammar path")
        seen.add(entry["path"])
        versions.setdefault((family, version), set()).add(kind)
        if target.stat().st_size > 1024 * 1024:
            raise ValueError("grammar exceeds the fixture byte bound")
        if hashlib.sha256(target.read_bytes()).hexdigest() != entry["sha256"]:
            raise ValueError("stale grammar digest")

    canonical = {
        path.relative_to(root).as_posix()
        for family in (*FAMILIES, "ecmascript")
        for kind in KINDS
        for path in (root / "code/grammars" / family).glob(f"*.{kind}")
    }
    if seen != canonical:
        raise ValueError("grammar roster omits or adds a canonical file")
    if any(kinds != set(KINDS) for kinds in versions.values()):
        raise ValueError("grammar version lacks token/parser pair")
    return {
        "families": len({family for family, _ in versions}),
        "grammar_files": len(entries),
        "versions": len(versions),
    }


def _validate_ast(
    node: Any, token_count: int, depth: int, budget: list[int], references: list[int]
) -> None:
    if depth > MAX_AST_DEPTH:
        raise ValueError("AST depth limit")
    budget[0] += 1
    if budget[0] > MAX_AST_ENTRIES:
        raise ValueError("AST entry limit")
    if not isinstance(node, dict):
        raise TypeError("AST entry shape")
    if set(node) == {"token"}:
        index = node["token"]
        if type(index) is not int or not 0 <= index < token_count - 1:
            raise ValueError("AST token reference")
        references.append(index)
    elif set(node) == {"rule", "children"}:
        if not isinstance(node["rule"], str) or not TOKEN_NAME.fullmatch(node["rule"]):
            raise ValueError("AST rule name")
        if not isinstance(node["children"], list) or len(node["children"]) > 512:
            raise ValueError("AST child bound")
        for child in node["children"]:
            _validate_ast(child, token_count, depth + 1, budget, references)
    else:
        raise ValueError("AST node shape")


def validate_cases(
    manifest: dict[str, Any], document: dict[str, Any]
) -> dict[str, int]:
    """Reject optimistic or incomplete case projections before native ports."""
    _closed(document, {"schema_version", "kind", "cases"}, "case corpus")
    if document["schema_version"] != 1 or document["kind"] != "cases":
        raise ValueError("unsupported case corpus")
    cases = document["cases"]
    if not isinstance(cases, list) or not 12 <= len(cases) <= MAX_CASES:
        raise ValueError("case roster size")
    versions: dict[tuple[str, str], set[str]] = {}
    for entry in manifest["grammars"]:
        versions.setdefault((entry["family"], entry["version"]), set()).add(
            entry["kind"]
        )
    valid_versions = {key for key, kinds in versions.items() if kinds == set(KINDS)}
    ids: set[str] = set()
    successes: set[str] = set()
    for case in cases:
        if not isinstance(case, dict) or case.get("outcome") != "ok":
            raise ValueError("v1 case must be a successful projection")
        _closed(
            case,
            {"id", "family", "version", "source", "outcome", "tokens", "ast"},
            "case",
        )
        if (
            not isinstance(case["id"], str)
            or len(case["id"]) > 120
            or not IDENTIFIER.fullmatch(case["id"])
            or case["id"] in ids
        ):
            raise ValueError("duplicate or invalid case ID")
        ids.add(case["id"])
        family = case["family"]
        if (family, case["version"]) not in valid_versions:
            raise ValueError("unknown grammar edition")
        source = case["source"]
        if (
            not isinstance(source, str)
            or len(source) > 4096
            or not source.isascii()
            or "\x00" in source
        ):
            raise ValueError("source byte bound or encoding")
        tokens = case["tokens"]
        if not isinstance(tokens, list) or not 1 <= len(tokens) <= 4096:
            raise ValueError("token roster size")
        for index, token in enumerate(tokens):
            _closed(token, {"type", "value", "line", "column"}, "token")
            if not isinstance(token["type"], str) or not TOKEN_NAME.fullmatch(
                token["type"]
            ):
                raise ValueError("token type")
            if not isinstance(token["value"], str) or len(token["value"]) > 4096:
                raise ValueError("token value")
            if (
                type(token["line"]) is not int
                or type(token["column"]) is not int
                or not 1 <= token["line"] <= 4096
                or not 1 <= token["column"] <= 4096
            ):
                raise ValueError("token position")
            if (token["type"] == "EOF") != (index == len(tokens) - 1):
                raise ValueError("EOF must be last and unique")
        references: list[int] = []
        _validate_ast(case["ast"], len(tokens), 0, [0], references)
        if references != list(range(len(tokens) - 1)):
            raise ValueError("AST must cover non-EOF tokens exactly once in order")
        successes.add(family)
    if successes != set(FAMILIES):
        raise ValueError("missing successful family projection")
    return {"families": len(successes), "cases": len(cases)}


def _read_json_bounded(path: Path, maximum: int) -> dict[str, Any]:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > maximum:
        raise ValueError(
            f"fixture missing, linked, or exceeds {maximum} bytes: {path.name}"
        )
    raw = path.read_bytes()
    if len(raw) > maximum:
        raise ValueError(f"fixture exceeds {maximum} bytes: {path.name}")
    document = json.loads(raw.decode("utf-8"))
    if not isinstance(document, dict):
        raise TypeError(f"fixture must be an object: {path.name}")
    return document


def validate_corpus(root: Path = REPO, fixtures: Path = FIXTURES) -> dict[str, int]:
    schema = _read_json_bounded(fixtures / "schema.json", MAX_SCHEMA_BYTES)
    manifest = _read_json_bounded(fixtures / "grammars.json", MAX_MANIFEST_BYTES)
    cases = _read_json_bounded(fixtures / "cases.json", MAX_CASE_BYTES)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    for document in (manifest, cases):
        validator.validate(document)
    return {**validate_grammars(root, manifest), **validate_cases(manifest, cases)}


if __name__ == "__main__":
    print(json.dumps(validate_corpus(), sort_keys=True))
