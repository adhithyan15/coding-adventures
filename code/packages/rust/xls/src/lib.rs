//! # xls — legacy `.xls` (BIFF8 / [MS-XLS]) reader (XLS01)
//!
//! A from-scratch, zero-third-party-dependency reader for **legacy Excel
//! `.xls` workbooks**. It decodes a workbook into a typed
//! `Workbook → Sheet → Cell` model. See `code/specs/XLS01-biff-reader.md` for
//! the full literate walkthrough.
//!
//! ## Where this sits in the stack
//!
//! ```text
//!   zip → deflate → xml → opc → spreadsheetml → xlsx-eval   (modern .xlsx)
//!   cfb → xls                                               (legacy .xls)  ← HERE
//! ```
//!
//! A modern `.xlsx` is a ZIP of XML. A legacy `.xls` is an **OLE2 Compound
//! File** (a tiny FAT filesystem crammed into one file) whose `Workbook`
//! stream is a flat sequence of **BIFF records**. The [`cfb`] crate turns the
//! outer container into named byte streams; this crate parses the `Workbook`
//! byte stream into cells.
//!
//! ```text
//!   .xls bytes ──cfb::CompoundFile::open──▶ container
//!                        │ .read_stream("Workbook")   (or "Book" for old files)
//!                        ▼
//!                   Vec<u8>  = the BIFF record stream   ◀── THIS CRATE parses this
//!                        ▼
//!   Workbook { sheets: [ Sheet { name, cells: [ Cell { row, col, value } ] } ] }
//! ```
//!
//! ## The one-paragraph mental model of BIFF
//!
//! A BIFF stream is **back-to-back records**. Every record is a 4-byte header —
//! `u16 record_type` (LE), `u16 size` (LE) — followed by `size` body bytes:
//!
//! ```text
//!   ┌──────────┬──────────┬───────────────────────────┐
//!   │ u16 type │ u16 size │  `size` bytes of body      │
//!   └──────────┴──────────┴───────────────────────────┘
//! ```
//!
//! Records are grouped into **substreams** bracketed by **BOF** (`0x0809`) and
//! **EOF** (`0x000A`). The BOF body `[2..4]` says what follows: `0x0005` =
//! workbook globals (the shared string table + the sheet directory), `0x0010` =
//! a worksheet (one sheet's cells).
//!
//! ## Reading untrusted bytes
//!
//! `.xls` files arrive as email attachments, so this parser assumes hostility:
//! it is `#![forbid(unsafe_code)]`, never `unwrap`/`panic!`s on input, and
//! bounds-checks every declared count/size against the bytes actually remaining
//! before reading or allocating. See the module security notes below.
//!
//! ```
//! # use xls::{open_xls, CellValue};
//! # fn demo(bytes: &[u8]) -> Result<(), xls::XlsError> {
//! let wb = open_xls(bytes)?;
//! for sheet in wb.sheets() {
//!     println!("sheet {}", sheet.name);
//!     for cell in sheet.cells() {
//!         println!("  ({},{}) = {:?}", cell.row, cell.col, cell.value);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![deny(warnings)]

use std::fmt;

// ---------------------------------------------------------------------------
// Record type codes. These are the BIFF8 opcodes we understand. Anything not
// in this list is skipped by its declared size (forward-compatibility).
// ---------------------------------------------------------------------------

const REC_FORMULA: u16 = 0x0006; // cell head + 8-byte cached result + rgce
const REC_EOF: u16 = 0x000A; // ends a substream
const REC_BOF: u16 = 0x0809; // begins a substream; body[2..4] = substream kind
const REC_CONTINUE: u16 = 0x003C; // spill for records that overflow 8224 bytes
const REC_BOUNDSHEET: u16 = 0x0085; // sheet directory entry (globals)
const REC_MULRK: u16 = 0x00BD; // run of RK numbers across adjacent columns
const REC_SST: u16 = 0x00FC; // shared string table (globals)
const REC_LABELSST: u16 = 0x00FD; // cell referencing SST[isst]
const REC_BLANK: u16 = 0x0201; // empty cell
const REC_NUMBER: u16 = 0x0203; // cell head + f64
const REC_LABEL: u16 = 0x0204; // cell head + inline (non-shared) string
const REC_BOOLERR: u16 = 0x0205; // cell head + (value, fError)
const REC_STRING: u16 = 0x0207; // cached string result of a preceding FORMULA
const REC_RK: u16 = 0x027E; // cell head + packed RK number

