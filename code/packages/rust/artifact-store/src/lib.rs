//! # artifact-store
//!
//! `artifact-store` manages durable outputs by separating:
//!
//! - a small JSON manifest (`Artifact`)
//! - one or more opaque binary/text revisions (`ArtifactRevision`)
//!
//! This mirrors how humans think about artifacts:
//!
//! ```text
//! "the plan"        --> artifact manifest
//! "version 3 plan"  --> artifact revision body
//! ```

use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::{parse as parse_json, JsonNumber, JsonValue};
use storage_core::{now_utc_ms, StorageBackend, StorageError, StorageListOptions, StoragePutInput};

const NAMESPACE: &str = "artifacts";

/// Lifecycle/retention state for an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactRetention {
    Temporary,
    Retained,
    Exported,
}

impl ArtifactRetention {
    fn as_str(self) -> &'static str {
        match self {
            ArtifactRetention::Temporary => "temporary",
            ArtifactRetention::Retained => "retained",
            ArtifactRetention::Exported => "exported",
        }
    }

    fn from_str(value: &str) -> Result<Self, StorageError> {
        match value {
            "temporary" => Ok(Self::Temporary),
            "retained" => Ok(Self::Retained),
            "exported" => Ok(Self::Exported),
            _ => Err(validation(
                "retention",
                format!("unsupported artifact retention '{value}'"),
            )),
        }
    }
}

/// Source information explaining who produced the artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactProvenance {
    pub session_id: Option<String>,
    pub tool_id: Option<String>,
    pub job_id: Option<String>,
    pub agent_id: Option<String>,
}

/// Artifact manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub artifact_id: String,
    pub collection: String,
    pub name: String,
    pub content_type: String,
    pub labels: Vec<String>,
    pub provenance: ArtifactProvenance,
    pub latest_revision: Option<String>,
    pub retention: ArtifactRetention,
}

/// One opaque artifact body revision.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRevision {
    pub revision_id: String,
    pub artifact_id: String,
    pub parent_revision_id: Option<String>,
    pub metadata: JsonValue,
    pub body: Vec<u8>,
    pub created_at: u64,
}

/// Metadata-only view of one revision for bounded history reads.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRevisionSummary {
    pub revision_id: String,
    pub artifact_id: String,
    pub parent_revision_id: Option<String>,
    pub metadata: JsonValue,
    pub created_at: u64,
    pub body_len: usize,
    pub content_hash: [u8; 32],
}

/// Input used when first creating an artifact manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateArtifactInput {
    pub artifact_id: String,
    pub collection: String,
    pub name: String,
    pub content_type: String,
    pub labels: Vec<String>,
    pub provenance: ArtifactProvenance,
}

/// Query options for listing artifact manifests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactListOptions {
    pub collection: Option<String>,
    pub labels: Vec<String>,
    pub retention: Option<ArtifactRetention>,
    pub limit: Option<usize>,
}

