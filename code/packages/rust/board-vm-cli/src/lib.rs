use core::fmt;
use std::time::Duration;

use board_vm_client::{
    BoardDescriptorInfo, BoardVmClient, ClientError, HelloAckInfo, RunReportInfo,
};
use board_vm_host::{write_blink_module, BlinkProgram, BLINK_MODULE_LEN};
use board_vm_serial::{
    available_ports, BoardSerialTransport, SerialConfig, SerialPortInfo, SerialTransportError,
    DEFAULT_BAUD_RATE, DEFAULT_TIMEOUT_MS,
};

pub const DEFAULT_PROGRAM_ID: u16 = 1;
pub const DEFAULT_INSTRUCTION_BUDGET: u32 = 12;
pub const DEFAULT_HOST_NONCE: u32 = 0xB0A2_D001;
pub const DEFAULT_HOST_NAME: &str = "board-vm-cli";
pub const DEFAULT_OPEN_SETTLE_MS: u64 = 250;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    ListPorts,
    Smoke(SmokeOptions),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeOptions {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub program_id: u16,
    pub instruction_budget: u32,
    pub host_nonce: u32,
}

impl SmokeOptions {
    pub fn serial_config(&self) -> SerialConfig {
        SerialConfig::new(&self.port)
            .baud_rate(self.baud_rate)
            .timeout(Duration::from_millis(self.timeout_ms))
            .dtr_on_open(true)
            .clear_on_open(true)
            .settle_on_open(Duration::from_millis(DEFAULT_OPEN_SETTLE_MS))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    MissingCommand,
    UnknownCommand(String),
    MissingValue(&'static str),
    MissingRequired(&'static str),
    InvalidNumber {
        option: &'static str,
        value: String,
    },
    UnknownOption(String),
    Serial(String),
    Client(ClientError),
    Smoke {
        stage: SmokeStage,
        source: ClientError,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "missing command"),
            Self::UnknownCommand(command) => write!(f, "unknown command: {command}"),
            Self::MissingValue(option) => write!(f, "missing value for {option}"),
            Self::MissingRequired(option) => write!(f, "missing required option: {option}"),
            Self::InvalidNumber { option, value } => {
                write!(f, "invalid numeric value for {option}: {value}")
            }
            Self::UnknownOption(option) => write!(f, "unknown option: {option}"),
            Self::Serial(error) => write!(f, "serial error: {error}"),
            Self::Client(error) => write!(f, "client error: {error:?}"),
            Self::Smoke { stage, source } => write!(f, "smoke failed during {stage}: {source:?}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<ClientError> for CliError {
    fn from(value: ClientError) -> Self {
        Self::Client(value)
    }
}

impl From<SerialTransportError> for CliError {
    fn from(value: SerialTransportError) -> Self {
        Self::Serial(format!("{value:?}"))
    }
}

pub fn parse_args<I, S>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let Some(command) = args.next() else {
        return Err(CliError::MissingCommand);
    };

    match command.as_str() {
        "list-ports" => Ok(CliCommand::ListPorts),
        "smoke" => parse_smoke_args(args),
        "help" | "--help" | "-h" => Ok(CliCommand::Help),
        other => Err(CliError::UnknownCommand(other.to_owned())),
    }
}

fn parse_smoke_args<I>(mut args: I) -> Result<CliCommand, CliError>
where
    I: Iterator<Item = String>,
{
    let mut port = None;
    let mut baud_rate = DEFAULT_BAUD_RATE;
    let mut timeout_ms = DEFAULT_TIMEOUT_MS;
    let mut program_id = DEFAULT_PROGRAM_ID;
    let mut instruction_budget = DEFAULT_INSTRUCTION_BUDGET;
    let mut host_nonce = DEFAULT_HOST_NONCE;

    while let Some(option) = args.next() {
        match option.as_str() {
            "--port" | "-p" => port = Some(next_value(&mut args, "--port")?),
            "--baud" | "-b" => {
                baud_rate = parse_number(next_value(&mut args, "--baud")?, "--baud")?
            }
            "--timeout-ms" => {
                timeout_ms = parse_number(next_value(&mut args, "--timeout-ms")?, "--timeout-ms")?
            }
            "--program-id" => {
                program_id = parse_number(next_value(&mut args, "--program-id")?, "--program-id")?
            }
            "--budget" => {
                instruction_budget = parse_number(next_value(&mut args, "--budget")?, "--budget")?
            }
            "--host-nonce" => {
                host_nonce = parse_number(next_value(&mut args, "--host-nonce")?, "--host-nonce")?
            }
            other => return Err(CliError::UnknownOption(other.to_owned())),
        }
    }

    let port = port.ok_or(CliError::MissingRequired("--port"))?;
    Ok(CliCommand::Smoke(SmokeOptions {
        port,
        baud_rate,
        timeout_ms,
        program_id,
        instruction_budget,
        host_nonce,
    }))
}

fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    args.next().ok_or(CliError::MissingValue(option))
}

fn parse_number<T>(value: String, option: &'static str) -> Result<T, CliError>
where
    T: core::str::FromStr,
{
    value
        .parse::<T>()
        .map_err(|_| CliError::InvalidNumber { option, value })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeReport {
    pub hello: HelloAckInfo,
    pub descriptor: BoardDescriptorInfo,
    pub run: RunReportInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmokeStage {
    Hello,
    Capabilities,
    UploadBlink,
    RunBlink,
}

impl fmt::Display for SmokeStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hello => write!(f, "hello"),
            Self::Capabilities => write!(f, "capabilities"),
            Self::UploadBlink => write!(f, "blink upload"),
            Self::RunBlink => write!(f, "blink run"),
        }
    }
}

pub fn run_smoke(options: &SmokeOptions) -> Result<SmokeReport, CliError> {
    let transport = BoardSerialTransport::<_, 1024>::open(&options.serial_config())?;
    let mut client: BoardVmClient<_, 512, 768, 768> = BoardVmClient::new(transport);
    let hello = client
        .hello_with_name(DEFAULT_HOST_NAME, options.host_nonce)
        .map_err(|source| CliError::Smoke {
            stage: SmokeStage::Hello,
            source,
        })?;
    let descriptor = client.query_caps().map_err(|source| CliError::Smoke {
        stage: SmokeStage::Capabilities,
        source,
    })?;
    let mut module = [0u8; BLINK_MODULE_LEN];
    let module_len =
        write_blink_module(BlinkProgram::onboard_led(), &mut module).map_err(|source| {
            CliError::Smoke {
                stage: SmokeStage::UploadBlink,
                source: source.into(),
            }
        })?;
    client
        .upload_program(options.program_id, &module[..module_len])
        .map_err(|source| CliError::Smoke {
            stage: SmokeStage::UploadBlink,
            source,
        })?;
    let run = client
        .run_background(options.program_id, options.instruction_budget)
        .map_err(|source| CliError::Smoke {
            stage: SmokeStage::RunBlink,
            source,
        })?;
    Ok(SmokeReport {
        hello,
        descriptor,
        run,
    })
}

pub fn list_ports() -> Result<Vec<SerialPortInfo>, CliError> {
    available_ports().map_err(|error| CliError::Serial(error.to_string()))
}

pub fn usage() -> &'static str {
    "usage:\n  board-vm list-ports\n  board-vm smoke --port <path> [--baud <rate>] [--timeout-ms <ms>] [--program-id <id>] [--budget <instructions>] [--host-nonce <u32>]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list_ports_command() {
        assert_eq!(parse_args(["list-ports"]).unwrap(), CliCommand::ListPorts);
    }

