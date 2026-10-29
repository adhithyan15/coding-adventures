use tls13_supported_groups::{
    decode_supported_groups, NamedGroupListError, NamedGroupListLimits, MAX_NAMED_GROUP_COUNT,
};

fn limits(max_group_count: usize) -> NamedGroupListLimits {
    NamedGroupListLimits::new(max_group_count).expect("valid test limit")
}

#[test]
fn decodes_known_unknown_and_private_use_identifiers_in_order() {
    let encoded = [0, 8, 0x00, 0x1d, 0x7a, 0x7a, 0xfe, 0x00, 0x01, 0x00];
    let list = decode_supported_groups(&encoded, limits(8)).expect("valid list");

    assert_eq!(list.extension_data(), &encoded);
    assert!(core::ptr::eq(
        list.extension_data().as_ptr(),
        encoded.as_ptr()
    ));
    assert_eq!(list.len(), 4);
    assert!(!list.is_empty());
    assert_eq!(list.get(0), Some(0x001d));
    assert_eq!(list.get(1), Some(0x7a7a));
    assert_eq!(list.get(2), Some(0xfe00));
    assert_eq!(list.get(3), Some(0x0100));
    assert_eq!(list.get(4), None);
    assert!(list.contains(0x001d));
    assert!(!list.contains(0x0017));
    assert_eq!(
        list.iter().collect::<Vec<_>>(),
        vec![0x001d, 0x7a7a, 0xfe00, 0x0100]
    );
}

#[test]
fn iterator_is_exact_size_fused_and_redacted() {
    let encoded = [0, 4, 0xaa, 0xbb, 0xcc, 0xdd];
    let list = decode_supported_groups(&encoded, limits(2)).expect("valid list");
    let mut groups = list.iter();

    assert_eq!(groups.len(), 2);
    assert_eq!(groups.size_hint(), (2, Some(2)));
    let fresh_debug = format!("{groups:?}");
    assert!(fresh_debug.contains("remaining_group_count: 2"));
    assert!(fresh_debug.contains("[OPAQUE]"));
    for disclosed in ["170", "187", "204", "221", "0xaa", "0xbb"] {
        assert!(!fresh_debug.contains(disclosed));
    }

    assert_eq!(groups.next(), Some(0xaabb));
    assert_eq!(groups.len(), 1);
    let partial_debug = format!("{groups:?}");
    assert!(partial_debug.contains("remaining_group_count: 1"));
    for disclosed in ["204", "221", "0xcc", "0xdd"] {
        assert!(!partial_debug.contains(disclosed));
    }
    assert_eq!(groups.next(), Some(0xccdd));
    assert_eq!(groups.next(), None);
    assert_eq!(groups.next(), None);
}

#[test]
fn rejects_malformed_vectors_and_applies_the_count_limit() {
    assert_eq!(
        decode_supported_groups(&[], limits(4)).unwrap_err(),
        NamedGroupListError::MissingListLength
    );
    assert_eq!(
        decode_supported_groups(&[0], limits(4)).unwrap_err(),
        NamedGroupListError::MissingListLength
    );
    assert_eq!(
        decode_supported_groups(&[0, 0], limits(4)).unwrap_err(),
        NamedGroupListError::EmptyList
    );
    assert_eq!(
        decode_supported_groups(&[0, 3, 0x00, 0x1d, 0x00], limits(4)).unwrap_err(),
        NamedGroupListError::OddListLength
    );
    assert_eq!(
        decode_supported_groups(&[0, 4, 0x00, 0x1d], limits(4)).unwrap_err(),
        NamedGroupListError::ListLengthMismatch
    );
    assert_eq!(
        decode_supported_groups(&[0, 2, 0x00, 0x1d, 0], limits(4)).unwrap_err(),
        NamedGroupListError::ListLengthMismatch
    );
    assert_eq!(
        decode_supported_groups(&[0, 4, 0x00, 0x1d, 0x00, 0x17], limits(1)).unwrap_err(),
        NamedGroupListError::TooManyGroups
    );
}

#[test]
fn validates_limits_and_retains_repeated_preferences_for_policy() {
    assert_eq!(
        NamedGroupListLimits::new(0),
        Err(NamedGroupListError::InvalidLimit)
    );
    assert_eq!(
        NamedGroupListLimits::new(MAX_NAMED_GROUP_COUNT + 1),
        Err(NamedGroupListError::InvalidLimit)
    );
    assert_eq!(
        NamedGroupListLimits::new(MAX_NAMED_GROUP_COUNT)
            .expect("protocol maximum")
            .max_group_count(),
        MAX_NAMED_GROUP_COUNT
    );

    let encoded = [0, 4, 0x00, 0x1d, 0x00, 0x1d];
    let list = decode_supported_groups(&encoded, limits(2)).expect("framing is valid");
    assert_eq!(list.iter().collect::<Vec<_>>(), vec![0x001d, 0x001d]);
}

#[test]
fn list_diagnostics_do_not_disclose_identifiers_or_input_bytes() {
    let encoded = [0, 2, 0xaa, 0xbb];
    let list = decode_supported_groups(&encoded, limits(1)).expect("valid list");
    let debug = format!("{list:?}");

    assert!(debug.contains("group_count: 1"));
    assert!(debug.contains("[OPAQUE]"));
    assert!(!debug.contains("43707"));
    assert!(!debug.contains("170"));
    assert!(!debug.contains("187"));
    assert!(!debug.contains("0xaa"));
    assert!(!debug.contains("0xbb"));
}

#[test]
fn errors_have_stable_payload_free_messages() {
    let cases = [
        (
            NamedGroupListError::InvalidLimit,
            "invalid TLS named-group list limit",
        ),
        (
            NamedGroupListError::MissingListLength,
            "missing TLS named-group list length",
        ),
        (NamedGroupListError::EmptyList, "empty TLS named-group list"),
        (
            NamedGroupListError::OddListLength,
            "odd TLS named-group list length",
        ),
        (
            NamedGroupListError::ListLengthMismatch,
            "TLS named-group list length mismatch",
        ),
        (
            NamedGroupListError::TooManyGroups,
            "TLS named-group list exceeds the caller limit",
        ),
    ];

    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}
