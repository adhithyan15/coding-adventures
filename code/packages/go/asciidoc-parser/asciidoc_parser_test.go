package asciidocparser

import (
	"testing"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// ─── Helper ───────────────────────────────────────────────────────────────────

func text(s string) *documentast.TextNode { return &documentast.TextNode{Value: s} }

// ─── Block-level tests ────────────────────────────────────────────────────────

func TestParseEmpty(t *testing.T) {
	doc := Parse("")
	if len(doc.Children) != 0 {
		t.Errorf("expected 0 children, got %d", len(doc.Children))
	}
}

func TestParseBlankLines(t *testing.T) {
	doc := Parse("\n\n\n")
	if len(doc.Children) != 0 {
		t.Errorf("expected 0 children for blank-only input, got %d", len(doc.Children))
	}
}

func TestParseHeading1(t *testing.T) {
	doc := Parse("= Hello World\n")
	if len(doc.Children) != 1 {
		t.Fatalf("expected 1 child, got %d", len(doc.Children))
	}
	h, ok := doc.Children[0].(*documentast.HeadingNode)
	if !ok {
		t.Fatalf("expected HeadingNode, got %T", doc.Children[0])
	}
	if h.Level != 1 {
		t.Errorf("expected level 1, got %d", h.Level)
	}
	if len(h.Children) != 1 {
		t.Fatalf("expected 1 inline child, got %d", len(h.Children))
	}
	if h.Children[0].(*documentast.TextNode).Value != "Hello World" {
		t.Errorf("unexpected heading text: %q", h.Children[0].(*documentast.TextNode).Value)
	}
}

func TestParseHeading2(t *testing.T) {
	doc := Parse("== Section\n")
	h := doc.Children[0].(*documentast.HeadingNode)
	if h.Level != 2 {
		t.Errorf("expected level 2, got %d", h.Level)
	}
}

func TestParseHeading6(t *testing.T) {
	doc := Parse("====== Deep\n")
	h := doc.Children[0].(*documentast.HeadingNode)
	if h.Level != 6 {
		t.Errorf("expected level 6, got %d", h.Level)
	}
}

func TestParseParagraph(t *testing.T) {
	doc := Parse("Hello world\n")
	if len(doc.Children) != 1 {
		t.Fatalf("expected 1 child, got %d", len(doc.Children))
	}
	p, ok := doc.Children[0].(*documentast.ParagraphNode)
	if !ok {
		t.Fatalf("expected ParagraphNode, got %T", doc.Children[0])
	}
	if p.Children[0].(*documentast.TextNode).Value != "Hello world" {
		t.Errorf("unexpected paragraph text")
	}
}

func TestParseMultiLineParagraph(t *testing.T) {
	doc := Parse("Line one\nLine two\n")
	if len(doc.Children) != 1 {
		t.Fatalf("expected 1 paragraph, got %d", len(doc.Children))
	}
}

func TestParseTwoParagraphs(t *testing.T) {
	doc := Parse("First\n\nSecond\n")
	if len(doc.Children) != 2 {
		t.Fatalf("expected 2 children, got %d", len(doc.Children))
	}
	_, ok1 := doc.Children[0].(*documentast.ParagraphNode)
	_, ok2 := doc.Children[1].(*documentast.ParagraphNode)
	if !ok1 || !ok2 {
		t.Error("expected both children to be ParagraphNode")
	}
}

func TestParseThematicBreak(t *testing.T) {
	doc := Parse("'''\n")
	if len(doc.Children) != 1 {
		t.Fatalf("expected 1 child, got %d", len(doc.Children))
	}
	if _, ok := doc.Children[0].(*documentast.ThematicBreakNode); !ok {
		t.Errorf("expected ThematicBreakNode, got %T", doc.Children[0])
	}
}

func TestParseCodeBlock(t *testing.T) {
	doc := Parse("----\nfoo := bar\n----\n")
	if len(doc.Children) != 1 {
		t.Fatalf("expected 1 child, got %d", len(doc.Children))
	}
	cb, ok := doc.Children[0].(*documentast.CodeBlockNode)
	if !ok {
		t.Fatalf("expected CodeBlockNode, got %T", doc.Children[0])
	}
	if cb.Language != "" {
		t.Errorf("expected no language, got %q", cb.Language)
	}
	if cb.Value != "foo := bar\n" {
		t.Errorf("unexpected code value: %q", cb.Value)
	}
}

func TestParseCodeBlockWithLanguage(t *testing.T) {
	doc := Parse("[source,go]\n----\nfmt.Println()\n----\n")
	cb := doc.Children[0].(*documentast.CodeBlockNode)
	if cb.Language != "go" {
		t.Errorf("expected language 'go', got %q", cb.Language)
	}
}

func TestParseLiteralBlock(t *testing.T) {
	doc := Parse("....\nsome literal\n....\n")
	cb, ok := doc.Children[0].(*documentast.CodeBlockNode)
	if !ok {
		t.Fatalf("expected CodeBlockNode for literal block, got %T", doc.Children[0])
	}
	if cb.Language != "" {
		t.Errorf("expected no language for literal block")
	}
}

func TestParsePassthroughBlock(t *testing.T) {
	doc := Parse("++++\n<div>raw</div>\n++++\n")
	rb, ok := doc.Children[0].(*documentast.RawBlockNode)
	if !ok {
		t.Fatalf("expected RawBlockNode, got %T", doc.Children[0])
	}
	if rb.Format != "html" {
		t.Errorf("expected format 'html', got %q", rb.Format)
	}
	if rb.Value != "<div>raw</div>" {
		t.Errorf("unexpected passthrough value: %q", rb.Value)
	}
}

func TestParseQuoteBlock(t *testing.T) {
	doc := Parse("____\nSome quote\n____\n")
	bq, ok := doc.Children[0].(*documentast.BlockquoteNode)
	if !ok {
		t.Fatalf("expected BlockquoteNode, got %T", doc.Children[0])
	}
	if len(bq.Children) != 1 {
		t.Errorf("expected 1 child inside blockquote, got %d", len(bq.Children))
	}
}

func TestParseUnorderedList(t *testing.T) {
	doc := Parse("* item one\n* item two\n")
	if len(doc.Children) != 1 {
		t.Fatalf("expected 1 list node, got %d", len(doc.Children))
	}
	list, ok := doc.Children[0].(*documentast.ListNode)
	if !ok {
		t.Fatalf("expected ListNode, got %T", doc.Children[0])
	}
	if list.Ordered {
		t.Error("expected unordered list")
	}
	if len(list.Children) != 2 {
		t.Errorf("expected 2 items, got %d", len(list.Children))
	}
}

func TestParseOrderedList(t *testing.T) {
	doc := Parse(". first\n. second\n. third\n")
	list, ok := doc.Children[0].(*documentast.ListNode)
	if !ok {
		t.Fatalf("expected ListNode, got %T", doc.Children[0])
	}
	if !list.Ordered {
		t.Error("expected ordered list")
	}
	if len(list.Children) != 3 {
		t.Errorf("expected 3 items, got %d", len(list.Children))
	}
}

func TestParseComment(t *testing.T) {
	doc := Parse("// this is a comment\nHello\n")
	if len(doc.Children) != 1 {
		t.Fatalf("expected 1 child (comment skipped), got %d", len(doc.Children))
	}
	if _, ok := doc.Children[0].(*documentast.ParagraphNode); !ok {
		t.Error("expected paragraph after comment")
	}
}

func TestParseHeadingThenParagraph(t *testing.T) {
	doc := Parse("= Title\n\nSome text.\n")
	if len(doc.Children) != 2 {
		t.Fatalf("expected 2 children, got %d", len(doc.Children))
	}
	if _, ok := doc.Children[0].(*documentast.HeadingNode); !ok {
		t.Error("expected first child to be HeadingNode")
	}
	if _, ok := doc.Children[1].(*documentast.ParagraphNode); !ok {
		t.Error("expected second child to be ParagraphNode")
	}
}

// ─── Inline tests ─────────────────────────────────────────────────────────────

func TestInlineStrong(t *testing.T) {
	// In AsciiDoc, *text* = strong (NOT emphasis!)
	nodes := parseInlines("Hello *world*!")
	// Expected: text("Hello "), strong("world"), text("!")
	found := false
	for _, n := range nodes {
		if s, ok := n.(*documentast.StrongNode); ok {
			if len(s.Children) == 1 {
				if t2, ok2 := s.Children[0].(*documentast.TextNode); ok2 && t2.Value == "world" {
					found = true
				}
			}
		}
	}
	if !found {
		t.Errorf("expected StrongNode containing 'world', got %+v", nodes)
	}
}

func TestInlineEmphasis(t *testing.T) {
	// In AsciiDoc, _text_ = emphasis
	nodes := parseInlines("Hello _world_!")
	found := false
	for _, n := range nodes {
		if e, ok := n.(*documentast.EmphasisNode); ok {
			if len(e.Children) == 1 {
				if t2, ok2 := e.Children[0].(*documentast.TextNode); ok2 && t2.Value == "world" {
					found = true
				}
			}
		}
	}
	if !found {
		t.Errorf("expected EmphasisNode containing 'world', got %+v", nodes)
	}
}

func TestInlineStrongUnconstrained(t *testing.T) {
	nodes := parseInlines("**bold**")
	if len(nodes) != 1 {
		t.Fatalf("expected 1 node, got %d", len(nodes))
	}
	if _, ok := nodes[0].(*documentast.StrongNode); !ok {
		t.Errorf("expected StrongNode, got %T", nodes[0])
	}
}

func TestInlineEmphasisUnconstrained(t *testing.T) {
	nodes := parseInlines("__em__")
	if len(nodes) != 1 {
		t.Fatalf("expected 1 node, got %d", len(nodes))
	}
	if _, ok := nodes[0].(*documentast.EmphasisNode); !ok {
		t.Errorf("expected EmphasisNode, got %T", nodes[0])
	}
}

func TestInlineCodeSpan(t *testing.T) {
	nodes := parseInlines("Use `foo()` now")
	found := false
	for _, n := range nodes {
		if cs, ok := n.(*documentast.CodeSpanNode); ok && cs.Value == "foo()" {
			found = true
		}
	}
	if !found {
		t.Errorf("expected CodeSpanNode with 'foo()', got %+v", nodes)
	}
}

func TestInlineLinkMacro(t *testing.T) {
	nodes := parseInlines("See link:https://example.com[Example] for more.")
	found := false
	for _, n := range nodes {
		if ln, ok := n.(*documentast.LinkNode); ok {
			if ln.Destination == "https://example.com" {
				found = true
			}
		}
	}
	if !found {
		t.Errorf("expected LinkNode to https://example.com, got %+v", nodes)
	}
}

func TestInlineImageMacro(t *testing.T) {
	nodes := parseInlines("image:cat.png[A cat]")
	if len(nodes) != 1 {
		t.Fatalf("expected 1 node, got %d", len(nodes))
	}
	img, ok := nodes[0].(*documentast.ImageNode)
	if !ok {
		t.Fatalf("expected ImageNode, got %T", nodes[0])
	}
	if img.Destination != "cat.png" {
		t.Errorf("unexpected destination: %q", img.Destination)
	}
	if img.Alt != "A cat" {
		t.Errorf("unexpected alt: %q", img.Alt)
	}
}

func TestInlineCrossRef(t *testing.T) {
	nodes := parseInlines("See <<section-id,Section Title>>.")
	found := false
	for _, n := range nodes {
		if ln, ok := n.(*documentast.LinkNode); ok {
			if ln.Destination == "#section-id" {
				found = true
			}
		}
	}
	if !found {
		t.Errorf("expected cross-ref link to #section-id, got %+v", nodes)
	}
}

func TestInlineCrossRefNoText(t *testing.T) {
	nodes := parseInlines("<<my-anchor>>")
	if len(nodes) != 1 {
		t.Fatalf("expected 1 node, got %d", len(nodes))
	}
	ln, ok := nodes[0].(*documentast.LinkNode)
	if !ok {
		t.Fatalf("expected LinkNode, got %T", nodes[0])
	}
	if ln.Destination != "#my-anchor" {
		t.Errorf("unexpected destination: %q", ln.Destination)
	}
}

func TestInlineAutolink(t *testing.T) {
	nodes := parseInlines("Visit https://example.com for details.")
	found := false
	for _, n := range nodes {
		if al, ok := n.(*documentast.AutolinkNode); ok {
			if al.Destination == "https://example.com" {
				found = true
			}
		}
	}
	if !found {
		t.Errorf("expected AutolinkNode to https://example.com, got %+v", nodes)
	}
}

func TestInlineURLWithBrackets(t *testing.T) {
	nodes := parseInlines("https://example.com[Click here]")
	if len(nodes) != 1 {
		t.Fatalf("expected 1 node, got %d", len(nodes))
	}
	ln, ok := nodes[0].(*documentast.LinkNode)
	if !ok {
		t.Fatalf("expected LinkNode, got %T", nodes[0])
	}
	if ln.Destination != "https://example.com" {
		t.Errorf("unexpected destination: %q", ln.Destination)
	}
}

func TestInlineSoftBreak(t *testing.T) {
	nodes := parseInlines("line one\nline two")
	found := false
	for _, n := range nodes {
		if _, ok := n.(*documentast.SoftBreakNode); ok {
			found = true
		}
	}
	if !found {
		t.Errorf("expected SoftBreakNode in %+v", nodes)
	}
}

func TestInlineHardBreakTwoSpaces(t *testing.T) {
	nodes := parseInlines("line one  \nline two")
	found := false
	for _, n := range nodes {
		if _, ok := n.(*documentast.HardBreakNode); ok {
			found = true
		}
	}
	if !found {
		t.Errorf("expected HardBreakNode in %+v", nodes)
	}
}

func TestInlineHardBreakBackslash(t *testing.T) {
	nodes := parseInlines("line one\\\nline two")
	found := false
	for _, n := range nodes {
		if _, ok := n.(*documentast.HardBreakNode); ok {
			found = true
		}
	}
	if !found {
		t.Errorf("expected HardBreakNode in %+v", nodes)
	}
}

func TestInlinePlainText(t *testing.T) {
	nodes := parseInlines("just plain text")
	if len(nodes) != 1 {
		t.Fatalf("expected 1 node, got %d", len(nodes))
	}
	if tn, ok := nodes[0].(*documentast.TextNode); !ok || tn.Value != "just plain text" {
		t.Errorf("unexpected node: %+v", nodes[0])
	}
}

func TestNodeType(t *testing.T) {
	doc := Parse("= H\n\nPara\n")
	if doc.Children[0].NodeType() != "heading" {
		t.Errorf("expected 'heading', got %q", doc.Children[0].NodeType())
	}
	if doc.Children[1].NodeType() != "paragraph" {
		t.Errorf("expected 'paragraph', got %q", doc.Children[1].NodeType())
	}
}

func TestParseNestedQuoteBlock(t *testing.T) {
	// Blockquote containing a heading
	doc := Parse("____\n== Inner Heading\n____\n")
	bq, ok := doc.Children[0].(*documentast.BlockquoteNode)
	if !ok {
		t.Fatalf("expected BlockquoteNode, got %T", doc.Children[0])
	}
	if len(bq.Children) < 1 {
		t.Fatal("expected children in blockquote")
	}
	if _, ok := bq.Children[0].(*documentast.HeadingNode); !ok {
		t.Errorf("expected HeadingNode inside blockquote, got %T", bq.Children[0])
	}
}

func TestVersion(t *testing.T) {
	if VERSION != "0.1.0" {
		t.Errorf("unexpected version: %q", VERSION)
	}
}
