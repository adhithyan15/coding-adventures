//! # doc-writer — Legacy `.doc` (Word 97-2003 binary) writer (DOCW01)
//!
//! A from-scratch, zero-third-party-dependency **writer** for the legacy
//! **`.doc`** format ([MS-DOC]) — the Word 97-2003 document. You give it a
//! simple [`Document`] (paragraphs of text); it emits a valid `.doc` byte buffer
//! by writing the **FIB** (File Information Block), a **piece table** (CLX), and
//! the text, all wrapped in an **OLE2 / Compound File Binary** container via the
//! sibling [`cfb-writer`] crate. See `code/specs/DOCW01-doc-writer.md` for the
//! full literate walkthrough.
//!
//! ## The one-paragraph mental model
//!
//! A `.doc` is a CFB container (see the `cfb`/`cfb-writer` crates — "a FAT
//! filesystem crammed into a single file") holding two streams:
//!
//! - **`WordDocument`** — starts with the **FIB**, a big fixed-layout header of
//!   offsets and lengths. The document *characters* also live here, but not at a
//!   fixed place: you follow a pointer to find them.
//! - **`1Table`** — holds the **CLX**, whose **piece table** (`PlcPcd`) maps
//!   *character positions* to *byte offsets* in `WordDocument`, and records
//!   whether each run is stored 8-bit (Latin-1) or 16-bit (UTF-16LE).
//!
//! The reader's retrieval path — which our round-trip test re-implements — is:
//!
//! ```text
//!   FIB (in WordDocument)  ── fcClx/lcbClx ──▶  CLX (in 1Table)
//!                                                   │  PlcPcd (piece table)
//!                                                   ▼
//!                                    for each piece: FcCompressed
//!                          bit 30 set → 8-bit text at (fc & 0x3FFFFFFF)/2
//!                          bit 30 clear → 16-bit text at fc
//! ```
//!
//! We always emit **one piece** covering the whole document. That is the whole
//! trick: build the FIB, build a one-piece piece table, wrap in CFB.
//!
//! ```
//! # use doc_writer::{Document, write_doc};
//! let mut doc = Document::new();
//! doc.add_paragraph("Hello, DOC!");
//! let bytes = write_doc(&doc);
//! // Opens with the CFB signature.
//! assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
//! ```

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Constants — the FIB field offsets/values and the piece-table encoding. These
// mirror exactly what a [MS-DOC] reader looks for; a writer and reader must
// agree byte-for-byte.
// ---------------------------------------------------------------------------

/// Fixed location, past the FIB, where we store the document text inside the
/// `WordDocument` stream. Any offset comfortably beyond the FIB fields we set
/// works; 2048 is a round, spec-safe choice.
const TEXT_OFFSET: usize = 2048;

/// `wIdent` — the magic that identifies a Word FIB (u16 @ 0).
const W_IDENT: u16 = 0xA5EC;
/// `nFib` — FIB version. Word 97 = 193 = `0x00C1` (u16 @ 2).
const N_FIB_WORD97: u16 = 0x00C1;
/// FibBase flags (u16 @ 10). Bit 9 (`0x0200`) is `fWhichTblStm`: 1 → the table
/// stream is named `1Table` (rather than `0Table`).
const FIB_FLAGS_1TABLE: u16 = 0x0200;
/// `csw` — count of 16-bit values in `fibRgW` = 14 (u16 @ 32).
const CSW: u16 = 0x000E;
/// `cslw` — count of 32-bit values in `fibRgLw` = 22 (u16 @ 62).
const CSLW: u16 = 0x0016;
/// `cbRgFcLcb` — count of FC/LCB pairs in the FIB blob = 93 (u16 @ 152).
const CB_RG_FC_LCB: u16 = 0x005D;

/// Byte offset within `WordDocument` of the `fcClx` field (u32).
const OFF_FC_CLX: usize = 0x1A2; // 418
/// Byte offset within `WordDocument` of the `lcbClx` field (u32).
const OFF_LCB_CLX: usize = 0x1A6; // 422
/// Byte offset within `WordDocument` of `ccpText` (u32, informational).
const OFF_CCP_TEXT: usize = 0x4C; // 76

