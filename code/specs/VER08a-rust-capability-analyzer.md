# VER08a — Rust Capability Analyzer and Deny-by-Default Name Allowlist

**Parent:** [VER00 §8](VER00-verified-shipping-program.md), [VER08](VER08-capability-analyzers.md), [VER23](VER23-generated-capability-modules.md), [VER26 §5 Step 27](VER26-ct-compare-pilot.md).

This specification formalizes **VER-P027a**: the zero-third-party-dependency static capability analyzer and deny-by-default name allowlist for Rust packages in the repository.

Revision note (audit hardening): this revision turns the behaviour of the crate into testable contracts, states what a lexical analyzer cannot see (§8), adds `CAP003` for constructs it can see but cannot analyse (§3.5), and requires that present-but-excepted constructs stay visible in the output (§6.1). Every divergence from the previous revision is logged in §9.

---

## 1. Objectives and Verification Ladder Placement

```text
  ┌─────────────────────────────────────────────────────────────┐
  │ L4: VER-P027b  Generated Effect Module & Proof Certificate  │
  ├─────────────────────────────────────────────────────────────┤
  │ L4: VER-P027a  Rust Capability Analyzer & Name Allowlist    │ ◄── THIS SPEC
  ├─────────────────────────────────────────────────────────────┤
  │ L3: VER20      Bounds & Constant-Time Trace Proofs (UNSAT)  │
  ├─────────────────────────────────────────────────────────────┤
  │ L2: VER06a     Coverage-Guided Fuzz Engine & Targets        │
  ├─────────────────────────────────────────────────────────────┤
  │ L5: VER07a     Equal-Instruction-Count Microbenchmarks       │
  ├─────────────────────────────────────────────────────────────┤
  │ L0: TF01       Conformance Corpus & Sabotage Runner         │
  └─────────────────────────────────────────────────────────────┘
```

The publication gate (VER00 §9) mandates that no package may ship without an audited capability profile (`required_capabilities.json`). The Rust capability analyzer provides the automated enforcement layer (Spec 13 Layer 3 / 4):

1. **Deny-by-Default Ambient Authority:** Packages declaring an empty capability profile (`capabilities: []`) or lacking a manifest are treated as pure computation. Any *visible* access to filesystem, network, processes, environment, system time, threads, standard I/O streams, randomness, or foreign function interfaces triggers a `CAP001` violation.
2. **Banned Construct Enforcement:** `unsafe`, inline assembly, and foreign linkage trigger `CAP002` unless granted an explicit exception in `banned_construct_exceptions`.
3. **Fail-Closed Operation:** malformed manifests, unreadable sources, unparseable token streams and resource exhaustion exit `2`. Constructs the analyzer can see but cannot analyse are `CAP003` (exit `1`) and cannot be waived by any manifest exception. See §7 for the exact meaning of "fail closed" and §8 for what is out of scope.
4. **Zero Third-Party Dependencies:** `std` only.

---

## 2. Manifest Contract (`required_capabilities.json`)

The analyzer reads `required_capabilities.json` exclusively from the package root (a symlink is refused):

- **Absent manifest:** pure computation (`capabilities = []`, no exceptions). The report says `manifest_present = false`.
- **Present manifest** must conform to `required_capabilities.schema.json` version 1:
  - `version` is the integer `1` (`1.0`, `1e0`, `"1"` are rejected);
  - `package` matches `rust/<name>`, `<name>` in `[a-z0-9_-]+`. The analyzer does **not** check `<name>` against the directory or crate name (gap G-4);
  - `capabilities` use the closed 11-category taxonomy (`fs net proc env ffi time stdin stdout rand thread ui`) with only the actions the schema lists for the category;
  - `banned_construct_exceptions` entries carry `construct` in {`unsafe_code`, `inline_assembly`, `foreign_linkage`}, `language = "rust"`, and a justification of at least 10 characters.
- **Fail-closed manifest validation (exit `2`):**
  - invalid JSON, including duplicate object keys at any depth (a duplicate would let two parsers disagree about what was declared), non-RFC-8259 numbers (leading zeros, bare `-`), raw control characters in strings, non-JSON whitespace, trailing content, nesting deeper than 100;
  - unknown top-level, capability or exception properties;
  - invalid category/action pairs;
  - missing or short (< 10 characters) justifications;
  - duplicate capability declarations (same `category:action:target`);
  - duplicate or unknown banned-construct exceptions.
- **Targets are not enforced (gap G-3).** `fs:read:/etc/hosts` covers *every* detected `fs:read`; the analyzer cannot know a call's runtime argument. `target` is documentation, not a verified bound.

---

## 3. Rust Detection Contract

The analyzer is **lexical**: it tokenizes each file (§3.1), resolves `use`-introduced names package-wide (§3.2), and classifies every path, macro name, method name and keyword it sees (§3.3-3.5). It does not parse Rust, expand macros, resolve types, or look inside other crates (§8).

### 3.1 Lexing contract

A leading byte-order mark is dropped, then a shebang line exactly as rustc's `strip_shebang` decides: a file starting `#!` has its first line skipped unless, after whitespace and *non-doc* comments, the next token is `[` (an inner attribute such as `#![no_std]`). A doc comment is a token, so `#! //! x` is a shebang line. Whitespace is exactly Unicode `Pattern_White_Space` (`\t \n \v \f \r` space, U+0085, U+200E, U+200F, U+2028, U+2029), the set rustc uses; `char::is_whitespace` is a different set. Identifiers are ASCII (`[A-Za-z_][A-Za-z0-9_]*`, `r#` raw form). Any other non-ASCII character outside a string, character literal or comment (a non-ASCII identifier, a non-`Pattern_White_Space` space such as U+00A0, a byte-order mark after offset 0) is `CAP003`: rustc's XID identifier rules and NFC comparison need Unicode tables this zero-dependency crate does not carry, so the analyzer cannot tell what such a name denotes. The lexer follows the package edition, read from `Cargo.toml` `[package] edition = "20xx"` before scanning: C-string literals (`c"…"`, `cr#"…"#`) exist only from edition 2021; in 2015/2018 `cr#"` is the identifier `cr`, a `#` and an ordinary string. The edition is *unknown* when `package.edition` is missing, inherited (`edition.workspace = true`), not a plain string, not one of 2015/2018/2021/2024, when any target sets its own `edition`, or when `Cargo.toml` cannot be parsed; then a raw C-string prefix (`cr"`, `cr#"`) is `CAP003`, because the two readings end the literal at different quotes. A cooked `c"…"` has the same boundaries in both readings and is never flagged. Comments (line, nested block, doc), string literals (plain, raw `r#"…"#`, byte, raw byte, C-string) and character/byte literals never produce detections. Lifetimes (`'a`) are distinguished from character literals (`'a'`, `'\n'`, `'\u{1F600}'`). Raw identifiers (`r#fs`) denote the same identifier as `fs`; `r#unsafe` is an ordinary identifier, a non-raw `unsafe` token is always a banned construct. An unterminated comment, string or raw string is a parse error (exit `2`).

Everything else - tokens inside `macro_rules!` bodies, macro-invocation arguments, `#[cfg(...)]`-disabled items, `#[allow(unsafe_code)]` regions, `#[cfg(test)]` modules - is analysed as ordinary code. `cfg` is ignored (conservative).

### 3.2 Name resolution (aliases, renames, globs, re-exports)

All files are lexed first; one package-wide binding table is built:

| Construct | Effect |
|---|---|
| `use a::b::c;` | binding `c -> a::b::c` |
| `use a::b as c;` | binding `c -> a::b` |
| `use a::{b, c as d, self};` | one binding per leaf; `self` binds the prefix's last segment |
| `use a::b::*;` | glob prefix `a::b` |
| `use ::a::b;` | leading `::` ignored |
| `use a::b as _;` | no binding; the path itself is still classified |
| `extern crate a as b;` | binding `b -> a`; any other shape (`extern crate a as b <more>;`, a metavariable name, no `;`) is `CAP003` |
| `type F = a::b;` / `type F<T> = &a::b<T>;` / `type F = (a::b);` / `type F = <a::B as c::D>::E;` | one binding `F -> p` for **every** path `p` on the right-hand side up to the `;` that ends the item (inside generic arguments, parentheses and qualified projections too; the alias's own generic parameters, lifetimes and keywords are skipped). A metavariable on the right-hand side (`type F = $t;`), a macro invocation (`type F = ty!();`, D44), or a projection (a qualified group `<..>::` anywhere on the right-hand side, or a path of two or more segments headed by one of the alias's own generic parameters: `type F<D> = <D as Deref>::Target;`, `type F<D: Deref> = D::Target;`, D52) also binds `F` to an *unknown type*, for which `F::name` gets the method-call verdict of `.name(` (§3.3). The alias's parameter names are its top-level parameter starts (attributed ones included), and its parameter list is delimited bracket-aware (a `>` inside `{ .. }`, `[ .. ]` or `( .. )` does not end it). `type Item;` / `type Item: Bound;` bind nothing. Paths in the alias's generic parameter *defaults* (`type F<P = std::path::Path> = P;`) are not bound yet: open gap CA-OG1 (§8.1). A metavariable *name* (`type $n = ..;`), a right-hand side with no `;` before a closing bracket or the end of the file, unbalanced generics or a non-identifier name is `CAP003`. `type` inside a `macro_rules!` matcher is a pattern and is ignored |

