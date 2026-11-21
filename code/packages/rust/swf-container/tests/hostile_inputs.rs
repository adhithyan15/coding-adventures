//! Hostile-input security regression and fuzz tests for `swf-container` (SWF01).
//!
//! Covers:
//! - CWE-191 / CWE-125 / CWE-400: Crafted `ZwsConfig` (`lc`/`lp`/`pb` out of bounds)
//! - CWE-409: LZMA `ShortRep` decompression bomb ratio enforcement
//! - Malformed dynamic Huffman and stored blocks in `zlib_decompress_bounded`
//! - Truncated tags and negative/oversized long tag lengths in `SwfTagIter`
//! - Seeded 1,000-case hostile byte fuzz campaign across `FWS`/`CWS`/`ZWS`/`BitReader`

#![forbid(unsafe_code)]

use swf_container::{
    lzma_decompress_bounded, zlib_decompress_bounded, BitReader, SwfDecompressorConfig, SwfError,
    SwfMovie, SwfTagIter, ZwsConfig,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }
}

struct LzmaRangeEncoder {
    low: u64,
    range: u32,
    cache_size: u64,
    cache: u8,
    out: Vec<u8>,
}

impl LzmaRangeEncoder {
    fn new() -> Self {
        Self {
            low: 0,
            range: 0xFFFF_FFFF,
            cache_size: 1,
            cache: 0,
            out: Vec::new(),
        }
    }

    fn shift_low(&mut self) {
        let low_u32 = self.low as u32;
        if low_u32 < 0xFF00_0000 || (self.low >> 32) != 0 {
            let mut temp = self.cache;
            loop {
                self.out.push(temp.wrapping_add((self.low >> 32) as u8));
                temp = 0xFF;
                self.cache_size -= 1;
                if self.cache_size == 0 {
                    break;
                }
            }
            self.cache = (low_u32 >> 24) as u8;
        }
        self.cache_size += 1;
        self.low = (low_u32 << 8) as u64;
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
        if self.range < 0x0100_0000 {
            self.range <<= 8;
            self.shift_low();
        }
    }

    fn finish(mut self) -> Vec<u8> {
        for _ in 0..5 {
            self.shift_low();
        }
        self.out
    }
}

#[test]
fn hostile_crafted_zws_config_bounds_rejected() {
    let dummy_lzma = [0u8; 16];
    let bad_configs = [
        ZwsConfig {
            version: 13,
            uncompressed_file_length: 16,
            compressed_length: 16,
            lc: 9,
            lp: 0,
            pb: 0,
            dict_size: 4096,
        },
        ZwsConfig {
            version: 13,
            uncompressed_file_length: 16,
            compressed_length: 16,
            lc: 0,
            lp: 5,
            pb: 0,
            dict_size: 4096,
        },
        ZwsConfig {
            version: 13,
            uncompressed_file_length: 16,
            compressed_length: 16,
            lc: 0,
            lp: 0,
            pb: 5,
            dict_size: 4096,
        },
        ZwsConfig {
            version: 13,
            uncompressed_file_length: 16,
            compressed_length: 16,
            lc: 8,
            lp: 5,
            pb: 0,
            dict_size: 4096,
        },
        ZwsConfig {
            version: 13,
            uncompressed_file_length: 16,
            compressed_length: 16,
            lc: 255,
            lp: 255,
            pb: 255,
            dict_size: 4096,
        },
    ];

    for cfg in bad_configs {
        let err = lzma_decompress_bounded(&dummy_lzma, 8, &cfg, 1024, 50.0)
            .expect_err("out-of-bounds ZwsConfig must be rejected");
        assert_eq!(err, SwfError::CorruptedData);
    }

    // Crafted ZWS prop_byte >= 225 must also fail in parse_zws_header.
    let mut bad_zws = vec![0u8; 25];
    bad_zws[0..3].copy_from_slice(b"ZWS");
    bad_zws[3] = 13;
    bad_zws[12] = 225;
    let err = ZwsConfig::parse_zws_header(&bad_zws).expect_err("prop_byte >= 225 must fail");
    assert_eq!(err, SwfError::CorruptedData);
}

