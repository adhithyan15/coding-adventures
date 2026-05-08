//! First-party smart-home integration catalog model and seed entries.
//!
//! The catalog is intentionally pure data. It lets D23 runtime packages and
//! D18D tools answer "what can this system support?" without starting workers,
//! opening sockets, reading secrets, or probing the local network.

#![forbid(unsafe_code)]

use smart_home_core::{CapabilityId, EntityKind, IntegrationId, ProtocolFamily, RuntimeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationCategory {
    ProtocolStandard,
    LocalHub,
    LocalDevice,
    BluetoothProfile,
    CloudHub,
    CameraMedia,
    EnergyClimate,
    NotificationChannel,
    DataService,
    HelperCalculated,
    VirtualAlias,
    SystemHardware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectivityClass {
    LocalPush,
    LocalPolling,
    CloudPush,
    CloudPolling,
    Calculated,
    AssumedState,
}

impl ConnectivityClass {
    pub fn as_home_assistant_iot_class(self) -> &'static str {
        match self {
            Self::LocalPush => "local_push",
            Self::LocalPolling => "local_polling",
            Self::CloudPush => "cloud_push",
            Self::CloudPolling => "cloud_polling",
            Self::Calculated => "calculated",
            Self::AssumedState => "assumed_state",
        }
    }

    pub fn is_local(self) -> bool {
        matches!(self, Self::LocalPush | Self::LocalPolling)
    }

    pub fn requires_cloud(self) -> bool {
        matches!(self, Self::CloudPush | Self::CloudPolling)
    }

    pub fn is_push(self) -> bool {
        matches!(self, Self::LocalPush | Self::CloudPush)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscoveryMechanism {
    Mdns,
    Ssdp,
    Bluetooth,
    Usb,
    Dhcp,
    Mqtt,
    Manual,
    CloudAccount,
    Webhook,
    FileConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthMode {
    None,
    LocalPairing,
    LocalToken,
    UsernamePassword,
    OAuth2,
    ApiKey,
    Certificate,
    RadioNetworkKey,
    MqttCredentials,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImplementationStatus {
    Cataloged,
    Specified,
    Scaffolded,
    Simulated,
    FirstPartyRuntime,
    ProductionReady,
    DelegatedToStandard,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrimitiveFamily {
    NormalizedModel,
    DiscoveryIndex,
    Mdns,
    Ssdp,
    Dhcp,
    LocalHttp,
    WebSocket,
    ServerSentEvents,
    Mqtt,
    BluetoothLowEnergy,
    Usb,
    SerialController,
    Radio802154,
    ZWaveSerialApi,
    MatterCommissioning,
    HomeKitPairing,
    CloudApi,
    Webhook,
    OAuth2,
    LocalPairing,
    LocalToken,
    CertificatePairing,
    RadioNetworkKey,
    MqttCredentials,
    CameraMedia,
    EnergyTelemetry,
    CalculatedState,
    CommandMapping,
    CapabilityPolicy,
    VaultLease,
    Supervision,
    TestSimulator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceReference {
    pub label: String,
    pub url: String,
    pub external_id: Option<String>,
}

impl SourceReference {
    pub fn home_assistant(domain: &'static str) -> Self {
        Self {
            label: "Home Assistant".to_string(),
            url: format!("https://www.home-assistant.io/integrations/{domain}/"),
            external_id: Some(domain.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationCatalogEntry {
    pub integration_id: IntegrationId,
    pub display_name: String,
    pub summary: String,
    pub category: IntegrationCategory,
    pub connectivity: ConnectivityClass,
    pub runtime_kind: RuntimeKind,
    pub implementation_status: ImplementationStatus,
    pub priority: u8,
    pub discovery_mechanisms: Vec<DiscoveryMechanism>,
    pub auth_modes: Vec<AuthMode>,
    pub required_capabilities: Vec<CapabilityId>,
    pub target_entity_kinds: Vec<EntityKind>,
    pub supported_protocols: Vec<ProtocolFamily>,
    pub depends_on_integrations: Vec<IntegrationId>,
    pub virtual_target: Option<IntegrationId>,
    pub virtual_iot_standards: Vec<ProtocolFamily>,
    pub required_primitives: Vec<PrimitiveFamily>,
    pub source_refs: Vec<SourceReference>,
    pub notes: Vec<String>,
}

impl IntegrationCatalogEntry {
    pub fn is_virtual(&self) -> bool {
        self.category == IntegrationCategory::VirtualAlias
    }

    pub fn is_local(&self) -> bool {
        self.connectivity.is_local()
    }

    pub fn requires_cloud(&self) -> bool {
        self.connectivity.requires_cloud()
    }

    pub fn supports_capability(&self, capability_id: &CapabilityId) -> bool {
        self.required_capabilities
            .iter()
            .any(|candidate| candidate == capability_id)
    }

    pub fn uses_discovery(&self, mechanism: DiscoveryMechanism) -> bool {
        self.discovery_mechanisms.contains(&mechanism)
    }

    pub fn requires_primitive(&self, primitive: PrimitiveFamily) -> bool {
        self.required_primitives.contains(&primitive)
    }
}

pub fn first_party_catalog() -> Vec<IntegrationCatalogEntry> {
    vec![
        hue_entry(),
        protocol_entry(
            "zigbee",
            "Zigbee",
            "Repository-owned Zigbee stack and coordinator integration.",
            ConnectivityClass::LocalPolling,
            ImplementationStatus::Scaffolded,
            0,
            ProtocolFamily::Zigbee,
            &["smart_home.read", "smart_home.command.light", "smart_home.manage_network"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor],
            &[DiscoveryMechanism::Usb, DiscoveryMechanism::Manual],
            &[AuthMode::RadioNetworkKey],
            "zha",
        ),
        protocol_entry(
            "zwave",
            "Z-Wave",
            "Repository-owned Z-Wave controller, serial API, and command-class integration.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Scaffolded,
            0,
            ProtocolFamily::ZWave,
            &[
                "smart_home.read",
                "smart_home.command.light",
                "smart_home.command.lock",
                "smart_home.manage_network",
            ],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor, EntityKind::Lock],
            &[DiscoveryMechanism::Usb, DiscoveryMechanism::Manual],
            &[AuthMode::RadioNetworkKey],
            "zwave_js",
        ),
        protocol_entry(
            "thread",
            "Thread",
            "Repository-owned Thread networking and diagnostics integration.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Scaffolded,
            0,
            ProtocolFamily::Thread,
            &["smart_home.read", "smart_home.manage_network"],
            &[EntityKind::NetworkDiagnostic],
            &[DiscoveryMechanism::Usb, DiscoveryMechanism::Mdns, DiscoveryMechanism::Manual],
            &[AuthMode::RadioNetworkKey],
            "thread",
        ),
        protocol_entry(
            "mqtt",
            "MQTT",
            "MQTT broker integration for power-user devices, Tasmota, sensors, and bridge-style ecosystems.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Specified,
            1,
            ProtocolFamily::Mqtt,
            &["smart_home.read", "smart_home.command.light", "smart_home.command.switch"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor],
            &[DiscoveryMechanism::Mqtt, DiscoveryMechanism::Manual],
            &[AuthMode::MqttCredentials],
            "mqtt",
        ),
        protocol_entry(
            "matter",
            "Matter",
            "Matter controller integration over Thread, Wi-Fi, and Ethernet.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Specified,
            1,
            ProtocolFamily::Matter,
            &["smart_home.read", "smart_home.command.light", "smart_home.command.lock"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor, EntityKind::Lock],
            &[DiscoveryMechanism::Mdns, DiscoveryMechanism::Manual],
            &[AuthMode::LocalPairing, AuthMode::Certificate],
            "matter",
        ),
        local_device_entry(
            "matter_bridge",
            "Matter Bridge",
            "Catalog route for bridged Matter devices exposed by other ecosystems.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            1,
            &["smart_home.read", "smart_home.command.light", "smart_home.command.switch"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor],
            &[DiscoveryMechanism::Mdns, DiscoveryMechanism::Manual],
            &[AuthMode::LocalPairing, AuthMode::Certificate],
            "matter",
        )
        .with_primitives(&[
            PrimitiveFamily::MatterCommissioning,
            PrimitiveFamily::Mdns,
            PrimitiveFamily::LocalPairing,
            PrimitiveFamily::CertificatePairing,
        ]),
        local_device_entry(
            "homekit_controller",
            "HomeKit Controller",
            "Local controller for devices that expose the HomeKit Accessory Protocol.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Specified,
            1,
            &["smart_home.read", "smart_home.command.light", "smart_home.command.lock"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor, EntityKind::Lock],
            &[DiscoveryMechanism::Mdns, DiscoveryMechanism::Bluetooth, DiscoveryMechanism::Manual],
            &[AuthMode::LocalPairing],
            "homekit_controller",
        ),
        local_device_entry(
            "esphome",
            "ESPHome",
            "Local ESPHome device integration for DIY sensors, lights, switches, and voice devices.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Specified,
            1,
            &["smart_home.read", "smart_home.command.light", "smart_home.command.switch"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor, EntityKind::Input],
            &[DiscoveryMechanism::Mdns, DiscoveryMechanism::Usb, DiscoveryMechanism::Manual],
            &[AuthMode::LocalToken],
            "esphome",
        ),
        tasmota_entry(),
        local_device_entry(
            "shelly",
            "Shelly",
            "Local Shelly relay, switch, cover, sensor, and energy integration.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            2,
            &["smart_home.read", "smart_home.command.switch"],
            &[EntityKind::Switch, EntityKind::Sensor],
            &[DiscoveryMechanism::Mdns, DiscoveryMechanism::Dhcp, DiscoveryMechanism::Manual],
            &[AuthMode::None, AuthMode::UsernamePassword],
            "shelly",
        ),
        local_device_entry(
            "tplink",
            "TP-Link Smart Home",
            "Local TP-Link and Kasa device integration for plugs, lights, switches, and cameras.",
            ConnectivityClass::LocalPolling,
            ImplementationStatus::Cataloged,
            2,
            &["smart_home.read", "smart_home.command.light", "smart_home.command.switch"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor],
            &[DiscoveryMechanism::Manual, DiscoveryMechanism::Dhcp],
            &[AuthMode::None, AuthMode::UsernamePassword],
            "tplink",
        ),
        virtual_alias(
            "tplink_tapo",
            "Tapo",
            "Tapo products route through the TP-Link integration catalog entry.",
            "tplink",
            2,
            "tplink_tapo",
        ),
        local_device_entry(
            "wled",
            "WLED",
            "Local WLED light and effect integration.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            2,
            &["smart_home.read", "smart_home.command.light"],
            &[EntityKind::Light],
            &[DiscoveryMechanism::Mdns, DiscoveryMechanism::Manual],
            &[AuthMode::None],
            "wled",
        ),
        local_device_entry(
            "lifx",
            "LIFX",
            "Local LIFX light integration.",
            ConnectivityClass::LocalPolling,
            ImplementationStatus::Cataloged,
            2,
            &["smart_home.read", "smart_home.command.light"],
            &[EntityKind::Light],
            &[DiscoveryMechanism::Dhcp, DiscoveryMechanism::Manual],
            &[AuthMode::None],
            "lifx",
        ),
        local_device_entry(
            "govee_light_local",
            "Govee Lights Local",
            "Local LAN control path for Govee lights.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            2,
            &["smart_home.read", "smart_home.command.light"],
            &[EntityKind::Light],
            &[DiscoveryMechanism::Dhcp, DiscoveryMechanism::Manual],
            &[AuthMode::None],
            "govee_light_local",
        ),
        bluetooth_entry(
            "switchbot",
            "SwitchBot Bluetooth",
            "Bluetooth integration for SwitchBot buttons, meters, curtains, and locks.",
            ImplementationStatus::Cataloged,
            2,
            &["smart_home.read", "smart_home.command.switch", "smart_home.command.lock"],
            &[EntityKind::Switch, EntityKind::Sensor, EntityKind::Lock, EntityKind::Input],
            "switchbot",
        ),
        local_hub_entry(
            "unifi",
            "UniFi Network",
            "Local UniFi controller integration for presence, network, and device telemetry.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            2,
            &["smart_home.read", "smart_home.diagnostics"],
            &[EntityKind::Sensor, EntityKind::NetworkDiagnostic],
            &[DiscoveryMechanism::Manual],
            &[AuthMode::ApiKey, AuthMode::UsernamePassword],
            "unifi",
        ),
        media_entry(
            "sonos",
            "Sonos",
            "Local Sonos speaker and media-player integration.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            2,
            "sonos",
        ),
        media_entry(
            "cast",
            "Google Cast",
            "Local Cast media-player integration.",
            ConnectivityClass::LocalPolling,
            ImplementationStatus::Cataloged,
            2,
            "cast",
        ),
        camera_entry(
            "onvif",
            "ONVIF",
            "Local ONVIF camera integration.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            3,
            "onvif",
        ),
        camera_entry(
            "reolink",
            "Reolink",
            "Reolink camera, doorbell, siren, and sensor hub integration.",
            ConnectivityClass::LocalPush,
            ImplementationStatus::Cataloged,
            3,
            "reolink",
        ),
        camera_entry(
            "ring",
            "Ring",
            "Cloud Ring camera and doorbell integration.",
            ConnectivityClass::CloudPolling,
            ImplementationStatus::Cataloged,
            3,
            "ring",
        ),
        cloud_hub_entry(
            "tuya",
            "Tuya",
            "Cloud Tuya hub for broad long-tail product coverage.",
            ConnectivityClass::CloudPush,
            ImplementationStatus::Cataloged,
            5,
            &["smart_home.read", "smart_home.command.light", "smart_home.command.switch"],
            &[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor],
            "tuya",
        ),
        energy_entry(
            "enphase_envoy",
            "Enphase Envoy",
            "Local solar and energy telemetry integration.",
            ConnectivityClass::LocalPolling,
            ImplementationStatus::Cataloged,
            4,
            "enphase_envoy",
        ),
        energy_entry(
            "fronius",
            "Fronius",
            "Local inverter and solar telemetry integration.",
            ConnectivityClass::LocalPolling,
            ImplementationStatus::Cataloged,
            4,
            "fronius",
        ),
        energy_entry(
            "tesla_powerwall",
            "Tesla Powerwall",
            "Local battery and energy telemetry integration.",
            ConnectivityClass::LocalPolling,
            ImplementationStatus::Cataloged,
            4,
            "tesla_powerwall",
        ),
        cloud_hub_entry(
            "ecobee",
            "ecobee",
            "Cloud thermostat and occupancy integration.",
            ConnectivityClass::CloudPolling,
            ImplementationStatus::Cataloged,
            4,
            &["smart_home.read", "smart_home.command.climate"],
            &[EntityKind::Thermostat, EntityKind::Sensor],
            "ecobee",
        ),
        cloud_hub_entry(
            "nest",
            "Google Nest",
            "Cloud thermostat, camera, and sensor integration.",
            ConnectivityClass::CloudPush,
            ImplementationStatus::Cataloged,
            4,
            &[
                "smart_home.read",
                "smart_home.command.climate",
                "smart_home.command.camera",
            ],
            &[EntityKind::Thermostat, EntityKind::Sensor, EntityKind::Unknown],
            "nest",
        ),
        cloud_hub_entry(
            "home_connect",
            "Home Connect",
            "Cloud appliance integration used by several appliance-brand aliases.",
            ConnectivityClass::CloudPush,
            ImplementationStatus::Cataloged,
            4,
            &["smart_home.read", "smart_home.command.switch"],
            &[EntityKind::Switch, EntityKind::Sensor],
            "home_connect",
        ),
        virtual_alias(
            "symfonisk",
            "IKEA SYMFONISK",
            "SYMFONISK speakers are supported through the Sonos integration.",
            "sonos",
            2,
            "symfonisk",
        ),
        virtual_standard(
            "ultraloq",
            "Ultraloq",
            "Ultraloq products route through the Z-Wave standard.",
            ProtocolFamily::ZWave,
            2,
            "ultraloq",
        ),
    ]
}

pub fn find_entry<'a>(
    catalog: &'a [IntegrationCatalogEntry],
    integration_id: &IntegrationId,
) -> Option<&'a IntegrationCatalogEntry> {
    catalog
        .iter()
        .find(|entry| &entry.integration_id == integration_id)
}

pub fn entries_by_category(
    catalog: &[IntegrationCatalogEntry],
    category: IntegrationCategory,
) -> Vec<&IntegrationCatalogEntry> {
    catalog
        .iter()
        .filter(|entry| entry.category == category)
        .collect()
}

pub fn entries_by_connectivity(
    catalog: &[IntegrationCatalogEntry],
    connectivity: ConnectivityClass,
) -> Vec<&IntegrationCatalogEntry> {
    catalog
        .iter()
        .filter(|entry| entry.connectivity == connectivity)
        .collect()
}

pub fn entries_by_status(
    catalog: &[IntegrationCatalogEntry],
    status: ImplementationStatus,
) -> Vec<&IntegrationCatalogEntry> {
    catalog
        .iter()
        .filter(|entry| entry.implementation_status == status)
        .collect()
}

pub fn entries_requiring_capability<'a>(
    catalog: &'a [IntegrationCatalogEntry],
    capability_id: &CapabilityId,
) -> Vec<&'a IntegrationCatalogEntry> {
    catalog
        .iter()
        .filter(|entry| entry.supports_capability(capability_id))
        .collect()
}

pub fn entries_requiring_primitive(
    catalog: &[IntegrationCatalogEntry],
    primitive: PrimitiveFamily,
) -> Vec<&IntegrationCatalogEntry> {
    catalog
        .iter()
        .filter(|entry| entry.requires_primitive(primitive))
        .collect()
}

pub fn entries_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
) -> Vec<&IntegrationCatalogEntry> {
    catalog
        .iter()
        .filter(|entry| entry.priority <= priority)
        .collect()
}

fn hue_entry() -> IntegrationCatalogEntry {
    base_entry(
        "hue",
        "Philips Hue",
        "Hue bridge integration with CLIP v2 resources, pairing, event stream, and command mapping.",
        IntegrationCategory::LocalHub,
        ConnectivityClass::LocalPush,
        ImplementationStatus::Scaffolded,
        0,
        "hue",
    )
    .with_capabilities(&[
        "smart_home.read",
        "smart_home.command.light",
        "smart_home.pair",
    ])
    .with_entities(&[
        EntityKind::Light,
        EntityKind::LightGroup,
        EntityKind::Scene,
        EntityKind::Sensor,
        EntityKind::Input,
    ])
    .with_protocols(vec![ProtocolFamily::Hue, ProtocolFamily::Zigbee])
    .with_discovery(&[DiscoveryMechanism::Mdns, DiscoveryMechanism::Manual])
    .with_auth(&[AuthMode::LocalPairing, AuthMode::LocalToken])
    .with_primitives(&[
        PrimitiveFamily::Mdns,
        PrimitiveFamily::LocalHttp,
        PrimitiveFamily::ServerSentEvents,
        PrimitiveFamily::LocalPairing,
        PrimitiveFamily::LocalToken,
        PrimitiveFamily::CommandMapping,
        PrimitiveFamily::VaultLease,
        PrimitiveFamily::Supervision,
        PrimitiveFamily::TestSimulator,
    ])
}

#[allow(clippy::too_many_arguments)]
fn protocol_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    connectivity: ConnectivityClass,
    status: ImplementationStatus,
    priority: u8,
    protocol: ProtocolFamily,
    capabilities: &[&'static str],
    entities: &[EntityKind],
    discovery: &[DiscoveryMechanism],
    auth: &[AuthMode],
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::ProtocolStandard,
        connectivity,
        status,
        priority,
        ha_domain,
    )
    .with_protocols(vec![protocol.clone()])
    .with_capabilities(capabilities)
    .with_entities(entities)
    .with_discovery(discovery)
    .with_auth(auth)
    .with_primitives(protocol_primitives(&protocol))
}

#[allow(clippy::too_many_arguments)]
fn local_hub_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    connectivity: ConnectivityClass,
    status: ImplementationStatus,
    priority: u8,
    capabilities: &[&'static str],
    entities: &[EntityKind],
    discovery: &[DiscoveryMechanism],
    auth: &[AuthMode],
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::LocalHub,
        connectivity,
        status,
        priority,
        ha_domain,
    )
    .with_capabilities(capabilities)
    .with_entities(entities)
    .with_discovery(discovery)
    .with_auth(auth)
    .with_primitives(&local_transport_primitives(connectivity, discovery, auth))
}

#[allow(clippy::too_many_arguments)]
fn local_device_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    connectivity: ConnectivityClass,
    status: ImplementationStatus,
    priority: u8,
    capabilities: &[&'static str],
    entities: &[EntityKind],
    discovery: &[DiscoveryMechanism],
    auth: &[AuthMode],
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::LocalDevice,
        connectivity,
        status,
        priority,
        ha_domain,
    )
    .with_capabilities(capabilities)
    .with_entities(entities)
    .with_discovery(discovery)
    .with_auth(auth)
    .with_primitives(&local_transport_primitives(connectivity, discovery, auth))
}

#[allow(clippy::too_many_arguments)]
fn bluetooth_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    status: ImplementationStatus,
    priority: u8,
    capabilities: &[&'static str],
    entities: &[EntityKind],
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::BluetoothProfile,
        ConnectivityClass::LocalPush,
        status,
        priority,
        ha_domain,
    )
    .with_capabilities(capabilities)
    .with_entities(entities)
    .with_discovery(&[DiscoveryMechanism::Bluetooth])
    .with_auth(&[AuthMode::None, AuthMode::LocalPairing])
    .with_primitives(&[
        PrimitiveFamily::BluetoothLowEnergy,
        PrimitiveFamily::LocalPairing,
        PrimitiveFamily::Supervision,
    ])
}

fn media_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    connectivity: ConnectivityClass,
    status: ImplementationStatus,
    priority: u8,
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::CameraMedia,
        connectivity,
        status,
        priority,
        ha_domain,
    )
    .with_capabilities(&["smart_home.read", "smart_home.command.media"])
    .with_entities(&[EntityKind::Unknown])
    .with_discovery(&[
        DiscoveryMechanism::Mdns,
        DiscoveryMechanism::Ssdp,
        DiscoveryMechanism::Manual,
    ])
    .with_auth(&[AuthMode::None, AuthMode::LocalToken])
    .with_primitives(&[
        PrimitiveFamily::Mdns,
        PrimitiveFamily::Ssdp,
        PrimitiveFamily::LocalHttp,
        PrimitiveFamily::LocalToken,
        PrimitiveFamily::CommandMapping,
        PrimitiveFamily::Supervision,
    ])
}