Bindings are expanded through other bindings at **every segment**, not only the head, to a fixed point: a binding `n -> p` gains `n -> t + p[i+1..]` for every segment `p[i]` that is itself a binding with spelling `t`, repeated until nothing changes, so `mod a { pub use std as s; } use a::s as t;` gives `t -> std` (two-level renames through modules, `super::`, `crate::`, nested `extern crate`, and globs over such renames are followed). A spelling records the set of original bindings it was derived from and is never substituted through one of them again (rustc never resolves a path through the same import twice; that is an import cycle), so self-similar names such as `pub use self::sha256::sha256;` with `use crate::sha256;` converge; when two derivations reach one spelling, the intersection of their sets is kept (fewer blocked substitutions, so the result only grows). Bounded: growth past 64 segments, past 20,000 spellings, past 4,000,000 substitution attempts or past 16 rounds is `CAP003`. Identical bindings are de-duplicated *before* expansion and every cap is checked *inside* the expansion loop, so no intermediate result can exceed it (§5, M4). For every path occurrence (in `use` leaves and everywhere else) the analyzer classifies the path **and** every *candidate*: for each path segment equal to a binding name, `binding_target + remaining_segments`; and, for each such spelling not headed `std`/`core`/`alloc`, `glob_prefix + suffix` for each glob and **every suffix** of the spelling that does not start at `crate`/`self`/`super`/`Self` (which module holds a glob is not tracked, so `m::process::Command` with `mod m { pub use std::*; }` is tried as `std::process::Command`). Glob prefixes are themselves normalised through the bindings at every segment (`use crate::s::*;` with `pub use std as s;` is a glob over `std`). A glob over a bare standard crate (`use std::*;`, `use core::*;`, `use alloc::*;`, also through an alias) is `CAP003`: every std module becomes nameable without a `std` segment. Glob spellings under any other glob may add `CAP001`/`CAP002` results, never `CAP003`; under a bare-root glob their `CAP003` is reported too. Pinned by tests:

- `use std::{fs, process::Command};`, `use std as s; s::process::Command`, `use std::fs as f; f::read(..)`, `use std::*; process::Command::new(..)` (also `CAP003`), `mod m { pub use std::fs::*; } m::remove_dir_all(..)` (`fs:delete`), and `a::s::process::Command` (another file has `pub use std as s;`) resolve to `std::` paths and are detected.
- Re-exporting a capability (`pub use std::process as p;`) is itself a detection at the re-export site.
- Bindings are package-wide, not per scope: an unrelated local item sharing a binding's name may yield a false positive. Accepted.
- Qualified paths name the self type: after `>::` the item is read as `<self type path>::item`, so `<File>::create(..)`, `<std::fs::File as T>::create(..)` and the turbofish `Path::<X>::exists` classify as `File::create`, `std::fs::File::create` and `Path::exists`. A self type in parentheses is read through them (`<(std::path::Path)>::exists` is `Path::exists`, D44). A macro invocation in the self position (`<ty!()>::exists`, `<ty!() as T>::exists`) is an *unknown type*: the item gets the method-call verdict of `.name(` (§3.3), D44. A turbofish path that follows another group (`<T as Tr>::A::<>::item`, `a::<>::b::<>::c`) is read through it: the walk back from `::<` continues through the `>::` before it and prepends what it finds, so the item is `<T as Tr>::A::item` (D53); more than 8 such groups in one path is `CAP003`. A `<` that cannot be matched within 512 tokens is `CAP003`; a self type with no path (`<[u8]>::len`) is pure.
- `Self` names the self type of the enclosing `impl` (D37): inside the braces of `impl [<..>] [Trait for] Type`, `Self::x` (and `<Self>::x`, `<Self as T>::x`) is also classified as `Type::x`, resolved through the bindings like any path, so `impl Probe for std::path::Path { .. Self::exists(self) .. }` is `fs:list`. Regions are found lexically: every non-raw `impl` token with a `{` before the next `;` (within 1,024 tokens, skipping `<..>`, `(..)`, `[..]`) opens one, and `Self` is resolved against **every** enclosing region, not only the innermost, so a misread region (a `-> impl Trait {` return type) adds spellings but can never hide the real impl. A self type behind `&`/`*`/`mut`/`dyn` or parentheses is read through them; a slice, tuple, array, `!` or fn-pointer self type adds nothing (its inherent items are pure). A self type containing a metavariable, a macro invocation or a qualified projection (`impl T for $t`, `impl T for ty!()` (D44), `impl T for <A as B>::C`), more than 32 nested regions, and any `Self` inside a `macro_rules!` body (it names whatever impl the macro expands in) are *unknown*: `Self::name` then gets the method-call verdict of `.name(` (§3.3), e.g. `Self::exists` is `fs:list`; for a longer path the method is the **last** segment (`Self::Target::exists` is `.exists(`, D49). `Self` is scoped to its region rather than added to the package-wide table, because a package-wide `Self` would carry one target per `impl` in the package and every `Self::new` would exceed the 64-spelling cap.
- Associated-type projections are *unknown types* (D49): `<T as Trait>::Assoc::item` (a qualified path whose group has a top-level `as`, followed by two or more segments, counting those behind a turbofish: `<T as Trait>::Assoc::<>::item`, D53), `Self::Assoc::item`, and `P::Assoc::item` where `P` is declared as a generic parameter anywhere in the file (`impl<P ..>`, `fn f<P ..>`, `struct`/`enum`/`union`/`trait`/`type NAME<P ..>`; a parameter name is any top-level identifier after `<`, `,`, `const` or the `]` of an attribute, so `<#[allow(unused)] P ..>` declares `P`, D51). The analyzer does not evaluate projections (`<PathBuf as Deref>::Target` is `Path`), so the item gets the method-call verdict of its last segment: `D::Target::exists(d)` with `D: Deref<Target = Path>` is `fs:list`. Rust accepts the shorthand `X::Assoc` only for a type parameter or `Self` (E0223), so these shapes are every way to *spell* a projection at its use. They are not every way to *reach* one: a type alias carries a projection to a use that names only the alias (`type X<D> = <D as Deref>::Target; X::<PathBuf>::exists(p)`), so an alias whose right-hand side holds a projection is bound to the unknown type (D52, row above). The same last-segment rule applies to an unknown alias (`type P = $t;` / `ty!()`, `P::Target::exists`) and an unknown `Self`.
- Type aliases are bindings: `type F = File; F::create(..)` reaches `std::fs::File::create` (`fs:write`), `type U = UdpSocket; U::bind(..)` reaches `net:listen`. (An associated `type Item = Foo;` inside an `impl` is also a binding; bindings are package-wide, see above.)
- Bounded work per path: a path longer than 64 segments, or one with more than 64 candidate spellings, is `CAP003`; after 200,000 extra spellings in one file the remaining paths are classified as written and the file carries a `CAP003`.

### 3.3 Capability classification

`P` is a resolved candidate path with a leading `::` removed. Matching is by whole segment, never by string prefix. First matching row wins.

