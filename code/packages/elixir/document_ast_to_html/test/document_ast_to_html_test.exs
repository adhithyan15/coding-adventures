defmodule CodingAdventures.DocumentAstToHtmlTest do
  use ExUnit.Case, async: true
  doctest CodingAdventures.DocumentAstToHtml

  alias CodingAdventures.DocumentAst
  alias CodingAdventures.DocumentAstToHtml, as: Renderer

  describe "block rendering" do
    test "empty document" do
      doc = DocumentAst.document([])
      assert Renderer.render(doc) == ""
    end

    test "paragraph" do
      doc = DocumentAst.document([DocumentAst.paragraph([DocumentAst.text("Hello")])])
      assert Renderer.render(doc) == "<p>Hello</p>\n"
    end

    test "heading level 1" do
      doc = DocumentAst.document([DocumentAst.heading(1, [DocumentAst.text("Title")])])
      assert Renderer.render(doc) == "<h1>Title</h1>\n"
    end

    test "heading level 6" do
      doc = DocumentAst.document([DocumentAst.heading(6, [DocumentAst.text("Deep")])])
      assert Renderer.render(doc) == "<h6>Deep</h6>\n"
    end

    test "thematic break" do
      doc = DocumentAst.document([DocumentAst.thematic_break()])
      assert Renderer.render(doc) == "<hr />\n"
    end

    test "code block without language" do
      doc = DocumentAst.document([DocumentAst.code_block(nil, "x = 1\n")])
      assert Renderer.render(doc) == "<pre><code>x = 1\n</code></pre>\n"
    end

    test "code block with language" do
      doc = DocumentAst.document([DocumentAst.code_block("ruby", "puts 'hi'\n")])
      assert Renderer.render(doc) == "<pre><code class=\"language-ruby\">puts 'hi'\n</code></pre>\n"
    end

    test "code block escapes html" do
      doc = DocumentAst.document([DocumentAst.code_block(nil, "<b>&amp;</b>\n")])
      assert Renderer.render(doc) == "<pre><code>&lt;b&gt;&amp;amp;&lt;/b&gt;\n</code></pre>\n"
    end

    test "blockquote" do
      doc = DocumentAst.document([
        DocumentAst.blockquote([
          DocumentAst.paragraph([DocumentAst.text("Quote")])
        ])
      ])
      assert Renderer.render(doc) == "<blockquote>\n<p>Quote</p>\n</blockquote>\n"
    end

    test "unordered list (tight)" do
      doc = DocumentAst.document([
        DocumentAst.list(false, nil, true, [
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("a")])]),
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("b")])])
        ])
      ])
      html = Renderer.render(doc)
      assert html == "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n"
    end

    test "ordered list with start" do
      doc = DocumentAst.document([
        DocumentAst.list(true, 3, false, [
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("item")])])
        ])
      ])
      html = Renderer.render(doc)
      assert String.starts_with?(html, "<ol start=\"3\">")
    end

    test "ordered list start=1 omits start attr" do
      doc = DocumentAst.document([
        DocumentAst.list(true, 1, false, [
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("item")])])
        ])
      ])
      html = Renderer.render(doc)
      assert String.starts_with?(html, "<ol>")
    end

    test "raw block html passthrough" do
      doc = DocumentAst.document([DocumentAst.raw_block("html", "<div>hi</div>\n")])
      assert Renderer.render(doc) == "<div>hi</div>\n"
    end

    test "raw block unknown format is skipped" do
      doc = DocumentAst.document([DocumentAst.raw_block("latex", "\\textbf{hi}\n")])
      assert Renderer.render(doc) == ""
    end
  end

  describe "inline rendering" do
    test "text html escaping" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.text("<b> & \"test\"")])
      ])
      assert Renderer.render(doc) == "<p>&lt;b&gt; &amp; &quot;test&quot;</p>\n"
    end

    test "emphasis" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.emphasis([DocumentAst.text("em")])])
      ])
      assert Renderer.render(doc) == "<p><em>em</em></p>\n"
    end

    test "strong" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.strong([DocumentAst.text("bold")])])
      ])
      assert Renderer.render(doc) == "<p><strong>bold</strong></p>\n"
    end

    test "code span" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.code_span("x + y")])
      ])
      assert Renderer.render(doc) == "<p><code>x + y</code></p>\n"
    end

    test "link" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([
          DocumentAst.link("https://example.com", "Title", [DocumentAst.text("Click")])
        ])
      ])
      assert Renderer.render(doc) == "<p><a href=\"https://example.com\" title=\"Title\">Click</a></p>\n"
    end

    test "link without title" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([
          DocumentAst.link("https://example.com", nil, [DocumentAst.text("Click")])
        ])
      ])
      assert Renderer.render(doc) == "<p><a href=\"https://example.com\">Click</a></p>\n"
    end

    test "image" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([
          DocumentAst.image("cat.png", nil, "a cat")
        ])
      ])
      assert Renderer.render(doc) == "<p><img src=\"cat.png\" alt=\"a cat\" /></p>\n"
    end

    test "image with title" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([
          DocumentAst.image("cat.png", "Cat!", "a cat")
        ])
      ])
      assert Renderer.render(doc) == "<p><img src=\"cat.png\" alt=\"a cat\" title=\"Cat!\" /></p>\n"
    end

    test "autolink URL" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.autolink("https://example.com", false)])
      ])
      assert Renderer.render(doc) == "<p><a href=\"https://example.com\">https://example.com</a></p>\n"
    end

    test "autolink email" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.autolink("user@example.com", true)])
      ])
      assert Renderer.render(doc) == "<p><a href=\"mailto:user@example.com\">user@example.com</a></p>\n"
    end

    test "raw inline html" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.raw_inline("html", "<em>raw</em>")])
      ])
      assert Renderer.render(doc) == "<p><em>raw</em></p>\n"
    end

    test "raw inline unknown format is skipped" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.raw_inline("latex", "\\textit{x}")])
      ])
      assert Renderer.render(doc) == "<p></p>\n"
    end

    test "hard break" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([
          DocumentAst.text("line1"),
          DocumentAst.hard_break(),
          DocumentAst.text("line2")
        ])
      ])
      assert Renderer.render(doc) == "<p>line1<br />\nline2</p>\n"
    end

    test "soft break" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([
          DocumentAst.text("line1"),
          DocumentAst.soft_break(),
          DocumentAst.text("line2")
        ])
      ])
      assert Renderer.render(doc) == "<p>line1\nline2</p>\n"
    end

    test "autolink URL with characters needing percent-encoding" do
      # Backslash is not a safe URL character, must be percent-encoded as %5C
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.autolink("https://example.com/a\\b", false)])
      ])
      html = Renderer.render(doc)
      assert html =~ "href=\"https://example.com/a%5Cb\""
    end

    test "autolink URL with ampersand is html-escaped" do
      doc = DocumentAst.document([
        DocumentAst.paragraph([DocumentAst.autolink("https://example.com/?a=1&b=2", false)])
      ])
      html = Renderer.render(doc)
      assert html =~ "href=\"https://example.com/?a=1&amp;b=2\""
    end

    test "unknown inline node type renders empty string" do
      # A map with an unrecognized type falls through to the catch-all render_inline clause
      doc = DocumentAst.document([
        DocumentAst.paragraph([%{type: :unknown_inline_node}])
      ])
      assert Renderer.render(doc) == "<p></p>\n"
    end
  end

  describe "list edge cases" do
    test "empty list item" do
      doc = DocumentAst.document([
        DocumentAst.list(false, nil, false, [
          DocumentAst.list_item([])
        ])
      ])
      assert Renderer.render(doc) == "<ul>\n<li></li>\n</ul>\n"
    end

    test "loose list renders paragraphs with <p> tags" do
      doc = DocumentAst.document([
        DocumentAst.list(false, nil, false, [
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("a")])]),
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("b")])])
        ])
      ])
      assert Renderer.render(doc) == "<ul>\n<li>\n<p>a</p>\n</li>\n<li>\n<p>b</p>\n</li>\n</ul>\n"
    end

    test "tight list item with paragraph followed by sublist" do
      # Exercises the render_tight_list_item([%{type: :paragraph} | rest]) clause
      sublist = DocumentAst.list(true, 1, true, [
        DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("sub")])])
      ])
      doc = DocumentAst.document([
        DocumentAst.list(false, nil, true, [
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("parent")]), sublist])
        ])
      ])
      html = Renderer.render(doc)
      assert html =~ "<li>parent\n<ol>"
      assert html =~ "<li>sub</li>"
    end

    test "tight list item starting with heading (non-paragraph first child)" do
      # Exercises the render_tight_list_item(children) catch-all clause
      doc = DocumentAst.document([
        DocumentAst.list(false, nil, true, [
          DocumentAst.list_item([
            DocumentAst.heading(2, [DocumentAst.text("Bar")]),
            DocumentAst.paragraph([DocumentAst.text("baz")])
          ])
        ])
      ])
      html = Renderer.render(doc)
      assert html =~ "<li>\n<h2>Bar</h2>\nbaz</li>"
    end

    test "tight list item starting with code block (non-paragraph, non-tight-para last)" do
      # Last child is a code_block, so its trailing \\n is kept before </li>
      doc = DocumentAst.document([
        DocumentAst.list(false, nil, true, [
          DocumentAst.list_item([DocumentAst.code_block("elixir", "x = 1\n")])
        ])
      ])
      html = Renderer.render(doc)
      assert html =~ "<li>\n<pre><code class=\"language-elixir\">x = 1\n</code></pre>\n</li>"
    end

    test "code block with whitespace-only language omits class attribute" do
      doc = DocumentAst.document([DocumentAst.code_block("   ", "code\n")])
      assert Renderer.render(doc) == "<pre><code>code\n</code></pre>\n"
    end

    test "ordered list with start=0 includes start attribute" do
      doc = DocumentAst.document([
        DocumentAst.list(true, 0, false, [
          DocumentAst.list_item([DocumentAst.paragraph([DocumentAst.text("zero")])])
        ])
      ])
      html = Renderer.render(doc)
      assert String.starts_with?(html, "<ol start=\"0\">")
    end
  end

  describe "fallback rendering" do
    test "unknown block node type renders empty string" do
      doc = DocumentAst.document([%{type: :unknown_block}])
      assert Renderer.render(doc) == ""
    end

    test "rendering a non-document node directly returns its fragment" do
      assert Renderer.render(DocumentAst.thematic_break()) == "<hr />\n"
      assert Renderer.render(DocumentAst.heading(3, [DocumentAst.text("Hi")])) == "<h3>Hi</h3>\n"
    end
  end
end
