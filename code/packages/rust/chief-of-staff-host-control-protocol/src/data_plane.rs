//! Strict bounded data-plane records carried by the authenticated host session.

use std::collections::BTreeMap;

use crate::ControlError;

pub(crate) const RECEIVE_REQUEST_TAG: u8 = 10;
pub(crate) const PUBLISH_REQUEST_TAG: u8 = 11;
pub(crate) const ACKNOWLEDGE_REQUEST_TAG: u8 = 12;
pub(crate) const COMPLETE_REQUEST_TAG: u8 = 13;
pub(crate) const RECEIVED_RESPONSE_TAG: u8 = 20;
pub(crate) const PUBLISHED_RESPONSE_TAG: u8 = 21;
pub(crate) const ACKNOWLEDGED_RESPONSE_TAG: u8 = 22;
pub(crate) const COMPLETED_RESPONSE_TAG: u8 = 23;
pub(crate) const FAILED_RESPONSE_TAG: u8 = 24;

/// Maximum plaintext bytes in one data-plane body before the six-byte control header.
pub const MAX_DATA_PLANE_RECORD_BYTES: usize = 768 * 1024;
/// Maximum verified channel payload or completion text in one v1 exchange.
pub const MAX_DATA_PLANE_PAYLOAD_BYTES: usize = 512 * 1024;
/// Maximum messages in one receive page.
pub const MAX_DATA_PLANE_MESSAGES: usize = 64;
const MAX_CONTENT_TYPE_BYTES: usize = 1_024;
const MAX_MODEL_BYTES: usize = 200;
const MAX_SYSTEM_BYTES: usize = 64 * 1024;
const MAX_PROMPT_MESSAGES: usize = 64;
const MAX_PROMPT_TEXT_BYTES: usize = 64 * 1024;
const MAX_STOP_SEQUENCES: usize = 16;
const MAX_STOP_SEQUENCE_BYTES: usize = 1_024;
const MAX_METADATA_ENTRIES: usize = 32;
const MAX_METADATA_KEY_BYTES: usize = 256;
const MAX_METADATA_VALUE_BYTES: usize = 4 * 1024;
const MAX_PROVIDER_FIELD_BYTES: usize = 512;
const MAX_PROVIDER_ENDPOINT_BYTES: usize = 4 * 1024;
const MAX_COMPLETION_TOKENS: u32 = 1_000_000;

/// Non-zero request identity minted monotonically by the child endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(u64);

impl RequestId {
    /// Validate a non-zero request identity.
    pub fn new(value: u64) -> Result<Self, ControlError> {
        if value == 0 {
            return Err(ControlError::InvalidDataPlaneRecord);
        }
        Ok(Self(value))
    }

    /// Return the wire integer.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Data-plane operation used to enforce response shape and correlation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataPlaneOperation {
    /// Read verified messages from one authorized channel.
    Receive,
    /// Publish plaintext to one authorized channel.
    Publish,
    /// Advance one authorized receiver cursor.
    Acknowledge,
    /// Execute one provider-neutral LLM completion.
    Complete,
}

/// Text-only role accepted by the first production Level 1 data plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptRole {
    /// System instruction inside the ordered prompt.
    System,
    /// User-authored input.
    User,
    /// Prior assistant output.
    Assistant,
}

/// One bounded provider-neutral prompt message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptMessage {
    /// Conversation role.
    pub role: PromptRole,
    /// UTF-8 text content.
    pub text: String,
}

/// Provider-neutral completion request used by a child host.
#[derive(Clone, Debug, PartialEq)]
pub struct CompletionCall {
    /// Provider-specific model selector.
    pub model: String,
    /// Optional top-level system instruction.
    pub system: Option<String>,
    /// Ordered text prompt.
    pub messages: Vec<PromptMessage>,
    /// Finite sampling temperature from zero through two.
    pub temperature: f32,
    /// Optional non-zero output-token cap.
    pub max_tokens: Option<u32>,
    /// Provider stop strings.
    pub stop_sequences: Vec<String>,
    /// Optional deterministic seed.
    pub seed: Option<u64>,
    /// Bounded audit metadata with canonical key order.
    pub metadata: BTreeMap<String, String>,
}

/// Provider identity returned with every successful completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionProvider {
    /// Provider vendor.
    pub vendor: String,
    /// Stable model family.
    pub model_family: String,
    /// Stable model version.
    pub model_version: String,
    /// Optional provider endpoint identity.
    pub endpoint: Option<String>,
}

/// Provider-reported token accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionUsage {
    /// Tokens consumed by the prompt.
    pub input_tokens: u64,
    /// Tokens emitted by the model.
    pub output_tokens: u64,
    /// Prompt tokens served from provider cache.
    pub cached_tokens: u64,
}

