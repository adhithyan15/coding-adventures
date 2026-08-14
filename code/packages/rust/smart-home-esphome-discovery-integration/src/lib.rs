//! Authorized bounded ESPHome native-API mDNS discovery for D23.

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
pub const INTEGRATION_ID: &str = "esphome";
pub const PROTOCOL_ID: &str = "esphome_native_api";
pub const MDNS_SERVICE_TYPE: &str = "_esphomelib._tcp.local";
pub const NOISE_PROTOCOL: &str = "Noise_NNpsk0_25519_ChaChaPoly_SHA256";
pub const MAX_RESPONSES: usize = 64;
const MAX_TXT_VALUE_BYTES: usize = 256;

#[derive(Debug)]
pub enum EspHomeDiscoveryError {
    Validation(String),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for EspHomeDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid ESPHome discovery: {message}"),
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EspHomeDiscoveryError {}

impl From<DiscoveryError> for EspHomeDiscoveryError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for EspHomeDiscoveryError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspHomeDiscoveryConfig {
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl Default for EspHomeDiscoveryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }
}

impl EspHomeDiscoveryConfig {
    pub fn validate(&self) -> Result<(), EspHomeDiscoveryError> {
        if self.timeout.is_zero() {
            return Err(EspHomeDiscoveryError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(EspHomeDiscoveryError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(EspHomeDiscoveryError::Validation(
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

pub trait EspHomeMdnsTransport {
    fn scan(&mut self, options: MdnsScanOptions) -> Result<MdnsScanResult, EspHomeDiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpEspHomeMdnsTransport;

impl EspHomeMdnsTransport for UdpEspHomeMdnsTransport {
    fn scan(&mut self, options: MdnsScanOptions) -> Result<MdnsScanResult, EspHomeDiscoveryError> {
        Ok(run_mdns_ipv4_scan(options)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspHomeDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EspHomeRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: EspHomeMdnsTransport>(
    config: &EspHomeDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<EspHomeDiscoveryReport, EspHomeDiscoveryError> {
    config.validate()?;
    let result = transport.scan(config.scan_options(discovered_at_ms)?)?;
    if normalized_service_type(&result.service_type) != normalized_service_type(MDNS_SERVICE_TYPE) {
        return Err(EspHomeDiscoveryError::Validation(format!(
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
                    "ESPHome identity {} advertises conflicting endpoints `{}` and `{}`",
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
                "invalid ESPHome advertisement `{}`: {error}",
                advertisement.instance_name
            )),
        }
    }

    Ok(EspHomeDiscoveryReport {
        records: records.into_values().collect(),
        failures,
    })
}

pub fn discover_into_runtime<T: EspHomeMdnsTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &EspHomeDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<EspHomeRuntimeCommitSummary, EspHomeDiscoveryError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(EspHomeDiscoveryError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = EspHomeRuntimeCommitSummary {
        failures: report.failures.len(),
        ..EspHomeRuntimeCommitSummary::default()
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
) -> Result<DiscoveryRecord, EspHomeDiscoveryError> {
    if normalized_service_type(&advertisement.service_type)
        != normalized_service_type(MDNS_SERVICE_TYPE)
    {
        return Err(EspHomeDiscoveryError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    if advertisement.port == 0 {
        return Err(EspHomeDiscoveryError::Validation(
            "native API port must be non-zero".to_string(),
        ));
    }

    let mac = normalize_mac(required_txt(advertisement, "mac")?)?;
    let version = safe_txt(required_txt(advertisement, "version")?, "version")?;
    let board = safe_txt(required_txt(advertisement, "board")?, "board")?;
    let config_hash = required_txt(advertisement, "config_hash")?;
    if config_hash.len() > 64 || !config_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(EspHomeDiscoveryError::Validation(
            "config_hash must be 1 to 64 hexadecimal characters".to_string(),
        ));
    }

    let encryption = advertisement.txt_value("api_encryption");
    let encryption_supported = advertisement.txt_value("api_encryption_supported");
    if encryption.is_some() && encryption_supported.is_some() {
        return Err(EspHomeDiscoveryError::Validation(
            "api_encryption and api_encryption_supported are mutually exclusive".to_string(),
        ));
    }
    for value in [encryption, encryption_supported].into_iter().flatten() {
        if value != NOISE_PROTOCOL {
            return Err(EspHomeDiscoveryError::Validation(
                "advertised native API encryption suite is unsupported".to_string(),
            ));
        }
    }
    let provisioning = advertisement.txt_value("api_provisioning");
    if provisioning.is_some_and(|value| value != "zero-psk") {
        return Err(EspHomeDiscoveryError::Validation(
            "advertised native API provisioning mode is unsupported".to_string(),
        ));
    }
    if provisioning.is_some() && encryption_supported.is_none() {
        return Err(EspHomeDiscoveryError::Validation(
            "zero-PSK provisioning requires advertised Noise support".to_string(),
        ));
    }

    let project_name = advertisement.txt_value("project_name");
    let project_version = advertisement.txt_value("project_version");
    if project_name.is_some() != project_version.is_some() {
        return Err(EspHomeDiscoveryError::Validation(
            "project_name and project_version must be advertised together".to_string(),
        ));
    }
    let friendly_name = advertisement
        .txt_value("friendly_name")
        .filter(|value| !value.is_empty())
        .map(|value| safe_txt(value, "friendly_name"))
        .transpose()?
        .unwrap_or_else(|| advertisement.instance_name.clone());
    safe_txt(&friendly_name, "instance_name")?;

    let security = if encryption.is_some() {
        "noise_psk"
    } else if encryption_supported.is_some() {
        "noise_supported"
    } else {
        "legacy_or_unadvertised"
    };
    let pairing_requirement = if encryption.is_some() || encryption_supported.is_some() {
        PairingRequirement::Credentials
    } else {
        PairingRequirement::Unknown
    };

    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        format!("mac-{mac}"),
        DiscoverySource::Mdns,
        BridgeTransport::LanTcp,
        advertisement.discovered_at_ms,
    )?
    .with_display_name(friendly_name)
    .with_address(advertisement.endpoint_with_scheme("tcp"))
    .with_hardware_model(board)
    .with_firmware_version(version)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(pairing_requirement)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE)
    .with_metadata("esphome.mac", mac)
    .with_metadata("esphome.config_hash", config_hash)
    .with_metadata("esphome.api_security", security);

    if let Some(platform) = advertisement.txt_value("platform") {
        record = record.with_metadata("esphome.platform", safe_txt(platform, "platform")?);
    }
    if let Some(network) = advertisement.txt_value("network") {
        let network = safe_txt(network, "network")?;
        if !["wifi", "ethernet", "thread"].contains(&network.as_str()) {
            return Err(EspHomeDiscoveryError::Validation(format!(
                "unsupported network value `{network}`"
            )));
        }
        record = record.with_metadata("esphome.network", network);
    }
    if let (Some(name), Some(version)) = (project_name, project_version) {
        record = record
            .with_metadata("esphome.project_name", safe_txt(name, "project_name")?)
            .with_metadata(
                "esphome.project_version",
                safe_txt(version, "project_version")?,
            );
    }
    if provisioning.is_some() {
        record = record.with_metadata("esphome.api_provisioning", "zero-psk");
    }
    Ok(record)
}

fn required_txt<'a>(
    advertisement: &'a MdnsAdvertisement,
    key: &str,
) -> Result<&'a str, EspHomeDiscoveryError> {
    advertisement
        .txt_value(key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| EspHomeDiscoveryError::Validation(format!("missing {key} TXT record")))
}

fn safe_txt(value: &str, field: &str) -> Result<String, EspHomeDiscoveryError> {
    if value.is_empty()
        || value.len() > MAX_TXT_VALUE_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii() && !byte.is_ascii_control())
    {
        return Err(EspHomeDiscoveryError::Validation(format!(
            "{field} must be non-empty bounded printable ASCII"
        )));
    }
    Ok(value.to_string())
}

fn normalize_mac(value: &str) -> Result<String, EspHomeDiscoveryError> {
    let mut normalized = String::with_capacity(12);
    for byte in value.bytes() {
        match byte {
            b':' | b'-' => {}
            byte if byte.is_ascii_hexdigit() => {
                normalized.push((byte as char).to_ascii_lowercase())
            }
            _ => {
                return Err(EspHomeDiscoveryError::Validation(
                    "mac must contain exactly 12 hexadecimal digits".to_string(),
                ))
            }
        }
    }
    if normalized.len() != 12 {
        return Err(EspHomeDiscoveryError::Validation(
            "mac must contain exactly 12 hexadecimal digits".to_string(),
        ));
    }
    Ok(normalized)
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

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        result: MdnsScanResult,
    }

    impl EspHomeMdnsTransport for FakeTransport {
        fn scan(
            &mut self,
            options: MdnsScanOptions,
        ) -> Result<MdnsScanResult, EspHomeDiscoveryError> {
            assert_eq!(options.service_type, MDNS_SERVICE_TYPE);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn advertisement(address: &str, mac: &str) -> MdnsAdvertisement {
        MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Kitchen Sensor",
            "kitchen-sensor.local",
            6053,
            1_000,
        )
        .unwrap()
        .with_address(address)
        .unwrap()
        .with_txt("version", "2026.7.0")
        .unwrap()
        .with_txt("config_hash", "A1B2C3D4")
        .unwrap()
        .with_txt("mac", mac)
        .unwrap()
        .with_txt("board", "esp32dev")
        .unwrap()
        .with_txt("platform", "ESP32")
        .unwrap()
        .with_txt("network", "wifi")
        .unwrap()
        .with_txt("api_encryption", NOISE_PROTOCOL)
        .unwrap()
        .with_txt("friendly_name", "Kitchen Sensor")
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
                CapabilityGrantId::trusted("grant:esphome-discovery-test"),
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
        let mut config = EspHomeDiscoveryConfig {
            timeout: Duration::ZERO,
            ..EspHomeDiscoveryConfig::default()
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
    fn normalizes_identity_and_noise_metadata() {
        let record =
            discovery_record(&advertisement("192.0.2.40", "AA:BB:CC:DD:EE:FF"), 9_000).unwrap();
        assert_eq!(record.native_bridge_id, "mac-aabbccddeeff");
        assert_eq!(record.address.as_deref(), Some("tcp://192.0.2.40:6053"));
        assert_eq!(record.hardware_model.as_deref(), Some("esp32dev"));
        assert_eq!(record.firmware_version.as_deref(), Some("2026.7.0"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::Credentials);
        assert_eq!(record.expires_at_ms, Some(9_000));
        assert!(record
            .metadata
            .iter()
            .any(|item| { item.key == "esphome.api_security" && item.value == "noise_psk" }));
    }

    #[test]
    fn rejects_incomplete_identity_and_inconsistent_security() {
        let missing_mac = MdnsAdvertisement::new(MDNS_SERVICE_TYPE, "Node", "node.local", 6053, 0)
            .unwrap()
            .with_txt("version", "2026.7.0")
            .unwrap()
            .with_txt("config_hash", "1234")
            .unwrap()
            .with_txt("board", "esp32")
            .unwrap();
        assert!(discovery_record(&missing_mac, 1).is_err());

        let inconsistent = advertisement("192.0.2.40", "AABBCCDDEEFF")
            .with_txt("api_encryption_supported", NOISE_PROTOCOL)
            .unwrap();
        assert!(discovery_record(&inconsistent, 1).is_err());
    }

    #[test]
    fn deduplicates_stable_identity_and_preserves_partial_failures() {
        let mut result = scan_result(vec![
            advertisement("192.0.2.40", "AABBCCDDEEFF"),
            advertisement("192.0.2.41", "AABBCCDDEEFF"),
        ]);
        result.failures.push(MdnsScanFailure {
            source: Some("192.0.2.99:5353".to_string()),
            message: "truncated DNS packet".to_string(),
        });
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            result,
        };
        let report = discover(&EspHomeDiscoveryConfig::default(), &mut transport, 1_000).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].address.as_deref(),
            Some("tcp://192.0.2.40:6053")
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
            &EspHomeDiscoveryConfig::default(),
            &mut transport,
            1_000,
        );
        assert!(matches!(
            result,
            Err(EspHomeDiscoveryError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn live_loopback_packet_is_parsed_and_committed() {
        struct LoopbackTransport;
        impl EspHomeMdnsTransport for LoopbackTransport {
            fn scan(
                &mut self,
                options: MdnsScanOptions,
            ) -> Result<MdnsScanResult, EspHomeDiscoveryError> {
                let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
                receiver
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
                sender
                    .send_to(
                        &esphome_mdns_response_packet(),
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

        let principal = AgentId::trusted("agent:esphome-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &EspHomeDiscoveryConfig::default(),
            &mut LoopbackTransport,
            2_000,
        )
        .unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "esphome.bridge.mac-aabbccddeeff"
            ))
            .is_some());
    }

    fn esphome_mdns_response_packet() -> Vec<u8> {
        let mut packet = Vec::new();
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0x8400);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 1);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 3);

        encode_name(MDNS_SERVICE_TYPE, &mut packet);
        push_record_header(&mut packet, 12);
        let ptr_length = reserve_length(&mut packet);
        encode_name("Kitchen Sensor._esphomelib._tcp.local", &mut packet);
        fill_length(&mut packet, ptr_length);

        encode_name("Kitchen Sensor._esphomelib._tcp.local", &mut packet);
        push_record_header(&mut packet, 33);
        let srv_length = reserve_length(&mut packet);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 6053);
        encode_name("kitchen-sensor.local", &mut packet);
        fill_length(&mut packet, srv_length);

        encode_name("Kitchen Sensor._esphomelib._tcp.local", &mut packet);
        push_record_header(&mut packet, 16);
        let txt_length = reserve_length(&mut packet);
        for (key, value) in [
            ("version", "2026.7.0"),
            ("config_hash", "A1B2C3D4"),
            ("mac", "AABBCCDDEEFF"),
            ("board", "esp32dev"),
            ("platform", "ESP32"),
            ("network", "wifi"),
            ("api_encryption", NOISE_PROTOCOL),
            ("friendly_name", "Kitchen Sensor"),
        ] {
            push_txt(&mut packet, key, value);
        }
        fill_length(&mut packet, txt_length);

        encode_name("kitchen-sensor.local", &mut packet);
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
