#![no_std]

use board_vm_ir::{
    decode_next, validate, CapabilitySet, Module, Op, CAP_ADC_READ, CAP_DAC_WRITE_U12,
    CAP_GPIO_CLOSE, CAP_GPIO_OPEN, CAP_GPIO_READ, CAP_GPIO_WRITE, CAP_I2C_OPEN, CAP_I2C_READ,
    CAP_I2C_READ_U8, CAP_I2C_TRANSFER, CAP_I2C_WRITE, CAP_I2C_WRITE_U8, CAP_LED_MATRIX_FRAME,
    CAP_PWM_WRITE, CAP_SPI_OPEN, CAP_SPI_TRANSFER, CAP_TIME_NOW_MS, CAP_TIME_SLEEP_MS,
    CAP_UART_OPEN, MAX_BYTE_BUFFER_LEN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteBuffer {
    len: u8,
    bytes: [u8; MAX_BYTE_BUFFER_LEN],
}

impl ByteBuffer {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; MAX_BYTE_BUFFER_LEN],
        }
    }

    pub fn from_slice(value: &[u8]) -> Result<Self, ByteBufferError> {
        if value.len() > MAX_BYTE_BUFFER_LEN {
            return Err(ByteBufferError);
        }
        let mut bytes = [0; MAX_BYTE_BUFFER_LEN];
        bytes[..value.len()].copy_from_slice(value);
        Ok(Self {
            len: value.len() as u8,
            bytes,
        })
    }

    pub const fn len(&self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }
}

impl Default for ByteBuffer {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteBufferError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Unit,
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    I16(i16),
    Handle(Handle),
    Bytes(ByteBuffer),
}

