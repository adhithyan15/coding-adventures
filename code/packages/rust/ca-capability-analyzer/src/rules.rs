//! # Capability Rules, Allowlist, and Banned Construct Definitions (VER08a §3.3-3.5)
//!
//! This module answers exactly one question: *given a fully resolved path, what
//! authority does naming it grant?*  Resolution of `use` aliases, renames and
//! globs happens in `parser.rs`; by the time a path reaches [`classify_resolved`]
//! it is spelled the way the standard library spells it (`std::fs::read`).
//!
//! ## Design: a total function with a deny-by-default std surface
//!
//! ```text
//!   path ──► Capability{category, action}   OS authority; must be declared (CAP001)
//!        ├─► BannedConstruct{construct}      waivable by manifest exception (CAP002)
//!        ├─► Unanalyzable{reason}            seen but cannot be analysed (CAP003)
//!        └─► AllowedPure                     in-memory computation only
//! ```
//!
//! The old classifier fell through to `AllowedPure` for *any* unknown name, which
//! contradicted the spec's "deny-by-default".  Now a path headed `std`, `core` or
//! `alloc` is `AllowedPure` only if its module is on an explicit pure list.
//! Names that are *not* headed by a standard crate (local items, third-party
//! crates) remain pure by default: a lexical analyzer cannot look inside them.
//! That is documented as gap G-1 in the spec rather than hidden.
//!
//! Matching is by whole path segment.  The old code used `str::starts_with`, so
//! `std::fs::read_dir_x` or `std::process_x` matched by accident.

/// Classification of a detected path, name or construct.
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
    /// Something the analyzer can see but cannot analyse (CAP003, never waivable).
    Unanalyzable { reason: &'static str },
}

const fn cap(category: &'static str, action: &'static str) -> Classification {
    Classification::Capability { category, action }
}

/// Classify a path as written (no alias resolution, not a macro invocation).
pub fn classify_path(path: &str) -> Classification {
    classify_resolved(path, false)
}

/// Standard-library crate roots.
pub fn is_std_root(s: &str) -> bool {
    matches!(s, "std" | "core" | "alloc")
}

/// Heads that mean "local to this crate"; never classified.
pub fn is_local_head(s: &str) -> bool {
    matches!(s, "crate" | "self" | "super" | "Self")
}

/// Single-segment module names treated as `std::<name>` when no `use` explains them.
pub const SHORTHAND_HEADS: &[&str] = &[
    "fs", "net", "process", "env", "thread", "time", "ffi", "io", "os",
];

/// Methods that reach the filesystem without naming `std::fs` (`Path::exists()` ...).
pub const FS_LIST_METHODS: &[&str] = &[
    "read_dir",
    "metadata",
    "symlink_metadata",
    "canonicalize",
    "read_link",
    "exists",
    "try_exists",
    "is_file",
    "is_dir",
    "is_symlink",
];

/// Methods that block with a timeout, i.e. read the clock and sleep, on types
/// that live in the *pure* `std::sync` module: `Receiver::recv_timeout`,
/// `Condvar::wait_timeout{,_ms,_while}` (and the unstable `recv_deadline`).
/// `thread::park_timeout` is `time:sleep` for the same reason; these reached
/// the clock without naming `std::time` or `std::thread` (security review
/// round 4, H6, case `sync-timed-wait`).
pub const TIMED_WAIT_METHODS: &[&str] = &[
    "recv_timeout",
    "recv_deadline",
    "wait_timeout",
    "wait_timeout_ms",
    "wait_timeout_while",
];

/// Methods that read the clock: `SystemTime::elapsed` / `Instant::elapsed`.
/// `std::time::UNIX_EPOCH.elapsed()` reads the current time without naming
/// `SystemTime` or `Instant` (H6, case `unix-epoch-elapsed`).
pub const CLOCK_READ_METHODS: &[&str] = &["elapsed"];

