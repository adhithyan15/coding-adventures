//! D18S build step 5, the shim: everything the hook checks and does around
//! installing the policy.
//!
//! ```text
//!   parent (supervisor)                     child (forked, single-threaded)
//!   -----------------------------------     ----------------------------------
//!   apply():
//!     environment checked (closed set)
//!     socketpair; child end → fd ≥ 512
//!     exec-once thread started ◄─────┐      no_new_privs
//!   spawn() ─── fork ───────────────────►   one thread? only 0-2 survive exec?
//!                                    │      landlock_restrict_self
//!                                    │      seccomp(NEW_LISTENER) → listener
//!     thread: recvmsg(SCM_RIGHTS) ◄──┴───── sendmsg(listener); close it
//!                                           launch probes (S-P4)
//!     thread: NOTIF_RECV  ◄──────────────── execveat(binary)  ← USER_NOTIF
//!     thread: NOTIF_SEND CONTINUE ──────►   the exec proceeds
//!     thread: close(listener)               the agent runs; any later
//!                                           execveat: ENOSYS, no listener
//! ```
//!
//! Exec once is S-I4d's second option. Seccomp filters survive exec, so the
//! exec the hook makes would otherwise stay available to the agent. Here it
//! is routed to a listener the supervisor holds, which lets exactly one exec
//! through and then goes away.
//!
//! Everything on the child side runs between fork and exec, so it allocates
//! nothing: stack buffers and raw syscalls only.

use crate::ConfinementError;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

// ---------------------------------------------------------------------------
// The environment's closed set (S-I4a)
// ---------------------------------------------------------------------------

/// The environment variable names an agent may be given, enumerated here as
/// S-I4a requires. Anything else refuses the launch. Adding a name means
/// amending D18S, not this list alone.
pub const GRANTABLE_ENVIRONMENT: [&str; 11] = [
    "TZ",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_COLLATE",
    "LC_CTYPE",
    "LC_MESSAGES",
    "LC_MONETARY",
    "LC_NUMERIC",
    "LC_TIME",
    "NO_COLOR",
];

/// S-I4a's deny-list, kept as a redundant second check. Every name here runs
/// code during some runtime's initialization. It is illustrative: the closed
/// set above is what keeps a new runtime's variable out.
const DENIED_PREFIXES: [&str; 5] = ["LD_", "DYLD_", "PYTHON", "COMPlus_", "DOTNET_"];
const DENIED_NAMES: [&str; 31] = [
    "GLIBC_TUNABLES",
    "NODE_OPTIONS",
    "NODE_PATH",
    "ELECTRON_RUN_AS_NODE",
    "BASH_ENV",
    "ENV",
    "SHELLOPTS",
    "PS4",
    "RUBYOPT",
    "RUBYLIB",
    "GEM_HOME",
    "GEM_PATH",
    "PERL5OPT",
    "PERL5LIB",
    "CLASSPATH",
    "JAVA_TOOL_OPTIONS",
    "_JAVA_OPTIONS",
    "JDK_JAVA_OPTIONS",
    "DOTNET_STARTUP_HOOKS",
    "CORECLR_PROFILER",
    "LUA_PATH",
    "LUA_CPATH",
    "JULIA_LOAD_PATH",
    "R_PROFILE",
    "GCONV_PATH",
    "LOCPATH",
    "PYTHONSTARTUP",
    "PYTHONPATH",
    "LD_PRELOAD",
    "LD_AUDIT",
    "DYLD_INSERT_LIBRARIES",
];

