#!/usr/bin/env python3
"""Validate authored Warrant syntax expectations and project them to TFV.

This is a data/transport gate, not a second Warrant parser. In particular it
never runs a subject parser to manufacture an expected acceptance or subtree.
Those answers stay reviewable authored data, consumed independently by Rust.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

import tfv

ROOT = Path(__file__).resolve().parents[2]
JSON_PATH = ROOT / "code/specs/fixtures/warrant-syntax-v1/cases.json"
TFV_PATH = JSON_PATH.with_suffix(".tfv")
MAX_JSON_BYTES = 2 * 1024 * 1024
MAX_TFV_BYTES = 8 * 1024 * 1024
FEATURES = frozenset({
    "declarations", "values", "operators", "quantifiers", "contracts",
    "types", "lexical", "layout", "precedence",
})
_ID = re.compile(r"[A-Za-z0-9._/-]{1,128}")
_WITNESS = re.compile(r"[a-z][a-z0-9_]*=[^\x00-\x1f]+")
_FIELDS = {"id", "feature", "source", "accept", "witnesses"}


class CorpusError(ValueError):
    """A fixed, non-reproducing data/transport diagnostic."""


def _unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise CorpusError("duplicate JSON key")
        result[key] = value
    return result


def _nonfinite(_: str) -> None:
    raise CorpusError("nonfinite JSON number")


def loads_document(text: str) -> object:
    if not _text(text, MAX_JSON_BYTES):
        raise CorpusError("JSON text exceeds byte budget or has invalid scalar text")
    try:
        return json.loads(text, object_pairs_hook=_unique_object, parse_constant=_nonfinite)
    except (json.JSONDecodeError, RecursionError, ValueError) as error:
        raise CorpusError("invalid JSON document") from error


def _text(value: object, limit: int) -> bool:
    # Character count is a cheap lower bound on encoded size; cap it before
    # encoding so a huge in-memory string cannot cause an unbounded copy.
    if not isinstance(value, str) or not value or len(value) > limit:
        return False
    try:
        return len(value.encode("utf-8")) <= limit
    except UnicodeError:
        return False


def validate_document(document: object) -> list[str]:
    problems = []
    if type(document) is not dict or set(document) != {"format", "cases"}:
        return ["root fields must be format and cases"]
    if document["format"] != "warrant-syntax-v1":
        problems.append("wrong format")
    cases = document["cases"]
    if type(cases) is not list or not 1 <= len(cases) <= 128:
        return problems + ["cases must be a nonempty list of at most 128 cases"]
    ids = set()
    covered = set()
    total_payload = 0
    for index, case in enumerate(cases):
        where = f"case {index}"
        if type(case) is not dict or set(case) != _FIELDS:
            problems.append(f"{where}: case fields differ")
            continue
        identity = case["id"]
        if not isinstance(identity, str) or not _ID.fullmatch(identity):
            problems.append(f"{where}: invalid id")
        elif identity in ids:
            problems.append(f"{where}: duplicate id")
        else:
            ids.add(identity)
        feature, accepted = case["feature"], case["accept"]
        if not isinstance(feature, str) or feature not in FEATURES:
            problems.append(f"{where}: unknown feature")
        elif type(accepted) is bool:
            covered.add((feature, accepted))
        if type(accepted) is not bool:
            problems.append(f"{where}: accept must be Bool")
        if not _text(case["source"], 4096):
            problems.append(f"{where}: source must be scalar UTF-8 text of 1..4096 bytes")
        else:
            total_payload += len(case["source"].encode("utf-8"))
        witnesses = case["witnesses"]
        if type(witnesses) is not list or len(witnesses) > 32:
            problems.append(f"{where}: witnesses must be a list of at most 32 entries")
            continue
        if accepted is True and not witnesses:
            problems.append(f"{where}: accepted case needs witnesses")
        if accepted is False and witnesses:
            problems.append(f"{where}: rejected case cannot have witnesses")
        seen = set()
        for witness in witnesses:
            if not _text(witness, 8192) or not _WITNESS.fullmatch(witness):
                problems.append(f"{where}: invalid witness")
            elif witness in seen:
                problems.append(f"{where}: duplicate witness")
            else:
                seen.add(witness)
                total_payload += len(witness.encode("utf-8"))
        if total_payload > MAX_JSON_BYTES:
            return problems + ["document payload exceeds byte budget"]
    if covered != {(feature, result) for feature in FEATURES for result in (False, True)}:
        problems.append("feature baseline coverage needs positive and negative cases for every feature")
    return problems


def generate_tfv(document: object) -> str:
    problems = validate_document(document)
    if problems:
        raise CorpusError("; ".join(problems))
    records = []
    for case in document["cases"]:
        records.append((case["id"], [
            ("source", ("text", case["source"])),
            ("accept", ("bool", case["accept"])),
            ("witnesses", ("list", [("text", s) for s in case["witnesses"]])),
        ]))
    return tfv.encode_file(records)


def _read_limited(path: Path, limit: int) -> bytes:
    with path.open("rb") as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise CorpusError("file exceeds byte budget")
    return data


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="write canonical LF TFV bytes")
    options = parser.parse_args(argv)
    try:
        document = loads_document(_read_limited(JSON_PATH, MAX_JSON_BYTES).decode("utf-8"))
        expected = generate_tfv(document).encode("utf-8")
        if options.write:
            TFV_PATH.write_bytes(expected)
        if _read_limited(TFV_PATH, MAX_TFV_BYTES) != expected:
            raise CorpusError("TFV bytes drifted; regenerate with --write")
    except (OSError, UnicodeError, CorpusError) as error:
        # File paths and parser excerpts may contain reproducing material.
        # Shape/transport diagnostics above are fixed vocabulary; OS/decoder
        # errors become a fixed class rather than their raw exception text.
        message = str(error) if isinstance(error, CorpusError) else "file/encoding error"
        print(f"warrant-syntax-v1: {message}")
        return 1
    print(f"warrant-syntax-v1: {len(document['cases'])} authored cases; TFV bytes current")
    return 0


if __name__ == "__main__":
    sys.exit(main())
