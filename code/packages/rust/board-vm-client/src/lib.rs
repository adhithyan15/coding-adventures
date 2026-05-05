use board_vm_host::{write_blink_module, BlinkProgram, HostError, HostSession, DEFAULT_HOST_NAME};
use board_vm_protocol::{
    decode_caps_report_header, decode_error_payload, decode_frame, decode_hello_ack,
    decode_program_begin, decode_program_chunk, decode_program_end, decode_run_report_header,
    ErrorPayload, HelloAck, MessageType, ProgramBegin, ProgramChunk, ProgramEnd, ProtocolError,
    RunReportHeader, RunStatus, FLAG_IS_ERROR_RESPONSE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    Io,
    ResponseTooLarge,
}

pub trait RawFrameTransport {
    fn exchange_raw_frame(
        &mut self,
        request: &[u8],
        response_out: &mut [u8],
    ) -> Result<usize, TransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    Host(HostError),
    Protocol(ProtocolError),
    Transport(TransportError),
    Board(BoardError),
    UnexpectedMessage {
        expected: MessageType,
        actual: MessageType,
    },
    RequestIdMismatch {
        expected: u16,
        actual: u16,
    },
    ResponseNotMarked,
}

impl From<HostError> for ClientError {
    fn from(value: HostError) -> Self {
        Self::Host(value)
    }
}

impl From<ProtocolError> for ClientError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<TransportError> for ClientError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardError {
    pub code: u16,
    pub request_id: u16,
    pub program_id: u16,
    pub bytecode_offset: u32,
    pub message: String,
}

