# Capability Cage (Rust)

## Overview

This spec defines the Rust implementation of the Capability Cage. The Go
implementation in `code/packages/go/capability-cage/` already establishes
the model — manifests as compile-time declarations, secure wrappers as the
only sanctioned path to OS resources, a swappable `Backend` for testing and
for routing through the host process. This document describes the Rust
port, which preserves those semantics while adapting them to Rust idioms
(enums for categories and actions, traits for backends, `Result` for
errors, build-time code generation for the per-package capability table).

The cage operates at three enforcement levels — lint-time, runtime, and
hard-cage (Tier 2/3 agents). For the Rust port, all three levels are in
scope, but the lint-time work is delegated to a separate `cargo` lint
that consumes the same manifest format. Runtime enforcement and the
secure wrapper API are the focus of this spec.

The Rust port is **not** an experiment in alternative semantics. Where
the Go and Rust libraries differ in behavior, the Rust library has a
bug. The conformance suite (defined below) checks that both libraries
agree on every published test case.

---

## Where It Fits

```
   Application code (any Rust package in this monorepo)
        │
        │  uses SecureFile, SecureNet, SecureProc, ...
        ▼
   Capability Cage (Rust)  ← THIS SPEC
   ┌────────────────────────────────────────────────────────┐
   │  Manifest (immutable list of Capability)                │
   │     loaded from required_capabilities.json              │
   │  Operation<T> wrapper                                    │
   │     audit envelope around every secure call             │
   │  Secure wrappers per category                            │
   │     SecureFile, SecureNet, SecureProc, SecureEnv,        │
   │     SecureTime, SecureStdio                              │
   │  Backend trait                                           │
   │     OpenBackend (calls stdlib) | TestBackend (mock) |    │
   │     HostRpcBackend (routes to host.*)                    │
   └────────────────────────────────────────────────────────┘
        │
        ▼
   Default: stdlib (OpenBackend)
   Hard cage: HostRpcBackend → secure-host-channel → host process
```

**Depends on:**
- `json-parser`, `json-value` — manifest parsing.
- `glob` (or hand-rolled matcher) — target glob matching.
- `time` — monotonic clock for audit timestamps.

**Used by:**
- Every Rust package in the monorepo for OS access.
- The `host-runtime-rust` crate, which injects a `HostRpcBackend` so the
  cage routes through the host channel instead of the OS.
- The CI lint that walks the source tree and verifies wrappers are used.

---

## Design Principles

1. **Zero-capability default.** A manifest with no entries grants nothing.
   Every secure-wrapper call returns `CapabilityViolationError` until the
   manifest declares the matching `(category, action, target)` triple.
2. **Single source of truth.** The manifest file is the authority. The
   generated code, the runtime check, the lint, and the OS sandbox all
   read it. Any drift means one of them is wrong.
3. **Secure wrappers are the only path.** Direct stdlib calls
   (`std::fs::read`, `std::net::TcpStream::connect`, `std::process::Command`)
   are forbidden in any package whose `required_capabilities.json` does
   not justify the corresponding category. The lint enforces this.
4. **Backend is swappable.** The cage performs the manifest check and
   delegates the actual OS call to a `Backend` trait object. Tests use a
   recording backend; hard-cage agents use a host-RPC backend; ordinary
   builds use the stdlib backend.
5. **No "all" escape.** There is no `Capability::all()`. Broad access
   means enumerating every `(category, action, target)` triple
   explicitly. Each line is a separately auditable decision.
6. **Audit-wrapped.** Every secure call passes through an `Operation<T>`
   envelope that records arguments, latency, and outcome. Tests assert
   on the audit trail. In production, audit records can stream to the
   host or to disk.
7. **Mirror the Go library.** Where Go has `Manifest::Check`, Rust has
   `Manifest::check`. Where Go has `SecureFile.ReadFile(m, path)`, Rust
   has `SecureFile::read_file(m, path)`. Same arguments, same returns,
   same errors. A conformance suite exercises both.

---

## Capability Taxonomy

