//! Comprehensive conformance tests for `swf-container` (SWF01).
//!
//! Tests:
//! - Container signatures (FWS, CWS, ZWS) and rejection of invalid prefixes
//! - SWF version validation (1..=50) and rejection of version 0
//! - Untrusted FileLength header parsing
//! - Stage RECT parsing via MSB-first BitReader (positive/negative twips, dimensions, conversions to pixels)
//! - FrameRate 8.8 fixed point decoding (integer + fractional parts)
//! - Short tags (< 63 bytes) and Long tags (0x3F + 4 bytes UI32 LE length)
//! - All SwfTagKind variants and round-tripping tag codes
//! - Zero-allocation SwfTagIter streaming over tag payloads
//! - Strict boundary checks (zero over-reads, truncated payload rejection)
//! - BitReader primitives (read_ub, read_sb, read_fb, read_matrix, read_cxform, read_cxform_with_alpha)
//! - Decompression bomb defense: 50:1 expansion ratio cap beyond 1 MB
//! - Max uncompressed byte cap enforcement
//! - Adler-32 checksum calculation and verification

use std::hint::black_box;
use swf_container::{
    compute_adler32, validate_ratio, validate_tag_code, validate_version, BitReader,
    PureCapabilities, SwfColorTransform, SwfDecompressorConfig, SwfError, SwfMovie, SwfSignature,
    SwfTagIter, SwfTagKind, ZwsConfig,
};

/// Bit writer helper for crafting synthetic SWF byte buffers.
struct BitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    bit_pos: usize, // 0..8 bits written to current_byte
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            current_byte: 0,
            bit_pos: 0,
        }
    }

    fn write_ub(&mut self, n: usize, val: u32) {
        for i in (0..n).rev() {
            let bit = ((val >> i) & 1) as u8;
            self.current_byte = (self.current_byte << 1) | bit;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bytes.push(self.current_byte);
                self.current_byte = 0;
                self.bit_pos = 0;
            }
        }
    }

    fn write_sb(&mut self, n: usize, val: i32) {
        let mask = if n == 32 {
            0xFFFF_FFFF
        } else {
            (1u32 << n) - 1
        };
        let u = (val as u32) & mask;
        self.write_ub(n, u);
    }

    fn write_fb(&mut self, n: usize, val: f64) {
        let fixed = (val * 65536.0).round() as i32;
        self.write_sb(n, fixed);
    }

    fn align(&mut self) {
        if self.bit_pos > 0 {
            self.current_byte <<= 8 - self.bit_pos;
            self.bytes.push(self.current_byte);
            self.current_byte = 0;
            self.bit_pos = 0;
        }
    }

    fn write_rect(&mut self, xmin: i32, xmax: i32, ymin: i32, ymax: i32) {
        let max_val = xmin.abs().max(xmax.abs()).max(ymin.abs()).max(ymax.abs());
        let mut bits = 1;
        while (1i32 << bits) <= max_val {
            bits += 1;
        }
        let nbits = (bits + 2).clamp(5, 31);

        self.write_ub(5, nbits as u32);
        self.write_sb(nbits, xmin);
        self.write_sb(nbits, xmax);
        self.write_sb(nbits, ymin);
        self.write_sb(nbits, ymax);
        self.align();
    }

    fn into_bytes(mut self) -> Vec<u8> {
        self.align();
        self.bytes
    }
}