impl From<ErrorPayload<'_>> for BoardError {
    fn from(value: ErrorPayload<'_>) -> Self {
        Self {
            code: value.code,
            request_id: value.request_id,
            program_id: value.program_id,
            bytecode_offset: value.bytecode_offset,
            message: value.message.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloAckInfo {
    pub selected_version: u8,
    pub board_name: String,
    pub runtime_name: String,
    pub host_nonce: u32,
    pub board_nonce: u32,
    pub max_frame_payload: u16,
}

impl From<HelloAck<'_>> for HelloAckInfo {
    fn from(value: HelloAck<'_>) -> Self {
        Self {
            selected_version: value.selected_version,
            board_name: value.board_name.to_owned(),
            runtime_name: value.runtime_name.to_owned(),
            host_nonce: value.host_nonce,
            board_nonce: value.board_nonce,
            max_frame_payload: value.max_frame_payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityInfo {
    pub id: u16,
    pub version: u8,
    pub flags: u16,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardDescriptorInfo {
    pub board_id: String,
    pub runtime_id: String,
    pub max_program_bytes: u32,
    pub max_stack_values: u8,
    pub max_handles: u8,
    pub supports_store_program: bool,
    pub capabilities: Vec<CapabilityInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadReport {
    pub program_id: u16,
    pub total_len: u32,
    pub program_crc32: u32,
}

impl From<ProgramBegin> for UploadReport {
    fn from(value: ProgramBegin) -> Self {
        Self {
            program_id: value.program_id,
            total_len: value.total_len,
            program_crc32: value.program_crc32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkReport {
    pub program_id: u16,
    pub offset: u32,
    pub len: usize,
}

impl From<ProgramChunk<'_>> for ChunkReport {
    fn from(value: ProgramChunk<'_>) -> Self {
        Self {
            program_id: value.program_id,
            offset: value.offset,
            len: value.bytes.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramEndReport {
    pub program_id: u16,
}

impl From<ProgramEnd> for ProgramEndReport {
    fn from(value: ProgramEnd) -> Self {
        Self {
            program_id: value.program_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunReportInfo {
    pub program_id: u16,
    pub status: RunStatus,
    pub instructions_executed: u32,
    pub elapsed_ms: u32,
    pub stack_depth: u8,
    pub open_handles: u8,
}

impl From<RunReportHeader> for RunReportInfo {
    fn from(value: RunReportHeader) -> Self {
        Self {
            program_id: value.program_id,
            status: value.status,
            instructions_executed: value.instructions_executed,
            elapsed_ms: value.elapsed_ms,
            stack_depth: value.stack_depth,
            open_handles: value.open_handles,
        }
    }
}

pub struct BoardVmClient<
    T,
    const PAYLOAD_BYTES: usize = 512,
    const REQUEST_BYTES: usize = 768,
    const RESPONSE_BYTES: usize = 768,
> {
    transport: T,
    session: HostSession,
    payload: [u8; PAYLOAD_BYTES],
    request: [u8; REQUEST_BYTES],
    response: [u8; RESPONSE_BYTES],
}

impl<T, const PAYLOAD_BYTES: usize, const REQUEST_BYTES: usize, const RESPONSE_BYTES: usize>
    BoardVmClient<T, PAYLOAD_BYTES, REQUEST_BYTES, RESPONSE_BYTES>
where
    T: RawFrameTransport,
{
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            session: HostSession::new(),
            payload: [0; PAYLOAD_BYTES],
            request: [0; REQUEST_BYTES],
            response: [0; RESPONSE_BYTES],
        }
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn hello(&mut self, host_nonce: u32) -> Result<HelloAckInfo, ClientError> {
        self.hello_with_name(DEFAULT_HOST_NAME, host_nonce)
    }

    pub fn hello_with_name(
        &mut self,
        host_name: &str,
        host_nonce: u32,
    ) -> Result<HelloAckInfo, ClientError> {
        let written = self.session.hello_frame(
            host_name,
            host_nonce,
            &mut self.payload,
            &mut self.request,
        )?;
        let payload =
            self.exchange_checked(written.request_id, written.len, MessageType::HELLO_ACK)?;
        Ok(decode_hello_ack(payload)?.into())
    }

    pub fn query_caps(&mut self) -> Result<BoardDescriptorInfo, ClientError> {
        let written = self.session.caps_query_frame(&mut self.request)?;
        let payload =
            self.exchange_checked(written.request_id, written.len, MessageType::CAPS_REPORT)?;
        let (header, mut decoder) = decode_caps_report_header(payload)?;
        let mut capabilities = Vec::new();
        for _ in 0..header.capability_count {
            let capability = decoder.read_capability_descriptor()?;
            capabilities.push(CapabilityInfo {
                id: capability.id,
                version: capability.version,
                flags: capability.flags,
                name: capability.name.to_owned(),
            });
        }
        decoder.finish()?;
        Ok(BoardDescriptorInfo {
            board_id: header.board_id.to_owned(),
            runtime_id: header.runtime_id.to_owned(),
            max_program_bytes: header.max_program_bytes,
            max_stack_values: header.max_stack_values,
            max_handles: header.max_handles,
            supports_store_program: header.supports_store_program,
            capabilities,
        })
    }

    pub fn upload_program(
        &mut self,
        program_id: u16,
        module: &[u8],
    ) -> Result<UploadReport, ClientError> {
        let written = self.session.program_begin_frame(
            program_id,
            module,
            &mut self.payload,
            &mut self.request,
        )?;
        let payload =
            self.exchange_checked(written.request_id, written.len, MessageType::PROGRAM_BEGIN)?;
        let begin = decode_program_begin(payload)?;

        let written = self.session.program_chunk_frame(
            program_id,
            0,
            module,
            &mut self.payload,
            &mut self.request,
        )?;
        let payload =
            self.exchange_checked(written.request_id, written.len, MessageType::PROGRAM_CHUNK)?;
        let chunk = decode_program_chunk(payload)?;
        if chunk.program_id != program_id || chunk.offset != 0 || chunk.bytes.len() != module.len()
        {
            return Err(ClientError::UnexpectedMessage {
                expected: MessageType::PROGRAM_CHUNK,
                actual: MessageType::PROGRAM_CHUNK,
            });
        }

        let written =
            self.session
                .program_end_frame(program_id, &mut self.payload, &mut self.request)?;
        let payload =
            self.exchange_checked(written.request_id, written.len, MessageType::PROGRAM_END)?;
        let end = decode_program_end(payload)?;
        if end.program_id != program_id {
            return Err(ClientError::UnexpectedMessage {
                expected: MessageType::PROGRAM_END,
                actual: MessageType::PROGRAM_END,
            });
        }

        Ok(begin.into())
    }

    pub fn run_background(
        &mut self,
        program_id: u16,
        instruction_budget: u32,
    ) -> Result<RunReportInfo, ClientError> {
        let written = self.session.run_background_frame(
            program_id,
            instruction_budget,
            &mut self.payload,
            &mut self.request,
        )?;
        self.expect_run_report(written.request_id, written.len)
    }

    pub fn stop(&mut self) -> Result<RunReportInfo, ClientError> {
        let written = self.session.stop_frame(&mut self.request)?;
        self.expect_run_report(written.request_id, written.len)
    }

    pub fn blink_onboard_led(
        &mut self,
        program_id: u16,
        instruction_budget: u32,
    ) -> Result<RunReportInfo, ClientError> {
        let mut module = [0u8; board_vm_host::BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module)?;
        self.upload_program(program_id, &module[..module_len])?;
        self.run_background(program_id, instruction_budget)
    }

    fn expect_run_report(
        &mut self,
        request_id: u16,
        request_len: usize,
    ) -> Result<RunReportInfo, ClientError> {
        let payload = self.exchange_checked(request_id, request_len, MessageType::RUN_REPORT)?;
        let (report, decoder) = decode_run_report_header(payload)?;
        decoder.finish()?;
        Ok(report.into())
    }

    fn exchange_checked(
        &mut self,
        request_id: u16,
        request_len: usize,
        expected_message: MessageType,
    ) -> Result<&[u8], ClientError> {
        let response_len = self
            .transport
            .exchange_raw_frame(&self.request[..request_len], &mut self.response)?;
        let frame = decode_frame(&self.response[..response_len])?;
        if frame.request_id != request_id {
            return Err(ClientError::RequestIdMismatch {
                expected: request_id,
                actual: frame.request_id,
            });
        }
        if frame.flags & FLAG_IS_ERROR_RESPONSE != 0 {
            return Err(ClientError::Board(
                decode_error_payload(frame.payload)?.into(),
            ));
        }
        if frame.message_type != expected_message {
            return Err(ClientError::UnexpectedMessage {
                expected: expected_message,
                actual: frame.message_type,
            });
        }
        Ok(frame.payload)
    }
}

impl<T, const PAYLOAD_BYTES: usize, const REQUEST_BYTES: usize, const RESPONSE_BYTES: usize> Default
    for BoardVmClient<T, PAYLOAD_BYTES, REQUEST_BYTES, RESPONSE_BYTES>
where
    T: RawFrameTransport + Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_loopback::{FakeEvent, LoopbackBoard, LOOPBACK_BOARD_ID, LOOPBACK_RUNTIME_ID};
    use board_vm_protocol::{RunStatus, CAP_PROGRAM_RAM_EXEC};
    use board_vm_runtime::{GpioMode, Level};

    struct LoopbackTransport {
        board: LoopbackBoard<256, 8, 8>,
        board_payload: [u8; 256],
    }

    impl LoopbackTransport {
        fn new() -> Self {
            Self {
                board: LoopbackBoard::new(),
                board_payload: [0; 256],
            }
        }
    }

    impl RawFrameTransport for LoopbackTransport {
        fn exchange_raw_frame(
            &mut self,
            request: &[u8],
            response_out: &mut [u8],
        ) -> Result<usize, TransportError> {
            self.board
                .handle_raw_frame(request, &mut self.board_payload, response_out)
                .map_err(|_| TransportError::Io)
        }
    }

    #[test]
    fn handshakes_and_reads_descriptor_over_loopback() {
        let transport = LoopbackTransport::new();
        let mut client: BoardVmClient<_, 256, 512, 512> = BoardVmClient::new(transport);

        let hello = client.hello_with_name("client-test", 0xCAFE_BABE).unwrap();
        assert_eq!(hello.selected_version, 1);
        assert_eq!(hello.board_name, LOOPBACK_BOARD_ID);
        assert_eq!(hello.runtime_name, LOOPBACK_RUNTIME_ID);
        assert_eq!(hello.host_nonce, 0xCAFE_BABE);

        let descriptor = client.query_caps().unwrap();
        assert_eq!(descriptor.board_id, LOOPBACK_BOARD_ID);
        assert_eq!(descriptor.runtime_id, LOOPBACK_RUNTIME_ID);
        assert_eq!(descriptor.max_program_bytes, 256);
        assert_eq!(descriptor.max_stack_values, 8);
        assert_eq!(descriptor.max_handles, 8);
        assert!(!descriptor.supports_store_program);
        assert!(descriptor
            .capabilities
            .iter()
            .any(|capability| capability.id == CAP_PROGRAM_RAM_EXEC));
    }

    #[test]
    fn uploads_runs_and_stops_blink_over_loopback_transport() {
        let transport = LoopbackTransport::new();
        let mut client: BoardVmClient<_, 256, 512, 512> = BoardVmClient::new(transport);

        client.hello(0x1234_5678).unwrap();
        client.query_caps().unwrap();
        let run = client.blink_onboard_led(1, 100).unwrap();
        assert_eq!(run.program_id, 1);
        assert_eq!(run.status, RunStatus::Running);
        assert!(run.instructions_executed > 0);
        assert_eq!(run.open_handles, 1);

        let board = &client.transport().board;
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

        let stop = client.stop().unwrap();
        assert_eq!(stop.status, RunStatus::Stopped);
        assert_eq!(stop.open_handles, 0);
    }

    #[test]
    fn surfaces_board_errors() {
        let transport = LoopbackTransport::new();
        let mut client: BoardVmClient<_, 256, 512, 512> = BoardVmClient::new(transport);
        let err = client.run_background(99, 10).unwrap_err();
        match err {
            ClientError::Board(board) => {
                assert_eq!(board.code, 0x0200);
                assert_eq!(board.message, "invalid program");
            }
            other => panic!("expected board error, got {other:?}"),
        }
    }
}