| Resolved path | Category:action |
|---|---|
| `asm`, `global_asm`, `naked_asm`, `{core,std}::arch::{asm,global_asm,naked_asm}` (macro or not) | banned `inline_assembly` |
| `std::fs::{read_dir, metadata, symlink_metadata, canonicalize, read_link, exists, try_exists, ReadDir, DirEntry, Metadata}` | `fs:list` |
| `std::fs::{create_dir, create_dir_all, DirBuilder}` | `fs:create` |
| `std::fs::{remove_file, remove_dir, remove_dir_all}` | `fs:delete` |
| `std::fs::{write, copy, rename, hard_link, soft_link, set_permissions, OpenOptions}`, `std::fs::File::{create, create_new, create_buffered, options, set_times, set_modified, set_permissions}` | `fs:write` |
| any other `std::fs` item (`read`, `read_to_string`, `File`, `File::open`, bare module) | `fs:read` |
| `std::net::TcpListener`, `std::net::UdpSocket::bind` | `net:listen` |
| `std::net::ToSocketAddrs` | `net:dns` |
| `std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, AddrParseError, Shutdown}` | pure (data) |
| any other `std::net` item (`TcpStream`, `UdpSocket`, bare module) | `net:connect` |
| `std::process::{exit, abort}` | `proc:signal` |
| any other `std::process` item (`Command`, `Child`, `ExitCode`, bare module) | `proc:exec` |
| `std::env::{set_var, remove_var, set_current_dir}` | `env:write` |
| any other `std::env` item (`var`, `vars`, `args`, `current_dir`, `consts`, bare module) | `env:read` |
| `std::backtrace::*` (reads `RUST_BACKTRACE`) | `env:read` |
| `std::thread::{sleep, sleep_ms, park_timeout, park_timeout_ms}` | `time:sleep` |
| any other `std::thread` item (`spawn`, `scope`, `Builder`, `current`, bare module) | `thread:spawn` |
| `std::time::{Duration, TryFromFloatSecsError}` | pure |
| any other `std::time` item (`Instant`, `SystemTime`, `UNIX_EPOCH`, bare module) | `time:read` |
| `…::Condvar::{wait_timeout, wait_timeout_ms, wait_timeout_while}`, `…::Receiver::{recv_timeout, recv_deadline}`, however qualified (`std::sync` is a pure module) | `time:sleep` |
| `std::io::{stdin, Stdin, StdinLock}` | `stdin:read` |
| `std::io::{stdout, stderr, Stdout, Stderr, StdoutLock, StderrLock, _print, _eprint}`; macros `print println eprint eprintln dbg` (bare or `std::`-qualified) | `stdout:write` |
| `std::io::{pipe, PipeReader, PipeWriter}` (an anonymous OS pipe: two raw file descriptors or handles; the taxonomy has no IPC category, so it shares the raw-handle row below, D47) | `ffi:call` |
| other `std::io` items (`Read`, `Write`, `Cursor`, `Error`, `copy`, …) | pure |
| `std::ffi::*`, `core::ffi::*`, `alloc::ffi::*` | `ffi:call` |
| `std::os::…::net::UnixListener`, `std::os::…::net::UnixDatagram::{bind, bind_addr}` (Unix-domain listeners: accept traffic like `TcpListener` / `UdpSocket::bind`, and create a socket file, D48) | `net:listen` |
| any other `std::os::…` containing segment `net` / `process` / `fs` / `thread` | `net:connect` / `proc:exec` / `fs:write` / `thread:spawn` |
| any other `std::os::*` (raw fds/handles) | `ffi:call` |
| `{core,std}::arch::*` item whose name starts `_rdrand` / `_rdseed` (hardware randomness) | `rand:read` |
| `std::path::absolute` (joins the current directory) | `env:read` |
| `…::Path::{read_dir metadata symlink_metadata canonicalize read_link exists try_exists is_file is_dir is_symlink}` and the same on `PathBuf`, however qualified (`Path::exists(p)`, `std::path::Path::metadata(p)`, `<PathBuf>::is_file(p)`, `let g = Path::canonicalize;`) | `fs:list` |
| any path containing segment `RandomState`; any path headed `rand getrandom fastrand oorandom rand_core rand_chacha` | `rand:read` |
| paths headed `libc nix winapi windows windows_sys libloading` | `ffi:call` |
| paths headed `tokio mio socket2 reqwest hyper ureq async_std smol` | `net:connect` |
| *unresolved-head shorthand*: a path headed `fs net process env thread time ffi io os` that no binding explains is treated as `std::<same path>` | as above |
| pure std modules/macros (below) | pure |

Method heuristic: `.name(` with `name` in {`read_dir metadata symlink_metadata canonicalize read_link exists try_exists is_file is_dir is_symlink`} is `fs:list`, because `Path::exists()` reaches the filesystem without naming `std::fs`; `.set_times(`, `.set_modified(`, `.set_permissions(` are `fs:write` (they write metadata through a handle opened only for reading); `.recv_timeout(`, `.recv_deadline(`, `.wait_timeout(`, `.wait_timeout_ms(`, `.wait_timeout_while(` are `time:sleep` (timed waits on the pure `std::sync` types, like `thread::park_timeout`); `.elapsed(` is `time:read` (`UNIX_EPOCH.elapsed()` reads the clock without naming `SystemTime`); `.to_socket_addrs(` is `net:dns` (a glob `use std::net::*;` brings `ToSocketAddrs` into scope without naming it, D46). A turbofish between the name and `(` does not hide the call: `.exists::<>(` is `.exists(` (D43), and `.name::<` counts as a call without the turbofish being walked, because a field takes no generic arguments (D50). This over-approximates (a user method called `exists` or `elapsed` is flagged).

*Receiver resolution (L4).* The heuristic is skipped for exactly one provable shape: the receiver is `Local::ctor(..)` optionally followed by `.unwrap()`, `.expect(..)`, `.clone()` or `?`, where `Local` is a struct/enum of the package whose own inherent `impl Local { .. }` declares both `ctor` (an associated fn returning `Self`, `Local`, `Result<Self|Local, _>` or `Option<Self|Local>`) and a `self` method of the called name, and `Local` is not rebound by a `use`/`type`, and no glob import in the package reaches outside the crate (a glob headed by anything but `crate`/`self`/`super`/`Self` after normalisation, e.g. `use std::path::*;` or `use dep::*;`, may supply a different `Local` that no binding records). Inherent methods win method resolution, so such a call cannot reach `Path::<name>`. Anything else - a variable, a field, another adapter, a trait method - stays flagged. Example: `Url::parse(s).unwrap().canonicalize()` in url-parser is the crate's own `Url::canonicalize`, not a filesystem call. Known bypasses of this waiver (a `cfg`-ed out or macro-body impl, generic instantiations, same-named types, arbitrary self types) are open gap CA-OG2 (§8.1).

`core::arch::*::__cpuid` and similar CPU-identification intrinsics are *informational only* (INFO): they reveal the CPU model but are not an OS authority, and are not reported.

Pure `std`/`core`/`alloc` modules: `alloc any arch array ascii borrow boxed cell char clone cmp collections convert default error f32 f64 fmt future hash hint i8 i16 i32 i64 i128 isize iter marker mem num ops option panic path pin prelude primitive ptr rc result slice str string sync task u8 u16 u32 u64 u128 usize vec`, plus `core::net`, `core::time`. Pure macros: `assert assert_eq assert_ne debug_assert debug_assert_eq debug_assert_ne matches vec format format_args panic unreachable unimplemented todo write writeln concat stringify line column file module_path cfg compile_error env option_env include_str include_bytes thread_local is_x86_feature_detected is_aarch64_feature_detected try` (`env!` is compile-time and pure).

**Deny by default:** a path headed `std`, `core` or `alloc` whose second segment is not a classified module, pure module or pure macro (e.g. `std::rt`, `std::intrinsics`, `std::simd`) is `CAP003` ("unclassified std path"). The previous "everything unknown is pure" fallthrough is retired for std surfaces.

Anything else (`crate`, local modules, unlisted third-party crates) is **pure by default** - gap G-1. Implicit authority never *named* in source (entropy seeding `HashMap::new()`, allocation, panic output to stderr) is not detected (G-2).

### 3.4 Banned constructs (`CAP002`, waivable)

| Construct id | Triggers |
|---|---|
| `unsafe_code` | every non-raw `unsafe` token: blocks, `unsafe fn/impl/trait/extern`, `#[unsafe(no_mangle)]`, inside `macro_rules!` bodies and macro arguments. `#[allow(unsafe_code)]`, `#![forbid(unsafe_code)]`, `#[cfg]` never suppress detection. |
| `inline_assembly` | `asm!`, `global_asm!`, `naked_asm!`; qualified, imported or renamed |
| `foreign_linkage` | an attribute containing any identifier in {`no_mangle export_name link_name link link_section link_ordinal linkage naked`}, also nested in `cfg_attr(…)` |

