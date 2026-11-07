//! # Exact bounded significant-layout capture (VER04b / P006b2)
//!
//! Warrant uses Python/Starlark style indentation to structure declaration
//! blocks. Standard grammar lexer range capture refuses indentation modes
//! because synthetic layout transitions (`INDENT`, `DEDENT`, `NEWLINE`) do not
//! correspond to positive-width raw source lexemes.
//!
//! This module captures significant layout directly over an admitted
//! [`SourceDocument`](crate::source::SourceDocument). Every synthetic token is
//! assigned an exact zero-width boundary position in normalized coordinates,
//! mapping via the source document's line index to an exact zero-width
//! [`OriginalSpan`](crate::source::OriginalSpan). No fictitious positive-width
//! spans are ever assigned to indentation whitespace or block transitions.

use std::fmt;
use std::ops::Range;

use crate::source::{OriginalSpan, SourceDocument};

const TOKEN_CEILING: usize = 1_048_576;
const DEFAULT_TOKEN_BUDGET: usize = 65_536;
const INDENT_DEPTH_CEILING: usize = 256;
const DEFAULT_INDENT_DEPTH: usize = 64;

/// Bounded limits for layout capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutLimits {
    pub max_tokens: usize,
    pub max_indent_depth: usize,
    pub max_scan_work: usize,
}

impl Default for LayoutLimits {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_TOKEN_BUDGET,
            max_indent_depth: DEFAULT_INDENT_DEPTH,
            max_scan_work: 3 * 1024 * 1024,
        }
    }
}

/// Closed layout failure kinds. Never contains raw source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutErrorKind {
    InvalidLimits,
    Work,
    TokenBudget,
    IndentDepth,
    TabInIndentation,
    InconsistentDedent,
    Allocation,
}

/// Layout error with stable code W0001 and optional source byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutError {
    kind: LayoutErrorKind,
    byte_range: Option<Range<usize>>,
}

impl LayoutError {
    pub fn budget(kind: LayoutErrorKind) -> Self {
        Self {
            kind,
            byte_range: None,
        }
    }

    pub fn at(kind: LayoutErrorKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            byte_range: Some(start..end),
        }
    }

    pub fn code(&self) -> &'static str {
        "W0001"
    }

    pub fn kind(&self) -> LayoutErrorKind {
        self.kind
    }

    pub fn byte_range(&self) -> Option<Range<usize>> {
        self.byte_range.clone()
    }
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            LayoutErrorKind::InvalidLimits => "invalid layout limits",
            LayoutErrorKind::Work => "layout scan work limit exceeded",
            LayoutErrorKind::TokenBudget => "token budget exceeded",
            LayoutErrorKind::IndentDepth => "indentation depth limit exceeded",
            LayoutErrorKind::TabInIndentation => "tabs are forbidden in indentation",
            LayoutErrorKind::InconsistentDedent => "inconsistent dedent level",
            LayoutErrorKind::Allocation => "buffer allocation failed",
        };
        write!(f, "{}: {message}", self.code())
    }
}

impl std::error::Error for LayoutError {}

/// Origin representation for layout tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSpan {
    /// A real token with a half-open normalized byte range `[start, end)`.
    Real { start: usize, end: usize },
    /// A synthetic layout token originating at a zero-width normalized boundary.
    Synthetic { at: usize },
}

/// The grammatical kind of a layout token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutTokenKind {
    Indent,
    Dedent,
    Newline,
    Word(String),
    Symbol(String),
    Number(String),
    Bytes(String),
    Text(String),
}

/// A captured token with its grammatical kind and exact origin span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutToken {
    pub kind: LayoutTokenKind,
    pub span: TokenSpan,
}

impl LayoutToken {
    /// Returns true if this token is synthetic (INDENT, DEDENT, or synthetic NEWLINE).
    pub fn is_synthetic(&self) -> bool {
        matches!(self.span, TokenSpan::Synthetic { .. })
    }

    /// Computes the original source span for this token.
    /// Synthetic tokens map to zero-width spans where start == end.
    pub fn original_span(&self, doc: &SourceDocument) -> Option<OriginalSpan> {
        match self.span {
            TokenSpan::Real { start, end } => doc.original_span(start, end),
            TokenSpan::Synthetic { at } => doc.original_span(at, at),
        }
    }
}

