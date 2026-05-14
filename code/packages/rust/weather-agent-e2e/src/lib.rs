#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use actor::{ActorResult, ActorStatus, ActorSystem, Behavior, Channel, Message};
use artifact_store::{
    AppendRevisionInput, ArtifactInventorySummary, ArtifactListOptions, ArtifactProvenance,
    ArtifactStore, CreateArtifactInput,
};
use capability_cage::{
    secure_file, Action as CageAction, Capability as CageCapability, CapabilityFlavor,
    CapabilityTrust, Category as CageCategory, Manifest,
};
use capability_os_sandbox::{
    current_kernel_sandbox_support, plan_all_supported, plan_for_current_os,
    run_with_kernel_sandbox, OsFamily, SandboxPlan, SandboxPlanSummary,
};
use chief_of_staff_tool_api::{
    InMemoryToolRuntime, JsonSchema, PrivilegeTier, RequestedBy, SchemaProperty, ToolApiError,
    ToolCallError, ToolConcurrency, ToolDefinition, ToolErrorKind, ToolEventKind,
    ToolExecutionJournal, ToolExecutionJournalHealthSummary, ToolHandlerOutput, ToolIdempotency,
    ToolInvocationRequest, ToolSideEffects, ToolStability, ToolStreaming,
};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use context_store::{
    AppendEntryInput, ContextEntryKind, ContextStore, ContextStoreInventorySummary,
    CreateSessionInput, CreateSnapshotInput, SessionListOptions,
};
use generic_job_protocol::{
    JobMetadata, JobRequest as ProtocolJobRequest, JobResult as ProtocolJobResult,
};
use generic_job_runtime::{
    ExecutorFleetStatusSummary, ExecutorFleetSummary, ExecutorLimits, RustThreadPool,
    RustThreadPoolOptions,
};
use http_core::BodyKind;
use memory_store::{
    MemoryClass, MemoryInventorySummary, MemoryListOptions, MemoryRecord, MemoryStore,
};
use os_job_core::{
    BackendKind, ConcurrencyPolicy, DateTimeParts, JobAction, JobSpec, JobTrigger, OutputPolicy,
    RetryPolicy,
};
use os_job_runtime::NativeJobRuntime;
use read_write_separation::{
    summarize_manifest, validate_manifest, Capability as RwsCapability, CapabilityManifestSummary,
    RwsViolation,
};
use skill_store::{
    InstallSkillAssetInput, SkillInventorySummary, SkillListOptions, SkillManifest, SkillStore,
};
use storage_core::{InMemoryStorageBackend, StorageError};
use tls_platform::TlsConfig;

const AGENT_ID: &str = "umbrella_today_agent";
const SESSION_ID: &str = "umbrella_today_session";
const JOB_ID: &str = "umbrella_today_job";
const USER_ID: &str = "seattle_user";
const FETCH_TOOL_ID: &str = "weather.fetch_current";
const CLASSIFY_TOOL_ID: &str = "weather.classify_umbrella";
const WRITE_TOOL_ID: &str = "file.write_text";
const WEATHER_API_DOMAIN: &str = "api.weather.gov";
const WEATHER_API_PORT: u16 = 443;

pub type UmbrellaResult<T> = Result<T, UmbrellaAgentError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmbrellaAgentError {
    message: String,
}

impl UmbrellaAgentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for UmbrellaAgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for UmbrellaAgentError {}

impl From<std::io::Error> for UmbrellaAgentError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

