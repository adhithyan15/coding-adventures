//! The Seatbelt shim's body (D18S step 7): run in a fresh process image,
//! install the profile, prove it is in force, and become the agent.
//!
//! ```text
//!   1. take the instructions out of the environment, and check
//!      what is left against the closed set               (S-I4a)
//!   2. move the inherited descriptors to 3, 4, ...
//!   3. RLIMIT_CORE = 0, soft and hard                     (no core dumps)
//!   4. the agent's and each grant's path still name the checked inodes
//!   5. sandbox_init(profile)                              (S-P3 on failure)
//!   6. the launch probes                                  (S-P4)
//!   7. refuse any open descriptor but 0-2 and the placed  (S-I4d)
//!   8. the agent's and each grant's path still name the checked inodes
//!   9. execve(agent)
//! ```
//!
//! Any failure writes one line to stderr (which is `/dev/null` for an
//! isolated agent; a test may pipe it) and exits 127 without exec'ing.
//! There is no path out of this function that runs the agent unconfined.
//!
//! The shim is our own code in a fresh image, so it may allocate freely:
//! that is the point of it. Seatbelt is per process, not per thread, so
//! the thread count does not matter to the install; the shim starts no
//! threads anyway.

use crate::{check_environment, ShimInstructions, INSTRUCTIONS_VARIABLE};
use std::ffi::{c_char, c_int, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

extern "C" {
    // libsystem_sandbox, re-exported by libSystem. Deprecated since 10.8
    // and still what `sandbox-exec` and browsers use; there is no other
    // way to sandbox a process one launched (recorded in D18S).
    fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *mut c_char) -> c_int;
    fn sandbox_free_error(errorbuf: *mut c_char);
}

/// The status of every refusal: the agent did not run.
pub const REFUSED: i32 = 127;

/// Run the shim. Never returns: it execs the agent or exits [`REFUSED`].
pub fn main() -> ! {
    match run() {
        Ok(never) => match never {},
        Err(why) => {
            eprintln!("chief-of-staff-seatbelt-shim: {why}");
            std::process::exit(REFUSED)
        }
    }
}

enum Never {}

fn run() -> Result<Never, String> {
    // 1. The instructions, removed so the agent never sees them. The shim
    // is single-threaded, so changing its environment races nothing.
    let text = std::env::var(INSTRUCTIONS_VARIABLE).map_err(|_| "no instructions".to_string())?;
    std::env::remove_var(INSTRUCTIONS_VARIABLE);
    let instructions =
        ShimInstructions::decode(&text).ok_or_else(|| "malformed instructions".to_string())?;

    // argv and envp for the agent, built now, while allocating is still
    // clearly fine.
    let mut argv = vec![cstring(instructions.agent.as_bytes())?];
    for argument in std::env::args_os().skip(1) {
        argv.push(cstring(argument.as_bytes())?);
    }
    // The supervisor checked this set; checking it again here costs
    // nothing and covers a command changed after it was checked.
    let variables: Vec<(Vec<u8>, Vec<u8>)> = std::env::vars_os()
        .map(|(name, value)| (name.as_bytes().to_vec(), value.as_bytes().to_vec()))
        .collect();
    check_environment(
        variables
            .iter()
            .map(|(name, value)| (name.as_slice(), value.as_slice())),
    )
    .map_err(|error| error.to_string())?;
    let mut envp = Vec::new();
    for (name, value) in std::env::vars_os() {
        let mut entry = name.as_bytes().to_vec();
        entry.push(b'=');
        entry.extend_from_slice(value.as_bytes());
        envp.push(cstring(&entry)?);
    }
    let agent = cstring(instructions.agent.as_bytes())?;

    // 2.
    place_inherited(&instructions.inherited)?;

    // 3. No core dump: it would hold whatever the process held.
    let none = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: setrlimit reads one rlimit through the pointer.
    if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &none) } != 0 {
        return Err("cannot set RLIMIT_CORE".into());
    }

    // 4.
    check_inodes(&instructions)?;

    // 5. The install. A failure is the end: nothing below runs unconfined.
    install(&instructions.profile)?;

    // 6. Each probe's outcome without the profile is different from its
    // outcome with it, so a profile that silently allowed it shows.
    probe_read(&instructions.shim)?;
    probe_connect()?;

    // 7. Last, so a descriptor anything above opened is seen too.
    check_open_descriptors(2 + instructions.inherited.len() as i32)?;

    // 8. As late as possible before the exec.
    check_inodes(&instructions)?;

    // 9.
    let mut argv_pointers: Vec<*const c_char> = argv.iter().map(|arg| arg.as_ptr()).collect();
    argv_pointers.push(std::ptr::null());
    let mut envp_pointers: Vec<*const c_char> = envp.iter().map(|var| var.as_ptr()).collect();
    envp_pointers.push(std::ptr::null());
    // SAFETY: every pointer is to a NUL-terminated CString that lives to
    // the end of this function, and both arrays end with a null pointer.
    unsafe {
        libc::execve(
            agent.as_ptr(),
            argv_pointers.as_ptr(),
            envp_pointers.as_ptr(),
        )
    };
    Err(format!(
        "execve failed: {}",
        std::io::Error::last_os_error()
    ))
}

fn cstring(bytes: &[u8]) -> Result<CString, String> {
    CString::new(bytes).map_err(|_| "a NUL inside an argument or variable".to_string())
}

