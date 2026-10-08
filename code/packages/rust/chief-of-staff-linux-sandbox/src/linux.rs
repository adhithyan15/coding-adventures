//! The Linux half: Landlock, seccomp, and the hook that installs them.
//!
//! Landlock's structures and constants are not in the `libc` crate, so they
//! are declared here from the kernel's UAPI (`linux/landlock.h`), which is a
//! stable ABI.

use crate::{ConfinementError, FileGrant};
use std::ffi::CString;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// What the parent built, shared with every child that installs it.
pub(crate) struct Prepared {
    ruleset: OwnedFd,
    /// The agent executable, opened `O_RDONLY | O_CLOEXEC` here in the
    /// parent. The child execs this descriptor, never a path (S-I4d), so
    /// what runs is the file that was parsed and given the Landlock rule,
    /// even if its path is replaced in between.
    binary: std::fs::File,
    pub(crate) landlock_abi: u32,
}

impl Prepared {
    pub(crate) fn build(executable: &Path, grants: &[FileGrant]) -> Result<Self, ConfinementError> {
        let binary = std::fs::File::open(executable).map_err(|error| {
            ConfinementError::Executable(format!("{}: {error}", executable.display()))
        })?;
        let binary = high_descriptor(binary);
        // The program is built per `apply`, around that spawn's socket; this
        // only refuses an architecture it is not built for, early.
        seccomp::program(binary.as_raw_fd(), binary.as_raw_fd())?;
        let abi = landlock::abi().ok_or(ConfinementError::LandlockUnavailable)?;
        if !grants.is_empty() && abi < landlock::ABI_TRUNCATE {
            return Err(ConfinementError::LandlockTooOld {
                abi,
                needed: landlock::ABI_TRUNCATE,
            });
        }
        use landlock::Target::{Executable, Grant, Interpreter, Optional, Required};
        let ruleset = landlock::Ruleset::new(abi)?;
        // The agent's own image: never also a grant (S-I6), or it could
        // rewrite the code it runs.
        let shown = || executable.display().to_string();
        let mut image = vec![Some(ruleset.add(
            binary.as_raw_fd(),
            landlock::READ_EXECUTE,
            Executable,
            &shown,
        )?)];
        if let Some(interpreter) = elf::interpreter(&binary)? {
            let interpreter = system_interpreter(&interpreter)?;
            image.push(ruleset.allow(&interpreter, landlock::READ_EXECUTE, Interpreter)?);
        }
        for directory in LIBRARY_DIRECTORIES {
            ruleset.allow(Path::new(directory), landlock::READ_TREE, Optional)?;
        }
        ruleset.allow(Path::new("/etc/ld.so.cache"), landlock::READ, Optional)?;
        ruleset.allow(
            Path::new("/dev/null"),
            landlock::READ | landlock::WRITE,
            Required,
        )?;
        ruleset.allow(Path::new("/dev/urandom"), landlock::READ, Optional)?;
        for grant in grants {
            let rights = if grant.write {
                landlock::WRITE | landlock::TRUNCATE
            } else {
                landlock::READ
            };
            // Review round 2, L2: the shared libraries are every agent's
            // runtime image, and the supervisor's too (S-I6). A write grant
            // there could rewrite libc for the whole host.
            if grant.write
                && std::fs::canonicalize(&grant.path).is_ok_and(|path| in_library_directory(&path))
            {
                return Err(ConfinementError::InexpressibleGrant(format!(
                    "{}: a shared library directory is never writable",
                    grant.path
                )));
            }
            let identity = ruleset.allow(Path::new(&grant.path), rights, Grant)?;
            if identity.is_some() && image.contains(&identity) {
                return Err(ConfinementError::InexpressibleGrant(format!(
                    "{}: the agent's own executable",
                    grant.path
                )));
            }
        }
        Ok(Self {
            ruleset: ruleset.into_fd(),
            binary,
            landlock_abi: abi,
        })
    }
}

/// Move `file` to a descriptor number at or above 512, where the agent's
/// own opens will not land by accident. The seccomp program pins `execveat`
/// to this number; a low one could coincide with a file the agent opened
/// (on purpose, the agent can still `dup2` onto it: see the spec's
/// residual). If the limit is too low for that, the file stays where it is.
fn high_descriptor(file: std::fs::File) -> std::fs::File {
    // SAFETY: duplicates an open descriptor; returns a new one or -1.
    let high = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 512) };
    if high < 0 {
        return file;
    }
    // SAFETY: a fresh descriptor the kernel just returned; `file` closes
    // the original when it drops.
    unsafe { std::fs::File::from_raw_fd(high) }
}

/// The system library directories: readable by every agent, and the only
/// place its interpreter may live.
const LIBRARY_DIRECTORIES: [&str; 4] = ["/lib", "/lib64", "/usr/lib", "/usr/lib64"];

/// Resolve the interpreter an executable names, and refuse it unless it is
/// in a system library directory.
///
/// Review M2: `PT_INTERP` is written by the agent's author. Taken on trust,
/// it put a rule of their choosing into the ruleset: a directory became a
/// whole-tree read, and any file became readable and executable. So the
/// name is resolved here (the real loaders are reached through symlinks:
/// `/lib64/ld-linux-x86-64.so.2` → `/usr/lib/x86_64-linux-gnu/...`), the
/// result must lie under a library directory, and `Ruleset::allow` then
/// opens the resolved path with no symlinks, as a regular file.
fn system_interpreter(named: &Path) -> Result<PathBuf, ConfinementError> {
    let outside = || {
        ConfinementError::Executable(format!(
            "interpreter {} is not in a system library directory",
            named.display()
        ))
    };
    let resolved = std::fs::canonicalize(named).map_err(|_| outside())?;
    // Review round 2, L1: inside the directories is not enough, or any
    // helper there (a setuid launcher, say) could be named. It must also be
    // named like a dynamic loader: `ld-linux-x86-64.so.2`, `ld-musl-*.so.1`.
    let loader = resolved
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("ld-") && name.contains(".so"));
    if loader && in_library_directory(&resolved) {
        Ok(resolved)
    } else {
        Err(outside())
    }
}

