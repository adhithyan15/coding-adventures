//! Pure local HTTP request planning primitives for smart-home integrations.
//!
//! This crate intentionally stops before doing I/O. It gives Hue, Shelly, WLED,
//! ESPHome, local cameras, and energy gateway workers a common way to describe
//! endpoints, auth material, and HTTP requests before a capability-caged runtime
//! decides whether and how to execute them.

#![forbid(unsafe_code)]

use http_core::{find_header, Header};
use smart_home_core::{BridgeId, IntegrationId, Metadata, VaultRef};
use std::fmt;

pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalHttpError {
    MissingHost,
    EmptyPath,
    AbsoluteUrlNotAllowed { path: String },
    BodyRequiresContentType,
    MissingHeaderName,
    DuplicateHeader { header_name: String },
    AuthHeaderConflict { header_name: String },
}

impl fmt::Display for LocalHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHost => write!(f, "local HTTP endpoint host must not be empty"),
            Self::EmptyPath => write!(f, "local HTTP request path must not be empty"),
            Self::AbsoluteUrlNotAllowed { path } => {
                write!(
                    f,
                    "local HTTP request path must not be an absolute URL: {path}"
                )
            }
            Self::BodyRequiresContentType => {
                write!(f, "body-bearing local HTTP requests require Content-Type")
            }
            Self::MissingHeaderName => write!(f, "local HTTP header name must not be empty"),
            Self::DuplicateHeader { header_name } => {
                write!(f, "duplicate local HTTP header: {header_name}")
            }
            Self::AuthHeaderConflict { header_name } => {
                write!(f, "auth would overwrite local HTTP header: {header_name}")
            }
        }
    }
}

impl std::error::Error for LocalHttpError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalHttpScheme {
    Http,
    Https,
}

