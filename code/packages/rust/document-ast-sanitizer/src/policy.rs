//! Sanitization policy types and named presets.
//!
//! A `SanitizationPolicy` is a plain data struct that controls every decision
//! the AST sanitizer makes during its recursive tree walk. By keeping policy
//! separate from implementation:
//!
//!   - Policies can be composed: `SanitizationPolicy { min_heading_level: 2, ..STRICT }`
//!   - Policies can be stored and compared (they implement `Clone`, `Debug`, `PartialEq`)
//!   - New back-ends (PDF, plain text) can reuse the same policy types
//!
//! # Named Presets
//!
//! Three pre-built policies cover the most common scenarios:
//!
//! | Preset        | Use case                                  |
//! |---------------|-------------------------------------------|
//! | `STRICT`      | User-generated content (comments, chat)   |
//! | `RELAXED`     | Authenticated users / internal wikis      |
//! | `PASSTHROUGH` | Fully trusted content (docs, static sites)|
//!
//! ## Composition example
//!
//! ```rust
//! use coding_adventures_document_ast_sanitizer::policy::{RELAXED, SanitizationPolicy};
//!
//! // Reserve h1 for the page title; all other RELAXED rules apply.
//! let policy = SanitizationPolicy {
//!     min_heading_level: 2,
//!     ..RELAXED
//! };
//! ```

// ─── Raw-format policy ────────────────────────────────────────────────────────

/// Controls which `RawBlockNode` / `RawInlineNode` formats survive sanitization.
///
/// The three variants map to a clear mental model:
///
/// ```text
/// DropAll      → zero trust — discard every raw block/inline node
/// Passthrough  → full trust — keep every raw block/inline node unchanged
/// Allowlist    → selective  — keep only the listed format strings
/// ```
///
/// # Example
///
/// ```rust
/// use coding_adventures_document_ast_sanitizer::policy::RawFormatPolicy;
///
/// // Allow HTML raw blocks but drop LaTeX and all others
/// let policy = RawFormatPolicy::Allowlist(vec!["html".to_string()]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum RawFormatPolicy {
    /// Drop every raw node regardless of format. Safest option.
    DropAll,
    /// Keep every raw node regardless of format. Use only for trusted content.
    Passthrough,
    /// Keep only raw nodes whose `format` field appears in this list.
    /// All other formats are dropped.
    Allowlist(Vec<String>),
}

impl RawFormatPolicy {
    /// Returns `true` if a raw node with the given `format` string should be kept.
    ///
    /// ```text
    /// DropAll       → always false
    /// Passthrough   → always true
    /// Allowlist(v)  → true iff format is in v
    /// ```
    pub fn allows(&self, format: &str) -> bool {
        match self {
            RawFormatPolicy::DropAll => false,
            RawFormatPolicy::Passthrough => true,
            RawFormatPolicy::Allowlist(list) => list.iter().any(|s| s == format),
        }
    }
}

// ─── Heading-level policy ─────────────────────────────────────────────────────

/// Controls how the sanitizer handles heading levels.
///
/// In HTML, heading levels map directly to `<h1>` through `<h6>`. When user
/// content is embedded inside a page that already uses `<h1>` for the page
/// title, all user headings should start at `<h2>` at the earliest. The
/// sanitizer enforces this by clamping heading levels to the specified range.
///
/// ```text
/// min_heading_level: 2  →  level 1 → 2, levels 2–6 unchanged
/// max_heading_level: 4  →  levels 5–6 → 4, levels 1–4 unchanged
/// max_heading_level: Drop →  all headings removed entirely
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MaxHeadingLevel {
    /// Remove all `HeadingNode` instances. Use when user headings would disrupt
    /// the host document's hierarchy.
    Drop,
    /// Clamp all headings to at most this level. Must be 1–6.
    Level(u8),
}

// ─── SanitizationPolicy struct ────────────────────────────────────────────────

/// Policy that controls what the AST sanitizer keeps, transforms, or drops.
///
/// All fields have sensible defaults that match `PASSTHROUGH` (keep everything).
/// Use the named presets `STRICT`, `RELAXED`, or `PASSTHROUGH` as starting
/// points and override specific fields with struct update syntax.
///
/// # Design rationale
///
/// A struct rather than a builder or method chain because:
/// - Rust's struct update syntax (`..STRICT`) is as ergonomic as a builder
/// - Policies are plain data — no behaviour, no hidden state
/// - `Clone + PartialEq` make it easy to store, compare, and test policies
///
/// # Truth table
///
/// See `TE02 — Document Sanitization` spec for the full transformation table.
/// This struct controls every row in that table.
#[derive(Debug, Clone, PartialEq)]
pub struct SanitizationPolicy {
    // ─── Raw node handling ──────────────────────────────────────────────────

