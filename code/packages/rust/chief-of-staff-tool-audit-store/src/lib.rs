//! # chief-of-staff-tool-audit-store
//!
//! Storage-backed persistence for D18D [`ToolAuditRecord`] rows.
//!
//! `chief-of-staff-tool-api` owns the runtime-facing audit vocabulary. This
//! crate keeps durable persistence separate by storing audit rows through the
//! D18A `StorageBackend` abstraction.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_tool_api::{
    query_tool_audit_records, ApprovalState, RequestedBy, ToolAuditRecord, ToolAuditRecordQuery,
    ToolAuditSink, ToolCallStatus, ToolErrorKind, ToolExecutionTrace, ToolInvocationRequest,
    ToolResultAuditSummary,
};
use coding_adventures_json_serializer::serialize as json_serialize;
use coding_adventures_json_value::{parse as json_parse, JsonNumber, JsonValue};
use storage_core::{
    StorageBackend, StorageError, StorageListOptions, StoragePutInput, StorageRecord,
    StorageRecordInventorySummary,
};

const AUDIT_NAMESPACE: &str = "chief.tool.audit";
const AUDIT_PREFIX: &str = "calls/";
const AUDIT_CONTENT_TYPE: &str = "application/vnd.coding-adventures.tool-audit+json";

/// Storage-backed D18D audit record store.
pub struct ToolAuditStore<S> {
    backend: S,
}

