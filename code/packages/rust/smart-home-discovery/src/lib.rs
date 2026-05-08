//! Transport-neutral discovery records for the D23 smart-home runtime.
//!
//! Discovery workers can use this crate to normalize LAN, radio, USB, MQTT,
//! cloud, webhook, manual, or simulator findings before a runtime decides
//! whether to pair, ignore, or supervise a bridge. The crate deliberately
//! performs no I/O.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier,
};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    EmptyField { field: &'static str },
    DuplicateTxtKey { key: String },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "{field} must not be empty"),
            Self::DuplicateTxtKey { key } => {
                write!(f, "mDNS TXT key `{key}` appears more than once")
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoverySource {
    Mdns,
    Ssdp,
    Bluetooth,
    Usb,
    Dhcp,
    Mqtt,
    Manual,
    CloudFallback,
    Webhook,
    Simulator,
}

impl DiscoverySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mdns => "mdns",
            Self::Ssdp => "ssdp",
            Self::Bluetooth => "bluetooth",
            Self::Usb => "usb",
            Self::Dhcp => "dhcp",
            Self::Mqtt => "mqtt",
            Self::Manual => "manual",
            Self::CloudFallback => "cloud_fallback",
            Self::Webhook => "webhook",
            Self::Simulator => "simulator",
        }
    }

    pub fn preference_rank(self) -> u8 {
        match self {
            Self::Manual => 100,
            Self::Mdns => 80,
            Self::Ssdp => 70,
            Self::Usb => 65,
            Self::Bluetooth => 60,
            Self::Mqtt => 55,
            Self::Dhcp => 45,
            Self::Webhook => 45,
            Self::CloudFallback => 40,
            Self::Simulator => 10,
        }
    }
}

impl fmt::Display for DiscoverySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoveryConfidence {
    Hint,
    Candidate,
    Verified,
    Paired,
}

impl DiscoveryConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hint => "hint",
            Self::Candidate => "candidate",
            Self::Verified => "verified",
            Self::Paired => "paired",
        }
    }

    pub fn preference_rank(self) -> u8 {
        match self {
            Self::Hint => 10,
            Self::Candidate => 40,
            Self::Verified => 80,
            Self::Paired => 100,
        }
    }
}

impl fmt::Display for DiscoveryConfidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PairingRequirement {
    Unknown,
    None,
    PhysicalPresence,
    LocalCode,
    Credentials,
    OAuth2,
    Certificate,
    RadioInclusion,
    MqttCredentials,
}

impl PairingRequirement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::None => "none",
            Self::PhysicalPresence => "physical_presence",
            Self::LocalCode => "local_code",
            Self::Credentials => "credentials",
            Self::OAuth2 => "oauth2",
            Self::Certificate => "certificate",
            Self::RadioInclusion => "radio_inclusion",
            Self::MqttCredentials => "mqtt_credentials",
        }
    }
}

impl fmt::Display for PairingRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRecord {
    pub integration_id: IntegrationId,
    pub protocol_family: ProtocolFamily,
    pub native_bridge_id: String,
    pub display_name: Option<String>,
    pub source: DiscoverySource,
    pub transport: BridgeTransport,
    pub address: Option<String>,
    pub network_interface: Option<String>,
    pub hardware_model: Option<String>,
    pub firmware_version: Option<String>,
    pub confidence: DiscoveryConfidence,
    pub pairing_requirement: PairingRequirement,
    pub discovered_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub metadata: Vec<Metadata>,
}

