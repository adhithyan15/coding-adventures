package documentasttohtml

import (
	"testing"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// TestToHtml verifies the renderer for all major node types.
func TestToHtml(t *testing.T) {
	tests := []struct {
		name string
		doc  *documentast.DocumentNode
		want string
	}{
		{
			name: "empty_document",
			doc:  &documentast.DocumentNode{},
			want: "",
		},
		{
			name: "heading_h1",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.HeadingNode{Level: 1, Children: []documentast.InlineNode{
					&documentast.TextNode{Value: "Hello"},
				}},
			}},
			want: "<h1>Hello</h1>\n",
		},
		{
			name: "heading_h6",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.HeadingNode{Level: 6, Children: []documentast.InlineNode{
					&documentast.TextNode{Value: "Deep"},
				}},
			}},
			want: "<h6>Deep</h6>\n",
		},
		{
			name: "paragraph",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.ParagraphNode{Children: []documentast.InlineNode{
					&documentast.TextNode{Value: "Hello world."},
				}},
			}},
			want: "<p>Hello world.</p>\n",
		},
		{
			name: "code_block_no_lang",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.CodeBlockNode{Value: "code here\n"},
			}},
			want: "<pre><code>code here\n</code></pre>\n",
		},
		{
			name: "code_block_with_lang",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.CodeBlockNode{Language: "go", Value: "fmt.Println()\n"},
			}},
			want: "<pre><code class=\"language-go\">fmt.Println()\n</code></pre>\n",
		},
		{
			name: "blockquote",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.BlockquoteNode{Children: []documentast.BlockNode{
					&documentast.ParagraphNode{Children: []documentast.InlineNode{
						&documentast.TextNode{Value: "quote"},
					}},
				}},
			}},
			want: "<blockquote>\n<p>quote</p>\n</blockquote>\n",
		},
		{
			name: "thematic_break",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.ThematicBreakNode{},
			}},
			want: "<hr />\n",
		},
		{
			name: "unordered_list_tight",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.ListNode{Ordered: false, Tight: true, Children: []*documentast.ListItemNode{
					{Children: []documentast.BlockNode{&documentast.ParagraphNode{Children: []documentast.InlineNode{&documentast.TextNode{Value: "foo"}}}}},
					{Children: []documentast.BlockNode{&documentast.ParagraphNode{Children: []documentast.InlineNode{&documentast.TextNode{Value: "bar"}}}}},
				}},
			}},
			want: "<ul>\n<li>foo</li>\n<li>bar</li>\n</ul>\n",
		},
		{
			name: "ordered_list_start_1",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.ListNode{Ordered: true, Start: 1, Tight: true, Children: []*documentast.ListItemNode{
					{Children: []documentast.BlockNode{&documentast.ParagraphNode{Children: []documentast.InlineNode{&documentast.TextNode{Value: "item"}}}}},
				}},
			}},
			want: "<ol>\n<li>item</li>\n</ol>\n",
		},
		{
			name: "ordered_list_start_0",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.ListNode{Ordered: true, Start: 0, Tight: true, Children: []*documentast.ListItemNode{
					{Children: []documentast.BlockNode{&documentast.ParagraphNode{Children: []documentast.InlineNode{&documentast.TextNode{Value: "item"}}}}},
				}},
			}},
			want: "<ol start=\"0\">\n<li>item</li>\n</ol>\n",
		},
		{
			name: "ordered_list_start_3",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.ListNode{Ordered: true, Start: 3, Tight: true, Children: []*documentast.ListItemNode{
					{Children: []documentast.BlockNode{&documentast.ParagraphNode{Children: []documentast.InlineNode{&documentast.TextNode{Value: "item"}}}}},
				}},
			}},
			want: "<ol start=\"3\">\n<li>item</li>\n</ol>\n",
		},
		{
			name: "raw_block_html",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.RawBlockNode{Format: "html", Value: "<div>raw</div>\n"},
			}},
			want: "<div>raw</div>\n",
		},
		{
			name: "raw_block_non_html_skipped",
			doc: &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.RawBlockNode{Format: "latex", Value: "\\textbf{X}\n"},
			}},
			want: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := ToHtml(tt.doc, RenderOptions{})
			if got != tt.want {
				t.Errorf("ToHtml()\n  got:  %q\n  want: %q", got, tt.want)
			}
		})
	}
}

