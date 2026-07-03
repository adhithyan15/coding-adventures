//! Tests for `xls-writer`.
//!
//! The headline test is [`round_trip_revenue_sheet`]: it writes a real `.xls`,
//! re-opens it with the `cfb` reader, extracts the `Workbook` stream, and walks
//! its BIFF records to prove every field — sheet name, `lbPlyPos` offsets, SST
//! contents, and each cell's type/address/value — survives the trip
//! `model → BIFF → CFB → cfb reader → BIFF → model`.

use super::*;
use cfb::CompoundFile;

// ---------------------------------------------------------------------------
// A tiny in-test BIFF walker. Real code never needs to *read* BIFF (that's the
// eventual xls-reader's job); the test walks records itself to keep the proof
// self-contained and dependency-free.
// ---------------------------------------------------------------------------

/// One parsed BIFF record: its type and a copy of its body bytes.
#[derive(Debug, Clone)]
struct BiffRecord {
    record_type: u16,
    /// Absolute byte offset of this record's *header* in the stream.
    offset: usize,
    body: Vec<u8>,
}

/// Walk a `Workbook` byte-stream into a flat list of BIFF records. Stops cleanly
/// at the first truncated header/body (never panics).
fn walk_biff(stream: &[u8]) -> Vec<BiffRecord> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 4 <= stream.len() {
        let record_type = u16::from_le_bytes([stream[i], stream[i + 1]]);
        let size = u16::from_le_bytes([stream[i + 2], stream[i + 3]]) as usize;
        let body_start = i + 4;
        let body_end = body_start + size;
        if body_end > stream.len() {
            break; // truncated — stop rather than read past the end
        }
        out.push(BiffRecord {
            record_type,
            offset: i,
            body: stream[body_start..body_end].to_vec(),
        });
        i = body_end;
    }
    out
}

/// Little-endian readers over a record body, for assertions.
fn le_u16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}
fn le_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}
fn le_f64(b: &[u8], off: usize) -> f64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[off..off + 8]);
    f64::from_le_bytes(a)
}

/// Decode a `ShortXLUnicodeString` at the given offset (u8 cch, u8 grbit, chars).
fn read_short_string(b: &[u8], off: usize) -> String {
    let cch = b[off] as usize;
    let high_byte = b[off + 1] & 0x01 == 1;
    let chars = &b[off + 2..];
    decode_chars(chars, cch, high_byte)
}

/// Decode `cch` characters from `chars` in either 8-bit or 16-bit form.
fn decode_chars(chars: &[u8], cch: usize, high_byte: bool) -> String {
    if high_byte {
        let mut units = Vec::with_capacity(cch);
        for k in 0..cch {
            let j = k * 2;
            if j + 1 < chars.len() {
                units.push(u16::from_le_bytes([chars[j], chars[j + 1]]));
            }
        }
        String::from_utf16_lossy(&units)
    } else {
        // Compressed: each byte is the low byte of a code unit.
        let units: Vec<u16> = chars[..cch.min(chars.len())].iter().map(|&c| c as u16).collect();
        String::from_utf16_lossy(&units)
    }
}

/// Parse the SST body into the list of distinct strings, honouring the per-string
/// `fHighByte` flag. Assumes no rich/ext trailers (which this writer never emits).
fn parse_sst(body: &[u8]) -> (u32, u32, Vec<String>) {
    let cst_total = le_u32(body, 0);
    let cst_unique = le_u32(body, 4);
    let mut strings = Vec::new();
    let mut i = 8usize;
    while i + 3 <= body.len() && strings.len() < cst_unique as usize {
        let cch = le_u16(body, i) as usize;
        let high_byte = body[i + 2] & 0x01 == 1;
        let chars_off = i + 3;
        let char_bytes = if high_byte { cch * 2 } else { cch };
        if chars_off + char_bytes > body.len() {
            break;
        }
        strings.push(decode_chars(&body[chars_off..], cch, high_byte));
        i = chars_off + char_bytes;
    }
    (cst_total, cst_unique, strings)
}