/// Whether `resolved`, a canonical path, lies strictly inside one of the
/// (canonicalized) library directories.
fn in_library_directory(resolved: &Path) -> bool {
    LIBRARY_DIRECTORIES.iter().any(|directory| {
        std::fs::canonicalize(directory)
            .is_ok_and(|directory| resolved.starts_with(&directory) && resolved != directory)
    })
}

impl Prepared {
    pub(crate) fn exec_descriptor(&self) -> RawFd {
        self.binary.as_raw_fd()
    }
}

/// Make `command` unspawnable: a hook that refuses every spawn with EPERM.
pub(crate) fn poison(command: &mut Command) {
    // SAFETY: the closure only builds a non-allocating io::Error.
    unsafe {
        command.pre_exec(|| Err(io::Error::from_raw_os_error(libc::EPERM)));
    }
}

/// Register the hook that installs `prepared` in the child, and then execs
/// the prepared binary from there.
///
/// The hook never returns on success: it ends in
/// `execveat(binary, "", argv, envp, AT_EMPTY_PATH)`, the one exec the
/// seccomp program allows (S-I4d). `std`'s own exec, by path, would need
/// `execve`, which the program kills. The command still supplies argv: its
/// program, as argv\[0\], then its arguments.
///
/// The environment is a closed set (S-I4a): exactly the variables set on
/// the command with `env`, and nothing inherited, whether or not
/// `env_clear` was called. (An earlier version passed the child's
/// `environ`, which `std` has not yet replaced when the hook runs: the
/// agent got the supervisor's whole environment, tokens included.)
pub(crate) fn install(
    prepared: Arc<Prepared>,
    command: &mut Command,
) -> Result<(), ConfinementError> {
    // S-I4a: only names from the closed set reach the agent.
    crate::shim::check_environment(
        command
            .get_envs()
            .filter_map(|(name, value)| Some((name.as_bytes(), value?.as_bytes()))),
    )?;
    // Exec once: the socket and the thread that lets one exec through.
    let exec_once = crate::shim::ExecOnce::start()
        .map_err(|error| ConfinementError::Landlock(format!("exec-once service: {error}")))?;
    let filter = seccomp::program(prepared.binary.as_raw_fd(), exec_once.child_end.as_raw_fd())?;
    let seal = seccomp::seal()?;
    // argv and envp are built here, in the parent: the child must not
    // allocate.
    let argv = Argv::new(
        std::iter::once(command.get_program().as_bytes().to_vec())
            .chain(command.get_args().map(|arg| arg.as_bytes().to_vec())),
    );
    let envp = Argv::new(command.get_envs().filter_map(|(name, value)| {
        let mut entry = name.as_bytes().to_vec();
        entry.push(b'=');
        entry.extend_from_slice(value?.as_bytes());
        Some(entry)
    }));
    // Everything above can fail; nothing below can. Only now is the command
    // touched (review round 4, M2): a failed `apply` must leave nothing a
    // caller could spawn half-configured. spawn-isolation's hook goes first,
    // then this one.
    chief_of_staff_spawn_isolation::isolate(command);
    // SAFETY: the closure runs in the forked child, before exec. It makes
    // raw syscalls only (prctl, open, getdents64, fcntl, landlock, seccomp,
    // sendmsg, readlinkat, close, execveat), with stack buffers. It
    // allocates nothing: the ruleset fd, the program, the binary, the
    // socket, argv and envp were built in the parent and are only read
    // here. The only errors it builds, `last_os_error` and
    // `from_raw_os_error`, do not allocate.
    unsafe {
        command.pre_exec(move || {
            install_in_child(&prepared, &filter, &seal, exec_once.child_end.as_raw_fd())?;
            exec_in_child(&prepared, &argv, &envp)
        });
    }
    Ok(())
}

/// argv or envp for the child: the strings, and the NULL-terminated
/// pointer array into them. Raw pointers are not `Send`, but these only
/// point into `_strings`, which moves with them.
struct Argv {
    _strings: Vec<CString>,
    pointers: Vec<*const libc::c_char>,
}

impl Argv {
    /// An entry with an interior NUL cannot be passed to `exec` whole; it is
    /// cut at the NUL, as C would read it.
    fn new(entries: impl Iterator<Item = Vec<u8>>) -> Self {
        let strings: Vec<CString> = entries
            .map(|mut entry| {
                if let Some(nul) = entry.iter().position(|byte| *byte == 0) {
                    entry.truncate(nul);
                }
                CString::new(entry).unwrap_or_default()
            })
            .collect();
        let mut pointers: Vec<*const libc::c_char> =
            strings.iter().map(|entry| entry.as_ptr()).collect();
        pointers.push(std::ptr::null());
        Self {
            _strings: strings,
            pointers,
        }
    }
}

// SAFETY: see the type's comment; nothing aliases the strings.
unsafe impl Send for Argv {}
// SAFETY: as above; the child only reads them.
unsafe impl Sync for Argv {}

fn exec_in_child(prepared: &Prepared, argv: &Argv, envp: &Argv) -> io::Result<()> {
    // SAFETY: a valid descriptor, an empty NUL-terminated path, and
    // NULL-terminated argv and envp. On success it does not return.
    unsafe {
        libc::syscall(
            libc::SYS_execveat,
            prepared.binary.as_raw_fd(),
            c"".as_ptr(),
            argv.pointers.as_ptr(),
            envp.pointers.as_ptr(),
            libc::AT_EMPTY_PATH,
        );
    }
    Err(io::Error::last_os_error())
}

