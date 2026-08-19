//! Peer-credential verification: the authoritative permission check.
//!
//! VLT-PM48 §4.1 states the hard requirement this module exists to satisfy:
//! filesystem permissions on the socket path are necessary but not
//! sufficient. A world-readable parent directory, a misconfigured umask, or a
//! filesystem that does not enforce Unix permissions the way the local host
//! expects can all leave a socket reachable by another local user even when
//! its own mode bit says `0600`. The kernel-verified identity of the actual
//! connecting process is the check this crate treats as authoritative; the
//! file mode ([`super::server`]) is defense in depth on top of it, not a
//! substitute for it.
//!
//! Two kernel mechanisms exist for this on the platforms this product ships
//! for:
//!
//! - Linux: `SO_PEERCRED`, returning a `ucred { pid, uid, gid }`.
//! - macOS and the BSDs: `getpeereid`, returning a `uid_t`/`gid_t` pair
//!   directly (macOS also exposes `LOCAL_PEERCRED` at the socket-option
//!   level, but `getpeereid` is the portable libc entry point across this
//!   family and is what this module uses).
//!
//! Both are queried *before* a single byte of the request is read — see
//! [`super::server`] — so an unauthorized peer receives no protocol response
//! at all, not even a typed rejection.

use std::os::unix::net::UnixStream;

/// Peer identity could not be established.
///
/// Deliberately one variant: whatever the platform-specific reason, the
/// connection is refused. A caller has no legitimate use for "which syscall
/// failed and why" on a local IPC boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerCredentialError;

/// Query the kernel-verified real user ID of the process on the other end of
/// `stream`.
///
/// # Errors
///
/// Returns [`PeerCredentialError`] if the platform has no supported
/// mechanism, or if the kernel call itself fails. Both fail closed: see
/// [`is_same_user`].
#[cfg(target_os = "linux")]
pub fn peer_uid(stream: &UnixStream) -> Result<u32, PeerCredentialError> {
    use std::mem::size_of;
    use std::os::unix::io::AsRawFd;
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let expected_length = size_of::<libc::ucred>() as libc::socklen_t;
    let mut length = expected_length;
    // SAFETY: `credentials` is a local, correctly sized `libc::ucred`, and
    // `length` is set to its exact size before the call, which the kernel
    // will only ever shrink, not grow.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    // The kernel is trusted to fill exactly `sizeof(ucred)` bytes on
    // success; a shorter write would mean `credentials.uid` was read from
    // memory this call never actually initialized. Both a nonzero return and
    // a shrunk `length` fail closed the same way.
    if result != 0 || length != expected_length {
        return Err(PeerCredentialError);
    }
    Ok(credentials.uid)
}

/// Query the kernel-verified real user ID of the process on the other end of
/// `stream`, via `getpeereid` (macOS and the BSD family).
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub fn peer_uid(stream: &UnixStream) -> Result<u32, PeerCredentialError> {
    use std::os::unix::io::AsRawFd;
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: `uid` and `gid` are local, correctly typed out-parameters; the
    // socket descriptor is borrowed for the duration of the call only.
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result != 0 {
        return Err(PeerCredentialError);
    }
    Ok(uid)
}

/// No supported peer-credential mechanism on this platform: fail closed.
///
/// Unreachable through the shipped product — [`super::server::AgentServer`]
/// only binds a socket on the platforms above — but a compile-time fallback
/// is preferable to leaving this function undefined for a Unix target this
/// crate has not been audited against, since the alternative would be
/// accepting every connection unconditionally rather than refusing them.
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
pub fn peer_uid(_stream: &UnixStream) -> Result<u32, PeerCredentialError> {
    Err(PeerCredentialError)
}

/// This process's own real user ID.
fn own_uid() -> u32 {
    // SAFETY: `getuid` takes no arguments, cannot fail, and touches no memory
    // this process does not already own.
    unsafe { libc::getuid() }
}

/// The security-critical comparison, isolated as pure data.
///
/// Split out from [`is_same_user`] so the actual authorization rule — exact
/// equality, nothing looser — is testable against a fabricated mismatched
/// UID without needing a second real local user account or a setuid helper,
/// neither of which a CI runner can provide. Root is granted no special
/// case: an agent started as an unprivileged user is not reachable by root
/// through this check, matching every other owner-only object this product
/// creates (`vault-pm-local-host`'s config, lock file, and object roots
/// apply the identical rule).
const fn authorized(peer_uid: u32, own_uid: u32) -> bool {
    peer_uid == own_uid
}

/// Whether the connecting peer is this same local user.
///
/// The whole check, stated as one function so it is not buried inside a
/// longer accept loop: a peer is authorized if and only if its
/// kernel-reported real UID equals this process's own real UID.
pub fn is_same_user(stream: &UnixStream) -> bool {
    peer_uid(stream).is_ok_and(|uid| authorized(uid, own_uid()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_connected_pair_reports_this_process_as_the_peer() {
        let (first, second) = UnixStream::pair().unwrap();
        assert_eq!(peer_uid(&first).unwrap(), own_uid());
        assert_eq!(peer_uid(&second).unwrap(), own_uid());
        assert!(is_same_user(&first));
        assert!(is_same_user(&second));
    }

    /// The comparison logic itself, exercised against a fabricated mismatch.
    ///
    /// A genuinely different-UID connection cannot be simulated portably in
    /// this environment (it would require a second real user account and a
    /// setuid helper — see this crate's spec, VLT-PM48 §8, for why that gap
    /// is accepted rather than worked around). What this test proves
    /// directly instead: [`authorized`] is exact equality and nothing
    /// looser, so every UID that merely differs from the expected one — by
    /// one, by wrapping around zero, or by an arbitrary unrelated value — is
    /// rejected, and the one value that matches is accepted.
    #[test]
    fn the_comparison_rejects_every_uid_other_than_the_expected_one() {
        let expected = 1_000_u32;
        for candidate in [
            expected.wrapping_add(1),
            expected.wrapping_sub(1),
            0,
            u32::MAX,
            u32::MAX / 2,
        ] {
            assert!(
                !authorized(candidate, expected),
                "{candidate} must be refused"
            );
        }
        assert!(authorized(expected, expected));
    }
}
