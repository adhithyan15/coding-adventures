use core::str::FromStr;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::format;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::{String, ToString};
use std::vec::Vec;

use crate::arduino_usb_link::{
    ARDUINO_ARM_AR_ENV_VAR, ARDUINO_ARM_COMPAT_ROOT_ENV_VAR, ARDUINO_ARM_GCC_ENV_VAR,
    ARDUINO_ARM_GXX_ENV_VAR, ARDUINO_CORE_ENV_VAR, ARDUINO_USB_LINK_ENV_VAR, UNO_R4_WIFI_FQBN,
};

pub const SERIAL_USB_SERVER_BIN: &str = "uno-r4-wifi-serialusb-server";
pub const TARGET_TRIPLE: &str = "thumbv7em-none-eabihf";
pub const FIRMWARE_PACKAGE: &str = "board-vm-uno-r4-firmware";
pub const DEFAULT_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_TIMEOUT_MS: u64 = 1_000;
pub const ARDUINO_BOSSAC_PATH_ENV_VAR: &str = "BOARD_VM_UNO_R4_BOSSAC_PATH";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
}

impl CommandSpec {
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn arg_path(mut self, path: &Path) -> Self {
        self.args.push(path.as_os_str().to_os_string());
        self
    }

    pub fn env(mut self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((name.into(), value.into()));
        self
    }

    pub fn env_path(mut self, name: impl Into<OsString>, value: &Path) -> Self {
        self.env
            .push((name.into(), value.as_os_str().to_os_string()));
        self
    }