/// Move `inherited[i]` to `3 + i`, and close every original. The moves go
/// through a range above both the originals and the targets, so a source
/// that is also a target is never overwritten before it is copied.
fn place_inherited(inherited: &[i32]) -> Result<(), String> {
    if inherited.is_empty() {
        return Ok(());
    }
    let count = inherited.len() as i32;
    let floor = inherited.iter().copied().max().unwrap_or(0).max(2 + count) + 1;
    let mut staged = Vec::with_capacity(inherited.len());
    for &fd in inherited {
        // SAFETY: fcntl(F_DUPFD_CLOEXEC) duplicates fd at or above floor,
        // or fails with EBADF if fd is not open.
        let copy = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, floor) };
        if copy == -1 {
            return Err(format!("inherited descriptor {fd} is not open"));
        }
        staged.push(copy);
    }
    for &fd in inherited {
        // SAFETY: closing a descriptor this process holds; its copy is kept.
        unsafe { libc::close(fd) };
    }
    for (slot, &copy) in staged.iter().enumerate() {
        let target = 3 + slot as i32;
        // SAFETY: dup2 onto a free slot (every original was closed, and
        // the copies sit above every target); the new descriptor is not
        // close-on-exec, so it reaches the agent.
        if unsafe { libc::dup2(copy, target) } != target {
            return Err(format!("cannot place descriptor {target}"));
        }
        // SAFETY: closing the staged copy.
        unsafe { libc::close(copy) };
    }
    Ok(())
}

/// Every open descriptor is 0, 1, 2 or one of the placed ones (up to
/// `highest`). The kernel's own list, so a descriptor however high is seen.
fn check_open_descriptors(highest: i32) -> Result<(), String> {
    let entry = std::mem::size_of::<libc::proc_fdinfo>();
    let mut buffer: Vec<libc::proc_fdinfo> = Vec::new();
    let mut capacity = 64usize;
    let filled = loop {
        buffer.resize(
            capacity,
            libc::proc_fdinfo {
                proc_fd: 0,
                proc_fdtype: 0,
            },
        );
        let bytes = (capacity * entry) as c_int;
        // SAFETY: proc_pidinfo writes at most `bytes` into the buffer,
        // which holds `capacity` entries, and returns how many it wrote.
        let written = unsafe {
            libc::proc_pidinfo(
                libc::getpid(),
                libc::PROC_PIDLISTFDS,
                0,
                buffer.as_mut_ptr().cast(),
                bytes,
            )
        };
        if written <= 0 {
            return Err("cannot list open descriptors".into());
        }
        // A full buffer may have been cut short: grow it and ask again,
        // up to a bound past which the list is refused, not trusted.
        if (written as usize) < capacity * entry {
            break written as usize / entry;
        }
        if capacity >= 1 << 20 {
            return Err("too many open descriptors to list".into());
        }
        capacity *= 4;
    };
    for info in &buffer[..filled] {
        if info.proc_fd > highest {
            return Err(format!("descriptor {} is open", info.proc_fd));
        }
    }
    Ok(())
}

/// The agent's path, and each grant's, still name the inode the
/// supervisor checked. Seatbelt matches by path, so a file put at a grant's
/// path afterwards would be granted in its place.
fn check_inodes(instructions: &ShimInstructions) -> Result<(), String> {
    let same = |path: &str, dev: u64, ino: u64| {
        std::fs::symlink_metadata(path).is_ok_and(|metadata| {
            metadata.file_type().is_file() && (metadata.dev(), metadata.ino()) == (dev, ino)
        })
    };
    if !same(
        &instructions.agent,
        instructions.agent_dev,
        instructions.agent_ino,
    ) {
        return Err("the agent's path no longer names the verified file".into());
    }
    for (path, dev, ino) in &instructions.grants {
        if !same(path, *dev, *ino) {
            return Err(format!("grant {path} no longer names the checked file"));
        }
    }
    Ok(())
}

fn install(profile: &str) -> Result<(), String> {
    let profile = cstring(profile.as_bytes())?;
    let mut error: *mut c_char = std::ptr::null_mut();
    // SAFETY: the profile is a NUL-terminated string; flags 0 means it is
    // profile text. On failure the library allocates an error string,
    // which is read and then freed with sandbox_free_error.
    let status = unsafe { sandbox_init(profile.as_ptr(), 0, &mut error) };
    if status == 0 {
        return Ok(());
    }
    let message = if error.is_null() {
        "unknown error".to_string()
    } else {
        // SAFETY: a non-null error buffer is a NUL-terminated string.
        let text = unsafe { std::ffi::CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: freeing the buffer sandbox_init allocated, once.
        unsafe { sandbox_free_error(error) };
        text
    };
    Err(format!("sandbox_init failed: {message}"))
}

/// A Seatbelt denial is `EPERM`. `EACCES` is accepted too: the supervisor
/// just read this file and runs as this user, so neither is an ambient
/// outcome.
fn denied(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::EPERM) | Some(libc::EACCES))
}

/// Reading the shim's own image: allowed without the profile (the
/// supervisor hashed it), denied with it, since it is not the agent and
/// not under a system library directory.
fn probe_read(shim: &str) -> Result<(), String> {
    match std::fs::File::open(shim) {
        Err(error) if denied(&error) => Ok(()),
        Err(error) => Err(format!("read probe: unexpected {error}")),
        Ok(_) => Err("read probe: the shim's image is readable".into()),
    }
}

/// A TCP connect to 127.0.0.1:1. Without the profile it is refused by the
/// peer (`ECONNREFUSED`); with it, the sandbox refuses first (`EPERM`).
fn probe_connect() -> Result<(), String> {
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], 1));
    match std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(2)) {
        Err(error) if denied(&error) => Ok(()),
        Err(error) => Err(format!("connect probe: unexpected {error}")),
        Ok(_) => Err("connect probe: the connection was made".into()),
    }
}
