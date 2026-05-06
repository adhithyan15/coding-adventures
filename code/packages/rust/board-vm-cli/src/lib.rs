use core::fmt;
use std::io::{BufRead, Write};
use std::time::Duration;

use board_vm_client::{
    BoardDescriptorInfo, BoardVmClient, ClientError, HelloAckInfo, RawFrameTransport,
    RunReportInfo, UploadReport,
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
    Repl(ReplOptions),
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
pub struct ReplOptions {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub program_id: u16,
    pub instruction_budget: u32,
    pub host_nonce: u32,
}

impl ReplOptions {
    pub fn serial_config(&self) -> SerialConfig {
        SerialConfig::new(&self.port)
            .baud_rate(self.baud_rate)
            .timeout(Duration::from_millis(self.timeout_ms))
            .dtr_on_open(true)
            .clear_on_open(true)
            .settle_on_open(Duration::from_millis(DEFAULT_OPEN_SETTLE_MS))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplCommand {
    Empty,
    Help,
    Hello,
    Caps,
    UploadBlink,
    Run { instruction_budget: Option<u32> },
    Blink { instruction_budget: Option<u32> },
    Stop,
    Quit,
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
    UnexpectedArgument(String),
    UnknownOption(String),
    Serial(String),
    Io(String),
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
            Self::UnexpectedArgument(argument) => write!(f, "unexpected argument: {argument}"),
            Self::UnknownOption(option) => write!(f, "unknown option: {option}"),
            Self::Serial(error) => write!(f, "serial error: {error}"),
            Self::Io(error) => write!(f, "io error: {error}"),
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

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
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
        "repl" => parse_repl_args(args),
        "help" | "--help" | "-h" => Ok(CliCommand::Help),
        other => Err(CliError::UnknownCommand(other.to_owned())),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionOptions {
    port: String,
    baud_rate: u32,
    timeout_ms: u64,
    program_id: u16,
    instruction_budget: u32,
    host_nonce: u32,
}

fn parse_smoke_args<I>(mut args: I) -> Result<CliCommand, CliError>
where
    I: Iterator<Item = String>,
{
    let options = parse_session_options(&mut args)?;
    Ok(CliCommand::Smoke(SmokeOptions {
        port: options.port,
        baud_rate: options.baud_rate,
        timeout_ms: options.timeout_ms,
        program_id: options.program_id,
        instruction_budget: options.instruction_budget,
        host_nonce: options.host_nonce,
    }))
}

fn parse_repl_args<I>(mut args: I) -> Result<CliCommand, CliError>
where
    I: Iterator<Item = String>,
{
    let options = parse_session_options(&mut args)?;
    Ok(CliCommand::Repl(ReplOptions {
        port: options.port,
        baud_rate: options.baud_rate,
        timeout_ms: options.timeout_ms,
        program_id: options.program_id,
        instruction_budget: options.instruction_budget,
        host_nonce: options.host_nonce,
    }))
}

fn parse_session_options<I>(args: &mut I) -> Result<SessionOptions, CliError>
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
            "--port" | "-p" => port = Some(next_value(args, "--port")?),
            "--baud" | "-b" => baud_rate = parse_number(next_value(args, "--baud")?, "--baud")?,
            "--timeout-ms" => {
                timeout_ms = parse_number(next_value(args, "--timeout-ms")?, "--timeout-ms")?
            }
            "--program-id" => {
                program_id = parse_number(next_value(args, "--program-id")?, "--program-id")?
            }
            "--budget" => {
                instruction_budget = parse_number(next_value(args, "--budget")?, "--budget")?
            }
            "--host-nonce" => {
                host_nonce = parse_number(next_value(args, "--host-nonce")?, "--host-nonce")?
            }
            other => return Err(CliError::UnknownOption(other.to_owned())),
        }
    }

    let port = port.ok_or(CliError::MissingRequired("--port"))?;
    Ok(SessionOptions {
        port,
        baud_rate,
        timeout_ms,
        program_id,
        instruction_budget,
        host_nonce,
    })
}

pub fn parse_repl_line(line: &str) -> Result<ReplCommand, CliError> {
    let mut words = line.split_whitespace();
    let Some(command) = words.next() else {
        return Ok(ReplCommand::Empty);
    };

    let command = match command {
        "help" | "?" => Ok(ReplCommand::Help),
        "hello" => Ok(ReplCommand::Hello),
        "caps" | "capabilities" => Ok(ReplCommand::Caps),
        "upload-blink" => Ok(ReplCommand::UploadBlink),
        "run" => Ok(ReplCommand::Run {
            instruction_budget: optional_repl_budget(words.next(), "run")?,
        }),
        "blink" => Ok(ReplCommand::Blink {
            instruction_budget: optional_repl_budget(words.next(), "blink")?,
        }),
        "stop" => Ok(ReplCommand::Stop),
        "quit" | "exit" => Ok(ReplCommand::Quit),
        other => Err(CliError::UnknownCommand(other.to_owned())),
    }?;

    if let Some(argument) = words.next() {
        return Err(CliError::UnexpectedArgument(argument.to_owned()));
    }

    Ok(command)
}

fn optional_repl_budget(
    value: Option<&str>,
    command: &'static str,
) -> Result<Option<u32>, CliError> {
    value
        .map(|value| parse_number(value.to_owned(), command))
        .transpose()
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

pub fn run_repl<R, W>(options: &ReplOptions, input: R, mut output: W) -> Result<(), CliError>
where
    R: BufRead,
    W: Write,
{
    let transport = BoardSerialTransport::<_, 1024>::open(&options.serial_config())?;
    let mut client: BoardVmClient<_, 512, 768, 768> = BoardVmClient::new(transport);
    let mut state = ReplState {
        program_id: options.program_id,
        instruction_budget: options.instruction_budget,
        host_nonce: options.host_nonce,
    };

    writeln!(
        output,
        "connected port={} baud={} timeout_ms={}",
        options.port, options.baud_rate, options.timeout_ms
    )?;
    let hello = client.hello_with_name(DEFAULT_HOST_NAME, state.host_nonce)?;
    write_hello(&mut output, &hello)?;
    write_repl_help(&mut output)?;

    run_repl_loop(&mut client, &mut state, input, output)
}

fn run_repl_loop<T, R, W>(
    client: &mut BoardVmClient<T, 512, 768, 768>,
    state: &mut ReplState,
    mut input: R,
    mut output: W,
) -> Result<(), CliError>
where
    T: RawFrameTransport,
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    loop {
        write!(output, "board-vm> ")?;
        output.flush()?;
        line.clear();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        let command = parse_repl_line(&line)?;
        if !run_repl_command(client, state, command, &mut output)? {
            break;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReplState {
    program_id: u16,
    instruction_budget: u32,
    host_nonce: u32,
}

fn run_repl_command<T, W>(
    client: &mut BoardVmClient<T, 512, 768, 768>,
    state: &mut ReplState,
    command: ReplCommand,
    output: &mut W,
) -> Result<bool, CliError>
where
    T: RawFrameTransport,
    W: Write,
{
    match command {
        ReplCommand::Empty => {}
        ReplCommand::Help => write_repl_help(output)?,
        ReplCommand::Hello => {
            let hello = client.hello_with_name(DEFAULT_HOST_NAME, state.host_nonce)?;
            write_hello(output, &hello)?;
        }
        ReplCommand::Caps => {
            let descriptor = client.query_caps()?;
            write_descriptor(output, &descriptor)?;
        }
        ReplCommand::UploadBlink => {
            let upload = upload_blink(client, state.program_id)?;
            write_upload(output, &upload)?;
        }
        ReplCommand::Run { instruction_budget } => {
            let run = client.run_background(
                state.program_id,
                instruction_budget.unwrap_or(state.instruction_budget),
            )?;
            write_run(output, &run)?;
        }
        ReplCommand::Blink { instruction_budget } => {
            let upload = upload_blink(client, state.program_id)?;
            write_upload(output, &upload)?;
            let run = client.run_background(
                state.program_id,
                instruction_budget.unwrap_or(state.instruction_budget),
            )?;
            write_run(output, &run)?;
        }
        ReplCommand::Stop => {
            let run = client.stop()?;
            write_run(output, &run)?;
        }
        ReplCommand::Quit => return Ok(false),
    }
    Ok(true)
}

fn upload_blink<T>(
    client: &mut BoardVmClient<T, 512, 768, 768>,
    program_id: u16,
) -> Result<UploadReport, CliError>
where
    T: RawFrameTransport,
{
    let mut module = [0u8; BLINK_MODULE_LEN];
    let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module)
        .map_err(|source| CliError::Client(source.into()))?;
    client
        .upload_program(program_id, &module[..module_len])
        .map_err(CliError::from)
}

fn write_hello<W>(output: &mut W, hello: &HelloAckInfo) -> Result<(), CliError>
where
    W: Write,
{
    writeln!(
        output,
        "hello board={} runtime={} protocol={} host_nonce=0x{:08X} board_nonce=0x{:08X}",
        hello.board_name,
        hello.runtime_name,
        hello.selected_version,
        hello.host_nonce,
        hello.board_nonce
    )?;
    Ok(())
}

fn write_descriptor<W>(output: &mut W, descriptor: &BoardDescriptorInfo) -> Result<(), CliError>
where
    W: Write,
{
    writeln!(
        output,
        "caps board={} runtime={} max_program_bytes={} stack={} handles={} store={} capabilities={}",
        descriptor.board_id,
        descriptor.runtime_id,
        descriptor.max_program_bytes,
        descriptor.max_stack_values,
        descriptor.max_handles,
        descriptor.supports_store_program,
        descriptor.capabilities.len()
    )?;
    for capability in &descriptor.capabilities {
        writeln!(
            output,
            "cap id=0x{:04X} version={} flags=0x{:04X} name={}",
            capability.id, capability.version, capability.flags, capability.name
        )?;
    }
    Ok(())
}

fn write_upload<W>(output: &mut W, upload: &UploadReport) -> Result<(), CliError>
where
    W: Write,
{
    writeln!(
        output,
        "upload program_id={} bytes={} crc32=0x{:08X}",
        upload.program_id, upload.total_len, upload.program_crc32
    )?;
    Ok(())
}

fn write_run<W>(output: &mut W, run: &RunReportInfo) -> Result<(), CliError>
where
    W: Write,
{
    writeln!(
        output,
        "run program_id={} status={:?} instructions={} elapsed_ms={} stack_depth={} open_handles={}",
        run.program_id,
        run.status,
        run.instructions_executed,
        run.elapsed_ms,
        run.stack_depth,
        run.open_handles
    )?;
    Ok(())
}

fn write_repl_help<W>(output: &mut W) -> Result<(), CliError>
where
    W: Write,
{
    writeln!(
        output,
        "commands: hello, caps, upload-blink, run [budget], blink [budget], stop, help, quit"
    )?;
    Ok(())
}

pub fn list_ports() -> Result<Vec<SerialPortInfo>, CliError> {
    available_ports().map_err(|error| CliError::Serial(error.to_string()))
}

pub fn usage() -> &'static str {
    "usage:\n  board-vm list-ports\n  board-vm smoke --port <path> [--baud <rate>] [--timeout-ms <ms>] [--program-id <id>] [--budget <instructions>] [--host-nonce <u32>]\n  board-vm repl --port <path> [--baud <rate>] [--timeout-ms <ms>] [--program-id <id>] [--budget <instructions>] [--host-nonce <u32>]"
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
    fn parses_repl_defaults() {
        let command = parse_args(["repl", "--port", "/dev/cu.usbmodem-test"]).unwrap();

        assert_eq!(
            command,
            CliCommand::Repl(ReplOptions {
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
    fn parses_repl_overrides() {
        let command = parse_args([
            "repl",
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
            CliCommand::Repl(ReplOptions {
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
    fn repl_serial_config_asserts_dtr_and_clears_stale_bytes() {
        let options = ReplOptions {
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
    fn parses_repl_commands() {
        assert_eq!(parse_repl_line("").unwrap(), ReplCommand::Empty);
        assert_eq!(parse_repl_line("help").unwrap(), ReplCommand::Help);
        assert_eq!(parse_repl_line("?").unwrap(), ReplCommand::Help);
        assert_eq!(parse_repl_line("hello").unwrap(), ReplCommand::Hello);
        assert_eq!(parse_repl_line("caps").unwrap(), ReplCommand::Caps);
        assert_eq!(
            parse_repl_line("upload-blink").unwrap(),
            ReplCommand::UploadBlink
        );
        assert_eq!(
            parse_repl_line("run").unwrap(),
            ReplCommand::Run {
                instruction_budget: None
            }
        );
        assert_eq!(
            parse_repl_line("run 42").unwrap(),
            ReplCommand::Run {
                instruction_budget: Some(42)
            }
        );
        assert_eq!(
            parse_repl_line("blink 24").unwrap(),
            ReplCommand::Blink {
                instruction_budget: Some(24)
            }
        );
        assert_eq!(parse_repl_line("stop").unwrap(), ReplCommand::Stop);
        assert_eq!(parse_repl_line("quit").unwrap(), ReplCommand::Quit);
        assert_eq!(parse_repl_line("exit").unwrap(), ReplCommand::Quit);
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
        assert_eq!(
            parse_args(["repl"]).unwrap_err(),
            CliError::MissingRequired("--port")
        );
    }

    #[test]
    fn rejects_unknown_option() {
        assert_eq!(
            parse_args(["smoke", "--port", "COM9", "--wat"]).unwrap_err(),
            CliError::UnknownOption("--wat".to_owned())
        );
        assert_eq!(
            parse_args(["repl", "--port", "COM9", "--wat"]).unwrap_err(),
            CliError::UnknownOption("--wat".to_owned())
        );
        assert_eq!(
            parse_repl_line("wat").unwrap_err(),
            CliError::UnknownCommand("wat".to_owned())
        );
        assert_eq!(
            parse_repl_line("run 10 extra").unwrap_err(),
            CliError::UnexpectedArgument("extra".to_owned())
        );
    }
}
