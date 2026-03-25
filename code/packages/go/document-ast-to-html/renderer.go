// Package documentasttohtml renders a Document AST to an HTML string.
//
// # Overview
//
// The renderer is a recursive tree walk over the Document AST (from the
// document-ast package). Each node type maps to HTML elements following the
// CommonMark spec HTML rendering rules (§Appendix C).
//
// # Node Mapping
//
//	DocumentNode      → rendered children
//	HeadingNode       → <h1>…</h1> through <h6>…</h6>
//	ParagraphNode     → <p>…</p>  (omitted in tight list context)
//	CodeBlockNode     → <pre><code [class="language-X"]>…</code></pre>
//	BlockquoteNode    → <blockquote>\n…</blockquote>
//	ListNode          → <ul> or <ol [start="N"]>
//	ListItemNode      → <li>…</li>
//	ThematicBreakNode → <hr />
//	RawBlockNode      → verbatim if format="html", skipped otherwise
//
//	TextNode          → HTML-escaped text
//	EmphasisNode      → <em>…</em>
//	StrongNode        → <strong>…</strong>
//	CodeSpanNode      → <code>…</code>
//	LinkNode          → <a href="…" [title="…"]>…</a>
//	ImageNode         → <img src="…" alt="…" [title="…"] />
//	AutolinkNode      → <a href="[mailto:]…">…</a>
//	RawInlineNode     → verbatim if format="html", skipped otherwise
//	HardBreakNode     → <br />\n
//	SoftBreakNode     → \n
//
// # Tight vs Loose Lists
//
// A tight list suppresses <p> tags around paragraph content in list items:
//
//	Tight:  <li>item text</li>
//	Loose:  <li><p>item text</p></li>
//
// # Security
//
//   - Text content and attribute values are HTML-escaped via EscapeHtml.
//   - RawBlockNode and RawInlineNode content is passed through verbatim when
//     format === "html" — this is intentional and spec-required.
//   - Link and image URLs are sanitized to block dangerous schemes:
//     javascript:, vbscript:, data:, blob:.
//   - Pass RenderOptions{Sanitize: true} for user-controlled Markdown.
//
// Spec: TE02 — Document AST → HTML
package documentasttohtml

