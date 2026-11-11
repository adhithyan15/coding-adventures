//! Security regression tests: hostile headers must not drive allocation.
//!
//! This test binary installs a tiny allocator wrapper that refuses (reporting
//! on stderr, which makes the process abort) any single allocation larger than
//! `MAX_ALLOC`, and tracks the peak number of live bytes.  If a decompressor
//! regresses to `Vec::with_capacity(header_value)` the binary dies immediately
//! with a clear message instead of OOM-killing the machine, and the peak
//! assertions catch slower growth.  The tiny inputs below never legitimately
//! need more than a few hundred KiB.

use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// No single allocation in this test binary may exceed 32 MiB.
const MAX_ALLOC: usize = 32 * 1024 * 1024;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct Guard;

unsafe impl GlobalAlloc for Guard {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() > MAX_ALLOC {
            let _ = writeln!(
                std::io::stderr(),
                "REGRESSION: attempted {}-byte allocation (limit {})",
                layout.size(),
                MAX_ALLOC
            );
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > MAX_ALLOC {
            let _ = writeln!(
                std::io::stderr(),
                "REGRESSION: attempted {}-byte reallocation (limit {})",
                new_size,
                MAX_ALLOC
            );
            return std::ptr::null_mut();
        }
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            let live = if new_size >= layout.size() {
                let d = new_size - layout.size();
                LIVE.fetch_add(d, Ordering::Relaxed) + d
            } else {
                let d = layout.size() - new_size;
                LIVE.fetch_sub(d, Ordering::Relaxed) - d
            };
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }
}

#[global_allocator]
static GLOBAL: Guard = Guard;

/// Runs `f`, asserting it finishes quickly.  Memory is policed by `Guard`.
fn quick<R>(f: impl FnOnce() -> R) -> R {
    let start = Instant::now();
    let r = f();
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "hostile input took {:?}",
        start.elapsed()
    );
    r
}

/// Small deterministic xorshift64* PRNG (no external dependencies).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// Every byte string of length 0, 1 and 2.
fn all_short_inputs() -> Vec<Vec<u8>> {
    let mut v = vec![vec![]];
    for a in 0..=255u8 {
        v.push(vec![a]);
    }
    for a in 0..=255u8 {
        for b in 0..=255u8 {
            v.push(vec![a, b]);
        }
    }
    v
}

/// Seeded random inputs of length 3..=600 over small alphabets so that
/// matches (and LZW dictionary growth) actually occur.
fn random_inputs() -> Vec<Vec<u8>> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut out = Vec::new();
    for i in 0..600 {
        let len = 3 + (rng.next() % 598) as usize;
        let alphabet = [2u64, 4, 16, 256][i % 4];
        out.push((0..len).map(|_| (rng.next() % alphabet) as u8).collect());
    }
    out
}

fn assert_peak_small() {
    let peak = PEAK.load(Ordering::Relaxed);
    assert!(peak < 64 * 1024 * 1024, "peak live bytes {peak} too large");
}

use lz78::{compress, decompress};

fn hdr(len: u32, count: u32) -> Vec<u8> {
    let mut v = len.to_be_bytes().to_vec();
    v.extend_from_slice(&count.to_be_bytes());
    v
}

#[test]
fn max_header_only() {
    assert!(quick(|| decompress(&[0xFF; 8])).is_empty());
}

#[test]
fn max_length_zero_tokens() {
    assert!(quick(|| decompress(&hdr(u32::MAX, 0))).is_empty());
}

#[test]
fn count_larger_than_remaining_bytes_yields_present_tokens() {
    let mut d = hdr(2, u32::MAX);
    d.extend_from_slice(&[0, 0, b'h', 0, 0, 0, b'i', 0]);
    assert_eq!(quick(|| decompress(&d)), b"hi");
    d.extend_from_slice(&[0, 0, 0]); // trailing partial token ignored
    assert_eq!(decompress(&d), b"hi");
}

#[test]
fn length_larger_than_tokens_supply() {
    let mut d = hdr(u32::MAX, 1);
    d.extend_from_slice(&[0, 0, b'h', 0]);
    assert_eq!(quick(|| decompress(&d)), b"h");
}

#[test]
fn dangling_dict_index_does_not_panic() {
    let mut d = hdr(10, 2);
    d.extend_from_slice(&[0, 0, b'a', 0, 0x27, 0x0F, b'b', 0]);
    assert_eq!(quick(|| decompress(&d)), b"a");
}

#[test]
fn chain_tokens_stop_at_declared_length() {
    // 60k tokens, each extending the previous entry: unbounded expansion
    // would be ~1.8e9 bytes.  Declared length 1000 must cap the work.
    let n: u32 = 60_000;
    let mut d = hdr(1000, n);
    for i in 0..n {
        d.extend_from_slice(&(i as u16).to_be_bytes());
        d.extend_from_slice(&[b'a', 0]);
    }
    let out = quick(|| decompress(&d));
    assert_eq!(out.len(), 1000);
}

#[test]
fn bounded_memory_after_hostile_cases() {
    assert_peak_small();
}

fn round_trip(data: &[u8]) {
    let c = compress(data, 65536);
    assert_eq!(decompress(&c), data, "round trip failed for {data:?}");
}

#[test]
fn round_trip_all_inputs_up_to_length_2() {
    for d in all_short_inputs() {
        round_trip(&d);
    }
}

#[test]
fn round_trip_seeded_random_inputs() {
    for d in random_inputs() {
        round_trip(&d);
    }
}