impl Default for Value {
    fn default() -> Self {
        Self::Unit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Handle {
    pub index: u8,
    pub generation: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioMode {
    Input,
    Output,
    InputPullup,
    InputPulldown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Low,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalError {
    InvalidPin,
    UnsupportedMode,
    ResourceBusy,
    BoardFault,
}

pub trait BoardHal {
    fn capabilities(&self) -> CapabilitySet;

    fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError>;
    fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError>;
    fn gpio_read(&mut self, token: u32) -> Result<Level, HalError>;
    fn gpio_close(&mut self, token: u32) -> Result<(), HalError>;

    fn sleep_ms(&mut self, duration_ms: u16) -> Result<(), HalError>;
    fn now_ms(&self) -> u32;

    fn pwm_write(&mut self, _pin: u16, _duty: u16) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn adc_read(&mut self, _pin: u16) -> Result<u16, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn dac_write_u12(&mut self, _pin: u16, _sample: u16) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn i2c_open(&mut self, _bus: u16) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn i2c_write_u8(&mut self, _token: u32, _address: u16, _byte: u8) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn i2c_write(&mut self, _token: u32, _address: u16, _bytes: &[u8]) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn i2c_read_u8(&mut self, _token: u32, _address: u16) -> Result<u8, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn i2c_read(&mut self, _token: u32, _address: u16, _len: u8) -> Result<ByteBuffer, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn i2c_transfer(
        &mut self,
        _token: u32,
        _address: u16,
        _write_bytes: &[u8],
        _read_len: u8,
    ) -> Result<ByteBuffer, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn spi_open(&mut self, _bus: u16) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn spi_transfer(
        &mut self,
        _token: u32,
        _cs_pin: u16,
        _write_bytes: &[u8],
        _read_len: u8,
    ) -> Result<ByteBuffer, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn uart_open(&mut self, _bus: u16) -> Result<u32, HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn led_matrix_frame(&mut self, _frame: [u32; 3]) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }

    fn supports_bootloader_reboot(&self) -> bool {
        false
    }

    fn reboot_to_bootloader(&mut self) -> Result<(), HalError> {
        Err(HalError::UnsupportedMode)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Halted,
    BudgetExceeded,
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunReport {
    pub status: RunStatus,
    pub instructions_executed: u32,
    pub return_value: Value,
    pub stack_depth: u8,
    pub open_handles: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunCursor {
    pub ip: usize,
    pub instructions_executed: u32,
    pub return_value: Value,
}

impl RunCursor {
    pub const fn new() -> Self {
        Self {
            ip: 0,
            instructions_executed: 0,
            return_value: Value::Unit,
        }
    }
}

impl Default for RunCursor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeError {
    pub ip: usize,
    pub kind: RuntimeErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    InvalidBytecode,
    ValidationFailed,
    StackOverflow,
    StackUnderflow,
    TypeMismatch,
    UnsupportedCapability,
    HandleNotFound,
    ResourceBusy,
    InvalidPin,
    UnsupportedMode,
    BoardFault,
    ByteBufferTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandleKind {
    Empty,
    Gpio,
    I2c,
    Spi,
    Uart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HandleSlot {
    generation: u8,
    kind: HandleKind,
    token: u32,
    open: bool,
}

impl Default for HandleSlot {
    fn default() -> Self {
        Self {
            generation: 0,
            kind: HandleKind::Empty,
            token: 0,
            open: false,
        }
    }
}

pub struct Runtime<H, const MAX_STACK: usize, const MAX_HANDLES: usize>
where
    H: BoardHal,
{
    hal: H,
    stack: [Value; MAX_STACK],
    stack_len: usize,
    handles: [HandleSlot; MAX_HANDLES],
}

impl<H, const MAX_STACK: usize, const MAX_HANDLES: usize> Runtime<H, MAX_STACK, MAX_HANDLES>
where
    H: BoardHal,
{
    pub fn new(hal: H) -> Self {
        Self {
            hal,
            stack: [Value::Unit; MAX_STACK],
            stack_len: 0,
            handles: [HandleSlot::default(); MAX_HANDLES],
        }
    }

    pub fn hal(&self) -> &H {
        &self.hal
    }

    pub fn hal_mut(&mut self) -> &mut H {
        &mut self.hal
    }

    pub fn reset_vm(&mut self) {
        self.stack_len = 0;
        for slot in &mut self.handles {
            if slot.open {
                slot.generation = slot.generation.wrapping_add(1).max(1);
            }
            *slot = HandleSlot {
                generation: slot.generation,
                ..HandleSlot::default()
            };
        }
    }

    pub fn run_module(
        &mut self,
        module: &Module<'_>,
        instruction_budget: u32,
    ) -> Result<RunReport, RuntimeError> {
        validate(module, self.hal.capabilities(), MAX_STACK as u8).map_err(|_| RuntimeError {
            ip: 0,
            kind: RuntimeErrorKind::ValidationFailed,
        })?;
        let mut cursor = RunCursor::new();
        self.run_code_slice_with_const_pool(
            module.code,
            module.const_pool,
            &mut cursor,
            instruction_budget,
        )
    }

    pub fn run_code(
        &mut self,
        code: &[u8],
        instruction_budget: u32,
    ) -> Result<RunReport, RuntimeError> {
        let mut cursor = RunCursor::new();
        self.run_code_slice(code, &mut cursor, instruction_budget)
    }

    pub fn run_module_slice(
        &mut self,
        module: &Module<'_>,
        cursor: &mut RunCursor,
        instruction_budget: u32,
    ) -> Result<RunReport, RuntimeError> {
        validate(module, self.hal.capabilities(), MAX_STACK as u8).map_err(|_| RuntimeError {
            ip: 0,
            kind: RuntimeErrorKind::ValidationFailed,
        })?;
        self.run_code_slice_with_const_pool(
            module.code,
            module.const_pool,
            cursor,
            instruction_budget,
        )
    }

    pub fn run_code_slice(
        &mut self,
        code: &[u8],
        cursor: &mut RunCursor,
        instruction_budget: u32,
    ) -> Result<RunReport, RuntimeError> {
        self.run_code_slice_with_const_pool(code, &[], cursor, instruction_budget)
    }

    fn run_code_slice_with_const_pool(
        &mut self,
        code: &[u8],
        const_pool: &[u8],
        cursor: &mut RunCursor,
        instruction_budget: u32,
    ) -> Result<RunReport, RuntimeError> {
        let slice_start = cursor.instructions_executed;

        while cursor.ip < code.len() {
            if cursor.instructions_executed.saturating_sub(slice_start) >= instruction_budget {
                return Ok(self.report(
                    RunStatus::BudgetExceeded,
                    cursor.instructions_executed,
                    cursor.return_value,
                ));
            }

            let instruction_ip = cursor.ip;
            let (op, next_ip) = decode_next(code, cursor.ip).map_err(|_| RuntimeError {
                ip: instruction_ip,
                kind: RuntimeErrorKind::InvalidBytecode,
            })?;
            cursor.ip = next_ip;
            cursor.instructions_executed += 1;

            match op {
                Op::Halt => {
                    return Ok(self.report(
                        RunStatus::Halted,
                        cursor.instructions_executed,
                        cursor.return_value,
                    ));
                }
                Op::Nop => {}
                Op::PushFalse => self.push(Value::Bool(false), instruction_ip)?,
                Op::PushTrue => self.push(Value::Bool(true), instruction_ip)?,
                Op::PushU8(value) => self.push(Value::U8(value), instruction_ip)?,
                Op::PushU16(value) => self.push(Value::U16(value), instruction_ip)?,
                Op::PushU32(value) => self.push(Value::U32(value), instruction_ip)?,
                Op::PushI16(value) => self.push(Value::I16(value), instruction_ip)?,
                Op::PushBytes { offset, len } => {
                    let start = offset as usize;
                    let end = start.checked_add(len as usize).ok_or(RuntimeError {
                        ip: instruction_ip,
                        kind: RuntimeErrorKind::InvalidBytecode,
                    })?;
                    let Some(bytes) = const_pool.get(start..end) else {
                        return Err(RuntimeError {
                            ip: instruction_ip,
                            kind: RuntimeErrorKind::InvalidBytecode,
                        });
                    };
                    let buffer = ByteBuffer::from_slice(bytes).map_err(|_| RuntimeError {
                        ip: instruction_ip,
                        kind: RuntimeErrorKind::ByteBufferTooLarge,
                    })?;
                    self.push(Value::Bytes(buffer), instruction_ip)?;
                }
                Op::Dup => {
                    let value = self.peek(instruction_ip)?;
                    self.push(value, instruction_ip)?;
                }
                Op::Drop => {
                    self.pop(instruction_ip)?;
                }
                Op::Swap => self.swap(instruction_ip)?,
                Op::Over => {
                    if self.stack_len < 2 {
                        return Err(RuntimeError {
                            ip: instruction_ip,
                            kind: RuntimeErrorKind::StackUnderflow,
                        });
                    }
                    let value = self.stack[self.stack_len - 2];
                    self.push(value, instruction_ip)?;
                }
                Op::JumpS8(offset) => {
                    cursor.ip = jump_target(cursor.ip, offset, instruction_ip)?;
                }
                Op::JumpIfFalseS8(offset) => {
                    if !self.pop_bool(instruction_ip)? {
                        cursor.ip = jump_target(cursor.ip, offset, instruction_ip)?;
                    }
                }
                Op::JumpIfTrueS8(offset) => {
                    if self.pop_bool(instruction_ip)? {
                        cursor.ip = jump_target(cursor.ip, offset, instruction_ip)?;
                    }
                }
                Op::CallU8(capability_id) => self.call(capability_id as u16, instruction_ip)?,
                Op::CallU16(capability_id) => self.call(capability_id, instruction_ip)?,
                Op::ReturnTop => {
                    cursor.return_value = self.pop(instruction_ip)?;
                    return Ok(self.report(
                        RunStatus::Halted,
                        cursor.instructions_executed,
                        cursor.return_value,
                    ));
                }
            }
        }

        Ok(self.report(
            RunStatus::Halted,
            cursor.instructions_executed,
            cursor.return_value,
        ))
    }

    fn report(
        &self,
        status: RunStatus,
        instructions_executed: u32,
        return_value: Value,
    ) -> RunReport {
        RunReport {
            status,
            instructions_executed,
            return_value,
            stack_depth: self.stack_len as u8,
            open_handles: self.open_handle_count(),
        }
    }

    fn call(&mut self, capability_id: u16, ip: usize) -> Result<(), RuntimeError> {
        match capability_id {
            CAP_GPIO_OPEN => {
                let mode = self.pop_gpio_mode(ip)?;
                let pin = self.pop_u16(ip)?;
                let token = self.hal.gpio_open(pin, mode).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })?;
                let handle = self.alloc_handle(HandleKind::Gpio, token, ip)?;
                self.push(Value::Handle(handle), ip)
            }
            CAP_GPIO_WRITE => {
                let level = if self.pop_bool(ip)? {
                    Level::High
                } else {
                    Level::Low
                };
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::Gpio, ip)?;
                self.hal
                    .gpio_write(token, level)
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })
            }
            CAP_GPIO_READ => {
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::Gpio, ip)?;
                let level = self.hal.gpio_read(token).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })?;
                self.push(Value::Bool(level == Level::High), ip)
            }
            CAP_GPIO_CLOSE => {
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::Gpio, ip)?;
                self.hal.gpio_close(token).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })?;
                self.close_handle(handle, ip)
            }
            CAP_TIME_SLEEP_MS => {
                let duration_ms = self.pop_u16(ip)?;
                self.hal.sleep_ms(duration_ms).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })
            }
            CAP_TIME_NOW_MS => {
                let now = self.hal.now_ms();
                self.push(Value::U32(now), ip)
            }
            CAP_PWM_WRITE => {
                let duty = self.pop_u16(ip)?;
                let pin = self.pop_u16(ip)?;
                self.hal.pwm_write(pin, duty).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })
            }
            CAP_ADC_READ => {
                let pin = self.pop_u16(ip)?;
                let sample = self.hal.adc_read(pin).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })?;
                self.push(Value::U16(sample), ip)
            }
            CAP_DAC_WRITE_U12 => {
                let sample = self.pop_u16(ip)?;
                let pin = self.pop_u16(ip)?;
                self.hal
                    .dac_write_u12(pin, sample)
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })
            }
            CAP_I2C_OPEN => {
                let bus = self.pop_u16(ip)?;
                let token = self.hal.i2c_open(bus).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })?;
                let handle = self.alloc_handle(HandleKind::I2c, token, ip)?;
                self.push(Value::Handle(handle), ip)
            }
            CAP_I2C_WRITE_U8 => {
                let byte = self.pop_u8(ip)?;
                let address = self.pop_u16(ip)?;
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::I2c, ip)?;
                self.hal
                    .i2c_write_u8(token, address, byte)
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })
            }
            CAP_I2C_WRITE => {
                let bytes = self.pop_bytes(ip)?;
                let address = self.pop_u16(ip)?;
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::I2c, ip)?;
                self.hal
                    .i2c_write(token, address, bytes.as_slice())
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })
            }
            CAP_I2C_READ_U8 => {
                let address = self.pop_u16(ip)?;
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::I2c, ip)?;
                let byte = self
                    .hal
                    .i2c_read_u8(token, address)
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })?;
                self.push(Value::U8(byte), ip)
            }
            CAP_I2C_READ => {
                let len = self.pop_u8(ip)?;
                if len as usize > MAX_BYTE_BUFFER_LEN {
                    return Err(RuntimeError {
                        ip,
                        kind: RuntimeErrorKind::ByteBufferTooLarge,
                    });
                }
                let address = self.pop_u16(ip)?;
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::I2c, ip)?;
                let bytes = self
                    .hal
                    .i2c_read(token, address, len)
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })?;
                self.push(Value::Bytes(bytes), ip)
            }
            CAP_I2C_TRANSFER => {
                let read_len = self.pop_u8(ip)?;
                if read_len as usize > MAX_BYTE_BUFFER_LEN {
                    return Err(RuntimeError {
                        ip,
                        kind: RuntimeErrorKind::ByteBufferTooLarge,
                    });
                }
                let write_bytes = self.pop_bytes(ip)?;
                let address = self.pop_u16(ip)?;
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::I2c, ip)?;
                let bytes = self
                    .hal
                    .i2c_transfer(token, address, write_bytes.as_slice(), read_len)
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })?;
                self.push(Value::Bytes(bytes), ip)
            }
            CAP_SPI_OPEN => {
                let bus = self.pop_u16(ip)?;
                let token = self.hal.spi_open(bus).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })?;
                let handle = self.alloc_handle(HandleKind::Spi, token, ip)?;
                self.push(Value::Handle(handle), ip)
            }
            CAP_SPI_TRANSFER => {
                let read_len = self.pop_u8(ip)?;
                if read_len as usize > MAX_BYTE_BUFFER_LEN {
                    return Err(RuntimeError {
                        ip,
                        kind: RuntimeErrorKind::ByteBufferTooLarge,
                    });
                }
                let write_bytes = self.pop_bytes(ip)?;
                let cs_pin = self.pop_u16(ip)?;
                let handle = self.pop_handle(ip)?;
                let token = self.handle_token(handle, HandleKind::Spi, ip)?;
                let bytes = self
                    .hal
                    .spi_transfer(token, cs_pin, write_bytes.as_slice(), read_len)
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })?;
                self.push(Value::Bytes(bytes), ip)
            }
            CAP_UART_OPEN => {
                let bus = self.pop_u16(ip)?;
                let token = self.hal.uart_open(bus).map_err(|err| RuntimeError {
                    ip,
                    kind: hal_error_kind(err),
                })?;
                let handle = self.alloc_handle(HandleKind::Uart, token, ip)?;
                self.push(Value::Handle(handle), ip)
            }
            CAP_LED_MATRIX_FRAME => {
                let word2 = self.pop_u32(ip)?;
                let word1 = self.pop_u32(ip)?;
                let word0 = self.pop_u32(ip)?;
                self.hal
                    .led_matrix_frame([word0, word1, word2])
                    .map_err(|err| RuntimeError {
                        ip,
                        kind: hal_error_kind(err),
                    })
            }
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::UnsupportedCapability,
            }),
        }
    }

    fn push(&mut self, value: Value, ip: usize) -> Result<(), RuntimeError> {
        if self.stack_len >= MAX_STACK {
            return Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::StackOverflow,
            });
        }
        self.stack[self.stack_len] = value;
        self.stack_len += 1;
        Ok(())
    }

    fn pop(&mut self, ip: usize) -> Result<Value, RuntimeError> {
        if self.stack_len == 0 {
            return Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::StackUnderflow,
            });
        }
        self.stack_len -= 1;
        let value = self.stack[self.stack_len];
        self.stack[self.stack_len] = Value::Unit;
        Ok(value)
    }

    fn peek(&self, ip: usize) -> Result<Value, RuntimeError> {
        if self.stack_len == 0 {
            return Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::StackUnderflow,
            });
        }
        Ok(self.stack[self.stack_len - 1])
    }

    fn swap(&mut self, ip: usize) -> Result<(), RuntimeError> {
        if self.stack_len < 2 {
            return Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::StackUnderflow,
            });
        }
        self.stack.swap(self.stack_len - 1, self.stack_len - 2);
        Ok(())
    }

    fn pop_bool(&mut self, ip: usize) -> Result<bool, RuntimeError> {
        match self.pop(ip)? {
            Value::Bool(value) => Ok(value),
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::TypeMismatch,
            }),
        }
    }

    fn pop_u16(&mut self, ip: usize) -> Result<u16, RuntimeError> {
        match self.pop(ip)? {
            Value::U8(value) => Ok(value as u16),
            Value::U16(value) => Ok(value),
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::TypeMismatch,
            }),
        }
    }

    fn pop_u8(&mut self, ip: usize) -> Result<u8, RuntimeError> {
        match self.pop(ip)? {
            Value::U8(value) => Ok(value),
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::TypeMismatch,
            }),
        }
    }

    fn pop_u32(&mut self, ip: usize) -> Result<u32, RuntimeError> {
        match self.pop(ip)? {
            Value::U8(value) => Ok(value as u32),
            Value::U16(value) => Ok(value as u32),
            Value::U32(value) => Ok(value),
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::TypeMismatch,
            }),
        }
    }

    fn pop_bytes(&mut self, ip: usize) -> Result<ByteBuffer, RuntimeError> {
        match self.pop(ip)? {
            Value::Bytes(value) => Ok(value),
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::TypeMismatch,
            }),
        }
    }

    fn pop_gpio_mode(&mut self, ip: usize) -> Result<GpioMode, RuntimeError> {
        match self.pop(ip)? {
            Value::U8(0) => Ok(GpioMode::Input),
            Value::U8(1) => Ok(GpioMode::Output),
            Value::U8(2) => Ok(GpioMode::InputPullup),
            Value::U8(3) => Ok(GpioMode::InputPulldown),
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::TypeMismatch,
            }),
        }
    }

    fn pop_handle(&mut self, ip: usize) -> Result<Handle, RuntimeError> {
        match self.pop(ip)? {
            Value::Handle(handle) => Ok(handle),
            _ => Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::TypeMismatch,
            }),
        }
    }

    fn alloc_handle(
        &mut self,
        kind: HandleKind,
        token: u32,
        ip: usize,
    ) -> Result<Handle, RuntimeError> {
        for (index, slot) in self.handles.iter_mut().enumerate() {
            if !slot.open {
                slot.generation = slot.generation.wrapping_add(1).max(1);
                slot.kind = kind;
                slot.token = token;
                slot.open = true;
                return Ok(Handle {
                    index: index as u8,
                    generation: slot.generation,
                });
            }
        }
        Err(RuntimeError {
            ip,
            kind: RuntimeErrorKind::ResourceBusy,
        })
    }

    fn handle_token(
        &self,
        handle: Handle,
        kind: HandleKind,
        ip: usize,
    ) -> Result<u32, RuntimeError> {
        let slot = self
            .handles
            .get(handle.index as usize)
            .ok_or(RuntimeError {
                ip,
                kind: RuntimeErrorKind::HandleNotFound,
            })?;
        if !slot.open || slot.generation != handle.generation || slot.kind != kind {
            return Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::HandleNotFound,
            });
        }
        Ok(slot.token)
    }

    fn close_handle(&mut self, handle: Handle, ip: usize) -> Result<(), RuntimeError> {
        let slot = self
            .handles
            .get_mut(handle.index as usize)
            .ok_or(RuntimeError {
                ip,
                kind: RuntimeErrorKind::HandleNotFound,
            })?;
        if !slot.open || slot.generation != handle.generation {
            return Err(RuntimeError {
                ip,
                kind: RuntimeErrorKind::HandleNotFound,
            });
        }
        slot.open = false;
        slot.kind = HandleKind::Empty;
        slot.token = 0;
        Ok(())
    }

    fn open_handle_count(&self) -> u8 {
        self.handles.iter().filter(|slot| slot.open).count() as u8
    }
}

