// Package documentast defines the Document AST — the format-agnostic
// Intermediate Representation (IR) for structured documents.
//
// # What is the Document AST?
//
// Think of it as the "LLVM IR of documents." Just as LLVM IR sits between
// many source languages (C, C++, Rust, Swift) and many machine targets
// (x86, ARM, WebAssembly), the Document AST sits between document source
// formats and output renderers:
//
//	Markdown ──────────────────────────► HTML
//	reStructuredText ──► Document AST ──► PDF
//	HTML ───────────────────────────────► Plain text
//	DOCX ───────────────────────────────► DOCX
//
// With this shared IR, N front-ends × M back-ends requires only N+M
// implementations instead of N×M. Each front-end produces a DocumentNode;
// each back-end consumes it.
//
// # Design Principles
//
// 1. Semantic, not notational — nodes carry meaning (heading, emphasis),
//    not syntax (### or ***).
//
// 2. Resolved, not deferred — all link references are resolved before the
//    IR is produced. The IR never contains unresolved [text][label] refs.
//
// 3. Format-agnostic — RawBlockNode and RawInlineNode carry a `format` tag
//    (e.g. "html", "latex") so back-ends can selectively pass through or
//    ignore format-specific raw content.
//
// 4. Minimal and stable — only universal document concepts appear here.
//    GFM tables, footnotes, etc. belong in extension packages.
//
// # Using the Type Hierarchy
//
// Go does not have discriminated unions, so we use the interface+switch pattern:
//
//	switch node := node.(type) {
//	case *HeadingNode:   // node.Level, node.Children
//	case *ParagraphNode: // node.Children
//	case *CodeBlockNode: // node.Language, node.Value
//	// ... etc
//	}
//
// Spec: TE00 — Document AST
package documentast

// ─── Root Interface ────────────────────────────────────────────────────────────

// Node is the root interface for all AST nodes — both block and inline.
// Use type assertions or type switches to access concrete fields.
type Node interface {
	// nodeType returns a string tag for the node type.
	// This mirrors the TypeScript `type` field and is used in tests.
	NodeType() string
}

// BlockNode is any node that appears at the block level — the structural
// skeleton of a document. Block nodes live directly inside DocumentNode,
// BlockquoteNode, and ListItemNode.
type BlockNode interface {
	Node
	blockNode() // marker method, prevents accidental implementation
}

// InlineNode is any node that appears inside block nodes that contain prose:
// headings, paragraphs, and list items.
type InlineNode interface {
	Node
	inlineNode() // marker method
}

// ─── Block Node Types ──────────────────────────────────────────────────────────

// DocumentNode is the root of every document produced by a front-end parser.
//
// Every IR value is exactly one DocumentNode. An empty document has an empty
// Children slice. DocumentNode is the only node type that cannot appear as
// a child of another node.
//
//	DocumentNode
//	  ├── HeadingNode (level 1)
//	  ├── ParagraphNode
//	  └── ListNode (ordered, tight)
//	        ├── ListItemNode
//	        └── ListItemNode
type DocumentNode struct {
	Children []BlockNode
}

func (n *DocumentNode) NodeType() string { return "document" }
func (n *DocumentNode) blockNode()       {}

// HeadingNode is a section heading with a nesting depth.
//
// Semantically corresponds to <h1>–<h6> in HTML, ===== / ----- underlines
// in RST, \section{} / \subsection{} in LaTeX, and Heading 1–6 in DOCX.
//
// Levels beyond 6 (if a source format supports them) are clamped to 6.
//
//	HeadingNode { Level: 2, Children: [TextNode { Value: "Hello" }] }
//	→ <h2>Hello</h2>
type HeadingNode struct {
	// Level is the heading depth: 1 (most prominent) to 6 (least prominent).
	Level    int // 1–6
	Children []InlineNode
}

func (n *HeadingNode) NodeType() string { return "heading" }
func (n *HeadingNode) blockNode()       {}