/// Methods that write file metadata through an already open handle
/// (`File::set_times`, `File::set_modified`, `File::set_permissions`).  A
/// handle opened for *reading* is enough for `futimens`/`fchmod`, so a package
/// declaring only `fs:read` could change timestamps and permissions
/// (review case `handle-method-escalation`).
pub const FS_WRITE_METHODS: &[&str] = &["set_times", "set_modified", "set_permissions"];

/// Methods that resolve a host name: `ToSocketAddrs::to_socket_addrs`.  A glob
/// (`use std::net::*;`) brings the trait into scope without naming it, so
/// `("example.com", 80).to_socket_addrs()` looked up a name under a manifest
/// declaring only `net:connect` (security review round 5, R5-3, case
/// `net-glob-to-socket-addrs`).
pub const DNS_METHODS: &[&str] = &["to_socket_addrs"];

/// The verdict for a *method call* `.name(..)` whose receiver type is unknown.
///
/// A lexical analyzer cannot see the receiver's type, so a method that reaches
/// the OS on some std type is flagged on every receiver.  This over-approximates
/// a user method that happens to share the name (the receiver waiver in
/// `parser.rs` removes the one shape it can prove local, VER08a §3.3 L4).
pub fn method_call_rule(name: &str) -> Option<Classification> {
    if FS_LIST_METHODS.contains(&name) {
        Some(cap("fs", "list"))
    } else if FS_WRITE_METHODS.contains(&name) {
        Some(cap("fs", "write"))
    } else if TIMED_WAIT_METHODS.contains(&name) {
        Some(cap("time", "sleep"))
    } else if CLOCK_READ_METHODS.contains(&name) {
        Some(cap("time", "read"))
    } else if DNS_METHODS.contains(&name) {
        Some(cap("net", "dns"))
    } else {
        None
    }
}

/// UFCS forms of the method rules on types whose own name is pure:
/// `…::Path::<fs method>` / `…::PathBuf::<fs method>` reach the filesystem
/// exactly as `p.<fs method>()` does (`fs:list`), and `…::Condvar::wait_timeout`
/// / `…::Receiver::recv_timeout` block exactly as `c.wait_timeout(..)` does
/// (`time:sleep`).
pub fn path_method_rule(segs: &[&str]) -> Option<Classification> {
    let n = segs.len();
    if n < 2 {
        return None;
    }
    let (ty, method) = (segs[n - 2], segs[n - 1]);
    if matches!(ty, "Path" | "PathBuf") && FS_LIST_METHODS.contains(&method) {
        return Some(cap("fs", "list"));
    }
    if matches!(ty, "Condvar" | "Receiver") && TIMED_WAIT_METHODS.contains(&method) {
        return Some(cap("time", "sleep"));
    }
    None
}

/// `core`/`std` intrinsics that read hardware randomness (`_rdrand64_step`, `_rdseed32_step`).
fn is_hw_random_intrinsic(name: &str) -> bool {
    name.starts_with("_rdrand") || name.starts_with("_rdseed")
}

/// Attribute identifiers that create or alter foreign linkage.
pub const FOREIGN_LINKAGE_ATTRS: &[&str] = &[
    "no_mangle",
    "export_name",
    "link_name",
    "link",
    "link_section",
    "link_ordinal",
    "linkage",
    "naked",
];

/// Pure `std`/`core`/`alloc` modules (VER08a §3.3).  `net` and `time` are pure
/// only under `core` (they hold data types there); see [`classify_segments`].
const PURE_MODULES: &[&str] = &[
    "alloc", "any", "arch", "array", "ascii", "borrow", "boxed", "cell", "char", "clone", "cmp",
    "collections", "convert", "default", "error", "f32", "f64", "fmt", "future", "hash", "hint",
    "i8", "i16", "i32", "i64", "i128", "isize", "iter", "marker", "mem", "num", "ops", "option",
    "panic", "path", "pin", "prelude", "primitive", "ptr", "rc", "result", "slice", "str",
    "string", "sync", "task", "u8", "u16", "u32", "u64", "u128", "usize", "vec",
];