fn camera_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    connectivity: ConnectivityClass,
    status: ImplementationStatus,
    priority: u8,
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::CameraMedia,
        connectivity,
        status,
        priority,
        ha_domain,
    )
    .with_capabilities(&["smart_home.read", "smart_home.command.camera"])
    .with_entities(&[EntityKind::Unknown, EntityKind::Sensor])
    .with_discovery(&[DiscoveryMechanism::Mdns, DiscoveryMechanism::Manual])
    .with_auth(&[AuthMode::UsernamePassword, AuthMode::ApiKey])
    .with_primitives(&[
        PrimitiveFamily::Mdns,
        PrimitiveFamily::LocalHttp,
        PrimitiveFamily::CameraMedia,
        PrimitiveFamily::CapabilityPolicy,
        PrimitiveFamily::VaultLease,
        PrimitiveFamily::Supervision,
    ])
    .with_notes(&["Camera/media integrations require privacy-sensitive D21 policy."])
}

fn energy_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    connectivity: ConnectivityClass,
    status: ImplementationStatus,
    priority: u8,
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::EnergyClimate,
        connectivity,
        status,
        priority,
        ha_domain,
    )
    .with_capabilities(&["smart_home.read", "smart_home.command.energy"])
    .with_entities(&[EntityKind::Sensor])
    .with_discovery(&[
        DiscoveryMechanism::Mdns,
        DiscoveryMechanism::Dhcp,
        DiscoveryMechanism::Manual,
    ])
    .with_auth(&[
        AuthMode::None,
        AuthMode::LocalToken,
        AuthMode::UsernamePassword,
    ])
    .with_primitives(&[
        PrimitiveFamily::Mdns,
        PrimitiveFamily::Dhcp,
        PrimitiveFamily::LocalHttp,
        PrimitiveFamily::EnergyTelemetry,
        PrimitiveFamily::VaultLease,
        PrimitiveFamily::Supervision,
    ])
}

