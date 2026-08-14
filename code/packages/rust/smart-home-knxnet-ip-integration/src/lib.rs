//! Authorized bounded KNXnet/IP interface discovery for D23.

#![forbid(unsafe_code)]

use knxnet_ip_protocol::{
    decode_search_response, encode_search_request, KnxnetIpError, SearchResponse,
    KNXNET_IP_DEFAULT_PORT, KNXNET_IP_SYSTEM_MULTICAST, MAX_KNXNET_IP_DATAGRAM_BYTES,
};
use smart_home_core::{AgentId, BridgeTransport, IntegrationId, ProtocolFamily, SmartHomeTool};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource, DiscoveryUpsert,
    PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use udp_client::{UdpClient, UdpDatagram, UdpError, UdpOptions};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "knxnet_ip";
pub const PROTOCOL_ID: &str = "knxnet_ip";
pub const MAX_RESPONSES: usize = 64;

#[derive(Debug)]
pub enum KnxIntegrationError {
    Validation(String),
    Protocol(KnxnetIpError),
    Udp(UdpError),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for KnxIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid KNXnet/IP input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Udp(error) => error.fmt(formatter),
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for KnxIntegrationError {}

impl From<KnxnetIpError> for KnxIntegrationError {
    fn from(error: KnxnetIpError) -> Self {
        Self::Protocol(error)
    }
}

impl From<UdpError> for KnxIntegrationError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<DiscoveryError> for KnxIntegrationError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for KnxIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnxnetIpDiscoveryConfig {
    pub local_interface: Ipv4Addr,
    pub destination: SocketAddrV4,
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
}

impl KnxnetIpDiscoveryConfig {
    pub fn new(local_interface: Ipv4Addr) -> Self {
        Self {
            local_interface,
            destination: SocketAddrV4::new(KNXNET_IP_SYSTEM_MULTICAST, KNXNET_IP_DEFAULT_PORT),
            timeout: Duration::from_millis(750),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
        }
    }

