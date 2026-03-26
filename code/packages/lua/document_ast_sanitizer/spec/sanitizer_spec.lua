-- spec/sanitizer_spec.lua
-- ========================
--
-- Busted test suite for coding_adventures.document_ast_sanitizer.
--
-- Covers:
--   * All policy options from the SanitizationPolicy table
--   * Every node type in the Document AST truth table
--   * XSS bypass vectors from the TE02 spec testing strategy
--   * Immutability guarantee (input not mutated)
--   * PASSTHROUGH behaves as identity
--   * Empty-children pruning

-- Adjust package.path so the test runner finds our source and the
-- document_ast dependency without installing them.
package.path = "../src/?.lua;../src/?/init.lua;"
  .. "../../document_ast/src/?.lua;../../document_ast/src/?/init.lua;"
  .. package.path

local S   = require("coding_adventures.document_ast_sanitizer")
local ast = require("coding_adventures.document_ast")

-- ─── Convenience builders ─────────────────────────────────────────────────────

-- Build a document node wrapping one or more block children.
local function doc(...)
  return ast.document({ ... })
end

-- Build a document with a single paragraph containing one or more inlines.
local function para_doc(...)
  return ast.document({ ast.paragraph({ ... }) })
end

-- ─── describe / it blocks ────────────────────────────────────────────────────

