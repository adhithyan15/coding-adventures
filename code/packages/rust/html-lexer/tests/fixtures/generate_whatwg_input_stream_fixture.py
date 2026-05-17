#!/usr/bin/env python3

"""Generate Venture's WHATWG input-stream preprocessing fixture.

The HTML input stream normalizes CRLF and bare CR to LF before tokenization.
This fixture keeps that invariant explicit across tokenizer contexts where a
newline can appear in text, markup, attributes, comments, doctypes, raw text,
RCDATA, script data, and seeded continuation states.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-input-stream.json")

NEWLINE_FORMS = {
    "lf": "\n",
    "cr": "\r",
    "crlf": "\r\n",
    "mixed": "\r\n\r\n\r",
}

TEMPLATES = [
    {
        "id": "data-text",
        "description": "data state text",
        "input": "alpha{nl}beta",
        "normalized": "alpha{normalized}beta",
    },
    {
        "id": "tag-name-boundary",
        "description": "tag name whitespace boundary",
        "input": "<p{nl}class=x>alpha</p>",
        "normalized": "<p{normalized}class=x>alpha</p>",
    },
    {
        "id": "before-attribute-name",
        "description": "before attribute name state",
        "input": "<p class=x{nl}title=y>alpha</p>",
        "normalized": "<p class=x{normalized}title=y>alpha</p>",
    },
    {
        "id": "double-quoted-attribute",
        "description": "double quoted attribute value",
        "input": "<p title=\"alpha{nl}beta\">x</p>",
        "normalized": "<p title=\"alpha{normalized}beta\">x</p>",
    },
    {
        "id": "single-quoted-attribute",
        "description": "single quoted attribute value",
        "input": "<p title='alpha{nl}beta'>x</p>",
        "normalized": "<p title='alpha{normalized}beta'>x</p>",
    },
    {
        "id": "unquoted-attribute",
        "description": "unquoted attribute whitespace boundary",
        "input": "<p title=alpha{nl}beta=gamma>x</p>",
        "normalized": "<p title=alpha{normalized}beta=gamma>x</p>",
    },
    {
        "id": "comment-data",
        "description": "comment data",
        "input": "<!--alpha{nl}beta-->",
        "normalized": "<!--alpha{normalized}beta-->",
    },
    {
        "id": "bogus-comment-data",
        "description": "bogus comment data",
        "input": "<?alpha{nl}beta>",
        "normalized": "<?alpha{normalized}beta>",
    },
    {
        "id": "doctype-before-name",
        "description": "DOCTYPE whitespace before name",
        "input": "<!DOCTYPE{nl}html>",
        "normalized": "<!DOCTYPE{normalized}html>",
    },
    {
        "id": "doctype-public-identifier",
        "description": "DOCTYPE public identifier",
        "input": '<!DOCTYPE html PUBLIC "alpha{nl}beta" "system">',
        "normalized": '<!DOCTYPE html PUBLIC "alpha{normalized}beta" "system">',
    },
    {
        "id": "doctype-system-identifier",
        "description": "DOCTYPE system identifier",
        "input": '<!DOCTYPE html SYSTEM "alpha{nl}beta">',
        "normalized": '<!DOCTYPE html SYSTEM "alpha{normalized}beta">',
    },
    {
        "id": "named-reference-recovery",
        "description": "named character reference recovery before newline",
        "input": "alpha&not{nl}beta",
        "normalized": "alpha&not{normalized}beta",
    },
    {
        "id": "numeric-reference-recovery",
        "description": "numeric character reference missing semicolon before newline",
        "input": "alpha&#65{nl}beta",
        "normalized": "alpha&#65{normalized}beta",
    },
    {
        "id": "rcdata-text",
        "description": "RCDATA text",
        "input": "alpha{nl}beta</title>",
        "normalized": "alpha{normalized}beta</title>",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
    },
    {
        "id": "rcdata-end-tag-name",
        "description": "RCDATA end tag name recovery",
        "input": "</tit{nl}le>",
        "normalized": "</tit{normalized}le>",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
    },
    {
        "id": "rawtext-text",
        "description": "RAWTEXT text",
        "input": "alpha{nl}beta</style>",
        "normalized": "alpha{normalized}beta</style>",
        "initial_state": "RAWTEXT state",
        "last_start_tag": "style",
    },
    {
        "id": "rawtext-end-tag-name",
        "description": "RAWTEXT end tag name recovery",
        "input": "</sty{nl}le>",
        "normalized": "</sty{normalized}le>",
        "initial_state": "RAWTEXT state",
        "last_start_tag": "style",
    },
    {
        "id": "script-data-text",
        "description": "script data text",
        "input": "alpha{nl}beta</script>",
        "normalized": "alpha{normalized}beta</script>",
        "initial_state": "Script data state",
        "last_start_tag": "script",
    },
    {
        "id": "script-data-escaped",
        "description": "script data escaped text",
        "input": "alpha{nl}beta--></script>",
        "normalized": "alpha{normalized}beta--></script>",
        "initial_state": "Script data escaped state",
        "last_start_tag": "script",
    },
    {
        "id": "plaintext-text",
        "description": "PLAINTEXT text",
        "input": "alpha{nl}<beta>",
        "normalized": "alpha{normalized}<beta>",
        "initial_state": "PLAINTEXT state",
    },
    {
        "id": "cdata-section",
        "description": "CDATA section text",
        "input": "alpha{nl}beta]]>",
        "normalized": "alpha{normalized}beta]]>",
        "initial_state": "CDATA section state",
    },
    {
        "id": "seeded-comment-state",
        "description": "seeded comment continuation",
        "input": "alpha{nl}beta-->",
        "normalized": "alpha{normalized}beta-->",
        "initial_state": "Comment state",
        "current_comment": "seed:",
    },
    {
        "id": "seeded-doctype-public-identifier",
        "description": "seeded DOCTYPE public identifier continuation",
        "input": 'alpha{nl}beta">',
        "normalized": 'alpha{normalized}beta">',
        "initial_state": "DOCTYPE public identifier double quoted state",
        "current_doctype": {
            "name": "html",
            "public_identifier": "",
            "system_identifier": None,
            "force_quirks": False,
        },
    },
]

POSITION_TEMPLATES = [
    {
        "id": "data-null-after-newline",
        "description": "unexpected NULL after preprocessed newline in data",
        "input": "alpha{nl}\u0000",
        "initial_state": "Data state",
        "diagnostic": "unexpected-null-character",
    },
    {
        "id": "attribute-null-after-newline",
        "description": "unexpected NULL after preprocessed newline in attribute value",
        "input": "<p title='alpha{nl}\u0000'>",
        "initial_state": "Data state",
        "diagnostic": "unexpected-null-character",
    },
    {
        "id": "comment-null-after-newline",
        "description": "unexpected NULL after preprocessed newline in comment",
        "input": "<!--alpha{nl}\u0000-->",
        "initial_state": "Data state",
        "diagnostic": "unexpected-null-character",
    },
    {
        "id": "rcdata-null-after-newline",
        "description": "unexpected NULL after preprocessed newline in RCDATA",
        "input": "alpha{nl}\u0000</title>",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
        "diagnostic": "unexpected-null-character",
    },
]


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    text = json.dumps(fixture, indent=2, ensure_ascii=False, sort_keys=True) + "\n"

    if args.check:
        existing = output.read_text()
        if existing != text:
            raise SystemExit(f"{output} is stale; regenerate it")
        return 0

    output.write_text(text)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG input-stream preprocessing fixture JSON."
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
        "format": "whatwg-html-input-stream-preprocessing/v1",
        "description": (
            "CRLF and bare CR preprocessing equivalence cases for HTML "
            "tokenization contexts."
        ),
        "newline_forms": [
            {"id": form_id, "source": source, "normalized": normalize_newlines(source)}
            for form_id, source in NEWLINE_FORMS.items()
        ],
        "cases": [
            materialize_case(template, form_id, source)
            for template in TEMPLATES
            for form_id, source in NEWLINE_FORMS.items()
        ],
        "position_cases": [
            materialize_position_case(template, form_id, source)
            for template in POSITION_TEMPLATES
            for form_id, source in NEWLINE_FORMS.items()
        ],
    }


def materialize_case(
    template: dict[str, object], newline_id: str, newline_source: str
) -> dict[str, object]:
    normalized_newline = normalize_newlines(newline_source)
    case = {
        "id": f"{template['id']}-{newline_id}",
        "description": f"{template['description']} with {newline_id}",
        "input": str(template["input"]).format(nl=newline_source),
        "normalized": str(template["normalized"]).format(normalized=normalized_newline),
    }
    copy_context(template, case)
    return case


def materialize_position_case(
    template: dict[str, object], newline_id: str, newline_source: str
) -> dict[str, object]:
    before_newline = str(template["input"]).split("{nl}", 1)[0]
    case = {
        "id": f"{template['id']}-{newline_id}",
        "description": f"{template['description']} with {newline_id}",
        "input": str(template["input"]).format(nl=newline_source),
        "diagnostic": template["diagnostic"],
        "expected_line": before_newline.count("\n") + normalize_newlines(newline_source).count("\n") + 1,
        "expected_column": 1,
    }
    copy_context(template, case)
    return case


def copy_context(source: dict[str, object], target: dict[str, object]) -> None:
    for key in [
        "initial_state",
        "last_start_tag",
        "current_comment",
        "current_doctype",
    ]:
        if key in source:
            target[key] = source[key]


def normalize_newlines(value: str) -> str:
    return value.replace("\r\n", "\n").replace("\r", "\n")


if __name__ == "__main__":
    raise SystemExit(main())
