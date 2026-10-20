use coding_adventures_sha256::sha256_hex;
use x509_webpki_roots_snapshot::{
    reviewed_webpki_roots, TrustAnchorSnapshot, TrustAnchorSnapshotErrorKind,
    CRATES_IO_ARCHIVE_SHA256, REVIEWED_ROOT_COUNT, ROOT_SET_SHA256, SNAPSHOT_METADATA_JSON,
    UPSTREAM_COMMIT, UPSTREAM_RELEASE, UPSTREAM_TAG,
};

const MAGIC: &[u8] = b"webpki-roots-canonical-v1\0";

fn encoded_anchor(subject: &[u8], key: &[u8], constraints: Option<&[u8]>) -> Vec<u8> {
    let mut encoded = MAGIC.to_vec();
    encoded.extend_from_slice(&(subject.len() as u64).to_be_bytes());
    encoded.extend_from_slice(subject);
    encoded.extend_from_slice(&(key.len() as u64).to_be_bytes());
    encoded.extend_from_slice(key);
    encoded.push(u8::from(constraints.is_some()));
    if let Some(constraints) = constraints {
        encoded.extend_from_slice(&(constraints.len() as u64).to_be_bytes());
        encoded.extend_from_slice(constraints);
    }
    encoded
}

#[test]
fn pins_reviewed_provenance_count_and_root_set_digest() {
    assert_eq!(UPSTREAM_RELEASE, "1.0.9");
    assert_eq!(UPSTREAM_TAG, "v/1.0.9");
    assert_eq!(UPSTREAM_COMMIT, "0a553dbc8b3f18ea05c4f881cffa3f2d005d0d30");
    assert_eq!(
        CRATES_IO_ARCHIVE_SHA256,
        "7dcd9d09a39985f5344844e66b0c530a33843579125f23e21e9f0f220850f22a"
    );
    assert_eq!(REVIEWED_ROOT_COUNT, 121);

    let snapshot = reviewed_webpki_roots();
    assert_eq!(sha256_hex(snapshot.encoded()), ROOT_SET_SHA256);
    assert_eq!(
        ROOT_SET_SHA256,
        "0aa729fbb339061e8aad2e49fefd83674cd6b2e45eeb586c0d456cf33b70dc01"
    );
    assert_eq!(snapshot.validate(), Ok(()));
    for pinned_value in [
        UPSTREAM_RELEASE,
        UPSTREAM_TAG,
        UPSTREAM_COMMIT,
        CRATES_IO_ARCHIVE_SHA256,
        ROOT_SET_SHA256,
    ] {
        assert!(SNAPSHOT_METADATA_JSON.contains(pinned_value));
    }
}

#[test]
fn exposes_only_borrowed_anchor_fields_and_preserves_order() {
    let anchors = reviewed_webpki_roots()
        .iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(anchors.len(), REVIEWED_ROOT_COUNT);
    assert_eq!(
        anchors[0].subject(),
        b"1\x0b0\t\x06\x03U\x04\x06\x13\x02US1\x170\x15\x06\x03U\x04\n\x13\x0eDigiCert, Inc.1&0$\x06\x03U\x04\x03\x13\x1dDigiCert TLS ECC P384 Root G5"
    );
    assert!(anchors
        .iter()
        .all(|anchor| !anchor.subject_public_key_info().is_empty()));
    assert_eq!(
        anchors
            .iter()
            .filter(|anchor| anchor.name_constraints().is_some())
            .count(),
        1
    );
}

#[test]
fn parses_a_minimal_canonical_snapshot() {
    let encoded = encoded_anchor(b"subject", b"key", Some(b"constraints"));
    let snapshot = TrustAnchorSnapshot::from_bytes(&encoded, 1);
    assert_eq!(snapshot.expected_count(), 1);
    assert_eq!(snapshot.validate(), Ok(()));
    let anchor = snapshot.iter().next().unwrap().unwrap();
    assert_eq!(anchor.subject(), b"subject");
    assert_eq!(anchor.subject_public_key_info(), b"key");
    assert_eq!(anchor.name_constraints(), Some(b"constraints".as_slice()));
}

