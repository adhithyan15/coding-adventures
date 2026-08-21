//! # `doc` — a zero-third-party-dependency reader for legacy Word `.doc` files
//!
//! This crate reads a **Word 97–2003 binary document** (`.doc`, the format
//! specified by [MS-DOC]) and hands you back its **main-document text**.
//!
//! It is a *literate* implementation: read it top to bottom and you will learn
//! how Word actually stores text and how to reassemble it. The companion spec
//! `code/specs/DOC01-binary-reader.md` covers the same ground with diagrams.
//!
//! ## Why this is harder than it sounds
//!
//! A `.doc` is not a flat blob of characters. It is an **OLE2 Compound File**
//! (a "filesystem in a file"; our [`cfb`] crate opens it) whose text is
//! *scattered* across runs inside a stream called `WordDocument`. The order in
//! which those runs must be glued back together is recorded separately, in a
//! **piece table** (the `CLX`) that lives in a second stream (`0Table` or
//! `1Table`). To recover the logical document you replay that piece table.
//!
//! ```text
//!   .doc bytes ──cfb::open──▶  ┌───────────────────────────────────────┐
//!                              │ CFB container                         │
//!                              │  ├─ "WordDocument"  = FIB + text runs  │
//!                              │  └─ "1Table"/"0Table" = CLX/PlcPcd     │
//!                              └───────────────────────────────────────┘
//! ```
//!
//! ## The pipeline
//!
//! 1. Open the bytes with [`cfb`].
//! 2. Read `WordDocument`; parse a handful of **FIB** header fields (magic,
//!    which table stream, and where/how big the CLX is).
//! 3. Read the selected table stream; slice out the **CLX**.
//! 4. Walk the CLX parts, skipping property runs (`Prc`), until the piece table
//!    (`Pcdt` → `PlcPcd`).
//! 5. For each piece, decode its `FcCompressed` field to find its bytes in
//!    `WordDocument` and whether they are 8-bit or 16-bit; decode and append.
//!
//! ## Security
//!
//! Input is untrusted and attacker-controlled. `#![forbid(unsafe_code)]`, no
//! `unwrap`/`expect`/`panic!`, no panicking indexing, checked arithmetic on
//! every file-derived offset/length, and hard caps on decoded size and piece
//! count. A hostile file yields a typed [`DocError`], never a panic.
//!
//! ```
//! # use doc::open_doc;
//! # fn demo(bytes: &[u8]) -> Result<(), doc::DocError> {
//! let document = open_doc(bytes)?;
//! println!("{}", document.text());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

use std::fmt;

// A test-only fixture: a real CFB-wrapped `.doc` decoding to "Hello, DOC!".
#[cfg(test)]
mod fixture;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Constants: FIB field offsets, CLX tags, FcCompressed bits, and safety caps.
// ---------------------------------------------------------------------------

/// `wIdent` — the FIB magic number. A real Word binary always starts with this.
const WIDENT_MAGIC: u16 = 0xA5EC;

/// Byte offset of `wIdent` (`u16`) within the `WordDocument` stream.
const OFF_WIDENT: usize = 0x000;
/// Byte offset of the FibBase flag word (`u16`) holding `fWhichTblStm`.
const OFF_FLAGS: usize = 0x00A;
/// Byte offset of `fcClx` (`u32`) — where the CLX starts in the table stream.
const OFF_FCCLX: usize = 0x1A2;
/// Byte offset of `lcbClx` (`u32`) — the CLX's byte length.
const OFF_LCBCLX: usize = 0x1A6;

/// `fWhichTblStm` bit: when set, the table stream is `"1Table"`, else `"0Table"`.
const FWHICH_TBL_STM: u16 = 0x0200;

/// CLX part tag for a `Prc` (property run) — skip it.
const CLXT_PRC: u8 = 0x01;
/// CLX part tag for a `Pcdt` (the piece table) — the payload we want.
const CLXT_PCDT: u8 = 0x02;

/// `FcCompressed` bit 30: set ⇒ 8-bit (compressed) text; clear ⇒ 16-bit UTF-16.
const FC_COMPRESSED_BIT: u32 = 0x4000_0000;
/// `FcCompressed` low-30-bit mask — the packed byte offset lives here.
const FC_OFFSET_MASK: u32 = 0x3FFF_FFFF;

