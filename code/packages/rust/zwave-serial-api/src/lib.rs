//! Z-Wave Serial API host/controller primitives.
//!
//! `zwave-core` owns raw Serial API frames. This crate starts the host-side
//! control-plane layer: function ids, request/response/callback classification,
//! controller capability snapshots, and callback correlation.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use zwave_core::{HomeId, NodeId, SerialFrame, SerialFrameType, ZWaveError};

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
    Truncated { needed: usize, remaining: usize },
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
