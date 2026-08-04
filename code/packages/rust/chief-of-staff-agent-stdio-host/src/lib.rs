//! Subprocess host for D18 Level 4 any-language agents.
//!
//! The runtime composes the pure JSON-lines codec with injected channel
//! endpoints. Package verification, sandbox policy, restart policy, and
//! timeout supervision remain outside this package.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_agent_stdio_protocol::{
    decode_response_line, encode_input_line, AgentInput, AgentResponse, ProtocolError,
    MAX_LINE_BYTES,
};
use chief_of_staff_channel_crypto::Sequence;
use chief_of_staff_channel_endpoints::{
    ChannelEndpointError, MessageId, Originator, PublishedMessage, Receiver,
};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};

/// Shell-free executable and arguments for one Level 4 agent session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentCommand {
    program: String,
    args: Vec<String>,
}

impl AgentCommand {
    /// Validate an exact program path/name and argument vector.
    pub fn new<I, S>(program: impl Into<String>, args: I) -> Result<Self, AgentStdioError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let command = Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        };
        if command.program.is_empty() || command.program.contains('\0') {
            return Err(AgentStdioError::InvalidCommand(
                "program must be non-empty and contain no NUL",
            ));
        }
        if command.args.iter().any(|argument| argument.contains('\0')) {
            return Err(AgentStdioError::InvalidCommand(
                "arguments must contain no NUL",
            ));
        }
        Ok(command)
    }

    /// Borrow the exact executable path or name.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Borrow the exact argument vector.
    pub fn args(&self) -> &[String] {
        &self.args
    }
}

/// Error from process launch, JSON-lines transport, or protocol validation.
#[derive(Debug)]
pub enum AgentStdioError {
    /// An executable or argument violated the shell-free command contract.
    InvalidCommand(&'static str),
    /// A caller attempted to reuse an invalidated process session.
    SessionClosed,
    /// The operating system rejected process launch.
    Spawn(std::io::Error),
    /// A child pipe was unexpectedly unavailable.
    MissingPipe(&'static str),
    /// Writing or flushing the child stdin pipe failed.
    Write(std::io::Error),
    /// Reading the child stdout pipe failed.
    Read(std::io::Error),
    /// The child closed stdout before returning a complete response line.
    UnexpectedEof(Option<ExitStatus>),
    /// The child had already exited before the next request.
    ProcessExited(ExitStatus),
    /// A response exceeded the bounded line size.
    LineTooLong,
    /// A response line was not UTF-8.
    NonUtf8,
    /// The shared Level 4 codec rejected input encoding or agent output.
    Protocol(ProtocolError),
}

impl Display for AgentStdioError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand(message) => write!(formatter, "invalid agent command: {message}"),
            Self::SessionClosed => formatter.write_str("agent process session is closed"),
            Self::Spawn(error) => write!(formatter, "agent process launch failed: {error}"),
            Self::MissingPipe(pipe) => write!(formatter, "agent process omitted its {pipe} pipe"),
            Self::Write(error) => write!(formatter, "agent stdin write failed: {error}"),
            Self::Read(error) => write!(formatter, "agent stdout read failed: {error}"),
            Self::UnexpectedEof(Some(status)) => {
                write!(
                    formatter,
                    "agent stdout closed before a response ({status})"
                )
            }
            Self::UnexpectedEof(None) => {
                formatter.write_str("agent stdout closed before a response")
            }
            Self::ProcessExited(status) => {
                write!(
                    formatter,
                    "agent process exited before the next request ({status})"
                )
            }
            Self::LineTooLong => formatter.write_str("agent response line exceeds protocol limit"),
            Self::NonUtf8 => formatter.write_str("agent response line is not UTF-8"),
            Self::Protocol(error) => write!(formatter, "agent protocol failed: {error}"),
        }
    }
}

