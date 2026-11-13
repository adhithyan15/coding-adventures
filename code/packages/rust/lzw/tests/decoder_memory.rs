//! SEC-002: the decoder's dictionary memory must be linear, not quadratic.
//!
//! The first decoder stored every dictionary entry as its own `Vec<u8>`.  An
//! entry is the previous entry plus one byte, so a chain of `k` entries holds
//! `k * (k + 1) / 2` bytes: the dictionary of a 13 KB stream was as large as
//! its 32 MiB output, and the two together doubled the peak.  The fix stores
//! each entry as `(prefix code, byte, length)` and writes the string out only
//! once, into the output.
//!
//! This binary installs a counting allocator and decodes a 12 MiB chain bomb
//! (under the output cap, so it must succeed).  The live heap peaks at the
//! output buffer plus a fixed dictionary; a quadratic dictionary would add the
//! output's size again.  It is the only test in the binary, so nothing else
//! allocates while the peak is read.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
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
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size >= layout.size() {
                let d = new_size - layout.size();
                let live = LIVE.fetch_add(d, Ordering::Relaxed) + d;
                PEAK.fetch_max(live, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        p
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The chain bomb of `decompression_cap.rs`, written again here so each test
/// binary stands alone: `CLEAR, 65, 258, 259, ..., STOP`.
fn chain_bomb(codes: usize) -> Vec<u8> {
    let (mut buf, mut n, mut out) = (0u64, 0u32, Vec::new());
    let mut size = 9u32;
    let mut next_code = 258usize;
    let mut put = |code: u32, size: u32| {
        buf |= (code as u64) << n;
        n += size;
        while n >= 8 {
            out.push((buf & 0xFF) as u8);
            buf >>= 8;
            n -= 8;
        }
    };
    put(256, 9);
    for code in std::iter::once(65u32).chain((0..codes).map(|i| 258 + i as u32)) {
        put(code, size);
        next_code += 1;
        if next_code > (1usize << size) && size < 16 {
            size += 1;
        }
    }
    put(257, size);
    if n > 0 {
        out.push((buf & 0xFF) as u8);
    }
    let mut stream = u32::MAX.to_be_bytes().to_vec();
    stream.extend_from_slice(&out);
    stream
}

#[test]
fn dictionary_memory_does_not_grow_with_the_square_of_the_stream() {
    // 4900 codes: 1 + sum(i + 2) = about 12 million bytes, under the 16 MiB cap.
    let stream = chain_bomb(4900);
    let expected = 1 + (0..4900usize).map(|i| i + 2).sum::<usize>();

    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let out = lzw::decompress(&stream).expect("a 12 MiB chain is under the cap");
    let peak = PEAK.load(Ordering::Relaxed) - before;

    assert_eq!(out.len(), expected);
    // The output Vec may be up to twice its length (amortised growth); the
    // dictionary of the fixed decoder is well under 2 MiB.  A quadratic
    // dictionary holds `expected` more bytes, which breaks this bound.
    eprintln!("peak {peak} for output {expected}");
    let allowed = 2 * expected + 2 * 1024 * 1024;
    assert!(
        peak <= allowed,
        "peak live heap {peak} bytes for a {expected}-byte output (allowed {allowed}): \
         the dictionary is not linear"
    );
    assert!(peak >= expected, "the output itself is live at the peak");
}
