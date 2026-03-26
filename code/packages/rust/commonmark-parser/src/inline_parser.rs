//! Inline Parser
//!
//! Phase 2 of CommonMark parsing: scan raw inline content strings (produced
//! by the block parser) and emit inline AST nodes — emphasis, links, code
//! spans, etc.
//!
//! # Overview of Inline Constructs
//!
//! CommonMark recognises ten inline constructs, processed left-to-right:
//!
//!   1. Backslash escapes       `\*`    → literal `*`
//!   2. HTML character refs     `&amp;` → `&`
//!   3. Code spans              `` `code` ``
//!   4. HTML inline             `<em>`, `<!-- -->`, `<?...?>`
//!   5. Autolinks               `<https://example.com>`, `<me@example.com>`
//!   6. Hard line breaks        two trailing spaces + newline, or `\` + newline
//!   7. Soft line breaks        single newline within a paragraph
//!   8. Emphasis / strong       `*em*`, `**strong**`, `_em_`, `__strong__`
//!   9. Links                   `[text](url)`, `[text][label]`, `[text][]`
//!  10. Images                  `![alt](url)`, `![alt][label]`
//!
//! # The Delimiter Stack Algorithm
//!
//! Emphasis is the hardest part of CommonMark inline parsing. The rules are
//! context-sensitive: whether `*` or `_` can open or close emphasis depends
//! on what precedes and follows the run. CommonMark Appendix A defines the
//! canonical "delimiter stack" algorithm.
//!
//! The algorithm has two phases:
//!
//!   A. SCAN — read the input left-to-right, building a flat list of "tokens":
//!      ordinary text, delimiter runs (* ** _ __), code spans, links, etc.
//!      Each delimiter run is tagged as "can_open", "can_close", or both.
//!
//!   B. RESOLVE — walk the token list, matching openers with the nearest
//!      valid closers. For each matched pair, wrap the intervening tokens
//!      in an emphasis or strong node.
//!
//! # Flanking Rules (CommonMark spec §6.2)
//!
//! A delimiter run of `*` is LEFT-FLANKING (can open) if:
//!   (a) not followed by Unicode whitespace, AND
//!   (b) either not followed by Unicode punctuation,
//!       OR preceded by Unicode whitespace or Unicode punctuation.
//!
//! A delimiter run of `*` is RIGHT-FLANKING (can close) if:
//!   (a) not preceded by Unicode whitespace, AND
//!   (b) either not preceded by Unicode punctuation,
//!       OR followed by Unicode whitespace or Unicode punctuation.
//!
//! For `_`, the open/close rules add extra conditions to avoid
//! intra-word emphasis in identifiers like `foo_bar_baz`.

use document_ast::*;
use crate::block_parser::{FinalBlock, LinkRefMap};
use crate::scanner::{
    Scanner, is_ascii_punctuation, is_unicode_punctuation,
    is_ascii_whitespace, is_unicode_whitespace, normalize_link_label, normalize_url,
};
use crate::entities::{decode_entity, decode_entities};

// ─── Token Types ──────────────────────────────────────────────────────────────

/// A delimiter run: maximal run of `*` or `_`.
#[derive(Debug, Clone)]
struct DelimToken {
    ch: char,      // `*` or `_`
    count: usize,  // length of the run
    can_open: bool,
    can_close: bool,
    active: bool,
}

/// A fully-resolved inline node.
#[derive(Debug, Clone)]
struct NodeToken {
    node: InlineNode,
}

/// A bracket opener `[` or `![` — may become a link or image.
#[derive(Debug, Clone)]
struct BracketToken {
    is_image: bool,
    active: bool,
    source_pos: usize, // scanner position immediately after `[`
}

#[derive(Debug, Clone)]
enum Token {
    Delim(DelimToken),
    Node(NodeToken),
    Bracket(BracketToken),
}

// ─── Inline Parser ────────────────────────────────────────────────────────────