The taxonomy is fixed by the spec at 8 categories and 14 actions. New
categories require a spec amendment (and a corresponding update to the
JSON schema published alongside).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Fs,
    Net,
    Proc,
    Env,
    Ffi,
    Time,
    Stdin,
    Stdout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Read,
    Write,
    Create,
    Delete,
    List,
    Connect,
    Listen,
    Dns,
    Exec,
    Fork,
    Signal,
    Call,
    Load,
    Sleep,
}
```

Not every (category, action) combination is meaningful. The valid
pairings are:

```
fs       read, write, create, delete, list
net      connect, listen, dns
proc     exec, fork, signal
env      read, write
ffi      call, load
time     read, sleep
stdin    read
stdout   write
```

Constructing a `Capability` with an invalid (category, action) pair
returns `Err(InvalidCombination)`. The manifest loader rejects manifests
that contain invalid pairs at load time, not at first use.

---

## Capability and Manifest

```rust
pub struct Capability {
    pub category:      Category,
    pub action:        Action,
    pub target:        String,
    pub justification: String,
}

impl Capability {
    pub fn new(
        category: Category,
        action:   Action,
        target:   impl Into<String>,
        justification: impl Into<String>,
    ) -> Result<Self, InvalidCombination> { ... }
}
```

```rust
pub struct Manifest {
    capabilities: Vec<Capability>,
    // Internal precomputed indexes for fast lookup; opaque.
}

impl Manifest {
    /// Construct a manifest from a slice of capabilities. Validates
    /// that every (category, action) pair is meaningful.
    pub fn new(capabilities: Vec<Capability>)
        -> Result<Self, InvalidCombination>;

    /// The pre-built zero-capability manifest. Equivalent to
    /// `Manifest::new(vec![]).unwrap()`.
    pub fn empty() -> &'static Manifest;

    /// Load a manifest from `required_capabilities.json` at the given
    /// path. Returns `Err` for missing file, invalid JSON, schema
    /// violations, or invalid capability pairs.
    pub fn load_from_file(path: &Path) -> Result<Self, ManifestError>;

    /// Load a manifest from a JSON string.
    pub fn load_from_str(s: &str) -> Result<Self, ManifestError>;

    /// Returns true if the manifest declares a capability covering the
    /// (category, action, target) triple. Glob targets in the manifest
    /// are matched against literal `target`.
    pub fn has(&self, category: Category, action: Action, target: &str)
        -> bool;

    /// Returns Ok(()) if the manifest covers the triple, otherwise
    /// returns CapabilityViolationError.
    pub fn check(&self, category: Category, action: Action, target: &str)
        -> Result<(), CapabilityViolationError>;

    /// Borrow the underlying capability list.
    pub fn capabilities(&self) -> &[Capability];
}
```

The `Manifest` is immutable after construction. There is no `add` method.
To extend a manifest, build a new one with a longer list — a deliberate
choice that makes manifest provenance traceable in code review.

---

## Operation Wrapper

Every secure-wrapper function passes through an `Operation<T>` audit
envelope that records the call's name, properties, outcome, and latency.
This mirrors the Go pattern (`StartNew[T]("name", default, fn)`) but
adapts to Rust idioms.

```rust
pub struct Operation<T> {
    name:        &'static str,
    properties:  Vec<(String, String)>,
    started_at:  Instant,
    default:     T,
    /* opaque */
}

pub struct OperationResult<T> {
    pub successful:  bool,
    pub partial:     bool,
    pub value:       T,
    pub error:       Option<Box<dyn std::error::Error + Send + Sync>>,
    pub elapsed_ns:  u64,
    pub properties:  Vec<(String, String)>,
}

impl<T: Clone> Operation<T> {
    /// Adds a key-value property that will appear in the audit record.
    /// Use for the function's arguments (path, host, etc.).
    pub fn add_property(&mut self, key: impl Into<String>, value: impl Into<String>);
}

pub struct ResultFactory<T> { /* opaque */ }