#[test]
fn hostile_lzma_short_rep_bomb_triggers_ratio_limit() {
    // Encode 1 literal byte (0x41 = 'A') followed by 1,100,000 ShortRep steps
    // (is_match=1, is_rep=1, is_rep_g0=0, is_rep0_long=0) with lc=0, lp=0, pb=0.
    let mut enc = LzmaRangeEncoder::new();
    let mut is_match = [1024u16; 12 * 16];
    let mut is_rep = [1024u16; 12];
    let mut is_rep_g0 = [1024u16; 12];
    let mut is_rep0_long = [1024u16; 12 * 16];
    let mut lit_probs = [1024u16; 0x300];

    // Step 0: literal 0x41 ('A') at state=0, pos_state=0
    enc.encode_bit(&mut is_match[0], 0);
    let mut sym = 1usize;
    for bit_idx in (0..8).rev() {
        let b = (0x41u32 >> bit_idx) & 1;
        enc.encode_bit(&mut lit_probs[sym], b);
        sym = (sym << 1) | (b as usize);
    }

    // Steps 1..=1_100_000: ShortRep (copies output[len - 1 - 0] = 0x41)
    let mut state = 0usize;
    for _ in 0..1_100_000 {
        let match_idx = state << 4; // pb=0 -> pos_state=0
        enc.encode_bit(&mut is_match[match_idx], 1);
        enc.encode_bit(&mut is_rep[state], 1);
        enc.encode_bit(&mut is_rep_g0[state], 0);
        enc.encode_bit(&mut is_rep0_long[match_idx], 0);
        state = if state < 7 { 9 } else { 11 };
    }

    let payload = enc.finish();
    // Payload should remain compact (< 16 KiB) for 1.1 MB output (> 67:1 ratio > 50:1 cap).
    assert!(
        payload.len() < 16_384,
        "ShortRep payload unexpectedly large: {}",
        payload.len()
    );

    let cfg = ZwsConfig {
        version: 13,
        uncompressed_file_length: 1_100_009,
        compressed_length: payload.len() as u32,
        lc: 0,
        lp: 0,
        pb: 0,
        dict_size: 4096,
    };

    let err = lzma_decompress_bounded(&payload, 1_100_001, &cfg, 10_000_000, 50.0)
        .expect_err("ShortRep bomb must trigger DecompressionBombDetected");
    assert_eq!(err, SwfError::DecompressionBombDetected);
}

