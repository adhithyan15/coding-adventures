package documenthtmlsanitizer_test

import (
	"strings"
	"testing"

	sanitizer "github.com/adhithyan15/coding-adventures/code/packages/go/document-html-sanitizer"
)

// ─── Script Element Removal ───────────────────────────────────────────────────

func TestScriptElementRemoved(t *testing.T) {
	input := `<p>Safe</p><script>alert(1)</script>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "<script") || strings.Contains(result, "alert") {
		t.Errorf("expected script to be removed, got %q", result)
	}
	if !strings.Contains(result, "<p>Safe</p>") {
		t.Errorf("expected safe content preserved, got %q", result)
	}
}

func TestScriptWithSrcAttributeRemoved(t *testing.T) {
	input := `<p>ok</p><script src="https://evil.com/xss.js"></script>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "evil.com") || strings.Contains(result, "script") {
		t.Errorf("expected external script to be removed, got %q", result)
	}
}

func TestScriptUppercaseRemoved(t *testing.T) {
	// Case-insensitive element matching
	input := `<SCRIPT>alert(1)</SCRIPT>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(strings.ToLower(result), "<script") || strings.Contains(result, "alert") {
		t.Errorf("expected uppercase SCRIPT to be removed, got %q", result)
	}
}

func TestScriptMixedCaseRemoved(t *testing.T) {
	input := `<ScRiPt>alert(1)</ScRiPt>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "alert") {
		t.Errorf("expected mixed-case script to be removed, got %q", result)
	}
}

func TestStyleElementRemoved(t *testing.T) {
	input := `<style>body { color: red; }</style><p>text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "<style") || strings.Contains(result, "color: red") {
		t.Errorf("expected style element to be removed, got %q", result)
	}
}

func TestIframeRemoved(t *testing.T) {
	input := `<iframe src="https://evil.com"></iframe><p>safe</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "iframe") || strings.Contains(result, "evil.com") {
		t.Errorf("expected iframe to be removed, got %q", result)
	}
}

func TestObjectRemoved(t *testing.T) {
	input := `<object data="plugin.swf"></object>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "object") || strings.Contains(result, "plugin.swf") {
		t.Errorf("expected object to be removed, got %q", result)
	}
}

func TestEmbedRemoved(t *testing.T) {
	input := `<embed src="plugin.swf" />`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "embed") {
		t.Errorf("expected embed to be removed, got %q", result)
	}
}

func TestFormRemoved(t *testing.T) {
	input := `<form action="/steal"><input type="password" /></form>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "<form") || strings.Contains(result, "<input") {
		t.Errorf("expected form to be removed, got %q", result)
	}
}

func TestMetaRemoved(t *testing.T) {
	input := `<meta http-equiv="refresh" content="0;url=https://evil.com">`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "meta") || strings.Contains(result, "evil.com") {
		t.Errorf("expected meta to be removed, got %q", result)
	}
}

// ─── Event Handler Attribute Removal ─────────────────────────────────────────

func TestOnloadRemovedFromImg(t *testing.T) {
	input := `<img onload="alert(1)" src="x.png">`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "onload") || strings.Contains(result, "alert") {
		t.Errorf("expected onload to be stripped, got %q", result)
	}
	if !strings.Contains(result, `src="x.png"`) {
		t.Errorf("expected src attribute preserved, got %q", result)
	}
}

func TestOnclickRemovedFromA(t *testing.T) {
	input := `<a onclick="alert(1)">click</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "onclick") || strings.Contains(result, "alert") {
		t.Errorf("expected onclick to be stripped, got %q", result)
	}
	if !strings.Contains(result, ">click</a>") {
		t.Errorf("expected link text preserved, got %q", result)
	}
}

func TestOnfocusRemovedFromDiv(t *testing.T) {
	input := `<div onfocus="alert(1)" tabindex="0">content</div>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "onfocus") {
		t.Errorf("expected onfocus to be stripped, got %q", result)
	}
}

