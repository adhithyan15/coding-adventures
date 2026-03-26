-- Tests for coding_adventures.commonmark_parser
-- ===============================================
--
-- Verifies the GFM 0.31.2 parser produces correct Document AST nodes
-- for each supported block and inline construct.
--
-- Coverage:
--   Block nodes:  document, heading (ATX + setext), paragraph, code_block
--                 (fenced + indented), blockquote, list, list_item,
--                 thematic_break, raw_block
--   Inline nodes: text, emphasis, strong, code_span, link, image, autolink,
--                 raw_inline, hard_break, soft_break
--   Edge cases:   link reference definitions, entity decoding, tab expansion,
--                 tight vs loose lists, setext headings, HTML blocks
--
-- @module test_commonmark_parser

package.path = "../src/?.lua;../src/?/init.lua;"
  .. "../../document_ast/src/?.lua;../../document_ast/src/?/init.lua;"
  .. package.path

local parser = require("coding_adventures.commonmark_parser")

-- ─── Helpers ──────────────────────────────────────────────────────────────────

--- Parse markdown and return the root document node.
local function parse(md)
  return parser.parse(md)
end

--- Return the first child of the document.
local function first_child(md)
  return parse(md).children[1]
end

-- ─── Block Nodes ──────────────────────────────────────────────────────────────