/// Parse a raw inline content string into a list of InlineNode trees.
pub fn parse_inline(raw: &str, link_refs: &LinkRefMap) -> Vec<InlineNode> {
    let mut scanner = Scanner::new(raw);
    let mut tokens: Vec<Token> = Vec::new();
    let mut bracket_stack: Vec<usize> = Vec::new(); // indices into tokens
    let mut text_buf = String::new();

    fn flush_text(tokens: &mut Vec<Token>, text_buf: &mut String) {
        if !text_buf.is_empty() {
            tokens.push(Token::Node(NodeToken {
                node: InlineNode::Text(TextNode { value: std::mem::take(text_buf) }),
            }));
        }
    }

    while !scanner.done() {
        let ch = scanner.peek_char(0);

        // 1. Backslash escape
        if ch == '\\' {
            let next = scanner.peek_char(1);
            if next != '\0' && is_ascii_punctuation(next) {
                scanner.skip(1 + next.len_utf8());
                text_buf.push(next);
                continue;
            }
            if next == '\n' {
                scanner.skip(2);
                flush_text(&mut tokens, &mut text_buf);
                tokens.push(Token::Node(NodeToken { node: InlineNode::HardBreak(HardBreakNode) }));
                continue;
            }
            scanner.skip(1);
            text_buf.push('\\');
            continue;
        }

        // 2. HTML character reference
        if ch == '&' {
            // Match &name; or &#NNN; or &#xHHH;
            let rest = scanner.rest();
            if let Some(end) = find_entity_end(rest) {
                let entity = &rest[..end];
                let decoded = decode_entity(entity);
                text_buf.push_str(&decoded);
                scanner.skip(end);
                continue;
            }
            scanner.skip(1);
            text_buf.push('&');
            continue;
        }

        // 3. Code span
        if ch == '`' {
            if let Some(span) = try_code_span(&mut scanner) {
                flush_text(&mut tokens, &mut text_buf);
                tokens.push(Token::Node(NodeToken { node: InlineNode::CodeSpan(span) }));
                continue;
            }
            let ticks = scanner.consume_while(|c| c == '`').to_string();
            text_buf.push_str(&ticks);
            continue;
        }

        // 4 & 5. HTML inline and autolinks
        if ch == '<' {
            if let Some(autolink) = try_autolink(&mut scanner) {
                flush_text(&mut tokens, &mut text_buf);
                tokens.push(Token::Node(NodeToken { node: InlineNode::Autolink(autolink) }));
                continue;
            }
            if let Some(html) = try_html_inline(&mut scanner) {
                flush_text(&mut tokens, &mut text_buf);
                tokens.push(Token::Node(NodeToken { node: InlineNode::RawInline(html) }));
                continue;
            }
            scanner.skip(1);
            text_buf.push('<');
            continue;
        }

        // Image opener `![`
        if ch == '!' && scanner.peek_char(1) == '[' {
            flush_text(&mut tokens, &mut text_buf);
            let tok_idx = tokens.len();
            scanner.skip(2);
            bracket_stack.push(tok_idx);
            tokens.push(Token::Bracket(BracketToken {
                is_image: true, active: true, source_pos: scanner.pos,
            }));
            continue;
        }

        // Link opener `[`
        if ch == '[' {
            flush_text(&mut tokens, &mut text_buf);
            let tok_idx = tokens.len();
            scanner.skip(1);
            bracket_stack.push(tok_idx);
            tokens.push(Token::Bracket(BracketToken {
                is_image: false, active: true, source_pos: scanner.pos,
            }));
            continue;
        }

        // Link/image closer `]`
        if ch == ']' {
            scanner.skip(1);

            // Check if top of bracket stack is a deactivated non-image opener
            if let Some(&top_idx) = bracket_stack.last() {
                if let Token::Bracket(ref bt) = tokens[top_idx].clone() {
                    if !bt.active && !bt.is_image {
                        bracket_stack.pop();
                        text_buf.push(']');
                        continue;
                    }
                }
            }

            let opener_stack_idx = find_active_bracket_opener(&bracket_stack, &tokens);
            if opener_stack_idx == -1 {
                text_buf.push(']');
                continue;
            }

            let opener_tok_idx = bracket_stack[opener_stack_idx as usize];
            let opener = match &tokens[opener_tok_idx] {
                Token::Bracket(b) => b.clone(),
                _ => { text_buf.push(']'); continue; }
            };

            flush_text(&mut tokens, &mut text_buf);

            let inner_tokens_after_opener: Vec<Token> = tokens[opener_tok_idx + 1..].to_vec();
            let closer_pos = scanner.pos - 1;
            let inner_text_for_label = scanner.source[opener.source_pos..closer_pos].to_string();

            let link_result = try_link_after_close(&mut scanner, link_refs, &inner_text_for_label);

            if link_result.is_none() {
                // Deactivate opener
                if let Token::Bracket(ref mut bt) = tokens[opener_tok_idx] {
                    bt.active = false;
                }
                bracket_stack.remove(opener_stack_idx as usize);
                text_buf.push(']');
                continue;
            }

            let link_result = link_result.unwrap();

            flush_text(&mut tokens, &mut text_buf);

            // Splice: remove the opener bracket token and all inner tokens from
            // the token list. After flush_text above, opener_tok_idx is still
            // valid (flush_text appends to the end, not before the opener).
            //
            // tokens.drain(opener_tok_idx..) removes:
            //   [0] = the opener bracket token (BracketToken)
            //   [1..] = any inner resolved tokens (links, text, etc.)
            // We keep tokens[..opener_tok_idx] and replace with the new link/image.
            let drained: Vec<Token> = tokens.drain(opener_tok_idx..).collect();
            // drained[0] is the opener bracket token, drained[1..] are inner tokens
            let inner_for_resolve: Vec<Token> = drained.into_iter().skip(1).collect();
            bracket_stack.truncate(opener_stack_idx as usize);

            let inner_nodes = resolve_emphasis(inner_for_resolve);

            if opener.is_image {
                let alt_text = extract_plain_text(&inner_nodes);
                tokens.push(Token::Node(NodeToken {
                    node: InlineNode::Image(ImageNode {
                        destination: link_result.destination,
                        title: link_result.title,
                        alt: alt_text,
                    }),
                }));
            } else {
                tokens.push(Token::Node(NodeToken {
                    node: InlineNode::Link(LinkNode {
                        destination: link_result.destination,
                        title: link_result.title,
                        children: inner_nodes,
                    }),
                }));
                // Deactivate all preceding non-image link openers
                for &idx in &bracket_stack {
                    if let Token::Bracket(ref mut bt) = tokens[idx] {
                        if !bt.is_image {
                            bt.active = false;
                        }
                    }
                }
            }
            continue;
        }

        // 8. Emphasis/strong delimiter run
        if ch == '*' || ch == '_' || (ch == '~' && scanner.peek_at(1) == '~') {
            flush_text(&mut tokens, &mut text_buf);
            let delim = scan_delimiter_run(&mut scanner, ch);
            tokens.push(Token::Delim(delim));
            continue;
        }

        // 6 & 7. Line breaks
        if ch == '\n' {
            scanner.skip(1);
            if text_buf.ends_with("  ") || text_buf.trim_end_matches(|c| c == ' ' || c == '\t').len() + 2 <= text_buf.len() {
                // Two or more trailing spaces → hard break
                let trimmed = text_buf.trim_end_matches(|c: char| c == ' ' || c == '\t').to_string();
                text_buf = trimmed;
                flush_text(&mut tokens, &mut text_buf);
                tokens.push(Token::Node(NodeToken { node: InlineNode::HardBreak(HardBreakNode) }));
            } else {
                let trimmed = text_buf.trim_end_matches(|c: char| c == ' ' || c == '\t').to_string();
                text_buf = trimmed;
                flush_text(&mut tokens, &mut text_buf);
                tokens.push(Token::Node(NodeToken { node: InlineNode::SoftBreak(SoftBreakNode) }));
            }
            continue;
        }

        text_buf.push(ch);
        scanner.skip(ch.len_utf8());
    }

    flush_text(&mut tokens, &mut text_buf);

    // ── Resolve Phase ─────────────────────────────────────────────────────
    resolve_emphasis(tokens)
}

