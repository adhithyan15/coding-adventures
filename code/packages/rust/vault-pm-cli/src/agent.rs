//! Local agent lifecycle, IPC client calls, and the opportunistic-reuse seam.
//!
//! VLT-PM48. This module is the CLI-side half of the local agent VLT-PM00
//! §14.5 promised for Phase 1B: it drives `vault-pm agent
//! start|stop|status|unlock|lock` and the hidden `run-foreground` verb the
//! agent process itself runs as, and it is the *only* place in this crate
//! that imports `coding_adventures_vault_pm_agent_host`. Every other module
//! reaches the agent only through [`passphrase_for`] and
//! [`forget_cached_passphrase_on_rejection`] below.
//!
//! # What this module does not do
//!
//! It never verifies a passphrase itself. [`agent_unlock`] performs the same
//! authenticated open every other command performs — see
//! `open_authenticated_access` — and hands the agent a passphrase only after
//! that open has already succeeded against the real vault. The agent crate
//! trusts every `Unlock` it receives precisely because this is the only
//! caller, and this caller only calls it once authentication is done. See
//! `VLT-PM48-local-agent-ipc.md` §4.2 for the full argument.
//!
//! # Windows
//!
//! Deferred (VLT-PM48 §9). `coding_adventures_vault_pm_agent_host` compiles
//! everywhere but its socket-touching modules are Unix-only, so every
//! function below has a `cfg(not(unix))` twin that reports
//! [`CliFailure::Unsupported`] without referencing them.

use crate::{CliFailure, CliHost, CliOutput, HostError};
use coding_adventures_vault_pm_config::ConfigName;
use coding_adventures_vault_pm_local_host::{LocalVaultPaths, LocalWriterGuard};
use coding_adventures_zeroize::Zeroizing;

/// Fixed, secret-free argument vector `agent start` re-executes this same
/// binary with.
///
/// Mirrors `CLIPBOARD_CLEAR_ARGUMENTS`'s reasoning one module up: the
/// detached agent process takes no caller-supplied data on its argument
/// vector, only on the socket it later binds.
#[cfg(unix)]
const AGENT_RUN_FOREGROUND_ARGUMENTS: &[&str] = &["agent", "run-foreground"];

#[cfg(unix)]
mod imp {
    use super::*;
    use crate::{
        application_locator, application_store, configured_repository_factory, configured_vault,
        decode_config, map_application, map_host, map_local_host,
    };
    use coding_adventures_vault_pm_agent_host::state::VaultStatus;
    use coding_adventures_vault_pm_agent_host::{
        client, lifecycle, server::AgentServer, AgentHostError,
    };
    use coding_adventures_vault_pm_application::VaultAccessV1;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    /// How long `agent start` waits for the freshly spawned process to
    /// answer a `Ping` before reporting failure.
    const START_READY_TIMEOUT: Duration = Duration::from_secs(5);

    fn map_agent_host_error(error: AgentHostError) -> CliFailure {
        match error {
            AgentHostError::UnsupportedPlatform => CliFailure::Unsupported,
            AgentHostError::AlreadyRunning
            | AgentHostError::RuntimeUnavailable
            | AgentHostError::InsecureExistingSocket
            | AgentHostError::Unavailable
            | AgentHostError::Unauthorized
            | AgentHostError::Protocol
            | AgentHostError::Io => CliFailure::AgentUnavailable,
        }
    }

    /// Opportunistically use a running, unlocked agent instead of prompting.
    ///
    /// This is the whole of VLT-PM48 §2 requirement 4's "opportunistic reuse"
    /// half: every failure — no agent running, the vault never unlocked
    /// through `agent unlock`, an expired idle bound, a stale or unreachable
    /// socket — falls back to the host's ordinary terminal prompt. One-shot
    /// operation is unconditionally correct with no agent present; this
    /// function can only ever make a command *skip a prompt*, never change
    /// what it does.
    pub(crate) fn passphrase_for(
        host: &dyn CliHost,
        paths: &LocalVaultPaths,
        vault_name: &ConfigName,
    ) -> Result<Zeroizing<Vec<u8>>, HostError> {
        if let Some(passphrase) =
            client::cached_passphrase(paths.agent_socket_path(), vault_name.as_str())
        {
            return Ok(passphrase);
        }
        host.read_existing_passphrase()
    }

