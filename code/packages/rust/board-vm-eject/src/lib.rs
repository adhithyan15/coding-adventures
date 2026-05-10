#![no_std]

#[cfg(test)]
extern crate std;

use core::fmt::{self, Write};

use board_vm_host::{crc32_ieee, write_blink_module, BlinkProgram, HostError};
use board_vm_ir::{parse_module, CAP_GPIO_OPEN, CAP_GPIO_WRITE, CAP_TIME_SLEEP_MS, MODULE_VERSION};
use board_vm_protocol::{ProgramFormat, BOOT_RUN_AT_BOOT, BOOT_RUN_IF_NO_HOST, BOOT_STORE_ONLY};

pub const DEFAULT_EJECT_SLOT: u8 = 0;
pub const DEFAULT_BOOT_POLICY: u8 = BOOT_RUN_IF_NO_HOST;
pub const BLINK_REQUIRED_CAPABILITIES: [u16; 3] =
    [CAP_GPIO_OPEN, CAP_GPIO_WRITE, CAP_TIME_SLEEP_MS];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EjectError {
    Host(HostError),
    InvalidBootPolicy(u8),
    Fmt,
}

impl From<HostError> for EjectError {
    fn from(value: HostError) -> Self {
        Self::Host(value)
    }
}

impl From<fmt::Error> for EjectError {
    fn from(_: fmt::Error) -> Self {
        Self::Fmt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectOptions {
    pub program_id: u16,
    pub slot: u8,
    pub boot_policy: u8,
}

impl EjectOptions {
    pub const fn new(program_id: u16) -> Self {
        Self {
            program_id,
            slot: DEFAULT_EJECT_SLOT,
            boot_policy: DEFAULT_BOOT_POLICY,
        }
    }

    pub const fn slot(mut self, slot: u8) -> Self {
        self.slot = slot;
        self
    }

    pub const fn boot_policy(mut self, boot_policy: u8) -> Self {
        self.boot_policy = boot_policy;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EjectedProgram<'a> {
    pub program_id: u16,
    pub slot: u8,
    pub boot_policy: u8,
    pub format: ProgramFormat,
    pub module_version: u8,
    pub module_flags: u8,
    pub max_stack: u8,
    pub module_crc32: u32,
    pub module: &'a [u8],
    pub required_capabilities: &'static [u16],
}

impl<'a> EjectedProgram<'a> {
    pub const fn module_len(self) -> usize {
        self.module.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustConstNames<'a> {
    pub program_id: &'a str,
    pub slot: &'a str,
    pub boot_policy: &'a str,
    pub program_format: &'a str,
    pub module_version: &'a str,
    pub module_flags: &'a str,
    pub max_stack: &'a str,
    pub module_crc32: &'a str,
    pub module: &'a str,
}

impl<'a> RustConstNames<'a> {
    pub const fn board_vm_defaults() -> Self {
        Self {
            program_id: "BOARD_VM_PROGRAM_ID",
            slot: "BOARD_VM_PROGRAM_SLOT",
            boot_policy: "BOARD_VM_BOOT_POLICY",
            program_format: "BOARD_VM_PROGRAM_FORMAT",
            module_version: "BOARD_VM_MODULE_VERSION",
            module_flags: "BOARD_VM_MODULE_FLAGS",
            max_stack: "BOARD_VM_MODULE_MAX_STACK",
            module_crc32: "BOARD_VM_PROGRAM_CRC32",
            module: "BOARD_VM_PROGRAM",
        }
    }
}

pub fn build_blink_eject_artifact<'a>(
    program: BlinkProgram,
    options: EjectOptions,
    module_out: &'a mut [u8],
) -> Result<EjectedProgram<'a>, EjectError> {
    validate_boot_policy(options.boot_policy)?;

    let module_len = write_blink_module(program, module_out)?;
    let module = &module_out[..module_len];
    build_module_eject_artifact(module, &BLINK_REQUIRED_CAPABILITIES, options)
}

pub fn build_module_eject_artifact<'a>(
    module: &'a [u8],
    required_capabilities: &'static [u16],
    options: EjectOptions,
) -> Result<EjectedProgram<'a>, EjectError> {
    validate_boot_policy(options.boot_policy)?;

    let parsed = parse_module(module).map_err(HostError::from)?;

    Ok(EjectedProgram {
        program_id: options.program_id,
        slot: options.slot,
        boot_policy: options.boot_policy,
        format: ProgramFormat::BvmModule,
        module_version: MODULE_VERSION,
        module_flags: parsed.flags,
        max_stack: parsed.max_stack,
        module_crc32: crc32_ieee(module),
        module,
        required_capabilities,
    })
}

pub fn write_embedded_rust_constants<W>(
    artifact: &EjectedProgram<'_>,
    names: RustConstNames<'_>,
    out: &mut W,
) -> Result<(), EjectError>
where
    W: Write,
{
    writeln!(
        out,
        "pub const {}: u16 = {};",
        names.program_id, artifact.program_id
    )?;
    writeln!(out, "pub const {}: u8 = {};", names.slot, artifact.slot)?;
    writeln!(
        out,
        "pub const {}: u8 = {};",
        names.boot_policy, artifact.boot_policy
    )?;
    writeln!(
        out,
        "pub const {}: u8 = {};",
        names.program_format,
        artifact.format.as_u8()
    )?;
    writeln!(
        out,
        "pub const {}: u8 = {};",
        names.module_version, artifact.module_version
    )?;
    writeln!(
        out,
        "pub const {}: u8 = 0x{:02X};",
        names.module_flags, artifact.module_flags
    )?;
    writeln!(
        out,
        "pub const {}: u8 = {};",
        names.max_stack, artifact.max_stack
    )?;
    writeln!(
        out,
        "pub const {}: u32 = 0x{:08X};",
        names.module_crc32, artifact.module_crc32
    )?;
    writeln!(
        out,
        "pub const {}: [u8; {}] = [",
        names.module,
        artifact.module.len()
    )?;

    for chunk in artifact.module.chunks(12) {
        write!(out, "    ")?;
        for byte in chunk {
            write!(out, "0x{byte:02X}, ")?;
        }
        writeln!(out)?;
    }

    writeln!(out, "];")?;
    Ok(())
}

