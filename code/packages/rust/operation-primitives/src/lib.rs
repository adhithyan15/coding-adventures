#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use capability_cage::{
    Action, Capability, CapabilityViolationError, Category, InvalidCombination, Manifest,
    ManifestError,
};
use url_parser::Url;

const DEFAULT_HTTPS_PORT: u16 = 443;

/// Three-state result produced by an operation callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationResult<T> {
    pub did_succeed: bool,
    pub did_fail_unexpectedly: bool,
    pub return_value: T,
    pub error: Option<String>,
}

impl<T> OperationResult<T> {
    pub fn success(value: T) -> Self {
        Self {
            did_succeed: true,
            did_fail_unexpectedly: false,
            return_value: value,
            error: None,
        }
    }

    pub fn expected_failure(value: T, error: impl Into<String>) -> Self {
        Self {
            did_succeed: false,
            did_fail_unexpectedly: false,
            return_value: value,
            error: Some(error.into()),
        }
    }

    pub fn unexpected_failure(value: T, error: impl Into<String>) -> Self {
        Self {
            did_succeed: false,
            did_fail_unexpectedly: true,
            return_value: value,
            error: Some(error.into()),
        }
    }

    pub fn from_parts(
        did_succeed: bool,
        did_fail_unexpectedly: bool,
        value: T,
        error: Option<String>,
    ) -> Self {
        Self {
            did_succeed,
            did_fail_unexpectedly,
            return_value: value,
            error,
        }
    }
}

/// Creates [`OperationResult`] values inside callbacks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResultFactory<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> ResultFactory<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn generate(
        &self,
        did_succeed: bool,
        did_fail_unexpectedly: bool,
        value: T,
    ) -> OperationResult<T> {
        OperationResult::from_parts(did_succeed, did_fail_unexpectedly, value, None)
    }

    pub fn succeed(&self, value: T) -> OperationResult<T> {
        OperationResult::success(value)
    }

    pub fn fail(&self, value: T, error: impl Into<String>) -> OperationResult<T> {
        OperationResult::expected_failure(value, error)
    }

    pub fn fail_unexpectedly(&self, value: T, error: impl Into<String>) -> OperationResult<T> {
        OperationResult::unexpected_failure(value, error)
    }
}

/// Mutable callback context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationScope {
    name: String,
    property_bag: BTreeMap<String, String>,
}

impl OperationScope {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            property_bag: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_property(&mut self, name: impl Into<String>, value: impl ToString) {
        self.property_bag.insert(name.into(), value.to_string());
    }

    pub fn properties(&self) -> &BTreeMap<String, String> {
        &self.property_bag
    }
}

/// Error kind for operation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationErrorKind {
    Expected,
    Unexpected,
}

/// Error returned when an operation does not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationError {
    pub name: String,
    pub kind: OperationErrorKind,
    pub message: String,
    pub properties: BTreeMap<String, String>,
}

impl OperationError {
    pub fn is_expected(&self) -> bool {
        self.kind == OperationErrorKind::Expected
    }

    pub fn is_unexpected(&self) -> bool {
        self.kind == OperationErrorKind::Unexpected
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            OperationErrorKind::Expected => {
                write!(f, "operation {:?} failed: {}", self.name, self.message)
            }
            OperationErrorKind::Unexpected => write!(
                f,
                "operation {:?} failed unexpectedly: {}",
                self.name, self.message
            ),
        }
    }
}

impl std::error::Error for OperationError {}

/// Outcome preserving Go-style `(value, error)` semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationOutcome<T> {
    pub value: T,
    pub error: Option<OperationError>,
}

impl<T> OperationOutcome<T> {
    pub fn into_result(self) -> Result<T, OperationError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.value),
        }
    }
}

/// A named unit of work.
pub struct Operation<T, F>
where
    F: FnOnce(&mut OperationScope, &ResultFactory<T>) -> OperationResult<T>,
{
    name: String,
    fallback: T,
    callback: Option<F>,
    re_panic: bool,
}

