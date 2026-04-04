//! Inline Content Parser
//!
//! Phase 2 of AsciiDoc parsing: convert raw inline content strings into
//! a tree of `InlineNode` values.
//!
//! # Priority Order
//!
//! Inline constructs are checked in priority order so that longer markers
//! (e.g. `**`) take precedence over shorter ones (e.g. `*`):
//!
//!  1. Two trailing spaces + `\n`  → `HardBreak`
//!  2. `\` + `\n`                  → `HardBreak`
//!  3. `\n`                        → `SoftBreak`
//!  4. Backtick `` ` ``            → `CodeSpan` (verbatim)
//!  5. `**...**`                   → `Strong` (unconstrained)
//!  6. `__...__`                   → `Emphasis` (unconstrained)
//!  7. `*...*`                     → `Strong` (constrained; AsciiDoc * = strong!)
//!  8. `_..._`                     → `Emphasis` (constrained)
//!  9. `link:url[text]`            → `Link`
//! 10. `image:url[alt]`            → `Image`
//! 11. `<<anchor,text>>`           → `Link`
//! 12. `https://` / `http://`      → `Link` (with `[text]`) or `Autolink` (bare)
//! 13. Default                     → text buffer
//!
//! # AsciiDoc Bold vs Markdown Bold
//!
//! In AsciiDoc, `*text*` produces `<strong>` (bold), NOT `<em>` (italic).
//! This is the opposite of Markdown. The inline parser maps:
//!   `*` → Strong
//!   `_` → Emphasis

use document_ast::{
    AutolinkNode, CodeSpanNode, EmphasisNode, HardBreakNode, ImageNode,
    InlineNode, LinkNode, SoftBreakNode, StrongNode, TextNode,
};