#[allow(clippy::too_many_arguments)]
fn cloud_hub_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    connectivity: ConnectivityClass,
    status: ImplementationStatus,
    priority: u8,
    capabilities: &[&'static str],
    entities: &[EntityKind],
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::CloudHub,
        connectivity,
        status,
        priority,
        ha_domain,
    )
    .with_capabilities(capabilities)
    .with_entities(entities)
    .with_discovery(&[
        DiscoveryMechanism::CloudAccount,
        DiscoveryMechanism::Webhook,
    ])
    .with_auth(&[AuthMode::OAuth2, AuthMode::ApiKey])
    .with_primitives(&[
        PrimitiveFamily::CloudApi,
        PrimitiveFamily::Webhook,
        PrimitiveFamily::OAuth2,
        PrimitiveFamily::CommandMapping,
        PrimitiveFamily::CapabilityPolicy,
        PrimitiveFamily::VaultLease,
        PrimitiveFamily::Supervision,
    ])
}

fn tasmota_entry() -> IntegrationCatalogEntry {
    base_entry(
        "tasmota",
        "Tasmota",
        "MQTT-native Tasmota device integration.",
        IntegrationCategory::LocalDevice,
        ConnectivityClass::LocalPush,
        ImplementationStatus::DelegatedToStandard,
        1,
        "tasmota",
    )
    .with_capabilities(&[
        "smart_home.read",
        "smart_home.command.light",
        "smart_home.command.switch",
    ])
    .with_entities(&[EntityKind::Light, EntityKind::Switch, EntityKind::Sensor])
    .with_discovery(&[DiscoveryMechanism::Mqtt])
    .with_auth(&[AuthMode::MqttCredentials])
    .with_dependencies(&["mqtt"])
    .with_protocols(vec![ProtocolFamily::Mqtt])
    .with_primitives(&[
        PrimitiveFamily::Mqtt,
        PrimitiveFamily::MqttCredentials,
        PrimitiveFamily::CommandMapping,
        PrimitiveFamily::CapabilityPolicy,
        PrimitiveFamily::Supervision,
    ])
}