/// `clxt` value marking a CLX component as a `Pcdt` (piece-table container).
const CLXT_PCDT: u8 = 0x02;

/// The `fCompressed` flag (bit 30) inside an `FcCompressed`. Set → the text is
/// 8-bit (one Latin-1 byte per char) and the stored offset is doubled.
const FC_COMPRESSED_FLAG: u32 = 0x4000_0000;
/// Mask recovering the offset payload from an `FcCompressed` (low 30 bits).
const FC_OFFSET_MASK: u32 = 0x3FFF_FFFF;

/// The CFB stream name for the FIB + text.
const STREAM_WORD_DOCUMENT: &str = "WordDocument";
/// The CFB stream name for the table (CLX). We always emit `1Table` (see
/// [`FIB_FLAGS_1TABLE`]).
const STREAM_1TABLE: &str = "1Table";

// ---------------------------------------------------------------------------
// Little-endian writers. Small helpers that patch a pre-sized buffer. None can
// panic on the public path because every buffer is sized to a computed length
// and every offset here is a compile-time constant within that length.
// ---------------------------------------------------------------------------

/// Write a `u16` little-endian at `off`, if it fits. Returns whether it fit.
#[inline]
fn put_u16(buf: &mut [u8], off: usize, v: u16) -> bool {
    match buf.get_mut(off..off + 2) {
        Some(slot) => {
            slot.copy_from_slice(&v.to_le_bytes());
            true
        }
        None => false,
    }
}

/// Write a `u32` little-endian at `off`, if it fits. Returns whether it fit.
#[inline]
fn put_u32(buf: &mut [u8], off: usize, v: u32) -> bool {
    match buf.get_mut(off..off + 4) {
        Some(slot) => {
            slot.copy_from_slice(&v.to_le_bytes());
            true
        }
        None => false,
    }
}

// ---------------------------------------------------------------------------
// The public document model.
// ---------------------------------------------------------------------------

/// A minimal document model: an ordered list of paragraphs. The emitted `.doc`
/// stores the paragraphs joined by Word's paragraph mark, the carriage return
/// `\r` (`0x0D`), as a single run of text.
#[derive(Debug, Default, Clone)]
pub struct Document {
    /// Paragraphs, in order. Empty means an empty document.
    paragraphs: Vec<String>,
}

impl Document {
    /// Create an empty document. [`write_doc`] on it yields a valid, minimal
    /// `.doc` with zero characters.
    pub fn new() -> Self {
        Document {
            paragraphs: Vec::new(),
        }
    }

    /// Append a paragraph of text. Paragraphs are joined by `\r` when written.
    pub fn add_paragraph(&mut self, text: &str) {
        self.paragraphs.push(text.to_string());
    }

    /// The document's logical text: paragraphs joined by the paragraph mark
    /// `\r`. This is exactly what a reader reassembles from the piece table.
    fn text(&self) -> String {
        self.paragraphs.join("\r")
    }
}

// ---------------------------------------------------------------------------
// The encoder.
// ---------------------------------------------------------------------------

/// Which on-disk encoding a single piece of text uses. This is the single most
/// important bit in the whole format: it selects how a reader turns bytes back
/// into characters, and it is carried by the `fCompressed` flag of the PCD's
/// `FcCompressed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    /// 8-bit: one Latin-1 byte per character. Only usable when every character
    /// is `<= U+00FF`. Compact, and the flag is *set*.
    Compressed8,
    /// 16-bit: two UTF-16LE bytes per character. The faithful fallback for any
    /// character `> U+00FF`. The flag is *clear*.
    Uncompressed16,
}

