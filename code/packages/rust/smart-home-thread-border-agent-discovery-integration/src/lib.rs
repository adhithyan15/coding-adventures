//! Authorized bounded Thread Border Agent mDNS discovery for D23.

#![forbid(unsafe_code)]

use smart_home_core::{AgentId, BridgeTransport, IntegrationId, ProtocolFamily, SmartHomeTool};
use smart_home_discovery::{
    run_mdns_ipv4_scan, DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource,
    DiscoveryUpsert, MdnsAdvertisement, MdnsScanOptions, MdnsScanResult, PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::net::Ipv6Addr;
use std::str;
use std::time::Duration;

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "thread_border_agent";
pub const MDNS_SERVICE_TYPE: &str = "_meshcop._udp.local";
pub const MAX_RESPONSES: usize = 64;
const MAX_TEXT_BYTES: usize = 32;
const RECORD_VERSION: &str = "1";

#[derive(Debug)]
pub enum ThreadBorderAgentDiscoveryError {
    Validation(String),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for ThreadBorderAgentDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(
                    formatter,
                    "invalid Thread Border Agent discovery: {message}"
                )
            }
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ThreadBorderAgentDiscoveryError {}

impl From<DiscoveryError> for ThreadBorderAgentDiscoveryError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for ThreadBorderAgentDiscoveryError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadBorderAgentDiscoveryConfig {
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl Default for ThreadBorderAgentDiscoveryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }
}

