//! # Evasion corpus (VER08a §3)
//!
//! Every syntactic route to a capability or banned construct that we could think
//! of, each with an EXACT expected verdict.  A verdict is the sorted set of:
//!
//! * `cat:action`  - a capability detection (CAP001 candidate), e.g. `fs:read`
//! * `!construct`  - a banned construct (CAP002 candidate), e.g. `!unsafe_code`
//! * `?`           - an unanalysable construct (CAP003)
//!
//! An empty verdict means "clean".  Three kinds of case live here:
//!
//! 1. **routes** - the source reaches an authority; the verdict must name it.
//!    A miss is a false negative (a security bug).
//! 2. **decoys**  - the text only *looks* like a route (strings, comments,
//!    lifetimes, lookalike names); the verdict must be empty.
//! 3. **documented over-approximations** - known false positives that the spec
//!    accepts (marked `FP:` in the name), pinned so they cannot change silently.

use coding_adventures_ca_capability_analyzer::{scan_source, scan_sources};

fn verdict_of(res: &coding_adventures_ca_capability_analyzer::ScanResult) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for d in &res.detections {
        v.push(format!("{}:{}", d.category, d.action));
    }
    for b in &res.banned {
        v.push(format!("!{}", b.construct));
    }
    if !res.unanalyzable.is_empty() {
        v.push("?".to_string());
    }
    v.sort();
    v.dedup();
    v
}

fn verdict(src: &str) -> Vec<String> {
    let res = scan_source("src/case.rs", src).unwrap_or_else(|e| panic!("scan error: {e}\n{src}"));
    verdict_of(&res)
}

type Case = (&'static str, &'static str, &'static [&'static str]);

/// Cases that reach a filesystem authority.
const FS: &[Case] = &[
    ("fs fq read", "fn f(){ let _ = std::fs::read(\"x\"); }", &["fs:read"]),
    ("fs fq read_to_string", "fn f(){ std::fs::read_to_string(\"x\"); }", &["fs:read"]),
    ("fs fq File::open", "fn f(){ std::fs::File::open(\"x\"); }", &["fs:read"]),
    ("fs fq File::create", "fn f(){ std::fs::File::create(\"x\"); }", &["fs:write"]),
    ("fs fq File::create_new", "fn f(){ std::fs::File::create_new(\"x\"); }", &["fs:write"]),
    ("fs fq OpenOptions", "fn f(){ std::fs::OpenOptions::new(); }", &["fs:write"]),
    ("fs fq write", "fn f(){ std::fs::write(\"a\", \"b\"); }", &["fs:write"]),
    ("fs fq copy", "fn f(){ std::fs::copy(\"a\", \"b\"); }", &["fs:write"]),
    ("fs fq rename", "fn f(){ std::fs::rename(\"a\", \"b\"); }", &["fs:write"]),
    ("fs fq hard_link", "fn f(){ std::fs::hard_link(\"a\", \"b\"); }", &["fs:write"]),
    ("fs fq set_permissions", "fn f(){ std::fs::set_permissions(\"a\", p); }", &["fs:write"]),
    ("fs fq create_dir", "fn f(){ std::fs::create_dir(\"a\"); }", &["fs:create"]),
    ("fs fq create_dir_all", "fn f(){ std::fs::create_dir_all(\"a\"); }", &["fs:create"]),
    ("fs fq remove_file", "fn f(){ std::fs::remove_file(\"a\"); }", &["fs:delete"]),
    ("fs fq remove_dir", "fn f(){ std::fs::remove_dir(\"a\"); }", &["fs:delete"]),
    ("fs fq remove_dir_all", "fn f(){ std::fs::remove_dir_all(\"a\"); }", &["fs:delete"]),
    ("fs fq read_dir", "fn f(){ std::fs::read_dir(\".\"); }", &["fs:list"]),
    ("fs fq metadata", "fn f(){ std::fs::metadata(\".\"); }", &["fs:list"]),
    ("fs fq symlink_metadata", "fn f(){ std::fs::symlink_metadata(\".\"); }", &["fs:list"]),
    ("fs fq canonicalize", "fn f(){ std::fs::canonicalize(\".\"); }", &["fs:list"]),
    ("fs fq read_link", "fn f(){ std::fs::read_link(\".\"); }", &["fs:list"]),
    ("fs leading ::", "fn f(){ ::std::fs::read(\"x\"); }", &["fs:read"]),
    ("fs raw ident segment", "fn f(){ std::r#fs::read(\"x\"); }", &["fs:read"]),
    ("fs raw ident root", "fn f(){ r#std::fs::read(\"x\"); }", &["fs:read"]),
    ("fs spaces in path", "fn f(){ std :: fs :: read(\"x\"); }", &["fs:read"]),
    ("fs comments in path", "fn f(){ std /*a*/ :: // b\n fs :: read(\"x\"); }", &["fs:read"]),
    ("fs newline in path", "fn f(){ std\n::\nfs\n::\nread(\"x\"); }", &["fs:read"]),
    ("fs use leaf", "use std::fs::write;", &["fs:write"]),
    ("fs use leaf + call", "use std::fs::write; fn f(){ write(\"a\",\"b\"); }", &["fs:write"]),
    ("fs use rename + call", "use std::fs::read as r; fn f(){ r(\"x\"); }", &["fs:read"]),
    ("fs use module alias", "use std::fs as f; fn g(){ f::remove_file(\"x\"); }", &["fs:delete", "fs:read"]),
    ("fs use group", "use std::{fs, io};", &["fs:read"]),
    ("fs use nested group", "use std::{fs::{File, OpenOptions}, io};", &["fs:read", "fs:write"]),
    ("fs use group self", "use std::fs::{self, File};", &["fs:read"]),
    ("fs use glob then call", "use std::fs::*; fn f(){ read_dir(\".\"); }", &["fs:list", "fs:read"]),
    // A glob over a whole standard crate is CAP003 since round 3 (HIGH-1).
    ("fs use root glob", "use std::*; fn f(){ fs::read(\"x\"); }", &["?", "fs:read"]),
    ("fs use std alias", "use std as s; fn f(){ s::fs::File::open(\"x\"); }", &["fs:read"]),
    ("fs use std self alias", "use std::{self as s}; fn f(){ s::fs::read(\"x\"); }", &["fs:read"]),
    ("fs extern crate alias", "extern crate std as s; fn f(){ s::fs::read(\"x\"); }", &["fs:read"]),
    ("proc alias of module then leaf", "use std::process as p; use p::Command; fn f(){ Command::new(\"x\"); }", &["proc:exec"]),
    ("ffi extern crate libc alias", "extern crate libc as l; fn f(){ l::getpid(); }", &["ffi:call"]),
    ("fs glob through alias", "use std as s; use s::fs::*; fn f(){ read_dir(\".\"); }", &["fs:list", "fs:read"]),
    ("local module glob is ignored", "use crate::m::*; use foo::*; fn f(){ read(1); read_dir(2); }", &[]),
    ("third-party capability glob", "use libc::*; fn f(){ getpid(); }", &["ffi:call"]),
    ("fs alias of alias", "use std as s; use s::fs as q; fn f(){ q::write(\"a\",\"b\"); }", &["fs:read", "fs:write"]),
    ("fs innocent rename", "use std::fs::File as Innocent; fn f(){ Innocent::open(\"x\"); }", &["fs:read"]),
    ("fs type alias", "type F = std::fs::File;", &["fs:read"]),
    ("fs in generic bound", "fn f<T: From<std::fs::File>>(){}", &["fs:read"]),
    ("fs in macro_rules body", "macro_rules! m { () => { std::fs::read(\"x\") } }", &["fs:read"]),
    ("fs in macro args", "fn f(){ foo!(std::fs::read(\"x\")); }", &["fs:read"]),
    ("fs in cfg(test) mod", "#[cfg(test)] mod t { fn f(){ std::fs::read(\"x\"); } }", &["fs:read"]),
    ("fs in cfg(any()) fn", "#[cfg(any())] fn f(){ std::fs::read(\"x\"); }", &["fs:read"]),
    ("fs nested modules", "mod a { mod b { pub fn f(){ std::fs::read(\"x\"); } } }", &["fs:read"]),
    ("fs in closure", "fn f(){ let c = || std::fs::read(\"x\"); c(); }", &["fs:read"]),
    ("fs in static initialiser", "static F: fn() = || { std::fs::read(\"x\"); };", &["fs:read"]),
    ("fs in Drop", "struct X; impl Drop for X { fn drop(&mut self){ std::fs::write(\"a\",\"b\"); } }", &["fs:write"]),
    ("fs in trait default", "trait T { fn f(){ std::fs::read(\"x\"); } }", &["fs:read"]),
    ("fs in const fn body", "const fn f(){ let _ = std::fs::read; }", &["fs:read"]),
    ("fs unknown item defaults to read", "fn f(){ std::fs::some_new_api(); }", &["fs:read"]),
    // Non-ASCII identifiers are CAP003 since round 3 (HIGH-6): no XID/NFC tables.
    ("fs unicode ident under fs", "fn f(){ std::fs::r\u{e9}ad(); }", &["?", "fs:read"]),
    ("fs shorthand head", "fn f(){ fs::read(\"x\"); }", &["fs:read"]),
    ("fs shorthand write", "fn f(){ fs::write(\"a\",\"b\"); }", &["fs:write"]),
    ("fs method exists", "fn f(p: &std::path::Path){ p.exists(); }", &["fs:list"]),
    ("fs method metadata", "fn f(p: &std::path::Path){ p.metadata(); }", &["fs:list"]),
    ("fs method read_dir", "fn f(p: &std::path::Path){ p.read_dir(); }", &["fs:list"]),
    ("fs method is_dir", "fn f(p: &std::path::Path){ p.is_dir(); }", &["fs:list"]),
    ("fs method canonicalize", "fn f(p: &std::path::Path){ p.canonicalize(); }", &["fs:list"]),
    ("fs method try_exists", "fn f(p: &std::path::Path){ p.try_exists(); }", &["fs:list"]),
    ("fs method chained", "fn f(p: std::path::PathBuf){ p.join(\"a\").is_file(); }", &["fs:list"]),
    ("fs CRLF source", "use std::fs;\r\nfn f(){ fs::read(\"x\"); }\r\n", &["fs:read"]),
    ("fs BOM prefix", "\u{feff}fn f(){ std::fs::read(\"x\"); }", &["fs:read"]),
    ("fs tab/formfeed whitespace", "fn f(){\tstd::fs::\u{c}read(\"x\"); }", &["fs:read"]),
];

