use tls13_extension_block::{decode_extension_block, ExtensionBlock, ExtensionBlockLimits};
use tls13_extension_context_policy::{
    ExtensionContext, ExtensionContextPolicy, ExtensionContextPolicyError, ExtensionContextRule,
    ExtensionContextSet, ServerExtensionContext, STANDARD_TLS13_EXTENSION_RULES,
};

fn encoded_block(types: &[u16]) -> Vec<u8> {
    let payload_length = types.len() * 4;
    let mut encoded = Vec::with_capacity(payload_length + 2);
    encoded.extend_from_slice(&(payload_length as u16).to_be_bytes());
    for extension_type in types {
        encoded.extend_from_slice(&extension_type.to_be_bytes());
        encoded.extend_from_slice(&0_u16.to_be_bytes());
    }
    encoded
}

fn decode(encoded: &[u8]) -> ExtensionBlock<'_> {
    decode_extension_block(
        encoded,
        ExtensionBlockLimits::new(u16::MAX as usize, 64).expect("valid test limits"),
    )
    .expect("test block is structurally valid")
}

fn contexts(contexts: &[ExtensionContext]) -> ExtensionContextSet {
    let mut set = ExtensionContextSet::empty();
    for context in contexts {
        set = set.with(*context);
    }
    set
}

#[test]
fn standard_rules_encode_the_complete_rfc_8446_table() {
    use ExtensionContext::{
        Certificate, CertificateRequest, ClientHello, EncryptedExtensions, HelloRetryRequest,
        NewSessionTicket, ServerHello,
    };

    let expected = [
        (0, contexts(&[ClientHello, EncryptedExtensions])),
        (1, contexts(&[ClientHello, EncryptedExtensions])),
        (5, contexts(&[ClientHello, CertificateRequest, Certificate])),
        (10, contexts(&[ClientHello, EncryptedExtensions])),
        (13, contexts(&[ClientHello, CertificateRequest])),
        (14, contexts(&[ClientHello, EncryptedExtensions])),
        (15, contexts(&[ClientHello, EncryptedExtensions])),
        (16, contexts(&[ClientHello, EncryptedExtensions])),
        (
            18,
            contexts(&[ClientHello, CertificateRequest, Certificate]),
        ),
        (19, contexts(&[ClientHello, EncryptedExtensions])),
        (20, contexts(&[ClientHello, EncryptedExtensions])),
        (21, contexts(&[ClientHello])),
        (51, contexts(&[ClientHello, ServerHello, HelloRetryRequest])),
        (41, contexts(&[ClientHello, ServerHello])),
        (45, contexts(&[ClientHello])),
        (
            42,
            contexts(&[ClientHello, EncryptedExtensions, NewSessionTicket]),
        ),
        (44, contexts(&[ClientHello, HelloRetryRequest])),
        (43, contexts(&[ClientHello, ServerHello, HelloRetryRequest])),
        (47, contexts(&[ClientHello, CertificateRequest])),
        (48, contexts(&[CertificateRequest])),
        (49, contexts(&[ClientHello])),
        (50, contexts(&[ClientHello, CertificateRequest])),
    ];

    assert_eq!(STANDARD_TLS13_EXTENSION_RULES.len(), expected.len());
    for ((expected_type, expected_contexts), actual) in
        expected.into_iter().zip(STANDARD_TLS13_EXTENSION_RULES)
    {
        assert_eq!(actual.extension_type().wire_value(), expected_type);
        assert_eq!(actual.contexts(), expected_contexts);
    }
}

#[test]
fn caller_owned_rules_are_bounded_unique_and_data_driven() {
    let private_rule = ExtensionContextRule::new(
        0xff01,
        ExtensionContextSet::of(ExtensionContext::ClientHello),
    );
    let private_rules = [private_rule];
    let policy = ExtensionContextPolicy::new(&private_rules).expect("one rule is valid");
    assert_eq!(policy.len(), 1);
    assert!(!policy.is_empty());

    let private_encoded = encoded_block(&[0xff01]);
    assert_eq!(
        policy.validate_client_hello(&decode(&private_encoded)),
        Ok(())
    );

    let empty_context_rule = ExtensionContextRule::new(1, ExtensionContextSet::empty());
    assert_eq!(
        ExtensionContextPolicy::new(&[empty_context_rule]),
        Err(ExtensionContextPolicyError::EmptyRuleContexts)
    );
    assert_eq!(
        ExtensionContextPolicy::new(&[private_rule, private_rule]),
        Err(ExtensionContextPolicyError::DuplicateRule)
    );

    let too_many = [private_rule; 65];
    assert_eq!(
        ExtensionContextPolicy::new(&too_many),
        Err(ExtensionContextPolicyError::TooManyRules)
    );
    assert!(ExtensionContextPolicy::new(&[]).unwrap().is_empty());
}

