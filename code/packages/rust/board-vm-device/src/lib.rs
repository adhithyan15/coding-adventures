#![no_std]

use board_vm_ir::{
    parse_module, validate, CAP_GPIO_CLOSE, CAP_GPIO_OPEN, CAP_GPIO_READ, CAP_GPIO_WRITE,
    CAP_TIME_SLEEP_MS,
};
use board_vm_protocol::{
    decode_frame, decode_hello, decode_program_begin, decode_program_chunk, decode_program_end,
    decode_run_request, decode_wire_frame, encode_caps_report, encode_error_payload, encode_frame,
    encode_hello_ack, encode_program_begin, encode_program_chunk, encode_program_end,
    encode_run_report_header, encode_wire_frame, CapabilityDescriptor, CapsReportHeader,
    ErrorPayload, Frame, HelloAck, MessageType, ProgramBegin, ProgramChunk, ProgramEnd,
    ProtocolError, RunReportHeader, RunStatus as ProtocolRunStatus, CAP_FLAG_BYTECODE_CALLABLE,
    CAP_FLAG_PROTOCOL_FEATURE, CAP_PROGRAM_RAM_EXEC, FLAG_IS_ERROR_RESPONSE, FLAG_IS_RESPONSE,
    NO_BYTECODE_OFFSET, NO_PROGRAM_ID, RUN_FLAG_BACKGROUND_RUN, RUN_FLAG_RESET_VM_BEFORE_RUN,
};
use board_vm_runtime::{BoardHal, RunStatus as RuntimeRunStatus, Runtime, RuntimeError};

pub const DEFAULT_DEVICE_RUNTIME_ID: &str = "board-vm-device";
pub const DEFAULT_MAX_FRAME_PAYLOAD: u16 = 256;

pub const ERROR_INVALID_FRAME: u16 = 0x0001;
pub const ERROR_UNSUPPORTED_MESSAGE: u16 = 0x0003;
pub const ERROR_PAYLOAD_TOO_LARGE: u16 = 0x0004;
pub const ERROR_BAD_CRC: u16 = 0x0005;
pub const ERROR_INVALID_PROGRAM: u16 = 0x0200;
pub const ERROR_INVALID_BYTECODE: u16 = 0x0201;
pub const ERROR_BOARD_FAULT: u16 = 0x0400;

