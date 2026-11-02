use http1_chunked_wire_budget::{
    Http1ChunkedWireBudget, Http1ChunkedWireBudgetError, Http1ChunkedWireLimit,
};

#[test]
fn starts_empty_with_the_exact_caller_limit() {
    let limit = Http1ChunkedWireLimit::new(12);
    let budget = Http1ChunkedWireBudget::new(limit);

    assert_eq!(limit.maximum_wire_bytes(), 12);
    assert_eq!(budget.limit(), limit);
    assert_eq!(budget.accounted_wire_bytes(), 0);
    assert_eq!(budget.remaining_wire_bytes(), 12);
}

#[test]
fn advances_in_segments_to_the_exact_boundary() {
    let budget = Http1ChunkedWireBudget::new(Http1ChunkedWireLimit::new(12))
        .advance(5)
        .unwrap()
        .advance(7)
        .unwrap();

    assert_eq!(budget.accounted_wire_bytes(), 12);
    assert_eq!(budget.remaining_wire_bytes(), 0);
}

#[test]
fn zero_limit_accepts_only_the_initial_state() {
    let result = Http1ChunkedWireBudget::new(Http1ChunkedWireLimit::new(0)).advance(1);
    assert!(matches!(
        result,
        Err(Http1ChunkedWireBudgetError::WireBudgetExceeded)
    ));
}

#[test]
fn rejects_zero_length_accounting() {
    let result = Http1ChunkedWireBudget::new(Http1ChunkedWireLimit::new(1)).advance(0);
    assert!(matches!(
        result,
        Err(Http1ChunkedWireBudgetError::ZeroLengthAdvance)
    ));
}

#[test]
fn rejects_cumulative_wire_bytes_beyond_the_limit() {
    let result = Http1ChunkedWireBudget::new(Http1ChunkedWireLimit::new(5))
        .advance(3)
        .unwrap()
        .advance(3);
    assert!(matches!(
        result,
        Err(Http1ChunkedWireBudgetError::WireBudgetExceeded)
    ));
}

#[test]
fn rejects_an_addition_that_overflows_platform_length() {
    let result = Http1ChunkedWireBudget::new(Http1ChunkedWireLimit::new(usize::MAX))
        .advance(usize::MAX)
        .unwrap()
        .advance(1);
    assert!(matches!(
        result,
        Err(Http1ChunkedWireBudgetError::WireLengthOverflow)
    ));
}

#[test]
fn stable_errors_carry_no_peer_bytes() {
    for error in [
        Http1ChunkedWireBudgetError::ZeroLengthAdvance,
        Http1ChunkedWireBudgetError::WireLengthOverflow,
        Http1ChunkedWireBudgetError::WireBudgetExceeded,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(!format!("{error:?}").contains("secret"));
    }
}