func TestOnloadRemovedFromSvg(t *testing.T) {
	input := `<svg onload="alert(1)">content</svg>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "onload") {
		t.Errorf("expected onload to be stripped from svg, got %q", result)
	}
}

func TestAllOnAttributesStripped(t *testing.T) {
	// Verify that several on* attributes are all stripped.
	eventAttrs := []string{"onclick", "ondblclick", "onmousedown", "onmouseup",
		"onmouseover", "onmouseout", "onkeydown", "onkeyup", "onkeypress",
		"onsubmit", "onreset", "onchange", "onblur", "onerror"}
	for _, attr := range eventAttrs {
		input := `<div ` + attr + `="alert(1)">text</div>`
		result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
		if strings.Contains(result, attr) {
			t.Errorf("expected %q to be stripped, got %q", attr, result)
		}
	}
}

// ─── URL Sanitization in Attributes ──────────────────────────────────────────

func TestJavascriptHrefCleared(t *testing.T) {
	input := `<a href="javascript:alert(1)">click</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "javascript") {
		t.Errorf("expected javascript: href to be cleared, got %q", result)
	}
	// The link element itself should survive; only the href is cleared.
	if !strings.Contains(result, ">click</a>") {
		t.Errorf("expected link text preserved, got %q", result)
	}
}

func TestDataHrefCleared(t *testing.T) {
	input := `<a href="data:text/html,<script>alert(1)</script>">click</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "data:") {
		t.Errorf("expected data: href to be cleared, got %q", result)
	}
}

func TestVbscriptHrefCleared(t *testing.T) {
	input := `<a href="vbscript:MsgBox(1)">click</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "vbscript") {
		t.Errorf("expected vbscript: href to be cleared, got %q", result)
	}
}

func TestHttpsHrefPreserved(t *testing.T) {
	input := `<a href="https://example.com">safe link</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `href="https://example.com"`) {
		t.Errorf("expected https href preserved, got %q", result)
	}
}

func TestHttpHrefPreserved(t *testing.T) {
	input := `<a href="http://example.com">link</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `href="http://example.com"`) {
		t.Errorf("expected http href preserved, got %q", result)
	}
}

func TestMailtoHrefPreserved(t *testing.T) {
	input := `<a href="mailto:user@example.com">email</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `href="mailto:user@example.com"`) {
		t.Errorf("expected mailto href preserved, got %q", result)
	}
}

func TestRelativeHrefPreserved(t *testing.T) {
	// Relative URLs have no scheme and always pass through.
	input := `<a href="/about">about</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `href="/about"`) {
		t.Errorf("expected relative href preserved, got %q", result)
	}
}

func TestJavascriptSrcCleared(t *testing.T) {
	// src attribute is also a URL-bearing attribute.
	input := `<img src="javascript:alert(1)" alt="xss">`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "javascript") {
		t.Errorf("expected javascript: src to be cleared, got %q", result)
	}
}

func TestUppercaseJavascriptHrefCleared(t *testing.T) {
	// Case-insensitive scheme check.
	input := `<a href="JAVASCRIPT:alert(1)">click</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(strings.ToLower(result), "javascript") {
		t.Errorf("expected JAVASCRIPT: to be cleared (case-insensitive), got %q", result)
	}
}

func TestNullByteJavascriptHrefCleared(t *testing.T) {
	// null-byte bypass attempt.
	input := "<a href=\"java\x00script:alert(1)\">click</a>"
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "javascript") || strings.Contains(result, "alert") {
		t.Errorf("expected null-byte bypass to be blocked, got %q", result)
	}
}

func TestZeroWidthBypassHrefCleared(t *testing.T) {
	input := "<a href=\"\u200bjavascript:alert(1)\">click</a>"
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "javascript") {
		t.Errorf("expected zero-width-space bypass to be blocked, got %q", result)
	}
}

func TestFragmentUrlPreserved(t *testing.T) {
	// "#javascript:alert(1)" is a fragment URL — the colon is after "#"
	// so "javascript" is NOT the scheme. It's a same-page anchor link
	// and should be treated as relative (safe) by the scheme check.
	input := `<a href="#javascript:alert(1)">anchor</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `href="#javascript:alert(1)"`) {
		t.Errorf("expected fragment URL to be preserved as relative, got %q", result)
	}
}

