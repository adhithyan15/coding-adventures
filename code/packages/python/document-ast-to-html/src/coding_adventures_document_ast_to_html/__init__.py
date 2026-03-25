"""Document AST → HTML Renderer.

Converts a Document AST (produced by any front-end parser) into an HTML
string. The renderer is a simple recursive tree walk — each node type maps
to HTML elements following the CommonMark spec HTML rendering rules.

=== Node mapping ===

  DocumentNode      → rendered children
  HeadingNode       → <h1>…</h1> through <h6>…</h6>
  ParagraphNode     → <p>…</p>  (omitted in tight list context)
  CodeBlockNode     → <pre><code [class="language-X"]>…</code></pre>
  BlockquoteNode    → <blockquote>\\n…</blockquote>
  ListNode          → <ul> or <ol [start="N"]>
  ListItemNode      → <li>…</li>
  ThematicBreakNode → <hr />
  RawBlockNode      → verbatim if format="html", skipped otherwise

  TextNode          → HTML-escaped text
  EmphasisNode      → <em>…</em>
  StrongNode        → <strong>…</strong>
  CodeSpanNode      → <code>…</code>
  LinkNode          → <a href="…" [title="…"]>…</a>
  ImageNode         → <img src="…" alt="…" [title="…"] />
  AutolinkNode      → <a href="[mailto:]…">…</a>
  RawInlineNode     → verbatim if format="html", skipped otherwise
  HardBreakNode     → <br />\\n
  SoftBreakNode     → \\n

=== Tight vs Loose Lists ===

A tight list suppresses <p> tags around paragraph content in list items:

  Tight:   <li>item text</li>
  Loose:   <li><p>item text</p></li>

The `tight` flag on ListNode controls this.

=== Security ===

- Text content and attribute values are HTML-escaped via escape_html.
- RawBlockNode and RawInlineNode content is passed through verbatim when
  format == "html" — this is intentional and spec-required.
- Link and image URLs are sanitized to block dangerous schemes:
  javascript:, vbscript:, data:, blob:.
"""

from coding_adventures_document_ast_to_html.renderer import RenderOptions, to_html

__all__ = ["to_html", "RenderOptions"]
