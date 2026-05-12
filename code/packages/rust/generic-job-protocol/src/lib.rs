//! # generic-job-protocol
//!
//! A small, versioned protocol for moving `JobRequest<T>` and `JobResponse<U>`
//! values across language and process boundaries.
//!
//! The crate deliberately stops at the protocol seam. It does not own worker
//! threads, process pools, sockets, or callbacks. TCP runtimes, FFI bridges,
//! and language packages can all use these same envelopes while choosing their
//! own executor implementation.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const JOB_PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRequest<T> {
    pub id: String,
    pub payload: T,
    #[serde(default)]
    pub metadata: JobMetadata,
}

impl<T> JobRequest<T> {
    pub fn new(id: impl Into<String>, payload: T) -> Self {
        Self {
            id: id.into(),
            payload,
            metadata: JobMetadata::default(),
        }
    }

    pub fn with_metadata(mut self, metadata: JobMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn validate_envelope(&self) -> Result<(), JobValidationError> {
        validate_required_token("id", &self.id)?;
        self.metadata.validate()
    }

    pub fn summary(&self) -> JobRequestSummary {
        JobRequestSummary {
            id: self.id.clone(),
            metadata: self.metadata.summary(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobResponse<T> {
    pub id: String,
    pub result: JobResult<T>,
    #[serde(default)]
    pub metadata: JobMetadata,
}

impl<T> JobResponse<T> {
    pub fn ok(id: impl Into<String>, payload: T) -> Self {
        Self {
            id: id.into(),
            result: JobResult::Ok { payload },
            metadata: JobMetadata::default(),
        }
    }

    pub fn error(id: impl Into<String>, error: JobError) -> Self {
        Self {
            id: id.into(),
            result: JobResult::Error { error },
            metadata: JobMetadata::default(),
        }
    }

    pub fn with_metadata(mut self, metadata: JobMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn validate_envelope(&self) -> Result<(), JobValidationError> {
        validate_required_token("id", &self.id)?;
        self.metadata.validate()
    }

    pub fn terminal_status(&self) -> JobTerminalStatus {
        self.result.terminal_status()
    }

    pub fn is_success(&self) -> bool {
        self.result.is_success()
    }

    pub fn is_failure(&self) -> bool {
        self.result.is_failure()
    }

    pub fn summary(&self) -> JobResponseSummary {
        let (retryable_error, error_code, message) = match &self.result {
            JobResult::Ok { .. } => (false, None, None),
            JobResult::Error { error } => (
                error.retryable,
                Some(error.code.clone()),
                Some(error.message.clone()),
            ),
            JobResult::Cancelled { cancellation } => {
                (false, None, Some(cancellation.message.clone()))
            }
            JobResult::TimedOut { timeout } => (false, None, Some(timeout.message.clone())),
        };

        JobResponseSummary {
            id: self.id.clone(),
            status: self.terminal_status(),
            attempt: self.metadata.attempt,
            trace_id: self.metadata.trace_id.clone(),
            retryable_error,
            error_code,
            message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobMetadata {
    #[serde(default)]
    pub created_at_ms: u64,
    #[serde(default)]
    pub deadline_at_ms: Option<u64>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub affinity_key: Option<String>,
    #[serde(default)]
    pub sequence: Option<u64>,
    #[serde(default)]
    pub attempt: u32,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}

impl Default for JobMetadata {
    fn default() -> Self {
        Self {
            created_at_ms: 0,
            deadline_at_ms: None,
            priority: 0,
            affinity_key: None,
            sequence: None,
            attempt: 0,
            trace_id: None,
            tags: BTreeMap::new(),
        }
    }
}

impl JobMetadata {
    pub fn with_created_at_ms(mut self, created_at_ms: u64) -> Self {
        self.created_at_ms = created_at_ms;
        self
    }

    pub fn with_deadline_at_ms(mut self, deadline_at_ms: u64) -> Self {
        self.deadline_at_ms = Some(deadline_at_ms);
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_affinity_key(mut self, affinity_key: impl Into<String>) -> Self {
        self.affinity_key = Some(affinity_key.into());
        self
    }

    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = Some(sequence);
        self
    }

    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    pub fn next_attempt(&self) -> Self {
        let mut next = self.clone();
        next.attempt = next.attempt.saturating_add(1);
        next
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        self.deadline_at_ms
            .is_some_and(|deadline_at_ms| now_ms >= deadline_at_ms)
    }

    pub fn remaining_deadline_ms_at(&self, now_ms: u64) -> Option<u64> {
        self.deadline_at_ms
            .map(|deadline_at_ms| deadline_at_ms.saturating_sub(now_ms))
    }

    pub fn validate(&self) -> Result<(), JobValidationError> {
        if let Some(deadline_at_ms) = self.deadline_at_ms {
            if deadline_at_ms < self.created_at_ms {
                return Err(JobValidationError::DeadlineBeforeCreated {
                    created_at_ms: self.created_at_ms,
                    deadline_at_ms,
                });
            }
        }
        if let Some(affinity_key) = self.affinity_key.as_deref() {
            validate_required_token("affinity_key", affinity_key)?;
        }
        if let Some(trace_id) = self.trace_id.as_deref() {
            validate_required_token("trace_id", trace_id)?;
        }
        for (key, value) in &self.tags {
            validate_required_token("tag_key", key)?;
            validate_single_line("tag_value", value)?;
        }
        Ok(())
    }

    pub fn summary(&self) -> JobMetadataSummary {
        JobMetadataSummary {
            has_deadline: self.deadline_at_ms.is_some(),
            priority: self.priority,
            has_affinity_key: self.affinity_key.is_some(),
            has_sequence: self.sequence.is_some(),
            attempt: self.attempt,
            is_retry: self.attempt > 0,
            has_trace_id: self.trace_id.is_some(),
            tag_count: self.tags.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobMetadataSummary {
    pub has_deadline: bool,
    pub priority: i32,
    pub has_affinity_key: bool,
    pub has_sequence: bool,
    pub attempt: u32,
    pub is_retry: bool,
    pub has_trace_id: bool,
    pub tag_count: usize,
}

impl JobMetadataSummary {
    pub fn is_routable(self) -> bool {
        self.has_affinity_key
    }

    pub fn is_ordered(self) -> bool {
        self.has_sequence
    }

    pub fn is_traceable(self) -> bool {
        self.has_trace_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRequestSummary {
    pub id: String,
    pub metadata: JobMetadataSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRetryPolicy {
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default)]
    pub initial_backoff_ms: u64,
    #[serde(default)]
    pub max_backoff_ms: u64,
}

impl Default for JobRetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1_000,
            max_backoff_ms: 30_000,
        }
    }
}

impl JobRetryPolicy {
    pub fn disabled() -> Self {
        Self {
            max_retries: 0,
            initial_backoff_ms: 0,
            max_backoff_ms: 0,
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_initial_backoff_ms(mut self, initial_backoff_ms: u64) -> Self {
        self.initial_backoff_ms = initial_backoff_ms;
        self
    }

    pub fn with_max_backoff_ms(mut self, max_backoff_ms: u64) -> Self {
        self.max_backoff_ms = max_backoff_ms;
        self
    }

    pub fn retry_delay_ms_for_attempt(&self, attempt: u32) -> u64 {
        let shift = attempt.min(63);
        let multiplier = if shift == 63 {
            u64::MAX
        } else {
            1_u64 << shift
        };
        self.initial_backoff_ms
            .saturating_mul(multiplier)
            .min(self.max_backoff_ms)
    }

    pub fn decision_for_error(&self, metadata: &JobMetadata, error: &JobError) -> JobRetryDecision {
        if !error.retryable {
            return JobRetryDecision::Stop {
                reason: JobRetryStopReason::NotRetryable,
            };
        }
        if metadata.attempt >= self.max_retries {
            return JobRetryDecision::Stop {
                reason: JobRetryStopReason::AttemptsExhausted,
            };
        }

        let next_metadata = metadata.next_attempt();
        JobRetryDecision::Retry {
            metadata: next_metadata,
            delay_ms: self.retry_delay_ms_for_attempt(metadata.attempt),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum JobRetryStopReason {
    NotRetryable,
    AttemptsExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum JobRetryDecision {
    Retry {
        metadata: JobMetadata,
        delay_ms: u64,
    },
    Stop {
        reason: JobRetryStopReason,
    },
}

impl JobRetryDecision {
    pub fn should_retry(&self) -> bool {
        matches!(self, Self::Retry { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobValidationError {
    EmptyField {
        field: &'static str,
    },
    MultilineField {
        field: &'static str,
    },
    DeadlineBeforeCreated {
        created_at_ms: u64,
        deadline_at_ms: u64,
    },
}

impl fmt::Display for JobValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "{field} must not be empty"),
            Self::MultilineField { field } => write!(f, "{field} must be single-line"),
            Self::DeadlineBeforeCreated {
                created_at_ms,
                deadline_at_ms,
            } => write!(
                f,
                "deadline_at_ms {deadline_at_ms} must be greater than or equal to created_at_ms {created_at_ms}"
            ),
        }
    }
}

impl std::error::Error for JobValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum JobResult<T> {
    Ok { payload: T },
    Error { error: JobError },
    Cancelled { cancellation: JobCancellation },
    TimedOut { timeout: JobTimeout },
}

impl<T> JobResult<T> {
    pub fn terminal_status(&self) -> JobTerminalStatus {
        match self {
            Self::Ok { .. } => JobTerminalStatus::Ok,
            Self::Error { .. } => JobTerminalStatus::Error,
            Self::Cancelled { .. } => JobTerminalStatus::Cancelled,
            Self::TimedOut { .. } => JobTerminalStatus::TimedOut,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }

    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobTerminalStatus {
    Ok,
    Error,
    Cancelled,
    TimedOut,
}

impl JobTerminalStatus {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Ok)
    }

    pub fn is_failure(self) -> bool {
        !self.is_success()
    }

    pub fn is_cancelled(self) -> bool {
        matches!(self, Self::Cancelled)
    }

    pub fn is_timed_out(self) -> bool {
        matches!(self, Self::TimedOut)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobResponseSummary {
    pub id: String,
    pub status: JobTerminalStatus,
    pub attempt: u32,
    pub trace_id: Option<String>,
    pub retryable_error: bool,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

impl JobResponseSummary {
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn is_failure(&self) -> bool {
        self.status.is_failure()
    }

    pub fn is_retryable_failure(&self) -> bool {
        self.is_failure() && self.retryable_error
    }

    pub fn was_cancelled(&self) -> bool {
        self.status.is_cancelled()
    }

    pub fn timed_out(&self) -> bool {
        self.status.is_timed_out()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobBatchSummary {
    pub total_requests: usize,
    pub deadline_bound_requests: usize,
    pub routable_requests: usize,
    pub ordered_requests: usize,
    pub retry_requests: usize,
    pub traceable_requests: usize,
    pub total_responses: usize,
    pub successful_responses: usize,
    pub failed_responses: usize,
    pub retryable_failures: usize,
    pub cancelled_responses: usize,
    pub timed_out_responses: usize,
    pub traceable_responses: usize,
    pub max_attempt_observed: u32,
}

impl JobBatchSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_summaries<'a, 'b, RI, SI>(requests: RI, responses: SI) -> Self
    where
        RI: IntoIterator<Item = &'a JobRequestSummary>,
        SI: IntoIterator<Item = &'b JobResponseSummary>,
    {
        let mut summary = Self::empty();
        summary.include_requests(requests);
        summary.include_responses(responses);
        summary
    }

    pub fn from_request_summaries<'a, I>(requests: I) -> Self
    where
        I: IntoIterator<Item = &'a JobRequestSummary>,
    {
        let mut summary = Self::empty();
        summary.include_requests(requests);
        summary
    }

    pub fn from_response_summaries<'a, I>(responses: I) -> Self
    where
        I: IntoIterator<Item = &'a JobResponseSummary>,
    {
        let mut summary = Self::empty();
        summary.include_responses(responses);
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_requests == 0 && self.total_responses == 0
    }

    pub fn total_jobs_seen(&self) -> usize {
        self.total_requests + self.total_responses
    }

    pub fn has_failures(&self) -> bool {
        self.failed_responses > 0
    }

    pub fn has_retryable_failures(&self) -> bool {
        self.retryable_failures > 0
    }

    pub fn has_timeouts(&self) -> bool {
        self.timed_out_responses > 0
    }

    pub fn has_routing_facts(&self) -> bool {
        self.routable_requests > 0 || self.ordered_requests > 0
    }

    fn include_requests<'a, I>(&mut self, requests: I)
    where
        I: IntoIterator<Item = &'a JobRequestSummary>,
    {
        for request in requests {
            self.total_requests += 1;
            if request.metadata.has_deadline {
                self.deadline_bound_requests += 1;
            }
            if request.metadata.is_routable() {
                self.routable_requests += 1;
            }
            if request.metadata.is_ordered() {
                self.ordered_requests += 1;
            }
            if request.metadata.is_retry {
                self.retry_requests += 1;
            }
            if request.metadata.is_traceable() {
                self.traceable_requests += 1;
            }
            self.max_attempt_observed = self.max_attempt_observed.max(request.metadata.attempt);
        }
    }

    fn include_responses<'a, I>(&mut self, responses: I)
    where
        I: IntoIterator<Item = &'a JobResponseSummary>,
    {
        for response in responses {
            self.total_responses += 1;
            if response.is_success() {
                self.successful_responses += 1;
            }
            if response.is_failure() {
                self.failed_responses += 1;
            }
            if response.is_retryable_failure() {
                self.retryable_failures += 1;
            }
            if response.was_cancelled() {
                self.cancelled_responses += 1;
            }
            if response.timed_out() {
                self.timed_out_responses += 1;
            }
            if response.trace_id.is_some() {
                self.traceable_responses += 1;
            }
            self.max_attempt_observed = self.max_attempt_observed.max(response.attempt);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub retryable: bool,
    pub origin: JobErrorOrigin,
    #[serde(default)]
    pub detail: Option<String>,
}

impl JobError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        origin: JobErrorOrigin,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            origin,
            detail: None,
        }
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobErrorOrigin {
    Producer,
    Executor,
    Worker,
    Codec,
    Timeout,
    Cancellation,
    PanicOrException,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobCancellation {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobTimeout {
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobFrameKind {
    Request,
    Response,
    Heartbeat,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobWireFrame<T> {
    pub version: u16,
    pub kind: JobFrameKind,
    pub body: T,
}

impl<T> JobWireFrame<JobRequest<T>> {
    pub fn request(body: JobRequest<T>) -> Self {
        Self {
            version: JOB_PROTOCOL_VERSION,
            kind: JobFrameKind::Request,
            body,
        }
    }
}

impl<T> JobWireFrame<JobResponse<T>> {
    pub fn response(body: JobResponse<T>) -> Self {
        Self {
            version: JOB_PROTOCOL_VERSION,
            kind: JobFrameKind::Response,
            body,
        }
    }
}

#[derive(Debug)]
pub enum JobCodecError {
    OversizedFrame {
        actual: usize,
        max: usize,
    },
    WrongVersion {
        expected: u16,
        actual: u16,
    },
    WrongKind {
        expected: JobFrameKind,
        actual: JobFrameKind,
    },
    Json(serde_json::Error),
}

impl fmt::Display for JobCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OversizedFrame { actual, max } => {
                write!(f, "job frame has {actual} bytes, exceeding maximum {max}")
            }
            Self::WrongVersion { expected, actual } => {
                write!(
                    f,
                    "job frame version mismatch: expected {expected}, got {actual}"
                )
            }
            Self::WrongKind { expected, actual } => {
                write!(
                    f,
                    "job frame kind mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::Json(error) => write!(f, "job frame JSON error: {error}"),
        }
    }
}

impl std::error::Error for JobCodecError {}

impl From<serde_json::Error> for JobCodecError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn encode_request_json_line<T: Serialize>(
    request: &JobRequest<T>,
) -> Result<String, JobCodecError> {
    encode_frame_json_line(&JobWireFrame {
        version: JOB_PROTOCOL_VERSION,
        kind: JobFrameKind::Request,
        body: request,
    })
}

pub fn encode_response_json_line<T: Serialize>(
    response: &JobResponse<T>,
) -> Result<String, JobCodecError> {
    encode_frame_json_line(&JobWireFrame {
        version: JOB_PROTOCOL_VERSION,
        kind: JobFrameKind::Response,
        body: response,
    })
}

pub fn decode_request_json_line<T: DeserializeOwned>(
    line: &str,
) -> Result<JobRequest<T>, JobCodecError> {
    decode_request_json_line_with_limit(line, DEFAULT_MAX_FRAME_BYTES)
}

pub fn decode_response_json_line<T: DeserializeOwned>(
    line: &str,
) -> Result<JobResponse<T>, JobCodecError> {
    decode_response_json_line_with_limit(line, DEFAULT_MAX_FRAME_BYTES)
}

pub fn decode_request_json_line_with_limit<T: DeserializeOwned>(
    line: &str,
    max_frame_bytes: usize,
) -> Result<JobRequest<T>, JobCodecError> {
    let frame = decode_frame_json_line_with_limit::<JobRequest<T>>(line, max_frame_bytes)?;
    validate_frame_header(&frame, JobFrameKind::Request)?;
    Ok(frame.body)
}

pub fn decode_response_json_line_with_limit<T: DeserializeOwned>(
    line: &str,
    max_frame_bytes: usize,
) -> Result<JobResponse<T>, JobCodecError> {
    let frame = decode_frame_json_line_with_limit::<JobResponse<T>>(line, max_frame_bytes)?;
    validate_frame_header(&frame, JobFrameKind::Response)?;
    Ok(frame.body)
}

fn encode_frame_json_line<T: Serialize>(frame: &JobWireFrame<T>) -> Result<String, JobCodecError> {
    let mut encoded = serde_json::to_string(frame)?;
    encoded.push('\n');
    Ok(encoded)
}

fn decode_frame_json_line_with_limit<T: DeserializeOwned>(
    line: &str,
    max_frame_bytes: usize,
) -> Result<JobWireFrame<T>, JobCodecError> {
    if line.len() > max_frame_bytes {
        return Err(JobCodecError::OversizedFrame {
            actual: line.len(),
            max: max_frame_bytes,
        });
    }
    Ok(serde_json::from_str(line.trim_end_matches(['\r', '\n']))?)
}

fn validate_frame_header<T>(
    frame: &JobWireFrame<T>,
    expected_kind: JobFrameKind,
) -> Result<(), JobCodecError> {
    if frame.version != JOB_PROTOCOL_VERSION {
        return Err(JobCodecError::WrongVersion {
            expected: JOB_PROTOCOL_VERSION,
            actual: frame.version,
        });
    }
    if frame.kind != expected_kind {
        return Err(JobCodecError::WrongKind {
            expected: expected_kind,
            actual: frame.kind,
        });
    }
    Ok(())
}

fn validate_required_token(field: &'static str, value: &str) -> Result<(), JobValidationError> {
    if value.trim().is_empty() {
        return Err(JobValidationError::EmptyField { field });
    }
    validate_single_line(field, value)
}

fn validate_single_line(field: &'static str, value: &str) -> Result<(), JobValidationError> {
    if value.contains('\n') || value.contains('\r') {
        Err(JobValidationError::MultilineField { field })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct EchoPayload {
        text: String,
    }

    #[test]
    fn request_json_line_round_trips_payload_and_metadata() {
        let request = JobRequest::new(
            "job-1",
            EchoPayload {
                text: "hello".to_string(),
            },
        )
        .with_metadata(
            JobMetadata::default()
                .with_affinity_key("connection-7")
                .with_sequence(42),
        );

        let encoded = encode_request_json_line(&request).expect("encode request");
        let decoded: JobRequest<EchoPayload> =
            decode_request_json_line(&encoded).expect("decode request");

        assert_eq!(decoded, request);
    }

    #[test]
    fn metadata_helpers_track_deadlines_attempts_and_tags() {
        let metadata = JobMetadata::default()
            .with_created_at_ms(100)
            .with_deadline_at_ms(250)
            .with_priority(10)
            .with_attempt(2)
            .with_trace_id("trace-1")
            .with_affinity_key("device-7")
            .with_sequence(42)
            .with_tag("runtime", "rust");

        assert_eq!(metadata.priority, 10);
        assert_eq!(metadata.attempt, 2);
        assert_eq!(metadata.tags.get("runtime"), Some(&"rust".to_string()));
        assert!(!metadata.is_expired_at(249));
        assert!(metadata.is_expired_at(250));
        assert_eq!(metadata.remaining_deadline_ms_at(240), Some(10));
        assert_eq!(metadata.remaining_deadline_ms_at(260), Some(0));
        assert_eq!(metadata.next_attempt().attempt, 3);
        metadata.validate().unwrap();

        let summary = metadata.summary();
        assert_eq!(
            summary,
            JobMetadataSummary {
                has_deadline: true,
                priority: 10,
                has_affinity_key: true,
                has_sequence: true,
                attempt: 2,
                is_retry: true,
                has_trace_id: true,
                tag_count: 1,
            }
        );
        assert!(summary.is_routable());
        assert!(summary.is_ordered());
        assert!(summary.is_traceable());
    }

    #[test]
    fn request_summary_captures_metadata_without_payload() {
        let request = JobRequest::new(
            "job-7",
            EchoPayload {
                text: "payload should stay opaque".to_string(),
            },
        )
        .with_metadata(
            JobMetadata::default()
                .with_created_at_ms(10)
                .with_deadline_at_ms(20)
                .with_affinity_key("worker-a")
                .with_sequence(3)
                .with_trace_id("trace-7")
                .with_tag("queue", "interactive"),
        );

        assert_eq!(
            request.summary(),
            JobRequestSummary {
                id: "job-7".to_string(),
                metadata: JobMetadataSummary {
                    has_deadline: true,
                    priority: 0,
                    has_affinity_key: true,
                    has_sequence: true,
                    attempt: 0,
                    is_retry: false,
                    has_trace_id: true,
                    tag_count: 1,
                },
            }
        );
    }

    #[test]
    fn retry_policy_classifies_retryable_errors_with_capped_backoff() {
        let metadata = JobMetadata::default()
            .with_created_at_ms(100)
            .with_deadline_at_ms(1_000)
            .with_attempt(2)
            .with_trace_id("trace-7");
        let policy = JobRetryPolicy::default()
            .with_initial_backoff_ms(250)
            .with_max_backoff_ms(750);
        let error = JobError::new("busy", "worker saturated", JobErrorOrigin::Executor)
            .with_retryable(true)
            .with_detail("queue depth exceeded");

        let decision = policy.decision_for_error(&metadata, &error);

        assert!(decision.should_retry());
        assert_eq!(policy.retry_delay_ms_for_attempt(0), 250);
        assert_eq!(policy.retry_delay_ms_for_attempt(1), 500);
        assert_eq!(policy.retry_delay_ms_for_attempt(2), 750);
        assert_eq!(policy.retry_delay_ms_for_attempt(63), 750);
        assert_eq!(error.detail.as_deref(), Some("queue depth exceeded"));
        match decision {
            JobRetryDecision::Retry { metadata, delay_ms } => {
                assert_eq!(metadata.attempt, 3);
                assert_eq!(metadata.trace_id.as_deref(), Some("trace-7"));
                assert_eq!(delay_ms, 750);
            }
            JobRetryDecision::Stop { .. } => panic!("retryable error should retry"),
        }
    }

    #[test]
    fn retry_policy_stops_for_non_retryable_or_exhausted_errors() {
        let policy = JobRetryPolicy::default().with_max_retries(1);
        let retryable = JobError::new("busy", "worker saturated", JobErrorOrigin::Executor)
            .with_retryable(true);
        let permanent = JobError::new("bad_input", "invalid request", JobErrorOrigin::Producer);

        assert_eq!(
            policy.decision_for_error(&JobMetadata::default(), &permanent),
            JobRetryDecision::Stop {
                reason: JobRetryStopReason::NotRetryable
            }
        );
        assert_eq!(
            policy.decision_for_error(&JobMetadata::default().with_attempt(1), &retryable),
            JobRetryDecision::Stop {
                reason: JobRetryStopReason::AttemptsExhausted
            }
        );
        assert!(!JobRetryPolicy::disabled()
            .decision_for_error(&JobMetadata::default(), &retryable)
            .should_retry());
    }

    #[test]
    fn response_summary_captures_terminal_status_and_retry_facts() {
        let response: JobResponse<EchoPayload> = JobResponse::error(
            "job-1",
            JobError::new("busy", "worker saturated", JobErrorOrigin::Executor)
                .with_retryable(true),
        )
        .with_metadata(
            JobMetadata::default()
                .with_attempt(2)
                .with_trace_id("trace-9"),
        );

        let summary = response.summary();

        assert_eq!(response.terminal_status(), JobTerminalStatus::Error);
        assert!(response.is_failure());
        assert!(summary.is_failure());
        assert!(summary.is_retryable_failure());
        assert!(!summary.is_success());
        assert!(!summary.was_cancelled());
        assert!(!summary.timed_out());
        assert_eq!(
            summary,
            JobResponseSummary {
                id: "job-1".to_string(),
                status: JobTerminalStatus::Error,
                attempt: 2,
                trace_id: Some("trace-9".to_string()),
                retryable_error: true,
                error_code: Some("busy".to_string()),
                message: Some("worker saturated".to_string()),
            }
        );
    }

    #[test]
    fn result_status_helpers_distinguish_success_cancel_and_timeout() {
        let ok = JobResult::Ok {
            payload: EchoPayload {
                text: "done".to_string(),
            },
        };
        let cancelled: JobResult<EchoPayload> = JobResult::Cancelled {
            cancellation: JobCancellation {
                message: "user stopped job".to_string(),
            },
        };
        let timed_out: JobResult<EchoPayload> = JobResult::TimedOut {
            timeout: JobTimeout {
                message: "deadline exceeded".to_string(),
            },
        };

        assert_eq!(ok.terminal_status(), JobTerminalStatus::Ok);
        assert!(ok.is_success());
        assert!(ok.terminal_status().is_success());
        assert!(!ok.terminal_status().is_failure());
        assert_eq!(cancelled.terminal_status(), JobTerminalStatus::Cancelled);
        assert!(cancelled.is_failure());
        assert!(cancelled.terminal_status().is_cancelled());
        assert_eq!(timed_out.terminal_status(), JobTerminalStatus::TimedOut);
        assert!(timed_out.is_failure());
        assert!(timed_out.terminal_status().is_timed_out());
    }

    #[test]
    fn response_summary_helpers_classify_terminal_states() {
        let ok_summary = JobResponse::ok(
            "job-1",
            EchoPayload {
                text: "done".to_string(),
            },
        )
        .summary();
        let cancelled_summary: JobResponse<EchoPayload> = JobResponse {
            id: "job-2".to_string(),
            result: JobResult::Cancelled {
                cancellation: JobCancellation {
                    message: "user stopped job".to_string(),
                },
            },
            metadata: JobMetadata::default(),
        };
        let timed_out_summary: JobResponse<EchoPayload> = JobResponse {
            id: "job-3".to_string(),
            result: JobResult::TimedOut {
                timeout: JobTimeout {
                    message: "deadline exceeded".to_string(),
                },
            },
            metadata: JobMetadata::default(),
        };

        assert!(ok_summary.is_success());
        assert!(!ok_summary.is_failure());

        let cancelled_summary = cancelled_summary.summary();
        assert!(cancelled_summary.is_failure());
        assert!(cancelled_summary.was_cancelled());
        assert!(!cancelled_summary.is_retryable_failure());

        let timed_out_summary = timed_out_summary.summary();
        assert!(timed_out_summary.is_failure());
        assert!(timed_out_summary.timed_out());
        assert!(!timed_out_summary.was_cancelled());
    }

    #[test]
    fn batch_summary_rolls_up_request_and_response_read_models() {
        let requests = vec![
            JobRequest::new(
                "job-1",
                EchoPayload {
                    text: "first".to_string(),
                },
            )
            .with_metadata(
                JobMetadata::default()
                    .with_created_at_ms(10)
                    .with_deadline_at_ms(40)
                    .with_affinity_key("worker-a")
                    .with_sequence(1)
                    .with_trace_id("trace-a"),
            )
            .summary(),
            JobRequest::new(
                "job-2",
                EchoPayload {
                    text: "retry".to_string(),
                },
            )
            .with_metadata(JobMetadata::default().with_attempt(1))
            .summary(),
        ];
        let responses = vec![
            JobResponse::ok(
                "job-1",
                EchoPayload {
                    text: "done".to_string(),
                },
            )
            .with_metadata(JobMetadata::default().with_trace_id("trace-a"))
            .summary(),
            JobResponse::<EchoPayload>::error(
                "job-2",
                JobError::new("busy", "worker saturated", JobErrorOrigin::Executor)
                    .with_retryable(true),
            )
            .with_metadata(
                JobMetadata::default()
                    .with_attempt(2)
                    .with_trace_id("trace-b"),
            )
            .summary(),
            JobResponse::<EchoPayload> {
                id: "job-3".to_string(),
                result: JobResult::TimedOut {
                    timeout: JobTimeout {
                        message: "deadline exceeded".to_string(),
                    },
                },
                metadata: JobMetadata::default().with_attempt(3),
            }
            .summary(),
            JobResponse::<EchoPayload> {
                id: "job-4".to_string(),
                result: JobResult::Cancelled {
                    cancellation: JobCancellation {
                        message: "user stopped job".to_string(),
                    },
                },
                metadata: JobMetadata::default().with_attempt(1),
            }
            .summary(),
        ];

        let summary = JobBatchSummary::from_summaries(&requests, &responses);

        assert_eq!(
            summary,
            JobBatchSummary {
                total_requests: 2,
                deadline_bound_requests: 1,
                routable_requests: 1,
                ordered_requests: 1,
                retry_requests: 1,
                traceable_requests: 1,
                total_responses: 4,
                successful_responses: 1,
                failed_responses: 3,
                retryable_failures: 1,
                cancelled_responses: 1,
                timed_out_responses: 1,
                traceable_responses: 2,
                max_attempt_observed: 3,
            }
        );
        assert_eq!(summary.total_jobs_seen(), 6);
        assert!(summary.has_routing_facts());
        assert!(summary.has_failures());
        assert!(summary.has_retryable_failures());
        assert!(summary.has_timeouts());

        let request_only = JobBatchSummary::from_request_summaries(&requests);
        assert_eq!(request_only.total_requests, 2);
        assert_eq!(request_only.total_responses, 0);
        assert_eq!(request_only.max_attempt_observed, 1);

        let response_only = JobBatchSummary::from_response_summaries(&responses);
        assert_eq!(response_only.total_requests, 0);
        assert_eq!(response_only.total_responses, 4);
        assert_eq!(response_only.max_attempt_observed, 3);

        let empty = JobBatchSummary::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.total_jobs_seen(), 0);
        assert!(!empty.has_routing_facts());
        assert!(!empty.has_failures());
    }

    #[test]
    fn envelope_validation_rejects_empty_ids_and_bad_metadata() {
        let request = JobRequest::new(
            "",
            EchoPayload {
                text: "hello".to_string(),
            },
        );
        assert!(matches!(
            request.validate_envelope(),
            Err(JobValidationError::EmptyField { field: "id" })
        ));

        let response = JobResponse::ok(
            "job-1",
            EchoPayload {
                text: "world".to_string(),
            },
        )
        .with_metadata(JobMetadata::default().with_trace_id("trace\nbad"));
        assert!(matches!(
            response.validate_envelope(),
            Err(JobValidationError::MultilineField { field: "trace_id" })
        ));

        let deadline_before_created = JobMetadata::default()
            .with_created_at_ms(200)
            .with_deadline_at_ms(100);
        assert!(matches!(
            deadline_before_created.validate(),
            Err(JobValidationError::DeadlineBeforeCreated { .. })
        ));

        let bad_tag = JobMetadata::default().with_tag("", "value");
        assert!(matches!(
            bad_tag.validate(),
            Err(JobValidationError::EmptyField { field: "tag_key" })
        ));
    }

    #[test]
    fn response_json_line_round_trips_ok_payload() {
        let response = JobResponse::ok(
            "job-1",
            EchoPayload {
                text: "world".to_string(),
            },
        )
        .with_metadata(JobMetadata::default().with_sequence(7));

        let encoded = encode_response_json_line(&response).expect("encode response");
        let decoded: JobResponse<EchoPayload> =
            decode_response_json_line(&encoded).expect("decode response");

        assert_eq!(decoded, response);
    }

    #[test]
    fn response_json_line_round_trips_portable_error() {
        let error = JobError::new(
            "worker_failed",
            "worker raised an exception",
            JobErrorOrigin::Worker,
        );
        let response: JobResponse<EchoPayload> = JobResponse::error("job-1", error.clone());

        let encoded = encode_response_json_line(&response).expect("encode error response");
        let decoded: JobResponse<EchoPayload> =
            decode_response_json_line(&encoded).expect("decode error response");

        assert_eq!(decoded.result, JobResult::Error { error });
    }

    #[test]
    fn decoder_rejects_wrong_version_and_kind() {
        let wrong_version = r#"{"version":999,"kind":"request","body":{"id":"job-1","payload":{"text":"hello"},"metadata":{}}}"#;
        assert!(matches!(
            decode_request_json_line::<EchoPayload>(wrong_version),
            Err(JobCodecError::WrongVersion { .. })
        ));

        let wrong_kind = r#"{"version":1,"kind":"response","body":{"id":"job-1","payload":{"text":"hello"},"metadata":{}}}"#;
        assert!(matches!(
            decode_request_json_line::<EchoPayload>(wrong_kind),
            Err(JobCodecError::WrongKind { .. })
        ));
    }

    #[test]
    fn decoder_rejects_oversized_frames() {
        let request = JobRequest::new(
            "job-1",
            EchoPayload {
                text: "hello".to_string(),
            },
        );
        let encoded = encode_request_json_line(&request).expect("encode request");

        assert!(matches!(
            decode_request_json_line_with_limit::<EchoPayload>(&encoded, 8),
            Err(JobCodecError::OversizedFrame { .. })
        ));
    }
}