    /// Best-effort self-heal after an agent-opportunistic command comes back
    /// `Locked`.
    ///
    /// See `coding_adventures_vault_pm_agent_host::client::forget_on_rejection`
    /// for the full argument. Safe to call unconditionally: it is a no-op
    /// whether or not the rejected passphrase actually came from the agent,
    /// and whether or not an agent is even running.
    pub(crate) fn forget_cached_passphrase_on_rejection(
        paths: &LocalVaultPaths,
        vault_name: &ConfigName,
    ) {
        client::forget_on_rejection(paths.agent_socket_path(), vault_name.as_str());
    }

    pub(crate) fn agent_start(host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
        let paths = host.paths().map_err(map_host)?;
        let socket_path = paths.agent_socket_path();
        if client::is_running(socket_path) {
            return Ok(CliOutput::success("Agent: already running.\n"));
        }
        let program = std::env::current_exe().map_err(|_| CliFailure::AgentUnavailable)?;
        lifecycle::spawn_detached(&program, AGENT_RUN_FOREGROUND_ARGUMENTS)
            .map_err(map_agent_host_error)?;
        if client::wait_until_ready(socket_path, START_READY_TIMEOUT) {
            Ok(CliOutput::success("Agent: started.\n"))
        } else {
            Err(CliFailure::AgentUnavailable)
        }
    }

    pub(crate) fn agent_stop(host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
        let paths = host.paths().map_err(map_host)?;
        // Stopping an agent that was never running is not a failure — the
        // same idempotent contract `lock` and `Self::agent_lock` promise.
        match client::shutdown(paths.agent_socket_path()) {
            Ok(()) => Ok(CliOutput::success("Agent: stopped.\n")),
            Err(_) => Ok(CliOutput::success("Agent: not running.\n")),
        }
    }

    pub(crate) fn agent_lock(
        host: &dyn CliHost,
        selected_vault: Option<&ConfigName>,
    ) -> Result<CliOutput, CliFailure> {
        let paths = host.paths().map_err(map_host)?;
        let vault_name = selected_vault.map(ConfigName::as_str);
        match client::lock(paths.agent_socket_path(), vault_name) {
            Ok(()) => Ok(CliOutput::success("Agent: locked.\n")),
            Err(_) => Ok(CliOutput::success("Agent: not running.\n")),
        }
    }

    pub(crate) fn agent_status(
        host: &dyn CliHost,
        selected_vault: Option<&ConfigName>,
        json: bool,
    ) -> Result<CliOutput, CliFailure> {
        let paths = host.paths().map_err(map_host)?;
        let entries = client::status(paths.agent_socket_path()).ok();
        Ok(render_agent_status(entries, selected_vault, json))
    }

