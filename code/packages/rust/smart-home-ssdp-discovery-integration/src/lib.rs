//! Authorized bounded UPnP SSDP discovery for D23.

#![forbid(unsafe_code)]

use smart_home_core::{AgentId, BridgeTransport, IntegrationId, ProtocolFamily, SmartHomeTool};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource, DiscoveryUpsert,
    PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use ssdp_protocol::{
    decode_search_response, encode_m_search, SearchRequest, SearchResponse, SsdpError,
    MAX_DATAGRAM_BYTES,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use udp_client::{UdpClient, UdpDatagram, UdpDiscoveryEndpoint, UdpError, UdpOptions};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "upnp_ssdp";
pub const PROTOCOL_ID: &str = "upnp_ssdp";
pub const MAX_RESPONSES: usize = 64;
pub const MAX_RECORD_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug)]
pub enum SsdpIntegrationError {
    Validation(String),
    Protocol(SsdpError),
    Url(UrlError),
    Udp(UdpError),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for SsdpIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid SSDP input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid SSDP LOCATION: {error}"),
            Self::Udp(error) => error.fmt(formatter),
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SsdpIntegrationError {}

impl From<SsdpError> for SsdpIntegrationError {
    fn from(error: SsdpError) -> Self {
        Self::Protocol(error)
    }
}

impl From<UrlError> for SsdpIntegrationError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<UdpError> for SsdpIntegrationError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<DiscoveryError> for SsdpIntegrationError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for SsdpIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsdpDiscoveryConfig {
    pub local_interface: Ipv4Addr,
    pub destination: SocketAddrV4,
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub maximum_record_ttl: Duration,
    pub request: SearchRequest,
}

impl SsdpDiscoveryConfig {
    pub fn new(local_interface: Ipv4Addr) -> Self {
        let endpoint = UdpDiscoveryEndpoint::ssdp_ipv4();
        let destination = match endpoint.destination {
            SocketAddr::V4(address) => address,
            SocketAddr::V6(_) => unreachable!("SSDP IPv4 endpoint must be IPv4"),
        };
        Self {
            local_interface,
            destination,
            timeout: Duration::from_secs(3),
            maximum_responses: 32,
            maximum_record_ttl: Duration::from_secs(60 * 60),
            request: SearchRequest::default(),
        }
    }