/// Extract the `Workbook` stream from written `.xls` bytes via the cfb reader.
fn open_workbook_stream(xls: &[u8]) -> Vec<u8> {
    let cf = CompoundFile::open(xls).expect("written .xls should parse as CFB");
    cf.read_stream("Workbook")
        .expect("written .xls should contain a Workbook stream")
}

// ---------------------------------------------------------------------------
// THE round-trip proof.
// ---------------------------------------------------------------------------

#[test]
fn round_trip_revenue_sheet() {
    // Build the model: one sheet "Revenue" with four cells.
    let mut wb = Workbook::new();
    let sheet = wb.add_sheet("Revenue");
    sheet.set_string(0, 0, "Q1");
    sheet.set_number(0, 1, 1000.0);
    sheet.set_string(1, 0, "Total");
    sheet.set_number(1, 1, 1234.5);

    let xls = write_xls(&wb);

    // It's a real CFB (OLE2 magic).
    assert_eq!(&xls[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);

    let stream = open_workbook_stream(&xls);
    let records = walk_biff(&stream);

    // ---- BOUNDSHEET: exactly one, named "Revenue", lbPlyPos → worksheet BOF --
    let boundsheets: Vec<&BiffRecord> =
        records.iter().filter(|r| r.record_type == REC_BOUNDSHEET).collect();
    assert_eq!(boundsheets.len(), 1, "expected exactly one BOUNDSHEET");
    let bs = boundsheets[0];
    let lb_ply_pos = le_u32(&bs.body, 0) as usize;
    // hsState=0, dt=0, then name at offset 6.
    assert_eq!(bs.body[4], 0, "hsState should be visible");
    assert_eq!(bs.body[5], 0, "dt should be worksheet");
    assert_eq!(read_short_string(&bs.body, 6), "Revenue");

    // The lbPlyPos must land exactly on a BOF record whose dt = worksheet.
    let bof_at_offset = records
        .iter()
        .find(|r| r.offset == lb_ply_pos)
        .expect("lbPlyPos should point at the start of some record");
    assert_eq!(bof_at_offset.record_type, REC_BOF, "lbPlyPos should point at a BOF");
    let dt = le_u16(&bof_at_offset.body, 2);
    assert_eq!(dt, DT_WORKSHEET, "the pointed-at BOF must be a worksheet BOF");

    // ---- SST: contains "Q1" and "Total" -----------------------------------
    let sst = records
        .iter()
        .find(|r| r.record_type == REC_SST)
        .expect("there should be an SST record");
    let (cst_total, cst_unique, strings) = parse_sst(&sst.body);
    assert_eq!(cst_total, 2, "two string cells");
    assert_eq!(cst_unique, 2, "two distinct strings");
    assert!(strings.iter().any(|s| s == "Q1"), "SST should contain Q1: {strings:?}");
    assert!(strings.iter().any(|s| s == "Total"), "SST should contain Total");

    // Map string -> isst for cell checks.
    let isst_of = |want: &str| -> u32 {
        strings.iter().position(|s| s == want).expect("string present") as u32
    };

    // ---- Cells: walk the worksheet substream -------------------------------
    // Gather LABELSST and NUMBER records (they only appear inside the worksheet).
    let labelssts: Vec<&BiffRecord> =
        records.iter().filter(|r| r.record_type == REC_LABELSST).collect();
    let numbers: Vec<&BiffRecord> =
        records.iter().filter(|r| r.record_type == REC_NUMBER).collect();
    assert_eq!(labelssts.len(), 2);
    assert_eq!(numbers.len(), 2);

    // Helper: find a LABELSST at (row, col) and return its isst.
    let find_label = |row: u16, col: u16| -> u32 {
        let r = labelssts
            .iter()
            .find(|r| le_u16(&r.body, 0) == row && le_u16(&r.body, 2) == col)
            .unwrap_or_else(|| panic!("no LABELSST at ({row},{col})"));
        le_u32(&r.body, 6)
    };
    let find_number = |row: u16, col: u16| -> f64 {
        let r = numbers
            .iter()
            .find(|r| le_u16(&r.body, 0) == row && le_u16(&r.body, 2) == col)
            .unwrap_or_else(|| panic!("no NUMBER at ({row},{col})"));
        le_f64(&r.body, 6)
    };

    // (0,0)="Q1", (0,1)=1000.0, (1,0)="Total", (1,1)=1234.5.
    assert_eq!(find_label(0, 0), isst_of("Q1"));
    assert_eq!(find_number(0, 1), 1000.0);
    assert_eq!(find_label(1, 0), isst_of("Total"));
    assert_eq!(find_number(1, 1), 1234.5);
}

// ---------------------------------------------------------------------------
// Unit tests.
// ---------------------------------------------------------------------------

#[test]
fn sst_dedups_identical_strings() {
    let mut wb = Workbook::new();
    let s = wb.add_sheet("S");
    s.set_string(0, 0, "same");
    s.set_string(1, 0, "same");

    let stream = open_workbook_stream(&write_xls(&wb));
    let records = walk_biff(&stream);
    let sst = records.iter().find(|r| r.record_type == REC_SST).unwrap();
    let (cst_total, cst_unique, strings) = parse_sst(&sst.body);
    assert_eq!(cst_total, 2, "cstTotal counts every string cell");
    assert_eq!(cst_unique, 1, "cstUnique counts distinct strings");
    assert_eq!(strings, vec!["same".to_string()]);

    // Both LABELSST records reference the same (index 0) entry.
    let labels: Vec<&BiffRecord> =
        records.iter().filter(|r| r.record_type == REC_LABELSST).collect();
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().all(|r| le_u32(&r.body, 6) == 0));
}

