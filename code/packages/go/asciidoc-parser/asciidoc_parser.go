// Package asciidocparser converts AsciiDoc text to a Document AST.
//
// AsciiDoc is a lightweight markup language designed for technical writing.
// This parser converts AsciiDoc source text into a DocumentNode — the
// format-agnostic intermediate representation defined in the document-ast
// package. The result can then be fed to any back-end renderer (HTML, PDF,
// plain text, etc.).
//
// # Two-Phase Parsing
//
//	Phase 1 — Block structure (block_parser.go):
//	  Input text → lines → block tree with raw inline content strings.
//	  Headings, paragraphs, lists, code blocks, blockquotes, and thematic
//	  breaks are identified and structured into a tree.
//
//	Phase 2 — Inline content (inline_parser.go):
//	  Each block's raw content string → inline nodes.
//	  Bold, italic, links, images, and code spans are parsed.
//
// # AsciiDoc vs Markdown Differences
//
// Key differences from Markdown:
//   - Headings use = signs: = Title (level 1), == Section (level 2), etc.
//   - *bold* means STRONG (not emphasis!) — this is a frequent source of confusion
//   - _italic_ means emphasis
//   - ** and __ are unconstrained (work mid-word)
//   - Code blocks delimited by ---- (not backticks), preceded by [source,lang]
//   - Thematic break: ''' (three single-quotes)
//   - Links: link:url[text] or <<anchor,text>>
//   - Images: image:url[alt]
//
// # Quick Start
//
//	import parser "github.com/adhithyan15/coding-adventures/code/packages/go/asciidoc-parser"
//
//	doc := parser.Parse("= Hello\n\nWorld *with* bold.\n")
//	doc.Children[0].NodeType()  // "heading"
//	doc.Children[1].NodeType()  // "paragraph"
//
// Spec: TE03 — AsciiDoc Parser
package asciidocparser

import (
	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// Parse converts AsciiDoc source text into a DocumentNode AST.
//
// The result conforms to the Document AST spec (TE00) — a format-agnostic IR
// with all inline markup parsed.
//
//	doc := Parse("== Section\n\n- item 1\n- item 2\n")
//	doc.Children[0].NodeType()  // "heading"
//	doc.Children[1].NodeType()  // "list"
func Parse(text string) *documentast.DocumentNode {
	blocks := parseBlocks(text)
	return &documentast.DocumentNode{Children: blocks}
}

// VERSION is the version of this asciidoc-parser package.
const VERSION = "0.1.0"