    pub fn validate(&self) -> Result<(), SsdpIntegrationError> {
        if !is_local_ipv4(self.local_interface) {
            return Err(SsdpIntegrationError::Validation(
                "local interface must be a private, link-local, or loopback IPv4 address"
                    .to_string(),
            ));
        }
        if self.destination.port() == 0
            || (!self.destination.ip().is_multicast() && !is_local_ipv4(*self.destination.ip()))
        {
            return Err(SsdpIntegrationError::Validation(
                "destination must be local IPv4 unicast or multicast with a non-zero port"
                    .to_string(),
            ));
        }
        if self.timeout.is_zero() || self.timeout > Duration::from_secs(10) {
            return Err(SsdpIntegrationError::Validation(
                "timeout must be between 1 millisecond and 10 seconds".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(SsdpIntegrationError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.maximum_record_ttl.is_zero() || self.maximum_record_ttl > MAX_RECORD_TTL {
            return Err(SsdpIntegrationError::Validation(
                "maximum record TTL must be between 1 millisecond and 24 hours".to_string(),
            ));
        }
        let _ = encode_m_search(&self.request)?;
        Ok(())
    }
}

pub trait SsdpTransport {
    fn search(
        &mut self,
        config: &SsdpDiscoveryConfig,
        request: &[u8],
    ) -> Result<Vec<UdpDatagram>, SsdpIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpSsdpTransport;

impl SsdpTransport for UdpSsdpTransport {
    fn search(
        &mut self,
        config: &SsdpDiscoveryConfig,
        request: &[u8],
    ) -> Result<Vec<UdpDatagram>, SsdpIntegrationError> {
        let client = UdpClient::bind(UdpOptions {
            bind_addr: Some(SocketAddr::V4(SocketAddrV4::new(config.local_interface, 0))),
            max_datagram_size: MAX_DATAGRAM_BYTES,
            read_timeout: Some(config.timeout),
            write_timeout: Some(config.timeout),
        })?;
        client.send_to(request, SocketAddr::V4(config.destination))?;
        let mut datagrams = Vec::new();
        while datagrams.len() < config.maximum_responses {
            match client.recv_from() {
                Ok(datagram) => datagrams.push(datagram),
                Err(UdpError::Timeout) => break,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(datagrams)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsdpDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SsdpRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

#[derive(Debug, Clone)]
struct AggregateResponse {
    source: SocketAddrV4,
    response: SearchResponse,
    targets: BTreeSet<String>,
    max_age_seconds: u32,
}

pub fn discover<T: SsdpTransport>(
    config: &SsdpDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<SsdpDiscoveryReport, SsdpIntegrationError> {
    config.validate()?;
    let request = encode_m_search(&config.request)?;
    let datagrams = transport.search(config, &request)?;
    let mut devices = BTreeMap::<String, AggregateResponse>::new();
    let mut failures = Vec::new();

    for datagram in datagrams {
        let source = match datagram.source {
            SocketAddr::V4(source) => source,
            SocketAddr::V6(source) => {
                failures.push(format!("ignored IPv6 SSDP response from {source}"));
                continue;
            }
        };
        if !is_local_ipv4(*source.ip()) {
            failures.push(format!("ignored non-local SSDP response from {source}"));
            continue;
        }
        let response =
            match decode_search_response(&datagram.payload, &config.request.search_target) {
                Ok(response) => response,
                Err(error) => {
                    failures.push(format!("invalid SSDP response from {source}: {error}"));
                    continue;
                }
            };
        if let Err(error) = validate_location(&response.location, source) {
            failures.push(format!("invalid SSDP response from {source}: {error}"));
            continue;
        }
        let key = response.unique_device_name.clone();
        match devices.get_mut(&key) {
            Some(existing)
                if existing.source != source || existing.response.location != response.location =>
            {
                failures.push(format!(
                    "SSDP device {key} advertised conflicting endpoints {} and {}",
                    existing.response.location, response.location
                ));
            }
            Some(existing) => {
                existing.targets.insert(response.search_target);
                existing.max_age_seconds = existing.max_age_seconds.min(response.max_age_seconds);
            }
            None => {
                let mut targets = BTreeSet::new();
                targets.insert(response.search_target.clone());
                devices.insert(
                    key,
                    AggregateResponse {
                        source,
                        max_age_seconds: response.max_age_seconds,
                        response,
                        targets,
                    },
                );
            }
        }
    }

    let records = devices
        .into_values()
        .map(|aggregate| discovery_record(config, aggregate, discovered_at_ms))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SsdpDiscoveryReport { records, failures })
}

pub fn discover_into_runtime<T: SsdpTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &SsdpDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<SsdpRuntimeCommitSummary, SsdpIntegrationError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(SsdpIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = SsdpRuntimeCommitSummary {
        failures: report.failures.len(),
        ..SsdpRuntimeCommitSummary::default()
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

fn validate_location(location: &str, source: SocketAddrV4) -> Result<(), SsdpIntegrationError> {
    let parsed = Url::parse(location)?;
    if parsed.scheme != "http"
        || parsed.userinfo.is_some()
        || parsed.query.is_some()
        || parsed.fragment.is_some()
        || parsed.path.is_empty()
    {
        return Err(SsdpIntegrationError::Validation(
            "LOCATION must be credential-free HTTP with a path and no query or fragment"
                .to_string(),
        ));
    }
    let host = parsed
        .host
        .as_deref()
        .ok_or_else(|| SsdpIntegrationError::Validation("LOCATION is missing a host".to_string()))?
        .parse::<Ipv4Addr>()
        .map_err(|_| {
            SsdpIntegrationError::Validation(
                "LOCATION host must be an explicit IPv4 address".to_string(),
            )
        })?;
    if host != *source.ip() || !is_local_ipv4(host) {
        return Err(SsdpIntegrationError::Validation(
            "LOCATION host must equal the local UDP response source".to_string(),
        ));
    }
    if parsed.effective_port() == Some(0) {
        return Err(SsdpIntegrationError::Validation(
            "LOCATION port must be non-zero".to_string(),
        ));
    }
    Ok(())
}

fn discovery_record(
    config: &SsdpDiscoveryConfig,
    aggregate: AggregateResponse,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, DiscoveryError> {
    let advertised_ttl = Duration::from_secs(u64::from(aggregate.max_age_seconds));
    let ttl = advertised_ttl.min(config.maximum_record_ttl);
    let ttl_ms = u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX);
    let targets = aggregate.targets.into_iter().collect::<Vec<_>>().join(",");
    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_native_id(&aggregate.response.unique_device_name),
        DiscoverySource::Ssdp,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )?
    .with_display_name("UPnP Device")
    .with_address(aggregate.response.location.clone())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_expires_at_ms(discovered_at_ms.saturating_add(ttl_ms))
    .with_metadata("upnp.udn", aggregate.response.unique_device_name)
    .with_metadata("upnp.search_targets", targets)
    .with_metadata("upnp.server", aggregate.response.server)
    .with_metadata("upnp.source", aggregate.source.to_string())
    .with_metadata(
        "upnp.max_age_seconds",
        aggregate.max_age_seconds.to_string(),
    )
    .with_metadata("upnp.discovery_destination", config.destination.to_string());
    if let Some(boot_id) = aggregate.response.boot_id {
        record = record.with_metadata("upnp.boot_id", boot_id.to_string());
    }
    if let Some(config_id) = aggregate.response.config_id {
        record = record.with_metadata("upnp.config_id", config_id.to_string());
    }
    Ok(record)
}

fn stable_native_id(udn: &str) -> String {
    let identifier = udn.strip_prefix("uuid:").unwrap_or(udn);
    let mut prefix = String::new();
    let mut previous_dash = false;
    for character in identifier.chars().take(48) {
        let normalized = if character.is_ascii_alphanumeric() {
            previous_dash = false;
            character.to_ascii_lowercase()
        } else if !previous_dash {
            previous_dash = true;
            '-'
        } else {
            continue;
        };
        prefix.push(normalized);
    }
    let prefix = prefix.trim_matches('-');
    let prefix = if prefix.is_empty() { "device" } else { prefix };
    format!("{prefix}-{:016x}", fnv1a64(udn.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn is_local_ipv4(address: Ipv4Addr) -> bool {
    address.is_private() || address.is_link_local() || address.is_loopback()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    fn response(location: &str, target: &str, udn: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=1800\r\nEXT:\r\nLOCATION: {location}\r\nSERVER: TestOS/1.0 UPnP/2.0 Test/1.0\r\nST: {target}\r\nUSN: {udn}::{target}\r\nBOOTID.UPNP.ORG: 4\r\nCONFIGID.UPNP.ORG: 9\r\n\r\n"
        )
        .into_bytes()
    }

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        replies: Vec<UdpDatagram>,
    }

    impl SsdpTransport for FakeTransport {
        fn search(
            &mut self,
            _config: &SsdpDiscoveryConfig,
            request: &[u8],
        ) -> Result<Vec<UdpDatagram>, SsdpIntegrationError> {
            assert!(request.starts_with(b"M-SEARCH * HTTP/1.1\r\n"));
            assert!(request.ends_with(b"\r\n\r\n"));
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.replies.clone())
        }
    }

    fn datagram(source: SocketAddrV4, payload: Vec<u8>) -> UdpDatagram {
        UdpDatagram {
            source: SocketAddr::V4(source),
            destination: "127.0.0.1:1900".parse().unwrap(),
            payload,
        }
    }

    fn config() -> SsdpDiscoveryConfig {
        SsdpDiscoveryConfig::new(Ipv4Addr::LOCALHOST)
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ =
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                    CapabilityGrantId::trusted("grant:ssdp-test"),
                    principal.clone(),
                    PrivilegeTier::LowRisk,
                    "test",
                    0,
                ));
    }

    #[test]
    fn validates_explicit_local_scope_and_bounds() {
        let mut config = SsdpDiscoveryConfig::new(Ipv4Addr::UNSPECIFIED);
        assert!(config.validate().is_err());
        config.local_interface = Ipv4Addr::LOCALHOST;
        config.destination = "8.8.8.8:1900".parse().unwrap();
        assert!(config.validate().is_err());
        config.destination = "127.0.0.1:1900".parse().unwrap();
        config.maximum_responses = MAX_RESPONSES + 1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn normalizes_valid_response_and_preserves_partial_failure() {
        let source: SocketAddrV4 = "192.168.1.10:1900".parse().unwrap();
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            replies: vec![
                datagram(
                    source,
                    response(
                        "http://192.168.1.10:1400/xml/device.xml",
                        "upnp:rootdevice",
                        "uuid:device-123",
                    ),
                ),
                datagram(source, b"not ssdp".to_vec()),
            ],
        };
        let report = discover(&config(), &mut transport, 1_000).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.records[0].source, DiscoverySource::Ssdp);
        assert_eq!(report.records[0].expires_at_ms, Some(1_801_000));
        assert!(report.records[0]
            .native_bridge_id
            .starts_with("device-123-"));
    }

    #[test]
    fn deduplicates_targets_for_one_udn() {
        let source: SocketAddrV4 = "192.168.1.10:1900".parse().unwrap();
        let location = "http://192.168.1.10:1400/xml/device.xml";
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            replies: vec![
                datagram(
                    source,
                    response(location, "upnp:rootdevice", "uuid:device-123"),
                ),
                datagram(
                    source,
                    response(
                        location,
                        "urn:schemas-upnp-org:device:MediaRenderer:1",
                        "uuid:device-123",
                    ),
                ),
            ],
        };
        let report = discover(&config(), &mut transport, 0).unwrap();
        assert_eq!(report.records.len(), 1);
        assert!(report.failures.is_empty());
        let targets = report.records[0]
            .metadata
            .iter()
            .find(|item| item.key == "upnp.search_targets")
            .unwrap();
        assert!(targets.value.contains("MediaRenderer"));
        assert!(targets.value.contains("upnp:rootdevice"));
    }

    #[test]
    fn rejects_source_location_mismatch_and_conflicting_identity() {
        let first: SocketAddrV4 = "192.168.1.10:1900".parse().unwrap();
        let second: SocketAddrV4 = "192.168.1.11:1900".parse().unwrap();
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            replies: vec![
                datagram(
                    first,
                    response(
                        "http://192.168.1.10/device.xml",
                        "upnp:rootdevice",
                        "uuid:device-123",
                    ),
                ),
                datagram(
                    second,
                    response(
                        "http://192.168.1.11/device.xml",
                        "upnp:rootdevice",
                        "uuid:device-123",
                    ),
                ),
                datagram(
                    first,
                    response(
                        "http://192.168.1.99/device.xml",
                        "upnp:rootdevice",
                        "uuid:spoofed",
                    ),
                ),
            ],
        };
        let report = discover(&config(), &mut transport, 0).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 2);
    }

    #[test]
    fn denies_before_udp_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            replies: Vec::new(),
        };
        let mut runtime = SmartHomeRuntime::new();
        let result = discover_into_runtime(
            &mut runtime,
            AgentId::trusted("agent:denied"),
            &config(),
            &mut transport,
            0,
        );
        assert!(matches!(
            result,
            Err(SsdpIntegrationError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn discovers_over_live_loopback_udp_and_records_runtime_candidate() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let destination = match server.local_addr().unwrap() {
            SocketAddr::V4(address) => address,
            SocketAddr::V6(_) => unreachable!(),
        };
        let responder = thread::spawn(move || {
            let mut probe = [0u8; 1024];
            let (length, source) = server.recv_from(&mut probe).unwrap();
            assert!(probe[..length].starts_with(b"M-SEARCH * HTTP/1.1\r\n"));
            let payload = response(
                "http://127.0.0.1:1400/device.xml",
                "upnp:rootdevice",
                "uuid:loopback-device",
            );
            server.send_to(&payload, source).unwrap();
        });

        let mut config = config();
        config.destination = destination;
        config.timeout = Duration::from_millis(100);
        let principal = AgentId::trusted("agent:ssdp-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &config,
            &mut UdpSsdpTransport,
            2_000,
        )
        .unwrap();
        responder.join().unwrap();
        assert_eq!(summary.inserted, 1);
        assert_eq!(runtime.discovery().len(), 1);
    }
}
