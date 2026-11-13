//! SEC-002: the linked-entry decoder must behave exactly like the string
//! decoder it replaced, on every stream both can finish.
//!
//! The fix changed how the decoder stores its dictionary (a chain of
//! `(prefix, byte)` links instead of a `Vec<u8>` per entry) and added an output
//! cap.  Neither may change what a stream *means*.  This file keeps the old
//! decoder as `reference_decompress`, transcribed from CMP03's decoding
//! algorithm with no cap, and feeds both the same streams:
//!
//! * every stream the encoder produces (the two must return the input);
//! * seeded random garbage behind a valid CLEAR code, which hits the invalid
//!   code, tricky-token, mid-stream CLEAR, truncation and dictionary-full
//!   paths with hostile codes the encoder would never emit.
//!
//! For each stream the two decoders must agree on `Ok` versus `Err` and, for
//! `Ok`, on every byte.  (Error *messages* are not compared: the contract is
//! "an error", see LZ01.)  Streams whose reference output passes the cap are
//! skipped; those are the point of the cap, tested in `decompression_cap.rs`.

use lzw::{compress, decompress, MAX_OUTPUT_LEN};

/// CMP03 decoding, as the first implementation wrote it: one `Vec<u8>` per
/// dictionary entry, no output cap.
fn reference_decompress(data: &[u8]) -> Result<Vec<u8>, ()> {
    if data.len() < 4 {
        return Err(());
    }
    let original_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let payload = &data[4..];

    // LSB-first reader of `size`-bit codes; `None` at the end of the input.
    let (mut pos, mut buf, mut have) = (0usize, 0u64, 0u32);
    let mut read = |size: u32| -> Option<usize> {
        while have < size {
            if pos >= payload.len() {
                if have == 0 {
                    return None;
                }
                break;
            }
            buf |= (payload[pos] as u64) << have;
            pos += 1;
            have += 8;
        }
        if have < size {
            return None;
        }
        let code = (buf & ((1u64 << size) - 1)) as usize;
        buf >>= size;
        have -= size;
        Some(code)
    };

    let mut dict: Vec<Vec<u8>> = (0u8..=255).map(|b| vec![b]).collect();
    dict.push(vec![]);
    dict.push(vec![]);
    let (mut next_code, mut size) = (258usize, 9u32);
    let mut prev: Option<usize> = None;
    let mut out: Vec<u8> = Vec::new();

    if read(size).ok_or(())? != 256 {
        return Err(());
    }
    loop {
        if out.len() >= original_length {
            break;
        }
        let Some(code) = read(size) else { break };
        if code == 256 {
            dict.truncate(258);
            next_code = 258;
            size = 9;
            prev = None;
            continue;
        }
        if code == 257 {
            break;
        }
        let entry = if code < dict.len() {
            dict[code].clone()
        } else if code == dict.len() {
            let p = prev.ok_or(())?;
            let mut e = dict[p].clone();
            e.push(dict[p][0]);
            e
        } else {
            return Err(());
        };
        out.extend_from_slice(&entry);
        if next_code < (1 << 16) {
            next_code += 1;
            if next_code > (1usize << size) && size < 16 {
                size += 1;
            }
        }
        if let Some(p) = prev {
            if dict.len() < (1 << 16) {
                let mut e = dict[p].clone();
                e.push(entry[0]);
                dict.push(e);
            }
        }
        prev = Some(code);
        if out.len() > 4 * MAX_OUTPUT_LEN {
            return Err(()); // far past the cap: stop the reference too
        }
    }
    out.truncate(original_length);
    Ok(out)
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

/// Both decoders on one stream; skip streams the cap rejects.
fn agree(stream: &[u8]) {
    let want = reference_decompress(stream);
    let got = decompress(stream);
    match want {
        Ok(bytes) if bytes.len() > MAX_OUTPUT_LEN => {
            assert!(got.is_err(), "over the cap must be rejected");
        }
        Ok(bytes) => assert_eq!(got.as_ref().ok(), Some(&bytes), "bytes differ for {stream:02x?}"),
        Err(()) => assert!(got.is_err(), "reference rejects but decompress accepted {stream:02x?}"),
    }
}

#[test]
fn encoder_streams_decode_identically() {
    let mut rng = Rng(0xDEC0_DE12_3456_789A);
    for i in 0..200 {
        let len = (rng.next() % 3000) as usize;
        let alphabet = [1u64, 2, 4, 16, 256][i % 5];
        let data: Vec<u8> = (0..len).map(|_| (rng.next() % alphabet) as u8).collect();
        let stream = compress(&data);
        assert_eq!(reference_decompress(&stream).unwrap(), data);
        agree(&stream);
    }
}

#[test]
fn declared_lengths_that_cut_an_entry_short_agree() {
    // Streams of long entries, cut off mid-entry by the declared length: the
    // linked decoder must skip trailing links and deliver the exact prefix.
    let data = vec![b'Z'; 50_000];
    let stream = compress(&data);
    for declared in [0u32, 1, 2, 3, 257, 258, 1000, 4999, 5000, 49_999, 50_000, 60_000] {
        let mut s = stream.clone();
        s[..4].copy_from_slice(&declared.to_be_bytes());
        agree(&s);
    }
}

#[test]
fn random_garbage_behind_a_clear_code_agrees() {
    let mut rng = Rng(0x0BAD_C0DE_FEED_F00D);
    for i in 0..4000 {
        let len = (rng.next() % 400) as usize;
        // Bias the alphabet so small codes (valid references) are common.
        let mask = [0xFFu64, 0x0F, 0x03, 0xFF][i % 4];
        let mut stream = u32::MAX.to_be_bytes().to_vec();
        stream.extend_from_slice(&[0x00, 0x01]); // CLEAR_CODE (256) at 9 bits, LSB first
        stream.extend((0..len).map(|_| (rng.next() & mask) as u8));
        // Half the streams get a header that stops the decoder part-way.
        if i % 2 == 1 {
            let declared = (rng.next() % 5000) as u32;
            stream[..4].copy_from_slice(&declared.to_be_bytes());
        }
        agree(&stream);
    }
}

#[test]
fn every_two_byte_payload_after_a_clear_agrees() {
    // CLEAR (9 bits) followed by every possible 16 free bits: all 65,536
    // values, covering one full 9-bit code and the start of a second, so every
    // first data code (literal, control, invalid, tricky) is hit.
    for lo in 0..=255u8 {
        for hi in 0..=255u8 {
            let stream = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x01 | (lo << 1), (lo >> 7) | (hi << 1), hi >> 7];
            agree(&stream);
        }
    }
}
