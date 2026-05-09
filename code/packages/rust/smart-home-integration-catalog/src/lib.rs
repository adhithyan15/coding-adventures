//! First-party smart-home integration catalog model and seed entries.
//!
//! The catalog is intentionally pure data. It lets D23 runtime packages and
//! D18D tools answer "what can this system support?" without starting workers,
//! opening sockets, reading secrets, or probing the local network.

#![forbid(unsafe_code)]

use smart_home_core::{
    CapabilityId, EntityKind, IntegrationId, PrivilegeTier, ProtocolFamily, RuntimeKind,
    ToolDescriptor, ToolSideEffects,
};

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

impl PrimitiveFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NormalizedModel => "normalized_model",
            Self::DiscoveryIndex => "discovery_index",
            Self::Mdns => "mdns",
            Self::Ssdp => "ssdp",
            Self::Dhcp => "dhcp",
            Self::LocalHttp => "local_http",
            Self::WebSocket => "websocket",
            Self::ServerSentEvents => "server_sent_events",
            Self::Mqtt => "mqtt",
            Self::BluetoothLowEnergy => "bluetooth_low_energy",
            Self::Usb => "usb",
            Self::SerialController => "serial_controller",
            Self::Radio802154 => "radio_802154",
            Self::ZWaveSerialApi => "zwave_serial_api",
            Self::MatterCommissioning => "matter_commissioning",
            Self::HomeKitPairing => "homekit_pairing",
            Self::CloudApi => "cloud_api",
            Self::Webhook => "webhook",
            Self::OAuth2 => "oauth2",
            Self::LocalPairing => "local_pairing",
            Self::LocalToken => "local_token",
            Self::CertificatePairing => "certificate_pairing",
            Self::RadioNetworkKey => "radio_network_key",
            Self::MqttCredentials => "mqtt_credentials",
            Self::CameraMedia => "camera_media",
            Self::EnergyTelemetry => "energy_telemetry",
            Self::CalculatedState => "calculated_state",
            Self::CommandMapping => "command_mapping",
            Self::CapabilityPolicy => "capability_policy",
            Self::VaultLease => "vault_lease",
            Self::Supervision => "supervision",
            Self::TestSimulator => "test_simulator",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationPolicySurface {
    LocalActuation,
    EntryAccess,
    ClimateControl,
    CameraMedia,
    EnergyManagement,
    CredentialLease,
    CredentialedCloud,
    RadioNetworkManagement,
    NetworkInfrastructure,
}

