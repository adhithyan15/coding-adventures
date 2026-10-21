use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use std::cell::Cell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_issuer_candidate_selection::select_unique_x509_issuer_candidate_by;
use x509_certificate_policy::{
    validate_certificate_role_policy, X509CertificateIssuerLinkErrorKind, X509CertificateRole,
    X509CertificateRoleErrorKind,
};
use x509_rsa_pkcs1_v1_5_sha256_signature::X509RsaSha256SignatureErrorKind;
use x509_rsa_public_key::{RsaPublicKeyErrorKind, RsaPublicKeyPolicy};
use x509_rsa_sha256_issuer_link::{
    verify_selected_x509_rsa_sha256_issuer_link,
    verify_selected_x509_rsa_sha256_issuer_link_with_role_policy,
    verify_x509_rsa_sha256_issuer_link, X509RsaSha256IssuerLinkErrorKind,
    X509RsaSha256IssuerLinkPolicy, X509RsaSha256IssuerRolePolicyLinkErrorKind,
};

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const BASIC_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x13];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];

// A static OpenSSL-generated 2048-bit self-signed CA certificate. OpenSSL is
// not invoked by the tests; this is an independent interoperability vector.
const SELF_SIGNED_CA_HEX: &str = concat!(
    "30820312308201faa003020102020101300d06092a864886f70d01010b0500301a",
    "3118301606035504030c0f666978747572652e6578616d706c65301e170d3236",
    "313031393136353130395a170d3237313031393136353130395a301a31183016",
    "06035504030c0f666978747572652e6578616d706c6530820122300d06092a86",
    "4886f70d01010105000382010f003082010a0282010100b1704ece68a28c2002",
    "9617b6e50735035f7f4e719901f46c74f160a0cb8dfb504bac37977163d2e6a0",
    "7801b9f8e006f0045bfca893d7cddc9546279447451e65e94c52c82bb9b538b5",
    "40298cdd4211a97ba46a640e0509290b441e96dd6e3cd13946bfd4bbd39a5930",
    "0a6075a66aeca7115e7d41d258f171f2242883906ff9d1c52486eea86a3adae",
    "e5c52fbe04c0f074f1c4cc8f7ab216e3d3a227b319d8de0d28ef161baf9d620",
    "2287e70d500d245992876e67a52aae248fc60abbde6e9fb9964235bc8da41b5a",
    "a3cad65faf228b3bae1a1d0b00790aa696e7466ecb65b730e91d3042d92fb99d",
    "cf9e555c17444b42276b431046fc61dd40c67e7bf29d223b0203010001a36330",
    "61301d0603551d0e04160414ae8d3a1356755f6cd8cebae15bb23645b159ef66",
    "301f0603551d23041830168014ae8d3a1356755f6cd8cebae15bb23645b159ef",
    "66300f0603551d130101ff040530030101ff300e0603551d0f0101ff04040302",
    "0106300d06092a864886f70d01010b0500038201010004696164b97466d61f87",
    "2561c4d9efec6ad141cfe017c6273e3d8d2cee8cf20413b0eb798ac0119b326d",
    "6c534cc0b5a1b1169b158b9c6235196b5a57a9620f082ddbc4f73f759d1411ac",
    "447d33ce4669eaaf72d369b1449e9fb9a075f480b78ac571e52504268ffc2a43",
    "d178505a4aba282345fd74393ab189c57fb6fd7ff99f50ded63d595df45b19836",
    "f8d1aa36920c9d6ae220d15e9a1d98d6783ac9d04d385ac75aa3fd56929d992",
    "fa10b3745a5f3e616c9d062703f12538e730f94e1e456fcb3fa67a25f7e81c8",
    "04e53b3269d75c4b2d53ac835d2c419c25ad7d4236a740e337a16d340ae87bf",
    "2a38f3de4701edaf6720b752d544022206cf06cb35ed6f"
);

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

fn fixture() -> Vec<u8> {
    decode_hex(SELF_SIGNED_CA_HEX)
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn policy() -> X509RsaSha256IssuerLinkPolicy<'static> {
    X509RsaSha256IssuerLinkPolicy::new(
        Asn1Limits::default(),
        RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap(),
        Asn1Limits::default(),
        RsaPublicOperationLimits::new(256).unwrap(),
    )
}

fn mutate_first(encoded: &mut [u8], needle: &[u8], mutate: impl FnOnce(&mut [u8])) {
    let offset = encoded
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("fixture contains mutation target");
    mutate(&mut encoded[offset..offset + needle.len()]);
}

fn mutate_every(encoded: &mut [u8], needle: &[u8], mut mutate: impl FnMut(&mut [u8])) {
    let mut search_start = 0;
    let mut matches = 0;
    while let Some(relative_offset) = encoded[search_start..]
        .windows(needle.len())
        .position(|window| window == needle)
    {
        let offset = search_start + relative_offset;
        mutate(&mut encoded[offset..offset + needle.len()]);
        search_start = offset + needle.len();
        matches += 1;
    }
    assert!(matches > 0, "fixture contains mutation target");
}