impl<T, F> Operation<T, F>
where
    F: FnOnce(&mut OperationScope, &ResultFactory<T>) -> OperationResult<T>,
{
    pub fn panic_on_unexpected(mut self) -> Self {
        self.re_panic = true;
        self
    }

    pub fn get_outcome(mut self) -> OperationOutcome<T> {
        let name = self.name.clone();
        let mut scope = OperationScope::new(name.clone());
        let rf = ResultFactory::<T>::new();
        let callback = self
            .callback
            .take()
            .expect("operation callback should be present exactly once");

        let result = catch_unwind(AssertUnwindSafe(|| callback(&mut scope, &rf)));
        let properties = scope.property_bag;

        let operation_result = match result {
            Ok(operation_result) => operation_result,
            Err(panic_value) => {
                if self.re_panic {
                    resume_unwind(panic_value);
                }
                OperationResult::unexpected_failure(
                    self.fallback,
                    "callback panicked before producing an operation result",
                )
            }
        };

        if operation_result.did_succeed {
            return OperationOutcome {
                value: operation_result.return_value,
                error: None,
            };
        }

        let kind = if operation_result.did_fail_unexpectedly {
            OperationErrorKind::Unexpected
        } else {
            OperationErrorKind::Expected
        };
        let message = operation_result.error.unwrap_or_else(|| match kind {
            OperationErrorKind::Expected => "operation failed".to_string(),
            OperationErrorKind::Unexpected => "operation failed unexpectedly".to_string(),
        });

        OperationOutcome {
            value: operation_result.return_value,
            error: Some(OperationError {
                name,
                kind,
                message,
                properties,
            }),
        }
    }

    pub fn get_result(self) -> Result<T, OperationError> {
        self.get_outcome().into_result()
    }
}

/// Create an operation without executing it.
pub fn start_new<T, F>(name: impl Into<String>, fallback: T, callback: F) -> Operation<T, F>
where
    F: FnOnce(&mut OperationScope, &ResultFactory<T>) -> OperationResult<T>,
{
    Operation {
        name: name.into(),
        fallback,
        callback: Some(callback),
        re_panic: false,
    }
}

/// Error returned by the operation-side HTTP client before transport starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationHttpClientError {
    message: String,
}

impl OperationHttpClientError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for OperationHttpClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for OperationHttpClientError {}

impl From<ManifestError> for OperationHttpClientError {
    fn from(value: ManifestError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<InvalidCombination> for OperationHttpClientError {
    fn from(value: InvalidCombination) -> Self {
        Self::new(value.to_string())
    }
}

impl From<CapabilityViolationError> for OperationHttpClientError {
    fn from(value: CapabilityViolationError) -> Self {
        Self::new(value.to_string())
    }
}

/// A transport-ready request that has already passed manifest enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationHttpRequest {
    method: String,
    url: String,
    host: String,
    port: u16,
    authority: String,
    path_and_query: String,
    declared_domains: Vec<String>,
}

impl OperationHttpRequest {
    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }

    pub fn path_and_query(&self) -> &str {
        &self.path_and_query
    }

    pub fn declared_domains(&self) -> &[String] {
        &self.declared_domains
    }
}

/// Operation-side HTTP client generated from a package capability manifest.
///
/// Agent code should not construct raw network URLs past this boundary. The
/// generated operation client owns the manifest and refuses any request whose
/// HTTPS host/authority is not declared by `net:dns` and `net:connect` in the
/// package's `required_capabilities.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationHttpClient {
    manifest: Manifest,
    declared_domains: Vec<String>,
}

impl OperationHttpClient {
    pub fn from_required_capabilities_json(
        manifest_json: &str,
    ) -> Result<Self, OperationHttpClientError> {
        let manifest = Manifest::load_from_str(manifest_json)?;
        Self::from_manifest(manifest)
    }