/// BOF substream kind for the workbook globals substream.
const SUB_GLOBALS: u16 = 0x0005;
/// BOF substream kind for a worksheet substream.
const SUB_WORKSHEET: u16 = 0x0010;

// ---------------------------------------------------------------------------
// Safety caps. A hostile file must never make us allocate unbounded memory or
// loop forever. These ceilings comfortably exceed any real legacy workbook.
// ---------------------------------------------------------------------------

/// Hard cap on the number of unique SST strings we will accept, independent of
/// the (possibly lying) declared `cstUnique`. Real workbooks stay well under
/// this; it stops a tiny stream from claiming billions of strings.
const MAX_SST_STRINGS: usize = 4_000_000;
/// Hard cap on CONTINUE records chained after an SST — refuses a pathological
/// chain.
const MAX_CONTINUE_RECORDS: usize = 1_000_000;
/// Hard cap on cells emitted from a single MULRK run.
const MAX_MULRK_SPAN: usize = 65_536;

// ---------------------------------------------------------------------------
// Public error type.
// ---------------------------------------------------------------------------

/// Everything that can go wrong reading a (possibly hostile) `.xls` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XlsError {
    /// The outer OLE2/CFB container failed to parse.
    Cfb(cfb::CfbError),
    /// The container has neither a `Workbook` nor a `Book` stream.
    NoWorkbookStream,
    /// A record or field ran past the end of the stream we were reading.
    Truncated,
    /// A declared count or size exceeded the bytes available (or our safety
    /// caps) — we refuse rather than allocate.
    TooLarge,
    /// A string's bytes were not valid UTF-16.
    BadString,
}

impl fmt::Display for XlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XlsError::Cfb(e) => write!(f, "OLE2/CFB container error: {e}"),
            XlsError::NoWorkbookStream => {
                write!(f, "no Workbook or Book stream in the compound file")
            }
            XlsError::Truncated => write!(f, "BIFF record ran past end of stream"),
            XlsError::TooLarge => write!(f, "declared size/count exceeds available bytes"),
            XlsError::BadString => write!(f, "string bytes were not valid UTF-16"),
        }
    }
}

impl std::error::Error for XlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            XlsError::Cfb(e) => Some(e),
            _ => None,
        }
    }
}

impl From<cfb::CfbError> for XlsError {
    fn from(e: cfb::CfbError) -> Self {
        XlsError::Cfb(e)
    }
}

// ---------------------------------------------------------------------------
// Public model: Workbook → Sheet → Cell.
// ---------------------------------------------------------------------------

/// A decoded cell value. Row/col are 0-based (as BIFF stores them).
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// A numeric value (from RK, MULRK, NUMBER, or a numeric FORMULA cache).
    Number(f64),
    /// Text (from LABELSST / SST, LABEL, or a string FORMULA cache).
    Text(String),
    /// A boolean (from BOOLERR with fError=0, or a boolean FORMULA cache).
    Bool(bool),
    /// An error code (from BOOLERR with fError=1, or an error FORMULA cache).
    Error(u8),
    /// A formula cell, carrying whatever cached result we decoded. We do not
    /// decode the formula expression itself.
    Formula { cached: Box<CellValue> },
    /// An explicitly blank cell.
    Blank,
}

/// One cell at a (row, col) position.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// 0-based row.
    pub row: u32,
    /// 0-based column.
    pub col: u32,
    /// The decoded value.
    pub value: CellValue,
}

/// One worksheet: a name and its cells (in the order they appeared).
#[derive(Debug, Clone)]
pub struct Sheet {
    /// The sheet's display name.
    pub name: String,
    cells: Vec<Cell>,
}

impl Sheet {
    /// All cells on this sheet, in record order.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Look up a cell by (row, col). Linear scan — fine for our sizes.
    pub fn cell(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells.iter().find(|c| c.row == row && c.col == col)
    }
}

/// A decoded workbook: its sheets in file order.
#[derive(Debug, Clone)]
pub struct Workbook {
    sheets: Vec<Sheet>,
}

impl Workbook {
    /// The sheets, in the order the BOUNDSHEET records appeared.
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }

    /// Look up a sheet by exact name.
    pub fn sheet(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.name == name)
    }
}

