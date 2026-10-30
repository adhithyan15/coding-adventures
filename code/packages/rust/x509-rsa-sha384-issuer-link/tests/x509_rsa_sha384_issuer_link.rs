use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use std::cell::Cell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{X509CertificateIssuerLinkErrorKind, X509CertificateRoleErrorKind};
use x509_rsa_pkcs1_v1_5_sha384_signature::X509RsaSha384SignatureErrorKind;
use x509_rsa_public_key::{RsaPublicKeyErrorKind, RsaPublicKeyPolicy};
use x509_rsa_sha384_issuer_link::{
    verify_x509_rsa_sha384_issuer_link, X509RsaSha384IssuerLinkErrorKind,
    X509RsaSha384IssuerLinkPolicy,
};

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const SHA384_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0c];
const BASIC_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x13];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];

// A static OpenSSL-generated 2048-bit self-signed CA certificate. OpenSSL is
// not invoked by the tests; this DER is an independent interoperability vector.
const SELF_SIGNED_CERTIFICATE_HEX: &str = concat!(
    "30820310308201f8a003020102020101300d06092a864886f70d01010c05003021311f301d06035504030c167368613338342e666978747572652e6578616d70",
    "6c65301e170d3236313033303032323832355a170d3237313033303032323832355a3021311f301d06035504030c167368613338342e666978747572652e6578",
    "616d706c6530820122300d06092a864886f70d01010105000382010f003082010a0282010100e5c26b4da0d68213228c242721a88219ab5cf64ca8067c27b789",
    "b36abaff76bdb64d57c37c2439f0ae19dd32eb41d280f68ee6d5e0ce816d9e2b385a73c3628b5ca9010b9582b937518c39d0c526a2c01328edd369e04c43ac72",
    "9308654fb519dac3304776f57db33e631343e152217198147708472871a4e25e8ecbbccb82879c47a92de08d03a869f5ed8a08473e596492157edb58ce513dea",
    "bd7721b967ea110bf511227bec9065539754fb7af7463fcd7df73bfd833e35eebc73ca4430077eced3fd162e7724295946087f3f40cf5549c3f9ffc1612d29f5",
    "4d2c3f78e6ad105d457158d476a6fd10722b1b7779768e21f77d5867b8a126c9e444198132b10203010001a3533051301d0603551d0e0416041416e34b32fc1c",
    "2724645c75555724c44dfc5dcf24301f0603551d2304183016801416e34b32fc1c2724645c75555724c44dfc5dcf24300f0603551d130101ff040530030101ff",
    "300d06092a864886f70d01010c05000382010100ba857105c96ad671b513d5c4e59fa68d741b63f751ffbdcbc0040c0859b3972422b693c83e81070abf690d61",
    "a2eba9b15d1e381a9d6af63ae87535e593ea641ccd5bc14fd592fd7152d8422d0de1dd9c3b67c680ac8910e883e17315faa3b9df1c036d62102f98ad5e1a1f06",
    "b76237dbd77af451a250812693f28b3f75b09a074d80fba4fb9c47286c2d7f9432657c3f1db95dfda27a0782ea429e770695033de21031d9a6496830098e89e9",
    "8d0a67d34c72f5cc83363b3ef88a77e18bac0e68d75bd4b3f326010cbc4b82d141ac3176f987bbd621d473aa042b7b5fab05c773c979a71a88154d1a9455bd74",
    "120ba2b39c2a63a36a070bde359f22cad2f46386",
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

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(value.len() + 4);
    encoded.push(tag);
    match value.len() {
        length @ 0..=127 => encoded.push(length as u8),
        length @ 128..=255 => encoded.extend([0x81, length as u8]),
        length @ 256..=65_535 => {
            encoded.extend([0x82, (length >> 8) as u8, length as u8]);
        }
        _ => panic!("test value too large"),
    }
    encoded.extend_from_slice(value);
    encoded
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(0x05, &[]));
    der(0x30, &fields)
}

fn extension(oid: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    if critical {
        fields.extend(der(0x01, &[0xff]));
    }
    fields.extend(der(0x04, value));
    der(0x30, &fields)
}

fn issuer_candidate(child: &X509Certificate<'_>) -> Vec<u8> {
    let signature_algorithm = algorithm(SHA384_WITH_RSA_ENCRYPTION_OID_DER);
    let mut validity = der(0x17, b"261030000000Z");
    validity.extend(der(0x17, b"361030000000Z"));

    let basic_constraints = extension(
        BASIC_CONSTRAINTS_OID_DER,
        true,
        &der(0x30, &der(0x01, &[0xff])),
    );
    let key_usage = extension(KEY_USAGE_OID_DER, true, &der(0x03, &[0x02, 0x04]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x02]));
    tbs.extend(&signature_algorithm);
    tbs.extend_from_slice(child.tbs_certificate().issuer().encoded());
    tbs.extend(der(0x30, &validity));
    tbs.extend_from_slice(child.tbs_certificate().issuer().encoded());
    tbs.extend(der(
        0x30,
        child.tbs_certificate().subject_public_key_info().contents(),
    ));
    tbs.extend(der(
        0xa3,
        &der(0x30, &[basic_constraints, key_usage].concat()),
    ));

    let mut certificate = der(0x30, &tbs);
    certificate.extend(&signature_algorithm);
    certificate.extend(der(0x03, &[0x00, 0x00]));
    der(0x30, &certificate)
}