func TestFtpHrefAllowedInRelaxed(t *testing.T) {
	input := `<a href="ftp://files.example.com/file.zip">download</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_RELAXED)
	if !strings.Contains(result, `href="ftp://files.example.com/file.zip"`) {
		t.Errorf("RELAXED: expected ftp href preserved, got %q", result)
	}
}

// ─── CSS Expression Injection Prevention ─────────────────────────────────────

func TestCssExpressionStripped(t *testing.T) {
	input := `<p style="width:expression(alert(1))">text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "expression") || strings.Contains(result, "alert") {
		t.Errorf("expected CSS expression to be stripped, got %q", result)
	}
	if !strings.Contains(result, ">text</p>") {
		t.Errorf("expected paragraph text preserved, got %q", result)
	}
}

func TestCssUrlJavascriptStripped(t *testing.T) {
	input := `<p style="background:url(javascript:alert(1))">text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "javascript") {
		t.Errorf("expected CSS url(javascript:) to be stripped, got %q", result)
	}
}

func TestCssExpressionCaseInsensitiveStripped(t *testing.T) {
	input := `<p style="width:EXPRESSION(alert(1))">text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(strings.ToLower(result), "expression") {
		t.Errorf("expected uppercase EXPRESSION to be stripped, got %q", result)
	}
}

func TestSafeCssStylePreserved(t *testing.T) {
	// A benign style attribute should pass through unchanged.
	input := `<p style="color:red;font-size:14px">text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `style="color:red;font-size:14px"`) {
		t.Errorf("expected safe style preserved, got %q", result)
	}
}

func TestCssUrlWithHttpsPreserved(t *testing.T) {
	// url(https://...) in a style attribute should not be stripped.
	input := `<div style="background:url(https://example.com/img.png)">x</div>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, "url(https://example.com/img.png)") {
		t.Errorf("expected https url() in style to be preserved, got %q", result)
	}
}

// ─── HTML Comment Removal ─────────────────────────────────────────────────────

func TestCommentStripped(t *testing.T) {
	input := `<!-- comment --><p>ok</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "<!--") || strings.Contains(result, "comment") {
		t.Errorf("expected comment to be stripped, got %q", result)
	}
	if !strings.Contains(result, "<p>ok</p>") {
		t.Errorf("expected paragraph preserved, got %q", result)
	}
}

func TestCommentWithScriptStripped(t *testing.T) {
	// Attack vector: hide script content inside a comment.
	input := `<!--<img src=x onerror=alert(1)>--><p>safe</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "<!--") || strings.Contains(result, "onerror") {
		t.Errorf("expected comment-hidden attack to be stripped, got %q", result)
	}
}

func TestIEConditionalCommentStripped(t *testing.T) {
	input := `<!--[if IE]><script>alert(1)</script><![endif]-->`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "<!--") || strings.Contains(result, "alert") {
		t.Errorf("expected IE conditional comment to be stripped, got %q", result)
	}
}

func TestCommentPreservedInPassthrough(t *testing.T) {
	input := `<!-- keep me --><p>text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_PASSTHROUGH)
	if !strings.Contains(result, "<!-- keep me -->") {
		t.Errorf("PASSTHROUGH: expected comment preserved, got %q", result)
	}
}

func TestCommentPreservedInRelaxed(t *testing.T) {
	// HTML_RELAXED has dropComments: false
	input := `<!-- template marker --><p>text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_RELAXED)
	if !strings.Contains(result, "<!-- template marker -->") {
		t.Errorf("RELAXED: expected comment preserved, got %q", result)
	}
}

// ─── srcdoc and formaction Attributes ────────────────────────────────────────

