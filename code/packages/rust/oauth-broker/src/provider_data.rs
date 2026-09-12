//! Bounded static public-provider data decoding.

use super::{BrokerError, BrokerProvider};
use coding_adventures_bounded_json::{parse_with_depth_limit, JsonNumber, JsonValue};
use coding_adventures_oauth::{ProviderConfig, ProviderId, TokenResponseFormat};
use coding_adventures_zeroize::Zeroizing;
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA_VERSION: i64 = 1;
const MAX_PROVIDER_DATA_BYTES: usize = 64 * 1024;
const MAX_PROVIDER_DATA_DEPTH: usize = 16;
const MAX_PROVIDER_DATA_FIELDS: usize = 10;
const MAX_EXTRA_PARAMETERS: usize = 32;

impl BrokerProvider {
    /// Decode and validate one static public Authorization Code provider profile.
    ///
    /// The zeroizing input contains only non-secret provider policy; `client_id`
    /// and `redirect_uri` remain caller-owned deployment data. The schema is
    /// exact and versioned: unknown or duplicate fields fail closed, endpoints
    /// are validated by [`ProviderConfig`], PKCE remains mandatory in the OAuth
    /// core, and the mix-up defense must be selected explicitly. Reading a file
    /// or any other external effect remains the host's audited responsibility.
    ///
    /// The v1 object has this shape (`revocation_endpoint` is optional):
    ///
    /// ```text
    /// {
    ///   "schema_version": 1,
    ///   "provider": "example",
    ///   "authorization_endpoint": "https://login.example/authorize",
    ///   "token_endpoint": "https://login.example/token",
    ///   "revocation_endpoint": "https://login.example/revoke",
    ///   "mix_up_defense": {
    ///     "kind": "authorization_response_issuer",
    ///     "issuer": "https://login.example"
    ///   },
    ///   "authorization_extra_parameters": {},
    ///   "token_response_format": "json",
    ///   "refresh_lead_seconds": 300
    /// }
    /// ```
    pub fn from_public_provider_data(
        body: Zeroizing<Vec<u8>>,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<Self, BrokerError> {
        decode_public_provider_data(&body, client_id.into(), redirect_uri.into())
    }
}

fn decode_public_provider_data(
    body: &[u8],
    client_id: String,
    redirect_uri: String,
) -> Result<BrokerProvider, BrokerError> {
    if body.is_empty() || body.len() > MAX_PROVIDER_DATA_BYTES {
        return Err(BrokerError::InvalidProviderData);
    }
    let text = std::str::from_utf8(body).map_err(|_| BrokerError::InvalidProviderData)?;
    let root = parse_with_depth_limit(text, MAX_PROVIDER_DATA_DEPTH)
        .map_err(|_| BrokerError::InvalidProviderData)?;
    let fields = object(&root)?;
    exact_fields(
        fields,
        &[
            "schema_version",
            "provider",
            "authorization_endpoint",
            "token_endpoint",
            "revocation_endpoint",
            "mix_up_defense",
            "authorization_extra_parameters",
            "token_response_format",
            "refresh_lead_seconds",
        ],
        MAX_PROVIDER_DATA_FIELDS,
    )?;
    if integer(fields, "schema_version")? != SCHEMA_VERSION {
        return Err(BrokerError::InvalidProviderData);
    }

    let provider = ProviderId::new(string(fields, "provider")?)
        .map_err(|_| BrokerError::InvalidProviderData)?;
    let mut config = ProviderConfig::new(
        provider,
        string(fields, "authorization_endpoint")?,
        string(fields, "token_endpoint")?,
        client_id,
        redirect_uri,
    )
    .map_err(|_| BrokerError::InvalidProviderData)?;

    if let Some(endpoint) = optional_string(fields, "revocation_endpoint")? {
        config = config
            .with_revocation_endpoint(endpoint)
            .map_err(|_| BrokerError::InvalidProviderData)?;
    }
    config = apply_mix_up_defense(config, member(fields, "mix_up_defense")?)?;
    config = config
        .with_authorization_extra_parameters(string_map(member(
            fields,
            "authorization_extra_parameters",
        )?)?)
        .map_err(|_| BrokerError::InvalidProviderData)?;

    let response_format = match string(fields, "token_response_format")? {
        "json" => TokenResponseFormat::Json,
        "form_encoded" => TokenResponseFormat::FormEncoded,
        _ => return Err(BrokerError::InvalidProviderData),
    };
    let refresh_lead_seconds = nonnegative_integer(fields, "refresh_lead_seconds")?;
    BrokerProvider::new(config, response_format, refresh_lead_seconds)
        .map_err(|_| BrokerError::InvalidProviderData)
}

fn apply_mix_up_defense(
    config: ProviderConfig,
    value: &JsonValue,
) -> Result<ProviderConfig, BrokerError> {
    let fields = object(value)?;
    let kind = string(fields, "kind")?;
    match kind {
        "authorization_response_issuer" => {
            exact_fields(fields, &["kind", "issuer"], 2)?;
            config
                .with_expected_issuer(string(fields, "issuer")?)
                .map_err(|_| BrokerError::InvalidProviderData)
        }
        "distinct_redirect_uri" => {
            exact_fields(fields, &["kind"], 1)?;
            Ok(config.with_distinct_redirect_uri())
        }
        _ => Err(BrokerError::InvalidProviderData),
    }
}

fn object(value: &JsonValue) -> Result<&[(String, JsonValue)], BrokerError> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(BrokerError::InvalidProviderData),
    }
}