const NET_PROC_ENV_TIME: &[Case] = &[
    ("net TcpStream", "fn f(){ std::net::TcpStream::connect(\"x\"); }", &["net:connect"]),
    ("net TcpListener", "fn f(){ std::net::TcpListener::bind(\"x\"); }", &["net:listen"]),
    ("net UdpSocket::bind", "fn f(){ std::net::UdpSocket::bind(\"x\"); }", &["net:listen"]),
    ("net UdpSocket other", "fn f(){ std::net::UdpSocket::unbound(); }", &["net:connect"]),
    ("net ToSocketAddrs", "use std::net::ToSocketAddrs;", &["net:dns"]),
    ("net use + bind via alias", "use std::net::UdpSocket; fn f(){ UdpSocket::bind(\"x\"); }", &["net:connect", "net:listen"]),
    ("net glob", "use std::net::*; fn f(){ TcpStream::connect(\"x\"); }", &["net:connect"]),
    ("net shorthand", "fn f(){ net::TcpStream::connect(\"x\"); }", &["net:connect"]),
    ("net os unix", "use std::os::unix::net::UnixStream;", &["net:connect"]),
    ("net tokio", "fn f(){ tokio::net::TcpStream::connect(\"x\"); }", &["net:connect"]),
    ("net reqwest", "fn f(){ reqwest::get(\"x\"); }", &["net:connect"]),
    ("proc Command::new", "fn f(){ std::process::Command::new(\"ls\"); }", &["proc:exec"]),
    ("proc use Command", "use std::process::Command; fn f(){ Command::new(\"ls\"); }", &["proc:exec"]),
    ("proc renamed Command", "use std::process::Command as C; fn f(){ C::new(\"ls\"); }", &["proc:exec"]),
    ("proc group", "use std::{process::Command, io};", &["proc:exec"]),
    ("proc glob", "use std::process::*; fn f(){ Command::new(\"ls\"); }", &["proc:exec"]),
    ("proc module self", "use std::process::{self}; fn f(){ process::Command::new(\"x\"); }", &["proc:exec"]),
    ("proc exit", "fn f(){ std::process::exit(1); }", &["proc:signal"]),
    ("proc abort", "fn f(){ std::process::abort(); }", &["proc:signal"]),
    ("proc renamed exit", "use std::process::exit as quit; fn f(){ quit(1); }", &["proc:signal"]),
    ("proc id", "fn f(){ std::process::id(); }", &["proc:exec"]),
    ("proc ExitCode", "fn f() -> std::process::ExitCode { todo!() }", &["proc:exec"]),
    ("proc CommandExt", "use std::os::unix::process::CommandExt;", &["proc:exec"]),
    ("proc via re-exported std (other file shape)", "pub use std::process as p;", &["proc:exec"]),
    ("proc shorthand", "fn f(){ process::Command::new(\"x\"); }", &["proc:exec"]),
    ("env var", "fn f(){ std::env::var(\"X\"); }", &["env:read"]),
    ("env args", "fn f(){ std::env::args(); }", &["env:read"]),
    ("env vars_os", "fn f(){ std::env::vars_os(); }", &["env:read"]),
    ("env current_dir", "fn f(){ std::env::current_dir(); }", &["env:read"]),
    ("env set_var", "fn f(){ std::env::set_var(\"A\", \"b\"); }", &["env:write"]),
    ("env remove_var", "fn f(){ std::env::remove_var(\"A\"); }", &["env:write"]),
    ("env set_current_dir", "fn f(){ std::env::set_current_dir(\".\"); }", &["env:write"]),
    ("env use + call", "use std::env; fn f(){ env::var(\"X\"); }", &["env:read"]),
    ("env use rename", "use std::env as e; fn f(){ e::set_var(\"A\",\"b\"); }", &["env:read", "env:write"]),
    ("env backtrace", "fn f(){ std::backtrace::Backtrace::capture(); }", &["env:read"]),
    ("env macro is compile-time", "fn f(){ let _ = std::env!(\"X\"); let _ = core::env!(\"Y\"); let _ = env!(\"Z\"); let _ = option_env!(\"W\"); }", &[]),
    ("time Instant::now", "fn f(){ std::time::Instant::now(); }", &["time:read"]),
    ("time SystemTime::now", "fn f(){ std::time::SystemTime::now(); }", &["time:read"]),
    ("time use Instant", "use std::time::Instant;", &["time:read"]),
    ("time module use", "use std::time; fn f(){ time::Instant::now(); }", &["time:read"]),
    ("time Duration is pure", "use std::time::Duration; fn f(){ Duration::from_secs(1); }", &[]),
    // D35: `UNIX_EPOCH` is a `SystemTime`, and `UNIX_EPOCH.elapsed()` reads the clock.
    ("time UNIX_EPOCH reads the clock", "use std::time::UNIX_EPOCH;", &["time:read"]),
    ("time core::time pure", "use core::time::Duration;", &[]),
    ("time sleep", "fn f(){ std::thread::sleep(d); }", &["time:sleep"]),
    ("time sleep via use thread", "use std::thread; fn f(){ thread::sleep(d); }", &["thread:spawn", "time:sleep"]),
    ("time sleep glob", "use std::thread::*; fn f(){ sleep(d); }", &["thread:spawn", "time:sleep"]),
    ("thread spawn", "fn f(){ std::thread::spawn(|| {}); }", &["thread:spawn"]),
    ("thread scope", "fn f(){ std::thread::scope(|_| {}); }", &["thread:spawn"]),
    ("thread Builder", "fn f(){ std::thread::Builder::new(); }", &["thread:spawn"]),
    ("thread use spawn", "use std::thread::spawn; fn f(){ spawn(|| {}); }", &["thread:spawn"]),
    ("thread shorthand", "fn f(){ thread::spawn(|| {}); }", &["thread:spawn"]),
];

