"""GFM pipeline — thin re-export of parse() and to_html().

This package is the single-import convenience wrapper for the full
GitHub Flavored Markdown → HTML pipeline. It re-exports:

  - `parse(markdown)` from coding_adventures_gfm_parser
  - `to_html(document, options?)` from coding_adventures_document_ast_to_html

So you can do:

    from coding_adventures_gfm import parse, to_html
    html = to_html(parse("# Hello\\n"))

instead of importing from both packages separately.

=== The Pipeline ===

    Markdown
      ↓  parse()  [coding_adventures_gfm_parser]
    DocumentNode  [coding_adventures_document_ast]
      ↓  to_html()  [coding_adventures_document_ast_to_html]
    HTML string

=== Direct usage ===

    from coding_adventures_gfm import parse, to_html

    # Parse to AST
    doc = parse("# Hello\\n\\nWorld\\n")
    doc["type"]  # "document"

    # Render to HTML
    html = to_html(doc)
    # "<h1>Hello</h1>\\n<p>World</p>\\n"

    # One-liner
    html = to_html(parse("# Hello\\n\\nWorld\\n"))
"""

from coding_adventures_gfm_parser import parse
from coding_adventures_document_ast_to_html import RenderOptions, to_html

VERSION = "0.1.0"

__all__ = ["parse", "to_html", "RenderOptions", "VERSION"]
