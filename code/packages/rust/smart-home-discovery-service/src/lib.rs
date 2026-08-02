//! Actor-owned durable lifecycle for scheduled smart-home mDNS discovery.

#![forbid(unsafe_code)]

use std::any::Any;
use std::fmt;

use actor::{ActorError, ActorResult, ActorSystem, Message};
use coding_adventures_json_serializer::serialize as serialize_json;
use coding_adventures_json_value::{parse as parse_json, JsonNumber, JsonValue};
use smart_home_core::{IntegrationId, Metadata};
use smart_home_discovery::{
    DiscoverySource, DiscoveryWorkerId, DiscoveryWorkerKind, DiscoveryWorkerRunStatus,
    MdnsWorkerScanExecutor,
};
use smart_home_runtime::{
    DiscoverySupervisorRunReport, DiscoveryWorkerQuery, MdnsDiscoveryRunAdapter, RuntimeError,
    ScheduledDiscoveryWorker, SmartHomeRuntime, WorkerStatus,
};
use storage_core::{
    StorageBackend, StorageError, StorageListOptions, StoragePutInput, StorageRecord,
};

pub const SCHEDULE_NAMESPACE: &str = "smart_home.discovery_service.schedules";
pub const RUN_NAMESPACE: &str = "smart_home.discovery_service.runs";
pub const STATE_NAMESPACE: &str = "smart_home.discovery_service.state";
pub const STATE_KEY: &str = "service";
pub const TICK_CONTENT_TYPE: &str = "application/vnd.smart-home.discovery-tick+json";

const JSON_CONTENT_TYPE: &str = "application/json";
const SCHEMA_VERSION: u64 = 1;

#[derive(Debug)]
pub enum DiscoveryServiceError {
    Storage(StorageError),
    Runtime(RuntimeError),
    InvalidData(String),
}

impl fmt::Display for DiscoveryServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "discovery service storage failed: {error}"),
            Self::Runtime(error) => write!(f, "discovery service runtime failed: {error}"),
            Self::InvalidData(message) => write!(f, "invalid discovery service data: {message}"),
        }
    }
}

impl std::error::Error for DiscoveryServiceError {}

impl From<StorageError> for DiscoveryServiceError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<RuntimeError> for DiscoveryServiceError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryServiceTick {
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
}

impl DiscoveryServiceTick {
    pub fn new(started_at_ms: u64, completed_at_ms: u64) -> Result<Self, DiscoveryServiceError> {
        if completed_at_ms < started_at_ms {
            return Err(invalid_data(
                "completed_at_ms must be greater than or equal to started_at_ms",
            ));
        }
        Ok(Self {
            started_at_ms,
            completed_at_ms,
        })
    }

    pub fn into_message(self, sender_id: &str) -> Result<Message, DiscoveryServiceError> {
        let body = encode_json(&JsonValue::Object(vec![
            ("schema_version".to_string(), json_u64(SCHEMA_VERSION)),
            ("started_at_ms".to_string(), json_u64(self.started_at_ms)),
            (
                "completed_at_ms".to_string(),
                json_u64(self.completed_at_ms),
            ),
        ]))?;
        Ok(Message::new(sender_id, TICK_CONTENT_TYPE, body, None))
    }