impl ArtifactListOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_collection(mut self, collection: impl Into<String>) -> Self {
        self.collection = Some(collection.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    pub fn with_labels<I, S>(mut self, labels: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.labels.extend(labels.into_iter().map(Into::into));
        self
    }

    pub fn with_retention(mut self, retention: ArtifactRetention) -> Self {
        self.retention = Some(retention);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Query options for listing revision history without returning opaque bodies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactRevisionListOptions {
    pub after_revision_id: Option<String>,
    pub latest_first: bool,
    pub limit: Option<usize>,
}

impl ArtifactRevisionListOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after_revision(mut self, revision_id: impl Into<String>) -> Self {
        self.after_revision_id = Some(revision_id.into());
        self
    }

    pub fn latest_first(mut self) -> Self {
        self.latest_first = true;
        self
    }

    pub fn oldest_first(mut self) -> Self {
        self.latest_first = false;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Input used when appending one revision.
#[derive(Debug, Clone, PartialEq)]
pub struct AppendRevisionInput {
    pub revision_id: String,
    pub metadata: JsonValue,
    pub body: Vec<u8>,
}

/// Typed artifact store layered on top of `storage-core`.
pub struct ArtifactStore<S: StorageBackend> {
    backend: S,
}

impl<S: StorageBackend> ArtifactStore<S> {
    pub fn new(backend: S) -> Self {
        Self { backend }
    }

    pub fn create_artifact(&self, input: CreateArtifactInput) -> Result<Artifact, StorageError> {
        validate_id("artifact_id", &input.artifact_id)?;
        validate_id("collection", &input.collection)?;
        validate_name(&input.name)?;
        validate_content_type(&input.content_type)?;
        validate_id_list("labels", &input.labels)?;
        validate_provenance(&input.provenance)?;

        self.backend.initialize()?;
        let artifact = Artifact {
            artifact_id: input.artifact_id,
            collection: input.collection,
            name: input.name,
            content_type: input.content_type,
            labels: input.labels,
            provenance: input.provenance,
            latest_revision: None,
            retention: ArtifactRetention::Temporary,
        };
        self.persist_artifact(&artifact, None)
    }

    pub fn fetch_artifact(&self, artifact_id: &str) -> Result<Option<Artifact>, StorageError> {
        validate_id("artifact_id", artifact_id)?;
        self.backend.initialize()?;
        let Some(record) = self.backend.get(NAMESPACE, &artifact_key(artifact_id))? else {
            return Ok(None);
        };
        decode_artifact(&record.body).map(Some)
    }

    pub fn append_revision(
        &self,
        artifact_id: &str,
        input: AppendRevisionInput,
    ) -> Result<ArtifactRevision, StorageError> {
        validate_id("artifact_id", artifact_id)?;
        validate_id("revision_id", &input.revision_id)?;
        validate_json_object("metadata", &input.metadata)?;

        let Some((artifact, revision)) = self.fetch_artifact_with_revision(artifact_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: artifact_key(artifact_id),
            });
        };

        let artifact_revision = ArtifactRevision {
            revision_id: input.revision_id,
            artifact_id: artifact_id.to_string(),
            parent_revision_id: artifact.latest_revision.clone(),
            metadata: input.metadata,
            body: input.body,
            created_at: now_utc_ms(),
        };

        self.backend.put(StoragePutInput::new(
            NAMESPACE,
            revision_key(artifact_id, &artifact_revision.revision_id),
            &artifact.content_type,
            revision_record_metadata(&artifact_revision),
            artifact_revision.body.clone(),
        )?)?;

        let mut updated_artifact = artifact;
        updated_artifact.latest_revision = Some(artifact_revision.revision_id.clone());
        let _ = self.persist_artifact(&updated_artifact, Some(revision))?;
        Ok(artifact_revision)
    }

    pub fn fetch_latest_revision(
        &self,
        artifact_id: &str,
    ) -> Result<Option<ArtifactRevision>, StorageError> {
        let Some(artifact) = self.fetch_artifact(artifact_id)? else {
            return Ok(None);
        };
        let Some(revision_id) = artifact.latest_revision.as_deref() else {
            return Ok(None);
        };
        self.fetch_revision_by_id(artifact_id, revision_id)
    }

    pub fn fetch_revision_by_id(
        &self,
        artifact_id: &str,
        revision_id: &str,
    ) -> Result<Option<ArtifactRevision>, StorageError> {
        validate_id("artifact_id", artifact_id)?;
        validate_id("revision_id", revision_id)?;
        self.backend.initialize()?;
        let Some(record) = self
            .backend
            .get(NAMESPACE, &revision_key(artifact_id, revision_id))?
        else {
            return Ok(None);
        };
        decode_revision_from_record(artifact_id, revision_id, &record.metadata, &record.body)
            .map(Some)
    }

    pub fn list_revisions(
        &self,
        artifact_id: &str,
        options: ArtifactRevisionListOptions,
    ) -> Result<Vec<ArtifactRevisionSummary>, StorageError> {
        validate_id("artifact_id", artifact_id)?;
        validate_revision_list_options(&options)?;
        if options.limit == Some(0) {
            return Ok(Vec::new());
        }
        if self.fetch_artifact(artifact_id)?.is_none() {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: artifact_key(artifact_id),
            });
        }

        self.backend.initialize()?;
        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some(format!("revisions/{artifact_id}/")),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;
        let mut revisions = page
            .records
            .iter()
            .map(|record| decode_revision_summary_from_record(artifact_id, record))
            .collect::<Result<Vec<_>, _>>()?;
        revisions.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.revision_id.cmp(&right.revision_id))
        });
        if options.latest_first {
            revisions.reverse();
        }
        let revisions = window_revision_summaries(revisions, &options)?;
        Ok(revisions)
    }

    pub fn list_by_collection(&self, collection: &str) -> Result<Vec<Artifact>, StorageError> {
        self.list_artifacts(ArtifactListOptions::new().for_collection(collection))
    }

    pub fn list_artifacts(
        &self,
        options: ArtifactListOptions,
    ) -> Result<Vec<Artifact>, StorageError> {
        if let Some(collection) = options.collection.as_deref() {
            validate_id("collection", collection)?;
        }
        validate_id_list("labels", &options.labels)?;
        if options.limit == Some(0) {
            return Ok(Vec::new());
        }

        self.backend.initialize()?;
        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some("manifests/".to_string()),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;

        let mut artifacts = Vec::new();
        for record in &page.records {
            let artifact = decode_artifact(&record.body)?;
            if !artifact_matches_list_options(&artifact, &options) {
                continue;
            }
            artifacts.push(artifact);
            if let Some(limit) = options.limit {
                if artifacts.len() >= limit {
                    break;
                }
            }
        }
        Ok(artifacts)
    }

    pub fn attach_labels(
        &self,
        artifact_id: &str,
        labels: Vec<String>,
    ) -> Result<Artifact, StorageError> {
        validate_id_list("labels", &labels)?;
        let Some((mut artifact, revision)) = self.fetch_artifact_with_revision(artifact_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: artifact_key(artifact_id),
            });
        };
        artifact.labels = labels;
        self.persist_artifact(&artifact, Some(revision))
    }

    pub fn mark_retention(
        &self,
        artifact_id: &str,
        retention: ArtifactRetention,
    ) -> Result<Artifact, StorageError> {
        let Some((mut artifact, revision)) = self.fetch_artifact_with_revision(artifact_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: artifact_key(artifact_id),
            });
        };
        artifact.retention = retention;
        self.persist_artifact(&artifact, Some(revision))
    }

    fn fetch_artifact_with_revision(
        &self,
        artifact_id: &str,
    ) -> Result<Option<(Artifact, storage_core::Revision)>, StorageError> {
        self.backend.initialize()?;
        let Some(record) = self.backend.get(NAMESPACE, &artifact_key(artifact_id))? else {
            return Ok(None);
        };
        let artifact = decode_artifact(&record.body)?;
        Ok(Some((artifact, record.revision)))
    }

    fn persist_artifact(
        &self,
        artifact: &Artifact,
        if_revision: Option<storage_core::Revision>,
    ) -> Result<Artifact, StorageError> {
        let record = self.backend.put(
            StoragePutInput::new(
                NAMESPACE,
                artifact_key(&artifact.artifact_id),
                "application/json",
                artifact_record_metadata(artifact),
                encode_json(&artifact_to_json(artifact))?,
            )?
            .with_if_revision(if_revision),
        )?;
        decode_artifact(&record.body)
    }
}

