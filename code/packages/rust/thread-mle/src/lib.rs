//! Thread Mesh Link Establishment primitives.
//!
//! MLE is the Thread control plane for roles, neighbors, parent/child attach,
//! and network data exchange. This crate starts with pure message/TLV parsing
//! and a deterministic attach-state skeleton. It intentionally performs no UDP,
//! CoAP, DTLS, radio, commissioning, or border-router I/O.

#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRole {
    Disabled,
    Detached,
    Child,
    Router,
    Leader,
}

impl DeviceRole {
    pub fn can_route(self) -> bool {
        matches!(self, Self::Router | Self::Leader)
    }

    pub fn is_attached(self) -> bool {
        matches!(self, Self::Child | Self::Router | Self::Leader)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MleCommand {
    LinkRequest,
    LinkAccept,
    LinkAcceptAndRequest,
    LinkReject,
    Advertisement,
    Update,
    UpdateRequest,
    DataRequest,
    DataResponse,
    ParentRequest,
    ParentResponse,
    ChildIdRequest,
    ChildIdResponse,
    ChildUpdateRequest,
    ChildUpdateResponse,
    Announce,
    DiscoveryRequest,
    DiscoveryResponse,
    Unknown(u8),
}

impl MleCommand {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => Self::LinkRequest,
            1 => Self::LinkAccept,
            2 => Self::LinkAcceptAndRequest,
            3 => Self::LinkReject,
            4 => Self::Advertisement,
            5 => Self::Update,
            6 => Self::UpdateRequest,
            7 => Self::DataRequest,
            8 => Self::DataResponse,
            9 => Self::ParentRequest,
            10 => Self::ParentResponse,
            11 => Self::ChildIdRequest,
            12 => Self::ChildIdResponse,
            13 => Self::ChildUpdateRequest,
            14 => Self::ChildUpdateResponse,
            15 => Self::Announce,
            16 => Self::DiscoveryRequest,
            17 => Self::DiscoveryResponse,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::LinkRequest => 0,
            Self::LinkAccept => 1,
            Self::LinkAcceptAndRequest => 2,
            Self::LinkReject => 3,
            Self::Advertisement => 4,
            Self::Update => 5,
            Self::UpdateRequest => 6,
            Self::DataRequest => 7,
            Self::DataResponse => 8,
            Self::ParentRequest => 9,
            Self::ParentResponse => 10,
            Self::ChildIdRequest => 11,
            Self::ChildIdResponse => 12,
            Self::ChildUpdateRequest => 13,
            Self::ChildUpdateResponse => 14,
            Self::Announce => 15,
            Self::DiscoveryRequest => 16,
            Self::DiscoveryResponse => 17,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlvType {
    SourceAddress,
    Mode,
    Timeout,
    Challenge,
    Response,
    LinkLayerFrameCounter,
    MleFrameCounter,
    Route64,
    Address16,
    LeaderData,
    NetworkData,
    TlvRequest,
    ScanMask,
    Connectivity,
    LinkMargin,
    Status,
    Version,
    AddressRegistration,
    Channel,
    PanId,
    ActiveTimestamp,
    PendingTimestamp,
    Unknown(u8),
}

impl TlvType {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => Self::SourceAddress,
            1 => Self::Mode,
            2 => Self::Timeout,
            3 => Self::Challenge,
            4 => Self::Response,
            5 => Self::LinkLayerFrameCounter,
            8 => Self::MleFrameCounter,
            9 => Self::Route64,
            10 => Self::Address16,
            11 => Self::LeaderData,
            12 => Self::NetworkData,
            13 => Self::TlvRequest,
            14 => Self::ScanMask,
            15 => Self::Connectivity,
            16 => Self::LinkMargin,
            17 => Self::Status,
            18 => Self::Version,
            19 => Self::AddressRegistration,
            20 => Self::Channel,
            21 => Self::PanId,
            22 => Self::ActiveTimestamp,
            23 => Self::PendingTimestamp,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::SourceAddress => 0,
            Self::Mode => 1,
            Self::Timeout => 2,
            Self::Challenge => 3,
            Self::Response => 4,
            Self::LinkLayerFrameCounter => 5,
            Self::MleFrameCounter => 8,
            Self::Route64 => 9,
            Self::Address16 => 10,
            Self::LeaderData => 11,
            Self::NetworkData => 12,
            Self::TlvRequest => 13,
            Self::ScanMask => 14,
            Self::Connectivity => 15,
            Self::LinkMargin => 16,
            Self::Status => 17,
            Self::Version => 18,
            Self::AddressRegistration => 19,
            Self::Channel => 20,
            Self::PanId => 21,
            Self::ActiveTimestamp => 22,
            Self::PendingTimestamp => 23,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tlv {
    pub tlv_type: TlvType,
    pub value: Vec<u8>,
}

impl Tlv {
    pub fn new(tlv_type: TlvType, value: Vec<u8>) -> Result<Self, MleError> {
        if value.len() > u8::MAX as usize {
            return Err(MleError::TlvTooLong(value.len()));
        }
        Ok(Self { tlv_type, value })
    }

    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), MleError> {
        if self.value.len() > u8::MAX as usize {
            return Err(MleError::TlvTooLong(self.value.len()));
        }
        out.push(self.tlv_type.as_byte());
        out.push(self.value.len() as u8);
        out.extend_from_slice(&self.value);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MleMessage {
    pub command: MleCommand,
    pub tlvs: Vec<Tlv>,
}

impl MleMessage {
    pub fn parse(bytes: &[u8]) -> Result<Self, MleError> {
        let Some((&command, rest)) = bytes.split_first() else {
            return Err(MleError::Truncated {
                needed: 1,
                remaining: 0,
            });
        };
        let mut cursor = Cursor::new(rest);
        let mut tlvs = Vec::new();
        while cursor.remaining() > 0 {
            let tlv_type = TlvType::from_byte(cursor.read_u8()?);
            let len = cursor.read_u8()? as usize;
            let value = cursor.read_bytes(len)?.to_vec();
            tlvs.push(Tlv { tlv_type, value });
        }
        Ok(Self {
            command: MleCommand::from_byte(command),
            tlvs,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, MleError> {
        let mut out = Vec::new();
        out.push(self.command.as_byte());
        for tlv in &self.tlvs {
            tlv.encode(&mut out)?;
        }
        Ok(out)
    }

    pub fn find_tlv(&self, tlv_type: TlvType) -> Option<&Tlv> {
        self.tlvs.iter().find(|tlv| tlv.tlv_type == tlv_type)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanMask {
    pub routers: bool,
    pub end_devices: bool,
}

impl ScanMask {
    pub fn parse(value: u8) -> Self {
        Self {
            routers: value & 0x80 != 0,
            end_devices: value & 0x40 != 0,
        }
    }

    pub fn encode(self) -> u8 {
        ((self.routers as u8) << 7) | ((self.end_devices as u8) << 6)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub receiver_on_when_idle: bool,
    pub secure_data_requests: bool,
    pub full_thread_device: bool,
    pub full_network_data: bool,
}

impl Mode {
    pub fn parse(value: u8) -> Self {
        Self {
            receiver_on_when_idle: value & (1 << 3) != 0,
            secure_data_requests: value & (1 << 2) != 0,
            full_thread_device: value & (1 << 1) != 0,
            full_network_data: value & 1 != 0,
        }
    }

    pub fn encode(self) -> u8 {
        ((self.receiver_on_when_idle as u8) << 3)
            | ((self.secure_data_requests as u8) << 2)
            | ((self.full_thread_device as u8) << 1)
            | (self.full_network_data as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachState {
    Detached,
    ParentSearch,
    ParentCandidate,
    ChildIdRequestSent,
    Attached(DeviceRole),
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachAction {
    SendParentRequest,
    SendChildIdRequest,
    BecomeChild,
    BecomeRouter,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachMachine {
    state: AttachState,
}

impl AttachMachine {
    pub fn new() -> Self {
        Self {
            state: AttachState::Detached,
        }
    }

    pub fn state(&self) -> AttachState {
        self.state
    }

    pub fn start(&mut self) -> AttachAction {
        self.state = AttachState::ParentSearch;
        AttachAction::SendParentRequest
    }

    pub fn on_message(&mut self, message: &MleMessage) -> AttachAction {
        match (self.state, message.command) {
            (AttachState::ParentSearch, MleCommand::ParentResponse) => {
                self.state = AttachState::ParentCandidate;
                AttachAction::SendChildIdRequest
            }
            (AttachState::ParentCandidate, MleCommand::ChildIdResponse) => {
                let role = role_from_child_id_response(message);
                self.state = AttachState::Attached(role);
                match role {
                    DeviceRole::Router | DeviceRole::Leader => AttachAction::BecomeRouter,
                    _ => AttachAction::BecomeChild,
                }
            }
            (_, MleCommand::LinkReject) => {
                self.state = AttachState::Rejected;
                AttachAction::None
            }
            _ => AttachAction::None,
        }
    }
}

impl Default for AttachMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MleError {
    Truncated { needed: usize, remaining: usize },
    TlvTooLong(usize),
}

impl fmt::Display for MleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => {
                write!(
                    f,
                    "truncated Thread MLE bytes: needed {needed}, had {remaining}"
                )
            }
            Self::TlvTooLong(len) => write!(f, "Thread MLE TLV too long: {len}"),
        }
    }
}

impl std::error::Error for MleError {}

fn role_from_child_id_response(message: &MleMessage) -> DeviceRole {
    let mode = message
        .find_tlv(TlvType::Mode)
        .and_then(|tlv| tlv.value.first().copied())
        .map(Mode::parse);
    match mode {
        Some(mode) if mode.full_thread_device => DeviceRole::Router,
        _ => DeviceRole::Child,
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> Result<u8, MleError> {
        if self.remaining() < 1 {
            return Err(MleError::Truncated {
                needed: 1,
                remaining: self.remaining(),
            });
        }
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], MleError> {
        if self.remaining() < len {
            return Err(MleError::Truncated {
                needed: len,
                remaining: self.remaining(),
            });
        }
        let bytes = &self.bytes[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mle_message_round_trips_tlvs() {
        let message = MleMessage {
            command: MleCommand::ParentRequest,
            tlvs: vec![
                Tlv::new(
                    TlvType::ScanMask,
                    vec![ScanMask {
                        routers: true,
                        end_devices: false,
                    }
                    .encode()],
                )
                .unwrap(),
                Tlv::new(TlvType::Version, vec![0x00, 0x04]).unwrap(),
            ],
        };

        assert_eq!(
            MleMessage::parse(&message.encode().unwrap()).unwrap(),
            message
        );
    }

    #[test]
    fn scan_mask_and_mode_bits_round_trip() {
        let scan = ScanMask {
            routers: true,
            end_devices: true,
        };
        let mode = Mode {
            receiver_on_when_idle: true,
            secure_data_requests: true,
            full_thread_device: false,
            full_network_data: true,
        };

        assert_eq!(ScanMask::parse(scan.encode()), scan);
        assert_eq!(Mode::parse(mode.encode()), mode);
    }

    #[test]
    fn attach_machine_follows_parent_then_child_id_flow() {
        let mut machine = AttachMachine::new();
        let parent_response = MleMessage {
            command: MleCommand::ParentResponse,
            tlvs: Vec::new(),
        };
        let child_id_response = MleMessage {
            command: MleCommand::ChildIdResponse,
            tlvs: vec![Tlv::new(
                TlvType::Mode,
                vec![Mode {
                    receiver_on_when_idle: false,
                    secure_data_requests: true,
                    full_thread_device: false,
                    full_network_data: true,
                }
                .encode()],
            )
            .unwrap()],
        };

        assert_eq!(machine.start(), AttachAction::SendParentRequest);
        assert_eq!(
            machine.on_message(&parent_response),
            AttachAction::SendChildIdRequest
        );
        assert_eq!(
            machine.on_message(&child_id_response),
            AttachAction::BecomeChild
        );
        assert_eq!(machine.state(), AttachState::Attached(DeviceRole::Child));
    }

    #[test]
    fn attach_machine_can_attach_as_router_candidate() {
        let mut machine = AttachMachine::new();
        machine.start();
        machine.on_message(&MleMessage {
            command: MleCommand::ParentResponse,
            tlvs: Vec::new(),
        });

        let action = machine.on_message(&MleMessage {
            command: MleCommand::ChildIdResponse,
            tlvs: vec![Tlv::new(
                TlvType::Mode,
                vec![Mode {
                    receiver_on_when_idle: true,
                    secure_data_requests: true,
                    full_thread_device: true,
                    full_network_data: true,
                }
                .encode()],
            )
            .unwrap()],
        });

        assert_eq!(action, AttachAction::BecomeRouter);
        assert_eq!(machine.state(), AttachState::Attached(DeviceRole::Router));
    }

    #[test]
    fn malformed_tlv_reports_truncation() {
        assert_eq!(
            MleMessage::parse(&[
                MleCommand::ParentRequest.as_byte(),
                TlvType::Version.as_byte(),
                2,
                1
            ]),
            Err(MleError::Truncated {
                needed: 2,
                remaining: 1
            })
        );
    }
}
