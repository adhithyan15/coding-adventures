//! Shell-free Tier 3 hardware-key approval for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_tool_api::{ApprovalAssurance, PrivilegeTier};
use chief_of_staff_trust_checker::{
    ApprovalOutcome, ApprovalPrompt, ApprovalProvider, ApprovalRequirement, TrustRequest,
};
use core::fmt::{self, Display, Formatter};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Instant;

const PROTOCOL_HEADER: &[u8] = b"CHIEF-TIER3-HARDWARE-KEY/1\n";
const RESPONSE_MAX_BYTES: usize = 32;
const PROTOCOL_ENVIRONMENT: &str = "CHIEF_APPROVAL_PROTOCOL";

/// Stable payload-blind failure from the external hardware-key boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareKeyApprovalError {
    /// The helper path was not an absolute normalized path.
    InvalidExecutable,
    /// The provider was asked for notification or biometric assurance.
    UnsupportedRequirement,
    /// The configured helper could not be started.
    SpawnFailed,
    /// The child process did not expose the requested protocol pipes.
    ProtocolPipeUnavailable,
    /// The complete exact-resource prompt could not be delivered before the deadline.
    RequestWriteFailed,
    /// The helper's response pipe could not be read.
    ResponseReadFailed,
    /// The helper did not acknowledge that it presented the hardware-key challenge.
    HardwareKeyNotAcknowledged,
    /// The helper returned anything other than one canonical hardware-key decision line.
    InvalidResponse,
    /// The helper could not be inspected, terminated, or reaped safely.
    ProcessControlFailed,
    /// A protocol worker terminated without reporting its result.
    ProtocolWorkerFailed,
}

impl Display for HardwareKeyApprovalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidExecutable => "hardware-key approval: invalid executable",
            Self::UnsupportedRequirement => "hardware-key approval: unsupported requirement",
            Self::SpawnFailed => "hardware-key approval: helper spawn failed",
            Self::ProtocolPipeUnavailable => "hardware-key approval: protocol pipe unavailable",
            Self::RequestWriteFailed => "hardware-key approval: request write failed",
            Self::ResponseReadFailed => "hardware-key approval: response read failed",
            Self::HardwareKeyNotAcknowledged => {
                "hardware-key approval: hardware-key challenge not acknowledged"
            }
            Self::InvalidResponse => "hardware-key approval: invalid response",
            Self::ProcessControlFailed => "hardware-key approval: process control failed",
            Self::ProtocolWorkerFailed => "hardware-key approval: protocol worker failed",
        })
    }
}

impl std::error::Error for HardwareKeyApprovalError {}

/// Shell-free external helper provider for canonical Tier 3 hardware-key approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareKeyCommandProvider {
    executable: PathBuf,
}

impl HardwareKeyCommandProvider {
    /// Retain one absolute normalized helper executable path.
    pub fn new(executable: impl Into<PathBuf>) -> Result<Self, HardwareKeyApprovalError> {
        let executable = executable.into();
        if !is_normalized_absolute(&executable) {
            return Err(HardwareKeyApprovalError::InvalidExecutable);
        }
        Ok(Self { executable })
    }

    /// Return the exact helper executable path.
    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

impl ApprovalProvider for HardwareKeyCommandProvider {
    type Error = HardwareKeyApprovalError;