describe("document_ast_sanitizer", function()

  -- ─── URL utilities ────────────────────────────────────────────────────────

  describe("strip_control_chars", function()
    it("removes ASCII C0 controls (\\x00-\\x1F)", function()
      -- A null byte embedded in "javascript"
      local url = "java\000script:alert(1)"
      local stripped = S.strip_control_chars(url)
      assert.falsy(stripped:find("\000", 1, true))
    end)

    it("removes carriage return", function()
      local url = "java\rscript:alert(1)"
      assert.equals("javascript:alert(1)", S.strip_control_chars(url))
    end)

    it("removes zero-width space (U+200B)", function()
      local url = "\226\128\139javascript:alert(1)"
      local stripped = S.strip_control_chars(url)
      assert.equals("javascript:alert(1)", stripped)
    end)

    it("leaves normal URLs unchanged", function()
      assert.equals("https://example.com", S.strip_control_chars("https://example.com"))
    end)
  end)

  describe("extract_scheme", function()
    it("extracts https scheme", function()
      assert.equals("https", S.extract_scheme("https://example.com"))
    end)

    it("extracts javascript scheme (lowercase)", function()
      assert.equals("javascript", S.extract_scheme("JAVASCRIPT:alert(1)"))
    end)

    it("returns nil for relative path with no colon", function()
      assert.is_nil(S.extract_scheme("/relative/path"))
    end)

    it("returns nil when colon appears after slash", function()
      assert.is_nil(S.extract_scheme("/path:with:colons"))
    end)

    it("returns nil when colon appears after question mark", function()
      assert.is_nil(S.extract_scheme("?q=foo:bar"))
    end)

    it("returns nil for empty string", function()
      assert.is_nil(S.extract_scheme(""))
    end)

    it("extracts mailto", function()
      assert.equals("mailto", S.extract_scheme("mailto:foo@example.com"))
    end)
  end)

  describe("is_scheme_allowed", function()
    it("allows any scheme when allowedUrlSchemes is false", function()
      assert.is_true(S.is_scheme_allowed("javascript:alert(1)", false))
    end)

    it("allows relative URLs regardless of scheme list", function()
      assert.is_true(S.is_scheme_allowed("/relative", { "https" }))
    end)

    it("allows http when in list", function()
      assert.is_true(S.is_scheme_allowed("http://example.com", { "http", "https" }))
    end)

    it("blocks javascript scheme", function()
      assert.is_false(S.is_scheme_allowed("javascript:alert(1)", { "http", "https" }))
    end)

    it("blocks javascript scheme with embedded null byte", function()
      -- java\x00script: should be stripped to javascript: which is blocked
      assert.is_false(S.is_scheme_allowed("java\000script:alert(1)", { "http", "https" }))
    end)

    it("blocks javascript scheme with zero-width space", function()
      local url = "\226\128\139javascript:alert(1)"
      assert.is_false(S.is_scheme_allowed(url, { "http", "https" }))
    end)

    it("blocks vbscript scheme", function()
      assert.is_false(S.is_scheme_allowed("vbscript:MsgBox(1)", { "http", "https" }))
    end)

    it("blocks data: scheme", function()
      assert.is_false(S.is_scheme_allowed("data:text/html,<b>hi</b>", { "http", "https" }))
    end)

    it("blocks blob: scheme", function()
      assert.is_false(S.is_scheme_allowed("blob:https://example.com/uuid", { "http", "https" }))
    end)
  end)

  -- ─── Immutability ─────────────────────────────────────────────────────────

  describe("immutability", function()
    it("does not mutate input document", function()
      local original = para_doc(ast.text("hello"), ast.link("javascript:alert(1)", nil, { ast.text("click") }))
      -- Deep copy the destinations so we can compare after sanitization
      local link_dest_before = original.children[1].children[2].destination

      S.sanitize(original, S.STRICT)

      -- The original link destination must be unchanged
      assert.equals(link_dest_before, original.children[1].children[2].destination)
    end)
  end)

  -- ─── PASSTHROUGH is identity ──────────────────────────────────────────────

  describe("PASSTHROUGH", function()
    it("preserves a simple text paragraph", function()
      local d = para_doc(ast.text("Hello world"))
      local out = S.sanitize(d, S.PASSTHROUGH)
      assert.equals("document", out.type)
      assert.equals(1, #out.children)
      assert.equals("paragraph", out.children[1].type)
      assert.equals("text", out.children[1].children[1].type)
      assert.equals("Hello world", out.children[1].children[1].value)
    end)

    it("preserves raw_block nodes", function()
      local d = doc(ast.raw_block("html", "<b>bold</b>"))
      local out = S.sanitize(d, S.PASSTHROUGH)
      assert.equals(1, #out.children)
      assert.equals("raw_block", out.children[1].type)
      assert.equals("<b>bold</b>", out.children[1].value)
    end)

    it("preserves javascript: links", function()
      local d = para_doc(ast.link("javascript:alert(1)", nil, { ast.text("click") }))
      local out = S.sanitize(d, S.PASSTHROUGH)
      local link = out.children[1].children[1]
      assert.equals("link", link.type)
      assert.equals("javascript:alert(1)", link.destination)
    end)

    it("preserves GFM nodes", function()
      local d = doc(
        ast.list(false, nil, true, {
          ast.task_item(true, { ast.paragraph({ ast.strikethrough({ ast.text("done") }) }) })
        }),
        ast.table({ nil }, {
          ast.table_row(true, { ast.table_cell({ ast.text("A") }) })
        })
      )
      local out = S.sanitize(d, S.PASSTHROUGH)
      assert.equals("task_item", out.children[1].children[1].type)
      assert.equals("table", out.children[2].type)
    end)
  end)

  -- ─── Raw block handling ───────────────────────────────────────────────────

  describe("raw_block handling", function()
    it("drops all raw_blocks when allowRawBlockFormats='drop-all'", function()
      local d = doc(ast.raw_block("html", "<b>hi</b>"))
      local out = S.sanitize(d, { allowRawBlockFormats = "drop-all" })
      assert.equals(0, #out.children)
    end)

    it("keeps raw_block when format in allowlist", function()
      local d = doc(ast.raw_block("html", "<b>hi</b>"))
      local out = S.sanitize(d, { allowRawBlockFormats = { "html" } })
      assert.equals(1, #out.children)
      assert.equals("raw_block", out.children[1].type)
    end)

    it("drops raw_block when format NOT in allowlist", function()
      local d = doc(ast.raw_block("latex", "\\LaTeX"))
      local out = S.sanitize(d, { allowRawBlockFormats = { "html" } })
      assert.equals(0, #out.children)
    end)

    it("keeps raw_block when allowRawBlockFormats='passthrough'", function()
      local d = doc(ast.raw_block("latex", "\\LaTeX"))
      local out = S.sanitize(d, { allowRawBlockFormats = "passthrough" })
      assert.equals(1, #out.children)
    end)
  end)

  -- ─── Raw inline handling ──────────────────────────────────────────────────

  describe("raw_inline handling", function()
    it("drops all raw_inline when allowRawInlineFormats='drop-all'", function()
      local d = para_doc(ast.raw_inline("html", "<b>hi</b>"))
      local out = S.sanitize(d, { allowRawInlineFormats = "drop-all" })
      -- paragraph is empty → dropped → document has 0 children
      assert.equals(0, #out.children)
    end)

    it("keeps raw_inline when format in allowlist", function()
      local d = para_doc(ast.raw_inline("html", "<b>hi</b>"))
      local out = S.sanitize(d, { allowRawInlineFormats = { "html" } })
      assert.equals(1, #out.children)
      assert.equals("raw_inline", out.children[1].children[1].type)
    end)

    it("STRICT drops raw_inline nodes", function()
      local d = para_doc(ast.text("before"), ast.raw_inline("html", "<script>"))
      local out = S.sanitize(d, S.STRICT)
      local para = out.children[1]
      assert.equals(1, #para.children)
      assert.equals("text", para.children[1].type)
    end)
  end)

  -- ─── URL scheme handling ──────────────────────────────────────────────────

  describe("URL scheme handling in links", function()
    it("blocks javascript: URL in link (STRICT)", function()
      local d = para_doc(ast.link("javascript:alert(1)", nil, { ast.text("click") }))
      local out = S.sanitize(d, S.STRICT)
      local link = out.children[1].children[1]
      assert.equals("link", link.type)
      assert.equals("", link.destination)
    end)

    it("blocks JAVASCRIPT: (uppercase) in link", function()
      local d = para_doc(ast.link("JAVASCRIPT:alert(1)", nil, { ast.text("x") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)

    it("allows https: URL in link (STRICT)", function()
      local d = para_doc(ast.link("https://example.com", nil, { ast.text("site") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("https://example.com", out.children[1].children[1].destination)
    end)

    it("allows relative URL in link (STRICT)", function()
      local d = para_doc(ast.link("/about", nil, { ast.text("about") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("/about", out.children[1].children[1].destination)
    end)

    it("blocks data: URL in link", function()
      local d = para_doc(ast.link("data:text/html,<b>hi</b>", nil, { ast.text("x") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)

    it("blocks vbscript: URL in link", function()
      local d = para_doc(ast.link("vbscript:MsgBox(1)", nil, { ast.text("x") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)

    it("blocks blob: URL in link", function()
      local d = para_doc(ast.link("blob:https://origin/uuid", nil, { ast.text("x") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)
  end)

  describe("URL scheme handling in autolinks", function()
    it("drops autolink with javascript: scheme", function()
      local d = para_doc(ast.autolink("javascript:alert(1)", false))
      local out = S.sanitize(d, S.STRICT)
      -- autolink dropped → paragraph empty → paragraph dropped
      assert.equals(0, #out.children)
    end)

    it("keeps autolink with https: scheme", function()
      local d = para_doc(ast.autolink("https://example.com", false))
      local out = S.sanitize(d, S.STRICT)
      assert.equals(1, #out.children)
      assert.equals("autolink", out.children[1].children[1].type)
    end)

    it("drops autolink with data: scheme (STRICT)", function()
      local d = para_doc(ast.autolink("data:text/html,x", false))
      local out = S.sanitize(d, S.STRICT)
      assert.equals(0, #out.children)
    end)
  end)

  describe("URL scheme handling in images", function()
    it("blocks javascript: URL in image destination", function()
      local d = doc(ast.paragraph({
        ast.image("javascript:alert(1)", nil, "alt text")
      }))
      local out = S.sanitize(d, S.RELAXED)
      local img = out.children[1].children[1]
      assert.equals("image", img.type)
      assert.equals("", img.destination)
    end)

    it("allows https: URL in image (STRICT, transformImageToText=false)", function()
      local policy = {
        allowRawBlockFormats  = "drop-all",
        allowRawInlineFormats = "drop-all",
        allowedUrlSchemes     = { "http", "https", "mailto" },
        dropImages            = false,
        transformImageToText  = false,
      }
      local d = doc(ast.paragraph({ ast.image("https://img.example.com/cat.png", nil, "cat") }))
      local out = S.sanitize(d, policy)
      local img = out.children[1].children[1]
      assert.equals("https://img.example.com/cat.png", img.destination)
    end)
  end)

  -- ─── Heading level clamping ───────────────────────────────────────────────

  describe("heading level clamping", function()
    it("drops all headings when maxHeadingLevel='drop'", function()
      local d = doc(ast.heading(1, { ast.text("Title") }))
      local out = S.sanitize(d, { maxHeadingLevel = "drop" })
      assert.equals(0, #out.children)
    end)

    it("clamps h1 up to minHeadingLevel=2", function()
      local d = doc(ast.heading(1, { ast.text("Title") }))
      local out = S.sanitize(d, { minHeadingLevel = 2 })
      assert.equals(2, out.children[1].level)
    end)

    it("does not change h3 when minHeadingLevel=2", function()
      local d = doc(ast.heading(3, { ast.text("Sub") }))
      local out = S.sanitize(d, { minHeadingLevel = 2 })
      assert.equals(3, out.children[1].level)
    end)

    it("clamps h5 down to maxHeadingLevel=3", function()
      local d = doc(ast.heading(5, { ast.text("Deep") }))
      local out = S.sanitize(d, { maxHeadingLevel = 3 })
      assert.equals(3, out.children[1].level)
    end)

    it("STRICT clamps h1 to h2", function()
      local d = doc(ast.heading(1, { ast.text("Page") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals(2, out.children[1].level)
    end)

    it("STRICT leaves h3 unchanged", function()
      local d = doc(ast.heading(3, { ast.text("Section") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals(3, out.children[1].level)
    end)
  end)

  -- ─── Image policy ─────────────────────────────────────────────────────────

  describe("image handling", function()
    it("drops image when dropImages=true", function()
      local d = doc(ast.paragraph({
        ast.image("https://example.com/img.png", nil, "photo")
      }))
      local out = S.sanitize(d, { dropImages = true })
      -- image dropped → paragraph empty → paragraph dropped
      assert.equals(0, #out.children)
    end)

    it("transforms image to text when transformImageToText=true", function()
      local d = doc(ast.paragraph({
        ast.image("https://example.com/img.png", nil, "a cute cat")
      }))
      local out = S.sanitize(d, { transformImageToText = true })
      local inline = out.children[1].children[1]
      assert.equals("text", inline.type)
      assert.equals("a cute cat", inline.value)
    end)

    it("STRICT transforms image to alt text", function()
      local d = doc(ast.paragraph({
        ast.image("https://example.com/img.png", nil, "product photo")
      }))
      local out = S.sanitize(d, S.STRICT)
      local inline = out.children[1].children[1]
      assert.equals("text", inline.type)
      assert.equals("product photo", inline.value)
    end)

    it("dropImages takes precedence over transformImageToText", function()
      local d = doc(ast.paragraph({
        ast.image("https://example.com/img.png", nil, "alt")
      }))
      local out = S.sanitize(d, { dropImages = true, transformImageToText = true })
      assert.equals(0, #out.children)
    end)
  end)

  -- ─── Link dropping / child promotion ─────────────────────────────────────

  describe("link dropping", function()
    it("promotes link children when dropLinks=true", function()
      local d = para_doc(ast.link("https://example.com", nil, { ast.text("click here") }))
      local out = S.sanitize(d, { dropLinks = true })
      -- link is gone; text child promoted
      local para = out.children[1]
      assert.equals(1, #para.children)
      assert.equals("text", para.children[1].type)
      assert.equals("click here", para.children[1].value)
    end)

    it("promotes multiple children from a link", function()
      local d = para_doc(ast.link("https://example.com", nil, {
        ast.text("see "),
        ast.emphasis({ ast.text("this") }),
        ast.text(" page"),
      }))
      local out = S.sanitize(d, { dropLinks = true })
      local para = out.children[1]
      assert.equals(3, #para.children)
    end)
  end)

  -- ─── Blockquote policy ────────────────────────────────────────────────────

  describe("blockquote handling", function()
    it("drops blockquote when dropBlockquotes=true", function()
      local d = doc(ast.blockquote({ ast.paragraph({ ast.text("quote") }) }))
      local out = S.sanitize(d, { dropBlockquotes = true })
      assert.equals(0, #out.children)
    end)

    it("keeps blockquote when dropBlockquotes=false", function()
      local d = doc(ast.blockquote({ ast.paragraph({ ast.text("quote") }) }))
      local out = S.sanitize(d, { dropBlockquotes = false })
      assert.equals("blockquote", out.children[1].type)
    end)
  end)

  -- ─── Code block policy ────────────────────────────────────────────────────

  describe("code_block handling", function()
    it("drops code_block when dropCodeBlocks=true", function()
      local d = doc(ast.code_block("lua", "local x = 1\n"))
      local out = S.sanitize(d, { dropCodeBlocks = true })
      assert.equals(0, #out.children)
    end)

    it("keeps code_block when dropCodeBlocks=false", function()
      local d = doc(ast.code_block("lua", "local x = 1\n"))
      local out = S.sanitize(d, { dropCodeBlocks = false })
      assert.equals("code_block", out.children[1].type)
      assert.equals("local x = 1\n", out.children[1].value)
    end)
  end)

  -- ─── code_span transformation ─────────────────────────────────────────────

  describe("code_span handling", function()
    it("transforms code_span to text when transformCodeSpanToText=true", function()
      local d = para_doc(ast.code_span("local x = 1"))
      local out = S.sanitize(d, { transformCodeSpanToText = true })
      local inline = out.children[1].children[1]
      assert.equals("text", inline.type)
      assert.equals("local x = 1", inline.value)
    end)

    it("keeps code_span when transformCodeSpanToText=false", function()
      local d = para_doc(ast.code_span("local x = 1"))
      local out = S.sanitize(d, { transformCodeSpanToText = false })
      assert.equals("code_span", out.children[1].children[1].type)
    end)
  end)

  -- ─── Empty-children pruning ───────────────────────────────────────────────

  describe("empty-children pruning", function()
    it("drops paragraph when only child (raw_inline) is dropped", function()
      local d = para_doc(ast.raw_inline("html", "<script>x</script>"))
      local out = S.sanitize(d, { allowRawInlineFormats = "drop-all" })
      assert.equals(0, #out.children)
    end)

    it("keeps document node even when all children are dropped", function()
      local d = doc(ast.raw_block("html", "<script>x</script>"))
      local out = S.sanitize(d, { allowRawBlockFormats = "drop-all" })
      assert.equals("document", out.type)
      assert.equals(0, #out.children)
    end)

    it("drops emphasis when all inline children are dropped", function()
      local d = para_doc(ast.emphasis({ ast.raw_inline("html", "<b>bold</b>") }))
      local out = S.sanitize(d, { allowRawInlineFormats = "drop-all" })
      -- empty emphasis → dropped → empty para → dropped
      assert.equals(0, #out.children)
    end)

    it("drops list when all items are dropped", function()
      local d = doc(ast.list(false, nil, true, {
        ast.list_item({ ast.paragraph({ ast.raw_inline("html", "<script>") }) })
      }))
      local out = S.sanitize(d, { allowRawInlineFormats = "drop-all" })
      assert.equals(0, #out.children)
    end)
  end)

  -- ─── Leaf node passthrough ────────────────────────────────────────────────

  describe("leaf nodes", function()
    it("always keeps thematic_break", function()
      local d = doc(ast.thematic_break())
      local out = S.sanitize(d, S.STRICT)
      assert.equals("thematic_break", out.children[1].type)
    end)

    it("always keeps hard_break", function()
      local d = para_doc(ast.hard_break())
      local out = S.sanitize(d, S.STRICT)
      assert.equals("hard_break", out.children[1].children[1].type)
    end)

    it("always keeps soft_break", function()
      local d = para_doc(ast.soft_break())
      local out = S.sanitize(d, S.STRICT)
      assert.equals("soft_break", out.children[1].children[1].type)
    end)
  end)

  -- ─── Nested structure ─────────────────────────────────────────────────────

  describe("nested structures", function()
    it("recurses into list items", function()
      local d = doc(ast.list(false, nil, true, {
        ast.list_item({ ast.paragraph({ ast.text("item"), ast.raw_inline("html", "<b>") }) }),
      }))
      local out = S.sanitize(d, { allowRawInlineFormats = "drop-all" })
      local item_para = out.children[1].children[1].children[1]
      assert.equals(1, #item_para.children)
      assert.equals("text", item_para.children[1].type)
    end)

    it("recurses into blockquotes", function()
      local d = doc(ast.blockquote({
        ast.paragraph({ ast.raw_inline("html", "<script>") })
      }))
      local out = S.sanitize(d, { allowRawInlineFormats = "drop-all" })
      -- raw_inline dropped → empty para → para dropped → empty blockquote → blockquote dropped
      assert.equals(0, #out.children)
    end)

    it("keeps nested text and emphasis under STRICT", function()
      local d = para_doc(
        ast.emphasis({ ast.text("important") }),
        ast.text(" note")
      )
      local out = S.sanitize(d, S.STRICT)
      local para = out.children[1]
      assert.equals(2, #para.children)
      assert.equals("emphasis", para.children[1].type)
      assert.equals("text", para.children[2].type)
    end)
  end)

  -- ─── STRICT preset end-to-end ────────────────────────────────────────────

  describe("STRICT preset", function()
    it("drops script injection via raw_block", function()
      local d = doc(ast.raw_block("html", "<script>alert(1)</script>"))
      local out = S.sanitize(d, S.STRICT)
      assert.equals(0, #out.children)
    end)

    it("sanitizes javascript: link", function()
      local d = para_doc(ast.link("javascript:alert(1)", nil, { ast.text("xss") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)

    it("sanitizes JAVASCRIPT: (uppercase) link", function()
      local d = para_doc(ast.link("JAVASCRIPT:alert(1)", nil, { ast.text("xss") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)

    it("sanitizes link with null byte in scheme (java\\x00script:)", function()
      local d = para_doc(ast.link("java\000script:alert(1)", nil, { ast.text("xss") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)

    it("sanitizes link with zero-width space in scheme", function()
      local url = "\226\128\139javascript:alert(1)"
      local d = para_doc(ast.link(url, nil, { ast.text("xss") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("", out.children[1].children[1].destination)
    end)

    it("keeps https: link", function()
      local d = para_doc(ast.link("https://example.com", nil, { ast.text("ok") }))
      local out = S.sanitize(d, S.STRICT)
      assert.equals("https://example.com", out.children[1].children[1].destination)
    end)

    it("drops autolink with data: scheme", function()
      local d = para_doc(ast.autolink("data:text/html,<script>alert(1)</script>", false))
      local out = S.sanitize(d, S.STRICT)
      assert.equals(0, #out.children)
    end)

    it("converts image to alt text", function()
      local d = para_doc(ast.image("https://evil.com/track.gif", nil, "my photo"))
      local out = S.sanitize(d, S.STRICT)
      local inline = out.children[1].children[1]
      assert.equals("text", inline.type)
      assert.equals("my photo", inline.value)
    end)
  end)

  -- ─── RELAXED preset ───────────────────────────────────────────────────────

  describe("RELAXED preset", function()
    it("keeps html raw_block", function()
      local d = doc(ast.raw_block("html", "<div>safe</div>"))
      local out = S.sanitize(d, S.RELAXED)
      assert.equals(1, #out.children)
    end)

    it("drops latex raw_block", function()
      local d = doc(ast.raw_block("latex", "\\LaTeX"))
      local out = S.sanitize(d, S.RELAXED)
      assert.equals(0, #out.children)
    end)

    it("keeps ftp: URL in link", function()
      local d = para_doc(ast.link("ftp://files.example.com", nil, { ast.text("files") }))
      local out = S.sanitize(d, S.RELAXED)
      assert.equals("ftp://files.example.com", out.children[1].children[1].destination)
    end)

    it("keeps images unchanged", function()
      local d = para_doc(ast.image("https://example.com/img.png", nil, "photo"))
      local out = S.sanitize(d, S.RELAXED)
      local img = out.children[1].children[1]
      assert.equals("image", img.type)
      assert.equals("https://example.com/img.png", img.destination)
    end)
  end)

  -- ─── with_defaults helper ─────────────────────────────────────────────────

  describe("with_defaults", function()
    it("returns PASSTHROUGH when no overrides given", function()
      local p = S.with_defaults({})
      assert.equals("passthrough", p.allowRawBlockFormats)
    end)

    it("merges overrides on top of PASSTHROUGH", function()
      local p = S.with_defaults({ dropLinks = true, minHeadingLevel = 2 })
      assert.is_true(p.dropLinks)
      assert.equals(2, p.minHeadingLevel)
      -- unoveridden fields keep PASSTHROUGH values
      assert.equals("passthrough", p.allowRawBlockFormats)
    end)
  end)

end)
