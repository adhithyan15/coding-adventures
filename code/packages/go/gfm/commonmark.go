// Package commonmark is a thin pipeline package that combines the GFM
// parser and HTML renderer into a single convenient API.
//
// It re-exports the Parse function from commonmark-parser and the ToHtml
// function from document-ast-to-html, providing a single import point for
// the most common use case: Markdown → HTML.
//
// # Quick Start
//
//	import "github.com/adhithyan15/coding-adventures/code/packages/go/commonmark"
//
//	// Parse Markdown to HTML (GFM 0.31.2 spec compliance):
//	html := commonmark.ToHtml("# Hello\n\nWorld *with* emphasis.\n")
//	// → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"
//
//	// Parse only (get the DocumentNode AST):
//	doc := commonmark.Parse("# Hello\n")
//	// doc.Children[0].NodeType() == "heading"
//
//	// Render user-provided Markdown safely (strip raw HTML):
//	safeHtml := commonmark.ToHtmlSafe("# Hello\n\n<script>evil</script>\n")
//
// # Architecture
//
// This package is a thin wrapper. The actual implementation is in:
//   - commonmark-parser: Markdown → DocumentNode (2-phase parsing)
//   - document-ast-to-html: DocumentNode → HTML string
//   - document-ast: the shared intermediate representation types
//
// Spec: TE00 (Document AST), TE01 (GFM parser), TE02 (HTML renderer)
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery. This
// package declares zero OS capabilities, so no op.File / op.Net
// namespace fields are available inside callbacks.
package commonmark

import (
	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
	renderer "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-to-html"
	parser "github.com/adhithyan15/coding-adventures/code/packages/go/gfm-parser"
)

// Parse parses a GitHub Flavored Markdown string into a DocumentNode AST.
//
// The result conforms to the Document AST spec (TE00). All link references
// are resolved and all inline markup is parsed.
//
//	doc := commonmark.Parse("# Hello\n\nWorld\n")
//	doc.Children[0].NodeType()  // "heading"
func Parse(markdown string) *documentast.DocumentNode {
	result, _ := StartNew[*documentast.DocumentNode]("gfm.Parse", nil,
		func(op *Operation[*documentast.DocumentNode], rf *ResultFactory[*documentast.DocumentNode]) *OperationResult[*documentast.DocumentNode] {
			op.AddProperty("markdownLen", len(markdown))
			return rf.Generate(true, false, parser.Parse(markdown))
		}).GetResult()
	return result
}

// ToHtml parses Markdown and renders it to an HTML string.
//
// Raw HTML passthrough is enabled by default (required for GFM spec
// compliance). If you render untrusted Markdown (user content), use ToHtmlSafe
// instead to strip raw HTML from the output.
//
//	html := commonmark.ToHtml("# Hello\n\nWorld\n")
//	// → "<h1>Hello</h1>\n<p>World</p>\n"
func ToHtml(markdown string) string {
	result, _ := StartNew[string]("gfm.ToHtml", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("markdownLen", len(markdown))
			doc := parser.Parse(markdown)
			return rf.Generate(true, false, renderer.ToHtml(doc, renderer.RenderOptions{}))
		}).GetResult()
	return result
}

// ToHtmlSafe parses Markdown and renders it to an HTML string with all raw
// HTML stripped. Use this when rendering untrusted user-provided Markdown.
//
//	html := commonmark.ToHtmlSafe("Hello\n\n<script>evil</script>\n")
//	// → "<p>Hello</p>\n"  (the <script> is dropped)
func ToHtmlSafe(markdown string) string {
	result, _ := StartNew[string]("gfm.ToHtmlSafe", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("markdownLen", len(markdown))
			doc := parser.Parse(markdown)
			return rf.Generate(true, false, renderer.ToHtml(doc, renderer.RenderOptions{Sanitize: true}))
		}).GetResult()
	return result
}

// VERSION is the version of this commonmark package.
const VERSION = "0.1.0"

// COMMONMARK_VERSION is the GFM spec version supported.
const COMMONMARK_VERSION = "0.31.2"