impl<T: Clone> ResultFactory<T> {
    /// Successful completion with a value.
    pub fn ok(self, partial: bool, value: T) -> OperationResult<T>;

    /// Failure with an error and a fallback value.
    pub fn err(self, value: T, err: impl std::error::Error + Send + Sync + 'static)
        -> OperationResult<T>;
}

/// Run a secure operation inside an audit envelope.
///
/// The closure receives a mutable Operation and a ResultFactory; it
/// must return an OperationResult constructed via the factory. The
/// envelope records elapsed time and forwards the OperationResult to
/// any installed audit sink before returning the value to the caller.
pub fn start_new<T: Clone>(
    name:    &'static str,
    default: T,
    f: impl FnOnce(&mut Operation<T>, ResultFactory<T>) -> OperationResult<T>,
) -> (T, Option<Box<dyn std::error::Error + Send + Sync>>);

pub trait AuditSink: Send + Sync {
    fn record(&self, op: &OperationRecord);
}

pub struct OperationRecord {
    pub name:        &'static str,
    pub properties:  Vec<(String, String)>,
    pub successful:  bool,
    pub partial:     bool,
    pub elapsed_ns:  u64,
    pub timestamp:   SystemTime,
    pub error_kind:  Option<String>,
}

/// Install a process-wide audit sink. Returns a guard that restores the
/// previous sink when dropped.
pub fn set_audit_sink(sink: Arc<dyn AuditSink>) -> AuditGuard;
```

The default audit sink is a no-op. Tests install a recording sink to
assert on call sequences. The `host-runtime-rust` crate installs a
sink that forwards records to the host's audit channel.

---

## Backend Trait

The cage performs the manifest check, then delegates the actual OS call
to a `Backend` implementation. The default is `OpenBackend` (calls
stdlib). Tests inject mocks. Hard-cage agents inject a backend that
routes calls to the host process via the `secure-host-channel`.

```rust
pub trait Backend: Send + Sync {
    // Filesystem
    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write_file(&self, path: &Path, data: &[u8]) -> io::Result<()>;
    fn create_file(&self, path: &Path) -> io::Result<()>;
    fn delete_file(&self, path: &Path) -> io::Result<()>;
    fn list_dir(&self, path: &Path) -> io::Result<Vec<String>>;

    // Network
    fn dial(&self, network: &str, addr: &str) -> io::Result<Box<dyn StreamConn>>;
    fn listen(&self, network: &str, addr: &str) -> io::Result<Box<dyn StreamListener>>;
    fn lookup_host(&self, host: &str) -> io::Result<Vec<String>>;

    // Process
    fn command(&self, name: &str, args: &[String]) -> io::Result<Box<dyn ChildProcess>>;
    fn kill(&self, pid: u32, sig: i32) -> io::Result<()>;

    // Environment
    fn getenv(&self, key: &str) -> Option<String>;
    fn setenv(&self, key: &str, value: &str) -> io::Result<()>;

    // Time
    fn now(&self) -> SystemTime;
    fn sleep(&self, d: Duration);

    // Standard streams
    fn read_stdin(&self, buf: &mut [u8]) -> io::Result<usize>;
    fn write_stdout(&self, buf: &[u8]) -> io::Result<usize>;
}

pub struct OpenBackend;     // default — calls stdlib
pub struct TestBackend;     // recording / scripted responses for tests
pub struct DenyAllBackend;  // returns Err(PermissionDenied) for everything

/// Replace the process-wide default backend. Returns a guard that
/// restores the previous backend on drop.
pub fn with_backend(backend: Arc<dyn Backend>) -> BackendGuard;
```

The trait surface mirrors the Go interface so a future cross-language
audit can compare them line by line. The Rust types are richer
(`StreamConn`, `StreamListener`, `ChildProcess` are themselves traits
with read/write/spawn methods), but the operation surface is the same.

---

## Secure Wrappers

Every category has a corresponding secure-wrapper module. The wrappers
are functions, not methods on a struct, to mirror the Go API and to
keep call sites short.

### `secure_file`

```rust
pub mod secure_file {
    use super::*;