fn exact_fields(
    fields: &[(String, JsonValue)],
    allowed: &[&str],
    maximum: usize,
) -> Result<(), BrokerError> {
    if fields.len() > maximum {
        return Err(BrokerError::InvalidProviderData);
    }
    let mut seen = BTreeSet::new();
    for (name, _) in fields {
        if !allowed.contains(&name.as_str()) || !seen.insert(name.as_str()) {
            return Err(BrokerError::InvalidProviderData);
        }
    }
    Ok(())
}

fn member<'a>(fields: &'a [(String, JsonValue)], name: &str) -> Result<&'a JsonValue, BrokerError> {
    let mut matches = fields.iter().filter(|(candidate, _)| candidate == name);
    let value = matches
        .next()
        .map(|(_, value)| value)
        .ok_or(BrokerError::InvalidProviderData)?;
    if matches.next().is_some() {
        return Err(BrokerError::InvalidProviderData);
    }
    Ok(value)
}

fn optional_member<'a>(
    fields: &'a [(String, JsonValue)],
    name: &str,
) -> Result<Option<&'a JsonValue>, BrokerError> {
    let mut matches = fields.iter().filter(|(candidate, _)| candidate == name);
    let value = matches.next().map(|(_, value)| value);
    if matches.next().is_some() {
        return Err(BrokerError::InvalidProviderData);
    }
    Ok(value)
}

fn string<'a>(fields: &'a [(String, JsonValue)], name: &str) -> Result<&'a str, BrokerError> {
    match member(fields, name)? {
        JsonValue::String(value) => Ok(value),
        _ => Err(BrokerError::InvalidProviderData),
    }
}

fn optional_string<'a>(
    fields: &'a [(String, JsonValue)],
    name: &str,
) -> Result<Option<&'a str>, BrokerError> {
    match optional_member(fields, name)? {
        Some(JsonValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(BrokerError::InvalidProviderData),
        None => Ok(None),
    }
}

fn integer(fields: &[(String, JsonValue)], name: &str) -> Result<i64, BrokerError> {
    match member(fields, name)? {
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(*value),
        _ => Err(BrokerError::InvalidProviderData),
    }
}

fn nonnegative_integer(fields: &[(String, JsonValue)], name: &str) -> Result<u64, BrokerError> {
    u64::try_from(integer(fields, name)?).map_err(|_| BrokerError::InvalidProviderData)
}

