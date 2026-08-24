//! Authorized bounded IPP Everywhere mDNS discovery for D23.

#![forbid(unsafe_code)]

use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use ipp_protocol::{
    decode_get_printer_attributes_response, encode_get_printer_attributes, IppProtocolError,
    PrinterAttributes, PrinterState, MAX_MESSAGE_BYTES,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind,
};
use smart_home_discovery::{
    run_mdns_ipv4_scan, DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource,
    DiscoveryUpsert, MdnsAdvertisement, MdnsScanOptions, MdnsScanResult, PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::Duration;

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "ipp_printer";
pub const PROTOCOL_ID: &str = "ipp_everywhere";
pub const MDNS_SERVICE_TYPE: &str = "_ipp._tcp.local";
pub const MAX_RESPONSES: usize = 64;
const MAX_TXT_VALUE_BYTES: usize = 255;
pub const DEFAULT_MAX_STATUS_RESPONSE_BYTES: usize = MAX_MESSAGE_BYTES + 16 * 1024;
pub const MAX_STATUS_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum IppPrinterDiscoveryError {
    Validation(String),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for IppPrinterDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(formatter, "invalid IPP printer discovery: {message}")
            }
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for IppPrinterDiscoveryError {}

impl From<DiscoveryError> for IppPrinterDiscoveryError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for IppPrinterDiscoveryError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IppPrinterDiscoveryConfig {
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl Default for IppPrinterDiscoveryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }
}

impl IppPrinterDiscoveryConfig {
    pub fn validate(&self) -> Result<(), IppPrinterDiscoveryError> {
        if self.timeout.is_zero() {
            return Err(IppPrinterDiscoveryError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(IppPrinterDiscoveryError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(IppPrinterDiscoveryError::Validation(
                "record TTL must be non-zero".to_string(),
            ));
        }
        Ok(())
    }

    fn scan_options(&self, discovered_at_ms: u64) -> Result<MdnsScanOptions, DiscoveryError> {
        Ok(
            MdnsScanOptions::new(MDNS_SERVICE_TYPE, discovered_at_ms, self.timeout)?
                .with_max_responses(self.maximum_responses),
        )
    }
}

pub trait IppPrinterMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, IppPrinterDiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpIppPrinterMdnsTransport;

impl IppPrinterMdnsTransport for UdpIppPrinterMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, IppPrinterDiscoveryError> {
        Ok(run_mdns_ipv4_scan(options)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IppPrinterDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IppPrinterRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: IppPrinterMdnsTransport>(
    config: &IppPrinterDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<IppPrinterDiscoveryReport, IppPrinterDiscoveryError> {
    config.validate()?;
    let result = transport.scan(config.scan_options(discovered_at_ms)?)?;
    if normalized_service_type(&result.service_type) != normalized_service_type(MDNS_SERVICE_TYPE) {
        return Err(IppPrinterDiscoveryError::Validation(format!(
            "scanner returned unexpected service type `{}`",
            result.service_type
        )));
    }

    let mut failures = result
        .failures
        .into_iter()
        .map(|failure| match failure.source {
            Some(source) => format!("invalid mDNS reply from {source}: {}", failure.message),
            None => format!("invalid mDNS reply: {}", failure.message),
        })
        .collect::<Vec<_>>();
    let ttl_ms = u64::try_from(config.record_ttl.as_millis()).unwrap_or(u64::MAX);
    let expires_at_ms = discovered_at_ms.saturating_add(ttl_ms);
    let mut records = BTreeMap::<String, DiscoveryRecord>::new();

    for advertisement in result.advertisements {
        match discovery_record(&advertisement, expires_at_ms) {
            Ok(record) => match records.get(&record.native_bridge_id) {
                Some(existing) if existing.address != record.address => failures.push(format!(
                    "IPP printer {} advertises conflicting endpoints `{}` and `{}`",
                    record.native_bridge_id,
                    existing.address.as_deref().unwrap_or(""),
                    record.address.as_deref().unwrap_or("")
                )),
                Some(_) => {}
                None => {
                    records.insert(record.native_bridge_id.clone(), record);
                }
            },
            Err(error) => failures.push(format!(
                "invalid IPP printer advertisement `{}`: {error}",
                advertisement.instance_name
            )),
        }
    }

    Ok(IppPrinterDiscoveryReport {
        records: records.into_values().collect(),
        failures,
    })
}

pub fn discover_into_runtime<T: IppPrinterMdnsTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &IppPrinterDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<IppPrinterRuntimeCommitSummary, IppPrinterDiscoveryError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(IppPrinterDiscoveryError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = IppPrinterRuntimeCommitSummary {
        failures: report.failures.len(),
        ..IppPrinterRuntimeCommitSummary::default()
    };
    for record in report.records {
        match runtime.record_discovery(record)? {
            DiscoveryUpsert::Inserted => summary.inserted += 1,
            DiscoveryUpsert::Replaced(_) => summary.replaced += 1,
            DiscoveryUpsert::Ignored(_) => summary.ignored += 1,
        }
    }
    Ok(summary)
}

pub fn discovery_record(
    advertisement: &MdnsAdvertisement,
    expires_at_ms: u64,
) -> Result<DiscoveryRecord, IppPrinterDiscoveryError> {
    if normalized_service_type(&advertisement.service_type)
        != normalized_service_type(MDNS_SERVICE_TYPE)
    {
        return Err(IppPrinterDiscoveryError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    if advertisement.port == 0 {
        return Err(IppPrinterDiscoveryError::Validation(
            "IPP port must be non-zero".to_string(),
        ));
    }
    if advertisement.addresses.is_empty() {
        return Err(IppPrinterDiscoveryError::Validation(
            "IPP advertisement must resolve at least one IP address".to_string(),
        ));
    }
    validate_unique_txt_keys(advertisement)?;

    let display_name = safe_text(&advertisement.instance_name, "instance name", 256)?;
    let resource_path = validate_resource_path(required_txt(advertisement, "rp")?)?;
    let txt_version = optional_txt(advertisement, "txtvers").unwrap_or("1");
    if txt_version != "1" {
        return Err(IppPrinterDiscoveryError::Validation(
            "txtvers must be 1".to_string(),
        ));
    }
    let printer_uuid = optional_txt(advertisement, "UUID")
        .filter(|value| !value.is_empty())
        .map(validate_uuid)
        .transpose()?;
    let authentication =
        parse_authentication(optional_txt(advertisement, "air").unwrap_or("none"))?;
    let pairing_requirement = authentication.pairing_requirement();
    let hardware_model = optional_txt(advertisement, "ty")
        .filter(|value| !value.is_empty())
        .map(|value| safe_text(value, "ty", 255))
        .transpose()?;
    let location = optional_txt(advertisement, "note")
        .filter(|value| !value.is_empty())
        .map(|value| safe_text(value, "note", 255))
        .transpose()?;
    let tls = optional_txt(advertisement, "TLS")
        .filter(|value| !value.is_empty())
        .map(validate_tls_version)
        .transpose()?;
    let document_formats = optional_txt(advertisement, "pdl")
        .filter(|value| !value.is_empty())
        .map(validate_document_formats)
        .transpose()?;
    let color = optional_txt(advertisement, "Color")
        .map(|value| parse_tristate(value, "Color"))
        .transpose()?;
    let duplex = optional_txt(advertisement, "Duplex")
        .map(|value| parse_tristate(value, "Duplex"))
        .transpose()?;
    let native_bridge_id = printer_uuid.as_ref().map_or_else(
        || fallback_identity(advertisement, &resource_path),
        |uuid| format!("printer-{uuid}"),
    );
    let address = format!(
        "ipp://{}:{}/{}",
        advertisement.preferred_address(),
        advertisement.port,
        resource_path
    );

    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        native_bridge_id,
        DiscoverySource::Mdns,
        BridgeTransport::LanTcp,
        advertisement.discovered_at_ms,
    )?
    .with_display_name(display_name)
    .with_address(address)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(pairing_requirement)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE)
    .with_metadata("ipp.resource_path", &resource_path)
    .with_metadata("ipp.txt_version", txt_version)
    .with_metadata("ipp.authentication", authentication.as_str());
    if let Some(hardware_model) = hardware_model {
        record = record.with_hardware_model(hardware_model);
    }
    if let Some(printer_uuid) = printer_uuid {
        record = record.with_metadata("ipp.printer_uuid", printer_uuid);
    }
    if let Some(location) = location {
        record = record.with_metadata("ipp.location", location);
    }
    if let Some(tls) = tls {
        record = record.with_metadata("ipp.maximum_tls_version", tls);
    }
    if let Some(document_formats) = document_formats {
        record = record.with_metadata("ipp.document_formats", document_formats);
    }
    if let Some(color) = color {
        record = record.with_metadata("ipp.color_supported", color);
    }
    if let Some(duplex) = duplex {
        record = record.with_metadata("ipp.duplex_supported", duplex);
    }
    Ok(record)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IppAuthentication {
    None,
    Certificate,
    Negotiate,
    OAuth2,
    UsernamePassword,
}

impl IppAuthentication {
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Certificate => "certificate",
            Self::Negotiate => "negotiate",
            Self::OAuth2 => "oauth",
            Self::UsernamePassword => "username,password",
        }
    }

    const fn pairing_requirement(self) -> PairingRequirement {
        match self {
            Self::None => PairingRequirement::None,
            Self::Certificate => PairingRequirement::Certificate,
            Self::Negotiate | Self::UsernamePassword => PairingRequirement::Credentials,
            Self::OAuth2 => PairingRequirement::OAuth2,
        }
    }
}

fn parse_authentication(value: &str) -> Result<IppAuthentication, IppPrinterDiscoveryError> {
    match value {
        "none" => Ok(IppAuthentication::None),
        "certificate" => Ok(IppAuthentication::Certificate),
        "negotiate" => Ok(IppAuthentication::Negotiate),
        "oauth" => Ok(IppAuthentication::OAuth2),
        "username,password" => Ok(IppAuthentication::UsernamePassword),
        _ => Err(IppPrinterDiscoveryError::Validation(
            "air contains an unsupported authentication requirement".to_string(),
        )),
    }
}

fn required_txt<'a>(
    advertisement: &'a MdnsAdvertisement,
    key: &str,
) -> Result<&'a str, IppPrinterDiscoveryError> {
    optional_txt(advertisement, key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| IppPrinterDiscoveryError::Validation(format!("missing {key} TXT record")))
}

fn optional_txt<'a>(advertisement: &'a MdnsAdvertisement, key: &str) -> Option<&'a str> {
    advertisement
        .txt
        .iter()
        .find(|entry| entry.key.eq_ignore_ascii_case(key))
        .map(|entry| entry.value.as_str())
}

