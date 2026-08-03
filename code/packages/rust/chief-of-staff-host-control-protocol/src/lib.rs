//! Authenticated D18 host readiness, heartbeat, and termination protocol.
//!
//! The secure host channel provides opaque authenticated bytes. This crate adds
//! the minimum lifecycle state machine required by process supervision without
//! opening streams, polling clocks, or spawning processes.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_secure_host_channel::{ChannelError, ChannelRole, SecureHostChannel, SessionId};
use core::fmt::{self, Display, Formatter};

const MAGIC: &[u8; 4] = b"D18C";
const VERSION: u8 = 1;
const READY_TAG: u8 = 1;
const HEARTBEAT_TAG: u8 = 2;
const TERMINATE_TAG: u8 = 3;
const HEADER_BYTES: usize = 6;
const READY_BYTES: usize = HEADER_BYTES + 32;

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
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

/// Authenticated orchestrator event accepted by a child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrchestratorEvent {
    /// Begin graceful host shutdown.
    Terminate,
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
    /// The peer sent a valid message kind owned by the opposite direction.
    WrongMessageDirection,
    /// The operation violates readiness or termination ordering.
    InvalidState,
    /// The child's independently verified package differs from registration.
    PackageMismatch,
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
            Self::WrongMessageDirection => "host-control: wrong message direction",
            Self::InvalidState => "host-control: invalid lifecycle state",
            Self::PackageMismatch => "host-control: package identity mismatch",
            Self::Closed => "host-control: endpoint is closed",
        })
    }
}

impl std::error::Error for ControlError {}