fn virtual_alias(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    target: &'static str,
    priority: u8,
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::VirtualAlias,
        ConnectivityClass::Calculated,
        ImplementationStatus::DelegatedToStandard,
        priority,
        ha_domain,
    )
    .with_virtual_target(target)
    .with_primitives(&[
        PrimitiveFamily::NormalizedModel,
        PrimitiveFamily::DiscoveryIndex,
        PrimitiveFamily::CalculatedState,
    ])
    .with_notes(&["Virtual aliases route users and agents to an implementation entry."])
}

fn virtual_standard(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    standard: ProtocolFamily,
    priority: u8,
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    base_entry(
        id,
        name,
        summary,
        IntegrationCategory::VirtualAlias,
        ConnectivityClass::Calculated,
        ImplementationStatus::DelegatedToStandard,
        priority,
        ha_domain,
    )
    .with_virtual_iot_standards(vec![standard])
    .with_primitives(&[
        PrimitiveFamily::NormalizedModel,
        PrimitiveFamily::DiscoveryIndex,
        PrimitiveFamily::CalculatedState,
    ])
    .with_notes(&["Virtual standard aliases route pairing through the protocol stack."])
}

#[allow(clippy::too_many_arguments)]
fn base_entry(
    id: &'static str,
    name: &'static str,
    summary: &'static str,
    category: IntegrationCategory,
    connectivity: ConnectivityClass,
    implementation_status: ImplementationStatus,
    priority: u8,
    ha_domain: &'static str,
) -> IntegrationCatalogEntry {
    IntegrationCatalogEntry {
        integration_id: IntegrationId::trusted(id),
        display_name: name.to_string(),
        summary: summary.to_string(),
        category,
        connectivity,
        runtime_kind: if category == IntegrationCategory::VirtualAlias {
            RuntimeKind::InProcessRust
        } else {
            RuntimeKind::RustWorkerProcess
        },
        implementation_status,
        priority,
        discovery_mechanisms: Vec::new(),
        auth_modes: Vec::new(),
        required_capabilities: vec![capability("smart_home.read")],
        target_entity_kinds: Vec::new(),
        supported_protocols: Vec::new(),
        depends_on_integrations: Vec::new(),
        virtual_target: None,
        virtual_iot_standards: Vec::new(),
        required_primitives: vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::VaultLease,
            PrimitiveFamily::Supervision,
        ],
        source_refs: vec![SourceReference::home_assistant(ha_domain)],
        notes: Vec::new(),
    }
}

