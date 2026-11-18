use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
use rsa_pss_sha384_signature::RsaPssSha384SignatureLimits;
use rsa_pss_sha512_signature::RsaPssSha512SignatureLimits;
use tls13_certificate_verify_input::{
    build_certificate_verify_input, CertificateVerifyRole, TranscriptHashAlgorithm,
    MAXIMUM_INPUT_LENGTH,
};
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_rsa_pss_certificate_verify::{
    hash_tls13_certificate_verify_input, verify_tls13_rsa_pss_certificate_verify,
    Tls13RsaPssCertificateVerifyErrorKind,
};
use tls13_rsa_pss_signature_routing::{
    Tls13RsaPssSignatureLimits, Tls13RsaPssSignatureRoutingErrorKind,
};
use tls13_signature_scheme_policy::{
    AuthorizedSignatureScheme, SignatureScheme, SignatureSchemePolicy,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const TRANSCRIPT_HASH: [u8; 32] = [0x42; 32];

// Generated with OpenSSL 3.6.3 from one 2048-bit RSA key over the exact server
// CertificateVerify input carrying TRANSCRIPT_HASH, using fixed PSS salt
// lengths 32, 48, and 64. Public SPKI SHA-256:
// 788c1757121c5ecb316f28071c365567622350700d459ee3ff9747167c19186d.
const PUBLIC_KEY_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a0282010100",
    "bf2a4ea8f623cf56597004c69545dddfbda858ad98d92f16384fa1c31ecbd7aa",
    "6cb660c6b4d7212cbb09397761a4290a88ba8c456be76c475f9f584d06da26e9",
    "7b4345d27407fae415f476eb90c86314979997e138f024754834c1f04bfe07a3",
    "cb02984c52a2f329b58876fa1c145f2caa269d369810417071bc7569d9ef93534",
    "10d0f4fc6624d5b575cf3f29dcb0e60f79c1bd23d596a80cecfd9eafde22d671",
    "e16d7387468763b41c517ff8eda6e73879203519798ce31ee918ceef79aca31d7",
    "aa2fe9d4d7bdce1b26659ce04dc445c6e5c436c55199ca3e5d5e99ad88ac17ac",
    "e661c034370414a97a79d0ef4f01a68a4618914759475f8fbca835aedb9acd",
    "0203010001",
);
const SHA256_SIGNATURE_HEX: &str = concat!(
    "40992ed3e07a1ffe7b88a14f520e141ec9c77d2948103071c9ee469e0a44dbe1",
    "a26addd0dd17f5891e8fca5077aa4eb77becf8b02bbeb9a913bc8f801b6d982e",
    "577265b076aa2c3f2437afb9d1d86b8f626189b0d55ad1f92a4b8f7cc599487a",
    "0823f166900bd46db253ed2c5212e35914bcab6ac22f70eec17213e33d85a89c",
    "a5e5500e3b7f53d00f302a4de54c5bbeb69dda3bf6db1908785ea5759095ced1",
    "13c6147a181bbe795746f6bf7b972a0cf88d245ffcda495d3537b20c2f589bfd",
    "81619fd404135f0072d253df455738a15f46a12bb3070eb64e2f1c4121c8dffb",
    "6770db602d38a7239c1e8bd100173966089572a4d09afb3a7b0cb5f7d8fcbb17",
);
const SHA384_SIGNATURE_HEX: &str = concat!(
    "bebc0e109b0ffdc6e84a8f22f6e20cca967417d7f0fad55edeaa476de1f11c35",
    "a16d77bc461919f7fd3ea2655d4b6f14d9af9996fb3303a3b5e63fb9c7ac5d18",
    "dc29ef15cb66b2078be6798777fa77e1927cd489badf5682279842602d5267c4b",
    "8c352aef657eb9cd4c57b649f7250751cab7a1371a8a6a620946097959425a4e",
    "fadd9267ae41cdbf667ff2f518436d974396876ceaee73e1f259e38e15cdde565",
    "c56b2ce78b69cbe9f56f50194df0a05cf821e1f011da4c56e55793851bb75aa",
    "fd63e31903c2a594cdd7b70c214ad0a20cf143f12484d53bc23902d43f5da086",
    "c0336914bb1d13fca8f46e15b00e4dc655de60d436631cdd7dbec3243fbca90",
);
const SHA512_SIGNATURE_HEX: &str = concat!(
    "883446c7f846ba2181786423cc6bb9d6a89903d97c5bfb3ed1bbf1a3cc18e528",
    "7dd980049e8638ae618f5544ff8da3de7d48a56648232b267319e3ffc188be16",
    "c7d66f20df82bad0e4158a7a3384031975c41b7a710b8636286cbc49b8ca456e",
    "b781f1beaa79fd6b6545101602b025830251a23f7bc2fd6dfdada751e6ef18ba",
    "25a040e8253d30c7a2cc726b3a44d2a1e18e5ec07d61cf3dcde781dd6f08ee8",
    "85026b0adb4c8dd2553cb5ad7751e18c36c16872b39cd095600cf263c5221613",
    "a061f780922390e7229fa3212912a650b4b6c897614f9e06dcc37c249dd9d98d",
    "d227e7fa1546f91cf8cfb67ff806029276d01d9d333d94ba7d855e7dde5d9d354",
);

