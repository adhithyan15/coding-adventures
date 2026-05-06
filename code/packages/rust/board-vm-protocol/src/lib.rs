#![no_std]

use core::str;

pub const PROTOCOL_VERSION: u8 = 1;
pub const FRAME_CRC_BYTES: usize = 2;

pub const FLAG_RESPONSE_REQUIRED: u8 = 0b0000_0001;
pub const FLAG_IS_RESPONSE: u8 = 0b0000_0010;
pub const FLAG_IS_ERROR_RESPONSE: u8 = 0b0000_0100;
pub const FLAG_COMPRESSED_PAYLOAD: u8 = 0b0000_1000;
pub const ALLOWED_V1_FLAGS: u8 = FLAG_RESPONSE_REQUIRED | FLAG_IS_RESPONSE | FLAG_IS_ERROR_RESPONSE;

pub const CAP_PROGRAM_RAM_EXEC: u16 = 0x7001;
pub const CAP_PROGRAM_STORE: u16 = 0x7002;
pub const CAP_TRANSPORT_PIPELINING: u16 = 0x7003;

pub const CAP_FLAG_BYTECODE_CALLABLE: u16 = 0b0000_0001;
pub const CAP_FLAG_PROTOCOL_FEATURE: u16 = 0b0000_0010;
pub const CAP_FLAG_BOARD_METADATA: u16 = 0b0000_0100;

pub const RUN_FLAG_RESET_VM_BEFORE_RUN: u8 = 0b0000_0001;
pub const RUN_FLAG_KEEP_HANDLES_AFTER_RUN: u8 = 0b0000_0010;
pub const RUN_FLAG_BACKGROUND_RUN: u8 = 0b0000_0100;
pub const ALLOWED_RUN_FLAGS: u8 =
    RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_KEEP_HANDLES_AFTER_RUN | RUN_FLAG_BACKGROUND_RUN;

pub const BOOT_STORE_ONLY: u8 = 0x00;
pub const BOOT_RUN_AT_BOOT: u8 = 0x01;
pub const BOOT_RUN_IF_NO_HOST: u8 = 0x02;

pub const NO_PROGRAM_ID: u16 = 0xFFFF;
pub const NO_BYTECODE_OFFSET: u32 = 0xFFFF_FFFF;

pub const GOLDEN_HELLO_PAYLOAD_BVM_V1: [u8; 10] =
    [0x01, 0x01, 0x03, b'b', b'v', b'm', 0xCD, 0xAB, 0x34, 0x12];
