use board_vm_ir::{
    parse_module, CapabilitySet, CAP_GPIO_CLOSE, CAP_GPIO_OPEN, CAP_GPIO_READ, CAP_GPIO_WRITE,
    CAP_TIME_SLEEP_MS,
};
use board_vm_protocol::{
    decode_frame, decode_hello, decode_program_begin, decode_program_chunk, decode_program_end,
    decode_run_request, encode_caps_report, encode_error_payload, encode_frame, encode_hello_ack,
    encode_program_begin, encode_program_chunk, encode_program_end, encode_run_report_header,
    CapabilityDescriptor, CapsReportHeader, ErrorPayload, Frame, HelloAck, MessageType,
    ProgramBegin, ProgramChunk, ProgramEnd, ProtocolError, RunReportHeader,
    RunStatus as ProtocolRunStatus, CAP_FLAG_BYTECODE_CALLABLE, CAP_FLAG_PROTOCOL_FEATURE,
    CAP_PROGRAM_RAM_EXEC, FLAG_IS_ERROR_RESPONSE, FLAG_IS_RESPONSE, NO_BYTECODE_OFFSET,
    NO_PROGRAM_ID, RUN_FLAG_BACKGROUND_RUN, RUN_FLAG_RESET_VM_BEFORE_RUN,
};
use board_vm_runtime::{
    BoardHal, GpioMode, HalError, Level, RunStatus as RuntimeRunStatus, Runtime, RuntimeErrorKind,
};

pub const LOOPBACK_BOARD_ID: &str = "loopback-uno-r4";
pub const LOOPBACK_RUNTIME_ID: &str = "board-vm-loopback";
pub const LOOPBACK_BOARD_NONCE: u32 = 0xB04D_1001;
pub const DEFAULT_MAX_FRAME_PAYLOAD: u16 = 256;

const ERROR_INVALID_FRAME: u16 = 0x0001;
const ERROR_UNSUPPORTED_MESSAGE: u16 = 0x0003;
const ERROR_PAYLOAD_TOO_LARGE: u16 = 0x0004;
const ERROR_BAD_CRC: u16 = 0x0005;
const ERROR_INVALID_PROGRAM: u16 = 0x0200;
const ERROR_INVALID_BYTECODE: u16 = 0x0201;
const ERROR_BOARD_FAULT: u16 = 0x0400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackError {
    Protocol(ProtocolError),
    OutputTooSmall,
}

impl From<ProtocolError> for LoopbackError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeEvent {
    GpioOpen {
        pin: u16,
        mode: GpioMode,
        token: u32,
    },
    GpioWrite {
        token: u32,
        level: Level,
    },
    GpioRead {
        token: u32,
    },
    GpioClose {
        token: u32,
    },
    SleepMs(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeHal {
    now_ms: u32,
    next_token: u32,
    events: Vec<FakeEvent>,
}

impl FakeHal {
    pub fn new() -> Self {
        Self {
            now_ms: 0,
            next_token: 1,
            events: Vec::new(),
        }
    }

    pub fn now_ms(&self) -> u32 {
        self.now_ms
    }

    pub fn events(&self) -> &[FakeEvent] {
        &self.events
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl Default for FakeHal {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardHal for FakeHal {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::blink_mvp()
    }

    fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
        if !matches!(mode, GpioMode::Output | GpioMode::Input) {
            return Err(HalError::UnsupportedMode);
        }
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1).max(1);
        self.events.push(FakeEvent::GpioOpen { pin, mode, token });
        Ok(token)
    }

    fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
        self.events.push(FakeEvent::GpioWrite { token, level });
        Ok(())
    }

    fn gpio_read(&mut self, token: u32) -> Result<Level, HalError> {
        self.events.push(FakeEvent::GpioRead { token });
        Ok(Level::Low)
    }

    fn gpio_close(&mut self, token: u32) -> Result<(), HalError> {
        self.events.push(FakeEvent::GpioClose { token });
        Ok(())
    }

    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
        self.now_ms = self.now_ms.wrapping_add(duration_ms as u32);
        self.events.push(FakeEvent::SleepMs(duration_ms));
        Ok(())
    }