fn install_in_child(
    prepared: &Prepared,
    filter: &[libc::sock_filter],
    seal: &[libc::sock_filter],
    exec_once: RawFd,
) -> io::Result<()> {
    // 1. No setuid binary, and no file capability, can ever raise this
    //    process's privileges again. Unprivileged seccomp and Landlock both
    //    require it.
    // SAFETY: integer arguments only.
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // 2. The shim's checks, while /proc is still reachable: one thread
    //    (S-I4b), and nothing but 0-2 survives the exec (S-I4d).
    crate::shim::assert_single_thread()?;
    crate::shim::assert_survivors()?;
    // 3. The filesystem the agent can see.
    // SAFETY: integer arguments; the fd is the parent's ruleset.
    let restricted = unsafe {
        libc::syscall(
            libc::SYS_landlock_restrict_self,
            prepared.ruleset.as_raw_fd(),
            0u32,
        )
    };
    if restricted != 0 {
        return Err(io::Error::last_os_error());
    }
    // 4. The syscalls the agent can make, after the calls above, which it
    //    denies. NEW_LISTENER returns the listener that exec once needs.
    let program = libc::sock_fprog {
        len: filter.len() as libc::c_ushort,
        filter: filter.as_ptr() as *mut libc::sock_filter,
    };
    // SAFETY: `program` points at the parent's filter, which outlives this
    // call; the kernel copies it.
    let listener = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            libc::SECCOMP_FILTER_FLAG_NEW_LISTENER,
            &program as *const libc::sock_fprog,
        )
    };
    if listener < 0 {
        return Err(io::Error::last_os_error());
    }
    let listener = listener as RawFd;
    // 5. Hand the listener to the parent's exec-once thread, and keep no
    //    copy: the agent must never hold it.
    let sent = crate::shim::send_listener(exec_once, listener);
    // SAFETY: the listener this hook just received.
    unsafe { libc::close(listener) };
    sent?;
    // 6. The seal: no more sendmsg, and no more seccomp.
    let sealing = libc::sock_fprog {
        len: seal.len() as libc::c_ushort,
        filter: seal.as_ptr() as *mut libc::sock_filter,
    };
    // SAFETY: as for the first program.
    let sealed = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            0u32,
            &sealing as *const libc::sock_fprog,
        )
    };
    if sealed != 0 {
        return Err(io::Error::last_os_error());
    }
    // 7. The launch probes (S-P4): each layer must answer as installed.
    crate::shim::probe()
}

pub(crate) mod landlock {
    //! A Landlock ruleset, from the kernel's stable UAPI.

    use super::*;

    const CREATE_RULESET_VERSION: u32 = 1 << 0;
    const RULE_PATH_BENEATH: u32 = 1;

    // Filesystem rights, by the ABI that introduced them.
    pub(crate) const EXECUTE: u64 = 1 << 0;
    const WRITE_FILE: u64 = 1 << 1;
    const READ_FILE: u64 = 1 << 2;
    const READ_DIR: u64 = 1 << 3;
    /// ABI 1: every right up to MAKE_SYM.
    const ABI1_FS: u64 = (1 << 13) - 1;
    const REFER: u64 = 1 << 13; // ABI 2
    const TRUNCATE_RIGHT: u64 = 1 << 14; // ABI 3
    const IOCTL_DEV: u64 = 1 << 15; // ABI 5
    /// The rights a rule on a non-directory may carry.
    const FILE_RIGHTS: u64 = EXECUTE | WRITE_FILE | READ_FILE | TRUNCATE_RIGHT | IOCTL_DEV;

    // TCP (ABI 4) and scoping (ABI 6): handled, with no rules, so denied.
    const NET_BIND_TCP: u64 = 1 << 0;
    const NET_CONNECT_TCP: u64 = 1 << 1;
    const SCOPE_ABSTRACT_UNIX_SOCKET: u64 = 1 << 0;
    const SCOPE_SIGNAL: u64 = 1 << 1;

    pub(crate) const ABI_TRUNCATE: u32 = 3;

    pub(crate) const READ: u64 = READ_FILE;
    pub(crate) const WRITE: u64 = WRITE_FILE;
    pub(crate) const TRUNCATE: u64 = TRUNCATE_RIGHT;
    pub(crate) const READ_EXECUTE: u64 = READ_FILE | EXECUTE;
    pub(crate) const READ_TREE: u64 = READ_FILE | READ_DIR;

    #[repr(C)]
    struct RulesetAttr {
        handled_access_fs: u64,
        handled_access_net: u64,
        scoped: u64,
    }

    #[repr(C, packed)]
    struct PathBeneathAttr {
        allowed_access: u64,
        parent_fd: i32,
    }

