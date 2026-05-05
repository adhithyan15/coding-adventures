use core::str::FromStr;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::format;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::{String, ToString};
use std::vec::Vec;

use crate::arduino_usb_link::{ARDUINO_CORE_ENV_VAR, ARDUINO_USB_LINK_ENV_VAR, UNO_R4_WIFI_FQBN};

pub const SERIAL_USB_SERVER_BIN: &str = "uno-r4-wifi-serialusb-server";
pub const TARGET_TRIPLE: &str = "thumbv7em-none-eabihf";
pub const FIRMWARE_PACKAGE: &str = "board-vm-uno-r4-firmware";
pub const DEFAULT_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_TIMEOUT_MS: u64 = 1_000;

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

    fn build_command(&self) -> CommandSpec {
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

        command
    }

    fn objcopy_command(&self) -> CommandSpec {
        CommandSpec::new(self.objcopy.as_os_str().to_os_string())
            .arg("-O")
            .arg("binary")
            .arg_path(&self.elf_path())
            .arg_path(&self.bin_path())
    }

    fn upload_command(&self) -> Result<CommandSpec, ArtifactCliError> {
        let port = self.port.as_ref().ok_or(ArtifactCliError::MissingPort(
            "--port is required with --upload",
        ))?;

        Ok(
            CommandSpec::new(self.arduino_cli.as_os_str().to_os_string())
                .arg("upload")
                .arg("-p")
                .arg(port.as_str())
                .arg("-b")
                .arg(UNO_R4_WIFI_FQBN)
                .arg("-i")
                .arg_path(&self.bin_path()),
        )
    }

    fn smoke_command(&self) -> Result<CommandSpec, ArtifactCliError> {
        let port = self.port.as_ref().ok_or(ArtifactCliError::MissingPort(
            "--port is required with --smoke",
        ))?;

        Ok(CommandSpec::new("cargo")
            .arg("run")
            .arg("-p")
            .arg("board-vm-cli")
            .arg("--bin")
            .arg("board-vm")
            .arg("--")
            .arg("smoke")
            .arg("--port")
            .arg(port.as_str())
            .arg("--baud")
            .arg(self.smoke_baud_rate.to_string())
            .arg("--timeout-ms")
            .arg(self.smoke_timeout_ms.to_string()))
    }
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
            "--target-dir" => {
                options.target_dir = PathBuf::from(next_value(&mut args, "--target-dir")?);
            }
            "--objcopy" => {
                options.objcopy = PathBuf::from(next_value(&mut args, "--objcopy")?);
            }
            "--arduino-cli" => {
                options.arduino_cli = PathBuf::from(next_value(&mut args, "--arduino-cli")?);
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
    "usage:\n  uno-r4-wifi-serialusb-artifact [--core <arduino-core>] [--target-dir <dir>] [--objcopy <path>] [--print-only] [--upload --port <path>] [--smoke --port <path>] [--baud <rate>] [--timeout-ms <ms>]"
}

pub fn resolve_rust_llvm_objcopy() -> Option<PathBuf> {
    let rustc_output = Command::new("rustup")
        .arg("which")
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
                "--target-dir",
                "out",
                "--objcopy",
                "/bin/llvm-objcopy",
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
        assert_eq!(run.options.target_dir, PathBuf::from("out"));
        assert_eq!(run.options.objcopy, PathBuf::from("/bin/llvm-objcopy"));
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
}
