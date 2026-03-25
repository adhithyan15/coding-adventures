defmodule CodingAdventures.DocumentHtmlSanitizerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.DocumentHtmlSanitizer
  alias CodingAdventures.DocumentHtmlSanitizer.Policy
  alias CodingAdventures.DocumentHtmlSanitizer.UrlUtils

  defp sanitize(html, policy), do: DocumentHtmlSanitizer.sanitize_html(html, policy)

  # ── UrlUtils tests ────────────────────────────────────────────────────────

  describe "UrlUtils.strip_control_chars/1" do
    test "removes null bytes" do
      assert UrlUtils.strip_control_chars("java\x00script:") == "javascript:"
    end

    test "removes zero-width space" do
      assert UrlUtils.strip_control_chars("\u200Bhttps:") == "https:"
    end

    test "leaves normal URLs unchanged" do
      assert UrlUtils.strip_control_chars("https://example.com") == "https://example.com"
    end
  end

  describe "UrlUtils.scheme_allowed?/2" do
    test "allows https" do
      assert UrlUtils.scheme_allowed?("https://example.com", ["http", "https"])
    end

    test "blocks javascript:" do
      refute UrlUtils.scheme_allowed?("javascript:alert(1)", ["http", "https"])
    end

    test "allows relative URLs" do
      assert UrlUtils.scheme_allowed?("/path", ["http", "https"])
    end

    test "nil schemes allows everything" do
      assert UrlUtils.scheme_allowed?("javascript:alert(1)", nil)
    end

    test "null-byte bypass blocked" do
      refute UrlUtils.scheme_allowed?("java\x00script:alert(1)", ["http", "https"])
    end
  end

  # ── Policy presets ────────────────────────────────────────────────────────

  describe "Policy presets" do
    test "html_strict/0 has sensible defaults" do
      p = Policy.html_strict()
      assert "script" in p.drop_elements
      assert "iframe" in p.drop_elements
      assert p.drop_comments == true
      assert p.sanitize_style_attributes == true
      assert p.allowed_url_schemes == ["http", "https", "mailto"]
    end

    test "html_relaxed/0 keeps style and form" do
      p = Policy.html_relaxed()
      refute "style" in p.drop_elements
      refute "form" in p.drop_elements
      assert "iframe" in p.drop_elements
      assert p.drop_comments == false
    end

    test "html_passthrough/0 has no restrictions" do
      p = Policy.html_passthrough()
      assert p.drop_elements == []
      assert p.drop_comments == false
      assert p.sanitize_style_attributes == false
      assert p.allowed_url_schemes == nil
    end
  end

  # ── Comment stripping ─────────────────────────────────────────────────────

  describe "HTML comment handling" do
    test "drops comment when drop_comments is true" do
      assert sanitize("<!-- comment --><p>ok</p>", Policy.html_strict()) == "<p>ok</p>"
    end

    test "drops comment at end of string" do
      assert sanitize("<p>ok</p><!-- trailing -->", Policy.html_strict()) == "<p>ok</p>"
    end

    test "drops multi-line comment" do
      html = "<!--\n<img src=x onerror=alert(1)>\n--><p>safe</p>"
      assert sanitize(html, Policy.html_strict()) == "<p>safe</p>"
    end

    test "preserves comment when drop_comments is false" do
      result = sanitize("<!-- kept --><p>ok</p>", Policy.html_relaxed())
      assert result == "<!-- kept --><p>ok</p>"
    end

    test "passthrough preserves comment" do
      result = sanitize("<!-- kept -->", Policy.html_passthrough())
      assert result == "<!-- kept -->"
    end

    test "drops IE conditional comment XSS" do
      html = "<!--[if IE]><script>alert(1)</script><![endif]--><p>safe</p>"
      result = sanitize(html, Policy.html_strict())
      # Comment dropped entirely
      refute String.contains?(result, "script")
    end
  end

  # ── Element dropping ──────────────────────────────────────────────────────

  describe "element dropping" do
    test "drops script tag and content" do
      assert sanitize("<p>Safe</p><script>alert(1)</script>", Policy.html_strict()) ==
               "<p>Safe</p>"
    end

    test "drops script with src" do
      assert sanitize("<script src=\"https://evil.com/xss.js\"></script>", Policy.html_strict()) ==
               ""
    end

    test "drops uppercase SCRIPT" do
      assert sanitize("<SCRIPT>alert(1)</SCRIPT>", Policy.html_strict()) == ""
    end

    test "drops Mixed-Case Script" do
      assert sanitize("<Script>alert(1)</Script>", Policy.html_strict()) == ""
    end

    test "drops style element" do
      result = sanitize("<style>body{color:red}</style><p>ok</p>", Policy.html_strict())
      assert result == "<p>ok</p>"
    end

    test "drops iframe" do
      result = sanitize("<iframe src=\"https://evil.com\"></iframe>", Policy.html_strict())
      assert result == ""
    end

    test "drops object" do
      result = sanitize("<object data=\"x.swf\"></object>", Policy.html_strict())
      assert result == ""
    end

    test "drops embed" do
      result = sanitize("<embed src=\"x.swf\">", Policy.html_strict())
      assert result == ""
    end

    test "drops form" do
      result = sanitize("<form action=\"/login\"><input></form>", Policy.html_strict())
      assert result == ""
    end

    test "drops meta tag" do
      result = sanitize("<meta http-equiv=\"refresh\" content=\"0;url=evil.com\">", Policy.html_strict())
      assert result == ""
    end

    test "drops base tag" do
      result = sanitize("<base href=\"https://evil.com\">", Policy.html_strict())
      assert result == ""
    end

    test "drops noscript" do
      result = sanitize("<noscript><img src=x onerror=alert(1)></noscript>", Policy.html_strict())
      assert result == ""
    end

    test "passthrough keeps all elements" do
      html = "<script>alert(1)</script><p>ok</p>"
      assert sanitize(html, Policy.html_passthrough()) == html
    end

    test "safe elements kept" do
      html = "<p>Hello <strong>world</strong></p>"
      assert sanitize(html, Policy.html_strict()) == html
    end

    test "custom drop_elements list respected" do
      p = %Policy{Policy.html_passthrough() | drop_elements: ["marquee"]}
      result = sanitize("<marquee>scrolling</marquee><p>ok</p>", p)
      assert result == "<p>ok</p>"
    end
  end

  # ── Event handler stripping ───────────────────────────────────────────────

  describe "event handler attribute stripping" do
    test "strips onload from img" do
      result = sanitize("<img onload=\"alert(1)\" src=\"x.png\">", Policy.html_strict())
      refute String.contains?(result, "onload")
      assert String.contains?(result, "src=\"x.png\"")
    end

    test "strips onclick from a" do
      result = sanitize("<a onclick=\"alert(1)\">click</a>", Policy.html_strict())
      refute String.contains?(result, "onclick")
    end

    test "strips onfocus from div" do
      result = sanitize("<div onfocus=\"alert(1)\" tabindex=\"0\">", Policy.html_strict())
      refute String.contains?(result, "onfocus")
    end

    test "strips onload from svg" do
      result = sanitize("<svg onload=\"alert(1)\">", Policy.html_strict())
      refute String.contains?(result, "onload")
    end

    test "strips onerror" do
      result = sanitize("<img src=\"x\" onerror=\"alert(1)\">", Policy.html_strict())
      refute String.contains?(result, "onerror")
    end

    test "strips onmouseover" do
      result = sanitize("<p onmouseover=\"evil()\">text</p>", Policy.html_strict())
      refute String.contains?(result, "onmouseover")
    end

    test "event handler stripped case-insensitively" do
      result = sanitize("<img ONLOAD=\"alert(1)\">", Policy.html_strict())
      refute String.contains?(result, "ONLOAD")
      refute String.contains?(result, "onload")
    end

    test "passthrough still strips event handlers (baseline security, not configurable)" do
      # on* stripping is always applied regardless of policy — it's baseline security.
      # The passthrough policy disables element dropping, comment dropping,
      # and URL/style sanitization, but does NOT disable event handler stripping.
      result = sanitize("<img onload=\"alert(1)\" src=\"x.png\">", Policy.html_passthrough())
      refute String.contains?(result, "onload")
      assert String.contains?(result, "src=\"x.png\"")
    end
  end

  # ── srcdoc and formaction stripping ──────────────────────────────────────

  describe "srcdoc and formaction" do
    test "strips srcdoc from iframe (even if iframe not dropped)" do
      p = %Policy{Policy.html_strict() | drop_elements: []}
      result = sanitize("<iframe srcdoc=\"<script>alert(1)</script>\"></iframe>", p)
      refute String.contains?(result, "srcdoc")
    end

    test "strips formaction from button" do
      p = %Policy{Policy.html_strict() | drop_elements: []}
      result = sanitize("<button formaction=\"/phishing\">click</button>", p)
      refute String.contains?(result, "formaction")
    end
  end

  # ── URL scheme sanitization in href ──────────────────────────────────────

  describe "href URL sanitization" do
    test "clears javascript: href" do
      result = sanitize("<a href=\"javascript:alert(1)\">click</a>", Policy.html_strict())
      assert String.contains?(result, "href=\"\"")
      refute String.contains?(result, "javascript")
    end

    test "preserves https: href" do
      html = "<a href=\"https://example.com\">link</a>"
      assert sanitize(html, Policy.html_strict()) == html
    end

    test "preserves relative href" do
      html = "<a href=\"/about\">About</a>"
      assert sanitize(html, Policy.html_strict()) == html
    end

    test "clears data: href" do
      result = sanitize(
        "<a href=\"data:text/html,<script>alert(1)</script>\">x</a>",
        Policy.html_strict()
      )
      assert String.contains?(result, "href=\"\"")
    end

    test "clears vbscript: href" do
      result = sanitize("<a href=\"vbscript:MsgBox(1)\">x</a>", Policy.html_strict())
      assert String.contains?(result, "href=\"\"")
    end

    test "clears null-byte bypass javascript:" do
      result = sanitize("<a href=\"java\x00script:alert(1)\">x</a>", Policy.html_strict())
      assert String.contains?(result, "href=\"\"")
    end

    test "passthrough preserves javascript: href" do
      html = "<a href=\"javascript:alert(1)\">click</a>"
      assert sanitize(html, Policy.html_passthrough()) == html
    end
  end

  # ── URL scheme sanitization in src ───────────────────────────────────────

  describe "src URL sanitization" do
    test "clears javascript: src" do
      result = sanitize("<img src=\"javascript:alert(1)\">", Policy.html_strict())
      assert String.contains?(result, "src=\"\"")
    end

    test "preserves https: src" do
      html = "<img src=\"https://example.com/img.png\">"
      assert sanitize(html, Policy.html_strict()) == html
    end

    test "preserves relative src" do
      html = "<img src=\"/images/cat.png\">"
      assert sanitize(html, Policy.html_strict()) == html
    end
  end

  # ── Style attribute sanitization ──────────────────────────────────────────

  describe "style attribute sanitization" do
    test "strips style containing expression()" do
      result = sanitize("<p style=\"width:expression(alert(1))\">x</p>", Policy.html_strict())
      refute String.contains?(result, "style")
    end

    test "strips style containing url() with non-http content" do
      result = sanitize(
        "<p style=\"background:url(javascript:alert(1))\">x</p>",
        Policy.html_strict()
      )
      refute String.contains?(result, "style")
    end

    test "preserves safe style" do
      result = sanitize("<p style=\"color:red\">x</p>", Policy.html_strict())
      assert String.contains?(result, "style=\"color:red\"")
    end

    test "preserves style with https url()" do
      result = sanitize(
        "<p style=\"background:url(https://example.com/bg.png)\">x</p>",
        Policy.html_strict()
      )
      assert String.contains?(result, "style=")
    end

    test "passthrough keeps dangerous style" do
      html = "<p style=\"width:expression(alert(1))\">x</p>"
      assert sanitize(html, Policy.html_passthrough()) == html
    end
  end

  # ── Custom drop_attributes ────────────────────────────────────────────────

  describe "custom drop_attributes" do
    test "drops specified attribute from all elements" do
      p = %Policy{Policy.html_strict() | drop_attributes: ["data-evil"]}
      result = sanitize("<p data-evil=\"yes\">text</p>", p)
      refute String.contains?(result, "data-evil")
    end
  end

  # ── Full pipeline / combined XSS vectors ─────────────────────────────────

  describe "full pipeline sanitization" do
    test "mixed safe and unsafe HTML" do
      html = "<p>Hello</p><script>alert(1)</script><p>World</p>"
      assert sanitize(html, Policy.html_strict()) == "<p>Hello</p><p>World</p>"
    end

    test "comment with script content dropped" do
      html = "<!--<img src=x onerror=alert(1)>--><p>ok</p>"
      result = sanitize(html, Policy.html_strict())
      refute String.contains?(result, "onerror")
      assert result == "<p>ok</p>"
    end

    test "iframe with srcdoc dropped" do
      html = "<iframe srcdoc=\"<script>alert(1)</script>\"></iframe>"
      assert sanitize(html, Policy.html_strict()) == ""
    end

    test "img with onerror dropped (event handler stripped)" do
      html = "<img src=\"x.png\" onerror=\"alert(1)\">"
      result = sanitize(html, Policy.html_strict())
      refute String.contains?(result, "onerror")
    end

    test "anchor with javascript href cleared" do
      html = "<a href=\"javascript:alert(1)\">click</a>"
      result = sanitize(html, Policy.html_strict())
      refute String.contains?(result, "javascript")
      assert String.contains?(result, ">click</a>")
    end

    test "deeply nested content preserved while script dropped" do
      html = "<div><p><em>text</em></p><script>evil()</script></div>"
      result = sanitize(html, Policy.html_strict())
      assert result == "<div><p><em>text</em></p></div>"
    end

    test "relaxed preset keeps style elements" do
      html = "<style>p{color:red}</style><p>ok</p>"
      result = sanitize(html, Policy.html_relaxed())
      assert String.contains?(result, "<style>")
    end

    test "relaxed preset still drops script elements" do
      html = "<script>alert(1)</script><p>ok</p>"
      assert sanitize(html, Policy.html_relaxed()) == "<p>ok</p>"
    end

    test "passthrough keeps elements but still strips on* handlers" do
      # passthrough disables element dropping — script is kept
      # But on* event handler stripping is baseline and always applied
      html = "<script>alert(1)</script><p onclick=\"evil()\">ok</p>"
      result = sanitize(html, Policy.html_passthrough())
      assert String.contains?(result, "<script>alert(1)</script>")
      refute String.contains?(result, "onclick")
      assert String.contains?(result, ">ok</p>")
    end
  end

  # ── Single-quoted and unquoted attribute values ───────────────────────────

  describe "single-quoted attribute values" do
    test "clears single-quoted javascript: href" do
      result = sanitize("<a href='javascript:alert(1)'>x</a>", Policy.html_strict())
      assert String.contains?(result, "href=''")
      refute String.contains?(result, "javascript")
    end

    test "clears single-quoted javascript: src" do
      result = sanitize("<img src='javascript:alert(1)'>", Policy.html_strict())
      assert String.contains?(result, "src=''")
    end

    test "strips single-quoted expression style" do
      result = sanitize("<p style='width:expression(alert(1))'>x</p>", Policy.html_strict())
      refute String.contains?(result, "style")
    end

    test "preserves safe single-quoted href" do
      html = "<a href='https://example.com'>link</a>"
      result = sanitize(html, Policy.html_strict())
      assert String.contains?(result, "href='https://example.com'")
    end
  end

  # ── Edge cases ────────────────────────────────────────────────────────────

  describe "edge cases" do
    test "empty string" do
      assert sanitize("", Policy.html_strict()) == ""
    end

    test "plain text (no HTML)" do
      assert sanitize("hello world", Policy.html_strict()) == "hello world"
    end

    test "multiple script tags all dropped" do
      html = "<script>a()</script><p>ok</p><script>b()</script>"
      assert sanitize(html, Policy.html_strict()) == "<p>ok</p>"
    end

    test "html with only safe elements" do
      html = "<h1>Title</h1><p>Paragraph <strong>bold</strong> <em>em</em></p>"
      assert sanitize(html, Policy.html_strict()) == html
    end

    test "relaxed preset strips event handlers too" do
      result = sanitize("<p onclick=\"evil()\">text</p>", Policy.html_relaxed())
      refute String.contains?(result, "onclick")
    end
  end
end