    /// The running Landlock ABI, or `None` if Landlock is unavailable.
    pub(crate) fn abi() -> Option<u32> {
        // SAFETY: the version query takes a null attribute and size 0.
        let version = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                std::ptr::null::<RulesetAttr>(),
                0usize,
                CREATE_RULESET_VERSION,
            )
        };
        u32::try_from(version).ok().filter(|abi| *abi >= 1)
    }

    /// Every filesystem right the ABI knows: each one is denied unless a
    /// rule grants it.
    fn handled_fs(abi: u32) -> u64 {
        let mut rights = ABI1_FS;
        if abi >= 2 {
            rights |= REFER;
        }
        if abi >= 3 {
            rights |= TRUNCATE_RIGHT;
        }
        if abi >= 5 {
            rights |= IOCTL_DEV;
        }
        rights
    }

    /// `struct open_how` (`linux/openat2.h`).
    #[repr(C)]
    struct OpenHow {
        flags: u64,
        mode: u64,
        resolve: u64,
    }
    const RESOLVE_NO_SYMLINKS: u64 = 0x04;

    /// How a path the ruleset names is treated.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Target {
        /// Skipped if missing (a library directory this system lacks).
        Optional,
        /// The launch fails if missing.
        Required,
        /// A plan grant: required, and an exact regular file.
        Grant,
        /// The executable's resolved interpreter: as exact as a grant.
        Interpreter,
        /// The agent executable itself, already open: a regular file.
        Executable,
    }

    impl Target {
        /// Opened with no symlinks, and refused unless a regular file.
        fn exact(self) -> bool {
            matches!(self, Self::Grant | Self::Interpreter | Self::Executable)
        }
    }

    pub(crate) struct Ruleset {
        fd: OwnedFd,
        handled: u64,
    }

    impl Ruleset {
        pub(crate) fn new(abi: u32) -> Result<Self, ConfinementError> {
            let handled = handled_fs(abi);
            let attr = RulesetAttr {
                handled_access_fs: handled,
                handled_access_net: if abi >= 4 {
                    NET_BIND_TCP | NET_CONNECT_TCP
                } else {
                    0
                },
                scoped: if abi >= 6 {
                    SCOPE_ABSTRACT_UNIX_SOCKET | SCOPE_SIGNAL
                } else {
                    0
                },
            };
            // Older kernels reject fields they do not know, so the size
            // passed covers only the ones this ABI has.
            let size = match abi {
                1..=3 => 8usize,
                4 | 5 => 16,
                _ => 24,
            };
            // SAFETY: `attr` is a valid `landlock_ruleset_attr` prefix of
            // `size` bytes; the call returns a new O_CLOEXEC fd or -1.
            let fd = unsafe {
                libc::syscall(
                    libc::SYS_landlock_create_ruleset,
                    &attr as *const RulesetAttr,
                    size,
                    0u32,
                )
            };
            if fd < 0 {
                return Err(ConfinementError::Landlock(
                    io::Error::last_os_error().to_string(),
                ));
            }
            // SAFETY: a fresh descriptor the kernel just returned.
            let fd = unsafe { OwnedFd::from_raw_fd(fd as RawFd) };
            Ok(Self { fd, handled })
        }

        /// Allow `rights` beneath `path`, and return the file's identity
        /// (device, inode). A missing `Optional` path is skipped and returns
        /// `None`; a missing `Required` one is an error. A `Grant` must be
        /// an existing regular file, reached through no symlink: a directory
        /// would grant its whole tree, and a symlink would grant whatever it
        /// points at.
        pub(crate) fn allow(
            &self,
            path: &Path,
            rights: u64,
            target: Target,
        ) -> Result<Option<(u64, u64)>, ConfinementError> {
            let shown = || path.display().to_string();
            let c_path = CString::new(path.as_os_str().as_bytes())
                .map_err(|_| ConfinementError::Path(shown()))?;
            let flags = libc::O_PATH | libc::O_CLOEXEC;
            let fd = if target.exact() {
                // A grant is opened with `openat2(RESOLVE_NO_SYMLINKS)`: a
                // symlink anywhere in the path refuses the open, so the
                // rule lands on the file the plan names and nothing else.
                // (`openat2` is Linux 5.6; Landlock needs 5.13.)
                let how = OpenHow {
                    flags: flags as u64,
                    mode: 0,
                    resolve: RESOLVE_NO_SYMLINKS,
                };
                // SAFETY: a NUL-terminated path and an `open_how` of the
                // size passed; the call returns a new fd or -1.
                unsafe {
                    libc::syscall(
                        libc::SYS_openat2,
                        libc::AT_FDCWD,
                        c_path.as_ptr(),
                        &how as *const OpenHow,
                        std::mem::size_of::<OpenHow>(),
                    ) as RawFd
                }
            } else {
                // SAFETY: a NUL-terminated path; O_PATH opens without reading.
                unsafe { libc::open(c_path.as_ptr(), flags) }
            };
            if fd < 0 {
                return if target == Target::Optional {
                    Ok(None)
                } else {
                    Err(ConfinementError::Path(shown()))
                };
            }
            // SAFETY: a fresh descriptor the kernel just returned.
            let fd = unsafe { OwnedFd::from_raw_fd(fd) };
            self.add(fd.as_raw_fd(), rights, target, &shown).map(Some)
        }

        /// Allow `rights` on the file `fd` is open on, and return its
        /// identity (device, inode). The rule lands on that inode, whatever
        /// its path names by the time the agent runs.
        pub(crate) fn add(
            &self,
            fd: RawFd,
            rights: u64,
            target: Target,
            shown: &dyn Fn() -> String,
        ) -> Result<(u64, u64), ConfinementError> {
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: `fstat` writes one `stat`, or fails.
            if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
                return Err(ConfinementError::Path(shown()));
            }
            // SAFETY: initialized by the successful call.
            let stat = unsafe { stat.assume_init() };
            let kind = stat.st_mode & libc::S_IFMT;
            if target.exact() && kind != libc::S_IFREG {
                let why = format!("{}: not a regular file", shown());
                return Err(if target == Target::Grant {
                    ConfinementError::InexpressibleGrant(why)
                } else {
                    ConfinementError::Executable(why)
                });
            }
            let is_directory = kind == libc::S_IFDIR;
            let mut allowed = rights & self.handled;
            if !is_directory {
                allowed &= FILE_RIGHTS;
            }
            let rule = PathBeneathAttr {
                allowed_access: allowed,
                parent_fd: fd,
            };
            // SAFETY: a valid rule structure and the ruleset's own fd.
            let added = unsafe {
                libc::syscall(
                    libc::SYS_landlock_add_rule,
                    self.fd.as_raw_fd(),
                    RULE_PATH_BENEATH,
                    &rule as *const PathBeneathAttr,
                    0u32,
                )
            };
            if added != 0 {
                return Err(ConfinementError::Landlock(format!(
                    "{}: {}",
                    shown(),
                    io::Error::last_os_error()
                )));
            }
            #[allow(clippy::unnecessary_cast)] // the widths differ by target
            Ok((stat.st_dev as u64, stat.st_ino as u64))
        }

        pub(crate) fn into_fd(self) -> OwnedFd {
            self.fd
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn handled_rights_grow_with_the_abi() {
            assert_eq!(handled_fs(1), ABI1_FS);
            assert_eq!(handled_fs(2), ABI1_FS | REFER);
            assert_eq!(handled_fs(3), ABI1_FS | REFER | TRUNCATE_RIGHT);
            assert_eq!(handled_fs(4), handled_fs(3));
            assert_eq!(handled_fs(5), handled_fs(3) | IOCTL_DEV);
        }
    }
}

pub(crate) mod seccomp {
    //! The seccomp-BPF program: arch check, allowlist, kill by default.

    use super::*;

    const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;
    const AUDIT_ARCH_AARCH64: u32 = 0xC000_00B7;
    /// x32 syscall numbers on x86_64 carry this bit; they are refused.
    #[cfg(target_arch = "x86_64")]
    const X32_SYSCALL_BIT: u32 = 0x4000_0000;

