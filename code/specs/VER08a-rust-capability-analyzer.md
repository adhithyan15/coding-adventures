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

A leading byte-order mark is dropped, then a shebang line exactly as rustc's `strip_shebang` decides: a file starting `#!` has its first line skipped unless, after whitespace and *non-doc* comments, the next token is `[` (an inner attribute such as `#![no_std]`). A doc comment is a token, so `#! //! x` is a shebang line. Whitespace is exactly Unicode `Pattern_White_Space` (`\t \n \v \f \r` space, U+0085, U+200E, U+200F, U+2028, U+2029), the set rustc uses; `char::is_whitespace` is a different set. Identifiers are ASCII (`[A-Za-z_][A-Za-z0-9_]*`, `r#` raw form). Any other non-ASCII character outside a string, character literal or comment (a non-ASCII identifier, a non-`Pattern_White_Space` space such as U+00A0, a byte-order mark after offset 0) is `CAP003`: rustc's XID identifier rules and NFC comparison need Unicode tables this zero-dependency crate does not carry, so the analyzer cannot tell what such a name denotes. Comments (line, nested block, doc), string literals (plain, raw `r#"…"#`, byte, raw byte, C-string) and character/byte literals never produce detections. Lifetimes (`'a`) are distinguished from character literals (`'a'`, `'\n'`, `'\u{1F600}'`). Raw identifiers (`r#fs`) denote the same identifier as `fs`; `r#unsafe` is an ordinary identifier, a non-raw `unsafe` token is always a banned construct. An unterminated comment, string or raw string is a parse error (exit `2`).

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
| `type F = a::b;` / `type F<T> = &a::b<T>;` | binding `F -> a::b` (the first path after `=`, skipping `& * mut dyn impl const` and lifetimes) |

Binding heads are expanded through other bindings to a fixed point (bounded; a cycle or growth past 64 segments is `CAP003`). Identical bindings are de-duplicated *before* expansion and every cap is checked *inside* the expansion loop, so no intermediate result can exceed it (§5, M4). For every path occurrence (in `use` leaves and everywhere else) the analyzer classifies the path **and** every *candidate*: for each path segment equal to a binding name, `binding_target + remaining_segments`; and, for a path not headed `std`/`core`/`alloc`/`crate`/`self`/`super`/`Self`, `glob_prefix + path` for each glob. Glob candidates may add `CAP001`/`CAP002` results, never `CAP003`. Pinned by tests:

- `use std::{fs, process::Command};`, `use std as s; s::process::Command`, `use std::fs as f; f::read(..)`, `use std::*; process::Command::new(..)`, and `a::s::process::Command` (another file has `pub use std as s;`) resolve to `std::` paths and are detected.
- Re-exporting a capability (`pub use std::process as p;`) is itself a detection at the re-export site.
- Bindings are package-wide, not per scope: an unrelated local item sharing a binding's name may yield a false positive. Accepted.
- Qualified paths name the self type: after `>::` the item is read as `<self type path>::item`, so `<File>::create(..)`, `<std::fs::File as T>::create(..)` and the turbofish `Path::<X>::exists` classify as `File::create`, `std::fs::File::create` and `Path::exists`. A `<` that cannot be matched within 512 tokens is `CAP003`; a self type with no path (`<[u8]>::len`) is pure.
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
| `std::fs::{write, copy, rename, hard_link, soft_link, set_permissions, OpenOptions}`, `std::fs::File::{create, create_new, options}` | `fs:write` |
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
| `std::thread::{sleep, sleep_ms, park_timeout}` | `time:sleep` |
| any other `std::thread` item (`spawn`, `scope`, `Builder`, `current`, bare module) | `thread:spawn` |
| `std::time::{Duration, TryFromFloatSecsError, UNIX_EPOCH}` | pure |
| any other `std::time` item (`Instant`, `SystemTime`, bare module) | `time:read` |
| `std::io::{stdin, Stdin, StdinLock}` | `stdin:read` |
| `std::io::{stdout, stderr, Stdout, Stderr, StdoutLock, StderrLock, _print, _eprint}`; macros `print println eprint eprintln dbg` (bare or `std::`-qualified) | `stdout:write` |
| other `std::io` items (`Read`, `Write`, `Cursor`, `Error`, `copy`, …) | pure |
| `std::ffi::*`, `core::ffi::*`, `alloc::ffi::*` | `ffi:call` |
| `std::os::…` containing segment `net` / `process` / `fs` / `thread` | `net:connect` / `proc:exec` / `fs:write` / `thread:spawn` |
| any other `std::os::*` (raw fds/handles) | `ffi:call` |
| `{core,std}::arch::*` item whose name starts `_rdrand` / `_rdseed` (hardware randomness) | `rand:read` |
| `std::path::absolute` (joins the current directory) | `env:read` |
| `…::Path::{read_dir metadata symlink_metadata canonicalize read_link exists try_exists is_file is_dir is_symlink}` and the same on `PathBuf`, however qualified (`Path::exists(p)`, `std::path::Path::metadata(p)`, `<PathBuf>::is_file(p)`, `let g = Path::canonicalize;`) | `fs:list` |
| any path containing segment `RandomState`; any path headed `rand getrandom fastrand oorandom rand_core rand_chacha` | `rand:read` |
| paths headed `libc nix winapi windows windows_sys libloading` | `ffi:call` |
| paths headed `tokio mio socket2 reqwest hyper ureq async_std smol` | `net:connect` |
| *unresolved-head shorthand*: a path headed `fs net process env thread time ffi io os` that no binding explains is treated as `std::<same path>` | as above |
| pure std modules/macros (below) | pure |