impl DiscoveryRecord {
    pub fn new(
        integration_id: IntegrationId,
        protocol_family: ProtocolFamily,
        native_bridge_id: impl Into<String>,
        source: DiscoverySource,
        transport: BridgeTransport,
        discovered_at_ms: u64,
    ) -> Result<Self, DiscoveryError> {
        let native_bridge_id = non_empty("native_bridge_id", native_bridge_id)?;
        Ok(Self {
            integration_id,
            protocol_family,
            native_bridge_id,
            display_name: None,
            source,
            transport,
            address: None,
            network_interface: None,
            hardware_model: None,
            firmware_version: None,
            confidence: DiscoveryConfidence::Candidate,
            pairing_requirement: PairingRequirement::Unknown,
            discovered_at_ms,
            expires_at_ms: None,
            metadata: Vec::new(),
        })
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    pub fn with_address(mut self, address: impl Into<String>) -> Self {
        self.address = Some(address.into());
        self
    }

    pub fn with_network_interface(
        mut self,
        network_interface: impl Into<String>,
    ) -> Result<Self, DiscoveryError> {
        self.network_interface = Some(non_empty("network_interface", network_interface)?);
        Ok(self)
    }

    pub fn with_hardware_model(mut self, hardware_model: impl Into<String>) -> Self {
        self.hardware_model = Some(hardware_model.into());
        self
    }

    pub fn with_firmware_version(mut self, firmware_version: impl Into<String>) -> Self {
        self.firmware_version = Some(firmware_version.into());
        self
    }

    pub fn with_confidence(mut self, confidence: DiscoveryConfidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_pairing_requirement(mut self, pairing_requirement: PairingRequirement) -> Self {
        self.pairing_requirement = pairing_requirement;
        self
    }

    pub fn with_expires_at_ms(mut self, expires_at_ms: u64) -> Self {
        self.expires_at_ms = Some(expires_at_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn bridge_id(&self) -> BridgeId {
        BridgeId::trusted(format!(
            "{}.bridge.{}",
            self.integration_id.as_str(),
            self.native_bridge_id
        ))
    }

    pub fn protocol_identifier(&self) -> ProtocolIdentifier {
        ProtocolIdentifier::new(
            self.protocol_family.clone(),
            "bridge",
            self.native_bridge_id.as_str(),
        )
        .expect("discovery records validate native bridge ids")
    }

    pub fn age_ms_at(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.discovered_at_ms)
    }

    pub fn is_stale_at(&self, now_ms: u64, ttl_ms: u64) -> bool {
        self.is_expired_at(now_ms) || self.age_ms_at(now_ms) >= ttl_ms
    }

    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        self.expires_at_ms
            .is_some_and(|expires_at_ms| now_ms >= expires_at_ms)
    }

    pub fn has_address(&self) -> bool {
        self.address
            .as_ref()
            .is_some_and(|address| !address.is_empty())
    }

    pub fn preference_key(&self) -> DiscoveryPreferenceKey {
        DiscoveryPreferenceKey {
            confidence_rank: self.confidence.preference_rank(),
            source_rank: self.source.preference_rank(),
            has_address: self.has_address(),
            discovered_at_ms: self.discovered_at_ms,
        }
    }

    pub fn is_preferred_over(&self, other: &Self) -> bool {
        self.preference_key() > other.preference_key()
    }

    pub fn to_bridge_candidate(&self) -> Bridge {
        let mut bridge = Bridge::new(
            self.bridge_id(),
            self.integration_id.clone(),
            self.transport,
        );
        bridge.address = self.address.clone();
        bridge.hardware_model = self.hardware_model.clone();
        bridge.firmware_version = self.firmware_version.clone();
        bridge.health = Health::Unpaired;
        bridge.identifiers.push(self.protocol_identifier());
        bridge.metadata.push(Metadata::new(
            "smart_home.discovery.source",
            self.source.as_str(),
        ));
        bridge.metadata.push(Metadata::new(
            "smart_home.discovery.native_bridge_id",
            self.native_bridge_id.as_str(),
        ));
        bridge.metadata.push(Metadata::new(
            "smart_home.discovery.confidence",
            self.confidence.as_str(),
        ));
        bridge.metadata.push(Metadata::new(
            "smart_home.discovery.pairing_requirement",
            self.pairing_requirement.as_str(),
        ));
        if let Some(display_name) = &self.display_name {
            bridge.metadata.push(Metadata::new(
                "smart_home.discovery.display_name",
                display_name,
            ));
        }
        if let Some(network_interface) = &self.network_interface {
            bridge.metadata.push(Metadata::new(
                "smart_home.discovery.network_interface",
                network_interface,
            ));
        }
        if let Some(expires_at_ms) = self.expires_at_ms {
            bridge.metadata.push(Metadata::new(
                "smart_home.discovery.expires_at_ms",
                expires_at_ms.to_string(),
            ));
        }
        bridge.metadata.extend(self.metadata.clone());
        bridge
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveryPreferenceKey {
    pub confidence_rank: u8,
    pub source_rank: u8,
    pub has_address: bool,
    pub discovered_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualBridgeInput {
    pub integration_id: IntegrationId,
    pub protocol_family: ProtocolFamily,
    pub native_bridge_id: String,
    pub address: String,
    pub transport: BridgeTransport,
    pub discovered_at_ms: u64,
}

impl ManualBridgeInput {
    pub fn into_record(self) -> Result<DiscoveryRecord, DiscoveryError> {
        let address = non_empty("address", self.address)?;
        DiscoveryRecord::new(
            self.integration_id,
            self.protocol_family,
            self.native_bridge_id,
            DiscoverySource::Manual,
            self.transport,
            self.discovered_at_ms,
        )
        .map(|record| record.with_address(address))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsTxtEntry {
    pub key: String,
    pub value: String,
}

impl MdnsTxtEntry {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Result<Self, DiscoveryError> {
        Ok(Self {
            key: non_empty("txt.key", key)?,
            value: value.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsAdvertisement {
    pub service_type: String,
    pub instance_name: String,
    pub host_name: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub txt: Vec<MdnsTxtEntry>,
    pub discovered_at_ms: u64,
}

impl MdnsAdvertisement {
    pub fn new(
        service_type: impl Into<String>,
        instance_name: impl Into<String>,
        host_name: impl Into<String>,
        port: u16,
        discovered_at_ms: u64,
    ) -> Result<Self, DiscoveryError> {
        Ok(Self {
            service_type: non_empty("service_type", service_type)?,
            instance_name: non_empty("instance_name", instance_name)?,
            host_name: non_empty("host_name", host_name)?,
            addresses: Vec::new(),
            port,
            txt: Vec::new(),
            discovered_at_ms,
        })
    }

    pub fn with_address(mut self, address: impl Into<String>) -> Result<Self, DiscoveryError> {
        self.addresses.push(non_empty("address", address)?);
        Ok(self)
    }

    pub fn with_txt(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, DiscoveryError> {
        let entry = MdnsTxtEntry::new(key, value)?;
        if self.txt.iter().any(|existing| existing.key == entry.key) {
            return Err(DiscoveryError::DuplicateTxtKey { key: entry.key });
        }
        self.txt.push(entry);
        Ok(self)
    }

    pub fn txt_value(&self, key: &str) -> Option<&str> {
        self.txt
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.as_str())
    }

    pub fn preferred_address(&self) -> &str {
        self.addresses
            .first()
            .map(String::as_str)
            .unwrap_or(self.host_name.as_str())
    }

    pub fn endpoint_with_scheme(&self, scheme: &str) -> String {
        format!("{}://{}:{}", scheme, self.preferred_address(), self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveryKey {
    integration_id: String,
    native_bridge_id: String,
}

impl DiscoveryKey {
    pub fn new(integration_id: &IntegrationId, native_bridge_id: &str) -> Self {
        Self {
            integration_id: integration_id.as_str().to_string(),
            native_bridge_id: native_bridge_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiscoveryCatalog {
    records: BTreeMap<DiscoveryKey, DiscoveryRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryUpsert {
    Inserted,
    Replaced(DiscoveryRecord),
    Ignored(DiscoveryRecord),
}

impl DiscoveryCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, record: DiscoveryRecord) -> Option<DiscoveryRecord> {
        let key = DiscoveryKey::new(&record.integration_id, &record.native_bridge_id);
        self.records.insert(key, record)
    }

    pub fn record_preferred(&mut self, record: DiscoveryRecord) -> DiscoveryUpsert {
        let key = DiscoveryKey::new(&record.integration_id, &record.native_bridge_id);
        match self.records.get(&key) {
            None => {
                self.records.insert(key, record);
                DiscoveryUpsert::Inserted
            }
            Some(existing) if record.is_preferred_over(existing) => {
                let replaced = self
                    .records
                    .insert(key, record)
                    .expect("existing discovery record disappeared during replacement");
                DiscoveryUpsert::Replaced(replaced)
            }
            Some(_) => DiscoveryUpsert::Ignored(record),
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn get(
        &self,
        integration_id: &IntegrationId,
        native_bridge_id: &str,
    ) -> Option<&DiscoveryRecord> {
        self.records
            .get(&DiscoveryKey::new(integration_id, native_bridge_id))
    }

    pub fn records(&self) -> impl Iterator<Item = &DiscoveryRecord> {
        self.records.values()
    }

    pub fn records_for_integration(&self, integration_id: &IntegrationId) -> Vec<&DiscoveryRecord> {
        self.records
            .values()
            .filter(|record| &record.integration_id == integration_id)
            .collect()
    }

    pub fn records_for_source(&self, source: DiscoverySource) -> Vec<&DiscoveryRecord> {
        self.records
            .values()
            .filter(|record| record.source == source)
            .collect()
    }

    pub fn fresh_records(&self, now_ms: u64, ttl_ms: u64) -> Vec<&DiscoveryRecord> {
        self.records
            .values()
            .filter(|record| !record.is_stale_at(now_ms, ttl_ms))
            .collect()
    }

    pub fn bridge_candidates(&self) -> Vec<Bridge> {
        self.records()
            .map(DiscoveryRecord::to_bridge_candidate)
            .collect()
    }
}

fn non_empty(field: &'static str, value: impl Into<String>) -> Result<String, DiscoveryError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(DiscoveryError::EmptyField { field });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_bridge_input_projects_to_unpaired_bridge_candidate() {
        let record = ManualBridgeInput {
            integration_id: IntegrationId::trusted("hue"),
            protocol_family: ProtocolFamily::Hue,
            native_bridge_id: "001788fffeabcdef".to_string(),
            address: "https://192.0.2.10".to_string(),
            transport: BridgeTransport::LanHttp,
            discovered_at_ms: 1_000,
        }
        .into_record()
        .unwrap()
        .with_display_name("Living Room Bridge")
        .with_hardware_model("BSB002")
        .with_firmware_version("1.66.1960062030");

        let bridge = record.to_bridge_candidate();

        assert_eq!(record.source, DiscoverySource::Manual);
        assert_eq!(bridge.bridge_id.as_str(), "hue.bridge.001788fffeabcdef");
        assert_eq!(bridge.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(bridge.transport, BridgeTransport::LanHttp);
        assert_eq!(bridge.health, Health::Unpaired);
        assert_eq!(bridge.address.as_deref(), Some("https://192.0.2.10"));
        assert!(bridge.auth_ref.is_none());
        assert_eq!(bridge.identifiers[0].family, ProtocolFamily::Hue);
        assert!(bridge.metadata.iter().any(|metadata| metadata.key
            == "smart_home.discovery.display_name"
            && metadata.value == "Living Room Bridge"));
    }

    #[test]
    fn mdns_advertisements_expose_txt_and_endpoint_helpers() {
        let advertisement = MdnsAdvertisement::new(
            "_hue._tcp.local",
            "Philips Hue - ABCDEF",
            "hue-bridge.local",
            443,
            2_000,
        )
        .unwrap()
        .with_address("192.0.2.10")
        .unwrap()
        .with_txt("bridgeid", "001788fffeabcdef")
        .unwrap()
        .with_txt("modelid", "BSB002")
        .unwrap();

        assert_eq!(
            advertisement.txt_value("bridgeid"),
            Some("001788fffeabcdef")
        );
        assert_eq!(advertisement.preferred_address(), "192.0.2.10");
        assert_eq!(
            advertisement.endpoint_with_scheme("https"),
            "https://192.0.2.10:443"
        );
    }

    #[test]
    fn mdns_txt_keys_must_be_unique() {
        let error = MdnsAdvertisement::new("_hue._tcp.local", "Hue", "hue.local", 443, 2_000)
            .unwrap()
            .with_txt("bridgeid", "a")
            .unwrap()
            .with_txt("bridgeid", "b")
            .unwrap_err();

        assert_eq!(
            error,
            DiscoveryError::DuplicateTxtKey {
                key: "bridgeid".to_string()
            }
        );
    }

    #[test]
    fn catalog_replaces_candidates_by_integration_and_native_id() {
        let integration_id = IntegrationId::trusted("hue");
        let mut catalog = DiscoveryCatalog::new();
        let first = ManualBridgeInput {
            integration_id: integration_id.clone(),
            protocol_family: ProtocolFamily::Hue,
            native_bridge_id: "bridge-1".to_string(),
            address: "https://192.0.2.10".to_string(),
            transport: BridgeTransport::LanHttp,
            discovered_at_ms: 1_000,
        }
        .into_record()
        .unwrap();
        let second = ManualBridgeInput {
            address: "https://192.0.2.11".to_string(),
            discovered_at_ms: 2_000,
            ..ManualBridgeInput {
                integration_id: integration_id.clone(),
                protocol_family: ProtocolFamily::Hue,
                native_bridge_id: "bridge-1".to_string(),
                address: String::new(),
                transport: BridgeTransport::LanHttp,
                discovered_at_ms: 0,
            }
        }
        .into_record()
        .unwrap();

        assert!(catalog.is_empty());
        assert!(catalog.record(first).is_none());
        assert!(catalog.record(second).is_some());

        let stored = catalog.get(&integration_id, "bridge-1").unwrap();
        assert_eq!(catalog.len(), 1);
        assert_eq!(stored.address.as_deref(), Some("https://192.0.2.11"));
        assert_eq!(catalog.records_for_integration(&integration_id).len(), 1);
        assert_eq!(catalog.records_for_source(DiscoverySource::Manual).len(), 1);
        assert_eq!(catalog.bridge_candidates()[0].health, Health::Unpaired);
    }

    #[test]
    fn catalog_can_keep_preferred_candidate_for_same_bridge() {
        let integration_id = IntegrationId::trusted("hue");
        let manual = ManualBridgeInput {
            integration_id: integration_id.clone(),
            protocol_family: ProtocolFamily::Hue,
            native_bridge_id: "bridge-1".to_string(),
            address: "https://192.0.2.10".to_string(),
            transport: BridgeTransport::LanHttp,
            discovered_at_ms: 1_000,
        }
        .into_record()
        .unwrap();
        let mdns = DiscoveryRecord::new(
            integration_id.clone(),
            ProtocolFamily::Hue,
            "bridge-1",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            2_000,
        )
        .unwrap()
        .with_address("https://192.0.2.11");
        let cloud = DiscoveryRecord::new(
            integration_id.clone(),
            ProtocolFamily::Hue,
            "bridge-1",
            DiscoverySource::CloudFallback,
            BridgeTransport::Cloud,
            3_000,
        )
        .unwrap();

        let mut catalog = DiscoveryCatalog::new();

        assert_eq!(
            catalog.record_preferred(manual.clone()),
            DiscoveryUpsert::Inserted
        );
        assert_eq!(
            catalog.record_preferred(mdns.clone()),
            DiscoveryUpsert::Ignored(mdns)
        );
        assert_eq!(
            catalog.record_preferred(cloud.clone()),
            DiscoveryUpsert::Ignored(cloud)
        );
        assert_eq!(
            catalog.get(&integration_id, "bridge-1").unwrap().address,
            manual.address
        );
    }

    #[test]
    fn discovery_records_report_freshness_at_time() {
        let record = ManualBridgeInput {
            integration_id: IntegrationId::trusted("zwave"),
            protocol_family: ProtocolFamily::ZWave,
            native_bridge_id: "controller-1".to_string(),
            address: "/dev/tty.usbmodem1".to_string(),
            transport: BridgeTransport::Serial,
            discovered_at_ms: 1_000,
        }
        .into_record()
        .unwrap();
        let mut catalog = DiscoveryCatalog::new();
        catalog.record(record.clone());

        assert_eq!(record.age_ms_at(1_250), 250);
        assert!(!record.is_stale_at(1_499, 500));
        assert!(record.is_stale_at(1_500, 500));
        assert_eq!(catalog.fresh_records(1_499, 500).len(), 1);
        assert!(catalog.fresh_records(1_500, 500).is_empty());
    }

    #[test]
    fn discovery_records_can_carry_primitive_observation_metadata() {
        let record = DiscoveryRecord::new(
            IntegrationId::trusted("matter"),
            ProtocolFamily::Matter,
            "fabric-candidate-1",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            1_000,
        )
        .unwrap()
        .with_network_interface("en0")
        .unwrap()
        .with_confidence(DiscoveryConfidence::Verified)
        .with_pairing_requirement(PairingRequirement::Certificate)
        .with_expires_at_ms(2_000);

        let bridge = record.to_bridge_candidate();

        assert_eq!(record.network_interface.as_deref(), Some("en0"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::Certificate);
        assert!(!record.is_expired_at(1_999));
        assert!(record.is_expired_at(2_000));
        assert!(bridge.metadata.iter().any(|metadata| metadata.key
            == "smart_home.discovery.network_interface"
            && metadata.value == "en0"));
        assert!(bridge
            .metadata
            .iter()
            .any(|metadata| metadata.key == "smart_home.discovery.confidence"
                && metadata.value == "verified"));
        assert!(bridge.metadata.iter().any(|metadata| metadata.key
            == "smart_home.discovery.pairing_requirement"
            && metadata.value == "certificate"));
    }

    #[test]
    fn higher_confidence_observations_replace_lower_rank_sources() {
        let integration_id = IntegrationId::trusted("mqtt");
        let dhcp_hint = DiscoveryRecord::new(
            integration_id.clone(),
            ProtocolFamily::Mqtt,
            "broker-1",
            DiscoverySource::Dhcp,
            BridgeTransport::LanHttp,
            2_000,
        )
        .unwrap()
        .with_address("http://192.0.2.20")
        .with_confidence(DiscoveryConfidence::Hint);
        let mqtt_verified = DiscoveryRecord::new(
            integration_id.clone(),
            ProtocolFamily::Mqtt,
            "broker-1",
            DiscoverySource::Mqtt,
            BridgeTransport::LocalProcess,
            1_000,
        )
        .unwrap()
        .with_confidence(DiscoveryConfidence::Verified)
        .with_pairing_requirement(PairingRequirement::MqttCredentials);

        let mut catalog = DiscoveryCatalog::new();

        assert_eq!(
            catalog.record_preferred(dhcp_hint),
            DiscoveryUpsert::Inserted
        );
        assert!(matches!(
            catalog.record_preferred(mqtt_verified),
            DiscoveryUpsert::Replaced(_)
        ));
        assert_eq!(
            catalog.get(&integration_id, "broker-1").unwrap().source,
            DiscoverySource::Mqtt
        );
    }

    #[test]
    fn empty_manual_address_is_rejected() {
        let error = ManualBridgeInput {
            integration_id: IntegrationId::trusted("zwave"),
            protocol_family: ProtocolFamily::ZWave,
            native_bridge_id: "controller-1".to_string(),
            address: " ".to_string(),
            transport: BridgeTransport::Serial,
            discovered_at_ms: 1_000,
        }
        .into_record()
        .unwrap_err();

        assert_eq!(error, DiscoveryError::EmptyField { field: "address" });
    }
}