    // seccomp_data: nr at 0, arch at 4, args[i] at 16 + 8 * i. The low 32
    // bits of an argument come first on a little-endian machine, and every
    // argument filtered here is an int the kernel truncates to 32 bits.
    const NR: u32 = 0;
    const ARCH: u32 = 4;
    const fn argument(index: u32) -> u32 {
        16 + 8 * index
    }

    const LD: u16 = (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16;
    const JEQ: u16 = (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16;
    const JSET: u16 = (libc::BPF_JMP | libc::BPF_JSET | libc::BPF_K) as u16;
    #[cfg(target_arch = "x86_64")]
    const JGE: u16 = (libc::BPF_JMP | libc::BPF_JGE | libc::BPF_K) as u16;
    const RET: u16 = (libc::BPF_RET | libc::BPF_K) as u16;

    const KILL: u32 = libc::SECCOMP_RET_KILL_PROCESS;
    const USER_NOTIF: u32 = libc::SECCOMP_RET_USER_NOTIF;
    const ALLOW: u32 = libc::SECCOMP_RET_ALLOW;
    const ENOSYS: u32 = libc::SECCOMP_RET_ERRNO | (libc::ENOSYS as u32 & libc::SECCOMP_RET_DATA);
    const EACCES: u32 = libc::SECCOMP_RET_ERRNO | (libc::EACCES as u32 & libc::SECCOMP_RET_DATA);

    fn op(code: u16, k: u32) -> libc::sock_filter {
        libc::sock_filter {
            code,
            jt: 0,
            jf: 0,
            k,
        }
    }

    fn jump(code: u16, k: u32, jt: u8, jf: u8) -> libc::sock_filter {
        libc::sock_filter { code, jt, jf, k }
    }

    /// The syscalls a compiled agent may make with any arguments.
    ///
    /// Memory, signals, futexes and threads' bookkeeping; time; reading,
    /// writing and polling its own descriptors; opening files (Landlock
    /// decides which); identity queries; and exiting. `execve` is not
    /// here: the only exec is the argument-matched `execveat` below.
    fn allowed() -> Vec<libc::c_long> {
        let mut calls = vec![
            libc::SYS_read,
            libc::SYS_write,
            libc::SYS_readv,
            libc::SYS_writev,
            libc::SYS_pread64,
            libc::SYS_pwrite64,
            libc::SYS_close,
            libc::SYS_fstat,
            libc::SYS_newfstatat,
            libc::SYS_statx,
            libc::SYS_lseek,
            libc::SYS_mmap,
            libc::SYS_munmap,
            libc::SYS_mprotect,
            libc::SYS_madvise,
            libc::SYS_mremap,
            libc::SYS_brk,
            libc::SYS_rt_sigaction,
            libc::SYS_rt_sigprocmask,
            libc::SYS_rt_sigreturn,
            libc::SYS_sigaltstack,
            libc::SYS_futex,
            libc::SYS_set_robust_list,
            libc::SYS_rseq,
            libc::SYS_set_tid_address,
            libc::SYS_gettid,
            libc::SYS_getpid,
            libc::SYS_getppid,
            libc::SYS_getuid,
            libc::SYS_geteuid,
            libc::SYS_getgid,
            libc::SYS_getegid,
            libc::SYS_sched_getaffinity,
            libc::SYS_sched_yield,
            libc::SYS_getrandom,
            libc::SYS_exit,
            libc::SYS_exit_group,
            libc::SYS_clock_gettime,
            libc::SYS_clock_getres,
            libc::SYS_clock_nanosleep,
            libc::SYS_nanosleep,
            libc::SYS_gettimeofday,
            libc::SYS_ppoll,
            libc::SYS_pselect6,
            libc::SYS_epoll_create1,
            libc::SYS_epoll_ctl,
            libc::SYS_epoll_pwait,
            libc::SYS_eventfd2,
            libc::SYS_fcntl,
            libc::SYS_dup,
            libc::SYS_dup3,
            libc::SYS_openat,
            libc::SYS_faccessat,
            libc::SYS_faccessat2,
            libc::SYS_getcwd,
            libc::SYS_uname,
            libc::SYS_restart_syscall,
        ];
        #[cfg(target_arch = "x86_64")]
        calls.extend([
            libc::SYS_open,
            libc::SYS_stat,
            libc::SYS_lstat,
            libc::SYS_access,
            libc::SYS_poll,
            libc::SYS_select,
            libc::SYS_arch_prctl,
            libc::SYS_dup2,
            libc::SYS_epoll_wait,
        ]);
        calls
    }

    /// Syscalls that fail with `EACCES` instead of killing.
    ///
    /// `readlink` is the one path lookup std reaches for on its own
    /// (`current_exe`), and Landlock does not mediate it. Allowed, it would
    /// read `/proc/<supervisor>/fd/*` and name every file the supervisor
    /// holds open, the vault among them. Refused softly, `current_exe`
    /// returns an error instead of killing the agent.
    fn refused() -> Vec<libc::c_long> {
        let mut calls = vec![libc::SYS_readlinkat];
        #[cfg(target_arch = "x86_64")]
        calls.push(libc::SYS_readlink);
        calls
    }

    /// The seal: a second, stacked filter that kills `sendmsg` and
    /// `seccomp` and allows everything else, leaving the first filter to
    /// decide. Stacked filters only tighten (the strictest answer wins),
    /// so once it is in, neither call exists for the agent.
    pub(crate) fn seal() -> Result<Vec<libc::sock_filter>, ConfinementError> {
        let arch = if cfg!(target_arch = "x86_64") {
            AUDIT_ARCH_X86_64
        } else if cfg!(target_arch = "aarch64") {
            AUDIT_ARCH_AARCH64
        } else {
            return Err(ConfinementError::Unsupported("architecture"));
        };
        let nr = |call: libc::c_long| call as u32;
        Ok(vec![
            op(LD, ARCH),
            jump(JEQ, arch, 1, 0),
            op(RET, KILL),
            op(LD, NR),
            jump(JEQ, nr(libc::SYS_sendmsg), 1, 0),
            jump(JEQ, nr(libc::SYS_seccomp), 0, 1),
            op(RET, KILL),
            op(RET, ALLOW),
        ])
    }

    /// Build the program. An architecture this module does not know is
    /// refused here, rather than run with a filter for another table.
    pub(crate) fn program(
        binary: RawFd,
        exec_once: RawFd,
    ) -> Result<Vec<libc::sock_filter>, ConfinementError> {
        let arch = if cfg!(target_arch = "x86_64") {
            AUDIT_ARCH_X86_64
        } else if cfg!(target_arch = "aarch64") {
            AUDIT_ARCH_AARCH64
        } else {
            return Err(ConfinementError::Unsupported("architecture"));
        };
        let nr = |call: libc::c_long| call as u32;
        let mut program = vec![
            // Arch check first: syscall numbers mean nothing for another ABI.
            op(LD, ARCH),
            jump(JEQ, arch, 1, 0),
            op(RET, KILL),
            op(LD, NR),
        ];
        #[cfg(target_arch = "x86_64")]
        program.extend([jump(JGE, X32_SYSCALL_BIT, 0, 1), op(RET, KILL)]);

        // clone: threads (CLONE_THREAD) only.
        program.extend([
            jump(JEQ, nr(libc::SYS_clone), 0, 4),
            op(LD, argument(0)),
            jump(JSET, libc::CLONE_THREAD as u32, 0, 1),
            op(RET, ALLOW),
            op(RET, KILL),
        ]);
        // execveat: only `binary`, by descriptor (AT_EMPTY_PATH), and then
        // only through the listener (exec once, S-I4d). Filters survive
        // exec, so the hook's own exec must not stay granted to the agent:
        // the supervisor lets the first through and closes the listener,
        // after which this returns ENOSYS.
        program.extend([
            jump(JEQ, nr(libc::SYS_execveat), 0, 6),
            op(LD, argument(0)),
            jump(JEQ, binary as u32, 0, 3),
            op(LD, argument(4)),
            jump(JEQ, libc::AT_EMPTY_PATH as u32, 0, 1),
            op(RET, USER_NOTIF),
            op(RET, KILL),
        ]);
        // sendmsg: only on the exec-once socket, for the hook to send the
        // listener. The seal below then kills it outright, so the agent
        // never has it (review round 4, M1: a pinned number is not an
        // object; with a socket for a channel, the agent could dup2 it
        // there and pass descriptors).
        program.extend([
            jump(JEQ, nr(libc::SYS_sendmsg), 0, 4),
            op(LD, argument(0)),
            jump(JEQ, exec_once as u32, 0, 1),
            op(RET, ALLOW),
            op(RET, KILL),
        ]);
        // seccomp: only to stack a plain filter (SET_MODE_FILTER, no flags:
        // no second listener, no TSYNC), for the hook to install the seal.
        // The seal kills seccomp too.
        program.extend([
            jump(JEQ, nr(libc::SYS_seccomp), 0, 6),
            op(LD, argument(0)),
            jump(JEQ, libc::SECCOMP_SET_MODE_FILTER, 0, 3),
            op(LD, argument(1)),
            jump(JEQ, 0, 0, 1),
            op(RET, ALLOW),
            op(RET, KILL),
        ]);
        // clone3: its flags are in memory, out of the filter's sight.
        // ENOSYS makes libc fall back to clone, whose flags are checked.
        program.extend([jump(JEQ, nr(libc::SYS_clone3), 0, 1), op(RET, ENOSYS)]);
        // ioctl: never TIOCSTI or TIOCLINUX (S-I2).
        program.extend([
            jump(JEQ, nr(libc::SYS_ioctl), 0, 5),
            op(LD, argument(1)),
            jump(JEQ, libc::TIOCSTI as u32, 2, 0),
            jump(JEQ, libc::TIOCLINUX as u32, 1, 0),
            op(RET, ALLOW),
            op(RET, KILL),
        ]);
        // prctl: thread names only.
        program.extend([
            jump(JEQ, nr(libc::SYS_prctl), 0, 5),
            op(LD, argument(0)),
            jump(JEQ, libc::PR_SET_NAME as u32, 2, 0),
            jump(JEQ, libc::PR_GET_NAME as u32, 1, 0),
            op(RET, KILL),
            op(RET, ALLOW),
        ]);
        // prlimit64: the calling process only (pid 0), never a foreign pid.
        program.extend([
            jump(JEQ, nr(libc::SYS_prlimit64), 0, 4),
            op(LD, argument(0)),
            jump(JEQ, 0, 1, 0),
            op(RET, KILL),
            op(RET, ALLOW),
        ]);
        for call in allowed() {
            program.extend([jump(JEQ, nr(call), 0, 1), op(RET, ALLOW)]);
        }
        for call in refused() {
            program.extend([jump(JEQ, nr(call), 0, 1), op(RET, EACCES)]);
        }
        program.push(op(RET, KILL));
        if program.len() > libc::BPF_MAXINSNS as usize {
            return Err(ConfinementError::Unsupported("seccomp program too long"));
        }
        Ok(program)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// A tiny interpreter for the subset of classic BPF the program
        /// uses, so the decision for any (nr, args) can be read directly.
        fn run(program: &[libc::sock_filter], arch: u32, nr: u32, args: [u32; 6]) -> u32 {
            let word = |offset: u32| match offset {
                0 => nr,
                4 => arch,
                o if o >= 16 && (o - 16) % 8 == 0 => args[((o - 16) / 8) as usize],
                o => panic!("unexpected load at {o}"),
            };
            let (mut pc, mut a) = (0usize, 0u32);
            loop {
                let i = program[pc];
                match i.code {
                    LD => a = word(i.k),
                    JEQ => pc += if a == i.k { i.jt } else { i.jf } as usize,
                    JSET => pc += if a & i.k != 0 { i.jt } else { i.jf } as usize,
                    #[cfg(target_arch = "x86_64")]
                    JGE => pc += if a >= i.k { i.jt } else { i.jf } as usize,
                    RET => return i.k,
                    other => panic!("unexpected opcode {other}"),
                }
                pc += 1;
            }
        }

        fn arch() -> u32 {
            if cfg!(target_arch = "x86_64") {
                AUDIT_ARCH_X86_64
            } else {
                AUDIT_ARCH_AARCH64
            }
        }

        #[test]
        fn the_seal_leaves_the_agent_no_sendmsg_and_no_seccomp() {
            // Stacked filters: the strictest answer wins. The first program
            // lets the hook send its listener and stack the seal; the seal
            // then takes both away for good (review round 4, M1).
            let first = program(BINARY, SOCKET).unwrap();
            let seal = seal().unwrap();
            let socket = SOCKET as u32;
            let mode = libc::SECCOMP_SET_MODE_FILTER;
            for (nr, args, before) in [
                (libc::SYS_sendmsg, [socket, 0, 0, 0, 0, 0], ALLOW),
                (libc::SYS_seccomp, [mode, 0, 0, 0, 0, 0], ALLOW),
            ] {
                assert_eq!(run(&first, arch(), nr as u32, args), before);
                assert_eq!(run(&seal, arch(), nr as u32, args), KILL);
            }
            // A listener or TSYNC is never allowed, even before the seal.
            let listener = libc::SECCOMP_FILTER_FLAG_NEW_LISTENER as u32;
            assert_eq!(
                run(
                    &first,
                    arch(),
                    libc::SYS_seccomp as u32,
                    [mode, listener, 0, 0, 0, 0]
                ),
                KILL
            );
            // Everything else, the seal leaves to the first program.
            assert_eq!(run(&seal, arch(), libc::SYS_read as u32, [0; 6]), ALLOW);
            assert_eq!(run(&seal, arch() ^ 1, libc::SYS_read as u32, [0; 6]), KILL);
        }

        /// A descriptor number for the program to pin execveat to.
        const BINARY: RawFd = 7;
        /// And one for the exec-once socket.
        const SOCKET: RawFd = 9;

        #[test]
        fn the_program_decides_what_the_spec_says() {
            let program = program(BINARY, SOCKET).unwrap();
            let decide = |nr: libc::c_long, args: [u32; 6]| run(&program, arch(), nr as u32, args);
            let none = [0; 6];
            assert_eq!(decide(libc::SYS_read, none), ALLOW);
            // The one exec: the prepared descriptor, by AT_EMPTY_PATH.
            let empty = libc::AT_EMPTY_PATH as u32;
            let binary = BINARY as u32;
            assert_eq!(decide(libc::SYS_execve, none), KILL);
            assert_eq!(
                decide(libc::SYS_execveat, [binary, 0, 0, 0, empty, 0]),
                USER_NOTIF
            );
            let socket = SOCKET as u32;
            assert_eq!(decide(libc::SYS_sendmsg, [socket, 0, 0, 0, 0, 0]), ALLOW);
            assert_eq!(decide(libc::SYS_sendmsg, [socket + 1, 0, 0, 0, 0, 0]), KILL);
            assert_eq!(
                decide(libc::SYS_execveat, [binary + 1, 0, 0, 0, empty, 0]),
                KILL
            );
            assert_eq!(decide(libc::SYS_execveat, [binary, 0, 0, 0, 0, 0]), KILL);
            for denied in [
                libc::SYS_socket,
                libc::SYS_socketpair,
                libc::SYS_io_uring_setup,
                libc::SYS_ptrace,
                libc::SYS_kill,
                libc::SYS_tgkill,
                libc::SYS_bpf,
                libc::SYS_mount,
                libc::SYS_unshare,
                libc::SYS_setns,
                libc::SYS_pidfd_open,
                libc::SYS_pidfd_getfd,
                libc::SYS_process_vm_readv,
                libc::SYS_userfaultfd,
                libc::SYS_perf_event_open,
                libc::SYS_keyctl,
                libc::SYS_shmget,
                libc::SYS_mq_open,
            ] {
                assert_eq!(decide(denied, none), KILL, "syscall {denied}");
            }
            let thread = libc::CLONE_THREAD as u32 | libc::CLONE_VM as u32;
            assert_eq!(decide(libc::SYS_clone, [thread, 0, 0, 0, 0, 0]), ALLOW);
            assert_eq!(
                decide(libc::SYS_clone, [libc::SIGCHLD as u32, 0, 0, 0, 0, 0]),
                KILL
            );
            assert_eq!(decide(libc::SYS_clone3, none), ENOSYS);
            assert_eq!(
                decide(libc::SYS_ioctl, [0, libc::TIOCSTI as u32, 0, 0, 0, 0]),
                KILL
            );
            assert_eq!(
                decide(libc::SYS_ioctl, [0, libc::TIOCLINUX as u32, 0, 0, 0, 0]),
                KILL
            );
            assert_eq!(
                decide(libc::SYS_ioctl, [0, libc::FIONBIO as u32, 0, 0, 0, 0]),
                ALLOW
            );
            assert_eq!(
                decide(libc::SYS_prctl, [libc::PR_SET_NAME as u32, 0, 0, 0, 0, 0]),
                ALLOW
            );
            assert_eq!(
                decide(
                    libc::SYS_prctl,
                    [libc::PR_SET_DUMPABLE as u32, 0, 0, 0, 0, 0]
                ),
                KILL
            );
            assert_eq!(decide(libc::SYS_prlimit64, none), ALLOW);
            assert_eq!(decide(libc::SYS_readlinkat, none), EACCES);
            assert_eq!(decide(libc::SYS_prlimit64, [4242, 0, 0, 0, 0, 0]), KILL);
            // Another architecture's numbers never reach the allowlist.
            assert_eq!(
                run(&program, 0x4000_0003, libc::SYS_read as u32, none),
                KILL
            );
            #[cfg(target_arch = "x86_64")]
            assert_eq!(
                decide(libc::SYS_read | X32_SYSCALL_BIT as libc::c_long, none),
                KILL
            );
        }
    }
}

pub(crate) mod elf {
    //! Just enough ELF to find a dynamically linked executable's
    //! interpreter (`PT_INTERP`), which the kernel execs on its behalf and
    //! Landlock therefore has to allow.

    use super::*;
    use std::io::Read;

    const PT_INTERP: u32 = 3;

    pub(crate) fn interpreter(
        executable: &std::fs::File,
    ) -> Result<Option<PathBuf>, ConfinementError> {
        let bad = |why: &str| ConfinementError::Executable(why.to_string());
        let mut bytes = Vec::new();
        // Read through a duplicate of the descriptor. The offset it moves is
        // shared, and irrelevant: the exec does not read through it.
        executable
            .try_clone()
            .map_err(|_| bad("cannot read"))?
            .take(1 << 20)
            .read_to_end(&mut bytes)
            .map_err(|_| bad("cannot read"))?;
        // ELF64, little-endian: the only kind x86_64 and aarch64 run.
        if bytes.len() < 64 || &bytes[..4] != b"\x7fELF" || bytes[4] != 2 || bytes[5] != 1 {
            return Err(bad("not a 64-bit little-endian ELF file"));
        }
        // Every offset and size below comes from the file, which the agent's
        // author wrote. All arithmetic on them is checked: an overflow is a
        // refused launch, never a panic in the supervisor.
        let truncated = || bad("truncated or malformed program headers");
        let slice = |at: usize, len: usize| {
            at.checked_add(len)
                .and_then(|end| bytes.get(at..end))
                .ok_or_else(truncated)
        };
        let u16_at = |at: usize| slice(at, 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
        let u32_at = |at: usize| slice(at, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let u64_at = |at: usize| {
            slice(at, 8).and_then(|b| {
                usize::try_from(u64::from_le_bytes(b.try_into().unwrap())).map_err(|_| truncated())
            })
        };
        let (phoff, phentsize, phnum) = (u64_at(0x20)?, u16_at(0x36)?, u16_at(0x38)?);
        for index in 0..phnum {
            let header = index
                .checked_mul(phentsize)
                .and_then(|step| phoff.checked_add(step))
                .ok_or_else(truncated)?;
            if u32_at(header)? != PT_INTERP {
                continue;
            }
            let offset = u64_at(header.checked_add(8).ok_or_else(truncated)?)?;
            let size = u64_at(header.checked_add(32).ok_or_else(truncated)?)?;
            let name = slice(offset, size)?;
            // The name ends at its first NUL; an empty one, or one that
            // keeps going without a terminator, is malformed.
            let end = name.iter().position(|b| *b == 0).ok_or_else(truncated)?;
            if end == 0 {
                return Err(truncated());
            }
            return Ok(Some(PathBuf::from(std::ffi::OsStr::from_bytes(
                &name[..end],
            ))));
        }
        Ok(None)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_dynamic_executable_names_its_interpreter() {
            let own = std::env::current_exe().unwrap();
            let interpreter = interpreter(&std::fs::File::open(&own).unwrap())
                .unwrap()
                .expect("test binaries are dynamic");
            assert!(interpreter.is_absolute());
            assert!(interpreter.exists(), "{}", interpreter.display());
        }

        /// A minimal ELF64 header with the given program header table.
        fn elf_with(phoff: u64, phentsize: u16, phnum: u16, tail: &[u8]) -> Vec<u8> {
            let mut bytes = vec![0u8; 64];
            bytes[..6].copy_from_slice(b"\x7fELF\x02\x01");
            bytes[0x20..0x28].copy_from_slice(&phoff.to_le_bytes());
            bytes[0x36..0x38].copy_from_slice(&phentsize.to_le_bytes());
            bytes[0x38..0x3a].copy_from_slice(&phnum.to_le_bytes());
            bytes.extend_from_slice(tail);
            bytes
        }

        fn parse(bytes: &[u8]) -> Result<Option<PathBuf>, ConfinementError> {
            let path = std::env::temp_dir().join(format!(
                "elf-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::write(&path, bytes).unwrap();
            let parsed = interpreter(&std::fs::File::open(&path).unwrap());
            std::fs::remove_file(&path).unwrap();
            parsed
        }

        /// A PT_INTERP header at offset 64 naming `name` at offset 120.
        fn interp_header(name: &[u8], size: u64) -> Vec<u8> {
            let mut header = vec![0u8; 56];
            header[..4].copy_from_slice(&PT_INTERP.to_le_bytes());
            header[8..16].copy_from_slice(&120u64.to_le_bytes());
            header[32..40].copy_from_slice(&size.to_le_bytes());
            header.extend_from_slice(name);
            header
        }

        #[test]
        fn hostile_headers_are_refused_not_panicked_on() {
            // Review M1: offsets near usize::MAX overflowed and panicked.
            for (phoff, phentsize, phnum) in [
                (u64::MAX, 56, 1),
                (u64::MAX - 3, 56, 1),
                (64, u16::MAX, u16::MAX),
                (1 << 40, 56, 1),
            ] {
                assert!(parse(&elf_with(phoff, phentsize, phnum, &[])).is_err());
            }
            for size in [u64::MAX, u64::MAX - 100, 1 << 40] {
                let bytes = elf_with(64, 56, 1, &interp_header(b"/lib/ld.so\0", size));
                assert!(parse(&bytes).is_err(), "size {size}");
            }
        }

        #[test]
        fn the_interpreter_name_ends_at_its_first_nul() {
            let bytes = elf_with(64, 56, 1, &interp_header(b"/lib/ld.so\0\0\0", 13));
            assert_eq!(parse(&bytes).unwrap(), Some(PathBuf::from("/lib/ld.so")));
            for bad in [&b"\0/lib/ld.so"[..], b"/lib/ld.so"] {
                let bytes = elf_with(64, 56, 1, &interp_header(bad, bad.len() as u64));
                assert!(parse(&bytes).is_err(), "{bad:?}");
            }
        }

        #[test]
        fn a_static_executable_has_no_interpreter() {
            assert_eq!(parse(&elf_with(64, 56, 0, &[])).unwrap(), None);
        }

        #[test]
        fn a_non_elf_file_is_refused() {
            let script = std::env::temp_dir().join(format!("not-elf-{}", std::process::id()));
            std::fs::write(&script, b"#!/bin/sh\n").unwrap();
            assert!(interpreter(&std::fs::File::open(&script).unwrap()).is_err());
            let _ = std::fs::remove_file(&script);
        }
    }
}
