//! Test child for `chief-of-staff-macos-sandbox`: does one thing, chosen by
//! its first argument, and reports the outcome on stdout.
//!
//! ```text
//!   hello              print "ok"
//!   argv               its arguments, one per line, argv[0] first
//!   env                the environment's names, one per line
//!   read <path>        open and read; "ok" or "errno=<n>"
//!   write <path>       open an existing file for writing, write; same
//!   connect <port>     TCP connect to 127.0.0.1:<port>; same
//!   fork               fork, the child exits at once; same
//!   exec <path>        execv without forking; "errno=<n>" if refused
//!   signal <pid>       kill(pid, 0); same
//!   pidpath <pid>      proc_pidpath(pid); same
//!   taskport <pid>     task_for_pid(pid); "ok" or "kern=<n>"
//!   descriptors        each open descriptor above 2, one per line
//!   core               RLIMIT_CORE as "<soft> <hard>"
//!   inherited <n>      read descriptor n to its end and print it
//! ```
//!
//! Seatbelt denials come back as errors, not signals, so every mode
//! reports. Each mode also runs unconfined, as the test's control, so each
//! must be harmless when it succeeds.

#[cfg(target_os = "macos")]
fn main() {
    use std::io::{Read, Write};
    let args: Vec<String> = std::env::args().collect();
    let errno = || std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    let report = |result: std::io::Result<()>| match result {
        Ok(()) => println!("ok"),
        Err(error) => println!("errno={}", error.raw_os_error().unwrap_or(0)),
    };
    let number = |index: usize| -> i32 { args[index].parse().unwrap() };
    match args.get(1).map(String::as_str) {
        Some("hello") => println!("ok"),
        Some("argv") => args.iter().for_each(|arg| println!("{arg}")),
        Some("env") => {
            std::env::vars_os().for_each(|(name, _)| println!("{}", name.to_string_lossy()))
        }
        Some("read") => report(std::fs::File::open(&args[2]).and_then(|mut file| {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).map(|_| ())
        })),
        Some("write") => report(
            std::fs::OpenOptions::new()
                .write(true)
                .open(&args[2])
                .and_then(|mut file| file.write_all(b"w")),
        ),
        Some("connect") => {
            let address = std::net::SocketAddr::from(([127, 0, 0, 1], number(2) as u16));
            report(std::net::TcpStream::connect(address).map(|_| ()))
        }
        Some("fork") => {
            // SAFETY: the child calls only _exit.
            match unsafe { libc::fork() } {
                -1 => println!("errno={}", errno()),
                0 => unsafe { libc::_exit(0) },
                child => {
                    let mut status = 0;
                    // SAFETY: reaping our own child.
                    unsafe { libc::waitpid(child, &mut status, 0) };
                    println!("ok")
                }
            }
        }
        Some("exec") => {
            let path = std::ffi::CString::new(args[2].as_str()).unwrap();
            let argv = [path.as_ptr(), std::ptr::null()];
            // SAFETY: NUL-terminated strings and a null-terminated array.
            unsafe { libc::execv(path.as_ptr(), argv.as_ptr()) };
            println!("errno={}", errno())
        }
        Some("signal") => {
            // SAFETY: signal 0 checks permission and delivers nothing.
            if unsafe { libc::kill(number(2), 0) } == 0 {
                println!("ok")
            } else {
                println!("errno={}", errno())
            }
        }
        Some("pidpath") => {
            let mut buffer = [0u8; 4096];
            // SAFETY: writes at most the buffer's length.
            let written =
                unsafe { libc::proc_pidpath(number(2), buffer.as_mut_ptr().cast(), 4096) };
            if written > 0 {
                println!("ok")
            } else {
                println!("errno={}", errno())
            }
        }
        Some("taskport") => {
            let mut task: libc::mach_port_t = 0;
            // SAFETY: task_for_pid writes one port name through the pointer.
            #[allow(deprecated)]
            let status =
                unsafe { libc::task_for_pid(libc::mach_task_self(), number(2), &mut task) };
            if status == 0 {
                println!("ok")
            } else {
                println!("kern={status}")
            }
        }
        Some("descriptors") => {
            for fd in 3..4096 {
                // SAFETY: F_GETFD only reads a descriptor's flags.
                if unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1 {
                    println!("{fd}");
                }
            }
        }
        Some("core") => {
            let mut limit = libc::rlimit {
                rlim_cur: 1,
                rlim_max: 1,
            };
            // SAFETY: getrlimit writes one rlimit through the pointer.
            unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limit) };
            println!("{} {}", limit.rlim_cur, limit.rlim_max)
        }
        Some("inherited") => {
            use std::os::fd::FromRawFd;
            // SAFETY: the test placed this descriptor for the probe, which
            // takes it over.
            let mut file = unsafe { std::fs::File::from_raw_fd(number(2)) };
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => println!("{contents}"),
                Err(error) => println!("errno={}", error.raw_os_error().unwrap_or(0)),
            }
        }
        other => {
            eprintln!("unknown mode {other:?}");
            std::process::exit(2)
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("macos-sandbox-probe: macOS only");
    std::process::exit(2)
}
