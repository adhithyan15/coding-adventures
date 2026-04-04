defmodule CodingAdventures.AsciidocTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Asciidoc

  defp to_html(text), do: Asciidoc.to_html(text)

  # ── Empty input ───────────────────────────────────────────────────────────────

  describe "to_html/1 with empty input" do
    test "empty string returns empty string" do
      assert to_html("") == ""
    end

    test "whitespace-only returns empty string" do
      assert to_html("\n\n") == ""
    end
  end

  # ── Headings ─────────────────────────────────────────────────────────────────

  describe "heading rendering" do
    test "level 1 renders to h1" do
      html = to_html("= Hello\n")
      assert html =~ "<h1>"
      assert html =~ "Hello"
      assert html =~ "</h1>"
    end

    test "level 2 renders to h2" do
      html = to_html("== Section\n")
      assert html =~ "<h2>"
      assert html =~ "</h2>"
    end

    test "level 3 renders to h3" do
      html = to_html("=== Sub\n")
      assert html =~ "<h3>"
    end

    test "level 4 renders to h4" do
      html = to_html("==== Deep\n")
      assert html =~ "<h4>"
    end

    test "level 5 renders to h5" do
      html = to_html("===== Deeper\n")
      assert html =~ "<h5>"
    end

    test "level 6 renders to h6" do
      html = to_html("====== Deepest\n")
      assert html =~ "<h6>"
    end
  end

  # ── Paragraph ─────────────────────────────────────────────────────────────────

  describe "paragraph rendering" do
    test "paragraph renders to p tag" do
      html = to_html("Hello world.\n")
      assert html =~ "<p>"
      assert html =~ "Hello world."
      assert html =~ "</p>"
    end

    test "multiple paragraphs" do
      html = to_html("First.\n\nSecond.\n")
      assert html =~ "<p>First."
      assert html =~ "<p>Second."
    end
  end

  # ── Thematic break ────────────────────────────────────────────────────────────

  describe "thematic break rendering" do
    test "''' renders to hr tag" do
      html = to_html("'''\n")
      assert html =~ "<hr"
    end
  end

  # ── Code blocks ──────────────────────────────────────────────────────────────

  describe "code block rendering" do
    test "code block renders pre and code tags" do
      html = to_html("[source,elixir]\n----\nIO.puts(\"hi\")\n----\n")
      assert html =~ "<pre>"
      assert html =~ "<code"
      assert html =~ "IO.puts"
    end

    test "code block without language" do
      html = to_html("----\nplain code\n----\n")
      assert html =~ "<code"
      assert html =~ "plain code"
    end

    test "literal block renders as code" do
      html = to_html("....\nliteral\n....\n")
      assert html =~ "<code"
      assert html =~ "literal"
    end
  end

  # ── Raw / passthrough block ───────────────────────────────────────────────────

  describe "passthrough block rendering" do
    test "passthrough HTML is included verbatim" do
      html = to_html("++++\n<div class=\"custom\">raw</div>\n++++\n")
      assert html =~ "<div class=\"custom\">"
      assert html =~ "raw"
    end
  end

  # ── Blockquote ────────────────────────────────────────────────────────────────

  describe "blockquote rendering" do
    test "quote block renders to blockquote tag" do
      html = to_html("____\nA famous quote.\n____\n")
      assert html =~ "<blockquote>"
      assert html =~ "A famous quote."
      assert html =~ "</blockquote>"
    end
  end

  # ── Lists ─────────────────────────────────────────────────────────────────────

  describe "unordered list rendering" do
    test "unordered list renders to ul/li tags" do
      html = to_html("* Alpha\n* Beta\n")
      assert html =~ "<ul>"
      assert html =~ "<li>"
      assert html =~ "Alpha"
      assert html =~ "Beta"
    end
  end

  describe "ordered list rendering" do
    test "ordered list renders to ol/li tags" do
      html = to_html(". First\n. Second\n")
      assert html =~ "<ol"
      assert html =~ "<li>"
      assert html =~ "First"
      assert html =~ "Second"
    end
  end

  # ── Inline formatting ─────────────────────────────────────────────────────────

  describe "inline strong" do
    test "*bold* renders to strong tag" do
      html = to_html("*bold*\n")
      assert html =~ "<strong>"
      assert html =~ "bold"
      assert html =~ "</strong>"
    end

    test "**bold** renders to strong tag" do
      html = to_html("**bold**\n")
      assert html =~ "<strong>"
    end
  end

  describe "inline emphasis" do
    test "_italic_ renders to em tag" do
      html = to_html("_italic_\n")
      assert html =~ "<em>"
      assert html =~ "italic"
      assert html =~ "</em>"
    end

    test "__italic__ renders to em tag" do
      html = to_html("__italic__\n")
      assert html =~ "<em>"
    end
  end

  describe "inline code span" do
    test "backtick renders to code tag" do
      html = to_html("`snippet`\n")
      assert html =~ "<code>"
      assert html =~ "snippet"
    end
  end

  describe "inline link" do
    test "link macro renders to anchor tag" do
      html = to_html("link:https://example.com[Example]\n")
      assert html =~ "<a href=\"https://example.com\""
      assert html =~ "Example"
    end
  end

  describe "inline image" do
    test "image macro renders to img tag" do
      html = to_html("image:photo.png[A photo]\n")
      assert html =~ "<img"
      assert html =~ "photo.png"
    end
  end

  describe "autolink" do
    test "bare https:// URL renders as autolink" do
      html = to_html("https://example.com\n")
      assert html =~ "https://example.com"
    end
  end

  # ── Realistic document ────────────────────────────────────────────────────────

  describe "realistic document" do
    test "full document produces well-formed HTML structure" do
      input = """
      = Getting Started

      This is a *quick* guide.

      == Installation

      Run the following:

      [source,shell]
      ----
      mix deps.get
      ----

      Then call `MyApp.start/0`.
      """

      html = to_html(input)

      # All major elements present
      assert html =~ "<h1>"
      assert html =~ "<h2>"
      assert html =~ "<p>"
      assert html =~ "<strong>"
      assert html =~ "<code"
      assert html =~ "mix deps.get"
    end

    test "document with comment lines — comments are absent from HTML" do
      html = to_html("// hidden comment\nVisible text\n")
      refute html =~ "hidden comment"
      assert html =~ "Visible text"
    end

    test "soft breaks become whitespace in HTML" do
      html = to_html("Line one\nLine two\n")
      # Both lines should appear in the output (as single paragraph)
      assert html =~ "Line one"
      assert html =~ "Line two"
    end
  end
end
