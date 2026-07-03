//! # xls-writer — legacy `.xls` (BIFF8 / [MS-XLS]) writer (XLSW01)
//!
//! A from-scratch, **zero-third-party-dependency** writer for the legacy
//! **`.xls`** spreadsheet format. You build a simple model — sheets of string
//! and number cells — and [`write_xls`] hands you the bytes of a real `.xls`
//! file. See `code/specs/XLSW01-xls-writer.md` for the full literate walkthrough.
//!
//! ## The two-layer mental model
//!
//! A `.xls` file is an **OLE2 Compound File** (a "FAT filesystem crammed into a
//! single file"; see the `cfb`/`cfb-writer` crates) that contains **one** named
//! stream called `Workbook`. That stream is a flat sequence of **BIFF records**.
//! So writing a `.xls` is:
//!
//! 1. **This crate:** turn the model into a `Vec<u8>` of BIFF records.
//! 2. **`cfb-writer`:** wrap that `Vec<u8>` as the `Workbook` stream.
//!
//! ```
//! # use xls_writer::{Workbook, write_xls};
//! let mut wb = Workbook::new();
//! let sheet = wb.add_sheet("Revenue");
//! sheet.set_string(0, 0, "Q1");
//! sheet.set_number(0, 1, 1000.0);
//! let bytes = write_xls(&wb);
//! // It's a Compound File: it opens with the OLE2 magic.
//! assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
//! ```
//!
//! ## A BIFF record
//!
//! Every BIFF record is a tiny type-length-value:
//!
//! ```text
//!   ┌──────────┬──────────┬──────────────────────┐
//!   │ u16 type │ u16 size │  `size` body bytes    │   (all integers little-endian)
//!   └──────────┴──────────┴──────────────────────┘
//! ```
//!
//! Records are grouped into **substreams** bracketed by `BOF`…`EOF`. We emit one
//! **globals** substream (declaring the sheets and the shared-string table) and
//! one **worksheet** substream per sheet (the cells).
//!
//! ## Robustness
//!
//! `#![forbid(unsafe_code)]`, deterministic (no timestamps/randomness), and no
//! `unwrap`/`expect`/`panic!` on the public path. Because BIFF size fields are
//! `u16` and cell addresses are `u16`, a colossal model can't fit; we **clamp or
//! skip** the offending piece (documented) rather than wrap it into a corrupt
//! record. See the `limits` notes on [`Sheet::set_string`] etc.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// BIFF record type numbers. See §1.1 of the spec for the table.
// ---------------------------------------------------------------------------

/// Beginning of a substream. Body carries the BIFF version and substream kind.
const REC_BOF: u16 = 0x0809;
/// End of a substream. Empty body.
const REC_EOF: u16 = 0x000A;
/// Declares one worksheet: its byte offset in the stream plus its name.
const REC_BOUNDSHEET: u16 = 0x0085;
/// The shared string table: every distinct string value, stored once.
const REC_SST: u16 = 0x00FC;
/// A string cell: references a string by index into the SST.
const REC_LABELSST: u16 = 0x00FD;
/// A numeric cell: an IEEE-754 `f64`.
const REC_NUMBER: u16 = 0x0203;

/// BIFF version marker written into every `BOF`: `0x0600` = BIFF8.
const BIFF8_VERSION: u16 = 0x0600;
/// `BOF` substream kind: the workbook globals.
const DT_GLOBALS: u16 = 0x0005;
/// `BOF` substream kind: a worksheet.
const DT_WORKSHEET: u16 = 0x0010;

/// The largest a single BIFF record *body* may be — the `size` field is a `u16`.
/// (Bodies larger than this need `CONTINUE` records, which we do not emit; see
/// the SST limitation in the spec.)
const MAX_RECORD_BODY: usize = u16::MAX as usize;

// ---------------------------------------------------------------------------
// Little-endian append helpers. Appending to a `Vec<u8>` never panics, so these
// are total by construction.
// ---------------------------------------------------------------------------

