//! Authorized bounded BACnet/IP discovery for D23.

#![forbid(unsafe_code)]

use bacnet_protocol::{
    decode_i_am, encode_who_is, BacnetError, IAmResponse, WhoIsRequest, BACNET_IP_DEFAULT_PORT,
    MAX_BACNET_IP_DATAGRAM_BYTES,
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
pub const INTEGRATION_ID: &str = "bacnet_ip";
pub const PROTOCOL_ID: &str = "bacnet_ip";
pub const MAX_RESPONSES: usize = 64;

#[derive(Debug)]
pub enum BacnetIntegrationError {
    Validation(String),
    Protocol(BacnetError),
    Udp(UdpError),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
}

impl fmt::Display for BacnetIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid BACnet/IP input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Udp(error) => error.fmt(formatter),
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BacnetIntegrationError {}

impl From<BacnetError> for BacnetIntegrationError {
    fn from(error: BacnetError) -> Self {
        Self::Protocol(error)
    }
}

impl From<UdpError> for BacnetIntegrationError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<DiscoveryError> for BacnetIntegrationError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for BacnetIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacnetIpDiscoveryConfig {
    pub destination: SocketAddrV4,
    pub bind_addr: SocketAddrV4,
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
    pub request: WhoIsRequest,
}

impl BacnetIpDiscoveryConfig {
    pub fn new(destination: Ipv4Addr) -> Self {
        Self {
            destination: SocketAddrV4::new(destination, BACNET_IP_DEFAULT_PORT),
            bind_addr: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
            timeout: Duration::from_millis(750),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
            request: WhoIsRequest::All,
        }
    }

    pub fn validate(&self) -> Result<(), BacnetIntegrationError> {
        if self.destination.ip().is_unspecified() {
            return Err(BacnetIntegrationError::Validation(
                "destination must be an explicit IPv4 address".to_string(),
            ));
        }
        if self.destination.port() == 0 {
            return Err(BacnetIntegrationError::Validation(
                "destination port must be non-zero".to_string(),
            ));
        }
        if self.timeout.is_zero() {
            return Err(BacnetIntegrationError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(BacnetIntegrationError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(BacnetIntegrationError::Validation(
                "record TTL must be non-zero".to_string(),
            ));
        }
        let _ = encode_who_is(self.request)?;
        Ok(())
    }
}

pub trait BacnetIpTransport {
    fn discover(
        &mut self,
        config: &BacnetIpDiscoveryConfig,
        probe: &[u8],
    ) -> Result<Vec<UdpDatagram>, BacnetIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpBacnetIpTransport;

impl BacnetIpTransport for UdpBacnetIpTransport {
    fn discover(
        &mut self,
        config: &BacnetIpDiscoveryConfig,
        probe: &[u8],
    ) -> Result<Vec<UdpDatagram>, BacnetIntegrationError> {
        let client = UdpClient::bind(UdpOptions {
            bind_addr: Some(SocketAddr::V4(config.bind_addr)),
            max_datagram_size: MAX_BACNET_IP_DATAGRAM_BYTES,
            read_timeout: Some(config.timeout),
            write_timeout: Some(config.timeout),
        })?;
        client.set_broadcast(true)?;
        client.send_to(probe, SocketAddr::V4(config.destination))?;
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
pub struct BacnetDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BacnetRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: BacnetIpTransport>(
    config: &BacnetIpDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<BacnetDiscoveryReport, BacnetIntegrationError> {
    config.validate()?;
    let probe = encode_who_is(config.request)?;
    let datagrams = transport.discover(config, &probe)?;
    let mut devices = BTreeMap::<u32, (SocketAddrV4, IAmResponse)>::new();
    let mut failures = Vec::new();

    for datagram in datagrams {
        let source = match datagram.source {
            SocketAddr::V4(source) => source,
            SocketAddr::V6(source) => {
                failures.push(format!("ignored IPv6 BACnet/IP reply from {source}"));
                continue;
            }
        };
        match decode_i_am(&datagram.payload) {
            Ok(response) => {
                let effective_source = response.forwarded_from.unwrap_or(source);
                match devices.get(&response.device_instance) {
                    Some((existing, _)) if *existing != effective_source => failures.push(format!(
                        "BACnet device {} replied from both {existing} and {effective_source}",
                        response.device_instance
                    )),
                    Some(_) => {}
                    None => {
                        devices.insert(response.device_instance, (effective_source, response));
                    }
                }
            }
            Err(error) => failures.push(format!("invalid BACnet/IP reply from {source}: {error}")),
        }
    }

    let ttl_ms = u64::try_from(config.record_ttl.as_millis()).unwrap_or(u64::MAX);
    let expires_at_ms = discovered_at_ms.saturating_add(ttl_ms);
    let records = devices
        .into_values()
        .map(|(source, response)| {
            discovery_record(config, source, &response, discovered_at_ms, expires_at_ms)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BacnetDiscoveryReport { records, failures })
}

pub fn discover_into_runtime<T: BacnetIpTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &BacnetIpDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<BacnetRuntimeCommitSummary, BacnetIntegrationError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(BacnetIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = BacnetRuntimeCommitSummary {
        failures: report.failures.len(),
        ..BacnetRuntimeCommitSummary::default()
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
    config: &BacnetIpDiscoveryConfig,
    source: SocketAddrV4,
    response: &IAmResponse,
    discovered_at_ms: u64,
    expires_at_ms: u64,
) -> Result<DiscoveryRecord, DiscoveryError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        format!("device-{}", response.device_instance),
        DiscoverySource::UdpBroadcast,
        BridgeTransport::LanUdp,
        discovered_at_ms,
    )?
    .with_display_name(format!("BACnet Device {}", response.device_instance))
    .with_address(source.to_string())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata(
        "bacnet.device_instance",
        response.device_instance.to_string(),
    )
    .with_metadata(
        "bacnet.max_apdu_length_accepted",
        response.max_apdu_length_accepted.to_string(),
    )
    .with_metadata(
        "bacnet.segmentation_supported",
        response.segmentation_supported.as_str(),
    )
    .with_metadata("bacnet.vendor_id", response.vendor_id.to_string())
    .with_metadata("bacnet.bvlc_function", response.bvlc_function.as_str())
    .with_metadata(
        "bacnet.discovery_destination",
        config.destination.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    fn i_am(instance: u32) -> Vec<u8> {
        let object_id = (8u32 << 22) | instance;
        let mut bytes = vec![
            0x81, 0x0a, 0x00, 0x14, 0x01, 0x00, 0x10, 0x00, 0xc4, 0, 0, 0, 0, 0x22, 0x05, 0xc4,
            0x91, 0x03, 0x21, 0x63,
        ];
        bytes[9..13].copy_from_slice(&object_id.to_be_bytes());
        bytes
    }

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        replies: Vec<UdpDatagram>,
    }

    impl BacnetIpTransport for FakeTransport {
        fn discover(
            &mut self,
            _config: &BacnetIpDiscoveryConfig,
            probe: &[u8],
        ) -> Result<Vec<UdpDatagram>, BacnetIntegrationError> {
            assert_eq!(probe, [0x81, 0x0b, 0, 8, 1, 0, 0x10, 0x08]);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.replies.clone())
        }
    }

    fn datagram(source: SocketAddrV4, payload: Vec<u8>) -> UdpDatagram {
        UdpDatagram {
            source: SocketAddr::V4(source),
            destination: "127.0.0.1:47808".parse().unwrap(),
            payload,
        }
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ =
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                    CapabilityGrantId::trusted("grant:bacnet-test"),
                    principal.clone(),
                    PrivilegeTier::LowRisk,
                    "test",
                    0,
                ));
    }

    #[test]
    fn validates_configuration_bounds() {
        let mut config = BacnetIpDiscoveryConfig::new(Ipv4Addr::UNSPECIFIED);
        assert!(config.validate().is_err());
        config.destination = "127.0.0.1:47808".parse().unwrap();
        config.maximum_responses = 0;
        assert!(config.validate().is_err());
        config.maximum_responses = MAX_RESPONSES + 1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn normalizes_valid_replies_and_preserves_partial_failures() {
        let source: SocketAddrV4 = "192.0.2.10:47808".parse().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            replies: vec![datagram(source, i_am(123)), datagram(source, vec![0; 8])],
        };
        let config = BacnetIpDiscoveryConfig::new(Ipv4Addr::BROADCAST);
        let report = discover(&config, &mut transport, 1_000).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 1);
        let record = &report.records[0];
        assert_eq!(record.native_bridge_id, "device-123");
        assert_eq!(record.address.as_deref(), Some("192.0.2.10:47808"));
        assert_eq!(record.expires_at_ms, Some(301_000));
    }

    #[test]
    fn reports_conflicting_duplicate_device_instances() {
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            replies: vec![
                datagram("192.0.2.10:47808".parse().unwrap(), i_am(7)),
                datagram("192.0.2.11:47808".parse().unwrap(), i_am(7)),
            ],
        };
        let report = discover(
            &BacnetIpDiscoveryConfig::new(Ipv4Addr::BROADCAST),
            &mut transport,
            0,
        )
        .unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 1);
    }

    #[test]
    fn denies_before_transport_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            replies: Vec::new(),
        };
        let mut runtime = SmartHomeRuntime::new();
        let result = discover_into_runtime(
            &mut runtime,
            AgentId::trusted("agent:denied"),
            &BacnetIpDiscoveryConfig::new(Ipv4Addr::LOCALHOST),
            &mut transport,
            0,
        );
        assert!(matches!(
            result,
            Err(BacnetIntegrationError::Runtime(
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
            assert_eq!(&probe[..length], [0x81, 0x0b, 0, 8, 1, 0, 0x10, 0x08]);
            server.send_to(&i_am(321), source).unwrap();
        });

        let mut config = BacnetIpDiscoveryConfig::new(*destination.ip());
        config.destination = destination;
        config.timeout = Duration::from_millis(100);
        let principal = AgentId::trusted("agent:bacnet-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &config,
            &mut UdpBacnetIpTransport,
            2_000,
        )
        .unwrap();
        responder.join().unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "bacnet_ip.bridge.device-321"
            ))
            .is_some());
    }
}
