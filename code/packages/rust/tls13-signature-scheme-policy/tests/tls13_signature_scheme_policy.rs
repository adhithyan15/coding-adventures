use tls13_signature_scheme_policy::{
    PolicyError, PublicKeyAlgorithm, SignatureHash, SignatureScheme, SignatureSchemePolicy,
    SUPPORTED_SIGNATURE_SCHEME_COUNT,
};

const SCHEMES: [SignatureScheme; SUPPORTED_SIGNATURE_SCHEME_COUNT] = [
    SignatureScheme::RsaPssRsaeSha256,
    SignatureScheme::RsaPssRsaeSha384,
    SignatureScheme::RsaPssRsaeSha512,
];

#[test]
fn wire_identifiers_round_trip_exactly() {
    for (scheme, wire_id) in SCHEMES.into_iter().zip([0x0804, 0x0805, 0x0806]) {
        assert_eq!(scheme.wire_id(), wire_id);
        assert_eq!(SignatureScheme::from_wire_id(wire_id).unwrap(), scheme);
    }
}

#[test]
fn parameters_bind_hash_mgf_salt_and_rsae_key_form() {
    for (scheme, hash, length) in [
        (SignatureScheme::RsaPssRsaeSha256, SignatureHash::Sha256, 32),
        (SignatureScheme::RsaPssRsaeSha384, SignatureHash::Sha384, 48),
        (SignatureScheme::RsaPssRsaeSha512, SignatureHash::Sha512, 64),
    ] {
        let parameters = scheme.parameters();
        assert_eq!(parameters.scheme(), scheme);
        assert_eq!(parameters.message_hash(), hash);
        assert_eq!(parameters.mgf1_hash(), hash);
        assert_eq!(parameters.salt_length(), length);
        assert_eq!(
            parameters.public_key_algorithm(),
            PublicKeyAlgorithm::RsaEncryption
        );
        assert_eq!(hash.output_length(), length);
    }
}

#[test]
fn policy_preserves_preference_order_and_authorizes_members() {
    let allowed = [
        SignatureScheme::RsaPssRsaeSha512,
        SignatureScheme::RsaPssRsaeSha256,
    ];
    let policy = SignatureSchemePolicy::new(&allowed).unwrap();

    assert_eq!(policy.ordered_schemes(), &allowed);
    let selected = policy.select(0x0804).unwrap();
    assert_eq!(selected.scheme(), SignatureScheme::RsaPssRsaeSha256);
    assert_eq!(selected.parameters(), selected.scheme().parameters());
}

#[test]
fn policy_rejects_empty_and_duplicate_allowlists() {
    assert_eq!(
        SignatureSchemePolicy::new(&[]).unwrap_err(),
        PolicyError::EmptyPolicy
    );
    let duplicate = [
        SignatureScheme::RsaPssRsaeSha256,
        SignatureScheme::RsaPssRsaeSha384,
        SignatureScheme::RsaPssRsaeSha256,
    ];
    assert_eq!(
        SignatureSchemePolicy::new(&duplicate).unwrap_err(),
        PolicyError::DuplicateSignatureScheme
    );
}

#[test]
fn policy_rejects_known_but_disallowed_schemes() {
    let allowed = [SignatureScheme::RsaPssRsaeSha256];
    let error = SignatureSchemePolicy::new(&allowed)
        .unwrap()
        .select(0x0805)
        .unwrap_err();
    assert_eq!(error, PolicyError::DisallowedSignatureScheme);
}

#[test]
fn unsupported_and_barred_scheme_families_fail_closed() {
    for wire_id in [
        0x0000, // anonymous / unassigned here
        0x0403, // ECDSA P-256 SHA-256
        0x0807, // Ed25519
        0x0809, // RSA-PSS with RSASSA-PSS public-key OID
        0x0201, // legacy RSA PKCS #1 SHA-1
        u16::MAX,
    ] {
        assert_eq!(
            SignatureScheme::from_wire_id(wire_id).unwrap_err(),
            PolicyError::UnknownSignatureScheme
        );
    }
}

#[test]
fn errors_are_stable_and_retain_no_wire_identifier() {
    let error = SignatureScheme::from_wire_id(0x7d7d).unwrap_err();
    assert_eq!(error.to_string(), "unknown TLS 1.3 signature scheme");
    assert_eq!(format!("{error:?}"), "UnknownSignatureScheme");
    assert!(!error.to_string().contains("32125"));
    assert!(!format!("{error:?}").contains("32125"));

    for (kind, message) in [
        (
            PolicyError::EmptyPolicy,
            "empty TLS 1.3 signature-scheme policy",
        ),
        (
            PolicyError::DuplicateSignatureScheme,
            "duplicate TLS 1.3 signature scheme",
        ),
        (
            PolicyError::DisallowedSignatureScheme,
            "TLS 1.3 signature scheme is not allowed",
        ),
    ] {
        assert_eq!(kind.to_string(), message);
    }
}

#[test]
fn public_registry_count_matches_the_closed_enum() {
    assert_eq!(SUPPORTED_SIGNATURE_SCHEME_COUNT, SCHEMES.len());
}