fn artifact_key(artifact_id: &str) -> String {
    format!("manifests/{artifact_id}.json")
}

fn revision_key(artifact_id: &str, revision_id: &str) -> String {
    format!("revisions/{artifact_id}/{revision_id}.bin")
}

fn revision_id_from_key(artifact_id: &str, key: &str) -> Result<String, StorageError> {
    let prefix = format!("revisions/{artifact_id}/");
    let Some(rest) = key.strip_prefix(&prefix) else {
        return Err(validation(
            "revision_key",
            "revision key had unexpected prefix",
        ));
    };
    let Some(revision_id) = rest.strip_suffix(".bin") else {
        return Err(validation(
            "revision_key",
            "revision key had unexpected suffix",
        ));
    };
    validate_id("revision_id", revision_id)?;
    Ok(revision_id.to_string())
}

fn artifact_matches_list_options(artifact: &Artifact, options: &ArtifactListOptions) -> bool {
    if let Some(collection) = options.collection.as_deref() {
        if artifact.collection != collection {
            return false;
        }
    }
    if let Some(retention) = options.retention {
        if artifact.retention != retention {
            return false;
        }
    }
    options
        .labels
        .iter()
        .all(|label| artifact.labels.iter().any(|candidate| candidate == label))
}

fn validate_revision_list_options(
    options: &ArtifactRevisionListOptions,
) -> Result<(), StorageError> {
    if let Some(after_revision_id) = options.after_revision_id.as_deref() {
        validate_id("after_revision_id", after_revision_id)?;
    }
    Ok(())
}