fn jump_target(next_ip: usize, offset: i8, instruction_ip: usize) -> Result<usize, RuntimeError> {
    let target = next_ip as isize + offset as isize;
    if target < 0 {
        return Err(RuntimeError {
            ip: instruction_ip,
            kind: RuntimeErrorKind::InvalidBytecode,
        });
    }
    Ok(target as usize)
}

fn hal_error_kind(error: HalError) -> RuntimeErrorKind {
    match error {
        HalError::InvalidPin => RuntimeErrorKind::InvalidPin,
        HalError::UnsupportedMode => RuntimeErrorKind::UnsupportedMode,
        HalError::ResourceBusy => RuntimeErrorKind::ResourceBusy,
        HalError::BoardFault => RuntimeErrorKind::BoardFault,
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    const BLINK_CODE: &[u8] = &[
        0x12, 0x0d, 0x12, 0x01, 0x40, 0x01, 0x20, 0x11, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10,
        0x20, 0x10, 0x40, 0x02, 0x13, 0xfa, 0x00, 0x40, 0x10, 0x30, 0xec,
    ];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Open(u16, GpioMode),
        Write(u32, Level),
        Sleep(u16),
        PwmWrite(u16, u16),
        AdcRead(u16),
        DacWriteU12(u16, u16),
        I2cOpen(u16),
        I2cWriteU8(u32, u16, u8),
        I2cWrite(u32, u16, ByteBuffer),
        I2cReadU8(u32, u16),
        I2cRead(u32, u16, u8),
        I2cTransfer(u32, u16, ByteBuffer, u8),
        SpiOpen(u16),
        SpiTransfer(u32, u16, ByteBuffer, u8),
        UartOpen(u16),
        LedMatrixFrame([u32; 3]),
    }

    struct FakeHal {
        events: Vec<Event>,
        now_ms: u32,
    }

    impl FakeHal {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                now_ms: 0,
            }
        }
    }

    impl BoardHal for FakeHal {
        fn capabilities(&self) -> CapabilitySet {
            CapabilitySet::blink_mvp()
                .with_pwm()
                .with_adc()
                .with_dac()
                .with_i2c()
                .with_spi()
                .with_uart()
                .with_led_matrix()
        }

        fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
            self.events.push(Event::Open(pin, mode));
            Ok(pin as u32)
        }

        fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
            self.events.push(Event::Write(token, level));
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
            self.events.push(Event::Sleep(duration_ms));
            Ok(())
        }

        fn now_ms(&self) -> u32 {
            self.now_ms
        }

        fn pwm_write(&mut self, pin: u16, duty: u16) -> Result<(), HalError> {
            self.events.push(Event::PwmWrite(pin, duty));
            Ok(())
        }

        fn adc_read(&mut self, pin: u16) -> Result<u16, HalError> {
            self.events.push(Event::AdcRead(pin));
            Ok(0x03ff)
        }

        fn dac_write_u12(&mut self, pin: u16, sample: u16) -> Result<(), HalError> {
            self.events.push(Event::DacWriteU12(pin, sample));
            Ok(())
        }

        fn i2c_open(&mut self, bus: u16) -> Result<u32, HalError> {
            self.events.push(Event::I2cOpen(bus));
            Ok(0x1_2000 | bus as u32)
        }

        fn i2c_write_u8(&mut self, token: u32, address: u16, byte: u8) -> Result<(), HalError> {
            self.events.push(Event::I2cWriteU8(token, address, byte));
            Ok(())
        }

        fn i2c_write(&mut self, token: u32, address: u16, bytes: &[u8]) -> Result<(), HalError> {
            self.events.push(Event::I2cWrite(
                token,
                address,
                ByteBuffer::from_slice(bytes).unwrap(),
            ));
            Ok(())
        }

        fn i2c_read_u8(&mut self, token: u32, address: u16) -> Result<u8, HalError> {
            self.events.push(Event::I2cReadU8(token, address));
            Ok(0x5a)
        }

        fn i2c_read(&mut self, token: u32, address: u16, len: u8) -> Result<ByteBuffer, HalError> {
            self.events.push(Event::I2cRead(token, address, len));
            ByteBuffer::from_slice(&[0xca, 0xfe, 0x42][..len as usize])
                .map_err(|_| HalError::UnsupportedMode)
        }

        fn i2c_transfer(
            &mut self,
            token: u32,
            address: u16,
            write_bytes: &[u8],
            read_len: u8,
        ) -> Result<ByteBuffer, HalError> {
            self.events.push(Event::I2cTransfer(
                token,
                address,
                ByteBuffer::from_slice(write_bytes).unwrap(),
                read_len,
            ));
            ByteBuffer::from_slice(&[0x11, 0x22, 0x33][..read_len as usize])
                .map_err(|_| HalError::UnsupportedMode)
        }

        fn spi_open(&mut self, bus: u16) -> Result<u32, HalError> {
            self.events.push(Event::SpiOpen(bus));
            Ok(0x2_2000 | bus as u32)
        }

        fn spi_transfer(
            &mut self,
            token: u32,
            cs_pin: u16,
            write_bytes: &[u8],
            read_len: u8,
        ) -> Result<ByteBuffer, HalError> {
            self.events.push(Event::SpiTransfer(
                token,
                cs_pin,
                ByteBuffer::from_slice(write_bytes).unwrap(),
                read_len,
            ));
            let mut response = [0u8; MAX_BYTE_BUFFER_LEN];
            response[0] = 0x9f;
            response[1] = 0x01;
            response[2] = 0x02;
            ByteBuffer::from_slice(&response[..read_len as usize])
                .map_err(|_| HalError::UnsupportedMode)
        }

        fn uart_open(&mut self, bus: u16) -> Result<u32, HalError> {
            self.events.push(Event::UartOpen(bus));
            Ok(0x3_2000 | bus as u32)
        }

        fn led_matrix_frame(&mut self, frame: [u32; 3]) -> Result<(), HalError> {
            self.events.push(Event::LedMatrixFrame(frame));
            Ok(())
        }
    }

    #[test]
    fn blink_runs_until_budget_and_records_events() {
        let mut runtime: Runtime<FakeHal, 8, 4> = Runtime::new(FakeHal::new());
        let report = runtime.run_code(BLINK_CODE, 13).unwrap();

        assert_eq!(report.status, RunStatus::BudgetExceeded);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            runtime.hal().events,
            vec![
                Event::Open(13, GpioMode::Output),
                Event::Write(13, Level::High),
                Event::Sleep(250),
                Event::Write(13, Level::Low),
                Event::Sleep(250),
            ]
        );
    }

    #[test]
    fn run_code_slice_resumes_from_cursor() {
        let mut runtime: Runtime<FakeHal, 8, 4> = Runtime::new(FakeHal::new());
        let mut cursor = RunCursor::new();

        let first = runtime.run_code_slice(BLINK_CODE, &mut cursor, 3).unwrap();
        assert_eq!(first.status, RunStatus::BudgetExceeded);
        assert_eq!(first.instructions_executed, 3);
        assert_eq!(
            runtime.hal().events,
            vec![Event::Open(13, GpioMode::Output)]
        );

        let second = runtime.run_code_slice(BLINK_CODE, &mut cursor, 10).unwrap();
        assert_eq!(second.status, RunStatus::BudgetExceeded);
        assert_eq!(second.instructions_executed, 13);
        assert_eq!(
            runtime.hal().events,
            vec![
                Event::Open(13, GpioMode::Output),
                Event::Write(13, Level::High),
                Event::Sleep(250),
                Event::Write(13, Level::Low),
                Event::Sleep(250),
            ]
        );

        let third = runtime.run_code_slice(BLINK_CODE, &mut cursor, 4).unwrap();
        assert_eq!(third.status, RunStatus::BudgetExceeded);
        assert_eq!(third.instructions_executed, 17);
        assert_eq!(&runtime.hal().events[5..], &[Event::Write(13, Level::High)]);
    }

    #[test]
    fn return_top_reports_value() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let report = runtime.run_code(&[0x13, 0x34, 0x12, 0x50], 10).unwrap();
        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.return_value, Value::U16(0x1234));
    }

    #[test]
    fn push_bytes_returns_const_pool_slice() {
        let module = Module {
            flags: 0,
            max_stack: 1,
            code: &[0x16, 0x01, 0x00, 0x03, 0x50],
            const_pool: &[0x00, 0xCA, 0xFE, 0x42],
        };
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());

        let report = runtime.run_module(&module, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        let expected = ByteBuffer::from_slice(&[0xCA, 0xFE, 0x42]).unwrap();
        assert_eq!(report.return_value, Value::Bytes(expected));
        match report.return_value {
            Value::Bytes(bytes) => assert_eq!(bytes.as_slice(), &[0xCA, 0xFE, 0x42]),
            other => panic!("unexpected return value: {other:?}"),
        }
    }

    #[test]
    fn led_matrix_frame_dispatches_three_words() {
        let mut runtime: Runtime<FakeHal, 4, 1> = Runtime::new(FakeHal::new());
        let code = [
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
        ];

        let report = runtime.run_code(&code, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(
            runtime.hal().events,
            vec![Event::LedMatrixFrame([
                0x3184_A444,
                0x4404_2081,
                0x100A_0040,
            ])]
        );
    }

    #[test]
    fn pwm_write_dispatches_pin_and_normalized_duty() {
        let mut runtime: Runtime<FakeHal, 4, 1> = Runtime::new(FakeHal::new());
        let code = [0x12, 3, 0x13, 0x00, 0x80, 0x40, CAP_PWM_WRITE as u8, 0x00];

        let report = runtime.run_code(&code, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(runtime.hal().events, vec![Event::PwmWrite(3, 0x8000)]);
    }

    #[test]
    fn adc_read_dispatches_pin_and_returns_sample() {
        let mut runtime: Runtime<FakeHal, 4, 1> = Runtime::new(FakeHal::new());
        let code = [0x12, 14, 0x40, CAP_ADC_READ as u8, 0x50];

        let report = runtime.run_code(&code, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.return_value, Value::U16(0x03ff));
        assert_eq!(runtime.hal().events, vec![Event::AdcRead(14)]);
    }

    #[test]
    fn dac_write_u12_dispatches_pin_and_sample() {
        let mut runtime: Runtime<FakeHal, 4, 1> = Runtime::new(FakeHal::new());
        let code = [
            0x12,
            14,
            0x13,
            0x00,
            0x08,
            0x40,
            CAP_DAC_WRITE_U12 as u8,
            0x00,
        ];

        let report = runtime.run_code(&code, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(runtime.hal().events, vec![Event::DacWriteU12(14, 0x0800)]);
    }

    #[test]
    fn i2c_open_dispatches_bus_and_returns_handle() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let code = [0x12, 0, 0x40, CAP_I2C_OPEN as u8, 0x50];

        let report = runtime.run_code(&code, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(
            report.return_value,
            Value::Handle(Handle {
                index: 0,
                generation: 1
            })
        );
        assert_eq!(report.open_handles, 1);
        assert_eq!(runtime.hal().events, vec![Event::I2cOpen(0)]);
    }

    #[test]
    fn spi_open_dispatches_bus_and_returns_handle() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let code = [0x12, 0, 0x40, CAP_SPI_OPEN as u8, 0x50];

        let report = runtime.run_code(&code, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(
            report.return_value,
            Value::Handle(Handle {
                index: 0,
                generation: 1
            })
        );
        assert_eq!(report.open_handles, 1);
        assert_eq!(runtime.hal().events, vec![Event::SpiOpen(0)]);
    }

    #[test]
    fn uart_open_dispatches_bus_and_returns_handle() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let code = [0x12, 0, 0x40, CAP_UART_OPEN as u8, 0x50];

        let report = runtime.run_code(&code, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(
            report.return_value,
            Value::Handle(Handle {
                index: 0,
                generation: 1
            })
        );
        assert_eq!(report.open_handles, 1);
        assert_eq!(runtime.hal().events, vec![Event::UartOpen(0)]);
    }

    #[test]
    fn spi_transfer_dispatches_handle_cs_pin_write_bytes_and_returns_bytes() {
        let mut runtime: Runtime<FakeHal, 5, 2> = Runtime::new(FakeHal::new());
        let open = [0x12, 0, 0x40, CAP_SPI_OPEN as u8, 0x20, 0x50];
        let transfer = Module {
            flags: board_vm_ir::FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
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

        runtime.run_code(&open, 10).unwrap();
        let report = runtime.run_module(&transfer, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            report.return_value,
            Value::Bytes(ByteBuffer::from_slice(&[0x9f, 0x01, 0x02]).unwrap())
        );
        assert_eq!(
            runtime.hal().events,
            vec![
                Event::SpiOpen(0),
                Event::SpiTransfer(0x2_2000, 10, ByteBuffer::from_slice(&[0x9f]).unwrap(), 3)
            ]
        );
    }

    #[test]
    fn i2c_write_u8_dispatches_handle_address_and_byte() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let open = [0x12, 1, 0x40, CAP_I2C_OPEN as u8, 0x20, 0x50];
        let write = [0x20, 0x12, 0x3c, 0x12, 0xa5, 0x40, CAP_I2C_WRITE_U8 as u8];

        runtime.run_code(&open, 10).unwrap();
        let report = runtime.run_code(&write, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            runtime.hal().events,
            vec![Event::I2cOpen(1), Event::I2cWriteU8(0x1_2001, 0x3c, 0xa5)]
        );
    }

    #[test]
    fn i2c_read_u8_dispatches_handle_address_and_returns_byte() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let open = [0x12, 1, 0x40, CAP_I2C_OPEN as u8, 0x20, 0x50];
        let read = [0x20, 0x12, 0x3c, 0x40, CAP_I2C_READ_U8 as u8, 0x50];

        runtime.run_code(&open, 10).unwrap();
        let report = runtime.run_code(&read, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.return_value, Value::U8(0x5a));
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            runtime.hal().events,
            vec![Event::I2cOpen(1), Event::I2cReadU8(0x1_2001, 0x3c)]
        );
    }

    #[test]
    fn i2c_write_dispatches_handle_address_and_bytes() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let open = [0x12, 1, 0x40, CAP_I2C_OPEN as u8, 0x20, 0x50];
        let write = Module {
            flags: board_vm_ir::FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
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

        runtime.run_code(&open, 10).unwrap();
        let report = runtime.run_module(&write, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            runtime.hal().events,
            vec![
                Event::I2cOpen(1),
                Event::I2cWrite(
                    0x1_2001,
                    0x3c,
                    ByteBuffer::from_slice(&[0xde, 0xad, 0xbe]).unwrap()
                )
            ]
        );
    }

    #[test]
    fn i2c_read_dispatches_handle_address_and_returns_bytes() {
        let mut runtime: Runtime<FakeHal, 4, 2> = Runtime::new(FakeHal::new());
        let open = [0x12, 1, 0x40, CAP_I2C_OPEN as u8, 0x20, 0x50];
        let read = [0x20, 0x12, 0x3c, 0x12, 0x03, 0x40, CAP_I2C_READ as u8, 0x50];

        runtime.run_code(&open, 10).unwrap();
        let report = runtime.run_code(&read, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            report.return_value,
            Value::Bytes(ByteBuffer::from_slice(&[0xca, 0xfe, 0x42]).unwrap())
        );
        assert_eq!(
            runtime.hal().events,
            vec![Event::I2cOpen(1), Event::I2cRead(0x1_2001, 0x3c, 3)]
        );
    }

    #[test]
    fn i2c_transfer_dispatches_handle_address_write_bytes_and_returns_bytes() {
        let mut runtime: Runtime<FakeHal, 5, 2> = Runtime::new(FakeHal::new());
        let open = [0x12, 1, 0x40, CAP_I2C_OPEN as u8, 0x20, 0x50];
        let transfer = Module {
            flags: board_vm_ir::FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
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

        runtime.run_code(&open, 10).unwrap();
        let report = runtime.run_module(&transfer, 10).unwrap();

        assert_eq!(report.status, RunStatus::Halted);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            report.return_value,
            Value::Bytes(ByteBuffer::from_slice(&[0x11, 0x22, 0x33]).unwrap())
        );
        assert_eq!(
            runtime.hal().events,
            vec![
                Event::I2cOpen(1),
                Event::I2cTransfer(
                    0x1_2001,
                    0x3c,
                    ByteBuffer::from_slice(&[0x00, 0x10]).unwrap(),
                    3
                )
            ]
        );
    }
}
