//! Authorized bounded Google Cast mDNS discovery for D23.

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
pub const INTEGRATION_ID: &str = "cast";
pub const PROTOCOL_ID: &str = "google_cast_v2";
pub const MDNS_SERVICE_TYPE: &str = "_googlecast._tcp.local";
pub const MAX_RESPONSES: usize = 64;
const MAX_TXT_VALUE_BYTES: usize = 256;
const MIN_PROTOCOL_VERSION: u8 = 2;
const MAX_PROTOCOL_VERSION: u8 = 99;
const CAPABILITY_VIDEO_OUTPUT: u64 = 1 << 0;
const CAPABILITY_VIDEO_INPUT: u64 = 1 << 1;
const CAPABILITY_AUDIO_OUTPUT: u64 = 1 << 2;
const CAPABILITY_AUDIO_INPUT: u64 = 1 << 3;
const CAPABILITY_DEVELOPER_MODE: u64 = 1 << 4;

#[derive(Debug)]
pub enum GoogleCastDiscoveryError {
    Validation(String),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for GoogleCastDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(formatter, "invalid Google Cast discovery: {message}")
            }
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GoogleCastDiscoveryError {}

impl From<DiscoveryError> for GoogleCastDiscoveryError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for GoogleCastDiscoveryError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleCastDiscoveryConfig {
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl Default for GoogleCastDiscoveryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }
}