/// Cap on total decoded text so a lying piece table cannot exhaust memory.
const MAX_TEXT_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
/// Cap on the number of pieces, likewise defensive against a hostile `lcb`.
const MAX_PIECES: usize = 1_000_000;
/// Cap on the number of CLX parts (`Prc`s) walked before the `Pcdt`. A real
/// document has a tiny handful; this bounds worst-case work independent of the
/// attacker-controlled CLX length, so a giant run of empty `Prc` parts cannot
/// pin a CPU core.
const MAX_CLX_PARTS: usize = 100_000;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Everything that can go wrong reading a `.doc`.
#[derive(Debug)]
pub enum DocError {
    /// The underlying compound-file container was unreadable. Wraps the
    /// original [`cfb::CfbError`] as its [`source`](std::error::Error::source).
    Cfb(cfb::CfbError),
    /// No `WordDocument` stream, or its `wIdent` magic was not `0xA5EC`. This is
    /// a valid CFB, but not a Word binary document.
    NotWordDocument,
    /// The FIB selects a table stream (`0Table`/`1Table`) that is not present.
    NoTableStream,
    /// The CLX / `PlcPcd` piece table is internally inconsistent (bad length
    /// arithmetic, an unknown part tag, or a non-advancing part).
    MalformedPieceTable,
    /// A declared offset or length ran past the actual bytes available (the FIB,
    /// the CLX slice, or a piece pointer into `WordDocument`).
    Truncated,
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocError::Cfb(_) => write!(f, "underlying compound file could not be read"),
            DocError::NotWordDocument => {
                write!(f, "not a Word binary document (missing WordDocument stream or bad wIdent magic)")
            }
            DocError::NoTableStream => {
                write!(f, "the table stream selected by the FIB is missing")
            }
            DocError::MalformedPieceTable => write!(f, "malformed CLX / PlcPcd piece table"),
            DocError::Truncated => write!(f, "a declared offset or length ran past the available bytes"),
        }
    }
}

impl std::error::Error for DocError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DocError::Cfb(e) => Some(e),
            _ => None,
        }
    }
}

impl From<cfb::CfbError> for DocError {
    fn from(e: cfb::CfbError) -> Self {
        DocError::Cfb(e)
    }
}

// ---------------------------------------------------------------------------
// The reassembled document
// ---------------------------------------------------------------------------

/// A parsed `.doc`, holding its reassembled main-document text.
#[derive(Debug, Clone)]
pub struct Document {
    text: String,
}

impl Document {
    /// The main-document text, with all pieces glued together in order.
    pub fn text(&self) -> &str {
        &self.text
    }
}

// ---------------------------------------------------------------------------
// Little-endian reads — each bounds-checked, never panicking.
// ---------------------------------------------------------------------------

