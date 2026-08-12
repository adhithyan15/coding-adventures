//! Authenticated D18 host lifecycle and bounded data-plane protocol.
//!
//! The secure host channel provides opaque authenticated bytes. This crate adds
//! the minimum lifecycle state machine required by process supervision without
//! opening streams, polling clocks, or spawning processes.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_secure_host_channel::{ChannelError, ChannelRole, SecureHostChannel, SessionId};
use core::fmt::{self, Display, Formatter};

mod data_plane;
mod launch;

pub use data_plane::{
    validate_data_plane_response, CompletionCall, CompletionFinishReason, CompletionProvider,
    CompletionResult, CompletionUsage, DataPlaneFailure, DataPlaneMessage, DataPlaneOperation,
    DataPlaneRequest, DataPlaneResponse, ModelToolCall, ModelToolChoice, ModelToolDefinition,
    ModelToolResult, PromptMessage, PromptRole, RequestId, ToolCompletionCall,
    ToolCompletionOutput, ToolCompletionResult, MAX_DATA_PLANE_MESSAGES,
    MAX_DATA_PLANE_PAYLOAD_BYTES, MAX_DATA_PLANE_RECORD_BYTES, MAX_MODEL_TOOLS,
    MAX_MODEL_TOOL_CALL_ID_BYTES, MAX_MODEL_TOOL_DESCRIPTION_BYTES, MAX_MODEL_TOOL_JSON_BYTES,
    MAX_MODEL_TOOL_NAME_BYTES,
};
use data_plane::{DataRecord, ACKNOWLEDGED_RESPONSE_TAG, ACKNOWLEDGE_REQUEST_TAG};
use data_plane::{
    COMPLETED_RESPONSE_TAG, COMPLETE_REQUEST_TAG, COMPLETE_WITH_TOOLS_REQUEST_TAG,
    EXECUTE_TOOL_REQUEST_TAG, FAILED_RESPONSE_TAG, LIST_MODEL_TOOLS_REQUEST_TAG,
    MODEL_TOOLS_LISTED_RESPONSE_TAG, PUBLISHED_RESPONSE_TAG, PUBLISH_REQUEST_TAG,
    RECEIVED_RESPONSE_TAG, RECEIVE_REQUEST_TAG, TOOL_COMPLETED_RESPONSE_TAG,
    TOOL_EXECUTED_RESPONSE_TAG,
};
use launch::{decode_launch_bindings, encode_launch_bindings};
pub use launch::{
    ChannelBinding, ChannelBindingAccess, LaunchBindings, LevelOneModelBinding,
    MAX_LAUNCH_CHANNEL_BINDINGS, MAX_LAUNCH_CHANNEL_NAME_BYTES, MAX_LAUNCH_COMPLETION_TOKENS,
    MAX_LAUNCH_MODEL_BYTES,
};

const MAGIC: &[u8; 4] = b"D18C";
const VERSION: u8 = 1;
const READY_TAG: u8 = 1;
const HEARTBEAT_TAG: u8 = 2;
const TERMINATE_TAG: u8 = 3;
const PACKAGE_TRUST_TAG: u8 = 4;
const LAUNCH_BINDINGS_TAG: u8 = 5;
const HEADER_BYTES: usize = 6;
const READY_BYTES: usize = HEADER_BYTES + 32;
const MAX_PACKAGE_KEY_ID_BYTES: usize = 128;
const PACKAGE_TRUST_FIXED_BYTES: usize = 1 + 1 + 32;

/// Observable state of one authenticated control endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlState {
    /// The child has not yet confirmed its independently verified package.
    AwaitingReady,
    /// Package identity matched and authenticated heartbeats are accepted.
    Running,
    /// Graceful termination was requested and no further messages are allowed.
    Terminating,
    /// A peer, codec, identity, or secure-channel failure closed the endpoint.
    Closed,
}

/// Authenticated child event with a supervisor-trusted receipt time.
#[derive(Clone, Debug, PartialEq)]
pub enum ChildEvent {
    /// The child independently verified and started the exact package hash.
    Ready {
        /// SHA-256 package identity confirmed by the child.
        package_hash: [u8; 32],
        /// Monotonic time sampled by the supervising caller after receipt.
        received_at_ns: u64,
    },
    /// The ready child remains responsive on the authenticated channel.
    Heartbeat {
        /// Monotonic time sampled by the supervising caller after receipt.
        received_at_ns: u64,
    },
    /// One correlated data-plane request from a ready child.
    Request(DataPlaneRequest),
}

/// Authenticated orchestrator event accepted by a child.
#[derive(Clone, Debug, PartialEq)]
pub enum OrchestratorEvent {
    /// The exact package-signing trust selected by the supervising parent.
    PackageTrust(PackageTrust),
    /// Pipeline-authorized channel UUIDs and optional Level 1 model settings.
    LaunchBindings(LaunchBindings),
    /// Begin graceful host shutdown.
    Terminate,
    /// One exactly correlated response to the child's pending request.
    Response(DataPlaneResponse),
}

/// Bounded host-control failure with input-independent diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlError {
    /// The secure channel belongs to the opposite endpoint role.
    WrongChannelRole,
    /// A secure-channel authentication, identity, sequence, or codec failure.
    Channel(ChannelError),
    /// The plaintext control record was truncated, padded, or otherwise malformed.
    MalformedRecord,
    /// The plaintext control version is unsupported.
    UnsupportedVersion,
    /// The plaintext control message tag is unknown.
    UnknownMessageKind,
    /// A data-plane body violated its static shape, UTF-8, UUID, or size bounds.
    InvalidDataPlaneRecord,
    /// Package-signing trust violated its closed key, tier, or identifier contract.
    InvalidPackageTrust,
    /// Launch bindings violated their name, UUID, model, uniqueness, or size contract.
    InvalidLaunchBindings,
    /// The peer sent a valid message kind owned by the opposite direction.
    WrongMessageDirection,
    /// The operation violates readiness or termination ordering.
    InvalidState,
    /// The child's independently verified package differs from registration.
    PackageMismatch,
    /// A response identity or successful response operation mismatched the request.
    CorrelationMismatch,
    /// The serialized protocol permits only one outstanding data-plane request.
    RequestInFlight,
    /// The endpoint exhausted the non-zero request identity space.
    RequestIdExhausted,
    /// A prior terminal failure permanently closed this endpoint.
    Closed,
}

impl Display for ControlError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongChannelRole => "host-control: wrong secure-channel role",
            Self::Channel(_) => "host-control: secure-channel failure",
            Self::MalformedRecord => "host-control: malformed record",
            Self::UnsupportedVersion => "host-control: unsupported version",
            Self::UnknownMessageKind => "host-control: unknown message kind",
            Self::InvalidDataPlaneRecord => "host-control: invalid data-plane record",
            Self::InvalidPackageTrust => "host-control: invalid package trust",
            Self::InvalidLaunchBindings => "host-control: invalid launch bindings",
            Self::WrongMessageDirection => "host-control: wrong message direction",
            Self::InvalidState => "host-control: invalid lifecycle state",
            Self::PackageMismatch => "host-control: package identity mismatch",
            Self::CorrelationMismatch => "host-control: data-plane correlation mismatch",
            Self::RequestInFlight => "host-control: data-plane request already in flight",
            Self::RequestIdExhausted => "host-control: request identity space exhausted",
            Self::Closed => "host-control: endpoint is closed",
        })
    }
}