fn find_entity_end(s: &str) -> Option<usize> {
    if !s.starts_with('&') { return None; }
    let inner_start = 1;
    let bytes = s.as_bytes();

    if inner_start >= bytes.len() { return None; }

    // &#xHHH; or &#NNN;
    if bytes[inner_start] == b'#' {
        if inner_start + 1 >= bytes.len() { return None; }
        let start = inner_start + 1;
        let (is_hex, digit_start) = if bytes[start] == b'x' || bytes[start] == b'X' {
            (true, start + 1)
        } else {
            (false, start)
        };
        if digit_start >= bytes.len() { return None; }
        let end = if is_hex {
            bytes[digit_start..].iter().take(6).take_while(|&&b| b.is_ascii_hexdigit()).count()
        } else {
            bytes[digit_start..].iter().take(7).take_while(|&&b| b.is_ascii_digit()).count()
        };
        if end == 0 { return None; }
        let semi_pos = digit_start + end;
        if semi_pos < bytes.len() && bytes[semi_pos] == b';' {
            return Some(semi_pos + 1);
        }
        return None;
    }

    // Named: &name;
    if !bytes[inner_start].is_ascii_alphabetic() { return None; }
    let end = bytes[inner_start..].iter().take(32)
        .take_while(|&&b| b.is_ascii_alphanumeric())
        .count();
    if end == 0 { return None; }
    let semi_pos = inner_start + end;
    if semi_pos < bytes.len() && bytes[semi_pos] == b';' {
        Some(semi_pos + 1)
    } else {
        None
    }
}

// ─── Delimiter Run Scanning ────────────────────────────────────────────────────

fn scan_delimiter_run(scanner: &mut Scanner, char: char) -> DelimToken {
    let source = scanner.source.clone();
    let run_start = scanner.pos;

    let pre_char = if run_start > 0 {
        // Get last char before run_start
        source[..run_start].chars().last().unwrap_or('\0')
    } else { '\0' };

    let run = scanner.consume_while(|c| c == char).to_string();
    let count = run.chars().count();
    let post_char = scanner.source[scanner.pos..].chars().next().unwrap_or('\0');

    let after_whitespace = post_char == '\0' || is_unicode_whitespace(post_char);
    let after_punctuation = post_char != '\0' && is_unicode_punctuation(post_char);
    let before_whitespace = pre_char == '\0' || is_unicode_whitespace(pre_char);
    let before_punctuation = pre_char != '\0' && is_unicode_punctuation(pre_char);

    let left_flanking = !after_whitespace && (!after_punctuation || before_whitespace || before_punctuation);
    let right_flanking = !before_whitespace && (!before_punctuation || after_whitespace || after_punctuation);

    let (can_open, can_close) = if char == '*' {
        (left_flanking, right_flanking)
    } else if char == '~' {
        (count >= 2 && left_flanking, count >= 2 && right_flanking)
    } else {
        // `_` rules: stricter to avoid intra-word
        let can_open = left_flanking && (!right_flanking || before_punctuation);
        let can_close = right_flanking && (!left_flanking || after_punctuation);
        (can_open, can_close)
    };

    DelimToken { ch: char, count, can_open, can_close, active: true }
}

// ─── Emphasis Resolution ──────────────────────────────────────────────────────

