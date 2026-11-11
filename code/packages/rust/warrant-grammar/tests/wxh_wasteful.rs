//! VER04j section 6.7, observer controls: a subject whose results are exactly
//! the honest subject's but which allocates ten times what it needs is told
//! apart from it, by the `heap` row alone (the output is within its bound and
//! no allocation comes near the cap).

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxh_subject {
        include!("support/wxh_subject_wasteful.rs");
    }
    include!("generated/wxh_evidence.rs");
}

include!("support/wxh_runner.rs");

#[test]
fn equal_results_with_ten_times_the_allocation_fail_the_heap_bound() {
    // A successful decompression carries the waste (a 64 KiB scratch buffer
    // against a heap bound of a few KiB): the example fails on the heap row.
    let o = outcome("example/three_bytes_within_the_limit");
    assert!(!o.passed());
    let m = message(o);
    assert!(m.contains("bound-heap-exceeded"), "{m}");
    assert!(m.contains("heap bound of"), "{m}");
    // The answer itself is right: the examples that only error allocate nothing
    // and pass.
    for id in ["example/one_byte_over_the_limit", "example/short_header_is_corrupt"] {
        let o = outcome(id);
        assert!(o.passed(), "{id}: {}", message(o));
    }
}

#[test]
fn the_waste_is_below_the_cap_so_it_is_the_row_not_the_cap_that_catches_it() {
    let m = message(outcome("example/three_bytes_within_the_limit"));
    assert!(!m.contains("allocation cap"), "{m}");
}
