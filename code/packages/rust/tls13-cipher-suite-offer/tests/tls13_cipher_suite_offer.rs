use tls13_cipher_suite_offer::{
    CipherSuiteOfferError, ClientCipherSuiteOffer, MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT,
    MAX_ENCODED_CLIENT_CIPHER_SUITE_OFFER_LENGTH,
};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, PolicyError,
};

fn policy(suites: &[CipherSuite]) -> CipherSuitePolicy {
    let mut entries = Vec::new();
    for suite in suites {
        entries.push(CipherSuitePolicyEntry::new(*suite, 1 << 20).unwrap());
    }
    CipherSuitePolicy::new(&entries).unwrap()
}

#[test]
fn preserves_order_and_unknown_identifiers_during_encoding() {
    let wire_ids = [0x0a0a, 0x1302, 0x1301];
    let offer = ClientCipherSuiteOffer::new(
        &wire_ids,
        policy(&[
            CipherSuite::TlsAes128GcmSha256,
            CipherSuite::TlsAes256GcmSha384,
        ]),
    )
    .unwrap();

    assert_eq!(offer.len(), 3);
    assert!(!offer.is_empty());
    assert_eq!(offer.get(0), Some(0x0a0a));
    assert_eq!(offer.get(1), Some(0x1302));
    assert_eq!(offer.get(3), None);
    assert!(offer.contains(0x1301));
    assert!(!offer.contains(0x1303));
    assert_eq!(offer.encoded_len(), 8);

    let mut output = [0xa5; 10];
    assert_eq!(offer.encode(&mut output), Ok(8));
    assert_eq!(&output[..8], &[0, 6, 0x0a, 0x0a, 0x13, 0x02, 0x13, 0x01]);
    assert_eq!(&output[8..], &[0xa5, 0xa5]);
}

#[test]
fn validates_offer_bounds_duplicates_and_usability() {
    let allowed = policy(&[CipherSuite::TlsAes128GcmSha256]);
    assert_eq!(
        ClientCipherSuiteOffer::new(&[], allowed).unwrap_err(),
        CipherSuiteOfferError::EmptyOffer
    );
    assert_eq!(
        ClientCipherSuiteOffer::new(&[0x1301, 0x1301], allowed).unwrap_err(),
        CipherSuiteOfferError::DuplicateCipherSuite
    );
    assert_eq!(
        ClientCipherSuiteOffer::new(&[0x0a0a, 0x1a1a], allowed).unwrap_err(),
        CipherSuiteOfferError::NoPolicyAuthorizedCipherSuite
    );

    let too_many = [0x1301; MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT + 1];
    assert_eq!(
        ClientCipherSuiteOffer::new(&too_many, allowed).unwrap_err(),
        CipherSuiteOfferError::TooManyCipherSuites
    );
}

#[test]
fn accepts_the_reviewed_maximum_unique_entry_count() {
    let mut wire_ids = [0_u16; MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT];
    for (index, wire_id) in wire_ids.iter_mut().enumerate() {
        *wire_id = 0x0a00 + index as u16;
    }
    wire_ids[MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT - 1] = 0x1301;

    let offer =
        ClientCipherSuiteOffer::new(&wire_ids, policy(&[CipherSuite::TlsAes128GcmSha256])).unwrap();
    assert_eq!(offer.len(), MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT);
    assert_eq!(
        offer.encoded_len(),
        MAX_ENCODED_CLIENT_CIPHER_SUITE_OFFER_LENGTH
    );

    let mut output = [0_u8; MAX_ENCODED_CLIENT_CIPHER_SUITE_OFFER_LENGTH];
    assert_eq!(offer.encode(&mut output), Ok(output.len()));
    assert_eq!(&output[..2], &[0, 128]);
    assert_eq!(&output[output.len() - 2..], &[0x13, 0x01]);
}

#[test]
fn encoding_capacity_failure_is_atomic() {
    let wire_ids = [0x1301, 0x1302];
    let offer = ClientCipherSuiteOffer::new(
        &wire_ids,
        policy(&[
            CipherSuite::TlsAes128GcmSha256,
            CipherSuite::TlsAes256GcmSha384,
        ]),
    )
    .unwrap();
    let mut output = [0x5a; 5];

    assert_eq!(
        offer.encode(&mut output),
        Err(CipherSuiteOfferError::OutputTooSmall)
    );
    assert_eq!(output, [0x5a; 5]);
}

#[test]
fn server_selection_must_be_offered_and_policy_authorized() {
    let wire_ids = [0x0a0a, 0x1301, 0x1302];
    let offer =
        ClientCipherSuiteOffer::new(&wire_ids, policy(&[CipherSuite::TlsAes128GcmSha256])).unwrap();

    let selected = offer.validate_server_selection(0x1301).unwrap();
    assert_eq!(
        selected.parameters().suite(),
        CipherSuite::TlsAes128GcmSha256
    );
    assert_eq!(selected.record_limit(), 1 << 20);
    assert_eq!(
        offer.validate_server_selection(0x1303).unwrap_err(),
        CipherSuiteOfferError::UnofferedServerSelection
    );
    assert_eq!(
        offer.validate_server_selection(0x1302).unwrap_err(),
        CipherSuiteOfferError::PolicyRejectedServerSelection(PolicyError::DisallowedCipherSuite)
    );
    assert_eq!(
        offer.validate_server_selection(0x0a0a).unwrap_err(),
        CipherSuiteOfferError::PolicyRejectedServerSelection(PolicyError::UnknownCipherSuite)
    );
}

#[test]
fn diagnostics_omit_offer_and_selection_identifiers() {
    let wire_ids = [0x0a0a, 0x1301];
    let offer =
        ClientCipherSuiteOffer::new(&wire_ids, policy(&[CipherSuite::TlsAes128GcmSha256])).unwrap();
    let debug = format!("{offer:?}");

    assert!(debug.contains("[OPAQUE POLICY DATA]"));
    assert!(!debug.contains("2570"));
    assert!(!debug.contains("4865"));

    let errors = [
        CipherSuiteOfferError::EmptyOffer,
        CipherSuiteOfferError::TooManyCipherSuites,
        CipherSuiteOfferError::DuplicateCipherSuite,
        CipherSuiteOfferError::NoPolicyAuthorizedCipherSuite,
        CipherSuiteOfferError::OutputTooSmall,
        CipherSuiteOfferError::UnofferedServerSelection,
        CipherSuiteOfferError::PolicyRejectedServerSelection(PolicyError::UnknownCipherSuite),
    ];
    for error in errors {
        assert!(!format!("{error:?}").contains("0x"));
        assert!(!error.to_string().contains("0x"));
    }
}
