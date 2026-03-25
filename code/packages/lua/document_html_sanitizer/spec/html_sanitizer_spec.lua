-- spec/html_sanitizer_spec.lua
-- ==============================
--
-- Busted test suite for coding_adventures.document_html_sanitizer.
--
-- Covers:
--   * Script element removal (with and without attributes)
--   * Event handler attribute removal (onclick, onload, etc.)
--   * URL sanitization in href and src attributes
--   * CSS expression injection prevention
--   * HTML comment stripping
--   * srcdoc and formaction attribute removal
--   * HTML_PASSTHROUGH passes everything through
--   * HTML_RELAXED drops fewer elements than HTML_STRICT
--   * XSS bypass vectors (uppercase, null bytes, zero-width chars)
--   * URL scheme allowlist enforcement

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local S = require("coding_adventures.document_html_sanitizer")

describe("document_html_sanitizer", function()

  -- ─── Passthrough ────────────────────────────────────────────────────────────

  describe("HTML_PASSTHROUGH", function()
    it("passes script tags unchanged", function()
      local html = "<script>alert(1)</script>"
      assert.equals(html, S.sanitize_html(html, S.HTML_PASSTHROUGH))
    end)

    it("passes event handlers unchanged", function()
      local html = '<img src="x.png" onload="alert(1)">'
      assert.equals(html, S.sanitize_html(html, S.HTML_PASSTHROUGH))
    end)

    it("passes javascript: href unchanged", function()
      local html = '<a href="javascript:alert(1)">click</a>'
      assert.equals(html, S.sanitize_html(html, S.HTML_PASSTHROUGH))
    end)

    it("passes comments unchanged", function()
      local html = "<!-- secret -->hello"
      assert.equals(html, S.sanitize_html(html, S.HTML_PASSTHROUGH))
    end)

    it("passes empty string", function()
      assert.equals("", S.sanitize_html("", S.HTML_PASSTHROUGH))
    end)

    it("handles nil input", function()
      assert.equals("", S.sanitize_html(nil, S.HTML_PASSTHROUGH))
    end)
  end)

  -- ─── Script element removal ──────────────────────────────────────────────────

  describe("script element removal (HTML_STRICT)", function()
    it("removes <script>…</script>", function()
      local result = S.sanitize_html(
        "<p>Safe</p><script>alert(1)</script>",
        S.HTML_STRICT
      )
      assert.falsy(result:find("<script"))
      assert.falsy(result:find("alert(1)"))
      assert.truthy(result:find("<p>Safe</p>"))
    end)

    it("removes <script> with src attribute", function()
      local result = S.sanitize_html(
        '<p>ok</p><script src="https://evil.com/xss.js"></script>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("evil.com"))
    end)

    it("removes <SCRIPT> (uppercase tag)", function()
      -- Our sanitizer lower-cases the tag name when matching
      local result = S.sanitize_html(
        "<SCRIPT>alert(1)</SCRIPT>",
        S.HTML_STRICT
      )
      assert.falsy(result:lower():find("script"))
      assert.falsy(result:find("alert"))
    end)

    it("removes <style>…</style>", function()
      local result = S.sanitize_html(
        "<style>body{color:red}</style><p>text</p>",
        S.HTML_STRICT
      )
      assert.falsy(result:find("<style"))
      assert.falsy(result:find("body{color:red}"))
    end)

    it("removes <iframe>…</iframe>", function()
      local result = S.sanitize_html(
        '<iframe src="https://evil.com"></iframe><p>safe</p>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("iframe"))
      assert.falsy(result:find("evil.com"))
    end)

    it("removes <form>…</form>", function()
      local result = S.sanitize_html(
        "<form><input type='hidden'></form><p>ok</p>",
        S.HTML_STRICT
      )
      assert.falsy(result:find("form"))
    end)

    it("removes <meta> tag", function()
      local result = S.sanitize_html(
        '<meta http-equiv="refresh" content="0;url=https://evil.com"><p>ok</p>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("meta"))
      assert.falsy(result:find("evil.com"))
    end)
  end)

  -- ─── Event handler removal ───────────────────────────────────────────────────

  describe("event handler attribute removal", function()
    it("removes onload from <img>", function()
      local result = S.sanitize_html(
        '<img src="x.png" onload="alert(1)">',
        S.HTML_STRICT
      )
      assert.falsy(result:find("onload"))
      assert.falsy(result:find("alert"))
      -- src attribute should remain
      assert.truthy(result:find('src="x.png"') or result:find("src=x.png"))
    end)

    it("removes onclick from <a>", function()
      local result = S.sanitize_html(
        '<a onclick="alert(1)">click</a>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("onclick"))
    end)

    it("removes onfocus from <div>", function()
      local result = S.sanitize_html(
        '<div onfocus="alert(1)" tabindex="0">text</div>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("onfocus"))
    end)

    it("removes onload from <svg>", function()
      local result = S.sanitize_html(
        '<svg onload="alert(1)"></svg>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("onload"))
    end)

    it("removes onerror from <img>", function()
      local result = S.sanitize_html(
        '<img src="x" onerror="alert(1)">',
        S.HTML_STRICT
      )
      assert.falsy(result:find("onerror"))
    end)
  end)

  -- ─── srcdoc and formaction removal ───────────────────────────────────────────

  describe("srcdoc and formaction removal", function()
    it("removes srcdoc attribute", function()
      local result = S.sanitize_html(
        '<iframe srcdoc="<script>alert(1)</script>"></iframe>',
        S.HTML_STRICT
      )
      -- iframe itself is dropped by HTML_STRICT
      assert.falsy(result:find("srcdoc"))
    end)

    it("removes formaction attribute from input", function()
      -- Use HTML_RELAXED so input isn't dropped (it would be by STRICT)
      -- HTML_RELAXED still drops on* and srcdoc/formaction
      local result = S.sanitize_html(
        '<input type="submit" formaction="https://evil.com">',
        S.HTML_RELAXED
      )
      assert.falsy(result:find("formaction"))
    end)
  end)

  -- ─── URL sanitization in href ─────────────────────────────────────────────

  describe("href URL sanitization", function()
    it("blanks javascript: href", function()
      local result = S.sanitize_html(
        '<a href="javascript:alert(1)">click</a>',
        S.HTML_STRICT
      )
      -- href should be empty or attribute removed
      assert.falsy(result:find("javascript"))
    end)

    it("keeps https: href", function()
      local result = S.sanitize_html(
        '<a href="https://example.com">link</a>',
        S.HTML_STRICT
      )
      assert.truthy(result:find("https://example.com"))
    end)

    it("keeps relative href", function()
      local result = S.sanitize_html(
        '<a href="/about">about</a>',
        S.HTML_STRICT
      )
      assert.truthy(result:find("/about"))
    end)

    it("blanks vbscript: href", function()
      local result = S.sanitize_html(
        '<a href="vbscript:MsgBox(1)">click</a>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("vbscript"))
    end)

    it("blanks data: href", function()
      -- Use a data: URL without angle brackets to avoid the tag pattern
      -- greedily stopping at the first > inside the attribute value.
      local result = S.sanitize_html(
        '<a href="data:text/plain,hello">click</a>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("data:"))
    end)
  end)

  -- ─── URL sanitization in src ──────────────────────────────────────────────

  describe("src URL sanitization", function()
    it("blanks javascript: src in img", function()
      local result = S.sanitize_html(
        '<img src="javascript:alert(1)" alt="x">',
        S.HTML_STRICT
      )
      assert.falsy(result:find("javascript"))
    end)

    it("keeps https: src in img", function()
      local result = S.sanitize_html(
        '<img src="https://example.com/img.png" alt="img">',
        S.HTML_STRICT
      )
      assert.truthy(result:find("https://example.com/img.png"))
    end)
  end)

  -- ─── Control character URL bypass vectors ────────────────────────────────────

  describe("control character URL bypass vectors", function()
    it("blocks javascript: with embedded null byte (java\\x00script:)", function()
      local url = "java\000script:alert(1)"
      local html = '<a href="' .. url .. '">click</a>'
      local result = S.sanitize_html(html, S.HTML_STRICT)
      assert.falsy(result:find("javascript"))
    end)

    it("blocks javascript: with carriage return (java\\rscript:)", function()
      local html = '<a href="java\rscript:alert(1)">click</a>'
      local result = S.sanitize_html(html, S.HTML_STRICT)
      -- After stripping the CR, becomes javascript: which is blocked
      assert.falsy(result:find("script:"))
    end)

    it("blocks javascript: with zero-width space", function()
      local url = "\226\128\139javascript:alert(1)"
      local html = '<a href="' .. url .. '">click</a>'
      local result = S.sanitize_html(html, S.HTML_STRICT)
      assert.falsy(result:find("javascript"))
    end)
  end)

  -- ─── CSS expression injection ─────────────────────────────────────────────

  describe("CSS expression injection (sanitize_style_attributes=true)", function()
    it("strips style with expression()", function()
      local result = S.sanitize_html(
        '<p style="width:expression(alert(1))">x</p>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("expression"))
      assert.falsy(result:find("style="))
    end)

    it("strips style with background:url(javascript:...)", function()
      local result = S.sanitize_html(
        '<p style="background:url(javascript:alert(1))">x</p>',
        S.HTML_STRICT
      )
      -- style attribute should be removed
      assert.falsy(result:find("style="))
    end)

    it("keeps style with safe url(http://...)", function()
      local result = S.sanitize_html(
        '<p style="background:url(http://example.com/bg.png)">x</p>',
        S.HTML_STRICT
      )
      -- style attribute should be kept
      assert.truthy(result:find("style="))
    end)

    it("keeps safe style (no url, no expression)", function()
      local result = S.sanitize_html(
        '<p style="color:red;font-size:12px">x</p>',
        S.HTML_STRICT
      )
      assert.truthy(result:find("style="))
    end)
  end)

  -- ─── Comment stripping ────────────────────────────────────────────────────

  describe("HTML comment stripping", function()
    it("removes comments when drop_comments=true", function()
      local result = S.sanitize_html(
        "<!-- comment --><p>ok</p>",
        S.HTML_STRICT
      )
      assert.falsy(result:find("<!%-%-"))
      assert.truthy(result:find("<p>ok</p>"))
    end)

    it("removes conditional comment with script", function()
      local result = S.sanitize_html(
        "<!--[if IE]><script>alert(1)</script><![endif]--><p>ok</p>",
        S.HTML_STRICT
      )
      assert.falsy(result:find("alert"))
    end)

    it("keeps comments when drop_comments=false", function()
      local result = S.sanitize_html(
        "<!-- keep me --><p>ok</p>",
        S.HTML_RELAXED
      )
      assert.truthy(result:find("<!%-%-"))
    end)

    it("removes comments with embedded img/onerror XSS", function()
      local result = S.sanitize_html(
        '<!--<img src=x onerror=alert(1)>--><p>ok</p>',
        S.HTML_STRICT
      )
      assert.falsy(result:find("alert"))
      assert.falsy(result:find("<!%-%-"))
    end)
  end)

  -- ─── HTML_RELAXED preset ──────────────────────────────────────────────────

  describe("HTML_RELAXED preset", function()
    it("drops script but keeps style", function()
      local result = S.sanitize_html(
        "<style>.x{color:red}</style><p>ok</p>",
        S.HTML_RELAXED
      )
      -- style is NOT in RELAXED drop_elements
      assert.truthy(result:find("<style>"))
    end)

    it("drops script element", function()
      local result = S.sanitize_html(
        "<script>alert(1)</script><p>ok</p>",
        S.HTML_RELAXED
      )
      assert.falsy(result:find("alert"))
    end)

    it("allows ftp: URLs", function()
      local result = S.sanitize_html(
        '<a href="ftp://files.example.com">files</a>',
        S.HTML_RELAXED
      )
      assert.truthy(result:find("ftp://files.example.com"))
    end)

    it("still strips on* event handlers", function()
      local result = S.sanitize_html(
        '<div onclick="alert(1)">click</div>',
        S.HTML_RELAXED
      )
      assert.falsy(result:find("onclick"))
    end)

    it("keeps HTML comments", function()
      local result = S.sanitize_html(
        "<!-- comment --><p>ok</p>",
        S.HTML_RELAXED
      )
      assert.truthy(result:find("<!%-%-"))
    end)
  end)

  -- ─── HTML_STRICT full pipeline ────────────────────────────────────────────

  describe("HTML_STRICT full pipeline", function()
    it("sanitizes mixed dangerous HTML", function()
      local dirty = table.concat({
        "<p>Hello</p>",
        "<script>alert(1)</script>",
        '<img src="ok.png" onerror="alert(2)">',
        '<a href="javascript:alert(3)">click</a>',
        "<!-- comment -->",
        '<p style="width:expression(alert(4))">styled</p>',
      })
      local result = S.sanitize_html(dirty, S.HTML_STRICT)
      assert.falsy(result:find("alert"))
      assert.falsy(result:find("script"))
      assert.falsy(result:find("onerror"))
      assert.falsy(result:find("javascript"))
      assert.falsy(result:find("<!%-%-"))
      assert.falsy(result:find("expression"))
      assert.truthy(result:find("<p>Hello</p>"))
    end)

    it("preserves safe content", function()
      local clean = table.concat({
        "<h1>Title</h1>",
        "<p>A <strong>bold</strong> statement.</p>",
        '<a href="https://example.com">link</a>',
        '<img src="https://example.com/img.png" alt="pic">',
        "<ul><li>item 1</li><li>item 2</li></ul>",
        "<blockquote><p>quoted</p></blockquote>",
      })
      local result = S.sanitize_html(clean, S.HTML_STRICT)
      assert.truthy(result:find("<h1>Title</h1>"))
      assert.truthy(result:find("<strong>bold</strong>"))
      assert.truthy(result:find("https://example.com"))
      assert.truthy(result:find("https://example.com/img.png"))
    end)
  end)

  -- ─── URL utility functions ────────────────────────────────────────────────

  describe("url_utils (exported)", function()
    describe("strip_control_chars", function()
      it("strips null byte", function()
        assert.equals("javascript:alert(1)",
          S.strip_control_chars("java\000script:alert(1)"))
      end)
      it("strips carriage return", function()
        assert.equals("javascript:alert(1)",
          S.strip_control_chars("java\rscript:alert(1)"))
      end)
      it("strips zero-width space", function()
        assert.equals("javascript:alert(1)",
          S.strip_control_chars("\226\128\139javascript:alert(1)"))
      end)
    end)

    describe("extract_scheme", function()
      it("extracts https", function()
        assert.equals("https", S.extract_scheme("https://example.com"))
      end)
      it("returns nil for relative path", function()
        assert.is_nil(S.extract_scheme("/relative"))
      end)
      it("lowercases scheme", function()
        assert.equals("javascript", S.extract_scheme("JAVASCRIPT:alert(1)"))
      end)
    end)

    describe("is_scheme_allowed", function()
      it("blocks javascript:", function()
        assert.is_false(S.is_scheme_allowed("javascript:alert(1)", { "http", "https" }))
      end)
      it("allows https:", function()
        assert.is_true(S.is_scheme_allowed("https://ok.com", { "http", "https" }))
      end)
      it("allows relative URLs", function()
        assert.is_true(S.is_scheme_allowed("/relative", { "https" }))
      end)
    end)
  end)

end)
