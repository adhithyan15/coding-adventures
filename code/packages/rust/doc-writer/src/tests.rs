//! Tests for the `.doc` writer.
//!
//! The centrepiece is the **round-trip** test: we write a document, open it with
//! the independent `cfb` reader, and then **re-implement** the [MS-DOC]
//! piece-table retrieval from first principles (FIB → CLX → PlcPcd → PCD →
//! FcCompressed → text). If the reassembled text equals the input, we have
//! proven the emitted bytes are a genuinely readable `.doc`, not just something
//! that happens to satisfy our own encoder.

use super::*;
use cfb::CompoundFile;

// ---------------------------------------------------------------------------
// A standalone, reader-side re-implementation of `.doc` text retrieval. This
// intentionally shares NO code with the writer beyond the raw byte constants of
// the format — it is the adversary that keeps the writer honest.
// ---------------------------------------------------------------------------

/// One decoded piece: how many characters, from which WordDocument byte offset,
/// in which encoding.
#[derive(Debug, Clone, Copy)]
struct Piece {
    /// Number of characters (8-bit) or UTF-16 code units (16-bit) in the piece.
    n_chars: u32,
    /// Real byte offset into the WordDocument stream.
    offset: usize,
    /// True → 8-bit compressed (1 byte/char); false → 16-bit (2 bytes/char).
    compressed: bool,
}

fn read_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}
fn read_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

/// Given the two `.doc` streams, reassemble the document text exactly as a
/// [MS-DOC] reader would.
fn retrieve_text(word_document: &[u8], table: &[u8]) -> String {
    // 1. Validate the FIB magic and the table-stream selector.
    assert_eq!(read_u16(word_document, 0), 0xA5EC, "wIdent (FIB magic)");
    let flags = read_u16(word_document, 10);
    let f_which_tbl_stm = (flags & 0x0200) != 0;
    assert!(f_which_tbl_stm, "fWhichTblStm should select 1Table");

    // 2. Locate the CLX in the table stream.
    let fc_clx = read_u32(word_document, 0x1A2) as usize;
    let lcb_clx = read_u32(word_document, 0x1A6) as usize;
    let clx = &table[fc_clx..fc_clx + lcb_clx];

    // 3. Parse the CLX: clxt == 0x02 (Pcdt), then lcb, then the PlcPcd.
    assert_eq!(clx[0], 0x02, "clxt should be Pcdt");
    let lcb = read_u32(clx, 1) as usize;
    let plc_pcd = &clx[5..5 + lcb];

    // 4. A PlcPcd with n pieces is 4*(n+1) CP bytes + 8*n PCD bytes = 12n + 4.
    assert!(lcb >= 4, "PlcPcd too short");
    let n = (lcb - 4) / 12;
    let cp_bytes = 4 * (n + 1);

    // 5. Decode each PCD's FcCompressed and read the piece's characters.
    let mut pieces = Vec::with_capacity(n);
    for i in 0..n {
        let cp_i = read_u32(plc_pcd, i * 4);
        let cp_next = read_u32(plc_pcd, (i + 1) * 4);
        let piece_chars = cp_next - cp_i;

        let pcd_off = cp_bytes + i * 8;
        // PCD = u16 flags, u32 FcCompressed, u16 prm.
        let fc = read_u32(plc_pcd, pcd_off + 2);
        let compressed = (fc & 0x4000_0000) != 0;
        let raw = (fc & 0x3FFF_FFFF) as usize;
        let offset = if compressed { raw / 2 } else { raw };

        pieces.push(Piece {
            n_chars: piece_chars,
            offset,
            compressed,
        });
    }

    // 6. Concatenate the pieces into the reassembled text.
    let mut out = String::new();
    for p in pieces {
        if p.compressed {
            // 1 byte per char, Latin-1.
            for k in 0..p.n_chars as usize {
                let b = word_document[p.offset + k];
                out.push(b as char);
            }
        } else {
            // 2 bytes per UTF-16LE code unit.
            let mut units = Vec::with_capacity(p.n_chars as usize);
            for k in 0..p.n_chars as usize {
                units.push(read_u16(word_document, p.offset + k * 2));
            }
            out.push_str(&String::from_utf16(&units).expect("valid UTF-16"));
        }
    }
    out
}