    pub fn validate(&self) -> Result<(), KnxIntegrationError> {
        if self.local_interface.is_unspecified()
            || self.local_interface.is_broadcast()
            || self.local_interface.is_multicast()
        {
            return Err(KnxIntegrationError::Validation(
                "local interface must be an explicit unicast IPv4 address".to_string(),
            ));
        }
        if self.destination.ip().is_unspecified() || self.destination.ip().is_broadcast() {
            return Err(KnxIntegrationError::Validation(
                "destination must be an explicit multicast or unicast IPv4 address".to_string(),
            ));
        }
        if self.destination.port() == 0 {
            return Err(KnxIntegrationError::Validation(
                "destination port must be non-zero".to_string(),
            ));
        }
        if self.timeout.is_zero() {
            return Err(KnxIntegrationError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(KnxIntegrationError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(KnxIntegrationError::Validation(
                "record TTL must be non-zero".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnxTransportReport {
    pub request_endpoint: SocketAddrV4,
    pub datagrams: Vec<UdpDatagram>,
}

pub trait KnxnetIpTransport {
    fn discover(
        &mut self,
        config: &KnxnetIpDiscoveryConfig,
    ) -> Result<KnxTransportReport, KnxIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpKnxnetIpTransport;

impl KnxnetIpTransport for UdpKnxnetIpTransport {
    fn discover(
        &mut self,
        config: &KnxnetIpDiscoveryConfig,
    ) -> Result<KnxTransportReport, KnxIntegrationError> {
        let client = UdpClient::bind(UdpOptions {
            bind_addr: Some(SocketAddr::V4(SocketAddrV4::new(config.local_interface, 0))),
            max_datagram_size: MAX_KNXNET_IP_DATAGRAM_BYTES,
            read_timeout: Some(config.timeout),
            write_timeout: Some(config.timeout),
        })?;
        let request_endpoint = match client.local_addr()? {
            SocketAddr::V4(endpoint) => endpoint,
            SocketAddr::V6(_) => {
                return Err(KnxIntegrationError::Validation(
                    "local interface did not produce an IPv4 socket".to_string(),
                ))
            }
        };
        let probe = encode_search_request(request_endpoint)?;
        client.send_to(&probe, SocketAddr::V4(config.destination))?;
        let mut datagrams = Vec::new();
        while datagrams.len() < config.maximum_responses {
            match client.recv_from() {
                Ok(datagram) => datagrams.push(datagram),
                Err(UdpError::Timeout) => break,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(KnxTransportReport {
            request_endpoint,
            datagrams,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnxDiscoveryReport {
    pub request_endpoint: SocketAddrV4,
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KnxRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: KnxnetIpTransport>(
    config: &KnxnetIpDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<KnxDiscoveryReport, KnxIntegrationError> {
    config.validate()?;
    let exchange = transport.discover(config)?;
    let mut interfaces = BTreeMap::<[u8; 6], (SocketAddrV4, SearchResponse)>::new();
    let mut failures = Vec::new();

    for datagram in exchange.datagrams {
        let source = match datagram.source {
            SocketAddr::V4(source) => source,
            SocketAddr::V6(source) => {
                failures.push(format!("ignored IPv6 KNXnet/IP reply from {source}"));
                continue;
            }
        };
        match decode_search_response(&datagram.payload) {
            Ok(response) if response.control_endpoint != source => failures.push(format!(
                "KNXnet/IP reply from {source} advertises mismatched control endpoint {}",
                response.control_endpoint
            )),
            Ok(response) => match interfaces.get(&response.serial_number) {
                Some((existing, _)) if *existing != response.control_endpoint => {
                    failures.push(format!(
                        "KNXnet/IP serial {} replied from both {existing} and {}",
                        response.serial_number_hex(),
                        response.control_endpoint
                    ));
                }
                Some(_) => {}
                None => {
                    interfaces.insert(response.serial_number, (source, response));
                }
            },
            Err(error) => failures.push(format!("invalid KNXnet/IP reply from {source}: {error}")),
        }
    }

    let ttl_ms = u64::try_from(config.record_ttl.as_millis()).unwrap_or(u64::MAX);
    let expires_at_ms = discovered_at_ms.saturating_add(ttl_ms);
    let records = interfaces
        .into_values()
        .map(|(source, response)| {
            discovery_record(config, source, &response, discovered_at_ms, expires_at_ms)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(KnxDiscoveryReport {
        request_endpoint: exchange.request_endpoint,
        records,
        failures,
    })
}

pub fn discover_into_runtime<T: KnxnetIpTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &KnxnetIpDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<KnxRuntimeCommitSummary, KnxIntegrationError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(KnxIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = KnxRuntimeCommitSummary {
        failures: report.failures.len(),
        ..KnxRuntimeCommitSummary::default()
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

fn discovery_record(
    config: &KnxnetIpDiscoveryConfig,
    source: SocketAddrV4,
    response: &SearchResponse,
    discovered_at_ms: u64,
    expires_at_ms: u64,
) -> Result<DiscoveryRecord, DiscoveryError> {
    let source_kind = if config.destination.ip().is_multicast() {
        DiscoverySource::UdpMulticast
    } else {
        DiscoverySource::Manual
    };
    let serial = response.serial_number_hex();
    let services = response
        .supported_service_families
        .iter()
        .map(|family| format!("{}:{}", family.family_id, family.version))
        .collect::<Vec<_>>()
        .join(",");
    let display_name = if response.friendly_name.is_empty() {
        format!("KNXnet/IP {}", response.individual_address_string())
    } else {
        response.friendly_name.clone()
    };
    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        format!("serial-{serial}"),
        source_kind,
        BridgeTransport::LanUdp,
        discovered_at_ms,
    )?
    .with_display_name(display_name)
    .with_address(source.to_string())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::Unknown)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata("knx.serial_number", serial)
    .with_metadata(
        "knx.individual_address",
        response.individual_address_string(),
    )
    .with_metadata("knx.medium", response.medium.as_str())
    .with_metadata(
        "knx.programming_mode",
        response.programming_mode.to_string(),
    )
    .with_metadata("knx.project_id", response.project_id.to_string())
    .with_metadata("knx.installation_id", response.installation_id.to_string())
    .with_metadata("knx.mac_address", response.mac_address_hex())
    .with_metadata("knx.supported_service_families", services)
    .with_metadata("knx.discovery_destination", config.destination.to_string());
    if let Some(address) = response.routing_multicast_address {
        record = record.with_metadata("knx.routing_multicast_address", address.to_string());
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    fn search_response(endpoint: SocketAddrV4, serial: [u8; 6]) -> Vec<u8> {
        let mut bytes = vec![
            6, 0x10, 0x02, 0x02, 0, 74, 8, 1, 0, 0, 0, 0, 0, 0, 54, 1, 0x02, 0, 0x11, 0x0a, 0x12,
            0x33, 0, 0, 0, 0, 0, 0, 224, 0, 23, 12, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        bytes[8..12].copy_from_slice(&endpoint.ip().octets());
        bytes[12..14].copy_from_slice(&endpoint.port().to_be_bytes());
        bytes[22..28].copy_from_slice(&serial);
        let mut name = [0u8; 30];
        name[..13].copy_from_slice(b"KNX Interface");
        bytes.extend_from_slice(&name);
        bytes.extend_from_slice(&[6, 2, 2, 1, 4, 1]);
        bytes
    }

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        report: KnxTransportReport,
    }

    impl KnxnetIpTransport for FakeTransport {
        fn discover(
            &mut self,
            config: &KnxnetIpDiscoveryConfig,
        ) -> Result<KnxTransportReport, KnxIntegrationError> {
            assert_eq!(config.local_interface, Ipv4Addr::LOCALHOST);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.report.clone())
        }
    }

    fn datagram(source: SocketAddrV4, payload: Vec<u8>) -> UdpDatagram {
        UdpDatagram {
            source: SocketAddr::V4(source),
            destination: "127.0.0.1:50000".parse().unwrap(),
            payload,
        }
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ =
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                    CapabilityGrantId::trusted("grant:knx-test"),
                    principal.clone(),
                    PrivilegeTier::LowRisk,
                    "test",
                    0,
                ));
    }

    #[test]
    fn validates_explicit_interface_and_bounds() {
        let mut config = KnxnetIpDiscoveryConfig::new(Ipv4Addr::UNSPECIFIED);
        assert!(config.validate().is_err());
        config.local_interface = Ipv4Addr::LOCALHOST;
        config.maximum_responses = 0;
        assert!(config.validate().is_err());
        config.maximum_responses = MAX_RESPONSES + 1;
        assert!(config.validate().is_err());
        config.maximum_responses = 1;
        config.destination.set_port(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn normalizes_valid_replies_and_preserves_partial_failures() {
        let source: SocketAddrV4 = "192.0.2.10:3671".parse().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            report: KnxTransportReport {
                request_endpoint: "127.0.0.1:50000".parse().unwrap(),
                datagrams: vec![
                    datagram(source, search_response(source, [1, 2, 3, 4, 5, 6])),
                    datagram(source, vec![0; 8]),
                ],
            },
        };
        let config = KnxnetIpDiscoveryConfig::new(Ipv4Addr::LOCALHOST);
        let report = discover(&config, &mut transport, 1_000).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 1);
        let record = &report.records[0];
        assert_eq!(record.native_bridge_id, "serial-010203040506");
        assert_eq!(record.address.as_deref(), Some("192.0.2.10:3671"));
        assert_eq!(record.expires_at_ms, Some(301_000));
        assert_eq!(record.pairing_requirement, PairingRequirement::Unknown);
    }

    #[test]
    fn rejects_endpoint_mismatch_and_conflicting_serial_claims() {
        let source_a: SocketAddrV4 = "192.0.2.10:3671".parse().unwrap();
        let source_b: SocketAddrV4 = "192.0.2.11:3671".parse().unwrap();
        let serial = [1, 2, 3, 4, 5, 6];
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            report: KnxTransportReport {
                request_endpoint: "127.0.0.1:50000".parse().unwrap(),
                datagrams: vec![
                    datagram(source_a, search_response(source_a, serial)),
                    datagram(source_b, search_response(source_b, serial)),
                    datagram(source_b, search_response(source_a, [6, 5, 4, 3, 2, 1])),
                ],
            },
        };
        let report = discover(
            &KnxnetIpDiscoveryConfig::new(Ipv4Addr::LOCALHOST),
            &mut transport,
            0,
        )
        .unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 2);
    }

    #[test]
    fn denies_before_transport_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            report: KnxTransportReport {
                request_endpoint: "127.0.0.1:50000".parse().unwrap(),
                datagrams: Vec::new(),
            },
        };
        let mut runtime = SmartHomeRuntime::new();
        let result = discover_into_runtime(
            &mut runtime,
            AgentId::trusted("agent:denied"),
            &KnxnetIpDiscoveryConfig::new(Ipv4Addr::LOCALHOST),
            &mut transport,
            0,
        );
        assert!(matches!(
            result,
            Err(KnxIntegrationError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn discovers_over_live_loopback_udp_and_records_runtime_bridge() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let destination = match server.local_addr().unwrap() {
            SocketAddr::V4(address) => address,
            SocketAddr::V6(_) => unreachable!(),
        };
        let responder = thread::spawn(move || {
            let mut probe = [0u8; 64];
            let (length, source) = server.recv_from(&mut probe).unwrap();
            assert_eq!(length, 14);
            assert_eq!(&probe[..6], [6, 0x10, 0x02, 0x01, 0, 14]);
            let advertised = SocketAddrV4::new(
                Ipv4Addr::new(probe[8], probe[9], probe[10], probe[11]),
                u16::from_be_bytes([probe[12], probe[13]]),
            );
            assert_eq!(source, SocketAddr::V4(advertised));
            server
                .send_to(&search_response(destination, [1, 1, 2, 3, 5, 8]), source)
                .unwrap();
        });

        let mut config = KnxnetIpDiscoveryConfig::new(Ipv4Addr::LOCALHOST);
        config.destination = destination;
        config.timeout = Duration::from_millis(100);
        let principal = AgentId::trusted("agent:knx-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &config,
            &mut UdpKnxnetIpTransport,
            2_000,
        )
        .unwrap();
        responder.join().unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "knxnet_ip.bridge.serial-010102030508"
            ))
            .is_some());
    }
}