impl ThreadBorderAgentDiscoveryConfig {
    pub fn validate(&self) -> Result<(), ThreadBorderAgentDiscoveryError> {
        if self.timeout.is_zero() {
            return Err(ThreadBorderAgentDiscoveryError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(ThreadBorderAgentDiscoveryError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(ThreadBorderAgentDiscoveryError::Validation(
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

pub trait ThreadBorderAgentMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, ThreadBorderAgentDiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpThreadBorderAgentMdnsTransport;

impl ThreadBorderAgentMdnsTransport for UdpThreadBorderAgentMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, ThreadBorderAgentDiscoveryError> {
        Ok(run_mdns_ipv4_scan(options)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadBorderAgentDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThreadBorderAgentRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BorderAgentState {
    raw: u32,
    connection_mode: u8,
    interface_state: u8,
    availability: u8,
    role: u8,
    backbone_active: bool,
    backbone_primary: bool,
    epskc_supported: bool,
    multi_ail_state: u8,
    admitter_supported: bool,
}

pub fn discover<T: ThreadBorderAgentMdnsTransport>(
    config: &ThreadBorderAgentDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<ThreadBorderAgentDiscoveryReport, ThreadBorderAgentDiscoveryError> {
    config.validate()?;
    let result = transport.scan(config.scan_options(discovered_at_ms)?)?;
    if normalized_service_type(&result.service_type) != normalized_service_type(MDNS_SERVICE_TYPE) {
        return Err(ThreadBorderAgentDiscoveryError::Validation(format!(
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
            "discarded {} Thread Border Agent advertisements beyond the configured result limit",
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
                    "Thread Border Agent {} advertises conflicting endpoints `{}` and `{}`",
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
                "invalid Thread Border Agent advertisement `{}`: {error}",
                advertisement.instance_name
            )),
        }
    }

    Ok(ThreadBorderAgentDiscoveryReport {
        records: records.into_values().collect(),
        failures,
    })
}

pub fn discover_into_runtime<T: ThreadBorderAgentMdnsTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &ThreadBorderAgentDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<ThreadBorderAgentRuntimeCommitSummary, ThreadBorderAgentDiscoveryError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(ThreadBorderAgentDiscoveryError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = ThreadBorderAgentRuntimeCommitSummary {
        failures: report.failures.len(),
        ..ThreadBorderAgentRuntimeCommitSummary::default()
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
) -> Result<DiscoveryRecord, ThreadBorderAgentDiscoveryError> {
    if normalized_service_type(&advertisement.service_type)
        != normalized_service_type(MDNS_SERVICE_TYPE)
    {
        return Err(ThreadBorderAgentDiscoveryError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    if advertisement.port == 0 {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "Border Agent UDP port must be non-zero".to_string(),
        ));
    }
    if advertisement.addresses.is_empty() {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "Border Agent advertisement must resolve at least one IP address".to_string(),
        ));
    }

    let record_version = required_text(advertisement, "rv", 7)?;
    if record_version != RECORD_VERSION {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "rv must be the supported MeshCoP TXT record version 1".to_string(),
        ));
    }
    let thread_version = required_text(advertisement, "tv", 15)?;
    let state = parse_state(required_raw(advertisement, "sb")?)?;
    let extended_address = exact_hex(required_raw(advertisement, "xa")?, 8, "xa")?;
    let agent_id = optional_raw(advertisement, "id")
        .map(|value| exact_hex(value, 16, "id"))
        .transpose()?;
    let identity = agent_id
        .as_deref()
        .map(|id| format!("border-agent-{id}"))
        .unwrap_or_else(|| format!("border-agent-xa-{extended_address}"));
    let display_name = safe_text(
        &advertisement.instance_name,
        "instance name",
        MAX_TEXT_BYTES,
    )?;
    let model = optional_text(advertisement, "mn", 31)?;

    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Thread,
        identity,
        DiscoverySource::Mdns,
        BridgeTransport::LanUdp,
        advertisement.discovered_at_ms,
    )?
    .with_display_name(display_name)
    .with_address(advertisement.endpoint_with_scheme("udp"))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE)
    .with_metadata("thread.record_version", record_version)
    .with_metadata("thread.version", thread_version)
    .with_metadata("thread.extended_address", &extended_address)
    .with_metadata("thread.state_bitmap", format!("{:08x}", state.raw))
    .with_metadata(
        "thread.connection_mode",
        connection_mode_name(state.connection_mode),
    )
    .with_metadata(
        "thread.interface_state",
        interface_state_name(state.interface_state),
    )
    .with_metadata("thread.availability", availability_name(state.availability))
    .with_metadata("thread.role", role_name(state.role))
    .with_metadata("thread.backbone_active", state.backbone_active.to_string())
    .with_metadata(
        "thread.backbone_primary",
        state.backbone_primary.to_string(),
    )
    .with_metadata("thread.epskc_supported", state.epskc_supported.to_string())
    .with_metadata(
        "thread.multi_ail_state",
        multi_ail_name(state.multi_ail_state),
    )
    .with_metadata(
        "thread.admitter_supported",
        state.admitter_supported.to_string(),
    );

    if let Some(model) = model {
        record = record
            .with_hardware_model(model.clone())
            .with_metadata("thread.vendor_model", model);
    }
    if let Some(agent_id) = agent_id {
        record = record.with_metadata("thread.border_agent_id", agent_id);
    }
    for (key, metadata_key, expected_length) in [
        ("xp", "thread.extended_pan_id", 8),
        ("at", "thread.active_timestamp", 8),
        ("vo", "thread.vendor_oui", 3),
    ] {
        if let Some(value) = optional_raw(advertisement, key) {
            record = record.with_metadata(metadata_key, exact_hex(value, expected_length, key)?);
        }
    }
    for (key, metadata_key, maximum) in [
        ("nn", "thread.network_name", 16),
        ("dn", "thread.domain_name", 16),
        ("vn", "thread.vendor_name", 31),
    ] {
        if let Some(value) = optional_text(advertisement, key, maximum)? {
            record = record.with_metadata(metadata_key, value);
        }
    }
    if let Some(value) = optional_raw(advertisement, "pt") {
        record = record.with_metadata("thread.partition_id", parse_u32(value, "pt")?.to_string());
    }
    if let Some(value) = optional_raw(advertisement, "sq") {
        record = record.with_metadata("thread.bbr_sequence", parse_u8(value, "sq")?.to_string());
    }
    if let Some(value) = optional_raw(advertisement, "bb") {
        let port = parse_u16(value, "bb")?;
        if port == 0 {
            return Err(ThreadBorderAgentDiscoveryError::Validation(
                "bb must be a non-zero UDP port".to_string(),
            ));
        }
        record = record.with_metadata("thread.bbr_port", port.to_string());
    }
    if let Some(value) = optional_raw(advertisement, "omr") {
        record = record.with_metadata("thread.omr_prefix", parse_omr_prefix(value)?);
    }
    Ok(record)
}

fn required_raw<'a>(
    advertisement: &'a MdnsAdvertisement,
    key: &str,
) -> Result<&'a [u8], ThreadBorderAgentDiscoveryError> {
    advertisement
        .raw_txt_value(key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ThreadBorderAgentDiscoveryError::Validation(format!("missing {key} TXT record"))
        })
}

fn optional_raw<'a>(advertisement: &'a MdnsAdvertisement, key: &str) -> Option<&'a [u8]> {
    advertisement
        .raw_txt_value(key)
        .filter(|value| !value.is_empty())
}