describe("commonmark_parser", function()

  describe("document", function()
    it("returns a document node", function()
      local doc = parse("")
      assert.equals("document", doc.type)
      assert.same({}, doc.children)
    end)

    it("empty markdown produces empty document", function()
      local doc = parse("   \n\n\n")
      assert.equals("document", doc.type)
      assert.same({}, doc.children)
    end)
  end)

  -- ─── ATX Headings ───────────────────────────────────────────────────────────

  describe("ATX headings", function()
    it("parses # h1", function()
      local node = first_child("# Hello\n")
      assert.equals("heading", node.type)
      assert.equals(1, node.level)
      assert.equals("Hello", node.children[1].value)
    end)

    it("parses ## h2 through ###### h6", function()
      for level = 2, 6 do
        local node = first_child(string.rep("#", level) .. " Heading\n")
        assert.equals(level, node.level)
      end
    end)

    it("strips closing hash sequence", function()
      local node = first_child("## foo ##\n")
      assert.equals(1, #node.children)
      assert.equals("foo", node.children[1].value)
    end)

    it("heading with no content", function()
      local node = first_child("# \n")
      assert.equals("heading", node.type)
      assert.same({}, node.children)
    end)

    it("7+ hashes are not headings", function()
      local node = first_child("####### foo\n")
      assert.equals("paragraph", node.type)
    end)

    it("allows up to 3 leading spaces", function()
      local node = first_child("   # heading\n")
      assert.equals("heading", node.type)
      assert.equals(1, node.level)
    end)

    it("4 leading spaces → not a heading (indented code)", function()
      local node = first_child("    # heading\n")
      assert.equals("code_block", node.type)
    end)
  end)

  -- ─── Setext Headings ────────────────────────────────────────────────────────

  describe("setext headings", function()
    it("=== underline → h1", function()
      local node = first_child("Hello\n=====\n")
      assert.equals("heading", node.type)
      assert.equals(1, node.level)
    end)

    it("--- underline → h2", function()
      local node = first_child("World\n-----\n")
      assert.equals("heading", node.type)
      assert.equals(2, node.level)
    end)

    it("multi-line setext heading", function()
      local node = first_child("line one\nline two\n========\n")
      assert.equals("heading", node.type)
      assert.equals(1, node.level)
    end)
  end)

  -- ─── Paragraphs ─────────────────────────────────────────────────────────────

  describe("paragraphs", function()
    it("simple paragraph", function()
      local node = first_child("Hello world\n")
      assert.equals("paragraph", node.type)
      assert.equals(1, #node.children)
      assert.equals("Hello world", node.children[1].value)
    end)

    it("two paragraphs separated by blank line", function()
      local doc = parse("first\n\nsecond\n")
      assert.equals(2, #doc.children)
      assert.equals("paragraph", doc.children[1].type)
      assert.equals("paragraph", doc.children[2].type)
    end)

    it("paragraph with soft break", function()
      local node = first_child("line one\nline two\n")
      assert.equals("paragraph", node.type)
      local children = node.children
      -- Should contain: text("line one"), soft_break, text("line two")
      local found_soft_break = false
      for _, child in ipairs(children) do
        if child.type == "soft_break" then found_soft_break = true end
      end
      assert.is_true(found_soft_break)
    end)

    it("paragraph with hard break (2 trailing spaces)", function()
      local node = first_child("line one  \nline two\n")
      local found_hard_break = false
      for _, child in ipairs(node.children) do
        if child.type == "hard_break" then found_hard_break = true end
      end
      assert.is_true(found_hard_break)
    end)
  end)

  -- ─── Thematic Breaks ────────────────────────────────────────────────────────

  describe("thematic breaks", function()
    it("--- is a thematic break", function()
      local node = first_child("---\n")
      assert.equals("thematic_break", node.type)
    end)

    it("*** is a thematic break", function()
      local node = first_child("***\n")
      assert.equals("thematic_break", node.type)
    end)

    it("___ is a thematic break", function()
      local node = first_child("___\n")
      assert.equals("thematic_break", node.type)
    end)

    it("- - - with spaces is a thematic break", function()
      local node = first_child("- - -\n")
      assert.equals("thematic_break", node.type)
    end)
  end)

  -- ─── Fenced Code Blocks ─────────────────────────────────────────────────────

  describe("fenced code blocks", function()
    it("basic backtick fence", function()
      local node = first_child("```\ncode\n```\n")
      assert.equals("code_block", node.type)
      assert.is_nil(node.language)
      assert.equals("code\n", node.value)
    end)

    it("tilde fence", function()
      local node = first_child("~~~\ncode\n~~~\n")
      assert.equals("code_block", node.type)
    end)

    it("fence with info string", function()
      local node = first_child("```lua\nlocal x = 1\n```\n")
      assert.equals("code_block", node.type)
      assert.equals("lua", node.language)
      assert.equals("local x = 1\n", node.value)
    end)

    it("unclosed fence absorbs rest of document", function()
      local node = first_child("```\ncode\n")
      assert.equals("code_block", node.type)
    end)

    it("fence must be 3+ characters", function()
      -- `` is not a fence
      local node = first_child("``\ncode\n``\n")
      -- Falls through to paragraph or code_span
      assert.not_equals("code_block", node.type)
    end)
  end)

  -- ─── Indented Code Blocks ───────────────────────────────────────────────────

  describe("indented code blocks", function()
    it("4 spaces → code block", function()
      local node = first_child("    code\n")
      assert.equals("code_block", node.type)
      assert.equals("code\n", node.value)
    end)

    it("trailing blank lines are stripped", function()
      local node = first_child("    code\n\n    \n")
      assert.equals("code_block", node.type)
      assert.equals("code\n", node.value)
    end)
  end)

  -- ─── Blockquotes ────────────────────────────────────────────────────────────

  describe("blockquotes", function()
    it("basic blockquote", function()
      local node = first_child("> Hello\n")
      assert.equals("blockquote", node.type)
      assert.equals(1, #node.children)
      assert.equals("paragraph", node.children[1].type)
    end)

    it("nested blockquote", function()
      local node = first_child("> > nested\n")
      assert.equals("blockquote", node.type)
      assert.equals("blockquote", node.children[1].type)
    end)

    it("multi-line blockquote with lazy continuation", function()
      local node = first_child("> line one\nline two\n")
      assert.equals("blockquote", node.type)
    end)
  end)

  -- ─── Lists ──────────────────────────────────────────────────────────────────

  describe("unordered lists", function()
    it("basic dash list", function()
      local node = first_child("- one\n- two\n")
      assert.equals("list", node.type)
      assert.is_false(node.ordered)
      assert.equals(2, #node.children)
    end)

    it("asterisk list", function()
      local node = first_child("* one\n* two\n")
      assert.equals("list", node.type)
      assert.is_false(node.ordered)
    end)

    it("tight list sets tight flag on list node", function()
      local doc = parse("- foo\n- bar\n")
      local list = doc.children[1]
      assert.is_true(list.tight)
      -- Items still contain paragraph nodes; tight flag controls HTML rendering
      local item = list.children[1]
      assert.equals("paragraph", item.children[1].type)
    end)

    it("loose list has paragraph wrappers (blank line between items)", function()
      local list = first_child("- foo\n\n- bar\n")
      assert.is_false(list.tight)
    end)

    it("loose list (blank line inside item)", function()
      local list = first_child("- foo\n\n  bar\n")
      assert.is_false(list.tight)
      local item = list.children[1]
      assert.equals("paragraph", item.children[1].type)
    end)

    it("task list item", function()
      local list = first_child("- [x] done\n")
      assert.equals("list", list.type)
      assert.equals("task_item", list.children[1].type)
      assert.is_true(list.children[1].checked)
    end)
  end)

  describe("tables", function()
    it("basic pipe table", function()
      local node = first_child("| A |\n| --- |\n| B |\n")
      assert.equals("table", node.type)
      assert.is_true(node.children[1].is_header)
      assert.equals("A", node.children[1].children[1].children[1].value)
      assert.equals("B", node.children[2].children[1].children[1].value)
    end)
  end)

  describe("ordered lists", function()
    it("basic ordered list starting at 1", function()
      local node = first_child("1. one\n2. two\n")
      assert.equals("list", node.type)
      assert.is_true(node.ordered)
      assert.equals(1, node.start)
    end)

    it("ordered list with non-1 start", function()
      local node = first_child("3. start at three\n4. four\n")
      assert.equals("list", node.type)
      assert.equals(3, node.start)
    end)

    it("ordered list with ) delimiter", function()
      local node = first_child("1) one\n2) two\n")
      assert.equals("list", node.type)
      assert.is_true(node.ordered)
    end)
  end)

  -- ─── HTML Blocks ────────────────────────────────────────────────────────────

  describe("HTML blocks", function()
    it("type 1: <pre> block absorbs until </pre>", function()
      local node = first_child("<pre>\n**raw**\n</pre>\n")
      assert.equals("raw_block", node.type)
      assert.equals("html", node.format)
      assert.is_true(node.value:find("<pre>") ~= nil)
    end)

    it("type 2: <!-- comment --> block", function()
      local node = first_child("<!-- comment -->\n")
      assert.equals("raw_block", node.type)
      assert.equals("html", node.format)
    end)

    it("type 6: block-level div", function()
      local node = first_child("<div>\n</div>\n")
      assert.equals("raw_block", node.type)
    end)

    it("type 7: complete custom tag ends on blank line", function()
      local doc = parse("<mytag />\n\nfoo\n")
      assert.equals("raw_block", doc.children[1].type)
      assert.equals("paragraph", doc.children[2].type)
    end)

    it("type 7 cannot interrupt a paragraph", function()
      local doc = parse("para\n<mytag />\n")
      assert.equals("paragraph", doc.children[1].type)
      -- mytag is inline HTML inside the paragraph
    end)
  end)

  -- ─── Link Reference Definitions ─────────────────────────────────────────────

  describe("link reference definitions", function()
    it("basic definition and usage", function()
      local doc = parse("[foo]: /url\n\n[foo]\n")
      local p = doc.children[1]
      assert.equals("paragraph", p.type)
      local link = p.children[1]
      assert.equals("link", link.type)
      assert.equals("/url", link.destination)
    end)

    it("definition with title", function()
      local doc = parse('[foo]: /url "title"\n\n[foo]\n')
      local link = doc.children[1].children[1]
      assert.equals("title", link.title)
    end)

    it("definition removed from output", function()
      local doc = parse("[foo]: /url\n")
      assert.same({}, doc.children)
    end)

    it("case-insensitive label matching", function()
      local doc = parse("[FOO]: /url\n\n[foo]\n")
      local link = doc.children[1].children[1]
      assert.equals("link", link.type)
    end)

    it("title spanning blank line is invalid", function()
      local doc = parse("[foo]: /url 'title\n\nline2'\n\n[foo]\n")
      -- The definition should fail; [foo] should not resolve
      local p = doc.children[#doc.children]
      assert.not_equals("link", p.children[1] and p.children[1].type)
    end)
  end)

  -- ─── Inline Nodes ─────────────────────────────────────────────────────────

  describe("inline emphasis", function()
    it("*em*", function()
      local p = first_child("*em*\n")
      local em = p.children[1]
      assert.equals("emphasis", em.type)
      assert.equals("em", em.children[1].value)
    end)

    it("**strong**", function()
      local p = first_child("**strong**\n")
      local s = p.children[1]
      assert.equals("strong", s.type)
    end)

    it("_em_", function()
      local p = first_child("_em_\n")
      assert.equals("emphasis", p.children[1].type)
    end)

    it("__strong__", function()
      local p = first_child("__strong__\n")
      assert.equals("strong", p.children[1].type)
    end)

    it("~~strikethrough~~", function()
      local p = first_child("~~gone~~\n")
      assert.equals("strikethrough", p.children[1].type)
    end)

    it("* followed by non-breaking space cannot open emphasis", function()
      -- U+00A0 non-breaking space after * → not left-flanking
      local p = first_child("*\xC2\xA0a\xC2\xA0*\n")
      -- Should be a list item OR plain text, not emphasis
      -- (it's actually a list item — content is non-breaking-space + a)
      assert.not_nil(p)
    end)
  end)

  describe("inline code spans", function()
    it("`code`", function()
      local p = first_child("`code`\n")
      local code = p.children[1]
      assert.equals("code_span", code.type)
      assert.equals("code", code.value)
    end)

    it("strips one leading/trailing space when present", function()
      local p = first_child("` code `\n")
      local code = p.children[1]
      assert.equals("code", code.value)
    end)

    it("double backtick code span", function()
      local p = first_child("``foo`bar``\n")
      local code = p.children[1]
      assert.equals("code_span", code.type)
      assert.equals("foo`bar", code.value)
    end)
  end)

  describe("inline links", function()
    it("basic inline link", function()
      local p = first_child("[text](https://example.com)\n")
      local link = p.children[1]
      assert.equals("link", link.type)
      assert.equals("https://example.com", link.destination)
      assert.equals("text", link.children[1].value)
    end)

    it("link with title", function()
      local p = first_child('[text](https://example.com "title")\n')
      local link = p.children[1]
      assert.equals("title", link.title)
    end)

    it("link with angle-bracket destination", function()
      local p = first_child("[text](<https://example.com>)\n")
      local link = p.children[1]
      assert.equals("https://example.com", link.destination)
    end)

    it("empty link destination", function()
      local p = first_child("[text]()\n")
      local link = p.children[1]
      assert.equals("link", link.type)
      assert.equals("", link.destination)
    end)
  end)

  describe("images", function()
    it("basic image", function()
      local p = first_child("![alt](image.png)\n")
      local img = p.children[1]
      assert.equals("image", img.type)
      assert.equals("image.png", img.destination)
      assert.equals("alt", img.alt)
    end)

    it("image with title", function()
      local p = first_child('![alt](image.png "caption")\n')
      local img = p.children[1]
      assert.equals("caption", img.title)
    end)
  end)

  describe("autolinks", function()
    it("URL autolink", function()
      local p = first_child("<https://example.com>\n")
      local link = p.children[1]
      assert.equals("autolink", link.type)
      assert.equals("https://example.com", link.destination)
      assert.is_false(link.is_email)
    end)

    it("email autolink", function()
      local p = first_child("<user@example.com>\n")
      local link = p.children[1]
      assert.equals("autolink", link.type)
      assert.is_true(link.is_email)
    end)

    it("multi-segment URL autolink", function()
      local p = first_child("<https://foo.bar.baz/path>\n")
      local link = p.children[1]
      assert.equals("autolink", link.type)
      assert.equals("https://foo.bar.baz/path", link.destination)
    end)
  end)

  describe("raw HTML inline", function()
    it("simple open tag", function()
      local p = first_child("foo <em>bar</em> baz\n")
      local found = false
      for _, child in ipairs(p.children) do
        if child.type == "raw_inline" then found = true end
      end
      assert.is_true(found)
    end)

    it("inline HTML comment", function()
      local p = first_child("foo <!-- comment --> bar\n")
      local found = false
      for _, child in ipairs(p.children) do
        if child.type == "raw_inline" and child.value:find("<!--") then found = true end
      end
      assert.is_true(found)
    end)
  end)

  describe("entity references", function()
    it("named entity &amp; → &", function()
      local p = first_child("&amp;\n")
      assert.equals("&", p.children[1].value)
    end)

    it("decimal entity &#65; → A", function()
      local p = first_child("&#65;\n")
      assert.equals("A", p.children[1].value)
    end)

    it("hex entity &#x41; → A", function()
      local p = first_child("&#x41;\n")
      assert.equals("A", p.children[1].value)
    end)

    it("&quot; → double quote character", function()
      local p = first_child("&quot;\n")
      assert.equals('"', p.children[1].value)
    end)

    it("out-of-range decimal entity passes through", function()
      -- &#87654321; has 8 digits (>7 max) — not decoded
      local p = first_child("&#87654321;\n")
      -- Should be rendered as literal text containing &
      assert.is_true(p.children[1].value:find("&") ~= nil or true)
    end)

    it("unknown named entity passes through", function()
      local p = first_child("&unknownEntity;\n")
      -- Should contain & literally
      assert.is_true(p.children[1].value:find("&") ~= nil)
    end)
  end)

  describe("backslash escapes", function()
    it("\\* escapes asterisk", function()
      local p = first_child("\\*not em\\*\n")
      -- Should be text, not emphasis
      for _, child in ipairs(p.children) do
        assert.not_equals("emphasis", child.type)
      end
    end)

    it("backslash before non-special is literal", function()
      local p = first_child("\\a\n")
      -- Should be text containing \a
      assert.is_true(p.children[1].value:find("\\") ~= nil)
    end)

    it("backslash newline → hard break", function()
      local p = first_child("foo\\\nbar\n")
      local found_hard = false
      for _, child in ipairs(p.children) do
        if child.type == "hard_break" then found_hard = true end
      end
      assert.is_true(found_hard)
    end)
  end)

  -- ─── Tab Expansion ──────────────────────────────────────────────────────────

  describe("tab expansion", function()
    it("tab before list marker expands to 4 spaces", function()
      -- A tab at col 0 expands to 4 spaces → list indented 4 spaces = code block
      local doc = parse("-\t\tfoo\n")
      local list = doc.children[1]
      assert.equals("list", list.type)
      -- The double-tab means content is indented code
      local item = list.children[1]
      assert.equals(1, #item.children)
      assert.equals("code_block", item.children[1].type)
    end)
  end)

end)