#[test]
fn client_hello_requires_known_allowed_types_and_psk_last() {
    let policy = ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap();

    let valid_encoded = encoded_block(&[0, 43, 51, 41]);
    assert_eq!(
        policy.validate_client_hello(&decode(&valid_encoded)),
        Ok(())
    );

    let psk_not_last = encoded_block(&[0, 41, 51]);
    assert_eq!(
        policy.validate_client_hello(&decode(&psk_not_last)),
        Err(ExtensionContextPolicyError::PreSharedKeyNotLast)
    );

    let wrong_context = encoded_block(&[48]);
    assert_eq!(
        policy.validate_client_hello(&decode(&wrong_context)),
        Err(ExtensionContextPolicyError::ForbiddenContext)
    );

    let unknown = encoded_block(&[0xff01]);
    assert_eq!(
        policy.validate_client_hello(&decode(&unknown)),
        Err(ExtensionContextPolicyError::UnrecognizedExtension)
    );

    let empty = encoded_block(&[]);
    assert_eq!(policy.validate_client_hello(&decode(&empty)), Ok(()));
}

#[test]
fn server_responses_require_matching_client_offers_except_hrr_cookie() {
    let policy = ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap();
    let offered_encoded = encoded_block(&[0, 5, 13, 43, 51]);
    let offered = decode(&offered_encoded);

    let server_hello_encoded = encoded_block(&[43, 51]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::ServerHello,
            &decode(&server_hello_encoded),
            &offered,
        ),
        Ok(())
    );

    let encrypted_extensions_encoded = encoded_block(&[0]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::EncryptedExtensions,
            &decode(&encrypted_extensions_encoded),
            &offered,
        ),
        Ok(())
    );

    let retry_encoded = encoded_block(&[43, 44, 51]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::HelloRetryRequest,
            &decode(&retry_encoded),
            &offered,
        ),
        Ok(())
    );

    let certificate_encoded = encoded_block(&[5]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::Certificate,
            &decode(&certificate_encoded),
            &offered,
        ),
        Ok(())
    );

    let not_offered = encoded_block(&[42]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::EncryptedExtensions,
            &decode(&not_offered),
            &offered,
        ),
        Err(ExtensionContextPolicyError::UnsolicitedExtension)
    );
}

#[test]
fn server_requests_and_tickets_apply_context_without_offer_binding() {
    let policy = ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap();
    let client_encoded = encoded_block(&[]);
    let client = decode(&client_encoded);

    let request_encoded = encoded_block(&[5, 13, 18, 47, 48, 50]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::CertificateRequest,
            &decode(&request_encoded),
            &client,
        ),
        Ok(())
    );

    let ticket_encoded = encoded_block(&[42]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::NewSessionTicket,
            &decode(&ticket_encoded),
            &client,
        ),
        Ok(())
    );

    let wrong_ticket_encoded = encoded_block(&[0]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::NewSessionTicket,
            &decode(&wrong_ticket_encoded),
            &client,
        ),
        Err(ExtensionContextPolicyError::ForbiddenContext)
    );
}

#[test]
fn server_validation_rejects_invalid_offers_wrong_contexts_and_unknown_types() {
    let policy = ExtensionContextPolicy::new(&STANDARD_TLS13_EXTENSION_RULES).unwrap();
    let invalid_offer_encoded = encoded_block(&[41, 51]);
    let empty_encoded = encoded_block(&[]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::ServerHello,
            &decode(&empty_encoded),
            &decode(&invalid_offer_encoded),
        ),
        Err(ExtensionContextPolicyError::PreSharedKeyNotLast)
    );

    let valid_offer_encoded = encoded_block(&[13]);
    let wrong_context_encoded = encoded_block(&[13]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::ServerHello,
            &decode(&wrong_context_encoded),
            &decode(&valid_offer_encoded),
        ),
        Err(ExtensionContextPolicyError::ForbiddenContext)
    );

    let unknown_encoded = encoded_block(&[0xff01]);
    assert_eq!(
        policy.validate_server_message(
            ServerExtensionContext::CertificateRequest,
            &decode(&unknown_encoded),
            &decode(&valid_offer_encoded),
        ),
        Err(ExtensionContextPolicyError::UnrecognizedExtension)
    );
}

#[test]
fn diagnostics_are_payload_free_and_stable() {
    let errors = [
        ExtensionContextPolicyError::TooManyRules,
        ExtensionContextPolicyError::EmptyRuleContexts,
        ExtensionContextPolicyError::DuplicateRule,
        ExtensionContextPolicyError::UnrecognizedExtension,
        ExtensionContextPolicyError::ForbiddenContext,
        ExtensionContextPolicyError::UnsolicitedExtension,
        ExtensionContextPolicyError::PreSharedKeyNotLast,
    ];

    for error in errors {
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(!display.contains("0x"));
        assert!(!debug.contains("0x"));
        assert!(!display.contains("ff01"));
        assert!(!debug.contains("ff01"));
    }

    let runtime_union = ExtensionContextSet::of(ExtensionContext::ClientHello)
        .union(ExtensionContextSet::of(ExtensionContext::ServerHello));
    assert!(runtime_union.contains(ExtensionContext::ClientHello));
    assert!(runtime_union.contains(ExtensionContext::ServerHello));
    assert!(!runtime_union.contains(ExtensionContext::Certificate));
    let context_debug = format!("{runtime_union:?}");
    assert!(context_debug.contains("client_hello: true"));
    assert!(context_debug.contains("server_hello: true"));
    assert!(context_debug.contains("encrypted_extensions: false"));
    assert!(context_debug.contains("certificate: false"));
    assert!(context_debug.contains("certificate_request: false"));
    assert!(context_debug.contains("new_session_ticket: false"));
    assert!(context_debug.contains("hello_retry_request: false"));
}
