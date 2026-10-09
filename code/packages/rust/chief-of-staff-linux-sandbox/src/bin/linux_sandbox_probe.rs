//! Test child for `chief-of-staff-linux-sandbox`: does one thing, chosen by
//! its first argument, and reports the outcome on stdout.
//!
//! ```text
//!   hello              write, start and join a thread, exit 0
//!   read <path>        open and read; "ok" or "errno=<n>"
//!   write <path>       open for writing and write; "ok" or "errno=<n>"
//!   readlink <path>    read a symlink; "ok" or "errno=<n>"
//!   env [names]        the environment's names, its arguments, the values
//!   exec <path> [args]      execve without forking; "errno=<n>" if refused
//!   execveat <path> [args]  the same, by descriptor (AT_EMPTY_PATH)
//!   execveat-on <n> <path> [args]    dup2 the file onto fd n, exec fd n
//!   execveat-path <n> <path> [args]  execveat(n, absolute path)
//!   opendir <path>     list a directory; "ok" or "errno=<n>"
//!   socket | unix      socket(AF_INET) / socket(AF_UNIX)
//!   fork               a new process (clone without CLONE_THREAD)
//!   io_uring | ptrace | kill | tiocsti | mount | bpf | seccomp | sendmsg
//!   descriptors        each open descriptor above 2, as "n:inode", with
//!                      whether it is close-on-exec (it must not be)
//!   undumpable         prctl(PR_SET_DUMPABLE, 0), then report PR_GET_DUMPABLE
//!   dumpable           prctl(PR_SET_DUMPABLE, 1): only ever a kill confined
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
        Some("descriptors") => {
            // fcntl, not /proc/self/fd: Landlock leaves /proc unopenable.
            let mut open = Vec::new();
            for fd in 3..1024 {
                let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
                if flags < 0 {
                    continue;
                }
                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let inode = if unsafe { libc::fstat(fd, &mut stat) } == 0 {
                    stat.st_ino
                } else {
                    0
                };
                let cloexec = flags & libc::FD_CLOEXEC != 0;
                open.push(format!(
                    "{fd}:{inode}{}",
                    if cloexec { ":cloexec" } else { "" }
                ));
            }
            println!("descriptors={}", open.join(","));
        }
        Some("undumpable") => {
            let set = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
            let now = unsafe { libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) };
            println!("set={set} dumpable={now}");
        }
        Some("dumpable") => {
            // Harmless when it succeeds: the probe was dumpable already.
            let set = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 1, 0, 0, 0) };
            println!("set={set}");
        }
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
        Some("env") => {
            let mut names: Vec<String> = std::env::vars().map(|(name, _)| name).collect();
            names.sort();
            println!("names={}", names.join(","));
            println!("args={}", args[1..].join(","));
            for name in &args[1..] {
                println!("{name}={}", std::env::var(name).unwrap_or_default());
            }
        }
        Some("readlink") => report(std::fs::read_link(&args[1]).map(|_| ())),
        Some("opendir") => report(std::fs::read_dir(&args[1]).map(|_| ())),
        Some(mode @ ("execveat-on" | "execveat-path")) => {
            // Exec by the pinned descriptor number: either by putting the
            // file there (dup2), or with an absolute path, which ignores
            // the descriptor argument.
            let pinned: i32 = args[1].parse().unwrap();
            let path = std::ffi::CString::new(args[2].as_str()).unwrap();
            let rest: Vec<std::ffi::CString> = args[3..]
                .iter()
                .map(|arg| std::ffi::CString::new(arg.as_str()).unwrap())
                .collect();
            let mut argv = vec![path.as_ptr()];
            argv.extend(rest.iter().map(|arg| arg.as_ptr()));
            argv.push(std::ptr::null());
            let envp = [std::ptr::null::<libc::c_char>()];
            let target = if mode == "execveat-on" {
                let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY) };
                unsafe { libc::dup2(fd, pinned) };
                c"".as_ptr()
            } else {
                path.as_ptr()
            };
            // SAFETY: NUL-terminated path and argv/envp arrays.
            unsafe {
                libc::syscall(
                    libc::SYS_execveat,
                    pinned,
                    target,
                    argv.as_ptr(),
                    envp.as_ptr(),
                    libc::AT_EMPTY_PATH,
                )
            };
            println!("errno={}", errno());
        }
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
        Some("seccomp") => {
            // Stacking a filter: allowed to the hook, sealed for the agent.
            unsafe {
                libc::syscall(
                    libc::SYS_seccomp,
                    libc::SECCOMP_SET_MODE_FILTER,
                    0,
                    std::ptr::null::<u8>(),
                )
            };
            println!("survived");
        }
        Some("sendmsg") => {
            // Pass /dev/null over stdout. Unconfined, stdout is a pipe and
            // this is ENOTSOCK; confined, sendmsg is sealed.
            let null = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY) };
            let mut byte = [0u8; 1];
            let mut iov = libc::iovec {
                iov_base: byte.as_mut_ptr().cast(),
                iov_len: 1,
            };
            let mut control = [0u64; 4];
            let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
            message.msg_iov = &mut iov;
            message.msg_iovlen = 1;
            message.msg_control = control.as_mut_ptr().cast();
            message.msg_controllen = unsafe { libc::CMSG_SPACE(4) } as _;
            unsafe {
                let header = &mut *libc::CMSG_FIRSTHDR(&message);
                header.cmsg_level = libc::SOL_SOCKET;
                header.cmsg_type = libc::SCM_RIGHTS;
                header.cmsg_len = libc::CMSG_LEN(4) as _;
                std::ptr::write_unaligned(libc::CMSG_DATA(header).cast::<i32>(), null);
                libc::sendmsg(1, &message, libc::MSG_NOSIGNAL);
            }
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