pub const GOLDEN_HELLO_RAW_FRAME_BVM_V1: [u8; 18] = [
    0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, b'b', b'v', b'm', 0xCD, 0xAB, 0x34, 0x12,
    0x19, 0x49,
];
pub const GOLDEN_HELLO_WIRE_FRAME_BVM_V1: [u8; 20] = [
    0x13, 0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, b'b', b'v', b'm', 0xCD, 0xAB, 0x34,
    0x12, 0x19, 0x49, 0x00,
];
pub const GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1: [u8; 11] = [
    0x01, 0x00, 0x01, 0x24, 0x00, 0x00, 0x00, 0xBE, 0xBA, 0xFE, 0xCA,
];
pub const GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1: [u8; 11] = [
    0x01, 0x00, 0x05, 0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    OutputTooSmall,
    InputTooShort,
    MissingTerminator,
    InvalidCobs,
    TruncatedUleb,
    UlebOverflow,
    PayloadTooLarge,
    PayloadLengthMismatch,
    BadCrc,
    UnsupportedVersion(u8),
    ReservedFlags(u8),
    UnsupportedValue(u8),
    InvalidBool(u8),
    InvalidUtf8,
    TrailingBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageType(pub u8);

impl MessageType {
    pub const HELLO: Self = Self(0x01);
    pub const HELLO_ACK: Self = Self(0x02);
    pub const CAPS_QUERY: Self = Self(0x03);
    pub const CAPS_REPORT: Self = Self(0x04);
    pub const PROGRAM_BEGIN: Self = Self(0x05);
    pub const PROGRAM_CHUNK: Self = Self(0x06);
    pub const PROGRAM_END: Self = Self(0x07);
    pub const RUN: Self = Self(0x08);
    pub const RUN_REPORT: Self = Self(0x09);
    pub const STOP: Self = Self(0x0A);
    pub const RESET_VM: Self = Self(0x0B);
    pub const STORE_PROGRAM: Self = Self(0x0C);
    pub const RUN_STORED: Self = Self(0x0D);
    pub const READ_STATE: Self = Self(0x0E);
    pub const STATE_REPORT: Self = Self(0x0F);
    pub const SUBSCRIBE: Self = Self(0x10);
    pub const EVENT: Self = Self(0x11);
    pub const LOG: Self = Self(0x12);
    pub const ERROR: Self = Self(0x13);
    pub const PING: Self = Self(0x14);
    pub const PONG: Self = Self(0x15);

    pub const fn is_vendor_extension(self) -> bool {
        self.0 >= 0x80
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramFormat {
    BvmModule,
}

impl ProgramFormat {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::BvmModule => 0x01,
        }
    }

    pub fn from_u8(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0x01 => Ok(Self::BvmModule),
            other => Err(ProtocolError::UnsupportedValue(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Halted,
    Running,
    Stopped,
    BudgetExceeded,
    Faulted,
}

impl RunStatus {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Halted => 0x00,
            Self::Running => 0x01,
            Self::Stopped => 0x02,
            Self::BudgetExceeded => 0x03,
            Self::Faulted => 0x04,
        }
    }

    pub fn from_u8(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0x00 => Ok(Self::Halted),
            0x01 => Ok(Self::Running),
            0x02 => Ok(Self::Stopped),
            0x03 => Ok(Self::BudgetExceeded),
            0x04 => Ok(Self::Faulted),
            other => Err(ProtocolError::UnsupportedValue(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value<'a> {
    Unit,
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    I16(i16),
    Handle(u16),
    Bytes(&'a [u8]),
    String(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    pub flags: u8,
    pub message_type: MessageType,
    pub request_id: u16,
    pub payload: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hello<'a> {
    pub min_version: u8,
    pub max_version: u8,
    pub host_name: &'a str,
    pub host_nonce: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelloAck<'a> {
    pub selected_version: u8,
    pub board_name: &'a str,
    pub runtime_name: &'a str,
    pub host_nonce: u32,
    pub board_nonce: u32,
    pub max_frame_payload: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityDescriptor<'a> {
    pub id: u16,
    pub version: u8,
    pub flags: u16,
    pub name: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapsReportHeader<'a> {
    pub board_id: &'a str,
    pub runtime_id: &'a str,
    pub max_program_bytes: u32,
    pub max_stack_values: u8,
    pub max_handles: u8,
    pub supports_store_program: bool,
    pub capability_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramBegin {
    pub program_id: u16,
    pub format: ProgramFormat,
    pub total_len: u32,
    pub program_crc32: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramChunk<'a> {
    pub program_id: u16,
    pub offset: u32,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramEnd {
    pub program_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunRequest {
    pub program_id: u16,
    pub flags: u8,
    pub instruction_budget: u32,
    pub time_budget_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunReportHeader {
    pub program_id: u16,
    pub status: RunStatus,
    pub instructions_executed: u32,
    pub elapsed_ms: u32,
    pub stack_depth: u8,
    pub open_handles: u8,
    pub return_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreProgram {
    pub program_id: u16,
    pub slot: u8,
    pub boot_policy: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorPayload<'a> {
    pub code: u16,
    pub request_id: u16,
    pub program_id: u16,
    pub bytecode_offset: u32,
    pub message: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ping {
    pub nonce: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pong {
    pub nonce: u32,
}

pub fn encode_frame(frame: &Frame<'_>, out: &mut [u8]) -> Result<usize, ProtocolError> {
    validate_flags(frame.flags)?;
    if frame.payload.len() > u32::MAX as usize {
        return Err(ProtocolError::PayloadTooLarge);
    }

    let mut encoder = Encoder::new(out);
    encoder.write_u8(PROTOCOL_VERSION)?;
    encoder.write_u8(frame.flags)?;
    encoder.write_u8(frame.message_type.0)?;
    encoder.write_u16(frame.request_id)?;
    encoder.write_uleb128(frame.payload.len() as u32)?;
    encoder.write_slice(frame.payload)?;

    let raw_len = encoder.len();
    let crc = crc16_ccitt_false(&encoder.as_slice()[..raw_len]);
    encoder.write_u16(crc)?;
    Ok(encoder.len())
}

pub fn decode_frame(bytes: &[u8]) -> Result<Frame<'_>, ProtocolError> {
    if bytes.len() < 8 {
        return Err(ProtocolError::InputTooShort);
    }
    let crc_offset = bytes.len() - FRAME_CRC_BYTES;
    let expected_crc = read_le_u16(bytes, crc_offset)?;
    let actual_crc = crc16_ccitt_false(&bytes[..crc_offset]);
    if expected_crc != actual_crc {
        return Err(ProtocolError::BadCrc);
    }

    let mut decoder = Decoder::new(&bytes[..crc_offset]);
    let version = decoder.read_u8()?;
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(version));
    }
    let flags = decoder.read_u8()?;
    validate_flags(flags)?;
    let message_type = MessageType(decoder.read_u8()?);
    let request_id = decoder.read_u16()?;
    let payload_len = decoder.read_uleb128()? as usize;
    if decoder.remaining_len() != payload_len {
        return Err(ProtocolError::PayloadLengthMismatch);
    }
    let payload = decoder.read_slice(payload_len)?;
    Ok(Frame {
        flags,
        message_type,
        request_id,
        payload,
    })
}

pub fn encode_wire_frame(raw_with_crc: &[u8], out: &mut [u8]) -> Result<usize, ProtocolError> {
    let encoded_len = cobs_encode(raw_with_crc, out)?;
    if encoded_len >= out.len() {
        return Err(ProtocolError::OutputTooSmall);
    }
    out[encoded_len] = 0;
    Ok(encoded_len + 1)
}

pub fn decode_wire_frame(wire_frame: &[u8], out: &mut [u8]) -> Result<usize, ProtocolError> {
    let frame = match wire_frame.last() {
        Some(0) => &wire_frame[..wire_frame.len() - 1],
        Some(_) => return Err(ProtocolError::MissingTerminator),
        None => return Err(ProtocolError::InputTooShort),
    };
    cobs_decode(frame, out)
}

pub fn encode_stream_frame(
    frame: &Frame<'_>,
    raw_out: &mut [u8],
    wire_out: &mut [u8],
) -> Result<usize, ProtocolError> {
    let raw_len = encode_frame(frame, raw_out)?;
    encode_wire_frame(&raw_out[..raw_len], wire_out)
}

pub fn decode_stream_frame<'a>(
    wire_frame: &[u8],
    raw_out: &'a mut [u8],
) -> Result<Frame<'a>, ProtocolError> {
    let raw_len = decode_wire_frame(wire_frame, raw_out)?;
    decode_frame(&raw_out[..raw_len])
}

pub fn cobs_encode(input: &[u8], out: &mut [u8]) -> Result<usize, ProtocolError> {
    if out.is_empty() {
        return Err(ProtocolError::OutputTooSmall);
    }

    let mut read_index = 0;
    let mut write_index = 1;
    let mut code_index = 0;
    let mut code: u8 = 1;

    while read_index < input.len() {
        if input[read_index] == 0 {
            if code_index >= out.len() {
                return Err(ProtocolError::OutputTooSmall);
            }
            out[code_index] = code;
            code_index = write_index;
            write_index = write_index
                .checked_add(1)
                .ok_or(ProtocolError::OutputTooSmall)?;
            code = 1;
            read_index += 1;
        } else {
            if write_index >= out.len() {
                return Err(ProtocolError::OutputTooSmall);
            }
            out[write_index] = input[read_index];
            write_index += 1;
            code += 1;
            read_index += 1;

            if code == 0xFF {
                if code_index >= out.len() {
                    return Err(ProtocolError::OutputTooSmall);
                }
                out[code_index] = code;
                if read_index == input.len() {
                    return Ok(write_index);
                }
                code_index = write_index;
                write_index = write_index
                    .checked_add(1)
                    .ok_or(ProtocolError::OutputTooSmall)?;
                code = 1;
            }
        }
    }

    if code_index >= out.len() {
        return Err(ProtocolError::OutputTooSmall);
    }
    out[code_index] = code;
    Ok(write_index)
}

pub fn cobs_decode(input: &[u8], out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut read_index = 0;
    let mut write_index = 0;

    while read_index < input.len() {
        let code = input[read_index];
        if code == 0 {
            return Err(ProtocolError::InvalidCobs);
        }
        read_index += 1;

        let end = read_index
            .checked_add((code - 1) as usize)
            .ok_or(ProtocolError::InvalidCobs)?;
        if end > input.len() {
            return Err(ProtocolError::InvalidCobs);
        }
        let copy_len = end - read_index;
        if write_index + copy_len > out.len() {
            return Err(ProtocolError::OutputTooSmall);
        }
        out[write_index..write_index + copy_len].copy_from_slice(&input[read_index..end]);
        write_index += copy_len;
        read_index = end;

        if code != 0xFF && read_index < input.len() {
            if write_index >= out.len() {
                return Err(ProtocolError::OutputTooSmall);
            }
            out[write_index] = 0;
            write_index += 1;
        }
    }

    Ok(write_index)
}

pub fn crc16_ccitt_false(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for byte in bytes {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

pub fn encode_hello(value: &Hello<'_>, out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u8(value.min_version)?;
    encoder.write_u8(value.max_version)?;
    encoder.write_string(value.host_name)?;
    encoder.write_u32(value.host_nonce)?;
    Ok(encoder.len())
}

pub fn decode_hello(bytes: &[u8]) -> Result<Hello<'_>, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = Hello {
        min_version: decoder.read_u8()?,
        max_version: decoder.read_u8()?,
        host_name: decoder.read_string()?,
        host_nonce: decoder.read_u32()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_hello_ack(value: &HelloAck<'_>, out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u8(value.selected_version)?;
    encoder.write_string(value.board_name)?;
    encoder.write_string(value.runtime_name)?;
    encoder.write_u32(value.host_nonce)?;
    encoder.write_u32(value.board_nonce)?;
    encoder.write_u16(value.max_frame_payload)?;
    Ok(encoder.len())
}

pub fn decode_hello_ack(bytes: &[u8]) -> Result<HelloAck<'_>, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = HelloAck {
        selected_version: decoder.read_u8()?,
        board_name: decoder.read_string()?,
        runtime_name: decoder.read_string()?,
        host_nonce: decoder.read_u32()?,
        board_nonce: decoder.read_u32()?,
        max_frame_payload: decoder.read_u16()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_capability_descriptor(
    value: &CapabilityDescriptor<'_>,
    out: &mut [u8],
) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.id)?;
    encoder.write_u8(value.version)?;
    encoder.write_u16(value.flags)?;
    encoder.write_string(value.name)?;
    Ok(encoder.len())
}

pub fn decode_capability_descriptor(
    bytes: &[u8],
) -> Result<CapabilityDescriptor<'_>, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = decoder.read_capability_descriptor()?;
    decoder.finish()?;
    Ok(value)
}

pub fn encode_caps_report(
    header: &CapsReportHeader<'_>,
    capabilities: &[CapabilityDescriptor<'_>],
    out: &mut [u8],
) -> Result<usize, ProtocolError> {
    if header.capability_count != capabilities.len() as u32 {
        return Err(ProtocolError::PayloadLengthMismatch);
    }
    let mut encoder = Encoder::new(out);
    encoder.write_string(header.board_id)?;
    encoder.write_string(header.runtime_id)?;
    encoder.write_u32(header.max_program_bytes)?;
    encoder.write_u8(header.max_stack_values)?;
    encoder.write_u8(header.max_handles)?;
    encoder.write_bool(header.supports_store_program)?;
    encoder.write_uleb128(header.capability_count)?;
    for capability in capabilities {
        encoder.write_capability_descriptor(capability)?;
    }
    Ok(encoder.len())
}

pub fn decode_caps_report_header(
    bytes: &[u8],
) -> Result<(CapsReportHeader<'_>, Decoder<'_>), ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let header = CapsReportHeader {
        board_id: decoder.read_string()?,
        runtime_id: decoder.read_string()?,
        max_program_bytes: decoder.read_u32()?,
        max_stack_values: decoder.read_u8()?,
        max_handles: decoder.read_u8()?,
        supports_store_program: decoder.read_bool()?,
        capability_count: decoder.read_uleb128()?,
    };
    Ok((header, decoder))
}

pub fn encode_program_begin(value: &ProgramBegin, out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.program_id)?;
    encoder.write_u8(value.format.as_u8())?;
    encoder.write_u32(value.total_len)?;
    encoder.write_u32(value.program_crc32)?;
    Ok(encoder.len())
}

pub fn decode_program_begin(bytes: &[u8]) -> Result<ProgramBegin, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = ProgramBegin {
        program_id: decoder.read_u16()?,
        format: ProgramFormat::from_u8(decoder.read_u8()?)?,
        total_len: decoder.read_u32()?,
        program_crc32: decoder.read_u32()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_program_chunk(
    value: &ProgramChunk<'_>,
    out: &mut [u8],
) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.program_id)?;
    encoder.write_u32(value.offset)?;
    encoder.write_bytes(value.bytes)?;
    Ok(encoder.len())
}

pub fn decode_program_chunk(bytes: &[u8]) -> Result<ProgramChunk<'_>, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = ProgramChunk {
        program_id: decoder.read_u16()?,
        offset: decoder.read_u32()?,
        bytes: decoder.read_bytes()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_program_end(value: &ProgramEnd, out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.program_id)?;
    Ok(encoder.len())
}

pub fn decode_program_end(bytes: &[u8]) -> Result<ProgramEnd, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = ProgramEnd {
        program_id: decoder.read_u16()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_run_request(value: &RunRequest, out: &mut [u8]) -> Result<usize, ProtocolError> {
    if value.flags & !ALLOWED_RUN_FLAGS != 0 {
        return Err(ProtocolError::ReservedFlags(
            value.flags & !ALLOWED_RUN_FLAGS,
        ));
    }
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.program_id)?;
    encoder.write_u8(value.flags)?;
    encoder.write_u32(value.instruction_budget)?;
    encoder.write_u32(value.time_budget_ms)?;
    Ok(encoder.len())
}

pub fn decode_run_request(bytes: &[u8]) -> Result<RunRequest, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let program_id = decoder.read_u16()?;
    let flags = decoder.read_u8()?;
    if flags & !ALLOWED_RUN_FLAGS != 0 {
        return Err(ProtocolError::ReservedFlags(flags & !ALLOWED_RUN_FLAGS));
    }
    let value = RunRequest {
        program_id,
        flags,
        instruction_budget: decoder.read_u32()?,
        time_budget_ms: decoder.read_u32()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_run_report_header(
    value: &RunReportHeader,
    out: &mut [u8],
) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.program_id)?;
    encoder.write_u8(value.status.as_u8())?;
    encoder.write_u32(value.instructions_executed)?;
    encoder.write_u32(value.elapsed_ms)?;
    encoder.write_u8(value.stack_depth)?;
    encoder.write_u8(value.open_handles)?;
    encoder.write_uleb128(value.return_count)?;
    Ok(encoder.len())
}

pub fn decode_run_report_header(
    bytes: &[u8],
) -> Result<(RunReportHeader, Decoder<'_>), ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = RunReportHeader {
        program_id: decoder.read_u16()?,
        status: RunStatus::from_u8(decoder.read_u8()?)?,
        instructions_executed: decoder.read_u32()?,
        elapsed_ms: decoder.read_u32()?,
        stack_depth: decoder.read_u8()?,
        open_handles: decoder.read_u8()?,
        return_count: decoder.read_uleb128()?,
    };
    Ok((value, decoder))
}

pub fn encode_store_program(value: &StoreProgram, out: &mut [u8]) -> Result<usize, ProtocolError> {
    validate_boot_policy(value.boot_policy)?;
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.program_id)?;
    encoder.write_u8(value.slot)?;
    encoder.write_u8(value.boot_policy)?;
    Ok(encoder.len())
}

pub fn decode_store_program(bytes: &[u8]) -> Result<StoreProgram, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = StoreProgram {
        program_id: decoder.read_u16()?,
        slot: decoder.read_u8()?,
        boot_policy: decoder.read_u8()?,
    };
    validate_boot_policy(value.boot_policy)?;
    decoder.finish()?;
    Ok(value)
}

pub fn encode_error_payload(
    value: &ErrorPayload<'_>,
    out: &mut [u8],
) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u16(value.code)?;
    encoder.write_u16(value.request_id)?;
    encoder.write_u16(value.program_id)?;
    encoder.write_u32(value.bytecode_offset)?;
    encoder.write_string(value.message)?;
    Ok(encoder.len())
}

