# frozen_string_literal: true

# Document AST → HTML Renderer
#
# Converts a DocumentNode AST (produced by any front-end parser that conforms
# to the Document AST spec TE00) into an HTML string. The renderer is a
# simple recursive tree walk — each node type maps to HTML elements following
# the CommonMark specification HTML rendering rules (Appendix C).
#
# === Node mapping ===
#
#   DocumentNode      → rendered children (no wrapper element)
#   HeadingNode       → <h1>…</h1> through <h6>…</h6>
#   ParagraphNode     → <p>…</p>  (content only in tight list context)
#   CodeBlockNode     → <pre><code [class="language-X"]>…</code></pre>
#   BlockquoteNode    → <blockquote>\n…</blockquote>
#   ListNode          → <ul> or <ol [start="N"]>
#   ListItemNode      → <li>…</li>
#   ThematicBreakNode → <hr />
#   RawBlockNode      → verbatim if format="html", skipped otherwise
#
#   TextNode          → HTML-escaped text
#   EmphasisNode      → <em>…</em>
#   StrongNode        → <strong>…</strong>
#   CodeSpanNode      → <code>…</code>
#   LinkNode          → <a href="…" [title="…"]>…</a>
#   ImageNode         → <img src="…" alt="…" [title="…"] />
#   AutolinkNode      → <a href="[mailto:]…">…</a>
#   RawInlineNode     → verbatim if format="html", skipped otherwise
#   HardBreakNode     → <br />\n
#   SoftBreakNode     → \n
#
# === Tight vs Loose Lists ===
#
# CommonMark distinguishes tight lists (no blank lines between items) from
# loose lists (blank-line-separated items or items with multiple blocks).
#
# In a tight list, the `<p>` wrapper around paragraph content is suppressed:
#
#   Tight:   <li>item text</li>
#   Loose:   <li><p>item text</p></li>
#
# The `tight` flag on `ListNode` controls this.
#
# === Security ===
#
# - Text content and attribute values are HTML-escaped via `escape_html`.
# - `RawBlockNode` and `RawInlineNode` content passes through verbatim when
#   `format == "html"` — this is intentional and spec-required. If you need
#   to render untrusted user-supplied Markdown, pass `sanitize: true` to
#   strip all raw HTML from the output.
# - Link and image URLs are sanitized to block dangerous schemes:
#   `javascript:`, `vbscript:`, `data:`, `blob:`.