impl<S> ToolAuditStore<S>
where
    S: StorageBackend,
{
    /// Create a store over the supplied storage backend.
    pub fn new(backend: S) -> Self {
        Self { backend }
    }

    /// Borrow the underlying storage backend.
    pub fn backend(&self) -> &S {
        &self.backend
    }

    /// Record a payload-free audit row exactly once.
    pub fn record_audit(&self, record: ToolAuditRecord) -> Result<ToolAuditRecord, StorageError> {
        self.backend.initialize()?;
        let key = audit_key(&record.call_id);
        if let Some(existing) = self.backend.get_summary(AUDIT_NAMESPACE, &key)? {
            return Err(StorageError::Conflict {
                namespace: AUDIT_NAMESPACE.to_string(),
                key,
                expected_revision: None,
                actual_revision: Some(existing.revision.to_string()),
            });
        }

        let json = audit_record_to_json(&record);
        let body = json_to_body(&json)?;
        self.backend.put(StoragePutInput::new(
            AUDIT_NAMESPACE,
            key,
            AUDIT_CONTENT_TYPE,
            audit_metadata(&record),
            body,
        )?)?;
        Ok(record)
    }

    /// Derive and record the audit row for one canonical execution trace.
    pub fn record_trace(
        &self,
        request: &ToolInvocationRequest,
        trace: &ToolExecutionTrace,
    ) -> Result<ToolAuditRecord, StorageError> {
        self.record_audit(ToolAuditRecord::from_trace(request, trace))
    }

    /// Fetch one audit row by call id.
    pub fn fetch_audit(&self, call_id: &str) -> Result<Option<ToolAuditRecord>, StorageError> {
        self.backend.initialize()?;
        self.backend
            .get(AUDIT_NAMESPACE, &audit_key(call_id))?
            .map(|record| decode_audit_record(&record))
            .transpose()
    }

    /// List persisted audit records and apply the API-level audit query.
    pub fn query_audits(
        &self,
        query: &ToolAuditRecordQuery,
    ) -> Result<Vec<ToolAuditRecord>, StorageError> {
        let records = self.list_all_audits()?;
        Ok(query_tool_audit_records(records.iter(), query)
            .into_iter()
            .cloned()
            .collect())
    }

    /// Replay queried audit rows into another D18D audit sink.
    pub fn replay_audits<T>(
        &self,
        query: &ToolAuditRecordQuery,
        sink: &mut T,
    ) -> Result<ToolAuditReplaySummary, StorageError>
    where
        T: ToolAuditSink,
    {
        let records = self.query_audits(query)?;
        let inventory = ToolAuditStoreInventorySummary::from_records(&records);
        let replayed_records = records.len();
        for record in records {
            sink.record_tool_audit(record);
        }
        Ok(ToolAuditReplaySummary {
            replayed_records,
            inventory,
        })
    }

    /// Summarize persisted audit rows without exposing payloads.
    pub fn inventory_summary(&self) -> Result<ToolAuditStoreInventorySummary, StorageError> {
        let records = self.list_all_audits()?;
        Ok(ToolAuditStoreInventorySummary::from_records(&records))
    }

    /// Summarize storage records used by the audit store without loading bodies.
    pub fn storage_inventory_summary(&self) -> Result<StorageRecordInventorySummary, StorageError> {
        self.backend.initialize()?;
        let page = self.backend.list_summaries(
            AUDIT_NAMESPACE,
            StorageListOptions {
                prefix: Some(AUDIT_PREFIX.to_string()),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;
        Ok(page.inventory_summary())
    }

    fn list_all_audits(&self) -> Result<Vec<ToolAuditRecord>, StorageError> {
        self.backend.initialize()?;
        let page = self.backend.list(
            AUDIT_NAMESPACE,
            StorageListOptions {
                prefix: Some(AUDIT_PREFIX.to_string()),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;
        page.records.iter().map(decode_audit_record).collect()
    }
}

/// Summary for replaying stored audit rows into a sink.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ToolAuditReplaySummary {
    /// Number of records replayed into the sink.
    pub replayed_records: usize,
    /// Payload-free summary of the replayed records.
    pub inventory: ToolAuditStoreInventorySummary,
}

impl ToolAuditReplaySummary {
    /// Return whether no audit rows were replayed.
    pub fn is_empty(&self) -> bool {
        self.replayed_records == 0
    }

    /// Return whether any replayed row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.inventory.requires_follow_up()
    }
}

/// Storage-backed implementation of the D18D [`ToolAuditSink`] interface.
///
/// The sink trait is intentionally infallible so runtimes can emit audit rows
/// without taking a storage dependency. This adapter records storage failures
/// for the host to inspect after a batch or invocation.
pub struct StorageToolAuditSink<S> {
    store: ToolAuditStore<S>,
    failures: Vec<ToolAuditSinkFailure>,
}

impl<S> StorageToolAuditSink<S>
where
    S: StorageBackend,
{
    /// Create a sink backed by a storage backend.
    pub fn new(backend: S) -> Self {
        Self::from_store(ToolAuditStore::new(backend))
    }

    /// Create a sink backed by an existing audit store.
    pub fn from_store(store: ToolAuditStore<S>) -> Self {
        Self {
            store,
            failures: Vec::new(),
        }
    }

    /// Borrow the underlying audit store.
    pub fn store(&self) -> &ToolAuditStore<S> {
        &self.store
    }

    /// Return storage failures captured by the sink.
    pub fn failures(&self) -> &[ToolAuditSinkFailure] {
        &self.failures
    }

    /// Return whether any audit row failed to persist.
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    /// Remove all captured storage failures and return them to the caller.
    pub fn drain_failures(&mut self) -> Vec<ToolAuditSinkFailure> {
        std::mem::take(&mut self.failures)
    }

    /// Consume the sink and return the underlying audit store.
    pub fn into_store(self) -> ToolAuditStore<S> {
        self.store
    }
}

impl<S> ToolAuditSink for StorageToolAuditSink<S>
where
    S: StorageBackend,
{
    fn record_tool_audit(&mut self, record: ToolAuditRecord) {
        if let Err(error) = self.store.record_audit(record.clone()) {
            self.failures
                .push(ToolAuditSinkFailure::new(record.call_id, error));
        }
    }
}

/// One audit row that could not be persisted by [`StorageToolAuditSink`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSinkFailure {
    /// Call id for the audit row that failed to persist.
    pub call_id: String,
    /// Storage error rendered without exposing payloads.
    pub message: String,
}

impl ToolAuditSinkFailure {
    /// Create a failure summary for one call id and storage error.
    pub fn new(call_id: impl Into<String>, error: StorageError) -> Self {
        Self {
            call_id: call_id.into(),
            message: error.to_string(),
        }
    }
}

/// Payload-free inventory summary for persisted tool audit rows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ToolAuditStoreInventorySummary {
    /// Total audit rows.
    pub total_records: usize,
    /// Completed calls.
    pub completed_records: usize,
    /// Failed calls.
    pub failed_records: usize,
    /// Active calls that should not normally remain in persisted audit.
    pub active_records: usize,
    /// Calls waiting on approval.
    pub approval_pending_records: usize,
    /// Calls with denied approval.
    pub approval_denied_records: usize,
    /// Calls with expired approval.
    pub approval_expired_records: usize,
    /// Calls whose result summary contains an error.
    pub records_with_errors: usize,
    /// Calls whose result summary emitted artifact or memory references.
    pub records_with_references: usize,
}

