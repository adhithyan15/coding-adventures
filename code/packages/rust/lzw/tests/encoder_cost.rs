//! SEC-002a: the encoder's cost must be linear in the input, and its output
//! must not change.
//!
//! The first encoder kept its dictionary as `HashMap<Vec<u8>, u16>` and asked
//! "is `w + [b]` known?" for every input byte, which hashes the *whole* current
//! prefix.  On redundant input the prefix grows by one byte per emitted code,
//! so the cost is the sum of the prefix lengths: a megabyte of zeros took
//! seconds, sixteen megabytes minutes.  `compress` has no error channel, so a
//! caller feeding it a large redundant file (or an attacker feeding one) could
//! not even bound the time.  The fix keys the dictionary by `(prefix code,
//! next byte)`, a constant-size key, so each input byte costs one lookup.
//!
//! Two things are tested here:
//!
//! 1. `compress` of a redundant 8 MiB input finishes quickly.  The old
//!    encoder needed about half a minute in a debug build; the bound is
//!    generous (10 s) so a slow CI machine does not flake, and the two
//!    behaviours are four orders of magnitude apart.
//! 2. The output is byte for byte what the old algorithm produced.  The old
//!    encoder is kept below as `reference_compress`, written from CMP03 and
//!    deliberately simple, and compared on structured and seeded random inputs.
//!    The wire format is a contract with every other port, so "faster" must
//!    never mean "different".

use lzw::{compress, decompress};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[test]
fn compress_of_redundant_input_is_linear_time() {
    let zeros = vec![0u8; 8 * 1024 * 1024];
    let start = Instant::now();
    let packed = compress(&zeros);
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "compress of 8 MiB of zeros took {elapsed:?} (quadratic prefix hashing)"
    );
    assert!(packed.len() < 16 * 1024, "zeros compress well: {} bytes", packed.len());
}

/// CMP03 section "Encoding Algorithm", transcribed directly: a dictionary of
/// byte strings, a growing prefix `w`, and the bit packing of the wire format.
fn reference_compress(data: &[u8]) -> Vec<u8> {
    fn put(out: &mut Vec<u8>, buf: &mut u64, n: &mut u32, code: u32, size: u32) {
        *buf |= (code as u64) << *n;
        *n += size;
        while *n >= 8 {
            out.push((*buf & 0xFF) as u8);
            *buf >>= 8;
            *n -= 8;
        }
    }
    let mut dict: HashMap<Vec<u8>, u32> = (0u32..256).map(|b| (vec![b as u8], b)).collect();
    let (mut out, mut buf, mut n) = (Vec::new(), 0u64, 0u32);
    let (mut next_code, mut size) = (258u32, 9u32);
    put(&mut out, &mut buf, &mut n, 256, size);
    let mut w: Vec<u8> = Vec::new();
    for &b in data {
        w.push(b);
        if !dict.contains_key(&w) {
            w.pop();
            put(&mut out, &mut buf, &mut n, dict[&w], size);
            if next_code < 65536 {
                let mut e = w.clone();
                e.push(b);
                dict.insert(e, next_code);
                next_code += 1;
                if next_code > (1 << size) && size < 16 {
                    size += 1;
                }
            } else {
                put(&mut out, &mut buf, &mut n, 256, size);
                dict = (0u32..256).map(|b| (vec![b as u8], b)).collect();
                next_code = 258;
                size = 9;
            }
            w = vec![b];
        }
    }
    if !w.is_empty() {
        put(&mut out, &mut buf, &mut n, dict[&w], size);
    }
    put(&mut out, &mut buf, &mut n, 257, size);
    if n > 0 {
        out.push((buf & 0xFF) as u8);
    }
    let mut stream = (data.len() as u32).to_be_bytes().to_vec();
    stream.extend_from_slice(&out);
    stream
}

/// Small deterministic xorshift generator (no dependencies).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

#[test]
fn output_is_byte_identical_to_the_reference_encoder() {
    let mut rng = Rng(0x1D2B_3C4D_5E6F_7081);
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        b"TOBEORNOTTOBEORTOBEORNOT".to_vec(),
        vec![7; 3000],
        (0..=255u8).collect(),
        (0..70_000u32).map(|i| (i % 251) as u8).collect(),
    ];
    // Random inputs over alphabets of 2, 4, 16 and 256 symbols, lengths up to
    // about 2000 (matches and dictionary growth occur).
    for i in 0..120 {
        let len = (rng.next() % 2000) as usize;
        let alphabet = [2u64, 4, 16, 256][i % 4];
        inputs.push((0..len).map(|_| (rng.next() % alphabet) as u8).collect());
    }
    // One input long enough to fill the 65,536-entry dictionary and trigger
    // the mid-stream CLEAR_CODE reset (random bytes fill it in ~70 KB).
    inputs.push((0..300_000).map(|_| (rng.next() % 256) as u8).collect());

    for data in &inputs {
        let got = compress(data);
        assert_eq!(got, reference_compress(data), "wire format changed for a {}-byte input", data.len());
        assert_eq!(decompress(&got).unwrap(), *data, "round trip for a {}-byte input", data.len());
    }
}