    fn now_ms(&self) -> u32 {
        self.now_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UploadState {
    program_id: u16,
    expected_len: usize,
    expected_crc32: u32,
    received_len: usize,
    active: bool,
}

impl Default for UploadState {
    fn default() -> Self {
        Self {
            program_id: 0,
            expected_len: 0,
            expected_crc32: 0,
            received_len: 0,
            active: false,
        }
    }
}

pub struct LoopbackBoard<
    const MAX_PROGRAM_BYTES: usize,
    const MAX_STACK: usize,
    const MAX_HANDLES: usize,
> {
    runtime: Runtime<FakeHal, MAX_STACK, MAX_HANDLES>,
    program: [u8; MAX_PROGRAM_BYTES],
    program_len: usize,
    program_id: u16,
    upload: UploadState,
}

impl<const MAX_PROGRAM_BYTES: usize, const MAX_STACK: usize, const MAX_HANDLES: usize>
    LoopbackBoard<MAX_PROGRAM_BYTES, MAX_STACK, MAX_HANDLES>
{
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(FakeHal::new()),
            program: [0; MAX_PROGRAM_BYTES],
            program_len: 0,
            program_id: 0,
            upload: UploadState::default(),
        }
    }

    pub fn fake_hal(&self) -> &FakeHal {
        self.runtime.hal()
    }

    pub fn fake_hal_mut(&mut self) -> &mut FakeHal {
        self.runtime.hal_mut()
    }

