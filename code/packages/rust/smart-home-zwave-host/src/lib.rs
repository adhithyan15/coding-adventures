//! Production serial host for the normalized smart-home Z-Wave runtime.

#![forbid(unsafe_code)]

use serialport::{ClearBuffer, SerialPort};
use smart_home_core::{AgentId, BridgeId, CommandResult, CommandStatus};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeEvent, SmartHomeRuntime};
use smart_home_zwave_integration::{
    InstalledZWaveNode, ZWaveCommandDispatch, ZWaveControllerConfig, ZWaveDispatchState,
    ZWaveIntegrationError, ZWaveNodeInterview, ZWaveRuntimeIntegration, ZWaveSerialOutcome,
};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::io::{self, Read, Write};
use std::time::Duration;
use zwave_core::{SerialFrame, ZWaveError, ACK, CAN, NAK, SOF};
use zwave_serial_api::{
    get_controller_capabilities_request, get_version_request, memory_get_id_request,
    serial_api_get_init_data_request, ControllerCapabilities, MemoryId, SendDataTransactionState,
    SerialApiError, SerialApiInitData, SerialApiVersion, SerialMessage, SerialMessageKind,
};

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_TIMEOUT_MS: u64 = 1_000;
pub const DEFAULT_MAX_RETRIES: u8 = 2;
pub const DEFAULT_MAX_QUEUED_MESSAGES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveHostConfig {
    pub bridge_id: BridgeId,
    pub serial_path: String,
    pub baud_rate: u32,
    pub timeout: Duration,
    pub max_retries: u8,
    pub max_queued_messages: usize,
    pub clear_on_open: bool,
    pub controller_manufacturer: String,
}

impl ZWaveHostConfig {
    pub fn new(bridge_id: BridgeId, serial_path: impl Into<String>) -> Self {
        Self {
            bridge_id,
            serial_path: serial_path.into(),
            baud_rate: DEFAULT_BAUD_RATE,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            max_retries: DEFAULT_MAX_RETRIES,
            max_queued_messages: DEFAULT_MAX_QUEUED_MESSAGES,
            clear_on_open: true,
            controller_manufacturer: "Z-Wave".to_string(),
        }
    }

