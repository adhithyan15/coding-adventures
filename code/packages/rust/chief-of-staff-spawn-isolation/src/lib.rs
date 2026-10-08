//! # Descriptor isolation at the agent spawn site (D18S S-I2, S-I3)
//!
//! An agent process must start holding exactly three descriptors: its
//! channel on stdin and stdout, and a stderr that goes nowhere useful to it.
//! Anything more is an escape that needs no syscall the sandbox denies,
//! because the process already holds the object. A leaked socket to the
//! supervisor, a leaked file, or a terminal it can push keystrokes into
//! (`ioctl(TIOCSTI)`) are each complete.
//!
//! [`isolate`] arranges four things on a [`Command`] before it is spawned:
//!
//! ```text
//!   fd   what the child gets        why
//!   ---  -------------------------  ------------------------------------------
//!   0,1  the caller's pipes         the channel (S-I2); refused if a terminal
//!   2    /dev/null                  runtime noise must not enter the protocol
//!                                   stream, nor reach the supervisor (S-I2)
//!   3+   nothing: close-on-exec     a leaked descriptor is a complete escape
//!                                   (S-I3)
//! ```
//!
//! And on Unix the child starts a **new session** (`setsid`), so it has no
//! controlling terminal. Checking fds 0-2 alone is not enough: a child left
//! in the supervisor's session can `open("/dev/tty")` and push keystrokes
//! into the terminal the supervisor was started from. After `setsid`, that
//! open fails, and Linux refuses `TIOCSTI` on any terminal that is not the
//! caller's controlling one. Opening a terminal *by its path*
//! (`/dev/pts/N`) is the sandbox's job (S-I2's `TIOCSTI` filter, Landlock).
//!
//! ## Why mark close-on-exec, rather than close
//!
//! D18S S-I3 asks for `close_range(3, ~0U, 0)` between fork and exec.
//! Closing breaks one thing: `std::process::Command` keeps a pipe open in
//! the child, itself close-on-exec, to report a failed `exec` back to the
//! parent. Closed early, a failed exec looks like a child that started and
//! exited. Marking every descriptor above 2 close-on-exec has the same
//! effect *at* exec, every one of them closed, and keeps that pipe working
//! until then.
//!
//! ```text
//!   Linux     close_range(3, ~0U, CLOSE_RANGE_CLOEXEC)    one syscall (5.11+)
//!             else: every fd listed in /proc/self/fd, read with getdents64
//!             into a stack buffer; that sees every open descriptor, however
//!             high. If neither works, the spawn is refused.
//!   others    fcntl(fd, F_SETFD, FD_CLOEXEC) for every open fd from 3 up to
//!             the larger of the soft and hard descriptor limits, read in
//!             the parent and capped at 2^20. A descriptor above that bound,
//!             only possible if the limit was lowered after it was opened,
//!             is not reached.
//!   Windows   stderr is NUL, and nothing else: no terminal check, no session,
//!             and inheritable handles still pass to the child, since std
//!             spawns with bInheritHandles=TRUE. The explicit handle list S-I3
//!             asks for needs PROC_THREAD_ATTRIBUTE_HANDLE_LIST, which stable
//!             std cannot pass (D18S build step 8).
//! ```
//!
//! ## Why this crate exists
//!
//! `CommandExt::pre_exec` is `unsafe`, and the supervisor crates are
//! `#![forbid(unsafe_code)]`. The one hook lives here, small enough to
//! review on its own, and the callers stay safe.

#![deny(unsafe_op_in_unsafe_fn)]

use std::process::{Command, Stdio};

/// Isolate `command`'s descriptors for an agent spawn.
///
/// Call it last, after stdin and stdout are set: it sets stderr itself, and
/// a later `stderr(..)` would undo that. Do not also set a process group:
/// the child calls `setsid`, which a group leader cannot.
///
/// On Unix, a spawn whose stdin, stdout or stderr is a terminal fails with
/// [`std::io::ErrorKind::PermissionDenied`]. On Windows only stderr is set;
/// see the module documentation.
pub fn isolate(command: &mut Command) -> &mut Command {
    command.stderr(Stdio::null());
    #[cfg(unix)]
    unix::install(command);
    command
}

#[cfg(unix)]
mod unix {
    use std::io;
    use std::mem::MaybeUninit;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    /// The highest descriptor number the fallback loop visits. A limit of
    /// "unlimited", or a very large one, is capped here so a spawn never
    /// spends seconds in `fcntl`.
    const LOOP_CEILING: libc::c_int = 1 << 20;

