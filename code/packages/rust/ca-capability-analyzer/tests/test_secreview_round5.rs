//! # Round-5 security review regressions (VER08a divergence D43 onwards)
//!
//! A third review of the round-3 and round-4 fixes built more small crates
//! that the analyzer passed (exit 0) while `cargo check` compiled a
//! capability call into them.  The cheap ones are fixed and pinned here; the
//! rest are documented as known open gaps in VER08a §8.1.  Each test names
//! the review's case directory, and every group carries decoys that must stay
//! clean so a fix does not flood real packages with false positives.
//!
//! Verdict notation is the one used by `test_evasion_corpus.rs`:
//!
//! * `cat:action` - a capability detection (CAP001 candidate)
//! * `!construct` - a banned construct (CAP002 candidate)
//! * `?`          - an unanalysable construct (CAP003, never waivable)

use coding_adventures_ca_capability_analyzer::{scan_source, ScanResult};

fn verdict_of(res: &ScanResult) -> Vec<String> {
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

/// Run a group of cases and report every mismatch at once.
fn check(label: &str, cases: &[Case]) {
    let mut failures = Vec::new();
    for (name, src, expected) in cases {
        let got = verdict(src);
        let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        want.sort();
        if got != want {
            failures.push(format!("[{label}] {name}\n  src: {src:?}\n  want: {want:?}\n  got:  {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{} case(s) failed:\n{}", failures.len(), failures.join("\n"));
}

// ---------------------------------------------------------------------------
// R5-1: an empty (or any) turbofish between the method name and `(`
// ---------------------------------------------------------------------------
//
// The method heuristic looked only at the token right after the name, so
// `p.exists::<>()` - legal Rust for a method with no generic parameters -
// slipped past it.

#[test]
fn r5_1_turbofish_on_the_method_name_keeps_the_method_heuristic() {
    check(
        "R5-1",
        &[
            (
                "turbofish-exists",
                "pub fn probe() -> bool { std::path::Path::new(\"Cargo.toml\").exists::<>() }",
                &["fs:list"],
            ),
            (
                "turbofish-wait-timeout",
                "use std::sync::{Condvar, Mutex};\nuse std::time::Duration;\npub fn nap() {\n    let m = Mutex::new(());\n    let c = Condvar::new();\n    let g = m.lock().unwrap();\n    let _ = c.wait_timeout::<>(g, Duration::from_millis(10));\n}",
                &["time:sleep"],
            ),
            // A non-empty turbofish needs a method with generic parameters
            // (`p.is_file::<u8>()` is E0107); the timed waits have one.
            (
                "non-empty turbofish",
                "#[allow(deprecated)]\nfn f(c: &std::sync::Condvar, g: std::sync::MutexGuard<'_, u8>) { let _ = c.wait_timeout_ms::<u8>(g, 1); }",
                &["time:sleep"],
            ),
            (
                "nested turbofish",
                "fn f(c: &std::sync::Condvar, g: std::sync::MutexGuard<'_, Vec<Box<u8>>>) { let _ = c.wait_timeout::<Vec<Box<u8>>>(g, std::time::Duration::from_millis(1)); }",
                &["time:sleep"],
            ),
            ("turbofish on a set_times call", "fn f(h: &H) { h.set_times::<>(t); }", &["fs:write"]),
        ],
    );
}

#[test]
fn r5_1_turbofish_decoys_stay_clean() {
    check(
        "R5-1 decoys",
        &[
            ("turbofish on a pure method", "fn f(v: Vec<u8>) -> Vec<u16> { v.into_iter().map(u16::from).collect::<Vec<_>>() }", &[]),
            ("field named exists", "fn f(s: &S) -> bool { s.exists }", &[]),
            ("field compared through a turbofish-looking shape", "fn f(s: &S) -> bool { s.exists < 3 }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// R5-2: a macro invocation in type position is an unknown type
// ---------------------------------------------------------------------------
//
// `ty!()` expands to a type the analyzer never sees.  The readers of a
// qualified self type (`<ty!()>::exists`), an impl self type
// (`impl P for ty!()`) and a type alias right-hand side (`type F = ty!();`)
// stopped at the `!` and took the macro's *name* as the type: `ty::exists`,
// a local, pure path.  Each now yields an unknown type, which gets the
// verdict of a method call `.name(` on an unknown receiver, exactly as a
// metavariable does.

#[test]
fn r5_2_a_type_macro_is_an_unknown_type() {
    check(
        "R5-2",
        &[
            (
                "type-macro-qualified",
                "macro_rules! ty { () => { std::path::Path }; }\npub fn probe(p: &std::path::Path) -> bool { <ty!()>::exists(p) }",
                &["fs:list"],
            ),
            (
                "type-macro-alias",
                "macro_rules! ty { () => { std::path::Path }; }\ntype F = ty!();\npub fn probe(p: &std::path::Path) -> bool { F::exists(p) }",
                &["fs:list"],
            ),
            (
                "type-macro-impl-self",
                "macro_rules! ty { () => { std::path::Path }; }\npub trait P { fn go(&self) -> bool; }\nimpl P for ty!() { fn go(&self) -> bool { Self::exists(self) } }\npub fn run() -> bool { std::path::Path::new(\"Cargo.toml\").go() }",
                &["fs:list"],
            ),
            (
                "type-macro-condvar",
                "macro_rules! cv { () => { std::sync::Condvar }; }\npub fn nap() {\n    let m = std::sync::Mutex::new(());\n    let c = <cv!()>::new();\n    let g = m.lock().unwrap();\n    let _ = <cv!()>::wait_timeout(&c, g, std::time::Duration::from_millis(10));\n}",
                &["time:sleep"],
            ),
            (
                "qualified with a trait",
                "macro_rules! ty { () => { std::path::Path }; }\npub trait Probe { fn exists(&self) -> bool; }\nfn f(p: &std::path::Path) -> bool { <ty!() as Probe>::exists(p) }",
                &["fs:list"],
            ),
            (
                "qualified, macro behind a reference",
                "macro_rules! ty { () => { std::path::Path }; }\nfn f(p: &std::path::Path) -> bool { <&ty!()>::is_file(&p) }",
                &["fs:list"],
            ),
            (
                "qualified, path-qualified macro",
                "fn f(p: &std::path::Path) -> bool { <crate::ty!()>::read_link(p).is_ok() }",
                &["fs:list"],
            ),
            (
                "inherent impl over a macro",
                "macro_rules! ty { () => { std::path::Path }; }\nimpl ty!() { fn go(&self) -> bool { Self::is_symlink(self) } }",
                &["fs:list"],
            ),
            (
                "alias to a macro with brackets",
                "type F = ty![];\nfn f(p: &std::path::Path) -> bool { F::try_exists(p).is_ok() }",
                &["fs:list"],
            ),
            (
                "alias to a path-qualified macro",
                "type F = crate::ty!{};\nfn f(p: &std::path::Path) -> bool { F::canonicalize(p).is_ok() }",
                &["fs:list"],
            ),
            // Found while fixing R5-2: a parenthesised qualified self type.
            (
                "parenthesised qualified self type",
                "#![allow(unused_parens)]\npub fn a(p: &std::path::Path) -> bool { <(std::path::Path)>::exists(p) }",
                &["fs:list"],
            ),
        ],
    );
}

#[test]
fn r5_2_type_macro_decoys_stay_clean() {
    check(
        "R5-2 decoys",
        &[
            ("qualified pure item", "macro_rules! ty { () => { Vec<u8> }; }\nfn f() -> Vec<u8> { <ty!()>::new() }", &[]),
            ("qualified trait item", "fn f() -> u8 { <ty!() as Default>::default() }", &[]),
            ("impl self with a local ctor", "impl Default for ty!() { fn default() -> Self { Self::new() } }", &[]),
            ("alias with a pure item", "type V = ty!();\nfn f() -> V { V::new() }", &[]),
            ("never type alias", "type Never = !;", &[]),
            ("macro in a value position", "fn f() -> usize { vec![1u8].len() }", &[]),
            ("parenthesised pure self type", "#![allow(unused_parens)]\nfn f() -> Vec<u8> { <(Vec<u8>)>::new() }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// R5-3: gaps in the std table
// ---------------------------------------------------------------------------

#[test]
fn r5_3a_park_timeout_ms_is_a_timed_sleep() {
    check(
        "R5-3a",
        &[
            ("park-timeout-ms", "#[allow(deprecated)]\npub fn nap() { std::thread::park_timeout_ms(10) }", &["time:sleep"]),
            ("park-timeout-control", "pub fn nap() { std::thread::park_timeout(std::time::Duration::from_millis(10)) }", &["time:sleep"]),
            ("through a use", "use std::thread::park_timeout_ms as p;\n#[allow(deprecated)]\npub fn nap() { p(10) }", &["time:sleep"]),
            // Decoys: the rest of `std::thread` stays `thread:spawn`.
            ("park without a timeout", "pub fn f() { std::thread::park() }", &["thread:spawn"]),
            ("spawn", "pub fn f() { std::thread::spawn(|| {}); }", &["thread:spawn"]),
        ],
    );
}

#[test]
fn r5_3b_to_socket_addrs_is_a_dns_lookup() {
    check(
        "R5-3b",
        &[
            (
                "net-glob-to-socket-addrs",
                "use std::net::*;\npub fn resolve() -> usize { (\"127.0.0.1\", 80).to_socket_addrs().map(|i| i.count()).unwrap_or(0) }\npub fn _keep(_: Option<IpAddr>) {}",
                // The glob itself names the `std::net` module (`net:connect`,
                // which the review's manifest declares); the lookup is `net:dns`.
                &["net:connect", "net:dns"],
            ),
            ("bare receiver", "fn f(s: &str) { let _ = s.to_socket_addrs(); }", &["net:dns"]),
            ("turbofish", "fn f(s: &str) { let _ = s.to_socket_addrs::<>(); }", &["net:dns"]),
            (
                "UFCS through a glob",
                "use std::net::*;\nfn f() { let _ = ToSocketAddrs::to_socket_addrs(&(\"h\", 80)); }",
                &["net:connect", "net:dns"],
            ),
            // Decoys: the address data types stay pure.
            ("address data type", "fn f(a: std::net::IpAddr) -> bool { a.is_loopback() }", &[]),
            ("field named to_socket_addrs", "fn f(s: &S) -> u8 { s.to_socket_addrs }", &[]),
        ],
    );
}

/// `std::io::pipe()` creates an anonymous OS pipe: two raw file descriptors
/// (handles on Windows).  The taxonomy has no IPC category; VER08a already
/// classifies raw OS handles (`std::os::fd`, `std::os::unix::io`, ...) as
/// `ffi:call`, so the pipe and its two handle types join that row.  `CAP003`
/// would be wrong: the analyzer sees the construct exactly, and `CAP003` is
/// never waivable, so no package could declare a pipe.
#[test]
fn r5_3c_io_pipe_is_a_raw_os_handle() {
    check(
        "R5-3c",
        &[
            ("io-pipe", "pub fn mk() -> bool { std::io::pipe().is_ok() }", &["ffi:call"]),
            ("imported", "use std::io::{pipe, PipeReader, PipeWriter};\npub fn mk() -> Option<(PipeReader, PipeWriter)> { pipe().ok() }", &["ffi:call"]),
            ("reader type only", "pub fn f(_: std::io::PipeReader) {}", &["ffi:call"]),
            ("writer type only", "pub fn f(_: &std::io::PipeWriter) {}", &["ffi:call"]),
            ("shorthand head", "pub fn mk() -> bool { io::pipe().is_ok() }", &["ffi:call"]),
            // Decoys: the rest of `std::io` stays pure.
            ("io traits and cursor", "use std::io::{Cursor, Read, Write};\npub fn f(b: &[u8]) -> usize { let mut v = Vec::new(); Cursor::new(b).read_to_end(&mut v).unwrap_or(0) }", &[]),
            ("io copy between in-memory buffers", "pub fn f() { let _ = std::io::copy(&mut std::io::empty(), &mut std::io::sink()); }", &[]),
            ("local fn named pipe", "fn pipe() -> u8 { 0 }\npub fn f() -> u8 { pipe() }", &[]),
        ],
    );
}

#[test]
fn r5_3d_unix_listeners_are_net_listen() {
    check(
        "R5-3d",
        &[
            (
                "unix-listener-class",
                "pub fn binder() -> fn(&'static str) -> std::io::Result<std::os::unix::net::UnixListener> {\n    std::os::unix::net::UnixListener::bind::<&'static str>\n}",
                &["net:listen"],
            ),
            ("UnixListener through a use", "use std::os::unix::net::UnixListener;\npub fn f() { let _ = UnixListener::bind(\"/tmp/s\"); }", &["net:listen"]),
            ("UnixDatagram::bind", "pub fn f() { let _ = std::os::unix::net::UnixDatagram::bind(\"/tmp/s\"); }", &["net:listen"]),
            (
                "UnixDatagram::bind_addr through a use",
                "use std::os::unix::net::{SocketAddr, UnixDatagram};\npub fn f(a: &SocketAddr) { let _ = UnixDatagram::bind_addr(a); }",
                &["net:connect", "net:listen"],
            ),
            // Decoys: connecting sockets stay `net:connect`.
            ("UnixStream::connect", "pub fn f() { let _ = std::os::unix::net::UnixStream::connect(\"/tmp/s\"); }", &["net:connect"]),
            ("UnixDatagram::unbound", "pub fn f() { let _ = std::os::unix::net::UnixDatagram::unbound(); }", &["net:connect"]),
        ],
    );
}

// ---------------------------------------------------------------------------
// R5-4: associated-type projections and unknown types look at the method
// ---------------------------------------------------------------------------
//
// `<PathBuf as Deref>::Target` is `Path`, so `<PathBuf as Deref>::Target::exists`
// is `Path::exists`; so is `D::Target::exists` for `D: Deref<Target = Path>`,
// and `Self::Target::exists` under a `Deref<Target = Path>` supertrait.  The
// analyzer does not evaluate projections, so the type is unknown, and an
// unknown type gets the method-call verdict of its *last* segment.  The old
// unknown-type checks looked at `segs[1]` (`Target`), not at the method.
//
// Rust accepts the shorthand `X::Assoc` only when `X` is a type parameter or
// `Self` (a concrete type needs the qualified form, E0223), so these three
// shapes are every way to spell a projection at its use:
//
//   <T as Trait>::Assoc::item     qualified, with a trait
//   Self::Assoc::item             Self
//   P::Assoc::item                P a generic parameter declared in the file
//
// They are not every way to *reach* one: a type alias can carry the
// projection to a use that names only the alias (F3 below).

#[test]
fn r5_4_projections_get_the_method_verdict() {
    check(
        "R5-4",
        &[
            (
                "proj-deref-target",
                "pub fn probe(p: &std::path::Path) -> bool {\n    <std::path::PathBuf as std::ops::Deref>::Target::exists(p)\n}",
                &["fs:list"],
            ),
            (
                "proj-generic-bound",
                "use std::ops::Deref;\nuse std::path::{Path, PathBuf};\npub fn probe<D: Deref<Target = Path>>(d: &D) -> bool {\n    D::Target::exists(d)\n}\npub fn run() -> bool { probe(&PathBuf::from(\"Cargo.toml\")) }",
                &["fs:list"],
            ),
            (
                "proj-self-target-trait",
                "use std::ops::Deref;\nuse std::path::{Path, PathBuf};\npub trait Probe: Deref<Target = Path> {\n    fn probe(&self) -> bool { Self::Target::exists(self) }\n}\nimpl Probe for PathBuf {}\npub fn run() -> bool { PathBuf::from(\"Cargo.toml\").probe() }",
                &["fs:list"],
            ),
            (
                "proj-blanket-impl",
                "use std::ops::Deref;\nuse std::path::{Path, PathBuf};\npub trait Probe { fn probe(&self) -> bool; }\nimpl<T: Deref<Target = Path>> Probe for T {\n    fn probe(&self) -> bool { <Self as Deref>::Target::exists(self) }\n}\npub fn run() -> bool { PathBuf::from(\"Cargo.toml\").probe() }",
                &["fs:list"],
            ),
            (
                "proj-condvar-wait",
                "use std::ops::Deref;\nuse std::sync::{Arc, Condvar, Mutex};\nuse std::time::Duration;\npub fn nap() {\n    let m = Mutex::new(());\n    let c = Arc::new(Condvar::new());\n    let g = m.lock().unwrap();\n    let _ = <Arc<Condvar> as Deref>::Target::wait_timeout(&c, g, Duration::from_millis(10));\n}",
                &["time:sleep"],
            ),
            (
                "Self projection in a blanket impl, shorthand",
                "use std::ops::Deref;\npub trait Probe { fn probe(&self) -> bool; }\nimpl<T: Deref<Target = std::path::Path>> Probe for T { fn probe(&self) -> bool { Self::Target::is_dir(self) } }",
                &["fs:list"],
            ),
            (
                "generic projection, qualified without a trait",
                "fn f<D: std::ops::Deref<Target = std::path::Path>>(d: &D) -> bool { <D>::Target::is_file(d) }",
                &["fs:list"],
            ),
            (
                "generic of an impl, nested projection",
                "struct W<T>(T);\nimpl<T: Tr> W<T> { fn f(&self) -> bool { T::A::B::read_link(&self.0).is_ok() } }",
                &["fs:list"],
            ),
            (
                "unknown Self, projection",
                "macro_rules! m { ($t:ty) => { impl Probe for $t { fn probe(&self) -> bool { Self::Target::exists(self) } } } }",
                &["fs:list"],
            ),
            (
                "unknown alias, projection",
                "macro_rules! a { ($t:ty) => { type P = $t; }; }\na!(std::path::PathBuf);\npub fn f(p: &std::path::Path) -> bool { P::Target::try_exists(p).is_ok() }",
                &["fs:list"],
            ),
            (
                "type macro, qualified projection",
                "fn f(p: &std::path::Path) -> bool { <ty!() as std::ops::Deref>::Target::canonicalize(p).is_ok() }",
                &["fs:list"],
            ),
        ],
    );
}

#[test]
fn r5_4_projection_decoys_stay_clean() {
    check(
        "R5-4 decoys",
        &[
            ("projection to a pure item", "fn f<I: Iterator>() -> Option<I::Item> where I::Item: Default { Some(I::Item::default()) }", &[]),
            ("Self projection to a pure item", "trait T: Iterator where Self::Item: Default { fn z() -> Self::Item { Self::Item::default() } }", &[]),
            ("qualified projection to a pure item", "fn f<I: Iterator>(x: u8) -> <I as Iterator>::Item where <I as Iterator>::Item: From<u8> { <I as Iterator>::Item::from(x) }", &[]),
            ("qualified trait method named like nothing listed", "fn f(v: &Vec<u8>) -> usize { <Vec<u8> as AsRef<[u8]>>::as_ref(v).len() }", &[]),
            ("module path ending in a listed name", "mod cleanup { pub fn metadata(_: u8) {} }\nfn f() { cleanup::metadata(1); }", &[]),
            ("generic name used as a value path", "fn f<T: Default>() -> T { T::default() }", &[]),
            ("local type with a listed associated fn", "struct C;\nimpl C { fn exists() -> bool { true } }\nfn f() -> bool { C::exists() }", &[]),
        ],
    );
}

// ---------------------------------------------------------------------------
// Follow-up review of the round-5 fixes
// ---------------------------------------------------------------------------
//
// A focused review of D43-D49 found no regressions, but four shapes the new
// turbofish and projection rules did not reach.  Each test names the
// review's case directory.

// F4: a `>` inside brackets ended the turbofish skip early.
//
// `skip_angle` counts `<` and `>` without regard to brackets, so in
// `c.wait_timeout::<[u8; (2 > 1) as usize]>(g, d)` the comparison `2 > 1`
// closed the turbofish, the token after it was `1`, not `(`, and the call was
// read as a field.  A field takes no generic arguments, so `.name::<` is
// always a method call: the heuristic no longer looks for the `(`.
#[test]
fn f4_a_turbofish_after_a_method_name_is_always_a_call() {
    check(
        "F4",
        &[
            (
                "tf_paren_gt",
                "use std::sync::{Condvar, Mutex};\nuse std::time::Duration;\npub fn nap() -> bool {\n    let m = Mutex::new([0u8; 1]);\n    let c = Condvar::new();\n    let g = m.lock().unwrap();\n    let r = c.wait_timeout::<[u8; (2 > 1) as usize]>(g, Duration::from_millis(1));\n    r.is_ok()\n}",
                &["time:sleep"],
            ),
            (
                "comparison in a braced array length",
                "fn f(c: &std::sync::Condvar, g: std::sync::MutexGuard<'_, [u8; 1]>) { let _ = c.wait_timeout::<[u8; { 2 > 1 } as usize]>(g, std::time::Duration::from_millis(1)); }",
                &["time:sleep"],
            ),
            (
                "shift in an array length",
                "#[allow(deprecated)]\nfn f(c: &std::sync::Condvar, g: std::sync::MutexGuard<'_, [u8; 2]>) { let _ = c.wait_timeout_ms::<[u8; 4 >> 1]>(g, 1); }",
                &["time:sleep"],
            ),
            // Not valid Rust (the turbofish never closes): this only exercises
            // the lexer and the walk at the end of the token stream.
            ("turbofish never closed before the end", "fn f(p: &P) -> bool { p.is_dir::<", &["fs:list"]),
            // Decoys: the rule is about listed method names only.
            ("pure method with a bracketed comparison", "fn f(v: &[u8]) -> Vec<u8> { v.iter().copied().collect::<Vec<[u8; (2 > 1) as usize]>>() }", &[]),
            ("field named exists, compared", "fn f(s: &S) -> bool { s.exists > 1 }", &[]),
        ],
    );
}

// F2: an attribute on a generic parameter hid its name.
//
// `generic_param_names` took an identifier as a parameter only right after
// `<` or `,`.  In `fn probe<#[allow(unused)] D: Deref<Target = Path>>` the
// token before `D` is the `]` of the attribute, so `D` was not a parameter
// and `D::Target::exists(d)` read as a local path.
#[test]
fn f2_an_attribute_does_not_hide_a_generic_parameter() {
    check(
        "F2",
        &[
            (
                "proj_attr_param",
                "use std::ops::Deref;\nuse std::path::{Path, PathBuf};\npub fn probe<#[allow(unused)] D: Deref<Target = Path>>(d: &D) -> bool {\n    D::Target::exists(d)\n}\npub fn run() -> bool { probe(&PathBuf::from(\"Cargo.toml\")) }",
                &["fs:list"],
            ),
            (
                "two attributes, second parameter",
                "fn f<'a, #[cfg(all())] #[allow(unused)] D: std::ops::Deref<Target = std::path::Path>>(d: &'a D) -> bool { D::Target::is_file(d) }",
                &["fs:list"],
            ),
            (
                "attribute on an impl parameter",
                "struct W<U>(U);\nimpl<#[allow(unused)] T: Tr> W<T> { fn f(&self) -> bool { T::A::read_link(&self.0).is_ok() } }",
                &["fs:list"],
            ),
            (
                "attribute on a const parameter",
                "fn f<#[allow(unused)] const N: usize, #[allow(unused)] D: Tr>(d: &D) -> bool { D::A::is_dir(d) }",
                &["fs:list"],
            ),
            // Decoys: an attributed parameter used as a value path stays pure.
            ("attributed parameter, pure item", "fn f<#[allow(unused)] T: Default>() -> T { T::default() }", &[]),
            ("attributed parameter, pure projection", "fn f<#[allow(unused)] I: Iterator>() -> Option<I::Item> where I::Item: Default { Some(I::Item::default()) }", &[]),
        ],
    );
}

// F3: a generic type alias carried a projection.
//
// `type X<D> = <D as std::ops::Deref>::Target;` binds `X` to `Deref` and
// `Target` only (`D` is the alias's own parameter, skipped), so
// `X::<std::path::PathBuf>::exists(p)` - `Path::exists` - read as
// `Deref::exists` / `Target::exists`, both pure.  An alias whose right-hand
// side holds a projection is now also bound to the unknown type.
#[test]
fn f3_a_projection_in_a_type_alias_is_an_unknown_type() {
    check(
        "F3",
        &[
            (
                "proj_alias_generic",
                "type X<D> = <D as std::ops::Deref>::Target;\npub fn probe() -> bool {\n    let p = std::path::Path::new(\"Cargo.toml\");\n    X::<std::path::PathBuf>::exists(p)\n}",
                &["fs:list"],
            ),
            (
                "proj_alias_short",
                "#![allow(type_alias_bounds)]\ntype X<D: std::ops::Deref> = D::Target;\npub fn probe() -> bool {\n    let p = std::path::Path::new(\"Cargo.toml\");\n    X::<std::path::PathBuf>::exists(p)\n}",
                &["fs:list"],
            ),
            (
                "qualified without a trait, qualified use",
                "type X<D: std::ops::Deref> = <D>::Target;\nfn f(p: &std::path::Path) -> bool { <X<std::path::PathBuf>>::is_file(p) }",
                &["fs:list"],
            ),
            (
                "attributed alias parameter",
                "type X<#[allow(unused)] D: std::ops::Deref> = D::Target;\nfn f(p: &std::path::Path) -> bool { X::<std::path::PathBuf>::is_dir(p) }",
                &["fs:list"],
            ),
            (
                "parameter after a bracketed default",
                "type X<const B: bool = { 2 > 1 }, D = std::path::PathBuf> = <D as std::ops::Deref>::Target;\nfn f(p: &std::path::Path) -> bool { X::<true>::read_link(p).is_ok() }",
                &["fs:list"],
            ),
            // Found while fixing F3: a `>` in a bracketed default ended the
            // parameter list of a trait early, hiding the parameter after it.
            (
                "trait parameter after a bracketed default",
                "trait Tr<T = [u8; (2 > 1) as usize], D: std::ops::Deref<Target = std::path::Path> = std::path::PathBuf> { fn f(d: &D) -> bool { D::Target::exists(d) } }",
                &["fs:list"],
            ),
            // Decoys: aliases without a projection, and projections to pure items.
            ("generic alias, no projection", "type V<T> = Vec<T>;\nfn f() -> V<u8> { V::<u8>::new() }", &[]),
            ("projection alias, pure item", "type It<I> = <I as Iterator>::Item;\nfn f<I: Iterator>() -> Option<It<I>> where It<I>: Default { Some(It::<I>::default()) }", &[]),
            ("bound names are not parameters", "type F<D: AsRef<std::fs::File>> = (std::fs::File, D);\nfn f() -> usize { std::mem::size_of::<F<u8>>() }", &["fs:read"]),
            ("fn-pointer alias with a return arrow", "type F = fn(u8) -> ::core::primitive::u8;\nfn f(g: F) -> u8 { g(1) }", &[]),
        ],
    );
}

// F1: a turbofish on the associated-type segment hid a projection.
//
// `<PathBuf as Deref>::Target::<>::exists(p)` is `Path::exists`.  At
// `exists` the analyzer saw the turbofish `::<>::` and walked back over
// `Target` only: the walk stopped at the `>` closing `<PathBuf as Deref>`, so
// the path read `Target::exists`, with no qualified group and no projection.
// The walk now continues through that `>` into the group, so the item is
// `<PathBuf as Deref>::Target::exists`, the projection of D49.
#[test]
fn f1_a_turbofish_on_the_associated_type_keeps_the_projection() {
    check(
        "F1",
        &[
            (
                "proj_qual_tf",
                "pub fn probe() -> bool {\n    let p = std::path::Path::new(\"Cargo.toml\");\n    <std::path::PathBuf as std::ops::Deref>::Target::<>::exists(p)\n}",
                &["fs:list"],
            ),
            (
                "proj_angle_tf",
                "use std::ops::Deref;\nuse std::path::{Path, PathBuf};\npub fn probe<D: Deref<Target = Path>>(d: &D) -> bool {\n    <D>::Target::<>::exists(d)\n}\npub fn run() -> bool { probe(&PathBuf::from(\"Cargo.toml\")) }",
                &["fs:list"],
            ),
            (
                "Self, qualified with a trait",
                "use std::ops::Deref;\npub trait Probe { fn probe(&self) -> bool; }\nimpl<T: Deref<Target = std::path::Path>> Probe for T { fn probe(&self) -> bool { <Self as Deref>::Target::<>::is_file(self) } }",
                &["fs:list"],
            ),
            (
                "timed wait through a projection",
                "use std::ops::Deref;\nuse std::sync::{Arc, Condvar, Mutex};\npub fn nap() {\n    let m = Mutex::new(());\n    let c = Arc::new(Condvar::new());\n    let g = m.lock().unwrap();\n    let _ = <Arc<Condvar> as Deref>::Target::<>::wait_timeout(&c, g, std::time::Duration::from_millis(1));\n}",
                &["time:sleep"],
            ),
            (
                "two turbofish segments",
                "fn f(p: &std::path::Path) -> bool { <std::path::PathBuf as std::ops::Deref>::Target::<>::Other::<>::read_link(p).is_ok() }",
                &["fs:list"],
            ),
            // Controls that already held: no qualified group in front.
            ("generic parameter shorthand", "fn f<D: std::ops::Deref<Target = std::path::Path>>(d: &D) -> bool { D::Target::<>::exists(d) }", &["fs:list"]),
            ("Self shorthand", "trait P: std::ops::Deref<Target = std::path::Path> { fn f(&self) -> bool { Self::Target::<>::is_dir(self) } }", &["fs:list"]),
            // Decoys: projections and turbofish chains to pure items.
            ("projection to a pure item", "fn f<I: Iterator>() -> I::Item where I::Item: Default { <I as Iterator>::Item::<>::default() }", &[]),
            ("turbofish on a generic type", "fn f() -> std::collections::HashMap<u8, u8> { std::collections::HashMap::<u8, u8>::new() }", &[]),
            ("qualified trait method with a turbofish", "fn f(v: &[u8]) -> Vec<u8> { <[u8] as ToOwned>::to_owned::<>(v) }", &[]),
        ],
    );
}

/// The F1 walk through turbofish groups is bounded: a chain of more than 8
/// groups is `CAP003` (fail closed), so the work per path stays small.
#[test]
fn f1_turbofish_chain_walk_is_bounded() {
    let short = format!("fn f() {{ let _ = <A as B>::C{}::new(); }}", "::<>::D".repeat(8));
    assert_eq!(verdict(&short), Vec::<String>::new(), "{short}");
    let long = format!("fn f() {{ let _ = <A as B>::C{}::new(); }}", "::<>::D".repeat(9));
    assert_eq!(verdict(&long), vec!["?".to_string()], "{long}");
}