/// Parse a raw AsciiDoc inline string into a `Vec<InlineNode>`.
///
/// This is Phase 2 of the parser. The result is ready for any back-end
/// renderer that consumes the Document AST.
pub fn parse_inlines(raw: &str) -> Vec<InlineNode> {
    if raw.is_empty() {
        return Vec::new();
    }
    let src = raw.as_bytes();
    let mut pos = 0usize;
    let mut nodes: Vec<InlineNode> = Vec::new();
    let mut text_buf = String::new();

    let flush_text = |buf: &mut String, nodes: &mut Vec<InlineNode>| {
        if !buf.is_empty() {
            nodes.push(InlineNode::Text(TextNode { value: buf.clone() }));
            buf.clear();
        }
    };

    while pos < src.len() {
        // ── Hard break: two trailing spaces before \n ──────────────────────
        if src[pos] == b' '
            && pos + 2 < src.len()
            && src[pos + 1] == b' '
            && src[pos + 2] == b'\n'
        {
            flush_text(&mut text_buf, &mut nodes);
            nodes.push(InlineNode::HardBreak(HardBreakNode));
            pos += 3;
            continue;
        }

        // ── Hard break: backslash before \n ───────────────────────────────
        if src[pos] == b'\\'
            && pos + 1 < src.len()
            && src[pos + 1] == b'\n'
        {
            flush_text(&mut text_buf, &mut nodes);
            nodes.push(InlineNode::HardBreak(HardBreakNode));
            pos += 2;
            continue;
        }

        // ── Soft break: plain newline ─────────────────────────────────────
        if src[pos] == b'\n' {
            flush_text(&mut text_buf, &mut nodes);
            nodes.push(InlineNode::SoftBreak(SoftBreakNode));
            pos += 1;
            continue;
        }

        // ── Code span: `...` ─────────────────────────────────────────────
        if src[pos] == b'`' {
            if let Some((node, advance)) = try_code_span(raw, pos) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── Strong unconstrained: **...** ────────────────────────────────
        if pos + 1 < src.len() && src[pos] == b'*' && src[pos + 1] == b'*' {
            if let Some((node, advance)) = try_marker(raw, pos, "**", |inner| {
                InlineNode::Strong(StrongNode { children: inner })
            }) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── Emphasis unconstrained: __...__ ──────────────────────────────
        if pos + 1 < src.len() && src[pos] == b'_' && src[pos + 1] == b'_' {
            if let Some((node, advance)) = try_marker(raw, pos, "__", |inner| {
                InlineNode::Emphasis(EmphasisNode { children: inner })
            }) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── Strong constrained: *...* (AsciiDoc: * = strong!) ────────────
        if src[pos] == b'*' {
            if let Some((node, advance)) = try_constrained(raw, pos, b'*', |inner| {
                InlineNode::Strong(StrongNode { children: inner })
            }) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── Emphasis constrained: _..._ ──────────────────────────────────
        if src[pos] == b'_' {
            if let Some((node, advance)) = try_constrained(raw, pos, b'_', |inner| {
                InlineNode::Emphasis(EmphasisNode { children: inner })
            }) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── link:url[text] ────────────────────────────────────────────────
        if raw[pos..].starts_with("link:") {
            if let Some((node, advance)) = try_link_macro(raw, pos) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── image:url[alt] ────────────────────────────────────────────────
        if raw[pos..].starts_with("image:") {
            if let Some((node, advance)) = try_image_macro(raw, pos) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── Cross-reference: <<anchor,text>> or <<anchor>> ────────────────
        if pos + 1 < src.len() && src[pos] == b'<' && src[pos + 1] == b'<' {
            if let Some((node, advance)) = try_cross_ref(raw, pos) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── URL with optional bracket text ────────────────────────────────
        if raw[pos..].starts_with("https://") || raw[pos..].starts_with("http://") {
            if let Some((node, advance)) = try_url(raw, pos) {
                flush_text(&mut text_buf, &mut nodes);
                nodes.push(node);
                pos += advance;
                continue;
            }
        }

        // ── Default: consume one byte ─────────────────────────────────────
        // Safety: we're iterating over bytes, so this is valid ASCII or
        // the first byte of a multi-byte UTF-8 sequence. We push the byte
        // as a char after checking it is valid UTF-8 using the str slice.
        let ch_len = utf8_char_len(src[pos]);
        if pos + ch_len <= src.len() {
            text_buf.push_str(&raw[pos..pos + ch_len]);
            pos += ch_len;
        } else {
            text_buf.push(src[pos] as char);
            pos += 1;
        }
    }

    flush_text(&mut text_buf, &mut nodes);
    nodes
}

// ─── Inline construct parsers ─────────────────────────────────────────────────

/// Try to parse a code span starting at `pos`.
fn try_code_span(src: &str, pos: usize) -> Option<(InlineNode, usize)> {
    let bytes = src.as_bytes();
    if bytes[pos] != b'`' {
        return None;
    }
    // Count opening backticks
    let mut num_ticks = 0usize;
    while pos + num_ticks < bytes.len() && bytes[pos + num_ticks] == b'`' {
        num_ticks += 1;
    }
    let marker = "`".repeat(num_ticks);
    let rest = &src[pos + num_ticks..];
    let idx = rest.find(&marker)?;
    let mut content = &rest[..idx];
    // Strip single leading/trailing space if present on both sides
    if content.len() >= 2 && content.starts_with(' ') && content.ends_with(' ') {
        content = &content[1..content.len() - 1];
    }
    let advance = num_ticks + idx + num_ticks;
    Some((InlineNode::CodeSpan(CodeSpanNode { value: content.to_string() }), advance))
}

/// Try to parse an unconstrained span with marker (e.g. `**`, `__`).
fn try_marker<F>(src: &str, pos: usize, marker: &str, wrap: F) -> Option<(InlineNode, usize)>
where
    F: Fn(Vec<InlineNode>) -> InlineNode,
{
    let mlen = marker.len();
    if pos + mlen > src.len() {
        return None;
    }
    if &src[pos..pos + mlen] != marker {
        return None;
    }
    let rest = &src[pos + mlen..];
    let idx = rest.find(marker)?;
    let inner_str = &rest[..idx];
    let inner = parse_inlines(inner_str);
    let advance = mlen + idx + mlen;
    Some((wrap(inner), advance))
}

/// Try to parse a constrained span with single-char marker `ch`.
///
/// Constrained means the opener must not be followed by the same char again,
/// and boundaries must be non-word chars (or string start/end).
fn try_constrained<F>(src: &str, pos: usize, ch: u8, wrap: F) -> Option<(InlineNode, usize)>
where
    F: Fn(Vec<InlineNode>) -> InlineNode,
{
    let bytes = src.as_bytes();
    if bytes[pos] != ch {
        return None;
    }
    // Must not be doubled (** or __)
    if pos + 1 < bytes.len() && bytes[pos + 1] == ch {
        return None;
    }
    // Left boundary: start of string or non-word char
    if pos > 0 && is_word_char(bytes[pos - 1]) {
        return None;
    }
    // Find closing marker
    let mut end = pos + 1;
    while end < bytes.len() {
        if bytes[end] == ch {
            // Not doubled
            if end + 1 < bytes.len() && bytes[end + 1] == ch {
                end += 1;
                continue;
            }
            // Right boundary: end of string or non-word char
            if end + 1 < bytes.len() && is_word_char(bytes[end + 1]) {
                end += 1;
                continue;
            }
            let inner_str = &src[pos + 1..end];
            if inner_str.is_empty() {
                return None;
            }
            let inner = parse_inlines(inner_str);
            let advance = end - pos + 1;
            return Some((wrap(inner), advance));
        }
        end += 1;
    }
    None
}

/// Try to parse `link:url[text]`.
fn try_link_macro(src: &str, pos: usize) -> Option<(InlineNode, usize)> {
    let rest = &src[pos..];
    if !rest.starts_with("link:") {
        return None;
    }
    let after = &rest[5..];
    let bracket = after.find('[')?;
    let url = &after[..bracket];
    if url.is_empty() {
        return None;
    }
    let after2 = &after[bracket + 1..];
    let close = after2.find(']')?;
    let label = &after2[..close];
    let advance = 5 + bracket + 1 + close + 1;
    let children = if label.is_empty() {
        vec![InlineNode::Text(TextNode { value: url.to_string() })]
    } else {
        parse_inlines(label)
    };
    Some((InlineNode::Link(LinkNode {
        destination: url.to_string(),
        title: None,
        children,
    }), advance))
}

/// Try to parse `image:url[alt]`.
fn try_image_macro(src: &str, pos: usize) -> Option<(InlineNode, usize)> {
    let rest = &src[pos..];
    if !rest.starts_with("image:") {
        return None;
    }
    let after = &rest[6..];
    let bracket = after.find('[')?;
    let url = &after[..bracket];
    if url.is_empty() {
        return None;
    }
    let after2 = &after[bracket + 1..];
    let close = after2.find(']')?;
    let alt = &after2[..close];
    let advance = 6 + bracket + 1 + close + 1;
    Some((InlineNode::Image(ImageNode {
        destination: url.to_string(),
        title: None,
        alt: alt.to_string(),
    }), advance))
}

/// Try to parse `<<anchor,text>>` or `<<anchor>>`.
fn try_cross_ref(src: &str, pos: usize) -> Option<(InlineNode, usize)> {
    let rest = &src[pos..];
    if !rest.starts_with("<<") {
        return None;
    }
    let after = &rest[2..];
    let close = after.find(">>")?;
    let inner = &after[..close];
    let advance = 2 + close + 2;
    let parts: Vec<&str> = inner.splitn(2, ',').collect();
    let anchor = format!("#{}", parts[0].trim());
    let label = if parts.len() == 2 { parts[1].trim().to_string() } else { parts[0].trim().to_string() };
    Some((InlineNode::Link(LinkNode {
        destination: anchor,
        title: None,
        children: vec![InlineNode::Text(TextNode { value: label })],
    }), advance))
}

/// Try to parse a URL (`http://` or `https://`) with optional `[text]` suffix.
fn try_url(src: &str, pos: usize) -> Option<(InlineNode, usize)> {
    let rest = &src[pos..];
    // Find end of URL: space, newline, or `[`
    let url_end = rest.find(|c| c == ' ' || c == '\n' || c == '[').unwrap_or(rest.len());
    let url = &rest[..url_end];
    if url.is_empty() {
        return None;
    }
    // Check for [text] suffix
    if url_end < rest.len() && rest.as_bytes()[url_end] == b'[' {
        let after = &rest[url_end + 1..];
        if let Some(close) = after.find(']') {
            let label = &after[..close];
            let advance = url_end + 1 + close + 1;
            let children = if label.is_empty() {
                vec![InlineNode::Text(TextNode { value: url.to_string() })]
            } else {
                parse_inlines(label)
            };
            return Some((InlineNode::Link(LinkNode {
                destination: url.to_string(),
                title: None,
                children,
            }), advance));
        }
    }
    // Bare URL → autolink
    Some((InlineNode::Autolink(AutolinkNode {
        destination: url.to_string(),
        is_email: false,
    }), url_end))
}

// ─── Character classification ─────────────────────────────────────────────────

/// Returns `true` for ASCII letters, digits, and underscore (word characters).
fn is_word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Returns the byte length of the UTF-8 character starting with `first_byte`.
fn utf8_char_len(first_byte: u8) -> usize {
    if first_byte & 0b1111_0000 == 0b1111_0000 { 4 }
    else if first_byte & 0b1110_0000 == 0b1110_0000 { 3 }
    else if first_byte & 0b1100_0000 == 0b1100_0000 { 2 }
    else { 1 }
}