    pub(super) fn install(command: &mut Command) {
        // Read in the parent: `getrlimit` is not async-signal-safe, so it
        // must not run between fork and exec.
        let limit = descriptor_limit();
        // SAFETY: the closure runs in the forked child, before exec, where
        // another thread of the parent may have held a lock at fork time. It
        // therefore calls only async-signal-safe functions (`tcgetattr`,
        // `setsid`, `fcntl`, `open`, `close`, and the raw `close_range` and
        // `getdents64` syscalls), allocates nothing, and touches no state
        // shared with the parent. The errors it builds,
        // `io::Error::from_raw_os_error` and `last_os_error`, do not allocate.
        unsafe {
            command.pre_exec(move || isolate_in_child(limit));
        }
    }

    fn descriptor_limit() -> libc::c_int {
        let mut limit = MaybeUninit::<libc::rlimit>::uninit();
        // SAFETY: `getrlimit` writes one `rlimit` through the pointer and
        // returns 0, or returns -1 and writes nothing.
        let bound = if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, limit.as_mut_ptr()) } == 0 {
            // SAFETY: initialized by the successful call above.
            let limit = unsafe { limit.assume_init() };
            // The hard limit too: a descriptor opened before the soft limit
            // was lowered can sit above the soft limit.
            limit.rlim_cur.max(limit.rlim_max)
        } else {
            LOOP_CEILING as libc::rlim_t
        };
        libc::c_int::try_from(bound)
            .unwrap_or(LOOP_CEILING)
            .clamp(256, LOOP_CEILING)
    }

    /// Between fork and exec: refuse a terminal, leave the supervisor's
    /// session, then mark everything above fd 2 close-on-exec.
    fn isolate_in_child(limit: libc::c_int) -> io::Result<()> {
        for fd in 0..=2 {
            if is_terminal(fd) {
                // S-I2: a channel descriptor that is a terminal grants
                // `ioctl(TIOCSTI)`, command execution as the supervisor's user.
                return Err(io::Error::from_raw_os_error(libc::EPERM));
            }
        }
        // No controlling terminal: `/dev/tty` no longer opens. The forked
        // child is never a process-group leader, so this cannot fail with
        // EPERM unless a caller set a process group (see `isolate`).
        // SAFETY: `setsid` takes no arguments and is async-signal-safe.
        if unsafe { libc::setsid() } == -1 {
            return Err(io::Error::last_os_error());
        }
        mark_close_on_exec_above_stderr(limit)
    }

    /// `isatty` is not on POSIX's async-signal-safe list; `tcgetattr`, which
    /// is what it does, is.
    fn is_terminal(fd: libc::c_int) -> bool {
        let mut termios = MaybeUninit::<libc::termios>::uninit();
        // SAFETY: `tcgetattr` writes one `termios` through the pointer when
        // `fd` is a terminal, and fails without writing otherwise. The value
        // is never read.
        unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) == 0 }
    }

    #[cfg(target_os = "linux")]
    fn mark_close_on_exec_above_stderr(limit: libc::c_int) -> io::Result<()> {
        // SAFETY: a raw syscall with three integer arguments and no memory.
        let marked = unsafe {
            libc::syscall(
                libc::SYS_close_range,
                3 as libc::c_uint,
                libc::c_uint::MAX,
                libc::CLOSE_RANGE_CLOEXEC,
            )
        } == 0;
        if marked {
            return Ok(());
        }
        // ENOSYS before 5.9, EINVAL for the flag before 5.11, or a seccomp
        // profile that denies it. Listing /proc/self/fd sees every open
        // descriptor, however high; a bounded loop would not. If /proc is
        // not there either, refuse the spawn rather than guess.
        let _ = limit;
        // A listing that cannot be read is reported as EPERM, like the
        // terminal refusal, rather than as its own errno: ENOENT for a
        // missing /proc would read to the caller like a missing program.
        mark_listed_in_proc().map_err(|_| io::Error::from_raw_os_error(libc::EPERM))
    }

    /// Mark every descriptor above 2 listed in `/proc/self/fd`.
    ///
    /// Async-signal-safe: a raw `open`, `getdents64` into a stack buffer and
    /// `fcntl`, with no allocation. Marking does not close anything, so the
    /// listing stays valid while it is walked.
    #[cfg(target_os = "linux")]
    pub(super) fn mark_listed_in_proc() -> io::Result<()> {
        // SAFETY: a NUL-terminated path literal; `open` returns a descriptor
        // or -1.
        let directory = unsafe {
            libc::open(
                c"/proc/self/fd".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if directory == -1 {
            return Err(io::Error::last_os_error());
        }
        let result = (|| {
            let mut buffer = [0u8; 4096];
            loop {
                // SAFETY: `getdents64` writes at most `buffer.len()` bytes of
                // `linux_dirent64` records into the buffer, and returns how
                // many it wrote, 0 at the end, or -1.
                let filled = unsafe {
                    libc::syscall(
                        libc::SYS_getdents64,
                        directory,
                        buffer.as_mut_ptr(),
                        buffer.len(),
                    )
                };
                if filled < 0 {
                    return Err(io::Error::last_os_error());
                }
                if filled == 0 {
                    return Ok(());
                }
                let filled = filled as usize;
                // linux_dirent64: d_ino u64, d_off i64, d_reclen u16,
                // d_type u8, then the NUL-terminated name, at offset 19.
                let mut offset = 0;
                while offset + 19 <= filled {
                    let length =
                        u16::from_ne_bytes([buffer[offset + 16], buffer[offset + 17]]) as usize;
                    if length < 19 || offset + length > filled {
                        return Err(io::Error::from_raw_os_error(libc::EIO));
                    }
                    if let Some(fd) = parse_fd(&buffer[offset + 19..offset + length]) {
                        if fd > 2 && fd != directory {
                            mark_one(fd)?;
                        }
                    }
                    offset += length;
                }
            }
        })();
        // SAFETY: closing the directory descriptor opened above.
        unsafe { libc::close(directory) };
        result
    }

    /// A directory entry name, NUL-terminated, as a descriptor number.
    /// `.` and `..` are not numbers and come back `None`.
    #[cfg(target_os = "linux")]
    fn parse_fd(name: &[u8]) -> Option<libc::c_int> {
        let mut fd: libc::c_int = 0;
        let mut digits = 0;
        for &byte in name {
            if byte == 0 {
                break;
            }
            if !byte.is_ascii_digit() {
                return None;
            }
            fd = fd
                .checked_mul(10)?
                .checked_add(libc::c_int::from(byte - b'0'))?;
            digits += 1;
        }
        (digits > 0).then_some(fd)
    }

    #[cfg(not(target_os = "linux"))]
    fn mark_close_on_exec_above_stderr(limit: libc::c_int) -> io::Result<()> {
        mark_each(limit)
    }

    /// The portable loop: one `fcntl` pair per open descriptor below
    /// `limit`.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    fn mark_each(limit: libc::c_int) -> io::Result<()> {
        for fd in 3..limit {
            mark_one(fd)?;
        }
        Ok(())
    }

    /// Mark `fd` close-on-exec, if it is open.
    fn mark_one(fd: libc::c_int) -> io::Result<()> {
        // SAFETY: `fcntl` with F_GETFD/F_SETFD reads and writes only the
        // descriptor's flags; on a closed fd it fails with EBADF.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags == -1 {
            return Ok(());
        }
        // SAFETY: as above.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_fallback_limit_is_bounded() {
            let limit = descriptor_limit();
            assert!((256..=LOOP_CEILING).contains(&limit));
        }

        fn is_cloexec(fd: libc::c_int) -> bool {
            // SAFETY: a flag read on a descriptor the test owns.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            flags & libc::FD_CLOEXEC != 0
        }

        /// A descriptor without FD_CLOEXEC, at `at` if given.
        fn leak(at: Option<libc::c_int>) -> libc::c_int {
            // SAFETY: `dup`/`dup2` of stdout return a new descriptor without
            // FD_CLOEXEC, or -1.
            let fd = unsafe {
                match at {
                    None => libc::dup(1),
                    Some(at) => libc::dup2(1, at),
                }
            };
            assert!(fd > 2);
            assert!(!is_cloexec(fd));
            fd
        }

        // One test, so the steps that mark this process's own descriptors
        // never run concurrently with a step that expects one unmarked.
        #[test]
        fn each_marking_strategy_reaches_an_open_descriptor() {
            let low = leak(None);
            mark_each(low + 1).unwrap();
            assert!(is_cloexec(low));

            #[cfg(target_os = "linux")]
            {
                // The highest descriptor the soft limit allows: the listing
                // reaches it with no bound to get wrong.
                let mut limit = MaybeUninit::<libc::rlimit>::uninit();
                // SAFETY: `getrlimit` writes one `rlimit`, or fails.
                assert_eq!(
                    unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, limit.as_mut_ptr()) },
                    0
                );
                // SAFETY: initialized by the successful call.
                let soft = unsafe { limit.assume_init() }.rlim_cur;
                let top = libc::c_int::try_from(soft.saturating_sub(1)).unwrap_or(65_535);
                let high = leak(Some(top.min(65_535)));
                mark_listed_in_proc().unwrap();
                assert!(is_cloexec(high));
                assert_eq!(parse_fd(b"123\0"), Some(123));
                assert_eq!(parse_fd(b".\0"), None);
                assert_eq!(parse_fd(b"..\0"), None);
                assert_eq!(parse_fd(b"\0"), None);
                // SAFETY: closing a descriptor this test opened.
                unsafe { libc::close(high) };
            }
            // SAFETY: as above.
            unsafe { libc::close(low) };
        }
    }
}