#[test]
fn non_ascii_forces_wide_encoding() {
    // "sun ☃" contains U+2603 (> 0xFF) → the whole string must use the 16-bit
    // (fHighByte) encoding and still decode correctly.
    let mut wb = Workbook::new();
    wb.add_sheet("S").set_string(0, 0, "sun ☃");

    let stream = open_workbook_stream(&write_xls(&wb));
    let records = walk_biff(&stream);
    let sst = records.iter().find(|r| r.record_type == REC_SST).unwrap();
    // The single string's grbit (byte at body[8+2]) must have fHighByte set.
    assert_eq!(sst.body[10] & 0x01, 0x01, "non-Latin1 string must set fHighByte");
    let (_total, _unique, strings) = parse_sst(&sst.body);
    assert_eq!(strings, vec!["sun ☃".to_string()]);
}

#[test]
fn latin1_string_uses_compressed_encoding() {
    // "café" — every code unit (incl. é = U+00E9) is ≤ 0xFF → compressed 8-bit.
    let mut wb = Workbook::new();
    wb.add_sheet("S").set_string(0, 0, "café");
    let stream = open_workbook_stream(&write_xls(&wb));
    let records = walk_biff(&stream);
    let sst = records.iter().find(|r| r.record_type == REC_SST).unwrap();
    assert_eq!(sst.body[10] & 0x01, 0x00, "Latin1 string should be compressed");
    let (_t, _u, strings) = parse_sst(&sst.body);
    assert_eq!(strings, vec!["café".to_string()]);
}