/// Refuse any name outside the closed set, and, redundantly, any name on the
/// deny-list. Values are constrained too (review round 4, L4): `TZ=/path` or
/// a locale value with a path in it makes glibc read that file at start. The
/// sandbox would bound the read, but a value is a name, not a path: letters,
/// digits and `._+-@,`, with `/` allowed only inside (`America/New_York`)
/// and never `..`.
pub(crate) fn check_environment<'a>(
    variables: impl IntoIterator<Item = (&'a [u8], &'a [u8])>,
) -> Result<(), ConfinementError> {
    for (name, value) in variables {
        let shown = || String::from_utf8_lossy(name).into_owned();
        let denied = DENIED_NAMES.iter().any(|denied| denied.as_bytes() == name)
            || DENIED_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix.as_bytes()));
        if denied {
            return Err(ConfinementError::Environment(format!(
                "{}: runs code at runtime start (S-I4a deny-list)",
                shown()
            )));
        }
        if !GRANTABLE_ENVIRONMENT
            .iter()
            .any(|granted| granted.as_bytes() == name)
        {
            return Err(ConfinementError::Environment(format!(
                "{}: not in the grantable set",
                shown()
            )));
        }
        let plain = value
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-@,/".contains(byte));
        if !plain || value.starts_with(b"/") || value.windows(2).any(|pair| pair == b"..") {
            return Err(ConfinementError::Environment(format!(
                "{}: its value must be a plain name, not a path",
                shown()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Exec once: the parent's side
// ---------------------------------------------------------------------------

/// One `apply`'s exec-once service: the socket end the child sends its
/// listener on, and the thread that answers it.
pub(crate) struct ExecOnce {
    /// The child's end, at a fixed high number the seccomp program allows
    /// `sendmsg` on. Close-on-exec: no agent ever holds it.
    pub(crate) child_end: OwnedFd,
}

impl ExecOnce {
    /// Make the socketpair and start the thread. The thread serves every
    /// child spawned from the command, one listener each, and exits when
    /// the last copy of the child end is closed.
    pub(crate) fn start() -> io::Result<Self> {
        let mut ends = [-1; 2];
        // SAFETY: writes two descriptors into `ends`, or fails.
        let made = unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                ends.as_mut_ptr(),
            )
        };
        if made != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: both were just returned by the kernel, and nothing else
        // owns them.
        let (parent_end, child_end) =
            unsafe { (OwnedFd::from_raw_fd(ends[0]), OwnedFd::from_raw_fd(ends[1])) };
        let child_end = high(child_end)?;
        std::thread::Builder::new()
            .name("exec-once".into())
            .spawn(move || serve(parent_end))?;
        Ok(Self { child_end })
    }
}

/// Move `fd` to a number at or above 512, close-on-exec.
fn high(fd: OwnedFd) -> io::Result<OwnedFd> {
    // SAFETY: duplicates an open descriptor; returns a new one or -1.
    let moved = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 512) };
    if moved < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh descriptor; `fd` closes the original when it drops.
    Ok(unsafe { OwnedFd::from_raw_fd(moved) })
}

/// Receive each child's listener in turn and let its one exec through.
///
/// Only the supervisor and its forked children, before they exec, can send
/// here; the agent never holds the child end. Even so (review round 4,
/// L1) nothing received is trusted: a message that is not exactly one
/// descriptor, or a descriptor that is not a seccomp listener, is closed and
/// skipped, and the service keeps going. It stops only when every copy of
/// the child end is closed.
fn serve(parent_end: OwnedFd) {
    loop {
        match receive(parent_end.as_raw_fd()) {
            Received::Closed => return,
            Received::Nothing => continue,
            Received::Descriptor(fd, sender) => {
                if is_listener(&fd) {
                    continue_first_exec(fd, sender);
                }
            }
        }
    }
}

enum Received {
    /// Every sender is gone (or the socket failed): stop serving.
    Closed,
    /// A message without exactly one descriptor: ignore it.
    Nothing,
    /// One descriptor, and the pid its sender put in the payload.
    Descriptor(OwnedFd, libc::pid_t),
}

