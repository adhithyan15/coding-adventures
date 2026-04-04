// Package asciidoc is a thin pipeline package that combines the AsciiDoc
// parser and HTML renderer into a single convenient API.
//
// It re-exports the Parse function from asciidoc-parser and provides ToHtml
// which pipes through document-ast-to-html, giving a single import point for
// the most common use case: AsciiDoc → HTML.
//
// # Quick Start
//
//	import "github.com/adhithyan15/coding-adventures/code/packages/go/asciidoc"
//
//	// Parse AsciiDoc to HTML:
//	html := asciidoc.ToHtml("= Hello\n\nWorld *with* bold.\n")
//	// → "<h1>Hello</h1>\n<p>World <strong>with</strong> bold.</p>\n"
//
//	// Parse only (get the DocumentNode AST):
//	doc := asciidoc.Parse("= Hello\n")
//	// doc.Children[0].NodeType() == "heading"
//
// # Architecture
//
// This package is a thin wrapper. The actual implementation is in:
//   - asciidoc-parser: AsciiDoc text → DocumentNode (2-phase parsing)
//   - document-ast-to-html: DocumentNode → HTML string
//   - document-ast: the shared intermediate representation types
//
// Spec: TE03 (AsciiDoc parser), TE00 (Document AST), TE02 (HTML renderer)
package asciidoc

import (
	parser "github.com/adhithyan15/coding-adventures/code/packages/go/asciidoc-parser"
	renderer "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-to-html"
	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// Parse parses an AsciiDoc string into a DocumentNode AST.
//
// The result conforms to the Document AST spec (TE00). All inline markup is
// parsed and all block structure is resolved.
//
//	doc := asciidoc.Parse("= Hello\n\nWorld\n")
//	doc.Children[0].NodeType()  // "heading"
func Parse(text string) *documentast.DocumentNode {
	return parser.Parse(text)
}

// ToHtml parses AsciiDoc text and renders it to an HTML string.
//
// Raw HTML passthrough blocks (++++ ... ++++) are rendered verbatim.
// For untrusted input, use ToHtmlSafe instead.
//
//	html := asciidoc.ToHtml("= Hello\n\nWorld\n")
//	// → "<h1>Hello</h1>\n<p>World</p>\n"
func ToHtml(text string) string {
	doc := parser.Parse(text)
	return renderer.ToHtml(doc, renderer.RenderOptions{})
}

// ToHtmlSafe parses AsciiDoc text and renders HTML with all raw HTML stripped.
// Use this when rendering untrusted user-provided AsciiDoc content.
//
//	html := asciidoc.ToHtmlSafe("Some text\n\n++++\n<script>evil</script>\n++++\n")
//	// The passthrough block is stripped.
func ToHtmlSafe(text string) string {
	doc := parser.Parse(text)
	return renderer.ToHtml(doc, renderer.RenderOptions{Sanitize: true})
}

// VERSION is the version of this asciidoc package.
const VERSION = "0.1.0"