func TestSrcdocStripped(t *testing.T) {
	// srcdoc is in the default DropAttributes for HTML_STRICT.
	input := `<iframe srcdoc="<script>alert(1)</script>"></iframe>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	// iframe is in DropElements for STRICT, so the whole thing should be gone.
	if strings.Contains(result, "srcdoc") || strings.Contains(result, "alert") {
		t.Errorf("expected srcdoc/iframe to be removed, got %q", result)
	}
}

func TestFormactionStripped(t *testing.T) {
	// formaction overrides the form's action URL.
	// form is in DropElements for STRICT, so test with RELAXED + custom drop.
	policy := sanitizer.HTML_RELAXED
	policy.DropAttributes = append(policy.DropAttributes, "formaction")
	input := `<button formaction="https://evil.com">submit</button>`
	result := sanitizer.SanitizeHtml(input, policy)
	if strings.Contains(result, "formaction") {
		t.Errorf("expected formaction to be stripped, got %q", result)
	}
}

// ─── PASSTHROUGH Preset ───────────────────────────────────────────────────────

func TestPassthroughPreservesScript(t *testing.T) {
	// HTML_PASSTHROUGH must keep everything as-is.
	input := `<script>alert(1)</script>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_PASSTHROUGH)
	if result != input {
		t.Errorf("PASSTHROUGH: expected input unchanged, got %q", result)
	}
}

func TestPassthroughPreservesOnclick(t *testing.T) {
	input := `<a onclick="doSomething()">click</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_PASSTHROUGH)
	if !strings.Contains(result, "onclick") {
		t.Errorf("PASSTHROUGH: expected onclick preserved, got %q", result)
	}
}

func TestPassthroughPreservesJavascriptHref(t *testing.T) {
	input := `<a href="javascript:void(0)">click</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_PASSTHROUGH)
	if !strings.Contains(result, "javascript:void(0)") {
		t.Errorf("PASSTHROUGH: expected javascript: href preserved, got %q", result)
	}
}

// ─── RELAXED Preset ───────────────────────────────────────────────────────────

func TestRelaxedRemovesScript(t *testing.T) {
	input := `<script>alert(1)</script><p>ok</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_RELAXED)
	if strings.Contains(result, "script") || strings.Contains(result, "alert") {
		t.Errorf("RELAXED: expected script removed, got %q", result)
	}
}

func TestRelaxedKeepsStyleElement(t *testing.T) {
	// style element is NOT in HTML_RELAXED.DropElements
	input := `<style>p { color: red; }</style><p>text</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_RELAXED)
	if !strings.Contains(result, "<style>") {
		t.Errorf("RELAXED: expected style element kept, got %q", result)
	}
}

// ─── Safe Content Preservation ───────────────────────────────────────────────

func TestSafeHtmlPreserved(t *testing.T) {
	// A document containing only safe HTML should be unchanged (modulo
	// whitespace normalization in attributes).
	input := `<h1>Title</h1><p>Hello <em>world</em>!</p><ul><li>item</li></ul>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, "<h1>Title</h1>") {
		t.Errorf("expected h1 preserved, got %q", result)
	}
	if !strings.Contains(result, "<em>world</em>") {
		t.Errorf("expected em preserved, got %q", result)
	}
	if !strings.Contains(result, "<li>item</li>") {
		t.Errorf("expected li preserved, got %q", result)
	}
}

func TestEmptyStringReturnsEmpty(t *testing.T) {
	result := sanitizer.SanitizeHtml("", sanitizer.HTML_STRICT)
	if result != "" {
		t.Errorf("expected empty string for empty input, got %q", result)
	}
}

func TestPlainTextPreserved(t *testing.T) {
	input := "Hello, world!"
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if result != input {
		t.Errorf("expected plain text preserved, got %q", result)
	}
}

// ─── URL Utils Tests ──────────────────────────────────────────────────────────
// (Testing the package-internal behaviour through SanitizeHtml)

func TestSchemeCheckCaseInsensitive(t *testing.T) {
	cases := []string{"JAVASCRIPT", "Javascript", "jAvAsCrIpT"}
	for _, scheme := range cases {
		input := `<a href="` + scheme + `:alert(1)">x</a>`
		result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
		if strings.Contains(strings.ToLower(result), strings.ToLower(scheme)+":") {
			t.Errorf("expected %s: scheme to be blocked, got %q", scheme, result)
		}
	}
}

// ─── Multi-element Safety Tests ───────────────────────────────────────────────

func TestMultipleDangerousElementsAllRemoved(t *testing.T) {
	input := `<p>before</p>` +
		`<script>s1</script>` +
		`<p>middle</p>` +
		`<script>s2</script>` +
		`<p>after</p>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "script") || strings.Contains(result, "s1") || strings.Contains(result, "s2") {
		t.Errorf("expected all script elements removed, got %q", result)
	}
	if !strings.Contains(result, "before") || !strings.Contains(result, "middle") || !strings.Contains(result, "after") {
		t.Errorf("expected safe paragraphs preserved, got %q", result)
	}
}