/// Read a little-endian `u16` at `off`, or `None` if it would run off the end.
fn read_u16_le(buf: &[u8], off: usize) -> Option<u16> {
    let end = off.checked_add(2)?;
    let bytes = buf.get(off..end)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Read a little-endian `u32` at `off`, or `None` if it would run off the end.
fn read_u32_le(buf: &[u8], off: usize) -> Option<u32> {
    let end = off.checked_add(4)?;
    let bytes = buf.get(off..end)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Open a legacy `.doc` byte buffer and extract its main-document text.
///
/// See the module docs for the full pipeline. Errors are all typed
/// ([`DocError`]); no input can cause a panic.
pub fn open_doc(bytes: &[u8]) -> Result<Document, DocError> {
    // 1. Open the OLE2 compound-file container.
    let cf = cfb::CompoundFile::open(bytes)?;

    // 2. The WordDocument stream holds the FIB and the text runs.
    let word_document = cf
        .read_stream("WordDocument")
        .ok_or(DocError::NotWordDocument)?;

    // 3. Verify the wIdent magic. A valid CFB that is not a Word doc lands here.
    let ident = read_u16_le(&word_document, OFF_WIDENT).ok_or(DocError::NotWordDocument)?;
    if ident != WIDENT_MAGIC {
        return Err(DocError::NotWordDocument);
    }

    // 4. fWhichTblStm selects which table stream carries the CLX.
    let flags = read_u16_le(&word_document, OFF_FLAGS).ok_or(DocError::Truncated)?;
    let table_name = if flags & FWHICH_TBL_STM != 0 {
        "1Table"
    } else {
        "0Table"
    };

    // 5. Read the selected table stream.
    let table = cf.read_stream(table_name).ok_or(DocError::NoTableStream)?;

    // 6. Where and how big is the CLX inside the table stream?
    let fc_clx = read_u32_le(&word_document, OFF_FCCLX).ok_or(DocError::Truncated)? as usize;
    let lcb_clx = read_u32_le(&word_document, OFF_LCBCLX).ok_or(DocError::Truncated)? as usize;

    // 7-9. Everything from here is pure piece-table logic — factored into a seam
    // that unit tests can exercise with synthetic buffers (no CFB required).
    let text = extract_text(&word_document, &table, fc_clx, lcb_clx)?;
    Ok(Document { text })
}

// ---------------------------------------------------------------------------
// The piece-table engine (the testable seam)
// ---------------------------------------------------------------------------

/// Given the raw `WordDocument` bytes, the raw `table` stream bytes, and the
/// FIB-declared CLX location (`fc_clx`, `lcb_clx`), reassemble the text.
///
/// This is deliberately independent of `cfb` so it can be unit-tested with
/// hand-built byte arrays.
fn extract_text(
    word_document: &[u8],
    table: &[u8],
    fc_clx: usize,
    lcb_clx: usize,
) -> Result<String, DocError> {
    // Slice the CLX out of the table stream — bounds-checked against the ACTUAL
    // returned bytes (which include CFB zero padding, so never trust lengths).
    let clx_end = fc_clx.checked_add(lcb_clx).ok_or(DocError::Truncated)?;
    let clx = table.get(fc_clx..clx_end).ok_or(DocError::Truncated)?;

    // Walk CLX parts until we find the Pcdt (the piece table).
    let plc_pcd = find_plc_pcd(clx)?;

    // Parse the PlcPcd and decode every piece.
    decode_pieces(word_document, plc_pcd)
}

/// Scan the CLX's sequence of parts and return the `PlcPcd` bytes from the
/// `Pcdt` part. `Prc` parts (property runs) are skipped. The loop always
/// advances or errors — it can never spin.
fn find_plc_pcd(clx: &[u8]) -> Result<&[u8], DocError> {
    let mut pos: usize = 0;
    // Bound the number of parts walked. `pos` already advances by >= 3 each
    // iteration, so the loop terminates; this cap additionally bounds the work
    // by a constant rather than by the attacker-controlled CLX length, so a
    // multi-hundred-MB run of empty `Prc` parts cannot become a CPU-bound DoS.
    for _ in 0..MAX_CLX_PARTS {
        // A part must have at least its one-byte tag.
        let &clxt = clx.get(pos).ok_or(DocError::MalformedPieceTable)?;
        match clxt {
            CLXT_PRC => {
                // Prc: tag + i16 cbGrpprl + cbGrpprl bytes to skip.
                let len_off = pos.checked_add(1).ok_or(DocError::MalformedPieceTable)?;
                let cb = read_u16_le(clx, len_off).ok_or(DocError::MalformedPieceTable)? as i16;
                // cbGrpprl is signed in the spec but a run length; negative is
                // nonsensical and would break progress — reject it.
                if cb < 0 {
                    return Err(DocError::MalformedPieceTable);
                }
                // Advance past tag(1) + length(2) + payload(cb). This is > 0, so
                // the loop is guaranteed to make progress.
                let next = pos
                    .checked_add(3)
                    .and_then(|p| p.checked_add(cb as usize))
                    .ok_or(DocError::MalformedPieceTable)?;
                // The skipped payload must actually fit inside the CLX.
                if next > clx.len() {
                    return Err(DocError::Truncated);
                }
                pos = next;
            }
            CLXT_PCDT => {
                // Pcdt: tag + u32 lcb + lcb bytes of PlcPcd.
                let len_off = pos.checked_add(1).ok_or(DocError::MalformedPieceTable)?;
                let lcb = read_u32_le(clx, len_off).ok_or(DocError::MalformedPieceTable)? as usize;
                let data_start = pos.checked_add(5).ok_or(DocError::MalformedPieceTable)?;
                let data_end = data_start.checked_add(lcb).ok_or(DocError::Truncated)?;
                return clx.get(data_start..data_end).ok_or(DocError::Truncated);
            }
            // Any other tag: we do not know how long this part is, so we cannot
            // safely advance. Reject rather than risk a non-terminating loop.
            _ => return Err(DocError::MalformedPieceTable),
        }
    }
    // Walked MAX_CLX_PARTS parts without reaching a Pcdt: treat as malformed.
    Err(DocError::MalformedPieceTable)
}

/// Parse a `PlcPcd` and decode each piece into the growing output string.
///
/// PlcPcd layout (length `lcb`):
///
/// ```text
///   ┌──────────────────────────────┬──────────────────────────────┐
///   │ CP array: (n+1) × u32         │ PCD array: n × 8 bytes        │
///   │ cp[0] cp[1] ... cp[n]         │ pcd[0] ... pcd[n-1]           │
///   └──────────────────────────────┴──────────────────────────────┘
///   lcb = (n+1)*4 + n*8  ⇒  n = (lcb - 4) / 12
/// ```
fn decode_pieces(word_document: &[u8], plc_pcd: &[u8]) -> Result<String, DocError> {
    let lcb = plc_pcd.len();

    // Solve for the piece count. Need lcb >= 4 (at least the two boundary CPs of
    // a single piece) and the remainder to divide evenly by 12.
    if lcb < 4 {
        return Err(DocError::MalformedPieceTable);
    }
    let rem = lcb - 4;
    if !rem.is_multiple_of(12) {
        return Err(DocError::MalformedPieceTable);
    }
    let n = rem / 12;
    if n > MAX_PIECES {
        return Err(DocError::MalformedPieceTable);
    }

    // The PCD array begins right after the (n+1) CPs.
    // pcd_base = (n + 1) * 4, all checked.
    let pcd_base = n
        .checked_add(1)
        .and_then(|c| c.checked_mul(4))
        .ok_or(DocError::MalformedPieceTable)?;

    let mut out = String::new();

    for i in 0..n {
        // cp[i] and cp[i+1] give this piece's character range.
        let cp_i = read_u32_le(plc_pcd, i.checked_mul(4).ok_or(DocError::MalformedPieceTable)?)
            .ok_or(DocError::MalformedPieceTable)?;
        let cp_next = read_u32_le(
            plc_pcd,
            i.checked_add(1)
                .and_then(|j| j.checked_mul(4))
                .ok_or(DocError::MalformedPieceTable)?,
        )
        .ok_or(DocError::MalformedPieceTable)?;

        // Character count for this piece. A decreasing CP is malformed.
        let count = cp_next
            .checked_sub(cp_i)
            .ok_or(DocError::MalformedPieceTable)? as usize;

        // The 8-byte PCD for piece i; FcCompressed is the u32 at offset +2.
        let pcd_off = pcd_base
            .checked_add(i.checked_mul(8).ok_or(DocError::MalformedPieceTable)?)
            .ok_or(DocError::MalformedPieceTable)?;
        let fc_off = pcd_off.checked_add(2).ok_or(DocError::MalformedPieceTable)?;
        let fc = read_u32_le(plc_pcd, fc_off).ok_or(DocError::MalformedPieceTable)?;

        decode_one_piece(word_document, fc, count, &mut out)?;

        // Enforce the total-size cap as we go.
        if out.len() > MAX_TEXT_BYTES {
            return Err(DocError::MalformedPieceTable);
        }
    }

    Ok(out)
}

/// Decode a single piece described by `fc` (`FcCompressed`) of `count`
/// characters, appending to `out`.
///
/// FcCompressed bit trick:
/// * `fCompressed = fc & 0x4000_0000` (bit 30). If set → 8-bit text at byte
///   offset `real/2`, 1 byte per char. Else → UTF-16LE at `real`, 2 bytes/char.
/// * `real = fc & 0x3FFF_FFFF` (low 30 bits).
fn decode_one_piece(
    word_document: &[u8],
    fc: u32,
    count: usize,
    out: &mut String,
) -> Result<(), DocError> {
    let compressed = fc & FC_COMPRESSED_BIT != 0;
    let real = (fc & FC_OFFSET_MASK) as usize;

    if compressed {
        // 8-bit: bytes at real/2, one Latin-1 byte per character.
        let start = real / 2;
        let end = start.checked_add(count).ok_or(DocError::Truncated)?;
        let slice = word_document.get(start..end).ok_or(DocError::Truncated)?;
        for &b in slice {
            // Latin-1: byte b is Unicode scalar U+00bb (always valid).
            out.push(b as char);
        }
    } else {
        // 16-bit: bytes at real, two little-endian bytes per character.
        let byte_len = count.checked_mul(2).ok_or(DocError::Truncated)?;
        let end = real.checked_add(byte_len).ok_or(DocError::Truncated)?;
        let slice = word_document.get(real..end).ok_or(DocError::Truncated)?;
        decode_utf16le(slice, out)?;
    }
    Ok(())
}

/// Decode a little-endian UTF-16 byte slice (whose length is a multiple of 2)
/// into `out`, handling surrogate pairs. Malformed/lone surrogates become the
/// Unicode replacement character rather than erroring — matches how Word
/// consumers treat stray code units, and never panics.
fn decode_utf16le(slice: &[u8], out: &mut String) -> Result<(), DocError> {
    // Build the u16 code-unit iterator from LE byte pairs.
    let units = slice
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes([c[0], c[1]]));
    for unit in char::decode_utf16(units) {
        match unit {
            Ok(c) => out.push(c),
            Err(_) => out.push('\u{FFFD}'),
        }
    }
    Ok(())
}
