#![no_std]

pub const MODULE_MAGIC: [u8; 4] = *b"BVM1";
pub const MODULE_VERSION: u8 = 1;

pub const FLAG_PROGRAM_MAY_RUN_FOREVER: u8 = 0b0000_0001;
pub const FLAG_PROGRAM_USES_EVENTS: u8 = 0b0000_0010;
pub const FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES: u8 = 0b0000_0100;
const KNOWN_MODULE_FLAGS: u8 = FLAG_PROGRAM_MAY_RUN_FOREVER
    | FLAG_PROGRAM_USES_EVENTS
    | FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES;

pub const CAP_GPIO_OPEN: u16 = 0x01;
pub const CAP_GPIO_WRITE: u16 = 0x02;
pub const CAP_GPIO_READ: u16 = 0x03;
pub const CAP_GPIO_CLOSE: u16 = 0x04;
pub const CAP_TIME_SLEEP_MS: u16 = 0x10;
pub const CAP_TIME_NOW_MS: u16 = 0x11;

const CAP_GPIO_OPEN_U8: u8 = CAP_GPIO_OPEN as u8;
const CAP_GPIO_WRITE_U8: u8 = CAP_GPIO_WRITE as u8;
const CAP_GPIO_READ_U8: u8 = CAP_GPIO_READ as u8;
const CAP_GPIO_CLOSE_U8: u8 = CAP_GPIO_CLOSE as u8;
const CAP_TIME_SLEEP_MS_U8: u8 = CAP_TIME_SLEEP_MS as u8;
const CAP_TIME_NOW_MS_U8: u8 = CAP_TIME_NOW_MS as u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Halt,
    Nop,
    PushFalse,
    PushTrue,
    PushU8(u8),
    PushU16(u16),
    PushU32(u32),
    PushI16(i16),
    Dup,
    Drop,
    Swap,
    Over,
    JumpS8(i8),
    JumpIfFalseS8(i8),
    JumpIfTrueS8(i8),
    CallU8(u8),
    CallU16(u16),
    ReturnTop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEof,
    UnknownOpcode(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleError {
    TooShort,
    BadMagic,
    UnsupportedVersion(u8),
    ReservedFlags(u8),
    ReservedHeaderByte(u8),
    TruncatedUleb,
    LengthOutOfBounds,
    TrailingBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidateError {
    Decode(DecodeError),
    MaxStackIsZero,
    DeclaredStackTooLarge,
    StackUnderflow(usize),
    StackOverflow(usize),
    JumpTargetOutOfBounds(usize),
    JumpTargetNotBoundary(usize),
    UnsupportedCapability(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module<'a> {
    pub flags: u8,
    pub max_stack: u8,
    pub code: &'a [u8],
    pub const_pool: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet {
    pub gpio_digital: bool,
    pub time: bool,
}

impl CapabilitySet {
    pub const fn empty() -> Self {
        Self {
            gpio_digital: false,
            time: false,
        }
    }

    pub const fn blink_mvp() -> Self {
        Self {
            gpio_digital: true,
            time: true,
        }
    }

    pub const fn supports(self, capability_id: u16) -> bool {
        match capability_id {
            CAP_GPIO_OPEN | CAP_GPIO_WRITE | CAP_GPIO_READ | CAP_GPIO_CLOSE => self.gpio_digital,
            CAP_TIME_SLEEP_MS | CAP_TIME_NOW_MS => self.time,
            _ => false,
        }
    }
}

pub fn decode_next(code: &[u8], ip: usize) -> Result<(Op, usize), DecodeError> {
    let opcode = *code.get(ip).ok_or(DecodeError::UnexpectedEof)?;
    let next = ip + 1;
    match opcode {
        0x00 => Ok((Op::Halt, next)),
        0x01 => Ok((Op::Nop, next)),
        0x10 => Ok((Op::PushFalse, next)),
        0x11 => Ok((Op::PushTrue, next)),
        0x12 => Ok((Op::PushU8(read_u8(code, next)?), next + 1)),
        0x13 => Ok((Op::PushU16(read_u16(code, next)?), next + 2)),
        0x14 => Ok((Op::PushU32(read_u32(code, next)?), next + 4)),
        0x15 => Ok((Op::PushI16(read_u16(code, next)? as i16), next + 2)),
        0x20 => Ok((Op::Dup, next)),
        0x21 => Ok((Op::Drop, next)),
        0x22 => Ok((Op::Swap, next)),
        0x23 => Ok((Op::Over, next)),
        0x30 => Ok((Op::JumpS8(read_i8(code, next)?), next + 1)),
        0x31 => Ok((Op::JumpIfFalseS8(read_i8(code, next)?), next + 1)),
        0x32 => Ok((Op::JumpIfTrueS8(read_i8(code, next)?), next + 1)),
        0x40 => Ok((Op::CallU8(read_u8(code, next)?), next + 1)),
        0x41 => Ok((Op::CallU16(read_u16(code, next)?), next + 2)),
        0x50 => Ok((Op::ReturnTop, next)),
        unknown => Err(DecodeError::UnknownOpcode(unknown)),
    }
}

pub fn parse_module(bytes: &[u8]) -> Result<Module<'_>, ModuleError> {
    if bytes.len() < 8 {
        return Err(ModuleError::TooShort);
    }
    if bytes[0..4] != MODULE_MAGIC {
        return Err(ModuleError::BadMagic);
    }
    if bytes[4] != MODULE_VERSION {
        return Err(ModuleError::UnsupportedVersion(bytes[4]));
    }
    let flags = bytes[5];
    if flags & !KNOWN_MODULE_FLAGS != 0 {
        return Err(ModuleError::ReservedFlags(flags & !KNOWN_MODULE_FLAGS));
    }
    let max_stack = bytes[6];
    if bytes[7] != 0 {
        return Err(ModuleError::ReservedHeaderByte(bytes[7]));
    }

    let mut offset = 8;
    let code_len = read_uleb128(bytes, &mut offset)? as usize;
    let code_end = offset
        .checked_add(code_len)
        .ok_or(ModuleError::LengthOutOfBounds)?;
    if code_end > bytes.len() {
        return Err(ModuleError::LengthOutOfBounds);
    }
    let code = &bytes[offset..code_end];
    offset = code_end;

    let const_len = read_uleb128(bytes, &mut offset)? as usize;
    let const_end = offset
        .checked_add(const_len)
        .ok_or(ModuleError::LengthOutOfBounds)?;
    if const_end > bytes.len() {
        return Err(ModuleError::LengthOutOfBounds);
    }
    let const_pool = &bytes[offset..const_end];
    if const_end != bytes.len() {
        return Err(ModuleError::TrailingBytes);
    }

    Ok(Module {
        flags,
        max_stack,
        code,
        const_pool,
    })
}

pub fn validate(
    module: &Module<'_>,
    board_caps: CapabilitySet,
    board_max_stack: u8,
) -> Result<(), ValidateError> {
    if module.max_stack == 0 {
        return Err(ValidateError::MaxStackIsZero);
    }
    if module.max_stack > board_max_stack {
        return Err(ValidateError::DeclaredStackTooLarge);
    }

    let mut ip = 0;
    let mut depth: i16 = 0;
    while ip < module.code.len() {
        let instruction_start = ip;
        let (op, next_ip) = decode_next(module.code, ip).map_err(ValidateError::Decode)?;
        validate_stack_effect(op, instruction_start, &mut depth, module.max_stack)?;
        validate_capability(op, board_caps)?;
        validate_jump_target(module.code, op, next_ip)?;
        ip = next_ip;
    }
    Ok(())
}

fn validate_stack_effect(
    op: Op,
    instruction_start: usize,
    depth: &mut i16,
    max_stack: u8,
) -> Result<(), ValidateError> {
    let (pops, pushes) = stack_effect(op);
    if *depth < pops {
        return Err(ValidateError::StackUnderflow(instruction_start));
    }
    *depth = *depth - pops + pushes;
    if *depth > max_stack as i16 {
        return Err(ValidateError::StackOverflow(instruction_start));
    }
    Ok(())
}

fn validate_capability(op: Op, board_caps: CapabilitySet) -> Result<(), ValidateError> {
    let capability_id = match op {
        Op::CallU8(id) => id as u16,
        Op::CallU16(id) => id,
        _ => return Ok(()),
    };
    if board_caps.supports(capability_id) {
        Ok(())
    } else {
        Err(ValidateError::UnsupportedCapability(capability_id))
    }
}

fn validate_jump_target(code: &[u8], op: Op, next_ip: usize) -> Result<(), ValidateError> {
    let offset = match op {
        Op::JumpS8(offset) | Op::JumpIfFalseS8(offset) | Op::JumpIfTrueS8(offset) => offset,
        _ => return Ok(()),
    };
    let target = next_ip as isize + offset as isize;
    if target < 0 || target as usize > code.len() {
        return Err(ValidateError::JumpTargetOutOfBounds(target.max(0) as usize));
    }
    let target = target as usize;
    if target != code.len() && !is_instruction_boundary(code, target) {
        return Err(ValidateError::JumpTargetNotBoundary(target));
    }
    Ok(())
}

fn is_instruction_boundary(code: &[u8], target: usize) -> bool {
    let mut ip = 0;
    while ip < code.len() {
        if ip == target {
            return true;
        }
        match decode_next(code, ip) {
            Ok((_, next_ip)) => ip = next_ip,
            Err(_) => return false,
        }
    }
    target == code.len()
}

fn stack_effect(op: Op) -> (i16, i16) {
    match op {
        Op::Halt | Op::Nop | Op::JumpS8(_) => (0, 0),
        Op::PushFalse
        | Op::PushTrue
        | Op::PushU8(_)
        | Op::PushU16(_)
        | Op::PushU32(_)
        | Op::PushI16(_) => (0, 1),
        Op::Dup => (1, 2),
        Op::Drop => (1, 0),
        Op::Swap => (2, 2),
        Op::Over => (2, 3),
        Op::JumpIfFalseS8(_) | Op::JumpIfTrueS8(_) => (1, 0),
        Op::CallU8(CAP_GPIO_OPEN_U8) | Op::CallU16(CAP_GPIO_OPEN) => (2, 1),
        Op::CallU8(CAP_GPIO_WRITE_U8) | Op::CallU16(CAP_GPIO_WRITE) => (2, 0),
        Op::CallU8(CAP_GPIO_READ_U8) | Op::CallU16(CAP_GPIO_READ) => (1, 1),
        Op::CallU8(CAP_GPIO_CLOSE_U8) | Op::CallU16(CAP_GPIO_CLOSE) => (1, 0),
        Op::CallU8(CAP_TIME_SLEEP_MS_U8) | Op::CallU16(CAP_TIME_SLEEP_MS) => (1, 0),
        Op::CallU8(CAP_TIME_NOW_MS_U8) | Op::CallU16(CAP_TIME_NOW_MS) => (0, 1),
        Op::CallU8(_) | Op::CallU16(_) => (0, 0),
        Op::ReturnTop => (1, 0),
    }
}

fn read_u8(code: &[u8], offset: usize) -> Result<u8, DecodeError> {
    code.get(offset).copied().ok_or(DecodeError::UnexpectedEof)
}

fn read_i8(code: &[u8], offset: usize) -> Result<i8, DecodeError> {
    read_u8(code, offset).map(|value| value as i8)
}

fn read_u16(code: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let low = *code.get(offset).ok_or(DecodeError::UnexpectedEof)? as u16;
    let high = *code.get(offset + 1).ok_or(DecodeError::UnexpectedEof)? as u16;
    Ok(low | (high << 8))
}

fn read_u32(code: &[u8], offset: usize) -> Result<u32, DecodeError> {
    let b0 = *code.get(offset).ok_or(DecodeError::UnexpectedEof)? as u32;
    let b1 = *code.get(offset + 1).ok_or(DecodeError::UnexpectedEof)? as u32;
    let b2 = *code.get(offset + 2).ok_or(DecodeError::UnexpectedEof)? as u32;
    let b3 = *code.get(offset + 3).ok_or(DecodeError::UnexpectedEof)? as u32;
    Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
}

fn read_uleb128(bytes: &[u8], offset: &mut usize) -> Result<u32, ModuleError> {
    let mut result = 0u32;
    let mut shift = 0u32;
    loop {
        let byte = *bytes.get(*offset).ok_or(ModuleError::TruncatedUleb)?;
        *offset += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 32 {
            return Err(ModuleError::TruncatedUleb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLINK_CODE: &[u8] = &[
        0x12, 0x0d, 0x12, 0x01, 0x40, 0x01, 0x20, 0x11, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10,
        0x20, 0x10, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10, 0x30, 0xec,
    ];

    #[test]
    fn decodes_push_u16() {
        let (op, next) = decode_next(&[0x13, 0xfa, 0x00], 0).unwrap();
        assert_eq!(op, Op::PushU16(250));
        assert_eq!(next, 3);
    }

    #[test]
    fn parses_blink_module() {
        let module_bytes = [
            0x42, 0x56, 0x4d, 0x31, 0x01, 0x01, 0x04, 0x00, 0x1a, 0x12, 0x0d, 0x12, 0x01, 0x40,
            0x01, 0x20, 0x11, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10, 0x20, 0x10, 0x40, 0x02,
            0x13, 0xfa, 0x00, 0x40, 0x10, 0x30, 0xec, 0x00,
        ];
        let module = parse_module(&module_bytes).unwrap();
        assert_eq!(module.flags, FLAG_PROGRAM_MAY_RUN_FOREVER);
        assert_eq!(module.max_stack, 4);
        assert_eq!(module.code, BLINK_CODE);
        assert!(module.const_pool.is_empty());
    }

    #[test]
    fn validates_blink_code() {
        let module = Module {
            flags: FLAG_PROGRAM_MAY_RUN_FOREVER,
            max_stack: 4,
            code: BLINK_CODE,
            const_pool: &[],
        };
        validate(&module, CapabilitySet::blink_mvp(), 8).unwrap();
    }

    #[test]
    fn rejects_jump_into_operand() {
        let code = [0x12, 0x01, 0x30, 0xfd];
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &code,
            const_pool: &[],
        };
        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 8),
            Err(ValidateError::JumpTargetNotBoundary(1))
        );
    }

    #[test]
    fn rejects_missing_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x13, 0x01, 0x00, 0x40, 0x10],
            const_pool: &[],
        };
        assert_eq!(
            validate(&module, CapabilitySet::empty(), 8),
            Err(ValidateError::UnsupportedCapability(CAP_TIME_SLEEP_MS))
        );
    }
}