/// Open written `.doc` bytes and retrieve the text end-to-end.
fn round_trip(bytes: &[u8]) -> String {
    let cf = CompoundFile::open(bytes).expect(".doc should be a valid CFB");
    let wd = cf.read_stream("WordDocument").expect("WordDocument stream");
    let table = cf.read_stream("1Table").expect("1Table stream");
    retrieve_text(&wd, &table)
}

// ---------------------------------------------------------------------------
// The required round-trip proofs.
// ---------------------------------------------------------------------------

#[test]
fn round_trips_hello_doc() {
    let mut doc = Document::new();
    doc.add_paragraph("Hello, DOC!");
    let bytes = write_doc(&doc);
    // Opens with the CFB signature.
    assert_eq!(
        &bytes[0..8],
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
    );
    assert_eq!(round_trip(&bytes), "Hello, DOC!");
}

#[test]
fn round_trips_multiple_paragraphs_joined_by_cr() {
    let mut doc = Document::new();
    doc.add_paragraph("a");
    doc.add_paragraph("b");
    doc.add_paragraph("c");
    let bytes = write_doc(&doc);
    assert_eq!(round_trip(&bytes), "a\rb\rc");
}

#[test]
fn round_trips_non_latin1_via_16bit_fallback() {
    // "café 你好" contains a char (你, 好) beyond U+00FF → forces 16-bit.
    let mut doc = Document::new();
    doc.add_paragraph("café 你好");
    let bytes = write_doc(&doc);
    assert_eq!(round_trip(&bytes), "café 你好");
}

#[test]
fn round_trips_latin1_supplement_still_8bit() {
    // "café" — all chars <= U+00FF, so this stays 8-bit compressed, and the
    // é (U+00E9) round-trips as a single Latin-1 byte.
    let mut doc = Document::new();
    doc.add_paragraph("café");
    let bytes = write_doc(&doc);
    // Confirm it really used the 8-bit path: the WordDocument stream length is
    // TEXT_OFFSET + 4 (one byte per char), not +8.
    let cf = CompoundFile::open(&bytes).unwrap();
    let wd = cf.read_stream("WordDocument").unwrap();
    assert_eq!(wd.len(), TEXT_OFFSET + 4);
    assert_eq!(round_trip(&bytes), "café");
}

#[test]
fn round_trips_astral_char_16bit_surrogate_pair() {
    // An astral char (😀 U+1F600) is a surrogate PAIR in UTF-16: two code units.
    // The CP array must count code units, and retrieval must recombine them.
    let mut doc = Document::new();
    doc.add_paragraph("hi 😀");
    let bytes = write_doc(&doc);
    assert_eq!(round_trip(&bytes), "hi 😀");
}

// ---------------------------------------------------------------------------
// FcCompressed encoding unit tests (the bit-30 trick).
// ---------------------------------------------------------------------------

#[test]
fn fc_compressed_8bit_encoding() {
    // TEXT_OFFSET=2048 → (2048*2) | 0x40000000 = 0x40001000.
    let fc = build_fc_compressed(TEXT_OFFSET, Encoding::Compressed8).unwrap();
    assert_eq!(fc, 0x4000_1000);
    // Decode the way a reader does.
    assert!(fc & 0x4000_0000 != 0, "flag set");
    assert_eq!((fc & 0x3FFF_FFFF) / 2, TEXT_OFFSET as u32);
}

#[test]
fn fc_compressed_16bit_encoding() {
    // 16-bit: offset stored as-is, flag clear.
    let fc = build_fc_compressed(TEXT_OFFSET, Encoding::Uncompressed16).unwrap();
    assert_eq!(fc, TEXT_OFFSET as u32);
    assert!(fc & 0x4000_0000 == 0, "flag clear");
    assert_eq!(fc & 0x3FFF_FFFF, TEXT_OFFSET as u32);
}

#[test]
fn fc_compressed_rejects_overflowing_offset() {
    // An 8-bit offset whose doubled value exceeds the 30-bit field is rejected.
    let too_big = (0x3FFF_FFFF / 2) + 1;
    assert!(build_fc_compressed(too_big, Encoding::Compressed8).is_none());
    // And a 16-bit offset beyond the 30-bit field.
    assert!(build_fc_compressed(0x4000_0000, Encoding::Uncompressed16).is_none());
}

// ---------------------------------------------------------------------------
// CLX byte-layout unit tests.
// ---------------------------------------------------------------------------

