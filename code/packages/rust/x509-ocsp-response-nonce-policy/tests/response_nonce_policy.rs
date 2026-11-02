use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_ocsp_response_data::{decode_x509_ocsp_response_data, X509OcspResponseData};
use x509_ocsp_response_nonce_policy::{
    validate_required_x509_ocsp_response_nonce, X509OcspResponseNoncePolicyError,
    X509OcspResponseNoncePolicyErrorKind, ID_PKIX_OCSP_NONCE_OID, MAX_OCSP_NONCE_LENGTH,
    REQUIRED_CLIENT_NONCE_LENGTH,
};

const NONCE_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x02];
const SUBJECT_ALT_NAME_OID_DER: &[u8] = &[0x55, 0x1d, 0x11];

fn der_length(length: usize) -> Vec<u8> {
    if length < 128 {
        return vec![length as u8];
    }
    let bytes = length.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap();
    let significant = &bytes[first..];
    let mut encoded = vec![0x80 | significant.len() as u8];
    encoded.extend_from_slice(significant);
    encoded
}

fn element(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    encoded.extend_from_slice(&der_length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let value: Vec<u8> = fields.iter().flatten().copied().collect();
    element(0x30, &value)
}

fn cert_id() -> Vec<u8> {
    let algorithm = sequence(&[
        element(0x06, &[0x2b, 0x0e, 0x03, 0x02, 0x1a]),
        vec![0x05, 0x00],
    ]);
    sequence(&[
        algorithm,
        element(0x04, &[0x11; 20]),
        element(0x04, &[0x22; 20]),
        element(0x02, &[1]),
    ])
}

fn single_response() -> Vec<u8> {
    sequence(&[
        cert_id(),
        vec![0x80, 0x00],
        element(0x18, b"20261024123456Z"),
    ])
}

fn extension(oid: &[u8], critical: bool, embedded_value: &[u8]) -> Vec<u8> {
    let mut fields = vec![element(0x06, oid)];
    if critical {
        fields.push(vec![0x01, 0x01, 0xff]);
    }
    fields.push(element(0x04, embedded_value));
    sequence(&fields)
}

fn response_extensions(values: &[Vec<u8>]) -> Vec<u8> {
    element(0xa1, &sequence(values))
}

fn response_data(optional_extensions: Option<Vec<u8>>) -> Vec<u8> {
    let mut fields = vec![
        element(0xa2, &element(0x04, &[0x44; 20])),
        element(0x18, b"20261024120000Z"),
        sequence(&[single_response()]),
    ];
    if let Some(extensions) = optional_extensions {
        fields.push(extensions);
    }
    sequence(&fields)
}

fn nonce_extension(nonce: &[u8]) -> Vec<u8> {
    extension(NONCE_OID_DER, false, &element(0x04, nonce))
}

fn evaluate<T>(
    encoded: &[u8],
    limits: Asn1Limits,
    requested_nonce: &[u8; REQUIRED_CLIENT_NONCE_LENGTH],
    evaluate: impl FnOnce(
        Result<
            x509_ocsp_response_nonce_policy::ValidatedX509OcspResponseNonce<'_, '_, '_>,
            X509OcspResponseNoncePolicyError,
        >,
        &X509OcspResponseData<'_>,
    ) -> T,
) -> T {
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(encoded).unwrap();
    let response = decode_x509_ocsp_response_data(&mut decoder, root).unwrap();
    let result =
        validate_required_x509_ocsp_response_nonce(&mut decoder, &response, requested_nonce);
    evaluate(result, &response)
}

fn error(
    encoded: &[u8],
    requested_nonce: &[u8; REQUIRED_CLIENT_NONCE_LENGTH],
) -> X509OcspResponseNoncePolicyError {
    evaluate(
        encoded,
        Asn1Limits::default(),
        requested_nonce,
        |result, _| result.err().expect("fixture must fail nonce policy"),
    )
}

#[test]
fn retains_the_exact_response_and_modern_request_nonce() {
    let requested_nonce = [0x5a; REQUIRED_CLIENT_NONCE_LENGTH];
    let encoded = response_data(Some(response_extensions(&[nonce_extension(
        &requested_nonce,
    )])));

    evaluate(
        &encoded,
        Asn1Limits::default(),
        &requested_nonce,
        |result, response| {
            let proof = result.unwrap();
            assert!(std::ptr::eq(proof.response_data(), response));
            assert!(std::ptr::eq(proof.requested_nonce(), &requested_nonce));
            assert_eq!(proof.requested_nonce(), &[0x5a; 32]);
        },
    );
    assert_eq!(ID_PKIX_OCSP_NONCE_OID, &[1, 3, 6, 1, 5, 5, 7, 48, 1, 2]);
    assert_eq!(MAX_OCSP_NONCE_LENGTH, 32);
}

#[test]
fn requires_one_exact_non_critical_nonce_extension() {
    let requested_nonce = [0x31; REQUIRED_CLIENT_NONCE_LENGTH];
    let missing = response_data(None);
    assert_eq!(
        error(&missing, &requested_nonce).kind(),
        X509OcspResponseNoncePolicyErrorKind::MissingResponseExtensions
    );

    let wrong = response_data(Some(response_extensions(&[extension(
        SUBJECT_ALT_NAME_OID_DER,
        false,
        &element(0x04, &requested_nonce),
    )])));
    assert_eq!(
        error(&wrong, &requested_nonce).kind(),
        X509OcspResponseNoncePolicyErrorKind::UnexpectedResponseExtension
    );

    let critical = response_data(Some(response_extensions(&[extension(
        NONCE_OID_DER,
        true,
        &element(0x04, &requested_nonce),
    )])));
    assert_eq!(
        error(&critical, &requested_nonce).kind(),
        X509OcspResponseNoncePolicyErrorKind::CriticalNonceExtension
    );

    let extra = response_data(Some(response_extensions(&[
        nonce_extension(&requested_nonce),
        extension(SUBJECT_ALT_NAME_OID_DER, false, &[0x05, 0x00]),
    ])));
    assert_eq!(
        error(&extra, &requested_nonce).kind(),
        X509OcspResponseNoncePolicyErrorKind::UnexpectedResponseExtensionCount
    );
}

#[test]
fn rejects_malformed_and_out_of_range_nonce_values() {
    let requested_nonce = [0xa7; REQUIRED_CLIENT_NONCE_LENGTH];
    for malformed in [vec![0x05, 0x00], vec![0x04, 0x01, 0xaa, 0x05, 0x00]] {
        let encoded = response_data(Some(response_extensions(&[extension(
            NONCE_OID_DER,
            false,
            &malformed,
        )])));
        assert!(matches!(
            error(&encoded, &requested_nonce).kind(),
            X509OcspResponseNoncePolicyErrorKind::InvalidNonceEncoding(_)
        ));
    }

    for invalid_length in [0, MAX_OCSP_NONCE_LENGTH + 1] {
        let nonce = vec![0x19; invalid_length];
        let encoded = response_data(Some(response_extensions(&[nonce_extension(&nonce)])));
        assert_eq!(
            error(&encoded, &requested_nonce).kind(),
            X509OcspResponseNoncePolicyErrorKind::InvalidNonceLength
        );
    }
}

#[test]
fn requires_the_complete_echoed_value() {
    let requested_nonce = [0x42; REQUIRED_CLIENT_NONCE_LENGTH];
    let short_but_rfc_valid =
        response_data(Some(response_extensions(&[nonce_extension(&[0x42; 16])])));
    assert_eq!(
        error(&short_but_rfc_valid, &requested_nonce).kind(),
        X509OcspResponseNoncePolicyErrorKind::NonceMismatch
    );

    let mut wrong_first = requested_nonce;
    wrong_first[0] ^= 1;
    let first = response_data(Some(response_extensions(&[nonce_extension(&wrong_first)])));
    assert_eq!(
        error(&first, &requested_nonce).kind(),
        X509OcspResponseNoncePolicyErrorKind::NonceMismatch
    );

    let mut wrong_last = requested_nonce;
    wrong_last[31] ^= 1;
    let last = response_data(Some(response_extensions(&[nonce_extension(&wrong_last)])));
    assert_eq!(
        error(&last, &requested_nonce).kind(),
        X509OcspResponseNoncePolicyErrorKind::NonceMismatch
    );
}

#[test]
fn embedded_nonce_preserves_depth_and_shared_element_budgets() {
    let requested_nonce = [0x77; REQUIRED_CLIENT_NONCE_LENGTH];
    let encoded = response_data(Some(response_extensions(&[nonce_extension(
        &requested_nonce,
    )])));

    let mut structural_decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = structural_decoder.decode_exact(&encoded).unwrap();
    let response = decode_x509_ocsp_response_data(&mut structural_decoder, root).unwrap();
    let mut depth_decoder = Asn1Decoder::new(Asn1Limits {
        max_depth: 5,
        ..Asn1Limits::default()
    });
    let depth_error =
        validate_required_x509_ocsp_response_nonce(&mut depth_decoder, &response, &requested_nonce)
            .err()
            .expect("depth limit must reject embedded nonce");
    assert_eq!(
        depth_error.kind(),
        X509OcspResponseNoncePolicyErrorKind::InvalidNonceEncoding(
            Asn1ErrorKind::DepthLimitExceeded
        )
    );
    let mut exact_depth_decoder = Asn1Decoder::new(Asn1Limits {
        max_depth: 6,
        ..Asn1Limits::default()
    });
    assert!(validate_required_x509_ocsp_response_nonce(
        &mut exact_depth_decoder,
        &response,
        &requested_nonce,
    )
    .is_ok());

    let element_error = evaluate(
        &encoded,
        Asn1Limits {
            max_total_elements: 20,
            ..Asn1Limits::default()
        },
        &requested_nonce,
        |result, _| {
            result
                .err()
                .expect("element limit must reject embedded nonce")
        },
    );
    assert_eq!(
        element_error.kind(),
        X509OcspResponseNoncePolicyErrorKind::InvalidNonceEncoding(
            Asn1ErrorKind::ElementLimitExceeded
        )
    );
    evaluate(
        &encoded,
        Asn1Limits {
            max_total_elements: 21,
            ..Asn1Limits::default()
        },
        &requested_nonce,
        |result, _| assert!(result.is_ok()),
    );
}

#[test]
fn errors_are_stable_and_do_not_disclose_nonce_bytes() {
    let requested_nonce = [0xde; REQUIRED_CLIENT_NONCE_LENGTH];
    let encoded = response_data(Some(response_extensions(&[nonce_extension(
        &[0xad; REQUIRED_CLIENT_NONCE_LENGTH],
    )])));
    let mismatch = error(&encoded, &requested_nonce);
    assert_eq!(mismatch.offset(), 0);
    assert_eq!(
        mismatch.to_string(),
        "X.509 OCSP response nonce policy error NonceMismatch at byte 0"
    );
    let debug = format!("{mismatch:?}");
    assert!(!debug.contains("222"));
    assert!(!debug.contains("173"));
}