fn required_text(
    advertisement: &MdnsAdvertisement,
    key: &str,
    maximum: usize,
) -> Result<String, ThreadBorderAgentDiscoveryError> {
    raw_text(required_raw(advertisement, key)?, key, maximum)
}

fn optional_text(
    advertisement: &MdnsAdvertisement,
    key: &str,
    maximum: usize,
) -> Result<Option<String>, ThreadBorderAgentDiscoveryError> {
    optional_raw(advertisement, key)
        .map(|value| raw_text(value, key, maximum))
        .transpose()
}

fn raw_text(
    value: &[u8],
    field: &str,
    maximum: usize,
) -> Result<String, ThreadBorderAgentDiscoveryError> {
    let text = str::from_utf8(value).map_err(|_| {
        ThreadBorderAgentDiscoveryError::Validation(format!("{field} must be valid UTF-8"))
    })?;
    safe_text(text, field, maximum)
}

fn safe_text(
    value: &str,
    field: &str,
    maximum: usize,
) -> Result<String, ThreadBorderAgentDiscoveryError> {
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return Err(ThreadBorderAgentDiscoveryError::Validation(format!(
            "{field} must be non-empty bounded text without control characters"
        )));
    }
    Ok(value.to_string())
}

fn exact_hex(
    value: &[u8],
    expected_length: usize,
    field: &str,
) -> Result<String, ThreadBorderAgentDiscoveryError> {
    if value.len() != expected_length {
        return Err(ThreadBorderAgentDiscoveryError::Validation(format!(
            "{field} must contain exactly {expected_length} bytes"
        )));
    }
    Ok(hex(value))
}

fn hex(value: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(value.len() * 2);
    for byte in value {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

fn parse_state(value: &[u8]) -> Result<BorderAgentState, ThreadBorderAgentDiscoveryError> {
    let raw = parse_u32(value, "sb")?;
    let state = BorderAgentState {
        raw,
        connection_mode: (raw & 0x7) as u8,
        interface_state: ((raw >> 3) & 0x3) as u8,
        availability: ((raw >> 5) & 0x3) as u8,
        backbone_active: raw & (1 << 7) != 0,
        backbone_primary: raw & (1 << 8) != 0,
        role: ((raw >> 9) & 0x3) as u8,
        epskc_supported: raw & (1 << 11) != 0,
        multi_ail_state: ((raw >> 12) & 0x3) as u8,
        admitter_supported: raw & (1 << 14) != 0,
    };
    if state.connection_mode > 4 {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "sb contains an invalid connection mode".to_string(),
        ));
    }
    if state.interface_state > 2 {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "sb contains an invalid Thread interface state".to_string(),
        ));
    }
    if state.availability > 1 {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "sb contains an invalid availability state".to_string(),
        ));
    }
    if state.multi_ail_state > 2 {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "sb contains an invalid multi-AIL state".to_string(),
        ));
    }
    Ok(state)
}

fn parse_u8(value: &[u8], field: &str) -> Result<u8, ThreadBorderAgentDiscoveryError> {
    value
        .first()
        .copied()
        .filter(|_| value.len() == 1)
        .ok_or_else(|| {
            ThreadBorderAgentDiscoveryError::Validation(format!(
                "{field} must contain exactly one byte"
            ))
        })
}

fn parse_u16(value: &[u8], field: &str) -> Result<u16, ThreadBorderAgentDiscoveryError> {
    let bytes: [u8; 2] = value.try_into().map_err(|_| {
        ThreadBorderAgentDiscoveryError::Validation(format!(
            "{field} must contain exactly two bytes"
        ))
    })?;
    Ok(u16::from_be_bytes(bytes))
}

fn parse_u32(value: &[u8], field: &str) -> Result<u32, ThreadBorderAgentDiscoveryError> {
    let bytes: [u8; 4] = value.try_into().map_err(|_| {
        ThreadBorderAgentDiscoveryError::Validation(format!(
            "{field} must contain exactly four bytes"
        ))
    })?;
    Ok(u32::from_be_bytes(bytes))
}

