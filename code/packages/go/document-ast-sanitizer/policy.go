// Package documentastsanitizer performs a policy-driven transformation of a
// Document AST, pruning or neutralising nodes that violate a caller-defined
// SanitizationPolicy.
//
// # Why a separate package?
//
// Sanitization is a *separate pipeline concern* from parsing and rendering.
// The parser's job is to faithfully decode source text into an AST. The
// renderer's job is to faithfully encode an AST as output HTML. Neither
// component should decide what content is "safe" — that decision belongs to
// the caller, who knows the security context (trusted editor preview vs.
// public comment thread).
//
// By extracting sanitization into its own package we gain:
//
//  1. A composable, independently-testable unit.
//  2. A structured policy API (not a coarse boolean).
//  3. A single canonical implementation reused across every back-end.
//
// Spec: TE02 — Document Sanitization
package documentastsanitizer

// ─── Policy Type ──────────────────────────────────────────────────────────────

// RawFormatPolicy controls which RawBlockNode or RawInlineNode formats are
// allowed through the sanitizer. Three modes are supported:
//
//   - RawDropAll       — drop every raw node regardless of format (safest)
//   - RawPassthrough   — keep every raw node regardless of format
//   - RawAllowList     — keep only nodes whose Format is in AllowedFormats
type RawFormatMode int

const (
	// RawDropAll drops every raw node regardless of format.
	// Recommended for user-generated content.
	RawDropAll RawFormatMode = iota

	// RawPassthrough keeps every raw node regardless of format.
	// Use only for trusted content.
	RawPassthrough

	// RawAllowList keeps only nodes whose Format is in AllowedFormats.
	RawAllowList
)

// RawFormatPolicy bundles a RawFormatMode with its optional allowlist.
//
// When Mode == RawAllowList, AllowedFormats must be populated.
// When Mode != RawAllowList, AllowedFormats is ignored.
//
//	// Keep HTML raw blocks but drop LaTeX and all others:
//	RawFormatPolicy{Mode: RawAllowList, AllowedFormats: []string{"html"}}
type RawFormatPolicy struct {
	Mode           RawFormatMode
	AllowedFormats []string
}

// SanitizationPolicy controls what the AST sanitizer keeps, transforms, or drops.
//
// IMPORTANT — zero-value behaviour: a zero-value SanitizationPolicy is NOT
// equivalent to PASSTHROUGH. Because RawDropAll == 0 (iota), a zero-value
// policy drops all RawBlockNode and RawInlineNode values. Always use one of
// the named presets (STRICT, RELAXED, PASSTHROUGH) rather than constructing
// a SanitizationPolicy literal from scratch unless you understand all defaults.
//
// Use the named presets as starting points and override individual fields for
// fine-grained control:
//
//	// Reserve h1 for the page title; user content starts at h2
//	policy := RELAXED
//	policy.MinHeadingLevel = 2
type SanitizationPolicy struct {

	// ─── Raw node handling ────────────────────────────────────────────────

	// AllowRawBlockFormats controls which RawBlockNode formats pass through.
	// Zero value (RawPassthrough) keeps everything.
	AllowRawBlockFormats RawFormatPolicy

	// AllowRawInlineFormats controls which RawInlineNode formats pass through.
	// Zero value (RawPassthrough) keeps everything.
	AllowRawInlineFormats RawFormatPolicy

	// ─── URL scheme policy ────────────────────────────────────────────────

	// AllowedUrlSchemes is an allowlist of URL schemes permitted in
	// LinkNode.Destination, ImageNode.Destination, and AutolinkNode.Destination.
	//
	// URLs whose scheme is not in this list have their destination replaced
	// with "" (making the link inert). Relative URLs always pass through.
	//
	// A nil slice means "allow any scheme" (PASSTHROUGH behaviour).
	// Default (nil): all schemes allowed.
	AllowedUrlSchemes []string

	// AllowAllSchemes bypasses scheme checking when true. This is the
	// PASSTHROUGH behaviour. Set by the PASSTHROUGH preset.
	AllowAllSchemes bool

	// ─── Node type policy ─────────────────────────────────────────────────

	// DropLinks drops all LinkNode instances. The link's inline children are
	// promoted to the parent container, preserving the text.
	// Default: false.
	DropLinks bool

	// DropImages drops all ImageNode instances entirely (no text fallback).
	// Takes precedence over TransformImageToText.
	// Default: false.
	DropImages bool

	// TransformImageToText replaces each ImageNode with a TextNode containing
	// the image's alt text. Ignored when DropImages is true.
	// Default: false.
	TransformImageToText bool

	// MaxHeadingLevel is the deepest heading level allowed (1–6). Headings
	// with a level greater than MaxHeadingLevel are clamped down to
	// MaxHeadingLevel.
	//
	// Set to -1 to drop ALL heading nodes entirely (equivalent to the spec's
	// "drop" option).
	//
	// Default (0): treated as 6 (no clamping).
	MaxHeadingLevel int

	// MinHeadingLevel is the shallowest heading level allowed (1–6). Headings
	// with a level less than MinHeadingLevel are promoted (level raised) to
	// MinHeadingLevel.
	//
	// Default (0): treated as 1 (no promotion).
	MinHeadingLevel int

	// DropBlockquotes drops all BlockquoteNode instances. Children are NOT
	// promoted.
	// Default: false.
	DropBlockquotes bool

	// DropCodeBlocks drops all CodeBlockNode instances.
	// Default: false.
	DropCodeBlocks bool

	// TransformCodeSpanToText converts CodeSpanNode instances to TextNode
	// instances containing the raw code value.
	// Default: false.
	TransformCodeSpanToText bool
}