// ParagraphNode is a block of prose containing one or more inline nodes.
//
// Paragraphs are the most common block type. Any content that is not more
// specifically typed (heading, list, code block, etc.) becomes a paragraph.
//
//	ParagraphNode {
//	  Children: [TextNode { Value: "Hello " }, EmphasisNode { ... }]
//	}
//	→ <p>Hello <em>world</em></p>
type ParagraphNode struct {
	Children []InlineNode
}

func (n *ParagraphNode) NodeType() string { return "paragraph" }
func (n *ParagraphNode) blockNode()       {}

// CodeBlockNode is a block of literal code or pre-formatted text.
//
// The Value is raw — it is NOT decoded for HTML entities and NOT processed
// for inline markup. The Value field always ends with \n.
//
// Syntax highlighting tools can use the Language hint (e.g. "typescript").
// Language is "" (empty string) when unknown, not nil, to simplify callers.
//
//	// Fenced code block:
//	// ```typescript
//	// const x = 1;
//	// ```
//	CodeBlockNode { Language: "typescript", Value: "const x = 1;\n" }
//	→ <pre><code class="language-typescript">const x = 1;</code></pre>
type CodeBlockNode struct {
	// Language is the syntax language hint, e.g. "typescript", "python".
	// Empty string "" when the info string was absent or empty.
	Language string
	// Value is the raw source code, including the trailing newline.
	// Never HTML-encoded.
	Value string
}

func (n *CodeBlockNode) NodeType() string { return "code_block" }
func (n *CodeBlockNode) blockNode()       {}

// BlockquoteNode is a block of content set apart as a quotation or aside.
//
// Can contain any block nodes, including nested blockquotes.
// In HTML renders as <blockquote>…</blockquote>.
//
//	BlockquoteNode {
//	  Children: [ParagraphNode { Children: [TextNode { Value: "quote" }] }]
//	}
//	→ <blockquote>\n<p>quote</p>\n</blockquote>
type BlockquoteNode struct {
	Children []BlockNode
}

func (n *BlockquoteNode) NodeType() string { return "blockquote" }
func (n *BlockquoteNode) blockNode()       {}

// ListNode is an ordered (numbered) or unordered (bulleted) list.
//
// A ListNode contains one or more ListItemNode children. Each list item
// contains block-level content (paragraphs, nested lists, code blocks, etc.).
//
// # Tight vs Loose
//
// The Tight flag is a rendering hint from the source.
// A tight list is written without blank lines between items; a loose list has
// blank lines. In HTML, tight lists suppress <p> wrappers around paragraph
// content. Other back-ends may use this flag differently or ignore it.
//
// # Ordered List Start
//
// Start records the opening item number. 1 is the default; 42 means the list
// begins at forty-two. 0 for unordered lists (Start has no semantic meaning
// for unordered lists).
//
//	ListNode { Ordered: false, Start: 0, Tight: true, Children: [...] }
//	→ <ul>\n<li>item1</li>\n<li>item2</li>\n</ul>
//
//	ListNode { Ordered: true, Start: 3, Tight: false, Children: [...] }
//	→ <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>
type ListNode struct {
	Ordered  bool
	// Start is the opening number for ordered lists (default 1).
	// For unordered lists, Start is 0 (meaningless).
	Start    int
	// Tight = no blank lines between items and no blank line inside any item.
	// In HTML tight mode, paragraph content is rendered without <p> tags.
	Tight    bool
	Children []*ListItemNode
}

func (n *ListNode) NodeType() string { return "list" }
func (n *ListNode) blockNode()       {}

// ListItemNode is one item in a ListNode. Contains block-level content.
//
// For tight lists, the children are typically ParagraphNodes whose content
// is rendered without wrapping <p> tags (the Tight flag on the parent
// ListNode controls this).
type ListItemNode struct {
	Children []BlockNode
}

func (n *ListItemNode) NodeType() string { return "list_item" }
func (n *ListItemNode) blockNode()       {}

// ThematicBreakNode is a visual separator between sections. Leaf node — no children.
//
// In HTML renders as <hr />. In RST: ----. In plain text: ---.
type ThematicBreakNode struct{}

