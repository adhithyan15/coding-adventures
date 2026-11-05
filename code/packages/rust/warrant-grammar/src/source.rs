//! # A source buffer has two rulers
//!
//! CRLF is two bytes on disk and one LF for the lexer. Keeping both buffers and
//! one pair of line starts lets us translate a boundary without guessing from a
//! token's transformed value. This stage admits source; it does not parse it.
use std::borrow::Cow;
use std::fmt;
use std::mem::size_of;
use std::ops::Range;

const SOURCE_CEILING: usize = 1024 * 1024;
const OWNED_CEILING: usize = 64 * 1024 * 1024;
const WORK_CEILING: usize = 3 * SOURCE_CEILING;

/// Tool-owned source-stage limits. Callers can tighten, never raise, this profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLimits {
    pub max_source_bytes: usize,
    pub max_owned_bytes: usize,
    pub max_scan_work: usize,
}
impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: SOURCE_CEILING,
            max_owned_bytes: OWNED_CEILING,
            max_scan_work: WORK_CEILING,
        }
    }
}

/// Closed admission failures. No variant owns untrusted source or token text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceErrorKind {
    InvalidLimits,
    SourceBytes,
    Work,
    OwnedBytes,
    Utf8,
    Bom,
    Nul,
    BareCr,
    Allocation,
}

/// Errors expose only a stable class and optional original raw-byte interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError {
    kind: SourceErrorKind,
    byte_range: Option<Range<usize>>,
}
impl SourceError {
    fn budget(kind: SourceErrorKind) -> Self {
        Self {
            kind,
            byte_range: None,
        }
    }
    fn at(kind: SourceErrorKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            byte_range: Some(start..end),
        }
    }
    pub fn code(&self) -> &'static str {
        "W0001"
    }
    pub fn kind(&self) -> SourceErrorKind {
        self.kind
    }
    pub fn byte_range(&self) -> Option<Range<usize>> {
        self.byte_range.clone()
    }
}
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            SourceErrorKind::InvalidLimits => "invalid source limits",
            SourceErrorKind::SourceBytes => "source byte limit",
            SourceErrorKind::Work => "source scan work limit",
            SourceErrorKind::OwnedBytes => "source buffer capacity limit",
            SourceErrorKind::Utf8 => "invalid UTF-8",
            SourceErrorKind::Bom => "source BOM forbidden",
            SourceErrorKind::Nul => "source NUL forbidden",
            SourceErrorKind::BareCr => "bare CR forbidden",
            SourceErrorKind::Allocation => "source buffer allocation failed",
        };
        write!(f, "{}: {message}", self.code())
    }
}
impl std::error::Error for SourceError {}

/// The byte offset is zero-based; line and byte-column are one-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalPosition {
    pub byte_offset: usize,
    pub line: usize,
    pub byte_column: usize,
}

/// A half-open original range, constructed against this document's real buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalSpan {
    pub start: OriginalPosition,
    pub end: OriginalPosition,
}

struct LineStart {
    original: usize,
    normalized: usize,
}

/// Source-bound data, without a path/revision binding or a syntax verdict.
/// Debug deliberately omits both buffers; source access must be explicit.
pub struct SourceDocument<'src> {
    original: &'src str,
    normalized: Cow<'src, str>,
    lines: Vec<LineStart>,
    owned_bytes: usize,
}
impl fmt::Debug for SourceDocument<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceDocument").finish_non_exhaustive()
    }
}

fn capacity_bytes(lines: usize, string: usize) -> Result<usize, SourceError> {
    lines
        .checked_mul(size_of::<LineStart>())
        .and_then(|bytes| bytes.checked_add(string))
        .ok_or_else(|| SourceError::budget(SourceErrorKind::OwnedBytes))
}
fn within_capacity(bytes: usize, limits: SourceLimits) -> Result<(), SourceError> {
    if bytes > limits.max_owned_bytes {
        return Err(SourceError::budget(SourceErrorKind::OwnedBytes));
    }
    Ok(())
}

