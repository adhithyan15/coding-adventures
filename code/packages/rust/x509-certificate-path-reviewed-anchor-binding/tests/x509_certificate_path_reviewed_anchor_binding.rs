use der_asn1::Asn1Limits;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_construction::{
    construct_unique_x509_certificate_path_by, ConstructedX509CertificatePath,
};
use x509_certificate_path_reviewed_anchor_binding::{
    bind_constructed_x509_certificate_path_to_reviewed_anchor,
    X509CertificatePathReviewedAnchorBindingErrorKind,
};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;
use x509_webpki_roots_snapshot::{reviewed_webpki_roots, TrustAnchor};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x03];

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

fn algorithm() -> Vec<u8> {
    der(0x30, &der(0x06, SIGNATURE_OID))
}

fn validity() -> Vec<u8> {
    let mut fields = der(0x17, b"261001000000Z");
    fields.extend(der(0x17, b"271001000000Z"));
    der(0x30, &fields)
}

fn name_contents(common_name: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, common_name));
    der(0x31, &der(0x30, &attribute))
}

fn test_subject_public_key_info_contents() -> Vec<u8> {
    let mut fields = algorithm();
    fields.extend(der(0x03, &[0x00, 0x01]));
    fields
}

fn certificate(
    issuer_contents: &[u8],
    subject_contents: &[u8],
    subject_public_key_info_contents: &[u8],
) -> Vec<u8> {
    let mut tbs_fields = der(0x02, &[0x01]);
    tbs_fields.extend(algorithm());
    tbs_fields.extend(der(0x30, issuer_contents));
    tbs_fields.extend(validity());
    tbs_fields.extend(der(0x30, subject_contents));
    tbs_fields.extend(der(0x30, subject_public_key_info_contents));
    let mut certificate_fields = der(0x30, &tbs_fields);
    certificate_fields.extend(algorithm());
    certificate_fields.extend(der(0x03, &[0x00, 0x00]));
    der(0x30, &certificate_fields)
}

fn decoded(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn construct_path<'target, 'target_encoded, 'candidates, 'candidate_encoded>(
    target: &'target X509Certificate<'target_encoded>,
    candidates: &'candidates [X509Certificate<'candidate_encoded>],
    terminal_subject_contents: &[u8],
) -> ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded> {
    construct_unique_x509_certificate_path_by(
        target,
        candidates,
        8,
        8,
        |certificate| {
            certificate.tbs_certificate().subject().contents() == terminal_subject_contents
        },
        |issuer, subject| issuer.encoded() == subject.encoded(),
    )
    .unwrap()
}

fn first_reviewed_anchor() -> TrustAnchor<'static> {
    reviewed_webpki_roots().iter().next().unwrap().unwrap()
}

#[test]
fn binds_the_exact_constructed_terminal_candidate_to_reviewed_anchor_evidence() {
    let anchor = first_reviewed_anchor();
    let root_encoded = certificate(
        anchor.subject(),
        anchor.subject(),
        anchor.subject_public_key_info(),
    );
    let target_encoded = certificate(
        anchor.subject(),
        &name_contents(b"target.example"),
        &test_subject_public_key_info_contents(),
    );
    let target = decoded(&target_encoded);
    let candidates = [decoded(&root_encoded)];
    let path = construct_path(&target, &candidates, anchor.subject());
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&candidates[0]).unwrap();

    let bound =
        bind_constructed_x509_certificate_path_to_reviewed_anchor(path, reviewed_anchor).unwrap();

    assert_eq!(bound.constructed_path().certificate_count(), 2);
    assert_eq!(
        bound
            .constructed_path()
            .candidate_indices()
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert!(std::ptr::eq(
        bound.constructed_path().issuer_certificate(0).unwrap(),
        bound.reviewed_anchor().candidate_certificate()
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundConstructedX509CertificatePathToReviewedAnchor"
    );
}

#[test]
fn rejects_a_target_only_path_even_when_the_target_is_a_reviewed_root_identity() {
    let anchor = first_reviewed_anchor();
    let root_encoded = certificate(
        anchor.subject(),
        anchor.subject(),
        anchor.subject_public_key_info(),
    );
    let target = decoded(&root_encoded);
    let candidates = Vec::new();
    let path = construct_path(&target, &candidates, anchor.subject());
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&target).unwrap();

    let error = bind_constructed_x509_certificate_path_to_reviewed_anchor(path, reviewed_anchor)
        .expect_err("a server path must select at least one issuer before its anchor");

    assert_eq!(
        error.kind(),
        X509CertificatePathReviewedAnchorBindingErrorKind::TargetIsTerminal
    );
    assert_eq!(
        error.to_string(),
        "X.509 certificate path reviewed-anchor binding error TargetIsTerminal"
    );
}

#[test]
fn rejects_reviewed_anchor_evidence_for_a_nonterminal_candidate() {
    let mut anchors = reviewed_webpki_roots().iter().map(Result::unwrap);
    let intermediate_identity = anchors.next().unwrap();
    let terminal_identity = anchors.next().unwrap();
    let intermediate_encoded = certificate(
        terminal_identity.subject(),
        intermediate_identity.subject(),
        intermediate_identity.subject_public_key_info(),
    );
    let root_encoded = certificate(
        terminal_identity.subject(),
        terminal_identity.subject(),
        terminal_identity.subject_public_key_info(),
    );
    let target_encoded = certificate(
        intermediate_identity.subject(),
        &name_contents(b"target.example"),
        &test_subject_public_key_info_contents(),
    );
    let target = decoded(&target_encoded);
    let candidates = [decoded(&intermediate_encoded), decoded(&root_encoded)];
    let path = construct_path(&target, &candidates, terminal_identity.subject());
    let wrong_anchor = select_reviewed_webpki_root_anchor(&candidates[0]).unwrap();

    let error = bind_constructed_x509_certificate_path_to_reviewed_anchor(path, wrong_anchor)
        .expect_err("a reviewed nonterminal candidate must not bind as the path anchor");

    assert_eq!(
        error.kind(),
        X509CertificatePathReviewedAnchorBindingErrorKind::EvidenceMismatch
    );
}

#[test]
fn rejects_byte_equal_terminal_certificate_evidence_from_a_distinct_decode() {
    let anchor = first_reviewed_anchor();
    let root_encoded = certificate(
        anchor.subject(),
        anchor.subject(),
        anchor.subject_public_key_info(),
    );
    let target_encoded = certificate(
        anchor.subject(),
        &name_contents(b"target.example"),
        &test_subject_public_key_info_contents(),
    );
    let target = decoded(&target_encoded);
    let candidates = [decoded(&root_encoded)];
    let independently_decoded_root = decoded(&root_encoded);
    let path = construct_path(&target, &candidates, anchor.subject());
    let wrong_anchor = select_reviewed_webpki_root_anchor(&independently_decoded_root).unwrap();

    let error = bind_constructed_x509_certificate_path_to_reviewed_anchor(path, wrong_anchor)
        .expect_err("byte-equal certificates from distinct decodes must not bind");

    assert_eq!(
        error.kind(),
        X509CertificatePathReviewedAnchorBindingErrorKind::EvidenceMismatch
    );
    assert!(!error.to_string().contains("target.example"));
}