    fn render_agent_status(
        entries: Option<Vec<VaultStatus>>,
        selected_vault: Option<&ConfigName>,
        json: bool,
    ) -> CliOutput {
        let Some(entries) = entries else {
            return CliOutput::success(if json {
                "{\"agent\":\"not_running\"}\n".to_owned()
            } else {
                "Agent: not running.\n".to_owned()
            });
        };
        if let Some(name) = selected_vault {
            let remaining_seconds = entries
                .iter()
                .find(|entry| entry.vault_name == name.as_str())
                .map(|entry| entry.remaining.as_secs());
            return CliOutput::success(if json {
                match remaining_seconds {
                    Some(seconds) => format!(
                        "{{\"agent\":\"running\",\"vault\":\"{}\",\"unlocked\":true,\"remaining_seconds\":{seconds}}}\n",
                        name.as_str()
                    ),
                    None => format!(
                        "{{\"agent\":\"running\",\"vault\":\"{}\",\"unlocked\":false}}\n",
                        name.as_str()
                    ),
                }
            } else {
                match remaining_seconds {
                    Some(seconds) => {
                        format!(
                            "Agent: running.\n{}: unlocked ({seconds}s remaining)\n",
                            name.as_str()
                        )
                    }
                    None => format!("Agent: running.\n{}: locked\n", name.as_str()),
                }
            });
        }
        if json {
            let mut body = String::from("{\"agent\":\"running\",\"vaults\":[");
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    body.push(',');
                }
                body.push_str(&format!(
                    "{{\"name\":\"{}\",\"remaining_seconds\":{}}}",
                    entry.vault_name,
                    entry.remaining.as_secs()
                ));
            }
            body.push_str("]}\n");
            return CliOutput::success(body);
        }
        if entries.is_empty() {
            return CliOutput::success("Agent: running. No vaults retained.\n");
        }
        let mut body = String::from("Agent: running.\n");
        for entry in &entries {
            body.push_str(&format!(
                "  {}: unlocked ({}s remaining)\n",
                entry.vault_name,
                entry.remaining.as_secs()
            ));
        }
        CliOutput::success(body)
    }

    /// Verify a passphrase against the resolved vault, then hand it to the
    /// agent.
    ///
    /// This is `open_authenticated_access`'s own unlock step, reused for its
    /// verification side effect only: the session it opens is immediately
    /// locked again and discarded. VLT-PM48 §4.2 states why this order is not
    /// optional — the agent must never retain a passphrase this process has
    /// not already confirmed against the real vault, because unlike
    /// `vault-pm shell`'s single retained value, a bad one here would poison
    /// every later one-shot command's opportunistic lookup until it expired
    /// or `agent lock` was run by hand.
    pub(crate) fn agent_unlock(
        host: &dyn CliHost,
        paths: &LocalVaultPaths,
        writer: &LocalWriterGuard,
        selected_vault: Option<&ConfigName>,
    ) -> Result<CliOutput, CliFailure> {
        let exact_config = writer
            .load_config()
            .map_err(map_local_host)?
            .ok_or(CliFailure::InvalidCommand)?;
        let config = decode_config(&exact_config)?;
        let vault = configured_vault(paths, &config, selected_vault)?;
        let vault_name = selected_vault
            .cloned()
            .unwrap_or_else(|| config.default_vault().clone());
        let idle_bound_ms = u64::from(vault.auto_lock_seconds()).saturating_mul(1_000);
        let locator = application_locator(vault.locator());
        let application_store = application_store(paths);
        let repository_factory = configured_repository_factory(&config, vault)?;
        let mut access = VaultAccessV1::locked(locator);
        let passphrase = host.read_existing_passphrase().map_err(map_host)?;
        let retained = Zeroizing::new(passphrase.to_vec());
        access
            .unlock_recovering_pending_publication(
                passphrase,
                &application_store,
                &application_store,
                &repository_factory,
            )
            .map_err(map_application)?;
        access.lock();
        client::unlock(
            paths.agent_socket_path(),
            vault_name.as_str(),
            retained,
            idle_bound_ms,
        )
        .map_err(map_agent_host_error)?;
        Ok(CliOutput::success("Agent: unlocked.\n"))
    }

    pub(crate) fn agent_run_foreground(host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
        let paths = host.paths().map_err(map_host)?;
        let prepared = paths.prepare().map_err(map_local_host)?;
        prepared.ensure_runtime_root().map_err(map_local_host)?;
        let server = AgentServer::bind(paths.agent_socket_path()).map_err(map_agent_host_error)?;
        let signal = AtomicBool::new(false);
        server.run(&signal);
        Ok(CliOutput::success(""))
    }
}

#[cfg(not(unix))]
mod imp {
    use super::*;

    pub(crate) fn passphrase_for(
        host: &dyn CliHost,
        _paths: &LocalVaultPaths,
        _vault_name: &ConfigName,
    ) -> Result<Zeroizing<Vec<u8>>, HostError> {
        // No agent transport exists on this platform (VLT-PM48 §9), so every
        // command falls back to its ordinary terminal prompt unconditionally.
        host.read_existing_passphrase()
    }

    pub(crate) fn forget_cached_passphrase_on_rejection(
        _paths: &LocalVaultPaths,
        _vault_name: &ConfigName,
    ) {
    }

    pub(crate) fn agent_start(_host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
        Err(CliFailure::Unsupported)
    }

    pub(crate) fn agent_stop(_host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
        Err(CliFailure::Unsupported)
    }

    pub(crate) fn agent_lock(
        _host: &dyn CliHost,
        _selected_vault: Option<&ConfigName>,
    ) -> Result<CliOutput, CliFailure> {
        Err(CliFailure::Unsupported)
    }

    pub(crate) fn agent_status(
        _host: &dyn CliHost,
        _selected_vault: Option<&ConfigName>,
        _json: bool,
    ) -> Result<CliOutput, CliFailure> {
        Err(CliFailure::Unsupported)
    }

    pub(crate) fn agent_unlock(
        _host: &dyn CliHost,
        _paths: &LocalVaultPaths,
        _writer: &LocalWriterGuard,
        _selected_vault: Option<&ConfigName>,
    ) -> Result<CliOutput, CliFailure> {
        Err(CliFailure::Unsupported)
    }

    pub(crate) fn agent_run_foreground(_host: &dyn CliHost) -> Result<CliOutput, CliFailure> {
        Err(CliFailure::Unsupported)
    }
}

pub(crate) use imp::*;