#[test]
fn verifies_one_exact_self_signed_ca_link_and_binds_both_certificates() {
    let child_encoded = fixture();
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, &[0x02, 0x01, 0x01], |serial| {
        serial[2] = 2;
    });
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);
    let calls = Cell::new(0);

    let verified = verify_x509_rsa_sha256_issuer_link(
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
    assert_eq!(format!("{verified:?}"), "VerifiedX509RsaSha256IssuerLink");
    assert!(std::ptr::eq(verified.child(), &child));
    assert!(std::ptr::eq(verified.candidate_issuer(), &candidate_issuer));
    assert_eq!(verified.issuer_public_key().modulus_bits(), 2_048);
    assert_eq!(verified.issuer_public_key().public_exponent(), 65_537);
}

#[test]
fn authenticates_the_exact_pair_consumed_from_selection_evidence() {
    let child_encoded = fixture();
    let mut other_encoded = fixture();
    mutate_every(&mut other_encoded, b"fixture.example", |name| {
        name[0] = b'o';
    });
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, &[0x02, 0x01, 0x01], |serial| {
        serial[2] = 2;
    });
    let child = decode(&child_encoded);
    let candidates = vec![decode(&other_encoded), decode(&issuer_encoded)];
    let calls = Cell::new(0);

    let selected = select_unique_x509_issuer_candidate_by(
        &child,
        &candidates,
        2,
        |child_issuer, candidate_subject| {
            calls.set(calls.get() + 1);
            child_issuer.encoded() == candidate_subject.encoded()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(selected.candidate_index(), 1);

    let verified = verify_selected_x509_rsa_sha256_issuer_link(selected, policy()).unwrap();

    assert_eq!(calls.get(), 2, "authentication must not repeat Name policy");
    assert!(std::ptr::eq(verified.child(), &child));
    assert!(std::ptr::eq(verified.candidate_issuer(), &candidates[1]));
    assert_eq!(verified.issuer_public_key().modulus_bits(), 2_048);
}

#[test]
fn authenticates_selected_pair_with_exact_critical_role_policy_evidence() {
    let child_encoded = fixture();
    let mut other_encoded = fixture();
    mutate_every(&mut other_encoded, b"fixture.example", |name| {
        name[0] = b'o';
    });
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, &[0x02, 0x01, 0x01], |serial| {
        serial[2] = 2;
    });
    let child = decode(&child_encoded);
    let candidates = vec![decode(&other_encoded), decode(&issuer_encoded)];
    let name_calls = Cell::new(0);
    let critical_calls = Cell::new(0);

    let selected = select_unique_x509_issuer_candidate_by(
        &child,
        &candidates,
        2,
        |child_issuer, candidate_subject| {
            name_calls.set(name_calls.get() + 1);
            child_issuer.encoded() == candidate_subject.encoded()
        },
    )
    .unwrap();
    let issuer_role_policy = validate_certificate_role_policy(
        &candidates[1],
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| {
            critical_calls.set(critical_calls.get() + 1);
            oid.equals(&[2, 5, 29, 19]) || oid.equals(&[2, 5, 29, 15])
        },
    )
    .unwrap();

    let verified = verify_selected_x509_rsa_sha256_issuer_link_with_role_policy(
        selected,
        issuer_role_policy,
        policy(),
    )
    .unwrap();

    assert_eq!(name_calls.get(), 2);
    assert_eq!(critical_calls.get(), 2);
    assert!(std::ptr::eq(verified.verified_link().child(), &child));
    assert!(std::ptr::eq(
        verified.verified_link().candidate_issuer(),
        &candidates[1]
    ));
    assert!(std::ptr::eq(
        verified.issuer_role_policy().certificate(),
        &candidates[1]
    ));
    assert_eq!(
        verified.issuer_role_policy().role_extensions().role(),
        X509CertificateRole::IssuingCertificateAuthority
    );
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509RsaSha256IssuerRolePolicyLink"
    );
}

#[test]
fn rejects_role_policy_evidence_for_a_different_selected_candidate_first() {
    let child_encoded = fixture();
    let mut selected_issuer_encoded = fixture();
    mutate_first(
        &mut selected_issuer_encoded,
        RSA_ENCRYPTION_OID_DER,
        |oid| {
            oid[oid.len() - 1] = 0x0b;
        },
    );
    let mut role_issuer_encoded = fixture();
    mutate_first(&mut role_issuer_encoded, &[0x02, 0x01, 0x01], |serial| {
        serial[2] = 2;
    });
    let child = decode(&child_encoded);
    let candidates = vec![decode(&selected_issuer_encoded)];
    let role_issuer = decode(&role_issuer_encoded);

    let selected = select_unique_x509_issuer_candidate_by(
        &child,
        &candidates,
        1,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
    )
    .unwrap();
    let issuer_role_policy = validate_certificate_role_policy(
        &role_issuer,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 19]) || oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();

    let error = verify_selected_x509_rsa_sha256_issuer_link_with_role_policy(
        selected,
        issuer_role_policy,
        policy(),
    )
    .expect_err("different-certificate evidence must fail");

    assert_eq!(
        error.kind(),
        X509RsaSha256IssuerRolePolicyLinkErrorKind::EvidenceMismatch
    );
}

