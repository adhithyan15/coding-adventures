#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer text-mode delimiter fixture.

RCDATA, RAWTEXT, and script-data tokenization all use parser-provided
last-start-tag context to decide whether an apparent end tag is a real delimiter
or literal text. This fixture pins the observable delimiter matrix across
matching tags, mismatches, whitespace, attributes, self-closing recovery, and
seeded end-tag continuation states.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-text-mode-delimiters.json")

TEXT_MODE_CONTEXTS = [
    {
        "mode": "rcdata",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
        "entity_prefix": "Tom &amp; ",
        "entity_text": "Tom & ",
        "mismatch_tag": "style",
    },
    {
        "mode": "rawtext",
        "initial_state": "RAWTEXT state",
        "last_start_tag": "style",
        "entity_prefix": "Tom &amp; ",
        "entity_text": "Tom &amp; ",
        "mismatch_tag": "title",
    },
    {
        "mode": "script-data",
        "initial_state": "Script data state",
        "last_start_tag": "script",
        "entity_prefix": "Tom &amp; ",
        "entity_text": "Tom &amp; ",
        "mismatch_tag": "style",
    },
    {
        "mode": "script-data-escaped",
        "initial_state": "Script data escaped state",
        "last_start_tag": "script",
        "entity_prefix": "Tom &amp; ",
        "entity_text": "Tom &amp; ",
        "mismatch_tag": "style",
        "script_like_eof": True,
    },
]

SEEDED_END_TAG_STATES = [
    {
        "id": "rcdata-name",
        "initial_state": "RCDATA end tag name state",
        "last_start_tag": "title",
        "current_end_tag": "title",
        "temporary_buffer": "title",
    },
    {
        "id": "rawtext-name",
        "initial_state": "RAWTEXT end tag name state",
        "last_start_tag": "style",
        "current_end_tag": "style",
        "temporary_buffer": "style",
    },
    {
        "id": "script-name",
        "initial_state": "Script data end tag name state",
        "last_start_tag": "script",
        "current_end_tag": "script",
        "temporary_buffer": "script",
    },
    {
        "id": "script-escaped-name",
        "initial_state": "Script data escaped end tag name state",
        "last_start_tag": "script",
        "current_end_tag": "script",
        "temporary_buffer": "script",
        "script_like_eof": True,
    },
]