fn resolve_emphasis(mut tokens: Vec<Token>) -> Vec<InlineNode> {
    let mut i = 0;
    while i < tokens.len() {
        let closer = match &tokens[i] {
            Token::Delim(d) if d.can_close && d.active => d.clone(),
            _ => { i += 1; continue; }
        };

        // Search backwards for an opener
        let mut opener_idx: Option<usize> = None;
        for j in (0..i).rev() {
            let t = match &tokens[j] {
                Token::Delim(d) if d.can_open && d.active && d.ch == closer.ch => d.clone(),
                _ => continue,
            };

            // Mod-3 rule
            if (t.can_open && t.can_close) || (closer.can_open && closer.can_close) {
                if (t.count + closer.count) % 3 == 0 && t.count % 3 != 0 {
                    continue;
                }
            }
            opener_idx = Some(j);
            break;
        }

        let opener_idx = match opener_idx {
            None => { i += 1; continue; }
            Some(idx) => idx,
        };

        let opener_count = match &tokens[opener_idx] {
            Token::Delim(d) => d.count,
            _ => { i += 1; continue; }
        };
        let closer_count = match &tokens[i] {
            Token::Delim(d) => d.count,
            _ => { i += 1; continue; }
        };

        let use_len = if closer.ch == '~' || (opener_count >= 2 && closer_count >= 2) { 2 } else { 1 };
        let is_strong = use_len == 2;

        // Collect inner tokens and resolve recursively
        let inner_slice: Vec<Token> = tokens[opener_idx + 1..i].to_vec();
        let inner_nodes = resolve_emphasis(inner_slice);

        let emph_node: InlineNode = if closer.ch == '~' {
            InlineNode::Strikethrough(StrikethroughNode { children: inner_nodes })
        } else if is_strong {
            InlineNode::Strong(StrongNode { children: inner_nodes })
        } else {
            InlineNode::Emphasis(EmphasisNode { children: inner_nodes })
        };

        // Replace inner tokens with emphasis node
        tokens.splice(opener_idx + 1..i, std::iter::once(Token::Node(NodeToken { node: emph_node })));
        // After splice, closer is at opener_idx + 2
        i = opener_idx + 2;

        // Reduce counts
        match &mut tokens[opener_idx] {
            Token::Delim(d) => d.count -= use_len,
            _ => {}
        }
        match &mut tokens[i] {
            Token::Delim(d) => d.count -= use_len,
            _ => {}
        }

        // If opener count is 0, remove it
        let opener_empty = match &tokens[opener_idx] {
            Token::Delim(d) => d.count == 0,
            _ => false,
        };
        if opener_empty {
            tokens.remove(opener_idx);
            i = opener_idx + 1; // shifted
        }

        // If closer count is 0, remove it
        let closer_empty = match tokens.get(i) {
            Some(Token::Delim(d)) => d.count == 0,
            _ => false,
        };
        if closer_empty {
            tokens.remove(i);
        }
    }

    tokens.into_iter().flat_map(|tok| -> Vec<InlineNode> {
        match tok {
            Token::Node(n) => vec![n.node],
            Token::Bracket(b) => vec![InlineNode::Text(TextNode {
                value: if b.is_image { "![".to_string() } else { "[".to_string() },
            })],
            Token::Delim(d) => vec![InlineNode::Text(TextNode {
                value: d.ch.to_string().repeat(d.count),
            })],
        }
    }).collect()
}

// ─── Code Span ────────────────────────────────────────────────────────────────

fn try_code_span(scanner: &mut Scanner) -> Option<CodeSpanNode> {
    let saved = scanner.pos;
    let open_ticks = scanner.consume_while(|c| c == '`').to_string();
    let tick_len = open_ticks.len();

    let mut content = String::new();
    while !scanner.done() {
        if scanner.peek_char(0) == '`' {
            let close_pos = scanner.pos;
            let close_ticks = scanner.consume_while(|c| c == '`').to_string();
            if close_ticks.len() == tick_len {
                // Normalize: CR/LF → space
                content = content.replace('\r', " ").replace('\n', " ");
                // Strip one leading+trailing space if content not all-space
                if content.len() >= 2
                    && content.starts_with(' ')
                    && content.ends_with(' ')
                    && content.trim() != ""
                {
                    content = content[1..content.len() - 1].to_string();
                }
                return Some(CodeSpanNode { value: content });
            }
            content.push_str(&close_ticks);
        } else {
            let ch = scanner.advance();
            content.push(ch);
        }
    }

    scanner.pos = saved;
    None
}

// ─── HTML Inline ─────────────────────────────────────────────────────────────