impl ToolAuditStoreInventorySummary {
    /// Construct an empty summary.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Summarize audit records without exposing payloads.
    pub fn from_records(records: &[ToolAuditRecord]) -> Self {
        let mut summary = Self::empty();
        for record in records {
            summary.total_records += 1;
            match record.status {
                ToolCallStatus::Completed => summary.completed_records += 1,
                ToolCallStatus::Failed => summary.failed_records += 1,
                ToolCallStatus::Queued
                | ToolCallStatus::Validating
                | ToolCallStatus::AwaitingApproval
                | ToolCallStatus::Running => summary.active_records += 1,
                ToolCallStatus::Cancelled => {}
            }
            match record.approval_state {
                ApprovalState::Pending => summary.approval_pending_records += 1,
                ApprovalState::Denied => summary.approval_denied_records += 1,
                ApprovalState::Expired => summary.approval_expired_records += 1,
                ApprovalState::NotRequired | ApprovalState::Granted => {}
            }
            if record.result_summary.has_error {
                summary.records_with_errors += 1;
            }
            if record.result_summary.emitted_references() {
                summary.records_with_references += 1;
            }
        }
        summary
    }

    /// Return whether there are no audit rows.
    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    /// Return whether any row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.active_records > 0
            || self.failed_records > 0
            || self.approval_pending_records > 0
            || self.approval_denied_records > 0
            || self.approval_expired_records > 0
            || self.records_with_errors > 0
    }
}

fn audit_key(call_id: &str) -> String {
    format!("{AUDIT_PREFIX}{call_id}.json")
}

fn json_to_body(value: &JsonValue) -> Result<Vec<u8>, StorageError> {
    json_serialize(value)
        .map(|json| json.into_bytes())
        .map_err(|error| StorageError::Backend {
            message: format!("tool audit json serialization failed: {error}"),
        })
}

fn decode_audit_record(record: &StorageRecord) -> Result<ToolAuditRecord, StorageError> {
    let text = std::str::from_utf8(&record.body).map_err(|error| StorageError::Backend {
        message: format!("tool audit body was not utf-8: {error}"),
    })?;
    let json = json_parse(text).map_err(|error| StorageError::Backend {
        message: format!("tool audit body was not json: {error}"),
    })?;
    audit_record_from_json(&json)
}

fn audit_metadata(record: &ToolAuditRecord) -> JsonValue {
    JsonValue::Object(vec![
        string_field("call_id", &record.call_id),
        string_field("tool_id", &record.tool_id),
        string_field("requested_by", record.requested_by.as_str()),
        string_field("status", record.status.as_str()),
        string_field("approval_state", record.approval_state.as_str()),
        bool_field("result_ok", record.result_summary.ok),
        bool_field("has_error", record.result_summary.has_error),
        usize_field(
            "artifact_ref_count",
            record.result_summary.artifact_ref_count,
        ),
        usize_field("memory_ref_count", record.result_summary.memory_ref_count),
        bool_field("has_references", record.result_summary.emitted_references()),
    ])
}

fn audit_record_to_json(record: &ToolAuditRecord) -> JsonValue {
    JsonValue::Object(vec![
        string_field("call_id", &record.call_id),
        string_field("tool_id", &record.tool_id),
        string_field("requested_by", record.requested_by.as_str()),
        optional_u64_field("started_at", record.started_at),
        optional_u64_field("completed_at", record.completed_at),
        string_field("status", record.status.as_str()),
        string_field("approval_state", record.approval_state.as_str()),
        optional_string_field("lock_scope", record.lock_scope.as_deref()),
        (
            "result_summary".to_string(),
            result_summary_to_json(&record.result_summary),
        ),
    ])
}

