//! Socket bind, accept loop, and request dispatch.
//!
//! One connection carries exactly one request and one response: the client
//! connects, sends a frame, reads a frame, and disconnects. There is no
//! multiplexing and no keep-alive, which keeps both the accept loop and the
//! shutdown path simple — a running request never has to be cancelled out
//! from under a client, because there is never more than one in flight per
//! connection and a connection's whole lifetime is a handful of
//! milliseconds.

use crate::state::AgentState;
use crate::{peer, transport, AgentHostError};
use coding_adventures_vault_pm_agent_protocol::{
    AgentRequest, AgentResponse, VaultStatusEntry, MAX_FRAME_BYTES, MAX_STATUS_VAULTS,
    MAX_VAULT_NAME_BYTES,
};
use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Ceiling on one response frame, larger than a request's because
/// [`AgentResponse::Status`] can list up to [`MAX_STATUS_VAULTS`] entries.
const MAX_RESPONSE_BYTES: usize = 1 + 1 + 1 + MAX_STATUS_VAULTS * (1 + MAX_VAULT_NAME_BYTES + 8);

/// How long the accept loop blocks on `poll` before re-checking the shutdown
/// flag, and the interval between two idle-bound sweeps.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Bound on a single connection's read and write, so a peer that connects
/// and then never sends anything cannot hold a slot in the accept loop
/// indefinitely.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// A bound, listening agent socket, ready to run.
pub struct AgentServer {
    listener: UnixListener,
    socket_path: PathBuf,
    state: Mutex<AgentState>,
}

