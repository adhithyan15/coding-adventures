#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer seeded attribute boundary fixture."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-attribute-boundaries.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    text = json.dumps(build_fixture(), indent=2, ensure_ascii=False, sort_keys=True) + "\n"

    if args.check:
        if output.read_text() != text:
            raise SystemExit(f"{output} is stale; regenerate it")
        return 0

    output.write_text(text)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer seeded attribute boundary fixture JSON."
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="Fixture path to write; defaults beside this script.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit non-zero if the checked-in fixture differs from generated output.",
    )
    return parser.parse_args()


def build_fixture() -> dict[str, object]:
    return {
        "format": "whatwg-html-tokenizer-attribute-boundaries/v1",
        "description": (
            "Seeded start-tag attribute continuation states for before/name/"
            "value/quoted/unquoted/self-closing boundaries."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    return [
        case(
            "before-attribute-name-commits-new-attribute",
            "before_attribute_name",
            " foo=bar>",
            ["StartTag(name=a, attributes=[id=root, foo=bar], self_closing=false)", "EOF"],
            seed=seed("a", attributes=[attr("id", "root")]),
        ),
        case(
            "before-attribute-name-self-closing",
            "before_attribute_name",
            "/>",
            ["StartTag(name=img, attributes=[alt=x], self_closing=true)", "EOF"],
            seed=seed("img", attributes=[attr("alt", "x")]),
        ),
        case(
            "before-attribute-name-eof-drops-partial-tag",
            "before_attribute_name",
            "",
            ["EOF"],
            seed=seed("a", attributes=[attr("href", "x")]),
            diagnostics=["eof-in-tag"],
        ),
        case(
            "attribute-name-finishes-on-equals",
            "attribute_name",
            "=bar>",
            ["StartTag(name=a, attributes=[data=bar], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("data", "")),
        ),
        case(
            "attribute-name-whitespace-commits-boolean",
            "attribute_name",
            " next=2>",
            ["StartTag(name=a, attributes=[checked=, next=2], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("checked", "")),
        ),
        case(
            "attribute-name-null-replacement",
            "attribute_name",
            "\u0000=value>",
            ["StartTag(name=a, attributes=[data�=value], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("data", "")),
            diagnostics=["unexpected-null-character"],
        ),
        case(
            "after-attribute-name-equals-value",
            "after_attribute_name",
            "=bar>",
            ["StartTag(name=a, attributes=[foo=bar], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("foo", "")),
        ),
        case(
            "after-attribute-name-next-attribute",
            "after_attribute_name",
            " bar=2>",
            ["StartTag(name=a, attributes=[foo=, bar=2], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("foo", "")),
        ),
        case(
            "before-attribute-value-double-quoted",
            "before_attribute_value",
            "\"hello\">",
            ["StartTag(name=a, attributes=[title=hello], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("title", "")),
        ),
        case(
            "before-attribute-value-single-quoted",
            "before_attribute_value",
            "'hello'>",
            ["StartTag(name=a, attributes=[title=hello], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("title", "")),
        ),
        case(
            "before-attribute-value-unquoted",
            "before_attribute_value",
            "hello>",
            ["StartTag(name=a, attributes=[title=hello], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("title", "")),
        ),
        case(
            "attribute-value-double-quoted-charref",
            "attribute_value_double_quoted",
            "&amp;B\">",
            ["StartTag(name=a, attributes=[title=A&B], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("title", "A")),
        ),
        case(
            "attribute-value-single-quoted-null",
            "attribute_value_single_quoted",
            "\u0000B'>",
            ["StartTag(name=a, attributes=[title=A�B], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("title", "A")),
            diagnostics=["unexpected-null-character"],
        ),
        case(
            "attribute-value-unquoted-space-delimiter",
            "attribute_value_unquoted",
            "B next=2>",
            ["StartTag(name=a, attributes=[title=AB, next=2], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("title", "A")),
        ),
        case(
            "attribute-value-unquoted-unexpected-equals",
            "attribute_value_unquoted",
            "=B>",
            ["StartTag(name=a, attributes=[title=A=B], self_closing=false)", "EOF"],
            seed=seed("a", current_attribute=attr("title", "A")),
            diagnostics=["unexpected-character-in-unquoted-attribute-value"],
        ),
        case(
            "after-attribute-value-quoted-next-attribute",
            "after_attribute_value_quoted",
            " next=2>",
            ["StartTag(name=a, attributes=[title=A, next=2], self_closing=false)", "EOF"],
            seed=seed("a", attributes=[attr("title", "A")]),
        ),
        case(
            "after-attribute-value-quoted-missing-whitespace",
            "after_attribute_value_quoted",
            "next=2>",
            ["StartTag(name=a, attributes=[title=A, next=2], self_closing=false)", "EOF"],
            seed=seed("a", attributes=[attr("title", "A")]),
            diagnostics=["missing-whitespace-between-attributes"],
        ),
        case(
            "self-closing-start-tag-emits",
            "self_closing_start_tag",
            ">",
            ["StartTag(name=br, attributes=[], self_closing=true)", "EOF"],
            seed=seed("br", self_closing=True),
        ),
        case(
            "self-closing-start-tag-reconsumes-attribute",
            "self_closing_start_tag",
            "x=1>",
            ["StartTag(name=br, attributes=[x=1], self_closing=false)", "EOF"],
            seed=seed("br"),
            diagnostics=["unexpected-solidus-in-tag"],
        ),
    ]


def case(
    case_id: str,
    initial_state: str,
    input_text: str,
    tokens: list[str],
    *,
    seed: dict[str, object],
    diagnostics: list[str] | None = None,
) -> dict[str, object]:
    item: dict[str, object] = {
        "id": case_id,
        "description": f"{initial_state} seeded attribute boundary case `{case_id}`",
        "input": input_text,
        "initial_state": initial_state,
        "start_tag": seed,
        "tokens": tokens,
    }
    if diagnostics is not None:
        item["diagnostics"] = diagnostics
    return item


def seed(
    name: str,
    *,
    attributes: list[dict[str, str]] | None = None,
    current_attribute: dict[str, str] | None = None,
    self_closing: bool = False,
) -> dict[str, object]:
    item: dict[str, object] = {
        "name": name,
        "attributes": attributes or [],
        "self_closing": self_closing,
    }
    if current_attribute is not None:
        item["current_attribute"] = current_attribute
    return item


def attr(name: str, value: str) -> dict[str, str]:
    return {"name": name, "value": value}


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