fn artifact_record_metadata(artifact: &Artifact) -> JsonValue {
    JsonValue::Object(vec![
        (
            "collection".to_string(),
            JsonValue::String(artifact.collection.clone()),
        ),
        (
            "retention".to_string(),
            JsonValue::String(artifact.retention.as_str().to_string()),
        ),
        ("labels".to_string(), string_array_json(&artifact.labels)),
    ])
}

fn revision_record_metadata(revision: &ArtifactRevision) -> JsonValue {
    JsonValue::Object(vec![
        (
            "artifact_id".to_string(),
            JsonValue::String(revision.artifact_id.clone()),
        ),
        (
            "revision_id".to_string(),
            JsonValue::String(revision.revision_id.clone()),
        ),
        (
            "parent_revision_id".to_string(),
            optional_string_json(revision.parent_revision_id.as_deref()),
        ),
        (
            "created_at".to_string(),
            JsonValue::Number(JsonNumber::Integer(revision.created_at as i64)),
        ),
        ("metadata".to_string(), revision.metadata.clone()),
    ])
}

fn artifact_to_json(artifact: &Artifact) -> JsonValue {
    JsonValue::Object(vec![
        (
            "artifact_id".to_string(),
            JsonValue::String(artifact.artifact_id.clone()),
        ),
        (
            "collection".to_string(),
            JsonValue::String(artifact.collection.clone()),
        ),
        ("name".to_string(), JsonValue::String(artifact.name.clone())),
        (
            "content_type".to_string(),
            JsonValue::String(artifact.content_type.clone()),
        ),
        ("labels".to_string(), string_array_json(&artifact.labels)),
        (
            "provenance".to_string(),
            provenance_to_json(&artifact.provenance),
        ),
        (
            "latest_revision".to_string(),
            optional_string_json(artifact.latest_revision.as_deref()),
        ),
        (
            "retention".to_string(),
            JsonValue::String(artifact.retention.as_str().to_string()),
        ),
    ])
}

fn provenance_to_json(provenance: &ArtifactProvenance) -> JsonValue {
    JsonValue::Object(vec![
        (
            "session_id".to_string(),
            optional_string_json(provenance.session_id.as_deref()),
        ),
        (
            "tool_id".to_string(),
            optional_string_json(provenance.tool_id.as_deref()),
        ),
        (
            "job_id".to_string(),
            optional_string_json(provenance.job_id.as_deref()),
        ),
        (
            "agent_id".to_string(),
            optional_string_json(provenance.agent_id.as_deref()),
        ),
    ])
}

fn decode_artifact(body: &[u8]) -> Result<Artifact, StorageError> {
    let value = decode_json(body)?;
    let object = expect_object("artifact", &value)?;
    Ok(Artifact {
        artifact_id: required_string(object, "artifact_id")?,
        collection: required_string(object, "collection")?,
        name: required_string(object, "name")?,
        content_type: required_string(object, "content_type")?,
        labels: required_string_array(object, "labels")?,
        provenance: decode_provenance(required_value(object, "provenance")?)?,
        latest_revision: optional_string(object, "latest_revision")?,
        retention: ArtifactRetention::from_str(&required_string(object, "retention")?)?,
    })
}