    pub fn read_file(m: &Manifest, path: &Path) -> io::Result<Vec<u8>>;
    pub fn write_file(m: &Manifest, path: &Path, data: &[u8]) -> io::Result<()>;
    pub fn create_file(m: &Manifest, path: &Path) -> io::Result<()>;
    pub fn delete_file(m: &Manifest, path: &Path) -> io::Result<()>;
    pub fn list_dir(m: &Manifest, path: &Path) -> io::Result<Vec<String>>;
}
```

Each function:
1. Begins an `Operation` with the function name.
2. Adds the path as a property.
3. Calls `m.check(Fs, <action>, path.to_str())`. On error, returns
   `CapabilityViolationError` wrapped as `io::Error::other`.
4. Calls the corresponding `Backend` method.
5. Closes the operation with success/failure.
6. Returns the value or the error.

### `secure_net`

```rust
pub mod secure_net {
    pub fn dial(m: &Manifest, network: &str, addr: &str)
        -> io::Result<Box<dyn StreamConn>>;
    pub fn listen(m: &Manifest, network: &str, addr: &str)
        -> io::Result<Box<dyn StreamListener>>;
    pub fn lookup_host(m: &Manifest, host: &str)
        -> io::Result<Vec<String>>;
}
```

`addr` for dial/listen is matched against `net:connect:host:port` /
`net:listen:host:port` patterns in the manifest.

### `secure_proc`

```rust
pub mod secure_proc {
    pub fn command(m: &Manifest, name: &str, args: &[String])
        -> io::Result<Box<dyn ChildProcess>>;
    pub fn kill(m: &Manifest, pid: u32, sig: i32) -> io::Result<()>;
    pub fn fork(m: &Manifest) -> io::Result<()>;
}
```

### `secure_env`

```rust
pub mod secure_env {
    pub fn getenv(m: &Manifest, key: &str) -> io::Result<Option<String>>;
    pub fn setenv(m: &Manifest, key: &str, value: &str) -> io::Result<()>;
}
```

### `secure_time`

```rust
pub mod secure_time {
    pub fn now(m: &Manifest) -> io::Result<SystemTime>;
    pub fn sleep(m: &Manifest, d: Duration) -> io::Result<()>;
}
```

### `secure_stdio`

```rust
pub mod secure_stdio {
    pub fn read_stdin(m: &Manifest, buf: &mut [u8]) -> io::Result<usize>;
    pub fn write_stdout(m: &Manifest, buf: &[u8]) -> io::Result<usize>;
}
```

---

## Glob Matching

Manifest targets may include glob patterns. The matcher must follow the
same rules as the Go implementation so the conformance suite passes:

| Pattern               | Matches                                   |
|-----------------------|-------------------------------------------|
| `foo`                 | exactly `foo`                             |
| `*`                   | any single path component (no separators) |
| `**`                  | any number of components                  |
| `*.tokens`            | any single component ending in `.tokens`  |
| `./grammars/*.tokens` | one-level files under `./grammars/`       |
| `./grammars/**/*.tokens` | any-depth `.tokens` under `./grammars/`|
| `host:port`           | literal host and port (for net targets)   |
| `*:443`               | any host on port 443                      |
| `api.weather.gov:*`   | any port on `api.weather.gov`             |

The matcher is path-aware: separators in `target` (the literal call
argument) are treated as boundaries. `*` matches one component, `**`
matches across components.

```rust
pub fn match_target(pattern: &str, candidate: &str) -> bool;
```

This function is also exposed publicly for use by the lint and the
sandbox compiler.

---

## Code Generation: `gen_capabilities.rs`

The Go library generates `gen_capabilities.go` per package as a
build-time step that materializes the package's `Manifest` from the
JSON. The Rust library does the same via a `build.rs` script.

```rust
// Generated content (rust/build.rs output)
// DO NOT EDIT — regenerate with `cargo build`.

use coding_adventures_capability_cage::{Manifest, Capability, Category, Action};

pub fn package_manifest() -> Manifest {
    Manifest::new(vec![
        Capability::new(Category::Fs, Action::Read,
            "../../grammars/json.tokens",
            "Reads token grammar definition file to build the lexer DFA.")
            .unwrap(),
    ]).unwrap()
}
```

Every package's `lib.rs` (or `main.rs`) imports `package_manifest()` once
at startup, stores it in a `LazyLock`, and passes it to every secure
call. Because the manifest is generated from the JSON, it cannot drift
from the declared file.

The `build.rs` reads `required_capabilities.json` at the package root,
parses it, validates each entry against the JSON schema, and writes the
generated Rust file to `OUT_DIR`. If the JSON is missing or invalid,
the build fails with a clear error pointing to the manifest file.

---

## Lint

A separate `cargo` lint walks the package source and rejects any direct
use of stdlib OS APIs (`std::fs`, `std::net`, `std::process`,
`std::env`, etc.) outside of the cage's own backend implementations.
The lint reads the same `required_capabilities.json` to know what the
package is permitted to do, then verifies that every secure-wrapper
call site has a matching manifest entry.

The lint is the first ring of the defense-in-depth chain (R1a). It runs
in CI as a blocking check. The runtime check (R1b) inside the secure
wrapper is the second ring.

This spec defines the lint's contract; the lint implementation is its
own follow-up package (`capability-cage-lint`).

---

## Errors

```rust
pub struct CapabilityViolationError {
    pub category: Category,
    pub action:   Action,
    pub target:   String,
    pub message:  String,
}

impl Display for CapabilityViolationError { /* ... */ }
impl std::error::Error for CapabilityViolationError {}

pub enum InvalidCombination {
    UnsupportedPair { category: Category, action: Action },
    EmptyTarget,
    InvalidTargetFormat { reason: String },
}

pub enum ManifestError {
    Io(io::Error),
    Parse(String),
    Schema { path: String, reason: String },
    InvalidCombination(InvalidCombination),
}
```

Every secure-wrapper function returns `io::Result<T>` — the
`CapabilityViolationError` is wrapped as `io::Error::other(...)` so the
caller's existing `io::Result` propagation just works. The original
error is recoverable via `downcast_ref::<CapabilityViolationError>()`.

---

## Hard Cage: `HostRpcBackend`

For Tier 2 and Tier 3 agents, the cage's `Backend` is replaced with
`HostRpcBackend` (defined in `host-runtime-rust`, not in this crate).
Every backend method serializes its arguments into a Host Protocol
JSON-RPC request, sends it via the `secure-host-channel`, and decodes
the response.

The agent's code does not change. The same `secure_file::read_file(&m,
path)` call goes through the same manifest check, but instead of
hitting the local filesystem, it ends up as a `fs.read` request to the
host, which performs its own manifest check and ultimately calls the
OS. The double check (cage in agent + cage in host) is the
defense-in-depth principle made concrete.

The `HostRpcBackend` is documented separately in `host-runtime-rust.md`.

---

## Public API Summary

```rust
// Re-exports from the crate root
pub use cage::{
    Category, Action, Capability, Manifest, ManifestError,
    InvalidCombination, CapabilityViolationError,
    Backend, OpenBackend, TestBackend, DenyAllBackend,
    BackendGuard, with_backend,
    AuditSink, AuditGuard, OperationRecord, set_audit_sink,
    secure_file, secure_net, secure_proc,
    secure_env,  secure_time, secure_stdio,
    match_target,
};
```

---

## Test Strategy

### Unit Tests

1. **Manifest construction.**
   - `Manifest::empty()` has zero capabilities.
   - `Manifest::new(vec![...])` round-trips the capability list.
   - Invalid `(category, action)` pairs return `InvalidCombination`.
2. **Has / Check.**
   - Exact-match target returns true.
   - Glob match returns true.
   - Wrong category, wrong action, or non-matching target returns false.
   - `check` returns Ok for matches, `Err(CapabilityViolationError)` otherwise.
3. **Manifest loader.**
   - Valid JSON loads correctly.
   - Missing file returns `Io`.
   - Malformed JSON returns `Parse`.
   - Schema-violating JSON returns `Schema`.
4. **Glob matcher.** Each row of the glob match table above is a test.
5. **Secure wrappers.** For each function:
   - Granted manifest + valid call → success, audit record emitted.
   - Denied manifest → `CapabilityViolationError` wrapped in
     `io::Error`, no backend call made.
   - Backend error → original error preserved.
6. **Backend swap.**
   - `with_backend(TestBackend)` routes calls to the test backend.
   - Guard restores `OpenBackend` on drop.
   - `DenyAllBackend` returns `PermissionDenied` for everything.
7. **Audit sink.**
   - Records arrive in order.
   - Each record has the correct name, properties, and elapsed time.
   - Failed calls have `successful = false` and an `error_kind`.

### Conformance Suite

Both the Go and Rust libraries import a JSON file
`tests/conformance/cases.json` containing test cases of the form:

```
{
  "manifest": [
    { "category": "fs", "action": "read",
      "target": "./grammars/*.tokens" }
  ],
  "checks": [
    { "category": "fs", "action": "read",
      "target": "./grammars/json.tokens", "expect": "ok" },
    { "category": "fs", "action": "read",
      "target": "./grammars/sub/json.tokens", "expect": "deny" },
    { "category": "fs", "action": "write",
      "target": "./grammars/json.tokens", "expect": "deny" }
  ]
}
```

The Go test loads each case and asserts; the Rust test loads the same
file and asserts the same outcomes. Any divergence is a bug — the spec
treats Go and Rust as two implementations of one contract.

### Build-Script Tests

8. The build script generates a Rust file matching a fixed expected
   shape for known input manifests.
9. Missing `required_capabilities.json` fails the build with a
   pointing-finger error message.
10. Schema-invalid manifest fails the build with a useful diagnostic.

### Coverage Target

`>=95%` line coverage. The cage is the security boundary for every
package; bugs here weaken every other package.

---

## Trade-Offs

**Wrappers as functions, not methods.** A more idiomatic Rust API might
be `SecureFile::new(&m).read(path)`. We choose free functions to mirror
the Go API exactly so the conformance suite is a line-by-line port.
Future versions may add a builder-style sugar layer on top.

**`io::Result` everywhere.** Rust callers expect `io::Result` for OS
ops. Wrapping `CapabilityViolationError` inside `io::Error::other`
keeps the surface uniform but loses static type information about the
denial. Callers who care can downcast; most just propagate.

**Build.rs for code generation.** A `proc-macro` would be cleaner but
adds a compile-time dependency and slows incremental builds. The
build.rs approach matches the Go pattern of a generated `.go` file
checked in (well, generated to OUT_DIR — checked-in versions are
optional).

**No async wrappers in v1.** Every secure call is synchronous. Async
wrappers (`async fn read_file_async`) are deferred until a real use
case demands them. The host runtime can wrap synchronous backend calls
in `tokio::task::spawn_blocking` when it needs concurrency.

**`with_backend` is a process-wide swap, not a per-task one.** Tests
that need different backends per test run them serially (the Rust
default for `cargo test --test-threads=1` in the affected suite). A
per-task or per-thread backend can be added later if real workloads
need it; today, simpler is better.

**The lint is its own crate.** Keeping the runtime cage lean (no
syn/proc-macro2 deps) means the lint lives separately. Most consumers
need only the runtime; CI pulls in the lint.

---

## Future Extensions

- **Async secure wrappers** (`async fn`) once the host runtime needs
  concurrent secure calls.
- **Per-task backend** for test parallelism.
- **Streaming wrappers** for long-running ops (file reads, network
  transfers) where holding the entire body in memory is wrong.
- **Capability negotiation** for inter-package calls: when package A
  calls package B, B's manifest is checked against A's, so A cannot
  acquire B's capabilities by transitive call.

These are deliberately out of scope for V1.