fn parse_omr_prefix(value: &[u8]) -> Result<String, ThreadBorderAgentDiscoveryError> {
    let prefix_length = value.first().copied().ok_or_else(|| {
        ThreadBorderAgentDiscoveryError::Validation("omr must contain a prefix length".to_string())
    })?;
    if prefix_length > 128 {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "omr prefix length must not exceed 128 bits".to_string(),
        ));
    }
    let byte_length = usize::from(prefix_length).div_ceil(8);
    if value.len() != byte_length + 1 {
        return Err(ThreadBorderAgentDiscoveryError::Validation(
            "omr bytes must exactly match the advertised prefix length".to_string(),
        ));
    }
    let mut octets = [0u8; 16];
    octets[..byte_length].copy_from_slice(&value[1..]);
    Ok(format!("{}/{prefix_length}", Ipv6Addr::from(octets)))
}

fn connection_mode_name(value: u8) -> &'static str {
    ["disabled", "pskc", "pskd", "vendor", "x509"][value as usize]
}

fn interface_state_name(value: u8) -> &'static str {
    ["not_initialized", "initialized", "active"][value as usize]
}

fn availability_name(value: u8) -> &'static str {
    ["infrequent", "high"][value as usize]
}

fn role_name(value: u8) -> &'static str {
    ["disabled_or_detached", "child", "router", "leader"][value as usize]
}