// TestInlineRendering exercises all inline node types through a paragraph.
func TestInlineRendering(t *testing.T) {
	para := func(children ...documentast.InlineNode) *documentast.DocumentNode {
		return &documentast.DocumentNode{Children: []documentast.BlockNode{
			&documentast.ParagraphNode{Children: children},
		}}
	}

	tests := []struct {
		name string
		doc  *documentast.DocumentNode
		want string
	}{
		{
			name: "text_escaping",
			doc:  para(&documentast.TextNode{Value: "a & b < c > d \" e"}),
			want: "<p>a &amp; b &lt; c &gt; d &quot; e</p>\n",
		},
		{
			name: "emphasis",
			doc:  para(&documentast.EmphasisNode{Children: []documentast.InlineNode{&documentast.TextNode{Value: "em"}}}),
			want: "<p><em>em</em></p>\n",
		},
		{
			name: "strong",
			doc:  para(&documentast.StrongNode{Children: []documentast.InlineNode{&documentast.TextNode{Value: "bold"}}}),
			want: "<p><strong>bold</strong></p>\n",
		},
		{
			name: "code_span",
			doc:  para(&documentast.CodeSpanNode{Value: "code"}),
			want: "<p><code>code</code></p>\n",
		},
		{
			name: "link",
			doc: para(&documentast.LinkNode{
				Destination: "https://example.com",
				Children:    []documentast.InlineNode{&documentast.TextNode{Value: "link"}},
			}),
			want: "<p><a href=\"https://example.com\">link</a></p>\n",
		},
		{
			name: "link_with_title",
			doc: para(&documentast.LinkNode{
				Destination: "https://example.com",
				Title:       "My title",
				HasTitle:    true,
				Children:    []documentast.InlineNode{&documentast.TextNode{Value: "link"}},
			}),
			want: "<p><a href=\"https://example.com\" title=\"My title\">link</a></p>\n",
		},
		{
			name: "image",
			doc: para(&documentast.ImageNode{
				Destination: "img.png",
				Alt:         "alt text",
			}),
			want: "<p><img src=\"img.png\" alt=\"alt text\" /></p>\n",
		},
		{
			name: "autolink_url",
			doc:  para(&documentast.AutolinkNode{Destination: "https://example.com", IsEmail: false}),
			want: "<p><a href=\"https://example.com\">https://example.com</a></p>\n",
		},
		{
			name: "autolink_email",
			doc:  para(&documentast.AutolinkNode{Destination: "user@example.com", IsEmail: true}),
			want: "<p><a href=\"mailto:user@example.com\">user@example.com</a></p>\n",
		},
		{
			name: "hard_break",
			doc:  para(&documentast.TextNode{Value: "foo"}, &documentast.HardBreakNode{}, &documentast.TextNode{Value: "bar"}),
			want: "<p>foo<br />\nbar</p>\n",
		},
		{
			name: "soft_break",
			doc:  para(&documentast.TextNode{Value: "foo"}, &documentast.SoftBreakNode{}, &documentast.TextNode{Value: "bar"}),
			want: "<p>foo\nbar</p>\n",
		},
		{
			name: "raw_inline_html",
			doc:  para(&documentast.RawInlineNode{Format: "html", Value: "<em>raw</em>"}),
			want: "<p><em>raw</em></p>\n",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := ToHtml(tt.doc, RenderOptions{})
			if got != tt.want {
				t.Errorf("ToHtml()\n  got:  %q\n  want: %q", got, tt.want)
			}
		})
	}
}

// TestSanitize verifies that dangerous URL schemes are blocked.
func TestSanitize(t *testing.T) {
	tests := []struct {
		name string
		url  string
		safe bool
	}{
		{"javascript", "javascript:alert(1)", false},
		{"JAVASCRIPT_upper", "JAVASCRIPT:alert(1)", false},
		{"vbscript", "vbscript:msgbox(1)", false},
		{"data", "data:text/html,<h1>hi</h1>", false},
		{"blob", "blob:https://example.com/abc", false},
		{"https_ok", "https://example.com", true},
		{"http_ok", "http://example.com", true},
		{"relative_ok", "/path/to/page", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			doc := &documentast.DocumentNode{Children: []documentast.BlockNode{
				&documentast.ParagraphNode{Children: []documentast.InlineNode{
					&documentast.LinkNode{
						Destination: tt.url,
						Children:    []documentast.InlineNode{&documentast.TextNode{Value: "click"}},
					},
				}},
			}}
			got := ToHtml(doc, RenderOptions{Sanitize: true})
			if tt.safe {
				// URL should be preserved (not replaced with empty string)
				if got == "<p><a href=\"\">click</a></p>\n" {
					t.Errorf("URL %q was blocked but should be allowed; got: %q", tt.url, got)
				}
			} else {
				// Dangerous URL should be replaced with empty string by sanitizeURL
				if got != "<p><a href=\"\">click</a></p>\n" {
					t.Errorf("URL %q should be blocked (href should be empty); got: %q", tt.url, got)
				}
			}
		})
	}
}

// TestEscapeHtml verifies that all special HTML characters are escaped.
func TestEscapeHtml(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{"plain", "plain"},
		{"<br>", "&lt;br&gt;"},
		{"a & b", "a &amp; b"},
		{`say "hi"`, "say &quot;hi&quot;"},
		{"a'b", "a'b"},
		{"<a href=\"url\">text</a>", "&lt;a href=&quot;url&quot;&gt;text&lt;/a&gt;"},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got := EscapeHtml(tt.input)
			if got != tt.want {
				t.Errorf("EscapeHtml(%q) = %q, want %q", tt.input, got, tt.want)
			}
		})
	}
}
