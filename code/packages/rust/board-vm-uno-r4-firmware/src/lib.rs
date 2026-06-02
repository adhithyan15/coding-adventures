#![no_std]

use board_vm_ir::{
    collect_required_capabilities, parse_module, validate, CapabilitySet, Module, ModuleError,
    RequiredCapabilitiesError, ValidateError, MODULE_VERSION,
};
use board_vm_protocol::{ProgramFormat, BOOT_RUN_AT_BOOT, BOOT_RUN_IF_NO_HOST, BOOT_STORE_ONLY};
use board_vm_runtime::{BoardHal, RunReport, RunStatus, Runtime, RuntimeError};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedFirmwareProgramSummary {
    pub program_id: u16,
    pub slot: u8,
    pub boot_policy: u8,
    pub program_format: u8,
    pub module_version: u8,
    pub module_flags: u8,
    pub max_stack: u8,
    pub module_crc32: u32,
    pub required_capability_count: usize,
    pub module_len: usize,
}

impl<'a> EjectedFirmwareProgram<'a> {
    pub const fn module_len(self) -> usize {
        self.module.len()
    }

    pub const fn summary(self) -> EjectedFirmwareProgramSummary {
        EjectedFirmwareProgramSummary {
            program_id: self.program_id,
            slot: self.slot,
            boot_policy: self.boot_policy,
            program_format: self.program_format,
            module_version: self.module_version,
            module_flags: self.module_flags,
            max_stack: self.max_stack,
            module_crc32: self.module_crc32,
            required_capability_count: self.required_capabilities.len(),
            module_len: self.module.len(),
        }
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
pub enum FirmwareSmokeErrorKind {
    Module,
    Validate,
    Runtime,
    RequiredCapabilities,
    UnsupportedProgramFormat,
    InvalidBootPolicy,
    ArtifactMetadataMismatch,
    ArtifactCrcMismatch,
    ArtifactRequiredCapabilitiesMismatch,
}

impl FirmwareSmokeErrorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Validate => "validate",
            Self::Runtime => "runtime",
            Self::RequiredCapabilities => "required_capabilities",
            Self::UnsupportedProgramFormat => "unsupported_program_format",
            Self::InvalidBootPolicy => "invalid_boot_policy",
            Self::ArtifactMetadataMismatch => "artifact_metadata_mismatch",
            Self::ArtifactCrcMismatch => "artifact_crc_mismatch",
            Self::ArtifactRequiredCapabilitiesMismatch => "artifact_required_capabilities_mismatch",
        }
    }
}