pub fn decode_error_payload(bytes: &[u8]) -> Result<ErrorPayload<'_>, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = ErrorPayload {
        code: decoder.read_u16()?,
        request_id: decoder.read_u16()?,
        program_id: decoder.read_u16()?,
        bytecode_offset: decoder.read_u32()?,
        message: decoder.read_string()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_ping(value: &Ping, out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u32(value.nonce)?;
    Ok(encoder.len())
}

pub fn decode_ping(bytes: &[u8]) -> Result<Ping, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = Ping {
        nonce: decoder.read_u32()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_pong(value: &Pong, out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_u32(value.nonce)?;
    Ok(encoder.len())
}

pub fn decode_pong(bytes: &[u8]) -> Result<Pong, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = Pong {
        nonce: decoder.read_u32()?,
    };
    decoder.finish()?;
    Ok(value)
}

pub fn encode_value(value: &Value<'_>, out: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut encoder = Encoder::new(out);
    encoder.write_value(value)?;
    Ok(encoder.len())
}

pub fn decode_value(bytes: &[u8]) -> Result<Value<'_>, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = decoder.read_value()?;
    decoder.finish()?;
    Ok(value)
}

pub struct Encoder<'a> {
    out: &'a mut [u8],
    len: usize,
}

impl<'a> Encoder<'a> {
    pub fn new(out: &'a mut [u8]) -> Self {
        Self { out, len: 0 }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.out[..self.len]
    }

