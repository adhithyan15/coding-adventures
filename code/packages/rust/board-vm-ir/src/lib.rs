#![no_std]

pub const MODULE_MAGIC: [u8; 4] = *b"BVM1";
pub const MODULE_VERSION: u8 = 1;
pub const MAX_BYTE_BUFFER_LEN: usize = 32;

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
pub const CAP_PWM_WRITE: u16 = 0x20;
pub const CAP_ADC_READ: u16 = 0x21;
pub const CAP_DAC_WRITE_U12: u16 = 0x22;
pub const CAP_I2C_OPEN: u16 = 0x23;
pub const CAP_I2C_WRITE_U8: u16 = 0x24;
pub const CAP_I2C_READ_U8: u16 = 0x25;
pub const CAP_I2C_WRITE: u16 = 0x26;
pub const CAP_I2C_READ: u16 = 0x27;
pub const CAP_I2C_TRANSFER: u16 = 0x28;
pub const CAP_SPI_OPEN: u16 = 0x29;
pub const CAP_SPI_TRANSFER: u16 = 0x2A;
pub const CAP_UART_OPEN: u16 = 0x2B;
pub const CAP_LED_MATRIX_FRAME: u16 = 0x30;

const CAP_GPIO_OPEN_U8: u8 = CAP_GPIO_OPEN as u8;
const CAP_GPIO_WRITE_U8: u8 = CAP_GPIO_WRITE as u8;
const CAP_GPIO_READ_U8: u8 = CAP_GPIO_READ as u8;
const CAP_GPIO_CLOSE_U8: u8 = CAP_GPIO_CLOSE as u8;
const CAP_TIME_SLEEP_MS_U8: u8 = CAP_TIME_SLEEP_MS as u8;
const CAP_TIME_NOW_MS_U8: u8 = CAP_TIME_NOW_MS as u8;
const CAP_PWM_WRITE_U8: u8 = CAP_PWM_WRITE as u8;
const CAP_ADC_READ_U8: u8 = CAP_ADC_READ as u8;
const CAP_DAC_WRITE_U12_U8: u8 = CAP_DAC_WRITE_U12 as u8;
const CAP_I2C_OPEN_U8: u8 = CAP_I2C_OPEN as u8;
const CAP_I2C_WRITE_U8_U8: u8 = CAP_I2C_WRITE_U8 as u8;
const CAP_I2C_READ_U8_U8: u8 = CAP_I2C_READ_U8 as u8;
const CAP_I2C_WRITE_CAP_U8: u8 = CAP_I2C_WRITE as u8;
const CAP_I2C_READ_CAP_U8: u8 = CAP_I2C_READ as u8;
const CAP_I2C_TRANSFER_U8: u8 = CAP_I2C_TRANSFER as u8;
const CAP_SPI_OPEN_U8: u8 = CAP_SPI_OPEN as u8;
const CAP_SPI_TRANSFER_U8: u8 = CAP_SPI_TRANSFER as u8;
const CAP_UART_OPEN_U8: u8 = CAP_UART_OPEN as u8;
const CAP_LED_MATRIX_FRAME_U8: u8 = CAP_LED_MATRIX_FRAME as u8;

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
    PushBytes { offset: u16, len: u8 },
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
    ConstPoolOutOfBounds(usize),
    ByteBufferTooLarge(usize),
    UnsupportedCapability(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredCapabilitiesError {
    Decode(DecodeError),
    OutputTooSmall,
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
    pub pwm: bool,
    pub adc: bool,
    pub dac: bool,
    pub i2c: bool,
    pub spi: bool,
    pub uart: bool,
    pub led_matrix: bool,
}

impl CapabilitySet {
    pub const fn empty() -> Self {
        Self {
            gpio_digital: false,
            time: false,
            pwm: false,
            adc: false,
            dac: false,
            i2c: false,
            spi: false,
            uart: false,
            led_matrix: false,
        }
    }

    pub const fn blink_mvp() -> Self {
        Self {
            gpio_digital: true,
            time: true,
            pwm: false,
            adc: false,
            dac: false,
            i2c: false,
            spi: false,
            uart: false,
            led_matrix: false,
        }
    }

    pub const fn with_pwm(self) -> Self {
        Self { pwm: true, ..self }
    }

    pub const fn with_adc(self) -> Self {
        Self { adc: true, ..self }
    }

    pub const fn with_dac(self) -> Self {
        Self { dac: true, ..self }
    }

    pub const fn with_i2c(self) -> Self {
        Self { i2c: true, ..self }
    }

    pub const fn with_spi(self) -> Self {
        Self { spi: true, ..self }
    }

    pub const fn with_uart(self) -> Self {
        Self { uart: true, ..self }
    }

    pub const fn with_led_matrix(self) -> Self {
        Self {
            led_matrix: true,
            ..self
        }
    }

    pub const fn supports(self, capability_id: u16) -> bool {
        match capability_id {
            CAP_GPIO_OPEN | CAP_GPIO_WRITE | CAP_GPIO_READ | CAP_GPIO_CLOSE => self.gpio_digital,
            CAP_TIME_SLEEP_MS | CAP_TIME_NOW_MS => self.time,
            CAP_PWM_WRITE => self.pwm,
            CAP_ADC_READ => self.adc,
            CAP_DAC_WRITE_U12 => self.dac,
            CAP_I2C_OPEN | CAP_I2C_WRITE_U8 | CAP_I2C_READ_U8 | CAP_I2C_WRITE | CAP_I2C_READ
            | CAP_I2C_TRANSFER => self.i2c,
            CAP_SPI_OPEN | CAP_SPI_TRANSFER => self.spi,
            CAP_UART_OPEN => self.uart,
            CAP_LED_MATRIX_FRAME => self.led_matrix,
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
        0x16 => Ok((
            Op::PushBytes {
                offset: read_u16(code, next)?,
                len: read_u8(code, next + 2)?,
            },
            next + 3,
        )),
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
    let mut depth: i16 = if module.flags & FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES != 0 {
        1
    } else {
        0
    };
    while ip < module.code.len() {
        let instruction_start = ip;
        let (op, next_ip) = decode_next(module.code, ip).map_err(ValidateError::Decode)?;
        validate_stack_effect(op, instruction_start, &mut depth, module.max_stack)?;
        validate_capability(op, board_caps)?;
        validate_const_pool_access(module.const_pool, op, instruction_start)?;
        validate_jump_target(module.code, op, next_ip)?;
        ip = next_ip;
    }
    Ok(())
}

pub fn collect_required_capabilities(
    module: &Module<'_>,
    out: &mut [u16],
) -> Result<usize, RequiredCapabilitiesError> {
    let mut ip = 0;
    let mut count = 0;
    while ip < module.code.len() {
        let (op, next_ip) =
            decode_next(module.code, ip).map_err(RequiredCapabilitiesError::Decode)?;
        if let Some(capability_id) = called_capability(op) {
            count = push_unique_capability(out, count, capability_id)?;
        }
        ip = next_ip;
    }
    Ok(count)
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
    let Some(capability_id) = called_capability(op) else {
        return Ok(());
    };
    if board_caps.supports(capability_id) {
        Ok(())
    } else {
        Err(ValidateError::UnsupportedCapability(capability_id))
    }
}

fn validate_const_pool_access(
    const_pool: &[u8],
    op: Op,
    instruction_start: usize,
) -> Result<(), ValidateError> {
    let Op::PushBytes { offset, len } = op else {
        return Ok(());
    };
    if len as usize > MAX_BYTE_BUFFER_LEN {
        return Err(ValidateError::ByteBufferTooLarge(instruction_start));
    }
    let offset = offset as usize;
    let end = offset
        .checked_add(len as usize)
        .ok_or(ValidateError::ConstPoolOutOfBounds(instruction_start))?;
    if end > const_pool.len() {
        return Err(ValidateError::ConstPoolOutOfBounds(instruction_start));
    }
    Ok(())
}

pub fn called_capability(op: Op) -> Option<u16> {
    match op {
        Op::CallU8(id) => Some(id as u16),
        Op::CallU16(id) => Some(id),
        _ => None,
    }
}

fn push_unique_capability(
    out: &mut [u16],
    count: usize,
    capability_id: u16,
) -> Result<usize, RequiredCapabilitiesError> {
    if out[..count]
        .iter()
        .any(|existing| *existing == capability_id)
    {
        return Ok(count);
    }
    let Some(slot) = out.get_mut(count) else {
        return Err(RequiredCapabilitiesError::OutputTooSmall);
    };
    *slot = capability_id;
    Ok(count + 1)
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
        | Op::PushI16(_)
        | Op::PushBytes { .. } => (0, 1),
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
        Op::CallU8(CAP_PWM_WRITE_U8) | Op::CallU16(CAP_PWM_WRITE) => (2, 0),
        Op::CallU8(CAP_ADC_READ_U8) | Op::CallU16(CAP_ADC_READ) => (1, 1),
        Op::CallU8(CAP_DAC_WRITE_U12_U8) | Op::CallU16(CAP_DAC_WRITE_U12) => (2, 0),
        Op::CallU8(CAP_I2C_OPEN_U8) | Op::CallU16(CAP_I2C_OPEN) => (1, 1),
        Op::CallU8(CAP_I2C_WRITE_U8_U8) | Op::CallU16(CAP_I2C_WRITE_U8) => (3, 0),
        Op::CallU8(CAP_I2C_READ_U8_U8) | Op::CallU16(CAP_I2C_READ_U8) => (2, 1),
        Op::CallU8(CAP_I2C_WRITE_CAP_U8) | Op::CallU16(CAP_I2C_WRITE) => (3, 0),
        Op::CallU8(CAP_I2C_READ_CAP_U8) | Op::CallU16(CAP_I2C_READ) => (3, 1),
        Op::CallU8(CAP_I2C_TRANSFER_U8) | Op::CallU16(CAP_I2C_TRANSFER) => (4, 1),
        Op::CallU8(CAP_SPI_OPEN_U8) | Op::CallU16(CAP_SPI_OPEN) => (1, 1),
        Op::CallU8(CAP_SPI_TRANSFER_U8) | Op::CallU16(CAP_SPI_TRANSFER) => (4, 1),
        Op::CallU8(CAP_UART_OPEN_U8) | Op::CallU16(CAP_UART_OPEN) => (1, 1),
        Op::CallU8(CAP_LED_MATRIX_FRAME_U8) | Op::CallU16(CAP_LED_MATRIX_FRAME) => (3, 0),
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
    fn decodes_push_bytes() {
        let (op, next) = decode_next(&[0x16, 0x34, 0x12, 0x03], 0).unwrap();
        assert_eq!(
            op,
            Op::PushBytes {
                offset: 0x1234,
                len: 3
            }
        );
        assert_eq!(next, 4);
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
    fn validates_push_bytes_const_pool_slice() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x16, 0x01, 0x00, 0x02, 0x50],
            const_pool: &[0xAA, 0xBE, 0xEF],
        };

        validate(&module, CapabilitySet::empty(), 8).unwrap();
    }

    #[test]
    fn rejects_push_bytes_const_pool_out_of_bounds() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x16, 0x02, 0x00, 0x02],
            const_pool: &[0xAA],
        };

        assert_eq!(
            validate(&module, CapabilitySet::empty(), 8),
            Err(ValidateError::ConstPoolOutOfBounds(0))
        );
    }

    #[test]
    fn rejects_push_bytes_larger_than_vm_buffer() {
        let const_pool = [0u8; MAX_BYTE_BUFFER_LEN + 1];
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x16, 0x00, 0x00, (MAX_BYTE_BUFFER_LEN + 1) as u8],
            const_pool: &const_pool,
        };

        assert_eq!(
            validate(&module, CapabilitySet::empty(), 8),
            Err(ValidateError::ByteBufferTooLarge(0))
        );
    }

    #[test]
    fn collects_unique_required_capabilities() {
        let module = Module {
            flags: FLAG_PROGRAM_MAY_RUN_FOREVER,
            max_stack: 4,
            code: BLINK_CODE,
            const_pool: &[],
        };
        let mut capabilities = [0u16; 4];

        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();

        assert_eq!(count, 3);
        assert_eq!(
            &capabilities[..count],
            &[CAP_GPIO_OPEN, CAP_GPIO_WRITE, CAP_TIME_SLEEP_MS]
        );
    }

    #[test]
    fn collects_call_u16_capabilities() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x41, 0x34, 0x12, 0x41, 0x34, 0x12],
            const_pool: &[],
        };
        let mut capabilities = [0u16; 2];

        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();

        assert_eq!(count, 1);
        assert_eq!(capabilities[0], 0x1234);
    }

    #[test]
    fn validates_led_matrix_frame_capability() {
        let module = Module {
            flags: 0,
            max_stack: 3,
            code: &[
                0x14,
                0x44,
                0xA4,
                0x84,
                0x31,
                0x14,
                0x81,
                0x20,
                0x04,
                0x44,
                0x14,
                0x40,
                0x00,
                0x0A,
                0x10,
                0x40,
                CAP_LED_MATRIX_FRAME as u8,
            ],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_led_matrix(), 3).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_LED_MATRIX_FRAME]);
    }

    #[test]
    fn validates_pwm_write_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 3, 0x13, 0x00, 0x80, 0x40, CAP_PWM_WRITE as u8],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_pwm(), 2).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_PWM_WRITE]);
    }

    #[test]
    fn validates_adc_read_capability() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x12, 14, 0x40, CAP_ADC_READ as u8, 0x50],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_adc(), 1).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_ADC_READ]);
    }

    #[test]
    fn validates_dac_write_u12_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 14, 0x13, 0x00, 0x08, 0x40, CAP_DAC_WRITE_U12 as u8],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_dac(), 2).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_DAC_WRITE_U12]);
    }

    #[test]
    fn validates_i2c_open_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 0, 0x40, CAP_I2C_OPEN as u8, 0x50],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_i2c(), 2).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_I2C_OPEN]);
    }

    #[test]
    fn validates_i2c_write_u8_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 4,
            code: &[0x20, 0x12, 0x3c, 0x12, 0xa5, 0x40, CAP_I2C_WRITE_U8 as u8],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_i2c(), 4).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_I2C_WRITE_U8]);
    }

    #[test]
    fn validates_i2c_read_u8_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 3,
            code: &[0x20, 0x12, 0x3c, 0x40, CAP_I2C_READ_U8 as u8, 0x50],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_i2c(), 4).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_I2C_READ_U8]);
    }

    #[test]
    fn validates_i2c_write_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 4,
            code: &[
                0x20,
                0x12,
                0x3c,
                0x16,
                0x00,
                0x00,
                0x03,
                0x40,
                CAP_I2C_WRITE as u8,
            ],
            const_pool: &[0xde, 0xad, 0xbe],
        };

        validate(&module, CapabilitySet::blink_mvp().with_i2c(), 4).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_I2C_WRITE]);
    }

    #[test]
    fn validates_i2c_read_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 4,
            code: &[0x20, 0x12, 0x3c, 0x12, 0x03, 0x40, CAP_I2C_READ as u8, 0x50],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_i2c(), 4).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_I2C_READ]);
    }

    #[test]
    fn validates_spi_open_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 0, 0x40, CAP_SPI_OPEN as u8, 0x50],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_spi(), 2).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_SPI_OPEN]);
    }

    #[test]
    fn validates_uart_open_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 0, 0x40, CAP_UART_OPEN as u8, 0x50],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp().with_uart(), 2).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_UART_OPEN]);
    }

    #[test]
    fn validates_spi_transfer_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 5,
            code: &[
                0x20,
                0x13,
                0x0a,
                0x00,
                0x16,
                0x00,
                0x00,
                0x01,
                0x12,
                0x03,
                0x40,
                CAP_SPI_TRANSFER as u8,
                0x50,
            ],
            const_pool: &[0x9f],
        };

        validate(&module, CapabilitySet::blink_mvp().with_spi(), 5).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&module, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_SPI_TRANSFER]);
    }

    #[test]
    fn rejects_pwm_write_without_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 3, 0x13, 0x00, 0x80, 0x40, CAP_PWM_WRITE as u8],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 2),
            Err(ValidateError::UnsupportedCapability(CAP_PWM_WRITE))
        );
    }

    #[test]
    fn rejects_adc_read_without_capability() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x12, 14, 0x40, CAP_ADC_READ as u8, 0x50],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 1),
            Err(ValidateError::UnsupportedCapability(CAP_ADC_READ))
        );
    }

    #[test]
    fn rejects_dac_write_u12_without_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 14, 0x13, 0x00, 0x08, 0x40, CAP_DAC_WRITE_U12 as u8],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 2),
            Err(ValidateError::UnsupportedCapability(CAP_DAC_WRITE_U12))
        );
    }

    #[test]
    fn rejects_i2c_open_without_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 0, 0x40, CAP_I2C_OPEN as u8, 0x50],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 2),
            Err(ValidateError::UnsupportedCapability(CAP_I2C_OPEN))
        );
    }

    #[test]
    fn rejects_spi_open_without_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 0, 0x40, CAP_SPI_OPEN as u8, 0x50],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 2),
            Err(ValidateError::UnsupportedCapability(CAP_SPI_OPEN))
        );
    }

    #[test]
    fn rejects_uart_open_without_capability() {
        let module = Module {
            flags: 0,
            max_stack: 2,
            code: &[0x12, 0, 0x40, CAP_UART_OPEN as u8, 0x50],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 2),
            Err(ValidateError::UnsupportedCapability(CAP_UART_OPEN))
        );
    }

    #[test]
    fn rejects_spi_transfer_without_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 5,
            code: &[
                0x20,
                0x13,
                0x0a,
                0x00,
                0x16,
                0x00,
                0x00,
                0x01,
                0x12,
                0x03,
                0x40,
                CAP_SPI_TRANSFER as u8,
                0x50,
            ],
            const_pool: &[0x9f],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 5),
            Err(ValidateError::UnsupportedCapability(CAP_SPI_TRANSFER))
        );
    }

    #[test]
    fn rejects_i2c_write_u8_without_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 4,
            code: &[0x20, 0x12, 0x3c, 0x12, 0xa5, 0x40, CAP_I2C_WRITE_U8 as u8],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 4),
            Err(ValidateError::UnsupportedCapability(CAP_I2C_WRITE_U8))
        );
    }

    #[test]
    fn rejects_i2c_read_u8_without_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 3,
            code: &[0x20, 0x12, 0x3c, 0x40, CAP_I2C_READ_U8 as u8, 0x50],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 4),
            Err(ValidateError::UnsupportedCapability(CAP_I2C_READ_U8))
        );
    }

    #[test]
    fn rejects_i2c_write_without_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 4,
            code: &[
                0x20,
                0x12,
                0x3c,
                0x16,
                0x00,
                0x00,
                0x03,
                0x40,
                CAP_I2C_WRITE as u8,
            ],
            const_pool: &[0xde, 0xad, 0xbe],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 4),
            Err(ValidateError::UnsupportedCapability(CAP_I2C_WRITE))
        );
    }

    #[test]
    fn rejects_i2c_read_without_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 4,
            code: &[0x20, 0x12, 0x3c, 0x12, 0x03, 0x40, CAP_I2C_READ as u8, 0x50],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 4),
            Err(ValidateError::UnsupportedCapability(CAP_I2C_READ))
        );
    }

    #[test]
    fn rejects_i2c_transfer_without_capability() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 5,
            code: &[
                0x20,
                0x12,
                0x3c,
                0x16,
                0x00,
                0x00,
                0x02,
                0x12,
                0x03,
                0x40,
                CAP_I2C_TRANSFER as u8,
                0x50,
            ],
            const_pool: &[0x00, 0x10],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 5),
            Err(ValidateError::UnsupportedCapability(CAP_I2C_TRANSFER))
        );
    }

    #[test]
    fn rejects_led_matrix_frame_without_capability() {
        let module = Module {
            flags: 0,
            max_stack: 3,
            code: &[
                0x14,
                0,
                0,
                0,
                0,
                0x14,
                0,
                0,
                0,
                0,
                0x14,
                0,
                0,
                0,
                0,
                0x40,
                CAP_LED_MATRIX_FRAME as u8,
            ],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 3),
            Err(ValidateError::UnsupportedCapability(CAP_LED_MATRIX_FRAME))
        );
    }

    #[test]
    fn persistent_handle_modules_validate_with_existing_stack_handle() {
        let module = Module {
            flags: FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            max_stack: 1,
            code: &[0x40, CAP_GPIO_CLOSE as u8],
            const_pool: &[],
        };

        validate(&module, CapabilitySet::blink_mvp(), 8).unwrap();
    }

    #[test]
    fn handle_stack_modules_without_persistent_flag_still_underflow() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x40, CAP_GPIO_CLOSE as u8],
            const_pool: &[],
        };

        assert_eq!(
            validate(&module, CapabilitySet::blink_mvp(), 8),
            Err(ValidateError::StackUnderflow(0))
        );
    }

    #[test]
    fn rejects_capability_output_overflow() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x40, 0x01, 0x40, 0x02],
            const_pool: &[],
        };
        let mut capabilities = [0u16; 1];

        assert_eq!(
            collect_required_capabilities(&module, &mut capabilities),
            Err(RequiredCapabilitiesError::OutputTooSmall)
        );
    }

    #[test]
    fn reports_decode_error_while_collecting_capabilities() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x41, 0x01],
            const_pool: &[],
        };
        let mut capabilities = [0u16; 1];

        assert_eq!(
            collect_required_capabilities(&module, &mut capabilities),
            Err(RequiredCapabilitiesError::Decode(
                DecodeError::UnexpectedEof
            ))
        );
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
