//! SEC-002: a decompression bomb must fail closed.
//!
//! A *decompression bomb* is a short, perfectly well-formed compressed stream
//! that expands to a huge output.  LZW is especially exposed because a single
//! code can stand for thousands of bytes: the code for "the previous string
//! plus its own first byte" (the *tricky token*, see the decoder docs) makes
//! the dictionary grow by one byte per code, so `k` codes of ~13 bits each
//! expand to about `k * k / 2` bytes.  A 15 KB stream is 40 MB; a 128 KB one
//! is two gigabytes.
//!
//! The header's `original_length` does not help: it is attacker-controlled, so
//! the bomb simply declares `0xFFFF_FFFF`.
//!
//! The fix is a hard cap on delivered output that `decompress` enforces while
//! it expands.  These tests state it from the outside, so they keep passing if
//! the implementation behind the cap is rewritten:
//!
//! * a bomb that would exceed the cap is an `Err`, not a multi-gigabyte `Ok`;
//! * a stream that stays under the cap is unaffected, whatever its header says;
//! * a header that declares a *small* length still wins (the decoder stops
//!   early), so the cap never turns a valid truncation into an error.

use lzw::{compress, decompress};

/// The cap, spelled as a literal: the contract (LZ01 and `lzw.warrant`) says
/// 16 MiB, and a test that read the constant back would pass if someone
/// changed it.
const CAP: usize = 16 * 1024 * 1024;

/// A minimal LSB-first bit writer, enough to hand-craft a stream.  (The crate's
/// own `BitWriter` is private; a test that borrowed it would also share its
/// bugs.)
struct Bits {
    buf: u64,
    n: u32,
    out: Vec<u8>,
}

impl Bits {
    fn new() -> Self {
        Bits { buf: 0, n: 0, out: Vec::new() }
    }
    fn put(&mut self, code: u32, size: u32) {
        self.buf |= (code as u64) << self.n;
        self.n += size;
        while self.n >= 8 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            self.n -= 8;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push((self.buf & 0xFF) as u8);
        }
        self.out
    }
}

/// Build the chain bomb: `CLEAR, 65, 258, 259, ..., STOP` with `codes` tricky
/// tokens and the given header.  Decoded, it is `1 + sum(i + 2 for i in
/// 0..codes)` bytes of `'A'`.
///
/// The code width tracks the decoder's `next_code` exactly (see CMP03): it
/// starts at 9 and grows when `next_code` passes a power of two.
fn chain_bomb(codes: usize, declared_length: u32) -> Vec<u8> {
    let mut w = Bits::new();
    let mut size = 9u32;
    let mut next_code = 258usize;
    w.put(256, 9); // CLEAR_CODE, at the initial width
    for code in std::iter::once(65u32).chain((0..codes).map(|i| 258 + i as u32)) {
        w.put(code, size);
        next_code += 1;
        if next_code > (1usize << size) && size < 16 {
            size += 1;
        }
    }
    w.put(257, size); // STOP_CODE
    let mut out = declared_length.to_be_bytes().to_vec();
    out.extend_from_slice(&w.finish());
    out
}

/// Bytes the chain bomb expands to before any cap: `1 + sum(i + 2)`.
fn chain_bomb_expansion(codes: usize) -> usize {
    1 + (0..codes).map(|i| i + 2).sum::<usize>()
}

#[test]
fn a_chain_bomb_over_the_cap_is_rejected() {
    // 9000 codes: about 40 MB of output from a 15 KB stream.
    let codes = 9000;
    assert!(chain_bomb_expansion(codes) > 2 * CAP, "the fixture must exceed the cap");
    let stream = chain_bomb(codes, u32::MAX);
    assert!(stream.len() < 20 * 1024, "the bomb is small: {} bytes", stream.len());
    assert!(decompress(&stream).is_err(), "a decompression bomb must be an Err");
}

#[test]
fn a_chain_bomb_is_rejected_whatever_length_it_declares() {
    // Declaring a length over the cap changes nothing; neither does declaring
    // exactly the cap plus one.  (A declared length at or under the cap stops
    // the decoder early instead; see the next test.)
    let codes = 9000;
    for declared in [u32::MAX, (CAP as u32) + 1, 1 << 30] {
        assert!(decompress(&chain_bomb(codes, declared)).is_err(), "declared {declared}");
    }
}

#[test]
fn a_declared_length_under_the_cap_stops_the_decoder_early() {
    // The same bomb, but the header asks for only 1000 bytes: the decoder
    // delivers exactly those and never reaches the cap.
    let out = decompress(&chain_bomb(9000, 1000)).expect("the declared prefix is valid");
    assert_eq!(out, vec![b'A'; 1000]);
}

#[test]
fn a_stream_just_under_the_cap_still_decodes() {
    // The largest chain whose expansion is <= CAP: as big as the contract allows.
    let mut codes = 1;
    while chain_bomb_expansion(codes + 1) <= CAP {
        codes += 1;
    }
    let out = decompress(&chain_bomb(codes, u32::MAX)).expect("under the cap is valid");
    assert_eq!(out.len(), chain_bomb_expansion(codes));
    assert!(out.iter().all(|&b| b == b'A'));
}

#[test]
fn the_cap_counts_delivered_bytes_not_expanded_bytes() {
    // The first `codes` whose expansion passes the cap.  Left alone it is a
    // bomb; with a declared length of exactly the cap the decoder stops there
    // and delivers the cap.  Only delivered bytes count against the limit.
    let mut codes = 1;
    while chain_bomb_expansion(codes) <= CAP {
        codes += 1;
    }
    assert!(chain_bomb_expansion(codes) > CAP);
    assert!(decompress(&chain_bomb(codes, u32::MAX)).is_err());
    let out = decompress(&chain_bomb(codes, CAP as u32)).expect("exactly the cap is allowed");
    assert_eq!(out.len(), CAP);
}

#[test]
fn highly_redundant_real_data_under_the_cap_round_trips() {
    // Real compressor output, not a hand-crafted stream: a quarter-megabyte of
    // zeros is a several-hundred-fold amplification and must keep working (a
    // ratio limit would have rejected it; the absolute cap does not).
    let zeros = vec![0u8; 1 << 18];
    assert_eq!(decompress(&compress(&zeros)).unwrap(), zeros);
}

#[test]
fn the_public_cap_is_sixteen_mebibytes() {
    assert_eq!(lzw::MAX_OUTPUT_LEN, CAP);
}
