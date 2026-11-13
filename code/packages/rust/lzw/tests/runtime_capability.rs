//! VER08b runtime capability conformance for `lzw`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation).  This test proves it at run time: the *full public API* --
//! `compress` and `decompress`, including hostile input and the output cap
//! (SEC-002) -- is exercised inside a process that has been sandboxed,
//! mechanically from that manifest, to the minimal armed baseline (allocator +
//! exit + TAP output).  An independent `strace` observer must agree, negative
//! controls must be caught, and (when Docker works) the same policy is run in a
//! container with no network, a read-only root and no capabilities.
//!
//! Phase A (`prepare`, unconstrained) builds every input vector, including the
//! hand-crafted decompression bombs.  Phase B (the returned closure, armed)
//! only calls the library and returns named checks: no file, socket, clock,
//! environment or process access is possible there, and none is allowed to
//! appear.
//!
//! Hostile input is *the point* for a decompressor, so phase B feeds it plenty:
//! truncated headers, a missing CLEAR code, invalid codes, headers that declare
//! 4 GiB, and streams that expand to tens of megabytes.  None of these may
//! panic (a panic prints to stderr, which the jail forbids, so a panic here
//! would fail the gate loudly), and the bomb must be an `Err`.  The 16 MiB the
//! decoder is allowed to deliver is allocated inside the jail, which exercises
//! the allocator's large-allocation path (`mmap`) under the seccomp filter.
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports a
//! pass it did not earn.  Set `RCG_STRICT=1` to turn not-live into failure.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use lzw::{compress, decompress, MAX_OUTPUT_LEN};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

/// LSB-first bit packer for hand-crafted streams (the crate's own is private).
fn pack(codes: &[(u32, u32)]) -> Vec<u8> {
    let (mut buf, mut n, mut out) = (0u64, 0u32, Vec::new());
    for &(code, size) in codes {
        buf |= (code as u64) << n;
        n += size;
        while n >= 8 {
            out.push((buf & 0xFF) as u8);
            buf >>= 8;
            n -= 8;
        }
    }
    if n > 0 {
        out.push((buf & 0xFF) as u8);
    }
    out
}

/// `CLEAR, 65, 258, 259, ..., STOP` behind a header declaring `declared`:
/// decoded, `1 + sum(i + 2 for i in 0..codes)` bytes of `'A'` (SEC-002).  The
/// code width follows the decoder's `next_code`, as in CMP03.
fn chain_bomb(codes: usize, declared: u32) -> Vec<u8> {
    let mut seq = vec![(256u32, 9u32)];
    let (mut size, mut next_code) = (9u32, 258usize);
    for code in std::iter::once(65u32).chain((0..codes).map(|i| 258 + i as u32)) {
        seq.push((code, size));
        next_code += 1;
        if next_code > (1usize << size) && size < 16 {
            size += 1;
        }
    }
    seq.push((257, size));
    let mut out = declared.to_be_bytes().to_vec();
    out.extend_from_slice(&pack(&seq));
    out
}