// ─── Named Presets ────────────────────────────────────────────────────────────
//
// Three presets cover the most common usage patterns. Copy and modify a preset
// for custom policies:
//
//	myPolicy := STRICT
//	myPolicy.AllowedUrlSchemes = []string{"http", "https", "ftp"}

// STRICT is the recommended policy for user-generated content: comments,
// forum posts, chat messages.
//
// What STRICT does:
//   - Drops all raw HTML/format passthrough nodes (no <script> injection)
//   - Allows only http, https, and mailto URL schemes
//   - Converts images to alt text instead of rendering <img> tags
//   - Clamps headings to h2–h6 (h1 is reserved for the page title)
//   - Keeps links, blockquotes, and code blocks
//   - Does NOT convert code spans to plain text
var STRICT = SanitizationPolicy{
	AllowRawBlockFormats:    RawFormatPolicy{Mode: RawDropAll},
	AllowRawInlineFormats:   RawFormatPolicy{Mode: RawDropAll},
	AllowedUrlSchemes:       []string{"http", "https", "mailto"},
	DropImages:              false,
	TransformImageToText:    true,
	MinHeadingLevel:         2,
	MaxHeadingLevel:         6,
	DropLinks:               false,
	DropBlockquotes:         false,
	DropCodeBlocks:          false,
	TransformCodeSpanToText: false,
}

// RELAXED is the recommended policy for semi-trusted content: authenticated
// users, internal wikis, editors with a known identity.
//
// What RELAXED does:
//   - Allows HTML raw blocks (but drops LaTeX and other formats)
//   - Allows http, https, mailto, and ftp URL schemes
//   - Images pass through unchanged
//   - Headings unrestricted (h1–h6 all allowed)
var RELAXED = SanitizationPolicy{
	AllowRawBlockFormats:    RawFormatPolicy{Mode: RawAllowList, AllowedFormats: []string{"html"}},
	AllowRawInlineFormats:   RawFormatPolicy{Mode: RawAllowList, AllowedFormats: []string{"html"}},
	AllowedUrlSchemes:       []string{"http", "https", "mailto", "ftp"},
	DropImages:              false,
	TransformImageToText:    false,
	MinHeadingLevel:         1,
	MaxHeadingLevel:         6,
	DropLinks:               false,
	DropBlockquotes:         false,
	DropCodeBlocks:          false,
	TransformCodeSpanToText: false,
}

// PASSTHROUGH performs no sanitization at all. Every node passes through
// unchanged. Equivalent to not calling Sanitize() at all.
//
// Use this for fully trusted content: documentation, static sites, your own
// markdown files.
var PASSTHROUGH = SanitizationPolicy{
	AllowRawBlockFormats:    RawFormatPolicy{Mode: RawPassthrough},
	AllowRawInlineFormats:   RawFormatPolicy{Mode: RawPassthrough},
	AllowAllSchemes:         true,
	DropImages:              false,
	TransformImageToText:    false,
	MinHeadingLevel:         1,
	MaxHeadingLevel:         6,
	DropLinks:               false,
	DropBlockquotes:         false,
	DropCodeBlocks:          false,
	TransformCodeSpanToText: false,
}