fn decode_provenance(value: &JsonValue) -> Result<ArtifactProvenance, StorageError> {
    let object = expect_object("provenance", value)?;
    Ok(ArtifactProvenance {
        session_id: optional_string(object, "session_id")?,
        tool_id: optional_string(object, "tool_id")?,
        job_id: optional_string(object, "job_id")?,
        agent_id: optional_string(object, "agent_id")?,
    })
}

fn decode_revision_from_record(
    artifact_id: &str,
    revision_id: &str,
    metadata: &JsonValue,
    body: &[u8],
) -> Result<ArtifactRevision, StorageError> {
    let object = expect_object("revision_metadata", metadata)?;
    Ok(ArtifactRevision {
        revision_id: revision_id.to_string(),
        artifact_id: artifact_id.to_string(),
        parent_revision_id: optional_string(object, "parent_revision_id")?,
        metadata: required_value(object, "metadata")?.clone(),
        body: body.to_vec(),
        created_at: required_u64(object, "created_at")?,
    })
}

fn decode_revision_summary_from_record(
    artifact_id: &str,
    record: &storage_core::StorageRecord,
) -> Result<ArtifactRevisionSummary, StorageError> {
    let revision_id = revision_id_from_key(artifact_id, &record.key)?;
    let object = expect_object("revision_metadata", &record.metadata)?;
    Ok(ArtifactRevisionSummary {
        revision_id,
        artifact_id: artifact_id.to_string(),
        parent_revision_id: optional_string(object, "parent_revision_id")?,
        metadata: required_value(object, "metadata")?.clone(),
        created_at: required_u64(object, "created_at")?,
        body_len: record.body.len(),
        content_hash: record.content_hash,
    })
}

fn window_revision_summaries(
    revisions: Vec<ArtifactRevisionSummary>,
    options: &ArtifactRevisionListOptions,
) -> Result<Vec<ArtifactRevisionSummary>, StorageError> {
    let start = match options.after_revision_id.as_deref() {
        Some(after_revision_id) => revisions
            .iter()
            .position(|revision| revision.revision_id == after_revision_id)
            .map(|index| index + 1)
            .ok_or_else(|| {
                validation(
                    "after_revision_id",
                    format!("revision '{after_revision_id}' was not found"),
                )
            })?,
        None => 0,
    };
    let limit = options.limit.unwrap_or(usize::MAX);
    Ok(revisions.into_iter().skip(start).take(limit).collect())
}

fn encode_json(value: &JsonValue) -> Result<Vec<u8>, StorageError> {
    let text = serialize(value).map_err(|error| validation("json", error.message))?;
    Ok(text.into_bytes())
}

fn decode_json(bytes: &[u8]) -> Result<JsonValue, StorageError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| validation("body", "artifact manifest must be UTF-8"))?;
    parse_json(text).map_err(|error| validation("body", error.message))
}

fn expect_object<'a>(
    label: &str,
    value: &'a JsonValue,
) -> Result<&'a Vec<(String, JsonValue)>, StorageError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(validation(label, format!("{label} must be a JSON object"))),
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
        _ => Err(validation(field, "field must be a string")),
    }
}

fn optional_string(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Option<String>, StorageError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(value) => Ok(Some(value.clone())),
        _ => Err(validation(field, "field must be null or a string")),
    }
}