const IO_RAND_FFI: &[Case] = &[
    ("stdin fq", "fn f(){ std::io::stdin(); }", &["stdin:read"]),
    ("stdin type", "fn f(_: std::io::Stdin){}", &["stdin:read"]),
    ("stdin use io", "use std::io; fn f(){ io::stdin(); }", &["stdin:read"]),
    ("stdin use rename", "use std::io::stdin as i; fn f(){ i(); }", &["stdin:read"]),
    ("stdin glob", "use std::io::*; fn f(){ stdin(); }", &["stdin:read"]),
    ("stdout fq", "fn f(){ std::io::stdout(); }", &["stdout:write"]),
    ("stderr fq", "fn f(){ std::io::stderr(); }", &["stdout:write"]),
    ("stdout _print", "fn f(){ std::io::_print(format_args!(\"x\")); }", &["stdout:write"]),
    ("stdout use io", "use std::io; fn f(){ io::stdout().flush(); }", &["stdout:write"]),
    ("stdout lock type", "fn f(_: std::io::StdoutLock){}", &["stdout:write"]),
    ("println", "fn f(){ println!(\"x\"); }", &["stdout:write"]),
    ("print", "fn f(){ print!(\"x\"); }", &["stdout:write"]),
    ("eprintln", "fn f(){ eprintln!(\"x\"); }", &["stdout:write"]),
    ("eprint", "fn f(){ eprint!(\"x\"); }", &["stdout:write"]),
    ("dbg", "fn f(){ dbg!(1); }", &["stdout:write"]),
    ("std::println", "fn f(){ std::println!(\"x\"); }", &["stdout:write"]),
    ("std::dbg", "fn f(){ std::dbg!(1); }", &["stdout:write"]),
    ("println renamed", "use std::println as p; fn f(){ p!(\"x\"); }", &["stdout:write"]),
    ("println in macro_rules", "macro_rules! m { () => { println!(\"x\") } }", &["stdout:write"]),
    ("println spaced bang", "fn f(){ println ! (\"x\"); }", &["stdout:write"]),
    ("io pure traits", "use std::io::{self, Read, Write, Cursor, BufReader, ErrorKind}; fn f(_: io::Result<()>){}", &[]),
    ("io Write as _", "use std::io::Write as _;", &[]),
    ("rand RandomState", "fn f(){ std::collections::hash_map::RandomState::new(); }", &["rand:read"]),
    ("rand use RandomState", "use std::hash::RandomState;", &["rand:read"]),
    ("rand crate", "fn f(){ rand::random::<u8>(); }", &["rand:read"]),
    ("rand getrandom", "fn f(){ getrandom::getrandom(&mut b); }", &["rand:read"]),
    ("rand fastrand", "fn f(){ fastrand::u8(..); }", &["rand:read"]),
    ("ffi extern C block", "extern \"C\" { fn x(); }", &["ffi:call"]),
    ("ffi unsafe extern block", "unsafe extern \"C\" { fn x(); }", &["!unsafe_code", "ffi:call"]),
    ("ffi extern C fn", "pub extern \"C\" fn f(){}", &["ffi:call"]),
    ("ffi extern C-unwind", "extern \"C-unwind\" fn f(){}", &["ffi:call"]),
    ("ffi extern system", "extern \"system\" fn f(){}", &["ffi:call"]),
    ("ffi extern vectorcall", "extern \"vectorcall\" fn f(){}", &["ffi:call"]),
    ("ffi extern efiapi", "extern \"efiapi\" fn f(){}", &["ffi:call"]),
    ("ffi extern bare fn", "extern fn f(){}", &["ffi:call"]),
    ("ffi extern bare block", "extern { fn x(); }", &["ffi:call"]),
    ("ffi extern escaped abi", "extern \"\\x43\" fn f(){}", &["ffi:call"]),
    ("ffi extern raw abi", "extern r\"C\" fn f(){}", &["ffi:call"]),
    ("ffi fn pointer type", "fn f(_: extern \"C\" fn(i32)){}", &["ffi:call"]),
    ("ffi extern Rust is not ffi", "extern \"Rust\" fn f(){}", &[]),
    ("ffi extern crate not ffi", "extern crate alloc; extern crate core as c;", &[]),
    ("ffi CString", "fn f(){ std::ffi::CString::new(\"x\"); }", &["ffi:call"]),
    ("ffi core::ffi", "fn f(_: core::ffi::c_int){}", &["ffi:call"]),
    ("ffi alloc::ffi", "use alloc::ffi::CString;", &["ffi:call"]),
    ("ffi libc", "fn f(){ libc::getpid(); }", &["ffi:call"]),
    ("ffi os::raw", "use std::os::raw::c_int;", &["ffi:call"]),
    ("ffi os::fd", "use std::os::fd::AsRawFd;", &["ffi:call"]),
    ("ffi libloading", "fn f(){ libloading::Library::new(\"x\"); }", &["ffi:call"]),
    ("ffi nix", "fn f(){ nix::unistd::getpid(); }", &["ffi:call"]),
    ("ffi shorthand ffi::", "fn f(){ ffi::CStr::from_ptr(p); }", &["ffi:call"]),
];

