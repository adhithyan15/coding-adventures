use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
use rsa_pss_sha384_signature::RsaPssSha384SignatureLimits;
use rsa_pss_sha512_signature::RsaPssSha512SignatureLimits;
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_rsa_pss_signature_routing::{
    verify_tls13_rsa_pss_signature, RsaPssMessageDigest, RsaPssSignatureProfile,
    Tls13RsaPssSignatureLimits, Tls13RsaPssSignatureRoutingErrorKind, VerifiedTls13RsaPssSignature,
};
use tls13_signature_scheme_policy::{
    AuthorizedSignatureScheme, SignatureScheme, SignatureSchemePolicy,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const SHA256_DIGEST: [u8; 32] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];
const SHA384_DIGEST: [u8; 48] = [
    0xcb, 0x00, 0x75, 0x3f, 0x45, 0xa3, 0x5e, 0x8b, 0xb5, 0xa0, 0x3d, 0x69, 0x9a, 0xc6, 0x50, 0x07,
    0x27, 0x2c, 0x32, 0xab, 0x0e, 0xde, 0xd1, 0x63, 0x1a, 0x8b, 0x60, 0x5a, 0x43, 0xff, 0x5b, 0xed,
    0x80, 0x86, 0x07, 0x2b, 0xa1, 0xe7, 0xcc, 0x23, 0x58, 0xba, 0xec, 0xa1, 0x34, 0xc8, 0x25, 0xa7,
];
const SHA512_DIGEST: [u8; 64] = [
    0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20, 0x41, 0x31,
    0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55, 0xd3, 0x9a,
    0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23, 0xa3, 0xfe, 0xeb, 0xbd,
    0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e, 0x2a, 0x9a, 0xc9, 0x4f, 0xa5, 0x4c, 0xa4, 0x9f,
];

// Generated with OpenSSL 3.6.3 from one 2048-bit RSA key over the message
// `abc`, using PSS salt lengths 32, 48, and 64 respectively. Public SPKI
// SHA-256: 322ace8c256ccd4fc56400d5f6ebf9d8d42c3d33dffd4cd20f69a630783c54ca.
const PUBLIC_KEY_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a0282010100",
    "aaf4c35e78d18d27c6c5646b734fb1f2c5e34d59a6c346ed4e307b1f799fa57f",
    "9c2fc8f478dda0568fb7d9a008c69d9a4f3578cff390a2f4d1f91ea1a234e780",
    "faca60d56065bdc7f4e73845184e64a2ce87472c5e861eb8937d5bf1fece87095",
    "a5f69e3150a360e126f824252aba0d2c8d96cc016f03bdea7c68d5ade583b6e1",
    "88147bc96aa39fc76570785dabc9c39e98154e9fe147df2abda7a044217c96cc",
    "8932475c26db06599f4230accb2f991019acaf8181daacc0a096980219d53bd1",
    "f84cf83c0b439dae1c81ccbee5afb7094286ef933f65c717b26e635b6c55a2ad",
    "d045cf3a09045e32f851692d09f0f5e0fcb0ace77630893f469dcfa01bb1865",
    "0203010001",
);

const SHA256_SIGNATURE_HEX: &str = concat!(
    "07eeaf24fce70839e27694cfbd48eb8f9c0edb4e01758772109e69d468cc65d0",
    "e7b005d8e98506076b94ca7be1aa7da91e3bd205342c3ec68b5a824507d0598c",
    "150784bcdb2e03b189f24fc2d3872c9c945fa6c2b24ddc7e3e669ab62d019932",
    "73133addb1d87e703e7c7b0039c2871e96781a65fcfeb7f10eb0065dcbcb07ef",
    "35d850e00a707f5154ca9c830e3927ea9953192e05728e847a704ceb92b9711ab",
    "431ae023b216f070c8cbe5413a3d841b34df8fb4ca4122517dec393296251889f",
    "587d41d907fa03124c7f18e4f7c9185bada7ec2cb246a3724ca810bdf355279d",
    "ea8400ef781f7d079b80a48ff573b11d24bd965e6cd6ec8b30ea8070633afd",
);
const SHA384_SIGNATURE_HEX: &str = concat!(
    "54fd69d049c18f455c319f4f7b831468b622ecc026948a189af8cd07db7a161a",
    "edbdfa3239855232fd3264a7899e09d2f11572786ffb04a7ac4588883e063311",
    "0a70ff3cf862a588db37fdcffdcf9c89e59a94879abda2eab91d26223e10ce09",
    "27cc328bfab322d3530ffb5338b5ffb83d29e10ca2ecfdecccdc357842f787307",
    "fbe9dd7b611ef17b4bfbd305c3c837d177ab9e4ef0a71342b3a842385f73bc1",
    "22d0903f6f070c5f078ef61f1cc42a1de110b2e3185978cdae6162bccdd6b01f",
    "e02e7317018500b2acbfea7a6a3bf294a70609e80ea609f35b2ebd9bab5f21bd",
    "1c6bf6a556e1ec6d00c4c5e2f4232599aafad54049e85dca559754464d8a9e5d",
);
const SHA512_SIGNATURE_HEX: &str = concat!(
    "2e92d44a73a0e37e2f1e834aacb61dbf284c0fefd3987ba44266af1abd1eea5c",
    "43da67199cd029117c4afc97ad9d82b02d6538ba5528004570c08e11d4d30944",
    "f45d1102405eaae08992e9ae79c1a172617ba1ec424f72ce93cfc6d6d39b5b09",
    "2bb4ab6fac5f88e34ca83678cc3c5df54d40ae8c9a0c47983a8b07b8fa99fa0",
    "f1cce4e625e4ec776758d9a0c10a7d3591ca78e4a159e4ce7f20f83bb737630",
    "9a6c1247aeee0f8174133386b17b1659f41f972e4b997043e11ccb4d0382b5ef",
    "e3b0670b80c84becc8c741a0df1b33d3bf9d9ee682d7d61b9c2fa91b94013337",
    "122f5754eba31bdacba2082ea642c6e2da817d8499b15bf6b55d25c62e459afaec",
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
fn routes_sha256_and_binds_exact_inputs() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let message = CertificateVerifyMessage::new(0x0804, &signature).unwrap();
    let scheme = authorized(SignatureScheme::RsaPssRsaeSha256);
    let mut scratch = [0xa5; 223];

    let verified = verify_tls13_rsa_pss_signature(
        scheme,
        &message,
        key,
        RsaPssMessageDigest::Sha256(&SHA256_DIGEST),
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap();

    assert_eq!(verified.profile(), RsaPssSignatureProfile::Sha256);
    assert_eq!(verified.authorized_scheme(), scheme);
    assert_eq!(verified.message(), &message);
    assert_eq!(verified.public_key().modulus(), key.modulus());
    assert_eq!(verified.digest().as_bytes(), SHA256_DIGEST);
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedTls13RsaPssSignature::Sha256"
    );
    let VerifiedTls13RsaPssSignature::Sha256(routed) = verified else {
        panic!("wrong route")
    };
    assert_eq!(routed.signature().signature(), signature);
    assert_eq!(routed.signature().message_digest(), &SHA256_DIGEST);
}