impl Error for AgentStdioError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn(error) | Self::Write(error) | Self::Read(error) => Some(error),
            Self::Protocol(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ProtocolError> for AgentStdioError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

/// Response provider used by the channel orchestration layer.
pub trait AgentResponder {
    /// Produce exactly one response for the current verified input.
    fn respond(&mut self, input: &AgentInput) -> Result<AgentResponse, AgentStdioError>;
}

/// One owned long-lived child process speaking `chief-agent-stdio-v1`.
pub struct StdioAgentSession {
    child: Child,
    writer: Option<BufWriter<ChildStdin>>,
    reader: Option<BufReader<ChildStdout>>,
    closed: bool,
}

impl StdioAgentSession {
    /// Launch one already-authorized executable with piped stdin/stdout.
    ///
    /// No shell is invoked. The child inherits stderr so the caller's runtime
    /// can route uncaught failures into its normal logging path.
    pub fn spawn(command: &AgentCommand) -> Result<Self, AgentStdioError> {
        let mut child = Command::new(command.program())
            .args(command.args())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(AgentStdioError::Spawn)?;
        let stdin = child
            .stdin
            .take()
            .ok_or(AgentStdioError::MissingPipe("stdin"));
        let stdout = child
            .stdout
            .take()
            .ok_or(AgentStdioError::MissingPipe("stdout"));
        match (stdin, stdout) {
            (Ok(stdin), Ok(stdout)) => Ok(Self {
                child,
                writer: Some(BufWriter::new(stdin)),
                reader: Some(BufReader::new(stdout)),
                closed: false,
            }),
            (stdin, stdout) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(stdin
                    .err()
                    .or_else(|| stdout.err())
                    .expect("one pipe failed"))
            }
        }
    }

    /// Return the operating-system child identifier while the session owns it.
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }

    /// Kill and reap the owned child. Repeating shutdown is harmless.
    pub fn shutdown(&mut self) {
        if self.closed {
            return;
        }
        self.writer.take();
        self.reader.take();
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
        self.closed = true;
    }

    fn fail<T>(&mut self, error: AgentStdioError) -> Result<T, AgentStdioError> {
        self.shutdown();
        Err(error)
    }

    fn preflight(&mut self) -> Result<(), AgentStdioError> {
        if self.closed {
            return Err(AgentStdioError::SessionClosed);
        }
        match self.child.try_wait() {
            Ok(Some(status)) => self.fail(AgentStdioError::ProcessExited(status)),
            Ok(None) => Ok(()),
            Err(error) => self.fail(AgentStdioError::Read(error)),
        }
    }
}

impl AgentResponder for StdioAgentSession {
    fn respond(&mut self, input: &AgentInput) -> Result<AgentResponse, AgentStdioError> {
        self.preflight()?;
        let line = encode_input_line(input)?;
        let write_result = match self.writer.as_mut() {
            Some(writer) => writer
                .write_all(line.as_bytes())
                .and_then(|_| writer.flush()),
            None => return self.fail(AgentStdioError::MissingPipe("stdin")),
        };
        if let Err(error) = write_result {
            return self.fail(AgentStdioError::Write(error));
        }

        let mut response_bytes = Vec::new();
        let read_result = match self.reader.as_mut() {
            Some(reader) => reader
                .take((MAX_LINE_BYTES + 1) as u64)
                .read_until(b'\n', &mut response_bytes),
            None => return self.fail(AgentStdioError::MissingPipe("stdout")),
        };
        let bytes_read = match read_result {
            Ok(bytes_read) => bytes_read,
            Err(error) => return self.fail(AgentStdioError::Read(error)),
        };
        if bytes_read == 0 {
            let status = self.child.try_wait().ok().flatten();
            return self.fail(AgentStdioError::UnexpectedEof(status));
        }
        if response_bytes.len() > MAX_LINE_BYTES {
            return self.fail(AgentStdioError::LineTooLong);
        }
        let response_line = match std::str::from_utf8(&response_bytes) {
            Ok(line) => line,
            Err(_) => return self.fail(AgentStdioError::NonUtf8),
        };
        match decode_response_line(response_line, input.message_id()) {
            Ok(response) => Ok(response),
            Err(error) => self.fail(AgentStdioError::Protocol(error)),
        }
    }
}

