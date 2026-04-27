//! # memory-store
//!
//! `memory-store` captures durable knowledge that should survive any one
//! session. The store stays intentionally simple in phase one:
//!
//! - each memory is one JSON record
//! - lexical search is a scan over subject/body/tags
//! - superseding and expiry are explicit fields on the record

use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::{parse as parse_json, JsonNumber, JsonValue};
use storage_core::{now_utc_ms, StorageBackend, StorageError, StorageListOptions, StoragePutInput};

const NAMESPACE: &str = "memory";

/// Kind of memory being stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryClass {
    Profile,
    Fact,
    Episodic,
    Procedure,
    Warning,
}

impl MemoryClass {
    fn as_str(self) -> &'static str {
        match self {
            MemoryClass::Profile => "profile",
            MemoryClass::Fact => "fact",
            MemoryClass::Episodic => "episodic",
            MemoryClass::Procedure => "procedure",
            MemoryClass::Warning => "warning",
        }
    }

    fn from_str(value: &str) -> Result<Self, StorageError> {
        match value {
            "profile" => Ok(Self::Profile),
            "fact" => Ok(Self::Fact),
            "episodic" => Ok(Self::Episodic),
            "procedure" => Ok(Self::Procedure),
            "warning" => Ok(Self::Warning),
            _ => Err(validation(
                "class",
                format!("unsupported memory class '{value}'"),
            )),
        }
    }
}

/// Durable memory record.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryRecord {
    pub memory_id: String,
    pub class: MemoryClass,
    pub subject: String,
    pub body: String,
    pub confidence: f64,
    pub source_refs: Vec<String>,
    pub tags: Vec<String>,
    pub supersedes: Vec<String>,
    pub created_at: u64,
    pub reviewed_at: Option<u64>,
    pub expires_at: Option<u64>,
    pub tombstoned: bool,
}

/// Typed memory store layered on `storage-core`.
pub struct MemoryStore<S: StorageBackend> {
    backend: S,
}

impl<S: StorageBackend> MemoryStore<S> {
    pub fn new(backend: S) -> Self {
        Self { backend }
    }

    pub fn remember(&self, memory: MemoryRecord) -> Result<MemoryRecord, StorageError> {
        validate_memory(&memory)?;
        self.backend.initialize()?;
        self.persist_memory(&memory, None)
    }

    pub fn fetch_memory(&self, memory_id: &str) -> Result<Option<MemoryRecord>, StorageError> {
        validate_id("memory_id", memory_id)?;
        self.backend.initialize()?;
        let Some(record) = self.backend.get(NAMESPACE, &memory_key(memory_id))? else {
            return Ok(None);
        };
        decode_memory(&record.body).map(Some)
    }

    pub fn update_confidence(
        &self,
        memory_id: &str,
        confidence: f64,
    ) -> Result<MemoryRecord, StorageError> {
        validate_confidence(confidence)?;
        let Some((mut memory, revision)) = self.fetch_memory_with_revision(memory_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: memory_key(memory_id),
            });
        };
        memory.confidence = confidence;
        memory.reviewed_at = Some(now_utc_ms());
        self.persist_memory(&memory, Some(revision))
    }

    pub fn supersede_old_memory(
        &self,
        memory_id: &str,
        superseded_id: &str,
    ) -> Result<MemoryRecord, StorageError> {
        validate_id("superseded_id", superseded_id)?;
        let Some((mut memory, revision)) = self.fetch_memory_with_revision(memory_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: memory_key(memory_id),
            });
        };
        if !memory.supersedes.iter().any(|value| value == superseded_id) {
            memory.supersedes.push(superseded_id.to_string());
        }
        self.persist_memory(&memory, Some(revision))
    }

    pub fn list_by_class(&self, class: MemoryClass) -> Result<Vec<MemoryRecord>, StorageError> {
        self.list_memories(|memory| memory.class == class && !memory.tombstoned)
    }

    pub fn list_by_tag(&self, tag: &str) -> Result<Vec<MemoryRecord>, StorageError> {
        validate_id("tag", tag)?;
        self.list_memories(|memory| {
            memory.tags.iter().any(|value| value == tag) && !memory.tombstoned
        })
    }

    pub fn search_lexical(&self, query: &str) -> Result<Vec<MemoryRecord>, StorageError> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Err(validation("query", "must not be empty"));
        }
        self.list_memories(|memory| {
            !memory.tombstoned
                && [
                    memory.subject.to_ascii_lowercase(),
                    memory.body.to_ascii_lowercase(),
                    memory.tags.join(" ").to_ascii_lowercase(),
                ]
                .iter()
                .any(|haystack| haystack.contains(&needle))
        })
    }

    pub fn mark_expired(
        &self,
        memory_id: &str,
        expires_at: u64,
    ) -> Result<MemoryRecord, StorageError> {
        let Some((mut memory, revision)) = self.fetch_memory_with_revision(memory_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: memory_key(memory_id),
            });
        };
        memory.expires_at = Some(expires_at);
        self.persist_memory(&memory, Some(revision))
    }

    pub fn forget_tombstone(&self, memory_id: &str) -> Result<MemoryRecord, StorageError> {
        let Some((mut memory, revision)) = self.fetch_memory_with_revision(memory_id)? else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: memory_key(memory_id),
            });
        };
        memory.tombstoned = true;
        memory.reviewed_at = Some(now_utc_ms());
        self.persist_memory(&memory, Some(revision))
    }

    fn list_memories<F>(&self, predicate: F) -> Result<Vec<MemoryRecord>, StorageError>
    where
        F: Fn(&MemoryRecord) -> bool,
    {
        self.backend.initialize()?;
        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some("records/".to_string()),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;
        page.records
            .iter()
            .map(|record| decode_memory(&record.body))
            .filter(|result| result.as_ref().map(&predicate).unwrap_or(true))
            .collect()
    }

    fn fetch_memory_with_revision(
        &self,
        memory_id: &str,
    ) -> Result<Option<(MemoryRecord, storage_core::Revision)>, StorageError> {
        self.backend.initialize()?;
        let Some(record) = self.backend.get(NAMESPACE, &memory_key(memory_id))? else {
            return Ok(None);
        };
        let memory = decode_memory(&record.body)?;
        Ok(Some((memory, record.revision)))
    }

    fn persist_memory(
        &self,
        memory: &MemoryRecord,
        if_revision: Option<storage_core::Revision>,
    ) -> Result<MemoryRecord, StorageError> {
        let record = self.backend.put(
            StoragePutInput::new(
                NAMESPACE,
                memory_key(&memory.memory_id),
                "application/json",
                memory_record_metadata(memory),
                encode_json(&memory_to_json(memory))?,
            )?
            .with_if_revision(if_revision),
        )?;
        decode_memory(&record.body)
    }
}