const BANNED: &[Case] = &[
    ("unsafe block", "fn f(){ unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe fn", "unsafe fn f(){}", &["!unsafe_code"]),
    ("unsafe impl", "unsafe impl Send for X {}", &["!unsafe_code"]),
    ("unsafe trait", "unsafe trait T {}", &["!unsafe_code"]),
    ("unsafe pub fn", "pub unsafe fn f(){}", &["!unsafe_code"]),
    ("unsafe in closure", "fn f(){ let c = || unsafe { g() }; c(); }", &["!unsafe_code"]),
    ("unsafe in macro_rules body", "macro_rules! m { () => { unsafe { g() } } }", &["!unsafe_code"]),
    ("unsafe in macro args", "fn f(){ m!(unsafe { g() }); }", &["!unsafe_code"]),
    ("unsafe tt passthrough", "macro_rules! m { ($($t:tt)*) => { $($t)* } } fn f(){ m!(unsafe { g() }); }", &["!unsafe_code"]),
    ("unsafe behind allow on fn", "#[allow(unsafe_code)] fn f(){ unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe behind inner allow", "#![allow(unsafe_code)]\nfn f(){ unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe behind allow on nested mod", "#[allow(unsafe_code)] mod a { mod b { pub fn f(){ unsafe { g() } } } }", &["!unsafe_code"]),
    ("unsafe behind allow on impl method", "struct S; #[allow(unsafe_code)] impl S { fn f(&self){ unsafe { g() } } }", &["!unsafe_code"]),
    ("unsafe behind allow on nested fn", "#[allow(unsafe_code)] fn outer(){ fn inner(){ unsafe { g() } } }", &["!unsafe_code"]),
    ("unsafe behind allow on statement", "fn f(){ #[allow(unsafe_code)] let _ = unsafe { g() }; }", &["!unsafe_code"]),
    ("unsafe behind cfg_attr allow", "#[cfg_attr(all(), allow(unsafe_code))] fn f(){ unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe behind forbid in other scope", "#![forbid(unsafe_code)]\nmod m { #[allow(unsafe_code)] pub fn f(){ unsafe { g() } } }", &["!unsafe_code"]),
    ("unsafe in attribute form", "#[unsafe(no_mangle)] pub fn f(){}", &["!foreign_linkage", "!unsafe_code"]),
    ("unsafe after unicode escape char", "fn f(){ let _ = '\\u{41}'; unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe after quote char", "fn f(){ let _ = '\"'; unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe after escaped quote char", "fn f(){ let _ = '\\''; unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe after byte char", "fn f(){ let _ = b'\\''; unsafe { g() } }", &["!unsafe_code"]),
    ("unsafe after lifetime", "fn f<'a>(x: &'a str) -> &'a str { unsafe { x } }", &["!unsafe_code"]),
    ("unsafe after label", "fn f(){ 'outer: loop { unsafe { break 'outer; } } }", &["!unsafe_code"]),
    ("unsafe adjacent to char (no space)", "fn f(c: char){ if c == 'a'{unsafe{}} }", &["!unsafe_code"]),
    ("unsafe after nested comment", "/* /* */ */ unsafe fn f(){}", &["!unsafe_code"]),
    ("unsafe after raw string with hashes", "const S: &str = r##\"a\"#b\"##; unsafe fn f(){}", &["!unsafe_code"]),
    ("unsafe after escaped-quote string", "const S: &str = \"\\\" x \\\"\"; unsafe fn f(){}", &["!unsafe_code"]),
    ("unsafe after byte string", "const S: &[u8] = b\"a\\\"b\"; unsafe fn f(){}", &["!unsafe_code"]),
    ("unsafe after raw byte string", "const S: &[u8] = br#\"a\"b\"#; unsafe fn f(){}", &["!unsafe_code"]),
    ("unsafe CRLF", "fn f(){\r\n unsafe {}\r\n}\r\n", &["!unsafe_code"]),
    // A non-ASCII identifier is itself CAP003 since round 3 (HIGH-6).
    ("unsafe after non-ascii ident", "fn \u{fc}ber(){ unsafe {} }", &["!unsafe_code", "?"]),
    ("ptr::read in unsafe", "unsafe fn f(p: *const u8) -> u8 { std::ptr::read(p) }", &["!unsafe_code"]),
    ("transmute in unsafe", "fn f(){ unsafe { std::mem::transmute::<u32, f32>(1) }; }", &["!unsafe_code"]),
    ("core ptr write_volatile in unsafe", "fn f(p: *mut u8){ unsafe { core::ptr::write_volatile(p, 0) } }", &["!unsafe_code"]),
    ("transmute import alone is pure", "use std::mem::transmute;", &[]),
    ("ptr::null is pure", "fn f() -> *const u8 { core::ptr::null() }", &[]),
    ("asm bare", "fn f(){ asm!(\"nop\"); }", &["!inline_assembly"]),
    ("asm core::arch", "fn f(){ core::arch::asm!(\"nop\"); }", &["!inline_assembly"]),
    ("asm std::arch", "fn f(){ std::arch::asm!(\"nop\"); }", &["!inline_assembly"]),
    ("asm use", "use core::arch::asm;", &["!inline_assembly"]),
    ("asm renamed", "use core::arch::asm as a; fn f(){ a!(\"nop\"); }", &["!inline_assembly"]),
    ("asm module alias", "use core::arch as arch; fn f(){ arch::asm!(\"nop\"); }", &["!inline_assembly"]),
    ("global_asm", "global_asm!(\".text\");", &["!inline_assembly"]),
    ("core global_asm", "core::arch::global_asm!(\".text\");", &["!inline_assembly"]),
    ("naked_asm", "fn f(){ core::arch::naked_asm!(\"ret\"); }", &["!inline_assembly"]),
    ("arch intrinsics are pure", "use core::arch::x86_64::_mm_pause;", &[]),
    ("attr no_mangle", "#[no_mangle] pub fn f(){}", &["!foreign_linkage"]),
    ("inner attr no_mangle", "#![no_mangle]", &["!foreign_linkage"]),
    ("attr export_name", "#[export_name = \"x\"] pub fn f(){}", &["!foreign_linkage"]),
    ("attr link_name", "extern \"C\" { #[link_name = \"y\"] fn x(); }", &["!foreign_linkage", "ffi:call"]),
    ("attr link", "#[link(name = \"c\")] extern \"C\" {}", &["!foreign_linkage", "ffi:call"]),
    ("attr link_section", "#[link_section = \".x\"] static X: u8 = 0;", &["!foreign_linkage"]),
    ("attr link_ordinal", "#[link_ordinal(1)] fn x(){}", &["!foreign_linkage"]),
    ("attr linkage", "#[linkage = \"weak\"] fn x(){}", &["!foreign_linkage"]),
    ("attr naked", "#[naked] fn x(){}", &["!foreign_linkage"]),
    ("attr cfg_attr no_mangle", "#[cfg_attr(unix, no_mangle)] pub fn f(){}", &["!foreign_linkage"]),
    ("attr cfg_attr nested", "#[cfg_attr(a, cfg_attr(b, export_name = \"x\"))] pub fn f(){}", &["!foreign_linkage"]),
    ("attr spaced", "# [ no_mangle ] pub fn f(){}", &["!foreign_linkage"]),
    ("attr multiline", "#[\n  no_mangle\n]\npub fn f(){}", &["!foreign_linkage"]),
];

const UNANALYZABLE: &[Case] = &[
    ("include!", "include!(\"gen.rs\");", &["?"]),
    ("std::include!", "std::include!(\"gen.rs\");", &["?"]),
    ("core::include!", "core::include!(\"gen.rs\");", &["?"]),
    ("include! with concat/env", "include!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));", &["?"]),
    ("include! in macro_rules", "macro_rules! m { () => { include!(\"x.rs\") } }", &["?"]),
    ("include_str! is data", "const S: &str = include_str!(\"x.txt\");", &[]),
    ("include_bytes! is data", "const S: &[u8] = include_bytes!(\"x.bin\");", &[]),
    ("path attr", "#[path = \"x.rs\"] mod x;", &["?"]),
    ("path attr hidden dir", "#[path = \".hidden/x.rs\"] mod x;", &["?"]),
    ("path attr in cfg_attr", "#[cfg_attr(unix, path = \"x.rs\")] mod x;", &["?"]),
    ("path attr inner", "mod x { #![path = \"y.rs\"] }", &["?"]),
    ("proc_macro attr", "#[proc_macro] pub fn m(t: TokenStream) -> TokenStream { t }", &["?"]),
    ("proc_macro_derive attr", "#[proc_macro_derive(X)] pub fn m(t: TokenStream) -> TokenStream { t }", &["?"]),
    ("proc_macro_attribute attr", "#[proc_macro_attribute] pub fn m(a: TokenStream, t: TokenStream) -> TokenStream { t }", &["?"]),
    ("macro meta after ::", "macro_rules! m { ($a:ident) => { std::$a::read(\"x\") } }", &["?"]),
    ("macro meta before ::", "macro_rules! m { ($p:path) => { $p::read(\"x\") } }", &["?"]),
    ("macro meta mid path", "macro_rules! m { ($a:ident) => { std::fs::$a(\"x\") } }", &["?", "fs:read"]),
    ("macro repetition joined path", "macro_rules! m { ($($s:ident)::*) => { std::$($s)::* } }", &["?"]),
    ("macro repetition then ::", "macro_rules! m { ($($s:ident),*) => { $($s)::*::x } }", &["?"]),
    ("macro nested def", "macro_rules! o { () => { macro_rules! i { ($a:ident) => { std::$a::read() } } } }", &["?"]),
    ("macro spliced attribute name", "macro_rules! m { ($a:meta) => { #[$a] pub fn f(){} } }", &["?"]),
    ("macro spliced inner attribute", "macro_rules! m { ($a:meta) => { #![$a] } }", &["?"]),
    ("macro spliced cfg_attr", "macro_rules! m { ($a:meta) => { #[cfg_attr(unix, $a)] pub fn f(){} } }", &["?"]),
    ("FP: macro attribute forwarding #[$m] is flagged", "macro_rules! m { ($(#[$m:meta])* $n:ident) => { $(#[$m])* #[derive(Debug)] pub struct $n; } }", &["?"]),
    ("macro $crate is fine", "macro_rules! m { () => { $crate::helper() } }", &[]),
    ("macro $crate nested path is fine", "macro_rules! m { () => { $crate::a::b::c() } }", &[]),
    ("macro plain metavars are fine", "macro_rules! m { ($e:expr) => { $e + 1 } }", &[]),
    ("macro metavar as value is fine", "macro_rules! m { ($x:ident) => { let $x = 1; } }", &[]),
    ("unknown std module", "fn f(){ std::rt::lang_start(); }", &["?"]),
    ("unknown std path via use", "use std::intrinsics;", &["?"]),
    ("unknown core module", "fn f(){ core::intrinsics::abort(); }", &["?"]),
    ("unknown alloc module", "use alloc::weird::Thing;", &["?"]),
    ("unknown std macro", "fn f(){ std::mystery!(); }", &["?"]),
    ("unknown std via alias", "use std as s; fn f(){ s::rt::lang_start(); }", &["?"]),
    // Since D34 a spelling never reuses an import it was derived from, so a
    // cycle converges instead of tripping the round cap.  rustc rejects it.
    ("alias 3-cycle converges (invalid Rust anyway)", "use b::x as a; use c::x as b; use a::x as c;", &[]),
    ("alias cycle hiding nothing but std still resolves", "use std::env as e; use b::x as a; use a::y as b; fn f() { e::var(\"X\"); }", &["env:read"]),
    ("alias 2-cycle converges (invalid Rust anyway)", "use a::b as c; use c::d as a;", &[]),
    ("unparseable use", "use std::{fs, ;", &["?", "fs:read"]),
];

