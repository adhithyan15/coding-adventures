"""AsciiDoc parser — converts AsciiDoc text to a Document AST.

This package implements an AsciiDoc front-end for the Document AST IR
(coding_adventures_document_ast). It is the AsciiDoc counterpart to the
CommonMark parser (coding_adventures_commonmark_parser).

=== Pipeline position ===

    AsciiDoc source text
        ↓  parse()          ← you are here
    DocumentNode            (coding_adventures_document_ast)
        ↓  to_html()
    HTML string             (coding_adventures_document_ast_to_html)

=== Quick Start ===

    from coding_adventures_asciidoc_parser import parse

    doc = parse("= Hello World\\n\\nThis is a *bold* paragraph.\\n")
    doc["type"]                  # "document"
    doc["children"][0]["type"]   # "heading"
    doc["children"][0]["level"]  # 1
    doc["children"][1]["type"]   # "paragraph"

=== Supported constructs ===

Block-level:
  - Headings: = Level 1 through ====== Level 6
  - Paragraphs (default for plain text)
  - Thematic breaks: ''' (three or more single-quotes)
  - Fenced code blocks: ---- with optional [source,lang] attribute
  - Literal blocks: ....
  - Passthrough blocks: ++++ (→ RawBlockNode { format: "html" })
  - Quote/blockquote blocks: ____
  - Unordered lists: * item, ** nested item, etc.
  - Ordered lists: . item, .. nested item, etc.
  - Line comments: // ... (skipped)

Inline:
  - Strong: *text* and **text**   (AsciiDoc * = bold, not italic!)
  - Emphasis: _text_ and __text__
  - Code spans: `code` (verbatim)
  - Links: link:url[text]
  - Images: image:url[alt]
  - Cross-references: <<anchor,text>> and <<anchor>>
  - URLs: https://url[text] and bare https://... (autolink)
  - Hard breaks: two trailing spaces or \\ before newline
  - Soft breaks: bare newlines within paragraphs

Spec: TE03 — AsciiDoc Parser
"""

from __future__ import annotations

from coding_adventures_asciidoc_parser.block_parser import parse_blocks

VERSION = "0.1.0"


def parse(text: str) -> dict:
    """Parse AsciiDoc source text into a DocumentNode AST.

    The result is a plain dict conforming to the DocumentNode TypedDict
    from coding_adventures_document_ast. All block and inline nodes are
    fully resolved — there are no deferred references.

    @param text  The AsciiDoc source string.
    @returns     A DocumentNode dict with a "children" list of block nodes.

    Example:
        doc = parse("== Section\\n\\n- item one\\n- item two\\n")
        doc["children"][0]["type"]    # "heading"
        doc["children"][0]["level"]   # 2
        doc["children"][1]["type"]    # "list"
        doc["children"][1]["ordered"] # False
    """
    return parse_blocks(text)


__all__ = ["parse", "VERSION"]