impl<'src> SourceDocument<'src> {
    /// Admit before scans, preflight before allocation, then construct privately.
    /// Three byte-scans are reserved even when LF input takes the borrowed path.
    pub fn admit(bytes: &'src [u8], limits: SourceLimits) -> Result<Self, SourceError> {
        if limits.max_source_bytes > SOURCE_CEILING
            || limits.max_owned_bytes > OWNED_CEILING
            || limits.max_scan_work > WORK_CEILING
        {
            return Err(SourceError::budget(SourceErrorKind::InvalidLimits));
        }
        if bytes.len() > limits.max_source_bytes {
            return Err(SourceError::budget(SourceErrorKind::SourceBytes));
        }
        let work = bytes
            .len()
            .checked_mul(3)
            .ok_or_else(|| SourceError::budget(SourceErrorKind::Work))?;
        if work > limits.max_scan_work {
            return Err(SourceError::budget(SourceErrorKind::Work));
        }
        let original = std::str::from_utf8(bytes).map_err(|error| {
            let start = error.valid_up_to();
            let end = error
                .error_len()
                .map_or(bytes.len(), |length| start + length);
            SourceError::at(SourceErrorKind::Utf8, start, end)
        })?;
        if bytes.starts_with(b"\xef\xbb\xbf") {
            return Err(SourceError::at(SourceErrorKind::Bom, 0, 3));
        }

        // This pass owns nothing. The admitted 1-MiB source ceiling also bounds
        // counters, but arithmetic remains checked at every reservation boundary.
        let (mut line_count, mut removed) = (1usize, 0usize);
        for (offset, &byte) in bytes.iter().enumerate() {
            match byte {
                0 => return Err(SourceError::at(SourceErrorKind::Nul, offset, offset + 1)),
                b'\r' if bytes.get(offset + 1) != Some(&b'\n') => {
                    return Err(SourceError::at(SourceErrorKind::BareCr, offset, offset + 1));
                }
                b'\r' => removed += 1,
                b'\n' => line_count += 1,
                _ => {}
            }
        }
        let normalized_len = bytes.len() - removed;
        let string_bytes = if removed == 0 { 0 } else { normalized_len };
        within_capacity(capacity_bytes(line_count, string_bytes)?, limits)?;

        let mut lines = Vec::new();
        lines
            .try_reserve_exact(line_count)
            .map_err(|_| SourceError::budget(SourceErrorKind::Allocation))?;
        within_capacity(capacity_bytes(lines.capacity(), string_bytes)?, limits)?;
        let mut rewritten = if removed == 0 {
            None
        } else {
            let mut text = String::new();
            text.try_reserve_exact(normalized_len)
                .map_err(|_| SourceError::budget(SourceErrorKind::Allocation))?;
            Some(text)
        };
        let owned_bytes = capacity_bytes(
            lines.capacity(),
            rewritten.as_ref().map_or(0, String::capacity),
        )?;
        within_capacity(owned_bytes, limits)?;

        // A pair (raw start, normalized start) is enough for the whole line.
        // At a collapsed LF's start we still use the preceding line; at its end
        // binary search selects the next line, incorporating the removed CR.
        lines.push(LineStart {
            original: 0,
            normalized: 0,
        });
        let (mut removed_so_far, mut segment_start) = (0usize, 0usize);
        for (offset, &byte) in bytes.iter().enumerate() {
            if byte == b'\r' {
                removed_so_far += 1;
                if let Some(text) = &mut rewritten {
                    // CR/LF are ASCII, hence these slices end on UTF-8 boundaries.
                    text.push_str(&original[segment_start..offset]);
                    text.push('\n');
                    segment_start = offset + 2;
                }
            } else if byte == b'\n' {
                lines.push(LineStart {
                    original: offset + 1,
                    normalized: offset + 1 - removed_so_far,
                });
            }
        }
        let normalized = if let Some(mut text) = rewritten {
            text.push_str(&original[segment_start..]);
            Cow::Owned(text)
        } else {
            Cow::Borrowed(original)
        };
        Ok(Self {
            original,
            normalized,
            lines,
            owned_bytes,
        })
    }

    pub fn original(&self) -> &'src str {
        self.original
    }
    pub fn normalized(&self) -> &str {
        &self.normalized
    }
    pub fn is_normalized_borrowed(&self) -> bool {
        matches!(self.normalized, Cow::Borrowed(_))
    }
    pub fn owned_buffer_bytes(&self) -> usize {
        self.owned_bytes
    }

    /// Validate both normalized boundaries before translating. No prefix scans,
    /// allocations or token-value searches are involved in this source relation.
    pub fn original_span(&self, start: usize, end: usize) -> Option<OriginalSpan> {
        if start > end
            || !self.normalized.is_char_boundary(start)
            || !self.normalized.is_char_boundary(end)
        {
            return None;
        }
        Some(OriginalSpan {
            start: self.position(start),
            end: self.position(end),
        })
    }

    fn position(&self, offset: usize) -> OriginalPosition {
        // There is always the initial (0,0) record, including an empty document.
        let index = self.lines.partition_point(|line| line.normalized <= offset) - 1;
        let line = &self.lines[index];
        let column_offset = offset - line.normalized;
        OriginalPosition {
            byte_offset: line.original + column_offset,
            line: index + 1,
            byte_column: column_offset + 1,
        }
    }
}
