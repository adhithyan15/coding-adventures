defmodule CodingAdventures.DocumentAstSanitizerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.DocumentAst, as: AST
  alias CodingAdventures.DocumentAstSanitizer
  alias CodingAdventures.DocumentAstSanitizer.Policy
  alias CodingAdventures.DocumentAstSanitizer.UrlUtils

  # ── Helpers ────────────────────────────────────────────────────────────────

  defp doc(children), do: AST.document(children)
  defp sanitize(document, policy), do: DocumentAstSanitizer.sanitize(document, policy)

  # ── UrlUtils tests ─────────────────────────────────────────────────────────

  describe "UrlUtils.strip_control_chars/1" do
    test "removes null bytes" do
      assert UrlUtils.strip_control_chars("java\x00script:") == "javascript:"
    end

    test "removes carriage return" do
      assert UrlUtils.strip_control_chars("java\rscript:") == "javascript:"
    end

    test "removes newline" do
      assert UrlUtils.strip_control_chars("java\nscript:") == "javascript:"
    end

    test "removes tab" do
      assert UrlUtils.strip_control_chars("java\tscript:") == "javascript:"
    end

    test "removes zero-width space U+200B" do
      assert UrlUtils.strip_control_chars("\u200Bjavascript:") == "javascript:"
    end

    test "removes zero-width joiner U+200D" do
      assert UrlUtils.strip_control_chars("java\u200Dscript:") == "javascript:"
    end

    test "removes word joiner U+2060" do
      assert UrlUtils.strip_control_chars("java\u2060script:") == "javascript:"
    end

    test "removes BOM U+FEFF" do
      assert UrlUtils.strip_control_chars("\uFEFFhttps:") == "https:"
    end

    test "leaves normal URLs unchanged" do
      assert UrlUtils.strip_control_chars("https://example.com") == "https://example.com"
    end
  end

  describe "UrlUtils.extract_scheme/1" do
    test "extracts https scheme" do
      assert UrlUtils.extract_scheme("https://example.com") == "https"
    end

    test "extracts http scheme" do
      assert UrlUtils.extract_scheme("http://example.com") == "http"
    end

    test "extracts mailto scheme" do
      assert UrlUtils.extract_scheme("mailto:user@example.com") == "mailto"
    end

    test "extracts javascript scheme" do
      assert UrlUtils.extract_scheme("javascript:alert(1)") == "javascript"
    end

    test "extracts data scheme" do
      assert UrlUtils.extract_scheme("data:text/html,<script>alert(1)</script>") == "data"
    end

    test "returns nil for relative path" do
      assert UrlUtils.extract_scheme("/relative/path") == nil
    end

    test "returns nil for relative path without leading slash" do
      assert UrlUtils.extract_scheme("relative/path") == nil
    end

    test "returns nil for protocol-relative URL" do
      assert UrlUtils.extract_scheme("//example.com/path") == nil
    end

    test "returns nil when colon is in query string" do
      assert UrlUtils.extract_scheme("page?key=value:thing") == nil
    end

    test "returns nil when colon is in path" do
      assert UrlUtils.extract_scheme("path/to:something") == nil
    end

    test "lowercases the scheme" do
      assert UrlUtils.extract_scheme("JAVASCRIPT:alert(1)") == "javascript"
      assert UrlUtils.extract_scheme("HTTPS://example.com") == "https"
    end
  end

  describe "UrlUtils.scheme_allowed?/2" do
    test "allows https with allowlist" do
      assert UrlUtils.scheme_allowed?("https://example.com", ["http", "https"])
    end

    test "blocks javascript with allowlist" do
      refute UrlUtils.scheme_allowed?("javascript:alert(1)", ["http", "https"])
    end

    test "blocks data: scheme" do
      refute UrlUtils.scheme_allowed?("data:text/html,xss", ["http", "https"])
    end

    test "blocks vbscript: scheme" do
      refute UrlUtils.scheme_allowed?("vbscript:MsgBox(1)", ["http", "https"])
    end

    test "allows relative URLs" do
      assert UrlUtils.scheme_allowed?("/relative", ["http", "https"])
      assert UrlUtils.scheme_allowed?("../up", ["http", "https"])
    end

    test "allows all when schemes is nil" do
      assert UrlUtils.scheme_allowed?("javascript:alert(1)", nil)
      assert UrlUtils.scheme_allowed?("data:text/html,x", nil)
    end

    test "strips control chars before checking — null byte bypass" do
      # "java\x00script:" after stripping becomes "javascript:" — blocked
      refute UrlUtils.scheme_allowed?("java\x00script:alert(1)", ["http", "https"])
    end

    test "strips zero-width char bypass" do
      refute UrlUtils.scheme_allowed?("\u200Bjavascript:alert(1)", ["http", "https"])
    end

    test "case-insensitive scheme check" do
      assert UrlUtils.scheme_allowed?("HTTPS://example.com", ["http", "https"])
      refute UrlUtils.scheme_allowed?("JAVASCRIPT:alert(1)", ["http", "https"])
    end
  end

  # ── Policy presets ─────────────────────────────────────────────────────────

  describe "Policy presets" do
    test "strict/0 returns a Policy struct" do
      p = Policy.strict()
      assert p.allow_raw_block_formats == :drop_all
      assert p.allow_raw_inline_formats == :drop_all
      assert p.allowed_url_schemes == ["http", "https", "mailto"]
      assert p.transform_image_to_text == true
      assert p.min_heading_level == 2
      assert p.drop_links == false
    end

    test "relaxed/0 allows html raw formats" do
      p = Policy.relaxed()
      assert p.allow_raw_block_formats == ["html"]
      assert p.allow_raw_inline_formats == ["html"]
      assert p.allowed_url_schemes == ["http", "https", "mailto", "ftp"]
      assert p.transform_image_to_text == false
    end

    test "passthrough/0 allows everything" do
      p = Policy.passthrough()
      assert p.allow_raw_block_formats == :passthrough
      assert p.allow_raw_inline_formats == :passthrough
      assert p.allowed_url_schemes == nil
      assert p.min_heading_level == 1
      assert p.max_heading_level == 6
    end
  end

  # ── Immutability ───────────────────────────────────────────────────────────

  describe "immutability" do
    test "input document is not mutated" do
      original = doc([AST.paragraph([AST.text("hello")])])
      _sanitized = sanitize(original, Policy.strict())
      # Original must still be the same map value
      assert original == doc([AST.paragraph([AST.text("hello")])])
    end

    test "passthrough is effectively identity" do
      d = doc([
        AST.paragraph([AST.text("hello"), AST.emphasis([AST.text("world")])]),
        AST.heading(1, [AST.text("Title")]),
        AST.code_block("elixir", "IO.puts \"hi\"\n"),
        AST.thematic_break(),
        AST.table([:left], [
          AST.table_row(true, [AST.table_cell([AST.strikethrough([AST.text("x")])])]),
          AST.table_row(false, [AST.table_cell([AST.text("y")])])
        ])
      ])
      assert sanitize(d, Policy.passthrough()) == d
    end
  end

  # ── DocumentNode ───────────────────────────────────────────────────────────

  describe "document node" do
    test "empty document remains valid" do
      d = doc([])
      result = sanitize(d, Policy.strict())
      assert result == doc([])
    end

    test "document with all children dropped becomes empty doc" do
      d = doc([AST.raw_block("html", "<script>alert(1)</script>\n")])
      result = sanitize(d, Policy.strict())
      assert result == doc([])
    end
  end

  # ── ThematicBreak ──────────────────────────────────────────────────────────

  describe "thematic_break" do
    test "always kept as-is" do
      d = doc([AST.thematic_break()])
      assert sanitize(d, Policy.strict()) == d
    end
  end

  describe "task_item and table nodes" do
    test "task items recurse into children" do
      d = doc([
        AST.list(false, nil, true, [
          AST.task_item(true, [AST.paragraph([AST.text("x"), AST.raw_inline("html", "<span>x</span>")])])
        ])
      ])

      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      assert sanitize(d, p) ==
               doc([
                 AST.list(false, nil, true, [
                   AST.task_item(true, [AST.paragraph([AST.text("x")])])
                 ])
               ])
    end

    test "table cells sanitize inline content" do
      d = doc([
        AST.table([:left], [
          AST.table_row(true, [
            AST.table_cell([AST.raw_inline("html", "<em>x</em>")])
          ])
        ])
      ])

      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end
  end

  # ── CodeBlock ─────────────────────────────────────────────────────────────

  describe "code_block" do
    test "kept by default" do
      d = doc([AST.code_block("elixir", "x = 1\n")])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "dropped when drop_code_blocks is true" do
      d = doc([AST.code_block("elixir", "x = 1\n")])
      p = %Policy{Policy.passthrough() | drop_code_blocks: true}
      assert sanitize(d, p) == doc([])
    end

    test "strict preset keeps code blocks" do
      d = doc([AST.code_block("bash", "rm -rf /\n")])
      # Strict doesn't drop code blocks
      assert sanitize(d, Policy.strict()) == d
    end
  end

  # ── Blockquote ────────────────────────────────────────────────────────────

  describe "blockquote" do
    test "kept by default" do
      d = doc([AST.blockquote([AST.paragraph([AST.text("quote")])])])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "dropped when drop_blockquotes is true" do
      d = doc([AST.blockquote([AST.paragraph([AST.text("quote")])])])
      p = %Policy{Policy.passthrough() | drop_blockquotes: true}
      assert sanitize(d, p) == doc([])
    end

    test "dropped if all children dropped" do
      d = doc([
        AST.blockquote([AST.paragraph([AST.raw_inline("html", "<b>bold</b>")])])
      ])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      # Paragraph becomes empty → dropped → blockquote becomes empty → dropped
      assert sanitize(d, p) == doc([])
    end
  end

  # ── Heading ───────────────────────────────────────────────────────────────

  describe "heading" do
    test "kept unchanged when within bounds" do
      d = doc([AST.heading(3, [AST.text("Section")])])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "dropped when max_heading_level is :drop" do
      d = doc([AST.heading(1, [AST.text("Big Title")])])
      p = %Policy{Policy.passthrough() | max_heading_level: :drop}
      assert sanitize(d, p) == doc([])
    end

    test "clamped up to min_heading_level" do
      # h1 with min=2 → level becomes 2
      d = doc([AST.heading(1, [AST.text("Title")])])
      p = %Policy{Policy.passthrough() | min_heading_level: 2}
      result = sanitize(d, p)
      [heading] = result.children
      assert heading.level == 2
    end

    test "clamped down to max_heading_level" do
      # h5 with max=3 → level becomes 3
      d = doc([AST.heading(5, [AST.text("Deep")])])
      p = %Policy{Policy.passthrough() | max_heading_level: 3}
      result = sanitize(d, p)
      [heading] = result.children
      assert heading.level == 3
    end

    test "h1 stays at h1 when min=1 and max=6" do
      d = doc([AST.heading(1, [AST.text("Title")])])
      result = sanitize(d, Policy.passthrough())
      [heading] = result.children
      assert heading.level == 1
    end

    test "strict preset clamps h1 to h2" do
      d = doc([AST.heading(1, [AST.text("Title")])])
      result = sanitize(d, Policy.strict())
      [heading] = result.children
      assert heading.level == 2
    end

    test "heading with all inline children dropped is dropped" do
      d = doc([AST.heading(1, [AST.raw_inline("html", "<b>bold</b>")])])
      assert sanitize(d, Policy.strict()) == doc([])
    end
  end

  # ── Paragraph ─────────────────────────────────────────────────────────────

  describe "paragraph" do
    test "kept with inline children" do
      d = doc([AST.paragraph([AST.text("hello")])])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "dropped when all children are dropped" do
      d = doc([AST.paragraph([AST.raw_inline("html", "<b>bold</b>")])])
      assert sanitize(d, Policy.strict()) == doc([])
    end
  end

  # ── List ──────────────────────────────────────────────────────────────────

  describe "list" do
    test "kept when children survive" do
      d = doc([
        AST.list(false, nil, true, [
          AST.list_item([AST.paragraph([AST.text("item")])])
        ])
      ])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "dropped when all list items are dropped" do
      d = doc([
        AST.list(false, nil, true, [
          AST.list_item([AST.paragraph([AST.raw_inline("html", "<b>x</b>")])])
        ])
      ])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end
  end

  # ── RawBlock ──────────────────────────────────────────────────────────────

  describe "raw_block" do
    test "dropped when allow_raw_block_formats is :drop_all" do
      d = doc([AST.raw_block("html", "<div>raw</div>\n")])
      p = %Policy{Policy.passthrough() | allow_raw_block_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end

    test "kept when allow_raw_block_formats is :passthrough" do
      d = doc([AST.raw_block("html", "<div>raw</div>\n")])
      p = %Policy{Policy.passthrough() | allow_raw_block_formats: :passthrough}
      assert sanitize(d, p) == d
    end

    test "kept when format is in allowlist" do
      d = doc([AST.raw_block("html", "<div>raw</div>\n")])
      p = %Policy{Policy.passthrough() | allow_raw_block_formats: ["html"]}
      assert sanitize(d, p) == d
    end

    test "dropped when format is not in allowlist" do
      d = doc([AST.raw_block("latex", "\\section{Title}\n")])
      p = %Policy{Policy.passthrough() | allow_raw_block_formats: ["html"]}
      assert sanitize(d, p) == doc([])
    end

    test "strict preset drops raw blocks" do
      d = doc([AST.raw_block("html", "<script>alert(1)</script>\n")])
      assert sanitize(d, Policy.strict()) == doc([])
    end

    test "relaxed preset keeps html raw blocks" do
      d = doc([AST.raw_block("html", "<div>ok</div>\n")])
      assert sanitize(d, Policy.relaxed()) == d
    end

    test "relaxed preset drops latex raw blocks" do
      d = doc([AST.raw_block("latex", "\\textbf{hello}\n")])
      assert sanitize(d, Policy.relaxed()) == doc([])
    end
  end

  # ── TextNode ──────────────────────────────────────────────────────────────

  describe "text node" do
    test "always kept" do
      d = doc([AST.paragraph([AST.text("hello world")])])
      assert sanitize(d, Policy.strict()) == d
    end
  end

  # ── Emphasis and Strong ───────────────────────────────────────────────────

  describe "emphasis" do
    test "kept with children" do
      d = doc([AST.paragraph([AST.emphasis([AST.text("em")])])])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "dropped when all children dropped" do
      d = doc([AST.paragraph([AST.emphasis([AST.raw_inline("html", "<b>x</b>")])])])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      result = sanitize(d, p)
      # Emphasis has no children → dropped; paragraph also empty → dropped
      assert result == doc([])
    end
  end

  describe "strong" do
    test "kept with children" do
      d = doc([AST.paragraph([AST.strong([AST.text("bold")])])])
      assert sanitize(d, Policy.passthrough()) == d
    end
  end

  # ── CodeSpan ──────────────────────────────────────────────────────────────

  describe "code_span" do
    test "kept as code_span by default" do
      d = doc([AST.paragraph([AST.code_span("const x = 1")])])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "converted to text when transform_code_span_to_text is true" do
      d = doc([AST.paragraph([AST.code_span("const x = 1")])])
      p = %Policy{Policy.passthrough() | transform_code_span_to_text: true}
      result = sanitize(d, p)
      assert result == doc([AST.paragraph([AST.text("const x = 1")])])
    end
  end

  # ── LinkNode ──────────────────────────────────────────────────────────────

  describe "link" do
    test "kept when URL scheme is allowed" do
      d = doc([
        AST.paragraph([AST.link("https://example.com", nil, [AST.text("click")])])
      ])
      assert sanitize(d, Policy.strict()) == d
    end

    test "destination set to empty string when scheme not allowed" do
      d = doc([
        AST.paragraph([AST.link("javascript:alert(1)", nil, [AST.text("click")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "children promoted when drop_links is true" do
      d = doc([
        AST.paragraph([
          AST.link("https://example.com", nil, [AST.text("click here")])
        ])
      ])
      p = %Policy{Policy.passthrough() | drop_links: true}
      result = sanitize(d, p)
      [para] = result.children
      # Link removed; its text child promoted
      assert para.children == [AST.text("click here")]
    end

    test "multiple link children all promoted" do
      d = doc([
        AST.paragraph([
          AST.link("https://a.com", nil, [AST.text("A"), AST.text("B")])
        ])
      ])
      p = %Policy{Policy.passthrough() | drop_links: true}
      result = sanitize(d, p)
      [para] = result.children
      assert para.children == [AST.text("A"), AST.text("B")]
    end

    test "UPPERCASE javascript: scheme blocked" do
      d = doc([
        AST.paragraph([AST.link("JAVASCRIPT:alert(1)", nil, [AST.text("x")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "null-byte javascript bypass blocked" do
      d = doc([
        AST.paragraph([AST.link("java\x00script:alert(1)", nil, [AST.text("x")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "data: scheme blocked" do
      d = doc([
        AST.paragraph([
          AST.link("data:text/html,<script>alert(1)</script>", nil, [AST.text("x")])
        ])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "relative URL always passes" do
      d = doc([
        AST.paragraph([AST.link("/about", nil, [AST.text("About")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == "/about"
    end

    test "mailto: passes in strict preset" do
      d = doc([
        AST.paragraph([AST.link("mailto:user@example.com", nil, [AST.text("email")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == "mailto:user@example.com"
    end
  end

  # ── ImageNode ─────────────────────────────────────────────────────────────

  describe "image" do
    test "kept unchanged by default" do
      d = doc([AST.paragraph([AST.image("cat.png", nil, "a cat")])])
      assert sanitize(d, Policy.passthrough()) == d
    end

    test "dropped when drop_images is true" do
      d = doc([AST.paragraph([AST.image("cat.png", nil, "a cat")])])
      p = %Policy{Policy.passthrough() | drop_images: true}
      # Image dropped → paragraph empty → paragraph dropped
      assert sanitize(d, p) == doc([])
    end

    test "converted to text when transform_image_to_text is true" do
      d = doc([AST.paragraph([AST.image("cat.png", nil, "a cat")])])
      p = %Policy{Policy.passthrough() | transform_image_to_text: true}
      result = sanitize(d, p)
      assert result == doc([AST.paragraph([AST.text("a cat")])])
    end

    test "drop_images takes precedence over transform_image_to_text" do
      d = doc([AST.paragraph([AST.image("cat.png", nil, "a cat")])])
      p = %Policy{Policy.passthrough() | drop_images: true, transform_image_to_text: true}
      assert sanitize(d, p) == doc([])
    end

    test "destination cleared when scheme not allowed" do
      d = doc([AST.paragraph([AST.image("javascript:alert(1)", nil, "xss")])])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      # Image converted to text (strict preset), so just check text value
      [node] = para.children
      assert node.type == :text
      assert node.value == "xss"
    end

    test "javascript src cleared when only URL sanitization active" do
      d = doc([AST.paragraph([AST.image("javascript:alert(1)", nil, "xss")])])
      p = %Policy{Policy.passthrough() | allowed_url_schemes: ["http", "https"]}
      result = sanitize(d, p)
      [para] = result.children
      [img] = para.children
      assert img.destination == ""
    end

    test "strict preset transforms image to alt text" do
      d = doc([AST.paragraph([AST.image("https://example.com/img.png", nil, "photo")])])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      assert para.children == [AST.text("photo")]
    end
  end

  # ── AutolinkNode ──────────────────────────────────────────────────────────

  describe "autolink" do
    test "kept when scheme is allowed" do
      d = doc([AST.paragraph([AST.autolink("https://example.com", false)])])
      assert sanitize(d, Policy.strict()) == d
    end

    test "kept for email autolinks" do
      d = doc([AST.paragraph([AST.autolink("user@example.com", true)])])
      assert sanitize(d, Policy.strict()) == d
    end

    test "dropped when scheme not allowed" do
      d = doc([AST.paragraph([AST.autolink("data:text/html,xss", false)])])
      result = sanitize(d, Policy.strict())
      # Paragraph becomes empty → dropped
      assert result == doc([])
    end
  end

  # ── RawInline ─────────────────────────────────────────────────────────────

  describe "raw_inline" do
    test "dropped when allow_raw_inline_formats is :drop_all" do
      d = doc([AST.paragraph([AST.raw_inline("html", "<b>bold</b>")])])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end

    test "kept when allow_raw_inline_formats is :passthrough" do
      d = doc([AST.paragraph([AST.raw_inline("html", "<b>bold</b>")])])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :passthrough}
      assert sanitize(d, p) == d
    end

    test "kept when format is in allowlist" do
      d = doc([AST.paragraph([AST.raw_inline("html", "<b>bold</b>")])])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: ["html"]}
      assert sanitize(d, p) == d
    end

    test "dropped when format not in allowlist" do
      d = doc([AST.paragraph([AST.raw_inline("latex", "\\textbf{x}")])])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: ["html"]}
      assert sanitize(d, p) == doc([])
    end

    test "strict preset drops raw inline html" do
      d = doc([AST.paragraph([AST.raw_inline("html", "<script>alert(1)</script>")])])
      assert sanitize(d, Policy.strict()) == doc([])
    end
  end

  # ── HardBreak and SoftBreak ───────────────────────────────────────────────

  describe "hard_break and soft_break" do
    test "hard_break always kept" do
      d = doc([AST.paragraph([AST.text("line1"), AST.hard_break(), AST.text("line2")])])
      assert sanitize(d, Policy.strict()) == d
    end

    test "soft_break always kept" do
      d = doc([AST.paragraph([AST.text("line1"), AST.soft_break(), AST.text("line2")])])
      assert sanitize(d, Policy.strict()) == d
    end
  end

  # ── Empty children pruning ────────────────────────────────────────────────

  describe "empty children pruning" do
    test "paragraph whose only child is dropped is itself dropped" do
      d = doc([AST.paragraph([AST.raw_inline("html", "<b>x</b>")])])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end

    test "emphasis whose only child is dropped is itself dropped" do
      d = doc([
        AST.paragraph([
          AST.emphasis([AST.raw_inline("html", "<b>x</b>")])
        ])
      ])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end

    test "heading whose only inline child is dropped is itself dropped" do
      d = doc([AST.heading(2, [AST.raw_inline("html", "<b>Title</b>")])])
      p = %Policy{Policy.passthrough() | allow_raw_inline_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end

    test "document with all children dropped stays as empty document" do
      d = doc([
        AST.raw_block("html", "<div>x</div>\n"),
        AST.raw_block("latex", "\\section{x}\n")
      ])
      p = %Policy{Policy.passthrough() | allow_raw_block_formats: :drop_all}
      assert sanitize(d, p) == doc([])
    end
  end

  # ── XSS vectors ───────────────────────────────────────────────────────────

  describe "XSS vectors" do
    test "script tag via raw_block dropped by strict" do
      d = doc([AST.raw_block("html", "<script>alert(1)</script>\n")])
      assert sanitize(d, Policy.strict()) == doc([])
    end

    test "inline script via raw_inline dropped by strict" do
      d = doc([AST.paragraph([AST.raw_inline("html", "<script>alert(1)</script>")])])
      assert sanitize(d, Policy.strict()) == doc([])
    end

    test "javascript: link XSS blocked" do
      d = doc([
        AST.paragraph([AST.link("javascript:alert(1)", nil, [AST.text("click me")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "JAVASCRIPT: (uppercase) link XSS blocked" do
      d = doc([
        AST.paragraph([AST.link("JAVASCRIPT:alert(1)", nil, [AST.text("x")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "java\\x00script: null-byte bypass blocked" do
      d = doc([
        AST.paragraph([AST.link("java\x00script:alert(1)", nil, [AST.text("x")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "blob: scheme blocked" do
      d = doc([
        AST.paragraph([
          AST.link("blob:https://origin/some-uuid", nil, [AST.text("x")])
        ])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "vbscript: scheme blocked" do
      d = doc([
        AST.paragraph([AST.link("vbscript:MsgBox(1)", nil, [AST.text("x")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end

    test "data: URL autolink dropped" do
      d = doc([
        AST.paragraph([AST.autolink("data:text/html,<script>alert(1)</script>", false)])
      ])
      assert sanitize(d, Policy.strict()) == doc([])
    end

    test "zero-width bypass in link destination blocked" do
      d = doc([
        AST.paragraph([AST.link("\u200Bjavascript:alert(1)", nil, [AST.text("x")])])
      ])
      result = sanitize(d, Policy.strict())
      [para] = result.children
      [link] = para.children
      assert link.destination == ""
    end
  end

  # ── Complex document sanitization ─────────────────────────────────────────

  describe "complex document" do
    test "mixed document sanitized correctly by strict" do
      d = doc([
        AST.heading(1, [AST.text("My Title")]),
        AST.paragraph([
          AST.text("Hello "),
          AST.link("https://example.com", nil, [AST.text("world")]),
          AST.text(".")
        ]),
        AST.raw_block("html", "<script>alert(1)</script>\n"),
        AST.code_block("elixir", "IO.puts \"hi\"\n"),
        AST.paragraph([
          AST.image("https://example.com/img.png", nil, "photo"),
          AST.raw_inline("html", "<b>dangerous</b>")
        ])
      ])

      result = sanitize(d, Policy.strict())

      # 4 children: heading, paragraph, code_block, paragraph (raw_block dropped)
      [h2, para, code, last_para] = result.children

      # Heading clamped from h1 to h2
      assert h2.type == :heading
      assert h2.level == 2

      # Link preserved (https allowed)
      [_hello, link, _dot] = para.children
      assert link.destination == "https://example.com"

      # raw_block dropped (not present in result.children)

      # code_block kept
      assert code.type == :code_block

      # Last paragraph: image converted to text (strict), raw_inline dropped
      # → only text node "photo" remains
      assert last_para.children == [AST.text("photo")]
    end
  end
end
