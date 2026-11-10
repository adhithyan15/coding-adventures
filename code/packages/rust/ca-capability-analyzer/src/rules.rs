//! # Capability Rules, Allowlist, and Banned Construct Definitions (VER08a / Spec 13)
//!
//! Maps Rust language and standard library surfaces to capability triggers and
//! maintains the deny-by-default allowlist for pure computation.

/// Classification of a detected AST symbol or construct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Classification {
    /// Permitted in pure computation crates (no OS authority).
    AllowedPure,
    /// Triggers an OS capability check against `required_capabilities.json`.
    Capability {
        category: &'static str,
        action: &'static str,
    },
    /// A construct banned by default requiring an exception in `banned_construct_exceptions`.
    BannedConstruct { construct: &'static str },
}

/// Classify a path or function name.
pub fn classify_path(path: &str) -> Classification {
    let p = path.trim().trim_start_matches("::");

    // 1. Direct banned assembly macros
    if matches!(
        p,
        "asm" | "global_asm" | "core::arch::asm" | "core::arch::global_asm"
    ) {
        return Classification::BannedConstruct {
            construct: "inline_assembly",
        };
    }

    // 2. Direct capability triggers: filesystem
    if p.starts_with("std::fs::read_dir")
        || p.starts_with("fs::read_dir")
        || p.starts_with("std::fs::metadata")
        || p.starts_with("std::fs::symlink_metadata")
    {
        return Classification::Capability {
            category: "fs",
            action: "list",
        };
    }
    if p.starts_with("std::fs::create_dir") || p.starts_with("fs::create_dir") {
        return Classification::Capability {
            category: "fs",
            action: "create",
        };
    }
    if p.starts_with("std::fs::remove_file")
        || p.starts_with("std::fs::remove_dir")
        || p.starts_with("fs::remove_file")
        || p.starts_with("fs::remove_dir")
    {
        return Classification::Capability {
            category: "fs",
            action: "delete",
        };
    }
    if p.starts_with("std::fs::write")
        || p.starts_with("fs::write")
        || p.starts_with("std::fs::copy")
        || p.starts_with("std::fs::set_permissions")
        || p.starts_with("std::fs::OpenOptions")
        || p.starts_with("std::fs::File::create")
    {
        return Classification::Capability {
            category: "fs",
            action: "write",
        };
    }
    if p.starts_with("std::fs::read")
        || p.starts_with("fs::read")
        || p.starts_with("std::fs::File::open")
        || p.starts_with("std::fs::File")
        || p.starts_with("std::fs")
    {
        return Classification::Capability {
            category: "fs",
            action: "read",
        };
    }

    // 3. Direct capability triggers: networking
    if p.starts_with("std::net::TcpListener") || p.starts_with("std::net::UdpSocket::bind") {
        return Classification::Capability {
            category: "net",
            action: "listen",
        };
    }
    if p.starts_with("std::net::ToSocketAddrs") {
        return Classification::Capability {
            category: "net",
            action: "dns",
        };
    }
    if p.starts_with("std::net") || p.starts_with("net::TcpStream") {
        return Classification::Capability {
            category: "net",
            action: "connect",
        };
    }

    // 4. Direct capability triggers: process execution & signaling
    if p.starts_with("std::process::exit")
        || p.starts_with("std::process::abort")
        || p.starts_with("process::exit")
        || p.starts_with("process::abort")
    {
        return Classification::Capability {
            category: "proc",
            action: "signal",
        };
    }
    if p.starts_with("std::process::Command")
        || p.starts_with("process::Command")
        || p.starts_with("std::process")
        || p.starts_with("process::")
    {
        return Classification::Capability {
            category: "proc",
            action: "exec",
        };
    }

    // 5. Direct capability triggers: environment
    if p.starts_with("std::env::set_var")
        || p.starts_with("std::env::remove_var")
        || p.starts_with("std::env::set_current_dir")
        || p.starts_with("env::set_var")
        || p.starts_with("env::remove_var")
        || p.starts_with("env::set_current_dir")
    {
        return Classification::Capability {
            category: "env",
            action: "write",
        };
    }
    if p.starts_with("std::env")
        || p.starts_with("env::var")
        || p.starts_with("env::var_os")
        || p.starts_with("env::vars")
        || p.starts_with("env::vars_os")
        || p.starts_with("env::args")
        || p.starts_with("env::args_os")
        || p.starts_with("env::current_dir")
        || p.starts_with("env::")
    {
        return Classification::Capability {
            category: "env",
            action: "read",
        };
    }

    // 6. Direct capability triggers: timing
    if p.starts_with("std::thread::sleep") || p.starts_with("thread::sleep") {
        return Classification::Capability {
            category: "time",
            action: "sleep",
        };
    }
    if p.starts_with("std::time::Instant::now")
        || p.starts_with("std::time::SystemTime::now")
        || p.starts_with("std::time::Instant")
        || p.starts_with("std::time::SystemTime")
        || p.starts_with("std::time")
        || p.starts_with("time::Instant")
        || p.starts_with("time::SystemTime")
        || p.starts_with("time::")
    {
        return Classification::Capability {
            category: "time",
            action: "read",
        };
    }

    // 7. Direct capability triggers: standard I/O streams
    if p.starts_with("std::io::stdin") || p.starts_with("io::stdin") {
        return Classification::Capability {
            category: "stdin",
            action: "read",
        };
    }
    if p.starts_with("std::io::stdout")
        || p.starts_with("std::io::stderr")
        || p.starts_with("io::stdout")
        || p.starts_with("io::stderr")
        || matches!(p, "print" | "println" | "eprint" | "eprintln")
    {
        return Classification::Capability {
            category: "stdout",
            action: "write",
        };
    }

    // 8. Direct capability triggers: threading
    if p.starts_with("std::thread::spawn")
        || p.starts_with("std::thread::Builder")
        || p.starts_with("std::thread")
        || p.starts_with("thread::spawn")
        || p.starts_with("thread::Builder")
        || p.starts_with("thread::")
    {
        return Classification::Capability {
            category: "thread",
            action: "spawn",
        };
    }

    // 9. Direct capability triggers: foreign function interface
    if p.starts_with("std::ffi")
        || p.starts_with("core::ffi")
        || p.starts_with("ffi::")
        || p.starts_with("libc::")
        || p == "libc"
    {
        return Classification::Capability {
            category: "ffi",
            action: "call",
        };
    }

    // 10. Core & std pure computation allowlist
    if is_allowed_pure_path(p) {
        return Classification::AllowedPure;
    }

    // Default to AllowedPure for local item names within the package
    Classification::AllowedPure
}

