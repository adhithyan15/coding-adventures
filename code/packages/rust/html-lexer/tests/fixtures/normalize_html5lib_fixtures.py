#!/usr/bin/env python3

"""Normalize html5lib-style tokenizer fixtures into Venture's lexer schema."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SUPPORTED_INITIAL_STATES = {
    "Data state",
    "PLAINTEXT state",
    "RCDATA state",
    "RAWTEXT state",
    "Script data state",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Normalize html5lib-style tokenizer tests into Venture fixture JSON."
    )
    parser.add_argument("input", type=Path, help="Path to raw html5lib-style tokenizer fixture")
    parser.add_argument(
        "output", type=Path, help="Path to write normalized venture-html-lexer fixture JSON"
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    raw = json.loads(args.input.read_text())
    tests = raw.get("tests", [])

    normalized_cases: list[dict[str, Any]] = []
    skipped: list[dict[str, str]] = []

    for index, test in enumerate(tests, start=1):
        supported, reason = is_supported(test)
        if not supported:
            skipped.append(
                {
                    "id": f"html5lib-smoke-{index}",
                    "description": test.get("description", f"case {index}"),
                    "reason": reason,
                }
            )
            continue

        normalized_cases.append(normalize_case(index, test))

    normalized = {
        "format": "venture-html-lexer-fixtures/v1",
        "suite": "html5lib-smoke",
        "description": "Normalized html5lib-style tokenizer smoke cases lowered into the Venture fixture schema.",
        "source": args.input.name,
        "generator": "normalize_html5lib_fixtures.py",
        "supported_initial_states": sorted(SUPPORTED_INITIAL_STATES),
        "cases": normalized_cases,
        "skipped": skipped,
    }

    args.output.write_text(json.dumps(normalized, indent=2) + "\n")
    return 0


def is_supported(test: dict[str, Any]) -> tuple[bool, str]:
    initial_states = test.get("initialStates", [])
    if len(initial_states) > 1:
        return False, f"unsupported initialStates={initial_states!r}"

    if initial_states and initial_states[0] not in SUPPORTED_INITIAL_STATES:
        return False, f"unsupported initialStates={initial_states!r}"

    last_start_tag = test.get("lastStartTag")
    if initial_states in (
        ["RCDATA state"],
        ["RAWTEXT state"],
        ["Script data state"],
    ) and not isinstance(last_start_tag, str):
        return False, f"{initial_states[0]} requires lastStartTag"

    if (
        initial_states
        not in (["RCDATA state"], ["RAWTEXT state"], ["Script data state"])
        and last_start_tag is not None
    ):
        return False, f"unsupported lastStartTag={last_start_tag!r}"

    for token in test.get("output", []):
        kind = token[0]
        if kind not in {"Character", "StartTag", "EndTag", "Comment", "DOCTYPE"}:
            return False, f"unsupported token kind `{kind}`"

    return True, ""


def normalize_case(index: int, test: dict[str, Any]) -> dict[str, Any]:
    tokens: list[str] = []
    pending_text = ""

    for token in test["output"]:
        kind = token[0]
        if kind == "Character":
            pending_text += token[1]
            continue

        if pending_text:
            tokens.append(f"Text(data={pending_text})")
            pending_text = ""

        if kind == "StartTag":
            tokens.append(normalize_start_tag(token))
        elif kind == "EndTag":
            tokens.append(f"EndTag(name={token[1]})")
        elif kind == "Comment":
            tokens.append(f"Comment(data={token[1]})")
        elif kind == "DOCTYPE":
            force_quirks = not bool(token[4])
            tokens.append(f"Doctype(name={token[1]}, force_quirks={str(force_quirks).lower()})")

    if pending_text:
        tokens.append(f"Text(data={pending_text})")

    tokens.append("EOF")

    normalized = {
        "id": f"html5lib-smoke-{index}",
        "description": test.get("description", f"case {index}"),
        "input": test["input"],
        "tokens": tokens,
        "diagnostics": [error["code"] for error in test.get("errors", [])],
    }

    initial_states = test.get("initialStates", [])
    if initial_states:
        normalized["initial_state"] = initial_states[0]

    last_start_tag = test.get("lastStartTag")
    if last_start_tag is not None:
        normalized["last_start_tag"] = last_start_tag

    return normalized


def normalize_start_tag(token: list[Any]) -> str:
    name = token[1]
    attributes = token[2]
    self_closing = bool(token[3]) if len(token) > 3 else False

    if not attributes:
        attribute_summary = "[]"
    else:
        pairs = ", ".join(f"{key}={value}" for key, value in attributes.items())
        attribute_summary = f"[{pairs}]"

    return (
        f"StartTag(name={name}, attributes={attribute_summary}, "
        f"self_closing={str(self_closing).lower()})"
    )


if __name__ == "__main__":
    raise SystemExit(main())
