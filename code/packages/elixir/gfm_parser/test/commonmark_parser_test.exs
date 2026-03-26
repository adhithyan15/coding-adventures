defmodule CodingAdventures.CommonmarkParserTest do
  use ExUnit.Case, async: true
  doctest CodingAdventures.CommonmarkParser

  alias CodingAdventures.CommonmarkParser
  alias CodingAdventures.DocumentAstToHtml, as: Renderer

  # Helper: parse markdown and render to HTML
  defp md(text) do
    text |> CommonmarkParser.parse() |> Renderer.render()
  end

  describe "block structure" do
    test "empty input" do
      assert md("") == ""
    end

    test "paragraph" do
      assert md("Hello world") == "<p>Hello world</p>\n"
    end

    test "two paragraphs" do
      assert md("Hello\n\nWorld") == "<p>Hello</p>\n<p>World</p>\n"
    end

    test "ATX heading level 1" do
      assert md("# Hello") == "<h1>Hello</h1>\n"
    end

    test "ATX heading level 3" do
      assert md("### Third") == "<h3>Third</h3>\n"
    end

    test "setext heading level 1" do
      assert md("Hello\n=====") == "<h1>Hello</h1>\n"
    end

    test "setext heading level 2" do
      assert md("Hello\n-----") == "<h2>Hello</h2>\n"
    end

    test "thematic break" do
      assert md("---") == "<hr />\n"
    end

    test "fenced code block" do
      assert md("```\ncode here\n```") == "<pre><code>code here\n</code></pre>\n"
    end

    test "fenced code block with language" do
      assert md("```elixir\nIO.puts \"hi\"\n```") == "<pre><code class=\"language-elixir\">IO.puts &quot;hi&quot;\n</code></pre>\n"
    end

    test "indented code block" do
      assert md("    code") == "<pre><code>code\n</code></pre>\n"
    end

    test "blockquote" do
      assert md("> quoted") == "<blockquote>\n<p>quoted</p>\n</blockquote>\n"
    end

    test "unordered list" do
      result = md("- item1\n- item2")
      assert result == "<ul>\n<li>item1</li>\n<li>item2</li>\n</ul>\n"
    end

    test "ordered list" do
      result = md("1. first\n2. second")
      assert result == "<ol>\n<li>first</li>\n<li>second</li>\n</ol>\n"
    end
  end

  describe "inline parsing" do
    test "emphasis" do
      assert md("*em*") == "<p><em>em</em></p>\n"
    end

    test "emphasis with underscores" do
      assert md("_em_") == "<p><em>em</em></p>\n"
    end

    test "strong" do
      assert md("**bold**") == "<p><strong>bold</strong></p>\n"
    end

    test "strikethrough" do
      assert md("~~gone~~") == "<p><del>gone</del></p>\n"
    end

    test "code span" do
      assert md("`code`") == "<p><code>code</code></p>\n"
    end

    test "inline link" do
      assert md("[text](url)") == "<p><a href=\"url\">text</a></p>\n"
    end

    test "inline link with title" do
      assert md("[text](url \"title\")") == "<p><a href=\"url\" title=\"title\">text</a></p>\n"
    end

    test "autolink url" do
      assert md("<https://example.com>") == "<p><a href=\"https://example.com\">https://example.com</a></p>\n"
    end

    test "autolink email" do
      assert md("<user@example.com>") == "<p><a href=\"mailto:user@example.com\">user@example.com</a></p>\n"
    end

    test "image" do
      assert md("![alt](img.png)") == "<p><img src=\"img.png\" alt=\"alt\" /></p>\n"
    end

    test "hard break from two spaces" do
      assert md("line1  \nline2") == "<p>line1<br />\nline2</p>\n"
    end

    test "soft break" do
      assert md("line1\nline2") == "<p>line1\nline2</p>\n"
    end

    test "html entity in text" do
      assert md("&amp;") == "<p>&amp;</p>\n"
    end

    test "escaped punctuation" do
      assert md("\\*literal\\*") == "<p>*literal*</p>\n"
    end
  end

  describe "link reference definitions" do
    test "full reference link" do
      assert md("[text][ref]\n\n[ref]: https://example.com") ==
        "<p><a href=\"https://example.com\">text</a></p>\n"
    end

    test "reference with title" do
      assert md("[text][ref]\n\n[ref]: https://example.com \"Title\"") ==
        "<p><a href=\"https://example.com\" title=\"Title\">text</a></p>\n"
    end

    test "shortcut reference" do
      assert md("[ref]\n\n[ref]: https://example.com") ==
        "<p><a href=\"https://example.com\">ref</a></p>\n"
    end
  end

  describe "HTML blocks" do
    test "html block passthrough" do
      result = md("<div>\nhello\n</div>")
      assert result == "<div>\nhello\n</div>\n"
    end
  end

  describe "GFM extensions" do
    test "task list items" do
      assert md("- [x] done\n- [ ] todo\n") ==
               "<ul>\n<li><input type=\"checkbox\" disabled=\"\" checked=\"\" /> done</li>\n<li><input type=\"checkbox\" disabled=\"\" /> todo</li>\n</ul>\n"
    end

    test "pipe table" do
      assert md("| a | b |\n| :--- | ---: |\n| c | d |\n") ==
               "<table>\n<thead>\n<tr>\n<th align=\"left\">a</th>\n<th align=\"right\">b</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td align=\"left\">c</td>\n<td align=\"right\">d</td>\n</tr>\n</tbody>\n</table>\n"
    end
  end
end
