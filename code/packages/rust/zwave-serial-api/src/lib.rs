//! Z-Wave Serial API host/controller primitives.
//!
//! `zwave-core` owns raw Serial API frames. This crate starts the host-side
//! control-plane layer: function ids, request/response/callback classification,
//! controller capability snapshots, and callback correlation.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use zwave_core::{CommandClassId, HomeId, NodeId, SerialFrame, SerialFrameType, ZWaveError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(pub u8);

impl FunctionId {
    pub const SERIAL_API_GET_INIT_DATA: Self = Self(0x02);
    pub const APPLICATION_COMMAND_HANDLER: Self = Self(0x04);
    pub const GET_CONTROLLER_CAPABILITIES: Self = Self(0x05);
    pub const SERIAL_API_SET_TIMEOUTS: Self = Self(0x06);
    pub const SEND_DATA: Self = Self(0x13);
    pub const GET_VERSION: Self = Self(0x15);
    pub const MEMORY_GET_ID: Self = Self(0x20);
    pub const REQUEST_NODE_INFO: Self = Self(0x60);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialMessageKind {
    Request,
    Response,
    Callback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialMessage {
    pub kind: SerialMessageKind,
    pub function_id: FunctionId,
    pub callback_id: Option<u8>,
    pub payload: Vec<u8>,
}

impl SerialMessage {
    pub fn from_frame(frame: SerialFrame) -> Self {
        let function_id = FunctionId(frame.function_id);
        let callback_id = callback_id_for(function_id, &frame.payload);
        let kind = match frame.frame_type {
            SerialFrameType::Response => SerialMessageKind::Response,
            SerialFrameType::Request if callback_id.is_some() => SerialMessageKind::Callback,
            SerialFrameType::Request => SerialMessageKind::Request,
        };
        Self {
            kind,
            function_id,
            callback_id,
            payload: frame.payload,
        }
    }

    pub fn request(function_id: FunctionId, payload: Vec<u8>) -> Self {
        Self {
            kind: SerialMessageKind::Request,
            function_id,
            callback_id: callback_id_for(function_id, &payload),
            payload,
        }
    }

    pub fn to_frame(&self) -> SerialFrame {
        let frame_type = match self.kind {
            SerialMessageKind::Response => SerialFrameType::Response,
            SerialMessageKind::Request | SerialMessageKind::Callback => SerialFrameType::Request,
        };
        SerialFrame::new(frame_type, self.function_id.0, self.payload.clone())
    }

    pub fn encode(&self) -> Result<Vec<u8>, ZWaveError> {
        self.to_frame().encode()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerCapabilities {
    pub is_secondary: bool,
    pub is_sis_present: bool,
    pub was_real_primary: bool,
    pub is_suc: bool,
    pub supports_timers: bool,
}

impl ControllerCapabilities {
    pub fn parse(flags: u8) -> Self {
        Self {
            is_secondary: flags & 0x01 != 0,
            is_sis_present: flags & 0x02 != 0,
            was_real_primary: flags & 0x04 != 0,
            is_suc: flags & 0x08 != 0,
            supports_timers: flags & 0x10 != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitDataCapabilities {
    pub is_end_device_api: bool,
    pub supports_timers: bool,
    pub is_secondary_controller: bool,
    pub is_sis: bool,
}

impl InitDataCapabilities {
    pub fn parse(flags: u8) -> Self {
        Self {
            is_end_device_api: flags & 0x01 != 0,
            supports_timers: flags & 0x02 != 0,
            is_secondary_controller: flags & 0x04 != 0,
            is_sis: flags & 0x08 != 0,
        }
    }

    pub fn is_controller_api(self) -> bool {
        !self.is_end_device_api
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZWaveChip {
    pub chip_type: u8,
    pub chip_version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialApiInitData {
    pub version: u8,
    pub capabilities: InitDataCapabilities,
    pub nodes: Vec<NodeId>,
    pub chip: ZWaveChip,
}

impl SerialApiInitData {
    pub fn parse(payload: &[u8]) -> Result<Self, SerialApiError> {
        if payload.len() < 5 {
            return Err(SerialApiError::Truncated {
                needed: 5,
                remaining: payload.len(),
            });
        }

        let version = payload[0];
        let capabilities = InitDataCapabilities::parse(payload[1]);
        let node_mask_len = payload[2] as usize;
        let needed = 3 + node_mask_len + 2;
        if payload.len() < needed {
            return Err(SerialApiError::Truncated {
                needed,
                remaining: payload.len(),
            });
        }

        let node_mask = &payload[3..3 + node_mask_len];
        let chip = ZWaveChip {
            chip_type: payload[3 + node_mask_len],
            chip_version: payload[4 + node_mask_len],
        };
        Ok(Self {
            version,
            capabilities,
            nodes: classic_nodes_from_mask(node_mask)?,
            chip,
        })
    }

    pub fn from_message(message: &SerialMessage) -> Result<Self, SerialApiError> {
        expect_function(message, FunctionId::SERIAL_API_GET_INIT_DATA)?;
        Self::parse(&message.payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationCommand {
    pub rx_status: u8,
    pub source_node: NodeId,
    pub command: Vec<u8>,
    pub trailing: Vec<u8>,
}

impl ApplicationCommand {
    pub fn parse(payload: &[u8]) -> Result<Self, SerialApiError> {
        if payload.len() < 3 {
            return Err(SerialApiError::Truncated {
                needed: 3,
                remaining: payload.len(),
            });
        }

        let command_len = payload[2] as usize;
        let needed = 3 + command_len;
        if payload.len() < needed {
            return Err(SerialApiError::Truncated {
                needed,
                remaining: payload.len(),
            });
        }

        Ok(Self {
            rx_status: payload[0],
            source_node: NodeId::classic(payload[1])
                .map_err(|err| SerialApiError::Core(err.to_string()))?,
            command: payload[3..needed].to_vec(),
            trailing: payload[needed..].to_vec(),
        })
    }

    pub fn from_message(message: &SerialMessage) -> Result<Self, SerialApiError> {
        expect_function(message, FunctionId::APPLICATION_COMMAND_HANDLER)?;
        Self::parse(&message.payload)
    }

    pub fn command_class_id(&self) -> Option<CommandClassId> {
        self.command
            .first()
            .map(|id| CommandClassId(u16::from(*id)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryId {
    pub home_id: HomeId,
    pub controller_node_id: NodeId,
}

impl MemoryId {
    pub fn parse(payload: &[u8]) -> Result<Self, SerialApiError> {
        if payload.len() < 5 {
            return Err(SerialApiError::Truncated {
                needed: 5,
                remaining: payload.len(),
            });
        }
        Ok(Self {
            home_id: HomeId(u32::from_be_bytes([
                payload[0], payload[1], payload[2], payload[3],
            ])),
            controller_node_id: NodeId::classic(payload[4])
                .map_err(|err| SerialApiError::Core(err.to_string()))?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestKey {
    pub function_id: FunctionId,
    pub callback_id: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRequest {
    pub key: RequestKey,
    pub sent_at_ms: u64,
    pub timeout_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RequestTracker {
    pending: BTreeMap<RequestKey, PendingRequest>,
}

impl RequestTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track(
        &mut self,
        message: &SerialMessage,
        sent_at_ms: u64,
        timeout_ms: u64,
    ) -> Result<RequestKey, SerialApiError> {
        if message.kind != SerialMessageKind::Request {
            return Err(SerialApiError::NotRequest);
        }
        let key = RequestKey {
            function_id: message.function_id,
            callback_id: message.callback_id,
        };
        self.pending.insert(
            key,
            PendingRequest {
                key,
                sent_at_ms,
                timeout_at_ms: sent_at_ms.saturating_add(timeout_ms),
            },
        );
        Ok(key)
    }

    pub fn complete(&mut self, message: &SerialMessage) -> Option<PendingRequest> {
        match message.kind {
            SerialMessageKind::Response => {
                let key = RequestKey {
                    function_id: message.function_id,
                    callback_id: None,
                };
                self.pending
                    .remove(&key)
                    .or_else(|| remove_first_function_match(&mut self.pending, message.function_id))
            }
            SerialMessageKind::Callback => {
                let key = RequestKey {
                    function_id: message.function_id,
                    callback_id: message.callback_id,
                };
                self.pending.remove(&key)
            }
            SerialMessageKind::Request => None,
        }
    }

    pub fn expire(&mut self, now_ms: u64) -> Vec<PendingRequest> {
        let expired_keys: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(key, pending)| (now_ms >= pending.timeout_at_ms).then_some(*key))
            .collect();
        expired_keys
            .into_iter()
            .filter_map(|key| self.pending.remove(&key))
            .collect()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialApiError {
    Truncated {
        needed: usize,
        remaining: usize,
    },
    UnexpectedFunction {
        expected: FunctionId,
        actual: FunctionId,
    },
    NotRequest,
    Core(String),
}

impl fmt::Display for SerialApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Z-Wave Serial API payload: needed {needed} bytes, had {remaining}"
            ),
            Self::UnexpectedFunction { expected, actual } => write!(
                f,
                "expected Z-Wave Serial API function 0x{:02x}, got 0x{:02x}",
                expected.0, actual.0
            ),
            Self::NotRequest => write!(f, "only request messages can be tracked"),
            Self::Core(message) => write!(f, "Z-Wave core error: {message}"),
        }
    }
}

impl std::error::Error for SerialApiError {}

fn callback_id_for(function_id: FunctionId, payload: &[u8]) -> Option<u8> {
    match function_id {
        FunctionId::SEND_DATA | FunctionId::REQUEST_NODE_INFO => payload.last().copied(),
        _ => None,
    }
}

fn expect_function(message: &SerialMessage, expected: FunctionId) -> Result<(), SerialApiError> {
    if message.function_id == expected {
        Ok(())
    } else {
        Err(SerialApiError::UnexpectedFunction {
            expected,
            actual: message.function_id,
        })
    }
}

fn classic_nodes_from_mask(mask: &[u8]) -> Result<Vec<NodeId>, SerialApiError> {
    let mut nodes = Vec::new();
    for (byte_index, byte) in mask.iter().enumerate() {
        for bit_index in 0..8 {
            if byte & (1 << bit_index) != 0 {
                let raw_node_id = byte_index * 8 + bit_index + 1;
                if raw_node_id > u8::MAX as usize {
                    return Err(SerialApiError::Core(format!(
                        "classic Z-Wave node id {raw_node_id} exceeds one byte"
                    )));
                }
                nodes.push(
                    NodeId::classic(raw_node_id as u8)
                        .map_err(|err| SerialApiError::Core(err.to_string()))?,
                );
            }
        }
    }
    Ok(nodes)
}

fn remove_first_function_match(
    pending: &mut BTreeMap<RequestKey, PendingRequest>,
    function_id: FunctionId,
) -> Option<PendingRequest> {
    let key = pending
        .keys()
        .copied()
        .find(|key| key.function_id == function_id)?;
    pending.remove(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_node(mask: &mut [u8], node_id: usize) {
        let zero_based = node_id - 1;
        mask[zero_based / 8] |= 1 << (zero_based % 8);
    }

    #[test]
    fn serial_message_round_trips_through_core_frame() {
        let message = SerialMessage::request(FunctionId::SEND_DATA, vec![0x02, 0x25, 0x01, 0x44]);
        let encoded = message.encode().unwrap();
        let parsed = SerialMessage::from_frame(SerialFrame::parse(&encoded).unwrap());

        assert_eq!(parsed.function_id, FunctionId::SEND_DATA);
        assert_eq!(parsed.kind, SerialMessageKind::Callback);
        assert_eq!(parsed.callback_id, Some(0x44));
    }

    #[test]
    fn controller_capabilities_parse_flags() {
        let caps = ControllerCapabilities::parse(0b0001_1011);

        assert!(caps.is_secondary);
        assert!(caps.is_sis_present);
        assert!(!caps.was_real_primary);
        assert!(caps.is_suc);
        assert!(caps.supports_timers);
    }

    #[test]
    fn serial_api_init_data_extracts_controller_nodes_from_bitmask() {
        let mut node_mask = vec![0; 29];
        set_node(&mut node_mask, 1);
        set_node(&mut node_mask, 5);
        set_node(&mut node_mask, 9);
        set_node(&mut node_mask, 232);
        let mut payload = vec![7, 0b0000_1010, node_mask.len() as u8];
        payload.extend_from_slice(&node_mask);
        payload.extend_from_slice(&[0x08, 0x00]);

        let init = SerialApiInitData::parse(&payload).unwrap();

        assert_eq!(init.version, 7);
        assert!(init.capabilities.is_controller_api());
        assert!(init.capabilities.supports_timers);
        assert!(init.capabilities.is_sis);
        assert_eq!(
            init.nodes,
            vec![
                NodeId::Classic(1),
                NodeId::Classic(5),
                NodeId::Classic(9),
                NodeId::Classic(232)
            ]
        );
        assert_eq!(
            init.chip,
            ZWaveChip {
                chip_type: 0x08,
                chip_version: 0x00
            }
        );
    }

    #[test]
    fn serial_api_init_data_accepts_end_device_payload_without_nodes() {
        let message = SerialMessage {
            kind: SerialMessageKind::Response,
            function_id: FunctionId::SERIAL_API_GET_INIT_DATA,
            callback_id: None,
            payload: vec![7, 0b0000_0001, 0, 0x08, 0x00],
        };

        let init = SerialApiInitData::from_message(&message).unwrap();

        assert!(init.capabilities.is_end_device_api);
        assert!(init.nodes.is_empty());
    }

    #[test]
    fn application_command_handler_wraps_source_and_command_bytes() {
        let message = SerialMessage {
            kind: SerialMessageKind::Request,
            function_id: FunctionId::APPLICATION_COMMAND_HANDLER,
            callback_id: None,
            payload: vec![0x01, 0x05, 0x03, 0x25, 0x03, 0xff, 0x99],
        };

        let command = ApplicationCommand::from_message(&message).unwrap();

        assert_eq!(command.rx_status, 0x01);
        assert_eq!(command.source_node, NodeId::Classic(5));
        assert_eq!(command.command, vec![0x25, 0x03, 0xff]);
        assert_eq!(command.trailing, vec![0x99]);
        assert_eq!(
            command.command_class_id(),
            Some(CommandClassId::SWITCH_BINARY)
        );
    }

    #[test]
    fn typed_payload_helpers_reject_wrong_function_id() {
        let message = SerialMessage::request(FunctionId::GET_VERSION, Vec::new());

        assert_eq!(
            SerialApiInitData::from_message(&message),
            Err(SerialApiError::UnexpectedFunction {
                expected: FunctionId::SERIAL_API_GET_INIT_DATA,
                actual: FunctionId::GET_VERSION
            })
        );
    }

    #[test]
    fn memory_id_extracts_home_and_controller_node() {
        let id = MemoryId::parse(&[0x12, 0x34, 0x56, 0x78, 0x05]).unwrap();

        assert_eq!(id.home_id, HomeId(0x1234_5678));
        assert_eq!(id.controller_node_id, NodeId::Classic(5));
    }

    #[test]
    fn request_tracker_correlates_callback_by_callback_id() {
        let request = SerialMessage::request(FunctionId::SEND_DATA, vec![0x02, 0x25, 0x01, 0x44]);
        let callback = SerialMessage {
            kind: SerialMessageKind::Callback,
            function_id: FunctionId::SEND_DATA,
            callback_id: Some(0x44),
            payload: vec![0x44, 0x00],
        };
        let mut tracker = RequestTracker::new();
        let key = tracker.track(&request, 100, 500).unwrap();

        assert_eq!(key.callback_id, Some(0x44));
        assert!(tracker.complete(&callback).is_some());
        assert_eq!(tracker.pending_len(), 0);
    }

    #[test]
    fn request_tracker_expires_timed_out_requests() {
        let request = SerialMessage::request(FunctionId::GET_VERSION, Vec::new());
        let mut tracker = RequestTracker::new();
        tracker.track(&request, 100, 50).unwrap();

        assert!(tracker.expire(149).is_empty());
        assert_eq!(tracker.expire(150).len(), 1);
        assert_eq!(tracker.pending_len(), 0);
    }
}
