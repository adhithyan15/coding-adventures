//! # `ppt` — a from-scratch reader for legacy PowerPoint (`.ppt`) files
//!
//! A `.ppt` file (PowerPoint 97–2003) is **not** a zip of XML the way a modern
//! `.pptx` is. It is a **Compound File** — an "OLE2" container, a tiny
//! FAT-style filesystem living inside one file ([MS-CFB]) — whose streams hold
//! a tree of binary **records** ([MS-PPT]). To read the words on the slides we:
//!
//! 1. Open the outer container. This crate delegates that entirely to the
//!    [`cfb`] crate — we add **zero** new container parsing.
//! 2. Pull out the one stream that holds the presentation records; it is named
//!    exactly `"PowerPoint Document"`.
//! 3. Walk the [MS-PPT] record tree in that stream and collect the text atoms.
//!
//! ```no_run
//! # fn demo(bytes: &[u8]) -> Result<(), ppt::PptError> {
//! let deck = ppt::open_ppt(bytes)?;
//! for (i, slide) in deck.slides().iter().enumerate() {
//!     println!("--- slide {} ---\n{}", i + 1, slide.text());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## The [MS-PPT] record format
//!
//! Everything in the "PowerPoint Document" stream is a **record**. Records sit
//! back-to-back, and *container* records hold child records inside their body —
//! so the whole stream is a depth-first tree. Every record begins with an
//! 8-byte **RecordHeader** (all little-endian):
//!
//! ```text
//!  offset  size  field       meaning
//!  ------  ----  ----------  ------------------------------------------------
//!    0      u16  recVerInst  low 4 bits  = recVer   (0xF ⇒ CONTAINER)
//!                            high 12 bits = recInstance (a per-type sub-variant)
//!    2      u16  recType     what kind of record this is (table below)
//!    4      u32  recLen      length of the BODY that follows (excludes header)
//!  ------  ----  ----------
//!    8    recLen  body       child records (container) OR raw data (atom)
//! ```
//!
//! Bit layout of the first `u16` (call it `w`):
//!
//! ```text
//!    15                                 4   3        0
//!   +-------------------------------------+-----------+
//!   |            recInstance (12)          |  recVer(4)|
//!   +-------------------------------------+-----------+
//!
//!   recVer      = w & 0x000F      (0xF means: body is child records)
//!   recInstance = w >> 4          (unused here; documented for completeness)
//! ```
//!
//! **Container vs atom is decided solely by `recVer == 0xF`.** A container's
//! body is more records (we recurse); an atom's body is opaque data we
//! interpret by `recType`.
//!
//! ### Record types this reader acts on
//!
//! | recType  | name           | kind      | what we do                                    |
//! | -------- | -------------- | --------- | --------------------------------------------- |
//! | `0x03E8` | Document       | container | recurse (top-level wrapper around slides)     |
//! | `0x03EE` | Slide          | container | **start a new [`Slide`]; recurse for its text** |
//! | `0x0FA0` | TextCharsAtom  | atom      | body is UTF-16LE → one text run               |
//! | `0x0FA8` | TextBytesAtom  | atom      | body is one byte per char (U+0000..=U+00FF)   |
//! | other    | —              | either    | if container, recurse; if atom, ignore body   |
//!
//! Any other container is still recursed into: in a real file a slide's text
//! lives several containers deep (`PPDrawing` → `OfficeArtDgContainer` → … →
//! `TextBox`). We do not need to understand those wrappers — only to not stop
//! at them.
//!
//! ## Security posture — this is attacker-controlled input
//!
//! The bytes come from an untrusted file, so the parser must never panic, hang,
//! or overflow. `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/`panic!`, all
//! reads bounds-checked, arithmetic uses `checked_add`, recursion is depth-
//! capped, and total slides / text are size-capped. See the module constants
//! and `walk` for the specifics, and `PPT01-binary-reader.md` for rationale.

#![forbid(unsafe_code)]

use std::fmt;

// ---------------------------------------------------------------------------
// Record-type constants ([MS-PPT] §2.13.24 RecordType enumeration, subset).
// ---------------------------------------------------------------------------