fn create_tag_bytes(code: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if payload.len() < 0x3F {
        let header = (code << 6) | (payload.len() as u16);
        out.extend_from_slice(&header.to_le_bytes());
    } else {
        let header = (code << 6) | 0x3F;
        out.extend_from_slice(&header.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(payload);
    out
}

fn build_synthetic_swf(
    sig: &[u8; 3],
    version: u8,
    rect: (i32, i32, i32, i32),
    frame_rate_raw: u16,
    frame_count: u16,
    tags: &[u8],
) -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.write_rect(rect.0, rect.1, rect.2, rect.3);
    let mut body = bw.into_bytes();
    body.extend_from_slice(&frame_rate_raw.to_le_bytes());
    body.extend_from_slice(&frame_count.to_le_bytes());
    body.extend_from_slice(tags);

    let total_len = 8 + body.len();

    let mut swf = Vec::new();
    swf.extend_from_slice(sig);
    swf.push(version);
    swf.extend_from_slice(&(total_len as u32).to_le_bytes());
    swf.extend_from_slice(&body);
    swf
}

// --- Header Tests ---

#[test]
fn test_uncompressed_fws_header() {
    let tags = create_tag_bytes(0, &[]); // End tag
    let swf = build_synthetic_swf(b"FWS", 10, (0, 11000, 0, 8000), 0x1800, 1, &tags);

    let config = SwfDecompressorConfig::default_decompressor_config();
    let movie = SwfMovie::from_swf_bytes(&swf, &config).expect("parse valid FWS");
    let h = movie.movie_header();

    assert_eq!(h.header_signature(), SwfSignature::Fws);
    assert_eq!(h.header_version(), 10);
    assert_eq!(h.header_file_length(), swf.len() as u32);
    assert_eq!(h.header_stage_rect().xmin, 0);
    assert_eq!(h.header_stage_rect().xmax, 11000);
    assert_eq!(h.header_stage_rect().ymin, 0);
    assert_eq!(h.header_stage_rect().ymax, 8000);
    assert_eq!(h.header_stage_rect().rect_width_twips(), 11000);
    assert_eq!(h.header_stage_rect().rect_height_twips(), 8000);
    assert_eq!(h.header_stage_rect().rect_width_pixels(), 550.0);
    assert_eq!(h.header_stage_rect().rect_height_pixels(), 400.0);
    assert_eq!(h.header_frame_rate(), 24.0);
    assert_eq!(h.header_frame_count(), 1);
}

#[test]
fn test_signature_variants() {
    assert_eq!(
        SwfSignature::from_signature_bytes(*b"FWS"),
        Ok(SwfSignature::Fws)
    );
    assert_eq!(
        SwfSignature::from_signature_bytes(*b"CWS"),
        Ok(SwfSignature::Cws)
    );
    assert_eq!(
        SwfSignature::from_signature_bytes(*b"ZWS"),
        Ok(SwfSignature::Zws)
    );
    assert_eq!(
        SwfSignature::from_signature_bytes(*b"BAD"),
        Err(SwfError::InvalidSignature)
    );
    assert_eq!(SwfSignature::Fws.as_signature_bytes(), *b"FWS");
    assert_eq!(SwfSignature::Cws.as_signature_bytes(), *b"CWS");
    assert_eq!(SwfSignature::Zws.as_signature_bytes(), *b"ZWS");
}

#[test]
fn test_unsupported_version_zero() {
    let swf = build_synthetic_swf(b"FWS", 0, (0, 100, 0, 100), 0x0C00, 1, &[]);
    let config = SwfDecompressorConfig::default_decompressor_config();
    assert_eq!(
        SwfMovie::from_swf_bytes(&swf, &config),
        Err(SwfError::UnsupportedVersion)
    );
}

#[test]
fn test_unsupported_version_exceeding_50() {
    let swf = build_synthetic_swf(b"FWS", 51, (0, 100, 0, 100), 0x0C00, 1, &[]);
    let config = SwfDecompressorConfig::default_decompressor_config();
    assert_eq!(
        SwfMovie::from_swf_bytes(&swf, &config),
        Err(SwfError::UnsupportedVersion)
    );
}

#[test]
fn test_frame_rate_fractional() {
    // 23.976 fps: 23 integer (0x17), 0.976 * 256 ≈ 250 (0xFA) -> 0x17FA
    let raw: u16 = 0x17FA;
    let swf = build_synthetic_swf(b"FWS", 9, (0, 100, 0, 100), raw, 10, &[]);
    let config = SwfDecompressorConfig::default_decompressor_config();
    let movie = SwfMovie::from_swf_bytes(&swf, &config).expect("valid swf");
    let fps = movie.movie_header().header_frame_rate();
    assert!((fps - 23.976).abs() < 0.01);
    assert_eq!(movie.movie_header().header_frame_rate_raw(), raw);
}

// --- Tag Streaming & Framing Tests ---

#[test]
fn test_short_and_long_tags_streaming() {
    let short_payload = [1u8, 2, 3, 4, 5];
    let long_payload = vec![42u8; 100]; // >= 63 bytes triggers long tag framing

    let mut tags = Vec::new();
    tags.extend_from_slice(&create_tag_bytes(9, &[0xFF, 0x00, 0x00])); // SetBackgroundColor
    tags.extend_from_slice(&create_tag_bytes(1, &[])); // ShowFrame
    tags.extend_from_slice(&create_tag_bytes(12, &short_payload)); // DoAction
    tags.extend_from_slice(&create_tag_bytes(82, &long_payload)); // DoABC (long)
    tags.extend_from_slice(&create_tag_bytes(0, &[])); // End

    let swf = build_synthetic_swf(b"FWS", 9, (0, 200, 0, 200), 0x0C00, 1, &tags);
    let config = SwfDecompressorConfig::default_decompressor_config();
    let movie = SwfMovie::from_swf_bytes(&swf, &config).expect("movie");

    let parsed_tags: Result<Vec<_>, _> = movie.movie_tags().collect();
    let parsed_tags = parsed_tags.expect("all tags valid");

    assert_eq!(parsed_tags.len(), 5);

    assert_eq!(parsed_tags[0].header.kind, SwfTagKind::SetBackgroundColor);
    assert_eq!(parsed_tags[0].header.length, 3);
    assert!(!parsed_tags[0].header.is_long);
    assert_eq!(parsed_tags[0].data, &[0xFF, 0x00, 0x00]);

    assert_eq!(parsed_tags[1].header.kind, SwfTagKind::ShowFrame);
    assert_eq!(parsed_tags[1].header.length, 0);

    assert_eq!(parsed_tags[2].header.kind, SwfTagKind::DoAction);
    assert_eq!(parsed_tags[2].header.length, 5);
    assert_eq!(parsed_tags[2].data, &short_payload);

    assert_eq!(parsed_tags[3].header.kind, SwfTagKind::DoABC);
    assert_eq!(parsed_tags[3].header.length, 100);
    assert!(parsed_tags[3].header.is_long);
    assert_eq!(parsed_tags[3].data, &long_payload[..]);

    assert_eq!(parsed_tags[4].header.kind, SwfTagKind::End);
    assert_eq!(parsed_tags[4].header.length, 0);
    assert!(parsed_tags[4].header.is_end_tag());
}

#[test]
fn test_all_swf_tag_kinds() {
    let cases = [
        (0, SwfTagKind::End, "End"),
        (1, SwfTagKind::ShowFrame, "ShowFrame"),
        (2, SwfTagKind::DefineShape, "DefineShape"),
        (4, SwfTagKind::PlaceObject, "PlaceObject"),
        (5, SwfTagKind::RemoveObject, "RemoveObject"),
        (6, SwfTagKind::DefineBits, "DefineBits"),
        (9, SwfTagKind::SetBackgroundColor, "SetBackgroundColor"),
        (12, SwfTagKind::DoAction, "DoAction"),
        (14, SwfTagKind::DefineSound, "DefineSound"),
        (15, SwfTagKind::StartSound, "StartSound"),
        (18, SwfTagKind::SoundStreamHead, "SoundStreamHead"),
        (19, SwfTagKind::SoundStreamBlock, "SoundStreamBlock"),
        (20, SwfTagKind::DefineBitsLossless, "DefineBitsLossless"),
        (21, SwfTagKind::DefineBitsJPEG2, "DefineBitsJPEG2"),
        (22, SwfTagKind::DefineShape2, "DefineShape2"),
        (26, SwfTagKind::PlaceObject2, "PlaceObject2"),
        (28, SwfTagKind::RemoveObject2, "RemoveObject2"),
        (32, SwfTagKind::DefineShape3, "DefineShape3"),
        (35, SwfTagKind::DefineBitsJPEG3, "DefineBitsJPEG3"),
        (36, SwfTagKind::DefineBitsLossless2, "DefineBitsLossless2"),
        (39, SwfTagKind::DefineSprite, "DefineSprite"),
        (45, SwfTagKind::SoundStreamHead2, "SoundStreamHead2"),
        (59, SwfTagKind::DoInitAction, "DoInitAction"),
        (69, SwfTagKind::FileAttributes, "FileAttributes"),
        (70, SwfTagKind::PlaceObject3, "PlaceObject3"),
        (76, SwfTagKind::SymbolClass, "SymbolClass"),
        (77, SwfTagKind::Metadata, "Metadata"),
        (82, SwfTagKind::DoABC, "DoABC"),
        (83, SwfTagKind::DefineShape4, "DefineShape4"),
        (90, SwfTagKind::DefineBitsJPEG4, "DefineBitsJPEG4"),
        (999, SwfTagKind::Unknown(999), "Unknown"),
    ];

    for (code, expected_kind, expected_name) in cases {
        let kind = SwfTagKind::from_code(code);
        assert_eq!(kind, expected_kind);
        assert_eq!(kind.tag_code(), code);
        assert_eq!(kind.tag_name(), expected_name);
    }
}

#[test]
fn test_tag_truncated_payload_fails() {
    let mut bad_tag = Vec::new();
    let header: u16 = (12 << 6) | 10; // tag 12 with length 10
    bad_tag.extend_from_slice(&header.to_le_bytes());
    bad_tag.extend_from_slice(&[1, 2, 3]); // only 3 bytes provided!

    let mut iter = SwfTagIter::from_payload(&bad_tag);
    assert_eq!(iter.next(), Some(Err(SwfError::UnexpectedEof)));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_long_tag_negative_length_rejected() {
    let mut bad_tag = Vec::new();
    let header: u16 = (82 << 6) | 0x3F;
    bad_tag.extend_from_slice(&header.to_le_bytes());
    let neg_len: i32 = -5;
    bad_tag.extend_from_slice(&neg_len.to_le_bytes());

    let mut iter = SwfTagIter::from_payload(&bad_tag);
    assert_eq!(iter.next(), Some(Err(SwfError::InvalidTagLength)));
}

// --- BitReader, Matrix, CXForm Tests ---

#[test]
fn test_bit_reader_ub_sb_fb() {
    let data = [0b1011_0100, 0b1111_0000];
    let mut r = BitReader::from_bits(&data);

    assert_eq!(r.read_ub(1), Ok(1));
    assert_eq!(r.read_ub(3), Ok(0b011));
    assert_eq!(r.read_sb(4), Ok(4)); // 0b0100 sign-extended in 4 bits is +4
}

#[test]
fn test_matrix_record_reading() {
    let mut bw = BitWriter::new();
    // has_scale = 1
    bw.write_ub(1, 1);
    bw.write_ub(5, 20); // 20 bits for scale
    bw.write_fb(20, 2.5);
    bw.write_fb(20, 0.5);
    // has_rotate = 0
    bw.write_ub(1, 0);
    // translate: 10 bits
    bw.write_ub(5, 10);
    bw.write_sb(10, 400); // 400 twips = 20 pixels
    bw.write_sb(10, -200); // -200 twips = -10 pixels
    let bytes = bw.into_bytes();

    let mut r = BitReader::from_bits(&bytes);
    let m = r.read_matrix().expect("matrix");

    assert!((m.scale_x - 2.5).abs() < 0.001);
    assert!((m.scale_y - 0.5).abs() < 0.001);
    assert_eq!(m.rotate_skew_0, 0.0);
    assert_eq!(m.rotate_skew_1, 0.0);
    assert_eq!(m.translate_x, 400);
    assert_eq!(m.translate_y, -200);
    assert_eq!(m.matrix_translate_pixels(), (20.0, -10.0));
}

#[test]
fn test_cxform_identity_and_apply() {
    let cx = SwfColorTransform::default_cxform();
    let initial_rgba = [100, 150, 200, 255];
    assert_eq!(cx.cxform_apply_rgba(initial_rgba), initial_rgba);

    // Custom tint: mult 0.5, add 50
    let tint = SwfColorTransform {
        r_mult: 128, // 0.5
        g_mult: 128,
        b_mult: 128,
        a_mult: 256,
        r_add: 50,
        g_add: 0,
        b_add: -20,
        a_add: 0,
    };
    // r: (100 * 128 >> 8) + 50 = 50 + 50 = 100
    // g: (150 * 128 >> 8) + 0 = 75
    // b: (200 * 128 >> 8) - 20 = 100 - 20 = 80
    // a: 255
    let res = tint.cxform_apply_rgba(initial_rgba);
    assert_eq!(res, [100, 75, 80, 255]);
}

#[test]
fn test_cxform_bitstream_reading() {
    let mut bw = BitWriter::new();
    // has_add = 1, has_mult = 1, nbits = 10 (needs 10 bits for positive 256 in signed format)
    bw.write_ub(1, 1);
    bw.write_ub(1, 1);
    bw.write_ub(4, 10);
    // mult terms (r, g, b)
    bw.write_sb(10, 256);
    bw.write_sb(10, 128);
    bw.write_sb(10, 64);
    // add terms (r, g, b)
    bw.write_sb(10, 10);
    bw.write_sb(10, 20);
    bw.write_sb(10, -30);
    let bytes = bw.into_bytes();

    let mut r = BitReader::from_bits(&bytes);
    let cx = r.read_cxform().expect("cxform");

    assert_eq!(cx.r_mult, 256);
    assert_eq!(cx.g_mult, 128);
    assert_eq!(cx.b_mult, 64);
    assert_eq!(cx.r_add, 10);
    assert_eq!(cx.g_add, 20);
    assert_eq!(cx.b_add, -30);
}

// --- CWS (ZLIB) Decompression & Anti-Bomb Tests ---

#[test]
fn test_cws_round_trip() {
    let tags = create_tag_bytes(0, &[]);
    let fws = build_synthetic_swf(b"FWS", 10, (0, 8000, 0, 6000), 0x0C00, 1, &tags);

    // Body to compress starts at offset 8
    let body = &fws[8..];
    let compressed_body = deflate::zlib_compress(body);

    let mut cws = Vec::new();
    cws.extend_from_slice(b"CWS");
    cws.push(10);
    cws.extend_from_slice(&(fws.len() as u32).to_le_bytes());
    cws.extend_from_slice(&compressed_body);

    let config = SwfDecompressorConfig::default_decompressor_config();
    let movie = SwfMovie::from_swf_bytes(&cws, &config).expect("decompress CWS");

    assert_eq!(movie.movie_header().header_signature(), SwfSignature::Cws);
    assert_eq!(movie.movie_header().header_version(), 10);
    assert_eq!(
        movie.movie_header().header_stage_rect().rect_width_twips(),
        8000
    );
}

#[test]
fn test_cws_checksum_mismatch() {
    let tags = create_tag_bytes(0, &[]);
    let fws = build_synthetic_swf(b"FWS", 10, (0, 8000, 0, 6000), 0x0C00, 1, &tags);
    let mut compressed_body = deflate::zlib_compress(&fws[8..]);

    // Corrupt the Adler-32 checksum at the end
    let last = compressed_body.len() - 1;
    compressed_body[last] ^= 0xFF;

    let mut cws = Vec::new();
    cws.extend_from_slice(b"CWS");
    cws.push(10);
    cws.extend_from_slice(&(fws.len() as u32).to_le_bytes());
    cws.extend_from_slice(&compressed_body);

    let config = SwfDecompressorConfig::default_decompressor_config();
    assert_eq!(
        SwfMovie::from_swf_bytes(&cws, &config),
        Err(SwfError::ChecksumMismatch)
    );
}

#[test]
fn test_cws_max_uncompressed_bytes_cap() {
    let tags = create_tag_bytes(0, &[]);
    let fws = build_synthetic_swf(b"FWS", 10, (0, 8000, 0, 6000), 0x0C00, 1, &tags);
    let compressed_body = deflate::zlib_compress(&fws[8..]);

    let mut cws = Vec::new();
    cws.extend_from_slice(b"CWS");
    cws.push(10);
    cws.extend_from_slice(&(fws.len() as u32).to_le_bytes());
    cws.extend_from_slice(&compressed_body);

    // Limit buffer to 5 bytes
    let config =
        SwfDecompressorConfig::default_decompressor_config().with_max_uncompressed_bytes(5);
    assert_eq!(
        SwfMovie::from_swf_bytes(&cws, &config),
        Err(SwfError::OutputLimitExceeded)
    );
}

struct LsbBitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    bit_pos: usize,
}

impl LsbBitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            current_byte: 0,
            bit_pos: 0,
        }
    }

    fn write_bit(&mut self, bit: u8) {
        self.current_byte |= (bit & 1) << self.bit_pos;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bytes.push(self.current_byte);
            self.current_byte = 0;
            self.bit_pos = 0;
        }
    }

    fn write_code(&mut self, code: u32, len: usize) {
        // Huffman codes in Deflate: the most significant bit of the code is transmitted first
        for i in (0..len).rev() {
            let bit = ((code >> i) & 1) as u8;
            self.write_bit(bit);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bit_pos > 0 {
            self.bytes.push(self.current_byte);
        }
        self.bytes
    }
}

