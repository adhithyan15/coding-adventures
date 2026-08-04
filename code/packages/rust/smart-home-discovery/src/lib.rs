//! Discovery records and mDNS scan helpers for the D23 smart-home runtime.
//!
//! Discovery workers can use this crate to normalize LAN, radio, USB, MQTT,
//! cloud, webhook, manual, or simulator findings before a runtime decides
//! whether to pair, ignore, or supervise a bridge. Most helpers are pure data
//! shaping; the mDNS scan functions use `udp-client` for bounded datagram I/O
//! while leaving vendor-specific interpretation to integration crates.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier,
};
use smart_home_integration_catalog::{AuthMode, DiscoveryMechanism, IntegrationCatalogEntry};
use std::{cmp::Ordering, collections::BTreeMap, fmt, net::Ipv6Addr, time::Duration};
use udp_client::{send_to_and_collect, UdpDiscoveryEndpoint, UdpOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    EmptyField {
        field: &'static str,
    },
    DuplicateTxtKey {
        key: String,
    },
    InvalidMdnsMessage {
        message: String,
    },
    InvalidMdnsScanOption {
        field: &'static str,
        message: String,
    },
    MdnsTransport {
        message: String,
    },
    WorkerIntegrationMismatch {
        worker_integration_id: String,
        record_integration_id: String,
    },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "{field} must not be empty"),
            Self::DuplicateTxtKey { key } => {
                write!(f, "mDNS TXT key `{key}` appears more than once")
            }
            Self::InvalidMdnsMessage { message } => {
                write!(f, "invalid mDNS message: {message}")
            }
            Self::InvalidMdnsScanOption { field, message } => {
                write!(f, "invalid mDNS scan option `{field}`: {message}")
            }
            Self::MdnsTransport { message } => write!(f, "mDNS transport failed: {message}"),
            Self::WorkerIntegrationMismatch {
                worker_integration_id,
                record_integration_id,
            } => write!(
                f,
                "discovery worker for `{worker_integration_id}` cannot report `{record_integration_id}` records"
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoverySource {
    Mdns,
    WsDiscovery,
    Ssdp,
    UdpMulticast,
    UdpBroadcast,
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
            Self::WsDiscovery => "ws_discovery",
            Self::Ssdp => "ssdp",
            Self::UdpMulticast => "udp_multicast",
            Self::UdpBroadcast => "udp_broadcast",
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
            Self::WsDiscovery => 85,
            Self::Mdns => 80,
            Self::Ssdp => 70,
            Self::UdpMulticast => 70,
            Self::UdpBroadcast => 70,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiscoveryFingerprint(String);

impl DiscoveryFingerprint {
    pub fn new(value: impl Into<String>) -> Result<Self, DiscoveryError> {
        Ok(Self(non_empty("discovery_fingerprint", value)?))
    }

    pub fn for_record(record: &DiscoveryRecord) -> Self {
        Self(format!(
            "{}:{}:{}",
            record.source.as_str(),
            record.integration_id.as_str(),
            record.native_bridge_id
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiscoveryFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySignalStatus {
    Fresh,
    Stale,
    Expired,
}

impl DiscoverySignalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverySignal {
    pub fingerprint: DiscoveryFingerprint,
    pub integration_id: IntegrationId,
    pub native_bridge_id: String,
    pub source: DiscoverySource,
    pub confidence: DiscoveryConfidence,
    pub observed_at_ms: u64,
    pub stale_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub metadata: Vec<Metadata>,
}

impl DiscoverySignal {
    pub fn from_record(record: &DiscoveryRecord, ttl_ms: u64) -> Self {
        Self {
            fingerprint: DiscoveryFingerprint::for_record(record),
            integration_id: record.integration_id.clone(),
            native_bridge_id: record.native_bridge_id.clone(),
            source: record.source,
            confidence: record.confidence,
            observed_at_ms: record.discovered_at_ms,
            stale_at_ms: record.discovered_at_ms.saturating_add(ttl_ms),
            expires_at_ms: record.expires_at_ms,
            metadata: record.metadata.clone(),
        }
    }

    pub fn age_ms_at(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.observed_at_ms)
    }

    pub fn status_at(&self, now_ms: u64) -> DiscoverySignalStatus {
        if self
            .expires_at_ms
            .is_some_and(|expires_at_ms| now_ms >= expires_at_ms)
        {
            DiscoverySignalStatus::Expired
        } else if now_ms >= self.stale_at_ms {
            DiscoverySignalStatus::Stale
        } else {
            DiscoverySignalStatus::Fresh
        }
    }

    pub fn next_transition_at_ms(&self, now_ms: u64) -> Option<u64> {
        match self.status_at(now_ms) {
            DiscoverySignalStatus::Fresh => [Some(self.stale_at_ms), self.expires_at_ms]
                .into_iter()
                .flatten()
                .filter(|deadline| *deadline > now_ms)
                .min(),
            DiscoverySignalStatus::Stale => self
                .expires_at_ms
                .filter(|expires_at_ms| *expires_at_ms > now_ms),
            DiscoverySignalStatus::Expired => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverySignalSummary {
    pub fresh: usize,
    pub stale: usize,
    pub expired: usize,
    pub next_transition_at_ms: Option<u64>,
}

impl DiscoverySignalSummary {
    pub fn from_signals(signals: &[DiscoverySignal], now_ms: u64) -> Self {
        let mut summary = Self {
            fresh: 0,
            stale: 0,
            expired: 0,
            next_transition_at_ms: None,
        };

        for signal in signals {
            match signal.status_at(now_ms) {
                DiscoverySignalStatus::Fresh => summary.fresh += 1,
                DiscoverySignalStatus::Stale => summary.stale += 1,
                DiscoverySignalStatus::Expired => summary.expired += 1,
            }

            if let Some(deadline) = signal.next_transition_at_ms(now_ms) {
                summary.next_transition_at_ms = Some(
                    summary
                        .next_transition_at_ms
                        .map_or(deadline, |current| current.min(deadline)),
                );
            }
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRecordSummary {
    pub total: usize,
    pub with_address: usize,
    pub fresh: usize,
    pub stale: usize,
    pub expired: usize,
    pub by_source: BTreeMap<DiscoverySource, usize>,
    pub by_confidence: BTreeMap<DiscoveryConfidence, usize>,
    pub by_pairing_requirement: BTreeMap<PairingRequirement, usize>,
}

impl DiscoveryRecordSummary {
    pub fn from_records<'a>(
        records: impl IntoIterator<Item = &'a DiscoveryRecord>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Self {
        let mut summary = Self {
            total: 0,
            with_address: 0,
            fresh: 0,
            stale: 0,
            expired: 0,
            by_source: BTreeMap::new(),
            by_confidence: BTreeMap::new(),
            by_pairing_requirement: BTreeMap::new(),
        };

        for record in records {
            summary.total += 1;
            if record.has_address() {
                summary.with_address += 1;
            }
            match record.signal(ttl_ms).status_at(now_ms) {
                DiscoverySignalStatus::Fresh => summary.fresh += 1,
                DiscoverySignalStatus::Stale => summary.stale += 1,
                DiscoverySignalStatus::Expired => summary.expired += 1,
            }
            *summary.by_source.entry(record.source).or_insert(0) += 1;
            *summary.by_confidence.entry(record.confidence).or_insert(0) += 1;
            *summary
                .by_pairing_requirement
                .entry(record.pairing_requirement)
                .or_insert(0) += 1;
        }

        summary
    }

    pub fn count_for_source(&self, source: DiscoverySource) -> usize {
        self.by_source.get(&source).copied().unwrap_or(0)
    }

    pub fn count_for_confidence(&self, confidence: DiscoveryConfidence) -> usize {
        self.by_confidence.get(&confidence).copied().unwrap_or(0)
    }

    pub fn count_for_pairing_requirement(&self, requirement: PairingRequirement) -> usize {
        self.by_pairing_requirement
            .get(&requirement)
            .copied()
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiscoveryWorkerId(String);

impl DiscoveryWorkerId {
    pub fn new(value: impl Into<String>) -> Result<Self, DiscoveryError> {
        Ok(Self(non_empty("discovery_worker_id", value)?))
    }

    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiscoveryWorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoveryWorkerKind {
    MdnsScan,
    CloudFallback,
    ManualSeed,
    Composite,
    Simulator,
}

impl DiscoveryWorkerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MdnsScan => "mdns_scan",
            Self::CloudFallback => "cloud_fallback",
            Self::ManualSeed => "manual_seed",
            Self::Composite => "composite",
            Self::Simulator => "simulator",
        }
    }
}

impl fmt::Display for DiscoveryWorkerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoveryWorkerRunStatus {
    Completed,
    Partial,
    Failed,
}

impl DiscoveryWorkerRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Partial => "partial",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for DiscoveryWorkerRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryWorkerFailure {
    pub source: DiscoverySource,
    pub message: String,
    pub metadata: Vec<Metadata>,
}

impl DiscoveryWorkerFailure {
    pub fn new(
        source: DiscoverySource,
        message: impl Into<String>,
    ) -> Result<Self, DiscoveryError> {
        Ok(Self {
            source,
            message: non_empty("discovery_worker_failure.message", message)?,
            metadata: Vec::new(),
        })
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryWorkerRun {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub kind: DiscoveryWorkerKind,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<DiscoveryWorkerFailure>,
    pub metadata: Vec<Metadata>,
}

impl DiscoveryWorkerRun {
    pub fn new(
        worker_id: DiscoveryWorkerId,
        integration_id: IntegrationId,
        kind: DiscoveryWorkerKind,
        started_at_ms: u64,
        completed_at_ms: u64,
    ) -> Self {
        Self {
            worker_id,
            integration_id,
            kind,
            started_at_ms,
            completed_at_ms,
            records: Vec::new(),
            failures: Vec::new(),
            metadata: Vec::new(),
        }
    }

    pub fn push_record(&mut self, record: DiscoveryRecord) -> Result<(), DiscoveryError> {
        if record.integration_id != self.integration_id {
            return Err(DiscoveryError::WorkerIntegrationMismatch {
                worker_integration_id: self.integration_id.as_str().to_string(),
                record_integration_id: record.integration_id.as_str().to_string(),
            });
        }
        self.records.push(record);
        Ok(())
    }

    pub fn push_failure(&mut self, failure: DiscoveryWorkerFailure) {
        self.failures.push(failure);
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    pub fn duration_ms(&self) -> u64 {
        self.completed_at_ms.saturating_sub(self.started_at_ms)
    }

    pub fn status(&self) -> DiscoveryWorkerRunStatus {
        match (self.records.is_empty(), self.failures.is_empty()) {
            (false, true) | (true, true) => DiscoveryWorkerRunStatus::Completed,
            (false, false) => DiscoveryWorkerRunStatus::Partial,
            (true, false) => DiscoveryWorkerRunStatus::Failed,
        }
    }

    pub fn summary_at(
        &self,
        now_ms: u64,
        ttl_ms: u64,
        inserted_count: usize,
        replaced_count: usize,
        ignored_count: usize,
    ) -> DiscoveryWorkerRunSummary {
        DiscoveryWorkerRunSummary::from_run_at(
            self,
            now_ms,
            ttl_ms,
            inserted_count,
            replaced_count,
            ignored_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryWorkerRunSummary {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub kind: DiscoveryWorkerKind,
    pub status: DiscoveryWorkerRunStatus,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub duration_ms: u64,
    pub record_count: usize,
    pub failure_count: usize,
    pub inserted_count: usize,
    pub replaced_count: usize,
    pub ignored_count: usize,
    pub record_summary: DiscoveryRecordSummary,
    pub signal_summary: DiscoverySignalSummary,
}

impl DiscoveryWorkerRunSummary {
    pub fn from_run_at(
        run: &DiscoveryWorkerRun,
        now_ms: u64,
        ttl_ms: u64,
        inserted_count: usize,
        replaced_count: usize,
        ignored_count: usize,
    ) -> Self {
        let signals = run
            .records
            .iter()
            .map(|record| record.signal(ttl_ms))
            .collect::<Vec<_>>();
        Self {
            worker_id: run.worker_id.clone(),
            integration_id: run.integration_id.clone(),
            kind: run.kind,
            status: run.status(),
            started_at_ms: run.started_at_ms,
            completed_at_ms: run.completed_at_ms,
            duration_ms: run.duration_ms(),
            record_count: run.records.len(),
            failure_count: run.failures.len(),
            inserted_count,
            replaced_count,
            ignored_count,
            record_summary: DiscoveryRecordSummary::from_records(
                run.records.iter(),
                now_ms,
                ttl_ms,
            ),
            signal_summary: DiscoverySignalSummary::from_signals(&signals, now_ms),
        }
    }

    pub fn accepted_count(&self) -> usize {
        self.inserted_count + self.replaced_count
    }

    pub fn has_catalog_changes(&self) -> bool {
        self.accepted_count() > 0
    }

    pub fn has_failures(&self) -> bool {
        self.failure_count > 0
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
pub struct CatalogDiscoveryHint {
    pub integration_id: IntegrationId,
    pub display_name: String,
    pub priority: u8,
    pub protocol_family: ProtocolFamily,
    pub discovery_mechanism: DiscoveryMechanism,
    pub source: DiscoverySource,
    pub transport: BridgeTransport,
    pub pairing_requirement: PairingRequirement,
}

impl CatalogDiscoveryHint {
    pub fn from_entry(entry: &IntegrationCatalogEntry) -> Vec<Self> {
        let protocol_family = primary_protocol_for_entry(entry);
        let pairing_requirement = pairing_requirement_for_auth_modes(&entry.auth_modes);

        entry
            .discovery_mechanisms
            .iter()
            .copied()
            .map(|discovery_mechanism| Self {
                integration_id: entry.integration_id.clone(),
                display_name: entry.display_name.clone(),
                priority: entry.priority,
                protocol_family: protocol_family.clone(),
                discovery_mechanism,
                source: source_for_discovery_mechanism(discovery_mechanism),
                transport: transport_for_discovery_mechanism(discovery_mechanism),
                pairing_requirement,
            })
            .collect()
    }

    pub fn to_record(
        &self,
        native_bridge_id: impl Into<String>,
        discovered_at_ms: u64,
    ) -> Result<DiscoveryRecord, DiscoveryError> {
        DiscoveryRecord::new(
            self.integration_id.clone(),
            self.protocol_family.clone(),
            native_bridge_id,
            self.source,
            self.transport,
            discovered_at_ms,
        )
        .map(|record| {
            record
                .with_display_name(self.display_name.clone())
                .with_pairing_requirement(self.pairing_requirement)
                .with_metadata("smart_home.discovery.catalog_hint", "true")
                .with_metadata(
                    "smart_home.discovery.mechanism",
                    discovery_mechanism_name(self.discovery_mechanism),
                )
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoveryPairingAction {
    Ready,
    PressPhysicalButton,
    EnterLocalCode,
    ProvideCredentials,
    CompleteOAuth2,
    InstallCertificate,
    StartRadioInclusion,
    ConfigureMqttCredentials,
    InvestigateUnknownRequirement,
}

impl DiscoveryPairingAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::PressPhysicalButton => "press_physical_button",
            Self::EnterLocalCode => "enter_local_code",
            Self::ProvideCredentials => "provide_credentials",
            Self::CompleteOAuth2 => "complete_oauth2",
            Self::InstallCertificate => "install_certificate",
            Self::StartRadioInclusion => "start_radio_inclusion",
            Self::ConfigureMqttCredentials => "configure_mqtt_credentials",
            Self::InvestigateUnknownRequirement => "investigate_unknown_requirement",
        }
    }

    pub fn requires_human_action(self) -> bool {
        !matches!(self, Self::Ready)
    }
}

impl fmt::Display for DiscoveryPairingAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryPairingTarget {
    pub fingerprint: DiscoveryFingerprint,
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub native_bridge_id: String,
    pub display_name: Option<String>,
    pub priority: u8,
    pub source: DiscoverySource,
    pub confidence: DiscoveryConfidence,
    pub signal_status: DiscoverySignalStatus,
    pub pairing_requirement: PairingRequirement,
    pub action: DiscoveryPairingAction,
    pub address: Option<String>,
    pub discovered_at_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiscoveryPairingPlanSort {
    #[default]
    PlanRank,
    NewestFirst,
    IntegrationThenBridge,
    SourcePreference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryPairingPlanOptions {
    pub integration_ids: Vec<IntegrationId>,
    pub sources: Vec<DiscoverySource>,
    pub signal_statuses: Vec<DiscoverySignalStatus>,
    pub pairing_requirements: Vec<PairingRequirement>,
    pub actions: Vec<DiscoveryPairingAction>,
    pub priority_at_or_before: Option<u8>,
    pub requires_human_action: Option<bool>,
    pub actionable_only: bool,
    pub sort: DiscoveryPairingPlanSort,
    pub limit: Option<usize>,
}

impl Default for DiscoveryPairingPlanOptions {
    fn default() -> Self {
        Self {
            integration_ids: Vec::new(),
            sources: Vec::new(),
            signal_statuses: Vec::new(),
            pairing_requirements: Vec::new(),
            actions: Vec::new(),
            priority_at_or_before: None,
            requires_human_action: None,
            actionable_only: false,
            sort: DiscoveryPairingPlanSort::PlanRank,
            limit: None,
        }
    }
}

impl DiscoveryPairingPlanOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_integration(mut self, integration_id: IntegrationId) -> Self {
        self.integration_ids.push(integration_id);
        self
    }

    pub fn with_source(mut self, source: DiscoverySource) -> Self {
        self.sources.push(source);
        self
    }

    pub fn with_signal_status(mut self, status: DiscoverySignalStatus) -> Self {
        self.signal_statuses.push(status);
        self
    }

    pub fn with_pairing_requirement(mut self, requirement: PairingRequirement) -> Self {
        self.pairing_requirements.push(requirement);
        self
    }

    pub fn with_action(mut self, action: DiscoveryPairingAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn at_or_before_priority(mut self, priority: u8) -> Self {
        self.priority_at_or_before = Some(priority);
        self
    }

    pub fn requiring_human_action(mut self, requires_human_action: bool) -> Self {
        self.requires_human_action = Some(requires_human_action);
        self
    }

    pub fn actionable_only(mut self, actionable_only: bool) -> Self {
        self.actionable_only = actionable_only;
        self
    }

    pub fn sorted_by(mut self, sort: DiscoveryPairingPlanSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn matches_target(&self, target: &DiscoveryPairingTarget) -> bool {
        if self.actionable_only && !target.is_actionable() {
            return false;
        }
        if let Some(priority) = self.priority_at_or_before {
            if target.priority > priority {
                return false;
            }
        }
        if let Some(requires_human_action) = self.requires_human_action {
            if target.requires_human_action() != requires_human_action {
                return false;
            }
        }
        if !self.integration_ids.is_empty()
            && !self
                .integration_ids
                .iter()
                .any(|integration_id| integration_id == &target.integration_id)
        {
            return false;
        }
        if !self.sources.is_empty() && !self.sources.contains(&target.source) {
            return false;
        }
        if !self.signal_statuses.is_empty() && !self.signal_statuses.contains(&target.signal_status)
        {
            return false;
        }
        if !self.pairing_requirements.is_empty()
            && !self
                .pairing_requirements
                .contains(&target.pairing_requirement)
        {
            return false;
        }
        if !self.actions.is_empty() && !self.actions.contains(&target.action) {
            return false;
        }

        true
    }
}

impl DiscoveryPairingTarget {
    pub fn requires_human_action(&self) -> bool {
        self.action.requires_human_action()
    }

    pub fn is_actionable(&self) -> bool {
        self.signal_status != DiscoverySignalStatus::Expired
            && self.action != DiscoveryPairingAction::InvestigateUnknownRequirement
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryPairingPlan {
    pub generated_at_ms: u64,
    pub targets: Vec<DiscoveryPairingTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryPairingPlanSummary {
    pub generated_at_ms: u64,
    pub total: usize,
    pub actionable: usize,
    pub ready: usize,
    pub requires_human_action: usize,
    pub blocked_unknown_requirement: usize,
    pub fresh: usize,
    pub stale: usize,
    pub by_source: BTreeMap<DiscoverySource, usize>,
    pub by_pairing_requirement: BTreeMap<PairingRequirement, usize>,
    pub by_action: BTreeMap<DiscoveryPairingAction, usize>,
    pub next_actionable_target: Option<DiscoveryPairingTarget>,
}

impl DiscoveryPairingPlanSummary {
    pub fn from_plan(plan: &DiscoveryPairingPlan) -> Self {
        let mut summary = Self {
            generated_at_ms: plan.generated_at_ms,
            total: 0,
            actionable: 0,
            ready: 0,
            requires_human_action: 0,
            blocked_unknown_requirement: 0,
            fresh: 0,
            stale: 0,
            by_source: BTreeMap::new(),
            by_pairing_requirement: BTreeMap::new(),
            by_action: BTreeMap::new(),
            next_actionable_target: None,
        };

        for target in &plan.targets {
            summary.total += 1;
            if target.is_actionable() {
                summary.actionable += 1;
                if summary.next_actionable_target.is_none() {
                    summary.next_actionable_target = Some(target.clone());
                }
            }
            if target.action == DiscoveryPairingAction::Ready {
                summary.ready += 1;
            }
            if target.requires_human_action() {
                summary.requires_human_action += 1;
            }
            if target.action == DiscoveryPairingAction::InvestigateUnknownRequirement {
                summary.blocked_unknown_requirement += 1;
            }
            match target.signal_status {
                DiscoverySignalStatus::Fresh => summary.fresh += 1,
                DiscoverySignalStatus::Stale => summary.stale += 1,
                DiscoverySignalStatus::Expired => {}
            }
            *summary.by_source.entry(target.source).or_insert(0) += 1;
            *summary
                .by_pairing_requirement
                .entry(target.pairing_requirement)
                .or_insert(0) += 1;
            *summary.by_action.entry(target.action).or_insert(0) += 1;
        }

        summary
    }

    pub fn count_for_source(&self, source: DiscoverySource) -> usize {
        self.by_source.get(&source).copied().unwrap_or(0)
    }

    pub fn count_for_pairing_requirement(&self, requirement: PairingRequirement) -> usize {
        self.by_pairing_requirement
            .get(&requirement)
            .copied()
            .unwrap_or(0)
    }

    pub fn count_for_action(&self, action: DiscoveryPairingAction) -> usize {
        self.by_action.get(&action).copied().unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

impl DiscoveryPairingPlan {
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.targets.len()
    }

    pub fn actionable_targets(&self) -> Vec<&DiscoveryPairingTarget> {
        self.targets
            .iter()
            .filter(|target| target.is_actionable())
            .collect()
    }

    pub fn targets_for_integration(
        &self,
        integration_id: &IntegrationId,
    ) -> Vec<&DiscoveryPairingTarget> {
        self.targets
            .iter()
            .filter(|target| &target.integration_id == integration_id)
            .collect()
    }

    pub fn summary(&self) -> DiscoveryPairingPlanSummary {
        DiscoveryPairingPlanSummary::from_plan(self)
    }

    pub fn query(&self, options: &DiscoveryPairingPlanOptions) -> DiscoveryPairingPlan {
        let mut targets = self
            .targets
            .iter()
            .filter(|target| options.matches_target(target))
            .cloned()
            .collect::<Vec<_>>();
        sort_pairing_targets(&mut targets, options.sort);
        if let Some(limit) = options.limit {
            targets.truncate(limit);
        }
        DiscoveryPairingPlan {
            generated_at_ms: self.generated_at_ms,
            targets,
        }
    }
}

pub fn catalog_discovery_hints(catalog: &[IntegrationCatalogEntry]) -> Vec<CatalogDiscoveryHint> {
    catalog
        .iter()
        .flat_map(CatalogDiscoveryHint::from_entry)
        .collect()
}

pub fn discovery_hints_for_integration(
    catalog: &[IntegrationCatalogEntry],
    integration_id: &IntegrationId,
) -> Vec<CatalogDiscoveryHint> {
    catalog
        .iter()
        .filter(|entry| &entry.integration_id == integration_id)
        .flat_map(CatalogDiscoveryHint::from_entry)
        .collect()
}

pub fn discovery_hints_for_source(
    catalog: &[IntegrationCatalogEntry],
    source: DiscoverySource,
) -> Vec<CatalogDiscoveryHint> {
    catalog_discovery_hints(catalog)
        .into_iter()
        .filter(|hint| hint.source == source)
        .collect()
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

    pub fn fingerprint(&self) -> DiscoveryFingerprint {
        DiscoveryFingerprint::for_record(self)
    }

    pub fn signal(&self, ttl_ms: u64) -> DiscoverySignal {
        DiscoverySignal::from_record(self, ttl_ms)
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

pub const MDNS_DNS_CLASS_IN: u16 = 1;
pub const MDNS_DNS_TYPE_A: u16 = 1;
pub const MDNS_DNS_TYPE_PTR: u16 = 12;
pub const MDNS_DNS_TYPE_TXT: u16 = 16;
pub const MDNS_DNS_TYPE_AAAA: u16 = 28;
pub const MDNS_DNS_TYPE_SRV: u16 = 33;
pub const MDNS_DEFAULT_MAX_DATAGRAM_SIZE: usize = 1500;
pub const MDNS_DEFAULT_MAX_RESPONSES: usize = 32;
pub const MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY: &str = "smart_home.discovery.service_type";
pub const MDNS_UNICAST_RESPONSE_CLASS_BIT: u16 = 0x8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MdnsScanNetwork {
    Ipv4,
    Ipv6,
}

impl MdnsScanNetwork {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
        }
    }
}

impl fmt::Display for MdnsScanNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsWorkerScanRequest {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub network_interface: String,
    pub network: MdnsScanNetwork,
    pub service_type: String,
    pub discovered_at_ms: u64,
    pub timeout: Duration,
    pub max_responses: usize,
    pub max_datagram_size: usize,
    pub unicast_response: bool,
    pub metadata: Vec<Metadata>,
}

impl MdnsWorkerScanRequest {
    pub fn new(
        worker_id: DiscoveryWorkerId,
        integration_id: IntegrationId,
        network_interface: impl Into<String>,
        network: MdnsScanNetwork,
        service_type: impl Into<String>,
        discovered_at_ms: u64,
        timeout: Duration,
    ) -> Result<Self, DiscoveryError> {
        Ok(Self {
            worker_id,
            integration_id,
            network_interface: non_empty("mdns.network_interface", network_interface)?,
            network,
            service_type: non_empty("mdns.service_type", service_type)?,
            discovered_at_ms,
            timeout,
            max_responses: MDNS_DEFAULT_MAX_RESPONSES,
            max_datagram_size: MDNS_DEFAULT_MAX_DATAGRAM_SIZE,
            unicast_response: true,
            metadata: Vec::new(),
        })
    }

    pub fn with_max_responses(mut self, max_responses: usize) -> Self {
        self.max_responses = max_responses;
        self
    }

    pub fn with_max_datagram_size(mut self, max_datagram_size: usize) -> Self {
        self.max_datagram_size = max_datagram_size;
        self
    }

    pub fn multicast_response(mut self) -> Self {
        self.unicast_response = false;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn options(&self) -> Result<MdnsScanOptions, DiscoveryError> {
        let mut options = MdnsScanOptions::new(
            self.service_type.clone(),
            self.discovered_at_ms,
            self.timeout,
        )?
        .with_max_responses(self.max_responses)
        .with_max_datagram_size(self.max_datagram_size);
        if !self.unicast_response {
            options = options.multicast_response();
        }
        Ok(options)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsWorkerScanPlan {
    pub generated_at_ms: u64,
    pub requests: Vec<MdnsWorkerScanRequest>,
}

impl MdnsWorkerScanPlan {
    pub fn new(generated_at_ms: u64) -> Self {
        Self {
            generated_at_ms,
            requests: Vec::new(),
        }
    }

    pub fn push_request(&mut self, request: MdnsWorkerScanRequest) {
        self.requests.push(request);
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    pub fn requests_for_worker(
        &self,
        worker_id: &DiscoveryWorkerId,
    ) -> Vec<&MdnsWorkerScanRequest> {
        self.requests
            .iter()
            .filter(|request| &request.worker_id == worker_id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsQuestion {
    pub service_type: String,
    pub unicast_response: bool,
}

impl MdnsQuestion {
    pub fn new(service_type: impl Into<String>) -> Result<Self, DiscoveryError> {
        Ok(Self {
            service_type: non_empty("mdns.service_type", service_type)?,
            unicast_response: true,
        })
    }

    pub fn multicast_response(mut self) -> Self {
        self.unicast_response = false;
        self
    }

    pub fn to_query_packet(&self) -> Result<Vec<u8>, DiscoveryError> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&0u16.to_be_bytes());
        packet.extend_from_slice(&0u16.to_be_bytes());
        packet.extend_from_slice(&1u16.to_be_bytes());
        packet.extend_from_slice(&0u16.to_be_bytes());
        packet.extend_from_slice(&0u16.to_be_bytes());
        packet.extend_from_slice(&0u16.to_be_bytes());
        encode_dns_name(&self.service_type, &mut packet)?;
        packet.extend_from_slice(&MDNS_DNS_TYPE_PTR.to_be_bytes());
        let class = if self.unicast_response {
            MDNS_DNS_CLASS_IN | MDNS_UNICAST_RESPONSE_CLASS_BIT
        } else {
            MDNS_DNS_CLASS_IN
        };
        packet.extend_from_slice(&class.to_be_bytes());
        Ok(packet)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsResponsePacket {
    pub source: Option<String>,
    pub payload: Vec<u8>,
}

impl MdnsResponsePacket {
    pub fn new(payload: impl Into<Vec<u8>>) -> Self {
        Self {
            source: None,
            payload: payload.into(),
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsScanFailure {
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsScanResult {
    pub service_type: String,
    pub discovered_at_ms: u64,
    pub datagram_count: usize,
    pub advertisements: Vec<MdnsAdvertisement>,
    pub failures: Vec<MdnsScanFailure>,
}

impl MdnsScanResult {
    pub fn from_packets(
        service_type: impl Into<String>,
        discovered_at_ms: u64,
        packets: impl IntoIterator<Item = MdnsResponsePacket>,
    ) -> Result<Self, DiscoveryError> {
        let service_type = non_empty("mdns.service_type", service_type)?;
        let mut result = Self {
            service_type: service_type.clone(),
            discovered_at_ms,
            datagram_count: 0,
            advertisements: Vec::new(),
            failures: Vec::new(),
        };

        for packet in packets {
            result.datagram_count += 1;
            match mdns_advertisements_from_response(
                &packet.payload,
                &service_type,
                discovered_at_ms,
            ) {
                Ok(mut advertisements) => result.advertisements.append(&mut advertisements),
                Err(error) => result.failures.push(MdnsScanFailure {
                    source: packet.source,
                    message: error.to_string(),
                }),
            }
        }

        Ok(result)
    }

    pub fn len(&self) -> usize {
        self.advertisements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.advertisements.is_empty()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsWorkerScanSuccess {
    pub request: MdnsWorkerScanRequest,
    pub result: MdnsScanResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsWorkerScanFailure {
    pub request: MdnsWorkerScanRequest,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsWorkerScanReport {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub service_type: String,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub successes: Vec<MdnsWorkerScanSuccess>,
    pub failures: Vec<MdnsWorkerScanFailure>,
    pub metadata: Vec<Metadata>,
}

impl MdnsWorkerScanReport {
    pub fn new(
        worker_id: DiscoveryWorkerId,
        integration_id: IntegrationId,
        service_type: impl Into<String>,
        started_at_ms: u64,
        completed_at_ms: u64,
    ) -> Result<Self, DiscoveryError> {
        Ok(Self {
            worker_id,
            integration_id,
            service_type: non_empty("mdns.service_type", service_type)?,
            started_at_ms,
            completed_at_ms,
            successes: Vec::new(),
            failures: Vec::new(),
            metadata: Vec::new(),
        })
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn push_success(
        &mut self,
        request: MdnsWorkerScanRequest,
        result: MdnsScanResult,
    ) -> Result<(), DiscoveryError> {
        self.validate_request(&request)?;
        if result.service_type != self.service_type {
            return Err(DiscoveryError::InvalidMdnsScanOption {
                field: "service_type",
                message: format!(
                    "expected `{}` result but received `{}`",
                    self.service_type, result.service_type
                ),
            });
        }
        self.successes
            .push(MdnsWorkerScanSuccess { request, result });
        Ok(())
    }

    pub fn push_failure(
        &mut self,
        request: MdnsWorkerScanRequest,
        message: impl Into<String>,
    ) -> Result<(), DiscoveryError> {
        self.validate_request(&request)?;
        self.failures.push(MdnsWorkerScanFailure {
            request,
            message: non_empty("mdns.worker_scan_failure.message", message)?,
        });
        Ok(())
    }

    pub fn completed_scan_count(&self) -> usize {
        self.successes.len()
    }

    pub fn failed_scan_count(&self) -> usize {
        self.failures.len()
    }

    pub fn datagram_count(&self) -> usize {
        self.successes
            .iter()
            .map(|success| success.result.datagram_count)
            .sum()
    }

    pub fn advertisement_count(&self) -> usize {
        self.successes
            .iter()
            .map(|success| success.result.advertisements.len())
            .sum()
    }

    pub fn packet_failure_count(&self) -> usize {
        self.successes
            .iter()
            .map(|success| success.result.failures.len())
            .sum()
    }

    pub fn has_failures(&self) -> bool {
        self.failed_scan_count() > 0 || self.packet_failure_count() > 0
    }

    pub fn aggregate_result(&self) -> MdnsScanResult {
        let mut result = MdnsScanResult {
            service_type: self.service_type.clone(),
            discovered_at_ms: self.completed_at_ms,
            datagram_count: self.datagram_count(),
            advertisements: Vec::new(),
            failures: Vec::new(),
        };

        for success in &self.successes {
            result
                .advertisements
                .extend(success.result.advertisements.iter().cloned());
            for failure in &success.result.failures {
                result.failures.push(MdnsScanFailure {
                    source: Some(mdns_worker_scan_source(
                        &success.request,
                        failure.source.as_deref(),
                    )),
                    message: failure.message.clone(),
                });
            }
        }

        for failure in &self.failures {
            result.failures.push(MdnsScanFailure {
                source: Some(mdns_worker_scan_source(&failure.request, None)),
                message: failure.message.clone(),
            });
        }

        result
    }

    fn validate_request(&self, request: &MdnsWorkerScanRequest) -> Result<(), DiscoveryError> {
        if request.worker_id != self.worker_id {
            return Err(DiscoveryError::InvalidMdnsScanOption {
                field: "worker_id",
                message: format!(
                    "expected `{}` request but received `{}`",
                    self.worker_id, request.worker_id
                ),
            });
        }
        if request.integration_id != self.integration_id {
            return Err(DiscoveryError::InvalidMdnsScanOption {
                field: "integration_id",
                message: format!(
                    "expected `{}` request but received `{}`",
                    self.integration_id, request.integration_id
                ),
            });
        }
        if request.service_type != self.service_type {
            return Err(DiscoveryError::InvalidMdnsScanOption {
                field: "service_type",
                message: format!(
                    "expected `{}` request but received `{}`",
                    self.service_type, request.service_type
                ),
            });
        }
        Ok(())
    }
}

pub trait MdnsWorkerScanExecutor {
    fn run_request(
        &mut self,
        request: &MdnsWorkerScanRequest,
    ) -> Result<MdnsScanResult, DiscoveryError>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UdpMdnsWorkerScanExecutor;

impl MdnsWorkerScanExecutor for UdpMdnsWorkerScanExecutor {
    fn run_request(
        &mut self,
        request: &MdnsWorkerScanRequest,
    ) -> Result<MdnsScanResult, DiscoveryError> {
        run_mdns_worker_scan_request(request)
    }
}

pub fn run_mdns_worker_scan_request(
    request: &MdnsWorkerScanRequest,
) -> Result<MdnsScanResult, DiscoveryError> {
    match request.network {
        MdnsScanNetwork::Ipv4 => run_mdns_ipv4_scan(request.options()?),
        MdnsScanNetwork::Ipv6 => run_mdns_ipv6_scan(request.options()?),
    }
}

pub fn run_mdns_worker_scan_report_with_executor<E>(
    requests: &[MdnsWorkerScanRequest],
    started_at_ms: u64,
    completed_at_ms: u64,
    executor: &mut E,
) -> Result<MdnsWorkerScanReport, DiscoveryError>
where
    E: MdnsWorkerScanExecutor + ?Sized,
{
    let first = requests
        .first()
        .ok_or_else(|| DiscoveryError::InvalidMdnsScanOption {
            field: "requests",
            message: "must include at least one mDNS scan request".to_string(),
        })?;
    let mut report = MdnsWorkerScanReport::new(
        first.worker_id.clone(),
        first.integration_id.clone(),
        first.service_type.clone(),
        started_at_ms,
        completed_at_ms,
    )?;

    for request in requests {
        report.validate_request(request)?;
        match executor.run_request(request) {
            Ok(result) => report.push_success(request.clone(), result)?,
            Err(error) => report.push_failure(request.clone(), error.to_string())?,
        }
    }

    Ok(report)
}

pub fn run_mdns_worker_scan_report(
    requests: &[MdnsWorkerScanRequest],
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<MdnsWorkerScanReport, DiscoveryError> {
    let mut executor = UdpMdnsWorkerScanExecutor;
    run_mdns_worker_scan_report_with_executor(
        requests,
        started_at_ms,
        completed_at_ms,
        &mut executor,
    )
}

pub fn run_mdns_worker_scan_plan_with_executor<E>(
    plan: &MdnsWorkerScanPlan,
    started_at_ms: u64,
    completed_at_ms: u64,
    executor: &mut E,
) -> Result<Vec<MdnsWorkerScanReport>, DiscoveryError>
where
    E: MdnsWorkerScanExecutor + ?Sized,
{
    let mut groups: Vec<Vec<MdnsWorkerScanRequest>> = Vec::new();
    'requests: for request in &plan.requests {
        for group in &mut groups {
            if group
                .first()
                .is_some_and(|existing| mdns_worker_scan_same_scope(existing, request))
            {
                group.push(request.clone());
                continue 'requests;
            }
        }
        groups.push(vec![request.clone()]);
    }

    let mut reports = Vec::with_capacity(groups.len());
    for group in groups {
        reports.push(run_mdns_worker_scan_report_with_executor(
            &group,
            started_at_ms,
            completed_at_ms,
            executor,
        )?);
    }
    Ok(reports)
}

pub fn run_mdns_worker_scan_plan(
    plan: &MdnsWorkerScanPlan,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<Vec<MdnsWorkerScanReport>, DiscoveryError> {
    let mut executor = UdpMdnsWorkerScanExecutor;
    run_mdns_worker_scan_plan_with_executor(plan, started_at_ms, completed_at_ms, &mut executor)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsScanOptions {
    pub service_type: String,
    pub discovered_at_ms: u64,
    pub timeout: Duration,
    pub max_responses: usize,
    pub max_datagram_size: usize,
    pub unicast_response: bool,
}

impl MdnsScanOptions {
    pub fn new(
        service_type: impl Into<String>,
        discovered_at_ms: u64,
        timeout: Duration,
    ) -> Result<Self, DiscoveryError> {
        Ok(Self {
            service_type: non_empty("mdns.service_type", service_type)?,
            discovered_at_ms,
            timeout,
            max_responses: MDNS_DEFAULT_MAX_RESPONSES,
            max_datagram_size: MDNS_DEFAULT_MAX_DATAGRAM_SIZE,
            unicast_response: true,
        })
    }

    pub fn with_max_responses(mut self, max_responses: usize) -> Self {
        self.max_responses = max_responses;
        self
    }

    pub fn with_max_datagram_size(mut self, max_datagram_size: usize) -> Self {
        self.max_datagram_size = max_datagram_size;
        self
    }

    pub fn multicast_response(mut self) -> Self {
        self.unicast_response = false;
        self
    }

    pub fn query_packet(&self) -> Result<Vec<u8>, DiscoveryError> {
        let mut question = MdnsQuestion::new(self.service_type.clone())?;
        if !self.unicast_response {
            question = question.multicast_response();
        }
        question.to_query_packet()
    }
}

pub fn run_mdns_ipv4_scan(options: MdnsScanOptions) -> Result<MdnsScanResult, DiscoveryError> {
    validate_mdns_scan_options(&options)?;
    let endpoint = UdpDiscoveryEndpoint::mdns_ipv4();
    let query = options.query_packet()?;
    let udp_options = endpoint.options(
        options.max_datagram_size,
        Some(options.timeout),
        Some(options.timeout),
    );
    let datagrams = send_to_and_collect(
        endpoint.destination,
        &query,
        udp_options,
        options.max_responses,
    )
    .map_err(|error| DiscoveryError::MdnsTransport {
        message: error.to_string(),
    })?;
    let packets = datagrams.into_iter().map(|datagram| {
        MdnsResponsePacket::new(datagram.payload).with_source(datagram.source.to_string())
    });
    MdnsScanResult::from_packets(options.service_type, options.discovered_at_ms, packets)
}

pub fn run_mdns_ipv6_scan(options: MdnsScanOptions) -> Result<MdnsScanResult, DiscoveryError> {
    validate_mdns_scan_options(&options)?;
    let endpoint = UdpDiscoveryEndpoint::mdns_ipv6();
    let query = options.query_packet()?;
    let udp_options = UdpOptions {
        bind_addr: Some(endpoint.bind_addr()),
        max_datagram_size: options.max_datagram_size,
        read_timeout: Some(options.timeout),
        write_timeout: Some(options.timeout),
    };
    let datagrams = send_to_and_collect(
        endpoint.destination,
        &query,
        udp_options,
        options.max_responses,
    )
    .map_err(|error| DiscoveryError::MdnsTransport {
        message: error.to_string(),
    })?;
    let packets = datagrams.into_iter().map(|datagram| {
        MdnsResponsePacket::new(datagram.payload).with_source(datagram.source.to_string())
    });
    MdnsScanResult::from_packets(options.service_type, options.discovered_at_ms, packets)
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

    pub fn signals(&self, ttl_ms: u64) -> Vec<DiscoverySignal> {
        self.records().map(|record| record.signal(ttl_ms)).collect()
    }

    pub fn signal_summary_at(&self, now_ms: u64, ttl_ms: u64) -> DiscoverySignalSummary {
        DiscoverySignalSummary::from_signals(&self.signals(ttl_ms), now_ms)
    }

    pub fn record_summary_at(&self, now_ms: u64, ttl_ms: u64) -> DiscoveryRecordSummary {
        DiscoveryRecordSummary::from_records(self.records(), now_ms, ttl_ms)
    }

    pub fn bridge_candidates(&self) -> Vec<Bridge> {
        self.records()
            .map(DiscoveryRecord::to_bridge_candidate)
            .collect()
    }

    pub fn pairing_plan_at(
        &self,
        catalog: &[IntegrationCatalogEntry],
        now_ms: u64,
        ttl_ms: u64,
    ) -> DiscoveryPairingPlan {
        let mut targets = self
            .records()
            .filter_map(|record| pairing_target_for_record(record, catalog, now_ms, ttl_ms))
            .collect::<Vec<_>>();
        sort_pairing_targets(&mut targets, DiscoveryPairingPlanSort::PlanRank);
        DiscoveryPairingPlan {
            generated_at_ms: now_ms,
            targets,
        }
    }

    pub fn pairing_plan_with_options_at(
        &self,
        catalog: &[IntegrationCatalogEntry],
        now_ms: u64,
        ttl_ms: u64,
        options: &DiscoveryPairingPlanOptions,
    ) -> DiscoveryPairingPlan {
        self.pairing_plan_at(catalog, now_ms, ttl_ms).query(options)
    }

    pub fn pairing_plan_summary_at(
        &self,
        catalog: &[IntegrationCatalogEntry],
        now_ms: u64,
        ttl_ms: u64,
    ) -> DiscoveryPairingPlanSummary {
        self.pairing_plan_at(catalog, now_ms, ttl_ms).summary()
    }
}

fn pairing_target_for_record(
    record: &DiscoveryRecord,
    catalog: &[IntegrationCatalogEntry],
    now_ms: u64,
    ttl_ms: u64,
) -> Option<DiscoveryPairingTarget> {
    let signal_status = record.signal(ttl_ms).status_at(now_ms);
    if signal_status == DiscoverySignalStatus::Expired {
        return None;
    }
    let catalog_entry = catalog
        .iter()
        .find(|entry| entry.integration_id == record.integration_id);
    Some(DiscoveryPairingTarget {
        fingerprint: record.fingerprint(),
        bridge_id: record.bridge_id(),
        integration_id: record.integration_id.clone(),
        native_bridge_id: record.native_bridge_id.clone(),
        display_name: record
            .display_name
            .clone()
            .or_else(|| catalog_entry.map(|entry| entry.display_name.clone())),
        priority: catalog_entry.map_or(u8::MAX, |entry| entry.priority),
        source: record.source,
        confidence: record.confidence,
        signal_status,
        pairing_requirement: record.pairing_requirement,
        action: pairing_action_for_requirement(record.pairing_requirement),
        address: record.address.clone(),
        discovered_at_ms: record.discovered_at_ms,
    })
}

fn pairing_action_for_requirement(requirement: PairingRequirement) -> DiscoveryPairingAction {
    match requirement {
        PairingRequirement::Unknown => DiscoveryPairingAction::InvestigateUnknownRequirement,
        PairingRequirement::None => DiscoveryPairingAction::Ready,
        PairingRequirement::PhysicalPresence => DiscoveryPairingAction::PressPhysicalButton,
        PairingRequirement::LocalCode => DiscoveryPairingAction::EnterLocalCode,
        PairingRequirement::Credentials => DiscoveryPairingAction::ProvideCredentials,
        PairingRequirement::OAuth2 => DiscoveryPairingAction::CompleteOAuth2,
        PairingRequirement::Certificate => DiscoveryPairingAction::InstallCertificate,
        PairingRequirement::RadioInclusion => DiscoveryPairingAction::StartRadioInclusion,
        PairingRequirement::MqttCredentials => DiscoveryPairingAction::ConfigureMqttCredentials,
    }
}

fn signal_status_rank(status: DiscoverySignalStatus) -> u8 {
    match status {
        DiscoverySignalStatus::Fresh => 0,
        DiscoverySignalStatus::Stale => 1,
        DiscoverySignalStatus::Expired => 2,
    }
}

fn sort_pairing_targets(targets: &mut [DiscoveryPairingTarget], sort: DiscoveryPairingPlanSort) {
    match sort {
        DiscoveryPairingPlanSort::PlanRank => targets.sort_by(compare_pairing_targets_by_rank),
        DiscoveryPairingPlanSort::NewestFirst => targets.sort_by(|left, right| {
            right
                .discovered_at_ms
                .cmp(&left.discovered_at_ms)
                .then_with(|| compare_pairing_targets_by_rank(left, right))
        }),
        DiscoveryPairingPlanSort::IntegrationThenBridge => targets.sort_by(|left, right| {
            left.integration_id
                .cmp(&right.integration_id)
                .then_with(|| left.native_bridge_id.cmp(&right.native_bridge_id))
        }),
        DiscoveryPairingPlanSort::SourcePreference => targets.sort_by(|left, right| {
            right
                .source
                .preference_rank()
                .cmp(&left.source.preference_rank())
                .then_with(|| compare_pairing_targets_by_rank(left, right))
        }),
    }
}

fn compare_pairing_targets_by_rank(
    left: &DiscoveryPairingTarget,
    right: &DiscoveryPairingTarget,
) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| {
            signal_status_rank(left.signal_status).cmp(&signal_status_rank(right.signal_status))
        })
        .then_with(|| {
            right
                .confidence
                .preference_rank()
                .cmp(&left.confidence.preference_rank())
        })
        .then_with(|| {
            right
                .source
                .preference_rank()
                .cmp(&left.source.preference_rank())
        })
        .then_with(|| right.discovered_at_ms.cmp(&left.discovered_at_ms))
        .then_with(|| left.integration_id.cmp(&right.integration_id))
        .then_with(|| left.native_bridge_id.cmp(&right.native_bridge_id))
}

fn primary_protocol_for_entry(entry: &IntegrationCatalogEntry) -> ProtocolFamily {
    entry
        .supported_protocols
        .first()
        .cloned()
        .unwrap_or_else(|| ProtocolFamily::Vendor(entry.integration_id.as_str().to_string()))
}

fn source_for_discovery_mechanism(mechanism: DiscoveryMechanism) -> DiscoverySource {
    match mechanism {
        DiscoveryMechanism::Mdns => DiscoverySource::Mdns,
        DiscoveryMechanism::WsDiscovery => DiscoverySource::WsDiscovery,
        DiscoveryMechanism::Ssdp => DiscoverySource::Ssdp,
        DiscoveryMechanism::UdpMulticast => DiscoverySource::UdpMulticast,
        DiscoveryMechanism::UdpBroadcast => DiscoverySource::UdpBroadcast,
        DiscoveryMechanism::Bluetooth => DiscoverySource::Bluetooth,
        DiscoveryMechanism::Usb => DiscoverySource::Usb,
        DiscoveryMechanism::Dhcp => DiscoverySource::Dhcp,
        DiscoveryMechanism::Mqtt => DiscoverySource::Mqtt,
        DiscoveryMechanism::Manual | DiscoveryMechanism::FileConfig => DiscoverySource::Manual,
        DiscoveryMechanism::CloudAccount => DiscoverySource::CloudFallback,
        DiscoveryMechanism::Webhook => DiscoverySource::Webhook,
    }
}

fn transport_for_discovery_mechanism(mechanism: DiscoveryMechanism) -> BridgeTransport {
    match mechanism {
        DiscoveryMechanism::Mdns => BridgeTransport::Mdns,
        DiscoveryMechanism::WsDiscovery => BridgeTransport::LanHttp,
        DiscoveryMechanism::UdpMulticast => BridgeTransport::LanUdp,
        DiscoveryMechanism::UdpBroadcast => BridgeTransport::LanUdp,
        DiscoveryMechanism::Bluetooth => BridgeTransport::Ble,
        DiscoveryMechanism::Usb => BridgeTransport::Serial,
        DiscoveryMechanism::Mqtt | DiscoveryMechanism::FileConfig => BridgeTransport::LocalProcess,
        DiscoveryMechanism::CloudAccount | DiscoveryMechanism::Webhook => BridgeTransport::Cloud,
        DiscoveryMechanism::Ssdp | DiscoveryMechanism::Dhcp | DiscoveryMechanism::Manual => {
            BridgeTransport::LanHttp
        }
    }
}

fn pairing_requirement_for_auth_modes(auth_modes: &[AuthMode]) -> PairingRequirement {
    if auth_modes.contains(&AuthMode::OAuth2) {
        PairingRequirement::OAuth2
    } else if auth_modes.contains(&AuthMode::RadioNetworkKey) {
        PairingRequirement::RadioInclusion
    } else if auth_modes.contains(&AuthMode::Certificate) {
        PairingRequirement::Certificate
    } else if auth_modes.contains(&AuthMode::MqttCredentials) {
        PairingRequirement::MqttCredentials
    } else if auth_modes.contains(&AuthMode::LocalPairing) {
        PairingRequirement::PhysicalPresence
    } else if auth_modes.iter().any(|auth_mode| {
        matches!(
            auth_mode,
            AuthMode::LocalToken | AuthMode::UsernamePassword | AuthMode::ApiKey
        )
    }) {
        PairingRequirement::Credentials
    } else if auth_modes.contains(&AuthMode::None) {
        PairingRequirement::None
    } else {
        PairingRequirement::Unknown
    }
}

fn discovery_mechanism_name(mechanism: DiscoveryMechanism) -> &'static str {
    match mechanism {
        DiscoveryMechanism::Mdns => "mdns",
        DiscoveryMechanism::WsDiscovery => "ws_discovery",
        DiscoveryMechanism::Ssdp => "ssdp",
        DiscoveryMechanism::UdpMulticast => "udp_multicast",
        DiscoveryMechanism::UdpBroadcast => "udp_broadcast",
        DiscoveryMechanism::Bluetooth => "bluetooth",
        DiscoveryMechanism::Usb => "usb",
        DiscoveryMechanism::Dhcp => "dhcp",
        DiscoveryMechanism::Mqtt => "mqtt",
        DiscoveryMechanism::Manual => "manual",
        DiscoveryMechanism::CloudAccount => "cloud_account",
        DiscoveryMechanism::Webhook => "webhook",
        DiscoveryMechanism::FileConfig => "file_config",
    }
}

fn validate_mdns_scan_options(options: &MdnsScanOptions) -> Result<(), DiscoveryError> {
    if options.timeout.is_zero() {
        return Err(DiscoveryError::InvalidMdnsScanOption {
            field: "timeout",
            message: "must be greater than zero".to_string(),
        });
    }
    if options.max_responses == 0 {
        return Err(DiscoveryError::InvalidMdnsScanOption {
            field: "max_responses",
            message: "must be greater than zero".to_string(),
        });
    }
    if options.max_datagram_size == 0 {
        return Err(DiscoveryError::InvalidMdnsScanOption {
            field: "max_datagram_size",
            message: "must be greater than zero".to_string(),
        });
    }
    Ok(())
}

fn mdns_worker_scan_same_scope(
    left: &MdnsWorkerScanRequest,
    right: &MdnsWorkerScanRequest,
) -> bool {
    left.worker_id == right.worker_id
        && left.integration_id == right.integration_id
        && left.service_type == right.service_type
}

fn mdns_worker_scan_source(request: &MdnsWorkerScanRequest, source: Option<&str>) -> String {
    match source {
        Some(source) => format!(
            "{}/{}:{source}",
            request.network_interface,
            request.network.as_str()
        ),
        None => format!("{}/{}", request.network_interface, request.network.as_str()),
    }
}

fn encode_dns_name(name: &str, packet: &mut Vec<u8>) -> Result<(), DiscoveryError> {
    let name = name.trim().trim_end_matches('.');
    if name.is_empty() {
        return Err(DiscoveryError::EmptyField { field: "dns.name" });
    }
    for label in name.split('.') {
        if label.is_empty() {
            return Err(DiscoveryError::InvalidMdnsMessage {
                message: format!("DNS name `{name}` contains an empty label"),
            });
        }
        if label.len() > 63 {
            return Err(DiscoveryError::InvalidMdnsMessage {
                message: format!("DNS label `{label}` is longer than 63 bytes"),
            });
        }
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedDnsRecord {
    name: String,
    record_type: u16,
    data: ParsedDnsRecordData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedDnsRecordData {
    Ptr(String),
    Srv { port: u16, target: String },
    Txt(Vec<MdnsTxtEntry>),
    Address(String),
    Ignored,
}

fn mdns_advertisements_from_response(
    packet: &[u8],
    service_type: &str,
    discovered_at_ms: u64,
) -> Result<Vec<MdnsAdvertisement>, DiscoveryError> {
    let records = parse_dns_records(packet)?;
    let service_key = normalize_dns_name(service_type);
    let mut ptr_instances = Vec::new();
    let mut srv_by_instance = BTreeMap::<String, (u16, String)>::new();
    let mut txt_by_instance = BTreeMap::<String, Vec<MdnsTxtEntry>>::new();
    let mut addresses_by_host = BTreeMap::<String, Vec<String>>::new();

    for record in records {
        match record.data {
            ParsedDnsRecordData::Ptr(instance_name)
                if normalize_dns_name(&record.name) == service_key =>
            {
                ptr_instances.push(instance_name);
            }
            ParsedDnsRecordData::Srv { port, target } => {
                srv_by_instance.insert(normalize_dns_name(&record.name), (port, target));
            }
            ParsedDnsRecordData::Txt(txt) => {
                txt_by_instance.insert(normalize_dns_name(&record.name), txt);
            }
            ParsedDnsRecordData::Address(address) => {
                addresses_by_host
                    .entry(normalize_dns_name(&record.name))
                    .or_default()
                    .push(address);
            }
            ParsedDnsRecordData::Ptr(_) | ParsedDnsRecordData::Ignored => {}
        }
    }

    let mut advertisements = Vec::new();
    for instance_fqdn in ptr_instances {
        let instance_key = normalize_dns_name(&instance_fqdn);
        let (port, host_name) = srv_by_instance
            .get(&instance_key)
            .cloned()
            .unwrap_or_else(|| (0, instance_fqdn.clone()));
        let txt = txt_by_instance.remove(&instance_key).unwrap_or_default();
        let addresses = addresses_by_host
            .get(&normalize_dns_name(&host_name))
            .cloned()
            .unwrap_or_default();
        let mut advertisement = MdnsAdvertisement::new(
            service_type.to_string(),
            mdns_instance_display_name(&instance_fqdn, service_type),
            trim_trailing_dot(&host_name),
            port,
            discovered_at_ms,
        )?;
        for address in addresses {
            advertisement = advertisement.with_address(address)?;
        }
        for entry in txt {
            advertisement = advertisement.with_txt(entry.key, entry.value)?;
        }
        advertisements.push(advertisement);
    }

    Ok(advertisements)
}

fn parse_dns_records(packet: &[u8]) -> Result<Vec<ParsedDnsRecord>, DiscoveryError> {
    if packet.len() < 12 {
        return Err(invalid_mdns_message("DNS header is shorter than 12 bytes"));
    }

    let question_count = read_u16(packet, 4)? as usize;
    let answer_count = read_u16(packet, 6)? as usize;
    let authority_count = read_u16(packet, 8)? as usize;
    let additional_count = read_u16(packet, 10)? as usize;
    let mut offset = 12;

    for _ in 0..question_count {
        let (_, next_offset) = parse_dns_name(packet, offset)?;
        offset = next_offset;
        offset = offset
            .checked_add(4)
            .filter(|next| *next <= packet.len())
            .ok_or_else(|| invalid_mdns_message("question extends beyond packet"))?;
    }

    let record_count = answer_count
        .checked_add(authority_count)
        .and_then(|count| count.checked_add(additional_count))
        .ok_or_else(|| invalid_mdns_message("record count overflow"))?;
    let mut records = Vec::new();
    for _ in 0..record_count {
        let (name, next_offset) = parse_dns_name(packet, offset)?;
        offset = next_offset;
        let record_type = read_u16(packet, offset)?;
        let _class = read_u16(packet, offset + 2)?;
        let _ttl = read_u32(packet, offset + 4)?;
        let data_len = read_u16(packet, offset + 8)? as usize;
        let data_offset = offset + 10;
        let data_end = data_offset
            .checked_add(data_len)
            .filter(|end| *end <= packet.len())
            .ok_or_else(|| invalid_mdns_message("record data extends beyond packet"))?;
        let data = parse_dns_record_data(packet, record_type, data_offset, data_end)?;
        records.push(ParsedDnsRecord {
            name,
            record_type,
            data,
        });
        offset = data_end;
    }
    Ok(records)
}

fn parse_dns_record_data(
    packet: &[u8],
    record_type: u16,
    data_offset: usize,
    data_end: usize,
) -> Result<ParsedDnsRecordData, DiscoveryError> {
    match record_type {
        MDNS_DNS_TYPE_PTR => {
            let (name, _) = parse_dns_name(packet, data_offset)?;
            Ok(ParsedDnsRecordData::Ptr(name))
        }
        MDNS_DNS_TYPE_SRV => {
            if data_end.saturating_sub(data_offset) < 7 {
                return Err(invalid_mdns_message("SRV record is too short"));
            }
            let port = read_u16(packet, data_offset + 4)?;
            let (target, _) = parse_dns_name(packet, data_offset + 6)?;
            Ok(ParsedDnsRecordData::Srv { port, target })
        }
        MDNS_DNS_TYPE_TXT => parse_txt_record(packet, data_offset, data_end),
        MDNS_DNS_TYPE_A => {
            if data_end.saturating_sub(data_offset) != 4 {
                return Err(invalid_mdns_message("A record must be exactly 4 bytes"));
            }
            Ok(ParsedDnsRecordData::Address(format!(
                "{}.{}.{}.{}",
                packet[data_offset],
                packet[data_offset + 1],
                packet[data_offset + 2],
                packet[data_offset + 3]
            )))
        }
        MDNS_DNS_TYPE_AAAA => {
            if data_end.saturating_sub(data_offset) != 16 {
                return Err(invalid_mdns_message("AAAA record must be exactly 16 bytes"));
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&packet[data_offset..data_end]);
            Ok(ParsedDnsRecordData::Address(
                Ipv6Addr::from(octets).to_string(),
            ))
        }
        _ => Ok(ParsedDnsRecordData::Ignored),
    }
}

fn parse_txt_record(
    packet: &[u8],
    mut offset: usize,
    data_end: usize,
) -> Result<ParsedDnsRecordData, DiscoveryError> {
    let mut entries = Vec::new();
    while offset < data_end {
        let len = packet[offset] as usize;
        offset += 1;
        let entry_end = offset
            .checked_add(len)
            .filter(|end| *end <= data_end)
            .ok_or_else(|| invalid_mdns_message("TXT entry extends beyond record"))?;
        if len > 0 {
            let entry = String::from_utf8_lossy(&packet[offset..entry_end]).to_string();
            let (key, value) = entry.split_once('=').unwrap_or((entry.as_str(), ""));
            entries.push(MdnsTxtEntry::new(key, value)?);
        }
        offset = entry_end;
    }
    Ok(ParsedDnsRecordData::Txt(entries))
}

fn parse_dns_name(packet: &[u8], offset: usize) -> Result<(String, usize), DiscoveryError> {
    let mut labels = Vec::new();
    let mut cursor = offset;
    let mut consumed_offset = None;
    let mut jumps = 0usize;

    loop {
        if cursor >= packet.len() {
            return Err(invalid_mdns_message("DNS name extends beyond packet"));
        }
        let len = packet[cursor];
        if len & 0xC0 == 0xC0 {
            if cursor + 1 >= packet.len() {
                return Err(invalid_mdns_message(
                    "compressed DNS name pointer is truncated",
                ));
            }
            let pointer = ((len as usize & 0x3F) << 8) | packet[cursor + 1] as usize;
            if pointer >= packet.len() {
                return Err(invalid_mdns_message(
                    "compressed DNS name pointer is out of range",
                ));
            }
            consumed_offset.get_or_insert(cursor + 2);
            cursor = pointer;
            jumps += 1;
            if jumps > 16 {
                return Err(invalid_mdns_message("compressed DNS name pointer loop"));
            }
            continue;
        }
        if len & 0xC0 != 0 {
            return Err(invalid_mdns_message("unsupported DNS name label encoding"));
        }
        cursor += 1;
        if len == 0 {
            let next_offset = consumed_offset.unwrap_or(cursor);
            return Ok((labels.join("."), next_offset));
        }
        let label_len = len as usize;
        let label_end = cursor
            .checked_add(label_len)
            .filter(|end| *end <= packet.len())
            .ok_or_else(|| invalid_mdns_message("DNS label extends beyond packet"))?;
        labels.push(String::from_utf8_lossy(&packet[cursor..label_end]).to_string());
        cursor = label_end;
    }
}

fn read_u16(packet: &[u8], offset: usize) -> Result<u16, DiscoveryError> {
    let bytes = packet
        .get(offset..offset + 2)
        .ok_or_else(|| invalid_mdns_message("expected u16 beyond packet"))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32(packet: &[u8], offset: usize) -> Result<u32, DiscoveryError> {
    let bytes = packet
        .get(offset..offset + 4)
        .ok_or_else(|| invalid_mdns_message("expected u32 beyond packet"))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn mdns_instance_display_name(instance_fqdn: &str, service_type: &str) -> String {
    let instance = trim_trailing_dot(instance_fqdn);
    let service = trim_trailing_dot(service_type);
    let suffix = format!(".{service}");
    if instance
        .to_ascii_lowercase()
        .ends_with(&suffix.to_ascii_lowercase())
    {
        instance[..instance.len().saturating_sub(suffix.len())].to_string()
    } else {
        instance.to_string()
    }
}

fn normalize_dns_name(name: &str) -> String {
    trim_trailing_dot(name).to_ascii_lowercase()
}

fn trim_trailing_dot(name: &str) -> String {
    name.trim().trim_end_matches('.').to_string()
}

fn invalid_mdns_message(message: impl Into<String>) -> DiscoveryError {
    DiscoveryError::InvalidMdnsMessage {
        message: message.into(),
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
    use smart_home_integration_catalog::first_party_catalog;

    #[derive(Debug)]
    struct ScriptedMdnsWorkerScanExecutor {
        outcomes: std::collections::VecDeque<Result<MdnsScanResult, DiscoveryError>>,
        requests: Vec<MdnsWorkerScanRequest>,
    }

    impl ScriptedMdnsWorkerScanExecutor {
        fn new(outcomes: impl IntoIterator<Item = Result<MdnsScanResult, DiscoveryError>>) -> Self {
            Self {
                outcomes: outcomes.into_iter().collect(),
                requests: Vec::new(),
            }
        }
    }

    impl MdnsWorkerScanExecutor for ScriptedMdnsWorkerScanExecutor {
        fn run_request(
            &mut self,
            request: &MdnsWorkerScanRequest,
        ) -> Result<MdnsScanResult, DiscoveryError> {
            self.requests.push(request.clone());
            self.outcomes.pop_front().unwrap_or_else(|| {
                Err(DiscoveryError::MdnsTransport {
                    message: "missing scripted mDNS outcome".to_string(),
                })
            })
        }
    }

    #[test]
    fn catalog_hints_project_hue_discovery_and_pairing_shape() {
        let catalog = first_party_catalog();
        let hints = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("hue"));
        let mdns_hint = hints
            .iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mdns)
            .unwrap();

        assert!(hints
            .iter()
            .any(|hint| hint.discovery_mechanism == DiscoveryMechanism::Manual));
        assert_eq!(mdns_hint.source, DiscoverySource::Mdns);
        assert_eq!(mdns_hint.transport, BridgeTransport::Mdns);
        assert_eq!(mdns_hint.protocol_family, ProtocolFamily::Hue);
        assert_eq!(
            mdns_hint.pairing_requirement,
            PairingRequirement::PhysicalPresence
        );

        let record = mdns_hint
            .to_record("001788fffeabcdef", 1_000)
            .unwrap()
            .with_address("https://192.0.2.10");
        let bridge = record.to_bridge_candidate();

        assert_eq!(bridge.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(bridge.health, Health::Unpaired);
        assert!(bridge.metadata.iter().any(|metadata| metadata.key
            == "smart_home.discovery.catalog_hint"
            && metadata.value == "true"));
        assert!(bridge
            .metadata
            .iter()
            .any(|metadata| metadata.key == "smart_home.discovery.mechanism"
                && metadata.value == "mdns"));
    }

    #[test]
    fn catalog_hints_group_scan_work_by_source() {
        let catalog = first_party_catalog();
        let mqtt_hints = discovery_hints_for_source(&catalog, DiscoverySource::Mqtt);
        let usb_hints = discovery_hints_for_source(&catalog, DiscoverySource::Usb);

        assert!(mqtt_hints
            .iter()
            .any(|hint| hint.integration_id == IntegrationId::trusted("mqtt")));
        assert!(mqtt_hints
            .iter()
            .any(|hint| hint.integration_id == IntegrationId::trusted("tasmota")));
        assert!(usb_hints
            .iter()
            .any(|hint| hint.integration_id == IntegrationId::trusted("zigbee")));
        assert!(usb_hints
            .iter()
            .any(|hint| hint.integration_id == IntegrationId::trusted("zwave")));
    }

    #[test]
    fn catalog_hints_preserve_pairing_requirements_from_auth_modes() {
        let catalog = first_party_catalog();
        let zwave = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("zwave"));
        let matter = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("matter"));
        let tuya = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("tuya"));

        assert!(zwave
            .iter()
            .all(|hint| hint.pairing_requirement == PairingRequirement::RadioInclusion));
        assert!(matter
            .iter()
            .all(|hint| hint.pairing_requirement == PairingRequirement::Certificate));
        assert!(tuya
            .iter()
            .all(|hint| hint.pairing_requirement == PairingRequirement::OAuth2));
    }

    #[test]
    fn pairing_plan_orders_actionable_discoveries_by_catalog_priority() {
        let catalog = first_party_catalog();
        let hue_hint = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("hue"))
            .into_iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mdns)
            .unwrap();
        let mqtt_hint = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("mqtt"))
            .into_iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mqtt)
            .unwrap();
        let hue = hue_hint
            .to_record("001788fffeabcdef", 1_900)
            .unwrap()
            .with_address("https://192.0.2.10")
            .with_confidence(DiscoveryConfidence::Verified);
        let mqtt = mqtt_hint
            .to_record("broker-1", 1_000)
            .unwrap()
            .with_confidence(DiscoveryConfidence::Candidate);
        let expired_matter = DiscoveryRecord::new(
            IntegrationId::trusted("matter"),
            ProtocolFamily::Matter,
            "fabric-expired",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            1_000,
        )
        .unwrap()
        .with_pairing_requirement(PairingRequirement::Certificate)
        .with_expires_at_ms(1_500);
        let mut discoveries = DiscoveryCatalog::new();
        discoveries.record(hue);
        discoveries.record(mqtt);
        discoveries.record(expired_matter);

        let plan = discoveries.pairing_plan_at(&catalog, 2_000, 500);

        assert_eq!(plan.generated_at_ms, 2_000);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan.actionable_targets().len(), 2);
        assert!(plan
            .targets_for_integration(&IntegrationId::trusted("matter"))
            .is_empty());
        assert!(matches!(
            plan.targets.as_slice(),
            [hue_target, mqtt_target]
                if hue_target.integration_id == IntegrationId::trusted("hue")
                    && hue_target.display_name.as_deref() == Some("Philips Hue")
                    && hue_target.action == DiscoveryPairingAction::PressPhysicalButton
                    && hue_target.requires_human_action()
                    && hue_target.signal_status == DiscoverySignalStatus::Fresh
                    && mqtt_target.integration_id == IntegrationId::trusted("mqtt")
                    && mqtt_target.action == DiscoveryPairingAction::ConfigureMqttCredentials
                    && mqtt_target.signal_status == DiscoverySignalStatus::Stale
        ));
    }

    #[test]
    fn pairing_plan_summarizes_host_action_queue() {
        let catalog = first_party_catalog();
        let hue_hint = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("hue"))
            .into_iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mdns)
            .unwrap();
        let mqtt_hint = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("mqtt"))
            .into_iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mqtt)
            .unwrap();
        let hue = hue_hint
            .to_record("001788fffeabcdef", 1_900)
            .unwrap()
            .with_confidence(DiscoveryConfidence::Verified);
        let mqtt = mqtt_hint
            .to_record("broker-1", 1_000)
            .unwrap()
            .with_confidence(DiscoveryConfidence::Candidate);
        let unknown_hue = DiscoveryRecord::new(
            IntegrationId::trusted("hue"),
            ProtocolFamily::Hue,
            "001788fffeunknown",
            DiscoverySource::Manual,
            BridgeTransport::LanHttp,
            1_950,
        )
        .unwrap()
        .with_pairing_requirement(PairingRequirement::Unknown);
        let mut discoveries = DiscoveryCatalog::new();
        discoveries.record(hue);
        discoveries.record(mqtt);
        discoveries.record(unknown_hue);

        let summary = discoveries.pairing_plan_summary_at(&catalog, 2_000, 500);

        assert_eq!(summary.generated_at_ms, 2_000);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.actionable, 2);
        assert_eq!(summary.ready, 0);
        assert_eq!(summary.requires_human_action, 3);
        assert_eq!(summary.blocked_unknown_requirement, 1);
        assert_eq!(summary.fresh, 2);
        assert_eq!(summary.stale, 1);
        assert_eq!(summary.count_for_source(DiscoverySource::Mdns), 1);
        assert_eq!(summary.count_for_source(DiscoverySource::Mqtt), 1);
        assert_eq!(summary.count_for_source(DiscoverySource::Manual), 1);
        assert_eq!(
            summary.count_for_pairing_requirement(PairingRequirement::PhysicalPresence),
            1
        );
        assert_eq!(
            summary.count_for_action(DiscoveryPairingAction::PressPhysicalButton),
            1
        );
        assert_eq!(
            summary.count_for_action(DiscoveryPairingAction::ConfigureMqttCredentials),
            1
        );
        assert_eq!(
            summary.count_for_action(DiscoveryPairingAction::InvestigateUnknownRequirement),
            1
        );
        assert_eq!(
            summary
                .next_actionable_target
                .as_ref()
                .map(|target| target.action),
            Some(DiscoveryPairingAction::PressPhysicalButton)
        );
        assert!(!summary.is_empty());
    }

    #[test]
    fn pairing_plan_options_filter_human_action_queue() {
        let catalog = first_party_catalog();
        let hue_hint = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("hue"))
            .into_iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mdns)
            .unwrap();
        let mqtt_hint = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("mqtt"))
            .into_iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mqtt)
            .unwrap();
        let hue = hue_hint
            .to_record("001788fffeabcdef", 1_500)
            .unwrap()
            .with_confidence(DiscoveryConfidence::Verified);
        let mqtt = mqtt_hint
            .to_record("broker-1", 1_900)
            .unwrap()
            .with_confidence(DiscoveryConfidence::Candidate);
        let mut discoveries = DiscoveryCatalog::new();
        discoveries.record(hue);
        discoveries.record(mqtt);

        let human_mqtt = discoveries.pairing_plan_with_options_at(
            &catalog,
            2_000,
            1_000,
            &DiscoveryPairingPlanOptions::new()
                .actionable_only(true)
                .requiring_human_action(true)
                .with_action(DiscoveryPairingAction::ConfigureMqttCredentials)
                .sorted_by(DiscoveryPairingPlanSort::NewestFirst),
        );

        assert_eq!(human_mqtt.len(), 1);
        assert_eq!(
            human_mqtt.targets[0].integration_id,
            IntegrationId::trusted("mqtt")
        );
        assert_eq!(
            human_mqtt.targets[0].action,
            DiscoveryPairingAction::ConfigureMqttCredentials
        );
    }

    #[test]
    fn pairing_plan_options_bound_priority_and_source_views() {
        let catalog = first_party_catalog();
        let hue_hint = discovery_hints_for_integration(&catalog, &IntegrationId::trusted("hue"))
            .into_iter()
            .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mdns)
            .unwrap();
        let zwave_hint =
            discovery_hints_for_integration(&catalog, &IntegrationId::trusted("zwave"))
                .into_iter()
                .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Usb)
                .unwrap();
        let tasmota_hint =
            discovery_hints_for_integration(&catalog, &IntegrationId::trusted("tasmota"))
                .into_iter()
                .find(|hint| hint.discovery_mechanism == DiscoveryMechanism::Mqtt)
                .unwrap();
        let mut discoveries = DiscoveryCatalog::new();
        discoveries.record(hue_hint.to_record("001788fffeabcdef", 1_000).unwrap());
        discoveries.record(zwave_hint.to_record("zwave-stick-1", 1_500).unwrap());
        discoveries.record(tasmota_hint.to_record("plug-1", 1_800).unwrap());

        let source_view = discoveries.pairing_plan_with_options_at(
            &catalog,
            2_000,
            2_000,
            &DiscoveryPairingPlanOptions::new()
                .at_or_before_priority(0)
                .with_source(DiscoverySource::Usb)
                .with_pairing_requirement(PairingRequirement::RadioInclusion)
                .limited_to(1),
        );

        assert_eq!(source_view.len(), 1);
        assert_eq!(
            source_view.targets[0].integration_id,
            IntegrationId::trusted("zwave")
        );
        assert_eq!(source_view.targets[0].source, DiscoverySource::Usb);
        assert!(source_view.targets[0].requires_human_action());
    }

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
    fn mdns_questions_build_ptr_queries_with_unicast_response_class() {
        let query = MdnsQuestion::new("_hue._tcp.local")
            .unwrap()
            .to_query_packet()
            .unwrap();

        assert_eq!(&query[0..2], &[0, 0]);
        assert_eq!(&query[4..6], &[0, 1]);
        assert!(query.windows(4).any(|window| window == b"_hue"));
        assert_eq!(
            &query[query.len() - 4..],
            &[
                (MDNS_DNS_TYPE_PTR >> 8) as u8,
                MDNS_DNS_TYPE_PTR as u8,
                ((MDNS_DNS_CLASS_IN | MDNS_UNICAST_RESPONSE_CLASS_BIT) >> 8) as u8,
                (MDNS_DNS_CLASS_IN | MDNS_UNICAST_RESPONSE_CLASS_BIT) as u8,
            ]
        );

        let multicast_query = MdnsQuestion::new("_hue._tcp.local")
            .unwrap()
            .multicast_response()
            .to_query_packet()
            .unwrap();
        assert_eq!(
            &multicast_query[multicast_query.len() - 2..],
            &[(MDNS_DNS_CLASS_IN >> 8) as u8, MDNS_DNS_CLASS_IN as u8]
        );
    }

    #[test]
    fn mdns_scan_result_parses_dns_sd_hue_advertisements_with_compression() {
        let packet = hue_mdns_response_packet();
        let result = MdnsScanResult::from_packets(
            "_hue._tcp.local",
            5_000,
            [MdnsResponsePacket::new(packet).with_source("192.0.2.10:5353")],
        )
        .unwrap();

        assert_eq!(result.datagram_count, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result.failure_count(), 0);

        let advertisement = &result.advertisements[0];
        assert_eq!(advertisement.service_type, "_hue._tcp.local");
        assert_eq!(advertisement.instance_name, "Living Room Hue");
        assert_eq!(advertisement.host_name, "hue-bridge.local");
        assert_eq!(advertisement.port, 443);
        assert_eq!(advertisement.addresses, vec!["192.0.2.10"]);
        assert_eq!(
            advertisement.txt_value("bridgeid"),
            Some("001788fffeabcdef")
        );
        assert_eq!(advertisement.txt_value("modelid"), Some("BSB002"));
        assert_eq!(advertisement.discovered_at_ms, 5_000);
    }

    #[test]
    fn mdns_scan_result_preserves_malformed_datagram_failures() {
        let result = MdnsScanResult::from_packets(
            "_hue._tcp.local",
            5_000,
            [
                MdnsResponsePacket::new([1, 2, 3]).with_source("192.0.2.20:5353"),
                MdnsResponsePacket::new(hue_mdns_response_packet()).with_source("192.0.2.10:5353"),
            ],
        )
        .unwrap();

        assert_eq!(result.datagram_count, 2);
        assert_eq!(result.len(), 1);
        assert_eq!(result.failure_count(), 1);
        assert_eq!(
            result.failures[0].source.as_deref(),
            Some("192.0.2.20:5353")
        );
        assert!(result.failures[0]
            .message
            .contains("DNS header is shorter than 12 bytes"));
    }

    #[test]
    fn mdns_scan_options_validate_bounded_socket_scans() {
        let error = run_mdns_ipv4_scan(
            MdnsScanOptions::new("_hue._tcp.local", 1_000, Duration::ZERO).unwrap(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            DiscoveryError::InvalidMdnsScanOption {
                field: "timeout",
                message: "must be greater than zero".to_string()
            }
        );

        let error = run_mdns_ipv4_scan(
            MdnsScanOptions::new("_hue._tcp.local", 1_000, Duration::from_millis(1))
                .unwrap()
                .with_max_responses(0),
        )
        .unwrap_err();
        assert_eq!(
            error,
            DiscoveryError::InvalidMdnsScanOption {
                field: "max_responses",
                message: "must be greater than zero".to_string()
            }
        );
    }

    #[test]
    fn mdns_worker_scan_requests_project_socket_options_and_scope() {
        let request = MdnsWorkerScanRequest::new(
            DiscoveryWorkerId::trusted("hue-mdns-worker"),
            IntegrationId::trusted("hue"),
            "en0",
            MdnsScanNetwork::Ipv6,
            "_hue._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap()
        .with_max_responses(8)
        .with_max_datagram_size(512)
        .multicast_response()
        .with_metadata("fixture", "mdns_worker_scan_request");
        let mut plan = MdnsWorkerScanPlan::new(4_990);
        plan.push_request(request.clone());
        let options = request.options().unwrap();

        assert_eq!(plan.generated_at_ms, 4_990);
        assert_eq!(plan.len(), 1);
        assert_eq!(
            plan.requests_for_worker(&DiscoveryWorkerId::trusted("hue-mdns-worker"))[0].network,
            MdnsScanNetwork::Ipv6
        );
        assert_eq!(request.network_interface, "en0");
        assert_eq!(request.network, MdnsScanNetwork::Ipv6);
        assert_eq!(request.network.as_str(), "ipv6");
        assert_eq!(options.service_type, "_hue._tcp.local");
        assert_eq!(options.discovered_at_ms, 5_000);
        assert_eq!(options.timeout, Duration::from_millis(250));
        assert_eq!(options.max_responses, 8);
        assert_eq!(options.max_datagram_size, 512);
        assert!(!options.unicast_response);
        assert!(request
            .metadata
            .iter()
            .any(|metadata| metadata.key == "fixture"
                && metadata.value == "mdns_worker_scan_request"));
    }

    #[test]
    fn mdns_worker_scan_reports_aggregate_interface_results_and_failures() {
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        let integration_id = IntegrationId::trusted("hue");
        let ipv4 = MdnsWorkerScanRequest::new(
            worker_id.clone(),
            integration_id.clone(),
            "en0",
            MdnsScanNetwork::Ipv4,
            "_hue._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap();
        let ipv6 = MdnsWorkerScanRequest::new(
            worker_id.clone(),
            integration_id.clone(),
            "en0",
            MdnsScanNetwork::Ipv6,
            "_hue._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap();
        let result = MdnsScanResult::from_packets(
            "_hue._tcp.local",
            5_000,
            [
                MdnsResponsePacket::new(hue_mdns_response_packet()).with_source("192.0.2.10:5353"),
                MdnsResponsePacket::new([1, 2, 3]).with_source("192.0.2.20:5353"),
            ],
        )
        .unwrap();
        let mut report = MdnsWorkerScanReport::new(
            worker_id.clone(),
            integration_id,
            "_hue._tcp.local",
            4_950,
            5_050,
        )
        .unwrap()
        .with_metadata("fixture", "mdns_worker_scan_report");

        report.push_success(ipv4, result).unwrap();
        report
            .push_failure(ipv6, "IPv6 multicast route is unavailable")
            .unwrap();
        let aggregate = report.aggregate_result();

        assert_eq!(report.completed_scan_count(), 1);
        assert_eq!(report.failed_scan_count(), 1);
        assert_eq!(report.datagram_count(), 2);
        assert_eq!(report.advertisement_count(), 1);
        assert_eq!(report.packet_failure_count(), 1);
        assert!(report.has_failures());
        assert_eq!(aggregate.service_type, "_hue._tcp.local");
        assert_eq!(aggregate.datagram_count, 2);
        assert_eq!(aggregate.len(), 1);
        assert_eq!(aggregate.failure_count(), 2);
        assert_eq!(
            aggregate.failures[0].source.as_deref(),
            Some("en0/ipv4:192.0.2.20:5353")
        );
        assert!(aggregate.failures[0]
            .message
            .contains("DNS header is shorter than 12 bytes"));
        assert_eq!(aggregate.failures[1].source.as_deref(), Some("en0/ipv6"));
        assert_eq!(
            aggregate.failures[1].message,
            "IPv6 multicast route is unavailable"
        );
    }

    #[test]
    fn mdns_worker_scan_report_runner_collects_transport_failures() {
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        let integration_id = IntegrationId::trusted("hue");
        let ipv4 = MdnsWorkerScanRequest::new(
            worker_id.clone(),
            integration_id.clone(),
            "en0",
            MdnsScanNetwork::Ipv4,
            "_hue._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap();
        let ipv6 = MdnsWorkerScanRequest::new(
            worker_id,
            integration_id,
            "en0",
            MdnsScanNetwork::Ipv6,
            "_hue._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap();
        let success = MdnsScanResult::from_packets(
            "_hue._tcp.local",
            5_000,
            [MdnsResponsePacket::new(hue_mdns_response_packet()).with_source("192.0.2.10:5353")],
        )
        .unwrap();
        let mut executor = ScriptedMdnsWorkerScanExecutor::new([
            Ok(success),
            Err(DiscoveryError::MdnsTransport {
                message: "IPv6 multicast route is unavailable".to_string(),
            }),
        ]);

        let report = run_mdns_worker_scan_report_with_executor(
            &[ipv4.clone(), ipv6.clone()],
            4_950,
            5_050,
            &mut executor,
        )
        .unwrap();
        let aggregate = report.aggregate_result();

        assert_eq!(executor.requests, vec![ipv4, ipv6]);
        assert_eq!(report.completed_scan_count(), 1);
        assert_eq!(report.failed_scan_count(), 1);
        assert_eq!(report.datagram_count(), 1);
        assert_eq!(report.advertisement_count(), 1);
        assert_eq!(aggregate.failure_count(), 1);
        assert_eq!(aggregate.failures[0].source.as_deref(), Some("en0/ipv6"));
        assert_eq!(
            aggregate.failures[0].message,
            "mDNS transport failed: IPv6 multicast route is unavailable"
        );
    }

    #[test]
    fn mdns_worker_scan_plan_runner_groups_reports_by_worker_scope() {
        let hue_worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        let hue_integration_id = IntegrationId::trusted("hue");
        let matter_worker_id = DiscoveryWorkerId::trusted("matter-mdns-worker");
        let matter_integration_id = IntegrationId::trusted("matter");
        let hue_ipv4 = MdnsWorkerScanRequest::new(
            hue_worker_id.clone(),
            hue_integration_id.clone(),
            "en0",
            MdnsScanNetwork::Ipv4,
            "_hue._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap();
        let matter_ipv4 = MdnsWorkerScanRequest::new(
            matter_worker_id.clone(),
            matter_integration_id,
            "bridge0",
            MdnsScanNetwork::Ipv4,
            "_matter._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap();
        let hue_ipv6 = MdnsWorkerScanRequest::new(
            hue_worker_id.clone(),
            hue_integration_id,
            "en0",
            MdnsScanNetwork::Ipv6,
            "_hue._tcp.local",
            5_000,
            Duration::from_millis(250),
        )
        .unwrap();
        let mut plan = MdnsWorkerScanPlan::new(4_990);
        plan.push_request(hue_ipv4.clone());
        plan.push_request(matter_ipv4.clone());
        plan.push_request(hue_ipv6.clone());
        let mut executor = ScriptedMdnsWorkerScanExecutor::new([
            MdnsScanResult::from_packets(
                "_hue._tcp.local",
                5_000,
                Vec::<MdnsResponsePacket>::new(),
            ),
            MdnsScanResult::from_packets(
                "_hue._tcp.local",
                5_000,
                Vec::<MdnsResponsePacket>::new(),
            ),
            MdnsScanResult::from_packets(
                "_matter._tcp.local",
                5_000,
                Vec::<MdnsResponsePacket>::new(),
            ),
        ]);

        let reports =
            run_mdns_worker_scan_plan_with_executor(&plan, 4_950, 5_050, &mut executor).unwrap();

        assert_eq!(
            executor.requests,
            vec![hue_ipv4.clone(), hue_ipv6.clone(), matter_ipv4.clone()]
        );
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].worker_id, hue_worker_id);
        assert_eq!(reports[0].service_type, "_hue._tcp.local");
        assert_eq!(reports[0].completed_scan_count(), 2);
        assert_eq!(reports[1].worker_id, matter_worker_id);
        assert_eq!(reports[1].service_type, "_matter._tcp.local");
        assert_eq!(reports[1].completed_scan_count(), 1);
    }

    #[test]
    fn discovery_worker_runs_summarize_records_failures_and_catalog_outcomes() {
        let record = ManualBridgeInput {
            integration_id: IntegrationId::trusted("hue"),
            protocol_family: ProtocolFamily::Hue,
            native_bridge_id: "001788fffeabcdef".to_string(),
            address: "https://192.0.2.10".to_string(),
            transport: BridgeTransport::LanHttp,
            discovered_at_ms: 1_900,
        }
        .into_record()
        .unwrap()
        .with_confidence(DiscoveryConfidence::Verified)
        .with_pairing_requirement(PairingRequirement::PhysicalPresence);
        let mut run = DiscoveryWorkerRun::new(
            DiscoveryWorkerId::trusted("hue-mdns-scan"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            1_800,
            1_950,
        )
        .with_metadata("scan", "lan");

        run.push_record(record).unwrap();
        run.push_failure(
            DiscoveryWorkerFailure::new(DiscoverySource::Mdns, "ignored malformed TXT").unwrap(),
        );
        let summary = run.summary_at(2_000, 500, 1, 0, 0);

        assert_eq!(run.status(), DiscoveryWorkerRunStatus::Partial);
        assert_eq!(run.duration_ms(), 150);
        assert_eq!(run.len(), 1);
        assert!(run.has_failures());
        assert_eq!(
            summary.worker_id,
            DiscoveryWorkerId::trusted("hue-mdns-scan")
        );
        assert_eq!(summary.kind, DiscoveryWorkerKind::MdnsScan);
        assert_eq!(summary.status, DiscoveryWorkerRunStatus::Partial);
        assert_eq!(summary.record_count, 1);
        assert_eq!(summary.failure_count, 1);
        assert_eq!(summary.inserted_count, 1);
        assert_eq!(summary.accepted_count(), 1);
        assert!(summary.has_catalog_changes());
        assert_eq!(summary.record_summary.fresh, 1);
        assert_eq!(summary.signal_summary.fresh, 1);
        assert_eq!(
            summary
                .record_summary
                .count_for_source(DiscoverySource::Manual),
            1
        );
        assert_eq!(
            summary
                .record_summary
                .count_for_pairing_requirement(PairingRequirement::PhysicalPresence),
            1
        );
    }

    #[test]
    fn discovery_worker_runs_reject_records_from_another_integration() {
        let mut run = DiscoveryWorkerRun::new(
            DiscoveryWorkerId::trusted("hue-mdns-scan"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            1_000,
            1_100,
        );
        let mqtt_record = ManualBridgeInput {
            integration_id: IntegrationId::trusted("mqtt"),
            protocol_family: ProtocolFamily::Mqtt,
            native_bridge_id: "broker-1".to_string(),
            address: "mqtt://192.0.2.20".to_string(),
            transport: BridgeTransport::LocalProcess,
            discovered_at_ms: 1_050,
        }
        .into_record()
        .unwrap();

        let error = run.push_record(mqtt_record).unwrap_err();

        assert_eq!(
            error,
            DiscoveryError::WorkerIntegrationMismatch {
                worker_integration_id: "hue".to_string(),
                record_integration_id: "mqtt".to_string()
            }
        );
        assert!(run.is_empty());
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
    fn discovery_records_project_fingerprints_and_freshness_signals() {
        let record = ManualBridgeInput {
            integration_id: IntegrationId::trusted("hue"),
            protocol_family: ProtocolFamily::Hue,
            native_bridge_id: "bridge-1".to_string(),
            address: "https://192.0.2.10".to_string(),
            transport: BridgeTransport::LanHttp,
            discovered_at_ms: 1_000,
        }
        .into_record()
        .unwrap()
        .with_metadata("source_detail", "manual-entry");

        let signal = record.signal(500);

        assert_eq!(record.fingerprint().as_str(), "manual:hue:bridge-1");
        assert_eq!(signal.fingerprint.as_str(), "manual:hue:bridge-1");
        assert_eq!(signal.stale_at_ms, 1_500);
        assert_eq!(signal.age_ms_at(1_250), 250);
        assert_eq!(signal.status_at(1_499), DiscoverySignalStatus::Fresh);
        assert_eq!(signal.status_at(1_500), DiscoverySignalStatus::Stale);
        assert_eq!(signal.next_transition_at_ms(1_250), Some(1_500));
        assert!(signal
            .metadata
            .contains(&Metadata::new("source_detail", "manual-entry")));
    }

    #[test]
    fn explicit_expiry_takes_priority_over_ttl_staleness() {
        let record = DiscoveryRecord::new(
            IntegrationId::trusted("matter"),
            ProtocolFamily::Matter,
            "fabric-candidate-1",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            1_000,
        )
        .unwrap()
        .with_expires_at_ms(1_200);

        let signal = record.signal(1_000);

        assert_eq!(signal.status_at(1_199), DiscoverySignalStatus::Fresh);
        assert_eq!(signal.status_at(1_200), DiscoverySignalStatus::Expired);
        assert_eq!(signal.next_transition_at_ms(1_001), Some(1_200));
        assert_eq!(signal.next_transition_at_ms(1_200), None);
    }

    #[test]
    fn discovery_catalog_summarizes_signal_freshness() {
        let mut catalog = DiscoveryCatalog::new();
        let fresh = DiscoveryRecord::new(
            IntegrationId::trusted("hue"),
            ProtocolFamily::Hue,
            "bridge-fresh",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            1_800,
        )
        .unwrap();
        let stale = DiscoveryRecord::new(
            IntegrationId::trusted("mqtt"),
            ProtocolFamily::Mqtt,
            "broker-stale",
            DiscoverySource::Mqtt,
            BridgeTransport::LocalProcess,
            1_000,
        )
        .unwrap()
        .with_expires_at_ms(3_000);
        let expired = DiscoveryRecord::new(
            IntegrationId::trusted("matter"),
            ProtocolFamily::Matter,
            "fabric-expired",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            1_700,
        )
        .unwrap()
        .with_expires_at_ms(1_900);

        catalog.record(fresh);
        catalog.record(stale);
        catalog.record(expired);

        let summary = catalog.signal_summary_at(2_000, 500);

        assert_eq!(
            summary,
            DiscoverySignalSummary {
                fresh: 1,
                stale: 1,
                expired: 1,
                next_transition_at_ms: Some(2_300),
            }
        );
        assert_eq!(catalog.signals(500).len(), 3);
    }

    #[test]
    fn discovery_catalog_summarizes_record_shape_for_planning() {
        let mut catalog = DiscoveryCatalog::new();
        let hue = DiscoveryRecord::new(
            IntegrationId::trusted("hue"),
            ProtocolFamily::Hue,
            "bridge-1",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            1_800,
        )
        .unwrap()
        .with_address("https://192.0.2.10")
        .with_confidence(DiscoveryConfidence::Verified)
        .with_pairing_requirement(PairingRequirement::PhysicalPresence);
        let mqtt = DiscoveryRecord::new(
            IntegrationId::trusted("mqtt"),
            ProtocolFamily::Mqtt,
            "broker-1",
            DiscoverySource::Mqtt,
            BridgeTransport::LocalProcess,
            1_000,
        )
        .unwrap()
        .with_confidence(DiscoveryConfidence::Candidate)
        .with_pairing_requirement(PairingRequirement::MqttCredentials);
        let matter = DiscoveryRecord::new(
            IntegrationId::trusted("matter"),
            ProtocolFamily::Matter,
            "fabric-expired",
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            1_700,
        )
        .unwrap()
        .with_confidence(DiscoveryConfidence::Hint)
        .with_pairing_requirement(PairingRequirement::Certificate)
        .with_expires_at_ms(1_900);

        catalog.record(hue);
        catalog.record(mqtt);
        catalog.record(matter);

        let summary = catalog.record_summary_at(2_000, 500);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.with_address, 1);
        assert_eq!(summary.fresh, 1);
        assert_eq!(summary.stale, 1);
        assert_eq!(summary.expired, 1);
        assert_eq!(summary.count_for_source(DiscoverySource::Mdns), 2);
        assert_eq!(summary.count_for_source(DiscoverySource::Mqtt), 1);
        assert_eq!(
            summary.count_for_confidence(DiscoveryConfidence::Verified),
            1
        );
        assert_eq!(
            summary.count_for_pairing_requirement(PairingRequirement::PhysicalPresence),
            1
        );
        assert_eq!(
            summary.count_for_pairing_requirement(PairingRequirement::MqttCredentials),
            1
        );
        assert!(!summary.is_empty());
    }

    #[test]
    fn discovery_record_summary_handles_empty_inputs() {
        let records: Vec<DiscoveryRecord> = Vec::new();
        let summary = DiscoveryRecordSummary::from_records(records.iter(), 2_000, 500);

        assert!(summary.is_empty());
        assert_eq!(summary.total, 0);
        assert_eq!(summary.count_for_source(DiscoverySource::Mdns), 0);
        assert_eq!(
            summary.count_for_pairing_requirement(PairingRequirement::PhysicalPresence),
            0
        );
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

    fn hue_mdns_response_packet() -> Vec<u8> {
        let mut packet = Vec::new();
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0x8400);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 1);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 3);

        let service_offset = packet.len();
        encode_dns_name("_hue._tcp.local", &mut packet).unwrap();
        push_record_fixed_header(&mut packet, MDNS_DNS_TYPE_PTR, 120);
        let ptr_len_offset = reserve_rdlength(&mut packet);
        let instance_offset = packet.len();
        encode_dns_name("Living Room Hue._hue._tcp.local", &mut packet).unwrap();
        fill_rdlength(&mut packet, ptr_len_offset);

        push_name_pointer(&mut packet, instance_offset);
        push_record_fixed_header(&mut packet, MDNS_DNS_TYPE_SRV, 120);
        let srv_len_offset = reserve_rdlength(&mut packet);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 0);
        push_u16(&mut packet, 443);
        let host_offset = packet.len();
        encode_dns_name("hue-bridge.local", &mut packet).unwrap();
        fill_rdlength(&mut packet, srv_len_offset);

        push_name_pointer(&mut packet, instance_offset);
        push_record_fixed_header(&mut packet, MDNS_DNS_TYPE_TXT, 120);
        let txt_len_offset = reserve_rdlength(&mut packet);
        push_txt_entry(&mut packet, "bridgeid", "001788fffeabcdef");
        push_txt_entry(&mut packet, "modelid", "BSB002");
        fill_rdlength(&mut packet, txt_len_offset);

        push_name_pointer(&mut packet, host_offset);
        push_record_fixed_header(&mut packet, MDNS_DNS_TYPE_A, 120);
        let a_len_offset = reserve_rdlength(&mut packet);
        packet.extend_from_slice(&[192, 0, 2, 10]);
        fill_rdlength(&mut packet, a_len_offset);

        assert_eq!(service_offset, 12);
        packet
    }

    fn push_record_fixed_header(packet: &mut Vec<u8>, record_type: u16, ttl: u32) {
        push_u16(packet, record_type);
        push_u16(packet, MDNS_DNS_CLASS_IN);
        packet.extend_from_slice(&ttl.to_be_bytes());
    }

    fn reserve_rdlength(packet: &mut Vec<u8>) -> usize {
        let offset = packet.len();
        push_u16(packet, 0);
        offset
    }

    fn fill_rdlength(packet: &mut [u8], len_offset: usize) {
        let data_start = len_offset + 2;
        let len = packet.len() - data_start;
        packet[len_offset..len_offset + 2].copy_from_slice(&(len as u16).to_be_bytes());
    }

    fn push_name_pointer(packet: &mut Vec<u8>, offset: usize) {
        assert!(offset <= 0x3FFF);
        let pointer = 0xC000 | offset as u16;
        push_u16(packet, pointer);
    }

    fn push_txt_entry(packet: &mut Vec<u8>, key: &str, value: &str) {
        let entry = format!("{key}={value}");
        assert!(entry.len() <= u8::MAX as usize);
        packet.push(entry.len() as u8);
        packet.extend_from_slice(entry.as_bytes());
    }

    fn push_u16(packet: &mut Vec<u8>, value: u16) {
        packet.extend_from_slice(&value.to_be_bytes());
    }
}
