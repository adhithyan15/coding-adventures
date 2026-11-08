# VER08a — Rust Capability Analyzer and Deny-by-Default Name Allowlist

**Parent:** [VER00 §8](VER00-verified-shipping-program.md), [VER08](VER08-capability-analyzers.md), [VER23](VER23-generated-capability-modules.md), [VER26 §5 Step 27](VER26-ct-compare-pilot.md).

This specification formalizes **VER-P027a**: the zero-third-party-dependency static capability analyzer and deny-by-default name allowlist for Rust packages in the repository.

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

The publication gate (VER00 §9) mandates that no package may ship without an audited capability profile (`required_capabilities.json`). The Rust capability analyzer provides the automated, fail-closed enforcement layer (Spec 13 Layer 3 / 4) ensuring that:

1. **Deny-by-Default Ambient Authority:**
   Packages declaring an empty capability profile (`capabilities: []`) or lacking a manifest are treated as pure computation. Any ambient access to filesystem, network, processes, environment, system time, threads, standard I/O streams, or foreign function interfaces triggers a `CAP001` violation.

2. **Banned Construct Enforcement:**
   Constructs that bypass type safety or static analysis—notably `unsafe` blocks, `unsafe fn`, inline assembly (`asm!`, `global_asm!`), and unvetted FFI declarations (`extern "C"`)—trigger a `CAP002` violation unless granted an explicit, hardware-key-signed exception in `banned_construct_exceptions`.

3. **Fail-Closed Operation:**
   Any malformed JSON manifest, schema divergence, source file read error, unparseable token stream, or resource exhaustion fails closed with exit code `2`.

4. **Zero Third-Party Dependencies:**
   Implemented exclusively with the Rust standard library (`std`), requiring zero external crates (no `syn`, no `serde`, no `regex`, no `libc`).

---

## 2. Manifest Contract (`required_capabilities.json`)

The analyzer reads `required_capabilities.json` exclusively from the package root directory:

- **Absent Manifest:** Defaults to pure computation (`capabilities = []`, `banned_construct_exceptions = []`).
- **Present Manifest:** Must conform to `required_capabilities.schema.json` (version 1):
  - Valid `package` field matching `rust/<crate_name>`;
  - Valid `capabilities` list using the closed 11-category taxonomy (`fs`, `net`, `proc`, `env`, `ffi`, `time`, `stdin`, `stdout`, `rand`, `thread`, `ui`);
  - Valid `banned_construct_exceptions` array.
- **Fail-Closed Manifest Validation:**
  - Unknown fields fail closed;
  - Invalid category/action combinations fail closed;
  - Missing justifications (< 10 characters) fail closed;
  - Duplicate capability declarations fail closed.

---

## 3. Rust Detection & Mapping Table

The analyzer inspects Rust source tokens and AST paths, identifying the following OS capability triggers:

| Rust Standard Library Surface | Category | Action | Default Target |
|---|---|---|---|
| `std::fs::read`, `read_to_string`, `File::open` | `fs` | `read` | `*` |
| `std::fs::write`, `File::create`, `OpenOptions` | `fs` | `write` | `*` |
| `std::fs::create_dir`, `create_dir_all` | `fs` | `create` | `*` |
| `std::fs::remove_file`, `remove_dir`, `remove_dir_all` | `fs` | `delete` | `*` |
| `std::fs::read_dir`, `metadata`, `symlink_metadata` | `fs` | `list` | `*` |
| `std::net::TcpStream::connect`, `UdpSocket::connect`, `ToSocketAddrs` | `net` | `connect` | `*` |
| `std::net::TcpListener::bind`, `UdpSocket::bind` | `net` | `listen` | `*` |
| `std::process::Command::new`, `Command::spawn`, `Command::status` | `proc` | `exec` | `*` |
| `std::process::exit`, `std::process::abort` | `proc` | `signal` | `*` |
| `std::env::var`, `var_os`, `vars`, `vars_os`, `args`, `args_os` | `env` | `read` | `*` |
| `std::env::set_var`, `remove_var`, `set_current_dir` | `env` | `write` | `*` |
| `std::time::Instant::now`, `SystemTime::now` | `time` | `read` | `*` |
| `std::thread::spawn`, `Builder::spawn` | `thread` | `spawn` | `*` |
| `std::io::stdin` | `stdin` | `read` | `*` |
| `std::io::stdout`, `std::io::stderr`, `print!`, `println!`, `eprint!`, `eprintln!` | `stdout` | `write` | `*` |
| `extern "C"`, `extern "system"`, `std::ffi`, `core::ffi` | `ffi` | `call` | `*` |

### 3.1 Banned Constructs

The following constructs evade static reachability and are banned by default:

1. **`unsafe` Blocks and Functions (`unsafe { ... }`, `unsafe fn`):**
   Banned under `construct = "unsafe_code"`.
2. **Inline Assembly (`core::arch::asm!`, `global_asm!`):**
   Banned under `construct = "inline_assembly"`.
3. **External Linkage Directives (`#[no_mangle]`, `#[link_name]`):**
   Banned under `construct = "foreign_linkage"`.

If an exception is declared in `banned_construct_exceptions` with a matching `construct`, `language = "rust"`, and justification ($\ge 10$ chars), the construct is permitted.

---

## 4. Deny-by-Default Name Allowlist for Pure Computation

For crates declaring pure computation (`rust/ct-compare`, math libraries, parsers, codecs), ambient access is denied. The pure computation core allowlist permits:

- `core::*` (arithmetic, boolean logic, slices, formatting, options, results, hints, iterators);
- `core::hint::black_box`;
- `std::vec::*`, `std::string::*`, `std::collections::*` (in-memory data structures);
- `std::cmp::*`, `std::ops::*`, `std::convert::*`, `std::mem::*`;
- Standard macros: `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!`, `matches!`, `vec!`, `format!`, `panic!`, `unreachable!`, `todo!`.

Any reference to unvetted paths outside the allowlist triggers a check against the capability table.

---

## 5. Resource Ceilings and Scanner Boundaries

To guard against malicious or pathological source trees, the scanner enforces hard limits:

- Maximum source files: **10,000**;
- Maximum size per source file: **4 MiB**;
- Maximum cumulative source size: **64 MiB**;
- Maximum directory traversal depth: **64**;
- Maximum `required_capabilities.json` size: **1 MiB**;
- Refusal of symlinks: Source files that are symlinks are rejected to avoid directory escaping;
- Excluded paths: `target/`, `.git/`, `tests/` (when analyzing crate library root).

---

## 6. Exit Behavior and Diagnostic Format

The CLI tool executes as:

```bash
ca-capability-analyzer-rs --dir <PACKAGE_DIR> [--verbose]
```

Exit codes:
- **`0`**: Clean. Every detected capability is declared; every banned construct is excepted.
- **`1`**: Violations found. Undeclared capability (`CAP001`) or unexempt banned construct (`CAP002`).
- **`2`**: Tool error. Invalid CLI arguments, malformed manifest JSON, source I/O failure, or parse failure.

Violation diagnostic format:
```text
CAP001: Undeclared capability 'fs:read:*' detected at src/lib.rs:42:15 ('std::fs::read')
CAP002: Banned construct 'unsafe_code' detected at src/barrier.rs:104:9
```
