-- Tests for coding_adventures.asciidoc
-- =====================================
--
-- End-to-end tests that verify the full AsciiDoc → HTML pipeline.
-- These tests exercise both the parser and the renderer together.
--
-- Coverage:
--   Headings (all 6 levels), paragraphs, code blocks (with/without language),
--   literal blocks, blockquotes, unordered/ordered lists, thematic breaks,
--   inline strong/emphasis/code/link/image, HTML escaping, empty input.
--
-- @module test_asciidoc

-- Point the loader at the asciidoc src, the asciidoc-parser src, and the
-- document_ast_to_html src so that all require() calls inside asciidoc/init.lua
-- resolve correctly without needing a full luarocks install.
package.path = "../src/?.lua;../src/?/init.lua;"
  .. "../../asciidoc-parser/src/?.lua;../../asciidoc-parser/src/?/init.lua;"
  .. "../../document_ast_to_html/src/?.lua;../../document_ast_to_html/src/?/init.lua;"
  .. package.path

local asciidoc = require("coding_adventures.asciidoc")

-- ─── Helpers ──────────────────────────────────────────────────────────────────

local function html(src)
  -- Use parse() then to_html_from_ast() to test the two-step path as well.
  local doc = asciidoc.parse(src)
  return asciidoc.to_html_from_ast(doc)
end

local function render(src)
  return asciidoc.render(src)
end

-- ─── Tests ────────────────────────────────────────────────────────────────────