fn validate_unique_txt_keys(
    advertisement: &MdnsAdvertisement,
) -> Result<(), IppPrinterDiscoveryError> {
    let mut keys = BTreeSet::new();
    for entry in &advertisement.txt {
        let normalized = entry.key.to_ascii_lowercase();
        if !keys.insert(normalized) {
            return Err(IppPrinterDiscoveryError::Validation(format!(
                "duplicate case-insensitive TXT key `{}`",
                entry.key
            )));
        }
    }
    Ok(())
}

fn safe_text(
    value: &str,
    field: &str,
    maximum_bytes: usize,
) -> Result<String, IppPrinterDiscoveryError> {
    if value.is_empty()
        || value.len() > maximum_bytes.min(MAX_TXT_VALUE_BYTES)
        || value.chars().any(char::is_control)
    {
        return Err(IppPrinterDiscoveryError::Validation(format!(
            "{field} must be non-empty bounded text without control characters"
        )));
    }
    Ok(value.to_string())
}

fn validate_resource_path(value: &str) -> Result<String, IppPrinterDiscoveryError> {
    if value.is_empty() || value.len() > 255 || value.starts_with('/') || !value.is_ascii() {
        return Err(IppPrinterDiscoveryError::Validation(
            "rp must be a non-empty bounded relative URI path without a leading slash".to_string(),
        ));
    }
    if value
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(IppPrinterDiscoveryError::Validation(
            "rp must not contain dot path segments".to_string(),
        ));
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(IppPrinterDiscoveryError::Validation(
                    "rp contains an invalid percent escape".to_string(),
                ));
            }
            index += 3;
            continue;
        }
        if !(byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
                    | b'/'
            ))
        {
            return Err(IppPrinterDiscoveryError::Validation(
                "rp contains a character outside the URI path grammar".to_string(),
            ));
        }
        index += 1;
    }
    Ok(value.to_string())
}