trait EntryBuilder {
    fn with_capabilities(self, capabilities: &[&'static str]) -> Self;
    fn with_entities(self, entities: &[EntityKind]) -> Self;
    fn with_discovery(self, discovery: &[DiscoveryMechanism]) -> Self;
    fn with_auth(self, auth_modes: &[AuthMode]) -> Self;
    fn with_protocols(self, protocols: Vec<ProtocolFamily>) -> Self;
    fn with_dependencies(self, integration_ids: &[&'static str]) -> Self;
    fn with_virtual_target(self, integration_id: &'static str) -> Self;
    fn with_virtual_iot_standards(self, standards: Vec<ProtocolFamily>) -> Self;
    fn with_primitives(self, primitives: &[PrimitiveFamily]) -> Self;
    fn with_notes(self, notes: &[&'static str]) -> Self;
}

impl EntryBuilder for IntegrationCatalogEntry {
    fn with_capabilities(mut self, capabilities: &[&'static str]) -> Self {
        self.required_capabilities = dedupe_capabilities(capabilities);
        self
    }

    fn with_entities(mut self, entities: &[EntityKind]) -> Self {
        self.target_entity_kinds = entities.to_vec();
        self
    }

    fn with_discovery(mut self, discovery: &[DiscoveryMechanism]) -> Self {
        self.discovery_mechanisms = discovery.to_vec();
        self
    }

    fn with_auth(mut self, auth_modes: &[AuthMode]) -> Self {
        self.auth_modes = auth_modes.to_vec();
        self
    }

    fn with_protocols(mut self, protocols: Vec<ProtocolFamily>) -> Self {
        self.supported_protocols = protocols;
        self
    }

    fn with_dependencies(mut self, integration_ids: &[&'static str]) -> Self {
        self.depends_on_integrations = integration_ids
            .iter()
            .map(|integration_id| IntegrationId::trusted(*integration_id))
            .collect();
        self
    }

    fn with_virtual_target(mut self, integration_id: &'static str) -> Self {
        self.virtual_target = Some(IntegrationId::trusted(integration_id));
        self
    }

    fn with_virtual_iot_standards(mut self, standards: Vec<ProtocolFamily>) -> Self {
        self.virtual_iot_standards = standards;
        self
    }

    fn with_primitives(mut self, primitives: &[PrimitiveFamily]) -> Self {
        for primitive in primitives {
            if !self.required_primitives.contains(primitive) {
                self.required_primitives.push(*primitive);
            }
        }
        self
    }

    fn with_notes(mut self, notes: &[&'static str]) -> Self {
        self.notes = notes.iter().map(|note| (*note).to_string()).collect();
        self
    }
}

fn protocol_primitives(protocol: &ProtocolFamily) -> &'static [PrimitiveFamily] {
    match protocol {
        ProtocolFamily::Hue => &[
            PrimitiveFamily::Mdns,
            PrimitiveFamily::LocalHttp,
            PrimitiveFamily::ServerSentEvents,
            PrimitiveFamily::LocalPairing,
        ],
        ProtocolFamily::Zigbee => &[
            PrimitiveFamily::Usb,
            PrimitiveFamily::SerialController,
            PrimitiveFamily::Radio802154,
            PrimitiveFamily::RadioNetworkKey,
        ],
        ProtocolFamily::ZWave => &[
            PrimitiveFamily::Usb,
            PrimitiveFamily::SerialController,
            PrimitiveFamily::ZWaveSerialApi,
            PrimitiveFamily::RadioNetworkKey,
        ],
        ProtocolFamily::Thread => &[
            PrimitiveFamily::Usb,
            PrimitiveFamily::SerialController,
            PrimitiveFamily::Radio802154,
            PrimitiveFamily::Mdns,
            PrimitiveFamily::RadioNetworkKey,
        ],
        ProtocolFamily::Matter => &[
            PrimitiveFamily::Mdns,
            PrimitiveFamily::MatterCommissioning,
            PrimitiveFamily::CertificatePairing,
            PrimitiveFamily::LocalPairing,
        ],
        ProtocolFamily::Mqtt => &[
            PrimitiveFamily::Mqtt,
            PrimitiveFamily::MqttCredentials,
            PrimitiveFamily::CommandMapping,
        ],
        ProtocolFamily::Vendor(_) => &[PrimitiveFamily::LocalHttp, PrimitiveFamily::CommandMapping],
    }
}

fn local_transport_primitives(
    connectivity: ConnectivityClass,
    discovery: &[DiscoveryMechanism],
    auth: &[AuthMode],
) -> Vec<PrimitiveFamily> {
    let mut primitives = vec![
        PrimitiveFamily::LocalHttp,
        PrimitiveFamily::CommandMapping,
        PrimitiveFamily::Supervision,
    ];

    if connectivity.is_push() {
        primitives.push(PrimitiveFamily::WebSocket);
    }

    for mechanism in discovery {
        let primitive = match mechanism {
            DiscoveryMechanism::Mdns => Some(PrimitiveFamily::Mdns),
            DiscoveryMechanism::Ssdp => Some(PrimitiveFamily::Ssdp),
            DiscoveryMechanism::Bluetooth => Some(PrimitiveFamily::BluetoothLowEnergy),
            DiscoveryMechanism::Usb => Some(PrimitiveFamily::Usb),
            DiscoveryMechanism::Dhcp => Some(PrimitiveFamily::Dhcp),
            DiscoveryMechanism::Mqtt => Some(PrimitiveFamily::Mqtt),
            DiscoveryMechanism::Manual
            | DiscoveryMechanism::CloudAccount
            | DiscoveryMechanism::Webhook
            | DiscoveryMechanism::FileConfig => None,
        };
        if let Some(primitive) = primitive {
            primitives.push(primitive);
        }
    }

    for mode in auth {
        let primitive = match mode {
            AuthMode::None => None,
            AuthMode::LocalPairing => Some(PrimitiveFamily::LocalPairing),
            AuthMode::LocalToken => Some(PrimitiveFamily::LocalToken),
            AuthMode::UsernamePassword | AuthMode::ApiKey => Some(PrimitiveFamily::VaultLease),
            AuthMode::OAuth2 => Some(PrimitiveFamily::OAuth2),
            AuthMode::Certificate => Some(PrimitiveFamily::CertificatePairing),
            AuthMode::RadioNetworkKey => Some(PrimitiveFamily::RadioNetworkKey),
            AuthMode::MqttCredentials => Some(PrimitiveFamily::MqttCredentials),
        };
        if let Some(primitive) = primitive {
            primitives.push(primitive);
        }
    }

    dedupe_primitives(primitives)
}

fn dedupe_primitives(primitives: Vec<PrimitiveFamily>) -> Vec<PrimitiveFamily> {
    let mut result = Vec::new();
    for primitive in primitives {
        if !result.contains(&primitive) {
            result.push(primitive);
        }
    }
    result
}

fn dedupe_capabilities(capabilities: &[&'static str]) -> Vec<CapabilityId> {
    let mut result = Vec::new();
    for capability_id in capabilities {
        let capability_id = capability(capability_id);
        if !result.contains(&capability_id) {
            result.push(capability_id);
        }
    }
    result
}

fn capability(value: &'static str) -> CapabilityId {
    CapabilityId::trusted(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_party_catalog_includes_current_and_multiplier_integrations() {
        let catalog = first_party_catalog();

        assert!(find_entry(&catalog, &IntegrationId::trusted("hue")).is_some());
        assert!(find_entry(&catalog, &IntegrationId::trusted("mqtt")).is_some());
        assert!(find_entry(&catalog, &IntegrationId::trusted("matter")).is_some());
        assert!(find_entry(&catalog, &IntegrationId::trusted("esphome")).is_some());
        assert!(catalog.len() >= 30);
    }

    #[test]
    fn connectivity_class_preserves_home_assistant_iot_class_names() {
        assert_eq!(
            ConnectivityClass::LocalPush.as_home_assistant_iot_class(),
            "local_push"
        );
        assert!(ConnectivityClass::CloudPolling.requires_cloud());
        assert!(ConnectivityClass::LocalPolling.is_local());
        assert!(ConnectivityClass::CloudPush.is_push());
    }

    #[test]
    fn virtual_aliases_route_to_real_targets_or_standards() {
        let catalog = first_party_catalog();
        let tapo = find_entry(&catalog, &IntegrationId::trusted("tplink_tapo")).unwrap();
        let ultraloq = find_entry(&catalog, &IntegrationId::trusted("ultraloq")).unwrap();

        assert!(tapo.is_virtual());
        assert_eq!(tapo.virtual_target, Some(IntegrationId::trusted("tplink")));
        assert_eq!(ultraloq.virtual_iot_standards, vec![ProtocolFamily::ZWave]);
        assert_eq!(tapo.runtime_kind, RuntimeKind::InProcessRust);
    }

    #[test]
    fn capability_queries_find_high_risk_surfaces() {
        let catalog = first_party_catalog();
        let lock_entries = entries_requiring_capability(
            &catalog,
            &CapabilityId::trusted("smart_home.command.lock"),
        );

        assert!(lock_entries
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("zwave")));
        assert!(lock_entries
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("matter")));
    }

    #[test]
    fn priority_queries_include_wave_zero_and_one() {
        let catalog = first_party_catalog();
        let early = entries_at_or_before_priority(&catalog, 1);

        assert!(early
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("hue")));
        assert!(early
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("mqtt")));
        assert!(!early
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("tuya")));
    }

    #[test]
    fn local_push_entries_can_be_filtered_for_supervision_shape() {
        let catalog = first_party_catalog();
        let local_push = entries_by_connectivity(&catalog, ConnectivityClass::LocalPush);

        assert!(local_push
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("hue")));
        assert!(local_push
            .iter()
            .all(|entry| entry.connectivity == ConnectivityClass::LocalPush));
    }

    #[test]
    fn hue_records_the_trial_run_primitive_shape() {
        let catalog = first_party_catalog();
        let hue = find_entry(&catalog, &IntegrationId::trusted("hue")).unwrap();

        assert!(hue.requires_primitive(PrimitiveFamily::Mdns));
        assert!(hue.requires_primitive(PrimitiveFamily::LocalHttp));
        assert!(hue.requires_primitive(PrimitiveFamily::ServerSentEvents));
        assert!(hue.requires_primitive(PrimitiveFamily::LocalPairing));
        assert!(hue.requires_primitive(PrimitiveFamily::Supervision));
    }

    #[test]
    fn primitive_queries_group_delegated_mqtt_device_families() {
        let catalog = first_party_catalog();
        let mqtt_entries = entries_requiring_primitive(&catalog, PrimitiveFamily::Mqtt);

        assert!(mqtt_entries
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("mqtt")));
        assert!(mqtt_entries
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("tasmota")));
    }

    #[test]
    fn camera_entries_are_marked_as_privacy_sensitive_primitives() {
        let catalog = first_party_catalog();
        let camera_entries = entries_requiring_primitive(&catalog, PrimitiveFamily::CameraMedia);

        assert!(camera_entries
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("onvif")));
        assert!(camera_entries.iter().all(|entry| entry
            .required_primitives
            .contains(&PrimitiveFamily::CapabilityPolicy)));
    }
}