    pub fn handle_raw_frame(
        &mut self,
        raw_frame: &[u8],
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, LoopbackError> {
        let request = decode_frame(raw_frame)?;
        let result = self.dispatch(request, payload_out, frame_out);
        match result {
            Ok(len) => Ok(len),
            Err(error) => self.write_error_response(request.request_id, error, frame_out),
        }
    }

    fn dispatch(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        match request.message_type {
            MessageType::HELLO => self.handle_hello(request, payload_out, frame_out),
            MessageType::CAPS_QUERY => self.handle_caps_query(request, payload_out, frame_out),
            MessageType::PROGRAM_BEGIN => {
                self.handle_program_begin(request, payload_out, frame_out)
            }
            MessageType::PROGRAM_CHUNK => {
                self.handle_program_chunk(request, payload_out, frame_out)
            }
            MessageType::PROGRAM_END => self.handle_program_end(request, payload_out, frame_out),
            MessageType::RUN => self.handle_run(request, payload_out, frame_out),
            MessageType::STOP => self.handle_stop(request, payload_out, frame_out),
            _ => Err(BoardError::UnsupportedMessage),
        }
    }

    fn handle_hello(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        let hello = decode_hello(request.payload)?;
        if hello.min_version > 1 || hello.max_version < 1 {
            return Err(BoardError::InvalidFrame);
        }
        let payload_len = encode_hello_ack(
            &HelloAck {
                selected_version: 1,
                board_name: LOOPBACK_BOARD_ID,
                runtime_name: LOOPBACK_RUNTIME_ID,
                host_nonce: hello.host_nonce,
                board_nonce: LOOPBACK_BOARD_NONCE,
                max_frame_payload: DEFAULT_MAX_FRAME_PAYLOAD,
            },
            payload_out,
        )?;
        write_response(
            MessageType::HELLO_ACK,
            request.request_id,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    fn handle_caps_query(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        if !request.payload.is_empty() {
            return Err(BoardError::InvalidFrame);
        }
        let caps = capability_descriptors();
        let payload_len = encode_caps_report(
            &CapsReportHeader {
                board_id: LOOPBACK_BOARD_ID,
                runtime_id: LOOPBACK_RUNTIME_ID,
                max_program_bytes: MAX_PROGRAM_BYTES as u32,
                max_stack_values: MAX_STACK as u8,
                max_handles: MAX_HANDLES as u8,
                supports_store_program: false,
                capability_count: caps.len() as u32,
            },
            &caps,
            payload_out,
        )?;
        write_response(
            MessageType::CAPS_REPORT,
            request.request_id,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    fn handle_program_begin(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        let begin = decode_program_begin(request.payload)?;
        if begin.total_len as usize > MAX_PROGRAM_BYTES {
            return Err(BoardError::PayloadTooLarge);
        }
        self.upload = UploadState {
            program_id: begin.program_id,
            expected_len: begin.total_len as usize,
            expected_crc32: begin.program_crc32,
            received_len: 0,
            active: true,
        };
        self.program_len = 0;
        write_program_begin_response(begin, request.request_id, payload_out, frame_out)
    }

    fn handle_program_chunk(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        let chunk = decode_program_chunk(request.payload)?;
        if !self.upload.active || chunk.program_id != self.upload.program_id {
            return Err(BoardError::InvalidProgram);
        }
        let offset = chunk.offset as usize;
        let end = offset
            .checked_add(chunk.bytes.len())
            .ok_or(BoardError::PayloadTooLarge)?;
        if end > self.upload.expected_len || end > MAX_PROGRAM_BYTES {
            return Err(BoardError::PayloadTooLarge);
        }
        self.program[offset..end].copy_from_slice(chunk.bytes);
        self.upload.received_len = self.upload.received_len.max(end);
        write_program_chunk_response(chunk, request.request_id, payload_out, frame_out)
    }

    fn handle_program_end(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        let end = decode_program_end(request.payload)?;
        if !self.upload.active || end.program_id != self.upload.program_id {
            return Err(BoardError::InvalidProgram);
        }
        if self.upload.received_len != self.upload.expected_len {
            return Err(BoardError::InvalidProgram);
        }
        let program = &self.program[..self.upload.expected_len];
        if crc32_ieee(program) != self.upload.expected_crc32 {
            return Err(BoardError::InvalidProgram);
        }
        let module = parse_module(program).map_err(|_| BoardError::InvalidBytecode)?;
        board_vm_ir::validate(&module, CapabilitySet::blink_mvp(), MAX_STACK as u8)
            .map_err(|_| BoardError::InvalidBytecode)?;
        self.program_len = self.upload.expected_len;
        self.program_id = end.program_id;
        self.upload.active = false;
        write_program_end_response(end, request.request_id, payload_out, frame_out)
    }

    fn handle_run(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        let run = decode_run_request(request.payload)?;
        if run.program_id != self.program_id || self.program_len == 0 {
            return Err(BoardError::InvalidProgram);
        }
        if run.flags & RUN_FLAG_RESET_VM_BEFORE_RUN != 0 {
            self.runtime.reset_vm();
        }

        let module = parse_module(&self.program[..self.program_len])
            .map_err(|_| BoardError::InvalidBytecode)?;
        let report = self
            .runtime
            .run_module(&module, run.instruction_budget)
            .map_err(BoardError::Runtime)?;
        let status = match report.status {
            RuntimeRunStatus::Halted => ProtocolRunStatus::Halted,
            RuntimeRunStatus::BudgetExceeded if run.flags & RUN_FLAG_BACKGROUND_RUN != 0 => {
                ProtocolRunStatus::Running
            }
            RuntimeRunStatus::BudgetExceeded => ProtocolRunStatus::BudgetExceeded,
            RuntimeRunStatus::Faulted => ProtocolRunStatus::Faulted,
        };
        let payload_len = encode_run_report_header(
            &RunReportHeader {
                program_id: run.program_id,
                status,
                instructions_executed: report.instructions_executed,
                elapsed_ms: self.runtime.hal().now_ms(),
                stack_depth: report.stack_depth,
                open_handles: report.open_handles,
                return_count: 0,
            },
            payload_out,
        )?;
        write_response(
            MessageType::RUN_REPORT,
            request.request_id,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    fn handle_stop(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardError> {
        if !request.payload.is_empty() {
            return Err(BoardError::InvalidFrame);
        }
        self.runtime.reset_vm();
        let payload_len = encode_run_report_header(
            &RunReportHeader {
                program_id: self.program_id,
                status: ProtocolRunStatus::Stopped,
                instructions_executed: 0,
                elapsed_ms: self.runtime.hal().now_ms(),
                stack_depth: 0,
                open_handles: 0,
                return_count: 0,
            },
            payload_out,
        )?;
        write_response(
            MessageType::RUN_REPORT,
            request.request_id,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    fn write_error_response(
        &self,
        request_id: u16,
        error: BoardError,
        frame_out: &mut [u8],
    ) -> Result<usize, LoopbackError> {
        let mut payload = [0u8; 96];
        let payload_len = encode_error_payload(
            &ErrorPayload {
                code: error.code(),
                request_id,
                program_id: NO_PROGRAM_ID,
                bytecode_offset: NO_BYTECODE_OFFSET,
                message: error.message(),
            },
            &mut payload,
        )?;
        Ok(encode_frame(
            &Frame {
                flags: FLAG_IS_RESPONSE | FLAG_IS_ERROR_RESPONSE,
                message_type: MessageType::ERROR,
                request_id,
                payload: &payload[..payload_len],
            },
            frame_out,
        )?)
    }
}

impl<const MAX_PROGRAM_BYTES: usize, const MAX_STACK: usize, const MAX_HANDLES: usize> Default
    for LoopbackBoard<MAX_PROGRAM_BYTES, MAX_STACK, MAX_HANDLES>
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoardError {
    Protocol(ProtocolError),
    InvalidFrame,
    UnsupportedMessage,
    PayloadTooLarge,
    InvalidProgram,
    InvalidBytecode,
    Runtime(board_vm_runtime::RuntimeError),
}

impl From<ProtocolError> for BoardError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl BoardError {
    fn code(self) -> u16 {
        match self {
            Self::Protocol(ProtocolError::BadCrc) => ERROR_BAD_CRC,
            Self::Protocol(_) | Self::InvalidFrame => ERROR_INVALID_FRAME,
            Self::UnsupportedMessage => ERROR_UNSUPPORTED_MESSAGE,
            Self::PayloadTooLarge => ERROR_PAYLOAD_TOO_LARGE,
            Self::InvalidProgram => ERROR_INVALID_PROGRAM,
            Self::InvalidBytecode => ERROR_INVALID_BYTECODE,
            Self::Runtime(error) => match error.kind {
                RuntimeErrorKind::InvalidBytecode | RuntimeErrorKind::ValidationFailed => {
                    ERROR_INVALID_BYTECODE
                }
                RuntimeErrorKind::StackOverflow
                | RuntimeErrorKind::StackUnderflow
                | RuntimeErrorKind::TypeMismatch
                | RuntimeErrorKind::UnsupportedCapability
                | RuntimeErrorKind::HandleNotFound
                | RuntimeErrorKind::ResourceBusy
                | RuntimeErrorKind::InvalidPin
                | RuntimeErrorKind::UnsupportedMode => ERROR_INVALID_PROGRAM,
                RuntimeErrorKind::BoardFault => ERROR_BOARD_FAULT,
            },
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Protocol(_) | Self::InvalidFrame => "invalid frame",
            Self::UnsupportedMessage => "unsupported message",
            Self::PayloadTooLarge => "payload too large",
            Self::InvalidProgram => "invalid program",
            Self::InvalidBytecode => "invalid bytecode",
            Self::Runtime(error) => match error.kind {
                RuntimeErrorKind::StackOverflow
                | RuntimeErrorKind::StackUnderflow
                | RuntimeErrorKind::TypeMismatch
                | RuntimeErrorKind::UnsupportedCapability
                | RuntimeErrorKind::HandleNotFound
                | RuntimeErrorKind::ResourceBusy
                | RuntimeErrorKind::InvalidPin
                | RuntimeErrorKind::UnsupportedMode => "runtime rejected program",
                RuntimeErrorKind::InvalidBytecode | RuntimeErrorKind::ValidationFailed => {
                    "invalid bytecode"
                }
                RuntimeErrorKind::BoardFault => "board fault",
            },
        }
    }
}

fn write_response(
    message_type: MessageType,
    request_id: u16,
    payload: &[u8],
    frame_out: &mut [u8],
) -> Result<usize, BoardError> {
    Ok(encode_frame(
        &Frame {
            flags: FLAG_IS_RESPONSE,
            message_type,
            request_id,
            payload,
        },
        frame_out,
    )?)
}

fn write_program_begin_response(
    begin: ProgramBegin,
    request_id: u16,
    payload_out: &mut [u8],
    frame_out: &mut [u8],
) -> Result<usize, BoardError> {
    let payload_len = encode_program_begin(&begin, payload_out)?;
    write_response(
        MessageType::PROGRAM_BEGIN,
        request_id,
        &payload_out[..payload_len],
        frame_out,
    )
}

fn write_program_chunk_response(
    chunk: ProgramChunk<'_>,
    request_id: u16,
    payload_out: &mut [u8],
    frame_out: &mut [u8],
) -> Result<usize, BoardError> {
    let payload_len = encode_program_chunk(&chunk, payload_out)?;
    write_response(
        MessageType::PROGRAM_CHUNK,
        request_id,
        &payload_out[..payload_len],
        frame_out,
    )
}

fn write_program_end_response(
    end: ProgramEnd,
    request_id: u16,
    payload_out: &mut [u8],
    frame_out: &mut [u8],
) -> Result<usize, BoardError> {
    let payload_len = encode_program_end(&end, payload_out)?;
    write_response(
        MessageType::PROGRAM_END,
        request_id,
        &payload_out[..payload_len],
        frame_out,
    )
}

fn capability_descriptors() -> [CapabilityDescriptor<'static>; 6] {
    [
        CapabilityDescriptor {
            id: CAP_GPIO_OPEN,
            version: 1,
            flags: CAP_FLAG_BYTECODE_CALLABLE,
            name: "gpio.open",
        },
        CapabilityDescriptor {
            id: CAP_GPIO_WRITE,
            version: 1,
            flags: CAP_FLAG_BYTECODE_CALLABLE,
            name: "gpio.write",
        },
        CapabilityDescriptor {
            id: CAP_GPIO_READ,
            version: 1,
            flags: CAP_FLAG_BYTECODE_CALLABLE,
            name: "gpio.read",
        },
        CapabilityDescriptor {
            id: CAP_GPIO_CLOSE,
            version: 1,
            flags: CAP_FLAG_BYTECODE_CALLABLE,
            name: "gpio.close",
        },
        CapabilityDescriptor {
            id: CAP_TIME_SLEEP_MS,
            version: 1,
            flags: CAP_FLAG_BYTECODE_CALLABLE,
            name: "time.sleep_ms",
        },
        CapabilityDescriptor {
            id: CAP_PROGRAM_RAM_EXEC,
            version: 1,
            flags: CAP_FLAG_PROTOCOL_FEATURE,
            name: "program.ram_exec",
        },
    ]
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_host::{write_blink_module, BlinkProgram, HostSession, DEFAULT_PROGRAM_ID};
    use board_vm_protocol::{
        decode_caps_report_header, decode_error_payload, decode_frame, decode_hello_ack,
        decode_run_report_header,
    };

    type Board = LoopbackBoard<256, 8, 8>;

    fn handle(board: &mut Board, request: &[u8], payload: &mut [u8], response: &mut [u8]) -> usize {
        board
            .handle_raw_frame(request, payload, response)
            .expect("loopback response")
    }

    #[test]
    fn hello_and_caps_round_trip() {
        let mut board = Board::new();
        let mut session = HostSession::new();
        let mut host_payload = [0u8; 96];
        let mut request = [0u8; 128];
        let mut board_payload = [0u8; 256];
        let mut response = [0u8; 320];

        let hello = session
            .hello_frame("test-host", 0xCAFE_BABE, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut board,
            &request[..hello.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.flags, FLAG_IS_RESPONSE);
        assert_eq!(frame.message_type, MessageType::HELLO_ACK);
        assert_eq!(frame.request_id, hello.request_id);
        let hello_ack = decode_hello_ack(frame.payload).unwrap();
        assert_eq!(hello_ack.selected_version, 1);
        assert_eq!(hello_ack.board_name, LOOPBACK_BOARD_ID);
        assert_eq!(hello_ack.runtime_name, LOOPBACK_RUNTIME_ID);
        assert_eq!(hello_ack.host_nonce, 0xCAFE_BABE);
        assert_eq!(hello_ack.board_nonce, LOOPBACK_BOARD_NONCE);

        let caps = session.caps_query_frame(&mut request).unwrap();
        let response_len = handle(
            &mut board,
            &request[..caps.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::CAPS_REPORT);
        assert_eq!(frame.request_id, caps.request_id);
        let (header, mut decoder) = decode_caps_report_header(frame.payload).unwrap();
        assert_eq!(header.board_id, LOOPBACK_BOARD_ID);
        assert_eq!(header.runtime_id, LOOPBACK_RUNTIME_ID);
        assert_eq!(header.max_program_bytes, 256);
        assert_eq!(header.max_stack_values, 8);
        assert_eq!(header.max_handles, 8);
        assert!(!header.supports_store_program);
        assert_eq!(header.capability_count, 6);
        let first = decoder.read_capability_descriptor().unwrap();
        assert_eq!(first.id, CAP_GPIO_OPEN);
        assert_eq!(first.name, "gpio.open");
        for _ in 1..header.capability_count {
            decoder.read_capability_descriptor().unwrap();
        }
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn uploads_runs_and_stops_blink_program() {
        let mut board = Board::new();
        let mut session = HostSession::new();
        let mut module = [0u8; board_vm_host::BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let module = &module[..module_len];
        let mut host_payload = [0u8; 128];
        let mut request = [0u8; 192];
        let mut board_payload = [0u8; 256];
        let mut response = [0u8; 320];

        let begin = session
            .program_begin_frame(DEFAULT_PROGRAM_ID, module, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut board,
            &request[..begin.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::PROGRAM_BEGIN);
        assert_eq!(frame.request_id, begin.request_id);

        let chunk = session
            .program_chunk_frame(
                DEFAULT_PROGRAM_ID,
                0,
                module,
                &mut host_payload,
                &mut request,
            )
            .unwrap();
        let response_len = handle(
            &mut board,
            &request[..chunk.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::PROGRAM_CHUNK);
        assert_eq!(frame.request_id, chunk.request_id);

        let end = session
            .program_end_frame(DEFAULT_PROGRAM_ID, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut board,
            &request[..end.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::PROGRAM_END);
        assert_eq!(frame.request_id, end.request_id);

        let run = session
            .run_background_frame(DEFAULT_PROGRAM_ID, 100, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut board,
            &request[..run.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::RUN_REPORT);
        assert_eq!(frame.request_id, run.request_id);
        let (report, decoder) = decode_run_report_header(frame.payload).unwrap();
        assert_eq!(decoder.finish(), Ok(()));
        assert_eq!(report.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(report.status, ProtocolRunStatus::Running);
        assert!(report.instructions_executed > 0);
        assert!(report.elapsed_ms >= 500);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            &board.fake_hal().events()[..5],
            &[
                FakeEvent::GpioOpen {
                    pin: 13,
                    mode: GpioMode::Output,
                    token: 1
                },
                FakeEvent::GpioWrite {
                    token: 1,
                    level: Level::High
                },
                FakeEvent::SleepMs(250),
                FakeEvent::GpioWrite {
                    token: 1,
                    level: Level::Low
                },
                FakeEvent::SleepMs(250),
            ]
        );

        let stop = session.stop_frame(&mut request).unwrap();
        let response_len = handle(
            &mut board,
            &request[..stop.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::RUN_REPORT);
        let (report, decoder) = decode_run_report_header(frame.payload).unwrap();
        assert_eq!(decoder.finish(), Ok(()));
        assert_eq!(report.status, ProtocolRunStatus::Stopped);
        assert_eq!(report.open_handles, 0);
    }

    #[test]
    fn reports_error_for_bad_upload_crc() {
        let mut board = Board::new();
        let mut session = HostSession::new();
        let mut module = [0u8; board_vm_host::BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let mut corrupt_module = module;
        corrupt_module[10] ^= 0x01;
        let module = &module[..module_len];
        let corrupt_module = &corrupt_module[..module_len];
        let mut host_payload = [0u8; 128];
        let mut request = [0u8; 192];
        let mut board_payload = [0u8; 256];
        let mut response = [0u8; 320];

        let begin = session
            .program_begin_frame(DEFAULT_PROGRAM_ID, module, &mut host_payload, &mut request)
            .unwrap();
        handle(
            &mut board,
            &request[..begin.len],
            &mut board_payload,
            &mut response,
        );
        let chunk = session
            .program_chunk_frame(
                DEFAULT_PROGRAM_ID,
                0,
                corrupt_module,
                &mut host_payload,
                &mut request,
            )
            .unwrap();
        handle(
            &mut board,
            &request[..chunk.len],
            &mut board_payload,
            &mut response,
        );
        let end = session
            .program_end_frame(DEFAULT_PROGRAM_ID, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut board,
            &request[..end.len],
            &mut board_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.flags, FLAG_IS_RESPONSE | FLAG_IS_ERROR_RESPONSE);
        assert_eq!(frame.message_type, MessageType::ERROR);
        let error = decode_error_payload(frame.payload).unwrap();
        assert_eq!(error.code, ERROR_INVALID_PROGRAM);
        assert_eq!(error.request_id, end.request_id);
    }
}