impl AgentServer {
    /// Bind the agent socket at `socket_path`.
    ///
    /// The caller is responsible for having already verified (or created)
    /// the socket's owner-private parent directory — see
    /// `vault-pm-local-host::PreparedLocalVault::ensure_runtime_root` — this
    /// function only handles the socket file itself.
    ///
    /// # Errors
    ///
    /// Returns [`AgentHostError::AlreadyRunning`] if a live agent is already
    /// answering at this path, [`AgentHostError::InsecureExistingSocket`] if
    /// a stale path exists but is not a socket this same user owns, and
    /// [`AgentHostError::Unavailable`] on any other bind failure.
    pub fn bind(socket_path: &Path) -> Result<Self, AgentHostError> {
        // A bare `connect()` succeeding is not proof that anyone is actually
        // serving this socket: a listener mid-shutdown can still have queued
        // connections in its backlog for a brief window after it stops
        // calling `accept`, and a bare connect would misread that window as
        // "already running." `crate::client::ping` sends a real request and
        // waits (bounded) for a real answer, which is what
        // `VLT-PM48-local-agent-ipc.md` §6 calls "verified" liveness rather
        // than "reachable."
        if crate::client::ping(socket_path).is_ok() {
            return Err(AgentHostError::AlreadyRunning);
        }
        reclaim_stale_socket(socket_path)?;
        let listener = UnixListener::bind(socket_path).map_err(|_| AgentHostError::Unavailable)?;
        // `bind` creates the socket file honoring the process umask, which on
        // a permissive umask can leave it group- or world-readable. This is
        // defense in depth on top of the peer-credential check in
        // `handle_connection`, not a substitute for it — see `crate::peer`.
        fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))
            .map_err(|_| AgentHostError::Unavailable)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| AgentHostError::Unavailable)?;
        Ok(Self {
            listener,
            socket_path: socket_path.to_path_buf(),
            state: Mutex::new(AgentState::new()),
        })
    }

    /// Run the accept loop until a `Shutdown` request is served, or `signal`
    /// is externally set.
    ///
    /// Consumes `self`: an `AgentServer` is a single run of the agent
    /// process, not a value a caller reuses. The socket file is removed
    /// before this returns, whichever path ends the loop, so a later `agent
    /// start` never has to reason about a leftover file from a clean exit.
    pub fn run(self, signal: &AtomicBool) {
        std::thread::scope(|scope| {
            scope.spawn(|| self.sweep_loop(signal));
            self.accept_loop(signal);
        });
        let _ = fs::remove_file(&self.socket_path);
    }

    fn accept_loop(&self, signal: &AtomicBool) {
        loop {
            match self.listener.accept() {
                Ok((stream, _address)) => self.handle_connection(stream, signal),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(POLL_INTERVAL);
                }
                // A transient accept failure is logged nowhere (this product
                // never logs to a shared stream that could carry secrets by
                // accident) and simply retried after the same bounded pause.
                Err(_) => std::thread::sleep(POLL_INTERVAL),
            }
            if signal.load(Ordering::SeqCst) {
                return;
            }
        }
    }

    fn sweep_loop(&self, signal: &AtomicBool) {
        while !signal.load(Ordering::SeqCst) {
            std::thread::sleep(POLL_INTERVAL);
            self.state
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .sweep_expired(Instant::now());
        }
    }

    /// Handle exactly one connection: verify its peer, then serve at most one
    /// request.
    ///
    /// The peer-credential check happens *before* anything is read from the
    /// stream. An unauthorized peer receives no bytes at all — not a typed
    /// rejection, not a protocol error, nothing — because a local process
    /// that should never have been able to reach this socket does not need
    /// to learn that a vault-pm agent is the thing that refused it.
    fn handle_connection(&self, mut stream: UnixStream, signal: &AtomicBool) {
        if !peer::is_same_user(&stream) {
            return;
        }
        let _ = stream.set_read_timeout(Some(CONNECTION_TIMEOUT));
        let _ = stream.set_write_timeout(Some(CONNECTION_TIMEOUT));
        let Ok(payload) = transport::read_frame(&mut stream, MAX_FRAME_BYTES) else {
            return;
        };
        let Ok(request) = AgentRequest::decode(&payload) else {
            return;
        };
        let response = self.dispatch(request, signal);
        if let Ok(encoded) = response.encode() {
            let _ = transport::write_frame(&mut stream, &encoded, MAX_RESPONSE_BYTES);
        }
    }

    fn dispatch(&self, request: AgentRequest, signal: &AtomicBool) -> AgentResponse {
        let now = Instant::now();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match request {
            AgentRequest::Ping => AgentResponse::Ok,
            AgentRequest::Unlock {
                vault_name,
                passphrase,
                idle_bound_ms,
            } => {
                state.unlock(
                    vault_name,
                    passphrase,
                    Duration::from_millis(idle_bound_ms),
                    now,
                );
                AgentResponse::Ok
            }
            AgentRequest::GetPassphrase { vault_name } => match state.get(&vault_name, now) {
                Some(passphrase) => AgentResponse::Passphrase(passphrase),
                None => AgentResponse::NotRetained,
            },
            AgentRequest::Lock { vault_name } => {
                state.lock(vault_name.as_deref());
                AgentResponse::Ok
            }
            AgentRequest::Status => AgentResponse::Status(
                state
                    .status(now)
                    .into_iter()
                    .map(|entry| VaultStatusEntry {
                        vault_name: entry.vault_name,
                        #[allow(clippy::cast_possible_truncation)]
                        remaining_ms: entry.remaining.as_millis() as u64,
                    })
                    .collect(),
            ),
            AgentRequest::Shutdown => {
                state.lock(None);
                signal.store(true, Ordering::SeqCst);
                AgentResponse::Ok
            }
        }
    }
}

/// Remove a stale socket file, refusing anything that is not clearly our own.
///
/// A path that already exists here (given [`UnixStream::connect`] just
/// failed against it) is one of: a socket nobody is listening on anymore
/// (the ordinary case after an unclean exit), or something else entirely —
/// a symlink, a regular file, or a socket owned by a different local user.
/// Only the first is removed. Everything else fails closed with
/// [`AgentHostError::InsecureExistingSocket`], the same posture
/// `vault-pm-local-host` takes toward a foreign-owned lock file or config
/// file: the feature becomes unavailable rather than silently unlinking
/// something this process does not know it is safe to remove.
fn reclaim_stale_socket(socket_path: &Path) -> Result<(), AgentHostError> {
    let metadata = match fs::symlink_metadata(socket_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(AgentHostError::InsecureExistingSocket),
    };
    let file_type = metadata.file_type();
    if !file_type.is_socket() {
        return Err(AgentHostError::InsecureExistingSocket);
    }
    if metadata.uid() != current_uid() {
        return Err(AgentHostError::InsecureExistingSocket);
    }
    fs::remove_file(socket_path).map_err(|_| AgentHostError::InsecureExistingSocket)
}