fn build_fixed_huffman_bomb_zlib(repeat_count: usize) -> (Vec<u8>, usize) {
    let mut writer = LsbBitWriter::new();
    // BFINAL=1 (1 bit), BTYPE=01 (2 bits) -> 3 bits (0b01, then 1)
    writer.write_bit(1);
    writer.write_bit(1);
    writer.write_bit(0);

    // Literal 'A' (sym 65) -> code 0x30 + 65 = 113 = 0b01110001 (8 bits)
    writer.write_code(0b01110001, 8);

    // Repetitions of (Length 258, Dist 1)
    // Sym 285 -> code 285 - 280 + 0b11000000 = 0b11000101 (8 bits)
    // Dist 0 -> code 0 (5 bits)
    for _ in 0..repeat_count {
        writer.write_code(0b11000101, 8);
        writer.write_code(0b00000, 5);
    }

    // End of block (sym 256) -> code 0 (7 bits)
    writer.write_code(0b0000000, 7);

    let raw_deflate = writer.finish();
    let uncompressed_len = 1 + repeat_count * 258;

    // Adler-32 of uncompressed 'A' bytes
    let uncompressed_data = vec![b'A'; uncompressed_len];
    let adler = compute_adler32(&uncompressed_data);

    let mut zlib = Vec::with_capacity(2 + raw_deflate.len() + 4);
    zlib.push(0x78);
    zlib.push(0x9C);
    zlib.extend_from_slice(&raw_deflate);
    zlib.extend_from_slice(&adler.to_be_bytes());

    (zlib, uncompressed_len)
}

