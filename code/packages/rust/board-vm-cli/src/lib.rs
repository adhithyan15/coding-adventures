use core::fmt;
use std::fs;
use std::io::{BufRead, Write};
use std::time::Duration;

use board_vm_client::{
    BoardDescriptorInfo, BoardVmClient, ClientError, HelloAckInfo, RawFrameTransport,
    RunReportInfo, RunValue, UploadReport,
};
use board_vm_eject::{
    build_blink_eject_artifact, write_embedded_rust_constants,
    EjectOptions as ArtifactEjectOptions, RustConstNames, DEFAULT_BOOT_POLICY, DEFAULT_EJECT_SLOT,
};
use board_vm_host::{
    write_blink_module, write_gpio_read_module, write_time_now_module, BlinkProgram,
    GpioReadProgram, TimeNowProgram, BLINK_MODULE_LEN, GPIO_READ_MODULE_LEN, TIME_NOW_MODULE_LEN,
};
use board_vm_protocol::{BOOT_RUN_AT_BOOT, BOOT_RUN_IF_NO_HOST, BOOT_STORE_ONLY};
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
    EjectBlink(EjectBlinkOptions),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EjectBlinkOptions {
    pub output: String,
    pub program_id: u16,
    pub slot: u8,
    pub boot_policy: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplCommand {
    Empty,
    Help,
    Hello,
    Caps,
    UploadBlink,
    UploadGpioRead {
        pin: u8,
        mode: ReplGpioReadMode,
    },
    UploadTimeNow,
    Run {
        instruction_budget: Option<u32>,
    },
    Blink {
        instruction_budget: Option<u32>,
    },
    GpioRead {
        pin: u8,
        mode: ReplGpioReadMode,
        instruction_budget: Option<u32>,
    },
    TimeNow {
        instruction_budget: Option<u32>,
    },
    Stop,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplGpioReadMode {
    Input,
    InputPullup,
    InputPulldown,
}

impl ReplGpioReadMode {
    fn program(self, pin: u8) -> GpioReadProgram {
        match self {
            Self::Input => GpioReadProgram::input(pin),
            Self::InputPullup => GpioReadProgram::input_pullup(pin),
            Self::InputPulldown => GpioReadProgram::input_pulldown(pin),
        }
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
    UnexpectedArgument(String),
    UnknownOption(String),
    Serial(String),
    Io(String),
    Client(ClientError),
    Eject(String),
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
            Self::Eject(error) => write!(f, "eject error: {error}"),
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
        "eject" => parse_eject_args(args),
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

fn parse_eject_args<I>(mut args: I) -> Result<CliCommand, CliError>
where
    I: Iterator<Item = String>,
{
    let target = args
        .next()
        .ok_or(CliError::MissingRequired("eject target"))?;
    match target.as_str() {
        "blink" => parse_eject_blink_args(args),
        other => Err(CliError::UnknownCommand(format!("eject {other}"))),
    }
}

fn parse_eject_blink_args<I>(mut args: I) -> Result<CliCommand, CliError>
where
    I: Iterator<Item = String>,
{
    let mut output = None;
    let mut program_id = DEFAULT_PROGRAM_ID;
    let mut slot = DEFAULT_EJECT_SLOT;
    let mut boot_policy = DEFAULT_BOOT_POLICY;

    while let Some(option) = args.next() {
        match option.as_str() {
            "--out" | "--output" | "-o" => output = Some(next_value(&mut args, "--out")?),
            "--program-id" => {
                program_id = parse_number(next_value(&mut args, "--program-id")?, "--program-id")?
            }
            "--slot" => slot = parse_number(next_value(&mut args, "--slot")?, "--slot")?,
            "--boot-policy" => {
                boot_policy = parse_boot_policy(next_value(&mut args, "--boot-policy")?)?
            }
            other => return Err(CliError::UnknownOption(other.to_owned())),
        }
    }

    let output = output.ok_or(CliError::MissingRequired("--out"))?;
    Ok(CliCommand::EjectBlink(EjectBlinkOptions {
        output,
        program_id,
        slot,
        boot_policy,
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
        "upload-gpio-read" | "upload-gpio.read" => {
            let (pin, mode, _) = parse_repl_gpio_read_args(&mut words, "upload-gpio-read", false)?;
            Ok(ReplCommand::UploadGpioRead { pin, mode })
        }
        "upload-time-now" | "upload-time.now" => Ok(ReplCommand::UploadTimeNow),
        "run" => Ok(ReplCommand::Run {
            instruction_budget: optional_repl_budget(words.next(), "run")?,
        }),
        "blink" => Ok(ReplCommand::Blink {
            instruction_budget: optional_repl_budget(words.next(), "blink")?,
        }),
        "gpio-read" | "gpio.read" => {
            let (pin, mode, instruction_budget) =
                parse_repl_gpio_read_args(&mut words, "gpio-read", true)?;
            Ok(ReplCommand::GpioRead {
                pin,
                mode,
                instruction_budget,
            })
        }
        "time-now" | "time.now" | "now" => Ok(ReplCommand::TimeNow {
            instruction_budget: optional_repl_budget(words.next(), "time-now")?,
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

fn parse_boot_policy(value: String) -> Result<u8, CliError> {
    match value.as_str() {
        "store-only" => Ok(BOOT_STORE_ONLY),
        "run-at-boot" => Ok(BOOT_RUN_AT_BOOT),
        "run-if-no-host" => Ok(BOOT_RUN_IF_NO_HOST),
        _ => parse_number(value, "--boot-policy"),
    }
}

fn optional_repl_budget(
    value: Option<&str>,
    command: &'static str,
) -> Result<Option<u32>, CliError> {
    value
        .map(|value| parse_number(value.to_owned(), command))
        .transpose()
}

fn parse_repl_gpio_read_args<'a, I>(
    words: &mut I,
    command: &'static str,
    allow_budget: bool,
) -> Result<(u8, ReplGpioReadMode, Option<u32>), CliError>
where
    I: Iterator<Item = &'a str>,
{
    let pin = words
        .next()
        .ok_or(CliError::MissingRequired("gpio-read pin"))
        .and_then(|value| parse_number(value.to_owned(), command))?;
    let mut mode = ReplGpioReadMode::Input;
    let mut instruction_budget = None;

    if let Some(value) = words.next() {
        if allow_budget && integer_literal(value) {
            instruction_budget = Some(parse_number(value.to_owned(), command)?);
        } else {
            mode = parse_repl_gpio_read_mode(value)?;
        }
    }

    if allow_budget && instruction_budget.is_none() {
        instruction_budget = optional_repl_budget(words.next(), command)?;
    }

    Ok((pin, mode, instruction_budget))
}

fn parse_repl_gpio_read_mode(value: &str) -> Result<ReplGpioReadMode, CliError> {
    match value {
        "input" | "in" => Ok(ReplGpioReadMode::Input),
        "pullup" | "input-pullup" | "input_pullup" => Ok(ReplGpioReadMode::InputPullup),
        "pulldown" | "input-pulldown" | "input_pulldown" => Ok(ReplGpioReadMode::InputPulldown),
        other => Err(CliError::InvalidNumber {
            option: "gpio-read mode",
            value: other.to_owned(),
        }),
    }
}

fn integer_literal(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EjectReport {
    pub output: String,
    pub program_id: u16,
    pub slot: u8,
    pub boot_policy: u8,
    pub module_len: usize,
    pub module_crc32: u32,
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

pub fn run_eject_blink(options: &EjectBlinkOptions) -> Result<EjectReport, CliError> {
    let (source, report) = render_blink_eject(options)?;
    fs::write(&options.output, source)?;
    Ok(report)
}

pub fn render_blink_eject(options: &EjectBlinkOptions) -> Result<(String, EjectReport), CliError> {
    let mut module = [0u8; BLINK_MODULE_LEN];
    let artifact = build_blink_eject_artifact(
        BlinkProgram::onboard_led(),
        ArtifactEjectOptions::new(options.program_id)
            .slot(options.slot)
            .boot_policy(options.boot_policy),
        &mut module,
    )
    .map_err(|error| CliError::Eject(format!("{error:?}")))?;

    let mut source = String::new();
    source.push_str("// Generated by board-vm eject blink.\n");
    source.push_str("// Embed this from a board-specific firmware crate.\n\n");
    write_embedded_rust_constants(&artifact, RustConstNames::board_vm_defaults(), &mut source)
        .map_err(|error| CliError::Eject(format!("{error:?}")))?;

    let report = EjectReport {
        output: options.output.clone(),
        program_id: artifact.program_id,
        slot: artifact.slot,
        boot_policy: artifact.boot_policy,
        module_len: artifact.module_len(),
        module_crc32: artifact.module_crc32,
    };
    Ok((source, report))
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
        ReplCommand::UploadGpioRead { pin, mode } => {
            let upload = upload_gpio_read(client, state.program_id, pin, mode)?;
            write_upload(output, &upload)?;
        }
        ReplCommand::UploadTimeNow => {
            let upload = upload_time_now(client, state.program_id)?;
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
        ReplCommand::GpioRead {
            pin,
            mode,
            instruction_budget,
        } => {
            let upload = upload_gpio_read(client, state.program_id, pin, mode)?;
            write_upload(output, &upload)?;
            let run = client.run_background(
                state.program_id,
                instruction_budget.unwrap_or(state.instruction_budget),
            )?;
            write_run(output, &run)?;
        }
        ReplCommand::TimeNow { instruction_budget } => {
            let upload = upload_time_now(client, state.program_id)?;
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

fn upload_gpio_read<T>(
    client: &mut BoardVmClient<T, 512, 768, 768>,
    program_id: u16,
    pin: u8,
    mode: ReplGpioReadMode,
) -> Result<UploadReport, CliError>
where
    T: RawFrameTransport,
{
    let mut module = [0u8; GPIO_READ_MODULE_LEN];
    let module_len = write_gpio_read_module(mode.program(pin), &mut module)
        .map_err(|source| CliError::Client(source.into()))?;
    client
        .upload_program(program_id, &module[..module_len])
        .map_err(CliError::from)
}

fn upload_time_now<T>(
    client: &mut BoardVmClient<T, 512, 768, 768>,
    program_id: u16,
) -> Result<UploadReport, CliError>
where
    T: RawFrameTransport,
{
    let mut module = [0u8; TIME_NOW_MODULE_LEN];
    let module_len = write_time_now_module(TimeNowProgram::new(), &mut module)
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
    if run.returns.is_empty() {
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
    } else {
        writeln!(
            output,
            "run program_id={} status={:?} instructions={} elapsed_ms={} stack_depth={} open_handles={} returns=[{}]",
            run.program_id,
            run.status,
            run.instructions_executed,
            run.elapsed_ms,
            run.stack_depth,
            run.open_handles,
            format_run_values(&run.returns)
        )?;
    }
    Ok(())
}

fn format_run_values(values: &[RunValue]) -> String {
    let mut out = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format_run_value(value));
    }
    out
}

fn format_run_value(value: &RunValue) -> String {
    match value {
        RunValue::Unit => "unit".to_owned(),
        RunValue::Bool(value) => value.to_string(),
        RunValue::U8(value) => value.to_string(),
        RunValue::U16(value) => value.to_string(),
        RunValue::U32(value) => value.to_string(),
        RunValue::I16(value) => value.to_string(),
        RunValue::Handle(value) => format!("handle:{value}"),
        RunValue::Bytes(value) => format!("bytes:{}b", value.len()),
        RunValue::String(value) => format!("string:{value}"),
    }
}

fn write_repl_help<W>(output: &mut W) -> Result<(), CliError>
where
    W: Write,
{
    writeln!(
        output,
        "commands: hello, caps, upload-blink, upload-gpio-read <pin> [mode], upload-time-now, run [budget], blink [budget], gpio-read <pin> [mode] [budget], time-now [budget], stop, help, quit"
    )?;
    Ok(())
}

pub fn list_ports() -> Result<Vec<SerialPortInfo>, CliError> {
    available_ports().map_err(|error| CliError::Serial(error.to_string()))
}

pub fn usage() -> &'static str {
    "usage:\n  board-vm list-ports\n  board-vm smoke --port <path> [--baud <rate>] [--timeout-ms <ms>] [--program-id <id>] [--budget <instructions>] [--host-nonce <u32>]\n  board-vm repl --port <path> [--baud <rate>] [--timeout-ms <ms>] [--program-id <id>] [--budget <instructions>] [--host-nonce <u32>]\n  board-vm eject blink --out <path> [--program-id <id>] [--slot <slot>] [--boot-policy store-only|run-at-boot|run-if-no-host|<u8>]"
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
    fn parses_eject_blink_defaults() {
        let command = parse_args(["eject", "blink", "--out", "/tmp/blink.rs"]).unwrap();

        assert_eq!(
            command,
            CliCommand::EjectBlink(EjectBlinkOptions {
                output: "/tmp/blink.rs".to_owned(),
                program_id: DEFAULT_PROGRAM_ID,
                slot: DEFAULT_EJECT_SLOT,
                boot_policy: DEFAULT_BOOT_POLICY,
            })
        );
    }

    #[test]
    fn parses_eject_blink_overrides() {
        let command = parse_args([
            "eject",
            "blink",
            "--output",
            "blink.rs",
            "--program-id",
            "9",
            "--slot",
            "3",
            "--boot-policy",
            "run-at-boot",
        ])
        .unwrap();

        assert_eq!(
            command,
            CliCommand::EjectBlink(EjectBlinkOptions {
                output: "blink.rs".to_owned(),
                program_id: 9,
                slot: 3,
                boot_policy: BOOT_RUN_AT_BOOT,
            })
        );

        let numeric =
            parse_args(["eject", "blink", "--out", "blink.rs", "--boot-policy", "0"]).unwrap();
        assert_eq!(
            numeric,
            CliCommand::EjectBlink(EjectBlinkOptions {
                output: "blink.rs".to_owned(),
                program_id: DEFAULT_PROGRAM_ID,
                slot: DEFAULT_EJECT_SLOT,
                boot_policy: BOOT_STORE_ONLY,
            })
        );
    }

    #[test]
    fn renders_blink_eject_constants() {
        let options = EjectBlinkOptions {
            output: "blink.rs".to_owned(),
            program_id: 7,
            slot: 2,
            boot_policy: BOOT_RUN_IF_NO_HOST,
        };

        let (source, report) = render_blink_eject(&options).unwrap();

        assert_eq!(
            report,
            EjectReport {
                output: "blink.rs".to_owned(),
                program_id: 7,
                slot: 2,
                boot_policy: BOOT_RUN_IF_NO_HOST,
                module_len: BLINK_MODULE_LEN,
                module_crc32: 0xBAD6_949E,
            }
        );
        assert!(source.starts_with("// Generated by board-vm eject blink."));
        assert!(source.contains("pub const BOARD_VM_PROGRAM_ID: u16 = 7;"));
        assert!(source.contains("pub const BOARD_VM_PROGRAM_SLOT: u8 = 2;"));
        assert!(source.contains("pub const BOARD_VM_BOOT_POLICY: u8 = 2;"));
        assert!(source.contains("pub const BOARD_VM_PROGRAM: [u8; 36] = ["));
    }

    #[test]
    fn run_eject_blink_writes_file() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "board-vm-cli-eject-blink-{}.rs",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let options = EjectBlinkOptions {
            output: path.display().to_string(),
            program_id: DEFAULT_PROGRAM_ID,
            slot: DEFAULT_EJECT_SLOT,
            boot_policy: DEFAULT_BOOT_POLICY,
        };

        let report = run_eject_blink(&options).unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(report.output, options.output);
        assert!(source.contains("pub const BOARD_VM_PROGRAM_ID: u16 = 1;"));
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
            parse_repl_line("upload-gpio-read 13 pullup").unwrap(),
            ReplCommand::UploadGpioRead {
                pin: 13,
                mode: ReplGpioReadMode::InputPullup,
            }
        );
        assert_eq!(
            parse_repl_line("upload-time-now").unwrap(),
            ReplCommand::UploadTimeNow
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
        assert_eq!(
            parse_repl_line("gpio-read 13 pullup 24").unwrap(),
            ReplCommand::GpioRead {
                pin: 13,
                mode: ReplGpioReadMode::InputPullup,
                instruction_budget: Some(24),
            }
        );
        assert_eq!(
            parse_repl_line("gpio-read 13 24").unwrap(),
            ReplCommand::GpioRead {
                pin: 13,
                mode: ReplGpioReadMode::Input,
                instruction_budget: Some(24),
            }
        );
        assert_eq!(
            parse_repl_line("time-now 24").unwrap(),
            ReplCommand::TimeNow {
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
        assert_eq!(
            parse_args(["eject"]).unwrap_err(),
            CliError::MissingRequired("eject target")
        );
        assert_eq!(
            parse_args(["eject", "blink"]).unwrap_err(),
            CliError::MissingRequired("--out")
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
        assert_eq!(
            parse_args(["eject", "wat"]).unwrap_err(),
            CliError::UnknownCommand("eject wat".to_owned())
        );
        assert_eq!(
            parse_args(["eject", "blink", "--out", "blink.rs", "--wat"]).unwrap_err(),
            CliError::UnknownOption("--wat".to_owned())
        );
    }
}
