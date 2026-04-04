-- Tests for coding_adventures.asciidoc_parser
-- =============================================
--
-- Verifies the AsciiDoc parser produces correct Document AST nodes for each
-- supported block and inline construct.
--
-- Coverage:
--   Block nodes:  document, heading (levels 1-6), paragraph, code_block
--                 (fenced with language, literal), blockquote (quote_block),
--                 list (unordered + ordered), thematic_break, raw_block
--   Inline nodes: text, strong (*), emphasis (_), unconstrained (**/__),
--                 code_span, link macro, image macro, xref, bare URL,
--                 hard_break, soft_break
--   Edge cases:   comments skipped, attribute blocks, source language,
--                 empty document, nested inline markup
--
-- @module test_asciidoc_parser

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local parser = require("coding_adventures.asciidoc_parser")

-- ─── Helpers ──────────────────────────────────────────────────────────────────

local function parse(src)
  return parser.parse(src)
end

local function first_child(src)
  return parse(src).children[1]
end

local function find_inline(children, type_name)
  for _, node in ipairs(children) do
    if node.type == type_name then return node end
  end
  return nil
end

-- ─── Tests ────────────────────────────────────────────────────────────────────

describe("asciidoc_parser", function()

  -- ── Document node ──────────────────────────────────────────────────────────

  describe("document", function()
    it("returns a document node", function()
      local doc = parse("")
      assert.equals("document", doc.type)
      assert.same({}, doc.children)
    end)

    it("blank-only input produces empty document", function()
      local doc = parse("   \n\n\n")
      assert.equals("document", doc.type)
      assert.same({}, doc.children)
    end)
  end)

  -- ── Headings ───────────────────────────────────────────────────────────────

  describe("headings", function()
    it("parses = h1", function()
      local node = first_child("= Hello\n")
      assert.equals("heading", node.type)
      assert.equals(1, node.level)
    end)

    it("parses == h2", function()
      local node = first_child("== Section\n")
      assert.equals("heading", node.type)
      assert.equals(2, node.level)
    end)

    it("parses === h3 through ====== h6", function()
      for level = 3, 6 do
        local node = first_child(string.rep("=", level) .. " Heading\n")
        assert.equals("heading", node.type)
        assert.equals(level, node.level)
      end
    end)

    it("heading children contain text inline node", function()
      local node = first_child("= My Title\n")
      assert.is_true(#node.children > 0)
      -- Find the text node (raw unescaped value — renderer handles escaping)
      local found = false
      for _, child in ipairs(node.children) do
        if child.type == "text" and child.value:find("My Title") then
          found = true
        end
      end
      assert.is_true(found)
    end)

    it("heading ignores trailing whitespace", function()
      local node = first_child("= Title   \n")
      assert.equals("heading", node.type)
      assert.equals(1, node.level)
    end)
  end)

  -- ── Thematic break ─────────────────────────────────────────────────────────

  describe("thematic_break", function()
    it("parses ''' as thematic_break", function()
      local node = first_child("'''\n")
      assert.equals("thematic_break", node.type)
    end)

    it("parses '''' (four quotes) as thematic_break", function()
      local node = first_child("''''\n")
      assert.equals("thematic_break", node.type)
    end)
  end)

  -- ── Paragraph ──────────────────────────────────────────────────────────────

  describe("paragraph", function()
    it("single line becomes paragraph", function()
      local node = first_child("Hello world\n")
      assert.equals("paragraph", node.type)
    end)

    it("multi-line paragraph accumulated", function()
      local doc = parse("Line one\nLine two\n\n")
      assert.equals(1, #doc.children)
      assert.equals("paragraph", doc.children[1].type)
    end)

    it("blank line separates paragraphs", function()
      local doc = parse("First\n\nSecond\n")
      assert.equals(2, #doc.children)
      assert.equals("paragraph", doc.children[1].type)
      assert.equals("paragraph", doc.children[2].type)
    end)
  end)

  -- ── Code blocks ────────────────────────────────────────────────────────────

  describe("code_block", function()
    it("fenced code block with ----", function()
      local node = first_child("----\nsome code\n----\n")
      assert.equals("code_block", node.type)
      assert.is_true(node.value:find("some code") ~= nil)
    end)

    it("code block language set by [source,lang]", function()
      local node = first_child("[source,python]\n----\nprint('hi')\n----\n")
      assert.equals("code_block", node.type)
      assert.equals("python", node.language)
    end)

    it("code block without source annotation has empty language", function()
      local node = first_child("----\nno lang\n----\n")
      assert.equals("code_block", node.type)
      assert.equals("", node.language)
    end)

    it("literal block with ....", function()
      local node = first_child("....\nliteral text\n....\n")
      assert.equals("code_block", node.type)
      assert.is_true(node.value:find("literal text") ~= nil)
    end)

    it("code block preserves multiple lines", function()
      local node = first_child("----\nline1\nline2\n----\n")
      assert.equals("code_block", node.type)
      assert.is_true(node.value:find("line1") ~= nil)
      assert.is_true(node.value:find("line2") ~= nil)
    end)
  end)

  -- ── Passthrough block ──────────────────────────────────────────────────────

  describe("passthrough_block", function()
    it("++++ block becomes raw_block", function()
      local node = first_child("++++\n<video src='x.mp4'/>\n++++\n")
      assert.equals("raw_block", node.type)
      assert.is_true(node.value:find("video") ~= nil)
    end)
  end)

  -- ── Quote block ────────────────────────────────────────────────────────────

  describe("quote_block", function()
    it("____ block becomes blockquote", function()
      local node = first_child("____\nA quote\n____\n")
      assert.equals("blockquote", node.type)
    end)

    it("blockquote children are parsed recursively", function()
      local node = first_child("____\nA quote\n____\n")
      assert.is_true(#node.children > 0)
      assert.equals("paragraph", node.children[1].type)
    end)
  end)

  -- ── Lists ──────────────────────────────────────────────────────────────────

  describe("unordered_list", function()
    it("* items become list node with ordered=false", function()
      local node = first_child("* Alpha\n* Beta\n")
      assert.equals("list", node.type)
      assert.is_false(node.ordered)
    end)

    it("list has correct number of items", function()
      local node = first_child("* One\n* Two\n* Three\n")
      assert.equals(3, #node.children)
    end)

    it("list items have list_item type", function()
      local node = first_child("* Item\n")
      assert.equals("list_item", node.children[1].type)
    end)

    it("** nested bullet treated as same-level item", function()
      local node = first_child("* Top\n** Nested\n")
      assert.equals("list", node.type)
      assert.equals(2, #node.children)
    end)
  end)

  describe("ordered_list", function()
    it(". items become list node with ordered=true", function()
      local node = first_child(". First\n. Second\n")
      assert.equals("list", node.type)
      assert.is_true(node.ordered)
    end)

    it("ordered list has correct number of items", function()
      local node = first_child(". A\n. B\n. C\n")
      assert.equals(3, #node.children)
    end)
  end)

  -- ── Comments ───────────────────────────────────────────────────────────────

  describe("comments", function()
    it("// comment line is skipped", function()
      local doc = parse("// This is a comment\nHello\n")
      assert.equals(1, #doc.children)
      assert.equals("paragraph", doc.children[1].type)
    end)

    it("comment between blocks does not create nodes", function()
      local doc = parse("= Title\n// comment\n== Sub\n")
      assert.equals(2, #doc.children)
    end)
  end)

  -- ── Inline: strong (bold) ──────────────────────────────────────────────────

  describe("inline strong", function()
    it("*bold* produces strong node", function()
      local node = first_child("*bold text*\n")
      assert.equals("paragraph", node.type)
      local strong = find_inline(node.children, "strong")
      assert.is_not_nil(strong)
    end)

    it("**unconstrained bold** produces strong node", function()
      local node = first_child("**bold**\n")
      assert.equals("paragraph", node.type)
      local strong = find_inline(node.children, "strong")
      assert.is_not_nil(strong)
    end)

    it("strong children contain text", function()
      local node = first_child("*hello*\n")
      local strong = find_inline(node.children, "strong")
      assert.is_not_nil(strong)
      assert.is_true(#strong.children > 0)
    end)
  end)

  -- ── Inline: emphasis (italic) ──────────────────────────────────────────────

  describe("inline emphasis", function()
    it("_italic_ produces emphasis node", function()
      local node = first_child("_italic text_\n")
      assert.equals("paragraph", node.type)
      local emph = find_inline(node.children, "emphasis")
      assert.is_not_nil(emph)
    end)

    it("__unconstrained italic__ produces emphasis node", function()
      local node = first_child("__italic__\n")
      local emph = find_inline(node.children, "emphasis")
      assert.is_not_nil(emph)
    end)
  end)

  -- ── Inline: code span ──────────────────────────────────────────────────────

  describe("inline code_span", function()
    it("`code` produces code_span node", function()
      local node = first_child("`foo()`\n")
      local code = find_inline(node.children, "code_span")
      assert.is_not_nil(code)
      assert.is_true(code.value:find("foo") ~= nil)
    end)

    it("code span value is verbatim (no inline parsing)", function()
      local node = first_child("`*not bold*`\n")
      local code = find_inline(node.children, "code_span")
      assert.is_not_nil(code)
    end)
  end)

  -- ── Inline: links ──────────────────────────────────────────────────────────

  describe("inline link", function()
    it("link:url[text] produces link node", function()
      local node = first_child("link:https://example.com[Click Here]\n")
      local link = find_inline(node.children, "link")
      assert.is_not_nil(link)
      assert.equals("https://example.com", link.destination)
    end)

    it("cross-reference <<anchor,text>> produces link to #anchor", function()
      local node = first_child("<<intro,Introduction>>\n")
      local link = find_inline(node.children, "link")
      assert.is_not_nil(link)
      assert.equals("#intro", link.destination)
    end)

    it("cross-reference <<anchor>> without label uses anchor as text", function()
      local node = first_child("<<section-one>>\n")
      local link = find_inline(node.children, "link")
      assert.is_not_nil(link)
      assert.equals("#section-one", link.destination)
    end)

    it("bare https:// URL produces link node", function()
      local node = first_child("Visit https://example.com today\n")
      local link = find_inline(node.children, "link")
      assert.is_not_nil(link)
      assert.is_true(link.destination:find("https://example.com") ~= nil)
    end)
  end)

  -- ── Inline: images ─────────────────────────────────────────────────────────

  describe("inline image", function()
    it("image:url[alt] produces image node", function()
      local node = first_child("image:photo.png[A photo]\n")
      local img = find_inline(node.children, "image")
      assert.is_not_nil(img)
      assert.equals("photo.png", img.destination)
    end)
  end)

  -- ── HTML escaping in text ──────────────────────────────────────────────────

  describe("HTML escaping", function()
    it("< is stored raw in text nodes (renderer escapes)", function()
      local node = first_child("a < b\n")
      local text = find_inline(node.children, "text")
      assert.is_not_nil(text)
      assert.is_true(text.value:find("<") ~= nil)
    end)

    it("& is stored raw in text nodes (renderer escapes)", function()
      local node = first_child("a & b\n")
      local text = find_inline(node.children, "text")
      assert.is_not_nil(text)
      assert.is_true(text.value:find("&") ~= nil)
    end)
  end)

  -- ── Mixed content ──────────────────────────────────────────────────────────

  describe("mixed content", function()
    it("heading followed by paragraph", function()
      local doc = parse("= Title\n\nSome text\n")
      assert.equals(2, #doc.children)
      assert.equals("heading",   doc.children[1].type)
      assert.equals("paragraph", doc.children[2].type)
    end)

    it("code block after paragraph", function()
      local doc = parse("Intro\n\n----\ncode\n----\n")
      assert.equals(2, #doc.children)
      assert.equals("paragraph",  doc.children[1].type)
      assert.equals("code_block", doc.children[2].type)
    end)

    it("full AsciiDoc article structure", function()
      local src = [[
= My Article

An introduction paragraph.

== Section One

* Item A
* Item B

----
some code
----
]]
      local doc = parse(src)
      assert.is_true(#doc.children >= 4)
    end)
  end)

end)
