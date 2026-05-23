//! JSDoc comment extractor.
//!
//! Scans **raw JavaScript source** for `/** ... */` block comments,
//! strips the enclosing markers plus per-line `* ` continuation prefixes,
//! and returns each comment as a [`BlockComment`] ready to feed into the
//! `jsdoc-lexer` / `jsdoc-parser` pipeline.
//!
//! # Scope (v1)
//!
//! - **Raw-source scan only.** v1 does *not* yet take a
//!   [`javascript-ast::Program`] — that integration is deferred to a
//!   follow-up so this crate stays independently testable (no JS pipeline
//!   spin-up required).
//! - **Byte-anchor only.** Each [`BlockComment`] records the byte offset
//!   of the comment's `/**` opener as `anchor_byte`. The CLOC05 "anchor
//!   to the next anchorable declaration" logic happens once the AST
//!   integration lands.
//! - **Best-effort string handling.** v1 does *not* pre-strip JS string
//!   literals before scanning. A `/** ... */` inside a string will be
//!   reported as a real comment. This matches what the old "regex over
//!   raw source" world has always done and is a documented limitation;
//!   the AST-driven follow-up gets it right.
//!
//! # What it does for you
//!
//! Given `let x = 1; /** @type {number} */ let y = 2;` it returns one
//! `BlockComment` whose `inner` is `"@type {number}"` (markers stripped,
//! whitespace around the inner content trimmed), whose `span` covers
//! bytes from the `/` of `/**` through the `/` of `*/`, and whose
//! `anchor_byte` is the offset of the `/`.
//!
//! Multi-line comments have their per-line `* ` continuation prefixes
//! removed, so:
//!
//! ```text
//! /**
//!  * @param {string} name
//!  * @returns {boolean}
//!  */
//! ```
//!
//! …yields an `inner` value of:
//!
//! ```text
//! @param {string} name
//! @returns {boolean}
//! ```

use coding_adventures_javascript_tokens::Span;

/// One JSDoc block comment extracted from a JavaScript source string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockComment {
    /// The half-open `[start, end)` byte range of the comment **including**
    /// the surrounding `/**` and `*/` markers.
    pub span: Span,

    /// The cleaned interior text — markers stripped, per-line `* `
    /// continuation prefixes removed, leading/trailing whitespace
    /// trimmed. Ready to feed into `jsdoc-lexer`.
    pub inner: String,

    /// Byte offset of the comment's opening `/`. Same as `span.start`,
    /// surfaced as its own field so downstream code can pass it as the
    /// CV-`Origin.location` value without reaching into `span`.
    ///
    /// When the AST integration follow-up lands, this becomes "byte
    /// offset of the AST node this comment anchors to" — for now it's
    /// just the comment's own start.
    pub anchor_byte: u32,
}

/// Scan `source` for `/** ... */` JSDoc block comments.
///
/// Returns each found comment in source order with markers stripped and
/// continuation prefixes removed. Comments that aren't valid JSDoc
/// (single `*` block comments like `/* ... */`) are ignored — JSDoc
/// specifically requires `/**` (double-asterisk opener).
///
/// See the crate-level docs for the v1 limitations: no string-literal
/// awareness, no AST anchoring.
pub fn extract_block_comments(source: &str) -> Vec<BlockComment> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i + 4 <= bytes.len() {
        // Look for a `/**` opener that isn't `/***...` continuing (a
        // single triple-star is still valid JSDoc; `/****` is technically
        // also accepted by most tools, treat it the same).
        if bytes[i] == b'/' && bytes[i + 1] == b'*' && bytes[i + 2] == b'*' {
            // Reject the `/**/` empty comment — that's a `/*` `*/` empty
            // block comment, not JSDoc. The byte after `/**` must NOT be
            // `/`.
            if i + 3 < bytes.len() && bytes[i + 3] == b'/' {
                // `/**/ ` — empty non-JSDoc comment. Skip just the `/*`.
                i += 2;
                continue;
            }

            // Find the closing `*/`. We need at least three more bytes
            // after the opener for there to be content + closer.
            let body_start = i + 3;
            let mut j = body_start;
            let mut end = None;
            while j + 1 < bytes.len() {
                if bytes[j] == b'*' && bytes[j + 1] == b'/' {
                    end = Some(j);
                    break;
                }
                j += 1;
            }

            match end {
                Some(close_at) => {
                    let raw_inner = &source[body_start..close_at];
                    let inner = clean_inner(raw_inner);
                    let end_after_marker = close_at + 2; // past `*/`
                    out.push(BlockComment {
                        span: Span::new(i as u32, end_after_marker as u32),
                        inner,
                        anchor_byte: i as u32,
                    });
                    i = end_after_marker;
                    continue;
                }
                None => {
                    // Unterminated /** ... — stop scanning. A real
                    // pipeline would surface this as a lexer error; this
                    // crate just bails.
                    break;
                }
            }
        }
        i += 1;
    }

    out
}

/// Clean a block comment's raw interior.
///
/// Strips per-line `* ` (or `*`) continuation prefixes left over from
/// the convention of starting each JSDoc continuation line with ` * `.
/// Also trims leading/trailing whitespace on the result so the lexer
/// doesn't see decorative padding.
///
/// Empty lines are preserved (without their prefix) so multi-paragraph
/// comments keep their paragraph breaks.
fn clean_inner(raw: &str) -> String {
    // Continuation prefixes only apply to lines AFTER the first one: the
    // first line is the content immediately following the opening `/**`
    // (or `/***`), where any `*` is actual body, not a column-aligned
    // line marker. Subsequent lines may have ` * ` to align with the
    // opener's column — those are decorative and get stripped.
    let mut out = String::with_capacity(raw.len());
    let mut first = true;
    for line in raw.split('\n') {
        if first {
            out.push_str(line);
            first = false;
        } else {
            out.push('\n');
            out.push_str(strip_continuation_prefix(line));
        }
    }
    out.trim().to_string()
}