const SHA256_INPUT_DIGEST_HEX: &str =
    "7299026ceac069ad3d182d6ea2bb4090b7258dcbe9b86ca655f39ef1686e0ee6";
const SHA384_INPUT_DIGEST_HEX: &str = concat!(
    "e89952f9c31b5d94c0bf0aa2dffd98c94a41087517fc92ccd4c829beaf3808a2",
    "1ebe64d1a33011944bd6ed78eadf13a2",
);
const SHA512_INPUT_DIGEST_HEX: &str = concat!(
    "b476ef5d3aff95044655fccf81858278e18d79cd8b291e6ae37ac044a15adca",
    "ded3a83d132afbdc0156f7cb5f316203d3d61694704847e7923d96d15bd04a3e9",
);

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn validated_key(encoded: &[u8]) -> ValidatedRsaPublicKey<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let subject_public_key_info = decode_x509_subject_public_key_info(&mut decoder, root).unwrap();
    let policy = RsaPublicKeyPolicy::new(2048, 2048, &[65_537]).unwrap();
    validate_rsa_public_key(subject_public_key_info, policy, Asn1Limits::default()).unwrap()
}

fn authorized(scheme: SignatureScheme) -> AuthorizedSignatureScheme {
    SignatureSchemePolicy::new(&[scheme])
        .unwrap()
        .select(scheme.wire_id())
        .unwrap()
}

#[test]
fn hashes_exact_input_under_all_authorized_profiles() {
    let mut input_storage = [0xa5; MAXIMUM_INPUT_LENGTH];
    let input = build_certificate_verify_input(
        CertificateVerifyRole::Server,
        TranscriptHashAlgorithm::Sha256,
        &TRANSCRIPT_HASH,
        &mut input_storage,
    )
    .unwrap();

    let cases = [
        (SignatureScheme::RsaPssRsaeSha256, SHA256_INPUT_DIGEST_HEX),
        (SignatureScheme::RsaPssRsaeSha384, SHA384_INPUT_DIGEST_HEX),
        (SignatureScheme::RsaPssRsaeSha512, SHA512_INPUT_DIGEST_HEX),
    ];

    for (scheme, expected_hex) in cases {
        let authorized = authorized(scheme);
        let hashed = hash_tls13_certificate_verify_input(authorized, &input);
        assert_eq!(hashed.authorized_scheme(), authorized);
        assert_eq!(hashed.input(), &input);
        assert_eq!(hashed.digest().as_bytes(), decode_hex(expected_hex));
        assert_eq!(format!("{hashed:?}"), "HashedTls13CertificateVerifyInput");
    }
}