func (n *ThematicBreakNode) NodeType() string { return "thematic_break" }
func (n *ThematicBreakNode) blockNode()       {}

// RawBlockNode is a block of raw content to be passed through verbatim to a
// specific back-end.
//
// The Format field identifies the target renderer (e.g. "html", "latex", "rtf").
// Back-ends that do not recognise Format MUST skip this node silently — they
// should not corrupt output with content intended for a different renderer.
//
// # Generalisation of HtmlBlockNode
//
// The CommonMark AST (TE01) has HtmlBlockNode { type: "html_block" }.
// The Document AST replaces it with RawBlockNode { Format: "html" }.
// The semantics are identical for HTML output; the Format tag extends the
// concept to any target format.
//
//	Back-end contract:
//	  format matches output → emit value verbatim (no escaping)
//	  format does not match → skip silently
//
//	Format    HTML back-end   LaTeX back-end  plain-text
//	────────  ─────────────   ──────────────  ──────────
//	"html"    emit            skip            skip
//	"latex"   skip            emit            skip
//	"rtf"     skip            skip            skip
type RawBlockNode struct {
	// Format is the target back-end format tag, e.g. "html", "latex", "rtf".
	Format string
	// Value is the raw content — never HTML-encoded or otherwise processed.
	Value string
}

func (n *RawBlockNode) NodeType() string { return "raw_block" }
func (n *RawBlockNode) blockNode()       {}

// ─── Inline Node Types ────────────────────────────────────────────────────────

// TextNode is plain text with no markup.
//
// All HTML character references (&amp;, &#65;, &#x41;) are decoded into
// their Unicode equivalents before being stored. The Value field contains
// the final, display-ready Unicode string.
//
// Adjacent text nodes are automatically merged during inline parsing — a
// well-formed IR never has two consecutive TextNode siblings.
//
//	"Hello &amp; world" → TextNode { Value: "Hello & world" }
type TextNode struct {
	// Value is the decoded Unicode string, ready for display.
	// Never contains raw HTML entities.
	Value string
}

func (n *TextNode) NodeType() string { return "text" }
func (n *TextNode) inlineNode()      {}

// EmphasisNode is stressed emphasis. In HTML renders as <em>.
// In Markdown: *text* or _text_. In RST: :emphasis:. In DOCX: italic.
//
//	EmphasisNode { Children: [TextNode { Value: "hello" }] }
//	→ <em>hello</em>
type EmphasisNode struct {
	Children []InlineNode
}

func (n *EmphasisNode) NodeType() string { return "emphasis" }
func (n *EmphasisNode) inlineNode()      {}

// StrongNode is strong importance. In HTML renders as <strong>.
// In Markdown: **text** or __text__. In RST: **bold**. In DOCX: bold.
//
//	StrongNode { Children: [TextNode { Value: "bold" }] }
//	→ <strong>bold</strong>
type StrongNode struct {
	Children []InlineNode
}

func (n *StrongNode) NodeType() string { return "strong" }
func (n *StrongNode) inlineNode()      {}

// CodeSpanNode is inline code. The value is raw — not decoded for HTML entities
// and not processed for Markdown. Leading and trailing spaces are stripped when
// the content is surrounded by spaces on both sides.
//
//	`const x = 1` → CodeSpanNode { Value: "const x = 1" }
//	→ <code>const x = 1</code>
type CodeSpanNode struct {
	// Value is the raw code content, not decoded.
	Value string
}

func (n *CodeSpanNode) NodeType() string { return "code_span" }
func (n *CodeSpanNode) inlineNode()      {}

// LinkNode is a hyperlink with resolved destination.
//
// The Destination is always a fully resolved URL — all reference indirections
// have been resolved by the front-end. The IR never contains unresolved
// reference links.
//
// Links cannot be nested — a LinkNode cannot contain another LinkNode.
//
//	LinkNode {
//	  Destination: "https://example.com",
//	  Title: "Example",
//	  Children: [TextNode { Value: "click here" }]
//	}
//	→ <a href="https://example.com" title="Example">click here</a>
type LinkNode struct {
	// Destination is the fully resolved URL. Never a [label] reference.
	Destination string
	// Title is the optional tooltip / hover text. Empty string if absent.
	// Use HasTitle to distinguish "empty title" from "no title".
	Title    string
	HasTitle bool
	Children []InlineNode
}

