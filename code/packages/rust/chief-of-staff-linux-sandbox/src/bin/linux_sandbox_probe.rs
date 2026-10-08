//! Test child for `chief-of-staff-linux-sandbox`: does one thing, chosen by
//! its first argument, and reports the outcome on stdout.
//!
//! ```text
//!   hello              write, start and join a thread, exit 0
//!   read <path>        open and read; "ok" or "errno=<n>"
//!   write <path>       open for writing and write; "ok" or "errno=<n>"
//!   readlink <path>    read a symlink; "ok" or "errno=<n>"
//!   exec <path> [args]      execve without forking; "errno=<n>" if refused
//!   execveat <path> [args]  the same, by descriptor (AT_EMPTY_PATH)
//!   socket | unix      socket(AF_INET) / socket(AF_UNIX)
//!   fork               a new process (clone without CLONE_THREAD)
//!   io_uring | ptrace | kill | tiocsti | mount | bpf
//! ```
//!
//! Under the sandbox, each denied syscall class kills the process with
//! SIGSYS before it can print anything, and the test checks the signal.
//!
//! Each mode also runs unconfined, as the test's control, and CI may run it
//! as root. So every mode must be harmless when its syscall *succeeds*:
//! signal 0, a fork whose child exits at once, a mount that cannot resolve.

#[cfg(target_os = "linux")]
fn main() {
    use std::io::{Read, Write};
    let args: Vec<String> = std::env::args().skip(1).collect();
    let errno = || std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    let report = |result: std::io::Result<()>| match result {
        Ok(()) => println!("ok"),
        Err(error) => println!("errno={}", error.raw_os_error().unwrap_or(0)),
    };
    match args.first().map(String::as_str) {
        Some("hello") => {
            println!("hello");
            let joined = std::thread::spawn(|| 2 + 2).join().unwrap();
            println!("thread={joined}");
        }
        Some("read") => report((|| {
            let mut text = String::new();
            std::fs::File::open(&args[1])?.read_to_string(&mut text)?;
            Ok(())
        })()),
        Some("write") => report((|| {
            std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&args[1])?
                .write_all(b"written")
        })()),
        Some("readlink") => report(std::fs::read_link(&args[1]).map(|_| ())),
        Some("execveat") => {
            // Open the file and exec it by descriptor, as the hook does.
            let path = std::ffi::CString::new(args[1].as_str()).unwrap();
            let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
            let rest: Vec<std::ffi::CString> = args[2..]
                .iter()
                .map(|arg| std::ffi::CString::new(arg.as_str()).unwrap())
                .collect();
            let mut argv = vec![path.as_ptr()];
            argv.extend(rest.iter().map(|arg| arg.as_ptr()));
            argv.push(std::ptr::null());
            let envp = [std::ptr::null::<libc::c_char>()];
            // SAFETY: a descriptor (or -1), an empty path, NULL-terminated
            // argv and envp.
            unsafe {
                libc::syscall(
                    libc::SYS_execveat,
                    fd,
                    c"".as_ptr(),
                    argv.as_ptr(),
                    envp.as_ptr(),
                    libc::AT_EMPTY_PATH,
                )
            };
            println!("errno={}", errno());
        }
        Some("exec") => {
            let path = std::ffi::CString::new(args[1].as_str()).unwrap();
            let rest: Vec<std::ffi::CString> = args[2..]
                .iter()
                .map(|arg| std::ffi::CString::new(arg.as_str()).unwrap())
                .collect();
            let mut argv = vec![path.as_ptr()];
            argv.extend(rest.iter().map(|arg| arg.as_ptr()));
            argv.push(std::ptr::null());
            // SAFETY: a NUL-terminated path and a NULL-terminated argv.
            unsafe { libc::execv(path.as_ptr(), argv.as_ptr()) };
            println!("errno={}", errno());
        }
        // SAFETY (each arm below): a raw syscall the sandbox must refuse; the
        // probe exists to be killed by it.
        Some("socket") => {
            unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
            println!("survived");
        }
        Some("unix") => {
            unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
            println!("survived");
        }
        Some("fork") => {
            // The control's child leaves at once, so only the parent reports.
            if unsafe { libc::fork() } == 0 {
                unsafe { libc::_exit(0) };
            }
            println!("survived");
        }
        Some("io_uring") => {
            let mut params = [0u8; 120];
            unsafe { libc::syscall(libc::SYS_io_uring_setup, 1, params.as_mut_ptr()) };
            println!("survived");
        }
        Some("ptrace") => {
            unsafe { libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) };
            println!("survived");
        }
        Some("kill") => {
            unsafe { libc::kill(libc::getppid(), 0) };
            println!("survived");
        }
        Some("tiocsti") => {
            // Never with a terminal on stdin: unconfined, it would type
            // into it. The tests give the probe /dev/null.
            if unsafe { libc::isatty(0) } == 1 {
                eprintln!("tiocsti needs a non-terminal stdin");
                std::process::exit(2);
            }
            let byte = b'x';
            unsafe { libc::ioctl(0, libc::TIOCSTI, &byte) };
            println!("survived");
        }
        Some("mount") => {
            // The unconfined control runs too, and as root (a CI container)
            // a real mount would succeed and change the host. So both the
            // filesystem type and the mount point are ones that cannot
            // exist: the control fails with an errno and survives, and the
            // confined probe is killed before the kernel looks at either.
            unsafe {
                libc::mount(
                    c"none".as_ptr(),
                    c"/nonexistent/linux-sandbox-probe".as_ptr(),
                    c"linux-sandbox-probe-no-such-fs".as_ptr(),
                    0,
                    std::ptr::null(),
                )
            };
            println!("survived");
        }
        Some("bpf") => {
            unsafe { libc::syscall(libc::SYS_bpf, 0, 0, 0) };
            println!("survived");
        }
        _ => {
            eprintln!("unknown probe");
            std::process::exit(2);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