/// Encode `text` into the WordDocument text-region bytes for the chosen
/// encoding. Returns `None` if the byte length would overflow `usize` (a
/// pathologically huge document), so the caller can degrade gracefully.
fn encode_text_bytes(text: &str, encoding: Encoding) -> Option<Vec<u8>> {
    match encoding {
        Encoding::Compressed8 => {
            // One byte per char. Every char is guaranteed `<= 0xFF` here.
            let mut out = Vec::with_capacity(text.chars().count());
            for ch in text.chars() {
                // Defensive: never truncate silently. Callers only pick this
                // encoding after `pick_encoding` verified the range, but stay
                // total regardless.
                let cp = ch as u32;
                if cp > 0xFF {
                    return None;
                }
                out.push(cp as u8);
            }
            Some(out)
        }
        Encoding::Uncompressed16 => {
            // Two bytes per UTF-16 code unit. `encode_utf16` handles surrogate
            // pairs for astral chars, so the count can exceed the char count.
            let units: Vec<u16> = text.encode_utf16().collect();
            let byte_len = units.len().checked_mul(2)?;
            let mut out = Vec::with_capacity(byte_len);
            for u in units {
                out.extend_from_slice(&u.to_le_bytes());
            }
            Some(out)
        }
    }
}

/// Choose the encoding for a document: 8-bit compressed if every character fits
/// in Latin-1 (`<= U+00FF`), otherwise 16-bit uncompressed — the faithful path
/// that never mangles a character.
fn pick_encoding(text: &str) -> Encoding {
    if text.chars().all(|c| (c as u32) <= 0xFF) {
        Encoding::Compressed8
    } else {
        Encoding::Uncompressed16
    }
}

/// Build the `FcCompressed` `u32` for storing text at `text_offset` under the
/// given encoding. Returns `None` if the offset cannot be represented (would
/// collide with the flag bit or overflow when doubled).
///
/// - **8-bit:** the offset is stored *doubled*, with bit 30 set. A reader
///   recovers the real offset as `(fc & 0x3FFFFFFF) / 2`. The doubled offset
///   must fit in the low 30 bits.
/// - **16-bit:** the offset is stored *as-is*, bit 30 clear. It must fit in the
///   low 30 bits directly.
fn build_fc_compressed(text_offset: usize, encoding: Encoding) -> Option<u32> {
    match encoding {
        Encoding::Compressed8 => {
            // Double the offset, checked, then ensure it fits in 30 bits.
            let doubled = (text_offset as u64).checked_mul(2)?;
            if doubled > FC_OFFSET_MASK as u64 {
                return None;
            }
            Some((doubled as u32) | FC_COMPRESSED_FLAG)
        }
        Encoding::Uncompressed16 => {
            if text_offset as u64 > FC_OFFSET_MASK as u64 {
                return None;
            }
            Some(text_offset as u32)
        }
    }
}

/// Build the `1Table` stream: a CLX consisting of a single `Pcdt` wrapping a
/// one-piece `PlcPcd`.
///
/// Layout (see DOCW01 §4):
/// ```text
///   u8  clxt = 0x02
///   u32 lcb  = 16                 (length of the PlcPcd)
///   PlcPcd:
///     CP array : u32 0, u32 n     (2 CPs → 1 piece)
///     PCD      : u16 0, u32 fc, u16 0
/// ```
fn build_clx(n_chars: u32, fc_compressed: u32) -> Vec<u8> {
    // PlcPcd = CP array (8 bytes) + one PCD (8 bytes) = 16 bytes.
    let mut plc_pcd = Vec::with_capacity(16);
    plc_pcd.extend_from_slice(&0u32.to_le_bytes()); // CP[0] = 0
    plc_pcd.extend_from_slice(&n_chars.to_le_bytes()); // CP[1] = n
    plc_pcd.extend_from_slice(&0u16.to_le_bytes()); // PCD flags
    plc_pcd.extend_from_slice(&fc_compressed.to_le_bytes()); // FcCompressed
    plc_pcd.extend_from_slice(&0u16.to_le_bytes()); // prm

    let lcb = plc_pcd.len() as u32; // == 16

    let mut clx = Vec::with_capacity(1 + 4 + plc_pcd.len());
    clx.push(CLXT_PCDT); // clxt = Pcdt
    clx.extend_from_slice(&lcb.to_le_bytes()); // lcb
    clx.extend_from_slice(&plc_pcd); // the PlcPcd itself
    clx
}

