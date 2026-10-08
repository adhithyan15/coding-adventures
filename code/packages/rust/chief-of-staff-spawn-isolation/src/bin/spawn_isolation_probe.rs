//! Test child for `chief-of-staff-spawn-isolation`: reports what it was
//! handed.
//!
//! ```text
//!   open_above_2=3,9         every descriptor from 3 to 1023 that is open
//!   stderr_is_dev_null=true  whether fd 2 is the /dev/null device
//!   session_leader=true      whether it leads its own session, and so has
//!                            no controlling terminal it did not open
//! ```

#[cfg(unix)]
fn main() {
    use std::os::unix::fs::MetadataExt;

    let open: Vec<String> = (3..1024)
        // SAFETY: F_GETFD only reads a descriptor's flags.
        .filter(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) } != -1)
        .map(|fd| fd.to_string())
        .collect();
    println!("open_above_2={}", open.join(","));

    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `fstat` writes one `stat` for fd 2, or fails.
    let stderr = (unsafe { libc::fstat(2, stat.as_mut_ptr()) } == 0)
        // SAFETY: initialized by the successful call.
        .then(|| unsafe { stat.assume_init() });
    let dev_null = std::fs::metadata("/dev/null").ok();
    let is_dev_null = match (stderr, dev_null) {
        // `dev_t` is u64 on Linux and i32 on macOS, so the cast is needed on
        // some targets and redundant on others.
        #[allow(clippy::unnecessary_cast)]
        (Some(stderr), Some(null)) => {
            stderr.st_rdev as u64 == null.rdev() && stderr.st_mode & libc::S_IFMT == libc::S_IFCHR
        }
        _ => false,
    };
    println!("stderr_is_dev_null={is_dev_null}");
    // SAFETY: `getsid(0)` and `getpid` take no pointers.
    let session_leader = unsafe { libc::getsid(0) == libc::getpid() };
    println!("session_leader={session_leader}");
}

#[cfg(not(unix))]
fn main() {
    println!("open_above_2=unsupported");
}