    pub fn write_u8(&mut self, value: u8) -> Result<(), ProtocolError> {
        self.write_slice(&[value])
    }

    pub fn write_bool(&mut self, value: bool) -> Result<(), ProtocolError> {
        self.write_u8(if value { 1 } else { 0 })
    }

    pub fn write_u16(&mut self, value: u16) -> Result<(), ProtocolError> {
        self.write_slice(&value.to_le_bytes())
    }

    pub fn write_u32(&mut self, value: u32) -> Result<(), ProtocolError> {
        self.write_slice(&value.to_le_bytes())
    }

    pub fn write_i16(&mut self, value: i16) -> Result<(), ProtocolError> {
        self.write_slice(&value.to_le_bytes())
    }

    pub fn write_uleb128(&mut self, mut value: u32) -> Result<(), ProtocolError> {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.write_u8(byte)?;
            if value == 0 {
                return Ok(());
            }
        }
    }

    pub fn write_string(&mut self, value: &str) -> Result<(), ProtocolError> {
        self.write_bytes(value.as_bytes())
    }

    pub fn write_bytes(&mut self, value: &[u8]) -> Result<(), ProtocolError> {
        if value.len() > u32::MAX as usize {
            return Err(ProtocolError::PayloadTooLarge);
        }
        self.write_uleb128(value.len() as u32)?;
        self.write_slice(value)
    }

    pub fn write_capability_descriptor(
        &mut self,
        value: &CapabilityDescriptor<'_>,
    ) -> Result<(), ProtocolError> {
        self.write_u16(value.id)?;
        self.write_u8(value.version)?;
        self.write_u16(value.flags)?;
        self.write_string(value.name)
    }

    pub fn write_value(&mut self, value: &Value<'_>) -> Result<(), ProtocolError> {
        match value {
            Value::Unit => self.write_u8(0x00),
            Value::Bool(value) => {
                self.write_u8(0x01)?;
                self.write_bool(*value)
            }
            Value::U8(value) => {
                self.write_u8(0x02)?;
                self.write_u8(*value)
            }
            Value::U16(value) => {
                self.write_u8(0x03)?;
                self.write_u16(*value)
            }
            Value::U32(value) => {
                self.write_u8(0x04)?;
                self.write_u32(*value)
            }
            Value::I16(value) => {
                self.write_u8(0x05)?;
                self.write_i16(*value)
            }
            Value::Handle(value) => {
                self.write_u8(0x06)?;
                self.write_u16(*value)
            }
            Value::Bytes(value) => {
                self.write_u8(0x07)?;
                self.write_bytes(value)
            }
            Value::String(value) => {
                self.write_u8(0x08)?;
                self.write_string(value)
            }
        }
    }

    pub fn write_slice(&mut self, value: &[u8]) -> Result<(), ProtocolError> {
        let end = self
            .len
            .checked_add(value.len())
            .ok_or(ProtocolError::OutputTooSmall)?;
        if end > self.out.len() {
            return Err(ProtocolError::OutputTooSmall);
        }
        self.out[self.len..end].copy_from_slice(value);
        self.len = end;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoder<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    pub const fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn remaining_len(&self) -> usize {
        self.input.len() - self.offset
    }

    pub fn finish(&self) -> Result<(), ProtocolError> {
        if self.offset == self.input.len() {
            Ok(())
        } else {
            Err(ProtocolError::TrailingBytes)
        }
    }

    pub fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        let value = *self
            .input
            .get(self.offset)
            .ok_or(ProtocolError::InputTooShort)?;
        self.offset += 1;
        Ok(value)
    }

    pub fn read_bool(&mut self) -> Result<bool, ProtocolError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(ProtocolError::InvalidBool(other)),
        }
    }

    pub fn read_u16(&mut self) -> Result<u16, ProtocolError> {
        let value = read_le_u16(self.input, self.offset)?;
        self.offset += 2;
        Ok(value)
    }

    pub fn read_u32(&mut self) -> Result<u32, ProtocolError> {
        let value = read_le_u32(self.input, self.offset)?;
        self.offset += 4;
        Ok(value)
    }

    pub fn read_i16(&mut self) -> Result<i16, ProtocolError> {
        Ok(self.read_u16()? as i16)
    }

    pub fn read_uleb128(&mut self) -> Result<u32, ProtocolError> {
        let mut value = 0u32;
        let mut shift = 0;
        loop {
            if shift >= 35 {
                return Err(ProtocolError::UlebOverflow);
            }
            let byte = self.read_u8().map_err(|_| ProtocolError::TruncatedUleb)?;
            let chunk = (byte & 0x7F) as u32;
            if shift == 28 && chunk > 0x0F {
                return Err(ProtocolError::UlebOverflow);
            }
            value |= chunk << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
    }

    pub fn read_string(&mut self) -> Result<&'a str, ProtocolError> {
        let bytes = self.read_bytes()?;
        str::from_utf8(bytes).map_err(|_| ProtocolError::InvalidUtf8)
    }

    pub fn read_bytes(&mut self) -> Result<&'a [u8], ProtocolError> {
        let len = self.read_uleb128()? as usize;
        self.read_slice(len)
    }

    pub fn read_slice(&mut self, len: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(ProtocolError::PayloadTooLarge)?;
        if end > self.input.len() {
            return Err(ProtocolError::InputTooShort);
        }
        let slice = &self.input[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    pub fn read_capability_descriptor(
        &mut self,
    ) -> Result<CapabilityDescriptor<'a>, ProtocolError> {
        Ok(CapabilityDescriptor {
            id: self.read_u16()?,
            version: self.read_u8()?,
            flags: self.read_u16()?,
            name: self.read_string()?,
        })
    }

    pub fn read_value(&mut self) -> Result<Value<'a>, ProtocolError> {
        match self.read_u8()? {
            0x00 => Ok(Value::Unit),
            0x01 => Ok(Value::Bool(self.read_bool()?)),
            0x02 => Ok(Value::U8(self.read_u8()?)),
            0x03 => Ok(Value::U16(self.read_u16()?)),
            0x04 => Ok(Value::U32(self.read_u32()?)),
            0x05 => Ok(Value::I16(self.read_i16()?)),
            0x06 => Ok(Value::Handle(self.read_u16()?)),
            0x07 => Ok(Value::Bytes(self.read_bytes()?)),
            0x08 => Ok(Value::String(self.read_string()?)),
            other => Err(ProtocolError::UnsupportedValue(other)),
        }
    }
}

