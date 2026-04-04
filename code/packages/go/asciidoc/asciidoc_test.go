package asciidoc

import (
	"strings"
	"testing"
)

func TestToHtmlHeading(t *testing.T) {
	html := ToHtml("= Hello\n")
	if !strings.Contains(html, "<h1>") || !strings.Contains(html, "Hello") {
		t.Errorf("expected h1 heading, got: %q", html)
	}
}

func TestToHtmlParagraph(t *testing.T) {
	html := ToHtml("Hello world\n")
	if !strings.Contains(html, "<p>") || !strings.Contains(html, "Hello world") {
		t.Errorf("expected paragraph, got: %q", html)
	}
}

func TestToHtmlStrong(t *testing.T) {
	html := ToHtml("Hello *world*\n")
	if !strings.Contains(html, "<strong>world</strong>") {
		t.Errorf("expected strong, got: %q", html)
	}
}

func TestToHtmlEmphasis(t *testing.T) {
	html := ToHtml("Hello _world_\n")
	if !strings.Contains(html, "<em>world</em>") {
		t.Errorf("expected em, got: %q", html)
	}
}

func TestToHtmlCodeBlock(t *testing.T) {
	html := ToHtml("[source,go]\n----\nfmt.Println()\n----\n")
	if !strings.Contains(html, "<pre>") || !strings.Contains(html, "fmt.Println") {
		t.Errorf("expected code block, got: %q", html)
	}
}

func TestToHtmlList(t *testing.T) {
	html := ToHtml("* foo\n* bar\n")
	if !strings.Contains(html, "<ul>") || !strings.Contains(html, "<li>") {
		t.Errorf("expected unordered list, got: %q", html)
	}
}

func TestToHtmlOrderedList(t *testing.T) {
	html := ToHtml(". alpha\n. beta\n")
	if !strings.Contains(html, "<ol") || !strings.Contains(html, "<li>") {
		t.Errorf("expected ordered list, got: %q", html)
	}
}

func TestToHtmlThematicBreak(t *testing.T) {
	html := ToHtml("'''\n")
	if !strings.Contains(html, "<hr") {
		t.Errorf("expected hr, got: %q", html)
	}
}

func TestToHtmlPassthrough(t *testing.T) {
	html := ToHtml("++++\n<div>raw</div>\n++++\n")
	if !strings.Contains(html, "<div>raw</div>") {
		t.Errorf("expected raw div passthrough, got: %q", html)
	}
}

func TestToHtmlSafeStripsPassthrough(t *testing.T) {
	html := ToHtmlSafe("++++\n<script>evil</script>\n++++\n\nSafe text\n")
	if strings.Contains(html, "<script>") {
		t.Errorf("expected script to be stripped, got: %q", html)
	}
	if !strings.Contains(html, "Safe text") {
		t.Errorf("expected safe text to be present, got: %q", html)
	}
}

func TestParseReturnsDocumentNode(t *testing.T) {
	doc := Parse("= Title\n")
	if doc == nil {
		t.Fatal("expected non-nil DocumentNode")
	}
	if len(doc.Children) != 1 {
		t.Errorf("expected 1 child, got %d", len(doc.Children))
	}
}

func TestVersion(t *testing.T) {
	if VERSION != "0.1.0" {
		t.Errorf("unexpected version: %q", VERSION)
	}
}
