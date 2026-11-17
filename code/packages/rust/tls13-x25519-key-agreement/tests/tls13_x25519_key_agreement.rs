use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy, PolicyError};
use tls13_x25519_key_agreement::{derive_x25519_shared_secret, KeyAgreementErrorKind};

const ALICE_PRIVATE: [u8; 32] = [
    0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66, 0x45,
    0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9, 0x2c, 0x2a,
];
const BOB_PUBLIC: [u8; 32] = [
    0xde, 0x9e, 0xdb, 0x7d, 0x7b, 0x7d, 0xc1, 0xb4, 0xd3, 0x5b, 0x61, 0xc2, 0xec, 0xe4, 0x35, 0x37,
    0x3f, 0x83, 0x43, 0xc8, 0x5b, 0x78, 0x67, 0x4d, 0xad, 0xfc, 0x7e, 0x14, 0x6f, 0x88, 0x2b, 0x4f,
];
const SHARED_SECRET: [u8; 32] = [
    0x4a, 0x5d, 0x9d, 0x5b, 0xa4, 0xce, 0x2d, 0xe1, 0x72, 0x8e, 0x3b, 0xf4, 0x80, 0x35, 0x0f, 0x25,
    0xe0, 0x7e, 0x21, 0xc9, 0x47, 0xd1, 0x9e, 0x33, 0x76, 0xf0, 0x9b, 0x3c, 0x1e, 0x16, 0x17, 0x42,
];

fn selected(group: NamedGroup) -> tls13_named_group_policy::SelectedNamedGroup {
    NamedGroupPolicy::new(&[group])
        .expect("single group policy")
        .select(group.wire_id())
        .expect("allowed group")
}

fn error_kind<T>(
    result: Result<T, tls13_x25519_key_agreement::KeyAgreementError>,
) -> KeyAgreementErrorKind {
    match result {
        Ok(_) => panic!("expected key-agreement rejection"),
        Err(error) => error.kind(),
    }
}

#[test]
fn selected_x25519_derives_rfc_secret_and_wipes_exact_output_on_drop() {
    let selected_group = selected(NamedGroup::X25519);
    let mut output = [0xa5; 32];
    {
        let secret =
            derive_x25519_shared_secret(selected_group, &ALICE_PRIVATE, &BOB_PUBLIC, &mut output)
                .expect("RFC 7748 vector must derive");
        assert_eq!(secret.selected_group(), selected_group);
        assert_eq!(secret.as_bytes(), &SHARED_SECRET);
        assert_eq!(format!("{secret:?}"), "X25519SharedSecret");
    }
    assert_eq!(output, [0; 32]);
}

#[test]
fn unsupported_selected_group_leaves_output_unchanged() {
    let mut output = [0xa5; 32];
    let peer = [0x42; 65];
    let kind = error_kind(derive_x25519_shared_secret(
        selected(NamedGroup::Secp256r1),
        &ALICE_PRIVATE,
        &peer,
        &mut output,
    ));
    assert_eq!(kind, KeyAgreementErrorKind::UnsupportedSelectedGroup);
    assert_eq!(output, [0xa5; 32]);
}

#[test]
fn invalid_x25519_peer_length_leaves_output_unchanged() {
    let mut output = [0xa5; 32];
    let kind = error_kind(derive_x25519_shared_secret(
        selected(NamedGroup::X25519),
        &ALICE_PRIVATE,
        &[0x42; 31],
        &mut output,
    ));
    assert_eq!(
        kind,
        KeyAgreementErrorKind::NamedGroupPolicy(PolicyError::InvalidKeyExchangeLength)
    );
    assert_eq!(output, [0xa5; 32]);
}

#[test]
fn low_order_peer_key_is_rejected_without_releasing_output() {
    let mut output = [0xa5; 32];
    let kind = error_kind(derive_x25519_shared_secret(
        selected(NamedGroup::X25519),
        &ALICE_PRIVATE,
        &[0; 32],
        &mut output,
    ));
    assert_eq!(kind, KeyAgreementErrorKind::PeerPublicKeyRejected);
    assert_eq!(output, [0xa5; 32]);
}

#[test]
fn public_error_text_contains_only_stable_categories() {
    let mut output = [0xa5; 32];
    let result = derive_x25519_shared_secret(
        selected(NamedGroup::X25519),
        &ALICE_PRIVATE,
        &[0; 32],
        &mut output,
    );
    let error = match result {
        Ok(_) => panic!("low-order peer key must fail"),
        Err(error) => error,
    };
    assert_eq!(
        format!("{error:?}"),
        "KeyAgreementError { kind: PeerPublicKeyRejected }"
    );
    assert_eq!(
        error.to_string(),
        "TLS 1.3 key-agreement error PeerPublicKeyRejected"
    );
}