fn result_summary_to_json(summary: &ToolResultAuditSummary) -> JsonValue {
    JsonValue::Object(vec![
        bool_field("ok", summary.ok),
        bool_field("has_output", summary.has_output),
        bool_field("has_error", summary.has_error),
        optional_string_field("error_kind", summary.error_kind.map(|kind| kind.as_str())),
        usize_field("artifact_ref_count", summary.artifact_ref_count),
        usize_field("memory_ref_count", summary.memory_ref_count),
        optional_u64_field("bytes_in", summary.bytes_in),
        optional_u64_field("bytes_out", summary.bytes_out),
    ])
}

fn audit_record_from_json(value: &JsonValue) -> Result<ToolAuditRecord, StorageError> {
    let object = expect_object(value, "$")?;
    Ok(ToolAuditRecord {
        call_id: required_string(object, "call_id")?,
        tool_id: required_string(object, "tool_id")?,
        requested_by: parse_requested_by(&required_string(object, "requested_by")?)?,
        started_at: optional_u64(object, "started_at")?,
        completed_at: optional_u64(object, "completed_at")?,
        status: parse_status(&required_string(object, "status")?)?,
        approval_state: parse_approval_state(&required_string(object, "approval_state")?)?,
        lock_scope: optional_string(object, "lock_scope")?,
        result_summary: result_summary_from_json(required_value(object, "result_summary")?)?,
    })
}

fn result_summary_from_json(value: &JsonValue) -> Result<ToolResultAuditSummary, StorageError> {
    let object = expect_object(value, "$.result_summary")?;
    Ok(ToolResultAuditSummary {
        ok: required_bool(object, "ok")?,
        has_output: required_bool(object, "has_output")?,
        has_error: required_bool(object, "has_error")?,
        error_kind: optional_string(object, "error_kind")?
            .as_deref()
            .map(parse_error_kind)
            .transpose()?,
        artifact_ref_count: required_usize(object, "artifact_ref_count")?,
        memory_ref_count: required_usize(object, "memory_ref_count")?,
        bytes_in: optional_u64(object, "bytes_in")?,
        bytes_out: optional_u64(object, "bytes_out")?,
    })
}

fn string_field(name: &str, value: &str) -> (String, JsonValue) {
    (name.to_string(), JsonValue::String(value.to_string()))
}

fn bool_field(name: &str, value: bool) -> (String, JsonValue) {
    (name.to_string(), JsonValue::Bool(value))
}

fn usize_field(name: &str, value: usize) -> (String, JsonValue) {
    (
        name.to_string(),
        JsonValue::Number(JsonNumber::Integer(value as i64)),
    )
}

fn optional_string_field(name: &str, value: Option<&str>) -> (String, JsonValue) {
    (
        name.to_string(),
        value
            .map(|value| JsonValue::String(value.to_string()))
            .unwrap_or(JsonValue::Null),
    )
}

fn optional_u64_field(name: &str, value: Option<u64>) -> (String, JsonValue) {
    (
        name.to_string(),
        value
            .map(|value| JsonValue::Number(JsonNumber::Integer(value as i64)))
            .unwrap_or(JsonValue::Null),
    )
}

fn expect_object<'a>(
    value: &'a JsonValue,
    path: &'static str,
) -> Result<&'a Vec<(String, JsonValue)>, StorageError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(invalid_json(path, "must be an object")),
    }
}

fn required_value<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
) -> Result<&'a JsonValue, StorageError> {
    object
        .iter()
        .find(|(name, _)| name == field)
        .map(|(_, value)| value)
        .ok_or_else(|| invalid_json(field, "is required"))
}

fn required_string(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<String, StorageError> {
    match required_value(object, field)? {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(invalid_json(field, "must be a string")),
    }
}

fn optional_string(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<Option<String>, StorageError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(value) => Ok(Some(value.clone())),
        _ => Err(invalid_json(field, "must be a string or null")),
    }
}

fn required_bool(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<bool, StorageError> {
    match required_value(object, field)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(invalid_json(field, "must be a boolean")),
    }
}

fn required_usize(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<usize, StorageError> {
    optional_u64(object, field)?
        .map(|value| value as usize)
        .ok_or_else(|| invalid_json(field, "must be a non-negative integer"))
}

fn optional_u64(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<Option<u64>, StorageError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => Ok(Some(*value as u64)),
        _ => Err(invalid_json(
            field,
            "must be a non-negative integer or null",
        )),
    }
}