    fn from_message(message: &Message) -> Result<Self, DiscoveryServiceError> {
        if message.content_type != TICK_CONTENT_TYPE {
            return Err(invalid_data(format!(
                "tick message content type must be `{TICK_CONTENT_TYPE}`"
            )));
        }
        let value = decode_json(&message.payload)?;
        let object = expect_object("tick", &value)?;
        require_schema_version(object)?;
        Self::new(
            required_u64(object, "started_at_ms")?,
            required_u64(object, "completed_at_ms")?,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoveryServiceSnapshot {
    pub tick_count: u64,
    pub last_tick_started_at_ms: Option<u64>,
    pub last_tick_completed_at_ms: Option<u64>,
    pub last_planned_instruction_count: usize,
    pub last_mdns_request_count: usize,
    pub last_recorded_run_count: usize,
    pub last_failed_run_count: usize,
    pub last_error: Option<String>,
}

pub struct DiscoveryServiceActorState<B, E, A> {
    runtime: SmartHomeRuntime,
    backend: B,
    executor: E,
    adapter: A,
    ttl_ms: u64,
    snapshot: DiscoveryServiceSnapshot,
    last_report: Option<DiscoverySupervisorRunReport>,
}

impl<B, E, A> DiscoveryServiceActorState<B, E, A>
where
    B: StorageBackend,
    E: MdnsWorkerScanExecutor,
    A: MdnsDiscoveryRunAdapter,
{
    pub fn open(
        backend: B,
        executor: E,
        adapter: A,
        ttl_ms: u64,
    ) -> Result<Self, DiscoveryServiceError> {
        if ttl_ms == 0 {
            return Err(invalid_data("ttl_ms must be greater than zero"));
        }
        backend.initialize()?;
        let snapshot = load_service_snapshot(&backend)?.unwrap_or_default();
        let mut state = Self {
            runtime: SmartHomeRuntime::new(),
            backend,
            executor,
            adapter,
            ttl_ms,
            snapshot,
            last_report: None,
        };
        state.restore_schedules()?;
        Ok(state)
    }

    pub fn runtime(&self) -> &SmartHomeRuntime {
        &self.runtime
    }

    pub fn snapshot(&self) -> &DiscoveryServiceSnapshot {
        &self.snapshot
    }

    pub fn last_report(&self) -> Option<&DiscoverySupervisorRunReport> {
        self.last_report.as_ref()
    }

    pub fn register_worker(
        &mut self,
        worker: ScheduledDiscoveryWorker,
    ) -> Result<Option<ScheduledDiscoveryWorker>, DiscoveryServiceError> {
        if worker.kind != DiscoveryWorkerKind::MdnsScan
            || !worker.sources.contains(&DiscoverySource::Mdns)
        {
            return Err(invalid_data(
                "discovery service workers must be scheduled mDNS scans",
            ));
        }
        let previous = self.runtime.register_discovery_worker_schedule(worker)?;
        self.persist_schedules()?;
        Ok(previous)
    }

    pub fn tick(
        &mut self,
        tick: DiscoveryServiceTick,
    ) -> Result<&DiscoverySupervisorRunReport, DiscoveryServiceError> {
        self.snapshot.tick_count = self.snapshot.tick_count.saturating_add(1);
        self.snapshot.last_tick_started_at_ms = Some(tick.started_at_ms);
        self.snapshot.last_tick_completed_at_ms = Some(tick.completed_at_ms);

        let result = self.runtime.run_due_mdns_discovery_workers_with_executor(
            tick.started_at_ms,
            tick.completed_at_ms,
            self.ttl_ms,
            &mut self.executor,
            &mut self.adapter,
        );

        match result {
            Ok(report) => {
                self.snapshot.last_planned_instruction_count = report.planned_instruction_count;
                self.snapshot.last_mdns_request_count = report.mdns_request_count;
                self.snapshot.last_recorded_run_count = report.recorded_run_count();
                self.snapshot.last_failed_run_count = report.failed_run_count();
                self.snapshot.last_error = None;
                self.persist_schedules()?;
                self.persist_run_report(&report)?;
                self.persist_service_snapshot()?;
                self.last_report = Some(report);
                Ok(self
                    .last_report
                    .as_ref()
                    .expect("report was assigned before returning"))
            }
            Err(error) => {
                self.snapshot.last_error = Some(error.to_string());
                self.persist_schedules()?;
                self.persist_service_snapshot()?;
                Err(error.into())
            }
        }
    }

    pub fn persisted_run_records(&self) -> Result<Vec<StorageRecord>, DiscoveryServiceError> {
        Ok(self
            .backend
            .list(
                RUN_NAMESPACE,
                StorageListOptions {
                    recursive: true,
                    ..StorageListOptions::default()
                },
            )?
            .records)
    }

    fn restore_schedules(&mut self) -> Result<(), DiscoveryServiceError> {
        let page = self.backend.list(
            SCHEDULE_NAMESPACE,
            StorageListOptions {
                recursive: true,
                ..StorageListOptions::default()
            },
        )?;
        for record in page.records {
            let worker = decode_worker(&record.body)?;
            self.runtime.register_discovery_worker_schedule(worker)?;
        }
        Ok(())
    }

    fn persist_schedules(&self) -> Result<(), DiscoveryServiceError> {
        let workers = self
            .runtime
            .query_discovery_worker_schedules(&DiscoveryWorkerQuery::new());
        for worker in workers {
            self.backend.put(StoragePutInput::new(
                SCHEDULE_NAMESPACE,
                worker.worker_id.as_str(),
                JSON_CONTENT_TYPE,
                schema_metadata(),
                encode_json(&encode_worker(worker))?,
            )?)?;
        }
        Ok(())
    }

    fn persist_run_report(
        &self,
        report: &DiscoverySupervisorRunReport,
    ) -> Result<(), DiscoveryServiceError> {
        let key = format!(
            "{:020}-{:020}",
            report.completed_at_ms, self.snapshot.tick_count
        );
        self.backend.put(StoragePutInput::new(
            RUN_NAMESPACE,
            key,
            JSON_CONTENT_TYPE,
            schema_metadata(),
            encode_json(&encode_run_report(report))?,
        )?)?;
        Ok(())
    }

    fn persist_service_snapshot(&self) -> Result<(), DiscoveryServiceError> {
        persist_service_snapshot(&self.backend, &self.snapshot)
    }

    fn record_message_error(&mut self, error: &DiscoveryServiceError) {
        self.snapshot.last_error = Some(error.to_string());
        let _ = self.persist_service_snapshot();
    }
}

pub fn install_discovery_service_actor<B, E, A>(
    system: &mut ActorSystem,
    actor_id: &str,
    state: DiscoveryServiceActorState<B, E, A>,
) -> Result<String, ActorError>
where
    B: StorageBackend + 'static,
    E: MdnsWorkerScanExecutor + 'static,
    A: MdnsDiscoveryRunAdapter + 'static,
{
    system.create_actor(
        actor_id,
        Box::new(state),
        Box::new(|state: Box<dyn Any>, message| {
            let mut state = *state
                .downcast::<DiscoveryServiceActorState<B, E, A>>()
                .expect("discovery service actor received the wrong state type");
            match DiscoveryServiceTick::from_message(message) {
                Ok(tick) => {
                    let _ = state.tick(tick);
                }
                Err(error) => state.record_message_error(&error),
            }
            ActorResult {
                new_state: Box::new(state),
                messages_to_send: Vec::new(),
                actors_to_create: Vec::new(),
                stop: false,
            }
        }),
    )
}

fn encode_worker(worker: &ScheduledDiscoveryWorker) -> JsonValue {
    JsonValue::Object(vec![
        ("schema_version".to_string(), json_u64(SCHEMA_VERSION)),
        (
            "worker_id".to_string(),
            JsonValue::String(worker.worker_id.as_str().to_string()),
        ),
        (
            "integration_id".to_string(),
            JsonValue::String(worker.integration_id.as_str().to_string()),
        ),
        (
            "kind".to_string(),
            JsonValue::String(worker.kind.as_str().to_string()),
        ),
        (
            "sources".to_string(),
            string_array(worker.sources.iter().map(|source| source.as_str())),
        ),
        (
            "network_interfaces".to_string(),
            string_array(worker.network_interfaces.iter().map(String::as_str)),
        ),
        (
            "status".to_string(),
            JsonValue::String(worker.status.as_str().to_string()),
        ),
        ("interval_ms".to_string(), json_u64(worker.interval_ms)),
        (
            "run_timeout_ms".to_string(),
            json_u64(worker.run_timeout_ms),
        ),
        (
            "retry_delay_ms".to_string(),
            json_u64(worker.retry_delay_ms),
        ),
        (
            "max_retry_delay_ms".to_string(),
            json_u64(worker.max_retry_delay_ms),
        ),
        (
            "retry_backoff_multiplier".to_string(),
            json_u64(u64::from(worker.retry_backoff_multiplier)),
        ),
        (
            "next_due_at_ms".to_string(),
            json_u64(worker.next_due_at_ms),
        ),
        (
            "last_started_at_ms".to_string(),
            json_optional_u64(worker.last_started_at_ms),
        ),
        (
            "last_completed_at_ms".to_string(),
            json_optional_u64(worker.last_completed_at_ms),
        ),
        (
            "last_run_status".to_string(),
            worker
                .last_run_status
                .map(|status| JsonValue::String(status.as_str().to_string()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "last_record_count".to_string(),
            json_usize(worker.last_record_count),
        ),
        (
            "last_failure_count".to_string(),
            json_usize(worker.last_failure_count),
        ),
        (
            "last_catalog_change_count".to_string(),
            json_usize(worker.last_catalog_change_count),
        ),
        (
            "total_run_count".to_string(),
            json_u64(worker.total_run_count),
        ),
        (
            "consecutive_failure_count".to_string(),
            json_u64(u64::from(worker.consecutive_failure_count)),
        ),
        ("metadata".to_string(), encode_metadata(&worker.metadata)),
    ])
}

fn decode_worker(bytes: &[u8]) -> Result<ScheduledDiscoveryWorker, DiscoveryServiceError> {
    let value = decode_json(bytes)?;
    let object = expect_object("worker", &value)?;
    require_schema_version(object)?;
    let kind = parse_worker_kind(&required_string(object, "kind")?)?;
    let sources = required_string_array(object, "sources")?
        .into_iter()
        .map(|source| parse_discovery_source(&source))
        .collect::<Result<Vec<_>, _>>()?;
    let mut worker = ScheduledDiscoveryWorker::new(
        DiscoveryWorkerId::trusted(required_string(object, "worker_id")?),
        IntegrationId::trusted(required_string(object, "integration_id")?),
        kind,
        required_u64(object, "interval_ms")?,
        required_u64(object, "run_timeout_ms")?,
        required_u64(object, "next_due_at_ms")?,
    );
    worker.sources = sources;
    worker.network_interfaces = required_string_array(object, "network_interfaces")?;
    worker.status = parse_worker_status(&required_string(object, "status")?)?;
    worker.retry_delay_ms = required_u64(object, "retry_delay_ms")?;
    worker.max_retry_delay_ms = required_u64(object, "max_retry_delay_ms")?;
    worker.retry_backoff_multiplier = required_u32(object, "retry_backoff_multiplier")?;
    worker.last_started_at_ms = optional_u64(object, "last_started_at_ms")?;
    worker.last_completed_at_ms = optional_u64(object, "last_completed_at_ms")?;
    worker.last_run_status = optional_run_status(object, "last_run_status")?;
    worker.last_record_count = required_usize(object, "last_record_count")?;
    worker.last_failure_count = required_usize(object, "last_failure_count")?;
    worker.last_catalog_change_count = required_usize(object, "last_catalog_change_count")?;
    worker.total_run_count = required_u64(object, "total_run_count")?;
    worker.consecutive_failure_count = required_u32(object, "consecutive_failure_count")?;
    worker.metadata = decode_metadata(required_value(object, "metadata")?)?;
    worker
        .validate()
        .map_err(|error| invalid_data(error.to_string()))?;
    Ok(worker)
}

fn encode_run_report(report: &DiscoverySupervisorRunReport) -> JsonValue {
    JsonValue::Object(vec![
        ("schema_version".to_string(), json_u64(SCHEMA_VERSION)),
        ("started_at_ms".to_string(), json_u64(report.started_at_ms)),
        (
            "completed_at_ms".to_string(),
            json_u64(report.completed_at_ms),
        ),
        ("ttl_ms".to_string(), json_u64(report.ttl_ms)),
        (
            "planned_instruction_count".to_string(),
            json_usize(report.planned_instruction_count),
        ),
        (
            "mdns_request_count".to_string(),
            json_usize(report.mdns_request_count),
        ),
        (
            "mdns_report_count".to_string(),
            json_usize(report.mdns_report_count),
        ),
        (
            "summaries".to_string(),
            JsonValue::Array(
                report
                    .summaries
                    .iter()
                    .map(|summary| {
                        JsonValue::Object(vec![
                            (
                                "worker_id".to_string(),
                                JsonValue::String(summary.worker_id.as_str().to_string()),
                            ),
                            (
                                "integration_id".to_string(),
                                JsonValue::String(summary.integration_id.as_str().to_string()),
                            ),
                            (
                                "status".to_string(),
                                JsonValue::String(summary.status.as_str().to_string()),
                            ),
                            ("record_count".to_string(), json_usize(summary.record_count)),
                            (
                                "failure_count".to_string(),
                                json_usize(summary.failure_count),
                            ),
                            (
                                "inserted_count".to_string(),
                                json_usize(summary.inserted_count),
                            ),
                            (
                                "replaced_count".to_string(),
                                json_usize(summary.replaced_count),
                            ),
                            (
                                "ignored_count".to_string(),
                                json_usize(summary.ignored_count),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "failures".to_string(),
            JsonValue::Array(
                report
                    .failures
                    .iter()
                    .map(|failure| {
                        JsonValue::Object(vec![
                            (
                                "worker_id".to_string(),
                                JsonValue::String(failure.worker_id.as_str().to_string()),
                            ),
                            (
                                "integration_id".to_string(),
                                JsonValue::String(failure.integration_id.as_str().to_string()),
                            ),
                            (
                                "message".to_string(),
                                JsonValue::String(failure.message.clone()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn persist_service_snapshot<B: StorageBackend>(
    backend: &B,
    snapshot: &DiscoveryServiceSnapshot,
) -> Result<(), DiscoveryServiceError> {
    backend.put(StoragePutInput::new(
        STATE_NAMESPACE,
        STATE_KEY,
        JSON_CONTENT_TYPE,
        schema_metadata(),
        encode_json(&encode_service_snapshot(snapshot))?,
    )?)?;
    Ok(())
}

fn load_service_snapshot<B: StorageBackend>(
    backend: &B,
) -> Result<Option<DiscoveryServiceSnapshot>, DiscoveryServiceError> {
    backend
        .get(STATE_NAMESPACE, STATE_KEY)?
        .map(|record| decode_service_snapshot(&record.body))
        .transpose()
}

fn encode_service_snapshot(snapshot: &DiscoveryServiceSnapshot) -> JsonValue {
    JsonValue::Object(vec![
        ("schema_version".to_string(), json_u64(SCHEMA_VERSION)),
        ("tick_count".to_string(), json_u64(snapshot.tick_count)),
        (
            "last_tick_started_at_ms".to_string(),
            json_optional_u64(snapshot.last_tick_started_at_ms),
        ),
        (
            "last_tick_completed_at_ms".to_string(),
            json_optional_u64(snapshot.last_tick_completed_at_ms),
        ),
        (
            "last_planned_instruction_count".to_string(),
            json_usize(snapshot.last_planned_instruction_count),
        ),
        (
            "last_mdns_request_count".to_string(),
            json_usize(snapshot.last_mdns_request_count),
        ),
        (
            "last_recorded_run_count".to_string(),
            json_usize(snapshot.last_recorded_run_count),
        ),
        (
            "last_failed_run_count".to_string(),
            json_usize(snapshot.last_failed_run_count),
        ),
        (
            "last_error".to_string(),
            snapshot
                .last_error
                .as_ref()
                .map(|error| JsonValue::String(error.clone()))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn decode_service_snapshot(
    bytes: &[u8],
) -> Result<DiscoveryServiceSnapshot, DiscoveryServiceError> {
    let value = decode_json(bytes)?;
    let object = expect_object("service snapshot", &value)?;
    require_schema_version(object)?;
    Ok(DiscoveryServiceSnapshot {
        tick_count: required_u64(object, "tick_count")?,
        last_tick_started_at_ms: optional_u64(object, "last_tick_started_at_ms")?,
        last_tick_completed_at_ms: optional_u64(object, "last_tick_completed_at_ms")?,
        last_planned_instruction_count: required_usize(object, "last_planned_instruction_count")?,
        last_mdns_request_count: required_usize(object, "last_mdns_request_count")?,
        last_recorded_run_count: required_usize(object, "last_recorded_run_count")?,
        last_failed_run_count: required_usize(object, "last_failed_run_count")?,
        last_error: optional_string(object, "last_error")?,
    })
}

fn encode_metadata(metadata: &[Metadata]) -> JsonValue {
    JsonValue::Array(
        metadata
            .iter()
            .map(|entry| {
                JsonValue::Object(vec![
                    ("key".to_string(), JsonValue::String(entry.key.clone())),
                    ("value".to_string(), JsonValue::String(entry.value.clone())),
                ])
            })
            .collect(),
    )
}

fn decode_metadata(value: &JsonValue) -> Result<Vec<Metadata>, DiscoveryServiceError> {
    let JsonValue::Array(entries) = value else {
        return Err(invalid_data("metadata must be an array"));
    };
    entries
        .iter()
        .map(|entry| {
            let object = expect_object("metadata entry", entry)?;
            Ok(Metadata::new(
                required_string(object, "key")?,
                required_string(object, "value")?,
            ))
        })
        .collect()
}

fn parse_worker_kind(value: &str) -> Result<DiscoveryWorkerKind, DiscoveryServiceError> {
    match value {
        "mdns_scan" => Ok(DiscoveryWorkerKind::MdnsScan),
        _ => Err(invalid_data(format!(
            "unsupported discovery worker kind `{value}`"
        ))),
    }
}

fn parse_discovery_source(value: &str) -> Result<DiscoverySource, DiscoveryServiceError> {
    match value {
        "mdns" => Ok(DiscoverySource::Mdns),
        "ssdp" => Ok(DiscoverySource::Ssdp),
        "udp_multicast" => Ok(DiscoverySource::UdpMulticast),
        "udp_broadcast" => Ok(DiscoverySource::UdpBroadcast),
        "bluetooth" => Ok(DiscoverySource::Bluetooth),
        "usb" => Ok(DiscoverySource::Usb),
        "dhcp" => Ok(DiscoverySource::Dhcp),
        "mqtt" => Ok(DiscoverySource::Mqtt),
        "manual" => Ok(DiscoverySource::Manual),
        "cloud_fallback" => Ok(DiscoverySource::CloudFallback),
        "webhook" => Ok(DiscoverySource::Webhook),
        "simulator" => Ok(DiscoverySource::Simulator),
        _ => Err(invalid_data(format!(
            "unsupported discovery source `{value}`"
        ))),
    }
}

fn parse_worker_status(value: &str) -> Result<WorkerStatus, DiscoveryServiceError> {
    match value {
        "starting" => Ok(WorkerStatus::Starting),
        "running" => Ok(WorkerStatus::Running),
        "unhealthy" => Ok(WorkerStatus::Unhealthy),
        "restarting" => Ok(WorkerStatus::Restarting),
        "stopped" => Ok(WorkerStatus::Stopped),
        _ => Err(invalid_data(format!("unsupported worker status `{value}`"))),
    }
}

fn parse_run_status(value: &str) -> Result<DiscoveryWorkerRunStatus, DiscoveryServiceError> {
    match value {
        "completed" => Ok(DiscoveryWorkerRunStatus::Completed),
        "partial" => Ok(DiscoveryWorkerRunStatus::Partial),
        "failed" => Ok(DiscoveryWorkerRunStatus::Failed),
        _ => Err(invalid_data(format!("unsupported run status `{value}`"))),
    }
}

fn encode_json(value: &JsonValue) -> Result<Vec<u8>, DiscoveryServiceError> {
    serialize_json(value)
        .map(String::into_bytes)
        .map_err(|error| invalid_data(error.message))
}

fn decode_json(bytes: &[u8]) -> Result<JsonValue, DiscoveryServiceError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| invalid_data("stored JSON body must be UTF-8"))?;
    parse_json(text).map_err(|error| invalid_data(error.message))
}

fn expect_object<'a>(
    label: &str,
    value: &'a JsonValue,
) -> Result<&'a [(String, JsonValue)], DiscoveryServiceError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(invalid_data(format!("{label} must be a JSON object"))),
    }
}

fn required_value<'a>(
    object: &'a [(String, JsonValue)],
    field: &str,
) -> Result<&'a JsonValue, DiscoveryServiceError> {
    object
        .iter()
        .find(|(name, _)| name == field)
        .map(|(_, value)| value)
        .ok_or_else(|| invalid_data(format!("required field `{field}` was missing")))
}

fn required_string(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<String, DiscoveryServiceError> {
    match required_value(object, field)? {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(invalid_data(format!("field `{field}` must be a string"))),
    }
}

fn optional_string(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Option<String>, DiscoveryServiceError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(value) => Ok(Some(value.clone())),
        _ => Err(invalid_data(format!(
            "field `{field}` must be null or a string"
        ))),
    }
}

fn required_u64(object: &[(String, JsonValue)], field: &str) -> Result<u64, DiscoveryServiceError> {
    match required_value(object, field)? {
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => Ok(*value as u64),
        _ => Err(invalid_data(format!(
            "field `{field}` must be a non-negative integer"
        ))),
    }
}

fn optional_u64(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Option<u64>, DiscoveryServiceError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => Ok(Some(*value as u64)),
        _ => Err(invalid_data(format!(
            "field `{field}` must be null or a non-negative integer"
        ))),
    }
}

fn required_usize(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<usize, DiscoveryServiceError> {
    usize::try_from(required_u64(object, field)?)
        .map_err(|_| invalid_data(format!("field `{field}` does not fit usize")))
}

fn required_u32(object: &[(String, JsonValue)], field: &str) -> Result<u32, DiscoveryServiceError> {
    u32::try_from(required_u64(object, field)?)
        .map_err(|_| invalid_data(format!("field `{field}` does not fit u32")))
}

fn required_string_array(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Vec<String>, DiscoveryServiceError> {
    match required_value(object, field)? {
        JsonValue::Array(values) => values
            .iter()
            .map(|value| match value {
                JsonValue::String(value) => Ok(value.clone()),
                _ => Err(invalid_data(format!(
                    "field `{field}` array elements must be strings"
                ))),
            })
            .collect(),
        _ => Err(invalid_data(format!("field `{field}` must be an array"))),
    }
}

fn optional_run_status(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Option<DiscoveryWorkerRunStatus>, DiscoveryServiceError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(value) => parse_run_status(value).map(Some),
        _ => Err(invalid_data(format!(
            "field `{field}` must be null or a string"
        ))),
    }
}

fn require_schema_version(object: &[(String, JsonValue)]) -> Result<(), DiscoveryServiceError> {
    let version = required_u64(object, "schema_version")?;
    if version != SCHEMA_VERSION {
        return Err(invalid_data(format!(
            "unsupported schema version `{version}`"
        )));
    }
    Ok(())
}

fn schema_metadata() -> JsonValue {
    JsonValue::Object(vec![(
        "schema_version".to_string(),
        json_u64(SCHEMA_VERSION),
    )])
}

fn json_u64(value: u64) -> JsonValue {
    JsonValue::Number(JsonNumber::Integer(value as i64))
}

fn json_usize(value: usize) -> JsonValue {
    json_u64(value as u64)
}

fn json_optional_u64(value: Option<u64>) -> JsonValue {
    value.map(json_u64).unwrap_or(JsonValue::Null)
}

fn string_array<'a>(values: impl IntoIterator<Item = &'a str>) -> JsonValue {
    JsonValue::Array(
        values
            .into_iter()
            .map(|value| JsonValue::String(value.to_string()))
            .collect(),
    )
}

fn invalid_data(message: impl Into<String>) -> DiscoveryServiceError {
    DiscoveryServiceError::InvalidData(message.into())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use smart_home_discovery::{
        DiscoveryError, DiscoveryWorkerRun, MdnsResponsePacket, MdnsScanResult,
        MdnsWorkerScanReport, MdnsWorkerScanRequest,
    };
    use storage_local_folder::LocalFolderStorageBackend;

    use super::*;

    static TEST_DIRECTORY_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug, Default)]
    struct RecordingExecutor {
        requests: Vec<MdnsWorkerScanRequest>,
    }

    impl MdnsWorkerScanExecutor for RecordingExecutor {
        fn run_request(
            &mut self,
            request: &MdnsWorkerScanRequest,
        ) -> Result<MdnsScanResult, DiscoveryError> {
            self.requests.push(request.clone());
            MdnsScanResult::from_packets(
                request.service_type.clone(),
                request.discovered_at_ms,
                Vec::<MdnsResponsePacket>::new(),
            )
        }
    }

    #[derive(Debug)]
    struct ScriptedAdapter {
        failures: VecDeque<Option<String>>,
    }

    impl ScriptedAdapter {
        fn successful() -> Self {
            Self {
                failures: VecDeque::from([None]),
            }
        }

        fn failing(message: &str) -> Self {
            Self {
                failures: VecDeque::from([Some(message.to_string())]),
            }
        }
    }

    impl MdnsDiscoveryRunAdapter for ScriptedAdapter {
        type Error = String;

        fn worker_run_from_mdns_scan_report(
            &mut self,
            report: &MdnsWorkerScanReport,
        ) -> Result<DiscoveryWorkerRun, Self::Error> {
            if let Some(Some(message)) = self.failures.pop_front() {
                return Err(message);
            }
            Ok(DiscoveryWorkerRun::new(
                report.worker_id.clone(),
                report.integration_id.clone(),
                DiscoveryWorkerKind::MdnsScan,
                report.started_at_ms,
                report.completed_at_ms,
            ))
        }
    }

    fn worker(first_due_at_ms: u64) -> ScheduledDiscoveryWorker {
        ScheduledDiscoveryWorker::new(
            DiscoveryWorkerId::trusted("hue-mdns"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            5_000,
            250,
            first_due_at_ms,
        )
        .with_source(DiscoverySource::Mdns)
        .with_network_interface("en7")
        .with_retry_backoff(500, 4_000, 2)
        .with_metadata("smart_home.discovery.service_type", "_hue._tcp.local")
    }

    fn test_directory(label: &str) -> PathBuf {
        let suffix = TEST_DIRECTORY_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "smart-home-discovery-service-{}-{label}-{suffix}",
            std::process::id()
        ))
    }

    #[test]
    fn actor_tick_binds_selected_interface_and_restores_durable_cadence() {
        let root = test_directory("success");
        let backend = LocalFolderStorageBackend::new(&root);
        let mut state = DiscoveryServiceActorState::open(
            backend,
            RecordingExecutor::default(),
            ScriptedAdapter::successful(),
            30_000,
        )
        .unwrap();
        state.register_worker(worker(1_000)).unwrap();

        let mut system = ActorSystem::new();
        install_discovery_service_actor(&mut system, "mdns-service", state).unwrap();
        system
            .send(
                "mdns-service",
                DiscoveryServiceTick::new(1_100, 1_180)
                    .unwrap()
                    .into_message("clock")
                    .unwrap(),
            )
            .unwrap();
        assert!(system.process_next("mdns-service").unwrap());

        let actor = system.actors.get("mdns-service").unwrap();
        let state = actor
            .state
            .downcast_ref::<DiscoveryServiceActorState<
                LocalFolderStorageBackend,
                RecordingExecutor,
                ScriptedAdapter,
            >>()
            .unwrap();
        assert_eq!(state.executor.requests.len(), 2);
        assert!(state
            .executor
            .requests
            .iter()
            .all(|request| request.network_interface == "en7"));
        assert_eq!(state.snapshot().tick_count, 1);
        assert_eq!(state.snapshot().last_mdns_request_count, 2);
        assert_eq!(state.persisted_run_records().unwrap().len(), 1);

        drop(system);
        let restored = DiscoveryServiceActorState::open(
            LocalFolderStorageBackend::new(&root),
            RecordingExecutor::default(),
            ScriptedAdapter::successful(),
            30_000,
        )
        .unwrap();
        let schedule = restored
            .runtime()
            .discovery_worker_schedule(&DiscoveryWorkerId::trusted("hue-mdns"))
            .unwrap();
        assert_eq!(restored.snapshot().tick_count, 1);
        assert_eq!(schedule.total_run_count, 1);
        assert_eq!(schedule.status, WorkerStatus::Running);
        assert_eq!(
            schedule.last_run_status,
            Some(DiscoveryWorkerRunStatus::Completed)
        );
        assert_eq!(schedule.next_due_at_ms, 6_180);
        assert_eq!(restored.persisted_run_records().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_run_backoff_and_audit_survive_restart() {
        let root = test_directory("failure");
        let mut state = DiscoveryServiceActorState::open(
            LocalFolderStorageBackend::new(&root),
            RecordingExecutor::default(),
            ScriptedAdapter::failing("unsupported advertisement"),
            30_000,
        )
        .unwrap();
        state.register_worker(worker(2_000)).unwrap();
        let report = state
            .tick(DiscoveryServiceTick::new(2_100, 2_180).unwrap())
            .unwrap();
        assert_eq!(report.failed_run_count(), 1);

        let restored = DiscoveryServiceActorState::open(
            LocalFolderStorageBackend::new(&root),
            RecordingExecutor::default(),
            ScriptedAdapter::successful(),
            30_000,
        )
        .unwrap();
        let schedule = restored
            .runtime()
            .discovery_worker_schedule(&DiscoveryWorkerId::trusted("hue-mdns"))
            .unwrap();
        assert_eq!(restored.snapshot().last_failed_run_count, 1);
        assert_eq!(schedule.status, WorkerStatus::Unhealthy);
        assert_eq!(
            schedule.last_run_status,
            Some(DiscoveryWorkerRunStatus::Failed)
        );
        assert_eq!(schedule.consecutive_failure_count, 1);
        assert_eq!(schedule.next_due_at_ms, 2_680);
        assert_eq!(restored.persisted_run_records().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn persisted_udp_sources_round_trip() {
        assert_eq!(
            parse_discovery_source("udp_multicast").unwrap(),
            DiscoverySource::UdpMulticast
        );
        assert_eq!(
            parse_discovery_source("udp_broadcast").unwrap(),
            DiscoverySource::UdpBroadcast
        );
    }
}