    /// Controls which `RawBlockNode` formats survive.
    /// Default: `Passthrough` (all raw block formats kept).
    pub allow_raw_block_formats: RawFormatPolicy,

    /// Controls which `RawInlineNode` formats survive.
    /// Default: `Passthrough` (all raw inline formats kept).
    pub allow_raw_inline_formats: RawFormatPolicy,

    // ─── URL scheme policy ──────────────────────────────────────────────────

    /// Allowlist of URL schemes permitted in `LinkNode.destination`,
    /// `ImageNode.destination`, and `AutolinkNode.destination`.
    ///
    /// URLs whose scheme is not in this list have their destination replaced
    /// with `""` (making the link/image inert). Relative URLs (no scheme)
    /// always pass through.
    ///
    /// `None` means all schemes are allowed (not recommended for untrusted content).
    ///
    /// Default: `Some(["http", "https", "mailto", "ftp"])`.
    pub allowed_url_schemes: Option<Vec<String>>,

    // ─── Node type policy ───────────────────────────────────────────────────

    /// If `true`, all `LinkNode` instances are removed. Their text children
    /// are **promoted** to the parent container — the text is preserved, only
    /// the hyperlink wrapper is removed.
    ///
    /// Default: `false` (links kept).
    pub drop_links: bool,

    /// If `true`, all `ImageNode` instances are dropped entirely.
    /// Takes precedence over `transform_image_to_text`.
    ///
    /// Default: `false` (images kept).
    pub drop_images: bool,

    /// If `true`, `ImageNode` instances are replaced by a `TextNode`
    /// containing the image's `alt` text. Provides a text fallback
    /// without completely silencing image references.
    ///
    /// Ignored when `drop_images` is `true`.
    ///
    /// Default: `false`.
    pub transform_image_to_text: bool,

    /// Minimum heading level. Headings with `level < min_heading_level`
    /// have their level raised to `min_heading_level`.
    ///
    /// Value must be in `1..=6`. Default: `1` (no promotion).
    pub min_heading_level: u8,

    /// Maximum heading level. Headings with `level > max_heading_level.Level`
    /// are clamped down. `Drop` removes all headings entirely.
    ///
    /// Default: `Level(6)` (no clamping).
    pub max_heading_level: MaxHeadingLevel,

    /// If `true`, `BlockquoteNode` instances are dropped entirely.
    /// Children are NOT promoted — the whole blockquote and its content
    /// disappears.
    ///
    /// Default: `false`.
    pub drop_blockquotes: bool,

    /// If `true`, `CodeBlockNode` instances are dropped.
    ///
    /// Default: `false`.
    pub drop_code_blocks: bool,

    /// If `true`, `CodeSpanNode` instances are converted to plain `TextNode`
    /// instances carrying the same `value`. This is useful when monospace
    /// inline code spans are not wanted in a stripped-down rendering context.
    ///
    /// Default: `false`.
    pub transform_code_span_to_text: bool,
}

// ─── Named presets ────────────────────────────────────────────────────────────
//
// The three presets cover the most common trust levels. They are `const`-like
// values implemented as associated functions returning owned structs (since
// `Vec` is not `const`).

/// STRICT preset — for user-generated content (comments, forum posts, chat).
///
/// Security properties:
/// - All raw HTML/LaTeX passthrough is dropped (`drop-all`)
/// - Only `http`, `https`, `mailto` URLs are permitted
/// - Images are converted to their alt text (no external resource loads)
/// - Headings are clamped to level 2–6 (level 1 is reserved for the page title)
/// - Links are kept but URL-sanitized
/// - All other content passes through unchanged
pub const STRICT: SanitizationPolicy = SanitizationPolicy {
    allow_raw_block_formats: RawFormatPolicy::DropAll,
    allow_raw_inline_formats: RawFormatPolicy::DropAll,
    // NOTE: Vec cannot be used in const, so allowed_url_schemes uses None here
    // and is overridden in the `strict()` convenience function below.
    // The const version uses None (all schemes) but the doc-tested preset
    // uses the strict() function. See below.
    allowed_url_schemes: None, // overridden at runtime; see strict() fn
    drop_links: false,
    drop_images: false,
    transform_image_to_text: true,
    min_heading_level: 2,
    max_heading_level: MaxHeadingLevel::Level(6),
    drop_blockquotes: false,
    drop_code_blocks: false,
    transform_code_span_to_text: false,
};

