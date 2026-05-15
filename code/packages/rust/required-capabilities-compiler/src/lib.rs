#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

use capability_cage::{Action, Category, Manifest, ManifestError};
use coding_adventures_json_value::{parse, JsonValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedOperationFile {
    pub package: String,
    pub rust_source: String,
    pub http_dns_domains: Vec<String>,
    pub http_connect_authorities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredCapabilitiesCompileError {
    message: String,
}

impl RequiredCapabilitiesCompileError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RequiredCapabilitiesCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for RequiredCapabilitiesCompileError {}

impl From<ManifestError> for RequiredCapabilitiesCompileError {
    fn from(value: ManifestError) -> Self {
        Self::new(value.to_string())
    }
}

pub fn compile_required_capabilities_json(
    manifest_json: &str,
) -> Result<GeneratedOperationFile, RequiredCapabilitiesCompileError> {
    let manifest = Manifest::load_from_str(manifest_json)?;
    let root = parse(manifest_json)
        .map_err(|error| RequiredCapabilitiesCompileError::new(error.to_string()))?;
    let package = package_name(&root)?;
    let http_dns_domains = collect_dns_domains(&manifest)?;
    let http_connect_authorities = collect_connect_authorities(&manifest)?;

    if http_dns_domains.is_empty() && http_connect_authorities.is_empty() {
        return Err(RequiredCapabilitiesCompileError::new(
            "required capabilities did not declare net:dns or net:connect entries for generated HTTP operations",
        ));
    }

    let rust_source =
        render_operation_source(&package, &http_dns_domains, &http_connect_authorities);
    Ok(GeneratedOperationFile {
        package,
        rust_source,
        http_dns_domains,
        http_connect_authorities,
    })
}

fn package_name(root: &JsonValue) -> Result<String, RequiredCapabilitiesCompileError> {
    let JsonValue::Object(pairs) = root else {
        return Err(RequiredCapabilitiesCompileError::new(
            "required capabilities root must be a JSON object",
        ));
    };

    pairs
        .iter()
        .find_map(|(key, value)| match (key.as_str(), value) {
            ("package", JsonValue::String(package)) => Some(package.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            RequiredCapabilitiesCompileError::new(
                "required capabilities JSON must include a string package field",
            )
        })
}

fn collect_dns_domains(
    manifest: &Manifest,
) -> Result<Vec<String>, RequiredCapabilitiesCompileError> {
    let mut domains = Vec::new();
    for capability in manifest.capabilities() {
        if capability.category == Category::Net && capability.action == Action::Dns {
            push_unique(&mut domains, normalize_exact_domain(&capability.target)?);
        }
    }
    Ok(domains)
}

fn collect_connect_authorities(
    manifest: &Manifest,
) -> Result<Vec<String>, RequiredCapabilitiesCompileError> {
    let mut authorities = Vec::new();
    for capability in manifest.capabilities() {
        if capability.category == Category::Net && capability.action == Action::Connect {
            let (host, port) = parse_connect_target(&capability.target)?;
            push_unique(&mut authorities, format!("{host}:{port}"));
        }
    }
    Ok(authorities)
}

fn parse_connect_target(target: &str) -> Result<(String, u16), RequiredCapabilitiesCompileError> {
    let (host, port_text) = target.rsplit_once(':').ok_or_else(|| {
        RequiredCapabilitiesCompileError::new(format!(
            "net:connect target must include host:port for generated HTTP operations, got {target}"
        ))
    })?;
    let host = normalize_exact_domain(host)?;
    let port = port_text.parse::<u16>().map_err(|error| {
        RequiredCapabilitiesCompileError::new(format!(
            "net:connect target had invalid port '{port_text}': {error}"
        ))
    })?;
    if port == 0 {
        return Err(RequiredCapabilitiesCompileError::new(
            "net:connect target used invalid port 0",
        ));
    }
    Ok((host, port))
}

fn normalize_exact_domain(domain: &str) -> Result<String, RequiredCapabilitiesCompileError> {
    let normalized = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(RequiredCapabilitiesCompileError::new(
            "generated HTTP operation domain target must not be empty",
        ));
    }
    if normalized.contains('*') {
        return Err(RequiredCapabilitiesCompileError::new(format!(
            "generated HTTP operation domains must be exact; wildcard target '{domain}' is not allowed"
        )));
    }
    if normalized.starts_with('[')
        || normalized.contains(']')
        || normalized
            .chars()
            .any(|ch| ch.is_ascii_whitespace() || matches!(ch, '/' | '\\' | ':' | '@' | '?' | '#'))
    {
        return Err(RequiredCapabilitiesCompileError::new(format!(
            "generated HTTP operation target is not an exact DNS name: {domain}"
        )));
    }
    Ok(normalized)
}

fn render_operation_source(
    package: &str,
    http_dns_domains: &[String],
    http_connect_authorities: &[String],
) -> String {
    let dns_domains = render_str_slice(http_dns_domains);
    let connect_authorities = render_str_slice(http_connect_authorities);
    let package_literal = rust_string_literal(package);

    format!(
        r#"// @generated by required-capabilities-compiler. Do not edit by hand.
#![allow(dead_code)]

pub const GENERATED_OPERATION_PACKAGE: &str = {package_literal};
pub const HTTP_DNS_DOMAINS: &[&str] = &[{dns_domains}];
pub const HTTP_CONNECT_AUTHORITIES: &[&str] = &[{connect_authorities}];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedOperationHttpClient {{
    inner: operation_primitives::OperationHttpClient,
}}

impl GeneratedOperationHttpClient {{
    pub fn new() -> Result<Self, operation_primitives::OperationHttpClientError> {{
        Ok(Self {{
            inner: operation_primitives::OperationHttpClient::from_compiled_allowlist(
                HTTP_DNS_DOMAINS,
                HTTP_CONNECT_AUTHORITIES,
            )?,
        }})
    }}

    pub fn declared_domains(&self) -> &[String] {{
        self.inner.declared_domains()
    }}

    pub fn preflight_get(
        &self,
        url: &str,
    ) -> Result<
        operation_primitives::OperationHttpRequest,
        operation_primitives::OperationHttpClientError,
    > {{
        self.inner.preflight_get(url)
    }}

    pub fn get_with_transport<T, E, F>(
        &self,
        url: &str,
        fallback: T,
        transport: F,
    ) -> operation_primitives::OperationOutcome<T>
    where
        T: Clone,
        E: std::fmt::Display,
        F: FnOnce(operation_primitives::OperationHttpRequest) -> Result<T, E>,
    {{
        self.inner.get_with_transport(url, fallback, transport)
    }}

    pub fn into_inner(self) -> operation_primitives::OperationHttpClient {{
        self.inner
    }}
}}

pub fn generated_http_client(
) -> Result<GeneratedOperationHttpClient, operation_primitives::OperationHttpClientError> {{
    GeneratedOperationHttpClient::new()
}}
"#
    )
}

fn render_str_slice(values: &[String]) -> String {
    values
        .iter()
        .map(|value| rust_string_literal(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rust_string_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            ch if ch.is_control() => literal.push_str(&format!("\\u{{{:x}}}", ch as u32)),
            ch => literal.push(ch),
        }
    }
    literal.push('"');
    literal
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
    fn compiles_http_allowlist_into_operation_source() {
        let generated = compile_required_capabilities_json(weather_manifest_json()).unwrap();

        assert_eq!(generated.package, "rust/weather-agent-e2e");
        assert_eq!(generated.http_dns_domains, vec!["api.weather.gov"]);
        assert_eq!(
            generated.http_connect_authorities,
            vec!["api.weather.gov:443"]
        );
        assert!(generated
            .rust_source
            .contains("pub const HTTP_DNS_DOMAINS: &[&str] = &[\"api.weather.gov\"];"));
        assert!(generated
            .rust_source
            .contains("GeneratedOperationHttpClient"));
        assert!(generated.rust_source.contains("from_compiled_allowlist"));
    }

    #[test]
    fn rejects_wildcard_http_capabilities() {
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

        let error = compile_required_capabilities_json(json).unwrap_err();
        assert!(error.to_string().contains("exact"));
    }

    #[test]
    fn normalizes_and_deduplicates_http_targets() {
        let json = r#"{
          "version": 1,
          "package": "rust/weather-agent-e2e",
          "capabilities": [
            {
              "category": "net",
              "action": "dns",
              "target": "API.WEATHER.GOV.",
              "justification": "Resolve Weather.gov."
            },
            {
              "category": "net",
              "action": "dns",
              "target": "api.weather.gov",
              "justification": "Resolve Weather.gov."
            },
            {
              "category": "net",
              "action": "connect",
              "target": "API.WEATHER.GOV.:443",
              "justification": "Fetch Weather.gov over TLS."
            }
          ]
        }"#;

        let generated = compile_required_capabilities_json(json).unwrap();

        assert_eq!(generated.http_dns_domains, vec!["api.weather.gov"]);
        assert_eq!(
            generated.http_connect_authorities,
            vec!["api.weather.gov:443"]
        );
    }
}
