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
use std::cmp::Ordering;
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

impl MemoryRecord {
    pub fn is_active_at(&self, now_ms: u64) -> bool {
        !self.tombstoned && self.expires_at.is_none_or(|expires_at| now_ms < expires_at)
    }
}

/// Why a memory should be surfaced for human, agent, or scheduled review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryReviewReason {
    LowConfidence,
    NeverReviewed,
    StaleReview,
    ExpiringSoon,
    Expired,
}

/// One memory plus the deterministic reasons it matched a review policy.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryReviewCandidate {
    pub memory: MemoryRecord,
    pub reasons: Vec<MemoryReviewReason>,
}

/// Portable ordering for memory review queues.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MemoryReviewSort {
    #[default]
    Urgency,
    MemoryId,
    CreatedAtAsc,
    ConfidenceAsc,
    ExpiresAtAsc,
}

/// Portable policy used by agents, tools, and jobs to surface memories that
/// need review without relying on backend-specific queries.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryReviewOptions {
    pub now_ms: u64,
    pub max_confidence: Option<f64>,
    pub stale_after_ms: Option<u64>,
    pub expiring_within_ms: Option<u64>,
    pub include_expired: bool,
    pub include_tombstoned: bool,
    pub sort: MemoryReviewSort,
    pub limit: Option<usize>,
}

impl Default for MemoryReviewOptions {
    fn default() -> Self {
        Self {
            now_ms: 0,
            max_confidence: None,
            stale_after_ms: None,
            expiring_within_ms: None,
            include_expired: false,
            include_tombstoned: false,
            sort: MemoryReviewSort::Urgency,
            limit: None,
        }
    }
}

impl MemoryReviewOptions {
    pub fn at(now_ms: u64) -> Self {
        Self {
            now_ms,
            ..Self::default()
        }
    }

    pub fn max_confidence(mut self, confidence: f64) -> Self {
        self.max_confidence = Some(confidence);
        self
    }

    pub fn stale_after_ms(mut self, duration_ms: u64) -> Self {
        self.stale_after_ms = Some(duration_ms);
        self
    }

    pub fn expiring_within_ms(mut self, duration_ms: u64) -> Self {
        self.expiring_within_ms = Some(duration_ms);
        self
    }

    pub fn include_expired(mut self, include: bool) -> Self {
        self.include_expired = include;
        self
    }

    pub fn include_tombstoned(mut self, include: bool) -> Self {
        self.include_tombstoned = include;
        self
    }

    pub fn sorted_by(mut self, sort: MemoryReviewSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Portable ordering for bounded memory list/read tools.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MemoryListSort {
    #[default]
    MemoryId,
    CreatedAtAsc,
    CreatedAtDesc,
    ConfidenceDesc,
    Subject,
}

/// Portable list selectors used by D18A/D18D tools before backend-specific
/// indexes exist.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryListOptions {
    pub classes: Vec<MemoryClass>,
    pub tags: Vec<String>,
    pub source_refs: Vec<String>,
    pub active_at: Option<u64>,
    pub min_confidence: Option<f64>,
    pub include_tombstoned: bool,
    pub sort: MemoryListSort,
    pub limit: Option<usize>,
}

impl Default for MemoryListOptions {
    fn default() -> Self {
        Self {
            classes: Vec::new(),
            tags: Vec::new(),
            source_refs: Vec::new(),
            active_at: None,
            min_confidence: None,
            include_tombstoned: false,
            sort: MemoryListSort::MemoryId,
            limit: None,
        }
    }
}

impl MemoryListOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_class(mut self, class: MemoryClass) -> Self {
        self.classes.push(class);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_source_ref(mut self, source_ref: impl Into<String>) -> Self {
        self.source_refs.push(source_ref.into());
        self
    }

    pub fn active_at(mut self, now_ms: u64) -> Self {
        self.active_at = Some(now_ms);
        self
    }

    pub fn min_confidence(mut self, confidence: f64) -> Self {
        self.min_confidence = Some(confidence);
        self
    }

    pub fn include_tombstoned(mut self, include: bool) -> Self {
        self.include_tombstoned = include;
        self
    }

    pub fn sorted_by(mut self, sort: MemoryListSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Portable search knobs used by D18D memory tools before backend-specific
/// indexes exist.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemorySearchOptions {
    pub active_at: Option<u64>,
    pub limit: Option<usize>,
}

impl MemorySearchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_at(mut self, now_ms: u64) -> Self {
        self.active_at = Some(now_ms);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
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