/// Orchestrator-side lifecycle wrapper around one secure host channel.
pub struct OrchestratorControl {
    channel: SecureHostChannel,
    expected_package_hash: [u8; 32],
    state: ControlState,
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
            (ControlState::AwaitingReady, ControlRecord::Ready(package_hash)) => {
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
            (_, ControlRecord::Terminate) => Err(self.close(ControlError::WrongMessageDirection)),
            _ => Err(self.close(ControlError::InvalidState)),
        }
    }

    /// Encrypt one graceful-termination request and stop accepting messages.
    pub fn terminate(&mut self) -> Result<Vec<u8>, ControlError> {
        match self.state {
            ControlState::AwaitingReady | ControlState::Running => {}
            ControlState::Closed => return Err(ControlError::Closed),
            ControlState::Terminating => return Err(ControlError::InvalidState),
        }
        let frame = match self.channel.send(&encode_record(ControlRecord::Terminate)) {
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
        })
    }

    /// Encrypt the first readiness record after independent package verification.
    pub fn ready(&mut self, package_hash: [u8; 32]) -> Result<Vec<u8>, ControlError> {
        if self.state == ControlState::Closed {
            return Err(ControlError::Closed);
        }
        if self.state != ControlState::AwaitingReady {
            return Err(ControlError::InvalidState);
        }
        let frame = match self
            .channel
            .send(&encode_record(ControlRecord::Ready(package_hash)))
        {
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
        match self.channel.send(&encode_record(ControlRecord::Heartbeat)) {
            Ok(frame) => Ok(frame),
            Err(error) => Err(self.close(ControlError::Channel(error))),
        }
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
            ControlRecord::Terminate => {
                self.state = ControlState::Terminating;
                Ok(OrchestratorEvent::Terminate)
            }
            ControlRecord::Ready(_) | ControlRecord::Heartbeat => {
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

    fn close(&mut self, error: ControlError) -> ControlError {
        self.state = ControlState::Closed;
        error
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlRecord {
    Ready([u8; 32]),
    Heartbeat,
    Terminate,
}

fn encode_record(record: ControlRecord) -> Vec<u8> {
    let mut output = Vec::with_capacity(match record {
        ControlRecord::Ready(_) => READY_BYTES,
        ControlRecord::Heartbeat | ControlRecord::Terminate => HEADER_BYTES,
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
    }
    output
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

    #[test]
    fn matching_ready_and_heartbeats_preserve_trusted_receipt_times() {
        let hash = [7u8; 32];
        let (mut orchestrator, mut child) = control_pair(hash);
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
    fn package_mismatch_fails_closed() {
        let (mut orchestrator, mut child) = control_pair([1u8; 32]);
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
        let (_, mut child) = control_pair([3u8; 32]);
        assert_eq!(child.heartbeat(), Err(ControlError::InvalidState));
        assert_eq!(child.state(), ControlState::AwaitingReady);
        child.ready([3u8; 32]).unwrap();
        assert_eq!(child.ready([3u8; 32]), Err(ControlError::InvalidState));
        assert_eq!(child.state(), ControlState::Running);
    }

    #[test]
    fn peer_lifecycle_violations_fail_closed() {
        let (orchestrator_channel, mut child_channel) = raw_pair(3);
        let mut orchestrator = OrchestratorControl::new(orchestrator_channel, [4; 32]).unwrap();
        let early_heartbeat = child_channel
            .send(&encode_record(ControlRecord::Heartbeat))
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&early_heartbeat, 1),
            Err(ControlError::InvalidState)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (orchestrator_channel, child_channel) = raw_pair(4);
        let mut orchestrator = OrchestratorControl::new(orchestrator_channel, [4; 32]).unwrap();
        let mut child = ChildControl::new(child_channel).unwrap();
        let first = child.ready([4; 32]).unwrap();
        orchestrator.receive_child(&first, 1).unwrap();
        let duplicate = child
            .channel
            .send(&encode_record(ControlRecord::Ready([4; 32])))
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
            .send(&encode_record(ControlRecord::Heartbeat))
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&post_terminate, 9),
            Err(ControlError::InvalidState)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator, mut child) = control_pair([6; 32]);
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
            .send(&encode_record(ControlRecord::Terminate))
            .unwrap();
        assert_eq!(
            orchestrator.receive_child(&terminate, 1),
            Err(ControlError::WrongMessageDirection)
        );
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator_channel, child_channel) = raw_pair(6);
        let mut child = ChildControl::new(child_channel).unwrap();
        let ready = orchestrator_channel
            .send(&encode_record(ControlRecord::Ready([7; 32])))
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
        let (mut orchestrator, mut child) = control_pair(hash);
        let mut ready = child.ready(hash).unwrap();
        *ready.last_mut().unwrap() ^= 1;
        assert!(matches!(
            orchestrator.receive_child(&ready, 1),
            Err(ControlError::Channel(ChannelError::Crypto))
        ));
        assert_eq!(orchestrator.state(), ControlState::Closed);

        let (mut orchestrator, mut child) = control_pair(hash);
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
            ControlRecord::Ready([9; 32]),
            ControlRecord::Heartbeat,
            ControlRecord::Terminate,
        ];
        for record in records {
            let encoded = encode_record(record);
            assert_eq!(decode_record(&encoded), Ok(record));
            for end in 0..encoded.len() {
                assert_eq!(
                    decode_record(&encoded[..end]),
                    Err(ControlError::MalformedRecord)
                );
            }
            let mut trailing = encoded;
            trailing.push(0);
            assert_eq!(decode_record(&trailing), Err(ControlError::MalformedRecord));
        }

        let mut bad_magic = encode_record(ControlRecord::Heartbeat);
        bad_magic[0] = b'X';
        assert_eq!(
            decode_record(&bad_magic),
            Err(ControlError::MalformedRecord)
        );
        let mut bad_version = encode_record(ControlRecord::Heartbeat);
        bad_version[4] = 2;
        assert_eq!(
            decode_record(&bad_version),
            Err(ControlError::UnsupportedVersion)
        );
        let mut bad_tag = encode_record(ControlRecord::Heartbeat);
        bad_tag[5] = 99;
        assert_eq!(
            decode_record(&bad_tag),
            Err(ControlError::UnknownMessageKind)
        );
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
            ControlError::WrongMessageDirection,
            ControlError::InvalidState,
            ControlError::PackageMismatch,
            ControlError::Closed,
        ];
        for error in errors {
            assert!(error.to_string().starts_with("host-control:"));
        }
    }
}
