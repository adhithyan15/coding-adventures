//! # context-store
//!
//! `context-store` turns the low-level `storage-core` record API into the
//! higher-level session vocabulary a Chief of Staff runtime needs.
//!
//! At the storage layer every record is just:
//!
//! ```text
//! (namespace, key, metadata, body, revision)
//! ```
//!
//! At the context layer those records become:
//!
//! ```text
//! session  -->  ordered entries  -->  snapshots / compaction checkpoints
//! ```
//!
//! This crate owns:
//!
//! - typed Rust models for sessions, entries, and snapshots
//! - stable key layout on top of `storage-core`
//! - JSON encoding/decoding for context records
//! - compare-and-swap updates when mutating session state

use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::{parse as parse_json, JsonNumber, JsonValue};
use storage_core::{
    now_utc_ms, Revision, StorageBackend, StorageError, StorageListOptions, StoragePutInput,
    TimestampMs,
};

const NAMESPACE: &str = "context";

/// Lifecycle state of one session transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Paused,
    Archived,
}

impl SessionStatus {
    fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Paused => "paused",
            SessionStatus::Archived => "archived",
        }
    }

    fn from_str(value: &str) -> Result<Self, StorageError> {
        match value {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "archived" => Ok(Self::Archived),
            _ => Err(validation(
                "status",
                format!("unsupported session status '{value}'"),
            )),
        }
    }
}

/// Type of one context entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextEntryKind {
    User,
    Assistant,
    ToolCall,
    ToolResult,
    Summary,
    Note,
    AttachmentRef,
}

impl ContextEntryKind {
    fn as_str(self) -> &'static str {
        match self {
            ContextEntryKind::User => "user",
            ContextEntryKind::Assistant => "assistant",
            ContextEntryKind::ToolCall => "tool_call",
            ContextEntryKind::ToolResult => "tool_result",
            ContextEntryKind::Summary => "summary",
            ContextEntryKind::Note => "note",
            ContextEntryKind::AttachmentRef => "attachment_ref",
        }
    }

    fn from_str(value: &str) -> Result<Self, StorageError> {
        match value {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "tool_call" => Ok(Self::ToolCall),
            "tool_result" => Ok(Self::ToolResult),
            "summary" => Ok(Self::Summary),
            "note" => Ok(Self::Note),
            "attachment_ref" => Ok(Self::AttachmentRef),
            _ => Err(validation(
                "kind",
                format!("unsupported context entry kind '{value}'"),
            )),
        }
    }
}

/// Session header stored under `sessions/<session_id>.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextSession {
    pub session_id: String,
    pub owner_id: String,
    pub title: String,
    pub status: SessionStatus,
    pub latest_revision: Option<String>,
    pub head_pointer: Option<String>,
}

/// Compact read-side view of a selected set of session headers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContextSessionCatalogSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub paused_sessions: usize,
    pub archived_sessions: usize,
    pub sessions_with_snapshots: usize,
    pub sessions_without_snapshots: usize,
}

impl ContextSessionCatalogSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_sessions<'a, I>(sessions: I) -> Self
    where
        I: IntoIterator<Item = &'a ContextSession>,
    {
        let mut summary = Self::empty();
        for session in sessions {
            summary.total_sessions += 1;
            match session.status {
                SessionStatus::Active => summary.active_sessions += 1,
                SessionStatus::Paused => summary.paused_sessions += 1,
                SessionStatus::Archived => summary.archived_sessions += 1,
            }
            if session.head_pointer.is_some() {
                summary.sessions_with_snapshots += 1;
            } else {
                summary.sessions_without_snapshots += 1;
            }
        }
        summary
    }

    pub fn open_sessions(&self) -> usize {
        self.active_sessions + self.paused_sessions
    }

    pub fn has_open_sessions(&self) -> bool {
        self.open_sessions() > 0
    }

    pub fn has_uncheckpointed_sessions(&self) -> bool {
        self.sessions_without_snapshots > 0
    }
}

/// Portable ordering for bounded session list/read tools.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SessionListSort {
    #[default]
    SessionId,
    OwnerThenTitle,
    StatusThenTitle,
    Title,
}

/// Options for listing session headers without reading transcript bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionListOptions {
    pub owner_id: Option<String>,
    pub statuses: Vec<SessionStatus>,
    pub sort: SessionListSort,
    pub limit: Option<usize>,
}

impl Default for SessionListOptions {
    fn default() -> Self {
        Self {
            owner_id: None,
            statuses: Vec::new(),
            sort: SessionListSort::SessionId,
            limit: None,
        }
    }
}

impl SessionListOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_owner(mut self, owner_id: impl Into<String>) -> Self {
        self.owner_id = Some(owner_id.into());
        self
    }

    pub fn with_status(mut self, status: SessionStatus) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn sorted_by(mut self, sort: SessionListSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// One ordered event in a session transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextEntry {
    pub entry_id: String,
    pub session_id: String,
    pub kind: ContextEntryKind,
    pub timestamp: TimestampMs,
    pub metadata: JsonValue,
    pub body: JsonValue,
}

/// Body-free projection of one context entry for read-side transcript indexes.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextEntrySummary {
    pub entry_id: String,
    pub session_id: String,
    pub kind: ContextEntryKind,
    pub timestamp: TimestampMs,
    pub metadata: JsonValue,
}

impl ContextEntrySummary {
    pub fn from_entry(entry: &ContextEntry) -> Self {
        Self {
            entry_id: entry.entry_id.clone(),
            session_id: entry.session_id.clone(),
            kind: entry.kind,
            timestamp: entry.timestamp,
            metadata: entry.metadata.clone(),
        }
    }
}

/// Compact aggregate over body-free transcript entry summaries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContextTranscriptSummary {
    pub total_entries: usize,
    pub user_entries: usize,
    pub assistant_entries: usize,
    pub tool_call_entries: usize,
    pub tool_result_entries: usize,
    pub summary_entries: usize,
    pub note_entries: usize,
    pub attachment_ref_entries: usize,
    pub entries_with_metadata: usize,
    pub first_timestamp: Option<TimestampMs>,
    pub latest_timestamp: Option<TimestampMs>,
}

impl ContextTranscriptSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_entry_summaries<'a, I>(entries: I) -> Self
    where
        I: IntoIterator<Item = &'a ContextEntrySummary>,
    {
        let mut summary = Self::empty();
        for entry in entries {
            summary.total_entries += 1;
            match entry.kind {
                ContextEntryKind::User => summary.user_entries += 1,
                ContextEntryKind::Assistant => summary.assistant_entries += 1,
                ContextEntryKind::ToolCall => summary.tool_call_entries += 1,
                ContextEntryKind::ToolResult => summary.tool_result_entries += 1,
                ContextEntryKind::Summary => summary.summary_entries += 1,
                ContextEntryKind::Note => summary.note_entries += 1,
                ContextEntryKind::AttachmentRef => summary.attachment_ref_entries += 1,
            }
            if metadata_has_fields(&entry.metadata) {
                summary.entries_with_metadata += 1;
            }
            summary.first_timestamp = Some(
                summary
                    .first_timestamp
                    .map_or(entry.timestamp, |timestamp| timestamp.min(entry.timestamp)),
            );
            summary.latest_timestamp = Some(
                summary
                    .latest_timestamp
                    .map_or(entry.timestamp, |timestamp| timestamp.max(entry.timestamp)),
            );
        }
        summary
    }

    pub fn record_summary(&mut self, summary: &Self) {
        self.total_entries += summary.total_entries;
        self.user_entries += summary.user_entries;
        self.assistant_entries += summary.assistant_entries;
        self.tool_call_entries += summary.tool_call_entries;
        self.tool_result_entries += summary.tool_result_entries;
        self.summary_entries += summary.summary_entries;
        self.note_entries += summary.note_entries;
        self.attachment_ref_entries += summary.attachment_ref_entries;
        self.entries_with_metadata += summary.entries_with_metadata;
        if let Some(timestamp) = summary.first_timestamp {
            self.first_timestamp = Some(
                self.first_timestamp
                    .map_or(timestamp, |current| current.min(timestamp)),
            );
        }
        if let Some(timestamp) = summary.latest_timestamp {
            self.latest_timestamp = Some(
                self.latest_timestamp
                    .map_or(timestamp, |current| current.max(timestamp)),
            );
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_entries == 0
    }

    pub fn conversational_entries(&self) -> usize {
        self.user_entries + self.assistant_entries
    }

    pub fn tool_interaction_entries(&self) -> usize {
        self.tool_call_entries + self.tool_result_entries
    }

    pub fn has_compaction_material(&self) -> bool {
        self.summary_entries > 0 || self.note_entries > 0 || self.attachment_ref_entries > 0
    }
}

/// One compaction/checkpoint snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextSnapshot {
    pub snapshot_id: String,
    pub session_id: String,
    pub basis_entry_id: String,
    pub token_estimate: u64,
    pub included_entry_ids: Vec<String>,
    pub summary_refs: Vec<String>,
    pub memory_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
}

/// Compact aggregate over compaction snapshots for read-side inspectors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContextSnapshotSummary {
    pub total_snapshots: usize,
    pub included_entry_refs: usize,
    pub summary_refs: usize,
    pub memory_refs: usize,
    pub artifact_refs: usize,
    pub snapshots_with_memory_refs: usize,
    pub snapshots_with_artifact_refs: usize,
    pub total_token_estimate: u64,
    pub min_token_estimate: Option<u64>,
    pub max_token_estimate: Option<u64>,
}