#[test]
fn multiple_sheets_get_distinct_lbplypos() {
    let mut wb = Workbook::new();
    wb.add_sheet("Alpha").set_number(0, 0, 1.0);
    wb.add_sheet("Beta").set_number(0, 0, 2.0);
    wb.add_sheet("Gamma").set_string(0, 0, "g");

    let stream = open_workbook_stream(&write_xls(&wb));
    let records = walk_biff(&stream);

    let boundsheets: Vec<&BiffRecord> =
        records.iter().filter(|r| r.record_type == REC_BOUNDSHEET).collect();
    assert_eq!(boundsheets.len(), 3);

    // Names in order.
    assert_eq!(read_short_string(&boundsheets[0].body, 6), "Alpha");
    assert_eq!(read_short_string(&boundsheets[1].body, 6), "Beta");
    assert_eq!(read_short_string(&boundsheets[2].body, 6), "Gamma");

    // Each lbPlyPos points at a distinct worksheet BOF, in increasing order.
    let mut positions = Vec::new();
    for bs in &boundsheets {
        let pos = le_u32(&bs.body, 0) as usize;
        let target = records
            .iter()
            .find(|r| r.offset == pos)
            .expect("lbPlyPos should hit a record boundary");
        assert_eq!(target.record_type, REC_BOF);
        assert_eq!(le_u16(&target.body, 2), DT_WORKSHEET);
        positions.push(pos);
    }
    // Distinct and strictly increasing (sheets are laid out in order).
    assert!(positions[0] < positions[1] && positions[1] < positions[2]);
}

#[test]
fn empty_workbook_does_not_panic() {
    let wb = Workbook::new();
    let xls = write_xls(&wb);
    // Still a valid CFB with a (tiny) Workbook stream: globals BOF + EOF + SST.
    let stream = open_workbook_stream(&xls);
    let records = walk_biff(&stream);
    assert_eq!(records.first().map(|r| r.record_type), Some(REC_BOF));
    assert!(records.iter().any(|r| r.record_type == REC_EOF));
}

#[test]
fn empty_sheet_does_not_panic() {
    let mut wb = Workbook::new();
    wb.add_sheet("Empty"); // a sheet with no cells
    let stream = open_workbook_stream(&write_xls(&wb));
    let records = walk_biff(&stream);
    // BOUNDSHEET present, worksheet substream is just BOF then EOF.
    let bs = records.iter().find(|r| r.record_type == REC_BOUNDSHEET).unwrap();
    assert_eq!(read_short_string(&bs.body, 6), "Empty");
    let ply = le_u32(&bs.body, 0) as usize;
    let bof = records.iter().find(|r| r.offset == ply).unwrap();
    assert_eq!(bof.record_type, REC_BOF);
    assert_eq!(le_u16(&bof.body, 2), DT_WORKSHEET);
}

#[test]
fn out_of_range_cell_is_skipped() {
    // A column beyond u16::MAX cannot be represented; the cell is skipped, and a
    // valid neighbour still appears.
    let mut wb = Workbook::new();
    let s = wb.add_sheet("S");
    s.set_number(0, 70_000, 9.0); // col > 65535 → skipped
    s.set_number(0, 5, 7.0); // valid
    let stream = open_workbook_stream(&write_xls(&wb));
    let records = walk_biff(&stream);
    let numbers: Vec<&BiffRecord> =
        records.iter().filter(|r| r.record_type == REC_NUMBER).collect();
    assert_eq!(numbers.len(), 1, "the out-of-range cell must be skipped");
    assert_eq!(le_u16(&numbers[0].body, 2), 5); // col field is at body offset 2
    assert_eq!(le_f64(&numbers[0].body, 6), 7.0);
}

#[test]
fn number_record_preserves_exact_f64() {
    // A value with a full mantissa (would NOT fit RK) must round-trip exactly via
    // the NUMBER record.
    let mut wb = Workbook::new();
    wb.add_sheet("S").set_number(3, 4, std::f64::consts::PI);
    let stream = open_workbook_stream(&write_xls(&wb));
    let records = walk_biff(&stream);
    let n = records.iter().find(|r| r.record_type == REC_NUMBER).unwrap();
    assert_eq!(le_u16(&n.body, 0), 3);
    assert_eq!(le_u16(&n.body, 2), 4);
    assert_eq!(le_f64(&n.body, 6), std::f64::consts::PI);
}

#[test]
fn output_is_deterministic() {
    let build = || {
        let mut wb = Workbook::new();
        let s = wb.add_sheet("D");
        s.set_string(0, 0, "x");
        s.set_number(0, 1, 2.5);
        write_xls(&wb)
    };
    assert_eq!(build(), build(), "identical models must produce identical bytes");
}