/// Strip the leading `* ` continuation marker (and any whitespace
/// before it) from a single comment line.
///
/// Examples:
///   ` * @param {string} x` → `@param {string} x`
///   `\t  *@type {number}` → `@type {number}`
///   `   *`                → `` (empty)
///   `no asterisk`         → `no asterisk` (untouched)
fn strip_continuation_prefix(line: &str) -> &str {
    let trimmed_leading = line.trim_start();
    if let Some(rest) = trimmed_leading.strip_prefix('*') {
        // Eat one optional space after the asterisk to drop the `* `
        // form's space. Leave any further whitespace intact.
        rest.strip_prefix(' ').unwrap_or(rest)
    } else {
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_yields_nothing() {
        assert!(extract_block_comments("").is_empty());
    }

    #[test]
    fn source_with_no_comments_yields_nothing() {
        assert!(extract_block_comments("let x = 1; var y = 2;").is_empty());
    }

    #[test]
    fn single_line_jsdoc_extracts_cleanly() {
        let src = "let x = 1; /** @type {number} */ let y = 2;";
        let comments = extract_block_comments(src);
        assert_eq!(comments.len(), 1);
        let c = &comments[0];
        assert_eq!(c.inner, "@type {number}");
        // The span starts at the `/` of `/**` and ends one past the `/` of `*/`.
        let start = src.find("/**").unwrap() as u32;
        let end = src.find("*/").unwrap() as u32 + 2;
        assert_eq!(c.span, Span::new(start, end));
        assert_eq!(c.anchor_byte, start);
    }

    #[test]
    fn multi_line_jsdoc_strips_continuation_prefix() {
        let src = "\
/**
 * @param {string} name
 * @returns {boolean}
 */
function f(name) {}
";
        let comments = extract_block_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(
            comments[0].inner,
            "@param {string} name\n@returns {boolean}"
        );
    }

    #[test]
    fn multiple_comments_are_returned_in_order() {
        let src = "\
/** @type {number} */ let x = 1;
/** @type {string} */ let y = 'two';
";
        let comments = extract_block_comments(src);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].inner, "@type {number}");
        assert_eq!(comments[1].inner, "@type {string}");
        // Spans are non-overlapping and ordered.
        assert!(comments[0].span.end <= comments[1].span.start);
    }

    #[test]
    fn empty_jsdoc_comment_extracts_empty_inner() {
        let src = "/***/let x;";
        let comments = extract_block_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "");
        // span covers /***/ — 5 bytes.
        assert_eq!(comments[0].span, Span::new(0, 5));
    }

    #[test]
    fn non_jsdoc_block_comment_is_skipped() {
        // Single-asterisk block comment is NOT JSDoc.
        let src = "/* @type {number} */ let x;";
        assert!(extract_block_comments(src).is_empty());
    }

    #[test]
    fn empty_non_jsdoc_block_comment_is_skipped() {
        // `/**/ ` is a `/* */ ` empty block comment, not JSDoc.
        let src = "/**/let x;";
        assert!(extract_block_comments(src).is_empty());
    }

    #[test]
    fn triple_star_after_open_is_still_jsdoc() {
        // `/*** ... */` — many JSDoc tools accept this; we do too.
        let src = "/*** @type {number} */ let x;";
        let comments = extract_block_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "* @type {number}");
        // The third `*` is treated as part of the inner body. That
        // matches what jsdoc.app and TypeScript's checkJs do.
    }

    #[test]
    fn unterminated_comment_stops_scanning_gracefully() {
        let src = "let x = 1; /** unterminated\n   * still going";
        // Pipeline-friendly behavior: bail rather than panic. The lexer
        // pipeline would surface this as an error one level up.
        let comments = extract_block_comments(src);
        assert!(comments.is_empty(), "expected no comments, got {:?}", comments);
    }

    #[test]
    fn comment_with_tabs_in_continuation_strips_correctly() {
        let src = "/**\n\t* @type {string}\n\t*/\n";
        let comments = extract_block_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "@type {string}");
    }

    #[test]
    fn block_comment_inside_string_is_a_false_positive_v1() {
        // DOCUMENTED LIMITATION (CLOC05 §"v1 scope notes"): v1 doesn't
        // pre-strip string literals, so a `/** */` inside a string IS
        // reported. The AST-driven follow-up gets this right.
        let src = r#"let s = "/** not really a comment */";"#;
        let comments = extract_block_comments(src);
        assert_eq!(
            comments.len(),
            1,
            "v1 limitation: comments inside strings are still extracted"
        );
        assert_eq!(comments[0].inner, "not really a comment");
    }

    #[test]
    fn anchor_byte_matches_span_start() {
        let comments = extract_block_comments("  /** @type {boolean} */");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].anchor_byte, comments[0].span.start);
    }

    #[test]
    fn empty_blank_lines_inside_comment_preserved() {
        // Paragraph break between two tags should survive.
        let src = "\
/**
 * First paragraph.
 *
 * @type {number}
 */
";
        let comments = extract_block_comments(src);
        assert_eq!(comments.len(), 1);
        let inner = &comments[0].inner;
        // The two halves are joined by exactly one blank line.
        assert!(inner.contains("First paragraph."));
        assert!(inner.contains("@type {number}"));
        assert!(inner.contains("\n\n"), "expected blank line between halves; got {inner:?}");
    }
}