impl ContextSnapshotSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_snapshots<'a, I>(snapshots: I) -> Self
    where
        I: IntoIterator<Item = &'a ContextSnapshot>,
    {
        let mut summary = Self::empty();
        for snapshot in snapshots {
            summary.total_snapshots += 1;
            summary.included_entry_refs += snapshot.included_entry_ids.len();
            summary.summary_refs += snapshot.summary_refs.len();
            summary.memory_refs += snapshot.memory_refs.len();
            summary.artifact_refs += snapshot.artifact_refs.len();
            summary.total_token_estimate = summary
                .total_token_estimate
                .saturating_add(snapshot.token_estimate);
            summary.min_token_estimate = Some(
                summary
                    .min_token_estimate
                    .map_or(snapshot.token_estimate, |tokens| {
                        tokens.min(snapshot.token_estimate)
                    }),
            );
            summary.max_token_estimate = Some(
                summary
                    .max_token_estimate
                    .map_or(snapshot.token_estimate, |tokens| {
                        tokens.max(snapshot.token_estimate)
                    }),
            );
            if !snapshot.memory_refs.is_empty() {
                summary.snapshots_with_memory_refs += 1;
            }
            if !snapshot.artifact_refs.is_empty() {
                summary.snapshots_with_artifact_refs += 1;
            }
        }
        summary
    }

    pub fn record_summary(&mut self, summary: &Self) {
        self.total_snapshots += summary.total_snapshots;
        self.included_entry_refs += summary.included_entry_refs;
        self.summary_refs += summary.summary_refs;
        self.memory_refs += summary.memory_refs;
        self.artifact_refs += summary.artifact_refs;
        self.snapshots_with_memory_refs += summary.snapshots_with_memory_refs;
        self.snapshots_with_artifact_refs += summary.snapshots_with_artifact_refs;
        self.total_token_estimate = self
            .total_token_estimate
            .saturating_add(summary.total_token_estimate);
        if let Some(tokens) = summary.min_token_estimate {
            self.min_token_estimate = Some(
                self.min_token_estimate
                    .map_or(tokens, |current| current.min(tokens)),
            );
        }
        if let Some(tokens) = summary.max_token_estimate {
            self.max_token_estimate = Some(
                self.max_token_estimate
                    .map_or(tokens, |current| current.max(tokens)),
            );
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_snapshots == 0
    }

    pub fn has_memory_refs(&self) -> bool {
        self.memory_refs > 0
    }

    pub fn has_artifact_refs(&self) -> bool {
        self.artifact_refs > 0
    }
}

/// Compact one-session overview for host status and compaction planners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextSessionSummary {
    pub status: SessionStatus,
    pub has_head_pointer: bool,
    pub head_snapshot_found: bool,
    pub transcript: ContextTranscriptSummary,
    pub snapshots: ContextSnapshotSummary,
    pub uncheckpointed_entries: usize,
}

impl ContextSessionSummary {
    pub fn from_parts(
        session: &ContextSession,
        entries: &[ContextEntrySummary],
        snapshots: &[ContextSnapshot],
    ) -> Self {
        let transcript = ContextTranscriptSummary::from_entry_summaries(entries);
        let snapshot_summary = ContextSnapshotSummary::from_snapshots(snapshots);
        let head_snapshot = session.head_pointer.as_deref().and_then(|head_pointer| {
            snapshots
                .iter()
                .find(|snapshot| snapshot.snapshot_id == head_pointer)
        });
        let uncheckpointed_entries = match head_snapshot {
            Some(snapshot) => entries
                .iter()
                .filter(|entry| !snapshot.included_entry_ids.contains(&entry.entry_id))
                .count(),
            None => entries.len(),
        };

        Self {
            status: session.status,
            has_head_pointer: session.head_pointer.is_some(),
            head_snapshot_found: head_snapshot.is_some(),
            transcript,
            snapshots: snapshot_summary,
            uncheckpointed_entries,
        }
    }

    pub fn is_archived(&self) -> bool {
        self.status == SessionStatus::Archived
    }

    pub fn has_missing_head_snapshot(&self) -> bool {
        self.has_head_pointer && !self.head_snapshot_found
    }

    pub fn has_uncheckpointed_entries(&self) -> bool {
        self.uncheckpointed_entries > 0
    }

    pub fn has_tool_activity(&self) -> bool {
        self.transcript.tool_interaction_entries() > 0
    }

    pub fn has_external_refs(&self) -> bool {
        self.snapshots.has_memory_refs() || self.snapshots.has_artifact_refs()
    }
}

/// Store-level context inventory for D18A host and compaction status checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextStoreInventorySummary {
    pub sessions: ContextSessionCatalogSummary,
    pub transcripts: ContextTranscriptSummary,
    pub snapshots: ContextSnapshotSummary,
    pub sessions_with_uncheckpointed_entries: usize,
    pub sessions_with_missing_head_snapshots: usize,
    pub sessions_with_tool_activity: usize,
    pub sessions_with_external_refs: usize,
}

impl ContextStoreInventorySummary {
    pub fn empty() -> Self {
        Self {
            sessions: ContextSessionCatalogSummary::empty(),
            transcripts: ContextTranscriptSummary::empty(),
            snapshots: ContextSnapshotSummary::empty(),
            sessions_with_uncheckpointed_entries: 0,
            sessions_with_missing_head_snapshots: 0,
            sessions_with_tool_activity: 0,
            sessions_with_external_refs: 0,
        }
    }

    pub fn from_parts(sessions: &[ContextSession], summaries: &[ContextSessionSummary]) -> Self {
        let mut inventory = Self {
            sessions: ContextSessionCatalogSummary::from_sessions(sessions),
            ..Self::empty()
        };

        for summary in summaries {
            inventory.transcripts.record_summary(&summary.transcript);
            inventory.snapshots.record_summary(&summary.snapshots);
            if summary.has_uncheckpointed_entries() {
                inventory.sessions_with_uncheckpointed_entries += 1;
            }
            if summary.has_missing_head_snapshot() {
                inventory.sessions_with_missing_head_snapshots += 1;
            }
            if summary.has_tool_activity() {
                inventory.sessions_with_tool_activity += 1;
            }
            if summary.has_external_refs() {
                inventory.sessions_with_external_refs += 1;
            }
        }

        inventory
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.total_sessions == 0
    }

    pub fn has_context_material(&self) -> bool {
        self.transcripts.total_entries > 0 || self.snapshots.total_snapshots > 0
    }

    pub fn has_compaction_attention_items(&self) -> bool {
        self.sessions_with_uncheckpointed_entries > 0
            || self.sessions_with_missing_head_snapshots > 0
    }

    pub fn has_tool_activity(&self) -> bool {
        self.sessions_with_tool_activity > 0
    }

    pub fn has_external_refs(&self) -> bool {
        self.sessions_with_external_refs > 0
    }
}

/// Portable ordering for bounded snapshot list/read tools.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SnapshotListSort {
    #[default]
    SnapshotId,
    BasisEntryId,
    TokenEstimateAsc,
    TokenEstimateDesc,
}

/// Options for listing compaction snapshots without reading transcript bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotListOptions {
    pub basis_entry_id: Option<String>,
    pub summary_refs: Vec<String>,
    pub memory_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub sort: SnapshotListSort,
    pub limit: Option<usize>,
}

impl Default for SnapshotListOptions {
    fn default() -> Self {
        Self {
            basis_entry_id: None,
            summary_refs: Vec::new(),
            memory_refs: Vec::new(),
            artifact_refs: Vec::new(),
            sort: SnapshotListSort::SnapshotId,
            limit: None,
        }
    }
}

impl SnapshotListOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_basis_entry(mut self, basis_entry_id: impl Into<String>) -> Self {
        self.basis_entry_id = Some(basis_entry_id.into());
        self
    }

    pub fn with_summary_ref(mut self, summary_ref: impl Into<String>) -> Self {
        self.summary_refs.push(summary_ref.into());
        self
    }

    pub fn with_memory_ref(mut self, memory_ref: impl Into<String>) -> Self {
        self.memory_refs.push(memory_ref.into());
        self
    }

    pub fn with_artifact_ref(mut self, artifact_ref: impl Into<String>) -> Self {
        self.artifact_refs.push(artifact_ref.into());
        self
    }

    pub fn sorted_by(mut self, sort: SnapshotListSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Input used when creating a new session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSessionInput {
    pub session_id: String,
    pub owner_id: String,
    pub title: String,
}

/// Input used when appending one entry.
#[derive(Debug, Clone, PartialEq)]
pub struct AppendEntryInput {
    pub entry_id: String,
    pub kind: ContextEntryKind,
    pub timestamp: Option<TimestampMs>,
    pub metadata: JsonValue,
    pub body: JsonValue,
}

/// Options for reading a bounded window of ordered entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FetchEntriesOptions {
    pub after_entry_id: Option<String>,
    pub kinds: Vec<ContextEntryKind>,
    pub since_timestamp: Option<TimestampMs>,
    pub until_timestamp: Option<TimestampMs>,
    pub limit: Option<usize>,
}

