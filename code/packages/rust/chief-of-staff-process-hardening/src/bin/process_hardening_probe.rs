//! Test child for `chief-of-staff-process-hardening`: harden itself, then
//! report what the kernel says, so a test reads the result from outside.
//!
//! ```text
//!   (no args)   suppress, then print the report and the read-back values
//!   untouched   print the read-back values without suppressing (control)
//!   wait        then print "waiting" and block until stdin has a line
//!   child       then exec an untouched copy and print its values as child_*
//! ```

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let harden = args.first().map(String::as_str) != Some("untouched");
    if harden {
        match chief_of_staff_process_hardening::suppress_core_dumps() {
            Ok(protection) => {
                println!("applied={}", protection.applied.join(";"));
                println!("missing={}", protection.missing.join(";"));
            }
            Err(error) => {
                println!("error={error}");
                std::process::exit(1);
            }
        }
    }
    report();
    if args.iter().any(|arg| arg == "wait") {
        // Stay alive, hardened, until the test has looked at /proc.
        println!("waiting");
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
    }
    if args.iter().any(|arg| arg == "child") {
        // Exec an untouched copy of this probe, and relabel its report.
        let me = std::env::current_exe().unwrap();
        let output = std::process::Command::new(me)
            .arg("untouched")
            .output()
            .unwrap();
        for line in String::from_utf8(output.stdout).unwrap().lines() {
            println!("child_{line}");
        }
    }
}

#[cfg(unix)]
fn report() {
    let mut limit = libc::rlimit {
        rlim_cur: 7,
        rlim_max: 7,
    };
    // SAFETY: getrlimit writes one rlimit.
    unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limit) };
    println!("core_limit={}/{}", limit.rlim_cur, limit.rlim_max);
    #[cfg(target_os = "linux")]
    {
        // SAFETY: integer arguments only.
        let dumpable = unsafe { libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) };
        println!("dumpable={dumpable}");
        // A non-dumpable process's /proc entries belong to root, but the
        // process itself can still list its own descriptors: the spawn
        // paths (spawn-isolation, the Linux sandbox's shim) depend on it.
        let own = std::fs::read_dir("/proc/self/fd").map(|entries| entries.count());
        println!("own_fds_listable={}", own.is_ok());
    }
}

#[cfg(not(unix))]
fn report() {}
