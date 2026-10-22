//! Exact ranges describe the inspected buffer, while Token values describe grammar
//! output. A quoted/escaped/case-normalized value cannot serve as a byte ruler.
//! This module keeps that distinction explicit without changing existing Tokens.
use crate::token::{LexerError, Token};
use std::fmt;
use std::mem::size_of;

/// Independent zero-based source coordinates; UTF16 counts astral scalars twice.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    pub byte_offset: usize,
    pub scalar_offset: usize,
    pub utf16_offset: usize,
    pub line: usize,
    pub scalar_column: usize,
    pub utf16_column: usize,
}

/// Half-open UTF8 range. Detached public values require domain validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

/// Token ordinals address the unmodified lexer stream, including terminal EOF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexicalPieceKind {
    Token(usize),
    Trivia,
    Eof(usize),
}

/// One actual accepted match, skip, fallback character or EOF point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexicalPiece {
    pub kind: LexicalPieceKind,
    pub range: SourceRange,
}

/// Bounds concern additional descriptors/cursor work, not existing regex cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexicalLimits {
    pub max_source_bytes: usize,
    pub max_pieces: usize,
    pub max_retained_bytes: usize,
    pub max_work: usize,
}
impl Default for LexicalLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 8 * 1024 * 1024,
            max_pieces: 262_144,
            max_retained_bytes: 64 * 1024 * 1024,
            max_work: 32 * 1024 * 1024,
        }
    }
}
impl LexicalLimits {
    /// Check before the source character vector or extra descriptors are owned.
    pub(crate) fn admit_source(self, bytes: usize) -> Result<(), LexicalError> {
        if self.max_source_bytes > 128 * 1024 * 1024
            || self.max_pieces > 1_048_576
            || self.max_retained_bytes > 256 * 1024 * 1024
            || self.max_work > 512 * 1024 * 1024
        {
            return Err(LexicalError::Limit("configuration ceiling"));
        }
        if bytes > self.max_source_bytes {
            return Err(LexicalError::Limit("source bytes"));
        }
        if bytes > self.max_work {
            return Err(LexicalError::Limit("coordinate work"));
        }
        Ok(())
    }
    fn piece_capacity(self) -> usize {
        self.max_pieces
            .min(self.max_retained_bytes / size_of::<LexicalPiece>())
    }
}

/// Capture rejection is distinct from an ordinary malformed-source lexer error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalError {
    Unsupported(&'static str),
    Limit(&'static str),
    Invalid(&'static str),
    Allocation,
    Lex(LexerError),
}
impl fmt::Display for LexicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(s) => write!(f, "Lexical capture unsupported: {s}"),
            Self::Limit(s) => write!(f, "Lexical capture limit: {s}"),
            Self::Invalid(s) => write!(f, "Invalid lexical partition: {s}"),
            Self::Allocation => write!(f, "Lexical descriptor allocation failed"),
            Self::Lex(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for LexicalError {}
impl From<LexerError> for LexicalError {
    fn from(value: LexerError) -> Self {
        Self::Lex(value)
    }
}
impl LexicalError {
    /// Standard legacy tokenization cannot reach capture-specific errors; retain
    /// its original malformed-source error and positional fields unchanged.
    pub(crate) fn into_lexer_error(self) -> LexerError {
        match self {
            Self::Lex(e) => e,
            other => LexerError {
                message: other.to_string(),
                line: 1,
                column: 1,
            },
        }
    }
}

/// Only actual successful capture constructs this source-bound, read-only result.
/// It proves matcher coverage; detached descriptors prove structural coverage only.
#[derive(Debug)]
pub struct LexicalSnapshot<'src> {
    pub(crate) source: &'src str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) pieces: Vec<LexicalPiece>,
}
impl<'src> LexicalSnapshot<'src> {
    pub fn source(&self) -> &'src str {
        self.source
    }
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
    pub fn pieces(&self) -> &[LexicalPiece] {
        &self.pieces
    }
    pub fn into_tokens(self) -> Vec<Token> {
        self.tokens
    }
}

/// This cursor is deliberately separate from the legacy LF-only lexer location.
/// CR consumes a line immediately; an immediately following LF consumes offsets
/// but not another line. This rule remains correct across adjacent piece boundaries.
#[derive(Default)]
pub(crate) struct CoordinateCursor {
    pub(crate) position: SourcePosition,
    previous_cr: bool,
}
impl CoordinateCursor {
    pub(crate) fn advance(&mut self, ch: char) {
        // Only admitted source characters advance this private cursor. All counters
        // are bounded by 128 MiB of UTF8 source, well below usize even on 32-bit.
        self.position.byte_offset += ch.len_utf8();
        self.position.scalar_offset += 1;
        self.position.utf16_offset += ch.len_utf16();
        if ch == '\n' && self.previous_cr {
            // CRLF is one line terminator, even when captured as two pieces.
        } else if matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
            self.position.line += 1;
            self.position.scalar_column = 0;
            self.position.utf16_column = 0;
        } else {
            self.position.scalar_column += 1;
            self.position.utf16_column += ch.len_utf16();
        }
        self.previous_cr = ch == '\r';
    }
}