    fn request_approval(
        &mut self,
        prompt: ApprovalPrompt<'_>,
    ) -> Result<ApprovalOutcome, Self::Error> {
        let timeout = match prompt.requirement() {
            ApprovalRequirement::HardwareKey { timeout } => timeout,
            ApprovalRequirement::None
            | ApprovalRequirement::Notification { .. }
            | ApprovalRequirement::Biometric { .. } => {
                return Err(HardwareKeyApprovalError::UnsupportedRequirement)
            }
        };
        let request = encode_request(prompt.request(), timeout.as_millis());
        let mut child = Command::new(&self.executable)
            .env_clear()
            .env(PROTOCOL_ENVIRONMENT, "3")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| HardwareKeyApprovalError::SpawnFailed)?;
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                stop_child(&mut child)?;
                return Err(HardwareKeyApprovalError::ProtocolPipeUnavailable);
            }
        };
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                stop_child(&mut child)?;
                return Err(HardwareKeyApprovalError::ProtocolPipeUnavailable);
            }
        };

        let (sender, receiver) = mpsc::channel();
        let write_sender = sender.clone();
        if thread::Builder::new()
            .name("chief-hardware-key-request".to_string())
            .spawn(move || {
                let result = write_request(stdin, &request);
                let _ = write_sender.send(ProtocolEvent::Written(result));
            })
            .is_err()
        {
            stop_child(&mut child)?;
            return Err(HardwareKeyApprovalError::ProtocolWorkerFailed);
        }
        if thread::Builder::new()
            .name("chief-hardware-key-response".to_string())
            .spawn(move || read_response_lines(stdout, sender))
            .is_err()
        {
            stop_child(&mut child)?;
            return Err(HardwareKeyApprovalError::ProtocolWorkerFailed);
        }

        let deadline = Instant::now() + timeout;
        let mut written = false;
        let mut acknowledged = false;
        let mut response = None;
        while !written || response.is_none() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match receiver.recv_timeout(remaining) {
                Ok(ProtocolEvent::Written(Ok(()))) => written = true,
                Ok(ProtocolEvent::Written(Err(()))) => {
                    stop_child(&mut child)?;
                    return Err(HardwareKeyApprovalError::RequestWriteFailed);
                }
                Ok(ProtocolEvent::Line(Ok(line))) if !acknowledged => {
                    if line != b"ready\n" {
                        stop_child(&mut child)?;
                        return Err(HardwareKeyApprovalError::InvalidResponse);
                    }
                    acknowledged = true;
                }
                Ok(ProtocolEvent::Line(Ok(line))) => {
                    response = match parse_response(&line) {
                        Ok(response) => Some(response),
                        Err(error) => {
                            stop_child(&mut child)?;
                            return Err(error);
                        }
                    };
                }
                Ok(ProtocolEvent::Line(Err(()))) => {
                    stop_child(&mut child)?;
                    return Err(HardwareKeyApprovalError::ResponseReadFailed);
                }
                Ok(ProtocolEvent::ReadEnded) if response.is_some() => {}
                Ok(ProtocolEvent::ReadEnded) => {
                    stop_child(&mut child)?;
                    return Err(HardwareKeyApprovalError::InvalidResponse);
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => {
                    stop_child(&mut child)?;
                    return Err(HardwareKeyApprovalError::ProtocolWorkerFailed);
                }
            }
        }

        if !written {
            stop_child(&mut child)?;
            return Err(HardwareKeyApprovalError::RequestWriteFailed);
        }
        if let Some(response) = response {
            stop_child(&mut child)?;
            return Ok(response);
        }
        if !acknowledged {
            stop_child(&mut child)?;
            return Err(HardwareKeyApprovalError::HardwareKeyNotAcknowledged);
        }

        match child
            .try_wait()
            .map_err(|_| HardwareKeyApprovalError::ProcessControlFailed)?
        {
            Some(_) => Err(HardwareKeyApprovalError::InvalidResponse),
            None => {
                stop_child(&mut child)?;
                Ok(ApprovalOutcome::TimedOut)
            }
        }
    }
}

enum ProtocolEvent {
    Written(Result<(), ()>),
    Line(Result<Vec<u8>, ()>),
    ReadEnded,
}

fn is_normalized_absolute(executable: &Path) -> bool {
    executable.is_absolute()
        && !executable
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

fn encode_request(request: &TrustRequest, timeout_millis: u128) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(PROTOCOL_HEADER);
    encoded_field(&mut encoded, b"request_id ", request.request_id());
    encoded_field(&mut encoded, b"requested_by ", request.requested_by());
    encoded.extend_from_slice(b"effective_tier ");
    encoded.push(tier_byte(request.effective_tier()));
    encoded.push(b'\n');
    encoded.extend_from_slice(b"timeout_ms ");
    encoded.extend_from_slice(timeout_millis.to_string().as_bytes());
    encoded.push(b'\n');
    encoded.extend_from_slice(b"resources ");
    encoded.extend_from_slice(request.resources().len().to_string().as_bytes());
    encoded.push(b'\n');
    for resource in request.resources() {
        encoded.extend_from_slice(b"resource ");
        encoded.push(tier_byte(resource.tier()));
        encoded.push(b' ');
        encode_hex(&mut encoded, resource.resource_id().as_bytes());
        encoded.push(b'\n');
    }
    encoded.extend_from_slice(b"end\n");
    encoded
}

fn encoded_field(target: &mut Vec<u8>, prefix: &[u8], value: &str) {
    target.extend_from_slice(prefix);
    encode_hex(target, value.as_bytes());
    target.push(b'\n');
}

fn encode_hex(target: &mut Vec<u8>, value: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in value {
        target.push(HEX[usize::from(byte >> 4)]);
        target.push(HEX[usize::from(byte & 0x0f)]);
    }
}

fn tier_byte(tier: PrivilegeTier) -> u8 {
    match tier {
        PrivilegeTier::Tier0 => b'0',
        PrivilegeTier::Tier1 => b'1',
        PrivilegeTier::Tier2 => b'2',
        PrivilegeTier::Tier3 => b'3',
    }
}

fn write_request(mut stdin: impl Write, request: &[u8]) -> Result<(), ()> {
    stdin.write_all(request).map_err(|_| ())?;
    stdin.flush().map_err(|_| ())
}