#[test]
fn clx_layout_is_exact() {
    // n_chars = 11 ("Hello, DOC!"), fc = 0x40001000.
    let clx = build_clx(11, 0x4000_1000);
    // clxt(1) + lcb(4) + PlcPcd(16) = 21 bytes.
    assert_eq!(clx.len(), 21);
    assert_eq!(clx[0], 0x02); // clxt = Pcdt
    assert_eq!(read_u32(&clx, 1), 16); // lcb
    // PlcPcd: CP[0]=0, CP[1]=11, then PCD (u16 0, u32 fc, u16 0).
    assert_eq!(read_u32(&clx, 5), 0); // CP[0]
    assert_eq!(read_u32(&clx, 9), 11); // CP[1]
    assert_eq!(read_u16(&clx, 13), 0); // PCD flags
    assert_eq!(read_u32(&clx, 15), 0x4000_1000); // FcCompressed
    assert_eq!(read_u16(&clx, 19), 0); // prm
}

#[test]
fn lcb_clx_in_fib_matches_emitted_clx_length() {
    let mut doc = Document::new();
    doc.add_paragraph("Hello, DOC!");
    let bytes = write_doc(&doc);
    let cf = CompoundFile::open(&bytes).unwrap();
    let wd = cf.read_stream("WordDocument").unwrap();
    let table = cf.read_stream("1Table").unwrap();
    let lcb_clx = read_u32(&wd, 0x1A6) as usize;
    // The whole 1Table stream IS the CLX (fcClx = 0), and lcbClx should equal
    // its length exactly.
    assert_eq!(lcb_clx, table.len());
    assert_eq!(lcb_clx, 21); // 1 + 4 + 16
}

// ---------------------------------------------------------------------------
// FIB field spot-checks.
// ---------------------------------------------------------------------------

#[test]
fn fib_fields_are_set() {
    let mut doc = Document::new();
    doc.add_paragraph("Hello, DOC!");
    let bytes = write_doc(&doc);
    let cf = CompoundFile::open(&bytes).unwrap();
    let wd = cf.read_stream("WordDocument").unwrap();
    assert_eq!(read_u16(&wd, 0), 0xA5EC); // wIdent
    assert_eq!(read_u16(&wd, 2), 0x00C1); // nFib (Word 97)
    assert_eq!(read_u16(&wd, 10), 0x0200); // fWhichTblStm
    assert_eq!(read_u16(&wd, 32), 0x000E); // csw
    assert_eq!(read_u16(&wd, 62), 0x0016); // cslw
    assert_eq!(read_u32(&wd, 0x4C), 11); // ccpText
    assert_eq!(read_u16(&wd, 152), 0x005D); // cbRgFcLcb
    assert_eq!(read_u32(&wd, 0x1A2), 0); // fcClx
}

// ---------------------------------------------------------------------------
// pick_encoding / edge cases.
// ---------------------------------------------------------------------------

#[test]
fn pick_encoding_boundary_at_00ff() {
    // U+00FF is the last Latin-1 char → still 8-bit.
    assert_eq!(pick_encoding("\u{00FF}"), Encoding::Compressed8);
    // U+0100 is the first char requiring 16-bit.
    assert_eq!(pick_encoding("\u{0100}"), Encoding::Uncompressed16);
}

#[test]
fn empty_document_does_not_panic_and_round_trips() {
    let doc = Document::new();
    let bytes = write_doc(&doc);
    // Valid CFB.
    assert_eq!(
        &bytes[0..8],
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
    );
    // Zero characters reassemble to the empty string.
    assert_eq!(round_trip(&bytes), "");
}

#[test]
fn single_empty_paragraph_round_trips() {
    let mut doc = Document::new();
    doc.add_paragraph("");
    let bytes = write_doc(&doc);
    assert_eq!(round_trip(&bytes), "");
}

#[test]
fn output_is_deterministic() {
    let mut doc = Document::new();
    doc.add_paragraph("Deterministic?");
    let a = write_doc(&doc);
    let b = write_doc(&doc);
    assert_eq!(a, b);
}

#[test]
fn encode_text_bytes_8bit_is_one_byte_per_char() {
    let bytes = encode_text_bytes("AZ", Encoding::Compressed8).unwrap();
    assert_eq!(bytes, vec![0x41, 0x5A]);
}

#[test]
fn encode_text_bytes_16bit_is_utf16le() {
    let bytes = encode_text_bytes("A你", Encoding::Uncompressed16).unwrap();
    // 'A' = U+0041 → 41 00 ; '你' = U+4F60 → 60 4F.
    assert_eq!(bytes, vec![0x41, 0x00, 0x60, 0x4F]);
}