// ---------------------------------------------------------------------------
// Little-endian primitive reads. Each returns `None` on out-of-bounds rather
// than panicking, so the whole parser is total on any input.
// ---------------------------------------------------------------------------

#[inline]
fn read_u16(buf: &[u8], off: usize) -> Option<u16> {
    let b = buf.get(off..off.checked_add(2)?)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

#[inline]
fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    let b = buf.get(off..off.checked_add(4)?)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

#[inline]
fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    let b = buf.get(off..off.checked_add(8)?)?;
    let mut arr = [0u8; 8];
    arr.copy_from_slice(b);
    Some(f64::from_le_bytes(arr))
}

// ---------------------------------------------------------------------------
// Record framing.
//
// A single BIFF record borrowed out of the stream, plus the byte offset where
// its header started (needed to match a worksheet BOF to a BOUNDSHEET's
// lbPlyPos).
// ---------------------------------------------------------------------------

struct Record<'a> {
    /// Byte offset of this record's 4-byte header within the stream.
    header_offset: usize,
    /// The record opcode (`u16 type`).
    rec_type: u16,
    /// The record body (`size` bytes; already bounds-checked to fit).
    body: &'a [u8],
}

/// Walk the stream into a `Vec<Record>`. Records whose declared `size` runs
/// past the buffer end the walk (permissive: we keep everything valid so far).
fn parse_records(stream: &[u8]) -> Vec<Record<'_>> {
    let mut records = Vec::new();
    let mut pos = 0usize;
    // Stop when fewer than 4 bytes (a full header) remain.
    while pos.checked_add(4).is_some_and(|end| end <= stream.len()) {
        // These reads cannot fail given the loop guard, but stay total anyway.
        let (rec_type, size) = match (read_u16(stream, pos), read_u16(stream, pos + 2)) {
            (Some(t), Some(s)) => (t, s as usize),
            _ => break,
        };
        let body_start = pos + 4;
        let body_end = match body_start.checked_add(size) {
            Some(e) if e <= stream.len() => e,
            // A record claiming more bytes than remain: truncated file. Stop.
            _ => break,
        };
        // `get` cannot fail here, but keep it total.
        let Some(body) = stream.get(body_start..body_end) else {
            break;
        };
        records.push(Record {
            header_offset: pos,
            rec_type,
            body,
        });
        pos = body_end;
    }
    records
}

// ---------------------------------------------------------------------------
// XLUnicodeString / ShortXLUnicodeString decoding.
//
// Both are a character count, a grbit flag byte (bit0 = fHighByte), then the
// characters: 8-bit latin1 (one byte each) when fHighByte is clear, or 16-bit
// UTF-16LE (two bytes each) when set. They differ only in the width of the
// count field: ShortXLUnicodeString uses a u8 count, XLUnicodeString a u16.
// These flat variants (no rich/phonetic extensions) appear in BOUNDSHEET,
// LABEL, and the STRING record.
// ---------------------------------------------------------------------------

/// Decode a flat unicode string whose count is already known, starting at
/// `off` (which points at the grbit byte). Returns `(text, bytes_consumed_from_off)`.
fn read_flat_string(body: &[u8], off: usize, cch: usize) -> Result<(String, usize), XlsError> {
    let grbit = *body.get(off).ok_or(XlsError::Truncated)?;
    let high_byte = grbit & 0x01 != 0;
    let char_bytes = if high_byte { 2 } else { 1 };
    let data_len = cch.checked_mul(char_bytes).ok_or(XlsError::TooLarge)?;
    let data_start = off.checked_add(1).ok_or(XlsError::TooLarge)?;
    let data_end = data_start.checked_add(data_len).ok_or(XlsError::TooLarge)?;
    let data = body.get(data_start..data_end).ok_or(XlsError::Truncated)?;
    let text = decode_chars(data, high_byte)?;
    // consumed = 1 (grbit) + data_len
    Ok((text, 1 + data_len))
}

/// Decode `data` as latin1 (one byte/char) or UTF-16LE (two bytes/char).
fn decode_chars(data: &[u8], high_byte: bool) -> Result<String, XlsError> {
    if high_byte {
        // UTF-16LE. `data.len()` is a multiple of 2 by construction.
        let mut units = Vec::with_capacity(data.len() / 2);
        let mut i = 0;
        while i + 1 < data.len() {
            units.push(u16::from_le_bytes([data[i], data[i + 1]]));
            i += 2;
        }
        String::from_utf16(&units).map_err(|_| XlsError::BadString)
    } else {
        // 8-bit chars are latin1 code points (U+0000..=U+00FF).
        Ok(data.iter().map(|&b| b as char).collect())
    }
}