const PURE: &[Case] = &[
    ("collections", "use std::collections::{HashMap, HashSet, BTreeMap, VecDeque};", &[]),
    ("fmt", "use core::fmt::{self, Display, Write as _};", &[]),
    ("std fmt", "use std::fmt::Debug;", &[]),
    ("cmp/mem/ptr", "use core::{cmp::min, mem::swap, ptr::null};", &[]),
    ("alloc", "use alloc::vec::Vec; use alloc::string::String; use alloc::boxed::Box;", &[]),
    ("sync", "use std::sync::{Arc, Mutex, RwLock, Once, OnceLock}; use std::sync::mpsc::channel;", &[]),
    ("sync atomic", "use std::sync::atomic::{AtomicUsize, Ordering};", &[]),
    ("path", "use std::path::{Path, PathBuf};", &[]),
    ("cell/rc", "use std::cell::RefCell; use std::rc::Rc;", &[]),
    ("panic", "use std::panic::catch_unwind;", &[]),
    ("any", "use std::any::Any;", &[]),
    ("hint", "use core::hint::black_box;", &[]),
    ("std::alloc Layout", "use std::alloc::Layout;", &[]),
    ("consts", "fn f(){ let _ = std::f64::consts::PI; let _ = std::u32::MAX; }", &[]),
    ("char", "fn f(){ std::char::from_u32(65); }", &[]),
    ("net data types", "use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};", &[]),
    ("core::net", "use core::net::IpAddr;", &[]),
    ("pure macros", "fn f(){ assert!(true); assert_eq!(1, 1); assert_ne!(1, 2); debug_assert!(true); matches!(1, 1); let _ = vec![1]; let _ = format!(\"x\"); panic!(\"x\"); }", &[]),
    ("pure macros 2", "fn f(){ unreachable!(); todo!(); unimplemented!(); let _ = concat!(\"a\", \"b\"); let _ = stringify!(x); let _ = line!(); let _ = file!(); let _ = column!(); let _ = module_path!(); let _ = cfg!(unix); }", &[]),
    ("write!/writeln!", "fn f(w: &mut String){ use core::fmt::Write; write!(w, \"x\").ok(); writeln!(w).ok(); }", &[]),
    ("std qualified pure macros", "fn f(){ let _ = std::vec![1]; let _ = std::format!(\"x\"); std::assert!(true); std::panic!(\"x\"); }", &[]),
    ("thread_local", "thread_local! { static X: u8 = 0; }", &[]),
    ("std::thread_local", "std::thread_local! { static X: u8 = 0; }", &[]),
    ("feature detect", "fn f(){ std::is_x86_feature_detected!(\"avx2\"); }", &[]),
    ("local module crate path", "mod fs { pub fn read(){} } fn f(){ crate::fs::read(); self::fs::read(); }", &[]),
    ("local item named Command", "struct Command; impl Command { fn new() -> Self { Command } }", &[]),
    ("local fn named spawn", "fn spawn(){} fn f(){ spawn(); }", &[]),
    ("local fn named read_dir_x", "fn read_dir_x(){} fn f(){ read_dir_x(); }", &[]),
    ("empty file", "", &[]),
    ("whitespace only", " \n\t\r\n", &[]),
    ("pub use super", "pub use super::*;", &[]),
    ("use crate glob", "use crate::*;", &[]),
    ("use as underscore", "use std::fmt::Write as _; use std::io::Read as _;", &[]),
    ("range then ident", "fn f(){ for _ in 0..exists(1) {} }", &[]),
    ("neq not macro", "fn f(a: i32){ let _ = a!=2; let _ = a != 3; }", &[]),
    ("numbers", "fn f(){ let _ = 1.max(2); let _ = 0x1F; let _ = 1e-5; let _ = 1_000u64; let _ = 1.5f32; }", &[]),
    ("field not method", "fn f(s: S){ let _ = s.exists; let _ = s.metadata; let _ = s.is_file; }", &[]),
    ("method w/ similar name", "fn f(s: S){ s.exists_x(); s.my_metadata(); }", &[]),
    ("raw ident unsafe decl", "fn r#unsafe(){} fn f(){ r#unsafe(); }", &[]),
    ("raw ident others", "fn r#match(){} fn f(){ r#match(); }", &[]),
    // Not a decoy any more: non-ASCII identifiers are CAP003 (round 3, HIGH-6),
    // but `\u{fc}nsafe` must still not read as the keyword `unsafe`.
    ("non-ascii idents", "fn f(){ let \u{fc}nsafe = 1; let \u{43a}\u{43e}\u{434} = 2; }", &["?"]),
    ("lookalike idents", "fn f(){ let unsafely = 1; let is_unsafe = 2; let my_unsafe = 3; let unsafe_ = 4; let _unsafe = 5; }", &[]),
    ("lint attrs mention unsafe_code", "#![deny(unsafe_code)]\n#![forbid(unsafe_code)]\n#![warn(unsafe_op_in_unsafe_fn)]", &[]),
    ("benign attrs", "#[derive(Debug, Clone)] #[repr(C)] #[inline(never)] #[used] #[global_allocator] #[must_use] #![no_std] struct S;", &[]),
    ("attr string mentions no_mangle", "#[doc = \"no_mangle link export_name path\"] fn f(){}", &[]),
    ("cfg path-like", "#[cfg(feature = \"path\")] fn f(){}", &[]),
    ("metavar after dot is fine", "macro_rules! m { ($s:ident, $f:ident) => { $s.$f } }", &[]),
];