pub const BLINK_MVP_CAPABILITIES: [CapabilityDescriptor<'static>; 6] = [
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
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceError {
    Protocol(ProtocolError),
    OutputTooSmall,
}

impl From<ProtocolError> for DeviceError {
    fn from(value: ProtocolError) -> Self {
        match value {
            ProtocolError::OutputTooSmall => Self::OutputTooSmall,
            other => Self::Protocol(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDescriptor<'a> {
    pub board_id: &'a str,
    pub runtime_id: &'a str,
    pub board_nonce: u32,
    pub max_frame_payload: u16,
    pub supports_store_program: bool,
    pub capabilities: &'a [CapabilityDescriptor<'a>],
}

impl<'a> DeviceDescriptor<'a> {
    pub const fn blink_mvp(board_id: &'a str, board_nonce: u32) -> Self {
        Self {
            board_id,
            runtime_id: DEFAULT_DEVICE_RUNTIME_ID,
            board_nonce,
            max_frame_payload: DEFAULT_MAX_FRAME_PAYLOAD,
            supports_store_program: false,
            capabilities: &BLINK_MVP_CAPABILITIES,
        }
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

pub struct BoardVmDevice<
    'a,
    H,
    const MAX_PROGRAM_BYTES: usize,
    const MAX_STACK: usize,
    const MAX_HANDLES: usize,
> where
    H: BoardHal,
{
    descriptor: DeviceDescriptor<'a>,
    runtime: Runtime<H, MAX_STACK, MAX_HANDLES>,
    program: [u8; MAX_PROGRAM_BYTES],
    program_len: usize,
    program_id: u16,
    upload: UploadState,
}

impl<'a, H, const MAX_PROGRAM_BYTES: usize, const MAX_STACK: usize, const MAX_HANDLES: usize>
    BoardVmDevice<'a, H, MAX_PROGRAM_BYTES, MAX_STACK, MAX_HANDLES>
where
    H: BoardHal,
{
    pub fn new(descriptor: DeviceDescriptor<'a>, hal: H) -> Self {
        Self {
            descriptor,
            runtime: Runtime::new(hal),
            program: [0; MAX_PROGRAM_BYTES],
            program_len: 0,
            program_id: 0,
            upload: UploadState::default(),
        }
    }

    pub fn descriptor(&self) -> DeviceDescriptor<'a> {
        self.descriptor
    }

    pub fn runtime(&self) -> &Runtime<H, MAX_STACK, MAX_HANDLES> {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime<H, MAX_STACK, MAX_HANDLES> {
        &mut self.runtime
    }

    pub fn hal(&self) -> &H {
        self.runtime.hal()
    }

    pub fn hal_mut(&mut self) -> &mut H {
        self.runtime.hal_mut()
    }

    pub fn loaded_program_id(&self) -> Option<u16> {
        if self.program_len == 0 {
            None
        } else {
            Some(self.program_id)
        }
    }

    pub fn handle_raw_frame(
        &mut self,
        raw_frame: &[u8],
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, DeviceError> {
        let request = match decode_frame(raw_frame) {
            Ok(request) => request,
            Err(error) => {
                return self.write_error_response(
                    0,
                    NO_PROGRAM_ID,
                    BoardFault::Protocol(error),
                    payload_out,
                    frame_out,
                );
            }
        };
        let result = self.dispatch(request, payload_out, frame_out);
        match result {
            Ok(len) => Ok(len),
            Err(error) => self.write_error_response(
                request.request_id,
                self.program_id,
                error,
                payload_out,
                frame_out,
            ),
        }
    }

    fn dispatch(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardFault> {
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
            _ => Err(BoardFault::UnsupportedMessage),
        }
    }

    fn handle_hello(
        &mut self,
        request: Frame<'_>,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, BoardFault> {
        let hello = decode_hello(request.payload)?;
        if hello.min_version > 1 || hello.max_version < 1 {
            return Err(BoardFault::InvalidFrame);
        }
        let payload_len = encode_hello_ack(
            &HelloAck {
                selected_version: 1,
                board_name: self.descriptor.board_id,
                runtime_name: self.descriptor.runtime_id,
                host_nonce: hello.host_nonce,
                board_nonce: self.descriptor.board_nonce,
                max_frame_payload: self.descriptor.max_frame_payload,
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
    ) -> Result<usize, BoardFault> {
        if !request.payload.is_empty() {
            return Err(BoardFault::InvalidFrame);
        }
        let payload_len = encode_caps_report(
            &CapsReportHeader {
                board_id: self.descriptor.board_id,
                runtime_id: self.descriptor.runtime_id,
                max_program_bytes: MAX_PROGRAM_BYTES as u32,
                max_stack_values: MAX_STACK as u8,
                max_handles: MAX_HANDLES as u8,
                supports_store_program: self.descriptor.supports_store_program,
                capability_count: self.descriptor.capabilities.len() as u32,
            },
            self.descriptor.capabilities,
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
    ) -> Result<usize, BoardFault> {
        let begin = decode_program_begin(request.payload)?;
        if begin.total_len as usize > MAX_PROGRAM_BYTES {
            return Err(BoardFault::PayloadTooLarge);
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
    ) -> Result<usize, BoardFault> {
        let chunk = decode_program_chunk(request.payload)?;
        if !self.upload.active || chunk.program_id != self.upload.program_id {
            return Err(BoardFault::InvalidProgram);
        }
        let offset = chunk.offset as usize;
        let end = offset
            .checked_add(chunk.bytes.len())
            .ok_or(BoardFault::PayloadTooLarge)?;
        if end > self.upload.expected_len || end > MAX_PROGRAM_BYTES {
            return Err(BoardFault::PayloadTooLarge);
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
    ) -> Result<usize, BoardFault> {
        let end = decode_program_end(request.payload)?;
        if !self.upload.active || end.program_id != self.upload.program_id {
            return Err(BoardFault::InvalidProgram);
        }
        if self.upload.received_len != self.upload.expected_len {
            return Err(BoardFault::InvalidProgram);
        }
        let program = &self.program[..self.upload.expected_len];
        if crc32_ieee(program) != self.upload.expected_crc32 {
            return Err(BoardFault::InvalidProgram);
        }
        let module = parse_module(program).map_err(|_| BoardFault::InvalidBytecode)?;
        validate(&module, self.runtime.hal().capabilities(), MAX_STACK as u8)
            .map_err(|_| BoardFault::InvalidBytecode)?;
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
    ) -> Result<usize, BoardFault> {
        let run = decode_run_request(request.payload)?;
        if run.program_id != self.program_id || self.program_len == 0 {
            return Err(BoardFault::InvalidProgram);
        }
        if run.flags & RUN_FLAG_RESET_VM_BEFORE_RUN != 0 {
            self.runtime.reset_vm();
        }

        let module = parse_module(&self.program[..self.program_len])
            .map_err(|_| BoardFault::InvalidBytecode)?;
        let report = self
            .runtime
            .run_module(&module, run.instruction_budget)
            .map_err(BoardFault::Runtime)?;
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
    ) -> Result<usize, BoardFault> {
        if !request.payload.is_empty() {
            return Err(BoardFault::InvalidFrame);
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
        program_id: u16,
        error: BoardFault,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<usize, DeviceError> {
        let payload_len = encode_error_payload(
            &ErrorPayload {
                code: error.code(),
                request_id,
                program_id,
                bytecode_offset: error.bytecode_offset(),
                message: error.message(),
            },
            payload_out,
        )?;
        Ok(encode_frame(
            &Frame {
                flags: FLAG_IS_RESPONSE | FLAG_IS_ERROR_RESPONSE,
                message_type: MessageType::ERROR,
                request_id,
                payload: &payload_out[..payload_len],
            },
            frame_out,
        )?)
    }
}

pub trait DeviceByteStream {
    type Error;

    fn read_byte(&mut self) -> Result<u8, Self::Error>;

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStreamError<E> {
    Stream(E),
    IncomingWireFrameTooLarge,
    RawFrameTooLarge,
    ResponseWireFrameTooLarge,
    Protocol(ProtocolError),
    Device(DeviceError),
}

pub struct DeviceStreamEndpoint<
    S,
    const WIRE_BYTES: usize = 1024,
    const RAW_BYTES: usize = 512,
    const PAYLOAD_BYTES: usize = 256,
> {
    stream: S,
    wire: [u8; WIRE_BYTES],
    raw_request: [u8; RAW_BYTES],
    raw_response: [u8; RAW_BYTES],
    payload: [u8; PAYLOAD_BYTES],
}

impl<S, const WIRE_BYTES: usize, const RAW_BYTES: usize, const PAYLOAD_BYTES: usize>
    DeviceStreamEndpoint<S, WIRE_BYTES, RAW_BYTES, PAYLOAD_BYTES>
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            wire: [0; WIRE_BYTES],
            raw_request: [0; RAW_BYTES],
            raw_response: [0; RAW_BYTES],
            payload: [0; PAYLOAD_BYTES],
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    pub fn inner(&self) -> &S {
        &self.stream
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

impl<S, const WIRE_BYTES: usize, const RAW_BYTES: usize, const PAYLOAD_BYTES: usize>
    DeviceStreamEndpoint<S, WIRE_BYTES, RAW_BYTES, PAYLOAD_BYTES>
where
    S: DeviceByteStream,
{
    pub fn serve_one<
        'a,
        H,
        const MAX_PROGRAM_BYTES: usize,
        const MAX_STACK: usize,
        const MAX_HANDLES: usize,
    >(
        &mut self,
        device: &mut BoardVmDevice<'a, H, MAX_PROGRAM_BYTES, MAX_STACK, MAX_HANDLES>,
    ) -> Result<usize, DeviceStreamError<S::Error>>
    where
        H: BoardHal,
    {
        let wire_len = self.read_wire_frame()?;
        let request_len = decode_wire_frame(&self.wire[..wire_len], &mut self.raw_request)
            .map_err(map_decode_wire_error)?;
        let response_len = device
            .handle_raw_frame(
                &self.raw_request[..request_len],
                &mut self.payload,
                &mut self.raw_response,
            )
            .map_err(map_device_stream_error)?;
        let write_len = encode_wire_frame(&self.raw_response[..response_len], &mut self.wire)
            .map_err(map_encode_wire_error)?;
        self.stream
            .write_all(&self.wire[..write_len])
            .map_err(DeviceStreamError::Stream)?;
        self.stream.flush().map_err(DeviceStreamError::Stream)?;
        Ok(write_len)
    }

    fn read_wire_frame(&mut self) -> Result<usize, DeviceStreamError<S::Error>> {
        let mut len = 0;
        loop {
            if len >= self.wire.len() {
                return Err(DeviceStreamError::IncomingWireFrameTooLarge);
            }

            let byte = self.stream.read_byte().map_err(DeviceStreamError::Stream)?;
            self.wire[len] = byte;
            len += 1;

            if byte == 0 {
                return Ok(len);
            }
        }
    }
}

fn map_decode_wire_error<E>(error: ProtocolError) -> DeviceStreamError<E> {
    match error {
        ProtocolError::OutputTooSmall | ProtocolError::PayloadTooLarge => {
            DeviceStreamError::RawFrameTooLarge
        }
        other => DeviceStreamError::Protocol(other),
    }
}

fn map_encode_wire_error<E>(error: ProtocolError) -> DeviceStreamError<E> {
    match error {
        ProtocolError::OutputTooSmall | ProtocolError::PayloadTooLarge => {
            DeviceStreamError::ResponseWireFrameTooLarge
        }
        other => DeviceStreamError::Protocol(other),
    }
}

fn map_device_stream_error<E>(error: DeviceError) -> DeviceStreamError<E> {
    match error {
        DeviceError::OutputTooSmall => DeviceStreamError::RawFrameTooLarge,
        other => DeviceStreamError::Device(other),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoardFault {
    Protocol(ProtocolError),
    InvalidFrame,
    UnsupportedMessage,
    PayloadTooLarge,
    InvalidProgram,
    InvalidBytecode,
    Runtime(RuntimeError),
}

impl From<ProtocolError> for BoardFault {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl BoardFault {
    fn code(self) -> u16 {
        match self {
            Self::Protocol(ProtocolError::BadCrc) => ERROR_BAD_CRC,
            Self::Protocol(_) | Self::InvalidFrame => ERROR_INVALID_FRAME,
            Self::UnsupportedMessage => ERROR_UNSUPPORTED_MESSAGE,
            Self::PayloadTooLarge => ERROR_PAYLOAD_TOO_LARGE,
            Self::InvalidProgram => ERROR_INVALID_PROGRAM,
            Self::InvalidBytecode => ERROR_INVALID_BYTECODE,
            Self::Runtime(error) => match error.kind {
                board_vm_runtime::RuntimeErrorKind::InvalidBytecode
                | board_vm_runtime::RuntimeErrorKind::ValidationFailed => ERROR_INVALID_BYTECODE,
                board_vm_runtime::RuntimeErrorKind::BoardFault => ERROR_BOARD_FAULT,
                _ => ERROR_INVALID_PROGRAM,
            },
        }
    }

    fn bytecode_offset(self) -> u32 {
        match self {
            Self::Runtime(error) => error.ip as u32,
            _ => NO_BYTECODE_OFFSET,
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
                board_vm_runtime::RuntimeErrorKind::InvalidBytecode
                | board_vm_runtime::RuntimeErrorKind::ValidationFailed => "invalid bytecode",
                board_vm_runtime::RuntimeErrorKind::BoardFault => "board fault",
                _ => "runtime rejected program",
            },
        }
    }
}

fn write_response(
    message_type: MessageType,
    request_id: u16,
    payload: &[u8],
    frame_out: &mut [u8],
) -> Result<usize, BoardFault> {
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
) -> Result<usize, BoardFault> {
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
) -> Result<usize, BoardFault> {
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
) -> Result<usize, BoardFault> {
    let payload_len = encode_program_end(&end, payload_out)?;
    write_response(
        MessageType::PROGRAM_END,
        request_id,
        &payload_out[..payload_len],
        frame_out,
    )
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
    use board_vm_ir::CapabilitySet;
    use board_vm_protocol::{
        decode_caps_report_header, decode_error_payload, decode_frame, decode_hello_ack,
        decode_run_report_header, decode_wire_frame, encode_wire_frame, RunStatus,
    };
    use board_vm_runtime::{GpioMode, HalError, Level};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        GpioOpen { pin: u16, mode: GpioMode },
        GpioWrite { token: u32, level: Level },
        SleepMs(u16),
    }

    struct FakeHal {
        now_ms: u32,
        next_token: u32,
        events: [Option<Event>; 128],
        event_len: usize,
        capabilities: CapabilitySet,
    }

    impl FakeHal {
        fn new(capabilities: CapabilitySet) -> Self {
            Self {
                now_ms: 0,
                next_token: 1,
                events: [None; 128],
                event_len: 0,
                capabilities,
            }
        }

        fn push_event(&mut self, event: Event) {
            if self.event_len < self.events.len() {
                self.events[self.event_len] = Some(event);
                self.event_len += 1;
            }
        }

        fn events(&self) -> &[Option<Event>] {
            &self.events[..self.event_len]
        }
    }

    impl BoardHal for FakeHal {
        fn capabilities(&self) -> CapabilitySet {
            self.capabilities
        }

        fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
            let token = self.next_token;
            self.next_token += 1;
            self.push_event(Event::GpioOpen { pin, mode });
            Ok(token)
        }

        fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
            self.push_event(Event::GpioWrite { token, level });
            Ok(())
        }

        fn gpio_read(&mut self, _token: u32) -> Result<Level, HalError> {
            Ok(Level::Low)
        }

        fn gpio_close(&mut self, _token: u32) -> Result<(), HalError> {
            Ok(())
        }

        fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError> {
            self.now_ms += duration_ms as u32;
            self.push_event(Event::SleepMs(duration_ms));
            Ok(())
        }

        fn now_ms(&self) -> u32 {
            self.now_ms
        }
    }

    type Device = BoardVmDevice<'static, FakeHal, 256, 8, 8>;

    fn new_device() -> Device {
        BoardVmDevice::new(
            DeviceDescriptor::blink_mvp("test-board", 0xB04D_1001),
            FakeHal::new(CapabilitySet::blink_mvp()),
        )
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestStreamError {
        EndOfInput,
        WriteBufferTooSmall,
    }

    struct ScriptedByteStream<const READ_BYTES: usize, const WRITE_BYTES: usize> {
        read: [u8; READ_BYTES],
        read_len: usize,
        read_offset: usize,
        written: [u8; WRITE_BYTES],
        written_len: usize,
        flush_count: usize,
    }

    impl<const READ_BYTES: usize, const WRITE_BYTES: usize>
        ScriptedByteStream<READ_BYTES, WRITE_BYTES>
    {
        fn with_read(bytes: &[u8]) -> Self {
            assert!(bytes.len() <= READ_BYTES);
            let mut read = [0; READ_BYTES];
            read[..bytes.len()].copy_from_slice(bytes);
            Self {
                read,
                read_len: bytes.len(),
                read_offset: 0,
                written: [0; WRITE_BYTES],
                written_len: 0,
                flush_count: 0,
            }
        }

        fn written(&self) -> &[u8] {
            &self.written[..self.written_len]
        }
    }

    impl<const READ_BYTES: usize, const WRITE_BYTES: usize> DeviceByteStream
        for ScriptedByteStream<READ_BYTES, WRITE_BYTES>
    {
        type Error = TestStreamError;

        fn read_byte(&mut self) -> Result<u8, Self::Error> {
            if self.read_offset >= self.read_len {
                return Err(TestStreamError::EndOfInput);
            }
            let byte = self.read[self.read_offset];
            self.read_offset += 1;
            Ok(byte)
        }

        fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            let end = self
                .written_len
                .checked_add(bytes.len())
                .ok_or(TestStreamError::WriteBufferTooSmall)?;
            if end > WRITE_BYTES {
                return Err(TestStreamError::WriteBufferTooSmall);
            }
            self.written[self.written_len..end].copy_from_slice(bytes);
            self.written_len = end;
            Ok(())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.flush_count += 1;
            Ok(())
        }
    }

    fn handle(
        device: &mut Device,
        request: &[u8],
        payload: &mut [u8],
        response: &mut [u8],
    ) -> usize {
        device
            .handle_raw_frame(request, payload, response)
            .expect("device response")
    }

    #[test]
    fn stream_endpoint_serves_wire_framed_handshake() {
        let mut device = new_device();
        let mut session = HostSession::new();
        let mut host_payload = [0u8; 96];
        let mut request = [0u8; 128];
        let mut request_wire = [0u8; 192];
        let mut raw_response = [0u8; 128];

        let hello = session
            .hello_frame("stream-host", 0xCAFE_BABE, &mut host_payload, &mut request)
            .unwrap();
        let request_wire_len = encode_wire_frame(&request[..hello.len], &mut request_wire).unwrap();
        let stream = ScriptedByteStream::<192, 192>::with_read(&request_wire[..request_wire_len]);
        let mut endpoint = DeviceStreamEndpoint::<_, 192, 128, 128>::new(stream);

        let written_len = endpoint.serve_one(&mut device).unwrap();
        let stream = endpoint.into_inner();
        assert_eq!(written_len, stream.written().len());
        assert_eq!(stream.flush_count, 1);

        let raw_len = decode_wire_frame(stream.written(), &mut raw_response).unwrap();
        let frame = decode_frame(&raw_response[..raw_len]).unwrap();
        assert_eq!(frame.flags, FLAG_IS_RESPONSE);
        assert_eq!(frame.message_type, MessageType::HELLO_ACK);
        assert_eq!(frame.request_id, hello.request_id);
        let hello_ack = decode_hello_ack(frame.payload).unwrap();
        assert_eq!(hello_ack.board_name, "test-board");
        assert_eq!(hello_ack.runtime_name, DEFAULT_DEVICE_RUNTIME_ID);
        assert_eq!(hello_ack.host_nonce, 0xCAFE_BABE);
        assert_eq!(hello_ack.board_nonce, 0xB04D_1001);
    }

    #[test]
    fn stream_endpoint_rejects_unbounded_wire_frame() {
        let mut device = new_device();
        let stream = ScriptedByteStream::<5, 8>::with_read(&[1, 2, 3, 4, 5]);
        let mut endpoint = DeviceStreamEndpoint::<_, 4, 8, 8>::new(stream);

        assert_eq!(
            endpoint.serve_one(&mut device),
            Err(DeviceStreamError::IncomingWireFrameTooLarge)
        );
    }

    #[test]
    fn handshakes_and_reports_descriptor() {
        let mut device = new_device();
        let mut session = HostSession::new();
        let mut host_payload = [0u8; 96];
        let mut request = [0u8; 128];
        let mut device_payload = [0u8; 256];
        let mut response = [0u8; 320];

        let hello = session
            .hello_frame("test-host", 0xCAFE_BABE, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut device,
            &request[..hello.len],
            &mut device_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.flags, FLAG_IS_RESPONSE);
        assert_eq!(frame.message_type, MessageType::HELLO_ACK);
        assert_eq!(frame.request_id, hello.request_id);
        let hello_ack = decode_hello_ack(frame.payload).unwrap();
        assert_eq!(hello_ack.board_name, "test-board");
        assert_eq!(hello_ack.runtime_name, DEFAULT_DEVICE_RUNTIME_ID);
        assert_eq!(hello_ack.host_nonce, 0xCAFE_BABE);
        assert_eq!(hello_ack.board_nonce, 0xB04D_1001);

        let caps = session.caps_query_frame(&mut request).unwrap();
        let response_len = handle(
            &mut device,
            &request[..caps.len],
            &mut device_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::CAPS_REPORT);
        assert_eq!(frame.request_id, caps.request_id);
        let (header, mut decoder) = decode_caps_report_header(frame.payload).unwrap();
        assert_eq!(header.board_id, "test-board");
        assert_eq!(header.runtime_id, DEFAULT_DEVICE_RUNTIME_ID);
        assert_eq!(header.max_program_bytes, 256);
        assert_eq!(header.max_stack_values, 8);
        assert_eq!(header.max_handles, 8);
        assert!(!header.supports_store_program);
        assert_eq!(header.capability_count, BLINK_MVP_CAPABILITIES.len() as u32);
        let first = decoder.read_capability_descriptor().unwrap();
        assert_eq!(first.id, CAP_GPIO_OPEN);
        assert_eq!(first.name, "gpio.open");
    }

    #[test]
    fn uploads_runs_and_stops_program_against_generic_hal() {
        let mut device = new_device();
        let mut session = HostSession::new();
        let mut module = [0u8; board_vm_host::BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let module = &module[..module_len];
        let mut host_payload = [0u8; 128];
        let mut request = [0u8; 192];
        let mut device_payload = [0u8; 256];
        let mut response = [0u8; 320];

        let begin = session
            .program_begin_frame(DEFAULT_PROGRAM_ID, module, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut device,
            &request[..begin.len],
            &mut device_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::PROGRAM_BEGIN);

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
            &mut device,
            &request[..chunk.len],
            &mut device_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::PROGRAM_CHUNK);

        let end = session
            .program_end_frame(DEFAULT_PROGRAM_ID, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut device,
            &request[..end.len],
            &mut device_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::PROGRAM_END);
        assert_eq!(device.loaded_program_id(), Some(DEFAULT_PROGRAM_ID));

        let run = session
            .run_background_frame(DEFAULT_PROGRAM_ID, 100, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut device,
            &request[..run.len],
            &mut device_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::RUN_REPORT);
        let (report, decoder) = decode_run_report_header(frame.payload).unwrap();
        decoder.finish().unwrap();
        assert_eq!(report.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(report.status, RunStatus::Running);
        assert!(report.instructions_executed > 0);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            &device.hal().events()[..5],
            &[
                Some(Event::GpioOpen {
                    pin: 13,
                    mode: GpioMode::Output
                }),
                Some(Event::GpioWrite {
                    token: 1,
                    level: Level::High
                }),
                Some(Event::SleepMs(250)),
                Some(Event::GpioWrite {
                    token: 1,
                    level: Level::Low
                }),
                Some(Event::SleepMs(250)),
            ]
        );

        let stop = session.stop_frame(&mut request).unwrap();
        let response_len = handle(
            &mut device,
            &request[..stop.len],
            &mut device_payload,
            &mut response,
        );
        let frame = decode_frame(&response[..response_len]).unwrap();
        let (report, decoder) = decode_run_report_header(frame.payload).unwrap();
        decoder.finish().unwrap();
        assert_eq!(report.status, RunStatus::Stopped);
        assert_eq!(report.open_handles, 0);
    }

    #[test]
    fn rejects_programs_not_supported_by_hal_capabilities() {
        let mut device = BoardVmDevice::new(
            DeviceDescriptor::blink_mvp("limited-board", 1),
            FakeHal::new(CapabilitySet::empty()),
        );
        let mut session = HostSession::new();
        let mut module = [0u8; board_vm_host::BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let module = &module[..module_len];
        let mut host_payload = [0u8; 128];
        let mut request = [0u8; 192];
        let mut device_payload = [0u8; 256];
        let mut response = [0u8; 320];

        let begin = session
            .program_begin_frame(DEFAULT_PROGRAM_ID, module, &mut host_payload, &mut request)
            .unwrap();
        handle(
            &mut device,
            &request[..begin.len],
            &mut device_payload,
            &mut response,
        );
        let chunk = session
            .program_chunk_frame(
                DEFAULT_PROGRAM_ID,
                0,
                module,
                &mut host_payload,
                &mut request,
            )
            .unwrap();
        handle(
            &mut device,
            &request[..chunk.len],
            &mut device_payload,
            &mut response,
        );
        let end = session
            .program_end_frame(DEFAULT_PROGRAM_ID, &mut host_payload, &mut request)
            .unwrap();
        let response_len = handle(
            &mut device,
            &request[..end.len],
            &mut device_payload,
            &mut response,
        );

        let frame = decode_frame(&response[..response_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::ERROR);
        let error = decode_error_payload(frame.payload).unwrap();
        assert_eq!(error.code, ERROR_INVALID_BYTECODE);
        assert_eq!(error.message, "invalid bytecode");
    }
}