// ---------------------------------------------------------------------------
// The record-spanning byte reader for the SST.
//
// Why this exists: no BIFF record body may exceed 8224 bytes, so a large SST
// spills into following CONTINUE (0x003C) records. Worse, a single string's
// character data may split across a record boundary — and when it does, the
// FIRST byte of the CONTINUE body is a FRESH grbit/fHighByte flag for the
// REMAINDER of that string. So the string's tail may switch 16-bit ↔ 8-bit.
//
// This reader presents [SST body, CONTINUE body, CONTINUE body, …] as one
// logical byte source. Fixed-width fields (counts, flags) are read from the
// current record and MUST NOT span a boundary (they never do in practice). The
// per-string CHARACTER decode is the only thing that spans, and it re-reads the
// flag byte at each boundary.
// ---------------------------------------------------------------------------

struct SstReader<'a> {
    /// The SST body followed by each CONTINUE body, in order.
    chunks: Vec<&'a [u8]>,
    /// Index of the chunk we are currently reading.
    chunk: usize,
    /// Offset within `chunks[chunk]`.
    pos: usize,
}

impl<'a> SstReader<'a> {
    fn new(chunks: Vec<&'a [u8]>) -> Self {
        SstReader {
            chunks,
            chunk: 0,
            pos: 0,
        }
    }

    /// Bytes remaining in the current chunk.
    fn cur_remaining(&self) -> usize {
        self.chunks
            .get(self.chunk)
            .map(|c| c.len().saturating_sub(self.pos))
            .unwrap_or(0)
    }

    /// Advance to the next chunk if the current one is exhausted. Returns false
    /// when there are no more chunks.
    fn ensure_chunk(&mut self) -> bool {
        while self.chunk < self.chunks.len() && self.cur_remaining() == 0 {
            self.chunk += 1;
            self.pos = 0;
        }
        self.chunk < self.chunks.len()
    }

    /// Read one byte, advancing (and crossing into the next chunk if needed).
    fn read_u8(&mut self) -> Result<u8, XlsError> {
        if !self.ensure_chunk() {
            return Err(XlsError::Truncated);
        }
        let chunk = self.chunks.get(self.chunk).ok_or(XlsError::Truncated)?;
        let b = *chunk.get(self.pos).ok_or(XlsError::Truncated)?;
        self.pos += 1;
        Ok(b)
    }

    /// Read a u16 from the current chunk. Fixed-width fields are not permitted
    /// to span a chunk boundary (BIFF never splits them).
    fn read_u16(&mut self) -> Result<u16, XlsError> {
        if !self.ensure_chunk() {
            return Err(XlsError::Truncated);
        }
        let chunk = self.chunks.get(self.chunk).ok_or(XlsError::Truncated)?;
        let v = read_u16(chunk, self.pos).ok_or(XlsError::Truncated)?;
        self.pos += 2;
        Ok(v)
    }

    /// Read a u32 from the current chunk (no boundary spanning).
    fn read_u32_field(&mut self) -> Result<u32, XlsError> {
        if !self.ensure_chunk() {
            return Err(XlsError::Truncated);
        }
        let chunk = self.chunks.get(self.chunk).ok_or(XlsError::Truncated)?;
        let v = read_u32(chunk, self.pos).ok_or(XlsError::Truncated)?;
        self.pos += 4;
        Ok(v)
    }

    /// Skip `n` bytes, crossing chunk boundaries as needed (used to skip rich
    /// formatting runs and phonetic ext blocks).
    fn skip(&mut self, mut n: usize) -> Result<(), XlsError> {
        while n > 0 {
            if !self.ensure_chunk() {
                return Err(XlsError::Truncated);
            }
            let take = self.cur_remaining().min(n);
            self.pos += take;
            n -= take;
        }
        Ok(())
    }