#[test]
fn test_cws_decompression_bomb_detected() {
    // 5,000 repetitions of 258 bytes = 1,290,001 uncompressed bytes (> 1 MB)
    // from an ~8 KB compressed stream (ratio ~158:1 > 50:1 limit)
    let (compressed, uncompressed_len) = build_fixed_huffman_bomb_zlib(5000);

    assert!(uncompressed_len > 1024 * 1024);
    assert!(uncompressed_len as f64 / compressed.len() as f64 > 50.0);

    let mut cws = Vec::new();
    cws.extend_from_slice(b"CWS");
    cws.push(10);
    let total_uncompressed = (8 + uncompressed_len) as u32;
    cws.extend_from_slice(&total_uncompressed.to_le_bytes());
    cws.extend_from_slice(&compressed);

    let config = SwfDecompressorConfig::default_decompressor_config();
    assert_eq!(
        SwfMovie::from_swf_bytes(&cws, &config),
        Err(SwfError::DecompressionBombDetected)
    );
}

#[test]
fn test_cws_odd_aligned_decompression_bomb() {
    // 3 literal bytes (odd length 3), followed by 258-byte repeats (always odd length: 3, 261, 519, ...)
    let mut writer = LsbBitWriter::new();
    writer.write_bit(1);
    writer.write_bit(1);
    writer.write_bit(0);

    // 3 literals: 'A', 'B', 'C'
    // Sym 65 ('A') -> 0b01110001
    // Sym 66 ('B') -> 0b01110010
    // Sym 67 ('C') -> 0b01110011
    writer.write_code(0b01110001, 8);
    writer.write_code(0b01110010, 8);
    writer.write_code(0b01110011, 8);

    // 5000 repetitions of (Length 258, Dist 3)
    // Sym 285 -> code 0b11000101 (8 bits)
    // Dist 2 (code 2 for distance 3, 5 bits) -> 0b00010
    for _ in 0..5000 {
        writer.write_code(0b11000101, 8);
        writer.write_code(0b00010, 5);
    }
    writer.write_code(0b0000000, 7); // end of block

    let raw_deflate = writer.finish();
    let uncompressed_len = 3 + 5000 * 258;

    let mut uncompressed_data = Vec::with_capacity(uncompressed_len);
    uncompressed_data.extend_from_slice(b"ABC");
    while uncompressed_data.len() < uncompressed_len {
        let remaining = uncompressed_len - uncompressed_data.len();
        let chunk = remaining.min(3);
        uncompressed_data.extend_from_slice(&b"ABC"[..chunk]);
    }
    let adler = compute_adler32(&uncompressed_data);

    let mut zlib = Vec::with_capacity(2 + raw_deflate.len() + 4);
    zlib.push(0x78);
    zlib.push(0x9C);
    zlib.extend_from_slice(&raw_deflate);
    zlib.extend_from_slice(&adler.to_be_bytes());

    let mut cws = Vec::new();
    cws.extend_from_slice(b"CWS");
    cws.push(10);
    let total_uncompressed = (8 + uncompressed_len) as u32;
    cws.extend_from_slice(&total_uncompressed.to_le_bytes());
    cws.extend_from_slice(&zlib);

    let config = SwfDecompressorConfig::default_decompressor_config();
    assert_eq!(
        SwfMovie::from_swf_bytes(&cws, &config),
        Err(SwfError::DecompressionBombDetected)
    );
}

