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

Comments (line, nested block, doc), string literals (plain, raw `r#"…"#`, byte, raw byte, C-string) and character/byte literals never produce detections. Lifetimes (`'a`) are distinguished from character literals (`'a'`, `'\n'`, `'\u{1F600}'`). Raw identifiers (`r#fs`) denote the same identifier as `fs`; `r#unsafe` is an ordinary identifier, a non-raw `unsafe` token is always a banned construct. An unterminated comment, string or raw string is a parse error (exit `2`).

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
| `extern crate a as b;` | binding `b -> a` |

Binding heads are expanded through other bindings to a fixed point (bounded; a cycle or growth past 64 segments is `CAP003`). For every path occurrence (in `use` leaves and everywhere else) the analyzer classifies the path **and** every *candidate*: for each path segment equal to a binding name, `binding_target + remaining_segments`; and, for a path not headed `std`/`core`/`alloc`/`crate`/`self`/`super`/`Self`, `glob_prefix + path` for each glob. Glob candidates may add `CAP001`/`CAP002` results, never `CAP003`. Pinned by tests:

- `use std::{fs, process::Command};`, `use std as s; s::process::Command`, `use std::fs as f; f::read(..)`, `use std::*; process::Command::new(..)`, and `a::s::process::Command` (another file has `pub use std as s;`) resolve to `std::` paths and are detected.
- Re-exporting a capability (`pub use std::process as p;`) is itself a detection at the re-export site.
- Bindings are package-wide, not per scope: an unrelated local item sharing a binding's name may yield a false positive. Accepted.

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
| any path containing segment `RandomState`; any path headed `rand getrandom fastrand oorandom rand_core rand_chacha` | `rand:read` |
| paths headed `libc nix winapi windows windows_sys libloading` | `ffi:call` |
| paths headed `tokio mio socket2 reqwest hyper ureq async_std smol` | `net:connect` |
| *unresolved-head shorthand*: a path headed `fs net process env thread time ffi io os` that no binding explains is treated as `std::<same path>` | as above |
| pure std modules/macros (below) | pure |

Method heuristic: `.name(` with `name` in {`read_dir metadata symlink_metadata canonicalize read_link exists try_exists is_file is_dir is_symlink`} is `fs:list`, because `Path::exists()` reaches the filesystem without naming `std::fs`. This over-approximates (a user method called `exists` is flagged).

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

An exception waives the construct for the whole package, not per item. Waived constructs are **still reported** (§6.1).

### 3.5 Unanalysable constructs (`CAP003`, never waivable, exit `1`)

| Construct | Why |
|---|---|
| `include!(…)` (bare, `std::`, `core::`) | pulls in code the scanner never sees (`include_str!`/`include_bytes!` pull in data and are allowed) |
| `#[path = "…"]` (also inside `cfg_attr`) | module location is untracked; may point at an unscanned directory |
| in a `macro_rules!` body, a metavariable adjacent to `::` (`std::$m::read`, `$p::x`, `$(…)::*`) | the expansion constructs a path the scanner never sees |
| `#[proc_macro]`, `#[proc_macro_derive]`, `#[proc_macro_attribute]`, or `proc-macro = true` in `Cargo.toml` | compile-time code execution with full authority |
| unclassified `std`/`core`/`alloc` path (§3.3) | deny by default |
| alias cycle / unbounded expansion (§3.2) | resolution did not converge |
| `Cargo.toml` `path`/`build` keys (`[lib]`, `[[bin]]`, `[[example]]`, `[[test]]`, `[[bench]]`, `[package]`) that are absolute, contain `..`, start with a hidden component, or start with an unscanned directory (`tests benches examples target node_modules _build`) | compiled code outside the scanned set |

### 3.6 Third-party dependencies

Names under `[dependencies]`, `[build-dependencies]` and `[target.*.dependencies]` in `Cargo.toml` are listed as `unanalyzed_dependencies` in the report and printed as `WARN` lines. They do not change the exit code. A package with unanalyzed dependencies must not be presented as "no unsafe / no FFI / no process" on this analyzer's authority alone.

---

## 4. Deny-by-Default Name Allowlist

The pure module/macro lists of §3.3 are the allowlist for `std`/`core`/`alloc`. Local items and unlisted external crates are pure by default (G-1).

---

## 5. Resource Ceilings and Scanner Boundaries

- Maximum source files **10,000**; per file **4 MiB**; cumulative **64 MiB**; directory depth **64**; manifest **1 MiB**.
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
- **G-8** Type-directed routes beyond the §3.3 method heuristic.
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