impl IntegrationPolicySurface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalActuation => "local_actuation",
            Self::EntryAccess => "entry_access",
            Self::ClimateControl => "climate_control",
            Self::CameraMedia => "camera_media",
            Self::EnergyManagement => "energy_management",
            Self::CredentialLease => "credential_lease",
            Self::CredentialedCloud => "credentialed_cloud",
            Self::RadioNetworkManagement => "radio_network_management",
            Self::NetworkInfrastructure => "network_infrastructure",
        }
    }

    pub fn required_tier(self) -> PrivilegeTier {
        match self {
            Self::EntryAccess => PrivilegeTier::HighRisk,
            Self::LocalActuation => PrivilegeTier::LowRisk,
            Self::ClimateControl
            | Self::CameraMedia
            | Self::EnergyManagement
            | Self::CredentialLease
            | Self::CredentialedCloud
            | Self::RadioNetworkManagement
            | Self::NetworkInfrastructure => PrivilegeTier::HumanApproval,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationCatalogTool {
    ListIntegrations,
    DescribeIntegration,
    ListPrimitives,
    DescribePrimitive,
}

impl IntegrationCatalogTool {
    pub fn tool_id(self) -> &'static str {
        match self {
            Self::ListIntegrations => "smart_home.list_integrations",
            Self::DescribeIntegration => "smart_home.describe_integration",
            Self::ListPrimitives => "smart_home.list_primitives",
            Self::DescribePrimitive => "smart_home.describe_primitive",
        }
    }

    pub fn descriptor(self) -> ToolDescriptor {
        read_catalog_tool(self.tool_id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveFamilyDescriptor {
    pub primitive: PrimitiveFamily,
    pub display_name: &'static str,
    pub summary: &'static str,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EcosystemSurveyPlatform {
    HomeAssistant,
    Hubitat,
    HomeyPro,
    SmartThings,
    OpenHab,
    Homebridge,
    IoBroker,
    Domoticz,
    Jeedom,
    HomeSeer,
    AppleHome,
    GoogleHome,
    AmazonAlexa,
    ZWaveAlliance,
    ThreadGroup,
}

impl EcosystemSurveyPlatform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HomeAssistant => "home_assistant",
            Self::Hubitat => "hubitat",
            Self::HomeyPro => "homey_pro",
            Self::SmartThings => "smartthings",
            Self::OpenHab => "openhab",
            Self::Homebridge => "homebridge",
            Self::IoBroker => "iobroker",
            Self::Domoticz => "domoticz",
            Self::Jeedom => "jeedom",
            Self::HomeSeer => "homeseer",
            Self::AppleHome => "apple_home",
            Self::GoogleHome => "google_home",
            Self::AmazonAlexa => "amazon_alexa",
            Self::ZWaveAlliance => "zwave_alliance",
            Self::ThreadGroup => "thread_group",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcosystemSurveySource {
    pub platform: EcosystemSurveyPlatform,
    pub display_name: &'static str,
    pub source_url: &'static str,
    pub source_surface: &'static str,
    pub contributes: &'static str,
    pub primitive_hints: Vec<PrimitiveFamily>,
}

impl EcosystemSurveySource {
    pub fn requires_primitive(&self, primitive: PrimitiveFamily) -> bool {
        self.primitive_hints.contains(&primitive)
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

    pub fn policy_surfaces(&self) -> Vec<IntegrationPolicySurface> {
        policy_surfaces_for_entry(self)
    }

    pub fn has_policy_surface(&self, surface: IntegrationPolicySurface) -> bool {
        self.policy_surfaces().contains(&surface)
    }

    pub fn highest_policy_tier(&self) -> PrivilegeTier {
        self.policy_surfaces()
            .into_iter()
            .map(IntegrationPolicySurface::required_tier)
            .max()
            .unwrap_or(PrivilegeTier::ReadOnly)
    }
}

pub fn integration_catalog_tool_descriptors() -> Vec<ToolDescriptor> {
    [
        IntegrationCatalogTool::ListIntegrations,
        IntegrationCatalogTool::DescribeIntegration,
        IntegrationCatalogTool::ListPrimitives,
        IntegrationCatalogTool::DescribePrimitive,
    ]
    .into_iter()
    .map(IntegrationCatalogTool::descriptor)
    .collect()
}

pub fn primitive_family_descriptors() -> Vec<PrimitiveFamilyDescriptor> {
    all_primitive_families()
        .iter()
        .copied()
        .map(describe_primitive_family)
        .collect()
}

pub fn ecosystem_survey_sources() -> Vec<EcosystemSurveySource> {
    vec![
        ecosystem_source(
            EcosystemSurveyPlatform::HomeAssistant,
            "Home Assistant",
            "https://www.home-assistant.io/integrations/",
            "public integration index and Core manifests",
            "Broad integration taxonomy, IoT classes, integration types, virtual aliases, and source references.",
            &[
                PrimitiveFamily::DiscoveryIndex,
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::WebSocket,
                PrimitiveFamily::Mqtt,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::Webhook,
                PrimitiveFamily::CalculatedState,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::Hubitat,
            "Hubitat",
            "https://docs2.hubitat.com/en/devices/list-of-compatible-devices",
            "compatible-device and driver documentation",
            "Local hub, Zigbee, Z-Wave, LAN, cloud app, Groovy driver, and Matter-over-Thread lessons.",
            &[
                PrimitiveFamily::Radio802154,
                PrimitiveFamily::ZWaveSerialApi,
                PrimitiveFamily::MatterCommissioning,
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::Supervision,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::HomeyPro,
            "Homey Pro",
            "https://homey.app/en-us/apps/homey-pro/",
            "app store and protocol-rich hub model",
            "App-style integration packaging, guided pairing, local radios, cloud apps, flows, energy, media, and security categories.",
            &[
                PrimitiveFamily::BluetoothLowEnergy,
                PrimitiveFamily::Radio802154,
                PrimitiveFamily::ZWaveSerialApi,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::LocalPairing,
                PrimitiveFamily::Supervision,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::SmartThings,
            "SmartThings",
            "https://support.smartthings.com/hc/en-us/articles/360052390111-Devices-in-SmartThings",
            "device, hub, Edge driver, and Matter documentation",
            "Hub-mediated devices, local Edge drivers, partner devices, Matter, Zigbee, Z-Wave, LAN, and cloud linked services.",
            &[
                PrimitiveFamily::MatterCommissioning,
                PrimitiveFamily::Radio802154,
                PrimitiveFamily::ZWaveSerialApi,
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::CapabilityPolicy,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::OpenHab,
            "openHAB",
            "https://www.openhab.org/addons/",
            "add-ons and bindings reference",
            "Protocol-first bindings, automation add-ons, persistence, transformations, voice, UI, and service adapters.",
            &[
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::Mqtt,
                PrimitiveFamily::SerialController,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::CalculatedState,
                PrimitiveFamily::Supervision,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::Homebridge,
            "Homebridge",
            "https://homebridge.io/plugins",
            "plugin directory and verified plugin program",
            "HomeKit bridge semantics, plugin quality gates, and Node sidecar packaging lessons.",
            &[
                PrimitiveFamily::HomeKitPairing,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::CapabilityPolicy,
                PrimitiveFamily::Supervision,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::IoBroker,
            "ioBroker",
            "https://download.iobroker.net/sources-dist.json",
            "adapter catalog JSON",
            "Large admin-installable adapter ecosystem and source-catalog metadata shape.",
            &[
                PrimitiveFamily::DiscoveryIndex,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::Mqtt,
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::CalculatedState,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::Domoticz,
            "Domoticz",
            "https://www.domoticz.com/wiki/Hardware",
            "hardware and protocol wiki",
            "Hardware-gateway framing across 433/868/915 MHz, Z-Wave, Zigbee, cameras, Modbus, MQTT, and serial devices.",
            &[
                PrimitiveFamily::SerialController,
                PrimitiveFamily::Radio802154,
                PrimitiveFamily::ZWaveSerialApi,
                PrimitiveFamily::Mqtt,
                PrimitiveFamily::CameraMedia,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::Jeedom,
            "Jeedom",
            "https://market.jeedom.com/",
            "plugin market and smart-home solution pages",
            "Local-first plugin marketplace, multi-protocol setup, and commercial/community plugin separation.",
            &[
                PrimitiveFamily::DiscoveryIndex,
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::SerialController,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::Supervision,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::HomeSeer,
            "HomeSeer",
            "https://shop.homeseer.com/pages/software-plugins",
            "plugin documentation and hub/software pages",
            "Commercial local hub/software plugin ecosystem with Zigbee, Z-Wave, Matter, Hue, ONVIF, and cloud examples.",
            &[
                PrimitiveFamily::ZWaveSerialApi,
                PrimitiveFamily::Radio802154,
                PrimitiveFamily::MatterCommissioning,
                PrimitiveFamily::LocalHttp,
                PrimitiveFamily::CameraMedia,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::AppleHome,
            "Apple Home",
            "https://developer.apple.com/apple-home/",
            "Apple Home developer page",
            "HomeKit, Matter, ThreadNetwork, EnergyKit, MFi, Works with Apple Home, and certification boundaries.",
            &[
                PrimitiveFamily::HomeKitPairing,
                PrimitiveFamily::MatterCommissioning,
                PrimitiveFamily::Radio802154,
                PrimitiveFamily::CertificatePairing,
                PrimitiveFamily::EnergyTelemetry,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::GoogleHome,
            "Google Home",
            "https://developers.home.google.com/matter/supported-devices",
            "Matter supported device types",
            "Matter controller surface and device-type-specific support that must map through capability policy.",
            &[
                PrimitiveFamily::MatterCommissioning,
                PrimitiveFamily::CertificatePairing,
                PrimitiveFamily::CapabilityPolicy,
                PrimitiveFamily::CloudApi,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::AmazonAlexa,
            "Amazon Alexa",
            "https://developer.amazon.com/en-US/docs/alexa/smarthome/supported-matter-device-categories.html",
            "Matter device categories and smart-home API docs",
            "Voice/cloud ecosystem plus Matter controller surface with explicit category and security restrictions.",
            &[
                PrimitiveFamily::MatterCommissioning,
                PrimitiveFamily::CloudApi,
                PrimitiveFamily::CapabilityPolicy,
                PrimitiveFamily::Webhook,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::ZWaveAlliance,
            "Z-Wave Alliance",
            "https://z-wavealliance.org/development-resources-overview/z-wave-command-classes/",
            "command-class development resources",
            "Command classes are the application primitive for Z-Wave capability mapping, reports, interviews, and routing.",
            &[
                PrimitiveFamily::ZWaveSerialApi,
                PrimitiveFamily::SerialController,
                PrimitiveFamily::RadioNetworkKey,
                PrimitiveFamily::CommandMapping,
            ],
        ),
        ecosystem_source(
            EcosystemSurveyPlatform::ThreadGroup,
            "Thread Group",
            "https://threadgroup.org/Newsroom/Blog/thread-with-matter-better-connections-smarter-homes",
            "Thread with Matter technical framing",
            "Thread is the IP mesh/network primitive while Matter is the application layer; border-router health is runtime state.",
            &[
                PrimitiveFamily::Radio802154,
                PrimitiveFamily::MatterCommissioning,
                PrimitiveFamily::RadioNetworkKey,
                PrimitiveFamily::Supervision,
            ],
        ),
    ]
}

pub fn survey_source_for_platform(
    sources: &[EcosystemSurveySource],
    platform: EcosystemSurveyPlatform,
) -> Option<&EcosystemSurveySource> {
    sources.iter().find(|source| source.platform == platform)
}

pub fn survey_sources_requiring_primitive(
    sources: &[EcosystemSurveySource],
    primitive: PrimitiveFamily,
) -> Vec<&EcosystemSurveySource> {
    sources
        .iter()
        .filter(|source| source.requires_primitive(primitive))
        .collect()
}

pub fn describe_primitive_family(primitive: PrimitiveFamily) -> PrimitiveFamilyDescriptor {
    let (display_name, summary) = match primitive {
        PrimitiveFamily::NormalizedModel => (
            "Normalized Model",
            "Bridge, device, entity, capability, event, command, health, and audit records.",
        ),
        PrimitiveFamily::DiscoveryIndex => (
            "Discovery Index",
            "Reusable observations that connect discovery sources to catalog entries.",
        ),
        PrimitiveFamily::Mdns => (
            "mDNS",
            "Local DNS-SD discovery for LAN devices and bridges.",
        ),
        PrimitiveFamily::Ssdp => (
            "SSDP",
            "UPnP-style discovery for media and legacy LAN devices.",
        ),
        PrimitiveFamily::Dhcp => (
            "DHCP",
            "Network-observed address hints for LAN device candidates.",
        ),
        PrimitiveFamily::LocalHttp => (
            "Local HTTP",
            "HTTP/HTTPS request primitives for local APIs.",
        ),
        PrimitiveFamily::WebSocket => ("WebSocket", "Bidirectional local or cloud event streams."),
        PrimitiveFamily::ServerSentEvents => (
            "Server-Sent Events",
            "One-way event streams such as Hue CLIP v2 SSE.",
        ),
        PrimitiveFamily::Mqtt => (
            "MQTT",
            "Broker topics, retained state, and command publications.",
        ),
        PrimitiveFamily::BluetoothLowEnergy => (
            "Bluetooth Low Energy",
            "BLE advertisements, GATT reads, and host adapter health.",
        ),
        PrimitiveFamily::Usb => ("USB", "USB device enumeration for radios and controllers."),
        PrimitiveFamily::SerialController => (
            "Serial Controller",
            "Serial transport leases for radio and fieldbus controllers.",
        ),
        PrimitiveFamily::Radio802154 => (
            "802.15.4 Radio",
            "Low-power radio substrate used by Zigbee and Thread-class stacks.",
        ),
        PrimitiveFamily::ZWaveSerialApi => (
            "Z-Wave Serial API",
            "Z-Wave controller serial API framing and lifecycle.",
        ),
        PrimitiveFamily::MatterCommissioning => (
            "Matter Commissioning",
            "Matter onboarding, fabrics, and commissioning metadata.",
        ),
        PrimitiveFamily::HomeKitPairing => (
            "HomeKit Pairing",
            "HAP pairing and accessory model projection.",
        ),
        PrimitiveFamily::CloudApi => ("Cloud API", "OAuth/API-key cloud service calls and quotas."),
        PrimitiveFamily::Webhook => ("Webhook", "Inbound callback registration and delivery."),
        PrimitiveFamily::OAuth2 => ("OAuth2", "Cloud account authorization and token refresh."),
        PrimitiveFamily::LocalPairing => {
            ("Local Pairing", "Physical-presence or local-code setup.")
        }
        PrimitiveFamily::LocalToken => ("Local Token", "Local API token storage and leasing."),
        PrimitiveFamily::CertificatePairing => (
            "Certificate Pairing",
            "Certificate or mTLS-style local trust setup.",
        ),
        PrimitiveFamily::RadioNetworkKey => (
            "Radio Network Key",
            "Mesh/radio network secrets and rotation.",
        ),
        PrimitiveFamily::MqttCredentials => (
            "MQTT Credentials",
            "Broker credentials and client identity leases.",
        ),
        PrimitiveFamily::CameraMedia => (
            "Camera Media",
            "Privacy-sensitive snapshots, streams, and camera events.",
        ),
        PrimitiveFamily::EnergyTelemetry => (
            "Energy Telemetry",
            "Energy, climate, utility, and production measurements.",
        ),
        PrimitiveFamily::CalculatedState => (
            "Calculated State",
            "Internal derived entities and dependency-driven state.",
        ),
        PrimitiveFamily::CommandMapping => (
            "Command Mapping",
            "Idempotent mapping from canonical commands to native effects.",
        ),
        PrimitiveFamily::CapabilityPolicy => (
            "Capability Policy",
            "Capability, privilege, and approval rules for tool execution.",
        ),
        PrimitiveFamily::VaultLease => ("Vault Lease", "Time-bounded secret access for workers."),
        PrimitiveFamily::Supervision => (
            "Supervision",
            "Worker health, restart, backoff, heartbeat, and stale-state policy.",
        ),
        PrimitiveFamily::TestSimulator => (
            "Test Simulator",
            "Fake bridges, brokers, radios, streams, and cloud APIs.",
        ),
    };

    PrimitiveFamilyDescriptor {
        primitive,
        display_name,
        summary,
    }
}

pub fn all_primitive_families() -> &'static [PrimitiveFamily] {
    &[
        PrimitiveFamily::NormalizedModel,
        PrimitiveFamily::DiscoveryIndex,
        PrimitiveFamily::Mdns,
        PrimitiveFamily::Ssdp,
        PrimitiveFamily::Dhcp,
        PrimitiveFamily::LocalHttp,
        PrimitiveFamily::WebSocket,
        PrimitiveFamily::ServerSentEvents,
        PrimitiveFamily::Mqtt,
        PrimitiveFamily::BluetoothLowEnergy,
        PrimitiveFamily::Usb,
        PrimitiveFamily::SerialController,
        PrimitiveFamily::Radio802154,
        PrimitiveFamily::ZWaveSerialApi,
        PrimitiveFamily::MatterCommissioning,
        PrimitiveFamily::HomeKitPairing,
        PrimitiveFamily::CloudApi,
        PrimitiveFamily::Webhook,
        PrimitiveFamily::OAuth2,
        PrimitiveFamily::LocalPairing,
        PrimitiveFamily::LocalToken,
        PrimitiveFamily::CertificatePairing,
        PrimitiveFamily::RadioNetworkKey,
        PrimitiveFamily::MqttCredentials,
        PrimitiveFamily::CameraMedia,
        PrimitiveFamily::EnergyTelemetry,
        PrimitiveFamily::CalculatedState,
        PrimitiveFamily::CommandMapping,
        PrimitiveFamily::CapabilityPolicy,
        PrimitiveFamily::VaultLease,
        PrimitiveFamily::Supervision,
        PrimitiveFamily::TestSimulator,
    ]
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

pub fn entries_with_policy_surface(
    catalog: &[IntegrationCatalogEntry],
    surface: IntegrationPolicySurface,
) -> Vec<&IntegrationCatalogEntry> {
    catalog
        .iter()
        .filter(|entry| entry.has_policy_surface(surface))
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

pub fn policy_surfaces_for_entry(entry: &IntegrationCatalogEntry) -> Vec<IntegrationPolicySurface> {
    let mut surfaces = Vec::new();

    if entry.required_capabilities.iter().any(is_local_actuator) {
        surfaces.push(IntegrationPolicySurface::LocalActuation);
    }
    if entry
        .required_capabilities
        .contains(&CapabilityId::trusted("smart_home.command.lock"))
        || entry.target_entity_kinds.contains(&EntityKind::Lock)
    {
        surfaces.push(IntegrationPolicySurface::EntryAccess);
    }
    if entry
        .required_capabilities
        .contains(&CapabilityId::trusted("smart_home.command.climate"))
        || entry.target_entity_kinds.contains(&EntityKind::Thermostat)
    {
        surfaces.push(IntegrationPolicySurface::ClimateControl);
    }
    if entry.category == IntegrationCategory::CameraMedia
        || entry
            .required_primitives
            .contains(&PrimitiveFamily::CameraMedia)
        || entry
            .required_capabilities
            .contains(&CapabilityId::trusted("smart_home.command.camera"))
    {
        surfaces.push(IntegrationPolicySurface::CameraMedia);
    }
    if entry.category == IntegrationCategory::EnergyClimate
        || entry
            .required_primitives
            .contains(&PrimitiveFamily::EnergyTelemetry)
        || entry
            .required_capabilities
            .contains(&CapabilityId::trusted("smart_home.command.energy"))
    {
        surfaces.push(IntegrationPolicySurface::EnergyManagement);
    }
    if entry.auth_modes.iter().any(requires_secret_lease) {
        surfaces.push(IntegrationPolicySurface::CredentialLease);
    }
    if entry.connectivity.requires_cloud()
        || entry
            .discovery_mechanisms
            .contains(&DiscoveryMechanism::CloudAccount)
    {
        surfaces.push(IntegrationPolicySurface::CredentialedCloud);
    }
    if entry
        .required_capabilities
        .contains(&CapabilityId::trusted("smart_home.manage_network"))
        || entry.auth_modes.contains(&AuthMode::RadioNetworkKey)
    {
        surfaces.push(IntegrationPolicySurface::RadioNetworkManagement);
    }
    if entry
        .required_capabilities
        .contains(&CapabilityId::trusted("smart_home.diagnostics"))
        || entry
            .target_entity_kinds
            .contains(&EntityKind::NetworkDiagnostic)
    {
        surfaces.push(IntegrationPolicySurface::NetworkInfrastructure);
    }

    dedupe_policy_surfaces(surfaces)
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

fn is_local_actuator(capability_id: &CapabilityId) -> bool {
    matches!(
        capability_id.as_str(),
        "smart_home.command.light" | "smart_home.command.switch" | "smart_home.command.media"
    )
}

fn requires_secret_lease(auth_mode: &AuthMode) -> bool {
    !matches!(auth_mode, AuthMode::None)
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

fn dedupe_policy_surfaces(
    surfaces: Vec<IntegrationPolicySurface>,
) -> Vec<IntegrationPolicySurface> {
    let mut result = Vec::new();
    for surface in surfaces {
        if !result.contains(&surface) {
            result.push(surface);
        }
    }
    result
}

fn capability(value: &'static str) -> CapabilityId {
    CapabilityId::trusted(value)
}

fn ecosystem_source(
    platform: EcosystemSurveyPlatform,
    display_name: &'static str,
    source_url: &'static str,
    source_surface: &'static str,
    contributes: &'static str,
    primitive_hints: &[PrimitiveFamily],
) -> EcosystemSurveySource {
    EcosystemSurveySource {
        platform,
        display_name,
        source_url,
        source_surface,
        contributes,
        primitive_hints: primitive_hints.to_vec(),
    }
}

fn read_catalog_tool(tool_id: &'static str) -> ToolDescriptor {
    ToolDescriptor {
        tool_id,
        side_effects: ToolSideEffects::Read,
        required_capabilities: vec![CapabilityId::trusted("smart_home.read")],
        required_tier: PrivilegeTier::ReadOnly,
    }
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
    fn catalog_tools_are_read_only_d18d_descriptors() {
        let descriptors = integration_catalog_tool_descriptors();

        assert_eq!(descriptors.len(), 4);
        assert!(descriptors
            .iter()
            .any(|descriptor| descriptor.tool_id == "smart_home.list_integrations"));
        assert!(descriptors
            .iter()
            .all(|descriptor| descriptor.side_effects == ToolSideEffects::Read));
        assert!(descriptors
            .iter()
            .all(|descriptor| descriptor.required_tier == PrivilegeTier::ReadOnly));
        assert!(descriptors.iter().all(|descriptor| descriptor
            .required_capabilities
            .contains(&CapabilityId::trusted("smart_home.read"))));
    }

    #[test]
    fn primitive_family_descriptors_cover_every_primitive() {
        let descriptors = primitive_family_descriptors();

        assert_eq!(descriptors.len(), all_primitive_families().len());
        assert!(descriptors
            .iter()
            .any(|descriptor| descriptor.primitive == PrimitiveFamily::Mqtt
                && descriptor.display_name == "MQTT"));
        assert!(descriptors.iter().any(|descriptor| descriptor.primitive
            == PrimitiveFamily::Supervision
            && descriptor.summary.contains("restart")));
    }

    #[test]
    fn ecosystem_survey_sources_cover_reference_platforms() {
        let sources = ecosystem_survey_sources();

        assert_eq!(sources.len(), 15);
        assert_eq!(
            EcosystemSurveyPlatform::HomeAssistant.as_str(),
            "home_assistant"
        );
        assert!(
            survey_source_for_platform(&sources, EcosystemSurveyPlatform::HomeAssistant)
                .unwrap()
                .source_url
                .contains("home-assistant.io/integrations")
        );
        assert!(survey_source_for_platform(&sources, EcosystemSurveyPlatform::Hubitat).is_some());
        assert!(survey_source_for_platform(&sources, EcosystemSurveyPlatform::HomeyPro).is_some());
        assert!(survey_source_for_platform(&sources, EcosystemSurveyPlatform::OpenHab).is_some());
        assert!(sources
            .iter()
            .all(|source| !source.primitive_hints.is_empty()));
    }

    #[test]
    fn ecosystem_survey_sources_group_protocol_primitives() {
        let sources = ecosystem_survey_sources();
        let matter_sources =
            survey_sources_requiring_primitive(&sources, PrimitiveFamily::MatterCommissioning);
        let zwave_sources =
            survey_sources_requiring_primitive(&sources, PrimitiveFamily::ZWaveSerialApi);
        let mqtt_sources = survey_sources_requiring_primitive(&sources, PrimitiveFamily::Mqtt);

        assert!(matter_sources
            .iter()
            .any(|source| source.platform == EcosystemSurveyPlatform::AppleHome));
        assert!(matter_sources
            .iter()
            .any(|source| source.platform == EcosystemSurveyPlatform::GoogleHome));
        assert!(zwave_sources
            .iter()
            .any(|source| source.platform == EcosystemSurveyPlatform::ZWaveAlliance));
        assert!(mqtt_sources
            .iter()
            .any(|source| source.platform == EcosystemSurveyPlatform::HomeAssistant));
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

    #[test]
    fn policy_surfaces_capture_privacy_and_entry_access() {
        let catalog = first_party_catalog();
        let onvif = find_entry(&catalog, &IntegrationId::trusted("onvif")).unwrap();
        let zwave = find_entry(&catalog, &IntegrationId::trusted("zwave")).unwrap();

        assert!(onvif.has_policy_surface(IntegrationPolicySurface::CameraMedia));
        assert_eq!(onvif.highest_policy_tier(), PrivilegeTier::HumanApproval);

        assert!(zwave.has_policy_surface(IntegrationPolicySurface::EntryAccess));
        assert!(zwave.has_policy_surface(IntegrationPolicySurface::RadioNetworkManagement));
        assert_eq!(zwave.highest_policy_tier(), PrivilegeTier::HighRisk);
    }

    #[test]
    fn policy_surfaces_expose_cloud_and_credential_boundaries() {
        let catalog = first_party_catalog();
        let ring = find_entry(&catalog, &IntegrationId::trusted("ring")).unwrap();
        let hue = find_entry(&catalog, &IntegrationId::trusted("hue")).unwrap();

        assert!(ring.has_policy_surface(IntegrationPolicySurface::CredentialedCloud));
        assert!(ring.has_policy_surface(IntegrationPolicySurface::CredentialLease));
        assert!(hue.has_policy_surface(IntegrationPolicySurface::CredentialLease));
        assert!(!hue.has_policy_surface(IntegrationPolicySurface::CredentialedCloud));
    }

    #[test]
    fn policy_surface_queries_group_d21_review_targets() {
        let catalog = first_party_catalog();
        let cameras = entries_with_policy_surface(&catalog, IntegrationPolicySurface::CameraMedia);
        let local_actuators =
            entries_with_policy_surface(&catalog, IntegrationPolicySurface::LocalActuation);

        assert!(cameras
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("reolink")));
        assert!(local_actuators
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("hue")));
        assert!(local_actuators
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("mqtt")));
    }
}