pub(crate) struct RangeRecorder {
    pub(crate) cursor: CoordinateCursor,
    pub(crate) pieces: Vec<LexicalPiece>,
    limits: LexicalLimits,
    work: usize,
}
impl RangeRecorder {
    pub(crate) fn new(bytes: usize, limits: LexicalLimits) -> Result<Self, LexicalError> {
        limits.admit_source(bytes)?;
        Ok(Self {
            cursor: CoordinateCursor::default(),
            pieces: Vec::new(),
            limits,
            work: bytes,
        })
    }
    /// Reserve count, work and geometric capacity BEFORE a matched token String
    /// or descriptor is owned. Capacity accounting prevents hidden vector slack
    /// from escaping a tiny descriptor-byte cap. Allocator bookkeeping is separate.
    pub(crate) fn prepare(&mut self) -> Result<SourcePosition, LexicalError> {
        let needed = self
            .pieces
            .len()
            .checked_add(1)
            .ok_or(LexicalError::Limit("piece count"))?;
        if needed > self.limits.max_pieces {
            return Err(LexicalError::Limit("piece count"));
        }
        let capacity_limit = self.limits.piece_capacity();
        if needed > capacity_limit {
            return Err(LexicalError::Limit("retained descriptor bytes"));
        }
        let work = self
            .work
            .checked_add(1)
            .ok_or(LexicalError::Limit("coordinate work"))?;
        if work > self.limits.max_work {
            return Err(LexicalError::Limit("coordinate work"));
        }
        if needed > self.pieces.capacity() {
            let capacity = self
                .pieces
                .capacity()
                .checked_mul(2)
                .unwrap_or(capacity_limit)
                .max(needed)
                .min(capacity_limit);
            self.pieces
                .try_reserve_exact(capacity - self.pieces.len())
                .map_err(|_| LexicalError::Allocation)?;
            // Vec reports allocated element capacity. Refuse an allocator-provided
            // capacity exceeding the admitted logical backing-store bound.
            if self.pieces.capacity() > capacity_limit {
                return Err(LexicalError::Limit("retained descriptor bytes"));
            }
        }
        self.work = work;
        Ok(self.cursor.position)
    }
    pub(crate) fn finish(&mut self, start: SourcePosition, kind: LexicalPieceKind) {
        self.pieces.push(LexicalPiece {
            kind,
            range: SourceRange {
                start,
                end: self.cursor.position,
            },
        });
    }
}

/// Validate coordinates/partition/ordinals against supplied source in one forward
/// walk. This does not authenticate bytes or recognize grammar Token/Trivia kinds.
/// Callers must separately bind content identity and actual-matcher evidence.
pub fn validate_lexical_partition(
    source: &str,
    token_count: usize,
    pieces: &[LexicalPiece],
    limits: LexicalLimits,
) -> Result<(), LexicalError> {
    limits.admit_source(source.len())?;
    if pieces.len() > limits.max_pieces {
        return Err(LexicalError::Limit("piece count"));
    }
    let retained = pieces
        .len()
        .checked_mul(size_of::<LexicalPiece>())
        .ok_or(LexicalError::Limit("retained descriptor bytes"))?;
    if retained > limits.max_retained_bytes {
        return Err(LexicalError::Limit("retained descriptor bytes"));
    }
    let work = source
        .len()
        .checked_add(pieces.len())
        .ok_or(LexicalError::Limit("coordinate work"))?;
    if work > limits.max_work {
        return Err(LexicalError::Limit("coordinate work"));
    }
    if token_count == 0 || token_count > pieces.len() {
        return Err(LexicalError::Invalid("token count"));
    }
    let mut cursor = CoordinateCursor::default();
    let mut next_token = 0usize;
    let mut saw_eof = false;
    for (ordinal, piece) in pieces.iter().enumerate() {
        let range = piece.range;
        if range.start != cursor.position {
            return Err(LexicalError::Invalid(
                "start coordinate or byte gap/overlap",
            ));
        }
        let end = range.end.byte_offset;
        if end < range.start.byte_offset || end > source.len() || !source.is_char_boundary(end) {
            return Err(LexicalError::Invalid("end byte boundary"));
        }
        match piece.kind {
            LexicalPieceKind::Token(index) => {
                if index != next_token || index >= token_count - 1 {
                    return Err(LexicalError::Invalid("token ordinal"));
                }
                if end == range.start.byte_offset {
                    return Err(LexicalError::Invalid("empty token"));
                }
                next_token += 1;
            }
            LexicalPieceKind::Trivia => {
                if end == range.start.byte_offset {
                    return Err(LexicalError::Invalid("empty trivia"));
                }
            }
            LexicalPieceKind::Eof(index) => {
                if saw_eof
                    || ordinal != pieces.len() - 1
                    || index != next_token
                    || index != token_count - 1
                    || end != source.len()
                    || range.start.byte_offset != end
                {
                    return Err(LexicalError::Invalid("terminal EOF"));
                }
                next_token += 1;
                saw_eof = true;
            }
        }
        for ch in source[range.start.byte_offset..end].chars() {
            cursor.advance(ch);
        }
        if range.end != cursor.position {
            return Err(LexicalError::Invalid("end coordinate"));
        }
    }
    if !saw_eof || cursor.position.byte_offset != source.len() || next_token != token_count {
        return Err(LexicalError::Invalid("incomplete coverage or EOF"));
    }
    Ok(())
}