#[test]
fn hostile_malformed_zlib_dynamic_and_stored_blocks() {
    // Valid zlib header (CM=8, CINFO=7 => 0x78, FCHECK => 0x9C)
    // 1) Stored block (BFINAL=1, BTYPE=00) with mismatched LEN/NLEN
    let bad_stored = [0x78, 0x9C, 0x01, 0x05, 0x00, 0x00, 0x00];
    assert!(zlib_decompress_bounded(&bad_stored, 1024, 50.0).is_err());

    // 2) Reserved BTYPE=11
    let bad_btype = [0x78, 0x9C, 0x07, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(
        zlib_decompress_bounded(&bad_btype, 1024, 50.0),
        Err(SwfError::CorruptedData)
    );

    // 3) Dynamic Huffman (BFINAL=1, BTYPE=10) with HLIT > 286 (HLIT=31 => 257+31=288 > 286)
    let bad_dyn_hlit = [0x78, 0x9C, 0xFD, 0x01, 0x00, 0x00, 0x00, 0x00];
    assert!(zlib_decompress_bounded(&bad_dyn_hlit, 1024, 50.0).is_err());

    // 4) Dynamic Huffman truncated mid-tree
    let trunc_dyn = [0x78, 0x9C, 0x05, 0x00, 0x00, 0x00];
    assert!(zlib_decompress_bounded(&trunc_dyn, 1024, 50.0).is_err());

    // 5) Bad zlib FCHECK
    let bad_fcheck = [0x78, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01];
    assert_eq!(
        zlib_decompress_bounded(&bad_fcheck, 1024, 50.0),
        Err(SwfError::CorruptedData)
    );
}

#[test]
fn hostile_truncated_and_negative_long_tag_lengths() {
    // Short tag (code=1, len=10 => (1<<6)|10 = 0x004A LE) with only 3 bytes remaining
    let trunc_short = [0x4A, 0x00, 0xAA, 0xBB, 0xCC];
    let mut iter = SwfTagIter::from_payload(&trunc_short);
    assert_eq!(iter.next(), Some(Err(SwfError::UnexpectedEof)));

    // Long tag (len=0x3F) with negative signed i32 length (-1 == 0xFFFF_FFFF)
    let neg_long = [0x7F, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00];
    let mut iter = SwfTagIter::from_payload(&neg_long);
    assert_eq!(iter.next(), Some(Err(SwfError::InvalidTagLength)));

    // Long tag claiming 0x7FFF_FFFF bytes on a 10-byte buffer
    let huge_long = [0x7F, 0x00, 0xFF, 0xFF, 0xFF, 0x7F, 0x00, 0x00];
    let mut iter = SwfTagIter::from_payload(&huge_long);
    assert_eq!(iter.next(), Some(Err(SwfError::UnexpectedEof)));

    // Truncated long tag header (only 4 bytes instead of 6)
    let trunc_long_hdr = [0x7F, 0x00, 0x01, 0x00];
    let mut iter = SwfTagIter::from_payload(&trunc_long_hdr);
    assert_eq!(iter.next(), Some(Err(SwfError::UnexpectedEof)));
}

#[test]
fn hostile_byte_fuzz_1000_cases_no_panics() {
    let mut rng = SplitMix64(0x5F01_CAFE_DEAD_BEEF);
    let cfg = SwfDecompressorConfig::default();

    for i in 0..1_000 {
        let len = rng.next_usize(128);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();

        // Force signature prefixes on most iterations so parsers reach deep branches.
        if buf.len() >= 4 {
            match i % 4 {
                0 => buf[0..3].copy_from_slice(b"FWS"),
                1 => buf[0..3].copy_from_slice(b"CWS"),
                2 => buf[0..3].copy_from_slice(b"ZWS"),
                _ => {}
            }
        }

        let _ = SwfMovie::from_swf_bytes(&buf, &cfg);

        // Also fuzz SwfTagIter directly
        let mut tag_iter = SwfTagIter::from_payload(&buf);
        for _ in 0..16 {
            match tag_iter.next() {
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            }
        }

        // Fuzz BitReader primitives
        let mut br = BitReader::from_bits(&buf);
        let _ = br.read_rect();
        let mut br2 = BitReader::from_bits(&buf);
        let _ = br2.read_matrix();
        let mut br3 = BitReader::from_bits(&buf);
        let _ = br3.read_cxform_with_alpha();
        let mut br4 = BitReader::from_bits(&buf);
        let _ = br4.read_ub(rng.next_usize(33));

        // Fuzz zlib_decompress_bounded directly
        let _ = zlib_decompress_bounded(&buf, 4096, 50.0);

        // Fuzz lzma_decompress_bounded with arbitrary ZwsConfig
        let zcfg = ZwsConfig {
            version: 13,
            uncompressed_file_length: 64,
            compressed_length: buf.len() as u32,
            lc: (rng.next_u64() % 12) as u8,
            lp: (rng.next_u64() % 8) as u8,
            pb: (rng.next_u64() % 8) as u8,
            dict_size: 4096,
        };
        let _ = lzma_decompress_bounded(&buf, 32, &zcfg, 4096, 50.0);
    }
}