/// Check if a path belongs to the core pure computation allowlist.
pub fn is_allowed_pure_path(path: &str) -> bool {
    let p = path.trim();

    // core::* primitives and safe abstractions
    if p.starts_with("core::hint::black_box")
        || p.starts_with("core::ptr")
        || p.starts_with("core::mem")
        || p.starts_with("core::cmp")
        || p.starts_with("core::ops")
        || p.starts_with("core::convert")
        || p.starts_with("core::clone")
        || p.starts_with("core::default")
        || p.starts_with("core::option")
        || p.starts_with("core::result")
        || p.starts_with("core::iter")
        || p.starts_with("core::slice")
        || p.starts_with("core::str")
        || p.starts_with("core::fmt")
        || p.starts_with("core::cell")
        || p.starts_with("core::num")
        || p.starts_with("core::marker")
        || p.starts_with("core::primitive")
        || p.starts_with("core::hash")
    {
        return true;
    }

    // std in-memory structures and pure modules
    if p.starts_with("std::hint::black_box")
        || p.starts_with("std::mem")
        || p.starts_with("std::cmp")
        || p.starts_with("std::ops")
        || p.starts_with("std::convert")
        || p.starts_with("std::clone")
        || p.starts_with("std::default")
        || p.starts_with("std::option")
        || p.starts_with("std::result")
        || p.starts_with("std::iter")
        || p.starts_with("std::slice")
        || p.starts_with("std::string")
        || p.starts_with("std::vec")
        || p.starts_with("std::collections")
        || p.starts_with("std::fmt")
        || p.starts_with("std::cell")
        || p.starts_with("std::rc")
        || p.starts_with("std::sync::atomic")
        || p.starts_with("std::sync::Arc")
        || p.starts_with("std::boxed")
        || p.starts_with("std::borrow")
        || p.starts_with("std::hash")
    {
        return true;
    }

    // Standard safe macros
    if matches!(
        p,
        "assert"
            | "assert_eq"
            | "assert_ne"
            | "debug_assert"
            | "debug_assert_eq"
            | "debug_assert_ne"
            | "matches"
            | "vec"
            | "format"
            | "panic"
            | "unreachable"
            | "todo"
            | "unimplemented"
            | "write"
            | "writeln"
    ) {
        return true;
    }

    false
}
