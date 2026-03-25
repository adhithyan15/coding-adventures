// Package commonmarkparser implements a CommonMark 0.31.2 compliant Markdown
// parser that produces a Document AST.
//
// The parser converts Markdown source text into a DocumentNode — the
// format-agnostic IR defined in the document-ast package. The result is
// ready for any back-end renderer (HTML, PDF, plain text, …).
//
// # Two-Phase Parsing
//
// CommonMark parsing is inherently two-phase because block structure must
// be determined before inline content can be parsed:
//
//	Phase 1 — Block structure (block_parser.go):
//	  Input text → lines → block tree with raw inline content strings.
//	  Headings, paragraphs, lists, code blocks, blockquotes, and HTML blocks
//	  are identified and structured into a tree.
//
//	Phase 2 — Inline content (inline_parser.go):
//	  Each block's raw content string → inline nodes.
//	  Emphasis, links, images, code spans, autolinks, etc. are parsed.
//
// The phases cannot be merged because block structure determines where inline
// content lives. A `*` that starts a list item is structural; a `*` inside
// a paragraph may be emphasis.
//
// # Quick Start
//
//	import parser "github.com/adhithyan15/coding-adventures/code/packages/go/commonmark-parser"
//
//	doc := parser.Parse("# Hello\n\nWorld *with* emphasis.\n")
//	doc.Type   // "document"
//	doc.Children[0].NodeType()  // "heading"
//	doc.Children[1].NodeType()  // "paragraph"
//
// # Spec Compliance
//
// This parser targets CommonMark 0.31.2 (https://spec.commonmark.org/0.31.2/).
// All 652 examples in the CommonMark specification test suite pass.
//
// Spec: TE01 — CommonMark Parser (Go port of TypeScript TE01 implementation)
package commonmarkparser

import (
	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// Parse parses a CommonMark Markdown string into a DocumentNode AST.
//
// The result conforms to the Document AST spec (TE00) — a format-agnostic IR
// with all link references resolved and all inline markup parsed.
//
//	doc := Parse("## Heading\n\n- item 1\n- item 2\n")
//	doc.Children[0].NodeType()  // "heading"
//	doc.Children[1].NodeType()  // "list"
func Parse(markdown string) *documentast.DocumentNode {
	// Phase 1: Block parsing — builds the structural skeleton
	result := parseBlocks(markdown)

	// Convert mutable intermediate tree to final AST with raw inline content IDs
	converted := convertToAST(result.document, result.linkRefs)

	// Phase 2: Inline parsing — fills in emphasis, links, code spans, etc.
	resolveInlineContent(converted.document, converted.rawInlineContent, result.linkRefs)

	// Replace internal heading/paragraph nodes (with rawIDs) with proper types.
	// After resolveInlineContent, the Children slices are populated.
	return flattenAST(converted.document)
}

// flattenAST replaces headingNodeWithID and paragraphNodeWithID with their
// concrete DocumentAST types. This is safe to call after resolveInlineContent
// has populated the Children slices.
func flattenAST(doc *documentast.DocumentNode) *documentast.DocumentNode {
	result := &documentast.DocumentNode{}
	for _, child := range doc.Children {
		if n := flattenBlock(child); n != nil {
			result.Children = append(result.Children, n)
		}
	}
	return result
}

func flattenBlock(block documentast.BlockNode) documentast.BlockNode {
	switch b := block.(type) {
	case *headingNodeWithID:
		return &documentast.HeadingNode{
			Level:    b.HeadingNode.Level,
			Children: b.HeadingNode.Children,
		}
	case *paragraphNodeWithID:
		return &documentast.ParagraphNode{
			Children: b.ParagraphNode.Children,
		}
	case *documentast.DocumentNode:
		result := &documentast.DocumentNode{}
		for _, child := range b.Children {
			if n := flattenBlock(child); n != nil {
				result.Children = append(result.Children, n)
			}
		}
		return result
	case *documentast.BlockquoteNode:
		result := &documentast.BlockquoteNode{}
		for _, child := range b.Children {
			if n := flattenBlock(child); n != nil {
				result.Children = append(result.Children, n)
			}
		}
		return result
	case *documentast.ListNode:
		result := &documentast.ListNode{
			Ordered: b.Ordered,
			Start:   b.Start,
			Tight:   b.Tight,
		}
		for _, item := range b.Children {
			if n := flattenBlock(item); n != nil {
				result.Children = append(result.Children, n.(*documentast.ListItemNode))
			}
		}
		return result
	case *documentast.ListItemNode:
		result := &documentast.ListItemNode{}
		for _, child := range b.Children {
			if n := flattenBlock(child); n != nil {
				result.Children = append(result.Children, n)
			}
		}
		return result
	default:
		return block
	}
}

// VERSION is the version of this commonmark-parser package.
const VERSION = "0.1.0"

// COMMONMARK_VERSION is the CommonMark spec version this parser targets.
const COMMONMARK_VERSION = "0.31.2"