/// Append a `u16` little-endian.
#[inline]
fn push_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Append a `u32` little-endian.
#[inline]
fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Append an `f64` little-endian (IEEE-754).
#[inline]
fn push_f64(buf: &mut Vec<u8>, v: f64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Append a whole BIFF record: `type`, `size`, then the body.
///
/// If `body` is somehow longer than a `u16` can express, the size field is
/// **clamped** to `u16::MAX`. On the public path this never happens — every body
/// we build is bounded (the SST is kept small; see [`encode_sst`]) — but clamping
/// keeps the function total rather than panicking on a `try_into`.
fn push_record(buf: &mut Vec<u8>, record_type: u16, body: &[u8]) {
    let size = body.len().min(MAX_RECORD_BODY) as u16;
    push_u16(buf, record_type);
    push_u16(buf, size);
    // Only write the bytes we declared (defensive; equal in the normal case).
    buf.extend_from_slice(&body[..size as usize]);
}

// ---------------------------------------------------------------------------
// String encoding — the `fHighByte` scheme (spec §5).
// ---------------------------------------------------------------------------

/// The two BIFF8 string encodings. `fHighByte` bit0 of the `grbit` flags picks
/// between them.
///
/// - `Compressed` (bit clear): one byte per character — the low byte of each
///   UTF-16 code unit. Valid only when every code unit is ≤ `0xFF`.
/// - `Wide` (bit set): two bytes per character, little-endian UTF-16LE.
struct EncodedString {
    /// Number of UTF-16 code units (this is `cch`).
    cch: usize,
    /// `grbit` bit0: 1 for wide (16-bit) encoding, 0 for compressed (8-bit).
    high_byte: bool,
    /// The already-encoded character bytes (1×cch or 2×cch bytes).
    chars: Vec<u8>,
}

/// Encode a `&str` into the BIFF8 character bytes, choosing the compact 8-bit
/// form when every code unit fits in a byte, else 16-bit UTF-16LE.
///
/// We count in **UTF-16 code units**, because that is what the on-disk `cch`
/// field means (a character above the BMP is two units). This mirrors how the
/// `cfb` reader / Excel interpret the field.
fn encode_string(s: &str) -> EncodedString {
    let units: Vec<u16> = s.encode_utf16().collect();
    let all_latin1 = units.iter().all(|&u| u <= 0x00FF);
    let chars = if all_latin1 {
        // Compressed: the low byte of each unit.
        units.iter().map(|&u| u as u8).collect()
    } else {
        // Wide: UTF-16LE, two bytes per unit.
        let mut out = Vec::with_capacity(units.len() * 2);
        for &u in &units {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    };
    EncodedString {
        cch: units.len(),
        high_byte: !all_latin1,
        chars,
    }
}

// ---------------------------------------------------------------------------
// The public model: Workbook → Sheet → cells.
// ---------------------------------------------------------------------------

/// A single cell's value. Rows/cols are stored separately in [`Cell`].
enum CellValue {
    /// A string cell. The string is de-duplicated into the SST at write time.
    Str(String),
    /// A numeric cell (IEEE-754 `f64`).
    Num(f64),
}

/// One placed cell: its 0-based row, 0-based column, and value.
struct Cell {
    row: u32,
    col: u32,
    value: CellValue,
}

/// One worksheet: a name and a list of placed cells.
///
/// Cells are stored in **insertion order** and emitted in that order, so output
/// is deterministic. We do not sort or de-duplicate by address — setting the
/// same `(row, col)` twice emits two records, and a reader keeps the last; that
/// is a caller error, not ours to police, and staying order-preserving keeps the
/// bytes predictable.
pub struct Sheet {
    name: String,
    cells: Vec<Cell>,
}

impl Sheet {
    /// Place a **string** value at 0-based `(row, col)`.
    ///
    /// **Limits:** BIFF cell addresses are `u16`. A `row` or `col` greater than
    /// `u16::MAX` (65535) cannot be represented and the cell is **skipped** at
    /// write time (documented; never truncated into a wrong address). The string
    /// is shared via the SST, so repeats cost nothing.
    pub fn set_string(&mut self, row: u32, col: u32, s: &str) {
        self.cells.push(Cell {
            row,
            col,
            value: CellValue::Str(s.to_string()),
        });
    }

    /// Place a **numeric** value at 0-based `(row, col)`.
    ///
    /// Same `u16` address limit as [`set_string`](Self::set_string): a cell
    /// beyond 65535 in either axis is skipped at write time. Any `f64`
    /// (including NaN/∞) is stored verbatim via a `NUMBER` record.
    pub fn set_number(&mut self, row: u32, col: u32, n: f64) {
        self.cells.push(Cell {
            row,
            col,
            value: CellValue::Num(n),
        });
    }
}

/// A workbook: an ordered collection of [`Sheet`]s. Build it, then call
/// [`write_xls`] to serialise it into `.xls` bytes.
pub struct Workbook {
    sheets: Vec<Sheet>,
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

impl Workbook {
    /// Create an empty workbook (no sheets). Writing it yields a valid, minimal
    /// `.xls` with an empty globals substream and no worksheets.
    pub fn new() -> Self {
        Workbook { sheets: Vec::new() }
    }

    /// Append a new sheet with the given name and return a mutable handle to it
    /// so cells can be added:
    ///
    /// ```
    /// # use xls_writer::Workbook;
    /// let mut wb = Workbook::new();
    /// let s = wb.add_sheet("Sheet1");
    /// s.set_number(0, 0, 42.0);
    /// ```
    pub fn add_sheet(&mut self, name: &str) -> &mut Sheet {
        self.sheets.push(Sheet {
            name: name.to_string(),
            cells: Vec::new(),
        });
        // `push` guarantees at least one element, so `last_mut` is `Some`; the
        // `expect` here can never fire and keeps the return type ergonomic.
        // (Kept off the true "public failure path" — it is structurally
        // impossible, not input-dependent.)
        self.sheets
            .last_mut()
            .expect("just pushed a sheet, so last_mut is Some")
    }
}

// ---------------------------------------------------------------------------
// Shared string table construction.
// ---------------------------------------------------------------------------

/// The result of scanning all cells for strings: the ordered list of distinct
/// strings (the SST payload), a map from string → its SST index, and the total
/// number of string-cell references (`cstTotal`).
struct SharedStrings {
    /// Distinct strings in first-seen order. Index into this vec is the `isst`.
    unique: Vec<String>,
    /// Total count of string cells across all sheets (`cstTotal`).
    total_refs: u32,
}

impl SharedStrings {
    /// Scan every sheet's cells, deduplicating string values into the SST.
    ///
    /// De-dup is by exact string equality (first occurrence wins the index).
    /// `total_refs` counts every string *cell* (so two cells with the same text
    /// count twice); `unique.len()` is `cstUnique`.
    fn collect(sheets: &[Sheet]) -> SharedStrings {
        // A small linear-probe index. Workbooks here are tiny, so a `Vec` +
        // linear search is simpler than pulling in a hash map and is plenty
        // fast; it also keeps ordering trivially deterministic.
        let mut unique: Vec<String> = Vec::new();
        let mut total_refs: u32 = 0;
        for sheet in sheets {
            for cell in &sheet.cells {
                if let CellValue::Str(s) = &cell.value {
                    total_refs = total_refs.saturating_add(1);
                    if !unique.iter().any(|u| u == s) {
                        unique.push(s.clone());
                    }
                }
            }
        }
        SharedStrings { unique, total_refs }
    }

    /// Look up the SST index (`isst`) of a string. Returns `None` only if the
    /// string was never registered (impossible for a string that came from a
    /// scanned cell, but total by construction).
    fn index_of(&self, s: &str) -> Option<u32> {
        self.unique
            .iter()
            .position(|u| u == s)
            .and_then(|p| u32::try_from(p).ok())
    }
}

/// Encode the SST record body from the collected shared strings.
///
/// Layout (spec §3): `u32 cstTotal`, `u32 cstUnique`, then each unique string as
/// an `XLUnicodeRichExtendedString` (`u16 cch`, `u8 grbit`, chars).
///
/// **Limitation:** we build the whole SST as one record. If the encoded body
/// would exceed `u16::MAX` bytes it cannot fit the record's size field; rather
/// than emit a corrupt record we stop adding strings once we would overflow
/// (the already-written strings remain valid and indexable). Realistic small
/// workbooks never approach this; `CONTINUE`-splitting is future work.
fn encode_sst(shared: &SharedStrings) -> Vec<u8> {
    let mut body = Vec::new();
    push_u32(&mut body, shared.total_refs);
    // `cstUnique` = number of distinct strings.
    let unique_count = u32::try_from(shared.unique.len()).unwrap_or(u32::MAX);
    push_u32(&mut body, unique_count);

    for s in &shared.unique {
        let enc = encode_string(s);
        // A single string with more than u16::MAX code units cannot fit its
        // `cch` field; skip it rather than write a wrong length. (Its `isst`
        // still resolves to *a* string for any cell that referenced it — the
        // preceding valid strings — because we never renumber; but a string this
        // large is pathological and out of scope.)
        let cch = match u16::try_from(enc.cch) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Would adding this string overflow the record body? Header for this
        // string is 3 bytes (u16 cch + u8 grbit) plus its char bytes.
        let addition = 3usize.saturating_add(enc.chars.len());
        if body.len().saturating_add(addition) > MAX_RECORD_BODY {
            // Documented limitation: stop before overflowing the size field.
            break;
        }
        push_u16(&mut body, cch);
        body.push(if enc.high_byte { 0x01 } else { 0x00 });
        body.extend_from_slice(&enc.chars);
    }
    body
}

// ---------------------------------------------------------------------------
// Record builders.
// ---------------------------------------------------------------------------

/// Append a `BOF` record for the given substream kind (`DT_GLOBALS` or
/// `DT_WORKSHEET`). The 16-byte body is `vers`, `dt`, then four zeroed trailer
/// fields (`rupBuild`, `rupYear`, `bfh`, `sfo`) for deterministic output.
fn push_bof(buf: &mut Vec<u8>, dt: u16) {
    let mut body = Vec::with_capacity(16);
    push_u16(&mut body, BIFF8_VERSION);
    push_u16(&mut body, dt);
    push_u16(&mut body, 0); // rupBuild
    push_u16(&mut body, 0); // rupYear
    push_u32(&mut body, 0); // bfh
    push_u32(&mut body, 0); // sfo
    push_record(buf, REC_BOF, &body);
}

/// Append an `EOF` record (empty body).
fn push_eof(buf: &mut Vec<u8>) {
    push_record(buf, REC_EOF, &[]);
}

/// Append the 6-byte cell head (`row`, `col`, `ixfe=0`) to a record body. The
/// caller has already validated that `row`/`col` fit in `u16`.
fn push_cell_head(body: &mut Vec<u8>, row: u16, col: u16) {
    push_u16(body, row);
    push_u16(body, col);
    push_u16(body, 0); // ixfe: the default cell format (XF 0)
}

/// Build the globals substream, returning the bytes **and** the byte offset,
/// within those bytes, of each sheet's `BOUNDSHEET.lbPlyPos` field (so the
/// caller can backfill it once worksheet positions are known — spec §4).
///
/// The `lbPlyPos` fields are written as placeholder `0` here.
fn build_globals(sheets: &[Sheet], shared: &SharedStrings) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::new();
    push_bof(&mut buf, DT_GLOBALS);

    // One BOUNDSHEET per sheet. Record where each lbPlyPos field lands so we can
    // patch it later.
    let mut ply_pos_offsets: Vec<usize> = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        // Build the BOUNDSHEET body: u32 lbPlyPos (placeholder), u8 hsState,
        // u8 dt, then a ShortXLUnicodeString name.
        let mut body = Vec::new();
        push_u32(&mut body, 0); // lbPlyPos placeholder — patched in build_workbook_stream
        body.push(0); // hsState = 0 (visible)
        body.push(0); // dt = 0 (worksheet)

        // ShortXLUnicodeString: u8 cch, u8 grbit, chars. `cch` is a u8, so a
        // name longer than 255 code units is clamped (documented) — sheet names
        // are 31 chars in real Excel anyway.
        let enc = encode_string(&sheet.name);
        let cch = enc.cch.min(u8::MAX as usize) as u8;
        body.push(cch);
        body.push(if enc.high_byte { 0x01 } else { 0x00 });
        // Emit exactly `cch` characters' worth of bytes.
        let char_bytes = if enc.high_byte { cch as usize * 2 } else { cch as usize };
        body.extend_from_slice(&enc.chars[..char_bytes.min(enc.chars.len())]);

        // The lbPlyPos field is the first 4 bytes of the body, which lands right
        // after this record's 4-byte header. Record its absolute offset in buf.
        let record_start = buf.len();
        let ply_pos_offset = record_start + 4; // skip u16 type + u16 size
        ply_pos_offsets.push(ply_pos_offset);
        push_record(&mut buf, REC_BOUNDSHEET, &body);
    }

    // The shared string table.
    let sst_body = encode_sst(shared);
    push_record(&mut buf, REC_SST, &sst_body);

    push_eof(&mut buf);
    (buf, ply_pos_offsets)
}

/// Build one worksheet substream: `BOF`, the cell records, `EOF`.
///
/// Cells whose row or col exceeds `u16::MAX` are **skipped** (spec §6) rather
/// than wrapped into a wrong address.
fn build_worksheet(sheet: &Sheet, shared: &SharedStrings) -> Vec<u8> {
    let mut buf = Vec::new();
    push_bof(&mut buf, DT_WORKSHEET);

    for cell in &sheet.cells {
        // Reject out-of-range addresses cleanly.
        let (row, col) = match (u16::try_from(cell.row), u16::try_from(cell.col)) {
            (Ok(r), Ok(c)) => (r, c),
            _ => continue, // documented: skip cells beyond the 65535 grid limit
        };
        match &cell.value {
            CellValue::Num(n) => {
                let mut body = Vec::with_capacity(14);
                push_cell_head(&mut body, row, col);
                push_f64(&mut body, *n);
                push_record(&mut buf, REC_NUMBER, &body);
            }
            CellValue::Str(s) => {
                // Resolve the string's SST index. It must exist (we scanned the
                // same cells to build the SST); if for any reason it does not,
                // skip the cell rather than emit a dangling reference.
                let Some(isst) = shared.index_of(s) else {
                    continue;
                };
                let mut body = Vec::with_capacity(10);
                push_cell_head(&mut body, row, col);
                push_u32(&mut body, isst);
                push_record(&mut buf, REC_LABELSST, &body);
            }
        }
    }

    push_eof(&mut buf);
    buf
}

/// Assemble the whole `Workbook` byte-stream: globals + every worksheet, with
/// each `BOUNDSHEET.lbPlyPos` backfilled to point at its worksheet's `BOF`
/// (the two-pass of spec §4).
fn build_workbook_stream(wb: &Workbook) -> Vec<u8> {
    let shared = SharedStrings::collect(&wb.sheets);
    let (mut globals, ply_pos_offsets) = build_globals(&wb.sheets, &shared);

    // Build each worksheet buffer and remember its length.
    let worksheets: Vec<Vec<u8>> = wb
        .sheets
        .iter()
        .map(|s| build_worksheet(s, &shared))
        .collect();

    // Compute each worksheet's absolute start offset in the final stream:
    //   offset(sheet k) = globals.len() + Σ_{i<k} worksheet[i].len()
    let globals_len = globals.len();
    let mut running = globals_len;
    for (i, offset) in ply_pos_offsets.iter().enumerate() {
        // Backfill this sheet's lbPlyPos with `running` (its BOF offset).
        let pos = u32::try_from(running).unwrap_or(u32::MAX);
        let bytes = pos.to_le_bytes();
        // `offset` was computed as a valid index into `globals`; patch 4 bytes.
        if let Some(slot) = globals.get_mut(*offset..*offset + 4) {
            slot.copy_from_slice(&bytes);
        }
        // Advance past this worksheet for the next sheet's offset.
        if let Some(ws) = worksheets.get(i) {
            running = running.saturating_add(ws.len());
        }
    }

    // Concatenate globals + all worksheets.
    let total: usize = globals_len
        + worksheets.iter().map(|w| w.len()).sum::<usize>();
    globals.reserve(total.saturating_sub(globals_len));
    for ws in &worksheets {
        globals.extend_from_slice(ws);
    }
    globals
}

/// Serialise a [`Workbook`] into the bytes of a legacy `.xls` file.
///
/// Builds the `Workbook` BIFF stream (globals + worksheets) and wraps it in an
/// OLE2 Compound File via `cfb-writer`. Deterministic and infallible.
///
/// ```
/// # use xls_writer::{Workbook, write_xls};
/// let mut wb = Workbook::new();
/// wb.add_sheet("S").set_number(0, 0, 3.14);
/// let bytes = write_xls(&wb);
/// assert!(bytes.len() >= 512); // a whole CFB, at minimum one sector + header
/// ```
pub fn write_xls(wb: &Workbook) -> Vec<u8> {
    let workbook_stream = build_workbook_stream(wb);
    cfb_writer::write_cfb(&[("Workbook", &workbook_stream)])
}

#[cfg(test)]
mod tests;