import (
	"fmt"
	"regexp"
	"strings"
	"unicode/utf8"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// RenderOptions controls rendering behaviour.
type RenderOptions struct {
	// Sanitize drops all RawBlockNode and RawInlineNode content when true.
	//
	// You MUST set Sanitize: true when rendering untrusted Markdown
	// (e.g. user-supplied content in a web application). Raw HTML passthrough
	// is a CommonMark spec requirement and is enabled by default, but it means
	// an attacker who can write Markdown can inject arbitrary HTML —
	// including <script> tags — into the rendered output.
	//
	// Default: false (raw HTML passes through verbatim — spec-compliant).
	Sanitize bool
}

// ToHtml renders a Document AST to an HTML string.
//
// The input is a *DocumentNode as produced by any front-end parser that
// implements the Document AST spec (TE00). The output is a valid HTML fragment.
//
//	// Trusted Markdown (documentation, static content):
//	html := ToHtml(doc, RenderOptions{})
//
//	// Untrusted Markdown (user-supplied content):
//	html := ToHtml(doc, RenderOptions{Sanitize: true})
func ToHtml(document *documentast.DocumentNode, opts RenderOptions) string {
	return renderBlocks(document.Children, false, opts)
}

// ─── Block Rendering ──────────────────────────────────────────────────────────

func renderBlocks(blocks []documentast.BlockNode, tight bool, opts RenderOptions) string {
	var b strings.Builder
	for _, block := range blocks {
		b.WriteString(renderBlock(block, tight, opts))
	}
	return b.String()
}

func renderBlock(block documentast.BlockNode, tight bool, opts RenderOptions) string {
	switch b := block.(type) {
	case *documentast.DocumentNode:
		return renderBlocks(b.Children, false, opts)
	case *documentast.HeadingNode:
		return renderHeading(b, opts)
	case *documentast.ParagraphNode:
		return renderParagraph(b, tight, opts)
	case *documentast.CodeBlockNode:
		return renderCodeBlock(b)
	case *documentast.BlockquoteNode:
		return renderBlockquote(b, opts)
	case *documentast.ListNode:
		return renderList(b, opts)
	case *documentast.ListItemNode:
		return renderListItem(b, false, opts)
	case *documentast.ThematicBreakNode:
		return "<hr />\n"
	case *documentast.RawBlockNode:
		return renderRawBlock(b, opts)
	default:
		return ""
	}
}

// renderHeading renders an ATX or setext heading.
//
//	HeadingNode { Level: 1, Children: [TextNode { Value: "Hello" }] }
//	→ <h1>Hello</h1>\n
func renderHeading(node *documentast.HeadingNode, opts RenderOptions) string {
	inner := renderInlines(node.Children, opts)
	return fmt.Sprintf("<h%d>%s</h%d>\n", node.Level, inner, node.Level)
}

// renderParagraph renders a paragraph.
//
// In tight list context, the <p> wrapper is omitted and only the inner
// content is emitted (followed by a newline).
//
//	ParagraphNode → <p>Hello <em>world</em></p>\n
//	ParagraphNode (tight) → Hello <em>world</em>\n
func renderParagraph(node *documentast.ParagraphNode, tight bool, opts RenderOptions) string {
	inner := renderInlines(node.Children, opts)
	if tight {
		return inner + "\n"
	}
	return "<p>" + inner + "</p>\n"
}

// renderCodeBlock renders a fenced or indented code block.
//
// The content is HTML-escaped but not Markdown-processed. If the block has a
// language (info string), the <code> tag gets a class="language-<lang>" attribute.
//
//	CodeBlockNode { Language: "ts", Value: "const x = 1;\n" }
//	→ <pre><code class="language-ts">const x = 1;\n</code></pre>\n
func renderCodeBlock(node *documentast.CodeBlockNode) string {
	escaped := EscapeHtml(node.Value)
	if node.Language != "" {
		return "<pre><code class=\"language-" + EscapeHtml(node.Language) + "\">" + escaped + "</code></pre>\n"
	}
	return "<pre><code>" + escaped + "</code></pre>\n"
}

// renderBlockquote renders a blockquote.
//
//	BlockquoteNode → <blockquote>\n<p>…</p>\n</blockquote>\n
func renderBlockquote(node *documentast.BlockquoteNode, opts RenderOptions) string {
	inner := renderBlocks(node.Children, false, opts)
	return "<blockquote>\n" + inner + "</blockquote>\n"
}

// renderList renders an ordered or unordered list.
//
// Ordered lists with a start number other than 1 get a start attribute.
// The tight flag is passed to each list item so <p> tags are omitted.
//
//	ListNode { Ordered: false, Tight: true }
//	→ <ul>\n<li>item1</li>\n<li>item2</li>\n</ul>\n
//
//	ListNode { Ordered: true, Start: 3, Tight: false }
//	→ <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>\n
func renderList(node *documentast.ListNode, opts RenderOptions) string {
	tag := "ul"
	var startAttr string
	if node.Ordered {
		tag = "ol"
		if node.Start != 1 {
			startAttr = fmt.Sprintf(` start="%d"`, node.Start)
		}
	}

	var b strings.Builder
	b.WriteString("<")
	b.WriteString(tag)
	b.WriteString(startAttr)
	b.WriteString(">\n")
	for _, item := range node.Children {
		b.WriteString(renderListItem(item, node.Tight, opts))
	}
	b.WriteString("</")
	b.WriteString(tag)
	b.WriteString(">\n")
	return b.String()
}

// renderListItem renders a single list item.
//
// Tight single-paragraph items: <li>text</li> (no <p> wrapper).
// All other items (multiple blocks, non-paragraph first child):
//
//	<li>\ncontent\n</li>
//
// An empty item renders as <li></li>.
func renderListItem(node *documentast.ListItemNode, tight bool, opts RenderOptions) string {
	if len(node.Children) == 0 {
		return "<li></li>\n"
	}

	if tight {
		if firstPara, ok := node.Children[0].(*documentast.ParagraphNode); ok {
			firstContent := renderInlines(firstPara.Children, opts)
			if len(node.Children) == 1 {
				return "<li>" + firstContent + "</li>\n"
			}
			// Multiple children: inline the first paragraph, block-render the rest
			rest := renderBlocks(node.Children[1:], tight, opts)
			return "<li>" + firstContent + "\n" + rest + "</li>\n"
		}
	}

	// Loose or non-paragraph first child: block-level format
	inner := renderBlocks(node.Children, tight, opts)
	lastChild := node.Children[len(node.Children)-1]
	if tight {
		if _, ok := lastChild.(*documentast.ParagraphNode); ok && strings.HasSuffix(inner, "\n") {
			return "<li>\n" + inner[:len(inner)-1] + "</li>\n"
		}
	}
	return "<li>\n" + inner + "</li>\n"
}

// renderRawBlock renders a raw block node.
//
// If opts.Sanitize is true, the node is always skipped.
// Otherwise, if format === "html", emit the raw value verbatim.
// Skip silently for any other format.
//
//	RawBlockNode { Format: "html", Value: "<div>raw</div>\n" }
//	→ <div>raw</div>\n                (sanitize: false — default)
//	→ (empty string)                  (sanitize: true)
func renderRawBlock(node *documentast.RawBlockNode, opts RenderOptions) string {
	if opts.Sanitize {
		return ""
	}
	if node.Format == "html" {
		return node.Value
	}
	return ""
}

// ─── Inline Rendering ─────────────────────────────────────────────────────────

func renderInlines(nodes []documentast.InlineNode, opts RenderOptions) string {
	var b strings.Builder
	for _, n := range nodes {
		b.WriteString(renderInline(n, opts))
	}
	return b.String()
}

func renderInline(node documentast.InlineNode, opts RenderOptions) string {
	switch n := node.(type) {
	case *documentast.TextNode:
		return EscapeHtml(n.Value)
	case *documentast.EmphasisNode:
		return "<em>" + renderInlines(n.Children, opts) + "</em>"
	case *documentast.StrongNode:
		return "<strong>" + renderInlines(n.Children, opts) + "</strong>"
	case *documentast.CodeSpanNode:
		return "<code>" + EscapeHtml(n.Value) + "</code>"
	case *documentast.LinkNode:
		return renderLink(n, opts)
	case *documentast.ImageNode:
		return renderImage(n)
	case *documentast.AutolinkNode:
		return renderAutolink(n)
	case *documentast.RawInlineNode:
		return renderRawInline(n, opts)
	case *documentast.HardBreakNode:
		return "<br />\n"
	case *documentast.SoftBreakNode:
		return "\n"
	default:
		return ""
	}
}

// renderLink renders an inline or reference link.
//
//	LinkNode { Destination: "https://x.com", Title: "X", Children: […] }
//	→ <a href="https://x.com" title="X">…</a>
func renderLink(node *documentast.LinkNode, opts RenderOptions) string {
	href := EscapeHtml(sanitizeURL(node.Destination))
	var titleAttr string
	if node.HasTitle {
		titleAttr = ` title="` + EscapeHtml(node.Title) + `"`
	}
	inner := renderInlines(node.Children, opts)
	return `<a href="` + href + `"` + titleAttr + `>` + inner + `</a>`
}

// renderImage renders an inline image.
//
//	ImageNode { Destination: "cat.png", Alt: "a cat" }
//	→ <img src="cat.png" alt="a cat" />
func renderImage(node *documentast.ImageNode) string {
	src := EscapeHtml(sanitizeURL(node.Destination))
	alt := EscapeHtml(node.Alt)
	var titleAttr string
	if node.HasTitle {
		titleAttr = ` title="` + EscapeHtml(node.Title) + `"`
	}
	return `<img src="` + src + `" alt="` + alt + `"` + titleAttr + ` />`
}

// renderAutolink renders an autolink `<url>` or `<email>`.
//
// For email autolinks, the href gets a mailto: prefix.
//
//	AutolinkNode { Destination: "user@example.com", IsEmail: true }
//	→ <a href="mailto:user@example.com">user@example.com</a>
func renderAutolink(node *documentast.AutolinkNode) string {
	dest := sanitizeURL(node.Destination)
	var href string
	if node.IsEmail {
		href = "mailto:" + EscapeHtml(dest)
	} else {
		href = EscapeHtml(sanitizeURL(normalizeURLForAutolink(dest)))
	}
	text := EscapeHtml(node.Destination)
	return `<a href="` + href + `">` + text + `</a>`
}

// normalizeURLForAutolink percent-encodes characters in autolink URLs that
// need encoding per the CommonMark spec.
func normalizeURLForAutolink(url string) string {
	var b strings.Builder
	b.Grow(len(url))
	for i := 0; i < len(url); {
		r, size := utf8.DecodeRuneInString(url[i:])
		if shouldPercentEncodeURL(r) {
			var buf [4]byte
			n := utf8.EncodeRune(buf[:], r)
			for j := 0; j < n; j++ {
				b.WriteString(percentEncode(buf[j]))
			}
		} else {
			b.WriteRune(r)
		}
		i += size
	}
	return b.String()
}

func shouldPercentEncodeURL(ch rune) bool {
	if ch >= 'a' && ch <= 'z' {
		return false
	}
	if ch >= 'A' && ch <= 'Z' {
		return false
	}
	if ch >= '0' && ch <= '9' {
		return false
	}
	switch ch {
	case '-', '_', '.', '~', ':', '/', '?', '#', '@', '!', '$', '&',
		'\'', '(', ')', '*', '+', ',', ';', '=', '%':
		return false
	}
	return true
}

func percentEncode(b byte) string {
	const hex = "0123456789ABCDEF"
	return "%" + string(hex[b>>4]) + string(hex[b&0xf])
}

// renderRawInline renders a raw inline node.
//
// If opts.Sanitize is true, the node is always skipped.
func renderRawInline(node *documentast.RawInlineNode, opts RenderOptions) string {
	if opts.Sanitize {
		return ""
	}
	if node.Format == "html" {
		return node.Value
	}
	return ""
}

// ─── URL Sanitization ─────────────────────────────────────────────────────────
//
// CommonMark spec §C.3 intentionally leaves URL sanitization to the implementor.
// Without scheme filtering, user-controlled Markdown is vulnerable to XSS via
// javascript: and data: URIs — both are valid URL characters that HTML-escaping
// does not neutralize.
//
// We use a targeted blocklist of the schemes that are execution-capable:
//
//	javascript:  — executes JS in the browser's origin
//	vbscript:    — executes VBScript (IE legacy)
//	data:        — can embed scripts as data:text/html
//	blob:        — same-origin blob URLs can execute scripts
//
// All other schemes (irc:, ftp:, mailto:, etc.) pass through unchanged.

var dangerousScheme = regexp.MustCompile(`(?i)^(?:javascript|vbscript|data|blob):`)

// urlControlChars matches control characters and invisible characters that
// browsers may strip before scheme detection, enabling bypass attacks:
//   - U+0000–U+001F C0 controls (TAB, LF, CR, etc.)
//   - U+007F–U+009F DEL + C1 controls
//   - U+200B–U+200D zero-width characters
//   - U+2060 word joiner, U+FEFF BOM
var urlControlChars = regexp.MustCompile("[\u0000-\u001F\u007F-\u009F\u200B-\u200D\u2060\uFEFF]")

// sanitizeURL strips control characters and blocks dangerous schemes.
// Returns "" if the URL uses an execution-capable scheme.
func sanitizeURL(url string) string {
	stripped := urlControlChars.ReplaceAllString(url, "")
	if dangerousScheme.MatchString(stripped) {
		return ""
	}
	return stripped
}

// ─── HTML Escaping ────────────────────────────────────────────────────────────

// EscapeHtml encodes characters that must be escaped in HTML attribute values
// and text content. Characters escaped: & < > "
func EscapeHtml(text string) string {
	needsEscape := false
	for _, ch := range text {
		if ch == '&' || ch == '<' || ch == '>' || ch == '"' {
			needsEscape = true
			break
		}
	}
	if !needsEscape {
		return text
	}

	var b strings.Builder
	b.Grow(len(text) + 16)
	for _, ch := range text {
		switch ch {
		case '&':
			b.WriteString("&amp;")
		case '<':
			b.WriteString("&lt;")
		case '>':
			b.WriteString("&gt;")
		case '"':
			b.WriteString("&quot;")
		default:
			b.WriteRune(ch)
		}
	}
	return b.String()
}