Method heuristic: `.name(` with `name` in {`read_dir metadata symlink_metadata canonicalize read_link exists try_exists is_file is_dir is_symlink`} is `fs:list`, because `Path::exists()` reaches the filesystem without naming `std::fs`. This over-approximates (a user method called `exists` is flagged).

*Receiver resolution (L4).* The heuristic is skipped for exactly one provable shape: the receiver is `Local::ctor(..)` optionally followed by `.unwrap()`, `.expect(..)`, `.clone()` or `?`, where `Local` is a struct/enum of the package whose own inherent `impl Local { .. }` declares both `ctor` (an associated fn returning `Self`, `Local`, `Result<Self|Local, _>` or `Option<Self|Local>`) and a `self` method of the called name, and `Local` is not rebound by a `use`/`type`. Inherent methods win method resolution, so such a call cannot reach `Path::<name>`. Anything else - a variable, a field, another adapter, a trait method - stays flagged. Example: `Url::parse(s).unwrap().canonicalize()` in url-parser is the crate's own `Url::canonicalize`, not a filesystem call.

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
| `include!(…)` (bare, `std::`, `core::`) | pulls in code the scanner never sees (`include_str!`/`include_bytes!` pull in data and are allowed) |
| in a `macro_rules!` body, a `$` metavariable inside a `<..>` group followed by `::` (`<$t>::read_dir`), or a `$(..)` repetition (any separator / kleene operator) followed by `::` (`$($t)*::x`) | the caller chooses the path root, so `Path`/`File` UFCS is hidden (N1, N2). Fails closed; no method list. Exception: the item after `::` is a SCREAMING_CASE associated constant (`<$t>::MAX`), which cannot perform I/O (segment-tree) |
| `include_str!(…)` / `include_bytes!(…)` | deliberately **not** flagged: they embed *data*, never compiled code, so a path outside the scan set adds no authority; acting on the bytes (parsing, executing) is visible in scanned code (L-B) |
| `#[path = "…"]` (also inside `cfg_attr`) | module location is untracked; may point at an unscanned directory |
| in a `macro_rules!` body, a metavariable adjacent to `::` (`std::$m::read`, `$p::x`, `$(…)::*`) | the expansion constructs a path the scanner never sees |
| in a `macro_rules!` body, an attribute whose name is a metavariable (`#[$a]`) or a `cfg_attr` containing one | `m!(no_mangle)` would create `#[no_mangle]` unseen. This also flags the common `$(#[$meta:meta])*` forwarding idiom (accepted false positive) |
| `#[proc_macro]`, `#[proc_macro_derive]`, `#[proc_macro_attribute]`, or `proc-macro = true` in `Cargo.toml` | compile-time code execution with full authority |
| unclassified `std`/`core`/`alloc` path (§3.3) | deny by default |
| alias cycle / unbounded expansion (§3.2) | resolution did not converge |
| `Cargo.toml` key `path`, `build`, `proc-macro` or `proc_macro` **anywhere outside** the dependency tables (§3.5.1) whose value is not verifiable; for `path`/`build`: absolute, contains `..`, starts with a hidden component, or starts with an unscanned directory (`tests benches examples target node_modules _build`) | shipped code outside the scanned set. (`[[test]]`/`[[bench]]`/`[[example]]` `path`s are not shipped and not examined, G-5.) |
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
- Work ceilings (M4, each exceeded -> `CAP003`, never a hang or an allocation abort): 20,000 distinct use bindings / alias entries; 256 distinct glob imports; 64 segments per path; 64 spellings per path; 200,000 extra spellings per file; `macro_rules!` body scanning bounded to 4x the token count per file; `Cargo.toml` value nesting 32. Bracket matching is a single pass per file (`#[#[#[…` is linear).
- Symlinks (file or directory) inside the package are refused (exit `2`).
- Every `.rs` file under the package root is scanned, **including** `src/capabilities.rs` (previously exempt; a hand-edited generated module is scanned like any file) and a root `build.rs`.
- Excluded: hidden directories (unreachable as modules without `#[path]`, which is `CAP003`); at the package **root only** `target/ node_modules/ _build/ tests/ benches/ examples/`. A nested directory named `target`, `tests`, … (e.g. `src/target/mod.rs`) **is** scanned.
- `tests/`, `benches/`, `examples/` are not shipped and not analysed (gap G-5).

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
- **G-6** Macros are not expanded: authority produced only by expansion of *another crate's* macros (`#[derive(ThirdParty)]`, `thirdparty::mac!()`) is invisible. Metavariable-spliced paths in local `macro_rules!` are `CAP003`; method names spliced via `.$m(` are not detected.
- **G-7** Dynamic behaviour (function pointers, trait objects, `dlopen` through any crate, `/proc/self/mem` through a declared fs capability).
- **G-8** Type-directed routes beyond the §3.3 method heuristic and the type-alias / qualified-path / receiver rules of §3.2-3.3. A binding whose target begins `crate::`/`super::`/`self::` (`type F = super::File;`, `use super::File as G;`) is not followed into the module it names; only same-named bindings elsewhere in the package are.
- **G-9** `cfg` is ignored (conservative); compiler flags, `.cargo/config.toml` and `RUSTFLAGS` are not examined.

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
