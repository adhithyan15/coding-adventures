//! Detached process spawn for `agent start`.
//!
//! `vault-pm agent start` is a one-shot command like every other `vault-pm`
//! invocation, and it must return promptly rather than becoming the
//! long-lived agent process itself. The mechanism is the same one
//! `vault-pm-cli-host::clipboard::spawn_detached_clearer` already uses for
//! the timed clipboard clear (VLT-PM46 §4.3): a double `fork` so the
//! long-lived process is orphaned to `init` rather than remaining this
//! command's child, and `setsid` so it leaves the terminal's session and is
//! not signaled when that terminal closes.
//!
//! This is a separate, simpler implementation rather than a shared one with
//! the clipboard clearer, because the two have different contracts on their
//! standard streams: the clearer receives a secret-free parameter block on a
//! pipe (its whole reason for existing is that argv is world-readable), while
//! the agent takes no parameters at all — its socket path is re-derived from
//! the same platform paths on both ends — and every stream is simply
//! discarded.

use crate::AgentHostError;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// Spawn `program arguments...` as a detached background process and return
/// once it has been launched.
///
/// This does not wait for the spawned process to become ready — see
/// [`crate::client::wait_until_ready`] for that — only that the fork
/// succeeded. `arguments` is `&'static [&'static str]`-shaped at every call
/// site in this product (the hidden `agent run-foreground` verb takes no
/// caller-supplied data), matching the same "fixed argument vector, nothing
/// interpolated" discipline `vault-pm-cli-host::clipboard` documents for its
/// own spawn.
///
/// # Errors
///
/// Returns [`AgentHostError::Unavailable`] if the process could not be
/// spawned.
///
/// # Panics
///
/// This function must be called from a single-threaded process. See
/// `vault-pm-cli-host::clipboard::spawn_detached_clearer`'s own safety note:
/// glibc's `fork` is not safe to call from the post-fork child of a
/// multithreaded parent, because it runs `pthread_atfork` handlers under an
/// internal lock another thread may hold. `vault-pm agent start` runs before
/// this process has spawned any thread of its own, which is what makes this
/// sound.
pub fn spawn_detached(program: &Path, arguments: &[&str]) -> Result<(), AgentHostError> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: the closure runs in the freshly forked child, between `fork`
    // and `execvp`. It is non-capturing, allocates nothing, and calls only
    // `fork`, `setsid`, and `_exit`, all async-signal-safe per POSIX. See
    // this function's own doc comment for the single-threaded precondition
    // that makes the inner `fork` itself sound.
    unsafe {
        command.pre_exec(|| {
            match libc::fork() {
                -1 => return Err(std::io::Error::last_os_error()),
                0 => {}
                _ => libc::_exit(0),
            }
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|_| AgentHostError::Unavailable)?;
    // The direct child is the short-lived intermediate; it has already
    // exited (or is about to) by the time this call returns in the ordinary
    // case, so reaping it here is immediate and leaves the grandchild
    // orphaned as intended, exactly as `spawn_detached_clearer` reasons about
    // its own intermediate.
    let _ = child.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    fn scratch_path(name: &str) -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "vault-pm-agent-host-lifecycle-{name}-{}-{sequence}",
            std::process::id()
        ))
    }

    #[test]
    fn a_detached_process_outlives_its_direct_parent() {
        let destination = scratch_path("detached");
        let _ = std::fs::remove_file(&destination);
        let script = format!("echo alive > {}", destination.display());
        spawn_detached(Path::new("/bin/sh"), &["-c", &script]).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if destination.exists() {
                break;
            }
            assert!(Instant::now() < deadline, "detached process never ran");
            std::thread::sleep(Duration::from_millis(20));
        }
        std::fs::remove_file(&destination).unwrap();
    }

    #[test]
    fn spawning_a_missing_program_fails_closed() {
        assert_eq!(
            spawn_detached(Path::new("/nonexistent/vault-pm-agent-test-binary"), &[]).unwrap_err(),
            AgentHostError::Unavailable
        );
    }
}
