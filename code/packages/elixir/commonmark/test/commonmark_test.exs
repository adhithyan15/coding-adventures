defmodule CodingAdventures.CommonmarkTest do
  use ExUnit.Case, async: true
  doctest CodingAdventures.Commonmark

  alias CodingAdventures.Commonmark

  describe "to_html/1" do
    test "renders a paragraph" do
      assert Commonmark.to_html("Hello, world!\n") == "<p>Hello, world!</p>\n"
    end

    test "renders a heading" do
      assert Commonmark.to_html("# Hello\n") == "<h1>Hello</h1>\n"
    end

    test "renders emphasis and strong" do
      assert Commonmark.to_html("*em* and **strong**\n") ==
               "<p><em>em</em> and <strong>strong</strong></p>\n"
    end

    test "renders a link" do
      assert Commonmark.to_html("[link](http://example.com)\n") ==
               "<p><a href=\"http://example.com\">link</a></p>\n"
    end

    test "renders a blockquote" do
      assert Commonmark.to_html("> quoted\n") ==
               "<blockquote>\n<p>quoted</p>\n</blockquote>\n"
    end

    test "renders an unordered list" do
      assert Commonmark.to_html("- a\n- b\n") ==
               "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n"
    end

    test "renders an ordered list" do
      assert Commonmark.to_html("1. first\n2. second\n") ==
               "<ol>\n<li>first</li>\n<li>second</li>\n</ol>\n"
    end

    test "renders a thematic break" do
      assert Commonmark.to_html("---\n") == "<hr />\n"
    end

    test "renders a fenced code block" do
      # Code block content is HTML-escaped: " becomes &quot;
      assert Commonmark.to_html("```elixir\nIO.puts(\"hello\")\n```\n") ==
               "<pre><code class=\"language-elixir\">IO.puts(&quot;hello&quot;)\n</code></pre>\n"
    end

    test "renders an image" do
      assert Commonmark.to_html("![alt](image.png)\n") ==
               "<p><img src=\"image.png\" alt=\"alt\" /></p>\n"
    end

    test "handles link reference definitions" do
      input = "[link][ref]\n\n[ref]: /uri \"title\"\n"
      assert Commonmark.to_html(input) ==
               "<p><a href=\"/uri\" title=\"title\">link</a></p>\n"
    end

    test "renders inline code" do
      assert Commonmark.to_html("`code`\n") == "<p><code>code</code></p>\n"
    end

    test "handles empty input" do
      assert Commonmark.to_html("") == ""
    end

    test "handles only whitespace" do
      assert Commonmark.to_html("   \n") == ""
    end
  end
end
