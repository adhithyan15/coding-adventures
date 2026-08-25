//! Connect-and-request helpers used by `vault-pm-cli`.
//!
//! Every function here is a complete round trip: connect, send one request,
//! read one response, disconnect. None of them retry, and every one of them
//! is bounded by [`DEFAULT_TIMEOUT`], because every caller in this product
//! must remain correct with no agent running at all — VLT-PM48 §2 requirement
//! 4. A slow or wedged agent must fail exactly as fast as a missing one, so a
//! one-shot command's opportunistic check never becomes the reason it hangs.

use crate::state::VaultStatus;
use crate::{transport, AgentHostError};
use coding_adventures_vault_pm_agent_protocol::{
    AgentRequest, AgentResponse, VaultStatusEntry, MAX_STATUS_VAULTS, MAX_VAULT_NAME_BYTES,
};
use coding_adventures_zeroize::Zeroizing;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Ceiling on a response frame this client will read.
const MAX_RESPONSE_BYTES: usize = 1 + 1 + 1 + MAX_STATUS_VAULTS * (1 + MAX_VAULT_NAME_BYTES + 8);

/// Bound on one full request/response round trip.
///
/// Generous enough for a healthy agent on a loaded machine, and short enough
/// that a one-shot command opportunistically checking for a cached
/// passphrase never keeps a person waiting noticeably longer than the
/// terminal prompt it would otherwise have shown immediately.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1_500);

fn round_trip(
    socket_path: &Path,
    request: &AgentRequest,
    timeout: Duration,
) -> Result<AgentResponse, AgentHostError> {
    let mut stream = UnixStream::connect(socket_path).map_err(|_| AgentHostError::Unavailable)?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|_| AgentHostError::Io)?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|_| AgentHostError::Io)?;
    let payload = request.encode().map_err(|_| AgentHostError::Protocol)?;
    transport::write_frame(
        &mut stream,
        &payload,
        coding_adventures_vault_pm_agent_protocol::MAX_FRAME_BYTES,
    )
    .map_err(|_| AgentHostError::Io)?;
    let response_bytes =
        transport::read_frame(&mut stream, MAX_RESPONSE_BYTES).map_err(|_| AgentHostError::Io)?;
    AgentResponse::decode(&response_bytes).map_err(|_| AgentHostError::Protocol)
}

/// Ask the agent to confirm it is listening and answering.
///
/// # Errors
///
/// Returns [`AgentHostError`] on any connection, protocol, or unexpected
/// response failure — including simply "nothing is listening," which is the
/// ordinary and expected state when no agent has been started.
pub fn ping(socket_path: &Path) -> Result<bool, AgentHostError> {
    match round_trip(socket_path, &AgentRequest::Ping, DEFAULT_TIMEOUT)? {
        AgentResponse::Ok => Ok(true),
        _ => Err(AgentHostError::Protocol),
    }
}

