package documentastsanitizer_test

import (
	"reflect"
	"testing"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
	sanitizer "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-sanitizer"
)

// ─── Test Helpers ─────────────────────────────────────────────────────────────

// doc wraps block nodes into a DocumentNode for convenience.
func doc(children ...documentast.BlockNode) *documentast.DocumentNode {
	return &documentast.DocumentNode{Children: children}
}

// para wraps inline nodes into a ParagraphNode.
func para(children ...documentast.InlineNode) *documentast.ParagraphNode {
	return &documentast.ParagraphNode{Children: children}
}

// heading creates a HeadingNode.
func heading(level int, children ...documentast.InlineNode) *documentast.HeadingNode {
	return &documentast.HeadingNode{Level: level, Children: children}
}

// text creates a TextNode.
func text(s string) *documentast.TextNode {
	return &documentast.TextNode{Value: s}
}

// link creates a LinkNode.
func link(dest string, children ...documentast.InlineNode) *documentast.LinkNode {
	return &documentast.LinkNode{Destination: dest, Children: children}
}

// image creates an ImageNode.
func image(dest, alt string) *documentast.ImageNode {
	return &documentast.ImageNode{Destination: dest, Alt: alt}
}

// codeBlock creates a CodeBlockNode.
func codeBlock(lang, value string) *documentast.CodeBlockNode {
	return &documentast.CodeBlockNode{Language: lang, Value: value}
}

// codeSpan creates a CodeSpanNode.
func codeSpan(v string) *documentast.CodeSpanNode {
	return &documentast.CodeSpanNode{Value: v}
}

// rawBlock creates a RawBlockNode.
func rawBlock(format, value string) *documentast.RawBlockNode {
	return &documentast.RawBlockNode{Format: format, Value: value}
}

// rawInline creates a RawInlineNode.
func rawInline(format, value string) *documentast.RawInlineNode {
	return &documentast.RawInlineNode{Format: format, Value: value}
}

// autolink creates an AutolinkNode.
func autolink(dest string, isEmail bool) *documentast.AutolinkNode {
	return &documentast.AutolinkNode{Destination: dest, IsEmail: isEmail}
}

// blockquote wraps block nodes into a BlockquoteNode.
func blockquote(children ...documentast.BlockNode) *documentast.BlockquoteNode {
	return &documentast.BlockquoteNode{Children: children}
}

// emph wraps inline nodes into an EmphasisNode.
func emph(children ...documentast.InlineNode) *documentast.EmphasisNode {
	return &documentast.EmphasisNode{Children: children}
}

// strong wraps inline nodes into a StrongNode.
func strong(children ...documentast.InlineNode) *documentast.StrongNode {
	return &documentast.StrongNode{Children: children}
}

// hardBreak creates a HardBreakNode.
func hardBreak() *documentast.HardBreakNode { return &documentast.HardBreakNode{} }

// softBreak creates a SoftBreakNode.
func softBreak() *documentast.SoftBreakNode { return &documentast.SoftBreakNode{} }

// thematicBreak creates a ThematicBreakNode.
func thematicBreak() *documentast.ThematicBreakNode { return &documentast.ThematicBreakNode{} }

// ─── PASSTHROUGH Tests ────────────────────────────────────────────────────────

// TestPassthroughPreservesAllNodeTypes verifies that PASSTHROUGH is effectively
// identity — every node type passes through unchanged.
func TestPassthroughPreservesAllNodeTypes(t *testing.T) {
	input := doc(
		heading(1, text("Title")),
		para(
			text("Hello "),
			emph(text("world")),
			text(" and "),
			strong(text("bold")),
		),
		codeBlock("go", "fmt.Println()\n"),
		blockquote(para(text("quote"))),
		rawBlock("html", "<div>raw</div>\n"),
		para(
			link("https://example.com", text("click")),
			text(" "),
			image("img.png", "alt text"),
			text(" "),
			codeSpan("code"),
			text(" "),
			rawInline("html", "<em>raw</em>"),
			hardBreak(),
			softBreak(),
		),
		thematicBreak(),
	)

	result := sanitizer.Sanitize(input, sanitizer.PASSTHROUGH)

	// Document should have the same number of top-level blocks.
	if len(result.Children) != len(input.Children) {
		t.Errorf("PASSTHROUGH: expected %d blocks, got %d", len(input.Children), len(result.Children))
	}
}

