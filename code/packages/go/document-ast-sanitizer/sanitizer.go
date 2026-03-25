package documentastsanitizer

import (
	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// Sanitize applies a SanitizationPolicy to a DocumentNode and returns a new,
// sanitized DocumentNode tree.
//
// The input document is never mutated — Sanitize is a pure function that
// always returns a freshly constructed tree. Callers can safely pass the same
// document through multiple sanitizers with different policies.
//
// Every node type in the Document AST is handled explicitly. Unknown node
// types returned by future AST versions will be silently dropped (fail-safe),
// with a comment explaining why.
//
// Basic usage:
//
//	// User-generated content — strict policy (comments, forum posts)
//	safe := Sanitize(parse(userMarkdown), STRICT)
//	html := ToHtml(safe)
//
//	// Documentation — pass everything through
//	doc := Sanitize(parse(trustedMarkdown), PASSTHROUGH)
//	html := ToHtml(doc)
func Sanitize(document *documentast.DocumentNode, policy SanitizationPolicy) *documentast.DocumentNode {
	children := sanitizeBlocks(document.Children, policy)
	return &documentast.DocumentNode{Children: children}
}

// ─── Block node sanitization ──────────────────────────────────────────────────

// sanitizeBlocks processes a slice of block nodes, returning only the nodes
// that survive the policy. Nodes that are dropped return nil from sanitizeBlock;
// this function filters those out.
func sanitizeBlocks(nodes []documentast.BlockNode, policy SanitizationPolicy) []documentast.BlockNode {
	result := make([]documentast.BlockNode, 0, len(nodes))
	for _, node := range nodes {
		sanitized := sanitizeBlock(node, policy)
		if sanitized != nil {
			result = append(result, sanitized)
		}
	}
	return result
}

// sanitizeBlock dispatches to the correct handler for each block node type.
// Returns nil when the node should be dropped from the output.
//
// Truth table (from spec TE02):
//
//	DocumentNode       → always recurse into children (handled separately)
//	HeadingNode        → drop if maxHeadingLevel=="drop"; clamp level; recurse
//	ParagraphNode      → recurse; drop if children become empty
//	CodeBlockNode      → drop if dropCodeBlocks; keep as-is otherwise
//	BlockquoteNode     → drop if dropBlockquotes; recurse otherwise
//	ListNode           → recurse into list items
//	ListItemNode       → recurse into children
//	ThematicBreakNode  → keep as-is (leaf)
//	RawBlockNode       → keep/drop per allowRawBlockFormats
//	Unknown            → drop (fail-safe)
func sanitizeBlock(node documentast.BlockNode, policy SanitizationPolicy) documentast.BlockNode {
	switch n := node.(type) {
	case *documentast.HeadingNode:
		return sanitizeHeading(n, policy)
	case *documentast.ParagraphNode:
		return sanitizeParagraph(n, policy)
	case *documentast.CodeBlockNode:
		return sanitizeCodeBlock(n, policy)
	case *documentast.BlockquoteNode:
		return sanitizeBlockquote(n, policy)
	case *documentast.ListNode:
		return sanitizeList(n, policy)
	case *documentast.ListItemNode:
		return sanitizeListItem(n, policy)
	case *documentast.ThematicBreakNode:
		// ThematicBreakNode is a leaf with no content — always pass through.
		return n
	case *documentast.RawBlockNode:
		return sanitizeRawBlock(n, policy)
	default:
		// Unknown node type: drop silently (fail-safe principle).
		// When new node types are added to the AST, this package must be
		// updated to handle them explicitly. Until then, dropping is safer
		// than passing unknown nodes through.
		return nil
	}
}

// sanitizeHeading handles HeadingNode sanitization.
//
// Three cases from the spec truth table:
//  1. MaxHeadingLevel == -1  → drop the entire heading
//  2. level < MinHeadingLevel → promote (raise level number) to MinHeadingLevel
//  3. level > MaxHeadingLevel → demote (lower level number) to MaxHeadingLevel
//  4. otherwise              → keep level, recurse into children
//
// Note: "level" in document-ast terms follows HTML semantics — level 1 is the
// most prominent (h1) and level 6 is the least prominent (h6). Promotion means
// INCREASING the level number (h1 → h2 when minHeadingLevel: 2).
func sanitizeHeading(node *documentast.HeadingNode, policy SanitizationPolicy) documentast.BlockNode {
	// Resolve defaults: 0 means "not set"
	min := policy.MinHeadingLevel
	if min == 0 {
		min = 1
	}
	max := policy.MaxHeadingLevel
	if max == 0 {
		max = 6
	}

	// -1 signals "drop all headings"
	if max == -1 {
		return nil
	}

	// Sanitize children first (the heading might become empty)
	children := sanitizeInlines(node.Children, policy)
	if len(children) == 0 {
		// A heading with no visible content is dropped.
		return nil
	}

	// Clamp the level within [min, max].
	level := node.Level
	if level < min {
		level = min
	}
	if level > max {
		level = max
	}

	return &documentast.HeadingNode{Level: level, Children: children}
}

// sanitizeParagraph recurses into the paragraph's inline children. If all
// children are dropped (e.g. a paragraph containing only a RawInlineNode that
// gets dropped), the paragraph itself is dropped to avoid empty <p></p> tags.
func sanitizeParagraph(node *documentast.ParagraphNode, policy SanitizationPolicy) documentast.BlockNode {
	children := sanitizeInlines(node.Children, policy)
	if len(children) == 0 {
		// Spec: "When all children of a container node are dropped, the parent
		// node is itself dropped from the output."
		return nil
	}
	return &documentast.ParagraphNode{Children: children}
}

// sanitizeCodeBlock keeps or drops the entire CodeBlockNode as a unit. The
// value inside a code block is not further processed for HTML injection because
// renderers always HTML-escape it.
func sanitizeCodeBlock(node *documentast.CodeBlockNode, policy SanitizationPolicy) documentast.BlockNode {
	if policy.DropCodeBlocks {
		return nil
	}
	// Leaf node — return a copy to maintain immutability.
	return &documentast.CodeBlockNode{Language: node.Language, Value: node.Value}
}

// sanitizeBlockquote keeps or drops the BlockquoteNode. Unlike links, children
// are NOT promoted when a blockquote is dropped — the spec says "children are
// not promoted" for blockquotes.
func sanitizeBlockquote(node *documentast.BlockquoteNode, policy SanitizationPolicy) documentast.BlockNode {
	if policy.DropBlockquotes {
		return nil
	}
	children := sanitizeBlocks(node.Children, policy)
	if len(children) == 0 {
		return nil
	}
	return &documentast.BlockquoteNode{Children: children}
}

// sanitizeList recurses into each ListItemNode. A list with no surviving items
// is dropped.
func sanitizeList(node *documentast.ListNode, policy SanitizationPolicy) documentast.BlockNode {
	items := make([]*documentast.ListItemNode, 0, len(node.Children))
	for _, item := range node.Children {
		sanitized := sanitizeListItem(item, policy)
		if sanitized != nil {
			items = append(items, sanitized)
		}
	}
	if len(items) == 0 {
		return nil
	}
	return &documentast.ListNode{
		Ordered:  node.Ordered,
		Start:    node.Start,
		Tight:    node.Tight,
		Children: items,
	}
}

// sanitizeListItem recurses into a ListItemNode's block children. An empty
// list item is kept (HTML allows empty <li> tags and dropping them would
// change list numbering for ordered lists).
func sanitizeListItem(node *documentast.ListItemNode, policy SanitizationPolicy) *documentast.ListItemNode {
	children := sanitizeBlocks(node.Children, policy)
	return &documentast.ListItemNode{Children: children}
}

// sanitizeRawBlock applies the AllowRawBlockFormats policy to a RawBlockNode.
//
// Three modes (from the truth table):
//
//	RawDropAll     → drop node (always)
//	RawPassthrough → keep node as-is (always)
//	RawAllowList   → keep if node.Format is in the allowlist, else drop
func sanitizeRawBlock(node *documentast.RawBlockNode, policy SanitizationPolicy) documentast.BlockNode {
	switch policy.AllowRawBlockFormats.Mode {
	case RawDropAll:
		return nil
	case RawPassthrough:
		return &documentast.RawBlockNode{Format: node.Format, Value: node.Value}
	case RawAllowList:
		for _, allowed := range policy.AllowRawBlockFormats.AllowedFormats {
			if allowed == node.Format {
				return &documentast.RawBlockNode{Format: node.Format, Value: node.Value}
			}
		}
		return nil
	default:
		return nil
	}
}

// ─── Inline node sanitization ─────────────────────────────────────────────────

// sanitizeInlines processes a slice of inline nodes. Some transformations
// (dropLinks child promotion) can expand one node into multiple nodes, so
// the return type is a flat slice.
func sanitizeInlines(nodes []documentast.InlineNode, policy SanitizationPolicy) []documentast.InlineNode {
	result := make([]documentast.InlineNode, 0, len(nodes))
	for _, node := range nodes {
		expanded := sanitizeInline(node, policy)
		result = append(result, expanded...)
	}
	return result
}

// sanitizeInline dispatches to the correct handler for each inline node type.
// Returns a slice because some nodes expand to multiple nodes (link promotion)
// and some collapse to zero nodes (drop).
//
// Truth table (from spec TE02):
//
//	TextNode      → keep as-is
//	EmphasisNode  → recurse; drop if children empty
//	StrongNode    → recurse; drop if children empty
//	CodeSpanNode  → convert to TextNode if transformCodeSpanToText; else keep
//	LinkNode      → promote children if dropLinks; sanitize URL; recurse
//	ImageNode     → drop / convert to text / sanitize URL
//	AutolinkNode  → drop if scheme not allowed; sanitize URL
//	RawInlineNode → keep/drop per allowRawInlineFormats
//	HardBreakNode → keep as-is
//	SoftBreakNode → keep as-is
//	Unknown       → drop (fail-safe)
func sanitizeInline(node documentast.InlineNode, policy SanitizationPolicy) []documentast.InlineNode {
	switch n := node.(type) {
	case *documentast.TextNode:
		// Plain text: always pass through unchanged.
		return []documentast.InlineNode{&documentast.TextNode{Value: n.Value}}

	case *documentast.EmphasisNode:
		return sanitizeEmphasis(n, policy)

	case *documentast.StrongNode:
		return sanitizeStrong(n, policy)

	case *documentast.CodeSpanNode:
		return sanitizeCodeSpan(n, policy)

	case *documentast.LinkNode:
		return sanitizeLink(n, policy)

	case *documentast.ImageNode:
		return sanitizeImage(n, policy)

	case *documentast.AutolinkNode:
		return sanitizeAutolink(n, policy)

	case *documentast.RawInlineNode:
		return sanitizeRawInline(n, policy)

	case *documentast.HardBreakNode:
		return []documentast.InlineNode{&documentast.HardBreakNode{}}

	case *documentast.SoftBreakNode:
		return []documentast.InlineNode{&documentast.SoftBreakNode{}}

	default:
		// Unknown node type: drop silently (fail-safe).
		return nil
	}
}

// sanitizeEmphasis recurses into the emphasis children. Drops the wrapper if
// all children were dropped (avoids empty <em></em>).
func sanitizeEmphasis(node *documentast.EmphasisNode, policy SanitizationPolicy) []documentast.InlineNode {
	children := sanitizeInlines(node.Children, policy)
	if len(children) == 0 {
		return nil
	}
	return []documentast.InlineNode{&documentast.EmphasisNode{Children: children}}
}

// sanitizeStrong recurses into the strong children. Drops the wrapper if
// all children were dropped.
func sanitizeStrong(node *documentast.StrongNode, policy SanitizationPolicy) []documentast.InlineNode {
	children := sanitizeInlines(node.Children, policy)
	if len(children) == 0 {
		return nil
	}
	return []documentast.InlineNode{&documentast.StrongNode{Children: children}}
}

// sanitizeCodeSpan either keeps the CodeSpanNode as-is or converts it to a
// TextNode. Code span values are raw (not decoded), but text nodes are decoded
// display text — the caller requested this transformation, so we use the value
// as-is as the best available approximation.
func sanitizeCodeSpan(node *documentast.CodeSpanNode, policy SanitizationPolicy) []documentast.InlineNode {
	if policy.TransformCodeSpanToText {
		return []documentast.InlineNode{&documentast.TextNode{Value: node.Value}}
	}
	return []documentast.InlineNode{&documentast.CodeSpanNode{Value: node.Value}}
}

// sanitizeLink handles the three possible link outcomes:
//
//  1. dropLinks == true  → promote all children to the parent (link wrapper removed)
//  2. URL scheme not allowed → keep the link node, but replace destination with ""
//  3. otherwise → sanitize URL, recurse into children
//
// "Promoting" children means returning them directly instead of wrapping them
// in a LinkNode. The calling sanitizeInlines function flattens the result into
// its own accumulator, so the children appear in-line at the link's position.
func sanitizeLink(node *documentast.LinkNode, policy SanitizationPolicy) []documentast.InlineNode {
	if policy.DropLinks {
		// Promote children: the link wrapper is removed, but the text stays.
		//
		// Before:  LinkNode { "click here" } → <a href="...">click here</a>
		// After:   TextNode { "click here" } → click here
		return sanitizeInlines(node.Children, policy)
	}

	dest := node.Destination
	if !IsSchemeAllowed(dest, policy) {
		dest = ""
	}

	children := sanitizeInlines(node.Children, policy)
	if len(children) == 0 {
		// A link with no visible text is still kept (inert link with empty href).
		// But if the destination is also empty, drop the node.
		if dest == "" {
			return nil
		}
	}
	return []documentast.InlineNode{&documentast.LinkNode{
		Destination: dest,
		Title:       node.Title,
		HasTitle:    node.HasTitle,
		Children:    children,
	}}
}

// sanitizeImage handles the three possible image outcomes:
//
//  1. dropImages == true             → drop entirely (no text fallback)
//  2. transformImageToText == true   → replace with TextNode containing alt text
//  3. URL scheme not allowed         → keep node, set destination to ""
//  4. otherwise                      → sanitize URL, keep as-is
func sanitizeImage(node *documentast.ImageNode, policy SanitizationPolicy) []documentast.InlineNode {
	if policy.DropImages {
		return nil
	}

	if policy.TransformImageToText {
		return []documentast.InlineNode{&documentast.TextNode{Value: node.Alt}}
	}

	dest := node.Destination
	if !IsSchemeAllowed(dest, policy) {
		dest = ""
	}

	return []documentast.InlineNode{&documentast.ImageNode{
		Destination: dest,
		Title:       node.Title,
		HasTitle:    node.HasTitle,
		Alt:         node.Alt,
	}}
}

// sanitizeAutolink checks the URL scheme and either keeps or drops the node.
//
// Unlike LinkNode, AutolinkNode has no children to promote — the link text IS
// the URL, so there is nothing meaningful to preserve. When the scheme is not
// allowed, the entire node is dropped.
func sanitizeAutolink(node *documentast.AutolinkNode, policy SanitizationPolicy) []documentast.InlineNode {
	// Email autolinks always use mailto: scheme. Check that explicitly.
	dest := node.Destination
	if node.IsEmail {
		// The destination is "user@example.com" — the mailto: prefix is
		// implied. We check "mailto" against the allowlist.
		if !policy.AllowAllSchemes {
			mailtoAllowed := false
			for _, s := range policy.AllowedUrlSchemes {
				if s == "mailto" {
					mailtoAllowed = true
					break
				}
			}
			if !mailtoAllowed {
				return nil
			}
		}
	} else {
		if !IsSchemeAllowed(dest, policy) {
			return nil
		}
	}

	return []documentast.InlineNode{&documentast.AutolinkNode{
		Destination: dest,
		IsEmail:     node.IsEmail,
	}}
}

// sanitizeRawInline applies the AllowRawInlineFormats policy to a RawInlineNode.
// Mirrors the block-level sanitizeRawBlock logic exactly.
func sanitizeRawInline(node *documentast.RawInlineNode, policy SanitizationPolicy) []documentast.InlineNode {
	switch policy.AllowRawInlineFormats.Mode {
	case RawDropAll:
		return nil
	case RawPassthrough:
		return []documentast.InlineNode{&documentast.RawInlineNode{Format: node.Format, Value: node.Value}}
	case RawAllowList:
		for _, allowed := range policy.AllowRawInlineFormats.AllowedFormats {
			if allowed == node.Format {
				return []documentast.InlineNode{&documentast.RawInlineNode{Format: node.Format, Value: node.Value}}
			}
		}
		return nil
	default:
		return nil
	}
}