fn parse_requested_by(value: &str) -> Result<RequestedBy, StorageError> {
    match value {
        "user" => Ok(RequestedBy::User),
        "session" => Ok(RequestedBy::Session),
        "job" => Ok(RequestedBy::Job),
        "agent" => Ok(RequestedBy::Agent),
        "system" => Ok(RequestedBy::System),
        _ => Err(invalid_json(
            "requested_by",
            "contains an unknown request origin",
        )),
    }
}

fn parse_status(value: &str) -> Result<ToolCallStatus, StorageError> {
    match value {
        "queued" => Ok(ToolCallStatus::Queued),
        "validating" => Ok(ToolCallStatus::Validating),
        "awaiting_approval" => Ok(ToolCallStatus::AwaitingApproval),
        "running" => Ok(ToolCallStatus::Running),
        "completed" => Ok(ToolCallStatus::Completed),
        "failed" => Ok(ToolCallStatus::Failed),
        "cancelled" => Ok(ToolCallStatus::Cancelled),
        _ => Err(invalid_json("status", "contains an unknown call status")),
    }
}

fn parse_approval_state(value: &str) -> Result<ApprovalState, StorageError> {
    match value {
        "not_required" => Ok(ApprovalState::NotRequired),
        "pending" => Ok(ApprovalState::Pending),
        "granted" => Ok(ApprovalState::Granted),
        "denied" => Ok(ApprovalState::Denied),
        "expired" => Ok(ApprovalState::Expired),
        _ => Err(invalid_json(
            "approval_state",
            "contains an unknown approval state",
        )),
    }
}

fn parse_error_kind(value: &str) -> Result<ToolErrorKind, StorageError> {
    match value {
        "ToolNotFound" => Ok(ToolErrorKind::ToolNotFound),
        "ToolValidationError" => Ok(ToolErrorKind::ToolValidationError),
        "ToolPermissionDenied" => Ok(ToolErrorKind::ToolPermissionDenied),
        "ToolTierDenied" => Ok(ToolErrorKind::ToolTierDenied),
        "ToolApprovalRequired" => Ok(ToolErrorKind::ToolApprovalRequired),
        "ToolApprovalDenied" => Ok(ToolErrorKind::ToolApprovalDenied),
        "ToolConflict" => Ok(ToolErrorKind::ToolConflict),
        "ToolTimeout" => Ok(ToolErrorKind::ToolTimeout),
        "ToolCancelled" => Ok(ToolErrorKind::ToolCancelled),
        "ToolExecutionError" => Ok(ToolErrorKind::ToolExecutionError),
        _ => Err(invalid_json(
            "error_kind",
            "contains an unknown tool error kind",
        )),
    }
}