fn try_html_inline(scanner: &mut Scanner) -> Option<RawInlineNode> {
    if scanner.peek_char(0) != '<' { return None; }
    let saved = scanner.pos;
    scanner.skip(1);

    let ch = scanner.peek_char(0);

    // HTML comment: <!-- ... -->
    if scanner.match_str("!--") {
        let content_start = scanner.pos;
        // Content must not start with `>` or `->`
        if scanner.peek_char(0) == '>' || scanner.source[scanner.pos..].starts_with("->") {
            let invalid_len = if scanner.peek_char(0) == '>' { 1 } else { 2 };
            scanner.skip(invalid_len);
            let val = scanner.source[saved..scanner.pos].to_string();
            return Some(RawInlineNode { format: "html".to_string(), value: val });
        }
        loop {
            if scanner.done() { scanner.pos = saved; return None; }
            if scanner.match_str("-->") {
                let content = &scanner.source[content_start..scanner.pos - 3];
                if content.ends_with('-') { scanner.pos = saved; return None; }
                let val = scanner.source[saved..scanner.pos].to_string();
                return Some(RawInlineNode { format: "html".to_string(), value: val });
            }
            scanner.skip(1);
        }
    }

    // Processing instruction: <? ... ?>
    if scanner.match_str("?") {
        loop {
            if scanner.done() { scanner.pos = saved; return None; }
            if scanner.match_str("?>") {
                let val = scanner.source[saved..scanner.pos].to_string();
                return Some(RawInlineNode { format: "html".to_string(), value: val });
            }
            scanner.skip(1);
        }
    }

    // CDATA: <![CDATA[ ... ]]>
    if scanner.match_str("![CDATA[") {
        loop {
            if scanner.done() { scanner.pos = saved; return None; }
            if scanner.match_str("]]>") {
                let val = scanner.source[saved..scanner.pos].to_string();
                return Some(RawInlineNode { format: "html".to_string(), value: val });
            }
            scanner.skip(1);
        }
    }

    // Declaration: <!UPPER...>
    if scanner.match_str("!") {
        if scanner.peek_char(0).is_ascii_uppercase() {
            scanner.consume_while(|c| c != '>');
            if scanner.match_str(">") {
                let val = scanner.source[saved..scanner.pos].to_string();
                return Some(RawInlineNode { format: "html".to_string(), value: val });
            }
        }
        scanner.pos = saved;
        return None;
    }

    // Closing tag: </tagname>
    if ch == '/' {
        scanner.skip(1);
        let tag = scanner.consume_while(|c| c.is_alphanumeric() || c == '-').to_string();
        if tag.is_empty() { scanner.pos = saved; return None; }
        scanner.skip_spaces();
        if !scanner.match_str(">") { scanner.pos = saved; return None; }
        let val = scanner.source[saved..scanner.pos].to_string();
        return Some(RawInlineNode { format: "html".to_string(), value: val });
    }

    // Open tag: <tagname attrs> or <tagname attrs/>
    if ch.is_ascii_alphabetic() {
        scanner.consume_while(|c| c.is_alphanumeric() || c == '-');
        let mut newlines = 0usize;

        loop {
            let space_len = scanner.skip_spaces();
            if newlines == 0 && scanner.peek_char(0) == '\n' {
                newlines += 1;
                scanner.skip(1);
                scanner.skip_spaces();
            }
            let next = scanner.peek_char(0);
            if next == '>' || next == '/' || next == '\0' { break; }
            if next == '\n' { scanner.pos = saved; return None; }
            if space_len == 0 && newlines == 0 { scanner.pos = saved; return None; }

            // Attribute name
            if !next.is_ascii_alphabetic() && next != '_' && next != ':' {
                scanner.pos = saved; return None;
            }
            scanner.consume_while(|c: char| c.is_alphanumeric() || c == '_' || c == ':' || c == '.' || c == '-');

            let before_eq = scanner.pos;
            scanner.skip_spaces();
            if scanner.peek_char(0) == '=' {
                scanner.skip(1);
                scanner.skip_spaces();
                let q = scanner.peek_char(0);
                if q == '"' || q == '\'' {
                    scanner.skip(1);
                    let mut closed = false;
                    while !scanner.done() {
                        let vc = scanner.peek_char(0);
                        if vc == q { scanner.skip(q.len_utf8()); closed = true; break; }
                        if vc == '\n' {
                            if newlines >= 1 { scanner.pos = saved; return None; }
                            newlines += 1;
                        }
                        scanner.skip(vc.len_utf8());
                    }
                    if !closed { scanner.pos = saved; return None; }
                } else {
                    let unquoted = scanner.consume_while(|c: char| {
                        !c.is_ascii_whitespace() && c != '"' && c != '\'' && c != '=' && c != '<' && c != '>' && c != '`'
                    }).to_string();
                    if unquoted.is_empty() { scanner.pos = saved; return None; }
                }
            } else {
                scanner.pos = before_eq;
            }
        }

        if !scanner.match_str("/>") && !scanner.match_str(">") {
            scanner.pos = saved; return None;
        }
        let val = scanner.source[saved..scanner.pos].to_string();
        return Some(RawInlineNode { format: "html".to_string(), value: val });
    }

    scanner.pos = saved;
    None
}

// ─── Autolink ─────────────────────────────────────────────────────────────────

fn try_autolink(scanner: &mut Scanner) -> Option<AutolinkNode> {
    if scanner.peek_char(0) != '<' { return None; }
    let saved = scanner.pos;
    scanner.skip(1);

    let start = scanner.pos;

    // Email autolink: local@domain
    let local = scanner.consume_while(|c: char| c != '<' && c != '>' && c != '@' && !c.is_ascii_whitespace()).to_string();
    if !local.is_empty() && scanner.peek_char(0) == '@' {
        scanner.skip(1);
        let domain = scanner.consume_while(|c: char| c != '<' && c != '>').to_string();
        if !domain.is_empty() && scanner.match_str(">") {
            // Validate
            if is_valid_email_local(&local) && is_valid_email_domain(&domain) {
                return Some(AutolinkNode {
                    destination: format!("{}@{}", local, domain),
                    is_email: true,
                });
            }
        }
    }

    // Retry as URL autolink
    scanner.pos = start;
    let scheme = scanner.consume_while(|c: char| c.is_alphanumeric() || c == '+' || c == '-' || c == '.').to_string();
    if scheme.len() >= 2 && scheme.len() <= 32 && scanner.match_str(":") {
        let path = scanner.consume_while(|c: char| c != ' ' && c != '<' && c != '>' && c != '\n').to_string();
        if scanner.match_str(">") {
            return Some(AutolinkNode {
                destination: format!("{}:{}", scheme, path),
                is_email: false,
            });
        }
    }

    scanner.pos = saved;
    None
}