fn string_map(value: &JsonValue) -> Result<BTreeMap<String, String>, BrokerError> {
    let fields = object(value)?;
    if fields.len() > MAX_EXTRA_PARAMETERS {
        return Err(BrokerError::InvalidProviderData);
    }
    let mut result = BTreeMap::new();
    for (name, value) in fields {
        let JsonValue::String(value) = value else {
            return Err(BrokerError::InvalidProviderData);
        };
        if result.insert(name.clone(), value.clone()).is_some() {
            return Err(BrokerError::InvalidProviderData);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_oauth::{
        begin_authorization, EntropySource, OAuthAuditError, OAuthAuditEvent, OAuthAuditSink,
        OAuthError, OAuthTraceId,
    };

    const VALID: &str = r#"{
        "schema_version":1,
        "provider":"fixture-static",
        "authorization_endpoint":"https://login.static.example/authorize",
        "token_endpoint":"https://login.static.example/token",
        "revocation_endpoint":"https://login.static.example/revoke",
        "mix_up_defense":{
            "kind":"authorization_response_issuer",
            "issuer":"https://login.static.example"
        },
        "authorization_extra_parameters":{
            "access_type":"offline",
            "prompt":"consent"
        },
        "token_response_format":"json",
        "refresh_lead_seconds":300
    }"#;

    struct FixedEntropy;

    impl EntropySource for FixedEntropy {
        fn fill(&mut self, destination: &mut [u8]) -> Result<(), OAuthError> {
            destination.fill(0x42);
            Ok(())
        }
    }

    #[derive(Default)]
    struct Audit(Vec<OAuthAuditEvent>);

    impl OAuthAuditSink for Audit {
        fn publish(&mut self, event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
            self.0.push(event.clone());
            Ok(())
        }
    }

    fn decode(body: &str) -> Result<BrokerProvider, BrokerError> {
        BrokerProvider::from_public_provider_data(
            Zeroizing::new(body.as_bytes().to_vec()),
            "deployment-client",
            "http://127.0.0.1:49152/oauth/callback",
        )
    }

    #[test]
    fn static_public_data_builds_the_generic_authorization_path() {
        let provider = decode(VALID).unwrap();
        assert_eq!(provider.provider().as_str(), "fixture-static");
        assert_eq!(provider.config().client_id(), "deployment-client");
        assert_eq!(provider.response_format(), TokenResponseFormat::Json);
        assert_eq!(provider.refresh_lead_seconds(), 300);
        let debug = format!("{provider:?}");
        assert!(!debug.contains("deployment-client"));
        assert!(!debug.contains("login.static.example"));

        let request = begin_authorization(
            provider.config(),
            &["files.read"],
            OAuthTraceId::new([0x31; 16]),
            &mut FixedEntropy,
        )
        .publish_then_release(&mut Audit::default())
        .unwrap();
        assert!(request
            .url()
            .as_str()
            .starts_with("https://login.static.example/authorize?"));
        assert!(request.url().as_str().contains("access_type=offline"));
        assert!(request.url().as_str().contains("prompt=consent"));
    }

    #[test]
    fn distinct_redirect_and_form_response_are_explicit_data() {
        let body = VALID
            .replace(
                r#""mix_up_defense":{
            "kind":"authorization_response_issuer",
            "issuer":"https://login.static.example"
        }"#,
                r#""mix_up_defense":{"kind":"distinct_redirect_uri"}"#,
            )
            .replace(
                r#""token_response_format":"json""#,
                r#""token_response_format":"form_encoded""#,
            );
        let provider = decode(&body).unwrap();
        assert_eq!(provider.response_format(), TokenResponseFormat::FormEncoded);
    }

    #[test]
    fn schema_shape_policy_and_core_configuration_fail_closed() {
        let cases = [
            VALID.replace(r#""schema_version":1"#, r#""schema_version":2"#),
            VALID.replace(r#""provider":"fixture-static""#, r#""unknown":true"#),
            VALID.replace(
                r#""token_response_format":"json""#,
                r#""token_response_format":"JSON""#,
            ),
            VALID.replace(
                r#""refresh_lead_seconds":300"#,
                r#""refresh_lead_seconds":86401"#,
            ),
            VALID.replace(
                "https://login.static.example/token",
                "http://login.static.example/token",
            ),
            VALID.replace(r#""access_type":"offline""#, r#""client_id":"override""#),
            VALID.replace(
                r#""kind":"authorization_response_issuer""#,
                r#""kind":"AuthorizationResponseIssuer""#,
            ),
        ];
        for body in cases {
            assert_eq!(decode(&body).unwrap_err(), BrokerError::InvalidProviderData);
        }
    }

    #[test]
    fn malformed_duplicate_unbounded_and_non_utf8_inputs_fail_closed() {
        let duplicate = VALID.replace(
            r#""schema_version":1"#,
            r#""schema_version":1,"schema_version":1"#,
        );
        assert_eq!(
            decode(&duplicate).unwrap_err(),
            BrokerError::InvalidProviderData
        );
        assert_eq!(decode("{").unwrap_err(), BrokerError::InvalidProviderData);
        assert_eq!(decode("").unwrap_err(), BrokerError::InvalidProviderData);
        assert_eq!(
            BrokerProvider::from_public_provider_data(
                Zeroizing::new(vec![0xff]),
                "deployment-client",
                "http://127.0.0.1:49152/oauth/callback",
            )
            .unwrap_err(),
            BrokerError::InvalidProviderData
        );
        assert_eq!(
            BrokerProvider::from_public_provider_data(
                Zeroizing::new(vec![b' '; MAX_PROVIDER_DATA_BYTES + 1]),
                "deployment-client",
                "http://127.0.0.1:49152/oauth/callback",
            )
            .unwrap_err(),
            BrokerError::InvalidProviderData
        );
    }

    #[test]
    fn deployment_client_data_is_never_read_from_the_profile() {
        let body = VALID.replace(
            r#""authorization_endpoint":"https://login.static.example/authorize""#,
            r#""client_id":"profile-client""#,
        );
        assert_eq!(decode(&body).unwrap_err(), BrokerError::InvalidProviderData);
    }

    #[test]
    fn public_provider_data_errors_are_text_free() {
        let error = decode("not-json").unwrap_err();
        assert_eq!(format!("{error:?}"), "InvalidProviderData");
        assert_eq!(error.to_string(), "oauth broker: InvalidProviderData");
    }
}