    pub fn baud_rate(mut self, baud_rate: u32) -> Self {
        self.baud_rate = baud_rate;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn max_retries(mut self, max_retries: u8) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn max_queued_messages(mut self, max_queued_messages: usize) -> Self {
        self.max_queued_messages = max_queued_messages;
        self
    }

    pub fn clear_on_open(mut self, clear_on_open: bool) -> Self {
        self.clear_on_open = clear_on_open;
        self
    }

    pub fn controller_manufacturer(mut self, manufacturer: impl Into<String>) -> Self {
        self.controller_manufacturer = manufacturer.into();
        self
    }

    fn validate(&self) -> Result<(), ZWaveHostError> {
        if self.serial_path.trim().is_empty() {
            return Err(ZWaveHostError::Validation(
                "serial path must not be empty".to_string(),
            ));
        }
        if self.baud_rate == 0 {
            return Err(ZWaveHostError::Validation(
                "baud rate must be greater than zero".to_string(),
            ));
        }
        if self.timeout.is_zero() {
            return Err(ZWaveHostError::Validation(
                "serial timeout must be greater than zero".to_string(),
            ));
        }
        if self.max_queued_messages == 0 {
            return Err(ZWaveHostError::Validation(
                "queued message limit must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveControllerBootstrap {
    pub version: SerialApiVersion,
    pub memory_id: MemoryId,
    pub capabilities: ControllerCapabilities,
    pub init_data: SerialApiInitData,
}

impl ZWaveControllerBootstrap {
    pub fn known_node_count(&self) -> usize {
        self.init_data.nodes.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveHostCommandReport {
    pub dispatch: ZWaveCommandDispatch,
    pub response_state: ZWaveDispatchState,
    pub completed: Option<CommandResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZWaveHostPumpReport {
    pub serial_outcome: ZWaveSerialOutcome,
    pub completed: Option<CommandResult>,
}

enum WireItem {
    Ack,
    Nak,
    Can,
    Message(SerialMessage),
}

pub struct ZWaveSerialSession<S> {
    stream: S,
    max_retries: u8,
    max_queued_messages: usize,
    queued_messages: VecDeque<SerialMessage>,
}

impl<S> ZWaveSerialSession<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            max_retries: DEFAULT_MAX_RETRIES,
            max_queued_messages: DEFAULT_MAX_QUEUED_MESSAGES,
            queued_messages: VecDeque::new(),
        }
    }

    pub fn with_limits(
        stream: S,
        max_retries: u8,
        max_queued_messages: usize,
    ) -> Result<Self, ZWaveHostError> {
        if max_queued_messages == 0 {
            return Err(ZWaveHostError::Validation(
                "queued message limit must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            stream,
            max_retries,
            max_queued_messages,
            queued_messages: VecDeque::new(),
        })
    }

    pub fn queued_message_count(&self) -> usize {
        self.queued_messages.len()
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: Read + Write> ZWaveSerialSession<S> {
    pub fn request(&mut self, request: &SerialMessage) -> Result<SerialMessage, ZWaveHostError> {
        if request.kind != SerialMessageKind::Request {
            return Err(ZWaveHostError::Validation(
                "serial exchange requires a request message".to_string(),
            ));
        }
        let encoded = request.encode()?;
        let mut attempts = 0u16;
        let max_attempts = u16::from(self.max_retries) + 1;

        'transmit: loop {
            attempts += 1;
            self.stream.write_all(&encoded)?;
            self.stream.flush()?;
            loop {
                match self.read_wire_item()? {
                    WireItem::Ack => break 'transmit,
                    WireItem::Nak | WireItem::Can if attempts < max_attempts => {
                        continue 'transmit;
                    }
                    WireItem::Nak => {
                        return Err(ZWaveHostError::RetryExhausted {
                            control: NAK,
                            attempts,
                        });
                    }
                    WireItem::Can => {
                        return Err(ZWaveHostError::RetryExhausted {
                            control: CAN,
                            attempts,
                        });
                    }
                    WireItem::Message(message) => self.queue_message(message)?,
                }
            }
        }

        loop {
            let message = self.read_next_message()?;
            if message.kind == SerialMessageKind::Response
                && message.function_id == request.function_id
            {
                return Ok(message);
            }
            self.queue_message(message)?;
        }
    }

    pub fn receive(&mut self) -> Result<SerialMessage, ZWaveHostError> {
        if let Some(message) = self.queued_messages.pop_front() {
            return Ok(message);
        }
        self.read_next_message()
    }

    fn read_next_message(&mut self) -> Result<SerialMessage, ZWaveHostError> {
        match self.read_wire_item()? {
            WireItem::Message(message) => Ok(message),
            WireItem::Ack => Err(ZWaveHostError::UnexpectedControl(ACK)),
            WireItem::Nak => Err(ZWaveHostError::UnexpectedControl(NAK)),
            WireItem::Can => Err(ZWaveHostError::UnexpectedControl(CAN)),
        }
    }

    fn read_wire_item(&mut self) -> Result<WireItem, ZWaveHostError> {
        let mut first = [0u8; 1];
        self.stream.read_exact(&mut first)?;
        match first[0] {
            ACK => Ok(WireItem::Ack),
            NAK => Ok(WireItem::Nak),
            CAN => Ok(WireItem::Can),
            SOF => {
                let mut length = [0u8; 1];
                self.stream.read_exact(&mut length)?;
                let remaining = usize::from(length[0]);
                if remaining < 3 {
                    self.reject_frame()?;
                    return Err(ZWaveHostError::InvalidFrameLength(remaining));
                }
                let mut bytes = Vec::with_capacity(remaining + 2);
                bytes.extend_from_slice(&[SOF, length[0]]);
                let mut tail = vec![0u8; remaining];
                self.stream.read_exact(&mut tail)?;
                bytes.extend_from_slice(&tail);
                match SerialFrame::parse(&bytes) {
                    Ok(frame) => {
                        self.stream.write_all(&[ACK])?;
                        self.stream.flush()?;
                        Ok(WireItem::Message(SerialMessage::from_frame(frame)))
                    }
                    Err(error) => {
                        self.reject_frame()?;
                        Err(ZWaveHostError::Core(error))
                    }
                }
            }
            other => Err(ZWaveHostError::UnexpectedControl(other)),
        }
    }

    fn reject_frame(&mut self) -> Result<(), ZWaveHostError> {
        self.stream.write_all(&[NAK])?;
        self.stream.flush()?;
        Ok(())
    }

    fn queue_message(&mut self, message: SerialMessage) -> Result<(), ZWaveHostError> {
        if self.queued_messages.len() >= self.max_queued_messages {
            return Err(ZWaveHostError::QueuedMessageLimit {
                limit: self.max_queued_messages,
            });
        }
        self.queued_messages.push_back(message);
        Ok(())
    }
}

pub struct ZWaveHost<S> {
    config: ZWaveHostConfig,
    session: ZWaveSerialSession<S>,
    runtime: SmartHomeRuntime,
    integration: ZWaveRuntimeIntegration,
    controller: ZWaveControllerBootstrap,
    pending_commands: BTreeMap<u8, CommandResult>,
}

impl<S: Read + Write> ZWaveHost<S> {
    pub fn bootstrap(
        config: ZWaveHostConfig,
        stream: S,
        mut runtime: SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<Self, ZWaveHostError> {
        config.validate()?;
        let mut session = ZWaveSerialSession::with_limits(
            stream,
            config.max_retries,
            config.max_queued_messages,
        )?;
        let controller = bootstrap_controller(&mut session)?;
        if !controller.init_data.capabilities.is_controller_api() {
            return Err(ZWaveHostError::ControllerApiRequired);
        }
        let controller_model = format!(
            "Serial API library 0x{:02x}",
            controller.version.library_type
        );
        let integration_config = ZWaveControllerConfig::new(
            config.bridge_id.clone(),
            controller.memory_id.home_id,
            controller.memory_id.controller_node_id,
            config.serial_path.clone(),
        )
        .with_identity(&config.controller_manufacturer, controller_model)
        .with_firmware_version(&controller.version.version);
        let integration = ZWaveRuntimeIntegration::new(integration_config)?;
        integration.install_controller(&mut runtime, observed_at_ms)?;

        Ok(Self {
            config,
            session,
            runtime,
            integration,
            controller,
            pending_commands: BTreeMap::new(),
        })
    }

    pub fn config(&self) -> &ZWaveHostConfig {
        &self.config
    }

    pub fn controller(&self) -> &ZWaveControllerBootstrap {
        &self.controller
    }

    pub fn runtime(&self) -> &SmartHomeRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut SmartHomeRuntime {
        &mut self.runtime
    }

    pub fn integration(&self) -> &ZWaveRuntimeIntegration {
        &self.integration
    }

    pub fn session(&self) -> &ZWaveSerialSession<S> {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut ZWaveSerialSession<S> {
        &mut self.session
    }

    pub fn install_node(
        &mut self,
        interview: ZWaveNodeInterview,
    ) -> Result<InstalledZWaveNode, ZWaveHostError> {
        Ok(self
            .integration
            .install_node(&mut self.runtime, interview)?)
    }

    pub fn dispatch_command(
        &mut self,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<ZWaveHostCommandReport, ZWaveHostError> {
        let dispatch =
            self.integration
                .dispatch_command(&mut self.runtime, principal_id, request, now_ms)?;
        self.pending_commands
            .insert(dispatch.callback_id, dispatch.command_result.clone());
        let response = match self.session.request(&dispatch.serial_message) {
            Ok(response) => response,
            Err(error) => {
                self.publish_transport_failure(dispatch.callback_id, &error);
                return Err(error);
            }
        };
        let outcome =
            self.integration
                .handle_serial_message(&mut self.runtime, &response, now_ms)?;
        let ZWaveSerialOutcome::DispatchState(response_state) = outcome else {
            return Err(ZWaveHostError::UnexpectedIntegrationOutcome);
        };
        let completed = self.complete_dispatch_if_terminal(&response_state);
        Ok(ZWaveHostCommandReport {
            dispatch,
            response_state,
            completed,
        })
    }

    pub fn pump_once(
        &mut self,
        observed_at_ms: u64,
    ) -> Result<ZWaveHostPumpReport, ZWaveHostError> {
        let message = self.session.receive()?;
        let serial_outcome =
            self.integration
                .handle_serial_message(&mut self.runtime, &message, observed_at_ms)?;
        let completed = match &serial_outcome {
            ZWaveSerialOutcome::DispatchState(state) => self.complete_dispatch_if_terminal(state),
            ZWaveSerialOutcome::StateEvent(_) | ZWaveSerialOutcome::Ignored => None,
        };
        Ok(ZWaveHostPumpReport {
            serial_outcome,
            completed,
        })
    }

    pub fn expire_commands(&mut self, now_ms: u64) -> Vec<CommandResult> {
        let states = self.integration.expire_dispatches(now_ms);
        states
            .iter()
            .filter_map(|state| self.complete_dispatch_if_terminal(state))
            .collect()
    }

    pub fn into_parts(
        self,
    ) -> (
        ZWaveSerialSession<S>,
        SmartHomeRuntime,
        ZWaveRuntimeIntegration,
    ) {
        (self.session, self.runtime, self.integration)
    }

    fn complete_dispatch_if_terminal(
        &mut self,
        state: &ZWaveDispatchState,
    ) -> Option<CommandResult> {
        let status = match state.state {
            SendDataTransactionState::Succeeded => CommandStatus::Accepted,
            SendDataTransactionState::Failed(_) => CommandStatus::Failed,
            SendDataTransactionState::TimedOut => CommandStatus::TimedOut,
            SendDataTransactionState::AwaitingResponse
            | SendDataTransactionState::AwaitingCallback => return None,
        };
        let accepted = self.pending_commands.remove(&state.callback_id)?;
        let message = match state.state {
            SendDataTransactionState::Succeeded => "Z-Wave controller applied command".to_string(),
            SendDataTransactionState::Failed(error) => {
                format!("Z-Wave command failed: {error:?}")
            }
            SendDataTransactionState::TimedOut => "Z-Wave command timed out".to_string(),
            SendDataTransactionState::AwaitingResponse
            | SendDataTransactionState::AwaitingCallback => unreachable!(),
        };
        let completed = CommandResult {
            command_id: accepted.command_id,
            status,
            bridge_id: accepted.bridge_id,
            correlation_id: accepted.correlation_id,
            message: Some(message),
        };
        self.runtime
            .event_bus_mut()
            .publish(RuntimeEvent::CommandResult(completed.clone()));
        Some(completed)
    }

    fn publish_transport_failure(&mut self, callback_id: u8, error: &ZWaveHostError) {
        let Some(accepted) = self.pending_commands.remove(&callback_id) else {
            return;
        };
        self.runtime
            .event_bus_mut()
            .publish(RuntimeEvent::CommandResult(CommandResult {
                command_id: accepted.command_id,
                status: CommandStatus::Failed,
                bridge_id: accepted.bridge_id,
                correlation_id: accepted.correlation_id,
                message: Some(format!("Z-Wave serial transport failed: {error}")),
            }));
    }
}

impl ZWaveHost<Box<dyn SerialPort>> {
    pub fn open(
        config: ZWaveHostConfig,
        runtime: SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<Self, ZWaveHostError> {
        config.validate()?;
        let port = open_serial_port(&config)?;
        Self::bootstrap(config, port, runtime, observed_at_ms)
    }
}

pub fn open_serial_port(config: &ZWaveHostConfig) -> Result<Box<dyn SerialPort>, ZWaveHostError> {
    config.validate()?;
    let port = serialport::new(&config.serial_path, config.baud_rate)
        .timeout(config.timeout)
        .data_bits(serialport::DataBits::Eight)
        .flow_control(serialport::FlowControl::None)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .open()
        .map_err(ZWaveHostError::Open)?;
    if config.clear_on_open {
        port.clear(ClearBuffer::All)
            .map_err(ZWaveHostError::Configure)?;
    }
    Ok(port)
}

pub fn bootstrap_controller<S: Read + Write>(
    session: &mut ZWaveSerialSession<S>,
) -> Result<ZWaveControllerBootstrap, ZWaveHostError> {
    let version = SerialApiVersion::from_message(&session.request(&get_version_request())?)?;
    let memory_id = MemoryId::from_message(&session.request(&memory_get_id_request())?)?;
    let capabilities = ControllerCapabilities::from_message(
        &session.request(&get_controller_capabilities_request())?,
    )?;
    let init_data =
        SerialApiInitData::from_message(&session.request(&serial_api_get_init_data_request())?)?;
    Ok(ZWaveControllerBootstrap {
        version,
        memory_id,
        capabilities,
        init_data,
    })
}

#[derive(Debug)]
pub enum ZWaveHostError {
    Validation(String),
    Open(serialport::Error),
    Configure(serialport::Error),
    Io(io::Error),
    Core(ZWaveError),
    SerialApi(SerialApiError),
    Integration(ZWaveIntegrationError),
    InvalidFrameLength(usize),
    UnexpectedControl(u8),
    RetryExhausted { control: u8, attempts: u16 },
    QueuedMessageLimit { limit: usize },
    ControllerApiRequired,
    UnexpectedIntegrationOutcome,
}

impl fmt::Display for ZWaveHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Z-Wave host: {message}"),
            Self::Open(error) => write!(formatter, "failed to open Z-Wave serial port: {error}"),
            Self::Configure(error) => {
                write!(formatter, "failed to configure Z-Wave serial port: {error}")
            }
            Self::Io(error) => write!(formatter, "Z-Wave serial I/O failed: {error}"),
            Self::Core(error) => error.fmt(formatter),
            Self::SerialApi(error) => error.fmt(formatter),
            Self::Integration(error) => error.fmt(formatter),
            Self::InvalidFrameLength(length) => {
                write!(formatter, "invalid Z-Wave serial frame length {length}")
            }
            Self::UnexpectedControl(control) => {
                write!(
                    formatter,
                    "unexpected Z-Wave serial control byte 0x{control:02x}"
                )
            }
            Self::RetryExhausted { control, attempts } => write!(
                formatter,
                "Z-Wave controller returned 0x{control:02x}; exhausted {attempts} attempts"
            ),
            Self::QueuedMessageLimit { limit } => write!(
                formatter,
                "Z-Wave unsolicited message queue reached its limit of {limit}"
            ),
            Self::ControllerApiRequired => {
                write!(formatter, "Z-Wave Serial API reports an end-device API")
            }
            Self::UnexpectedIntegrationOutcome => {
                write!(
                    formatter,
                    "Z-Wave integration did not return a dispatch state"
                )
            }
        }
    }
}

impl std::error::Error for ZWaveHostError {}

impl From<io::Error> for ZWaveHostError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ZWaveError> for ZWaveHostError {
    fn from(error: ZWaveError) -> Self {
        Self::Core(error)
    }
}

impl From<SerialApiError> for ZWaveHostError {
    fn from(error: SerialApiError) -> Self {
        Self::SerialApi(error)
    }
}

impl From<ZWaveIntegrationError> for ZWaveHostError {
    fn from(error: ZWaveIntegrationError) -> Self {
        Self::Integration(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        CapabilityGrant, CapabilityGrantId, CommandType, IntegrationId, PrivilegeTier, Value,
    };
    use std::collections::VecDeque;
    use zwave_core::{CommandClassId, NodeId, SerialFrameType};
    use zwave_serial_api::FunctionId;

    #[derive(Default)]
    struct ScriptedStream {
        read: VecDeque<u8>,
        written: Vec<u8>,
    }

    impl ScriptedStream {
        fn with_read(bytes: Vec<u8>) -> Self {
            Self {
                read: bytes.into(),
                written: Vec::new(),
            }
        }
    }

    impl Read for ScriptedStream {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.read.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "script exhausted",
                ));
            }
            let count = output.len().min(self.read.len());
            for slot in &mut output[..count] {
                *slot = self
                    .read
                    .pop_front()
                    .expect("count is bounded by queue length");
            }
            Ok(count)
        }
    }

    impl Write for ScriptedStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn response(function_id: FunctionId, payload: Vec<u8>) -> Vec<u8> {
        SerialFrame::new(SerialFrameType::Response, function_id.0, payload)
            .encode()
            .unwrap()
    }

    fn callback(function_id: FunctionId, payload: Vec<u8>) -> Vec<u8> {
        SerialFrame::new(SerialFrameType::Request, function_id.0, payload)
            .encode()
            .unwrap()
    }

    fn exchange_bytes(function_id: FunctionId, payload: Vec<u8>) -> Vec<u8> {
        let mut bytes = vec![ACK];
        bytes.extend(response(function_id, payload));
        bytes
    }

    fn bootstrap_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(exchange_bytes(
            FunctionId::GET_VERSION,
            [b"Z-Wave 7.19\0".as_slice(), &[0x07]].concat(),
        ));
        bytes.extend(exchange_bytes(
            FunctionId::MEMORY_GET_ID,
            vec![0xde, 0xad, 0xbe, 0xef, 0x01],
        ));
        bytes.extend(exchange_bytes(
            FunctionId::GET_CONTROLLER_CAPABILITIES,
            vec![0x1a],
        ));
        bytes.extend(exchange_bytes(
            FunctionId::SERIAL_API_GET_INIT_DATA,
            vec![1, 0, 1, 0x11, 7, 1],
        ));
        bytes
    }

    fn host_with_extra(extra: Vec<u8>) -> ZWaveHost<ScriptedStream> {
        let mut bytes = bootstrap_bytes();
        bytes.extend(extra);
        ZWaveHost::bootstrap(
            ZWaveHostConfig::new(BridgeId::trusted("zwave-controller"), "/dev/ttyUSB0"),
            ScriptedStream::with_read(bytes),
            SmartHomeRuntime::new(),
            1_000,
        )
        .unwrap()
    }

    #[test]
    fn serial_session_retries_nak_and_acknowledges_valid_response() {
        let request = get_version_request();
        let encoded_request = request.encode().unwrap();
        let mut bytes = vec![NAK, ACK];
        bytes.extend(response(
            FunctionId::GET_VERSION,
            [b"Z-Wave 7.19\0".as_slice(), &[0x07]].concat(),
        ));
        let mut session =
            ZWaveSerialSession::with_limits(ScriptedStream::with_read(bytes), 1, 8).unwrap();

        let message = session.request(&request).unwrap();
        let stream = session.into_inner();

        assert_eq!(message.function_id, FunctionId::GET_VERSION);
        assert_eq!(
            stream.written,
            [
                encoded_request.as_slice(),
                encoded_request.as_slice(),
                &[ACK]
            ]
            .concat()
        );
    }

    #[test]
    fn malformed_controller_frame_is_naked() {
        let request = get_version_request();
        let mut malformed = response(FunctionId::GET_VERSION, vec![1, 2, 3]);
        let checksum = malformed.len() - 1;
        malformed[checksum] ^= 1;
        let mut bytes = vec![ACK];
        bytes.extend(malformed);
        let mut session = ZWaveSerialSession::new(ScriptedStream::with_read(bytes));

        let error = session.request(&request).unwrap_err();
        let stream = session.into_inner();

        assert!(matches!(error, ZWaveHostError::Core(_)));
        assert_eq!(stream.written.last(), Some(&NAK));
    }

    #[test]
    fn unsolicited_frames_are_queued_while_waiting_for_response() {
        let application = callback(
            FunctionId::APPLICATION_COMMAND_HANDLER,
            vec![1, 5, 3, 0x25, 0x03, 0xff],
        );
        let mut bytes = vec![ACK];
        bytes.extend(application);
        bytes.extend(response(FunctionId::GET_VERSION, vec![b'7', 0, 7]));
        let mut session = ZWaveSerialSession::new(ScriptedStream::with_read(bytes));

        let response = session.request(&get_version_request()).unwrap();
        assert_eq!(response.kind, SerialMessageKind::Response);
        assert_eq!(session.queued_message_count(), 1);
        let queued = session.receive().unwrap();
        assert_eq!(queued.function_id, FunctionId::APPLICATION_COMMAND_HANDLER);
        assert_eq!(session.queued_message_count(), 0);
    }

    #[test]
    fn bootstrap_installs_controller_and_preserves_inventory() {
        let host = host_with_extra(Vec::new());

        assert_eq!(host.controller().version.version, "Z-Wave 7.19");
        assert_eq!(host.controller().memory_id.home_id.0, 0xdead_beef);
        assert_eq!(
            host.controller().memory_id.controller_node_id,
            NodeId::classic(1).unwrap()
        );
        assert_eq!(
            host.controller().init_data.nodes,
            vec![NodeId::classic(1).unwrap(), NodeId::classic(5).unwrap()]
        );
        assert_eq!(host.controller().known_node_count(), 2);
        let bridge = host
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("zwave-controller"))
            .unwrap();
        assert_eq!(bridge.integration_id, IntegrationId::trusted("zwave"));
        assert_eq!(bridge.address.as_deref(), Some("/dev/ttyUSB0"));
    }

    #[test]
    fn authorized_command_crosses_wire_and_callback_publishes_completion() {
        let callback_id = 1u8;
        let mut extra = exchange_bytes(FunctionId::SEND_DATA, vec![1]);
        extra.extend(callback(FunctionId::SEND_DATA, vec![callback_id, 0]));
        let mut host = host_with_extra(extra);
        let installed = host
            .install_node(ZWaveNodeInterview::new(
                NodeId::classic(5).unwrap(),
                "Kitchen Switch",
                [CommandClassId::SWITCH_BINARY],
            ))
            .unwrap();
        let principal = AgentId::trusted("agent:zwave-host-test");
        host.runtime_mut().registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:zwave-host-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                0,
            ),
        );

        let command = host
            .dispatch_command(
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                2_000,
            )
            .unwrap();
        assert_eq!(command.dispatch.callback_id, callback_id);
        assert_eq!(
            command.response_state.state,
            SendDataTransactionState::AwaitingCallback
        );
        assert!(command.completed.is_none());

        let pumped = host.pump_once(2_010).unwrap();
        let completed = pumped.completed.unwrap();
        assert_eq!(completed.status, CommandStatus::Accepted);
        assert_eq!(host.integration().pending_dispatch_count(), 0);
        assert!(host.runtime().event_bus().published().iter().any(|event| {
            matches!(
                event,
                RuntimeEvent::CommandResult(result)
                    if result.command_id == completed.command_id
                        && result.message.as_deref()
                            == Some("Z-Wave controller applied command")
            )
        }));
    }

    #[test]
    fn command_timeouts_publish_terminal_runtime_results() {
        let mut host = host_with_extra(exchange_bytes(FunctionId::SEND_DATA, vec![1]));
        let installed = host
            .install_node(ZWaveNodeInterview::new(
                NodeId::classic(5).unwrap(),
                "Kitchen Switch",
                [CommandClassId::SWITCH_BINARY],
            ))
            .unwrap();
        let principal = AgentId::trusted("agent:zwave-timeout-test");
        host.runtime_mut().registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:zwave-timeout-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                0,
            ),
        );
        host.dispatch_command(
            principal,
            RuntimeCommandToolRequest::new(installed.entity_id, CommandType::TurnOff, Value::Null)
                .with_timeout_ms(100),
            3_000,
        )
        .unwrap();