fn read_response_lines(mut stdout: impl Read, sender: mpsc::Sender<ProtocolEvent>) {
    loop {
        match read_line(&mut stdout) {
            Ok(line) if line.is_empty() => {
                let _ = sender.send(ProtocolEvent::ReadEnded);
                return;
            }
            Ok(line) => {
                if sender.send(ProtocolEvent::Line(Ok(line))).is_err() {
                    return;
                }
            }
            Err(()) => {
                let _ = sender.send(ProtocolEvent::Line(Err(())));
                return;
            }
        }
    }
}

fn read_line(mut stdout: impl Read) -> Result<Vec<u8>, ()> {
    let mut response = Vec::with_capacity(RESPONSE_MAX_BYTES);
    loop {
        let mut byte = [0u8; 1];
        match stdout.read(&mut byte).map_err(|_| ())? {
            0 => return Ok(response),
            1 => {
                response.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(response);
                }
                if response.len() == RESPONSE_MAX_BYTES {
                    return Err(());
                }
            }
            _ => unreachable!("one-byte reads cannot return more than one byte"),
        }
    }
}

fn parse_response(response: &[u8]) -> Result<ApprovalOutcome, HardwareKeyApprovalError> {
    match response {
        b"approve hardware-key\n" => Ok(ApprovalOutcome::Approved(ApprovalAssurance::HardwareKey)),
        b"deny\n" => Ok(ApprovalOutcome::Denied),
        _ => Err(HardwareKeyApprovalError::InvalidResponse),
    }
}

fn stop_child(child: &mut Child) -> Result<(), HardwareKeyApprovalError> {
    match child
        .try_wait()
        .map_err(|_| HardwareKeyApprovalError::ProcessControlFailed)?
    {
        Some(_) => Ok(()),
        None => {
            if child.kill().is_err()
                && child
                    .try_wait()
                    .map_err(|_| HardwareKeyApprovalError::ProcessControlFailed)?
                    .is_none()
            {
                return Err(HardwareKeyApprovalError::ProcessControlFailed);
            }
            child
                .wait()
                .map(|_| ())
                .map_err(|_| HardwareKeyApprovalError::ProcessControlFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_trust_checker::{TrustChecker, TrustCheckerError, TrustResource};

    #[test]
    fn paths_and_responses_are_canonical() {
        assert!(HardwareKeyCommandProvider::new(PathBuf::from("relative")).is_err());
        assert!(HardwareKeyCommandProvider::new(PathBuf::from("/tmp/../helper")).is_err());
        assert_eq!(
            parse_response(b"approve hardware-key\n"),
            Ok(ApprovalOutcome::Approved(ApprovalAssurance::HardwareKey))
        );
        assert_eq!(parse_response(b"deny\n"), Ok(ApprovalOutcome::Denied));
        for invalid in [
            b"approve\n".as_slice(),
            b"approve biometric\n".as_slice(),
            b"approve hardware-key".as_slice(),
            b"APPROVE HARDWARE-KEY\n".as_slice(),
            b"\n".as_slice(),
        ] {
            assert_eq!(
                parse_response(invalid),
                Err(HardwareKeyApprovalError::InvalidResponse)
            );
        }
    }

    #[test]
    fn request_protocol_is_versioned_bounded_and_exact() {
        let request = TrustRequest::new(
            "request-3",
            "operator:local",
            vec![
                TrustResource::new("channel:weather", PrivilegeTier::Tier0).unwrap(),
                TrustResource::new("package:vault-admin", PrivilegeTier::Tier3).unwrap(),
            ],
        )
        .unwrap();
        let encoded = String::from_utf8(encode_request(&request, 60_000)).unwrap();
        assert_eq!(
            encoded,
            concat!(
                "CHIEF-TIER3-HARDWARE-KEY/1\n",
                "request_id 726571756573742d33\n",
                "requested_by 6f70657261746f723a6c6f63616c\n",
                "effective_tier 3\n",
                "timeout_ms 60000\n",
                "resources 2\n",
                "resource 0 6368616e6e656c3a77656174686572\n",
                "resource 3 7061636b6167653a7661756c742d61646d696e\n",
                "end\n"
            )
        );
    }

    #[test]
    fn non_hardware_key_requirements_fail_closed() {
        let provider = HardwareKeyCommandProvider::new(test_absolute_path()).unwrap();
        for tier in [PrivilegeTier::Tier1, PrivilegeTier::Tier2] {
            let request = TrustRequest::new(
                "request",
                "operator:local",
                vec![TrustResource::new("resource", tier).unwrap()],
            )
            .unwrap();
            let mut checker = TrustChecker::new(provider.clone());
            assert!(matches!(
                checker.authorize(&request),
                Err(TrustCheckerError::Provider(
                    HardwareKeyApprovalError::UnsupportedRequirement
                ))
            ));
        }
    }

    #[cfg(unix)]
    fn test_absolute_path() -> PathBuf {
        PathBuf::from("/helper")
    }

    #[cfg(windows)]
    fn test_absolute_path() -> PathBuf {
        PathBuf::from(r"C:\helper.exe")
    }
}
