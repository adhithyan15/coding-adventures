use board_vm_ir::{
    parse_module, validate, CapabilitySet, ModuleError, ValidateError, CAP_GPIO_OPEN,
    CAP_GPIO_WRITE, CAP_TIME_SLEEP_MS, FLAG_PROGRAM_MAY_RUN_FOREVER, MODULE_MAGIC, MODULE_VERSION,
};
use board_vm_protocol::{
    encode_frame, encode_hello, encode_program_begin, encode_program_chunk, encode_program_end,
    encode_run_request, encode_store_program, encode_stream_frame, encode_wire_frame, Frame, Hello,
    MessageType, ProgramBegin, ProgramChunk, ProgramEnd, ProgramFormat, ProtocolError, RunRequest,
    StoreProgram, BOOT_RUN_IF_NO_HOST, FLAG_RESPONSE_REQUIRED, RUN_FLAG_BACKGROUND_RUN,
    RUN_FLAG_RESET_VM_BEFORE_RUN,
};

pub const DEFAULT_HOST_NAME: &str = "board-vm-host";
pub const DEFAULT_PROGRAM_ID: u16 = 1;
pub const DEFAULT_INSTRUCTION_BUDGET: u32 = 1000;
pub const BLINK_CODE_LEN: usize = 26;
pub const BLINK_MODULE_LEN: usize = 36;

const OP_PUSH_FALSE: u8 = 0x10;
const OP_PUSH_TRUE: u8 = 0x11;
const OP_PUSH_U8: u8 = 0x12;
const OP_PUSH_U16: u8 = 0x13;
const OP_DUP: u8 = 0x20;
const OP_JUMP_S8: u8 = 0x30;
const OP_CALL_U8: u8 = 0x40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    OutputTooSmall,
    Protocol(ProtocolError),
    Module(ModuleError),
    Validate(ValidateError),
    ProgramTooLarge,
    JumpOutOfRange,
}

impl From<ProtocolError> for HostError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<ModuleError> for HostError {
    fn from(value: ModuleError) -> Self {
        Self::Module(value)
    }
}