// TestPassthroughDoesNotMutateInput confirms immutability — the original document
// is not changed after sanitization.
func TestPassthroughDoesNotMutateInput(t *testing.T) {
	input := doc(heading(1, text("Original")))
	original := input.Children[0].(*documentast.HeadingNode).Level

	result := sanitizer.Sanitize(input, sanitizer.PASSTHROUGH)

	// Modify the result (should not affect input).
	resultHeading := result.Children[0].(*documentast.HeadingNode)
	resultHeading.Level = 99

	if input.Children[0].(*documentast.HeadingNode).Level != original {
		t.Error("Sanitize mutated the input document")
	}
}

// ─── RawBlock Tests ───────────────────────────────────────────────────────────

func TestRawBlockDropAll(t *testing.T) {
	input := doc(rawBlock("html", "<script>alert(1)</script>\n"))
	result := sanitizer.Sanitize(input, sanitizer.SanitizationPolicy{
		AllowRawBlockFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawDropAll},
	})
	if len(result.Children) != 0 {
		t.Errorf("expected raw block to be dropped, got %d children", len(result.Children))
	}
}

func TestRawBlockPassthrough(t *testing.T) {
	input := doc(rawBlock("html", "<div>raw</div>\n"))
	result := sanitizer.Sanitize(input, sanitizer.SanitizationPolicy{
		AllowRawBlockFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawPassthrough},
	})
	if len(result.Children) != 1 {
		t.Errorf("expected raw block to pass through, got %d children", len(result.Children))
	}
}

func TestRawBlockAllowListKeepsMatchingFormat(t *testing.T) {
	input := doc(rawBlock("html", "<div>html</div>\n"))
	policy := sanitizer.SanitizationPolicy{
		AllowRawBlockFormats: sanitizer.RawFormatPolicy{
			Mode:           sanitizer.RawAllowList,
			AllowedFormats: []string{"html"},
		},
	}
	result := sanitizer.Sanitize(input, policy)
	if len(result.Children) != 1 {
		t.Errorf("expected html raw block to be kept, got %d children", len(result.Children))
	}
}

func TestRawBlockAllowListDropsNonMatchingFormat(t *testing.T) {
	input := doc(rawBlock("latex", "\\begin{equation}x=1\\end{equation}\n"))
	policy := sanitizer.SanitizationPolicy{
		AllowRawBlockFormats: sanitizer.RawFormatPolicy{
			Mode:           sanitizer.RawAllowList,
			AllowedFormats: []string{"html"},
		},
	}
	result := sanitizer.Sanitize(input, policy)
	if len(result.Children) != 0 {
		t.Errorf("expected latex raw block to be dropped, got %d children", len(result.Children))
	}
}

func TestRawInlineDropAll(t *testing.T) {
	input := doc(para(rawInline("html", "<em>raw</em>")))
	// Para becomes empty → para is also dropped.
	result := sanitizer.Sanitize(input, sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawDropAll},
	})
	if len(result.Children) != 0 {
		t.Errorf("expected para to be dropped (empty after raw inline drop), got %d children", len(result.Children))
	}
}

func TestRawInlinePassthrough(t *testing.T) {
	input := doc(para(rawInline("html", "<em>raw</em>")))
	result := sanitizer.Sanitize(input, sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawPassthrough},
	})
	if len(result.Children) != 1 {
		t.Errorf("expected raw inline to pass through, got %d children", len(result.Children))
	}
}

func TestRawInlineAllowListKeepsHtml(t *testing.T) {
	input := doc(para(rawInline("html", "<b>bold</b>")))
	policy := sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{
			Mode:           sanitizer.RawAllowList,
			AllowedFormats: []string{"html"},
		},
	}
	result := sanitizer.Sanitize(input, policy)
	if len(result.Children) != 1 {
		t.Errorf("expected html raw inline to pass through, got %d", len(result.Children))
	}
}

