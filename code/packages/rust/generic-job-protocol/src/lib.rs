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
