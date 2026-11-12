//! # SHA-256 conformance (FIPS 180-4)
//!
//! The certificate digests bind security claims, so the hand-rolled hasher is
//! tested against an independent implementation:
//!
//! * the FIPS 180-4 / NIST example messages (empty, "abc", the 448-bit and
//!   896-bit messages, one million `a`);
//! * `tests/data/sha256_vectors.txt`: every length 0..=200 plus larger and
//!   block-boundary lengths, digests produced by Python `hashlib`
//!   (`tests/data/gen_sha256_vectors.py` regenerates the file);
//! * streaming behaviour: any split of the input into `update` calls must give
//!   the one-shot digest, including splits that straddle the 56/64-byte padding
//!   boundaries.

use coding_adventures_ca_capability_analyzer::{sha256, sha256_hex, Sha256};

const VECTORS: &str = include_str!("data/sha256_vectors.txt");

/// Same generator as `gen_sha256_vectors.py`.
fn message(len: usize) -> Vec<u8> {
    (0..len).map(|j| ((j * 167 + len * 13 + 5) % 256) as u8).collect()
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn nist_fips_180_4_example_messages() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // 448-bit message: padding needs a second block.
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    // 896-bit message.
    assert_eq!(
        sha256_hex(
            b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
        ),
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
    );
}

#[test]
fn nist_one_million_a_one_shot_and_streamed() {
    let want = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";
    let data = vec![b'a'; 1_000_000];
    assert_eq!(sha256_hex(&data), want);

    // NIST also specifies feeding it in small pieces.
    let mut h = Sha256::new();
    for chunk in data.chunks(1000) {
        h.update(chunk);
    }
    assert_eq!(hex(h.finalize()), want);

    let mut h = Sha256::new();
    for chunk in data.chunks(63) {
        h.update(chunk);
    }
    assert_eq!(hex(h.finalize()), want);
}

#[test]
fn hashlib_vectors_every_length_0_to_200_plus_boundaries() {
    let mut checked = 0;
    let mut lengths = Vec::new();
    for line in VECTORS.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (len, digest) = line.split_once(' ').expect("vector line is `len digest`");
        let len: usize = len.parse().unwrap();
        assert_eq!(digest.len(), 64, "length {len}: digest must be 64 hex chars");
        assert_eq!(sha256_hex(&message(len)), digest, "length {len}");
        lengths.push(len);
        checked += 1;
    }
    assert!(checked >= 220, "vector file shrank to {checked} entries");
    for n in 0..=200 {
        assert!(lengths.contains(&n), "length {n} missing from vector file");
    }
    // Padding boundaries named in the audit.
    for n in [55, 56, 63, 64, 119, 120] {
        assert!(lengths.contains(&n), "boundary {n} missing");
    }
}

#[test]
fn padding_boundaries_differ_from_each_other() {
    // 55 bytes fits in one block; 56 forces a second; 63/64/65 straddle a block edge.
    let digests: Vec<String> = [55, 56, 57, 63, 64, 65, 119, 120, 121]
        .iter()
        .map(|&n| sha256_hex(&message(n)))
        .collect();
    for i in 0..digests.len() {
        for j in i + 1..digests.len() {
            assert_ne!(digests[i], digests[j]);
        }
    }
}

#[test]
fn every_two_way_split_matches_one_shot() {
    for len in [0usize, 1, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129, 200] {
        let msg = message(len);
        let want = sha256(&msg);
        for cut in 0..=len {
            let mut h = Sha256::new();
            h.update(&msg[..cut]);
            h.update(&msg[cut..]);
            assert_eq!(h.finalize(), want, "len {len}, cut {cut}");
        }
    }
}

#[test]
fn byte_at_a_time_and_odd_chunks_match_one_shot() {
    for len in [0usize, 1, 55, 56, 64, 100, 200, 1000] {
        let msg = message(len);
        let want = sha256(&msg);
        let mut h = Sha256::new();
        for b in &msg {
            h.update(std::slice::from_ref(b));
        }
        assert_eq!(h.finalize(), want, "byte-wise len {len}");
        for chunk_size in [2usize, 3, 7, 13, 31, 33, 63, 65, 127] {
            let mut h = Sha256::new();
            for c in msg.chunks(chunk_size) {
                h.update(c);
            }
            assert_eq!(h.finalize(), want, "len {len} chunk {chunk_size}");
        }
    }
}

#[test]
fn empty_updates_are_noops() {
    let mut h = Sha256::new();
    h.update(b"");
    h.update(b"abc");
    h.update(b"");
    assert_eq!(hex(h.finalize()), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}

#[test]
fn clone_forks_state_independently() {
    let mut a = Sha256::new();
    a.update(b"ab");
    let mut b = a.clone();
    a.update(b"c");
    b.update(b"d");
    assert_eq!(hex(a.finalize()), sha256_hex(b"abc"));
    assert_eq!(hex(b.finalize()), sha256_hex(b"abd"));
}

#[test]
fn default_equals_new() {
    let d: Sha256 = Default::default();
    assert_eq!(hex(d.finalize()), sha256_hex(b""));
}

#[test]
fn single_bit_flip_changes_digest_everywhere() {
    let base = message(130);
    let want = sha256(&base);
    for i in 0..base.len() {
        let mut m = base.clone();
        m[i] ^= 1;
        assert_ne!(sha256(&m), want, "flip at byte {i} must change the digest");
    }
}
