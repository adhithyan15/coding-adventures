//! Tests for the `doc` reader.
//!
//! We test at two levels:
//!
//! * **End-to-end** against the real CFB-wrapped fixture (`open_doc`).
//! * **Unit level** against the `extract_text` seam and its helpers, using
//!   hand-built synthetic buffers so the piece-table logic is exercised without
//!   constructing a whole compound file.
//!
//! The synthetic builders below mirror the on-disk layouts documented in
//! `lib.rs`, so reading them is itself a tutorial in the formats.

use super::*;
use crate::fixture;

// ---------------------------------------------------------------------------
// Synthetic-buffer builders
// ---------------------------------------------------------------------------

/// Build a `PlcPcd`: an (n+1)-entry CP array followed by an n-entry PCD array.
///
/// `cps` is the list of character positions (length must be pieces.len()+1).
/// `fcs` is one `FcCompressed` value per piece.
fn build_plc_pcd(cps: &[u32], fcs: &[u32]) -> Vec<u8> {
    assert_eq!(cps.len(), fcs.len() + 1, "need n+1 CPs for n pieces");
    let mut v = Vec::new();
    for &cp in cps {
        v.extend_from_slice(&cp.to_le_bytes());
    }
    for &fc in fcs {
        // PCD = u16 flags(0) + u32 FcCompressed + u16 prm(0) = 8 bytes.
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&fc.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
    }
    v
}

/// Wrap a `PlcPcd` in a single `Pcdt` CLX part (tag 0x02 + u32 lcb + data).
fn build_clx_pcdt(plc_pcd: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.push(CLXT_PCDT);
    v.extend_from_slice(&(plc_pcd.len() as u32).to_le_bytes());
    v.extend_from_slice(plc_pcd);
    v
}

/// Build a `Prc` CLX part (tag 0x01 + i16 cbGrpprl + cbGrpprl bytes).
fn build_clx_prc(payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.push(CLXT_PRC);
    v.extend_from_slice(&(payload.len() as i16).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// Compute the `FcCompressed` value for a compressed (8-bit) piece whose bytes
/// begin at `word_document_offset`. The stored offset is `2 * off` and bit 30 set.
fn fc_compressed(word_document_offset: u32) -> u32 {
    FC_COMPRESSED_BIT | (word_document_offset * 2)
}

/// Compute the `FcCompressed` value for an uncompressed (UTF-16) piece whose
/// bytes begin at `word_document_offset`. Bit 30 clear, offset stored directly.
fn fc_uncompressed(word_document_offset: u32) -> u32 {
    word_document_offset & FC_OFFSET_MASK
}

// ---------------------------------------------------------------------------
// End-to-end: the required fixture test
// ---------------------------------------------------------------------------

#[test]
fn fixture_decodes_to_hello_doc() {
    let d = open_doc(fixture::MINIMAL_DOC).expect("open doc");
    assert_eq!(d.text(), "Hello, DOC!");
}

#[test]
fn document_is_debug_and_clone() {
    let d = open_doc(fixture::MINIMAL_DOC).expect("open doc");
    let cloned = d.clone();
    assert_eq!(cloned.text(), "Hello, DOC!");
    // Debug should render without panicking.
    let _ = format!("{d:?}");
}

// ---------------------------------------------------------------------------
// FcCompressed decode — compressed (8-bit) single piece
// ---------------------------------------------------------------------------

#[test]
fn single_compressed_piece() {
    // WordDocument: "Hi" at offset 0.
    let word = b"Hi".to_vec();
    let plc = build_plc_pcd(&[0, 2], &[fc_compressed(0)]);
    let clx = build_clx_pcdt(&plc);
    // Table = the CLX at fc_clx=0.
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "Hi");
}

#[test]
fn compressed_piece_at_nonzero_offset() {
    // Text sits partway into WordDocument; FcCompressed offset must locate it.
    let mut word = vec![0u8; 10];
    word.extend_from_slice(b"XY"); // at offset 10
    let plc = build_plc_pcd(&[0, 2], &[fc_compressed(10)]);
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "XY");
}

