use coding_adventures_bounded_json::{parse, serialize, JsonNumber, JsonValue};
use coding_adventures_oauth::{
    decode_authorization_server_metadata, prepare_authorization_server_metadata,
    AuthorizationServerMetadata, DeviceAuthorizationProfile, MetadataViolation, OAuthAuditAction,
    OAuthAuditError, OAuthAuditEvent, OAuthAuditSink, OAuthError, OAuthTraceId, ProviderId,
};
use coding_adventures_zeroize::Zeroizing;

const CASES: &str =
    include_str!("../../../../specs/fixtures/oauth-provider-metadata-v1/cases.json");

#[derive(Default)]
struct RecordingAudit {
    events: Vec<OAuthAuditEvent>,
}

impl OAuthAuditSink for RecordingAudit {
    fn publish(&mut self, event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
        self.events.push(event.clone());
        Ok(())
    }
}

fn object(value: &JsonValue) -> &[(String, JsonValue)] {
    let JsonValue::Object(fields) = value else {
        panic!("fixture value must be an object");
    };
    fields
}

fn member<'a>(fields: &'a [(String, JsonValue)], name: &str) -> &'a JsonValue {
    let matches = fields
        .iter()
        .filter(|(candidate, _)| candidate == name)
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "fixture field {name} must occur once");
    &matches[0].1
}

fn string<'a>(fields: &'a [(String, JsonValue)], name: &str) -> &'a str {
    let JsonValue::String(value) = member(fields, name) else {
        panic!("fixture field {name} must be a string");
    };
    value
}

fn boolean(fields: &[(String, JsonValue)], name: &str) -> bool {
    let JsonValue::Bool(value) = member(fields, name) else {
        panic!("fixture field {name} must be a boolean");
    };
    *value
}

fn strings(fields: &[(String, JsonValue)], name: &str) -> Vec<String> {
    let JsonValue::Array(values) = member(fields, name) else {
        panic!("fixture field {name} must be an array");
    };
    values
        .iter()
        .map(|value| {
            let JsonValue::String(value) = value else {
                panic!("fixture field {name} must contain only strings");
            };
            value.clone()
        })
        .collect()
}

fn decode_error_code(error: &OAuthError) -> &'static str {
    match error {
        OAuthError::InvalidMetadata(MetadataViolation::Endpoint) => "endpoint",
        OAuthError::InvalidMetadata(MetadataViolation::Issuer) => "issuer",
        OAuthError::InvalidMetadata(MetadataViolation::Pkce) => "pkce",
        OAuthError::InvalidMetadata(MetadataViolation::TokenAuthenticationSigningAlgorithm) => {
            "token_authentication_signing_algorithm"
        }
        _ => "unexpected",
    }
}

fn decode_case(
    fields: &[(String, JsonValue)],
    trace: OAuthTraceId,
    audit: &mut RecordingAudit,
) -> Result<AuthorizationServerMetadata, OAuthError> {
    let provider = ProviderId::new(string(fields, "provider")).unwrap();
    let issuer = string(fields, "issuer");
    let metadata = serialize(member(fields, "metadata")).unwrap();
    let first_event = audit.events.len();

    let request = prepare_authorization_server_metadata(provider.clone(), issuer, trace)
        .publish_then_release(audit)
        .unwrap();
    let decoded = decode_authorization_server_metadata(
        request.response_context(),
        200,
        "application/json",
        Zeroizing::new(metadata.into_bytes()),
    )
    .publish_then_release(audit);

    let events = &audit.events[first_event..];
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].action(), OAuthAuditAction::MetadataRequestPrepare);
    assert_eq!(
        events[1].action(),
        OAuthAuditAction::MetadataResponseValidate
    );
    for event in events {
        assert_eq!(event.provider(), &provider);
        assert_eq!(event.trace(), trace);
    }
    decoded
}

#[test]
fn synthetic_provider_profiles_share_one_metadata_conformance_contract() {
    let root = parse(CASES).unwrap();
    let root = object(&root);
    assert_eq!(string(root, "contract"), "oauth-provider-metadata-v1");
    assert_eq!(
        string(root, "provenance"),
        "synthetic profiles; no live provider values"
    );
    assert_eq!(
        member(root, "schema_version"),
        &JsonValue::Number(JsonNumber::Integer(1))
    );

    let JsonValue::Array(cases) = member(root, "cases") else {
        panic!("fixture cases must be an array");
    };
    assert!(cases.len() >= 9);

    let mut audit = RecordingAudit::default();
    for (index, case) in cases.iter().enumerate() {
        let case = object(case);
        let name = string(case, "name");
        let expected = object(member(case, "expected"));
        let trace = OAuthTraceId::new([(index + 1) as u8; 16]);
        let decoded = decode_case(case, trace, &mut audit);
        let expected_decode = string(expected, "decode");

        if expected_decode != "ok" {
            let error = decoded.unwrap_err();
            assert_eq!(decode_error_code(&error), expected_decode, "case {name}");
            continue;
        }

        let metadata = decoded.unwrap();
        assert_eq!(metadata.provider().as_str(), string(case, "provider"));
        assert_eq!(metadata.issuer(), string(case, "issuer"));
        assert_eq!(
            metadata.token_endpoint_auth_methods_supported(),
            strings(expected, "auth_methods"),
            "case {name}"
        );
        assert_eq!(
            metadata.token_endpoint_auth_signing_alg_values_supported(),
            strings(expected, "signing_algorithms"),
            "case {name}"
        );

        let device = DeviceAuthorizationProfile::from_metadata(&metadata, "fixture-client");
        assert_eq!(
            device.is_ok(),
            boolean(expected, "device_profile"),
            "case {name}"
        );

        match string(expected, "public_profile") {
            "rfc9207" => {
                let config = metadata
                    .into_provider_config("fixture-client", "http://127.0.0.1:49152/oauth/callback")
                    .unwrap();
                assert_eq!(config.provider().as_str(), string(case, "provider"));
            }
            "distinct_redirect" => {
                let config = metadata
                    .into_provider_config_with_distinct_redirect_uri(
                        "fixture-client",
                        "http://127.0.0.1:49153/oauth/callback",
                    )
                    .unwrap();
                assert_eq!(config.provider().as_str(), string(case, "provider"));
            }
            "rejected" => {
                assert!(
                    metadata
                        .into_provider_config(
                            "fixture-client",
                            "http://127.0.0.1:49154/oauth/callback",
                        )
                        .is_err()
                );
                let metadata = decode_case(case, trace, &mut audit).unwrap();
                assert!(metadata
                    .into_provider_config_with_distinct_redirect_uri(
                        "fixture-client",
                        "http://127.0.0.1:49155/oauth/callback",
                    )
                    .is_err());
            }
            mode => panic!("case {name} has unknown public profile {mode}"),
        }
    }
}