    pub fn to_command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        for (name, value) in &self.env {
            command.env(name, value);
        }
        command
    }

    pub fn shell_line(&self) -> String {
        let mut parts = Vec::new();
        for (name, value) in &self.env {
            parts.push(format!(
                "{}={}",
                shell_escape(name.as_os_str()),
                shell_escape(value.as_os_str())
            ));
        }
        parts.push(shell_escape(self.program.as_os_str()));
        for arg in &self.args {
            parts.push(shell_escape(arg.as_os_str()));
        }
        parts.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialUsbArtifactOptions {
    pub arduino_core: Option<PathBuf>,
    pub rustc: Option<PathBuf>,
    pub arm_gcc: Option<PathBuf>,
    pub arm_gxx: Option<PathBuf>,
    pub arm_ar: Option<PathBuf>,
    pub arm_compat_root: Option<PathBuf>,
    pub bossac_path: Option<PathBuf>,
    pub target_dir: PathBuf,
    pub objcopy: PathBuf,
    pub arduino_cli: PathBuf,
    pub release: bool,
    pub port: Option<String>,
    pub upload: bool,
    pub smoke: bool,
    pub smoke_baud_rate: u32,
    pub smoke_timeout_ms: u64,
}

impl Default for SerialUsbArtifactOptions {
    fn default() -> Self {
        Self {
            arduino_core: env::var_os(ARDUINO_CORE_ENV_VAR).map(PathBuf::from),
            rustc: env::var_os("RUSTC").map(PathBuf::from),
            arm_gcc: env::var_os(ARDUINO_ARM_GCC_ENV_VAR).map(PathBuf::from),
            arm_gxx: env::var_os(ARDUINO_ARM_GXX_ENV_VAR).map(PathBuf::from),
            arm_ar: env::var_os(ARDUINO_ARM_AR_ENV_VAR).map(PathBuf::from),
            arm_compat_root: env::var_os(ARDUINO_ARM_COMPAT_ROOT_ENV_VAR).map(PathBuf::from),
            bossac_path: env::var_os(ARDUINO_BOSSAC_PATH_ENV_VAR).map(PathBuf::from),
            target_dir: PathBuf::from("target"),
            objcopy: PathBuf::from("llvm-objcopy"),
            arduino_cli: PathBuf::from("arduino-cli"),
            release: true,
            port: None,
            upload: false,
            smoke: false,
            smoke_baud_rate: DEFAULT_BAUD_RATE,
            smoke_timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

impl SerialUsbArtifactOptions {
    pub fn fill_host_defaults(&mut self) {
        if self.rustc.is_none() {
            self.rustc = resolve_rustup_stable_rustc();
        }

        if self.arm_gcc.is_none() {
            self.arm_gcc = find_on_path("arm-none-eabi-gcc");
        }
        if self.arm_gxx.is_none() {
            self.arm_gxx = find_on_path("arm-none-eabi-g++");
        }
        if self.arm_ar.is_none() {
            self.arm_ar = find_on_path("arm-none-eabi-ar");
        }

        if self.arm_compat_root.is_none() {
            self.arm_compat_root = self
                .arduino_core
                .as_deref()
                .and_then(arduino_arm_compat_root_from_core);
        }
    }

    pub fn profile_dir(&self) -> &'static str {
        if self.release {
            "release"
        } else {
            "debug"
        }
    }

    pub fn elf_path(&self) -> PathBuf {
        self.target_dir
            .join(TARGET_TRIPLE)
            .join(self.profile_dir())
            .join(SERIAL_USB_SERVER_BIN)
    }

    pub fn bin_path(&self) -> PathBuf {
        self.elf_path().with_extension("bin")
    }

    pub fn command_plan(&self) -> Result<Vec<CommandSpec>, ArtifactCliError> {
        let mut commands = Vec::new();
        commands.push(self.build_command());
        commands.push(self.objcopy_command());

        if self.upload {
            commands.push(self.upload_command()?);
        }

        if self.smoke {
            commands.push(self.smoke_command()?);
        }

        Ok(commands)
    }

    pub fn build_command(&self) -> CommandSpec {
        let mut command = CommandSpec::new("rustup")
            .arg("run")
            .arg("stable")
            .arg("cargo")
            .arg("build")
            .arg("--target")
            .arg(TARGET_TRIPLE)
            .arg("-p")
            .arg(FIRMWARE_PACKAGE)
            .arg("--bin")
            .arg(SERIAL_USB_SERVER_BIN)
            .env(ARDUINO_USB_LINK_ENV_VAR, "1");

        if self.release {
            command = command.arg("--release");
        }

        if let Some(core) = &self.arduino_core {
            command = command.env_path(ARDUINO_CORE_ENV_VAR, core);
        }
        if let Some(rustc) = &self.rustc {
            command = command.env_path("RUSTC", rustc);
        }
        if let Some(gcc) = &self.arm_gcc {
            command = command.env_path(ARDUINO_ARM_GCC_ENV_VAR, gcc);
        }
        if let Some(gxx) = &self.arm_gxx {
            command = command.env_path(ARDUINO_ARM_GXX_ENV_VAR, gxx);
        }
        if let Some(ar) = &self.arm_ar {
            command = command.env_path(ARDUINO_ARM_AR_ENV_VAR, ar);
        }
        if let Some(root) = &self.arm_compat_root {
            command = command.env_path(ARDUINO_ARM_COMPAT_ROOT_ENV_VAR, root);
        }

        command
    }

    pub fn objcopy_command(&self) -> CommandSpec {
        CommandSpec::new(self.objcopy.as_os_str().to_os_string())
            .arg("-O")
            .arg("binary")
            .arg_path(&self.elf_path())
            .arg_path(&self.bin_path())
    }

    pub fn upload_command(&self) -> Result<CommandSpec, ArtifactCliError> {
        let port = self.port.as_ref().ok_or(ArtifactCliError::MissingPort(
            "--port is required with --upload",
        ))?;

        let mut command = CommandSpec::new(self.arduino_cli.as_os_str().to_os_string())
            .arg("upload")
            .arg("-p")
            .arg(port.as_str())
            .arg("-b")
            .arg(UNO_R4_WIFI_FQBN)
            .arg("-i")
            .arg_path(&self.bin_path());

        if let Some(path) = &self.bossac_path {
            command = command.arg("--upload-property").arg(format!(
                "runtime.tools.bossac-1.9.1-arduino5.path={}",
                path.display()
            ));
        }

        Ok(command)
    }

    pub fn smoke_command(&self) -> Result<CommandSpec, ArtifactCliError> {
        let port = self.port.as_ref().ok_or(ArtifactCliError::MissingPort(
            "--port is required with --smoke",
        ))?;

        Ok(self.smoke_command_for_port(port))
    }

    pub fn smoke_command_for_port(&self, port: &str) -> CommandSpec {
        CommandSpec::new("cargo")
            .arg("run")
            .arg("-p")
            .arg("board-vm-cli")
            .arg("--bin")
            .arg("board-vm")
            .arg("--")
            .arg("smoke")
            .arg("--port")
            .arg(port)
            .arg("--baud")
            .arg(self.smoke_baud_rate.to_string())
            .arg("--timeout-ms")
            .arg(self.smoke_timeout_ms.to_string())
    }

    pub fn smoke_port_after_upload_output(
        &self,
        upload_output: &str,
    ) -> Result<String, ArtifactCliError> {
        if let Some(port) = parse_new_upload_port(upload_output) {
            return Ok(port);
        }

        self.port.clone().ok_or(ArtifactCliError::MissingPort(
            "--port is required with --smoke",
        ))
    }
}

pub fn parse_new_upload_port(output: &str) -> Option<String> {
    output.lines().rev().find_map(|line| {
        let (_, port) = line.split_once("New upload port:")?;
        let port = port
            .trim()
            .split_once(" (")
            .map_or_else(|| port.trim(), |(port, _)| port.trim());

        (!port.is_empty()).then(|| port.to_string())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialUsbArtifactRun {
    pub options: SerialUsbArtifactOptions,
    pub print_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactInvocation {
    Run(SerialUsbArtifactRun),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactCliError {
    MissingValue(&'static str),
    MissingPort(&'static str),
    InvalidNumber { option: &'static str, value: String },
    UnknownOption(String),
}

impl fmt::Display for ArtifactCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue(option) => write!(f, "missing value for {option}"),
            Self::MissingPort(message) => write!(f, "{message}"),
            Self::InvalidNumber { option, value } => {
                write!(f, "invalid numeric value for {option}: {value}")
            }
            Self::UnknownOption(option) => write!(f, "unknown option: {option}"),
        }
    }
}

impl std::error::Error for ArtifactCliError {}

pub fn parse_artifact_args<I, S>(
    args: I,
    defaults: SerialUsbArtifactOptions,
) -> Result<ArtifactInvocation, ArtifactCliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let mut options = defaults;
    let mut print_only = false;

    while let Some(option) = args.next() {
        match option.as_str() {
            "--help" | "-h" => return Ok(ArtifactInvocation::Help),
            "--core" => {
                options.arduino_core = Some(PathBuf::from(next_value(&mut args, "--core")?));
            }
            "--rustc" => {
                options.rustc = Some(PathBuf::from(next_value(&mut args, "--rustc")?));
            }
            "--arm-toolchain-bin" => {
                let bin_dir = PathBuf::from(next_value(&mut args, "--arm-toolchain-bin")?);
                options.arm_gcc = Some(bin_dir.join(executable_name("arm-none-eabi-gcc")));
                options.arm_gxx = Some(bin_dir.join(executable_name("arm-none-eabi-g++")));
                options.arm_ar = Some(bin_dir.join(executable_name("arm-none-eabi-ar")));
            }
            "--arm-gcc" => {
                options.arm_gcc = Some(PathBuf::from(next_value(&mut args, "--arm-gcc")?));
            }
            "--arm-gxx" => {
                options.arm_gxx = Some(PathBuf::from(next_value(&mut args, "--arm-gxx")?));
            }
            "--arm-ar" => {
                options.arm_ar = Some(PathBuf::from(next_value(&mut args, "--arm-ar")?));
            }
            "--arm-compat-root" => {
                options.arm_compat_root =
                    Some(PathBuf::from(next_value(&mut args, "--arm-compat-root")?));
            }
            "--target-dir" => {
                options.target_dir = PathBuf::from(next_value(&mut args, "--target-dir")?);
            }
            "--objcopy" => {
                options.objcopy = PathBuf::from(next_value(&mut args, "--objcopy")?);
            }
            "--arduino-cli" => {
                options.arduino_cli = PathBuf::from(next_value(&mut args, "--arduino-cli")?);
            }
            "--bossac-path" => {
                options.bossac_path = Some(PathBuf::from(next_value(&mut args, "--bossac-path")?));
            }
            "--port" | "-p" => {
                options.port = Some(next_value(&mut args, "--port")?);
            }
            "--baud" => {
                options.smoke_baud_rate = parse_number(next_value(&mut args, "--baud")?, "--baud")?;
            }
            "--timeout-ms" => {
                options.smoke_timeout_ms =
                    parse_number(next_value(&mut args, "--timeout-ms")?, "--timeout-ms")?;
            }
            "--debug" => options.release = false,
            "--upload" => options.upload = true,
            "--smoke" => options.smoke = true,
            "--print-only" => print_only = true,
            other => return Err(ArtifactCliError::UnknownOption(other.to_string())),
        }
    }

    Ok(ArtifactInvocation::Run(SerialUsbArtifactRun {
        options,
        print_only,
    }))
}

pub fn usage() -> &'static str {
    "usage:\n  uno-r4-wifi-serialusb-artifact [--core <arduino-core>] [--rustc <path>] [--arm-toolchain-bin <dir>] [--arm-gcc <path>] [--arm-gxx <path>] [--arm-ar <path>] [--arm-compat-root <dir>] [--target-dir <dir>] [--objcopy <path>] [--arduino-cli <path>] [--bossac-path <dir>] [--print-only] [--upload --port <bootloader-or-runtime-port>] [--smoke --port <serial-port>] [--baud <rate>] [--timeout-ms <ms>]"
}

pub fn resolve_rust_llvm_objcopy() -> Option<PathBuf> {
    let rustc_output = Command::new("rustup")
        .arg("which")
        .arg("--toolchain")
        .arg("stable")
        .arg("rustc")
        .output()
        .ok()?;
    if !rustc_output.status.success() {
        return None;
    }

    let rustc_path = String::from_utf8_lossy(&rustc_output.stdout)
        .trim()
        .to_string();
    let version_output = Command::new(&rustc_path).arg("-vV").output().ok()?;
    if !version_output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&version_output.stdout);
    let host = detect_host_triple(&version)?;
    let objcopy = rust_llvm_objcopy_path_from_rustc(Path::new(&rustc_path), &host)?;
    objcopy.exists().then_some(objcopy)
}

pub fn resolve_rustup_stable_rustc() -> Option<PathBuf> {
    let output = Command::new("rustup")
        .arg("which")
        .arg("--toolchain")
        .arg("stable")
        .arg("rustc")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

pub fn find_on_path(program: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|dir| dir.join(executable_name(program)))
        .find(|candidate| candidate.exists())
}

pub fn arduino_arm_compat_root_from_core(core_dir: &Path) -> Option<PathBuf> {
    let packages_dir = core_dir.ancestors().nth(4)?;
    let root = packages_dir
        .join("arduino/tools/arm-none-eabi-gcc")
        .join("7-2017q4");
    root.exists().then_some(root)
}

pub fn detect_host_triple(rustc_verbose_version: &str) -> Option<String> {
    rustc_verbose_version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(ToString::to_string)
}

pub fn rust_llvm_objcopy_path_from_rustc(rustc_path: &Path, host: &str) -> Option<PathBuf> {
    let toolchain_root = rustc_path.parent()?.parent()?;
    Some(
        toolchain_root
            .join("lib")
            .join("rustlib")
            .join(host)
            .join("bin")
            .join(executable_name("llvm-objcopy")),
    )
}

fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, ArtifactCliError>
where
    I: Iterator<Item = String>,
{
    args.next().ok_or(ArtifactCliError::MissingValue(option))
}

fn parse_number<T>(value: String, option: &'static str) -> Result<T, ArtifactCliError>
where
    T: FromStr,
{
    value
        .parse::<T>()
        .map_err(|_| ArtifactCliError::InvalidNumber { option, value })
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn shell_escape(value: &OsStr) -> String {
    let value = value.to_string_lossy();
    if value.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '=' | '+')
    }) {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn options() -> SerialUsbArtifactOptions {
        SerialUsbArtifactOptions {
            arduino_core: Some(PathBuf::from("/arduino/renesas_uno/1.5.3")),
            rustc: Some(PathBuf::from("/rustup/stable/bin/rustc")),
            arm_gcc: Some(PathBuf::from("/opt/toolchain/bin/arm-none-eabi-gcc")),
            arm_gxx: Some(PathBuf::from("/opt/toolchain/bin/arm-none-eabi-g++")),
            arm_ar: Some(PathBuf::from("/opt/toolchain/bin/arm-none-eabi-ar")),
            arm_compat_root: Some(PathBuf::from("/arduino/tools/arm-none-eabi-gcc/7-2017q4")),
            bossac_path: Some(PathBuf::from("/tmp/arduino-bossa/bin")),
            target_dir: PathBuf::from("target"),
            objcopy: PathBuf::from("/rust/llvm-objcopy"),
            arduino_cli: PathBuf::from("arduino-cli"),
            release: true,
            port: None,
            upload: false,
            smoke: false,
            smoke_baud_rate: DEFAULT_BAUD_RATE,
            smoke_timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    fn args(command: &CommandSpec) -> Vec<String> {
        command
            .args
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn plans_release_build_with_the_arduino_usb_link_enabled() {
        let commands = options().command_plan().unwrap();
        let build = &commands[0];

        assert_eq!(build.program, OsString::from("rustup"));
        assert_eq!(
            args(build),
            [
                "run",
                "stable",
                "cargo",
                "build",
                "--target",
                TARGET_TRIPLE,
                "-p",
                FIRMWARE_PACKAGE,
                "--bin",
                SERIAL_USB_SERVER_BIN,
                "--release",
            ]
        );
        assert!(build.env.contains(&(
            OsString::from(ARDUINO_USB_LINK_ENV_VAR),
            OsString::from("1")
        )));
        assert!(build.env.contains(&(
            OsString::from(ARDUINO_CORE_ENV_VAR),
            OsString::from("/arduino/renesas_uno/1.5.3")
        )));
        assert!(build.env.contains(&(
            OsString::from("RUSTC"),
            OsString::from("/rustup/stable/bin/rustc")
        )));
        assert!(build.env.contains(&(
            OsString::from(ARDUINO_ARM_GCC_ENV_VAR),
            OsString::from("/opt/toolchain/bin/arm-none-eabi-gcc")
        )));
        assert!(build.env.contains(&(
            OsString::from(ARDUINO_ARM_GXX_ENV_VAR),
            OsString::from("/opt/toolchain/bin/arm-none-eabi-g++")
        )));
        assert!(build.env.contains(&(
            OsString::from(ARDUINO_ARM_AR_ENV_VAR),
            OsString::from("/opt/toolchain/bin/arm-none-eabi-ar")
        )));
        assert!(build.env.contains(&(
            OsString::from(ARDUINO_ARM_COMPAT_ROOT_ENV_VAR),
            OsString::from("/arduino/tools/arm-none-eabi-gcc/7-2017q4")
        )));
    }

    #[test]
    fn plans_objcopy_from_serialusb_elf_to_bootloader_bin() {
        let commands = options().command_plan().unwrap();
        let objcopy = &commands[1];

        assert_eq!(objcopy.program, OsString::from("/rust/llvm-objcopy"));
        assert_eq!(
            args(objcopy),
            [
                "-O",
                "binary",
                "target/thumbv7em-none-eabihf/release/uno-r4-wifi-serialusb-server",
                "target/thumbv7em-none-eabihf/release/uno-r4-wifi-serialusb-server.bin",
            ]
        );
    }

    #[test]
    fn upload_and_smoke_steps_target_the_uno_r4_wifi_port() {
        let mut options = options();
        options.port = Some("/dev/cu.usbmodem-test".to_string());
        options.upload = true;
        options.smoke = true;

        let commands = options.command_plan().unwrap();

        assert_eq!(
            args(&commands[2]),
            [
                "upload",
                "-p",
                "/dev/cu.usbmodem-test",
                "-b",
                UNO_R4_WIFI_FQBN,
                "-i",
                "target/thumbv7em-none-eabihf/release/uno-r4-wifi-serialusb-server.bin",
                "--upload-property",
                "runtime.tools.bossac-1.9.1-arduino5.path=/tmp/arduino-bossa/bin",
            ]
        );
        assert_eq!(
            args(&commands[3]),
            [
                "run",
                "-p",
                "board-vm-cli",
                "--bin",
                "board-vm",
                "--",
                "smoke",
                "--port",
                "/dev/cu.usbmodem-test",
                "--baud",
                "115200",
                "--timeout-ms",
                "1000",
            ]
        );
    }

    #[test]
    fn parses_new_upload_port_from_arduino_cli_output() {
        assert_eq!(
            parse_new_upload_port(
                "Sketch uses 30720 bytes.\nNew upload port: /dev/cu.usbmodem1101 (serial)\n"
            ),
            Some("/dev/cu.usbmodem1101".to_string())
        );
        assert_eq!(
            parse_new_upload_port(
                "New upload port: /dev/cu.usbmodem9070692469E42 (serial)\n\
                 New upload port: /dev/cu.usbmodem1101 (serial)\n"
            ),
            Some("/dev/cu.usbmodem1101".to_string())
        );
        assert_eq!(parse_new_upload_port("No new serial port found."), None);
    }

    #[test]
    fn smoke_after_upload_prefers_the_runtime_port_reported_by_arduino_cli() {
        let mut options = options();
        options.port = Some("/dev/cu.usbmodem9070692469E42".to_string());

        let port = options
            .smoke_port_after_upload_output(
                "Resetting board...\nNew upload port: /dev/cu.usbmodem1101 (serial)\n",
            )
            .unwrap();
        let smoke = options.smoke_command_for_port(&port);

        assert_eq!(port, "/dev/cu.usbmodem1101");
        assert_eq!(
            args(&smoke),
            [
                "run",
                "-p",
                "board-vm-cli",
                "--bin",
                "board-vm",
                "--",
                "smoke",
                "--port",
                "/dev/cu.usbmodem1101",
                "--baud",
                "115200",
                "--timeout-ms",
                "1000",
            ]
        );
    }

    #[test]
    fn smoke_after_upload_falls_back_to_the_requested_port_without_a_handoff() {
        let mut options = options();
        options.port = Some("/dev/cu.usbmodem1101".to_string());

        assert_eq!(
            options.smoke_port_after_upload_output("No new upload port found."),
            Ok("/dev/cu.usbmodem1101".to_string())
        );
    }

    #[test]
    fn upload_requires_a_port() {
        let mut options = options();
        options.upload = true;

        assert_eq!(
            options.command_plan().unwrap_err(),
            ArtifactCliError::MissingPort("--port is required with --upload")
        );
    }

    #[test]
    fn parses_print_upload_and_smoke_options() {
        let invocation = parse_artifact_args(
            [
                "--print-only",
                "--core",
                "/core",
                "--rustc",
                "/rustc",
                "--arm-toolchain-bin",
                "/native/bin",
                "--arm-compat-root",
                "/compat",
                "--target-dir",
                "out",
                "--objcopy",
                "/bin/llvm-objcopy",
                "--bossac-path",
                "/bossac/bin",
                "--port",
                "/dev/cu.usbmodem-test",
                "--upload",
                "--smoke",
                "--baud",
                "57600",
                "--timeout-ms",
                "250",
            ],
            SerialUsbArtifactOptions::default(),
        )
        .unwrap();

        let ArtifactInvocation::Run(run) = invocation else {
            panic!("expected run invocation");
        };

        assert!(run.print_only);
        assert_eq!(run.options.arduino_core, Some(PathBuf::from("/core")));
        assert_eq!(run.options.rustc, Some(PathBuf::from("/rustc")));
        assert_eq!(
            run.options.arm_gcc,
            Some(PathBuf::from("/native/bin/arm-none-eabi-gcc"))
        );
        assert_eq!(
            run.options.arm_gxx,
            Some(PathBuf::from("/native/bin/arm-none-eabi-g++"))
        );
        assert_eq!(
            run.options.arm_ar,
            Some(PathBuf::from("/native/bin/arm-none-eabi-ar"))
        );
        assert_eq!(run.options.arm_compat_root, Some(PathBuf::from("/compat")));
        assert_eq!(run.options.target_dir, PathBuf::from("out"));
        assert_eq!(run.options.objcopy, PathBuf::from("/bin/llvm-objcopy"));
        assert_eq!(run.options.bossac_path, Some(PathBuf::from("/bossac/bin")));
        assert_eq!(run.options.port, Some("/dev/cu.usbmodem-test".to_string()));
        assert!(run.options.upload);
        assert!(run.options.smoke);
        assert_eq!(run.options.smoke_baud_rate, 57_600);
        assert_eq!(run.options.smoke_timeout_ms, 250);
    }

    #[test]
    fn derives_rust_llvm_objcopy_from_rustc_path_and_host() {
        assert_eq!(
            detect_host_triple("rustc 1.90.0\nhost: aarch64-apple-darwin\nrelease: 1.90.0\n"),
            Some("aarch64-apple-darwin".to_string())
        );
        assert_eq!(
            rust_llvm_objcopy_path_from_rustc(
                Path::new("/toolchains/stable-aarch64-apple-darwin/bin/rustc"),
                "aarch64-apple-darwin",
            )
            .unwrap(),
            PathBuf::from(
                "/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy"
            )
        );
    }

    #[test]
    fn derives_arduino_arm_compat_root_from_core_dir() {
        assert_eq!(
            arduino_arm_compat_root_from_core(Path::new(
                "/Arduino15/packages/arduino/hardware/renesas_uno/1.5.3"
            )),
            None
        );
    }
}