    /// Read `cch` characters of a string, honouring the CONTINUE gotcha: when a
    /// string's data crosses a chunk boundary, the first byte of the new chunk
    /// is a fresh fHighByte flag for the remainder. `high_byte` is the flag in
    /// force for the first segment.
    fn read_string_chars(
        &mut self,
        mut cch: usize,
        mut high_byte: bool,
    ) -> Result<String, XlsError> {
        let mut out = String::new();
        while cch > 0 {
            if !self.ensure_chunk() {
                return Err(XlsError::Truncated);
            }
            let char_bytes = if high_byte { 2 } else { 1 };
            // How many whole chars can we take from what's left of this chunk?
            let avail = self.cur_remaining();
            let chars_here = (avail / char_bytes).min(cch);
            if chars_here > 0 {
                let take = chars_here * char_bytes;
                let chunk = self.chunks.get(self.chunk).ok_or(XlsError::Truncated)?;
                let data = chunk
                    .get(self.pos..self.pos + take)
                    .ok_or(XlsError::Truncated)?;
                out.push_str(&decode_chars(data, high_byte)?);
                self.pos += take;
                cch -= chars_here;
            }
            if cch == 0 {
                break;
            }
            // We consumed all whole chars we could from this chunk; if a 16-bit
            // char straddles the boundary or the chunk is simply exhausted,
            // move on and read a NEW fHighByte flag for the remainder.
            //
            // Advance past any leftover partial byte AND onto the next chunk.
            self.pos = self.chunks.get(self.chunk).map(|c| c.len()).unwrap_or(0);
            if !self.ensure_chunk() {
                return Err(XlsError::Truncated);
            }
            let grbit = self.read_u8()?;
            high_byte = grbit & 0x01 != 0;
        }
        Ok(out)
    }
}

/// Parse the SST body + its trailing CONTINUE bodies into the string pool.
///
/// The pool is indexed by `LABELSST.isst`. `cst_unique` is the declared unique
/// count; we cap it and stop early if the chunks run out (a truncated file
/// yields as many strings as we could safely decode).
fn parse_sst(chunks: Vec<&[u8]>) -> Result<Vec<String>, XlsError> {
    let mut reader = SstReader::new(chunks);
    let _cst_total = reader.read_u32_field()?;
    let cst_unique = reader.read_u32_field()? as usize;
    if cst_unique > MAX_SST_STRINGS {
        return Err(XlsError::TooLarge);
    }
    // Do NOT pre-allocate `cst_unique` slots — a lying count must not drive
    // allocation. Grow as we actually decode.
    let mut pool = Vec::new();
    for _ in 0..cst_unique {
        // If the chunks are exhausted mid-table, stop cleanly with what we have.
        if !reader.ensure_chunk() {
            break;
        }
        let cch = reader.read_u16()? as usize;
        let grbit = reader.read_u8()?;
        let high_byte = grbit & 0x01 != 0;
        let f_ext_st = grbit & 0x04 != 0; // phonetic data present
        let f_rich_st = grbit & 0x08 != 0; // rich formatting runs present
        // Order matters: cRun (if rich) then cbExtRst (if ext), BEFORE chars.
        let c_run = if f_rich_st { reader.read_u16()? as usize } else { 0 };
        let cb_ext_rst = if f_ext_st { reader.read_u32_field()? as usize } else { 0 };
        // The character data (may span CONTINUE records, flipping fHighByte).
        let text = reader.read_string_chars(cch, high_byte)?;
        // Skip the FormatRun array (cRun × 4 bytes) then the phonetic ext block.
        let run_bytes = c_run.checked_mul(4).ok_or(XlsError::TooLarge)?;
        reader.skip(run_bytes)?;
        reader.skip(cb_ext_rst)?;
        pool.push(text);
    }
    Ok(pool)
}

// ---------------------------------------------------------------------------
// RK number decoding — the packed-number trick.
//
//   bit0  fx100 : if set, divide the decoded number by 100
//   bit1  fInt  : if set, value is a 30-bit signed integer; else truncated f64
//   bits 2..31  : the 30-bit payload
//
// fInt set  → sign-extend the 30-bit payload into an i32.
// fInt clear→ payload is the top 30 bits of an f64 (low 34 bits were zero):
//             f64::from_bits( ((rk & 0xFFFF_FFFC) as u64) << 32 ).
// ---------------------------------------------------------------------------

