#![no_std]

use board_vm_ir::{parse_module, validate, ModuleError, ValidateError};
use board_vm_runtime::{BoardHal, RunReport, Runtime, RuntimeError};

#[cfg(target_arch = "arm")]
pub mod uno_r4_wifi_led;

pub const EMBEDDED_BLINK_MODULE: [u8; 36] = [
    0x42, 0x56, 0x4D, 0x31, 0x01, 0x01, 0x04, 0x00, 0x1A, 0x12, 0x0D, 0x12, 0x01, 0x40, 0x01, 0x20,
    0x11, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40, 0x10, 0x20, 0x10, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40,
    0x10, 0x30, 0xEC, 0x00,
];

pub const SMOKE_INSTRUCTION_BUDGET: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareSmokeError {
    Module(ModuleError),
    Validate(ValidateError),
    Runtime(RuntimeError),
}

impl From<ModuleError> for FirmwareSmokeError {
    fn from(value: ModuleError) -> Self {
        Self::Module(value)
    }
}

impl From<ValidateError> for FirmwareSmokeError {
    fn from(value: ValidateError) -> Self {
        Self::Validate(value)
    }
}

impl From<RuntimeError> for FirmwareSmokeError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

pub fn validate_embedded_blink_module(board_max_stack: u8) -> Result<(), FirmwareSmokeError> {
    let module = parse_module(&EMBEDDED_BLINK_MODULE)?;
    validate(
        &module,
        board_vm_ir::CapabilitySet::blink_mvp(),
        board_max_stack,
    )?;
    Ok(())
}

pub fn run_blink_smoke_once<H, const MAX_STACK: usize, const MAX_HANDLES: usize>(
    runtime: &mut Runtime<H, MAX_STACK, MAX_HANDLES>,
    instruction_budget: u32,
) -> Result<RunReport, FirmwareSmokeError>
where
    H: BoardHal,
{
    let module = parse_module(&EMBEDDED_BLINK_MODULE)?;
    runtime.reset_vm();
    Ok(runtime.run_module(&module, instruction_budget)?)
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_ir::{parse_module, CapabilitySet, FLAG_PROGRAM_MAY_RUN_FOREVER};
    use board_vm_runtime::{GpioMode, HalError, Level, RunStatus};
    use std::vec::Vec;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Open { pin: u16, mode: GpioMode },
        Write { token: u32, level: Level },
        Sleep(u16),
    }

    struct FakeHal {
        now_ms: u32,
        next_token: u32,
        events: Vec<Event>,
    }

    impl FakeHal {
        fn new() -> Self {
            Self {
                now_ms: 0,
                next_token: 1,
                events: Vec::new(),
            }
        }
    }

    impl BoardHal for FakeHal {
        fn capabilities(&self) -> CapabilitySet {
            CapabilitySet::blink_mvp()
        }

        fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
            let token = self.next_token;
            self.next_token = self.next_token.wrapping_add(1).max(1);
            self.events.push(Event::Open { pin, mode });
            Ok(token)
        }

        fn gpio_write(&mut self, token: u32, level: Level) -> Result<(), HalError> {
            self.events.push(Event::Write { token, level });
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
    fn embedded_blink_module_is_valid_bvm1_bytecode() {
        let module = parse_module(&EMBEDDED_BLINK_MODULE).unwrap();

        assert_eq!(module.flags, FLAG_PROGRAM_MAY_RUN_FOREVER);
        assert_eq!(module.max_stack, 4);
        assert!(module.const_pool.is_empty());
        validate_embedded_blink_module(16).unwrap();
    }

    #[test]
    fn smoke_cycle_runs_blink_bytecode_against_hal() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let report = run_blink_smoke_once(&mut runtime, SMOKE_INSTRUCTION_BUDGET).unwrap();

        assert_eq!(report.status, RunStatus::BudgetExceeded);
        assert_eq!(report.open_handles, 1);
        assert_eq!(
            &runtime.hal().events[..5],
            &[
                Event::Open {
                    pin: 13,
                    mode: GpioMode::Output
                },
                Event::Write {
                    token: 1,
                    level: Level::High
                },
                Event::Sleep(250),
                Event::Write {
                    token: 1,
                    level: Level::Low
                },
                Event::Sleep(250),
            ]
        );
    }
}