fn is_valid_email_local(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c: char| c.is_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(c))
}

fn is_valid_email_domain(s: &str) -> bool {
    // label(.label)* where each label is [a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9]?
    if s.is_empty() { return false; }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.is_empty() { return true; }
    for part in &parts {
        if part.is_empty() { return false; }
        if !part.chars().all(|c: char| c.is_alphanumeric() || c == '-') { return false; }
        if part.starts_with('-') || part.ends_with('-') { return false; }
    }
    true
}

// ─── Link/Image Destination Parsing ──────────────────────────────────────────

struct LinkResult {
    destination: String,
    title: Option<String>,
}

fn try_link_after_close(
    scanner: &mut Scanner,
    link_refs: &LinkRefMap,
    inner_text: &str,
) -> Option<LinkResult> {
    let saved = scanner.pos;

    // Inline link: ( destination "title" )
    if scanner.peek_char(0) == '(' {
        let inline_result = try_inline_link(scanner, saved);
        if let Some(r) = inline_result {
            return Some(r);
        }
        scanner.pos = saved;
    }

    // Full reference: [label] or collapsed []
    if scanner.peek_char(0) == '[' {
        scanner.skip(1);
        let mut label_buf = String::new();
        let mut valid = true;
        while !scanner.done() {
            let c = scanner.peek_char(0);
            if c == ']' { scanner.skip(1); break; }
            if c == '\n' || c == '[' { valid = false; break; }
            if c == '\\' {
                scanner.skip(1);
                if !scanner.done() {
                    let next = scanner.advance();
                    label_buf.push('\\');
                    label_buf.push(next);
                }
            } else {
                label_buf.push(c);
                scanner.skip(c.len_utf8());
            }
        }
        if valid {
            if label_buf.trim().is_empty() {
                // Collapsed reference — use inner text
                let label = normalize_link_label(inner_text);
                if let Some(r) = link_refs.get(&label) {
                    return Some(LinkResult { destination: r.destination.clone(), title: r.title.clone() });
                }
            } else {
                let label = normalize_link_label(&label_buf);
                if let Some(r) = link_refs.get(&label) {
                    return Some(LinkResult { destination: r.destination.clone(), title: r.title.clone() });
                }
            }
        }
        scanner.pos = saved;
        return None;
    }

    // Shortcut reference — use inner text as label
    let label = normalize_link_label(inner_text);
    if let Some(r) = link_refs.get(&label) {
        return Some(LinkResult { destination: r.destination.clone(), title: r.title.clone() });
    }

    None
}

fn try_inline_link(scanner: &mut Scanner, outer_saved: usize) -> Option<LinkResult> {
    scanner.skip(1); // consume `(`
    skip_optional_spaces_and_newline(scanner);

    let mut destination = String::new();

    if scanner.peek_char(0) == '<' {
        scanner.skip(1);
        let mut dest_buf = String::new();
        loop {
            if scanner.done() { return None; }
            let c = scanner.peek_char(0);
            if c == '\n' || c == '\r' { return None; }
            if c == '\\' {
                scanner.skip(1);
                let next = scanner.advance();
                if is_ascii_punctuation(next) { dest_buf.push(next); } else { dest_buf.push('\\'); dest_buf.push(next); }
            } else if c == '>' {
                scanner.skip(1); break;
            } else if c == '<' {
                return None;
            } else {
                dest_buf.push(c); scanner.skip(c.len_utf8());
            }
        }
        destination = normalize_url(&decode_entities(&dest_buf));
    } else {
        let mut depth = 0i32;
        let dest_start = scanner.pos;
        loop {
            if scanner.done() { break; }
            let c = scanner.peek_char(0);
            match c {
                '(' => { depth += 1; scanner.skip(1); }
                ')' => { if depth == 0 { break; } depth -= 1; scanner.skip(1); }
                '\\' => { scanner.skip(2); }
                c if is_ascii_whitespace(c) => break,
                c => { scanner.skip(c.len_utf8()); }
            }
        }
        let dest_raw = scanner.source[dest_start..scanner.pos].to_string();
        destination = normalize_url(&decode_entities(&apply_backslash_escapes_inline(&dest_raw)));
    }

    skip_optional_spaces_and_newline(scanner);

    let mut title: Option<String> = None;
    let q = scanner.peek_char(0);
    if q == '"' || q == '\'' || q == '(' {
        let close_q = if q == '(' { ')' } else { q };
        scanner.skip(q.len_utf8());
        let mut title_buf = String::new();
        loop {
            if scanner.done() { title = None; break; }
            let c = scanner.peek_char(0);
            if c == '\\' {
                scanner.skip(1);
                let next = scanner.advance();
                if is_ascii_punctuation(next) { title_buf.push(next); } else { title_buf.push('\\'); title_buf.push(next); }
            } else if c == close_q {
                scanner.skip(close_q.len_utf8());
                title = Some(decode_entities(&title_buf));
                break;
            } else if c == '\n' && q == '(' {
                title = None; break;
            } else {
                title_buf.push(c); scanner.skip(c.len_utf8());
            }
        }
    }

    scanner.skip_spaces();
    if !scanner.match_str(")") { return None; }
    Some(LinkResult { destination, title })
}