RECOVERY_STATES = [
    {
        "id": "rcdata-whitespace",
        "initial_state": "RCDATA end tag whitespace state",
        "last_start_tag": "title",
        "current_end_tag": "title",
        "temporary_buffer": "title ",
        "diagnostic": "unexpected-whitespace-after-end-tag-name",
    },
    {
        "id": "rawtext-attributes",
        "initial_state": "RAWTEXT end tag attributes state",
        "last_start_tag": "style",
        "current_end_tag": "style",
        "temporary_buffer": "style class=x",
        "diagnostic": "end-tag-with-attributes",
    },
    {
        "id": "script-self-closing",
        "initial_state": "Script data self-closing end tag state",
        "last_start_tag": "script",
        "current_end_tag": "script",
        "temporary_buffer": "script",
        "diagnostic": "end-tag-with-trailing-solidus",
    },
    {
        "id": "script-escaped-attributes",
        "initial_state": "Script data escaped end tag attributes state",
        "last_start_tag": "script",
        "current_end_tag": "script",
        "temporary_buffer": "script class=x",
        "diagnostic": "end-tag-with-attributes",
        "script_like_eof": True,
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
        description="Generate WHATWG tokenizer text-mode delimiter fixture JSON."
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
        "format": "whatwg-html-tokenizer-text-mode-delimiters/v1",
        "description": (
            "Text-mode end-tag delimiter recovery for parser-controlled "
            "RCDATA, RAWTEXT, script data, and seeded continuation states."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    for context in TEXT_MODE_CONTEXTS:
        cases.extend(text_mode_cases(context))
    cases.extend(seeded_end_tag_cases())
    cases.extend(script_escape_cases())
    return cases


def text_mode_cases(context: dict[str, object]) -> list[dict[str, object]]:
    mode = str(context["mode"])
    state = str(context["initial_state"])
    tag = str(context["last_start_tag"])
    mismatch = str(context["mismatch_tag"])
    entity_prefix = str(context["entity_prefix"])
    entity_text = str(context["entity_text"])
    script_like_eof = bool(context.get("script_like_eof", False))

    base = {
        "initial_state": state,
        "last_start_tag": tag,
    }
    cases = [
        {
            **base,
            "id": f"{mode}-matching-end-tag",
            "description": f"{state} recognizes its matching end tag delimiter",
            "input": f"{entity_prefix}</{tag}>after",
            "tokens": [
                f"Text(data={entity_text})",
                f"EndTag(name={tag})",
                "Text(data=after)",
                "EOF",
            ],
        },
        {
            **base,
            "id": f"{mode}-uppercase-matching-end-tag",
            "description": f"{state} matches end tag names ASCII-case-insensitively",
            "input": f"alpha</{tag.upper()}>after",
            "tokens": [
                "Text(data=alpha)",
                f"EndTag(name={tag})",
                "Text(data=after)",
                "EOF",
            ],
        },
        {
            **base,
            "id": f"{mode}-mismatched-end-tag",
            "description": f"{state} keeps mismatched apparent end tags literal",
            "input": f"alpha</{mismatch}>tail</{tag}>",
            "tokens": [
                f"Text(data=alpha</{mismatch}>tail)",
                f"EndTag(name={tag})",
                "EOF",
            ],
        },
        {
            **base,
            "id": f"{mode}-longer-prefix-mismatch",
            "description": f"{state} requires a delimiter after the matching tag name",
            "input": f"alpha</{tag}x>tail</{tag}>",
            "tokens": [
                f"Text(data=alpha</{tag}x>tail)",
                f"EndTag(name={tag})",
                "EOF",
            ],
        },
        {
            **base,
            "id": f"{mode}-whitespace-after-end-tag-name",
            "description": f"{state} accepts whitespace after a matching end tag name with a diagnostic",
            "input": f"alpha</{tag} >tail",
            "tokens": ["Text(data=alpha)", f"EndTag(name={tag})", "Text(data=tail)", "EOF"],
            "diagnostics": ["unexpected-whitespace-after-end-tag-name"],
        },
        {
            **base,
            "id": f"{mode}-form-feed-after-end-tag-name",
            "description": f"{state} treats form feed as whitespace after a matching end tag name",
            "input": f"alpha</{tag}\f>tail",
            "tokens": ["Text(data=alpha)", f"EndTag(name={tag})", "Text(data=tail)", "EOF"],
            "diagnostics": ["unexpected-whitespace-after-end-tag-name"],
        },
        {
            **base,
            "id": f"{mode}-end-tag-with-attributes",
            "description": f"{state} emits matching end tags with recoverable attributes",
            "input": f"alpha</{tag} class=x>tail",
            "tokens": ["Text(data=alpha)", f"EndTag(name={tag})", "Text(data=tail)", "EOF"],
            "diagnostics": ["end-tag-with-attributes"],
        },
        {
            **base,
            "id": f"{mode}-self-closing-end-tag",
            "description": f"{state} emits matching self-closing-looking end tags with a diagnostic",
            "input": f"alpha</{tag}/>tail",
            "tokens": ["Text(data=alpha)", f"EndTag(name={tag})", "Text(data=tail)", "EOF"],
            "diagnostics": ["end-tag-with-trailing-solidus"],
        },
        {
            **base,
            "id": f"{mode}-eof-after-end-tag-open",
            "description": f"{state} keeps a dangling end-tag opener literal at EOF",
            "input": "alpha</",
            "tokens": ["Text(data=alpha</)", "EOF"],
        },
    ]
    return cases


def seeded_end_tag_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    for context in SEEDED_END_TAG_STATES:
        tag = str(context["last_start_tag"])
        script_like_eof = bool(context.get("script_like_eof", False))
        base = {
            "initial_state": context["initial_state"],
            "last_start_tag": context["last_start_tag"],
            "current_end_tag": context["current_end_tag"],
            "temporary_buffer": context["temporary_buffer"],
        }
        cases.extend(
            [
                {
                    **base,
                    "id": f"seeded-{context['id']}-delimiter",
                    "description": f"{context['initial_state']} emits the seeded matching end tag",
                    "input": ">after",
                    "tokens": [f"EndTag(name={tag})", "Text(data=after)", "EOF"],
                },
                {
                    **base,
                    "id": f"seeded-{context['id']}-eof",
                    "description": f"{context['initial_state']} keeps seeded text literal at EOF",
                    "input": "",
                    "tokens": [f"Text(data=</{context['temporary_buffer']})", "EOF"],
                },
            ]
        )
    for context in RECOVERY_STATES:
        tag = str(context["last_start_tag"])
        cases.append(
            {
                "id": f"seeded-{context['id']}-delimiter",
                "description": f"{context['initial_state']} emits seeded matching end tag with recovery diagnostic",
                "input": ">after",
                "initial_state": context["initial_state"],
                "last_start_tag": context["last_start_tag"],
                "current_end_tag": context["current_end_tag"],
                "temporary_buffer": context["temporary_buffer"],
                "tokens": [f"EndTag(name={tag})", "Text(data=after)", "EOF"],
                "diagnostics": [context["diagnostic"]],
            }
        )
    cases.extend(
        [
            {
                "id": "seeded-rcdata-name-mismatch",
                "description": "seeded RCDATA end-tag name mismatch remains literal before a later delimiter",
                "input": ">text</title>",
                "initial_state": "RCDATA end tag name state",
                "last_start_tag": "title",
                "current_end_tag": "style",
                "temporary_buffer": "style",
                "tokens": ["Text(data=</style>text)", "EndTag(name=title)", "EOF"],
            },
            {
                "id": "seeded-script-escaped-self-closing-mismatch",
                "description": "seeded script escaped self-closing mismatch remains literal before a later delimiter",
                "input": ">tail</script>",
                "initial_state": "Script data escaped self-closing end tag state",
                "last_start_tag": "script",
                "current_end_tag": "style",
                "temporary_buffer": "style",
                "tokens": ["Text(data=</style/>tail)", "EndTag(name=script)", "EOF"],
            },
        ]
    )
    return cases


def script_escape_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "script-data-comment-like-escape",
            "description": "script data keeps comment-like escape text before the matching end tag",
            "input": "alpha<!-- hidden --></script>after",
            "initial_state": "Script data state",
            "last_start_tag": "script",
            "tokens": [
                "Text(data=alpha<!-- hidden -->)",
                "EndTag(name=script)",
                "Text(data=after)",
                "EOF",
            ],
        },
        {
            "id": "script-escaped-double-escape-start",
            "description": "script escaped double-escape start keeps inner script delimiters literal",
            "input": "alpha<script>inside</script>after</script>",
            "initial_state": "Script data escaped state",
            "last_start_tag": "script",
            "tokens": [
                "Text(data=alpha<script>inside</script>after)",
                "EndTag(name=script)",
                "EOF",
            ],
        },
        {
            "id": "script-double-escaped-less-than-slash",
            "description": "script double escaped less-than state exits on slash-prefixed script text",
            "input": "/script>inside</script>after</script>",
            "initial_state": "Script data double escaped less-than sign state",
            "last_start_tag": "script",
            "tokens": [
                "Text(data=/script>inside)",
                "EndTag(name=script)",
                "Text(data=after)",
                "EndTag(name=script)",
                "EOF",
            ],
        },
    ]


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
