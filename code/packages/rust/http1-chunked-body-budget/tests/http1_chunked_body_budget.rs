use http1_chunked_body_budget::{
    Http1ChunkedBodyBudget, Http1ChunkedBodyBudgetError, Http1ChunkedBodyLimits,
};

#[test]
fn starts_empty_with_exact_caller_limits() {
    let limits = Http1ChunkedBodyLimits::new(10, 2);
    let budget = Http1ChunkedBodyBudget::new(limits);

    assert_eq!(limits.maximum_decoded_body_bytes(), 10);
    assert_eq!(limits.maximum_non_terminal_chunks(), 2);
    assert_eq!(budget.limits(), limits);
    assert_eq!(budget.decoded_body_bytes(), 0);
    assert_eq!(budget.non_terminal_chunk_count(), 0);
    assert_eq!(budget.remaining_decoded_body_bytes(), 10);
    assert_eq!(budget.remaining_non_terminal_chunks(), 2);
}

#[test]
fn advances_to_both_exact_boundaries() {
    let limits = Http1ChunkedBodyLimits::new(10, 2);
    let budget = Http1ChunkedBodyBudget::new(limits)
        .advance(4)
        .unwrap()
        .advance(6)
        .unwrap();

    assert_eq!(budget.decoded_body_bytes(), 10);
    assert_eq!(budget.non_terminal_chunk_count(), 2);
    assert_eq!(budget.remaining_decoded_body_bytes(), 0);
    assert_eq!(budget.remaining_non_terminal_chunks(), 0);
}

#[test]
fn zero_size_is_reserved_for_terminal_framing() {
    let result = Http1ChunkedBodyBudget::new(Http1ChunkedBodyLimits::new(10, 2)).advance(0);
    assert!(matches!(
        result,
        Err(Http1ChunkedBodyBudgetError::ZeroSizedDataChunk)
    ));
}

#[test]
fn chunk_count_limit_is_checked_before_length_arithmetic() {
    let result = Http1ChunkedBodyBudget::new(Http1ChunkedBodyLimits::new(0, 0)).advance(usize::MAX);
    assert!(matches!(
        result,
        Err(Http1ChunkedBodyBudgetError::TooManyChunks)
    ));
}

#[test]
fn rejects_a_chunk_beyond_the_decoded_body_limit() {
    let result = Http1ChunkedBodyBudget::new(Http1ChunkedBodyLimits::new(5, 1)).advance(6);
    assert!(matches!(
        result,
        Err(Http1ChunkedBodyBudgetError::DecodedBodyTooLarge)
    ));
}

#[test]
fn rejects_an_addition_that_overflows_platform_length() {
    let result = Http1ChunkedBodyBudget::new(Http1ChunkedBodyLimits::new(usize::MAX, 2))
        .advance(usize::MAX)
        .unwrap()
        .advance(1);
    assert!(matches!(
        result,
        Err(Http1ChunkedBodyBudgetError::DecodedLengthOverflow)
    ));
}

#[test]
fn rejects_a_non_terminal_chunk_beyond_the_count_limit() {
    let result = Http1ChunkedBodyBudget::new(Http1ChunkedBodyLimits::new(2, 1))
        .advance(1)
        .unwrap()
        .advance(1);
    assert!(matches!(
        result,
        Err(Http1ChunkedBodyBudgetError::TooManyChunks)
    ));
}

#[test]
fn stable_errors_carry_no_peer_bytes() {
    for error in [
        Http1ChunkedBodyBudgetError::ZeroSizedDataChunk,
        Http1ChunkedBodyBudgetError::TooManyChunks,
        Http1ChunkedBodyBudgetError::DecodedLengthOverflow,
        Http1ChunkedBodyBudgetError::DecodedBodyTooLarge,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(!format!("{error:?}").contains("secret"));
    }
}