impl FetchEntriesOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after_entry(mut self, entry_id: impl Into<String>) -> Self {
        self.after_entry_id = Some(entry_id.into());
        self
    }

    pub fn with_kind(mut self, kind: ContextEntryKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn since(mut self, timestamp: TimestampMs) -> Self {
        self.since_timestamp = Some(timestamp);
        self
    }

    pub fn until(mut self, timestamp: TimestampMs) -> Self {
        self.until_timestamp = Some(timestamp);
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Input used when creating a snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSnapshotInput {
    pub snapshot_id: String,
    pub basis_entry_id: String,
    pub token_estimate: u64,
    pub included_entry_ids: Vec<String>,
    pub summary_refs: Vec<String>,
    pub memory_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
}

/// Typed context store layered on one `StorageBackend`.
pub struct ContextStore<S: StorageBackend> {
    backend: S,
}

impl<S: StorageBackend> ContextStore<S> {
    /// Wrap a storage backend with context semantics.
    pub fn new(backend: S) -> Self {
        Self { backend }
    }

    /// Borrow the underlying backend.
    pub fn backend(&self) -> &S {
        &self.backend
    }

    /// Create a new session header.
    pub fn create_session(
        &self,
        input: CreateSessionInput,
    ) -> Result<ContextSession, StorageError> {
        validate_id("session_id", &input.session_id)?;
        validate_id("owner_id", &input.owner_id)?;
        validate_title(&input.title)?;

        self.backend.initialize()?;
        let session = ContextSession {
            session_id: input.session_id,
            owner_id: input.owner_id,
            title: input.title,
            status: SessionStatus::Active,
            latest_revision: None,
            head_pointer: None,
        };
        self.persist_session(&session, None)
    }

    /// Open one session by id.
    pub fn open_session(&self, session_id: &str) -> Result<Option<ContextSession>, StorageError> {
        validate_id("session_id", session_id)?;
        self.backend.initialize()?;
        let Some(record) = self.backend.get(NAMESPACE, &session_key(session_id))? else {
            return Ok(None);
        };
        decode_session_record(&record.body, Some(record.revision.to_string())).map(Some)
    }

    /// List session headers with portable owner/status/sort/limit filters.
    pub fn list_sessions(
        &self,
        options: SessionListOptions,
    ) -> Result<Vec<ContextSession>, StorageError> {
        validate_session_list_options(&options)?;
        self.backend.initialize()?;
        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some("sessions/".to_string()),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;

        let mut sessions = page
            .records
            .iter()
            .map(|record| decode_session_record(&record.body, Some(record.revision.to_string())))
            .filter(|result| {
                result
                    .as_ref()
                    .map(|session| session_matches_list_options(session, &options))
                    .unwrap_or(true)
            })
            .collect::<Result<Vec<_>, _>>()?;
        sort_sessions(&mut sessions, options.sort);
        if let Some(limit) = options.limit {
            sessions.truncate(limit);
        }
        Ok(sessions)
    }

    /// Summarize selected session headers without reading transcript bodies.
    pub fn session_catalog_summary(
        &self,
        options: SessionListOptions,
    ) -> Result<ContextSessionCatalogSummary, StorageError> {
        let sessions = self.list_sessions(options)?;
        Ok(ContextSessionCatalogSummary::from_sessions(&sessions))
    }

    /// Append one entry to a session transcript.
    pub fn append_entry(
        &self,
        session_id: &str,
        input: AppendEntryInput,
    ) -> Result<ContextEntry, StorageError> {
        validate_id("session_id", session_id)?;
        validate_id("entry_id", &input.entry_id)?;
        validate_json_object("metadata", &input.metadata)?;

        let Some((session, revision)) = self.fetch_session_with_revision(session_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: session_key(session_id),
            });
        };
        if session.status == SessionStatus::Archived {
            return Err(validation(
                "status",
                "cannot append entries to an archived session",
            ));
        }

        let entry = ContextEntry {
            entry_id: input.entry_id,
            session_id: session_id.to_string(),
            kind: input.kind,
            timestamp: input.timestamp.unwrap_or_else(now_utc_ms),
            metadata: input.metadata,
            body: input.body,
        };
        let key = entry_key(&entry.session_id, entry.timestamp, &entry.entry_id);
        let body = encode_json(&entry_to_json(&entry))?;
        self.backend.put(StoragePutInput::new(
            NAMESPACE,
            key,
            "application/json",
            entry_record_metadata(&entry),
            body,
        )?)?;

        let mut updated_session = session;
        updated_session.latest_revision = Some(revision.to_string());
        let _ = self.persist_session(&updated_session, Some(revision))?;
        Ok(entry)
    }

    /// Fetch ordered entries for one session.
    pub fn fetch_ordered_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<ContextEntry>, StorageError> {
        self.fetch_entries(session_id, FetchEntriesOptions::default())
    }

    /// Fetch a bounded ordered entry window for one session.
    pub fn fetch_entries(
        &self,
        session_id: &str,
        options: FetchEntriesOptions,
    ) -> Result<Vec<ContextEntry>, StorageError> {
        validate_id("session_id", session_id)?;
        validate_fetch_entries_options(&options)?;

        self.backend.initialize()?;
        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some(format!("entries/{session_id}/")),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;

        let entries = page
            .records
            .iter()
            .map(|record| decode_entry_record(&record.body))
            .collect::<Result<Vec<_>, _>>()?;
        window_entries(entries, options)
    }

    /// Fetch body-free ordered entry summaries for one session.
    pub fn fetch_entry_summaries(
        &self,
        session_id: &str,
        options: FetchEntriesOptions,
    ) -> Result<Vec<ContextEntrySummary>, StorageError> {
        Ok(self
            .fetch_entries(session_id, options)?
            .iter()
            .map(ContextEntrySummary::from_entry)
            .collect())
    }

    /// Summarize a body-free ordered entry window for read-side tooling.
    pub fn transcript_summary(
        &self,
        session_id: &str,
        options: FetchEntriesOptions,
    ) -> Result<ContextTranscriptSummary, StorageError> {
        let entries = self.fetch_entry_summaries(session_id, options)?;
        Ok(ContextTranscriptSummary::from_entry_summaries(&entries))
    }

    /// Create a new snapshot and advance the session head pointer to it.
    pub fn create_snapshot(
        &self,
        session_id: &str,
        input: CreateSnapshotInput,
    ) -> Result<ContextSnapshot, StorageError> {
        validate_id("session_id", session_id)?;
        validate_id("snapshot_id", &input.snapshot_id)?;
        validate_id("basis_entry_id", &input.basis_entry_id)?;
        validate_id_list("included_entry_ids", &input.included_entry_ids)?;
        validate_id_list("summary_refs", &input.summary_refs)?;
        validate_id_list("memory_refs", &input.memory_refs)?;
        validate_id_list("artifact_refs", &input.artifact_refs)?;

        let Some((session, revision)) = self.fetch_session_with_revision(session_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: session_key(session_id),
            });
        };

        let snapshot = ContextSnapshot {
            snapshot_id: input.snapshot_id,
            session_id: session_id.to_string(),
            basis_entry_id: input.basis_entry_id,
            token_estimate: input.token_estimate,
            included_entry_ids: input.included_entry_ids,
            summary_refs: input.summary_refs,
            memory_refs: input.memory_refs,
            artifact_refs: input.artifact_refs,
        };
        let key = snapshot_key(session_id, &snapshot.snapshot_id);
        self.backend.put(StoragePutInput::new(
            NAMESPACE,
            key,
            "application/json",
            snapshot_record_metadata(&snapshot),
            encode_json(&snapshot_to_json(&snapshot))?,
        )?)?;

        let mut updated_session = session;
        updated_session.head_pointer = Some(snapshot.snapshot_id.clone());
        updated_session.latest_revision = Some(revision.to_string());
        let _ = self.persist_session(&updated_session, Some(revision))?;
        Ok(snapshot)
    }

    /// Fetch the latest snapshot using the session head pointer.
    pub fn fetch_latest_snapshot(
        &self,
        session_id: &str,
    ) -> Result<Option<ContextSnapshot>, StorageError> {
        let Some(session) = self.open_session(session_id)? else {
            return Ok(None);
        };
        let Some(snapshot_id) = session.head_pointer.as_deref() else {
            return Ok(None);
        };
        let Some(record) = self
            .backend
            .get(NAMESPACE, &snapshot_key(session_id, snapshot_id))?
        else {
            return Ok(None);
        };
        decode_snapshot_record(&record.body).map(Some)
    }

    /// List compaction snapshots for one session with portable filters.
    pub fn list_snapshots(
        &self,
        session_id: &str,
        options: SnapshotListOptions,
    ) -> Result<Vec<ContextSnapshot>, StorageError> {
        validate_id("session_id", session_id)?;
        validate_snapshot_list_options(&options)?;

        self.backend.initialize()?;
        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some(format!("snapshots/{session_id}/")),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;

        let mut snapshots = page
            .records
            .iter()
            .map(|record| decode_snapshot_record(&record.body))
            .filter(|result| {
                result
                    .as_ref()
                    .map(|snapshot| snapshot_matches_list_options(snapshot, &options))
                    .unwrap_or(true)
            })
            .collect::<Result<Vec<_>, _>>()?;
        sort_snapshots(&mut snapshots, options.sort);
        if let Some(limit) = options.limit {
            snapshots.truncate(limit);
        }
        Ok(snapshots)
    }

    /// Summarize selected compaction snapshots without reading transcript bodies.
    pub fn snapshot_summary(
        &self,
        session_id: &str,
        options: SnapshotListOptions,
    ) -> Result<ContextSnapshotSummary, StorageError> {
        let snapshots = self.list_snapshots(session_id, options)?;
        Ok(ContextSnapshotSummary::from_snapshots(&snapshots))
    }

    /// Summarize one session across header, transcript, and checkpoints.
    pub fn session_summary(
        &self,
        session_id: &str,
    ) -> Result<Option<ContextSessionSummary>, StorageError> {
        let Some(session) = self.open_session(session_id)? else {
            return Ok(None);
        };
        let entries = self.fetch_entry_summaries(session_id, FetchEntriesOptions::default())?;
        let snapshots = self.list_snapshots(session_id, SnapshotListOptions::default())?;
        Ok(Some(ContextSessionSummary::from_parts(
            &session, &entries, &snapshots,
        )))
    }

    /// Summarize selected sessions across catalog, transcript, and checkpoints.
    pub fn inventory_summary(
        &self,
        options: SessionListOptions,
    ) -> Result<ContextStoreInventorySummary, StorageError> {
        let sessions = self.list_sessions(options)?;
        let mut summaries = Vec::with_capacity(sessions.len());
        for session in &sessions {
            let entries =
                self.fetch_entry_summaries(&session.session_id, FetchEntriesOptions::default())?;
            let snapshots =
                self.list_snapshots(&session.session_id, SnapshotListOptions::default())?;
            summaries.push(ContextSessionSummary::from_parts(
                session, &entries, &snapshots,
            ));
        }
        Ok(ContextStoreInventorySummary::from_parts(
            &sessions, &summaries,
        ))
    }

    /// Create a compaction snapshot that covers all entries up to and including
    /// `basis_entry_id` and references an already-created summary entry.
    pub fn compact_before_entry(
        &self,
        session_id: &str,
        basis_entry_id: &str,
        summary_entry_id: &str,
    ) -> Result<ContextSnapshot, StorageError> {
        validate_id("basis_entry_id", basis_entry_id)?;
        validate_id("summary_entry_id", summary_entry_id)?;

        let entries = self.fetch_ordered_entries(session_id)?;
        let mut included = Vec::new();
        let mut reached_basis = false;
        for entry in entries {
            included.push(entry.entry_id.clone());
            if entry.entry_id == basis_entry_id {
                reached_basis = true;
                break;
            }
        }

        if !reached_basis {
            return Err(validation(
                "basis_entry_id",
                format!("entry '{basis_entry_id}' was not found in session '{session_id}'"),
            ));
        }

        self.create_snapshot(
            session_id,
            CreateSnapshotInput {
                snapshot_id: format!("compact-{basis_entry_id}"),
                basis_entry_id: basis_entry_id.to_string(),
                token_estimate: included.len() as u64,
                included_entry_ids: included,
                summary_refs: vec![summary_entry_id.to_string()],
                memory_refs: Vec::new(),
                artifact_refs: Vec::new(),
            },
        )
    }

    /// Mark a session as archived.
    pub fn archive_session(&self, session_id: &str) -> Result<ContextSession, StorageError> {
        let Some((mut session, revision)) = self.fetch_session_with_revision(session_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: session_key(session_id),
            });
        };
        session.status = SessionStatus::Archived;
        self.persist_session(&session, Some(revision))
    }

    fn fetch_session_with_revision(
        &self,
        session_id: &str,
    ) -> Result<Option<(ContextSession, Revision)>, StorageError> {
        self.backend.initialize()?;
        let Some(record) = self.backend.get(NAMESPACE, &session_key(session_id))? else {
            return Ok(None);
        };
        let session = decode_session_record(&record.body, Some(record.revision.to_string()))?;
        Ok(Some((session, record.revision)))
    }

    fn persist_session(
        &self,
        session: &ContextSession,
        if_revision: Option<Revision>,
    ) -> Result<ContextSession, StorageError> {
        let body = encode_json(&session_to_json(session))?;
        let record = self.backend.put(
            StoragePutInput::new(
                NAMESPACE,
                session_key(&session.session_id),
                "application/json",
                session_record_metadata(session),
                body,
            )?
            .with_if_revision(if_revision),
        )?;
        decode_session_record(&record.body, Some(record.revision.to_string()))
    }
}