impl From<actor::ActorError> for UmbrellaAgentError {
    fn from(value: actor::ActorError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<StorageError> for UmbrellaAgentError {
    fn from(value: StorageError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<ToolApiError> for UmbrellaAgentError {
    fn from(value: ToolApiError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<generic_job_runtime::SubmitError> for UmbrellaAgentError {
    fn from(value: generic_job_runtime::SubmitError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<generic_job_runtime::RuntimeError> for UmbrellaAgentError {
    fn from(value: generic_job_runtime::RuntimeError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<os_job_core::JobError> for UmbrellaAgentError {
    fn from(value: os_job_core::JobError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<capability_cage::InvalidCombination> for UmbrellaAgentError {
    fn from(value: capability_cage::InvalidCombination) -> Self {
        Self::new(value.to_string())
    }
}

impl From<capability_cage::ManifestError> for UmbrellaAgentError {
    fn from(value: capability_cage::ManifestError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<capability_cage::CapabilityViolationError> for UmbrellaAgentError {
    fn from(value: capability_cage::CapabilityViolationError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<RwsViolation> for UmbrellaAgentError {
    fn from(value: RwsViolation) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmbrellaAgentConfig {
    pub location: String,
    pub output_path: PathBuf,
    pub tick_id: String,
    pub fetched_at_ms: u64,
    pub fetched_at_iso: String,
    pub weather_source: WeatherSource,
    pub supervisor_probe: UmbrellaSupervisorProbe,
}

impl UmbrellaAgentConfig {
    pub fn deterministic_seattle(output_path: impl Into<PathBuf>) -> Self {
        Self {
            location: "Seattle".to_string(),
            output_path: output_path.into(),
            tick_id: "8a7b0000000000000000000000000001".to_string(),
            fetched_at_ms: 1_778_624_400_000,
            fetched_at_iso: "2026-05-12T12:00:00.000Z".to_string(),
            weather_source: WeatherSource::Fixture(WeatherSnapshot::rainy_seattle_fixture()),
            supervisor_probe: UmbrellaSupervisorProbe::default(),
        }
    }

    pub fn live_seattle_with_timestamp(
        output_path: impl Into<PathBuf>,
        fetched_at_ms: u64,
        fetched_at_iso: impl Into<String>,
    ) -> Self {
        Self {
            location: "Seattle".to_string(),
            output_path: output_path.into(),
            tick_id: "8a7b0000000000000000000000000001".to_string(),
            fetched_at_ms,
            fetched_at_iso: fetched_at_iso.into(),
            weather_source: WeatherSource::LiveNws {
                user_agent: "coding-adventures-weather-agent-e2e/0.1 (adhithyan15)".to_string(),
            },
            supervisor_probe: UmbrellaSupervisorProbe::default(),
        }
    }

    pub fn with_killed_child_before_tick(mut self, actor_id: impl Into<String>) -> Self {
        self.supervisor_probe.kill_child_before_tick = Some(actor_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeatherSource {
    Fixture(WeatherSnapshot),
    LiveNws { user_agent: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UmbrellaSupervisorProbe {
    pub kill_child_before_tick: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherFetchSourceKind {
    Fixture,
    LiveHttps,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherFetchSummary {
    pub source_kind: WeatherFetchSourceKind,
    pub https_requests: usize,
    pub tls_handshakes: usize,
    pub http_statuses: Vec<u16>,
    pub endpoint_count: usize,
    pub forecast_endpoint: String,
    pub http_domain_policy_enforced: bool,
    pub declared_http_domains: Vec<String>,
}

impl WeatherFetchSummary {
    fn fixture(snapshot: &WeatherSnapshot) -> Self {
        Self {
            source_kind: WeatherFetchSourceKind::Fixture,
            https_requests: 0,
            tls_handshakes: 0,
            http_statuses: vec![snapshot.http_status],
            endpoint_count: 1,
            forecast_endpoint: snapshot.endpoint_url.clone(),
            http_domain_policy_enforced: false,
            declared_http_domains: Vec::new(),
        }
    }

    fn live(
        points_status: u16,
        forecast: &WeatherSnapshot,
        domain_policy: &DeclaredHttpDomainPolicy,
    ) -> Self {
        Self {
            source_kind: WeatherFetchSourceKind::LiveHttps,
            https_requests: 2,
            tls_handshakes: 2,
            http_statuses: vec![points_status, forecast.http_status],
            endpoint_count: 2,
            forecast_endpoint: forecast.endpoint_url.clone(),
            http_domain_policy_enforced: true,
            declared_http_domains: domain_policy.declared_domains().to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredHttpDomainPolicy {
    manifest: Manifest,
    declared_domains: Vec<String>,
}

impl DeclaredHttpDomainPolicy {
    fn from_required_capabilities_json(manifest_json: &str) -> UmbrellaResult<Self> {
        let manifest = Manifest::load_from_str(manifest_json)?;
        let mut declared_domains = Vec::new();

        for capability in manifest.capabilities() {
            if capability.category != CageCategory::Net {
                continue;
            }
            match capability.action {
                CageAction::Dns => {
                    let domain = normalize_exact_domain(&capability.target)?;
                    push_unique(&mut declared_domains, domain);
                }
                CageAction::Connect => {
                    let endpoint =
                        DeclaredHttpEndpoint::from_capability_target(&capability.target)?;
                    push_unique(&mut declared_domains, endpoint.host);
                }
                _ => {}
            }
        }

        if declared_domains.is_empty() {
            return Err(UmbrellaAgentError::new(
                "HTTP domain policy found no net:dns or net:connect entries in required_capabilities.json",
            ));
        }

        Ok(Self {
            manifest,
            declared_domains,
        })
    }

    fn declared_domains(&self) -> &[String] {
        &self.declared_domains
    }

    fn enforce_https_url(&self, url: &str) -> UmbrellaResult<DeclaredHttpRequest> {
        let request = DeclaredHttpRequest::parse(url)?;
        self.manifest
            .check(CageCategory::Net, CageAction::Dns, &request.host)
            .map_err(UmbrellaAgentError::from)?;
        self.manifest
            .check(CageCategory::Net, CageAction::Connect, &request.authority)
            .map_err(UmbrellaAgentError::from)?;
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredHttpEndpoint {
    host: String,
    port: u16,
}

impl DeclaredHttpEndpoint {
    fn from_capability_target(target: &str) -> UmbrellaResult<Self> {
        let (host, port_text) = target.rsplit_once(':').ok_or_else(|| {
            UmbrellaAgentError::new(format!(
                "HTTP connect capability target must include host:port, got {target}"
            ))
        })?;
        let host = normalize_exact_domain(host)?;
        let port = port_text.parse::<u16>().map_err(|error| {
            UmbrellaAgentError::new(format!(
                "HTTP connect capability target had invalid port '{port_text}': {error}"
            ))
        })?;
        if port == 0 {
            return Err(UmbrellaAgentError::new(
                "HTTP connect capability target used invalid port 0",
            ));
        }
        Ok(Self { host, port })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredHttpRequest {
    host: String,
    port: u16,
    authority: String,
    path_and_query: String,
}

impl DeclaredHttpRequest {
    fn parse(url: &str) -> UmbrellaResult<Self> {
        let rest = url.strip_prefix("https://").ok_or_else(|| {
            UmbrellaAgentError::new(format!(
                "HTTP client only permits HTTPS URLs declared in required_capabilities.json, got {url}"
            ))
        })?;
        let path_start = rest
            .char_indices()
            .find_map(|(index, ch)| matches!(ch, '/' | '?' | '#').then_some(index));
        let (authority, path_and_query) = match path_start {
            Some(index) => {
                let suffix = &rest[index..];
                let path = if suffix.starts_with('?') {
                    format!("/{suffix}")
                } else {
                    suffix.to_string()
                };
                (&rest[..index], path)
            }
            None => (rest, "/".to_string()),
        };
        if authority.is_empty() {
            return Err(UmbrellaAgentError::new(format!(
                "HTTPS URL has no authority: {url}"
            )));
        }
        if authority.contains('@') {
            return Err(UmbrellaAgentError::new(format!(
                "HTTPS URL userinfo is not allowed by the declared-domain policy: {url}"
            )));
        }
        let endpoint = parse_https_authority(authority)?;
        let path_and_query = if path_and_query.is_empty() {
            "/".to_string()
        } else {
            path_and_query
        };
        if path_and_query.contains('#') {
            return Err(UmbrellaAgentError::new(format!(
                "HTTPS URL fragments are not sent over HTTP and are not allowed: {url}"
            )));
        }
        Ok(Self {
            authority: format!("{}:{}", endpoint.host, endpoint.port),
            host: endpoint.host,
            port: endpoint.port,
            path_and_query,
        })
    }
}

fn parse_https_authority(authority: &str) -> UmbrellaResult<DeclaredHttpEndpoint> {
    if authority.starts_with('[') || authority.contains(']') {
        return Err(UmbrellaAgentError::new(format!(
            "HTTP declared-domain policy only accepts DNS names, got {authority}"
        )));
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port_text)) => {
            let port = port_text.parse::<u16>().map_err(|error| {
                UmbrellaAgentError::new(format!(
                    "HTTPS URL authority had invalid port '{port_text}': {error}"
                ))
            })?;
            (host, port)
        }
        None => (authority, WEATHER_API_PORT),
    };
    if port == 0 {
        return Err(UmbrellaAgentError::new(format!(
            "HTTPS URL authority used invalid port {port}"
        )));
    }
    Ok(DeclaredHttpEndpoint {
        host: normalize_exact_domain(host)?,
        port,
    })
}

fn normalize_exact_domain(domain: &str) -> UmbrellaResult<String> {
    let normalized = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(UmbrellaAgentError::new(
            "HTTP domain capability target must not be empty",
        ));
    }
    if normalized.contains('*') {
        return Err(UmbrellaAgentError::new(format!(
            "HTTP domain policy requires exact domains; wildcard target '{domain}' is not allowed"
        )));
    }
    if normalized
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || matches!(ch, '/' | '\\' | ':' | '@' | '?' | '#'))
    {
        return Err(UmbrellaAgentError::new(format!(
            "HTTP domain capability target is not a DNS name: {domain}"
        )));
    }
    Ok(normalized)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WeatherFetchPayload {
    snapshot: WeatherSnapshot,
    summary: WeatherFetchSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherSnapshot {
    pub location: String,
    pub endpoint_url: String,
    pub http_status: u16,
    pub fetched_at_ms: u64,
    pub fetched_at_iso: String,
    pub high_temp_f: i64,
    pub precip_pct: u8,
    pub raw_body: String,
}

impl WeatherSnapshot {
    pub fn rainy_seattle_fixture() -> Self {
        Self {
            location: "Seattle".to_string(),
            endpoint_url: "https://api.weather.gov/gridpoints/SEW/124,67/forecast".to_string(),
            http_status: 200,
            fetched_at_ms: 1_778_624_400_000,
            fetched_at_iso: "2026-05-12T12:00:00.000Z".to_string(),
            high_temp_f: 52,
            precip_pct: 72,
            raw_body: "Seattle forecast fixture: cool rain likely today.".to_string(),
        }
    }

    fn to_json(&self) -> JsonValue {
        object(vec![
            ("location", string(&self.location)),
            ("endpoint_url", string(&self.endpoint_url)),
            ("http_status", int(self.http_status as i64)),
            ("fetched_at_ms", int(self.fetched_at_ms as i64)),
            ("fetched_at_iso", string(&self.fetched_at_iso)),
            ("high_temp_f", int(self.high_temp_f)),
            ("precip_pct", int(self.precip_pct as i64)),
            ("raw_body", string(&self.raw_body)),
        ])
    }

    fn from_json(value: &JsonValue) -> Result<Self, ToolCallError> {
        Ok(Self {
            location: field_string(value, "location")?,
            endpoint_url: field_string(value, "endpoint_url")?,
            http_status: field_i64(value, "http_status")? as u16,
            fetched_at_ms: field_i64(value, "fetched_at_ms")? as u64,
            fetched_at_iso: field_string(value, "fetched_at_iso")?,
            high_temp_f: field_i64(value, "high_temp_f")?,
            precip_pct: field_i64(value, "precip_pct")? as u8,
            raw_body: field_string(value, "raw_body")?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationKind {
    NoAction,
    JacketOnly,
    UmbrellaOnly,
    Both,
}

impl RecommendationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoAction => "NoAction",
            Self::JacketOnly => "JacketOnly",
            Self::UmbrellaOnly => "UmbrellaOnly",
            Self::Both => "Both",
        }
    }

    pub fn needs_umbrella(self) -> bool {
        matches!(self, Self::UmbrellaOnly | Self::Both)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmbrellaRecommendation {
    pub kind: RecommendationKind,
    pub location: String,
    pub high_temp_f: i64,
    pub precip_pct: u8,
    pub fetched_at_ms: u64,
    pub fetched_at_iso: String,
    pub tick_id: String,
    pub explanation: String,
}

impl UmbrellaRecommendation {
    fn from_snapshot(snapshot: &WeatherSnapshot, tick_id: &str) -> Self {
        let needs_umbrella = snapshot.precip_pct >= 40;
        let needs_jacket = snapshot.high_temp_f < 60;
        let kind = match (needs_umbrella, needs_jacket) {
            (false, false) => RecommendationKind::NoAction,
            (false, true) => RecommendationKind::JacketOnly,
            (true, false) => RecommendationKind::UmbrellaOnly,
            (true, true) => RecommendationKind::Both,
        };
        let explanation = if kind.needs_umbrella() {
            format!(
                "Bring an umbrella today in {}: precipitation chance is {}%.",
                snapshot.location, snapshot.precip_pct
            )
        } else {
            format!(
                "No umbrella needed today in {}: precipitation chance is {}%.",
                snapshot.location, snapshot.precip_pct
            )
        };

        Self {
            kind,
            location: snapshot.location.clone(),
            high_temp_f: snapshot.high_temp_f,
            precip_pct: snapshot.precip_pct,
            fetched_at_ms: snapshot.fetched_at_ms,
            fetched_at_iso: snapshot.fetched_at_iso.clone(),
            tick_id: tick_id.to_string(),
            explanation,
        }
    }

    fn to_json(&self) -> JsonValue {
        object(vec![
            ("kind", string(self.kind.as_str())),
            ("location", string(&self.location)),
            ("high_temp_f", int(self.high_temp_f)),
            ("precip_pct", int(self.precip_pct as i64)),
            ("fetched_at_ms", int(self.fetched_at_ms as i64)),
            ("fetched_at_iso", string(&self.fetched_at_iso)),
            ("tick_id", string(&self.tick_id)),
            ("explanation", string(&self.explanation)),
        ])
    }

    fn from_json(value: &JsonValue) -> Result<Self, ToolCallError> {
        let kind = match field_string(value, "kind")?.as_str() {
            "NoAction" => RecommendationKind::NoAction,
            "JacketOnly" => RecommendationKind::JacketOnly,
            "UmbrellaOnly" => RecommendationKind::UmbrellaOnly,
            "Both" => RecommendationKind::Both,
            other => {
                return Err(ToolCallError::new(
                    ToolErrorKind::ToolValidationError,
                    format!("unsupported recommendation kind '{other}'"),
                ))
            }
        };
        Ok(Self {
            kind,
            location: field_string(value, "location")?,
            high_temp_f: field_i64(value, "high_temp_f")?,
            precip_pct: field_i64(value, "precip_pct")? as u8,
            fetched_at_ms: field_i64(value, "fetched_at_ms")? as u64,
            fetched_at_iso: field_string(value, "fetched_at_iso")?,
            tick_id: field_string(value, "tick_id")?,
            explanation: field_string(value, "explanation")?,
        })
    }

    pub fn log_line(&self) -> String {
        format!(
            "{}  tick={} kind={:<12} high_f={} precip_pct={} location={} decision={}",
            self.fetched_at_iso,
            self.tick_id,
            self.kind.as_str(),
            self.high_temp_f,
            self.precip_pct,
            self.location,
            self.explanation
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRwsSummary {
    pub fetcher: CapabilityManifestSummary,
    pub classifier: CapabilityManifestSummary,
    pub writer: CapabilityManifestSummary,
    pub combined_manifest_rejected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmbrellaSupervisorSummary {
    pub child_count: usize,
    pub stopped_children: usize,
    pub failed_children: usize,
    pub messages_processed: u64,
    pub dead_letters: usize,
    pub killed_children: Vec<String>,
    pub restarted_children: Vec<String>,
    pub restart_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmbrellaAgentRun {
    pub recommendation: UmbrellaRecommendation,
    pub output_path: PathBuf,
    pub output_text: String,
    pub weather_fetch: WeatherFetchSummary,
    pub sandbox_plan: UmbrellaSandboxPlanSummary,
    pub kernel_sandbox: UmbrellaKernelSandboxSummary,
    pub supervisor: UmbrellaSupervisorSummary,
    pub tool_journal_health: ToolExecutionJournalHealthSummary,
    pub context_inventory: ContextStoreInventorySummary,
    pub artifact_inventory: ArtifactInventorySummary,
    pub memory_inventory: MemoryInventorySummary,
    pub skill_inventory: SkillInventorySummary,
    pub job_plan_backend: BackendKind,
    pub job_plan_file_count: usize,
    pub job_executor_status: ExecutorFleetStatusSummary,
    pub rws: HostRwsSummary,
    pub actor_channel_messages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmbrellaSandboxPlanSummary {
    pub package: String,
    pub plan_count: usize,
    pub current_os: OsFamily,
    pub current_os_rules: usize,
    pub total_rules: usize,
    pub direct_rules: usize,
    pub brokered_rules: usize,
    pub launch_time_rules: usize,
    pub advisory_rules: usize,
    pub native_rules: usize,
    pub host_broker_rules: usize,
    pub os_summaries: Vec<SandboxPlanSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmbrellaKernelSandboxSummary {
    pub os: OsFamily,
    pub primitive: String,
    pub available: bool,
    pub enforced: bool,
    pub allowed_write_succeeded: bool,
    pub denied_write_blocked: bool,
    pub network_outbound_enforced: bool,
    pub network_denied_outbound_blocked: bool,
    pub weather_https_allowed: bool,
    pub host_exact_kernel_enforced: bool,
    pub denied_path: PathBuf,
    pub denied_network_target: String,
    pub status_code: Option<i32>,
    pub stderr_contains_operation_not_permitted: bool,
    pub reason: String,
}

impl UmbrellaSandboxPlanSummary {
    fn from_summaries(os_summaries: Vec<SandboxPlanSummary>, current_os: OsFamily) -> Self {
        let package = os_summaries
            .first()
            .map(|summary| summary.package.clone())
            .unwrap_or_default();
        let current_os_rules = os_summaries
            .iter()
            .find(|summary| summary.os == current_os)
            .map(|summary| summary.total_rules)
            .unwrap_or_default();
        Self {
            package,
            plan_count: os_summaries.len(),
            current_os,
            current_os_rules,
            total_rules: os_summaries.iter().map(|summary| summary.total_rules).sum(),
            direct_rules: os_summaries
                .iter()
                .map(|summary| summary.direct_rules)
                .sum(),
            brokered_rules: os_summaries
                .iter()
                .map(|summary| summary.brokered_rules)
                .sum(),
            launch_time_rules: os_summaries
                .iter()
                .map(|summary| summary.launch_time_rules)
                .sum(),
            advisory_rules: os_summaries
                .iter()
                .map(|summary| summary.advisory_rules)
                .sum(),
            native_rules: os_summaries
                .iter()
                .map(|summary| summary.native_rules)
                .sum(),
            host_broker_rules: os_summaries
                .iter()
                .map(|summary| summary.host_broker_rules)
                .sum(),
            os_summaries,
        }
    }

    pub fn summary_for_os(&self, os: OsFamily) -> Option<&SandboxPlanSummary> {
        self.os_summaries.iter().find(|summary| summary.os == os)
    }
}

pub fn run_umbrella_today_agent(config: UmbrellaAgentConfig) -> UmbrellaResult<UmbrellaAgentRun> {
    let pipeline = Arc::new(UmbrellaPipeline::new(config.clone())?);
    pipeline.bootstrap_substrate()?;

    let sandbox_plan = plan_agent_sandbox(&config)?;
    let kernel_sandbox = probe_agent_kernel_sandbox(&config)?;
    let (job_plan_backend, job_plan_file_count) = plan_agent_job(&config)?;
    let job_executor_status = run_executor_tick(&config)?;

    let mut system = ActorSystem::new();
    let mut supervisor_events = SupervisorEvents::default();
    spawn_weather_children(&mut system, &pipeline)?;
    if let Some(actor_id) = &config.supervisor_probe.kill_child_before_tick {
        system.stop_actor(actor_id)?;
        supervisor_events.killed_children.push(actor_id.clone());
    }
    restart_stopped_weather_children(&mut system, &pipeline, &mut supervisor_events)?;
    system.send(
        "weather-fetcher",
        Message::text("umbrella-supervisor", "tick"),
    )?;
    let actor_stats = system.run_until_done();

    pipeline.raise_actor_errors()?;

    let recommendation = pipeline.recommendation()?;
    let output_text = fs::read_to_string(&config.output_path)?;
    let weather_fetch = pipeline.weather_fetch_summary()?;
    let tool_journal_health = pipeline
        .journal
        .lock()
        .expect("tool journal mutex poisoned")
        .health_summary();
    let context_inventory = pipeline
        .context_store
        .inventory_summary(SessionListOptions::new().for_owner(USER_ID))?;
    let artifact_inventory = pipeline
        .artifact_store
        .inventory_summary(ArtifactListOptions::new().for_agent(AGENT_ID))?;
    let memory_inventory = pipeline.memory_store.inventory_summary(
        MemoryListOptions::new().with_tag("umbrella"),
        config.fetched_at_ms,
    )?;
    let skill_inventory = pipeline
        .skill_store
        .inventory_summary(SkillListOptions::new())?;
    let actor_channel_messages = pipeline
        .channel
        .lock()
        .expect("actor channel mutex poisoned")
        .len();

    Ok(UmbrellaAgentRun {
        recommendation,
        output_path: config.output_path,
        output_text,
        weather_fetch,
        sandbox_plan,
        kernel_sandbox,
        supervisor: supervisor_summary(&system, &actor_stats, supervisor_events),
        tool_journal_health,
        context_inventory,
        artifact_inventory,
        memory_inventory,
        skill_inventory,
        job_plan_backend,
        job_plan_file_count,
        job_executor_status,
        rws: validate_host_boundaries(&pipeline.config.output_path)?,
        actor_channel_messages,
    })
}

struct UmbrellaPipeline {
    config: UmbrellaAgentConfig,
    tool_runtime: InMemoryToolRuntime,
    journal: Mutex<ToolExecutionJournal>,
    channel: Mutex<Channel>,
    context_store: ContextStore<InMemoryStorageBackend>,
    artifact_store: ArtifactStore<InMemoryStorageBackend>,
    memory_store: MemoryStore<InMemoryStorageBackend>,
    skill_store: SkillStore<InMemoryStorageBackend>,
    snapshot: Mutex<Option<WeatherSnapshot>>,
    weather_fetch_summary: Mutex<Option<WeatherFetchSummary>>,
    recommendation: Mutex<Option<UmbrellaRecommendation>>,
    actor_errors: Mutex<Vec<String>>,
}

impl UmbrellaPipeline {
    fn new(config: UmbrellaAgentConfig) -> UmbrellaResult<Self> {
        let mut tool_runtime = InMemoryToolRuntime::new();
        let http_domain_policy = umbrella_agent_http_domain_policy(&config.output_path)?;
        register_weather_fetch_tool(
            &mut tool_runtime,
            config.weather_source.clone(),
            config.fetched_at_ms,
            config.fetched_at_iso.clone(),
            http_domain_policy,
        )
        .map_err(UmbrellaAgentError::from)?;
        register_weather_classifier_tool(&mut tool_runtime)?;
        register_file_writer_tool(&mut tool_runtime, writer_manifest(&config.output_path)?)?;

        Ok(Self {
            config,
            tool_runtime,
            journal: Mutex::new(ToolExecutionJournal::new()),
            channel: Mutex::new(Channel::new(
                "umbrella_today_events",
                "umbrella-today-events",
            )),
            context_store: ContextStore::new(InMemoryStorageBackend::new()),
            artifact_store: ArtifactStore::new(InMemoryStorageBackend::new()),
            memory_store: MemoryStore::new(InMemoryStorageBackend::new()),
            skill_store: SkillStore::new(InMemoryStorageBackend::new()),
            snapshot: Mutex::new(None),
            weather_fetch_summary: Mutex::new(None),
            recommendation: Mutex::new(None),
            actor_errors: Mutex::new(Vec::new()),
        })
    }

    fn bootstrap_substrate(&self) -> UmbrellaResult<()> {
        self.context_store.create_session(CreateSessionInput {
            session_id: SESSION_ID.to_string(),
            owner_id: USER_ID.to_string(),
            title: "Umbrella today agent".to_string(),
        })?;
        self.append_context(
            "user_request",
            ContextEntryKind::User,
            object(vec![
                ("request", string("Do I need an umbrella today?")),
                ("location", string(&self.config.location)),
            ]),
        )?;
        self.memory_store.remember(MemoryRecord {
            memory_id: "user_lives_in_seattle".to_string(),
            class: MemoryClass::Profile,
            subject: "user_location".to_string(),
            body: "The user lives in Seattle, so umbrella checks default to Seattle.".to_string(),
            confidence: 1.0,
            source_refs: vec!["user_request".to_string()],
            tags: vec![
                "umbrella".to_string(),
                "seattle".to_string(),
                "profile".to_string(),
            ],
            supersedes: vec![],
            created_at: self.config.fetched_at_ms,
            reviewed_at: Some(self.config.fetched_at_ms),
            expires_at: None,
            tombstoned: false,
        })?;
        self.memory_store.remember(MemoryRecord {
            memory_id: "umbrella_threshold_rule".to_string(),
            class: MemoryClass::Procedure,
            subject: "umbrella_rule".to_string(),
            body: "Recommend an umbrella when precipitation chance is at least 40%.".to_string(),
            confidence: 1.0,
            source_refs: vec!["weather_agent_spec".to_string()],
            tags: vec![
                "umbrella".to_string(),
                "weather".to_string(),
                "procedure".to_string(),
            ],
            supersedes: vec![],
            created_at: self.config.fetched_at_ms,
            reviewed_at: Some(self.config.fetched_at_ms),
            expires_at: None,
            tombstoned: false,
        })?;
        self.skill_store.install_skill(
            SkillManifest {
                skill_id: "umbrella_today".to_string(),
                version: "v1".to_string(),
                name: "Umbrella Today".to_string(),
                description:
                    "Fetch Seattle weather, classify umbrella need, and write a text report."
                        .to_string(),
                entrypoints: vec!["run_once".to_string()],
                required_tools: vec![
                    FETCH_TOOL_ID.to_string(),
                    CLASSIFY_TOOL_ID.to_string(),
                    WRITE_TOOL_ID.to_string(),
                ],
                required_capabilities: vec![
                    "weather_api_read".to_string(),
                    "filesystem_write".to_string(),
                ],
                assets: vec!["instructions.md".to_string()],
                source: object(vec![("spec", string("code/specs/weather-agent.md"))]),
                active: true,
            },
            vec![InstallSkillAssetInput {
                asset_path: "instructions.md".to_string(),
                content_type: "text/markdown".to_string(),
                body: b"Run once, use Seattle weather, then write the umbrella decision.".to_vec(),
            }],
        )?;
        Ok(())
    }

    fn fetch_step(&self) -> UmbrellaResult<()> {
        self.append_context(
            "fetch_tool_call",
            ContextEntryKind::ToolCall,
            object(vec![
                ("tool_id", string(FETCH_TOOL_ID)),
                ("location", string(&self.config.location)),
            ]),
        )?;
        let output = self.invoke_tool(
            "fetch_current_weather",
            FETCH_TOOL_ID,
            object(vec![("location", string(&self.config.location))]),
        )?;
        let snapshot = WeatherSnapshot::from_json(&output).map_err(tool_error_to_agent)?;
        let fetch_summary = match &self.config.weather_source {
            WeatherSource::Fixture(_) => WeatherFetchSummary::fixture(&snapshot),
            WeatherSource::LiveNws { .. } => {
                let http_domain_policy =
                    umbrella_agent_http_domain_policy(&self.config.output_path)?;
                WeatherFetchSummary::live(200, &snapshot, &http_domain_policy)
            }
        };
        *self.snapshot.lock().expect("snapshot mutex poisoned") = Some(snapshot.clone());
        *self
            .weather_fetch_summary
            .lock()
            .expect("weather fetch summary mutex poisoned") = Some(fetch_summary);
        self.append_context("fetch_tool_result", ContextEntryKind::ToolResult, output)?;
        self.append_channel(
            "weather-fetcher",
            format!("snapshot:{}", snapshot.endpoint_url),
        );
        Ok(())
    }

    fn classify_step(&self) -> UmbrellaResult<()> {
        let snapshot = self.snapshot()?;
        self.append_context(
            "classify_tool_call",
            ContextEntryKind::ToolCall,
            object(vec![("tool_id", string(CLASSIFY_TOOL_ID))]),
        )?;
        let output = self.invoke_tool(
            "classify_umbrella_need",
            CLASSIFY_TOOL_ID,
            object(vec![
                ("snapshot", snapshot.to_json()),
                ("tick_id", string(&self.config.tick_id)),
            ]),
        )?;
        let recommendation =
            UmbrellaRecommendation::from_json(&output).map_err(tool_error_to_agent)?;
        *self
            .recommendation
            .lock()
            .expect("recommendation mutex poisoned") = Some(recommendation.clone());
        self.append_context("classify_tool_result", ContextEntryKind::ToolResult, output)?;
        self.append_channel(
            "weather-classifier",
            format!("recommendation:{}", recommendation.kind.as_str()),
        );
        Ok(())
    }

    fn write_step(&self) -> UmbrellaResult<()> {
        let recommendation = self.recommendation()?;
        let line = recommendation.log_line();
        self.append_context(
            "write_tool_call",
            ContextEntryKind::ToolCall,
            object(vec![
                ("tool_id", string(WRITE_TOOL_ID)),
                (
                    "output_path",
                    string(self.config.output_path.to_string_lossy().as_ref()),
                ),
            ]),
        )?;
        let output = self.invoke_tool(
            "write_umbrella_report",
            WRITE_TOOL_ID,
            object(vec![
                (
                    "output_path",
                    string(self.config.output_path.to_string_lossy().as_ref()),
                ),
                ("line", string(&line)),
            ]),
        )?;
        self.append_context("write_tool_result", ContextEntryKind::ToolResult, output)?;
        self.artifact_store.create_artifact(CreateArtifactInput {
            artifact_id: "umbrella_today_report".to_string(),
            collection: "weather".to_string(),
            name: "umbrella-today.txt".to_string(),
            content_type: "text/plain".to_string(),
            labels: vec![
                "umbrella".to_string(),
                "weather".to_string(),
                "e2e".to_string(),
            ],
            provenance: ArtifactProvenance {
                session_id: Some(SESSION_ID.to_string()),
                tool_id: Some(WRITE_TOOL_ID.to_string()),
                job_id: Some(JOB_ID.to_string()),
                agent_id: Some(AGENT_ID.to_string()),
            },
        })?;
        self.artifact_store.append_revision(
            "umbrella_today_report",
            AppendRevisionInput {
                revision_id: "rev1".to_string(),
                metadata: object(vec![("tick_id", string(&self.config.tick_id))]),
                body: line.as_bytes().to_vec(),
            },
        )?;
        self.append_context(
            "assistant_final",
            ContextEntryKind::Assistant,
            object(vec![
                (
                    "needs_umbrella",
                    JsonValue::Bool(recommendation.kind.needs_umbrella()),
                ),
                ("decision", string(&recommendation.explanation)),
                (
                    "output_path",
                    string(self.config.output_path.to_string_lossy().as_ref()),
                ),
            ]),
        )?;
        self.context_store.create_snapshot(
            SESSION_ID,
            CreateSnapshotInput {
                snapshot_id: "umbrella_today_snapshot".to_string(),
                basis_entry_id: "assistant_final".to_string(),
                token_estimate: 256,
                included_entry_ids: vec![
                    "user_request".to_string(),
                    "fetch_tool_call".to_string(),
                    "fetch_tool_result".to_string(),
                    "classify_tool_call".to_string(),
                    "classify_tool_result".to_string(),
                    "write_tool_call".to_string(),
                    "write_tool_result".to_string(),
                    "assistant_final".to_string(),
                ],
                summary_refs: vec!["umbrella_today_summary".to_string()],
                memory_refs: vec![
                    "user_lives_in_seattle".to_string(),
                    "umbrella_threshold_rule".to_string(),
                ],
                artifact_refs: vec!["umbrella_today_report".to_string()],
            },
        )?;
        self.append_channel(
            "file-writer",
            format!("written:{}", self.config.output_path.display()),
        );
        Ok(())
    }

    fn invoke_tool(
        &self,
        call_id: &str,
        tool_id: &str,
        arguments: JsonValue,
    ) -> UmbrellaResult<JsonValue> {
        let request = ToolInvocationRequest {
            call_id: call_id.to_string(),
            tool_id: tool_id.to_string(),
            arguments,
            requested_by: RequestedBy::Agent,
            session_id: Some(SESSION_ID.to_string()),
            job_id: Some(JOB_ID.to_string()),
            agent_id: Some(AGENT_ID.to_string()),
            user_id: Some(USER_ID.to_string()),
            requested_at: self.config.fetched_at_ms,
            deadline_at: Some(self.config.fetched_at_ms.saturating_add(30_000)),
            idempotency_key: Some(format!("{}-{}", self.config.tick_id, call_id)),
        };
        let trace = self.tool_runtime.invoke_with_events(&request);
        let result = trace.result.clone();
        self.journal
            .lock()
            .expect("tool journal mutex poisoned")
            .record_trace(request, trace);
        if result.ok {
            Ok(result.output.unwrap_or(JsonValue::Null))
        } else {
            Err(UmbrellaAgentError::new(
                result
                    .error
                    .map(|err| err.message)
                    .unwrap_or_else(|| "tool call failed without error details".to_string()),
            ))
        }
    }

    fn append_context(
        &self,
        entry_id: &str,
        kind: ContextEntryKind,
        body: JsonValue,
    ) -> UmbrellaResult<()> {
        self.context_store.append_entry(
            SESSION_ID,
            AppendEntryInput {
                entry_id: entry_id.to_string(),
                kind,
                timestamp: Some(self.config.fetched_at_ms),
                metadata: object(vec![
                    ("agent_id", string(AGENT_ID)),
                    ("tick_id", string(&self.config.tick_id)),
                ]),
                body,
            },
        )?;
        Ok(())
    }

    fn append_channel(&self, sender: &str, payload: String) {
        self.channel
            .lock()
            .expect("actor channel mutex poisoned")
            .append(Message::text(sender, &payload));
    }

    fn record_actor_error(&self, error: impl Into<String>) {
        self.actor_errors
            .lock()
            .expect("actor errors mutex poisoned")
            .push(error.into());
    }

    fn raise_actor_errors(&self) -> UmbrellaResult<()> {
        let errors = self
            .actor_errors
            .lock()
            .expect("actor errors mutex poisoned");
        if errors.is_empty() {
            Ok(())
        } else {
            Err(UmbrellaAgentError::new(errors.join("; ")))
        }
    }

    fn snapshot(&self) -> UmbrellaResult<WeatherSnapshot> {
        self.snapshot
            .lock()
            .expect("snapshot mutex poisoned")
            .clone()
            .ok_or_else(|| UmbrellaAgentError::new("weather snapshot was not produced"))
    }

    fn recommendation(&self) -> UmbrellaResult<UmbrellaRecommendation> {
        self.recommendation
            .lock()
            .expect("recommendation mutex poisoned")
            .clone()
            .ok_or_else(|| UmbrellaAgentError::new("umbrella recommendation was not produced"))
    }

    fn weather_fetch_summary(&self) -> UmbrellaResult<WeatherFetchSummary> {
        self.weather_fetch_summary
            .lock()
            .expect("weather fetch summary mutex poisoned")
            .clone()
            .ok_or_else(|| UmbrellaAgentError::new("weather fetch summary was not produced"))
    }
}

fn fetcher_behavior() -> Behavior {
    Box::new(|state, _msg| {
        let pipeline = unwrap_pipeline_state(state);
        let mut messages = Vec::new();
        if let Err(error) = pipeline.fetch_step() {
            pipeline.record_actor_error(format!("fetcher failed: {error}"));
        } else {
            messages.push((
                "weather-classifier".to_string(),
                Message::text("weather-fetcher", "snapshot-ready"),
            ));
        }
        ActorResult {
            new_state: Box::new(pipeline),
            messages_to_send: messages,
            actors_to_create: vec![],
            stop: true,
        }
    })
}

fn classifier_behavior() -> Behavior {
    Box::new(|state, _msg| {
        let pipeline = unwrap_pipeline_state(state);
        let mut messages = Vec::new();
        if let Err(error) = pipeline.classify_step() {
            pipeline.record_actor_error(format!("classifier failed: {error}"));
        } else {
            messages.push((
                "file-writer".to_string(),
                Message::text("weather-classifier", "recommendation-ready"),
            ));
        }
        ActorResult {
            new_state: Box::new(pipeline),
            messages_to_send: messages,
            actors_to_create: vec![],
            stop: true,
        }
    })
}

fn writer_behavior() -> Behavior {
    Box::new(|state, _msg| {
        let pipeline = unwrap_pipeline_state(state);
        if let Err(error) = pipeline.write_step() {
            pipeline.record_actor_error(format!("writer failed: {error}"));
        }
        ActorResult {
            new_state: Box::new(pipeline),
            messages_to_send: vec![],
            actors_to_create: vec![],
            stop: true,
        }
    })
}

#[derive(Debug, Default)]
struct SupervisorEvents {
    killed_children: Vec<String>,
    restarted_children: Vec<String>,
}

fn spawn_weather_children(
    system: &mut ActorSystem,
    pipeline: &Arc<UmbrellaPipeline>,
) -> UmbrellaResult<()> {
    create_weather_child(system, "weather-fetcher", pipeline)?;
    create_weather_child(system, "weather-classifier", pipeline)?;
    create_weather_child(system, "file-writer", pipeline)?;
    Ok(())
}

fn restart_stopped_weather_children(
    system: &mut ActorSystem,
    pipeline: &Arc<UmbrellaPipeline>,
    events: &mut SupervisorEvents,
) -> UmbrellaResult<()> {
    for actor_id in ["weather-fetcher", "weather-classifier", "file-writer"] {
        let needs_restart = system
            .actors
            .get(actor_id)
            .map(|actor| actor.status == ActorStatus::Stopped)
            .unwrap_or(true);
        if needs_restart {
            system.actors.remove(actor_id);
            create_weather_child(system, actor_id, pipeline)?;
            events.restarted_children.push(actor_id.to_string());
        }
    }
    Ok(())
}

fn create_weather_child(
    system: &mut ActorSystem,
    actor_id: &str,
    pipeline: &Arc<UmbrellaPipeline>,
) -> UmbrellaResult<()> {
    let behavior = match actor_id {
        "weather-fetcher" => fetcher_behavior(),
        "weather-classifier" => classifier_behavior(),
        "file-writer" => writer_behavior(),
        other => {
            return Err(UmbrellaAgentError::new(format!(
                "unknown weather child '{other}'"
            )))
        }
    };
    system.create_actor(actor_id, Box::new(pipeline.clone()), behavior)?;
    Ok(())
}

fn unwrap_pipeline_state(state: Box<dyn std::any::Any>) -> Arc<UmbrellaPipeline> {
    *state
        .downcast::<Arc<UmbrellaPipeline>>()
        .expect("umbrella actor state should be the shared pipeline")
}

fn register_weather_fetch_tool(
    runtime: &mut InMemoryToolRuntime,
    source: WeatherSource,
    fetched_at_ms: u64,
    fetched_at_iso: String,
    http_domain_policy: DeclaredHttpDomainPolicy,
) -> Result<(), ToolApiError> {
    runtime.register_handler(
        tool_definition(
            FETCH_TOOL_ID,
            "Fetch current weather",
            "Fetch the current Seattle weather snapshot through the host weather boundary.",
            JsonSchema::Object {
                properties: vec![SchemaProperty::new("location", JsonSchema::String)],
                required: vec!["location".to_string()],
                allow_unknown_fields: false,
            },
            Some(snapshot_schema()),
            ToolSideEffects::External,
            ToolIdempotency::Conditional,
            ToolConcurrency::Serialized,
            ToolStreaming::Events,
            vec!["weather_api_read"],
            vec!["weather", "fetch", "e2e"],
        ),
        move |arguments, _context| {
            let location = field_string(&arguments, "location")?;
            let payload = fetch_weather_from_source(
                &source,
                location,
                fetched_at_ms,
                &fetched_at_iso,
                &http_domain_policy,
            )
            .map_err(|error| {
                ToolCallError::new(
                    ToolErrorKind::ToolExecutionError,
                    format!("failed to fetch weather snapshot: {error}"),
                )
            })?;
            let snapshot = payload.snapshot;
            let source_name = match payload.summary.source_kind {
                WeatherFetchSourceKind::Fixture => "fixture",
                WeatherFetchSourceKind::LiveHttps => "live_https",
            };
            Ok(ToolHandlerOutput::new(snapshot.to_json()).with_event(
                ToolEventKind::Progress,
                object(vec![
                    ("source", string(source_name)),
                    ("endpoint_url", string(&snapshot.endpoint_url)),
                    ("https_requests", int(payload.summary.https_requests as i64)),
                    ("tls_handshakes", int(payload.summary.tls_handshakes as i64)),
                    (
                        "http_domain_policy_enforced",
                        JsonValue::Bool(payload.summary.http_domain_policy_enforced),
                    ),
                ]),
            ))
        },
    )
}

fn fetch_weather_from_source(
    source: &WeatherSource,
    location: String,
    fetched_at_ms: u64,
    fetched_at_iso: &str,
    http_domain_policy: &DeclaredHttpDomainPolicy,
) -> UmbrellaResult<WeatherFetchPayload> {
    match source {
        WeatherSource::Fixture(snapshot) if snapshot.location == location => {
            Ok(WeatherFetchPayload {
                snapshot: snapshot.clone(),
                summary: WeatherFetchSummary::fixture(snapshot),
            })
        }
        WeatherSource::Fixture(snapshot) => {
            let snapshot = WeatherSnapshot {
                location,
                ..snapshot.clone()
            };
            Ok(WeatherFetchPayload {
                summary: WeatherFetchSummary::fixture(&snapshot),
                snapshot,
            })
        }
        WeatherSource::LiveNws { user_agent } => fetch_live_nws_weather(
            location,
            fetched_at_ms,
            fetched_at_iso,
            user_agent,
            http_domain_policy,
        ),
    }
}

fn fetch_live_nws_weather(
    location: String,
    fetched_at_ms: u64,
    fetched_at_iso: &str,
    user_agent: &str,
    http_domain_policy: &DeclaredHttpDomainPolicy,
) -> UmbrellaResult<WeatherFetchPayload> {
    let points_url = format!("https://{WEATHER_API_DOMAIN}/points/47.6062,-122.3321");
    let points = https_get_declared_weather_url(&points_url, user_agent, http_domain_policy)?;
    if points.status != 200 {
        return Err(UmbrellaAgentError::new(format!(
            "Weather.gov points request returned HTTP {}",
            points.status
        )));
    }

    let points_json = coding_adventures_json_value::parse(&points.body)
        .map_err(|error| UmbrellaAgentError::new(format!("points JSON parse failed: {error}")))?;
    let forecast_url = extract_forecast_url(&points_json)?;
    let forecast = https_get_declared_weather_url(&forecast_url, user_agent, http_domain_policy)?;
    if forecast.status != 200 {
        return Err(UmbrellaAgentError::new(format!(
            "Weather.gov forecast request returned HTTP {}",
            forecast.status
        )));
    }

    let forecast_json = coding_adventures_json_value::parse(&forecast.body)
        .map_err(|error| UmbrellaAgentError::new(format!("forecast JSON parse failed: {error}")))?;
    let snapshot = snapshot_from_nws_forecast(
        location,
        forecast_url,
        forecast.status,
        fetched_at_ms,
        fetched_at_iso,
        &forecast.body,
        &forecast_json,
    )?;
    let summary = WeatherFetchSummary::live(points.status, &snapshot, http_domain_policy);
    Ok(WeatherFetchPayload { snapshot, summary })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpFetchResponse {
    status: u16,
    body: String,
}

fn https_get_declared_weather_url(
    url: &str,
    user_agent: &str,
    http_domain_policy: &DeclaredHttpDomainPolicy,
) -> UmbrellaResult<HttpFetchResponse> {
    let request_target = http_domain_policy.enforce_https_url(url)?;
    let mut tls_config = TlsConfig::https_default();
    tls_config.connect_timeout = Duration::from_secs(10);
    tls_config.handshake_timeout = Duration::from_secs(10);
    tls_config.read_timeout = Some(Duration::from_secs(15));
    tls_config.write_timeout = Some(Duration::from_secs(10));

    let connector = tls_platform::default_connector();
    let mut stream = connector
        .connect(&request_target.host, request_target.port, &tls_config)
        .map_err(|error| UmbrellaAgentError::new(format!("TLS connect failed: {error}")))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: {user_agent}\r\nAccept: application/geo+json, application/json\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n",
        request_target.path_and_query,
        request_target.host
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response_bytes = Vec::new();
    stream.read_to_end(&mut response_bytes)?;
    let parsed = http1::parse_response_head(&response_bytes)
        .map_err(|error| UmbrellaAgentError::new(format!("HTTP/1 parse failed: {error}")))?;
    let body = decode_http_body(&response_bytes, parsed.body_offset, &parsed.body_kind)?;
    let body = String::from_utf8(body)
        .map_err(|error| UmbrellaAgentError::new(format!("HTTP body was not UTF-8: {error}")))?;
    Ok(HttpFetchResponse {
        status: parsed.head.status,
        body,
    })
}

fn decode_http_body(
    response: &[u8],
    body_offset: usize,
    body_kind: &BodyKind,
) -> UmbrellaResult<Vec<u8>> {
    let body = response
        .get(body_offset..)
        .ok_or_else(|| UmbrellaAgentError::new("HTTP body offset exceeded response length"))?;
    match body_kind {
        BodyKind::None => Ok(Vec::new()),
        BodyKind::ContentLength(length) => body
            .get(..*length)
            .map(|slice| slice.to_vec())
            .ok_or_else(|| UmbrellaAgentError::new("HTTP body shorter than Content-Length")),
        BodyKind::UntilEof => Ok(body.to_vec()),
        BodyKind::Chunked => decode_chunked_body(body),
    }
}

fn decode_chunked_body(mut body: &[u8]) -> UmbrellaResult<Vec<u8>> {
    let mut decoded = Vec::new();
    loop {
        let line_end = body
            .windows(2)
            .position(|pair| pair == b"\r\n")
            .ok_or_else(|| UmbrellaAgentError::new("chunked body missing chunk-size line"))?;
        let size_line = std::str::from_utf8(&body[..line_end]).map_err(|error| {
            UmbrellaAgentError::new(format!("invalid chunk size UTF-8: {error}"))
        })?;
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|error| UmbrellaAgentError::new(format!("invalid chunk size: {error}")))?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(decoded);
        }
        if body.len() < size + 2 {
            return Err(UmbrellaAgentError::new("chunked body ended mid-chunk"));
        }
        decoded.extend_from_slice(&body[..size]);
        if &body[size..size + 2] != b"\r\n" {
            return Err(UmbrellaAgentError::new("chunk missing trailing CRLF"));
        }
        body = &body[size + 2..];
    }
}

fn extract_forecast_url(points_json: &JsonValue) -> UmbrellaResult<String> {
    json_string(json_field(
        json_field(points_json, "properties")?,
        "forecast",
    )?)
    .map(str::to_string)
}

fn snapshot_from_nws_forecast(
    location: String,
    endpoint_url: String,
    http_status: u16,
    fetched_at_ms: u64,
    fetched_at_iso: &str,
    raw_body: &str,
    forecast_json: &JsonValue,
) -> UmbrellaResult<WeatherSnapshot> {
    let periods = json_array(json_field(
        json_field(forecast_json, "properties")?,
        "periods",
    )?)?;
    let period = periods
        .iter()
        .find(|period| {
            json_field(period, "name")
                .and_then(json_string)
                .map(|name| name.eq_ignore_ascii_case("today"))
                .unwrap_or(false)
        })
        .or_else(|| periods.first())
        .ok_or_else(|| UmbrellaAgentError::new("forecast JSON had no periods"))?;

    let high_temp_f = json_i64(json_field(period, "temperature")?)?.clamp(-100, 150);
    let precip_value = json_field(period, "probabilityOfPrecipitation")
        .ok()
        .and_then(|precip| json_field(precip, "value").ok())
        .and_then(json_optional_i64)
        .unwrap_or(0)
        .clamp(0, 100) as u8;

    Ok(WeatherSnapshot {
        location,
        endpoint_url,
        http_status,
        fetched_at_ms,
        fetched_at_iso: fetched_at_iso.to_string(),
        high_temp_f,
        precip_pct: precip_value,
        raw_body: raw_body.to_string(),
    })
}

fn register_weather_classifier_tool(runtime: &mut InMemoryToolRuntime) -> Result<(), ToolApiError> {
    runtime.register_handler(
        tool_definition(
            CLASSIFY_TOOL_ID,
            "Classify umbrella need",
            "Classify a weather snapshot into the umbrella recommendation schema.",
            JsonSchema::Object {
                properties: vec![
                    SchemaProperty::new("snapshot", snapshot_schema()),
                    SchemaProperty::new("tick_id", JsonSchema::String),
                ],
                required: vec!["snapshot".to_string(), "tick_id".to_string()],
                allow_unknown_fields: false,
            },
            Some(recommendation_schema()),
            ToolSideEffects::None,
            ToolIdempotency::Always,
            ToolConcurrency::Safe,
            ToolStreaming::Events,
            vec!["internal_reasoning"],
            vec!["weather", "classify", "e2e"],
        ),
        move |arguments, _context| {
            let snapshot_value = field(&arguments, "snapshot")?;
            let snapshot = WeatherSnapshot::from_json(snapshot_value)?;
            let tick_id = field_string(&arguments, "tick_id")?;
            let recommendation = UmbrellaRecommendation::from_snapshot(&snapshot, &tick_id);
            Ok(ToolHandlerOutput::new(recommendation.to_json()).with_event(
                ToolEventKind::Progress,
                object(vec![
                    ("kind", string(recommendation.kind.as_str())),
                    (
                        "needs_umbrella",
                        JsonValue::Bool(recommendation.kind.needs_umbrella()),
                    ),
                ]),
            ))
        },
    )
}

fn register_file_writer_tool(
    runtime: &mut InMemoryToolRuntime,
    manifest: Manifest,
) -> Result<(), ToolApiError> {
    runtime.register_handler(
        tool_definition(
            WRITE_TOOL_ID,
            "Write text file",
            "Write the umbrella recommendation to a text file through capability-caged fs access.",
            JsonSchema::Object {
                properties: vec![
                    SchemaProperty::new("output_path", JsonSchema::String),
                    SchemaProperty::new("line", JsonSchema::String),
                ],
                required: vec!["output_path".to_string(), "line".to_string()],
                allow_unknown_fields: false,
            },
            Some(JsonSchema::Object {
                properties: vec![
                    SchemaProperty::new("output_path", JsonSchema::String),
                    SchemaProperty::new("bytes_written", JsonSchema::Integer),
                ],
                required: vec!["output_path".to_string(), "bytes_written".to_string()],
                allow_unknown_fields: false,
            }),
            ToolSideEffects::Write,
            ToolIdempotency::Conditional,
            ToolConcurrency::Serialized,
            ToolStreaming::Events,
            vec!["filesystem_write"],
            vec!["file", "write", "e2e"],
        ),
        move |arguments, _context| {
            let output_path = field_string(&arguments, "output_path")?;
            let line = field_string(&arguments, "line")?;
            let mut bytes = line.into_bytes();
            bytes.push(b'\n');
            secure_file::write_file(&manifest, Path::new(&output_path), &bytes).map_err(|err| {
                ToolCallError::new(
                    ToolErrorKind::ToolExecutionError,
                    format!("failed to write umbrella report: {err}"),
                )
            })?;
            Ok(ToolHandlerOutput::new(object(vec![
                ("output_path", string(&output_path)),
                ("bytes_written", int(bytes.len() as i64)),
            ]))
            .with_artifact_ref("umbrella_today_report")
            .with_event(
                ToolEventKind::Artifact,
                object(vec![("path", string(&output_path))]),
            ))
        },
    )
}

fn tool_definition(
    tool_id: &str,
    display_name: &str,
    description: &str,
    input_schema: JsonSchema,
    output_schema: Option<JsonSchema>,
    side_effects: ToolSideEffects,
    idempotency: ToolIdempotency,
    concurrency: ToolConcurrency,
    streaming: ToolStreaming,
    required_capabilities: Vec<&str>,
    tags: Vec<&str>,
) -> ToolDefinition {
    ToolDefinition {
        tool_id: tool_id.to_string(),
        display_name: display_name.to_string(),
        description: description.to_string(),
        input_schema,
        output_schema,
        side_effects,
        idempotency,
        concurrency,
        streaming,
        required_tier: PrivilegeTier::Tier1,
        required_capabilities: required_capabilities
            .into_iter()
            .map(str::to_string)
            .collect(),
        preferred_lock_scope: None,
        timeout_seconds: Some(30),
        tags: tags.into_iter().map(str::to_string).collect(),
        stability: ToolStability::Experimental,
    }
}

fn snapshot_schema() -> JsonSchema {
    JsonSchema::Object {
        properties: vec![
            SchemaProperty::new("location", JsonSchema::String),
            SchemaProperty::new("endpoint_url", JsonSchema::String),
            SchemaProperty::new("http_status", JsonSchema::Integer),
            SchemaProperty::new("fetched_at_ms", JsonSchema::Integer),
            SchemaProperty::new("fetched_at_iso", JsonSchema::String),
            SchemaProperty::new("high_temp_f", JsonSchema::Integer),
            SchemaProperty::new("precip_pct", JsonSchema::Integer),
            SchemaProperty::new("raw_body", JsonSchema::String),
        ],
        required: vec![
            "location".to_string(),
            "endpoint_url".to_string(),
            "http_status".to_string(),
            "fetched_at_ms".to_string(),
            "fetched_at_iso".to_string(),
            "high_temp_f".to_string(),
            "precip_pct".to_string(),
            "raw_body".to_string(),
        ],
        allow_unknown_fields: false,
    }
}

fn recommendation_schema() -> JsonSchema {
    JsonSchema::Object {
        properties: vec![
            SchemaProperty::new(
                "kind",
                JsonSchema::Enum {
                    values: vec![
                        string("NoAction"),
                        string("JacketOnly"),
                        string("UmbrellaOnly"),
                        string("Both"),
                    ],
                },
            ),
            SchemaProperty::new("location", JsonSchema::String),
            SchemaProperty::new("high_temp_f", JsonSchema::Integer),
            SchemaProperty::new("precip_pct", JsonSchema::Integer),
            SchemaProperty::new("fetched_at_ms", JsonSchema::Integer),
            SchemaProperty::new("fetched_at_iso", JsonSchema::String),
            SchemaProperty::new("tick_id", JsonSchema::String),
            SchemaProperty::new("explanation", JsonSchema::String),
        ],
        required: vec![
            "kind".to_string(),
            "location".to_string(),
            "high_temp_f".to_string(),
            "precip_pct".to_string(),
            "fetched_at_ms".to_string(),
            "fetched_at_iso".to_string(),
            "tick_id".to_string(),
            "explanation".to_string(),
        ],
        allow_unknown_fields: false,
    }
}

fn plan_agent_sandbox(config: &UmbrellaAgentConfig) -> UmbrellaResult<UmbrellaSandboxPlanSummary> {
    let manifest_json = umbrella_agent_required_capabilities_json(&config.output_path);
    let plans = plan_all_supported(&manifest_json).map_err(|error| {
        UmbrellaAgentError::new(format!(
            "failed to lower Weather Agent sandbox plan: {error}"
        ))
    })?;
    let summaries = plans.into_iter().map(|plan| plan.summary()).collect();
    Ok(UmbrellaSandboxPlanSummary::from_summaries(
        summaries,
        OsFamily::current(),
    ))
}

fn probe_agent_kernel_sandbox(
    config: &UmbrellaAgentConfig,
) -> UmbrellaResult<UmbrellaKernelSandboxSummary> {
    let support = current_kernel_sandbox_support();
    let denied_path = kernel_denied_probe_path(&config.output_path);
    let denied_network_target = "localhost:<ephemeral-undeclared-port>".to_string();
    if !support.available {
        return Ok(UmbrellaKernelSandboxSummary {
            os: support.os,
            primitive: support.primitive,
            available: false,
            enforced: false,
            allowed_write_succeeded: false,
            denied_write_blocked: false,
            network_outbound_enforced: false,
            network_denied_outbound_blocked: false,
            weather_https_allowed: false,
            host_exact_kernel_enforced: false,
            denied_path,
            denied_network_target,
            status_code: None,
            stderr_contains_operation_not_permitted: false,
            reason: support.reason,
        });
    }

    if let Some(parent) = config.output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if denied_path.exists() {
        fs::remove_file(&denied_path)?;
    }

    let manifest_json = umbrella_agent_required_capabilities_json(&config.output_path);
    let plan = plan_for_current_os(&manifest_json).map_err(|error| {
        UmbrellaAgentError::new(format!(
            "failed to lower Weather Agent kernel sandbox plan: {error}"
        ))
    })?;
    let allowed_arg = canonical_kernel_probe_path(&config.output_path)?;
    let denied_arg = canonical_kernel_probe_path(&denied_path)?;
    let output = run_with_kernel_sandbox(
        &plan,
        "/bin/sh",
        [
            "-c",
            "printf 'kernel sandbox allowed' > \"$1\"; printf 'kernel sandbox denied' > \"$2\"",
            "sh",
            &allowed_arg,
            &denied_arg,
        ],
    )
    .map_err(|error| {
        UmbrellaAgentError::new(format!(
            "failed to launch Weather Agent kernel sandbox probe: {error}"
        ))
    })?;

    let allowed_write_succeeded = fs::read_to_string(&config.output_path)
        .map(|contents| contents == "kernel sandbox allowed")
        .unwrap_or(false);
    let denied_write_blocked = !denied_path.exists();
    let network_denied = probe_denied_kernel_network_target(&plan)?;
    let weather_https_allowed = probe_weather_https_kernel_target(config, &plan)?;
    let stderr_contains_operation_not_permitted = output.stderr.contains("Operation not permitted");
    let enforced = allowed_write_succeeded
        && denied_write_blocked
        && network_denied.blocked
        && stderr_contains_operation_not_permitted
        && !output.success();

    if !enforced {
        return Err(UmbrellaAgentError::new(format!(
            "Weather Agent kernel sandbox probe did not enforce the manifest: status={:?} stderr={}",
            output.status_code, output.stderr
        )));
    }

    Ok(UmbrellaKernelSandboxSummary {
        os: output.os,
        primitive: output.primitive,
        available: true,
        enforced,
        allowed_write_succeeded,
        denied_write_blocked,
        network_outbound_enforced: network_denied.blocked,
        network_denied_outbound_blocked: network_denied.blocked,
        weather_https_allowed,
        host_exact_kernel_enforced: false,
        denied_path,
        denied_network_target: network_denied.target,
        status_code: output.status_code,
        stderr_contains_operation_not_permitted,
        reason: "kernel denied undeclared filesystem writes and undeclared outbound TCP ports; macOS Seatbelt constrains Weather.gov to TCP 443 while TLS/application checks retain host identity".to_string(),
    })
}

struct NetworkProbeSummary {
    target: String,
    blocked: bool,
}

fn probe_denied_kernel_network_target(plan: &SandboxPlan) -> UmbrellaResult<NetworkProbeSummary> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port().to_string();
    let output =
        run_with_kernel_sandbox(plan, "/usr/bin/nc", ["-z", "-G", "1", "localhost", &port])
            .map_err(|error| {
                UmbrellaAgentError::new(format!(
                    "failed to launch Weather Agent kernel network deny probe: {error}"
                ))
            })?;

    Ok(NetworkProbeSummary {
        target: format!("localhost:{port}"),
        blocked: !output.success(),
    })
}

fn probe_weather_https_kernel_target(
    config: &UmbrellaAgentConfig,
    plan: &SandboxPlan,
) -> UmbrellaResult<bool> {
    if !matches!(config.weather_source, WeatherSource::LiveNws { .. }) {
        return Ok(false);
    }

    let output = run_with_kernel_sandbox(
        plan,
        "/usr/bin/curl",
        ["-I", "--max-time", "8", "https://api.weather.gov"],
    )
    .map_err(|error| {
        UmbrellaAgentError::new(format!(
            "failed to launch Weather Agent kernel Weather.gov allow probe: {error}"
        ))
    })?;
    Ok(output.success())
}

fn kernel_denied_probe_path(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "umbrella-today.txt".to_string());
    output_path.with_file_name(format!("{file_name}.undeclared"))
}

fn canonical_kernel_probe_path(path: &Path) -> UmbrellaResult<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if let Ok(canonical) = fs::canonicalize(&absolute) {
        return Ok(canonical.to_string_lossy().to_string());
    }

    let parent = absolute.parent().ok_or_else(|| {
        UmbrellaAgentError::new(format!(
            "kernel sandbox probe path has no parent: {}",
            absolute.display()
        ))
    })?;
    let file_name = absolute.file_name().ok_or_else(|| {
        UmbrellaAgentError::new(format!(
            "kernel sandbox probe path has no final component: {}",
            absolute.display()
        ))
    })?;
    Ok(fs::canonicalize(parent)?
        .join(file_name)
        .to_string_lossy()
        .to_string())
}

fn umbrella_agent_required_capabilities_json(output_path: &Path) -> String {
    let output_path = json_escape(&output_path.to_string_lossy());
    format!(
        r#"{{
  "version": 1,
  "package": "rust/weather-agent-e2e",
  "capabilities": [
    {{
      "category": "net",
      "action": "dns",
      "target": "api.weather.gov",
      "justification": "Resolve Weather.gov for the live umbrella-today weather fetch."
    }},
    {{
      "category": "net",
      "action": "connect",
      "target": "api.weather.gov:443",
      "justification": "Fetch Weather.gov points and forecast resources over TLS."
    }},
    {{
      "category": "fs",
      "action": "write",
      "target": "{output_path}",
      "justification": "Write the umbrella-today decision text file."
    }}
  ],
  "justification": "Weather Agent E2E fetches Seattle weather and writes the umbrella decision."
}}"#
    )
}

fn umbrella_agent_http_domain_policy(
    output_path: &Path,
) -> UmbrellaResult<DeclaredHttpDomainPolicy> {
    DeclaredHttpDomainPolicy::from_required_capabilities_json(
        &umbrella_agent_required_capabilities_json(output_path),
    )
}

fn json_escape(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn plan_agent_job(config: &UmbrellaAgentConfig) -> UmbrellaResult<(BackendKind, usize)> {
    let spec = JobSpec {
        job_id: JOB_ID.to_string(),
        name: "Umbrella today".to_string(),
        description: "Run the supervised umbrella-today agent once.".to_string(),
        action: JobAction::AgentRun {
            agent_id: AGENT_ID.to_string(),
            args: vec![
                "--location".to_string(),
                config.location.clone(),
                "--once".to_string(),
            ],
            input: None,
        },
        trigger: JobTrigger::Once {
            at: DateTimeParts {
                year: 2026,
                month: 5,
                day: 12,
                hour: 7,
                minute: 0,
                second: 0,
            },
        },
        concurrency_policy: ConcurrencyPolicy::Skip,
        retry_policy: RetryPolicy {
            max_attempts: 1,
            initial_backoff_seconds: 60,
            max_backoff_seconds: Some(300),
        },
        timeout_seconds: Some(60),
        env: vec![],
        working_directory: None,
        output_policy: OutputPolicy {
            stdout_path: Some(config.output_path.to_string_lossy().to_string()),
            stderr_path: None,
            append: true,
        },
        enabled: true,
    };
    let runtime = NativeJobRuntime::for_in_process();
    let plan = runtime.compile_install_plan(&spec)?;
    Ok((plan.backend, plan.files_to_write.len()))
}

fn run_executor_tick(config: &UmbrellaAgentConfig) -> UmbrellaResult<ExecutorFleetStatusSummary> {
    let pool = RustThreadPool::spawn(
        RustThreadPoolOptions {
            worker_count: 1,
            limits: ExecutorLimits {
                max_queue_depth: 4,
                max_payload_bytes: 4 * 1024,
                max_response_bytes: 4 * 1024,
            },
            default_job_timeout: Some(Duration::from_secs(5)),
        },
        |request: ProtocolJobRequest<String>| ProtocolJobResult::Ok {
            payload: format!("accepted:{}", request.payload),
        },
    );
    let request = ProtocolJobRequest::new(JOB_ID, "umbrella_tick".to_string()).with_metadata(
        JobMetadata::default()
            .with_created_at_ms(config.fetched_at_ms)
            .with_trace_id(config.tick_id.clone())
            .with_tag("agent", AGENT_ID),
    );
    pool.submit(request)?;
    let response = pool
        .recv_response_timeout(Duration::from_secs(2))?
        .ok_or_else(|| UmbrellaAgentError::new("umbrella job executor did not respond"))?;
    if !response.is_success() {
        return Err(UmbrellaAgentError::new("umbrella job executor failed"));
    }
    Ok(ExecutorFleetSummary::from_snapshots([&pool.snapshot()]).status_summary())
}

fn validate_host_boundaries(output_path: &Path) -> UmbrellaResult<HostRwsSummary> {
    let fetcher = vec![RwsCapability::new("net", "connect", "api.weather.gov:443")
        .with_flavor(CapabilityFlavor::Ingestion)
        .with_trust(CapabilityTrust::Untrusted)
        .with_justification("fetch current Seattle weather")];
    let classifier = vec![
        RwsCapability::new("channel", "read", "weather_snapshot")
            .with_flavor(CapabilityFlavor::Internal)
            .with_trust(CapabilityTrust::Trusted)
            .with_justification("read trust-laundered snapshot"),
        RwsCapability::new("channel", "write", "umbrella_recommendation")
            .with_flavor(CapabilityFlavor::Internal)
            .with_trust(CapabilityTrust::Trusted)
            .with_justification("emit schema-pinned recommendation"),
    ];
    let writer = vec![
        RwsCapability::new("channel", "read", "umbrella_recommendation")
            .with_flavor(CapabilityFlavor::Internal)
            .with_trust(CapabilityTrust::Trusted)
            .with_justification("read trusted recommendation"),
        RwsCapability::new("fs", "write", output_path.to_string_lossy())
            .with_flavor(CapabilityFlavor::Actuation)
            .with_trust(CapabilityTrust::Trusted)
            .with_justification("write umbrella report text file"),
    ];

    validate_manifest(&fetcher)?;
    validate_manifest(&classifier)?;
    validate_manifest(&writer)?;

    let mut combined = fetcher.clone();
    combined.extend(writer.clone());
    let combined_manifest_rejected = validate_manifest(&combined).is_err();

    Ok(HostRwsSummary {
        fetcher: summarize_manifest(&fetcher),
        classifier: summarize_manifest(&classifier),
        writer: summarize_manifest(&writer),
        combined_manifest_rejected,
    })
}

fn writer_manifest(output_path: &Path) -> UmbrellaResult<Manifest> {
    let capability = CageCapability::new(
        CageCategory::Fs,
        CageAction::Write,
        output_path.to_string_lossy(),
        "write umbrella report text file",
    )?
    .with_flavor(CapabilityFlavor::Actuation)
    .with_trust(CapabilityTrust::Trusted);
    Ok(Manifest::try_new(vec![capability])?)
}

fn supervisor_summary(
    system: &ActorSystem,
    actor_stats: &std::collections::HashMap<String, u64>,
    events: SupervisorEvents,
) -> UmbrellaSupervisorSummary {
    let child_count = system.actors.len();
    let stopped_children = system
        .actors
        .values()
        .filter(|actor| actor.status == ActorStatus::Stopped)
        .count();
    let messages_processed = actor_stats
        .get("messages_processed")
        .copied()
        .unwrap_or_default();
    UmbrellaSupervisorSummary {
        child_count,
        stopped_children,
        failed_children: child_count.saturating_sub(stopped_children),
        messages_processed,
        dead_letters: system.dead_letters.len(),
        restart_count: events.restarted_children.len(),
        killed_children: events.killed_children,
        restarted_children: events.restarted_children,
    }
}

fn object(entries: Vec<(&str, JsonValue)>) -> JsonValue {
    JsonValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn string(value: &str) -> JsonValue {
    JsonValue::String(value.to_string())
}

fn int(value: i64) -> JsonValue {
    JsonValue::Number(JsonNumber::Integer(value))
}

fn field<'a>(value: &'a JsonValue, key: &str) -> Result<&'a JsonValue, ToolCallError> {
    match value {
        JsonValue::Object(entries) => entries
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
            .ok_or_else(|| {
                ToolCallError::new(
                    ToolErrorKind::ToolValidationError,
                    format!("missing field '{key}'"),
                )
            }),
        _ => Err(ToolCallError::new(
            ToolErrorKind::ToolValidationError,
            "expected JSON object",
        )),
    }
}

fn field_string(value: &JsonValue, key: &str) -> Result<String, ToolCallError> {
    match field(value, key)? {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(ToolCallError::new(
            ToolErrorKind::ToolValidationError,
            format!("field '{key}' must be a string"),
        )),
    }
}

fn field_i64(value: &JsonValue, key: &str) -> Result<i64, ToolCallError> {
    match field(value, key)? {
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(*value),
        _ => Err(ToolCallError::new(
            ToolErrorKind::ToolValidationError,
            format!("field '{key}' must be an integer"),
        )),
    }
}

fn json_field<'a>(value: &'a JsonValue, key: &str) -> UmbrellaResult<&'a JsonValue> {
    match value {
        JsonValue::Object(entries) => entries
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
            .ok_or_else(|| UmbrellaAgentError::new(format!("missing JSON field '{key}'"))),
        _ => Err(UmbrellaAgentError::new("expected JSON object")),
    }
}

fn json_string(value: &JsonValue) -> UmbrellaResult<&str> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(UmbrellaAgentError::new("expected JSON string")),
    }
}

fn json_array(value: &JsonValue) -> UmbrellaResult<&[JsonValue]> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(UmbrellaAgentError::new("expected JSON array")),
    }
}

fn json_i64(value: &JsonValue) -> UmbrellaResult<i64> {
    match value {
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(*value),
        JsonValue::Number(JsonNumber::Float(value)) if value.is_finite() => Ok(*value as i64),
        _ => Err(UmbrellaAgentError::new("expected JSON number")),
    }
}

fn json_optional_i64(value: &JsonValue) -> Option<i64> {
    match value {
        JsonValue::Null => None,
        JsonValue::Number(JsonNumber::Integer(value)) => Some(*value),
        JsonValue::Number(JsonNumber::Float(value)) if value.is_finite() => Some(*value as i64),
        _ => None,
    }
}

fn tool_error_to_agent(error: ToolCallError) -> UmbrellaAgentError {
    UmbrellaAgentError::new(format!("{}: {}", error.kind, error.message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rws_boundaries_reject_unsplit_fetch_and_write_host() {
        let path = Path::new("/tmp/umbrella-rws-test.txt");
        let summary = validate_host_boundaries(path).unwrap();
        assert_eq!(summary.fetcher.untrusted_inputs, 1);
        assert_eq!(summary.writer.external_actuations, 1);
        assert!(summary.combined_manifest_rejected);
    }

    #[test]
    fn deterministic_recommendation_requires_umbrella_for_rainy_seattle_fixture() {
        let snapshot = WeatherSnapshot::rainy_seattle_fixture();
        let recommendation = UmbrellaRecommendation::from_snapshot(&snapshot, "tick");
        assert_eq!(recommendation.kind, RecommendationKind::Both);
        assert!(recommendation.kind.needs_umbrella());
        assert!(recommendation
            .log_line()
            .contains("Bring an umbrella today"));
    }

    #[test]
    fn http_domain_policy_allows_only_required_capability_domains() {
        let policy =
            umbrella_agent_http_domain_policy(Path::new("/tmp/umbrella-domain-policy.txt"))
                .unwrap();

        let request = policy
            .enforce_https_url("https://api.weather.gov/gridpoints/SEW/124,67/forecast")
            .unwrap();
        assert_eq!(request.host, "api.weather.gov");
        assert_eq!(request.authority, "api.weather.gov:443");
        assert_eq!(request.path_and_query, "/gridpoints/SEW/124,67/forecast");
        assert_eq!(policy.declared_domains(), &["api.weather.gov".to_string()]);

        let error = policy
            .enforce_https_url("https://example.com/gridpoints/SEW/124,67/forecast")
            .unwrap_err();
        assert!(error.to_string().contains("net:dns:example.com"));
        assert!(error.to_string().contains("required_capabilities.json"));
    }

    #[test]
    fn http_domain_policy_rejects_wildcard_required_capabilities() {
        let json = r#"{
          "version": 1,
          "package": "rust/weather-agent-e2e",
          "capabilities": [
            {
              "category": "net",
              "action": "dns",
              "target": "*",
              "justification": "too broad"
            },
            {
              "category": "net",
              "action": "connect",
              "target": "*:443",
              "justification": "too broad"
            }
          ]
        }"#;

        let error = DeclaredHttpDomainPolicy::from_required_capabilities_json(json).unwrap_err();
        assert!(error.to_string().contains("exact domains"));
    }
}