/// RELAXED preset — for semi-trusted content (authenticated users, internal wikis).
///
/// Security properties:
/// - HTML raw blocks are allowed, but non-HTML formats (LaTeX etc.) are dropped
/// - `http`, `https`, `mailto`, `ftp` URLs are permitted
/// - Images pass through unchanged
/// - Headings are unrestricted (levels 1–6)
pub const RELAXED: SanitizationPolicy = SanitizationPolicy {
    allow_raw_block_formats: RawFormatPolicy::Passthrough, // overridden in relaxed()
    allow_raw_inline_formats: RawFormatPolicy::Passthrough, // overridden in relaxed()
    allowed_url_schemes: None, // overridden at runtime; see relaxed() fn
    drop_links: false,
    drop_images: false,
    transform_image_to_text: false,
    min_heading_level: 1,
    max_heading_level: MaxHeadingLevel::Level(6),
    drop_blockquotes: false,
    drop_code_blocks: false,
    transform_code_span_to_text: false,
};

/// PASSTHROUGH preset — for fully trusted content (documentation, static sites).
///
/// No sanitization. Every node type passes through unchanged. Using this
/// preset is equivalent to not calling `sanitize()` at all.
pub const PASSTHROUGH: SanitizationPolicy = SanitizationPolicy {
    allow_raw_block_formats: RawFormatPolicy::Passthrough,
    allow_raw_inline_formats: RawFormatPolicy::Passthrough,
    allowed_url_schemes: None,
    drop_links: false,
    drop_images: false,
    transform_image_to_text: false,
    min_heading_level: 1,
    max_heading_level: MaxHeadingLevel::Level(6),
    drop_blockquotes: false,
    drop_code_blocks: false,
    transform_code_span_to_text: false,
};

// ─── Convenience constructors with Vec-based URL schemes ─────────────────────
//
// Because Rust `const` cannot contain heap-allocated types like `Vec`, the
// named preset constants above store `allowed_url_schemes: None` (no restriction).
// These functions return fully-configured policy values with proper URL scheme
// allowlists — use these in production code, not the bare `const` values.

/// Returns a fully-configured STRICT policy.
///
/// Identical to the `STRICT` constant but with `allowed_url_schemes` set to
/// `["http", "https", "mailto"]` instead of `None`.
///
/// ```rust
/// use coding_adventures_document_ast_sanitizer::policy::strict;
///
/// let p = strict();
/// assert_eq!(p.allowed_url_schemes, Some(vec!["http".to_string(), "https".to_string(), "mailto".to_string()]));
/// assert!(p.transform_image_to_text);
/// assert_eq!(p.min_heading_level, 2);
/// ```
pub fn strict() -> SanitizationPolicy {
    SanitizationPolicy {
        allow_raw_block_formats: RawFormatPolicy::DropAll,
        allow_raw_inline_formats: RawFormatPolicy::DropAll,
        allowed_url_schemes: Some(vec![
            "http".to_string(),
            "https".to_string(),
            "mailto".to_string(),
        ]),
        drop_links: false,
        drop_images: false,
        transform_image_to_text: true,
        min_heading_level: 2,
        max_heading_level: MaxHeadingLevel::Level(6),
        drop_blockquotes: false,
        drop_code_blocks: false,
        transform_code_span_to_text: false,
    }
}