func (n *LinkNode) NodeType() string { return "link" }
func (n *LinkNode) inlineNode()      {}

// ImageNode is an embedded image.
//
// Like LinkNode, Destination is always the fully resolved URL. The Alt
// field is the plain-text fallback description (all inline markup stripped).
//
// # Alt text
//
// Alt is a plain string (not inline nodes) because alt text is by definition
// a plain-text description for screen readers and fallback contexts.
// For example, ![**hello**](img.png) produces ImageNode { Alt: "hello", … }
// — markup is stripped before storing.
//
//	ImageNode { Destination: "cat.png", Alt: "a cat" }
//	→ <img src="cat.png" alt="a cat" />
type ImageNode struct {
	// Destination is the fully resolved image URL.
	Destination string
	// Title is the optional tooltip / hover text. Empty string if absent.
	Title    string
	HasTitle bool
	// Alt is the plain-text alt description, markup stripped.
	Alt string
}

func (n *ImageNode) NodeType() string { return "image" }
func (n *ImageNode) inlineNode()      {}

// AutolinkNode is a URL or email address presented as a direct link,
// without custom link text. The link text in all back-ends is the raw
// address itself.
//
// # Why preserve IsEmail?
//
// 1. HTML back-ends need to prepend mailto: for email autolinks:
//    <https://example.com> → <a href="https://example.com">…</a> but
//    <user@example.com> → <a href="mailto:user@example.com">…</a>.
//
// 2. Other back-ends (PDF, DOCX) may format email addresses differently
//    from URLs — e.g. not underlining email addresses in print output.
//
//	AutolinkNode { Destination: "user@example.com", IsEmail: true }
//	→ <a href="mailto:user@example.com">user@example.com</a>
type AutolinkNode struct {
	// Destination is the URL or email address, without the surrounding < >.
	Destination string
	// IsEmail is true for email autolinks; false for URL autolinks.
	IsEmail bool
}

func (n *AutolinkNode) NodeType() string { return "autolink" }
func (n *AutolinkNode) inlineNode()      {}

// RawInlineNode is an inline span of raw content to be passed through verbatim
// to a specific back-end. The Format field names the target renderer.
//
// The same back-end contract applies as for RawBlockNode: emit verbatim if
// Format matches, skip silently if it does not.
//
//	RawInlineNode { Format: "html", Value: "<em>raw</em>" }
//	→ (HTML back-end) <em>raw</em>
//	→ (LaTeX back-end) (nothing)
type RawInlineNode struct {
	// Format is the target back-end format tag, e.g. "html", "latex".
	Format string
	// Value is the raw content — never escaped or processed.
	Value string
}

func (n *RawInlineNode) NodeType() string { return "raw_inline" }
func (n *RawInlineNode) inlineNode()      {}

// HardBreakNode is a forced line break within a paragraph.
//
// Forces <br /> in HTML, \newline in LaTeX, a literal \n in plain-text
// renderers. In Markdown, produced by two or more trailing spaces before a
// newline, or a backslash \ immediately before a newline.
type HardBreakNode struct{}

func (n *HardBreakNode) NodeType() string { return "hard_break" }
func (n *HardBreakNode) inlineNode()      {}

// SoftBreakNode is a soft line break — a newline within a paragraph that
// is not a hard break.
//
// In HTML, soft breaks render as \n (browsers collapse to a single space).
// In plain text, they render as a literal newline. The back-end controls
// the exact rendering.
//
// The IR preserves soft breaks so that back-ends controlling line-wrapping
// behaviour can make the right choice. A back-end may also discard soft
// breaks and re-wrap paragraphs independently.
type SoftBreakNode struct{}

func (n *SoftBreakNode) NodeType() string { return "soft_break" }
func (n *SoftBreakNode) inlineNode()      {}
