use tls13_certificate_message::{
    decode_tls13_certificate_message, Tls13CertificateMessageErrorKind,
    Tls13CertificateMessageField, Tls13CertificateMessageLimits,
};

fn u24(value: usize) -> [u8; 3] {
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

fn entry(certificate: &[u8], extensions: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&u24(certificate.len()));
    encoded.extend_from_slice(certificate);
    encoded.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
    encoded.extend_from_slice(extensions);
    encoded
}

fn message(context: &[u8], entries: &[Vec<u8>]) -> Vec<u8> {
    let list: Vec<u8> = entries
        .iter()
        .flat_map(|entry| entry.iter().copied())
        .collect();
    let mut encoded = vec![context.len() as u8];
    encoded.extend_from_slice(context);
    encoded.extend_from_slice(&u24(list.len()));
    encoded.extend_from_slice(&list);
    encoded
}

fn limits() -> Tls13CertificateMessageLimits {
    Tls13CertificateMessageLimits::new(1024, 32, 900, 4, 512, 128)
}

#[test]
fn decodes_borrowed_context_and_entries() {
    let encoded = message(
        &[0xaa, 0xbb],
        &[
            entry(&[0x30, 0x01, 0x00], &[]),
            entry(&[0x30, 0x00], &[0, 1, 0, 0]),
        ],
    );
    let decoded = decode_tls13_certificate_message(&encoded, limits()).unwrap();
    assert_eq!(decoded.encoded(), encoded);
    assert_eq!(decoded.request_context(), [0xaa, 0xbb]);
    assert_eq!(decoded.certificate_count(), 2);
    let entries: Vec<_> = decoded.entries().collect();
    assert_eq!(entries[0].index(), 0);
    assert_eq!(entries[0].certificate_data(), [0x30, 0x01, 0x00]);
    assert!(entries[0].extensions().is_empty());
    assert_eq!(entries[1].index(), 1);
    assert_eq!(entries[1].certificate_data(), [0x30, 0x00]);
    assert_eq!(entries[1].extensions(), [0, 1, 0, 0]);
}

#[test]
fn accepts_structural_empty_list_and_full_limit_boundaries() {
    let empty = message(&[7], &[]);
    let decoded = decode_tls13_certificate_message(
        &empty,
        Tls13CertificateMessageLimits::new(empty.len(), 1, 0, 0, 0, 0),
    )
    .unwrap();
    assert_eq!(decoded.request_context(), [7]);
    assert_eq!(decoded.certificate_count(), 0);
    assert_eq!(decoded.entries().next(), None);

    let one_entry = entry(&[1, 2], &[3]);
    let encoded = message(&[], std::slice::from_ref(&one_entry));
    assert!(decode_tls13_certificate_message(
        &encoded,
        Tls13CertificateMessageLimits::new(encoded.len(), 0, one_entry.len(), 1, 2, 1),
    )
    .is_ok());
}

#[test]
fn enforces_all_caller_owned_limits() {
    let encoded = message(&[1, 2], &[entry(&[3, 4], &[5, 6])]);
    let cases = [
        (
            Tls13CertificateMessageLimits::new(encoded.len() - 1, 2, 20, 1, 2, 2),
            Tls13CertificateMessageErrorKind::MessageTooLarge,
        ),
        (
            Tls13CertificateMessageLimits::new(100, 1, 20, 1, 2, 2),
            Tls13CertificateMessageErrorKind::RequestContextTooLarge,
        ),
        (
            Tls13CertificateMessageLimits::new(100, 2, 8, 1, 2, 2),
            Tls13CertificateMessageErrorKind::CertificateListTooLarge,
        ),
        (
            Tls13CertificateMessageLimits::new(100, 2, 20, 0, 2, 2),
            Tls13CertificateMessageErrorKind::TooManyCertificates,
        ),
        (
            Tls13CertificateMessageLimits::new(100, 2, 20, 1, 1, 2),
            Tls13CertificateMessageErrorKind::CertificateTooLarge,
        ),
        (
            Tls13CertificateMessageLimits::new(100, 2, 20, 1, 2, 1),
            Tls13CertificateMessageErrorKind::ExtensionsTooLarge,
        ),
    ];
    for (limit, expected) in cases {
        assert_eq!(
            decode_tls13_certificate_message(&encoded, limit)
                .unwrap_err()
                .kind(),
            expected
        );
    }
}

#[test]
fn rejects_empty_certificate_and_trailing_data() {
    let empty_certificate = message(&[], &[entry(&[], &[])]);
    assert_eq!(
        decode_tls13_certificate_message(&empty_certificate, limits())
            .unwrap_err()
            .kind(),
        Tls13CertificateMessageErrorKind::EmptyCertificate
    );

    let mut trailing = message(&[], &[]);
    trailing.push(9);
    assert_eq!(
        decode_tls13_certificate_message(&trailing, limits())
            .unwrap_err()
            .kind(),
        Tls13CertificateMessageErrorKind::TrailingData
    );
}

#[test]
fn rejects_each_truncated_field() {
    let cases: &[(&[u8], Tls13CertificateMessageField)] = &[
        (&[], Tls13CertificateMessageField::RequestContext),
        (&[2, 1], Tls13CertificateMessageField::RequestContext),
        (
            &[0, 0, 0],
            Tls13CertificateMessageField::CertificateListLength,
        ),
        (&[0, 0, 0, 1], Tls13CertificateMessageField::CertificateList),
        (
            &[0, 0, 0, 2, 0, 0],
            Tls13CertificateMessageField::CertificateLength,
        ),
        (
            &[0, 0, 0, 5, 0, 0, 3, 1, 0],
            Tls13CertificateMessageField::Certificate,
        ),
        (
            &[0, 0, 0, 4, 0, 0, 1, 1],
            Tls13CertificateMessageField::ExtensionsLength,
        ),
        (
            &[0, 0, 0, 7, 0, 0, 1, 1, 0, 2, 9],
            Tls13CertificateMessageField::Extensions,
        ),
    ];
    for (encoded, field) in cases {
        assert_eq!(
            decode_tls13_certificate_message(encoded, limits())
                .unwrap_err()
                .kind(),
            Tls13CertificateMessageErrorKind::Truncated(*field)
        );
    }
}

#[test]
fn failures_are_payload_redacted() {
    let encoded = message(&[], &[entry(&[], &[])]);
    let error = decode_tls13_certificate_message(&encoded, limits()).unwrap_err();
    assert!(error.offset() > 0);
    assert!(!format!("{error:?}").contains("0000000000"));
    assert!(!error.to_string().contains("0000000000"));
}

#[test]
fn successful_debug_is_payload_redacted() {
    let encoded = message(&[222], &[entry(&[221], &[220])]);
    let decoded = decode_tls13_certificate_message(&encoded, limits()).unwrap();
    let message_debug = format!("{decoded:?}");
    let iterator_debug = format!("{:?}", decoded.entries());
    let entry_debug = format!("{:?}", decoded.entries().next().unwrap());
    for debug in [message_debug, iterator_debug, entry_debug] {
        assert!(!debug.contains("222"));
        assert!(!debug.contains("221"));
        assert!(!debug.contains("220"));
    }
}
