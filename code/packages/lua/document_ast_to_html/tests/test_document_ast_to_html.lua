-- Tests for coding_adventures.document_ast_to_html
-- ==================================================
--
-- Verifies the HTML renderer produces spec-compliant output for every
-- Document AST node type. These tests use the AST constructors directly
-- so they are independent of the parser — they test the renderer in isolation.
--
-- @module test_document_ast_to_html

package.path = "../src/?.lua;../src/?/init.lua;"
  .. "../../document_ast/src/?.lua;../../document_ast/src/?/init.lua;"
  .. package.path

local html  = require("coding_adventures.document_ast_to_html")
local ast   = require("coding_adventures.document_ast")

-- ─── Convenience helpers ──────────────────────────────────────────────────────

--- Render a document node to HTML string.
local function render(doc, opts)
  return html.to_html(doc, opts)
end

--- Build a document with a single block child.
local function doc_of(block)
  return ast.document({ block })
end

--- Build a paragraph with one or more inline children.
local function para(...)
  local children = { ... }
  return ast.paragraph(children)
end

-- ─── Tests ────────────────────────────────────────────────────────────────────

describe("document_ast_to_html", function()

  -- ─── Document ─────────────────────────────────────────────────────────────

  describe("document()", function()
    it("empty document produces empty string", function()
      assert.equals("", render(ast.document()))
    end)

    it("document renders its children", function()
      local doc = ast.document({ ast.paragraph({ ast.text("Hello") }) })
      assert.equals("<p>Hello</p>\n", render(doc))
    end)
  end)

  -- ─── Headings ─────────────────────────────────────────────────────────────

  describe("heading()", function()
    for level = 1, 6 do
      it("h" .. level, function()
        local node = ast.heading(level, { ast.text("Title") })
        local doc = doc_of(node)
        assert.equals(
          string.format("<h%d>Title</h%d>\n", level, level),
          render(doc)
        )
      end)
    end

    it("heading with emphasis inside", function()
      local node = ast.heading(2, { ast.emphasis({ ast.text("em") }) })
      assert.equals("<h2><em>em</em></h2>\n", render(doc_of(node)))
    end)

    it("heading text is HTML-escaped", function()
      local node = ast.heading(1, { ast.text("<b>bold & bright</b>") })
      assert.equals("<h1>&lt;b&gt;bold &amp; bright&lt;/b&gt;</h1>\n", render(doc_of(node)))
    end)
  end)

  -- ─── Paragraphs ───────────────────────────────────────────────────────────

  describe("paragraph()", function()
    it("wraps text in <p>", function()
      local node = para(ast.text("Hello"))
      assert.equals("<p>Hello</p>\n", render(doc_of(node)))
    end)

    it("tight context omits <p> wrapper", function()
      local item = ast.list_item({ para(ast.text("item")) })
      local list = ast.list(false, nil, true, { item })
      assert.equals("<ul>\n<li>item</li>\n</ul>\n", render(doc_of(list)))
    end)

    it("loose context keeps <p> wrapper", function()
      local item = ast.list_item({ para(ast.text("item")) })
      local list = ast.list(false, nil, false, { item })
      assert.equals("<ul>\n<li>\n<p>item</p>\n</li>\n</ul>\n", render(doc_of(list)))
    end)
  end)

  -- ─── Code blocks ──────────────────────────────────────────────────────────

  describe("code_block()", function()
    it("without language", function()
      local node = ast.code_block(nil, "hello\n")
      assert.equals("<pre><code>hello\n</code></pre>\n", render(doc_of(node)))
    end)

    it("with language adds class attribute", function()
      local node = ast.code_block("lua", "local x = 1\n")
      assert.equals('<pre><code class="language-lua">local x = 1\n</code></pre>\n',
        render(doc_of(node)))
    end)

    it("code content is HTML-escaped", function()
      local node = ast.code_block(nil, "<script>alert(1)</script>\n")
      assert.is_true(render(doc_of(node)):find("&lt;script&gt;") ~= nil)
    end)

    it("language name is HTML-escaped", function()
      local node = ast.code_block('lang"name', "code\n")
      assert.is_true(render(doc_of(node)):find("&quot;") ~= nil)
    end)
  end)

  -- ─── Blockquotes ──────────────────────────────────────────────────────────

  describe("blockquote()", function()
    it("wraps in <blockquote>", function()
      local node = ast.blockquote({ para(ast.text("quote")) })
      assert.equals("<blockquote>\n<p>quote</p>\n</blockquote>\n",
        render(doc_of(node)))
    end)

    it("empty blockquote", function()
      local node = ast.blockquote({})
      assert.equals("<blockquote>\n</blockquote>\n", render(doc_of(node)))
    end)
  end)

  -- ─── Lists ────────────────────────────────────────────────────────────────

  describe("list()", function()
    it("tight unordered list", function()
      local items = {
        ast.list_item({ para(ast.text("one")) }),
        ast.list_item({ para(ast.text("two")) }),
      }
      local list = ast.list(false, nil, true, items)
      assert.equals("<ul>\n<li>one</li>\n<li>two</li>\n</ul>\n", render(doc_of(list)))
    end)

    it("loose unordered list", function()
      local items = {
        ast.list_item({ para(ast.text("one")) }),
        ast.list_item({ para(ast.text("two")) }),
      }
      local list = ast.list(false, nil, false, items)
      assert.equals(
        "<ul>\n<li>\n<p>one</p>\n</li>\n<li>\n<p>two</p>\n</li>\n</ul>\n",
        render(doc_of(list)))
    end)

    it("ordered list starting at 1 (no start attribute)", function()
      local items = { ast.list_item({ para(ast.text("item")) }) }
      local list = ast.list(true, 1, true, items)
      assert.equals("<ol>\n<li>item</li>\n</ol>\n", render(doc_of(list)))
    end)

    it("ordered list starting at 3 (start attribute)", function()
      local items = { ast.list_item({ para(ast.text("item")) }) }
      local list = ast.list(true, 3, true, items)
      assert.equals('<ol start="3">\n<li>item</li>\n</ol>\n', render(doc_of(list)))
    end)

    it("empty list item", function()
      local list = ast.list(false, nil, true, { ast.list_item({}) })
      assert.equals("<ul>\n<li></li>\n</ul>\n", render(doc_of(list)))
    end)

    it("task list item", function()
      local list = ast.list(false, nil, true, {
        ast.task_item(true, { para(ast.text("done")) })
      })
      assert.equals('<ul>\n<li><input type="checkbox" disabled="" checked="" /> done</li>\n</ul>\n', render(doc_of(list)))
    end)
  end)

  describe("table()", function()
    it("renders header and body rows", function()
      local table_node = ast.table({ nil }, {
        ast.table_row(true, { ast.table_cell({ ast.text("A") }) }),
        ast.table_row(false, { ast.table_cell({ ast.text("B") }) }),
      })
      assert.equals("<table>\n<thead>\n<tr>\n<th>A</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td>B</td>\n</tr>\n</tbody>\n</table>\n", render(doc_of(table_node)))
    end)
  end)

  -- ─── Thematic break ───────────────────────────────────────────────────────

  describe("thematic_break()", function()
    it("renders as <hr />", function()
      local node = ast.thematic_break()
      assert.equals("<hr />\n", render(doc_of(node)))
    end)
  end)

  -- ─── Raw block ────────────────────────────────────────────────────────────

  describe("raw_block()", function()
    it("html format passes through", function()
      local node = ast.raw_block("html", "<div>test</div>\n")
      assert.equals("<div>test</div>\n", render(doc_of(node)))
    end)

    it("non-html format is skipped", function()
      local node = ast.raw_block("latex", "\\textbf{bold}\n")
      assert.equals("", render(doc_of(node)))
    end)

    it("sanitize option suppresses raw html", function()
      local node = ast.raw_block("html", "<script>evil()</script>\n")
      assert.equals("", render(doc_of(node), { sanitize = true }))
    end)
  end)

  -- ─── Inline: text ─────────────────────────────────────────────────────────

  describe("text()", function()
    it("renders plain text", function()
      assert.equals("<p>Hello</p>\n", render(doc_of(para(ast.text("Hello")))))
    end)

    it("HTML-escapes special characters", function()
      local p = para(ast.text("a & b < c > d \" e"))
      assert.equals("<p>a &amp; b &lt; c &gt; d &quot; e</p>\n", render(doc_of(p)))
    end)
  end)

  -- ─── Inline: emphasis / strong ────────────────────────────────────────────

  describe("emphasis()", function()
    it("renders as <em>", function()
      local p = para(ast.emphasis({ ast.text("hi") }))
      assert.equals("<p><em>hi</em></p>\n", render(doc_of(p)))
    end)
  end)

  describe("strong()", function()
    it("renders as <strong>", function()
      local p = para(ast.strong({ ast.text("bold") }))
      assert.equals("<p><strong>bold</strong></p>\n", render(doc_of(p)))
    end)
  end)

  describe("strikethrough()", function()
    it("renders as <del>", function()
      local p = para(ast.strikethrough({ ast.text("gone") }))
      assert.equals("<p><del>gone</del></p>\n", render(doc_of(p)))
    end)
  end)

  -- ─── Inline: code span ────────────────────────────────────────────────────

  describe("code_span()", function()
    it("renders as <code>", function()
      local p = para(ast.code_span("x + 1"))
      assert.equals("<p><code>x + 1</code></p>\n", render(doc_of(p)))
    end)

    it("escapes HTML in code span", function()
      local p = para(ast.code_span("<script>"))
      assert.equals("<p><code>&lt;script&gt;</code></p>\n", render(doc_of(p)))
    end)
  end)

  -- ─── Inline: link ─────────────────────────────────────────────────────────

  describe("link()", function()
    it("renders as <a href>", function()
      local p = para(ast.link("https://example.com", nil, { ast.text("click") }))
      assert.equals('<p><a href="https://example.com">click</a></p>\n', render(doc_of(p)))
    end)

    it("link with title", function()
      local p = para(ast.link("/", "Home page", { ast.text("Home") }))
      assert.equals('<p><a href="/" title="Home page">Home</a></p>\n', render(doc_of(p)))
    end)

    it("dangerous scheme is sanitized to empty", function()
      local p = para(ast.link("javascript:evil()", nil, { ast.text("x") }))
      assert.equals('<p><a href="">x</a></p>\n', render(doc_of(p)))
    end)

    it("href is HTML-escaped", function()
      local p = para(ast.link('/path?a=1&b=2', nil, { ast.text("link") }))
      assert.is_true(render(doc_of(p)):find("&amp;") ~= nil)
    end)
  end)

  -- ─── Inline: image ────────────────────────────────────────────────────────

  describe("image()", function()
    it("renders as <img>", function()
      local p = para(ast.image("cat.png", nil, "a cat"))
      assert.equals('<p><img src="cat.png" alt="a cat" /></p>\n', render(doc_of(p)))
    end)

    it("image with title", function()
      local p = para(ast.image("cat.png", "Kitty", "cat"))
      assert.equals('<p><img src="cat.png" alt="cat" title="Kitty" /></p>\n',
        render(doc_of(p)))
    end)
  end)

  -- ─── Inline: autolink ─────────────────────────────────────────────────────

  describe("autolink()", function()
    it("URL autolink", function()
      local p = para(ast.autolink("https://example.com", false))
      assert.equals('<p><a href="https://example.com">https://example.com</a></p>\n',
        render(doc_of(p)))
    end)

    it("email autolink", function()
      local p = para(ast.autolink("user@example.com", true))
      assert.is_true(render(doc_of(p)):find('href="mailto:') ~= nil)
    end)
  end)

  -- ─── Inline: raw_inline ───────────────────────────────────────────────────

  describe("raw_inline()", function()
    it("html format passes through verbatim", function()
      local p = para(ast.raw_inline("html", "<em>hi</em>"))
      assert.equals("<p><em>hi</em></p>\n", render(doc_of(p)))
    end)

    it("non-html format is skipped", function()
      local p = para(ast.raw_inline("latex", "\\textbf{x}"))
      assert.equals("<p></p>\n", render(doc_of(p)))
    end)

    it("sanitize suppresses raw html inline", function()
      local p = para(ast.raw_inline("html", "<script>"))
      assert.equals("<p></p>\n", render(doc_of(p), { sanitize = true }))
    end)
  end)

  -- ─── Inline: breaks ───────────────────────────────────────────────────────

  describe("hard_break()", function()
    it("renders as <br />\\n", function()
      local p = para(ast.text("a"), ast.hard_break(), ast.text("b"))
      assert.equals("<p>a<br />\nb</p>\n", render(doc_of(p)))
    end)
  end)

  describe("soft_break()", function()
    it("renders as newline", function()
      local p = para(ast.text("a"), ast.soft_break(), ast.text("b"))
      assert.equals("<p>a\nb</p>\n", render(doc_of(p)))
    end)
  end)

  -- ─── URL sanitization ─────────────────────────────────────────────────────

  describe("URL sanitization", function()
    it("javascript: scheme is blocked", function()
      local p = para(ast.link("javascript:alert(1)", nil, { ast.text("x") }))
      assert.equals('<p><a href="">x</a></p>\n', render(doc_of(p)))
    end)

    it("vbscript: scheme is blocked", function()
      local p = para(ast.link("vbscript:MsgBox()", nil, { ast.text("x") }))
      assert.equals('<p><a href="">x</a></p>\n', render(doc_of(p)))
    end)

    it("data: scheme is blocked", function()
      local p = para(ast.link("data:text/html,<h1>hi</h1>", nil, { ast.text("x") }))
      assert.equals('<p><a href="">x</a></p>\n', render(doc_of(p)))
    end)

    it("http: scheme is allowed", function()
      local p = para(ast.link("http://safe.com", nil, { ast.text("x") }))
      assert.is_true(render(doc_of(p)):find('href="http://safe.com"') ~= nil)
    end)

    it("relative URL is allowed", function()
      local p = para(ast.link("/local/path", nil, { ast.text("x") }))
      assert.is_true(render(doc_of(p)):find('href="/local/path"') ~= nil)
    end)
  end)

end)