fn multi_ail_name(value: u8) -> &'static str {
    ["disabled", "not_detected", "detected"][value as usize]
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

    const AGENT_ID: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];
    const EXT_ADDRESS: [u8; 8] = [0x02, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        result: MdnsScanResult,
    }

    impl ThreadBorderAgentMdnsTransport for FakeTransport {
        fn scan(
            &mut self,
            options: MdnsScanOptions,
        ) -> Result<MdnsScanResult, ThreadBorderAgentDiscoveryError> {
            assert_eq!(options.service_type, MDNS_SERVICE_TYPE);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn advertisement(address: &str, agent_id: &[u8]) -> MdnsAdvertisement {
        MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "OpenThread BorderRouter #6677",
            "otbr.local",
            49191,
            1_000,
        )
        .unwrap()
        .with_address(address)
        .unwrap()
        .with_txt("rv", "1")
        .unwrap()
        .with_raw_txt("id", agent_id.to_vec())
        .unwrap()
        .with_txt("tv", "1.4.0")
        .unwrap()
        .with_raw_txt("sb", 0x0000_0eb1u32.to_be_bytes().to_vec())
        .unwrap()
        .with_txt("nn", "Home Thread")
        .unwrap()
        .with_raw_txt("xp", b"ABCDEFGH".to_vec())
        .unwrap()
        .with_raw_txt("pt", 0x1234_5678u32.to_be_bytes().to_vec())
        .unwrap()
        .with_raw_txt("xa", EXT_ADDRESS.to_vec())
        .unwrap()
        .with_raw_txt("omr", vec![64, 0xfd, 0x00, 0xca, 0xfe, 0xba, 0xbe, 0, 0])
        .unwrap()
        .with_txt("vn", "OpenThread")
        .unwrap()
        .with_txt("mn", "OTBR")
        .unwrap()
        .with_raw_txt("vo", vec![0xaa, 0xbb, 0xcc])
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
                CapabilityGrantId::trusted("grant:thread-border-agent-discovery-test"),
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
        let mut config = ThreadBorderAgentDiscoveryConfig {
            timeout: Duration::ZERO,
            ..ThreadBorderAgentDiscoveryConfig::default()
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
    fn normalizes_binary_identity_state_and_topology() {
        let record = discovery_record(&advertisement("192.0.2.70", &AGENT_ID), 9_000).unwrap();
        assert_eq!(
            record.native_bridge_id,
            "border-agent-00112233445566778899aabbccddeeff"
        );
        assert_eq!(record.address.as_deref(), Some("udp://192.0.2.70:49191"));
        assert_eq!(record.hardware_model.as_deref(), Some("OTBR"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::Credentials);
        assert_eq!(record.expires_at_ms, Some(9_000));
        for (key, value) in [
            ("thread.connection_mode", "pskc"),
            ("thread.interface_state", "active"),
            ("thread.role", "leader"),
            ("thread.network_name", "Home Thread"),
            ("thread.partition_id", "305419896"),
            ("thread.omr_prefix", "fd00:cafe:babe::/64"),
        ] {
            assert!(record
                .metadata
                .iter()
                .any(|item| item.key == key && item.value == value));
        }
    }

    #[test]
    fn rejects_incomplete_or_invalid_meshcop_contract() {
        let mut missing_state = advertisement("192.0.2.70", &AGENT_ID);
        missing_state.txt.retain(|entry| entry.key != "sb");
        assert!(discovery_record(&missing_state, 1).is_err());

        let bad_id = advertisement("192.0.2.70", &AGENT_ID[..15]);
        assert!(discovery_record(&bad_id, 1).is_err());

        let mut bad_version = advertisement("192.0.2.70", &AGENT_ID);
        bad_version.txt.retain(|entry| entry.key != "rv");
        bad_version = bad_version.with_txt("rv", "2").unwrap();
        assert!(discovery_record(&bad_version, 1).is_err());

        let mut bad_state = advertisement("192.0.2.70", &AGENT_ID);
        bad_state.txt.retain(|entry| entry.key != "sb");
        bad_state = bad_state
            .with_raw_txt("sb", 0x0000_0007u32.to_be_bytes().to_vec())
            .unwrap();
        assert!(discovery_record(&bad_state, 1).is_err());

        let mut unresolved = advertisement("192.0.2.70", &AGENT_ID);
        unresolved.addresses.clear();
        assert!(discovery_record(&unresolved, 1).is_err());
    }

    #[test]
    fn deduplicates_identity_preserves_failures_and_caps_results() {
        let mut result = scan_result(vec![
            advertisement("192.0.2.70", &AGENT_ID),
            advertisement("192.0.2.71", &AGENT_ID),
            advertisement("192.0.2.72", &[0x44; 16]),
        ]);
        result.failures.push(MdnsScanFailure {
            source: Some("192.0.2.99:5353".to_string()),
            message: "truncated DNS packet".to_string(),
        });
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            result,
        };
        let report = discover(
            &ThreadBorderAgentDiscoveryConfig {
                maximum_responses: 2,
                ..ThreadBorderAgentDiscoveryConfig::default()
            },
            &mut transport,
            1_000,
        )
        .unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].address.as_deref(),
            Some("udp://192.0.2.70:49191")
        );
        assert_eq!(report.failures.len(), 3);
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
            &ThreadBorderAgentDiscoveryConfig::default(),
            &mut transport,
            1_000,
        );
        assert!(matches!(
            result,
            Err(ThreadBorderAgentDiscoveryError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn live_loopback_packet_preserves_binary_txt_and_commits() {
        struct LoopbackTransport;
        impl ThreadBorderAgentMdnsTransport for LoopbackTransport {
            fn scan(
                &mut self,
                options: MdnsScanOptions,
            ) -> Result<MdnsScanResult, ThreadBorderAgentDiscoveryError> {
                let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
                receiver
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
                sender
                    .send_to(
                        &meshcop_mdns_response_packet(),
                        receiver.local_addr().unwrap(),
                    )
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

        let principal = AgentId::trusted("agent:thread-border-agent-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &ThreadBorderAgentDiscoveryConfig::default(),
            &mut LoopbackTransport,
            2_000,
        )
        .unwrap();
        assert_eq!(summary.inserted, 1, "{summary:?}");
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "thread_border_agent.bridge.border-agent-00112233445566778899aabbccddeeff"
            ))
            .is_some());
    }

    fn meshcop_mdns_response_packet() -> Vec<u8> {
        let mut packet = Vec::new();
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0x8400);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 1);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 3);

        let instance = "OpenThread BorderRouter #6677._meshcop._udp.local";
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
        push_u16(&mut packet, 49191);
        encode_name("otbr.local", &mut packet);
        fill_length(&mut packet, srv_length);

        encode_name(instance, &mut packet);
        push_record_header(&mut packet, 16);
        let txt_length = reserve_length(&mut packet);
        for (key, value) in [("rv", b"1".as_slice()), ("tv", b"1.4.0".as_slice())] {
            push_raw_txt(&mut packet, key, value);
        }
        push_raw_txt(&mut packet, "id", &AGENT_ID);
        push_raw_txt(&mut packet, "sb", &0x0000_0eb1u32.to_be_bytes());
        push_raw_txt(&mut packet, "xa", &EXT_ADDRESS);
        push_raw_txt(&mut packet, "nn", b"Home Thread");
        fill_length(&mut packet, txt_length);

        encode_name("otbr.local", &mut packet);
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

    fn push_raw_txt(packet: &mut Vec<u8>, key: &str, value: &[u8]) {
        let length = key.len() + 1 + value.len();
        packet.push(length as u8);
        packet.extend_from_slice(key.as_bytes());
        packet.push(b'=');
        packet.extend_from_slice(value);
    }

    fn push_u16(packet: &mut Vec<u8>, value: u16) {
        packet.extend_from_slice(&value.to_be_bytes());
    }
}