    pub fn list_memories_with_options(
        &self,
        options: MemoryListOptions,
    ) -> Result<Vec<MemoryRecord>, StorageError> {
        validate_memory_list_options(&options)?;
        let mut memories =
            self.list_memories(|memory| memory_matches_list_options(memory, &options))?;
        sort_memories(&mut memories, options.sort);
        if let Some(limit) = options.limit {
            memories.truncate(limit);
        }
        Ok(memories)
    }

    pub fn search_lexical(&self, query: &str) -> Result<Vec<MemoryRecord>, StorageError> {
        self.search_lexical_with_options(query, MemorySearchOptions::default())
    }

    pub fn search_lexical_with_options(
        &self,
        query: &str,
        options: MemorySearchOptions,
    ) -> Result<Vec<MemoryRecord>, StorageError> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Err(validation("query", "must not be empty"));
        }
        let mut matches = self.list_memories(|memory| {
            memory_matches_search(memory, &needle)
                && options
                    .active_at
                    .is_none_or(|now_ms| memory.is_active_at(now_ms))
        })?;
        if let Some(limit) = options.limit {
            matches.truncate(limit);
        }
        Ok(matches)
    }

    pub fn search_active_lexical_at(
        &self,
        query: &str,
        now_ms: u64,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryRecord>, StorageError> {
        self.search_lexical_with_options(
            query,
            MemorySearchOptions {
                active_at: Some(now_ms),
                limit,
            },
        )
    }

    pub fn review_candidates(
        &self,
        options: MemoryReviewOptions,
    ) -> Result<Vec<MemoryReviewCandidate>, StorageError> {
        validate_memory_review_options(&options)?;
        let mut candidates = self
            .list_memories(|memory| options.include_tombstoned || !memory.tombstoned)?
            .into_iter()
            .filter_map(|memory| {
                if !options.include_expired
                    && memory
                        .expires_at
                        .is_some_and(|expires_at| expires_at <= options.now_ms)
                {
                    return None;
                }
                let reasons = memory_review_reasons(&memory, &options);
                if reasons.is_empty() {
                    None
                } else {
                    Some(MemoryReviewCandidate { memory, reasons })
                }
            })
            .collect::<Vec<_>>();
        sort_review_candidates(&mut candidates, options.sort);
        if let Some(limit) = options.limit {
            candidates.truncate(limit);
        }
        Ok(candidates)
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

fn memory_matches_search(memory: &MemoryRecord, needle: &str) -> bool {
    !memory.tombstoned
        && [
            memory.subject.to_ascii_lowercase(),
            memory.body.to_ascii_lowercase(),
            memory.tags.join(" ").to_ascii_lowercase(),
        ]
        .iter()
        .any(|haystack| haystack.contains(needle))
}

fn memory_matches_list_options(memory: &MemoryRecord, options: &MemoryListOptions) -> bool {
    if !options.include_tombstoned && memory.tombstoned {
        return false;
    }
    if let Some(now_ms) = options.active_at {
        if !memory.is_active_at(now_ms) {
            return false;
        }
    }
    if let Some(min_confidence) = options.min_confidence {
        if memory.confidence < min_confidence {
            return false;
        }
    }
    if !options.classes.is_empty() && !options.classes.contains(&memory.class) {
        return false;
    }
    if !options
        .tags
        .iter()
        .all(|tag| memory.tags.iter().any(|candidate| candidate == tag))
    {
        return false;
    }
    if !options.source_refs.iter().all(|source_ref| {
        memory
            .source_refs
            .iter()
            .any(|candidate| candidate == source_ref)
    }) {
        return false;
    }

    true
}

fn memory_review_reasons(
    memory: &MemoryRecord,
    options: &MemoryReviewOptions,
) -> Vec<MemoryReviewReason> {
    let mut reasons = Vec::new();
    if let Some(max_confidence) = options.max_confidence {
        if memory.confidence <= max_confidence {
            reasons.push(MemoryReviewReason::LowConfidence);
        }
    }
    if let Some(stale_after_ms) = options.stale_after_ms {
        match memory.reviewed_at {
            Some(reviewed_at) if reviewed_at.saturating_add(stale_after_ms) <= options.now_ms => {
                reasons.push(MemoryReviewReason::StaleReview);
            }
            None => reasons.push(MemoryReviewReason::NeverReviewed),
            _ => {}
        }
    }
    if let Some(expires_at) = memory.expires_at {
        if expires_at <= options.now_ms {
            reasons.push(MemoryReviewReason::Expired);
        } else if options
            .expiring_within_ms
            .is_some_and(|window| expires_at <= options.now_ms.saturating_add(window))
        {
            reasons.push(MemoryReviewReason::ExpiringSoon);
        }
    }
    reasons
}

fn sort_review_candidates(candidates: &mut [MemoryReviewCandidate], sort: MemoryReviewSort) {
    match sort {
        MemoryReviewSort::Urgency => candidates.sort_by(compare_review_urgency),
        MemoryReviewSort::MemoryId => {
            candidates.sort_by(|left, right| left.memory.memory_id.cmp(&right.memory.memory_id))
        }
        MemoryReviewSort::CreatedAtAsc => {
            candidates.sort_by(|left, right| compare_by_created_at_asc(&left.memory, &right.memory))
        }
        MemoryReviewSort::ConfidenceAsc => candidates.sort_by(|left, right| {
            left.memory
                .confidence
                .partial_cmp(&right.memory.confidence)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.memory.memory_id.cmp(&right.memory.memory_id))
        }),
        MemoryReviewSort::ExpiresAtAsc => candidates.sort_by(|left, right| {
            left.memory
                .expires_at
                .unwrap_or(u64::MAX)
                .cmp(&right.memory.expires_at.unwrap_or(u64::MAX))
                .then_with(|| left.memory.memory_id.cmp(&right.memory.memory_id))
        }),
    }
}

fn compare_review_urgency(left: &MemoryReviewCandidate, right: &MemoryReviewCandidate) -> Ordering {
    review_urgency_rank(left)
        .cmp(&review_urgency_rank(right))
        .then_with(|| right.reasons.len().cmp(&left.reasons.len()))
        .then_with(|| {
            left.memory
                .confidence
                .partial_cmp(&right.memory.confidence)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| {
            left.memory
                .expires_at
                .unwrap_or(u64::MAX)
                .cmp(&right.memory.expires_at.unwrap_or(u64::MAX))
        })
        .then_with(|| left.memory.created_at.cmp(&right.memory.created_at))
        .then_with(|| left.memory.memory_id.cmp(&right.memory.memory_id))
}

fn review_urgency_rank(candidate: &MemoryReviewCandidate) -> u8 {
    if candidate.reasons.contains(&MemoryReviewReason::Expired) {
        0
    } else if candidate
        .reasons
        .contains(&MemoryReviewReason::ExpiringSoon)
    {
        1
    } else if candidate
        .reasons
        .contains(&MemoryReviewReason::LowConfidence)
    {
        2
    } else if candidate
        .reasons
        .contains(&MemoryReviewReason::NeverReviewed)
    {
        3
    } else {
        4
    }
}

fn sort_memories(memories: &mut [MemoryRecord], sort: MemoryListSort) {
    match sort {
        MemoryListSort::MemoryId => {
            memories.sort_by(|left, right| left.memory_id.cmp(&right.memory_id))
        }
        MemoryListSort::CreatedAtAsc => memories.sort_by(compare_by_created_at_asc),
        MemoryListSort::CreatedAtDesc => {
            memories.sort_by(|left, right| compare_by_created_at_asc(right, left))
        }
        MemoryListSort::ConfidenceDesc => memories.sort_by(|left, right| {
            right
                .confidence
                .partial_cmp(&left.confidence)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.memory_id.cmp(&right.memory_id))
        }),
        MemoryListSort::Subject => memories.sort_by(|left, right| {
            left.subject
                .cmp(&right.subject)
                .then_with(|| left.memory_id.cmp(&right.memory_id))
        }),
    }
}

fn compare_by_created_at_asc(left: &MemoryRecord, right: &MemoryRecord) -> Ordering {
    left.created_at
        .cmp(&right.created_at)
        .then_with(|| left.memory_id.cmp(&right.memory_id))
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

fn validate_memory_list_options(options: &MemoryListOptions) -> Result<(), StorageError> {
    validate_id_list("tags", &options.tags)?;
    validate_id_list("source_refs", &options.source_refs)?;
    if let Some(confidence) = options.min_confidence {
        validate_confidence(confidence)?;
    }
    Ok(())
}

fn validate_memory_review_options(options: &MemoryReviewOptions) -> Result<(), StorageError> {
    if let Some(confidence) = options.max_confidence {
        validate_confidence(confidence)?;
    }
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

    fn memory_with_id(memory_id: &str, subject: &str, body: &str) -> MemoryRecord {
        MemoryRecord {
            memory_id: memory_id.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            ..memory()
        }
    }

    fn memory_with_created_confidence(
        memory_id: &str,
        subject: &str,
        body: &str,
        created_at: u64,
        confidence: f64,
    ) -> MemoryRecord {
        MemoryRecord {
            created_at,
            confidence,
            ..memory_with_id(memory_id, subject, body)
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
    fn active_search_options_filter_expired_memories_and_limit_results() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let _ = store
            .remember(memory_with_id("pref-tone", "Tone", "Prefer concise recaps"))
            .unwrap();
        let _ = store
            .remember(memory_with_id(
                "pref-format",
                "Format",
                "Prefer concise bullet lists",
            ))
            .unwrap();
        let _ = store
            .remember(memory_with_id(
                "old-tone",
                "Old tone",
                "Prefer concise summaries",
            ))
            .unwrap();
        let _ = store
            .remember(memory_with_id(
                "tombstoned-tone",
                "Tombstoned tone",
                "Prefer concise drafts",
            ))
            .unwrap();
        let expired = store.mark_expired("old-tone", 50).unwrap();
        let _ = store.forget_tombstone("tombstoned-tone").unwrap();

        assert!(expired.is_active_at(49));
        assert!(!expired.is_active_at(50));
        assert_eq!(store.search_lexical("concise").unwrap().len(), 3);

        let active = store
            .search_active_lexical_at("concise", 100, None)
            .unwrap();
        assert_eq!(
            active
                .iter()
                .map(|memory| memory.memory_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pref-format", "pref-tone"]
        );

        let limited = store
            .search_lexical_with_options(
                "concise",
                MemorySearchOptions::new().active_at(100).with_limit(1),
            )
            .unwrap();
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].memory_id, "pref-format");
    }

    #[test]
    fn list_options_compose_filters_sorting_and_limits() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let mut profile =
            memory_with_created_confidence("pref-tone", "Tone", "Prefer concise recaps", 10, 0.8);
        profile.tags = vec!["writing".to_string(), "tone".to_string()];
        profile.source_refs = vec!["session-1".to_string()];

        let mut runbook = memory_with_created_confidence(
            "runbook",
            "Runbook",
            "Prefer concise operational steps",
            30,
            0.95,
        );
        runbook.class = MemoryClass::Procedure;
        runbook.tags = vec!["writing".to_string(), "ops".to_string()];
        runbook.source_refs = vec!["session-1".to_string(), "spec-d18".to_string()];

        let mut low_confidence = memory_with_created_confidence(
            "draft-style",
            "Draft style",
            "Prefer concise drafts",
            40,
            0.4,
        );
        low_confidence.tags = vec!["writing".to_string()];
        low_confidence.source_refs = vec!["session-1".to_string()];

        let mut expired = memory_with_created_confidence(
            "old-style",
            "Old style",
            "Prefer concise summaries",
            50,
            0.9,
        );
        expired.tags = vec!["writing".to_string()];
        expired.source_refs = vec!["session-1".to_string()];
        expired.expires_at = Some(75);

        let _ = store.remember(profile).unwrap();
        let _ = store.remember(runbook).unwrap();
        let _ = store.remember(low_confidence).unwrap();
        let _ = store.remember(expired).unwrap();

        let matches = store
            .list_memories_with_options(
                MemoryListOptions::new()
                    .with_tag("writing")
                    .with_source_ref("session-1")
                    .active_at(100)
                    .min_confidence(0.75)
                    .sorted_by(MemoryListSort::CreatedAtDesc)
                    .with_limit(2),
            )
            .unwrap();

        assert_eq!(
            matches
                .iter()
                .map(|memory| memory.memory_id.as_str())
                .collect::<Vec<_>>(),
            vec!["runbook", "pref-tone"]
        );
    }

    #[test]
    fn list_options_can_include_tombstoned_records_when_requested() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let mut warning = memory_with_id("warn-old-token", "Token", "Old token is invalid");
        warning.class = MemoryClass::Warning;
        let mut active = memory_with_id("warn-active-token", "Active token", "Token is scoped");
        active.class = MemoryClass::Warning;

        let _ = store.remember(warning).unwrap();
        let _ = store.remember(active).unwrap();
        let _ = store.forget_tombstone("warn-old-token").unwrap();

        let default_matches = store
            .list_memories_with_options(MemoryListOptions::new().with_class(MemoryClass::Warning))
            .unwrap();
        assert_eq!(default_matches.len(), 1);
        assert_eq!(default_matches[0].memory_id, "warn-active-token");

        let with_tombstones = store
            .list_memories_with_options(
                MemoryListOptions::new()
                    .with_class(MemoryClass::Warning)
                    .include_tombstoned(true),
            )
            .unwrap();
        assert_eq!(with_tombstones.len(), 2);
    }

    #[test]
    fn review_candidates_explain_low_confidence_stale_and_expiring_memories() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let mut low_confidence =
            memory_with_created_confidence("low-confidence", "Maybe", "Tentative fact", 10, 0.35);
        low_confidence.reviewed_at = Some(90);
        let mut stale =
            memory_with_created_confidence("stale", "Old preference", "Needs recheck", 20, 0.9);
        stale.reviewed_at = Some(40);
        let mut expiring =
            memory_with_created_confidence("expiring", "Lease", "Credential note", 30, 0.95);
        expiring.reviewed_at = Some(95);
        expiring.expires_at = Some(125);
        let mut expired =
            memory_with_created_confidence("expired", "Expired", "Past note", 40, 0.95);
        expired.reviewed_at = Some(95);
        expired.expires_at = Some(99);

        let _ = store.remember(low_confidence).unwrap();
        let _ = store.remember(stale).unwrap();
        let _ = store.remember(expiring).unwrap();
        let _ = store.remember(expired).unwrap();

        let candidates = store
            .review_candidates(
                MemoryReviewOptions::at(100)
                    .max_confidence(0.5)
                    .stale_after_ms(50)
                    .expiring_within_ms(30)
                    .include_expired(true),
            )
            .unwrap();

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.memory.memory_id.as_str())
                .collect::<Vec<_>>(),
            vec!["expired", "expiring", "low-confidence", "stale"]
        );
        assert_eq!(candidates[0].reasons, vec![MemoryReviewReason::Expired]);
        assert_eq!(
            candidates[1].reasons,
            vec![MemoryReviewReason::ExpiringSoon]
        );
        assert_eq!(
            candidates[2].reasons,
            vec![MemoryReviewReason::LowConfidence]
        );
        assert_eq!(candidates[3].reasons, vec![MemoryReviewReason::StaleReview]);
    }

    #[test]
    fn review_candidates_can_skip_expired_records_and_limit_by_sort() {
        let store = MemoryStore::new(InMemoryStorageBackend::new());
        let mut expired = memory_with_created_confidence("expired", "Expired", "Past", 10, 0.2);
        expired.expires_at = Some(90);
        let low = memory_with_created_confidence("low", "Low", "Maybe", 20, 0.4);
        let lower = memory_with_created_confidence("lower", "Lower", "Maybe", 30, 0.1);

        let _ = store.remember(expired).unwrap();
        let _ = store.remember(low).unwrap();
        let _ = store.remember(lower).unwrap();

        let candidates = store
            .review_candidates(
                MemoryReviewOptions::at(100)
                    .max_confidence(0.5)
                    .sorted_by(MemoryReviewSort::ConfidenceAsc)
                    .with_limit(1),
            )
            .unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].memory.memory_id, "lower");
        assert_eq!(
            candidates[0].reasons,
            vec![MemoryReviewReason::LowConfidence]
        );
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
        assert!(
            validate_memory_review_options(&MemoryReviewOptions::at(100).max_confidence(1.5))
                .is_err()
        );
        assert!(MemoryStore::new(InMemoryStorageBackend::new())
            .list_memories_with_options(MemoryListOptions::new().with_tag("bad tag"))
            .is_err());
        assert!(matches!(
            MemoryStore::new(InMemoryStorageBackend::new()).search_lexical("   "),
            Err(StorageError::Validation { .. })
        ));
        assert!(expect_object("memory", &JsonValue::String("bad".to_string())).is_err());
    }
}