#[test]
fn rejects_non_ca_role_policy_evidence_for_the_selected_candidate_first() {
    let child_encoded = fixture();
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, BASIC_CONSTRAINTS_OID_DER, |oid| {
        oid[2] = 0x7e;
    });
    mutate_first(&mut issuer_encoded, KEY_USAGE_OID_DER, |oid| {
        oid[2] = 0x7d;
    });
    let child = decode(&child_encoded);
    let candidates = vec![decode(&issuer_encoded)];

    let selected = select_unique_x509_issuer_candidate_by(
        &child,
        &candidates,
        1,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
    )
    .unwrap();
    let issuer_role_policy = validate_certificate_role_policy(
        &candidates[0],
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |_| true,
    )
    .unwrap();

    let error = verify_selected_x509_rsa_sha256_issuer_link_with_role_policy(
        selected,
        issuer_role_policy,
        policy(),
    )
    .expect_err("non-CA role evidence must fail");

    assert_eq!(
        error.kind(),
        X509RsaSha256IssuerRolePolicyLinkErrorKind::EvidenceMismatch
    );
}

#[test]
fn exposes_the_exact_caller_owned_component_policies() {
    let policy = policy();
    assert_eq!(policy.issuer_role_limits(), Asn1Limits::default());
    assert_eq!(policy.issuer_key_limits(), Asn1Limits::default());
    assert_eq!(policy.issuer_key_policy().minimum_modulus_bits(), 2_048);
    assert_eq!(policy.issuer_key_policy().maximum_modulus_bits(), 2_048);
    assert_eq!(
        policy.issuer_key_policy().allowed_public_exponents(),
        &[65_537]
    );
    assert_eq!(
        policy.public_operation_limits().maximum_modulus_bytes(),
        256
    );
}

#[test]
fn rejects_name_policy_mismatch_before_issuer_processing() {
    let encoded = fixture();
    let child = decode(&encoded);
    let candidate_issuer = decode(&encoded);
    let calls = Cell::new(0);

    let error = verify_x509_rsa_sha256_issuer_link(
        &child,
        &candidate_issuer,
        |child_issuer, candidate_subject| {
            calls.set(calls.get() + 1);
            assert_eq!(
                child_issuer.encoded(),
                child.tbs_certificate().issuer().encoded()
            );
            assert_eq!(
                candidate_subject.encoded(),
                candidate_issuer.tbs_certificate().subject().encoded()
            );
            false
        },
        policy(),
    )
    .unwrap_err();

    assert_eq!(calls.get(), 1);
    assert_eq!(
        error.kind(),
        X509RsaSha256IssuerLinkErrorKind::IssuerName(
            X509CertificateIssuerLinkErrorKind::NameMismatch
        )
    );
}

#[test]
fn rejects_a_candidate_without_issuing_ca_role() {
    let child_encoded = fixture();
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, BASIC_CONSTRAINTS_OID_DER, |oid| {
        oid[2] = 0x12
    });
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);

    let error = verify_x509_rsa_sha256_issuer_link(
        &child,
        &candidate_issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        policy(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaSha256IssuerLinkErrorKind::IssuerRole(
            X509CertificateRoleErrorKind::MissingBasicConstraints
        )
    );
}

#[test]
fn rejects_a_candidate_with_the_wrong_public_key_algorithm() {
    let child_encoded = fixture();
    let mut issuer_encoded = fixture();
    mutate_first(&mut issuer_encoded, RSA_ENCRYPTION_OID_DER, |oid| {
        oid[oid.len() - 1] = 0x0b;
    });
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);

    let error = verify_x509_rsa_sha256_issuer_link(
        &child,
        &candidate_issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        policy(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaSha256IssuerLinkErrorKind::IssuerPublicKey(RsaPublicKeyErrorKind::WrongAlgorithm)
    );
}

#[test]
fn rejects_a_tampered_child_signature_with_redacted_error() {
    let mut child_encoded = fixture();
    let issuer_encoded = fixture();
    let last = child_encoded.len() - 1;
    child_encoded[last] ^= 1;
    let child = decode(&child_encoded);
    let candidate_issuer = decode(&issuer_encoded);

    let error = verify_x509_rsa_sha256_issuer_link(
        &child,
        &candidate_issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        policy(),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509RsaSha256IssuerLinkErrorKind::ChildSignature(
            X509RsaSha256SignatureErrorKind::EncodedMessage(_)
                | X509RsaSha256SignatureErrorKind::PublicOperation(_)
        )
    ));
    assert!(!error.to_string().contains("fixture"));
}
