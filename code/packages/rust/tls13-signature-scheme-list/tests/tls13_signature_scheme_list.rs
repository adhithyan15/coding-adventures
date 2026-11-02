use tls13_signature_scheme_list::{
    decode_signature_scheme_list, SignatureSchemeListError, SignatureSchemeListLimits,
    MAX_SIGNATURE_SCHEME_COUNT,
};

fn limits(max_scheme_count: usize) -> SignatureSchemeListLimits {
    SignatureSchemeListLimits::new(max_scheme_count).expect("valid test limit")
}

#[test]
fn decodes_known_unknown_and_private_use_identifiers_in_order() {
    let encoded = [0, 8, 0x08, 0x04, 0x7a, 0x7a, 0xfe, 0x00, 0x08, 0x07];
    let list = decode_signature_scheme_list(&encoded, limits(8)).expect("valid list");

    assert_eq!(list.extension_data(), &encoded);
    assert!(core::ptr::eq(
        list.extension_data().as_ptr(),
        encoded.as_ptr()
    ));
    assert_eq!(list.len(), 4);
    assert!(!list.is_empty());
    assert_eq!(list.get(0), Some(0x0804));
    assert_eq!(list.get(1), Some(0x7a7a));
    assert_eq!(list.get(2), Some(0xfe00));
    assert_eq!(list.get(3), Some(0x0807));
    assert_eq!(list.get(4), None);
    assert!(list.contains(0x7a7a));
    assert!(!list.contains(0x0805));
    assert_eq!(
        list.iter().collect::<Vec<_>>(),
        vec![0x0804, 0x7a7a, 0xfe00, 0x0807]
    );
}

#[test]
fn iterator_is_exact_size_and_fused() {
    let encoded = [0, 4, 0x08, 0x04, 0x08, 0x05];
    let list = decode_signature_scheme_list(&encoded, limits(2)).expect("valid list");
    let mut schemes = list.iter();

    assert_eq!(schemes.len(), 2);
    assert_eq!(schemes.size_hint(), (2, Some(2)));
    assert_eq!(schemes.next(), Some(0x0804));
    assert_eq!(schemes.len(), 1);
    assert_eq!(schemes.next(), Some(0x0805));
    assert_eq!(schemes.next(), None);
    assert_eq!(schemes.next(), None);
}

#[test]
fn rejects_malformed_vectors_and_applies_the_count_limit() {
    assert_eq!(
        decode_signature_scheme_list(&[], limits(4)).unwrap_err(),
        SignatureSchemeListError::MissingListLength
    );
    assert_eq!(
        decode_signature_scheme_list(&[0], limits(4)).unwrap_err(),
        SignatureSchemeListError::MissingListLength
    );
    assert_eq!(
        decode_signature_scheme_list(&[0, 0], limits(4)).unwrap_err(),
        SignatureSchemeListError::EmptyList
    );
    assert_eq!(
        decode_signature_scheme_list(&[0, 3, 0x08, 0x04, 0x08], limits(4)).unwrap_err(),
        SignatureSchemeListError::OddListLength
    );
    assert_eq!(
        decode_signature_scheme_list(&[0, 4, 0x08, 0x04], limits(4)).unwrap_err(),
        SignatureSchemeListError::ListLengthMismatch
    );
    assert_eq!(
        decode_signature_scheme_list(&[0, 2, 0x08, 0x04, 0], limits(4)).unwrap_err(),
        SignatureSchemeListError::ListLengthMismatch
    );
    assert_eq!(
        decode_signature_scheme_list(&[0, 4, 0x08, 0x04, 0x08, 0x05], limits(1)).unwrap_err(),
        SignatureSchemeListError::TooManySchemes
    );
}

#[test]
fn validates_limits_and_retains_repeated_preferences_for_policy() {
    assert_eq!(
        SignatureSchemeListLimits::new(0),
        Err(SignatureSchemeListError::InvalidLimit)
    );
    assert_eq!(
        SignatureSchemeListLimits::new(MAX_SIGNATURE_SCHEME_COUNT + 1),
        Err(SignatureSchemeListError::InvalidLimit)
    );
    assert_eq!(
        SignatureSchemeListLimits::new(MAX_SIGNATURE_SCHEME_COUNT)
            .expect("protocol maximum")
            .max_scheme_count(),
        MAX_SIGNATURE_SCHEME_COUNT
    );

    let encoded = [0, 4, 0x08, 0x04, 0x08, 0x04];
    let list = decode_signature_scheme_list(&encoded, limits(2)).expect("framing is valid");
    assert_eq!(list.iter().collect::<Vec<_>>(), vec![0x0804, 0x0804]);
}

#[test]
fn diagnostics_do_not_disclose_identifiers_or_input_bytes() {
    let encoded = [0, 4, 0xaa, 0xbb, 0xcc, 0xdd];
    let list = decode_signature_scheme_list(&encoded, limits(2)).expect("valid list");
    let debug = format!("{list:?}");

    assert!(debug.contains("scheme_count: 2"));
    assert!(debug.contains("[OPAQUE]"));
    assert!(!debug.contains("43690"));
    assert!(!debug.contains("48042"));
    assert!(!debug.contains("52445"));
    assert!(!debug.contains("0xaa"));
    assert!(!debug.contains("0xbb"));

    let mut iter = list.iter();
    let fresh_debug = format!("{iter:?}");
    assert!(fresh_debug.contains("remaining_scheme_count: 2"));
    assert!(fresh_debug.contains("[OPAQUE]"));
    assert!(!fresh_debug.contains("170"));
    assert!(!fresh_debug.contains("187"));
    assert!(!fresh_debug.contains("204"));
    assert!(!fresh_debug.contains("221"));
    assert!(!fresh_debug.contains("0xaa"));
    assert_eq!(iter.next(), Some(0xaabb));

    let partial_debug = format!("{iter:?}");
    assert!(partial_debug.contains("remaining_scheme_count: 1"));
    assert!(partial_debug.contains("[OPAQUE]"));
    assert!(!partial_debug.contains("204"));
    assert!(!partial_debug.contains("221"));
    assert!(!partial_debug.contains("0xcc"));
    assert!(!partial_debug.contains("0xdd"));
}

#[test]
fn errors_have_stable_payload_free_messages() {
    let cases = [
        (
            SignatureSchemeListError::InvalidLimit,
            "invalid TLS signature-scheme list limit",
        ),
        (
            SignatureSchemeListError::MissingListLength,
            "missing TLS signature-scheme list length",
        ),
        (
            SignatureSchemeListError::EmptyList,
            "empty TLS signature-scheme list",
        ),
        (
            SignatureSchemeListError::OddListLength,
            "odd TLS signature-scheme list length",
        ),
        (
            SignatureSchemeListError::ListLengthMismatch,
            "TLS signature-scheme list length mismatch",
        ),
        (
            SignatureSchemeListError::TooManySchemes,
            "TLS signature-scheme list exceeds the caller limit",
        ),
    ];

    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}
