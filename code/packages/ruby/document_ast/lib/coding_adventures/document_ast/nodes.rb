# frozen_string_literal: true

# Document AST Node Definitions
#
# The Document AST is the "LLVM IR of documents" — a stable, typed,
# immutable tree that every front-end parser produces and every back-end
# renderer consumes.
#
#   Markdown ────────────────────────────────► HTML
#   reStructuredText ────► Document AST ────► PDF
#   HTML ────────────────────────────────────► Plain text
#   DOCX ────────────────────────────────────► DOCX
#
# With a shared IR, N front-ends × M back-ends requires only N + M
# implementations instead of N × M.
#
# === Design Principles ===
#
#   1. Semantic, not notational — nodes carry meaning, not syntax
#   2. Resolved, not deferred   — all link references resolved before IR
#   3. Format-agnostic          — RawBlockNode/RawInlineNode carry a `format` tag
#   4. Immutable and typed      — we use Ruby's Data class (Ruby 3.2+)
#   5. Minimal and stable       — only universal document concepts
#
# === Ruby Data class ===
#
# Ruby 3.2 introduced `Data.define` — lightweight immutable value objects.
# Unlike Struct, Data instances are frozen by default and compare by value.
# They're perfect for AST nodes where we want structural equality.
#
#   node1 = TextNode.new(value: "hello")
#   node2 = TextNode.new(value: "hello")
#   node1 == node2  # => true (value equality, not identity)
#
# This is the Ruby equivalent of TypeScript's `readonly` interfaces.

