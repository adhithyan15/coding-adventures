#![no_std]

use board_vm_ir::{
    parse_module, validate, CapabilitySet, Module, ModuleError, ValidateError, MODULE_VERSION,
};
use board_vm_protocol::{ProgramFormat, BOOT_RUN_AT_BOOT, BOOT_RUN_IF_NO_HOST, BOOT_STORE_ONLY};
use board_vm_runtime::{BoardHal, RunReport, Runtime, RuntimeError};

pub mod arduino_usb_link;
pub mod ejected_blink;
#[cfg(target_arch = "arm")]
pub mod scripted_probe_stream;
#[cfg(not(target_arch = "arm"))]
pub mod serial_usb_artifact;
pub mod serial_usb_server;
#[cfg(target_arch = "arm")]
pub mod uno_r4_wifi_backend;
#[cfg(target_arch = "arm")]
pub mod uno_r4_wifi_led;

pub const EMBEDDED_BLINK_MODULE: [u8; 36] = [
    0x42, 0x56, 0x4D, 0x31, 0x01, 0x01, 0x04, 0x00, 0x1A, 0x12, 0x0D, 0x12, 0x01, 0x40, 0x01, 0x20,
    0x11, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40, 0x10, 0x20, 0x10, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40,
    0x10, 0x30, 0xEC, 0x00,
];

pub const SMOKE_INSTRUCTION_BUDGET: u32 = 100;
pub const EJECTED_INSTRUCTION_BUDGET: u32 = SMOKE_INSTRUCTION_BUDGET;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedFirmwareProgram<'a> {
    pub program_id: u16,
    pub slot: u8,
    pub boot_policy: u8,
    pub program_format: u8,
    pub module_version: u8,
    pub module_flags: u8,
    pub max_stack: u8,
    pub module_crc32: u32,
    pub module: &'a [u8],
}

impl<'a> EjectedFirmwareProgram<'a> {
    pub const fn module_len(self) -> usize {
        self.module.len()
    }

    pub const fn blink() -> Self {
        Self {
            program_id: ejected_blink::BOARD_VM_PROGRAM_ID,
            slot: ejected_blink::BOARD_VM_PROGRAM_SLOT,
            boot_policy: ejected_blink::BOARD_VM_BOOT_POLICY,
            program_format: ejected_blink::BOARD_VM_PROGRAM_FORMAT,
            module_version: ejected_blink::BOARD_VM_MODULE_VERSION,
            module_flags: ejected_blink::BOARD_VM_MODULE_FLAGS,
            max_stack: ejected_blink::BOARD_VM_MODULE_MAX_STACK,
            module_crc32: ejected_blink::BOARD_VM_PROGRAM_CRC32,
            module: &ejected_blink::BOARD_VM_PROGRAM,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareSmokeError {
    Module(ModuleError),
    Validate(ValidateError),
    Runtime(RuntimeError),
    UnsupportedProgramFormat(u8),
    InvalidBootPolicy(u8),
    ArtifactMetadataMismatch,
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

pub fn validate_ejected_program(
    program: EjectedFirmwareProgram<'_>,
    capabilities: CapabilitySet,
    board_max_stack: u8,
) -> Result<(), FirmwareSmokeError> {
    let module = parse_checked_ejected_program(program)?;
    validate(&module, capabilities, board_max_stack)?;
    Ok(())
}

pub fn validate_ejected_blink_program(board_max_stack: u8) -> Result<(), FirmwareSmokeError> {
    validate_ejected_program(
        EjectedFirmwareProgram::blink(),
        CapabilitySet::blink_mvp(),
        board_max_stack,
    )
}

pub fn run_ejected_program_once<H, const MAX_STACK: usize, const MAX_HANDLES: usize>(
    runtime: &mut Runtime<H, MAX_STACK, MAX_HANDLES>,
    program: EjectedFirmwareProgram<'_>,
    instruction_budget: u32,
) -> Result<RunReport, FirmwareSmokeError>
where
    H: BoardHal,
{
    let module = parse_checked_ejected_program(program)?;
    validate(&module, runtime.hal().capabilities(), MAX_STACK as u8)?;
    runtime.reset_vm();
    Ok(runtime.run_module(&module, instruction_budget)?)
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

fn parse_checked_ejected_program(
    program: EjectedFirmwareProgram<'_>,
) -> Result<Module<'_>, FirmwareSmokeError> {
    if program.program_format != ProgramFormat::BvmModule.as_u8() {
        return Err(FirmwareSmokeError::UnsupportedProgramFormat(
            program.program_format,
        ));
    }
    match program.boot_policy {
        BOOT_STORE_ONLY | BOOT_RUN_AT_BOOT | BOOT_RUN_IF_NO_HOST => {}
        other => return Err(FirmwareSmokeError::InvalidBootPolicy(other)),
    }
    if program.module_version != MODULE_VERSION {
        return Err(FirmwareSmokeError::Module(ModuleError::UnsupportedVersion(
            program.module_version,
        )));
    }

    let module = parse_module(program.module)?;
    if module.flags != program.module_flags || module.max_stack != program.max_stack {
        return Err(FirmwareSmokeError::ArtifactMetadataMismatch);
    }
    Ok(module)
}

#[cfg(any(test, not(target_arch = "arm")))]
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
    fn ejected_blink_program_matches_embedded_blink_bytecode() {
        let program = EjectedFirmwareProgram::blink();

        assert_eq!(program.program_id, 1);
        assert_eq!(program.slot, 0);
        assert_eq!(program.boot_policy, BOOT_RUN_IF_NO_HOST);
        assert_eq!(program.program_format, ProgramFormat::BvmModule.as_u8());
        assert_eq!(program.module_version, MODULE_VERSION);
        assert_eq!(program.module_len(), EMBEDDED_BLINK_MODULE.len());
        assert_eq!(program.module, &EMBEDDED_BLINK_MODULE);
        validate_ejected_blink_program(16).unwrap();
    }

    #[test]
    fn rejects_ejected_artifact_metadata_mismatch() {
        let mut program = EjectedFirmwareProgram::blink();
        program.max_stack = program.max_stack.saturating_add(1);

        assert_eq!(
            validate_ejected_program(program, CapabilitySet::blink_mvp(), 16).unwrap_err(),
            FirmwareSmokeError::ArtifactMetadataMismatch
        );
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

    #[test]
    fn ejected_cycle_runs_blink_bytecode_against_hal() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let report = run_ejected_program_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        )
        .unwrap();

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