impl std::error::Error for ControlError {}

/// Signing-key class carried to the child before package readiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageTrustType {
    /// Operator-controlled production package key.
    Production,
    /// Local developer key, restricted to Tier 0 or Tier 1 packages.
    Developer,
    /// Independently operated third-party package key.
    ThirdParty,
}

/// Exact public package trust authenticated to one child session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageTrust {
    key_id: String,
    key_type: PackageTrustType,
    public_key: [u8; 32],
    maximum_tier: u8,
}

impl PackageTrust {
    /// Validate one package-signing public key and its maximum privilege tier.
    pub fn new(
        key_id: impl Into<String>,
        key_type: PackageTrustType,
        public_key: [u8; 32],
        maximum_tier: u8,
    ) -> Result<Self, ControlError> {
        let key_id = key_id.into();
        if key_id.is_empty()
            || key_id.len() > MAX_PACKAGE_KEY_ID_BYTES
            || !key_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || maximum_tier > 3
            || (key_type == PackageTrustType::Developer && maximum_tier > 1)
        {
            return Err(ControlError::InvalidPackageTrust);
        }
        Ok(Self {
            key_id,
            key_type,
            public_key,
            maximum_tier,
        })
    }

    /// Return the stable package key identifier.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Return the authenticated package trust class.
    pub fn key_type(&self) -> PackageTrustType {
        self.key_type
    }

    /// Return the raw Ed25519 public verification key.
    pub fn public_key(&self) -> [u8; 32] {
        self.public_key
    }

    /// Return the maximum package privilege tier accepted for this key.
    pub fn maximum_tier(&self) -> u8 {
        self.maximum_tier
    }
}

/// Orchestrator-side lifecycle wrapper around one secure host channel.
pub struct OrchestratorControl {
    channel: SecureHostChannel,
    expected_package_hash: [u8; 32],
    state: ControlState,
    trust_sent: bool,
    launch_bindings_sent: bool,
    last_request_id: u64,
    pending_request: Option<(RequestId, DataPlaneOperation)>,
}

impl OrchestratorControl {
    /// Bind an orchestrator channel to the immutable registered package identity.
    pub fn new(
        channel: SecureHostChannel,
        expected_package_hash: [u8; 32],
    ) -> Result<Self, ControlError> {
        if channel.role() != ChannelRole::Orchestrator {
            return Err(ControlError::WrongChannelRole);
        }
        Ok(Self {
            channel,
            expected_package_hash,
            state: ControlState::AwaitingReady,
            trust_sent: false,
            launch_bindings_sent: false,
            last_request_id: 0,
            pending_request: None,
        })
    }