fn apply_backslash_escapes_inline(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                if is_ascii_punctuation(next) {
                    result.push(next); chars.next(); continue;
                }
            }
            result.push('\\');
        } else {
            result.push(ch);
        }
    }
    result
}

fn skip_optional_spaces_and_newline(scanner: &mut Scanner) {
    scanner.skip_spaces();
    if scanner.peek_char(0) == '\n' {
        scanner.skip(1);
        scanner.skip_spaces();
    } else if scanner.peek_char(0) == '\r' && scanner.peek_char(1) == '\n' {
        scanner.skip(2);
        scanner.skip_spaces();
    }
}

fn find_active_bracket_opener(bracket_stack: &[usize], tokens: &[Token]) -> i64 {
    for i in (0..bracket_stack.len()).rev() {
        let idx = bracket_stack[i];
        if let Some(Token::Bracket(ref bt)) = tokens.get(idx) {
            if bt.active {
                return i as i64;
            }
        }
    }
    -1
}

fn extract_plain_text(nodes: &[InlineNode]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            InlineNode::Text(t) => result.push_str(&t.value),
            InlineNode::CodeSpan(c) => result.push_str(&c.value),
            InlineNode::HardBreak(_) => result.push('\n'),
            InlineNode::SoftBreak(_) => result.push(' '),
            InlineNode::Emphasis(e) => result.push_str(&extract_plain_text(&e.children)),
            InlineNode::Strong(s) => result.push_str(&extract_plain_text(&s.children)),
            InlineNode::Strikethrough(s) => result.push_str(&extract_plain_text(&s.children)),
            InlineNode::Link(l) => result.push_str(&extract_plain_text(&l.children)),
            InlineNode::Image(img) => result.push_str(&img.alt),
            InlineNode::Autolink(a) => result.push_str(&a.destination),
            _ => {}
        }
    }
    result
}

// ─── Document-Level Inline Resolution ─────────────────────────────────────────

/// Walk the block AST from Phase 1 and fill in inline content.
///
/// Each `FinalBlock::Heading` and `FinalBlock::Paragraph` carries a raw content
/// string. This function parses those strings into inline nodes and produces
/// the final `DocumentNode`.
pub fn resolve_document(blocks: Vec<FinalBlock>, link_refs: &LinkRefMap) -> DocumentNode {
    DocumentNode {
        children: resolve_blocks(blocks, link_refs),
    }
}

fn resolve_blocks(blocks: Vec<FinalBlock>, link_refs: &LinkRefMap) -> Vec<BlockNode> {
    blocks.into_iter().filter_map(|b| resolve_block(b, link_refs)).collect()
}

fn resolve_list_item(children: Vec<FinalBlock>, link_refs: &LinkRefMap) -> ListChildNode {
    if let Some((checked, adjusted_children)) = extract_task_item(children) {
        let c = resolve_blocks(adjusted_children, link_refs);
        ListChildNode::TaskItem(TaskItemNode { checked, children: c })
    } else {
        let c = resolve_blocks(children, link_refs);
        ListChildNode::ListItem(ListItemNode { children: c })
    }
}

fn extract_task_item(children: Vec<FinalBlock>) -> Option<(bool, Vec<FinalBlock>)> {
    let mut iter = children.into_iter();
    let first = iter.next()?;
    match first {
        FinalBlock::Paragraph { raw_content } => {
            let rest = raw_content.strip_prefix("[ ]").map(|s| (false, s))
                .or_else(|| raw_content.strip_prefix("[x]").map(|s| (true, s)))
                .or_else(|| raw_content.strip_prefix("[X]").map(|s| (true, s)))?;
            let suffix = rest.1;
            if !suffix.is_empty() && !suffix.starts_with(' ') && !suffix.starts_with('\t') {
                return None;
            }
            let mut next_children = vec![FinalBlock::Paragraph { raw_content: suffix.trim_start_matches([' ', '\t']).to_string() }];
            next_children.extend(iter);
            Some((rest.0, next_children))
        }
        other => {
            let mut out = vec![other];
            out.extend(iter);
            let _ = out;
            None
        }
    }
}

fn try_parse_table_block(raw: &str, link_refs: &LinkRefMap) -> Option<TableNode> {
    let lines: Vec<&str> = raw.split('\n').collect();
    if lines.len() < 2 {
        return None;
    }
    let header = split_table_row(lines[0])?;
    let delimiter = split_table_row(lines[1])?;
    if header.is_empty() || header.len() != delimiter.len() {
        return None;
    }
    let mut align = Vec::with_capacity(delimiter.len());
    for cell in &delimiter {
        let trimmed = cell.trim();
        if !(trimmed.starts_with(':') || trimmed.starts_with('-')) || trimmed.chars().filter(|c| *c == '-').count() < 3 {
            return None;
        }
        if !trimmed.chars().all(|c| c == ':' || c == '-') {
            return None;
        }
        let left = trimmed.starts_with(':');
        let right = trimmed.ends_with(':');
        align.push(match (left, right) {
            (true, true) => TableAlignment::Center,
            (true, false) => TableAlignment::Left,
            (false, true) => TableAlignment::Right,
            (false, false) => TableAlignment::None,
        });
    }

    let mut rows = Vec::new();
    for line in lines.iter().skip(2) {
        if line.trim().is_empty() {
            return None;
        }
        let cells = split_table_row(line)?;
        rows.push(normalize_table_row(cells, header.len()));
    }

    let make_row = |cells: Vec<String>, is_header: bool| TableRowNode {
        is_header,
        children: cells
            .into_iter()
            .map(|content| TableCellNode { children: parse_inline(&content, link_refs) })
            .collect(),
    };

    let mut children = vec![make_row(normalize_table_row(header, delimiter.len()), true)];
    children.extend(rows.into_iter().map(|row| make_row(row, false)));
    Some(TableNode { align, children })
}