#[test]
fn compressed_piece_decodes_latin1_high_byte() {
    // Byte 0xE9 is Latin-1 'é' (U+00E9).
    let word = vec![0xE9u8];
    let plc = build_plc_pcd(&[0, 1], &[fc_compressed(0)]);
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "\u{00E9}");
}

// ---------------------------------------------------------------------------
// FcCompressed decode — uncompressed (UTF-16) single piece
// ---------------------------------------------------------------------------

#[test]
fn single_uncompressed_utf16_piece() {
    // "Hi" as UTF-16LE = 48 00 69 00.
    let word = vec![0x48, 0x00, 0x69, 0x00];
    let plc = build_plc_pcd(&[0, 2], &[fc_uncompressed(0)]);
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "Hi");
}

#[test]
fn uncompressed_piece_decodes_surrogate_pair() {
    // U+1F600 GRINNING FACE = surrogate pair D83D DE00, LE bytes 3D D8 00 DE.
    let word = vec![0x3D, 0xD8, 0x00, 0xDE];
    // Two UTF-16 code units = 2 "characters" in the CP model.
    let plc = build_plc_pcd(&[0, 2], &[fc_uncompressed(0)]);
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "\u{1F600}");
}

#[test]
fn uncompressed_lone_surrogate_becomes_replacement() {
    // Lone high surrogate D83D -> replacement char, no panic.
    let word = vec![0x3D, 0xD8];
    let plc = build_plc_pcd(&[0, 1], &[fc_uncompressed(0)]);
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "\u{FFFD}");
}

// ---------------------------------------------------------------------------
// Multi-piece concatenation in CP order
// ---------------------------------------------------------------------------

#[test]
fn multi_piece_concatenates_in_order() {
    // WordDocument holds two 8-bit runs plus one UTF-16 run.
    // Layout: "AB" @0 (compressed), then UTF-16 "cd" @8, then "E" @2 (compressed).
    let mut word = Vec::new();
    word.extend_from_slice(b"AB"); // offset 0..2  -> piece uses offset 0
    word.extend_from_slice(b"E?"); // offset 2..4  -> compressed 'E' at offset 2
    word.extend_from_slice(&[0, 0, 0, 0]); // padding to reach offset 8
    word.extend_from_slice(&[0x63, 0x00, 0x64, 0x00]); // "cd" UTF-16 at offset 8

    // Three pieces: [0,2)="AB", [2,4)="cd" (2 chars), [4,5)="E".
    let cps = [0u32, 2, 4, 5];
    let fcs = [
        fc_compressed(0),   // "AB"
        fc_uncompressed(8), // "cd"
        fc_compressed(2),   // "E"
    ];
    let plc = build_plc_pcd(&cps, &fcs);
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "ABcdE");
}

// ---------------------------------------------------------------------------
// Prc-before-Pcdt skipping
// ---------------------------------------------------------------------------

#[test]
fn prc_part_is_skipped_before_pcdt() {
    let word = b"Hi".to_vec();
    let plc = build_plc_pcd(&[0, 2], &[fc_compressed(0)]);

    // One Prc (5 arbitrary bytes) then the Pcdt.
    let mut clx = build_clx_prc(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE]);
    clx.extend_from_slice(&build_clx_pcdt(&plc));

    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "Hi");
}

#[test]
fn multiple_prc_parts_are_skipped() {
    let word = b"Hi".to_vec();
    let plc = build_plc_pcd(&[0, 2], &[fc_compressed(0)]);
    let mut clx = build_clx_prc(&[1, 2, 3]);
    clx.extend_from_slice(&build_clx_prc(&[])); // zero-length Prc
    clx.extend_from_slice(&build_clx_prc(&[9]));
    clx.extend_from_slice(&build_clx_pcdt(&plc));
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "Hi");
}

