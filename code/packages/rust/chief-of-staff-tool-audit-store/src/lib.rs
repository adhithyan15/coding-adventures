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

    /// Return whether a drain tick would deliver at least one row.
    pub fn has_pending_records(&self) -> bool {
        !self.is_idle()
    }

    /// Return whether a supervisor should run a drain tick.
    pub fn should_drain(&self) -> bool {
        self.has_pending_records()
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

    /// Return whether this planned page has rows to drain.
    pub fn has_pending_records(&self) -> bool {
        !self.is_idle()
    }

    /// Return whether any inspected row needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.inventory.requires_follow_up()
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

    /// Return whether no rows are waiting in the planned run.
    pub fn is_idle(&self) -> bool {
        self.planned_records == 0
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

    /// Return whether no audit rows were replayed.
    pub fn is_idle(&self) -> bool {
        self.drained_records == 0
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

    /// Return whether this run outcome asks the scheduler to take action.
    pub fn requires_scheduler_action(&self) -> bool {
        self.outcome().requires_scheduler_action()
    }

    /// Return the recommended scheduler action for this drain run.
    pub fn scheduler_action(&self) -> ToolAuditSupervisorDrainSchedulerAction {
        self.outcome().scheduler_action()
    }

    /// Return a payload-free, flattened summary for host logs and schedulers.
    pub fn summary(&self) -> ToolAuditSupervisorDrainRunSummary {
        ToolAuditSupervisorDrainRunSummary {
            checkpoint_name: self.plan.checkpoint_name.clone(),
            outcome: self.outcome(),
            scheduler_action: self.scheduler_action(),
            max_records_per_tick: self.plan.max_records_per_tick,
            max_ticks: self.plan.max_ticks,
            planned_pages: self.plan.page_count(),
            drain_ticks: self.drain.tick_count(),
            planned_records: self.plan.planned_records,
            drained_records: self.drain.drained_records,
            planned_follow_up_records: self.plan.follow_up_record_count(),
            drained_follow_up_records: self.drain.follow_up_record_count(),
            record_count_delta: self.record_count_delta(),
            follow_up_record_count_delta: self.follow_up_record_count_delta(),
            has_record_count_drift: self.has_record_count_drift(),
            has_follow_up_record_count_drift: self.has_follow_up_record_count_drift(),
            count_drift_kind: self.count_drift_kind(),
            requires_count_drift_investigation: self.requires_count_drift_investigation(),
            matches_planned_record_count: self.matches_planned_record_count(),
            matches_planned_follow_up_record_count: self.matches_planned_follow_up_record_count(),
            reached_end_of_log: self.reached_end_of_log(),
            exhausted_tick_budget: self.exhausted_tick_budget(),
            requires_follow_up: self.requires_follow_up(),
            advanced_checkpoint: self.advanced_checkpoint(),
            last_checkpoint: self.last_checkpoint().cloned(),
        }
    }

    /// Return whether the actual run delivered the planned number of rows.
    pub fn matches_planned_record_count(&self) -> bool {
        self.plan.planned_records == self.drain.drained_records
    }

    /// Return whether the actual run preserved the planned follow-up pressure count.
    pub fn matches_planned_follow_up_record_count(&self) -> bool {
        self.plan.follow_up_record_count() == self.drain.follow_up_record_count()
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

    /// Return whether count drift should be investigated by the host.
    pub fn requires_count_drift_investigation(&self) -> bool {
        self.count_drift_kind().requires_investigation()
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

    /// Return the last replay checkpoint observed by the actual run.
    pub fn last_checkpoint(&self) -> Option<&ToolAuditReadCheckpoint> {
        self.drain.last_checkpoint()
    }
}

/// Payload-free, flattened host summary for a bounded supervisor drain run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAuditSupervisorDrainRunSummary {
    /// Reader or supervisor checkpoint name drained.
    pub checkpoint_name: String,
    /// Scheduler-facing classification for this run.
    pub outcome: ToolAuditSupervisorDrainRunOutcome,
    /// Recommended host scheduler action for this run.
    pub scheduler_action: ToolAuditSupervisorDrainSchedulerAction,
    /// Maximum rows each drain tick may replay.
    pub max_records_per_tick: usize,
    /// Maximum drain ticks requested for this run.
    pub max_ticks: usize,
    /// Number of pages discovered by the preflight plan.
    pub planned_pages: usize,
    /// Number of drain ticks that actually ran.
    pub drain_ticks: usize,
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
    /// Replayed-minus-planned follow-up pressure count delta.
    pub follow_up_record_count_delta: i128,
    /// Whether row count drift was observed.
    pub has_record_count_drift: bool,
    /// Whether follow-up pressure count drift was observed.
    pub has_follow_up_record_count_drift: bool,
    /// Stable classification of observed planned-vs-actual count drift.
    pub count_drift_kind: ToolAuditSupervisorDrainCountDriftKind,
    /// Whether count drift should be investigated by the host.
    pub requires_count_drift_investigation: bool,
    /// Whether the actual run delivered the planned number of rows.
    pub matches_planned_record_count: bool,
    /// Whether the actual run preserved the planned follow-up pressure count.
    pub matches_planned_follow_up_record_count: bool,
    /// Whether the actual run reached the current end of the audit log.
    pub reached_end_of_log: bool,
    /// Whether the actual run used every allowed tick.
    pub exhausted_tick_budget: bool,
    /// Whether planned or replayed rows need follow-up.
    pub requires_follow_up: bool,
    /// Whether any durable checkpoint advanced during the run.
    pub advanced_checkpoint: bool,
    /// Last replay checkpoint observed by the actual run.
    pub last_checkpoint: Option<ToolAuditReadCheckpoint>,
}

impl ToolAuditSupervisorDrainRunSummary {
    /// Return the stable scheduler-facing label for this run outcome.
    pub fn outcome_label(&self) -> &'static str {
        self.outcome.as_str()
    }

    /// Return whether this run outcome asks the scheduler to take action.
    pub fn requires_scheduler_action(&self) -> bool {
        self.scheduler_action.requires_scheduler_action()
    }

    /// Return whether the scheduler should run another drain pass.
    pub fn requests_continuation(&self) -> bool {
        self.scheduler_action.requests_continuation()
    }

    /// Return whether follow-up pressure should be routed to the host.
    pub fn routes_follow_up(&self) -> bool {
        self.scheduler_action.routes_follow_up()
    }

    /// Return whether the host should investigate preflight/drain drift.
    pub fn requires_plan_drift_investigation(&self) -> bool {
        self.scheduler_action.requires_plan_drift_investigation()
    }

    /// Return whether planned and replayed follow-up pressure counts match.
    pub fn matches_follow_up_pressure(&self) -> bool {
        self.matches_planned_follow_up_record_count
    }

    /// Return whether any planned-vs-actual count drift was observed.
    pub fn has_count_drift(&self) -> bool {
        self.has_record_count_drift || self.has_follow_up_record_count_drift
    }

    /// Return the stable count-drift classification label.
    pub fn count_drift_label(&self) -> &'static str {
        self.count_drift_kind.as_str()
    }

    /// Return whether count drift should be investigated by the host.
    pub fn requires_count_drift_investigation(&self) -> bool {
        self.requires_count_drift_investigation
    }

    /// Return whether the actual run replayed more rows than planned.
    pub fn replayed_extra_records(&self) -> bool {
        self.record_count_delta > 0
    }

    /// Return whether the actual run replayed fewer rows than planned.
    pub fn missed_planned_records(&self) -> bool {
        self.record_count_delta < 0
    }

    /// Return whether the actual run replayed more follow-up pressure than planned.
    pub fn replayed_extra_follow_up_records(&self) -> bool {
        self.follow_up_record_count_delta > 0
    }

    /// Return whether the actual run replayed less follow-up pressure than planned.
    pub fn missed_planned_follow_up_records(&self) -> bool {
        self.follow_up_record_count_delta < 0
    }

    /// Return the stable scheduler-action label for host logs.
    pub fn scheduler_action_label(&self) -> &'static str {
        self.scheduler_action.as_str()
    }

    /// Return whether no audit rows were replayed.
    pub fn is_idle(&self) -> bool {
        self.drained_records == 0
    }

    /// Return whether at least one audit row was replayed.
    pub fn made_progress(&self) -> bool {
        self.drained_records > 0
    }

    /// Return whether the supervisor should schedule another drain run.
    pub fn should_continue(&self) -> bool {
        !self.reached_end_of_log
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

    /// Return whether the scheduler should take action for this outcome.
    pub fn requires_scheduler_action(self) -> bool {
        self.scheduler_action().requires_scheduler_action()
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

    /// Return whether every attempted row was persisted.
    pub fn completed_without_failures(&self) -> bool {
        self.attempted_records == self.stored_records && self.failed_records == 0
    }

    /// Return whether any persisted row or storage failure needs follow-up.
    pub fn requires_follow_up(&self) -> bool {
        self.failed_records > 0 || self.inventory.requires_follow_up()
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

        let empty = store
            .query_audits_after_checkpoint(&second.next_checkpoint, Some(10))
            .unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.next_checkpoint, second.next_checkpoint);
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
        assert_eq!(older.checkpoint, first.checkpoint);
        assert_eq!(
            newer.checkpoint,
            ToolAuditReadCheckpoint::new(151, "call_3")
        );
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
        assert_eq!(
            plan.pages[0].next_checkpoint,
            ToolAuditReadCheckpoint::new(120, "call_3")
        );
        assert_eq!(plan.pages[1].pending_records, 2);
        assert_eq!(plan.pages[1].inventory.follow_up_records, 1);
        assert!(plan.pages[1].requires_follow_up());
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
        assert_eq!(plan.planned_records, 4);
        assert!(plan.reached_end_of_log());
        assert!(!plan.exhausted_tick_budget());
        assert!(!plan.should_continue());
        assert!(!plan.pages[2].has_pending_records());
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
        assert_eq!(plan.planned_records, 0);
        assert!(plan.is_idle());
        assert!(!plan.has_pending_records());
        assert!(!plan.requires_follow_up());
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
        assert_eq!(report.plan.max_ticks, 2);
        assert_eq!(report.plan.page_count(), 2);
        assert_eq!(report.plan.planned_records, 4);
        assert_eq!(report.drain.tick_count(), 2);
        assert_eq!(report.drain.drained_records, 4);
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
        assert_eq!(report.drain.follow_up_record_count(), 1);
        assert!(report.matches_planned_record_count());
        assert!(report.matches_planned_follow_up_record_count());
        assert_eq!(report.record_count_delta(), 0);
        assert_eq!(report.follow_up_record_count_delta(), 0);
        assert!(!report.has_record_count_drift());
        assert!(!report.has_follow_up_record_count_drift());
        assert!(!report.has_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::NoDrift
        );
        assert_eq!(report.count_drift_label(), "no_count_drift");
        assert!(!report.requires_count_drift_investigation());
        assert!(report.requires_follow_up());
        assert!(report.reached_end_of_log());
        assert!(!report.exhausted_tick_budget());
        assert!(!report.should_continue());
        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::NeedsFollowUp
        );
        let summary = report.summary();
        assert!(summary.requires_scheduler_action());
        assert!(!summary.requests_continuation());
        assert!(summary.routes_follow_up());
        assert!(!summary.requires_plan_drift_investigation());
        assert_eq!(summary.planned_follow_up_records, 1);
        assert_eq!(summary.drained_follow_up_records, 1);
        assert!(summary.matches_planned_follow_up_record_count);
        assert!(summary.matches_follow_up_pressure());
        assert_eq!(summary.record_count_delta, 0);
        assert_eq!(summary.follow_up_record_count_delta, 0);
        assert!(!summary.has_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(!summary.has_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::NoDrift
        );
        assert_eq!(summary.count_drift_label(), "no_count_drift");
        assert!(!summary.requires_count_drift_investigation);
        assert!(!summary.requires_count_drift_investigation());
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
        assert!(!report.made_progress());
        assert!(!report.advanced_checkpoint());
        assert!(report.reached_end_of_log());
        assert!(!report.should_continue());
        assert_eq!(report.outcome(), ToolAuditSupervisorDrainRunOutcome::Idle);
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
        assert_eq!(
            report.outcome(),
            ToolAuditSupervisorDrainRunOutcome::CaughtUp
        );
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
                ToolAuditSupervisorDrainSchedulerAction::NoAction,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::CaughtUp,
                "caught_up",
                ToolAuditSupervisorDrainSchedulerAction::NoAction,
                false,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::NeedsContinuation,
                "needs_continuation",
                ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation,
                true,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::NeedsFollowUp,
                "needs_follow_up",
                ToolAuditSupervisorDrainSchedulerAction::RouteFollowUp,
                true,
            ),
            (
                ToolAuditSupervisorDrainRunOutcome::PlanDiverged,
                "plan_diverged",
                ToolAuditSupervisorDrainSchedulerAction::InvestigatePlanDrift,
                true,
            ),
        ];

        for (outcome, label, scheduler_action, requires_action) in cases {
            assert_eq!(outcome.as_str(), label);
            assert_eq!(outcome.to_string(), label);
            assert_eq!(
                ToolAuditSupervisorDrainRunOutcome::from_label(label),
                Some(outcome)
            );
            assert_eq!(outcome.scheduler_action(), scheduler_action);
            assert_eq!(outcome.requires_scheduler_action(), requires_action);
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
        assert!(report.requires_scheduler_action());
        assert_eq!(
            report.scheduler_action(),
            ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation
        );
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
        assert_eq!(
            summary.outcome,
            ToolAuditSupervisorDrainRunOutcome::NeedsContinuation
        );
        assert_eq!(summary.outcome_label(), "needs_continuation");
        assert_eq!(
            summary.scheduler_action,
            ToolAuditSupervisorDrainSchedulerAction::ScheduleContinuation
        );
        assert_eq!(summary.scheduler_action_label(), "schedule_continuation");
        assert!(summary.requires_scheduler_action());
        assert!(summary.requests_continuation());
        assert!(!summary.routes_follow_up());
        assert!(!summary.requires_plan_drift_investigation());
        assert_eq!(summary.max_records_per_tick, 2);
        assert_eq!(summary.max_ticks, 1);
        assert_eq!(summary.planned_pages, 1);
        assert_eq!(summary.drain_ticks, 1);
        assert_eq!(summary.planned_records, 2);
        assert_eq!(summary.drained_records, 2);
        assert_eq!(summary.planned_follow_up_records, 0);
        assert_eq!(summary.drained_follow_up_records, 0);
        assert_eq!(summary.record_count_delta, 0);
        assert_eq!(summary.follow_up_record_count_delta, 0);
        assert!(!summary.has_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(!summary.has_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::NoDrift
        );
        assert_eq!(summary.count_drift_label(), "no_count_drift");
        assert!(!summary.requires_count_drift_investigation);
        assert!(!summary.requires_count_drift_investigation());
        assert!(summary.matches_planned_record_count);
        assert!(summary.matches_planned_follow_up_record_count);
        assert!(summary.matches_follow_up_pressure());
        assert!(!summary.reached_end_of_log);
        assert!(summary.exhausted_tick_budget);
        assert!(!summary.requires_follow_up);
        assert!(summary.advanced_checkpoint);
        assert_eq!(summary.last_checkpoint, report.last_checkpoint().cloned());
        assert!(!summary.is_idle());
        assert!(summary.made_progress());
        assert!(summary.should_continue());
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
        assert_eq!(summary.outcome_label(), "plan_diverged");
        assert_eq!(
            summary.scheduler_action,
            ToolAuditSupervisorDrainSchedulerAction::InvestigatePlanDrift
        );
        assert_eq!(summary.scheduler_action_label(), "investigate_plan_drift");
        assert!(!summary.matches_planned_record_count);
        assert_eq!(report.record_count_delta(), 1);
        assert!(report.has_record_count_drift());
        assert!(!report.has_follow_up_record_count_drift());
        assert!(report.has_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert_eq!(report.count_drift_label(), "record_count_drift");
        assert!(report.requires_count_drift_investigation());
        assert_eq!(summary.record_count_delta, 1);
        assert!(summary.has_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(summary.has_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert_eq!(summary.count_drift_label(), "record_count_drift");
        assert!(summary.requires_count_drift_investigation);
        assert!(summary.requires_count_drift_investigation());
        assert!(summary.replayed_extra_records());
        assert!(!summary.missed_planned_records());
        assert!(summary.requires_scheduler_action());
        assert!(!summary.requests_continuation());
        assert!(!summary.routes_follow_up());
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
        assert_eq!(report.record_count_delta(), -1);
        assert!(report.has_record_count_drift());
        assert!(!report.has_follow_up_record_count_drift());
        assert!(report.has_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert!(report.requires_count_drift_investigation());
        assert_eq!(summary.record_count_delta, -1);
        assert!(summary.has_record_count_drift);
        assert!(!summary.has_follow_up_record_count_drift);
        assert!(summary.has_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::RecordCountDrift
        );
        assert!(summary.requires_count_drift_investigation);
        assert!(summary.requires_count_drift_investigation());
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
        assert!(!report.has_record_count_drift());
        assert!(report.has_follow_up_record_count_drift());
        assert!(report.has_count_drift());
        assert_eq!(
            report.count_drift_kind(),
            ToolAuditSupervisorDrainCountDriftKind::FollowUpRecordCountDrift
        );
        assert_eq!(report.count_drift_label(), "follow_up_record_count_drift");
        assert!(report.requires_count_drift_investigation());
        assert_eq!(summary.record_count_delta, 0);
        assert_eq!(summary.follow_up_record_count_delta, -1);
        assert!(!summary.has_record_count_drift);
        assert!(summary.has_follow_up_record_count_drift);
        assert!(summary.has_count_drift());
        assert_eq!(
            summary.count_drift_kind,
            ToolAuditSupervisorDrainCountDriftKind::FollowUpRecordCountDrift
        );
        assert_eq!(summary.count_drift_label(), "follow_up_record_count_drift");
        assert!(summary.requires_count_drift_investigation);
        assert!(summary.requires_count_drift_investigation());
        assert!(summary.matches_planned_record_count);
        assert!(!summary.matches_planned_follow_up_record_count);
        assert!(!summary.matches_follow_up_pressure());
        assert!(!summary.replayed_extra_follow_up_records());
        assert!(summary.missed_planned_follow_up_records());
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
        assert_eq!(summary.inventory.total_records, 2);
        assert_eq!(summary.inventory.failed_records, 1);
        assert_eq!(summary.inventory.follow_up_records, 1);
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].call_id, "call_1");

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
        assert_eq!(summary.inventory.follow_up_records, 1);
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