/// Why a provider stopped generating.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionFinishReason {
    /// Natural end or stop sequence.
    Stop,
    /// Output-token cap reached.
    MaxTokens,
    /// Provider refused the request.
    Refusal,
    /// Provider-specific reason outside the stable taxonomy.
    Other,
}

/// Successful provider-neutral completion result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionResult {
    /// Generated text.
    pub text: String,
    /// Provider-reported model.
    pub model: String,
    /// Provider audit identity.
    pub provider: CompletionProvider,
    /// Provider token accounting.
    pub usage: CompletionUsage,
    /// Provider stop reason.
    pub finish_reason: CompletionFinishReason,
    /// Provider-reported latency in milliseconds.
    pub latency_ms: u64,
}

/// Verified plaintext channel message returned to the child host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataPlaneMessage {
    /// Canonical UUID-v7 message identity.
    pub message_id: [u8; 16],
    /// Durable global channel sequence.
    pub sequence: u64,
    /// Authenticated originator timestamp.
    pub timestamp_ns: u64,
    /// Authenticated MIME content type.
    pub content_type: String,
    /// Verified plaintext payload.
    pub payload: Vec<u8>,
}

/// Stable failure category returned without provider or payload diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataPlaneFailure {
    /// Request shape or static bounds were rejected.
    InvalidRequest,
    /// The package is not authorized for the requested operation.
    Unauthorized,
    /// Channel receive, publish, or acknowledgement failed.
    Channel,
    /// Provider-neutral completion failed.
    Completion,
    /// A required service is temporarily unavailable.
    Unavailable,
    /// An internal adapter failed without a safe public detail.
    Internal,
}

/// Authenticated child-to-orchestrator data-plane request.
#[derive(Clone, Debug, PartialEq)]
pub enum DataPlaneRequest {
    /// Read a bounded page from one channel.
    Receive {
        /// Correlation identity.
        id: RequestId,
        /// Canonical UUID-v7 channel identity.
        channel_id: [u8; 16],
        /// Maximum messages, from one through 64.
        limit: u16,
    },
    /// Publish one bounded payload.
    Publish {
        /// Correlation identity.
        id: RequestId,
        /// Canonical UUID-v7 channel identity.
        channel_id: [u8; 16],
        /// MIME content type.
        content_type: String,
        /// Plaintext payload.
        payload: Vec<u8>,
    },
    /// Acknowledge one previously delivered message.
    Acknowledge {
        /// Correlation identity.
        id: RequestId,
        /// Canonical UUID-v7 channel identity.
        channel_id: [u8; 16],
        /// Canonical UUID-v7 delivered-message identity.
        message_id: [u8; 16],
    },
    /// Execute one provider-neutral completion.
    Complete {
        /// Correlation identity.
        id: RequestId,
        /// Validated completion input.
        call: CompletionCall,
    },
}

impl DataPlaneRequest {
    /// Return the correlation identity.
    pub fn id(&self) -> RequestId {
        match self {
            Self::Receive { id, .. }
            | Self::Publish { id, .. }
            | Self::Acknowledge { id, .. }
            | Self::Complete { id, .. } => *id,
        }
    }

    /// Return the required response operation.
    pub fn operation(&self) -> DataPlaneOperation {
        match self {
            Self::Receive { .. } => DataPlaneOperation::Receive,
            Self::Publish { .. } => DataPlaneOperation::Publish,
            Self::Acknowledge { .. } => DataPlaneOperation::Acknowledge,
            Self::Complete { .. } => DataPlaneOperation::Complete,
        }
    }
}

/// Authenticated orchestrator-to-child data-plane response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataPlaneResponse {
    /// Ordered verified messages, possibly empty.
    Received {
        /// Correlation identity.
        id: RequestId,
        /// At most 64 verified messages.
        messages: Vec<DataPlaneMessage>,
    },
    /// Durable publication receipt.
    Published {
        /// Correlation identity.
        id: RequestId,
        /// Canonical UUID-v7 message identity.
        message_id: [u8; 16],
        /// Durable global channel sequence.
        sequence: u64,
        /// Authenticated publish timestamp.
        timestamp_ns: u64,
    },
    /// Monotonic receiver cursor after acknowledgement.
    Acknowledged {
        /// Correlation identity.
        id: RequestId,
        /// Durable sequence acknowledged through.
        sequence: u64,
    },
    /// Successful provider-neutral completion.
    Completed {
        /// Correlation identity.
        id: RequestId,
        /// Completion output and audit identity.
        result: Box<CompletionResult>,
    },
    /// Redacted operation failure.
    Failed {
        /// Correlation identity.
        id: RequestId,
        /// Stable non-sensitive failure class.
        failure: DataPlaneFailure,
    },
}