/// Poll `ping` until it succeeds or `timeout` elapses.
///
/// Used by `agent start` so the command does not return before the freshly
/// spawned agent is actually ready to serve `agent unlock`. Each attempt is
/// bounded by [`DEFAULT_TIMEOUT`], which is correct for this function's one
/// real use: while a freshly spawned agent is still starting, nothing is
/// listening yet, so a `ping` fails (connection refused) almost instantly —
/// the retry loop gets many real attempts regardless of `DEFAULT_TIMEOUT`'s
/// size. That assumption does *not* hold for a server that is up but
/// transiently refusing connections (e.g. sitting at a concurrency cap) —
/// see [`ping_with_timeout`]'s doc comment for why that case needs a
/// different per-attempt bound, not this function.
pub fn wait_until_ready(socket_path: &Path, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if ping(socket_path).unwrap_or(false) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Like [`ping`], but with a caller-chosen per-attempt timeout instead of
/// the fixed [`DEFAULT_TIMEOUT`].
///
/// Test-only. Every real caller in this crate goes through [`ping`] or
/// [`wait_until_ready`], which are deliberately pinned to `DEFAULT_TIMEOUT`
/// — see this module's doc comment for why that constant is tuned the way
/// it is for a one-shot command talking to a *healthy* agent.
///
/// This exists for a different situation: `server::tests`'
/// `connections_past_the_concurrency_cap_are_dropped_and_capacity_recovers`
/// polls a server that has just been asked to free a connection slot it is
/// sitting at the cap for. A `ping`'s connection can land in the accept
/// backlog before the server's accept loop is scheduled to reject or serve
/// it, and then block for the *entire* `DEFAULT_TIMEOUT` waiting for a
/// response that was never coming — so on a CPU-contended CI runner, a
/// single `DEFAULT_TIMEOUT`-bound attempt can by itself consume nearly an
/// entire short retry budget, leaving the retry loop far fewer real attempts
/// than its interval would suggest and defeating the whole point of
/// retrying. (This is exactly what made that test's first fix — a bounded
/// retry built on [`wait_until_ready`] — itself flake in CI.) A short
/// per-attempt timeout here means a slow or still-refused attempt costs only
/// that much of the budget, so a caller-driven retry loop gets many more
/// real attempts within its own overall deadline.
#[cfg(test)]
pub(crate) fn ping_with_timeout(
    socket_path: &Path,
    timeout: Duration,
) -> Result<bool, AgentHostError> {
    match round_trip(socket_path, &AgentRequest::Ping, timeout)? {
        AgentResponse::Ok => Ok(true),
        _ => Err(AgentHostError::Protocol),
    }
}

/// Ask the agent to retain `passphrase` for `vault_name`.
///
/// The caller must have already verified this passphrase against the real
/// vault; see [`coding_adventures_vault_pm_agent_protocol::AgentRequest::Unlock`].
///
/// # Errors
///
/// Returns [`AgentHostError`] if the agent is unreachable or refuses the
/// request.
pub fn unlock(
    socket_path: &Path,
    vault_name: &str,
    passphrase: Zeroizing<Vec<u8>>,
    idle_bound_ms: u64,
) -> Result<(), AgentHostError> {
    let request = AgentRequest::Unlock {
        vault_name: vault_name.to_owned(),
        passphrase,
        idle_bound_ms,
    };
    match round_trip(socket_path, &request, DEFAULT_TIMEOUT)? {
        AgentResponse::Ok => Ok(()),
        _ => Err(AgentHostError::Protocol),
    }
}

/// Ask the agent for `vault_name`'s retained passphrase.
///
/// # Errors
///
/// Returns [`AgentHostError`] if the agent is unreachable. Returns `Ok(None)`
/// — not an error — when the agent is reachable but has nothing retained for
/// this vault, or the retained value has expired.
pub fn get_passphrase(
    socket_path: &Path,
    vault_name: &str,
) -> Result<Option<Zeroizing<Vec<u8>>>, AgentHostError> {
    let request = AgentRequest::GetPassphrase {
        vault_name: vault_name.to_owned(),
    };
    match round_trip(socket_path, &request, DEFAULT_TIMEOUT)? {
        AgentResponse::Passphrase(passphrase) => Ok(Some(passphrase)),
        AgentResponse::NotRetained => Ok(None),
        _ => Err(AgentHostError::Protocol),
    }
}

/// Best-effort opportunistic lookup for a one-shot command.
///
/// Collapses every failure — no agent running, a stale or unresponsive
/// socket, a protocol mismatch — to `None`, which callers treat identically
/// to "no cached passphrase": fall back to the ordinary terminal prompt.
/// VLT-PM48 §2 requirement 4 states this is a hard requirement, not an
/// optimization: one-shot operation must remain correct with no agent
/// running at all, so this function is the one place in this crate that is
/// permitted to silently discard an error rather than propagate it.
pub fn cached_passphrase(socket_path: &Path, vault_name: &str) -> Option<Zeroizing<Vec<u8>>> {
    get_passphrase(socket_path, vault_name).ok().flatten()
}

/// Ask the agent to forget one vault's retained passphrase, or every vault's.
///
/// # Errors
///
/// Returns [`AgentHostError`] if the agent is unreachable or refuses the
/// request. Forgetting a vault the agent never retained is not an error —
/// see `AgentState::lock` — so this only fails on a genuine transport or
/// protocol problem.
pub fn lock(socket_path: &Path, vault_name: Option<&str>) -> Result<(), AgentHostError> {
    let request = AgentRequest::Lock {
        vault_name: vault_name.map(str::to_owned),
    };
    match round_trip(socket_path, &request, DEFAULT_TIMEOUT)? {
        AgentResponse::Ok => Ok(()),
        _ => Err(AgentHostError::Protocol),
    }
}

/// Best-effort self-heal: forget a vault's cached passphrase after it was
/// rejected.
///
/// Used after a one-shot command that consumed an agent-supplied passphrase
/// comes back `Locked` — the same self-healing `ShellSession::lock` performs
/// in-process (VLT-PM40 §3.4 rule 2) — so a stale cached value (for example,
/// after an out-of-band `passphrase rotate` on another device) does not keep
/// failing every later opportunistic use until the configured idle bound
/// elapses on its own. Every failure is swallowed: this is cleanup, not the
/// command's own result, and a caller that already has a `Locked` failure to
/// report must not replace it with an unrelated agent-connectivity error.
pub fn forget_on_rejection(socket_path: &Path, vault_name: &str) {
    let _ = lock(socket_path, Some(vault_name));
}

/// Ask the agent which vaults currently have a retained, unexpired
/// passphrase.
///
/// # Errors
///
/// Returns [`AgentHostError`] if the agent is unreachable or refuses the
/// request.
pub fn status(socket_path: &Path) -> Result<Vec<VaultStatus>, AgentHostError> {
    match round_trip(socket_path, &AgentRequest::Status, DEFAULT_TIMEOUT)? {
        AgentResponse::Status(entries) => Ok(entries
            .into_iter()
            .map(
                |VaultStatusEntry {
                     vault_name,
                     remaining_ms,
                 }| VaultStatus {
                    vault_name,
                    remaining: Duration::from_millis(remaining_ms),
                },
            )
            .collect()),
        _ => Err(AgentHostError::Protocol),
    }
}

/// Ask the agent to forget everything and stop listening.
///
/// # Errors
///
/// Returns [`AgentHostError`] if the agent is unreachable or refuses the
/// request. A caller that wants "stopped or already not running" to both be
/// success — `agent stop`'s own contract — treats
/// [`AgentHostError::Unavailable`] from this function as success rather than
/// propagating it; this function itself reports connection failure exactly
/// as any other request would, so that distinction stays a caller-level
/// policy rather than being baked in here.
pub fn shutdown(socket_path: &Path) -> Result<(), AgentHostError> {
    match round_trip(socket_path, &AgentRequest::Shutdown, DEFAULT_TIMEOUT)? {
        AgentResponse::Ok => Ok(()),
        _ => Err(AgentHostError::Protocol),
    }
}

/// Whether an agent is currently reachable at `socket_path`.
///
/// A thin, explicitly named wrapper over `ping` for callers (`agent status`)
/// that want a boolean rather than a `Result` for the ordinary "is it
/// running" question.
pub fn is_running(socket_path: &Path) -> bool {
    ping(socket_path).unwrap_or(false)
}
