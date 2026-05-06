#!/usr/bin/env python3

"""Normalize html5lib-style tokenizer fixtures into Venture's lexer schema."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SUPPORTED_INITIAL_STATES = {
    "CDATA section bracket state",
    "CDATA section end state",
    "CDATA section state",
    "Bogus comment state",
    "Comment end bang state",
    "Comment end dash state",
    "Comment end state",
    "Comment less-than sign bang dash dash state",
    "Comment less-than sign bang dash state",
    "Comment less-than sign bang state",
    "Comment less-than sign state",
    "Comment start dash state",
    "Comment start state",
    "Comment state",
    "Data state",
    "DOCTYPE after keyword state",
    "DOCTYPE keyword C state",
    "DOCTYPE keyword E state",
    "DOCTYPE keyword O state",
    "DOCTYPE keyword P state",
    "DOCTYPE keyword T state",
    "DOCTYPE keyword Y state",
    "DOCTYPE name state",
    "DOCTYPE public identifier double quoted state",
    "DOCTYPE public identifier single quoted state",
    "DOCTYPE public keyword B state",
    "DOCTYPE public keyword C state",
    "DOCTYPE public keyword I state",
    "DOCTYPE public keyword L state",
    "DOCTYPE public keyword U state",
    "DOCTYPE system identifier double quoted state",
    "DOCTYPE system identifier single quoted state",
    "DOCTYPE system keyword E state",
    "DOCTYPE system keyword M state",
    "DOCTYPE system keyword S state",
    "DOCTYPE system keyword T state",
    "DOCTYPE system keyword Y state",
    "After DOCTYPE name state",
    "After DOCTYPE public identifier state",
    "After DOCTYPE public keyword state",
    "After DOCTYPE system identifier state",
    "After DOCTYPE system keyword state",
    "Before DOCTYPE name state",
    "Before DOCTYPE public identifier state",
    "Before DOCTYPE system identifier state",
    "Between DOCTYPE public and system identifiers state",
    "Bogus DOCTYPE state",
    "PLAINTEXT state",
    "RCDATA end tag attributes state",
    "RCDATA end tag name state",
    "RCDATA end tag open state",
    "RCDATA end tag whitespace state",
    "RCDATA less-than sign state",
    "RCDATA self-closing end tag state",
    "RCDATA state",
    "RAWTEXT end tag attributes state",
    "RAWTEXT end tag name state",
    "RAWTEXT end tag open state",
    "RAWTEXT end tag whitespace state",
    "RAWTEXT less-than sign state",
    "RAWTEXT self-closing end tag state",
    "RAWTEXT state",
    "Script data double escape end state",
    "Script data double escape start state",
    "Script data double escaped dash dash state",
    "Script data double escaped dash state",
    "Script data double escaped less-than sign state",
    "Script data double escaped state",
    "Script data end tag attributes state",
    "Script data end tag name state",
    "Script data end tag open state",
    "Script data end tag whitespace state",
    "Script data escape start dash state",
    "Script data escape start state",
    "Script data escaped dash dash state",
    "Script data escaped dash state",
    "Script data escaped end tag attributes state",
    "Script data escaped end tag name state",
    "Script data escaped end tag open state",
    "Script data escaped end tag whitespace state",
    "Script data escaped less-than sign state",
    "Script data escaped self-closing end tag state",
    "Script data escaped state",
    "Script data less-than sign state",
    "Script data self-closing end tag state",
    "Script data state",
}

LAST_START_TAG_INITIAL_STATES = {
    "RCDATA end tag attributes state",
    "RCDATA end tag name state",
    "RCDATA end tag open state",
    "RCDATA end tag whitespace state",
    "RCDATA less-than sign state",
    "RCDATA self-closing end tag state",
    "RCDATA state",
    "RAWTEXT end tag attributes state",
    "RAWTEXT end tag name state",
    "RAWTEXT end tag open state",
    "RAWTEXT end tag whitespace state",
    "RAWTEXT less-than sign state",
    "RAWTEXT self-closing end tag state",
    "RAWTEXT state",
    "Script data double escape end state",
    "Script data double escape start state",
    "Script data double escaped dash dash state",
    "Script data double escaped dash state",
    "Script data double escaped less-than sign state",
    "Script data double escaped state",
    "Script data end tag attributes state",
    "Script data end tag name state",
    "Script data end tag open state",
    "Script data end tag whitespace state",
    "Script data escape start dash state",
    "Script data escape start state",
    "Script data escaped dash dash state",
    "Script data escaped dash state",
    "Script data escaped end tag attributes state",
    "Script data escaped end tag name state",
    "Script data escaped end tag open state",
    "Script data escaped end tag whitespace state",
    "Script data escaped less-than sign state",
    "Script data escaped self-closing end tag state",
    "Script data escaped state",
    "Script data less-than sign state",
    "Script data self-closing end tag state",
    "Script data state",
}

END_TAG_SEED_INITIAL_STATES = {
    "RCDATA end tag attributes state",
    "RCDATA end tag name state",
    "RCDATA end tag whitespace state",
    "RCDATA self-closing end tag state",
    "RAWTEXT end tag attributes state",
    "RAWTEXT end tag name state",
    "RAWTEXT end tag whitespace state",
    "RAWTEXT self-closing end tag state",
    "Script data end tag attributes state",
    "Script data end tag name state",
    "Script data end tag whitespace state",
    "Script data self-closing end tag state",
    "Script data escaped end tag attributes state",
    "Script data escaped end tag name state",
    "Script data escaped end tag whitespace state",
    "Script data escaped self-closing end tag state",
}

COMMENT_SEED_INITIAL_STATES = {
    "Bogus comment state",
    "Comment end bang state",
    "Comment end dash state",
    "Comment end state",
    "Comment less-than sign bang dash dash state",
    "Comment less-than sign bang dash state",
    "Comment less-than sign bang state",
    "Comment less-than sign state",
    "Comment start dash state",
    "Comment start state",
    "Comment state",
}

DOCTYPE_SEED_INITIAL_STATES = {
    "DOCTYPE after keyword state",
    "DOCTYPE keyword C state",
    "DOCTYPE keyword E state",
    "DOCTYPE keyword O state",
    "DOCTYPE keyword P state",
    "DOCTYPE keyword T state",
    "DOCTYPE keyword Y state",
    "DOCTYPE name state",
    "DOCTYPE public identifier double quoted state",
    "DOCTYPE public identifier single quoted state",
    "DOCTYPE public keyword B state",
    "DOCTYPE public keyword C state",
    "DOCTYPE public keyword I state",
    "DOCTYPE public keyword L state",
    "DOCTYPE public keyword U state",
    "DOCTYPE system identifier double quoted state",
    "DOCTYPE system identifier single quoted state",
    "DOCTYPE system keyword E state",
    "DOCTYPE system keyword M state",
    "DOCTYPE system keyword S state",
    "DOCTYPE system keyword T state",
    "DOCTYPE system keyword Y state",
    "After DOCTYPE name state",
    "After DOCTYPE public identifier state",
    "After DOCTYPE public keyword state",
    "After DOCTYPE system identifier state",
    "After DOCTYPE system keyword state",
    "Before DOCTYPE name state",
    "Before DOCTYPE public identifier state",
    "Before DOCTYPE system identifier state",
    "Between DOCTYPE public and system identifiers state",
    "Bogus DOCTYPE state",
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

    needs_end_tag_seed = [
        initial_state
        for initial_state in initial_states
        if initial_state in END_TAG_SEED_INITIAL_STATES
    ]
    has_end_tag_seed = isinstance(test.get("currentEndTag"), str) and isinstance(
        test.get("temporaryBuffer"), str
    )
    has_partial_end_tag_seed = test.get("currentEndTag") is not None or test.get(
        "temporaryBuffer"
    ) is not None
    if needs_end_tag_seed and not has_end_tag_seed:
        return False, f"{needs_end_tag_seed[0]} requires currentEndTag and temporaryBuffer"

    if has_partial_end_tag_seed and not has_end_tag_seed:
        return False, "currentEndTag and temporaryBuffer must be provided together"

    if has_end_tag_seed and len(needs_end_tag_seed) != len(initial_states):
        return False, "currentEndTag/temporaryBuffer only supported for continuation states"

    needs_comment_seed = [
        initial_state
        for initial_state in initial_states
        if initial_state in COMMENT_SEED_INITIAL_STATES
    ]
    has_comment_seed = isinstance(test.get("currentComment"), str)
    if needs_comment_seed and not has_comment_seed:
        return False, f"{needs_comment_seed[0]} requires currentComment"

    if has_comment_seed and len(needs_comment_seed) != len(initial_states):
        return False, "currentComment only supported for comment continuation states"

    if has_comment_seed and (has_end_tag_seed or has_partial_end_tag_seed):
        return False, "currentComment cannot be combined with end-tag continuation context"

    needs_doctype_seed = [
        initial_state
        for initial_state in initial_states
        if initial_state in DOCTYPE_SEED_INITIAL_STATES
    ]
    has_doctype_seed = isinstance(test.get("currentDoctype"), dict)
    if needs_doctype_seed and not has_doctype_seed:
        return False, f"{needs_doctype_seed[0]} requires currentDoctype"

    if has_doctype_seed and len(needs_doctype_seed) != len(initial_states):
        return False, "currentDoctype only supported for doctype continuation states"

    if has_doctype_seed and (has_end_tag_seed or has_partial_end_tag_seed or has_comment_seed):
        return False, "currentDoctype cannot be combined with other current-token context"

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

    current_end_tag = test.get("currentEndTag")
    if current_end_tag is not None:
        normalized["current_end_tag"] = current_end_tag

    temporary_buffer = test.get("temporaryBuffer")
    if temporary_buffer is not None:
        normalized["temporary_buffer"] = temporary_buffer

    current_comment = test.get("currentComment")
    if current_comment is not None:
        normalized["current_comment"] = current_comment

    current_doctype = test.get("currentDoctype")
    if current_doctype is not None:
        normalized["current_doctype"] = current_doctype

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