    pub fn from_compiled_allowlist(
        dns_domains: &[&str],
        connect_authorities: &[&str],
    ) -> Result<Self, OperationHttpClientError> {
        let mut capabilities = Vec::new();
        for domain in dns_domains {
            capabilities.push(Capability::new(
                Category::Net,
                Action::Dns,
                normalize_exact_domain(domain)?,
                "generated operation HTTP client DNS allowlist",
            )?);
        }
        for authority in connect_authorities {
            let (host, port) = parse_connect_target(authority)?;
            capabilities.push(Capability::new(
                Category::Net,
                Action::Connect,
                format!("{host}:{port}"),
                "generated operation HTTP client connect allowlist",
            )?);
        }

        Self::from_manifest(Manifest::try_new(capabilities)?)
    }

    fn from_manifest(manifest: Manifest) -> Result<Self, OperationHttpClientError> {
        let declared_domains = declared_http_domains(&manifest)?;
        Ok(Self {
            manifest,
            declared_domains,
        })
    }

    pub fn declared_domains(&self) -> &[String] {
        &self.declared_domains
    }

    pub fn preflight_get(
        &self,
        url: &str,
    ) -> Result<OperationHttpRequest, OperationHttpClientError> {
        self.preflight("GET", url)
    }

    pub fn get_with_transport<T, E, F>(
        &self,
        url: &str,
        fallback: T,
        transport: F,
    ) -> OperationOutcome<T>
    where
        T: Clone,
        E: fmt::Display,
        F: FnOnce(OperationHttpRequest) -> Result<T, E>,
    {
        self.fetch_with_transport("GET", url, fallback, transport)
    }

    pub fn fetch_with_transport<T, E, F>(
        &self,
        method: &str,
        url: &str,
        fallback: T,
        transport: F,
    ) -> OperationOutcome<T>
    where
        T: Clone,
        E: fmt::Display,
        F: FnOnce(OperationHttpRequest) -> Result<T, E>,
    {
        start_new("host.network.fetch", fallback.clone(), |operation, rf| {
            operation.add_property("method", method);
            operation.add_property("url", url);

            let request = match self.preflight(method, url) {
                Ok(request) => request,
                Err(error) => return rf.fail(fallback.clone(), error.to_string()),
            };

            operation.add_property("host", request.host());
            operation.add_property("authority", request.authority());
            operation.add_property("path", request.path_and_query());
            operation.add_property("declared_domains", request.declared_domains().join(","));

            match transport(request) {
                Ok(value) => rf.succeed(value),
                Err(error) => rf.fail_unexpectedly(fallback, error.to_string()),
            }
        })
        .get_outcome()
    }

    fn preflight(
        &self,
        method: &str,
        url: &str,
    ) -> Result<OperationHttpRequest, OperationHttpClientError> {
        let parsed = Url::parse(url)
            .map_err(|error| OperationHttpClientError::new(format!("invalid URL: {error}")))?;
        if parsed.scheme != "https" {
            return Err(OperationHttpClientError::new(format!(
                "operation HTTP client only permits HTTPS URLs declared in required_capabilities.json, got {url}"
            )));
        }
        if parsed.userinfo.is_some() {
            return Err(OperationHttpClientError::new(format!(
                "operation HTTP client rejects userinfo in URL: {url}"
            )));
        }
        if parsed.fragment.is_some() {
            return Err(OperationHttpClientError::new(format!(
                "operation HTTP client rejects URL fragments before transport: {url}"
            )));
        }

        let host = parsed
            .host
            .as_deref()
            .ok_or_else(|| OperationHttpClientError::new(format!("HTTPS URL has no host: {url}")))
            .and_then(normalize_exact_domain)?;
        let port = parsed.port.unwrap_or(DEFAULT_HTTPS_PORT);
        if port == 0 {
            return Err(OperationHttpClientError::new(
                "operation HTTP client rejects port 0",
            ));
        }
        let authority = format!("{host}:{port}");
        self.manifest.check(Category::Net, Action::Dns, &host)?;
        self.manifest
            .check(Category::Net, Action::Connect, &authority)?;

        let path_and_query = match parsed.query {
            Some(query) => format!("{}?{query}", parsed.path),
            None => parsed.path,
        };

        Ok(OperationHttpRequest {
            method: method.to_string(),
            url: url.to_string(),
            host,
            port,
            authority,
            path_and_query,
            declared_domains: self.declared_domains.clone(),
        })
    }
}