/// Pure std macros (compile-time or in-memory).
const PURE_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "matches",
    "vec",
    "format",
    "format_args",
    "panic",
    "unreachable",
    "unimplemented",
    "todo",
    "write",
    "writeln",
    "concat",
    "stringify",
    "line",
    "column",
    "file",
    "module_path",
    "cfg",
    "compile_error",
    "env",
    "option_env",
    "include_str",
    "include_bytes",
    "thread_local",
    "is_x86_feature_detected",
    "is_aarch64_feature_detected",
    "try",
];

/// Macros that write to the process's standard streams.
const STDOUT_MACROS: &[&str] = &["print", "println", "eprint", "eprintln", "dbg"];

/// Classify a fully resolved path.  `is_macro` is true for `path!(...)`.
pub fn classify_resolved(path: &str, is_macro: bool) -> Classification {
    let p = path.trim().trim_start_matches("::");
    let segs: Vec<&str> = p.split("::").filter(|s| !s.is_empty()).collect();
    classify_segments(&segs, is_macro)
}

/// Head segment of a spelling that goes through a type alias whose
/// right-hand side is a `macro_rules!` metavariable (`type P = $t;`): the type
/// is chosen by the macro's caller.  It contains characters no identifier can,
/// so it never collides with a real name.
pub const UNKNOWN_TYPE_SEGMENT: &str = "<type chosen by a macro caller>";

/// Classify an already-split resolved path.
pub fn classify_segments(segs: &[&str], is_macro: bool) -> Classification {
    if segs.is_empty() {
        return Classification::AllowedPure;
    }
    // `P::name` with `type P = $t;`: the type is unknown, so `P::name` gets the
    // verdict a method call `.name(..)` on an unknown receiver gets (H3).  The
    // method is the *last* segment: `P::Target::exists` is `.exists(`
    // (security review round 5, R5-4), not `.Target(`.
    if segs[0] == UNKNOWN_TYPE_SEGMENT {
        return match segs.len() {
            1 => Classification::AllowedPure,
            n => method_call_rule(segs[n - 1]).unwrap_or(Classification::AllowedPure),
        };
    }
    let head = segs[0];
    let last = segs[segs.len() - 1];

    // Inline assembly: bare, or `{core,std}::arch::X`.
    if matches!(last, "asm" | "global_asm" | "naked_asm")
        && (segs.len() == 1 || (segs.len() == 3 && is_std_root(head) && segs[1] == "arch"))
    {
        return Classification::BannedConstruct {
            construct: "inline_assembly",
        };
    }

    // `include!` pulls in code that is never scanned.  The standard library
    // re-exports it from the preludes too (`std::prelude::v1::include!`,
    // `core::prelude::rust_2021::include!`), and `prelude` is a pure module, so
    // matching only `include!` / `std::include!` let the prelude spelling pass
    // (security review round 3, HIGH-4, PoC t3).  Any std/core/alloc path that
    // *ends* in `include` is the macro, whatever its length, and so is the
    // `use` that imports it (`use std::prelude::v1::include as i;`).
    // `include_str!` / `include_bytes!` embed data and stay pure (VER08a L-B).
    if last == "include" && ((is_macro && segs.len() == 1) || is_std_root(head)) {
        return Classification::Unanalyzable {
            reason: "code pulled in by include! is not scanned",
        };
    }

    // Randomness is named by `RandomState` wherever it appears.
    if segs.contains(&"RandomState") {
        return cap("rand", "read");
    }

    // `Path::exists(p)`, `PathBuf::read_dir(p)`, `<Path>::metadata(p)`, `let g = Path::canonicalize;`:
    // the method-call form is caught after a `.`; the path form is caught here,
    // whatever precedes it (`std::path::`, `core::`, nothing).  Over-approximates
    // a user type that happens to be called `Path` (VER08a §3.3).
    if let Some(c) = path_method_rule(segs) {
        return c;
    }

    // Bare stdout macros (also reached via `use std::println`).
    if segs.len() == 1 && STDOUT_MACROS.contains(&head) {
        return cap("stdout", "write");
    }

    if is_std_root(head) {
        return classify_std(segs, is_macro);
    }

    if is_local_head(head) {
        return Classification::AllowedPure;
    }

    // Well-known capability-bearing third-party crate roots (best effort, G-1).
    match head {
        "libc" | "nix" | "winapi" | "windows" | "windows_sys" | "libloading" => {
            return cap("ffi", "call")
        }
        "rand" | "getrandom" | "fastrand" | "oorandom" | "rand_core" | "rand_chacha" => {
            return cap("rand", "read")
        }
        "tokio" | "mio" | "socket2" | "reqwest" | "hyper" | "ureq" | "async_std" | "smol" => {
            return cap("net", "connect")
        }
        _ => {}
    }

    // Unresolved-head shorthand: `fs::read` with no `use std::fs` in sight.
    if segs.len() >= 2 && SHORTHAND_HEADS.contains(&head) {
        let mut full = vec!["std"];
        full.extend_from_slice(segs);
        return classify_std(&full, is_macro);
    }

    Classification::AllowedPure
}