module CodingAdventures
  module DocumentAstToHtml
    # ─── URL Sanitization ─────────────────────────────────────────────────────
    #
    # Block schemes that can execute code in the browser. This list mirrors
    # the TypeScript implementation's DANGEROUS_SCHEME constant.
    #
    #   javascript: — executes JS in the browser's origin
    #   vbscript:   — executes VBScript (IE legacy, still blocked by practice)
    #   data:       — can embed scripts as data:text/html or data:text/javascript
    #   blob:       — same-origin blob URLs can contain script content
    DANGEROUS_SCHEME = /\A(?:javascript|vbscript|data|blob):/i

    # Control characters stripped before scheme detection. These are code
    # points that WHATWG URL parsers silently ignore, allowing bypasses like
    # "java\rscript:" == "javascript:". We strip them defensively.
    #
    #   U+0000–U+001F  C0 controls (TAB, LF, CR, etc.)
    #   U+007F–U+009F  DEL + C1 controls
    #   U+200B         ZERO WIDTH SPACE
    #   U+200C         ZERO WIDTH NON-JOINER
    #   U+200D         ZERO WIDTH JOINER
    #   U+2060         WORD JOINER
    #   U+FEFF         BOM / ZERO WIDTH NO-BREAK SPACE
    URL_CONTROL_CHARS = /[\u0000-\u001F\u007F-\u009F\u200B-\u200D\u2060\uFEFF]/u

    # Percent-encode characters in a URL that should not appear unencoded in
    # HTML href/src attributes. Already-encoded sequences (e.g. `%20`) are
    # preserved by including `%` in the safe-character set.
    #
    # Safe characters (not encoded):
    #   - Unreserved: A-Z a-z 0-9 - . _ ~
    #   - Reserved (sub-delimiters & gen-delims): : / ? # @ ! $ & ' ( ) * + , ; =
    #   - Percent sign % (to avoid double-encoding)
    #
    # Characters NOT in the safe set (e.g. [ ] ` \ space) are percent-encoded.
    #
    # @param url [String] The raw URL.
    # @return [String] The URL with unsafe characters percent-encoded.
    def self.normalize_url(url)
      # Safe characters: unreserved (A-Z a-z 0-9 - . _ ~), reserved sub/gen
      # delimiters (: / ? # @ ! $ & ' ( ) * + , ; =), and % (to avoid
      # double-encoding already-encoded sequences).
      url.gsub(%r{[^\w\-.~:/?#@!$&'()*+,;=%]}) do |ch|
        ch.bytes.map { |b| format("%%%02X", b) }.join
      end
    end

    # Sanitize a URL by stripping control characters and blocking dangerous
    # execution-capable schemes.
    #
    # @param url [String] The raw URL.
    # @return [String] The sanitized URL, or "" if the scheme is dangerous.
    def self.sanitize_url(url)
      stripped = url.gsub(URL_CONTROL_CHARS, "")
      return "" if DANGEROUS_SCHEME.match?(stripped)
      stripped
    end

    # HTML-escape the four characters with special meaning in HTML content
    # and attribute values: & < > "
    #
    # Apostrophes are NOT escaped because CommonMark's reference implementation
    # uses double-quoted attributes throughout.
    #
    # @param text [String]
    # @return [String]
    def self.escape_html(text)
      text
        .gsub("&", "&amp;")
        .gsub("<", "&lt;")
        .gsub(">", "&gt;")
        .gsub('"', "&quot;")
    end

    # ─── Public Entry Point ───────────────────────────────────────────────────

    # Render a DocumentNode AST to an HTML string.
    #
    # The input must be a `DocumentAst::DocumentNode` as produced by any
    # front-end parser that implements the Document AST spec (TE00). The
    # output is a valid HTML fragment suitable for embedding in a page body.
    #
    # Security note: raw HTML passthrough is enabled by default (required for
    # CommonMark spec compliance). If you render untrusted Markdown (user
    # content, third-party data), pass `sanitize: true` to strip all raw HTML
    # from the output. Without this, an attacker who controls the Markdown
    # source can inject arbitrary HTML including `<script>` tags.
    #
    # @param document [DocumentAst::DocumentNode] The root document node.
    # @param sanitize [Boolean] Strip all raw HTML (RawBlockNode, RawInlineNode).
    # @return [String] An HTML fragment string.
    #
    # @example
    #   html = CodingAdventures::DocumentAstToHtml.to_html(doc)
    #   html = CodingAdventures::DocumentAstToHtml.to_html(doc, sanitize: true)
    def self.to_html(document, sanitize: false)
      render_blocks(document.children, tight: false, sanitize: sanitize)
    end

    # ─── Block Rendering ──────────────────────────────────────────────────────

    # Render a sequence of block nodes to HTML.
    # tight — whether inside a tight list (suppresses <p> wrappers).
    def self.render_blocks(blocks, tight:, sanitize:)
      blocks.map { |b| render_block(b, tight: tight, sanitize: sanitize) }.join
    end

    def self.render_block(block, tight:, sanitize:)
      case block.type
      when "document"
        render_blocks(block.children, tight: false, sanitize: sanitize)
      when "heading"
        render_heading(block, sanitize: sanitize)
      when "paragraph"
        render_paragraph(block, tight: tight, sanitize: sanitize)
      when "code_block"
        render_code_block(block)
      when "blockquote"
        render_blockquote(block, sanitize: sanitize)
      when "list"
        render_list(block, sanitize: sanitize)
      when "list_item"
        # ListItemNode is normally rendered inside render_list; if called
        # directly, use non-tight mode.
        render_list_item(block, tight: false, sanitize: sanitize)
      when "thematic_break"
        "<hr />\n"
      when "raw_block"
        render_raw_block(block, sanitize: sanitize)
      else
        ""
      end
    end

    # ─── Block Node Renderers ─────────────────────────────────────────────────

    # Render an ATX or setext heading.
    #
    # HeadingNode { level: 1, children: [TextNode { value: "Hello" }] }
    # → <h1>Hello</h1>\n
    def self.render_heading(node, sanitize:)
      inner = render_inlines(node.children, sanitize: sanitize)
      "<h#{node.level}>#{inner}</h#{node.level}>\n"
    end

    # Render a paragraph.
    #
    # In tight list context, the <p> wrapper is omitted and only the inner
    # content is emitted (followed by a newline). This is the CommonMark
    # tight list rule: tight items render their paragraph content directly
    # inside <li> without the block-level <p> element.
    def self.render_paragraph(node, tight:, sanitize:)
      inner = render_inlines(node.children, sanitize: sanitize)
      return "#{inner}\n" if tight
      "<p>#{inner}</p>\n"
    end

    # Render a fenced or indented code block.
    #
    # The content is HTML-escaped but not Markdown-processed. If the block
    # has a language (info string), the <code> tag gets a
    # class="language-<lang>" attribute per CommonMark convention.
    #
    #   CodeBlockNode { language: "ts", value: "const x = 1;\n" }
    #   → <pre><code class="language-ts">const x = 1;\n</code></pre>\n
    def self.render_code_block(node)
      escaped = escape_html(node.value)
      if node.language
        "<pre><code class=\"language-#{escape_html(node.language)}\">#{escaped}</code></pre>\n"
      else
        "<pre><code>#{escaped}</code></pre>\n"
      end
    end

    # Render a blockquote.
    #
    #   BlockquoteNode → <blockquote>\n<p>…</p>\n</blockquote>\n
    def self.render_blockquote(node, sanitize:)
      inner = render_blocks(node.children, tight: false, sanitize: sanitize)
      "<blockquote>\n#{inner}</blockquote>\n"
    end

    # Render an ordered or unordered list.
    #
    # Ordered lists with a start number other than 1 get a `start` attribute.
    # The `tight` flag is forwarded to each list item so <p> tags are omitted
    # inside tight items.
    #
    #   ListNode { ordered: false, tight: true }
    #   → <ul>\n<li>item1</li>\n<li>item2</li>\n</ul>\n
    #
    #   ListNode { ordered: true, start: 3, tight: false }
    #   → <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>\n
    def self.render_list(node, sanitize:)
      tag = node.ordered ? "ol" : "ul"
      # Only emit `start` when it's a valid integer != 1. We guard with
      # Integer check to prevent injection from programmatically built nodes.
      start_attr = if node.ordered && node.start && node.start != 1 && node.start.is_a?(Integer)
        " start=\"#{node.start}\""
      else
        ""
      end
      items = node.children.map { |item|
        render_list_item(item, tight: node.tight, sanitize: sanitize)
      }.join
      "<#{tag}#{start_attr}>\n#{items}</#{tag}>\n"
    end

    # Render a single list item.
    #
    # Tight single-paragraph items: <li>text</li>  (no <p> wrapper).
    # All other items (multiple blocks, non-paragraph first child):
    #   <li>\ncontent\n</li>.
    #
    # An empty item renders as <li></li>.
    def self.render_list_item(node, tight:, sanitize:)
      return "<li></li>\n" if node.children.empty?

      if tight && node.children[0]&.type == "paragraph"
        # Tight list: inline the first paragraph without <p> wrapper.
        first_para = node.children[0]
        first_content = render_inlines(first_para.children, sanitize: sanitize)
        if node.children.length == 1
          # Only one child — simple tight item.
          return "<li>#{first_content}</li>\n"
        end
        # Multiple children: inline the first, then block-render the rest.
        rest = render_blocks(node.children[1..], tight: tight, sanitize: sanitize)
        return "<li>#{first_content}\n#{rest}</li>\n"
      end

      # Loose or non-paragraph first child: block-level format with newlines.
      inner = render_blocks(node.children, tight: tight, sanitize: sanitize)
      last_child = node.children.last
      if tight && last_child&.type == "paragraph" && inner.end_with?("\n")
        # Strip trailing \n so content is flush with </li>.
        return "<li>\n#{inner[0..-2]}</li>\n"
      end
      "<li>\n#{inner}</li>\n"
    end

    # Render a raw block node.
    #
    # If `sanitize` is true, always returns "" — raw HTML must not appear in
    # sanitized output even if format is "html".
    #
    # Otherwise, if `format == "html"`, emit the raw value verbatim (not
    # escaped). Skip silently for any other format.
    def self.render_raw_block(node, sanitize:)
      return "" if sanitize
      return node.value if node.format == "html"
      ""
    end

    # ─── Inline Rendering ─────────────────────────────────────────────────────

    def self.render_inlines(nodes, sanitize:)
      nodes.map { |n| render_inline(n, sanitize: sanitize) }.join
    end

    def self.render_inline(node, sanitize:)
      case node.type
      when "text"
        escape_html(node.value)
      when "emphasis"
        "<em>#{render_inlines(node.children, sanitize: sanitize)}</em>"
      when "strong"
        "<strong>#{render_inlines(node.children, sanitize: sanitize)}</strong>"
      when "code_span"
        # Code span content is HTML-escaped but not Markdown-processed.
        "<code>#{escape_html(node.value)}</code>"
      when "link"
        render_link(node, sanitize: sanitize)
      when "image"
        render_image(node)
      when "autolink"
        render_autolink(node)
      when "raw_inline"
        render_raw_inline(node, sanitize: sanitize)
      when "hard_break"
        "<br />\n"
      when "soft_break"
        # CommonMark spec §6.12: a soft line break renders as a newline,
        # which browsers collapse to a space. We emit "\n" per the spec.
        "\n"
      else
        ""
      end
    end

    # ─── Inline Node Renderers ────────────────────────────────────────────────

    # Render an inline link [text](url "title") or resolved reference link.
    #
    # The URL is sanitized (blocks dangerous schemes) and HTML-escaped in the
    # href attribute. The title (if present) goes in a title attribute.
    def self.render_link(node, sanitize:)
      href = escape_html(sanitize_url(node.destination))
      title_attr = node.title ? " title=\"#{escape_html(node.title)}\"" : ""
      inner = render_inlines(node.children, sanitize: sanitize)
      "<a href=\"#{href}\"#{title_attr}>#{inner}</a>"
    end

    # Render an inline image ![alt](url "title").
    #
    # The alt attribute uses the pre-computed plain-text value (markup already
    # stripped by the parser when it walked the alt content).
    def self.render_image(node)
      src = escape_html(sanitize_url(node.destination))
      alt = escape_html(node.alt)
      title_attr = node.title ? " title=\"#{escape_html(node.title)}\"" : ""
      "<img src=\"#{src}\" alt=\"#{alt}\"#{title_attr} />"
    end

    # Render an autolink <url> or <email>.
    #
    # For email autolinks, the href gets a mailto: prefix.
    # The link text is the raw address (HTML-escaped).
    #
    # sanitize_url is applied to both URL and email destinations — email
    # autolinks are not exempt from URL sanitization.
    def self.render_autolink(node)
      dest = sanitize_url(node.destination)
      href = if node.is_email
        "mailto:#{escape_html(dest)}"
      else
        # Percent-encode unsafe characters before HTML-escaping the href.
        # This mirrors the CommonMark reference implementation which calls
        # normalizeUrl() on autolink destinations.
        escape_html(sanitize_url(normalize_url(dest)))
      end
      text = escape_html(node.destination)
      "<a href=\"#{href}\">#{text}</a>"
    end

    # Render a raw inline node.
    #
    # If `sanitize` is true, always returns "".
    # Otherwise, if `format == "html"`, emit the raw value verbatim.
    def self.render_raw_inline(node, sanitize:)
      return "" if sanitize
      return node.value if node.format == "html"
      ""
    end
  end
end