fn decode_rk(rk: u32) -> f64 {
    let fx100 = rk & 0x01 != 0;
    let f_int = rk & 0x02 != 0;
    let mut value = if f_int {
        // Top 30 bits (rk >> 2) as a SIGNED 30-bit integer: sign-extend bit 29.
        let raw = (rk >> 2) as i32; // 0..=0x3FFF_FFFF
        // If bit 29 (the sign bit of a 30-bit number) is set, subtract 2^30.
        let signed = if raw & 0x2000_0000 != 0 {
            raw - 0x4000_0000
        } else {
            raw
        };
        signed as f64
    } else {
        // Reconstruct the f64 from its top 30 bits.
        f64::from_bits(((rk & 0xFFFF_FFFC) as u64) << 32)
    };
    if fx100 {
        value /= 100.0;
    }
    value
}

// ---------------------------------------------------------------------------
// FORMULA cached-result decoding.
//
// The first 8 bytes of a FORMULA body are the cached result. If bytes [6..8]
// are 0xFFFF it is a SPECIAL value keyed by byte[0]:
//   0 → string (actual text in the FOLLOWING STRING record)
//   1 → boolean (byte[2])
//   2 → error   (byte[2])
//   3 → empty string
// Otherwise the 8 bytes are an IEEE-754 f64.
//
// We return the cached CellValue plus whether a following STRING record supplies
// the text (so the caller can consume it).
// ---------------------------------------------------------------------------

enum FormulaCache {
    Value(CellValue),
    /// The cached result is a string carried by the next STRING record.
    NeedsString,
}

fn decode_formula_cache(result: &[u8]) -> Result<FormulaCache, XlsError> {
    let b = result.get(0..8).ok_or(XlsError::Truncated)?;
    if b[6] == 0xFF && b[7] == 0xFF {
        match b[0] {
            0 => Ok(FormulaCache::NeedsString),
            1 => Ok(FormulaCache::Value(CellValue::Bool(b[2] != 0))),
            2 => Ok(FormulaCache::Value(CellValue::Error(b[2]))),
            _ => Ok(FormulaCache::Value(CellValue::Text(String::new()))), // 3 = empty
        }
    } else {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(b);
        Ok(FormulaCache::Value(CellValue::Number(f64::from_le_bytes(arr))))
    }
}

// ---------------------------------------------------------------------------
// The cell head: u16 row, u16 col, u16 xf. Returns (row, col) or an error.
// ---------------------------------------------------------------------------

fn read_cell_head(body: &[u8]) -> Result<(u32, u32), XlsError> {
    let row = read_u16(body, 0).ok_or(XlsError::Truncated)? as u32;
    let col = read_u16(body, 2).ok_or(XlsError::Truncated)? as u32;
    // xf at [4..6] is intentionally ignored.
    Ok((row, col))
}

// ---------------------------------------------------------------------------
// Top-level entry point.
// ---------------------------------------------------------------------------

/// Open a legacy `.xls` workbook from its raw bytes.
///
/// Opens the OLE2 container with [`cfb`], reads the `Workbook` stream (falling
/// back to `Book` for very old files), and parses the BIFF record stream into a
/// typed [`Workbook`].
pub fn open_xls(bytes: &[u8]) -> Result<Workbook, XlsError> {
    let cf = cfb::CompoundFile::open(bytes)?;
    // BIFF8 names the stream "Workbook"; BIFF5 and earlier use "Book".
    let stream = cf
        .read_stream("Workbook")
        .or_else(|| cf.read_stream("Book"))
        .ok_or(XlsError::NoWorkbookStream)?;
    parse_workbook_stream(&stream)
}