fn fixture() -> Vec<u8> {
    decode_hex(SELF_SIGNED_CERTIFICATE_HEX)
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn policy() -> X509RsaSha384IssuerLinkPolicy<'static> {
    X509RsaSha384IssuerLinkPolicy::new(
        Asn1Limits::default(),
        RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap(),
        Asn1Limits::default(),
        RsaPublicOperationLimits::new(256).unwrap(),
    )
}

fn mutate_first(encoded: &mut [u8], needle: &[u8]) {
    let offset = encoded
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("fixture contains mutation target");
    encoded[offset + needle.len() - 1] ^= 1;
}

#[test]
fn verifies_one_exact_self_signed_ca_link_and_binds_both_certificates() {
    let child_encoded = fixture();
    let child = decode(&child_encoded);
    let issuer_encoded = issuer_candidate(&child);
    let candidate_issuer = decode(&issuer_encoded);
    let calls = Cell::new(0);

    let verified = verify_x509_rsa_sha384_issuer_link(
        &child,
        &candidate_issuer,
        |child_issuer, candidate_subject| {
            calls.set(calls.get() + 1);
            child_issuer.encoded() == candidate_subject.encoded()
        },
        policy(),
    )
    .unwrap();

    assert_eq!(calls.get(), 1);
    assert_eq!(format!("{verified:?}"), "VerifiedX509RsaSha384IssuerLink");
    assert!(std::ptr::eq(verified.child(), &child));
    assert!(std::ptr::eq(verified.candidate_issuer(), &candidate_issuer));
    assert_eq!(verified.issuer_public_key().modulus_bits(), 2_048);
    assert_eq!(verified.issuer_public_key().public_exponent(), 65_537);
}

#[test]
fn exposes_the_exact_caller_owned_policy() {
    let policy = policy();
    assert_eq!(policy.issuer_role_limits(), Asn1Limits::default());
    assert_eq!(policy.issuer_key_limits(), Asn1Limits::default());
    assert_eq!(policy.issuer_key_policy().minimum_modulus_bits(), 2_048);
    assert_eq!(
        policy.public_operation_limits().maximum_modulus_bytes(),
        256
    );
}

#[test]
fn rejects_name_mismatch_before_later_policy() {
    let encoded = fixture();
    let child = decode(&encoded);
    let issuer_encoded = issuer_candidate(&child);
    let candidate_issuer = decode(&issuer_encoded);
    let error = verify_x509_rsa_sha384_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| false,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha384IssuerLinkErrorKind::IssuerName(
            X509CertificateIssuerLinkErrorKind::NameMismatch
        )
    ));
    assert!(!error.to_string().contains("sha384.fixture.example"));
}

#[test]
fn rejects_a_candidate_without_issuing_ca_role() {
    let child_encoded = fixture();
    let child = decode(&child_encoded);
    let mut issuer_encoded = issuer_candidate(&child);
    mutate_first(&mut issuer_encoded, BASIC_CONSTRAINTS_OID_DER);
    let candidate_issuer = decode(&issuer_encoded);
    let error = verify_x509_rsa_sha384_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| true,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha384IssuerLinkErrorKind::IssuerRole(
            X509CertificateRoleErrorKind::MissingBasicConstraints
        )
    ));
}

#[test]
fn rejects_a_candidate_with_an_unapproved_public_key_algorithm() {
    let child_encoded = fixture();
    let child = decode(&child_encoded);
    let mut issuer_encoded = issuer_candidate(&child);
    mutate_first(&mut issuer_encoded, RSA_ENCRYPTION_OID_DER);
    let candidate_issuer = decode(&issuer_encoded);
    let error = verify_x509_rsa_sha384_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| true,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha384IssuerLinkErrorKind::IssuerPublicKey(RsaPublicKeyErrorKind::WrongAlgorithm)
    ));
}

#[test]
fn rejects_a_corrupted_child_signature() {
    let mut child_encoded = fixture();
    let last = child_encoded.last_mut().unwrap();
    *last ^= 1;
    let child = decode(&child_encoded);
    let issuer_encoded = issuer_candidate(&child);
    let candidate_issuer = decode(&issuer_encoded);
    let error = verify_x509_rsa_sha384_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| true,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha384IssuerLinkErrorKind::ChildSignature(
            X509RsaSha384SignatureErrorKind::EncodedMessage(_)
        )
    ));
}
