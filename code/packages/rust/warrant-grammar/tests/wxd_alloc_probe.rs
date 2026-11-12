//! VER04k sections 2.7 ("Check before allocation") and 7.2: an allocation
//! probe in the Rust ground evaluator.
//!
//! A meter that charges *after* allocating is one of the evaluator mutants of
//! section 2.12, and no value comparison can see it: the answer is the same
//! W0703 either way.  What differs is that the mutant has already asked the
//! allocator for the whole result (`Vec::with_capacity(65_537)`, or, for a
//! 2^70 count, an amount that aborts the process).  So this test binary
//! installs a counting `#[global_allocator]` and asserts that the refused
//! requests allocate (almost) nothing.
//!
//! It is its own test binary because a binary can have only one global
//! allocator.  The counters are `const` thread-locals without destructors
//! (touching a lazily initialised or `Drop` thread-local from inside the
//! allocator could itself allocate), and only the measuring thread counts, so
//! the harness's other threads cannot disturb a measurement.
//!
//! A positive control (`range(65536)`, which must allocate its result) proves
//! the probe can see an allocation at all.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::HashMap;

use coding_adventures_warrant_grammar::codegen::{
    CodegenErrorKind, CodegenLimits, DiagnosticCode as CgCode, EvalValue, Evaluator,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static TOTAL: Cell<usize> = const { Cell::new(0) };
    static LARGEST: Cell<usize> = const { Cell::new(0) };
}

struct Probe;

// SAFETY: every method forwards to `System` unchanged; the only addition is
// bookkeeping in `const` thread-locals, which neither allocate nor unwind.
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: as for `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        // SAFETY: as for `alloc`; `ptr` came from this allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn record(size: usize) {
    let _ = COUNTING.try_with(|on| {
        if on.get() {
            let _ = TOTAL.try_with(|t| t.set(t.get().saturating_add(size)));
            let _ = LARGEST.try_with(|l| l.set(l.get().max(size)));
        }
    });
}

#[global_allocator]
static PROBE: Probe = Probe;

/// Run `f` with this thread's allocations counted; return its result, the
/// total bytes requested and the largest single request.
fn measured<R>(f: impl FnOnce() -> R) -> (R, usize, usize) {
    TOTAL.with(|t| t.set(0));
    LARGEST.with(|l| l.set(0));
    COUNTING.with(|c| c.set(true));
    let r = f();
    COUNTING.with(|c| c.set(false));
    (r, TOTAL.with(Cell::get), LARGEST.with(Cell::get))
}

// 65,536 copies of e2's 1 KiB operand are over the data budget.
const _: () =
    assert!(65_536 * (32 + 1024) > coding_adventures_warrant_grammar::codegen::DATA_BUDGET_BYTES);

/// The most a refused request may allocate: the argument vector of one call
/// and the diagnostic, never a result-sized buffer.
const SMALL: usize = 4096;

#[test]
fn refused_sequence_requests_allocate_nothing_before_w0703() {
    let src = "warrant probe v1\n  example e0:\n    replicate(0u8, 4294967296 * 4294967296 * 64)\n  example e1:\n    range(65537)\n  example e2:\n    replicate(b, 65536)\n  example e3:\n    range(65536)\n";
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let bodies: Vec<&Expression> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Example(e) => Some(&e.body),
            _ => None,
        })
        .collect();
    let mut ev = Evaluator::new(&file, CodegenLimits::default());
    // e2's operand is built before measuring.  It is small (reading a binder
    // copies it, which is not what is measured here) but 65,536 copies of it
    // reserve 65,536 x (32 + 1,024) bytes, over the 64 MiB data budget.
    let env: HashMap<&str, EvalValue> = HashMap::from([("b", EvalValue::Bytes(vec![0x5a; 1024]))]);

    for (i, kind) in [
        (0, CodegenErrorKind::SequenceTooLong),
        (1, CodegenErrorKind::SequenceTooLong),
        (2, CodegenErrorKind::DataBudgetExceeded),
    ] {
        ev.reset_work();
        let (res, total, largest) = measured(|| ev.eval(bodies[i], &env));
        let err = res.expect_err("must be refused");
        assert_eq!((err.code, err.kind), (CgCode::W0703, kind), "example e{i}");
        assert!(
            largest < SMALL && total < 4 * SMALL,
            "example e{i} allocated {total} bytes (largest single request {largest}) before W0703"
        );
    }

    // Positive control: an admitted range really allocates its result, so
    // the probe is capable of seeing the allocation the refusals avoid.
    ev.reset_work();
    let (res, total, largest) = measured(|| ev.eval(bodies[3], &env));
    let EvalValue::List(items) = res.expect("range(65536)") else {
        panic!("not a list")
    };
    assert_eq!(items.len(), 65_536);
    let floor = 65_536 * std::mem::size_of::<EvalValue>();
    assert!(
        largest >= floor && total >= floor,
        "the probe missed a {floor}-byte result (largest {largest}, total {total})"
    );
}
