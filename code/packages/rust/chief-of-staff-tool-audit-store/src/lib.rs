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
use std::fmt::{self, Display, Formatter};
use storage_core::{
    Revision, StorageBackend, StorageError, StorageListOptions, StoragePutInput, StorageRecord,
    StorageRecordInventorySummary,
};

const AUDIT_NAMESPACE: &str = "chief.tool.audit";
const AUDIT_PREFIX: &str = "calls/";
const AUDIT_CONTENT_TYPE: &str = "application/vnd.coding-adventures.tool-audit+json";
const CHECKPOINT_PREFIX: &str = "checkpoints/";
const CHECKPOINT_CONTENT_TYPE: &str =
    "application/vnd.coding-adventures.tool-audit-checkpoint+json";

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

    /// Record a batch of payload-free audit rows and summarize successes and
    /// storage failures.
    pub fn record_audit_batch<I>(&self, records: I) -> ToolAuditBatchWriteSummary
    where
        I: IntoIterator<Item = ToolAuditRecord>,
    {
        let mut summary = ToolAuditBatchWriteSummary::empty();
        let mut stored = Vec::new();
        for record in records {
            summary.attempted_records += 1;
            match self.record_audit(record.clone()) {
                Ok(record) => {
                    summary.stored_records += 1;
                    stored.push(record);
                }
                Err(error) => {
                    summary.failed_records += 1;
                    summary
                        .failures
                        .push(ToolAuditSinkFailure::new(record.call_id, error));
                }
            }
        }
        summary.inventory = ToolAuditStoreInventorySummary::from_records(&stored);
        summary
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

    /// Return a deterministic page of audit rows after a replay checkpoint.
    pub fn query_audits_after_checkpoint(
        &self,
        checkpoint: &ToolAuditReadCheckpoint,
        limit: Option<usize>,
    ) -> Result<ToolAuditCheckpointPage, StorageError> {
        let mut records = self.list_all_audits()?;
        records.sort_by(compare_audit_watermarks);
        let mut records: Vec<_> = records
            .into_iter()
            .filter(|record| audit_checkpoint_for(record).is_after(checkpoint))
            .collect();
        if let Some(limit) = limit {
            records.truncate(limit);
        }
        let next_checkpoint = records
            .last()
            .map(audit_checkpoint_for)
            .unwrap_or_else(|| checkpoint.clone());
        let inventory = ToolAuditStoreInventorySummary::from_records(&records);
        Ok(ToolAuditCheckpointPage {
            records,
            next_checkpoint,
            inventory,
        })
    }

    /// Fetch a named replay checkpoint for a supervisor or reader.
    pub fn fetch_checkpoint(
        &self,
        name: &str,
    ) -> Result<Option<ToolAuditStoredCheckpoint>, StorageError> {
        self.backend.initialize()?;
        self.backend
            .get(AUDIT_NAMESPACE, &checkpoint_key(name)?)?
            .map(|record| decode_stored_checkpoint(&record))
            .transpose()
    }

    /// Persist a named replay checkpoint for a supervisor or reader.
    pub fn save_checkpoint(
        &self,
        name: &str,
        checkpoint: ToolAuditReadCheckpoint,
    ) -> Result<ToolAuditStoredCheckpoint, StorageError> {
        self.backend.initialize()?;
        let record = self
            .backend
            .put(checkpoint_put_input(name, &checkpoint, None)?)?;
        decode_stored_checkpoint(&record)
    }

    /// Persist a named checkpoint only when it moves the reader forward.
    pub fn advance_checkpoint(
        &self,
        name: &str,
        checkpoint: ToolAuditReadCheckpoint,
    ) -> Result<ToolAuditStoredCheckpoint, StorageError> {
        self.backend.initialize()?;
        let key = checkpoint_key(name)?;
        let existing = self.backend.get(AUDIT_NAMESPACE, &key)?;
        if let Some(existing) = existing {
            let stored = decode_stored_checkpoint(&existing)?;
            if !checkpoint.is_after(&stored.checkpoint) {
                return Ok(stored);
            }
            let record = self.backend.put(checkpoint_put_input(
                name,
                &checkpoint,
                Some(existing.revision.clone()),
            )?)?;
            return decode_stored_checkpoint(&record);
        }

        let record = self
            .backend
            .put(checkpoint_put_input(name, &checkpoint, None)?)?;
        decode_stored_checkpoint(&record)
    }

    /// Inspect a supervisor checkpoint without delivering rows or advancing the
    /// durable cursor.
    pub fn inspect_supervisor_checkpoint(
        &self,
        checkpoint_name: &str,
        max_records: usize,
    ) -> Result<ToolAuditSupervisorCheckpointStatus, StorageError> {
        if max_records == 0 {
            return Err(StorageError::Validation {
                field: "max_records".to_string(),
                message: "must be greater than zero".to_string(),
            });
        }

        let stored_checkpoint = self.fetch_checkpoint(checkpoint_name)?;
        let starting_checkpoint = stored_checkpoint
            .as_ref()
            .map(|stored| stored.checkpoint.clone())
            .unwrap_or_else(ToolAuditReadCheckpoint::beginning);
        let page = self.query_audits_after_checkpoint(&starting_checkpoint, Some(max_records))?;
        let pending_records = page.len();
        let reached_end_of_log = pending_records < max_records;

        Ok(ToolAuditSupervisorCheckpointStatus {
            checkpoint_name: checkpoint_name.to_string(),
            max_records,
            stored_checkpoint,
            starting_checkpoint,
            next_checkpoint: page.next_checkpoint,
            pending_records,
            inventory: page.inventory,
            reached_end_of_log,
        })
    }

    /// Plan bounded supervisor replay pages without delivering rows or
    /// advancing the durable cursor.
    pub fn plan_supervisor_checkpoint_drain(
        &self,
        checkpoint_name: &str,
        max_records_per_tick: usize,
        max_ticks: usize,
    ) -> Result<ToolAuditSupervisorDrainPlanSummary, StorageError> {
        if max_records_per_tick == 0 {
            return Err(StorageError::Validation {
                field: "max_records_per_tick".to_string(),
                message: "must be greater than zero".to_string(),
            });
        }
        if max_ticks == 0 {
            return Err(StorageError::Validation {
                field: "max_ticks".to_string(),
                message: "must be greater than zero".to_string(),
            });
        }

        let stored_checkpoint = self.fetch_checkpoint(checkpoint_name)?;
        let starting_checkpoint = stored_checkpoint
            .as_ref()
            .map(|stored| stored.checkpoint.clone())
            .unwrap_or_else(ToolAuditReadCheckpoint::beginning);
        let mut next_start = starting_checkpoint.clone();
        let mut pages = Vec::new();
        let mut planned_records = 0;
        let mut inventory = ToolAuditStoreInventorySummary::empty();

        for _ in 0..max_ticks {
            let page =
                self.query_audits_after_checkpoint(&next_start, Some(max_records_per_tick))?;
            let pending_records = page.len();
            let reached_end_of_log = pending_records < max_records_per_tick;
            merge_inventory(&mut inventory, page.inventory);
            planned_records += pending_records;

            let summary = ToolAuditSupervisorDrainPlanPage {
                max_records: max_records_per_tick,
                starting_checkpoint: next_start,
                next_checkpoint: page.next_checkpoint,
                pending_records,
                inventory: page.inventory,
                reached_end_of_log,
            };
            next_start = summary.next_checkpoint.clone();
            pages.push(summary);

            if reached_end_of_log {
                break;
            }
        }

        Ok(ToolAuditSupervisorDrainPlanSummary {
            checkpoint_name: checkpoint_name.to_string(),
            max_records_per_tick,
            max_ticks,
            stored_checkpoint,
            starting_checkpoint,
            pages,
            planned_records,
            inventory,
        })
    }

    /// Replay the next deterministic page for a named checkpoint and advance
    /// that checkpoint after delivery.
    pub fn replay_audits_from_checkpoint<T>(
        &self,
        checkpoint_name: &str,
        limit: Option<usize>,
        sink: &mut T,
    ) -> Result<ToolAuditCheckpointReplaySummary, StorageError>
    where
        T: ToolAuditSink,
    {
        let starting_checkpoint = self
            .fetch_checkpoint(checkpoint_name)?
            .map(|stored| stored.checkpoint)
            .unwrap_or_else(ToolAuditReadCheckpoint::beginning);
        let page = self.query_audits_after_checkpoint(&starting_checkpoint, limit)?;
        let replayed_records = page.len();
        for record in page.records {
            sink.record_tool_audit(record);
        }
        let stored_checkpoint = if replayed_records == 0 {
            self.fetch_checkpoint(checkpoint_name)?
        } else {
            Some(self.advance_checkpoint(checkpoint_name, page.next_checkpoint.clone())?)
        };

        Ok(ToolAuditCheckpointReplaySummary {
            checkpoint_name: checkpoint_name.to_string(),
            starting_checkpoint,
            next_checkpoint: page.next_checkpoint,
            stored_checkpoint,
            replayed_records,
            inventory: page.inventory,
        })
    }

    /// Drain one bounded replay page for a supervisor checkpoint.
    ///
    /// This is the host-loop friendly form of checkpointed replay: callers get
    /// explicit progress and end-of-log signals for scheduling the next drain.
    pub fn drain_supervisor_checkpoint<T>(
        &self,
        checkpoint_name: &str,
        max_records: usize,
        sink: &mut T,
    ) -> Result<ToolAuditSupervisorDrainSummary, StorageError>
    where
        T: ToolAuditSink,
    {
        if max_records == 0 {
            return Err(StorageError::Validation {
                field: "max_records".to_string(),
                message: "must be greater than zero".to_string(),
            });
        }

        let replay =
            self.replay_audits_from_checkpoint(checkpoint_name, Some(max_records), sink)?;
        let reached_end_of_log = replay.replayed_records < max_records;
        Ok(ToolAuditSupervisorDrainSummary {
            max_records,
            replay,
            reached_end_of_log,
        })
    }

    /// Drain bounded supervisor replay ticks until the checkpoint catches up or
    /// the tick budget is exhausted.
    pub fn drain_supervisor_checkpoint_loop<T>(
        &self,
        checkpoint_name: &str,
        max_records_per_tick: usize,
        max_ticks: usize,
        sink: &mut T,
    ) -> Result<ToolAuditSupervisorDrainLoopSummary, StorageError>
    where
        T: ToolAuditSink,
    {
        if max_records_per_tick == 0 {
            return Err(StorageError::Validation {
                field: "max_records_per_tick".to_string(),
                message: "must be greater than zero".to_string(),
            });
        }
        if max_ticks == 0 {
            return Err(StorageError::Validation {
                field: "max_ticks".to_string(),
                message: "must be greater than zero".to_string(),
            });
        }

        let mut ticks = Vec::new();
        let mut drained_records = 0;
        for _ in 0..max_ticks {
            let tick =
                self.drain_supervisor_checkpoint(checkpoint_name, max_records_per_tick, sink)?;
            drained_records += tick.replay.replayed_records;
            let reached_end_of_log = tick.reached_end_of_log;
            ticks.push(tick);
            if reached_end_of_log {
                break;
            }
        }

        Ok(ToolAuditSupervisorDrainLoopSummary {
            max_records_per_tick,
            max_ticks,
            ticks,
            drained_records,
        })
    }

    /// Plan and execute a bounded supervisor replay drain.
    ///
    /// The report keeps the read-only preflight plan beside the actual drain
    /// loop result so schedulers can compare expected work to delivered work
    /// without exposing payloads.
    pub fn drain_supervisor_checkpoint_loop_with_plan<T>(
        &self,
        checkpoint_name: &str,
        max_records_per_tick: usize,
        max_ticks: usize,
        sink: &mut T,
    ) -> Result<ToolAuditSupervisorDrainRunReport, StorageError>
    where
        T: ToolAuditSink,
    {
        let plan = self.plan_supervisor_checkpoint_drain(
            checkpoint_name,
            max_records_per_tick,
            max_ticks,
        )?;
        let drain = self.drain_supervisor_checkpoint_loop(
            checkpoint_name,
            max_records_per_tick,
            max_ticks,
            sink,
        )?;
        Ok(ToolAuditSupervisorDrainRunReport { plan, drain })
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

/// Replay checkpoint for incrementally reading persisted audit rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolAuditReadCheckpoint {
    /// Monotonic audit timestamp chosen from completed_at, started_at, or zero.
    pub timestamp_ms: u64,
    /// Last call id observed at the timestamp.
    pub call_id: String,
}

impl ToolAuditReadCheckpoint {
    /// Return the beginning-of-log checkpoint.
    pub fn beginning() -> Self {
        Self::default()
    }

    /// Create a checkpoint from a timestamp and call id.
    pub fn new(timestamp_ms: u64, call_id: impl Into<String>) -> Self {
        Self {
            timestamp_ms,
            call_id: call_id.into(),
        }
    }

    /// Return this checkpoint's timestamp for host checkpoint logs.
    pub fn checkpoint_timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }

    /// Return this checkpoint's call id for host checkpoint logs.
    pub fn checkpoint_call_id(&self) -> &str {
        &self.call_id
    }

    /// Return whether this checkpoint is after another checkpoint.
    pub fn is_after(&self, other: &Self) -> bool {
        (self.timestamp_ms, self.call_id.as_str()) > (other.timestamp_ms, other.call_id.as_str())
    }
}

/// One deterministic page returned after a replay checkpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolAuditCheckpointPage {
    /// Audit rows in checkpoint order.
    pub records: Vec<ToolAuditRecord>,
    /// Checkpoint to use for the next read.
    pub next_checkpoint: ToolAuditReadCheckpoint,
    /// Payload-free summary of the returned records.
    pub inventory: ToolAuditStoreInventorySummary,
}

impl ToolAuditCheckpointPage {
    /// Return whether the page contains no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Return the number of records in the page.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Return whether the page row count agrees with the payload-free inventory.
    pub fn record_count_matches_inventory(&self) -> bool {
        self.len() == self.inventory.total_records
    }

    /// Return the checkpoint timestamp to use for the next read.
    pub fn next_checkpoint_timestamp_ms(&self) -> u64 {
        self.next_checkpoint.checkpoint_timestamp_ms()
    }

    /// Return the checkpoint call id to use for the next read.
    pub fn next_checkpoint_call_id(&self) -> &str {
        self.next_checkpoint.checkpoint_call_id()
    }
}

/// Durable named replay checkpoint stored beside audit rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditStoredCheckpoint {
    /// Reader or supervisor name that owns the checkpoint.
    pub name: String,
    /// Last observed audit checkpoint for the reader.
    pub checkpoint: ToolAuditReadCheckpoint,
    /// Storage timestamp for the checkpoint record.
    pub updated_at: u64,
}

impl ToolAuditStoredCheckpoint {
    /// Return the durable checkpoint timestamp for host state logs.
    pub fn checkpoint_timestamp_ms(&self) -> u64 {
        self.checkpoint.timestamp_ms
    }

    /// Return the durable checkpoint call id for host state logs.
    pub fn checkpoint_call_id(&self) -> &str {
        &self.checkpoint.call_id
    }
}

/// Summary for replaying one named checkpoint page into a sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditCheckpointReplaySummary {
    /// Reader or supervisor checkpoint name used for replay.
    pub checkpoint_name: String,
    /// Checkpoint used to start this replay page.
    pub starting_checkpoint: ToolAuditReadCheckpoint,
    /// Checkpoint for the next replay call.
    pub next_checkpoint: ToolAuditReadCheckpoint,
    /// Durable checkpoint after replay, if a checkpoint exists.
    pub stored_checkpoint: Option<ToolAuditStoredCheckpoint>,
    /// Number of audit records replayed into the sink.
    pub replayed_records: usize,
    /// Payload-free summary of the replayed records.
    pub inventory: ToolAuditStoreInventorySummary,
}

impl ToolAuditCheckpointReplaySummary {
    /// Return whether no audit rows were replayed.
    pub fn is_empty(&self) -> bool {
        self.replayed_records == 0
    }

    /// Return whether replayed row count agrees with the payload-free inventory.
    pub fn replayed_count_matches_inventory(&self) -> bool {
        self.replayed_records == self.inventory.total_records
    }

    /// Return the starting checkpoint timestamp for host replay logs.
    pub fn starting_checkpoint_timestamp_ms(&self) -> u64 {
        self.starting_checkpoint.timestamp_ms
    }

    /// Return the starting checkpoint call id for host replay logs.
    pub fn starting_checkpoint_call_id(&self) -> &str {
        &self.starting_checkpoint.call_id
    }

    /// Return the checkpoint timestamp saved after this replay page.
    pub fn next_checkpoint_timestamp_ms(&self) -> u64 {
        self.next_checkpoint.timestamp_ms
    }

    /// Return the checkpoint call id saved after this replay page.
    pub fn next_checkpoint_call_id(&self) -> &str {
        &self.next_checkpoint.call_id
    }

    /// Return whether any replayed row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.inventory.requires_follow_up()
    }
}

/// Read-only status for a supervisor checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSupervisorCheckpointStatus {
    /// Reader or supervisor checkpoint name inspected.
    pub checkpoint_name: String,
    /// Maximum rows inspected for this status check.
    pub max_records: usize,
    /// Durable checkpoint loaded for the supervisor, if one exists.
    pub stored_checkpoint: Option<ToolAuditStoredCheckpoint>,
    /// Checkpoint used to start this status check.
    pub starting_checkpoint: ToolAuditReadCheckpoint,
    /// Checkpoint the next drain would advance to after this inspected page.
    pub next_checkpoint: ToolAuditReadCheckpoint,
    /// Number of rows ready for the next drain page.
    pub pending_records: usize,
    /// Payload-free summary of the inspected pending rows.
    pub inventory: ToolAuditStoreInventorySummary,
    /// Whether this status check reached the current end of the audit log.
    pub reached_end_of_log: bool,
}

impl ToolAuditSupervisorCheckpointStatus {
    /// Return whether no rows are waiting after this checkpoint.
    pub fn is_idle(&self) -> bool {
        self.pending_records == 0
    }

    /// Return whether pending row count agrees with the payload-free inventory.
    pub fn pending_count_matches_inventory(&self) -> bool {
        self.pending_records == self.inventory.total_records
    }

    /// Return whether a drain tick would deliver at least one row.
    pub fn has_pending_records(&self) -> bool {
        !self.is_idle()
    }

    /// Return whether a supervisor should run a drain tick.
    pub fn should_drain(&self) -> bool {
        self.has_pending_records()
    }

    /// Return the starting checkpoint timestamp for host status logs.
    pub fn starting_checkpoint_timestamp_ms(&self) -> u64 {
        self.starting_checkpoint.timestamp_ms
    }

    /// Return the starting checkpoint call id for host status logs.
    pub fn starting_checkpoint_call_id(&self) -> &str {
        &self.starting_checkpoint.call_id
    }

    /// Return the checkpoint timestamp the next drain page would advance to.
    pub fn next_checkpoint_timestamp_ms(&self) -> u64 {
        self.next_checkpoint.timestamp_ms
    }

    /// Return the checkpoint call id the next drain page would advance to.
    pub fn next_checkpoint_call_id(&self) -> &str {
        &self.next_checkpoint.call_id
    }

    /// Return whether more rows may remain beyond the inspected page.
    pub fn should_continue_after_page(&self) -> bool {
        !self.reached_end_of_log
    }

    /// Return whether any inspected pending row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.inventory.requires_follow_up()
    }

    /// Return whether the next drain page would advance the durable checkpoint.
    pub fn would_advance_checkpoint(&self) -> bool {
        self.has_pending_records()
    }
}

/// One read-only page in a planned supervisor drain run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSupervisorDrainPlanPage {
    /// Maximum rows inspected for this planned page.
    pub max_records: usize,
    /// Checkpoint used to start this planned page.
    pub starting_checkpoint: ToolAuditReadCheckpoint,
    /// Checkpoint the matching drain tick would advance to after this page.
    pub next_checkpoint: ToolAuditReadCheckpoint,
    /// Number of rows waiting in this planned page.
    pub pending_records: usize,
    /// Payload-free summary of the planned page.
    pub inventory: ToolAuditStoreInventorySummary,
    /// Whether this planned page reached the current end of the audit log.
    pub reached_end_of_log: bool,
}

impl ToolAuditSupervisorDrainPlanPage {
    /// Return whether this planned page has no rows.
    pub fn is_idle(&self) -> bool {
        self.pending_records == 0
    }

    /// Return whether pending row count agrees with the payload-free inventory.
    pub fn pending_count_matches_inventory(&self) -> bool {
        self.pending_records == self.inventory.total_records
    }

    /// Return whether this planned page has rows to drain.
    pub fn has_pending_records(&self) -> bool {
        !self.is_idle()
    }

    /// Return whether a supervisor should run the matching drain tick.
    pub fn should_drain(&self) -> bool {
        self.has_pending_records()
    }

    /// Return the starting checkpoint timestamp for host preflight logs.
    pub fn starting_checkpoint_timestamp_ms(&self) -> u64 {
        self.starting_checkpoint.timestamp_ms
    }

    /// Return the starting checkpoint call id for host preflight logs.
    pub fn starting_checkpoint_call_id(&self) -> &str {
        &self.starting_checkpoint.call_id
    }

    /// Return the checkpoint timestamp a matching drain tick would advance to.
    pub fn next_checkpoint_timestamp_ms(&self) -> u64 {
        self.next_checkpoint.timestamp_ms
    }

    /// Return the checkpoint call id a matching drain tick would advance to.
    pub fn next_checkpoint_call_id(&self) -> &str {
        &self.next_checkpoint.call_id
    }

    /// Return whether more rows may remain beyond this planned page.
    pub fn should_continue_after_page(&self) -> bool {
        !self.reached_end_of_log
    }

    /// Return whether any inspected row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.inventory.requires_follow_up()
    }

    /// Return whether the matching drain tick would advance the durable checkpoint.
    pub fn would_advance_checkpoint(&self) -> bool {
        self.has_pending_records()
    }
}

/// Read-only plan for a bounded supervisor drain run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSupervisorDrainPlanSummary {
    /// Reader or supervisor checkpoint name planned.
    pub checkpoint_name: String,
    /// Maximum rows each planned drain tick may replay.
    pub max_records_per_tick: usize,
    /// Maximum drain ticks the supervisor asked to plan.
    pub max_ticks: usize,
    /// Durable checkpoint loaded for the supervisor, if one exists.
    pub stored_checkpoint: Option<ToolAuditStoredCheckpoint>,
    /// Checkpoint used to start this plan.
    pub starting_checkpoint: ToolAuditReadCheckpoint,
    /// Planned pages in checkpoint order.
    pub pages: Vec<ToolAuditSupervisorDrainPlanPage>,
    /// Total rows a matching bounded drain run would replay.
    pub planned_records: usize,
    /// Payload-free summary of all planned rows.
    pub inventory: ToolAuditStoreInventorySummary,
}

impl ToolAuditSupervisorDrainPlanSummary {
    /// Return the number of planned pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Return how many requested planning ticks were not needed.
    pub fn remaining_tick_budget(&self) -> usize {
        self.max_ticks.saturating_sub(self.page_count())
    }

    /// Return whether the plan consumed every requested planning tick.
    pub fn used_all_ticks(&self) -> bool {
        self.remaining_tick_budget() == 0
    }

    /// Return whether more planning ticks were available when planning stopped.
    pub fn has_remaining_tick_budget(&self) -> bool {
        self.remaining_tick_budget() > 0
    }

    /// Return whether no rows are waiting in the planned run.
    pub fn is_idle(&self) -> bool {
        self.planned_records == 0
    }

    /// Return whether planned row count agrees with the aggregate inventory.
    pub fn planned_count_matches_inventory(&self) -> bool {
        self.planned_records == self.inventory.total_records
    }

    /// Return whether planned row count agrees with the sum of planned pages.
    pub fn planned_count_matches_pages(&self) -> bool {
        self.planned_records
            == self
                .pages
                .iter()
                .map(|page| page.pending_records)
                .sum::<usize>()
    }

    /// Return whether every planned page count agrees with its inventory.
    pub fn page_inventory_counts_match(&self) -> bool {
        self.pages
            .iter()
            .all(ToolAuditSupervisorDrainPlanPage::pending_count_matches_inventory)
    }

    /// Return whether all planned row counts agree with their inventories.
    pub fn plan_inventory_counts_match(&self) -> bool {
        self.planned_count_matches_inventory()
            && self.planned_count_matches_pages()
            && self.page_inventory_counts_match()
    }

    /// Return whether a matching drain run would replay at least one row.
    pub fn has_pending_records(&self) -> bool {
        !self.is_idle()
    }

    /// Return whether the plan reached the current end of the audit log.
    pub fn reached_end_of_log(&self) -> bool {
        self.pages
            .last()
            .map(|page| page.reached_end_of_log)
            .unwrap_or(false)
    }

    /// Return whether planning stopped because it used every allowed tick.
    pub fn exhausted_tick_budget(&self) -> bool {
        self.page_count() == self.max_ticks && !self.reached_end_of_log()
    }

    /// Return whether the supervisor should schedule another drain run.
    pub fn should_continue(&self) -> bool {
        !self.reached_end_of_log()
    }

    /// Return whether any planned row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.inventory.requires_follow_up()
    }

    /// Return the number of planned rows that need follow-up.
    pub fn follow_up_record_count(&self) -> usize {
        self.inventory.follow_up_records
    }

    /// Return whether a matching drain run would advance the durable checkpoint.
    pub fn would_advance_checkpoint(&self) -> bool {
        self.has_pending_records()
    }

    /// Return the last checkpoint a matching drain run would observe.
    pub fn last_checkpoint(&self) -> Option<&ToolAuditReadCheckpoint> {
        self.pages.last().map(|page| &page.next_checkpoint)
    }
}

/// Host-loop summary for one supervisor audit drain tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSupervisorDrainSummary {
    /// Maximum rows the supervisor asked to drain in this tick.
    pub max_records: usize,
    /// Checkpointed replay result for the drain tick.
    pub replay: ToolAuditCheckpointReplaySummary,
    /// Whether this tick reached the current end of the audit log.
    pub reached_end_of_log: bool,
}

impl ToolAuditSupervisorDrainSummary {
    /// Return whether the drain tick delivered no rows.
    pub fn is_idle(&self) -> bool {
        self.replay.is_empty()
    }

    /// Return whether replayed row count agrees with the payload-free inventory.
    pub fn replayed_count_matches_inventory(&self) -> bool {
        self.replay.replayed_count_matches_inventory()
    }

    /// Return whether the drain tick delivered at least one row.
    pub fn made_progress(&self) -> bool {
        !self.is_idle()
    }

    /// Return whether the supervisor should immediately schedule another drain.
    pub fn should_continue(&self) -> bool {
        !self.reached_end_of_log
    }

    /// Return whether any replayed row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.replay.requires_follow_up()
    }

    /// Return the number of replayed rows that need follow-up.
    pub fn follow_up_record_count(&self) -> usize {
        self.replay.inventory.follow_up_records
    }

    /// Return whether the durable checkpoint advanced during this tick.
    pub fn advanced_checkpoint(&self) -> bool {
        self.made_progress()
    }
}

/// Host-loop summary for a bounded supervisor drain run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolAuditSupervisorDrainLoopSummary {
    /// Maximum rows each drain tick may replay.
    pub max_records_per_tick: usize,
    /// Maximum drain ticks the supervisor asked to run.
    pub max_ticks: usize,
    /// Payload-free summaries for each drain tick that ran.
    pub ticks: Vec<ToolAuditSupervisorDrainSummary>,
    /// Total audit rows replayed into the sink across all ticks.
    pub drained_records: usize,
}

impl ToolAuditSupervisorDrainLoopSummary {
    /// Return the number of drain ticks that ran.
    pub fn tick_count(&self) -> usize {
        self.ticks.len()
    }

    /// Return how many requested drain ticks did not run.
    pub fn remaining_tick_budget(&self) -> usize {
        self.max_ticks.saturating_sub(self.tick_count())
    }

    /// Return whether the run consumed every requested drain tick.
    pub fn used_all_ticks(&self) -> bool {
        self.remaining_tick_budget() == 0
    }

    /// Return whether more ticks were available when the run stopped.
    pub fn has_remaining_tick_budget(&self) -> bool {
        self.remaining_tick_budget() > 0
    }

    /// Return whether no audit rows were replayed.
    pub fn is_idle(&self) -> bool {
        self.drained_records == 0
    }

    /// Return whether drained row count agrees with the sum of drain ticks.
    pub fn drained_count_matches_ticks(&self) -> bool {
        self.drained_records
            == self
                .ticks
                .iter()
                .map(|tick| tick.replay.replayed_records)
                .sum::<usize>()
    }

    /// Return whether every drain tick count agrees with its inventory.
    pub fn tick_inventory_counts_match(&self) -> bool {
        self.ticks
            .iter()
            .all(ToolAuditSupervisorDrainSummary::replayed_count_matches_inventory)
    }

    /// Return whether all drained row counts agree with their inventories.
    pub fn drain_inventory_counts_match(&self) -> bool {
        self.drained_count_matches_ticks() && self.tick_inventory_counts_match()
    }

    /// Return whether at least one audit row was replayed.
    pub fn made_progress(&self) -> bool {
        self.drained_records > 0
    }

    /// Return whether the run reached the current end of the audit log.
    pub fn reached_end_of_log(&self) -> bool {
        self.ticks
            .last()
            .map(|tick| tick.reached_end_of_log)
            .unwrap_or(false)
    }

    /// Return whether the run stopped because it used every allowed tick.
    pub fn exhausted_tick_budget(&self) -> bool {
        self.tick_count() == self.max_ticks && !self.reached_end_of_log()
    }

    /// Return whether the supervisor should schedule another drain run.
    pub fn should_continue(&self) -> bool {
        !self.reached_end_of_log()
    }

    /// Return whether any replayed row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.ticks.iter().any(|tick| tick.requires_follow_up())
    }

    /// Return the number of replayed rows that need follow-up.
    pub fn follow_up_record_count(&self) -> usize {
        self.ticks
            .iter()
            .map(ToolAuditSupervisorDrainSummary::follow_up_record_count)
            .sum()
    }

    /// Return whether any durable checkpoint advanced during the run.
    pub fn advanced_checkpoint(&self) -> bool {
        self.ticks.iter().any(|tick| tick.advanced_checkpoint())
    }

    /// Return the last replay checkpoint observed by this run.
    pub fn last_checkpoint(&self) -> Option<&ToolAuditReadCheckpoint> {
        self.ticks.last().map(|tick| &tick.replay.next_checkpoint)
    }
}

/// Combined preflight and result summary for a bounded supervisor drain run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSupervisorDrainRunReport {
    /// Read-only drain plan captured before any rows were emitted.
    pub plan: ToolAuditSupervisorDrainPlanSummary,
    /// Actual bounded drain loop result.
    pub drain: ToolAuditSupervisorDrainLoopSummary,
}

impl ToolAuditSupervisorDrainRunReport {
    /// Classify the drain run for scheduler decisions.
    pub fn outcome(&self) -> ToolAuditSupervisorDrainRunOutcome {
        if !self.matches_planned_record_count() {
            return ToolAuditSupervisorDrainRunOutcome::PlanDiverged;
        }
        if self.requires_follow_up() {
            return ToolAuditSupervisorDrainRunOutcome::NeedsFollowUp;
        }
        if self.should_continue() {
            return ToolAuditSupervisorDrainRunOutcome::NeedsContinuation;
        }
        if self.made_progress() {
            return ToolAuditSupervisorDrainRunOutcome::CaughtUp;
        }
        ToolAuditSupervisorDrainRunOutcome::Idle
    }

    /// Return the stable scheduler-facing label for this run outcome.
    pub fn outcome_label(&self) -> &'static str {
        self.outcome().as_str()
    }

    /// Return whether the outcome label parses back to the typed outcome.
    pub fn outcome_label_matches_outcome(&self) -> bool {
        ToolAuditSupervisorDrainRunOutcome::from_label(self.outcome_label()) == Some(self.outcome())
    }

    /// Return whether this run outcome leaves the scheduler idle.
    pub fn outcome_is_idle(&self) -> bool {
        self.outcome().is_idle()
    }

    /// Return whether rows were replayed and this run reached the current log end.
    pub fn is_caught_up(&self) -> bool {
        self.outcome().is_caught_up()
    }

    /// Return whether this run needs another drain pass.
    pub fn needs_continuation(&self) -> bool {
        self.outcome().needs_continuation()
    }

    /// Return whether this run outcome routes follow-up pressure to the host.
    pub fn needs_follow_up(&self) -> bool {
        self.outcome().needs_follow_up()
    }

    /// Return whether this run diverged from its preflight plan.
    pub fn plan_diverged(&self) -> bool {
        self.outcome().plan_diverged()
    }

    /// Return whether this run outcome asks the scheduler to take action.
    pub fn requires_scheduler_action(&self) -> bool {
        self.outcome().requires_scheduler_action()
    }

    /// Return whether this run intentionally leaves the scheduler idle.
    pub fn is_no_scheduler_action(&self) -> bool {
        self.scheduler_action().is_no_action()
    }

    /// Return the recommended scheduler action for this drain run.
    pub fn scheduler_action(&self) -> ToolAuditSupervisorDrainSchedulerAction {
        self.outcome().scheduler_action()
    }

    /// Return the stable scheduler-action label for this drain run.
    pub fn scheduler_action_label(&self) -> &'static str {
        self.scheduler_action().as_str()
    }

    /// Return whether the scheduler-action label parses back to the typed action.
    pub fn scheduler_action_label_matches_action(&self) -> bool {
        ToolAuditSupervisorDrainSchedulerAction::from_label(self.scheduler_action_label())
            == Some(self.scheduler_action())
    }

    /// Return whether the scheduler should run another drain pass.
    pub fn requests_continuation(&self) -> bool {
        self.scheduler_action().requests_continuation()
    }

    /// Return whether follow-up pressure should be routed to the host.
    pub fn routes_follow_up(&self) -> bool {
        self.scheduler_action().routes_follow_up()
    }

    /// Return whether the host should investigate preflight/drain drift.
    pub fn requires_plan_drift_investigation(&self) -> bool {
        self.scheduler_action().requires_plan_drift_investigation()
    }

    /// Classify why the host should investigate this run.
    pub fn host_investigation_kind(&self) -> ToolAuditSupervisorDrainHostInvestigationKind {
        ToolAuditSupervisorDrainHostInvestigationKind::from_investigation_flags(
            self.requires_plan_drift_investigation(),
            self.requires_count_drift_investigation(),
        )
    }

    /// Return the stable host-investigation classification label.
    pub fn host_investigation_label(&self) -> &'static str {
        self.host_investigation_kind().as_str()
    }

    /// Return whether the host-investigation label parses back to the typed classification.
    pub fn host_investigation_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainHostInvestigationKind::from_label(self.host_investigation_label())
            == Some(self.host_investigation_kind())
    }

    /// Return whether every stable classifier label parses back to its typed value.
    pub fn all_classifier_labels_match(&self) -> bool {
        self.outcome_label_matches_outcome()
            && self.scheduler_action_label_matches_action()
            && self.count_drift_label_matches_kind()
            && self.checkpoint_drift_label_matches_kind()
            && self.host_investigation_label_matches_kind()
    }

    /// Return whether the host should investigate any run divergence.
    pub fn requires_host_investigation(&self) -> bool {
        self.host_investigation_kind().requires_investigation()
    }

    /// Return whether the host investigation includes plan drift.
    pub fn requires_host_plan_drift_investigation(&self) -> bool {
        self.host_investigation_kind()
            .requires_plan_drift_investigation()
    }

    /// Return whether the host investigation includes count drift.
    pub fn requires_host_count_drift_investigation(&self) -> bool {
        self.host_investigation_kind()
            .requires_count_drift_investigation()
    }

    /// Return whether the host can skip drain-run investigation.
    pub fn is_no_host_investigation(&self) -> bool {
        self.host_investigation_kind().is_no_investigation()
    }

    /// Return whether any host attention is needed for this run.
    pub fn requires_host_attention(&self) -> bool {
        self.host_attention_kind().requires_attention()
    }

    /// Return whether no host attention is needed for this run.
    pub fn is_no_host_attention(&self) -> bool {
        self.host_attention_kind().is_no_attention()
    }

    /// Classify why the host should pay attention to this run.
    pub fn host_attention_kind(&self) -> ToolAuditSupervisorDrainHostAttentionKind {
        ToolAuditSupervisorDrainHostAttentionKind::from_attention_flags(
            self.requires_scheduler_action(),
            self.requires_host_investigation(),
            self.requires_run_integrity_investigation(),
        )
    }

    /// Return the stable host-attention classification label.
    pub fn host_attention_label(&self) -> &'static str {
        self.host_attention_kind().as_str()
    }

    /// Return whether the host-attention label parses back to the typed classification.
    pub fn host_attention_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainHostAttentionKind::from_label(self.host_attention_label())
            == Some(self.host_attention_kind())
    }

    /// Return whether host attention includes a scheduler action.
    pub fn host_attention_includes_scheduler_action(&self) -> bool {
        self.host_attention_kind().includes_scheduler_action()
    }

    /// Return whether host attention includes run-divergence investigation.
    pub fn host_attention_includes_host_investigation(&self) -> bool {
        self.host_attention_kind().includes_host_investigation()
    }

    /// Return whether host attention includes host-log integrity investigation.
    pub fn host_attention_includes_run_integrity_investigation(&self) -> bool {
        self.host_attention_kind()
            .includes_run_integrity_investigation()
    }

    /// Return whether more than one host-attention surface is active.
    pub fn has_multiple_host_attention(&self) -> bool {
        self.host_attention_kind().has_multiple_attention()
    }

    /// Return whether this run is terminal and needs no host attention.
    pub fn is_terminal_ready(&self) -> bool {
        self.terminal_readiness_kind().is_terminal_ready()
    }

    /// Classify whether this run is terminal-ready or still pending host work.
    pub fn terminal_readiness_kind(&self) -> ToolAuditSupervisorDrainTerminalReadinessKind {
        ToolAuditSupervisorDrainTerminalReadinessKind::from_outcome_and_attention(
            self.outcome(),
            self.host_attention_kind(),
        )
    }

    /// Return the stable terminal-readiness classification label.
    pub fn terminal_readiness_label(&self) -> &'static str {
        self.terminal_readiness_kind().as_str()
    }

    /// Return whether the terminal-readiness label parses back to the typed classification.
    pub fn terminal_readiness_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainTerminalReadinessKind::from_label(self.terminal_readiness_label())
            == Some(self.terminal_readiness_kind())
    }

    /// Return whether terminal readiness is blocked by scheduler action.
    pub fn terminal_readiness_requires_scheduler_action(&self) -> bool {
        self.terminal_readiness_kind().requires_scheduler_action()
    }

    /// Return whether terminal readiness is blocked by run-divergence investigation.
    pub fn terminal_readiness_requires_host_investigation(&self) -> bool {
        self.terminal_readiness_kind().requires_host_investigation()
    }

    /// Return whether terminal readiness is blocked by host-log integrity investigation.
    pub fn terminal_readiness_requires_run_integrity_investigation(&self) -> bool {
        self.terminal_readiness_kind()
            .requires_run_integrity_investigation()
    }

    /// Return whether terminal readiness is blocked by more than one host-attention surface.
    pub fn terminal_readiness_has_multiple_attention(&self) -> bool {
        self.terminal_readiness_kind().has_multiple_attention()
    }

    /// Return whether this run is terminal-ready because no rows were waiting.
    pub fn terminal_readiness_is_idle(&self) -> bool {
        self.terminal_readiness_kind().is_idle_ready()
    }

    /// Return whether this run is terminal-ready because it caught up to the log end.
    pub fn terminal_readiness_is_caught_up(&self) -> bool {
        self.terminal_readiness_kind().is_caught_up_ready()
    }

    /// Return whether the host has a pending decision to make for this run.
    pub fn requires_host_decision(&self) -> bool {
        self.host_decision_kind().requires_decision()
    }

    /// Classify the next host decision for this run.
    pub fn host_decision_kind(&self) -> ToolAuditSupervisorDrainHostDecisionKind {
        ToolAuditSupervisorDrainHostDecisionKind::from_decision_flags(
            self.requests_continuation(),
            self.routes_follow_up(),
            self.requires_host_plan_drift_investigation(),
            self.requires_host_count_drift_investigation(),
            self.requires_run_integrity_investigation(),
        )
    }

    /// Return the stable host-decision classification label.
    pub fn host_decision_label(&self) -> &'static str {
        self.host_decision_kind().as_str()
    }

    /// Return whether the host-decision label parses back to the typed classification.
    pub fn host_decision_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainHostDecisionKind::from_label(self.host_decision_label())
            == Some(self.host_decision_kind())
    }

    /// Return whether the host decision is that the run is terminal-ready.
    pub fn host_decision_is_terminal_ready(&self) -> bool {
        self.host_decision_kind().is_terminal_ready()
    }

    /// Return whether the next host decision is to schedule continuation.
    pub fn host_decision_schedules_continuation(&self) -> bool {
        self.requests_continuation()
    }

    /// Return whether the next host decision is to route follow-up pressure.
    pub fn host_decision_routes_follow_up(&self) -> bool {
        self.routes_follow_up()
    }

    /// Return whether the next host decision is to investigate plan drift.
    pub fn host_decision_investigates_plan_drift(&self) -> bool {
        self.requires_host_plan_drift_investigation()
    }

    /// Return whether the next host decision is to investigate count drift.
    pub fn host_decision_investigates_count_drift(&self) -> bool {
        self.requires_host_count_drift_investigation()
    }

    /// Return whether the next host decision is to investigate run integrity.
    pub fn host_decision_investigates_run_integrity(&self) -> bool {
        self.requires_run_integrity_investigation()
    }

    /// Return whether multiple host decisions are active.
    pub fn host_decision_has_multiple_actions(&self) -> bool {
        self.host_decision_kind().has_multiple_actions()
    }

    /// Return how many exact host-decision actions are active.
    pub fn host_decision_action_count(&self) -> usize {
        [
            self.host_decision_schedules_continuation(),
            self.host_decision_routes_follow_up(),
            self.host_decision_investigates_plan_drift(),
            self.host_decision_investigates_count_drift(),
            self.host_decision_investigates_run_integrity(),
        ]
        .into_iter()
        .filter(|needs_action| *needs_action)
        .count()
    }

    /// Return whether exactly one host-decision action is active.
    pub fn host_decision_has_single_action(&self) -> bool {
        self.host_decision_action_count() == 1
    }

    /// Classify the dashboard lane for the next host decision.
    pub fn host_decision_lane(&self) -> ToolAuditSupervisorDrainHostDecisionLane {
        self.host_decision_kind().lane()
    }

    /// Return the stable host-decision lane label.
    pub fn host_decision_lane_label(&self) -> &'static str {
        self.host_decision_lane().as_str()
    }

    /// Return whether the host-decision lane label parses back to the typed lane.
    pub fn host_decision_lane_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainHostDecisionLane::from_label(self.host_decision_lane_label())
            == Some(self.host_decision_lane())
    }

    /// Return whether the host decision belongs on the terminal dashboard lane.
    pub fn host_decision_uses_terminal_lane(&self) -> bool {
        self.host_decision_lane().is_terminal()
    }

    /// Return whether the host decision belongs on the scheduler dashboard lane.
    pub fn host_decision_uses_scheduler_lane(&self) -> bool {
        self.host_decision_lane().is_scheduler()
    }

    /// Return whether the host decision belongs on the follow-up dashboard lane.
    pub fn host_decision_uses_follow_up_lane(&self) -> bool {
        self.host_decision_lane().is_follow_up()
    }

    /// Return whether the host decision belongs on the investigation dashboard lane.
    pub fn host_decision_uses_investigation_lane(&self) -> bool {
        self.host_decision_lane().is_investigation()
    }

    /// Return whether the host decision spans multiple dashboard lanes.
    pub fn host_decision_uses_mixed_lane(&self) -> bool {
        self.host_decision_lane().is_mixed()
    }

    /// Classify the dashboard priority for the next host decision.
    pub fn host_decision_priority(&self) -> ToolAuditSupervisorDrainHostDecisionPriority {
        self.host_decision_kind().priority()
    }

    /// Return the stable host-decision priority label.
    pub fn host_decision_priority_label(&self) -> &'static str {
        self.host_decision_priority().as_str()
    }

    /// Return whether the host-decision priority label parses back to the typed priority.
    pub fn host_decision_priority_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainHostDecisionPriority::from_label(
            self.host_decision_priority_label(),
        ) == Some(self.host_decision_priority())
    }

    /// Return the sortable host-decision priority rank.
    pub fn host_decision_priority_rank(&self) -> u8 {
        self.host_decision_priority().rank()
    }

    /// Return whether the host decision has settled priority.
    pub fn host_decision_has_settled_priority(&self) -> bool {
        self.host_decision_priority().is_settled()
    }

    /// Return whether the host decision has routine-action priority.
    pub fn host_decision_has_routine_priority(&self) -> bool {
        self.host_decision_priority().is_routine_action()
    }

    /// Return whether the host decision has drift-investigation priority.
    pub fn host_decision_has_drift_investigation_priority(&self) -> bool {
        self.host_decision_priority().is_drift_investigation()
    }

    /// Return whether the host decision has integrity-investigation priority.
    pub fn host_decision_has_integrity_investigation_priority(&self) -> bool {
        self.host_decision_priority().is_integrity_investigation()
    }

    /// Return whether the host decision has mixed-action priority.
    pub fn host_decision_has_mixed_priority(&self) -> bool {
        self.host_decision_priority().is_mixed_action()
    }

    /// Return whether the host decision priority is investigation-grade.
    pub fn host_decision_has_investigation_priority(&self) -> bool {
        self.host_decision_priority().requires_investigation()
    }

    /// Classify whether the host decision can be queued, routed, or needs triage.
    pub fn host_decision_readiness(&self) -> ToolAuditSupervisorDrainHostDecisionReadiness {
        self.host_decision_kind().readiness()
    }

    /// Return the stable host-decision readiness label.
    pub fn host_decision_readiness_label(&self) -> &'static str {
        self.host_decision_readiness().as_str()
    }

    /// Return whether the host-decision readiness label parses back to its typed value.
    pub fn host_decision_readiness_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainHostDecisionReadiness::from_label(
            self.host_decision_readiness_label(),
        ) == Some(self.host_decision_readiness())
    }

    /// Return whether the host decision is settled and needs no queue entry.
    pub fn host_decision_is_settled(&self) -> bool {
        self.host_decision_readiness().is_settled()
    }

    /// Return whether the host decision should appear in an action queue.
    pub fn host_decision_is_actionable(&self) -> bool {
        self.host_decision_readiness().is_actionable()
    }

    /// Return whether the host decision can be routed without investigation triage.
    pub fn host_decision_is_auto_routable(&self) -> bool {
        self.host_decision_readiness().is_auto_routable()
    }

    /// Return whether the host decision needs manual review before routing.
    pub fn host_decision_readiness_requires_manual_review(&self) -> bool {
        self.host_decision_readiness().requires_manual_review()
    }

    /// Return whether the host decision readiness requires investigation.
    pub fn host_decision_readiness_requires_investigation(&self) -> bool {
        self.host_decision_readiness().requires_investigation()
    }

    /// Return whether the host decision readiness requires host-log integrity investigation.
    pub fn host_decision_readiness_requires_integrity_investigation(&self) -> bool {
        self.host_decision_readiness()
            .requires_integrity_investigation()
    }

    /// Return whether multiple decision surfaces need human triage.
    pub fn host_decision_needs_triage(&self) -> bool {
        self.host_decision_readiness().requires_triage()
    }

    /// Classify the concrete queue route for the host decision.
    pub fn host_decision_route(&self) -> ToolAuditSupervisorDrainHostDecisionRoute {
        self.host_decision_kind().route()
    }

    /// Return the stable host-decision route label.
    pub fn host_decision_route_label(&self) -> &'static str {
        self.host_decision_route().as_str()
    }

    /// Return whether the host-decision route label parses back to its typed route.
    pub fn host_decision_route_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainHostDecisionRoute::from_label(self.host_decision_route_label())
            == Some(self.host_decision_route())
    }

    /// Return whether the host decision has a concrete queue route.
    pub fn host_decision_has_route(&self) -> bool {
        self.host_decision_route().has_route()
    }

    /// Return whether the host decision routes to the scheduler queue.
    pub fn host_decision_routes_to_scheduler(&self) -> bool {
        self.host_decision_route().is_scheduler()
    }

    /// Return whether the host decision routes to the follow-up queue.
    pub fn host_decision_routes_to_follow_up(&self) -> bool {
        self.host_decision_route().is_follow_up()
    }

    /// Return whether the host decision routes to a drift-investigation queue.
    pub fn host_decision_routes_to_drift_investigation(&self) -> bool {
        self.host_decision_route().is_drift_investigation()
    }

    /// Return whether the host decision routes to a host-log integrity queue.
    pub fn host_decision_routes_to_integrity_investigation(&self) -> bool {
        self.host_decision_route().is_integrity_investigation()
    }

    /// Return whether the host decision routes to a triage queue.
    pub fn host_decision_routes_to_triage(&self) -> bool {
        self.host_decision_route().is_triage()
    }

    /// Return whether the host decision routes to any investigation queue.
    pub fn host_decision_routes_to_investigation(&self) -> bool {
        self.host_decision_route().is_investigation()
    }

    /// Return whether all host-decision classifier labels parse back to their typed values.
    pub fn host_decision_classifier_labels_match(&self) -> bool {
        self.host_decision_label_matches_kind()
            && self.host_decision_lane_label_matches_kind()
            && self.host_decision_priority_label_matches_kind()
            && self.host_decision_readiness_label_matches_kind()
            && self.host_decision_route_label_matches_kind()
    }

    /// Return whether any host-decision classifier label drifted from its typed value.
    pub fn has_host_decision_label_integrity_drift(&self) -> bool {
        !self.host_decision_classifier_labels_match()
    }

    /// Return whether all host-routing classifier labels parse back to their typed values.
    pub fn host_routing_classifier_labels_match(&self) -> bool {
        self.host_attention_label_matches_kind()
            && self.terminal_readiness_label_matches_kind()
            && self.host_decision_classifier_labels_match()
    }

    /// Return whether any host-routing classifier label drifted from its typed value.
    pub fn has_host_routing_label_integrity_drift(&self) -> bool {
        !self.host_routing_classifier_labels_match()
    }

    /// Return the reader or supervisor checkpoint name drained by this run.
    pub fn checkpoint_name(&self) -> &str {
        &self.plan.checkpoint_name
    }

    /// Return the checkpoint used to start the preflight plan.
    pub fn starting_checkpoint(&self) -> &ToolAuditReadCheckpoint {
        &self.plan.starting_checkpoint
    }

    /// Return the starting checkpoint timestamp for host run logs.
    pub fn starting_checkpoint_timestamp_ms(&self) -> u64 {
        self.starting_checkpoint().checkpoint_timestamp_ms()
    }

    /// Return the starting checkpoint call id for host run logs.
    pub fn starting_checkpoint_call_id(&self) -> &str {
        self.starting_checkpoint().checkpoint_call_id()
    }

    /// Return the durable checkpoint loaded before the run, if any.
    pub fn stored_checkpoint(&self) -> Option<&ToolAuditStoredCheckpoint> {
        self.plan.stored_checkpoint.as_ref()
    }

    /// Return whether a durable checkpoint existed before the run.
    pub fn has_stored_checkpoint(&self) -> bool {
        self.stored_checkpoint().is_some()
    }

    /// Return the durable checkpoint timestamp loaded before the run.
    pub fn stored_checkpoint_timestamp_ms(&self) -> Option<u64> {
        self.stored_checkpoint()
            .map(ToolAuditStoredCheckpoint::checkpoint_timestamp_ms)
    }

    /// Return the durable checkpoint call id loaded before the run.
    pub fn stored_checkpoint_call_id(&self) -> Option<&str> {
        self.stored_checkpoint()
            .map(ToolAuditStoredCheckpoint::checkpoint_call_id)
    }

    /// Return whether stored checkpoint presence agrees with flattened scalar fields.
    pub fn stored_checkpoint_presence_matches_fields(&self) -> bool {
        self.has_stored_checkpoint()
            == (self.stored_checkpoint_timestamp_ms().is_some()
                && self.stored_checkpoint_call_id().is_some())
    }

    /// Return whether the flattened stored checkpoint timestamp matches the object.
    pub fn stored_checkpoint_timestamp_matches_checkpoint(&self) -> bool {
        self.stored_checkpoint()
            .map(ToolAuditStoredCheckpoint::checkpoint_timestamp_ms)
            == self.stored_checkpoint_timestamp_ms()
    }

    /// Return whether the flattened stored checkpoint call id matches the object.
    pub fn stored_checkpoint_call_id_matches_checkpoint(&self) -> bool {
        self.stored_checkpoint()
            .map(ToolAuditStoredCheckpoint::checkpoint_call_id)
            == self.stored_checkpoint_call_id()
    }

    /// Return whether all flattened stored checkpoint fields agree with the object.
    pub fn stored_checkpoint_fields_match_checkpoint(&self) -> bool {
        self.stored_checkpoint_presence_matches_fields()
            && self.stored_checkpoint_timestamp_matches_checkpoint()
            && self.stored_checkpoint_call_id_matches_checkpoint()
    }

    /// Return whether the starting checkpoint matches the loaded durable checkpoint.
    pub fn starting_checkpoint_matches_stored_checkpoint(&self) -> bool {
        match self.stored_checkpoint() {
            Some(stored) => stored.checkpoint == *self.starting_checkpoint(),
            None => *self.starting_checkpoint() == ToolAuditReadCheckpoint::beginning(),
        }
    }

    /// Return the maximum rows each drain tick was allowed to replay.
    pub fn max_records_per_tick(&self) -> usize {
        self.plan.max_records_per_tick
    }

    /// Return the maximum drain ticks requested for this run.
    pub fn max_ticks(&self) -> usize {
        self.plan.max_ticks
    }

    /// Return the number of pages discovered by the preflight plan.
    pub fn planned_pages(&self) -> usize {
        self.plan.page_count()
    }

    /// Return the number of drain ticks that actually ran.
    pub fn drain_ticks(&self) -> usize {
        self.drain.tick_count()
    }

    /// Return how many requested drain ticks did not run.
    pub fn remaining_tick_budget(&self) -> usize {
        self.max_ticks().saturating_sub(self.drain_ticks())
    }

    /// Return whether the run consumed every requested drain tick.
    pub fn used_all_ticks(&self) -> bool {
        self.remaining_tick_budget() == 0
    }

    /// Return whether more ticks were available when the run stopped.
    pub fn has_remaining_tick_budget(&self) -> bool {
        self.remaining_tick_budget() > 0
    }

    /// Return the total rows the preflight plan expected to replay.
    pub fn planned_records(&self) -> usize {
        self.plan.planned_records
    }

    /// Return the total rows actually replayed into the sink.
    pub fn drained_records(&self) -> usize {
        self.drain.drained_records
    }

    /// Return planned rows with follow-up pressure.
    pub fn planned_follow_up_records(&self) -> usize {
        self.plan.follow_up_record_count()
    }

    /// Return replayed rows with follow-up pressure.
    pub fn drained_follow_up_records(&self) -> usize {
        self.drain.follow_up_record_count()
    }

    /// Return whether planned row counts agree with their inventories.
    pub fn plan_inventory_counts_match(&self) -> bool {
        self.plan.plan_inventory_counts_match()
    }

    /// Return whether replayed row counts agree with their inventories.
    pub fn drain_inventory_counts_match(&self) -> bool {
        self.drain.drain_inventory_counts_match()
    }

    /// Return whether every planned and replayed row count agrees with inventory.
    pub fn inventory_counts_match(&self) -> bool {
        self.plan_inventory_counts_match() && self.drain_inventory_counts_match()
    }

    /// Return whether remaining tick budget agrees with max ticks and drain ticks.
    pub fn remaining_tick_budget_matches_counts(&self) -> bool {
        self.remaining_tick_budget() == self.max_ticks().saturating_sub(self.drain_ticks())
    }

    /// Return whether flattened tick-budget status flags agree with remaining budget.
    pub fn tick_budget_status_flags_match(&self) -> bool {
        self.used_all_ticks() == (self.remaining_tick_budget() == 0)
            && self.has_remaining_tick_budget() == (self.remaining_tick_budget() > 0)
    }

    /// Return whether flattened drain status flags agree with source counts.
    pub fn drain_status_flags_match(&self) -> bool {
        self.is_idle() == (self.drained_records() == 0)
            && self.made_progress() == (self.drained_records() > 0)
            && self.should_continue() == !self.reached_end_of_log()
            && self.exhausted_tick_budget()
                == (self.drain_ticks() == self.max_ticks() && !self.reached_end_of_log())
            && self.advanced_checkpoint() == self.made_progress()
    }

    /// Return whether all flattened budget and status flags agree with sources.
    pub fn run_status_flags_match(&self) -> bool {
        self.remaining_tick_budget_matches_counts()
            && self.tick_budget_status_flags_match()
            && self.drain_status_flags_match()
    }

    /// Return a payload-free, flattened summary for host logs and schedulers.
    pub fn summary(&self) -> ToolAuditSupervisorDrainRunSummary {
        ToolAuditSupervisorDrainRunSummary {
            checkpoint_name: self.checkpoint_name().to_owned(),
            starting_checkpoint: self.starting_checkpoint().clone(),
            starting_checkpoint_timestamp_ms: self.starting_checkpoint_timestamp_ms(),
            starting_checkpoint_call_id: self.starting_checkpoint_call_id().to_owned(),
            stored_checkpoint: self.stored_checkpoint().cloned(),
            has_stored_checkpoint: self.has_stored_checkpoint(),
            stored_checkpoint_timestamp_ms: self.stored_checkpoint_timestamp_ms(),
            stored_checkpoint_call_id: self.stored_checkpoint_call_id().map(str::to_owned),
            stored_checkpoint_presence_matches_fields: self
                .stored_checkpoint_presence_matches_fields(),
            stored_checkpoint_timestamp_matches_checkpoint: self
                .stored_checkpoint_timestamp_matches_checkpoint(),
            stored_checkpoint_call_id_matches_checkpoint: self
                .stored_checkpoint_call_id_matches_checkpoint(),
            stored_checkpoint_fields_match_checkpoint: self
                .stored_checkpoint_fields_match_checkpoint(),
            starting_checkpoint_matches_stored_checkpoint: self
                .starting_checkpoint_matches_stored_checkpoint(),
            outcome: self.outcome(),
            outcome_label: self.outcome_label(),
            outcome_label_matches_outcome: self.outcome_label_matches_outcome(),
            outcome_is_idle: self.outcome_is_idle(),
            is_caught_up: self.is_caught_up(),
            needs_continuation: self.needs_continuation(),
            needs_follow_up: self.needs_follow_up(),
            plan_diverged: self.plan_diverged(),
            scheduler_action: self.scheduler_action(),
            scheduler_action_label: self.scheduler_action_label(),
            scheduler_action_label_matches_action: self.scheduler_action_label_matches_action(),
            is_no_scheduler_action: self.scheduler_action().is_no_action(),
            requires_scheduler_action: self.requires_scheduler_action(),
            requests_continuation: self.requests_continuation(),
            routes_follow_up: self.routes_follow_up(),
            max_records_per_tick: self.max_records_per_tick(),
            max_ticks: self.max_ticks(),
            planned_pages: self.planned_pages(),
            drain_ticks: self.drain_ticks(),
            remaining_tick_budget: self.remaining_tick_budget(),
            used_all_ticks: self.used_all_ticks(),
            has_remaining_tick_budget: self.has_remaining_tick_budget(),
            planned_records: self.planned_records(),
            drained_records: self.drained_records(),
            planned_follow_up_records: self.planned_follow_up_records(),
            drained_follow_up_records: self.drained_follow_up_records(),
            record_count_delta: self.record_count_delta(),
            replayed_extra_records: self.replayed_extra_records(),
            missed_planned_records: self.missed_planned_records(),
            follow_up_record_count_delta: self.follow_up_record_count_delta(),
            replayed_extra_follow_up_records: self.replayed_extra_follow_up_records(),
            missed_planned_follow_up_records: self.missed_planned_follow_up_records(),
            has_record_count_drift: self.has_record_count_drift(),
            has_follow_up_record_count_drift: self.has_follow_up_record_count_drift(),
            has_count_drift: self.has_count_drift(),
            count_drift_kind: self.count_drift_kind(),
            count_drift_label: self.count_drift_label(),
            count_drift_label_matches_kind: self.count_drift_label_matches_kind(),
            is_no_count_drift: self.is_no_count_drift(),
            has_planned_last_checkpoint_drift: self.has_planned_last_checkpoint_drift(),
            has_checkpoint_advance_drift: self.has_checkpoint_advance_drift(),
            has_checkpoint_plan_drift: self.has_checkpoint_plan_drift(),
            checkpoint_drift_kind: self.checkpoint_drift_kind(),
            checkpoint_drift_label: self.checkpoint_drift_label(),
            checkpoint_drift_label_matches_kind: self.checkpoint_drift_label_matches_kind(),
            is_no_checkpoint_drift: self.is_no_checkpoint_drift(),
            requires_checkpoint_drift_investigation: self.requires_checkpoint_drift_investigation(),
            requires_plan_drift_investigation: self.requires_plan_drift_investigation(),
            requires_count_drift_investigation: self.requires_count_drift_investigation(),
            host_investigation_kind: self.host_investigation_kind(),
            host_investigation_label: self.host_investigation_label(),
            host_investigation_label_matches_kind: self.host_investigation_label_matches_kind(),
            all_classifier_labels_match: self.all_classifier_labels_match(),
            has_inventory_count_integrity_drift: self.has_inventory_count_integrity_drift(),
            has_run_status_integrity_drift: self.has_run_status_integrity_drift(),
            has_checkpoint_boundary_integrity_drift: self.has_checkpoint_boundary_integrity_drift(),
            has_classifier_label_integrity_drift: self.has_classifier_label_integrity_drift(),
            has_run_integrity_drift: self.has_run_integrity_drift(),
            run_integrity_kind: self.run_integrity_kind(),
            run_integrity_label: self.run_integrity_label(),
            run_integrity_label_matches_kind: self.run_integrity_label_matches_kind(),
            is_run_integrity_clean: self.is_run_integrity_clean(),
            requires_run_integrity_investigation: self.requires_run_integrity_investigation(),
            requires_host_investigation: self.requires_host_investigation(),
            is_no_host_investigation: self.is_no_host_investigation(),
            requires_host_plan_drift_investigation: self.requires_host_plan_drift_investigation(),
            requires_host_count_drift_investigation: self.requires_host_count_drift_investigation(),
            requires_host_attention: self.requires_host_attention(),
            is_no_host_attention: self.is_no_host_attention(),
            host_attention_kind: self.host_attention_kind(),
            host_attention_label: self.host_attention_label(),
            host_attention_label_matches_kind: self.host_attention_label_matches_kind(),
            host_attention_includes_scheduler_action: self
                .host_attention_includes_scheduler_action(),
            host_attention_includes_host_investigation: self
                .host_attention_includes_host_investigation(),
            host_attention_includes_run_integrity_investigation: self
                .host_attention_includes_run_integrity_investigation(),
            has_multiple_host_attention: self.has_multiple_host_attention(),
            is_terminal_ready: self.is_terminal_ready(),
            terminal_readiness_kind: self.terminal_readiness_kind(),
            terminal_readiness_label: self.terminal_readiness_label(),
            terminal_readiness_label_matches_kind: self.terminal_readiness_label_matches_kind(),
            terminal_readiness_requires_scheduler_action: self
                .terminal_readiness_requires_scheduler_action(),
            terminal_readiness_requires_host_investigation: self
                .terminal_readiness_requires_host_investigation(),
            terminal_readiness_requires_run_integrity_investigation: self
                .terminal_readiness_requires_run_integrity_investigation(),
            terminal_readiness_has_multiple_attention: self
                .terminal_readiness_has_multiple_attention(),
            terminal_readiness_is_idle: self.terminal_readiness_is_idle(),
            terminal_readiness_is_caught_up: self.terminal_readiness_is_caught_up(),
            requires_host_decision: self.requires_host_decision(),
            host_decision_kind: self.host_decision_kind(),
            host_decision_label: self.host_decision_label(),
            host_decision_label_matches_kind: self.host_decision_label_matches_kind(),
            host_decision_is_terminal_ready: self.host_decision_is_terminal_ready(),
            host_decision_schedules_continuation: self.host_decision_schedules_continuation(),
            host_decision_routes_follow_up: self.host_decision_routes_follow_up(),
            host_decision_investigates_plan_drift: self.host_decision_investigates_plan_drift(),
            host_decision_investigates_count_drift: self.host_decision_investigates_count_drift(),
            host_decision_investigates_run_integrity: self
                .host_decision_investigates_run_integrity(),
            host_decision_has_multiple_actions: self.host_decision_has_multiple_actions(),
            host_decision_action_count: self.host_decision_action_count(),
            host_decision_has_single_action: self.host_decision_has_single_action(),
            host_decision_lane: self.host_decision_lane(),
            host_decision_lane_label: self.host_decision_lane_label(),
            host_decision_lane_label_matches_kind: self.host_decision_lane_label_matches_kind(),
            host_decision_uses_terminal_lane: self.host_decision_uses_terminal_lane(),
            host_decision_uses_scheduler_lane: self.host_decision_uses_scheduler_lane(),
            host_decision_uses_follow_up_lane: self.host_decision_uses_follow_up_lane(),
            host_decision_uses_investigation_lane: self.host_decision_uses_investigation_lane(),
            host_decision_uses_mixed_lane: self.host_decision_uses_mixed_lane(),
            host_decision_priority: self.host_decision_priority(),
            host_decision_priority_label: self.host_decision_priority_label(),
            host_decision_priority_label_matches_kind: self
                .host_decision_priority_label_matches_kind(),
            host_decision_priority_rank: self.host_decision_priority_rank(),
            host_decision_has_settled_priority: self.host_decision_has_settled_priority(),
            host_decision_has_routine_priority: self.host_decision_has_routine_priority(),
            host_decision_has_drift_investigation_priority: self
                .host_decision_has_drift_investigation_priority(),
            host_decision_has_integrity_investigation_priority: self
                .host_decision_has_integrity_investigation_priority(),
            host_decision_has_mixed_priority: self.host_decision_has_mixed_priority(),
            host_decision_has_investigation_priority: self
                .host_decision_has_investigation_priority(),
            host_decision_readiness: self.host_decision_readiness(),
            host_decision_readiness_label: self.host_decision_readiness_label(),
            host_decision_readiness_label_matches_kind: self
                .host_decision_readiness_label_matches_kind(),
            host_decision_is_settled: self.host_decision_is_settled(),
            host_decision_is_actionable: self.host_decision_is_actionable(),
            host_decision_is_auto_routable: self.host_decision_is_auto_routable(),
            host_decision_readiness_requires_manual_review: self
                .host_decision_readiness_requires_manual_review(),
            host_decision_readiness_requires_investigation: self
                .host_decision_readiness_requires_investigation(),
            host_decision_readiness_requires_integrity_investigation: self
                .host_decision_readiness_requires_integrity_investigation(),
            host_decision_needs_triage: self.host_decision_needs_triage(),
            host_decision_route: self.host_decision_route(),
            host_decision_route_label: self.host_decision_route_label(),
            host_decision_route_label_matches_kind: self.host_decision_route_label_matches_kind(),
            host_decision_has_route: self.host_decision_has_route(),
            host_decision_routes_to_scheduler: self.host_decision_routes_to_scheduler(),
            host_decision_routes_to_follow_up: self.host_decision_routes_to_follow_up(),
            host_decision_routes_to_drift_investigation: self
                .host_decision_routes_to_drift_investigation(),
            host_decision_routes_to_integrity_investigation: self
                .host_decision_routes_to_integrity_investigation(),
            host_decision_routes_to_triage: self.host_decision_routes_to_triage(),
            host_decision_routes_to_investigation: self.host_decision_routes_to_investigation(),
            host_decision_classifier_labels_match: self.host_decision_classifier_labels_match(),
            has_host_decision_label_integrity_drift: self.has_host_decision_label_integrity_drift(),
            host_routing_classifier_labels_match: self.host_routing_classifier_labels_match(),
            has_host_routing_label_integrity_drift: self.has_host_routing_label_integrity_drift(),
            matches_planned_record_count: self.matches_planned_record_count(),
            matches_record_count: self.matches_record_count(),
            matches_planned_follow_up_record_count: self.matches_planned_follow_up_record_count(),
            matches_follow_up_pressure: self.matches_follow_up_pressure(),
            plan_inventory_counts_match: self.plan_inventory_counts_match(),
            drain_inventory_counts_match: self.drain_inventory_counts_match(),
            inventory_counts_match: self.inventory_counts_match(),
            remaining_tick_budget_matches_counts: self.remaining_tick_budget_matches_counts(),
            tick_budget_status_flags_match: self.tick_budget_status_flags_match(),
            drain_status_flags_match: self.drain_status_flags_match(),
            run_status_flags_match: self.run_status_flags_match(),
            reached_end_of_log: self.reached_end_of_log(),
            exhausted_tick_budget: self.exhausted_tick_budget(),
            is_idle: self.is_idle(),
            made_progress: self.made_progress(),
            should_continue: self.should_continue(),
            requires_follow_up: self.requires_follow_up(),
            advanced_checkpoint: self.advanced_checkpoint(),
            planned_last_checkpoint: self.planned_last_checkpoint().cloned(),
            has_planned_last_checkpoint: self.has_planned_last_checkpoint(),
            planned_last_checkpoint_timestamp_ms: self.planned_last_checkpoint_timestamp_ms(),
            planned_last_checkpoint_call_id: self
                .planned_last_checkpoint_call_id()
                .map(str::to_owned),
            planned_last_checkpoint_presence_matches_fields: self
                .planned_last_checkpoint_presence_matches_fields(),
            planned_last_checkpoint_timestamp_matches_checkpoint: self
                .planned_last_checkpoint_timestamp_matches_checkpoint(),
            planned_last_checkpoint_call_id_matches_checkpoint: self
                .planned_last_checkpoint_call_id_matches_checkpoint(),
            planned_last_checkpoint_fields_match_checkpoint: self
                .planned_last_checkpoint_fields_match_checkpoint(),
            planned_last_checkpoint_advanced_from_starting: self
                .planned_last_checkpoint_advanced_from_starting(),
            planned_last_checkpoint_matches_starting_checkpoint: self
                .planned_last_checkpoint_matches_starting_checkpoint(),
            last_checkpoint: self.last_checkpoint().cloned(),
            has_last_checkpoint: self.has_last_checkpoint(),
            last_checkpoint_timestamp_ms: self.last_checkpoint_timestamp_ms(),
            last_checkpoint_call_id: self.last_checkpoint_call_id().map(str::to_owned),
            last_checkpoint_presence_matches_fields: self.last_checkpoint_presence_matches_fields(),
            last_checkpoint_timestamp_matches_checkpoint: self
                .last_checkpoint_timestamp_matches_checkpoint(),
            last_checkpoint_call_id_matches_checkpoint: self
                .last_checkpoint_call_id_matches_checkpoint(),
            last_checkpoint_fields_match_checkpoint: self.last_checkpoint_fields_match_checkpoint(),
            last_checkpoint_advanced_from_starting: self.last_checkpoint_advanced_from_starting(),
            last_checkpoint_matches_starting_checkpoint: self
                .last_checkpoint_matches_starting_checkpoint(),
            checkpoint_advance_matches_last_checkpoint: self
                .checkpoint_advance_matches_last_checkpoint(),
            planned_last_checkpoint_matches_last_checkpoint: self
                .planned_last_checkpoint_matches_last_checkpoint(),
            planned_checkpoint_advance_matches_drain: self
                .planned_checkpoint_advance_matches_drain(),
            checkpoint_plan_matches_drain: self.checkpoint_plan_matches_drain(),
            checkpoint_boundary_fields_match: self.checkpoint_boundary_fields_match(),
        }
    }

    /// Return whether the actual run delivered the planned number of rows.
    pub fn matches_planned_record_count(&self) -> bool {
        self.plan.planned_records == self.drain.drained_records
    }

    /// Return whether planned and replayed row counts match.
    pub fn matches_record_count(&self) -> bool {
        self.matches_planned_record_count()
    }

    /// Return whether the actual run preserved the planned follow-up pressure count.
    pub fn matches_planned_follow_up_record_count(&self) -> bool {
        self.plan.follow_up_record_count() == self.drain.follow_up_record_count()
    }

    /// Return whether planned and replayed follow-up pressure counts match.
    pub fn matches_follow_up_pressure(&self) -> bool {
        self.matches_planned_follow_up_record_count()
    }

    /// Return the replayed-minus-planned row count delta.
    pub fn record_count_delta(&self) -> i128 {
        signed_count_delta(self.drain.drained_records, self.plan.planned_records)
    }

    /// Return the replayed-minus-planned follow-up pressure count delta.
    pub fn follow_up_record_count_delta(&self) -> i128 {
        signed_count_delta(
            self.drain.follow_up_record_count(),
            self.plan.follow_up_record_count(),
        )
    }

    /// Return whether the actual run replayed more rows than planned.
    pub fn replayed_extra_records(&self) -> bool {
        self.record_count_delta() > 0
    }

    /// Return whether the actual run replayed fewer rows than planned.
    pub fn missed_planned_records(&self) -> bool {
        self.record_count_delta() < 0
    }

    /// Return whether the actual run replayed more follow-up pressure than planned.
    pub fn replayed_extra_follow_up_records(&self) -> bool {
        self.follow_up_record_count_delta() > 0
    }

    /// Return whether the actual run replayed less follow-up pressure than planned.
    pub fn missed_planned_follow_up_records(&self) -> bool {
        self.follow_up_record_count_delta() < 0
    }

    /// Return whether row count drift was observed.
    pub fn has_record_count_drift(&self) -> bool {
        self.record_count_delta() != 0
    }

    /// Return whether follow-up pressure count drift was observed.
    pub fn has_follow_up_record_count_drift(&self) -> bool {
        self.follow_up_record_count_delta() != 0
    }

    /// Return whether any planned-vs-actual count drift was observed.
    pub fn has_count_drift(&self) -> bool {
        self.has_record_count_drift() || self.has_follow_up_record_count_drift()
    }

    /// Return whether planned and replayed counts stayed aligned.
    pub fn is_no_count_drift(&self) -> bool {
        self.count_drift_kind().is_no_drift()
    }

    /// Classify the observed planned-vs-actual count drift.
    pub fn count_drift_kind(&self) -> ToolAuditSupervisorDrainCountDriftKind {
        ToolAuditSupervisorDrainCountDriftKind::from_drift_flags(
            self.has_record_count_drift(),
            self.has_follow_up_record_count_drift(),
        )
    }

    /// Return the stable count-drift classification label.
    pub fn count_drift_label(&self) -> &'static str {
        self.count_drift_kind().as_str()
    }

    /// Return whether the count-drift label parses back to the typed classification.
    pub fn count_drift_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainCountDriftKind::from_label(self.count_drift_label())
            == Some(self.count_drift_kind())
    }

    /// Return whether count drift should be investigated by the host.
    pub fn requires_count_drift_investigation(&self) -> bool {
        self.count_drift_kind().requires_investigation()
    }

    /// Return whether planned and actual final checkpoints diverged.
    pub fn has_planned_last_checkpoint_drift(&self) -> bool {
        !self.planned_last_checkpoint_matches_last_checkpoint()
    }

    /// Return whether planned checkpoint movement diverged from the actual drain.
    pub fn has_checkpoint_advance_drift(&self) -> bool {
        !self.planned_checkpoint_advance_matches_drain()
    }

    /// Return whether any planned-vs-actual checkpoint drift was observed.
    pub fn has_checkpoint_plan_drift(&self) -> bool {
        self.has_planned_last_checkpoint_drift() || self.has_checkpoint_advance_drift()
    }

    /// Return whether planned and actual checkpoint boundaries stayed aligned.
    pub fn is_no_checkpoint_drift(&self) -> bool {
        self.checkpoint_drift_kind().is_no_drift()
    }

    /// Classify the observed planned-vs-actual checkpoint drift.
    pub fn checkpoint_drift_kind(&self) -> ToolAuditSupervisorDrainCheckpointDriftKind {
        ToolAuditSupervisorDrainCheckpointDriftKind::from_drift_flags(
            self.has_planned_last_checkpoint_drift(),
            self.has_checkpoint_advance_drift(),
        )
    }

    /// Return the stable checkpoint-drift classification label.
    pub fn checkpoint_drift_label(&self) -> &'static str {
        self.checkpoint_drift_kind().as_str()
    }

    /// Return whether the checkpoint-drift label parses back to the typed classification.
    pub fn checkpoint_drift_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainCheckpointDriftKind::from_label(self.checkpoint_drift_label())
            == Some(self.checkpoint_drift_kind())
    }

    /// Return whether checkpoint drift should be investigated by the host.
    pub fn requires_checkpoint_drift_investigation(&self) -> bool {
        self.checkpoint_drift_kind().requires_investigation()
    }

    /// Return whether no audit rows were replayed.
    pub fn is_idle(&self) -> bool {
        self.drain.is_idle()
    }

    /// Return whether the actual run replayed at least one row.
    pub fn made_progress(&self) -> bool {
        self.drain.made_progress()
    }

    /// Return whether the actual run reached the current end of the audit log.
    pub fn reached_end_of_log(&self) -> bool {
        self.drain.reached_end_of_log()
    }

    /// Return whether the actual run used every allowed tick.
    pub fn exhausted_tick_budget(&self) -> bool {
        self.drain.exhausted_tick_budget()
    }

    /// Return whether the supervisor should schedule another drain run.
    pub fn should_continue(&self) -> bool {
        self.drain.should_continue()
    }

    /// Return whether planned or replayed rows need follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.plan.requires_follow_up() || self.drain.requires_follow_up()
    }

    /// Return whether any durable checkpoint advanced during the run.
    pub fn advanced_checkpoint(&self) -> bool {
        self.drain.advanced_checkpoint()
    }

    /// Return the last replay checkpoint the preflight plan expected to observe.
    pub fn planned_last_checkpoint(&self) -> Option<&ToolAuditReadCheckpoint> {
        self.plan.last_checkpoint()
    }

    /// Return whether the preflight plan observed a final replay checkpoint.
    pub fn has_planned_last_checkpoint(&self) -> bool {
        self.planned_last_checkpoint().is_some()
    }

    /// Return the timestamp for the final replay checkpoint planned before the run.
    pub fn planned_last_checkpoint_timestamp_ms(&self) -> Option<u64> {
        self.planned_last_checkpoint()
            .map(|checkpoint| checkpoint.timestamp_ms)
    }

    /// Return the call id for the final replay checkpoint planned before the run.
    pub fn planned_last_checkpoint_call_id(&self) -> Option<&str> {
        self.planned_last_checkpoint()
            .map(|checkpoint| checkpoint.call_id.as_str())
    }

    /// Return whether planned checkpoint presence agrees with flattened scalar fields.
    pub fn planned_last_checkpoint_presence_matches_fields(&self) -> bool {
        self.has_planned_last_checkpoint()
            == (self.planned_last_checkpoint_timestamp_ms().is_some()
                && self.planned_last_checkpoint_call_id().is_some())
    }

    /// Return whether the flattened planned timestamp matches the checkpoint object.
    pub fn planned_last_checkpoint_timestamp_matches_checkpoint(&self) -> bool {
        self.planned_last_checkpoint()
            .map(|checkpoint| checkpoint.timestamp_ms)
            == self.planned_last_checkpoint_timestamp_ms()
    }

    /// Return whether the flattened planned call id matches the checkpoint object.
    pub fn planned_last_checkpoint_call_id_matches_checkpoint(&self) -> bool {
        self.planned_last_checkpoint()
            .map(|checkpoint| checkpoint.call_id.as_str())
            == self.planned_last_checkpoint_call_id()
    }

    /// Return whether all flattened planned checkpoint fields agree with the object.
    pub fn planned_last_checkpoint_fields_match_checkpoint(&self) -> bool {
        self.planned_last_checkpoint_presence_matches_fields()
            && self.planned_last_checkpoint_timestamp_matches_checkpoint()
            && self.planned_last_checkpoint_call_id_matches_checkpoint()
    }

    /// Return whether the planned last checkpoint is after the starting checkpoint.
    pub fn planned_last_checkpoint_advanced_from_starting(&self) -> bool {
        self.planned_last_checkpoint()
            .map(|checkpoint| checkpoint.is_after(self.starting_checkpoint()))
            .unwrap_or(false)
    }

    /// Return whether the planned last checkpoint still matches the starting checkpoint.
    pub fn planned_last_checkpoint_matches_starting_checkpoint(&self) -> bool {
        self.planned_last_checkpoint() == Some(self.starting_checkpoint())
    }

    /// Return the last replay checkpoint observed by the actual run.
    pub fn last_checkpoint(&self) -> Option<&ToolAuditReadCheckpoint> {
        self.drain.last_checkpoint()
    }

    /// Return whether the actual run observed a replay checkpoint.
    pub fn has_last_checkpoint(&self) -> bool {
        self.last_checkpoint().is_some()
    }

    /// Return the timestamp for the last replay checkpoint observed by the actual run.
    pub fn last_checkpoint_timestamp_ms(&self) -> Option<u64> {
        self.last_checkpoint()
            .map(|checkpoint| checkpoint.timestamp_ms)
    }

    /// Return the call id for the last replay checkpoint observed by the actual run.
    pub fn last_checkpoint_call_id(&self) -> Option<&str> {
        self.last_checkpoint()
            .map(|checkpoint| checkpoint.call_id.as_str())
    }

    /// Return whether checkpoint presence agrees with flattened scalar fields.
    pub fn last_checkpoint_presence_matches_fields(&self) -> bool {
        self.has_last_checkpoint()
            == (self.last_checkpoint_timestamp_ms().is_some()
                && self.last_checkpoint_call_id().is_some())
    }

    /// Return whether the flattened checkpoint timestamp matches the checkpoint object.
    pub fn last_checkpoint_timestamp_matches_checkpoint(&self) -> bool {
        self.last_checkpoint()
            .map(|checkpoint| checkpoint.timestamp_ms)
            == self.last_checkpoint_timestamp_ms()
    }

    /// Return whether the flattened checkpoint call id matches the checkpoint object.
    pub fn last_checkpoint_call_id_matches_checkpoint(&self) -> bool {
        self.last_checkpoint()
            .map(|checkpoint| checkpoint.call_id.as_str())
            == self.last_checkpoint_call_id()
    }

    /// Return whether all flattened checkpoint fields agree with the checkpoint object.
    pub fn last_checkpoint_fields_match_checkpoint(&self) -> bool {
        self.last_checkpoint_presence_matches_fields()
            && self.last_checkpoint_timestamp_matches_checkpoint()
            && self.last_checkpoint_call_id_matches_checkpoint()
    }

    /// Return whether the last replay checkpoint is after the starting checkpoint.
    pub fn last_checkpoint_advanced_from_starting(&self) -> bool {
        self.last_checkpoint()
            .map(|checkpoint| checkpoint.is_after(self.starting_checkpoint()))
            .unwrap_or(false)
    }

    /// Return whether the last replay checkpoint still matches the starting checkpoint.
    pub fn last_checkpoint_matches_starting_checkpoint(&self) -> bool {
        self.last_checkpoint() == Some(self.starting_checkpoint())
    }

    /// Return whether the durable-advance flag agrees with last-checkpoint movement.
    pub fn checkpoint_advance_matches_last_checkpoint(&self) -> bool {
        self.advanced_checkpoint() == self.last_checkpoint_advanced_from_starting()
    }

    /// Return whether planned and actual final replay checkpoints match.
    pub fn planned_last_checkpoint_matches_last_checkpoint(&self) -> bool {
        self.planned_last_checkpoint() == self.last_checkpoint()
    }

    /// Return whether planned checkpoint movement agrees with the actual drain.
    pub fn planned_checkpoint_advance_matches_drain(&self) -> bool {
        self.plan.would_advance_checkpoint() == self.advanced_checkpoint()
    }

    /// Return whether the planned checkpoint boundary agrees with the actual drain.
    pub fn checkpoint_plan_matches_drain(&self) -> bool {
        self.planned_last_checkpoint_fields_match_checkpoint()
            && self.last_checkpoint_fields_match_checkpoint()
            && self.planned_last_checkpoint_matches_last_checkpoint()
            && self.planned_checkpoint_advance_matches_drain()
    }

    /// Return whether checkpoint-boundary fields agree with their source checkpoints.
    pub fn checkpoint_boundary_fields_match(&self) -> bool {
        self.stored_checkpoint_fields_match_checkpoint()
            && self.starting_checkpoint_matches_stored_checkpoint()
            && self.last_checkpoint_fields_match_checkpoint()
            && self.checkpoint_advance_matches_last_checkpoint()
            && self.checkpoint_plan_matches_drain()
    }

    /// Return whether planned or replayed inventory counts drifted from payloads.
    pub fn has_inventory_count_integrity_drift(&self) -> bool {
        !self.inventory_counts_match()
    }

    /// Return whether flattened run-status flags drifted from source counts.
    pub fn has_run_status_integrity_drift(&self) -> bool {
        !self.run_status_flags_match()
    }

    /// Return whether checkpoint-boundary fields drifted from source checkpoints.
    pub fn has_checkpoint_boundary_integrity_drift(&self) -> bool {
        !self.checkpoint_boundary_fields_match()
    }

    /// Return whether any stable classifier label drifted from its typed value.
    pub fn has_classifier_label_integrity_drift(&self) -> bool {
        !self.all_classifier_labels_match()
    }

    /// Return whether any flattened host-log integrity drift was observed.
    pub fn has_run_integrity_drift(&self) -> bool {
        self.has_inventory_count_integrity_drift()
            || self.has_run_status_integrity_drift()
            || self.has_checkpoint_boundary_integrity_drift()
            || self.has_classifier_label_integrity_drift()
    }

    /// Return whether all flattened host-log integrity checks are clean.
    pub fn is_run_integrity_clean(&self) -> bool {
        self.run_integrity_kind().is_no_drift()
    }

    /// Classify flattened host-log integrity drift.
    pub fn run_integrity_kind(&self) -> ToolAuditSupervisorDrainRunIntegrityKind {
        ToolAuditSupervisorDrainRunIntegrityKind::from_integrity_flags(
            self.has_inventory_count_integrity_drift(),
            self.has_run_status_integrity_drift(),
            self.has_checkpoint_boundary_integrity_drift(),
            self.has_classifier_label_integrity_drift(),
        )
    }

    /// Return the stable run-integrity classification label.
    pub fn run_integrity_label(&self) -> &'static str {
        self.run_integrity_kind().as_str()
    }

    /// Return whether the run-integrity label parses back to the typed classification.
    pub fn run_integrity_label_matches_kind(&self) -> bool {
        ToolAuditSupervisorDrainRunIntegrityKind::from_label(self.run_integrity_label())
            == Some(self.run_integrity_kind())
    }

    /// Return whether host-log integrity drift should be investigated.
    pub fn requires_run_integrity_investigation(&self) -> bool {
        self.run_integrity_kind().requires_investigation()
    }
}

/// Payload-free, flattened host summary for a bounded supervisor drain run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSupervisorDrainRunSummary {
    /// Reader or supervisor checkpoint name drained.
    pub checkpoint_name: String,
    /// Checkpoint used to start the preflight plan.
    pub starting_checkpoint: ToolAuditReadCheckpoint,
    /// Starting checkpoint timestamp for host run logs.
    pub starting_checkpoint_timestamp_ms: u64,
    /// Starting checkpoint call id for host run logs.
    pub starting_checkpoint_call_id: String,
    /// Durable checkpoint loaded before the run, if any.
    pub stored_checkpoint: Option<ToolAuditStoredCheckpoint>,
    /// Whether a durable checkpoint existed before the run.
    pub has_stored_checkpoint: bool,
    /// Durable checkpoint timestamp loaded before the run.
    pub stored_checkpoint_timestamp_ms: Option<u64>,
    /// Durable checkpoint call id loaded before the run.
    pub stored_checkpoint_call_id: Option<String>,
    /// Whether stored checkpoint presence agrees with flattened scalar fields.
    pub stored_checkpoint_presence_matches_fields: bool,
    /// Whether the flattened stored checkpoint timestamp matches the object.
    pub stored_checkpoint_timestamp_matches_checkpoint: bool,
    /// Whether the flattened stored checkpoint call id matches the object.
    pub stored_checkpoint_call_id_matches_checkpoint: bool,
    /// Whether all flattened stored checkpoint fields agree with the object.
    pub stored_checkpoint_fields_match_checkpoint: bool,
    /// Whether the starting checkpoint matches the loaded durable checkpoint.
    pub starting_checkpoint_matches_stored_checkpoint: bool,
    /// Scheduler-facing classification for this run.
    pub outcome: ToolAuditSupervisorDrainRunOutcome,
    /// Stable scheduler-facing outcome label for host logs.
    pub outcome_label: &'static str,
    /// Whether the outcome label parses back to the typed outcome.
    pub outcome_label_matches_outcome: bool,
    /// Whether the scheduler-facing outcome is idle.
    pub outcome_is_idle: bool,
    /// Whether rows were replayed and this run reached the current log end.
    pub is_caught_up: bool,
    /// Whether this run needs another drain pass.
    pub needs_continuation: bool,
    /// Whether this run outcome routes follow-up pressure to the host.
    pub needs_follow_up: bool,
    /// Whether this run diverged from its preflight plan.
    pub plan_diverged: bool,
    /// Recommended host scheduler action for this run.
    pub scheduler_action: ToolAuditSupervisorDrainSchedulerAction,
    /// Stable scheduler action label for host logs.
    pub scheduler_action_label: &'static str,
    /// Whether the scheduler-action label parses back to the typed action.
    pub scheduler_action_label_matches_action: bool,
    /// Whether this run intentionally leaves the scheduler idle.
    pub is_no_scheduler_action: bool,
    /// Whether this run asks the scheduler to take action.
    pub requires_scheduler_action: bool,
    /// Whether the scheduler should run another drain pass.
    pub requests_continuation: bool,
    /// Whether follow-up pressure should be routed to the host.
    pub routes_follow_up: bool,
    /// Maximum rows each drain tick may replay.
    pub max_records_per_tick: usize,
    /// Maximum drain ticks requested for this run.
    pub max_ticks: usize,
    /// Number of pages discovered by the preflight plan.
    pub planned_pages: usize,
    /// Number of drain ticks that actually ran.
    pub drain_ticks: usize,
    /// Requested drain ticks that did not run.
    pub remaining_tick_budget: usize,
    /// Whether the run consumed every requested drain tick.
    pub used_all_ticks: bool,
    /// Whether more ticks were available when the run stopped.
    pub has_remaining_tick_budget: bool,
    /// Total rows the preflight plan expected to replay.
    pub planned_records: usize,
    /// Total rows actually replayed into the sink.
    pub drained_records: usize,
    /// Planned rows with follow-up pressure.
    pub planned_follow_up_records: usize,
    /// Replayed rows with follow-up pressure.
    pub drained_follow_up_records: usize,
    /// Replayed-minus-planned row count delta.
    pub record_count_delta: i128,
    /// Whether more rows replayed than the preflight plan expected.
    pub replayed_extra_records: bool,
    /// Whether fewer rows replayed than the preflight plan expected.
    pub missed_planned_records: bool,
    /// Replayed-minus-planned follow-up pressure count delta.
    pub follow_up_record_count_delta: i128,
    /// Whether more follow-up pressure rows replayed than the preflight plan expected.
    pub replayed_extra_follow_up_records: bool,
    /// Whether fewer follow-up pressure rows replayed than the preflight plan expected.
    pub missed_planned_follow_up_records: bool,
    /// Whether row count drift was observed.
    pub has_record_count_drift: bool,
    /// Whether follow-up pressure count drift was observed.
    pub has_follow_up_record_count_drift: bool,
    /// Whether any planned-vs-actual count drift was observed.
    pub has_count_drift: bool,
    /// Stable classification of observed planned-vs-actual count drift.
    pub count_drift_kind: ToolAuditSupervisorDrainCountDriftKind,
    /// Stable count-drift classification label for host logs.
    pub count_drift_label: &'static str,
    /// Whether the count-drift label parses back to the typed classification.
    pub count_drift_label_matches_kind: bool,
    /// Whether planned and replayed counts stayed aligned.
    pub is_no_count_drift: bool,
    /// Whether planned and actual final checkpoints diverged.
    pub has_planned_last_checkpoint_drift: bool,
    /// Whether planned checkpoint movement diverged from the actual drain.
    pub has_checkpoint_advance_drift: bool,
    /// Whether any planned-vs-actual checkpoint drift was observed.
    pub has_checkpoint_plan_drift: bool,
    /// Stable classification of observed planned-vs-actual checkpoint drift.
    pub checkpoint_drift_kind: ToolAuditSupervisorDrainCheckpointDriftKind,
    /// Stable checkpoint-drift classification label for host logs.
    pub checkpoint_drift_label: &'static str,
    /// Whether the checkpoint-drift label parses back to the typed classification.
    pub checkpoint_drift_label_matches_kind: bool,
    /// Whether planned and actual checkpoint boundaries stayed aligned.
    pub is_no_checkpoint_drift: bool,
    /// Whether checkpoint drift should be investigated by the host.
    pub requires_checkpoint_drift_investigation: bool,
    /// Whether preflight/drain drift should be investigated by the host.
    pub requires_plan_drift_investigation: bool,
    /// Whether count drift should be investigated by the host.
    pub requires_count_drift_investigation: bool,
    /// Stable classification of why the host should investigate this run.
    pub host_investigation_kind: ToolAuditSupervisorDrainHostInvestigationKind,
    /// Stable host-investigation classification label for host logs.
    pub host_investigation_label: &'static str,
    /// Whether the host-investigation label parses back to the typed classification.
    pub host_investigation_label_matches_kind: bool,
    /// Whether every stable classifier label parses back to its typed value.
    pub all_classifier_labels_match: bool,
    /// Whether planned or replayed inventory counts drifted from payloads.
    pub has_inventory_count_integrity_drift: bool,
    /// Whether flattened run-status flags drifted from source counts.
    pub has_run_status_integrity_drift: bool,
    /// Whether checkpoint-boundary fields drifted from source checkpoints.
    pub has_checkpoint_boundary_integrity_drift: bool,
    /// Whether any stable classifier label drifted from its typed value.
    pub has_classifier_label_integrity_drift: bool,
    /// Whether any flattened host-log integrity drift was observed.
    pub has_run_integrity_drift: bool,
    /// Stable classification of flattened host-log integrity drift.
    pub run_integrity_kind: ToolAuditSupervisorDrainRunIntegrityKind,
    /// Stable run-integrity classification label for host logs.
    pub run_integrity_label: &'static str,
    /// Whether the run-integrity label parses back to the typed classification.
    pub run_integrity_label_matches_kind: bool,
    /// Whether all flattened host-log integrity checks are clean.
    pub is_run_integrity_clean: bool,
    /// Whether host-log integrity drift should be investigated.
    pub requires_run_integrity_investigation: bool,
    /// Whether the host should investigate any run divergence.
    pub requires_host_investigation: bool,
    /// Whether the host can skip drain-run investigation.
    pub is_no_host_investigation: bool,
    /// Whether the host investigation includes plan drift.
    pub requires_host_plan_drift_investigation: bool,
    /// Whether the host investigation includes count drift.
    pub requires_host_count_drift_investigation: bool,
    /// Whether any host attention is needed for this run.
    pub requires_host_attention: bool,
    /// Whether no host attention is needed for this run.
    pub is_no_host_attention: bool,
    /// Stable classification of why the host should pay attention to this run.
    pub host_attention_kind: ToolAuditSupervisorDrainHostAttentionKind,
    /// Stable host-attention classification label for host logs.
    pub host_attention_label: &'static str,
    /// Whether the host-attention label parses back to the typed classification.
    pub host_attention_label_matches_kind: bool,
    /// Whether host attention includes a scheduler action.
    pub host_attention_includes_scheduler_action: bool,
    /// Whether host attention includes run-divergence investigation.
    pub host_attention_includes_host_investigation: bool,
    /// Whether host attention includes host-log integrity investigation.
    pub host_attention_includes_run_integrity_investigation: bool,
    /// Whether more than one host-attention surface is active.
    pub has_multiple_host_attention: bool,
    /// Whether this run is terminal and needs no host attention.
    pub is_terminal_ready: bool,
    /// Stable classification of terminal readiness or pending host work.
    pub terminal_readiness_kind: ToolAuditSupervisorDrainTerminalReadinessKind,
    /// Stable terminal-readiness classification label for host logs.
    pub terminal_readiness_label: &'static str,
    /// Whether the terminal-readiness label parses back to the typed classification.
    pub terminal_readiness_label_matches_kind: bool,
    /// Whether terminal readiness is blocked by scheduler action.
    pub terminal_readiness_requires_scheduler_action: bool,
    /// Whether terminal readiness is blocked by run-divergence investigation.
    pub terminal_readiness_requires_host_investigation: bool,
    /// Whether terminal readiness is blocked by host-log integrity investigation.
    pub terminal_readiness_requires_run_integrity_investigation: bool,
    /// Whether terminal readiness is blocked by more than one host-attention surface.
    pub terminal_readiness_has_multiple_attention: bool,
    /// Whether this run is terminal-ready because no rows were waiting.
    pub terminal_readiness_is_idle: bool,
    /// Whether this run is terminal-ready because it caught up to the log end.
    pub terminal_readiness_is_caught_up: bool,
    /// Whether the host has a pending decision to make for this run.
    pub requires_host_decision: bool,
    /// Stable classification of the next host decision for this run.
    pub host_decision_kind: ToolAuditSupervisorDrainHostDecisionKind,
    /// Stable host-decision classification label for host logs.
    pub host_decision_label: &'static str,
    /// Whether the host-decision label parses back to the typed classification.
    pub host_decision_label_matches_kind: bool,
    /// Whether the host decision is that the run is terminal-ready.
    pub host_decision_is_terminal_ready: bool,
    /// Whether the next host decision is to schedule continuation.
    pub host_decision_schedules_continuation: bool,
    /// Whether the next host decision is to route follow-up pressure.
    pub host_decision_routes_follow_up: bool,
    /// Whether the next host decision is to investigate plan drift.
    pub host_decision_investigates_plan_drift: bool,
    /// Whether the next host decision is to investigate count drift.
    pub host_decision_investigates_count_drift: bool,
    /// Whether the next host decision is to investigate run integrity.
    pub host_decision_investigates_run_integrity: bool,
    /// Whether multiple host decisions are active.
    pub host_decision_has_multiple_actions: bool,
    /// Number of exact host-decision actions active.
    pub host_decision_action_count: usize,
    /// Whether exactly one host-decision action is active.
    pub host_decision_has_single_action: bool,
    /// Stable dashboard lane for the next host decision.
    pub host_decision_lane: ToolAuditSupervisorDrainHostDecisionLane,
    /// Stable host-decision lane label for dashboard grouping.
    pub host_decision_lane_label: &'static str,
    /// Whether the host-decision lane label parses back to the typed lane.
    pub host_decision_lane_label_matches_kind: bool,
    /// Whether the host decision belongs on the terminal dashboard lane.
    pub host_decision_uses_terminal_lane: bool,
    /// Whether the host decision belongs on the scheduler dashboard lane.
    pub host_decision_uses_scheduler_lane: bool,
    /// Whether the host decision belongs on the follow-up dashboard lane.
    pub host_decision_uses_follow_up_lane: bool,
    /// Whether the host decision belongs on the investigation dashboard lane.
    pub host_decision_uses_investigation_lane: bool,
    /// Whether the host decision spans multiple dashboard lanes.
    pub host_decision_uses_mixed_lane: bool,
    /// Stable dashboard priority for the next host decision.
    pub host_decision_priority: ToolAuditSupervisorDrainHostDecisionPriority,
    /// Stable host-decision priority label for dashboard sorting.
    pub host_decision_priority_label: &'static str,
    /// Whether the host-decision priority label parses back to the typed priority.
    pub host_decision_priority_label_matches_kind: bool,
    /// Sortable host-decision priority rank.
    pub host_decision_priority_rank: u8,
    /// Whether the host decision has settled priority.
    pub host_decision_has_settled_priority: bool,
    /// Whether the host decision has routine-action priority.
    pub host_decision_has_routine_priority: bool,
    /// Whether the host decision has drift-investigation priority.
    pub host_decision_has_drift_investigation_priority: bool,
    /// Whether the host decision has integrity-investigation priority.
    pub host_decision_has_integrity_investigation_priority: bool,
    /// Whether the host decision has mixed-action priority.
    pub host_decision_has_mixed_priority: bool,
    /// Whether the host decision priority is investigation-grade.
    pub host_decision_has_investigation_priority: bool,
    /// Stable readiness classification for queueing or routing the host decision.
    pub host_decision_readiness: ToolAuditSupervisorDrainHostDecisionReadiness,
    /// Stable host-decision readiness label for host queues.
    pub host_decision_readiness_label: &'static str,
    /// Whether the host-decision readiness label parses back to its typed value.
    pub host_decision_readiness_label_matches_kind: bool,
    /// Whether the host decision is settled and needs no queue entry.
    pub host_decision_is_settled: bool,
    /// Whether the host decision should appear in an action queue.
    pub host_decision_is_actionable: bool,
    /// Whether the host decision can be routed without investigation triage.
    pub host_decision_is_auto_routable: bool,
    /// Whether host-decision readiness needs manual review before routing.
    pub host_decision_readiness_requires_manual_review: bool,
    /// Whether host-decision readiness requires investigation.
    pub host_decision_readiness_requires_investigation: bool,
    /// Whether host-decision readiness requires host-log integrity investigation.
    pub host_decision_readiness_requires_integrity_investigation: bool,
    /// Whether multiple decision surfaces need human triage.
    pub host_decision_needs_triage: bool,
    /// Stable concrete queue route for the host decision.
    pub host_decision_route: ToolAuditSupervisorDrainHostDecisionRoute,
    /// Stable host-decision route label for host queues.
    pub host_decision_route_label: &'static str,
    /// Whether the host-decision route label parses back to its typed route.
    pub host_decision_route_label_matches_kind: bool,
    /// Whether the host decision has a concrete queue route.
    pub host_decision_has_route: bool,
    /// Whether the host decision routes to the scheduler queue.
    pub host_decision_routes_to_scheduler: bool,
    /// Whether the host decision routes to the follow-up queue.
    pub host_decision_routes_to_follow_up: bool,
    /// Whether the host decision routes to a drift-investigation queue.
    pub host_decision_routes_to_drift_investigation: bool,
    /// Whether the host decision routes to a host-log integrity queue.
    pub host_decision_routes_to_integrity_investigation: bool,
    /// Whether the host decision routes to a triage queue.
    pub host_decision_routes_to_triage: bool,
    /// Whether the host decision routes to any investigation queue.
    pub host_decision_routes_to_investigation: bool,
    /// Whether all host-decision classifier labels parse back to their typed values.
    pub host_decision_classifier_labels_match: bool,
    /// Whether any host-decision classifier label drifted from its typed value.
    pub has_host_decision_label_integrity_drift: bool,
    /// Whether all host-routing classifier labels parse back to their typed values.
    pub host_routing_classifier_labels_match: bool,
    /// Whether any host-routing classifier label drifted from its typed value.
    pub has_host_routing_label_integrity_drift: bool,
    /// Whether the actual run delivered the planned number of rows.
    pub matches_planned_record_count: bool,
    /// Whether planned and replayed row counts match.
    pub matches_record_count: bool,
    /// Whether the actual run preserved the planned follow-up pressure count.
    pub matches_planned_follow_up_record_count: bool,
    /// Whether planned and replayed follow-up pressure counts match.
    pub matches_follow_up_pressure: bool,
    /// Whether planned row counts agree with their inventories.
    pub plan_inventory_counts_match: bool,
    /// Whether replayed row counts agree with their inventories.
    pub drain_inventory_counts_match: bool,
    /// Whether every planned and replayed row count agrees with inventory.
    pub inventory_counts_match: bool,
    /// Whether remaining tick budget agrees with max ticks and drain ticks.
    pub remaining_tick_budget_matches_counts: bool,
    /// Whether tick-budget status flags agree with remaining budget.
    pub tick_budget_status_flags_match: bool,
    /// Whether drain status flags agree with source counts.
    pub drain_status_flags_match: bool,
    /// Whether all flattened budget and status flags agree with sources.
    pub run_status_flags_match: bool,
    /// Whether the actual run reached the current end of the audit log.
    pub reached_end_of_log: bool,
    /// Whether the actual run used every allowed tick.
    pub exhausted_tick_budget: bool,
    /// Whether no audit rows were replayed.
    pub is_idle: bool,
    /// Whether at least one audit row was replayed.
    pub made_progress: bool,
    /// Whether the supervisor should schedule another drain run.
    pub should_continue: bool,
    /// Whether planned or replayed rows need follow-up.
    pub requires_follow_up: bool,
    /// Whether any durable checkpoint advanced during the run.
    pub advanced_checkpoint: bool,
    /// Last replay checkpoint the preflight plan expected to observe.
    pub planned_last_checkpoint: Option<ToolAuditReadCheckpoint>,
    /// Whether the preflight plan observed a final replay checkpoint.
    pub has_planned_last_checkpoint: bool,
    /// Timestamp for the final replay checkpoint planned before the run.
    pub planned_last_checkpoint_timestamp_ms: Option<u64>,
    /// Call id for the final replay checkpoint planned before the run.
    pub planned_last_checkpoint_call_id: Option<String>,
    /// Whether planned checkpoint presence agrees with flattened scalar fields.
    pub planned_last_checkpoint_presence_matches_fields: bool,
    /// Whether the flattened planned timestamp matches the checkpoint object.
    pub planned_last_checkpoint_timestamp_matches_checkpoint: bool,
    /// Whether the flattened planned call id matches the checkpoint object.
    pub planned_last_checkpoint_call_id_matches_checkpoint: bool,
    /// Whether all flattened planned checkpoint fields agree with the object.
    pub planned_last_checkpoint_fields_match_checkpoint: bool,
    /// Whether the planned last checkpoint is after the starting checkpoint.
    pub planned_last_checkpoint_advanced_from_starting: bool,
    /// Whether the planned last checkpoint still matches the starting checkpoint.
    pub planned_last_checkpoint_matches_starting_checkpoint: bool,
    /// Last replay checkpoint observed by the actual run.
    pub last_checkpoint: Option<ToolAuditReadCheckpoint>,
    /// Whether the actual run observed a replay checkpoint.
    pub has_last_checkpoint: bool,
    /// Timestamp for the last replay checkpoint observed by the actual run.
    pub last_checkpoint_timestamp_ms: Option<u64>,
    /// Call id for the last replay checkpoint observed by the actual run.
    pub last_checkpoint_call_id: Option<String>,
    /// Whether checkpoint presence agrees with flattened scalar fields.
    pub last_checkpoint_presence_matches_fields: bool,
    /// Whether the flattened checkpoint timestamp matches the checkpoint object.
    pub last_checkpoint_timestamp_matches_checkpoint: bool,
    /// Whether the flattened checkpoint call id matches the checkpoint object.
    pub last_checkpoint_call_id_matches_checkpoint: bool,
    /// Whether all flattened checkpoint fields agree with the checkpoint object.
    pub last_checkpoint_fields_match_checkpoint: bool,
    /// Whether the last replay checkpoint is after the starting checkpoint.
    pub last_checkpoint_advanced_from_starting: bool,
    /// Whether the last replay checkpoint still matches the starting checkpoint.
    pub last_checkpoint_matches_starting_checkpoint: bool,
    /// Whether the durable-advance flag agrees with last-checkpoint movement.
    pub checkpoint_advance_matches_last_checkpoint: bool,
    /// Whether planned and actual final replay checkpoints match.
    pub planned_last_checkpoint_matches_last_checkpoint: bool,
    /// Whether planned checkpoint movement agrees with the actual drain.
    pub planned_checkpoint_advance_matches_drain: bool,
    /// Whether the planned checkpoint boundary agrees with the actual drain.
    pub checkpoint_plan_matches_drain: bool,
    /// Whether checkpoint-boundary fields agree with their source checkpoints.
    pub checkpoint_boundary_fields_match: bool,
}

impl ToolAuditSupervisorDrainRunSummary {
    /// Return the reader or supervisor checkpoint name drained by this run.
    pub fn checkpoint_name(&self) -> &str {
        &self.checkpoint_name
    }

    /// Return the checkpoint used to start the preflight plan.
    pub fn starting_checkpoint(&self) -> &ToolAuditReadCheckpoint {
        &self.starting_checkpoint
    }

    /// Return the starting checkpoint timestamp for host run logs.
    pub fn starting_checkpoint_timestamp_ms(&self) -> u64 {
        self.starting_checkpoint_timestamp_ms
    }

    /// Return the starting checkpoint call id for host run logs.
    pub fn starting_checkpoint_call_id(&self) -> &str {
        &self.starting_checkpoint_call_id
    }

    /// Return the durable checkpoint loaded before the run, if any.
    pub fn stored_checkpoint(&self) -> Option<&ToolAuditStoredCheckpoint> {
        self.stored_checkpoint.as_ref()
    }

    /// Return whether a durable checkpoint existed before the run.
    pub fn has_stored_checkpoint(&self) -> bool {
        self.has_stored_checkpoint
    }

    /// Return the durable checkpoint timestamp loaded before the run.
    pub fn stored_checkpoint_timestamp_ms(&self) -> Option<u64> {
        self.stored_checkpoint_timestamp_ms
    }

    /// Return the durable checkpoint call id loaded before the run.
    pub fn stored_checkpoint_call_id(&self) -> Option<&str> {
        self.stored_checkpoint_call_id.as_deref()
    }

    /// Return whether stored checkpoint presence agrees with flattened scalar fields.
    pub fn stored_checkpoint_presence_matches_fields(&self) -> bool {
        self.stored_checkpoint_presence_matches_fields
    }

    /// Return whether the flattened stored checkpoint timestamp matches the object.
    pub fn stored_checkpoint_timestamp_matches_checkpoint(&self) -> bool {
        self.stored_checkpoint_timestamp_matches_checkpoint
    }

    /// Return whether the flattened stored checkpoint call id matches the object.
    pub fn stored_checkpoint_call_id_matches_checkpoint(&self) -> bool {
        self.stored_checkpoint_call_id_matches_checkpoint
    }

    /// Return whether all flattened stored checkpoint fields agree with the object.
    pub fn stored_checkpoint_fields_match_checkpoint(&self) -> bool {
        self.stored_checkpoint_fields_match_checkpoint
    }

    /// Return whether the starting checkpoint matches the loaded durable checkpoint.
    pub fn starting_checkpoint_matches_stored_checkpoint(&self) -> bool {
        self.starting_checkpoint_matches_stored_checkpoint
    }

    /// Return the typed scheduler-facing classification for this run.
    pub fn outcome(&self) -> ToolAuditSupervisorDrainRunOutcome {
        self.outcome
    }

    /// Return the stable scheduler-facing label for this run outcome.
    pub fn outcome_label(&self) -> &'static str {
        self.outcome_label
    }

    /// Return whether the outcome label parses back to the typed outcome.
    pub fn outcome_label_matches_outcome(&self) -> bool {
        self.outcome_label_matches_outcome
    }

    /// Return whether the scheduler-facing outcome is idle.
    pub fn outcome_is_idle(&self) -> bool {
        self.outcome_is_idle
    }

    /// Return whether rows were replayed and this run reached the current log end.
    pub fn is_caught_up(&self) -> bool {
        self.is_caught_up
    }

    /// Return whether this run needs another drain pass.
    pub fn needs_continuation(&self) -> bool {
        self.needs_continuation
    }

    /// Return whether this run outcome routes follow-up pressure to the host.
    pub fn needs_follow_up(&self) -> bool {
        self.needs_follow_up
    }

    /// Return whether this run diverged from its preflight plan.
    pub fn plan_diverged(&self) -> bool {
        self.plan_diverged
    }

    /// Return the recommended scheduler action for this run.
    pub fn scheduler_action(&self) -> ToolAuditSupervisorDrainSchedulerAction {
        self.scheduler_action
    }

    /// Return whether this run outcome asks the scheduler to take action.
    pub fn requires_scheduler_action(&self) -> bool {
        self.requires_scheduler_action
    }

    /// Return whether this run intentionally leaves the scheduler idle.
    pub fn is_no_scheduler_action(&self) -> bool {
        self.is_no_scheduler_action
    }

    /// Return whether the scheduler should run another drain pass.
    pub fn requests_continuation(&self) -> bool {
        self.requests_continuation
    }

    /// Return whether follow-up pressure should be routed to the host.
    pub fn routes_follow_up(&self) -> bool {
        self.routes_follow_up
    }

    /// Return whether the host should investigate preflight/drain drift.
    pub fn requires_plan_drift_investigation(&self) -> bool {
        self.requires_plan_drift_investigation
    }

    /// Return the maximum rows each drain tick was allowed to replay.
    pub fn max_records_per_tick(&self) -> usize {
        self.max_records_per_tick
    }

    /// Return the maximum drain ticks requested for this run.
    pub fn max_ticks(&self) -> usize {
        self.max_ticks
    }

    /// Return the number of pages discovered by the preflight plan.
    pub fn planned_pages(&self) -> usize {
        self.planned_pages
    }

    /// Return the number of drain ticks that actually ran.
    pub fn drain_ticks(&self) -> usize {
        self.drain_ticks
    }

    /// Return how many requested drain ticks did not run.
    pub fn remaining_tick_budget(&self) -> usize {
        self.remaining_tick_budget
    }

    /// Return whether the run consumed every requested drain tick.
    pub fn used_all_ticks(&self) -> bool {
        self.used_all_ticks
    }

    /// Return whether more ticks were available when the run stopped.
    pub fn has_remaining_tick_budget(&self) -> bool {
        self.has_remaining_tick_budget
    }

    /// Return the total rows the preflight plan expected to replay.
    pub fn planned_records(&self) -> usize {
        self.planned_records
    }

    /// Return the total rows actually replayed into the sink.
    pub fn drained_records(&self) -> usize {
        self.drained_records
    }

    /// Return planned rows with follow-up pressure.
    pub fn planned_follow_up_records(&self) -> usize {
        self.planned_follow_up_records
    }

    /// Return replayed rows with follow-up pressure.
    pub fn drained_follow_up_records(&self) -> usize {
        self.drained_follow_up_records
    }

    /// Return replayed-minus-planned row count delta.
    pub fn record_count_delta(&self) -> i128 {
        self.record_count_delta
    }

    /// Return replayed-minus-planned follow-up pressure count delta.
    pub fn follow_up_record_count_delta(&self) -> i128 {
        self.follow_up_record_count_delta
    }

    /// Return whether planned and replayed row counts match.
    pub fn matches_record_count(&self) -> bool {
        self.matches_record_count
    }

    /// Return whether planned and replayed follow-up pressure counts match.
    pub fn matches_follow_up_pressure(&self) -> bool {
        self.matches_follow_up_pressure
    }

    /// Return whether the actual run delivered the planned number of rows.
    pub fn matches_planned_record_count(&self) -> bool {
        self.matches_planned_record_count
    }

    /// Return whether the actual run preserved the planned follow-up pressure count.
    pub fn matches_planned_follow_up_record_count(&self) -> bool {
        self.matches_planned_follow_up_record_count
    }

    /// Return whether planned row counts agree with their inventories.
    pub fn plan_inventory_counts_match(&self) -> bool {
        self.plan_inventory_counts_match
    }

    /// Return whether replayed row counts agree with their inventories.
    pub fn drain_inventory_counts_match(&self) -> bool {
        self.drain_inventory_counts_match
    }

    /// Return whether every planned and replayed row count agrees with inventory.
    pub fn inventory_counts_match(&self) -> bool {
        self.inventory_counts_match
    }

    /// Return whether remaining tick budget agrees with max ticks and drain ticks.
    pub fn remaining_tick_budget_matches_counts(&self) -> bool {
        self.remaining_tick_budget_matches_counts
    }

    /// Return whether tick-budget status flags agree with remaining budget.
    pub fn tick_budget_status_flags_match(&self) -> bool {
        self.tick_budget_status_flags_match
    }

    /// Return whether drain status flags agree with source counts.
    pub fn drain_status_flags_match(&self) -> bool {
        self.drain_status_flags_match
    }

    /// Return whether all flattened budget and status flags agree with sources.
    pub fn run_status_flags_match(&self) -> bool {
        self.run_status_flags_match
    }

    /// Return whether row count drift was observed.
    pub fn has_record_count_drift(&self) -> bool {
        self.has_record_count_drift
    }

    /// Return whether follow-up pressure count drift was observed.
    pub fn has_follow_up_record_count_drift(&self) -> bool {
        self.has_follow_up_record_count_drift
    }

    /// Return whether any planned-vs-actual count drift was observed.
    pub fn has_count_drift(&self) -> bool {
        self.has_count_drift
    }

    /// Return whether planned and replayed counts stayed aligned.
    pub fn is_no_count_drift(&self) -> bool {
        self.is_no_count_drift
    }

    /// Return whether planned and actual final checkpoints diverged.
    pub fn has_planned_last_checkpoint_drift(&self) -> bool {
        self.has_planned_last_checkpoint_drift
    }

    /// Return whether planned checkpoint movement diverged from the actual drain.
    pub fn has_checkpoint_advance_drift(&self) -> bool {
        self.has_checkpoint_advance_drift
    }

    /// Return whether any planned-vs-actual checkpoint drift was observed.
    pub fn has_checkpoint_plan_drift(&self) -> bool {
        self.has_checkpoint_plan_drift
    }

    /// Return whether planned and actual checkpoint boundaries stayed aligned.
    pub fn is_no_checkpoint_drift(&self) -> bool {
        self.is_no_checkpoint_drift
    }

    /// Return the typed count-drift classification.
    pub fn count_drift_kind(&self) -> ToolAuditSupervisorDrainCountDriftKind {
        self.count_drift_kind
    }

    /// Return the stable count-drift classification label.
    pub fn count_drift_label(&self) -> &'static str {
        self.count_drift_label
    }

    /// Return whether the count-drift label parses back to the typed classification.
    pub fn count_drift_label_matches_kind(&self) -> bool {
        self.count_drift_label_matches_kind
    }

    /// Return whether count drift should be investigated by the host.
    pub fn requires_count_drift_investigation(&self) -> bool {
        self.requires_count_drift_investigation
    }

    /// Return the typed checkpoint-drift classification.
    pub fn checkpoint_drift_kind(&self) -> ToolAuditSupervisorDrainCheckpointDriftKind {
        self.checkpoint_drift_kind
    }

    /// Return the stable checkpoint-drift classification label.
    pub fn checkpoint_drift_label(&self) -> &'static str {
        self.checkpoint_drift_label
    }

    /// Return whether the checkpoint-drift label parses back to the typed classification.
    pub fn checkpoint_drift_label_matches_kind(&self) -> bool {
        self.checkpoint_drift_label_matches_kind
    }

    /// Return whether checkpoint drift should be investigated by the host.
    pub fn requires_checkpoint_drift_investigation(&self) -> bool {
        self.requires_checkpoint_drift_investigation
    }

    /// Return the typed host-investigation classification.
    pub fn host_investigation_kind(&self) -> ToolAuditSupervisorDrainHostInvestigationKind {
        self.host_investigation_kind
    }

    /// Return the stable host-investigation classification label.
    pub fn host_investigation_label(&self) -> &'static str {
        self.host_investigation_label
    }

    /// Return whether the host-investigation label parses back to the typed classification.
    pub fn host_investigation_label_matches_kind(&self) -> bool {
        self.host_investigation_label_matches_kind
    }

    /// Return whether every stable classifier label parses back to its typed value.
    pub fn all_classifier_labels_match(&self) -> bool {
        self.all_classifier_labels_match
    }

    /// Return whether planned or replayed inventory counts drifted from payloads.
    pub fn has_inventory_count_integrity_drift(&self) -> bool {
        self.has_inventory_count_integrity_drift
    }

    /// Return whether flattened run-status flags drifted from source counts.
    pub fn has_run_status_integrity_drift(&self) -> bool {
        self.has_run_status_integrity_drift
    }

    /// Return whether checkpoint-boundary fields drifted from source checkpoints.
    pub fn has_checkpoint_boundary_integrity_drift(&self) -> bool {
        self.has_checkpoint_boundary_integrity_drift
    }

    /// Return whether any stable classifier label drifted from its typed value.
    pub fn has_classifier_label_integrity_drift(&self) -> bool {
        self.has_classifier_label_integrity_drift
    }

    /// Return whether any flattened host-log integrity drift was observed.
    pub fn has_run_integrity_drift(&self) -> bool {
        self.has_run_integrity_drift
    }

    /// Return the typed run-integrity classification.
    pub fn run_integrity_kind(&self) -> ToolAuditSupervisorDrainRunIntegrityKind {
        self.run_integrity_kind
    }

    /// Return the stable run-integrity classification label.
    pub fn run_integrity_label(&self) -> &'static str {
        self.run_integrity_label
    }

    /// Return whether the run-integrity label parses back to the typed classification.
    pub fn run_integrity_label_matches_kind(&self) -> bool {
        self.run_integrity_label_matches_kind
    }

    /// Return whether all flattened host-log integrity checks are clean.
    pub fn is_run_integrity_clean(&self) -> bool {
        self.is_run_integrity_clean
    }

    /// Return whether host-log integrity drift should be investigated.
    pub fn requires_run_integrity_investigation(&self) -> bool {
        self.requires_run_integrity_investigation
    }

    /// Return whether the host should investigate any run divergence.
    pub fn requires_host_investigation(&self) -> bool {
        self.requires_host_investigation
    }

    /// Return whether the host investigation includes plan drift.
    pub fn requires_host_plan_drift_investigation(&self) -> bool {
        self.requires_host_plan_drift_investigation
    }

    /// Return whether the host investigation includes count drift.
    pub fn requires_host_count_drift_investigation(&self) -> bool {
        self.requires_host_count_drift_investigation
    }

    /// Return whether the host can skip drain-run investigation.
    pub fn is_no_host_investigation(&self) -> bool {
        self.is_no_host_investigation
    }

    /// Return whether any host attention is needed for this run.
    pub fn requires_host_attention(&self) -> bool {
        self.requires_host_attention
    }

    /// Return whether no host attention is needed for this run.
    pub fn is_no_host_attention(&self) -> bool {
        self.is_no_host_attention
    }

    /// Return the typed host-attention classification.
    pub fn host_attention_kind(&self) -> ToolAuditSupervisorDrainHostAttentionKind {
        self.host_attention_kind
    }

    /// Return the stable host-attention classification label.
    pub fn host_attention_label(&self) -> &'static str {
        self.host_attention_label
    }

    /// Return whether the host-attention label parses back to the typed classification.
    pub fn host_attention_label_matches_kind(&self) -> bool {
        self.host_attention_label_matches_kind
    }

    /// Return whether host attention includes a scheduler action.
    pub fn host_attention_includes_scheduler_action(&self) -> bool {
        self.host_attention_includes_scheduler_action
    }

    /// Return whether host attention includes run-divergence investigation.
    pub fn host_attention_includes_host_investigation(&self) -> bool {
        self.host_attention_includes_host_investigation
    }

    /// Return whether host attention includes host-log integrity investigation.
    pub fn host_attention_includes_run_integrity_investigation(&self) -> bool {
        self.host_attention_includes_run_integrity_investigation
    }

    /// Return whether more than one host-attention surface is active.
    pub fn has_multiple_host_attention(&self) -> bool {
        self.has_multiple_host_attention
    }

    /// Return whether this run is terminal and needs no host attention.
    pub fn is_terminal_ready(&self) -> bool {
        self.is_terminal_ready
    }

    /// Return the typed terminal-readiness classification.
    pub fn terminal_readiness_kind(&self) -> ToolAuditSupervisorDrainTerminalReadinessKind {
        self.terminal_readiness_kind
    }

    /// Return the stable terminal-readiness classification label.
    pub fn terminal_readiness_label(&self) -> &'static str {
        self.terminal_readiness_label
    }

    /// Return whether the terminal-readiness label parses back to the typed classification.
    pub fn terminal_readiness_label_matches_kind(&self) -> bool {
        self.terminal_readiness_label_matches_kind
    }

    /// Return whether terminal readiness is blocked by scheduler action.
    pub fn terminal_readiness_requires_scheduler_action(&self) -> bool {
        self.terminal_readiness_requires_scheduler_action
    }

    /// Return whether terminal readiness is blocked by run-divergence investigation.
    pub fn terminal_readiness_requires_host_investigation(&self) -> bool {
        self.terminal_readiness_requires_host_investigation
    }

    /// Return whether terminal readiness is blocked by host-log integrity investigation.
    pub fn terminal_readiness_requires_run_integrity_investigation(&self) -> bool {
        self.terminal_readiness_requires_run_integrity_investigation
    }

    /// Return whether terminal readiness is blocked by more than one host-attention surface.
    pub fn terminal_readiness_has_multiple_attention(&self) -> bool {
        self.terminal_readiness_has_multiple_attention
    }

    /// Return whether this run is terminal-ready because no rows were waiting.
    pub fn terminal_readiness_is_idle(&self) -> bool {
        self.terminal_readiness_is_idle
    }

    /// Return whether this run is terminal-ready because it caught up to the log end.
    pub fn terminal_readiness_is_caught_up(&self) -> bool {
        self.terminal_readiness_is_caught_up
    }

    /// Return whether the host has a pending decision to make for this run.
    pub fn requires_host_decision(&self) -> bool {
        self.requires_host_decision
    }

    /// Return the typed host-decision classification.
    pub fn host_decision_kind(&self) -> ToolAuditSupervisorDrainHostDecisionKind {
        self.host_decision_kind
    }

    /// Return the stable host-decision classification label.
    pub fn host_decision_label(&self) -> &'static str {
        self.host_decision_label
    }

    /// Return whether the host-decision label parses back to the typed classification.
    pub fn host_decision_label_matches_kind(&self) -> bool {
        self.host_decision_label_matches_kind
    }

    /// Return whether the host decision is that the run is terminal-ready.
    pub fn host_decision_is_terminal_ready(&self) -> bool {
        self.host_decision_is_terminal_ready
    }

    /// Return whether the next host decision is to schedule continuation.
    pub fn host_decision_schedules_continuation(&self) -> bool {
        self.host_decision_schedules_continuation
    }

    /// Return whether the next host decision is to route follow-up pressure.
    pub fn host_decision_routes_follow_up(&self) -> bool {
        self.host_decision_routes_follow_up
    }

    /// Return whether the next host decision is to investigate plan drift.
    pub fn host_decision_investigates_plan_drift(&self) -> bool {
        self.host_decision_investigates_plan_drift
    }

    /// Return whether the next host decision is to investigate count drift.
    pub fn host_decision_investigates_count_drift(&self) -> bool {
        self.host_decision_investigates_count_drift
    }

    /// Return whether the next host decision is to investigate run integrity.
    pub fn host_decision_investigates_run_integrity(&self) -> bool {
        self.host_decision_investigates_run_integrity
    }

    /// Return whether multiple host decisions are active.
    pub fn host_decision_has_multiple_actions(&self) -> bool {
        self.host_decision_has_multiple_actions
    }

    /// Return how many exact host-decision actions are active.
    pub fn host_decision_action_count(&self) -> usize {
        self.host_decision_action_count
    }

    /// Return whether exactly one host-decision action is active.
    pub fn host_decision_has_single_action(&self) -> bool {
        self.host_decision_has_single_action
    }

    /// Return the typed host-decision dashboard lane.
    pub fn host_decision_lane(&self) -> ToolAuditSupervisorDrainHostDecisionLane {
        self.host_decision_lane
    }

    /// Return the stable host-decision lane label.
    pub fn host_decision_lane_label(&self) -> &'static str {
        self.host_decision_lane_label
    }

    /// Return whether the host-decision lane label parses back to the typed lane.
    pub fn host_decision_lane_label_matches_kind(&self) -> bool {
        self.host_decision_lane_label_matches_kind
    }

    /// Return whether the host decision belongs on the terminal dashboard lane.
    pub fn host_decision_uses_terminal_lane(&self) -> bool {
        self.host_decision_uses_terminal_lane
    }

    /// Return whether the host decision belongs on the scheduler dashboard lane.
    pub fn host_decision_uses_scheduler_lane(&self) -> bool {
        self.host_decision_uses_scheduler_lane
    }

    /// Return whether the host decision belongs on the follow-up dashboard lane.
    pub fn host_decision_uses_follow_up_lane(&self) -> bool {
        self.host_decision_uses_follow_up_lane
    }

    /// Return whether the host decision belongs on the investigation dashboard lane.
    pub fn host_decision_uses_investigation_lane(&self) -> bool {
        self.host_decision_uses_investigation_lane
    }

    /// Return whether the host decision spans multiple dashboard lanes.
    pub fn host_decision_uses_mixed_lane(&self) -> bool {
        self.host_decision_uses_mixed_lane
    }

    /// Return the typed host-decision dashboard priority.
    pub fn host_decision_priority(&self) -> ToolAuditSupervisorDrainHostDecisionPriority {
        self.host_decision_priority
    }

    /// Return the stable host-decision priority label.
    pub fn host_decision_priority_label(&self) -> &'static str {
        self.host_decision_priority_label
    }

    /// Return whether the host-decision priority label parses back to the typed priority.
    pub fn host_decision_priority_label_matches_kind(&self) -> bool {
        self.host_decision_priority_label_matches_kind
    }

    /// Return the sortable host-decision priority rank.
    pub fn host_decision_priority_rank(&self) -> u8 {
        self.host_decision_priority_rank
    }

    /// Return whether the host decision has settled priority.
    pub fn host_decision_has_settled_priority(&self) -> bool {
        self.host_decision_has_settled_priority
    }

    /// Return whether the host decision has routine-action priority.
    pub fn host_decision_has_routine_priority(&self) -> bool {
        self.host_decision_has_routine_priority
    }

    /// Return whether the host decision has drift-investigation priority.
    pub fn host_decision_has_drift_investigation_priority(&self) -> bool {
        self.host_decision_has_drift_investigation_priority
    }

    /// Return whether the host decision has integrity-investigation priority.
    pub fn host_decision_has_integrity_investigation_priority(&self) -> bool {
        self.host_decision_has_integrity_investigation_priority
    }

    /// Return whether the host decision has mixed-action priority.
    pub fn host_decision_has_mixed_priority(&self) -> bool {
        self.host_decision_has_mixed_priority
    }

    /// Return whether the host decision priority is investigation-grade.
    pub fn host_decision_has_investigation_priority(&self) -> bool {
        self.host_decision_has_investigation_priority
    }

    /// Return the typed host-decision readiness classification.
    pub fn host_decision_readiness(&self) -> ToolAuditSupervisorDrainHostDecisionReadiness {
        self.host_decision_readiness
    }

    /// Return the stable host-decision readiness label.
    pub fn host_decision_readiness_label(&self) -> &'static str {
        self.host_decision_readiness_label
    }

    /// Return whether the host-decision readiness label parses back to its typed value.
    pub fn host_decision_readiness_label_matches_kind(&self) -> bool {
        self.host_decision_readiness_label_matches_kind
    }

    /// Return whether the host decision is settled and needs no queue entry.
    pub fn host_decision_is_settled(&self) -> bool {
        self.host_decision_is_settled
    }

    /// Return whether the host decision should appear in an action queue.
    pub fn host_decision_is_actionable(&self) -> bool {
        self.host_decision_is_actionable
    }

    /// Return whether the host decision can be routed without investigation triage.
    pub fn host_decision_is_auto_routable(&self) -> bool {
        self.host_decision_is_auto_routable
    }

    /// Return whether host-decision readiness needs manual review before routing.
    pub fn host_decision_readiness_requires_manual_review(&self) -> bool {
        self.host_decision_readiness_requires_manual_review
    }

    /// Return whether host-decision readiness requires investigation.
    pub fn host_decision_readiness_requires_investigation(&self) -> bool {
        self.host_decision_readiness_requires_investigation
    }

    /// Return whether host-decision readiness requires host-log integrity investigation.
    pub fn host_decision_readiness_requires_integrity_investigation(&self) -> bool {
        self.host_decision_readiness_requires_integrity_investigation
    }

    /// Return whether multiple decision surfaces need human triage.
    pub fn host_decision_needs_triage(&self) -> bool {
        self.host_decision_needs_triage
    }

    /// Return the typed host-decision route.
    pub fn host_decision_route(&self) -> ToolAuditSupervisorDrainHostDecisionRoute {
        self.host_decision_route
    }

    /// Return the stable host-decision route label.
    pub fn host_decision_route_label(&self) -> &'static str {
        self.host_decision_route_label
    }

    /// Return whether the host-decision route label parses back to its typed route.
    pub fn host_decision_route_label_matches_kind(&self) -> bool {
        self.host_decision_route_label_matches_kind
    }

    /// Return whether the host decision has a concrete queue route.
    pub fn host_decision_has_route(&self) -> bool {
        self.host_decision_has_route
    }

    /// Return whether the host decision routes to the scheduler queue.
    pub fn host_decision_routes_to_scheduler(&self) -> bool {
        self.host_decision_routes_to_scheduler
    }

    /// Return whether the host decision routes to the follow-up queue.
    pub fn host_decision_routes_to_follow_up(&self) -> bool {
        self.host_decision_routes_to_follow_up
    }

    /// Return whether the host decision routes to a drift-investigation queue.
    pub fn host_decision_routes_to_drift_investigation(&self) -> bool {
        self.host_decision_routes_to_drift_investigation
    }

    /// Return whether the host decision routes to a host-log integrity queue.
    pub fn host_decision_routes_to_integrity_investigation(&self) -> bool {
        self.host_decision_routes_to_integrity_investigation
    }

    /// Return whether the host decision routes to a triage queue.
    pub fn host_decision_routes_to_triage(&self) -> bool {
        self.host_decision_routes_to_triage
    }

    /// Return whether the host decision routes to any investigation queue.
    pub fn host_decision_routes_to_investigation(&self) -> bool {
        self.host_decision_routes_to_investigation
    }

    /// Return whether all host-decision classifier labels parse back to their typed values.
    pub fn host_decision_classifier_labels_match(&self) -> bool {
        self.host_decision_classifier_labels_match
    }

    /// Return whether any host-decision classifier label drifted from its typed value.
    pub fn has_host_decision_label_integrity_drift(&self) -> bool {
        self.has_host_decision_label_integrity_drift
    }

    /// Return whether all host-routing classifier labels parse back to their typed values.
    pub fn host_routing_classifier_labels_match(&self) -> bool {
        self.host_routing_classifier_labels_match
    }

    /// Return whether any host-routing classifier label drifted from its typed value.
    pub fn has_host_routing_label_integrity_drift(&self) -> bool {
        self.has_host_routing_label_integrity_drift
    }

    /// Return whether the actual run replayed more rows than planned.
    pub fn replayed_extra_records(&self) -> bool {
        self.replayed_extra_records
    }

    /// Return whether the actual run replayed fewer rows than planned.
    pub fn missed_planned_records(&self) -> bool {
        self.missed_planned_records
    }

    /// Return whether the actual run replayed more follow-up pressure than planned.
    pub fn replayed_extra_follow_up_records(&self) -> bool {
        self.replayed_extra_follow_up_records
    }

    /// Return whether the actual run replayed less follow-up pressure than planned.
    pub fn missed_planned_follow_up_records(&self) -> bool {
        self.missed_planned_follow_up_records
    }

    /// Return the stable scheduler-action label for host logs.
    pub fn scheduler_action_label(&self) -> &'static str {
        self.scheduler_action_label
    }

    /// Return whether the scheduler-action label parses back to the typed action.
    pub fn scheduler_action_label_matches_action(&self) -> bool {
        self.scheduler_action_label_matches_action
    }

    /// Return whether no audit rows were replayed.
    pub fn is_idle(&self) -> bool {
        self.is_idle
    }

    /// Return whether at least one audit row was replayed.
    pub fn made_progress(&self) -> bool {
        self.made_progress
    }

    /// Return whether the supervisor should schedule another drain run.
    pub fn should_continue(&self) -> bool {
        self.should_continue
    }

    /// Return whether the actual run reached the current end of the audit log.
    pub fn reached_end_of_log(&self) -> bool {
        self.reached_end_of_log
    }

    /// Return whether the actual run consumed the configured tick budget.
    pub fn exhausted_tick_budget(&self) -> bool {
        self.exhausted_tick_budget
    }

    /// Return whether the planned or replayed rows contain follow-up pressure.
    pub fn requires_follow_up(&self) -> bool {
        self.requires_follow_up
    }

    /// Return whether the actual run advanced the durable checkpoint.
    pub fn advanced_checkpoint(&self) -> bool {
        self.advanced_checkpoint
    }

    /// Return the last replay checkpoint the preflight plan expected to observe.
    pub fn planned_last_checkpoint(&self) -> Option<&ToolAuditReadCheckpoint> {
        self.planned_last_checkpoint.as_ref()
    }

    /// Return whether the preflight plan observed a final replay checkpoint.
    pub fn has_planned_last_checkpoint(&self) -> bool {
        self.has_planned_last_checkpoint
    }

    /// Return the timestamp for the final replay checkpoint planned before the run.
    pub fn planned_last_checkpoint_timestamp_ms(&self) -> Option<u64> {
        self.planned_last_checkpoint_timestamp_ms
    }

    /// Return the call id for the final replay checkpoint planned before the run.
    pub fn planned_last_checkpoint_call_id(&self) -> Option<&str> {
        self.planned_last_checkpoint_call_id.as_deref()
    }

    /// Return whether planned checkpoint presence agrees with flattened scalar fields.
    pub fn planned_last_checkpoint_presence_matches_fields(&self) -> bool {
        self.planned_last_checkpoint_presence_matches_fields
    }

    /// Return whether the flattened planned timestamp matches the checkpoint object.
    pub fn planned_last_checkpoint_timestamp_matches_checkpoint(&self) -> bool {
        self.planned_last_checkpoint_timestamp_matches_checkpoint
    }

    /// Return whether the flattened planned call id matches the checkpoint object.
    pub fn planned_last_checkpoint_call_id_matches_checkpoint(&self) -> bool {
        self.planned_last_checkpoint_call_id_matches_checkpoint
    }

    /// Return whether all flattened planned checkpoint fields agree with the object.
    pub fn planned_last_checkpoint_fields_match_checkpoint(&self) -> bool {
        self.planned_last_checkpoint_fields_match_checkpoint
    }

    /// Return whether the planned last checkpoint is after the starting checkpoint.
    pub fn planned_last_checkpoint_advanced_from_starting(&self) -> bool {
        self.planned_last_checkpoint_advanced_from_starting
    }

    /// Return whether the planned last checkpoint still matches the starting checkpoint.
    pub fn planned_last_checkpoint_matches_starting_checkpoint(&self) -> bool {
        self.planned_last_checkpoint_matches_starting_checkpoint
    }

    /// Return the last replay checkpoint observed by the actual run.
    pub fn last_checkpoint(&self) -> Option<&ToolAuditReadCheckpoint> {
        self.last_checkpoint.as_ref()
    }

    /// Return whether the actual run observed a replay checkpoint.
    pub fn has_last_checkpoint(&self) -> bool {
        self.has_last_checkpoint
    }

    /// Return the timestamp for the last replay checkpoint observed by the actual run.
    pub fn last_checkpoint_timestamp_ms(&self) -> Option<u64> {
        self.last_checkpoint_timestamp_ms
    }

    /// Return the call id for the last replay checkpoint observed by the actual run.
    pub fn last_checkpoint_call_id(&self) -> Option<&str> {
        self.last_checkpoint_call_id.as_deref()
    }

    /// Return whether checkpoint presence agrees with flattened scalar fields.
    pub fn last_checkpoint_presence_matches_fields(&self) -> bool {
        self.last_checkpoint_presence_matches_fields
    }

    /// Return whether the flattened checkpoint timestamp matches the checkpoint object.
    pub fn last_checkpoint_timestamp_matches_checkpoint(&self) -> bool {
        self.last_checkpoint_timestamp_matches_checkpoint
    }

    /// Return whether the flattened checkpoint call id matches the checkpoint object.
    pub fn last_checkpoint_call_id_matches_checkpoint(&self) -> bool {
        self.last_checkpoint_call_id_matches_checkpoint
    }

    /// Return whether all flattened checkpoint fields agree with the checkpoint object.
    pub fn last_checkpoint_fields_match_checkpoint(&self) -> bool {
        self.last_checkpoint_fields_match_checkpoint
    }

    /// Return whether the last replay checkpoint is after the starting checkpoint.
    pub fn last_checkpoint_advanced_from_starting(&self) -> bool {
        self.last_checkpoint_advanced_from_starting
    }

    /// Return whether the last replay checkpoint still matches the starting checkpoint.
    pub fn last_checkpoint_matches_starting_checkpoint(&self) -> bool {
        self.last_checkpoint_matches_starting_checkpoint
    }

    /// Return whether the durable-advance flag agrees with last-checkpoint movement.
    pub fn checkpoint_advance_matches_last_checkpoint(&self) -> bool {
        self.checkpoint_advance_matches_last_checkpoint
    }

    /// Return whether planned and actual final replay checkpoints match.
    pub fn planned_last_checkpoint_matches_last_checkpoint(&self) -> bool {
        self.planned_last_checkpoint_matches_last_checkpoint
    }

    /// Return whether planned checkpoint movement agrees with the actual drain.
    pub fn planned_checkpoint_advance_matches_drain(&self) -> bool {
        self.planned_checkpoint_advance_matches_drain
    }

    /// Return whether the planned checkpoint boundary agrees with the actual drain.
    pub fn checkpoint_plan_matches_drain(&self) -> bool {
        self.checkpoint_plan_matches_drain
    }

    /// Return whether checkpoint-boundary fields agree with their source checkpoints.
    pub fn checkpoint_boundary_fields_match(&self) -> bool {
        self.checkpoint_boundary_fields_match
    }
}

/// Scheduler-facing classification for a bounded supervisor drain run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainRunOutcome {
    /// No rows were waiting and the checkpoint stayed untouched.
    Idle,
    /// Rows were replayed and the drain reached the current end of the log.
    CaughtUp,
    /// The tick budget was exhausted before reaching the end of the log.
    NeedsContinuation,
    /// Planned or replayed rows contain follow-up pressure.
    NeedsFollowUp,
    /// The preflight plan and actual drain delivered different row counts.
    PlanDiverged,
}

impl ToolAuditSupervisorDrainRunOutcome {
    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::CaughtUp => "caught_up",
            Self::NeedsContinuation => "needs_continuation",
            Self::NeedsFollowUp => "needs_follow_up",
            Self::PlanDiverged => "plan_diverged",
        }
    }

    /// Parse a stable snake_case outcome label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "idle" => Some(Self::Idle),
            "caught_up" => Some(Self::CaughtUp),
            "needs_continuation" => Some(Self::NeedsContinuation),
            "needs_follow_up" => Some(Self::NeedsFollowUp),
            "plan_diverged" => Some(Self::PlanDiverged),
            _ => None,
        }
    }

    /// Return whether no rows were waiting and the checkpoint stayed untouched.
    pub fn is_idle(self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Return whether rows were replayed and the run reached the current log end.
    pub fn is_caught_up(self) -> bool {
        matches!(self, Self::CaughtUp)
    }

    /// Return whether this outcome stopped before reaching the current log end.
    pub fn needs_continuation(self) -> bool {
        matches!(self, Self::NeedsContinuation)
    }

    /// Return whether this outcome contains follow-up pressure.
    pub fn needs_follow_up(self) -> bool {
        matches!(self, Self::NeedsFollowUp)
    }

    /// Return whether the preflight plan and actual drain counts diverged.
    pub fn plan_diverged(self) -> bool {
        matches!(self, Self::PlanDiverged)
    }

    /// Return whether the scheduler should take action for this outcome.
    pub fn requires_scheduler_action(self) -> bool {
        self.scheduler_action().requires_scheduler_action()
    }

    /// Return whether this outcome intentionally leaves the scheduler idle.
    pub fn is_no_scheduler_action(self) -> bool {
        self.scheduler_action().is_no_action()
    }

    /// Return the recommended scheduler action for this outcome.
    pub fn scheduler_action(self) -> ToolAuditSupervisorDrainSchedulerAction {
        match self {
            Self::Idle | Self::CaughtUp => ToolAuditSupervisorDrainSchedulerAction::NoAction,
            Self::NeedsContinuation => {
                ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation
            }
            Self::NeedsFollowUp => ToolAuditSupervisorDrainSchedulerAction::RouteFollowUp,
            Self::PlanDiverged => ToolAuditSupervisorDrainSchedulerAction::InvestigatePlanDrift,
        }
    }

    /// Return the stable scheduler-action label for this outcome.
    pub fn scheduler_action_label(self) -> &'static str {
        self.scheduler_action().as_str()
    }

    /// Return whether this outcome asks the host to schedule another drain pass.
    pub fn requests_continuation(self) -> bool {
        self.scheduler_action().requests_continuation()
    }

    /// Return whether this outcome routes follow-up pressure to the host.
    pub fn routes_follow_up(self) -> bool {
        self.scheduler_action().routes_follow_up()
    }

    /// Return whether this outcome asks the host to investigate plan drift.
    pub fn requires_plan_drift_investigation(self) -> bool {
        self.scheduler_action().requires_plan_drift_investigation()
    }
}

impl Display for ToolAuditSupervisorDrainRunOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Scheduler recommendation derived from a bounded supervisor drain outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainSchedulerAction {
    /// The run is terminal for now and needs no scheduler follow-up.
    NoAction,
    /// Schedule another bounded drain pass from the advanced checkpoint.
    ScheduleContinuation,
    /// Route replayed or planned follow-up pressure to the supervising host.
    RouteFollowUp,
    /// Investigate a mismatch between the preflight plan and actual drain.
    InvestigatePlanDrift,
}

impl ToolAuditSupervisorDrainSchedulerAction {
    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoAction => "no_action",
            Self::ScheduleContinuation => "schedule_continuation",
            Self::RouteFollowUp => "route_follow_up",
            Self::InvestigatePlanDrift => "investigate_plan_drift",
        }
    }

    /// Parse a stable snake_case scheduler action label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_action" => Some(Self::NoAction),
            "schedule_continuation" => Some(Self::ScheduleContinuation),
            "route_follow_up" => Some(Self::RouteFollowUp),
            "investigate_plan_drift" => Some(Self::InvestigatePlanDrift),
            _ => None,
        }
    }

    /// Return whether the host scheduler needs to take an explicit action.
    pub fn requires_scheduler_action(self) -> bool {
        !matches!(self, Self::NoAction)
    }

    /// Return whether this action intentionally leaves the scheduler idle.
    pub fn is_no_action(self) -> bool {
        matches!(self, Self::NoAction)
    }

    /// Return whether this action asks the host to schedule another drain pass.
    pub fn requests_continuation(self) -> bool {
        matches!(self, Self::ScheduleContinuation)
    }

    /// Return whether this action routes follow-up pressure to the host.
    pub fn routes_follow_up(self) -> bool {
        matches!(self, Self::RouteFollowUp)
    }

    /// Return whether this action asks the host to investigate plan drift.
    pub fn requires_plan_drift_investigation(self) -> bool {
        matches!(self, Self::InvestigatePlanDrift)
    }
}

impl Display for ToolAuditSupervisorDrainSchedulerAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable classification of planned-vs-actual count drift in a drain run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainCountDriftKind {
    /// Planned and replayed counts match.
    NoDrift,
    /// Row counts drifted while follow-up pressure counts matched.
    RecordCountDrift,
    /// Follow-up pressure counts drifted while row counts matched.
    FollowUpRecordCountDrift,
    /// Both row counts and follow-up pressure counts drifted.
    RecordAndFollowUpRecordCountDrift,
}

impl ToolAuditSupervisorDrainCountDriftKind {
    /// Classify drift from row-count and follow-up pressure drift flags.
    pub fn from_drift_flags(
        has_record_count_drift: bool,
        has_follow_up_record_count_drift: bool,
    ) -> Self {
        match (has_record_count_drift, has_follow_up_record_count_drift) {
            (false, false) => Self::NoDrift,
            (true, false) => Self::RecordCountDrift,
            (false, true) => Self::FollowUpRecordCountDrift,
            (true, true) => Self::RecordAndFollowUpRecordCountDrift,
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoDrift => "no_count_drift",
            Self::RecordCountDrift => "record_count_drift",
            Self::FollowUpRecordCountDrift => "follow_up_record_count_drift",
            Self::RecordAndFollowUpRecordCountDrift => "record_and_follow_up_record_count_drift",
        }
    }

    /// Parse a stable snake_case count-drift label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_count_drift" => Some(Self::NoDrift),
            "record_count_drift" => Some(Self::RecordCountDrift),
            "follow_up_record_count_drift" => Some(Self::FollowUpRecordCountDrift),
            "record_and_follow_up_record_count_drift" => {
                Some(Self::RecordAndFollowUpRecordCountDrift)
            }
            _ => None,
        }
    }

    /// Return whether row count drift was observed.
    pub fn has_record_count_drift(self) -> bool {
        matches!(
            self,
            Self::RecordCountDrift | Self::RecordAndFollowUpRecordCountDrift
        )
    }

    /// Return whether follow-up pressure count drift was observed.
    pub fn has_follow_up_record_count_drift(self) -> bool {
        matches!(
            self,
            Self::FollowUpRecordCountDrift | Self::RecordAndFollowUpRecordCountDrift
        )
    }

    /// Return whether any planned-vs-actual count drift was observed.
    pub fn has_count_drift(self) -> bool {
        !matches!(self, Self::NoDrift)
    }

    /// Return whether planned and replayed counts stayed aligned.
    pub fn is_no_drift(self) -> bool {
        matches!(self, Self::NoDrift)
    }

    /// Return whether this count drift classification needs investigation.
    pub fn requires_investigation(self) -> bool {
        self.has_count_drift()
    }
}

impl Display for ToolAuditSupervisorDrainCountDriftKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable classification of planned-vs-actual checkpoint drift in a drain run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainCheckpointDriftKind {
    /// Planned and actual checkpoint boundaries match.
    NoDrift,
    /// The final planned checkpoint and final replay checkpoint drifted.
    PlannedLastCheckpointDrift,
    /// Planned checkpoint movement and actual checkpoint advancement drifted.
    CheckpointAdvanceDrift,
    /// Both final checkpoint and checkpoint-advance signals drifted.
    PlannedLastCheckpointAndAdvanceDrift,
}

impl ToolAuditSupervisorDrainCheckpointDriftKind {
    /// Classify drift from final checkpoint and checkpoint-advance drift flags.
    pub fn from_drift_flags(
        has_planned_last_checkpoint_drift: bool,
        has_checkpoint_advance_drift: bool,
    ) -> Self {
        match (
            has_planned_last_checkpoint_drift,
            has_checkpoint_advance_drift,
        ) {
            (false, false) => Self::NoDrift,
            (true, false) => Self::PlannedLastCheckpointDrift,
            (false, true) => Self::CheckpointAdvanceDrift,
            (true, true) => Self::PlannedLastCheckpointAndAdvanceDrift,
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoDrift => "no_checkpoint_drift",
            Self::PlannedLastCheckpointDrift => "planned_last_checkpoint_drift",
            Self::CheckpointAdvanceDrift => "checkpoint_advance_drift",
            Self::PlannedLastCheckpointAndAdvanceDrift => {
                "planned_last_checkpoint_and_advance_drift"
            }
        }
    }

    /// Parse a stable snake_case checkpoint-drift label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_checkpoint_drift" => Some(Self::NoDrift),
            "planned_last_checkpoint_drift" => Some(Self::PlannedLastCheckpointDrift),
            "checkpoint_advance_drift" => Some(Self::CheckpointAdvanceDrift),
            "planned_last_checkpoint_and_advance_drift" => {
                Some(Self::PlannedLastCheckpointAndAdvanceDrift)
            }
            _ => None,
        }
    }

    /// Return whether the final planned and actual checkpoints drifted.
    pub fn has_planned_last_checkpoint_drift(self) -> bool {
        matches!(
            self,
            Self::PlannedLastCheckpointDrift | Self::PlannedLastCheckpointAndAdvanceDrift
        )
    }

    /// Return whether planned and actual checkpoint-advance signals drifted.
    pub fn has_checkpoint_advance_drift(self) -> bool {
        matches!(
            self,
            Self::CheckpointAdvanceDrift | Self::PlannedLastCheckpointAndAdvanceDrift
        )
    }

    /// Return whether any planned-vs-actual checkpoint drift was observed.
    pub fn has_checkpoint_plan_drift(self) -> bool {
        !matches!(self, Self::NoDrift)
    }

    /// Return whether planned and actual checkpoint boundaries stayed aligned.
    pub fn is_no_drift(self) -> bool {
        matches!(self, Self::NoDrift)
    }

    /// Return whether this checkpoint drift classification needs investigation.
    pub fn requires_investigation(self) -> bool {
        self.has_checkpoint_plan_drift()
    }
}

impl Display for ToolAuditSupervisorDrainCheckpointDriftKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable classification of flattened host-log integrity drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainRunIntegrityKind {
    /// All flattened host-log integrity checks are clean.
    NoDrift,
    /// Planned or replayed inventory counts drifted from payloads.
    InventoryCountDrift,
    /// Flattened run-status flags drifted from source counts.
    RunStatusDrift,
    /// Checkpoint-boundary fields drifted from source checkpoints.
    CheckpointBoundaryDrift,
    /// Stable classifier labels drifted from their typed values.
    ClassifierLabelDrift,
    /// More than one host-log integrity surface drifted.
    MultipleIntegrityDrift,
}

impl ToolAuditSupervisorDrainRunIntegrityKind {
    /// Classify host-log integrity drift from component drift flags.
    pub fn from_integrity_flags(
        has_inventory_count_integrity_drift: bool,
        has_run_status_integrity_drift: bool,
        has_checkpoint_boundary_integrity_drift: bool,
        has_classifier_label_integrity_drift: bool,
    ) -> Self {
        let drift_count = [
            has_inventory_count_integrity_drift,
            has_run_status_integrity_drift,
            has_checkpoint_boundary_integrity_drift,
            has_classifier_label_integrity_drift,
        ]
        .into_iter()
        .filter(|has_drift| *has_drift)
        .count();

        match drift_count {
            0 => Self::NoDrift,
            1 if has_inventory_count_integrity_drift => Self::InventoryCountDrift,
            1 if has_run_status_integrity_drift => Self::RunStatusDrift,
            1 if has_checkpoint_boundary_integrity_drift => Self::CheckpointBoundaryDrift,
            1 if has_classifier_label_integrity_drift => Self::ClassifierLabelDrift,
            _ => Self::MultipleIntegrityDrift,
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoDrift => "no_run_integrity_drift",
            Self::InventoryCountDrift => "inventory_count_integrity_drift",
            Self::RunStatusDrift => "run_status_integrity_drift",
            Self::CheckpointBoundaryDrift => "checkpoint_boundary_integrity_drift",
            Self::ClassifierLabelDrift => "classifier_label_integrity_drift",
            Self::MultipleIntegrityDrift => "multiple_run_integrity_drift",
        }
    }

    /// Parse a stable snake_case run-integrity label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_run_integrity_drift" => Some(Self::NoDrift),
            "inventory_count_integrity_drift" => Some(Self::InventoryCountDrift),
            "run_status_integrity_drift" => Some(Self::RunStatusDrift),
            "checkpoint_boundary_integrity_drift" => Some(Self::CheckpointBoundaryDrift),
            "classifier_label_integrity_drift" => Some(Self::ClassifierLabelDrift),
            "multiple_run_integrity_drift" => Some(Self::MultipleIntegrityDrift),
            _ => None,
        }
    }

    /// Return whether any host-log integrity drift was observed.
    pub fn has_run_integrity_drift(self) -> bool {
        !matches!(self, Self::NoDrift)
    }

    /// Return whether more than one host-log integrity surface drifted.
    pub fn has_multiple_integrity_drift(self) -> bool {
        matches!(self, Self::MultipleIntegrityDrift)
    }

    /// Return whether all flattened host-log integrity checks are clean.
    pub fn is_no_drift(self) -> bool {
        matches!(self, Self::NoDrift)
    }

    /// Return whether host-log integrity drift should be investigated.
    pub fn requires_investigation(self) -> bool {
        self.has_run_integrity_drift()
    }
}

impl Display for ToolAuditSupervisorDrainRunIntegrityKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable classification of why a host should investigate a drain run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainHostInvestigationKind {
    /// No host investigation is needed.
    NoInvestigation,
    /// The preflight plan and actual drain diverged.
    PlanDrift,
    /// Planned-vs-actual counts drifted without a plan-drift action.
    CountDrift,
    /// Both plan drift and count drift need investigation.
    PlanAndCountDrift,
}

impl ToolAuditSupervisorDrainHostInvestigationKind {
    /// Classify the required host investigation from plan and count flags.
    pub fn from_investigation_flags(
        requires_plan_drift_investigation: bool,
        requires_count_drift_investigation: bool,
    ) -> Self {
        match (
            requires_plan_drift_investigation,
            requires_count_drift_investigation,
        ) {
            (false, false) => Self::NoInvestigation,
            (true, false) => Self::PlanDrift,
            (false, true) => Self::CountDrift,
            (true, true) => Self::PlanAndCountDrift,
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoInvestigation => "no_investigation",
            Self::PlanDrift => "plan_drift",
            Self::CountDrift => "count_drift",
            Self::PlanAndCountDrift => "plan_and_count_drift",
        }
    }

    /// Parse a stable snake_case host-investigation label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_investigation" => Some(Self::NoInvestigation),
            "plan_drift" => Some(Self::PlanDrift),
            "count_drift" => Some(Self::CountDrift),
            "plan_and_count_drift" => Some(Self::PlanAndCountDrift),
            _ => None,
        }
    }

    /// Return whether plan drift should be investigated by the host.
    pub fn requires_plan_drift_investigation(self) -> bool {
        matches!(self, Self::PlanDrift | Self::PlanAndCountDrift)
    }

    /// Return whether count drift should be investigated by the host.
    pub fn requires_count_drift_investigation(self) -> bool {
        matches!(self, Self::CountDrift | Self::PlanAndCountDrift)
    }

    /// Return whether any host investigation is needed.
    pub fn requires_investigation(self) -> bool {
        !matches!(self, Self::NoInvestigation)
    }

    /// Return whether no host investigation is needed.
    pub fn is_no_investigation(self) -> bool {
        matches!(self, Self::NoInvestigation)
    }
}

impl Display for ToolAuditSupervisorDrainHostInvestigationKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable classification of why a host should pay attention to a drain run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainHostAttentionKind {
    /// No scheduler action, investigation, or integrity investigation is needed.
    NoAttention,
    /// A scheduler action is needed without other host-attention surfaces.
    SchedulerAction,
    /// Run-divergence investigation is needed without other host-attention surfaces.
    HostInvestigation,
    /// Host-log integrity investigation is needed without other host-attention surfaces.
    RunIntegrityInvestigation,
    /// Scheduler action and run-divergence investigation are both needed.
    SchedulerActionAndHostInvestigation,
    /// Scheduler action and host-log integrity investigation are both needed.
    SchedulerActionAndRunIntegrityInvestigation,
    /// Run-divergence and host-log integrity investigations are both needed.
    HostAndRunIntegrityInvestigation,
    /// Scheduler action, run-divergence investigation, and integrity investigation are needed.
    SchedulerActionHostAndRunIntegrityInvestigation,
}

impl ToolAuditSupervisorDrainHostAttentionKind {
    /// Classify host attention from scheduler, investigation, and integrity flags.
    pub fn from_attention_flags(
        requires_scheduler_action: bool,
        requires_host_investigation: bool,
        requires_run_integrity_investigation: bool,
    ) -> Self {
        match (
            requires_scheduler_action,
            requires_host_investigation,
            requires_run_integrity_investigation,
        ) {
            (false, false, false) => Self::NoAttention,
            (true, false, false) => Self::SchedulerAction,
            (false, true, false) => Self::HostInvestigation,
            (false, false, true) => Self::RunIntegrityInvestigation,
            (true, true, false) => Self::SchedulerActionAndHostInvestigation,
            (true, false, true) => Self::SchedulerActionAndRunIntegrityInvestigation,
            (false, true, true) => Self::HostAndRunIntegrityInvestigation,
            (true, true, true) => Self::SchedulerActionHostAndRunIntegrityInvestigation,
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoAttention => "no_attention",
            Self::SchedulerAction => "scheduler_action",
            Self::HostInvestigation => "host_investigation",
            Self::RunIntegrityInvestigation => "run_integrity_investigation",
            Self::SchedulerActionAndHostInvestigation => "scheduler_action_and_host_investigation",
            Self::SchedulerActionAndRunIntegrityInvestigation => {
                "scheduler_action_and_run_integrity_investigation"
            }
            Self::HostAndRunIntegrityInvestigation => "host_and_run_integrity_investigation",
            Self::SchedulerActionHostAndRunIntegrityInvestigation => {
                "scheduler_action_host_and_run_integrity_investigation"
            }
        }
    }

    /// Parse a stable snake_case host-attention label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_attention" => Some(Self::NoAttention),
            "scheduler_action" => Some(Self::SchedulerAction),
            "host_investigation" => Some(Self::HostInvestigation),
            "run_integrity_investigation" => Some(Self::RunIntegrityInvestigation),
            "scheduler_action_and_host_investigation" => {
                Some(Self::SchedulerActionAndHostInvestigation)
            }
            "scheduler_action_and_run_integrity_investigation" => {
                Some(Self::SchedulerActionAndRunIntegrityInvestigation)
            }
            "host_and_run_integrity_investigation" => Some(Self::HostAndRunIntegrityInvestigation),
            "scheduler_action_host_and_run_integrity_investigation" => {
                Some(Self::SchedulerActionHostAndRunIntegrityInvestigation)
            }
            _ => None,
        }
    }

    /// Return whether any host attention is needed.
    pub fn requires_attention(self) -> bool {
        !matches!(self, Self::NoAttention)
    }

    /// Return whether no host attention is needed.
    pub fn is_no_attention(self) -> bool {
        matches!(self, Self::NoAttention)
    }

    /// Return whether host attention includes a scheduler action.
    pub fn includes_scheduler_action(self) -> bool {
        matches!(
            self,
            Self::SchedulerAction
                | Self::SchedulerActionAndHostInvestigation
                | Self::SchedulerActionAndRunIntegrityInvestigation
                | Self::SchedulerActionHostAndRunIntegrityInvestigation
        )
    }

    /// Return whether host attention includes run-divergence investigation.
    pub fn includes_host_investigation(self) -> bool {
        matches!(
            self,
            Self::HostInvestigation
                | Self::SchedulerActionAndHostInvestigation
                | Self::HostAndRunIntegrityInvestigation
                | Self::SchedulerActionHostAndRunIntegrityInvestigation
        )
    }

    /// Return whether host attention includes host-log integrity investigation.
    pub fn includes_run_integrity_investigation(self) -> bool {
        matches!(
            self,
            Self::RunIntegrityInvestigation
                | Self::SchedulerActionAndRunIntegrityInvestigation
                | Self::HostAndRunIntegrityInvestigation
                | Self::SchedulerActionHostAndRunIntegrityInvestigation
        )
    }

    /// Return whether more than one host-attention surface is active.
    pub fn has_multiple_attention(self) -> bool {
        matches!(
            self,
            Self::SchedulerActionAndHostInvestigation
                | Self::SchedulerActionAndRunIntegrityInvestigation
                | Self::HostAndRunIntegrityInvestigation
                | Self::SchedulerActionHostAndRunIntegrityInvestigation
        )
    }
}

impl Display for ToolAuditSupervisorDrainHostAttentionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable classification of whether a host drain run is terminal-ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainTerminalReadinessKind {
    /// No rows were waiting and no host attention is needed.
    IdleReady,
    /// Rows were replayed to the current log end and no host attention is needed.
    CaughtUpReady,
    /// A scheduler action is still pending before the run is terminal-ready.
    SchedulerActionPending,
    /// Run-divergence investigation is still pending before the run is terminal-ready.
    HostInvestigationPending,
    /// Host-log integrity investigation is still pending before the run is terminal-ready.
    RunIntegrityInvestigationPending,
    /// Scheduler action and run-divergence investigation are both pending.
    SchedulerActionAndHostInvestigationPending,
    /// Scheduler action and host-log integrity investigation are both pending.
    SchedulerActionAndRunIntegrityInvestigationPending,
    /// Run-divergence and host-log integrity investigations are both pending.
    HostAndRunIntegrityInvestigationPending,
    /// Scheduler action, run-divergence investigation, and integrity investigation are pending.
    SchedulerActionHostAndRunIntegrityInvestigationPending,
}

impl ToolAuditSupervisorDrainTerminalReadinessKind {
    /// Classify terminal readiness from the run outcome and host-attention state.
    pub fn from_outcome_and_attention(
        outcome: ToolAuditSupervisorDrainRunOutcome,
        attention: ToolAuditSupervisorDrainHostAttentionKind,
    ) -> Self {
        match attention {
            ToolAuditSupervisorDrainHostAttentionKind::NoAttention if outcome.is_idle() => {
                Self::IdleReady
            }
            ToolAuditSupervisorDrainHostAttentionKind::NoAttention if outcome.is_caught_up() => {
                Self::CaughtUpReady
            }
            ToolAuditSupervisorDrainHostAttentionKind::NoAttention => Self::SchedulerActionPending,
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction => {
                Self::SchedulerActionPending
            }
            ToolAuditSupervisorDrainHostAttentionKind::HostInvestigation => {
                Self::HostInvestigationPending
            }
            ToolAuditSupervisorDrainHostAttentionKind::RunIntegrityInvestigation => {
                Self::RunIntegrityInvestigationPending
            }
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndHostInvestigation => {
                Self::SchedulerActionAndHostInvestigationPending
            }
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndRunIntegrityInvestigation => {
                Self::SchedulerActionAndRunIntegrityInvestigationPending
            }
            ToolAuditSupervisorDrainHostAttentionKind::HostAndRunIntegrityInvestigation => {
                Self::HostAndRunIntegrityInvestigationPending
            }
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionHostAndRunIntegrityInvestigation => {
                Self::SchedulerActionHostAndRunIntegrityInvestigationPending
            }
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IdleReady => "idle_ready",
            Self::CaughtUpReady => "caught_up_ready",
            Self::SchedulerActionPending => "scheduler_action_pending",
            Self::HostInvestigationPending => "host_investigation_pending",
            Self::RunIntegrityInvestigationPending => "run_integrity_investigation_pending",
            Self::SchedulerActionAndHostInvestigationPending => {
                "scheduler_action_and_host_investigation_pending"
            }
            Self::SchedulerActionAndRunIntegrityInvestigationPending => {
                "scheduler_action_and_run_integrity_investigation_pending"
            }
            Self::HostAndRunIntegrityInvestigationPending => {
                "host_and_run_integrity_investigation_pending"
            }
            Self::SchedulerActionHostAndRunIntegrityInvestigationPending => {
                "scheduler_action_host_and_run_integrity_investigation_pending"
            }
        }
    }

    /// Parse a stable snake_case terminal-readiness label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "idle_ready" => Some(Self::IdleReady),
            "caught_up_ready" => Some(Self::CaughtUpReady),
            "scheduler_action_pending" => Some(Self::SchedulerActionPending),
            "host_investigation_pending" => Some(Self::HostInvestigationPending),
            "run_integrity_investigation_pending" => Some(Self::RunIntegrityInvestigationPending),
            "scheduler_action_and_host_investigation_pending" => {
                Some(Self::SchedulerActionAndHostInvestigationPending)
            }
            "scheduler_action_and_run_integrity_investigation_pending" => {
                Some(Self::SchedulerActionAndRunIntegrityInvestigationPending)
            }
            "host_and_run_integrity_investigation_pending" => {
                Some(Self::HostAndRunIntegrityInvestigationPending)
            }
            "scheduler_action_host_and_run_integrity_investigation_pending" => {
                Some(Self::SchedulerActionHostAndRunIntegrityInvestigationPending)
            }
            _ => None,
        }
    }

    /// Return whether the run is terminal and no host attention is needed.
    pub fn is_terminal_ready(self) -> bool {
        matches!(self, Self::IdleReady | Self::CaughtUpReady)
    }

    /// Return whether no rows were waiting and no host attention is needed.
    pub fn is_idle_ready(self) -> bool {
        matches!(self, Self::IdleReady)
    }

    /// Return whether rows replayed to the log end and no host attention is needed.
    pub fn is_caught_up_ready(self) -> bool {
        matches!(self, Self::CaughtUpReady)
    }

    /// Return whether terminal readiness is blocked by scheduler action.
    pub fn requires_scheduler_action(self) -> bool {
        matches!(
            self,
            Self::SchedulerActionPending
                | Self::SchedulerActionAndHostInvestigationPending
                | Self::SchedulerActionAndRunIntegrityInvestigationPending
                | Self::SchedulerActionHostAndRunIntegrityInvestigationPending
        )
    }

    /// Return whether terminal readiness is blocked by run-divergence investigation.
    pub fn requires_host_investigation(self) -> bool {
        matches!(
            self,
            Self::HostInvestigationPending
                | Self::SchedulerActionAndHostInvestigationPending
                | Self::HostAndRunIntegrityInvestigationPending
                | Self::SchedulerActionHostAndRunIntegrityInvestigationPending
        )
    }

    /// Return whether terminal readiness is blocked by host-log integrity investigation.
    pub fn requires_run_integrity_investigation(self) -> bool {
        matches!(
            self,
            Self::RunIntegrityInvestigationPending
                | Self::SchedulerActionAndRunIntegrityInvestigationPending
                | Self::HostAndRunIntegrityInvestigationPending
                | Self::SchedulerActionHostAndRunIntegrityInvestigationPending
        )
    }

    /// Return whether terminal readiness is blocked by more than one host-attention surface.
    pub fn has_multiple_attention(self) -> bool {
        matches!(
            self,
            Self::SchedulerActionAndHostInvestigationPending
                | Self::SchedulerActionAndRunIntegrityInvestigationPending
                | Self::HostAndRunIntegrityInvestigationPending
                | Self::SchedulerActionHostAndRunIntegrityInvestigationPending
        )
    }
}

impl Display for ToolAuditSupervisorDrainTerminalReadinessKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable classification of the next host decision for a drain run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainHostDecisionKind {
    /// The run is terminal-ready and no host decision is pending.
    TerminalReady,
    /// The host should schedule another drain pass.
    ScheduleContinuation,
    /// The host should route follow-up pressure.
    RouteFollowUp,
    /// The host should investigate preflight/drain plan drift.
    InvestigatePlanDrift,
    /// The host should investigate planned-vs-replayed count drift.
    InvestigateCountDrift,
    /// The host should investigate flattened host-log integrity drift.
    InvestigateRunIntegrity,
    /// More than one host decision surface is active.
    MultipleActions,
}

impl ToolAuditSupervisorDrainHostDecisionKind {
    /// Classify the next host decision from scheduler, investigation, and integrity flags.
    pub fn from_decision_flags(
        schedules_continuation: bool,
        routes_follow_up: bool,
        investigates_plan_drift: bool,
        investigates_count_drift: bool,
        investigates_run_integrity: bool,
    ) -> Self {
        match [
            schedules_continuation,
            routes_follow_up,
            investigates_plan_drift,
            investigates_count_drift,
            investigates_run_integrity,
        ]
        .into_iter()
        .filter(|needs_decision| *needs_decision)
        .count()
        {
            0 => Self::TerminalReady,
            1 if schedules_continuation => Self::ScheduleContinuation,
            1 if routes_follow_up => Self::RouteFollowUp,
            1 if investigates_plan_drift => Self::InvestigatePlanDrift,
            1 if investigates_count_drift => Self::InvestigateCountDrift,
            1 if investigates_run_integrity => Self::InvestigateRunIntegrity,
            _ => Self::MultipleActions,
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TerminalReady => "terminal_ready",
            Self::ScheduleContinuation => "schedule_continuation",
            Self::RouteFollowUp => "route_follow_up",
            Self::InvestigatePlanDrift => "investigate_plan_drift",
            Self::InvestigateCountDrift => "investigate_count_drift",
            Self::InvestigateRunIntegrity => "investigate_run_integrity",
            Self::MultipleActions => "multiple_actions",
        }
    }

    /// Parse a stable snake_case host-decision label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "terminal_ready" => Some(Self::TerminalReady),
            "schedule_continuation" => Some(Self::ScheduleContinuation),
            "route_follow_up" => Some(Self::RouteFollowUp),
            "investigate_plan_drift" => Some(Self::InvestigatePlanDrift),
            "investigate_count_drift" => Some(Self::InvestigateCountDrift),
            "investigate_run_integrity" => Some(Self::InvestigateRunIntegrity),
            "multiple_actions" => Some(Self::MultipleActions),
            _ => None,
        }
    }

    /// Return whether the host has a pending decision to make.
    pub fn requires_decision(self) -> bool {
        !matches!(self, Self::TerminalReady)
    }

    /// Return whether the run is terminal-ready and no host decision is pending.
    pub fn is_terminal_ready(self) -> bool {
        matches!(self, Self::TerminalReady)
    }

    /// Return whether this is the single schedule-continuation decision.
    pub fn schedules_continuation(self) -> bool {
        matches!(self, Self::ScheduleContinuation)
    }

    /// Return whether this is the single route-follow-up decision.
    pub fn routes_follow_up(self) -> bool {
        matches!(self, Self::RouteFollowUp)
    }

    /// Return whether this is the single plan-drift investigation decision.
    pub fn investigates_plan_drift(self) -> bool {
        matches!(self, Self::InvestigatePlanDrift)
    }

    /// Return whether this is the single count-drift investigation decision.
    pub fn investigates_count_drift(self) -> bool {
        matches!(self, Self::InvestigateCountDrift)
    }

    /// Return whether this is the single run-integrity investigation decision.
    pub fn investigates_run_integrity(self) -> bool {
        matches!(self, Self::InvestigateRunIntegrity)
    }

    /// Return whether more than one host decision surface is active.
    pub fn has_multiple_actions(self) -> bool {
        matches!(self, Self::MultipleActions)
    }

    /// Return the dashboard lane for this host decision.
    pub fn lane(self) -> ToolAuditSupervisorDrainHostDecisionLane {
        ToolAuditSupervisorDrainHostDecisionLane::from_decision_kind(self)
    }

    /// Return the dashboard priority for this host decision.
    pub fn priority(self) -> ToolAuditSupervisorDrainHostDecisionPriority {
        ToolAuditSupervisorDrainHostDecisionPriority::from_decision_kind(self)
    }

    /// Return the queue readiness classification for this host decision.
    pub fn readiness(self) -> ToolAuditSupervisorDrainHostDecisionReadiness {
        ToolAuditSupervisorDrainHostDecisionReadiness::from_decision_kind(self)
    }

    /// Return the concrete queue route for this host decision.
    pub fn route(self) -> ToolAuditSupervisorDrainHostDecisionRoute {
        ToolAuditSupervisorDrainHostDecisionRoute::from_decision_kind(self)
    }
}

impl Display for ToolAuditSupervisorDrainHostDecisionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable dashboard lane for host drain decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainHostDecisionLane {
    /// Terminal-ready runs that need no host action.
    Terminal,
    /// Scheduler work such as continuation.
    Scheduler,
    /// Follow-up routing work.
    FollowUp,
    /// Investigation work for drift or host-log integrity.
    Investigation,
    /// Multiple lanes are active and should be grouped together.
    Mixed,
}

impl ToolAuditSupervisorDrainHostDecisionLane {
    /// Classify the dashboard lane from a host decision.
    pub fn from_decision_kind(decision: ToolAuditSupervisorDrainHostDecisionKind) -> Self {
        match decision {
            ToolAuditSupervisorDrainHostDecisionKind::TerminalReady => Self::Terminal,
            ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation => Self::Scheduler,
            ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp => Self::FollowUp,
            ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift
            | ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift
            | ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity => {
                Self::Investigation
            }
            ToolAuditSupervisorDrainHostDecisionKind::MultipleActions => Self::Mixed,
        }
    }

    /// Return a stable snake_case label for logs and dashboard grouping.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Scheduler => "scheduler",
            Self::FollowUp => "follow_up",
            Self::Investigation => "investigation",
            Self::Mixed => "mixed",
        }
    }

    /// Parse a stable snake_case host-decision lane label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "terminal" => Some(Self::Terminal),
            "scheduler" => Some(Self::Scheduler),
            "follow_up" => Some(Self::FollowUp),
            "investigation" => Some(Self::Investigation),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }

    /// Return whether this is the terminal dashboard lane.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal)
    }

    /// Return whether this is the scheduler dashboard lane.
    pub fn is_scheduler(self) -> bool {
        matches!(self, Self::Scheduler)
    }

    /// Return whether this is the follow-up dashboard lane.
    pub fn is_follow_up(self) -> bool {
        matches!(self, Self::FollowUp)
    }

    /// Return whether this is the investigation dashboard lane.
    pub fn is_investigation(self) -> bool {
        matches!(self, Self::Investigation)
    }

    /// Return whether this is the mixed dashboard lane.
    pub fn is_mixed(self) -> bool {
        matches!(self, Self::Mixed)
    }
}

impl Display for ToolAuditSupervisorDrainHostDecisionLane {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable dashboard priority for host drain decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainHostDecisionPriority {
    /// The run is settled and needs no host action.
    Settled,
    /// Routine scheduler or follow-up routing action is needed.
    RoutineAction,
    /// Drift investigation is needed.
    DriftInvestigation,
    /// Host-log integrity investigation is needed.
    IntegrityInvestigation,
    /// Multiple action surfaces are active.
    MixedAction,
}

impl ToolAuditSupervisorDrainHostDecisionPriority {
    /// Classify dashboard priority from a host decision.
    pub fn from_decision_kind(decision: ToolAuditSupervisorDrainHostDecisionKind) -> Self {
        match decision {
            ToolAuditSupervisorDrainHostDecisionKind::TerminalReady => Self::Settled,
            ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation
            | ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp => Self::RoutineAction,
            ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift
            | ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift => {
                Self::DriftInvestigation
            }
            ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity => {
                Self::IntegrityInvestigation
            }
            ToolAuditSupervisorDrainHostDecisionKind::MultipleActions => Self::MixedAction,
        }
    }

    /// Return a stable snake_case label for logs and dashboard sorting.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Settled => "settled",
            Self::RoutineAction => "routine_action",
            Self::DriftInvestigation => "drift_investigation",
            Self::IntegrityInvestigation => "integrity_investigation",
            Self::MixedAction => "mixed_action",
        }
    }

    /// Parse a stable snake_case host-decision priority label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "settled" => Some(Self::Settled),
            "routine_action" => Some(Self::RoutineAction),
            "drift_investigation" => Some(Self::DriftInvestigation),
            "integrity_investigation" => Some(Self::IntegrityInvestigation),
            "mixed_action" => Some(Self::MixedAction),
            _ => None,
        }
    }

    /// Return a stable ascending priority rank for dashboards.
    pub fn rank(self) -> u8 {
        match self {
            Self::Settled => 0,
            Self::RoutineAction => 10,
            Self::DriftInvestigation => 50,
            Self::IntegrityInvestigation => 80,
            Self::MixedAction => 90,
        }
    }

    /// Return whether this is settled priority.
    pub fn is_settled(self) -> bool {
        matches!(self, Self::Settled)
    }

    /// Return whether this is routine-action priority.
    pub fn is_routine_action(self) -> bool {
        matches!(self, Self::RoutineAction)
    }

    /// Return whether this is drift-investigation priority.
    pub fn is_drift_investigation(self) -> bool {
        matches!(self, Self::DriftInvestigation)
    }

    /// Return whether this is integrity-investigation priority.
    pub fn is_integrity_investigation(self) -> bool {
        matches!(self, Self::IntegrityInvestigation)
    }

    /// Return whether this is mixed-action priority.
    pub fn is_mixed_action(self) -> bool {
        matches!(self, Self::MixedAction)
    }

    /// Return whether the priority requires investigation.
    pub fn requires_investigation(self) -> bool {
        matches!(
            self,
            Self::DriftInvestigation | Self::IntegrityInvestigation | Self::MixedAction
        )
    }
}

impl Display for ToolAuditSupervisorDrainHostDecisionPriority {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable queue readiness classification for host drain decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainHostDecisionReadiness {
    /// The run is settled and does not need a queue entry.
    Settled,
    /// Routine scheduler or follow-up work can be routed directly.
    RoutineReady,
    /// Drift investigation is ready for an investigator queue.
    DriftInvestigationReady,
    /// Host-log integrity investigation is ready for an investigator queue.
    IntegrityInvestigationReady,
    /// Multiple active decision surfaces need human triage before routing.
    TriageRequired,
}

impl ToolAuditSupervisorDrainHostDecisionReadiness {
    /// Classify host-decision queue readiness from a host decision.
    pub fn from_decision_kind(decision: ToolAuditSupervisorDrainHostDecisionKind) -> Self {
        match decision {
            ToolAuditSupervisorDrainHostDecisionKind::TerminalReady => Self::Settled,
            ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation
            | ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp => Self::RoutineReady,
            ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift
            | ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift => {
                Self::DriftInvestigationReady
            }
            ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity => {
                Self::IntegrityInvestigationReady
            }
            ToolAuditSupervisorDrainHostDecisionKind::MultipleActions => Self::TriageRequired,
        }
    }

    /// Return a stable snake_case label for host queues and routing logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Settled => "settled",
            Self::RoutineReady => "routine_ready",
            Self::DriftInvestigationReady => "drift_investigation_ready",
            Self::IntegrityInvestigationReady => "integrity_investigation_ready",
            Self::TriageRequired => "triage_required",
        }
    }

    /// Parse a stable snake_case host-decision readiness label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "settled" => Some(Self::Settled),
            "routine_ready" => Some(Self::RoutineReady),
            "drift_investigation_ready" => Some(Self::DriftInvestigationReady),
            "integrity_investigation_ready" => Some(Self::IntegrityInvestigationReady),
            "triage_required" => Some(Self::TriageRequired),
            _ => None,
        }
    }

    /// Return whether the run is settled and needs no queue entry.
    pub fn is_settled(self) -> bool {
        matches!(self, Self::Settled)
    }

    /// Return whether this readiness should appear in a host action queue.
    pub fn is_actionable(self) -> bool {
        !self.is_settled()
    }

    /// Return whether this readiness can be routed without investigation triage.
    pub fn is_auto_routable(self) -> bool {
        matches!(self, Self::RoutineReady)
    }

    /// Return whether this readiness requires manual host review before routing.
    pub fn requires_manual_review(self) -> bool {
        matches!(
            self,
            Self::DriftInvestigationReady
                | Self::IntegrityInvestigationReady
                | Self::TriageRequired
        )
    }

    /// Return whether this readiness is already routed to an investigation queue.
    pub fn requires_investigation(self) -> bool {
        matches!(
            self,
            Self::DriftInvestigationReady | Self::IntegrityInvestigationReady
        )
    }

    /// Return whether this readiness is already routed to host-log integrity investigation.
    pub fn requires_integrity_investigation(self) -> bool {
        matches!(self, Self::IntegrityInvestigationReady)
    }

    /// Return whether multiple active decision surfaces require triage.
    pub fn requires_triage(self) -> bool {
        matches!(self, Self::TriageRequired)
    }
}

impl Display for ToolAuditSupervisorDrainHostDecisionReadiness {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable concrete queue route for host drain decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditSupervisorDrainHostDecisionRoute {
    /// The run is terminal-ready and should not enter an action queue.
    NoRoute,
    /// Scheduler continuation work should enter the scheduler queue.
    Scheduler,
    /// Follow-up pressure should enter the follow-up queue.
    FollowUp,
    /// Plan or count drift should enter the drift-investigation queue.
    DriftInvestigation,
    /// Host-log integrity drift should enter the integrity-investigation queue.
    IntegrityInvestigation,
    /// Multiple active decision surfaces should enter the triage queue.
    Triage,
}

impl ToolAuditSupervisorDrainHostDecisionRoute {
    /// Classify the concrete queue route from a host decision.
    pub fn from_decision_kind(decision: ToolAuditSupervisorDrainHostDecisionKind) -> Self {
        match decision {
            ToolAuditSupervisorDrainHostDecisionKind::TerminalReady => Self::NoRoute,
            ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation => Self::Scheduler,
            ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp => Self::FollowUp,
            ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift
            | ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift => {
                Self::DriftInvestigation
            }
            ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity => {
                Self::IntegrityInvestigation
            }
            ToolAuditSupervisorDrainHostDecisionKind::MultipleActions => Self::Triage,
        }
    }

    /// Return a stable snake_case label for host queue routing.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoRoute => "no_route",
            Self::Scheduler => "scheduler",
            Self::FollowUp => "follow_up",
            Self::DriftInvestigation => "drift_investigation",
            Self::IntegrityInvestigation => "integrity_investigation",
            Self::Triage => "triage",
        }
    }

    /// Parse a stable snake_case host-decision route label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_route" => Some(Self::NoRoute),
            "scheduler" => Some(Self::Scheduler),
            "follow_up" => Some(Self::FollowUp),
            "drift_investigation" => Some(Self::DriftInvestigation),
            "integrity_investigation" => Some(Self::IntegrityInvestigation),
            "triage" => Some(Self::Triage),
            _ => None,
        }
    }

    /// Return whether this route points at a concrete host queue.
    pub fn has_route(self) -> bool {
        !matches!(self, Self::NoRoute)
    }

    /// Return whether this route points at the scheduler queue.
    pub fn is_scheduler(self) -> bool {
        matches!(self, Self::Scheduler)
    }

    /// Return whether this route points at the follow-up queue.
    pub fn is_follow_up(self) -> bool {
        matches!(self, Self::FollowUp)
    }

    /// Return whether this route points at the drift-investigation queue.
    pub fn is_drift_investigation(self) -> bool {
        matches!(self, Self::DriftInvestigation)
    }

    /// Return whether this route points at the integrity-investigation queue.
    pub fn is_integrity_investigation(self) -> bool {
        matches!(self, Self::IntegrityInvestigation)
    }

    /// Return whether this route points at the triage queue.
    pub fn is_triage(self) -> bool {
        matches!(self, Self::Triage)
    }

    /// Return whether this route points at an investigation queue.
    pub fn is_investigation(self) -> bool {
        matches!(
            self,
            Self::DriftInvestigation | Self::IntegrityInvestigation
        )
    }
}

impl Display for ToolAuditSupervisorDrainHostDecisionRoute {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Payload-free summary for a batch audit write.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolAuditBatchWriteSummary {
    /// Number of rows attempted.
    pub attempted_records: usize,
    /// Number of rows persisted.
    pub stored_records: usize,
    /// Number of rows that failed to persist.
    pub failed_records: usize,
    /// Payload-free summary of successfully persisted rows.
    pub inventory: ToolAuditStoreInventorySummary,
    /// Storage failures captured by call id.
    pub failures: Vec<ToolAuditSinkFailure>,
}

impl ToolAuditBatchWriteSummary {
    /// Construct an empty batch summary.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return whether the batch had no rows.
    pub fn is_empty(&self) -> bool {
        self.attempted_records == 0
    }

    /// Return whether any audit rows were persisted.
    pub fn has_stored_records(&self) -> bool {
        self.stored_records > 0
    }

    /// Return whether any audit row failed to persist.
    pub fn has_storage_failures(&self) -> bool {
        self.failed_records > 0 || !self.failures.is_empty()
    }

    /// Return whether the stored row count matches the payload-free inventory.
    pub fn stored_count_matches_inventory(&self) -> bool {
        self.stored_records == self.inventory.total_records
    }

    /// Return whether the failed row count matches the captured failures.
    pub fn failed_count_matches_failures(&self) -> bool {
        self.failed_records == self.failures.len()
    }

    /// Return whether attempted rows equal stored plus failed rows.
    pub fn attempted_count_matches_results(&self) -> bool {
        self.stored_records.checked_add(self.failed_records) == Some(self.attempted_records)
    }

    /// Return whether all flattened write counts agree with their sources.
    pub fn write_counts_match(&self) -> bool {
        self.attempted_count_matches_results()
            && self.stored_count_matches_inventory()
            && self.failed_count_matches_failures()
    }

    /// Return whether flattened write counts drifted from their sources.
    pub fn has_write_count_integrity_drift(&self) -> bool {
        !self.write_counts_match()
    }

    /// Classify follow-up pressure in the persisted row inventory.
    pub fn inventory_follow_up_kind(&self) -> ToolAuditInventoryFollowUpKind {
        self.inventory.follow_up_kind()
    }

    /// Return the stable follow-up label for the persisted row inventory.
    pub fn inventory_follow_up_label(&self) -> &'static str {
        self.inventory.follow_up_label()
    }

    /// Return whether the inventory follow-up label parses back to its typed value.
    pub fn inventory_follow_up_label_matches_kind(&self) -> bool {
        self.inventory.follow_up_label_matches_kind()
    }

    /// Classify the payload-free outcome of this batch write.
    pub fn write_outcome(&self) -> ToolAuditBatchWriteOutcome {
        ToolAuditBatchWriteOutcome::from_write_summary(self)
    }

    /// Return the stable batch-write outcome label for host logs.
    pub fn write_outcome_label(&self) -> &'static str {
        self.write_outcome().as_str()
    }

    /// Return whether the batch-write outcome label parses back to its typed value.
    pub fn write_outcome_label_matches_outcome(&self) -> bool {
        ToolAuditBatchWriteOutcome::from_label(self.write_outcome_label())
            == Some(self.write_outcome())
    }

    /// Return whether every attempted row was persisted.
    pub fn completed_without_failures(&self) -> bool {
        self.attempted_records == self.stored_records && self.failed_records == 0
    }

    /// Return whether any persisted row or storage failure needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.has_storage_failures() || self.inventory.requires_follow_up()
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

    /// Classify follow-up pressure in the replayed row inventory.
    pub fn inventory_follow_up_kind(&self) -> ToolAuditInventoryFollowUpKind {
        self.inventory.follow_up_kind()
    }

    /// Return the stable follow-up label for the replayed row inventory.
    pub fn inventory_follow_up_label(&self) -> &'static str {
        self.inventory.follow_up_label()
    }

    /// Return whether the inventory follow-up label parses back to its typed value.
    pub fn inventory_follow_up_label_matches_kind(&self) -> bool {
        self.inventory.follow_up_label_matches_kind()
    }

    /// Classify the payload-free outcome of this replay.
    pub fn replay_outcome(&self) -> ToolAuditReplayOutcome {
        ToolAuditReplayOutcome::from_replay_summary(self)
    }

    /// Return the stable replay outcome label for host logs.
    pub fn replay_outcome_label(&self) -> &'static str {
        self.replay_outcome().as_str()
    }

    /// Return whether the replay outcome label parses back to its typed value.
    pub fn replay_outcome_label_matches_outcome(&self) -> bool {
        ToolAuditReplayOutcome::from_label(self.replay_outcome_label())
            == Some(self.replay_outcome())
    }

    /// Return whether any replayed row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.inventory.requires_follow_up()
    }
}

/// Payload-free outcome for a batch audit write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditBatchWriteOutcome {
    /// No audit rows were attempted.
    Empty,
    /// Every attempted row was persisted and no persisted row needs follow-up.
    CompletedClean,
    /// Every attempted row was persisted, but persisted rows need follow-up.
    CompletedWithFollowUp,
    /// One or more attempted rows failed to persist.
    StorageFailures,
    /// Flattened write counts drifted from their source values.
    CountIntegrityDrift,
}

impl ToolAuditBatchWriteOutcome {
    /// Classify a batch write summary for host dashboards.
    pub fn from_write_summary(summary: &ToolAuditBatchWriteSummary) -> Self {
        if summary.has_write_count_integrity_drift() {
            return Self::CountIntegrityDrift;
        }
        if summary.is_empty() {
            return Self::Empty;
        }
        if summary.has_storage_failures() {
            return Self::StorageFailures;
        }
        if summary.inventory.requires_follow_up() {
            return Self::CompletedWithFollowUp;
        }
        Self::CompletedClean
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::CompletedClean => "completed_clean",
            Self::CompletedWithFollowUp => "completed_with_follow_up",
            Self::StorageFailures => "storage_failures",
            Self::CountIntegrityDrift => "count_integrity_drift",
        }
    }

    /// Parse a stable snake_case batch-write outcome label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "empty" => Some(Self::Empty),
            "completed_clean" => Some(Self::CompletedClean),
            "completed_with_follow_up" => Some(Self::CompletedWithFollowUp),
            "storage_failures" => Some(Self::StorageFailures),
            "count_integrity_drift" => Some(Self::CountIntegrityDrift),
            _ => None,
        }
    }

    /// Return whether this write completed without host follow-up.
    pub fn is_completed_clean(self) -> bool {
        matches!(self, Self::CompletedClean)
    }

    /// Return whether this write needs host follow-up.
    pub fn requires_follow_up(self) -> bool {
        matches!(
            self,
            Self::CompletedWithFollowUp | Self::StorageFailures | Self::CountIntegrityDrift
        )
    }

    /// Return whether this write needs storage failure investigation.
    pub fn requires_storage_investigation(self) -> bool {
        matches!(self, Self::StorageFailures)
    }

    /// Return whether this write needs flattened count integrity investigation.
    pub fn requires_count_integrity_investigation(self) -> bool {
        matches!(self, Self::CountIntegrityDrift)
    }
}

impl Display for ToolAuditBatchWriteOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Payload-free outcome for replaying audit rows into a sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditReplayOutcome {
    /// No audit rows were replayed.
    Empty,
    /// Rows were replayed and none need follow-up.
    ReplayedClean,
    /// Rows were replayed and at least one needs follow-up.
    ReplayedWithFollowUp,
}

impl ToolAuditReplayOutcome {
    /// Classify a replay summary for host dashboards.
    pub fn from_replay_summary(summary: &ToolAuditReplaySummary) -> Self {
        if summary.is_empty() {
            return Self::Empty;
        }
        if summary.requires_follow_up() {
            return Self::ReplayedWithFollowUp;
        }
        Self::ReplayedClean
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::ReplayedClean => "replayed_clean",
            Self::ReplayedWithFollowUp => "replayed_with_follow_up",
        }
    }

    /// Parse a stable snake_case replay outcome label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "empty" => Some(Self::Empty),
            "replayed_clean" => Some(Self::ReplayedClean),
            "replayed_with_follow_up" => Some(Self::ReplayedWithFollowUp),
            _ => None,
        }
    }

    /// Return whether this replay delivered rows without follow-up pressure.
    pub fn is_replayed_clean(self) -> bool {
        matches!(self, Self::ReplayedClean)
    }

    /// Return whether this replay needs host follow-up.
    pub fn requires_follow_up(self) -> bool {
        matches!(self, Self::ReplayedWithFollowUp)
    }
}

impl Display for ToolAuditReplayOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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

    /// Persist a batch through the underlying store and keep any failures for
    /// host inspection.
    pub fn record_audit_batch<I>(&mut self, records: I) -> ToolAuditBatchWriteSummary
    where
        I: IntoIterator<Item = ToolAuditRecord>,
    {
        let summary = self.store.record_audit_batch(records);
        self.failures.extend(summary.failures.iter().cloned());
        summary
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
    /// Rows with any follow-up pressure, counted once per row.
    pub follow_up_records: usize,
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
            if audit_record_requires_follow_up(record) {
                summary.follow_up_records += 1;
            }
        }
        summary
    }

    /// Return whether there are no audit rows.
    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    /// Return whether any row is still active.
    pub fn has_active_records(&self) -> bool {
        self.active_records > 0
    }

    /// Return whether any persisted row failed.
    pub fn has_failed_records(&self) -> bool {
        self.failed_records > 0
    }

    /// Return whether any persisted row has approval pressure.
    pub fn has_approval_follow_up(&self) -> bool {
        self.approval_pending_records > 0
            || self.approval_denied_records > 0
            || self.approval_expired_records > 0
    }

    /// Return whether any persisted row recorded result errors.
    pub fn has_result_errors(&self) -> bool {
        self.records_with_errors > 0
    }

    /// Classify why this inventory needs follow-up.
    pub fn follow_up_kind(&self) -> ToolAuditInventoryFollowUpKind {
        ToolAuditInventoryFollowUpKind::from_follow_up_flags(
            self.has_active_records(),
            self.has_failed_records(),
            self.has_approval_follow_up(),
            self.has_result_errors(),
        )
    }

    /// Return the stable follow-up label for host logs.
    pub fn follow_up_label(&self) -> &'static str {
        self.follow_up_kind().as_str()
    }

    /// Return whether the follow-up label parses back to the typed classification.
    pub fn follow_up_label_matches_kind(&self) -> bool {
        ToolAuditInventoryFollowUpKind::from_label(self.follow_up_label())
            == Some(self.follow_up_kind())
    }

    /// Return whether multiple follow-up surfaces are active.
    pub fn has_multiple_follow_up_surfaces(&self) -> bool {
        self.follow_up_kind().has_multiple_follow_up_surfaces()
    }

    /// Return whether any row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.follow_up_records > 0
            || self.active_records > 0
            || self.failed_records > 0
            || self.approval_pending_records > 0
            || self.approval_denied_records > 0
            || self.approval_expired_records > 0
            || self.records_with_errors > 0
    }
}

/// Stable classification for inventory follow-up pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuditInventoryFollowUpKind {
    /// No persisted row needs host follow-up.
    NoFollowUp,
    /// At least one row is still active.
    ActiveRecords,
    /// At least one row failed.
    FailedRecords,
    /// At least one row is blocked by approval state.
    ApprovalPressure,
    /// At least one row reported a result error.
    ResultErrors,
    /// Multiple follow-up surfaces are active.
    MultipleFollowUp,
}

impl ToolAuditInventoryFollowUpKind {
    /// Classify inventory follow-up pressure from component flags.
    pub fn from_follow_up_flags(
        has_active_records: bool,
        has_failed_records: bool,
        has_approval_follow_up: bool,
        has_result_errors: bool,
    ) -> Self {
        match [
            has_active_records,
            has_failed_records,
            has_approval_follow_up,
            has_result_errors,
        ]
        .into_iter()
        .filter(|needs_follow_up| *needs_follow_up)
        .count()
        {
            0 => Self::NoFollowUp,
            1 if has_active_records => Self::ActiveRecords,
            1 if has_failed_records => Self::FailedRecords,
            1 if has_approval_follow_up => Self::ApprovalPressure,
            1 if has_result_errors => Self::ResultErrors,
            _ => Self::MultipleFollowUp,
        }
    }

    /// Return a stable snake_case label for logs and host summaries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoFollowUp => "no_follow_up",
            Self::ActiveRecords => "active_records",
            Self::FailedRecords => "failed_records",
            Self::ApprovalPressure => "approval_pressure",
            Self::ResultErrors => "result_errors",
            Self::MultipleFollowUp => "multiple_follow_up",
        }
    }

    /// Parse a stable snake_case inventory follow-up label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "no_follow_up" => Some(Self::NoFollowUp),
            "active_records" => Some(Self::ActiveRecords),
            "failed_records" => Some(Self::FailedRecords),
            "approval_pressure" => Some(Self::ApprovalPressure),
            "result_errors" => Some(Self::ResultErrors),
            "multiple_follow_up" => Some(Self::MultipleFollowUp),
            _ => None,
        }
    }

    /// Return whether this inventory needs any host follow-up.
    pub fn requires_follow_up(self) -> bool {
        !matches!(self, Self::NoFollowUp)
    }

    /// Return whether multiple follow-up surfaces are active.
    pub fn has_multiple_follow_up_surfaces(self) -> bool {
        matches!(self, Self::MultipleFollowUp)
    }
}

impl Display for ToolAuditInventoryFollowUpKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn signed_count_delta(actual: usize, planned: usize) -> i128 {
    actual as i128 - planned as i128
}

fn merge_inventory(
    target: &mut ToolAuditStoreInventorySummary,
    source: ToolAuditStoreInventorySummary,
) {
    target.total_records += source.total_records;
    target.completed_records += source.completed_records;
    target.failed_records += source.failed_records;
    target.active_records += source.active_records;
    target.approval_pending_records += source.approval_pending_records;
    target.approval_denied_records += source.approval_denied_records;
    target.approval_expired_records += source.approval_expired_records;
    target.records_with_errors += source.records_with_errors;
    target.records_with_references += source.records_with_references;
    target.follow_up_records += source.follow_up_records;
}

fn audit_record_requires_follow_up(record: &ToolAuditRecord) -> bool {
    matches!(
        record.status,
        ToolCallStatus::Queued
            | ToolCallStatus::Validating
            | ToolCallStatus::AwaitingApproval
            | ToolCallStatus::Running
            | ToolCallStatus::Failed
    ) || matches!(
        record.approval_state,
        ApprovalState::Pending | ApprovalState::Denied | ApprovalState::Expired
    ) || record.result_summary.has_error
}

fn audit_key(call_id: &str) -> String {
    format!("{AUDIT_PREFIX}{call_id}.json")
}

fn checkpoint_key(name: &str) -> Result<String, StorageError> {
    let key = format!("{CHECKPOINT_PREFIX}{name}.json");
    if name.is_empty() {
        return Err(StorageError::Validation {
            field: "checkpoint_name".to_string(),
            message: "must not be empty".to_string(),
        });
    }
    StoragePutInput::new(
        AUDIT_NAMESPACE,
        key.clone(),
        CHECKPOINT_CONTENT_TYPE,
        JsonValue::Object(Vec::new()),
        Vec::new(),
    )?;
    Ok(key)
}

fn checkpoint_put_input(
    name: &str,
    checkpoint: &ToolAuditReadCheckpoint,
    revision: Option<Revision>,
) -> Result<StoragePutInput, StorageError> {
    Ok(StoragePutInput::new(
        AUDIT_NAMESPACE,
        checkpoint_key(name)?,
        CHECKPOINT_CONTENT_TYPE,
        checkpoint_metadata(name, checkpoint),
        json_to_body(&checkpoint_to_json(name, checkpoint))?,
    )?
    .with_if_revision(revision))
}

fn audit_checkpoint_for(record: &ToolAuditRecord) -> ToolAuditReadCheckpoint {
    ToolAuditReadCheckpoint {
        timestamp_ms: audit_watermark_ms(record),
        call_id: record.call_id.clone(),
    }
}

fn compare_audit_watermarks(left: &ToolAuditRecord, right: &ToolAuditRecord) -> std::cmp::Ordering {
    (audit_watermark_ms(left), left.call_id.as_str())
        .cmp(&(audit_watermark_ms(right), right.call_id.as_str()))
}

fn audit_watermark_ms(record: &ToolAuditRecord) -> u64 {
    record
        .completed_at
        .or(record.started_at)
        .unwrap_or_default()
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

fn decode_stored_checkpoint(
    record: &StorageRecord,
) -> Result<ToolAuditStoredCheckpoint, StorageError> {
    let text = std::str::from_utf8(&record.body).map_err(|error| StorageError::Backend {
        message: format!("tool audit checkpoint body was not utf-8: {error}"),
    })?;
    let json = json_parse(text).map_err(|error| StorageError::Backend {
        message: format!("tool audit checkpoint body was not json: {error}"),
    })?;
    let object = expect_object(&json, "$")?;
    Ok(ToolAuditStoredCheckpoint {
        name: required_string(object, "name")?,
        checkpoint: ToolAuditReadCheckpoint::new(
            required_u64(object, "timestamp_ms")?,
            required_string(object, "call_id")?,
        ),
        updated_at: record.updated_at,
    })
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

fn checkpoint_metadata(name: &str, checkpoint: &ToolAuditReadCheckpoint) -> JsonValue {
    JsonValue::Object(vec![
        string_field("name", name),
        u64_field("timestamp_ms", checkpoint.timestamp_ms),
        string_field("call_id", &checkpoint.call_id),
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

fn checkpoint_to_json(name: &str, checkpoint: &ToolAuditReadCheckpoint) -> JsonValue {
    JsonValue::Object(vec![
        string_field("name", name),
        u64_field("timestamp_ms", checkpoint.timestamp_ms),
        string_field("call_id", &checkpoint.call_id),
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

fn u64_field(name: &str, value: u64) -> (String, JsonValue) {
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

fn required_u64(object: &[(String, JsonValue)], field: &'static str) -> Result<u64, StorageError> {
    optional_u64(object, field)?
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
    fn read_checkpoints_expose_scalar_accessors() {
        let beginning = ToolAuditReadCheckpoint::beginning();
        assert_eq!(beginning.checkpoint_timestamp_ms(), 0);
        assert_eq!(beginning.checkpoint_call_id(), "");

        let checkpoint = ToolAuditReadCheckpoint::new(151, "call_2");
        assert_eq!(checkpoint.checkpoint_timestamp_ms(), 151);
        assert_eq!(checkpoint.checkpoint_call_id(), "call_2");
    }

    #[test]
    fn checkpoint_pages_return_incremental_audit_rows() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                failed_record("call_2"),
                sample_record("call_1"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let first = store
            .query_audits_after_checkpoint(&ToolAuditReadCheckpoint::beginning(), Some(1))
            .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first.records[0].call_id, "call_1");
        assert_eq!(
            first.next_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_1")
        );
        assert_eq!(first.next_checkpoint_timestamp_ms(), 120);
        assert_eq!(first.next_checkpoint_call_id(), "call_1");
        assert_eq!(first.inventory.completed_records, 1);

        let second = store
            .query_audits_after_checkpoint(&first.next_checkpoint, Some(10))
            .unwrap();
        assert_eq!(
            second
                .records
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_3", "call_2"]
        );
        assert_eq!(
            second.next_checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_2")
        );
        assert_eq!(second.next_checkpoint_timestamp_ms(), 151);
        assert_eq!(second.next_checkpoint_call_id(), "call_2");

        let empty = store
            .query_audits_after_checkpoint(&second.next_checkpoint, Some(10))
            .unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.next_checkpoint, second.next_checkpoint);
        assert_eq!(empty.next_checkpoint_timestamp_ms(), 151);
        assert_eq!(empty.next_checkpoint_call_id(), "call_2");
    }

    #[test]
    fn checkpoint_pages_use_call_id_tiebreakers() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_b"), sample_record("call_a")])
            .completed_without_failures());

        let page = store
            .query_audits_after_checkpoint(&ToolAuditReadCheckpoint::new(120, "call_a"), None)
            .unwrap();

        assert_eq!(page.len(), 1);
        assert_eq!(page.records[0].call_id, "call_b");
        assert_eq!(
            page.next_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_b")
        );
        assert_eq!(page.next_checkpoint_timestamp_ms(), 120);
        assert_eq!(page.next_checkpoint_call_id(), "call_b");
    }

    #[test]
    fn named_checkpoints_are_persisted_and_resumed() {
        let root = temp_root("checkpoint-state");

        {
            let store = ToolAuditStore::new(LocalFolderStorageBackend::new(&root));
            let stored = store
                .save_checkpoint(
                    "supervisors/weather",
                    ToolAuditReadCheckpoint::new(151, "call_2"),
                )
                .unwrap();

            assert_eq!(stored.name, "supervisors/weather");
            assert_eq!(
                stored.checkpoint,
                ToolAuditReadCheckpoint::new(151, "call_2")
            );
            assert_eq!(stored.checkpoint_timestamp_ms(), 151);
            assert_eq!(stored.checkpoint_call_id(), "call_2");
        }

        {
            let store = ToolAuditStore::new(LocalFolderStorageBackend::new(&root));
            let stored = store
                .fetch_checkpoint("supervisors/weather")
                .unwrap()
                .expect("checkpoint should survive backend restart");

            assert_eq!(stored.name, "supervisors/weather");
            assert_eq!(
                stored.checkpoint,
                ToolAuditReadCheckpoint::new(151, "call_2")
            );
            assert_eq!(stored.checkpoint_timestamp_ms(), 151);
            assert_eq!(stored.checkpoint_call_id(), "call_2");
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn advancing_checkpoints_never_regresses_reader_state() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let first = store
            .advance_checkpoint("supervisor", ToolAuditReadCheckpoint::new(151, "call_2"))
            .unwrap();
        let older = store
            .advance_checkpoint("supervisor", ToolAuditReadCheckpoint::new(120, "call_1"))
            .unwrap();
        let newer = store
            .advance_checkpoint("supervisor", ToolAuditReadCheckpoint::new(151, "call_3"))
            .unwrap();

        assert_eq!(
            first.checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_2")
        );
        assert_eq!(first.checkpoint_timestamp_ms(), 151);
        assert_eq!(first.checkpoint_call_id(), "call_2");
        assert_eq!(older.checkpoint, first.checkpoint);
        assert_eq!(
            newer.checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_3")
        );
        assert_eq!(newer.checkpoint_timestamp_ms(), 151);
        assert_eq!(newer.checkpoint_call_id(), "call_3");
        assert_eq!(
            store
                .fetch_checkpoint("supervisor")
                .unwrap()
                .unwrap()
                .checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_3")
        );
    }

    #[test]
    fn stored_checkpoints_resume_incremental_audit_pages() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                failed_record("call_2"),
                sample_record("call_1"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let initial = store
            .fetch_checkpoint("supervisor")
            .unwrap()
            .map(|stored| stored.checkpoint)
            .unwrap_or_else(ToolAuditReadCheckpoint::beginning);
        let first = store
            .query_audits_after_checkpoint(&initial, Some(2))
            .unwrap();
        assert_eq!(
            first
                .records
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_1", "call_3"]
        );

        store
            .advance_checkpoint("supervisor", first.next_checkpoint.clone())
            .unwrap();
        let resumed = store
            .fetch_checkpoint("supervisor")
            .unwrap()
            .expect("checkpoint should be available for the next supervisor tick");
        let second = store
            .query_audits_after_checkpoint(&resumed.checkpoint, Some(2))
            .unwrap();

        assert_eq!(second.len(), 1);
        assert_eq!(second.records[0].call_id, "call_2");
        assert_eq!(
            second.next_checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_2")
        );
    }

    #[test]
    fn checkpointed_replay_advances_named_checkpoint() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                failed_record("call_2"),
                sample_record("call_1"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let first = store
            .replay_audits_from_checkpoint("supervisor", Some(2), &mut sink)
            .unwrap();

        assert_eq!(first.checkpoint_name, "supervisor");
        assert_eq!(
            first.starting_checkpoint,
            ToolAuditReadCheckpoint::beginning()
        );
        assert_eq!(
            first.next_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_3")
        );
        assert_eq!(first.starting_checkpoint_timestamp_ms(), 0);
        assert_eq!(first.starting_checkpoint_call_id(), "");
        assert_eq!(first.next_checkpoint_timestamp_ms(), 120);
        assert_eq!(first.next_checkpoint_call_id(), "call_3");
        assert_eq!(
            first
                .stored_checkpoint
                .as_ref()
                .map(|stored| &stored.checkpoint),
            Some(&first.next_checkpoint)
        );
        assert_eq!(first.replayed_records, 2);
        assert!(!first.requires_follow_up());
        assert_eq!(
            sink.records()
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_1", "call_3"]
        );

        let second = store
            .replay_audits_from_checkpoint("supervisor", Some(2), &mut sink)
            .unwrap();

        assert_eq!(second.starting_checkpoint, first.next_checkpoint);
        assert_eq!(
            second.next_checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_2")
        );
        assert_eq!(second.starting_checkpoint_timestamp_ms(), 120);
        assert_eq!(second.starting_checkpoint_call_id(), "call_3");
        assert_eq!(second.next_checkpoint_timestamp_ms(), 151);
        assert_eq!(second.next_checkpoint_call_id(), "call_2");
        assert_eq!(second.replayed_records, 1);
        assert!(second.requires_follow_up());
        assert_eq!(
            sink.records()
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_1", "call_3", "call_2"]
        );
    }

    #[test]
    fn checkpointed_replay_preserves_empty_checkpoint_state() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut sink = InMemoryToolAuditSink::new();

        let empty = store
            .replay_audits_from_checkpoint("supervisor", Some(10), &mut sink)
            .unwrap();

        assert!(empty.is_empty());
        assert_eq!(
            empty.starting_checkpoint,
            ToolAuditReadCheckpoint::beginning()
        );
        assert_eq!(empty.next_checkpoint, ToolAuditReadCheckpoint::beginning());
        assert_eq!(empty.starting_checkpoint_timestamp_ms(), 0);
        assert_eq!(empty.starting_checkpoint_call_id(), "");
        assert_eq!(empty.next_checkpoint_timestamp_ms(), 0);
        assert_eq!(empty.next_checkpoint_call_id(), "");
        assert_eq!(empty.stored_checkpoint, None);
        assert!(sink.records().is_empty());
        assert!(store.fetch_checkpoint("supervisor").unwrap().is_none());
    }

    #[test]
    fn supervisor_checkpoint_status_reports_pending_without_advancing() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                failed_record("call_3"),
                sample_record("call_1"),
                sample_record("call_2"),
            ])
            .completed_without_failures());
        store
            .save_checkpoint("supervisor", ToolAuditReadCheckpoint::new(120, "call_1"))
            .unwrap();

        let status = store
            .inspect_supervisor_checkpoint("supervisor", 2)
            .unwrap();

        assert_eq!(status.checkpoint_name, "supervisor");
        assert_eq!(status.max_records, 2);
        assert_eq!(
            status.starting_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_1")
        );
        assert_eq!(
            status.next_checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_3")
        );
        assert_eq!(status.pending_records, 2);
        assert!(status.has_pending_records());
        assert!(status.should_drain());
        assert!(status.should_continue_after_page());
        assert!(status.requires_follow_up());
        assert!(status.would_advance_checkpoint());
        assert_eq!(status.starting_checkpoint_timestamp_ms(), 120);
        assert_eq!(status.starting_checkpoint_call_id(), "call_1");
        assert_eq!(status.next_checkpoint_timestamp_ms(), 151);
        assert_eq!(status.next_checkpoint_call_id(), "call_3");
        assert_eq!(status.inventory.total_records, 2);
        assert_eq!(
            status
                .stored_checkpoint
                .as_ref()
                .map(|stored| &stored.checkpoint),
            Some(&ToolAuditReadCheckpoint::new(120, "call_1"))
        );
        assert_eq!(
            store
                .fetch_checkpoint("supervisor")
                .unwrap()
                .map(|stored| stored.checkpoint),
            Some(ToolAuditReadCheckpoint::new(120, "call_1"))
        );
    }

    #[test]
    fn supervisor_checkpoint_status_reports_idle_without_creating_checkpoint() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());

        let status = store
            .inspect_supervisor_checkpoint("supervisor", 10)
            .unwrap();

        assert_eq!(status.checkpoint_name, "supervisor");
        assert_eq!(status.max_records, 10);
        assert_eq!(
            status.starting_checkpoint,
            ToolAuditReadCheckpoint::beginning()
        );
        assert_eq!(status.next_checkpoint, ToolAuditReadCheckpoint::beginning());
        assert_eq!(status.pending_records, 0);
        assert!(status.is_idle());
        assert!(!status.has_pending_records());
        assert!(!status.should_drain());
        assert!(!status.should_continue_after_page());
        assert!(!status.requires_follow_up());
        assert!(!status.would_advance_checkpoint());
        assert_eq!(status.starting_checkpoint_timestamp_ms(), 0);
        assert_eq!(status.starting_checkpoint_call_id(), "");
        assert_eq!(status.next_checkpoint_timestamp_ms(), 0);
        assert_eq!(status.next_checkpoint_call_id(), "");
        assert_eq!(status.stored_checkpoint, None);
        assert!(store.fetch_checkpoint("supervisor").unwrap().is_none());
    }

    #[test]
    fn supervisor_checkpoint_status_rejects_zero_record_pages() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());

        let error = store
            .inspect_supervisor_checkpoint("supervisor", 0)
            .unwrap_err();

        assert!(error.to_string().contains("max_records"));
        assert!(store.fetch_checkpoint("supervisor").unwrap().is_none());
    }

    #[test]
    fn supervisor_drain_plan_reports_budgeted_pages_without_advancing() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                failed_record("call_5"),
                sample_record("call_3"),
                sample_record("call_4"),
            ])
            .completed_without_failures());
        store
            .save_checkpoint("supervisor", ToolAuditReadCheckpoint::new(120, "call_1"))
            .unwrap();

        let plan = store
            .plan_supervisor_checkpoint_drain("supervisor", 2, 2)
            .unwrap();

        assert_eq!(plan.checkpoint_name, "supervisor");
        assert_eq!(plan.max_records_per_tick, 2);
        assert_eq!(plan.max_ticks, 2);
        assert_eq!(plan.page_count(), 2);
        assert_eq!(plan.remaining_tick_budget(), 0);
        assert!(plan.used_all_ticks());
        assert!(!plan.has_remaining_tick_budget());
        assert_eq!(plan.planned_records, 4);
        assert_eq!(plan.inventory.total_records, 4);
        assert_eq!(plan.inventory.failed_records, 1);
        assert_eq!(plan.inventory.follow_up_records, 1);
        assert!(plan.has_pending_records());
        assert!(plan.requires_follow_up());
        assert!(plan.would_advance_checkpoint());
        assert!(plan.exhausted_tick_budget());
        assert!(plan.should_continue());
        assert!(!plan.reached_end_of_log());
        assert_eq!(
            plan.starting_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_1")
        );
        assert_eq!(
            plan.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(151, "call_5"))
        );
        assert_eq!(plan.pages[0].pending_records, 2);
        assert!(plan.pages[0].has_pending_records());
        assert!(plan.pages[0].should_drain());
        assert!(plan.pages[0].should_continue_after_page());
        assert!(plan.pages[0].would_advance_checkpoint());
        assert_eq!(plan.pages[0].starting_checkpoint_timestamp_ms(), 120);
        assert_eq!(plan.pages[0].starting_checkpoint_call_id(), "call_1");
        assert_eq!(plan.pages[0].next_checkpoint_timestamp_ms(), 120);
        assert_eq!(plan.pages[0].next_checkpoint_call_id(), "call_3");
        assert_eq!(
            plan.pages[0].next_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_3")
        );
        assert_eq!(plan.pages[1].pending_records, 2);
        assert_eq!(plan.pages[1].inventory.follow_up_records, 1);
        assert!(plan.pages[1].should_drain());
        assert!(plan.pages[1].should_continue_after_page());
        assert!(plan.pages[1].requires_follow_up());
        assert!(plan.pages[1].would_advance_checkpoint());
        assert_eq!(plan.pages[1].starting_checkpoint_timestamp_ms(), 120);
        assert_eq!(plan.pages[1].starting_checkpoint_call_id(), "call_3");
        assert_eq!(plan.pages[1].next_checkpoint_timestamp_ms(), 151);
        assert_eq!(plan.pages[1].next_checkpoint_call_id(), "call_5");
        assert_eq!(
            store
                .fetch_checkpoint("supervisor")
                .unwrap()
                .map(|stored| stored.checkpoint),
            Some(ToolAuditReadCheckpoint::new(120, "call_1"))
        );
    }

    #[test]
    fn supervisor_drain_plan_can_confirm_exact_page_end_with_idle_page() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
                sample_record("call_4"),
            ])
            .completed_without_failures());

        let plan = store
            .plan_supervisor_checkpoint_drain("supervisor", 2, 3)
            .unwrap();

        assert_eq!(plan.page_count(), 3);
        assert_eq!(plan.remaining_tick_budget(), 0);
        assert!(plan.used_all_ticks());
        assert!(!plan.has_remaining_tick_budget());
        assert_eq!(plan.planned_records, 4);
        assert!(plan.reached_end_of_log());
        assert!(!plan.exhausted_tick_budget());
        assert!(!plan.should_continue());
        assert!(!plan.pages[2].has_pending_records());
        assert!(!plan.pages[2].should_drain());
        assert!(!plan.pages[2].should_continue_after_page());
        assert!(!plan.pages[2].would_advance_checkpoint());
        assert_eq!(plan.pages[2].starting_checkpoint_timestamp_ms(), 120);
        assert_eq!(plan.pages[2].starting_checkpoint_call_id(), "call_4");
        assert_eq!(plan.pages[2].next_checkpoint_timestamp_ms(), 120);
        assert_eq!(plan.pages[2].next_checkpoint_call_id(), "call_4");
        assert_eq!(
            plan.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(120, "call_4"))
        );
        assert!(store.fetch_checkpoint("supervisor").unwrap().is_none());
    }

    #[test]
    fn supervisor_drain_plan_reports_idle_without_creating_checkpoint() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());

        let plan = store
            .plan_supervisor_checkpoint_drain("supervisor", 10, 3)
            .unwrap();

        assert_eq!(plan.page_count(), 1);
        assert_eq!(plan.remaining_tick_budget(), 2);
        assert!(!plan.used_all_ticks());
        assert!(plan.has_remaining_tick_budget());
        assert_eq!(plan.planned_records, 0);
        assert!(plan.is_idle());
        assert!(!plan.has_pending_records());
        assert!(!plan.pages[0].should_drain());
        assert!(!plan.pages[0].should_continue_after_page());
        assert_eq!(plan.pages[0].starting_checkpoint_timestamp_ms(), 0);
        assert_eq!(plan.pages[0].starting_checkpoint_call_id(), "");
        assert_eq!(plan.pages[0].next_checkpoint_timestamp_ms(), 0);
        assert_eq!(plan.pages[0].next_checkpoint_call_id(), "");
        assert!(!plan.requires_follow_up());
        assert!(!plan.pages[0].would_advance_checkpoint());
        assert!(!plan.would_advance_checkpoint());
        assert!(plan.reached_end_of_log());
        assert!(!plan.exhausted_tick_budget());
        assert_eq!(plan.stored_checkpoint, None);
        assert_eq!(
            plan.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::beginning())
        );
        assert!(store.fetch_checkpoint("supervisor").unwrap().is_none());
    }

    #[test]
    fn supervisor_drain_plan_rejects_zero_limits() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());

        let zero_records = store
            .plan_supervisor_checkpoint_drain("supervisor", 0, 1)
            .unwrap_err();
        assert!(zero_records.to_string().contains("max_records_per_tick"));

        let zero_ticks = store
            .plan_supervisor_checkpoint_drain("supervisor", 1, 0)
            .unwrap_err();
        assert!(zero_ticks.to_string().contains("max_ticks"));
        assert!(store.fetch_checkpoint("supervisor").unwrap().is_none());
    }

    #[test]
    fn supervisor_drain_reports_progress_and_continuation() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                failed_record("call_2"),
                sample_record("call_1"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let first = store
            .drain_supervisor_checkpoint("supervisor", 2, &mut sink)
            .unwrap();

        assert_eq!(first.max_records, 2);
        assert!(first.made_progress());
        assert!(first.advanced_checkpoint());
        assert!(first.should_continue());
        assert!(!first.reached_end_of_log);
        assert!(!first.requires_follow_up());
        assert_eq!(first.replay.replayed_records, 2);
        assert_eq!(
            first.replay.next_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_3")
        );

        let second = store
            .drain_supervisor_checkpoint("supervisor", 2, &mut sink)
            .unwrap();

        assert!(second.made_progress());
        assert!(!second.should_continue());
        assert!(second.reached_end_of_log);
        assert!(second.requires_follow_up());
        assert_eq!(second.replay.replayed_records, 1);
        assert_eq!(
            second.replay.next_checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_2")
        );
        assert_eq!(
            sink.records()
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_1", "call_3", "call_2"]
        );
    }

    #[test]
    fn supervisor_drain_reports_idle_end_of_log() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut sink = InMemoryToolAuditSink::new();

        let drain = store
            .drain_supervisor_checkpoint("supervisor", 10, &mut sink)
            .unwrap();

        assert!(drain.is_idle());
        assert!(!drain.made_progress());
        assert!(!drain.advanced_checkpoint());
        assert!(!drain.should_continue());
        assert!(drain.reached_end_of_log);
        assert!(drain.replay.stored_checkpoint.is_none());
    }

    #[test]
    fn supervisor_drain_rejects_zero_record_ticks() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut sink = InMemoryToolAuditSink::new();

        let error = store
            .drain_supervisor_checkpoint("supervisor", 0, &mut sink)
            .unwrap_err();

        assert!(error.to_string().contains("max_records"));
        assert!(sink.records().is_empty());
    }

    #[test]
    fn supervisor_drain_loop_stops_when_checkpoint_catches_up() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                failed_record("call_3"),
                sample_record("call_1"),
                sample_record("call_2"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let summary = store
            .drain_supervisor_checkpoint_loop("supervisor", 2, 3, &mut sink)
            .unwrap();

        assert_eq!(summary.max_records_per_tick, 2);
        assert_eq!(summary.max_ticks, 3);
        assert_eq!(summary.tick_count(), 2);
        assert_eq!(summary.remaining_tick_budget(), 1);
        assert!(!summary.used_all_ticks());
        assert!(summary.has_remaining_tick_budget());
        assert_eq!(summary.drained_records, 3);
        assert!(summary.made_progress());
        assert!(summary.advanced_checkpoint());
        assert!(summary.reached_end_of_log());
        assert!(!summary.exhausted_tick_budget());
        assert!(!summary.should_continue());
        assert!(summary.requires_follow_up());
        assert_eq!(
            summary.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(151, "call_3"))
        );
        assert_eq!(
            sink.records()
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_1", "call_2", "call_3"]
        );
    }

    #[test]
    fn supervisor_drain_loop_can_confirm_exact_page_end_with_idle_tick() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
                sample_record("call_4"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let summary = store
            .drain_supervisor_checkpoint_loop("supervisor", 2, 3, &mut sink)
            .unwrap();

        assert_eq!(summary.tick_count(), 3);
        assert_eq!(summary.remaining_tick_budget(), 0);
        assert!(summary.used_all_ticks());
        assert!(!summary.has_remaining_tick_budget());
        assert_eq!(summary.drained_records, 4);
        assert!(summary.reached_end_of_log());
        assert!(!summary.exhausted_tick_budget());
        assert!(!summary.ticks[2].made_progress());
        assert_eq!(
            summary.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(120, "call_4"))
        );
        assert_eq!(sink.records().len(), 4);
    }

    #[test]
    fn supervisor_drain_loop_reports_tick_budget_exhaustion() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
                sample_record("call_4"),
                sample_record("call_5"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let summary = store
            .drain_supervisor_checkpoint_loop("supervisor", 2, 2, &mut sink)
            .unwrap();

        assert_eq!(summary.tick_count(), 2);
        assert_eq!(summary.remaining_tick_budget(), 0);
        assert!(summary.used_all_ticks());
        assert!(!summary.has_remaining_tick_budget());
        assert_eq!(summary.drained_records, 4);
        assert!(!summary.reached_end_of_log());
        assert!(summary.exhausted_tick_budget());
        assert!(summary.should_continue());
        assert!(summary.advanced_checkpoint());
        assert_eq!(
            summary.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(120, "call_4"))
        );
        assert_eq!(
            store
                .fetch_checkpoint("supervisor")
                .unwrap()
                .map(|stored| stored.checkpoint),
            Some(ToolAuditReadCheckpoint::new(120, "call_4"))
        );
        assert_eq!(sink.records().len(), 4);
    }

    #[test]
    fn supervisor_drain_report_captures_preflight_and_actual_drain() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
                sample_record("call_4"),
                sample_record("call_5"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 2, &mut sink)
            .unwrap();

        assert_eq!(report.plan.max_records_per_tick, 2);
        assert_eq!(report.max_records_per_tick(), 2);
        assert_eq!(report.plan.max_ticks, 2);
        assert_eq!(report.max_ticks(), 2);
        assert_eq!(report.plan.page_count(), 2);
        assert_eq!(report.planned_pages(), 2);
        assert_eq!(report.plan.planned_records, 4);
        assert_eq!(report.planned_records(), 4);
        assert_eq!(report.drain.tick_count(), 2);
        assert_eq!(report.drain_ticks(), 2);
        assert_eq!(report.drain.drained_records, 4);
        assert_eq!(report.drained_records(), 4);
        assert_eq!(report.checkpoint_name(), "supervisor");
        assert_eq!(report.planned_follow_up_records(), 0);
        assert_eq!(report.drained_follow_up_records(), 0);
        assert!(report.matches_planned_record_count());
        assert!(report.made_progress());
        assert!(report.advanced_checkpoint());
        assert!(report.exhausted_tick_budget());
        assert!(report.should_continue());
        assert!(!report.reached_end_of_log());
        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::NeedsContinuation
        );
        assert_eq!(
            report.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(120, "call_4"))
        );
        assert!(report.has_last_checkpoint());
        assert_eq!(report.last_checkpoint_timestamp_ms(), Some(120));
        assert_eq!(report.last_checkpoint_call_id(), Some("call_4"));
        assert_eq!(sink.records().len(), 4);
        assert_eq!(
            store
                .fetch_checkpoint("supervisor")
                .unwrap()
                .map(|stored| stored.checkpoint),
            Some(ToolAuditReadCheckpoint::new(120, "call_4"))
        );
    }

    #[test]
    fn supervisor_drain_report_propagates_follow_up_pressure() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                failed_record("call_3"),
                sample_record("call_1"),
                sample_record("call_2"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 3, &mut sink)
            .unwrap();

        assert_eq!(report.plan.page_count(), 2);
        assert_eq!(report.plan.planned_records, 3);
        assert_eq!(report.drain.drained_records, 3);
        assert_eq!(report.plan.follow_up_record_count(), 1);
        assert_eq!(report.planned_follow_up_records(), 1);
        assert_eq!(report.drain.follow_up_record_count(), 1);
        assert_eq!(report.drained_follow_up_records(), 1);
        assert!(report.matches_planned_record_count());
        assert!(report.matches_record_count());
        assert!(report.matches_planned_follow_up_record_count());
        assert!(report.matches_follow_up_pressure());
        assert_eq!(report.record_count_delta(), 0);
        assert_eq!(report.follow_up_record_count_delta(), 0);
        assert!(!report.replayed_extra_records());
        assert!(!report.missed_planned_records());
        assert!(!report.replayed_extra_follow_up_records());
        assert!(!report.missed_planned_follow_up_records());
        assert!(!report.has_record_count_drift());
        assert!(!report.has_follow_up_record_count_drift());
        assert!(!report.has_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::NoDrift
        );
        assert_eq!(report.count_drift_label(), "no_count_drift");
        assert!(!report.requires_count_drift_investigation());
        assert!(!report.requires_plan_drift_investigation());
        assert_eq!(
            report.host_investigation_kind(),
            ToolAuditSupervisorDrainHostInvestigationKind::NoInvestigation
        );
        assert_eq!(report.host_investigation_label(), "no_investigation");
        assert!(!report.requires_host_investigation());
        assert!(!report.requires_host_plan_drift_investigation());
        assert!(!report.requires_host_count_drift_investigation());
        assert!(report.requires_follow_up());
        assert!(report.reached_end_of_log());
        assert!(!report.exhausted_tick_budget());
        assert!(!report.should_continue());
        assert!(!report.is_idle());
        assert!(report.made_progress());
        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::NeedsFollowUp
        );
        assert!(!report.is_caught_up());
        assert!(!report.needs_continuation());
        assert!(report.needs_follow_up());
        assert!(!report.plan_diverged());
        let summary = report.summary();
        assert_eq!(summary.outcome_label, "needs_follow_up");
        assert_eq!(summary.outcome_label(), "needs_follow_up");
        assert!(!summary.is_caught_up);
        assert!(!summary.is_caught_up());
        assert!(!summary.needs_continuation);
        assert!(!summary.needs_continuation());
        assert!(summary.needs_follow_up);
        assert!(summary.needs_follow_up());
        assert!(!summary.plan_diverged);
        assert!(!summary.plan_diverged());
        assert_eq!(summary.scheduler_action_label, "route_follow_up");
        assert_eq!(summary.scheduler_action_label(), "route_follow_up");
        assert!(!summary.is_no_scheduler_action);
        assert!(!summary.is_no_scheduler_action());
        assert!(summary.requires_scheduler_action);
        assert!(summary.requires_scheduler_action());
        assert!(!summary.requests_continuation);
        assert!(!summary.requests_continuation());
        assert!(summary.routes_follow_up);
        assert!(summary.routes_follow_up());
        assert!(!summary.requires_plan_drift_investigation);
        assert!(!summary.requires_plan_drift_investigation());
        assert_eq!(summary.planned_follow_up_records, 1);
        assert_eq!(summary.drained_follow_up_records, 1);
        assert!(summary.matches_planned_follow_up_record_count);
        assert!(summary.matches_follow_up_pressure());
        assert_eq!(summary.record_count_delta, 0);
        assert_eq!(summary.follow_up_record_count_delta, 0);
        assert!(!summary.has_record_count_drift);
        assert!(!summary.has_record_count_drift());
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift());
        assert!(!summary.has_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::NoDrift
        );
        assert_eq!(summary.count_drift_label(), "no_count_drift");
        assert!(!summary.requires_count_drift_investigation);
        assert!(!summary.requires_count_drift_investigation());
        assert_eq!(
            summary.host_investigation_kind,
            ToolAuditSupervisorDrainHostInvestigationKind::NoInvestigation
        );
        assert_eq!(summary.host_investigation_label(), "no_investigation");
        assert!(!summary.requires_host_investigation);
        assert!(!summary.requires_host_investigation());
        assert!(summary.is_no_host_investigation);
        assert!(summary.is_no_host_investigation());
        assert!(!summary.requires_host_plan_drift_investigation);
        assert!(!summary.requires_host_plan_drift_investigation());
        assert!(!summary.requires_host_count_drift_investigation);
        assert!(!summary.requires_host_count_drift_investigation());
        assert!(!summary.replayed_extra_records());
        assert!(!summary.missed_planned_records());
        assert!(!summary.replayed_extra_follow_up_records());
        assert!(!summary.missed_planned_follow_up_records());
        assert_eq!(
            report.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(151, "call_3"))
        );
        assert_eq!(
            sink.records()
                .iter()
                .map(|record| record.call_id.as_str())
                .collect::<Vec<_>>(),
            vec!["call_1", "call_2", "call_3"]
        );
    }

    #[test]
    fn supervisor_drain_report_reports_idle_without_creating_checkpoint() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut sink = InMemoryToolAuditSink::new();

        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 3, &mut sink)
            .unwrap();

        assert_eq!(report.plan.page_count(), 1);
        assert_eq!(report.plan.planned_records, 0);
        assert_eq!(report.drain.tick_count(), 1);
        assert_eq!(report.drain.drained_records, 0);
        assert!(report.matches_planned_record_count());
        assert!(report.is_idle());
        assert!(!report.made_progress());
        assert!(!report.advanced_checkpoint());
        assert!(report.reached_end_of_log());
        assert!(!report.should_continue());
        assert_eq!(report.outcome(), ToolAuditSupervisorDrainRunOutcome::Idle);
        assert!(!report.is_caught_up());
        assert!(!report.needs_continuation());
        assert!(!report.needs_follow_up());
        assert!(!report.plan_diverged());
        assert!(report.is_no_scheduler_action());
        assert!(!report.requires_scheduler_action());
        let summary = report.summary();
        assert_eq!(summary.scheduler_action_label, "no_action");
        assert!(!summary.is_caught_up());
        assert!(!summary.needs_continuation());
        assert!(!summary.needs_follow_up());
        assert!(!summary.plan_diverged());
        assert!(summary.is_no_scheduler_action);
        assert!(summary.is_no_scheduler_action());
        assert!(!summary.requires_scheduler_action);
        assert!(!summary.requires_scheduler_action());
        assert!(report.last_checkpoint().is_some());
        assert!(sink.records().is_empty());
        assert!(store.fetch_checkpoint("supervisor").unwrap().is_none());
    }

    #[test]
    fn supervisor_drain_report_outcome_reports_caught_up_progress() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();

        assert!(report.matches_planned_record_count());
        assert!(report.made_progress());
        assert!(report.reached_end_of_log());
        assert!(!report.requires_follow_up());
        assert!(!report.should_continue());
        assert_eq!(report.remaining_tick_budget(), 1);
        assert!(!report.used_all_ticks());
        assert!(report.has_remaining_tick_budget());
        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::CaughtUp
        );
        assert!(report.is_caught_up());
        assert!(!report.needs_continuation());
        assert!(!report.needs_follow_up());
        assert!(!report.plan_diverged());
        let summary = report.summary();
        assert!(summary.is_caught_up);
        assert!(summary.is_caught_up());
        assert!(!summary.needs_continuation);
        assert!(!summary.needs_continuation());
        assert!(!summary.needs_follow_up);
        assert!(!summary.needs_follow_up());
        assert!(!summary.plan_diverged);
        assert!(!summary.plan_diverged());
    }

    #[test]
    fn supervisor_drain_report_outcome_detects_plan_drift() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let mut report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();
        report.drain.drained_records += 1;

        assert!(!report.matches_planned_record_count());
        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::PlanDiverged
        );
    }

    #[test]
    fn supervisor_drain_outcome_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainRunOutcome::Idle,
                "idle",
                true,
                false,
                false,
                false,
                false,
                ToolAuditSupervisorDrainSchedulerAction::NoAction,
                "no_action",
                false,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::CaughtUp,
                "caught_up",
                false,
                true,
                false,
                false,
                false,
                ToolAuditSupervisorDrainSchedulerAction::NoAction,
                "no_action",
                false,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::NeedsContinuation,
                "needs_continuation",
                false,
                false,
                true,
                false,
                false,
                ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation,
                "schedule_continuation",
                true,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::NeedsFollowUp,
                "needs_follow_up",
                false,
                false,
                false,
                true,
                false,
                ToolAuditSupervisorDrainSchedulerAction::RouteFollowUp,
                "route_follow_up",
                true,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::PlanDiverged,
                "plan_diverged",
                false,
                false,
                false,
                false,
                true,
                ToolAuditSupervisorDrainSchedulerAction::InvestigatePlanDrift,
                "investigate_plan_drift",
                true,
                false,
                false,
                false,
                true,
            ),
        ];

        for (
            outcome,
            label,
            is_idle,
            is_caught_up,
            needs_continuation,
            needs_follow_up,
            plan_diverged,
            scheduler_action,
            scheduler_action_label,
            requires_action,
            is_no_action,
            requests_continuation,
            routes_follow_up,
            requires_plan_drift_investigation,
        ) in cases
        {
            assert_eq!(outcome.as_str(), label);
            assert_eq!(outcome.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainRunOutcome::from_label(label),
                Some(outcome)
            );
            assert_eq!(outcome.is_idle(), is_idle);
            assert_eq!(outcome.is_caught_up(), is_caught_up);
            assert_eq!(outcome.needs_continuation(), needs_continuation);
            assert_eq!(outcome.needs_follow_up(), needs_follow_up);
            assert_eq!(outcome.plan_diverged(), plan_diverged);
            assert_eq!(outcome.scheduler_action(), scheduler_action);
            assert_eq!(outcome.scheduler_action_label(), scheduler_action_label);
            assert_eq!(outcome.requires_scheduler_action(), requires_action);
            assert_eq!(outcome.is_no_scheduler_action(), is_no_action);
            assert_eq!(outcome.requests_continuation(), requests_continuation);
            assert_eq!(outcome.routes_follow_up(), routes_follow_up);
            assert_eq!(
                outcome.requires_plan_drift_investigation(),
                requires_plan_drift_investigation
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainRunOutcome::from_label("needs_attention"),
            None
        );
    }

    #[test]
    fn supervisor_drain_scheduler_action_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainSchedulerAction::NoAction,
                "no_action",
                false,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation,
                "schedule_continuation",
                true,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainSchedulerAction::RouteFollowUp,
                "route_follow_up",
                true,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainSchedulerAction::InvestigatePlanDrift,
                "investigate_plan_drift",
                true,
                false,
                false,
                false,
                true,
            ),
        ];

        for (
            action,
            label,
            requires_action,
            is_no_action,
            requests_continuation,
            routes_follow_up,
            requires_plan_drift_investigation,
        ) in cases
        {
            assert_eq!(action.as_str(), label);
            assert_eq!(action.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainSchedulerAction::from_label(label),
                Some(action)
            );
            assert_eq!(action.requires_scheduler_action(), requires_action);
            assert_eq!(action.is_no_action(), is_no_action);
            assert_eq!(action.requests_continuation(), requests_continuation);
            assert_eq!(action.routes_follow_up(), routes_follow_up);
            assert_eq!(
                action.requires_plan_drift_investigation(),
                requires_plan_drift_investigation
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainSchedulerAction::from_label("rerun_everything"),
            None
        );
    }

    #[test]
    fn supervisor_drain_count_drift_kind_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainCountDriftKind::NoDrift,
                "no_count_drift",
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift,
                "record_count_drift",
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainCountDriftKind::FollowUpRecordCountDrift,
                "follow_up_record_count_drift",
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainCountDriftKind::RecordAndFollowUpRecordCountDrift,
                "record_and_follow_up_record_count_drift",
                true,
                true,
            ),
        ];

        for (kind, label, has_record_count_drift, has_follow_up_record_count_drift) in cases {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainCountDriftKind::from_label(label),
                Some(kind)
            );
            assert_eq!(
                ToolAuditSupervisorDrainCountDriftKind::from_drift_flags(
                    has_record_count_drift,
                    has_follow_up_record_count_drift
                ),
                kind
            );
            assert_eq!(kind.has_record_count_drift(), has_record_count_drift);
            assert_eq!(
                kind.has_follow_up_record_count_drift(),
                has_follow_up_record_count_drift
            );
            assert_eq!(
                kind.has_count_drift(),
                has_record_count_drift || has_follow_up_record_count_drift
            );
            assert_eq!(
                kind.is_no_drift(),
                !(has_record_count_drift || has_follow_up_record_count_drift)
            );
            assert_eq!(
                kind.requires_investigation(),
                has_record_count_drift || has_follow_up_record_count_drift
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainCountDriftKind::from_label("driftish"),
            None
        );
    }

    #[test]
    fn supervisor_drain_checkpoint_drift_kind_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift,
                "no_checkpoint_drift",
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainCheckpointDriftKind::PlannedLastCheckpointDrift,
                "planned_last_checkpoint_drift",
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainCheckpointDriftKind::CheckpointAdvanceDrift,
                "checkpoint_advance_drift",
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainCheckpointDriftKind::PlannedLastCheckpointAndAdvanceDrift,
                "planned_last_checkpoint_and_advance_drift",
                true,
                true,
            ),
        ];

        for (kind, label, has_planned_last_checkpoint_drift, has_checkpoint_advance_drift) in cases
        {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainCheckpointDriftKind::from_label(label),
                Some(kind)
            );
            assert_eq!(
                ToolAuditSupervisorDrainCheckpointDriftKind::from_drift_flags(
                    has_planned_last_checkpoint_drift,
                    has_checkpoint_advance_drift
                ),
                kind
            );
            assert_eq!(
                kind.has_planned_last_checkpoint_drift(),
                has_planned_last_checkpoint_drift
            );
            assert_eq!(
                kind.has_checkpoint_advance_drift(),
                has_checkpoint_advance_drift
            );
            assert_eq!(
                kind.has_checkpoint_plan_drift(),
                has_planned_last_checkpoint_drift || has_checkpoint_advance_drift
            );
            assert_eq!(
                kind.is_no_drift(),
                !(has_planned_last_checkpoint_drift || has_checkpoint_advance_drift)
            );
            assert_eq!(
                kind.requires_investigation(),
                has_planned_last_checkpoint_drift || has_checkpoint_advance_drift
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainCheckpointDriftKind::from_label("checkpointish"),
            None
        );
    }

    #[test]
    fn supervisor_drain_run_integrity_kind_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainRunIntegrityKind::NoDrift,
                "no_run_integrity_drift",
                false,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunIntegrityKind::InventoryCountDrift,
                "inventory_count_integrity_drift",
                true,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunIntegrityKind::RunStatusDrift,
                "run_status_integrity_drift",
                false,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunIntegrityKind::CheckpointBoundaryDrift,
                "checkpoint_boundary_integrity_drift",
                false,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunIntegrityKind::ClassifierLabelDrift,
                "classifier_label_integrity_drift",
                false,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunIntegrityKind::MultipleIntegrityDrift,
                "multiple_run_integrity_drift",
                true,
                true,
                false,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainRunIntegrityKind::MultipleIntegrityDrift,
                "multiple_run_integrity_drift",
                false,
                true,
                true,
                true,
                true,
            ),
        ];

        for (
            kind,
            label,
            has_inventory_count_integrity_drift,
            has_run_status_integrity_drift,
            has_checkpoint_boundary_integrity_drift,
            has_classifier_label_integrity_drift,
            has_multiple_integrity_drift,
        ) in cases
        {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainRunIntegrityKind::from_label(label),
                Some(kind)
            );
            assert_eq!(
                ToolAuditSupervisorDrainRunIntegrityKind::from_integrity_flags(
                    has_inventory_count_integrity_drift,
                    has_run_status_integrity_drift,
                    has_checkpoint_boundary_integrity_drift,
                    has_classifier_label_integrity_drift,
                ),
                kind
            );
            assert_eq!(
                kind.has_run_integrity_drift(),
                has_inventory_count_integrity_drift
                    || has_run_status_integrity_drift
                    || has_checkpoint_boundary_integrity_drift
                    || has_classifier_label_integrity_drift
            );
            assert_eq!(
                kind.is_no_drift(),
                !(has_inventory_count_integrity_drift
                    || has_run_status_integrity_drift
                    || has_checkpoint_boundary_integrity_drift
                    || has_classifier_label_integrity_drift)
            );
            assert_eq!(
                kind.has_multiple_integrity_drift(),
                has_multiple_integrity_drift
            );
            assert_eq!(
                kind.requires_investigation(),
                has_inventory_count_integrity_drift
                    || has_run_status_integrity_drift
                    || has_checkpoint_boundary_integrity_drift
                    || has_classifier_label_integrity_drift
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainRunIntegrityKind::from_label("integrityish"),
            None
        );
    }

    #[test]
    fn supervisor_drain_host_investigation_kind_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainHostInvestigationKind::NoInvestigation,
                "no_investigation",
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostInvestigationKind::PlanDrift,
                "plan_drift",
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostInvestigationKind::CountDrift,
                "count_drift",
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostInvestigationKind::PlanAndCountDrift,
                "plan_and_count_drift",
                true,
                true,
            ),
        ];

        for (kind, label, requires_plan_drift_investigation, requires_count_drift_investigation) in
            cases
        {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainHostInvestigationKind::from_label(label),
                Some(kind)
            );
            assert_eq!(
                ToolAuditSupervisorDrainHostInvestigationKind::from_investigation_flags(
                    requires_plan_drift_investigation,
                    requires_count_drift_investigation
                ),
                kind
            );
            assert_eq!(
                kind.requires_plan_drift_investigation(),
                requires_plan_drift_investigation
            );
            assert_eq!(
                kind.requires_count_drift_investigation(),
                requires_count_drift_investigation
            );
            assert_eq!(
                kind.requires_investigation(),
                requires_plan_drift_investigation || requires_count_drift_investigation
            );
            assert_eq!(
                kind.is_no_investigation(),
                !(requires_plan_drift_investigation || requires_count_drift_investigation)
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainHostInvestigationKind::from_label("look_into_it"),
            None
        );
    }

    #[test]
    fn supervisor_drain_host_attention_kind_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainHostAttentionKind::NoAttention,
                "no_attention",
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction,
                "scheduler_action",
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostAttentionKind::HostInvestigation,
                "host_investigation",
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostAttentionKind::RunIntegrityInvestigation,
                "run_integrity_investigation",
                false,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndHostInvestigation,
                "scheduler_action_and_host_investigation",
                true,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndRunIntegrityInvestigation,
                "scheduler_action_and_run_integrity_investigation",
                true,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostAttentionKind::HostAndRunIntegrityInvestigation,
                "host_and_run_integrity_investigation",
                false,
                true,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionHostAndRunIntegrityInvestigation,
                "scheduler_action_host_and_run_integrity_investigation",
                true,
                true,
                true,
            ),
        ];

        for (
            kind,
            label,
            requires_scheduler_action,
            requires_host_investigation,
            requires_run_integrity_investigation,
        ) in cases
        {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainHostAttentionKind::from_label(label),
                Some(kind)
            );
            assert_eq!(
                ToolAuditSupervisorDrainHostAttentionKind::from_attention_flags(
                    requires_scheduler_action,
                    requires_host_investigation,
                    requires_run_integrity_investigation,
                ),
                kind
            );
            assert_eq!(
                kind.requires_attention(),
                requires_scheduler_action
                    || requires_host_investigation
                    || requires_run_integrity_investigation
            );
            assert_eq!(
                kind.is_no_attention(),
                !(requires_scheduler_action
                    || requires_host_investigation
                    || requires_run_integrity_investigation)
            );
            assert_eq!(kind.includes_scheduler_action(), requires_scheduler_action);
            assert_eq!(
                kind.includes_host_investigation(),
                requires_host_investigation
            );
            assert_eq!(
                kind.includes_run_integrity_investigation(),
                requires_run_integrity_investigation
            );
            assert_eq!(
                kind.has_multiple_attention(),
                [
                    requires_scheduler_action,
                    requires_host_investigation,
                    requires_run_integrity_investigation,
                ]
                .into_iter()
                .filter(|requires_attention| *requires_attention)
                .count()
                    > 1
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainHostAttentionKind::from_label("pay_attention"),
            None
        );
    }

    #[test]
    fn supervisor_drain_terminal_readiness_kind_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::IdleReady,
                "idle_ready",
                ToolAuditSupervisorDrainRunOutcome::Idle,
                ToolAuditSupervisorDrainHostAttentionKind::NoAttention,
                true,
                true,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::CaughtUpReady,
                "caught_up_ready",
                ToolAuditSupervisorDrainRunOutcome::CaughtUp,
                ToolAuditSupervisorDrainHostAttentionKind::NoAttention,
                true,
                false,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionPending,
                "scheduler_action_pending",
                ToolAuditSupervisorDrainRunOutcome::NeedsContinuation,
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction,
                false,
                false,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::HostInvestigationPending,
                "host_investigation_pending",
                ToolAuditSupervisorDrainRunOutcome::Idle,
                ToolAuditSupervisorDrainHostAttentionKind::HostInvestigation,
                false,
                false,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::RunIntegrityInvestigationPending,
                "run_integrity_investigation_pending",
                ToolAuditSupervisorDrainRunOutcome::Idle,
                ToolAuditSupervisorDrainHostAttentionKind::RunIntegrityInvestigation,
                false,
                false,
                false,
                false,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionAndHostInvestigationPending,
                "scheduler_action_and_host_investigation_pending",
                ToolAuditSupervisorDrainRunOutcome::PlanDiverged,
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndHostInvestigation,
                false,
                false,
                false,
                true,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionAndRunIntegrityInvestigationPending,
                "scheduler_action_and_run_integrity_investigation_pending",
                ToolAuditSupervisorDrainRunOutcome::NeedsContinuation,
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndRunIntegrityInvestigation,
                false,
                false,
                false,
                true,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::HostAndRunIntegrityInvestigationPending,
                "host_and_run_integrity_investigation_pending",
                ToolAuditSupervisorDrainRunOutcome::Idle,
                ToolAuditSupervisorDrainHostAttentionKind::HostAndRunIntegrityInvestigation,
                false,
                false,
                false,
                false,
                true,
                true,
            ),
            (
                ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionHostAndRunIntegrityInvestigationPending,
                "scheduler_action_host_and_run_integrity_investigation_pending",
                ToolAuditSupervisorDrainRunOutcome::PlanDiverged,
                ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionHostAndRunIntegrityInvestigation,
                false,
                false,
                false,
                true,
                true,
                true,
            ),
        ];

        for (
            kind,
            label,
            outcome,
            attention,
            is_terminal_ready,
            is_idle_ready,
            is_caught_up_ready,
            requires_scheduler_action,
            requires_host_investigation,
            requires_run_integrity_investigation,
        ) in cases
        {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainTerminalReadinessKind::from_label(label),
                Some(kind)
            );
            assert_eq!(
                ToolAuditSupervisorDrainTerminalReadinessKind::from_outcome_and_attention(
                    outcome, attention
                ),
                kind
            );
            assert_eq!(kind.is_terminal_ready(), is_terminal_ready);
            assert_eq!(kind.is_idle_ready(), is_idle_ready);
            assert_eq!(kind.is_caught_up_ready(), is_caught_up_ready);
            assert_eq!(kind.requires_scheduler_action(), requires_scheduler_action);
            assert_eq!(
                kind.requires_host_investigation(),
                requires_host_investigation
            );
            assert_eq!(
                kind.requires_run_integrity_investigation(),
                requires_run_integrity_investigation
            );
            assert_eq!(
                kind.has_multiple_attention(),
                [
                    requires_scheduler_action,
                    requires_host_investigation,
                    requires_run_integrity_investigation,
                ]
                .into_iter()
                .filter(|requires_attention| *requires_attention)
                .count()
                    > 1
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainTerminalReadinessKind::from_outcome_and_attention(
                ToolAuditSupervisorDrainRunOutcome::NeedsFollowUp,
                ToolAuditSupervisorDrainHostAttentionKind::NoAttention,
            ),
            ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionPending
        );
        assert_eq!(
            ToolAuditSupervisorDrainTerminalReadinessKind::from_label("almost_ready"),
            None
        );
    }

    #[test]
    fn supervisor_drain_host_decision_kind_labels_are_stable_for_hosts() {
        let cases = [
            (
                ToolAuditSupervisorDrainHostDecisionKind::TerminalReady,
                "terminal_ready",
                false,
                false,
                false,
                false,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation,
                "schedule_continuation",
                true,
                false,
                false,
                false,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp,
                "route_follow_up",
                false,
                true,
                false,
                false,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift,
                "investigate_plan_drift",
                false,
                false,
                true,
                false,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift,
                "investigate_count_drift",
                false,
                false,
                false,
                true,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity,
                "investigate_run_integrity",
                false,
                false,
                false,
                false,
                true,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::MultipleActions,
                "multiple_actions",
                true,
                false,
                false,
                false,
                true,
                true,
                false,
                true,
            ),
        ];

        for (
            kind,
            label,
            schedules_continuation,
            routes_follow_up,
            investigates_plan_drift,
            investigates_count_drift,
            investigates_run_integrity,
            requires_decision,
            is_terminal_ready,
            has_multiple_actions,
        ) in cases
        {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionKind::from_label(label),
                Some(kind)
            );
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionKind::from_decision_flags(
                    schedules_continuation,
                    routes_follow_up,
                    investigates_plan_drift,
                    investigates_count_drift,
                    investigates_run_integrity,
                ),
                kind
            );
            assert_eq!(kind.requires_decision(), requires_decision);
            assert_eq!(kind.is_terminal_ready(), is_terminal_ready);
            assert_eq!(
                kind.schedules_continuation(),
                kind == ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation
            );
            assert_eq!(
                kind.routes_follow_up(),
                kind == ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp
            );
            assert_eq!(
                kind.investigates_plan_drift(),
                kind == ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift
            );
            assert_eq!(
                kind.investigates_count_drift(),
                kind == ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift
            );
            assert_eq!(
                kind.investigates_run_integrity(),
                kind == ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity
            );
            assert_eq!(kind.has_multiple_actions(), has_multiple_actions);
        }
        assert_eq!(
            ToolAuditSupervisorDrainHostDecisionKind::from_label("choose_again"),
            None
        );
    }

    #[test]
    fn supervisor_drain_host_decision_lanes_are_stable_for_dashboards() {
        let cases = [
            (
                ToolAuditSupervisorDrainHostDecisionKind::TerminalReady,
                ToolAuditSupervisorDrainHostDecisionLane::Terminal,
                "terminal",
                true,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation,
                ToolAuditSupervisorDrainHostDecisionLane::Scheduler,
                "scheduler",
                false,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp,
                ToolAuditSupervisorDrainHostDecisionLane::FollowUp,
                "follow_up",
                false,
                false,
                true,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift,
                ToolAuditSupervisorDrainHostDecisionLane::Investigation,
                "investigation",
                false,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift,
                ToolAuditSupervisorDrainHostDecisionLane::Investigation,
                "investigation",
                false,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity,
                ToolAuditSupervisorDrainHostDecisionLane::Investigation,
                "investigation",
                false,
                false,
                false,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::MultipleActions,
                ToolAuditSupervisorDrainHostDecisionLane::Mixed,
                "mixed",
                false,
                false,
                false,
                false,
                true,
            ),
        ];

        for (
            decision,
            lane,
            label,
            is_terminal,
            is_scheduler,
            is_follow_up,
            is_investigation,
            is_mixed,
        ) in cases
        {
            assert_eq!(decision.lane(), lane);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionLane::from_decision_kind(decision),
                lane
            );
            assert_eq!(lane.as_str(), label);
            assert_eq!(lane.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionLane::from_label(label),
                Some(lane)
            );
            assert_eq!(lane.is_terminal(), is_terminal);
            assert_eq!(lane.is_scheduler(), is_scheduler);
            assert_eq!(lane.is_follow_up(), is_follow_up);
            assert_eq!(lane.is_investigation(), is_investigation);
            assert_eq!(lane.is_mixed(), is_mixed);
        }
        assert_eq!(
            ToolAuditSupervisorDrainHostDecisionLane::from_label("parking_lot"),
            None
        );
    }

    #[test]
    fn supervisor_drain_host_decision_priorities_are_stable_for_dashboards() {
        let cases = [
            (
                ToolAuditSupervisorDrainHostDecisionKind::TerminalReady,
                ToolAuditSupervisorDrainHostDecisionPriority::Settled,
                "settled",
                0,
                true,
                false,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation,
                ToolAuditSupervisorDrainHostDecisionPriority::RoutineAction,
                "routine_action",
                10,
                false,
                true,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp,
                ToolAuditSupervisorDrainHostDecisionPriority::RoutineAction,
                "routine_action",
                10,
                false,
                true,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift,
                ToolAuditSupervisorDrainHostDecisionPriority::DriftInvestigation,
                "drift_investigation",
                50,
                false,
                false,
                true,
                false,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift,
                ToolAuditSupervisorDrainHostDecisionPriority::DriftInvestigation,
                "drift_investigation",
                50,
                false,
                false,
                true,
                false,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity,
                ToolAuditSupervisorDrainHostDecisionPriority::IntegrityInvestigation,
                "integrity_investigation",
                80,
                false,
                false,
                false,
                true,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::MultipleActions,
                ToolAuditSupervisorDrainHostDecisionPriority::MixedAction,
                "mixed_action",
                90,
                false,
                false,
                false,
                false,
                true,
                true,
            ),
        ];

        for (
            decision,
            priority,
            label,
            rank,
            is_settled,
            is_routine_action,
            is_drift_investigation,
            is_integrity_investigation,
            is_mixed_action,
            requires_investigation,
        ) in cases
        {
            assert_eq!(decision.priority(), priority);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionPriority::from_decision_kind(decision),
                priority
            );
            assert_eq!(priority.as_str(), label);
            assert_eq!(priority.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionPriority::from_label(label),
                Some(priority)
            );
            assert_eq!(priority.rank(), rank);
            assert_eq!(priority.is_settled(), is_settled);
            assert_eq!(priority.is_routine_action(), is_routine_action);
            assert_eq!(priority.is_drift_investigation(), is_drift_investigation);
            assert_eq!(
                priority.is_integrity_investigation(),
                is_integrity_investigation
            );
            assert_eq!(priority.is_mixed_action(), is_mixed_action);
            assert_eq!(priority.requires_investigation(), requires_investigation);
        }
        assert_eq!(
            ToolAuditSupervisorDrainHostDecisionPriority::from_label("eventually"),
            None
        );
    }

    #[test]
    fn supervisor_drain_host_decision_readiness_is_stable_for_queues() {
        let cases = [
            (
                ToolAuditSupervisorDrainHostDecisionKind::TerminalReady,
                ToolAuditSupervisorDrainHostDecisionReadiness::Settled,
                "settled",
                true,
                false,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation,
                ToolAuditSupervisorDrainHostDecisionReadiness::RoutineReady,
                "routine_ready",
                false,
                true,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp,
                ToolAuditSupervisorDrainHostDecisionReadiness::RoutineReady,
                "routine_ready",
                false,
                true,
                true,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift,
                ToolAuditSupervisorDrainHostDecisionReadiness::DriftInvestigationReady,
                "drift_investigation_ready",
                false,
                true,
                false,
                true,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift,
                ToolAuditSupervisorDrainHostDecisionReadiness::DriftInvestigationReady,
                "drift_investigation_ready",
                false,
                true,
                false,
                true,
                true,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity,
                ToolAuditSupervisorDrainHostDecisionReadiness::IntegrityInvestigationReady,
                "integrity_investigation_ready",
                false,
                true,
                false,
                true,
                true,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::MultipleActions,
                ToolAuditSupervisorDrainHostDecisionReadiness::TriageRequired,
                "triage_required",
                false,
                true,
                false,
                true,
                false,
                false,
            ),
        ];

        for (
            decision,
            readiness,
            label,
            is_settled,
            is_actionable,
            is_auto_routable,
            requires_manual_review,
            requires_investigation,
            requires_integrity_investigation,
        ) in cases
        {
            assert_eq!(decision.readiness(), readiness);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionReadiness::from_decision_kind(decision),
                readiness
            );
            assert_eq!(readiness.as_str(), label);
            assert_eq!(readiness.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionReadiness::from_label(label),
                Some(readiness)
            );
            assert_eq!(readiness.is_settled(), is_settled);
            assert_eq!(readiness.is_actionable(), is_actionable);
            assert_eq!(readiness.is_auto_routable(), is_auto_routable);
            assert_eq!(readiness.requires_manual_review(), requires_manual_review);
            assert_eq!(readiness.requires_investigation(), requires_investigation);
            assert_eq!(
                readiness.requires_integrity_investigation(),
                requires_integrity_investigation
            );
            assert_eq!(
                readiness.requires_triage(),
                decision == ToolAuditSupervisorDrainHostDecisionKind::MultipleActions
            );
        }
        assert_eq!(
            ToolAuditSupervisorDrainHostDecisionReadiness::from_label("readyish"),
            None
        );
    }

    #[test]
    fn supervisor_drain_host_decision_routes_are_stable_for_queues() {
        let cases = [
            (
                ToolAuditSupervisorDrainHostDecisionKind::TerminalReady,
                ToolAuditSupervisorDrainHostDecisionRoute::NoRoute,
                "no_route",
                false,
                false,
                false,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation,
                ToolAuditSupervisorDrainHostDecisionRoute::Scheduler,
                "scheduler",
                true,
                true,
                false,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp,
                ToolAuditSupervisorDrainHostDecisionRoute::FollowUp,
                "follow_up",
                true,
                false,
                true,
                false,
                false,
                false,
                false,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift,
                ToolAuditSupervisorDrainHostDecisionRoute::DriftInvestigation,
                "drift_investigation",
                true,
                false,
                false,
                true,
                false,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift,
                ToolAuditSupervisorDrainHostDecisionRoute::DriftInvestigation,
                "drift_investigation",
                true,
                false,
                false,
                true,
                false,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity,
                ToolAuditSupervisorDrainHostDecisionRoute::IntegrityInvestigation,
                "integrity_investigation",
                true,
                false,
                false,
                false,
                true,
                false,
                true,
            ),
            (
                ToolAuditSupervisorDrainHostDecisionKind::MultipleActions,
                ToolAuditSupervisorDrainHostDecisionRoute::Triage,
                "triage",
                true,
                false,
                false,
                false,
                false,
                true,
                false,
            ),
        ];

        for (
            decision,
            route,
            label,
            has_route,
            is_scheduler,
            is_follow_up,
            is_drift_investigation,
            is_integrity_investigation,
            is_triage,
            is_investigation,
        ) in cases
        {
            assert_eq!(decision.route(), route);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionRoute::from_decision_kind(decision),
                route
            );
            assert_eq!(route.as_str(), label);
            assert_eq!(route.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainHostDecisionRoute::from_label(label),
                Some(route)
            );
            assert_eq!(route.has_route(), has_route);
            assert_eq!(route.is_scheduler(), is_scheduler);
            assert_eq!(route.is_follow_up(), is_follow_up);
            assert_eq!(route.is_drift_investigation(), is_drift_investigation);
            assert_eq!(
                route.is_integrity_investigation(),
                is_integrity_investigation
            );
            assert_eq!(route.is_triage(), is_triage);
            assert_eq!(route.is_investigation(), is_investigation);
        }
        assert_eq!(
            ToolAuditSupervisorDrainHostDecisionRoute::from_label("somewhere_else"),
            None
        );
    }

    #[test]
    fn supervisor_drain_report_exposes_outcome_label_and_action_flag() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut sink)
            .unwrap();

        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::NeedsContinuation
        );
        assert_eq!(report.outcome_label(), "needs_continuation");
        assert!(report.outcome_label_matches_outcome());
        assert!(!report.outcome_is_idle());
        assert!(!report.is_caught_up());
        assert!(report.needs_continuation());
        assert!(!report.needs_follow_up());
        assert!(!report.plan_diverged());
        assert!(report.requires_scheduler_action());
        assert!(!report.is_no_scheduler_action());
        assert!(report.requests_continuation());
        assert!(!report.routes_follow_up());
        assert!(!report.requires_plan_drift_investigation());
        assert_eq!(
            report.host_investigation_kind(),
            ToolAuditSupervisorDrainHostInvestigationKind::NoInvestigation
        );
        assert_eq!(report.host_investigation_label(), "no_investigation");
        assert!(report.host_investigation_label_matches_kind());
        assert!(!report.requires_host_investigation());
        assert!(report.is_no_count_drift());
        assert!(report.count_drift_label_matches_kind());
        assert!(!report.has_planned_last_checkpoint_drift());
        assert!(!report.has_checkpoint_advance_drift());
        assert!(!report.has_checkpoint_plan_drift());
        assert_eq!(
            report.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(report.checkpoint_drift_label(), "no_checkpoint_drift");
        assert!(report.checkpoint_drift_label_matches_kind());
        assert!(report.is_no_checkpoint_drift());
        assert!(!report.requires_checkpoint_drift_investigation());
        assert!(report.is_no_host_investigation());
        assert_eq!(
            report.scheduler_action(),
            ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation
        );
        assert_eq!(report.scheduler_action_label(), "schedule_continuation");
        assert!(report.scheduler_action_label_matches_action());
        assert!(report.all_classifier_labels_match());
        assert!(!report.has_inventory_count_integrity_drift());
        assert!(!report.has_run_status_integrity_drift());
        assert!(!report.has_checkpoint_boundary_integrity_drift());
        assert!(!report.has_classifier_label_integrity_drift());
        assert!(!report.has_run_integrity_drift());
        assert_eq!(
            report.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::NoDrift
        );
        assert_eq!(report.run_integrity_label(), "no_run_integrity_drift");
        assert!(report.run_integrity_label_matches_kind());
        assert!(report.is_run_integrity_clean());
        assert!(!report.requires_run_integrity_investigation());
        assert!(report.requires_host_attention());
        assert!(!report.is_no_host_attention());
        assert_eq!(
            report.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction
        );
        assert_eq!(report.host_attention_label(), "scheduler_action");
        assert!(report.host_attention_label_matches_kind());
        assert!(report.host_attention_includes_scheduler_action());
        assert!(!report.host_attention_includes_host_investigation());
        assert!(!report.host_attention_includes_run_integrity_investigation());
        assert!(!report.has_multiple_host_attention());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_host_fields() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut sink)
            .unwrap();
        let summary = report.summary();

        assert_eq!(summary.checkpoint_name, "supervisor");
        assert_eq!(summary.checkpoint_name(), "supervisor");
        assert_eq!(
            report.starting_checkpoint(),
            &ToolAuditReadCheckpoint::beginning()
        );
        assert_eq!(
            summary.starting_checkpoint,
            ToolAuditReadCheckpoint::beginning()
        );
        assert_eq!(
            summary.starting_checkpoint(),
            &ToolAuditReadCheckpoint::beginning()
        );
        assert_eq!(report.starting_checkpoint_timestamp_ms(), 0);
        assert_eq!(summary.starting_checkpoint_timestamp_ms, 0);
        assert_eq!(summary.starting_checkpoint_timestamp_ms(), 0);
        assert_eq!(report.starting_checkpoint_call_id(), "");
        assert_eq!(summary.starting_checkpoint_call_id, "");
        assert_eq!(summary.starting_checkpoint_call_id(), "");
        assert!(report.stored_checkpoint().is_none());
        assert!(!report.has_stored_checkpoint());
        assert_eq!(report.stored_checkpoint_timestamp_ms(), None);
        assert_eq!(report.stored_checkpoint_call_id(), None);
        assert!(report.stored_checkpoint_presence_matches_fields());
        assert!(report.stored_checkpoint_timestamp_matches_checkpoint());
        assert!(report.stored_checkpoint_call_id_matches_checkpoint());
        assert!(report.stored_checkpoint_fields_match_checkpoint());
        assert!(report.starting_checkpoint_matches_stored_checkpoint());
        assert!(summary.stored_checkpoint.is_none());
        assert!(summary.stored_checkpoint().is_none());
        assert!(!summary.has_stored_checkpoint);
        assert!(!summary.has_stored_checkpoint());
        assert_eq!(summary.stored_checkpoint_timestamp_ms, None);
        assert_eq!(summary.stored_checkpoint_timestamp_ms(), None);
        assert_eq!(summary.stored_checkpoint_call_id, None);
        assert_eq!(summary.stored_checkpoint_call_id(), None);
        assert!(summary.stored_checkpoint_presence_matches_fields);
        assert!(summary.stored_checkpoint_presence_matches_fields());
        assert!(summary.stored_checkpoint_timestamp_matches_checkpoint);
        assert!(summary.stored_checkpoint_timestamp_matches_checkpoint());
        assert!(summary.stored_checkpoint_call_id_matches_checkpoint);
        assert!(summary.stored_checkpoint_call_id_matches_checkpoint());
        assert!(summary.stored_checkpoint_fields_match_checkpoint);
        assert!(summary.stored_checkpoint_fields_match_checkpoint());
        assert!(summary.starting_checkpoint_matches_stored_checkpoint);
        assert!(summary.starting_checkpoint_matches_stored_checkpoint());
        assert_eq!(
            summary.outcome,
            ToolAuditSupervisorDrainRunOutcome::NeedsContinuation
        );
        assert_eq!(
            summary.outcome(),
            ToolAuditSupervisorDrainRunOutcome::NeedsContinuation
        );
        assert_eq!(summary.outcome_label, "needs_continuation");
        assert_eq!(summary.outcome_label(), "needs_continuation");
        assert!(summary.outcome_label_matches_outcome);
        assert!(summary.outcome_label_matches_outcome());
        assert!(!summary.outcome_is_idle);
        assert!(!summary.outcome_is_idle());
        assert!(!summary.is_caught_up);
        assert!(!summary.is_caught_up());
        assert!(summary.needs_continuation);
        assert!(summary.needs_continuation());
        assert!(!summary.needs_follow_up);
        assert!(!summary.needs_follow_up());
        assert!(!summary.plan_diverged);
        assert!(!summary.plan_diverged());
        assert_eq!(
            summary.scheduler_action,
            ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation
        );
        assert_eq!(
            summary.scheduler_action(),
            ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation
        );
        assert_eq!(summary.scheduler_action_label, "schedule_continuation");
        assert_eq!(summary.scheduler_action_label(), "schedule_continuation");
        assert!(summary.scheduler_action_label_matches_action);
        assert!(summary.scheduler_action_label_matches_action());
        assert!(!summary.is_no_scheduler_action);
        assert!(!summary.is_no_scheduler_action());
        assert!(summary.requires_scheduler_action);
        assert!(summary.requires_scheduler_action());
        assert!(summary.requests_continuation);
        assert!(summary.requests_continuation());
        assert!(!summary.routes_follow_up);
        assert!(!summary.routes_follow_up());
        assert!(!summary.requires_plan_drift_investigation);
        assert!(!summary.requires_plan_drift_investigation());
        assert_eq!(summary.max_records_per_tick, 2);
        assert_eq!(summary.max_records_per_tick(), 2);
        assert_eq!(summary.max_ticks, 1);
        assert_eq!(summary.max_ticks(), 1);
        assert_eq!(summary.planned_pages, 1);
        assert_eq!(summary.planned_pages(), 1);
        assert_eq!(summary.drain_ticks, 1);
        assert_eq!(summary.drain_ticks(), 1);
        assert_eq!(summary.remaining_tick_budget, 0);
        assert_eq!(summary.remaining_tick_budget(), 0);
        assert!(summary.used_all_ticks);
        assert!(summary.used_all_ticks());
        assert!(!summary.has_remaining_tick_budget);
        assert!(!summary.has_remaining_tick_budget());
        assert_eq!(summary.planned_records, 2);
        assert_eq!(summary.planned_records(), 2);
        assert_eq!(summary.drained_records, 2);
        assert_eq!(summary.drained_records(), 2);
        assert_eq!(summary.planned_follow_up_records, 0);
        assert_eq!(summary.planned_follow_up_records(), 0);
        assert_eq!(summary.drained_follow_up_records, 0);
        assert_eq!(summary.drained_follow_up_records(), 0);
        assert_eq!(summary.record_count_delta, 0);
        assert_eq!(summary.record_count_delta(), 0);
        assert!(!summary.replayed_extra_records);
        assert!(!summary.replayed_extra_records());
        assert!(!summary.missed_planned_records);
        assert!(!summary.missed_planned_records());
        assert_eq!(summary.follow_up_record_count_delta, 0);
        assert_eq!(summary.follow_up_record_count_delta(), 0);
        assert!(!summary.replayed_extra_follow_up_records);
        assert!(!summary.replayed_extra_follow_up_records());
        assert!(!summary.missed_planned_follow_up_records);
        assert!(!summary.missed_planned_follow_up_records());
        assert!(!summary.has_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(!summary.has_count_drift);
        assert!(!summary.has_count_drift());
        assert!(summary.is_no_count_drift);
        assert!(summary.is_no_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::NoDrift
        );
        assert_eq!(
            summary.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::NoDrift
        );
        assert_eq!(summary.count_drift_label, "no_count_drift");
        assert_eq!(summary.count_drift_label(), "no_count_drift");
        assert!(summary.count_drift_label_matches_kind);
        assert!(summary.count_drift_label_matches_kind());
        assert!(!summary.requires_count_drift_investigation);
        assert!(!summary.requires_count_drift_investigation());
        assert!(!summary.has_planned_last_checkpoint_drift);
        assert!(!summary.has_planned_last_checkpoint_drift());
        assert!(!summary.has_checkpoint_advance_drift);
        assert!(!summary.has_checkpoint_advance_drift());
        assert!(!summary.has_checkpoint_plan_drift);
        assert!(!summary.has_checkpoint_plan_drift());
        assert_eq!(
            summary.checkpoint_drift_kind,
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(
            summary.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(summary.checkpoint_drift_label, "no_checkpoint_drift");
        assert_eq!(summary.checkpoint_drift_label(), "no_checkpoint_drift");
        assert!(summary.checkpoint_drift_label_matches_kind);
        assert!(summary.checkpoint_drift_label_matches_kind());
        assert!(summary.is_no_checkpoint_drift);
        assert!(summary.is_no_checkpoint_drift());
        assert!(!summary.requires_checkpoint_drift_investigation);
        assert!(!summary.requires_checkpoint_drift_investigation());
        assert_eq!(
            summary.host_investigation_kind,
            ToolAuditSupervisorDrainHostInvestigationKind::NoInvestigation
        );
        assert_eq!(
            summary.host_investigation_kind(),
            ToolAuditSupervisorDrainHostInvestigationKind::NoInvestigation
        );
        assert_eq!(summary.host_investigation_label, "no_investigation");
        assert_eq!(summary.host_investigation_label(), "no_investigation");
        assert!(summary.host_investigation_label_matches_kind);
        assert!(summary.host_investigation_label_matches_kind());
        assert!(summary.all_classifier_labels_match);
        assert!(summary.all_classifier_labels_match());
        assert!(!summary.has_inventory_count_integrity_drift);
        assert!(!summary.has_inventory_count_integrity_drift());
        assert!(!summary.has_run_status_integrity_drift);
        assert!(!summary.has_run_status_integrity_drift());
        assert!(!summary.has_checkpoint_boundary_integrity_drift);
        assert!(!summary.has_checkpoint_boundary_integrity_drift());
        assert!(!summary.has_classifier_label_integrity_drift);
        assert!(!summary.has_classifier_label_integrity_drift());
        assert!(!summary.has_run_integrity_drift);
        assert!(!summary.has_run_integrity_drift());
        assert_eq!(
            summary.run_integrity_kind,
            ToolAuditSupervisorDrainRunIntegrityKind::NoDrift
        );
        assert_eq!(
            summary.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::NoDrift
        );
        assert_eq!(summary.run_integrity_label, "no_run_integrity_drift");
        assert_eq!(summary.run_integrity_label(), "no_run_integrity_drift");
        assert!(summary.run_integrity_label_matches_kind);
        assert!(summary.run_integrity_label_matches_kind());
        assert!(summary.is_run_integrity_clean);
        assert!(summary.is_run_integrity_clean());
        assert!(!summary.requires_run_integrity_investigation);
        assert!(!summary.requires_run_integrity_investigation());
        assert!(summary.requires_host_attention);
        assert!(summary.requires_host_attention());
        assert!(!summary.is_no_host_attention);
        assert!(!summary.is_no_host_attention());
        assert_eq!(
            summary.host_attention_kind,
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction
        );
        assert_eq!(
            summary.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction
        );
        assert_eq!(summary.host_attention_label, "scheduler_action");
        assert_eq!(summary.host_attention_label(), "scheduler_action");
        assert!(summary.host_attention_label_matches_kind);
        assert!(summary.host_attention_label_matches_kind());
        assert!(summary.host_attention_includes_scheduler_action);
        assert!(summary.host_attention_includes_scheduler_action());
        assert!(!summary.host_attention_includes_host_investigation);
        assert!(!summary.host_attention_includes_host_investigation());
        assert!(!summary.host_attention_includes_run_integrity_investigation);
        assert!(!summary.host_attention_includes_run_integrity_investigation());
        assert!(!summary.has_multiple_host_attention);
        assert!(!summary.has_multiple_host_attention());
        assert!(!summary.requires_host_investigation);
        assert!(!summary.requires_host_investigation());
        assert!(!summary.requires_host_plan_drift_investigation);
        assert!(!summary.requires_host_plan_drift_investigation());
        assert!(!summary.requires_host_count_drift_investigation);
        assert!(!summary.requires_host_count_drift_investigation());
        assert!(summary.is_no_host_investigation);
        assert!(summary.is_no_host_investigation());
        assert!(summary.matches_planned_record_count);
        assert!(summary.matches_planned_record_count());
        assert!(summary.matches_record_count);
        assert!(summary.matches_record_count());
        assert!(summary.matches_planned_follow_up_record_count);
        assert!(summary.matches_planned_follow_up_record_count());
        assert!(summary.matches_follow_up_pressure);
        assert!(summary.matches_follow_up_pressure());
        assert!(report.plan_inventory_counts_match());
        assert!(report.drain_inventory_counts_match());
        assert!(report.inventory_counts_match());
        assert!(summary.plan_inventory_counts_match);
        assert!(summary.plan_inventory_counts_match());
        assert!(summary.drain_inventory_counts_match);
        assert!(summary.drain_inventory_counts_match());
        assert!(summary.inventory_counts_match);
        assert!(summary.inventory_counts_match());
        assert!(report.remaining_tick_budget_matches_counts());
        assert!(report.tick_budget_status_flags_match());
        assert!(report.drain_status_flags_match());
        assert!(report.run_status_flags_match());
        assert!(summary.remaining_tick_budget_matches_counts);
        assert!(summary.remaining_tick_budget_matches_counts());
        assert!(summary.tick_budget_status_flags_match);
        assert!(summary.tick_budget_status_flags_match());
        assert!(summary.drain_status_flags_match);
        assert!(summary.drain_status_flags_match());
        assert!(summary.run_status_flags_match);
        assert!(summary.run_status_flags_match());
        assert!(!summary.reached_end_of_log);
        assert!(!summary.reached_end_of_log());
        assert!(summary.exhausted_tick_budget);
        assert!(summary.exhausted_tick_budget());
        assert!(!summary.requires_follow_up);
        assert!(!summary.requires_follow_up());
        assert!(summary.advanced_checkpoint);
        assert!(summary.advanced_checkpoint());
        assert_eq!(summary.last_checkpoint, report.last_checkpoint().cloned());
        assert_eq!(summary.last_checkpoint(), report.last_checkpoint());
        assert!(summary.has_last_checkpoint);
        assert_eq!(summary.last_checkpoint_timestamp_ms, Some(120));
        assert_eq!(summary.last_checkpoint_call_id.as_deref(), Some("call_2"));
        assert!(summary.has_last_checkpoint());
        assert_eq!(summary.last_checkpoint_timestamp_ms(), Some(120));
        assert_eq!(summary.last_checkpoint_call_id(), Some("call_2"));
        assert!(report.last_checkpoint_presence_matches_fields());
        assert!(report.last_checkpoint_timestamp_matches_checkpoint());
        assert!(report.last_checkpoint_call_id_matches_checkpoint());
        assert!(report.last_checkpoint_fields_match_checkpoint());
        assert!(summary.last_checkpoint_presence_matches_fields);
        assert!(summary.last_checkpoint_presence_matches_fields());
        assert!(summary.last_checkpoint_timestamp_matches_checkpoint);
        assert!(summary.last_checkpoint_timestamp_matches_checkpoint());
        assert!(summary.last_checkpoint_call_id_matches_checkpoint);
        assert!(summary.last_checkpoint_call_id_matches_checkpoint());
        assert!(summary.last_checkpoint_fields_match_checkpoint);
        assert!(summary.last_checkpoint_fields_match_checkpoint());
        assert!(!summary.is_idle);
        assert!(!summary.is_idle());
        assert!(summary.made_progress);
        assert!(summary.made_progress());
        assert!(summary.should_continue);
        assert!(summary.should_continue());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_checkpoint_boundary_fields() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());
        let stored = store
            .save_checkpoint("supervisor", ToolAuditReadCheckpoint::new(120, "call_1"))
            .unwrap();

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();

        assert_eq!(
            report.starting_checkpoint(),
            &ToolAuditReadCheckpoint::new(120, "call_1")
        );
        assert_eq!(report.starting_checkpoint_timestamp_ms(), 120);
        assert_eq!(report.starting_checkpoint_call_id(), "call_1");
        assert_eq!(report.stored_checkpoint(), Some(&stored));
        assert!(report.has_stored_checkpoint());
        assert_eq!(report.stored_checkpoint_timestamp_ms(), Some(120));
        assert_eq!(report.stored_checkpoint_call_id(), Some("call_1"));
        assert!(report.stored_checkpoint_presence_matches_fields());
        assert!(report.stored_checkpoint_timestamp_matches_checkpoint());
        assert!(report.stored_checkpoint_call_id_matches_checkpoint());
        assert!(report.stored_checkpoint_fields_match_checkpoint());
        assert!(report.starting_checkpoint_matches_stored_checkpoint());

        let summary = report.summary();
        assert_eq!(
            summary.starting_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_1")
        );
        assert_eq!(
            summary.starting_checkpoint(),
            &ToolAuditReadCheckpoint::new(120, "call_1")
        );
        assert_eq!(summary.starting_checkpoint_timestamp_ms, 120);
        assert_eq!(summary.starting_checkpoint_timestamp_ms(), 120);
        assert_eq!(summary.starting_checkpoint_call_id, "call_1");
        assert_eq!(summary.starting_checkpoint_call_id(), "call_1");
        assert_eq!(summary.stored_checkpoint, Some(stored.clone()));
        assert_eq!(summary.stored_checkpoint(), Some(&stored));
        assert!(summary.has_stored_checkpoint);
        assert!(summary.has_stored_checkpoint());
        assert_eq!(summary.stored_checkpoint_timestamp_ms, Some(120));
        assert_eq!(summary.stored_checkpoint_timestamp_ms(), Some(120));
        assert_eq!(summary.stored_checkpoint_call_id.as_deref(), Some("call_1"));
        assert_eq!(summary.stored_checkpoint_call_id(), Some("call_1"));
        assert!(summary.stored_checkpoint_presence_matches_fields);
        assert!(summary.stored_checkpoint_presence_matches_fields());
        assert!(summary.stored_checkpoint_timestamp_matches_checkpoint);
        assert!(summary.stored_checkpoint_timestamp_matches_checkpoint());
        assert!(summary.stored_checkpoint_call_id_matches_checkpoint);
        assert!(summary.stored_checkpoint_call_id_matches_checkpoint());
        assert!(summary.stored_checkpoint_fields_match_checkpoint);
        assert!(summary.stored_checkpoint_fields_match_checkpoint());
        assert!(summary.starting_checkpoint_matches_stored_checkpoint);
        assert!(summary.starting_checkpoint_matches_stored_checkpoint());

        let mut stale_summary = summary;
        stale_summary.stored_checkpoint_call_id = Some("stale".to_string());
        stale_summary.stored_checkpoint_call_id_matches_checkpoint = false;
        stale_summary.stored_checkpoint_fields_match_checkpoint = false;
        assert!(!stale_summary.stored_checkpoint_call_id_matches_checkpoint());
        assert!(!stale_summary.stored_checkpoint_fields_match_checkpoint());

        let mut stale_report = report;
        stale_report.plan.starting_checkpoint = ToolAuditReadCheckpoint::beginning();
        assert!(!stale_report.starting_checkpoint_matches_stored_checkpoint());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_budget_status_consistency_flags() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut sink)
            .unwrap();

        assert_eq!(report.max_ticks(), 1);
        assert_eq!(report.drain_ticks(), 1);
        assert_eq!(report.remaining_tick_budget(), 0);
        assert!(report.remaining_tick_budget_matches_counts());
        assert!(report.tick_budget_status_flags_match());
        assert!(report.drain_status_flags_match());
        assert!(report.run_status_flags_match());

        let summary = report.summary();
        assert_eq!(summary.max_ticks, 1);
        assert_eq!(summary.drain_ticks, 1);
        assert_eq!(summary.remaining_tick_budget, 0);
        assert!(summary.remaining_tick_budget_matches_counts);
        assert!(summary.remaining_tick_budget_matches_counts());
        assert!(summary.tick_budget_status_flags_match);
        assert!(summary.tick_budget_status_flags_match());
        assert!(summary.drain_status_flags_match);
        assert!(summary.drain_status_flags_match());
        assert!(summary.run_status_flags_match);
        assert!(summary.run_status_flags_match());

        let mut stale_budget_summary = summary.clone();
        stale_budget_summary.remaining_tick_budget = 1;
        stale_budget_summary.remaining_tick_budget_matches_counts = false;
        stale_budget_summary.tick_budget_status_flags_match = false;
        stale_budget_summary.run_status_flags_match = false;
        assert!(!stale_budget_summary.remaining_tick_budget_matches_counts());
        assert!(!stale_budget_summary.tick_budget_status_flags_match());
        assert!(!stale_budget_summary.run_status_flags_match());

        let mut stale_status_summary = summary;
        stale_status_summary.made_progress = false;
        stale_status_summary.drain_status_flags_match = false;
        stale_status_summary.run_status_flags_match = false;
        assert!(!stale_status_summary.drain_status_flags_match());
        assert!(!stale_status_summary.run_status_flags_match());

        let mut inconsistent_report = report;
        inconsistent_report.drain.drained_records = 0;
        assert!(!inconsistent_report.drain_status_flags_match());
        assert!(!inconsistent_report.run_status_flags_match());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_inventory_count_flags() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let page = store
            .query_audits_after_checkpoint(&ToolAuditReadCheckpoint::beginning(), Some(2))
            .unwrap();
        assert!(page.record_count_matches_inventory());

        let status = store
            .inspect_supervisor_checkpoint("supervisor", 2)
            .unwrap();
        assert!(status.pending_count_matches_inventory());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 2, &mut sink)
            .unwrap();

        assert!(report.plan.plan_inventory_counts_match());
        assert!(report.plan.planned_count_matches_inventory());
        assert!(report.plan.planned_count_matches_pages());
        assert!(report.plan.page_inventory_counts_match());
        assert!(report.plan.pages[0].pending_count_matches_inventory());
        assert!(report.drain.drain_inventory_counts_match());
        assert!(report.drain.drained_count_matches_ticks());
        assert!(report.drain.tick_inventory_counts_match());
        assert!(report.drain.ticks[0].replayed_count_matches_inventory());
        assert!(report.drain.ticks[0]
            .replay
            .replayed_count_matches_inventory());
        assert!(report.plan_inventory_counts_match());
        assert!(report.drain_inventory_counts_match());
        assert!(report.inventory_counts_match());

        let summary = report.summary();
        assert!(summary.plan_inventory_counts_match);
        assert!(summary.plan_inventory_counts_match());
        assert!(summary.drain_inventory_counts_match);
        assert!(summary.drain_inventory_counts_match());
        assert!(summary.inventory_counts_match);
        assert!(summary.inventory_counts_match());

        let mut plan_mismatch = report.clone();
        plan_mismatch.plan.pages[0].inventory.total_records += 1;
        let plan_mismatch_summary = plan_mismatch.summary();
        assert!(!plan_mismatch.plan.page_inventory_counts_match());
        assert!(!plan_mismatch.plan_inventory_counts_match());
        assert!(plan_mismatch.drain_inventory_counts_match());
        assert!(!plan_mismatch.inventory_counts_match());
        assert!(!plan_mismatch_summary.plan_inventory_counts_match);
        assert!(!plan_mismatch_summary.plan_inventory_counts_match());
        assert!(plan_mismatch_summary.drain_inventory_counts_match);
        assert!(!plan_mismatch_summary.inventory_counts_match);
        assert!(!plan_mismatch_summary.inventory_counts_match());

        let mut drain_mismatch = report;
        drain_mismatch.drain.ticks[0].replay.inventory.total_records += 1;
        let drain_mismatch_summary = drain_mismatch.summary();
        assert!(drain_mismatch.plan_inventory_counts_match());
        assert!(!drain_mismatch.drain.tick_inventory_counts_match());
        assert!(!drain_mismatch.drain_inventory_counts_match());
        assert!(!drain_mismatch.inventory_counts_match());
        assert!(drain_mismatch_summary.plan_inventory_counts_match);
        assert!(!drain_mismatch_summary.drain_inventory_counts_match);
        assert!(!drain_mismatch_summary.drain_inventory_counts_match());
        assert!(!drain_mismatch_summary.inventory_counts_match);
        assert!(!drain_mismatch_summary.inventory_counts_match());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_run_integrity_fields() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut sink)
            .unwrap();
        let summary = report.summary();

        assert!(!report.has_inventory_count_integrity_drift());
        assert!(!report.has_run_status_integrity_drift());
        assert!(!report.has_checkpoint_boundary_integrity_drift());
        assert!(!report.has_classifier_label_integrity_drift());
        assert!(!report.has_run_integrity_drift());
        assert_eq!(
            report.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::NoDrift
        );
        assert_eq!(report.run_integrity_label(), "no_run_integrity_drift");
        assert!(report.run_integrity_label_matches_kind());
        assert!(report.is_run_integrity_clean());
        assert!(!report.requires_run_integrity_investigation());
        assert!(!summary.has_inventory_count_integrity_drift);
        assert!(!summary.has_inventory_count_integrity_drift());
        assert!(!summary.has_run_status_integrity_drift);
        assert!(!summary.has_run_status_integrity_drift());
        assert!(!summary.has_checkpoint_boundary_integrity_drift);
        assert!(!summary.has_checkpoint_boundary_integrity_drift());
        assert!(!summary.has_classifier_label_integrity_drift);
        assert!(!summary.has_classifier_label_integrity_drift());
        assert!(!summary.has_run_integrity_drift);
        assert!(!summary.has_run_integrity_drift());
        assert_eq!(
            summary.run_integrity_kind,
            ToolAuditSupervisorDrainRunIntegrityKind::NoDrift
        );
        assert_eq!(
            summary.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::NoDrift
        );
        assert_eq!(summary.run_integrity_label, "no_run_integrity_drift");
        assert_eq!(summary.run_integrity_label(), "no_run_integrity_drift");
        assert!(summary.run_integrity_label_matches_kind);
        assert!(summary.run_integrity_label_matches_kind());
        assert!(summary.is_run_integrity_clean);
        assert!(summary.is_run_integrity_clean());
        assert!(!summary.requires_run_integrity_investigation);
        assert!(!summary.requires_run_integrity_investigation());

        let mut inventory_report = report.clone();
        inventory_report.plan.pages[0].inventory.total_records += 1;
        let inventory_summary = inventory_report.summary();
        assert!(inventory_report.has_inventory_count_integrity_drift());
        assert!(!inventory_report.has_run_status_integrity_drift());
        assert!(!inventory_report.has_checkpoint_boundary_integrity_drift());
        assert!(!inventory_report.has_classifier_label_integrity_drift());
        assert!(inventory_report.has_run_integrity_drift());
        assert_eq!(
            inventory_report.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::InventoryCountDrift
        );
        assert_eq!(
            inventory_report.run_integrity_label(),
            "inventory_count_integrity_drift"
        );
        assert!(!inventory_report.is_run_integrity_clean());
        assert!(inventory_report.requires_run_integrity_investigation());
        assert!(inventory_summary.has_inventory_count_integrity_drift);
        assert!(!inventory_summary.has_run_status_integrity_drift);
        assert!(!inventory_summary.has_checkpoint_boundary_integrity_drift);
        assert!(!inventory_summary.has_classifier_label_integrity_drift);
        assert!(inventory_summary.has_run_integrity_drift);
        assert_eq!(
            inventory_summary.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::InventoryCountDrift
        );
        assert_eq!(
            inventory_summary.run_integrity_label(),
            "inventory_count_integrity_drift"
        );
        assert!(inventory_summary.run_integrity_label_matches_kind());
        assert!(!inventory_summary.is_run_integrity_clean());
        assert!(inventory_summary.requires_run_integrity_investigation());

        let mut boundary_report = report.clone();
        boundary_report.drain.ticks[0].replay.next_checkpoint =
            ToolAuditReadCheckpoint::beginning();
        let boundary_summary = boundary_report.summary();
        assert!(!boundary_report.has_inventory_count_integrity_drift());
        assert!(!boundary_report.has_run_status_integrity_drift());
        assert!(boundary_report.has_checkpoint_boundary_integrity_drift());
        assert!(!boundary_report.has_classifier_label_integrity_drift());
        assert!(boundary_report.has_run_integrity_drift());
        assert_eq!(
            boundary_report.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::CheckpointBoundaryDrift
        );
        assert_eq!(
            boundary_report.run_integrity_label(),
            "checkpoint_boundary_integrity_drift"
        );
        assert!(boundary_report.run_integrity_label_matches_kind());
        assert!(!boundary_report.is_run_integrity_clean());
        assert!(boundary_report.requires_run_integrity_investigation());
        assert!(!boundary_summary.has_inventory_count_integrity_drift);
        assert!(!boundary_summary.has_run_status_integrity_drift);
        assert!(boundary_summary.has_checkpoint_boundary_integrity_drift);
        assert!(!boundary_summary.has_classifier_label_integrity_drift);
        assert!(boundary_summary.has_run_integrity_drift);
        assert_eq!(
            boundary_summary.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::CheckpointBoundaryDrift
        );
        assert_eq!(
            boundary_summary.run_integrity_label(),
            "checkpoint_boundary_integrity_drift"
        );
        assert!(boundary_summary.requires_run_integrity_investigation());

        let mut combined_report = report.clone();
        combined_report.plan.pages[0].inventory.total_records += 1;
        combined_report.drain.ticks[0].replay.next_checkpoint =
            ToolAuditReadCheckpoint::beginning();
        let combined_summary = combined_report.summary();
        assert!(combined_report.has_inventory_count_integrity_drift());
        assert!(combined_report.has_checkpoint_boundary_integrity_drift());
        assert!(combined_report.has_run_integrity_drift());
        assert_eq!(
            combined_report.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::MultipleIntegrityDrift
        );
        assert_eq!(
            combined_report.run_integrity_label(),
            "multiple_run_integrity_drift"
        );
        assert!(combined_report.requires_run_integrity_investigation());
        assert!(combined_summary.has_inventory_count_integrity_drift);
        assert!(combined_summary.has_checkpoint_boundary_integrity_drift);
        assert!(combined_summary.has_run_integrity_drift);
        assert_eq!(
            combined_summary.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::MultipleIntegrityDrift
        );
        assert_eq!(
            combined_summary.run_integrity_label(),
            "multiple_run_integrity_drift"
        );
        assert!(combined_summary.requires_run_integrity_investigation());

        let mut stale_status_summary = summary.clone();
        stale_status_summary.run_status_flags_match = false;
        stale_status_summary.has_run_status_integrity_drift = true;
        stale_status_summary.has_run_integrity_drift = true;
        stale_status_summary.run_integrity_kind =
            ToolAuditSupervisorDrainRunIntegrityKind::RunStatusDrift;
        stale_status_summary.run_integrity_label = "run_status_integrity_drift";
        stale_status_summary.is_run_integrity_clean = false;
        stale_status_summary.requires_run_integrity_investigation = true;
        assert!(!stale_status_summary.run_status_flags_match());
        assert!(stale_status_summary.has_run_status_integrity_drift());
        assert!(stale_status_summary.has_run_integrity_drift());
        assert_eq!(
            stale_status_summary.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::RunStatusDrift
        );
        assert_eq!(
            stale_status_summary.run_integrity_label(),
            "run_status_integrity_drift"
        );
        assert!(!stale_status_summary.is_run_integrity_clean());
        assert!(stale_status_summary.requires_run_integrity_investigation());

        let mut stale_label_summary = summary;
        stale_label_summary.all_classifier_labels_match = false;
        stale_label_summary.has_classifier_label_integrity_drift = true;
        stale_label_summary.has_run_integrity_drift = true;
        stale_label_summary.run_integrity_kind =
            ToolAuditSupervisorDrainRunIntegrityKind::ClassifierLabelDrift;
        stale_label_summary.run_integrity_label = "classifier_label_integrity_drift";
        stale_label_summary.is_run_integrity_clean = false;
        stale_label_summary.requires_run_integrity_investigation = true;
        assert!(!stale_label_summary.all_classifier_labels_match());
        assert!(stale_label_summary.has_classifier_label_integrity_drift());
        assert!(stale_label_summary.has_run_integrity_drift());
        assert_eq!(
            stale_label_summary.run_integrity_kind(),
            ToolAuditSupervisorDrainRunIntegrityKind::ClassifierLabelDrift
        );
        assert_eq!(
            stale_label_summary.run_integrity_label(),
            "classifier_label_integrity_drift"
        );
        assert!(stale_label_summary.run_integrity_label_matches_kind());
        assert!(!stale_label_summary.is_run_integrity_clean());
        assert!(stale_label_summary.requires_run_integrity_investigation());

        stale_label_summary.run_integrity_label = "integrityish";
        stale_label_summary.run_integrity_label_matches_kind = false;
        assert!(!stale_label_summary.run_integrity_label_matches_kind());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_host_attention_fields() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert!(!idle_report.requires_host_attention());
        assert!(idle_report.is_no_host_attention());
        assert_eq!(
            idle_report.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::NoAttention
        );
        assert_eq!(idle_report.host_attention_label(), "no_attention");
        assert!(idle_report.host_attention_label_matches_kind());
        assert!(!idle_report.host_attention_includes_scheduler_action());
        assert!(!idle_report.host_attention_includes_host_investigation());
        assert!(!idle_report.host_attention_includes_run_integrity_investigation());
        assert!(!idle_report.has_multiple_host_attention());
        assert!(!idle_summary.requires_host_attention);
        assert!(!idle_summary.requires_host_attention());
        assert!(idle_summary.is_no_host_attention);
        assert!(idle_summary.is_no_host_attention());
        assert_eq!(
            idle_summary.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::NoAttention
        );
        assert_eq!(idle_summary.host_attention_label(), "no_attention");
        assert!(idle_summary.host_attention_label_matches_kind());
        assert!(!idle_summary.host_attention_includes_scheduler_action());
        assert!(!idle_summary.host_attention_includes_host_investigation());
        assert!(!idle_summary.host_attention_includes_run_integrity_investigation());
        assert!(!idle_summary.has_multiple_host_attention());

        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut sink)
            .unwrap();
        let summary = report.summary();

        assert!(report.requires_host_attention());
        assert!(!report.is_no_host_attention());
        assert_eq!(
            report.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction
        );
        assert_eq!(report.host_attention_label(), "scheduler_action");
        assert!(report.host_attention_label_matches_kind());
        assert!(report.host_attention_includes_scheduler_action());
        assert!(!report.host_attention_includes_host_investigation());
        assert!(!report.host_attention_includes_run_integrity_investigation());
        assert!(!report.has_multiple_host_attention());
        assert!(summary.requires_host_attention);
        assert!(!summary.is_no_host_attention);
        assert_eq!(
            summary.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerAction
        );
        assert_eq!(summary.host_attention_label(), "scheduler_action");
        assert!(summary.host_attention_label_matches_kind());
        assert!(summary.host_attention_includes_scheduler_action());
        assert!(!summary.host_attention_includes_host_investigation());
        assert!(!summary.host_attention_includes_run_integrity_investigation());
        assert!(!summary.has_multiple_host_attention());

        let mut integrity_report = report.clone();
        integrity_report.drain.ticks[0].replay.next_checkpoint =
            ToolAuditReadCheckpoint::beginning();
        let integrity_summary = integrity_report.summary();
        assert!(integrity_report.requires_scheduler_action());
        assert!(!integrity_report.requires_host_investigation());
        assert!(integrity_report.requires_run_integrity_investigation());
        assert_eq!(
            integrity_report.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndRunIntegrityInvestigation
        );
        assert_eq!(
            integrity_report.host_attention_label(),
            "scheduler_action_and_run_integrity_investigation"
        );
        assert!(integrity_report.host_attention_includes_scheduler_action());
        assert!(!integrity_report.host_attention_includes_host_investigation());
        assert!(integrity_report.host_attention_includes_run_integrity_investigation());
        assert!(integrity_report.has_multiple_host_attention());
        assert_eq!(
            integrity_summary.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionAndRunIntegrityInvestigation
        );
        assert!(integrity_summary.host_attention_includes_scheduler_action());
        assert!(!integrity_summary.host_attention_includes_host_investigation());
        assert!(integrity_summary.host_attention_includes_run_integrity_investigation());
        assert!(integrity_summary.has_multiple_host_attention());

        let mut stale_investigation_summary = idle_summary.clone();
        stale_investigation_summary.requires_host_investigation = true;
        stale_investigation_summary.requires_host_attention = true;
        stale_investigation_summary.is_no_host_attention = false;
        stale_investigation_summary.host_attention_kind =
            ToolAuditSupervisorDrainHostAttentionKind::HostInvestigation;
        stale_investigation_summary.host_attention_label = "host_investigation";
        stale_investigation_summary.host_attention_includes_host_investigation = true;
        assert!(stale_investigation_summary.requires_host_attention());
        assert!(!stale_investigation_summary.is_no_host_attention());
        assert_eq!(
            stale_investigation_summary.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::HostInvestigation
        );
        assert_eq!(
            stale_investigation_summary.host_attention_label(),
            "host_investigation"
        );
        assert!(stale_investigation_summary.host_attention_label_matches_kind());
        assert!(!stale_investigation_summary.host_attention_includes_scheduler_action());
        assert!(stale_investigation_summary.host_attention_includes_host_investigation());
        assert!(!stale_investigation_summary.host_attention_includes_run_integrity_investigation());
        assert!(!stale_investigation_summary.has_multiple_host_attention());

        let mut stale_integrity_summary = idle_summary.clone();
        stale_integrity_summary.requires_run_integrity_investigation = true;
        stale_integrity_summary.requires_host_attention = true;
        stale_integrity_summary.is_no_host_attention = false;
        stale_integrity_summary.host_attention_kind =
            ToolAuditSupervisorDrainHostAttentionKind::RunIntegrityInvestigation;
        stale_integrity_summary.host_attention_label = "run_integrity_investigation";
        stale_integrity_summary.host_attention_includes_run_integrity_investigation = true;
        assert!(stale_integrity_summary.requires_host_attention());
        assert_eq!(
            stale_integrity_summary.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::RunIntegrityInvestigation
        );
        assert_eq!(
            stale_integrity_summary.host_attention_label(),
            "run_integrity_investigation"
        );
        assert!(stale_integrity_summary.host_attention_label_matches_kind());
        assert!(!stale_integrity_summary.host_attention_includes_scheduler_action());
        assert!(!stale_integrity_summary.host_attention_includes_host_investigation());
        assert!(stale_integrity_summary.host_attention_includes_run_integrity_investigation());
        assert!(!stale_integrity_summary.has_multiple_host_attention());

        let mut stale_multiple_summary = summary.clone();
        stale_multiple_summary.requires_host_investigation = true;
        stale_multiple_summary.requires_run_integrity_investigation = true;
        stale_multiple_summary.host_attention_kind =
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionHostAndRunIntegrityInvestigation;
        stale_multiple_summary.host_attention_label =
            "scheduler_action_host_and_run_integrity_investigation";
        stale_multiple_summary.host_attention_includes_host_investigation = true;
        stale_multiple_summary.host_attention_includes_run_integrity_investigation = true;
        stale_multiple_summary.has_multiple_host_attention = true;
        assert!(stale_multiple_summary.requires_host_attention());
        assert_eq!(
            stale_multiple_summary.host_attention_kind(),
            ToolAuditSupervisorDrainHostAttentionKind::SchedulerActionHostAndRunIntegrityInvestigation
        );
        assert_eq!(
            stale_multiple_summary.host_attention_label(),
            "scheduler_action_host_and_run_integrity_investigation"
        );
        assert!(stale_multiple_summary.host_attention_label_matches_kind());
        assert!(stale_multiple_summary.host_attention_includes_scheduler_action());
        assert!(stale_multiple_summary.host_attention_includes_host_investigation());
        assert!(stale_multiple_summary.host_attention_includes_run_integrity_investigation());
        assert!(stale_multiple_summary.has_multiple_host_attention());

        stale_multiple_summary.host_attention_label = "attentionish";
        stale_multiple_summary.host_attention_label_matches_kind = false;
        assert!(!stale_multiple_summary.host_attention_label_matches_kind());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_terminal_readiness_fields() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert!(idle_report.is_terminal_ready());
        assert_eq!(
            idle_report.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::IdleReady
        );
        assert_eq!(idle_report.terminal_readiness_label(), "idle_ready");
        assert!(idle_report.terminal_readiness_label_matches_kind());
        assert!(idle_report.terminal_readiness_is_idle());
        assert!(!idle_report.terminal_readiness_is_caught_up());
        assert!(!idle_report.terminal_readiness_requires_scheduler_action());
        assert!(!idle_report.terminal_readiness_requires_host_investigation());
        assert!(!idle_report.terminal_readiness_requires_run_integrity_investigation());
        assert!(!idle_report.terminal_readiness_has_multiple_attention());
        assert!(idle_summary.is_terminal_ready);
        assert!(idle_summary.is_terminal_ready());
        assert_eq!(
            idle_summary.terminal_readiness_kind,
            ToolAuditSupervisorDrainTerminalReadinessKind::IdleReady
        );
        assert_eq!(
            idle_summary.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::IdleReady
        );
        assert_eq!(idle_summary.terminal_readiness_label, "idle_ready");
        assert_eq!(idle_summary.terminal_readiness_label(), "idle_ready");
        assert!(idle_summary.terminal_readiness_label_matches_kind);
        assert!(idle_summary.terminal_readiness_label_matches_kind());
        assert!(idle_summary.terminal_readiness_is_idle);
        assert!(idle_summary.terminal_readiness_is_idle());
        assert!(!idle_summary.terminal_readiness_is_caught_up);
        assert!(!idle_summary.terminal_readiness_is_caught_up());
        assert!(!idle_summary.terminal_readiness_requires_scheduler_action);
        assert!(!idle_summary.terminal_readiness_requires_scheduler_action());
        assert!(!idle_summary.terminal_readiness_requires_host_investigation);
        assert!(!idle_summary.terminal_readiness_requires_host_investigation());
        assert!(!idle_summary.terminal_readiness_requires_run_integrity_investigation);
        assert!(!idle_summary.terminal_readiness_requires_run_integrity_investigation());
        assert!(!idle_summary.terminal_readiness_has_multiple_attention);
        assert!(!idle_summary.terminal_readiness_has_multiple_attention());

        let ready_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(ready_store
            .record_audit_batch(vec![sample_record("call_1"), sample_record("call_2")])
            .completed_without_failures());
        let mut ready_sink = InMemoryToolAuditSink::new();
        let ready_report = ready_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut ready_sink)
            .unwrap();
        let ready_summary = ready_report.summary();
        assert!(ready_report.is_terminal_ready());
        assert_eq!(
            ready_report.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::CaughtUpReady
        );
        assert_eq!(ready_report.terminal_readiness_label(), "caught_up_ready");
        assert!(!ready_report.terminal_readiness_is_idle());
        assert!(ready_report.terminal_readiness_is_caught_up());
        assert!(ready_summary.is_terminal_ready);
        assert_eq!(
            ready_summary.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::CaughtUpReady
        );
        assert_eq!(ready_summary.terminal_readiness_label(), "caught_up_ready");
        assert!(!ready_summary.terminal_readiness_is_idle());
        assert!(ready_summary.terminal_readiness_is_caught_up());

        let pending_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(pending_store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());
        let mut pending_sink = InMemoryToolAuditSink::new();
        let pending_report = pending_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut pending_sink)
            .unwrap();
        let pending_summary = pending_report.summary();
        assert!(!pending_report.is_terminal_ready());
        assert_eq!(
            pending_report.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionPending
        );
        assert_eq!(
            pending_report.terminal_readiness_label(),
            "scheduler_action_pending"
        );
        assert!(pending_report.terminal_readiness_requires_scheduler_action());
        assert!(!pending_report.terminal_readiness_requires_host_investigation());
        assert!(!pending_report.terminal_readiness_requires_run_integrity_investigation());
        assert!(!pending_report.terminal_readiness_has_multiple_attention());
        assert!(!pending_summary.is_terminal_ready);
        assert_eq!(
            pending_summary.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionPending
        );
        assert_eq!(
            pending_summary.terminal_readiness_label(),
            "scheduler_action_pending"
        );
        assert!(pending_summary.terminal_readiness_requires_scheduler_action());
        assert!(!pending_summary.terminal_readiness_requires_host_investigation());
        assert!(!pending_summary.terminal_readiness_requires_run_integrity_investigation());
        assert!(!pending_summary.terminal_readiness_has_multiple_attention());

        let mut integrity_report = pending_report.clone();
        integrity_report.drain.ticks[0].replay.next_checkpoint =
            ToolAuditReadCheckpoint::beginning();
        let integrity_summary = integrity_report.summary();
        assert_eq!(
            integrity_report.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionAndRunIntegrityInvestigationPending
        );
        assert_eq!(
            integrity_report.terminal_readiness_label(),
            "scheduler_action_and_run_integrity_investigation_pending"
        );
        assert!(integrity_report.terminal_readiness_requires_scheduler_action());
        assert!(!integrity_report.terminal_readiness_requires_host_investigation());
        assert!(integrity_report.terminal_readiness_requires_run_integrity_investigation());
        assert!(integrity_report.terminal_readiness_has_multiple_attention());
        assert_eq!(
            integrity_summary.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionAndRunIntegrityInvestigationPending
        );
        assert_eq!(
            integrity_summary.terminal_readiness_label(),
            "scheduler_action_and_run_integrity_investigation_pending"
        );
        assert!(integrity_summary.terminal_readiness_requires_scheduler_action());
        assert!(!integrity_summary.terminal_readiness_requires_host_investigation());
        assert!(integrity_summary.terminal_readiness_requires_run_integrity_investigation());
        assert!(integrity_summary.terminal_readiness_has_multiple_attention());

        let mut stale_host_summary = idle_summary.clone();
        stale_host_summary.is_terminal_ready = false;
        stale_host_summary.terminal_readiness_kind =
            ToolAuditSupervisorDrainTerminalReadinessKind::HostInvestigationPending;
        stale_host_summary.terminal_readiness_label = "host_investigation_pending";
        stale_host_summary.terminal_readiness_requires_host_investigation = true;
        stale_host_summary.terminal_readiness_is_idle = false;
        assert!(!stale_host_summary.is_terminal_ready());
        assert_eq!(
            stale_host_summary.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::HostInvestigationPending
        );
        assert_eq!(
            stale_host_summary.terminal_readiness_label(),
            "host_investigation_pending"
        );
        assert!(stale_host_summary.terminal_readiness_label_matches_kind());
        assert!(!stale_host_summary.terminal_readiness_requires_scheduler_action());
        assert!(stale_host_summary.terminal_readiness_requires_host_investigation());
        assert!(!stale_host_summary.terminal_readiness_requires_run_integrity_investigation());
        assert!(!stale_host_summary.terminal_readiness_has_multiple_attention());

        let mut stale_multiple_summary = pending_summary;
        stale_multiple_summary.terminal_readiness_kind =
            ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionHostAndRunIntegrityInvestigationPending;
        stale_multiple_summary.terminal_readiness_label =
            "scheduler_action_host_and_run_integrity_investigation_pending";
        stale_multiple_summary.terminal_readiness_requires_host_investigation = true;
        stale_multiple_summary.terminal_readiness_requires_run_integrity_investigation = true;
        stale_multiple_summary.terminal_readiness_has_multiple_attention = true;
        assert_eq!(
            stale_multiple_summary.terminal_readiness_kind(),
            ToolAuditSupervisorDrainTerminalReadinessKind::SchedulerActionHostAndRunIntegrityInvestigationPending
        );
        assert_eq!(
            stale_multiple_summary.terminal_readiness_label(),
            "scheduler_action_host_and_run_integrity_investigation_pending"
        );
        assert!(stale_multiple_summary.terminal_readiness_requires_scheduler_action());
        assert!(stale_multiple_summary.terminal_readiness_requires_host_investigation());
        assert!(stale_multiple_summary.terminal_readiness_requires_run_integrity_investigation());
        assert!(stale_multiple_summary.terminal_readiness_has_multiple_attention());
        assert!(stale_multiple_summary.terminal_readiness_label_matches_kind());

        stale_multiple_summary.terminal_readiness_label = "readinessish";
        stale_multiple_summary.terminal_readiness_label_matches_kind = false;
        assert!(!stale_multiple_summary.terminal_readiness_label_matches_kind());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_host_decision_fields() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert!(!idle_report.requires_host_decision());
        assert_eq!(
            idle_report.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::TerminalReady
        );
        assert_eq!(idle_report.host_decision_label(), "terminal_ready");
        assert!(idle_report.host_decision_label_matches_kind());
        assert!(idle_report.host_decision_is_terminal_ready());
        assert!(!idle_report.host_decision_schedules_continuation());
        assert!(!idle_report.host_decision_routes_follow_up());
        assert!(!idle_report.host_decision_investigates_plan_drift());
        assert!(!idle_report.host_decision_investigates_count_drift());
        assert!(!idle_report.host_decision_investigates_run_integrity());
        assert!(!idle_report.host_decision_has_multiple_actions());
        assert_eq!(idle_report.host_decision_action_count(), 0);
        assert!(!idle_report.host_decision_has_single_action());
        assert_eq!(
            idle_report.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::Terminal
        );
        assert_eq!(idle_report.host_decision_lane_label(), "terminal");
        assert!(idle_report.host_decision_lane_label_matches_kind());
        assert!(idle_report.host_decision_uses_terminal_lane());
        assert!(!idle_report.host_decision_uses_scheduler_lane());
        assert!(!idle_report.host_decision_uses_follow_up_lane());
        assert!(!idle_report.host_decision_uses_investigation_lane());
        assert!(!idle_report.host_decision_uses_mixed_lane());
        assert_eq!(
            idle_report.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::Settled
        );
        assert_eq!(idle_report.host_decision_priority_label(), "settled");
        assert!(idle_report.host_decision_priority_label_matches_kind());
        assert_eq!(idle_report.host_decision_priority_rank(), 0);
        assert!(idle_report.host_decision_has_settled_priority());
        assert!(!idle_report.host_decision_has_routine_priority());
        assert!(!idle_report.host_decision_has_drift_investigation_priority());
        assert!(!idle_report.host_decision_has_integrity_investigation_priority());
        assert!(!idle_report.host_decision_has_mixed_priority());
        assert!(!idle_report.host_decision_has_investigation_priority());
        assert!(!idle_summary.requires_host_decision);
        assert!(!idle_summary.requires_host_decision());
        assert_eq!(
            idle_summary.host_decision_kind,
            ToolAuditSupervisorDrainHostDecisionKind::TerminalReady
        );
        assert_eq!(
            idle_summary.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::TerminalReady
        );
        assert_eq!(idle_summary.host_decision_label, "terminal_ready");
        assert_eq!(idle_summary.host_decision_label(), "terminal_ready");
        assert!(idle_summary.host_decision_label_matches_kind);
        assert!(idle_summary.host_decision_label_matches_kind());
        assert!(idle_summary.host_decision_is_terminal_ready);
        assert!(idle_summary.host_decision_is_terminal_ready());
        assert!(!idle_summary.host_decision_schedules_continuation);
        assert!(!idle_summary.host_decision_schedules_continuation());
        assert!(!idle_summary.host_decision_routes_follow_up);
        assert!(!idle_summary.host_decision_routes_follow_up());
        assert!(!idle_summary.host_decision_investigates_plan_drift);
        assert!(!idle_summary.host_decision_investigates_plan_drift());
        assert!(!idle_summary.host_decision_investigates_count_drift);
        assert!(!idle_summary.host_decision_investigates_count_drift());
        assert!(!idle_summary.host_decision_investigates_run_integrity);
        assert!(!idle_summary.host_decision_investigates_run_integrity());
        assert!(!idle_summary.host_decision_has_multiple_actions);
        assert!(!idle_summary.host_decision_has_multiple_actions());
        assert_eq!(idle_summary.host_decision_action_count, 0);
        assert_eq!(idle_summary.host_decision_action_count(), 0);
        assert!(!idle_summary.host_decision_has_single_action);
        assert!(!idle_summary.host_decision_has_single_action());
        assert_eq!(
            idle_summary.host_decision_lane,
            ToolAuditSupervisorDrainHostDecisionLane::Terminal
        );
        assert_eq!(
            idle_summary.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::Terminal
        );
        assert_eq!(idle_summary.host_decision_lane_label, "terminal");
        assert_eq!(idle_summary.host_decision_lane_label(), "terminal");
        assert!(idle_summary.host_decision_lane_label_matches_kind);
        assert!(idle_summary.host_decision_lane_label_matches_kind());
        assert!(idle_summary.host_decision_uses_terminal_lane);
        assert!(idle_summary.host_decision_uses_terminal_lane());
        assert!(!idle_summary.host_decision_uses_scheduler_lane);
        assert!(!idle_summary.host_decision_uses_scheduler_lane());
        assert!(!idle_summary.host_decision_uses_follow_up_lane);
        assert!(!idle_summary.host_decision_uses_follow_up_lane());
        assert!(!idle_summary.host_decision_uses_investigation_lane);
        assert!(!idle_summary.host_decision_uses_investigation_lane());
        assert!(!idle_summary.host_decision_uses_mixed_lane);
        assert!(!idle_summary.host_decision_uses_mixed_lane());
        assert_eq!(
            idle_summary.host_decision_priority,
            ToolAuditSupervisorDrainHostDecisionPriority::Settled
        );
        assert_eq!(
            idle_summary.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::Settled
        );
        assert_eq!(idle_summary.host_decision_priority_label, "settled");
        assert_eq!(idle_summary.host_decision_priority_label(), "settled");
        assert!(idle_summary.host_decision_priority_label_matches_kind);
        assert!(idle_summary.host_decision_priority_label_matches_kind());
        assert_eq!(idle_summary.host_decision_priority_rank, 0);
        assert_eq!(idle_summary.host_decision_priority_rank(), 0);
        assert!(idle_summary.host_decision_has_settled_priority);
        assert!(idle_summary.host_decision_has_settled_priority());
        assert!(!idle_summary.host_decision_has_routine_priority);
        assert!(!idle_summary.host_decision_has_routine_priority());
        assert!(!idle_summary.host_decision_has_drift_investigation_priority);
        assert!(!idle_summary.host_decision_has_drift_investigation_priority());
        assert!(!idle_summary.host_decision_has_integrity_investigation_priority);
        assert!(!idle_summary.host_decision_has_integrity_investigation_priority());
        assert!(!idle_summary.host_decision_has_mixed_priority);
        assert!(!idle_summary.host_decision_has_mixed_priority());
        assert!(!idle_summary.host_decision_has_investigation_priority);
        assert!(!idle_summary.host_decision_has_investigation_priority());

        let continuation_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(continuation_store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());
        let mut continuation_sink = InMemoryToolAuditSink::new();
        let continuation_report = continuation_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut continuation_sink)
            .unwrap();
        let continuation_summary = continuation_report.summary();

        assert!(continuation_report.requires_host_decision());
        assert_eq!(
            continuation_report.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation
        );
        assert_eq!(
            continuation_report.host_decision_label(),
            "schedule_continuation"
        );
        assert!(continuation_report.host_decision_label_matches_kind());
        assert!(!continuation_report.host_decision_is_terminal_ready());
        assert!(continuation_report.host_decision_schedules_continuation());
        assert!(!continuation_report.host_decision_routes_follow_up());
        assert!(!continuation_report.host_decision_investigates_plan_drift());
        assert!(!continuation_report.host_decision_investigates_count_drift());
        assert!(!continuation_report.host_decision_investigates_run_integrity());
        assert!(!continuation_report.host_decision_has_multiple_actions());
        assert_eq!(continuation_report.host_decision_action_count(), 1);
        assert!(continuation_report.host_decision_has_single_action());
        assert_eq!(
            continuation_report.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::Scheduler
        );
        assert_eq!(continuation_report.host_decision_lane_label(), "scheduler");
        assert!(!continuation_report.host_decision_uses_terminal_lane());
        assert!(continuation_report.host_decision_uses_scheduler_lane());
        assert!(!continuation_report.host_decision_uses_follow_up_lane());
        assert!(!continuation_report.host_decision_uses_investigation_lane());
        assert!(!continuation_report.host_decision_uses_mixed_lane());
        assert_eq!(
            continuation_report.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::RoutineAction
        );
        assert_eq!(
            continuation_report.host_decision_priority_label(),
            "routine_action"
        );
        assert!(continuation_report.host_decision_priority_label_matches_kind());
        assert_eq!(continuation_report.host_decision_priority_rank(), 10);
        assert!(!continuation_report.host_decision_has_settled_priority());
        assert!(continuation_report.host_decision_has_routine_priority());
        assert!(!continuation_report.host_decision_has_investigation_priority());
        assert!(continuation_summary.requires_host_decision);
        assert_eq!(
            continuation_summary.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::ScheduleContinuation
        );
        assert_eq!(
            continuation_summary.host_decision_label(),
            "schedule_continuation"
        );
        assert!(continuation_summary.host_decision_schedules_continuation());
        assert!(!continuation_summary.host_decision_routes_follow_up());
        assert!(!continuation_summary.host_decision_investigates_run_integrity());
        assert!(!continuation_summary.host_decision_has_multiple_actions());
        assert_eq!(continuation_summary.host_decision_action_count(), 1);
        assert!(continuation_summary.host_decision_has_single_action());
        assert_eq!(
            continuation_summary.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::Scheduler
        );
        assert_eq!(continuation_summary.host_decision_lane_label(), "scheduler");
        assert!(continuation_summary.host_decision_uses_scheduler_lane());
        assert!(!continuation_summary.host_decision_uses_mixed_lane());
        assert_eq!(
            continuation_summary.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::RoutineAction
        );
        assert_eq!(
            continuation_summary.host_decision_priority_label(),
            "routine_action"
        );
        assert_eq!(continuation_summary.host_decision_priority_rank(), 10);
        assert!(continuation_summary.host_decision_has_routine_priority());
        assert!(!continuation_summary.host_decision_has_investigation_priority());

        let follow_up_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(follow_up_store
            .record_audit_batch(vec![failed_record("call_1")])
            .completed_without_failures());
        let mut follow_up_sink = InMemoryToolAuditSink::new();
        let follow_up_report = follow_up_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut follow_up_sink)
            .unwrap();
        let follow_up_summary = follow_up_report.summary();

        assert_eq!(
            follow_up_report.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp
        );
        assert_eq!(follow_up_report.host_decision_label(), "route_follow_up");
        assert!(follow_up_report.host_decision_routes_follow_up());
        assert!(!follow_up_report.host_decision_schedules_continuation());
        assert!(!follow_up_report.host_decision_has_multiple_actions());
        assert_eq!(follow_up_report.host_decision_action_count(), 1);
        assert!(follow_up_report.host_decision_has_single_action());
        assert_eq!(
            follow_up_report.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::FollowUp
        );
        assert_eq!(follow_up_report.host_decision_lane_label(), "follow_up");
        assert!(follow_up_report.host_decision_uses_follow_up_lane());
        assert_eq!(
            follow_up_report.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::RoutineAction
        );
        assert_eq!(follow_up_report.host_decision_priority_rank(), 10);
        assert!(follow_up_report.host_decision_has_routine_priority());
        assert_eq!(
            follow_up_summary.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::RouteFollowUp
        );
        assert!(follow_up_summary.host_decision_routes_follow_up());
        assert!(!follow_up_summary.host_decision_schedules_continuation());
        assert_eq!(follow_up_summary.host_decision_action_count(), 1);
        assert!(follow_up_summary.host_decision_has_single_action());
        assert_eq!(
            follow_up_summary.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::FollowUp
        );
        assert!(follow_up_summary.host_decision_uses_follow_up_lane());
        assert_eq!(
            follow_up_summary.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::RoutineAction
        );
        assert!(follow_up_summary.host_decision_has_routine_priority());

        let mut integrity_report = continuation_report;
        integrity_report.drain.ticks[0].replay.next_checkpoint =
            ToolAuditReadCheckpoint::beginning();
        let integrity_summary = integrity_report.summary();
        assert_eq!(
            integrity_report.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::MultipleActions
        );
        assert_eq!(integrity_report.host_decision_label(), "multiple_actions");
        assert!(integrity_report.host_decision_schedules_continuation());
        assert!(!integrity_report.host_decision_routes_follow_up());
        assert!(!integrity_report.host_decision_investigates_plan_drift());
        assert!(!integrity_report.host_decision_investigates_count_drift());
        assert!(integrity_report.host_decision_investigates_run_integrity());
        assert!(integrity_report.host_decision_has_multiple_actions());
        assert_eq!(integrity_report.host_decision_action_count(), 2);
        assert!(!integrity_report.host_decision_has_single_action());
        assert_eq!(
            integrity_report.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::Mixed
        );
        assert_eq!(integrity_report.host_decision_lane_label(), "mixed");
        assert!(!integrity_report.host_decision_uses_scheduler_lane());
        assert!(integrity_report.host_decision_uses_mixed_lane());
        assert_eq!(
            integrity_report.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::MixedAction
        );
        assert_eq!(
            integrity_report.host_decision_priority_label(),
            "mixed_action"
        );
        assert_eq!(integrity_report.host_decision_priority_rank(), 90);
        assert!(integrity_report.host_decision_has_mixed_priority());
        assert!(integrity_report.host_decision_has_investigation_priority());
        assert_eq!(
            integrity_summary.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::MultipleActions
        );
        assert_eq!(integrity_summary.host_decision_label(), "multiple_actions");
        assert!(integrity_summary.host_decision_schedules_continuation());
        assert!(!integrity_summary.host_decision_routes_follow_up());
        assert!(integrity_summary.host_decision_investigates_run_integrity());
        assert!(integrity_summary.host_decision_has_multiple_actions());
        assert_eq!(integrity_summary.host_decision_action_count(), 2);
        assert!(!integrity_summary.host_decision_has_single_action());
        assert_eq!(
            integrity_summary.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::Mixed
        );
        assert!(integrity_summary.host_decision_uses_mixed_lane());
        assert_eq!(
            integrity_summary.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::MixedAction
        );
        assert_eq!(integrity_summary.host_decision_priority_rank(), 90);
        assert!(integrity_summary.host_decision_has_mixed_priority());
        assert!(integrity_summary.host_decision_has_investigation_priority());

        let mut stale_plan_summary = idle_summary.clone();
        stale_plan_summary.requires_host_decision = true;
        stale_plan_summary.host_decision_kind =
            ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift;
        stale_plan_summary.host_decision_label = "investigate_plan_drift";
        stale_plan_summary.host_decision_is_terminal_ready = false;
        stale_plan_summary.host_decision_investigates_plan_drift = true;
        stale_plan_summary.host_decision_action_count = 1;
        stale_plan_summary.host_decision_has_single_action = true;
        stale_plan_summary.host_decision_lane =
            ToolAuditSupervisorDrainHostDecisionLane::Investigation;
        stale_plan_summary.host_decision_lane_label = "investigation";
        stale_plan_summary.host_decision_uses_terminal_lane = false;
        stale_plan_summary.host_decision_uses_investigation_lane = true;
        stale_plan_summary.host_decision_priority =
            ToolAuditSupervisorDrainHostDecisionPriority::DriftInvestigation;
        stale_plan_summary.host_decision_priority_label = "drift_investigation";
        stale_plan_summary.host_decision_priority_rank = 50;
        stale_plan_summary.host_decision_has_settled_priority = false;
        stale_plan_summary.host_decision_has_drift_investigation_priority = true;
        stale_plan_summary.host_decision_has_investigation_priority = true;
        assert!(stale_plan_summary.requires_host_decision());
        assert_eq!(
            stale_plan_summary.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::InvestigatePlanDrift
        );
        assert_eq!(
            stale_plan_summary.host_decision_label(),
            "investigate_plan_drift"
        );
        assert!(stale_plan_summary.host_decision_label_matches_kind());
        assert!(!stale_plan_summary.host_decision_is_terminal_ready());
        assert!(stale_plan_summary.host_decision_investigates_plan_drift());
        assert!(!stale_plan_summary.host_decision_has_multiple_actions());
        assert_eq!(stale_plan_summary.host_decision_action_count(), 1);
        assert!(stale_plan_summary.host_decision_has_single_action());
        assert_eq!(
            stale_plan_summary.host_decision_lane(),
            ToolAuditSupervisorDrainHostDecisionLane::Investigation
        );
        assert_eq!(
            stale_plan_summary.host_decision_lane_label(),
            "investigation"
        );
        assert!(stale_plan_summary.host_decision_lane_label_matches_kind());
        assert!(stale_plan_summary.host_decision_uses_investigation_lane());
        assert_eq!(
            stale_plan_summary.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::DriftInvestigation
        );
        assert_eq!(
            stale_plan_summary.host_decision_priority_label(),
            "drift_investigation"
        );
        assert!(stale_plan_summary.host_decision_priority_label_matches_kind());
        assert_eq!(stale_plan_summary.host_decision_priority_rank(), 50);
        assert!(stale_plan_summary.host_decision_has_drift_investigation_priority());
        assert!(stale_plan_summary.host_decision_has_investigation_priority());

        let mut stale_count_summary = idle_summary.clone();
        stale_count_summary.requires_host_decision = true;
        stale_count_summary.host_decision_kind =
            ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift;
        stale_count_summary.host_decision_label = "investigate_count_drift";
        stale_count_summary.host_decision_is_terminal_ready = false;
        stale_count_summary.host_decision_investigates_count_drift = true;
        stale_count_summary.host_decision_priority =
            ToolAuditSupervisorDrainHostDecisionPriority::DriftInvestigation;
        stale_count_summary.host_decision_priority_label = "drift_investigation";
        stale_count_summary.host_decision_priority_rank = 50;
        stale_count_summary.host_decision_has_drift_investigation_priority = true;
        stale_count_summary.host_decision_has_investigation_priority = true;
        assert_eq!(
            stale_count_summary.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::InvestigateCountDrift
        );
        assert_eq!(
            stale_count_summary.host_decision_label(),
            "investigate_count_drift"
        );
        assert!(stale_count_summary.host_decision_investigates_count_drift());
        assert_eq!(
            stale_count_summary.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::DriftInvestigation
        );
        assert!(stale_count_summary.host_decision_has_drift_investigation_priority());

        let mut stale_integrity_summary = idle_summary;
        stale_integrity_summary.requires_host_decision = true;
        stale_integrity_summary.host_decision_kind =
            ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity;
        stale_integrity_summary.host_decision_label = "investigate_run_integrity";
        stale_integrity_summary.host_decision_is_terminal_ready = false;
        stale_integrity_summary.host_decision_investigates_run_integrity = true;
        stale_integrity_summary.host_decision_priority =
            ToolAuditSupervisorDrainHostDecisionPriority::IntegrityInvestigation;
        stale_integrity_summary.host_decision_priority_label = "integrity_investigation";
        stale_integrity_summary.host_decision_priority_rank = 80;
        stale_integrity_summary.host_decision_has_integrity_investigation_priority = true;
        stale_integrity_summary.host_decision_has_investigation_priority = true;
        assert_eq!(
            stale_integrity_summary.host_decision_kind(),
            ToolAuditSupervisorDrainHostDecisionKind::InvestigateRunIntegrity
        );
        assert_eq!(
            stale_integrity_summary.host_decision_label(),
            "investigate_run_integrity"
        );
        assert!(stale_integrity_summary.host_decision_investigates_run_integrity());
        assert_eq!(
            stale_integrity_summary.host_decision_priority(),
            ToolAuditSupervisorDrainHostDecisionPriority::IntegrityInvestigation
        );
        assert_eq!(stale_integrity_summary.host_decision_priority_rank(), 80);
        assert!(stale_integrity_summary.host_decision_has_integrity_investigation_priority());
        assert!(stale_integrity_summary.host_decision_has_investigation_priority());

        stale_integrity_summary.host_decision_label = "decisionish";
        stale_integrity_summary.host_decision_label_matches_kind = false;
        assert!(!stale_integrity_summary.host_decision_label_matches_kind());
        stale_integrity_summary.host_decision_lane_label = "sidewalk";
        stale_integrity_summary.host_decision_lane_label_matches_kind = false;
        assert!(!stale_integrity_summary.host_decision_lane_label_matches_kind());
        stale_integrity_summary.host_decision_priority_label = "someday";
        stale_integrity_summary.host_decision_priority_label_matches_kind = false;
        assert!(!stale_integrity_summary.host_decision_priority_label_matches_kind());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_host_decision_readiness_fields() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert_eq!(
            idle_report.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::Settled
        );
        assert_eq!(idle_report.host_decision_readiness_label(), "settled");
        assert!(idle_report.host_decision_readiness_label_matches_kind());
        assert!(idle_report.host_decision_is_settled());
        assert!(!idle_report.host_decision_is_actionable());
        assert!(!idle_report.host_decision_is_auto_routable());
        assert!(!idle_report.host_decision_readiness_requires_manual_review());
        assert!(!idle_report.host_decision_readiness_requires_investigation());
        assert!(!idle_report.host_decision_readiness_requires_integrity_investigation());
        assert!(!idle_report.host_decision_needs_triage());
        assert_eq!(
            idle_summary.host_decision_readiness,
            ToolAuditSupervisorDrainHostDecisionReadiness::Settled
        );
        assert_eq!(
            idle_summary.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::Settled
        );
        assert_eq!(idle_summary.host_decision_readiness_label, "settled");
        assert_eq!(idle_summary.host_decision_readiness_label(), "settled");
        assert!(idle_summary.host_decision_readiness_label_matches_kind);
        assert!(idle_summary.host_decision_readiness_label_matches_kind());
        assert!(idle_summary.host_decision_is_settled);
        assert!(idle_summary.host_decision_is_settled());
        assert!(!idle_summary.host_decision_is_actionable);
        assert!(!idle_summary.host_decision_is_actionable());
        assert!(!idle_summary.host_decision_is_auto_routable);
        assert!(!idle_summary.host_decision_is_auto_routable());
        assert!(!idle_summary.host_decision_readiness_requires_manual_review);
        assert!(!idle_summary.host_decision_readiness_requires_manual_review());
        assert!(!idle_summary.host_decision_readiness_requires_investigation);
        assert!(!idle_summary.host_decision_readiness_requires_investigation());
        assert!(!idle_summary.host_decision_readiness_requires_integrity_investigation);
        assert!(!idle_summary.host_decision_readiness_requires_integrity_investigation());
        assert!(!idle_summary.host_decision_needs_triage);
        assert!(!idle_summary.host_decision_needs_triage());

        let continuation_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(continuation_store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());
        let mut continuation_sink = InMemoryToolAuditSink::new();
        let continuation_report = continuation_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut continuation_sink)
            .unwrap();
        let continuation_summary = continuation_report.summary();

        assert_eq!(
            continuation_report.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::RoutineReady
        );
        assert_eq!(
            continuation_report.host_decision_readiness_label(),
            "routine_ready"
        );
        assert!(continuation_report.host_decision_readiness_label_matches_kind());
        assert!(!continuation_report.host_decision_is_settled());
        assert!(continuation_report.host_decision_is_actionable());
        assert!(continuation_report.host_decision_is_auto_routable());
        assert!(!continuation_report.host_decision_readiness_requires_manual_review());
        assert!(!continuation_report.host_decision_readiness_requires_investigation());
        assert!(!continuation_report.host_decision_needs_triage());
        assert_eq!(
            continuation_summary.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::RoutineReady
        );
        assert_eq!(
            continuation_summary.host_decision_readiness_label(),
            "routine_ready"
        );
        assert!(continuation_summary.host_decision_is_actionable());
        assert!(continuation_summary.host_decision_is_auto_routable());
        assert!(!continuation_summary.host_decision_readiness_requires_manual_review());
        assert!(!continuation_summary.host_decision_needs_triage());

        let mut integrity_report = continuation_report;
        integrity_report.drain.ticks[0].replay.next_checkpoint =
            ToolAuditReadCheckpoint::beginning();
        let triage_summary = integrity_report.summary();

        assert_eq!(
            integrity_report.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::TriageRequired
        );
        assert_eq!(
            integrity_report.host_decision_readiness_label(),
            "triage_required"
        );
        assert!(integrity_report.host_decision_is_actionable());
        assert!(!integrity_report.host_decision_is_auto_routable());
        assert!(integrity_report.host_decision_readiness_requires_manual_review());
        assert!(!integrity_report.host_decision_readiness_requires_investigation());
        assert!(!integrity_report.host_decision_readiness_requires_integrity_investigation());
        assert!(integrity_report.host_decision_needs_triage());
        assert_eq!(
            triage_summary.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::TriageRequired
        );
        assert_eq!(
            triage_summary.host_decision_readiness_label(),
            "triage_required"
        );
        assert!(triage_summary.host_decision_readiness_requires_manual_review());
        assert!(!triage_summary.host_decision_readiness_requires_investigation());
        assert!(!triage_summary.host_decision_readiness_requires_integrity_investigation());
        assert!(triage_summary.host_decision_needs_triage());

        let mut drift_summary = idle_summary.clone();
        drift_summary.host_decision_readiness =
            ToolAuditSupervisorDrainHostDecisionReadiness::DriftInvestigationReady;
        drift_summary.host_decision_readiness_label = "drift_investigation_ready";
        drift_summary.host_decision_is_settled = false;
        drift_summary.host_decision_is_actionable = true;
        drift_summary.host_decision_is_auto_routable = false;
        drift_summary.host_decision_readiness_requires_manual_review = true;
        drift_summary.host_decision_readiness_requires_investigation = true;
        assert_eq!(
            drift_summary.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::DriftInvestigationReady
        );
        assert_eq!(
            drift_summary.host_decision_readiness_label(),
            "drift_investigation_ready"
        );
        assert!(drift_summary.host_decision_readiness_label_matches_kind());
        assert!(drift_summary.host_decision_is_actionable());
        assert!(!drift_summary.host_decision_is_auto_routable());
        assert!(drift_summary.host_decision_readiness_requires_manual_review());
        assert!(drift_summary.host_decision_readiness_requires_investigation());
        assert!(!drift_summary.host_decision_readiness_requires_integrity_investigation());
        assert!(!drift_summary.host_decision_needs_triage());

        let mut integrity_summary = idle_summary;
        integrity_summary.host_decision_readiness =
            ToolAuditSupervisorDrainHostDecisionReadiness::IntegrityInvestigationReady;
        integrity_summary.host_decision_readiness_label = "integrity_investigation_ready";
        integrity_summary.host_decision_is_settled = false;
        integrity_summary.host_decision_is_actionable = true;
        integrity_summary.host_decision_is_auto_routable = false;
        integrity_summary.host_decision_readiness_requires_manual_review = true;
        integrity_summary.host_decision_readiness_requires_investigation = true;
        integrity_summary.host_decision_readiness_requires_integrity_investigation = true;
        assert_eq!(
            integrity_summary.host_decision_readiness(),
            ToolAuditSupervisorDrainHostDecisionReadiness::IntegrityInvestigationReady
        );
        assert_eq!(
            integrity_summary.host_decision_readiness_label(),
            "integrity_investigation_ready"
        );
        assert!(integrity_summary.host_decision_readiness_requires_manual_review());
        assert!(integrity_summary.host_decision_readiness_requires_investigation());
        assert!(integrity_summary.host_decision_readiness_requires_integrity_investigation());
        assert!(!integrity_summary.host_decision_needs_triage());

        integrity_summary.host_decision_readiness_label = "readyish";
        integrity_summary.host_decision_readiness_label_matches_kind = false;
        assert!(!integrity_summary.host_decision_readiness_label_matches_kind());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_host_decision_route_fields() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert_eq!(
            idle_report.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::NoRoute
        );
        assert_eq!(idle_report.host_decision_route_label(), "no_route");
        assert!(idle_report.host_decision_route_label_matches_kind());
        assert!(!idle_report.host_decision_has_route());
        assert!(!idle_report.host_decision_routes_to_scheduler());
        assert!(!idle_report.host_decision_routes_to_follow_up());
        assert!(!idle_report.host_decision_routes_to_drift_investigation());
        assert!(!idle_report.host_decision_routes_to_integrity_investigation());
        assert!(!idle_report.host_decision_routes_to_triage());
        assert!(!idle_report.host_decision_routes_to_investigation());
        assert_eq!(
            idle_summary.host_decision_route,
            ToolAuditSupervisorDrainHostDecisionRoute::NoRoute
        );
        assert_eq!(
            idle_summary.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::NoRoute
        );
        assert_eq!(idle_summary.host_decision_route_label, "no_route");
        assert_eq!(idle_summary.host_decision_route_label(), "no_route");
        assert!(idle_summary.host_decision_route_label_matches_kind);
        assert!(idle_summary.host_decision_route_label_matches_kind());
        assert!(!idle_summary.host_decision_has_route);
        assert!(!idle_summary.host_decision_has_route());
        assert!(!idle_summary.host_decision_routes_to_scheduler);
        assert!(!idle_summary.host_decision_routes_to_scheduler());
        assert!(!idle_summary.host_decision_routes_to_follow_up);
        assert!(!idle_summary.host_decision_routes_to_follow_up());
        assert!(!idle_summary.host_decision_routes_to_drift_investigation);
        assert!(!idle_summary.host_decision_routes_to_drift_investigation());
        assert!(!idle_summary.host_decision_routes_to_integrity_investigation);
        assert!(!idle_summary.host_decision_routes_to_integrity_investigation());
        assert!(!idle_summary.host_decision_routes_to_triage);
        assert!(!idle_summary.host_decision_routes_to_triage());
        assert!(!idle_summary.host_decision_routes_to_investigation);
        assert!(!idle_summary.host_decision_routes_to_investigation());

        let continuation_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(continuation_store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());
        let mut continuation_sink = InMemoryToolAuditSink::new();
        let continuation_report = continuation_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut continuation_sink)
            .unwrap();
        let continuation_summary = continuation_report.summary();

        assert_eq!(
            continuation_report.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::Scheduler
        );
        assert_eq!(continuation_report.host_decision_route_label(), "scheduler");
        assert!(continuation_report.host_decision_route_label_matches_kind());
        assert!(continuation_report.host_decision_has_route());
        assert!(continuation_report.host_decision_routes_to_scheduler());
        assert!(!continuation_report.host_decision_routes_to_follow_up());
        assert!(!continuation_report.host_decision_routes_to_investigation());
        assert_eq!(
            continuation_summary.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::Scheduler
        );
        assert_eq!(
            continuation_summary.host_decision_route_label(),
            "scheduler"
        );
        assert!(continuation_summary.host_decision_has_route());
        assert!(continuation_summary.host_decision_routes_to_scheduler());
        assert!(!continuation_summary.host_decision_routes_to_follow_up());
        assert!(!continuation_summary.host_decision_routes_to_investigation());

        let follow_up_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(follow_up_store
            .record_audit_batch(vec![failed_record("call_1")])
            .completed_without_failures());
        let mut follow_up_sink = InMemoryToolAuditSink::new();
        let follow_up_report = follow_up_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut follow_up_sink)
            .unwrap();
        let follow_up_summary = follow_up_report.summary();

        assert_eq!(
            follow_up_report.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::FollowUp
        );
        assert_eq!(follow_up_report.host_decision_route_label(), "follow_up");
        assert!(follow_up_report.host_decision_has_route());
        assert!(!follow_up_report.host_decision_routes_to_scheduler());
        assert!(follow_up_report.host_decision_routes_to_follow_up());
        assert!(!follow_up_report.host_decision_routes_to_investigation());
        assert_eq!(
            follow_up_summary.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::FollowUp
        );
        assert!(follow_up_summary.host_decision_routes_to_follow_up());

        let mut triage_report = continuation_report;
        triage_report.drain.ticks[0].replay.next_checkpoint = ToolAuditReadCheckpoint::beginning();
        let triage_summary = triage_report.summary();
        assert_eq!(
            triage_report.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::Triage
        );
        assert_eq!(triage_report.host_decision_route_label(), "triage");
        assert!(triage_report.host_decision_has_route());
        assert!(triage_report.host_decision_routes_to_triage());
        assert!(!triage_report.host_decision_routes_to_investigation());
        assert_eq!(
            triage_summary.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::Triage
        );
        assert!(triage_summary.host_decision_routes_to_triage());
        assert!(!triage_summary.host_decision_routes_to_investigation());

        let mut drift_summary = idle_summary.clone();
        drift_summary.host_decision_route =
            ToolAuditSupervisorDrainHostDecisionRoute::DriftInvestigation;
        drift_summary.host_decision_route_label = "drift_investigation";
        drift_summary.host_decision_has_route = true;
        drift_summary.host_decision_routes_to_drift_investigation = true;
        drift_summary.host_decision_routes_to_investigation = true;
        assert_eq!(
            drift_summary.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::DriftInvestigation
        );
        assert_eq!(
            drift_summary.host_decision_route_label(),
            "drift_investigation"
        );
        assert!(drift_summary.host_decision_route_label_matches_kind());
        assert!(drift_summary.host_decision_has_route());
        assert!(drift_summary.host_decision_routes_to_drift_investigation());
        assert!(!drift_summary.host_decision_routes_to_integrity_investigation());
        assert!(drift_summary.host_decision_routes_to_investigation());

        let mut integrity_summary = idle_summary;
        integrity_summary.host_decision_route =
            ToolAuditSupervisorDrainHostDecisionRoute::IntegrityInvestigation;
        integrity_summary.host_decision_route_label = "integrity_investigation";
        integrity_summary.host_decision_has_route = true;
        integrity_summary.host_decision_routes_to_integrity_investigation = true;
        integrity_summary.host_decision_routes_to_investigation = true;
        assert_eq!(
            integrity_summary.host_decision_route(),
            ToolAuditSupervisorDrainHostDecisionRoute::IntegrityInvestigation
        );
        assert_eq!(
            integrity_summary.host_decision_route_label(),
            "integrity_investigation"
        );
        assert!(integrity_summary.host_decision_routes_to_integrity_investigation());
        assert!(integrity_summary.host_decision_routes_to_investigation());

        integrity_summary.host_decision_route_label = "elsewhere";
        integrity_summary.host_decision_route_label_matches_kind = false;
        assert!(!integrity_summary.host_decision_route_label_matches_kind());
    }

    #[test]
    fn supervisor_drain_summary_flattens_host_routing_label_integrity_flags() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();
        let clean_summary = report.summary();

        assert!(report.host_decision_classifier_labels_match());
        assert!(!report.has_host_decision_label_integrity_drift());
        assert!(report.host_routing_classifier_labels_match());
        assert!(!report.has_host_routing_label_integrity_drift());
        assert!(clean_summary.host_decision_classifier_labels_match);
        assert!(clean_summary.host_decision_classifier_labels_match());
        assert!(!clean_summary.has_host_decision_label_integrity_drift);
        assert!(!clean_summary.has_host_decision_label_integrity_drift());
        assert!(clean_summary.host_routing_classifier_labels_match);
        assert!(clean_summary.host_routing_classifier_labels_match());
        assert!(!clean_summary.has_host_routing_label_integrity_drift);
        assert!(!clean_summary.has_host_routing_label_integrity_drift());

        let mut stale_decision_summary = clean_summary.clone();
        stale_decision_summary.host_decision_route_label = "elsewhere";
        stale_decision_summary.host_decision_route_label_matches_kind = false;
        stale_decision_summary.host_decision_classifier_labels_match = false;
        stale_decision_summary.has_host_decision_label_integrity_drift = true;
        stale_decision_summary.host_routing_classifier_labels_match = false;
        stale_decision_summary.has_host_routing_label_integrity_drift = true;
        assert!(!stale_decision_summary.host_decision_route_label_matches_kind());
        assert!(!stale_decision_summary.host_decision_classifier_labels_match());
        assert!(stale_decision_summary.has_host_decision_label_integrity_drift());
        assert!(!stale_decision_summary.host_routing_classifier_labels_match());
        assert!(stale_decision_summary.has_host_routing_label_integrity_drift());

        let mut stale_attention_summary = clean_summary;
        stale_attention_summary.host_attention_label = "attentionish";
        stale_attention_summary.host_attention_label_matches_kind = false;
        stale_attention_summary.host_routing_classifier_labels_match = false;
        stale_attention_summary.has_host_routing_label_integrity_drift = true;
        assert!(stale_attention_summary.host_decision_classifier_labels_match());
        assert!(!stale_attention_summary.has_host_decision_label_integrity_drift());
        assert!(!stale_attention_summary.host_attention_label_matches_kind());
        assert!(!stale_attention_summary.host_routing_classifier_labels_match());
        assert!(stale_attention_summary.has_host_routing_label_integrity_drift());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_checkpoint_consistency_flags() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert_eq!(
            idle_report.last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::beginning())
        );
        assert!(idle_report.has_last_checkpoint());
        assert_eq!(idle_report.last_checkpoint_timestamp_ms(), Some(0));
        assert_eq!(idle_report.last_checkpoint_call_id(), Some(""));
        assert!(idle_report.last_checkpoint_presence_matches_fields());
        assert!(idle_report.last_checkpoint_timestamp_matches_checkpoint());
        assert!(idle_report.last_checkpoint_call_id_matches_checkpoint());
        assert!(idle_report.last_checkpoint_fields_match_checkpoint());
        assert_eq!(
            idle_summary.last_checkpoint,
            Some(ToolAuditReadCheckpoint::beginning())
        );
        assert!(idle_summary.has_last_checkpoint);
        assert_eq!(idle_summary.last_checkpoint_timestamp_ms, Some(0));
        assert_eq!(idle_summary.last_checkpoint_call_id.as_deref(), Some(""));
        assert!(idle_summary.last_checkpoint_presence_matches_fields);
        assert!(idle_summary.last_checkpoint_presence_matches_fields());
        assert!(idle_summary.last_checkpoint_timestamp_matches_checkpoint);
        assert!(idle_summary.last_checkpoint_timestamp_matches_checkpoint());
        assert!(idle_summary.last_checkpoint_call_id_matches_checkpoint);
        assert!(idle_summary.last_checkpoint_call_id_matches_checkpoint());
        assert!(idle_summary.last_checkpoint_fields_match_checkpoint);
        assert!(idle_summary.last_checkpoint_fields_match_checkpoint());

        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_1")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let mut summary = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap()
            .summary();
        assert!(summary.last_checkpoint_fields_match_checkpoint());

        summary.last_checkpoint_timestamp_ms = Some(999);
        summary.last_checkpoint_timestamp_matches_checkpoint = false;
        summary.last_checkpoint_fields_match_checkpoint = false;
        assert!(!summary.last_checkpoint_timestamp_matches_checkpoint());
        assert!(!summary.last_checkpoint_fields_match_checkpoint());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_checkpoint_advance_flags() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert!(!idle_report.last_checkpoint_advanced_from_starting());
        assert!(idle_report.last_checkpoint_matches_starting_checkpoint());
        assert!(idle_report.checkpoint_advance_matches_last_checkpoint());
        assert!(idle_report.checkpoint_boundary_fields_match());
        assert!(!idle_summary.last_checkpoint_advanced_from_starting);
        assert!(!idle_summary.last_checkpoint_advanced_from_starting());
        assert!(idle_summary.last_checkpoint_matches_starting_checkpoint);
        assert!(idle_summary.last_checkpoint_matches_starting_checkpoint());
        assert!(idle_summary.checkpoint_advance_matches_last_checkpoint);
        assert!(idle_summary.checkpoint_advance_matches_last_checkpoint());
        assert!(idle_summary.checkpoint_boundary_fields_match);
        assert!(idle_summary.checkpoint_boundary_fields_match());

        let stored_idle_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(stored_idle_store
            .record_audit_batch(vec![sample_record("call_1")])
            .completed_without_failures());
        stored_idle_store
            .save_checkpoint("supervisor", ToolAuditReadCheckpoint::new(120, "call_1"))
            .unwrap();

        let mut stored_idle_sink = InMemoryToolAuditSink::new();
        let stored_idle_report = stored_idle_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut stored_idle_sink)
            .unwrap();
        assert_eq!(
            stored_idle_report.starting_checkpoint(),
            &ToolAuditReadCheckpoint::new(120, "call_1")
        );
        assert!(!stored_idle_report.advanced_checkpoint());
        assert!(!stored_idle_report.last_checkpoint_advanced_from_starting());
        assert!(stored_idle_report.last_checkpoint_matches_starting_checkpoint());
        assert!(stored_idle_report.checkpoint_advance_matches_last_checkpoint());
        assert!(stored_idle_report.checkpoint_boundary_fields_match());

        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();

        assert!(report.advanced_checkpoint());
        assert!(report.last_checkpoint_advanced_from_starting());
        assert!(!report.last_checkpoint_matches_starting_checkpoint());
        assert!(report.checkpoint_advance_matches_last_checkpoint());
        assert!(report.checkpoint_boundary_fields_match());

        let summary = report.summary();
        assert!(summary.last_checkpoint_advanced_from_starting);
        assert!(summary.last_checkpoint_advanced_from_starting());
        assert!(!summary.last_checkpoint_matches_starting_checkpoint);
        assert!(!summary.last_checkpoint_matches_starting_checkpoint());
        assert!(summary.checkpoint_advance_matches_last_checkpoint);
        assert!(summary.checkpoint_advance_matches_last_checkpoint());
        assert!(summary.checkpoint_boundary_fields_match);
        assert!(summary.checkpoint_boundary_fields_match());

        let mut stale_summary = summary.clone();
        stale_summary.last_checkpoint_advanced_from_starting = false;
        stale_summary.checkpoint_advance_matches_last_checkpoint = false;
        stale_summary.checkpoint_boundary_fields_match = false;
        assert!(!stale_summary.last_checkpoint_advanced_from_starting());
        assert!(!stale_summary.checkpoint_advance_matches_last_checkpoint());
        assert!(!stale_summary.checkpoint_boundary_fields_match());

        let mut stale_report = report;
        stale_report.drain.ticks[0].replay.next_checkpoint = ToolAuditReadCheckpoint::beginning();
        assert!(stale_report.advanced_checkpoint());
        assert!(!stale_report.last_checkpoint_advanced_from_starting());
        assert!(!stale_report.checkpoint_advance_matches_last_checkpoint());
        assert!(!stale_report.checkpoint_boundary_fields_match());
    }

    #[test]
    fn supervisor_drain_report_summary_flattens_planned_checkpoint_fields() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert_eq!(
            idle_report.planned_last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::beginning())
        );
        assert!(idle_report.has_planned_last_checkpoint());
        assert_eq!(idle_report.planned_last_checkpoint_timestamp_ms(), Some(0));
        assert_eq!(idle_report.planned_last_checkpoint_call_id(), Some(""));
        assert!(idle_report.planned_last_checkpoint_presence_matches_fields());
        assert!(idle_report.planned_last_checkpoint_timestamp_matches_checkpoint());
        assert!(idle_report.planned_last_checkpoint_call_id_matches_checkpoint());
        assert!(idle_report.planned_last_checkpoint_fields_match_checkpoint());
        assert!(!idle_report.planned_last_checkpoint_advanced_from_starting());
        assert!(idle_report.planned_last_checkpoint_matches_starting_checkpoint());
        assert!(idle_report.planned_last_checkpoint_matches_last_checkpoint());
        assert!(idle_report.planned_checkpoint_advance_matches_drain());
        assert!(idle_report.checkpoint_plan_matches_drain());
        assert!(!idle_report.has_planned_last_checkpoint_drift());
        assert!(!idle_report.has_checkpoint_advance_drift());
        assert!(!idle_report.has_checkpoint_plan_drift());
        assert_eq!(
            idle_report.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(idle_report.checkpoint_drift_label(), "no_checkpoint_drift");
        assert!(idle_report.checkpoint_drift_label_matches_kind());
        assert!(idle_report.is_no_checkpoint_drift());
        assert!(!idle_report.requires_checkpoint_drift_investigation());
        assert_eq!(
            idle_summary.planned_last_checkpoint,
            Some(ToolAuditReadCheckpoint::beginning())
        );
        assert!(idle_summary.has_planned_last_checkpoint);
        assert_eq!(idle_summary.planned_last_checkpoint_timestamp_ms, Some(0));
        assert_eq!(
            idle_summary.planned_last_checkpoint_call_id.as_deref(),
            Some("")
        );
        assert!(idle_summary.planned_last_checkpoint_presence_matches_fields);
        assert!(idle_summary.planned_last_checkpoint_presence_matches_fields());
        assert!(idle_summary.planned_last_checkpoint_timestamp_matches_checkpoint);
        assert!(idle_summary.planned_last_checkpoint_timestamp_matches_checkpoint());
        assert!(idle_summary.planned_last_checkpoint_call_id_matches_checkpoint);
        assert!(idle_summary.planned_last_checkpoint_call_id_matches_checkpoint());
        assert!(idle_summary.planned_last_checkpoint_fields_match_checkpoint);
        assert!(idle_summary.planned_last_checkpoint_fields_match_checkpoint());
        assert!(!idle_summary.planned_last_checkpoint_advanced_from_starting);
        assert!(!idle_summary.planned_last_checkpoint_advanced_from_starting());
        assert!(idle_summary.planned_last_checkpoint_matches_starting_checkpoint);
        assert!(idle_summary.planned_last_checkpoint_matches_starting_checkpoint());
        assert!(idle_summary.planned_last_checkpoint_matches_last_checkpoint);
        assert!(idle_summary.planned_last_checkpoint_matches_last_checkpoint());
        assert!(idle_summary.planned_checkpoint_advance_matches_drain);
        assert!(idle_summary.planned_checkpoint_advance_matches_drain());
        assert!(idle_summary.checkpoint_plan_matches_drain);
        assert!(idle_summary.checkpoint_plan_matches_drain());
        assert!(!idle_summary.has_planned_last_checkpoint_drift);
        assert!(!idle_summary.has_planned_last_checkpoint_drift());
        assert!(!idle_summary.has_checkpoint_advance_drift);
        assert!(!idle_summary.has_checkpoint_advance_drift());
        assert!(!idle_summary.has_checkpoint_plan_drift);
        assert!(!idle_summary.has_checkpoint_plan_drift());
        assert_eq!(
            idle_summary.checkpoint_drift_kind,
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(
            idle_summary.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(idle_summary.checkpoint_drift_label, "no_checkpoint_drift");
        assert_eq!(idle_summary.checkpoint_drift_label(), "no_checkpoint_drift");
        assert!(idle_summary.checkpoint_drift_label_matches_kind);
        assert!(idle_summary.checkpoint_drift_label_matches_kind());
        assert!(idle_summary.is_no_checkpoint_drift);
        assert!(idle_summary.is_no_checkpoint_drift());
        assert!(!idle_summary.requires_checkpoint_drift_investigation);
        assert!(!idle_summary.requires_checkpoint_drift_investigation());

        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![
                sample_record("call_1"),
                sample_record("call_2"),
                sample_record("call_3"),
            ])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 2, 1, &mut sink)
            .unwrap();

        assert_eq!(
            report.planned_last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(120, "call_2"))
        );
        assert!(report.has_planned_last_checkpoint());
        assert_eq!(report.planned_last_checkpoint_timestamp_ms(), Some(120));
        assert_eq!(report.planned_last_checkpoint_call_id(), Some("call_2"));
        assert!(report.planned_last_checkpoint_presence_matches_fields());
        assert!(report.planned_last_checkpoint_timestamp_matches_checkpoint());
        assert!(report.planned_last_checkpoint_call_id_matches_checkpoint());
        assert!(report.planned_last_checkpoint_fields_match_checkpoint());
        assert!(report.planned_last_checkpoint_advanced_from_starting());
        assert!(!report.planned_last_checkpoint_matches_starting_checkpoint());
        assert!(report.planned_last_checkpoint_matches_last_checkpoint());
        assert!(report.planned_checkpoint_advance_matches_drain());
        assert!(report.checkpoint_plan_matches_drain());
        assert!(report.checkpoint_boundary_fields_match());
        assert!(!report.has_planned_last_checkpoint_drift());
        assert!(!report.has_checkpoint_advance_drift());
        assert!(!report.has_checkpoint_plan_drift());
        assert_eq!(
            report.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(report.checkpoint_drift_label(), "no_checkpoint_drift");
        assert!(report.checkpoint_drift_label_matches_kind());
        assert!(report.is_no_checkpoint_drift());
        assert!(!report.requires_checkpoint_drift_investigation());

        let summary = report.summary();
        assert_eq!(
            summary.planned_last_checkpoint,
            Some(ToolAuditReadCheckpoint::new(120, "call_2"))
        );
        assert_eq!(
            summary.planned_last_checkpoint(),
            Some(&ToolAuditReadCheckpoint::new(120, "call_2"))
        );
        assert!(summary.has_planned_last_checkpoint);
        assert!(summary.has_planned_last_checkpoint());
        assert_eq!(summary.planned_last_checkpoint_timestamp_ms, Some(120));
        assert_eq!(summary.planned_last_checkpoint_timestamp_ms(), Some(120));
        assert_eq!(
            summary.planned_last_checkpoint_call_id.as_deref(),
            Some("call_2")
        );
        assert_eq!(summary.planned_last_checkpoint_call_id(), Some("call_2"));
        assert!(summary.planned_last_checkpoint_presence_matches_fields);
        assert!(summary.planned_last_checkpoint_presence_matches_fields());
        assert!(summary.planned_last_checkpoint_timestamp_matches_checkpoint);
        assert!(summary.planned_last_checkpoint_timestamp_matches_checkpoint());
        assert!(summary.planned_last_checkpoint_call_id_matches_checkpoint);
        assert!(summary.planned_last_checkpoint_call_id_matches_checkpoint());
        assert!(summary.planned_last_checkpoint_fields_match_checkpoint);
        assert!(summary.planned_last_checkpoint_fields_match_checkpoint());
        assert!(summary.planned_last_checkpoint_advanced_from_starting);
        assert!(summary.planned_last_checkpoint_advanced_from_starting());
        assert!(!summary.planned_last_checkpoint_matches_starting_checkpoint);
        assert!(!summary.planned_last_checkpoint_matches_starting_checkpoint());
        assert!(summary.planned_last_checkpoint_matches_last_checkpoint);
        assert!(summary.planned_last_checkpoint_matches_last_checkpoint());
        assert!(summary.planned_checkpoint_advance_matches_drain);
        assert!(summary.planned_checkpoint_advance_matches_drain());
        assert!(summary.checkpoint_plan_matches_drain);
        assert!(summary.checkpoint_plan_matches_drain());
        assert!(!summary.has_planned_last_checkpoint_drift);
        assert!(!summary.has_planned_last_checkpoint_drift());
        assert!(!summary.has_checkpoint_advance_drift);
        assert!(!summary.has_checkpoint_advance_drift());
        assert!(!summary.has_checkpoint_plan_drift);
        assert!(!summary.has_checkpoint_plan_drift());
        assert_eq!(
            summary.checkpoint_drift_kind,
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(
            summary.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::NoDrift
        );
        assert_eq!(summary.checkpoint_drift_label, "no_checkpoint_drift");
        assert_eq!(summary.checkpoint_drift_label(), "no_checkpoint_drift");
        assert!(summary.checkpoint_drift_label_matches_kind);
        assert!(summary.checkpoint_drift_label_matches_kind());
        assert!(summary.is_no_checkpoint_drift);
        assert!(summary.is_no_checkpoint_drift());
        assert!(!summary.requires_checkpoint_drift_investigation);
        assert!(!summary.requires_checkpoint_drift_investigation());

        let mut stale_summary = summary.clone();
        stale_summary.planned_last_checkpoint_call_id = Some("stale".to_string());
        stale_summary.planned_last_checkpoint_call_id_matches_checkpoint = false;
        stale_summary.planned_last_checkpoint_fields_match_checkpoint = false;
        stale_summary.planned_last_checkpoint_matches_last_checkpoint = false;
        stale_summary.has_planned_last_checkpoint_drift = true;
        stale_summary.has_checkpoint_plan_drift = true;
        stale_summary.checkpoint_drift_kind =
            ToolAuditSupervisorDrainCheckpointDriftKind::PlannedLastCheckpointDrift;
        stale_summary.checkpoint_drift_label = "planned_last_checkpoint_drift";
        stale_summary.is_no_checkpoint_drift = false;
        stale_summary.requires_checkpoint_drift_investigation = true;
        stale_summary.checkpoint_plan_matches_drain = false;
        stale_summary.checkpoint_boundary_fields_match = false;
        assert!(!stale_summary.planned_last_checkpoint_call_id_matches_checkpoint());
        assert!(!stale_summary.planned_last_checkpoint_fields_match_checkpoint());
        assert!(!stale_summary.planned_last_checkpoint_matches_last_checkpoint());
        assert!(stale_summary.has_planned_last_checkpoint_drift());
        assert!(!stale_summary.has_checkpoint_advance_drift());
        assert!(stale_summary.has_checkpoint_plan_drift());
        assert_eq!(
            stale_summary.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::PlannedLastCheckpointDrift
        );
        assert_eq!(
            stale_summary.checkpoint_drift_label(),
            "planned_last_checkpoint_drift"
        );
        assert!(stale_summary.checkpoint_drift_label_matches_kind());
        assert!(!stale_summary.is_no_checkpoint_drift());
        assert!(stale_summary.requires_checkpoint_drift_investigation());
        assert!(!stale_summary.checkpoint_plan_matches_drain());
        assert!(!stale_summary.checkpoint_boundary_fields_match());

        let mut stale_checkpoint_report = report.clone();
        stale_checkpoint_report.drain.ticks[0]
            .replay
            .next_checkpoint = ToolAuditReadCheckpoint::beginning();
        assert!(!stale_checkpoint_report.planned_last_checkpoint_matches_last_checkpoint());
        assert!(stale_checkpoint_report.planned_checkpoint_advance_matches_drain());
        assert!(!stale_checkpoint_report.checkpoint_plan_matches_drain());
        assert!(!stale_checkpoint_report.checkpoint_boundary_fields_match());
        assert!(stale_checkpoint_report.has_planned_last_checkpoint_drift());
        assert!(!stale_checkpoint_report.has_checkpoint_advance_drift());
        assert!(stale_checkpoint_report.has_checkpoint_plan_drift());
        assert_eq!(
            stale_checkpoint_report.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::PlannedLastCheckpointDrift
        );
        assert_eq!(
            stale_checkpoint_report.checkpoint_drift_label(),
            "planned_last_checkpoint_drift"
        );
        assert!(stale_checkpoint_report.checkpoint_drift_label_matches_kind());
        assert!(!stale_checkpoint_report.is_no_checkpoint_drift());
        assert!(stale_checkpoint_report.requires_checkpoint_drift_investigation());

        let mut stale_advance_report = report.clone();
        stale_advance_report.plan.planned_records = 0;
        assert!(stale_advance_report.planned_last_checkpoint_matches_last_checkpoint());
        assert!(!stale_advance_report.planned_checkpoint_advance_matches_drain());
        assert!(!stale_advance_report.checkpoint_plan_matches_drain());
        assert!(!stale_advance_report.checkpoint_boundary_fields_match());
        assert!(!stale_advance_report.has_planned_last_checkpoint_drift());
        assert!(stale_advance_report.has_checkpoint_advance_drift());
        assert!(stale_advance_report.has_checkpoint_plan_drift());
        assert_eq!(
            stale_advance_report.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::CheckpointAdvanceDrift
        );
        assert_eq!(
            stale_advance_report.checkpoint_drift_label(),
            "checkpoint_advance_drift"
        );
        assert!(stale_advance_report.checkpoint_drift_label_matches_kind());
        assert!(!stale_advance_report.is_no_checkpoint_drift());
        assert!(stale_advance_report.requires_checkpoint_drift_investigation());

        let mut combined_drift_report = report;
        combined_drift_report.drain.ticks[0].replay.next_checkpoint =
            ToolAuditReadCheckpoint::beginning();
        combined_drift_report.plan.planned_records = 0;
        assert!(!combined_drift_report.planned_last_checkpoint_matches_last_checkpoint());
        assert!(!combined_drift_report.planned_checkpoint_advance_matches_drain());
        assert!(!combined_drift_report.checkpoint_plan_matches_drain());
        assert!(!combined_drift_report.checkpoint_boundary_fields_match());
        assert!(combined_drift_report.has_planned_last_checkpoint_drift());
        assert!(combined_drift_report.has_checkpoint_advance_drift());
        assert!(combined_drift_report.has_checkpoint_plan_drift());
        assert_eq!(
            combined_drift_report.checkpoint_drift_kind(),
            ToolAuditSupervisorDrainCheckpointDriftKind::PlannedLastCheckpointAndAdvanceDrift
        );
        assert_eq!(
            combined_drift_report.checkpoint_drift_label(),
            "planned_last_checkpoint_and_advance_drift"
        );
        assert!(combined_drift_report.checkpoint_drift_label_matches_kind());
        assert!(!combined_drift_report.is_no_checkpoint_drift());
        assert!(combined_drift_report.requires_checkpoint_drift_investigation());
    }

    #[test]
    fn supervisor_drain_report_summary_distinguishes_idle_outcome_from_idle_drain() {
        let empty_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut idle_sink = InMemoryToolAuditSink::new();
        let idle_report = empty_store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut idle_sink)
            .unwrap();
        let idle_summary = idle_report.summary();

        assert_eq!(
            idle_report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::Idle
        );
        assert!(idle_report.outcome_is_idle());
        assert!(idle_summary.outcome_is_idle);
        assert!(idle_summary.outcome_is_idle());
        assert!(idle_summary.is_idle);
        assert!(idle_summary.is_idle());

        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let mut report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();
        report.drain.drained_records = 0;
        let summary = report.summary();

        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::PlanDiverged
        );
        assert!(!report.outcome_is_idle());
        assert!(!summary.outcome_is_idle);
        assert!(!summary.outcome_is_idle());
        assert!(summary.is_idle);
        assert!(summary.is_idle());
        assert!(summary.plan_diverged);
        assert!(summary.plan_diverged());
    }

    #[test]
    fn supervisor_drain_report_summary_preserves_plan_drift_action() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let mut report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();
        report.drain.drained_records += 1;
        let summary = report.summary();

        assert_eq!(
            summary.outcome,
            ToolAuditSupervisorDrainRunOutcome::PlanDiverged
        );
        assert_eq!(summary.outcome_label, "plan_diverged");
        assert_eq!(summary.outcome_label(), "plan_diverged");
        assert!(report.plan_diverged());
        assert!(!report.is_caught_up());
        assert!(!report.needs_continuation());
        assert!(!report.needs_follow_up());
        assert!(summary.plan_diverged);
        assert!(summary.plan_diverged());
        assert!(!summary.is_caught_up);
        assert!(!summary.is_caught_up());
        assert!(!summary.needs_continuation);
        assert!(!summary.needs_continuation());
        assert!(!summary.needs_follow_up);
        assert!(!summary.needs_follow_up());
        assert_eq!(
            summary.scheduler_action,
            ToolAuditSupervisorDrainSchedulerAction::InvestigatePlanDrift
        );
        assert_eq!(summary.scheduler_action_label, "investigate_plan_drift");
        assert_eq!(summary.scheduler_action_label(), "investigate_plan_drift");
        assert!(!summary.matches_planned_record_count);
        assert_eq!(report.record_count_delta(), 1);
        assert!(report.has_record_count_drift());
        assert!(!report.has_follow_up_record_count_drift());
        assert!(report.has_count_drift());
        assert!(!report.is_no_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert_eq!(report.count_drift_label(), "record_count_drift");
        assert!(report.count_drift_label_matches_kind());
        assert!(report.requires_count_drift_investigation());
        assert!(report.requires_plan_drift_investigation());
        assert_eq!(
            report.host_investigation_kind(),
            ToolAuditSupervisorDrainHostInvestigationKind::PlanAndCountDrift
        );
        assert_eq!(report.host_investigation_label(), "plan_and_count_drift");
        assert!(report.host_investigation_label_matches_kind());
        assert!(report.requires_host_investigation());
        assert!(report.requires_host_plan_drift_investigation());
        assert!(report.requires_host_count_drift_investigation());
        assert!(!report.is_no_host_investigation());
        assert!(report.replayed_extra_records());
        assert!(!report.missed_planned_records());
        assert_eq!(summary.record_count_delta, 1);
        assert!(summary.replayed_extra_records);
        assert!(summary.replayed_extra_records());
        assert!(!summary.missed_planned_records);
        assert!(!summary.missed_planned_records());
        assert!(summary.has_record_count_drift);
        assert!(summary.has_record_count_drift());
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift());
        assert!(summary.has_count_drift);
        assert!(summary.has_count_drift());
        assert!(!summary.is_no_count_drift);
        assert!(!summary.is_no_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert_eq!(summary.count_drift_label, "record_count_drift");
        assert_eq!(summary.count_drift_label(), "record_count_drift");
        assert!(summary.count_drift_label_matches_kind);
        assert!(summary.count_drift_label_matches_kind());
        assert!(summary.requires_count_drift_investigation);
        assert!(summary.requires_count_drift_investigation());
        assert_eq!(
            summary.host_investigation_kind,
            ToolAuditSupervisorDrainHostInvestigationKind::PlanAndCountDrift
        );
        assert_eq!(summary.host_investigation_label, "plan_and_count_drift");
        assert_eq!(summary.host_investigation_label(), "plan_and_count_drift");
        assert!(summary.host_investigation_label_matches_kind);
        assert!(summary.host_investigation_label_matches_kind());
        assert!(summary.requires_host_investigation);
        assert!(summary.requires_host_investigation());
        assert!(summary.requires_host_plan_drift_investigation);
        assert!(summary.requires_host_plan_drift_investigation());
        assert!(summary.requires_host_count_drift_investigation);
        assert!(summary.requires_host_count_drift_investigation());
        assert!(!summary.is_no_host_investigation);
        assert!(!summary.is_no_host_investigation());
        assert!(summary.replayed_extra_records());
        assert!(!summary.missed_planned_records());
        assert!(!summary.is_no_scheduler_action);
        assert!(!summary.is_no_scheduler_action());
        assert!(summary.requires_scheduler_action);
        assert!(summary.requires_scheduler_action());
        assert!(!summary.requests_continuation);
        assert!(!summary.requests_continuation());
        assert!(!summary.routes_follow_up);
        assert!(!summary.routes_follow_up());
        assert!(summary.requires_plan_drift_investigation);
        assert!(summary.requires_plan_drift_investigation());
    }

    #[test]
    fn supervisor_drain_report_summary_detects_missed_record_delta() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![sample_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let mut report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();
        report.drain.drained_records -= 1;
        let summary = report.summary();

        assert!(!report.matches_planned_record_count());
        assert!(!report.matches_record_count());
        assert_eq!(report.record_count_delta(), -1);
        assert!(report.has_record_count_drift());
        assert!(!report.has_follow_up_record_count_drift());
        assert!(report.has_count_drift());
        assert!(!report.is_no_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert!(report.requires_count_drift_investigation());
        assert!(!report.replayed_extra_records());
        assert!(report.missed_planned_records());
        assert_eq!(
            report.host_investigation_kind(),
            ToolAuditSupervisorDrainHostInvestigationKind::PlanAndCountDrift
        );
        assert_eq!(report.host_investigation_label(), "plan_and_count_drift");
        assert!(report.requires_host_investigation());
        assert!(report.requires_host_plan_drift_investigation());
        assert!(report.requires_host_count_drift_investigation());
        assert!(!report.is_no_host_investigation());
        assert_eq!(summary.record_count_delta, -1);
        assert!(!summary.replayed_extra_records);
        assert!(!summary.replayed_extra_records());
        assert!(summary.missed_planned_records);
        assert!(summary.missed_planned_records());
        assert!(!summary.matches_record_count);
        assert!(!summary.matches_record_count());
        assert!(summary.has_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(summary.has_count_drift);
        assert!(summary.has_count_drift());
        assert!(!summary.is_no_count_drift);
        assert!(!summary.is_no_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert_eq!(summary.count_drift_label, "record_count_drift");
        assert!(summary.requires_count_drift_investigation);
        assert!(summary.requires_count_drift_investigation());
        assert_eq!(
            summary.host_investigation_kind,
            ToolAuditSupervisorDrainHostInvestigationKind::PlanAndCountDrift
        );
        assert_eq!(summary.host_investigation_label, "plan_and_count_drift");
        assert_eq!(summary.host_investigation_label(), "plan_and_count_drift");
        assert!(summary.requires_host_investigation);
        assert!(summary.requires_host_investigation());
        assert!(summary.requires_host_plan_drift_investigation);
        assert!(summary.requires_host_plan_drift_investigation());
        assert!(summary.requires_host_count_drift_investigation);
        assert!(summary.requires_host_count_drift_investigation());
        assert!(!summary.is_no_host_investigation);
        assert!(!summary.is_no_host_investigation());
        assert!(!summary.replayed_extra_records());
        assert!(summary.missed_planned_records());
        assert_eq!(
            summary.outcome,
            ToolAuditSupervisorDrainRunOutcome::PlanDiverged
        );
    }

    #[test]
    fn supervisor_drain_report_summary_detects_follow_up_count_drift() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![failed_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let mut report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();
        report.drain.ticks[0].replay.inventory.follow_up_records = 0;
        let summary = report.summary();

        assert!(report.matches_planned_record_count());
        assert!(!report.matches_planned_follow_up_record_count());
        assert_eq!(summary.planned_follow_up_records, 1);
        assert_eq!(summary.drained_follow_up_records, 0);
        assert_eq!(report.record_count_delta(), 0);
        assert_eq!(report.follow_up_record_count_delta(), -1);
        assert!(!report.replayed_extra_follow_up_records());
        assert!(report.missed_planned_follow_up_records());
        assert!(!report.has_record_count_drift());
        assert!(report.has_follow_up_record_count_drift());
        assert!(report.has_count_drift());
        assert!(!report.is_no_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::FollowUpRecordCountDrift
        );
        assert_eq!(report.count_drift_label(), "follow_up_record_count_drift");
        assert!(report.requires_count_drift_investigation());
        assert!(!report.requires_plan_drift_investigation());
        assert_eq!(
            report.host_investigation_kind(),
            ToolAuditSupervisorDrainHostInvestigationKind::CountDrift
        );
        assert_eq!(report.host_investigation_label(), "count_drift");
        assert!(report.requires_host_investigation());
        assert!(!report.requires_host_plan_drift_investigation());
        assert!(report.requires_host_count_drift_investigation());
        assert!(!report.is_no_host_investigation());
        assert_eq!(summary.record_count_delta, 0);
        assert!(!summary.replayed_extra_records);
        assert!(!summary.replayed_extra_records());
        assert!(!summary.missed_planned_records);
        assert!(!summary.missed_planned_records());
        assert_eq!(summary.follow_up_record_count_delta, -1);
        assert!(!summary.replayed_extra_follow_up_records);
        assert!(!summary.replayed_extra_follow_up_records());
        assert!(summary.missed_planned_follow_up_records);
        assert!(summary.missed_planned_follow_up_records());
        assert!(!summary.has_record_count_drift);
        assert!(!summary.has_record_count_drift());
        assert!(summary.has_follow_up_record_count_drift);
        assert!(summary.has_follow_up_record_count_drift());
        assert!(summary.has_count_drift);
        assert!(summary.has_count_drift());
        assert!(!summary.is_no_count_drift);
        assert!(!summary.is_no_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::FollowUpRecordCountDrift
        );
        assert_eq!(summary.count_drift_label, "follow_up_record_count_drift");
        assert_eq!(summary.count_drift_label(), "follow_up_record_count_drift");
        assert!(!summary.requires_plan_drift_investigation);
        assert!(!summary.requires_plan_drift_investigation());
        assert!(summary.requires_count_drift_investigation);
        assert!(summary.requires_count_drift_investigation());
        assert_eq!(
            summary.host_investigation_kind,
            ToolAuditSupervisorDrainHostInvestigationKind::CountDrift
        );
        assert_eq!(summary.host_investigation_label, "count_drift");
        assert_eq!(summary.host_investigation_label(), "count_drift");
        assert!(summary.requires_host_investigation);
        assert!(summary.requires_host_investigation());
        assert!(!summary.requires_host_plan_drift_investigation);
        assert!(!summary.requires_host_plan_drift_investigation());
        assert!(summary.requires_host_count_drift_investigation);
        assert!(summary.requires_host_count_drift_investigation());
        assert!(!summary.is_no_host_investigation);
        assert!(!summary.is_no_host_investigation());
        assert!(summary.matches_planned_record_count);
        assert!(summary.matches_planned_record_count());
        assert!(!summary.matches_planned_follow_up_record_count);
        assert!(!summary.matches_planned_follow_up_record_count());
        assert!(!summary.matches_follow_up_pressure);
        assert!(!summary.matches_follow_up_pressure());
        assert!(!summary.replayed_extra_follow_up_records());
        assert!(summary.missed_planned_follow_up_records());
    }

    #[test]
    fn supervisor_drain_report_detects_extra_follow_up_delta() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        assert!(store
            .record_audit_batch(vec![failed_record("call_1"), sample_record("call_2")])
            .completed_without_failures());

        let mut sink = InMemoryToolAuditSink::new();
        let mut report = store
            .drain_supervisor_checkpoint_loop_with_plan("supervisor", 10, 2, &mut sink)
            .unwrap();
        report.drain.ticks[0].replay.inventory.follow_up_records = 2;
        let summary = report.summary();

        assert_eq!(report.follow_up_record_count_delta(), 1);
        assert!(report.replayed_extra_follow_up_records());
        assert!(!report.missed_planned_follow_up_records());
        assert_eq!(summary.follow_up_record_count_delta, 1);
        assert!(summary.replayed_extra_follow_up_records);
        assert!(summary.replayed_extra_follow_up_records());
        assert!(!summary.missed_planned_follow_up_records);
        assert!(!summary.missed_planned_follow_up_records());
    }

    #[test]
    fn supervisor_drain_loop_rejects_zero_limits() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let mut sink = InMemoryToolAuditSink::new();

        let zero_records = store
            .drain_supervisor_checkpoint_loop("supervisor", 0, 1, &mut sink)
            .unwrap_err();
        assert!(zero_records.to_string().contains("max_records_per_tick"));

        let zero_ticks = store
            .drain_supervisor_checkpoint_loop("supervisor", 1, 0, &mut sink)
            .unwrap_err();
        assert!(zero_ticks.to_string().contains("max_ticks"));
        assert!(sink.records().is_empty());
    }

    #[test]
    fn batch_records_successes_and_storage_failures() {
        let store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let summary = store.record_audit_batch(vec![
            sample_record("call_1"),
            failed_record("call_2"),
            sample_record("call_1"),
        ]);

        assert_eq!(summary.attempted_records, 3);
        assert_eq!(summary.stored_records, 2);
        assert_eq!(summary.failed_records, 1);
        assert!(!summary.completed_without_failures());
        assert!(summary.requires_follow_up());
        assert!(summary.has_stored_records());
        assert!(summary.has_storage_failures());
        assert!(summary.stored_count_matches_inventory());
        assert!(summary.failed_count_matches_failures());
        assert!(summary.attempted_count_matches_results());
        assert!(summary.write_counts_match());
        assert!(!summary.has_write_count_integrity_drift());
        assert_eq!(
            summary.inventory_follow_up_kind(),
            ToolAuditInventoryFollowUpKind::MultipleFollowUp
        );
        assert_eq!(summary.inventory_follow_up_label(), "multiple_follow_up");
        assert!(summary.inventory_follow_up_label_matches_kind());
        assert_eq!(
            summary.write_outcome(),
            ToolAuditBatchWriteOutcome::StorageFailures
        );
        assert_eq!(summary.write_outcome_label(), "storage_failures");
        assert!(summary.write_outcome_label_matches_outcome());
        assert!(summary.write_outcome().requires_follow_up());
        assert!(summary.write_outcome().requires_storage_investigation());
        assert_eq!(summary.inventory.total_records, 2);
        assert_eq!(summary.inventory.failed_records, 1);
        assert_eq!(summary.inventory.follow_up_records, 1);
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].call_id, "call_1");

        let mut stale_summary = summary.clone();
        stale_summary.stored_records = 3;
        assert!(!stale_summary.stored_count_matches_inventory());
        assert!(!stale_summary.write_counts_match());
        assert!(stale_summary.has_write_count_integrity_drift());
        assert_eq!(
            stale_summary.write_outcome(),
            ToolAuditBatchWriteOutcome::CountIntegrityDrift
        );
        assert!(stale_summary
            .write_outcome()
            .requires_count_integrity_investigation());

        assert_eq!(
            store
                .query_audits(&ToolAuditRecordQuery::new())
                .unwrap()
                .len(),
            2
        );
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
                follow_up_records: 1,
            }
        );
        assert!(inventory.requires_follow_up());
        assert!(inventory.has_failed_records());
        assert!(inventory.has_approval_follow_up());
        assert!(inventory.has_result_errors());
        assert_eq!(
            inventory.follow_up_kind(),
            ToolAuditInventoryFollowUpKind::MultipleFollowUp
        );
        assert_eq!(inventory.follow_up_label(), "multiple_follow_up");
        assert!(inventory.follow_up_label_matches_kind());
        assert!(inventory.has_multiple_follow_up_surfaces());

        let storage = store.storage_inventory_summary().unwrap();
        assert_eq!(storage.total_records, 2);
        assert_eq!(storage.records_with_metadata, 2);
        assert_eq!(storage.json_records, 2);
    }

    #[test]
    fn inventory_summaries_count_follow_up_rows_once() {
        let clean = sample_record("call_clean");
        let failed = failed_record("call_failed");
        let mut active = sample_record("call_active");
        active.status = ToolCallStatus::Running;
        active.approval_state = ApprovalState::Pending;
        active.result_summary.ok = false;
        active.result_summary.has_error = true;
        active.result_summary.error_kind = Some(ToolErrorKind::ToolApprovalRequired);

        let inventory = ToolAuditStoreInventorySummary::from_records(&[clean, failed, active]);

        assert_eq!(inventory.total_records, 3);
        assert_eq!(inventory.completed_records, 1);
        assert_eq!(inventory.failed_records, 1);
        assert_eq!(inventory.active_records, 1);
        assert_eq!(inventory.approval_pending_records, 1);
        assert_eq!(inventory.approval_denied_records, 1);
        assert_eq!(inventory.records_with_errors, 2);
        assert_eq!(inventory.follow_up_records, 2);
        assert!(inventory.requires_follow_up());
        assert_eq!(
            inventory.follow_up_kind(),
            ToolAuditInventoryFollowUpKind::MultipleFollowUp
        );
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
        assert_eq!(
            summary.replay_outcome(),
            ToolAuditReplayOutcome::ReplayedWithFollowUp
        );
        assert_eq!(summary.replay_outcome_label(), "replayed_with_follow_up");
        assert!(summary.replay_outcome_label_matches_outcome());
        assert_eq!(
            summary.inventory_follow_up_kind(),
            ToolAuditInventoryFollowUpKind::MultipleFollowUp
        );
        assert_eq!(summary.inventory_follow_up_label(), "multiple_follow_up");
        assert!(summary.inventory_follow_up_label_matches_kind());
        assert_eq!(summary.inventory.failed_records, 1);
        assert_eq!(summary.inventory.follow_up_records, 1);
        assert_eq!(sink.records(), &[failed_record("call_2")]);
    }

    #[test]
    fn inventory_write_and_replay_health_labels_are_stable() {
        let clean_inventory =
            ToolAuditStoreInventorySummary::from_records(&[sample_record("call_clean")]);
        assert_eq!(
            clean_inventory.follow_up_kind(),
            ToolAuditInventoryFollowUpKind::NoFollowUp
        );
        assert_eq!(clean_inventory.follow_up_label(), "no_follow_up");
        assert!(!clean_inventory.follow_up_kind().requires_follow_up());

        let mut active = sample_record("call_active");
        active.status = ToolCallStatus::Running;
        let active_inventory = ToolAuditStoreInventorySummary::from_records(&[active]);
        assert_eq!(
            active_inventory.follow_up_kind(),
            ToolAuditInventoryFollowUpKind::ActiveRecords
        );
        assert_eq!(active_inventory.follow_up_label(), "active_records");

        let mut failed = sample_record("call_failed");
        failed.status = ToolCallStatus::Failed;
        let failed_inventory = ToolAuditStoreInventorySummary::from_records(&[failed]);
        assert_eq!(
            failed_inventory.follow_up_kind(),
            ToolAuditInventoryFollowUpKind::FailedRecords
        );
        assert_eq!(failed_inventory.follow_up_label(), "failed_records");

        let mut approval = sample_record("call_approval");
        approval.approval_state = ApprovalState::Pending;
        let approval_inventory = ToolAuditStoreInventorySummary::from_records(&[approval]);
        assert_eq!(
            approval_inventory.follow_up_kind(),
            ToolAuditInventoryFollowUpKind::ApprovalPressure
        );
        assert_eq!(approval_inventory.follow_up_label(), "approval_pressure");

        let mut error = sample_record("call_error");
        error.result_summary.has_error = true;
        let error_inventory = ToolAuditStoreInventorySummary::from_records(&[error]);
        assert_eq!(
            error_inventory.follow_up_kind(),
            ToolAuditInventoryFollowUpKind::ResultErrors
        );
        assert_eq!(error_inventory.follow_up_label(), "result_errors");
        assert_eq!(
            ToolAuditInventoryFollowUpKind::from_label("result_errors"),
            Some(ToolAuditInventoryFollowUpKind::ResultErrors)
        );

        let write_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let clean_write = write_store.record_audit_batch(vec![sample_record("write_clean")]);
        assert_eq!(
            clean_write.write_outcome(),
            ToolAuditBatchWriteOutcome::CompletedClean
        );
        assert_eq!(clean_write.write_outcome_label(), "completed_clean");
        assert!(clean_write.write_outcome().is_completed_clean());
        assert!(!clean_write.write_outcome().requires_follow_up());

        let follow_up_store = ToolAuditStore::new(InMemoryStorageBackend::new());
        let follow_up_write =
            follow_up_store.record_audit_batch(vec![failed_record("write_follow_up")]);
        assert_eq!(
            follow_up_write.write_outcome(),
            ToolAuditBatchWriteOutcome::CompletedWithFollowUp
        );
        assert_eq!(
            follow_up_write.write_outcome_label(),
            "completed_with_follow_up"
        );
        assert!(follow_up_write.write_outcome().requires_follow_up());

        let empty_replay = ToolAuditReplaySummary::default();
        assert_eq!(empty_replay.replay_outcome(), ToolAuditReplayOutcome::Empty);
        assert_eq!(empty_replay.replay_outcome_label(), "empty");

        let mut sink = InMemoryToolAuditSink::new();
        let clean_replay = write_store
            .replay_audits(&ToolAuditRecordQuery::new(), &mut sink)
            .unwrap();
        assert_eq!(
            clean_replay.replay_outcome(),
            ToolAuditReplayOutcome::ReplayedClean
        );
        assert_eq!(clean_replay.replay_outcome_label(), "replayed_clean");
        assert!(clean_replay.replay_outcome().is_replayed_clean());
        assert!(!clean_replay.replay_outcome().requires_follow_up());
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
    fn storage_sink_flushes_batches_and_tracks_failures() {
        let mut sink = StorageToolAuditSink::new(InMemoryStorageBackend::new());
        let first = sink.record_audit_batch(vec![sample_record("call_1"), failed_record("call_2")]);
        assert!(first.completed_without_failures());
        assert!(!sink.has_failures());

        let second = sink.record_audit_batch(vec![sample_record("call_1")]);
        assert_eq!(second.failed_records, 1);
        assert!(sink.has_failures());
        assert_eq!(sink.failures()[0].call_id, "call_1");
        assert_eq!(sink.store().inventory_summary().unwrap().total_records, 2);
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