fn invalid_json(field: &'static str, message: &'static str) -> StorageError {
    StorageError::Backend {
        message: format!("tool audit json field {field} {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_tool_api::{InMemoryToolAuditSink, ToolAuditRecordSort};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_core::InMemoryStorageBackend;
    use storage_local_folder::LocalFolderStorageBackend;

    fn sample_record(call_id: &str) -> ToolAuditRecord {
        ToolAuditRecord {
            call_id: call_id.to_string(),
            tool_id: "artifact.create".to_string(),
            requested_by: RequestedBy::Agent,
            started_at: Some(100),
            completed_at: Some(120),
            status: ToolCallStatus::Completed,
            approval_state: ApprovalState::NotRequired,
            lock_scope: Some("artifact".to_string()),
            result_summary: ToolResultAuditSummary {
                ok: true,
                has_output: true,
                has_error: false,
                error_kind: None,
                artifact_ref_count: 1,
                memory_ref_count: 0,
                bytes_in: Some(64),
                bytes_out: Some(128),
            },
        }
    }

    fn failed_record(call_id: &str) -> ToolAuditRecord {
        ToolAuditRecord {
            call_id: call_id.to_string(),
            tool_id: "memory.search".to_string(),
            requested_by: RequestedBy::Session,
            started_at: Some(150),
            completed_at: Some(151),
            status: ToolCallStatus::Failed,
            approval_state: ApprovalState::Denied,
            lock_scope: None,
            result_summary: ToolResultAuditSummary {
                ok: false,
                has_output: false,
                has_error: true,
                error_kind: Some(ToolErrorKind::ToolPermissionDenied),
                artifact_ref_count: 0,
                memory_ref_count: 0,
                bytes_in: None,
                bytes_out: None,
            },
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "chief-tool-audit-store-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn record_fetch_and_query_audit_rows() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let first = store.record_audit(sample_record("call_1")).unwrap();
        let second = store.record_audit(failed_record("call_2")).unwrap();

        assert_eq!(store.fetch_audit("call_1").unwrap(), Some(first.clone()));
        assert_eq!(store.fetch_audit("missing").unwrap(), None);

        let referenced = store
            .query_audits(
                &ToolAuditRecordQuery::new()
                    .with_references(true)
                    .sorted_by(ToolAuditRecordSort::CompletedAtDesc),
            )
            .unwrap();
        assert_eq!(referenced, vec![first]);

        let failures = store
            .query_audits(
                &ToolAuditRecordQuery::new()
                    .with_error(true)
                    .with_approval_state(ApprovalState::Denied),
            )
            .unwrap();
        assert_eq!(failures, vec![second]);
    }

    #[test]
    fn duplicate_audit_rows_are_rejected() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        store.record_audit(sample_record("call_1")).unwrap();

        let error = store
            .record_audit(sample_record("call_1"))
            .expect_err("call ids are append-only audit keys");

        assert!(matches!(error, StorageError::Conflict { .. }));
    }

    #[test]
    fn inventory_summaries_are_payload_free() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        store.record_audit(sample_record("call_1")).unwrap();
        store.record_audit(failed_record("call_2")).unwrap();

        let inventory = store.inventory_summary().unwrap();
        assert_eq!(
            inventory,
            ToolAuditStoreInventorySummary {
                total_records: 2,
                completed_records: 1,
                failed_records: 1,
                active_records: 0,
                approval_pending_records: 0,
                approval_denied_records: 1,
                approval_expired_records: 0,
                records_with_errors: 1,
                records_with_references: 1,
            }
        );
        assert!(inventory.requires_follow_up());

        let storage = store.storage_inventory_summary().unwrap();
        assert_eq!(storage.total_records, 2);
        assert_eq!(storage.records_with_metadata, 2);
        assert_eq!(storage.json_records, 2);
    }

    #[test]
    fn replay_audits_into_existing_sink() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        store.record_audit(sample_record("call_1")).unwrap();
        store.record_audit(failed_record("call_2")).unwrap();

        let mut sink = InMemoryToolAuditSink::new();
        let summary = store
            .replay_audits(
                &ToolAuditRecordQuery::new()
                    .with_error(true)
                    .sorted_by(ToolAuditRecordSort::CompletedAtDesc),
                &mut sink,
            )
            .unwrap();

        assert_eq!(summary.replayed_records, 1);
        assert!(summary.requires_follow_up());
        assert_eq!(summary.inventory.failed_records, 1);
        assert_eq!(sink.records(), &[failed_record("call_2")]);
    }

    #[test]
    fn storage_sink_records_rows_and_tracks_failures() {
        let mut sink = StorageToolAuditSink::new(InMemoryStorageBackend::new());

        sink.record_tool_audit(sample_record("call_1"));
        assert!(!sink.has_failures());
        assert_eq!(
            sink.store().fetch_audit("call_1").unwrap(),
            Some(sample_record("call_1"))
        );

        sink.record_tool_audit(sample_record("call_1"));
        assert!(sink.has_failures());
        let failures = sink.drain_failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].call_id, "call_1");
        assert!(failures[0].message.contains("storage conflict"));
        assert!(sink.failures().is_empty());
    }

    #[test]
    fn local_folder_backend_persists_audit_records() {
        let root = temp_root("local-folder");
        let record = sample_record("call_1");

        {
            let store = ToolAuditStore::new(LocalFolderStorageBackend::new(&root));
            store.record_audit(record.clone()).unwrap();
        }

        {
            let store = ToolAuditStore::new(LocalFolderStorageBackend::new(&root));
            assert_eq!(store.fetch_audit("call_1").unwrap(), Some(record));
            assert_eq!(store.inventory_summary().unwrap().total_records, 1);
        }

        let _ = fs::remove_dir_all(&root);
    }
}
