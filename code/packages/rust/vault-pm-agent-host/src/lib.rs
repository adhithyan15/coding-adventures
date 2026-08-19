//! Permission-checked local agent transport for vault-pm (VLT-PM48).
//!
//! This crate is the new host-side capability VLT-PM00 §23 item 12 named: a
//! Unix-domain-socket server and client that let a running `vault-pm agent`
//! process retain a master passphrase across one-shot command invocations,
//! and let those invocations reach it without either side trusting anything
//! the filesystem alone can prove. It knows nothing about vaults, records, or
//! cryptography — see `state` for exactly what it retains and why that is
//! safe to keep decoupled from `vault-pm-application` — and nothing about
//! command-line parsing, which stays in `vault-pm-cli`.
//!
//! # Module map
//!
//! - [`state`]: the pure in-memory retention store and idle-bound policy.
//! - [`transport`]: length-prefixed framing over any byte stream.
//! - [`peer`] (Unix only): the authoritative peer-credential check.
//! - [`server`] (Unix only): socket bind, accept loop, and request dispatch.
//! - [`client`] (Unix only): connect-and-request helpers used by the CLI.
//! - [`lifecycle`] (Unix only): detached process spawn and readiness polling.
//!
//! # Windows
//!
//! Deferred. See `VLT-PM48-local-agent-ipc.md` §9 for the explicit scope
//! decision: this crate compiles on every target, but every socket-touching
//! function is Unix-only and returns [`AgentHostError::UnsupportedPlatform`]
//! everywhere else.

#![deny(missing_docs)]

pub mod state;
pub mod transport;

#[cfg(unix)]
pub mod client;
#[cfg(unix)]
pub mod lifecycle;
#[cfg(unix)]
pub mod peer;
#[cfg(unix)]
pub mod server;

use core::fmt::{self, Display, Formatter};

/// Stable, closed failure taxonomy for this crate's host operations.
///
/// Deliberately coarse, matching `vault-pm-local-host::LocalHostError`'s own
/// convention: a local IPC boundary does not owe a caller a diagnosis of
/// exactly which syscall failed, and a finer taxonomy would tempt a caller
/// into branching on a distinction that carries no different remediation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentHostError {
    /// This platform has no supported agent transport.
    UnsupportedPlatform,
    /// The runtime directory or socket could not be prepared or verified.
    RuntimeUnavailable,
    /// A socket already exists at the resolved path and is answering —
    /// another agent instance is already running.
    AlreadyRunning,
    /// An existing object at the socket path failed the same ownership and
    /// type checks every other private root in this product enforces.
    InsecureExistingSocket,
    /// The socket could not be bound, or a spawn/wait failed.
    Unavailable,
    /// The connecting peer's credentials could not be verified, or did not
    /// match this local user.
    Unauthorized,
    /// A request or response could not be encoded or decoded.
    Protocol,
    /// The connection or an I/O operation on it failed or timed out.
    Io,
}

impl Display for AgentHostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "vault-pm agent host: unsupported platform",
            Self::RuntimeUnavailable => "vault-pm agent host: runtime directory unavailable",
            Self::AlreadyRunning => "vault-pm agent host: already running",
            Self::InsecureExistingSocket => "vault-pm agent host: insecure existing socket",
            Self::Unavailable => "vault-pm agent host: unavailable",
            Self::Unauthorized => "vault-pm agent host: unauthorized",
            Self::Protocol => "vault-pm agent host: protocol error",
            Self::Io => "vault-pm agent host: I/O error",
        })
    }
}

impl std::error::Error for AgentHostError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_a_stable_closed_message() {
        let expected = [
            (
                AgentHostError::UnsupportedPlatform,
                "vault-pm agent host: unsupported platform",
            ),
            (
                AgentHostError::RuntimeUnavailable,
                "vault-pm agent host: runtime directory unavailable",
            ),
            (
                AgentHostError::AlreadyRunning,
                "vault-pm agent host: already running",
            ),
            (
                AgentHostError::InsecureExistingSocket,
                "vault-pm agent host: insecure existing socket",
            ),
            (
                AgentHostError::Unavailable,
                "vault-pm agent host: unavailable",
            ),
            (
                AgentHostError::Unauthorized,
                "vault-pm agent host: unauthorized",
            ),
            (
                AgentHostError::Protocol,
                "vault-pm agent host: protocol error",
            ),
            (AgentHostError::Io, "vault-pm agent host: I/O error"),
        ];
        for (error, message) in expected {
            assert_eq!(error.to_string(), message);
            // `std::error::Error` is implemented and usable as a trait object,
            // the same contract every other closed error in this product line
            // carries.
            let _: &dyn std::error::Error = &error;
        }
    }
}