impl Drop for StdioAgentSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Result of polling one Level 4 input message.
#[derive(Clone, Debug)]
pub enum LevelFourRunOutcome {
    /// No verified channel message was waiting.
    Idle,
    /// One response was published and its input was acknowledged.
    Processed {
        /// Input message completed by the subprocess.
        input_message_id: MessageId,
        /// Sequence returned by the monotonic acknowledgement.
        acknowledged_through: Sequence,
        /// Receipt for the published subprocess response.
        output: PublishedMessage,
    },
}

/// Failure from a subprocess response or injected channel endpoint.
#[derive(Debug)]
pub enum LevelFourHostError {
    /// The subprocess session or protocol failed.
    Agent(AgentStdioError),
    /// A receive, publish, or acknowledge operation failed.
    Channel(ChannelEndpointError),
}

impl Display for LevelFourHostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agent(error) => write!(formatter, "Level 4 agent failed: {error}"),
            Self::Channel(error) => write!(formatter, "Level 4 channel failed: {error}"),
        }
    }
}

impl Error for LevelFourHostError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Agent(error) => Some(error),
            Self::Channel(error) => Some(error),
        }
    }
}

impl From<AgentStdioError> for LevelFourHostError {
    fn from(error: AgentStdioError) -> Self {
        Self::Agent(error)
    }
}

impl From<ChannelEndpointError> for LevelFourHostError {
    fn from(error: ChannelEndpointError) -> Self {
        Self::Channel(error)
    }
}

/// Level 4 process adapter bound to one response provider.
pub struct LevelFourHost<'a> {
    responder: &'a mut dyn AgentResponder,
}

impl<'a> LevelFourHost<'a> {
    /// Bind one responder after package verification and sandbox selection.
    pub fn new(responder: &'a mut dyn AgentResponder) -> Self {
        Self { responder }
    }

    /// Poll and process at most one verified channel message.
    ///
    /// Ordering is receive, subprocess response, publish, acknowledge. Any
    /// error before the last step therefore leaves the durable input cursor
    /// unchanged for replay.
    pub fn run_once(
        &mut self,
        receiver: &mut dyn Receiver,
        originator: &dyn Originator,
    ) -> Result<LevelFourRunOutcome, LevelFourHostError> {
        let Some(message) = receiver.receive(1)?.into_iter().next() else {
            return Ok(LevelFourRunOutcome::Idle);
        };
        let message_id = format_uuid(message.message_id.as_bytes());
        let input = AgentInput::new(
            message_id.clone(),
            format_uuid(&receiver.channel_id().0),
            message.sequence.0,
            message.timestamp_ns,
            message.content_type,
            message.payload,
        )
        .map_err(AgentStdioError::Protocol)?;
        let response = self.responder.respond(&input)?;
        if response.input_message_id() != message_id {
            return Err(AgentStdioError::Protocol(ProtocolError::CorrelationMismatch).into());
        }
        let output = originator.publish(response.payload(), response.content_type())?;
        let acknowledged_through = receiver.acknowledge(message.message_id)?;
        Ok(LevelFourRunOutcome::Processed {
            input_message_id: message.message_id,
            acknowledged_through,
            output,
        })
    }
}

fn format_uuid(bytes: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_and_uuid_format_are_stable() {
        let command = AgentCommand::new("python", ["agent.py", "--once"]).unwrap();
        assert_eq!(command.program(), "python");
        assert_eq!(command.args(), ["agent.py", "--once"]);
        assert!(AgentCommand::new("", std::iter::empty::<String>()).is_err());
        assert!(AgentCommand::new("python", ["bad\0arg"]).is_err());
        assert_eq!(
            format_uuid(&[
                0x01, 0x98, 0xab, 0xcd, 0xef, 0x01, 0x72, 0x34, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23,
                0x45, 0x67,
            ]),
            "0198abcd-ef01-7234-89ab-cdef01234567"
        );
    }
}
