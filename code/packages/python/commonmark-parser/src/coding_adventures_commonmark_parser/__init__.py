"""CommonMark 0.31.2 compliant Markdown parser.

Parses Markdown source text into a Document AST — the format-agnostic IR
defined in coding_adventures_document_ast. The result is a DocumentNode
ready for any back-end renderer (HTML, PDF, plain text, …).

The parse is two-phase:
  Phase 1 — Block structure: headings, lists, code blocks, blockquotes, …
  Phase 2 — Inline content: emphasis, links, images, code spans, …

=== Quick Start ===

    from coding_adventures_commonmark_parser import parse

    doc = parse("# Hello\\n\\nWorld *with* emphasis.\\n")
    doc["type"]                  # "document"
    doc["children"][0]["type"]   # "heading"
    doc["children"][1]["type"]   # "paragraph"

=== With the HTML renderer ===

    from coding_adventures_commonmark_parser import parse
    from coding_adventures_document_ast_to_html import to_html

    html = to_html(parse("# Hello\\n\\nWorld\\n"))
    # → "<h1>Hello</h1>\\n<p>World</p>\\n"
"""

from __future__ import annotations

from coding_adventures_document_ast import DocumentNode

from coding_adventures_commonmark_parser.block_parser import convert_to_ast, parse_blocks
from coding_adventures_commonmark_parser.inline_parser import resolve_inline_content

VERSION = "0.1.0"


def parse(markdown: str) -> DocumentNode:
    """Parse a CommonMark Markdown string into a DocumentNode AST.

    The result conforms to the Document AST spec (TE00) — a format-agnostic IR
    with all link references resolved and all inline markup parsed.

    @param markdown  The Markdown source string.
    @returns         The root DocumentNode.

    Example:
        doc = parse("## Heading\\n\\n- item 1\\n- item 2\\n")
        doc["children"][0]["type"]   # "heading"
        doc["children"][1]["type"]   # "list"
    """
    # Phase 1: Block parsing — builds the structural skeleton
    mutable_doc, link_refs = parse_blocks(markdown)
    result = convert_to_ast(mutable_doc, link_refs)

    # Phase 2: Inline parsing — fills in emphasis, links, code spans, etc.
    resolve_inline_content(result.document, result.raw_inline_content, link_refs)

    return result.document


__all__ = ["parse", "VERSION"]