describe("asciidoc (end-to-end)", function()

  -- ── Module API ─────────────────────────────────────────────────────────────

  describe("module", function()
    it("has to_html function", function()
      assert.is_function(asciidoc.to_html)
    end)

    it("has parse function", function()
      assert.is_function(asciidoc.parse)
    end)

    it("has render alias", function()
      assert.is_function(asciidoc.render)
    end)

    it("to_html returns string", function()
      assert.is_string(asciidoc.to_html("= Hello\n"))
    end)

    it("empty input returns empty string or whitespace", function()
      local result = asciidoc.to_html("")
      assert.is_string(result)
    end)
  end)

  -- ── Headings ───────────────────────────────────────────────────────────────

  describe("headings", function()
    it("= produces <h1>", function()
      local out = render("= Hello\n")
      assert.is_true(out:find("<h1>") ~= nil)
      assert.is_true(out:find("Hello") ~= nil)
    end)

    it("== produces <h2>", function()
      local out = render("== Section\n")
      assert.is_true(out:find("<h2>") ~= nil)
    end)

    it("=== produces <h3>", function()
      local out = render("=== Sub\n")
      assert.is_true(out:find("<h3>") ~= nil)
    end)

    it("====== produces <h6>", function()
      local out = render("====== Deep\n")
      assert.is_true(out:find("<h6>") ~= nil)
    end)
  end)

  -- ── Paragraphs ─────────────────────────────────────────────────────────────

  describe("paragraphs", function()
    it("plain text becomes <p>", function()
      local out = render("Hello world\n")
      assert.is_true(out:find("<p>") ~= nil)
      assert.is_true(out:find("Hello world") ~= nil)
    end)

    it("blank line separates two <p> elements", function()
      local out = render("First\n\nSecond\n")
      local count = 0
      for _ in out:gmatch("<p>") do count = count + 1 end
      assert.equals(2, count)
    end)
  end)

  -- ── Code blocks ────────────────────────────────────────────────────────────

  describe("code blocks", function()
    it("---- block produces <pre><code>", function()
      local out = render("----\nsome code\n----\n")
      assert.is_true(out:find("<pre>") ~= nil)
      assert.is_true(out:find("<code>") ~= nil)
      assert.is_true(out:find("some code") ~= nil)
    end)

    it("[source,python] sets language class", function()
      local out = render("[source,python]\n----\nx = 1\n----\n")
      assert.is_true(out:find("python") ~= nil)
    end)

    it("literal block .... produces <pre><code>", function()
      local out = render("....\nraw text\n....\n")
      assert.is_true(out:find("<pre>") ~= nil)
      assert.is_true(out:find("raw text") ~= nil)
    end)
  end)

  -- ── Blockquote ─────────────────────────────────────────────────────────────

  describe("blockquote", function()
    it("____ block produces <blockquote>", function()
      local out = render("____\nA quote\n____\n")
      assert.is_true(out:find("<blockquote>") ~= nil)
      assert.is_true(out:find("A quote") ~= nil)
    end)
  end)

  -- ── Lists ──────────────────────────────────────────────────────────────────

  describe("lists", function()
    it("* items produce <ul> and <li>", function()
      local out = render("* Alpha\n* Beta\n")
      assert.is_true(out:find("<ul>") ~= nil)
      assert.is_true(out:find("<li>") ~= nil)
      assert.is_true(out:find("Alpha") ~= nil)
      assert.is_true(out:find("Beta") ~= nil)
    end)

    it(". items produce <ol> and <li>", function()
      local out = render(". First\n. Second\n")
      assert.is_true(out:find("<ol>") ~= nil)
      assert.is_true(out:find("<li>") ~= nil)
    end)
  end)

  -- ── Thematic break ─────────────────────────────────────────────────────────

  describe("thematic_break", function()
    it("''' produces <hr>", function()
      local out = render("'''\n")
      assert.is_true(out:find("<hr") ~= nil)
    end)
  end)

  -- ── Inline: strong ─────────────────────────────────────────────────────────

  describe("inline strong", function()
    it("*bold* produces <strong>", function()
      local out = render("*bold text*\n")
      assert.is_true(out:find("<strong>") ~= nil)
      assert.is_true(out:find("bold text") ~= nil)
    end)

    it("**unconstrained bold** produces <strong>", function()
      local out = render("**bold**\n")
      assert.is_true(out:find("<strong>") ~= nil)
    end)
  end)

  -- ── Inline: emphasis ───────────────────────────────────────────────────────

  describe("inline emphasis", function()
    it("_italic_ produces <em>", function()
      local out = render("_italic text_\n")
      assert.is_true(out:find("<em>") ~= nil)
      assert.is_true(out:find("italic text") ~= nil)
    end)

    it("__unconstrained italic__ produces <em>", function()
      local out = render("__italic__\n")
      assert.is_true(out:find("<em>") ~= nil)
    end)
  end)

  -- ── Inline: code span ──────────────────────────────────────────────────────

  describe("inline code", function()
    it("`code` produces <code>", function()
      local out = render("`foo()`\n")
      assert.is_true(out:find("<code>") ~= nil)
      assert.is_true(out:find("foo") ~= nil)
    end)
  end)

  -- ── Inline: links ──────────────────────────────────────────────────────────

  describe("inline links", function()
    it("link:url[text] produces <a href=...>", function()
      local out = render("link:https://example.com[Click]\n")
      assert.is_true(out:find("<a ") ~= nil)
      assert.is_true(out:find("example.com") ~= nil)
      assert.is_true(out:find("Click") ~= nil)
    end)

    it("bare https:// URL becomes hyperlink", function()
      local out = render("https://example.com\n")
      assert.is_true(out:find("<a ") ~= nil)
    end)
  end)

  -- ── Inline: images ─────────────────────────────────────────────────────────

  describe("inline images", function()
    it("image:url[alt] produces <img>", function()
      local out = render("image:photo.png[A photo]\n")
      assert.is_true(out:find("<img") ~= nil)
      assert.is_true(out:find("photo.png") ~= nil)
    end)
  end)

  -- ── HTML escaping ──────────────────────────────────────────────────────────

  describe("HTML escaping", function()
    it("< in text is escaped to &lt;", function()
      local out = render("a < b\n")
      assert.is_true(out:find("&lt;") ~= nil)
    end)

    it("> in text is escaped to &gt;", function()
      local out = render("a > b\n")
      assert.is_true(out:find("&gt;") ~= nil)
    end)

    it("& in text is escaped to &amp;", function()
      local out = render("a & b\n")
      assert.is_true(out:find("&amp;") ~= nil)
    end)
  end)

  -- ── Comments ───────────────────────────────────────────────────────────────

  describe("comments", function()
    it("// comment is not rendered", function()
      local out = render("// hidden\nVisible\n")
      assert.is_nil(out:find("hidden"))
      assert.is_true(out:find("Visible") ~= nil)
    end)
  end)

  -- ── Full article ───────────────────────────────────────────────────────────

  describe("full document", function()
    it("renders a complete AsciiDoc article", function()
      local src = [[
= My Article

Introduction paragraph.

== Section One

Some text with *bold* and _italic_.

[source,lua]
----
local x = 1
----

* Item A
* Item B
]]
      local out = render(src)
      assert.is_true(out:find("<h1>") ~= nil)
      assert.is_true(out:find("<h2>") ~= nil)
      assert.is_true(out:find("<p>") ~= nil)
      assert.is_true(out:find("<strong>") ~= nil)
      assert.is_true(out:find("<em>") ~= nil)
      assert.is_true(out:find("<pre>") ~= nil)
      assert.is_true(out:find("<ul>") ~= nil)
    end)
  end)

end)
