#![no_std]

use board_vm_ir::{
    collect_required_capabilities, parse_module, validate, CapabilitySet, Module, ModuleError,
    RequiredCapabilitiesError, ValidateError, MODULE_VERSION,
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
#[cfg(target_arch = "arm")]
pub mod uno_r4_wifi_led_matrix;

pub const EMBEDDED_BLINK_MODULE: [u8; 36] = [
    0x42, 0x56, 0x4D, 0x31, 0x01, 0x01, 0x04, 0x00, 0x1A, 0x12, 0x0D, 0x12, 0x01, 0x40, 0x01, 0x20,
    0x11, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40, 0x10, 0x20, 0x10, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40,
    0x10, 0x30, 0xEC, 0x00,
];

pub const SMOKE_INSTRUCTION_BUDGET: u32 = 100;
pub const EJECTED_INSTRUCTION_BUDGET: u32 = SMOKE_INSTRUCTION_BUDGET;
pub const MAX_EJECTED_REQUIRED_CAPABILITIES: usize = 16;

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
    pub required_capabilities: &'a [u16],
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
            required_capabilities: &ejected_blink::BOARD_VM_REQUIRED_CAPABILITIES,
            module: &ejected_blink::BOARD_VM_PROGRAM,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareSmokeError {
    Module(ModuleError),
    Validate(ValidateError),
    Runtime(RuntimeError),
    RequiredCapabilities(RequiredCapabilitiesError),
    UnsupportedProgramFormat(u8),
    InvalidBootPolicy(u8),
    ArtifactMetadataMismatch,
    ArtifactCrcMismatch,
    ArtifactRequiredCapabilitiesMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EjectedBootAction {
    StoreOnly,
    Run,
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

impl From<RequiredCapabilitiesError> for FirmwareSmokeError {
    fn from(value: RequiredCapabilitiesError) -> Self {
        Self::RequiredCapabilities(value)
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

pub fn ejected_boot_action(
    program: EjectedFirmwareProgram<'_>,
) -> Result<EjectedBootAction, FirmwareSmokeError> {
    parse_checked_ejected_program(program)?;
    boot_action_for_policy(program.boot_policy)
}

pub fn run_ejected_boot_program_once<H, const MAX_STACK: usize, const MAX_HANDLES: usize>(
    runtime: &mut Runtime<H, MAX_STACK, MAX_HANDLES>,
    program: EjectedFirmwareProgram<'_>,
    instruction_budget: u32,
) -> Result<Option<RunReport>, FirmwareSmokeError>
where
    H: BoardHal,
{
    let module = parse_checked_ejected_program(program)?;
    match boot_action_for_policy(program.boot_policy)? {
        EjectedBootAction::StoreOnly => Ok(None),
        EjectedBootAction::Run => {
            validate(&module, runtime.hal().capabilities(), MAX_STACK as u8)?;
            runtime.reset_vm();
            Ok(Some(runtime.run_module(&module, instruction_budget)?))
        }
    }
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
    boot_action_for_policy(program.boot_policy)?;
    if program.module_version != MODULE_VERSION {
        return Err(FirmwareSmokeError::Module(ModuleError::UnsupportedVersion(
            program.module_version,
        )));
    }

    let module = parse_module(program.module)?;
    if crc32_ieee(program.module) != program.module_crc32 {
        return Err(FirmwareSmokeError::ArtifactCrcMismatch);
    }
    if module.flags != program.module_flags || module.max_stack != program.max_stack {
        return Err(FirmwareSmokeError::ArtifactMetadataMismatch);
    }
    let mut required_capabilities = [0u16; MAX_EJECTED_REQUIRED_CAPABILITIES];
    let required_len = collect_required_capabilities(&module, &mut required_capabilities)?;
    if &required_capabilities[..required_len] != program.required_capabilities {
        return Err(FirmwareSmokeError::ArtifactRequiredCapabilitiesMismatch);
    }
    Ok(module)
}

fn boot_action_for_policy(boot_policy: u8) -> Result<EjectedBootAction, FirmwareSmokeError> {
    match boot_policy {
        BOOT_STORE_ONLY => Ok(EjectedBootAction::StoreOnly),
        BOOT_RUN_AT_BOOT | BOOT_RUN_IF_NO_HOST => Ok(EjectedBootAction::Run),
        other => Err(FirmwareSmokeError::InvalidBootPolicy(other)),
    }
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

#[cfg(any(test, not(target_arch = "arm")))]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_ir::{
        parse_module, CapabilitySet, CAP_GPIO_OPEN, CAP_GPIO_WRITE, CAP_TIME_SLEEP_MS,
        FLAG_PROGRAM_MAY_RUN_FOREVER,
    };
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
        assert_eq!(program.module_crc32, crc32_ieee(program.module));
        assert_eq!(
            program.required_capabilities,
            [CAP_GPIO_OPEN, CAP_GPIO_WRITE, CAP_TIME_SLEEP_MS]
        );
        assert_eq!(program.module, &EMBEDDED_BLINK_MODULE);
        validate_ejected_blink_program(16).unwrap();
    }

    #[test]
    fn ejected_blink_boot_policy_runs_without_host() {
        assert_eq!(
            ejected_boot_action(EjectedFirmwareProgram::blink()).unwrap(),
            EjectedBootAction::Run
        );
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
    fn rejects_ejected_artifact_crc_mismatch() {
        let mut program = EjectedFirmwareProgram::blink();
        program.module_crc32 ^= 1;

        assert_eq!(
            validate_ejected_program(program, CapabilitySet::blink_mvp(), 16).unwrap_err(),
            FirmwareSmokeError::ArtifactCrcMismatch
        );
    }

    #[test]
    fn rejects_ejected_artifact_required_capabilities_mismatch() {
        let mut program = EjectedFirmwareProgram::blink();
        program.required_capabilities = &[CAP_GPIO_OPEN, CAP_TIME_SLEEP_MS];

        assert_eq!(
            validate_ejected_program(program, CapabilitySet::blink_mvp(), 16).unwrap_err(),
            FirmwareSmokeError::ArtifactRequiredCapabilitiesMismatch
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

    #[test]
    fn ejected_boot_cycle_skips_store_only_artifact() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);
        let mut program = EjectedFirmwareProgram::blink();
        program.boot_policy = BOOT_STORE_ONLY;

        let report =
            run_ejected_boot_program_once(&mut runtime, program, EJECTED_INSTRUCTION_BUDGET)
                .unwrap();

        assert!(report.is_none());
        assert!(runtime.hal().events.is_empty());
    }
}
