use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use std::cell::Cell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{X509CertificateIssuerLinkErrorKind, X509CertificateRoleErrorKind};
use x509_rsa_pkcs1_v1_5_sha512_signature::X509RsaSha512SignatureErrorKind;
use x509_rsa_public_key::{RsaPublicKeyErrorKind, RsaPublicKeyPolicy};
use x509_rsa_sha512_issuer_link::{
    verify_x509_rsa_sha512_issuer_link, X509RsaSha512IssuerLinkErrorKind,
    X509RsaSha512IssuerLinkPolicy,
};

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const BASIC_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x13];

// A static OpenSSL-generated 2048-bit self-signed CA certificate. OpenSSL is
// not invoked by the tests; this DER is an independent interoperability vector.
const SELF_SIGNED_CERTIFICATE_HEX: &str = concat!(
    "3082032e30820216a003020102020101300d06092a864886f70d01010d050030283126302406035504030c1d7368613531322e6973737565722e666978747572",
    "652e6578616d706c65301e170d3236313033303032313932325a170d3336313032373032313932325a30283126302406035504030c1d7368613531322e697373",
    "7565722e666978747572652e6578616d706c6530820122300d06092a864886f70d01010105000382010f003082010a0282010100cfebdff45a1087937fbf518b",
    "92aae50c604304e6c1f104ee98b10e808732a91b4e0775bc938ea8b1e8b7f2ab5ea4812d0d12a8731e8c14273a5c1997a8daeaf8e51078aaea9715e8f65c1ee9",
    "143d9200a1067e37114b21a8ab6ee4145525317aa60d29610c783f631a035ba96184fc0d6c9545c34435eef88f51f67518027bb12f6d3bd3ffc68f6ae1466e20",
    "deab7bb7c254a4c3bdd40d9dc0193aadc06802f7378198643bad26c7eefd7c3ac9b166a335716203e18f03cb6f719c09b4978fe0ba9e57c6e6ceead4c5d20bd8",
    "bfabce1fe44a7cfdc79ba6e839cfa021348df4c0970d236cb8e589f34be97978a11126e35d35fa6e88aa514044b5b225370bcd3f0203010001a3633061301d06",
    "03551d0e04160414850586ed6e7157aedf9a651a4406ca0e8a518c81301f0603551d23041830168014850586ed6e7157aedf9a651a4406ca0e8a518c81300f06",
    "03551d130101ff040530030101ff300e0603551d0f0101ff040403020106300d06092a864886f70d01010d050003820101003bfb2cc7d414d9ea41cbe4ec1e33",
    "ad926d7213ac15f28044aebdfda53a4a51adb8c01817ce489ea8e10624526668079d48f38970a981404417dc72858f0916f7b67c7007b6fa3c323515afc2b498",
    "6f221815d6a2668e3e425c781c9ab2c8f70aa560177fa8e2c9506df5b3a3971de2c86e2cb2ed0f79cd8ab39e93281498fbe54663082259882df4a4799e89f998",
    "c39003c12045a0ae075984e2336b2329528388bbef300c4e5d1a00d90a9048bc849c3901fafb30d0c264d42296fa2291643d5f30b23ea7d211b7b2232cc7ceb2",
    "9dddfd0eacfb63279d1bfe7700d665f65c02aca69a0947a59620e4aa459310d3089a3e492bcb1c57684fec8f62e70e39394f",
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

fn fixture() -> Vec<u8> {
    decode_hex(SELF_SIGNED_CERTIFICATE_HEX)
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn policy() -> X509RsaSha512IssuerLinkPolicy<'static> {
    X509RsaSha512IssuerLinkPolicy::new(
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
    let issuer_encoded = fixture();
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);
    let calls = Cell::new(0);

    let verified = verify_x509_rsa_sha512_issuer_link(
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
    assert_eq!(format!("{verified:?}"), "VerifiedX509RsaSha512IssuerLink");
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
    let candidate_issuer = decode(&encoded);
    let error = verify_x509_rsa_sha512_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| false,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha512IssuerLinkErrorKind::IssuerName(
            X509CertificateIssuerLinkErrorKind::NameMismatch
        )
    ));
    assert!(!error.to_string().contains("sha512.fixture.example"));
}

#[test]
fn rejects_a_candidate_without_issuing_ca_role() {
    let child_encoded = fixture();
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, BASIC_CONSTRAINTS_OID_DER);
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);
    let error = verify_x509_rsa_sha512_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| true,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha512IssuerLinkErrorKind::IssuerRole(
            X509CertificateRoleErrorKind::MissingBasicConstraints
        )
    ));
}

#[test]
fn rejects_a_candidate_with_an_unapproved_public_key_algorithm() {
    let child_encoded = fixture();
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, RSA_ENCRYPTION_OID_DER);
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);
    let error = verify_x509_rsa_sha512_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| true,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha512IssuerLinkErrorKind::IssuerPublicKey(RsaPublicKeyErrorKind::WrongAlgorithm)
    ));
}

#[test]
fn rejects_a_corrupted_child_signature() {
    let mut child_encoded = fixture();
    let issuer_encoded = fixture();
    let last = child_encoded.last_mut().unwrap();
    *last ^= 1;
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);
    let error = verify_x509_rsa_sha512_issuer_link(
        &child,
        &candidate_issuer,
        |_child_issuer, _candidate_subject| true,
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha512IssuerLinkErrorKind::ChildSignature(
            X509RsaSha512SignatureErrorKind::EncodedMessage(_)
        )
    ));
}
