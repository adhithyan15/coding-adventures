defmodule CodingAdventures.AsciidocParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.AsciidocParser
  alias CodingAdventures.DocumentAst

  # Convenience helpers to build expected nodes inline
  defp text(v), do: DocumentAst.text(v)
  defp strong(children), do: DocumentAst.strong(children)
  defp em(children), do: DocumentAst.emphasis(children)
  defp para(children), do: DocumentAst.paragraph(children)
  defp heading(level, children), do: DocumentAst.heading(level, children)
  defp code_block(lang, value), do: DocumentAst.code_block(lang, value)
  defp raw_block(fmt, value), do: DocumentAst.raw_block(fmt, value)
  defp thematic_break, do: DocumentAst.thematic_break()
  defp blockquote(children), do: DocumentAst.blockquote(children)
  defp list(ordered, start, tight, children), do: DocumentAst.list(ordered, start, tight, children)
  defp list_item(children), do: DocumentAst.list_item(children)
  defp code_span(v), do: DocumentAst.code_span(v)
  defp link(dest, title, children), do: DocumentAst.link(dest, title, children)
  defp image(dest, title, alt), do: DocumentAst.image(dest, title, alt)
  defp autolink(dest, is_email), do: DocumentAst.autolink(dest, is_email)
  defp soft_break, do: DocumentAst.soft_break()
  defp hard_break, do: DocumentAst.hard_break()

  defp parse(text) do
    AsciidocParser.parse(text)
  end

  defp children(doc), do: doc.children

  # ── Document structure ────────────────────────────────────────────────────────

  describe "parse/1 returns a document node" do
    test "empty string yields empty document" do
      doc = parse("")
      assert doc.type == :document
      assert doc.children == []
    end

    test "whitespace-only yields empty document" do
      doc = parse("   \n\n   ")
      assert doc.type == :document
      assert doc.children == []
    end
  end

  # ── Headings ─────────────────────────────────────────────────────────────────

  describe "headings" do
    test "level 1 — document title" do
      doc = parse("= My Title\n")
      assert children(doc) == [heading(1, [text("My Title")])]
    end

    test "level 2 — section" do
      doc = parse("== Section\n")
      assert children(doc) == [heading(2, [text("Section")])]
    end

    test "level 3" do
      doc = parse("=== Subsection\n")
      assert children(doc) == [heading(3, [text("Subsection")])]
    end

    test "level 4" do
      doc = parse("==== Level 4\n")
      assert children(doc) == [heading(4, [text("Level 4")])]
    end

    test "level 5" do
      doc = parse("===== Level 5\n")
      assert children(doc) == [heading(5, [text("Level 5")])]
    end

    test "level 6" do
      doc = parse("====== Level 6\n")
      assert children(doc) == [heading(6, [text("Level 6")])]
    end

    test "heading with inline formatting" do
      doc = parse("= *Bold* Title\n")
      assert children(doc) == [heading(1, [strong([text("Bold")]), text(" Title")])]
    end

    test "multiple headings in a document" do
      doc = parse("= Title\n\n== Section\n\n=== Sub\n")

      assert children(doc) == [
               heading(1, [text("Title")]),
               heading(2, [text("Section")]),
               heading(3, [text("Sub")])
             ]
    end
  end

  # ── Thematic break ────────────────────────────────────────────────────────────

  describe "thematic break" do
    test "three single quotes" do
      doc = parse("'''\n")
      assert children(doc) == [thematic_break()]
    end

    test "four single quotes" do
      doc = parse("''''\n")
      assert children(doc) == [thematic_break()]
    end

    test "five single quotes" do
      doc = parse("'''''\n")
      assert children(doc) == [thematic_break()]
    end

    test "thematic break between paragraphs" do
      doc = parse("Before\n\n'''\n\nAfter\n")

      assert children(doc) == [
               para([text("Before")]),
               thematic_break(),
               para([text("After")])
             ]
    end
  end

  # ── Code blocks ──────────────────────────────────────────────────────────────

  describe "code block with [source,lang] attribute" do
    test "elixir code block" do
      input = "[source,elixir]\n----\nIO.puts(\"hello\")\n----\n"
      doc = parse(input)
      assert children(doc) == [code_block("elixir", "IO.puts(\"hello\")\n")]
    end

    test "ruby code block" do
      input = "[source,ruby]\n----\nputs 'hello'\n----\n"
      doc = parse(input)
      assert children(doc) == [code_block("ruby", "puts 'hello'\n")]
    end

    test "multiline code block" do
      input = "[source,python]\n----\ndef add(a, b):\n    return a + b\n----\n"
      doc = parse(input)
      assert children(doc) == [code_block("python", "def add(a, b):\n    return a + b\n")]
    end
  end

  describe "code block without attribute" do
    test "plain code block has nil language" do
      input = "----\nsome code\n----\n"
      doc = parse(input)
      assert children(doc) == [code_block(nil, "some code\n")]
    end

    test "empty code block" do
      input = "----\n----\n"
      doc = parse(input)
      [block] = children(doc)
      assert block.type == :code_block
      assert block.language == nil
    end
  end

  # ── Literal block ─────────────────────────────────────────────────────────────

  describe "literal block (....)" do
    test "literal block has nil language" do
      input = "....\nliteral content\n....\n"
      doc = parse(input)
      assert children(doc) == [code_block(nil, "literal content\n")]
    end

    test "multiline literal block" do
      input = "....\nline 1\nline 2\n....\n"
      doc = parse(input)
      assert children(doc) == [code_block(nil, "line 1\nline 2\n")]
    end
  end

  # ── Passthrough block ─────────────────────────────────────────────────────────

  describe "passthrough block (++++)" do
    test "passthrough block becomes raw_block html" do
      input = "++++\n<div>hello</div>\n++++\n"
      doc = parse(input)
      assert children(doc) == [raw_block("html", "<div>hello</div>\n")]
    end

    test "multiline passthrough block" do
      input = "++++\n<p>one</p>\n<p>two</p>\n++++\n"
      doc = parse(input)
      assert children(doc) == [raw_block("html", "<p>one</p>\n<p>two</p>\n")]
    end
  end

  # ── Quote block ───────────────────────────────────────────────────────────────

  describe "quote block (____)" do
    test "quote block with paragraph inside" do
      input = "____\nA famous quote.\n____\n"
      doc = parse(input)
      assert children(doc) == [blockquote([para([text("A famous quote.")])])]
    end

    test "quote block with heading inside" do
      input = "____\n= Inner Title\n\nSome text.\n____\n"
      doc = parse(input)

      assert children(doc) == [
               blockquote([
                 heading(1, [text("Inner Title")]),
                 para([text("Some text.")])
               ])
             ]
    end
  end

  # ── Unordered list ────────────────────────────────────────────────────────────

  describe "unordered list" do
    test "single-level unordered list" do
      input = "* Alpha\n* Beta\n* Gamma\n"
      doc = parse(input)
      [lst] = children(doc)
      assert lst.type == :list
      assert lst.ordered == false
      assert length(lst.children) == 3

      [a, b, c] = lst.children
      assert a.type == :list_item
      assert b.type == :list_item
      assert c.type == :list_item
    end

    test "nested unordered list" do
      input = "* Parent\n** Child 1\n** Child 2\n* Parent 2\n"
      doc = parse(input)
      [lst] = children(doc)
      assert lst.type == :list
      # Two top-level items
      assert length(lst.children) == 2
      [parent1, _parent2] = lst.children
      # Parent 1 should have a nested list
      assert length(parent1.children) == 2
      [_para, nested] = parent1.children
      assert nested.type == :list
      assert length(nested.children) == 2
    end

    test "unordered list followed by paragraph" do
      input = "* Item\n\nParagraph after.\n"
      doc = parse(input)
      assert length(children(doc)) == 2
      [lst, p] = children(doc)
      assert lst.type == :list
      assert p.type == :paragraph
    end
  end

  # ── Ordered list ──────────────────────────────────────────────────────────────

  describe "ordered list" do
    test "single-level ordered list" do
      input = ". First\n. Second\n. Third\n"
      doc = parse(input)
      [lst] = children(doc)
      assert lst.type == :list
      assert lst.ordered == true
      assert length(lst.children) == 3
    end

    test "nested ordered list" do
      input = ". Parent\n.. Child\n. Parent 2\n"
      doc = parse(input)
      [lst] = children(doc)
      assert length(lst.children) == 2
      [p1, _p2] = lst.children
      # Parent 1 has para + nested list
      assert length(p1.children) == 2
    end
  end

  # ── Paragraph ─────────────────────────────────────────────────────────────────

  describe "paragraph" do
    test "plain paragraph" do
      doc = parse("Hello world.\n")
      assert children(doc) == [para([text("Hello world.")])]
    end

    test "multiple paragraphs separated by blank lines" do
      doc = parse("First paragraph.\n\nSecond paragraph.\n")
      assert children(doc) == [
               para([text("First paragraph.")]),
               para([text("Second paragraph.")])
             ]
    end

    test "paragraph with trailing blank" do
      doc = parse("Text\n\n")
      assert children(doc) == [para([text("Text")])]
    end
  end

  # ── Comment lines ─────────────────────────────────────────────────────────────

  describe "comment lines" do
    test "comment lines are skipped" do
      doc = parse("// This is a comment\nHello\n")
      assert children(doc) == [para([text("Hello")])]
    end

    test "multiple comment lines" do
      doc = parse("// comment 1\n// comment 2\nText\n")
      assert children(doc) == [para([text("Text")])]
    end

    test "document with only comments" do
      doc = parse("// nothing here\n// or here\n")
      assert children(doc) == []
    end
  end

  # ── Inline: strong ────────────────────────────────────────────────────────────

  describe "strong (bold)" do
    test "*bold* produces strong node (not emphasis)" do
      doc = parse("*bold*\n")
      [p] = children(doc)
      assert p.children == [strong([text("bold")])]
    end

    test "**bold** produces strong (unconstrained)" do
      doc = parse("**bold**\n")
      [p] = children(doc)
      assert p.children == [strong([text("bold")])]
    end

    test "strong inside sentence" do
      doc = parse("This is *important* text.\n")
      [p] = children(doc)
      assert p.children == [text("This is "), strong([text("important")]), text(" text.")]
    end

    test "multiple bold spans" do
      doc = parse("*a* and *b*\n")
      [p] = children(doc)
      assert p.children == [strong([text("a")]), text(" and "), strong([text("b")])]
    end
  end

  # ── Inline: emphasis ──────────────────────────────────────────────────────────

  describe "emphasis (italic)" do
    test "_italic_ produces emphasis node" do
      doc = parse("_italic_\n")
      [p] = children(doc)
      assert p.children == [em([text("italic")])]
    end

    test "__italic__ produces emphasis (unconstrained)" do
      doc = parse("__italic__\n")
      [p] = children(doc)
      assert p.children == [em([text("italic")])]
    end

    test "emphasis inside sentence" do
      doc = parse("This is _very_ important.\n")
      [p] = children(doc)
      assert p.children == [text("This is "), em([text("very")]), text(" important.")]
    end
  end

  # ── Inline: code span ─────────────────────────────────────────────────────────

  describe "code span" do
    test "backtick produces code_span" do
      doc = parse("`code`\n")
      [p] = children(doc)
      assert p.children == [code_span("code")]
    end

    test "code span with asterisks inside (verbatim)" do
      doc = parse("`*not bold*`\n")
      [p] = children(doc)
      assert p.children == [code_span("*not bold*")]
    end

    test "code span inside sentence" do
      doc = parse("Call `IO.puts/1` to print.\n")
      [p] = children(doc)
      assert p.children == [text("Call "), code_span("IO.puts/1"), text(" to print.")]
    end
  end

  # ── Inline: link macro ────────────────────────────────────────────────────────

  describe "link macro" do
    test "link:url[text] produces link node" do
      doc = parse("link:https://example.com[Example]\n")
      [p] = children(doc)
      assert p.children == [link("https://example.com", nil, [text("Example")])]
    end

    test "link with empty text" do
      doc = parse("link:https://example.com[]\n")
      [p] = children(doc)
      [lnk] = p.children
      assert lnk.type == :link
      assert lnk.destination == "https://example.com"
    end
  end

  # ── Inline: image macro ───────────────────────────────────────────────────────

  describe "image macro" do
    test "image:url[alt] produces image node" do
      doc = parse("image:cat.png[A cute cat]\n")
      [p] = children(doc)
      assert p.children == [image("cat.png", nil, "A cute cat")]
    end

    test "image with empty alt" do
      doc = parse("image:logo.svg[]\n")
      [p] = children(doc)
      [img] = p.children
      assert img.type == :image
      assert img.destination == "logo.svg"
      assert img.alt == ""
    end
  end

  # ── Inline: xref ─────────────────────────────────────────────────────────────

  describe "cross-reference (xref)" do
    test "<<anchor,text>> produces link to #anchor" do
      doc = parse("<<my-section,Go to section>>\n")
      [p] = children(doc)
      [lnk] = p.children
      assert lnk.type == :link
      assert lnk.destination == "#my-section"
      assert lnk.children == [text("Go to section")]
    end

    test "<<anchor>> without text uses anchor as text" do
      doc = parse("<<my-section>>\n")
      [p] = children(doc)
      [lnk] = p.children
      assert lnk.type == :link
      assert lnk.destination == "#my-section"
      assert lnk.children == [text("my-section")]
    end
  end

  # ── Inline: bare URL autolink ─────────────────────────────────────────────────

  describe "bare https:// URL autolink" do
    test "bare https:// URL produces autolink node" do
      doc = parse("Visit https://example.com today.\n")
      [p] = children(doc)
      [_visit, lnk, _today] = p.children
      assert lnk.type == :autolink
      assert lnk.destination == "https://example.com"
      assert lnk.is_email == false
    end

    test "bare http:// URL produces autolink" do
      doc = parse("http://example.com\n")
      [p] = children(doc)
      [lnk] = p.children
      assert lnk.type == :autolink
      assert lnk.destination == "http://example.com"
    end

    test "https URL with bracket text becomes link" do
      doc = parse("https://example.com[Click here]\n")
      [p] = children(doc)
      [lnk] = p.children
      assert lnk.type == :link
      assert lnk.destination == "https://example.com"
      assert lnk.children == [text("Click here")]
    end
  end

  # ── Inline: soft break ────────────────────────────────────────────────────────

  describe "soft break" do
    test "newline within paragraph produces soft_break" do
      doc = parse("Line one\nLine two\n")
      [p] = children(doc)
      assert p.children == [text("Line one"), soft_break(), text("Line two")]
    end
  end

  # ── Inline: hard break ────────────────────────────────────────────────────────

  describe "hard break" do
    test "two trailing spaces + newline produces hard_break" do
      doc = parse("Line one  \nLine two\n")
      [p] = children(doc)
      assert p.children == [text("Line one"), hard_break(), text("Line two")]
    end

    test "backslash + newline produces hard_break" do
      doc = parse("Line one\\\nLine two\n")
      [p] = children(doc)
      assert p.children == [text("Line one"), hard_break(), text("Line two")]
    end
  end

  # ── Inline in heading ─────────────────────────────────────────────────────────

  describe "inline formatting in headings" do
    test "heading with strong text" do
      doc = parse("== *Bold* Heading\n")
      [h] = children(doc)
      assert h.type == :heading
      assert h.level == 2
      assert h.children == [strong([text("Bold")]), text(" Heading")]
    end

    test "heading with emphasis" do
      doc = parse("= _Italic_ Doc Title\n")
      [h] = children(doc)
      assert h.children == [em([text("Italic")]), text(" Doc Title")]
    end

    test "heading with code span" do
      doc = parse("== The `mix` command\n")
      [h] = children(doc)
      assert h.children == [text("The "), code_span("mix"), text(" command")]
    end
  end

  # ── Mixed document ────────────────────────────────────────────────────────────

  describe "realistic mixed document" do
    test "full document with title, paragraph, and code block" do
      input = """
      = Getting Started

      Install the dependency:

      [source,elixir]
      ----
      {:my_lib, "~> 1.0"}
      ----

      Then call `MyLib.start/0`.
      """

      doc = parse(input)
      block_types = Enum.map(children(doc), & &1.type)
      assert :heading in block_types
      assert :paragraph in block_types
      assert :code_block in block_types
    end

    test "document with thematic break dividing sections" do
      input = "= Section A\n\n'''\n\n== Section B\n"
      doc = parse(input)
      block_types = Enum.map(children(doc), & &1.type)
      assert block_types == [:heading, :thematic_break, :heading]
    end
  end

  # ── BlockParser unit tests ────────────────────────────────────────────────────

  describe "BlockParser helper functions" do
    alias CodingAdventures.AsciidocParser.BlockParser

    test "blank? detects blank lines" do
      assert BlockParser.blank?("")
      assert BlockParser.blank?("   ")
      assert BlockParser.blank?("\t")
      refute BlockParser.blank?("text")
    end

    test "heading_line? detects headings" do
      assert BlockParser.heading_line?("= Title")
      assert BlockParser.heading_line?("=== Sub")
      refute BlockParser.heading_line?("======= Too deep")
      refute BlockParser.heading_line?("=No space")
      refute BlockParser.heading_line?("text")
    end

    test "listing_delimiter? detects ---- fences" do
      assert BlockParser.listing_delimiter?("----")
      assert BlockParser.listing_delimiter?("-------")
      refute BlockParser.listing_delimiter?("---")
      refute BlockParser.listing_delimiter?("====")
    end

    test "literal_delimiter? detects .... fences" do
      assert BlockParser.literal_delimiter?("....")
      refute BlockParser.literal_delimiter?("...")
    end

    test "passthrough_delimiter? detects ++++ fences" do
      assert BlockParser.passthrough_delimiter?("++++")
      refute BlockParser.passthrough_delimiter?("+++")
    end

    test "quote_delimiter? detects ____ fences" do
      assert BlockParser.quote_delimiter?("____")
      refute BlockParser.quote_delimiter?("___")
    end

    test "thematic_break_line? detects '''" do
      assert BlockParser.thematic_break_line?("'''")
      assert BlockParser.thematic_break_line?("''''")
      refute BlockParser.thematic_break_line?("''")
      refute BlockParser.thematic_break_line?("---")
    end

    test "parse_heading_line extracts level and text" do
      assert BlockParser.parse_heading_line("= Title") == {1, "Title"}
      assert BlockParser.parse_heading_line("== Section") == {2, "Section"}
      assert BlockParser.parse_heading_line("=== Sub") == {3, "Sub"}
    end

    test "parse_attr_list extracts language" do
      assert BlockParser.parse_attr_list("[source,elixir]") == %{language: "elixir"}
      assert BlockParser.parse_attr_list("[source, ruby]") == %{language: "ruby"}
      assert BlockParser.parse_attr_list("[NOTE]") == %{}
    end

    test "parse_list_item extracts level and text" do
      assert BlockParser.parse_list_item("* item", "*") == {1, "item"}
      assert BlockParser.parse_list_item("** nested", "*") == {2, "nested"}
      assert BlockParser.parse_list_item(". first", ".") == {1, "first"}
      assert BlockParser.parse_list_item(".. second", ".") == {2, "second"}
    end

    test "ensure_trailing_newline adds newline if missing" do
      assert BlockParser.ensure_trailing_newline("hello") == "hello\n"
      assert BlockParser.ensure_trailing_newline("hello\n") == "hello\n"
      assert BlockParser.ensure_trailing_newline("") == "\n"
    end

    test "build_nested_list creates correct list structure" do
      items = [{1, "A"}, {1, "B"}, {1, "C"}]
      lst = BlockParser.build_nested_list(items, false)
      assert lst.type == :list
      assert lst.ordered == false
      assert length(lst.children) == 3
    end

    test "build_nested_list creates nested structure" do
      items = [{1, "Parent"}, {2, "Child 1"}, {2, "Child 2"}]
      lst = BlockParser.build_nested_list(items, false)
      [parent] = lst.children
      # parent has para + nested list
      assert length(parent.children) == 2
      [_p, nested] = parent.children
      assert nested.type == :list
      assert length(nested.children) == 2
    end
  end

  # ── InlineParser unit tests ───────────────────────────────────────────────────

  describe "InlineParser" do
    alias CodingAdventures.AsciidocParser.InlineParser

    test "empty string returns empty list" do
      assert InlineParser.parse("") == []
    end

    test "plain text" do
      assert InlineParser.parse("hello") == [text("hello")]
    end

    test "strong *bold*" do
      assert InlineParser.parse("*bold*") == [strong([text("bold")])]
    end

    test "strong **bold**" do
      assert InlineParser.parse("**bold**") == [strong([text("bold")])]
    end

    test "emphasis _italic_" do
      assert InlineParser.parse("_italic_") == [em([text("italic")])]
    end

    test "emphasis __italic__" do
      assert InlineParser.parse("__italic__") == [em([text("italic")])]
    end

    test "code span" do
      assert InlineParser.parse("`code`") == [code_span("code")]
    end

    test "code span is verbatim (no inner parsing)" do
      assert InlineParser.parse("`*not bold*`") == [code_span("*not bold*")]
    end

    test "link macro" do
      result = InlineParser.parse("link:https://example.com[Click]")
      assert result == [link("https://example.com", nil, [text("Click")])]
    end

    test "image macro" do
      result = InlineParser.parse("image:photo.png[A photo]")
      assert result == [image("photo.png", nil, "A photo")]
    end

    test "xref with text" do
      result = InlineParser.parse("<<section-1,Section 1>>")
      [lnk] = result
      assert lnk.destination == "#section-1"
      assert lnk.children == [text("Section 1")]
    end

    test "xref without text" do
      result = InlineParser.parse("<<my-anchor>>")
      [lnk] = result
      assert lnk.destination == "#my-anchor"
    end

    test "https autolink" do
      result = InlineParser.parse("https://example.com")
      assert result == [autolink("https://example.com", false)]
    end

    test "http autolink" do
      result = InlineParser.parse("http://example.com")
      assert result == [autolink("http://example.com", false)]
    end

    test "https URL with bracket text is a link" do
      result = InlineParser.parse("https://example.com[Visit]")
      [lnk] = result
      assert lnk.type == :link
      assert lnk.destination == "https://example.com"
    end

    test "soft break from newline" do
      result = InlineParser.parse("line1\nline2")
      assert result == [text("line1"), soft_break(), text("line2")]
    end

    test "hard break from two trailing spaces" do
      result = InlineParser.parse("line1  \nline2")
      assert result == [text("line1"), hard_break(), text("line2")]
    end

    test "hard break from backslash newline" do
      result = InlineParser.parse("line1\\\nline2")
      assert result == [text("line1"), hard_break(), text("line2")]
    end

    test "nested strong inside emphasis" do
      # _*bold italic*_ — emphasis wrapping strong
      result = InlineParser.parse("_*bold italic*_")
      assert result == [em([strong([text("bold italic")])])]
    end

    test "mixed inline content" do
      result = InlineParser.parse("Hello *world* and `code`!")
      assert result == [
               text("Hello "),
               strong([text("world")]),
               text(" and "),
               code_span("code"),
               text("!")
             ]
    end
  end

  # ── HTML integration smoke test ───────────────────────────────────────────────

  describe "HTML rendering integration" do
    test "heading renders to h1 tag" do
      doc = parse("= Hello\n")
      html = CodingAdventures.DocumentAstToHtml.render(doc)
      assert String.contains?(html, "<h1>")
      assert String.contains?(html, "Hello")
    end

    test "paragraph renders to p tag" do
      doc = parse("Hello world.\n")
      html = CodingAdventures.DocumentAstToHtml.render(doc)
      assert String.contains?(html, "<p>")
      assert String.contains?(html, "Hello world.")
    end

    test "code block renders to pre/code tags" do
      doc = parse("[source,elixir]\n----\nIO.puts(\"hi\")\n----\n")
      html = CodingAdventures.DocumentAstToHtml.render(doc)
      assert String.contains?(html, "<code")
    end

    test "strong renders to strong tag" do
      doc = parse("*bold*\n")
      html = CodingAdventures.DocumentAstToHtml.render(doc)
      assert String.contains?(html, "<strong>")
    end

    test "emphasis renders to em tag" do
      doc = parse("_italic_\n")
      html = CodingAdventures.DocumentAstToHtml.render(doc)
      assert String.contains?(html, "<em>")
    end

    test "thematic break renders to hr tag" do
      doc = parse("'''\n")
      html = CodingAdventures.DocumentAstToHtml.render(doc)
      assert String.contains?(html, "<hr")
    end
  end
end