/// `recvmsg` one message. Every descriptor it carries is taken into an
/// `OwnedFd`, so any beyond the first is closed, not leaked.
fn receive(socket: RawFd) -> Received {
    let mut payload = [0u8; 4];
    let mut iov = libc::iovec {
        iov_base: payload.as_mut_ptr().cast(),
        iov_len: payload.len(),
    };
    // Room for several descriptors, so extras arrive (and are closed)
    // rather than being truncated into the void.
    let mut control = [0u64; 16];
    // SAFETY: an all-zero msghdr is valid; the fields set below point at
    // live buffers of the sizes given.
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = std::mem::size_of_val(&control) as _;
    // SAFETY: `message` describes valid buffers; MSG_CMSG_CLOEXEC marks the
    // received descriptors close-on-exec.
    let received = unsafe { libc::recvmsg(socket, &mut message, libc::MSG_CMSG_CLOEXEC) };
    if received < 0 {
        return if io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
            Received::Nothing
        } else {
            Received::Closed
        };
    }
    if received == 0 {
        return Received::Closed;
    }
    let mut descriptors: Vec<OwnedFd> = Vec::new();
    // SAFETY: the kernel filled `control`; the CMSG_* walk stays inside it.
    let mut header = unsafe { libc::CMSG_FIRSTHDR(&message) };
    while !header.is_null() {
        // SAFETY: a header the CMSG walk returned, inside `control`.
        let current = unsafe { &*header };
        if current.cmsg_level == libc::SOL_SOCKET && current.cmsg_type == libc::SCM_RIGHTS {
            // SAFETY: CMSG_LEN(0) is arithmetic.
            let payload = current.cmsg_len as usize - unsafe { libc::CMSG_LEN(0) } as usize;
            for index in 0..payload / std::mem::size_of::<RawFd>() {
                // SAFETY: `index` is within this header's payload.
                let fd = unsafe {
                    std::ptr::read_unaligned(libc::CMSG_DATA(current).cast::<RawFd>().add(index))
                };
                // SAFETY: the kernel installed it in this process for us.
                descriptors.push(unsafe { OwnedFd::from_raw_fd(fd) });
            }
        }
        // SAFETY: advances within `message`'s control buffer, or to null.
        header = unsafe { libc::CMSG_NXTHDR(&message, header) };
    }
    if message.msg_flags & libc::MSG_CTRUNC != 0
        || descriptors.len() != 1
        || received as usize != payload.len()
    {
        return Received::Nothing;
    }
    Received::Descriptor(
        descriptors.pop().unwrap(),
        libc::pid_t::from_ne_bytes(payload),
    )
}

/// Whether `fd` is a seccomp listener. `NOTIF_ID_VALID` on an id that was
/// never issued answers ENOENT on a listener, and ENOTTY or EINVAL on
/// anything else.
fn is_listener(fd: &OwnedFd) -> bool {
    let id: u64 = u64::MAX;
    // SAFETY: a valid descriptor and a pointer to a u64, as the ioctl takes.
    let answer = unsafe { libc::ioctl(fd.as_raw_fd(), libc::SECCOMP_IOCTL_NOTIF_ID_VALID, &id) };
    answer != 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT)
}

/// How long the service waits for a child's exec after receiving its
/// listener. The child sends the listener just before its probes and exec,
/// so this is generous; when it lapses the listener is dropped and that
/// child's exec fails with ENOSYS rather than the spawn hanging.
const EXEC_WAIT_MS: libc::c_int = 30_000;