        assert!(host.expire_commands(3_099).is_empty());
        let expired = host.expire_commands(3_100);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].status, CommandStatus::TimedOut);
        assert!(host.expire_commands(3_101).is_empty());
    }

    #[test]
    fn application_reports_cross_the_serial_pump_into_runtime_state() {
        let application = callback(
            FunctionId::APPLICATION_COMMAND_HANDLER,
            vec![1, 5, 3, 0x25, 0x03, 0xff],
        );
        let mut host = host_with_extra(application);
        let installed = host
            .install_node(ZWaveNodeInterview::new(
                NodeId::classic(5).unwrap(),
                "Kitchen Switch",
                [CommandClassId::SWITCH_BINARY],
            ))
            .unwrap();

        let pumped = host.pump_once(4_000).unwrap();

        assert!(matches!(
            pumped.serial_outcome,
            ZWaveSerialOutcome::StateEvent(_)
        ));
        assert_eq!(
            host.runtime()
                .registry()
                .state(&installed.entity_id)
                .unwrap()
                .value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))])
        );
    }

    #[test]
    fn serial_failure_publishes_failed_command_result() {
        let mut host = host_with_extra(Vec::new());
        let installed = host
            .install_node(ZWaveNodeInterview::new(
                NodeId::classic(5).unwrap(),
                "Kitchen Switch",
                [CommandClassId::SWITCH_BINARY],
            ))
            .unwrap();
        let principal = AgentId::trusted("agent:zwave-serial-failure-test");
        host.runtime_mut().registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:zwave-serial-failure-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                0,
            ),
        );

        let error = host
            .dispatch_command(
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                5_000,
            )
            .unwrap_err();

        assert!(matches!(error, ZWaveHostError::Io(_)));
        assert!(host.runtime().event_bus().published().iter().any(|event| {
            matches!(
                event,
                RuntimeEvent::CommandResult(result)
                    if result.status == CommandStatus::Failed
                        && result
                            .message
                            .as_deref()
                            .is_some_and(|message| message.contains("serial transport failed"))
            )
        }));
    }

    #[test]
    fn invalid_host_config_is_rejected_before_serial_io() {
        let result = ZWaveHost::bootstrap(
            ZWaveHostConfig::new(BridgeId::trusted("bridge"), ""),
            ScriptedStream::default(),
            SmartHomeRuntime::new(),
            0,
        );
        assert!(matches!(result, Err(ZWaveHostError::Validation(_))));
    }
}
