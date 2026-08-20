//! Authorized bounded HomeKit Accessory Protocol mDNS discovery for D23.

#![forbid(unsafe_code)]

use smart_home_core::{AgentId, BridgeTransport, IntegrationId, ProtocolFamily, SmartHomeTool};
use smart_home_discovery::{
    run_mdns_ipv4_scan, DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource,
    DiscoveryUpsert, MdnsAdvertisement, MdnsScanOptions, MdnsScanResult, PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "homekit_controller";
pub const PROTOCOL_ID: &str = "homekit_hap_ip";
pub const MDNS_SERVICE_TYPE: &str = "_hap._tcp.local";
pub const MAX_RESPONSES: usize = 64;
const MAX_TXT_VALUE_BYTES: usize = 256;
const STATUS_FLAG_NOT_PAIRED: u8 = 1 << 0;

#[derive(Debug)]
pub enum HomeKitDiscoveryError {
    Validation(String),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for HomeKitDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid HomeKit discovery: {message}"),
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for HomeKitDiscoveryError {}

impl From<DiscoveryError> for HomeKitDiscoveryError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for HomeKitDiscoveryError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeKitDiscoveryConfig {
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl Default for HomeKitDiscoveryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }
}

impl HomeKitDiscoveryConfig {
    pub fn validate(&self) -> Result<(), HomeKitDiscoveryError> {
        if self.timeout.is_zero() {
            return Err(HomeKitDiscoveryError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(HomeKitDiscoveryError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(HomeKitDiscoveryError::Validation(
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

pub trait HomeKitMdnsTransport {
    fn scan(&mut self, options: MdnsScanOptions) -> Result<MdnsScanResult, HomeKitDiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpHomeKitMdnsTransport;

impl HomeKitMdnsTransport for UdpHomeKitMdnsTransport {
    fn scan(&mut self, options: MdnsScanOptions) -> Result<MdnsScanResult, HomeKitDiscoveryError> {
        Ok(run_mdns_ipv4_scan(options)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeKitDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HomeKitRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: HomeKitMdnsTransport>(
    config: &HomeKitDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<HomeKitDiscoveryReport, HomeKitDiscoveryError> {
    config.validate()?;
    let result = transport.scan(config.scan_options(discovered_at_ms)?)?;
    if normalized_service_type(&result.service_type) != normalized_service_type(MDNS_SERVICE_TYPE) {
        return Err(HomeKitDiscoveryError::Validation(format!(
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
                    "HomeKit accessory {} advertises conflicting endpoints `{}` and `{}`",
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
                "invalid HomeKit advertisement `{}`: {error}",
                advertisement.instance_name
            )),
        }
    }

    Ok(HomeKitDiscoveryReport {
        records: records.into_values().collect(),
        failures,
    })
}

pub fn discover_into_runtime<T: HomeKitMdnsTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &HomeKitDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<HomeKitRuntimeCommitSummary, HomeKitDiscoveryError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(HomeKitDiscoveryError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = HomeKitRuntimeCommitSummary {
        failures: report.failures.len(),
        ..HomeKitRuntimeCommitSummary::default()
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
) -> Result<DiscoveryRecord, HomeKitDiscoveryError> {
    if normalized_service_type(&advertisement.service_type)
        != normalized_service_type(MDNS_SERVICE_TYPE)
    {
        return Err(HomeKitDiscoveryError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    if advertisement.port == 0 {
        return Err(HomeKitDiscoveryError::Validation(
            "HAP IP port must be non-zero".to_string(),
        ));
    }
    if advertisement.addresses.is_empty() {
        return Err(HomeKitDiscoveryError::Validation(
            "HAP IP advertisement must resolve at least one IP address".to_string(),
        ));
    }

    let device_id = normalize_device_id(required_txt(advertisement, "id")?)?;
    let configuration_number = parse_u16(required_txt(advertisement, "c#")?, "c#", false)?;
    let pairing_features = parse_u8(required_txt(advertisement, "ff")?, "ff")?;
    let model = safe_text(required_txt(advertisement, "md")?, "md")?;
    let protocol_version = parse_protocol_version(required_txt(advertisement, "pv")?)?;
    let state_number = parse_u8(required_txt(advertisement, "s#")?, "s#")?;
    if state_number != 1 {
        return Err(HomeKitDiscoveryError::Validation(
            "s# must be 1 for HAP IP".to_string(),
        ));
    }
    let status_flags = parse_u8(required_txt(advertisement, "sf")?, "sf")?;
    let category = parse_u16(required_txt(advertisement, "ci")?, "ci", false)?;
    let setup_hash = advertisement
        .txt_value("sh")
        .filter(|value| !value.is_empty())
        .map(validate_setup_hash)
        .transpose()?;
    let display_name = safe_text(&advertisement.instance_name, "instance name")?;

    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        format!("accessory-{device_id}"),
        DiscoverySource::Mdns,
        BridgeTransport::LanTcp,
        advertisement.discovered_at_ms,
    )?
    .with_display_name(display_name)
    .with_hardware_model(model)
    .with_address(advertisement.endpoint_with_scheme("tcp"))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::LocalCode)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE)
    .with_metadata("homekit.device_id", &device_id)
    .with_metadata(
        "homekit.configuration_number",
        configuration_number.to_string(),
    )
    .with_metadata("homekit.pairing_features", pairing_features.to_string())
    .with_metadata("homekit.protocol_version", protocol_version)
    .with_metadata("homekit.state_number", state_number.to_string())
    .with_metadata("homekit.status_flags", status_flags.to_string())
    .with_metadata("homekit.category", category.to_string())
    .with_metadata(
        "homekit.paired",
        (status_flags & STATUS_FLAG_NOT_PAIRED == 0).to_string(),
    );
    if let Some(setup_hash) = setup_hash {
        record = record.with_metadata("homekit.setup_hash", setup_hash);
    }
    Ok(record)
}

fn required_txt<'a>(
    advertisement: &'a MdnsAdvertisement,
    key: &str,
) -> Result<&'a str, HomeKitDiscoveryError> {
    advertisement
        .txt_value(key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| HomeKitDiscoveryError::Validation(format!("missing {key} TXT record")))
}

fn safe_text(value: &str, field: &str) -> Result<String, HomeKitDiscoveryError> {
    if value.is_empty() || value.len() > MAX_TXT_VALUE_BYTES || value.chars().any(char::is_control)
    {
        return Err(HomeKitDiscoveryError::Validation(format!(
            "{field} must be non-empty bounded text without control characters"
        )));
    }
    Ok(value.to_string())
}

fn normalize_device_id(value: &str) -> Result<String, HomeKitDiscoveryError> {
    if value.len() != 17 {
        return Err(HomeKitDiscoveryError::Validation(
            "id must contain six colon-separated hexadecimal octets".to_string(),
        ));
    }
    let mut normalized = String::with_capacity(12);
    for (index, byte) in value.bytes().enumerate() {
        if matches!(index, 2 | 5 | 8 | 11 | 14) {
            if byte != b':' {
                return Err(HomeKitDiscoveryError::Validation(
                    "id must contain six colon-separated hexadecimal octets".to_string(),
                ));
            }
        } else if byte.is_ascii_hexdigit() {
            normalized.push((byte as char).to_ascii_lowercase());
        } else {
            return Err(HomeKitDiscoveryError::Validation(
                "id must contain six colon-separated hexadecimal octets".to_string(),
            ));
        }
    }
    Ok(normalized)
}

fn parse_u8(value: &str, field: &str) -> Result<u8, HomeKitDiscoveryError> {
    parse_decimal(value, field, u8::MAX.into()).map(|value| value as u8)
}

fn parse_u16(value: &str, field: &str, allow_zero: bool) -> Result<u16, HomeKitDiscoveryError> {
    let value = parse_decimal(value, field, u16::MAX.into())? as u16;
    if !allow_zero && value == 0 {
        return Err(HomeKitDiscoveryError::Validation(format!(
            "{field} must be non-zero"
        )));
    }
    Ok(value)
}

fn parse_decimal(value: &str, field: &str, maximum: u64) -> Result<u64, HomeKitDiscoveryError> {
    if value.is_empty() || value.len() > 5 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(HomeKitDiscoveryError::Validation(format!(
            "{field} must be a bounded unsigned decimal integer"
        )));
    }
    let parsed = value.parse::<u64>().map_err(|_| {
        HomeKitDiscoveryError::Validation(format!("{field} is outside its unsigned range"))
    })?;
    if parsed > maximum {
        return Err(HomeKitDiscoveryError::Validation(format!(
            "{field} is outside its unsigned range"
        )));
    }
    Ok(parsed)
}

fn parse_protocol_version(value: &str) -> Result<String, HomeKitDiscoveryError> {
    if value.len() > 5 {
        return Err(HomeKitDiscoveryError::Validation(
            "pv must be a bounded major.minor version".to_string(),
        ));
    }
    let mut parts = value.split('.');
    let major = parts.next().unwrap_or_default();
    let minor = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || major.is_empty()
        || minor.is_empty()
        || major.len() > 2
        || minor.len() > 2
        || !major.bytes().all(|byte| byte.is_ascii_digit())
        || !minor.bytes().all(|byte| byte.is_ascii_digit())
        || major
            .parse::<u8>()
            .ok()
            .filter(|value| *value > 0)
            .is_none()
        || minor.parse::<u8>().is_err()
    {
        return Err(HomeKitDiscoveryError::Validation(
            "pv must be a bounded major.minor version with a non-zero major".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn validate_setup_hash(value: &str) -> Result<String, HomeKitDiscoveryError> {
    let bytes = value.as_bytes();
    if bytes.len() != 8
        || !bytes[..5]
            .iter()
            .copied()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
        || !matches!(bytes[5], b'A' | b'Q' | b'g' | b'w')
        || bytes[6..] != *b"=="
    {
        return Err(HomeKitDiscoveryError::Validation(
            "sh must be an eight-character Base64 setup hash".to_string(),
        ));
    }
    Ok(value.to_string())
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

    const DEVICE_ID: &str = "AA:BB:CC:DD:EE:FF";

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        result: MdnsScanResult,
    }

    impl HomeKitMdnsTransport for FakeTransport {
        fn scan(
            &mut self,
            options: MdnsScanOptions,
        ) -> Result<MdnsScanResult, HomeKitDiscoveryError> {
            assert_eq!(options.service_type, MDNS_SERVICE_TYPE);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn advertisement(address: &str, device_id: &str) -> MdnsAdvertisement {
        MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Living Room Bridge",
            "living-room.local",
            51826,
            1_000,
        )
        .unwrap()
        .with_address(address)
        .unwrap()
        .with_txt("c#", "7")
        .unwrap()
        .with_txt("ff", "0")
        .unwrap()
        .with_txt("id", device_id)
        .unwrap()
        .with_txt("md", "HomeKit Bridge")
        .unwrap()
        .with_txt("pv", "1.1")
        .unwrap()
        .with_txt("s#", "1")
        .unwrap()
        .with_txt("sf", "1")
        .unwrap()
        .with_txt("ci", "2")
        .unwrap()
        .with_txt("sh", "qFV4yg==")
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
                CapabilityGrantId::trusted("grant:homekit-discovery-test"),
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
        let mut config = HomeKitDiscoveryConfig {
            timeout: Duration::ZERO,
            ..HomeKitDiscoveryConfig::default()
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
    fn normalizes_accessory_identity_and_pairing_contract() {
        let record = discovery_record(&advertisement("192.0.2.50", DEVICE_ID), 9_000).unwrap();
        assert_eq!(record.native_bridge_id, "accessory-aabbccddeeff");
        assert_eq!(record.address.as_deref(), Some("tcp://192.0.2.50:51826"));
        assert_eq!(record.display_name.as_deref(), Some("Living Room Bridge"));
        assert_eq!(record.hardware_model.as_deref(), Some("HomeKit Bridge"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::LocalCode);
        assert_eq!(record.expires_at_ms, Some(9_000));
        assert!(record
            .metadata
            .iter()
            .any(|item| { item.key == "homekit.configuration_number" && item.value == "7" }));
        assert!(record
            .metadata
            .iter()
            .any(|item| item.key == "homekit.paired" && item.value == "false"));
        assert!(record
            .metadata
            .iter()
            .any(|item| item.key == "homekit.setup_hash" && item.value == "qFV4yg=="));
    }

    #[test]
    fn rejects_incomplete_or_invalid_hap_contract() {
        let mut missing_id = advertisement("192.0.2.50", DEVICE_ID);
        missing_id.txt.retain(|entry| entry.key != "id");
        assert!(discovery_record(&missing_id, 1).is_err());

        assert!(discovery_record(&advertisement("192.0.2.50", "AABBCCDDEEFF"), 1).is_err());

        let mut invalid_state = advertisement("192.0.2.50", DEVICE_ID);
        invalid_state.txt.retain(|entry| entry.key != "s#");
        invalid_state = invalid_state.with_txt("s#", "2").unwrap();
        assert!(discovery_record(&invalid_state, 1).is_err());

        let mut invalid_hash = advertisement("192.0.2.50", DEVICE_ID);
        invalid_hash.txt.retain(|entry| entry.key != "sh");
        invalid_hash = invalid_hash.with_txt("sh", "not-base").unwrap();
        assert!(discovery_record(&invalid_hash, 1).is_err());

        let mut noncanonical_hash = advertisement("192.0.2.50", DEVICE_ID);
        noncanonical_hash.txt.retain(|entry| entry.key != "sh");
        noncanonical_hash = noncanonical_hash.with_txt("sh", "qFV4yh==").unwrap();
        assert!(discovery_record(&noncanonical_hash, 1).is_err());

        let mut unicode_hash = advertisement("192.0.2.50", DEVICE_ID);
        unicode_hash.txt.retain(|entry| entry.key != "sh");
        unicode_hash = unicode_hash
            .with_txt("sh", "\u{00e9}\u{00e9}\u{00e9}\u{00e9}")
            .unwrap();
        assert!(discovery_record(&unicode_hash, 1).is_err());

        let mut unresolved = advertisement("192.0.2.50", DEVICE_ID);
        unresolved.addresses.clear();
        assert!(discovery_record(&unresolved, 1).is_err());
    }

    #[test]
    fn deduplicates_stable_identity_and_preserves_partial_failures() {
        let mut result = scan_result(vec![
            advertisement("192.0.2.50", DEVICE_ID),
            advertisement("192.0.2.51", DEVICE_ID),
        ]);
        result.failures.push(MdnsScanFailure {
            source: Some("192.0.2.99:5353".to_string()),
            message: "truncated DNS packet".to_string(),
        });
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            result,
        };
        let report = discover(&HomeKitDiscoveryConfig::default(), &mut transport, 1_000).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].address.as_deref(),
            Some("tcp://192.0.2.50:51826")
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
            &HomeKitDiscoveryConfig::default(),
            &mut transport,
            1_000,
        );
        assert!(matches!(
            result,
            Err(HomeKitDiscoveryError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn live_loopback_packet_is_parsed_and_committed() {
        struct LoopbackTransport;
        impl HomeKitMdnsTransport for LoopbackTransport {
            fn scan(
                &mut self,
                options: MdnsScanOptions,
            ) -> Result<MdnsScanResult, HomeKitDiscoveryError> {
                let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
                receiver
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
                sender
                    .send_to(
                        &homekit_mdns_response_packet(),
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

        let principal = AgentId::trusted("agent:homekit-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &HomeKitDiscoveryConfig::default(),
            &mut LoopbackTransport,
            2_000,
        )
        .unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "homekit_controller.bridge.accessory-aabbccddeeff"
            ))
            .is_some());
    }

    fn homekit_mdns_response_packet() -> Vec<u8> {
        let mut packet = Vec::new();
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0x8400);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 1);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 3);

        let instance = "Living Room Bridge._hap._tcp.local";
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
        push_u16(&mut packet, 51826);
        encode_name("living-room.local", &mut packet);
        fill_length(&mut packet, srv_length);

        encode_name(instance, &mut packet);
        push_record_header(&mut packet, 16);
        let txt_length = reserve_length(&mut packet);
        for (key, value) in [
            ("c#", "7"),
            ("ff", "0"),
            ("id", DEVICE_ID),
            ("md", "HomeKit Bridge"),
            ("pv", "1.1"),
            ("s#", "1"),
            ("sf", "1"),
            ("ci", "2"),
            ("sh", "qFV4yg=="),
        ] {
            push_txt(&mut packet, key, value);
        }
        fill_length(&mut packet, txt_length);

        encode_name("living-room.local", &mut packet);
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