#[test]
fn routes_sha384_and_sha512() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);

    let signature384 = decode_hex(SHA384_SIGNATURE_HEX);
    let message384 = CertificateVerifyMessage::new(0x0805, &signature384).unwrap();
    let mut scratch384 = [0xa5; 207];
    let verified384 = verify_tls13_rsa_pss_signature(
        authorized(SignatureScheme::RsaPssRsaeSha384),
        &message384,
        key,
        RsaPssMessageDigest::Sha384(&SHA384_DIGEST),
        &mut scratch384,
        Tls13RsaPssSignatureLimits::Sha384(RsaPssSha384SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap();
    assert_eq!(verified384.profile(), RsaPssSignatureProfile::Sha384);
    assert_eq!(verified384.digest().as_bytes(), SHA384_DIGEST);

    let signature512 = decode_hex(SHA512_SIGNATURE_HEX);
    let message512 = CertificateVerifyMessage::new(0x0806, &signature512).unwrap();
    let mut scratch512 = [0xa5; 191];
    let verified512 = verify_tls13_rsa_pss_signature(
        authorized(SignatureScheme::RsaPssRsaeSha512),
        &message512,
        key,
        RsaPssMessageDigest::Sha512(&SHA512_DIGEST),
        &mut scratch512,
        Tls13RsaPssSignatureLimits::Sha512(RsaPssSha512SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap();
    assert_eq!(verified512.profile(), RsaPssSignatureProfile::Sha512);
    assert_eq!(verified512.digest().as_bytes(), SHA512_DIGEST);
}

#[test]
fn rejects_every_cross_profile_input_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let sha256_message = CertificateVerifyMessage::new(0x0804, &signature).unwrap();
    let sha384_message = CertificateVerifyMessage::new(0x0805, &signature).unwrap();
    let scheme = authorized(SignatureScheme::RsaPssRsaeSha256);
    let sha256_limits =
        || Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(256, 256).unwrap());
    let mut scratch = [0xa5; 223];

    let wire = verify_tls13_rsa_pss_signature(
        scheme,
        &sha384_message,
        key,
        RsaPssMessageDigest::Sha256(&SHA256_DIGEST),
        &mut scratch,
        sha256_limits(),
    )
    .unwrap_err();
    assert_eq!(
        wire.kind(),
        Tls13RsaPssSignatureRoutingErrorKind::SignatureSchemeMismatch
    );
    assert_eq!(scratch, [0xa5; 223]);

    let digest = verify_tls13_rsa_pss_signature(
        scheme,
        &sha256_message,
        key,
        RsaPssMessageDigest::Sha384(&SHA384_DIGEST),
        &mut scratch,
        sha256_limits(),
    )
    .unwrap_err();
    assert_eq!(
        digest.kind(),
        Tls13RsaPssSignatureRoutingErrorKind::DigestProfileMismatch
    );
    assert_eq!(scratch, [0xa5; 223]);

    let limits = verify_tls13_rsa_pss_signature(
        scheme,
        &sha256_message,
        key,
        RsaPssMessageDigest::Sha256(&SHA256_DIGEST),
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha384(RsaPssSha384SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap_err();
    assert_eq!(
        limits.kind(),
        Tls13RsaPssSignatureRoutingErrorKind::LimitsProfileMismatch
    );
    assert_eq!(scratch, [0xa5; 223]);
}

#[test]
fn delegates_failures_and_redacts_diagnostics() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let signature = vec![0x42; 255];
    let message = CertificateVerifyMessage::new(0x0806, &signature).unwrap();
    let mut scratch = [0xa5; 191];

    let error = verify_tls13_rsa_pss_signature(
        authorized(SignatureScheme::RsaPssRsaeSha512),
        &message,
        key,
        RsaPssMessageDigest::Sha512(&SHA512_DIGEST),
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha512(RsaPssSha512SignatureLimits::new(256, 256).unwrap()),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        Tls13RsaPssSignatureRoutingErrorKind::Sha512(_)
    ));
    assert_eq!(scratch, [0xa5; 191]);
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("0806"));
    assert!(!diagnostic.contains("4242"));
    assert!(!diagnostic.contains("ddaf35"));
}