/// Parse a raw BIFF record stream (the contents of the Workbook/Book stream).
///
/// This is factored out so tests can feed a synthetic stream directly without
/// building a whole CFB container.
fn parse_workbook_stream(stream: &[u8]) -> Result<Workbook, XlsError> {
    let records = parse_records(stream);

    // ----- Pass 1: the GLOBALS substream — SST + sheet directory. -----------
    //
    // We collect the shared strings and the BOUNDSHEET directory. Each
    // BOUNDSHEET gives (name, lbPlyPos = byte offset of that sheet's BOF).
    let mut sst: Vec<String> = Vec::new();
    // (lbPlyPos, name) in sheet order.
    let mut boundsheets: Vec<(u32, String)> = Vec::new();

    // Find the globals substream: the first BOF whose kind is 0x0005. We scan
    // records tracking the current substream kind.
    let mut i = 0usize;
    let mut in_globals = false;
    while i < records.len() {
        let rec = &records[i];
        match rec.rec_type {
            REC_BOF => {
                let kind = read_u16(rec.body, 2).unwrap_or(0);
                in_globals = kind == SUB_GLOBALS;
            }
            REC_EOF => {
                if in_globals {
                    // Globals substream done; stop pass 1.
                    break;
                }
            }
            REC_BOUNDSHEET if in_globals => {
                if let Some((pos, name)) = parse_boundsheet(rec.body)? {
                    boundsheets.push((pos, name));
                }
            }
            REC_SST if in_globals => {
                // Gather the SST body plus any immediately-following CONTINUE
                // records into the record-spanning reader.
                let mut chunks: Vec<&[u8]> = vec![rec.body];
                let mut j = i + 1;
                let mut continues = 0usize;
                while j < records.len() && records[j].rec_type == REC_CONTINUE {
                    chunks.push(records[j].body);
                    j += 1;
                    continues += 1;
                    if continues > MAX_CONTINUE_RECORDS {
                        return Err(XlsError::TooLarge);
                    }
                }
                sst = parse_sst(chunks)?;
                // Skip the CONTINUE records we just consumed.
                i = j;
                continue;
            }
            _ => {}
        }
        i += 1;
    }

    // ----- Pass 2: each WORKSHEET substream → that sheet's cells. ------------
    //
    // We walk substreams again. When we enter a worksheet BOF, we match its
    // header offset against the collected lbPlyPos values to recover its name,
    // then collect cells until the matching EOF.
    let mut sheets_by_pos: Vec<(u32, Sheet)> = Vec::new();

    let mut i = 0usize;
    while i < records.len() {
        let rec = &records[i];
        if rec.rec_type == REC_BOF {
            let kind = read_u16(rec.body, 2).unwrap_or(0);
            if kind == SUB_WORKSHEET {
                let bof_offset = rec.header_offset as u32;
                // Name = the boundsheet whose lbPlyPos == this BOF offset.
                let name = boundsheets
                    .iter()
                    .find(|(pos, _)| *pos == bof_offset)
                    .map(|(_, n)| n.clone())
                    .unwrap_or_default();
                let mut cells = Vec::new();
                i += 1;
                // Collect cells until EOF (or end of records).
                while i < records.len() && records[i].rec_type != REC_EOF {
                    let (next, mut new_cells) = parse_cell_record(&records, i, &sst)?;
                    cells.append(&mut new_cells);
                    i = next;
                }
                sheets_by_pos.push((bof_offset, Sheet { name, cells }));
                // `i` now sits on the EOF (or end); continue outer loop.
                continue;
            }
        }
        i += 1;
    }

    // Order sheets by the BOUNDSHEET order (the order they were declared). Any
    // worksheet substream we could not name (unmatched offset) keeps its record
    // order at the end.
    let mut sheets = Vec::new();
    for (pos, _name) in &boundsheets {
        if let Some(idx) = sheets_by_pos.iter().position(|(p, _)| p == pos) {
            let (_, sheet) = sheets_by_pos.remove(idx);
            sheets.push(sheet);
        }
    }
    // Append any leftover (unmatched) worksheet substreams in record order.
    for (_, sheet) in sheets_by_pos {
        sheets.push(sheet);
    }

    Ok(Workbook { sheets })
}

/// Parse a BOUNDSHEET body → (lbPlyPos, name). Returns Ok(None) only on a body
/// too short to hold the fixed header (skipped permissively).
fn parse_boundsheet(body: &[u8]) -> Result<Option<(u32, String)>, XlsError> {
    let Some(lb_ply_pos) = read_u32(body, 0) else {
        return Ok(None);
    };
    // body[4] = hsState (visibility), body[5] = dt (type) — retained-but-unused.
    let cch = *body.get(6).ok_or(XlsError::Truncated)? as usize;
    // ShortXLUnicodeString: grbit at [7], chars from [8].
    let (name, _) = read_flat_string(body, 7, cch)?;
    Ok(Some((lb_ply_pos, name)))
}