/// `0x03E8` Document container — the top-level wrapper. We recurse into it.
const REC_DOCUMENT: u16 = 0x03E8;
/// `0x03EE` Slide container — one on-screen slide. Each becomes a [`Slide`].
const REC_SLIDE: u16 = 0x03EE;
/// `0x0FA0` TextCharsAtom — body is UTF-16LE text.
const REC_TEXT_CHARS: u16 = 0x0FA0;
/// `0x0FA8` TextBytesAtom — body is one byte per char (low byte of each UTF-16
/// unit); each byte `b` decodes to the Unicode scalar `U+00{b}`.
const REC_TEXT_BYTES: u16 = 0x0FA8;

/// A record is a *container* (its body is child records, so we recurse) exactly
/// when the low nibble of the first header `u16` — `recVer` — equals `0xF`.
const REC_VER_CONTAINER: u16 = 0x0F;

// ---------------------------------------------------------------------------
// Safety caps against hostile input.
// ---------------------------------------------------------------------------

/// The 8-byte RecordHeader is the smallest thing that can appear at a level.
const HEADER_LEN: usize = 8;

/// Maximum container nesting we will descend before stopping. Containers nest,
/// and a crafted chain of thousands of nested containers would overflow the
/// native stack — an *uncatchable* DoS. Real decks nest only a handful deep;
/// 64 is generous headroom while keeping the recursion bounded.
const MAX_DEPTH: usize = 64;

/// Upper bound on how many [`Slide`]s we will materialise. A hostile file could
/// otherwise claim millions of Slide containers to exhaust memory.
const MAX_SLIDES: usize = 100_000;

/// Upper bound on total decoded text bytes. Bounds allocation from a file that
/// packs enormous text atoms.
const MAX_TOTAL_TEXT_BYTES: usize = 64 * 1024 * 1024;

/// The CFB stream that holds the presentation record tree.
const DOCUMENT_STREAM: &str = "PowerPoint Document";

// ---------------------------------------------------------------------------
// Errors.
// ---------------------------------------------------------------------------

/// Everything that can go wrong reading a `.ppt`.
#[derive(Debug)]
pub enum PptError {
    /// The outer Compound File could not be read (not a CFB, truncated, bad
    /// FAT, …). Wraps the underlying [`cfb::CfbError`].
    Cfb(cfb::CfbError),
    /// The container opened fine but has no `"PowerPoint Document"` stream, so
    /// it is not a PowerPoint presentation (or not this kind of one).
    NoDocumentStream,
    /// A record header ran past the end of the available bytes — the stream is
    /// truncated or malformed. (The walk stops cleanly; this is reported when a
    /// declared length cannot be honoured.)
    Truncated,
}

impl fmt::Display for PptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PptError::Cfb(e) => write!(f, "compound-file error: {e}"),
            PptError::NoDocumentStream => {
                write!(f, "no \"PowerPoint Document\" stream (not a .ppt presentation)")
            }
            PptError::Truncated => write!(f, "record stream truncated or malformed"),
        }
    }
}

impl std::error::Error for PptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PptError::Cfb(e) => Some(e),
            _ => None,
        }
    }
}

/// Lets `?` turn a [`cfb::CfbError`] into a [`PptError`] automatically.
impl From<cfb::CfbError> for PptError {
    fn from(e: cfb::CfbError) -> Self {
        PptError::Cfb(e)
    }
}

// ---------------------------------------------------------------------------
// The model.
// ---------------------------------------------------------------------------

/// A whole presentation: its slides in document order.
#[derive(Debug, Clone, Default)]
pub struct Presentation {
    slides: Vec<Slide>,
}

impl Presentation {
    /// The slides, in the order they appear in the file.
    pub fn slides(&self) -> &[Slide] {
        &self.slides
    }

    /// How many slides the presentation has.
    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }
}

/// One slide's text, as a list of runs (one per TextChars/TextBytes atom found
/// inside the slide's container, in record order).
#[derive(Debug, Clone, Default)]
pub struct Slide {
    runs: Vec<String>,
}

impl Slide {
    /// All text on the slide, the runs joined by `'\n'` in record order.
    pub fn text(&self) -> String {
        self.runs.join("\n")
    }

    /// The individual text runs, each corresponding to one text atom.
    pub fn text_runs(&self) -> &[String] {
        &self.runs
    }
}

// ---------------------------------------------------------------------------
// Little-endian read helpers — all bounds-checked, never panicking.
// ---------------------------------------------------------------------------

