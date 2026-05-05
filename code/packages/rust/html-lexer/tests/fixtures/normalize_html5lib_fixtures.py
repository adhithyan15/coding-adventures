#!/usr/bin/env python3

"""Normalize html5lib-style tokenizer fixtures into Venture's lexer schema."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SUPPORTED_INITIAL_STATES = {
    "CDATA section state",
    "CDATA section bracket state",
    "CDATA section end state",
    "Data state",
    "PLAINTEXT state",
    "RCDATA end tag open state",
    "RCDATA state",
    "RCDATA less-than sign state",
    "RAWTEXT end tag open state",
    "RAWTEXT state",
    "RAWTEXT less-than sign state",
    "Script data double escape end state",
    "Script data double escape start state",
    "Script data double escaped dash dash state",
    "Script data double escaped dash state",
    "Script data double escaped less-than sign state",
    "Script data double escaped state",
    "Script data escape start dash state",
    "Script data escape start state",
    "Script data escaped dash dash state",
    "Script data escaped dash state",
    "Script data escaped end tag open state",
    "Script data escaped less-than sign state",
    "Script data escaped state",
    "Script data end tag open state",
    "Script data less-than sign state",
    "Script data state",
}

LAST_START_TAG_INITIAL_STATES = {
    "RCDATA state",
    "RCDATA less-than sign state",
    "RCDATA end tag open state",
    "RAWTEXT state",
    "RAWTEXT less-than sign state",
    "RAWTEXT end tag open state",
    "Script data double escape end state",
    "Script data double escape start state",
    "Script data double escaped dash dash state",
    "Script data double escaped dash state",
    "Script data double escaped less-than sign state",
    "Script data double escaped state",
    "Script data escape start dash state",
    "Script data escape start state",
    "Script data escaped dash dash state",
    "Script data escaped dash state",
    "Script data escaped end tag open state",
    "Script data escaped less-than sign state",
    "Script data escaped state",
    "Script data end tag open state",
    "Script data less-than sign state",
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

        initial_states = test.get("initialStates", [])
        if len(initial_states) <= 1:
            normalized_cases.append(
                normalize_case(index, test, initial_states[0] if initial_states else None)
            )
        else:
            for variant, initial_state in enumerate(initial_states, start=1):
                normalized_cases.append(normalize_case(index, test, initial_state, variant))

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
    if any(initial_state not in SUPPORTED_INITIAL_STATES for initial_state in initial_states):
        return False, f"unsupported initialStates={initial_states!r}"

    last_start_tag = test.get("lastStartTag")
    needs_last_start_tag = [
        initial_state
        for initial_state in initial_states
        if initial_state in LAST_START_TAG_INITIAL_STATES
    ]
    if needs_last_start_tag and not isinstance(last_start_tag, str):
        return False, f"{needs_last_start_tag[0]} requires lastStartTag"

    if last_start_tag is not None and len(needs_last_start_tag) != len(initial_states):
        return False, f"unsupported lastStartTag={last_start_tag!r}"

    for token in test.get("output", []):
        kind = token[0]
        if kind not in {"Character", "StartTag", "EndTag", "Comment", "DOCTYPE"}:
            return False, f"unsupported token kind `{kind}`"

    return True, ""


def normalize_case(
    index: int, test: dict[str, Any], initial_state: str | None, variant: int | None = None
) -> dict[str, Any]:
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
            name = "null" if token[1] is None else token[1]
            public_identifier = "null" if token[2] is None else token[2]
            system_identifier = "null" if token[3] is None else token[3]
            if token[2] is None and token[3] is None:
                tokens.append(f"Doctype(name={name}, force_quirks={str(force_quirks).lower()})")
            else:
                tokens.append(
                    "Doctype("
                    f"name={name}, "
                    f"public_identifier={public_identifier}, "
                    f"system_identifier={system_identifier}, "
                    f"force_quirks={str(force_quirks).lower()}"
                    ")"
                )

    if pending_text:
        tokens.append(f"Text(data={pending_text})")

    tokens.append("EOF")

    normalized = {
        "id": normalized_case_id(index, variant),
        "description": test.get("description", f"case {index}"),
        "input": test["input"],
        "tokens": tokens,
        "diagnostics": [error["code"] for error in test.get("errors", [])],
    }

    if initial_state is not None:
        normalized["initial_state"] = initial_state

    last_start_tag = test.get("lastStartTag")
    if last_start_tag is not None:
        normalized["last_start_tag"] = last_start_tag

    return normalized


def normalized_case_id(index: int, variant: int | None) -> str:
    if variant is None:
        return f"html5lib-smoke-{index}"
    return f"html5lib-smoke-{index}-{variant}"


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
