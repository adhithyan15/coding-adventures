#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer CDATA boundary fixture."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-cdata-boundaries.json")


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
        description="Generate WHATWG tokenizer CDATA boundary fixture JSON."
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
        "format": "whatwg-html-tokenizer-cdata-boundaries/v1",
        "description": (
            "Foreign-content CDATA section text, delimiter, NULL, EOF, "
            "HTML-content bogus-comment recovery, and seeded bracket/end states."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    return [
        cdata("cdata-basic-delimiter", "a]]><p>x</p>", ["Text(data=a)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-markup-stays-text", "<b>&amp;</b>]]><p>x</p>", ["Text(data=<b>&amp;</b>)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-delimiter-at-start", "]]><p>x</p>", ["StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-extra-closing-bracket", "a]]]><p>x</p>", ["Text(data=a])", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-many-brackets-before-delimiter", "a]]]]><p>x</p>", ["Text(data=a]])", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-single-bracket-not-delimiter", "a]b]]><p>x</p>", ["Text(data=a]b)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-double-bracket-not-delimiter", "a]]b]]><p>x</p>", ["Text(data=a]]b)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-greater-than-alone", "a>b]]><p>x</p>", ["Text(data=a>b)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        cdata("cdata-null-replacement", "a\u0000b]]><p>x</p>", ["Text(data=a�b)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"], diagnostics=["unexpected-null-character"]),
        cdata("cdata-eof-body", "open", ["Text(data=open)", "EOF"]),
        cdata("cdata-eof-after-single-bracket", "open]", ["Text(data=open])", "EOF"]),
        cdata("cdata-eof-after-double-bracket", "open]]", ["Text(data=open]])", "EOF"]),
        data("html-cdata-looking-declaration", "a<![CDATA[x]]>b", ["Text(data=a)", "Comment(data=[CDATA[x]])", "Text(data=b)", "EOF"], diagnostics=["cdata-in-html-content"]),
        data("html-cdata-looking-null", "a<![CDATA[x\u0000]]>b", ["Text(data=a)", "Comment(data=[CDATA[x�]])", "Text(data=b)", "EOF"], diagnostics=["cdata-in-html-content", "unexpected-null-character"]),
        data("html-cdata-looking-eof", "a<![CDATA[x", ["Text(data=a)", "Comment(data=[CDATA[x)", "EOF"], diagnostics=["cdata-in-html-content"]),
        state("seeded-cdata-bracket-delimiter", "CDATA section bracket state", ">tail", ["Text(data=]>tail)", "EOF"]),
        state("seeded-cdata-bracket-extra-bracket", "CDATA section bracket state", "]><p>x</p>", ["StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        state("seeded-cdata-bracket-not-delimiter", "CDATA section bracket state", "x]]><p>x</p>", ["Text(data=]x)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        state("seeded-cdata-bracket-null", "CDATA section bracket state", "\u0000]]><p>x</p>", ["Text(data=]�)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"], diagnostics=["unexpected-null-character"]),
        state("seeded-cdata-bracket-eof", "CDATA section bracket state", "", ["Text(data=])", "EOF"]),
        state("seeded-cdata-end-delimiter", "CDATA section end state", ">tail", ["Text(data=tail)", "EOF"]),
        state("seeded-cdata-end-more-brackets", "CDATA section end state", "]]><p>x</p>", ["Text(data=]])", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        state("seeded-cdata-end-not-delimiter", "CDATA section end state", "x]]><p>x</p>", ["Text(data=]]x)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"]),
        state("seeded-cdata-end-null", "CDATA section end state", "\u0000]]><p>x</p>", ["Text(data=]]�)", "StartTag(name=p, attributes=[], self_closing=false)", "Text(data=x)", "EndTag(name=p)", "EOF"], diagnostics=["unexpected-null-character"]),
        state("seeded-cdata-end-eof", "CDATA section end state", "", ["Text(data=]])", "EOF"]),
    ]


def data(
    case_id: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
) -> dict[str, object]:
    return {
        "id": case_id,
        "description": f"HTML data-state CDATA-looking recovery case `{case_id}`",
        "input": input_text,
        "tokens": tokens,
        **({"diagnostics": diagnostics} if diagnostics is not None else {}),
    }


def cdata(
    case_id: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
) -> dict[str, object]:
    return state(
        case_id,
        "CDATA section state",
        input_text,
        tokens,
        diagnostics=diagnostics,
    )


def state(
    case_id: str,
    initial_state: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
) -> dict[str, object]:
    return {
        "id": case_id,
        "description": f"{initial_state} boundary case `{case_id}`",
        "input": input_text,
        "initial_state": initial_state,
        "tokens": tokens,
        **({"diagnostics": diagnostics} if diagnostics is not None else {}),
    }


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