fn classify_std(segs: &[&str], is_macro: bool) -> Classification {
    let head = segs[0];
    if segs.len() == 1 {
        return Classification::AllowedPure;
    }
    let module = segs[1];
    let rest = &segs[2..];

    // `core` keeps pure data in `net` and `time`.
    if head == "core" && matches!(module, "net" | "time") {
        return Classification::AllowedPure;
    }

    match module {
        "fs" => fs_rule(rest),
        "net" => net_rule(rest),
        "process" => match rest.first() {
            Some(&"exit") | Some(&"abort") => cap("proc", "signal"),
            _ => cap("proc", "exec"),
        },
        "env" => {
            if is_macro && rest.is_empty() {
                // `env!("X")` is resolved by the compiler, not at run time.
                Classification::AllowedPure
            } else {
                match rest.first() {
                    Some(&"set_var") | Some(&"remove_var") | Some(&"set_current_dir") => {
                        cap("env", "write")
                    }
                    _ => cap("env", "read"),
                }
            }
        }
        "backtrace" => cap("env", "read"),
        // `std::path::absolute` joins the current directory (env:read); the
        // `Path`/`PathBuf` fs methods are handled by `path_method_rule`.
        "path" if rest == ["absolute"] => cap("env", "read"),
        // Hardware randomness; every other `arch` intrinsic is arithmetic.
        // (`__cpuid` reveals the CPU model: informational, VER08a §3.3.)
        "arch" if rest.iter().any(|s| is_hw_random_intrinsic(s)) => cap("rand", "read"),
        // `park_timeout_ms` is the deprecated `u32` form of `park_timeout`
        // (security review round 5, R5-3, case `park-timeout-ms`).
        "thread" => match rest.first() {
            Some(&"sleep") | Some(&"sleep_ms") | Some(&"park_timeout") | Some(&"park_timeout_ms") => {
                cap("time", "sleep")
            }
            _ => cap("thread", "spawn"),
        },
        // `UNIX_EPOCH` is a `SystemTime`: `UNIX_EPOCH.elapsed()` reads the clock
        // without naming `SystemTime` (H6), so it is no longer on the pure list.
        "time" => match rest.first() {
            Some(&"Duration") | Some(&"TryFromFloatSecsError") => Classification::AllowedPure,
            _ => cap("time", "read"),
        },
        "io" => match rest.first() {
            Some(&"stdin") | Some(&"Stdin") | Some(&"StdinLock") => cap("stdin", "read"),
            Some(&"stdout") | Some(&"stderr") | Some(&"Stdout") | Some(&"Stderr")
            | Some(&"StdoutLock") | Some(&"StderrLock") | Some(&"_print") | Some(&"_eprint") => {
                cap("stdout", "write")
            }
            // `std::io::pipe()` creates an anonymous OS pipe, two raw file
            // descriptors (handles on Windows).  The taxonomy has no IPC
            // category; raw OS handles (`std::os::fd`, ...) are `ffi:call`, so
            // the pipe and its handle types are too.  Not `CAP003`: the
            // construct is seen exactly, and `CAP003` is never waivable
            // (security review round 5, R5-3, case `io-pipe`).
            Some(&"pipe") | Some(&"PipeReader") | Some(&"PipeWriter") => cap("ffi", "call"),
            _ => Classification::AllowedPure,
        },
        "ffi" => cap("ffi", "call"),
        "os" => {
            if rest.contains(&"net") {
                os_net_rule(rest)
            } else if rest.contains(&"process") {
                cap("proc", "exec")
            } else if rest.contains(&"fs") {
                cap("fs", "write")
            } else if rest.contains(&"thread") {
                cap("thread", "spawn")
            } else {
                cap("ffi", "call")
            }
        }
        m if PURE_MODULES.contains(&m) => Classification::AllowedPure,
        m if STDOUT_MACROS.contains(&m) => cap("stdout", "write"),
        m if PURE_MACROS.contains(&m) => Classification::AllowedPure,
        _ => Classification::Unanalyzable {
            reason: "unclassified std/core/alloc path (deny by default)",
        },
    }
}