func TestRawInlineAllowListDropsLatex(t *testing.T) {
	input := doc(para(rawInline("latex", "$x=1$")))
	policy := sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{
			Mode:           sanitizer.RawAllowList,
			AllowedFormats: []string{"html"},
		},
	}
	result := sanitizer.Sanitize(input, policy)
	if len(result.Children) != 0 {
		t.Errorf("expected latex raw inline and empty para to be dropped, got %d children", len(result.Children))
	}
}

// ─── URL Scheme Tests ─────────────────────────────────────────────────────────

func TestLinkJavascriptSchemeBlocked(t *testing.T) {
	// javascript: URLs are the primary XSS vector in Markdown links.
	// The spec says: replace destination with "" when scheme is not allowed.
	input := doc(para(link("javascript:alert(1)", text("click me"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "" {
		t.Errorf("expected destination to be cleared, got %q", l.Destination)
	}
}

func TestLinkHttpsSchemeAllowed(t *testing.T) {
	input := doc(para(link("https://example.com", text("safe"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "https://example.com" {
		t.Errorf("expected https destination to pass through, got %q", l.Destination)
	}
}

func TestLinkRelativeUrlAlwaysAllowed(t *testing.T) {
	// Relative URLs have no scheme and always pass through, even in STRICT mode.
	input := doc(para(link("/relative/path", text("local"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "/relative/path" {
		t.Errorf("expected relative URL to pass through, got %q", l.Destination)
	}
}

func TestLinkVbscriptSchemeBlocked(t *testing.T) {
	input := doc(para(link("vbscript:MsgBox(1)", text("click"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "" {
		t.Errorf("expected vbscript: to be blocked, got %q", l.Destination)
	}
}

func TestLinkDataSchemeBlocked(t *testing.T) {
	input := doc(para(link("data:text/html,<script>alert(1)</script>", text("click"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "" {
		t.Errorf("expected data: to be blocked, got %q", l.Destination)
	}
}

func TestLinkBlobSchemeBlocked(t *testing.T) {
	input := doc(para(link("blob:https://origin/some-uuid", text("click"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "" {
		t.Errorf("expected blob: to be blocked, got %q", l.Destination)
	}
}

func TestLinkUppercaseJavascriptBlocked(t *testing.T) {
	// Attackers use uppercase scheme names to bypass case-sensitive checks.
	input := doc(para(link("JAVASCRIPT:alert(1)", text("click"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "" {
		t.Errorf("expected JAVASCRIPT: to be blocked (case-insensitive), got %q", l.Destination)
	}
}

func TestLinkNullByteBypassBlocked(t *testing.T) {
	// "java\x00script:" — null byte stripped → "javascript:" — should be blocked.
	input := doc(para(link("java\x00script:alert(1)", text("click"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "" {
		t.Errorf("expected null-byte bypass to be blocked, got %q", l.Destination)
	}
}

func TestLinkZeroWidthBypassBlocked(t *testing.T) {
	// "\u200bjavascript:alert(1)" — zero-width space stripped → "javascript:"
	input := doc(para(link("\u200bjavascript:alert(1)", text("click"))))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if l.Destination != "" {
		t.Errorf("expected zero-width-space bypass to be blocked, got %q", l.Destination)
	}
}

func TestAutolinkDataSchemeDropped(t *testing.T) {
	// Autolinks have no text to preserve, so if the scheme is disallowed,
	// the entire node is dropped.
	input := doc(para(autolink("data:text/plain,hello", false)))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)
	if len(result.Children) != 0 {
		t.Errorf("expected autolink with data: scheme to be dropped")
	}
}

func TestAutolinkHttpsKept(t *testing.T) {
	input := doc(para(autolink("https://example.com", false)))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)
	if len(result.Children) != 1 {
		t.Errorf("expected autolink with https: to be kept")
	}
}

func TestAutolinkEmailKeptWhenMailtoAllowed(t *testing.T) {
	// Email autolinks store the address without "mailto:" prefix.
	// We check "mailto" scheme against the allowlist.
	input := doc(para(autolink("user@example.com", true)))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)
	if len(result.Children) != 1 {
		t.Errorf("expected email autolink to be kept when mailto is in allowlist")
	}
}

func TestAutolinkEmailDroppedWhenMailtoNotAllowed(t *testing.T) {
	policy := sanitizer.SanitizationPolicy{
		AllowedUrlSchemes: []string{"http", "https"}, // no mailto
	}
	input := doc(para(autolink("user@example.com", true)))
	result := sanitizer.Sanitize(input, policy)
	if len(result.Children) != 0 {
		t.Errorf("expected email autolink to be dropped when mailto not in allowlist")
	}
}

// ─── Heading Level Tests ──────────────────────────────────────────────────────

func TestHeadingMinLevelPromotes(t *testing.T) {
	// h1 should be promoted to h2 when minHeadingLevel: 2
	input := doc(heading(1, text("Title")))
	policy := sanitizer.SanitizationPolicy{MinHeadingLevel: 2}
	result := sanitizer.Sanitize(input, policy)

	h := result.Children[0].(*documentast.HeadingNode)
	if h.Level != 2 {
		t.Errorf("expected level 2 (promoted from 1), got %d", h.Level)
	}
}

func TestHeadingMaxLevelClamps(t *testing.T) {
	// h5 should be clamped to h3 when maxHeadingLevel: 3
	input := doc(heading(5, text("Subsection")))
	policy := sanitizer.SanitizationPolicy{MaxHeadingLevel: 3}
	result := sanitizer.Sanitize(input, policy)

	h := result.Children[0].(*documentast.HeadingNode)
	if h.Level != 3 {
		t.Errorf("expected level 3 (clamped from 5), got %d", h.Level)
	}
}

func TestHeadingDropAllHeadings(t *testing.T) {
	// -1 signals "drop all headings"
	input := doc(heading(1, text("Title")), para(text("body")))
	policy := sanitizer.SanitizationPolicy{MaxHeadingLevel: -1}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 1 {
		t.Errorf("expected 1 block (heading dropped), got %d", len(result.Children))
	}
	if _, isPara := result.Children[0].(*documentast.ParagraphNode); !isPara {
		t.Error("expected remaining block to be a paragraph")
	}
}

func TestHeadingWithinLimitsUnchanged(t *testing.T) {
	input := doc(heading(3, text("Section")))
	policy := sanitizer.SanitizationPolicy{MinHeadingLevel: 2, MaxHeadingLevel: 5}
	result := sanitizer.Sanitize(input, policy)

	h := result.Children[0].(*documentast.HeadingNode)
	if h.Level != 3 {
		t.Errorf("expected level 3 unchanged, got %d", h.Level)
	}
}

func TestHeadingSTRICTClampsH1ToH2(t *testing.T) {
	// STRICT policy sets minHeadingLevel: 2
	input := doc(heading(1, text("Page Title")))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	h := result.Children[0].(*documentast.HeadingNode)
	if h.Level != 2 {
		t.Errorf("STRICT: expected h1 clamped to h2, got h%d", h.Level)
	}
}

// ─── Image Tests ──────────────────────────────────────────────────────────────

func TestDropImages(t *testing.T) {
	input := doc(para(image("cat.png", "a cat")))
	policy := sanitizer.SanitizationPolicy{DropImages: true}
	result := sanitizer.Sanitize(input, policy)

	// Para has no children → also dropped.
	if len(result.Children) != 0 {
		t.Errorf("expected image and empty para to be dropped, got %d children", len(result.Children))
	}
}

func TestTransformImageToText(t *testing.T) {
	input := doc(para(image("cat.png", "a cat")))
	policy := sanitizer.SanitizationPolicy{TransformImageToText: true}
	result := sanitizer.Sanitize(input, policy)

	p := result.Children[0].(*documentast.ParagraphNode)
	tn, ok := p.Children[0].(*documentast.TextNode)
	if !ok {
		t.Fatalf("expected TextNode, got %T", p.Children[0])
	}
	if tn.Value != "a cat" {
		t.Errorf("expected alt text %q, got %q", "a cat", tn.Value)
	}
}

func TestDropImagesHasPrecedenceOverTransformImageToText(t *testing.T) {
	// dropImages takes precedence over transformImageToText per the spec.
	input := doc(para(image("cat.png", "a cat")))
	policy := sanitizer.SanitizationPolicy{DropImages: true, TransformImageToText: true}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected drop to win over transform, got %d children", len(result.Children))
	}
}

func TestImageJavascriptSchemeCleared(t *testing.T) {
	// Use a custom policy that doesn't transform images to text,
	// so we can test that the destination is cleared for bad schemes.
	policy := sanitizer.SanitizationPolicy{
		AllowedUrlSchemes: []string{"http", "https", "mailto"},
	}
	input := doc(para(image("javascript:alert(1)", "xss")))
	result := sanitizer.Sanitize(input, policy)

	p := result.Children[0].(*documentast.ParagraphNode)
	img := p.Children[0].(*documentast.ImageNode)
	if img.Destination != "" {
		t.Errorf("expected javascript: image destination to be cleared, got %q", img.Destination)
	}
}

func TestImageSTRICTConvertsToAltText(t *testing.T) {
	// STRICT policy has transformImageToText: true
	input := doc(para(image("https://example.com/img.png", "example image")))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	tn, ok := p.Children[0].(*documentast.TextNode)
	if !ok {
		t.Fatalf("STRICT: expected TextNode, got %T", p.Children[0])
	}
	if tn.Value != "example image" {
		t.Errorf("STRICT: expected alt text, got %q", tn.Value)
	}
}

// ─── DropLinks Tests ──────────────────────────────────────────────────────────

func TestDropLinksPromotesChildren(t *testing.T) {
	// When dropLinks: true, the link wrapper is removed and children are
	// promoted to the parent. "click here" text is preserved as plain text.
	input := doc(para(link("https://example.com", text("click here"))))
	policy := sanitizer.SanitizationPolicy{
		DropLinks:       true,
		AllowAllSchemes: true,
	}
	result := sanitizer.Sanitize(input, policy)

	p := result.Children[0].(*documentast.ParagraphNode)
	if len(p.Children) != 1 {
		t.Fatalf("expected 1 inline after link promotion, got %d", len(p.Children))
	}
	tn, ok := p.Children[0].(*documentast.TextNode)
	if !ok {
		t.Fatalf("expected TextNode after promotion, got %T", p.Children[0])
	}
	if tn.Value != "click here" {
		t.Errorf("expected promoted text %q, got %q", "click here", tn.Value)
	}
}

func TestDropLinksPromotesMultipleChildren(t *testing.T) {
	// A link with emphasis inside: the emphasis child should be promoted.
	input := doc(para(link("https://example.com", text("read "), emph(text("this")))))
	policy := sanitizer.SanitizationPolicy{DropLinks: true, AllowAllSchemes: true}
	result := sanitizer.Sanitize(input, policy)

	p := result.Children[0].(*documentast.ParagraphNode)
	if len(p.Children) != 2 {
		t.Fatalf("expected 2 inlines after link promotion, got %d", len(p.Children))
	}
}

// ─── CodeBlock and CodeSpan Tests ─────────────────────────────────────────────

func TestDropCodeBlocks(t *testing.T) {
	input := doc(codeBlock("go", "fmt.Println()\n"))
	policy := sanitizer.SanitizationPolicy{DropCodeBlocks: true}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected code block to be dropped, got %d children", len(result.Children))
	}
}

func TestCodeBlockKeptByDefault(t *testing.T) {
	input := doc(codeBlock("go", "fmt.Println()\n"))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	if len(result.Children) != 1 {
		t.Errorf("STRICT: expected code block to be kept, got %d children", len(result.Children))
	}
}

func TestTransformCodeSpanToText(t *testing.T) {
	input := doc(para(codeSpan("const x = 1")))
	policy := sanitizer.SanitizationPolicy{TransformCodeSpanToText: true}
	result := sanitizer.Sanitize(input, policy)

	p := result.Children[0].(*documentast.ParagraphNode)
	tn, ok := p.Children[0].(*documentast.TextNode)
	if !ok {
		t.Fatalf("expected TextNode from code span transform, got %T", p.Children[0])
	}
	if tn.Value != "const x = 1" {
		t.Errorf("expected code span value as text, got %q", tn.Value)
	}
}

func TestCodeSpanKeptByDefault(t *testing.T) {
	input := doc(para(codeSpan("x = 1")))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	cs, ok := p.Children[0].(*documentast.CodeSpanNode)
	if !ok {
		t.Fatalf("STRICT: expected CodeSpanNode to be kept, got %T", p.Children[0])
	}
	if cs.Value != "x = 1" {
		t.Errorf("expected code span value unchanged, got %q", cs.Value)
	}
}

// ─── Blockquote Tests ─────────────────────────────────────────────────────────

func TestDropBlockquotes(t *testing.T) {
	input := doc(blockquote(para(text("quote"))))
	policy := sanitizer.SanitizationPolicy{DropBlockquotes: true}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected blockquote to be dropped, got %d children", len(result.Children))
	}
}

func TestBlockquoteChildrenNotPromotedWhenDropped(t *testing.T) {
	// Unlike links, blockquote children are NOT promoted when the blockquote
	// is dropped. The children vanish entirely.
	input := doc(blockquote(para(text("important quote"))))
	policy := sanitizer.SanitizationPolicy{DropBlockquotes: true}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected no children after blockquote drop (not promoted), got %d", len(result.Children))
	}
}

// ─── Empty Children Tests ─────────────────────────────────────────────────────

func TestEmptyDocumentIsValid(t *testing.T) {
	// DocumentNode is never dropped — an empty document is valid.
	input := doc()
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	if result == nil {
		t.Error("expected non-nil DocumentNode even for empty input")
	}
	if len(result.Children) != 0 {
		t.Errorf("expected 0 children, got %d", len(result.Children))
	}
}

func TestParaDroppedWhenAllChildrenDropped(t *testing.T) {
	// A paragraph containing only a raw inline that gets dropped should
	// itself be dropped to avoid empty <p></p> in output.
	input := doc(para(rawInline("html", "<script>alert(1)</script>")))
	policy := sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawDropAll},
	}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected empty paragraph to be dropped, got %d blocks", len(result.Children))
	}
}

func TestHeadingDroppedWhenAllChildrenDropped(t *testing.T) {
	// A heading containing only a raw inline that gets dropped should also
	// be dropped.
	input := doc(heading(2, rawInline("html", "<script>alert(1)</script>")))
	policy := sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawDropAll},
	}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected empty heading to be dropped, got %d blocks", len(result.Children))
	}
}

func TestBlockquoteDroppedWhenAllChildrenDropped(t *testing.T) {
	input := doc(blockquote(rawBlock("html", "<script>alert(1)</script>")))
	policy := sanitizer.SanitizationPolicy{
		AllowRawBlockFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawDropAll},
	}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected empty blockquote to be dropped, got %d blocks", len(result.Children))
	}
}

// ─── Immutability Tests ───────────────────────────────────────────────────────

func TestSanitizeIsImmutable(t *testing.T) {
	// Sanitize must never mutate the input tree. We build a document,
	// sanitize it, then verify the original is bit-for-bit identical to what
	// we built (structural comparison via reflect.DeepEqual on types).
	original := doc(
		heading(1, text("Title")),
		para(
			link("javascript:alert(1)", text("xss")),
			image("data:evil", "bad"),
		),
		rawBlock("html", "<script>evil</script>\n"),
	)

	// We cannot deep-equal pointers, but we can check the original has the
	// same structure it started with.
	originalH1Level := original.Children[0].(*documentast.HeadingNode).Level
	originalLinkDest := original.Children[1].(*documentast.ParagraphNode).Children[0].(*documentast.LinkNode).Destination

	sanitizer.Sanitize(original, sanitizer.STRICT)

	if original.Children[0].(*documentast.HeadingNode).Level != originalH1Level {
		t.Error("Sanitize mutated heading level in original")
	}
	if original.Children[1].(*documentast.ParagraphNode).Children[0].(*documentast.LinkNode).Destination != originalLinkDest {
		t.Error("Sanitize mutated link destination in original")
	}
}

// ─── ThematicBreak and Break Node Tests ───────────────────────────────────────

func TestThematicBreakPassesThrough(t *testing.T) {
	input := doc(thematicBreak())
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	if len(result.Children) != 1 {
		t.Errorf("expected thematic break to pass through, got %d children", len(result.Children))
	}
	if _, ok := result.Children[0].(*documentast.ThematicBreakNode); !ok {
		t.Error("expected ThematicBreakNode to pass through unchanged")
	}
}

func TestHardBreakPassesThrough(t *testing.T) {
	input := doc(para(text("line1"), hardBreak(), text("line2")))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	if _, ok := p.Children[1].(*documentast.HardBreakNode); !ok {
		t.Error("expected HardBreakNode to pass through")
	}
}

func TestSoftBreakPassesThrough(t *testing.T) {
	input := doc(para(text("line1"), softBreak(), text("line2")))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	p := result.Children[0].(*documentast.ParagraphNode)
	if _, ok := p.Children[1].(*documentast.SoftBreakNode); !ok {
		t.Error("expected SoftBreakNode to pass through")
	}
}

// ─── URL Utils Tests ──────────────────────────────────────────────────────────

func TestStripControlChars(t *testing.T) {
	cases := []struct {
		input    string
		expected string
	}{
		{"javascript:alert(1)", "javascript:alert(1)"},
		{"java\x00script:alert(1)", "javascript:alert(1)"},
		{"java\rscript:alert(1)", "javascript:alert(1)"},
		{"\u200bjavascript:alert(1)", "javascript:alert(1)"},
		{"\u200cjavascript:alert(1)", "javascript:alert(1)"},
		{"\uFEFFjavascript:alert(1)", "javascript:alert(1)"},
		{"https://example.com", "https://example.com"},
	}
	for _, tc := range cases {
		got := sanitizer.StripControlChars(tc.input)
		if got != tc.expected {
			t.Errorf("StripControlChars(%q) = %q, want %q", tc.input, got, tc.expected)
		}
	}
}

func TestExtractScheme(t *testing.T) {
	cases := []struct {
		input    string
		expected string
	}{
		{"https://example.com", "https"},
		{"JAVASCRIPT:alert(1)", "javascript"},
		{"mailto:user@example.com", "mailto"},
		{"/relative/path", ""},
		{"../also/relative", ""},
		{"?query=1", ""},
		{"foo/bar:baz", ""},          // colon after slash
		{"#javascript:alert(1)", ""}, // fragment URL — colon after #
		{"#section", ""},             // pure fragment
		{"no-colon-here", ""},
	}
	for _, tc := range cases {
		got := sanitizer.ExtractScheme(tc.input)
		if got != tc.expected {
			t.Errorf("ExtractScheme(%q) = %q, want %q", tc.input, got, tc.expected)
		}
	}
}

func TestIsSchemeAllowed(t *testing.T) {
	policy := sanitizer.SanitizationPolicy{
		AllowedUrlSchemes: []string{"http", "https", "mailto"},
	}
	cases := []struct {
		url      string
		allowed  bool
	}{
		{"https://example.com", true},
		{"http://example.com", true},
		{"mailto:user@example.com", true},
		{"javascript:alert(1)", false},
		{"data:text/html,<script>", false},
		{"/relative/path", true},          // relative always allowed
		{"JAVASCRIPT:alert(1)", false},     // case-insensitive
		{"java\x00script:alert(1)", false}, // null-byte bypass
	}
	for _, tc := range cases {
		got := sanitizer.IsSchemeAllowed(tc.url, policy)
		if got != tc.allowed {
			t.Errorf("IsSchemeAllowed(%q) = %v, want %v", tc.url, got, tc.allowed)
		}
	}
}

func TestIsSchemeAllowedWithAllowAllSchemes(t *testing.T) {
	// AllowAllSchemes == true should bypass all scheme checks.
	policy := sanitizer.SanitizationPolicy{AllowAllSchemes: true}
	if !sanitizer.IsSchemeAllowed("javascript:alert(1)", policy) {
		t.Error("expected AllowAllSchemes to allow javascript: scheme")
	}
}

// ─── Named Preset Smoke Tests ─────────────────────────────────────────────────

func TestSTRICTPresetDropsRawHtmlBlock(t *testing.T) {
	input := doc(rawBlock("html", "<script>alert(1)</script>\n"))
	result := sanitizer.Sanitize(input, sanitizer.STRICT)
	if len(result.Children) != 0 {
		t.Errorf("STRICT: expected raw html block dropped, got %d children", len(result.Children))
	}
}

func TestRELAXEDPresetKeepsHtmlRawBlock(t *testing.T) {
	input := doc(rawBlock("html", "<div>ok</div>\n"))
	result := sanitizer.Sanitize(input, sanitizer.RELAXED)
	if len(result.Children) != 1 {
		t.Errorf("RELAXED: expected html raw block kept, got %d children", len(result.Children))
	}
}

func TestRELAXEDPresetDropsLatexRawBlock(t *testing.T) {
	input := doc(rawBlock("latex", "$x=1$\n"))
	result := sanitizer.Sanitize(input, sanitizer.RELAXED)
	if len(result.Children) != 0 {
		t.Errorf("RELAXED: expected latex raw block dropped, got %d children", len(result.Children))
	}
}

func TestPASSFTHROUGHPresetKeepsEverything(t *testing.T) {
	input := doc(
		rawBlock("latex", "$x=1$\n"),
		para(link("javascript:alert(1)", text("xss"))),
		heading(1, text("Title")),
	)
	result := sanitizer.Sanitize(input, sanitizer.PASSTHROUGH)

	if len(result.Children) != 3 {
		t.Errorf("PASSTHROUGH: expected 3 blocks, got %d", len(result.Children))
	}
}

// ─── List Sanitization Tests ──────────────────────────────────────────────────

func TestListSanitizesChildren(t *testing.T) {
	// Lists should recursively sanitize their items.
	input := doc(&documentast.ListNode{
		Ordered: false,
		Tight:   true,
		Children: []*documentast.ListItemNode{
			{Children: []documentast.BlockNode{
				para(link("javascript:alert(1)", text("item 1"))),
			}},
		},
	})

	result := sanitizer.Sanitize(input, sanitizer.STRICT)

	list := result.Children[0].(*documentast.ListNode)
	item := list.Children[0]
	p := item.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)

	if l.Destination != "" {
		t.Errorf("expected link in list item to have destination cleared, got %q", l.Destination)
	}
}

// ─── Emphasis / Strong Sanitization Tests ────────────────────────────────────

func TestEmphasisDroppedWhenAllChildrenDropped(t *testing.T) {
	// Emphasis containing only a dropped raw inline should itself be dropped.
	input := doc(para(emph(rawInline("html", "<script>evil</script>"))))
	policy := sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawDropAll},
	}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected empty emphasis → empty paragraph → both dropped, got %d blocks", len(result.Children))
	}
}

func TestStrongDroppedWhenAllChildrenDropped(t *testing.T) {
	input := doc(para(strong(rawInline("html", "<script>evil</script>"))))
	policy := sanitizer.SanitizationPolicy{
		AllowRawInlineFormats: sanitizer.RawFormatPolicy{Mode: sanitizer.RawDropAll},
	}
	result := sanitizer.Sanitize(input, policy)

	if len(result.Children) != 0 {
		t.Errorf("expected empty strong → empty paragraph → both dropped, got %d blocks", len(result.Children))
	}
}

// ─── DeepEqual Structural Check ───────────────────────────────────────────────

func TestPassthroughIsStructurallyIdentical(t *testing.T) {
	// A document sanitized with PASSTHROUGH should be structurally identical
	// to the input (even though it's a different pointer/allocation).
	input := doc(
		heading(2, text("Hello")),
		para(text("world"), emph(text("!")), hardBreak()),
	)

	result := sanitizer.Sanitize(input, sanitizer.PASSTHROUGH)

	// Compare types of each block child.
	if len(result.Children) != len(input.Children) {
		t.Fatalf("expected same number of blocks")
	}
	for i, got := range result.Children {
		wantType := reflect.TypeOf(input.Children[i])
		gotType := reflect.TypeOf(got)
		if gotType != wantType {
			t.Errorf("block[%d]: expected type %v, got %v", i, wantType, gotType)
		}
	}
}

// ─── Link with Title Tests ────────────────────────────────────────────────────

func TestLinkTitleIsPreserved(t *testing.T) {
	input := doc(para(&documentast.LinkNode{
		Destination: "https://example.com",
		Title:       "Example Site",
		HasTitle:    true,
		Children:    []documentast.InlineNode{text("click")},
	}))

	result := sanitizer.Sanitize(input, sanitizer.STRICT)
	p := result.Children[0].(*documentast.ParagraphNode)
	l := p.Children[0].(*documentast.LinkNode)
	if !l.HasTitle || l.Title != "Example Site" {
		t.Errorf("expected link title to be preserved, got HasTitle=%v Title=%q", l.HasTitle, l.Title)
	}
}