// ---------------------------------------------------------------------------
// PlcPcd length arithmetic: n = (lcb-4)/12 and its rejection
// ---------------------------------------------------------------------------

#[test]
fn piece_count_math_two_pieces() {
    // Two pieces => lcb = (2+1)*4 + 2*8 = 12 + 16 = 28.
    let mut word = Vec::new();
    word.extend_from_slice(b"ABCD");
    let plc = build_plc_pcd(&[0, 2, 4], &[fc_compressed(0), fc_compressed(2)]);
    assert_eq!(plc.len(), 28);
    // n = (28-4)/12 = 2. Good.
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&word, &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "ABCD");
}

#[test]
fn plc_pcd_bad_remainder_rejected() {
    // lcb-4 not divisible by 12 -> MalformedPieceTable.
    // Build a Pcdt whose data length is, say, 4+5 = 9 (rem=5, 5%12!=0).
    let plc = vec![0u8; 9];
    let clx = build_clx_pcdt(&plc);
    let err = extract_text(&[], &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::MalformedPieceTable));
}

#[test]
fn plc_pcd_too_short_rejected() {
    // lcb < 4.
    let plc = vec![0u8; 3];
    let clx = build_clx_pcdt(&plc);
    let err = extract_text(&[], &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::MalformedPieceTable));
}

#[test]
fn zero_pieces_yields_empty_text() {
    // lcb = 4 => n = 0. A single CP, no PCDs. Valid, empty document.
    let plc = 0u32.to_le_bytes().to_vec(); // 4 bytes
    let clx = build_clx_pcdt(&plc);
    let text = extract_text(&[], &clx, 0, clx.len()).unwrap();
    assert_eq!(text, "");
}

// ---------------------------------------------------------------------------
// Error paths — must be clean typed errors, never panics
// ---------------------------------------------------------------------------

#[test]
fn non_cfb_bytes_yield_cfb_error() {
    let err = open_doc(b"not a compound file at all").unwrap_err();
    assert!(matches!(err, DocError::Cfb(_)));
    // source() should chain to the underlying CfbError.
    let src = std::error::Error::source(&err);
    assert!(src.is_some());
}

#[test]
fn empty_input_yields_cfb_error() {
    let err = open_doc(&[]).unwrap_err();
    assert!(matches!(err, DocError::Cfb(_)));
}

#[test]
fn fc_clx_past_table_is_truncated() {
    let word = b"Hi".to_vec();
    let table = vec![0u8; 4];
    // fc_clx beyond the table stream length.
    let err = extract_text(&word, &table, 100, 10).unwrap_err();
    assert!(matches!(err, DocError::Truncated));
}

#[test]
fn lcb_clx_past_table_is_truncated() {
    let word = b"Hi".to_vec();
    let table = vec![0u8; 10];
    // fc_clx valid, but fc_clx+lcb_clx overruns.
    let err = extract_text(&word, &table, 5, 100).unwrap_err();
    assert!(matches!(err, DocError::Truncated));
}

#[test]
fn fc_clx_lcb_overflow_is_truncated() {
    let table = vec![0u8; 10];
    let err = extract_text(&[], &table, usize::MAX, 1).unwrap_err();
    assert!(matches!(err, DocError::Truncated));
}

#[test]
fn compressed_piece_offset_past_worddocument_is_truncated() {
    // Piece claims text at WordDocument offset 1000 but the buffer is tiny.
    let word = vec![0u8; 4];
    let plc = build_plc_pcd(&[0, 5], &[fc_compressed(1000)]);
    let clx = build_clx_pcdt(&plc);
    let err = extract_text(&word, &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::Truncated));
}

#[test]
fn uncompressed_piece_offset_past_worddocument_is_truncated() {
    let word = vec![0u8; 4];
    let plc = build_plc_pcd(&[0, 5], &[fc_uncompressed(1000)]);
    let clx = build_clx_pcdt(&plc);
    let err = extract_text(&word, &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::Truncated));
}

