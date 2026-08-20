//! Authorized bounded Matter operational DNS-SD discovery for D23.

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
pub const INTEGRATION_ID: &str = "matter_operational_discovery";
pub const MDNS_SERVICE_TYPE: &str = "_matter._tcp.local";
pub const MAX_RESPONSES: usize = 64;
const MAX_HOST_BYTES: usize = 253;
const MAX_MRP_INTERVAL_MS: u64 = 3_600_000;
const TCP_CLIENT_FLAG: u8 = 1 << 1;
const TCP_SERVER_FLAG: u8 = 1 << 2;
const SUPPORTED_TCP_FLAGS: u8 = TCP_CLIENT_FLAG | TCP_SERVER_FLAG;

#[derive(Debug)]
pub enum MatterOperationalDiscoveryError {
    Validation(String),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for MatterOperationalDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(formatter, "invalid Matter operational discovery: {message}")
            }
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for MatterOperationalDiscoveryError {}

impl From<DiscoveryError> for MatterOperationalDiscoveryError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for MatterOperationalDiscoveryError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterOperationalDiscoveryConfig {
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl Default for MatterOperationalDiscoveryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }
}

impl MatterOperationalDiscoveryConfig {
    pub fn validate(&self) -> Result<(), MatterOperationalDiscoveryError> {
        if self.timeout.is_zero() {
            return Err(MatterOperationalDiscoveryError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(MatterOperationalDiscoveryError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(MatterOperationalDiscoveryError::Validation(
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

pub trait MatterOperationalMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, MatterOperationalDiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpMatterOperationalMdnsTransport;

impl MatterOperationalMdnsTransport for UdpMatterOperationalMdnsTransport {
    fn scan(
        &mut self,
        options: MdnsScanOptions,
    ) -> Result<MdnsScanResult, MatterOperationalDiscoveryError> {
        Ok(run_mdns_ipv4_scan(options)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterOperationalDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MatterOperationalRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: MatterOperationalMdnsTransport>(
    config: &MatterOperationalDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<MatterOperationalDiscoveryReport, MatterOperationalDiscoveryError> {
    config.validate()?;
    let result = transport.scan(config.scan_options(discovered_at_ms)?)?;
    if normalized_service_type(&result.service_type) != normalized_service_type(MDNS_SERVICE_TYPE) {
        return Err(MatterOperationalDiscoveryError::Validation(format!(
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
            "discarded {} Matter operational advertisements beyond the configured result limit",
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
                    "Matter node {} advertises conflicting endpoints `{}` and `{}`",
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
                "invalid Matter operational advertisement `{}`: {error}",
                advertisement.instance_name
            )),
        }
    }

    Ok(MatterOperationalDiscoveryReport {
        records: records.into_values().collect(),
        failures,
    })
}

pub fn discover_into_runtime<T: MatterOperationalMdnsTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &MatterOperationalDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<MatterOperationalRuntimeCommitSummary, MatterOperationalDiscoveryError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(MatterOperationalDiscoveryError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = MatterOperationalRuntimeCommitSummary {
        failures: report.failures.len(),
        ..MatterOperationalRuntimeCommitSummary::default()
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
) -> Result<DiscoveryRecord, MatterOperationalDiscoveryError> {
    if normalized_service_type(&advertisement.service_type)
        != normalized_service_type(MDNS_SERVICE_TYPE)
    {
        return Err(MatterOperationalDiscoveryError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    if advertisement.port == 0 {
        return Err(MatterOperationalDiscoveryError::Validation(
            "operational TCP port must be non-zero".to_string(),
        ));
    }
    if advertisement.addresses.is_empty() {
        return Err(MatterOperationalDiscoveryError::Validation(
            "operational advertisement must resolve at least one IP address".to_string(),
        ));
    }
    validate_host_name(&advertisement.host_name)?;

    let (compressed_fabric_id, node_id) = parse_operational_instance(&advertisement.instance_name)?;
    let idle_interval = optional_decimal(advertisement, "SII", 7, MAX_MRP_INTERVAL_MS, true)?;
    let active_interval = optional_decimal(advertisement, "SAI", 7, MAX_MRP_INTERVAL_MS, true)?;
    let active_threshold = optional_decimal(advertisement, "SAT", 5, u16::MAX.into(), false)?;
    let tcp_flags =
        optional_decimal(advertisement, "T", 1, u8::MAX.into(), true)?.unwrap_or(0) as u8;
    if tcp_flags & !SUPPORTED_TCP_FLAGS != 0 {
        return Err(MatterOperationalDiscoveryError::Validation(
            "T contains reserved or unsupported TCP capability flags".to_string(),
        ));
    }
    let icd = optional_boolean(advertisement, "ICD")?.unwrap_or(false);
    let native_id = format!(
        "fabric-{}-node-{}",
        compressed_fabric_id.to_ascii_lowercase(),
        node_id.to_ascii_lowercase()
    );

    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Matter,
        native_id,
        DiscoverySource::Mdns,
        BridgeTransport::LanTcp,
        advertisement.discovered_at_ms,
    )?
    .with_display_name(advertisement.instance_name.to_ascii_uppercase())
    .with_address(advertisement.endpoint_with_scheme("tcp"))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE)
    .with_metadata("matter.compressed_fabric_id", &compressed_fabric_id)
    .with_metadata("matter.node_id", &node_id)
    .with_metadata(
        "matter.tcp_client_supported",
        (tcp_flags & TCP_CLIENT_FLAG != 0).to_string(),
    )
    .with_metadata(
        "matter.tcp_server_supported",
        (tcp_flags & TCP_SERVER_FLAG != 0).to_string(),
    )
    .with_metadata("matter.icd", icd.to_string());

    for (key, value) in [
        ("matter.mrp_idle_interval_ms", idle_interval),
        ("matter.mrp_active_interval_ms", active_interval),
        ("matter.mrp_active_threshold_ms", active_threshold),
    ] {
        if let Some(value) = value {
            record = record.with_metadata(key, value.to_string());
        }
    }
    Ok(record)
}

fn parse_operational_instance(
    value: &str,
) -> Result<(String, String), MatterOperationalDiscoveryError> {
    if value.len() != 33 || value.as_bytes().get(16) != Some(&b'-') {
        return Err(MatterOperationalDiscoveryError::Validation(
            "instance must be 16 hexadecimal fabric-id digits, a hyphen, and 16 node-id digits"
                .to_string(),
        ));
    }
    let fabric = &value[..16];
    let node = &value[17..];
    if !fabric.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !node.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(MatterOperationalDiscoveryError::Validation(
            "instance fabric id and node id must be hexadecimal".to_string(),
        ));
    }
    Ok((fabric.to_ascii_uppercase(), node.to_ascii_uppercase()))
}

fn validate_host_name(value: &str) -> Result<(), MatterOperationalDiscoveryError> {
    if value.is_empty()
        || value.len() > MAX_HOST_BYTES
        || value.chars().any(char::is_control)
        || !value
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(".local")
    {
        return Err(MatterOperationalDiscoveryError::Validation(
            "host name must be bounded local DNS text without control characters".to_string(),
        ));
    }
    Ok(())
}

fn optional_txt<'a>(
    advertisement: &'a MdnsAdvertisement,
    key: &str,
) -> Result<Option<&'a str>, MatterOperationalDiscoveryError> {
    let mut matches = advertisement
        .txt
        .iter()
        .filter(|entry| entry.key.eq_ignore_ascii_case(key));
    let value = matches.next().map(|entry| entry.value.as_str());
    if matches.next().is_some() {
        return Err(MatterOperationalDiscoveryError::Validation(format!(
            "duplicate case-insensitive {key} TXT record"
        )));
    }
    if value == Some("") {
        return Err(MatterOperationalDiscoveryError::Validation(format!(
            "{key} TXT record must not be empty"
        )));
    }
    Ok(value)
}

fn optional_decimal(
    advertisement: &MdnsAdvertisement,
    key: &str,
    maximum_length: usize,
    maximum: u64,
    allow_zero: bool,
) -> Result<Option<u64>, MatterOperationalDiscoveryError> {
    optional_txt(advertisement, key)?
        .map(|value| parse_decimal(value, key, maximum_length, maximum, allow_zero))
        .transpose()
}

fn parse_decimal(
    value: &str,
    field: &str,
    maximum_length: usize,
    maximum: u64,
    allow_zero: bool,
) -> Result<u64, MatterOperationalDiscoveryError> {
    if value.len() > maximum_length
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(MatterOperationalDiscoveryError::Validation(format!(
            "{field} must be a canonical bounded unsigned decimal integer"
        )));
    }
    let parsed = value.parse::<u64>().map_err(|_| {
        MatterOperationalDiscoveryError::Validation(format!(
            "{field} is outside its unsigned range"
        ))
    })?;
    if parsed > maximum || (!allow_zero && parsed == 0) {
        return Err(MatterOperationalDiscoveryError::Validation(format!(
            "{field} is outside its permitted range"
        )));
    }
    Ok(parsed)
}

fn optional_boolean(
    advertisement: &MdnsAdvertisement,
    key: &str,
) -> Result<Option<bool>, MatterOperationalDiscoveryError> {
    optional_txt(advertisement, key)?
        .map(|value| match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(MatterOperationalDiscoveryError::Validation(format!(
                "{key} must be 0 or 1"
            ))),
        })
        .transpose()
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

    const INSTANCE: &str = "0123456789ABCDEF-1122334455667788";

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        result: MdnsScanResult,
    }

    impl MatterOperationalMdnsTransport for FakeTransport {
        fn scan(
            &mut self,
            options: MdnsScanOptions,
        ) -> Result<MdnsScanResult, MatterOperationalDiscoveryError> {
            assert_eq!(options.service_type, MDNS_SERVICE_TYPE);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn advertisement(address: &str, instance: &str) -> MdnsAdvertisement {
        MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            instance,
            "matter-node.local",
            5540,
            1_000,
        )
        .unwrap()
        .with_address(address)
        .unwrap()
        .with_txt("SII", "5000")
        .unwrap()
        .with_txt("SAI", "300")
        .unwrap()
        .with_txt("SAT", "4000")
        .unwrap()
        .with_txt("T", "6")
        .unwrap()
        .with_txt("ICD", "1")
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
                CapabilityGrantId::trusted("grant:matter-operational-discovery-test"),
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
        let mut config = MatterOperationalDiscoveryConfig {
            timeout: Duration::ZERO,
            ..MatterOperationalDiscoveryConfig::default()
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
    fn normalizes_operational_identity_and_common_txt() {
        let record = discovery_record(&advertisement("192.0.2.80", INSTANCE), 9_000).unwrap();
        assert_eq!(
            record.native_bridge_id,
            "fabric-0123456789abcdef-node-1122334455667788"
        );
        assert_eq!(record.address.as_deref(), Some("tcp://192.0.2.80:5540"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::Credentials);
        assert_eq!(record.expires_at_ms, Some(9_000));
        for (key, value) in [
            ("matter.compressed_fabric_id", "0123456789ABCDEF"),
            ("matter.node_id", "1122334455667788"),
            ("matter.mrp_idle_interval_ms", "5000"),
            ("matter.mrp_active_interval_ms", "300"),
            ("matter.mrp_active_threshold_ms", "4000"),
            ("matter.tcp_client_supported", "true"),
            ("matter.tcp_server_supported", "true"),
            ("matter.icd", "true"),
        ] {
            assert!(record
                .metadata
                .iter()
                .any(|item| item.key == key && item.value == value));
        }
    }

    #[test]
    fn rejects_invalid_operational_contract() {
        assert!(discovery_record(&advertisement("192.0.2.80", "bad-instance"), 1).is_err());

        let mut leading_zero = advertisement("192.0.2.80", INSTANCE);
        leading_zero.txt.retain(|entry| entry.key != "SII");
        leading_zero = leading_zero.with_txt("SII", "05000").unwrap();
        assert!(discovery_record(&leading_zero, 1).is_err());

        let mut invalid_flags = advertisement("192.0.2.80", INSTANCE);
        invalid_flags.txt.retain(|entry| entry.key != "T");
        invalid_flags = invalid_flags.with_txt("T", "1").unwrap();
        assert!(discovery_record(&invalid_flags, 1).is_err());

        let mut duplicate_case = advertisement("192.0.2.80", INSTANCE);
        duplicate_case = duplicate_case.with_txt("sii", "6000").unwrap();
        assert!(discovery_record(&duplicate_case, 1).is_err());

        let mut unresolved = advertisement("192.0.2.80", INSTANCE);
        unresolved.addresses.clear();
        assert!(discovery_record(&unresolved, 1).is_err());
    }

    #[test]
    fn deduplicates_identity_preserves_failures_and_caps_results() {
        let mut result = scan_result(vec![
            advertisement("192.0.2.80", INSTANCE),
            advertisement("192.0.2.81", &INSTANCE.to_ascii_lowercase()),
            advertisement("192.0.2.82", "FEDCBA9876543210-8877665544332211"),
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
            &MatterOperationalDiscoveryConfig {
                maximum_responses: 2,
                ..MatterOperationalDiscoveryConfig::default()
            },
            &mut transport,
            1_000,
        )
        .unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].address.as_deref(),
            Some("tcp://192.0.2.80:5540")
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
            &MatterOperationalDiscoveryConfig::default(),
            &mut transport,
            1_000,
        );
        assert!(matches!(
            result,
            Err(MatterOperationalDiscoveryError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn live_loopback_packet_is_parsed_and_committed() {
        struct LoopbackTransport;
        impl MatterOperationalMdnsTransport for LoopbackTransport {
            fn scan(
                &mut self,
                options: MdnsScanOptions,
            ) -> Result<MdnsScanResult, MatterOperationalDiscoveryError> {
                let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
                receiver
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
                sender
                    .send_to(
                        &matter_mdns_response_packet(),
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

        let principal = AgentId::trusted("agent:matter-operational-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &MatterOperationalDiscoveryConfig::default(),
            &mut LoopbackTransport,
            2_000,
        )
        .unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "matter_operational_discovery.bridge.fabric-0123456789abcdef-node-1122334455667788"
            ))
            .is_some());
    }

    fn matter_mdns_response_packet() -> Vec<u8> {
        let mut packet = Vec::new();
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0x8400);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 1);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 3);

        let instance = format!("{INSTANCE}.{MDNS_SERVICE_TYPE}");
        encode_name(MDNS_SERVICE_TYPE, &mut packet);
        push_record_header(&mut packet, 12);
        let ptr_length = reserve_length(&mut packet);
        encode_name(&instance, &mut packet);
        fill_length(&mut packet, ptr_length);

        encode_name(&instance, &mut packet);
        push_record_header(&mut packet, 33);
        let srv_length = reserve_length(&mut packet);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 5540);
        encode_name("matter-node.local", &mut packet);
        fill_length(&mut packet, srv_length);

        encode_name(&instance, &mut packet);
        push_record_header(&mut packet, 16);
        let txt_length = reserve_length(&mut packet);
        for (key, value) in [
            ("SII", "5000"),
            ("SAI", "300"),
            ("SAT", "4000"),
            ("T", "6"),
            ("ICD", "1"),
        ] {
            push_txt(&mut packet, key, value);
        }
        fill_length(&mut packet, txt_length);

        encode_name("matter-node.local", &mut packet);
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