/// Read a little-endian `u16` at `off`, or `None` if it would read past the
/// end. Uses `get(..)` (checked) rather than indexing (which would panic).
fn read_u16(buf: &[u8], off: usize) -> Option<u16> {
    let end = off.checked_add(2)?;
    let b = buf.get(off..end)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

/// Read a little-endian `u32` at `off`, or `None` if out of range.
fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    let end = off.checked_add(4)?;
    let b = buf.get(off..end)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

// ---------------------------------------------------------------------------
// Text-atom decoding.
// ---------------------------------------------------------------------------

/// Decode a **TextBytesAtom** body: one byte per character. Each byte `b` is the
/// low byte of a UTF-16 code unit, i.e. the Unicode scalar `U+00{b}`
/// (Latin-1). `char::from(u8)` is exactly that mapping and is total over
/// `0..=255`, so this never fails. A single trailing NUL (PowerPoint's
/// C-string terminator) is stripped.
fn decode_text_bytes(body: &[u8]) -> String {
    let body = strip_trailing_nul_byte(body);
    body.iter().map(|&b| char::from(b)).collect()
}

/// Decode a **TextCharsAtom** body: UTF-16LE. We read `u16` code units in pairs;
/// a trailing odd byte (malformed) is ignored. Unpaired surrogates become
/// U+FFFD rather than an error — hostile input must never make us panic. A
/// single trailing NUL code unit is stripped.
fn decode_text_chars(body: &[u8]) -> String {
    // `chunks_exact(2)` yields only whole pairs and silently drops a lone
    // trailing byte, which is exactly the lenient behaviour we want.
    let mut units: Vec<u16> = body
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| u16::from_le_bytes(*chunk))
        .collect();
    if units.last() == Some(&0) {
        units.pop();
    }
    // `decode_utf16` replaces unpaired surrogates with the caller-chosen char.
    char::decode_utf16(units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

/// Drop a single trailing `0x00`, if present (the C-string terminator).
fn strip_trailing_nul_byte(body: &[u8]) -> &[u8] {
    match body.split_last() {
        Some((0, head)) => head,
        _ => body,
    }
}

// ---------------------------------------------------------------------------
// The record walker.
// ---------------------------------------------------------------------------

/// Public entry point: open `bytes` as a `.ppt` and extract per-slide text.
///
/// Fails with [`PptError::Cfb`] if the bytes are not a valid Compound File, or
/// [`PptError::NoDocumentStream`] if they are a CFB with no "PowerPoint
/// Document" stream.
pub fn open_ppt(bytes: &[u8]) -> Result<Presentation, PptError> {
    let cf = cfb::CompoundFile::open(bytes)?;
    let stream = cf
        .read_stream(DOCUMENT_STREAM)
        .ok_or(PptError::NoDocumentStream)?;
    parse_document_stream(&stream)
}

/// Parse a raw "PowerPoint Document" record stream into a [`Presentation`].
///
/// Factored out from [`open_ppt`] so tests can feed synthetic record streams
/// without building a whole CFB around them.
///
/// The strategy (matching real-world `.ppt` text tooling like Apache POI and
/// `catppt`): walk the record tree; **each `0x03EE` Slide container becomes one
/// [`Slide`]**, and every text atom found anywhere inside it (recursing through
/// arbitrary intermediate containers) is one of its runs, in record order. Text
/// atoms outside any Slide container are ignored.
pub fn parse_document_stream(stream: &[u8]) -> Result<Presentation, PptError> {
    let mut deck = Presentation::default();
    // `budget` tracks total decoded text bytes so far, threaded through the walk
    // so we can stop before a hostile file makes us allocate too much.
    let mut budget: usize = 0;
    // Top level: no enclosing slide yet, depth 0.
    walk(stream, 0, &mut deck, None, &mut budget);
    Ok(deck)
}

/// Recursively walk one level of records within `buf`.
///
/// `current_slide` is `Some(index)` when we are *inside* a Slide container
/// (text atoms attach to `deck.slides[index]`), or `None` at/above the slide
/// level (text is ignored). `depth` guards against stack-overflow DoS.
///
/// This function is **infallible by construction**: on any malformation
/// (truncated header, over-long `recLen`, padding, depth/size cap) it simply
/// stops walking the current level. That is the safe behaviour for
/// attacker-controlled input — we return whatever well-formed text we managed
/// to read rather than propagating an error mid-tree.
fn walk(
    buf: &[u8],
    depth: usize,
    deck: &mut Presentation,
    current_slide: Option<usize>,
    budget: &mut usize,
) {
    // Depth cap: refuse to descend further. Returning cleanly (rather than
    // recursing) is what keeps a deeply-nested hostile file from smashing the
    // native stack.
    if depth >= MAX_DEPTH {
        return;
    }

    let mut off: usize = 0;
    loop {
        // --- Stop conditions -------------------------------------------------
        // Fewer than a full header remain: end of real data (or trailing
        // fragment). Stop this level cleanly.
        let Some(header_end) = off.checked_add(HEADER_LEN) else {
            return;
        };
        if header_end > buf.len() {
            return;
        }

        // Parse the 8-byte RecordHeader. These reads cannot fail given the
        // bounds check above, but we still go through the checked helpers.
        let (Some(ver_inst), Some(rec_type), Some(rec_len_u32)) =
            (read_u16(buf, off), read_u16(buf, off + 2), read_u32(buf, off + 4))
        else {
            return;
        };

        // Trailing zero padding: the CFB pads streams up to a sector/mini
        // boundary with zeros. A recType==0 && recLen==0 header is that padding
        // (never a real record) — stop the level rather than looping on zeros.
        if rec_type == 0 && rec_len_u32 == 0 {
            return;
        }

        let rec_len = rec_len_u32 as usize;
        let rec_ver = ver_inst & 0x000F;

        // Body bounds: the body must fit within `buf`. A `recLen` that runs past
        // the buffer is malformed — stop cleanly (no panic, no partial slice).
        let Some(body_end) = header_end.checked_add(rec_len) else {
            return;
        };
        if body_end > buf.len() {
            return;
        }
        // `get(..)` here is guaranteed Some by the check above; default to empty
        // rather than ever unwrapping.
        let body = buf.get(header_end..body_end).unwrap_or(&[]);

        // --- Dispatch on record kind ----------------------------------------
        if rec_ver == REC_VER_CONTAINER {
            // Container: its body is child records. What we do depends on type.
            match rec_type {
                REC_SLIDE => {
                    // Start a new slide (respecting the slide cap), then recurse
                    // with that slide as the attachment target.
                    if deck.slides.len() < MAX_SLIDES {
                        deck.slides.push(Slide::default());
                        let idx = deck.slides.len() - 1;
                        walk(body, depth + 1, deck, Some(idx), budget);
                    }
                    // If we're at the cap, we simply don't descend — bounded.
                }
                // Document wrapper or any other container: recurse, keeping the
                // same enclosing-slide context (usually None at the top).
                _ => walk(body, depth + 1, deck, current_slide, budget),
            }
        } else {
            // Atom: opaque data. Only decode text atoms, and only when we are
            // inside a slide AND still under the text budget. Checking the
            // budget *before* decoding matters for hostile input: it bounds the
            // transient decode work (not just retained text), so a deeply-nested
            // deck packed with huge text atoms cannot make us decode gigabytes
            // after the cap is already reached — once `budget` hits the cap we
            // stop decoding entirely.
            if let Some(idx) = current_slide {
                if *budget < MAX_TOTAL_TEXT_BYTES {
                    let decoded = match rec_type {
                        REC_TEXT_BYTES => Some(decode_text_bytes(body)),
                        REC_TEXT_CHARS => Some(decode_text_chars(body)),
                        _ => None,
                    };
                    if let Some(text) = decoded {
                        // Enforce the total-text cap before storing. `saturating_add`
                        // means at most one record can straddle the cap.
                        let next = budget.saturating_add(text.len());
                        if next <= MAX_TOTAL_TEXT_BYTES {
                            *budget = next;
                            if let Some(slide) = deck.slides.get_mut(idx) {
                                slide.runs.push(text);
                            }
                        }
                        // Over cap: silently drop further text (bounded allocation).
                    }
                }
            }
            // `REC_DOCUMENT` is documented as a container; if it ever appears as
            // an atom (malformed), we correctly ignore its body here.
            let _ = REC_DOCUMENT;
        }

        // --- Advance ---------------------------------------------------------
        // The cursor always moves forward by at least the header (8) plus the
        // body, so the loop is guaranteed to terminate — no in-place spin, no
        // hang. `body_end` is already `off + 8 + rec_len`.
        off = body_end;
    }
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;
