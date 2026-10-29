use tls13_named_group_policy::{
    KeyAgreementAlgorithm, NamedGroup, NamedGroupPolicy, PolicyError, SUPPORTED_NAMED_GROUP_COUNT,
};

#[test]
fn descriptors_bind_exact_wire_algorithms_and_lengths() {
    assert_eq!(SUPPORTED_NAMED_GROUP_COUNT, 2);
    assert_eq!(NamedGroup::from_wire_id(0x0017), Ok(NamedGroup::Secp256r1));
    assert_eq!(NamedGroup::from_wire_id(0x001d), Ok(NamedGroup::X25519));
    assert_eq!(NamedGroup::Secp256r1.wire_id(), 0x0017);
    assert_eq!(NamedGroup::X25519.wire_id(), 0x001d);

    let cases = [
        (
            NamedGroup::Secp256r1,
            KeyAgreementAlgorithm::EcdhSecp256r1,
            65,
            32,
        ),
        (NamedGroup::X25519, KeyAgreementAlgorithm::X25519, 32, 32),
    ];
    for (group, algorithm, key_exchange_length, shared_secret_length) in cases {
        let parameters = group.parameters();
        assert_eq!(parameters.group(), group);
        assert_eq!(parameters.key_agreement(), algorithm);
        assert_eq!(parameters.key_exchange_length(), key_exchange_length);
        assert_eq!(parameters.shared_secret_length(), shared_secret_length);
    }
}

#[test]
fn unsupported_wire_identifiers_fail_closed() {
    for wire_id in [0x0000, 0x0018, 0x001c, 0x001e, 0x0a0a, u16::MAX] {
        assert_eq!(
            NamedGroup::from_wire_id(wire_id),
            Err(PolicyError::UnknownNamedGroup)
        );
    }
}

#[test]
fn policies_are_nonempty_and_duplicate_free() {
    assert_eq!(NamedGroupPolicy::new(&[]), Err(PolicyError::EmptyPolicy));
    assert_eq!(
        NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::X25519]),
        Err(PolicyError::DuplicateNamedGroup)
    );
}

#[test]
fn selection_requires_exact_policy_authorization() {
    let policy = NamedGroupPolicy::new(&[NamedGroup::X25519]).unwrap();
    assert!(policy.allows(NamedGroup::X25519));
    assert!(!policy.allows(NamedGroup::Secp256r1));
    let selected = policy.select(0x001d).unwrap();
    assert_eq!(selected.parameters().group(), NamedGroup::X25519);
    assert_eq!(
        policy.select(0x0017),
        Err(PolicyError::DisallowedNamedGroup)
    );
    assert_eq!(policy.select(0x0018), Err(PolicyError::UnknownNamedGroup));
}

#[test]
fn selected_group_validates_exact_public_and_shared_lengths() {
    let selected = NamedGroupPolicy::new(&[NamedGroup::X25519])
        .unwrap()
        .select(0x001d)
        .unwrap();

    assert_eq!(selected.validate_key_exchange_length(32), Ok(()));
    assert_eq!(
        selected.validate_key_exchange_length(31),
        Err(PolicyError::InvalidKeyExchangeLength)
    );
    assert_eq!(
        selected.validate_key_exchange_length(33),
        Err(PolicyError::InvalidKeyExchangeLength)
    );
    assert_eq!(selected.validate_shared_secret_length(32), Ok(()));
    assert_eq!(
        selected.validate_shared_secret_length(0),
        Err(PolicyError::InvalidSharedSecretLength)
    );
}

#[test]
fn errors_are_stable_and_redacted() {
    let cases = [
        (PolicyError::UnknownNamedGroup, "unknown TLS named group"),
        (PolicyError::EmptyPolicy, "empty TLS named-group policy"),
        (
            PolicyError::DuplicateNamedGroup,
            "duplicate TLS named group",
        ),
        (
            PolicyError::DisallowedNamedGroup,
            "TLS named group is not allowed",
        ),
        (
            PolicyError::InvalidKeyExchangeLength,
            "invalid TLS named-group key-exchange length",
        ),
        (
            PolicyError::InvalidSharedSecretLength,
            "invalid TLS named-group shared-secret length",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
        assert!(!format!("{error:?}").contains("0x"));
    }
}
