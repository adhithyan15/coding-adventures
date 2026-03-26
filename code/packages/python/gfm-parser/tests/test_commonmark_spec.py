"""GFM 0.31.2 Specification Compliance Tests.

This file runs all 652 examples from the GFM 0.31.2 specification,
comparing our parser + HTML renderer output against the expected HTML.

The spec JSON is downloaded from https://spec.commonmark.org/0.31.2/spec.json
and stored locally at tests/spec.json.

=== Why 652 tests? ===

The GFM 0.31.2 spec has exactly 652 numbered examples covering:
  - Tabs and whitespace handling
  - ATX and setext headings
  - Thematic breaks
  - Fenced and indented code blocks
  - HTML blocks (7 types)
  - Blockquotes
  - Lists (ordered, unordered, tight, loose)
  - Inline: code spans, emphasis, links, images, autolinks, HTML, backslash escapes
  - Precedence rules
  - Edge cases

Passing all 652 examples means the parser is 100% GFM 0.31.2 compliant.
"""

import json
import os

import pytest
from coding_adventures_document_ast_to_html import to_html

from coding_adventures_gfm_parser import parse

# Load the spec from the local JSON file.
# The file is committed alongside this test — no network access needed.
_SPEC_PATH = os.path.join(os.path.dirname(__file__), "spec.json")

with open(_SPEC_PATH, encoding="utf-8") as f:
    _SPEC_EXAMPLES = json.load(f)

# Total expected: 652 examples
assert len(_SPEC_EXAMPLES) == 652, f"Expected 652 examples, got {len(_SPEC_EXAMPLES)}"


def _run_example(example: dict) -> tuple[str, str]:
    """Run a spec example and return (actual_html, expected_html)."""
    markdown = example["markdown"]
    expected_html = example["html"]
    document = parse(markdown)
    actual_html = to_html(document)
    return actual_html, expected_html


@pytest.mark.parametrize(
    "example",
    _SPEC_EXAMPLES,
    ids=[f"example_{e['example']}_{e['section'].replace(' ', '_')}" for e in _SPEC_EXAMPLES],
)
def test_commonmark_spec_example(example: dict) -> None:
    """Run a single GFM spec example.

    For each example, we parse the Markdown, render to HTML, and compare
    the output to the expected HTML from the spec.

    On failure, the test message includes the example number and section
    to make it easy to look up in the spec.
    """
    actual, expected = _run_example(example)

    assert actual == expected, (
        f"GFM spec example {example['example']} failed "
        f"(section: {example['section']}, "
        f"line {example['start_line']}–{example['end_line']}).\n\n"
        f"Markdown:\n{example['markdown']!r}\n\n"
        f"Expected:\n{expected!r}\n\n"
        f"Got:\n{actual!r}"
    )
