# frozen_string_literal: true

require "test_helper"

# Tests for CodingAdventures::DocumentHtmlSanitizer
#
# Organisation:
#   1. Policy presets smoke tests (STRICT, RELAXED, PASSTHROUGH)
#   2. Element dropping (script, iframe, etc.)
#   3. Event handler attribute stripping (on*)
#   4. URL attribute sanitization (href, src — all XSS vectors from spec)
#   5. CSS expression / style attribute sanitization
#   6. Comment stripping
#   7. Attribute dropping (srcdoc, formaction, custom)
#   8. PASSTHROUGH preserves everything
#   9. Custom policy via with()
#  10. Edge cases (empty string, plain text, deeply nested)

module CodingAdventures
  class TestDocumentHtmlSanitizer < Minitest::Test
    def san(html, policy = DocumentHtmlSanitizer::HTML_STRICT)
      DocumentHtmlSanitizer.sanitize_html(html, policy)
    end

    # ─── 1. Preset smoke tests ─────────────────────────────────────────────────

    def test_strict_basic_safe_html_unchanged
      safe = "<p>Hello <strong>world</strong></p>"
      assert_equal safe, san(safe)
    end

    def test_strict_removes_script_tag
      html = "<p>Safe</p><script>alert(1)</script>"
      result = san(html)
      refute_includes result, "<script"
      refute_includes result, "alert(1)"
      assert_includes result, "<p>Safe</p>"
    end

    def test_relaxed_allows_style_element
      html = "<style>body { color: red; }</style><p>text</p>"
      result = san(html, DocumentHtmlSanitizer::HTML_RELAXED)
      assert_includes result, "<style>"
    end

    def test_passthrough_preserves_everything
      html = "<script>alert(1)</script>"
      assert_equal html, san(html, DocumentHtmlSanitizer::HTML_PASSTHROUGH)
    end

    # ─── 2. Element dropping ───────────────────────────────────────────────────

    def test_drops_script_element_and_content
      assert_equal "<p>ok</p>", san("<p>ok</p><script>alert(1)</script>")
    end

    def test_drops_script_element_with_src
      result = san('<script src="https://evil.com/xss.js"></script>')
      refute_includes result, "script"
      refute_includes result, "evil.com"
    end

    def test_drops_uppercase_script
      result = san("<SCRIPT>alert(1)</SCRIPT>")
      refute_includes result.downcase, "script"
    end

    def test_drops_style_element
      result = san("<style>body{}</style>")
      refute_includes result, "<style"
    end

    def test_drops_iframe
      result = san("<iframe src='https://evil.com'></iframe>")
      refute_includes result, "iframe"
    end

    def test_drops_object
      result = san("<object data='plugin.swf'></object>")
      refute_includes result, "object"
    end

    def test_drops_embed
      result = san("<embed src='plugin.swf'>")
      refute_includes result, "embed"
    end

    def test_drops_form
      result = san("<form action='/phish'><input name='pw'></form>")
      refute_includes result, "form"
    end

    def test_drops_meta
      result = san("<meta http-equiv='refresh' content='0;url=https://evil.com'>")
      refute_includes result, "meta"
    end

    def test_drops_noscript
      result = san("<noscript><img src=x onerror=alert(1)></noscript>")
      refute_includes result, "noscript"
    end

    def test_drops_base
      result = san("<base href='https://evil.com'>")
      refute_includes result, "base"
    end

    def test_drops_link
      result = san("<link rel='stylesheet' href='https://evil.com/evil.css'>")
      refute_includes result, "<link"
    end

    # ─── 3. Event handler stripping ────────────────────────────────────────────

    def test_strips_onload_from_img
      result = san('<img onload="alert(1)" src="x.png">')
      refute_includes result, "onload"
      assert_includes result, 'src="x.png"'
    end

    def test_strips_onclick_from_anchor
      result = san('<a onclick="alert(1)" href="https://ok.com">click</a>')
      refute_includes result, "onclick"
      assert_includes result, "href="
    end

    def test_strips_onfocus
      result = san('<div onfocus="alert(1)" tabindex="0">text</div>')
      refute_includes result, "onfocus"
      assert_includes result, "tabindex"
    end

    def test_strips_onerror
      result = san('<img onerror="alert(1)" src="bad.png">')
      refute_includes result, "onerror"
    end

    def test_strips_all_on_prefixed_attributes
      # Check several at once.
      result = san('<p onmouseover="x" onmouseout="y" onkeydown="z">hi</p>')
      %w[onmouseover onmouseout onkeydown].each do |attr|
        refute_includes result, attr
      end
    end

    def test_strips_svg_onload
      result = san('<svg onload="alert(1)"><circle r="10"/></svg>')
      refute_includes result, "onload"
    end

    # ─── 4. URL attribute sanitization ────────────────────────────────────────

    def test_strips_javascript_href
      result = san('<a href="javascript:alert(1)">click</a>')
      assert_includes result, 'href=""'
      assert_includes result, "click"
    end

    def test_strips_javascript_href_uppercase
      result = san('<a href="JAVASCRIPT:alert(1)">click</a>')
      assert_includes result, 'href=""'
    end

    def test_strips_data_href
      result = san('<a href="data:text/html,<script>alert(1)</script>">x</a>')
      assert_includes result, 'href=""'
    end

    def test_strips_javascript_in_src
      result = san('<img src="javascript:alert(1)" alt="x">')
      assert_includes result, 'src=""'
    end

    def test_allows_https_href
      result = san('<a href="https://example.com">ok</a>')
      assert_includes result, 'href="https://example.com"'
    end

    def test_allows_mailto_href
      result = san('<a href="mailto:user@example.com">email</a>')
      assert_includes result, 'href="mailto:user@example.com"'
    end

    def test_allows_relative_href
      result = san('<a href="/path/to/page">local</a>')
      assert_includes result, 'href="/path/to/page"'
    end

    def test_strips_control_char_javascript_bypass_in_href
      # "java\x00script:alert(1)" — NUL hidden in scheme
      result = san("<a href=\"java\x00script:alert(1)\">x</a>")
      assert_includes result, 'href=""'
    end

    def test_strips_zero_width_space_bypass_in_href
      # Zero-width space before "javascript"
      result = san("<a href=\"\u200Bjavascript:alert(1)\">x</a>")
      assert_includes result, 'href=""'
    end

    def test_strips_vbscript_href
      result = san('<a href="vbscript:MsgBox(1)">click</a>')
      assert_includes result, 'href=""'
    end

    def test_strips_blob_src
      result = san('<img src="blob:https://origin/uuid" alt="x">')
      assert_includes result, 'src=""'
    end

    def test_ftp_blocked_by_strict
      result = san('<a href="ftp://files.example.com/file">dl</a>',
        DocumentHtmlSanitizer::HTML_STRICT)
      assert_includes result, 'href=""'
    end

    def test_ftp_allowed_by_relaxed
      result = san('<a href="ftp://files.example.com/file">dl</a>',
        DocumentHtmlSanitizer::HTML_RELAXED)
      assert_includes result, 'href="ftp://files.example.com/file"'
    end

    # ─── 5. CSS style attribute sanitization ──────────────────────────────────

    def test_strips_css_expression
      result = san('<p style="width:expression(alert(1))">x</p>')
      refute_includes result, "style="
      assert_includes result, "<p>"
    end

    def test_strips_css_background_url_javascript
      result = san('<p style="background:url(javascript:alert(1))">x</p>')
      refute_includes result, "style="
    end

    def test_allows_safe_style
      result = san('<p style="color:red">x</p>')
      assert_includes result, 'style="color:red"'
    end

    def test_allows_https_url_in_style
      result = san('<p style="background:url(https://example.com/img.png)">x</p>')
      assert_includes result, "style="
    end

    def test_style_not_sanitized_when_policy_off
      policy = DocumentHtmlSanitizer::HTML_STRICT.with(
        sanitize_style_attributes: false
      )
      result = san('<p style="width:expression(alert(1))">x</p>', policy)
      assert_includes result, "style="
    end

    # ─── 6. Comment stripping ──────────────────────────────────────────────────

    def test_strips_html_comments
      result = san("<!-- comment --><p>ok</p>")
      refute_includes result, "<!--"
      assert_includes result, "<p>ok</p>"
    end

    def test_strips_multiline_comments
      result = san("<!--\n  hidden payload\n--><p>ok</p>")
      refute_includes result, "<!--"
    end

    def test_strips_ie_conditional_comment
      result = san("<!--[if IE]><script>alert(1)</script><![endif]-->")
      refute_includes result, "<!--"
      refute_includes result, "alert(1)"
    end

    def test_comments_preserved_when_drop_comments_false
      html = "<!-- comment --><p>ok</p>"
      result = san(html, DocumentHtmlSanitizer::HTML_RELAXED)
      assert_includes result, "<!--"
    end

    def test_comment_hiding_img_onerror_stripped
      # Even if comment is preserved, the content would not execute because
      # it is inside a comment. But STRICT drops comments entirely.
      result = san("<!--<img src=x onerror=alert(1)>-->")
      refute_includes result, "onerror"
    end

    # ─── 7. Attribute dropping (srcdoc, formaction, custom) ───────────────────

    def test_strips_srcdoc
      result = san('<iframe srcdoc="<script>alert(1)</script>"></iframe>')
      refute_includes result, "srcdoc"
    end

    def test_strips_formaction
      result = san('<button formaction="https://evil.com/phish">submit</button>')
      refute_includes result, "formaction"
    end

    def test_custom_drop_attribute
      policy = DocumentHtmlSanitizer::HTML_STRICT.with(
        drop_attributes: %w[data-xss]
      )
      result = san('<p data-xss="bad" class="ok">text</p>', policy)
      refute_includes result, "data-xss"
      assert_includes result, 'class="ok"'
    end

    # ─── 8. PASSTHROUGH preserves everything ──────────────────────────────────

    def test_passthrough_keeps_script
      html = "<script>alert(1)</script>"
      assert_equal html, san(html, DocumentHtmlSanitizer::HTML_PASSTHROUGH)
    end

    def test_passthrough_keeps_comments
      html = "<!-- comment -->"
      assert_equal html, san(html, DocumentHtmlSanitizer::HTML_PASSTHROUGH)
    end

    def test_passthrough_keeps_event_handlers
      html = '<img onload="alert(1)">'
      result = san(html, DocumentHtmlSanitizer::HTML_PASSTHROUGH)
      assert_includes result, "onload"
    end

    def test_passthrough_keeps_javascript_href
      html = '<a href="javascript:alert(1)">x</a>'
      result = san(html, DocumentHtmlSanitizer::HTML_PASSTHROUGH)
      assert_includes result, "javascript:"
    end

    # ─── 9. Custom policy via with() ──────────────────────────────────────────

    def test_custom_policy_add_ftp_to_strict
      policy = DocumentHtmlSanitizer::HTML_STRICT.with(
        allowed_url_schemes: %w[http https mailto ftp]
      )
      result = san('<a href="ftp://files.example.com">dl</a>', policy)
      assert_includes result, "ftp://"
    end

    def test_custom_policy_drop_extra_elements
      policy = DocumentHtmlSanitizer::HTML_PASSTHROUGH.with(
        drop_elements: %w[marquee]
      )
      result = san("<marquee>spin</marquee><p>ok</p>", policy)
      refute_includes result, "marquee"
      assert_includes result, "<p>ok</p>"
    end

    # ─── 10. Edge cases ────────────────────────────────────────────────────────

    def test_empty_string
      assert_equal "", san("")
    end

    def test_plain_text_unchanged
      assert_equal "Hello world", san("Hello world")
    end

    def test_self_closing_img_tag
      result = san('<img src="https://ok.com/img.png" alt="ok" />')
      assert_includes result, 'src="https://ok.com/img.png"'
      assert_includes result, 'alt="ok"'
    end

    def test_multiple_scripts_all_removed
      html = "<script>a()</script><p>mid</p><script>b()</script>"
      result = san(html)
      refute_includes result, "script"
      assert_includes result, "<p>mid</p>"
    end

    def test_attributes_with_single_quotes_parsed
      result = san("<a href='https://example.com'>link</a>")
      assert_includes result, "example.com"
    end

    def test_boolean_attribute_preserved
      # 'disabled' has no value
      result = san("<button disabled>click</button>",
        DocumentHtmlSanitizer::HTML_PASSTHROUGH)
      assert_includes result, "disabled"
    end
  end
end