impl DataPlaneResponse {
    /// Return the correlation identity.
    pub fn id(&self) -> RequestId {
        match self {
            Self::Received { id, .. }
            | Self::Published { id, .. }
            | Self::Acknowledged { id, .. }
            | Self::Completed { id, .. }
            | Self::Failed { id, .. } => *id,
        }
    }

    /// Return the successful operation kind, or `None` for a generic failure.
    pub fn operation(&self) -> Option<DataPlaneOperation> {
        match self {
            Self::Received { .. } => Some(DataPlaneOperation::Receive),
            Self::Published { .. } => Some(DataPlaneOperation::Publish),
            Self::Acknowledged { .. } => Some(DataPlaneOperation::Acknowledge),
            Self::Completed { .. } => Some(DataPlaneOperation::Complete),
            Self::Failed { .. } => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum DataRecord {
    Request(DataPlaneRequest),
    Response(DataPlaneResponse),
}

pub(crate) fn encode(record: &DataRecord) -> Result<(u8, Vec<u8>), ControlError> {
    let mut encoder = Encoder::default();
    let tag = match record {
        DataRecord::Request(request) => {
            encoder.u64(request.id().get());
            match request {
                DataPlaneRequest::Receive {
                    channel_id, limit, ..
                } => {
                    validate_uuid_v7(channel_id)?;
                    if !(1..=MAX_DATA_PLANE_MESSAGES as u16).contains(limit) {
                        return Err(ControlError::InvalidDataPlaneRecord);
                    }
                    encoder.fixed(channel_id);
                    encoder.u16(*limit);
                    RECEIVE_REQUEST_TAG
                }
                DataPlaneRequest::Publish {
                    channel_id,
                    content_type,
                    payload,
                    ..
                } => {
                    validate_uuid_v7(channel_id)?;
                    validate_string(content_type, 1, MAX_CONTENT_TYPE_BYTES)?;
                    validate_bytes(payload, MAX_DATA_PLANE_PAYLOAD_BYTES)?;
                    encoder.fixed(channel_id);
                    encoder.string(content_type)?;
                    encoder.bytes(payload)?;
                    PUBLISH_REQUEST_TAG
                }
                DataPlaneRequest::Acknowledge {
                    channel_id,
                    message_id,
                    ..
                } => {
                    validate_uuid_v7(channel_id)?;
                    validate_uuid_v7(message_id)?;
                    encoder.fixed(channel_id);
                    encoder.fixed(message_id);
                    ACKNOWLEDGE_REQUEST_TAG
                }
                DataPlaneRequest::Complete { call, .. } => {
                    encode_completion_call(&mut encoder, call)?;
                    COMPLETE_REQUEST_TAG
                }
            }
        }
        DataRecord::Response(response) => {
            encoder.u64(response.id().get());
            match response {
                DataPlaneResponse::Received { messages, .. } => {
                    if messages.len() > MAX_DATA_PLANE_MESSAGES {
                        return Err(ControlError::InvalidDataPlaneRecord);
                    }
                    encoder.u16(messages.len() as u16);
                    for message in messages {
                        encode_message(&mut encoder, message)?;
                        encoder.ensure_record_bound()?;
                    }
                    RECEIVED_RESPONSE_TAG
                }
                DataPlaneResponse::Published {
                    message_id,
                    sequence,
                    timestamp_ns,
                    ..
                } => {
                    validate_uuid_v7(message_id)?;
                    encoder.fixed(message_id);
                    encoder.u64(*sequence);
                    encoder.u64(*timestamp_ns);
                    PUBLISHED_RESPONSE_TAG
                }
                DataPlaneResponse::Acknowledged { sequence, .. } => {
                    encoder.u64(*sequence);
                    ACKNOWLEDGED_RESPONSE_TAG
                }
                DataPlaneResponse::Completed { result, .. } => {
                    encode_completion_result(&mut encoder, result)?;
                    COMPLETED_RESPONSE_TAG
                }
                DataPlaneResponse::Failed { failure, .. } => {
                    encoder.u8(encode_failure(*failure));
                    FAILED_RESPONSE_TAG
                }
            }
        }
    };
    if encoder.bytes.len() > MAX_DATA_PLANE_RECORD_BYTES {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    Ok((tag, encoder.bytes))
}

pub(crate) fn decode(tag: u8, body: &[u8]) -> Result<DataRecord, ControlError> {
    if body.len() > MAX_DATA_PLANE_RECORD_BYTES {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    let mut decoder = Decoder::new(body);
    let id = RequestId::new(decoder.u64()?)?;
    let record = match tag {
        RECEIVE_REQUEST_TAG => DataRecord::Request(DataPlaneRequest::Receive {
            id,
            channel_id: decoder.uuid_v7()?,
            limit: {
                let limit = decoder.u16()?;
                if !(1..=MAX_DATA_PLANE_MESSAGES as u16).contains(&limit) {
                    return Err(ControlError::InvalidDataPlaneRecord);
                }
                limit
            },
        }),
        PUBLISH_REQUEST_TAG => DataRecord::Request(DataPlaneRequest::Publish {
            id,
            channel_id: decoder.uuid_v7()?,
            content_type: decoder.string(1, MAX_CONTENT_TYPE_BYTES)?,
            payload: decoder.bytes(MAX_DATA_PLANE_PAYLOAD_BYTES)?,
        }),
        ACKNOWLEDGE_REQUEST_TAG => DataRecord::Request(DataPlaneRequest::Acknowledge {
            id,
            channel_id: decoder.uuid_v7()?,
            message_id: decoder.uuid_v7()?,
        }),
        COMPLETE_REQUEST_TAG => DataRecord::Request(DataPlaneRequest::Complete {
            id,
            call: decode_completion_call(&mut decoder)?,
        }),
        RECEIVED_RESPONSE_TAG => {
            let count = decoder.u16()? as usize;
            if count > MAX_DATA_PLANE_MESSAGES {
                return Err(ControlError::InvalidDataPlaneRecord);
            }
            let mut messages = Vec::with_capacity(count);
            for _ in 0..count {
                messages.push(decode_message(&mut decoder)?);
            }
            DataRecord::Response(DataPlaneResponse::Received { id, messages })
        }
        PUBLISHED_RESPONSE_TAG => DataRecord::Response(DataPlaneResponse::Published {
            id,
            message_id: decoder.uuid_v7()?,
            sequence: decoder.u64()?,
            timestamp_ns: decoder.u64()?,
        }),
        ACKNOWLEDGED_RESPONSE_TAG => DataRecord::Response(DataPlaneResponse::Acknowledged {
            id,
            sequence: decoder.u64()?,
        }),
        COMPLETED_RESPONSE_TAG => DataRecord::Response(DataPlaneResponse::Completed {
            id,
            result: Box::new(decode_completion_result(&mut decoder)?),
        }),
        FAILED_RESPONSE_TAG => DataRecord::Response(DataPlaneResponse::Failed {
            id,
            failure: decode_failure(decoder.u8()?)?,
        }),
        _ => return Err(ControlError::UnknownMessageKind),
    };
    decoder.finish()?;
    Ok(record)
}

fn encode_message(encoder: &mut Encoder, message: &DataPlaneMessage) -> Result<(), ControlError> {
    validate_uuid_v7(&message.message_id)?;
    validate_string(&message.content_type, 1, MAX_CONTENT_TYPE_BYTES)?;
    validate_bytes(&message.payload, MAX_DATA_PLANE_PAYLOAD_BYTES)?;
    encoder.fixed(&message.message_id);
    encoder.u64(message.sequence);
    encoder.u64(message.timestamp_ns);
    encoder.string(&message.content_type)?;
    encoder.bytes(&message.payload)
}

fn decode_message(decoder: &mut Decoder<'_>) -> Result<DataPlaneMessage, ControlError> {
    Ok(DataPlaneMessage {
        message_id: decoder.uuid_v7()?,
        sequence: decoder.u64()?,
        timestamp_ns: decoder.u64()?,
        content_type: decoder.string(1, MAX_CONTENT_TYPE_BYTES)?,
        payload: decoder.bytes(MAX_DATA_PLANE_PAYLOAD_BYTES)?,
    })
}

fn encode_completion_call(
    encoder: &mut Encoder,
    call: &CompletionCall,
) -> Result<(), ControlError> {
    validate_string(&call.model, 1, MAX_MODEL_BYTES)?;
    if call.messages.is_empty() || call.messages.len() > MAX_PROMPT_MESSAGES {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    if !call.temperature.is_finite() || !(0.0..=2.0).contains(&call.temperature) {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    if call
        .max_tokens
        .is_some_and(|tokens| tokens == 0 || tokens > MAX_COMPLETION_TOKENS)
        || call.stop_sequences.len() > MAX_STOP_SEQUENCES
        || call.metadata.len() > MAX_METADATA_ENTRIES
    {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    encoder.string(&call.model)?;
    encoder.optional_string(call.system.as_deref(), MAX_SYSTEM_BYTES)?;
    encoder.u16(call.messages.len() as u16);
    for message in &call.messages {
        validate_string(&message.text, 1, MAX_PROMPT_TEXT_BYTES)?;
        encoder.u8(encode_role(message.role));
        encoder.string(&message.text)?;
        encoder.ensure_record_bound()?;
    }
    encoder.u32(call.temperature.to_bits());
    encoder.optional_u32(call.max_tokens);
    encoder.u8(call.stop_sequences.len() as u8);
    for stop in &call.stop_sequences {
        validate_string(stop, 1, MAX_STOP_SEQUENCE_BYTES)?;
        encoder.string(stop)?;
        encoder.ensure_record_bound()?;
    }
    encoder.optional_u64(call.seed);
    encoder.u8(call.metadata.len() as u8);
    for (key, value) in &call.metadata {
        validate_string(key, 1, MAX_METADATA_KEY_BYTES)?;
        validate_string(value, 0, MAX_METADATA_VALUE_BYTES)?;
        encoder.string(key)?;
        encoder.string(value)?;
        encoder.ensure_record_bound()?;
    }
    Ok(())
}

fn decode_completion_call(decoder: &mut Decoder<'_>) -> Result<CompletionCall, ControlError> {
    let model = decoder.string(1, MAX_MODEL_BYTES)?;
    let system = decoder.optional_string(MAX_SYSTEM_BYTES)?;
    let message_count = decoder.u16()? as usize;
    if message_count == 0 || message_count > MAX_PROMPT_MESSAGES {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    let mut messages = Vec::with_capacity(message_count);
    for _ in 0..message_count {
        messages.push(PromptMessage {
            role: decode_role(decoder.u8()?)?,
            text: decoder.string(1, MAX_PROMPT_TEXT_BYTES)?,
        });
    }
    let temperature = f32::from_bits(decoder.u32()?);
    if !temperature.is_finite() || !(0.0..=2.0).contains(&temperature) {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    let max_tokens = decoder.optional_u32()?;
    if max_tokens.is_some_and(|tokens| tokens == 0 || tokens > MAX_COMPLETION_TOKENS) {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    let stop_count = decoder.u8()? as usize;
    if stop_count > MAX_STOP_SEQUENCES {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    let mut stop_sequences = Vec::with_capacity(stop_count);
    for _ in 0..stop_count {
        stop_sequences.push(decoder.string(1, MAX_STOP_SEQUENCE_BYTES)?);
    }
    let seed = decoder.optional_u64()?;
    let metadata_count = decoder.u8()? as usize;
    if metadata_count > MAX_METADATA_ENTRIES {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    let mut metadata = BTreeMap::new();
    for _ in 0..metadata_count {
        let key = decoder.string(1, MAX_METADATA_KEY_BYTES)?;
        let value = decoder.string(0, MAX_METADATA_VALUE_BYTES)?;
        if metadata.insert(key, value).is_some() {
            return Err(ControlError::InvalidDataPlaneRecord);
        }
    }
    Ok(CompletionCall {
        model,
        system,
        messages,
        temperature,
        max_tokens,
        stop_sequences,
        seed,
        metadata,
    })
}

fn encode_completion_result(
    encoder: &mut Encoder,
    result: &CompletionResult,
) -> Result<(), ControlError> {
    validate_string(&result.text, 1, MAX_DATA_PLANE_PAYLOAD_BYTES)?;
    validate_string(&result.model, 1, MAX_MODEL_BYTES)?;
    validate_string(&result.provider.vendor, 1, MAX_PROVIDER_FIELD_BYTES)?;
    validate_string(&result.provider.model_family, 1, MAX_PROVIDER_FIELD_BYTES)?;
    validate_string(&result.provider.model_version, 1, MAX_PROVIDER_FIELD_BYTES)?;
    encoder.string(&result.text)?;
    encoder.string(&result.model)?;
    encoder.string(&result.provider.vendor)?;
    encoder.string(&result.provider.model_family)?;
    encoder.string(&result.provider.model_version)?;
    encoder.optional_string(
        result.provider.endpoint.as_deref(),
        MAX_PROVIDER_ENDPOINT_BYTES,
    )?;
    encoder.u64(result.usage.input_tokens);
    encoder.u64(result.usage.output_tokens);
    encoder.u64(result.usage.cached_tokens);
    encoder.u8(encode_finish_reason(result.finish_reason));
    encoder.u64(result.latency_ms);
    Ok(())
}

fn decode_completion_result(decoder: &mut Decoder<'_>) -> Result<CompletionResult, ControlError> {
    Ok(CompletionResult {
        text: decoder.string(1, MAX_DATA_PLANE_PAYLOAD_BYTES)?,
        model: decoder.string(1, MAX_MODEL_BYTES)?,
        provider: CompletionProvider {
            vendor: decoder.string(1, MAX_PROVIDER_FIELD_BYTES)?,
            model_family: decoder.string(1, MAX_PROVIDER_FIELD_BYTES)?,
            model_version: decoder.string(1, MAX_PROVIDER_FIELD_BYTES)?,
            endpoint: decoder.optional_string(MAX_PROVIDER_ENDPOINT_BYTES)?,
        },
        usage: CompletionUsage {
            input_tokens: decoder.u64()?,
            output_tokens: decoder.u64()?,
            cached_tokens: decoder.u64()?,
        },
        finish_reason: decode_finish_reason(decoder.u8()?)?,
        latency_ms: decoder.u64()?,
    })
}

fn validate_uuid_v7(bytes: &[u8; 16]) -> Result<(), ControlError> {
    if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    Ok(())
}

fn validate_string(value: &str, minimum: usize, maximum: usize) -> Result<(), ControlError> {
    if value.len() < minimum || value.len() > maximum || value.contains('\0') {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    Ok(())
}

fn validate_bytes(value: &[u8], maximum: usize) -> Result<(), ControlError> {
    if value.len() > maximum {
        return Err(ControlError::InvalidDataPlaneRecord);
    }
    Ok(())
}

fn encode_role(role: PromptRole) -> u8 {
    match role {
        PromptRole::System => 1,
        PromptRole::User => 2,
        PromptRole::Assistant => 3,
    }
}

fn decode_role(value: u8) -> Result<PromptRole, ControlError> {
    match value {
        1 => Ok(PromptRole::System),
        2 => Ok(PromptRole::User),
        3 => Ok(PromptRole::Assistant),
        _ => Err(ControlError::InvalidDataPlaneRecord),
    }
}

fn encode_finish_reason(reason: CompletionFinishReason) -> u8 {
    match reason {
        CompletionFinishReason::Stop => 1,
        CompletionFinishReason::MaxTokens => 2,
        CompletionFinishReason::Refusal => 3,
        CompletionFinishReason::Other => 4,
    }
}

fn decode_finish_reason(value: u8) -> Result<CompletionFinishReason, ControlError> {
    match value {
        1 => Ok(CompletionFinishReason::Stop),
        2 => Ok(CompletionFinishReason::MaxTokens),
        3 => Ok(CompletionFinishReason::Refusal),
        4 => Ok(CompletionFinishReason::Other),
        _ => Err(ControlError::InvalidDataPlaneRecord),
    }
}

fn encode_failure(failure: DataPlaneFailure) -> u8 {
    match failure {
        DataPlaneFailure::InvalidRequest => 1,
        DataPlaneFailure::Unauthorized => 2,
        DataPlaneFailure::Channel => 3,
        DataPlaneFailure::Completion => 4,
        DataPlaneFailure::Unavailable => 5,
        DataPlaneFailure::Internal => 6,
    }
}

fn decode_failure(value: u8) -> Result<DataPlaneFailure, ControlError> {
    match value {
        1 => Ok(DataPlaneFailure::InvalidRequest),
        2 => Ok(DataPlaneFailure::Unauthorized),
        3 => Ok(DataPlaneFailure::Channel),
        4 => Ok(DataPlaneFailure::Completion),
        5 => Ok(DataPlaneFailure::Unavailable),
        6 => Ok(DataPlaneFailure::Internal),
        _ => Err(ControlError::InvalidDataPlaneRecord),
    }
}

#[derive(Default)]
struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn ensure_record_bound(&self) -> Result<(), ControlError> {
        if self.bytes.len() > MAX_DATA_PLANE_RECORD_BYTES {
            Err(ControlError::InvalidDataPlaneRecord)
        } else {
            Ok(())
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn fixed(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), ControlError> {
        let length =
            u32::try_from(value.len()).map_err(|_| ControlError::InvalidDataPlaneRecord)?;
        self.u32(length);
        self.fixed(value);
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), ControlError> {
        self.bytes(value.as_bytes())
    }

    fn optional_string(&mut self, value: Option<&str>, maximum: usize) -> Result<(), ControlError> {
        match value {
            Some(value) => {
                validate_string(value, 0, maximum)?;
                self.u8(1);
                self.string(value)
            }
            None => {
                self.u8(0);
                Ok(())
            }
        }
    }

    fn optional_u32(&mut self, value: Option<u32>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.u32(value);
            }
            None => self.u8(0),
        }
    }

    fn optional_u64(&mut self, value: Option<u64>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.u64(value);
            }
            None => self.u8(0),
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ControlError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ControlError::InvalidDataPlaneRecord)?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or(ControlError::InvalidDataPlaneRecord)?;
        self.offset = end;
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, ControlError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ControlError> {
        let bytes = self
            .take(2)?
            .try_into()
            .map_err(|_| ControlError::InvalidDataPlaneRecord)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, ControlError> {
        let bytes = self
            .take(4)?
            .try_into()
            .map_err(|_| ControlError::InvalidDataPlaneRecord)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, ControlError> {
        let bytes = self
            .take(8)?
            .try_into()
            .map_err(|_| ControlError::InvalidDataPlaneRecord)?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn bytes(&mut self, maximum: usize) -> Result<Vec<u8>, ControlError> {
        let length = self.u32()? as usize;
        if length > maximum {
            return Err(ControlError::InvalidDataPlaneRecord);
        }
        Ok(self.take(length)?.to_vec())
    }

    fn string(&mut self, minimum: usize, maximum: usize) -> Result<String, ControlError> {
        let bytes = self.bytes(maximum)?;
        let value = String::from_utf8(bytes).map_err(|_| ControlError::InvalidDataPlaneRecord)?;
        validate_string(&value, minimum, maximum)?;
        Ok(value)
    }

    fn optional_string(&mut self, maximum: usize) -> Result<Option<String>, ControlError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.string(0, maximum)?)),
            _ => Err(ControlError::InvalidDataPlaneRecord),
        }
    }

    fn optional_u32(&mut self) -> Result<Option<u32>, ControlError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u32()?)),
            _ => Err(ControlError::InvalidDataPlaneRecord),
        }
    }

    fn optional_u64(&mut self) -> Result<Option<u64>, ControlError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u64()?)),
            _ => Err(ControlError::InvalidDataPlaneRecord),
        }
    }

    fn uuid_v7(&mut self) -> Result<[u8; 16], ControlError> {
        let bytes: [u8; 16] = self
            .take(16)?
            .try_into()
            .map_err(|_| ControlError::InvalidDataPlaneRecord)?;
        validate_uuid_v7(&bytes)?;
        Ok(bytes)
    }

    fn finish(self) -> Result<(), ControlError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(ControlError::InvalidDataPlaneRecord)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid_v7(last: u8) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        bytes
    }

    fn call_with_all_roles() -> CompletionCall {
        CompletionCall {
            model: "model".to_string(),
            system: None,
            messages: vec![
                PromptMessage {
                    role: PromptRole::System,
                    text: "system".to_string(),
                },
                PromptMessage {
                    role: PromptRole::User,
                    text: "user".to_string(),
                },
                PromptMessage {
                    role: PromptRole::Assistant,
                    text: "assistant".to_string(),
                },
            ],
            temperature: 2.0,
            max_tokens: None,
            stop_sequences: Vec::new(),
            seed: None,
            metadata: BTreeMap::new(),
        }
    }

    fn result(reason: CompletionFinishReason) -> CompletionResult {
        CompletionResult {
            text: "result".to_string(),
            model: "model".to_string(),
            provider: CompletionProvider {
                vendor: "vendor".to_string(),
                model_family: "family".to_string(),
                model_version: "version".to_string(),
                endpoint: None,
            },
            usage: CompletionUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 3,
            },
            finish_reason: reason,
            latency_ms: 4,
        }
    }

    #[test]
    fn every_enum_value_and_optional_absence_round_trips() {
        let request = DataRecord::Request(DataPlaneRequest::Complete {
            id: RequestId::new(1).unwrap(),
            call: call_with_all_roles(),
        });
        let (tag, body) = encode(&request).unwrap();
        assert_eq!(decode(tag, &body).unwrap(), request);

        for reason in [
            CompletionFinishReason::Stop,
            CompletionFinishReason::MaxTokens,
            CompletionFinishReason::Refusal,
            CompletionFinishReason::Other,
        ] {
            let response = DataRecord::Response(DataPlaneResponse::Completed {
                id: RequestId::new(2).unwrap(),
                result: Box::new(result(reason)),
            });
            let (tag, body) = encode(&response).unwrap();
            assert_eq!(decode(tag, &body).unwrap(), response);
        }

        for failure in [
            DataPlaneFailure::InvalidRequest,
            DataPlaneFailure::Unauthorized,
            DataPlaneFailure::Channel,
            DataPlaneFailure::Completion,
            DataPlaneFailure::Unavailable,
            DataPlaneFailure::Internal,
        ] {
            let response = DataRecord::Response(DataPlaneResponse::Failed {
                id: RequestId::new(3).unwrap(),
                failure,
            });
            let (tag, body) = encode(&response).unwrap();
            assert_eq!(decode(tag, &body).unwrap(), response);
        }
    }

    #[test]
    fn primitive_decoders_reject_noncanonical_discriminants_and_input() {
        assert_eq!(RequestId::new(0), Err(ControlError::InvalidDataPlaneRecord));
        assert_eq!(decode_role(0), Err(ControlError::InvalidDataPlaneRecord));
        assert_eq!(
            decode_finish_reason(0),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        assert_eq!(decode_failure(0), Err(ControlError::InvalidDataPlaneRecord));
        assert_eq!(
            decode(99, &1u64.to_be_bytes()),
            Err(ControlError::UnknownMessageKind)
        );

        let mut invalid_optional = Decoder::new(&[2]);
        assert_eq!(
            invalid_optional.optional_string(10),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        let mut invalid_optional = Decoder::new(&[2]);
        assert_eq!(
            invalid_optional.optional_u32(),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        let mut invalid_optional = Decoder::new(&[2]);
        assert_eq!(
            invalid_optional.optional_u64(),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        let mut invalid_utf8 = Decoder::new(&[0, 0, 0, 1, 0xff]);
        assert_eq!(
            invalid_utf8.string(0, 1),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        let mut truncated = Decoder::new(&[0]);
        assert_eq!(truncated.u64(), Err(ControlError::InvalidDataPlaneRecord));
    }

    #[test]
    fn static_bounds_reject_invalid_requests_and_responses() {
        let id = RequestId::new(1).unwrap();
        let invalid_requests = [
            DataPlaneRequest::Receive {
                id,
                channel_id: uuid_v7(1),
                limit: 0,
            },
            DataPlaneRequest::Receive {
                id,
                channel_id: [0; 16],
                limit: 1,
            },
            DataPlaneRequest::Publish {
                id,
                channel_id: uuid_v7(1),
                content_type: String::new(),
                payload: Vec::new(),
            },
            DataPlaneRequest::Publish {
                id,
                channel_id: uuid_v7(1),
                content_type: "text/plain".to_string(),
                payload: vec![0; MAX_DATA_PLANE_PAYLOAD_BYTES + 1],
            },
            DataPlaneRequest::Acknowledge {
                id,
                channel_id: uuid_v7(1),
                message_id: [0; 16],
            },
        ];
        for request in invalid_requests {
            assert_eq!(
                encode(&DataRecord::Request(request)),
                Err(ControlError::InvalidDataPlaneRecord)
            );
        }

        let too_many_messages = DataPlaneResponse::Received {
            id,
            messages: (0..=MAX_DATA_PLANE_MESSAGES)
                .map(|index| DataPlaneMessage {
                    message_id: uuid_v7(index as u8),
                    sequence: index as u64,
                    timestamp_ns: 0,
                    content_type: "text/plain".to_string(),
                    payload: Vec::new(),
                })
                .collect(),
        };
        assert_eq!(
            encode(&DataRecord::Response(too_many_messages)),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        let aggregate_too_large = DataPlaneResponse::Received {
            id,
            messages: [1, 2]
                .into_iter()
                .map(|last| DataPlaneMessage {
                    message_id: uuid_v7(last),
                    sequence: u64::from(last),
                    timestamp_ns: 0,
                    content_type: "application/octet-stream".to_string(),
                    payload: vec![0; MAX_DATA_PLANE_PAYLOAD_BYTES],
                })
                .collect(),
        };
        assert_eq!(
            encode(&DataRecord::Response(aggregate_too_large)),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        let invalid_published = DataPlaneResponse::Published {
            id,
            message_id: [0; 16],
            sequence: 0,
            timestamp_ns: 0,
        };
        assert_eq!(
            encode(&DataRecord::Response(invalid_published)),
            Err(ControlError::InvalidDataPlaneRecord)
        );
    }

    #[test]
    fn completion_bounds_fail_closed() {
        let id = RequestId::new(1).unwrap();
        let invalid_calls = [
            CompletionCall {
                model: String::new(),
                ..call_with_all_roles()
            },
            CompletionCall {
                messages: Vec::new(),
                ..call_with_all_roles()
            },
            CompletionCall {
                temperature: f32::NAN,
                ..call_with_all_roles()
            },
            CompletionCall {
                max_tokens: Some(0),
                ..call_with_all_roles()
            },
            CompletionCall {
                stop_sequences: vec!["x".to_string(); MAX_STOP_SEQUENCES + 1],
                ..call_with_all_roles()
            },
            CompletionCall {
                metadata: (0..=MAX_METADATA_ENTRIES)
                    .map(|index| (format!("key-{index}"), String::new()))
                    .collect(),
                ..call_with_all_roles()
            },
        ];
        for call in invalid_calls {
            assert_eq!(
                encode(&DataRecord::Request(DataPlaneRequest::Complete {
                    id,
                    call,
                })),
                Err(ControlError::InvalidDataPlaneRecord)
            );
        }

        let mut invalid_result = result(CompletionFinishReason::Stop);
        invalid_result.provider.vendor = String::new();
        assert_eq!(
            encode(&DataRecord::Response(DataPlaneResponse::Completed {
                id,
                result: Box::new(invalid_result),
            })),
            Err(ControlError::InvalidDataPlaneRecord)
        );
    }
}