fn session_key(session_id: &str) -> String {
    format!("sessions/{session_id}.json")
}

fn entry_key(session_id: &str, timestamp: TimestampMs, entry_id: &str) -> String {
    format!("entries/{session_id}/{timestamp:020}-{entry_id}.json")
}

fn snapshot_key(session_id: &str, snapshot_id: &str) -> String {
    format!("snapshots/{session_id}/{snapshot_id}.json")
}

fn session_record_metadata(session: &ContextSession) -> JsonValue {
    JsonValue::Object(vec![
        (
            "owner_id".to_string(),
            JsonValue::String(session.owner_id.clone()),
        ),
        (
            "status".to_string(),
            JsonValue::String(session.status.as_str().to_string()),
        ),
    ])
}

fn entry_record_metadata(entry: &ContextEntry) -> JsonValue {
    JsonValue::Object(vec![
        (
            "session_id".to_string(),
            JsonValue::String(entry.session_id.clone()),
        ),
        (
            "entry_id".to_string(),
            JsonValue::String(entry.entry_id.clone()),
        ),
        (
            "kind".to_string(),
            JsonValue::String(entry.kind.as_str().to_string()),
        ),
        (
            "timestamp".to_string(),
            JsonValue::Number(JsonNumber::Integer(entry.timestamp as i64)),
        ),
    ])
}

fn snapshot_record_metadata(snapshot: &ContextSnapshot) -> JsonValue {
    JsonValue::Object(vec![
        (
            "session_id".to_string(),
            JsonValue::String(snapshot.session_id.clone()),
        ),
        (
            "snapshot_id".to_string(),
            JsonValue::String(snapshot.snapshot_id.clone()),
        ),
        (
            "basis_entry_id".to_string(),
            JsonValue::String(snapshot.basis_entry_id.clone()),
        ),
    ])
}

fn session_to_json(session: &ContextSession) -> JsonValue {
    JsonValue::Object(vec![
        (
            "session_id".to_string(),
            JsonValue::String(session.session_id.clone()),
        ),
        (
            "owner_id".to_string(),
            JsonValue::String(session.owner_id.clone()),
        ),
        (
            "title".to_string(),
            JsonValue::String(session.title.clone()),
        ),
        (
            "status".to_string(),
            JsonValue::String(session.status.as_str().to_string()),
        ),
        (
            "latest_revision".to_string(),
            optional_string_json(session.latest_revision.as_deref()),
        ),
        (
            "head_pointer".to_string(),
            optional_string_json(session.head_pointer.as_deref()),
        ),
    ])
}

fn entry_to_json(entry: &ContextEntry) -> JsonValue {
    JsonValue::Object(vec![
        (
            "entry_id".to_string(),
            JsonValue::String(entry.entry_id.clone()),
        ),
        (
            "session_id".to_string(),
            JsonValue::String(entry.session_id.clone()),
        ),
        (
            "kind".to_string(),
            JsonValue::String(entry.kind.as_str().to_string()),
        ),
        (
            "timestamp".to_string(),
            JsonValue::Number(JsonNumber::Integer(entry.timestamp as i64)),
        ),
        ("metadata".to_string(), entry.metadata.clone()),
        ("body".to_string(), entry.body.clone()),
    ])
}

fn snapshot_to_json(snapshot: &ContextSnapshot) -> JsonValue {
    JsonValue::Object(vec![
        (
            "snapshot_id".to_string(),
            JsonValue::String(snapshot.snapshot_id.clone()),
        ),
        (
            "session_id".to_string(),
            JsonValue::String(snapshot.session_id.clone()),
        ),
        (
            "basis_entry_id".to_string(),
            JsonValue::String(snapshot.basis_entry_id.clone()),
        ),
        (
            "token_estimate".to_string(),
            JsonValue::Number(JsonNumber::Integer(snapshot.token_estimate as i64)),
        ),
        (
            "included_entry_ids".to_string(),
            string_array_json(&snapshot.included_entry_ids),
        ),
        (
            "summary_refs".to_string(),
            string_array_json(&snapshot.summary_refs),
        ),
        (
            "memory_refs".to_string(),
            string_array_json(&snapshot.memory_refs),
        ),
        (
            "artifact_refs".to_string(),
            string_array_json(&snapshot.artifact_refs),
        ),
    ])
}

fn decode_session_record(
    body: &[u8],
    latest_revision: Option<String>,
) -> Result<ContextSession, StorageError> {
    let value = decode_json(body)?;
    let object = expect_object("session", &value)?;
    Ok(ContextSession {
        session_id: required_string(object, "session_id")?,
        owner_id: required_string(object, "owner_id")?,
        title: required_string(object, "title")?,
        status: SessionStatus::from_str(&required_string(object, "status")?)?,
        latest_revision: latest_revision.or(optional_string(object, "latest_revision")?),
        head_pointer: optional_string(object, "head_pointer")?,
    })
}