impl LocalHttpScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalHttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl LocalHttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }

    pub fn is_idempotent_by_default(self) -> bool {
        matches!(self, Self::Get | Self::Put | Self::Delete)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalHttpAuth {
    None,
    BearerToken {
        vault_ref: VaultRef,
    },
    Basic {
        vault_ref: VaultRef,
    },
    HeaderToken {
        header_name: String,
        vault_ref: VaultRef,
    },
    ClientCertificate {
        vault_ref: VaultRef,
    },
}

impl LocalHttpAuth {
    pub fn required_vault_ref(&self) -> Option<&VaultRef> {
        match self {
            Self::None => None,
            Self::BearerToken { vault_ref }
            | Self::Basic { vault_ref }
            | Self::HeaderToken { vault_ref, .. }
            | Self::ClientCertificate { vault_ref } => Some(vault_ref),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalHttpEndpoint {
    pub integration_id: IntegrationId,
    pub bridge_id: BridgeId,
    pub scheme: LocalHttpScheme,
    pub host: String,
    pub port: Option<u16>,
    pub base_path: String,
    pub tls_name: Option<String>,
    pub accept_invalid_certs: bool,
    pub metadata: Vec<Metadata>,
}

impl LocalHttpEndpoint {
    pub fn new(
        integration_id: IntegrationId,
        bridge_id: BridgeId,
        scheme: LocalHttpScheme,
        host: impl Into<String>,
    ) -> Result<Self, LocalHttpError> {
        let host = host.into();
        if host.trim().is_empty() {
            return Err(LocalHttpError::MissingHost);
        }

        Ok(Self {
            integration_id,
            bridge_id,
            scheme,
            host,
            port: None,
            base_path: String::new(),
            tls_name: None,
            accept_invalid_certs: false,
            metadata: Vec::new(),
        })
    }

    pub fn hue_bridge(
        bridge_id: BridgeId,
        host: impl Into<String>,
    ) -> Result<Self, LocalHttpError> {
        Self::new(
            IntegrationId::trusted("hue"),
            bridge_id,
            LocalHttpScheme::Https,
            host,
        )
        .map(|endpoint| endpoint.with_metadata(Metadata::new("http.profile", "hue.clip.v2")))
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_base_path(mut self, base_path: impl Into<String>) -> Self {
        self.base_path = normalize_optional_base_path(&base_path.into());
        self
    }

    pub fn with_tls_name(mut self, tls_name: impl Into<String>) -> Self {
        self.tls_name = Some(tls_name.into());
        self
    }

    pub fn accept_invalid_certs(mut self, accept_invalid_certs: bool) -> Self {
        self.accept_invalid_certs = accept_invalid_certs;
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.push(metadata);
        self
    }

    pub fn origin(&self) -> String {
        match self.port {
            Some(port) if port != self.scheme.default_port() => {
                format!("{}://{}:{port}", self.scheme.as_str(), self.host)
            }
            _ => format!("{}://{}", self.scheme.as_str(), self.host),
        }
    }

    pub fn base_url(&self) -> String {
        let origin = self.origin();
        if self.base_path.is_empty() {
            origin
        } else {
            format!("{origin}{}", self.base_path)
        }
    }

    pub fn url_for_path(&self, path: &str) -> Result<String, LocalHttpError> {
        validate_relative_path(path)?;
        Ok(join_url_path(&self.base_url(), path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalHttpRequestTemplate {
    pub method: LocalHttpMethod,
    pub path: String,
    pub accept: Option<String>,
    pub content_type: Option<String>,
    pub timeout_ms: u64,
    pub idempotent: bool,
    pub auth: LocalHttpAuth,
    pub headers: Vec<Header>,
    pub metadata: Vec<Metadata>,
}

impl LocalHttpRequestTemplate {
    pub fn new(method: LocalHttpMethod, path: impl Into<String>) -> Result<Self, LocalHttpError> {
        let path = path.into();
        validate_relative_path(&path)?;
        Ok(Self {
            method,
            path,
            accept: None,
            content_type: None,
            timeout_ms: 5_000,
            idempotent: method.is_idempotent_by_default(),
            auth: LocalHttpAuth::None,
            headers: Vec::new(),
            metadata: Vec::new(),
        })
    }

    pub fn with_accept(mut self, accept: impl Into<String>) -> Self {
        self.accept = Some(accept.into());
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_idempotent(mut self, idempotent: bool) -> Self {
        self.idempotent = idempotent;
        self
    }

    pub fn with_auth(mut self, auth: LocalHttpAuth) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(Header {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.push(metadata);
        self
    }

    pub fn plan(
        &self,
        endpoint: &LocalHttpEndpoint,
        body: Vec<u8>,
    ) -> Result<LocalHttpRequestPlan, LocalHttpError> {
        let mut headers = Vec::new();

        if let Some(accept) = &self.accept {
            push_unique_header(&mut headers, "Accept", accept)?;
        }

        if let Some(content_type) = &self.content_type {
            push_unique_header(&mut headers, "Content-Type", content_type)?;
        }

        if !body.is_empty()
            && self.content_type.is_none()
            && find_header(&self.headers, "Content-Type").is_none()
        {
            return Err(LocalHttpError::BodyRequiresContentType);
        }

        for header in &self.headers {
            push_unique_header(&mut headers, &header.name, &header.value)?;
        }

        push_auth_headers(&mut headers, &self.auth)?;

        Ok(LocalHttpRequestPlan {
            integration_id: endpoint.integration_id.clone(),
            bridge_id: endpoint.bridge_id.clone(),
            method: self.method,
            url: endpoint.url_for_path(&self.path)?,
            headers,
            body,
            timeout_ms: self.timeout_ms,
            idempotent: self.idempotent,
            auth: self.auth.clone(),
            metadata: endpoint
                .metadata
                .iter()
                .chain(self.metadata.iter())
                .cloned()
                .collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalHttpRequestPlan {
    pub integration_id: IntegrationId,
    pub bridge_id: BridgeId,
    pub method: LocalHttpMethod,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
    pub timeout_ms: u64,
    pub idempotent: bool,
    pub auth: LocalHttpAuth,
    pub metadata: Vec<Metadata>,
}

impl LocalHttpRequestPlan {
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }

    pub fn required_vault_ref(&self) -> Option<&VaultRef> {
        self.auth.required_vault_ref()
    }

    pub fn has_body(&self) -> bool {
        !self.body.is_empty()
    }
}

fn validate_relative_path(path: &str) -> Result<(), LocalHttpError> {
    if path.trim().is_empty() {
        return Err(LocalHttpError::EmptyPath);
    }

    if path.starts_with("http://") || path.starts_with("https://") {
        return Err(LocalHttpError::AbsoluteUrlNotAllowed {
            path: path.to_string(),
        });
    }

    Ok(())
}

fn normalize_optional_base_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}

fn join_url_path(base_url: &str, path: &str) -> String {
    let trimmed_path = path.trim_start_matches('/');
    if trimmed_path.is_empty() {
        base_url.to_string()
    } else {
        format!("{}/{}", base_url.trim_end_matches('/'), trimmed_path)
    }
}

fn push_unique_header(
    headers: &mut Vec<Header>,
    name: &str,
    value: &str,
) -> Result<(), LocalHttpError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(LocalHttpError::MissingHeaderName);
    }

    if find_header(headers, name).is_some() {
        return Err(LocalHttpError::DuplicateHeader {
            header_name: name.to_string(),
        });
    }

    headers.push(Header {
        name: name.to_string(),
        value: value.to_string(),
    });
    Ok(())
}

fn push_auth_headers(
    headers: &mut Vec<Header>,
    auth: &LocalHttpAuth,
) -> Result<(), LocalHttpError> {
    match auth {
        LocalHttpAuth::None | LocalHttpAuth::ClientCertificate { .. } => Ok(()),
        LocalHttpAuth::BearerToken { vault_ref } => push_auth_header(
            headers,
            "Authorization",
            &format!("Bearer {}", vault(vault_ref)),
        ),
        LocalHttpAuth::Basic { vault_ref } => push_auth_header(
            headers,
            "Authorization",
            &format!("Basic {}", vault(vault_ref)),
        ),
        LocalHttpAuth::HeaderToken {
            header_name,
            vault_ref,
        } => push_auth_header(headers, header_name, &vault(vault_ref)),
    }
}

fn push_auth_header(
    headers: &mut Vec<Header>,
    name: &str,
    value: &str,
) -> Result<(), LocalHttpError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(LocalHttpError::MissingHeaderName);
    }

    if find_header(headers, name).is_some() {
        return Err(LocalHttpError::AuthHeaderConflict {
            header_name: name.to_string(),
        });
    }

    headers.push(Header {
        name: name.to_string(),
        value: value.to_string(),
    });
    Ok(())
}

fn vault(vault_ref: &VaultRef) -> String {
    format!("<vault:{}>", vault_ref.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge_id() -> BridgeId {
        BridgeId::trusted("bridge-1")
    }

    #[test]
    fn hue_endpoint_builds_clip_v2_resource_urls() {
        let endpoint = LocalHttpEndpoint::hue_bridge(bridge_id(), "192.168.1.2").unwrap();
        let plan = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, "/clip/v2/resource/light")
            .unwrap()
            .plan(&endpoint, Vec::new())
            .unwrap();

        assert_eq!(endpoint.origin(), "https://192.168.1.2");
        assert_eq!(plan.method.as_str(), "GET");
        assert_eq!(plan.url, "https://192.168.1.2/clip/v2/resource/light");
        assert_eq!(
            plan.metadata,
            vec![Metadata::new("http.profile", "hue.clip.v2")]
        );
    }

    #[test]
    fn request_templates_plan_headers_and_vault_backed_auth() {
        let endpoint = LocalHttpEndpoint::hue_bridge(bridge_id(), "hue.local").unwrap();
        let vault_ref = VaultRef::trusted("vault://hue/app-key");
        let body = br#"{"on":{"on":true}}"#.to_vec();

        let plan = LocalHttpRequestTemplate::new(LocalHttpMethod::Put, "/clip/v2/resource/light/1")
            .unwrap()
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_auth(LocalHttpAuth::HeaderToken {
                header_name: "hue-application-key".into(),
                vault_ref: vault_ref.clone(),
            })
            .with_metadata(Metadata::new("operation", "set-light-on"))
            .plan(&endpoint, body)
            .unwrap();

        assert_eq!(plan.header("accept"), Some("application/json"));
        assert_eq!(plan.header("content-type"), Some("application/json"));
        assert_eq!(
            plan.header("hue-application-key"),
            Some("<vault:vault://hue/app-key>")
        );
        assert_eq!(plan.required_vault_ref(), Some(&vault_ref));
        assert!(plan.idempotent);
        assert!(plan.has_body());
        assert!(plan
            .metadata
            .contains(&Metadata::new("operation", "set-light-on")));
    }

    #[test]
    fn request_templates_reject_body_without_content_type() {
        let endpoint = LocalHttpEndpoint::hue_bridge(bridge_id(), "hue.local").unwrap();
        let err = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, "/clip/v2/resource/scene")
            .unwrap()
            .plan(&endpoint, vec![b'{'])
            .unwrap_err();

        assert_eq!(err, LocalHttpError::BodyRequiresContentType);
    }

    #[test]
    fn bearer_auth_rejects_conflicting_authorization_header() {
        let endpoint = LocalHttpEndpoint::hue_bridge(bridge_id(), "hue.local").unwrap();
        let err = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, "/clip/v2/resource/light")
            .unwrap()
            .with_header("Authorization", "manual")
            .with_auth(LocalHttpAuth::BearerToken {
                vault_ref: VaultRef::trusted("vault://token"),
            })
            .plan(&endpoint, Vec::new())
            .unwrap_err();

        assert_eq!(
            err,
            LocalHttpError::AuthHeaderConflict {
                header_name: "Authorization".into()
            }
        );
    }

    #[test]
    fn endpoint_normalizes_base_and_relative_paths() {
        let endpoint = LocalHttpEndpoint::new(
            IntegrationId::trusted("wled"),
            bridge_id(),
            LocalHttpScheme::Http,
            "wled.local",
        )
        .unwrap()
        .with_port(8080)
        .with_base_path("/json/");

        assert_eq!(endpoint.base_url(), "http://wled.local:8080/json");
        assert_eq!(
            endpoint.url_for_path("state?live=true").unwrap(),
            "http://wled.local:8080/json/state?live=true"
        );
    }

    #[test]
    fn methods_record_idempotency_defaults() {
        let post = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, "/state").unwrap();
        let put = LocalHttpRequestTemplate::new(LocalHttpMethod::Put, "/state").unwrap();

        assert!(!post.idempotent);
        assert!(put.idempotent);
        assert_eq!(LocalHttpMethod::Patch.as_str(), "PATCH");
    }

    #[test]
    fn tls_name_and_invalid_cert_policy_are_explicit() {
        let endpoint = LocalHttpEndpoint::new(
            IntegrationId::trusted("camera"),
            bridge_id(),
            LocalHttpScheme::Https,
            "192.168.1.40",
        )
        .unwrap()
        .with_tls_name("camera.local")
        .accept_invalid_certs(true);

        assert_eq!(endpoint.tls_name.as_deref(), Some("camera.local"));
        assert!(endpoint.accept_invalid_certs);
    }
}