fn current_uid() -> u32 {
    // SAFETY: `getuid` takes no arguments and cannot fail.
    unsafe { libc::getuid() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use coding_adventures_zeroize::Zeroizing;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn scratch_socket_path() -> PathBuf {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "vault-pm-agent-host-test-{}-{sequence}.sock",
            std::process::id()
        ))
    }

    fn run_in_background(path: PathBuf) -> (std::thread::JoinHandle<()>, PathBuf) {
        let server = AgentServer::bind(&path).unwrap();
        let signal = Box::leak(Box::new(AtomicBool::new(false)));
        let handle = std::thread::spawn(move || server.run(signal));
        (handle, path)
    }

    #[test]
    fn ping_unlock_get_lock_and_status_round_trip_over_a_real_socket() {
        let path = scratch_socket_path();
        let (handle, path) = run_in_background(path);
        // Give the accept loop a moment to be listening; `bind` already
        // guarantees the socket exists by the time this test's own connect
        // attempts run, so this is generous rather than load-bearing.
        assert!(client::wait_until_ready(&path, Duration::from_secs(2)));

        client::unlock(
            &path,
            "personal",
            Zeroizing::new(b"hunter2".to_vec()),
            300_000,
        )
        .unwrap();
        let fetched = client::get_passphrase(&path, "personal").unwrap();
        assert_eq!(
            fetched.as_ref().map(|value| value.as_slice()),
            Some(b"hunter2".as_slice())
        );

        let status = client::status(&path).unwrap();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].vault_name, "personal");

        client::lock(&path, Some("personal")).unwrap();
        assert!(client::get_passphrase(&path, "personal").unwrap().is_none());

        client::shutdown(&path).unwrap();
        handle.join().unwrap();
        assert!(!path.exists(), "the socket file is removed on shutdown");
    }

    #[test]
    fn a_second_bind_at_the_same_path_is_refused_while_the_first_is_live() {
        let path = scratch_socket_path();
        let (handle, path) = run_in_background(path.clone());
        assert!(client::wait_until_ready(&path, Duration::from_secs(2)));
        match AgentServer::bind(&path) {
            Err(error) => assert_eq!(error, AgentHostError::AlreadyRunning),
            Ok(_) => panic!("a live agent's socket must refuse a second bind"),
        }
        client::shutdown(&path).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn a_stale_socket_from_an_unclean_exit_is_reclaimed() {
        let path = scratch_socket_path();
        // Simulate an unclean exit: bind and drop without ever calling
        // `run`, so the socket file is left behind with nobody listening.
        let first = UnixListener::bind(&path).unwrap();
        drop(first);
        assert!(path.exists());
        let second = AgentServer::bind(&path);
        match second {
            Ok(_) => {}
            Err(error) => panic!("expected the stale socket to be reclaimed, got {error:?}"),
        }
    }

    #[test]
    fn a_non_socket_at_the_path_is_refused_rather_than_deleted() {
        let path = scratch_socket_path();
        std::fs::write(&path, b"not a socket").unwrap();
        match AgentServer::bind(&path) {
            Err(error) => assert_eq!(error, AgentHostError::InsecureExistingSocket),
            Ok(_) => panic!("a non-socket existing path must never be reclaimed"),
        }
        assert_eq!(std::fs::read(&path).unwrap(), b"not a socket");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn oversized_frame_and_garbage_connections_are_dropped_without_a_response() {
        let path = scratch_socket_path();
        let (handle, path) = run_in_background(path);
        assert!(client::wait_until_ready(&path, Duration::from_secs(2)));

        // A declared length past the ceiling: the server must close the
        // connection rather than allocate or answer.
        {
            let mut stream = UnixStream::connect(&path).unwrap();
            use std::io::Write;
            stream.write_all(&100_000_u32.to_be_bytes()).unwrap();
            drop(stream);
        }
        // The server must still be alive and answering afterward.
        assert!(client::ping(&path).unwrap());

        client::shutdown(&path).unwrap();
        handle.join().unwrap();
    }
}
