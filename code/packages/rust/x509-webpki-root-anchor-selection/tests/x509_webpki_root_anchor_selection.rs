use der_asn1::Asn1Limits;
use x509_certificate::decode_x509_certificate;
use x509_webpki_root_anchor_selection::{
    select_reviewed_webpki_root_anchor, ReviewedWebPkiRootAnchorSelectionErrorKind,
};
use x509_webpki_roots_snapshot::{reviewed_webpki_roots, REVIEWED_ROOT_COUNT};

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

fn certificate(subject_contents: &[u8], subject_public_key_info: &[u8]) -> Vec<u8> {
    let mut tbs_fields = der(0x02, &[0x01]);
    tbs_fields.extend(algorithm());
    tbs_fields.extend(der(0x30, subject_contents));
    tbs_fields.extend(validity());
    tbs_fields.extend(der(0x30, subject_contents));
    tbs_fields.extend(der(0x30, subject_public_key_info));
    let mut certificate_fields = der(0x30, &tbs_fields);
    certificate_fields.extend(algorithm());
    certificate_fields.extend(der(0x03, &[0x00, 0x00]));
    der(0x30, &certificate_fields)
}

#[test]
fn selects_one_exact_reviewed_identity_and_binds_the_candidate() {
    let anchor = reviewed_webpki_roots().iter().next().unwrap().unwrap();
    let encoded = certificate(anchor.subject(), anchor.subject_public_key_info());
    let candidate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();

    let selected = select_reviewed_webpki_root_anchor(&candidate).unwrap();

    assert!(std::ptr::eq(selected.candidate_certificate(), &candidate));
    assert_eq!(selected.snapshot_index(), 0);
    assert_eq!(selected.subject_contents(), anchor.subject());
    assert_eq!(
        selected.subject_public_key_info_contents(),
        anchor.subject_public_key_info()
    );
    assert_eq!(selected.name_constraints(), anchor.name_constraints());
    assert_eq!(format!("{selected:?}"), "SelectedReviewedWebPkiRootAnchor");
}

#[test]
fn preserves_the_selected_anchor_name_constraints() {
    let (index, anchor) = reviewed_webpki_roots()
        .iter()
        .enumerate()
        .find_map(|(index, anchor)| {
            let anchor = anchor.unwrap();
            anchor.name_constraints().map(|_| (index, anchor))
        })
        .expect("reviewed snapshot has one constrained anchor");
    let encoded = certificate(anchor.subject(), anchor.subject_public_key_info());
    let candidate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();

    let selected = select_reviewed_webpki_root_anchor(&candidate).unwrap();

    assert_eq!(selected.snapshot_index(), index);
    assert_eq!(selected.name_constraints(), anchor.name_constraints());
}

#[test]
fn rejects_a_well_formed_unreviewed_identity_with_redacted_error() {
    let subject = der(0x31, &{
        let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
        attribute.extend(der(0x0c, b"unreviewed.example"));
        der(0x30, &attribute)
    });
    let key = {
        let mut fields = algorithm();
        fields.extend(der(0x03, &[0x00, 0x01]));
        fields
    };
    let encoded = certificate(&subject, &key);
    let candidate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();

    let error = select_reviewed_webpki_root_anchor(&candidate).unwrap_err();

    assert_eq!(
        error.kind(),
        ReviewedWebPkiRootAnchorSelectionErrorKind::NoMatch
    );
    assert!(!error.to_string().contains("unreviewed.example"));
}

#[test]
fn reviewed_snapshot_has_exactly_one_match_for_every_identity() {
    let anchors = reviewed_webpki_roots()
        .iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(anchors.len(), REVIEWED_ROOT_COUNT);
    for (index, anchor) in anchors.into_iter().enumerate() {
        let duplicate_count = reviewed_webpki_roots()
            .iter()
            .map(Result::unwrap)
            .filter(|candidate| {
                candidate.subject() == anchor.subject()
                    && candidate.subject_public_key_info() == anchor.subject_public_key_info()
            })
            .count();
        assert_eq!(duplicate_count, 1, "snapshot identity at index {index}");
    }
}