fn fs_rule(rest: &[&str]) -> Classification {
    match rest.first() {
        None => cap("fs", "read"),
        Some(&name) => match name {
            "read_dir" | "metadata" | "symlink_metadata" | "canonicalize" | "read_link"
            | "exists" | "try_exists" | "ReadDir" | "DirEntry" | "Metadata" => cap("fs", "list"),
            "create_dir" | "create_dir_all" | "DirBuilder" => cap("fs", "create"),
            "remove_file" | "remove_dir" | "remove_dir_all" => cap("fs", "delete"),
            "write" | "copy" | "rename" | "hard_link" | "soft_link" | "set_permissions"
            | "OpenOptions" => cap("fs", "write"),
            "File" => match rest.get(1) {
                // `create_buffered` creates/truncates exactly like `create`
                // (security review round 3, INFO, PoC t7); the metadata setters
                // write through a read-only handle (round 4, MEDIUM).
                Some(&"create") | Some(&"create_new") | Some(&"create_buffered") | Some(&"options") => {
                    cap("fs", "write")
                }
                Some(m) if FS_WRITE_METHODS.contains(m) => cap("fs", "write"),
                _ => cap("fs", "read"),
            },
            _ => cap("fs", "read"),
        },
    }
}

fn net_rule(rest: &[&str]) -> Classification {
    match rest.first() {
        None => cap("net", "connect"),
        Some(&name) => match name {
            "TcpListener" => cap("net", "listen"),
            "UdpSocket" => {
                if rest.get(1) == Some(&"bind") {
                    cap("net", "listen")
                } else {
                    cap("net", "connect")
                }
            }
            "ToSocketAddrs" => cap("net", "dns"),
            "IpAddr" | "Ipv4Addr" | "Ipv6Addr" | "SocketAddr" | "SocketAddrV4"
            | "SocketAddrV6" | "AddrParseError" | "Shutdown" => Classification::AllowedPure,
            _ => cap("net", "connect"),
        },
    }
}

/// `std::os::<platform>::net::…`: Unix-domain sockets.  A listener, and a
/// datagram socket bound to an address, accept traffic exactly as
/// `TcpListener` / `UdpSocket::bind` do (`net:listen`); they also create a
/// socket file (security review round 5, R5-3, case `unix-listener-class`).
/// Everything else there (`UnixStream`, `UnixDatagram::unbound`, the address
/// type) is `net:connect`, as before.
fn os_net_rule(rest: &[&str]) -> Classification {
    let net = rest.iter().position(|s| *s == "net").unwrap_or(0);
    let after = &rest[net + 1..];
    match after.first() {
        Some(&"UnixListener") => cap("net", "listen"),
        Some(&"UnixDatagram") if matches!(after.get(1), Some(&"bind") | Some(&"bind_addr")) => {
            cap("net", "listen")
        }
        _ => cap("net", "connect"),
    }
}

/// Check if a path belongs to the core pure computation allowlist.
pub fn is_allowed_pure_path(path: &str) -> bool {
    matches!(classify_resolved(path, false), Classification::AllowedPure)
}