fn memory_key(memory_id: &str) -> String {
    format!("records/{memory_id}.json")
}

fn memory_record_metadata(memory: &MemoryRecord) -> JsonValue {
    JsonValue::Object(vec![
        (
            "class".to_string(),
            JsonValue::String(memory.class.as_str().to_string()),
        ),
        ("tags".to_string(), string_array_json(&memory.tags)),
        ("tombstoned".to_string(), JsonValue::Bool(memory.tombstoned)),
    ])
}

fn memory_to_json(memory: &MemoryRecord) -> JsonValue {
    JsonValue::Object(vec![
        (
            "memory_id".to_string(),
            JsonValue::String(memory.memory_id.clone()),
        ),
        (
            "class".to_string(),
            JsonValue::String(memory.class.as_str().to_string()),
        ),
        (
            "subject".to_string(),
            JsonValue::String(memory.subject.clone()),
        ),
        ("body".to_string(), JsonValue::String(memory.body.clone())),
        (
            "confidence".to_string(),
            JsonValue::Number(JsonNumber::Float(memory.confidence)),
        ),
        (
            "source_refs".to_string(),
            string_array_json(&memory.source_refs),
        ),
        ("tags".to_string(), string_array_json(&memory.tags)),
        (
            "supersedes".to_string(),
            string_array_json(&memory.supersedes),
        ),
        (
            "created_at".to_string(),
            JsonValue::Number(JsonNumber::Integer(memory.created_at as i64)),
        ),
        (
            "reviewed_at".to_string(),
            optional_u64_json(memory.reviewed_at),
        ),
        (
            "expires_at".to_string(),
            optional_u64_json(memory.expires_at),
        ),
        ("tombstoned".to_string(), JsonValue::Bool(memory.tombstoned)),
    ])
}

fn decode_memory(body: &[u8]) -> Result<MemoryRecord, StorageError> {
    let value = decode_json(body)?;
    let object = expect_object("memory", &value)?;
    Ok(MemoryRecord {
        memory_id: required_string(object, "memory_id")?,
        class: MemoryClass::from_str(&required_string(object, "class")?)?,
        subject: required_string(object, "subject")?,
        body: required_string(object, "body")?,
        confidence: required_f64(object, "confidence")?,
        source_refs: required_string_array(object, "source_refs")?,
        tags: required_string_array(object, "tags")?,
        supersedes: required_string_array(object, "supersedes")?,
        created_at: required_u64(object, "created_at")?,
        reviewed_at: optional_u64(object, "reviewed_at")?,
        expires_at: optional_u64(object, "expires_at")?,
        tombstoned: required_bool(object, "tombstoned")?,
    })
}

fn encode_json(value: &JsonValue) -> Result<Vec<u8>, StorageError> {
    let text = serialize(value).map_err(|error| validation("json", error.message))?;
    Ok(text.into_bytes())
}