impl FirmwareSmokeError {
    pub const fn kind(self) -> FirmwareSmokeErrorKind {
        match self {
            Self::Module(_) => FirmwareSmokeErrorKind::Module,
            Self::Validate(_) => FirmwareSmokeErrorKind::Validate,
            Self::Runtime(_) => FirmwareSmokeErrorKind::Runtime,
            Self::RequiredCapabilities(_) => FirmwareSmokeErrorKind::RequiredCapabilities,
            Self::UnsupportedProgramFormat(_) => FirmwareSmokeErrorKind::UnsupportedProgramFormat,
            Self::InvalidBootPolicy(_) => FirmwareSmokeErrorKind::InvalidBootPolicy,
            Self::ArtifactMetadataMismatch => FirmwareSmokeErrorKind::ArtifactMetadataMismatch,
            Self::ArtifactCrcMismatch => FirmwareSmokeErrorKind::ArtifactCrcMismatch,
            Self::ArtifactRequiredCapabilitiesMismatch => {
                FirmwareSmokeErrorKind::ArtifactRequiredCapabilitiesMismatch
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EjectedBootAction {
    StoreOnly,
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedBootPlan {
    pub action: EjectedBootAction,
    pub summary: EjectedFirmwareProgramSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedBootRun {
    pub boot_plan: EjectedBootPlan,
    pub report: Option<RunReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedBootRunSummary {
    pub boot_plan: EjectedBootPlan,
    pub run_status: Option<RunStatus>,
    pub instructions_executed: Option<u32>,
    pub open_handles: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedBootFailure {
    pub program: EjectedFirmwareProgramSummary,
    pub boot_plan: Option<EjectedBootPlan>,
    pub error: FirmwareSmokeError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EjectedBootDiagnosticOutcome {
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EjectedBootDiagnosticStatus {
    Ran,
    SkippedStoreOnly,
    ValidationFailed,
    RuntimeFailed,
}

impl EjectedBootDiagnosticStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ran => "ran",
            Self::SkippedStoreOnly => "skipped_store_only",
            Self::ValidationFailed => "validation_failed",
            Self::RuntimeFailed => "runtime_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedBootDiagnosticSummary {
    pub outcome: EjectedBootDiagnosticOutcome,
    pub program: EjectedFirmwareProgramSummary,
    pub boot_plan: Option<EjectedBootPlan>,
    pub run_status: Option<RunStatus>,
    pub instructions_executed: Option<u32>,
    pub open_handles: Option<u8>,
    pub error: Option<FirmwareSmokeError>,
}

impl EjectedBootDiagnosticSummary {
    pub fn completed(self) -> bool {
        matches!(self.outcome, EjectedBootDiagnosticOutcome::Completed)
    }

    pub fn failed(self) -> bool {
        matches!(self.outcome, EjectedBootDiagnosticOutcome::Failed)
    }

    pub fn validated(self) -> bool {
        self.boot_plan.is_some()
    }

    pub fn ran(self) -> bool {
        matches!(self.status(), EjectedBootDiagnosticStatus::Ran)
    }

    pub fn skipped_store_only(self) -> bool {
        matches!(self.status(), EjectedBootDiagnosticStatus::SkippedStoreOnly)
    }

    pub fn status(self) -> EjectedBootDiagnosticStatus {
        match (self.outcome, self.boot_plan.map(|plan| plan.action)) {
            (EjectedBootDiagnosticOutcome::Completed, Some(EjectedBootAction::StoreOnly)) => {
                EjectedBootDiagnosticStatus::SkippedStoreOnly
            }
            (EjectedBootDiagnosticOutcome::Completed, _) => EjectedBootDiagnosticStatus::Ran,
            (EjectedBootDiagnosticOutcome::Failed, Some(_)) => {
                EjectedBootDiagnosticStatus::RuntimeFailed
            }
            (EjectedBootDiagnosticOutcome::Failed, None) => {
                EjectedBootDiagnosticStatus::ValidationFailed
            }
        }
    }

    pub fn status_label(self) -> &'static str {
        self.status().as_str()
    }

    pub fn error_kind(self) -> Option<FirmwareSmokeErrorKind> {
        self.error.map(FirmwareSmokeError::kind)
    }

    pub fn error_label(self) -> Option<&'static str> {
        self.error_kind().map(FirmwareSmokeErrorKind::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EjectedBootDiagnostic {
    Completed(EjectedBootRunSummary),
    Failed(EjectedBootFailure),
}

impl EjectedBootDiagnostic {
    pub fn completed(self) -> bool {
        matches!(self, Self::Completed(_))
    }

    pub fn failed(self) -> bool {
        matches!(self, Self::Failed(_))
    }

    pub fn summary(self) -> EjectedBootDiagnosticSummary {
        match self {
            Self::Completed(summary) => EjectedBootDiagnosticSummary {
                outcome: EjectedBootDiagnosticOutcome::Completed,
                program: summary.boot_plan.summary,
                boot_plan: Some(summary.boot_plan),
                run_status: summary.run_status,
                instructions_executed: summary.instructions_executed,
                open_handles: summary.open_handles,
                error: None,
            },
            Self::Failed(failure) => EjectedBootDiagnosticSummary {
                outcome: EjectedBootDiagnosticOutcome::Failed,
                program: failure.program,
                boot_plan: failure.boot_plan,
                run_status: None,
                instructions_executed: None,
                open_handles: None,
                error: Some(failure.error),
            },
        }
    }
}

impl EjectedBootRun {
    pub fn ran(self) -> bool {
        self.report.is_some()
    }

    pub fn summary(self) -> EjectedBootRunSummary {
        let (run_status, instructions_executed, open_handles) = match self.report {
            Some(report) => (
                Some(report.status),
                Some(report.instructions_executed),
                Some(report.open_handles),
            ),
            None => (None, None, None),
        };

        EjectedBootRunSummary {
            boot_plan: self.boot_plan,
            run_status,
            instructions_executed,
            open_handles,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedEjectedFirmwareProgram<'a> {
    pub module: Module<'a>,
    pub boot_plan: EjectedBootPlan,
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
    validate_ejected_firmware_program(program, capabilities, board_max_stack).map(|_| ())
}

pub fn validate_ejected_boot_plan(
    program: EjectedFirmwareProgram<'_>,
    capabilities: CapabilitySet,
    board_max_stack: u8,
) -> Result<EjectedBootPlan, FirmwareSmokeError> {
    Ok(validate_ejected_firmware_program(program, capabilities, board_max_stack)?.boot_plan)
}

pub fn validate_ejected_firmware_program<'a>(
    program: EjectedFirmwareProgram<'a>,
    capabilities: CapabilitySet,
    board_max_stack: u8,
) -> Result<ValidatedEjectedFirmwareProgram<'a>, FirmwareSmokeError> {
    let module = parse_checked_ejected_program(program)?;
    validate(&module, capabilities, board_max_stack)?;
    Ok(ValidatedEjectedFirmwareProgram {
        module,
        boot_plan: EjectedBootPlan {
            action: boot_action_for_policy(program.boot_policy)?,
            summary: program.summary(),
        },
    })
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
    Ok(ejected_boot_plan(program)?.action)
}

pub fn ejected_boot_plan(
    program: EjectedFirmwareProgram<'_>,
) -> Result<EjectedBootPlan, FirmwareSmokeError> {
    parse_checked_ejected_program(program)?;
    Ok(EjectedBootPlan {
        action: boot_action_for_policy(program.boot_policy)?,
        summary: program.summary(),
    })
}

pub fn run_ejected_boot_program_once<H, const MAX_STACK: usize, const MAX_HANDLES: usize>(
    runtime: &mut Runtime<H, MAX_STACK, MAX_HANDLES>,
    program: EjectedFirmwareProgram<'_>,
    instruction_budget: u32,
) -> Result<Option<RunReport>, FirmwareSmokeError>
where
    H: BoardHal,
{
    Ok(run_ejected_boot_program_checked_once(runtime, program, instruction_budget)?.report)
}

pub fn run_ejected_boot_program_checked_once<H, const MAX_STACK: usize, const MAX_HANDLES: usize>(
    runtime: &mut Runtime<H, MAX_STACK, MAX_HANDLES>,
    program: EjectedFirmwareProgram<'_>,
    instruction_budget: u32,
) -> Result<EjectedBootRun, FirmwareSmokeError>
where
    H: BoardHal,
{
    let validated =
        validate_ejected_firmware_program(program, runtime.hal().capabilities(), MAX_STACK as u8)?;
    run_validated_ejected_boot_program_once(runtime, validated, instruction_budget)
}

fn run_validated_ejected_boot_program_once<H, const MAX_STACK: usize, const MAX_HANDLES: usize>(
    runtime: &mut Runtime<H, MAX_STACK, MAX_HANDLES>,
    validated: ValidatedEjectedFirmwareProgram<'_>,
    instruction_budget: u32,
) -> Result<EjectedBootRun, FirmwareSmokeError>
where
    H: BoardHal,
{
    let report = match validated.boot_plan.action {
        EjectedBootAction::StoreOnly => None,
        EjectedBootAction::Run => {
            runtime.reset_vm();
            Some(runtime.run_module(&validated.module, instruction_budget)?)
        }
    };
    Ok(EjectedBootRun {
        boot_plan: validated.boot_plan,
        report,
    })
}

pub fn diagnose_ejected_boot_program_once<H, const MAX_STACK: usize, const MAX_HANDLES: usize>(
    runtime: &mut Runtime<H, MAX_STACK, MAX_HANDLES>,
    program: EjectedFirmwareProgram<'_>,
    instruction_budget: u32,
) -> EjectedBootDiagnostic
where
    H: BoardHal,
{
    let program_summary = program.summary();
    let validated = match validate_ejected_firmware_program(
        program,
        runtime.hal().capabilities(),
        MAX_STACK as u8,
    ) {
        Ok(validated) => validated,
        Err(error) => {
            return EjectedBootDiagnostic::Failed(EjectedBootFailure {
                program: program_summary,
                boot_plan: None,
                error,
            });
        }
    };
    let boot_plan = validated.boot_plan;
    match run_validated_ejected_boot_program_once(runtime, validated, instruction_budget) {
        Ok(boot_run) => EjectedBootDiagnostic::Completed(boot_run.summary()),
        Err(error) => EjectedBootDiagnostic::Failed(EjectedBootFailure {
            program: program_summary,
            boot_plan: Some(boot_plan),
            error,
        }),
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
    let validated =
        validate_ejected_firmware_program(program, runtime.hal().capabilities(), MAX_STACK as u8)?;
    runtime.reset_vm();
    Ok(runtime.run_module(&validated.module, instruction_budget)?)
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
    use board_vm_eject::{build_blink_eject_artifact, EjectOptions};
    use board_vm_host::BlinkProgram;
    use board_vm_ir::{
        parse_module, CapabilitySet, CAP_GPIO_OPEN, CAP_GPIO_WRITE, CAP_TIME_SLEEP_MS,
        FLAG_PROGRAM_MAY_RUN_FOREVER,
    };
    use board_vm_runtime::{GpioMode, HalError, Level, RunStatus, RuntimeErrorKind};
    use std::vec::Vec;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Open { pin: u16, mode: GpioMode },
        Write { token: u32, level: Level },
        Sleep(u16),
    }

    struct FakeHal {
        capabilities: CapabilitySet,
        fail_gpio_open: bool,
        now_ms: u32,
        next_token: u32,
        events: Vec<Event>,
    }

    impl FakeHal {
        fn new() -> Self {
            Self::with_capabilities(CapabilitySet::blink_mvp())
        }

        fn with_capabilities(capabilities: CapabilitySet) -> Self {
            Self {
                capabilities,
                fail_gpio_open: false,
                now_ms: 0,
                next_token: 1,
                events: Vec::new(),
            }
        }

        fn failing_gpio_open() -> Self {
            let mut hal = Self::new();
            hal.fail_gpio_open = true;
            hal
        }
    }

    impl BoardHal for FakeHal {
        fn capabilities(&self) -> CapabilitySet {
            self.capabilities
        }

        fn gpio_open(&mut self, pin: u16, mode: GpioMode) -> Result<u32, HalError> {
            if self.fail_gpio_open {
                return Err(HalError::BoardFault);
            }
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
    fn ejected_blink_summary_surfaces_embedded_artifact_contract() {
        assert_eq!(
            EjectedFirmwareProgram::blink().summary(),
            EjectedFirmwareProgramSummary {
                program_id: 1,
                slot: 0,
                boot_policy: BOOT_RUN_IF_NO_HOST,
                program_format: ProgramFormat::BvmModule.as_u8(),
                module_version: MODULE_VERSION,
                module_flags: FLAG_PROGRAM_MAY_RUN_FOREVER,
                max_stack: 4,
                module_crc32: 0xBAD6_949E,
                required_capability_count: 3,
                module_len: EMBEDDED_BLINK_MODULE.len(),
            }
        );
    }

    #[test]
    fn ejected_blink_program_matches_eject_generator_output() {
        let program = EjectedFirmwareProgram::blink();
        let mut generated_module = [0u8; EMBEDDED_BLINK_MODULE.len()];

        let artifact = build_blink_eject_artifact(
            BlinkProgram::onboard_led(),
            EjectOptions::new(program.program_id)
                .slot(program.slot)
                .boot_policy(program.boot_policy),
            &mut generated_module,
        )
        .unwrap();

        assert_eq!(program.program_id, artifact.program_id);
        assert_eq!(program.slot, artifact.slot);
        assert_eq!(program.boot_policy, artifact.boot_policy);
        assert_eq!(program.program_format, artifact.format.as_u8());
        assert_eq!(program.module_version, artifact.module_version);
        assert_eq!(program.module_flags, artifact.module_flags);
        assert_eq!(program.max_stack, artifact.max_stack);
        assert_eq!(program.module_crc32, artifact.module_crc32);
        assert_eq!(
            program.required_capabilities,
            artifact.required_capabilities
        );
        assert_eq!(program.module, artifact.module);
    }

    #[test]
    fn ejected_blink_summary_matches_eject_generator_summary() {
        let program = EjectedFirmwareProgram::blink();
        let mut generated_module = [0u8; EMBEDDED_BLINK_MODULE.len()];

        let artifact = build_blink_eject_artifact(
            BlinkProgram::onboard_led(),
            EjectOptions::new(program.program_id)
                .slot(program.slot)
                .boot_policy(program.boot_policy),
            &mut generated_module,
        )
        .unwrap();
        let firmware_summary = program.summary();
        let eject_summary = artifact.summary();

        assert_eq!(firmware_summary.program_id, eject_summary.program_id);
        assert_eq!(firmware_summary.slot, eject_summary.slot);
        assert_eq!(firmware_summary.boot_policy, eject_summary.boot_policy);
        assert_eq!(
            firmware_summary.program_format,
            eject_summary.program_format.as_u8()
        );
        assert_eq!(
            firmware_summary.module_version,
            eject_summary.module_version
        );
        assert_eq!(firmware_summary.module_flags, eject_summary.module_flags);
        assert_eq!(firmware_summary.max_stack, eject_summary.max_stack);
        assert_eq!(firmware_summary.module_crc32, eject_summary.module_crc32);
        assert_eq!(
            firmware_summary.required_capability_count,
            eject_summary.required_capability_count
        );
        assert_eq!(firmware_summary.module_len, eject_summary.module_len);
    }

    #[test]
    fn ejected_blink_boot_policy_runs_without_host() {
        assert_eq!(
            ejected_boot_action(EjectedFirmwareProgram::blink()).unwrap(),
            EjectedBootAction::Run
        );
    }

    #[test]
    fn ejected_blink_boot_plan_surfaces_checked_summary_and_action() {
        assert_eq!(
            ejected_boot_plan(EjectedFirmwareProgram::blink()).unwrap(),
            EjectedBootPlan {
                action: EjectedBootAction::Run,
                summary: EjectedFirmwareProgram::blink().summary(),
            }
        );
    }

    #[test]
    fn validated_ejected_blink_boot_plan_checks_board_contract() {
        assert_eq!(
            validate_ejected_boot_plan(
                EjectedFirmwareProgram::blink(),
                CapabilitySet::blink_mvp(),
                16
            )
            .unwrap(),
            EjectedBootPlan {
                action: EjectedBootAction::Run,
                summary: EjectedFirmwareProgram::blink().summary(),
            }
        );
    }

    #[test]
    fn validated_ejected_blink_program_carries_module_and_boot_plan() {
        let validated = validate_ejected_firmware_program(
            EjectedFirmwareProgram::blink(),
            CapabilitySet::blink_mvp(),
            16,
        )
        .unwrap();

        assert_eq!(validated.module.code.len(), 26);
        assert_eq!(validated.module.max_stack, 4);
        assert_eq!(
            validated.boot_plan,
            EjectedBootPlan {
                action: EjectedBootAction::Run,
                summary: EjectedFirmwareProgram::blink().summary(),
            }
        );
    }

    #[test]
    fn firmware_smoke_error_kind_labels_are_stable() {
        assert_eq!(FirmwareSmokeErrorKind::Module.as_str(), "module");
        assert_eq!(FirmwareSmokeErrorKind::Validate.as_str(), "validate");
        assert_eq!(FirmwareSmokeErrorKind::Runtime.as_str(), "runtime");
        assert_eq!(
            FirmwareSmokeErrorKind::RequiredCapabilities.as_str(),
            "required_capabilities"
        );
        assert_eq!(
            FirmwareSmokeErrorKind::UnsupportedProgramFormat.as_str(),
            "unsupported_program_format"
        );
        assert_eq!(
            FirmwareSmokeErrorKind::InvalidBootPolicy.as_str(),
            "invalid_boot_policy"
        );
        assert_eq!(
            FirmwareSmokeErrorKind::ArtifactMetadataMismatch.as_str(),
            "artifact_metadata_mismatch"
        );
        assert_eq!(
            FirmwareSmokeErrorKind::ArtifactCrcMismatch.as_str(),
            "artifact_crc_mismatch"
        );
        assert_eq!(
            FirmwareSmokeErrorKind::ArtifactRequiredCapabilitiesMismatch.as_str(),
            "artifact_required_capabilities_mismatch"
        );
    }

    #[test]
    fn rejects_validated_ejected_boot_plan_without_required_capability() {
        let error =
            validate_ejected_boot_plan(EjectedFirmwareProgram::blink(), CapabilitySet::empty(), 16)
                .unwrap_err();

        assert_eq!(
            error,
            FirmwareSmokeError::Validate(ValidateError::UnsupportedCapability(CAP_GPIO_OPEN))
        );
        assert_eq!(error.kind(), FirmwareSmokeErrorKind::Validate);
    }

    #[test]
    fn rejects_validated_ejected_boot_plan_when_board_stack_is_too_small() {
        let error = validate_ejected_boot_plan(
            EjectedFirmwareProgram::blink(),
            CapabilitySet::blink_mvp(),
            3,
        )
        .unwrap_err();

        assert_eq!(
            error,
            FirmwareSmokeError::Validate(ValidateError::DeclaredStackTooLarge)
        );
        assert_eq!(error.kind(), FirmwareSmokeErrorKind::Validate);
    }

    #[test]
    fn rejects_ejected_boot_plan_for_unsupported_program_format() {
        let mut program = EjectedFirmwareProgram::blink();
        program.program_format = 0xFF;

        let error = ejected_boot_plan(program).unwrap_err();

        assert_eq!(error, FirmwareSmokeError::UnsupportedProgramFormat(0xFF));
        assert_eq!(
            error.kind(),
            FirmwareSmokeErrorKind::UnsupportedProgramFormat
        );
    }

    #[test]
    fn rejects_ejected_boot_plan_for_invalid_boot_policy() {
        let mut program = EjectedFirmwareProgram::blink();
        program.boot_policy = 0xFF;

        let error = ejected_boot_plan(program).unwrap_err();

        assert_eq!(error, FirmwareSmokeError::InvalidBootPolicy(0xFF));
        assert_eq!(error.kind(), FirmwareSmokeErrorKind::InvalidBootPolicy);
    }

    #[test]
    fn rejects_ejected_boot_plan_for_crc_mismatch() {
        let mut program = EjectedFirmwareProgram::blink();
        program.module_crc32 ^= 1;

        let error = ejected_boot_plan(program).unwrap_err();

        assert_eq!(error, FirmwareSmokeError::ArtifactCrcMismatch);
        assert_eq!(error.kind(), FirmwareSmokeErrorKind::ArtifactCrcMismatch);
    }

    #[test]
    fn rejects_ejected_artifact_metadata_mismatch() {
        let mut program = EjectedFirmwareProgram::blink();
        program.max_stack = program.max_stack.saturating_add(1);

        let error = validate_ejected_program(program, CapabilitySet::blink_mvp(), 16).unwrap_err();

        assert_eq!(error, FirmwareSmokeError::ArtifactMetadataMismatch);
        assert_eq!(
            error.kind(),
            FirmwareSmokeErrorKind::ArtifactMetadataMismatch
        );
    }

    #[test]
    fn rejects_ejected_artifact_crc_mismatch() {
        let mut program = EjectedFirmwareProgram::blink();
        program.module_crc32 ^= 1;

        let error = validate_ejected_program(program, CapabilitySet::blink_mvp(), 16).unwrap_err();

        assert_eq!(error, FirmwareSmokeError::ArtifactCrcMismatch);
        assert_eq!(error.kind(), FirmwareSmokeErrorKind::ArtifactCrcMismatch);
    }

    #[test]
    fn rejects_ejected_artifact_required_capabilities_mismatch() {
        let mut program = EjectedFirmwareProgram::blink();
        program.required_capabilities = &[CAP_GPIO_OPEN, CAP_TIME_SLEEP_MS];

        let error = validate_ejected_program(program, CapabilitySet::blink_mvp(), 16).unwrap_err();

        assert_eq!(
            error,
            FirmwareSmokeError::ArtifactRequiredCapabilitiesMismatch
        );
        assert_eq!(
            error.kind(),
            FirmwareSmokeErrorKind::ArtifactRequiredCapabilitiesMismatch
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
    fn ejected_boot_cycle_reports_checked_run_outcome() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let boot_run = run_ejected_boot_program_checked_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        )
        .unwrap();

        assert_eq!(
            boot_run.boot_plan,
            EjectedBootPlan {
                action: EjectedBootAction::Run,
                summary: EjectedFirmwareProgram::blink().summary(),
            }
        );
        assert!(boot_run.ran());
        let report = boot_run.report.unwrap();
        assert_eq!(
            boot_run.summary(),
            EjectedBootRunSummary {
                boot_plan: EjectedBootPlan {
                    action: EjectedBootAction::Run,
                    summary: EjectedFirmwareProgram::blink().summary(),
                },
                run_status: Some(report.status),
                instructions_executed: Some(report.instructions_executed),
                open_handles: Some(report.open_handles),
            }
        );
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
    fn ejected_boot_cycle_reports_checked_store_only_skip() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);
        let mut program = EjectedFirmwareProgram::blink();
        program.boot_policy = BOOT_STORE_ONLY;

        let boot_run = run_ejected_boot_program_checked_once(
            &mut runtime,
            program,
            EJECTED_INSTRUCTION_BUDGET,
        )
        .unwrap();

        assert_eq!(
            boot_run.boot_plan,
            EjectedBootPlan {
                action: EjectedBootAction::StoreOnly,
                summary: program.summary(),
            }
        );
        assert!(!boot_run.ran());
        assert_eq!(
            boot_run.summary(),
            EjectedBootRunSummary {
                boot_plan: EjectedBootPlan {
                    action: EjectedBootAction::StoreOnly,
                    summary: program.summary(),
                },
                run_status: None,
                instructions_executed: None,
                open_handles: None,
            }
        );
        assert_eq!(boot_run.report, None);
        assert!(runtime.hal().events.is_empty());
    }

    #[test]
    fn ejected_boot_diagnostic_summarizes_completed_run() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let diagnostic = diagnose_ejected_boot_program_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        );

        assert!(diagnostic.completed());
        assert!(!diagnostic.failed());
        assert_eq!(
            diagnostic,
            EjectedBootDiagnostic::Completed(EjectedBootRunSummary {
                boot_plan: EjectedBootPlan {
                    action: EjectedBootAction::Run,
                    summary: EjectedFirmwareProgram::blink().summary(),
                },
                run_status: Some(RunStatus::BudgetExceeded),
                instructions_executed: Some(EJECTED_INSTRUCTION_BUDGET),
                open_handles: Some(1),
            })
        );
        assert!(!runtime.hal().events.is_empty());
    }

    #[test]
    fn ejected_boot_diagnostic_status_labels_are_stable() {
        assert_eq!(EjectedBootDiagnosticStatus::Ran.as_str(), "ran");
        assert_eq!(
            EjectedBootDiagnosticStatus::SkippedStoreOnly.as_str(),
            "skipped_store_only"
        );
        assert_eq!(
            EjectedBootDiagnosticStatus::ValidationFailed.as_str(),
            "validation_failed"
        );
        assert_eq!(
            EjectedBootDiagnosticStatus::RuntimeFailed.as_str(),
            "runtime_failed"
        );
    }

    #[test]
    fn ejected_boot_diagnostic_summary_surfaces_completed_run() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let summary = diagnose_ejected_boot_program_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        )
        .summary();

        assert!(summary.completed());
        assert!(!summary.failed());
        assert!(summary.validated());
        assert!(summary.ran());
        assert!(!summary.skipped_store_only());
        assert_eq!(summary.status(), EjectedBootDiagnosticStatus::Ran);
        assert_eq!(summary.status_label(), "ran");
        assert_eq!(summary.error_kind(), None);
        assert_eq!(summary.error_label(), None);
        assert_eq!(
            summary,
            EjectedBootDiagnosticSummary {
                outcome: EjectedBootDiagnosticOutcome::Completed,
                program: EjectedFirmwareProgram::blink().summary(),
                boot_plan: Some(EjectedBootPlan {
                    action: EjectedBootAction::Run,
                    summary: EjectedFirmwareProgram::blink().summary(),
                }),
                run_status: Some(RunStatus::BudgetExceeded),
                instructions_executed: Some(EJECTED_INSTRUCTION_BUDGET),
                open_handles: Some(1),
                error: None,
            }
        );
        assert!(!runtime.hal().events.is_empty());
    }

    #[test]
    fn ejected_boot_diagnostic_summary_status_surfaces_store_only_skip() {
        let hal = FakeHal::new();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);
        let mut program = EjectedFirmwareProgram::blink();
        program.boot_policy = BOOT_STORE_ONLY;

        let summary =
            diagnose_ejected_boot_program_once(&mut runtime, program, EJECTED_INSTRUCTION_BUDGET)
                .summary();

        assert!(summary.completed());
        assert!(!summary.failed());
        assert!(summary.validated());
        assert!(!summary.ran());
        assert!(summary.skipped_store_only());
        assert_eq!(
            summary.status(),
            EjectedBootDiagnosticStatus::SkippedStoreOnly
        );
        assert_eq!(summary.status_label(), "skipped_store_only");
        assert_eq!(summary.error_kind(), None);
        assert_eq!(summary.error_label(), None);
        assert_eq!(
            summary,
            EjectedBootDiagnosticSummary {
                outcome: EjectedBootDiagnosticOutcome::Completed,
                program: program.summary(),
                boot_plan: Some(EjectedBootPlan {
                    action: EjectedBootAction::StoreOnly,
                    summary: program.summary(),
                }),
                run_status: None,
                instructions_executed: None,
                open_handles: None,
                error: None,
            }
        );
        assert!(runtime.hal().events.is_empty());
    }

    #[test]
    fn ejected_boot_diagnostic_keeps_program_summary_on_failure() {
        let hal = FakeHal::with_capabilities(CapabilitySet::empty());
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let diagnostic = diagnose_ejected_boot_program_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        );

        assert!(!diagnostic.completed());
        assert!(diagnostic.failed());
        assert_eq!(
            diagnostic,
            EjectedBootDiagnostic::Failed(EjectedBootFailure {
                program: EjectedFirmwareProgram::blink().summary(),
                boot_plan: None,
                error: FirmwareSmokeError::Validate(ValidateError::UnsupportedCapability(
                    CAP_GPIO_OPEN
                )),
            })
        );
        assert!(runtime.hal().events.is_empty());
    }

    #[test]
    fn ejected_boot_diagnostic_summary_surfaces_validation_failure() {
        let hal = FakeHal::with_capabilities(CapabilitySet::empty());
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let summary = diagnose_ejected_boot_program_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        )
        .summary();

        assert!(!summary.completed());
        assert!(summary.failed());
        assert!(!summary.validated());
        assert!(!summary.ran());
        assert!(!summary.skipped_store_only());
        assert_eq!(
            summary.status(),
            EjectedBootDiagnosticStatus::ValidationFailed
        );
        assert_eq!(summary.status_label(), "validation_failed");
        assert_eq!(summary.error_kind(), Some(FirmwareSmokeErrorKind::Validate));
        assert_eq!(summary.error_label(), Some("validate"));
        assert_eq!(
            summary,
            EjectedBootDiagnosticSummary {
                outcome: EjectedBootDiagnosticOutcome::Failed,
                program: EjectedFirmwareProgram::blink().summary(),
                boot_plan: None,
                run_status: None,
                instructions_executed: None,
                open_handles: None,
                error: Some(FirmwareSmokeError::Validate(
                    ValidateError::UnsupportedCapability(CAP_GPIO_OPEN)
                )),
            }
        );
        assert!(runtime.hal().events.is_empty());
    }

    #[test]
    fn ejected_boot_diagnostic_keeps_boot_plan_on_runtime_failure() {
        let hal = FakeHal::failing_gpio_open();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let diagnostic = diagnose_ejected_boot_program_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        );

        assert!(diagnostic.failed());
        match diagnostic {
            EjectedBootDiagnostic::Failed(failure) => {
                assert_eq!(failure.program, EjectedFirmwareProgram::blink().summary());
                assert_eq!(
                    failure.boot_plan,
                    Some(EjectedBootPlan {
                        action: EjectedBootAction::Run,
                        summary: EjectedFirmwareProgram::blink().summary(),
                    })
                );
                match failure.error {
                    FirmwareSmokeError::Runtime(error) => {
                        assert_eq!(error.kind, RuntimeErrorKind::BoardFault);
                    }
                    other => panic!("expected runtime board fault, got {other:?}"),
                }
            }
            EjectedBootDiagnostic::Completed(_) => panic!("expected runtime failure"),
        }
        assert!(runtime.hal().events.is_empty());
    }

    #[test]
    fn ejected_boot_diagnostic_summary_surfaces_runtime_failure_plan() {
        let hal = FakeHal::failing_gpio_open();
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);

        let summary = diagnose_ejected_boot_program_once(
            &mut runtime,
            EjectedFirmwareProgram::blink(),
            EJECTED_INSTRUCTION_BUDGET,
        )
        .summary();

        assert!(!summary.completed());
        assert!(summary.failed());
        assert!(summary.validated());
        assert!(!summary.ran());
        assert!(!summary.skipped_store_only());
        assert_eq!(summary.status(), EjectedBootDiagnosticStatus::RuntimeFailed);
        assert_eq!(summary.status_label(), "runtime_failed");
        assert_eq!(summary.error_kind(), Some(FirmwareSmokeErrorKind::Runtime));
        assert_eq!(summary.error_label(), Some("runtime"));
        assert_eq!(summary.outcome, EjectedBootDiagnosticOutcome::Failed);
        assert_eq!(summary.program, EjectedFirmwareProgram::blink().summary());
        assert_eq!(
            summary.boot_plan,
            Some(EjectedBootPlan {
                action: EjectedBootAction::Run,
                summary: EjectedFirmwareProgram::blink().summary(),
            })
        );
        assert_eq!(summary.run_status, None);
        assert_eq!(summary.instructions_executed, None);
        assert_eq!(summary.open_handles, None);
        match summary.error {
            Some(FirmwareSmokeError::Runtime(error)) => {
                assert_eq!(error.kind, RuntimeErrorKind::BoardFault);
            }
            other => panic!("expected runtime board fault, got {other:?}"),
        }
        assert!(runtime.hal().events.is_empty());
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

    #[test]
    fn ejected_boot_cycle_validates_store_only_artifact_before_skip() {
        let hal = FakeHal::with_capabilities(CapabilitySet::empty());
        let mut runtime: Runtime<_, 16, 8> = Runtime::new(hal);
        let mut program = EjectedFirmwareProgram::blink();
        program.boot_policy = BOOT_STORE_ONLY;

        let error =
            run_ejected_boot_program_once(&mut runtime, program, EJECTED_INSTRUCTION_BUDGET)
                .unwrap_err();

        assert_eq!(
            error,
            FirmwareSmokeError::Validate(ValidateError::UnsupportedCapability(CAP_GPIO_OPEN))
        );
        assert!(runtime.hal().events.is_empty());
    }
}