fn validate_uuid(value: &str) -> Result<String, IppPrinterDiscoveryError> {
    if value.len() != 36 {
        return Err(IppPrinterDiscoveryError::Validation(
            "UUID must be a canonical 36-character UUID".to_string(),
        ));
    }
    let mut normalized = String::with_capacity(36);
    for (index, byte) in value.bytes().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return Err(IppPrinterDiscoveryError::Validation(
                    "UUID must be a canonical hyphenated UUID".to_string(),
                ));
            }
            normalized.push('-');
        } else if byte.is_ascii_hexdigit() {
            normalized.push((byte as char).to_ascii_lowercase());
        } else {
            return Err(IppPrinterDiscoveryError::Validation(
                "UUID must contain only hexadecimal digits and canonical hyphens".to_string(),
            ));
        }
    }
    Ok(normalized)
}

fn validate_document_formats(value: &str) -> Result<String, IppPrinterDiscoveryError> {
    if value.len() > MAX_TXT_VALUE_BYTES {
        return Err(IppPrinterDiscoveryError::Validation(
            "pdl exceeds the bounded TXT value limit".to_string(),
        ));
    }
    let formats = value.split(',').collect::<Vec<_>>();
    if formats.is_empty()
        || formats.iter().any(|format| {
            let Some((kind, subtype)) = format.split_once('/') else {
                return true;
            };
            kind.is_empty()
                || subtype.is_empty()
                || subtype.contains('/')
                || !format.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(
                            byte,
                            b'!' | b'#' | b'$' | b'&' | b'-' | b'^' | b'_' | b'.' | b'+' | b'/'
                        )
                })
        })
    {
        return Err(IppPrinterDiscoveryError::Validation(
            "pdl must contain a bounded comma-separated MIME type list".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn validate_tls_version(value: &str) -> Result<String, IppPrinterDiscoveryError> {
    if value == "none" {
        return Ok(value.to_string());
    }
    let Some((major, minor)) = value.split_once('.') else {
        return Err(IppPrinterDiscoveryError::Validation(
            "TLS must be none or a bounded major.minor version".to_string(),
        ));
    };
    if major.is_empty()
        || minor.is_empty()
        || major.len() > 2
        || minor.len() > 2
        || !major.bytes().all(|byte| byte.is_ascii_digit())
        || !minor.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(IppPrinterDiscoveryError::Validation(
            "TLS must be none or a bounded major.minor version".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn parse_tristate(value: &str, field: &str) -> Result<&'static str, IppPrinterDiscoveryError> {
    match value {
        "T" => Ok("true"),
        "F" => Ok("false"),
        "U" => Ok("unknown"),
        _ => Err(IppPrinterDiscoveryError::Validation(format!(
            "{field} must be T, F, or U"
        ))),
    }
}

fn fallback_identity(advertisement: &MdnsAdvertisement, resource_path: &str) -> String {
    let value = format!(
        "{}-{}-{}",
        advertisement.host_name, advertisement.port, resource_path
    );
    let mut identity = String::from("printer-endpoint-");
    let mut previous_hyphen = false;
    for character in value.chars().take(180) {
        if character.is_ascii_alphanumeric() {
            identity.push(character.to_ascii_lowercase());
            previous_hyphen = false;
        } else if !previous_hyphen {
            identity.push('-');
            previous_hyphen = true;
        }
    }
    while identity.ends_with('-') {
        identity.pop();
    }
    identity
}

fn normalized_service_type(value: &str) -> &str {
    value.trim_end_matches('.')
}

#[derive(Debug)]
pub enum IppPrinterStatusError {
    Validation(String),
    Io(String),
    Http(String),
    HttpStatus(u16),
    AuthenticationRequired,
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    Protocol(IppProtocolError),
    Runtime(RuntimeError),
}

impl fmt::Display for IppPrinterStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(formatter, "invalid IPP printer status input: {message}")
            }
            Self::Io(message) => write!(formatter, "IPP printer I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid IPP HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "IPP endpoint returned HTTP {status}"),
            Self::AuthenticationRequired => {
                formatter.write_str("IPP endpoint requires unsupported authentication")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "IPP response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "IPP response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for IppPrinterStatusError {}

impl From<IppProtocolError> for IppPrinterStatusError {
    fn from(error: IppProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<RuntimeError> for IppPrinterStatusError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IppPrinterStatusConfig {
    pub bridge_id: BridgeId,
    pub endpoint: SocketAddrV4,
    pub resource_path: String,
    pub timeout: Duration,
    pub maximum_response_bytes: usize,
}

impl IppPrinterStatusConfig {
    pub fn new(
        bridge_id: BridgeId,
        endpoint: SocketAddrV4,
        resource_path: impl Into<String>,
    ) -> Result<Self, IppPrinterStatusError> {
        let config = Self {
            bridge_id,
            endpoint,
            resource_path: resource_path.into(),
            timeout: Duration::from_secs(5),
            maximum_response_bytes: DEFAULT_MAX_STATUS_RESPONSE_BYTES,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), IppPrinterStatusError> {
        let bridge_id = self.bridge_id.as_str();
        let Some(native_id) = bridge_id.strip_prefix("ipp_printer.bridge.printer-") else {
            return Err(IppPrinterStatusError::Validation(
                "bridge id must retain the discovered ipp_printer printer identity".to_string(),
            ));
        };
        if native_id.is_empty()
            || native_id.len() > 220
            || !native_id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(IppPrinterStatusError::Validation(
                "bridge id must contain a bounded canonical printer identity".to_string(),
            ));
        }
        if self.endpoint.port() == 0 {
            return Err(IppPrinterStatusError::Validation(
                "endpoint port must be non-zero".to_string(),
            ));
        }
        if !is_local_ipv4(*self.endpoint.ip()) {
            return Err(IppPrinterStatusError::Validation(
                "endpoint must be a private, link-local, or loopback IPv4 literal".to_string(),
            ));
        }
        validate_resource_path(&self.resource_path)
            .map_err(|error| IppPrinterStatusError::Validation(error.to_string()))?;
        if self.timeout.is_zero() || self.timeout > MAX_STATUS_TIMEOUT {
            return Err(IppPrinterStatusError::Validation(
                "timeout must be non-zero and at most five seconds".to_string(),
            ));
        }
        if !(1024..=DEFAULT_MAX_STATUS_RESPONSE_BYTES).contains(&self.maximum_response_bytes) {
            return Err(IppPrinterStatusError::Validation(format!(
                "maximum response bytes must be between 1024 and {DEFAULT_MAX_STATUS_RESPONSE_BYTES}"
            )));
        }
        Ok(())
    }

    pub fn printer_uri(&self) -> String {
        format!("ipp://{}/{path}", self.endpoint, path = self.resource_path)
    }

    pub fn endpoint_url(&self) -> String {
        format!("http://{}/{path}", self.endpoint, path = self.resource_path)
    }
}

pub trait IppPrinterStatusTransport {
    fn post(
        &mut self,
        config: &IppPrinterStatusConfig,
        body: &[u8],
    ) -> Result<Vec<u8>, IppPrinterStatusError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IppPrinterLanTransport;

impl IppPrinterStatusTransport for IppPrinterLanTransport {
    fn post(
        &mut self,
        config: &IppPrinterStatusConfig,
        body: &[u8],
    ) -> Result<Vec<u8>, IppPrinterStatusError> {
        config.validate()?;
        if body.len() > MAX_MESSAGE_BYTES {
            return Err(IppPrinterStatusError::Validation(
                "IPP request exceeds the protocol message limit".to_string(),
            ));
        }
        let request = encode_status_http_request(config, body)?;
        let endpoint = std::net::SocketAddr::V4(config.endpoint);
        let mut stream = TcpStream::connect_timeout(&endpoint, config.timeout)
            .map_err(|error| IppPrinterStatusError::Io(error.to_string()))?;
        stream
            .set_read_timeout(Some(config.timeout))
            .map_err(|error| IppPrinterStatusError::Io(error.to_string()))?;
        stream
            .set_write_timeout(Some(config.timeout))
            .map_err(|error| IppPrinterStatusError::Io(error.to_string()))?;
        stream
            .write_all(&request)
            .map_err(|error| IppPrinterStatusError::Io(error.to_string()))?;
        stream
            .flush()
            .map_err(|error| IppPrinterStatusError::Io(error.to_string()))?;
        let response = read_status_http_response(&mut stream, config.maximum_response_bytes)?;
        decode_status_http_response(&response, config.maximum_response_bytes)
    }
}

fn encode_status_http_request(
    config: &IppPrinterStatusConfig,
    body: &[u8],
) -> Result<Vec<u8>, IppPrinterStatusError> {
    let target = format!("/{}", config.resource_path);
    let host = config.endpoint.to_string();
    if target.contains(['\r', '\n', '\0']) || host.contains(['\r', '\n', '\0']) {
        return Err(IppPrinterStatusError::Validation(
            "unsafe HTTP request text".to_string(),
        ));
    }
    let head = format!(
        "POST {target} HTTP/1.1\r\nHost: {host}\r\nAccept: application/ipp\r\nContent-Type: application/ipp\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    if head.len().saturating_add(body.len()) > MAX_MESSAGE_BYTES + 4096 {
        return Err(IppPrinterStatusError::Validation(
            "IPP HTTP request exceeds the fixed request limit".to_string(),
        ));
    }
    let mut request = head.into_bytes();
    request.extend_from_slice(body);
    Ok(request)
}

fn read_status_http_response(
    reader: &mut dyn Read,
    maximum: usize,
) -> Result<Vec<u8>, IppPrinterStatusError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| IppPrinterStatusError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(IppPrinterStatusError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_status_http_response(
    bytes: &[u8],
    maximum: usize,
) -> Result<Vec<u8>, IppPrinterStatusError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| IppPrinterStatusError::Http(error.to_string()))?;
    match parsed.head.status {
        401 | 403 => return Err(IppPrinterStatusError::AuthenticationRequired),
        status if !(200..300).contains(&status) => {
            return Err(IppPrinterStatusError::HttpStatus(status));
        }
        _ => {}
    }
    let content_type = parsed
        .head
        .header("Content-Type")
        .ok_or_else(|| IppPrinterStatusError::Http("missing Content-Type".to_string()))?;
    if !content_type.trim().eq_ignore_ascii_case("application/ipp") {
        return Err(IppPrinterStatusError::Http(
            "Content-Type must be application/ipp".to_string(),
        ));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(IppPrinterStatusError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            if input.len() != expected {
                return Err(IppPrinterStatusError::Http(
                    "IPP response contains bytes after Content-Length".to_string(),
                ));
            }
            input.to_vec()
        }
        BodyKind::None => {
            return Err(IppPrinterStatusError::Http(
                "IPP response body is required".to_string(),
            ));
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => {
            return Err(IppPrinterStatusError::Http(
                "chunked IPP responses are unsupported".to_string(),
            ));
        }
    };
    if body.len() > MAX_MESSAGE_BYTES || bytes.len() > maximum {
        return Err(IppPrinterStatusError::ResponseTooLarge {
            limit: maximum.min(MAX_MESSAGE_BYTES),
        });
    }
    Ok(body)
}

pub struct IppPrinterStatusClient<T> {
    config: IppPrinterStatusConfig,
    transport: T,
    next_request_id: u32,
}

impl<T: IppPrinterStatusTransport> IppPrinterStatusClient<T> {
    pub fn new(config: IppPrinterStatusConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            next_request_id: 1,
        }
    }

    pub fn config(&self) -> &IppPrinterStatusConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<PrinterAttributes, IppPrinterStatusError> {
        self.config.validate()?;
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.checked_add(1).ok_or_else(|| {
            IppPrinterStatusError::Validation("IPP request id space is exhausted".to_string())
        })?;
        let body = encode_get_printer_attributes(request_id, &self.config.printer_uri())?;
        let response = self.transport.post(&self.config, &body)?;
        Ok(decode_get_printer_attributes_response(
            &response, request_id,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledIppPrinterStatus {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
}

pub struct IppPrinterStatusRuntimeIntegration<T> {
    client: IppPrinterStatusClient<T>,
}

impl<T: IppPrinterStatusTransport> IppPrinterStatusRuntimeIntegration<T> {
    pub fn new(client: IppPrinterStatusClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledIppPrinterStatus, IppPrinterStatusError> {
        authorize_status_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_printer_status(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn install_printer_status(
    runtime: &mut SmartHomeRuntime,
    config: &IppPrinterStatusConfig,
    snapshot: &PrinterAttributes,
    observed_at_ms: u64,
) -> Result<InstalledIppPrinterStatus, IppPrinterStatusError> {
    config.validate()?;
    let native_id = config
        .bridge_id
        .as_str()
        .strip_prefix("ipp_printer.bridge.")
        .ok_or_else(|| IppPrinterStatusError::Validation("invalid IPP bridge id".to_string()))?;
    let device_id = DeviceId::trusted(format!("ipp-printer:{native_id}"));
    let entity_id = EntityId::trusted(format!("{}:diagnostic:printer", device_id.as_str()));
    let health = printer_health(snapshot);
    let protocol = ProtocolFamily::Vendor(PROTOCOL_ID.to_string());

    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.endpoint_url());
    bridge.hardware_model = Some(snapshot.printer_make_and_model.clone());
    bridge.health = health;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers =
        vec![
            ProtocolIdentifier::new(protocol.clone(), "ipp_endpoint", config.printer_uri())
                .map_err(|error| IppPrinterStatusError::Validation(error.to_string()))?,
        ];
    bridge.metadata = vec![Metadata::new("ipp.access", "credential_free_read_only")];
    runtime.upsert_bridge(bridge)?;

    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: "Unknown IPP manufacturer".to_string(),
        model: snapshot.printer_make_and_model.clone(),
        name: snapshot.printer_name.clone(),
        serial: None,
        firmware_version: None,
        room_id: None,
        entity_ids: vec![entity_id.clone()],
        identifiers: vec![
            ProtocolIdentifier::new(protocol, "printer_identity", native_id)
                .map_err(|error| IppPrinterStatusError::Validation(error.to_string()))?,
        ],
        health,
        metadata: vec![Metadata::new(
            "ipp.resource_path",
            config.resource_path.clone(),
        )],
    })?;

    runtime.upsert_entity(Entity {
        entity_id: entity_id.clone(),
        device_id: device_id.clone(),
        kind: EntityKind::NetworkDiagnostic,
        name: format!("{} Printer Status", snapshot.printer_name),
        capabilities: vec![Capability::new(
            CapabilityId::trusted("ipp.printer_status"),
            CapabilityMode::Observe,
            ValueKind::Object,
        )],
        state: Some(StateSnapshot {
            entity_id: entity_id.clone(),
            value: printer_status_value(snapshot),
            source: StateSource::Poll,
            observed_at_ms,
            received_at_ms: observed_at_ms,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        }),
        metadata: vec![Metadata::new("ipp.operation", "Get-Printer-Attributes")],
    })?;

    Ok(InstalledIppPrinterStatus {
        bridge_id: config.bridge_id.clone(),
        device_id,
        entity_id,
    })
}

fn authorize_status_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), IppPrinterStatusError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(IppPrinterStatusError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ))
    }
}

fn printer_health(snapshot: &PrinterAttributes) -> Health {
    if matches!(snapshot.printer_state, PrinterState::Stopped)
        || !snapshot.printer_is_accepting_jobs
        || snapshot
            .printer_state_reasons
            .iter()
            .any(|reason| reason != "none")
    {
        Health::Degraded
    } else {
        match snapshot.printer_state {
            PrinterState::Idle | PrinterState::Processing => Health::Online,
            PrinterState::Stopped => Health::Degraded,
            PrinterState::Unknown(_) => Health::Unknown,
        }
    }
}

fn printer_status_value(snapshot: &PrinterAttributes) -> Value {
    Value::Object(vec![
        (
            "accepting_jobs".to_string(),
            Value::Bool(snapshot.printer_is_accepting_jobs),
        ),
        (
            "make_and_model".to_string(),
            Value::Text(snapshot.printer_make_and_model.clone()),
        ),
        (
            "printer_info".to_string(),
            snapshot
                .printer_info
                .clone()
                .map_or(Value::Null, Value::Text),
        ),
        (
            "printer_location".to_string(),
            snapshot
                .printer_location
                .clone()
                .map_or(Value::Null, Value::Text),
        ),
        (
            "printer_name".to_string(),
            Value::Text(snapshot.printer_name.clone()),
        ),
        (
            "printer_state".to_string(),
            Value::Text(snapshot.printer_state.as_str().to_string()),
        ),
        (
            "printer_state_code".to_string(),
            Value::Integer(i64::from(snapshot.printer_state.code())),
        ),
        (
            "printer_state_reasons".to_string(),
            Value::Array(
                snapshot
                    .printer_state_reasons
                    .iter()
                    .cloned()
                    .map(Value::Text)
                    .collect(),
            ),
        ),
        (
            "queued_job_count".to_string(),
            Value::Integer(i64::from(snapshot.queued_job_count)),
        ),
        (
            "uptime_seconds".to_string(),
            Value::Integer(i64::from(snapshot.printer_up_time_seconds)),
        ),
    ])
}

fn is_local_ipv4(address: Ipv4Addr) -> bool {
    address.is_private() || address.is_link_local() || address.is_loopback()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use smart_home_discovery::{MdnsResponsePacket, MdnsScanFailure};
    use std::net::{TcpListener, UdpSocket};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    const PRINTER_UUID: &str = "12345678-9ABC-DEF0-1234-56789ABCDEF0";

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        result: MdnsScanResult,
    }

    #[derive(Debug)]
    struct FakeStatusTransport {
        calls: Arc<AtomicUsize>,
        response: Vec<u8>,
    }

    impl IppPrinterStatusTransport for FakeStatusTransport {
        fn post(
            &mut self,
            _config: &IppPrinterStatusConfig,
            _body: &[u8],
        ) -> Result<Vec<u8>, IppPrinterStatusError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.response.clone())
        }
    }

    impl IppPrinterMdnsTransport for FakeTransport {
        fn scan(
            &mut self,
            options: MdnsScanOptions,
        ) -> Result<MdnsScanResult, IppPrinterDiscoveryError> {
            assert_eq!(options.service_type, MDNS_SERVICE_TYPE);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn advertisement(address: &str, uuid: &str) -> MdnsAdvertisement {
        MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Office Printer",
            "office-printer.local",
            631,
            1_000,
        )
        .unwrap()
        .with_address(address)
        .unwrap()
        .with_txt("rp", "ipp/print")
        .unwrap()
        .with_txt("txtvers", "1")
        .unwrap()
        .with_txt("UUID", uuid)
        .unwrap()
        .with_txt("ty", "Example Laser 4000")
        .unwrap()
        .with_txt("note", "Office")
        .unwrap()
        .with_txt("air", "username,password")
        .unwrap()
        .with_txt("TLS", "1.3")
        .unwrap()
        .with_txt("pdl", "application/pdf,image/pwg-raster")
        .unwrap()
        .with_txt("Color", "T")
        .unwrap()
        .with_txt("Duplex", "F")
        .unwrap()
    }

    fn scan_result(advertisements: Vec<MdnsAdvertisement>) -> MdnsScanResult {
        MdnsScanResult {
            service_type: MDNS_SERVICE_TYPE.to_string(),
            discovered_at_ms: 1_000,
            datagram_count: advertisements.len(),
            advertisements,
            failures: Vec::new(),
        }
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:ipp-printer-discovery-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                0,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn validates_scan_bounds() {
        let mut config = IppPrinterDiscoveryConfig {
            timeout: Duration::ZERO,
            ..IppPrinterDiscoveryConfig::default()
        };
        assert!(config.validate().is_err());
        config.timeout = Duration::from_secs(1);
        config.maximum_responses = 0;
        assert!(config.validate().is_err());
        config.maximum_responses = MAX_RESPONSES + 1;
        assert!(config.validate().is_err());
        config.maximum_responses = 1;
        config.record_ttl = Duration::ZERO;
        assert!(config.validate().is_err());
    }

    #[test]
    fn normalizes_printer_identity_authentication_and_capabilities() {
        let record = discovery_record(&advertisement("192.0.2.60", PRINTER_UUID), 9_000).unwrap();
        assert_eq!(
            record.native_bridge_id,
            "printer-12345678-9abc-def0-1234-56789abcdef0"
        );
        assert_eq!(
            record.address.as_deref(),
            Some("ipp://192.0.2.60:631/ipp/print")
        );
        assert_eq!(record.display_name.as_deref(), Some("Office Printer"));
        assert_eq!(record.hardware_model.as_deref(), Some("Example Laser 4000"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::Credentials);
        assert_eq!(record.expires_at_ms, Some(9_000));
        assert!(record
            .metadata
            .iter()
            .any(|item| item.key == "ipp.authentication" && item.value == "username,password"));
        assert!(record
            .metadata
            .iter()
            .any(|item| item.key == "ipp.color_supported" && item.value == "true"));
        assert!(record
            .metadata
            .iter()
            .any(|item| item.key == "ipp.duplex_supported" && item.value == "false"));
    }

    #[test]
    fn rejects_incomplete_or_invalid_ipp_contract() {
        let mut missing_path = advertisement("192.0.2.60", PRINTER_UUID);
        missing_path
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("rp"));
        assert!(discovery_record(&missing_path, 1).is_err());

        let mut invalid_version = advertisement("192.0.2.60", PRINTER_UUID);
        invalid_version
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("txtvers"));
        invalid_version = invalid_version.with_txt("txtvers", "2").unwrap();
        assert!(discovery_record(&invalid_version, 1).is_err());

        assert!(discovery_record(&advertisement("192.0.2.60", "not-a-uuid"), 1).is_err());

        let mut traversal = advertisement("192.0.2.60", PRINTER_UUID);
        traversal
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("rp"));
        traversal = traversal.with_txt("rp", "../admin").unwrap();
        assert!(discovery_record(&traversal, 1).is_err());

        let mut invalid_color = advertisement("192.0.2.60", PRINTER_UUID);
        invalid_color
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("Color"));
        invalid_color = invalid_color.with_txt("Color", "yes").unwrap();
        assert!(discovery_record(&invalid_color, 1).is_err());

        let mut duplicate_path = advertisement("192.0.2.60", PRINTER_UUID);
        duplicate_path = duplicate_path.with_txt("RP", "other/queue").unwrap();
        assert!(discovery_record(&duplicate_path, 1).is_err());

        let mut invalid_tls = advertisement("192.0.2.60", PRINTER_UUID);
        invalid_tls
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("TLS"));
        invalid_tls = invalid_tls.with_txt("TLS", "latest").unwrap();
        assert!(discovery_record(&invalid_tls, 1).is_err());

        let mut unresolved = advertisement("192.0.2.60", PRINTER_UUID);
        unresolved.addresses.clear();
        assert!(discovery_record(&unresolved, 1).is_err());
    }

    #[test]
    fn deduplicates_stable_identity_and_preserves_partial_failures() {
        let mut result = scan_result(vec![
            advertisement("192.0.2.60", PRINTER_UUID),
            advertisement("192.0.2.61", PRINTER_UUID),
        ]);
        result.failures.push(MdnsScanFailure {
            source: Some("192.0.2.99:5353".to_string()),
            message: "truncated DNS packet".to_string(),
        });
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            result,
        };
        let report =
            discover(&IppPrinterDiscoveryConfig::default(), &mut transport, 1_000).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].address.as_deref(),
            Some("ipp://192.0.2.60:631/ipp/print")
        );
        assert_eq!(report.failures.len(), 2);
    }

    #[test]
    fn denies_before_transport_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            result: scan_result(Vec::new()),
        };
        let result = discover_into_runtime(
            &mut SmartHomeRuntime::new(),
            AgentId::trusted("agent:denied"),
            &IppPrinterDiscoveryConfig::default(),
            &mut transport,
            1_000,
        );
        assert!(matches!(
            result,
            Err(IppPrinterDiscoveryError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn validates_status_endpoint_identity_and_bounds() {
        let bridge_id =
            BridgeId::trusted("ipp_printer.bridge.printer-12345678-9abc-def0-1234-56789abcdef0");
        assert!(IppPrinterStatusConfig::new(
            bridge_id.clone(),
            "127.0.0.1:631".parse().unwrap(),
            "ipp/print",
        )
        .is_ok());
        assert!(IppPrinterStatusConfig::new(
            bridge_id.clone(),
            "8.8.8.8:631".parse().unwrap(),
            "ipp/print",
        )
        .is_err());
        assert!(IppPrinterStatusConfig::new(
            BridgeId::trusted("other.bridge.printer-one"),
            "127.0.0.1:631".parse().unwrap(),
            "ipp/print",
        )
        .is_err());
        assert!(IppPrinterStatusConfig::new(
            bridge_id.clone(),
            "127.0.0.1:631".parse().unwrap(),
            "../admin",
        )
        .is_err());
        let mut config =
            IppPrinterStatusConfig::new(bridge_id, "127.0.0.1:631".parse().unwrap(), "ipp/print")
                .unwrap();
        config.timeout = Duration::from_secs(6);
        assert!(config.validate().is_err());
        config.timeout = Duration::from_secs(1);
        config.maximum_response_bytes = 1023;
        assert!(config.validate().is_err());
    }

    #[test]
    fn status_read_denies_before_transport_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let config = status_config("127.0.0.1:631".parse().unwrap());
        let client = IppPrinterStatusClient::new(
            config,
            FakeStatusTransport {
                calls: calls.clone(),
                response: status_response(1, 3, &["none"], true),
            },
        );
        let mut integration = IppPrinterStatusRuntimeIntegration::new(client);
        let result = integration.inspect_and_install_authorized(
            &mut SmartHomeRuntime::new(),
            AgentId::trusted("agent:ipp-status-denied"),
            1_000,
        );
        assert!(matches!(
            result,
            Err(IppPrinterStatusError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn normalizes_fixed_printer_status_and_health() {
        let config = status_config("127.0.0.1:631".parse().unwrap());
        let snapshot = PrinterAttributes {
            printer_name: "Office".to_string(),
            printer_info: Some("Main printer".to_string()),
            printer_location: Some("Second floor".to_string()),
            printer_make_and_model: "Example Laser 2".to_string(),
            printer_state: PrinterState::Stopped,
            printer_state_reasons: vec!["media-empty-error".to_string()],
            printer_is_accepting_jobs: false,
            queued_job_count: 3,
            printer_up_time_seconds: 90,
        };
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_printer_status(&mut runtime, &config, &snapshot, 2_000).unwrap();
        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(bridge.health, Health::Degraded);
        let entity = runtime.registry().entity(&installed.entity_id).unwrap();
        assert_eq!(entity.kind, EntityKind::NetworkDiagnostic);
        assert_eq!(
            entity.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        assert!(matches!(
            entity.state.as_ref().unwrap().value,
            Value::Object(_)
        ));
        assert_eq!(
            entity.capabilities[0].capability_id,
            CapabilityId::trusted("ipp.printer_status")
        );
    }

    #[test]
    fn live_loopback_status_read_verifies_wire_and_commits() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = match listener.local_addr().unwrap() {
            std::net::SocketAddr::V4(endpoint) => endpoint,
            std::net::SocketAddr::V6(_) => unreachable!(),
        };
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream);
            let split = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .unwrap()
                + 4;
            let head = std::str::from_utf8(&request[..split]).unwrap();
            let body = &request[split..];
            assert!(head.starts_with("POST /ipp/print HTTP/1.1\r\n"));
            assert!(head.contains("Content-Type: application/ipp\r\n"));
            assert!(head.contains("Accept: application/ipp\r\n"));
            assert!(head.contains("Connection: close\r\n"));
            assert!(!head.to_ascii_lowercase().contains("authorization:"));
            assert_eq!(&body[..8], &[1, 1, 0, 11, 0, 0, 0, 1]);
            for attribute in ipp_protocol::REQUESTED_ATTRIBUTES {
                assert!(body
                    .windows(attribute.len())
                    .any(|window| window == attribute.as_bytes()));
            }

            let response_body = status_response(1, 4, &["none"], true);
            let response_head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/ipp\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_body.len()
            );
            stream.write_all(response_head.as_bytes()).unwrap();
            stream.write_all(&response_body).unwrap();
        });

        let principal = AgentId::trusted("agent:ipp-printer-status");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let client = IppPrinterStatusClient::new(status_config(endpoint), IppPrinterLanTransport);
        let mut integration = IppPrinterStatusRuntimeIntegration::new(client);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 3_000)
            .unwrap();
        server.join().unwrap();

        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(bridge.health, Health::Online);
        let expected_url = format!("http://{endpoint}/ipp/print");
        assert_eq!(bridge.address.as_deref(), Some(expected_url.as_str()));
        let entity = runtime.registry().entity(&installed.entity_id).unwrap();
        assert!(entity.state.as_ref().is_some());
    }

    #[test]
    fn rejects_authentication_chunking_and_wrong_media_type() {
        for response in [
            b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n".as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Type: application/ipp\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"
                .as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}"
                .as_slice(),
        ] {
            assert!(decode_status_http_response(
                response,
                DEFAULT_MAX_STATUS_RESPONSE_BYTES
            )
            .is_err());
        }
    }

    fn status_config(endpoint: SocketAddrV4) -> IppPrinterStatusConfig {
        IppPrinterStatusConfig::new(
            BridgeId::trusted("ipp_printer.bridge.printer-12345678-9abc-def0-1234-56789abcdef0"),
            endpoint,
            "ipp/print",
        )
        .unwrap()
    }

    fn status_response(request_id: u32, state: i32, reasons: &[&str], accepting: bool) -> Vec<u8> {
        let mut bytes = vec![1, 1, 0, 0];
        bytes.extend_from_slice(&request_id.to_be_bytes());
        bytes.push(0x01);
        push_ipp_attribute(&mut bytes, 0x47, "attributes-charset", b"utf-8");
        push_ipp_attribute(&mut bytes, 0x48, "attributes-natural-language", b"en-us");
        bytes.push(0x04);
        push_ipp_attribute(&mut bytes, 0x42, "printer-name", b"Office");
        push_ipp_attribute(&mut bytes, 0x41, "printer-info", b"Main printer");
        push_ipp_attribute(&mut bytes, 0x41, "printer-location", b"Second floor");
        push_ipp_attribute(
            &mut bytes,
            0x41,
            "printer-make-and-model",
            b"Example Laser 2",
        );
        push_ipp_attribute(&mut bytes, 0x23, "printer-state", &state.to_be_bytes());
        for (index, reason) in reasons.iter().enumerate() {
            push_ipp_attribute(
                &mut bytes,
                0x44,
                if index == 0 {
                    "printer-state-reasons"
                } else {
                    ""
                },
                reason.as_bytes(),
            );
        }
        push_ipp_attribute(
            &mut bytes,
            0x22,
            "printer-is-accepting-jobs",
            &[u8::from(accepting)],
        );
        push_ipp_attribute(&mut bytes, 0x21, "queued-job-count", &2i32.to_be_bytes());
        push_ipp_attribute(&mut bytes, 0x21, "printer-up-time", &90i32.to_be_bytes());
        bytes.push(0x03);
        bytes
    }

    fn push_ipp_attribute(bytes: &mut Vec<u8>, tag: u8, name: &str, value: &[u8]) {
        bytes.push(tag);
        bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&(value.len() as u16).to_be_bytes());
        bytes.extend_from_slice(value);
    }

    fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 4096];
        let mut expected = None;
        loop {
            let read = stream.read(&mut buffer).unwrap();
            assert!(read > 0);
            bytes.extend_from_slice(&buffer[..read]);
            if expected.is_none() {
                if let Some(offset) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let body_offset = offset + 4;
                    let head = std::str::from_utf8(&bytes[..body_offset]).unwrap();
                    let content_length = head
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("Content-Length: ")
                                .and_then(|value| value.parse::<usize>().ok())
                        })
                        .unwrap();
                    expected = Some(body_offset + content_length);
                }
            }
            if expected.is_some_and(|length| bytes.len() >= length) {
                break;
            }
        }
        bytes
    }

    #[test]
    fn live_loopback_packet_is_parsed_and_committed() {
        struct LoopbackTransport;
        impl IppPrinterMdnsTransport for LoopbackTransport {
            fn scan(
                &mut self,
                options: MdnsScanOptions,
            ) -> Result<MdnsScanResult, IppPrinterDiscoveryError> {
                let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
                receiver
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
                sender
                    .send_to(&ipp_mdns_response_packet(), receiver.local_addr().unwrap())
                    .unwrap();
                let mut bytes = [0u8; 1500];
                let (length, source) = receiver.recv_from(&mut bytes).unwrap();
                Ok(MdnsScanResult::from_packets(
                    options.service_type,
                    options.discovered_at_ms,
                    [MdnsResponsePacket::new(bytes[..length].to_vec())
                        .with_source(source.to_string())],
                )?)
            }
        }

        let principal = AgentId::trusted("agent:ipp-printer-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &IppPrinterDiscoveryConfig::default(),
            &mut LoopbackTransport,
            2_000,
        )
        .unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "ipp_printer.bridge.printer-12345678-9abc-def0-1234-56789abcdef0"
            ))
            .is_some());
    }

    fn ipp_mdns_response_packet() -> Vec<u8> {
        let mut packet = Vec::new();
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0x8400);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 1);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 3);

        let instance = "Office Printer._ipp._tcp.local";
        encode_name(MDNS_SERVICE_TYPE, &mut packet);
        push_record_header(&mut packet, 12);
        let ptr_length = reserve_length(&mut packet);
        encode_name(instance, &mut packet);
        fill_length(&mut packet, ptr_length);

        encode_name(instance, &mut packet);
        push_record_header(&mut packet, 33);
        let srv_length = reserve_length(&mut packet);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 631);
        encode_name("office-printer.local", &mut packet);
        fill_length(&mut packet, srv_length);

        encode_name(instance, &mut packet);
        push_record_header(&mut packet, 16);
        let txt_length = reserve_length(&mut packet);
        for (key, value) in [
            ("rp", "ipp/print"),
            ("txtvers", "1"),
            ("UUID", PRINTER_UUID),
            ("ty", "Example Laser 4000"),
            ("air", "none"),
            ("Color", "T"),
            ("Duplex", "T"),
        ] {
            push_txt(&mut packet, key, value);
        }
        fill_length(&mut packet, txt_length);

        encode_name("office-printer.local", &mut packet);
        push_record_header(&mut packet, 1);
        let address_length = reserve_length(&mut packet);
        packet.extend_from_slice(&[127, 0, 0, 1]);
        fill_length(&mut packet, address_length);
        packet
    }

    fn encode_name(name: &str, packet: &mut Vec<u8>) {
        for label in name.trim_end_matches('.').split('.') {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }
        packet.push(0);
    }

    fn push_record_header(packet: &mut Vec<u8>, record_type: u16) {
        push_u16(packet, record_type);
        push_u16(packet, 1);
        packet.extend_from_slice(&120u32.to_be_bytes());
    }

    fn reserve_length(packet: &mut Vec<u8>) -> usize {
        let offset = packet.len();
        push_u16(packet, 0);
        offset
    }

    fn fill_length(packet: &mut [u8], offset: usize) {
        let length = packet.len() - offset - 2;
        packet[offset..offset + 2].copy_from_slice(&(length as u16).to_be_bytes());
    }

    fn push_txt(packet: &mut Vec<u8>, key: &str, value: &str) {
        let entry = format!("{key}={value}");
        packet.push(entry.len() as u8);
        packet.extend_from_slice(entry.as_bytes());
    }

    fn push_u16(packet: &mut Vec<u8>, value: u16) {
        packet.extend_from_slice(&value.to_be_bytes());
    }
}