fn decode_json(bytes: &[u8]) -> Result<JsonValue, StorageError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| validation("body", "memory record must be UTF-8"))?;
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

fn required_bool(object: &[(String, JsonValue)], field: &str) -> Result<bool, StorageError> {
    match required_value(object, field)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(validation(field, "field must be a boolean")),
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

fn required_f64(object: &[(String, JsonValue)], field: &str) -> Result<f64, StorageError> {
    match required_value(object, field)? {
        JsonValue::Number(JsonNumber::Float(value)) => Ok(*value),
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(*value as f64),
        _ => Err(validation(field, "field must be numeric")),
    }
}

fn required_u64(object: &[(String, JsonValue)], field: &str) -> Result<u64, StorageError> {
    match required_value(object, field)? {
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => Ok(*value as u64),
        _ => Err(validation(field, "field must be a non-negative integer")),
    }
}

fn optional_u64(object: &[(String, JsonValue)], field: &str) -> Result<Option<u64>, StorageError> {
    match required_value(object, field)? {
        JsonValue::Null => Ok(None),
        JsonValue::Number(JsonNumber::Integer(value)) if *value >= 0 => Ok(Some(*value as u64)),
        _ => Err(validation(
            field,
            "field must be null or a non-negative integer",
        )),
    }
}

fn validate_memory(memory: &MemoryRecord) -> Result<(), StorageError> {
    validate_id("memory_id", &memory.memory_id)?;
    validate_subject(&memory.subject)?;
    validate_body(&memory.body)?;
    validate_confidence(memory.confidence)?;
    validate_id_list("source_refs", &memory.source_refs)?;
    validate_id_list("tags", &memory.tags)?;
    validate_id_list("supersedes", &memory.supersedes)?;
    Ok(())
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

fn validate_subject(value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        Err(validation("subject", "must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_body(value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        Err(validation("body", "must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_confidence(value: f64) -> Result<(), StorageError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        Err(validation(
            "confidence",
            "must be a finite number between 0 and 1",
        ))
    } else {
        Ok(())
    }
}

fn string_array_json(values: &[String]) -> JsonValue {
    JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::String(value.clone()))
            .collect(),
    )
}

fn optional_u64_json(value: Option<u64>) -> JsonValue {
    value
        .map(|value| JsonValue::Number(JsonNumber::Integer(value as i64)))
        .unwrap_or(JsonValue::Null)
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

    fn memory() -> MemoryRecord {
        MemoryRecord {
            memory_id: "pref-tone".to_string(),
            class: MemoryClass::Profile,
            subject: "Tone".to_string(),
            body: "Prefer concise recaps".to_string(),
            confidence: 0.8,
            source_refs: vec!["session-1".to_string()],
            tags: vec!["writing".to_string()],
            supersedes: vec![],
            created_at: 10,
            reviewed_at: None,
            expires_at: None,
            tombstoned: false,
        }
    }

    #[test]
    fn remember_and_search_round_trip() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let _ = store.remember(memory()).unwrap();

        let matches = store.search_lexical("concise").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].memory_id, "pref-tone");
    }

    #[test]
    fn confidence_and_tombstone_updates_work() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let _ = store.remember(memory()).unwrap();

        let updated = store.update_confidence("pref-tone", 0.95).unwrap();
        assert_eq!(updated.confidence, 0.95);

        let tombstoned = store.forget_tombstone("pref-tone").unwrap();
        assert!(tombstoned.tombstoned);
        assert!(store
            .list_by_class(MemoryClass::Profile)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn tag_listing_superseding_and_expiry_updates_work() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let _ = store.remember(memory()).unwrap();

        assert_eq!(store.fetch_memory("missing").unwrap(), None);
        assert_eq!(store.list_by_tag("writing").unwrap().len(), 1);

        let superseded = store.supersede_old_memory("pref-tone", "old-tone").unwrap();
        assert_eq!(superseded.supersedes, vec!["old-tone".to_string()]);

        let expired = store.mark_expired("pref-tone", 99).unwrap();
        assert_eq!(expired.expires_at, Some(99));
    }

    #[test]
    fn helper_validations_cover_error_paths() {
        assert_eq!(
            MemoryClass::from_str("warning").unwrap(),
            MemoryClass::Warning
        );
        assert!(MemoryClass::from_str("unknown").is_err());
        assert!(validate_confidence(1.5).is_err());
        assert!(validate_subject("").is_err());
        assert!(validate_body("").is_err());
        assert!(matches!(
            MemoryStore::new(InMemoryStorageBackend::new()).search_lexical("   "),
            Err(StorageError::Validation { .. })
        ));
        assert!(expect_object("memory", &JsonValue::String("bad".to_string())).is_err());
    }
}