fn decode_entry_record(body: &[u8]) -> Result<ContextEntry, StorageError> {
    let value = decode_json(body)?;
    let object = expect_object("entry", &value)?;
    Ok(ContextEntry {
        entry_id: required_string(object, "entry_id")?,
        session_id: required_string(object, "session_id")?,
        kind: ContextEntryKind::from_str(&required_string(object, "kind")?)?,
        timestamp: required_u64(object, "timestamp")?,
        metadata: required_value(object, "metadata")?.clone(),
        body: required_value(object, "body")?.clone(),
    })
}

fn decode_snapshot_record(body: &[u8]) -> Result<ContextSnapshot, StorageError> {
    let value = decode_json(body)?;
    let object = expect_object("snapshot", &value)?;
    Ok(ContextSnapshot {
        snapshot_id: required_string(object, "snapshot_id")?,
        session_id: required_string(object, "session_id")?,
        basis_entry_id: required_string(object, "basis_entry_id")?,
        token_estimate: required_u64(object, "token_estimate")?,
        included_entry_ids: required_string_array(object, "included_entry_ids")?,
        summary_refs: required_string_array(object, "summary_refs")?,
        memory_refs: required_string_array(object, "memory_refs")?,
        artifact_refs: required_string_array(object, "artifact_refs")?,
    })
}

fn encode_json(value: &JsonValue) -> Result<Vec<u8>, StorageError> {
    let text = serialize(value).map_err(|error| validation("json", error.message))?;
    Ok(text.into_bytes())
}

fn decode_json(bytes: &[u8]) -> Result<JsonValue, StorageError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| validation("body", "context record must be UTF-8"))?;
    parse_json(text).map_err(|error| validation("body", error.message))
}

fn expect_object<'a>(
    label: &str,
    value: &'a JsonValue,
) -> Result<&'a Vec<(String, JsonValue)>, StorageError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(validation(
            label,
            format!("{label} record must decode to a JSON object"),
        )),
    }
}

fn required_value<'a>(
    object: &'a [(String, JsonValue)],
    field: &str,
) -> Result<&'a JsonValue, StorageError> {
    object
        .iter()
        .find(|(name, _)| name == field)
        .map(|(_, value)| value)
        .ok_or_else(|| validation(field, "required field was missing"))
}

fn required_string(object: &[(String, JsonValue)], field: &str) -> Result<String, StorageError> {
    match required_value(object, field)? {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(validation(field, "field must be a JSON string")),
    }
}

fn optional_string(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Option<String>, StorageError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(value) => Ok(Some(value.clone())),
        _ => Err(validation(field, "field must be null or a JSON string")),
    }
}

fn required_u64(object: &[(String, JsonValue)], field: &str) -> Result<u64, StorageError> {
    match required_value(object, field)? {
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => Ok(*value as u64),
        _ => Err(validation(
            field,
            "field must be a non-negative integer JSON number",
        )),
    }
}

fn required_string_array(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Vec<String>, StorageError> {
    match required_value(object, field)? {
        JsonValue::Array(values) => values
            .iter()
            .map(|value| match value {
                JsonValue::String(string) => Ok(string.clone()),
                _ => Err(validation(field, "array elements must all be strings")),
            })
            .collect(),
        _ => Err(validation(field, "field must be an array")),
    }
}

fn validate_id(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(validation(field, "must not be empty"));
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
    {
        return Err(validation(
            field,
            "must use only ASCII letters, digits, dots, underscores, or hyphens",
        ));
    }
    Ok(())
}

fn validate_id_list(field: &str, values: &[String]) -> Result<(), StorageError> {
    for value in values {
        validate_id(field, value)?;
    }
    Ok(())
}

fn validate_title(value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(validation("title", "must not be empty"));
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(validation("title", "must not contain newlines"));
    }
    Ok(())
}

fn validate_json_object(field: &str, value: &JsonValue) -> Result<(), StorageError> {
    if matches!(value, JsonValue::Object(_)) {
        Ok(())
    } else {
        Err(validation(field, "must be a JSON object"))
    }
}

fn validate_session_list_options(options: &SessionListOptions) -> Result<(), StorageError> {
    if let Some(owner_id) = options.owner_id.as_deref() {
        validate_id("owner_id", owner_id)?;
    }
    if options.limit == Some(0) {
        return Err(validation("limit", "must be greater than zero"));
    }
    Ok(())
}

fn validate_snapshot_list_options(options: &SnapshotListOptions) -> Result<(), StorageError> {
    if let Some(basis_entry_id) = options.basis_entry_id.as_deref() {
        validate_id("basis_entry_id", basis_entry_id)?;
    }
    validate_id_list("summary_refs", &options.summary_refs)?;
    validate_id_list("memory_refs", &options.memory_refs)?;
    validate_id_list("artifact_refs", &options.artifact_refs)?;
    if options.limit == Some(0) {
        return Err(validation("limit", "must be greater than zero"));
    }
    Ok(())
}

fn validate_fetch_entries_options(options: &FetchEntriesOptions) -> Result<(), StorageError> {
    if let Some(after_entry_id) = options.after_entry_id.as_deref() {
        validate_id("after_entry_id", after_entry_id)?;
    }
    if options.limit == Some(0) {
        return Err(validation("limit", "must be greater than zero"));
    }
    if let (Some(since), Some(until)) = (options.since_timestamp, options.until_timestamp) {
        if since > until {
            return Err(validation(
                "timestamp_range",
                "since_timestamp must be less than or equal to until_timestamp",
            ));
        }
    }
    Ok(())
}

fn session_matches_list_options(session: &ContextSession, options: &SessionListOptions) -> bool {
    if let Some(owner_id) = options.owner_id.as_deref() {
        if session.owner_id != owner_id {
            return false;
        }
    }
    if !options.statuses.is_empty() && !options.statuses.contains(&session.status) {
        return false;
    }
    true
}

fn snapshot_matches_list_options(
    snapshot: &ContextSnapshot,
    options: &SnapshotListOptions,
) -> bool {
    if let Some(basis_entry_id) = options.basis_entry_id.as_deref() {
        if snapshot.basis_entry_id != basis_entry_id {
            return false;
        }
    }
    if !options
        .summary_refs
        .iter()
        .all(|required| snapshot.summary_refs.contains(required))
    {
        return false;
    }
    if !options
        .memory_refs
        .iter()
        .all(|required| snapshot.memory_refs.contains(required))
    {
        return false;
    }
    if !options
        .artifact_refs
        .iter()
        .all(|required| snapshot.artifact_refs.contains(required))
    {
        return false;
    }
    true
}

fn sort_sessions(sessions: &mut [ContextSession], sort: SessionListSort) {
    match sort {
        SessionListSort::SessionId => {
            sessions.sort_by(|left, right| left.session_id.cmp(&right.session_id))
        }
        SessionListSort::OwnerThenTitle => sessions.sort_by(|left, right| {
            left.owner_id
                .cmp(&right.owner_id)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.session_id.cmp(&right.session_id))
        }),
        SessionListSort::StatusThenTitle => sessions.sort_by(|left, right| {
            session_status_rank(left.status)
                .cmp(&session_status_rank(right.status))
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.session_id.cmp(&right.session_id))
        }),
        SessionListSort::Title => sessions.sort_by(|left, right| {
            left.title
                .cmp(&right.title)
                .then_with(|| left.session_id.cmp(&right.session_id))
        }),
    }
}

fn sort_snapshots(snapshots: &mut [ContextSnapshot], sort: SnapshotListSort) {
    match sort {
        SnapshotListSort::SnapshotId => {
            snapshots.sort_by(|left, right| left.snapshot_id.cmp(&right.snapshot_id))
        }
        SnapshotListSort::BasisEntryId => snapshots.sort_by(|left, right| {
            left.basis_entry_id
                .cmp(&right.basis_entry_id)
                .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
        }),
        SnapshotListSort::TokenEstimateAsc => snapshots.sort_by(|left, right| {
            left.token_estimate
                .cmp(&right.token_estimate)
                .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
        }),
        SnapshotListSort::TokenEstimateDesc => snapshots.sort_by(|left, right| {
            right
                .token_estimate
                .cmp(&left.token_estimate)
                .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
        }),
    }
}

fn session_status_rank(status: SessionStatus) -> u8 {
    match status {
        SessionStatus::Active => 0,
        SessionStatus::Paused => 1,
        SessionStatus::Archived => 2,
    }
}

fn optional_string_json(value: Option<&str>) -> JsonValue {
    value
        .map(|value| JsonValue::String(value.to_string()))
        .unwrap_or(JsonValue::Null)
}

fn string_array_json(values: &[String]) -> JsonValue {
    JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::String(value.clone()))
            .collect(),
    )
}

fn window_entries(
    entries: Vec<ContextEntry>,
    options: FetchEntriesOptions,
) -> Result<Vec<ContextEntry>, StorageError> {
    let start = match options.after_entry_id.as_deref() {
        Some(after_entry_id) => entries
            .iter()
            .position(|entry| entry.entry_id == after_entry_id)
            .map(|index| index + 1)
            .ok_or_else(|| {
                validation(
                    "after_entry_id",
                    format!("entry '{after_entry_id}' was not found"),
                )
            })?,
        None => 0,
    };
    let limit = options.limit.unwrap_or(usize::MAX);

    Ok(entries
        .into_iter()
        .skip(start)
        .filter(|entry| entry_matches_fetch_options(entry, &options))
        .take(limit)
        .collect())
}

