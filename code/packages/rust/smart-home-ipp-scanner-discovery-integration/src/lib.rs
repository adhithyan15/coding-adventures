//! Authorized bounded IPP Scan Service mDNS discovery for D23.

#![forbid(unsafe_code)]

use smart_home_core::{AgentId, BridgeTransport, IntegrationId, ProtocolFamily, SmartHomeTool};
use smart_home_discovery::{
    run_mdns_ipv4_scan, DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource,
    DiscoveryUpsert, MdnsAdvertisement, MdnsScanOptions, MdnsScanResult, PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::Duration;

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "ipp_scanner";
pub const PROTOCOL_ID: &str = "ipp_scan_service";
pub const MDNS_SERVICE_TYPE: &str = "_scan._sub._ipp._tcp.local";
pub const MAX_RESPONSES: usize = 64;
const MAX_TXT_VALUE_BYTES: usize = 255;
const DEFAULT_RESOURCE_PATH: &str = "ipp/scan";
const MAX_PUSH_DESTINATION_SCHEMES: usize = 8;

#[derive(Debug)]
pub enum IppScannerDiscoveryError {
    Validation(String),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for IppScannerDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(formatter, "invalid IPP scanner discovery: {message}")
            }
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for IppScannerDiscoveryError {}

impl From<DiscoveryError> for IppScannerDiscoveryError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for IppScannerDiscoveryError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IppScannerDiscoveryConfig {
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl Default for IppScannerDiscoveryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }
}