fn declared_http_domains(manifest: &Manifest) -> Result<Vec<String>, OperationHttpClientError> {
    let mut declared_domains = Vec::new();
    for capability in manifest.capabilities() {
        if capability.category != Category::Net {
            continue;
        }
        match capability.action {
            Action::Dns => push_unique(
                &mut declared_domains,
                normalize_exact_domain(&capability.target)?,
            ),
            Action::Connect => {
                let endpoint = parse_connect_target(&capability.target)?;
                push_unique(&mut declared_domains, endpoint.0);
            }
            _ => {}
        }
    }

    if declared_domains.is_empty() {
        return Err(OperationHttpClientError::new(
            "operation HTTP client found no net:dns or net:connect entries in required_capabilities.json",
        ));
    }

    Ok(declared_domains)
}

fn parse_connect_target(target: &str) -> Result<(String, u16), OperationHttpClientError> {
    let (host, port_text) = target.rsplit_once(':').ok_or_else(|| {
        OperationHttpClientError::new(format!(
            "net:connect target must include host:port for operation HTTP clients, got {target}"
        ))
    })?;
    let host = normalize_exact_domain(host)?;
    let port = port_text.parse::<u16>().map_err(|error| {
        OperationHttpClientError::new(format!(
            "net:connect target had invalid port '{port_text}': {error}"
        ))
    })?;
    if port == 0 {
        return Err(OperationHttpClientError::new(
            "net:connect target used invalid port 0",
        ));
    }
    Ok((host, port))
}