fn validate_flags(flags: u8) -> Result<(), ProtocolError> {
    if flags & !ALLOWED_V1_FLAGS != 0 {
        Err(ProtocolError::ReservedFlags(flags & !ALLOWED_V1_FLAGS))
    } else {
        Ok(())
    }
}

fn validate_boot_policy(value: u8) -> Result<(), ProtocolError> {
    match value {
        BOOT_STORE_ONLY | BOOT_RUN_AT_BOOT | BOOT_RUN_IF_NO_HOST => Ok(()),
        other => Err(ProtocolError::UnsupportedValue(other)),
    }
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Result<u16, ProtocolError> {
    let end = offset.checked_add(2).ok_or(ProtocolError::InputTooShort)?;
    if end > bytes.len() {
        return Err(ProtocolError::InputTooShort);
    }
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, ProtocolError> {
    let end = offset.checked_add(4).ok_or(ProtocolError::InputTooShort)?;
    if end > bytes.len() {
        return Err(ProtocolError::InputTooShort);
    }
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_matches_standard_check_vector() {
        assert_eq!(crc16_ccitt_false(b"123456789"), 0x29B1);
    }

    #[test]
    fn encodes_uleb128_boundaries() {
        let mut out = [0u8; 12];
        let mut encoder = Encoder::new(&mut out);
        encoder.write_uleb128(0).unwrap();
        encoder.write_uleb128(127).unwrap();
        encoder.write_uleb128(128).unwrap();
        encoder.write_uleb128(16_383).unwrap();
        encoder.write_uleb128(16_384).unwrap();
        assert_eq!(
            encoder.as_slice(),
            &[0x00, 0x7F, 0x80, 0x01, 0xFF, 0x7F, 0x80, 0x80, 0x01]
        );
    }

    #[test]
    fn encodes_and_decodes_hello_payload() {
        let hello = Hello {
            min_version: 1,
            max_version: 1,
            host_name: "bvm",
            host_nonce: 0x1234_ABCD,
        };
        let mut payload = [0u8; 16];
        let len = encode_hello(&hello, &mut payload).unwrap();
        assert_eq!(&payload[..len], GOLDEN_HELLO_PAYLOAD_BVM_V1);
        assert_eq!(decode_hello(&payload[..len]).unwrap(), hello);
    }

    #[test]
    fn encodes_raw_frame_with_crc() {
        let payload = [0x01, 0x01, 0x03, b'b', b'v', b'm', 0xCD, 0xAB, 0x34, 0x12];
        let frame = Frame {
            flags: FLAG_RESPONSE_REQUIRED,
            message_type: MessageType::HELLO,
            request_id: 0x1234,
            payload: &payload,
        };
        let mut out = [0u8; 32];
        let len = encode_frame(&frame, &mut out).unwrap();
        assert_eq!(&out[..len], GOLDEN_HELLO_RAW_FRAME_BVM_V1);
        assert_eq!(decode_frame(&out[..len]).unwrap(), frame);
    }

    #[test]
    fn encodes_hello_wire_frame_golden_vector() {
        let frame = Frame {
            flags: FLAG_RESPONSE_REQUIRED,
            message_type: MessageType::HELLO,
            request_id: 0x1234,
            payload: &GOLDEN_HELLO_PAYLOAD_BVM_V1,
        };
        let mut raw = [0u8; 32];
        let mut wire = [0u8; 32];

        let wire_len = encode_stream_frame(&frame, &mut raw, &mut wire).unwrap();

        assert_eq!(
            &raw[..GOLDEN_HELLO_RAW_FRAME_BVM_V1.len()],
            GOLDEN_HELLO_RAW_FRAME_BVM_V1
        );
        assert_eq!(&wire[..wire_len], GOLDEN_HELLO_WIRE_FRAME_BVM_V1);
    }

    #[test]
    fn rejects_bad_crc() {
        let mut raw = [
            0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, b'b', b'v', b'm', 0xCD, 0xAB,
            0x34, 0x12, 0x19, 0x49,
        ];
        raw[8] ^= 0x01;
        assert_eq!(decode_frame(&raw), Err(ProtocolError::BadCrc));
    }

    #[test]
    fn cobs_round_trips_payload_with_zeroes() {
        let raw = [0x11, 0x00, 0x22, 0x33, 0x00, 0x44];
        let mut encoded = [0u8; 16];
        let encoded_len = encode_wire_frame(&raw, &mut encoded).unwrap();
        assert_eq!(
            &encoded[..encoded_len],
            &[0x02, 0x11, 0x03, 0x22, 0x33, 0x02, 0x44, 0x00]
        );

        let mut decoded = [0u8; 16];
        let decoded_len = decode_wire_frame(&encoded[..encoded_len], &mut decoded).unwrap();
        assert_eq!(&decoded[..decoded_len], &raw);
    }

    #[test]
    fn cobs_uses_canonical_full_nonzero_block() {
        let raw = [0x7Au8; 254];
        let mut encoded = [0u8; 256];
        let encoded_len = encode_wire_frame(&raw, &mut encoded).unwrap();
        assert_eq!(encoded_len, 256);
        assert_eq!(encoded[0], 0xFF);
        assert_eq!(encoded[255], 0x00);

        let mut decoded = [0u8; 254];
        let decoded_len = decode_wire_frame(&encoded[..encoded_len], &mut decoded).unwrap();
        assert_eq!(decoded_len, raw.len());
        assert_eq!(decoded, raw);
    }

    #[test]
    fn encodes_and_decodes_stream_frame() {
        let payload = [0x14, 0x00, 0x00, 0x00];
        let frame = Frame {
            flags: FLAG_RESPONSE_REQUIRED,
            message_type: MessageType::PING,
            request_id: 7,
            payload: &payload,
        };
        let mut raw = [0u8; 32];
        let mut wire = [0u8; 40];
        let wire_len = encode_stream_frame(&frame, &mut raw, &mut wire).unwrap();
        assert_eq!(wire[wire_len - 1], 0);

        let mut decoded_raw = [0u8; 32];
        let decoded = decode_stream_frame(&wire[..wire_len], &mut decoded_raw).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn rejects_reserved_frame_flags() {
        let frame = Frame {
            flags: FLAG_COMPRESSED_PAYLOAD,
            message_type: MessageType::PING,
            request_id: 1,
            payload: &[],
        };
        let mut out = [0u8; 16];
        assert_eq!(
            encode_frame(&frame, &mut out),
            Err(ProtocolError::ReservedFlags(FLAG_COMPRESSED_PAYLOAD))
        );
    }

    #[test]
    fn encodes_upload_and_run_payloads() {
        let mut out = [0u8; 32];
        let begin = ProgramBegin {
            program_id: 1,
            format: ProgramFormat::BvmModule,
            total_len: 36,
            program_crc32: 0xCAFE_BABE,
        };
        let len = encode_program_begin(&begin, &mut out).unwrap();
        assert_eq!(&out[..len], GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1);
        assert_eq!(decode_program_begin(&out[..len]).unwrap(), begin);

        let run = RunRequest {
            program_id: 1,
            flags: RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN,
            instruction_budget: 1000,
            time_budget_ms: 0,
        };
        let len = encode_run_request(&run, &mut out).unwrap();
        assert_eq!(&out[..len], GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1);
        assert_eq!(decode_run_request(&out[..len]).unwrap(), run);
    }

    #[test]
    fn caps_report_header_leaves_decoder_at_first_capability() {
        let caps = [
            CapabilityDescriptor {
                id: CAP_PROGRAM_RAM_EXEC,
                version: 1,
                flags: CAP_FLAG_PROTOCOL_FEATURE,
                name: "program.ram_exec",
            },
            CapabilityDescriptor {
                id: CAP_PROGRAM_STORE,
                version: 1,
                flags: CAP_FLAG_PROTOCOL_FEATURE,
                name: "program.store",
            },
        ];
        let header = CapsReportHeader {
            board_id: "uno-r4-minima",
            runtime_id: "board-vm-rust",
            max_program_bytes: 1024,
            max_stack_values: 8,
            max_handles: 8,
            supports_store_program: true,
            capability_count: caps.len() as u32,
        };
        let mut out = [0u8; 128];
        let len = encode_caps_report(&header, &caps, &mut out).unwrap();
        let (decoded_header, mut decoder) = decode_caps_report_header(&out[..len]).unwrap();
        assert_eq!(decoded_header, header);
        assert_eq!(decoder.read_capability_descriptor().unwrap(), caps[0]);
        assert_eq!(decoder.read_capability_descriptor().unwrap(), caps[1]);
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn values_reject_unknown_tag() {
        assert_eq!(
            decode_value(&[0x99]),
            Err(ProtocolError::UnsupportedValue(0x99))
        );
    }
}
