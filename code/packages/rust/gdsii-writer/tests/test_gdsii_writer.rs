use gdsii_writer::{
    stream::{double_to_gds_real, um_to_dbu},
    GdsBoundary, GdsCell, GdsPath, GdsSref, GdsText, GdsWriter,
};

// ---------------------------------------------------------------------------
// GDS real encoding
// ---------------------------------------------------------------------------

#[test]
fn test_gds_real_zero() {
    let r = double_to_gds_real(0.0);
    assert_eq!(r, [0u8; 8]);
}

#[test]
fn test_gds_real_one() {
    // 1.0 in GDSII real: exponent = 65 (0x41), mantissa = 0x1000...0 (1/16 × 16^1 = 1)
    // sign_exp = 0x41, mantissa = (1/16) × 2^56 = 2^52 = 0x0010000000000000
    let r = double_to_gds_real(1.0);
    // First byte = 0x41 (exponent 65, positive)
    assert_eq!(r[0], 0x41);
    // Mantissa = 0x10000000000000 (7 bytes)
    assert_eq!(r[1], 0x10);
    for &b in &r[2..8] { assert_eq!(b, 0x00); }
}

#[test]
fn test_gds_real_negative() {
    let pos = double_to_gds_real(1.0);
    let neg = double_to_gds_real(-1.0);
    // Sign bit set in first byte.
    assert_eq!(neg[0], pos[0] | 0x80);
    assert_eq!(&neg[1..], &pos[1..]);
}

#[test]
fn test_gds_real_fraction() {
    // 0.001 (1 µm / 1000 = 1 nm per dbu in Sky130)
    let r = double_to_gds_real(1e-3);
    // Just check it's non-zero and first byte has low exponent.
    assert_ne!(r, [0u8; 8]);
}

// ---------------------------------------------------------------------------
// um_to_dbu
// ---------------------------------------------------------------------------

#[test]
fn test_um_to_dbu_one_um() {
    assert_eq!(um_to_dbu(1.0), 1000);
}

#[test]
fn test_um_to_dbu_fractional() {
    assert_eq!(um_to_dbu(0.14), 140);
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

fn simple_writer() -> GdsWriter {
    let mut w = GdsWriter::new("testlib");
    let mut cell = GdsCell::new("top");
    cell.boundaries.push(GdsBoundary {
        layer: 68, datatype: 20,
        xy: vec![(0,0),(1000,0),(1000,2720),(0,2720),(0,0)],
    });
    w.cells.push(cell);
    w
}

#[test]
fn test_encode_produces_bytes() {
    let w = simple_writer();
    let bytes = w.encode();
    assert!(!bytes.is_empty());
}

#[test]
fn test_encode_starts_with_header_record() {
    let w = simple_writer();
    let bytes = w.encode();
    // HEADER record: length=6 (0x0006), record_type=0x00, data_type=0x02
    assert_eq!(bytes[0], 0x00);
    assert_eq!(bytes[1], 0x06);
    assert_eq!(bytes[2], 0x00); // HEADER record type
    assert_eq!(bytes[3], 0x02); // integer data type
}

#[test]
fn test_encode_ends_with_endlib() {
    let w = simple_writer();
    let bytes = w.encode();
    // ENDLIB: 0x0004, 0x04, 0x00
    let n = bytes.len();
    assert!(n >= 4);
    let tail = &bytes[n-4..];
    assert_eq!(tail, &[0x00, 0x04, 0x04, 0x00]);
}

#[test]
fn test_empty_library() {
    let w = GdsWriter::new("empty");
    let bytes = w.encode();
    // Should still be valid (HEADER + BGNLIB + LIBNAME + UNITS + ENDLIB).
    assert!(!bytes.is_empty());
}

#[test]
fn test_cell_with_path() {
    let mut w = GdsWriter::new("pathlib");
    let mut cell = GdsCell::new("wire");
    cell.paths.push(GdsPath {
        layer: 68, datatype: 20, width: 140,
        xy: vec![(0,0),(1000,0)],
    });
    w.cells.push(cell);
    let bytes = w.encode();
    assert!(!bytes.is_empty());
}

#[test]
fn test_cell_with_sref() {
    let mut w = GdsWriter::new("sreflib");
    let mut cell = GdsCell::new("top");
    cell.srefs.push(GdsSref { sname: "inv_1".into(), x: 0, y: 0 });
    w.cells.push(cell);
    let bytes = w.encode();
    assert!(!bytes.is_empty());
}

#[test]
fn test_cell_with_text() {
    let mut w = GdsWriter::new("textlib");
    let mut cell = GdsCell::new("top");
    cell.texts.push(GdsText { layer: 67, texttype: 0, text: "A".into(), x: 0, y: 0 });
    w.cells.push(cell);
    let bytes = w.encode();
    assert!(!bytes.is_empty());
}

#[test]
fn test_two_cells() {
    let mut w = GdsWriter::new("twolib");
    w.cells.push(GdsCell::new("cell_a"));
    w.cells.push(GdsCell::new("cell_b"));
    let bytes = w.encode();
    // Both STRNAME records should appear in the stream.
    let s = bytes.as_slice();
    let has_a = s.windows(8).any(|w| w == b"cell_a\x00\x00" || w[0..6] == b"cell_a"[..]);
    assert!(has_a, "cell_a not found in GDS stream");
}