fn entry_matches_fetch_options(entry: &ContextEntry, options: &FetchEntriesOptions) -> bool {
    if !options.kinds.is_empty() && !options.kinds.contains(&entry.kind) {
        return false;
    }
    if let Some(since) = options.since_timestamp {
        if entry.timestamp < since {
            return false;
        }
    }
    if let Some(until) = options.until_timestamp {
        if entry.timestamp > until {
            return false;
        }
    }
    true
}

fn metadata_has_fields(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Object(fields) if !fields.is_empty())
}

fn validation(field: &str, message: impl Into<String>) -> StorageError {
    StorageError::Validation {
        field: field.to_string(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage_core::InMemoryStorageBackend;

    fn object(entries: &[(&str, JsonValue)]) -> JsonValue {
        JsonValue::Object(
            entries
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.clone()))
                .collect(),
        )
    }

    #[test]
    fn session_create_append_and_list_round_trip() {
        let store = ContextStore::new(InMemoryStorageBackend::new());

        let session = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        assert_eq!(session.status, SessionStatus::Active);

        let _entry = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-1".to_string(),
                    kind: ContextEntryKind::User,
                    timestamp: Some(10),
                    metadata: object(&[("source", JsonValue::String("ui".to_string()))]),
                    body: JsonValue::String("Need a roadmap".to_string()),
                },
            )
            .unwrap();

        let entries = store.fetch_ordered_entries("demo").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_id, "entry-1");
        assert_eq!(entries[0].kind, ContextEntryKind::User);
    }

    #[test]
    fn list_sessions_supports_owner_status_sort_and_limit() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        for (session_id, owner_id, title) in [
            ("alpha", "chief", "Alpha planning"),
            ("beta", "chief", "Beta archive"),
            ("other", "guest", "Guest notes"),
        ] {
            let _ = store
                .create_session(CreateSessionInput {
                    session_id: session_id.to_string(),
                    owner_id: owner_id.to_string(),
                    title: title.to_string(),
                })
                .unwrap();
        }
        let _ = store.archive_session("beta").unwrap();

        let active = store
            .list_sessions(
                SessionListOptions::new()
                    .for_owner("chief")
                    .with_status(SessionStatus::Active)
                    .sorted_by(SessionListSort::Title)
                    .limited_to(1),
            )
            .unwrap();
        assert_eq!(
            active
                .iter()
                .map(|session| session.session_id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha"]
        );

        let all_chief = store
            .list_sessions(
                SessionListOptions::new()
                    .for_owner("chief")
                    .with_status(SessionStatus::Archived)
                    .with_status(SessionStatus::Active)
                    .sorted_by(SessionListSort::StatusThenTitle),
            )
            .unwrap();
        assert_eq!(
            all_chief
                .iter()
                .map(|session| (session.session_id.as_str(), session.status))
                .collect::<Vec<_>>(),
            vec![
                ("alpha", SessionStatus::Active),
                ("beta", SessionStatus::Archived)
            ]
        );
    }

    #[test]
    fn session_catalog_summary_counts_status_and_checkpoint_coverage() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        for (session_id, owner_id, title) in [
            ("alpha", "chief", "Alpha planning"),
            ("beta", "chief", "Beta archive"),
            ("gamma", "chief", "Gamma notes"),
            ("guest", "guest", "Guest notes"),
        ] {
            let _ = store
                .create_session(CreateSessionInput {
                    session_id: session_id.to_string(),
                    owner_id: owner_id.to_string(),
                    title: title.to_string(),
                })
                .unwrap();
        }
        let _ = store.archive_session("beta").unwrap();
        let _ = store
            .append_entry(
                "alpha",
                AppendEntryInput {
                    entry_id: "summary-1".to_string(),
                    kind: ContextEntryKind::Summary,
                    timestamp: Some(10),
                    metadata: object(&[]),
                    body: JsonValue::String("summary".to_string()),
                },
            )
            .unwrap();
        let _ = store
            .create_snapshot(
                "alpha",
                CreateSnapshotInput {
                    snapshot_id: "snap-1".to_string(),
                    basis_entry_id: "summary-1".to_string(),
                    token_estimate: 12,
                    included_entry_ids: vec!["summary-1".to_string()],
                    summary_refs: vec!["summary-1".to_string()],
                    memory_refs: vec![],
                    artifact_refs: vec![],
                },
            )
            .unwrap();

        let summary = store
            .session_catalog_summary(SessionListOptions::new().for_owner("chief"))
            .unwrap();
        assert_eq!(
            summary,
            ContextSessionCatalogSummary {
                total_sessions: 3,
                active_sessions: 2,
                paused_sessions: 0,
                archived_sessions: 1,
                sessions_with_snapshots: 1,
                sessions_without_snapshots: 2,
            }
        );
        assert_eq!(summary.open_sessions(), 2);
        assert!(summary.has_open_sessions());
        assert!(summary.has_uncheckpointed_sessions());

        let archived = store
            .session_catalog_summary(
                SessionListOptions::new()
                    .for_owner("chief")
                    .with_status(SessionStatus::Archived),
            )
            .unwrap();
        assert_eq!(archived.total_sessions, 1);
        assert_eq!(archived.archived_sessions, 1);
        assert!(!archived.has_open_sessions());
    }

    #[test]
    fn fetch_entries_supports_after_cursor_and_limit() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        for (index, entry_id) in ["entry-1", "entry-2", "entry-3"].iter().enumerate() {
            let _ = store
                .append_entry(
                    "demo",
                    AppendEntryInput {
                        entry_id: (*entry_id).to_string(),
                        kind: ContextEntryKind::Note,
                        timestamp: Some((index as u64 + 1) * 10),
                        metadata: object(&[]),
                        body: JsonValue::String((*entry_id).to_string()),
                    },
                )
                .unwrap();
        }

        let entries = store
            .fetch_entries(
                "demo",
                FetchEntriesOptions::new()
                    .after_entry("entry-1")
                    .limited_to(1),
            )
            .unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.entry_id.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-2"]
        );

        let entries = store
            .fetch_entries("demo", FetchEntriesOptions::new().after_entry("entry-2"))
            .unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.entry_id.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-3"]
        );
    }

    #[test]
    fn fetch_entries_supports_kind_and_timestamp_filters() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        for (entry_id, kind, timestamp) in [
            ("entry-1", ContextEntryKind::User, 10),
            ("entry-2", ContextEntryKind::ToolCall, 20),
            ("entry-3", ContextEntryKind::ToolResult, 30),
            ("entry-4", ContextEntryKind::Note, 40),
            ("entry-5", ContextEntryKind::Assistant, 50),
        ] {
            let _ = store
                .append_entry(
                    "demo",
                    AppendEntryInput {
                        entry_id: entry_id.to_string(),
                        kind,
                        timestamp: Some(timestamp),
                        metadata: object(&[]),
                        body: JsonValue::String(entry_id.to_string()),
                    },
                )
                .unwrap();
        }

        let tool_entries = store
            .fetch_entries(
                "demo",
                FetchEntriesOptions::new()
                    .with_kind(ContextEntryKind::ToolCall)
                    .with_kind(ContextEntryKind::ToolResult)
                    .since(15)
                    .until(35)
                    .limited_to(4),
            )
            .unwrap();
        assert_eq!(
            tool_entries
                .iter()
                .map(|entry| (entry.entry_id.as_str(), entry.kind))
                .collect::<Vec<_>>(),
            vec![
                ("entry-2", ContextEntryKind::ToolCall),
                ("entry-3", ContextEntryKind::ToolResult)
            ]
        );

        let notes_after_cursor = store
            .fetch_entries(
                "demo",
                FetchEntriesOptions::new()
                    .after_entry("entry-1")
                    .with_kind(ContextEntryKind::Note)
                    .since(20),
            )
            .unwrap();
        assert_eq!(
            notes_after_cursor
                .iter()
                .map(|entry| entry.entry_id.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-4"]
        );
    }

    #[test]
    fn fetch_entry_summaries_project_metadata_without_bodies() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        let _ = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-1".to_string(),
                    kind: ContextEntryKind::User,
                    timestamp: Some(10),
                    metadata: object(&[("channel", JsonValue::String("ui".to_string()))]),
                    body: JsonValue::String("full user message".to_string()),
                },
            )
            .unwrap();
        let _ = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-2".to_string(),
                    kind: ContextEntryKind::ToolResult,
                    timestamp: Some(20),
                    metadata: object(&[("tool", JsonValue::String("smart_home".to_string()))]),
                    body: object(&[("secret_result", JsonValue::String("body".to_string()))]),
                },
            )
            .unwrap();

        let summaries = store
            .fetch_entry_summaries(
                "demo",
                FetchEntriesOptions::new()
                    .after_entry("entry-1")
                    .with_kind(ContextEntryKind::ToolResult)
                    .limited_to(1),
            )
            .unwrap();

        assert_eq!(
            summaries,
            vec![ContextEntrySummary {
                entry_id: "entry-2".to_string(),
                session_id: "demo".to_string(),
                kind: ContextEntryKind::ToolResult,
                timestamp: 20,
                metadata: object(&[("tool", JsonValue::String("smart_home".to_string()))]),
            }]
        );
    }

    #[test]
    fn transcript_summary_counts_entry_kinds_and_time_span() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        for (entry_id, kind, timestamp, metadata) in [
            (
                "entry-1",
                ContextEntryKind::User,
                30,
                object(&[("source", JsonValue::String("ui".to_string()))]),
            ),
            ("entry-2", ContextEntryKind::Assistant, 10, object(&[])),
            ("entry-3", ContextEntryKind::ToolCall, 20, object(&[])),
            (
                "entry-4",
                ContextEntryKind::ToolResult,
                40,
                object(&[("tool", JsonValue::String("search".to_string()))]),
            ),
            ("entry-5", ContextEntryKind::Summary, 50, object(&[])),
            ("entry-6", ContextEntryKind::Note, 60, object(&[])),
            ("entry-7", ContextEntryKind::AttachmentRef, 70, object(&[])),
        ] {
            let _ = store
                .append_entry(
                    "demo",
                    AppendEntryInput {
                        entry_id: entry_id.to_string(),
                        kind,
                        timestamp: Some(timestamp),
                        metadata,
                        body: JsonValue::String(entry_id.to_string()),
                    },
                )
                .unwrap();
        }

        let summary = store
            .transcript_summary("demo", FetchEntriesOptions::new())
            .unwrap();
        assert_eq!(
            summary,
            ContextTranscriptSummary {
                total_entries: 7,
                user_entries: 1,
                assistant_entries: 1,
                tool_call_entries: 1,
                tool_result_entries: 1,
                summary_entries: 1,
                note_entries: 1,
                attachment_ref_entries: 1,
                entries_with_metadata: 2,
                first_timestamp: Some(10),
                latest_timestamp: Some(70),
            }
        );
        assert_eq!(summary.conversational_entries(), 2);
        assert_eq!(summary.tool_interaction_entries(), 2);
        assert!(summary.has_compaction_material());
        assert!(!summary.is_empty());

        let tool_window = store
            .transcript_summary(
                "demo",
                FetchEntriesOptions::new()
                    .with_kind(ContextEntryKind::ToolCall)
                    .with_kind(ContextEntryKind::ToolResult),
            )
            .unwrap();
        assert_eq!(tool_window.total_entries, 2);
        assert_eq!(tool_window.tool_interaction_entries(), 2);
        assert_eq!(tool_window.first_timestamp, Some(20));
        assert_eq!(tool_window.latest_timestamp, Some(40));

        let empty = ContextTranscriptSummary::from_entry_summaries([]);
        assert!(empty.is_empty());
        assert_eq!(empty.first_timestamp, None);
        assert_eq!(empty.latest_timestamp, None);
    }

    #[test]
    fn fetch_entries_rejects_bad_windows() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();
        let _ = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-1".to_string(),
                    kind: ContextEntryKind::Note,
                    timestamp: Some(10),
                    metadata: object(&[]),
                    body: JsonValue::String("entry-1".to_string()),
                },
            )
            .unwrap();

        let missing = store
            .fetch_entries("demo", FetchEntriesOptions::new().after_entry("entry-2"))
            .unwrap_err();
        let zero_limit = store
            .fetch_entries("demo", FetchEntriesOptions::new().limited_to(0))
            .unwrap_err();
        let bad_range = store
            .fetch_entries("demo", FetchEntriesOptions::new().since(20).until(10))
            .unwrap_err();

        assert!(matches!(missing, StorageError::Validation { .. }));
        assert!(matches!(zero_limit, StorageError::Validation { .. }));
        assert!(matches!(bad_range, StorageError::Validation { .. }));
    }

    #[test]
    fn snapshot_updates_head_pointer() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();
        let _ = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-1".to_string(),
                    kind: ContextEntryKind::Summary,
                    timestamp: Some(10),
                    metadata: object(&[]),
                    body: JsonValue::String("summary".to_string()),
                },
            )
            .unwrap();

        let snapshot = store
            .create_snapshot(
                "demo",
                CreateSnapshotInput {
                    snapshot_id: "snap-1".to_string(),
                    basis_entry_id: "entry-1".to_string(),
                    token_estimate: 42,
                    included_entry_ids: vec!["entry-1".to_string()],
                    summary_refs: vec!["entry-1".to_string()],
                    memory_refs: vec![],
                    artifact_refs: vec![],
                },
            )
            .unwrap();

        assert_eq!(
            store.fetch_latest_snapshot("demo").unwrap(),
            Some(snapshot.clone())
        );

        let session = store.open_session("demo").unwrap().unwrap();
        assert_eq!(session.head_pointer, Some("snap-1".to_string()));
    }

    #[test]
    fn list_snapshots_supports_refs_sort_and_limit() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        for (index, entry_id) in ["entry-1", "entry-2", "entry-3"].iter().enumerate() {
            let _ = store
                .append_entry(
                    "demo",
                    AppendEntryInput {
                        entry_id: (*entry_id).to_string(),
                        kind: ContextEntryKind::Summary,
                        timestamp: Some((index as u64 + 1) * 10),
                        metadata: object(&[]),
                        body: JsonValue::String((*entry_id).to_string()),
                    },
                )
                .unwrap();
        }

        let _ = store
            .create_snapshot(
                "demo",
                CreateSnapshotInput {
                    snapshot_id: "snap-small".to_string(),
                    basis_entry_id: "entry-1".to_string(),
                    token_estimate: 10,
                    included_entry_ids: vec!["entry-1".to_string()],
                    summary_refs: vec!["summary-1".to_string()],
                    memory_refs: vec!["memory-shared".to_string()],
                    artifact_refs: vec![],
                },
            )
            .unwrap();
        let _ = store
            .create_snapshot(
                "demo",
                CreateSnapshotInput {
                    snapshot_id: "snap-large".to_string(),
                    basis_entry_id: "entry-3".to_string(),
                    token_estimate: 30,
                    included_entry_ids: vec![
                        "entry-1".to_string(),
                        "entry-2".to_string(),
                        "entry-3".to_string(),
                    ],
                    summary_refs: vec!["summary-2".to_string()],
                    memory_refs: vec!["memory-shared".to_string(), "memory-new".to_string()],
                    artifact_refs: vec!["artifact-plan".to_string()],
                },
            )
            .unwrap();

        let shared_memory = store
            .list_snapshots(
                "demo",
                SnapshotListOptions::new()
                    .with_memory_ref("memory-shared")
                    .sorted_by(SnapshotListSort::TokenEstimateDesc)
                    .limited_to(1),
            )
            .unwrap();
        assert_eq!(
            shared_memory
                .iter()
                .map(|snapshot| snapshot.snapshot_id.as_str())
                .collect::<Vec<_>>(),
            vec!["snap-large"]
        );

        let artifact_snapshots = store
            .list_snapshots(
                "demo",
                SnapshotListOptions::new()
                    .with_summary_ref("summary-2")
                    .with_artifact_ref("artifact-plan"),
            )
            .unwrap();
        assert_eq!(
            artifact_snapshots
                .iter()
                .map(|snapshot| snapshot.basis_entry_id.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-3"]
        );

        let summary = store
            .snapshot_summary(
                "demo",
                SnapshotListOptions::new()
                    .with_memory_ref("memory-shared")
                    .sorted_by(SnapshotListSort::TokenEstimateAsc),
            )
            .unwrap();
        assert_eq!(
            summary,
            ContextSnapshotSummary {
                total_snapshots: 2,
                included_entry_refs: 4,
                summary_refs: 2,
                memory_refs: 3,
                artifact_refs: 1,
                snapshots_with_memory_refs: 2,
                snapshots_with_artifact_refs: 1,
                total_token_estimate: 40,
                min_token_estimate: Some(10),
                max_token_estimate: Some(30),
            }
        );
        assert!(summary.has_memory_refs());
        assert!(summary.has_artifact_refs());
        assert!(!summary.is_empty());

        let empty = ContextSnapshotSummary::from_snapshots([]);
        assert!(empty.is_empty());
        assert_eq!(empty.min_token_estimate, None);
        assert_eq!(empty.max_token_estimate, None);
    }

    #[test]
    fn session_summary_combines_header_transcript_and_checkpoint_state() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        for (entry_id, kind, timestamp) in [
            ("entry-1", ContextEntryKind::User, 10),
            ("entry-2", ContextEntryKind::Assistant, 20),
            ("entry-3", ContextEntryKind::ToolResult, 30),
        ] {
            let _ = store
                .append_entry(
                    "demo",
                    AppendEntryInput {
                        entry_id: entry_id.to_string(),
                        kind,
                        timestamp: Some(timestamp),
                        metadata: object(&[]),
                        body: JsonValue::String(entry_id.to_string()),
                    },
                )
                .unwrap();
        }

        let _ = store
            .create_snapshot(
                "demo",
                CreateSnapshotInput {
                    snapshot_id: "snap-1".to_string(),
                    basis_entry_id: "entry-2".to_string(),
                    token_estimate: 25,
                    included_entry_ids: vec!["entry-1".to_string(), "entry-2".to_string()],
                    summary_refs: vec!["summary-1".to_string()],
                    memory_refs: vec!["memory-1".to_string()],
                    artifact_refs: vec!["artifact-1".to_string()],
                },
            )
            .unwrap();

        let summary = store.session_summary("demo").unwrap().unwrap();
        assert_eq!(summary.status, SessionStatus::Active);
        assert!(summary.has_head_pointer);
        assert!(summary.head_snapshot_found);
        assert_eq!(summary.transcript.total_entries, 3);
        assert_eq!(summary.transcript.conversational_entries(), 2);
        assert_eq!(summary.transcript.tool_interaction_entries(), 1);
        assert_eq!(summary.snapshots.total_snapshots, 1);
        assert_eq!(summary.snapshots.memory_refs, 1);
        assert_eq!(summary.snapshots.artifact_refs, 1);
        assert_eq!(summary.uncheckpointed_entries, 1);
        assert!(!summary.is_archived());
        assert!(!summary.has_missing_head_snapshot());
        assert!(summary.has_uncheckpointed_entries());
        assert!(summary.has_tool_activity());
        assert!(summary.has_external_refs());
        assert_eq!(store.session_summary("missing").unwrap(), None);

        let broken = ContextSessionSummary::from_parts(
            &ContextSession {
                session_id: "broken".to_string(),
                owner_id: "chief".to_string(),
                title: "Broken".to_string(),
                status: SessionStatus::Archived,
                latest_revision: None,
                head_pointer: Some("missing-snapshot".to_string()),
            },
            &[],
            &[],
        );
        assert!(broken.is_archived());
        assert!(broken.has_missing_head_snapshot());
        assert!(!broken.has_uncheckpointed_entries());
    }

    #[test]
    fn inventory_summary_rolls_up_selected_session_material() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "alpha".to_string(),
                owner_id: "chief".to_string(),
                title: "Active Planning".to_string(),
            })
            .unwrap();
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "beta".to_string(),
                owner_id: "chief".to_string(),
                title: "Archived Notes".to_string(),
            })
            .unwrap();
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "gamma".to_string(),
                owner_id: "other".to_string(),
                title: "Other Owner".to_string(),
            })
            .unwrap();

        for (entry_id, kind, timestamp) in [
            ("entry-1", ContextEntryKind::User, 10),
            ("entry-2", ContextEntryKind::ToolCall, 20),
            ("entry-3", ContextEntryKind::ToolResult, 30),
        ] {
            let _ = store
                .append_entry(
                    "alpha",
                    AppendEntryInput {
                        entry_id: entry_id.to_string(),
                        kind,
                        timestamp: Some(timestamp),
                        metadata: object(&[("kind", JsonValue::String(entry_id.to_string()))]),
                        body: JsonValue::String(entry_id.to_string()),
                    },
                )
                .unwrap();
        }
        let _ = store
            .append_entry(
                "beta",
                AppendEntryInput {
                    entry_id: "entry-4".to_string(),
                    kind: ContextEntryKind::Note,
                    timestamp: Some(40),
                    metadata: object(&[]),
                    body: JsonValue::String("note".to_string()),
                },
            )
            .unwrap();
        let _ = store
            .append_entry(
                "gamma",
                AppendEntryInput {
                    entry_id: "entry-5".to_string(),
                    kind: ContextEntryKind::Assistant,
                    timestamp: Some(50),
                    metadata: object(&[]),
                    body: JsonValue::String("other".to_string()),
                },
            )
            .unwrap();
        let _ = store
            .create_snapshot(
                "alpha",
                CreateSnapshotInput {
                    snapshot_id: "snap-1".to_string(),
                    basis_entry_id: "entry-2".to_string(),
                    token_estimate: 25,
                    included_entry_ids: vec!["entry-1".to_string(), "entry-2".to_string()],
                    summary_refs: vec!["summary-1".to_string()],
                    memory_refs: vec!["memory-1".to_string()],
                    artifact_refs: vec!["artifact-1".to_string()],
                },
            )
            .unwrap();
        let _ = store.archive_session("beta").unwrap();

        let summary = store
            .inventory_summary(SessionListOptions::new().for_owner("chief"))
            .unwrap();

        assert_eq!(
            summary.sessions,
            ContextSessionCatalogSummary {
                total_sessions: 2,
                active_sessions: 1,
                paused_sessions: 0,
                archived_sessions: 1,
                sessions_with_snapshots: 1,
                sessions_without_snapshots: 1,
            }
        );
        assert_eq!(
            summary.transcripts,
            ContextTranscriptSummary {
                total_entries: 4,
                user_entries: 1,
                assistant_entries: 0,
                tool_call_entries: 1,
                tool_result_entries: 1,
                summary_entries: 0,
                note_entries: 1,
                attachment_ref_entries: 0,
                entries_with_metadata: 3,
                first_timestamp: Some(10),
                latest_timestamp: Some(40),
            }
        );
        assert_eq!(
            summary.snapshots,
            ContextSnapshotSummary {
                total_snapshots: 1,
                included_entry_refs: 2,
                summary_refs: 1,
                memory_refs: 1,
                artifact_refs: 1,
                snapshots_with_memory_refs: 1,
                snapshots_with_artifact_refs: 1,
                total_token_estimate: 25,
                min_token_estimate: Some(25),
                max_token_estimate: Some(25),
            }
        );
        assert_eq!(summary.sessions_with_uncheckpointed_entries, 2);
        assert_eq!(summary.sessions_with_missing_head_snapshots, 0);
        assert_eq!(summary.sessions_with_tool_activity, 1);
        assert_eq!(summary.sessions_with_external_refs, 1);
        assert!(!summary.is_empty());
        assert!(summary.has_context_material());
        assert!(summary.has_compaction_attention_items());
        assert!(summary.has_tool_activity());
        assert!(summary.has_external_refs());

        let empty = store
            .inventory_summary(SessionListOptions::new().for_owner("missing"))
            .unwrap();
        assert_eq!(empty, ContextStoreInventorySummary::empty());
        assert!(empty.is_empty());
        assert!(!empty.has_context_material());
        assert!(!empty.has_compaction_attention_items());
    }

    #[test]
    fn list_snapshots_rejects_bad_filters() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        let bad_ref = store
            .list_snapshots(
                "demo",
                SnapshotListOptions::new().with_summary_ref("bad ref"),
            )
            .unwrap_err();
        let zero_limit = store
            .list_snapshots("demo", SnapshotListOptions::new().limited_to(0))
            .unwrap_err();

        assert!(matches!(bad_ref, StorageError::Validation { .. }));
        assert!(matches!(zero_limit, StorageError::Validation { .. }));
    }

    #[test]
    fn archived_session_rejects_new_entries() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        let archived = store.archive_session("demo").unwrap();
        assert_eq!(archived.status, SessionStatus::Archived);

        let error = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-2".to_string(),
                    kind: ContextEntryKind::User,
                    timestamp: Some(20),
                    metadata: object(&[]),
                    body: JsonValue::String("still here?".to_string()),
                },
            )
            .unwrap_err();

        assert!(matches!(error, StorageError::Validation { .. }));
    }

    #[test]
    fn compaction_handles_missing_and_present_basis_entries() {
        let store = ContextStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_session(CreateSessionInput {
                session_id: "demo".to_string(),
                owner_id: "chief".to_string(),
                title: "Planning".to_string(),
            })
            .unwrap();

        assert_eq!(store.open_session("missing").unwrap(), None);
        assert_eq!(store.fetch_latest_snapshot("demo").unwrap(), None);

        let _ = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-1".to_string(),
                    kind: ContextEntryKind::Assistant,
                    timestamp: Some(10),
                    metadata: object(&[]),
                    body: JsonValue::String("first".to_string()),
                },
            )
            .unwrap();
        let _ = store
            .append_entry(
                "demo",
                AppendEntryInput {
                    entry_id: "entry-2".to_string(),
                    kind: ContextEntryKind::ToolResult,
                    timestamp: Some(20),
                    metadata: object(&[]),
                    body: JsonValue::String("second".to_string()),
                },
            )
            .unwrap();

        let error = store
            .compact_before_entry("demo", "missing-entry", "entry-1")
            .unwrap_err();
        assert!(matches!(error, StorageError::Validation { .. }));

        let snapshot = store
            .compact_before_entry("demo", "entry-2", "entry-1")
            .unwrap();
        assert_eq!(snapshot.included_entry_ids, vec!["entry-1", "entry-2"]);
    }

    #[test]
    fn helper_validations_and_decoders_reject_invalid_shapes() {
        assert_eq!(
            SessionStatus::from_str("paused").unwrap(),
            SessionStatus::Paused
        );
        assert_eq!(
            ContextEntryKind::from_str("attachment_ref").unwrap(),
            ContextEntryKind::AttachmentRef
        );
        assert!(SessionStatus::from_str("unknown").is_err());
        assert!(ContextEntryKind::from_str("unknown").is_err());
        assert!(validate_title("bad\ntitle").is_err());
        assert!(validate_json_object("metadata", &JsonValue::String("bad".to_string())).is_err());
        assert!(
            validate_session_list_options(&SessionListOptions::new().for_owner("bad owner"))
                .is_err()
        );
        assert!(validate_session_list_options(&SessionListOptions::new().limited_to(0)).is_err());
        assert!(decode_json(&[0xff, 0xfe]).is_err());
        assert!(expect_object("entry", &JsonValue::String("bad".to_string())).is_err());
    }
}