// --- ZWS (LZMA) Tests ---

#[test]
fn test_zws_config_parsing() {
    let mut zws_bytes = Vec::new();
    zws_bytes.extend_from_slice(b"ZWS");
    zws_bytes.push(13); // version 13
    zws_bytes.extend_from_slice(&100_000u32.to_le_bytes()); // uncompressed len
    zws_bytes.extend_from_slice(&5_000u32.to_le_bytes()); // compressed len
    let prop_byte = 3 + 2 * 45; // lc=3, lp=0, pb=2 = 93 (0x5D)
    zws_bytes.push(prop_byte);
    zws_bytes.extend_from_slice(&0x0010_0000u32.to_le_bytes()); // dict_size 1MB

    let config = ZwsConfig::parse_zws_header(&zws_bytes).expect("parse zws config");
    assert_eq!(config.version, 13);
    assert_eq!(config.uncompressed_file_length, 100_000);
    assert_eq!(config.compressed_length, 5_000);
    assert_eq!(config.lc, 3);
    assert_eq!(config.lp, 0);
    assert_eq!(config.pb, 2);
    assert_eq!(config.zws_dict_size(), 0x0010_0000);
}

#[test]
fn test_zws_version_must_be_at_least_13() {
    let mut zws_bytes = Vec::new();
    zws_bytes.extend_from_slice(b"ZWS");
    zws_bytes.push(12); // Invalid for ZWS!
    zws_bytes.extend_from_slice(&[0; 13]);

    assert_eq!(
        ZwsConfig::parse_zws_header(&zws_bytes),
        Err(SwfError::UnsupportedVersion)
    );
}