fn normalize_exact_domain(domain: &str) -> Result<String, OperationHttpClientError> {
    let normalized = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(OperationHttpClientError::new(
            "operation HTTP domain target must not be empty",
        ));
    }
    if normalized.contains('*') {
        return Err(OperationHttpClientError::new(format!(
            "operation HTTP client requires exact domains; wildcard target '{domain}' is not allowed"
        )));
    }
    if normalized.starts_with('[')
        || normalized.contains(']')
        || normalized
            .chars()
            .any(|ch| ch.is_ascii_whitespace() || matches!(ch, '/' | '\\' | ':' | '@' | '?' | '#'))
    {
        return Err(OperationHttpClientError::new(format!(
            "operation HTTP domain target is not an exact DNS name: {domain}"
        )));
    }
    Ok(normalized)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weather_manifest_json() -> &'static str {
        r#"{
          "version": 1,
          "package": "rust/weather-agent-e2e",
          "capabilities": [
            {
              "category": "net",
              "action": "dns",
              "target": "api.weather.gov",
              "justification": "Resolve Weather.gov."
            },
            {
              "category": "net",
              "action": "connect",
              "target": "api.weather.gov:443",
              "justification": "Fetch Weather.gov over TLS."
            }
          ]
        }"#
    }

    #[test]
    fn successful_operation_returns_value() {
        let value = start_new("math.add", 0, |op, rf| {
            op.add_property("lhs", 2);
            op.add_property("rhs", 3);
            rf.succeed(5)
        })
        .get_result()
        .unwrap();

        assert_eq!(value, 5);
    }

    #[test]
    fn expected_failure_preserves_fallback_value_and_properties() {
        let outcome = start_new("fs.read", Vec::<u8>::new(), |op, rf| {
            op.add_property("path", "/etc/passwd");
            rf.fail(Vec::new(), "capability denied")
        })
        .get_outcome();

        assert_eq!(outcome.value, Vec::<u8>::new());
        let error = outcome.error.expect("expected failure should carry error");
        assert!(error.is_expected());
        assert_eq!(error.message, "capability denied");
        assert_eq!(
            error.properties.get("path").map(String::as_str),
            Some("/etc/passwd")
        );
    }

    #[test]
    fn unexpected_failure_can_be_returned_without_panic() {
        let error = start_new("planner.lower", "fallback".to_string(), |_op, rf| {
            rf.fail_unexpectedly("fallback".to_string(), "internal invariant broke")
        })
        .get_result()
        .unwrap_err();

        assert!(error.is_unexpected());
        assert_eq!(error.message, "internal invariant broke");
    }

    #[test]
    fn panic_becomes_unexpected_failure_by_default() {
        let outcome = start_new("panic.catcher", 42, |_op, _rf| -> OperationResult<i32> {
            panic!("boom")
        })
        .get_outcome();

        assert_eq!(outcome.value, 42);
        let error = outcome.error.expect("panic should become error");
        assert!(error.is_unexpected());
        assert!(error.message.contains("panicked"));
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn panic_on_unexpected_rethrows_callback_panic() {
        let _ = start_new("panic.rethrow", 0, |_op, _rf| -> OperationResult<i32> {
            panic!("boom")
        })
        .panic_on_unexpected()
        .get_result();
    }

    #[test]
    fn operation_http_client_generates_allowlist_from_required_capabilities() {
        let client =
            OperationHttpClient::from_required_capabilities_json(weather_manifest_json()).unwrap();

        let request = client
            .preflight_get("https://api.weather.gov/gridpoints/SEW/124,67/forecast?units=us")
            .unwrap();

        assert_eq!(request.method(), "GET");
        assert_eq!(request.host(), "api.weather.gov");
        assert_eq!(request.port(), 443);
        assert_eq!(request.authority(), "api.weather.gov:443");
        assert_eq!(
            request.path_and_query(),
            "/gridpoints/SEW/124,67/forecast?units=us"
        );
        assert_eq!(client.declared_domains(), &["api.weather.gov".to_string()]);
    }

    #[test]
    fn operation_http_client_blocks_undeclared_domains_before_transport() {
        let client =
            OperationHttpClient::from_required_capabilities_json(weather_manifest_json()).unwrap();
        let mut transport_called = false;
        let outcome = client.get_with_transport(
            "https://example.com/gridpoints/SEW/124,67/forecast",
            String::new(),
            |_request| -> Result<String, std::convert::Infallible> {
                transport_called = true;
                Ok("should not run".to_string())
            },
        );

        assert!(!transport_called);
        let error = outcome.error.expect("undeclared domain should fail");
        assert!(error.is_expected());
        assert_eq!(outcome.value, "");
        assert!(error.message.contains("net:dns:example.com"));
        assert!(error.message.contains("required_capabilities.json"));
        assert_eq!(
            error.properties.get("url").map(String::as_str),
            Some("https://example.com/gridpoints/SEW/124,67/forecast")
        );
    }

    #[test]
    fn operation_http_client_runs_transport_after_manifest_preflight() {
        let client =
            OperationHttpClient::from_required_capabilities_json(weather_manifest_json()).unwrap();

        let value = client
            .get_with_transport(
                "https://api.weather.gov/points/47.6062,-122.3321",
                String::new(),
                |request| -> Result<String, std::convert::Infallible> {
                    Ok(format!(
                        "{} {}",
                        request.authority(),
                        request.path_and_query()
                    ))
                },
            )
            .into_result()
            .unwrap();

        assert_eq!(value, "api.weather.gov:443 /points/47.6062,-122.3321");
    }

    #[test]
    fn operation_http_client_accepts_compiled_allowlist() {
        let client = OperationHttpClient::from_compiled_allowlist(
            &["api.weather.gov"],
            &["api.weather.gov:443"],
        )
        .unwrap();

        let request = client
            .preflight_get("https://api.weather.gov/points/47.6062,-122.3321")
            .unwrap();

        assert_eq!(request.host(), "api.weather.gov");
        assert_eq!(request.authority(), "api.weather.gov:443");
        assert_eq!(client.declared_domains(), &["api.weather.gov".to_string()]);
    }

    #[test]
    fn operation_http_client_rejects_wildcard_capabilities() {
        let json = r#"{
          "version": 1,
          "package": "rust/weather-agent-e2e",
          "capabilities": [
            {
              "category": "net",
              "action": "dns",
              "target": "*",
              "justification": "too broad"
            },
            {
              "category": "net",
              "action": "connect",
              "target": "*:443",
              "justification": "too broad"
            }
          ]
        }"#;

        let error = OperationHttpClient::from_required_capabilities_json(json).unwrap_err();
        assert!(error.to_string().contains("exact domains"));
    }
}