#[test]
fn decreasing_cp_is_malformed() {
    // cp[1] < cp[0] => checked_sub underflow => MalformedPieceTable.
    let word = vec![0u8; 16];
    let plc = build_plc_pcd(&[10, 5], &[fc_compressed(0)]);
    let clx = build_clx_pcdt(&plc);
    let err = extract_text(&word, &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::MalformedPieceTable));
}

#[test]
fn empty_clx_is_malformed() {
    // fc_clx=0, lcb_clx=0 => empty CLX, no parts => MalformedPieceTable.
    let err = extract_text(&[], &[], 0, 0).unwrap_err();
    assert!(matches!(err, DocError::MalformedPieceTable));
}

#[test]
fn unknown_clx_tag_is_malformed() {
    // A part tag we don't recognise: we cannot know its length, so reject.
    let clx = vec![0x7F, 0x00, 0x00];
    let err = extract_text(&[], &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::MalformedPieceTable));
}

#[test]
fn prc_negative_length_is_malformed() {
    // Prc with cbGrpprl = -1 (0xFFFF) must be rejected, not loop backward.
    let mut clx = Vec::new();
    clx.push(CLXT_PRC);
    clx.extend_from_slice(&(-1i16).to_le_bytes());
    let err = extract_text(&[], &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::MalformedPieceTable));
}

#[test]
fn prc_payload_past_clx_is_truncated() {
    // Prc claims a 100-byte payload but the CLX ends after the header.
    let mut clx = Vec::new();
    clx.push(CLXT_PRC);
    clx.extend_from_slice(&100i16.to_le_bytes());
    let err = extract_text(&[], &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::Truncated));
}

#[test]
fn pcdt_lcb_past_clx_is_truncated() {
    // Pcdt header says lcb=100 but no data follows.
    let mut clx = Vec::new();
    clx.push(CLXT_PCDT);
    clx.extend_from_slice(&100u32.to_le_bytes());
    let err = extract_text(&[], &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::Truncated));
}

#[test]
fn pcdt_truncated_header_is_malformed() {
    // Just the tag byte, no room for the u32 lcb.
    let clx = vec![CLXT_PCDT];
    let err = extract_text(&[], &clx, 0, clx.len()).unwrap_err();
    assert!(matches!(err, DocError::MalformedPieceTable));
}

// ---------------------------------------------------------------------------
// Error Display / source coverage
// ---------------------------------------------------------------------------

#[test]
fn all_errors_display_nonempty() {
    let variants: [DocError; 4] = [
        DocError::NotWordDocument,
        DocError::NoTableStream,
        DocError::MalformedPieceTable,
        DocError::Truncated,
    ];
    for e in variants {
        let s = format!("{e}");
        assert!(!s.is_empty());
        // Non-Cfb variants have no source.
        assert!(std::error::Error::source(&e).is_none());
    }
}

#[test]
fn cfb_error_display_and_from() {
    // Exercise the From<CfbError> conversion + Display + source.
    let cfb_err = cfb::CompoundFile::open(&[]).unwrap_err();
    let doc_err: DocError = cfb_err.into();
    assert!(matches!(doc_err, DocError::Cfb(_)));
    assert!(!format!("{doc_err}").is_empty());
    assert!(std::error::Error::source(&doc_err).is_some());
}

// ---------------------------------------------------------------------------
// Low-level LE readers
// ---------------------------------------------------------------------------

#[test]
fn read_helpers_bounds_check() {
    let buf = [0x01, 0x02, 0x03, 0x04];
    assert_eq!(read_u16_le(&buf, 0), Some(0x0201));
    assert_eq!(read_u32_le(&buf, 0), Some(0x04030201));
    // Off the end -> None, not a panic.
    assert_eq!(read_u16_le(&buf, 3), None);
    assert_eq!(read_u32_le(&buf, 1), None);
    assert_eq!(read_u16_le(&buf, usize::MAX), None);
    assert_eq!(read_u32_le(&buf, usize::MAX), None);
}