/// Build the `WordDocument` stream: a zeroed buffer of length
/// `TEXT_OFFSET + text_bytes.len()` with the FIB fields set and the text placed
/// at [`TEXT_OFFSET`]. Returns `None` if the buffer length would overflow.
fn build_word_document(text_bytes: &[u8], n_chars: u32, lcb_clx: u32) -> Option<Vec<u8>> {
    let total = TEXT_OFFSET.checked_add(text_bytes.len())?;
    let mut wd = vec![0u8; total];

    // --- FIB fields (see DOCW01 §3) --------------------------------------
    // Each of these offsets is < TEXT_OFFSET, so `put_*` always fits; we still
    // route through the fallible helpers to stay panic-free.
    put_u16(&mut wd, 0, W_IDENT);
    put_u16(&mut wd, 2, N_FIB_WORD97);
    put_u16(&mut wd, 10, FIB_FLAGS_1TABLE);
    put_u16(&mut wd, 32, CSW);
    put_u16(&mut wd, 62, CSLW);
    put_u32(&mut wd, OFF_CCP_TEXT, n_chars);
    put_u16(&mut wd, 152, CB_RG_FC_LCB);
    put_u32(&mut wd, OFF_FC_CLX, 0); // fcClx = 0 (CLX at start of 1Table)
    put_u32(&mut wd, OFF_LCB_CLX, lcb_clx); // lcbClx = CLX byte length

    // --- The text, at TEXT_OFFSET ----------------------------------------
    // `total == TEXT_OFFSET + text_bytes.len()`, so this range always fits.
    if let Some(slot) = wd.get_mut(TEXT_OFFSET..total) {
        slot.copy_from_slice(text_bytes);
    }

    Some(wd)
}

/// Serialise a [`Document`] into a legacy `.doc` byte buffer.
///
/// The pipeline (see DOCW01):
/// 1. Join paragraphs by `\r` to get the logical text.
/// 2. Pick 8-bit vs 16-bit encoding (16-bit iff any char `> U+00FF`).
/// 3. Encode the text bytes and compute the `FcCompressed` offset word.
/// 4. Build the one-piece CLX (`1Table`) and the FIB+text (`WordDocument`).
/// 5. Wrap both streams in a CFB container.
///
/// Totality: any arithmetic that a colossal document could overflow is
/// `checked_*`; on overflow (or an out-of-range offset) we degrade to emitting a
/// valid **empty** document rather than corrupt bytes. Output is deterministic.
pub fn write_doc(doc: &Document) -> Vec<u8> {
    let text = doc.text();
    // Character count: for 8-bit this is bytes; for 16-bit the CP array counts
    // UTF-16 code units (which is what CP positions index in a 16-bit piece).
    let encoding = pick_encoding(&text);

    let built = build_streams(&text, encoding);
    let (wd, table) = match built {
        Some(pair) => pair,
        // A pathologically huge document overflowed an offset/length. Fall back
        // to a valid empty document — never emit corrupt bytes.
        None => build_streams("", Encoding::Compressed8)
            .unwrap_or_else(|| (Vec::new(), Vec::new())),
    };

    cfb_writer::write_cfb(&[(STREAM_WORD_DOCUMENT, &wd), (STREAM_1TABLE, &table)])
}

/// Build the (`WordDocument`, `1Table`) byte pair for `text` under `encoding`,
/// or `None` if any length/offset overflows.
fn build_streams(text: &str, encoding: Encoding) -> Option<(Vec<u8>, Vec<u8>)> {
    let text_bytes = encode_text_bytes(text, encoding)?;

    // The CP array counts characters for 8-bit, UTF-16 code units for 16-bit —
    // in both cases exactly `text_bytes.len() / bytes_per_unit`.
    let n_chars_usize = match encoding {
        Encoding::Compressed8 => text_bytes.len(),
        Encoding::Uncompressed16 => text_bytes.len() / 2,
    };
    let n_chars = u32::try_from(n_chars_usize).ok()?;

    let fc = build_fc_compressed(TEXT_OFFSET, encoding)?;
    let table = build_clx(n_chars, fc);
    let lcb_clx = u32::try_from(table.len()).ok()?;
    let wd = build_word_document(&text_bytes, n_chars, lcb_clx)?;
    Some((wd, table))
}

#[cfg(test)]
mod tests;
