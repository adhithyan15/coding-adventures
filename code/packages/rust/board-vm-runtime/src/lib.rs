#![no_std]

use board_vm_ir::{
    decode_next, validate, CapabilitySet, Module, Op, CAP_GPIO_CLOSE, CAP_GPIO_OPEN, CAP_GPIO_READ,
    CAP_GPIO_WRITE, CAP_TIME_NOW_MS, CAP_TIME_SLEEP_MS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Unit,
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    I16(i16),
    Handle(Handle),
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandleKind {
    Empty,
    Gpio,
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
        self.run_code_slice(module.code, &mut cursor, instruction_budget)
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
        self.run_code_slice(module.code, cursor, instruction_budget)
    }

    pub fn run_code_slice(
        &mut self,
        code: &[u8],
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
}
