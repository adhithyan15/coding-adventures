"""coding_adventures_asciidoc — AsciiDoc pipeline convenience package.

Provides a single ``to_html(text)`` function that converts an AsciiDoc source
string all the way to an HTML fragment string.

=== The Pipeline ===

    AsciiDoc source
        ↓  parse()   [coding_adventures_asciidoc_parser]
    DocumentNode     [coding_adventures_document_ast]
        ↓  to_html() [coding_adventures_document_ast_to_html]
    HTML string

=== Usage ===

    from coding_adventures_asciidoc import to_html

    html = to_html("= Hello\\n\\nWorld *bold*.\\n")
    # "<h1>Hello</h1>\\n<p>World <strong>bold</strong>.</p>\\n"

Users who need access to the intermediate Document AST should use the
constituent packages directly:

    from coding_adventures_asciidoc_parser import parse
    from coding_adventures_document_ast_to_html import to_html as render

    doc  = parse("= Title\\n\\nBody.\\n")
    html = render(doc)

Spec: TE03 — AsciiDoc Parser
"""

from __future__ import annotations

from coding_adventures_asciidoc_parser import parse
from coding_adventures_document_ast_to_html import to_html as _render

VERSION = "0.1.0"


def to_html(text: str) -> str:
    """Convert an AsciiDoc source string to an HTML fragment.

    This is a convenience wrapper for the full parse → render pipeline.
    It is equivalent to calling:

        from coding_adventures_asciidoc_parser import parse
        from coding_adventures_document_ast_to_html import to_html
        html = to_html(parse(text))

    @param text  The AsciiDoc source string.
    @returns     An HTML fragment string (no ``<html>`` or ``<body>`` wrapper).

    Example:
        >>> to_html("= Hello\\n\\nWorld\\n")
        '<h1>Hello</h1>\\n<p>World</p>\\n'
    """
    doc = parse(text)
    return _render(doc)


__all__ = ["to_html", "VERSION"]
