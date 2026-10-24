//! # Annex B `-->` comments cost memory linear in the source
//!
//! Security review of #17914 found a quadratic path. A line-leading `-->`
//! comment is two tokens (`--` and `>…`) that the on-token callback
//! suppresses. The generic lexer used to hand every callback a *copy* of
//! the last emitted token, and suppressed tokens never replace that token.
//! So a huge string followed by N comment lines copied the string 2·N
//! times: 4 extra bytes of input bought another two copies.
//!
//! Wall-clock tests cannot see this reliably (a string that fits in cache
//! copies very fast), so this test counts bytes instead. A global allocator
//! tallies every allocation while the lexer runs twice, with and without a
//! 200 KB string before the same 20,000 comment lines. Copying the string
//! per comment would add about 2 × 20,000 × 200 KB = 8 GB between the two
//! runs; lexing linearly adds a few hundred KB.
//!
//! This lives in its own test binary because `#[global_allocator]` applies
//! to the whole binary, and the one test here runs alone in it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use coding_adventures_javascript_lexer::tokenize_javascript;

/// Counts the bytes requested while `COUNTING` is set; otherwise it is
/// just the system allocator.
struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System.alloc` with this `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
        }
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Bytes allocated while lexing `x = "<n × a>"` followed by `lines`
/// line-leading `-->` comments.
fn allocated_for(n: usize, lines: usize) -> usize {
    let mut src = String::from("x = \"");
    src.push_str(&"a".repeat(n));
    src.push('"');
    for _ in 0..lines {
        src.push_str("\n-->");
    }
    src.push('\n');

    ALLOCATED.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);
    let tokens = tokenize_javascript(&src, "es2025");
    COUNTING.store(false, Ordering::SeqCst);
    let allocated = ALLOCATED.load(Ordering::SeqCst);

    let tokens = tokens.expect("lexes");
    assert!(tokens.iter().all(|t| t.value != "--"), "every `-->` line is a comment");
    allocated
}

#[test]
fn html_close_comments_after_a_huge_token_allocate_linearly() {
    // Same 20,000 comment lines; only the string before them grows. Each
    // comment line has a fixed bookkeeping cost, so compare the two runs:
    // the extra allocation must be a small multiple of the extra 200 KB,
    // not 2 × 20,000 copies of it (about 8 GB).
    const LINES: usize = 20_000;
    const BIG: usize = 200_000;
    let small = allocated_for(0, LINES);
    let large = allocated_for(BIG, LINES);
    let extra = large.saturating_sub(small);
    assert!(
        extra < 64 * BIG,
        "a {BIG}-byte string before {LINES} `-->` lines cost {extra} extra bytes: \
         the callback context is copying tokens again"
    );
}
