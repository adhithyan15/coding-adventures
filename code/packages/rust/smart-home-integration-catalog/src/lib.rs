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
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
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
pub struct EcosystemPrimitiveCoverage {
    pub primitive: PrimitiveFamily,
    pub platforms: Vec<EcosystemSurveyPlatform>,
    pub source_count: usize,
}

impl EcosystemPrimitiveCoverage {
    pub fn platform_count(&self) -> usize {
        self.platforms.len()
    }

    pub fn is_gap(&self) -> bool {
        self.source_count == 0
    }

    pub fn covers_platform(&self, platform: EcosystemSurveyPlatform) -> bool {
        self.platforms.contains(&platform)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationCatalogSort {
    PriorityThenName,
    Name,
    CategoryThenPriority,
    StatusThenPriority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationCatalogQuery {
    pub categories: Vec<IntegrationCategory>,
    pub connectivity: Vec<ConnectivityClass>,
    pub implementation_statuses: Vec<ImplementationStatus>,
    pub required_primitives: Vec<PrimitiveFamily>,
    pub required_capabilities: Vec<CapabilityId>,
    pub policy_surfaces: Vec<IntegrationPolicySurface>,
    pub discovery_mechanisms: Vec<DiscoveryMechanism>,
    pub auth_modes: Vec<AuthMode>,
    pub protocol_families: Vec<ProtocolFamily>,
    pub priority_at_or_before: Option<u8>,
    pub include_virtual_aliases: bool,
    pub local_only: Option<bool>,
    pub cloud_required: Option<bool>,
    pub sort: IntegrationCatalogSort,
    pub limit: Option<usize>,
}

impl Default for IntegrationCatalogQuery {
    fn default() -> Self {
        Self {
            categories: Vec::new(),
            connectivity: Vec::new(),
            implementation_statuses: Vec::new(),
            required_primitives: Vec::new(),
            required_capabilities: Vec::new(),
            policy_surfaces: Vec::new(),
            discovery_mechanisms: Vec::new(),
            auth_modes: Vec::new(),
            protocol_families: Vec::new(),
            priority_at_or_before: None,
            include_virtual_aliases: true,
            local_only: None,
            cloud_required: None,
            sort: IntegrationCatalogSort::PriorityThenName,
            limit: None,
        }
    }
}

impl IntegrationCatalogQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_category(mut self, category: IntegrationCategory) -> Self {
        self.categories.push(category);
        self
    }

    pub fn with_connectivity(mut self, connectivity: ConnectivityClass) -> Self {
        self.connectivity.push(connectivity);
        self
    }

    pub fn with_status(mut self, status: ImplementationStatus) -> Self {
        self.implementation_statuses.push(status);
        self
    }

    pub fn requiring_primitive(mut self, primitive: PrimitiveFamily) -> Self {
        self.required_primitives.push(primitive);
        self
    }

    pub fn requiring_capability(mut self, capability_id: CapabilityId) -> Self {
        self.required_capabilities.push(capability_id);
        self
    }

    pub fn with_policy_surface(mut self, surface: IntegrationPolicySurface) -> Self {
        self.policy_surfaces.push(surface);
        self
    }

    pub fn with_discovery_mechanism(mut self, mechanism: DiscoveryMechanism) -> Self {
        self.discovery_mechanisms.push(mechanism);
        self
    }

    pub fn with_auth_mode(mut self, mode: AuthMode) -> Self {
        self.auth_modes.push(mode);
        self
    }

    pub fn with_protocol_family(mut self, protocol: ProtocolFamily) -> Self {
        self.protocol_families.push(protocol);
        self
    }

    pub fn at_or_before_priority(mut self, priority: u8) -> Self {
        self.priority_at_or_before = Some(priority);
        self
    }

    pub fn include_virtual_aliases(mut self, include: bool) -> Self {
        self.include_virtual_aliases = include;
        self
    }

    pub fn local_only(mut self, local_only: bool) -> Self {
        self.local_only = Some(local_only);
        self
    }

    pub fn cloud_required(mut self, cloud_required: bool) -> Self {
        self.cloud_required = Some(cloud_required);
        self
    }

    pub fn sorted_by(mut self, sort: IntegrationCatalogSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn matches_entry(&self, entry: &IntegrationCatalogEntry) -> bool {
        if !self.include_virtual_aliases && entry.is_virtual() {
            return false;
        }
        if let Some(priority) = self.priority_at_or_before {
            if entry.priority > priority {
                return false;
            }
        }
        if let Some(local_only) = self.local_only {
            if entry_local_only(entry) != local_only {
                return false;
            }
        }
        if let Some(cloud_required) = self.cloud_required {
            if entry_cloud_required(entry) != cloud_required {
                return false;
            }
        }
        if !matches_any(&self.categories, &entry.category) {
            return false;
        }
        if !matches_any(&self.connectivity, &entry.connectivity) {
            return false;
        }
        if !matches_any(&self.implementation_statuses, &entry.implementation_status) {
            return false;
        }
        if !self
            .required_primitives
            .iter()
            .all(|primitive| entry.requires_primitive(*primitive))
        {
            return false;
        }
        if !self
            .required_capabilities
            .iter()
            .all(|capability_id| entry.supports_capability(capability_id))
        {
            return false;
        }
        if !self
            .policy_surfaces
            .iter()
            .all(|surface| entry.has_policy_surface(*surface))
        {
            return false;
        }
        if !self
            .discovery_mechanisms
            .iter()
            .all(|mechanism| entry.uses_discovery(*mechanism))
        {
            return false;
        }
        if !self
            .auth_modes
            .iter()
            .all(|mode| entry.auth_modes.contains(mode))
        {
            return false;
        }
        if !self.protocol_families.is_empty()
            && !self.protocol_families.iter().any(|protocol| {
                entry
                    .supported_protocols
                    .iter()
                    .any(|candidate| candidate == protocol)
                    || entry
                        .virtual_iot_standards
                        .iter()
                        .any(|candidate| candidate == protocol)
            })
        {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveBacklogItem {
    pub primitive: PrimitiveFamily,
    pub highest_priority: u8,
    pub entry_count: usize,
    pub integration_ids: Vec<IntegrationId>,
}

impl PrimitiveBacklogItem {
    pub fn includes_integration(&self, integration_id: &IntegrationId) -> bool {
        self.integration_ids
            .iter()
            .any(|candidate| candidate == integration_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveBacklogCoverageItem {
    pub primitive: PrimitiveFamily,
    pub highest_priority: u8,
    pub entry_count: usize,
    pub integration_ids: Vec<IntegrationId>,
    pub source_count: usize,
    pub platforms: Vec<EcosystemSurveyPlatform>,
}

impl PrimitiveBacklogCoverageItem {
    pub fn platform_count(&self) -> usize {
        self.platforms.len()
    }

    pub fn covers_platform(&self, platform: EcosystemSurveyPlatform) -> bool {
        self.platforms.contains(&platform)
    }

    pub fn includes_integration(&self, integration_id: &IntegrationId) -> bool {
        self.integration_ids
            .iter()
            .any(|candidate| candidate == integration_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveBacklogCoverageSummary {
    pub total_primitives: usize,
    pub total_entries: usize,
    pub unique_integrations: usize,
    pub covered_primitives: usize,
    pub uncovered_primitives: usize,
    pub single_source_primitives: usize,
    pub multi_platform_primitives: usize,
    pub total_source_references: usize,
    pub total_platform_references: usize,
    pub first_uncovered_priority: Option<u8>,
    pub first_single_source_priority: Option<u8>,
    pub broadest_platform_count: usize,
}

impl PrimitiveBacklogCoverageSummary {
    pub fn from_items<'a>(
        items: impl IntoIterator<Item = &'a PrimitiveBacklogCoverageItem>,
    ) -> Self {
        let mut summary = Self {
            total_primitives: 0,
            total_entries: 0,
            unique_integrations: 0,
            covered_primitives: 0,
            uncovered_primitives: 0,
            single_source_primitives: 0,
            multi_platform_primitives: 0,
            total_source_references: 0,
            total_platform_references: 0,
            first_uncovered_priority: None,
            first_single_source_priority: None,
            broadest_platform_count: 0,
        };
        let mut integration_ids = BTreeSet::new();

        for item in items {
            summary.total_primitives += 1;
            summary.total_entries += item.entry_count;
            summary.total_source_references += item.source_count;
            summary.total_platform_references += item.platform_count();
            summary.broadest_platform_count =
                summary.broadest_platform_count.max(item.platform_count());

            if item.source_count == 0 {
                summary.uncovered_primitives += 1;
                summary.first_uncovered_priority = Some(
                    summary
                        .first_uncovered_priority
                        .map_or(item.highest_priority, |priority| {
                            priority.min(item.highest_priority)
                        }),
                );
            } else {
                summary.covered_primitives += 1;
            }

            if item.source_count == 1 {
                summary.single_source_primitives += 1;
                summary.first_single_source_priority = Some(
                    summary
                        .first_single_source_priority
                        .map_or(item.highest_priority, |priority| {
                            priority.min(item.highest_priority)
                        }),
                );
            }
            if item.platform_count() >= 2 {
                summary.multi_platform_primitives += 1;
            }

            for integration_id in &item.integration_ids {
                integration_ids.insert(integration_id.clone());
            }
        }

        summary.unique_integrations = integration_ids.len();
        summary
    }

    pub fn has_uncovered_primitives(&self) -> bool {
        self.uncovered_primitives > 0
    }

    pub fn has_single_source_primitives(&self) -> bool {
        self.single_source_primitives > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcosystemPlatformCoverageItem {
    pub platform: EcosystemSurveyPlatform,
    pub display_name: &'static str,
    pub source_url: &'static str,
    pub source_surface: &'static str,
    pub contributes: &'static str,
    pub primitive_hints: Vec<PrimitiveFamily>,
    pub backlog_primitives: Vec<PrimitiveFamily>,
    pub covered_backlog_primitives: Vec<PrimitiveFamily>,
    pub uncovered_backlog_primitives: Vec<PrimitiveFamily>,
    pub highest_backlog_priority: Option<u8>,
    pub backlog_entry_count: usize,
}

impl EcosystemPlatformCoverageItem {
    pub fn primitive_hint_count(&self) -> usize {
        self.primitive_hints.len()
    }

    pub fn backlog_primitive_count(&self) -> usize {
        self.backlog_primitives.len()
    }

    pub fn covered_backlog_primitive_count(&self) -> usize {
        self.covered_backlog_primitives.len()
    }

    pub fn uncovered_backlog_primitive_count(&self) -> usize {
        self.uncovered_backlog_primitives.len()
    }

    pub fn has_backlog_overlap(&self) -> bool {
        !self.covered_backlog_primitives.is_empty()
    }

    pub fn covers_primitive(&self, primitive: PrimitiveFamily) -> bool {
        self.primitive_hints.contains(&primitive)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcosystemPlatformCoverageSummary {
    pub total_platforms: usize,
    pub total_primitive_hints: usize,
    pub unique_primitive_hints: usize,
    pub backlog_primitive_count: usize,
    pub covered_backlog_primitives: usize,
    pub uncovered_backlog_primitives: usize,
    pub platforms_with_backlog_overlap: usize,
    pub platforms_covering_all_backlog_primitives: usize,
    pub first_covered_backlog_priority: Option<u8>,
}

impl EcosystemPlatformCoverageSummary {
    pub fn from_items<'a>(
        items: impl IntoIterator<Item = &'a EcosystemPlatformCoverageItem>,
    ) -> Self {
        let mut summary = Self {
            total_platforms: 0,
            total_primitive_hints: 0,
            unique_primitive_hints: 0,
            backlog_primitive_count: 0,
            covered_backlog_primitives: 0,
            uncovered_backlog_primitives: 0,
            platforms_with_backlog_overlap: 0,
            platforms_covering_all_backlog_primitives: 0,
            first_covered_backlog_priority: None,
        };
        let mut primitive_hints = BTreeSet::new();
        let mut backlog_primitives = BTreeSet::new();
        let mut covered_backlog_primitives = BTreeSet::new();

        for item in items {
            summary.total_platforms += 1;
            summary.total_primitive_hints += item.primitive_hint_count();
            if item.has_backlog_overlap() {
                summary.platforms_with_backlog_overlap += 1;
            }
            if item.backlog_primitive_count() > 0 && item.uncovered_backlog_primitive_count() == 0 {
                summary.platforms_covering_all_backlog_primitives += 1;
            }
            summary.first_covered_backlog_priority = match (
                summary.first_covered_backlog_priority,
                item.highest_backlog_priority,
            ) {
                (Some(left), Some(right)) => Some(left.min(right)),
                (None, Some(priority)) => Some(priority),
                (priority, None) => priority,
            };

            for primitive in &item.primitive_hints {
                primitive_hints.insert(*primitive);
            }
            for primitive in &item.backlog_primitives {
                backlog_primitives.insert(*primitive);
            }
            for primitive in &item.covered_backlog_primitives {
                covered_backlog_primitives.insert(*primitive);
            }
        }

        summary.unique_primitive_hints = primitive_hints.len();
        summary.backlog_primitive_count = backlog_primitives.len();
        summary.covered_backlog_primitives = covered_backlog_primitives.len();
        summary.uncovered_backlog_primitives = backlog_primitives
            .difference(&covered_backlog_primitives)
            .count();
        summary
    }

    pub fn has_uncovered_backlog_primitives(&self) -> bool {
        self.uncovered_backlog_primitives > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationPolicySurfaceInventoryItem {
    pub surface: IntegrationPolicySurface,
    pub required_tier: PrivilegeTier,
    pub highest_priority: u8,
    pub entry_count: usize,
    pub local_entry_count: usize,
    pub cloud_entry_count: usize,
    pub human_review_entry_count: usize,
    pub integration_ids: Vec<IntegrationId>,
}

impl IntegrationPolicySurfaceInventoryItem {
    pub fn includes_integration(&self, integration_id: &IntegrationId) -> bool {
        self.integration_ids
            .iter()
            .any(|candidate| candidate == integration_id)
    }

    pub fn requires_human_review(&self) -> bool {
        self.required_tier >= PrivilegeTier::HumanApproval
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationPolicySurfaceSummary {
    pub total_surfaces: usize,
    pub total_surface_entries: usize,
    pub unique_integrations: usize,
    pub local_surface_entries: usize,
    pub cloud_surface_entries: usize,
    pub human_review_surface_entries: usize,
    pub read_only_surfaces: usize,
    pub low_risk_surfaces: usize,
    pub human_approval_surfaces: usize,
    pub high_risk_surfaces: usize,
    pub first_review_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

impl IntegrationPolicySurfaceSummary {
    pub fn from_inventory<'a>(
        items: impl IntoIterator<Item = &'a IntegrationPolicySurfaceInventoryItem>,
    ) -> Self {
        let mut summary = Self {
            total_surfaces: 0,
            total_surface_entries: 0,
            unique_integrations: 0,
            local_surface_entries: 0,
            cloud_surface_entries: 0,
            human_review_surface_entries: 0,
            read_only_surfaces: 0,
            low_risk_surfaces: 0,
            human_approval_surfaces: 0,
            high_risk_surfaces: 0,
            first_review_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };
        let mut integration_ids = BTreeSet::new();

        for item in items {
            summary.total_surfaces += 1;
            summary.total_surface_entries += item.entry_count;
            summary.local_surface_entries += item.local_entry_count;
            summary.cloud_surface_entries += item.cloud_entry_count;
            summary.human_review_surface_entries += item.human_review_entry_count;
            match item.required_tier {
                PrivilegeTier::ReadOnly => summary.read_only_surfaces += 1,
                PrivilegeTier::LowRisk => summary.low_risk_surfaces += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_surfaces += 1,
                PrivilegeTier::HighRisk => summary.high_risk_surfaces += 1,
            }
            if item.requires_human_review() {
                summary.first_review_priority = Some(
                    summary
                        .first_review_priority
                        .map_or(item.highest_priority, |priority| {
                            priority.min(item.highest_priority)
                        }),
                );
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(item.required_tier);
            for integration_id in &item.integration_ids {
                integration_ids.insert(integration_id.clone());
            }
        }

        summary.unique_integrations = integration_ids.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_surfaces == 0
    }

    pub fn has_review_work(&self) -> bool {
        self.human_approval_surfaces > 0
            || self.high_risk_surfaces > 0
            || self.human_review_surface_entries > 0
    }

    pub fn has_high_risk_surface(&self) -> bool {
        self.high_risk_surfaces > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationActivationTarget {
    Direct,
    DelegatedIntegration(IntegrationId),
    DelegatedStandards(Vec<ProtocolFamily>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationPlan {
    pub requested_integration_id: IntegrationId,
    pub display_name: String,
    pub activation_target: IntegrationActivationTarget,
    pub implementation_status: ImplementationStatus,
    pub priority: u8,
    pub runtime_kind: RuntimeKind,
    pub required_primitives: Vec<PrimitiveFamily>,
    pub required_capabilities: Vec<CapabilityId>,
    pub auth_modes: Vec<AuthMode>,
    pub discovery_mechanisms: Vec<DiscoveryMechanism>,
    pub depends_on_integrations: Vec<IntegrationId>,
    pub policy_surfaces: Vec<IntegrationPolicySurface>,
    pub highest_policy_tier: PrivilegeTier,
    pub local_only: bool,
    pub cloud_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationPlanSummary {
    pub total_plans: usize,
    pub direct_targets: usize,
    pub delegated_integration_targets: usize,
    pub delegated_standard_targets: usize,
    pub plans_requiring_human_review: usize,
    pub local_only_plans: usize,
    pub cloud_required_plans: usize,
    pub plans_with_dependencies: usize,
    pub plans_with_required_primitives: usize,
    pub plans_with_required_capabilities: usize,
    pub unique_required_primitives: usize,
    pub unique_required_capabilities: usize,
    pub unique_dependencies: usize,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationReadinessReport {
    pub requested_integration_id: IntegrationId,
    pub display_name: String,
    pub activation_target: IntegrationActivationTarget,
    pub priority: u8,
    pub missing_primitives: Vec<PrimitiveFamily>,
    pub missing_capabilities: Vec<CapabilityId>,
    pub missing_dependencies: Vec<IntegrationId>,
    pub requires_human_review: bool,
    pub highest_policy_tier: PrivilegeTier,
    pub local_only: bool,
    pub cloud_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationCandidateRecommendation {
    ReadyToActivate,
    NeedsHumanReview,
    BlockedOnPrerequisites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationHealthStatus {
    Ready,
    NeedsReview,
    Blocked,
    Empty,
}

impl IntegrationActivationHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NeedsReview => "needs_review",
            Self::Blocked => "blocked",
            Self::Empty => "empty",
        }
    }

    pub fn requires_attention(self) -> bool {
        matches!(self, Self::NeedsReview | Self::Blocked)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationBriefingItemKind {
    Blocker,
    Review,
    Approval,
    Activation,
    Risk,
    Dependency,
}

impl IntegrationActivationBriefingItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocker => "blocker",
            Self::Review => "review",
            Self::Approval => "approval",
            Self::Activation => "activation",
            Self::Risk => "risk",
            Self::Dependency => "dependency",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationDecisionStatus {
    ReadyToApprove,
    BlockedOnPrerequisites,
}

impl IntegrationActivationDecisionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyToApprove => "ready_to_approve",
            Self::BlockedOnPrerequisites => "blocked_on_prerequisites",
        }
    }

    pub fn requires_attention(self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationEvidenceKind {
    ApprovalDecision,
    PolicyReview,
    PrimitiveBlocker,
    CapabilityBlocker,
    DependencyBlocker,
    PolicyRisk,
    DependencyEdge,
}

impl IntegrationActivationEvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalDecision => "approval_decision",
            Self::PolicyReview => "policy_review",
            Self::PrimitiveBlocker => "primitive_blocker",
            Self::CapabilityBlocker => "capability_blocker",
            Self::DependencyBlocker => "dependency_blocker",
            Self::PolicyRisk => "policy_risk",
            Self::DependencyEdge => "dependency_edge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationEvidenceStatus {
    SupportsApproval,
    RequiresReview,
    BlocksApproval,
}

impl IntegrationActivationEvidenceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SupportsApproval => "supports_approval",
            Self::RequiresReview => "requires_review",
            Self::BlocksApproval => "blocks_approval",
        }
    }

    pub fn requires_attention(self) -> bool {
        !matches!(self, Self::SupportsApproval)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationActionKind {
    ActivateIntegration,
    ReviewPolicy,
    ProvidePrimitive,
    GrantCapability,
    EnableDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationConstraintKind {
    Primitive,
    Capability,
    Dependency,
    PolicyReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationActivationRiskKind {
    PolicyTier,
    PolicySurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationCandidate {
    pub readiness_report: IntegrationReadinessReport,
    pub recommendation: IntegrationActivationCandidateRecommendation,
    pub blocker_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationCandidateSummary {
    pub total_candidates: usize,
    pub ready_to_activate_candidates: usize,
    pub needs_human_review_candidates: usize,
    pub blocked_candidates: usize,
    pub activation_ready_candidates: usize,
    pub candidates_requiring_human_review: usize,
    pub candidates_missing_primitives: usize,
    pub candidates_missing_capabilities: usize,
    pub candidates_missing_dependencies: usize,
    pub direct_targets: usize,
    pub delegated_integration_targets: usize,
    pub delegated_standard_targets: usize,
    pub local_only_candidates: usize,
    pub cloud_required_candidates: usize,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationAction {
    pub kind: IntegrationActivationActionKind,
    pub requested_integration_id: IntegrationId,
    pub display_name: String,
    pub priority: u8,
    pub recommendation: IntegrationActivationCandidateRecommendation,
    pub primitive: Option<PrimitiveFamily>,
    pub capability_id: Option<CapabilityId>,
    pub dependency_integration_id: Option<IntegrationId>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationActionSummary {
    pub total_actions: usize,
    pub activate_integration_actions: usize,
    pub review_policy_actions: usize,
    pub provide_primitive_actions: usize,
    pub grant_capability_actions: usize,
    pub enable_dependency_actions: usize,
    pub actionable_integration_count: usize,
    pub blocked_integration_count: usize,
    pub unique_integrations: usize,
    pub first_action_priority: Option<u8>,
    pub first_activation_priority: Option<u8>,
    pub first_blocker_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationConstraint {
    pub kind: IntegrationActivationConstraintKind,
    pub constraint_id: String,
    pub display_name: String,
    pub highest_priority: u8,
    pub affected_integration_ids: Vec<IntegrationId>,
    pub blocks_activation: bool,
    pub requires_human_review: bool,
    pub highest_policy_tier: PrivilegeTier,
    pub policy_surfaces: Vec<IntegrationPolicySurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationConstraintSummary {
    pub total_constraints: usize,
    pub blocking_constraints: usize,
    pub review_constraints: usize,
    pub primitive_constraints: usize,
    pub capability_constraints: usize,
    pub dependency_constraints: usize,
    pub policy_review_constraints: usize,
    pub affected_integrations: usize,
    pub first_blocking_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationRiskItem {
    pub kind: IntegrationActivationRiskKind,
    pub risk_id: String,
    pub display_name: String,
    pub required_tier: PrivilegeTier,
    pub policy_surface: Option<IntegrationPolicySurface>,
    pub highest_priority: u8,
    pub integration_ids: Vec<IntegrationId>,
    pub activation_ready_integration_ids: Vec<IntegrationId>,
    pub ready_to_activate_integration_ids: Vec<IntegrationId>,
    pub review_integration_ids: Vec<IntegrationId>,
    pub blocked_integration_ids: Vec<IntegrationId>,
    pub local_only_integration_ids: Vec<IntegrationId>,
    pub cloud_required_integration_ids: Vec<IntegrationId>,
    pub candidate_summary: IntegrationActivationCandidateSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationRiskSummary {
    pub total_risks: usize,
    pub policy_tier_risks: usize,
    pub policy_surface_risks: usize,
    pub total_risk_entries: usize,
    pub unique_integrations: usize,
    pub activation_ready_integrations: usize,
    pub ready_to_activate_integrations: usize,
    pub review_integrations: usize,
    pub blocked_integrations: usize,
    pub local_only_integrations: usize,
    pub cloud_required_integrations: usize,
    pub read_only_risks: usize,
    pub low_risk_risks: usize,
    pub human_approval_risks: usize,
    pub high_risk_risks: usize,
    pub first_ready_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationReviewItem {
    pub requested_integration_id: IntegrationId,
    pub display_name: String,
    pub priority: u8,
    pub activation_target: IntegrationActivationTarget,
    pub recommendation: IntegrationActivationCandidateRecommendation,
    pub blocker_count: usize,
    pub missing_primitives: Vec<PrimitiveFamily>,
    pub missing_capabilities: Vec<CapabilityId>,
    pub missing_dependencies: Vec<IntegrationId>,
    pub policy_surfaces: Vec<IntegrationPolicySurface>,
    pub required_tier: PrivilegeTier,
    pub local_only: bool,
    pub cloud_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationReviewSummary {
    pub total_reviews: usize,
    pub review_ready_integrations: usize,
    pub blocked_review_integrations: usize,
    pub reviews_missing_primitives: usize,
    pub reviews_missing_capabilities: usize,
    pub reviews_missing_dependencies: usize,
    pub direct_targets: usize,
    pub delegated_integration_targets: usize,
    pub delegated_standard_targets: usize,
    pub local_only_reviews: usize,
    pub cloud_required_reviews: usize,
    pub reviews_with_policy_surfaces: usize,
    pub reviews_without_policy_surfaces: usize,
    pub unique_policy_surfaces: usize,
    pub total_blockers: usize,
    pub read_only_reviews: usize,
    pub low_risk_reviews: usize,
    pub human_approval_reviews: usize,
    pub high_risk_reviews: usize,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationApprovalPacket {
    pub review: IntegrationActivationReviewItem,
    pub actions: Vec<IntegrationActivationAction>,
    pub action_summary: IntegrationActivationActionSummary,
    pub constraints: Vec<IntegrationActivationConstraint>,
    pub constraint_summary: IntegrationActivationConstraintSummary,
    pub risks: Vec<IntegrationActivationRiskItem>,
    pub risk_summary: IntegrationActivationRiskSummary,
    pub dependency_graph: IntegrationActivationDependencyGraph,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationApprovalSummary {
    pub total_packets: usize,
    pub approval_ready_packets: usize,
    pub blocked_packets: usize,
    pub local_only_packets: usize,
    pub cloud_required_packets: usize,
    pub packets_with_policy_surfaces: usize,
    pub packets_without_policy_surfaces: usize,
    pub unique_policy_surfaces: usize,
    pub total_actions: usize,
    pub activate_integration_actions: usize,
    pub review_policy_actions: usize,
    pub provide_primitive_actions: usize,
    pub grant_capability_actions: usize,
    pub enable_dependency_actions: usize,
    pub total_constraints: usize,
    pub blocking_constraints: usize,
    pub review_constraints: usize,
    pub total_risks: usize,
    pub policy_tier_risks: usize,
    pub policy_surface_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub read_only_packets: usize,
    pub low_risk_packets: usize,
    pub human_approval_packets: usize,
    pub high_risk_packets: usize,
    pub first_approval_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDecisionItem {
    pub packet: IntegrationActivationApprovalPacket,
    pub decision_status: IntegrationActivationDecisionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDecisionSummary {
    pub total_decisions: usize,
    pub ready_to_approve_decisions: usize,
    pub blocked_decisions: usize,
    pub local_only_decisions: usize,
    pub cloud_required_decisions: usize,
    pub decisions_with_policy_surfaces: usize,
    pub decisions_without_policy_surfaces: usize,
    pub unique_policy_surfaces: usize,
    pub total_actions: usize,
    pub activate_integration_actions: usize,
    pub review_policy_actions: usize,
    pub provide_primitive_actions: usize,
    pub grant_capability_actions: usize,
    pub enable_dependency_actions: usize,
    pub total_constraints: usize,
    pub blocking_constraints: usize,
    pub review_constraints: usize,
    pub total_risks: usize,
    pub policy_tier_risks: usize,
    pub policy_surface_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub read_only_decisions: usize,
    pub low_risk_decisions: usize,
    pub human_approval_decisions: usize,
    pub high_risk_decisions: usize,
    pub first_approval_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationEvidenceItem {
    pub kind: IntegrationActivationEvidenceKind,
    pub status: IntegrationActivationEvidenceStatus,
    pub decision_status: IntegrationActivationDecisionStatus,
    pub requested_integration_id: IntegrationId,
    pub display_name: String,
    pub priority: u8,
    pub detail_id: String,
    pub primitive: Option<PrimitiveFamily>,
    pub capability_id: Option<CapabilityId>,
    pub dependency_integration_id: Option<IntegrationId>,
    pub policy_surface: Option<IntegrationPolicySurface>,
    pub required_tier: PrivilegeTier,
    pub local_only: bool,
    pub cloud_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationEvidenceSummary {
    pub total_evidence: usize,
    pub approval_decision_evidence: usize,
    pub policy_review_evidence: usize,
    pub primitive_blocker_evidence: usize,
    pub capability_blocker_evidence: usize,
    pub dependency_blocker_evidence: usize,
    pub policy_risk_evidence: usize,
    pub dependency_edge_evidence: usize,
    pub supporting_evidence: usize,
    pub review_evidence: usize,
    pub blocking_evidence: usize,
    pub unique_integrations: usize,
    pub ready_to_approve_integrations: usize,
    pub blocked_integrations: usize,
    pub local_only_integrations: usize,
    pub cloud_required_integrations: usize,
    pub unique_policy_surfaces: usize,
    pub read_only_evidence: usize,
    pub low_risk_evidence: usize,
    pub human_approval_evidence: usize,
    pub high_risk_evidence: usize,
    pub first_supporting_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocking_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDossierItem {
    pub decision: IntegrationActivationDecisionItem,
    pub evidence: Vec<IntegrationActivationEvidenceItem>,
    pub evidence_summary: IntegrationActivationEvidenceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDossierSummary {
    pub total_dossiers: usize,
    pub ready_to_approve_dossiers: usize,
    pub blocked_dossiers: usize,
    pub local_only_dossiers: usize,
    pub cloud_required_dossiers: usize,
    pub dossiers_with_policy_surfaces: usize,
    pub dossiers_without_policy_surfaces: usize,
    pub unique_policy_surfaces: usize,
    pub total_actions: usize,
    pub total_constraints: usize,
    pub total_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub total_evidence: usize,
    pub supporting_evidence: usize,
    pub review_evidence: usize,
    pub blocking_evidence: usize,
    pub read_only_dossiers: usize,
    pub low_risk_dossiers: usize,
    pub human_approval_dossiers: usize,
    pub high_risk_dossiers: usize,
    pub first_approval_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationAgendaStage {
    pub priority: u8,
    pub candidates: Vec<IntegrationActivationCandidate>,
    pub candidate_summary: IntegrationActivationCandidateSummary,
    pub actions: Vec<IntegrationActivationAction>,
    pub action_summary: IntegrationActivationActionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationAgendaSummary {
    pub total_stages: usize,
    pub total_candidates: usize,
    pub total_actions: usize,
    pub stages_with_activation_work: usize,
    pub stages_with_blockers: usize,
    pub stages_with_review_work: usize,
    pub first_action_priority: Option<u8>,
    pub first_activation_priority: Option<u8>,
    pub first_blocker_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub candidate_summary: IntegrationActivationCandidateSummary,
    pub action_summary: IntegrationActivationActionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationRunwayStage {
    pub priority: u8,
    pub candidates: Vec<IntegrationActivationCandidate>,
    pub summary: IntegrationActivationCandidateSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationRunwaySummary {
    pub total_stages: usize,
    pub total_candidates: usize,
    pub actionable_stages: usize,
    pub ready_stages: usize,
    pub review_stages: usize,
    pub blocked_stages: usize,
    pub first_actionable_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub next_ready_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub candidate_summary: IntegrationActivationCandidateSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationReadinessSummary {
    pub total_reports: usize,
    pub activation_ready_reports: usize,
    pub blocked_reports: usize,
    pub reports_requiring_human_review: usize,
    pub cloud_required_reports: usize,
    pub local_only_reports: usize,
    pub direct_targets: usize,
    pub delegated_integration_targets: usize,
    pub delegated_standard_targets: usize,
    pub reports_missing_primitives: usize,
    pub reports_missing_capabilities: usize,
    pub reports_missing_dependencies: usize,
    pub unique_missing_primitives: usize,
    pub unique_missing_capabilities: usize,
    pub unique_missing_dependencies: usize,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationReadinessPrimitiveGap {
    pub primitive: PrimitiveFamily,
    pub highest_priority: u8,
    pub blocked_report_count: usize,
    pub integration_ids: Vec<IntegrationId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationReadinessCapabilityGap {
    pub capability_id: CapabilityId,
    pub highest_priority: u8,
    pub blocked_report_count: usize,
    pub integration_ids: Vec<IntegrationId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationReadinessDependencyGap {
    pub integration_id: IntegrationId,
    pub highest_priority: u8,
    pub blocked_report_count: usize,
    pub requested_integration_ids: Vec<IntegrationId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationReadinessGapInventory {
    pub total_reports: usize,
    pub activation_ready_reports: usize,
    pub blocked_reports: usize,
    pub primitive_gaps: Vec<IntegrationReadinessPrimitiveGap>,
    pub capability_gaps: Vec<IntegrationReadinessCapabilityGap>,
    pub dependency_gaps: Vec<IntegrationReadinessDependencyGap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDependencyNode {
    pub integration_id: IntegrationId,
    pub display_name: String,
    pub priority: u8,
    pub activation_target: IntegrationActivationTarget,
    pub depends_on_integrations: Vec<IntegrationId>,
    pub dependent_integration_ids: Vec<IntegrationId>,
    pub missing_dependencies: Vec<IntegrationId>,
    pub enabled: bool,
    pub activation_ready: bool,
    pub requires_human_review: bool,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDependencyEdge {
    pub dependency_integration_id: IntegrationId,
    pub dependent_integration_id: IntegrationId,
    pub dependency_display_name: Option<String>,
    pub dependent_display_name: String,
    pub dependency_priority: Option<u8>,
    pub dependent_priority: u8,
    pub satisfied: bool,
    pub blocks_activation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDependencyGraph {
    pub nodes: Vec<IntegrationActivationDependencyNode>,
    pub edges: Vec<IntegrationActivationDependencyEdge>,
    pub summary: IntegrationActivationDependencySummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDependencySummary {
    pub total_nodes: usize,
    pub enabled_nodes: usize,
    pub activation_ready_nodes: usize,
    pub blocked_nodes: usize,
    pub nodes_with_dependencies: usize,
    pub nodes_with_dependents: usize,
    pub nodes_with_missing_dependencies: usize,
    pub total_edges: usize,
    pub satisfied_edges: usize,
    pub blocking_edges: usize,
    pub unknown_dependency_edges: usize,
    pub first_blocked_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationHealthStage {
    pub priority: u8,
    pub health_status: IntegrationActivationHealthStatus,
    pub integration_ids: Vec<IntegrationId>,
    pub ready_to_activate_integration_ids: Vec<IntegrationId>,
    pub review_integration_ids: Vec<IntegrationId>,
    pub blocked_integration_ids: Vec<IntegrationId>,
    pub candidate_summary: IntegrationActivationCandidateSummary,
    pub gap_inventory: IntegrationReadinessGapInventory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationHealthSummary {
    pub total_stages: usize,
    pub total_integrations: usize,
    pub ready_stages: usize,
    pub review_stages: usize,
    pub blocked_stages: usize,
    pub empty_stages: usize,
    pub activation_ready_integrations: usize,
    pub ready_to_activate_integrations: usize,
    pub review_integrations: usize,
    pub blocked_integrations: usize,
    pub primitive_gap_count: usize,
    pub capability_gap_count: usize,
    pub dependency_gap_count: usize,
    pub total_unique_gaps: usize,
    pub first_ready_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub overall_status: IntegrationActivationHealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationMaintenanceWindow {
    pub priority: u8,
    pub health_status: IntegrationActivationHealthStatus,
    pub integration_ids: Vec<IntegrationId>,
    pub ready_to_activate_integration_ids: Vec<IntegrationId>,
    pub review_integration_ids: Vec<IntegrationId>,
    pub blocked_integration_ids: Vec<IntegrationId>,
    pub candidate_summary: IntegrationActivationCandidateSummary,
    pub action_summary: IntegrationActivationActionSummary,
    pub constraint_summary: IntegrationActivationConstraintSummary,
    pub risk_summary: IntegrationActivationRiskSummary,
    pub dependency_summary: IntegrationActivationDependencySummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationMaintenanceSummary {
    pub total_windows: usize,
    pub total_integrations: usize,
    pub ready_windows: usize,
    pub review_windows: usize,
    pub blocked_windows: usize,
    pub empty_windows: usize,
    pub activation_ready_integrations: usize,
    pub ready_to_activate_integrations: usize,
    pub review_integrations: usize,
    pub blocked_integrations: usize,
    pub windows_with_actions: usize,
    pub windows_with_activation_work: usize,
    pub windows_with_review_work: usize,
    pub windows_with_blockers: usize,
    pub windows_with_risks: usize,
    pub windows_with_dependency_blockers: usize,
    pub total_actions: usize,
    pub activate_integration_actions: usize,
    pub review_policy_actions: usize,
    pub blocking_constraints: usize,
    pub review_constraints: usize,
    pub total_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub first_ready_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub first_activation_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub overall_status: IntegrationActivationHealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationReadoutStage {
    pub priority: u8,
    pub health_status: IntegrationActivationHealthStatus,
    pub integration_ids: Vec<IntegrationId>,
    pub maintenance_window: IntegrationActivationMaintenanceWindow,
    pub dossiers: Vec<IntegrationActivationDossierItem>,
    pub dossier_summary: IntegrationActivationDossierSummary,
    pub evidence_summary: IntegrationActivationEvidenceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationReadoutSummary {
    pub total_readouts: usize,
    pub total_integrations: usize,
    pub ready_readouts: usize,
    pub review_readouts: usize,
    pub blocked_readouts: usize,
    pub empty_readouts: usize,
    pub activation_ready_integrations: usize,
    pub ready_to_activate_integrations: usize,
    pub review_integrations: usize,
    pub blocked_integrations: usize,
    pub readouts_with_activation_work: usize,
    pub readouts_with_approval_work: usize,
    pub readouts_with_review_work: usize,
    pub readouts_with_blockers: usize,
    pub readouts_with_risks: usize,
    pub readouts_with_dependency_blockers: usize,
    pub total_actions: usize,
    pub activate_integration_actions: usize,
    pub review_policy_actions: usize,
    pub blocking_constraints: usize,
    pub review_constraints: usize,
    pub total_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub total_dossiers: usize,
    pub ready_to_approve_dossiers: usize,
    pub blocked_dossiers: usize,
    pub total_evidence: usize,
    pub supporting_evidence: usize,
    pub review_evidence: usize,
    pub blocking_evidence: usize,
    pub first_ready_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub first_activation_priority: Option<u8>,
    pub first_approval_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub overall_status: IntegrationActivationHealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationBriefingItem {
    pub kind: IntegrationActivationBriefingItemKind,
    pub priority: u8,
    pub health_status: IntegrationActivationHealthStatus,
    pub integration_ids: Vec<IntegrationId>,
    pub action_count: usize,
    pub dossier_count: usize,
    pub evidence_count: usize,
    pub risk_count: usize,
    pub dependency_edge_count: usize,
    pub blocking_dependency_edge_count: usize,
    pub highest_policy_tier: PrivilegeTier,
    pub has_activation_work: bool,
    pub has_approval_ready_work: bool,
    pub has_review_work: bool,
    pub has_blockers: bool,
    pub has_risks: bool,
    pub has_dependency_blockers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationBriefingSummary {
    pub total_items: usize,
    pub unique_integrations: usize,
    pub activation_items: usize,
    pub approval_items: usize,
    pub review_items: usize,
    pub blocker_items: usize,
    pub risk_items: usize,
    pub dependency_items: usize,
    pub items_requiring_attention: usize,
    pub items_with_activation_work: usize,
    pub items_with_approval_work: usize,
    pub items_with_review_work: usize,
    pub items_with_blockers: usize,
    pub items_with_risks: usize,
    pub items_with_dependency_blockers: usize,
    pub total_actions: usize,
    pub total_dossiers: usize,
    pub total_evidence: usize,
    pub total_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub first_activation_priority: Option<u8>,
    pub first_approval_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub first_risk_priority: Option<u8>,
    pub first_dependency_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub overall_status: IntegrationActivationHealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDashboardCard {
    pub priority: u8,
    pub health_status: IntegrationActivationHealthStatus,
    pub integration_ids: Vec<IntegrationId>,
    pub briefing_item_count: usize,
    pub next_briefing_kind: Option<IntegrationActivationBriefingItemKind>,
    pub briefing_summary: IntegrationActivationBriefingSummary,
    pub action_count: usize,
    pub dossier_count: usize,
    pub evidence_count: usize,
    pub risk_count: usize,
    pub dependency_edge_count: usize,
    pub blocking_dependency_edge_count: usize,
    pub highest_policy_tier: PrivilegeTier,
    pub has_activation_work: bool,
    pub has_approval_ready_work: bool,
    pub has_review_work: bool,
    pub has_blockers: bool,
    pub has_risks: bool,
    pub has_dependency_blockers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationDashboardSummary {
    pub total_cards: usize,
    pub unique_integrations: usize,
    pub ready_cards: usize,
    pub review_cards: usize,
    pub blocked_cards: usize,
    pub empty_cards: usize,
    pub cards_requiring_attention: usize,
    pub cards_with_activation_work: usize,
    pub cards_with_approval_work: usize,
    pub cards_with_review_work: usize,
    pub cards_with_blockers: usize,
    pub cards_with_risks: usize,
    pub cards_with_dependency_blockers: usize,
    pub total_briefing_items: usize,
    pub activation_items: usize,
    pub approval_items: usize,
    pub review_items: usize,
    pub blocker_items: usize,
    pub risk_items: usize,
    pub dependency_items: usize,
    pub total_actions: usize,
    pub total_dossiers: usize,
    pub total_evidence: usize,
    pub total_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub first_activation_priority: Option<u8>,
    pub first_approval_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub first_risk_priority: Option<u8>,
    pub first_dependency_priority: Option<u8>,
    pub first_attention_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub overall_status: IntegrationActivationHealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationTimelineMilestone {
    pub sequence: usize,
    pub priority: u8,
    pub milestone_kind: Option<IntegrationActivationBriefingItemKind>,
    pub dashboard_card: IntegrationActivationDashboardCard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationActivationTimelineSummary {
    pub total_milestones: usize,
    pub unique_integrations: usize,
    pub ready_milestones: usize,
    pub review_milestones: usize,
    pub blocked_milestones: usize,
    pub empty_milestones: usize,
    pub blocker_milestones: usize,
    pub review_queue_milestones: usize,
    pub approval_milestones: usize,
    pub activation_milestones: usize,
    pub risk_milestones: usize,
    pub dependency_milestones: usize,
    pub milestones_requiring_attention: usize,
    pub milestones_with_activation_work: usize,
    pub milestones_with_approval_work: usize,
    pub milestones_with_review_work: usize,
    pub milestones_with_blockers: usize,
    pub milestones_with_risks: usize,
    pub milestones_with_dependency_blockers: usize,
    pub total_briefing_items: usize,
    pub total_actions: usize,
    pub total_dossiers: usize,
    pub total_evidence: usize,
    pub total_risks: usize,
    pub total_dependency_edges: usize,
    pub blocking_dependency_edges: usize,
    pub first_activation_sequence: Option<usize>,
    pub first_approval_sequence: Option<usize>,
    pub first_review_sequence: Option<usize>,
    pub first_blocked_sequence: Option<usize>,
    pub first_risk_sequence: Option<usize>,
    pub first_dependency_sequence: Option<usize>,
    pub first_attention_sequence: Option<usize>,
    pub first_activation_priority: Option<u8>,
    pub first_approval_priority: Option<u8>,
    pub first_review_priority: Option<u8>,
    pub first_blocked_priority: Option<u8>,
    pub first_risk_priority: Option<u8>,
    pub first_dependency_priority: Option<u8>,
    pub first_attention_priority: Option<u8>,
    pub highest_policy_tier: PrivilegeTier,
    pub overall_status: IntegrationActivationHealthStatus,
}

impl IntegrationActivationPlanSummary {
    pub fn from_plans<'a>(plans: impl IntoIterator<Item = &'a IntegrationActivationPlan>) -> Self {
        let mut summary = Self {
            total_plans: 0,
            direct_targets: 0,
            delegated_integration_targets: 0,
            delegated_standard_targets: 0,
            plans_requiring_human_review: 0,
            local_only_plans: 0,
            cloud_required_plans: 0,
            plans_with_dependencies: 0,
            plans_with_required_primitives: 0,
            plans_with_required_capabilities: 0,
            unique_required_primitives: 0,
            unique_required_capabilities: 0,
            unique_dependencies: 0,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };
        let mut required_primitives = BTreeSet::new();
        let mut required_capabilities = BTreeSet::new();
        let mut dependencies = BTreeSet::new();

        for plan in plans {
            summary.total_plans += 1;
            match &plan.activation_target {
                IntegrationActivationTarget::Direct => summary.direct_targets += 1,
                IntegrationActivationTarget::DelegatedIntegration(_) => {
                    summary.delegated_integration_targets += 1
                }
                IntegrationActivationTarget::DelegatedStandards(_) => {
                    summary.delegated_standard_targets += 1
                }
            }
            if plan.requires_human_review() {
                summary.plans_requiring_human_review += 1;
            }
            if plan.local_only {
                summary.local_only_plans += 1;
            }
            if plan.cloud_required {
                summary.cloud_required_plans += 1;
            }
            if !plan.depends_on_integrations.is_empty() {
                summary.plans_with_dependencies += 1;
            }
            if !plan.required_primitives.is_empty() {
                summary.plans_with_required_primitives += 1;
            }
            if !plan.required_capabilities.is_empty() {
                summary.plans_with_required_capabilities += 1;
            }
            for primitive in &plan.required_primitives {
                required_primitives.insert(*primitive);
            }
            for capability_id in &plan.required_capabilities {
                required_capabilities.insert(capability_id.clone());
            }
            for integration_id in &plan.depends_on_integrations {
                dependencies.insert(integration_id.clone());
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(plan.highest_policy_tier);
        }

        summary.unique_required_primitives = required_primitives.len();
        summary.unique_required_capabilities = required_capabilities.len();
        summary.unique_dependencies = dependencies.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_plans == 0
    }

    pub fn has_delegated_targets(&self) -> bool {
        self.delegated_integration_targets > 0 || self.delegated_standard_targets > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.plans_requiring_human_review > 0
    }
}

impl IntegrationReadinessSummary {
    pub fn from_reports<'a>(
        reports: impl IntoIterator<Item = &'a IntegrationReadinessReport>,
    ) -> Self {
        let mut summary = Self {
            total_reports: 0,
            activation_ready_reports: 0,
            blocked_reports: 0,
            reports_requiring_human_review: 0,
            cloud_required_reports: 0,
            local_only_reports: 0,
            direct_targets: 0,
            delegated_integration_targets: 0,
            delegated_standard_targets: 0,
            reports_missing_primitives: 0,
            reports_missing_capabilities: 0,
            reports_missing_dependencies: 0,
            unique_missing_primitives: 0,
            unique_missing_capabilities: 0,
            unique_missing_dependencies: 0,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };
        let mut missing_primitives = BTreeSet::new();
        let mut missing_capabilities = BTreeSet::new();
        let mut missing_dependencies = BTreeSet::new();

        for report in reports {
            summary.total_reports += 1;
            if report.activation_ready() {
                summary.activation_ready_reports += 1;
            } else {
                summary.blocked_reports += 1;
            }
            if report.requires_human_review {
                summary.reports_requiring_human_review += 1;
            }
            if report.cloud_required {
                summary.cloud_required_reports += 1;
            }
            if report.local_only {
                summary.local_only_reports += 1;
            }
            match &report.activation_target {
                IntegrationActivationTarget::Direct => summary.direct_targets += 1,
                IntegrationActivationTarget::DelegatedIntegration(_) => {
                    summary.delegated_integration_targets += 1
                }
                IntegrationActivationTarget::DelegatedStandards(_) => {
                    summary.delegated_standard_targets += 1
                }
            }
            if !report.missing_primitives.is_empty() {
                summary.reports_missing_primitives += 1;
            }
            if !report.missing_capabilities.is_empty() {
                summary.reports_missing_capabilities += 1;
            }
            if !report.missing_dependencies.is_empty() {
                summary.reports_missing_dependencies += 1;
            }
            for primitive in &report.missing_primitives {
                missing_primitives.insert(*primitive);
            }
            for capability_id in &report.missing_capabilities {
                missing_capabilities.insert(capability_id.clone());
            }
            for integration_id in &report.missing_dependencies {
                missing_dependencies.insert(integration_id.clone());
            }
            summary.highest_policy_tier =
                summary.highest_policy_tier.max(report.highest_policy_tier);
        }

        summary.unique_missing_primitives = missing_primitives.len();
        summary.unique_missing_capabilities = missing_capabilities.len();
        summary.unique_missing_dependencies = missing_dependencies.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_reports == 0
    }

    pub fn all_ready(&self) -> bool {
        self.total_reports > 0 && self.blocked_reports == 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_reports > 0
    }
}

impl IntegrationReadinessGapInventory {
    pub fn is_empty(&self) -> bool {
        self.total_reports == 0
    }

    pub fn has_gaps(&self) -> bool {
        self.total_unique_gaps() > 0
    }

    pub fn all_ready(&self) -> bool {
        self.total_reports > 0 && !self.has_gaps()
    }

    pub fn primitive_gap_count(&self) -> usize {
        self.primitive_gaps.len()
    }

    pub fn capability_gap_count(&self) -> usize {
        self.capability_gaps.len()
    }

    pub fn dependency_gap_count(&self) -> usize {
        self.dependency_gaps.len()
    }

    pub fn total_unique_gaps(&self) -> usize {
        self.primitive_gap_count() + self.capability_gap_count() + self.dependency_gap_count()
    }
}

impl IntegrationActivationDependencySummary {
    pub fn from_graph(
        nodes: &[IntegrationActivationDependencyNode],
        edges: &[IntegrationActivationDependencyEdge],
    ) -> Self {
        let mut summary = Self {
            total_nodes: nodes.len(),
            enabled_nodes: 0,
            activation_ready_nodes: 0,
            blocked_nodes: 0,
            nodes_with_dependencies: 0,
            nodes_with_dependents: 0,
            nodes_with_missing_dependencies: 0,
            total_edges: edges.len(),
            satisfied_edges: 0,
            blocking_edges: 0,
            unknown_dependency_edges: 0,
            first_blocked_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for node in nodes {
            if node.enabled {
                summary.enabled_nodes += 1;
            }
            if node.activation_ready {
                summary.activation_ready_nodes += 1;
            } else {
                summary.blocked_nodes += 1;
                summary.first_blocked_priority = Some(
                    summary
                        .first_blocked_priority
                        .map_or(node.priority, |priority| priority.min(node.priority)),
                );
            }
            if !node.depends_on_integrations.is_empty() {
                summary.nodes_with_dependencies += 1;
            }
            if !node.dependent_integration_ids.is_empty() {
                summary.nodes_with_dependents += 1;
            }
            if !node.missing_dependencies.is_empty() {
                summary.nodes_with_missing_dependencies += 1;
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(node.highest_policy_tier);
        }

        for edge in edges {
            if edge.satisfied {
                summary.satisfied_edges += 1;
            }
            if edge.blocks_activation {
                summary.blocking_edges += 1;
            }
            if edge.dependency_display_name.is_none() {
                summary.unknown_dependency_edges += 1;
            }
        }

        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_nodes == 0
    }

    pub fn has_dependency_edges(&self) -> bool {
        self.total_edges > 0
    }

    pub fn has_blocking_dependencies(&self) -> bool {
        self.blocking_edges > 0
    }
}

impl IntegrationActivationDependencyGraph {
    pub fn is_empty(&self) -> bool {
        self.summary.is_empty()
    }

    pub fn has_blocking_dependencies(&self) -> bool {
        self.summary.has_blocking_dependencies()
    }
}

impl IntegrationActivationHealthStage {
    pub fn from_candidates(
        priority: u8,
        mut candidates: Vec<IntegrationActivationCandidate>,
    ) -> Self {
        candidates.sort_by(compare_activation_candidates);
        let candidate_summary =
            IntegrationActivationCandidateSummary::from_candidates(candidates.iter());
        let reports = candidates
            .iter()
            .map(|candidate| candidate.readiness_report.clone())
            .collect::<Vec<_>>();
        let gap_inventory = readiness_gap_inventory_from_reports(reports.iter());
        let integration_ids = candidates
            .iter()
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let ready_to_activate_integration_ids = candidates
            .iter()
            .filter(|candidate| {
                candidate.recommendation
                    == IntegrationActivationCandidateRecommendation::ReadyToActivate
            })
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let review_integration_ids = candidates
            .iter()
            .filter(|candidate| {
                candidate.recommendation
                    == IntegrationActivationCandidateRecommendation::NeedsHumanReview
            })
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let blocked_integration_ids = candidates
            .iter()
            .filter(|candidate| candidate.is_blocked())
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let health_status = activation_health_status_for_summary(&candidate_summary);

        Self {
            priority,
            health_status,
            integration_ids,
            ready_to_activate_integration_ids,
            review_integration_ids,
            blocked_integration_ids,
            candidate_summary,
            gap_inventory,
        }
    }

    pub fn has_ready_work(&self) -> bool {
        self.candidate_summary.ready_to_activate_candidates > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.candidate_summary.needs_human_review_candidates > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.candidate_summary.blocked_candidates > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.health_status.requires_attention()
    }
}

impl IntegrationActivationHealthSummary {
    pub fn from_stages<'a>(
        stages: impl IntoIterator<Item = &'a IntegrationActivationHealthStage>,
    ) -> Self {
        let mut primitive_gaps = BTreeSet::new();
        let mut capability_gaps = BTreeSet::new();
        let mut dependency_gaps = BTreeSet::new();
        let mut summary = Self {
            total_stages: 0,
            total_integrations: 0,
            ready_stages: 0,
            review_stages: 0,
            blocked_stages: 0,
            empty_stages: 0,
            activation_ready_integrations: 0,
            ready_to_activate_integrations: 0,
            review_integrations: 0,
            blocked_integrations: 0,
            primitive_gap_count: 0,
            capability_gap_count: 0,
            dependency_gap_count: 0,
            total_unique_gaps: 0,
            first_ready_priority: None,
            first_review_priority: None,
            first_blocked_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            overall_status: IntegrationActivationHealthStatus::Empty,
        };

        for stage in stages {
            summary.total_stages += 1;
            summary.total_integrations += stage.candidate_summary.total_candidates;
            summary.activation_ready_integrations +=
                stage.candidate_summary.activation_ready_candidates;
            summary.ready_to_activate_integrations +=
                stage.candidate_summary.ready_to_activate_candidates;
            summary.review_integrations += stage.candidate_summary.needs_human_review_candidates;
            summary.blocked_integrations += stage.candidate_summary.blocked_candidates;
            summary.highest_policy_tier = summary
                .highest_policy_tier
                .max(stage.candidate_summary.highest_policy_tier);

            if stage.candidate_summary.is_empty() {
                summary.empty_stages += 1;
            }
            if stage.has_ready_work() {
                summary.ready_stages += 1;
                summary.first_ready_priority =
                    min_optional_priority(summary.first_ready_priority, Some(stage.priority));
            }
            if stage.has_review_work() {
                summary.review_stages += 1;
                summary.first_review_priority =
                    min_optional_priority(summary.first_review_priority, Some(stage.priority));
            }
            if stage.has_blockers() {
                summary.blocked_stages += 1;
                summary.first_blocked_priority =
                    min_optional_priority(summary.first_blocked_priority, Some(stage.priority));
            }

            for gap in &stage.gap_inventory.primitive_gaps {
                primitive_gaps.insert(gap.primitive);
            }
            for gap in &stage.gap_inventory.capability_gaps {
                capability_gaps.insert(gap.capability_id.clone());
            }
            for gap in &stage.gap_inventory.dependency_gaps {
                dependency_gaps.insert(gap.integration_id.clone());
            }
        }

        summary.primitive_gap_count = primitive_gaps.len();
        summary.capability_gap_count = capability_gaps.len();
        summary.dependency_gap_count = dependency_gaps.len();
        summary.total_unique_gaps = summary.primitive_gap_count
            + summary.capability_gap_count
            + summary.dependency_gap_count;
        summary.overall_status = activation_health_status_from_counts(
            summary.ready_to_activate_integrations,
            summary.review_integrations,
            summary.blocked_integrations,
        );
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_stages == 0
    }

    pub fn has_ready_work(&self) -> bool {
        self.ready_to_activate_integrations > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_integrations > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_integrations > 0 || self.total_unique_gaps > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.overall_status.requires_attention()
    }
}

impl IntegrationActivationMaintenanceWindow {
    pub fn from_candidates(
        catalog: &[IntegrationCatalogEntry],
        priority: u8,
        mut candidates: Vec<IntegrationActivationCandidate>,
        enabled_integrations: &[IntegrationId],
    ) -> Self {
        candidates.sort_by(compare_activation_candidates);
        let reports = candidates
            .iter()
            .map(|candidate| candidate.readiness_report.clone())
            .collect::<Vec<_>>();
        let integration_ids = candidates
            .iter()
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let ready_to_activate_integration_ids = candidates
            .iter()
            .filter(|candidate| {
                candidate.recommendation
                    == IntegrationActivationCandidateRecommendation::ReadyToActivate
            })
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let review_integration_ids = candidates
            .iter()
            .filter(|candidate| {
                candidate.recommendation
                    == IntegrationActivationCandidateRecommendation::NeedsHumanReview
            })
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let blocked_integration_ids = candidates
            .iter()
            .filter(|candidate| candidate.is_blocked())
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let candidate_summary =
            IntegrationActivationCandidateSummary::from_candidates(candidates.iter());
        let action_summary = IntegrationActivationActionSummary::from_actions(
            activation_actions_from_candidates(candidates.iter()).iter(),
        );
        let constraint_summary = IntegrationActivationConstraintSummary::from_constraints(
            activation_constraints_from_candidates(catalog, candidates.iter()).iter(),
        );
        let risk_summary = IntegrationActivationRiskSummary::from_risks(
            activation_risk_from_candidates(catalog, candidates.iter()).iter(),
        );
        let dependency_summary =
            activation_dependency_graph_from_reports(catalog, reports.iter(), enabled_integrations)
                .summary;
        let health_status = activation_health_status_for_summary(&candidate_summary);

        Self {
            priority,
            health_status,
            integration_ids,
            ready_to_activate_integration_ids,
            review_integration_ids,
            blocked_integration_ids,
            candidate_summary,
            action_summary,
            constraint_summary,
            risk_summary,
            dependency_summary,
        }
    }

    pub fn has_ready_work(&self) -> bool {
        self.candidate_summary.ready_to_activate_candidates > 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.action_summary.has_activation_work()
    }

    pub fn has_review_work(&self) -> bool {
        self.candidate_summary.has_review_work()
            || self.action_summary.has_review_work()
            || self.constraint_summary.has_review_work()
            || self.risk_summary.has_review_work()
    }

    pub fn has_blockers(&self) -> bool {
        self.candidate_summary.has_blockers()
            || self.action_summary.has_blockers()
            || self.constraint_summary.has_blockers()
            || self.risk_summary.has_blockers()
            || self.dependency_summary.has_blocking_dependencies()
    }

    pub fn has_risks(&self) -> bool {
        !self.risk_summary.is_empty()
    }

    pub fn has_dependency_blockers(&self) -> bool {
        self.dependency_summary.has_blocking_dependencies()
    }

    pub fn requires_attention(&self) -> bool {
        self.health_status.requires_attention()
            || self.has_review_work()
            || self.has_blockers()
            || self.risk_summary.requires_attention()
    }
}

impl IntegrationActivationMaintenanceSummary {
    pub fn from_windows<'a>(
        windows: impl IntoIterator<Item = &'a IntegrationActivationMaintenanceWindow>,
    ) -> Self {
        let mut summary = Self {
            total_windows: 0,
            total_integrations: 0,
            ready_windows: 0,
            review_windows: 0,
            blocked_windows: 0,
            empty_windows: 0,
            activation_ready_integrations: 0,
            ready_to_activate_integrations: 0,
            review_integrations: 0,
            blocked_integrations: 0,
            windows_with_actions: 0,
            windows_with_activation_work: 0,
            windows_with_review_work: 0,
            windows_with_blockers: 0,
            windows_with_risks: 0,
            windows_with_dependency_blockers: 0,
            total_actions: 0,
            activate_integration_actions: 0,
            review_policy_actions: 0,
            blocking_constraints: 0,
            review_constraints: 0,
            total_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            first_ready_priority: None,
            first_review_priority: None,
            first_blocked_priority: None,
            first_activation_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            overall_status: IntegrationActivationHealthStatus::Empty,
        };

        for window in windows {
            summary.total_windows += 1;
            summary.total_integrations += window.candidate_summary.total_candidates;
            summary.activation_ready_integrations +=
                window.candidate_summary.activation_ready_candidates;
            summary.ready_to_activate_integrations +=
                window.candidate_summary.ready_to_activate_candidates;
            summary.review_integrations += window.candidate_summary.needs_human_review_candidates;
            summary.blocked_integrations += window.candidate_summary.blocked_candidates;
            summary.total_actions += window.action_summary.total_actions;
            summary.activate_integration_actions +=
                window.action_summary.activate_integration_actions;
            summary.review_policy_actions += window.action_summary.review_policy_actions;
            summary.blocking_constraints += window.constraint_summary.blocking_constraints;
            summary.review_constraints += window.constraint_summary.review_constraints;
            summary.total_risks += window.risk_summary.total_risks;
            summary.total_dependency_edges += window.dependency_summary.total_edges;
            summary.blocking_dependency_edges += window.dependency_summary.blocking_edges;
            summary.highest_policy_tier = summary
                .highest_policy_tier
                .max(window.candidate_summary.highest_policy_tier)
                .max(window.action_summary.highest_policy_tier)
                .max(window.constraint_summary.highest_policy_tier)
                .max(window.risk_summary.highest_policy_tier)
                .max(window.dependency_summary.highest_policy_tier);

            match window.health_status {
                IntegrationActivationHealthStatus::Ready => summary.ready_windows += 1,
                IntegrationActivationHealthStatus::NeedsReview => summary.review_windows += 1,
                IntegrationActivationHealthStatus::Blocked => summary.blocked_windows += 1,
                IntegrationActivationHealthStatus::Empty => summary.empty_windows += 1,
            }

            if !window.action_summary.is_empty() {
                summary.windows_with_actions += 1;
            }
            if window.has_activation_work() {
                summary.windows_with_activation_work += 1;
                summary.first_activation_priority =
                    min_optional_priority(summary.first_activation_priority, Some(window.priority));
            }
            if window.has_ready_work() {
                summary.first_ready_priority =
                    min_optional_priority(summary.first_ready_priority, Some(window.priority));
            }
            if window.has_review_work() {
                summary.windows_with_review_work += 1;
                summary.first_review_priority =
                    min_optional_priority(summary.first_review_priority, Some(window.priority));
            }
            if window.has_blockers() {
                summary.windows_with_blockers += 1;
                summary.first_blocked_priority =
                    min_optional_priority(summary.first_blocked_priority, Some(window.priority));
            }
            if window.has_risks() {
                summary.windows_with_risks += 1;
            }
            if window.has_dependency_blockers() {
                summary.windows_with_dependency_blockers += 1;
            }
        }

        summary.overall_status = activation_health_status_from_counts(
            summary.ready_to_activate_integrations,
            summary.review_integrations,
            summary.blocked_integrations,
        );
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_windows == 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.activate_integration_actions > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_integrations > 0
            || self.review_policy_actions > 0
            || self.review_constraints > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_integrations > 0
            || self.blocking_constraints > 0
            || self.blocking_dependency_edges > 0
    }

    pub fn has_risks(&self) -> bool {
        self.total_risks > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.overall_status.requires_attention() || self.has_review_work() || self.has_blockers()
    }
}

impl IntegrationActivationReadoutStage {
    pub fn from_candidates(
        catalog: &[IntegrationCatalogEntry],
        priority: u8,
        mut candidates: Vec<IntegrationActivationCandidate>,
        enabled_integrations: &[IntegrationId],
    ) -> Self {
        candidates.sort_by(compare_activation_candidates);
        let integration_ids = candidates
            .iter()
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let maintenance_window = IntegrationActivationMaintenanceWindow::from_candidates(
            catalog,
            priority,
            candidates.clone(),
            enabled_integrations,
        );
        let dossiers =
            activation_dossiers_from_candidates(catalog, candidates.iter(), enabled_integrations);
        let dossier_summary = IntegrationActivationDossierSummary::from_dossiers(dossiers.iter());
        let evidence_summary = IntegrationActivationEvidenceSummary::from_evidence(
            dossiers.iter().flat_map(|dossier| dossier.evidence.iter()),
        );
        let health_status = maintenance_window.health_status;

        Self {
            priority,
            health_status,
            integration_ids,
            maintenance_window,
            dossiers,
            dossier_summary,
            evidence_summary,
        }
    }

    pub fn candidate_summary(&self) -> &IntegrationActivationCandidateSummary {
        &self.maintenance_window.candidate_summary
    }

    pub fn action_summary(&self) -> &IntegrationActivationActionSummary {
        &self.maintenance_window.action_summary
    }

    pub fn constraint_summary(&self) -> &IntegrationActivationConstraintSummary {
        &self.maintenance_window.constraint_summary
    }

    pub fn risk_summary(&self) -> &IntegrationActivationRiskSummary {
        &self.maintenance_window.risk_summary
    }

    pub fn dependency_summary(&self) -> &IntegrationActivationDependencySummary {
        &self.maintenance_window.dependency_summary
    }

    pub fn has_ready_work(&self) -> bool {
        self.maintenance_window.has_ready_work()
    }

    pub fn has_activation_work(&self) -> bool {
        self.maintenance_window.has_activation_work()
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.dossier_summary.has_approval_ready_work()
    }

    pub fn has_review_work(&self) -> bool {
        self.maintenance_window.has_review_work()
            || self.dossier_summary.has_review_work()
            || self.evidence_summary.has_review_work()
    }

    pub fn has_blockers(&self) -> bool {
        self.maintenance_window.has_blockers()
            || self.dossier_summary.has_blockers()
            || self.evidence_summary.has_blockers()
    }

    pub fn has_risks(&self) -> bool {
        self.maintenance_window.has_risks()
    }

    pub fn has_dependency_blockers(&self) -> bool {
        self.maintenance_window.has_dependency_blockers()
    }

    pub fn requires_attention(&self) -> bool {
        self.maintenance_window.requires_attention()
            || self.dossier_summary.requires_attention()
            || self.evidence_summary.requires_attention()
    }
}

impl IntegrationActivationReadoutSummary {
    pub fn from_readouts<'a>(
        readouts: impl IntoIterator<Item = &'a IntegrationActivationReadoutStage>,
    ) -> Self {
        let mut summary = Self {
            total_readouts: 0,
            total_integrations: 0,
            ready_readouts: 0,
            review_readouts: 0,
            blocked_readouts: 0,
            empty_readouts: 0,
            activation_ready_integrations: 0,
            ready_to_activate_integrations: 0,
            review_integrations: 0,
            blocked_integrations: 0,
            readouts_with_activation_work: 0,
            readouts_with_approval_work: 0,
            readouts_with_review_work: 0,
            readouts_with_blockers: 0,
            readouts_with_risks: 0,
            readouts_with_dependency_blockers: 0,
            total_actions: 0,
            activate_integration_actions: 0,
            review_policy_actions: 0,
            blocking_constraints: 0,
            review_constraints: 0,
            total_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            total_dossiers: 0,
            ready_to_approve_dossiers: 0,
            blocked_dossiers: 0,
            total_evidence: 0,
            supporting_evidence: 0,
            review_evidence: 0,
            blocking_evidence: 0,
            first_ready_priority: None,
            first_review_priority: None,
            first_blocked_priority: None,
            first_activation_priority: None,
            first_approval_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            overall_status: IntegrationActivationHealthStatus::Empty,
        };

        for readout in readouts {
            summary.total_readouts += 1;
            summary.total_integrations += readout.candidate_summary().total_candidates;
            summary.activation_ready_integrations +=
                readout.candidate_summary().activation_ready_candidates;
            summary.ready_to_activate_integrations +=
                readout.candidate_summary().ready_to_activate_candidates;
            summary.review_integrations +=
                readout.candidate_summary().needs_human_review_candidates;
            summary.blocked_integrations += readout.candidate_summary().blocked_candidates;

            summary.total_actions += readout.action_summary().total_actions;
            summary.activate_integration_actions +=
                readout.action_summary().activate_integration_actions;
            summary.review_policy_actions += readout.action_summary().review_policy_actions;
            summary.blocking_constraints += readout.constraint_summary().blocking_constraints;
            summary.review_constraints += readout.constraint_summary().review_constraints;
            summary.total_risks += readout.risk_summary().total_risks;
            summary.total_dependency_edges += readout.dependency_summary().total_edges;
            summary.blocking_dependency_edges += readout.dependency_summary().blocking_edges;

            summary.total_dossiers += readout.dossier_summary.total_dossiers;
            summary.ready_to_approve_dossiers += readout.dossier_summary.ready_to_approve_dossiers;
            summary.blocked_dossiers += readout.dossier_summary.blocked_dossiers;
            summary.total_evidence += readout.evidence_summary.total_evidence;
            summary.supporting_evidence += readout.evidence_summary.supporting_evidence;
            summary.review_evidence += readout.evidence_summary.review_evidence;
            summary.blocking_evidence += readout.evidence_summary.blocking_evidence;

            summary.highest_policy_tier = summary
                .highest_policy_tier
                .max(readout.candidate_summary().highest_policy_tier)
                .max(readout.action_summary().highest_policy_tier)
                .max(readout.constraint_summary().highest_policy_tier)
                .max(readout.risk_summary().highest_policy_tier)
                .max(readout.dependency_summary().highest_policy_tier)
                .max(readout.dossier_summary.highest_policy_tier)
                .max(readout.evidence_summary.highest_policy_tier);

            match readout.health_status {
                IntegrationActivationHealthStatus::Ready => summary.ready_readouts += 1,
                IntegrationActivationHealthStatus::NeedsReview => summary.review_readouts += 1,
                IntegrationActivationHealthStatus::Blocked => summary.blocked_readouts += 1,
                IntegrationActivationHealthStatus::Empty => summary.empty_readouts += 1,
            }

            if readout.has_activation_work() {
                summary.readouts_with_activation_work += 1;
                summary.first_activation_priority = min_optional_priority(
                    summary.first_activation_priority,
                    Some(readout.priority),
                );
            }
            if readout.has_approval_ready_work() {
                summary.readouts_with_approval_work += 1;
                summary.first_approval_priority =
                    min_optional_priority(summary.first_approval_priority, Some(readout.priority));
            }
            if readout.has_ready_work() {
                summary.first_ready_priority =
                    min_optional_priority(summary.first_ready_priority, Some(readout.priority));
            }
            if readout.has_review_work() {
                summary.readouts_with_review_work += 1;
                summary.first_review_priority =
                    min_optional_priority(summary.first_review_priority, Some(readout.priority));
            }
            if readout.has_blockers() {
                summary.readouts_with_blockers += 1;
                summary.first_blocked_priority =
                    min_optional_priority(summary.first_blocked_priority, Some(readout.priority));
            }
            if readout.has_risks() {
                summary.readouts_with_risks += 1;
            }
            if readout.has_dependency_blockers() {
                summary.readouts_with_dependency_blockers += 1;
            }
        }

        summary.overall_status = activation_health_status_from_counts(
            summary.ready_to_activate_integrations,
            summary.review_integrations,
            summary.blocked_integrations,
        );
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_readouts == 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.activate_integration_actions > 0
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.ready_to_approve_dossiers > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_integrations > 0
            || self.review_policy_actions > 0
            || self.review_constraints > 0
            || self.review_evidence > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_integrations > 0
            || self.blocking_constraints > 0
            || self.blocking_dependency_edges > 0
            || self.blocking_evidence > 0
    }

    pub fn has_risks(&self) -> bool {
        self.total_risks > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.overall_status.requires_attention()
            || self.has_approval_ready_work()
            || self.has_review_work()
            || self.has_blockers()
    }
}

impl IntegrationActivationBriefingItem {
    fn from_readout(
        kind: IntegrationActivationBriefingItemKind,
        readout: &IntegrationActivationReadoutStage,
    ) -> Self {
        Self {
            kind,
            priority: readout.priority,
            health_status: readout.health_status,
            integration_ids: readout.integration_ids.clone(),
            action_count: readout.action_summary().total_actions,
            dossier_count: readout.dossier_summary.total_dossiers,
            evidence_count: readout.evidence_summary.total_evidence,
            risk_count: readout.risk_summary().total_risks,
            dependency_edge_count: readout.dependency_summary().total_edges,
            blocking_dependency_edge_count: readout.dependency_summary().blocking_edges,
            highest_policy_tier: readout
                .candidate_summary()
                .highest_policy_tier
                .max(readout.action_summary().highest_policy_tier)
                .max(readout.constraint_summary().highest_policy_tier)
                .max(readout.risk_summary().highest_policy_tier)
                .max(readout.dependency_summary().highest_policy_tier)
                .max(readout.dossier_summary.highest_policy_tier)
                .max(readout.evidence_summary.highest_policy_tier),
            has_activation_work: readout.has_activation_work(),
            has_approval_ready_work: readout.has_approval_ready_work(),
            has_review_work: readout.has_review_work(),
            has_blockers: readout.has_blockers(),
            has_risks: readout.has_risks(),
            has_dependency_blockers: readout.has_dependency_blockers(),
        }
    }

    pub fn integration_count(&self) -> usize {
        self.integration_ids.len()
    }

    pub fn requires_attention(&self) -> bool {
        self.health_status.requires_attention()
            || self.has_approval_ready_work
            || self.has_review_work
            || self.has_blockers
            || self.has_risks
            || self.has_dependency_blockers
    }
}

impl IntegrationActivationBriefingSummary {
    pub fn from_items<'a>(
        items: impl IntoIterator<Item = &'a IntegrationActivationBriefingItem>,
    ) -> Self {
        let mut integration_ids = BTreeSet::new();
        let mut ready_items = 0;
        let mut review_status_items = 0;
        let mut blocked_status_items = 0;
        let mut empty_items = 0;
        let mut summary = Self {
            total_items: 0,
            unique_integrations: 0,
            activation_items: 0,
            approval_items: 0,
            review_items: 0,
            blocker_items: 0,
            risk_items: 0,
            dependency_items: 0,
            items_requiring_attention: 0,
            items_with_activation_work: 0,
            items_with_approval_work: 0,
            items_with_review_work: 0,
            items_with_blockers: 0,
            items_with_risks: 0,
            items_with_dependency_blockers: 0,
            total_actions: 0,
            total_dossiers: 0,
            total_evidence: 0,
            total_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            first_activation_priority: None,
            first_approval_priority: None,
            first_review_priority: None,
            first_blocked_priority: None,
            first_risk_priority: None,
            first_dependency_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            overall_status: IntegrationActivationHealthStatus::Empty,
        };

        for item in items {
            summary.total_items += 1;
            for integration_id in &item.integration_ids {
                integration_ids.insert(integration_id.clone());
            }
            summary.total_actions += item.action_count;
            summary.total_dossiers += item.dossier_count;
            summary.total_evidence += item.evidence_count;
            summary.total_risks += item.risk_count;
            summary.total_dependency_edges += item.dependency_edge_count;
            summary.blocking_dependency_edges += item.blocking_dependency_edge_count;
            summary.highest_policy_tier = summary.highest_policy_tier.max(item.highest_policy_tier);

            match item.health_status {
                IntegrationActivationHealthStatus::Ready => ready_items += 1,
                IntegrationActivationHealthStatus::NeedsReview => review_status_items += 1,
                IntegrationActivationHealthStatus::Blocked => blocked_status_items += 1,
                IntegrationActivationHealthStatus::Empty => empty_items += 1,
            }

            match item.kind {
                IntegrationActivationBriefingItemKind::Activation => {
                    summary.activation_items += 1;
                    summary.first_activation_priority = min_optional_priority(
                        summary.first_activation_priority,
                        Some(item.priority),
                    );
                }
                IntegrationActivationBriefingItemKind::Approval => {
                    summary.approval_items += 1;
                    summary.first_approval_priority =
                        min_optional_priority(summary.first_approval_priority, Some(item.priority));
                }
                IntegrationActivationBriefingItemKind::Review => {
                    summary.review_items += 1;
                    summary.first_review_priority =
                        min_optional_priority(summary.first_review_priority, Some(item.priority));
                }
                IntegrationActivationBriefingItemKind::Blocker => {
                    summary.blocker_items += 1;
                    summary.first_blocked_priority =
                        min_optional_priority(summary.first_blocked_priority, Some(item.priority));
                }
                IntegrationActivationBriefingItemKind::Risk => {
                    summary.risk_items += 1;
                    summary.first_risk_priority =
                        min_optional_priority(summary.first_risk_priority, Some(item.priority));
                }
                IntegrationActivationBriefingItemKind::Dependency => {
                    summary.dependency_items += 1;
                    summary.first_dependency_priority = min_optional_priority(
                        summary.first_dependency_priority,
                        Some(item.priority),
                    );
                }
            }

            if item.requires_attention() {
                summary.items_requiring_attention += 1;
            }
            if item.has_activation_work {
                summary.items_with_activation_work += 1;
            }
            if item.has_approval_ready_work {
                summary.items_with_approval_work += 1;
            }
            if item.has_review_work {
                summary.items_with_review_work += 1;
            }
            if item.has_blockers {
                summary.items_with_blockers += 1;
            }
            if item.has_risks {
                summary.items_with_risks += 1;
            }
            if item.has_dependency_blockers {
                summary.items_with_dependency_blockers += 1;
            }
        }

        summary.unique_integrations = integration_ids.len();
        summary.overall_status = if blocked_status_items > 0 || summary.blocker_items > 0 {
            IntegrationActivationHealthStatus::Blocked
        } else if review_status_items > 0 || summary.review_items > 0 || summary.approval_items > 0
        {
            IntegrationActivationHealthStatus::NeedsReview
        } else if ready_items > 0 {
            IntegrationActivationHealthStatus::Ready
        } else if empty_items > 0 {
            IntegrationActivationHealthStatus::Empty
        } else {
            IntegrationActivationHealthStatus::Empty
        };
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_items == 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.activation_items > 0 || self.items_with_activation_work > 0
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.approval_items > 0 || self.items_with_approval_work > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_items > 0 || self.items_with_review_work > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocker_items > 0 || self.items_with_blockers > 0
    }

    pub fn has_risks(&self) -> bool {
        self.risk_items > 0 || self.items_with_risks > 0
    }

    pub fn has_dependency_blockers(&self) -> bool {
        self.dependency_items > 0 || self.items_with_dependency_blockers > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.overall_status.requires_attention()
            || self.has_approval_ready_work()
            || self.has_review_work()
            || self.has_blockers()
            || self.has_risks()
            || self.has_dependency_blockers()
    }
}

impl IntegrationActivationDashboardCard {
    fn from_readout(readout: &IntegrationActivationReadoutStage) -> Self {
        let briefing_items = activation_briefing_items_from_readouts(std::iter::once(readout));
        let briefing_summary =
            IntegrationActivationBriefingSummary::from_items(briefing_items.iter());
        let next_briefing_kind = briefing_items.first().map(|item| item.kind);

        Self {
            priority: readout.priority,
            health_status: readout.health_status,
            integration_ids: readout.integration_ids.clone(),
            briefing_item_count: briefing_items.len(),
            next_briefing_kind,
            briefing_summary,
            action_count: readout.action_summary().total_actions,
            dossier_count: readout.dossier_summary.total_dossiers,
            evidence_count: readout.evidence_summary.total_evidence,
            risk_count: readout.risk_summary().total_risks,
            dependency_edge_count: readout.dependency_summary().total_edges,
            blocking_dependency_edge_count: readout.dependency_summary().blocking_edges,
            highest_policy_tier: readout
                .candidate_summary()
                .highest_policy_tier
                .max(readout.action_summary().highest_policy_tier)
                .max(readout.constraint_summary().highest_policy_tier)
                .max(readout.risk_summary().highest_policy_tier)
                .max(readout.dependency_summary().highest_policy_tier)
                .max(readout.dossier_summary.highest_policy_tier)
                .max(readout.evidence_summary.highest_policy_tier),
            has_activation_work: readout.has_activation_work(),
            has_approval_ready_work: readout.has_approval_ready_work(),
            has_review_work: readout.has_review_work(),
            has_blockers: readout.has_blockers(),
            has_risks: readout.has_risks(),
            has_dependency_blockers: readout.has_dependency_blockers(),
        }
    }

    pub fn integration_count(&self) -> usize {
        self.integration_ids.len()
    }

    pub fn requires_attention(&self) -> bool {
        self.health_status.requires_attention()
            || self.has_approval_ready_work
            || self.has_review_work
            || self.has_blockers
            || self.has_risks
            || self.has_dependency_blockers
    }
}

impl IntegrationActivationDashboardSummary {
    pub fn from_cards<'a>(
        cards: impl IntoIterator<Item = &'a IntegrationActivationDashboardCard>,
    ) -> Self {
        let mut integration_ids = BTreeSet::new();
        let mut summary = Self {
            total_cards: 0,
            unique_integrations: 0,
            ready_cards: 0,
            review_cards: 0,
            blocked_cards: 0,
            empty_cards: 0,
            cards_requiring_attention: 0,
            cards_with_activation_work: 0,
            cards_with_approval_work: 0,
            cards_with_review_work: 0,
            cards_with_blockers: 0,
            cards_with_risks: 0,
            cards_with_dependency_blockers: 0,
            total_briefing_items: 0,
            activation_items: 0,
            approval_items: 0,
            review_items: 0,
            blocker_items: 0,
            risk_items: 0,
            dependency_items: 0,
            total_actions: 0,
            total_dossiers: 0,
            total_evidence: 0,
            total_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            first_activation_priority: None,
            first_approval_priority: None,
            first_review_priority: None,
            first_blocked_priority: None,
            first_risk_priority: None,
            first_dependency_priority: None,
            first_attention_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            overall_status: IntegrationActivationHealthStatus::Empty,
        };

        for card in cards {
            summary.total_cards += 1;
            for integration_id in &card.integration_ids {
                integration_ids.insert(integration_id.clone());
            }

            match card.health_status {
                IntegrationActivationHealthStatus::Ready => summary.ready_cards += 1,
                IntegrationActivationHealthStatus::NeedsReview => summary.review_cards += 1,
                IntegrationActivationHealthStatus::Blocked => summary.blocked_cards += 1,
                IntegrationActivationHealthStatus::Empty => summary.empty_cards += 1,
            }

            summary.total_briefing_items += card.briefing_item_count;
            summary.activation_items += card.briefing_summary.activation_items;
            summary.approval_items += card.briefing_summary.approval_items;
            summary.review_items += card.briefing_summary.review_items;
            summary.blocker_items += card.briefing_summary.blocker_items;
            summary.risk_items += card.briefing_summary.risk_items;
            summary.dependency_items += card.briefing_summary.dependency_items;
            summary.total_actions += card.action_count;
            summary.total_dossiers += card.dossier_count;
            summary.total_evidence += card.evidence_count;
            summary.total_risks += card.risk_count;
            summary.total_dependency_edges += card.dependency_edge_count;
            summary.blocking_dependency_edges += card.blocking_dependency_edge_count;
            summary.highest_policy_tier = summary.highest_policy_tier.max(card.highest_policy_tier);

            if card.requires_attention() {
                summary.cards_requiring_attention += 1;
                summary.first_attention_priority =
                    min_optional_priority(summary.first_attention_priority, Some(card.priority));
            }
            if card.has_activation_work {
                summary.cards_with_activation_work += 1;
                summary.first_activation_priority =
                    min_optional_priority(summary.first_activation_priority, Some(card.priority));
            }
            if card.has_approval_ready_work {
                summary.cards_with_approval_work += 1;
                summary.first_approval_priority =
                    min_optional_priority(summary.first_approval_priority, Some(card.priority));
            }
            if card.has_review_work {
                summary.cards_with_review_work += 1;
                summary.first_review_priority =
                    min_optional_priority(summary.first_review_priority, Some(card.priority));
            }
            if card.has_blockers {
                summary.cards_with_blockers += 1;
                summary.first_blocked_priority =
                    min_optional_priority(summary.first_blocked_priority, Some(card.priority));
            }
            if card.has_risks {
                summary.cards_with_risks += 1;
                summary.first_risk_priority =
                    min_optional_priority(summary.first_risk_priority, Some(card.priority));
            }
            if card.has_dependency_blockers {
                summary.cards_with_dependency_blockers += 1;
                summary.first_dependency_priority =
                    min_optional_priority(summary.first_dependency_priority, Some(card.priority));
            }
        }

        summary.unique_integrations = integration_ids.len();
        summary.overall_status = if summary.blocked_cards > 0 || summary.cards_with_blockers > 0 {
            IntegrationActivationHealthStatus::Blocked
        } else if summary.review_cards > 0
            || summary.cards_with_review_work > 0
            || summary.cards_with_approval_work > 0
        {
            IntegrationActivationHealthStatus::NeedsReview
        } else if summary.ready_cards > 0 || summary.cards_with_activation_work > 0 {
            IntegrationActivationHealthStatus::Ready
        } else {
            IntegrationActivationHealthStatus::Empty
        };
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_cards == 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.cards_with_activation_work > 0 || self.activation_items > 0
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.cards_with_approval_work > 0 || self.approval_items > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.cards_with_review_work > 0 || self.review_items > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.cards_with_blockers > 0 || self.blocker_items > 0
    }

    pub fn has_risks(&self) -> bool {
        self.cards_with_risks > 0 || self.risk_items > 0
    }

    pub fn has_dependency_blockers(&self) -> bool {
        self.cards_with_dependency_blockers > 0 || self.dependency_items > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.overall_status.requires_attention()
            || self.has_approval_ready_work()
            || self.has_review_work()
            || self.has_blockers()
            || self.has_risks()
            || self.has_dependency_blockers()
    }
}

impl IntegrationActivationTimelineMilestone {
    fn from_dashboard_card(
        sequence: usize,
        dashboard_card: IntegrationActivationDashboardCard,
    ) -> Self {
        Self {
            sequence,
            priority: dashboard_card.priority,
            milestone_kind: dashboard_card.next_briefing_kind,
            dashboard_card,
        }
    }

    pub fn integration_count(&self) -> usize {
        self.dashboard_card.integration_count()
    }

    pub fn requires_attention(&self) -> bool {
        self.dashboard_card.requires_attention()
    }

    pub fn has_activation_work(&self) -> bool {
        self.dashboard_card.has_activation_work
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.dashboard_card.has_approval_ready_work
    }

    pub fn has_review_work(&self) -> bool {
        self.dashboard_card.has_review_work
    }

    pub fn has_blockers(&self) -> bool {
        self.dashboard_card.has_blockers
    }

    pub fn has_risks(&self) -> bool {
        self.dashboard_card.has_risks
    }

    pub fn has_dependency_blockers(&self) -> bool {
        self.dashboard_card.has_dependency_blockers
    }
}

impl IntegrationActivationTimelineSummary {
    pub fn from_milestones<'a>(
        milestones: impl IntoIterator<Item = &'a IntegrationActivationTimelineMilestone>,
    ) -> Self {
        let mut integration_ids = BTreeSet::new();
        let mut summary = Self {
            total_milestones: 0,
            unique_integrations: 0,
            ready_milestones: 0,
            review_milestones: 0,
            blocked_milestones: 0,
            empty_milestones: 0,
            blocker_milestones: 0,
            review_queue_milestones: 0,
            approval_milestones: 0,
            activation_milestones: 0,
            risk_milestones: 0,
            dependency_milestones: 0,
            milestones_requiring_attention: 0,
            milestones_with_activation_work: 0,
            milestones_with_approval_work: 0,
            milestones_with_review_work: 0,
            milestones_with_blockers: 0,
            milestones_with_risks: 0,
            milestones_with_dependency_blockers: 0,
            total_briefing_items: 0,
            total_actions: 0,
            total_dossiers: 0,
            total_evidence: 0,
            total_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            first_activation_sequence: None,
            first_approval_sequence: None,
            first_review_sequence: None,
            first_blocked_sequence: None,
            first_risk_sequence: None,
            first_dependency_sequence: None,
            first_attention_sequence: None,
            first_activation_priority: None,
            first_approval_priority: None,
            first_review_priority: None,
            first_blocked_priority: None,
            first_risk_priority: None,
            first_dependency_priority: None,
            first_attention_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            overall_status: IntegrationActivationHealthStatus::Empty,
        };

        for milestone in milestones {
            summary.total_milestones += 1;
            for integration_id in &milestone.dashboard_card.integration_ids {
                integration_ids.insert(integration_id.clone());
            }

            match milestone.dashboard_card.health_status {
                IntegrationActivationHealthStatus::Ready => summary.ready_milestones += 1,
                IntegrationActivationHealthStatus::NeedsReview => summary.review_milestones += 1,
                IntegrationActivationHealthStatus::Blocked => summary.blocked_milestones += 1,
                IntegrationActivationHealthStatus::Empty => summary.empty_milestones += 1,
            }
            match milestone.milestone_kind {
                Some(IntegrationActivationBriefingItemKind::Blocker) => {
                    summary.blocker_milestones += 1
                }
                Some(IntegrationActivationBriefingItemKind::Review) => {
                    summary.review_queue_milestones += 1
                }
                Some(IntegrationActivationBriefingItemKind::Approval) => {
                    summary.approval_milestones += 1
                }
                Some(IntegrationActivationBriefingItemKind::Activation) => {
                    summary.activation_milestones += 1
                }
                Some(IntegrationActivationBriefingItemKind::Risk) => summary.risk_milestones += 1,
                Some(IntegrationActivationBriefingItemKind::Dependency) => {
                    summary.dependency_milestones += 1
                }
                None => {}
            }

            let card = &milestone.dashboard_card;
            summary.total_briefing_items += card.briefing_item_count;
            summary.total_actions += card.action_count;
            summary.total_dossiers += card.dossier_count;
            summary.total_evidence += card.evidence_count;
            summary.total_risks += card.risk_count;
            summary.total_dependency_edges += card.dependency_edge_count;
            summary.blocking_dependency_edges += card.blocking_dependency_edge_count;
            summary.highest_policy_tier = summary.highest_policy_tier.max(card.highest_policy_tier);

            if milestone.requires_attention() {
                summary.milestones_requiring_attention += 1;
                summary.first_attention_sequence = summary
                    .first_attention_sequence
                    .or(Some(milestone.sequence));
                summary.first_attention_priority = min_optional_priority(
                    summary.first_attention_priority,
                    Some(milestone.priority),
                );
            }
            if milestone.has_activation_work() {
                summary.milestones_with_activation_work += 1;
                summary.first_activation_sequence = summary
                    .first_activation_sequence
                    .or(Some(milestone.sequence));
                summary.first_activation_priority = min_optional_priority(
                    summary.first_activation_priority,
                    Some(milestone.priority),
                );
            }
            if milestone.has_approval_ready_work() {
                summary.milestones_with_approval_work += 1;
                summary.first_approval_sequence =
                    summary.first_approval_sequence.or(Some(milestone.sequence));
                summary.first_approval_priority = min_optional_priority(
                    summary.first_approval_priority,
                    Some(milestone.priority),
                );
            }
            if milestone.has_review_work() {
                summary.milestones_with_review_work += 1;
                summary.first_review_sequence =
                    summary.first_review_sequence.or(Some(milestone.sequence));
                summary.first_review_priority =
                    min_optional_priority(summary.first_review_priority, Some(milestone.priority));
            }
            if milestone.has_blockers() {
                summary.milestones_with_blockers += 1;
                summary.first_blocked_sequence =
                    summary.first_blocked_sequence.or(Some(milestone.sequence));
                summary.first_blocked_priority =
                    min_optional_priority(summary.first_blocked_priority, Some(milestone.priority));
            }
            if milestone.has_risks() {
                summary.milestones_with_risks += 1;
                summary.first_risk_sequence =
                    summary.first_risk_sequence.or(Some(milestone.sequence));
                summary.first_risk_priority =
                    min_optional_priority(summary.first_risk_priority, Some(milestone.priority));
            }
            if milestone.has_dependency_blockers() {
                summary.milestones_with_dependency_blockers += 1;
                summary.first_dependency_sequence = summary
                    .first_dependency_sequence
                    .or(Some(milestone.sequence));
                summary.first_dependency_priority = min_optional_priority(
                    summary.first_dependency_priority,
                    Some(milestone.priority),
                );
            }
        }

        summary.unique_integrations = integration_ids.len();
        summary.overall_status =
            if summary.blocked_milestones > 0 || summary.milestones_with_blockers > 0 {
                IntegrationActivationHealthStatus::Blocked
            } else if summary.review_milestones > 0
                || summary.milestones_with_review_work > 0
                || summary.milestones_with_approval_work > 0
            {
                IntegrationActivationHealthStatus::NeedsReview
            } else if summary.ready_milestones > 0 || summary.milestones_with_activation_work > 0 {
                IntegrationActivationHealthStatus::Ready
            } else {
                IntegrationActivationHealthStatus::Empty
            };
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_milestones == 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.milestones_with_activation_work > 0 || self.activation_milestones > 0
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.milestones_with_approval_work > 0 || self.approval_milestones > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.milestones_with_review_work > 0 || self.review_queue_milestones > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.milestones_with_blockers > 0 || self.blocker_milestones > 0
    }

    pub fn has_risks(&self) -> bool {
        self.milestones_with_risks > 0 || self.risk_milestones > 0
    }

    pub fn has_dependency_blockers(&self) -> bool {
        self.milestones_with_dependency_blockers > 0 || self.dependency_milestones > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.overall_status.requires_attention()
            || self.has_approval_ready_work()
            || self.has_review_work()
            || self.has_blockers()
            || self.has_risks()
            || self.has_dependency_blockers()
    }
}

impl IntegrationActivationConstraintKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primitive => "primitive",
            Self::Capability => "capability",
            Self::Dependency => "dependency",
            Self::PolicyReview => "policy_review",
        }
    }
}

impl IntegrationActivationConstraint {
    pub fn affected_integration_count(&self) -> usize {
        self.affected_integration_ids.len()
    }
}

impl IntegrationActivationRiskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PolicyTier => "policy_tier",
            Self::PolicySurface => "policy_surface",
        }
    }
}

impl IntegrationActivationRiskItem {
    fn from_candidates(
        kind: IntegrationActivationRiskKind,
        risk_id: String,
        display_name: String,
        required_tier: PrivilegeTier,
        policy_surface: Option<IntegrationPolicySurface>,
        candidates: &[&IntegrationActivationCandidate],
    ) -> Self {
        let mut candidates = candidates.to_vec();
        candidates.sort_by(|left, right| compare_activation_candidates(left, right));

        let highest_priority = candidates
            .iter()
            .map(|candidate| candidate.readiness_report.priority)
            .min()
            .unwrap_or(u8::MAX);
        let integration_ids = candidates
            .iter()
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let activation_ready_integration_ids = candidates
            .iter()
            .filter(|candidate| candidate.activation_ready())
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let ready_to_activate_integration_ids = candidates
            .iter()
            .filter(|candidate| {
                candidate.recommendation
                    == IntegrationActivationCandidateRecommendation::ReadyToActivate
            })
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let review_integration_ids = candidates
            .iter()
            .filter(|candidate| {
                candidate.recommendation
                    == IntegrationActivationCandidateRecommendation::NeedsHumanReview
            })
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let blocked_integration_ids = candidates
            .iter()
            .filter(|candidate| candidate.is_blocked())
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let local_only_integration_ids = candidates
            .iter()
            .filter(|candidate| candidate.readiness_report.local_only)
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let cloud_required_integration_ids = candidates
            .iter()
            .filter(|candidate| candidate.readiness_report.cloud_required)
            .map(|candidate| candidate.readiness_report.requested_integration_id.clone())
            .collect::<Vec<_>>();
        let candidate_summary =
            IntegrationActivationCandidateSummary::from_candidates(candidates.iter().copied());

        Self {
            kind,
            risk_id,
            display_name,
            required_tier,
            policy_surface,
            highest_priority,
            integration_ids,
            activation_ready_integration_ids,
            ready_to_activate_integration_ids,
            review_integration_ids,
            blocked_integration_ids,
            local_only_integration_ids,
            cloud_required_integration_ids,
            candidate_summary,
        }
    }

    pub fn integration_count(&self) -> usize {
        self.integration_ids.len()
    }

    pub fn has_ready_work(&self) -> bool {
        !self.ready_to_activate_integration_ids.is_empty()
    }

    pub fn has_review_work(&self) -> bool {
        self.candidate_summary.has_review_work()
    }

    pub fn has_blockers(&self) -> bool {
        self.candidate_summary.has_blockers()
    }

    pub fn requires_attention(&self) -> bool {
        self.has_review_work() || self.has_blockers()
    }
}

impl IntegrationActivationRiskSummary {
    pub fn from_risks<'a>(
        risks: impl IntoIterator<Item = &'a IntegrationActivationRiskItem>,
    ) -> Self {
        let mut integrations = BTreeSet::new();
        let mut activation_ready_integrations = BTreeSet::new();
        let mut ready_to_activate_integrations = BTreeSet::new();
        let mut review_integrations = BTreeSet::new();
        let mut blocked_integrations = BTreeSet::new();
        let mut local_only_integrations = BTreeSet::new();
        let mut cloud_required_integrations = BTreeSet::new();
        let mut summary = Self {
            total_risks: 0,
            policy_tier_risks: 0,
            policy_surface_risks: 0,
            total_risk_entries: 0,
            unique_integrations: 0,
            activation_ready_integrations: 0,
            ready_to_activate_integrations: 0,
            review_integrations: 0,
            blocked_integrations: 0,
            local_only_integrations: 0,
            cloud_required_integrations: 0,
            read_only_risks: 0,
            low_risk_risks: 0,
            human_approval_risks: 0,
            high_risk_risks: 0,
            first_ready_priority: None,
            first_review_priority: None,
            first_blocked_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for risk in risks {
            summary.total_risks += 1;
            summary.total_risk_entries += risk.integration_count();
            match risk.kind {
                IntegrationActivationRiskKind::PolicyTier => summary.policy_tier_risks += 1,
                IntegrationActivationRiskKind::PolicySurface => summary.policy_surface_risks += 1,
            }
            match risk.required_tier {
                PrivilegeTier::ReadOnly => summary.read_only_risks += 1,
                PrivilegeTier::LowRisk => summary.low_risk_risks += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_risks += 1,
                PrivilegeTier::HighRisk => summary.high_risk_risks += 1,
            }
            for integration_id in &risk.integration_ids {
                integrations.insert(integration_id.clone());
            }
            for integration_id in &risk.activation_ready_integration_ids {
                activation_ready_integrations.insert(integration_id.clone());
            }
            for integration_id in &risk.ready_to_activate_integration_ids {
                ready_to_activate_integrations.insert(integration_id.clone());
            }
            for integration_id in &risk.review_integration_ids {
                review_integrations.insert(integration_id.clone());
            }
            for integration_id in &risk.blocked_integration_ids {
                blocked_integrations.insert(integration_id.clone());
            }
            for integration_id in &risk.local_only_integration_ids {
                local_only_integrations.insert(integration_id.clone());
            }
            for integration_id in &risk.cloud_required_integration_ids {
                cloud_required_integrations.insert(integration_id.clone());
            }
            if risk.has_ready_work() {
                summary.first_ready_priority = min_optional_priority(
                    summary.first_ready_priority,
                    Some(risk.highest_priority),
                );
            }
            if risk.has_review_work() {
                summary.first_review_priority = min_optional_priority(
                    summary.first_review_priority,
                    Some(risk.highest_priority),
                );
            }
            if risk.has_blockers() {
                summary.first_blocked_priority = min_optional_priority(
                    summary.first_blocked_priority,
                    Some(risk.highest_priority),
                );
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(risk.required_tier);
        }

        summary.unique_integrations = integrations.len();
        summary.activation_ready_integrations = activation_ready_integrations.len();
        summary.ready_to_activate_integrations = ready_to_activate_integrations.len();
        summary.review_integrations = review_integrations.len();
        summary.blocked_integrations = blocked_integrations.len();
        summary.local_only_integrations = local_only_integrations.len();
        summary.cloud_required_integrations = cloud_required_integrations.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_risks == 0
    }

    pub fn has_ready_work(&self) -> bool {
        self.ready_to_activate_integrations > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_integrations > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_integrations > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.has_review_work() || self.has_blockers()
    }
}

impl IntegrationActivationReviewItem {
    fn from_candidate(
        catalog: &[IntegrationCatalogEntry],
        candidate: &IntegrationActivationCandidate,
    ) -> Option<Self> {
        if !candidate.requires_human_review() {
            return None;
        }

        let report = &candidate.readiness_report;
        let mut policy_surfaces = find_entry(catalog, &report.requested_integration_id)
            .map(IntegrationCatalogEntry::policy_surfaces)
            .unwrap_or_default();
        policy_surfaces.sort();
        policy_surfaces.dedup();
        let required_tier = policy_surfaces
            .iter()
            .fold(report.highest_policy_tier, |tier, surface| {
                tier.max(surface.required_tier())
            });

        Some(Self {
            requested_integration_id: report.requested_integration_id.clone(),
            display_name: report.display_name.clone(),
            priority: report.priority,
            activation_target: report.activation_target.clone(),
            recommendation: candidate.recommendation,
            blocker_count: candidate.blocker_count,
            missing_primitives: report.missing_primitives.clone(),
            missing_capabilities: report.missing_capabilities.clone(),
            missing_dependencies: report.missing_dependencies.clone(),
            policy_surfaces,
            required_tier,
            local_only: report.local_only,
            cloud_required: report.cloud_required,
        })
    }

    pub fn activation_ready(&self) -> bool {
        self.missing_primitives.is_empty()
            && self.missing_capabilities.is_empty()
            && self.missing_dependencies.is_empty()
    }

    pub fn review_ready(&self) -> bool {
        self.activation_ready()
            && self.recommendation == IntegrationActivationCandidateRecommendation::NeedsHumanReview
    }

    pub fn is_blocked(&self) -> bool {
        self.recommendation == IntegrationActivationCandidateRecommendation::BlockedOnPrerequisites
            || !self.activation_ready()
    }

    pub fn has_policy_surfaces(&self) -> bool {
        !self.policy_surfaces.is_empty()
    }

    pub fn has_blockers(&self) -> bool {
        self.is_blocked()
    }

    pub fn requires_attention(&self) -> bool {
        true
    }
}

impl IntegrationActivationReviewSummary {
    pub fn from_reviews<'a>(
        reviews: impl IntoIterator<Item = &'a IntegrationActivationReviewItem>,
    ) -> Self {
        let mut policy_surfaces = BTreeSet::new();
        let mut summary = Self {
            total_reviews: 0,
            review_ready_integrations: 0,
            blocked_review_integrations: 0,
            reviews_missing_primitives: 0,
            reviews_missing_capabilities: 0,
            reviews_missing_dependencies: 0,
            direct_targets: 0,
            delegated_integration_targets: 0,
            delegated_standard_targets: 0,
            local_only_reviews: 0,
            cloud_required_reviews: 0,
            reviews_with_policy_surfaces: 0,
            reviews_without_policy_surfaces: 0,
            unique_policy_surfaces: 0,
            total_blockers: 0,
            read_only_reviews: 0,
            low_risk_reviews: 0,
            human_approval_reviews: 0,
            high_risk_reviews: 0,
            first_review_priority: None,
            first_blocked_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for review in reviews {
            summary.total_reviews += 1;
            if review.review_ready() {
                summary.review_ready_integrations += 1;
                summary.first_review_priority =
                    min_optional_priority(summary.first_review_priority, Some(review.priority));
            }
            if review.is_blocked() {
                summary.blocked_review_integrations += 1;
                summary.first_blocked_priority =
                    min_optional_priority(summary.first_blocked_priority, Some(review.priority));
            }
            if !review.missing_primitives.is_empty() {
                summary.reviews_missing_primitives += 1;
            }
            if !review.missing_capabilities.is_empty() {
                summary.reviews_missing_capabilities += 1;
            }
            if !review.missing_dependencies.is_empty() {
                summary.reviews_missing_dependencies += 1;
            }
            match &review.activation_target {
                IntegrationActivationTarget::Direct => summary.direct_targets += 1,
                IntegrationActivationTarget::DelegatedIntegration(_) => {
                    summary.delegated_integration_targets += 1
                }
                IntegrationActivationTarget::DelegatedStandards(_) => {
                    summary.delegated_standard_targets += 1
                }
            }
            if review.local_only {
                summary.local_only_reviews += 1;
            }
            if review.cloud_required {
                summary.cloud_required_reviews += 1;
            }
            if review.has_policy_surfaces() {
                summary.reviews_with_policy_surfaces += 1;
            } else {
                summary.reviews_without_policy_surfaces += 1;
            }
            for surface in &review.policy_surfaces {
                policy_surfaces.insert(*surface);
            }
            summary.total_blockers += review.blocker_count;
            match review.required_tier {
                PrivilegeTier::ReadOnly => summary.read_only_reviews += 1,
                PrivilegeTier::LowRisk => summary.low_risk_reviews += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_reviews += 1,
                PrivilegeTier::HighRisk => summary.high_risk_reviews += 1,
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(review.required_tier);
        }

        summary.unique_policy_surfaces = policy_surfaces.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_reviews == 0
    }

    pub fn has_review_ready_work(&self) -> bool {
        self.review_ready_integrations > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_review_integrations > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.total_reviews > 0
    }
}

impl IntegrationActivationApprovalPacket {
    fn from_candidate(
        catalog: &[IntegrationCatalogEntry],
        candidate: &IntegrationActivationCandidate,
        enabled_integrations: &[IntegrationId],
    ) -> Option<Self> {
        let review = IntegrationActivationReviewItem::from_candidate(catalog, candidate)?;
        let actions = activation_actions_from_candidates(std::iter::once(candidate));
        let action_summary = IntegrationActivationActionSummary::from_actions(actions.iter());
        let constraints =
            activation_constraints_from_candidates(catalog, std::iter::once(candidate));
        let constraint_summary =
            IntegrationActivationConstraintSummary::from_constraints(constraints.iter());
        let risks = activation_risk_from_candidates(catalog, std::iter::once(candidate));
        let risk_summary = IntegrationActivationRiskSummary::from_risks(risks.iter());
        let dependency_graph = activation_dependency_graph_from_reports(
            catalog,
            std::iter::once(&candidate.readiness_report),
            enabled_integrations,
        );

        Some(Self {
            review,
            actions,
            action_summary,
            constraints,
            constraint_summary,
            risks,
            risk_summary,
            dependency_graph,
        })
    }

    pub fn requested_integration_id(&self) -> &IntegrationId {
        &self.review.requested_integration_id
    }

    pub fn display_name(&self) -> &str {
        &self.review.display_name
    }

    pub fn priority(&self) -> u8 {
        self.review.priority
    }

    pub fn required_tier(&self) -> PrivilegeTier {
        self.review.required_tier
    }

    pub fn approval_ready(&self) -> bool {
        self.review.review_ready()
    }

    pub fn has_blockers(&self) -> bool {
        self.review.has_blockers()
            || self.constraint_summary.has_blockers()
            || self.dependency_graph.has_blocking_dependencies()
    }

    pub fn has_policy_surfaces(&self) -> bool {
        self.review.has_policy_surfaces()
    }

    pub fn requires_attention(&self) -> bool {
        true
    }
}

impl IntegrationActivationApprovalSummary {
    pub fn from_packets<'a>(
        packets: impl IntoIterator<Item = &'a IntegrationActivationApprovalPacket>,
    ) -> Self {
        let mut policy_surfaces = BTreeSet::new();
        let mut summary = Self {
            total_packets: 0,
            approval_ready_packets: 0,
            blocked_packets: 0,
            local_only_packets: 0,
            cloud_required_packets: 0,
            packets_with_policy_surfaces: 0,
            packets_without_policy_surfaces: 0,
            unique_policy_surfaces: 0,
            total_actions: 0,
            activate_integration_actions: 0,
            review_policy_actions: 0,
            provide_primitive_actions: 0,
            grant_capability_actions: 0,
            enable_dependency_actions: 0,
            total_constraints: 0,
            blocking_constraints: 0,
            review_constraints: 0,
            total_risks: 0,
            policy_tier_risks: 0,
            policy_surface_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            read_only_packets: 0,
            low_risk_packets: 0,
            human_approval_packets: 0,
            high_risk_packets: 0,
            first_approval_priority: None,
            first_blocked_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for packet in packets {
            summary.total_packets += 1;
            if packet.approval_ready() {
                summary.approval_ready_packets += 1;
                summary.first_approval_priority =
                    min_optional_priority(summary.first_approval_priority, Some(packet.priority()));
            }
            if packet.has_blockers() {
                summary.blocked_packets += 1;
                summary.first_blocked_priority =
                    min_optional_priority(summary.first_blocked_priority, Some(packet.priority()));
            }
            if packet.review.local_only {
                summary.local_only_packets += 1;
            }
            if packet.review.cloud_required {
                summary.cloud_required_packets += 1;
            }
            if packet.has_policy_surfaces() {
                summary.packets_with_policy_surfaces += 1;
            } else {
                summary.packets_without_policy_surfaces += 1;
            }
            for surface in &packet.review.policy_surfaces {
                policy_surfaces.insert(*surface);
            }

            summary.total_actions += packet.action_summary.total_actions;
            summary.activate_integration_actions +=
                packet.action_summary.activate_integration_actions;
            summary.review_policy_actions += packet.action_summary.review_policy_actions;
            summary.provide_primitive_actions += packet.action_summary.provide_primitive_actions;
            summary.grant_capability_actions += packet.action_summary.grant_capability_actions;
            summary.enable_dependency_actions += packet.action_summary.enable_dependency_actions;

            summary.total_constraints += packet.constraint_summary.total_constraints;
            summary.blocking_constraints += packet.constraint_summary.blocking_constraints;
            summary.review_constraints += packet.constraint_summary.review_constraints;

            summary.total_risks += packet.risk_summary.total_risks;
            summary.policy_tier_risks += packet.risk_summary.policy_tier_risks;
            summary.policy_surface_risks += packet.risk_summary.policy_surface_risks;

            summary.total_dependency_edges += packet.dependency_graph.summary.total_edges;
            summary.blocking_dependency_edges += packet.dependency_graph.summary.blocking_edges;

            match packet.required_tier() {
                PrivilegeTier::ReadOnly => summary.read_only_packets += 1,
                PrivilegeTier::LowRisk => summary.low_risk_packets += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_packets += 1,
                PrivilegeTier::HighRisk => summary.high_risk_packets += 1,
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(packet.required_tier());
        }

        summary.unique_policy_surfaces = policy_surfaces.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_packets == 0
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.approval_ready_packets > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_packets > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.total_packets > 0
    }
}

impl IntegrationActivationDecisionItem {
    fn from_packet(packet: IntegrationActivationApprovalPacket) -> Self {
        let decision_status = if packet.approval_ready() && !packet.has_blockers() {
            IntegrationActivationDecisionStatus::ReadyToApprove
        } else {
            IntegrationActivationDecisionStatus::BlockedOnPrerequisites
        };

        Self {
            packet,
            decision_status,
        }
    }

    pub fn requested_integration_id(&self) -> &IntegrationId {
        self.packet.requested_integration_id()
    }

    pub fn display_name(&self) -> &str {
        self.packet.display_name()
    }

    pub fn priority(&self) -> u8 {
        self.packet.priority()
    }

    pub fn required_tier(&self) -> PrivilegeTier {
        self.packet.required_tier()
    }

    pub fn approval_ready(&self) -> bool {
        self.packet.approval_ready()
    }

    pub fn has_blockers(&self) -> bool {
        self.packet.has_blockers()
    }

    pub fn has_policy_surfaces(&self) -> bool {
        self.packet.has_policy_surfaces()
    }

    pub fn requires_attention(&self) -> bool {
        self.decision_status.requires_attention()
    }
}

impl IntegrationActivationDecisionSummary {
    pub fn from_decisions<'a>(
        decisions: impl IntoIterator<Item = &'a IntegrationActivationDecisionItem>,
    ) -> Self {
        let mut policy_surfaces = BTreeSet::new();
        let mut summary = Self {
            total_decisions: 0,
            ready_to_approve_decisions: 0,
            blocked_decisions: 0,
            local_only_decisions: 0,
            cloud_required_decisions: 0,
            decisions_with_policy_surfaces: 0,
            decisions_without_policy_surfaces: 0,
            unique_policy_surfaces: 0,
            total_actions: 0,
            activate_integration_actions: 0,
            review_policy_actions: 0,
            provide_primitive_actions: 0,
            grant_capability_actions: 0,
            enable_dependency_actions: 0,
            total_constraints: 0,
            blocking_constraints: 0,
            review_constraints: 0,
            total_risks: 0,
            policy_tier_risks: 0,
            policy_surface_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            read_only_decisions: 0,
            low_risk_decisions: 0,
            human_approval_decisions: 0,
            high_risk_decisions: 0,
            first_approval_priority: None,
            first_blocked_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for decision in decisions {
            summary.total_decisions += 1;
            match decision.decision_status {
                IntegrationActivationDecisionStatus::ReadyToApprove => {
                    summary.ready_to_approve_decisions += 1;
                    summary.first_approval_priority = min_optional_priority(
                        summary.first_approval_priority,
                        Some(decision.priority()),
                    );
                }
                IntegrationActivationDecisionStatus::BlockedOnPrerequisites => {
                    summary.blocked_decisions += 1;
                    summary.first_blocked_priority = min_optional_priority(
                        summary.first_blocked_priority,
                        Some(decision.priority()),
                    );
                }
            }

            if decision.packet.review.local_only {
                summary.local_only_decisions += 1;
            }
            if decision.packet.review.cloud_required {
                summary.cloud_required_decisions += 1;
            }
            if decision.has_policy_surfaces() {
                summary.decisions_with_policy_surfaces += 1;
            } else {
                summary.decisions_without_policy_surfaces += 1;
            }
            for surface in &decision.packet.review.policy_surfaces {
                policy_surfaces.insert(*surface);
            }

            summary.total_actions += decision.packet.action_summary.total_actions;
            summary.activate_integration_actions +=
                decision.packet.action_summary.activate_integration_actions;
            summary.review_policy_actions += decision.packet.action_summary.review_policy_actions;
            summary.provide_primitive_actions +=
                decision.packet.action_summary.provide_primitive_actions;
            summary.grant_capability_actions +=
                decision.packet.action_summary.grant_capability_actions;
            summary.enable_dependency_actions +=
                decision.packet.action_summary.enable_dependency_actions;

            summary.total_constraints += decision.packet.constraint_summary.total_constraints;
            summary.blocking_constraints += decision.packet.constraint_summary.blocking_constraints;
            summary.review_constraints += decision.packet.constraint_summary.review_constraints;

            summary.total_risks += decision.packet.risk_summary.total_risks;
            summary.policy_tier_risks += decision.packet.risk_summary.policy_tier_risks;
            summary.policy_surface_risks += decision.packet.risk_summary.policy_surface_risks;

            summary.total_dependency_edges += decision.packet.dependency_graph.summary.total_edges;
            summary.blocking_dependency_edges +=
                decision.packet.dependency_graph.summary.blocking_edges;

            match decision.required_tier() {
                PrivilegeTier::ReadOnly => summary.read_only_decisions += 1,
                PrivilegeTier::LowRisk => summary.low_risk_decisions += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_decisions += 1,
                PrivilegeTier::HighRisk => summary.high_risk_decisions += 1,
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(decision.required_tier());
        }

        summary.unique_policy_surfaces = policy_surfaces.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_decisions == 0
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.ready_to_approve_decisions > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_decisions > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.total_decisions > 0
    }
}

impl IntegrationActivationEvidenceItem {
    fn from_decision(
        decision: &IntegrationActivationDecisionItem,
    ) -> Vec<IntegrationActivationEvidenceItem> {
        let packet = &decision.packet;
        let mut evidence = Vec::new();
        let approval_status =
            if decision.decision_status == IntegrationActivationDecisionStatus::ReadyToApprove {
                IntegrationActivationEvidenceStatus::SupportsApproval
            } else {
                IntegrationActivationEvidenceStatus::BlocksApproval
            };

        evidence.push(Self::for_decision(
            decision,
            IntegrationActivationEvidenceKind::ApprovalDecision,
            approval_status,
            decision.decision_status.as_str().to_string(),
        ));

        if packet.action_summary.review_policy_actions > 0 || packet.has_policy_surfaces() {
            if packet.review.policy_surfaces.is_empty() {
                evidence.push(Self::for_decision(
                    decision,
                    IntegrationActivationEvidenceKind::PolicyReview,
                    IntegrationActivationEvidenceStatus::RequiresReview,
                    "policy_review".to_string(),
                ));
            } else {
                for surface in &packet.review.policy_surfaces {
                    let mut row = Self::for_decision(
                        decision,
                        IntegrationActivationEvidenceKind::PolicyReview,
                        IntegrationActivationEvidenceStatus::RequiresReview,
                        surface.as_str().to_string(),
                    );
                    row.policy_surface = Some(*surface);
                    row.required_tier = row.required_tier.max(surface.required_tier());
                    evidence.push(row);
                }
            }
        }

        for primitive in &packet.review.missing_primitives {
            let mut row = Self::for_decision(
                decision,
                IntegrationActivationEvidenceKind::PrimitiveBlocker,
                IntegrationActivationEvidenceStatus::BlocksApproval,
                primitive.as_str().to_string(),
            );
            row.primitive = Some(*primitive);
            evidence.push(row);
        }

        for capability_id in &packet.review.missing_capabilities {
            let mut row = Self::for_decision(
                decision,
                IntegrationActivationEvidenceKind::CapabilityBlocker,
                IntegrationActivationEvidenceStatus::BlocksApproval,
                capability_id.as_str().to_string(),
            );
            row.capability_id = Some(capability_id.clone());
            evidence.push(row);
        }

        for dependency_id in &packet.review.missing_dependencies {
            let mut row = Self::for_decision(
                decision,
                IntegrationActivationEvidenceKind::DependencyBlocker,
                IntegrationActivationEvidenceStatus::BlocksApproval,
                dependency_id.as_str().to_string(),
            );
            row.dependency_integration_id = Some(dependency_id.clone());
            evidence.push(row);
        }

        for risk in &packet.risks {
            let mut row = Self::for_decision(
                decision,
                IntegrationActivationEvidenceKind::PolicyRisk,
                IntegrationActivationEvidenceStatus::RequiresReview,
                risk.risk_id.clone(),
            );
            row.policy_surface = risk.policy_surface;
            row.required_tier = row.required_tier.max(risk.required_tier);
            evidence.push(row);
        }

        for edge in &packet.dependency_graph.edges {
            let mut row = Self::for_decision(
                decision,
                IntegrationActivationEvidenceKind::DependencyEdge,
                if edge.blocks_activation {
                    IntegrationActivationEvidenceStatus::BlocksApproval
                } else {
                    IntegrationActivationEvidenceStatus::SupportsApproval
                },
                format!(
                    "{}->{}",
                    edge.dependency_integration_id.as_str(),
                    edge.dependent_integration_id.as_str()
                ),
            );
            row.dependency_integration_id = Some(edge.dependency_integration_id.clone());
            evidence.push(row);
        }

        evidence
    }

    fn for_decision(
        decision: &IntegrationActivationDecisionItem,
        kind: IntegrationActivationEvidenceKind,
        status: IntegrationActivationEvidenceStatus,
        detail_id: String,
    ) -> Self {
        Self {
            kind,
            status,
            decision_status: decision.decision_status,
            requested_integration_id: decision.requested_integration_id().clone(),
            display_name: decision.display_name().to_string(),
            priority: decision.priority(),
            detail_id,
            primitive: None,
            capability_id: None,
            dependency_integration_id: None,
            policy_surface: None,
            required_tier: decision.required_tier(),
            local_only: decision.packet.review.local_only,
            cloud_required: decision.packet.review.cloud_required,
        }
    }

    pub fn blocks_approval(&self) -> bool {
        self.status == IntegrationActivationEvidenceStatus::BlocksApproval
    }

    pub fn requires_attention(&self) -> bool {
        self.status.requires_attention()
    }
}

impl IntegrationActivationEvidenceSummary {
    pub fn from_evidence<'a>(
        evidence: impl IntoIterator<Item = &'a IntegrationActivationEvidenceItem>,
    ) -> Self {
        let mut integration_ids = BTreeSet::new();
        let mut ready_integration_ids = BTreeSet::new();
        let mut blocked_integration_ids = BTreeSet::new();
        let mut local_only_ids = BTreeSet::new();
        let mut cloud_required_ids = BTreeSet::new();
        let mut policy_surfaces = BTreeSet::new();
        let mut summary = Self {
            total_evidence: 0,
            approval_decision_evidence: 0,
            policy_review_evidence: 0,
            primitive_blocker_evidence: 0,
            capability_blocker_evidence: 0,
            dependency_blocker_evidence: 0,
            policy_risk_evidence: 0,
            dependency_edge_evidence: 0,
            supporting_evidence: 0,
            review_evidence: 0,
            blocking_evidence: 0,
            unique_integrations: 0,
            ready_to_approve_integrations: 0,
            blocked_integrations: 0,
            local_only_integrations: 0,
            cloud_required_integrations: 0,
            unique_policy_surfaces: 0,
            read_only_evidence: 0,
            low_risk_evidence: 0,
            human_approval_evidence: 0,
            high_risk_evidence: 0,
            first_supporting_priority: None,
            first_review_priority: None,
            first_blocking_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for item in evidence {
            summary.total_evidence += 1;
            integration_ids.insert(item.requested_integration_id.clone());
            match item.decision_status {
                IntegrationActivationDecisionStatus::ReadyToApprove => {
                    ready_integration_ids.insert(item.requested_integration_id.clone());
                }
                IntegrationActivationDecisionStatus::BlockedOnPrerequisites => {
                    blocked_integration_ids.insert(item.requested_integration_id.clone());
                }
            }
            if item.local_only {
                local_only_ids.insert(item.requested_integration_id.clone());
            }
            if item.cloud_required {
                cloud_required_ids.insert(item.requested_integration_id.clone());
            }
            if let Some(surface) = item.policy_surface {
                policy_surfaces.insert(surface);
            }

            match item.kind {
                IntegrationActivationEvidenceKind::ApprovalDecision => {
                    summary.approval_decision_evidence += 1;
                }
                IntegrationActivationEvidenceKind::PolicyReview => {
                    summary.policy_review_evidence += 1;
                }
                IntegrationActivationEvidenceKind::PrimitiveBlocker => {
                    summary.primitive_blocker_evidence += 1;
                }
                IntegrationActivationEvidenceKind::CapabilityBlocker => {
                    summary.capability_blocker_evidence += 1;
                }
                IntegrationActivationEvidenceKind::DependencyBlocker => {
                    summary.dependency_blocker_evidence += 1;
                }
                IntegrationActivationEvidenceKind::PolicyRisk => {
                    summary.policy_risk_evidence += 1;
                }
                IntegrationActivationEvidenceKind::DependencyEdge => {
                    summary.dependency_edge_evidence += 1;
                }
            }

            match item.status {
                IntegrationActivationEvidenceStatus::SupportsApproval => {
                    summary.supporting_evidence += 1;
                    summary.first_supporting_priority = min_optional_priority(
                        summary.first_supporting_priority,
                        Some(item.priority),
                    );
                }
                IntegrationActivationEvidenceStatus::RequiresReview => {
                    summary.review_evidence += 1;
                    summary.first_review_priority =
                        min_optional_priority(summary.first_review_priority, Some(item.priority));
                }
                IntegrationActivationEvidenceStatus::BlocksApproval => {
                    summary.blocking_evidence += 1;
                    summary.first_blocking_priority =
                        min_optional_priority(summary.first_blocking_priority, Some(item.priority));
                }
            }

            match item.required_tier {
                PrivilegeTier::ReadOnly => summary.read_only_evidence += 1,
                PrivilegeTier::LowRisk => summary.low_risk_evidence += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_evidence += 1,
                PrivilegeTier::HighRisk => summary.high_risk_evidence += 1,
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(item.required_tier);
        }

        summary.unique_integrations = integration_ids.len();
        summary.ready_to_approve_integrations = ready_integration_ids.len();
        summary.blocked_integrations = blocked_integration_ids.len();
        summary.local_only_integrations = local_only_ids.len();
        summary.cloud_required_integrations = cloud_required_ids.len();
        summary.unique_policy_surfaces = policy_surfaces.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_evidence == 0
    }

    pub fn has_supporting_evidence(&self) -> bool {
        self.supporting_evidence > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_evidence > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocking_evidence > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.has_review_work() || self.has_blockers()
    }
}

impl IntegrationActivationDossierItem {
    fn from_decision(decision: IntegrationActivationDecisionItem) -> Self {
        let mut evidence = IntegrationActivationEvidenceItem::from_decision(&decision);
        evidence.sort_by(compare_activation_evidence);
        let evidence_summary = IntegrationActivationEvidenceSummary::from_evidence(evidence.iter());

        Self {
            decision,
            evidence,
            evidence_summary,
        }
    }

    pub fn requested_integration_id(&self) -> &IntegrationId {
        self.decision.requested_integration_id()
    }

    pub fn display_name(&self) -> &str {
        self.decision.display_name()
    }

    pub fn priority(&self) -> u8 {
        self.decision.priority()
    }

    pub fn required_tier(&self) -> PrivilegeTier {
        self.decision.required_tier()
    }

    pub fn approval_ready(&self) -> bool {
        self.decision.approval_ready()
    }

    pub fn has_blockers(&self) -> bool {
        self.decision.has_blockers() || self.evidence_summary.has_blockers()
    }

    pub fn has_review_work(&self) -> bool {
        self.evidence_summary.has_review_work()
    }

    pub fn has_policy_surfaces(&self) -> bool {
        self.decision.has_policy_surfaces()
    }

    pub fn requires_attention(&self) -> bool {
        self.has_review_work() || self.has_blockers()
    }
}

impl IntegrationActivationDossierSummary {
    pub fn from_dossiers<'a>(
        dossiers: impl IntoIterator<Item = &'a IntegrationActivationDossierItem>,
    ) -> Self {
        let mut policy_surfaces = BTreeSet::new();
        let mut summary = Self {
            total_dossiers: 0,
            ready_to_approve_dossiers: 0,
            blocked_dossiers: 0,
            local_only_dossiers: 0,
            cloud_required_dossiers: 0,
            dossiers_with_policy_surfaces: 0,
            dossiers_without_policy_surfaces: 0,
            unique_policy_surfaces: 0,
            total_actions: 0,
            total_constraints: 0,
            total_risks: 0,
            total_dependency_edges: 0,
            blocking_dependency_edges: 0,
            total_evidence: 0,
            supporting_evidence: 0,
            review_evidence: 0,
            blocking_evidence: 0,
            read_only_dossiers: 0,
            low_risk_dossiers: 0,
            human_approval_dossiers: 0,
            high_risk_dossiers: 0,
            first_approval_priority: None,
            first_blocked_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for dossier in dossiers {
            summary.total_dossiers += 1;
            match dossier.decision.decision_status {
                IntegrationActivationDecisionStatus::ReadyToApprove => {
                    summary.ready_to_approve_dossiers += 1;
                    summary.first_approval_priority = min_optional_priority(
                        summary.first_approval_priority,
                        Some(dossier.priority()),
                    );
                }
                IntegrationActivationDecisionStatus::BlockedOnPrerequisites => {
                    summary.blocked_dossiers += 1;
                    summary.first_blocked_priority = min_optional_priority(
                        summary.first_blocked_priority,
                        Some(dossier.priority()),
                    );
                }
            }

            if dossier.decision.packet.review.local_only {
                summary.local_only_dossiers += 1;
            }
            if dossier.decision.packet.review.cloud_required {
                summary.cloud_required_dossiers += 1;
            }
            if dossier.has_policy_surfaces() {
                summary.dossiers_with_policy_surfaces += 1;
            } else {
                summary.dossiers_without_policy_surfaces += 1;
            }
            for surface in &dossier.decision.packet.review.policy_surfaces {
                policy_surfaces.insert(*surface);
            }

            summary.total_actions += dossier.decision.packet.action_summary.total_actions;
            summary.total_constraints +=
                dossier.decision.packet.constraint_summary.total_constraints;
            summary.total_risks += dossier.decision.packet.risk_summary.total_risks;
            summary.total_dependency_edges +=
                dossier.decision.packet.dependency_graph.summary.total_edges;
            summary.blocking_dependency_edges += dossier
                .decision
                .packet
                .dependency_graph
                .summary
                .blocking_edges;

            summary.total_evidence += dossier.evidence_summary.total_evidence;
            summary.supporting_evidence += dossier.evidence_summary.supporting_evidence;
            summary.review_evidence += dossier.evidence_summary.review_evidence;
            summary.blocking_evidence += dossier.evidence_summary.blocking_evidence;

            match dossier.required_tier() {
                PrivilegeTier::ReadOnly => summary.read_only_dossiers += 1,
                PrivilegeTier::LowRisk => summary.low_risk_dossiers += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_dossiers += 1,
                PrivilegeTier::HighRisk => summary.high_risk_dossiers += 1,
            }
            summary.highest_policy_tier = summary.highest_policy_tier.max(dossier.required_tier());
        }

        summary.unique_policy_surfaces = policy_surfaces.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_dossiers == 0
    }

    pub fn has_approval_ready_work(&self) -> bool {
        self.ready_to_approve_dossiers > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_evidence > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_dossiers > 0 || self.blocking_evidence > 0
    }

    pub fn requires_attention(&self) -> bool {
        self.has_review_work() || self.has_blockers()
    }
}

impl IntegrationActivationConstraintSummary {
    pub fn from_constraints<'a>(
        constraints: impl IntoIterator<Item = &'a IntegrationActivationConstraint>,
    ) -> Self {
        let mut affected_integrations = BTreeSet::new();
        let mut summary = Self {
            total_constraints: 0,
            blocking_constraints: 0,
            review_constraints: 0,
            primitive_constraints: 0,
            capability_constraints: 0,
            dependency_constraints: 0,
            policy_review_constraints: 0,
            affected_integrations: 0,
            first_blocking_priority: None,
            first_review_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for constraint in constraints {
            summary.total_constraints += 1;
            if constraint.blocks_activation {
                summary.blocking_constraints += 1;
                summary.first_blocking_priority = min_optional_priority(
                    summary.first_blocking_priority,
                    Some(constraint.highest_priority),
                );
            }
            if constraint.requires_human_review {
                summary.review_constraints += 1;
                summary.first_review_priority = min_optional_priority(
                    summary.first_review_priority,
                    Some(constraint.highest_priority),
                );
            }
            match constraint.kind {
                IntegrationActivationConstraintKind::Primitive => {
                    summary.primitive_constraints += 1
                }
                IntegrationActivationConstraintKind::Capability => {
                    summary.capability_constraints += 1
                }
                IntegrationActivationConstraintKind::Dependency => {
                    summary.dependency_constraints += 1
                }
                IntegrationActivationConstraintKind::PolicyReview => {
                    summary.policy_review_constraints += 1
                }
            }
            for integration_id in &constraint.affected_integration_ids {
                affected_integrations.insert(integration_id.clone());
            }
            summary.highest_policy_tier = summary
                .highest_policy_tier
                .max(constraint.highest_policy_tier);
        }

        summary.affected_integrations = affected_integrations.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_constraints == 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocking_constraints > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_constraints > 0
    }
}

impl IntegrationReadinessReport {
    pub fn activation_ready(&self) -> bool {
        self.missing_primitives.is_empty()
            && self.missing_capabilities.is_empty()
            && self.missing_dependencies.is_empty()
    }

    pub fn is_blocked(&self) -> bool {
        !self.activation_ready()
    }

    pub fn missing_prerequisite_count(&self) -> usize {
        self.missing_primitives.len()
            + self.missing_capabilities.len()
            + self.missing_dependencies.len()
    }

    pub fn missing_primitive(&self, primitive: PrimitiveFamily) -> bool {
        self.missing_primitives.contains(&primitive)
    }

    pub fn missing_capability(&self, capability_id: &CapabilityId) -> bool {
        self.missing_capabilities
            .iter()
            .any(|candidate| candidate == capability_id)
    }

    pub fn missing_dependency(&self, integration_id: &IntegrationId) -> bool {
        self.missing_dependencies
            .iter()
            .any(|candidate| candidate == integration_id)
    }

    pub fn delegates_to_integration(&self, integration_id: &IntegrationId) -> bool {
        matches!(
            &self.activation_target,
            IntegrationActivationTarget::DelegatedIntegration(target) if target == integration_id
        )
    }
}

impl IntegrationActivationCandidateRecommendation {
    fn from_report(report: &IntegrationReadinessReport) -> Self {
        if report.is_blocked() {
            Self::BlockedOnPrerequisites
        } else if report.requires_human_review {
            Self::NeedsHumanReview
        } else {
            Self::ReadyToActivate
        }
    }

    pub fn is_actionable(self) -> bool {
        matches!(self, Self::ReadyToActivate | Self::NeedsHumanReview)
    }
}

impl IntegrationActivationCandidate {
    pub fn from_report(report: IntegrationReadinessReport) -> Self {
        let blocker_count = report.missing_prerequisite_count();
        let recommendation = IntegrationActivationCandidateRecommendation::from_report(&report);
        Self {
            readiness_report: report,
            recommendation,
            blocker_count,
        }
    }

    pub fn activation_ready(&self) -> bool {
        self.readiness_report.activation_ready()
    }

    pub fn is_actionable(&self) -> bool {
        self.recommendation.is_actionable()
    }

    pub fn is_blocked(&self) -> bool {
        self.recommendation == IntegrationActivationCandidateRecommendation::BlockedOnPrerequisites
    }

    pub fn requires_human_review(&self) -> bool {
        self.readiness_report.requires_human_review
    }
}

impl IntegrationActivationCandidateSummary {
    pub fn from_candidates<'a>(
        candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
    ) -> Self {
        let mut summary = Self {
            total_candidates: 0,
            ready_to_activate_candidates: 0,
            needs_human_review_candidates: 0,
            blocked_candidates: 0,
            activation_ready_candidates: 0,
            candidates_requiring_human_review: 0,
            candidates_missing_primitives: 0,
            candidates_missing_capabilities: 0,
            candidates_missing_dependencies: 0,
            direct_targets: 0,
            delegated_integration_targets: 0,
            delegated_standard_targets: 0,
            local_only_candidates: 0,
            cloud_required_candidates: 0,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for candidate in candidates {
            summary.total_candidates += 1;
            match candidate.recommendation {
                IntegrationActivationCandidateRecommendation::ReadyToActivate => {
                    summary.ready_to_activate_candidates += 1
                }
                IntegrationActivationCandidateRecommendation::NeedsHumanReview => {
                    summary.needs_human_review_candidates += 1
                }
                IntegrationActivationCandidateRecommendation::BlockedOnPrerequisites => {
                    summary.blocked_candidates += 1
                }
            }
            if candidate.activation_ready() {
                summary.activation_ready_candidates += 1;
            }
            if candidate.requires_human_review() {
                summary.candidates_requiring_human_review += 1;
            }
            if !candidate.readiness_report.missing_primitives.is_empty() {
                summary.candidates_missing_primitives += 1;
            }
            if !candidate.readiness_report.missing_capabilities.is_empty() {
                summary.candidates_missing_capabilities += 1;
            }
            if !candidate.readiness_report.missing_dependencies.is_empty() {
                summary.candidates_missing_dependencies += 1;
            }
            match &candidate.readiness_report.activation_target {
                IntegrationActivationTarget::Direct => summary.direct_targets += 1,
                IntegrationActivationTarget::DelegatedIntegration(_) => {
                    summary.delegated_integration_targets += 1
                }
                IntegrationActivationTarget::DelegatedStandards(_) => {
                    summary.delegated_standard_targets += 1
                }
            }
            if candidate.readiness_report.local_only {
                summary.local_only_candidates += 1;
            }
            if candidate.readiness_report.cloud_required {
                summary.cloud_required_candidates += 1;
            }
            summary.highest_policy_tier = summary
                .highest_policy_tier
                .max(candidate.readiness_report.highest_policy_tier);
        }

        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_candidates == 0
    }

    pub fn has_actionable_candidates(&self) -> bool {
        self.ready_to_activate_candidates > 0 || self.needs_human_review_candidates > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.blocked_candidates > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.needs_human_review_candidates > 0
    }
}

impl IntegrationActivationActionKind {
    fn sort_rank(self) -> u8 {
        match self {
            Self::ActivateIntegration => 0,
            Self::ReviewPolicy => 1,
            Self::ProvidePrimitive => 2,
            Self::GrantCapability => 3,
            Self::EnableDependency => 4,
        }
    }
}

impl IntegrationActivationAction {
    pub fn activate(candidate: &IntegrationActivationCandidate) -> Self {
        Self::from_candidate(
            candidate,
            IntegrationActivationActionKind::ActivateIntegration,
        )
    }

    pub fn review_policy(candidate: &IntegrationActivationCandidate) -> Self {
        Self::from_candidate(candidate, IntegrationActivationActionKind::ReviewPolicy)
    }

    pub fn provide_primitive(
        candidate: &IntegrationActivationCandidate,
        primitive: PrimitiveFamily,
    ) -> Self {
        Self {
            primitive: Some(primitive),
            ..Self::from_candidate(candidate, IntegrationActivationActionKind::ProvidePrimitive)
        }
    }

    pub fn grant_capability(
        candidate: &IntegrationActivationCandidate,
        capability_id: CapabilityId,
    ) -> Self {
        Self {
            capability_id: Some(capability_id),
            ..Self::from_candidate(candidate, IntegrationActivationActionKind::GrantCapability)
        }
    }

    pub fn enable_dependency(
        candidate: &IntegrationActivationCandidate,
        dependency_integration_id: IntegrationId,
    ) -> Self {
        Self {
            dependency_integration_id: Some(dependency_integration_id),
            ..Self::from_candidate(candidate, IntegrationActivationActionKind::EnableDependency)
        }
    }

    pub fn is_activation(&self) -> bool {
        self.kind == IntegrationActivationActionKind::ActivateIntegration
    }

    pub fn blocks_activation(&self) -> bool {
        !self.is_activation()
    }

    fn from_candidate(
        candidate: &IntegrationActivationCandidate,
        kind: IntegrationActivationActionKind,
    ) -> Self {
        Self {
            kind,
            requested_integration_id: candidate.readiness_report.requested_integration_id.clone(),
            display_name: candidate.readiness_report.display_name.clone(),
            priority: candidate.readiness_report.priority,
            recommendation: candidate.recommendation,
            primitive: None,
            capability_id: None,
            dependency_integration_id: None,
            highest_policy_tier: candidate.readiness_report.highest_policy_tier,
        }
    }
}

impl IntegrationActivationActionSummary {
    pub fn from_actions<'a>(
        actions: impl IntoIterator<Item = &'a IntegrationActivationAction>,
    ) -> Self {
        let mut unique_integrations = BTreeSet::new();
        let mut actionable_integrations = BTreeSet::new();
        let mut blocked_integrations = BTreeSet::new();
        let mut summary = Self {
            total_actions: 0,
            activate_integration_actions: 0,
            review_policy_actions: 0,
            provide_primitive_actions: 0,
            grant_capability_actions: 0,
            enable_dependency_actions: 0,
            actionable_integration_count: 0,
            blocked_integration_count: 0,
            unique_integrations: 0,
            first_action_priority: None,
            first_activation_priority: None,
            first_blocker_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
        };

        for action in actions {
            summary.total_actions += 1;
            unique_integrations.insert(action.requested_integration_id.clone());
            summary.first_action_priority = Some(
                summary
                    .first_action_priority
                    .map_or(action.priority, |priority| priority.min(action.priority)),
            );
            summary.highest_policy_tier =
                summary.highest_policy_tier.max(action.highest_policy_tier);
            match action.kind {
                IntegrationActivationActionKind::ActivateIntegration => {
                    summary.activate_integration_actions += 1;
                    actionable_integrations.insert(action.requested_integration_id.clone());
                    summary.first_activation_priority = Some(
                        summary
                            .first_activation_priority
                            .map_or(action.priority, |priority| priority.min(action.priority)),
                    );
                }
                IntegrationActivationActionKind::ReviewPolicy => {
                    summary.review_policy_actions += 1;
                    blocked_integrations.insert(action.requested_integration_id.clone());
                    summary.first_blocker_priority = Some(
                        summary
                            .first_blocker_priority
                            .map_or(action.priority, |priority| priority.min(action.priority)),
                    );
                }
                IntegrationActivationActionKind::ProvidePrimitive => {
                    summary.provide_primitive_actions += 1;
                    blocked_integrations.insert(action.requested_integration_id.clone());
                    summary.first_blocker_priority = Some(
                        summary
                            .first_blocker_priority
                            .map_or(action.priority, |priority| priority.min(action.priority)),
                    );
                }
                IntegrationActivationActionKind::GrantCapability => {
                    summary.grant_capability_actions += 1;
                    blocked_integrations.insert(action.requested_integration_id.clone());
                    summary.first_blocker_priority = Some(
                        summary
                            .first_blocker_priority
                            .map_or(action.priority, |priority| priority.min(action.priority)),
                    );
                }
                IntegrationActivationActionKind::EnableDependency => {
                    summary.enable_dependency_actions += 1;
                    blocked_integrations.insert(action.requested_integration_id.clone());
                    summary.first_blocker_priority = Some(
                        summary
                            .first_blocker_priority
                            .map_or(action.priority, |priority| priority.min(action.priority)),
                    );
                }
            }
        }

        summary.unique_integrations = unique_integrations.len();
        summary.actionable_integration_count = actionable_integrations.len();
        summary.blocked_integration_count = blocked_integrations.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_actions == 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.activate_integration_actions > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.review_policy_actions > 0
            || self.provide_primitive_actions > 0
            || self.grant_capability_actions > 0
            || self.enable_dependency_actions > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.review_policy_actions > 0
    }
}

impl IntegrationActivationAgendaStage {
    pub fn from_candidates(
        priority: u8,
        mut candidates: Vec<IntegrationActivationCandidate>,
    ) -> Self {
        candidates.sort_by(compare_activation_candidates);
        let candidate_summary =
            IntegrationActivationCandidateSummary::from_candidates(candidates.iter());
        let actions = activation_actions_from_candidates(candidates.iter());
        let action_summary = IntegrationActivationActionSummary::from_actions(actions.iter());

        Self {
            priority,
            candidates,
            candidate_summary,
            actions,
            action_summary,
        }
    }

    pub fn has_activation_work(&self) -> bool {
        self.action_summary.has_activation_work()
    }

    pub fn has_blockers(&self) -> bool {
        self.action_summary.has_blockers()
    }

    pub fn has_review_work(&self) -> bool {
        self.action_summary.has_review_work()
    }

    pub fn has_actionable_candidates(&self) -> bool {
        self.candidate_summary.has_actionable_candidates()
    }
}

impl IntegrationActivationAgendaSummary {
    pub fn from_stages<'a>(
        stages: impl IntoIterator<Item = &'a IntegrationActivationAgendaStage>,
    ) -> Self {
        let mut candidates = Vec::new();
        let mut actions = Vec::new();
        let empty_candidates = Vec::<&IntegrationActivationCandidate>::new();
        let empty_actions = Vec::<&IntegrationActivationAction>::new();
        let mut summary = Self {
            total_stages: 0,
            total_candidates: 0,
            total_actions: 0,
            stages_with_activation_work: 0,
            stages_with_blockers: 0,
            stages_with_review_work: 0,
            first_action_priority: None,
            first_activation_priority: None,
            first_blocker_priority: None,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            candidate_summary: IntegrationActivationCandidateSummary::from_candidates(
                empty_candidates,
            ),
            action_summary: IntegrationActivationActionSummary::from_actions(empty_actions),
        };

        for stage in stages {
            summary.total_stages += 1;
            summary.total_candidates += stage.candidates.len();
            summary.total_actions += stage.actions.len();
            if stage.has_activation_work() {
                summary.stages_with_activation_work += 1;
            }
            if stage.has_blockers() {
                summary.stages_with_blockers += 1;
            }
            if stage.has_review_work() {
                summary.stages_with_review_work += 1;
            }
            summary.highest_policy_tier = summary
                .highest_policy_tier
                .max(stage.action_summary.highest_policy_tier)
                .max(stage.candidate_summary.highest_policy_tier);
            summary.first_action_priority = min_optional_priority(
                summary.first_action_priority,
                stage.action_summary.first_action_priority,
            );
            summary.first_activation_priority = min_optional_priority(
                summary.first_activation_priority,
                stage.action_summary.first_activation_priority,
            );
            summary.first_blocker_priority = min_optional_priority(
                summary.first_blocker_priority,
                stage.action_summary.first_blocker_priority,
            );
            candidates.extend(stage.candidates.iter());
            actions.extend(stage.actions.iter());
        }

        summary.candidate_summary =
            IntegrationActivationCandidateSummary::from_candidates(candidates);
        summary.action_summary = IntegrationActivationActionSummary::from_actions(actions);
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_stages == 0
    }

    pub fn has_activation_work(&self) -> bool {
        self.stages_with_activation_work > 0
    }

    pub fn has_blockers(&self) -> bool {
        self.stages_with_blockers > 0
    }

    pub fn has_review_work(&self) -> bool {
        self.stages_with_review_work > 0
    }
}

impl IntegrationActivationRunwayStage {
    pub fn from_candidates(
        priority: u8,
        mut candidates: Vec<IntegrationActivationCandidate>,
    ) -> Self {
        candidates.sort_by(compare_activation_candidates);
        let summary = IntegrationActivationCandidateSummary::from_candidates(candidates.iter());
        Self {
            priority,
            candidates,
            summary,
        }
    }

    pub fn has_actionable_candidates(&self) -> bool {
        self.summary.has_actionable_candidates()
    }

    pub fn has_blockers(&self) -> bool {
        self.summary.has_blockers()
    }

    pub fn has_review_work(&self) -> bool {
        self.summary.has_review_work()
    }
}

impl IntegrationActivationRunwaySummary {
    pub fn from_stages<'a>(
        stages: impl IntoIterator<Item = &'a IntegrationActivationRunwayStage>,
    ) -> Self {
        let stages = stages.into_iter().collect::<Vec<_>>();
        let candidate_summary = IntegrationActivationCandidateSummary::from_candidates(
            stages.iter().flat_map(|stage| stage.candidates.iter()),
        );
        let mut summary = Self {
            total_stages: 0,
            total_candidates: candidate_summary.total_candidates,
            actionable_stages: 0,
            ready_stages: 0,
            review_stages: 0,
            blocked_stages: 0,
            first_actionable_priority: None,
            first_blocked_priority: None,
            next_ready_priority: None,
            highest_policy_tier: candidate_summary.highest_policy_tier,
            candidate_summary,
        };

        for stage in stages {
            summary.total_stages += 1;
            if stage.has_actionable_candidates() {
                summary.actionable_stages += 1;
                if summary.first_actionable_priority.is_none() {
                    summary.first_actionable_priority = Some(stage.priority);
                }
            }
            if stage.summary.ready_to_activate_candidates > 0 {
                summary.ready_stages += 1;
                if summary.next_ready_priority.is_none() {
                    summary.next_ready_priority = Some(stage.priority);
                }
            }
            if stage.has_review_work() {
                summary.review_stages += 1;
            }
            if stage.has_blockers() {
                summary.blocked_stages += 1;
                if summary.first_blocked_priority.is_none() {
                    summary.first_blocked_priority = Some(stage.priority);
                }
            }
        }

        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_stages == 0
    }

    pub fn has_actionable_stage(&self) -> bool {
        self.actionable_stages > 0
    }

    pub fn has_blocked_stage(&self) -> bool {
        self.blocked_stages > 0
    }

    pub fn has_review_stage(&self) -> bool {
        self.review_stages > 0
    }
}

impl IntegrationActivationPlan {
    pub fn requires_human_review(&self) -> bool {
        self.highest_policy_tier >= PrivilegeTier::HumanApproval
    }

    pub fn requires_primitive(&self, primitive: PrimitiveFamily) -> bool {
        self.required_primitives.contains(&primitive)
    }

    pub fn requires_capability(&self, capability_id: &CapabilityId) -> bool {
        self.required_capabilities
            .iter()
            .any(|candidate| candidate == capability_id)
    }

    pub fn delegates_to_integration(&self, integration_id: &IntegrationId) -> bool {
        matches!(
            &self.activation_target,
            IntegrationActivationTarget::DelegatedIntegration(target) if target == integration_id
        )
    }

    pub fn delegates_to_standard(&self, protocol: &ProtocolFamily) -> bool {
        matches!(
            &self.activation_target,
            IntegrationActivationTarget::DelegatedStandards(standards)
                if standards.iter().any(|candidate| candidate == protocol)
        )
    }
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

pub fn ecosystem_platforms_requiring_primitive(
    sources: &[EcosystemSurveySource],
    primitive: PrimitiveFamily,
) -> Vec<EcosystemSurveyPlatform> {
    sources
        .iter()
        .filter(|source| source.requires_primitive(primitive))
        .map(|source| source.platform)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn ecosystem_primitive_coverage(
    sources: &[EcosystemSurveySource],
) -> Vec<EcosystemPrimitiveCoverage> {
    all_primitive_families()
        .iter()
        .copied()
        .map(|primitive| {
            let platforms = ecosystem_platforms_requiring_primitive(sources, primitive);
            EcosystemPrimitiveCoverage {
                primitive,
                source_count: survey_sources_requiring_primitive(sources, primitive).len(),
                platforms,
            }
        })
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

pub fn query_integrations<'a>(
    catalog: &'a [IntegrationCatalogEntry],
    query: &IntegrationCatalogQuery,
) -> Vec<&'a IntegrationCatalogEntry> {
    let mut entries = catalog
        .iter()
        .filter(|entry| query.matches_entry(entry))
        .collect::<Vec<_>>();

    sort_query_results(&mut entries, query.sort);
    if let Some(limit) = query.limit {
        entries.truncate(limit);
    }

    entries
}

pub fn primitive_backlog(catalog: &[IntegrationCatalogEntry]) -> Vec<PrimitiveBacklogItem> {
    primitive_backlog_at_or_before_priority(catalog, u8::MAX)
}

pub fn primitive_backlog_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
) -> Vec<PrimitiveBacklogItem> {
    let mut by_primitive: BTreeMap<PrimitiveFamily, (u8, Vec<IntegrationId>)> = BTreeMap::new();

    for entry in catalog.iter().filter(|entry| entry.priority <= priority) {
        for primitive in &entry.required_primitives {
            let (highest_priority, integration_ids) = by_primitive
                .entry(*primitive)
                .or_insert((entry.priority, Vec::new()));
            *highest_priority = (*highest_priority).min(entry.priority);
            if !integration_ids.contains(&entry.integration_id) {
                integration_ids.push(entry.integration_id.clone());
            }
        }
    }

    let mut backlog = by_primitive
        .into_iter()
        .map(
            |(primitive, (highest_priority, integration_ids))| PrimitiveBacklogItem {
                primitive,
                highest_priority,
                entry_count: integration_ids.len(),
                integration_ids,
            },
        )
        .collect::<Vec<_>>();
    backlog.sort_by(|left, right| {
        left.highest_priority
            .cmp(&right.highest_priority)
            .then_with(|| right.entry_count.cmp(&left.entry_count))
            .then_with(|| left.primitive.cmp(&right.primitive))
    });
    backlog
}

pub fn primitive_backlog_with_ecosystem_coverage(
    catalog: &[IntegrationCatalogEntry],
    sources: &[EcosystemSurveySource],
    priority: u8,
) -> Vec<PrimitiveBacklogCoverageItem> {
    primitive_backlog_at_or_before_priority(catalog, priority)
        .into_iter()
        .map(|item| {
            let platforms = ecosystem_platforms_requiring_primitive(sources, item.primitive);
            PrimitiveBacklogCoverageItem {
                primitive: item.primitive,
                highest_priority: item.highest_priority,
                entry_count: item.entry_count,
                integration_ids: item.integration_ids,
                source_count: survey_sources_requiring_primitive(sources, item.primitive).len(),
                platforms,
            }
        })
        .collect()
}

pub fn ecosystem_platform_coverage(
    catalog: &[IntegrationCatalogEntry],
    sources: &[EcosystemSurveySource],
    priority: u8,
) -> Vec<EcosystemPlatformCoverageItem> {
    let backlog = primitive_backlog_at_or_before_priority(catalog, priority);
    let backlog_primitives = backlog
        .iter()
        .map(|item| item.primitive)
        .collect::<Vec<_>>();
    let mut items = sources
        .iter()
        .map(|source| {
            let covered_backlog_primitives = backlog
                .iter()
                .filter(|item| source.requires_primitive(item.primitive))
                .map(|item| item.primitive)
                .collect::<Vec<_>>();
            let uncovered_backlog_primitives = backlog
                .iter()
                .filter(|item| !source.requires_primitive(item.primitive))
                .map(|item| item.primitive)
                .collect::<Vec<_>>();
            let highest_backlog_priority = backlog
                .iter()
                .filter(|item| source.requires_primitive(item.primitive))
                .map(|item| item.highest_priority)
                .min();
            let backlog_entry_count = backlog
                .iter()
                .filter(|item| source.requires_primitive(item.primitive))
                .map(|item| item.entry_count)
                .sum();

            EcosystemPlatformCoverageItem {
                platform: source.platform,
                display_name: source.display_name,
                source_url: source.source_url,
                source_surface: source.source_surface,
                contributes: source.contributes,
                primitive_hints: source.primitive_hints.clone(),
                backlog_primitives: backlog_primitives.clone(),
                covered_backlog_primitives,
                uncovered_backlog_primitives,
                highest_backlog_priority,
                backlog_entry_count,
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .covered_backlog_primitive_count()
            .cmp(&left.covered_backlog_primitive_count())
            .then_with(|| left.platform.cmp(&right.platform))
    });
    items
}

pub fn policy_surface_inventory(
    catalog: &[IntegrationCatalogEntry],
) -> Vec<IntegrationPolicySurfaceInventoryItem> {
    policy_surface_inventory_at_or_before_priority(catalog, u8::MAX)
}

pub fn policy_surface_inventory_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
) -> Vec<IntegrationPolicySurfaceInventoryItem> {
    #[derive(Default)]
    struct SurfaceAccumulator {
        highest_priority: u8,
        local_entry_count: usize,
        cloud_entry_count: usize,
        human_review_entry_count: usize,
        integration_ids: Vec<IntegrationId>,
    }

    let mut by_surface: BTreeMap<IntegrationPolicySurface, SurfaceAccumulator> = BTreeMap::new();

    for entry in catalog.iter().filter(|entry| entry.priority <= priority) {
        let surfaces = entry.policy_surfaces();
        let local_only = entry_local_only(entry);
        let cloud_required = entry_cloud_required(entry);
        let requires_human_review = entry.highest_policy_tier() >= PrivilegeTier::HumanApproval;

        for surface in surfaces {
            let accumulator = by_surface.entry(surface).or_insert(SurfaceAccumulator {
                highest_priority: entry.priority,
                ..SurfaceAccumulator::default()
            });
            accumulator.highest_priority = accumulator.highest_priority.min(entry.priority);
            if local_only {
                accumulator.local_entry_count += 1;
            }
            if cloud_required {
                accumulator.cloud_entry_count += 1;
            }
            if requires_human_review {
                accumulator.human_review_entry_count += 1;
            }
            if !accumulator.integration_ids.contains(&entry.integration_id) {
                accumulator
                    .integration_ids
                    .push(entry.integration_id.clone());
            }
        }
    }

    let mut inventory = by_surface
        .into_iter()
        .map(
            |(surface, accumulator)| IntegrationPolicySurfaceInventoryItem {
                surface,
                required_tier: surface.required_tier(),
                highest_priority: accumulator.highest_priority,
                entry_count: accumulator.integration_ids.len(),
                local_entry_count: accumulator.local_entry_count,
                cloud_entry_count: accumulator.cloud_entry_count,
                human_review_entry_count: accumulator.human_review_entry_count,
                integration_ids: accumulator.integration_ids,
            },
        )
        .collect::<Vec<_>>();
    inventory.sort_by(|left, right| {
        left.highest_priority
            .cmp(&right.highest_priority)
            .then_with(|| right.required_tier.cmp(&left.required_tier))
            .then_with(|| right.entry_count.cmp(&left.entry_count))
            .then_with(|| left.surface.cmp(&right.surface))
    });
    inventory
}

pub fn activation_plan_for_integration(
    catalog: &[IntegrationCatalogEntry],
    integration_id: &IntegrationId,
) -> Option<IntegrationActivationPlan> {
    find_entry(catalog, integration_id).map(activation_plan_for_entry)
}

pub fn activation_plans_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
) -> Vec<IntegrationActivationPlan> {
    entries_at_or_before_priority(catalog, priority)
        .into_iter()
        .map(activation_plan_for_entry)
        .collect()
}

pub fn readiness_report_for_integration(
    catalog: &[IntegrationCatalogEntry],
    integration_id: &IntegrationId,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Option<IntegrationReadinessReport> {
    activation_plan_for_integration(catalog, integration_id).map(|plan| {
        readiness_report_for_plan(
            &plan,
            available_primitives,
            allowed_capabilities,
            enabled_integrations,
        )
    })
}

pub fn readiness_reports_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationReadinessReport> {
    activation_plans_at_or_before_priority(catalog, priority)
        .into_iter()
        .map(|plan| {
            readiness_report_for_plan(
                &plan,
                available_primitives,
                allowed_capabilities,
                enabled_integrations,
            )
        })
        .collect()
}

pub fn activation_candidates_from_reports<'a>(
    reports: impl IntoIterator<Item = &'a IntegrationReadinessReport>,
) -> Vec<IntegrationActivationCandidate> {
    let mut candidates = reports
        .into_iter()
        .cloned()
        .map(IntegrationActivationCandidate::from_report)
        .collect::<Vec<_>>();

    candidates.sort_by(compare_activation_candidates);
    candidates
}

pub fn activation_candidates_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationCandidate> {
    let reports = readiness_reports_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_candidates_from_reports(reports.iter())
}

pub fn activation_actions_from_candidates<'a>(
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
) -> Vec<IntegrationActivationAction> {
    let mut actions = Vec::new();

    for candidate in candidates {
        if candidate.activation_ready() {
            if candidate.requires_human_review() {
                actions.push(IntegrationActivationAction::review_policy(candidate));
            } else {
                actions.push(IntegrationActivationAction::activate(candidate));
            }
        }

        for primitive in &candidate.readiness_report.missing_primitives {
            actions.push(IntegrationActivationAction::provide_primitive(
                candidate, *primitive,
            ));
        }
        for capability_id in &candidate.readiness_report.missing_capabilities {
            actions.push(IntegrationActivationAction::grant_capability(
                candidate,
                capability_id.clone(),
            ));
        }
        for integration_id in &candidate.readiness_report.missing_dependencies {
            actions.push(IntegrationActivationAction::enable_dependency(
                candidate,
                integration_id.clone(),
            ));
        }
        if candidate.is_blocked() && candidate.requires_human_review() {
            actions.push(IntegrationActivationAction::review_policy(candidate));
        }
    }

    actions.sort_by(compare_activation_actions);
    actions
}

pub fn activation_actions_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationAction> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_actions_from_candidates(candidates.iter())
}

pub fn activation_agenda_from_candidates(
    candidates: Vec<IntegrationActivationCandidate>,
) -> Vec<IntegrationActivationAgendaStage> {
    let mut stages_by_priority: BTreeMap<u8, Vec<IntegrationActivationCandidate>> = BTreeMap::new();
    for candidate in candidates {
        stages_by_priority
            .entry(candidate.readiness_report.priority)
            .or_default()
            .push(candidate);
    }

    stages_by_priority
        .into_iter()
        .map(|(priority, candidates)| {
            IntegrationActivationAgendaStage::from_candidates(priority, candidates)
        })
        .collect()
}

pub fn activation_agenda_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationAgendaStage> {
    activation_agenda_from_candidates(activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    ))
}

pub fn activation_runway_from_candidates(
    candidates: Vec<IntegrationActivationCandidate>,
) -> Vec<IntegrationActivationRunwayStage> {
    let mut stages_by_priority: BTreeMap<u8, Vec<IntegrationActivationCandidate>> = BTreeMap::new();
    for candidate in candidates {
        stages_by_priority
            .entry(candidate.readiness_report.priority)
            .or_default()
            .push(candidate);
    }

    stages_by_priority
        .into_iter()
        .map(|(priority, candidates)| {
            IntegrationActivationRunwayStage::from_candidates(priority, candidates)
        })
        .collect()
}

pub fn activation_runway_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationRunwayStage> {
    activation_runway_from_candidates(activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    ))
}

pub fn activation_health_from_candidates(
    candidates: Vec<IntegrationActivationCandidate>,
) -> Vec<IntegrationActivationHealthStage> {
    let mut stages_by_priority: BTreeMap<u8, Vec<IntegrationActivationCandidate>> = BTreeMap::new();
    for candidate in candidates {
        stages_by_priority
            .entry(candidate.readiness_report.priority)
            .or_default()
            .push(candidate);
    }

    stages_by_priority
        .into_iter()
        .map(|(priority, candidates)| {
            IntegrationActivationHealthStage::from_candidates(priority, candidates)
        })
        .collect()
}

pub fn activation_health_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationHealthStage> {
    activation_health_from_candidates(activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    ))
}

pub fn activation_maintenance_from_candidates(
    catalog: &[IntegrationCatalogEntry],
    candidates: Vec<IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationMaintenanceWindow> {
    let mut windows_by_priority: BTreeMap<u8, Vec<IntegrationActivationCandidate>> =
        BTreeMap::new();
    for candidate in candidates {
        windows_by_priority
            .entry(candidate.readiness_report.priority)
            .or_default()
            .push(candidate);
    }

    windows_by_priority
        .into_iter()
        .map(|(priority, candidates)| {
            IntegrationActivationMaintenanceWindow::from_candidates(
                catalog,
                priority,
                candidates,
                enabled_integrations,
            )
        })
        .collect()
}

pub fn activation_maintenance_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationMaintenanceWindow> {
    activation_maintenance_from_candidates(
        catalog,
        activation_candidates_at_or_before_priority(
            catalog,
            priority,
            available_primitives,
            allowed_capabilities,
            enabled_integrations,
        ),
        enabled_integrations,
    )
}

pub fn activation_constraints_from_candidates<'a>(
    catalog: &[IntegrationCatalogEntry],
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
) -> Vec<IntegrationActivationConstraint> {
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    let policy_tier_by_integration = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.readiness_report.requested_integration_id.clone(),
                candidate.readiness_report.highest_policy_tier,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let gap_inventory = readiness_gap_inventory_from_reports(
        candidates
            .iter()
            .map(|candidate| &candidate.readiness_report),
    );
    let mut constraints = Vec::new();

    for gap in gap_inventory.primitive_gaps {
        let primitive = describe_primitive_family(gap.primitive);
        constraints.push(IntegrationActivationConstraint {
            kind: IntegrationActivationConstraintKind::Primitive,
            constraint_id: format!("primitive:{}", gap.primitive.as_str()),
            display_name: primitive.display_name.to_string(),
            highest_priority: gap.highest_priority,
            highest_policy_tier: highest_policy_tier_for_integrations(
                &gap.integration_ids,
                &policy_tier_by_integration,
            ),
            affected_integration_ids: gap.integration_ids,
            blocks_activation: true,
            requires_human_review: false,
            policy_surfaces: Vec::new(),
        });
    }

    for gap in gap_inventory.capability_gaps {
        constraints.push(IntegrationActivationConstraint {
            kind: IntegrationActivationConstraintKind::Capability,
            constraint_id: format!("capability:{}", gap.capability_id.as_str()),
            display_name: gap.capability_id.as_str().to_string(),
            highest_priority: gap.highest_priority,
            highest_policy_tier: highest_policy_tier_for_integrations(
                &gap.integration_ids,
                &policy_tier_by_integration,
            ),
            affected_integration_ids: gap.integration_ids,
            blocks_activation: true,
            requires_human_review: false,
            policy_surfaces: Vec::new(),
        });
    }

    for gap in gap_inventory.dependency_gaps {
        constraints.push(IntegrationActivationConstraint {
            kind: IntegrationActivationConstraintKind::Dependency,
            constraint_id: format!("dependency:{}", gap.integration_id.as_str()),
            display_name: gap.integration_id.as_str().to_string(),
            highest_priority: gap.highest_priority,
            highest_policy_tier: highest_policy_tier_for_integrations(
                &gap.requested_integration_ids,
                &policy_tier_by_integration,
            ),
            affected_integration_ids: gap.requested_integration_ids,
            blocks_activation: true,
            requires_human_review: false,
            policy_surfaces: Vec::new(),
        });
    }

    let mut policy_review_constraints: BTreeMap<
        Option<IntegrationPolicySurface>,
        (u8, BTreeSet<IntegrationId>, PrivilegeTier),
    > = BTreeMap::new();

    for candidate in candidates {
        if !candidate.requires_human_review() {
            continue;
        }

        let policy_surfaces = find_entry(
            catalog,
            &candidate.readiness_report.requested_integration_id,
        )
        .map(IntegrationCatalogEntry::policy_surfaces)
        .unwrap_or_default();

        if policy_surfaces.is_empty() {
            let (highest_priority, integration_ids, highest_policy_tier) =
                policy_review_constraints.entry(None).or_insert((
                    candidate.readiness_report.priority,
                    BTreeSet::new(),
                    candidate.readiness_report.highest_policy_tier,
                ));
            *highest_priority = (*highest_priority).min(candidate.readiness_report.priority);
            integration_ids.insert(candidate.readiness_report.requested_integration_id.clone());
            *highest_policy_tier =
                (*highest_policy_tier).max(candidate.readiness_report.highest_policy_tier);
            continue;
        }

        for surface in policy_surfaces {
            let (highest_priority, integration_ids, highest_policy_tier) =
                policy_review_constraints.entry(Some(surface)).or_insert((
                    candidate.readiness_report.priority,
                    BTreeSet::new(),
                    surface.required_tier(),
                ));
            *highest_priority = (*highest_priority).min(candidate.readiness_report.priority);
            integration_ids.insert(candidate.readiness_report.requested_integration_id.clone());
            *highest_policy_tier = (*highest_policy_tier)
                .max(surface.required_tier())
                .max(candidate.readiness_report.highest_policy_tier);
        }
    }

    for (surface, (highest_priority, integration_ids, highest_policy_tier)) in
        policy_review_constraints
    {
        let (constraint_id, display_name, policy_surfaces) = match surface {
            Some(surface) => (
                format!("policy_review:{}", surface.as_str()),
                surface.as_str().to_string(),
                vec![surface],
            ),
            None => (
                "policy_review:human_approval".to_string(),
                "human_approval".to_string(),
                Vec::new(),
            ),
        };
        constraints.push(IntegrationActivationConstraint {
            kind: IntegrationActivationConstraintKind::PolicyReview,
            constraint_id,
            display_name,
            highest_priority,
            affected_integration_ids: integration_ids.into_iter().collect(),
            blocks_activation: false,
            requires_human_review: true,
            highest_policy_tier,
            policy_surfaces,
        });
    }

    constraints.sort_by(compare_activation_constraints);
    constraints
}

pub fn activation_constraints_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationConstraint> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_constraints_from_candidates(catalog, candidates.iter())
}

pub fn activation_risk_from_candidates<'a>(
    catalog: &[IntegrationCatalogEntry],
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
) -> Vec<IntegrationActivationRiskItem> {
    let mut candidates = candidates.into_iter().collect::<Vec<_>>();
    candidates.sort_by(|left, right| compare_activation_candidates(left, right));

    let mut by_tier: BTreeMap<PrivilegeTier, Vec<&IntegrationActivationCandidate>> =
        BTreeMap::new();
    let mut by_surface: BTreeMap<IntegrationPolicySurface, Vec<&IntegrationActivationCandidate>> =
        BTreeMap::new();

    for candidate in candidates {
        by_tier
            .entry(candidate.readiness_report.highest_policy_tier)
            .or_default()
            .push(candidate);

        if let Some(entry) = find_entry(
            catalog,
            &candidate.readiness_report.requested_integration_id,
        ) {
            for surface in entry.policy_surfaces() {
                by_surface.entry(surface).or_default().push(candidate);
            }
        }
    }

    let mut risks = Vec::new();
    for (tier, candidates) in by_tier {
        let tier_label = privilege_tier_label_for_catalog(tier);
        risks.push(IntegrationActivationRiskItem::from_candidates(
            IntegrationActivationRiskKind::PolicyTier,
            format!("policy_tier:{tier_label}"),
            tier_label.to_string(),
            tier,
            None,
            &candidates,
        ));
    }

    for (surface, candidates) in by_surface {
        risks.push(IntegrationActivationRiskItem::from_candidates(
            IntegrationActivationRiskKind::PolicySurface,
            format!("policy_surface:{}", surface.as_str()),
            surface.as_str().to_string(),
            surface.required_tier(),
            Some(surface),
            &candidates,
        ));
    }

    risks.sort_by(compare_activation_risks);
    risks
}

pub fn activation_risk_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationRiskItem> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_risk_from_candidates(catalog, candidates.iter())
}

pub fn activation_reviews_from_candidates<'a>(
    catalog: &[IntegrationCatalogEntry],
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
) -> Vec<IntegrationActivationReviewItem> {
    let mut reviews = candidates
        .into_iter()
        .filter_map(|candidate| IntegrationActivationReviewItem::from_candidate(catalog, candidate))
        .collect::<Vec<_>>();

    reviews.sort_by(compare_activation_reviews);
    reviews
}

pub fn activation_reviews_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationReviewItem> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_reviews_from_candidates(catalog, candidates.iter())
}

pub fn activation_approval_packets_from_candidates<'a>(
    catalog: &[IntegrationCatalogEntry],
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationApprovalPacket> {
    let mut packets = candidates
        .into_iter()
        .filter_map(|candidate| {
            IntegrationActivationApprovalPacket::from_candidate(
                catalog,
                candidate,
                enabled_integrations,
            )
        })
        .collect::<Vec<_>>();

    packets.sort_by(compare_activation_approval_packets);
    packets
}

pub fn activation_approval_packets_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationApprovalPacket> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_approval_packets_from_candidates(catalog, candidates.iter(), enabled_integrations)
}

pub fn activation_decisions_from_candidates<'a>(
    catalog: &[IntegrationCatalogEntry],
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationDecisionItem> {
    let mut decisions =
        activation_approval_packets_from_candidates(catalog, candidates, enabled_integrations)
            .into_iter()
            .map(IntegrationActivationDecisionItem::from_packet)
            .collect::<Vec<_>>();

    decisions.sort_by(compare_activation_decisions);
    decisions
}

pub fn activation_decisions_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationDecisionItem> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_decisions_from_candidates(catalog, candidates.iter(), enabled_integrations)
}

pub fn activation_evidence_from_decisions<'a>(
    decisions: impl IntoIterator<Item = &'a IntegrationActivationDecisionItem>,
) -> Vec<IntegrationActivationEvidenceItem> {
    let mut evidence = decisions
        .into_iter()
        .flat_map(IntegrationActivationEvidenceItem::from_decision)
        .collect::<Vec<_>>();

    evidence.sort_by(compare_activation_evidence);
    evidence
}

pub fn activation_evidence_from_candidates<'a>(
    catalog: &[IntegrationCatalogEntry],
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationEvidenceItem> {
    let decisions = activation_decisions_from_candidates(catalog, candidates, enabled_integrations);
    activation_evidence_from_decisions(decisions.iter())
}

pub fn activation_evidence_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationEvidenceItem> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_evidence_from_candidates(catalog, candidates.iter(), enabled_integrations)
}

pub fn activation_dossiers_from_decisions(
    decisions: impl IntoIterator<Item = IntegrationActivationDecisionItem>,
) -> Vec<IntegrationActivationDossierItem> {
    let mut dossiers = decisions
        .into_iter()
        .map(IntegrationActivationDossierItem::from_decision)
        .collect::<Vec<_>>();

    dossiers.sort_by(compare_activation_dossiers);
    dossiers
}

pub fn activation_dossiers_from_candidates<'a>(
    catalog: &[IntegrationCatalogEntry],
    candidates: impl IntoIterator<Item = &'a IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationDossierItem> {
    let decisions = activation_decisions_from_candidates(catalog, candidates, enabled_integrations);
    activation_dossiers_from_decisions(decisions)
}

pub fn activation_dossiers_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationDossierItem> {
    let candidates = activation_candidates_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_dossiers_from_candidates(catalog, candidates.iter(), enabled_integrations)
}

pub fn activation_readouts_from_candidates(
    catalog: &[IntegrationCatalogEntry],
    candidates: Vec<IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationReadoutStage> {
    let mut readouts_by_priority: BTreeMap<u8, Vec<IntegrationActivationCandidate>> =
        BTreeMap::new();
    for candidate in candidates {
        readouts_by_priority
            .entry(candidate.readiness_report.priority)
            .or_default()
            .push(candidate);
    }

    let mut readouts = readouts_by_priority
        .into_iter()
        .map(|(priority, candidates)| {
            IntegrationActivationReadoutStage::from_candidates(
                catalog,
                priority,
                candidates,
                enabled_integrations,
            )
        })
        .collect::<Vec<_>>();
    readouts.sort_by(compare_activation_readouts);
    readouts
}

pub fn activation_readouts_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationReadoutStage> {
    activation_readouts_from_candidates(
        catalog,
        activation_candidates_at_or_before_priority(
            catalog,
            priority,
            available_primitives,
            allowed_capabilities,
            enabled_integrations,
        ),
        enabled_integrations,
    )
}

pub fn activation_briefing_items_from_readouts<'a>(
    readouts: impl IntoIterator<Item = &'a IntegrationActivationReadoutStage>,
) -> Vec<IntegrationActivationBriefingItem> {
    let mut items = Vec::new();
    for readout in readouts {
        if readout.has_blockers() {
            items.push(IntegrationActivationBriefingItem::from_readout(
                IntegrationActivationBriefingItemKind::Blocker,
                readout,
            ));
        }
        if readout.has_review_work() {
            items.push(IntegrationActivationBriefingItem::from_readout(
                IntegrationActivationBriefingItemKind::Review,
                readout,
            ));
        }
        if readout.has_approval_ready_work() {
            items.push(IntegrationActivationBriefingItem::from_readout(
                IntegrationActivationBriefingItemKind::Approval,
                readout,
            ));
        }
        if readout.has_activation_work() {
            items.push(IntegrationActivationBriefingItem::from_readout(
                IntegrationActivationBriefingItemKind::Activation,
                readout,
            ));
        }
        if readout.has_risks() {
            items.push(IntegrationActivationBriefingItem::from_readout(
                IntegrationActivationBriefingItemKind::Risk,
                readout,
            ));
        }
        if readout.has_dependency_blockers() {
            items.push(IntegrationActivationBriefingItem::from_readout(
                IntegrationActivationBriefingItemKind::Dependency,
                readout,
            ));
        }
    }
    items.sort_by(compare_activation_briefing_items);
    items
}

pub fn activation_briefing_items_from_candidates(
    catalog: &[IntegrationCatalogEntry],
    candidates: Vec<IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationBriefingItem> {
    let readouts = activation_readouts_from_candidates(catalog, candidates, enabled_integrations);
    activation_briefing_items_from_readouts(readouts.iter())
}

pub fn activation_briefing_items_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationBriefingItem> {
    let readouts = activation_readouts_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_briefing_items_from_readouts(readouts.iter())
}

pub fn activation_dashboard_cards_from_readouts<'a>(
    readouts: impl IntoIterator<Item = &'a IntegrationActivationReadoutStage>,
) -> Vec<IntegrationActivationDashboardCard> {
    let mut cards = readouts
        .into_iter()
        .map(IntegrationActivationDashboardCard::from_readout)
        .collect::<Vec<_>>();
    cards.sort_by(compare_activation_dashboard_cards);
    cards
}

pub fn activation_dashboard_cards_from_candidates(
    catalog: &[IntegrationCatalogEntry],
    candidates: Vec<IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationDashboardCard> {
    let readouts = activation_readouts_from_candidates(catalog, candidates, enabled_integrations);
    activation_dashboard_cards_from_readouts(readouts.iter())
}

pub fn activation_dashboard_cards_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationDashboardCard> {
    let readouts = activation_readouts_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_dashboard_cards_from_readouts(readouts.iter())
}

pub fn activation_timeline_milestones_from_dashboard_cards(
    mut cards: Vec<IntegrationActivationDashboardCard>,
) -> Vec<IntegrationActivationTimelineMilestone> {
    cards.sort_by(compare_activation_dashboard_cards);
    let mut milestones = cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| {
            IntegrationActivationTimelineMilestone::from_dashboard_card(index + 1, card)
        })
        .collect::<Vec<_>>();
    milestones.sort_by(compare_activation_timeline_milestones);
    for (index, milestone) in milestones.iter_mut().enumerate() {
        milestone.sequence = index + 1;
    }
    milestones
}

pub fn activation_timeline_milestones_from_readouts<'a>(
    readouts: impl IntoIterator<Item = &'a IntegrationActivationReadoutStage>,
) -> Vec<IntegrationActivationTimelineMilestone> {
    activation_timeline_milestones_from_dashboard_cards(activation_dashboard_cards_from_readouts(
        readouts,
    ))
}

pub fn activation_timeline_milestones_from_candidates(
    catalog: &[IntegrationCatalogEntry],
    candidates: Vec<IntegrationActivationCandidate>,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationTimelineMilestone> {
    activation_timeline_milestones_from_dashboard_cards(activation_dashboard_cards_from_candidates(
        catalog,
        candidates,
        enabled_integrations,
    ))
}

pub fn activation_timeline_milestones_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationActivationTimelineMilestone> {
    activation_timeline_milestones_from_dashboard_cards(
        activation_dashboard_cards_at_or_before_priority(
            catalog,
            priority,
            available_primitives,
            allowed_capabilities,
            enabled_integrations,
        ),
    )
}

pub fn activation_dependency_graph_from_reports<'a>(
    catalog: &[IntegrationCatalogEntry],
    reports: impl IntoIterator<Item = &'a IntegrationReadinessReport>,
    enabled_integrations: &[IntegrationId],
) -> IntegrationActivationDependencyGraph {
    let reports = reports.into_iter().cloned().collect::<Vec<_>>();
    let mut dependent_ids_by_dependency: BTreeMap<IntegrationId, BTreeSet<IntegrationId>> =
        BTreeMap::new();
    let mut edges = Vec::new();

    for report in &reports {
        for dependency_id in activation_dependency_ids_for_report(catalog, report) {
            dependent_ids_by_dependency
                .entry(dependency_id.clone())
                .or_default()
                .insert(report.requested_integration_id.clone());
            let dependency = find_entry(catalog, &dependency_id);
            let satisfied = enabled_integrations
                .iter()
                .any(|enabled| enabled == &dependency_id);
            edges.push(IntegrationActivationDependencyEdge {
                dependency_integration_id: dependency_id,
                dependent_integration_id: report.requested_integration_id.clone(),
                dependency_display_name: dependency.map(|entry| entry.display_name.clone()),
                dependent_display_name: report.display_name.clone(),
                dependency_priority: dependency.map(|entry| entry.priority),
                dependent_priority: report.priority,
                satisfied,
                blocks_activation: !satisfied,
            });
        }
    }

    let mut nodes = reports
        .iter()
        .map(|report| {
            let integration_id = report.requested_integration_id.clone();
            let mut depends_on_integrations = activation_dependency_ids_for_report(catalog, report);
            depends_on_integrations.sort();
            depends_on_integrations.dedup();
            let dependent_integration_ids = dependent_ids_by_dependency
                .remove(&integration_id)
                .map(|ids| ids.into_iter().collect())
                .unwrap_or_default();
            let enabled = enabled_integrations
                .iter()
                .any(|enabled| enabled == &integration_id);

            IntegrationActivationDependencyNode {
                integration_id,
                display_name: report.display_name.clone(),
                priority: report.priority,
                activation_target: report.activation_target.clone(),
                depends_on_integrations,
                dependent_integration_ids,
                missing_dependencies: report.missing_dependencies.clone(),
                enabled,
                activation_ready: report.activation_ready(),
                requires_human_review: report.requires_human_review,
                highest_policy_tier: report.highest_policy_tier,
            }
        })
        .collect::<Vec<_>>();

    nodes.sort_by(compare_activation_dependency_nodes);
    edges.sort_by(compare_activation_dependency_edges);
    let summary = IntegrationActivationDependencySummary::from_graph(&nodes, &edges);

    IntegrationActivationDependencyGraph {
        nodes,
        edges,
        summary,
    }
}

pub fn activation_dependency_graph_at_or_before_priority(
    catalog: &[IntegrationCatalogEntry],
    priority: u8,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> IntegrationActivationDependencyGraph {
    let reports = readiness_reports_at_or_before_priority(
        catalog,
        priority,
        available_primitives,
        allowed_capabilities,
        enabled_integrations,
    );
    activation_dependency_graph_from_reports(catalog, reports.iter(), enabled_integrations)
}

pub fn readiness_gap_inventory_from_reports<'a>(
    reports: impl IntoIterator<Item = &'a IntegrationReadinessReport>,
) -> IntegrationReadinessGapInventory {
    let mut total_reports = 0;
    let mut activation_ready_reports = 0;
    let mut blocked_reports = 0;
    let mut primitive_gaps: BTreeMap<PrimitiveFamily, (u8, BTreeSet<IntegrationId>)> =
        BTreeMap::new();
    let mut capability_gaps: BTreeMap<CapabilityId, (u8, BTreeSet<IntegrationId>)> =
        BTreeMap::new();
    let mut dependency_gaps: BTreeMap<IntegrationId, (u8, BTreeSet<IntegrationId>)> =
        BTreeMap::new();

    for report in reports {
        total_reports += 1;
        if report.activation_ready() {
            activation_ready_reports += 1;
        } else {
            blocked_reports += 1;
        }

        for primitive in &report.missing_primitives {
            let (highest_priority, integration_ids) = primitive_gaps
                .entry(*primitive)
                .or_insert((report.priority, BTreeSet::new()));
            *highest_priority = (*highest_priority).min(report.priority);
            integration_ids.insert(report.requested_integration_id.clone());
        }
        for capability_id in &report.missing_capabilities {
            let (highest_priority, integration_ids) = capability_gaps
                .entry(capability_id.clone())
                .or_insert((report.priority, BTreeSet::new()));
            *highest_priority = (*highest_priority).min(report.priority);
            integration_ids.insert(report.requested_integration_id.clone());
        }
        for integration_id in &report.missing_dependencies {
            let (highest_priority, requested_integration_ids) = dependency_gaps
                .entry(integration_id.clone())
                .or_insert((report.priority, BTreeSet::new()));
            *highest_priority = (*highest_priority).min(report.priority);
            requested_integration_ids.insert(report.requested_integration_id.clone());
        }
    }

    let mut primitive_gaps = primitive_gaps
        .into_iter()
        .map(
            |(primitive, (highest_priority, integration_ids))| IntegrationReadinessPrimitiveGap {
                primitive,
                highest_priority,
                blocked_report_count: integration_ids.len(),
                integration_ids: integration_ids.into_iter().collect(),
            },
        )
        .collect::<Vec<_>>();
    primitive_gaps.sort_by(|left, right| {
        left.highest_priority
            .cmp(&right.highest_priority)
            .then_with(|| right.blocked_report_count.cmp(&left.blocked_report_count))
            .then_with(|| left.primitive.cmp(&right.primitive))
    });

    let mut capability_gaps = capability_gaps
        .into_iter()
        .map(|(capability_id, (highest_priority, integration_ids))| {
            IntegrationReadinessCapabilityGap {
                capability_id,
                highest_priority,
                blocked_report_count: integration_ids.len(),
                integration_ids: integration_ids.into_iter().collect(),
            }
        })
        .collect::<Vec<_>>();
    capability_gaps.sort_by(|left, right| {
        left.highest_priority
            .cmp(&right.highest_priority)
            .then_with(|| right.blocked_report_count.cmp(&left.blocked_report_count))
            .then_with(|| left.capability_id.cmp(&right.capability_id))
    });

    let mut dependency_gaps = dependency_gaps
        .into_iter()
        .map(
            |(integration_id, (highest_priority, requested_integration_ids))| {
                IntegrationReadinessDependencyGap {
                    integration_id,
                    highest_priority,
                    blocked_report_count: requested_integration_ids.len(),
                    requested_integration_ids: requested_integration_ids.into_iter().collect(),
                }
            },
        )
        .collect::<Vec<_>>();
    dependency_gaps.sort_by(|left, right| {
        left.highest_priority
            .cmp(&right.highest_priority)
            .then_with(|| right.blocked_report_count.cmp(&left.blocked_report_count))
            .then_with(|| left.integration_id.cmp(&right.integration_id))
    });

    IntegrationReadinessGapInventory {
        total_reports,
        activation_ready_reports,
        blocked_reports,
        primitive_gaps,
        capability_gaps,
        dependency_gaps,
    }
}

pub fn readiness_report_for_plan(
    plan: &IntegrationActivationPlan,
    available_primitives: &[PrimitiveFamily],
    allowed_capabilities: &[CapabilityId],
    enabled_integrations: &[IntegrationId],
) -> IntegrationReadinessReport {
    IntegrationReadinessReport {
        requested_integration_id: plan.requested_integration_id.clone(),
        display_name: plan.display_name.clone(),
        activation_target: plan.activation_target.clone(),
        priority: plan.priority,
        missing_primitives: missing_primitives(&plan.required_primitives, available_primitives),
        missing_capabilities: missing_capabilities(
            &plan.required_capabilities,
            allowed_capabilities,
        ),
        missing_dependencies: missing_dependencies_for_plan(plan, enabled_integrations),
        requires_human_review: plan.requires_human_review(),
        highest_policy_tier: plan.highest_policy_tier,
        local_only: plan.local_only,
        cloud_required: plan.cloud_required,
    }
}

pub fn activation_plan_for_entry(entry: &IntegrationCatalogEntry) -> IntegrationActivationPlan {
    let activation_target = if let Some(target) = &entry.virtual_target {
        IntegrationActivationTarget::DelegatedIntegration(target.clone())
    } else if !entry.virtual_iot_standards.is_empty() {
        IntegrationActivationTarget::DelegatedStandards(entry.virtual_iot_standards.clone())
    } else {
        IntegrationActivationTarget::Direct
    };
    let policy_surfaces = entry.policy_surfaces();
    let highest_policy_tier = policy_surfaces
        .iter()
        .map(|surface| surface.required_tier())
        .max()
        .unwrap_or(PrivilegeTier::ReadOnly);

    IntegrationActivationPlan {
        requested_integration_id: entry.integration_id.clone(),
        display_name: entry.display_name.clone(),
        activation_target,
        implementation_status: entry.implementation_status,
        priority: entry.priority,
        runtime_kind: entry.runtime_kind,
        required_primitives: entry.required_primitives.clone(),
        required_capabilities: entry.required_capabilities.clone(),
        auth_modes: entry.auth_modes.clone(),
        discovery_mechanisms: entry.discovery_mechanisms.clone(),
        depends_on_integrations: entry.depends_on_integrations.clone(),
        policy_surfaces,
        highest_policy_tier,
        local_only: entry.is_local() && !entry.requires_cloud(),
        cloud_required: entry.requires_cloud()
            || entry
                .discovery_mechanisms
                .contains(&DiscoveryMechanism::CloudAccount),
    }
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
            PrimitiveFamily::Supervision,
        ],
        ProtocolFamily::ZWave => &[
            PrimitiveFamily::Usb,
            PrimitiveFamily::SerialController,
            PrimitiveFamily::ZWaveSerialApi,
            PrimitiveFamily::RadioNetworkKey,
            PrimitiveFamily::Supervision,
        ],
        ProtocolFamily::Thread => &[
            PrimitiveFamily::Usb,
            PrimitiveFamily::SerialController,
            PrimitiveFamily::Radio802154,
            PrimitiveFamily::Mdns,
            PrimitiveFamily::RadioNetworkKey,
            PrimitiveFamily::Supervision,
        ],
        ProtocolFamily::Matter => &[
            PrimitiveFamily::Mdns,
            PrimitiveFamily::MatterCommissioning,
            PrimitiveFamily::CertificatePairing,
            PrimitiveFamily::LocalPairing,
            PrimitiveFamily::Supervision,
        ],
        ProtocolFamily::Mqtt => &[
            PrimitiveFamily::Mqtt,
            PrimitiveFamily::MqttCredentials,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::Supervision,
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

fn missing_primitives(
    required: &[PrimitiveFamily],
    available: &[PrimitiveFamily],
) -> Vec<PrimitiveFamily> {
    required
        .iter()
        .copied()
        .filter(|primitive| !available.contains(primitive))
        .collect()
}

fn missing_capabilities(required: &[CapabilityId], allowed: &[CapabilityId]) -> Vec<CapabilityId> {
    required
        .iter()
        .filter(|capability_id| !allowed.iter().any(|allowed| allowed == *capability_id))
        .cloned()
        .collect()
}

fn missing_dependencies_for_plan(
    plan: &IntegrationActivationPlan,
    enabled_integrations: &[IntegrationId],
) -> Vec<IntegrationId> {
    let mut dependencies = plan.depends_on_integrations.clone();
    if let IntegrationActivationTarget::DelegatedIntegration(target) = &plan.activation_target {
        if !dependencies.contains(target) {
            dependencies.push(target.clone());
        }
    }

    dependencies
        .into_iter()
        .filter(|integration_id| {
            !enabled_integrations
                .iter()
                .any(|enabled| enabled == integration_id)
        })
        .collect()
}

fn activation_dependency_ids_for_report(
    catalog: &[IntegrationCatalogEntry],
    report: &IntegrationReadinessReport,
) -> Vec<IntegrationId> {
    let mut dependencies =
        activation_plan_for_integration(catalog, &report.requested_integration_id)
            .map(|plan| plan.depends_on_integrations)
            .unwrap_or_default();
    if let IntegrationActivationTarget::DelegatedIntegration(target) = &report.activation_target {
        if !dependencies.contains(target) {
            dependencies.push(target.clone());
        }
    }
    dependencies.sort();
    dependencies.dedup();
    dependencies
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

fn matches_any<T: PartialEq>(filters: &[T], value: &T) -> bool {
    filters.is_empty() || filters.iter().any(|filter| filter == value)
}

fn entry_local_only(entry: &IntegrationCatalogEntry) -> bool {
    entry.is_local() && !entry.requires_cloud()
}

fn entry_cloud_required(entry: &IntegrationCatalogEntry) -> bool {
    entry.requires_cloud()
        || entry
            .discovery_mechanisms
            .contains(&DiscoveryMechanism::CloudAccount)
}

fn sort_query_results(entries: &mut Vec<&IntegrationCatalogEntry>, sort: IntegrationCatalogSort) {
    match sort {
        IntegrationCatalogSort::PriorityThenName => {
            entries.sort_by(|left, right| compare_by_priority_then_name(left, right))
        }
        IntegrationCatalogSort::Name => entries.sort_by(|left, right| compare_by_name(left, right)),
        IntegrationCatalogSort::CategoryThenPriority => entries.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then_with(|| compare_by_priority_then_name(left, right))
        }),
        IntegrationCatalogSort::StatusThenPriority => entries.sort_by(|left, right| {
            left.implementation_status
                .cmp(&right.implementation_status)
                .then_with(|| compare_by_priority_then_name(left, right))
        }),
    }
}

fn activation_health_status_for_summary(
    summary: &IntegrationActivationCandidateSummary,
) -> IntegrationActivationHealthStatus {
    activation_health_status_from_counts(
        summary.ready_to_activate_candidates,
        summary.needs_human_review_candidates,
        summary.blocked_candidates,
    )
}

fn activation_health_status_from_counts(
    ready_to_activate_count: usize,
    review_count: usize,
    blocked_count: usize,
) -> IntegrationActivationHealthStatus {
    if blocked_count > 0 {
        IntegrationActivationHealthStatus::Blocked
    } else if review_count > 0 {
        IntegrationActivationHealthStatus::NeedsReview
    } else if ready_to_activate_count > 0 {
        IntegrationActivationHealthStatus::Ready
    } else {
        IntegrationActivationHealthStatus::Empty
    }
}

fn highest_policy_tier_for_integrations(
    integration_ids: &[IntegrationId],
    policy_tier_by_integration: &BTreeMap<IntegrationId, PrivilegeTier>,
) -> PrivilegeTier {
    integration_ids
        .iter()
        .filter_map(|integration_id| policy_tier_by_integration.get(integration_id).copied())
        .max()
        .unwrap_or(PrivilegeTier::ReadOnly)
}

fn compare_activation_candidates(
    left: &IntegrationActivationCandidate,
    right: &IntegrationActivationCandidate,
) -> Ordering {
    left.recommendation
        .cmp(&right.recommendation)
        .then_with(|| {
            left.readiness_report
                .priority
                .cmp(&right.readiness_report.priority)
        })
        .then_with(|| left.blocker_count.cmp(&right.blocker_count))
        .then_with(|| {
            left.readiness_report
                .display_name
                .cmp(&right.readiness_report.display_name)
        })
        .then_with(|| {
            left.readiness_report
                .requested_integration_id
                .cmp(&right.readiness_report.requested_integration_id)
        })
}

fn compare_activation_actions(
    left: &IntegrationActivationAction,
    right: &IntegrationActivationAction,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| left.kind.sort_rank().cmp(&right.kind.sort_rank()))
        .then_with(|| left.display_name.cmp(&right.display_name))
        .then_with(|| {
            left.requested_integration_id
                .cmp(&right.requested_integration_id)
        })
        .then_with(|| left.primitive.cmp(&right.primitive))
        .then_with(|| left.capability_id.cmp(&right.capability_id))
        .then_with(|| {
            left.dependency_integration_id
                .cmp(&right.dependency_integration_id)
        })
}

fn compare_activation_constraints(
    left: &IntegrationActivationConstraint,
    right: &IntegrationActivationConstraint,
) -> Ordering {
    left.highest_priority
        .cmp(&right.highest_priority)
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| {
            right
                .affected_integration_ids
                .len()
                .cmp(&left.affected_integration_ids.len())
        })
        .then_with(|| left.constraint_id.cmp(&right.constraint_id))
}

fn compare_activation_risks(
    left: &IntegrationActivationRiskItem,
    right: &IntegrationActivationRiskItem,
) -> Ordering {
    left.highest_priority
        .cmp(&right.highest_priority)
        .then_with(|| right.required_tier.cmp(&left.required_tier))
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| right.integration_count().cmp(&left.integration_count()))
        .then_with(|| left.risk_id.cmp(&right.risk_id))
}

fn compare_activation_reviews(
    left: &IntegrationActivationReviewItem,
    right: &IntegrationActivationReviewItem,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| right.required_tier.cmp(&left.required_tier))
        .then_with(|| left.display_name.cmp(&right.display_name))
        .then_with(|| {
            left.requested_integration_id
                .cmp(&right.requested_integration_id)
        })
}

fn compare_activation_approval_packets(
    left: &IntegrationActivationApprovalPacket,
    right: &IntegrationActivationApprovalPacket,
) -> Ordering {
    left.priority()
        .cmp(&right.priority())
        .then_with(|| right.required_tier().cmp(&left.required_tier()))
        .then_with(|| left.display_name().cmp(right.display_name()))
        .then_with(|| {
            left.requested_integration_id()
                .cmp(right.requested_integration_id())
        })
}

fn compare_activation_decisions(
    left: &IntegrationActivationDecisionItem,
    right: &IntegrationActivationDecisionItem,
) -> Ordering {
    left.priority()
        .cmp(&right.priority())
        .then_with(|| left.decision_status.cmp(&right.decision_status))
        .then_with(|| right.required_tier().cmp(&left.required_tier()))
        .then_with(|| left.display_name().cmp(right.display_name()))
        .then_with(|| {
            left.requested_integration_id()
                .cmp(right.requested_integration_id())
        })
}

fn compare_activation_evidence(
    left: &IntegrationActivationEvidenceItem,
    right: &IntegrationActivationEvidenceItem,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| left.status.cmp(&right.status))
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| right.required_tier.cmp(&left.required_tier))
        .then_with(|| left.display_name.cmp(&right.display_name))
        .then_with(|| {
            left.requested_integration_id
                .cmp(&right.requested_integration_id)
        })
        .then_with(|| left.detail_id.cmp(&right.detail_id))
}

fn compare_activation_dossiers(
    left: &IntegrationActivationDossierItem,
    right: &IntegrationActivationDossierItem,
) -> Ordering {
    compare_activation_decisions(&left.decision, &right.decision)
        .then_with(|| {
            right
                .evidence_summary
                .blocking_evidence
                .cmp(&left.evidence_summary.blocking_evidence)
        })
        .then_with(|| {
            right
                .evidence_summary
                .review_evidence
                .cmp(&left.evidence_summary.review_evidence)
        })
        .then_with(|| {
            right
                .evidence_summary
                .total_evidence
                .cmp(&left.evidence_summary.total_evidence)
        })
}

fn compare_activation_readouts(
    left: &IntegrationActivationReadoutStage,
    right: &IntegrationActivationReadoutStage,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| right.has_blockers().cmp(&left.has_blockers()))
        .then_with(|| right.has_review_work().cmp(&left.has_review_work()))
        .then_with(|| {
            right
                .has_approval_ready_work()
                .cmp(&left.has_approval_ready_work())
        })
        .then_with(|| right.has_activation_work().cmp(&left.has_activation_work()))
        .then_with(|| right.has_risks().cmp(&left.has_risks()))
}

fn compare_activation_briefing_items(
    left: &IntegrationActivationBriefingItem,
    right: &IntegrationActivationBriefingItem,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| right.requires_attention().cmp(&left.requires_attention()))
        .then_with(|| right.integration_count().cmp(&left.integration_count()))
}

fn compare_activation_dashboard_cards(
    left: &IntegrationActivationDashboardCard,
    right: &IntegrationActivationDashboardCard,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| right.requires_attention().cmp(&left.requires_attention()))
        .then_with(|| right.has_blockers.cmp(&left.has_blockers))
        .then_with(|| right.has_review_work.cmp(&left.has_review_work))
        .then_with(|| {
            right
                .has_approval_ready_work
                .cmp(&left.has_approval_ready_work)
        })
        .then_with(|| right.has_activation_work.cmp(&left.has_activation_work))
        .then_with(|| right.integration_count().cmp(&left.integration_count()))
}

fn compare_activation_timeline_milestones(
    left: &IntegrationActivationTimelineMilestone,
    right: &IntegrationActivationTimelineMilestone,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| right.requires_attention().cmp(&left.requires_attention()))
        .then_with(|| left.milestone_kind.cmp(&right.milestone_kind))
        .then_with(|| right.has_blockers().cmp(&left.has_blockers()))
        .then_with(|| right.has_review_work().cmp(&left.has_review_work()))
        .then_with(|| {
            right
                .has_approval_ready_work()
                .cmp(&left.has_approval_ready_work())
        })
        .then_with(|| right.has_activation_work().cmp(&left.has_activation_work()))
        .then_with(|| right.integration_count().cmp(&left.integration_count()))
}

fn compare_activation_dependency_nodes(
    left: &IntegrationActivationDependencyNode,
    right: &IntegrationActivationDependencyNode,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| left.display_name.cmp(&right.display_name))
        .then_with(|| left.integration_id.cmp(&right.integration_id))
}

fn compare_activation_dependency_edges(
    left: &IntegrationActivationDependencyEdge,
    right: &IntegrationActivationDependencyEdge,
) -> Ordering {
    left.dependent_priority
        .cmp(&right.dependent_priority)
        .then_with(|| {
            left.dependent_display_name
                .cmp(&right.dependent_display_name)
        })
        .then_with(|| {
            left.dependent_integration_id
                .cmp(&right.dependent_integration_id)
        })
        .then_with(|| {
            left.dependency_integration_id
                .cmp(&right.dependency_integration_id)
        })
}

fn min_optional_priority(left: Option<u8>, right: Option<u8>) -> Option<u8> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(priority), None) | (None, Some(priority)) => Some(priority),
        (None, None) => None,
    }
}

fn privilege_tier_label_for_catalog(tier: PrivilegeTier) -> &'static str {
    match tier {
        PrivilegeTier::ReadOnly => "read_only",
        PrivilegeTier::LowRisk => "low_risk",
        PrivilegeTier::HumanApproval => "human_approval",
        PrivilegeTier::HighRisk => "high_risk",
    }
}

fn compare_by_priority_then_name(
    left: &IntegrationCatalogEntry,
    right: &IntegrationCatalogEntry,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| compare_by_name(left, right))
}

fn compare_by_name(left: &IntegrationCatalogEntry, right: &IntegrationCatalogEntry) -> Ordering {
    left.display_name
        .cmp(&right.display_name)
        .then_with(|| left.integration_id.cmp(&right.integration_id))
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
    fn ecosystem_coverage_reports_platforms_for_primitives() {
        let sources = ecosystem_survey_sources();
        let coverage = ecosystem_primitive_coverage(&sources);
        let matter = coverage
            .iter()
            .find(|item| item.primitive == PrimitiveFamily::MatterCommissioning)
            .unwrap();
        let vault = coverage
            .iter()
            .find(|item| item.primitive == PrimitiveFamily::VaultLease)
            .unwrap();

        assert_eq!(coverage.len(), all_primitive_families().len());
        assert!(matter.covers_platform(EcosystemSurveyPlatform::AppleHome));
        assert!(matter.covers_platform(EcosystemSurveyPlatform::GoogleHome));
        assert!(matter.platform_count() >= 5);
        assert!(vault.is_gap());
    }

    #[test]
    fn primitive_backlog_coverage_connects_rollout_primitives_to_survey_sources() {
        let catalog = first_party_catalog();
        let sources = ecosystem_survey_sources();
        let coverage = primitive_backlog_with_ecosystem_coverage(&catalog, &sources, 1);
        let mqtt = coverage
            .iter()
            .find(|item| item.primitive == PrimitiveFamily::Mqtt)
            .unwrap();
        let matter = coverage
            .iter()
            .find(|item| item.primitive == PrimitiveFamily::MatterCommissioning)
            .unwrap();

        assert!(mqtt.includes_integration(&IntegrationId::trusted("mqtt")));
        assert!(mqtt.includes_integration(&IntegrationId::trusted("tasmota")));
        assert!(mqtt.covers_platform(EcosystemSurveyPlatform::HomeAssistant));
        assert!(matter.includes_integration(&IntegrationId::trusted("matter")));
        assert!(matter.covers_platform(EcosystemSurveyPlatform::ThreadGroup));
        assert!(coverage
            .iter()
            .all(|item| item.source_count == item.platform_count()));
    }

    #[test]
    fn primitive_backlog_coverage_summary_highlights_rollout_gaps() {
        let catalog = first_party_catalog();
        let sources = ecosystem_survey_sources();
        let coverage = primitive_backlog_with_ecosystem_coverage(&catalog, &sources, 1);
        let summary = PrimitiveBacklogCoverageSummary::from_items(coverage.iter());

        assert_eq!(summary.total_primitives, coverage.len());
        assert!(summary.total_entries >= coverage.len());
        assert!(summary.unique_integrations >= 5);
        assert!(summary.covered_primitives > 0);
        assert!(summary.uncovered_primitives > 0);
        assert!(summary.single_source_primitives > 0);
        assert!(summary.multi_platform_primitives > 0);
        assert!(summary.total_source_references >= summary.covered_primitives);
        assert!(summary.total_platform_references >= summary.total_source_references);
        assert!(summary.first_uncovered_priority.is_some());
        assert!(summary.first_single_source_priority.is_some());
        assert!(summary.broadest_platform_count >= 2);
        assert!(summary.has_uncovered_primitives());
        assert!(summary.has_single_source_primitives());
    }

    #[test]
    fn ecosystem_platform_coverage_rolls_sources_against_backlog() {
        let catalog = first_party_catalog();
        let sources = ecosystem_survey_sources();
        let coverage = ecosystem_platform_coverage(&catalog, &sources, 1);
        let summary = EcosystemPlatformCoverageSummary::from_items(coverage.iter());
        let home_assistant = coverage
            .iter()
            .find(|item| item.platform == EcosystemSurveyPlatform::HomeAssistant)
            .unwrap();
        let thread_group = coverage
            .iter()
            .find(|item| item.platform == EcosystemSurveyPlatform::ThreadGroup)
            .unwrap();

        assert_eq!(coverage.len(), sources.len());
        assert!(home_assistant.covers_primitive(PrimitiveFamily::Mqtt));
        assert!(home_assistant.has_backlog_overlap());
        assert!(thread_group.covers_primitive(PrimitiveFamily::MatterCommissioning));
        assert!(thread_group.has_backlog_overlap());
        assert_eq!(summary.total_platforms, sources.len());
        assert!(summary.covered_backlog_primitives > 0);
        assert!(summary.platforms_with_backlog_overlap >= 5);
        assert!(summary.has_uncovered_backlog_primitives());
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
    fn primitive_backlog_ranks_rollout_wave_foundations() {
        let catalog = first_party_catalog();
        let backlog = primitive_backlog_at_or_before_priority(&catalog, 1);
        let supervision = backlog
            .iter()
            .find(|item| item.primitive == PrimitiveFamily::Supervision)
            .unwrap();
        let radio = backlog
            .iter()
            .find(|item| item.primitive == PrimitiveFamily::Radio802154)
            .unwrap();
        let mqtt = backlog
            .iter()
            .find(|item| item.primitive == PrimitiveFamily::Mqtt)
            .unwrap();

        assert_eq!(backlog[0].highest_priority, 0);
        assert_eq!(supervision.highest_priority, 0);
        assert!(supervision.entry_count >= 8);
        assert!(supervision.includes_integration(&IntegrationId::trusted("hue")));
        assert!(supervision.includes_integration(&IntegrationId::trusted("zigbee")));
        assert!(supervision.includes_integration(&IntegrationId::trusted("zwave")));
        assert!(supervision.includes_integration(&IntegrationId::trusted("thread")));
        assert!(radio.includes_integration(&IntegrationId::trusted("zigbee")));
        assert!(radio.includes_integration(&IntegrationId::trusted("thread")));
        assert!(mqtt.includes_integration(&IntegrationId::trusted("mqtt")));
        assert!(mqtt.includes_integration(&IntegrationId::trusted("tasmota")));
    }

    #[test]
    fn activation_plans_resolve_direct_virtual_and_standard_targets() {
        let catalog = first_party_catalog();
        let hue =
            activation_plan_for_integration(&catalog, &IntegrationId::trusted("hue")).unwrap();
        let tapo =
            activation_plan_for_integration(&catalog, &IntegrationId::trusted("tplink_tapo"))
                .unwrap();
        let ultraloq =
            activation_plan_for_integration(&catalog, &IntegrationId::trusted("ultraloq")).unwrap();

        assert_eq!(hue.activation_target, IntegrationActivationTarget::Direct);
        assert!(hue.local_only);
        assert!(hue.requires_primitive(PrimitiveFamily::LocalPairing));

        assert!(tapo.delegates_to_integration(&IntegrationId::trusted("tplink")));
        assert_eq!(tapo.runtime_kind, RuntimeKind::InProcessRust);

        assert!(ultraloq.delegates_to_standard(&ProtocolFamily::ZWave));
        assert!(ultraloq.requires_primitive(PrimitiveFamily::CalculatedState));
    }

    #[test]
    fn activation_plans_capture_review_and_cloud_boundaries() {
        let catalog = first_party_catalog();
        let zwave =
            activation_plan_for_integration(&catalog, &IntegrationId::trusted("zwave")).unwrap();
        let ring =
            activation_plan_for_integration(&catalog, &IntegrationId::trusted("ring")).unwrap();

        assert!(zwave.requires_human_review());
        assert_eq!(zwave.highest_policy_tier, PrivilegeTier::HighRisk);
        assert!(zwave
            .policy_surfaces
            .contains(&IntegrationPolicySurface::EntryAccess));
        assert!(zwave.requires_primitive(PrimitiveFamily::RadioNetworkKey));
        assert!(zwave.requires_capability(&CapabilityId::trusted("smart_home.command.lock")));

        assert!(ring.cloud_required);
        assert!(ring.requires_human_review());
        assert!(ring
            .policy_surfaces
            .contains(&IntegrationPolicySurface::CredentialedCloud));
    }

    #[test]
    fn activation_plans_follow_rollout_priority_waves() {
        let catalog = first_party_catalog();
        let early = activation_plans_at_or_before_priority(&catalog, 2);

        assert!(early
            .iter()
            .any(|plan| plan.requested_integration_id == IntegrationId::trusted("hue")));
        assert!(early
            .iter()
            .any(|plan| plan.requested_integration_id == IntegrationId::trusted("mqtt")));
        assert!(!early
            .iter()
            .any(|plan| plan.requested_integration_id == IntegrationId::trusted("tuya")));
        assert!(early.iter().any(|plan| plan
            .depends_on_integrations
            .contains(&IntegrationId::trusted("mqtt"))));

        let summary = IntegrationActivationPlanSummary::from_plans(early.iter());
        assert_eq!(summary.total_plans, early.len());
        assert!(summary.direct_targets > 0);
        assert!(summary.has_delegated_targets());
        assert!(summary.local_only_plans > 0);
        assert!(summary.plans_with_dependencies > 0);
        assert!(summary.plans_with_required_primitives > 0);
        assert!(summary.plans_with_required_capabilities > 0);
        assert!(summary.unique_required_primitives > 0);
        assert!(summary.unique_required_capabilities > 0);
        assert!(summary.unique_dependencies > 0);
        assert!(!summary.is_empty());
    }

    #[test]
    fn readiness_reports_identify_missing_primitives_and_capabilities() {
        let catalog = first_party_catalog();
        let report = readiness_report_for_integration(
            &catalog,
            &IntegrationId::trusted("hue"),
            &[PrimitiveFamily::Mdns],
            &[CapabilityId::trusted("smart_home.read")],
            &[],
        )
        .unwrap();

        assert!(report.is_blocked());
        assert!(report.missing_primitive(PrimitiveFamily::LocalHttp));
        assert!(report.missing_primitive(PrimitiveFamily::ServerSentEvents));
        assert!(report.missing_primitive(PrimitiveFamily::LocalPairing));
        assert!(report.missing_capability(&CapabilityId::trusted("smart_home.command.light")));
        assert!(report.missing_capability(&CapabilityId::trusted("smart_home.pair")));
        assert!(!report.missing_dependency(&IntegrationId::trusted("mqtt")));
        assert!(report.requires_human_review);
    }

    #[test]
    fn readiness_reports_mark_complete_direct_integrations_ready() {
        let catalog = first_party_catalog();
        let hue = find_entry(&catalog, &IntegrationId::trusted("hue")).unwrap();
        let report = readiness_report_for_integration(
            &catalog,
            &hue.integration_id,
            &hue.required_primitives,
            &hue.required_capabilities,
            &[],
        )
        .unwrap();

        assert!(report.activation_ready());
        assert_eq!(
            report.requested_integration_id,
            IntegrationId::trusted("hue")
        );
        assert_eq!(
            report.activation_target,
            IntegrationActivationTarget::Direct
        );
        assert!(report.local_only);
        assert!(!report.cloud_required);
        assert_eq!(report.highest_policy_tier, PrivilegeTier::HumanApproval);
    }

    #[test]
    fn activation_candidates_rank_ready_review_and_blocked_work() {
        let catalog = first_party_catalog();
        let hue = find_entry(&catalog, &IntegrationId::trusted("hue")).unwrap();
        let tasmota = find_entry(&catalog, &IntegrationId::trusted("tasmota")).unwrap();
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let review_report = readiness_report_for_integration(
            &catalog,
            &hue.integration_id,
            &hue.required_primitives,
            &hue.required_capabilities,
            &[],
        )
        .unwrap();
        let blocked_report = readiness_report_for_integration(
            &catalog,
            &tasmota.integration_id,
            &[],
            &[CapabilityId::trusted("smart_home.read")],
            &[],
        )
        .unwrap();

        assert!(review_report.activation_ready());
        assert!(review_report.requires_human_review);
        assert!(blocked_report.is_blocked());

        let candidates = activation_candidates_from_reports(
            [blocked_report, review_report, ready_report].iter(),
        );

        assert_eq!(
            candidates[0].recommendation,
            IntegrationActivationCandidateRecommendation::ReadyToActivate
        );
        assert_eq!(
            candidates[1].recommendation,
            IntegrationActivationCandidateRecommendation::NeedsHumanReview
        );
        assert_eq!(
            candidates[2].recommendation,
            IntegrationActivationCandidateRecommendation::BlockedOnPrerequisites
        );
        assert!(candidates[0].is_actionable());
        assert!(candidates[2].is_blocked());
        assert!(candidates[2].blocker_count > 0);

        let summary = IntegrationActivationCandidateSummary::from_candidates(candidates.iter());
        assert_eq!(summary.total_candidates, 3);
        assert_eq!(summary.ready_to_activate_candidates, 1);
        assert_eq!(summary.needs_human_review_candidates, 1);
        assert_eq!(summary.blocked_candidates, 1);
        assert_eq!(summary.activation_ready_candidates, 2);
        assert_eq!(summary.candidates_requiring_human_review, 2);
        assert!(summary.candidates_missing_primitives > 0);
        assert!(summary.candidates_missing_capabilities > 0);
        assert!(summary.candidates_missing_dependencies > 0);
        assert!(summary.has_actionable_candidates());
        assert!(summary.has_blockers());
        assert!(summary.has_review_work());
        assert!(!summary.is_empty());
    }

    #[test]
    fn activation_runway_groups_candidates_by_priority_wave() {
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 2,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_bridge"),
            display_name: "Review Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_sensor"),
            display_name: "Blocked Sensor".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: vec![PrimitiveFamily::Mqtt],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::LowRisk,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [ready_report, review_report, blocked_report].iter(),
        );

        let stages = activation_runway_from_candidates(candidates);

        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].priority, 1);
        assert_eq!(stages[0].summary.total_candidates, 2);
        assert_eq!(stages[0].summary.needs_human_review_candidates, 1);
        assert_eq!(stages[0].summary.blocked_candidates, 1);
        assert!(stages[0].has_actionable_candidates());
        assert!(stages[0].has_blockers());
        assert!(stages[0].has_review_work());
        assert_eq!(stages[1].priority, 2);
        assert_eq!(stages[1].summary.ready_to_activate_candidates, 1);

        let summary = IntegrationActivationRunwaySummary::from_stages(stages.iter());
        assert_eq!(summary.total_stages, 2);
        assert_eq!(summary.total_candidates, 3);
        assert_eq!(summary.actionable_stages, 2);
        assert_eq!(summary.ready_stages, 1);
        assert_eq!(summary.review_stages, 1);
        assert_eq!(summary.blocked_stages, 1);
        assert_eq!(summary.first_actionable_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(1));
        assert_eq!(summary.next_ready_priority, Some(2));
        assert_eq!(summary.candidate_summary.total_candidates, 3);
        assert!(summary.has_actionable_stage());
        assert!(summary.has_blocked_stage());
        assert!(summary.has_review_stage());
        assert!(!summary.is_empty());
    }

    #[test]
    fn activation_health_rolls_up_priority_stage_attention() {
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 2,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_bridge"),
            display_name: "Review Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_sensor"),
            display_name: "Blocked Sensor".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 1,
            missing_primitives: vec![PrimitiveFamily::Mqtt],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::LowRisk,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [ready_report, review_report, blocked_report].iter(),
        );

        let health = activation_health_from_candidates(candidates);

        assert_eq!(health.len(), 2);
        assert_eq!(health[0].priority, 1);
        assert_eq!(
            health[0].health_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(health[0].requires_attention());
        assert!(health[0].has_review_work());
        assert!(health[0].has_blockers());
        assert_eq!(health[0].candidate_summary.total_candidates, 2);
        assert_eq!(health[0].gap_inventory.primitive_gap_count(), 1);
        assert_eq!(health[0].gap_inventory.capability_gap_count(), 1);
        assert_eq!(health[0].gap_inventory.dependency_gap_count(), 1);
        assert!(health[0]
            .blocked_integration_ids
            .contains(&IntegrationId::trusted("blocked_sensor")));
        assert!(health[0]
            .review_integration_ids
            .contains(&IntegrationId::trusted("review_bridge")));
        assert_eq!(health[1].priority, 2);
        assert_eq!(
            health[1].health_status,
            IntegrationActivationHealthStatus::Ready
        );
        assert!(health[1]
            .ready_to_activate_integration_ids
            .contains(&IntegrationId::trusted("read_only_probe")));

        let summary = IntegrationActivationHealthSummary::from_stages(health.iter());
        assert_eq!(summary.total_stages, 2);
        assert_eq!(summary.total_integrations, 3);
        assert_eq!(summary.ready_stages, 1);
        assert_eq!(summary.review_stages, 1);
        assert_eq!(summary.blocked_stages, 1);
        assert_eq!(summary.ready_to_activate_integrations, 1);
        assert_eq!(summary.review_integrations, 1);
        assert_eq!(summary.blocked_integrations, 1);
        assert_eq!(summary.primitive_gap_count, 1);
        assert_eq!(summary.capability_gap_count, 1);
        assert_eq!(summary.dependency_gap_count, 1);
        assert_eq!(summary.total_unique_gaps, 3);
        assert_eq!(summary.first_ready_priority, Some(2));
        assert_eq!(summary.first_review_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(1));
        assert_eq!(
            summary.overall_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(summary.requires_attention());
        assert!(summary.has_ready_work());
        assert!(summary.has_review_work());
        assert!(summary.has_blockers());
        assert!(!summary.is_empty());
    }

    #[test]
    fn activation_maintenance_windows_roll_up_activation_work() {
        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let enabled_integrations = vec![IntegrationId::trusted("mqtt")];
        let candidates = activation_candidates_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &enabled_integrations,
        );

        let windows = activation_maintenance_from_candidates(
            &catalog,
            candidates.clone(),
            &enabled_integrations,
        );

        assert!(!windows.is_empty());
        assert!(windows.iter().all(|window| window.priority <= 2));
        assert!(windows
            .iter()
            .any(IntegrationActivationMaintenanceWindow::has_blockers));
        assert!(windows
            .iter()
            .any(IntegrationActivationMaintenanceWindow::has_review_work));
        let first_blocked = windows.iter().find(|window| window.has_blockers()).unwrap();
        assert!(!first_blocked.integration_ids.is_empty());
        assert!(first_blocked.action_summary.total_actions > 0);
        assert!(first_blocked.constraint_summary.total_constraints > 0);
        assert!(first_blocked.risk_summary.total_risks > 0);

        let summary = IntegrationActivationMaintenanceSummary::from_windows(windows.iter());
        assert_eq!(summary.total_windows, windows.len());
        assert_eq!(summary.total_integrations, candidates.len());
        assert!(summary.total_actions > 0);
        assert!(summary.blocking_constraints > 0);
        assert!(summary.total_risks > 0);
        assert!(summary.total_dependency_edges > 0);
        assert!(summary.windows_with_actions > 0);
        assert!(summary.windows_with_blockers > 0);
        assert!(summary.windows_with_review_work > 0);
        assert!(summary.windows_with_risks > 0);
        assert!(summary.requires_attention());
        assert!(summary.has_blockers());
        assert!(summary.has_review_work());
        assert!(summary.has_risks());
        assert!(!summary.is_empty());

        let windows_from_catalog = activation_maintenance_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &enabled_integrations,
        );
        assert_eq!(windows, windows_from_catalog);
    }

    #[test]
    fn activation_constraints_group_readiness_and_policy_review_work() {
        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let candidates = activation_candidates_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );

        let constraints = activation_constraints_from_candidates(&catalog, candidates.iter());

        assert!(constraints.iter().any(|constraint| constraint.kind
            == IntegrationActivationConstraintKind::Primitive
            && constraint.blocks_activation));
        assert!(constraints.iter().any(|constraint| constraint.kind
            == IntegrationActivationConstraintKind::Capability
            && constraint.blocks_activation));
        assert!(constraints.iter().any(|constraint| constraint.kind
            == IntegrationActivationConstraintKind::Dependency
            && constraint.blocks_activation));
        let policy_review = constraints
            .iter()
            .find(|constraint| constraint.kind == IntegrationActivationConstraintKind::PolicyReview)
            .unwrap();
        assert!(policy_review.requires_human_review);
        assert!(!policy_review.blocks_activation);
        assert!(!policy_review.policy_surfaces.is_empty());
        assert_eq!(
            policy_review.kind.as_str(),
            IntegrationActivationConstraintKind::PolicyReview.as_str()
        );

        let summary = IntegrationActivationConstraintSummary::from_constraints(constraints.iter());
        assert_eq!(summary.total_constraints, constraints.len());
        assert!(summary.blocking_constraints >= 3);
        assert!(summary.review_constraints > 0);
        assert!(summary.primitive_constraints > 0);
        assert!(summary.capability_constraints > 0);
        assert!(summary.dependency_constraints > 0);
        assert!(summary.policy_review_constraints > 0);
        assert!(summary.affected_integrations > 0);
        assert!(summary.first_blocking_priority <= Some(2));
        assert!(summary.first_review_priority <= Some(2));
        assert!(summary.has_blockers());
        assert!(summary.has_review_work());
        assert!(!summary.is_empty());

        let constraints_from_catalog = activation_constraints_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert_eq!(constraints, constraints_from_catalog);
    }

    #[test]
    fn activation_risk_groups_candidates_by_policy_tier_and_surface() {
        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let candidates = activation_candidates_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );

        let risks = activation_risk_from_candidates(&catalog, candidates.iter());

        assert!(risks
            .iter()
            .any(|risk| risk.kind == IntegrationActivationRiskKind::PolicyTier));
        assert!(risks
            .iter()
            .any(|risk| risk.kind == IntegrationActivationRiskKind::PolicySurface));

        let human_approval = risks
            .iter()
            .find(|risk| {
                risk.kind == IntegrationActivationRiskKind::PolicyTier
                    && risk.required_tier == PrivilegeTier::HumanApproval
            })
            .unwrap();
        assert!(human_approval.integration_count() >= 1);
        assert!(human_approval.requires_attention());

        let review_surface = risks
            .iter()
            .find(|risk| {
                risk.kind == IntegrationActivationRiskKind::PolicySurface
                    && risk.required_tier >= PrivilegeTier::HumanApproval
            })
            .unwrap();
        assert!(review_surface.policy_surface.is_some());
        assert!(review_surface.integration_count() >= 1);
        assert!(review_surface.requires_attention());

        let summary = IntegrationActivationRiskSummary::from_risks(risks.iter());
        assert_eq!(summary.total_risks, risks.len());
        assert!(summary.policy_tier_risks > 0);
        assert!(summary.policy_surface_risks > 0);
        assert!(summary.unique_integrations <= summary.total_risk_entries);
        assert!(summary.review_integrations > 0 || summary.blocked_integrations > 0);
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let risks_from_catalog = activation_risk_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert_eq!(risks, risks_from_catalog);
    }

    #[test]
    fn activation_reviews_queue_ready_and_blocked_human_review_work() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );

        let reviews = activation_reviews_from_candidates(&[], candidates.iter());

        assert_eq!(reviews.len(), 2);
        let review_ready = reviews
            .iter()
            .find(|review| review.requested_integration_id.as_str() == "review_ready_bridge")
            .unwrap();
        assert!(review_ready.activation_ready());
        assert!(review_ready.review_ready());
        assert!(!review_ready.has_blockers());
        assert!(review_ready.requires_attention());
        assert_eq!(review_ready.required_tier, PrivilegeTier::HumanApproval);
        let blocked_review = reviews
            .iter()
            .find(|review| review.requested_integration_id.as_str() == "blocked_review_camera")
            .unwrap();
        assert!(!blocked_review.activation_ready());
        assert!(!blocked_review.review_ready());
        assert!(blocked_review.has_blockers());
        assert_eq!(blocked_review.blocker_count, 3);
        assert_eq!(blocked_review.required_tier, PrivilegeTier::HighRisk);

        let summary = IntegrationActivationReviewSummary::from_reviews(reviews.iter());
        assert_eq!(summary.total_reviews, 2);
        assert_eq!(summary.review_ready_integrations, 1);
        assert_eq!(summary.blocked_review_integrations, 1);
        assert_eq!(summary.reviews_missing_primitives, 1);
        assert_eq!(summary.reviews_missing_capabilities, 1);
        assert_eq!(summary.reviews_missing_dependencies, 1);
        assert_eq!(summary.total_blockers, 3);
        assert_eq!(summary.first_review_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(2));
        assert_eq!(summary.local_only_reviews, 1);
        assert_eq!(summary.cloud_required_reviews, 1);
        assert_eq!(summary.human_approval_reviews, 1);
        assert_eq!(summary.high_risk_reviews, 1);
        assert_eq!(summary.highest_policy_tier, PrivilegeTier::HighRisk);
        assert!(summary.has_review_ready_work());
        assert!(summary.has_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_reviews = activation_reviews_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_reviews
            .iter()
            .any(IntegrationActivationReviewItem::has_policy_surfaces));
    }

    #[test]
    fn activation_approval_packets_bundle_review_work_for_human_decisions() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );

        let packets = activation_approval_packets_from_candidates(&[], candidates.iter(), &[]);

        assert_eq!(packets.len(), 2);
        let approval_ready = packets
            .iter()
            .find(|packet| packet.requested_integration_id().as_str() == "review_ready_bridge")
            .unwrap();
        assert!(approval_ready.approval_ready());
        assert!(!approval_ready.has_blockers());
        assert!(approval_ready.requires_attention());
        assert!(approval_ready.action_summary.review_policy_actions >= 1);
        assert!(approval_ready.constraint_summary.review_constraints >= 1);

        let blocked = packets
            .iter()
            .find(|packet| packet.requested_integration_id().as_str() == "blocked_review_camera")
            .unwrap();
        assert!(!blocked.approval_ready());
        assert!(blocked.has_blockers());
        assert_eq!(blocked.required_tier(), PrivilegeTier::HighRisk);
        assert!(blocked.action_summary.provide_primitive_actions >= 1);
        assert!(blocked.constraint_summary.blocking_constraints >= 1);
        assert!(blocked.risk_summary.total_risks >= 1);
        assert!(blocked.dependency_graph.summary.total_edges >= 1);
        assert!(blocked.dependency_graph.summary.blocking_edges >= 1);

        let summary = IntegrationActivationApprovalSummary::from_packets(packets.iter());
        assert_eq!(summary.total_packets, 2);
        assert_eq!(summary.approval_ready_packets, 1);
        assert_eq!(summary.blocked_packets, 1);
        assert!(summary.total_actions > 0);
        assert!(summary.review_policy_actions > 0);
        assert!(summary.blocking_constraints > 0);
        assert!(summary.total_risks > 0);
        assert!(summary.total_dependency_edges > 0);
        assert!(summary.blocking_dependency_edges > 0);
        assert_eq!(summary.human_approval_packets, 1);
        assert_eq!(summary.high_risk_packets, 1);
        assert_eq!(summary.first_approval_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(2));
        assert!(summary.has_approval_ready_work());
        assert!(summary.has_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_packets = activation_approval_packets_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_packets
            .iter()
            .any(IntegrationActivationApprovalPacket::has_policy_surfaces));
    }

    #[test]
    fn activation_decisions_project_approval_packets_into_decision_queue() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );

        let decisions = activation_decisions_from_candidates(&[], candidates.iter(), &[]);

        assert_eq!(decisions.len(), 2);
        let ready_to_approve = decisions
            .iter()
            .find(|decision| decision.requested_integration_id().as_str() == "review_ready_bridge")
            .unwrap();
        assert_eq!(
            ready_to_approve.decision_status,
            IntegrationActivationDecisionStatus::ReadyToApprove
        );
        assert!(ready_to_approve.approval_ready());
        assert!(!ready_to_approve.has_blockers());
        assert!(ready_to_approve.requires_attention());

        let blocked = decisions
            .iter()
            .find(|decision| {
                decision.requested_integration_id().as_str() == "blocked_review_camera"
            })
            .unwrap();
        assert_eq!(
            blocked.decision_status,
            IntegrationActivationDecisionStatus::BlockedOnPrerequisites
        );
        assert!(!blocked.approval_ready());
        assert!(blocked.has_blockers());
        assert_eq!(blocked.required_tier(), PrivilegeTier::HighRisk);

        let summary = IntegrationActivationDecisionSummary::from_decisions(decisions.iter());
        assert_eq!(summary.total_decisions, 2);
        assert_eq!(summary.ready_to_approve_decisions, 1);
        assert_eq!(summary.blocked_decisions, 1);
        assert!(summary.total_actions > 0);
        assert!(summary.review_policy_actions > 0);
        assert!(summary.blocking_constraints > 0);
        assert!(summary.total_risks > 0);
        assert!(summary.total_dependency_edges > 0);
        assert!(summary.blocking_dependency_edges > 0);
        assert_eq!(summary.human_approval_decisions, 1);
        assert_eq!(summary.high_risk_decisions, 1);
        assert_eq!(summary.first_approval_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(2));
        assert!(summary.has_approval_ready_work());
        assert!(summary.has_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_decisions = activation_decisions_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_decisions
            .iter()
            .any(IntegrationActivationDecisionItem::has_policy_surfaces));
    }

    #[test]
    fn activation_evidence_explains_decision_support_and_blockers() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );

        let evidence = activation_evidence_from_candidates(&[], candidates.iter(), &[]);

        assert!(evidence.iter().any(|item| {
            item.kind == IntegrationActivationEvidenceKind::ApprovalDecision
                && item.status == IntegrationActivationEvidenceStatus::SupportsApproval
                && item.requested_integration_id.as_str() == "review_ready_bridge"
        }));
        assert!(evidence.iter().any(|item| {
            item.kind == IntegrationActivationEvidenceKind::ApprovalDecision
                && item.status == IntegrationActivationEvidenceStatus::BlocksApproval
                && item.requested_integration_id.as_str() == "blocked_review_camera"
        }));
        assert!(evidence.iter().any(|item| {
            item.kind == IntegrationActivationEvidenceKind::PrimitiveBlocker
                && item.primitive == Some(PrimitiveFamily::CameraMedia)
                && item.blocks_approval()
        }));
        assert!(evidence.iter().any(|item| {
            item.kind == IntegrationActivationEvidenceKind::CapabilityBlocker
                && item.capability_id.as_ref().is_some_and(|capability_id| {
                    capability_id.as_str() == "smart_home.command.low_risk"
                })
                && item.blocks_approval()
        }));
        assert!(evidence.iter().any(|item| {
            item.kind == IntegrationActivationEvidenceKind::DependencyBlocker
                && item
                    .dependency_integration_id
                    .as_ref()
                    .is_some_and(|integration_id| integration_id.as_str() == "mqtt")
                && item.blocks_approval()
        }));
        assert!(evidence
            .iter()
            .any(|item| item.kind == IntegrationActivationEvidenceKind::PolicyRisk));

        let summary = IntegrationActivationEvidenceSummary::from_evidence(evidence.iter());
        assert_eq!(summary.approval_decision_evidence, 2);
        assert_eq!(summary.ready_to_approve_integrations, 1);
        assert_eq!(summary.blocked_integrations, 1);
        assert!(summary.total_evidence > summary.approval_decision_evidence);
        assert!(summary.supporting_evidence > 0);
        assert!(summary.review_evidence > 0);
        assert!(summary.blocking_evidence > 0);
        assert!(summary.policy_risk_evidence > 0);
        assert!(summary.primitive_blocker_evidence > 0);
        assert!(summary.capability_blocker_evidence > 0);
        assert!(summary.dependency_blocker_evidence > 0);
        assert_eq!(summary.first_supporting_priority, Some(1));
        assert_eq!(summary.first_blocking_priority, Some(2));
        assert!(summary.has_supporting_evidence());
        assert!(summary.has_review_work());
        assert!(summary.has_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_evidence = activation_evidence_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_evidence
            .iter()
            .any(|item| item.policy_surface.is_some()));
    }

    #[test]
    fn activation_dossiers_bundle_decisions_with_evidence_rollups() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );

        let dossiers = activation_dossiers_from_candidates(&[], candidates.iter(), &[]);

        assert_eq!(dossiers.len(), 2);
        let ready = dossiers
            .iter()
            .find(|dossier| dossier.requested_integration_id().as_str() == "review_ready_bridge")
            .unwrap();
        assert!(ready.approval_ready());
        assert!(!ready.has_blockers());
        assert!(ready.evidence_summary.has_supporting_evidence());
        assert!(ready.evidence_summary.has_review_work());
        assert!(!ready.evidence.is_empty());

        let blocked = dossiers
            .iter()
            .find(|dossier| dossier.requested_integration_id().as_str() == "blocked_review_camera")
            .unwrap();
        assert!(blocked.has_blockers());
        assert!(blocked.requires_attention());
        assert!(blocked.evidence_summary.has_blockers());
        assert_eq!(blocked.required_tier(), PrivilegeTier::HighRisk);

        let summary = IntegrationActivationDossierSummary::from_dossiers(dossiers.iter());
        assert_eq!(summary.total_dossiers, 2);
        assert_eq!(summary.ready_to_approve_dossiers, 1);
        assert_eq!(summary.blocked_dossiers, 1);
        assert!(summary.total_evidence >= ready.evidence.len() + blocked.evidence.len());
        assert!(summary.supporting_evidence > 0);
        assert!(summary.review_evidence > 0);
        assert!(summary.blocking_evidence > 0);
        assert_eq!(summary.first_approval_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(2));
        assert_eq!(summary.highest_policy_tier, PrivilegeTier::HighRisk);
        assert!(summary.has_approval_ready_work());
        assert!(summary.has_review_work());
        assert!(summary.has_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_dossiers = activation_dossiers_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_dossiers
            .iter()
            .any(IntegrationActivationDossierItem::has_policy_surfaces));
    }

    #[test]
    fn activation_readouts_roll_up_wave_health_dossiers_and_evidence() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );

        let readouts = activation_readouts_from_candidates(&[], candidates, &[]);

        assert_eq!(readouts.len(), 3);
        let approval_ready = readouts
            .iter()
            .find(|readout| readout.priority == 1)
            .unwrap();
        assert_eq!(
            approval_ready.health_status,
            IntegrationActivationHealthStatus::NeedsReview
        );
        assert!(approval_ready.has_approval_ready_work());
        assert!(approval_ready.has_review_work());
        assert_eq!(approval_ready.dossier_summary.ready_to_approve_dossiers, 1);
        assert!(approval_ready.evidence_summary.has_supporting_evidence());

        let blocked = readouts
            .iter()
            .find(|readout| readout.priority == 2)
            .unwrap();
        assert_eq!(
            blocked.health_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(blocked.has_blockers());
        assert!(blocked.has_review_work());
        assert_eq!(blocked.dossier_summary.blocked_dossiers, 1);
        assert!(blocked.evidence_summary.has_blockers());
        assert!(blocked.dependency_summary().has_blocking_dependencies());

        let ready = readouts
            .iter()
            .find(|readout| readout.priority == 3)
            .unwrap();
        assert_eq!(
            ready.health_status,
            IntegrationActivationHealthStatus::Ready
        );
        assert!(ready.has_activation_work());
        assert!(!ready.has_approval_ready_work());

        let summary = IntegrationActivationReadoutSummary::from_readouts(readouts.iter());
        assert_eq!(summary.total_readouts, 3);
        assert_eq!(summary.ready_readouts, 1);
        assert_eq!(summary.review_readouts, 1);
        assert_eq!(summary.blocked_readouts, 1);
        assert_eq!(summary.total_dossiers, 2);
        assert_eq!(summary.ready_to_approve_dossiers, 1);
        assert_eq!(summary.blocked_dossiers, 1);
        assert_eq!(summary.readouts_with_activation_work, 1);
        assert_eq!(summary.readouts_with_approval_work, 1);
        assert_eq!(summary.readouts_with_blockers, 2);
        assert!(summary.total_evidence > 0);
        assert!(summary.supporting_evidence > 0);
        assert!(summary.review_evidence > 0);
        assert!(summary.blocking_evidence > 0);
        assert_eq!(summary.first_approval_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(1));
        assert_eq!(summary.first_activation_priority, Some(3));
        assert_eq!(summary.highest_policy_tier, PrivilegeTier::HighRisk);
        assert_eq!(
            summary.overall_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(summary.has_activation_work());
        assert!(summary.has_approval_ready_work());
        assert!(summary.has_review_work());
        assert!(summary.has_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_readouts = activation_readouts_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_readouts
            .iter()
            .any(|readout| readout.dossier_summary.total_dossiers > 0));
    }

    #[test]
    fn activation_briefing_items_summarize_readout_attention_sections() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );
        let items = activation_briefing_items_from_candidates(&[], candidates, &[]);

        assert!(items.iter().any(|item| {
            item.priority == 1 && item.kind == IntegrationActivationBriefingItemKind::Approval
        }));
        assert!(items.iter().any(|item| {
            item.priority == 2 && item.kind == IntegrationActivationBriefingItemKind::Blocker
        }));
        assert!(items.iter().any(|item| {
            item.priority == 2 && item.kind == IntegrationActivationBriefingItemKind::Dependency
        }));
        assert!(items.iter().any(|item| {
            item.priority == 3 && item.kind == IntegrationActivationBriefingItemKind::Activation
        }));
        assert!(items.iter().all(|item| item.integration_count() >= 1));

        let summary = IntegrationActivationBriefingSummary::from_items(items.iter());
        assert!(summary.total_items >= 4);
        assert_eq!(summary.unique_integrations, 3);
        assert!(summary.activation_items >= 1);
        assert!(summary.approval_items >= 1);
        assert!(summary.review_items >= 1);
        assert!(summary.blocker_items >= 1);
        assert!(summary.risk_items >= 1);
        assert!(summary.dependency_items >= 1);
        assert!(summary.items_requiring_attention >= 1);
        assert!(summary.total_actions > 0);
        assert!(summary.total_dossiers > 0);
        assert!(summary.total_evidence > 0);
        assert!(summary.total_risks > 0);
        assert!(summary.blocking_dependency_edges > 0);
        assert_eq!(summary.first_approval_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(1));
        assert_eq!(summary.first_activation_priority, Some(3));
        assert_eq!(summary.highest_policy_tier, PrivilegeTier::HighRisk);
        assert_eq!(
            summary.overall_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(summary.has_activation_work());
        assert!(summary.has_approval_ready_work());
        assert!(summary.has_review_work());
        assert!(summary.has_blockers());
        assert!(summary.has_risks());
        assert!(summary.has_dependency_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_items = activation_briefing_items_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_items
            .iter()
            .any(|item| item.kind == IntegrationActivationBriefingItemKind::Blocker));
    }

    #[test]
    fn activation_dashboard_cards_condense_readouts_and_briefing_sections() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );
        let cards = activation_dashboard_cards_from_candidates(&[], candidates, &[]);

        assert_eq!(cards.len(), 3);
        let approval_ready = cards.iter().find(|card| card.priority == 1).unwrap();
        assert!(approval_ready.has_approval_ready_work);
        assert!(approval_ready.has_review_work);
        assert!(approval_ready.requires_attention());
        assert!(approval_ready.briefing_item_count >= 2);
        assert_eq!(
            approval_ready.next_briefing_kind,
            Some(IntegrationActivationBriefingItemKind::Blocker)
        );

        let blocked = cards.iter().find(|card| card.priority == 2).unwrap();
        assert_eq!(
            blocked.health_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(blocked.has_blockers);
        assert!(blocked.has_risks);
        assert!(blocked.has_dependency_blockers);
        assert!(blocked.blocking_dependency_edge_count > 0);

        let ready = cards.iter().find(|card| card.priority == 3).unwrap();
        assert_eq!(
            ready.health_status,
            IntegrationActivationHealthStatus::Ready
        );
        assert!(ready.has_activation_work);
        assert!(!ready.has_approval_ready_work);
        assert!(cards.iter().all(|card| card.integration_count() >= 1));

        let summary = IntegrationActivationDashboardSummary::from_cards(cards.iter());
        assert_eq!(summary.total_cards, 3);
        assert_eq!(summary.unique_integrations, 3);
        assert_eq!(summary.ready_cards, 1);
        assert_eq!(summary.review_cards, 1);
        assert_eq!(summary.blocked_cards, 1);
        assert!(summary.total_briefing_items >= 4);
        assert!(summary.activation_items >= 1);
        assert!(summary.approval_items >= 1);
        assert!(summary.review_items >= 1);
        assert!(summary.blocker_items >= 1);
        assert!(summary.risk_items >= 1);
        assert!(summary.dependency_items >= 1);
        assert!(summary.cards_requiring_attention >= 1);
        assert!(summary.total_actions > 0);
        assert!(summary.total_dossiers > 0);
        assert!(summary.total_evidence > 0);
        assert!(summary.total_risks > 0);
        assert!(summary.blocking_dependency_edges > 0);
        assert_eq!(summary.first_approval_priority, Some(1));
        assert_eq!(summary.first_blocked_priority, Some(1));
        assert_eq!(summary.first_activation_priority, Some(3));
        assert_eq!(summary.first_attention_priority, Some(1));
        assert_eq!(summary.highest_policy_tier, PrivilegeTier::HighRisk);
        assert_eq!(
            summary.overall_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(summary.has_activation_work());
        assert!(summary.has_approval_ready_work());
        assert!(summary.has_review_work());
        assert!(summary.has_blockers());
        assert!(summary.has_risks());
        assert!(summary.has_dependency_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_cards = activation_dashboard_cards_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_cards
            .iter()
            .any(IntegrationActivationDashboardCard::requires_attention));
    }

    #[test]
    fn activation_timeline_milestones_order_dashboard_cards_by_wave() {
        let review_ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_ready_bridge"),
            display_name: "Review Ready Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_review_camera"),
            display_name: "Blocked Review Camera".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 2,
            missing_primitives: vec![PrimitiveFamily::CameraMedia],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HighRisk,
            local_only: false,
            cloud_required: true,
        };
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 3,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [review_ready_report, blocked_review_report, ready_report].iter(),
        );
        let milestones = activation_timeline_milestones_from_candidates(&[], candidates, &[]);

        assert_eq!(milestones.len(), 3);
        assert_eq!(milestones[0].sequence, 1);
        assert_eq!(milestones[0].priority, 1);
        assert_eq!(
            milestones[0].milestone_kind,
            Some(IntegrationActivationBriefingItemKind::Blocker)
        );
        assert!(milestones[0].requires_attention());
        assert!(milestones[0].has_approval_ready_work());
        assert_eq!(milestones[1].sequence, 2);
        assert_eq!(milestones[1].priority, 2);
        assert!(milestones[1].has_dependency_blockers());
        assert_eq!(milestones[2].sequence, 3);
        assert_eq!(milestones[2].priority, 3);
        assert!(milestones[2].has_activation_work());

        let summary = IntegrationActivationTimelineSummary::from_milestones(milestones.iter());
        assert_eq!(summary.total_milestones, 3);
        assert_eq!(summary.unique_integrations, 3);
        assert_eq!(summary.ready_milestones, 1);
        assert_eq!(summary.review_milestones, 1);
        assert_eq!(summary.blocked_milestones, 1);
        assert!(summary.blocker_milestones >= 1);
        assert!(summary.activation_milestones >= 1);
        assert!(summary.milestones_requiring_attention >= 1);
        assert!(summary.total_briefing_items >= 4);
        assert!(summary.total_actions > 0);
        assert!(summary.total_dossiers > 0);
        assert!(summary.total_evidence > 0);
        assert!(summary.total_risks > 0);
        assert!(summary.blocking_dependency_edges > 0);
        assert_eq!(summary.first_attention_sequence, Some(1));
        assert_eq!(summary.first_attention_priority, Some(1));
        assert_eq!(summary.first_approval_sequence, Some(1));
        assert_eq!(summary.first_blocked_sequence, Some(1));
        assert_eq!(summary.first_activation_sequence, Some(3));
        assert_eq!(summary.highest_policy_tier, PrivilegeTier::HighRisk);
        assert_eq!(
            summary.overall_status,
            IntegrationActivationHealthStatus::Blocked
        );
        assert!(summary.has_activation_work());
        assert!(summary.has_approval_ready_work());
        assert!(summary.has_review_work());
        assert!(summary.has_blockers());
        assert!(summary.has_risks());
        assert!(summary.has_dependency_blockers());
        assert!(summary.requires_attention());
        assert!(!summary.is_empty());

        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let catalog_milestones = activation_timeline_milestones_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        assert!(catalog_milestones
            .iter()
            .any(IntegrationActivationTimelineMilestone::requires_attention));
    }

    #[test]
    fn activation_actions_explain_ready_review_and_blocker_work() {
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 2,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_bridge"),
            display_name: "Review Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_sensor"),
            display_name: "Blocked Sensor".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 1,
            missing_primitives: vec![PrimitiveFamily::Mqtt],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::LowRisk,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [ready_report, review_report, blocked_report].iter(),
        );

        let actions = activation_actions_from_candidates(candidates.iter());

        assert_eq!(actions.len(), 5);
        assert_eq!(
            actions[0].kind,
            IntegrationActivationActionKind::ReviewPolicy
        );
        assert_eq!(
            actions[1].kind,
            IntegrationActivationActionKind::ProvidePrimitive
        );
        assert_eq!(
            actions[2].kind,
            IntegrationActivationActionKind::GrantCapability
        );
        assert_eq!(
            actions[3].kind,
            IntegrationActivationActionKind::EnableDependency
        );
        assert_eq!(
            actions[4].kind,
            IntegrationActivationActionKind::ActivateIntegration
        );
        assert!(actions[0].blocks_activation());
        assert!(actions[4].is_activation());

        let summary = IntegrationActivationActionSummary::from_actions(actions.iter());
        assert_eq!(summary.total_actions, 5);
        assert_eq!(summary.activate_integration_actions, 1);
        assert_eq!(summary.review_policy_actions, 1);
        assert_eq!(summary.provide_primitive_actions, 1);
        assert_eq!(summary.grant_capability_actions, 1);
        assert_eq!(summary.enable_dependency_actions, 1);
        assert_eq!(summary.unique_integrations, 3);
        assert_eq!(summary.actionable_integration_count, 1);
        assert_eq!(summary.blocked_integration_count, 2);
        assert_eq!(summary.first_action_priority, Some(1));
        assert_eq!(summary.first_activation_priority, Some(2));
        assert_eq!(summary.first_blocker_priority, Some(1));
        assert!(summary.has_activation_work());
        assert!(summary.has_blockers());
        assert!(summary.has_review_work());
        assert!(!summary.is_empty());
    }

    #[test]
    fn activation_agenda_groups_actions_by_rollout_wave() {
        let ready_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("read_only_probe"),
            display_name: "Read-only Probe".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 2,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::ReadOnly,
            local_only: true,
            cloud_required: false,
        };
        let review_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("review_bridge"),
            display_name: "Review Bridge".to_string(),
            activation_target: IntegrationActivationTarget::Direct,
            priority: 1,
            missing_primitives: Vec::new(),
            missing_capabilities: Vec::new(),
            missing_dependencies: Vec::new(),
            requires_human_review: true,
            highest_policy_tier: PrivilegeTier::HumanApproval,
            local_only: true,
            cloud_required: false,
        };
        let blocked_report = IntegrationReadinessReport {
            requested_integration_id: IntegrationId::trusted("blocked_sensor"),
            display_name: "Blocked Sensor".to_string(),
            activation_target: IntegrationActivationTarget::DelegatedIntegration(
                IntegrationId::trusted("mqtt"),
            ),
            priority: 1,
            missing_primitives: vec![PrimitiveFamily::Mqtt],
            missing_capabilities: vec![CapabilityId::trusted("smart_home.command.low_risk")],
            missing_dependencies: vec![IntegrationId::trusted("mqtt")],
            requires_human_review: false,
            highest_policy_tier: PrivilegeTier::LowRisk,
            local_only: true,
            cloud_required: false,
        };
        let candidates = activation_candidates_from_reports(
            [ready_report, review_report, blocked_report].iter(),
        );

        let agenda = activation_agenda_from_candidates(candidates);

        assert_eq!(agenda.len(), 2);
        assert_eq!(agenda[0].priority, 1);
        assert_eq!(agenda[0].candidate_summary.total_candidates, 2);
        assert_eq!(agenda[0].action_summary.total_actions, 4);
        assert_eq!(agenda[0].action_summary.review_policy_actions, 1);
        assert_eq!(agenda[0].action_summary.provide_primitive_actions, 1);
        assert!(agenda[0].has_blockers());
        assert!(agenda[0].has_review_work());
        assert!(!agenda[0].has_activation_work());
        assert_eq!(agenda[1].priority, 2);
        assert_eq!(agenda[1].candidate_summary.ready_to_activate_candidates, 1);
        assert_eq!(agenda[1].action_summary.activate_integration_actions, 1);
        assert!(agenda[1].has_activation_work());

        let summary = IntegrationActivationAgendaSummary::from_stages(agenda.iter());
        assert_eq!(summary.total_stages, 2);
        assert_eq!(summary.total_candidates, 3);
        assert_eq!(summary.total_actions, 5);
        assert_eq!(summary.stages_with_activation_work, 1);
        assert_eq!(summary.stages_with_blockers, 1);
        assert_eq!(summary.stages_with_review_work, 1);
        assert_eq!(summary.first_action_priority, Some(1));
        assert_eq!(summary.first_activation_priority, Some(2));
        assert_eq!(summary.first_blocker_priority, Some(1));
        assert_eq!(summary.candidate_summary.total_candidates, 3);
        assert_eq!(summary.action_summary.total_actions, 5);
        assert!(summary.has_activation_work());
        assert!(summary.has_blockers());
        assert!(summary.has_review_work());
        assert!(!summary.is_empty());
    }

    #[test]
    fn readiness_reports_include_delegated_integration_dependencies() {
        let catalog = first_party_catalog();
        let all_primitives = all_primitive_families().to_vec();
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let blocked = readiness_report_for_integration(
            &catalog,
            &IntegrationId::trusted("tplink_tapo"),
            &all_primitives,
            &allowed_capabilities,
            &[],
        )
        .unwrap();

        assert!(blocked.is_blocked());
        assert!(blocked.missing_dependency(&IntegrationId::trusted("tplink")));
        assert!(blocked.delegates_to_integration(&IntegrationId::trusted("tplink")));

        let ready = readiness_report_for_integration(
            &catalog,
            &IntegrationId::trusted("tplink_tapo"),
            &all_primitives,
            &allowed_capabilities,
            &[IntegrationId::trusted("tplink")],
        )
        .unwrap();

        assert!(ready.activation_ready());
        assert!(ready.missing_dependencies.is_empty());
    }

    #[test]
    fn priority_readiness_reports_track_rollout_wave_blockers() {
        let catalog = first_party_catalog();
        let available_primitives = vec![
            PrimitiveFamily::NormalizedModel,
            PrimitiveFamily::DiscoveryIndex,
            PrimitiveFamily::CommandMapping,
            PrimitiveFamily::CapabilityPolicy,
            PrimitiveFamily::Supervision,
        ];
        let allowed_capabilities = vec![CapabilityId::trusted("smart_home.read")];
        let reports = readiness_reports_at_or_before_priority(
            &catalog,
            1,
            &available_primitives,
            &allowed_capabilities,
            &[],
        );
        let hue = reports
            .iter()
            .find(|report| report.requested_integration_id == IntegrationId::trusted("hue"))
            .unwrap();
        let tasmota = reports
            .iter()
            .find(|report| report.requested_integration_id == IntegrationId::trusted("tasmota"))
            .unwrap();

        assert!(hue.missing_primitive(PrimitiveFamily::LocalPairing));
        assert!(hue.missing_capability(&CapabilityId::trusted("smart_home.command.light")));
        assert!(tasmota.missing_primitive(PrimitiveFamily::Mqtt));
        assert!(tasmota.missing_dependency(&IntegrationId::trusted("mqtt")));

        let summary = IntegrationReadinessSummary::from_reports(reports.iter());
        assert_eq!(summary.total_reports, reports.len());
        assert!(summary.has_blockers());
        assert!(summary.blocked_reports > 0);
        assert!(summary.reports_missing_primitives > 0);
        assert!(summary.reports_missing_capabilities > 0);
        assert!(summary.reports_missing_dependencies > 0);
        assert!(summary.unique_missing_primitives > 0);
        assert!(summary.unique_missing_capabilities > 0);
        assert!(summary.unique_missing_dependencies > 0);
        assert!(!summary.all_ready());

        let gaps = readiness_gap_inventory_from_reports(reports.iter());
        assert_eq!(gaps.total_reports, reports.len());
        assert_eq!(gaps.blocked_reports, summary.blocked_reports);
        assert!(gaps.has_gaps());
        assert!(gaps.primitive_gap_count() > 0);
        assert!(gaps.capability_gap_count() > 0);
        assert!(gaps.dependency_gap_count() > 0);
        assert!(gaps.primitive_gaps.first().unwrap().highest_priority <= 1);
        assert!(gaps
            .primitive_gaps
            .iter()
            .any(|gap| gap.primitive == PrimitiveFamily::LocalPairing
                && gap.integration_ids.contains(&IntegrationId::trusted("hue"))));
        assert!(gaps
            .capability_gaps
            .iter()
            .any(|gap| gap.capability_id == CapabilityId::trusted("smart_home.command.light")));
        assert!(gaps.dependency_gaps.iter().any(|gap| gap.integration_id
            == IntegrationId::trusted("mqtt")
            && gap
                .requested_integration_ids
                .contains(&IntegrationId::trusted("tasmota"))));
    }

    #[test]
    fn activation_dependency_graph_tracks_satisfied_and_blocking_edges() {
        let catalog = first_party_catalog();
        let available_primitives = all_primitive_families().to_vec();
        let allowed_capabilities = vec![
            CapabilityId::trusted("smart_home.read"),
            CapabilityId::trusted("smart_home.command.light"),
        ];
        let graph = activation_dependency_graph_at_or_before_priority(
            &catalog,
            2,
            &available_primitives,
            &allowed_capabilities,
            &[IntegrationId::trusted("mqtt")],
        );

        assert!(!graph.is_empty());
        assert!(graph.summary.has_dependency_edges());
        assert!(graph.summary.satisfied_edges > 0);
        assert!(graph.summary.blocking_edges > 0);
        assert!(graph.summary.nodes_with_dependencies > 0);
        assert!(graph.summary.nodes_with_dependents > 0);
        assert!(graph.summary.nodes_with_missing_dependencies > 0);

        let tasmota = graph
            .nodes
            .iter()
            .find(|node| node.integration_id == IntegrationId::trusted("tasmota"))
            .unwrap();
        assert!(tasmota
            .depends_on_integrations
            .contains(&IntegrationId::trusted("mqtt")));
        assert!(tasmota.missing_dependencies.is_empty());

        let tapo = graph
            .nodes
            .iter()
            .find(|node| node.integration_id == IntegrationId::trusted("tplink_tapo"))
            .unwrap();
        assert!(tapo
            .missing_dependencies
            .contains(&IntegrationId::trusted("tplink")));

        let satisfied = graph
            .edges
            .iter()
            .find(|edge| {
                edge.dependent_integration_id == IntegrationId::trusted("tasmota")
                    && edge.dependency_integration_id == IntegrationId::trusted("mqtt")
            })
            .unwrap();
        assert!(satisfied.satisfied);
        assert!(!satisfied.blocks_activation);

        let blocked = graph
            .edges
            .iter()
            .find(|edge| {
                edge.dependent_integration_id == IntegrationId::trusted("tplink_tapo")
                    && edge.dependency_integration_id == IntegrationId::trusted("tplink")
            })
            .unwrap();
        assert!(!blocked.satisfied);
        assert!(blocked.blocks_activation);
        assert_eq!(
            blocked.dependency_display_name.as_deref(),
            Some("TP-Link Smart Home")
        );
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

    #[test]
    fn policy_surface_inventory_rolls_up_review_boundaries() {
        let catalog = first_party_catalog();
        let inventory = policy_surface_inventory(&catalog);
        let entry_access = inventory
            .iter()
            .find(|item| item.surface == IntegrationPolicySurface::EntryAccess)
            .unwrap();
        let credentialed_cloud = inventory
            .iter()
            .find(|item| item.surface == IntegrationPolicySurface::CredentialedCloud)
            .unwrap();
        let summary = IntegrationPolicySurfaceSummary::from_inventory(&inventory);

        assert_eq!(entry_access.required_tier, PrivilegeTier::HighRisk);
        assert!(entry_access.includes_integration(&IntegrationId::trusted("zwave")));
        assert!(entry_access.human_review_entry_count >= 1);
        assert_eq!(
            credentialed_cloud.required_tier,
            PrivilegeTier::HumanApproval
        );
        assert!(credentialed_cloud.cloud_entry_count >= 1);
        assert!(summary.total_surfaces >= 4);
        assert!(summary.unique_integrations >= 4);
        assert_eq!(summary.highest_policy_tier, PrivilegeTier::HighRisk);
        assert!(summary.has_review_work());
        assert!(summary.has_high_risk_surface());
        assert_eq!(summary.first_review_priority, Some(0));
    }

    #[test]
    fn catalog_query_composes_local_priority_and_primitive_filters() {
        let catalog = first_party_catalog();
        let query = IntegrationCatalogQuery::new()
            .include_virtual_aliases(false)
            .local_only(true)
            .at_or_before_priority(1)
            .requiring_primitive(PrimitiveFamily::Mqtt)
            .sorted_by(IntegrationCatalogSort::PriorityThenName);
        let results = query_integrations(&catalog, &query);

        assert!(results
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("mqtt")));
        assert!(results
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("tasmota")));
        assert!(!results
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("hue")));
        assert!(results.iter().all(|entry| {
            !entry.is_virtual()
                && entry_local_only(entry)
                && entry.priority <= 1
                && entry.requires_primitive(PrimitiveFamily::Mqtt)
        }));
    }

    #[test]
    fn catalog_query_can_bound_cloud_policy_results() {
        let catalog = first_party_catalog();
        let query = IntegrationCatalogQuery::new()
            .cloud_required(true)
            .with_policy_surface(IntegrationPolicySurface::CredentialedCloud)
            .sorted_by(IntegrationCatalogSort::Name)
            .limited_to(2);
        let results = query_integrations(&catalog, &query);

        assert_eq!(results.len(), 2);
        assert!(results
            .windows(2)
            .all(|window| window[0].display_name <= window[1].display_name));
        assert!(results.iter().all(|entry| {
            entry_cloud_required(entry)
                && entry.has_policy_surface(IntegrationPolicySurface::CredentialedCloud)
        }));
    }

    #[test]
    fn catalog_query_protocol_filters_include_standard_aliases() {
        let catalog = first_party_catalog();
        let query = IntegrationCatalogQuery::new()
            .with_category(IntegrationCategory::VirtualAlias)
            .with_protocol_family(ProtocolFamily::ZWave);
        let results = query_integrations(&catalog, &query);

        assert!(results.iter().all(|entry| entry.is_virtual()));
        assert!(results
            .iter()
            .any(|entry| entry.integration_id == IntegrationId::trusted("ultraloq")));
    }
}