struct LzmaTestEncoder {
    low: u64,
    range: u32,
    bytes: Vec<u8>,
}

impl LzmaTestEncoder {
    fn new() -> Self {
        Self {
            low: 0,
            range: 0xFFFF_FFFF,
            bytes: vec![0], // First byte of LZMA stream is ignored
        }
    }

    fn encode_bit(&mut self, prob: &mut u16, bit: u32) {
        let bound = (self.range >> 11) * (*prob as u32);
        if bit == 0 {
            self.range = bound;
            *prob += (2048 - *prob) >> 5;
        } else {
            self.low += bound as u64;
            self.range -= bound;
            *prob -= *prob >> 5;
        }
        self.shift_low();
    }

    fn encode_direct_bits(&mut self, val: u32, count: usize) {
        for i in (0..count).rev() {
            self.range >>= 1;
            let bit = (val >> i) & 1;
            if bit == 1 {
                self.low += self.range as u64;
            }
            self.shift_low();
        }
    }

    fn encode_bit_tree(&mut self, probs: &mut [u16], val: usize) {
        let num_bits = probs.len().ilog2() as usize;
        let mut model_index = 1;
        for i in (0..num_bits).rev() {
            let bit = ((val >> i) & 1) as u32;
            self.encode_bit(&mut probs[model_index], bit);
            model_index = (model_index << 1) | (bit as usize);
        }
    }