`extern "ABI"` in any position (item, block, fn-pointer type) is a detection `ffi:call` for **every** ABI string whose source text is not exactly `Rust` (`"C-unwind"`, `"system"`, `"vectorcall"`, escaped and raw strings are caught). `extern { … }`, `extern fn`, `unsafe extern` likewise. `extern crate` is not FFI.

`extern "Rust" { … }` (an *import block*, whatever the ABI string) declares symbols defined outside the crate, exactly like `extern "C" { … }`: it is a banned `foreign_linkage` and an `ffi:call` detection. `extern "Rust" fn f() {}` and `extern "Rust" fn(u8)` types are ordinary and not flagged.

An exception waives the construct for the whole package, not per item. Waived constructs are **still reported** (§6.1).

### 3.5 Unanalysable constructs (`CAP003`, never waivable, exit `1`)

| Construct | Why |
|---|---|
| a bare `include`, `include_str` or `include_bytes` identifier inside the arguments of any macro invocation (`m!(include, "../x.rs")`), and in a `macro_rules!` body any `$name!(…)` (a macro chosen by a metavariable) | the callee invokes the macro out of sight (M2). `$a != b` and `!$a` are not flagged. Accepted false positive: a variable named `include` used as a macro argument (`assert!(include)`) |
| `include!(…)`: bare, or any `std`/`core`/`alloc` path ending in `include` at any length (`std::include!`, `std::prelude::v1::include!`, `core::prelude::rust_2021::include!`), and a `use` importing such a path (`use std::prelude::v1::include as i;`) | pulls in code the scanner never sees (`include_str!`/`include_bytes!` pull in data and are allowed) |
| in a `macro_rules!` body, a `$` metavariable inside a `<..>` group followed by `::` (`<$t>::read_dir`), or a `$(..)` repetition (any separator / kleene operator) followed by `::` (`$($t)*::x`) | the caller chooses the path root, so `Path`/`File` UFCS is hidden (N1, N2). Fails closed; no method list. Exception: the item after `::` is a SCREAMING_CASE associated constant (`<$t>::MAX`), which cannot perform I/O (segment-tree) |
| `include_str!(…)` / `include_bytes!(…)` | deliberately **not** flagged: they embed *data*, never compiled code, so a path outside the scan set adds no authority; acting on the bytes (parsing, executing) is visible in scanned code (L-B) |
| `#[path = "…"]` (also inside `cfg_attr`) | module location is untracked; may point at an unscanned directory |
| in a `macro_rules!` body, a metavariable adjacent to `::` (`std::$m::read`, `$p::x`, `$(…)::*`) | the expansion constructs a path the scanner never sees |
| in a `macro_rules!` transcriber, a `#` that does not start `#[` or `#![` (`# $a`, `#! $a`, a `#` before a closing delimiter or used as a repetition separator) | `m!([path = ".."])` / `m!([no_mangle])` would create an attribute unseen (HIGH-2) |
| in a `macro_rules!` transcriber, a `tt`-fragment metavariable (or one not declared in the arm's matcher, or a `$(..)` repetition containing one) directly adjacent - without an operator or delimiter between - to an identifier that can be a path segment (anything but a strict non-path keyword such as `let`, `fn`, `struct`, `return`), to another metavariable, or followed by `[` or `!` (not `!=`); also between the last and first units of consecutive repetition iterations and around an identifier separator | a `tt` may be `::`, `#` or `!`: `std $s fs $s write(..)` with `m!(::)` is `std::fs::write` (HIGH-2). Fragments other than `tt` (`ident`, `expr`, `ty`, `path`, `vis`, …) cannot produce those tokens and are not flagged; neither is a verbatim pass-through `$($t)*` of a matcher repetition `$($t:tt)*` with the same separator and operator, which re-emits the caller's tokens in the caller's order. Accepted false positive: `a $op b` with `$op:tt` |
| in a `macro_rules!` transcriber, an `ident`- or `tt`-fragment metavariable (or one not declared in the matcher, or a repetition ending in one) directly followed by an identifier that can be a path segment, a string literal, another metavariable or repetition, a `{ .. }` group that can be a use tree (only identifiers, `::`, `,`, `*`, braces and metavariables, with `::`, `as`, `*` or a non-`ident` metavariable in it), or - at the start of an item - `<`; also across repetition iterations and separators (D39) | an `ident` fragment matches keywords: `($k:ident) => { $k std::io as out; }` with `kw!(use)` is `use std::io as out;`, `$k crate std as s;` with `kw!(extern)` is `extern crate std as s;`, and the binding never reaches the table (H4). The rule is on the transcriber, not the call site: a call-site rule ("a bare keyword argument") is unsound, since `kw!(use std;)` against `($k:ident $s:ident ;) => { $k $s as out; }` passes a complete `use std;` while the expansion binds `out`. Every tracked binding keyword (`use`, `extern crate`, `type`, `impl`, `mod`) is followed by a name, path or group, so a strict keyword after the metavariable (`$x as u8`, `for $i in`) is not a trigger. Accepted false positive: `$a < b` at the start of a statement |
| `use` directly followed by a metavariable other than `$crate` (`use $i as out;`, `use $p;`); `use $crate::..` is read as `use crate::..` (D39) | the caller chooses what is imported: `($i:ident) => { use $i as out; }` with `u!(std)` binds `out -> std` unseen |
| in a `macro_rules!` transcriber, `.` followed by a repetition `$( .. )`, or by a metavariable and then `(` or `::` (`$e.$m()`, `$e.$m::<T>()`); not after `..`, and not a spliced field `$e.$f` (D40) | the method is chosen by the caller, so `call!(Path::new(".."), exists)` calls `Path::exists` and the method heuristic never sees the name (H7) |
| a `macro_rules!` body that is not a sequence of `(matcher) => {transcriber}` arms | cannot tell matcher from transcriber (rustc rejects it). Matchers are patterns, not code, and are not checked by the splice rules |
| in a `macro_rules!` body, an attribute whose name is a metavariable (`#[$a]`) or a `cfg_attr` containing one | `m!(no_mangle)` would create `#[no_mangle]` unseen. This also flags the common `$(#[$meta:meta])*` forwarding idiom (accepted false positive) |
| `#[proc_macro]`, `#[proc_macro_derive]`, `#[proc_macro_attribute]`, or `proc-macro = true` in `Cargo.toml` | compile-time code execution with full authority |
| unclassified `std`/`core`/`alloc` path (§3.3) | deny by default |
| alias cycle / unbounded expansion (§3.2) | resolution did not converge |
| `Cargo.toml` key `path`, `build`, `proc-macro` or `proc_macro` **anywhere outside** the dependency tables (§3.5.1) whose value is not verifiable; for `path`/`build`: absolute, contains `..`, starts with a hidden component, or starts with an unscanned directory (`tests benches examples target node_modules _build`) | shipped code outside the scanned set. (`[[test]]`/`[[bench]]`/`[[example]]` `path`s are not shipped and not examined, G-5.) |
| `Cargo.toml` `[lib]`, `[[bin]]`, `[[example]]`, `[[test]]` or `[[bench]]` `path`, or `build`, whose value does not end in `.rs` (compared exactly; `src/b.RS` is refused too) (D41) | Cargo compiles the named file whatever its extension, but the scanner reads only `.rs` files: `[lib] path = "src/lib.txt"` compiled `lib.txt` while a decoy `src/lib.rs` was scanned (H5) |
| `Cargo.toml` text the reader cannot parse | fail closed (§3.5.1) |

#### 3.5.1 `Cargo.toml` reader (M3)

`Cargo.toml` is read by a small TOML-subset *tokenizer*, not line by line, so strings, arrays and inline tables are consumed as units and no state is lost between lines. Every value is flattened to a full key path (`bin`, `[]`, `path`) and one rule is applied to that path however the author spelled it: dotted keys (`lib.path`), quoted keys and tables (`["lib"]`, `'lib'.path`), top-level inline tables (`lib = { path = ".." }`, `package = { build = ".." }`), arrays of inline tables (`bin = [{ path = ".." }]`), multi-line strings and multi-line arrays.

- A watched key (`path`, `build`, `proc-macro`, `proc_macro`) is accepted only with a verifiable value: a plain single-line string without escapes that stays inside the scanned set; `false` for `build` / `proc-macro` / `proc_macro`. A multi-line string, an escape sequence (`"t\u0065sts/x.rs"` can spell `tests`), an array, a table, or a table-header form (`[lib.path]`) there is `CAP003`. `proc-macro = true` is `CAP003`.
- A watched key nested *under* another structure (a path segment named `path` that has children) is `CAP003`.
- Exempt: `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[target.*.…dependencies…]` (a dependency's `path` names another crate), `[workspace.dependencies]`, `[patch.*]`, `[replace]`, `[features]` (feature names may be called `path`/`build`), and `path` in `[[test]]`/`[[bench]]`/`[[example]]`.
- Any text the reader does not understand (unterminated string or bracket, missing `=`, escaped quoted key, nesting deeper than 32) is one `CAP003`; analysis never silently skips a line.

### 3.6 Third-party dependencies

Names under `[dependencies]`, `[build-dependencies]` and `[target.*.dependencies]` in `Cargo.toml` are listed as `unanalyzed_dependencies` in the report and printed as `WARN` lines. They do not change the exit code. A package with unanalyzed dependencies must not be presented as "no unsafe / no FFI / no process" on this analyzer's authority alone.

---

## 4. Deny-by-Default Name Allowlist

The pure module/macro lists of §3.3 are the allowlist for `std`/`core`/`alloc`. Local items and unlisted external crates are pure by default (G-1).

---

## 5. Resource Ceilings and Scanner Boundaries

- Maximum source files **10,000**; per file **4 MiB**; cumulative **64 MiB**; directory depth **64**; manifest **1 MiB**.
- Work ceilings (M4, each exceeded -> `CAP003`, never a hang or an allocation abort): 20,000 distinct use bindings / alias entries (also the spellings produced while closing the binding table, §3.2); 4,000,000 substitution attempts and 16 rounds while closing it; 256 distinct glob imports kept; 4,096 glob spellings produced while normalising globs (pure ones included); 64 segments per path; 64 spellings per path; 200,000 extra spellings (alias and glob) per file; `macro_rules!` body scanning bounded to 4x the token count per file; `Cargo.toml` value nesting 32. Bracket matching is a single pass per file (`#[#[#[…` is linear).
- Symlinks (file or directory) inside the package are refused (exit `2`).
- Every `.rs` file under the package root is scanned, **including** `src/capabilities.rs` (previously exempt; a hand-edited generated module is scanned like any file) and a root `build.rs`.
- Excluded: hidden directories (unreachable as modules without `#[path]`, which is `CAP003`); at the package **root only** `target/ node_modules/ _build/ tests/ benches/ examples/`. A nested directory named `target`, `tests`, … (e.g. `src/target/mod.rs`) **is** scanned.
- `tests/`, `benches/`, `examples/` are not shipped and not analysed (gap G-5).
- *Root-level files must not reach the skipped directories (HIGH-5).* A crate root at the package root (`build.rs`, or a `lib.rs`/`main.rs` placed there by a Cargo `path` key) resolves `mod NAME;` to `./NAME.rs` or `./NAME/mod.rs`. So in any `.rs` file directly at the root: a `mod` named like a skipped directory (ASCII case-insensitive; inline or out-of-line; any nesting, since inline `mod tests { mod x; }` reaches `tests/x.rs`), an out-of-line `mod` in a file whose stem is a skipped name (`tests.rs`, whose children live in `tests/`), and a `mod` keyword not followed by a name (handed to a macro, `m!(mod)`) are `CAP003`. In any `macro_rules!` transcriber, `mod $name;`, and `mod $name { .. }` whose body contains `mod` or a metavariable that can produce one (`tt`, `item`, `stmt`, undeclared, or an `ident` followed by a non-keyword name), are `CAP003`: the invocation may be in a root file. Cargo `path` keys naming a root-level file stay allowed; the file is scanned and this rule bounds what it can reach. (Chosen over treating every root-level Cargo target as escaping: the default `build.rs` is root-level whatever `Cargo.toml` says, so the `mod` rule is needed in any case.)

---

## 6. Exit Behavior and Diagnostic Format

```bash
ca-capability-analyzer-rs --dir <PACKAGE_DIR> [--verbose]
```

- **`0`** Clean: every detected capability declared; every banned construct excepted; no `CAP003`.
- **`1`** Violations (`CAP001`, `CAP002`, `CAP003`), certificate mismatch, or module drift.
- **`2`** Tool error: bad arguments, malformed manifest, source I/O failure, parse failure, quota exceeded.

```text
CAP001: Undeclared capability 'fs:read:*' detected at src/lib.rs:42:15 ('std::fs::read')
CAP002: Banned construct 'unsafe_code' detected at src/barrier.rs:104:9 ('unsafe')
CAP003: Unanalysable construct detected at src/lib.rs:3:0 ('include!'): code pulled in by include! is not scanned
```

Line is 1-based, column 0-based, both counted in characters.

### 6.1 Present-but-excepted must stay visible (AF-22)

A clean exit does **not** mean banned constructs are absent. The report carries `excepted_constructs`: every banned-construct occurrence a manifest exception waived, with file, line, column. `format_text` prints them on **every** PASS (not only `--verbose`) and ends every PASS with a machine-readable line:

```text
SUMMARY unsafe_code=present-excepted(2) inline_assembly=absent foreign_linkage=absent ffi=absent unanalyzed_dependencies=0
```

Per-construct status: `absent`, `present-excepted(N)`, `present-violation(N)`. `ffi` is `absent` or `present(N)` (declared `ffi:*` detections). The phrase "pure computation" is printed only when no capability and no excepted construct is present. Consumers (the assurance report) **must** require `unsafe_code=absent` before claiming "no unsafe"; `present-excepted` is not "no unsafe". Library API: `AnalysisReport::construct_status(&str) -> ConstructStatus`.

---

## 7. What "Fail Closed" Means Here

1. *Tool failure* (cannot read, cannot parse, over quota, malformed manifest): exit `2`, never a pass.
2. *Seen but not analysable* (§3.5): exit `1`, `CAP003`, not waivable.
3. *Unknown std surface* (§3.3): `CAP003`.
4. *Unseen constructs* (§8): **not** fail-closed; the verdict says nothing about them.

## 8. Out of Scope (honest limits of a lexical analyzer)

- **G-1** Names that do not resolve to `std`/`core`/`alloc` or the listed crates are assumed pure: local items, **third-party crates** (only reported, §3.6), safe wrappers around them. A dependency may do anything.
- **G-2** Implicit authority (entropy for `HashMap`, allocation, panic output).
- **G-3** Capability `target` scoping.
- **G-4** Manifest `package` is not matched to the directory.
- **G-5** `tests/`, `benches/`, `examples/`, `[dev-dependencies]` are not analysed though `cargo test` executes them.
- **G-6** Macros are not expanded: authority produced only by expansion of *another crate's* macros (`#[derive(ThirdParty)]`, `thirdparty::mac!()`) is invisible. Metavariable-spliced paths and attributes in local `macro_rules!` are `CAP003` (§3.5: `::` next to a metavariable, a `tt` metavariable next to an identifier or another metavariable, `#` not starting `#[`); method names spliced via `.$m(` are `CAP003` (D40), and so is an `ident`/`tt` metavariable in a keyword position (D39).
- **G-7** Dynamic behaviour (function pointers, trait objects, `dlopen` through any crate, `/proc/self/mem` through a declared fs capability).
- **G-8** Type-directed routes beyond the §3.3 method heuristic and the type-alias / qualified-path / receiver / `Self` rules of §3.2-3.3. A binding whose target begins `crate::`/`super::`/`self::` (`type F = super::File;`, `use super::File as G;`) is not followed into the module it names (modules are not tracked); every same-named binding elsewhere in the package is followed instead, at any segment (`G -> super::File` with `use std::fs::File;` anywhere reaches `std::fs::File`). Before D34 this sentence claimed the same, but only the *first* segment of a binding target was expanded, so `super::File` never reached the `File` binding.
- **G-9** `cfg` is ignored (conservative); compiler flags, `.cargo/config.toml` and `RUSTFLAGS` are not examined.

### 8.1 Known open gaps (security review round 3)

A third security review of the round-3/round-4 fixes built crates that the
analyzer passes (exit `0`) while `cargo check` compiles a capability call into
them. The cheap ones are fixed (D43-D49, `tests/test_secreview_round5.rs`,
including the review's G-A, associated-type projections, closed by D49). A
follow-up review of those fixes found four shapes they did not reach (a `>`
in brackets inside a method turbofish, an attributed generic parameter, a
projection carried by a generic type alias, a turbofish on the
associated-type segment); all four are fixed (D50-D53). The two below are
**known, open false negatives**. They are listed here, with a
stable id, as backlog items (`VER-BACKLOG.md`), not as accepted behaviour: a
`PASS` from this analyzer does not exclude them.

#### CA-OG1: generic defaults on type aliases (review G-C)

```rust
type F<P: ?Sized = std::path::Path> = P;
pub fn probe(p: &std::path::Path) -> bool { <F>::exists(p) }   // Path::exists: fs:list, reported pure
```

*Root cause.* `type_alias_leaves` (`src/parser.rs`) skips the alias's own
`<..>` generic parameter list, defaults included, and drops a right-hand side
that is a single parameter name as trivial (`type F<P> = P;`). So `F` gets no
binding, and `<F>::exists` / `F::exists` reads as a local path. In type
position `F` (no arguments) is `F<std::path::Path>`, i.e. `Path`. Case
`alias-default-param`.

*Intended fix.* Treat every path inside a parameter default
(`P = <type>`, `const N: T = <expr>`) as a right-hand-side path: bind the alias
to each of them, exactly as paths after `=` are bound. A right-hand side that
is a parameter with a default then reaches the default. A default that is a
metavariable or a macro invocation (`type F<P = $t> = P;`,
`type F<P = ty!()> = P;`) binds the alias to the unknown type, as a
right-hand side of that shape already does. A default that cannot be
delimited is `CAP003`, like the rest of the item.

#### CA-OG2: the local-receiver waiver (L4) can be bypassed (review G-E)

The waiver of §3.3 L4 skips the method heuristic for
`Local::ctor(..).name()` when `Local` has its own inherent `name`. It rests on
"an inherent method of that name always wins method resolution". Each case
below defeats that while `Local` derefs to `std::path::Path`, so the call is
`Path::exists` (`fs:list`) and the analyzer reports it pure:

| Case | Shape | Why the inherent method does not apply |
|---|---|---|
| `waiver-cfg-out` | `#[cfg(any())] impl L { pub fn exists(&self) -> bool { .. } }` | the inherent impl is compiled out |
| `waiver-macro-body` | the only `impl L { fn exists(&self) .. }` sits in a `macro_rules!` body that is never invoked | the impl never exists |
| `waiver-generic-instance` | `impl L<u8> { fn exists(&self) .. }`, `impl L<PathBuf> { fn new() -> Self }`, `Deref for L<PathBuf>` | `exists` is inherent on a different instantiation |
| `waiver-ctor-wrong-type` | `impl<T> L<T> { fn new(t: T) -> Self }`, `exists` only on `L<u8>`, `L::new(PathBuf::from(..))` | the constructor builds another instantiation |
| `waiver-same-name-module` | `mod a { struct L; impl L { fn new() .. fn exists(&self) .. } }`, `mod b { struct L(PathBuf); impl Deref for L .. L::new().exists() }` | the name `L` denotes a different type in `b` |
| `waiver-arbitrary-self` | `pub fn exists(self: Rc<Self>) -> bool` with a plain `L` receiver | the receiver type does not match, so lookup auto-derefs to `Path` |
| `og2_private2` | `pub struct L(PathBuf)` with `Deref<Target = Path>` and `pub fn new`, plus `mod imp { impl super::L { fn exists(&self) -> bool { false } } }`; the crate root calls `L::new().exists()` | the inherent `exists` is private to `imp`, so lookup skips it and auto-derefs to `Path` |

*Root cause.* `collect_local_types` (`src/parser.rs`) records constructors
and `self` methods keyed by the **bare type name**, from every `impl` token in
the file: it ignores `cfg` attributes, `macro_rules!` bodies, the impl's
generic arguments (`L<u8>` and `L<PathBuf>` are both `L`), the module the type
lives in, the receiver's type (any first parameter spelled `self` counts,
including `self: Rc<Self>`), and visibility (a private inherent method in
another module, which method lookup skips at the call site, still waives the
call; case `og2_private2`).

*Intended fix.* Refuse the waiver for `Name` whenever any
`impl Deref for <Name..>` or `impl DerefMut for <Name..>` exists in the
package: auto-deref is the only route from a local receiver to a std method,
so without it the waiver is unnecessary for soundness and with it the waiver
is unsafe. The check must also catch `Deref` impls that are not spelled
`impl Deref for Name`:

- a derive macro that generates one (`#[derive(derive_more::Deref)]`,
  `#[derive(Shrinkwrap)]` from `shrinkwraprs`, or any third-party derive);
- an impl generated by a `macro_rules!` transcriber;
- a renamed trait (`use std::ops::Deref as D; impl D for L`), so the trait
  name is resolved through the binding table rather than matched literally;
- an impl through a type alias (`type A = L; impl Deref for A`).

Prefer failing closed over recognising each spelling: refuse the waiver for
`Name` when it carries any `#[derive(..)]` naming something outside `std`'s
built-in derives, or when any `macro_rules!` body or macro invocation in the
package could emit an impl for it. Otherwise require exactly one definition of
`Name` in the package, an inherent impl with no generic arguments that is not
inside a `macro_rules!` body and carries no `cfg`/`cfg_attr` attribute, a
method visible at the call site (`pub`, or declared in the call site's module
or an ancestor of it), and a plain receiver (`self`, `&self`, `&mut self`,
`mut self`). Anything else keeps the method heuristic's verdict.

## 9. Divergence Log (previous revision -> this revision)

| # | Previous behaviour | Now | Why |
|---|---|---|---|
| D1 | `src/capabilities.rs` skipped by `analyze_package` | scanned | skip was a false-negative hole: edited generated file hid anything |
| D2 | `use std::{fs, process};` and any brace group saw only `std` (pure) | full use-tree expansion | false negative |
| D3 | unknown `std`/`core` paths pure | `CAP003` | spec promised deny-by-default |
| D4 | extern ABIs `C system cdecl stdcall win64 sysv64` only | every ABI but `Rust` | `"C-unwind"` etc. escaped |
| D5 | attributes matched by substring | exact identifier match, plus `link`, `link_section`, … | substring both over- and under-matched |
| D6 | `std::time::Duration`, `std::net::IpAddr` flagged | pure | false positives |
| D7 | directories named `target`/`node_modules`/`_build` skipped at any depth | root only | `mod target;` hid code |
| D8 | `#[path]`, `include!`, proc-macros, spliced macro paths unexamined | `CAP003` | cannot analyse |
| D9 | PASS output hid excepted `unsafe` | §6.1 | AF-22 |
| D10 | `\u{..}` char literals and lifetimes mis-lexed by ad-hoc skipper | proper lexer | could swallow following tokens |
| D11 | `rand` category had no Rust rule | `RandomState`, rand crates | taxonomy has `rand` |
| D12 | duplicate JSON keys last-wins; lenient numbers/whitespace | rejected | parser differential |
| D13 | duplicate/unknown banned exceptions silently accepted | rejected | consistency with capability duplicates |
| D14 | `Path::exists(p)`, `std::path::Path::metadata(p)`, `<PathBuf>::is_file(p)`, `let g = Path::canonicalize;` pure (`path` is a pure module; method list applied only after `.`) | `…::Path/PathBuf::<fs method>` is `fs:list`; `std::path::absolute` is `env:read` | M1 false negative |
| D15 | `m!(include, "../x.rs")` with `macro_rules! m { ($mac:ident, $f:expr) => { $mac!($f) } }` evaded `CAP003` | bare include-family names in macro arguments and `$name!(..)` in macro bodies are `CAP003` | M2 false negative |
| D16 | line-oriented `Cargo.toml` reader: top-level inline tables, `bin = [{..}]`, `package = { build = ".." }`, `lib = { proc-macro = true }`, quoted table names, multi-line strings and arrays resetting the section all evaded | tokenizing reader, one path-based rule, fail closed (§3.5.1) | M3 false negative |
| D17 | alias expansion built (entries x targets) before checking its cap (264 KB input -> 2.3 GB abort); candidate generation quadratic in path length; `matching_close` O(n^2) on `#[#[…`; uncapped glob count | de-dup before expansion, caps inside loops, per-path / per-file ceilings, single-pass bracket table (§5) | M4 denial of service |
| D18 | `type F = File; F::create(..)`, `<File>::create(..)`, `type U = UdpSocket; U::bind(..)` escalated within a category | type aliases are bindings; `>::item` is classified as `<self type>::item` (§3.2) | L1 |
| D19 | `core::arch` / `std::arch` wholly pure | `_rdrand*` / `_rdseed*` are `rand:read`; `__cpuid` informational | L2 |
| D20 | `extern "Rust" { … }` not flagged | `foreign_linkage` + `ffi:call` | L3 |
| D21 | `.canonicalize()` on any receiver was `fs:list` (url-parser's `Url::canonicalize` forced a false `fs:list` declaration) | receiver resolution for `Local::ctor(..)[.unwrap()/.expect()/.clone()/?]` with an inherent method of that name (§3.3) | L4 false positive |
| D22 | `macro_rules! ls { ($t:ty, $p:expr) => { <$t>::read_dir($p) } }` and `$($t)*::read_dir` (kleene op or separator between a repetition group and `::`) evaded the `$..::..` rule, which only fired when `$` touched `::` or a repetition group was directly followed by `::` | in a `macro_rules!` body, `CAP003` when a `$` metavariable occurs inside a `<..>` group followed by `::` (group search bounded to 512 tokens, fail closed), and when any `$(..)` repetition, with or without separator / kleene operator, is followed by `::` | N1 / N2 false negatives |
| D23 | `Cargo.toml` `path`/`build` roots (`tests`, `benches`, `examples`, `target`, ..) compared case-sensitively: `[lib] path = "Tests/evil.rs"` passed | compared with `eq_ignore_ascii_case` | L-A: case-insensitive filesystems resolve it to the unscanned `tests/` |
| D24 | (documentation only) `include_str!` / `include_bytes!` undocumented as unflagged | stated in §3.5: they pull in *data*, never compiled code, so a path pointing outside the scan set cannot add authority; the file's bytes are inert unless the program parses and acts on them, which is visible in scanned code. They remain pure macros. `include!` is the code-including form and stays `CAP003` | L-B |
| D25 | a leading `#!/x"` was lexed as `#` `!` `/` `x` and the start of a string, hiding the code after it until the next `"` | rustc's `strip_shebang` rule (§3.1) | HIGH-3 (round-3 review, PoC t1) false negative |
| D26 | whitespace was `char::is_whitespace` (U+200E between path segments split the path, PoC t9); identifiers used `is_alphabetic`/`is_alphanumeric` (U+00B7 ended the alias `s·x` at `s`, PoC t4); a BOM anywhere was whitespace; `extern crate a as b` took `b` and skipped to the next `;` | `Pattern_White_Space` only; non-ASCII outside literals/comments is `CAP003`; BOM only at offset 0; `extern crate` must end at `;` after the alias | HIGH-6 (round-3 review) false negative |
| D27 | `c"`/`cr#"` lexed as C strings in every edition; in a 2018 crate `cr#" a " { std::fs::write(..) } " "#` hid a real block (PoC t8) | edition read from `Cargo.toml`; C strings only from 2021; unknown edition makes `cr"`/`cr#"` `CAP003` (§3.1) | HIGH-7 (round-3 review) false negative |
| D28 | `include!` matched only at path length 1 or 2; `std::prelude::v1::include!(..)` (the prelude re-export; `prelude` is pure) passed (PoC t3) | any std/core/alloc path ending in `include` is `CAP003`, also in a `use` (§3.5) | HIGH-4 (round-3 review) false negative |
| D29 | glob spellings were `glob + <whole path>` only, and their `CAP003` was always suppressed: `mod m { pub use std::*; } m::process::Command::new(..)` passed (PoC t12) and `mod m { pub use std::fs::*; } m::remove_dir_all(..)` was only `fs:read` (PoC t13); glob heads were normalised at the first segment only; glob normalisation counted only kept globs, after materialising | `glob + suffix` for every suffix of every spelling; globs normalised at every segment; `use std::*` (and `core`/`alloc`, also via alias) is `CAP003` and does not suppress glob `CAP003`; 4,096-spelling cap counting pure globs (§3.2, §5) | HIGH-1 false negative, LOW denial of service (round-3 review) |
| D30 | the receiver waiver ignored glob imports: a local `PathBuf` with an inherent `read_dir` in one module plus `use std::path::*;` in another made std's `PathBuf::from(..).read_dir()` pure (PoC t11) | waiver refused when any glob reaches outside the crate (§3.3 L4) | MEDIUM-1 (round-3 review) false negative |
| D31 | splice rules fired only when `$` touched `::`, `<..>::` or `#[`; `($s:tt) => { std $s fs $s write(..) }` with `m!(::)` (PoC t2) and `($a:tt) => { # $a pub mod hidden; }` with `m!([path = ".."])` (PoC t6) passed; matchers were checked as if they were code | arms are parsed; in transcribers a `#` not starting `#[`/`#![` and a `tt` metavariable next to an identifier, another metavariable, `[` or `!` are `CAP003`; matchers are not checked; an unparseable body is `CAP003` (§3.5) | HIGH-2 (round-3 review) false negative |
| D32 | root-level files could `mod` into the root directories the scanner skips: `build.rs` `mod examples;` (PoC t5), `[lib] path = "lib.rs"` + `mod tests;` or `mod tests { mod x; }` (PoC t10) | such `mod` declarations in root-level files, `mod` handed to a macro there, and `mod $name` in transcribers are `CAP003` (§5) | HIGH-5 (round-3 review) false negative |
| D33 | `std::fs::File::create_buffered` was `fs:read` (PoC t7) | `fs:write` (§3.3) | INFO (round-3 review) |
| D34 | bindings were expanded only through their *first* segment and written paths substituted once, so a rename of a rename through a module was never followed: `mod a { pub use std as s; } use a::s as t; t::env::var(..)` was `a::s::env::var` (pure), likewise `use super::i as j` with `use std::io as i`, `use crate::Q as P`, `mod m { pub extern crate std as s2; } use m::s2 as s3;`, `use j::*` over such a rename, and `t::include!` (review cases `alias2-*`, `extern-crate-nested-2level`); G-8 wrongly said same-named bindings were followed | bindings closed under substitution at every segment to a fixed point, with per-spelling provenance so no import is used twice; caps 64 segments, 20,000 spellings, 4,000,000 attempts, 16 rounds, each `CAP003` (§3.2, §5); a converging alias cycle (`use b::x as a; use c::x as b; use a::x as c;`, rejected by rustc) is no longer `CAP003` | H1 (second review of round 3) false negative |
| D35 | `std::time::UNIX_EPOCH` was pure and `.elapsed()` unclassified, so `std::time::UNIX_EPOCH.elapsed()` read the clock (case `unix-epoch-elapsed`); `rx.recv_timeout(..)` and `c.wait_timeout(..)` on the pure `std::sync` types slept on the clock (case `sync-timed-wait`) | `UNIX_EPOCH` is `time:read`; `.elapsed(` is `time:read`; `.recv_timeout(` `.recv_deadline(` `.wait_timeout(` `.wait_timeout_ms(` `.wait_timeout_while(` and their `Condvar::`/`Receiver::` UFCS forms are `time:sleep` (§3.3) | H6 (second review of round 3) false negative |
| D36 | `file.set_times(..)` (also `set_modified`, `set_permissions`) on a handle from `File::open` wrote metadata under an `fs:read`-only manifest (case `handle-method-escalation`) | those methods are `fs:write`, as are `std::fs::File::{set_times, set_modified, set_permissions}` (§3.3) | MEDIUM (second review of round 3) false negative |
| D37 | `Self` was a local head and never resolved: `impl Probe for std::path::Path { fn probe(&self) -> bool { Self::exists(self) } }` and `<Self>::is_file(self)` were pure (cases `self-impl-path`, `qualified-self-angle`) | `Self` resolves to the self type of every enclosing `impl` region; an unknown self type (metavariable, projection, `macro_rules!` body) gives `Self::name` the method-call verdict (§3.2) | H2 (second review of round 3) false negative |
| D38 | `type` aliases bound only the first path after `=` and silently bound nothing for any other shape: `type $n = std::path::Path;` in a transcriber, `type P = (std::path::Path);`, `type P = <std::path::PathBuf as std::ops::Deref>::Target;` and `type P = $t;` all left `P::exists(..)` pure (cases `type-ident-name`, `type-paren`, `type-qualified`) | every right-hand-side path is bound; a right-hand-side metavariable binds an unknown type (method-call verdict); a metavariable name or an undelimitable item is `CAP003`; matchers are skipped (§3.2) | H3 (second review of round 3) false negative |
| D39 | an `ident` metavariable could supply the keyword of an item the transcriber spells: `($k:ident) => { $k std::io as out; }` with `kw!(use)` and `$k crate std as s;` with `kw!(extern)` bound `out`/`s` unseen (cases `ident-keyword-use`, `ident-keyword-extern-narrow`; with an `ffi:call` manifest the second passed); `$k {std::env as out};` (also with `$k:tt`) and `use $i as out;` likewise | an `ident`/`tt` metavariable followed by a path-capable identifier, a string, a metavariable, a possible use group, or `<` at item start is `CAP003`; `use $x` (not `$crate`) is `CAP003` (§3.5) | H4 (second review of round 3) false negative |
| D40 | `($e:expr, $m:ident) => { $e.$m() }` with `call!(Path::new(".."), exists)` called `Path::exists` and was pure (case `ident-method-splice`) | `.` followed by a repetition, or a metavariable then `(` / `::`, in a transcriber is `CAP003` (§3.5) | H7 (second review of round 3) false negative |
| D41 | `Cargo.toml` target paths and `build` were checked only for escaping the scanned set, while the scanner reads only `.rs` files: `[lib] path = "src/lib.txt"`, `[[bin]] path = "src/tool.inc"` and `build = "build.txt"` compiled unscanned code (cases `cargo-lib-nonrs`, `cargo-bin-nonrs`, `cargo-build-nonrs`), and the certificate listed the decoy `src/lib.rs` as `proved` | a target `path` or `build` not ending in `.rs` is `CAP003` (§3.5); certificates refuse a `[lib] path` other than `src/lib.rs` (VER23a §3.3, V12) | H5 (second review of round 3) false negative |
| D42 | the `$($t)*` pass-through test rescanned the whole matcher for every transcriber repetition: a 40,000-token matcher with 40,000 repetitions took about 30 s in a debug build (cases `perf-matcher-rescan-*`, `perf-valid-40000`) | the matcher's `$( $x:tt ) sep? op` repetitions are collected once per arm and looked up by name; verdicts unchanged (§5) | LOW (second review of round 3) denial of service |
| D43 | the method heuristic tested only the token right after the name, so `Path::new("Cargo.toml").exists::<>()` and `c.wait_timeout::<>(g, d)` (legal for a method with no generic parameters) were pure (cases `turbofish-exists`, `turbofish-wait-timeout`) | an optional turbofish `::<..>` after the method name is skipped before testing for `(`; a `::<` that does not close is treated as a call (§3.3) | R5-1 (third review) false negative |
| D44 | a macro invocation in type position was read as a path named after the macro: `<ty!()>::exists(p)` was `ty::exists`, `impl P for ty!() { .. Self::exists(self) .. }` resolved `Self` to `ty`, and `type F = ty!();` bound `F -> ty`, all pure although `ty!()` expanded to `std::path::Path` (cases `type-macro-qualified`, `type-macro-impl-self`, `type-macro-alias`, `type-macro-condvar`); and `<(std::path::Path)>::exists(p)` named no self type (found while fixing) | a qualified self type, an impl self type or a type-alias right-hand side that is a macro invocation (`name !`) is an *unknown type*, which gets the method-call verdict of `.name(`, as a metavariable already did; a parenthesised qualified self type is read through the parentheses (§3.2) | R5-2 (third review) false negative |
| D45 | `std::thread::park_timeout_ms(10)` (the deprecated `u32` form of `park_timeout`) was `thread:spawn`, so a `thread:spawn`-only manifest slept unseen (case `park-timeout-ms`) | `std::thread::park_timeout_ms` is `time:sleep` (§3.3) | R5-3 (third review) false negative |
| D46 | `use std::net::*; ("127.0.0.1", 80).to_socket_addrs()` resolved a name under a manifest declaring only `net:connect`: the glob is `net:connect` and the trait `ToSocketAddrs` (`net:dns`) is never named (case `net-glob-to-socket-addrs`) | `.to_socket_addrs(` is `net:dns` through the method heuristic, on any receiver (§3.3) | R5-3 (third review) false negative |
| D47 | `std::io` was pure apart from the standard streams, so `std::io::pipe()` created an OS pipe (two file descriptors) under an empty manifest (case `io-pipe`) | `std::io::{pipe, PipeReader, PipeWriter}` are `ffi:call`, the row for raw OS handles (`std::os::fd`, ...): the Spec 13 taxonomy has no IPC category, and `CAP003` would be wrong because the construct is seen exactly and `CAP003` is never waivable, so no package could declare a pipe (§3.3) | R5-3 (third review) false negative |
| D48 | every `std::os::…::net` item was `net:connect`, so `std::os::unix::net::UnixListener::bind` (listens, and creates a socket file) passed under a `net:connect`-only manifest (case `unix-listener-class`) | `std::os::…::net::UnixListener` and `UnixDatagram::{bind, bind_addr}` are `net:listen`, matching `std::net::TcpListener` and `UdpSocket::bind`; `UnixStream`, `UnixDatagram::unbound` and the address type stay `net:connect` (§3.3) | R5-3 (third review) false negative |
| D49 | associated-type projections were read as paths: `<std::path::PathBuf as std::ops::Deref>::Target::exists(p)`, `D::Target::exists(d)` with `D: Deref<Target = Path>`, `Self::Target::exists(self)` under a `Deref<Target = Path>` supertrait, `<Self as Deref>::Target::exists` in a blanket impl and `<Arc<Condvar> as Deref>::Target::wait_timeout` were pure; and the unknown-`Self` / unknown-alias checks looked at the second segment (`Target`) rather than the method (cases `proj-deref-target`, `proj-generic-bound`, `proj-self-target-trait`, `proj-blanket-impl`, `proj-condvar-wait`) | `<T as Tr>::A::item`, `Self::A::item` and `P::A::item` (`P` a generic parameter of the file) are unknown types; an unknown type's item gets the method-call verdict of the **last** segment (§3.2) | R5-4 (third review, G-A) false negative |
| D50 | D43 walked the turbofish to find the `(` with a bracket-blind `<`/`>` counter, so a `>` inside brackets closed it early: `c.wait_timeout::<[u8; (2 > 1) as usize]>(g, d)` read as a field and slept unseen (case `tf_paren_gt`) | `.name::<` is a method call whatever follows the turbofish: a field takes no generic arguments, so the turbofish is not walked (§3.3) | F4 (follow-up review of the third-review fixes) false negative |
| D51 | a generic parameter name was recognised only right after `<` or `,`, so an attribute hid it: in `fn probe<#[allow(unused)] D: Deref<Target = Path>>` the token before `D` is `]`, `D` was not a parameter, and `D::Target::exists(d)` read as a local path (case `proj_attr_param`) | an identifier after the `]` closing an attribute `#[..]` also starts a parameter (§3.2) | F2 (follow-up review of the third-review fixes) false negative |
| D52 | a type alias whose right-hand side was a projection bound only the paths in it: `type X<D> = <D as std::ops::Deref>::Target;` bound `X` to `Deref` and `Target` (`D`, the alias's own parameter, is skipped), and `type X<D: Deref> = D::Target;` to `D::Target`, so `X::<std::path::PathBuf>::exists(p)` (`Path::exists`) was pure (cases `proj_alias_generic`, `proj_alias_short`). Found while fixing: the alias's parameter reader took names inside bounds (`<D: AsRef<File>>` declared `File`) and missed attributed ones, and both parameter readers ended the list at a `>` inside a bracketed default (`<T = [u8; (2 > 1) as usize], D: ..>` hid `D`) | an alias whose right-hand side holds a qualified group `<..>::` or a path headed by one of its own parameters is also bound to the unknown type; parameter lists are read bracket-aware, top-level names only (§3.2). D49's claim that its three shapes "cover every projection path" is corrected: they cover every way to spell one at its use | F3 (follow-up review of the third-review fixes) false negative |
| D53 | the walk back from a turbofish `::<..>::` collected the segments before `::<` and stopped at a `>`, so a turbofish on the associated-type segment cut the path off from its qualified group: `<std::path::PathBuf as std::ops::Deref>::Target::<>::exists(p)` read as `Target::exists` and `<D>::Target::<>::exists(d)` as `Target::exists`, both pure (cases `proj_qual_tf`, `proj_angle_tf`) | the walk continues through a `>::` before the turbofish path into that group and prepends what it finds, carrying the innermost qualified group's `as` and the number of segments after it, so the D49 projection rule applies; at most 8 groups per path, more is `CAP003` (§3.2) | F1 (follow-up review of the third-review fixes) false negative |