fn split_table_row(line: &str) -> Option<Vec<String>> {
    if !line.contains('|') {
        return None;
    }
    let trimmed = line.trim();
    let had_outer_pipe = trimmed.starts_with('|') || trimmed.ends_with('|');
    let mut slice = trimmed;
    if let Some(rest) = slice.strip_prefix('|') {
        slice = rest;
    }
    if let Some(rest) = slice.strip_suffix('|') {
        slice = rest;
    }

    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    let mut pipe_count = 0;
    for ch in slice.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }
        if ch == '|' {
            pipe_count += 1;
            cells.push(current.trim().to_string());
            current.clear();
            continue;
        }
        current.push(ch);
    }
    cells.push(current.trim().to_string());
    if pipe_count == 0 && !had_outer_pipe {
        None
    } else {
        Some(cells)
    }
}

fn normalize_table_row(mut cells: Vec<String>, width: usize) -> Vec<String> {
    cells.truncate(width);
    while cells.len() < width {
        cells.push(String::new());
    }
    cells
}

fn resolve_block(block: FinalBlock, link_refs: &LinkRefMap) -> Option<BlockNode> {
    match block {
        FinalBlock::Heading { level, raw_content } => {
            let children = parse_inline(&raw_content, link_refs);
            Some(BlockNode::Heading(HeadingNode { level, children }))
        }
        FinalBlock::Paragraph { raw_content } => {
            if let Some(table) = try_parse_table_block(&raw_content, link_refs) {
                return Some(BlockNode::Table(table));
            }
            let children = parse_inline(&raw_content, link_refs);
            Some(BlockNode::Paragraph(ParagraphNode { children }))
        }
        FinalBlock::CodeBlock { language, value } => {
            Some(BlockNode::CodeBlock(CodeBlockNode { language, value }))
        }
        FinalBlock::HtmlBlock { value } => {
            Some(BlockNode::RawBlock(RawBlockNode { format: "html".to_string(), value }))
        }
        FinalBlock::ThematicBreak => Some(BlockNode::ThematicBreak(ThematicBreakNode)),
        FinalBlock::Blockquote { children } => {
            let c = resolve_blocks(children, link_refs);
            Some(BlockNode::Blockquote(BlockquoteNode { children: c }))
        }
        FinalBlock::List { ordered, start, tight, items } => {
            let list_items: Vec<ListChildNode> = items.into_iter().filter_map(|item| {
                if let FinalBlock::ListItem { children } = item {
                    Some(resolve_list_item(children, link_refs))
                } else { None }
            }).collect();
            Some(BlockNode::List(ListNode {
                ordered,
                start,
                tight,
                children: list_items,
            }))
        }
        FinalBlock::ListItem { children } => {
            let c = resolve_blocks(children, link_refs);
            Some(BlockNode::ListItem(ListItemNode { children: c }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn parse(s: &str) -> Vec<InlineNode> {
        parse_inline(s, &HashMap::new())
    }

    #[test]
    fn test_plain_text() {
        let nodes = parse("hello");
        assert_eq!(nodes.len(), 1);
        if let InlineNode::Text(t) = &nodes[0] {
            assert_eq!(t.value, "hello");
        }
    }

    #[test]
    fn test_emphasis() {
        let nodes = parse("*hello*");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(&nodes[0], InlineNode::Emphasis(_)));
    }

    #[test]
    fn test_strong() {
        let nodes = parse("**bold**");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(&nodes[0], InlineNode::Strong(_)));
    }

    #[test]
    fn test_code_span() {
        let nodes = parse("`code`");
        assert_eq!(nodes.len(), 1);
        if let InlineNode::CodeSpan(c) = &nodes[0] {
            assert_eq!(c.value, "code");
        }
    }

    #[test]
    fn test_soft_break() {
        let nodes = parse("hello\nworld");
        assert!(nodes.iter().any(|n| matches!(n, InlineNode::SoftBreak(_))));
    }

    #[test]
    fn test_hard_break() {
        let nodes = parse("hello  \nworld");
        assert!(nodes.iter().any(|n| matches!(n, InlineNode::HardBreak(_))));
    }

    #[test]
    fn test_backslash_escape() {
        let nodes = parse("\\*literal");
        if let InlineNode::Text(t) = &nodes[0] {
            assert!(t.value.starts_with('*'));
        }
    }

    #[test]
    fn test_entity_decode() {
        let nodes = parse("&amp;");
        if let InlineNode::Text(t) = &nodes[0] {
            assert_eq!(t.value, "&");
        }
    }
}