impl From<ValidateError> for HostError {
    fn from(value: ValidateError) -> Self {
        Self::Validate(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlinkProgram {
    pub pin: u8,
    pub high_ms: u16,
    pub low_ms: u16,
    pub max_stack: u8,
}

impl BlinkProgram {
    pub const fn onboard_led() -> Self {
        Self {
            pin: 13,
            high_ms: 250,
            low_ms: 250,
            max_stack: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleSpec<'a> {
    pub flags: u8,
    pub max_stack: u8,
    pub code: &'a [u8],
    pub const_pool: &'a [u8],
}

impl<'a> ModuleSpec<'a> {
    pub const fn new(flags: u8, max_stack: u8, code: &'a [u8]) -> Self {
        Self {
            flags,
            max_stack,
            code,
            const_pool: &[],
        }
    }

    pub const fn const_pool(mut self, const_pool: &'a [u8]) -> Self {
        self.const_pool = const_pool;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrittenFrame {
    pub request_id: u16,
    pub len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostSession {
    next_request_id: u16,
}

impl HostSession {
    pub const fn new() -> Self {
        Self { next_request_id: 1 }
    }

    pub const fn with_next_request_id(next_request_id: u16) -> Self {
        Self {
            next_request_id: if next_request_id == 0 {
                1
            } else {
                next_request_id
            },
        }
    }

    pub const fn next_request_id(&self) -> u16 {
        self.next_request_id
    }

    pub fn hello_frame(
        &mut self,
        host_name: &str,
        host_nonce: u32,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_hello(
            &Hello {
                min_version: 1,
                max_version: 1,
                host_name,
                host_nonce,
            },
            payload_out,
        )?;
        self.request_frame(MessageType::HELLO, &payload_out[..payload_len], frame_out)
    }

    pub fn caps_query_frame(&mut self, frame_out: &mut [u8]) -> Result<WrittenFrame, HostError> {
        self.request_frame(MessageType::CAPS_QUERY, &[], frame_out)
    }

    pub fn program_begin_frame(
        &mut self,
        program_id: u16,
        module: &[u8],
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        if module.len() > u32::MAX as usize {
            return Err(HostError::ProgramTooLarge);
        }
        let payload_len = encode_program_begin(
            &ProgramBegin {
                program_id,
                format: ProgramFormat::BvmModule,
                total_len: module.len() as u32,
                program_crc32: crc32_ieee(module),
            },
            payload_out,
        )?;
        self.request_frame(
            MessageType::PROGRAM_BEGIN,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn program_chunk_frame(
        &mut self,
        program_id: u16,
        offset: u32,
        chunk: &[u8],
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_program_chunk(
            &ProgramChunk {
                program_id,
                offset,
                bytes: chunk,
            },
            payload_out,
        )?;
        self.request_frame(
            MessageType::PROGRAM_CHUNK,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn program_end_frame(
        &mut self,
        program_id: u16,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_program_end(&ProgramEnd { program_id }, payload_out)?;
        self.request_frame(
            MessageType::PROGRAM_END,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn run_background_frame(
        &mut self,
        program_id: u16,
        instruction_budget: u32,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_run_request(
            &RunRequest {
                program_id,
                flags: RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN,
                instruction_budget,
                time_budget_ms: 0,
            },
            payload_out,
        )?;
        self.request_frame(MessageType::RUN, &payload_out[..payload_len], frame_out)
    }

    pub fn store_program_frame(
        &mut self,
        program_id: u16,
        slot: u8,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_store_program(
            &StoreProgram {
                program_id,
                slot,
                boot_policy: BOOT_RUN_IF_NO_HOST,
            },
            payload_out,
        )?;
        self.request_frame(
            MessageType::STORE_PROGRAM,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn stop_frame(&mut self, frame_out: &mut [u8]) -> Result<WrittenFrame, HostError> {
        self.request_frame(MessageType::STOP, &[], frame_out)
    }

    pub fn request_stream_frame(
        &mut self,
        message_type: MessageType,
        payload: &[u8],
        raw_out: &mut [u8],
        wire_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let request_id = self.take_request_id();
        let len = encode_stream_frame(
            &Frame {
                flags: FLAG_RESPONSE_REQUIRED,
                message_type,
                request_id,
                payload,
            },
            raw_out,
            wire_out,
        )?;
        Ok(WrittenFrame { request_id, len })
    }

    fn request_frame(
        &mut self,
        message_type: MessageType,
        payload: &[u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let request_id = self.take_request_id();
        let len = encode_frame(
            &Frame {
                flags: FLAG_RESPONSE_REQUIRED,
                message_type,
                request_id,
                payload,
            },
            frame_out,
        )?;
        Ok(WrittenFrame { request_id, len })
    }

    fn take_request_id(&mut self) -> u16 {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        if self.next_request_id == 0 {
            self.next_request_id = 1;
        }
        request_id
    }
}

impl Default for HostSession {
    fn default() -> Self {
        Self::new()
    }
}

pub fn write_blink_code(program: BlinkProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < BLINK_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, 1)?;
    write_call_u8(out, &mut offset, CAP_GPIO_OPEN)?;

    let loop_start = offset;
    write_u8(out, &mut offset, OP_DUP)?;
    write_u8(out, &mut offset, OP_PUSH_TRUE)?;
    write_call_u8(out, &mut offset, CAP_GPIO_WRITE)?;
    write_push_u16(out, &mut offset, program.high_ms)?;
    write_call_u8(out, &mut offset, CAP_TIME_SLEEP_MS)?;
    write_u8(out, &mut offset, OP_DUP)?;
    write_u8(out, &mut offset, OP_PUSH_FALSE)?;
    write_call_u8(out, &mut offset, CAP_GPIO_WRITE)?;
    write_push_u16(out, &mut offset, program.low_ms)?;
    write_call_u8(out, &mut offset, CAP_TIME_SLEEP_MS)?;

    let jump_next = offset + 2;
    let jump_offset = loop_start as isize - jump_next as isize;
    if !(i8::MIN as isize..=i8::MAX as isize).contains(&jump_offset) {
        return Err(HostError::JumpOutOfRange);
    }
    write_u8(out, &mut offset, OP_JUMP_S8)?;
    write_u8(out, &mut offset, jump_offset as i8 as u8)?;
    Ok(offset)
}

pub fn write_blink_module(program: BlinkProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < BLINK_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; BLINK_CODE_LEN];
    let code_len = write_blink_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(
            FLAG_PROGRAM_MAY_RUN_FOREVER,
            program.max_stack,
            &code[..code_len],
        ),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_module(spec: ModuleSpec<'_>, out: &mut [u8]) -> Result<usize, HostError> {
    if spec.code.len() > u32::MAX as usize || spec.const_pool.len() > u32::MAX as usize {
        return Err(HostError::ProgramTooLarge);
    }

    let mut offset = 0;
    write_slice(out, &mut offset, &MODULE_MAGIC)?;
    write_u8(out, &mut offset, MODULE_VERSION)?;
    write_u8(out, &mut offset, spec.flags)?;
    write_u8(out, &mut offset, spec.max_stack)?;
    write_u8(out, &mut offset, 0)?;
    write_uleb128(out, &mut offset, spec.code.len() as u32)?;
    write_slice(out, &mut offset, spec.code)?;
    write_uleb128(out, &mut offset, spec.const_pool.len() as u32)?;
    write_slice(out, &mut offset, spec.const_pool)?;

    parse_module(&out[..offset])?;
    Ok(offset)
}

pub fn write_blink_upload_and_run_frames(
    session: &mut HostSession,
    program_id: u16,
    module: &[u8],
    payload_out: &mut [u8],
    frames_out: &mut [&mut [u8]; 4],
) -> Result<[WrittenFrame; 4], HostError> {
    Ok([
        session.program_begin_frame(program_id, module, payload_out, frames_out[0])?,
        session.program_chunk_frame(program_id, 0, module, payload_out, frames_out[1])?,
        session.program_end_frame(program_id, payload_out, frames_out[2])?,
        session.run_background_frame(
            program_id,
            DEFAULT_INSTRUCTION_BUDGET,
            payload_out,
            frames_out[3],
        )?,
    ])
}

pub fn write_wire_frame(raw_frame: &[u8], wire_out: &mut [u8]) -> Result<usize, HostError> {
    Ok(encode_wire_frame(raw_frame, wire_out)?)
}

pub fn crc32_ieee(bytes: &[u8]) -> u32 {
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

fn write_call_u8(out: &mut [u8], offset: &mut usize, capability: u16) -> Result<(), HostError> {
    write_u8(out, offset, OP_CALL_U8)?;
    write_u8(out, offset, capability as u8)
}

fn write_push_u16(out: &mut [u8], offset: &mut usize, value: u16) -> Result<(), HostError> {
    write_u8(out, offset, OP_PUSH_U16)?;
    write_slice(out, offset, &value.to_le_bytes())
}

fn write_uleb128(out: &mut [u8], offset: &mut usize, mut value: u32) -> Result<(), HostError> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        write_u8(out, offset, byte)?;
        if value == 0 {
            return Ok(());
        }
    }
}

fn write_u8(out: &mut [u8], offset: &mut usize, value: u8) -> Result<(), HostError> {
    write_slice(out, offset, &[value])
}

fn write_slice(out: &mut [u8], offset: &mut usize, value: &[u8]) -> Result<(), HostError> {
    let end = offset
        .checked_add(value.len())
        .ok_or(HostError::OutputTooSmall)?;
    if end > out.len() {
        return Err(HostError::OutputTooSmall);
    }
    out[*offset..end].copy_from_slice(value);
    *offset = end;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_ir::{parse_module, validate, CapabilitySet, ModuleError};
    use board_vm_protocol::{
        decode_frame, decode_program_begin, decode_program_chunk, decode_program_end,
        decode_run_request, MessageType, RUN_FLAG_BACKGROUND_RUN, RUN_FLAG_RESET_VM_BEFORE_RUN,
    };

    const BLINK_MODULE_HEX: [u8; BLINK_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x01, 0x04, 0x00, 0x1A, 0x12, 0x0D, 0x12, 0x01, 0x40, 0x01,
        0x20, 0x11, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40, 0x10, 0x20, 0x10, 0x40, 0x02, 0x13, 0xFA,
        0x00, 0x40, 0x10, 0x30, 0xEC, 0x00,
    ];

    #[test]
    fn builds_blink_module_from_bvm05_fixture() {
        let mut module = [0u8; BLINK_MODULE_LEN];
        let len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        assert_eq!(len, BLINK_MODULE_LEN);
        assert_eq!(module, BLINK_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 4).unwrap();
    }

    #[test]
    fn writes_generic_module_from_code_and_const_pool() {
        let code = [0x00];
        let const_pool = [0xAA, 0x55];
        let mut module = [0u8; 32];

        let len = write_module(
            ModuleSpec::new(0, 1, &code).const_pool(&const_pool),
            &mut module,
        )
        .unwrap();

        let parsed = parse_module(&module[..len]).unwrap();
        assert_eq!(parsed.flags, 0);
        assert_eq!(parsed.max_stack, 1);
        assert_eq!(parsed.code, &code);
        assert_eq!(parsed.const_pool, &const_pool);
    }

    #[test]
    fn rejects_invalid_generic_module_flags() {
        let code = [0x00];
        let mut module = [0u8; 16];

        assert_eq!(
            write_module(ModuleSpec::new(0x80, 1, &code), &mut module),
            Err(HostError::Module(ModuleError::ReservedFlags(0x80)))
        );
    }

    #[test]
    fn crc32_matches_standard_check_vector() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32_ieee(&BLINK_MODULE_HEX), 0xBAD6_949E);
    }

    #[test]
    fn writes_handshake_and_caps_frames() {
        let mut session = HostSession::new();
        let mut payload = [0u8; 64];
        let mut frame = [0u8; 96];

        let hello = session
            .hello_frame(DEFAULT_HOST_NAME, 0xAABB_CCDD, &mut payload, &mut frame)
            .unwrap();
        assert_eq!(hello.request_id, 1);
        let decoded = decode_frame(&frame[..hello.len]).unwrap();
        assert_eq!(decoded.message_type, MessageType::HELLO);
        assert_eq!(decoded.request_id, 1);
        assert_eq!(decoded.flags, FLAG_RESPONSE_REQUIRED);

        let caps = session.caps_query_frame(&mut frame).unwrap();
        assert_eq!(caps.request_id, 2);
        let decoded = decode_frame(&frame[..caps.len]).unwrap();
        assert_eq!(decoded.message_type, MessageType::CAPS_QUERY);
        assert_eq!(decoded.payload, &[]);
    }

    #[test]
    fn writes_upload_and_run_sequence() {
        let mut session = HostSession::with_next_request_id(10);
        let mut module = [0u8; BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let module = &module[..module_len];
        let mut payload = [0u8; 96];
        let mut begin_frame = [0u8; 128];
        let mut chunk_frame = [0u8; 160];
        let mut end_frame = [0u8; 64];
        let mut run_frame = [0u8; 96];

        let begin = session
            .program_begin_frame(DEFAULT_PROGRAM_ID, module, &mut payload, &mut begin_frame)
            .unwrap();
        let decoded = decode_frame(&begin_frame[..begin.len]).unwrap();
        assert_eq!(begin.request_id, 10);
        assert_eq!(decoded.message_type, MessageType::PROGRAM_BEGIN);
        let begin_payload = decode_program_begin(decoded.payload).unwrap();
        assert_eq!(begin_payload.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(begin_payload.total_len, BLINK_MODULE_LEN as u32);
        assert_eq!(begin_payload.program_crc32, crc32_ieee(module));

        let chunk = session
            .program_chunk_frame(
                DEFAULT_PROGRAM_ID,
                0,
                module,
                &mut payload,
                &mut chunk_frame,
            )
            .unwrap();
        let decoded = decode_frame(&chunk_frame[..chunk.len]).unwrap();
        assert_eq!(chunk.request_id, 11);
        assert_eq!(decoded.message_type, MessageType::PROGRAM_CHUNK);
        let chunk_payload = decode_program_chunk(decoded.payload).unwrap();
        assert_eq!(chunk_payload.offset, 0);
        assert_eq!(chunk_payload.bytes, module);

        let end = session
            .program_end_frame(DEFAULT_PROGRAM_ID, &mut payload, &mut end_frame)
            .unwrap();
        let decoded = decode_frame(&end_frame[..end.len]).unwrap();
        assert_eq!(end.request_id, 12);
        assert_eq!(decoded.message_type, MessageType::PROGRAM_END);
        assert_eq!(
            decode_program_end(decoded.payload).unwrap().program_id,
            DEFAULT_PROGRAM_ID
        );

        let run = session
            .run_background_frame(
                DEFAULT_PROGRAM_ID,
                DEFAULT_INSTRUCTION_BUDGET,
                &mut payload,
                &mut run_frame,
            )
            .unwrap();
        let decoded = decode_frame(&run_frame[..run.len]).unwrap();
        assert_eq!(run.request_id, 13);
        assert_eq!(decoded.message_type, MessageType::RUN);
        let run_payload = decode_run_request(decoded.payload).unwrap();
        assert_eq!(run_payload.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(
            run_payload.flags,
            RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN
        );
        assert_eq!(run_payload.instruction_budget, DEFAULT_INSTRUCTION_BUDGET);
    }

    #[test]
    fn request_ids_wrap_without_using_zero() {
        let mut session = HostSession::with_next_request_id(u16::MAX);
        let mut frame = [0u8; 32];

        let first = session.stop_frame(&mut frame).unwrap();
        let second = session.stop_frame(&mut frame).unwrap();

        assert_eq!(first.request_id, u16::MAX);
        assert_eq!(second.request_id, 1);
        assert_eq!(session.next_request_id(), 2);
    }
}