fn required_u64(object: &[(String, JsonValue)], field: &str) -> Result<u64, StorageError> {
    match required_value(object, field)? {
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => Ok(*value as u64),
        _ => Err(validation(field, "field must be a non-negative integer")),
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
                _ => Err(validation(field, "array elements must be strings")),
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

fn validate_name(value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(validation("name", "must not be empty"));
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(validation("name", "must not contain newlines"));
    }
    Ok(())
}

fn validate_content_type(value: &str) -> Result<(), StorageError> {
    if !value.contains('/') {
        return Err(validation(
            "content_type",
            "must contain a slash like a MIME type",
        ));
    }
    Ok(())
}

fn validate_provenance(provenance: &ArtifactProvenance) -> Result<(), StorageError> {
    for value in [
        provenance.session_id.as_deref(),
        provenance.tool_id.as_deref(),
        provenance.job_id.as_deref(),
        provenance.agent_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_id("provenance", value)?;
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

    #[test]
    fn artifact_manifest_and_revision_round_trip() {
        let store = ArtifactStore::new(InMemoryStorageBackend::new());
        let artifact = store
            .create_artifact(CreateArtifactInput {
                artifact_id: "plan".to_string(),
                collection: "plans".to_string(),
                name: "Quarterly plan".to_string(),
                content_type: "text/plain".to_string(),
                labels: vec!["roadmap".to_string()],
                provenance: ArtifactProvenance {
                    session_id: Some("demo".to_string()),
                    tool_id: None,
                    job_id: None,
                    agent_id: Some("chief".to_string()),
                },
            })
            .unwrap();

        assert_eq!(artifact.collection, "plans");

        let revision = store
            .append_revision(
                "plan",
                AppendRevisionInput {
                    revision_id: "rev-1".to_string(),
                    metadata: JsonValue::Object(vec![]),
                    body: b"v1".to_vec(),
                },
            )
            .unwrap();

        assert_eq!(revision.parent_revision_id, None);
        assert_eq!(
            store.fetch_latest_revision("plan").unwrap().unwrap().body,
            b"v1".to_vec()
        );
    }

    #[test]
    fn collection_listing_and_label_updates_work() {
        let store = ArtifactStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_artifact(CreateArtifactInput {
                artifact_id: "plan".to_string(),
                collection: "plans".to_string(),
                name: "Quarterly plan".to_string(),
                content_type: "text/plain".to_string(),
                labels: vec![],
                provenance: ArtifactProvenance {
                    session_id: None,
                    tool_id: None,
                    job_id: None,
                    agent_id: None,
                },
            })
            .unwrap();
        let _ = store
            .create_artifact(CreateArtifactInput {
                artifact_id: "report".to_string(),
                collection: "reports".to_string(),
                name: "Weekly report".to_string(),
                content_type: "application/pdf".to_string(),
                labels: vec![],
                provenance: ArtifactProvenance {
                    session_id: None,
                    tool_id: None,
                    job_id: None,
                    agent_id: None,
                },
            })
            .unwrap();

        let plans = store.list_by_collection("plans").unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].artifact_id, "plan");

        let updated = store
            .attach_labels("plan", vec!["roadmap".to_string(), "approved".to_string()])
            .unwrap();
        assert_eq!(updated.labels.len(), 2);
    }

    #[test]
    fn artifact_listing_filters_by_collection_labels_retention_and_limit() {
        let store = ArtifactStore::new(InMemoryStorageBackend::new());
        for (artifact_id, collection, labels) in [
            ("plan-a", "plans", vec!["approved", "roadmap"]),
            ("plan-b", "plans", vec!["draft", "roadmap"]),
            ("report-a", "reports", vec!["approved"]),
        ] {
            let _ = store
                .create_artifact(CreateArtifactInput {
                    artifact_id: artifact_id.to_string(),
                    collection: collection.to_string(),
                    name: artifact_id.to_string(),
                    content_type: "text/plain".to_string(),
                    labels: labels.into_iter().map(str::to_string).collect(),
                    provenance: ArtifactProvenance {
                        session_id: None,
                        tool_id: Some("artifact-list".to_string()),
                        job_id: None,
                        agent_id: None,
                    },
                })
                .unwrap();
        }
        let _ = store
            .mark_retention("plan-a", ArtifactRetention::Retained)
            .unwrap();
        let _ = store
            .mark_retention("report-a", ArtifactRetention::Retained)
            .unwrap();

        let filtered = store
            .list_artifacts(
                ArtifactListOptions::new()
                    .for_collection("plans")
                    .with_label("approved")
                    .with_retention(ArtifactRetention::Retained)
                    .with_limit(1),
            )
            .unwrap();
        assert_eq!(
            filtered
                .iter()
                .map(|artifact| artifact.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["plan-a"]
        );

        let retained = store
            .list_artifacts(ArtifactListOptions::new().with_retention(ArtifactRetention::Retained))
            .unwrap();
        assert_eq!(retained.len(), 2);

        let none = store
            .list_artifacts(ArtifactListOptions::new().with_label("missing"))
            .unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn revision_listing_returns_bounded_metadata_without_bodies() {
        let store = ArtifactStore::new(InMemoryStorageBackend::new());
        let _ = store
            .create_artifact(CreateArtifactInput {
                artifact_id: "plan".to_string(),
                collection: "plans".to_string(),
                name: "Quarterly plan".to_string(),
                content_type: "text/plain".to_string(),
                labels: vec!["roadmap".to_string()],
                provenance: ArtifactProvenance {
                    session_id: Some("demo".to_string()),
                    tool_id: Some("artifact.write_revision".to_string()),
                    job_id: None,
                    agent_id: Some("chief".to_string()),
                },
            })
            .unwrap();

        for revision_id in ["rev-1", "rev-2", "rev-3"] {
            let _ = store
                .append_revision(
                    "plan",
                    AppendRevisionInput {
                        revision_id: revision_id.to_string(),
                        metadata: JsonValue::Object(vec![(
                            "label".to_string(),
                            JsonValue::String(revision_id.to_string()),
                        )]),
                        body: revision_id.as_bytes().to_vec(),
                    },
                )
                .unwrap();
        }

        let all = store
            .list_revisions("plan", ArtifactRevisionListOptions::new())
            .unwrap();
        assert_eq!(
            all.iter()
                .map(|revision| {
                    (
                        revision.revision_id.as_str(),
                        revision.parent_revision_id.as_deref(),
                        revision.body_len,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("rev-1", None, 5),
                ("rev-2", Some("rev-1"), 5),
                ("rev-3", Some("rev-2"), 5)
            ]
        );
        assert_ne!(all[0].content_hash, all[1].content_hash);

        let latest_window = store
            .list_revisions(
                "plan",
                ArtifactRevisionListOptions::new()
                    .latest_first()
                    .after_revision("rev-3")
                    .with_limit(1),
            )
            .unwrap();
        assert_eq!(
            latest_window
                .iter()
                .map(|revision| revision.revision_id.as_str())
                .collect::<Vec<_>>(),
            vec!["rev-2"]
        );
    }

    #[test]
    fn revision_listing_rejects_missing_artifacts_and_bad_cursors() {
        let store = ArtifactStore::new(InMemoryStorageBackend::new());
        let missing_artifact = store
            .list_revisions("missing", ArtifactRevisionListOptions::new())
            .unwrap_err();
        assert!(matches!(missing_artifact, StorageError::NotFound { .. }));

        let _ = store
            .create_artifact(CreateArtifactInput {
                artifact_id: "plan".to_string(),
                collection: "plans".to_string(),
                name: "Quarterly plan".to_string(),
                content_type: "text/plain".to_string(),
                labels: Vec::new(),
                provenance: ArtifactProvenance {
                    session_id: None,
                    tool_id: None,
                    job_id: None,
                    agent_id: None,
                },
            })
            .unwrap();
        let _ = store
            .append_revision(
                "plan",
                AppendRevisionInput {
                    revision_id: "rev-1".to_string(),
                    metadata: JsonValue::Object(vec![]),
                    body: b"v1".to_vec(),
                },
            )
            .unwrap();

        let missing_cursor = store
            .list_revisions(
                "plan",
                ArtifactRevisionListOptions::new().after_revision("rev-2"),
            )
            .unwrap_err();
        assert!(matches!(missing_cursor, StorageError::Validation { .. }));
        assert!(store
            .list_revisions("plan", ArtifactRevisionListOptions::new().with_limit(0))
            .unwrap()
            .is_empty());
    }
}
