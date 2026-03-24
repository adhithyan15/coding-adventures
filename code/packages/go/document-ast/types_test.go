package documentast

import "testing"

// TestNodeTypes verifies that every node type implements the Node interface
// and returns the correct type string. These are table-driven tests following
// the Go convention.
func TestBlockNodeTypes(t *testing.T) {
	tests := []struct {
		name     string
		node     BlockNode
		wantType string
	}{
		{"document", &DocumentNode{}, "document"},
		{"heading", &HeadingNode{Level: 1}, "heading"},
		{"paragraph", &ParagraphNode{}, "paragraph"},
		{"code_block", &CodeBlockNode{}, "code_block"},
		{"blockquote", &BlockquoteNode{}, "blockquote"},
		{"list", &ListNode{}, "list"},
		{"list_item", &ListItemNode{}, "list_item"},
		{"thematic_break", &ThematicBreakNode{}, "thematic_break"},
		{"raw_block", &RawBlockNode{Format: "html"}, "raw_block"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.node.NodeType()
			if got != tt.wantType {
				t.Errorf("NodeType() = %q, want %q", got, tt.wantType)
			}
		})
	}
}

func TestInlineNodeTypes(t *testing.T) {
	tests := []struct {
		name     string
		node     InlineNode
		wantType string
	}{
		{"text", &TextNode{Value: "hello"}, "text"},
		{"emphasis", &EmphasisNode{}, "emphasis"},
		{"strong", &StrongNode{}, "strong"},
		{"code_span", &CodeSpanNode{Value: "x"}, "code_span"},
		{"link", &LinkNode{Destination: "/"}, "link"},
		{"image", &ImageNode{Destination: "img.png"}, "image"},
		{"autolink", &AutolinkNode{Destination: "https://example.com"}, "autolink"},
		{"raw_inline", &RawInlineNode{Format: "html"}, "raw_inline"},
		{"hard_break", &HardBreakNode{}, "hard_break"},
		{"soft_break", &SoftBreakNode{}, "soft_break"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.node.NodeType()
			if got != tt.wantType {
				t.Errorf("NodeType() = %q, want %q", got, tt.wantType)
			}
		})
	}
}

// TestDocumentNodeIsAlsoNode verifies that DocumentNode satisfies both
// the Node and BlockNode interfaces (it's a block node at the root).
func TestDocumentNodeIsAlsoNode(t *testing.T) {
	var doc Node = &DocumentNode{}
	if doc.NodeType() != "document" {
		t.Errorf("DocumentNode as Node: got %q, want %q", doc.NodeType(), "document")
	}
}

// TestListNodeFields verifies the list node fields are accessible.
func TestListNodeFields(t *testing.T) {
	item := &ListItemNode{Children: []BlockNode{&ParagraphNode{}}}
	list := &ListNode{
		Ordered:  true,
		Start:    3,
		Tight:    false,
		Children: []*ListItemNode{item},
	}
	if !list.Ordered {
		t.Error("expected Ordered = true")
	}
	if list.Start != 3 {
		t.Errorf("Start = %d, want 3", list.Start)
	}
	if list.Tight {
		t.Error("expected Tight = false")
	}
	if len(list.Children) != 1 {
		t.Errorf("len(Children) = %d, want 1", len(list.Children))
	}
}

// TestLinkNodeHasTitle verifies the HasTitle flag works correctly.
func TestLinkNodeHasTitle(t *testing.T) {
	withTitle := &LinkNode{Destination: "/", Title: "My title", HasTitle: true}
	withoutTitle := &LinkNode{Destination: "/"}

	if !withTitle.HasTitle {
		t.Error("expected HasTitle = true")
	}
	if withTitle.Title != "My title" {
		t.Errorf("Title = %q, want %q", withTitle.Title, "My title")
	}
	if withoutTitle.HasTitle {
		t.Error("expected HasTitle = false for link without title")
	}
}