module CodingAdventures
  module DocumentAst
    # ─── Block Nodes ──────────────────────────────────────────────────────────
    #
    # Block nodes form the structural skeleton of a document. They live at the
    # top level of the document and can be nested (e.g. blockquotes, list items).

    # The root of every document produced by a front-end parser.
    #
    # Every IR value is exactly one DocumentNode. An empty document has an
    # empty children array. DocumentNode is the only node type that cannot
    # appear as a child of another node.
    #
    #   DocumentNode
    #     ├── HeadingNode (level 1)
    #     ├── ParagraphNode
    #     └── ListNode (ordered, tight)
    #           ├── ListItemNode
    #           └── ListItemNode
    DocumentNode = Data.define(:children) do
      # The type tag is used by renderers to dispatch on node type.
      def type = "document"

      def to_s
        "#<DocumentNode children=#{children.length}>"
      end
    end

    # A section heading with a nesting depth. Semantically corresponds to
    # <h1>–<h6> in HTML, === / --- underlines in RST, \section{} in LaTeX.
    #
    # Levels beyond 6 (if a source format supports them) are clamped to 6.
    #
    #   HeadingNode.new(level: 2, children: [TextNode.new(value: "Hello")])
    #   → <h2>Hello</h2>
    HeadingNode = Data.define(:level, :children) do
      def type = "heading"

      def to_s
        "#<HeadingNode level=#{level}>"
      end
    end

    # A block of prose. Contains one or more inline nodes — text, emphasis,
    # links, and soft breaks between the original source lines.
    #
    # Paragraphs are the most common block type. Any content that is not more
    # specifically typed (heading, list, code block, etc.) becomes a paragraph.
    #
    #   ParagraphNode { children: [TextNode("Hello "), EmphasisNode([TextNode("world")])] }
    #   → <p>Hello <em>world</em></p>
    ParagraphNode = Data.define(:children) do
      def type = "paragraph"
    end

    # A block of literal code or pre-formatted text.
    #
    # The `value` is raw — it is NOT decoded for HTML entities and NOT
    # processed for inline markup. Syntax highlighting tools can use the
    # `language` hint.
    #
    # The `value` field always ends with "\n".
    #
    #   CodeBlockNode.new(language: "ruby", value: "x = 1\n")
    #   → <pre><code class="language-ruby">x = 1\n</code></pre>
    CodeBlockNode = Data.define(:language, :value) do
      def type = "code_block"
    end

    # A block of content set apart as a quotation or aside.
    #
    # Can contain any block nodes, including nested blockquotes.
    #
    #   BlockquoteNode { children: [ParagraphNode { children: [TextNode("quote")] }] }
    #   → <blockquote>\n<p>quote</p>\n</blockquote>
    BlockquoteNode = Data.define(:children) do
      def type = "blockquote"
    end

    # An ordered (numbered) or unordered (bulleted) list.
    #
    # === Tight vs Loose ===
    #
    # A tight list is written without blank lines between items; a loose list
    # has blank lines. In HTML, tight lists suppress <p> wrappers around
    # paragraph content.
    #
    #   ListNode.new(ordered: false, start: nil, tight: true, children: [...])
    #   → <ul>\n<li>item1</li>\n</ul>
    #
    #   ListNode.new(ordered: true, start: 3, tight: false, children: [...])
    #   → <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>
    ListNode = Data.define(:ordered, :start, :tight, :children) do
      def type = "list"
    end

    # One item in a ListNode. Contains block-level content.
    #
    # For tight lists the children are typically ParagraphNodes whose content
    # is rendered without wrapping <p> tags.
    ListItemNode = Data.define(:children) do
      def type = "list_item"
    end

    # A GitHub Flavored Markdown task-list item.
    TaskItemNode = Data.define(:checked, :children) do
      def type = "task_item"
    end

    # A visual separator between sections. Leaf node — no children.
    #
    # In HTML renders as <hr />. In RST ----. In plain text ---.
    ThematicBreakNode = Data.define do
      def type = "thematic_break"
    end

    # A block of raw content to be passed through verbatim to a specific back-end.
    #
    # The `format` field identifies the target renderer (e.g. "html", "latex",
    # "rtf"). Back-ends that do not recognise `format` MUST skip this node
    # silently — they should not corrupt output with content intended for a
    # different renderer.
    #
    # **Generalisation of HtmlBlockNode.** The CommonMark AST has
    # HtmlBlockNode { type: "html_block" }. The Document AST replaces it with
    # RawBlockNode { type: "raw_block"; format: "html" }. The semantics are
    # identical for HTML output.
    #
    #   format     HTML back-end    LaTeX back-end    plain-text
    #   ─────────  ─────────────    ──────────────    ──────────
    #   "html"     emit             skip              skip
    #   "latex"    skip             emit              skip
    #   "rtf"      skip             skip              skip
    RawBlockNode = Data.define(:format, :value) do
      def type = "raw_block"
    end

    # A GitHub Flavored Markdown pipe table.
    TableNode = Data.define(:align, :children) do
      def type = "table"
    end

    # One row in a table.
    TableRowNode = Data.define(:is_header, :children) do
      def type = "table_row"
    end

    # One cell in a table.
    TableCellNode = Data.define(:children) do
      def type = "table_cell"
    end

    # ─── Inline Nodes ─────────────────────────────────────────────────────────
    #
    # Inline nodes live inside block nodes that contain prose content.
    # They represent formatted text spans, links, images, and structural
    # characters within a paragraph.

    # Plain text with no markup.
    #
    # All HTML character references (&amp;, &#65;, &#x41;) are decoded into
    # their Unicode equivalents before being stored. The `value` field contains
    # the final, display-ready Unicode string.
    #
    #   "Hello &amp; world" → TextNode.new(value: "Hello & world")
    TextNode = Data.define(:value) do
      def type = "text"
    end

    # Stressed emphasis. In HTML renders as <em>. In Markdown, *text* or _text_.
    #
    #   EmphasisNode.new(children: [TextNode.new(value: "hello")])
    #   → <em>hello</em>
    EmphasisNode = Data.define(:children) do
      def type = "emphasis"
    end

    # Strong importance. In HTML renders as <strong>. In Markdown, **text**.
    #
    #   StrongNode.new(children: [TextNode.new(value: "bold")])
    #   → <strong>bold</strong>
    StrongNode = Data.define(:children) do
      def type = "strong"
    end

    # Struck-through text. In HTML renders as <del>.
    StrikethroughNode = Data.define(:children) do
      def type = "strikethrough"
    end

    # Inline code. The value is raw — not decoded for HTML entities.
    # Leading and trailing spaces are stripped when surrounded on both sides.
    #
    #   `` `const x = 1` `` → CodeSpanNode.new(value: "const x = 1")
    #   → <code>const x = 1</code>
    CodeSpanNode = Data.define(:value) do
      def type = "code_span"
    end

    # A hyperlink with resolved destination.
    #
    # The `destination` is always a fully resolved URL — all reference
    # indirections have been resolved by the front-end. The IR never contains
    # unresolved reference links.
    #
    #   LinkNode.new(
    #     destination: "https://example.com",
    #     title: "Example",
    #     children: [TextNode.new(value: "click here")]
    #   )
    #   → <a href="https://example.com" title="Example">click here</a>
    LinkNode = Data.define(:destination, :title, :children) do
      def type = "link"
    end

    # An embedded image.
    #
    # Like LinkNode, `destination` is always the fully resolved URL. The `alt`
    # field is the plain-text fallback description (all inline markup stripped).
    #
    #   ImageNode.new(destination: "cat.png", alt: "a cat", title: nil)
    #   → <img src="cat.png" alt="a cat" />
    ImageNode = Data.define(:destination, :title, :alt) do
      def type = "image"
    end

    # A URL or email address presented as a direct link.
    # The link text in all back-ends is the raw address itself.
    #
    # **Why preserve is_email?** HTML back-ends need to prepend "mailto:" for
    # email autolinks. Other back-ends may format email addresses differently.
    #
    #   AutolinkNode.new(destination: "user@example.com", is_email: true)
    #   → <a href="mailto:user@example.com">user@example.com</a>
    AutolinkNode = Data.define(:destination, :is_email) do
      def type = "autolink"
    end

    # An inline span of raw content to be passed through verbatim.
    # The `format` field names the target renderer.
    #
    #   RawInlineNode.new(format: "html", value: "<em>raw</em>")
    #   → (HTML back-end) <em>raw</em>
    #   → (LaTeX back-end) (nothing)
    RawInlineNode = Data.define(:format, :value) do
      def type = "raw_inline"
    end

    # A forced line break within a paragraph.
    # In Markdown, two or more trailing spaces before a newline, or `\` + newline.
    HardBreakNode = Data.define do
      def type = "hard_break"
    end

    # A soft line break — a newline within a paragraph that is not a hard break.
    # In HTML, soft breaks render as "\n" (browsers collapse to a single space).
    SoftBreakNode = Data.define do
      def type = "soft_break"
    end
  end
end