    /// Authenticate and apply one exact next child record.
    ///
    /// `received_at_ns` must come from the supervising process's trusted monotonic
    /// clock after the encrypted frame was received; it is never child evidence.
    pub fn receive_child(
        &mut self,
        frame: &[u8],
        received_at_ns: u64,
    ) -> Result<ChildEvent, ControlError> {
        self.ensure_peer_input_allowed()?;
        let plaintext = match self.channel.receive(frame) {
            Ok(plaintext) => plaintext,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        let record = match decode_record(&plaintext) {
            Ok(record) => record,
            Err(error) => return Err(self.close(error)),
        };
        match (self.state, record) {
            (ControlState::AwaitingReady, ControlRecord::Ready(package_hash))
                if self.trust_sent && self.launch_bindings_sent =>
            {
                if package_hash != self.expected_package_hash {
                    return Err(self.close(ControlError::PackageMismatch));
                }
                self.state = ControlState::Running;
                Ok(ChildEvent::Ready {
                    package_hash,
                    received_at_ns,
                })
            }
            (ControlState::Running, ControlRecord::Heartbeat) => {
                Ok(ChildEvent::Heartbeat { received_at_ns })
            }
            (ControlState::Running, ControlRecord::Request(request)) => {
                let expected = self
                    .last_request_id
                    .checked_add(1)
                    .ok_or_else(|| self.close(ControlError::CorrelationMismatch))?;
                if self.pending_request.is_some() || request.id().get() != expected {
                    return Err(self.close(ControlError::CorrelationMismatch));
                }
                self.last_request_id = request.id().get();
                self.pending_request = Some((request.id(), request.operation()));
                Ok(ChildEvent::Request(request))
            }
            (
                _,
                ControlRecord::PackageTrust(_)
                | ControlRecord::LaunchBindings(_)
                | ControlRecord::Terminate
                | ControlRecord::Response(_),
            ) => Err(self.close(ControlError::WrongMessageDirection)),
            _ => Err(self.close(ControlError::InvalidState)),
        }
    }

    /// Authenticate the exact relevant package-signing trust before readiness.
    pub fn provide_package_trust(&mut self, trust: PackageTrust) -> Result<Vec<u8>, ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state != ControlState::AwaitingReady || self.trust_sent {
            return Err(ControlError::InvalidState);
        }
        let plaintext = encode_record(ControlRecord::PackageTrust(trust))?;
        let frame = match self.channel.send(&plaintext) {
            Ok(frame) => frame,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        self.trust_sent = true;
        Ok(frame)
    }

    /// Authenticate pipeline-authorized launch bindings after package trust and before readiness.
    pub fn provide_launch_bindings(
        &mut self,
        bindings: LaunchBindings,
    ) -> Result<Vec<u8>, ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state != ControlState::AwaitingReady
            || !self.trust_sent
            || self.launch_bindings_sent
        {
            return Err(ControlError::InvalidState);
        }
        let plaintext = encode_record(ControlRecord::LaunchBindings(bindings))?;
        let frame = match self.channel.send(&plaintext) {
            Ok(frame) => frame,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        self.launch_bindings_sent = true;
        Ok(frame)
    }

    /// Encrypt the response to the one pending child request.
    pub fn respond(&mut self, response: DataPlaneResponse) -> Result<Vec<u8>, ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state != ControlState::Running {
            return Err(ControlError::InvalidState);
        }
        let Some((expected_id, expected_operation)) = self.pending_request else {
            return Err(ControlError::CorrelationMismatch);
        };
        if response.id() != expected_id
            || response
                .operation()
                .is_some_and(|operation| operation != expected_operation)
        {
            return Err(ControlError::CorrelationMismatch);
        }
        let plaintext = encode_record(ControlRecord::Response(response))?;
        let frame = match self.channel.send(&plaintext) {
            Ok(frame) => frame,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        self.pending_request = None;
        Ok(frame)
    }

    /// Borrow the one request awaiting an orchestrator response.
    pub fn pending_request(&self) -> Option<(RequestId, DataPlaneOperation)> {
        self.pending_request
    }

    /// Encrypt one graceful-termination request and stop accepting messages.
    pub fn terminate(&mut self) -> Result<Vec<u8>, ControlError> {
        match self.state {
            ControlState::AwaitingReady | ControlState::Running => {}
            ControlState::Closed => return Err(ControlError::Closed),
            ControlState::Terminating => return Err(ControlError::InvalidState),
        }
        let plaintext = encode_record(ControlRecord::Terminate)?;
        let frame = match self.channel.send(&plaintext) {
            Ok(frame) => frame,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        self.state = ControlState::Terminating;
        Ok(frame)
    }

    /// Return the secure UUID-v7 session identity used as registry channel evidence.
    pub fn session_id(&self) -> SessionId {
        self.channel.session_id()
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> ControlState {
        self.state
    }

    fn ensure_peer_input_allowed(&mut self) -> Result<(), ControlError> {
        match self.state {
            ControlState::Closed => Err(ControlError::Closed),
            ControlState::Terminating => Err(self.close(ControlError::InvalidState)),
            ControlState::AwaitingReady | ControlState::Running => Ok(()),
        }
    }

    fn close(&mut self, error: ControlError) -> ControlError {
        self.state = ControlState::Closed;
        error
    }
}

/// Child-side lifecycle wrapper around one secure host channel.
pub struct ChildControl {
    channel: SecureHostChannel,
    state: ControlState,
    trust_received: bool,
    launch_bindings_received: bool,
    next_request_id: Option<u64>,
    pending_request: Option<(RequestId, DataPlaneOperation)>,
}

impl ChildControl {
    /// Construct a child endpoint before package readiness is announced.
    pub fn new(channel: SecureHostChannel) -> Result<Self, ControlError> {
        if channel.role() != ChannelRole::Child {
            return Err(ControlError::WrongChannelRole);
        }
        Ok(Self {
            channel,
            state: ControlState::AwaitingReady,
            trust_received: false,
            launch_bindings_received: false,
            next_request_id: Some(1),
            pending_request: None,
        })
    }

    /// Encrypt the first readiness record after independent package verification.
    pub fn ready(&mut self, package_hash: [u8; 32]) -> Result<Vec<u8>, ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state != ControlState::AwaitingReady
            || !self.trust_received
            || !self.launch_bindings_received
        {
            return Err(ControlError::InvalidState);
        }
        let plaintext = encode_record(ControlRecord::Ready(package_hash))?;
        let frame = match self.channel.send(&plaintext) {
            Ok(frame) => frame,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        self.state = ControlState::Running;
        Ok(frame)
    }

    /// Encrypt one heartbeat after readiness.
    pub fn heartbeat(&mut self) -> Result<Vec<u8>, ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state != ControlState::Running {
            return Err(ControlError::InvalidState);
        }
        let plaintext = encode_record(ControlRecord::Heartbeat)?;
        match self.channel.send(&plaintext) {
            Ok(frame) => Ok(frame),
            Err(error) => Err(self.close(ControlError::Channel(error))),
        }
    }

    /// Encrypt one bounded channel receive request.
    pub fn request_receive(
        &mut self,
        channel_id: [u8; 16],
        limit: u16,
    ) -> Result<(RequestId, Vec<u8>), ControlError> {
        self.send_request(|id| DataPlaneRequest::Receive {
            id,
            channel_id,
            limit,
        })
    }

    /// Encrypt one bounded channel publish request.
    pub fn request_publish(
        &mut self,
        channel_id: [u8; 16],
        content_type: String,
        payload: Vec<u8>,
    ) -> Result<(RequestId, Vec<u8>), ControlError> {
        self.send_request(|id| DataPlaneRequest::Publish {
            id,
            channel_id,
            content_type,
            payload,
        })
    }

    /// Encrypt one delivered-message acknowledgement request.
    pub fn request_acknowledge(
        &mut self,
        channel_id: [u8; 16],
        message_id: [u8; 16],
    ) -> Result<(RequestId, Vec<u8>), ControlError> {
        self.send_request(|id| DataPlaneRequest::Acknowledge {
            id,
            channel_id,
            message_id,
        })
    }

    /// Encrypt one provider-neutral completion request.
    pub fn request_completion(
        &mut self,
        call: CompletionCall,
    ) -> Result<(RequestId, Vec<u8>), ControlError> {
        self.send_request(|id| DataPlaneRequest::Complete { id, call })
    }

    /// Encrypt one provider-neutral tool-aware completion request.
    pub fn request_tool_completion(
        &mut self,
        call: ToolCompletionCall,
    ) -> Result<(RequestId, Vec<u8>), ControlError> {
        self.send_request(|id| DataPlaneRequest::CompleteWithTools {
            id,
            call: Box::new(call),
        })
    }

    /// Encrypt one parent-owned D18D execution request for a model-returned call.
    pub fn request_tool_execution(
        &mut self,
        call: ModelToolCall,
    ) -> Result<(RequestId, Vec<u8>), ControlError> {
        self.send_request(|id| DataPlaneRequest::ExecuteTool {
            id,
            call: Box::new(call),
        })
    }

    /// Encrypt one request for the exact parent-installed model tool catalog.
    pub fn request_model_tools(&mut self) -> Result<(RequestId, Vec<u8>), ControlError> {
        self.send_request(|id| DataPlaneRequest::ListModelTools { id })
    }

    /// Authenticate and apply one exact next orchestrator record.
    pub fn receive_orchestrator(
        &mut self,
        frame: &[u8],
    ) -> Result<OrchestratorEvent, ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state == ControlState::Terminating {
            return Err(self.close(ControlError::InvalidState));
        }
        let plaintext = match self.channel.receive(frame) {
            Ok(plaintext) => plaintext,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        let record = match decode_record(&plaintext) {
            Ok(record) => record,
            Err(error) => return Err(self.close(error)),
        };
        match record {
            ControlRecord::PackageTrust(trust) => {
                if self.state != ControlState::AwaitingReady
                    || self.trust_received
                    || self.launch_bindings_received
                {
                    return Err(self.close(ControlError::InvalidState));
                }
                self.trust_received = true;
                Ok(OrchestratorEvent::PackageTrust(trust))
            }
            ControlRecord::LaunchBindings(bindings) => {
                if self.state != ControlState::AwaitingReady
                    || !self.trust_received
                    || self.launch_bindings_received
                {
                    return Err(self.close(ControlError::InvalidState));
                }
                self.launch_bindings_received = true;
                Ok(OrchestratorEvent::LaunchBindings(bindings))
            }
            ControlRecord::Terminate => {
                self.state = ControlState::Terminating;
                Ok(OrchestratorEvent::Terminate)
            }
            ControlRecord::Response(response) => {
                if self.state != ControlState::Running {
                    return Err(self.close(ControlError::InvalidState));
                }
                let Some((expected_id, expected_operation)) = self.pending_request else {
                    return Err(self.close(ControlError::CorrelationMismatch));
                };
                if response.id() != expected_id
                    || response
                        .operation()
                        .is_some_and(|operation| operation != expected_operation)
                {
                    return Err(self.close(ControlError::CorrelationMismatch));
                }
                self.pending_request = None;
                Ok(OrchestratorEvent::Response(response))
            }
            ControlRecord::Ready(_) | ControlRecord::Heartbeat | ControlRecord::Request(_) => {
                Err(self.close(ControlError::WrongMessageDirection))
            }
        }
    }

    /// Return the secure UUID-v7 session identity used as registry channel evidence.
    pub fn session_id(&self) -> SessionId {
        self.channel.session_id()
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> ControlState {
        self.state
    }

    /// Borrow the one request awaiting an orchestrator response.
    pub fn pending_request(&self) -> Option<(RequestId, DataPlaneOperation)> {
        self.pending_request
    }

    fn send_request(
        &mut self,
        build: impl FnOnce(RequestId) -> DataPlaneRequest,
    ) -> Result<(RequestId, Vec<u8>), ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state != ControlState::Running {
            return Err(ControlError::InvalidState);
        }
        if self.pending_request.is_some() {
            return Err(ControlError::RequestInFlight);
        }
        let value = self
            .next_request_id
            .ok_or(ControlError::RequestIdExhausted)?;
        let id = RequestId::new(value)?;
        let request = build(id);
        let operation = request.operation();
        let plaintext = encode_record(ControlRecord::Request(request))?;
        let frame = match self.channel.send(&plaintext) {
            Ok(frame) => frame,
            Err(error) => return Err(self.close(ControlError::Channel(error))),
        };
        self.next_request_id = value.checked_add(1);
        self.pending_request = Some((id, operation));
        Ok((id, frame))
    }

    fn close(&mut self, error: ControlError) -> ControlError {
        self.state = ControlState::Closed;
        error
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ControlRecord {
    Ready([u8; 32]),
    Heartbeat,
    Terminate,
    PackageTrust(PackageTrust),
    LaunchBindings(LaunchBindings),
    Request(DataPlaneRequest),
    Response(DataPlaneResponse),
}

fn encode_record(record: ControlRecord) -> Result<Vec<u8>, ControlError> {
    let mut output = Vec::with_capacity(match &record {
        ControlRecord::Ready(_) => READY_BYTES,
        ControlRecord::Heartbeat | ControlRecord::Terminate => HEADER_BYTES,
        ControlRecord::PackageTrust(trust) => {
            HEADER_BYTES + 1 + trust.key_id.len() + PACKAGE_TRUST_FIXED_BYTES
        }
        ControlRecord::LaunchBindings(bindings) => {
            HEADER_BYTES + encode_launch_bindings(bindings).len()
        }
        ControlRecord::Request(_) | ControlRecord::Response(_) => HEADER_BYTES + 128,
    });
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    match record {
        ControlRecord::Ready(package_hash) => {
            output.push(READY_TAG);
            output.extend_from_slice(&package_hash);
        }
        ControlRecord::Heartbeat => output.push(HEARTBEAT_TAG),
        ControlRecord::Terminate => output.push(TERMINATE_TAG),
        ControlRecord::PackageTrust(trust) => {
            output.push(PACKAGE_TRUST_TAG);
            output.push(trust.key_id.len() as u8);
            output.extend_from_slice(trust.key_id.as_bytes());
            output.push(match trust.key_type {
                PackageTrustType::Production => 1,
                PackageTrustType::Developer => 2,
                PackageTrustType::ThirdParty => 3,
            });
            output.push(trust.maximum_tier);
            output.extend_from_slice(&trust.public_key);
        }
        ControlRecord::LaunchBindings(bindings) => {
            output.push(LAUNCH_BINDINGS_TAG);
            output.extend_from_slice(&encode_launch_bindings(&bindings));
        }
        ControlRecord::Request(request) => {
            let (tag, body) = data_plane::encode(&DataRecord::Request(request))?;
            output.push(tag);
            output.extend_from_slice(&body);
        }
        ControlRecord::Response(response) => {
            let (tag, body) = data_plane::encode(&DataRecord::Response(response))?;
            output.push(tag);
            output.extend_from_slice(&body);
        }
    }
    Ok(output)
}

fn decode_record(bytes: &[u8]) -> Result<ControlRecord, ControlError> {
    let header = bytes
        .get(..HEADER_BYTES)
        .ok_or(ControlError::MalformedRecord)?;
    if &header[..4] != MAGIC {
        return Err(ControlError::MalformedRecord);
    }
    if header[4] != VERSION {
        return Err(ControlError::UnsupportedVersion);
    }
    match header[5] {
        READY_TAG => {
            let package_hash = bytes
                .get(HEADER_BYTES..READY_BYTES)
                .and_then(|body| body.try_into().ok())
                .ok_or(ControlError::MalformedRecord)?;
            if bytes.len() != READY_BYTES {
                return Err(ControlError::MalformedRecord);
            }
            Ok(ControlRecord::Ready(package_hash))
        }
        HEARTBEAT_TAG => {
            if bytes.len() != HEADER_BYTES {
                return Err(ControlError::MalformedRecord);
            }
            Ok(ControlRecord::Heartbeat)
        }
        TERMINATE_TAG => {
            if bytes.len() != HEADER_BYTES {
                return Err(ControlError::MalformedRecord);
            }
            Ok(ControlRecord::Terminate)
        }
        PACKAGE_TRUST_TAG => {
            let body = &bytes[HEADER_BYTES..];
            let key_id_length = *body.first().ok_or(ControlError::InvalidPackageTrust)? as usize;
            if key_id_length == 0 || key_id_length > MAX_PACKAGE_KEY_ID_BYTES {
                return Err(ControlError::InvalidPackageTrust);
            }
            let expected_length = 1 + key_id_length + PACKAGE_TRUST_FIXED_BYTES;
            if body.len() != expected_length {
                return Err(ControlError::InvalidPackageTrust);
            }
            let key_id = std::str::from_utf8(&body[1..1 + key_id_length])
                .map_err(|_| ControlError::InvalidPackageTrust)?;
            let key_type = match body[1 + key_id_length] {
                1 => PackageTrustType::Production,
                2 => PackageTrustType::Developer,
                3 => PackageTrustType::ThirdParty,
                _ => return Err(ControlError::InvalidPackageTrust),
            };
            let maximum_tier = body[2 + key_id_length];
            let public_key = body[3 + key_id_length..]
                .try_into()
                .map_err(|_| ControlError::InvalidPackageTrust)?;
            Ok(ControlRecord::PackageTrust(PackageTrust::new(
                key_id,
                key_type,
                public_key,
                maximum_tier,
            )?))
        }
        LAUNCH_BINDINGS_TAG => Ok(ControlRecord::LaunchBindings(decode_launch_bindings(
            &bytes[HEADER_BYTES..],
        )?)),
        RECEIVE_REQUEST_TAG
        | PUBLISH_REQUEST_TAG
        | ACKNOWLEDGE_REQUEST_TAG
        | COMPLETE_REQUEST_TAG
        | COMPLETE_WITH_TOOLS_REQUEST_TAG
        | EXECUTE_TOOL_REQUEST_TAG
        | LIST_MODEL_TOOLS_REQUEST_TAG => {
            match data_plane::decode(header[5], &bytes[HEADER_BYTES..])? {
                DataRecord::Request(request) => Ok(ControlRecord::Request(request)),
                DataRecord::Response(_) => Err(ControlError::InvalidDataPlaneRecord),
            }
        }
        RECEIVED_RESPONSE_TAG
        | PUBLISHED_RESPONSE_TAG
        | ACKNOWLEDGED_RESPONSE_TAG
        | COMPLETED_RESPONSE_TAG
        | TOOL_COMPLETED_RESPONSE_TAG
        | TOOL_EXECUTED_RESPONSE_TAG
        | MODEL_TOOLS_LISTED_RESPONSE_TAG
        | FAILED_RESPONSE_TAG => match data_plane::decode(header[5], &bytes[HEADER_BYTES..])? {
            DataRecord::Response(response) => Ok(ControlRecord::Response(response)),
            DataRecord::Request(_) => Err(ControlError::InvalidDataPlaneRecord),
        },
        _ => Err(ControlError::UnknownMessageKind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_secure_host_channel::{ChildBootstrap, HostId, OrchestratorBootstrap};
    use coding_adventures_x3dh::generate_identity_keypair;

    fn session(last: u8) -> SessionId {
        let mut bytes = [0u8; 16];
        bytes[..6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        SessionId::new(bytes).unwrap()
    }

    fn raw_pair(last: u8) -> (SecureHostChannel, SecureHostChannel) {
        let identity = generate_identity_keypair();
        let bootstrap =
            OrchestratorBootstrap::new(&identity, HostId::new("host-a").unwrap(), session(last))
                .unwrap();
        let offer = bootstrap.offer().unwrap();
        let (child, hello) = ChildBootstrap::open(&offer).unwrap();
        let orchestrator = bootstrap.accept(&hello).unwrap();
        (orchestrator, child)
    }

    fn control_pair(hash: [u8; 32]) -> (OrchestratorControl, ChildControl) {
        let (orchestrator, child) = raw_pair(1);
        (
            OrchestratorControl::new(orchestrator, hash).unwrap(),
            ChildControl::new(child).unwrap(),
        )
    }

    fn uuid_v7(last: u8) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[..6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        bytes
    }

    fn completion_call() -> CompletionCall {
        CompletionCall {
            model: "test-model".to_string(),
            system: Some("answer weather questions".to_string()),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                text: "weather in Seattle".to_string(),
            }],
            temperature: 0.0,
            max_tokens: Some(128),
            stop_sequences: vec!["END".to_string()],
            seed: Some(7),
            metadata: [("agent".to_string(), "weather".to_string())]
                .into_iter()
                .collect(),
        }
    }

    fn completion_result() -> CompletionResult {
        CompletionResult {
            text: "Rain".to_string(),
            model: "test-model-1".to_string(),
            provider: CompletionProvider {
                vendor: "test".to_string(),
                model_family: "test-model".to_string(),
                model_version: "1".to_string(),
                endpoint: Some("local".to_string()),
            },
            usage: CompletionUsage {
                input_tokens: 4,
                output_tokens: 1,
                cached_tokens: 0,
            },
            finish_reason: CompletionFinishReason::Stop,
            latency_ms: 3,
        }
    }

    fn tool_completion_call() -> ToolCompletionCall {
        ToolCompletionCall {
            completion: completion_call(),
            tools: vec![ModelToolDefinition {
                name: "smart_home.list_entities".to_string(),
                description: "List normalized entities".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
            }],
            choice: ModelToolChoice::Required,
            results: Vec::new(),
        }
    }

    fn package_trust() -> PackageTrust {
        PackageTrust::new("prod-test", PackageTrustType::Production, [17; 32], 3).unwrap()
    }

    fn launch_bindings() -> LaunchBindings {
        LaunchBindings::new(
            vec![
                ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, uuid_v7(1))
                    .unwrap(),
                ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, uuid_v7(2))
                    .unwrap(),
            ],
            Some(LevelOneModelBinding::new("test-model", 0.0, 128).unwrap()),
        )
        .unwrap()
    }

    fn trusted_pair(hash: [u8; 32]) -> (OrchestratorControl, ChildControl) {
        let (mut orchestrator, mut child) = control_pair(hash);
        let trust = package_trust();
        let frame = orchestrator.provide_package_trust(trust.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::PackageTrust(trust)
        );
        let bindings = launch_bindings();
        let frame = orchestrator
            .provide_launch_bindings(bindings.clone())
            .unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::LaunchBindings(bindings)
        );
        (orchestrator, child)
    }

    fn running_pair(hash: [u8; 32]) -> (OrchestratorControl, ChildControl) {
        let (mut orchestrator, mut child) = trusted_pair(hash);
        let ready = child.ready(hash).unwrap();
        orchestrator.receive_child(&ready, 1).unwrap();
        (orchestrator, child)
    }

    #[test]
    fn matching_ready_and_heartbeats_preserve_trusted_receipt_times() {
        let hash = [7u8; 32];
        let (mut orchestrator, mut child) = trusted_pair(hash);
        let ready = child.ready(hash).unwrap();
        assert_eq!(
            orchestrator.receive_child(&ready, 100).unwrap(),
            ChildEvent::Ready {
                package_hash: hash,
                received_at_ns: 100,
            }
        );
        for time in [101, 150] {
            let heartbeat = child.heartbeat().unwrap();
            assert_eq!(
                orchestrator.receive_child(&heartbeat, time).unwrap(),
                ChildEvent::Heartbeat {
                    received_at_ns: time
                }
            );
        }
        assert_eq!(orchestrator.state(), ControlState::Running);
        assert_eq!(child.state(), ControlState::Running);
    }

    #[test]
    fn authenticated_data_plane_round_trips_every_operation() {
        let (mut orchestrator, mut child) = running_pair([10; 32]);

        let (receive_id, frame) = child.request_receive(uuid_v7(1), 2).unwrap();
        assert_eq!(receive_id.get(), 1);
        assert_eq!(
            orchestrator.receive_child(&frame, 2).unwrap(),
            ChildEvent::Request(DataPlaneRequest::Receive {
                id: receive_id,
                channel_id: uuid_v7(1),
                limit: 2,
            })
        );
        let received = DataPlaneResponse::Received {
            id: receive_id,
            messages: vec![DataPlaneMessage {
                message_id: uuid_v7(2),
                sequence: 9,
                timestamp_ns: 11,
                content_type: "text/plain".to_string(),
                payload: b"Seattle".to_vec(),
            }],
        };
        let frame = orchestrator.respond(received.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(received)
        );

        let (publish_id, frame) = child
            .request_publish(
                uuid_v7(3),
                "text/plain; charset=utf-8".to_string(),
                b"Rain".to_vec(),
            )
            .unwrap();
        assert_eq!(publish_id.get(), 2);
        assert!(matches!(
            orchestrator.receive_child(&frame, 3).unwrap(),
            ChildEvent::Request(DataPlaneRequest::Publish { id, .. }) if id == publish_id
        ));
        let published = DataPlaneResponse::Published {
            id: publish_id,
            message_id: uuid_v7(4),
            sequence: 10,
            timestamp_ns: 12,
        };
        let frame = orchestrator.respond(published.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(published)
        );

        let (ack_id, frame) = child.request_acknowledge(uuid_v7(1), uuid_v7(2)).unwrap();
        assert_eq!(ack_id.get(), 3);
        assert!(matches!(
            orchestrator.receive_child(&frame, 4).unwrap(),
            ChildEvent::Request(DataPlaneRequest::Acknowledge { id, .. }) if id == ack_id
        ));
        let acknowledged = DataPlaneResponse::Acknowledged {
            id: ack_id,
            sequence: 9,
        };
        let frame = orchestrator.respond(acknowledged.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(acknowledged)
        );

        let call = completion_call();
        let (completion_id, frame) = child.request_completion(call.clone()).unwrap();
        assert_eq!(completion_id.get(), 4);
        assert_eq!(
            orchestrator.receive_child(&frame, 5).unwrap(),
            ChildEvent::Request(DataPlaneRequest::Complete {
                id: completion_id,
                call,
            })
        );
        let completed = DataPlaneResponse::Completed {
            id: completion_id,
            result: Box::new(completion_result()),
        };
        let frame = orchestrator.respond(completed.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(completed)
        );

        let tool_call = tool_completion_call();
        let (model_tools_id, frame) = child.request_model_tools().unwrap();
        assert_eq!(model_tools_id.get(), 5);
        assert_eq!(
            orchestrator.receive_child(&frame, 6).unwrap(),
            ChildEvent::Request(DataPlaneRequest::ListModelTools { id: model_tools_id })
        );
        let model_tools_listed = DataPlaneResponse::ModelToolsListed {
            id: model_tools_id,
            tools: tool_call.tools.clone(),
        };
        let frame = orchestrator.respond(model_tools_listed.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(model_tools_listed)
        );

        let (tool_completion_id, frame) = child.request_tool_completion(tool_call.clone()).unwrap();
        assert_eq!(tool_completion_id.get(), 6);
        assert_eq!(
            orchestrator.receive_child(&frame, 7).unwrap(),
            ChildEvent::Request(DataPlaneRequest::CompleteWithTools {
                id: tool_completion_id,
                call: Box::new(tool_call),
            })
        );
        let tool_completed = DataPlaneResponse::ToolCompleted {
            id: tool_completion_id,
            result: Box::new(ToolCompletionResult {
                output: ToolCompletionOutput::ToolCall(ModelToolCall {
                    call_id: "call-1".to_string(),
                    name: "smart_home.list_entities".to_string(),
                    arguments: serde_json::json!({}),
                }),
                model: "test-model-1".to_string(),
                provider: completion_result().provider,
                usage: completion_result().usage,
                finish_reason: CompletionFinishReason::Stop,
                latency_ms: 4,
                polyfill_used: false,
            }),
        };
        let frame = orchestrator.respond(tool_completed.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(tool_completed)
        );

        let model_call = ModelToolCall {
            call_id: "call-1".to_string(),
            name: "smart_home.list_entities".to_string(),
            arguments: serde_json::json!({}),
        };
        let (tool_execution_id, frame) = child.request_tool_execution(model_call.clone()).unwrap();
        assert_eq!(tool_execution_id.get(), 7);
        assert_eq!(
            orchestrator.receive_child(&frame, 8).unwrap(),
            ChildEvent::Request(DataPlaneRequest::ExecuteTool {
                id: tool_execution_id,
                call: Box::new(model_call.clone()),
            })
        );
        let tool_executed = DataPlaneResponse::ToolExecuted {
            id: tool_execution_id,
            result: Box::new(ModelToolResult {
                call: model_call,
                output: serde_json::json!({"entities": []}),
                is_error: false,
            }),
        };
        let frame = orchestrator.respond(tool_executed.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(tool_executed)
        );

        let (failure_id, frame) = child.request_receive(uuid_v7(5), 1).unwrap();
        orchestrator.receive_child(&frame, 9).unwrap();
        let failed = DataPlaneResponse::Failed {
            id: failure_id,
            failure: DataPlaneFailure::Unavailable,
        };
        let frame = orchestrator.respond(failed.clone()).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame).unwrap(),
            OrchestratorEvent::Response(failed)
        );
        assert_eq!(child.pending_request(), None);
        assert_eq!(orchestrator.pending_request(), None);
    }

