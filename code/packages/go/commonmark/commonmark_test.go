package commonmark

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"testing"
)

// specExample represents one entry in the CommonMark spec test suite.
type specExample struct {
	Markdown  string `json:"markdown"`
	HTML      string `json:"html"`
	Example   int    `json:"example"`
	StartLine int    `json:"start_line"`
	EndLine   int    `json:"end_line"`
	Section   string `json:"section"`
}

// TestCommonMarkSpec runs all 652 CommonMark 0.31.2 spec examples.
//
// The spec JSON is loaded from spec.json (shipped alongside this package).
// Each example gives a Markdown input and the expected HTML output.
//
// If tests fail, the output shows the example number, section, and a
// clear diff so failures can be diagnosed against the spec.
func TestCommonMarkSpec(t *testing.T) {
	data, err := os.ReadFile("spec.json")
	if err != nil {
		t.Fatalf("could not read spec.json: %v\n(run tests from the package directory)", err)
	}

	var examples []specExample
	if err := json.Unmarshal(data, &examples); err != nil {
		t.Fatalf("could not parse spec.json: %v", err)
	}

	if len(examples) == 0 {
		t.Fatal("spec.json contained no examples")
	}

	t.Logf("Running %d CommonMark 0.31.2 spec examples", len(examples))

	passed := 0
	failed := 0

	for _, ex := range examples {
		ex := ex // capture loop variable
		t.Run(fmt.Sprintf("example_%d_%s", ex.Example, sanitizeSection(ex.Section)), func(t *testing.T) {
			got := ToHtml(ex.Markdown)
			if got != ex.HTML {
				failed++
				t.Errorf("Example %d (%s, lines %d-%d)\n"+
					"  Input:    %q\n"+
					"  Expected: %q\n"+
					"  Got:      %q",
					ex.Example, ex.Section, ex.StartLine, ex.EndLine,
					ex.Markdown, ex.HTML, got)
			} else {
				passed++
			}
		})
	}

	t.Logf("Results: %d passed, %d failed out of %d total", passed, failed, len(examples))
}

// sanitizeSection converts a section name to a valid Go test name component.
func sanitizeSection(s string) string {
	replacer := strings.NewReplacer(
		" ", "_",
		"/", "_",
		"(", "",
		")", "",
		"&", "and",
		"'", "",
	)
	return replacer.Replace(s)
}

// TestBasicParsing runs a few hand-crafted examples to verify the parser
// works for the most common Markdown constructs.
func TestBasicParsing(t *testing.T) {
	tests := []struct {
		name     string
		markdown string
		want     string
	}{
		{
			name:     "heading",
			markdown: "# Hello\n",
			want:     "<h1>Hello</h1>\n",
		},
		{
			name:     "paragraph",
			markdown: "Hello world.\n",
			want:     "<p>Hello world.</p>\n",
		},
		{
			name:     "emphasis",
			markdown: "*em* and **strong**\n",
			want:     "<p><em>em</em> and <strong>strong</strong></p>\n",
		},
		{
			name:     "code_block",
			markdown: "    code here\n",
			want:     "<pre><code>code here\n</code></pre>\n",
		},
		{
			name:     "fenced_code",
			markdown: "```go\nfmt.Println(\"hi\")\n```\n",
			want:     "<pre><code class=\"language-go\">fmt.Println(&quot;hi&quot;)\n</code></pre>\n",
		},
		{
			name:     "unordered_list",
			markdown: "- foo\n- bar\n",
			want:     "<ul>\n<li>foo</li>\n<li>bar</li>\n</ul>\n",
		},
		{
			name:     "ordered_list",
			markdown: "1. foo\n2. bar\n",
			want:     "<ol>\n<li>foo</li>\n<li>bar</li>\n</ol>\n",
		},
		{
			name:     "blockquote",
			markdown: "> quote\n",
			want:     "<blockquote>\n<p>quote</p>\n</blockquote>\n",
		},
		{
			name:     "inline_link",
			markdown: "[text](https://example.com)\n",
			want:     "<p><a href=\"https://example.com\">text</a></p>\n",
		},
		{
			name:     "thematic_break",
			markdown: "---\n",
			want:     "<hr />\n",
		},
		{
			name:     "hard_break",
			markdown: "foo  \nbar\n",
			want:     "<p>foo<br />\nbar</p>\n",
		},
		{
			name:     "code_span",
			markdown: "`code`\n",
			want:     "<p><code>code</code></p>\n",
		},
		{
			name:     "empty_document",
			markdown: "",
			want:     "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := ToHtml(tt.markdown)
			if got != tt.want {
				t.Errorf("ToHtml(%q)\n  got:  %q\n  want: %q", tt.markdown, got, tt.want)
			}
		})
	}
}

// TestSanitize verifies that ToHtmlSafe strips raw HTML.
func TestSanitize(t *testing.T) {
	markdown := "Hello\n\n<script>evil</script>\n"
	got := ToHtmlSafe(markdown)
	if strings.Contains(got, "<script>") {
		t.Errorf("ToHtmlSafe should strip <script> tags, got: %q", got)
	}
}