impl IppScannerDiscoveryConfig {
    pub fn validate(&self) -> Result<(), IppScannerDiscoveryError> {
        if self.timeout.is_zero() {
            return Err(IppScannerDiscoveryError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(IppScannerDiscoveryError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(IppScannerDiscoveryError::Validation(
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

pub trait IppScannerMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, IppScannerDiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpIppScannerMdnsTransport;

impl IppScannerMdnsTransport for UdpIppScannerMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, IppScannerDiscoveryError> {
        Ok(run_mdns_ipv4_scan(options)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IppScannerDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IppScannerRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: IppScannerMdnsTransport>(
    config: &IppScannerDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<IppScannerDiscoveryReport, IppScannerDiscoveryError> {
    config.validate()?;
    let result = transport.scan(config.scan_options(discovered_at_ms)?)?;
    if normalized_service_type(&result.service_type) != normalized_service_type(MDNS_SERVICE_TYPE) {
        return Err(IppScannerDiscoveryError::Validation(format!(
            "scanner returned unexpected service type `{}`",
            result.service_type
        )));
    }

    let advertisement_count = result.advertisements.len();
    let mut failures = result
        .failures
        .into_iter()
        .map(|failure| match failure.source {
            Some(source) => format!("invalid mDNS reply from {source}: {}", failure.message),
            None => format!("invalid mDNS reply: {}", failure.message),
        })
        .collect::<Vec<_>>();
    if advertisement_count > config.maximum_responses {
        failures.push(format!(
            "discarded {} IPP scanner advertisements beyond the configured result limit",
            advertisement_count - config.maximum_responses
        ));
    }
    let ttl_ms = u64::try_from(config.record_ttl.as_millis()).unwrap_or(u64::MAX);
    let expires_at_ms = discovered_at_ms.saturating_add(ttl_ms);
    let mut records = BTreeMap::<String, DiscoveryRecord>::new();

    for advertisement in result
        .advertisements
        .into_iter()
        .take(config.maximum_responses)
    {
        match discovery_record(&advertisement, expires_at_ms) {
            Ok(record) => match records.get(&record.native_bridge_id) {
                Some(existing) if existing.address != record.address => failures.push(format!(
                    "IPP scanner {} advertises conflicting endpoints `{}` and `{}`",
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
                "invalid IPP scanner advertisement `{}`: {error}",
                advertisement.instance_name
            )),
        }
    }

    Ok(IppScannerDiscoveryReport {
        records: records.into_values().collect(),
        failures,
    })
}

pub fn discover_into_runtime<T: IppScannerMdnsTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &IppScannerDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<IppScannerRuntimeCommitSummary, IppScannerDiscoveryError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(IppScannerDiscoveryError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = IppScannerRuntimeCommitSummary {
        failures: report.failures.len(),
        ..IppScannerRuntimeCommitSummary::default()
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
) -> Result<DiscoveryRecord, IppScannerDiscoveryError> {
    if normalized_service_type(&advertisement.service_type)
        != normalized_service_type(MDNS_SERVICE_TYPE)
    {
        return Err(IppScannerDiscoveryError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    if advertisement.port == 0 {
        return Err(IppScannerDiscoveryError::Validation(
            "IPP port must be non-zero".to_string(),
        ));
    }
    if advertisement.addresses.is_empty() {
        return Err(IppScannerDiscoveryError::Validation(
            "IPP advertisement must resolve at least one IP address".to_string(),
        ));
    }
    validate_unique_txt_keys(advertisement)?;

    let display_name = scanner_display_name(&advertisement.instance_name)?;
    let print_resource_path = optional_txt(advertisement, "rp")
        .map(validate_resource_path)
        .transpose()?;
    let resource_path =
        validate_resource_path(optional_txt(advertisement, "rs").unwrap_or(DEFAULT_RESOURCE_PATH))?;
    let txt_version = optional_txt(advertisement, "txtvers").unwrap_or("1");
    if txt_version != "1" {
        return Err(IppScannerDiscoveryError::Validation(
            "txtvers must be 1".to_string(),
        ));
    }
    let scanner_uuid = optional_txt(advertisement, "UUID")
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
    let document_feeder = parse_document_feeder(optional_txt(advertisement, "ADF").unwrap_or("N"))?;
    let transparency_adapter =
        parse_tristate(optional_txt(advertisement, "TMA").unwrap_or("U"), "TMA")?;
    let push_destination_schemes =
        parse_push_destination_schemes(optional_txt(advertisement, "Scan2").unwrap_or(""))?;
    let native_bridge_id = scanner_uuid.as_ref().map_or_else(
        || fallback_identity(advertisement, &resource_path),
        |uuid| format!("scanner-{uuid}"),
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
    .with_metadata("ipp.scan.resource_path", &resource_path)
    .with_metadata("ipp.txt_version", txt_version)
    .with_metadata("ipp.authentication", authentication.as_str());
    if let Some(hardware_model) = hardware_model {
        record = record.with_hardware_model(hardware_model);
    }
    if let Some(scanner_uuid) = scanner_uuid {
        record = record.with_metadata("ipp.scan.uuid", scanner_uuid);
    }
    if let Some(location) = location {
        record = record.with_metadata("ipp.location", location);
    }
    if let Some(tls) = tls {
        record = record.with_metadata("ipp.maximum_tls_version", tls);
    }
    if let Some(document_formats) = document_formats {
        record = record.with_metadata("ipp.scan.document_formats", document_formats);
    }
    record = record
        .with_metadata("ipp.scan.document_feeder", document_feeder)
        .with_metadata("ipp.scan.transparency_adapter", transparency_adapter)
        .with_metadata(
            "ipp.scan.push_destination_schemes",
            push_destination_schemes,
        );
    if optional_txt(advertisement, "rs").is_none() {
        record = record.with_metadata("ipp.scan.resource_path_defaulted", "true");
    }
    if let Some(print_resource_path) = print_resource_path {
        record = record
            .with_metadata("ipp.scan.multifunction", "true")
            .with_metadata("ipp.print.resource_path", print_resource_path);
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

fn parse_authentication(value: &str) -> Result<IppAuthentication, IppScannerDiscoveryError> {
    match value {
        "none" => Ok(IppAuthentication::None),
        "certificate" => Ok(IppAuthentication::Certificate),
        "negotiate" => Ok(IppAuthentication::Negotiate),
        "oauth" => Ok(IppAuthentication::OAuth2),
        "username,password" => Ok(IppAuthentication::UsernamePassword),
        _ => Err(IppScannerDiscoveryError::Validation(
            "air contains an unsupported authentication requirement".to_string(),
        )),
    }
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
) -> Result<(), IppScannerDiscoveryError> {
    let mut keys = BTreeSet::new();
    for entry in &advertisement.txt {
        let normalized = entry.key.to_ascii_lowercase();
        if !keys.insert(normalized) {
            return Err(IppScannerDiscoveryError::Validation(format!(
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
) -> Result<String, IppScannerDiscoveryError> {
    if value.is_empty()
        || value.len() > maximum_bytes.min(MAX_TXT_VALUE_BYTES)
        || value.chars().any(char::is_control)
    {
        return Err(IppScannerDiscoveryError::Validation(format!(
            "{field} must be non-empty bounded text without control characters"
        )));
    }
    Ok(value.to_string())
}

fn scanner_display_name(value: &str) -> Result<String, IppScannerDiscoveryError> {
    let base_service_suffix = "._ipp._tcp.local";
    let display_name = if value.to_ascii_lowercase().ends_with(base_service_suffix) {
        &value[..value.len() - base_service_suffix.len()]
    } else {
        value
    };
    safe_text(display_name, "instance name", 255)
}

fn validate_resource_path(value: &str) -> Result<String, IppScannerDiscoveryError> {
    if value.is_empty() || value.len() > 255 || value.starts_with('/') || !value.is_ascii() {
        return Err(IppScannerDiscoveryError::Validation(
            "IPP resource paths must be non-empty bounded relative URI paths without a leading slash"
                .to_string(),
        ));
    }
    if value
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(IppScannerDiscoveryError::Validation(
            "IPP resource paths must not contain dot path segments".to_string(),
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
                return Err(IppScannerDiscoveryError::Validation(
                    "IPP resource path contains an invalid percent escape".to_string(),
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
            return Err(IppScannerDiscoveryError::Validation(
                "IPP resource path contains a character outside the URI path grammar".to_string(),
            ));
        }
        index += 1;
    }
    Ok(value.to_string())
}

fn validate_uuid(value: &str) -> Result<String, IppScannerDiscoveryError> {
    if value.len() != 36 {
        return Err(IppScannerDiscoveryError::Validation(
            "UUID must be a canonical 36-character UUID".to_string(),
        ));
    }
    let mut normalized = String::with_capacity(36);
    for (index, byte) in value.bytes().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return Err(IppScannerDiscoveryError::Validation(
                    "UUID must be a canonical hyphenated UUID".to_string(),
                ));
            }
            normalized.push('-');
        } else if byte.is_ascii_hexdigit() {
            normalized.push((byte as char).to_ascii_lowercase());
        } else {
            return Err(IppScannerDiscoveryError::Validation(
                "UUID must contain only hexadecimal digits and canonical hyphens".to_string(),
            ));
        }
    }
    Ok(normalized)
}

fn validate_document_formats(value: &str) -> Result<String, IppScannerDiscoveryError> {
    if value.len() > MAX_TXT_VALUE_BYTES {
        return Err(IppScannerDiscoveryError::Validation(
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
        return Err(IppScannerDiscoveryError::Validation(
            "pdl must contain a bounded comma-separated MIME type list".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn validate_tls_version(value: &str) -> Result<String, IppScannerDiscoveryError> {
    if value == "none" {
        return Ok(value.to_string());
    }
    let Some((major, minor)) = value.split_once('.') else {
        return Err(IppScannerDiscoveryError::Validation(
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
        return Err(IppScannerDiscoveryError::Validation(
            "TLS must be none or a bounded major.minor version".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn parse_tristate(value: &str, field: &str) -> Result<&'static str, IppScannerDiscoveryError> {
    match value {
        "T" => Ok("true"),
        "F" => Ok("false"),
        "U" => Ok("unknown"),
        _ => Err(IppScannerDiscoveryError::Validation(format!(
            "{field} must be T, F, or U"
        ))),
    }
}

fn parse_document_feeder(value: &str) -> Result<&'static str, IppScannerDiscoveryError> {
    match value {
        "S" => Ok("simplex"),
        "D" => Ok("duplex"),
        "N" => Ok("none"),
        _ => Err(IppScannerDiscoveryError::Validation(
            "ADF must be S, D, or N".to_string(),
        )),
    }
}

fn parse_push_destination_schemes(value: &str) -> Result<String, IppScannerDiscoveryError> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let allowed = [
        "ftp", "ftps", "http", "https", "ipp", "ipps", "mailto", "smb",
    ];
    let schemes = value.split(',').collect::<Vec<_>>();
    if schemes.len() > MAX_PUSH_DESTINATION_SCHEMES
        || schemes
            .iter()
            .any(|scheme| !allowed.contains(scheme) || scheme.is_empty())
    {
        return Err(IppScannerDiscoveryError::Validation(
            "Scan2 must be a bounded comma-separated list of standard destination URI schemes"
                .to_string(),
        ));
    }
    let unique = schemes.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != schemes.len() {
        return Err(IppScannerDiscoveryError::Validation(
            "Scan2 must not contain duplicate destination URI schemes".to_string(),
        ));
    }
    Ok(schemes.join(","))
}

fn fallback_identity(advertisement: &MdnsAdvertisement, resource_path: &str) -> String {
    let value = format!(
        "{}-{}-{}",
        advertisement.host_name, advertisement.port, resource_path
    );
    let mut identity = String::from("scanner-endpoint-");
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

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use smart_home_discovery::{MdnsResponsePacket, MdnsScanFailure};
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const SCANNER_UUID: &str = "12345678-9ABC-DEF0-1234-56789ABCDEF0";

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        result: MdnsScanResult,
    }

    impl IppScannerMdnsTransport for FakeTransport {
        fn scan(
            &mut self,
            options: MdnsScanOptions,
        ) -> Result<MdnsScanResult, IppScannerDiscoveryError> {
            assert_eq!(options.service_type, MDNS_SERVICE_TYPE);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn advertisement(address: &str, uuid: &str) -> MdnsAdvertisement {
        MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Office Scanner",
            "office-scanner.local",
            631,
            1_000,
        )
        .unwrap()
        .with_address(address)
        .unwrap()
        .with_txt("rs", "ipp/scan")
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
        .with_txt("ADF", "D")
        .unwrap()
        .with_txt("TMA", "T")
        .unwrap()
        .with_txt("Scan2", "ftp,https")
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
                CapabilityGrantId::trusted("grant:ipp-scanner-discovery-test"),
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
        let mut config = IppScannerDiscoveryConfig {
            timeout: Duration::ZERO,
            ..IppScannerDiscoveryConfig::default()
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
    fn normalizes_scanner_identity_authentication_and_capabilities() {
        let record = discovery_record(&advertisement("192.0.2.60", SCANNER_UUID), 9_000).unwrap();
        assert_eq!(
            record.native_bridge_id,
            "scanner-12345678-9abc-def0-1234-56789abcdef0"
        );
        assert_eq!(
            record.address.as_deref(),
            Some("ipp://192.0.2.60:631/ipp/scan")
        );
        assert_eq!(record.display_name.as_deref(), Some("Office Scanner"));
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
            .any(|item| item.key == "ipp.scan.document_feeder" && item.value == "duplex"));
        assert!(record
            .metadata
            .iter()
            .any(|item| item.key == "ipp.scan.transparency_adapter" && item.value == "true"));
        assert!(record.metadata.iter().any(|item| {
            item.key == "ipp.scan.push_destination_schemes" && item.value == "ftp,https"
        }));

        let mut defaulted = advertisement("192.0.2.60", SCANNER_UUID);
        defaulted.txt.retain(|entry| {
            !["rs", "ADF", "TMA", "Scan2"]
                .iter()
                .any(|key| entry.key.eq_ignore_ascii_case(key))
        });
        let defaulted = discovery_record(&defaulted, 9_000).unwrap();
        assert_eq!(
            defaulted.address.as_deref(),
            Some("ipp://192.0.2.60:631/ipp/scan")
        );
        assert!(defaulted.metadata.iter().any(|item| {
            item.key == "ipp.scan.resource_path_defaulted" && item.value == "true"
        }));
        assert!(defaulted
            .metadata
            .iter()
            .any(|item| { item.key == "ipp.scan.document_feeder" && item.value == "none" }));
        assert!(defaulted.metadata.iter().any(|item| {
            item.key == "ipp.scan.transparency_adapter" && item.value == "unknown"
        }));
        assert!(defaulted.metadata.iter().any(|item| {
            item.key == "ipp.scan.push_destination_schemes" && item.value.is_empty()
        }));
    }

    #[test]
    fn rejects_incomplete_or_invalid_ipp_contract() {
        let multifunction = advertisement("192.0.2.60", SCANNER_UUID)
            .with_txt("rp", "ipp/print")
            .unwrap();
        let multifunction = discovery_record(&multifunction, 1).unwrap();
        assert!(multifunction
            .metadata
            .iter()
            .any(|item| { item.key == "ipp.print.resource_path" && item.value == "ipp/print" }));

        let mut invalid_print_path = advertisement("192.0.2.60", SCANNER_UUID);
        invalid_print_path = invalid_print_path.with_txt("rp", "../print").unwrap();
        assert!(discovery_record(&invalid_print_path, 1).is_err());

        let mut invalid_version = advertisement("192.0.2.60", SCANNER_UUID);
        invalid_version
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("txtvers"));
        invalid_version = invalid_version.with_txt("txtvers", "2").unwrap();
        assert!(discovery_record(&invalid_version, 1).is_err());

        assert!(discovery_record(&advertisement("192.0.2.60", "not-a-uuid"), 1).is_err());

        let mut traversal = advertisement("192.0.2.60", SCANNER_UUID);
        traversal
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("rs"));
        traversal = traversal.with_txt("rs", "../admin").unwrap();
        assert!(discovery_record(&traversal, 1).is_err());

        let mut invalid_adf = advertisement("192.0.2.60", SCANNER_UUID);
        invalid_adf
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("ADF"));
        invalid_adf = invalid_adf.with_txt("ADF", "T").unwrap();
        assert!(discovery_record(&invalid_adf, 1).is_err());

        let mut invalid_tma = advertisement("192.0.2.60", SCANNER_UUID);
        invalid_tma
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("TMA"));
        invalid_tma = invalid_tma.with_txt("TMA", "yes").unwrap();
        assert!(discovery_record(&invalid_tma, 1).is_err());

        let mut invalid_scan2 = advertisement("192.0.2.60", SCANNER_UUID);
        invalid_scan2
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("Scan2"));
        invalid_scan2 = invalid_scan2.with_txt("Scan2", "https,HTTPS").unwrap();
        assert!(discovery_record(&invalid_scan2, 1).is_err());

        let mut duplicate_path = advertisement("192.0.2.60", SCANNER_UUID);
        duplicate_path = duplicate_path.with_txt("RS", "other/queue").unwrap();
        assert!(discovery_record(&duplicate_path, 1).is_err());

        let mut invalid_tls = advertisement("192.0.2.60", SCANNER_UUID);
        invalid_tls
            .txt
            .retain(|entry| !entry.key.eq_ignore_ascii_case("TLS"));
        invalid_tls = invalid_tls.with_txt("TLS", "latest").unwrap();
        assert!(discovery_record(&invalid_tls, 1).is_err());

        let mut unresolved = advertisement("192.0.2.60", SCANNER_UUID);
        unresolved.addresses.clear();
        assert!(discovery_record(&unresolved, 1).is_err());
    }

    #[test]
    fn deduplicates_stable_identity_and_preserves_partial_failures() {
        let mut result = scan_result(vec![
            advertisement("192.0.2.60", SCANNER_UUID),
            advertisement("192.0.2.61", SCANNER_UUID),
            advertisement("192.0.2.62", "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
        ]);
        result.failures.push(MdnsScanFailure {
            source: Some("192.0.2.99:5353".to_string()),
            message: "truncated DNS packet".to_string(),
        });
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            result,
        };
        let config = IppScannerDiscoveryConfig {
            maximum_responses: 2,
            ..IppScannerDiscoveryConfig::default()
        };
        let report = discover(&config, &mut transport, 1_000).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].address.as_deref(),
            Some("ipp://192.0.2.60:631/ipp/scan")
        );
        assert_eq!(report.failures.len(), 3);
        assert!(report
            .failures
            .iter()
            .any(|failure| failure.contains("configured result limit")));
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
            &IppScannerDiscoveryConfig::default(),
            &mut transport,
            1_000,
        );
        assert!(matches!(
            result,
            Err(IppScannerDiscoveryError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn live_loopback_packet_is_parsed_and_committed() {
        struct LoopbackTransport;
        impl IppScannerMdnsTransport for LoopbackTransport {
            fn scan(
                &mut self,
                options: MdnsScanOptions,
            ) -> Result<MdnsScanResult, IppScannerDiscoveryError> {
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

        let principal = AgentId::trusted("agent:ipp-scanner-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &IppScannerDiscoveryConfig::default(),
            &mut LoopbackTransport,
            2_000,
        )
        .unwrap();
        assert_eq!(summary.inserted, 1);
        let bridge = runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "ipp_scanner.bridge.scanner-12345678-9abc-def0-1234-56789abcdef0",
            ))
            .unwrap();
        assert!(bridge.metadata.iter().any(|item| {
            item.key == "smart_home.discovery.display_name" && item.value == "Office Scanner"
        }));
    }

    fn ipp_mdns_response_packet() -> Vec<u8> {
        let mut packet = Vec::new();
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0x8400);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 1);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 3);

        let instance = "Office Scanner._ipp._tcp.local";
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
        encode_name("office-scanner.local", &mut packet);
        fill_length(&mut packet, srv_length);

        encode_name(instance, &mut packet);
        push_record_header(&mut packet, 16);
        let txt_length = reserve_length(&mut packet);
        for (key, value) in [
            ("rs", "ipp/scan"),
            ("txtvers", "1"),
            ("UUID", SCANNER_UUID),
            ("ty", "Example Laser 4000"),
            ("air", "none"),
            ("ADF", "S"),
            ("TMA", "F"),
            ("Scan2", "https"),
        ] {
            push_txt(&mut packet, key, value);
        }
        fill_length(&mut packet, txt_length);

        encode_name("office-scanner.local", &mut packet);
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