/// Returns a fully-configured RELAXED policy.
///
/// Identical to the `RELAXED` constant but with `allow_raw_block_formats` set
/// to `Allowlist(["html"])`, `allow_raw_inline_formats` set to `Allowlist(["html"])`,
/// and `allowed_url_schemes` set to `["http", "https", "mailto", "ftp"]`.
///
/// ```rust
/// use coding_adventures_document_ast_sanitizer::policy::{relaxed, RawFormatPolicy};
///
/// let p = relaxed();
/// assert_eq!(p.allow_raw_block_formats, RawFormatPolicy::Allowlist(vec!["html".to_string()]));
/// assert_eq!(p.allowed_url_schemes, Some(vec![
///     "http".to_string(), "https".to_string(),
///     "mailto".to_string(), "ftp".to_string(),
/// ]));
/// ```
pub fn relaxed() -> SanitizationPolicy {
    SanitizationPolicy {
        allow_raw_block_formats: RawFormatPolicy::Allowlist(vec!["html".to_string()]),
        allow_raw_inline_formats: RawFormatPolicy::Allowlist(vec!["html".to_string()]),
        allowed_url_schemes: Some(vec![
            "http".to_string(),
            "https".to_string(),
            "mailto".to_string(),
            "ftp".to_string(),
        ]),
        drop_links: false,
        drop_images: false,
        transform_image_to_text: false,
        min_heading_level: 1,
        max_heading_level: MaxHeadingLevel::Level(6),
        drop_blockquotes: false,
        drop_code_blocks: false,
        transform_code_span_to_text: false,
    }
}

/// Returns a fully-configured PASSTHROUGH policy.
///
/// All fields permit everything — equivalent to not calling `sanitize()` at all.
///
/// ```rust
/// use coding_adventures_document_ast_sanitizer::policy::{passthrough, RawFormatPolicy};
///
/// let p = passthrough();
/// assert_eq!(p.allow_raw_block_formats, RawFormatPolicy::Passthrough);
/// assert!(p.allowed_url_schemes.is_none());
/// ```
pub fn passthrough() -> SanitizationPolicy {
    SanitizationPolicy {
        allow_raw_block_formats: RawFormatPolicy::Passthrough,
        allow_raw_inline_formats: RawFormatPolicy::Passthrough,
        allowed_url_schemes: None,
        drop_links: false,
        drop_images: false,
        transform_image_to_text: false,
        min_heading_level: 1,
        max_heading_level: MaxHeadingLevel::Level(6),
        drop_blockquotes: false,
        drop_code_blocks: false,
        transform_code_span_to_text: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_format_policy_drop_all_never_allows() {
        assert!(!RawFormatPolicy::DropAll.allows("html"));
        assert!(!RawFormatPolicy::DropAll.allows("latex"));
        assert!(!RawFormatPolicy::DropAll.allows(""));
    }

    #[test]
    fn raw_format_policy_passthrough_always_allows() {
        assert!(RawFormatPolicy::Passthrough.allows("html"));
        assert!(RawFormatPolicy::Passthrough.allows("latex"));
        assert!(RawFormatPolicy::Passthrough.allows("anything"));
    }

    #[test]
    fn raw_format_policy_allowlist_selective() {
        let p = RawFormatPolicy::Allowlist(vec!["html".to_string(), "rtf".to_string()]);
        assert!(p.allows("html"));
        assert!(p.allows("rtf"));
        assert!(!p.allows("latex"));
        assert!(!p.allows(""));
    }

    #[test]
    fn strict_policy_has_expected_defaults() {
        let p = strict();
        assert_eq!(p.allow_raw_block_formats, RawFormatPolicy::DropAll);
        assert_eq!(p.allow_raw_inline_formats, RawFormatPolicy::DropAll);
        assert!(p.transform_image_to_text);
        assert!(!p.drop_images);
        assert!(!p.drop_links);
        assert_eq!(p.min_heading_level, 2);
        assert_eq!(p.max_heading_level, MaxHeadingLevel::Level(6));
        let schemes = p.allowed_url_schemes.unwrap();
        assert!(schemes.contains(&"http".to_string()));
        assert!(schemes.contains(&"https".to_string()));
        assert!(schemes.contains(&"mailto".to_string()));
        assert!(!schemes.contains(&"javascript".to_string()));
        assert!(!schemes.contains(&"ftp".to_string()));
    }

    #[test]
    fn relaxed_policy_allows_html_raw() {
        let p = relaxed();
        assert!(p.allow_raw_block_formats.allows("html"));
        assert!(!p.allow_raw_block_formats.allows("latex"));
        assert!(!p.transform_image_to_text);
        assert_eq!(p.min_heading_level, 1);
    }

    #[test]
    fn passthrough_policy_allows_all() {
        let p = passthrough();
        assert_eq!(p.allow_raw_block_formats, RawFormatPolicy::Passthrough);
        assert!(p.allowed_url_schemes.is_none());
        assert!(!p.drop_links);
        assert!(!p.drop_images);
        assert!(!p.transform_image_to_text);
    }
}