impl GoogleCastDiscoveryConfig {
    pub fn validate(&self) -> Result<(), GoogleCastDiscoveryError> {
        if self.timeout.is_zero() {
            return Err(GoogleCastDiscoveryError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(GoogleCastDiscoveryError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(GoogleCastDiscoveryError::Validation(
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

pub trait GoogleCastMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, GoogleCastDiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpGoogleCastMdnsTransport;

impl GoogleCastMdnsTransport for UdpGoogleCastMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, GoogleCastDiscoveryError> {
        Ok(run_mdns_ipv4_scan(options)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleCastDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GoogleCastRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: GoogleCastMdnsTransport>(
    config: &GoogleCastDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<GoogleCastDiscoveryReport, GoogleCastDiscoveryError> {
    config.validate()?;
    let result = transport.scan(config.scan_options(discovered_at_ms)?)?;
    if normalized_service_type(&result.service_type) != normalized_service_type(MDNS_SERVICE_TYPE) {
        return Err(GoogleCastDiscoveryError::Validation(format!(
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
                    "Google Cast receiver {} advertises conflicting endpoints `{}` and `{}`",
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
                "invalid Google Cast advertisement `{}`: {error}",
                advertisement.instance_name
            )),
        }
    }

    Ok(GoogleCastDiscoveryReport {
        records: records.into_values().collect(),
        failures,
    })
}

pub fn discover_into_runtime<T: GoogleCastMdnsTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &GoogleCastDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<GoogleCastRuntimeCommitSummary, GoogleCastDiscoveryError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(GoogleCastDiscoveryError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = GoogleCastRuntimeCommitSummary {
        failures: report.failures.len(),
        ..GoogleCastRuntimeCommitSummary::default()
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
) -> Result<DiscoveryRecord, GoogleCastDiscoveryError> {
    if normalized_service_type(&advertisement.service_type)
        != normalized_service_type(MDNS_SERVICE_TYPE)
    {
        return Err(GoogleCastDiscoveryError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    if advertisement.port == 0 {
        return Err(GoogleCastDiscoveryError::Validation(
            "CastV2 port must be non-zero".to_string(),
        ));
    }
    if advertisement.addresses.is_empty() {
        return Err(GoogleCastDiscoveryError::Validation(
            "CastV2 advertisement must resolve at least one IP address".to_string(),
        ));
    }

    let receiver_id = normalize_receiver_id(required_txt(advertisement, "id")?)?;
    let protocol_version = parse_protocol_version(required_txt(advertisement, "ve")?)?;
    let capabilities = parse_capabilities(required_txt(advertisement, "ca")?)?;
    let status = match required_txt(advertisement, "st")? {
        "0" => "idle",
        "1" => "busy",
        _ => {
            return Err(GoogleCastDiscoveryError::Validation(
                "st must be 0 (idle) or 1 (busy)".to_string(),
            ))
        }
    };
    let friendly_name = safe_text(required_txt(advertisement, "fn")?, "fn")?;
    let model = advertisement
        .txt_value("md")
        .filter(|value| !value.is_empty())
        .map(|value| safe_text(value, "md"))
        .transpose()?;

    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        format!("receiver-{receiver_id}"),
        DiscoverySource::Mdns,
        BridgeTransport::LanTcp,
        advertisement.discovered_at_ms,
    )?
    .with_display_name(friendly_name)
    .with_address(advertisement.endpoint_with_scheme("tcp"))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE)
    .with_metadata("cast.receiver_id", receiver_id)
    .with_metadata("cast.protocol_version", protocol_version.to_string())
    .with_metadata("cast.capabilities", capabilities.to_string())
    .with_metadata("cast.status", status);

    if let Some(model) = model {
        record = record.with_hardware_model(model);
    }
    for (mask, key) in [
        (CAPABILITY_VIDEO_OUTPUT, "cast.capability.video_output"),
        (CAPABILITY_VIDEO_INPUT, "cast.capability.video_input"),
        (CAPABILITY_AUDIO_OUTPUT, "cast.capability.audio_output"),
        (CAPABILITY_AUDIO_INPUT, "cast.capability.audio_input"),
        (CAPABILITY_DEVELOPER_MODE, "cast.capability.developer_mode"),
    ] {
        if capabilities & mask != 0 {
            record = record.with_metadata(key, "true");
        }
    }
    Ok(record)
}

fn required_txt<'a>(
    advertisement: &'a MdnsAdvertisement,
    key: &str,
) -> Result<&'a str, GoogleCastDiscoveryError> {
    advertisement
        .txt_value(key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| GoogleCastDiscoveryError::Validation(format!("missing {key} TXT record")))
}

fn safe_text(value: &str, field: &str) -> Result<String, GoogleCastDiscoveryError> {
    if value.is_empty() || value.len() > MAX_TXT_VALUE_BYTES || value.chars().any(char::is_control)
    {
        return Err(GoogleCastDiscoveryError::Validation(format!(
            "{field} must be non-empty bounded text without control characters"
        )));
    }
    Ok(value.to_string())
}

fn normalize_receiver_id(value: &str) -> Result<String, GoogleCastDiscoveryError> {
    let mut normalized = String::with_capacity(32);
    for byte in value.bytes() {
        match byte {
            b'-' => {}
            byte if byte.is_ascii_hexdigit() => {
                normalized.push((byte as char).to_ascii_lowercase())
            }
            _ => {
                return Err(GoogleCastDiscoveryError::Validation(
                    "id must be a 128-bit hexadecimal receiver UUID".to_string(),
                ))
            }
        }
    }
    if normalized.len() != 32 {
        return Err(GoogleCastDiscoveryError::Validation(
            "id must be a 128-bit hexadecimal receiver UUID".to_string(),
        ));
    }
    Ok(normalized)
}

fn parse_protocol_version(value: &str) -> Result<u8, GoogleCastDiscoveryError> {
    if value.is_empty() || value.len() > 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(GoogleCastDiscoveryError::Validation(
            "ve must be a one- or two-digit decimal Cast protocol version".to_string(),
        ));
    }
    let version = value.parse::<u8>().map_err(|_| {
        GoogleCastDiscoveryError::Validation("ve is outside the supported range".to_string())
    })?;
    if !(MIN_PROTOCOL_VERSION..=MAX_PROTOCOL_VERSION).contains(&version) {
        return Err(GoogleCastDiscoveryError::Validation(format!(
            "ve must be between {MIN_PROTOCOL_VERSION} and {MAX_PROTOCOL_VERSION}"
        )));
    }
    Ok(version)
}

fn parse_capabilities(value: &str) -> Result<u64, GoogleCastDiscoveryError> {
    if value.is_empty() || value.len() > 20 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(GoogleCastDiscoveryError::Validation(
            "ca must be a bounded unsigned decimal bitfield".to_string(),
        ));
    }
    value.parse::<u64>().map_err(|_| {
        GoogleCastDiscoveryError::Validation("ca is outside the unsigned 64-bit range".to_string())
    })
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

    const RECEIVER_ID: &str = "AABBCCDD-EEFF-0011-2233-445566778899";

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        result: MdnsScanResult,
    }

    impl GoogleCastMdnsTransport for FakeTransport {
        fn scan(
            &mut self,
            options: MdnsScanOptions,
        ) -> Result<MdnsScanResult, GoogleCastDiscoveryError> {
            assert_eq!(options.service_type, MDNS_SERVICE_TYPE);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn advertisement(address: &str, receiver_id: &str) -> MdnsAdvertisement {
        advertisement_with_contract(address, receiver_id, "02", "0")
    }

    fn advertisement_with_contract(
        address: &str,
        receiver_id: &str,
        version: &str,
        status: &str,
    ) -> MdnsAdvertisement {
        MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Chromecast-aabbccddeeff00112233445566778899",
            "living-room.local",
            8009,
            1_000,
        )
        .unwrap()
        .with_address(address)
        .unwrap()
        .with_txt("id", receiver_id)
        .unwrap()
        .with_txt("ve", version)
        .unwrap()
        .with_txt("ca", "5")
        .unwrap()
        .with_txt("st", status)
        .unwrap()
        .with_txt("fn", "Living Room")
        .unwrap()
        .with_txt("md", "Chromecast")
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
                CapabilityGrantId::trusted("grant:google-cast-discovery-test"),
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
        let mut config = GoogleCastDiscoveryConfig {
            timeout: Duration::ZERO,
            ..GoogleCastDiscoveryConfig::default()
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
    fn normalizes_receiver_identity_status_and_capabilities() {
        let record = discovery_record(&advertisement("192.0.2.50", RECEIVER_ID), 9_000).unwrap();
        assert_eq!(
            record.native_bridge_id,
            "receiver-aabbccddeeff00112233445566778899"
        );
        assert_eq!(record.address.as_deref(), Some("tcp://192.0.2.50:8009"));
        assert_eq!(record.display_name.as_deref(), Some("Living Room"));
        assert_eq!(record.hardware_model.as_deref(), Some("Chromecast"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::None);
        assert_eq!(record.expires_at_ms, Some(9_000));
        assert!(record
            .metadata
            .iter()
            .any(|item| { item.key == "cast.capability.video_output" && item.value == "true" }));
        assert!(record
            .metadata
            .iter()
            .any(|item| { item.key == "cast.capability.audio_output" && item.value == "true" }));
    }

    #[test]
    fn rejects_incomplete_or_invalid_receiver_contract() {
        let missing_id = MdnsAdvertisement::new(MDNS_SERVICE_TYPE, "Cast", "cast.local", 8009, 0)
            .unwrap()
            .with_address("192.0.2.50")
            .unwrap()
            .with_txt("ve", "02")
            .unwrap()
            .with_txt("ca", "5")
            .unwrap()
            .with_txt("st", "0")
            .unwrap()
            .with_txt("fn", "Cast")
            .unwrap();
        assert!(discovery_record(&missing_id, 1).is_err());

        let invalid_version = advertisement_with_contract("192.0.2.50", RECEIVER_ID, "1", "0");
        assert!(discovery_record(&invalid_version, 1).is_err());

        let invalid_status = advertisement_with_contract("192.0.2.50", RECEIVER_ID, "02", "2");
        assert!(discovery_record(&invalid_status, 1).is_err());

        let mut unresolved = advertisement("192.0.2.50", RECEIVER_ID);
        unresolved.addresses.clear();
        assert!(discovery_record(&unresolved, 1).is_err());
    }

    #[test]
    fn deduplicates_stable_identity_and_preserves_partial_failures() {
        let mut result = scan_result(vec![
            advertisement("192.0.2.50", RECEIVER_ID),
            advertisement("192.0.2.51", RECEIVER_ID),
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
            discover(&GoogleCastDiscoveryConfig::default(), &mut transport, 1_000).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].address.as_deref(),
            Some("tcp://192.0.2.50:8009")
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
            &GoogleCastDiscoveryConfig::default(),
            &mut transport,
            1_000,
        );
        assert!(matches!(
            result,
            Err(GoogleCastDiscoveryError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn live_loopback_packet_is_parsed_and_committed() {
        struct LoopbackTransport;
        impl GoogleCastMdnsTransport for LoopbackTransport {
            fn scan(
                &mut self,
                options: MdnsScanOptions,
            ) -> Result<MdnsScanResult, GoogleCastDiscoveryError> {
                let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
                receiver
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
                sender
                    .send_to(&cast_mdns_response_packet(), receiver.local_addr().unwrap())
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

        let principal = AgentId::trusted("agent:google-cast-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &GoogleCastDiscoveryConfig::default(),
            &mut LoopbackTransport,
            2_000,
        )
        .unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "cast.bridge.receiver-aabbccddeeff00112233445566778899"
            ))
            .is_some());
    }

    fn cast_mdns_response_packet() -> Vec<u8> {
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
        encode_name(
            "Chromecast-aabbccddeeff00112233445566778899._googlecast._tcp.local",
            &mut packet,
        );
        fill_length(&mut packet, ptr_length);

        encode_name(
            "Chromecast-aabbccddeeff00112233445566778899._googlecast._tcp.local",
            &mut packet,
        );
        push_record_header(&mut packet, 33);
        let srv_length = reserve_length(&mut packet);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 8009);
        encode_name("living-room.local", &mut packet);
        fill_length(&mut packet, srv_length);

        encode_name(
            "Chromecast-aabbccddeeff00112233445566778899._googlecast._tcp.local",
            &mut packet,
        );
        push_record_header(&mut packet, 16);
        let txt_length = reserve_length(&mut packet);
        for (key, value) in [
            ("id", "AABBCCDDEEFF00112233445566778899"),
            ("ve", "02"),
            ("ca", "5"),
            ("st", "0"),
            ("fn", "Living Room"),
            ("md", "Chromecast"),
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