/// Captures significant-layout tokens from an admitted source document.
pub fn capture_layout(
    doc: &SourceDocument,
    limits: LayoutLimits,
) -> Result<Vec<LayoutToken>, LayoutError> {
    if limits.max_tokens == 0
        || limits.max_tokens > TOKEN_CEILING
        || limits.max_indent_depth == 0
        || limits.max_indent_depth > INDENT_DEPTH_CEILING
    {
        return Err(LayoutError::budget(LayoutErrorKind::InvalidLimits));
    }

    let source = doc.normalized();
    let bytes = source.as_bytes();
    let source_len = bytes.len();

    let mut work_charged = 0usize;
    let mut check_work = |amount: usize| -> Result<(), LayoutError> {
        work_charged = work_charged
            .checked_add(amount)
            .ok_or_else(|| LayoutError::budget(LayoutErrorKind::Work))?;
        if work_charged > limits.max_scan_work {
            return Err(LayoutError::budget(LayoutErrorKind::Work));
        }
        Ok(())
    };

    check_work(source_len)?;

    let mut tokens = Vec::new();
    let mut indent_stack = Vec::with_capacity(limits.max_indent_depth.min(16));
    indent_stack.push(0usize);

    let mut pos = 0usize;

    let push_token =
        |token: LayoutToken, tokens: &mut Vec<LayoutToken>| -> Result<(), LayoutError> {
            if tokens.len() >= limits.max_tokens {
                return Err(LayoutError::budget(LayoutErrorKind::TokenBudget));
            }
            tokens.push(token);
            Ok(())
        };

    while pos < source_len {
        check_work(1)?;

        let line_start = pos;
        let mut line_end = pos;
        while line_end < source_len && bytes[line_end] != b'\n' {
            line_end += 1;
        }

        let line_bytes = &bytes[line_start..line_end];
        let mut idx = 0;
        let mut indent_col = 0;

        // Check indentation whitespace.
        let mut saw_tab = false;
        let mut tab_pos = 0;
        while idx < line_bytes.len() && (line_bytes[idx] == b' ' || line_bytes[idx] == b'\t') {
            if line_bytes[idx] == b'\t' && !saw_tab {
                saw_tab = true;
                tab_pos = line_start + idx;
            }
            indent_col += 1;
            idx += 1;
        }

        // Determine if line is blank or comment-only.
        let is_blank = idx == line_bytes.len();
        let is_comment =
            idx + 1 < line_bytes.len() && line_bytes[idx] == b'-' && line_bytes[idx + 1] == b'-';

        if !is_blank && !is_comment {
            if saw_tab {
                let orig_start = doc.original_span(tab_pos, tab_pos + 1);
                let range = orig_start.map(|s| s.start.byte_offset..s.end.byte_offset);
                return Err(LayoutError {
                    kind: LayoutErrorKind::TabInIndentation,
                    byte_range: range,
                });
            }

            let current_indent = *indent_stack.last().unwrap();
            let first_token_pos = line_start + idx;

            if indent_col > current_indent {
                if indent_stack.len() >= limits.max_indent_depth {
                    return Err(LayoutError::budget(LayoutErrorKind::IndentDepth));
                }
                indent_stack.push(indent_col);
                push_token(
                    LayoutToken {
                        kind: LayoutTokenKind::Indent,
                        span: TokenSpan::Synthetic {
                            at: first_token_pos,
                        },
                    },
                    &mut tokens,
                )?;
            } else if indent_col < current_indent {
                if !tokens.is_empty()
                    && !matches!(
                        tokens.last().map(|t| &t.kind),
                        Some(LayoutTokenKind::Newline)
                    )
                {
                    push_token(
                        LayoutToken {
                            kind: LayoutTokenKind::Newline,
                            span: TokenSpan::Synthetic { at: line_start },
                        },
                        &mut tokens,
                    )?;
                }

                while let Some(&top) = indent_stack.last() {
                    if top == indent_col {
                        break;
                    }
                    if top < indent_col {
                        return Err(LayoutError::budget(LayoutErrorKind::InconsistentDedent));
                    }
                    indent_stack.pop();
                    push_token(
                        LayoutToken {
                            kind: LayoutTokenKind::Dedent,
                            span: TokenSpan::Synthetic {
                                at: first_token_pos,
                            },
                        },
                        &mut tokens,
                    )?;
                }
            } else if !tokens.is_empty()
                && !matches!(
                    tokens.last().map(|t| &t.kind),
                    Some(LayoutTokenKind::Newline)
                )
            {
                push_token(
                    LayoutToken {
                        kind: LayoutTokenKind::Newline,
                        span: TokenSpan::Synthetic { at: line_start },
                    },
                    &mut tokens,
                )?;
            }

            // Lex tokens across the rest of the line.
            while idx < line_bytes.len() {
                check_work(1)?;
                let c = line_bytes[idx];

                if c == b' ' || c == b'\t' {
                    idx += 1;
                    continue;
                }

                if idx + 1 < line_bytes.len() && c == b'-' && line_bytes[idx + 1] == b'-' {
                    // Rest of line is comment trivia.
                    break;
                }

                let tok_start = line_start + idx;

                if c == b'"' {
                    idx += 1;
                    let str_start = idx;
                    while idx < line_bytes.len() && line_bytes[idx] != b'"' {
                        if line_bytes[idx] == b'\\' && idx + 1 < line_bytes.len() {
                            idx += 2;
                        } else {
                            idx += 1;
                        }
                    }
                    let str_end = idx;
                    if idx < line_bytes.len() && line_bytes[idx] == b'"' {
                        idx += 1;
                    }
                    let text = source[line_start + str_start..line_start + str_end].to_string();
                    push_token(
                        LayoutToken {
                            kind: LayoutTokenKind::Text(text),
                            span: TokenSpan::Real {
                                start: tok_start,
                                end: line_start + idx,
                            },
                        },
                        &mut tokens,
                    )?;
                } else if c == b'0'
                    && idx + 1 < line_bytes.len()
                    && (line_bytes[idx + 1] == b'x' || line_bytes[idx + 1] == b'X')
                {
                    idx += 2;
                    let hex_start = idx;
                    while idx < line_bytes.len() && line_bytes[idx].is_ascii_hexdigit() {
                        idx += 1;
                    }
                    let hex = source[line_start + hex_start..line_start + idx].to_string();
                    push_token(
                        LayoutToken {
                            kind: LayoutTokenKind::Bytes(hex),
                            span: TokenSpan::Real {
                                start: tok_start,
                                end: line_start + idx,
                            },
                        },
                        &mut tokens,
                    )?;
                } else if c.is_ascii_digit() {
                    let num_start = idx;
                    while idx < line_bytes.len()
                        && (line_bytes[idx].is_ascii_digit() || line_bytes[idx] == b'.')
                    {
                        if line_bytes[idx] == b'.'
                            && idx + 1 < line_bytes.len()
                            && line_bytes[idx + 1] == b'.'
                        {
                            // Part of '..' operator.
                            break;
                        }
                        idx += 1;
                    }
                    let num = source[line_start + num_start..line_start + idx].to_string();
                    push_token(
                        LayoutToken {
                            kind: LayoutTokenKind::Number(num),
                            span: TokenSpan::Real {
                                start: tok_start,
                                end: line_start + idx,
                            },
                        },
                        &mut tokens,
                    )?;
                } else if c.is_ascii_alphabetic() || c == b'_' {
                    let word_start = idx;
                    while idx < line_bytes.len()
                        && (line_bytes[idx].is_ascii_alphanumeric() || line_bytes[idx] == b'_')
                    {
                        idx += 1;
                    }
                    let word = source[line_start + word_start..line_start + idx].to_string();
                    push_token(
                        LayoutToken {
                            kind: LayoutTokenKind::Word(word),
                            span: TokenSpan::Real {
                                start: tok_start,
                                end: line_start + idx,
                            },
                        },
                        &mut tokens,
                    )?;
                } else {
                    // Multi-char or single-char operator / symbol.
                    let sym_start = idx;
                    if idx + 1 < line_bytes.len() {
                        let two = &line_bytes[idx..idx + 2];
                        if matches!(
                            two,
                            b"->" | b"=>" | b"==" | b"!=" | b"<=" | b">=" | b".." | b"::"
                        ) {
                            idx += 2;
                            let sym = source[line_start + sym_start..line_start + idx].to_string();
                            push_token(
                                LayoutToken {
                                    kind: LayoutTokenKind::Symbol(sym),
                                    span: TokenSpan::Real {
                                        start: tok_start,
                                        end: line_start + idx,
                                    },
                                },
                                &mut tokens,
                            )?;
                            continue;
                        }
                    }

                    idx += 1;
                    let sym = source[line_start + sym_start..line_start + idx].to_string();
                    push_token(
                        LayoutToken {
                            kind: LayoutTokenKind::Symbol(sym),
                            span: TokenSpan::Real {
                                start: tok_start,
                                end: line_start + idx,
                            },
                        },
                        &mut tokens,
                    )?;
                }
            }
        }

        pos = line_end;
        if pos < source_len && bytes[pos] == b'\n' {
            pos += 1;
        }
    }

    // End-of-file closure: pop all remaining indent levels down to 0.
    while indent_stack.len() > 1 {
        indent_stack.pop();
        push_token(
            LayoutToken {
                kind: LayoutTokenKind::Dedent,
                span: TokenSpan::Synthetic { at: source_len },
            },
            &mut tokens,
        )?;
    }

    // Trailing newline if not already present.
    if !tokens.is_empty()
        && !matches!(
            tokens.last().map(|t| &t.kind),
            Some(LayoutTokenKind::Newline)
        )
    {
        push_token(
            LayoutToken {
                kind: LayoutTokenKind::Newline,
                span: TokenSpan::Synthetic { at: source_len },
            },
            &mut tokens,
        )?;
    }

    Ok(tokens)
}