    #[test]
    fn local_data_plane_misuse_is_rejected_without_consuming_correlation() {
        let (_, mut awaiting_child) = control_pair([11; 32]);
        assert_eq!(
            awaiting_child.request_receive(uuid_v7(1), 1),
            Err(ControlError::InvalidState)
        );
        assert_eq!(awaiting_child.state(), ControlState::AwaitingReady);

        let (mut orchestrator, mut child) = running_pair([12; 32]);
        assert_eq!(
            child.request_receive([0; 16], 1),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        assert_eq!(child.pending_request(), None);

        let (id, frame) = child.request_receive(uuid_v7(1), 1).unwrap();
        assert_eq!(id.get(), 1);
        assert_eq!(
            child.request_receive(uuid_v7(1), 1),
            Err(ControlError::RequestInFlight)
        );
        orchestrator.receive_child(&frame, 2).unwrap();
        let wrong_shape = DataPlaneResponse::Acknowledged { id, sequence: 0 };
        assert_eq!(
            orchestrator.respond(wrong_shape),
            Err(ControlError::CorrelationMismatch)
        );
        assert_eq!(
            orchestrator.pending_request(),
            Some((id, DataPlaneOperation::Receive))
        );
        let response = DataPlaneResponse::Received {
            id,
            messages: Vec::new(),
        };
        let frame = orchestrator.respond(response).unwrap();
        child.receive_orchestrator(&frame).unwrap();

        let (_, mut exhausted_child) = running_pair([15; 32]);
        exhausted_child.next_request_id = None;
        assert_eq!(
            exhausted_child.request_receive(uuid_v7(1), 1),
            Err(ControlError::RequestIdExhausted)
        );
    }

    #[test]
    fn peer_correlation_mismatch_fails_closed() {
        let (mut orchestrator, mut child) = running_pair([13; 32]);
        let (id, request) = child.request_receive(uuid_v7(1), 1).unwrap();
        orchestrator.receive_child(&request, 2).unwrap();
        let wrong_id = RequestId::new(id.get() + 1).unwrap();
        let plaintext = encode_record(ControlRecord::Response(DataPlaneResponse::Received {
            id: wrong_id,
            messages: Vec::new(),
        }))
        .unwrap();
        let frame = orchestrator.channel.send(&plaintext).unwrap();
        assert_eq!(
            child.receive_orchestrator(&frame),
            Err(ControlError::CorrelationMismatch)
        );
        assert_eq!(child.state(), ControlState::Closed);

        let (mut orchestrator, mut child) = running_pair([16; 32]);
        child.next_request_id = Some(2);
        let (_, skipped_request) = child.request_receive(uuid_v7(1), 1).unwrap();
        assert_eq!(
            orchestrator.receive_child(&skipped_request, 3),
            Err(ControlError::CorrelationMismatch)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);
    }

    #[test]
    fn data_plane_codec_rejects_truncation_trailing_bytes_and_limits() {
        let request = ControlRecord::Request(DataPlaneRequest::Publish {
            id: RequestId::new(1).unwrap(),
            channel_id: uuid_v7(1),
            content_type: "application/octet-stream".to_string(),
            payload: vec![1, 2, 3],
        });
        let encoded = encode_record(request).unwrap();
        assert!(matches!(
            decode_record(&encoded),
            Ok(ControlRecord::Request(DataPlaneRequest::Publish { .. }))
        ));
        for end in 0..encoded.len() {
            assert!(decode_record(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            decode_record(&trailing),
            Err(ControlError::InvalidDataPlaneRecord)
        );

        let (mut orchestrator, mut child) = running_pair([14; 32]);
        let mut oversized = completion_call();
        oversized.model = "m".repeat(201);
        assert_eq!(
            child.request_completion(oversized),
            Err(ControlError::InvalidDataPlaneRecord)
        );
        assert_eq!(child.pending_request(), None);

        let (_, request) = child.request_receive(uuid_v7(1), 1).unwrap();
        orchestrator.receive_child(&request, 2).unwrap();
        let invalid_message = DataPlaneResponse::Received {
            id: RequestId::new(1).unwrap(),
            messages: vec![DataPlaneMessage {
                message_id: [0; 16],
                sequence: 0,
                timestamp_ns: 0,
                content_type: "text/plain".to_string(),
                payload: Vec::new(),
            }],
        };
        assert_eq!(
            orchestrator.respond(invalid_message),
            Err(ControlError::InvalidDataPlaneRecord)
        );
    }

    #[test]
    fn package_mismatch_fails_closed() {
        let (mut orchestrator, mut child) = trusted_pair([1u8; 32]);
        let ready = child.ready([2u8; 32]).unwrap();
        assert_eq!(
            orchestrator.receive_child(&ready, 10),
            Err(ControlError::PackageMismatch)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);
        assert_eq!(
            orchestrator.receive_child(&ready, 11),
            Err(ControlError::Closed)
        );
        assert_eq!(orchestrator.terminate(), Err(ControlError::Closed));
    }

    #[test]
    fn constructors_enforce_secure_channel_roles() {
        let (orchestrator, child) = raw_pair(2);
        assert!(matches!(
            ChildControl::new(orchestrator),
            Err(ControlError::WrongChannelRole)
        ));
        assert!(matches!(
            OrchestratorControl::new(child, [0; 32]),
            Err(ControlError::WrongChannelRole)
        ));
    }

    #[test]
    fn local_child_ordering_is_non_destructive() {
        let (mut orchestrator, mut child) = control_pair([3u8; 32]);
        assert_eq!(child.heartbeat(), Err(ControlError::InvalidState));
        assert_eq!(child.ready([3u8; 32]), Err(ControlError::InvalidState));
        assert_eq!(child.state(), ControlState::AwaitingReady);
        let trust = orchestrator.provide_package_trust(package_trust()).unwrap();
        child.receive_orchestrator(&trust).unwrap();
        assert_eq!(child.ready([3u8; 32]), Err(ControlError::InvalidState));
        let bindings = orchestrator
            .provide_launch_bindings(launch_bindings())
            .unwrap();
        child.receive_orchestrator(&bindings).unwrap();
        child.ready([3u8; 32]).unwrap();
        assert_eq!(child.ready([3u8; 32]), Err(ControlError::InvalidState));
        assert_eq!(child.state(), ControlState::Running);
    }

    #[test]
    fn peer_lifecycle_violations_fail_closed() {
        let (orchestrator_channel, mut child_channel) = raw_pair(3);
        let mut orchestrator = OrchestratorControl::new(orchestrator_channel, [4; 32]).unwrap();
        let early_heartbeat = child_channel
            .send(&encode_record(ControlRecord::Heartbeat).unwrap())
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&early_heartbeat, 1),
            Err(ControlError::InvalidState)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator, mut child) = trusted_pair([4; 32]);
        let first = child.ready([4; 32]).unwrap();
        orchestrator.receive_child(&first, 1).unwrap();
        let duplicate = child
            .channel
            .send(&encode_record(ControlRecord::Ready([4; 32])).unwrap())
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&duplicate, 2),
            Err(ControlError::InvalidState)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);
    }

    #[test]
    fn termination_works_before_and_after_readiness() {
        let (mut orchestrator, mut child) = control_pair([5; 32]);
        let terminate = orchestrator.terminate().unwrap();
        assert_eq!(
            child.receive_orchestrator(&terminate).unwrap(),
            OrchestratorEvent::Terminate
        );
        assert_eq!(orchestrator.state(), ControlState::Terminating);
        assert_eq!(child.state(), ControlState::Terminating);
        assert_eq!(orchestrator.terminate(), Err(ControlError::InvalidState));
        assert_eq!(child.ready([5; 32]), Err(ControlError::InvalidState));
        assert_eq!(child.heartbeat(), Err(ControlError::InvalidState));
        assert_eq!(
            child.receive_orchestrator(&terminate),
            Err(ControlError::InvalidState)
        );
        assert_eq!(child.state(), ControlState::Closed);
        let post_terminate = child
            .channel
            .send(&encode_record(ControlRecord::Heartbeat).unwrap())
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&post_terminate, 9),
            Err(ControlError::InvalidState)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator, mut child) = trusted_pair([6; 32]);
        let ready = child.ready([6; 32]).unwrap();
        orchestrator.receive_child(&ready, 8).unwrap();
        let terminate = orchestrator.terminate().unwrap();
        assert_eq!(
            child.receive_orchestrator(&terminate),
            Ok(OrchestratorEvent::Terminate)
        );
    }

    #[test]
    fn wrong_direction_kinds_fail_closed() {
        let (orchestrator_channel, mut child_channel) = raw_pair(5);
        let mut orchestrator = OrchestratorControl::new(orchestrator_channel, [7; 32]).unwrap();
        let terminate = child_channel
            .send(&encode_record(ControlRecord::Terminate).unwrap())
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&terminate, 1),
            Err(ControlError::WrongMessageDirection)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator_channel, child_channel) = raw_pair(6);
        let mut child = ChildControl::new(child_channel).unwrap();
        let ready = orchestrator_channel
            .send(&encode_record(ControlRecord::Ready([7; 32])).unwrap())
            .unwrap();
        assert_eq!(
            child.receive_orchestrator(&ready),
            Err(ControlError::WrongMessageDirection)
        );
        assert_eq!(child.state(), ControlState::Closed);
        assert_eq!(child.heartbeat(), Err(ControlError::Closed));
        assert_eq!(
            child.receive_orchestrator(&ready),
            Err(ControlError::Closed)
        );
    }

    #[test]
    fn secure_tampering_and_replay_fail_closed() {
        let hash = [8; 32];
        let (mut orchestrator, mut child) = trusted_pair(hash);
        let mut ready = child.ready(hash).unwrap();
        *ready.last_mut().unwrap() ^= 1;
        assert!(matches!(
            orchestrator.receive_child(&ready, 1),
            Err(ControlError::Channel(ChannelError::Crypto))
        ));
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator, mut child) = trusted_pair(hash);
        let ready = child.ready(hash).unwrap();
        orchestrator.receive_child(&ready, 1).unwrap();
        assert!(matches!(
            orchestrator.receive_child(&ready, 2),
            Err(ControlError::Channel(ChannelError::UnexpectedSequence))
        ));
        assert_eq!(orchestrator.state(), ControlState::Closed);
    }

    #[test]
    fn codec_is_strict_bounded_and_complete() {
        let records = [
            (ControlRecord::Ready([9; 32]), ControlError::MalformedRecord),
            (ControlRecord::Heartbeat, ControlError::MalformedRecord),
            (ControlRecord::Terminate, ControlError::MalformedRecord),
            (
                ControlRecord::PackageTrust(package_trust()),
                ControlError::InvalidPackageTrust,
            ),
        ];
        for (record, truncated_error) in records {
            let encoded = encode_record(record.clone()).unwrap();
            assert_eq!(decode_record(&encoded), Ok(record));
            for end in 0..encoded.len() {
                let error = decode_record(&encoded[..end]).unwrap_err();
                assert!(matches!(error, ControlError::MalformedRecord) || error == truncated_error);
            }
            let mut trailing = encoded;
            trailing.push(0);
            let error = decode_record(&trailing).unwrap_err();
            assert!(matches!(error, ControlError::MalformedRecord) || error == truncated_error);
        }

        let mut bad_magic = encode_record(ControlRecord::Heartbeat).unwrap();
        bad_magic[0] = b'X';
        assert_eq!(
            decode_record(&bad_magic),
            Err(ControlError::MalformedRecord)
        );
        let mut bad_version = encode_record(ControlRecord::Heartbeat).unwrap();
        bad_version[4] = 2;
        assert_eq!(
            decode_record(&bad_version),
            Err(ControlError::UnsupportedVersion)
        );
        let mut bad_tag = encode_record(ControlRecord::Heartbeat).unwrap();
        bad_tag[5] = 99;
        assert_eq!(
            decode_record(&bad_tag),
            Err(ControlError::UnknownMessageKind)
        );
    }

    #[test]
    fn package_trust_is_bounded_typed_and_required_before_ready() {
        for invalid in ["", "bad key", "x/", &"x".repeat(129)] {
            assert_eq!(
                PackageTrust::new(invalid, PackageTrustType::Production, [1; 32], 3),
                Err(ControlError::InvalidPackageTrust)
            );
        }
        assert_eq!(
            PackageTrust::new("dev", PackageTrustType::Developer, [1; 32], 2),
            Err(ControlError::InvalidPackageTrust)
        );
        assert_eq!(
            PackageTrust::new("prod", PackageTrustType::Production, [1; 32], 4),
            Err(ControlError::InvalidPackageTrust)
        );
        for trust in [
            PackageTrust::new("dev", PackageTrustType::Developer, [2; 32], 1).unwrap(),
            PackageTrust::new("third", PackageTrustType::ThirdParty, [3; 32], 2).unwrap(),
        ] {
            let record = ControlRecord::PackageTrust(trust);
            assert_eq!(
                decode_record(&encode_record(record.clone()).unwrap()),
                Ok(record)
            );
        }

        let mut invalid_type = encode_record(ControlRecord::PackageTrust(package_trust())).unwrap();
        let key_id_length = invalid_type[HEADER_BYTES] as usize;
        invalid_type[HEADER_BYTES + 1 + key_id_length] = 99;
        assert_eq!(
            decode_record(&invalid_type),
            Err(ControlError::InvalidPackageTrust)
        );
        let mut invalid_tier = encode_record(ControlRecord::PackageTrust(package_trust())).unwrap();
        invalid_tier[HEADER_BYTES + 2 + key_id_length] = 4;
        assert_eq!(
            decode_record(&invalid_tier),
            Err(ControlError::InvalidPackageTrust)
        );

        let hash = [31; 32];
        let (mut orchestrator, mut child) = control_pair(hash);
        assert_eq!(child.ready(hash), Err(ControlError::InvalidState));
        let trust = package_trust();
        assert_eq!(trust.key_id(), "prod-test");
        assert_eq!(trust.key_type(), PackageTrustType::Production);
        assert_eq!(trust.public_key(), [17; 32]);
        assert_eq!(trust.maximum_tier(), 3);
        let frame = orchestrator.provide_package_trust(trust.clone()).unwrap();
        assert_eq!(
            orchestrator.provide_package_trust(trust.clone()),
            Err(ControlError::InvalidState)
        );
        assert_eq!(
            child.receive_orchestrator(&frame),
            Ok(OrchestratorEvent::PackageTrust(trust))
        );
        assert_eq!(child.ready(hash), Err(ControlError::InvalidState));
        let bindings = orchestrator
            .provide_launch_bindings(launch_bindings())
            .unwrap();
        assert!(matches!(
            child.receive_orchestrator(&bindings),
            Ok(OrchestratorEvent::LaunchBindings(_))
        ));
        assert!(child.ready(hash).is_ok());

        let (orchestrator_channel, mut child_channel) = raw_pair(43);
        let mut orchestrator = OrchestratorControl::new(orchestrator_channel, hash).unwrap();
        let premature_ready = child_channel
            .send(&encode_record(ControlRecord::Ready(hash)).unwrap())
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&premature_ready, 1),
            Err(ControlError::InvalidState)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator, mut child) = control_pair(hash);
        let first = orchestrator.provide_package_trust(package_trust()).unwrap();
        child.receive_orchestrator(&first).unwrap();
        let duplicate = orchestrator
            .channel
            .send(&encode_record(ControlRecord::PackageTrust(package_trust())).unwrap())
            .unwrap();
        assert_eq!(
            child.receive_orchestrator(&duplicate),
            Err(ControlError::InvalidState)
        );
        assert_eq!(child.state(), ControlState::Closed);
    }

    #[test]
    fn launch_bindings_are_exactly_once_after_trust_and_before_ready() {
        let hash = [32; 32];
        let (mut orchestrator, mut child) = control_pair(hash);
        assert_eq!(
            orchestrator.provide_launch_bindings(launch_bindings()),
            Err(ControlError::InvalidState)
        );
        let trust = orchestrator.provide_package_trust(package_trust()).unwrap();
        child.receive_orchestrator(&trust).unwrap();
        let bindings = launch_bindings();
        let frame = orchestrator
            .provide_launch_bindings(bindings.clone())
            .unwrap();
        assert_eq!(
            orchestrator.provide_launch_bindings(bindings.clone()),
            Err(ControlError::InvalidState)
        );
        assert_eq!(
            child.receive_orchestrator(&frame),
            Ok(OrchestratorEvent::LaunchBindings(bindings.clone()))
        );

        let duplicate = orchestrator
            .channel
            .send(&encode_record(ControlRecord::LaunchBindings(bindings)).unwrap())
            .unwrap();
        assert_eq!(
            child.receive_orchestrator(&duplicate),
            Err(ControlError::InvalidState)
        );
        assert_eq!(child.state(), ControlState::Closed);

        let (orchestrator_channel, mut child_channel) = raw_pair(44);
        let mut orchestrator = OrchestratorControl::new(orchestrator_channel, hash).unwrap();
        let trust = orchestrator.provide_package_trust(package_trust()).unwrap();
        child_channel.receive(&trust).unwrap();
        let premature_ready = child_channel
            .send(&encode_record(ControlRecord::Ready(hash)).unwrap())
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&premature_ready, 1),
            Err(ControlError::InvalidState)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);
    }

    #[test]
    fn malformed_authenticated_plaintext_closes_both_roles() {
        let (orchestrator_channel, mut child_channel) = raw_pair(7);
        let mut orchestrator = OrchestratorControl::new(orchestrator_channel, [1; 32]).unwrap();
        let malformed = child_channel.send(b"bad").unwrap();
        assert_eq!(
            orchestrator.receive_child(&malformed, 1),
            Err(ControlError::MalformedRecord)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator_channel, child_channel) = raw_pair(8);
        let mut child = ChildControl::new(child_channel).unwrap();
        let malformed = orchestrator_channel.send(b"bad").unwrap();
        assert_eq!(
            child.receive_orchestrator(&malformed),
            Err(ControlError::MalformedRecord)
        );
        assert_eq!(child.state(), ControlState::Closed);
    }

    #[test]
    fn session_identity_and_diagnostics_are_stable() {
        let (orchestrator_channel, child_channel) = raw_pair(42);
        let orchestrator = OrchestratorControl::new(orchestrator_channel, [0; 32]).unwrap();
        let child = ChildControl::new(child_channel).unwrap();
        assert_eq!(orchestrator.session_id(), session(42));
        assert_eq!(child.session_id(), session(42));

        let errors = [
            ControlError::WrongChannelRole,
            ControlError::Channel(ChannelError::Crypto),
            ControlError::MalformedRecord,
            ControlError::UnsupportedVersion,
            ControlError::UnknownMessageKind,
            ControlError::InvalidDataPlaneRecord,
            ControlError::InvalidPackageTrust,
            ControlError::WrongMessageDirection,
            ControlError::InvalidState,
            ControlError::PackageMismatch,
            ControlError::CorrelationMismatch,
            ControlError::RequestInFlight,
            ControlError::RequestIdExhausted,
            ControlError::Closed,
        ];
        for error in errors {
            assert!(error.to_string().starts_with("host-control:"));
        }
    }
}