/// Small deterministic xorshift generator (no dependencies, no clock).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// Phase A: build every input vector.  Nothing here is needed by phase B except
/// through the captured values.
fn prepare() -> Exercise {
    // Warm the allocator (phase A, unconstrained).  glibc serves a block above its
    // mmap threshold (128 KiB at first) with `mmap`, and grows it with `mremap`,
    // which the VER08b baseline does not allow (`mmap` and `munmap` are in it,
    // `mremap` is not: "nothing is here just in case").  A decompressor's output
    // `Vec` grows past 256 KiB by design, so the first run of this gate failed with
    // `RCG001 mremap`.  Freeing one block just under glibc's maximum threshold
    // (32 MiB) makes glibc raise its mmap threshold to that size, so every block
    // the armed phase allocates, up to 16 MiB of output and its doubling, comes
    // from the `brk` heap, which the baseline allows.  If this ever stops working
    // (another libc) the gate fails loudly with RCG001, not silently.
    drop(std::hint::black_box(vec![1u8; 31 * 1024 * 1024]));

    let mut rng = Rng(0x4C5A_5731_4A41_494C);

    // Inputs for the round trip: the CMP03 vectors, boundaries, and data that
    // reaches every encoder state (width growth, the tricky token, a full
    // dictionary and its CLEAR reset).
    let mut round_trips: Vec<Vec<u8>> = vec![
        vec![],
        b"A".to_vec(),
        b"AB".to_vec(),
        b"ABABAB".to_vec(),
        b"AAAAAAA".to_vec(),
        b"TOBEORNOTTOBEORTOBEORNOT".to_vec(),
        (0..=255u8).collect(),
        (0..=255u8).chain(0..=255u8).collect(),
        vec![0u8; 70_000],
        vec![0xFFu8; 300],
        (0..70_000u32).map(|i| (i % 251) as u8).collect(),
    ];
    // Random bytes over alphabets of 2, 4, 16 and 256 symbols.
    for i in 0..24 {
        let len = (rng.next() % 3000) as usize;
        let alphabet = [2u64, 4, 16, 256][i % 4];
        round_trips.push((0..len).map(|_| (rng.next() % alphabet) as u8).collect());
    }
    // 300 KB of random bytes fills the 65,536-entry dictionary and forces the
    // encoder's mid-stream CLEAR_CODE reset (and the decoder's mirror of it).
    round_trips.push((0..300_000).map(|_| (rng.next() % 256) as u8).collect());

    // Known wire bytes (CMP03 vectors, from an independent implementation).
    let abab_stream = vec![0x00, 0x00, 0x00, 0x06, 0x00, 0x83, 0x08, 0x11, 0x28, 0x30, 0x20];
    let seven_a_stream = vec![0x00, 0x00, 0x00, 0x07, 0x00, 0x83, 0x08, 0x1C, 0x18, 0x24, 0x20];
    let empty_stream = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x02];
    let mid_stream_clear = vec![0x00, 0x00, 0x00, 0x02, 0x00, 0x83, 0x00, 0x0C, 0x12, 0x10];

    // Hostile streams.
    let short_inputs: Vec<Vec<u8>> = vec![vec![], vec![0], vec![0, 0], vec![0xFF, 0xFF, 0xFF]];
    let header_only = vec![0xFFu8, 0xFF, 0xFF, 0xFF];
    let not_clear = vec![0x00u8, 0x00, 0x00, 0x01, 0xFF, 0x00, 0x00];
    // CLEAR then code 259 while the dictionary holds 258 entries.
    let invalid_code = vec![0x00u8, 0x00, 0x00, 0x01, 0x00, 0x07, 0x02, 0x00];
    // CLEAR then 258: the tricky token with no previous string.
    let tricky_no_prev = vec![0x00u8, 0x00, 0x00, 0x01, 0x00, 0x05, 0x02, 0x00];
    // A small valid stream behind a 4 GiB declaration: delivers the stream.
    let huge_header_small_stream = vec![0xFFu8, 0xFF, 0xFF, 0xFF, 0x00, 0x83, 0x04, 0x04];

    // Decompression bombs (SEC-002).  9,000 chained codes would deliver about
    // 40 MB from a 15 KB stream, so they are rejected; a chain that stays under
    // the cap decodes; a small declaration stops a bomb at once.
    let mut under = 1;
    let expansion = |codes: usize| 1 + (0..codes).map(|i| i + 2).sum::<usize>();
    while expansion(under + 1) <= MAX_OUTPUT_LEN {
        under += 1;
    }
    let bomb = chain_bomb(9000, u32::MAX);
    let bomb_over_cap_declared = chain_bomb(9000, (MAX_OUTPUT_LEN as u32) + 1);
    let bomb_cut_short = chain_bomb(9000, 1000);
    let bomb_exactly_cap = chain_bomb(9000, MAX_OUTPUT_LEN as u32);
    let under_cap = chain_bomb(under, u32::MAX);
    let under_cap_len = expansion(under);

    Box::new(move || {
        let mut checks = Vec::new();

        let round_trip_ok = round_trips.iter().all(|d| decompress(&compress(d)).as_deref() == Ok(&d[..]));
        checks.push(check("round trip: vectors, boundaries, runs, width growth, dictionary reset", round_trip_ok));

        let header_ok = round_trips.iter().take(8).all(|d| {
            let c = compress(d);
            c.len() >= 7 && c[..4] == (d.len() as u32).to_be_bytes()
        });
        checks.push(check("compress: 4-byte big-endian length header, at least 7 bytes", header_ok));

        checks.push(check(
            "compress: known wire bytes (empty, ABABAB, AAAAAAA)",
            compress(b"") == empty_stream
                && compress(b"ABABAB") == abab_stream
                && compress(b"AAAAAAA") == seven_a_stream,
        ));
        checks.push(check(
            "decompress: known streams, tricky token, mid-stream CLEAR",
            decompress(&empty_stream).as_deref() == Ok(&b""[..])
                && decompress(&abab_stream).as_deref() == Ok(&b"ABABAB"[..])
                && decompress(&seven_a_stream).as_deref() == Ok(&b"AAAAAAA"[..])
                && decompress(&mid_stream_clear).as_deref() == Ok(&b"AA"[..]),
        ));

        checks.push(check(
            "decompress: inputs of length 0..=3 (no header) are rejected",
            short_inputs.iter().all(|s| decompress(s).is_err()),
        ));
        checks.push(check(
            "decompress: header only, missing CLEAR, invalid code, tricky token with no previous string",
            decompress(&header_only).is_err()
                && decompress(&not_clear).is_err()
                && decompress(&invalid_code).is_err()
                && decompress(&tricky_no_prev).is_err(),
        ));
        checks.push(check(
            "decompress: a 4 GiB declared length on a small valid stream is not an allocation size",
            decompress(&huge_header_small_stream).as_deref() == Ok(&b"A"[..]),
        ));

        checks.push(check(
            "decompress: a 40 MB chain bomb from a 15 KB stream is rejected, whatever it declares",
            bomb.len() < 20 * 1024
                && decompress(&bomb).is_err()
                && decompress(&bomb_over_cap_declared).is_err(),
        ));
        checks.push(check(
            "decompress: a declared length stops a bomb: 1000 bytes, and exactly the 16 MiB cap",
            decompress(&bomb_cut_short).as_deref() == Ok(&vec![b'A'; 1000][..])
                && decompress(&bomb_exactly_cap).map(|v| v.len()) == Ok(MAX_OUTPUT_LEN),
        ));
        checks.push(check(
            "decompress: the largest chain under the cap decodes in full",
            decompress(&under_cap).map(|v| v.len()) == Ok(under_cap_len),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/lzw",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