const DECOYS: &[Case] = &[
    ("line comment", "// std::fs::read(\"x\"); unsafe { } std::process::Command::new(\"x\")\nfn f(){}", &[]),
    ("doc comment", "/// std::fs::read unsafe { }\n//! std::process\nfn f(){}", &[]),
    ("block comment", "/* std::fs::read(\"x\"); unsafe { } */ fn f(){}", &[]),
    ("nested block comment", "/* /* unsafe */ std::fs::read */ fn f(){}", &[]),
    ("block comment with stars", "/***** unsafe *****/ fn f(){}", &[]),
    ("block comment with slash star", "/* // */ fn f(){}", &[]),
    ("string literal", "const S: &str = \"std::fs::read unsafe { } std::process::Command\";", &[]),
    ("string with escapes", "const S: &str = \"\\\" unsafe \\\\\" ; fn f(){}", &[]),
    ("string multiline", "const S: &str = \"a\nunsafe\nb\";", &[]),
    ("string line continuation", "const S: &str = \"a\\\n unsafe b\";", &[]),
    ("raw string", "const S: &str = r\"std::fs::read unsafe\";", &[]),
    ("raw string hashes", "const S: &str = r#\"unsafe \" std::fs::read\"#;", &[]),
    ("raw string many hashes", "const S: &str = r####\"unsafe \"### std::process\"####;", &[]),
    ("byte string", "const S: &[u8] = b\"unsafe std::fs\";", &[]),
    ("raw byte string", "const S: &[u8] = br#\"unsafe std::fs\"#;", &[]),
    ("c string", "const S: &core::ffi::CStr = c\"unsafe std::fs\";", &["ffi:call"]),
    ("char literals", "fn f(){ let _ = ['a', '\"', '\\'', '\\\\', '\\n', '\\u{1F600}', '\\x41']; }", &[]),
    ("lifetimes", "fn f<'a, 'static_, 'b: 'a>(x: &'a str, y: &'b str) {}", &[]),
    ("label", "fn f(){ 'unsafe_label: loop { break 'unsafe_label; } }", &[]),
    ("format string", "fn f(){ let _ = format!(\"{}\", \"std::fs::read\"); }", &[]),
    ("assert message", "fn f(){ assert!(true, \"unsafe std::process::exit\"); }", &[]),
    ("attribute string", "#[deprecated(note = \"use std::fs::read instead of unsafe\")] fn f(){}", &[]),
    ("comment between attr", "#[allow(dead_code)] /* unsafe */ fn f(){}", &[]),
    ("slash at end", "fn f(){ let _ = 4 / 2; /", &[]),
    ("shebang-like comment", "#!/usr/bin/env run-cargo-script\n//unsafe\nfn f(){}", &[]),
];

const FALSE_POSITIVES: &[Case] = &[
    ("FP: local mod named fs, bare path", "mod fs { pub fn read(){} } fn f(){ fs::read(); }", &["fs:read"]),
    ("FP: local method named exists", "struct S; impl S { fn exists(&self) -> bool { true } } fn f(s: S){ s.exists(); }", &["fs:list"]),
    ("FP: local type named File with std File imported elsewhere", "use std::fs::File; mod m { pub struct X; } fn f(){ let _: File; }", &["fs:read"]),
    ("FP: unsafe inside unused macro_rules", "macro_rules! never_used { () => { unsafe {} } }", &["!unsafe_code"]),
    ("FP: cfg-disabled code still counts", "#[cfg(any())] fn f(){ unsafe {} }", &["!unsafe_code"]),
    ("FP: whole std::ffi module (OsStr, CStr)", "use std::ffi::OsStr;", &["ffi:call"]),
];

