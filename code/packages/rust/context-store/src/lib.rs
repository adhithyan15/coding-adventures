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
        validate_id("session_id", session_id)?;
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

        page.records
            .iter()
            .map(|record| decode_entry_record(&record.body))
            .collect()
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
        assert!(decode_json(&[0xff, 0xfe]).is_err());
        assert!(expect_object("entry", &JsonValue::String("bad".to_string())).is_err());
    }
}