    #[test]
    fn parses_smoke_defaults() {
        let command = parse_args(["smoke", "--port", "/dev/cu.usbmodem-test"]).unwrap();

        assert_eq!(
            command,
            CliCommand::Smoke(SmokeOptions {
                port: "/dev/cu.usbmodem-test".to_owned(),
                baud_rate: DEFAULT_BAUD_RATE,
                timeout_ms: DEFAULT_TIMEOUT_MS,
                program_id: DEFAULT_PROGRAM_ID,
                instruction_budget: DEFAULT_INSTRUCTION_BUDGET,
                host_nonce: DEFAULT_HOST_NONCE,
            })
        );
    }

    #[test]
    fn parses_smoke_overrides() {
        let command = parse_args([
            "smoke",
            "--port",
            "COM9",
            "--baud",
            "57600",
            "--timeout-ms",
            "250",
            "--program-id",
            "7",
            "--budget",
            "200",
            "--host-nonce",
            "1234",
        ])
        .unwrap();

        assert_eq!(
            command,
            CliCommand::Smoke(SmokeOptions {
                port: "COM9".to_owned(),
                baud_rate: 57_600,
                timeout_ms: 250,
                program_id: 7,
                instruction_budget: 200,
                host_nonce: 1234,
            })
        );
    }

    #[test]
    fn smoke_serial_config_asserts_dtr_and_clears_stale_bytes() {
        let options = SmokeOptions {
            port: "/dev/cu.usbmodem-test".to_owned(),
            baud_rate: 57_600,
            timeout_ms: 250,
            program_id: 7,
            instruction_budget: 200,
            host_nonce: 1234,
        };

        let config = options.serial_config();

        assert_eq!(config.path, "/dev/cu.usbmodem-test");
        assert_eq!(config.baud_rate, 57_600);
        assert_eq!(config.timeout, Duration::from_millis(250));
        assert_eq!(config.dtr_on_open, Some(true));
        assert!(config.clear_on_open);
        assert_eq!(
            config.settle_on_open,
            Duration::from_millis(DEFAULT_OPEN_SETTLE_MS)
        );
    }

    #[test]
    fn requires_smoke_port() {
        assert_eq!(
            parse_args(["smoke"]).unwrap_err(),
            CliError::MissingRequired("--port")
        );
    }

    #[test]
    fn rejects_unknown_option() {
        assert_eq!(
            parse_args(["smoke", "--port", "COM9", "--wat"]).unwrap_err(),
            CliError::UnknownOption("--wat".to_owned())
        );
    }
}