#[test]
fn rejects_invalid_magic_truncation_and_count_drift() {
    let invalid_magic = TrustAnchorSnapshot::from_bytes(b"not-a-snapshot", 0);
    assert_eq!(
        invalid_magic.validate().unwrap_err().kind(),
        TrustAnchorSnapshotErrorKind::InvalidMagic
    );
    let mut invalid_magic_iter = invalid_magic.iter();
    assert!(invalid_magic_iter.next().unwrap().is_err());
    assert!(invalid_magic_iter.next().is_none());

    let truncated = TrustAnchorSnapshot::from_bytes(MAGIC, 1);
    assert_eq!(
        truncated.validate().unwrap_err().kind(),
        TrustAnchorSnapshotErrorKind::RootCountMismatch
    );

    let mut missing_value = MAGIC.to_vec();
    missing_value.extend_from_slice(&4u64.to_be_bytes());
    missing_value.extend_from_slice(b"ab");
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&missing_value, 1)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::Truncated
    );

    let mut missing_length = MAGIC.to_vec();
    missing_length.push(0);
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&missing_length, 1)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::Truncated
    );

    let mut overflowing_length = MAGIC.to_vec();
    overflowing_length.extend_from_slice(&u64::MAX.to_be_bytes());
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&overflowing_length, 1)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::LengthOverflow
    );

    let valid = encoded_anchor(b"subject", b"key", None);
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&valid, 2)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::RootCountMismatch
    );
}

#[test]
fn rejects_truncation_at_each_anchor_field_boundary() {
    let complete = encoded_anchor(b"subject", b"key", Some(b"constraints"));
    let boundaries = [
        MAGIC.len() + 8 + b"subject".len(),
        MAGIC.len() + 8 + b"subject".len() + 8 + b"key".len(),
        MAGIC.len() + 8 + b"subject".len() + 8 + b"key".len() + 1,
        complete.len() - 1,
    ];
    for boundary in boundaries {
        let truncated = &complete[..boundary];
        assert_eq!(
            TrustAnchorSnapshot::from_bytes(truncated, 1)
                .validate()
                .unwrap_err()
                .kind(),
            TrustAnchorSnapshotErrorKind::Truncated
        );
    }
}

#[test]
fn rejects_empty_fields_and_invalid_constraint_flags() {
    let empty_subject = encoded_anchor(b"", b"key", None);
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&empty_subject, 1)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::EmptySubject
    );

    let empty_key = encoded_anchor(b"subject", b"", None);
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&empty_key, 1)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::EmptySubjectPublicKeyInfo
    );

    let empty_constraints = encoded_anchor(b"subject", b"key", Some(b""));
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&empty_constraints, 1)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::EmptyNameConstraints
    );

    let mut invalid_flag = encoded_anchor(b"subject", b"key", None);
    *invalid_flag.last_mut().unwrap() = 2;
    assert_eq!(
        TrustAnchorSnapshot::from_bytes(&invalid_flag, 1)
            .validate()
            .unwrap_err()
            .kind(),
        TrustAnchorSnapshotErrorKind::InvalidNameConstraintsFlag
    );
}

#[test]
fn errors_retain_only_kind_and_local_offset() {
    let mut invalid_flag = encoded_anchor(b"subject", b"key", None);
    *invalid_flag.last_mut().unwrap() = 2;
    let error = TrustAnchorSnapshot::from_bytes(&invalid_flag, 1)
        .validate()
        .unwrap_err();
    assert_eq!(
        error.kind(),
        TrustAnchorSnapshotErrorKind::InvalidNameConstraintsFlag
    );
    assert_eq!(error.offset(), invalid_flag.len() - 1);
    assert!(!error.to_string().contains("subject"));
}