/// Parse one cell-bearing record at `records[i]`, returning the index to resume
/// at and the cells it produced (usually one; MULRK produces several; FORMULA
/// may consume a following STRING record).
fn parse_cell_record(
    records: &[Record<'_>],
    i: usize,
    sst: &[String],
) -> Result<(usize, Vec<Cell>), XlsError> {
    let rec = &records[i];
    let body = rec.body;
    let mut out = Vec::new();
    match rec.rec_type {
        REC_LABELSST => {
            let (row, col) = read_cell_head(body)?;
            let isst = read_u32(body, 6).ok_or(XlsError::Truncated)? as usize;
            let text = sst.get(isst).cloned().unwrap_or_default();
            out.push(Cell {
                row,
                col,
                value: CellValue::Text(text),
            });
        }
        REC_RK => {
            let (row, col) = read_cell_head(body)?;
            let rk = read_u32(body, 6).ok_or(XlsError::Truncated)?;
            out.push(Cell {
                row,
                col,
                value: CellValue::Number(decode_rk(rk)),
            });
        }
        REC_MULRK => {
            // row, colFirst, then N×(xf, rk), then colLast.
            let row = read_u16(body, 0).ok_or(XlsError::Truncated)? as u32;
            let col_first = read_u16(body, 2).ok_or(XlsError::Truncated)? as usize;
            // colLast lives at the very end of the body.
            let last_off = body.len().checked_sub(2).ok_or(XlsError::Truncated)?;
            let col_last = read_u16(body, last_off).ok_or(XlsError::Truncated)? as usize;
            // A malformed run (colLast < colFirst) yields zero cells (checked).
            if let Some(span) = col_last.checked_sub(col_first).map(|d| d + 1) {
                if span > MAX_MULRK_SPAN {
                    return Err(XlsError::TooLarge);
                }
                // Each entry is 6 bytes (u16 xf + u32 rk), starting at offset 4.
                for k in 0..span {
                    let entry_off = 4 + k * 6; // both bounded by span cap
                    let rk = read_u32(body, entry_off + 2).ok_or(XlsError::Truncated)?;
                    let col = (col_first + k) as u32;
                    out.push(Cell {
                        row,
                        col,
                        value: CellValue::Number(decode_rk(rk)),
                    });
                }
            }
        }
        REC_NUMBER => {
            let (row, col) = read_cell_head(body)?;
            let v = read_f64(body, 6).ok_or(XlsError::Truncated)?;
            out.push(Cell {
                row,
                col,
                value: CellValue::Number(v),
            });
        }
        REC_LABEL => {
            let (row, col) = read_cell_head(body)?;
            // XLUnicodeString: u16 cch at [6], grbit at [8], chars from [9].
            let cch = read_u16(body, 6).ok_or(XlsError::Truncated)? as usize;
            let (text, _) = read_flat_string(body, 8, cch)?;
            out.push(Cell {
                row,
                col,
                value: CellValue::Text(text),
            });
        }
        REC_BOOLERR => {
            let (row, col) = read_cell_head(body)?;
            let value = *body.get(6).ok_or(XlsError::Truncated)?;
            let f_error = *body.get(7).ok_or(XlsError::Truncated)?;
            let cv = if f_error != 0 {
                CellValue::Error(value)
            } else {
                CellValue::Bool(value != 0)
            };
            out.push(Cell { row, col, value: cv });
        }
        REC_BLANK => {
            let (row, col) = read_cell_head(body)?;
            out.push(Cell {
                row,
                col,
                value: CellValue::Blank,
            });
        }
        REC_FORMULA => {
            let (row, col) = read_cell_head(body)?;
            // Cached result is body[6..14].
            let result = body.get(6..14).ok_or(XlsError::Truncated)?;
            let cached = match decode_formula_cache(result)? {
                FormulaCache::Value(v) => v,
                FormulaCache::NeedsString => {
                    // The text is in the FOLLOWING STRING (0x0207) record.
                    if let Some(next) = records.get(i + 1) {
                        if next.rec_type == REC_STRING {
                            let cch =
                                read_u16(next.body, 0).ok_or(XlsError::Truncated)? as usize;
                            // grbit at [2], chars from [3].
                            let (text, _) = read_flat_string(next.body, 2, cch)?;
                            let cell = Cell {
                                row,
                                col,
                                value: CellValue::Formula {
                                    cached: Box::new(CellValue::Text(text)),
                                },
                            };
                            return Ok((i + 2, vec![cell]));
                        }
                    }
                    CellValue::Text(String::new())
                }
            };
            out.push(Cell {
                row,
                col,
                value: CellValue::Formula {
                    cached: Box::new(cached),
                },
            });
        }
        // Any other record inside a worksheet substream is skipped.
        _ => {}
    }
    Ok((i + 1, out))
}

#[cfg(test)]
mod fixture;

#[cfg(test)]
mod tests;