fn validate_boot_policy(boot_policy: u8) -> Result<(), EjectError> {
    match boot_policy {
        BOOT_STORE_ONLY | BOOT_RUN_AT_BOOT | BOOT_RUN_IF_NO_HOST => Ok(()),
        other => Err(EjectError::InvalidBootPolicy(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_host::{BLINK_MODULE_LEN, DEFAULT_PROGRAM_ID};
    use board_vm_ir::ModuleError;
    use std::string::String;

    #[test]
    fn builds_blink_eject_artifact() {
        let mut module = [0u8; BLINK_MODULE_LEN];

        let artifact = build_blink_eject_artifact(
            BlinkProgram::onboard_led(),
            EjectOptions::new(DEFAULT_PROGRAM_ID).slot(2),
            &mut module,
        )
        .unwrap();

        assert_eq!(artifact.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(artifact.slot, 2);
        assert_eq!(artifact.boot_policy, BOOT_RUN_IF_NO_HOST);
        assert_eq!(artifact.format, ProgramFormat::BvmModule);
        assert_eq!(artifact.module_version, MODULE_VERSION);
        assert_eq!(artifact.max_stack, 4);
        assert_eq!(artifact.module_len(), BLINK_MODULE_LEN);
        assert_eq!(artifact.module_crc32, crc32_ieee(artifact.module));
        assert_eq!(artifact.required_capabilities, BLINK_REQUIRED_CAPABILITIES);
        parse_module(artifact.module).unwrap();
    }

    #[test]
    fn builds_generic_module_eject_artifact() {
        let mut module = [0u8; BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let module = &module[..module_len];

        let artifact = build_module_eject_artifact(
            module,
            &BLINK_REQUIRED_CAPABILITIES,
            EjectOptions::new(DEFAULT_PROGRAM_ID).slot(9),
        )
        .unwrap();

        assert_eq!(artifact.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(artifact.slot, 9);
        assert_eq!(artifact.format, ProgramFormat::BvmModule);
        assert_eq!(artifact.module_version, MODULE_VERSION);
        assert_eq!(artifact.max_stack, 4);
        assert_eq!(artifact.module_crc32, crc32_ieee(module));
        assert_eq!(artifact.module, module);
        assert_eq!(artifact.required_capabilities, BLINK_REQUIRED_CAPABILITIES);
    }

    #[test]
    fn rejects_invalid_generic_module() {
        let error = build_module_eject_artifact(
            b"not-bvm",
            &BLINK_REQUIRED_CAPABILITIES,
            EjectOptions::new(DEFAULT_PROGRAM_ID),
        )
        .unwrap_err();

        assert_eq!(
            error,
            EjectError::Host(HostError::Module(ModuleError::TooShort))
        );
    }

    #[test]
    fn rejects_invalid_boot_policy_before_writing_module() {
        let mut module = [0xAA; BLINK_MODULE_LEN];

        let error = build_blink_eject_artifact(
            BlinkProgram::onboard_led(),
            EjectOptions::new(DEFAULT_PROGRAM_ID).boot_policy(0xFE),
            &mut module,
        )
        .unwrap_err();

        assert_eq!(error, EjectError::InvalidBootPolicy(0xFE));
        assert!(module.iter().all(|byte| *byte == 0xAA));
    }

    #[test]
    fn writes_embeddable_rust_constants() {
        let mut module = [0u8; BLINK_MODULE_LEN];
        let artifact = build_blink_eject_artifact(
            BlinkProgram::onboard_led(),
            EjectOptions::new(DEFAULT_PROGRAM_ID),
            &mut module,
        )
        .unwrap();
        let mut source = String::new();

        write_embedded_rust_constants(&artifact, RustConstNames::board_vm_defaults(), &mut source)
            .unwrap();

        assert!(source.contains("pub const BOARD_VM_PROGRAM_ID: u16 = 1;"));
        assert!(source.contains("pub const BOARD_VM_PROGRAM_SLOT: u8 = 0;"));
        assert!(source.contains("pub const BOARD_VM_BOOT_POLICY: u8 = 2;"));
        assert!(source.contains("pub const BOARD_VM_PROGRAM_FORMAT: u8 = 1;"));
        assert!(source.contains("pub const BOARD_VM_MODULE_VERSION: u8 = 1;"));
        assert!(source.contains("pub const BOARD_VM_MODULE_FLAGS: u8 = 0x01;"));
        assert!(source.contains("pub const BOARD_VM_MODULE_MAX_STACK: u8 = 4;"));
        assert!(source.contains("pub const BOARD_VM_PROGRAM_CRC32: u32 = 0xBAD6949E;"));
        assert!(source.contains("pub const BOARD_VM_PROGRAM: [u8; 36] = ["));
        assert!(source.contains("0x42, 0x56, 0x4D, 0x31"));
        assert!(source.ends_with("];\n"));
    }
}