/// Answer the first notification on `listener` with CONTINUE, then drop
/// it. If anything fails, the listener is dropped all the same, and the
/// child's exec fails with ENOSYS: the spawn errors rather than hangs.
fn continue_first_exec(listener: OwnedFd, sender: libc::pid_t) {
    let mut poll = libc::pollfd {
        fd: listener.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one valid pollfd. A child that dies before its exec hangs
    // up the listener, and poll returns with POLLHUP.
    let ready = unsafe { libc::poll(&mut poll, 1, EXEC_WAIT_MS) };
    if ready != 1 || poll.revents & libc::POLLIN == 0 {
        return;
    }
    // SAFETY: the kernel requires a zeroed buffer for NOTIF_RECV.
    let mut notification: libc::seccomp_notif = unsafe { std::mem::zeroed() };
    // SAFETY: a valid listener and a buffer of the size the ioctl names.
    if unsafe {
        libc::ioctl(
            listener.as_raw_fd(),
            libc::SECCOMP_IOCTL_NOTIF_RECV,
            &mut notification,
        )
    } != 0
    {
        return;
    }
    // Only the sender's own execveat is let through (review round 5, L1):
    // a listener from anywhere else gets no CONTINUE.
    let exec = notification.data.nr == libc::SYS_execveat as i32
        && notification.pid as libc::pid_t == sender;
    let response = libc::seccomp_notif_resp {
        id: notification.id,
        val: 0,
        // Only execveat is routed here, from the sender; anything else is
        // refused.
        error: if exec { 0 } else { -libc::EPERM },
        flags: if exec {
            libc::SECCOMP_USER_NOTIF_FLAG_CONTINUE as u32
        } else {
            0
        },
    };
    // SAFETY: a valid listener and response. The result does not matter:
    // the listener closes next either way.
    unsafe {
        libc::ioctl(
            listener.as_raw_fd(),
            libc::SECCOMP_IOCTL_NOTIF_SEND,
            &response,
        )
    };
}

// ---------------------------------------------------------------------------
// The child's side: no allocation from here down
// ---------------------------------------------------------------------------

/// Send `listener` over `socket` with `SCM_RIGHTS`, and this process's pid
/// as the payload, so the service answers only this process's exec.
pub(crate) fn send_listener(socket: RawFd, listener: RawFd) -> io::Result<()> {
    // SAFETY: getpid cannot fail.
    let mut payload = unsafe { libc::getpid() }.to_ne_bytes();
    let mut iov = libc::iovec {
        iov_base: payload.as_mut_ptr().cast(),
        iov_len: payload.len(),
    };
    let mut control = [0u64; 4];
    // SAFETY: as in `receive_descriptor`; the control buffer is 32 bytes,
    // more than CMSG_SPACE(4).
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    // SAFETY: CMSG_SPACE and CMSG_LEN are arithmetic.
    message.msg_controllen = unsafe { libc::CMSG_SPACE(4) } as _;
    // SAFETY: the control buffer holds one header.
    let header = unsafe { &mut *libc::CMSG_FIRSTHDR(&message) };
    header.cmsg_level = libc::SOL_SOCKET;
    header.cmsg_type = libc::SCM_RIGHTS;
    header.cmsg_len = unsafe { libc::CMSG_LEN(4) } as _;
    // SAFETY: CMSG_DATA points inside the control buffer.
    unsafe { std::ptr::write_unaligned(libc::CMSG_DATA(header).cast::<RawFd>(), listener) };
    // SAFETY: a valid socket and message.
    // MSG_NOSIGNAL (review round 4, L2): a dead service is an EPIPE the
    // spawn reports, never a SIGPIPE that kills the child unreported.
    if unsafe { libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) } != payload.len() as isize {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Call `each` with every entry name in `directory`, skipping `.` and `..`.
fn for_each_entry(
    directory: &std::ffi::CStr,
    mut each: impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<()> {
    // SAFETY: a NUL-terminated path; a new descriptor or -1.
    let fd = unsafe {
        libc::open(
            directory.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh descriptor, closed when this returns.
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let mut buffer = [0u8; 2048];
    loop {
        // SAFETY: getdents64 writes at most `buffer.len()` bytes.
        let read = unsafe {
            libc::syscall(
                libc::SYS_getdents64,
                fd.as_raw_fd(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        };
        if read < 0 {
            return Err(io::Error::last_os_error());
        }
        if read == 0 {
            return Ok(());
        }
        let mut at = 0usize;
        while at < read as usize {
            // linux_dirent64: d_ino (8), d_off (8), d_reclen (2), d_type
            // (1), then the NUL-terminated name.
            // Bounds-checked (review round 4, L3): a panic here, after
            // fork, would allocate. A malformed record refuses the spawn.
            let length = match buffer.get(at + 16..at + 18) {
                Some(bytes) => u16::from_ne_bytes([bytes[0], bytes[1]]) as usize,
                None => return Err(refused()),
            };
            let name = match buffer.get(at + 19..at + length) {
                Some(name) if length > 19 => name,
                _ => return Err(refused()),
            };
            let name = &name[..name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name.len())];
            if name != b"." && name != b".." {
                each(name)?;
            }
            at += length;
        }
    }
}

fn refused() -> io::Error {
    io::Error::from_raw_os_error(libc::EPERM)
}

/// S-I4b: exactly one thread, checked through `/proc/self/task`.
pub(crate) fn assert_single_thread() -> io::Result<()> {
    let mut threads = 0;
    for_each_entry(c"/proc/self/task", |_| {
        threads += 1;
        Ok(())
    })?;
    if threads == 1 {
        Ok(())
    } else {
        Err(refused())
    }
}

/// S-I4d step 2: after exec, exactly fds 0, 1 and 2 survive. They must be
/// open, and every other descriptor must be close-on-exec.
pub(crate) fn assert_survivors() -> io::Result<()> {
    for fd in 0..3 {
        // SAFETY: F_GETFD on any number; -1 if it is not open.
        if unsafe { libc::fcntl(fd, libc::F_GETFD) } < 0 {
            return Err(refused());
        }
    }
    for_each_entry(c"/proc/self/fd", |name| {
        let mut fd: i32 = 0;
        for digit in name {
            if !digit.is_ascii_digit() {
                return Err(refused());
            }
            fd = fd
                .saturating_mul(10)
                .saturating_add(i32::from(digit - b'0'));
        }
        if fd <= 2 {
            return Ok(());
        }
        // SAFETY: F_GETFD on a number just listed.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        // A descriptor closed since the listing (the listing's own) is fine.
        if flags >= 0 && flags & libc::FD_CLOEXEC == 0 {
            return Err(refused());
        }
        Ok(())
    })
}

/// S-P4: the launch probes. Each must fail with exactly EACCES; anything
/// else, success above all, means a layer is not in force.
pub(crate) fn probe() -> io::Result<()> {
    let errno = || io::Error::last_os_error().raw_os_error();
    // seccomp: readlinkat is mapped to EACCES. Unconfined, a process can
    // always read its own exe link (or gets ENOENT with no /proc).
    let mut link = [0u8; 64];
    // SAFETY: a NUL-terminated path and a buffer of the length given.
    let read = unsafe {
        libc::readlinkat(
            libc::AT_FDCWD,
            c"/proc/self/exe".as_ptr(),
            link.as_mut_ptr().cast(),
            link.len(),
        )
    };
    if read >= 0 || errno() != Some(libc::EACCES) {
        return Err(refused());
    }
    // Landlock: "/" has no rule. Unconfined, opening it always works.
    // SAFETY: a NUL-terminated path; a descriptor or -1.
    let root = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if root >= 0 {
        // SAFETY: the descriptor just opened.
        unsafe { libc::close(root) };
        return Err(refused());
    }
    if errno() != Some(libc::EACCES) {
        return Err(refused());
    }
    Ok(())
}

/// Which enforcement classes are confirmed at each launch, and which only
/// in CI, for the audit record (S-P4: it must not overclaim).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchVerification {
    pub at_launch: &'static [&'static str],
    pub ci_only: &'static [&'static str],
}

pub(crate) const VERIFICATION: LaunchVerification = LaunchVerification {
    at_launch: &[
        "seccomp: readlinkat returns EACCES (SECCOMP_RET_ERRNO class)",
        "landlock: opening / returns EACCES",
        "single thread before install (S-I4b)",
        "only fds 0-2 survive the exec (S-I4d)",
    ],
    ci_only: &[
        "seccomp kill classes: socket, fork, io_uring, ptrace, kill, ioctl(TIOCSTI), mount, bpf, execve",
        "exec once: a second execveat fails with ENOSYS",
        "landlock: ungranted reads and writes return EACCES",
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_is_a_closed_set() {
        let named = |name: &'static [u8]| (name, b"C".as_slice());
        assert!(check_environment([named(b"TZ"), named(b"LC_ALL")]).is_ok());
        for name in [
            b"HOME".as_slice(),
            b"PATH",
            b"LD_PRELOAD",
            b"LD_ANYTHING",
            b"PYTHONHOME",
            b"NODE_OPTIONS",
            b"GLIBC_TUNABLES",
            b"tz",
            b"",
        ] {
            assert!(
                check_environment([(name, b"C".as_slice())]).is_err(),
                "{name:?}"
            );
        }
    }

    #[test]
    fn values_are_names_not_paths() {
        for value in [
            b"UTC".as_slice(),
            b"America/New_York",
            b"en_US.UTF-8",
            b"C.UTF-8",
            b"de_DE@euro",
            b"1",
            b"",
        ] {
            assert!(
                check_environment([(b"TZ".as_slice(), value)]).is_ok(),
                "{value:?}"
            );
        }
        for value in [
            b"/etc/passwd".as_slice(),
            b":/vault/secrets",
            b"../../vault",
            b"Europe/../../x",
            b"UTC\n",
            b"a b",
            b"$(id)",
        ] {
            assert!(
                check_environment([(b"TZ".as_slice(), value)]).is_err(),
                "{value:?}"
            );
        }
    }

    #[test]
    fn the_deny_list_wins_with_its_own_reason() {
        let error = check_environment([(b"LD_PRELOAD".as_slice(), b"x".as_slice())]).unwrap_err();
        assert!(error.to_string().contains("deny-list"), "{error}");
    }

    #[test]
    fn no_grantable_name_is_denied() {
        for name in GRANTABLE_ENVIRONMENT {
            assert!(
                check_environment([(name.as_bytes(), b"C".as_slice())]).is_ok(),
                "{name}"
            );
        }
    }
}