    fn encode_reverse_bit_tree(&mut self, probs: &mut [u16], val: usize) {
        let num_bits = probs.len().ilog2() as usize;
        let mut model_index = 1;
        for i in 0..num_bits {
            let bit = ((val >> i) & 1) as u32;
            self.encode_bit(&mut probs[model_index], bit);
            model_index = (model_index << 1) | (bit as usize);
        }
    }

    fn shift_low(&mut self) {
        if self.range < 0x0100_0000 {
            self.bytes.push((self.low >> 24) as u8);
            self.low = (self.low << 8) & 0xFFFF_FFFF;
            self.range <<= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        for _ in 0..5 {
            self.bytes.push((self.low >> 24) as u8);
            self.low = (self.low << 8) & 0xFFFF_FFFF;
        }
        // Extra padding bytes so RangeDecoder doesn't EOF on trailing normalizations
        self.bytes.extend_from_slice(&[0u8; 32]);
        self.bytes
    }
}

#[test]
fn test_lzma_slot_13_and_state7_literal_match() {
    let mut enc = LzmaTestEncoder::new();

    let mut is_match = [1024u16; 12 * 16];
    let mut is_rep = [1024u16; 12];
    let mut pos_slot = [[1024u16; 64]; 4];
    let mut spec_pos = [1024u16; 115];
    let mut choice = 1024u16;
    let mut low = [1024u16; 16 * 8];
    let mut lit_probs = vec![1024u16; 0x300];

    let mut output: Vec<u8> = Vec::new();
    let mut state = 0usize;

    // Step 1: 130 literal 0 bytes
    for _ in 0..130 {
        let match_idx = state << 4;
        enc.encode_bit(&mut is_match[match_idx], 0); // literal
        let mut sym = 1;
        for _ in 0..8 {
            enc.encode_bit(&mut lit_probs[sym], 0);
            sym <<= 1;
        }
        output.push(0);
        state = 0;
    }
    assert_eq!(output.len(), 130);

    // Step 2: Normal match with slot == 13 (exercises spec_pos[83..115])
    let match_idx = state << 4;
    enc.encode_bit(&mut is_match[match_idx], 1); // match
    enc.encode_bit(&mut is_rep[state], 0); // normal match
    state = 7;
    enc.encode_bit(&mut choice, 0); // low len
    enc.encode_bit_tree(&mut low[0..8], 0); // len_raw = 0 -> len = 2
    let slot = 13usize;
    enc.encode_bit_tree(&mut pos_slot[0], slot); // slot = 13
    let num_direct_bits = (slot >> 1) - 1; // 5
    let dist_base = (2 | (slot & 1)) << num_direct_bits; // 96
    let offset = dist_base - slot; // 83
    assert_eq!(offset, 83);
    assert_eq!(offset + (1 << num_direct_bits), 115);
    let extra = 0usize;
    enc.encode_reverse_bit_tree(&mut spec_pos[offset..offset + 32], extra);
    let rep0 = dist_base + extra; // 96
    assert_eq!(rep0, 96);
    for _ in 0..2 {
        let idx = output.len() - 1 - rep0;
        let b = output[idx];
        output.push(b);
    }
    assert_eq!(output.len(), 132);

    // Step 3: Literal decoding in state >= 7 (exercises match_byte <<= 1)
    assert!(state >= 7);
    let match_idx = state << 4;
    enc.encode_bit(&mut is_match[match_idx], 0); // literal
    let mut match_byte = output[output.len() - 1 - rep0] as usize; // 0
    assert_eq!(match_byte, 0);
    let target_lit = 0usize;
    let mut sym = 1;
    let mut match_bit_enabled = true;
    while sym < 0x100 {
        let match_bit = if match_bit_enabled {
            (match_byte >> 7) & 1
        } else {
            0
        };
        match_byte <<= 1;
        let prob_idx = 0x100 + (match_bit << 8) + sym;
        let bit = 0u32;
        enc.encode_bit(&mut lit_probs[prob_idx], bit);
        sym = (sym << 1) | (bit as usize);
        if match_bit_enabled && bit as usize != match_bit {
            match_bit_enabled = false;
        }
    }
    output.push(target_lit as u8);
    state -= 3; // 4
    assert_eq!(state, 4);
    assert_eq!(output.len(), 133);

    let lzma_stream = enc.finish();

    // Now decompress with swf_container::lzma_decompress_bounded
    let config = ZwsConfig {
        version: 13,
        uncompressed_file_length: 133 + 8,
        compressed_length: lzma_stream.len() as u32,
        lc: 0,
        lp: 0,
        pb: 0,
        dict_size: 1 << 20,
    };

    let decompressed =
        swf_container::lzma_decompress_bounded(&lzma_stream, 133, &config, 100_000, 100.0)
            .expect("decompress synthesized LZMA stream exercising slot 13 and state 7 match");

    assert_eq!(decompressed.len(), 133);
    assert_eq!(decompressed, output);
}

#[test]
fn test_lzma_decode_direct_bits_slot_14() {
    let mut enc = LzmaTestEncoder::new();

    let mut is_match = [1024u16; 12 * 16];
    let mut is_rep = [1024u16; 12];
    let mut pos_slot = [[1024u16; 64]; 4];
    let mut align_probs = [1024u16; 16];
    let mut choice = 1024u16;
    let mut low = [1024u16; 16 * 8];
    let mut lit_probs = vec![1024u16; 0x300];

    let mut output: Vec<u8> = Vec::new();
    let mut state = 0usize;

    // Step 1: 130 literal 0 bytes
    for _ in 0..130 {
        let match_idx = state << 4;
        enc.encode_bit(&mut is_match[match_idx], 0); // literal
        let mut sym = 1;
        for _ in 0..8 {
            enc.encode_bit(&mut lit_probs[sym], 0);
            sym <<= 1;
        }
        output.push(0);
        state = 0;
    }
    assert_eq!(output.len(), 130);

    // Step 2: Normal match with slot == 14 (exercises decode_direct_bits and wrapping arithmetic)
    let match_idx = state << 4;
    enc.encode_bit(&mut is_match[match_idx], 1); // match
    enc.encode_bit(&mut is_rep[state], 0); // normal match
    enc.encode_bit(&mut choice, 0); // low len
    enc.encode_bit_tree(&mut low[0..8], 0); // len = 2
    let slot = 14usize;
    enc.encode_bit_tree(&mut pos_slot[0], slot); // slot = 14
    let num_direct_bits = (slot >> 1) - 1; // 6
    let dist_base = (2 | (slot & 1)) << num_direct_bits; // 128
    enc.encode_direct_bits(0, num_direct_bits - 4); // direct bits (2 bits: 0)
    enc.encode_reverse_bit_tree(&mut align_probs, 0); // align bits (4 bits: 0)
    let rep0 = dist_base; // 128
    assert_eq!(rep0, 128);
    for _ in 0..2 {
        let idx = output.len() - 1 - rep0;
        let b = output[idx];
        output.push(b);
    }
    assert_eq!(output.len(), 132);

    let lzma_stream = enc.finish();

    let config = ZwsConfig {
        version: 13,
        uncompressed_file_length: 132 + 8,
        compressed_length: lzma_stream.len() as u32,
        lc: 0,
        lp: 0,
        pb: 0,
        dict_size: 1 << 20,
    };

    let decompressed =
        swf_container::lzma_decompress_bounded(&lzma_stream, 132, &config, 100_000, 100.0)
            .expect("decompress synthesized LZMA stream exercising slot 14 and direct bits");

    assert_eq!(decompressed.len(), 132);
    assert_eq!(decompressed, output);
}

// --- Validation Functions (Warrant Contract) Tests ---

#[test]
fn test_warrant_validation_functions() {
    assert_eq!(validate_version(1), Ok(true));
    assert_eq!(validate_version(50), Ok(true));
    assert_eq!(validate_version(0), Err(SwfError::UnsupportedVersion));
    assert_eq!(validate_version(51), Err(SwfError::UnsupportedVersion));

    assert_eq!(validate_tag_code(0), Ok(true));
    assert_eq!(validate_tag_code(1023), Ok(true));
    assert_eq!(validate_tag_code(1024), Err(SwfError::InvalidTagLength));

    assert_eq!(validate_ratio(500_000, 10), Ok(true)); // under 1 MB threshold
    assert_eq!(validate_ratio(2_000_000, 50_000), Ok(true)); // 40:1 <= 50:1
    assert_eq!(
        validate_ratio(60_000_000, 1_000_000),
        Err(SwfError::DecompressionBombDetected)
    );
}

#[test]
fn test_adler32() {
    assert_eq!(compute_adler32(b""), 1);
    assert_eq!(compute_adler32(b"Wikipedia"), 0x11E60398);
}

#[test]
fn test_capabilities_token() {
    let cap = PureCapabilities;
    black_box(cap);
}