/// Security-review regressions (VER08a divergence log D14-D21).  Every case here
/// FAILED before the fix named in its prefix: M1 UFCS Path methods, M2
/// metavariable-chosen include!, L1 type aliases / qualified paths, L2 hardware
/// randomness, L3 `extern "Rust" {}`, L4 receiver resolution.
const SECFIX: &[Case] = &[
    // ---- M1: path-qualified calls to Path/PathBuf filesystem methods are not pure
    ("M1 Path::exists UFCS", "fn f(){ Path::exists(Path::new(\"/etc/passwd\")); }", &["fs:list"]),
    ("M1 std::path::Path::metadata", "fn f(p: &std::path::Path){ std::path::Path::metadata(p); }", &["fs:list"]),
    ("M1 PathBuf::read_dir", "fn f(p: &PathBuf){ PathBuf::read_dir(p); }", &["fs:list"]),
    ("M1 <PathBuf>::is_file", "fn f(p: &PathBuf){ <PathBuf>::is_file(p); }", &["fs:list"]),
    ("M1 fn item Path::canonicalize", "fn f(){ let g = Path::canonicalize; }", &["fs:list"]),
    ("M1 Path::try_exists", "fn f(p: &Path){ Path::try_exists(p); }", &["fs:list"]),
    ("M1 Path::symlink_metadata", "fn f(p: &Path){ Path::symlink_metadata(p); }", &["fs:list"]),
    ("M1 Path::read_link", "fn f(p: &Path){ Path::read_link(p); }", &["fs:list"]),
    ("M1 Path::is_dir", "fn f(p: &Path){ Path::is_dir(p); }", &["fs:list"]),
    ("M1 Path::is_symlink", "fn f(p: &Path){ Path::is_symlink(p); }", &["fs:list"]),
    ("M1 use Path then UFCS", "use std::path::Path; fn f(p: &Path){ Path::exists(p); }", &["fs:list"]),
    ("M1 use Path as alias then UFCS", "use std::path::Path as P; fn f(p: &P){ P::exists(p); }", &["fs:list"]),
    ("M1 core::path is not special", "fn f(p: &Path){ core::path::Path::exists(p); }", &["fs:list"]),
    ("M1 <Path as Foo>::exists", "fn f(p: &Path){ <Path as Foo>::exists(p); }", &["fs:list"]),
    ("M1 std::path::absolute reads cwd", "fn f(){ let _ = std::path::absolute(\"x\"); }", &["env:read"]),
    ("M1 use path::absolute", "use std::path::absolute; fn f(){ let _ = absolute(\"x\"); }", &["env:read"]),
    ("M1 glob path::*", "use std::path::*; fn f(){ let _ = absolute(\"x\"); }", &["env:read"]),
    ("M1 decoy Path::new", "fn f(){ let _ = std::path::Path::new(\"x\"); }", &[]),
    ("M1 decoy PathBuf::from", "fn f(){ let _ = std::path::PathBuf::from(\"x\"); }", &[]),
    ("M1 decoy Path::join / extension", "fn f(p: &Path){ p.join(\"a\").extension(); Path::file_name(p); }", &[]),
    ("M1 decoy turbofish Vec::<u8>::new", "fn f(){ let _ = Vec::<u8>::new(); }", &[]),
    ("M1 decoy <[u8]>::len", "fn f(x: &[u8]){ let _ = <[u8]>::len(x); }", &[]),
    // ---- M2: a macro name passed as an argument is invoked out of sight
    ("M2 $mac!(..) invoked with include", "macro_rules! m { ($mac:ident, $f:expr) => { $mac!($f) } } pub fn f(){ m!(include, \"../tests/evil.rs\"); }", &["?"]),
    ("M2 $mac!(..) definition alone", "macro_rules! m { ($mac:ident) => { $mac!() } }", &["?"]),
    ("M2 bare include as macro argument", "pub fn f(){ m!(include, \"../tests/evil.rs\"); }", &["?"]),
    ("M2 bare include_str as macro argument", "pub fn f(){ m!(include_str, \"x\"); }", &["?"]),
    ("M2 bare include_bytes as macro argument", "pub fn f(){ m!(include_bytes, \"x\"); }", &["?"]),
    ("M2 bare include in bracket macro", "pub fn f(){ m![include]; }", &["?"]),
    ("M2 bare include nested in args", "pub fn f(){ a!(b!(c, include)); }", &["?"]),
    ("M2 decoy include variable outside macro", "fn f(include: bool){ if include { g(include); } }", &[]),
    ("M2 decoy include_str! proper", "fn f(){ let _ = include_str!(\"x.txt\"); }", &[]),
    ("M2 decoy $a != 1 in macro_rules", "macro_rules! m { ($a:expr) => { $a != 1 } }", &[]),
    ("M2 decoy !$x in macro_rules", "macro_rules! m { ($x:expr) => { !$x } }", &[]),
    ("M2 decoy field named include after dot", "fn f(c: C){ assert!(c.include); }", &[]),
    // ---- N1: a type metavariable inside `<..>::item` hides Path UFCS
    ("N1 <$t>::read_dir", "macro_rules! ls { ($t:ty, $p:expr) => { <$t>::read_dir($p) } } pub fn f(){ let _ = ls!(std::path::Path, \"/\"); }", &["?"]),
    ("N1 <$t>::exists", "macro_rules! ls { ($t:ty, $p:expr) => { <$t>::exists($p) } }", &["?"]),
    ("N1 <$t>::try_exists", "macro_rules! ls { ($t:ty, $p:expr) => { <$t>::try_exists($p) } }", &["?"]),
    ("N1 <$t>::read_link", "macro_rules! ls { ($t:ty, $p:expr) => { <$t>::read_link($p) } }", &["?"]),
    ("N1 <$t>::canonicalize", "macro_rules! ls { ($t:ty, $p:expr) => { <$t>::canonicalize($p) } }", &["?"]),
    ("N1 <$t as Foo>::exists", "macro_rules! ls { ($t:ty, $p:expr) => { <$t as Foo>::exists($p) } }", &["?"]),
    ("N1 <Vec<$t>>::new nested", "macro_rules! ls { ($t:ty) => { <Vec<$t>>::new() } }", &["?"]),
    ("N1 decoy <$t>::MAX associated const", "macro_rules! m { ($($ty:ty),*) => { $( impl S for $ty { fn f() -> Self { <$ty>::MAX } } )* } }", &[]),
    ("N1 <$t>::Max not screaming still flagged", "macro_rules! m { ($t:ty) => { <$t>::Max }  }", &["?"]),
    ("N1 decoy <u8>::from in macro", "macro_rules! m { ($x:expr) => { <u8>::from($x) } }", &[]),
    ("N1 decoy $t in <..> not before ::", "macro_rules! m { ($t:ty) => { let _x: Vec<$t> = Vec::new(); } }", &[]),
    ("N1 decoy generic arrow return", "macro_rules! m { ($t:ty) => { fn f() -> Vec<$t> { Vec::new() } } }", &[]),
    // ---- N2: repetition splice before `::` with kleene op / separator
    ("N2 $($t)*::read_dir", "macro_rules! ls { ($($t:tt)*) => { $($t)*::read_dir } } pub fn f(){ ls!(std::path::Path); }", &["?"]),
    ("N2 $($t)+::x", "macro_rules! ls { ($($t:tt)+) => { $($t)+::read_dir } }", &["?"]),
    ("N2 $($t)?::x", "macro_rules! ls { ($($t:tt)?) => { $($t)?::read_dir } }", &["?"]),
    ("N2 $($t),*::x", "macro_rules! ls { ($($t:tt),*) => { $($t),*::read_dir } }", &["?"]),
    ("N2 $($t)::*", "macro_rules! ls { ($($t:tt)::*) => { $($t)::*::read_dir } }", &["?"]),
    ("N2 decoy repetition not before ::", "macro_rules! m { ($($t:expr),*) => { vec![$($t),*] } }", &[]),
    // ---- L1: type aliases and qualified paths do not hide a more powerful member
    ("L1 type alias then create", "type F = std::fs::File; fn f(){ F::create(\"x\"); }", &["fs:read", "fs:write"]),
    ("L1 type alias of imported File", "use std::fs::File; type F = File; fn f(){ F::create(\"x\"); }", &["fs:read", "fs:write"]),
    ("L1 generic type alias", "type F<T> = std::fs::File; fn f(){ F::<u8>::create(\"x\"); }", &["fs:read", "fs:write"]),
    ("L1 alias of alias", "use std::fs::File; type A = File; type B = A; fn f(){ B::create(\"x\"); }", &["fs:read", "fs:write"]),
    ("L1 <File>::create", "use std::fs::File; fn f(){ <File>::create(\"x\"); }", &["fs:read", "fs:write"]),
    ("L1 <std::fs::File>::create", "fn f(){ <std::fs::File>::create(\"x\"); }", &["fs:read", "fs:write"]),
    ("L1 <File as T>::create", "use std::fs::File; fn f(){ <File as Foo>::create(\"x\"); }", &["fs:read", "fs:write"]),
    ("L1 type alias UdpSocket::bind", "use std::net::UdpSocket; type U = UdpSocket; fn f(){ U::bind(\"x\"); }", &["net:connect", "net:listen"]),
    ("L1 <UdpSocket>::bind", "use std::net::UdpSocket; fn f(){ <UdpSocket>::bind(\"x\"); }", &["net:connect", "net:listen"]),
    ("L1 decoy type alias of integer", "type Id = u32; fn f(){ let _ = Id::from(1u8); }", &[]),
    ("L1 decoy assoc type without value", "trait T { type Item; type Out: Clone; }", &[]),
    ("L1 decoy alias of Vec", "type V = Vec<u8>; fn f(){ let _ = V::new(); }", &[]),
    // ---- qualified-path reading must never hide a plainly written path
    ("QP fat arrow before ::std path", "fn f(v: Vec<u8>){ match v.len() { 0 => ::std::fs::read(\"x\"), _ => todo!() }; }", &["fs:read"]),
    ("QP thin arrow before ::std path", "fn f<T: Clone>() -> ::std::fs::File { todo!() }", &["fs:read"]),
    ("QP generic closes then ::std path in comparison", "fn f(a: Vec<u8>, b: u8){ let _ = a.len() as u8 > b; let _ = ::std::fs::read(\"x\"); }", &["fs:read"]),
    ("QP qualified path keeps the plain reading", "use std::fs::*; fn f(){ <u8>::read(\"x\"); }", &["fs:read"]),
    // ---- L2: hardware randomness
    ("L2 _rdrand64_step", "fn f(){ let mut v = 0u64; unsafe { core::arch::x86_64::_rdrand64_step(&mut v); } }", &["!unsafe_code", "rand:read"]),
    ("L2 _rdseed32_step via std", "fn f(){ let mut v = 0u32; unsafe { std::arch::x86_64::_rdseed32_step(&mut v); } }", &["!unsafe_code", "rand:read"]),
    ("L2 glob arch", "use core::arch::x86_64::*; fn f(){ let mut v = 0u16; unsafe { _rdrand16_step(&mut v); } }", &["!unsafe_code", "rand:read"]),
    ("L2 module alias", "use core::arch::x86_64 as a; fn f(){ let mut v = 0u64; unsafe { a::_rdrand64_step(&mut v); } }", &["!unsafe_code", "rand:read"]),
    ("L2 decoy other intrinsic stays pure", "fn f(){ unsafe { let _ = core::arch::x86_64::_mm_add_epi32; } }", &["!unsafe_code"]),
    ("L2 decoy cpuid is informational only", "fn f(){ unsafe { let _ = core::arch::x86_64::__cpuid(0); } }", &["!unsafe_code"]),
    // ---- L3: extern "Rust" blocks
    ("L3 extern Rust block", "extern \"Rust\" { fn f(); }", &["!foreign_linkage", "ffi:call"]),
    ("L3 unsafe extern Rust block", "unsafe extern \"Rust\" { pub fn f(); }", &["!foreign_linkage", "!unsafe_code", "ffi:call"]),
    ("L3 decoy extern Rust fn definition", "pub extern \"Rust\" fn f() {}", &[]),
    ("L3 decoy extern Rust fn pointer type", "type Cb = extern \"Rust\" fn(u8);", &[]),
    // ---- L4: receiver resolution for the `.canonicalize()` heuristic
    (
        "L4 local type's own canonicalize via ctor+unwrap",
        "struct Url; impl Url { pub fn parse(s: &str) -> Result<Url, E> { todo!() } pub fn canonicalize(&self) -> Result<Url, E> { todo!() } } fn t(){ Url::parse(\"x\").unwrap().canonicalize().unwrap(); }",
        &[],
    ),
    (
        "L4 local ctor returning Option<Self> with expect",
        "struct U; impl U { pub fn new() -> Option<Self> { None } pub fn exists(&self) -> bool { true } } fn t() -> bool { U::new().expect(\"m\").exists() }",
        &[],
    ),
    (
        "L4 not resolved: ctor returns a PathBuf",
        "struct Url; impl Url { pub fn parse(s: &str) -> PathBuf { todo!() } pub fn canonicalize(&self) -> u8 { 0 } } fn t(){ Url::parse(\"x\").canonicalize(); }",
        &["fs:list"],
    ),
    (
        "L4 not resolved: local type has no such method of its own",
        "struct Url; impl Url { pub fn parse(s: &str) -> Result<Url, E> { todo!() } } fn t(){ Url::parse(\"x\").unwrap().canonicalize(); }",
        &["fs:list"],
    ),
    (
        "L4 not resolved: receiver is a variable",
        "struct Url; impl Url { pub fn parse(s: &str) -> Url { todo!() } pub fn canonicalize(&self) -> u8 { 0 } } fn t(u: Url){ u.canonicalize(); }",
        &["fs:list"],
    ),
    (
        "L4 not resolved: unknown adapter between ctor and call",
        "struct Url; impl Url { pub fn parse(s: &str) -> Url { todo!() } pub fn canonicalize(&self) -> u8 { 0 } } fn t(){ Url::parse(\"x\").into_inner().canonicalize(); }",
        &["fs:list"],
    ),
    (
        "L4 not resolved: trait impl does not count as inherent",
        "struct Url; trait T { fn canonicalize(&self); } impl T for Url { fn canonicalize(&self) {} } impl Url { pub fn parse(s: &str) -> Url { todo!() } } fn t(){ Url::parse(\"x\").canonicalize(); }",
        &["fs:list"],
    ),
    (
        "L4 not resolved: type name rebound by use",
        "use std::path::PathBuf as Url; fn t(){ Url::new().unwrap().canonicalize(); }",
        &["fs:list"],
    ),
];