func TestScriptWithNewlinesRemoved(t *testing.T) {
	// Multi-line scripts must also be caught.
	input := "<script>\nfunction evil() {\n  alert(1);\n}\nevil();\n</script>"
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if strings.Contains(result, "alert") || strings.Contains(result, "evil") {
		t.Errorf("expected multi-line script to be removed, got %q", result)
	}
}

// ─── Link Title Attribute Preservation ───────────────────────────────────────

func TestLinkTitleAttributePreserved(t *testing.T) {
	input := `<a href="https://example.com" title="Example">link</a>`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `title="Example"`) {
		t.Errorf("expected title attribute preserved, got %q", result)
	}
	if !strings.Contains(result, `href="https://example.com"`) {
		t.Errorf("expected href preserved, got %q", result)
	}
}

// ─── Image Alt and Src Tests ─────────────────────────────────────────────────

func TestImgSrcAndAltPreserved(t *testing.T) {
	input := `<img src="https://example.com/img.png" alt="an image">`
	result := sanitizer.SanitizeHtml(input, sanitizer.HTML_STRICT)
	if !strings.Contains(result, `src="https://example.com/img.png"`) {
		t.Errorf("expected img src preserved, got %q", result)
	}
	if !strings.Contains(result, `alt="an image"`) {
		t.Errorf("expected img alt preserved, got %q", result)
	}
}

// ─── Custom Policy Tests ──────────────────────────────────────────────────────

func TestCustomDropElementsPolicy(t *testing.T) {
	// Custom policy that only drops table elements.
	policy := sanitizer.HtmlSanitizationPolicy{
		DropElements:            []string{"table", "thead", "tbody", "tr", "td", "th"},
		DropAttributes:          []string{},
		AllowAllUrlSchemes:      true,
		DropComments:            false,
		SanitizeStyleAttributes: false,
	}
	input := `<p>text</p><table><tr><td>cell</td></tr></table><p>end</p>`
	result := sanitizer.SanitizeHtml(input, policy)
	if strings.Contains(result, "<table") || strings.Contains(result, "<td") {
		t.Errorf("expected table elements removed, got %q", result)
	}
	if !strings.Contains(result, "<p>text</p>") {
		t.Errorf("expected paragraphs preserved, got %q", result)
	}
}

func TestCustomDropAttributesPolicy(t *testing.T) {
	// Custom policy that drops class and id attributes.
	policy := sanitizer.HtmlSanitizationPolicy{
		DropElements:            []string{},
		DropAttributes:          []string{"class", "id"},
		AllowAllUrlSchemes:      true,
		DropComments:            false,
		SanitizeStyleAttributes: false,
	}
	input := `<p class="foo" id="bar">text</p>`
	result := sanitizer.SanitizeHtml(input, policy)
	if strings.Contains(result, "class=") || strings.Contains(result, "id=") {
		t.Errorf("expected class and id stripped, got %q", result)
	}
	if !strings.Contains(result, ">text</p>") {
		t.Errorf("expected text content preserved, got %q", result)
	}
}