#[test]
fn verifies_all_three_independent_signatures_and_binds_evidence() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let mut input_storage = [0xa5; MAXIMUM_INPUT_LENGTH];
    let input = build_certificate_verify_input(
        CertificateVerifyRole::Server,
        TranscriptHashAlgorithm::Sha256,
        &TRANSCRIPT_HASH,
        &mut input_storage,
    )
    .unwrap();

    let signature256 = decode_hex(SHA256_SIGNATURE_HEX);
    let message256 = CertificateVerifyMessage::new(0x0804, &signature256).unwrap();
    let hashed256 =
        hash_tls13_certificate_verify_input(authorized(SignatureScheme::RsaPssRsaeSha256), &input);
    let mut scratch256 = [0xa5; 223];
    let verified256 = verify_tls13_rsa_pss_certificate_verify(
        &hashed256,
        &message256,
        key,
        &mut scratch256,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap();
    assert!(std::ptr::eq(verified256.hashed_input(), &hashed256));
    assert_eq!(verified256.signature().message(), &message256);
    assert_eq!(
        verified256.signature().public_key().modulus(),
        key.modulus()
    );
    assert_eq!(
        format!("{verified256:?}"),
        "VerifiedTls13RsaPssCertificateVerify"
    );

    let signature384 = decode_hex(SHA384_SIGNATURE_HEX);
    let message384 = CertificateVerifyMessage::new(0x0805, &signature384).unwrap();
    let hashed384 =
        hash_tls13_certificate_verify_input(authorized(SignatureScheme::RsaPssRsaeSha384), &input);
    let mut scratch384 = [0xa5; 207];
    verify_tls13_rsa_pss_certificate_verify(
        &hashed384,
        &message384,
        key,
        &mut scratch384,
        Tls13RsaPssSignatureLimits::Sha384(RsaPssSha384SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap();

    let signature512 = decode_hex(SHA512_SIGNATURE_HEX);
    let message512 = CertificateVerifyMessage::new(0x0806, &signature512).unwrap();
    let hashed512 =
        hash_tls13_certificate_verify_input(authorized(SignatureScheme::RsaPssRsaeSha512), &input);
    let mut scratch512 = [0xa5; 191];
    verify_tls13_rsa_pss_certificate_verify(
        &hashed512,
        &message512,
        key,
        &mut scratch512,
        Tls13RsaPssSignatureLimits::Sha512(RsaPssSha512SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap();
}

#[test]
fn rejects_message_and_limit_substitution_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let matching_message = CertificateVerifyMessage::new(0x0804, &signature).unwrap();
    let wrong_message = CertificateVerifyMessage::new(0x0805, &signature).unwrap();
    let mut input_storage = [0xa5; MAXIMUM_INPUT_LENGTH];
    let input = build_certificate_verify_input(
        CertificateVerifyRole::Server,
        TranscriptHashAlgorithm::Sha256,
        &TRANSCRIPT_HASH,
        &mut input_storage,
    )
    .unwrap();
    let hashed =
        hash_tls13_certificate_verify_input(authorized(SignatureScheme::RsaPssRsaeSha256), &input);
    let mut scratch = [0xa5; 223];

    let message_error = verify_tls13_rsa_pss_certificate_verify(
        &hashed,
        &wrong_message,
        key,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap_err();
    assert_eq!(
        message_error.kind(),
        Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(
            Tls13RsaPssSignatureRoutingErrorKind::SignatureSchemeMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 223]);

    let limits_error = verify_tls13_rsa_pss_certificate_verify(
        &hashed,
        &matching_message,
        key,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha384(RsaPssSha384SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap_err();
    assert_eq!(
        limits_error.kind(),
        Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(
            Tls13RsaPssSignatureRoutingErrorKind::LimitsProfileMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 223]);
}

#[test]
fn changed_input_invalidates_signature_and_diagnostics_are_redacted() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let message = CertificateVerifyMessage::new(0x0804, &signature).unwrap();
    let mut input_storage = [0xa5; MAXIMUM_INPUT_LENGTH];
    let input = build_certificate_verify_input(
        CertificateVerifyRole::Server,
        TranscriptHashAlgorithm::Sha256,
        &[0x43; 32],
        &mut input_storage,
    )
    .unwrap();
    let hashed =
        hash_tls13_certificate_verify_input(authorized(SignatureScheme::RsaPssRsaeSha256), &input);
    let mut scratch = [0xa5; 223];

    let error = verify_tls13_rsa_pss_certificate_verify(
        &hashed,
        &message,
        key,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(
            Tls13RsaPssSignatureRoutingErrorKind::Sha256(_)
        )
    ));
    let diagnostic = format!("{hashed:?} {error:?} {error}");
    assert!(!diagnostic.contains("TLS 1.3, server"));
    assert!(!diagnostic.contains("40992e"));
    assert!(!diagnostic.contains("729902"));
    assert!(!diagnostic.contains("300"));
    assert!(!diagnostic.contains("223"));
}