fn run(label: &str, cases: &[Case]) -> usize {
    let mut failures = Vec::new();
    for (name, src, expected) in cases {
        let got = verdict(src);
        let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        want.sort();
        if got != want {
            failures.push(format!("[{label}] {name}\n  src: {src:?}\n  want: {want:?}\n  got:  {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{} corpus miss(es):\n{}", failures.len(), failures.join("\n"));
    cases.len()
}

#[test]
fn evasion_corpus_every_case_has_its_exact_verdict() {
    let mut total = 0;
    total += run("fs", FS);
    total += run("net/proc/env/time", NET_PROC_ENV_TIME);
    total += run("io/rand/ffi", IO_RAND_FFI);
    total += run("banned", BANNED);
    total += run("unanalyzable", UNANALYZABLE);
    total += run("pure", PURE);
    total += run("decoys", DECOYS);
    total += run("documented false positives", FALSE_POSITIVES);
    total += run("security-review regressions", SECFIX);
    total += multi_file_cases();
    // The corpus is a contract: shrinking it needs a deliberate edit of this number.
    assert!(total >= 300, "corpus shrank to {total} cases");
}

/// Cases whose route spans several files (package-wide name resolution).
fn multi_file_cases() -> usize {
    struct M(&'static str, &'static [(&'static str, &'static str)], &'static [&'static str]);
    let cases = [
        M(
            "re-exported std alias used from another file",
            &[("src/a.rs", "pub use std as s;"), ("src/b.rs", "fn f(){ crate::a::s::process::Command::new(\"x\"); }")],
            &["proc:exec"],
        ),
        M(
            "re-exported module alias used from another file",
            &[("src/a.rs", "pub use std::fs as storage;"), ("src/b.rs", "fn f(){ crate::a::storage::write(\"a\",\"b\"); }")],
            &["fs:read", "fs:write"],
        ),
        M(
            "std alias then glob in another file",
            &[("src/a.rs", "pub use std::process::*;"), ("src/b.rs", "use crate::a::*; fn f(){ Command::new(\"x\"); }")],
            &["proc:exec"],
        ),
        M(
            "renamed std root in one file, path in another",
            &[("src/a.rs", "extern crate std as hidden;"), ("src/b.rs", "fn f(){ hidden::env::var(\"X\"); }")],
            &["env:read"],
        ),
        M(
            "clean files stay clean together",
            &[("src/a.rs", "pub fn a() -> u8 { 1 }"), ("src/b.rs", "use crate::a::a; pub fn b() -> u8 { a() }")],
            &[],
        ),
        M(
            "unsafe in second file",
            &[("src/a.rs", "pub fn a() {}"), ("src/b.rs", "pub fn b() { unsafe {} }")],
            &["!unsafe_code"],
        ),
        M(
            "module and fn with the same name (pub use sha256::sha256) converges",
            &[("src/lib.rs", "pub use sha256::{sha256, sha256_hex}; use crate::sha256::sha256_hex as h;")],
            &[],
        ),
    ];
    for M(name, files, expected) in &cases {
        let res = scan_sources(files).unwrap_or_else(|e| panic!("{name}: {e}"));
        let got = verdict_of(&res);
        let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        want.sort();
        assert_eq!(got, want, "multi-file case '{name}'");
    }
    cases.len()
}

#[test]
fn unterminated_constructs_are_scan_errors_not_passes() {
    for (name, src) in [
        ("block comment", "/* unsafe"),
        ("nested block comment", "/* /* */ unsafe"),
        ("string", "const S: &str = \"unsafe"),
        ("raw string", "const S: &str = r#\"unsafe\""),
        ("raw string no hashes", "const S: &str = r\"unsafe"),
        ("byte string", "const S: &[u8] = b\"unsafe"),
    ] {
        assert!(scan_source("src/x.rs", src).is_err(), "{name} must fail closed");
    }
}

#[test]
fn positions_are_one_based_lines_zero_based_columns() {
    let res = scan_source("src/p.rs", "fn f(){}\n    let x = std::fs::read(\"a\");\n").unwrap();
    let d = &res.detections[0];
    assert_eq!((d.line, d.col), (2, 12));
    assert_eq!(d.file, "src/p.rs");
    assert_eq!(d.evidence, "std::fs::read");
}

#[test]
fn evidence_names_the_resolution_when_an_alias_was_involved() {
    let res = scan_source("src/p.rs", "use std::fs::read as r; fn f(){ r(\"a\"); }").unwrap();
    assert!(res.detections.iter().any(|d| d.evidence == "r => std::fs::read"));
}

#[test]
fn every_capability_category_is_reachable_by_some_route() {
    // Spec 13 has 11 categories; `ui` has no Rust std surface (documented), the other ten must be reachable.
    let mut seen = std::collections::BTreeSet::new();
    for group in [FS, NET_PROC_ENV_TIME, IO_RAND_FFI] {
        for (_, src, _) in group {
            for d in &scan_source("x.rs", src).unwrap().detections {
                seen.insert(d.category);
            }
        }
    }
    for cat in ["fs", "net", "proc", "env", "ffi", "time", "stdin", "stdout", "rand", "thread"] {
        assert!(seen.contains(cat), "no corpus route reaches category {cat}");
    }
}
