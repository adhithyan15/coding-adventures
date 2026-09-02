//! # wasm-runtime
//!
//! Complete WebAssembly 1.0 runtime — parse, validate, instantiate, execute.
//!
//! This crate composes the lower-level WASM packages into a single, user-facing
//! API. It handles the full pipeline:
//!
//! ```text
//! .wasm bytes  -->  Parse  -->  Validate  -->  Instantiate  -->  Execute
//!     |               |            |               |               |
//! &[u8]         WasmModule  ValidatedModule  WasmInstance    WasmValue[]
//!     |               |            |               |               |
//! (input)      (module-parser) (validator)    (this file)    (execution)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use wasm_runtime::WasmRuntime;
//!
//! let runtime = WasmRuntime::new();
//! let result = runtime.load_and_run(&square_wasm, "square", &[5]);
//! assert_eq!(result.unwrap(), vec![25]);
//! ```
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use wasm_execution::{
    evaluate_const_expr, evaluate_const_expr_gc, FuncRefTarget, GcObject, GlobalStorage, HostFunction, HostInterface,
    LinearMemory, SelfFunctionResolver, Table, TableElement, TrapError, WasmEngineConfig, WasmExecutionEngine,
    WasmValue,
};
use wasm_module_parser::WasmModuleParser;
use wasm_types::{
    CanonicalGroup, ExternalKind, FuncType, FunctionBody, GlobalType, Import, ImportTypeInfo, Limits, ValueType,
    WasmModule,
};
use wasm_validator::{validate, ValidatedModule, ValidationError};
use virtual_machine::VMError;

const WASI_ESUCCESS: i32 = 0;
const WASI_EBADF: i32 = 8;
const WASI_EINVAL: i32 = 28;
const WASI_ENOSYS: i32 = 52;

// ══════════════════════════════════════════════════════════════════════════════
// ProcExitError
// ══════════════════════════════════════════════════════════════════════════════

/// Thrown when a WASM program calls `proc_exit`.
///
/// Not a real error — it is the WASM program requesting clean termination.
/// The runtime catches this and returns the exit code.
#[derive(Debug, Clone)]
pub struct ProcExitError {
    /// The exit code the program requested.
    pub exit_code: i32,
}

impl std::fmt::Display for ProcExitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "proc_exit({})", self.exit_code)
    }
}

impl std::error::Error for ProcExitError {}

// ══════════════════════════════════════════════════════════════════════════════
// WasiClock and WasiRandom traits
// ══════════════════════════════════════════════════════════════════════════════

/// Provides time information to the WASI host functions.
///
/// Implement this trait to inject a fake or deterministic clock for testing.
/// The production implementation (`SystemClock`) uses the OS wall clock and
/// a lazy-initialized monotonic start instant.
///
/// ## Clock IDs (WASI preview1)
///
/// | ID | Meaning                       |
/// |----|-------------------------------|
/// |  0 | REALTIME — wall clock (UTC)   |
/// |  1 | MONOTONIC — never goes back   |
/// |  2 | PROCESS_CPUTIME (→ realtime)  |
/// |  3 | THREAD_CPUTIME (→ realtime)   |
///
/// All timestamps are in **nanoseconds**.
pub trait WasiClock: Send + Sync {
    /// Nanoseconds since Unix epoch (CLOCK_REALTIME).
    fn realtime_ns(&self) -> i64;

    /// Nanoseconds since an arbitrary monotonic start point (CLOCK_MONOTONIC).
    ///
    /// Guaranteed never to go backward on the same host, but the absolute
    /// value is meaningless across processes.
    fn monotonic_ns(&self) -> i64;

    /// Clock resolution in nanoseconds for the given clock ID.
    ///
    /// For example, many OS clocks have 1 ms (1_000_000 ns) resolution.
    fn resolution_ns(&self, clock_id: i32) -> i64;
}

/// Provides random bytes to the WASI `random_get` host function.
///
/// Implement this trait to inject a deterministic fake RNG for testing.
/// The production implementation (`SystemRandom`) uses a hash-based fallback
/// that is NOT cryptographically secure — swap it for getrandom or ring when
/// security matters.
pub trait WasiRandom: Send + Sync {
    /// Fill `buf` with random (or deterministic-test) bytes.
    fn fill_bytes(&self, buf: &mut [u8]);
}

// ══════════════════════════════════════════════════════════════════════════════
// SystemClock — production clock using OS time
// ══════════════════════════════════════════════════════════════════════════════

/// Production clock backed by `std::time::SystemTime` and `Instant`.
///
/// `realtime_ns` calls `SystemTime::now()` on every invocation.
/// `monotonic_ns` uses a lazy `Instant` initialized on first call so the
/// returned value is "nanoseconds since first monotonic measurement in this
/// process", not since boot.
pub struct SystemClock;

impl WasiClock for SystemClock {
    fn realtime_ns(&self) -> i64 {
        // Duration::as_nanos() returns u128; cast to i64 is valid until 2262.
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64
    }

    fn monotonic_ns(&self) -> i64 {
        // OnceLock captures the first call's Instant so subsequent calls
        // return elapsed time, giving a strictly non-decreasing sequence.
        use std::sync::OnceLock;
        static START: OnceLock<Instant> = OnceLock::new();
        let start = START.get_or_init(Instant::now);
        start.elapsed().as_nanos() as i64
    }

    fn resolution_ns(&self, _clock_id: i32) -> i64 {
        // 1 ms is a conservative resolution that is accurate for most OS
        // clocks (Linux typically achieves ~100 ns, macOS ~1 µs, Windows
        // ~15 ms, but 1 ms is a safe lower bound for all platforms).
        1_000_000
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SystemRandom — production random using hash-based fallback
// ══════════════════════════════════════════════════════════════════════════════

/// Production random that mixes `SystemTime` with per-byte index.
///
/// **This is NOT cryptographically secure.** It is acceptable for WASM
/// programs that use `random_get` for non-security purposes (e.g., seeding
/// a game). Swap `SystemRandom` for a `getrandom`- or `ring`-backed
/// implementation when security is required.
///
/// The design is intentionally swappable via `WasiConfig::random`.
pub struct SystemRandom;

impl WasiRandom for SystemRandom {
    fn fill_bytes(&self, buf: &mut [u8]) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Mix wall-clock time with the byte position to produce pseudorandom
        // output.  Each byte gets an independent hash so patterns don't
        // repeat for small buffers.
        for (i, b) in buf.iter_mut().enumerate() {
            let mut h = DefaultHasher::new();
            (SystemTime::now(), i).hash(&mut h);
            *b = h.finish() as u8;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// WasiConfig — configuration bundle for WasiStub
// ══════════════════════════════════════════════════════════════════════════════

/// Configuration for a WASI host implementation.
///
/// Pass this to `WasiStub::with_config` to customise arguments, environment
/// variables, I/O callbacks, and the injected clock / RNG.
///
/// ## Example — deterministic test config
///
/// ```rust,ignore
/// let cfg = WasiConfig {
///     args: vec!["myapp".into(), "hello".into()],
///     env:  vec!["HOME=/tmp".into()],
///     clock:  Box::new(FakeClock),
///     random: Box::new(FakeRandom),
///     ..Default::default()
/// };
/// ```
pub struct WasiConfig {
    /// Command-line arguments (`argv`).  The first element is conventionally
    /// the program name.
    pub args: Vec<String>,

    /// Environment variables in `"KEY=VALUE"` format.
    pub env: Vec<String>,

    /// Optional callback invoked for every line written to stdout (fd 1).
    #[allow(clippy::type_complexity)] // boxed host callback signature is intentional
    pub stdout_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,

    /// Optional callback invoked for every line written to stderr (fd 2).
    #[allow(clippy::type_complexity)] // boxed host callback signature is intentional
    pub stderr_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,

    /// Optional callback invoked when stdin bytes are requested (fd 0).
    #[allow(clippy::type_complexity)] // boxed host callback signature is intentional
    pub stdin_callback: Option<Box<dyn Fn(usize) -> Vec<u8> + Send + Sync>>,

    /// Injected clock.  Defaults to `SystemClock`.
    pub clock: Box<dyn WasiClock>,

    /// Injected random.  Defaults to `SystemRandom`.
    pub random: Box<dyn WasiRandom>,
}

impl Default for WasiConfig {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            env: Vec::new(),
            stdout_callback: None,
            stderr_callback: None,
            stdin_callback: None,
            clock: Box::new(SystemClock),
            random: Box::new(SystemRandom),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// WasiStub
// ══════════════════════════════════════════════════════════════════════════════

/// A minimal WASI host implementation.
///
/// Provides `fd_write` (captures stdout/stderr) and `proc_exit` (terminates
/// execution). All other WASI functions return ENOSYS (52).
pub struct WasiStub {
    /// Callback for stdout output.
    #[allow(dead_code)] // retained as API surface / scaffolding
    stdout_callback: Box<dyn Fn(&str)>,
}

impl WasiStub {
    /// Create a new WASI stub with a stdout callback.
    pub fn new(stdout_callback: impl Fn(&str) + 'static) -> Self {
        WasiStub {
            stdout_callback: Box::new(stdout_callback),
        }
    }
}

impl HostInterface for WasiStub {
    fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>> {
        if module_name != "wasi_snapshot_preview1" {
            return None;
        }

        match name {
            "proc_exit" => Some(Box::new(ProcExitFunc)),
            // Other WASI functions return ENOSYS
            _ => Some(Box::new(EnosysFunc {
                func_type: FuncType {
                    params: vec![],
                    results: vec![ValueType::I32],
                },
            })),
        }
    }

    fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        None
    }

    fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
        None
    }

    fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
        None
    }
}

/// Host function that implements proc_exit.
struct ProcExitFunc;

impl HostFunction for ProcExitFunc {
    fn func_type(&self) -> &FuncType {
        // We use a static-like approach. Since this is simple, just return a reference
        // to a locally constructed type. To avoid lifetime issues, we leak it.
        // In practice this is fine for a singleton.
        static FUNC_TYPE: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32],
            results: vec![],
        });
        &FUNC_TYPE
    }

    fn call(
        &self,
        args: &[WasmValue],
        _memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let exit_code = args.first().and_then(|v| v.as_i32().ok()).unwrap_or(0);
        Err(TrapError::new(format!("proc_exit({})", exit_code)))
    }
}

/// Host function that returns ENOSYS (52) for unimplemented WASI calls.
struct EnosysFunc {
    func_type: FuncType,
}

impl HostFunction for EnosysFunc {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }

    fn call(
        &self,
        _args: &[WasmValue],
        _memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        Ok(vec![WasmValue::I32(WASI_ENOSYS)])
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// WasiEnv — WASI Tier 3 host interface
// ══════════════════════════════════════════════════════════════════════════════

/// A full WASI Tier 3 host implementation.
///
/// Provides the eight new WASI functions on top of `proc_exit`:
///
/// | Function           | Description                                          |
/// |--------------------|------------------------------------------------------|
/// | `args_sizes_get`   | Return argc and total args buffer size               |
/// | `args_get`         | Write argv pointers and null-terminated strings      |
/// | `environ_sizes_get`| Return envc and total environ buffer size            |
/// | `environ_get`      | Write environ pointers and null-terminated strings   |
/// | `clock_res_get`    | Return clock resolution in nanoseconds               |
/// | `clock_time_get`   | Return current clock time in nanoseconds             |
/// | `random_get`       | Fill a WASM memory region with random bytes          |
/// | `sched_yield`      | Yield the scheduler (no-op in single-threaded host)  |
///
/// Memory-accessing functions (args_get, environ_get, clock_time_get,
/// clock_res_get, random_get) need to write directly into WASM linear
/// memory. Since `HostFunction::call` has no memory parameter, we use a
/// shared `Rc<RefCell<LinearMemory>>` that is populated by the runtime
/// **before** the first WASM call. See `WasiEnv::attach_memory`.
///
/// `Rc<RefCell<..>>`, not `Arc<Mutex<..>>` (W28): `LinearMemory` itself is
/// now `Rc<RefCell<..>>`-backed internally (see `wasm-execution`'s own
/// CHANGELOG) so that an imported memory shares live storage with its
/// exporting instance, rather than a `#[derive(Clone)]` producing an
/// independent copy. That makes `LinearMemory` — and therefore `WasiEnv`,
/// which holds one — no longer `Send`/`Sync`: wrapping a non-`Send` type
/// in `Arc<Mutex<..>>` would be a real, `clippy::arc_with_non_send_sync`-
/// flagged soundness hazard (an `Rc`'s non-atomic refcount has no
/// synchronization against a DIFFERENT, unrelated clone of the same
/// memory held elsewhere without going through this same `Mutex`), not
/// just a lint to silence. `HostFunction`/`HostInterface` (see
/// `wasm-execution`) have never required `Send` — `wasm-conformance`'s
/// own `CrossModuleFunction` already holds an `Rc<RefCell<WasmInstance>>`
/// — so this is a correctness fix, not a capability this crate actually
/// used: nothing in this repo shares a single `WasiEnv` across real OS
/// threads (confirmed by checking every consumer — `brainfuck-wasm-
/// compiler`/`ir-to-wasm-compiler`'s own tests each construct one
/// `WasiEnv` and use it entirely on one thread).
pub struct WasiEnv {
    /// Command-line arguments.
    pub args: Vec<String>,

    /// Environment variables in "KEY=VALUE" format.
    pub env: Vec<String>,

    /// Shared handle to WASM linear memory. Populated via `attach_memory`
    /// after instantiation.
    pub memory: Rc<RefCell<Option<LinearMemory>>>,

    /// Injected clock.
    pub clock: Arc<dyn WasiClock>,

    /// Injected random.
    pub random: Arc<dyn WasiRandom>,

    /// Callback for stdout output (fd 1).
    pub stdout_callback: Arc<dyn Fn(&str) + Send + Sync>,

    /// Callback for stderr output (fd 2).
    pub stderr_callback: Arc<dyn Fn(&str) + Send + Sync>,

    /// Callback for stdin bytes (fd 0).
    pub stdin_callback: Arc<dyn Fn(usize) -> Vec<u8> + Send + Sync>,
}

/// Preferred name for the full WASI host surface.
///
/// `WasiEnv` remains available, but new call sites should prefer `WasiHost`
/// to match the other language runtimes in the repo.
pub type WasiHost = WasiEnv;

impl WasiEnv {
    /// Create a `WasiEnv` from a `WasiConfig`.
    pub fn new(cfg: WasiConfig) -> Self {
        let stdout_callback: Arc<dyn Fn(&str) + Send + Sync> = match cfg.stdout_callback {
            Some(callback) => Arc::from(callback),
            None => Arc::new(|_: &str| {}),
        };
        let stderr_callback: Arc<dyn Fn(&str) + Send + Sync> = match cfg.stderr_callback {
            Some(callback) => Arc::from(callback),
            None => Arc::new(|_: &str| {}),
        };
        let stdin_callback: Arc<dyn Fn(usize) -> Vec<u8> + Send + Sync> =
            match cfg.stdin_callback {
                Some(callback) => Arc::from(callback),
                None => Arc::new(|_: usize| Vec::new()),
            };
        WasiEnv {
            args: cfg.args,
            env: cfg.env,
            memory: Rc::new(RefCell::new(None)),
            clock: Arc::from(cfg.clock),
            random: Arc::from(cfg.random),
            stdout_callback,
            stderr_callback,
            stdin_callback,
        }
    }

    /// Attach linear memory so that memory-accessing host functions can write
    /// into it.
    ///
    /// Call this after `WasmRuntime::instantiate` but before executing any
    /// WASM that calls WASI memory functions.
    pub fn attach_memory(&self, mem: LinearMemory) {
        *self.memory.borrow_mut() = Some(mem);
    }

    /// Retrieve the memory after execution (so the caller can inspect it or
    /// put it back into the `WasmInstance`).
    pub fn take_memory(&self) -> Option<LinearMemory> {
        self.memory.borrow_mut().take()
    }
}

impl HostInterface for WasiEnv {
    fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>> {
        if module_name != "wasi_snapshot_preview1" {
            return None;
        }

        match name {
            // ── Tier 1: stdio + process termination ───────────────────────
            "fd_write" => Some(Box::new(FdWriteFunc {
                memory: Rc::clone(&self.memory),
                stdout_callback: Arc::clone(&self.stdout_callback),
                stderr_callback: Arc::clone(&self.stderr_callback),
            })),
            "fd_read" => Some(Box::new(FdReadFunc {
                memory: Rc::clone(&self.memory),
                stdin_callback: Arc::clone(&self.stdin_callback),
            })),
            "proc_exit" => Some(Box::new(ProcExitFunc)),

            // ── Tier 3: arguments ─────────────────────────────────────────
            "args_sizes_get" => Some(Box::new(ArgsSizesGetFunc {
                args: self.args.clone(),
                memory: Rc::clone(&self.memory),
            })),
            "args_get" => Some(Box::new(ArgsGetFunc {
                args: self.args.clone(),
                memory: Rc::clone(&self.memory),
            })),

            // ── Tier 3: environment ───────────────────────────────────────
            "environ_sizes_get" => Some(Box::new(EnvironSizesGetFunc {
                env: self.env.clone(),
                memory: Rc::clone(&self.memory),
            })),
            "environ_get" => Some(Box::new(EnvironGetFunc {
                env: self.env.clone(),
                memory: Rc::clone(&self.memory),
            })),

            // ── Tier 3: clock ─────────────────────────────────────────────
            "clock_res_get" => Some(Box::new(ClockResGetFunc {
                clock: Arc::clone(&self.clock),
                memory: Rc::clone(&self.memory),
            })),
            "clock_time_get" => Some(Box::new(ClockTimeGetFunc {
                clock: Arc::clone(&self.clock),
                memory: Rc::clone(&self.memory),
            })),

            // ── Tier 3: random ────────────────────────────────────────────
            "random_get" => Some(Box::new(RandomGetFunc {
                random: Arc::clone(&self.random),
                memory: Rc::clone(&self.memory),
            })),

            // ── Tier 3: scheduler ─────────────────────────────────────────
            "sched_yield" => Some(Box::new(SchedYieldFunc)),

            // All other WASI functions return ENOSYS (function not supported).
            _ => Some(Box::new(EnosysFunc {
                func_type: FuncType {
                    params: vec![],
                    results: vec![ValueType::I32],
                },
            })),
        }
    }

    fn resolve_global(&self, _: &str, _: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        None
    }

    fn resolve_memory(&self, _: &str, _: &str) -> Option<LinearMemory> {
        None
    }

    fn resolve_table(&self, _: &str, _: &str) -> Option<Table> {
        None
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper: write an i64 as little-endian into shared memory
// ══════════════════════════════════════════════════════════════════════════════

/// Write a 64-bit integer at `ptr` in WASM linear memory (little-endian).
///
/// WASM is always little-endian. We split the i64 into two i32 halves and use
/// the existing `store_i32` primitives rather than duplicating byte-level code.
///
/// ```text
/// Memory layout (little-endian):
///   ptr+0 .. ptr+3  — low 32 bits
///   ptr+4 .. ptr+7  — high 32 bits
/// ```
fn write_i64_le(memory: &mut LinearMemory, ptr: usize, value: i64) -> Result<(), TrapError> {
    let lo = (value & 0xFFFF_FFFF) as i32;
    let hi = ((value >> 32) & 0xFFFF_FFFF) as i32;
    memory.store_i32(ptr, lo)?;
    memory.store_i32(ptr + 4, hi)?;
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper: write an i32 into shared memory
// ══════════════════════════════════════════════════════════════════════════════

fn write_i32_le(memory: &mut LinearMemory, ptr: usize, value: i32) -> Result<(), TrapError> {
    memory.store_i32(ptr, value)
}

fn with_linear_memory<T>(
    provided: Option<&mut LinearMemory>,
    shared: &Rc<RefCell<Option<LinearMemory>>>,
    action: impl FnOnce(&mut LinearMemory) -> Result<T, TrapError>,
) -> Result<T, TrapError> {
    if let Some(memory) = provided {
        return action(memory);
    }

    let mut guard = shared.borrow_mut();
    let memory = guard
        .as_mut()
        .ok_or_else(|| TrapError::new("no memory attached"))?;
    action(memory)
}

fn read_i32_le(memory: &mut LinearMemory, ptr: usize) -> Result<i32, TrapError> {
    memory.load_i32(ptr)
}

fn read_guest_bytes(
    memory: &mut LinearMemory,
    ptr: usize,
    len: usize,
) -> Result<Vec<u8>, TrapError> {
    let mut bytes = Vec::with_capacity(len);
    for offset in 0..len {
        bytes.push(memory.load_i32_8u(ptr + offset)? as u8);
    }
    Ok(bytes)
}

// ══════════════════════════════════════════════════════════════════════════════
// Tier 3 host functions
// ══════════════════════════════════════════════════════════════════════════════

// ── Tier 1: fd_write ────────────────────────────────────────────────────────

struct FdWriteFunc {
    memory: Rc<RefCell<Option<LinearMemory>>>,
    stdout_callback: Arc<dyn Fn(&str) + Send + Sync>,
    stderr_callback: Arc<dyn Fn(&str) + Send + Sync>,
}

impl HostFunction for FdWriteFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let fd = args[0].as_i32().map_err(|e| TrapError::new(e.message))?;
        let iovs_ptr = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let iovs_len = args[2].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let nwritten_ptr = args[3].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        if fd != 1 && fd != 2 {
            return Ok(vec![WasmValue::I32(WASI_EBADF)]);
        }

        let output = with_linear_memory(memory, &self.memory, |mem| {
            let mut output = Vec::new();
            for index in 0..iovs_len {
                let base = iovs_ptr + index * 8;
                let ptr = read_i32_le(mem, base)? as usize;
                let len = read_i32_le(mem, base + 4)? as usize;
                output.extend(read_guest_bytes(mem, ptr, len)?);
            }
            write_i32_le(mem, nwritten_ptr, output.len() as i32)?;
            Ok(output)
        })?;

        let text = String::from_utf8_lossy(&output);
        if fd == 1 {
            (self.stdout_callback)(&text);
        } else {
            (self.stderr_callback)(&text);
        }

        Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
    }
}

// ── Tier 1: fd_read ─────────────────────────────────────────────────────────

struct FdReadFunc {
    memory: Rc<RefCell<Option<LinearMemory>>>,
    stdin_callback: Arc<dyn Fn(usize) -> Vec<u8> + Send + Sync>,
}

impl HostFunction for FdReadFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let fd = args[0].as_i32().map_err(|e| TrapError::new(e.message))?;
        let iovs_ptr = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let iovs_len = args[2].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let nread_ptr = args[3].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        if fd != 0 {
            return Ok(vec![WasmValue::I32(WASI_EBADF)]);
        }

        with_linear_memory(memory, &self.memory, |mem| {
            let mut requested = 0usize;
            for index in 0..iovs_len {
                let base = iovs_ptr + index * 8;
                requested += read_i32_le(mem, base + 4)? as usize;
            }
            let stdin_bytes = (self.stdin_callback)(requested);
            let mut written = 0usize;
            for index in 0..iovs_len {
                if written >= stdin_bytes.len() {
                    break;
                }

                let base = iovs_ptr + index * 8;
                let ptr = read_i32_le(mem, base)? as usize;
                let len = read_i32_le(mem, base + 4)? as usize;
                let remaining = stdin_bytes.len() - written;
                let chunk_len = remaining.min(len);

                if chunk_len > 0 {
                    mem.write_bytes(ptr, &stdin_bytes[written..written + chunk_len])?;
                    written += chunk_len;
                }
            }

            write_i32_le(mem, nread_ptr, written as i32)?;
            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 1. args_sizes_get ────────────────────────────────────────────────────────

/// WASI `args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) → errno`
///
/// Writes two i32 values into linear memory:
/// - `*argc_ptr` = number of arguments
/// - `*argv_buf_size_ptr` = total bytes needed for all null-terminated argument
///   strings
///
/// Returns errno 0 (success).
///
/// ## WASI Spec
/// The "buf size" counts every argument as `len(arg_bytes) + 1` (the +1 is
/// the null terminator `\0`).
struct ArgsSizesGetFunc {
    args: Vec<String>,
    memory: Rc<RefCell<Option<LinearMemory>>>,
}

impl HostFunction for ArgsSizesGetFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let argc_ptr = args[0].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let buf_size_ptr = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        let argc = self.args.len() as i32;
        // Each argument occupies len(utf8) + 1 bytes (null terminator).
        let buf_size: i32 = self.args.iter().map(|a| a.len() as i32 + 1).sum();

        with_linear_memory(memory, &self.memory, |mem| {
            write_i32_le(mem, argc_ptr, argc)?;
            write_i32_le(mem, buf_size_ptr, buf_size)?;
            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 2. args_get ──────────────────────────────────────────────────────────────

/// WASI `args_get(argv_ptr: i32, argv_buf_ptr: i32) → errno`
///
/// Writes the argv pointer array and the raw argument strings into memory.
///
/// ## Memory layout
///
/// ```text
/// argv_ptr:
///   [i32] → address of "myapp\0"
///   [i32] → address of "hello\0"
///   ...
///
/// argv_buf_ptr:
///   b'm' b'y' b'a' b'p' b'p' 0x00
///   b'h' b'e' b'l' b'l' b'o' 0x00
/// ```
///
/// Each pointer in the argv array points into `argv_buf`.
struct ArgsGetFunc {
    args: Vec<String>,
    memory: Rc<RefCell<Option<LinearMemory>>>,
}

impl HostFunction for ArgsGetFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let argv_ptr = args[0].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let argv_buf_ptr = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        with_linear_memory(memory, &self.memory, |mem| {
            let mut buf_cursor = argv_buf_ptr;
            for (i, arg) in self.args.iter().enumerate() {
                let ptr_slot = argv_ptr + i * 4;
                write_i32_le(mem, ptr_slot, buf_cursor as i32)?;

                let bytes = arg.as_bytes();
                mem.write_bytes(buf_cursor, bytes)?;
                mem.write_bytes(buf_cursor + bytes.len(), &[0u8])?;
                buf_cursor += bytes.len() + 1;
            }

            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 3. environ_sizes_get ─────────────────────────────────────────────────────

/// WASI `environ_sizes_get(envc_ptr: i32, environ_buf_size_ptr: i32) → errno`
///
/// Same shape as `args_sizes_get` but for environment variables.
/// Each env var is a `"KEY=VALUE"` string.
struct EnvironSizesGetFunc {
    env: Vec<String>,
    memory: Rc<RefCell<Option<LinearMemory>>>,
}

impl HostFunction for EnvironSizesGetFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let envc_ptr = args[0].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let buf_size_ptr = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        let envc = self.env.len() as i32;
        let buf_size: i32 = self.env.iter().map(|e| e.len() as i32 + 1).sum();

        with_linear_memory(memory, &self.memory, |mem| {
            write_i32_le(mem, envc_ptr, envc)?;
            write_i32_le(mem, buf_size_ptr, buf_size)?;
            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 4. environ_get ───────────────────────────────────────────────────────────

/// WASI `environ_get(environ_ptr: i32, environ_buf_ptr: i32) → errno`
///
/// Same layout as `args_get` but for environment variables.
struct EnvironGetFunc {
    env: Vec<String>,
    memory: Rc<RefCell<Option<LinearMemory>>>,
}

impl HostFunction for EnvironGetFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let environ_ptr = args[0].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let environ_buf_ptr = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        with_linear_memory(memory, &self.memory, |mem| {
            let mut buf_cursor = environ_buf_ptr;
            for (i, var) in self.env.iter().enumerate() {
                let ptr_slot = environ_ptr + i * 4;
                write_i32_le(mem, ptr_slot, buf_cursor as i32)?;

                let bytes = var.as_bytes();
                mem.write_bytes(buf_cursor, bytes)?;
                mem.write_bytes(buf_cursor + bytes.len(), &[0u8])?;
                buf_cursor += bytes.len() + 1;
            }

            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 5. clock_res_get ─────────────────────────────────────────────────────────

/// WASI `clock_res_get(id: i32, resolution_ptr: i32) → errno`
///
/// Writes the clock resolution (in nanoseconds) as an i64 little-endian value
/// at `resolution_ptr`.
///
/// The resolution answers the question: "What is the smallest time difference
/// this clock can distinguish?" For most OS clocks this is 1 ms (1_000_000 ns).
struct ClockResGetFunc {
    clock: Arc<dyn WasiClock>,
    memory: Rc<RefCell<Option<LinearMemory>>>,
}

impl HostFunction for ClockResGetFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let id = args[0].as_i32().map_err(|e| TrapError::new(e.message))?;
        let resolution_ptr = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        let resolution = self.clock.resolution_ns(id);

        with_linear_memory(memory, &self.memory, |mem| {
            write_i64_le(mem, resolution_ptr, resolution)?;
            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 6. clock_time_get ────────────────────────────────────────────────────────

/// WASI `clock_time_get(id: i32, precision: i64, time_ptr: i32) → errno`
///
/// Writes the current time for the requested clock as an i64 (nanoseconds)
/// at `time_ptr`.
///
/// ## Clock IDs
///
/// | id | meaning                                   |
/// |----|-------------------------------------------|
/// |  0 | REALTIME — nanoseconds since Unix epoch   |
/// |  1 | MONOTONIC — nanoseconds since start       |
/// |  2 | PROCESS_CPUTIME — mapped to realtime      |
/// |  3 | THREAD_CPUTIME — mapped to realtime       |
/// | *  | Returns EINVAL (28)                       |
///
/// `precision` is the requested accuracy hint; we ignore it because our clock
/// always returns the best available precision.
struct ClockTimeGetFunc {
    clock: Arc<dyn WasiClock>,
    memory: Rc<RefCell<Option<LinearMemory>>>,
}

impl HostFunction for ClockTimeGetFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32, ValueType::I64, ValueType::I32],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let id = args[0].as_i32().map_err(|e| TrapError::new(e.message))?;
        // args[1] is precision (i64) — ignored.
        let time_ptr = args[2].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        // Map clock IDs to time sources.
        // IDs 0, 2, 3 all map to wall-clock time; ID 1 is monotonic.
        let ns = match id {
            0 | 2 | 3 => self.clock.realtime_ns(),
            1 => self.clock.monotonic_ns(),
            _ => return Ok(vec![WasmValue::I32(WASI_EINVAL)]),
        };

        with_linear_memory(memory, &self.memory, |mem| {
            write_i64_le(mem, time_ptr, ns)?;
            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 7. random_get ────────────────────────────────────────────────────────────

/// WASI `random_get(buf_ptr: i32, buf_len: i32) → errno`
///
/// Fills `buf_len` bytes starting at `buf_ptr` with random bytes from the
/// injected `WasiRandom` implementation.
///
/// The WASI spec says this should be cryptographically secure. Our default
/// `SystemRandom` is NOT crypto-secure — use `WasiConfig::random` to inject
/// a getrandom- or ring-backed implementation if that matters.
struct RandomGetFunc {
    random: Arc<dyn WasiRandom>,
    memory: Rc<RefCell<Option<LinearMemory>>>,
}

impl HostFunction for RandomGetFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        let buf_ptr = args[0].as_i32().map_err(|e| TrapError::new(e.message))? as usize;
        let buf_len = args[1].as_i32().map_err(|e| TrapError::new(e.message))? as usize;

        // Allocate a temporary buffer, fill it, then write to WASM memory.
        let mut buf = vec![0u8; buf_len];
        self.random.fill_bytes(&mut buf);

        with_linear_memory(memory, &self.memory, |mem| {
            mem.write_bytes(buf_ptr, &buf)?;
            Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
        })
    }
}

// ── 8. sched_yield ───────────────────────────────────────────────────────────

/// WASI `sched_yield() → errno`
///
/// Voluntarily yield the CPU to another thread or process.
///
/// In a single-threaded host (this runtime is single-threaded), yielding is a
/// no-op. We return errno 0 to signal success without actually calling
/// `std::thread::yield_now()` because WASM modules must not be able to cause
/// unbounded delays in host scheduling.
struct SchedYieldFunc;

impl HostFunction for SchedYieldFunc {
    fn func_type(&self) -> &FuncType {
        static FT: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        });
        &FT
    }

    fn call(
        &self,
        _args: &[WasmValue],
        _memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError> {
        Ok(vec![WasmValue::I32(WASI_ESUCCESS)])
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// WasmInstance
// ══════════════════════════════════════════════════════════════════════════════

/// A live, executable instance of a WASM module.
///
/// Contains all allocated runtime state and the export lookup table.
pub struct WasmInstance {
    /// The original parsed module.
    pub module: WasmModule,
    /// Every allocated/imported linear memory, in index order
    /// (multi-memory proposal, W16, task #85). Index 0 is "the" default
    /// memory every pre-existing load/store/bulk-memory instruction and
    /// data-segment application still implicitly targets.
    pub memories: Vec<LinearMemory>,
    /// Allocated tables.
    pub tables: Vec<Table>,
    /// Global variable values. `Rc<RefCell<GlobalStorage>>` (W35 third
    /// slice; was `Rc<RefCell<WasmValue>>`) -- see [`GlobalStorage`]'s own
    /// doc comment in `wasm-execution` (design §7) for why a global cell
    /// needs a real payload alongside its `WasmValue`. Still shared, not a
    /// plain owned value (real corpus vendoring pass, `instance.wast`'s
    /// own "Import is not generative" tests / `linking.wast`'s `mut_glob`
    /// tests) -- see `HostInterface::resolve_global`'s own doc comment in
    /// `wasm-execution` for the full cross-instance-sharing rationale,
    /// which mirrors `memories`/`tables` above exactly (W28).
    pub globals: Vec<Rc<RefCell<GlobalStorage>>>,
    /// Global type descriptors.
    pub global_types: Vec<GlobalType>,
    /// All function type signatures.
    pub func_types: Vec<FuncType>,
    /// Each function's own declared type-SECTION index (W33 second slice,
    /// item 4), parallel to `func_types` above (which holds the resolved
    /// `FuncType` SHAPE instead of the index that declared it) -- same
    /// "imports first, then module-defined" combined index space. Needed
    /// to recover a `funcref` value's real nominal type identity: `wasm-
    /// execution`'s `WasmValue::Ref(Some(i))` funcref payload IS the
    /// function index directly, so `func_type_indices[i]` is the only way
    /// to learn which type-section entry that function was declared
    /// with -- see `wasm-execution::WasmExecutionEngine::
    /// set_func_type_indices`'s own doc comment for the runtime consumer
    /// (`call_indirect`'s real subtype check, `ref.cast`/`ref.test`'s
    /// dynamic type check).
    pub func_type_indices: Vec<u32>,
    /// This module's own canonicalized type-group forms (W34 third slice,
    /// `code/specs/W34-wasm-gc-canonical-type-equivalence.md`) -- cloned
    /// once, at `instantiate()` time, from `ValidatedModule::
    /// canonical_types()` (already computed by `wasm-validator::validate`,
    /// which `instantiate()` requires having already succeeded -- see this
    /// struct's own `module` field, and `validate`'s own doc comment).
    /// Same index space as `func_type_indices`/`module.types` (one entry
    /// per flat type-section index). Threaded into `wasm-execution` by
    /// `build_engine` via `WasmExecutionEngine::set_canonical_types`,
    /// mirroring `type_subtyping`/`set_type_subtyping`'s own exact
    /// "parallel slice, not a whole `WasmModule`" pattern -- so `call_
    /// indirect`/`ref.cast`/`ref.test`'s runtime dispatch can use real
    /// canonical equivalence, not just the nominal `sub`-chain, without
    /// `wasm-execution::WasmExecutionContext` ever needing to hold a full
    /// `WasmModule` (see that struct's own doc comments on `types`/
    /// `func_types` for why it deliberately doesn't).
    pub canonical_types: Vec<Option<(Rc<CanonicalGroup>, u32)>>,
    /// Function bodies (None for imports).
    pub func_bodies: Vec<Option<FunctionBody>>,
    /// Resolved imported host functions. `Rc`, not `Box` (W35 first slice:
    /// `code/specs/W35-wasm-cross-instance-function-identity.md`) — mirrors
    /// `wasm-execution`'s own `host_functions` field, so a `FuncRefTarget`
    /// can eventually clone an existing import's callable cheaply instead
    /// of rebuilding it. `HostInterface::resolve_function` itself still
    /// returns `Box<dyn HostFunction>` (unchanged); `Rc::from(..)` converts
    /// at the one call site that resolves an import (below).
    pub host_functions: Vec<Option<Rc<dyn HostFunction>>>,
    /// Combined imported + module-defined tag index space (W-next), each
    /// entry a TYPE index into `func_types`... no -- into THIS instance's
    /// own `module.types` (the tag's declared param/result signature).
    /// Unlike `module.tags` (which, like `module.functions`, holds only
    /// module-DEFINED tags -- imports live separately in `module.
    /// imports`), this field is the FULL combined space, "imports first,
    /// then module-defined", matching every other index space
    /// (`func_types`/`global_types`/etc.) on this struct. See
    /// `instantiate()`'s own construction and `wasm-validator`'s
    /// identically-shaped `tag_types` in `type_check.rs::
    /// build_module_context` (which this mirrors, just carrying type
    /// INDICES here rather than resolved `FuncType`s, since `wasm-
    /// execution::WasmExecutionContext::tags` wants the former).
    pub tags: Vec<u32>,
    /// Canonical, cross-instance-safe tag identity per tag (W23), same
    /// combined index space as `tags` above: `tag_identities[N]` is tag
    /// `N`'s real identity. A module-DEFINED tag gets a freshly minted,
    /// never-repeating identity (from the process-wide [`NEXT_TAG_IDENTITY`]
    /// counter) exactly ONCE, at `instantiate()` time — unlike
    /// `wasm_execution::WasmExecutionContext::instance_id` (reminted every
    /// top-level call), this must survive across every later call on the
    /// SAME instance, since the whole point is that the same real tag
    /// keeps comparing equal to itself. An IMPORTED tag adopts the
    /// identity [`HostInterface::resolve_tag`] returns for it verbatim
    /// (the exporting instance's own already-minted identity), rather
    /// than minting an unrelated new one — this is what lets a `throw` in
    /// one module instance be caught by a `try_table` in another that
    /// imported the SAME tag (see `code/specs/
    /// W23-wasm-exceptions-cross-instance-tag-identity.md`). Threaded into
    /// the execution engine by `build_engine` via
    /// `wasm_execution::WasmExecutionEngine::set_tag_identities`, mirroring
    /// `tags`/`set_tags` exactly.
    pub tag_identities: Vec<u64>,
    /// Export map: name -> (kind, index).
    pub exports: Vec<(String, ExternalKind, u32)>,
    /// Persistent v128 (SIMD) value storage for this instance's whole
    /// lifetime -- see `code/specs/W15-wasm-v128-persistent-storage.md`.
    /// `WasmValue::V128(handle)` is an index into this `Vec`; without it
    /// living here, a v128-typed global's handle would go stale the
    /// moment one call ends (the old bug this field fixes). Index 0 is
    /// permanently reserved as the all-zero entry, matching
    /// `wasm_execution::WasmExecutionContext::v128_heap`'s own
    /// convention. `build_engine`/`call_engine`/`call_engine_with_v128`
    /// clone/restore it exactly like `globals`.
    pub v128_heap: Vec<[u8; 16]>,
    /// Persistent GC object (struct/array) heap for this instance's whole
    /// lifetime (W33 fourth slice) -- same "must survive past the call
    /// that created it" reasoning as `v128_heap` above, for a GLOBAL
    /// initializer that itself allocates a struct/array (`struct.wast`'s
    /// own `(global (ref $s) (struct.new $s ...))`, later read back via
    /// `global.get` from a SEPARATE, later `call()`). `build_engine`/
    /// `call_engine` clone/restore it exactly like `v128_heap`.
    pub gc_heap: Vec<Option<GcObject>>,
    /// Per-data-segment "already dropped" flags for this instance's whole
    /// lifetime (task #95) -- same index space as `module.data`, same
    /// persistent-across-calls shape as `v128_heap` above (`data.drop`'s
    /// effect from one call must still be visible in a later one).
    /// Initialized all-`false`, one entry per `module.data`, at
    /// instantiation time (see `instantiate()` below); `call_engine`
    /// threads it into/out of `wasm_execution::WasmExecutionEngine`
    /// exactly like `v128_heap` is.
    pub dropped_data_segments: Vec<bool>,
    /// Per-element-segment "already dropped" flags for this instance's
    /// whole lifetime (task #97) -- same index space as `module.elements`,
    /// same persistent-across-calls shape and reasoning as
    /// `dropped_data_segments` above (`elem.drop`'s effect from one call
    /// must still be visible in a later one). Initialized all-`false`, one
    /// entry per `module.elements`, at instantiation time; `call_engine`
    /// threads it into/out of `wasm_execution::WasmExecutionEngine` exactly
    /// like `dropped_data_segments` is.
    pub dropped_elements: Vec<bool>,
    /// Canonical, cross-instance-safe function identity per combined
    /// func-index-space entry (W35 third slice: `code/specs/
    /// W35-wasm-cross-instance-function-identity.md`, design §2) --
    /// mirrors [`tag_identities`](Self::tag_identities)'s own construction
    /// loop in `instantiate()` EXACTLY, sharing the SAME process-wide
    /// [`NEXT_TAG_IDENTITY`] counter (tags and functions are never
    /// compared against each other, so sharing one counter is harmless
    /// and avoids a second `AtomicU64` -- the spec's own explicit
    /// reasoning). A module-DEFINED function mints a fresh identity; an
    /// IMPORTED function adopts `host_func.identity()` verbatim (via
    /// [`HostInterface::resolve_function`], per `wasm_execution::
    /// HostFunction::identity`'s own doc comment). Threaded into
    /// `wasm-execution` via [`wasm_execution::WasmExecutionEngine::
    /// set_func_identities`], mirroring `set_tag_identities`'s exact
    /// shape.
    pub func_identities: Vec<u64>,
    /// This `WasmInstance`'s own process-wide-unique identity (W35 third
    /// slice, a further deviation from the spec's own literal design --
    /// not named anywhere in its text). Minted ONCE, at `instantiate()`
    /// time, from the dedicated [`NEXT_INSTANCE_IDENTITY`] counter (kept
    /// separate from `NEXT_TAG_IDENTITY`/its own function-identity reuse:
    /// this identifies a whole INSTANCE, a materially different kind of
    /// thing from a tag or a function, so sharing either counter would be
    /// a coincidence, not a deliberate design choice the way
    /// `func_identities` sharing `tag_identities`'s counter is).
    ///
    /// Exists to let `wasm-execution`'s `dispatch_resolved_func_ref`
    /// (via `effective_local_index`) tell "is `FuncRefTarget::local_index`
    /// meaningful for the ctx dispatching it right now" without
    /// `wasm-execution` ever naming `WasmInstance` -- see
    /// `wasm_execution::FuncRefTarget::owner_instance_identity`'s own doc
    /// comment for the full rationale (a measured `even`/`odd`
    /// recursion-depth regression this exists to avoid, while still
    /// fixing the real cross-instance dispatch bug). Threaded into
    /// `wasm-execution` via `wasm_execution::WasmExecutionEngine::
    /// set_instance_identity`, called unconditionally by `build_engine`
    /// (a plain `u64` copy -- unlike `SelfFunctionResolver` itself, this
    /// needs no live `Rc<RefCell<WasmInstance>>` at all).
    pub instance_identity: u64,
    /// `(table_index, offset, count)` for every entry an ACTIVE elem
    /// segment wrote during THIS instance's own `instantiate()` call
    /// (W35 fourth slice, security-review finding) -- the exact,
    /// precisely-bounded set of table slots [`resolve_all_table_funcrefs`]
    /// is safe to resolve, in THIS instance's own combined function-index
    /// space. See that function's own doc comment for why this replaced
    /// an earlier, less precise "scan every currently-`Raw` entry"
    /// design: a table's `Raw` entries are not exclusively "unresolved
    /// writes this instance just made" -- a LIVE `table.init`/`table.set`
    /// on some OTHER, already-registered instance sharing the same table
    /// can also leave one there, at any later point, which a scan-based
    /// fixup could not tell apart from this instance's own write.
    pub active_elem_writes: Vec<(u32, u32, u32)>,
}

/// Process-wide counter minting a fresh, never-repeating canonical tag
/// identity (W23) each time `instantiate()` builds a module-DEFINED tag's
/// entry in `WasmInstance::tag_identities`. Starts at `1` so `0` stays
/// reserved as "no real identity assigned" (mirroring
/// `wasm_execution::WasmExecutionContext::instance_id`'s own `0`-reserved
/// convention) -- see that field's own doc comment for why a tag's
/// identity must be minted once per real DEFINITION (persisting across
/// every call on the same instance), not once per call. `Relaxed`
/// ordering suffices: uniqueness, not cross-thread visibility of any
/// OTHER state, is the only property tag-identity comparison relies on.
///
/// **W35 third slice**: also mints [`WasmInstance::func_identities`]'
/// module-DEFINED entries -- the spec's own design §2 explicitly calls
/// for sharing this one counter between tags and functions ("tags and
/// functions are never compared against each other, so sharing one
/// counter is harmless").
static NEXT_TAG_IDENTITY: AtomicU64 = AtomicU64::new(1);

/// Process-wide counter minting a fresh, never-repeating
/// [`WasmInstance::instance_identity`] each time `instantiate()` builds a
/// new instance (W35 third slice, further deviation from the spec's own
/// literal text -- see that field's own doc comment for the full
/// rationale). Starts at `1` so `0` stays reserved as "no real identity
/// assigned," mirroring `NEXT_TAG_IDENTITY`'s own convention. Kept
/// deliberately SEPARATE from `NEXT_TAG_IDENTITY` (unlike
/// `func_identities`, which deliberately DOES share it) -- an instance
/// identity and a tag/function identity are never compared against each
/// other either, but there is no analogous "spec explicitly calls for
/// sharing" reasoning for this one, so a dedicated counter keeps its own
/// numbering independent and easier to reason about in isolation.
static NEXT_INSTANCE_IDENTITY: AtomicU64 = AtomicU64::new(1);

/// Build a link-error `TrapError` for a failed import (WASM05/W10) --
/// self-authored, capability-gap-shaped text (this crate's existing
/// convention), naming the exact import that failed.
fn link_error(reason: &str, imp: &Import) -> TrapError {
    TrapError::new(format!("{reason}: {}.{}", imp.module_name, imp.name))
}

/// The real spec's own limits-compatibility rule for a resolved import:
/// the *actual* min must be at least the *declared* min, and if the
/// *declared* side has a max, the actual side must too, and it must not
/// exceed the declared one. This is a subset check, not equality --
/// `(memory 1 1)` can satisfy an import declared as `(memory 1)` (no
/// max), but not the reverse.
fn limits_compatible(actual: &Limits, declared: &Limits) -> bool {
    if actual.min < declared.min {
        return false;
    }
    match declared.max {
        None => true,
        Some(declared_max) => matches!(actual.max, Some(actual_max) if actual_max <= declared_max),
    }
}

/// Build the three flat-type-index-keyed tables `wasm-execution` needs for
/// real struct/array runtime semantics (W33 fourth slice): field counts,
/// per-field storage types, and per-array-type element storage. Shared by
/// `instantiate()` (for `evaluate_const_expr_gc`, evaluating a GLOBAL
/// initializer that itself allocates) and `build_engine` (for the engine's
/// own `struct.new`/`array.new` handlers) so the two never drift apart.
///
/// Built on `WasmModule::struct_type_at`/`array_type_at` (`type_kinds`-aware,
/// see those methods' own doc comments) rather than the OLD "pad
/// `func_type_count` zeros, then append every struct's field count in
/// `struct_types` order" scheme this function replaces — that old scheme's
/// own assumption, "struct types follow ALL function types," is exactly
/// what a TEXT-format module (via `wasm-wast-parser`'s real struct/array
/// declarations) is free to violate: `struct.wast`'s/`array.wast`'s own
/// "Binding structure" modules declare a struct/array type, THEN a function
/// whose inline-only signature gets dedup'd into `types` AFTER it — see
/// `wasm_types::TypeKind`'s own doc comment for the full mechanism.
/// Iterating every flat index directly is correct regardless of declaration
/// order — the LANG77/Twig binary-only modules the old scheme targeted
/// (which never populate `type_kinds` at all) still resolve identically via
/// `struct_type_at`'s legacy-offset fallback, so this is a strict
/// generalization, not a behavior change for any pre-existing caller.
///
/// Returns three EMPTY vectors when the module declares no struct/array
/// type at all — callers should treat that as "leave `wasm-execution`'s own
/// tables unset," not "call the setters with empty vectors," since an empty
/// table changes what an out-of-range `struct.new`/`array.new` does (see
/// `build_engine`'s own call site for why that distinction matters).
fn struct_array_runtime_tables(
    module: &WasmModule,
) -> (Vec<u32>, Vec<Vec<wasm_types::StorageType>>, Vec<Option<wasm_types::StorageType>>) {
    if module.struct_types.is_empty() && module.array_types.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let total_type_count = module
        .type_kinds
        .len()
        .max(module.types.len() + module.struct_types.len() + module.array_types.len());
    let mut struct_field_counts = Vec::with_capacity(total_type_count);
    let mut struct_field_storage = Vec::with_capacity(total_type_count);
    let mut array_element_storage = Vec::with_capacity(total_type_count);
    for idx in 0..total_type_count as u32 {
        match module.struct_type_at(idx) {
            Some(st) => {
                struct_field_counts.push(st.fields.len() as u32);
                struct_field_storage.push(st.fields.iter().map(|f| f.storage).collect());
            }
            None => {
                struct_field_counts.push(0);
                struct_field_storage.push(Vec::new());
            }
        }
        array_element_storage.push(module.array_type_at(idx).map(|at| at.element.storage));
    }
    (struct_field_counts, struct_field_storage, array_element_storage)
}

// ══════════════════════════════════════════════════════════════════════════════
// LocalFunctionRef (W35 third slice)
// ══════════════════════════════════════════════════════════════════════════════

/// Wraps ONE of `instance`'s own functions (exported or not) as a
/// [`HostFunction`], callable by raw combined INDEX rather than by export
/// name (W35 third slice, design §3) -- the "wrap MY OWN local function"
/// counterpart to `wasm-conformance`'s `CrossModuleFunction` (which wraps
/// ANOTHER instance's EXPORTED function, called by name). Needed because
/// an active `elem` segment, or a `ref.func`-initialized global, can name
/// a function that is never exported at all (`linking.wast`'s own
/// `$Mt`/`$g` -- see `code/specs/
/// W35-wasm-cross-instance-function-identity.md`'s own root-cause trace)
/// yet still needs a real, self-contained, cross-instance-safe identity
/// the moment it's written into a table/global another instance can
/// later read.
///
/// Fields mirror `wasm-conformance::CrossModuleFunction`'s own snapshot-
/// at-construction pattern (see [`resolve_func_ref_for_instance`]'s own
/// doc comment for exactly how each is derived).
struct LocalFunctionRef {
    instance: Rc<RefCell<WasmInstance>>,
    func_index: u32,
    func_type: FuncType,
    identity: u64,
    /// W33 first slice pattern, mirroring `CrossModuleFunction`'s own
    /// `group_shape` field -- see `HostFunction::type_group_shape`'s own
    /// doc comment.
    group_shape: (u32, u32),
    /// Mirrors `CrossModuleFunction`'s own `is_final` field -- see
    /// `HostFunction::is_final`'s own doc comment.
    is_final: bool,
    /// W34 fourth slice pattern, mirroring `CrossModuleFunction`'s own
    /// `canonical_type` field -- see `HostFunction::canonical_type`'s own
    /// doc comment.
    canonical_type: Option<(Rc<CanonicalGroup>, u32)>,
}

impl HostFunction for LocalFunctionRef {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }

    fn identity(&self) -> u64 {
        self.identity
    }

    fn type_group_shape(&self) -> (u32, u32) {
        self.group_shape
    }

    fn is_final(&self) -> bool {
        self.is_final
    }

    fn canonical_type(&self) -> Option<(Rc<CanonicalGroup>, u32)> {
        self.canonical_type.clone()
    }

    /// Mirrors `CrossModuleFunction::canonically_matches` exactly, just
    /// climbing THIS (the LOCAL, not a foreign) instance's own
    /// `type_subtyping` chain -- a declared `sub` relationship is only
    /// ever meaningful within the module that declared it.
    fn canonically_matches(&self, target: &(Rc<CanonicalGroup>, u32), budget: &mut wasm_types::CrossModuleComparisonBudget) -> bool {
        let instance = self.instance.borrow();
        match instance.func_type_indices.get(self.func_index as usize) {
            Some(&type_idx) => wasm_types::canonical_chain_reaches(&instance.module.type_subtyping, &instance.canonical_types, type_idx, Some(target), budget),
            None => false,
        }
    }

    /// Calls the new by-index primitive from design §3
    /// ([`WasmRuntime::call_by_index`]), not `call_typed`
    /// (`CrossModuleFunction`'s own shape) -- `func_index` here need not
    /// even be exported (`linking.wast`'s own `$g` example).
    ///
    /// **W35 fourth slice, security-review finding**: `try_borrow_mut`,
    /// NOT a plain `borrow_mut()` that panics on conflict. Before this
    /// slice, a `LocalFunctionRef` was never actually reachable from
    /// production code at all (`resolve_func_ref_for_instance`'s own
    /// doc comment: "not currently called by `instantiate()`"), so this
    /// path never ran. This slice's own fixup pass makes it reachable —
    /// and a REAL, deterministic, ordinary (non-circular) linking pattern
    /// reaches it with `self.instance` ALREADY mutably borrowed: instance
    /// `B` calls into instance `A` (an ordinary, already-tested cross-
    /// module `call`, holding `B`'s own `Rc<RefCell<WasmInstance>>`
    /// borrowed for the whole call); `A`'s own `call_indirect` reads a
    /// table entry `B` earlier wrote (a `LocalFunctionRef` targeting
    /// `B` itself); `A`'s own `effective_local_index` can't find `B`'s
    /// function in `A`'s own `func_identities` (it isn't imported by
    /// `A`), so dispatch falls through to `callable.call(..)` here --
    /// re-entering `B`'s own, ALREADY-borrowed `Rc<RefCell<WasmInstance>>`.
    /// This is NOT the pre-existing, accepted "genuinely mutual
    /// cross-instance cycle" risk (`CrossModuleFunction`'s own doc
    /// comment) -- `B` calling `A` once, with `A` merely dispatching a
    /// stored reference back to `B`, is a completely ordinary linking
    /// shape, not a deliberately-constructed cycle. A bare `borrow_mut()`
    /// here would be a NEW, easily-triggered process panic (a real DoS
    /// against the embedding host, and specifically a real diagnostic
    /// gap for this crate's own conformance harness: an entire report run
    /// aborting instead of one directive grading `Fail`/`Trap`) on an
    /// ordinary corpus pattern, not the malicious-cycle corner case. A
    /// borrow conflict becomes a clean, catchable `TrapError` instead --
    /// this repo's own "never trade loud for silent" convention cuts the
    /// OTHER way here: a Rust panic is technically "loud," but it is an
    /// uncatchable process abort in this crate's own callers (a batch
    /// report run, or a real host embedding this interpreter), which is
    /// WORSE than a gracefully reported trap, not better.
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        let mut instance = self.instance.try_borrow_mut().map_err(|_| {
            TrapError::new(
                "cross-instance funcref dispatch failed: the target instance is already executing \
                 (a re-entrant call back into an instance already on the call stack) -- this trap, not a panic, \
                 is the correct failure mode for this shape"
                    .to_string(),
            )
        })?;
        WasmRuntime::new().call_by_index(&mut instance, self.func_index as usize, args)
    }
}

/// Resolve a combined-index-space function index into a real,
/// self-contained [`FuncRefTarget`] for `instance` (W35 third slice) --
/// mirrors `wasm_execution::WasmExecutionContext::resolve_function_ref`'s
/// own import/local split exactly, but usable BEFORE any
/// `WasmExecutionContext` exists at all.
///
/// **Not currently called by `instantiate()` itself** -- see that
/// function's own doc comment for why: a `LocalFunctionRef` this produces
/// for a LOCAL function holds `instance: instance_rc.clone()`, which is
/// only sound to construct when `instance_rc` is a genuinely long-lived,
/// permanent `Rc` (e.g. `wasm-conformance`'s `ModuleRegistry`, held for a
/// whole script's lifetime) -- `instantiate()` itself has no such
/// permanent home to offer, so calling this from inside it creates an
/// unavoidable self-referential `Rc` cycle the moment a module's own
/// elem/global entry references its own local function (the common
/// case), breaking `instantiate()`'s own "return a bare, owned
/// `WasmInstance`" contract. This function remains real, tested,
/// additive infrastructure (see this crate's own new unit tests) --
/// ready for slice 4 to call from `wasm-conformance`'s `ModuleRegistry`,
/// which CAN safely sustain the resulting cycle (per the spec's own
/// "Security and lifetime consideration" section). This is also the ONE
/// place `InstanceSelfResolver::resolve_local_function` (below)
/// delegates to, so both call sites share identical resolution logic.
///
/// An IMPORTED function (`instance.host_functions[func_index]` is `Some`)
/// is already cross-instance-safe -- clone its `Rc` and reuse its own
/// `identity()` verbatim, same as `resolve_function_ref`'s own import
/// branch. A LOCAL (module-defined) function is wrapped as a
/// [`LocalFunctionRef`], its `group_shape`/`is_final`/`canonical_type`
/// snapshotted from `instance.func_type_indices[func_index]`'s own
/// type-section entry -- the SAME "same snapshot-at-construction pattern
/// `CrossModuleFunction` already uses" the spec's own design §3 calls
/// for.
///
/// `local_index: Some(func_index)` in BOTH branches (a further, documented
/// deviation from the spec's own literal §4 text -- see
/// `wasm_execution::FuncRefTarget::owner_instance_identity`'s own doc
/// comment for the full rationale); `owner_instance_identity` is what
/// actually distinguishes the two cases' dispatch safety now.
pub fn resolve_func_ref_for_instance(instance_rc: &Rc<RefCell<WasmInstance>>, func_index: u32) -> Result<FuncRefTarget, TrapError> {
    let instance = instance_rc.borrow();
    if let Some(Some(hf)) = instance.host_functions.get(func_index as usize) {
        return Ok(FuncRefTarget {
            identity: hf.identity(),
            callable: hf.clone(),
            local_index: Some(func_index),
            // W35 fourth slice, a real gap this slice's own corpus
            // verification found (NOT `None`, contrary to this field's own
            // doc comment as slice 3 left it): `func_index` here is only
            // meaningful as "`ctx.host_functions[func_index]`" within THIS
            // resolving instance's own combined index space -- it is NOT
            // universally safe "in whatever ctx currently holds it" the
            // way that comment claims, once this target can be written
            // into a table/global SHARED with a genuinely different
            // instance (exactly what this slice's own fixup pass, unlike
            // slices 1-3, now does). Reproduced directly against
            // `linking.wast`'s own `$Ot` example: `$Ot` imports `$Mt`'s
            // `h` export as `$Ot`'s OWN combined-index-space slot 0, then
            // writes it (via `$Ot`'s own active elem segment) into
            // `$Mt`'s SHARED table at index 2. With `owner_instance_
            // identity: None` here, `$Mt`'s own later `call_indirect`
            // through that SAME table slot (`$Mt`'s own ctx, whose
            // `instance_identity` differs from `$Ot`'s) incorrectly
            // trusted `local_index: Some(0)` as ITS OWN slot 0 -- but
            // `$Mt` has NO imports, so `$Mt`'s own slot 0 is `$g`, a
            // COMPLETELY DIFFERENT function -- producing a confirmed,
            // silent WRONG ANSWER (`(assert_return (invoke $Mt "call"
            // (i32.const 2)) (i32.const -4))` returned `4` instead).
            // Tagging the RESOLVING instance's own identity here (mirroring
            // the LOCAL-function branch below exactly) makes
            // `effective_local_index` correctly fall through to
            // `target.callable.call(..)` -- the real, already-resolved
            // `CrossModuleFunction`/host function, safe to invoke from ANY
            // instance -- whenever a genuinely different instance's ctx
            // dispatches this target, while still taking the cheap
            // `local_index` path (byte-for-byte unchanged performance) for
            // every SAME-instance read, which is every case any
            // pre-slice-4 test ever reached.
            owner_instance_identity: Some(instance.instance_identity),
        });
    }
    let func_type = instance
        .func_types
        .get(func_index as usize)
        .cloned()
        .ok_or_else(|| TrapError::new(format!("undefined function {func_index} referenced by a funcref")))?;
    let identity = instance.func_identities.get(func_index as usize).copied().unwrap_or(0);
    let (group_shape, is_final, canonical_type) = match instance.func_type_indices.get(func_index as usize) {
        Some(&type_idx) => (
            instance.module.type_group_shape(type_idx),
            instance.module.type_subtyping_at(type_idx).is_final,
            instance.canonical_types.get(type_idx as usize).cloned().flatten(),
        ),
        None => ((1, 0), true, None),
    };
    let owner = instance.instance_identity;
    drop(instance);
    let local_ref = LocalFunctionRef {
        instance: instance_rc.clone(),
        func_index,
        func_type,
        identity,
        group_shape,
        is_final,
        canonical_type,
    };
    Ok(FuncRefTarget {
        identity,
        callable: Rc::new(local_ref),
        local_index: Some(func_index),
        owner_instance_identity: Some(owner),
    })
}

/// The declared element-type TAG (`wasm_types::TableType::element_type`)
/// for the COMBINED-index-space table `index` in `instance` (W35 fourth
/// slice) -- mirrors `combined_function_type_idx`-style helpers already
/// used at this crate's own call sites, just for tables' "imports first,
/// then module-defined" index space instead of functions'. `None` at an
/// out-of-range index (never expected for a validated module, but handled
/// without panicking).
///
/// **Why this matters for [`resolve_all_table_funcrefs`] below**: `Table`/
/// `TableStorage` (`wasm-execution`) do NOT track their own declared
/// element type at runtime -- an EXTERNREF (or any other non-funcref
/// reference) table's entries are `TableElement::Raw(u32)` for a
/// completely different reason than an unresolved FUNCREF entry is
/// (`TableElement`'s own doc comment: "the ONLY variant a non-funcref ...
/// table entry ever uses"). Reproduced directly during this slice's own
/// corpus verification: resolving every `Raw` entry in every table
/// unconditionally corrupted `elem.wast`'s own "Initializing a table with
/// an externref-type element segment" test -- a real `(ref.extern 42)`
/// value, stored as `Raw(42)`, got reinterpreted as function index `42`.
fn combined_table_element_type(instance: &WasmInstance, index: u32) -> Option<u8> {
    let imported_table_count = instance.module.imports.iter().filter(|i| i.kind == ExternalKind::Table).count() as u32;
    if index < imported_table_count {
        instance
            .module
            .imports
            .iter()
            .filter(|i| i.kind == ExternalKind::Table)
            .nth(index as usize)
            .and_then(|imp| match &imp.type_info {
                ImportTypeInfo::Table(tt) => Some(tt.element_type),
                _ => None,
            })
    } else {
        instance.module.tables.get((index - imported_table_count) as usize).map(|tt| tt.element_type)
    }
}

/// This codebase's single-byte encoding for a FUNCREF-family table
/// (`wasm_types::TableType::element_type`'s own doc comment: "every
/// concrete function reference is funcref-family"). See
/// `combined_table_element_type`'s own doc comment.
const FUNCREF_ELEMENT_TYPE: u8 = 0x70;

/// The "resolution fixup pass" (W35 fourth slice, `code/specs/
/// W35-wasm-cross-instance-function-identity.md`) for TABLES specifically:
/// resolve every `TableElement::Raw` entry `instance`'s own
/// `active_elem_writes` names (owned or imported table -- see the "why
/// ALL tables" note below) into a real, cross-instance-safe
/// `TableElement::Func`, via [`resolve_func_ref_for_instance`]. `pub` so
/// both this crate's own `instantiate()` (its error path -- see that
/// function's own doc comment) and `wasm-conformance`'s `ModuleRegistry`-
/// driven post-registration fixup call the exact same logic, rather than
/// maintaining two independent copies that could silently drift apart.
///
/// **Why driven by `active_elem_writes` (a precise, recorded list),
/// NOT a scan for `TableElement::Raw` entries** -- a security-review
/// finding, not the original design: an earlier version of this pass
/// scanned every currently-`Raw` entry in every table `instance.tables`
/// exposes, on the theory that "the only `Raw` entries left, by the time
/// `instance`'s own fixup runs, are ones `instance` itself just wrote."
/// That theory is FALSE in general: a LIVE `table.init`/`table.set`/
/// `table.fill`/`table.grow` on some OTHER, ALREADY-registered instance
/// sharing the SAME table can leave a `Raw` entry there too, at any LATER
/// point in the script -- well after `instance`'s own fixup already ran.
/// A scan-based fixup running for a THIRD instance that merely imports
/// that same table (without writing to it) could not tell such a
/// foreign, stale entry apart from one it should resolve using its OWN
/// context, misattributing it to the WRONG instance's combined
/// function-index space -- a silent wrong dispatch, exactly the bug
/// class this whole spec exists to fix, not reintroduce. Driving strictly
/// off `active_elem_writes` (populated ONLY by `instantiate()`'s own
/// active-elem-segment application, at the exact moment it happens, for
/// THIS instance alone) makes that misattribution structurally
/// impossible: no scanning, no guessing about provenance, only the exact
/// slots this instance's own instantiate() call is certain it just wrote.
///
/// **Why ALL tables, not just ones `instance` itself DECLARES**: a
/// module can write into an IMPORTED (shared) table via its own active
/// elem segment, in ITS OWN combined function-index space --
/// `linking.wast`'s own `$Ot` (imports `$Mt`'s `"tab"`, then overwrites
/// two of its entries via `$Ot`'s OWN `elem`) is exactly this shape. An
/// "owned tables only" fixup would never touch the entries `$Ot` itself
/// just wrote (imported, not owned, from `$Ot`'s perspective), leaving
/// them `Raw` forever -- `active_elem_writes` names the TABLE INDEX
/// directly (regardless of ownership), so this is handled for free.
///
/// **Why funcref-typed tables only**: see `combined_table_element_type`'s
/// own doc comment -- an externref table's `Raw` entries are a real,
/// opaque payload, never a function index, and must never be
/// reinterpreted as one. (Belt-and-suspenders here: `wasm-validator`
/// already guarantees an elem segment only ever targets a funcref-family
/// table, so this check should never actually trigger in practice for an
/// entry `active_elem_writes` names -- kept anyway as a second,
/// independent line of defense against exactly the class of bug this
/// function exists to prevent.)
///
/// **Why NOT globals too** (a deliberate narrowing from this pass's own
/// original design): see `wasm-conformance::resolve_owned_funcrefs`'s own
/// doc comment for the full, reproduced regression (`return_call_ref.
/// wast`'s own deep tail-recursion tests exhausting `wasm-execution`'s
/// `func_ref_heap` once a funcref-typed global's `func_ref` becomes
/// `Some`, since `global.get` mints a FRESH heap handle on every read
/// with no `owner_instance_identity`-style same-instance fast path).
/// Since no vendored corpus file needs cross-instance funcref-GLOBAL
/// resolution at all (confirmed by direct grep, per the spec's own text),
/// this pass is scoped to tables only.
pub fn resolve_all_table_funcrefs(instance_rc: &Rc<RefCell<WasmInstance>>) -> Result<(), TrapError> {
    // One short, shared borrow to snapshot everything needed -- dropped
    // before any resolution call runs, so `resolve_func_ref_for_instance`'s
    // own (also shared) `.borrow()` never overlaps a MUTABLE borrow of
    // `instance_rc` here. Never calls `instance_rc.borrow_mut()` at all:
    // the actual mutation goes through `Table`'s OWN inner
    // `Rc<RefCell<TableStorage>>` (a DIFFERENT `RefCell`), exactly like a
    // live `table.set` opcode handler already does.
    let (tables, writes) = {
        let instance = instance_rc.borrow();
        (instance.tables.clone(), instance.active_elem_writes.clone())
    };

    for (table_index, offset, count) in writes {
        let Some(table) = tables.get(table_index as usize) else {
            continue; // defensive only -- `instantiate()` never records an out-of-range table index
        };
        if combined_table_element_type(&instance_rc.borrow(), table_index) != Some(FUNCREF_ELEMENT_TYPE) {
            continue;
        }
        for slot in offset..offset.saturating_add(count) {
            if let Some(TableElement::Raw(func_idx)) = table.get(slot)? {
                let target = resolve_func_ref_for_instance(instance_rc, func_idx)?;
                // `Table` is `Clone` over a shared `Rc<RefCell<TableStorage>>`
                // (W28) -- mutating THIS local clone's inner storage is
                // observable through every other clone, no `&mut
                // WasmInstance` needed at all.
                let mut t = table.clone();
                t.set(slot, Some(TableElement::Func(target)))?;
            }
        }
    }

    Ok(())
}

/// Real [`SelfFunctionResolver`] implementation (W35 third slice, design
/// §4), closing over the `Rc<RefCell<WasmInstance>>` under construction --
/// delegates entirely to [`resolve_func_ref_for_instance`], so its own
/// resolution logic is exercised identically whether reached through this
/// trait impl or called directly.
///
/// **Not installed anywhere by this slice's own production code** --
/// neither by `instantiate()` (see that function's own doc comment: doing
/// so would create an unavoidable self-referential `Rc` cycle for the
/// common "elem/global entry references its own local function" case,
/// breaking `instantiate()`'s "return a bare, owned `WasmInstance`"
/// contract) nor by `build_engine` for ordinary per-call execution (see
/// that method's own doc comment: it only ever has a plain `&mut
/// WasmInstance`, never a live `Rc<RefCell<WasmInstance>>`, to construct
/// one from). Exercised directly by this slice's own unit tests
/// (constructing one by hand over a LONG-LIVED, test-owned `Rc<RefCell<
/// WasmInstance>>` and installing it via `set_self_resolver` on a
/// hand-built `WasmExecutionEngine`), proving the trait/setter machinery
/// works end-to-end and is ready for slice 4 to connect to a real,
/// permanently-alive `Rc` (`wasm-conformance`'s own `ModuleRegistry`).
pub struct InstanceSelfResolver {
    pub instance: Rc<RefCell<WasmInstance>>,
}

impl SelfFunctionResolver for InstanceSelfResolver {
    fn resolve_local_function(&self, func_index: u32) -> Result<FuncRefTarget, VMError> {
        resolve_func_ref_for_instance(&self.instance, func_index).map_err(|e| VMError::GenericError(e.to_string()))
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// WasmRuntime
// ══════════════════════════════════════════════════════════════════════════════

/// Complete WebAssembly 1.0 runtime.
///
/// Composes the parser, validator, and execution engine into a single
/// user-facing API.
///
/// ## Example
///
/// ```rust,ignore
/// let runtime = WasmRuntime::new();
/// let result = runtime.load_and_run(&wasm_bytes, "square", &[5]);
/// assert_eq!(result.unwrap(), vec![25]);
/// ```
pub struct WasmRuntime {
    host: Option<Box<dyn HostInterface>>,
}

impl WasmRuntime {
    /// Create a new runtime with no host interface.
    pub fn new() -> Self {
        WasmRuntime { host: None }
    }

    /// Create a new runtime with a host interface for import resolution.
    pub fn with_host(host: Box<dyn HostInterface>) -> Self {
        WasmRuntime { host: Some(host) }
    }

    /// Parse a .wasm binary into a WasmModule.
    pub fn load(&self, wasm_bytes: &[u8]) -> Result<WasmModule, String> {
        WasmModuleParser::parse(wasm_bytes).map_err(|e| format!("{}", e))
    }

    /// Validate a parsed module.
    pub fn validate(&self, module: &WasmModule) -> Result<ValidatedModule, ValidationError> {
        validate(module)
    }

    /// Instantiate a parsed module into a live instance.
    ///
    /// Fails with a link error -- distinguishable from a runtime
    /// `TrapError` only by its message text (this crate's existing
    /// convention of self-authored, capability-gap-shaped error text
    /// rather than a new error type every caller would need to match on)
    /// -- when any import can't be resolved by the host, or resolves to
    /// something whose actual type doesn't satisfy the module's declared
    /// import type. Earlier, `instantiate` never failed on an import: an
    /// unresolved function just got pushed as `None` (failing later, at
    /// *call* time, only if that specific import was ever invoked), and
    /// an unresolved memory/table/global silently fabricated a default
    /// value from the *declared* type instead of erroring. See
    /// `code/specs/W10-wasm-real-linking-and-unlinkable.md`.
    ///
    /// Takes a [`ValidatedModule`], not a raw [`WasmModule`] (task #100,
    /// security review follow-up to task #96): this crate's own
    /// `ValidatedModule` doc comment always documented the INTENT that
    /// "downstream code (the runtime) can accept `ValidatedModule` instead
    /// of `WasmModule` to ensure validation is never accidentally
    /// skipped", but `instantiate` never actually enforced it -- it took a
    /// plain `&WasmModule` and never called `validate()` itself, so every
    /// `validate()` check (including the memory/table allocation caps
    /// added for task #96) was silently bypassable by any caller who
    /// called `instantiate()` directly. Requiring `&ValidatedModule` here
    /// makes that guarantee a compile-time fact instead of a caller
    /// convention: call `WasmRuntime::validate()` first, matching the
    /// pattern `wasm-conformance`'s harness already used.
    pub fn instantiate(&self, validated: &ValidatedModule) -> Result<WasmInstance, TrapError> {
        let module = validated.module();
        // W34 third slice: cloned once, here, from `validated` (already
        // computed by `wasm-validator::validate` -- see `WasmInstance::
        // canonical_types`'s own doc comment for why this is the ONLY
        // place this crate ever needs to reach for it).
        let canonical_types = validated.canonical_types().to_vec();
        // W34 fourth slice, security-review finding: ONE budget for this
        // WHOLE `instantiate()` call's import-resolution loop below, not
        // one per import -- see `wasm_types::CrossModuleComparisonBudget`'s
        // own doc comment. Without this, a module could declare an
        // arbitrary, byte-cheap number of function imports all targeting
        // one expensive-but-individually-capped canonical comparison,
        // multiplying a bounded per-comparison cost by an attacker-chosen
        // import count with no aggregate limit.
        let mut canonical_comparison_budget = wasm_types::CrossModuleComparisonBudget::new();
        let mut func_types: Vec<FuncType> = Vec::new();
        // Combined imported + module-defined func_index -> TYPE-SECTION-index
        // space (W33 second slice), index-aligned with `func_types` above --
        // see `WasmInstance::func_type_indices`'s own doc comment.
        let mut func_type_indices: Vec<u32> = Vec::new();
        let mut func_bodies: Vec<Option<FunctionBody>> = Vec::new();
        let mut host_functions: Vec<Option<Rc<dyn HostFunction>>> = Vec::new();
        let mut global_types: Vec<GlobalType> = Vec::new();
        let mut globals: Vec<Rc<RefCell<GlobalStorage>>> = Vec::new();
        // Combined imported + module-defined function IDENTITY space (W35
        // third slice, design §2), index-aligned with `func_types`/
        // `host_functions` above -- see `WasmInstance::func_identities`'s
        // own doc comment. Mirrors `tag_identities`'s own construction
        // EXACTLY: an import adopts `host_func.identity()` verbatim
        // (pushed alongside `host_functions.push(Some(Rc::from(host_func)))`
        // below); a module-defined function mints a fresh identity from
        // the SAME `NEXT_TAG_IDENTITY` counter `tag_identities` uses
        // (pushed alongside `host_functions.push(None)` below).
        let mut func_identities: Vec<u64> = Vec::new();
        let mut memories: Vec<LinearMemory> = Vec::new();
        let mut tables: Vec<Table> = Vec::new();
        // Combined imported + module-defined tag index space (W-next),
        // mirroring `func_types`'s own "imports first, then declared"
        // construction just below -- `module.tags` ALONE (like `module.
        // functions`) holds only module-DEFINED tags' type indices,
        // imports living separately in `module.imports`; this Vec is the
        // full combined space `wasm-execution`'s `ctx.tags` needs (see
        // `wasm-validator`'s OWN identically-shaped `tag_types` in
        // `type_check.rs::build_module_context`, which this mirrors).
        let mut tags: Vec<u32> = Vec::new();
        // Combined imported + module-defined tag IDENTITY space (W23),
        // index-aligned with `tags` above -- see
        // `WasmInstance::tag_identities`'s own doc comment.
        let mut tag_identities: Vec<u64> = Vec::new();

        // Resolve imports.
        for imp in &module.imports {
            match &imp.type_info {
                ImportTypeInfo::Function(type_idx) => {
                    let ft = module.types[*type_idx as usize].clone();

                    let host_func = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_function(&imp.module_name, &imp.name))
                        .ok_or_else(|| link_error("unknown import", imp))?;

                    // W34 fourth slice (`code/specs/
                    // W34-wasm-gc-canonical-type-equivalence.md`): when
                    // BOTH the importing module's own declared type and
                    // the exporting host function report a real canonical
                    // identity, compare THOSE directly instead of the
                    // pre-existing three-part conservative guard below --
                    // real cross-module canonical equivalence needs no
                    // shared numbering between the two modules at all (see
                    // `wasm_types::canonical_type_entries_equivalent`'s own
                    // doc comment), and SUBSUMES every one of the three
                    // checks below (shape/finality are already folded into
                    // `CanonicalSubtype`'s own fields; structural shape is
                    // the whole point of the comparison) -- this is what
                    // finally accepts an isomorphic-but-differently-
                    // numbered `rec` group import (`type-equivalence.wast`'s
                    // own "Semantic types (link time)" section) while
                    // STILL rejecting a genuine topology mismatch the old
                    // guard couldn't see (`type-subtyping.wast`'s `M5`/
                    // `M10`/`M11` cases -- see that file's own "Linking"
                    // section).
                    //
                    // Falls back to the three-part guard, UNCHANGED, when
                    // EITHER side reports `None` (no real canonical
                    // identity available -- e.g. a WASI-shim host import,
                    // or a type this slice's canonicalizer couldn't
                    // resolve) -- this is strictly additive over the old
                    // guard's own already-proven-sound behavior for every
                    // such import: the guard is never removed, only
                    // bypassed in favor of a MORE PRECISE check when one is
                    // actually available.
                    let module_canonical_type = canonical_types.get(*type_idx as usize).cloned().flatten();
                    match (host_func.canonical_type(), &module_canonical_type) {
                        (Some(_), Some(module_ct)) => {
                            // `canonically_matches` (not a plain equality
                            // check) -- a func import is satisfiable by an
                            // export whose actual type is a nominal
                            // SUBTYPE of the declared import type, not
                            // only an exact canonical match (see that
                            // method's own doc comment; `type-subtyping.
                            // wast`'s `M6`/`M7` "Linking" cases need
                            // exactly this).
                            if !host_func.canonically_matches(module_ct, &mut canonical_comparison_budget) {
                                return Err(link_error("incompatible import type", imp));
                            }
                        }
                        _ => {
                            if host_func.func_type() != &ft {
                                return Err(link_error("incompatible import type", imp));
                            }
                            // W33 first slice: a plain `FuncType` shape match
                            // alone isn't enough once `rec` groups exist -- two
                            // structurally-identical members of a `rec` group at
                            // DIFFERENT positions are DISTINCT types under the
                            // real GC canonicalization algorithm (`code/specs/
                            // W33-wasm-gc-recursive-type-subtyping.md`'s own
                            // item (3b)). This conservative guard ANDs a
                            // `(rec_group_size, rec_group_position)` match onto
                            // the pre-existing structural check -- see
                            // `tag.wast`'s own `assert_unlinkable` case, which
                            // needs exactly this. Safe for every PRE-EXISTING
                            // import (both sides report the singleton-group
                            // default, always trivially matching): can only ADD
                            // a rejection on top of the check above, never
                            // remove one.
                            if host_func.type_group_shape() != module.type_group_shape(*type_idx) {
                                return Err(link_error("incompatible import type", imp));
                            }
                            // W33 first slice: finality is as much a part of a
                            // type's real canonical identity as its shape or
                            // `rec`-group position -- `(sub (func))` (open) and
                            // `(sub final (func))` (final) are DISTINCT types
                            // even though `FuncType` equality can't see it (see
                            // `HostFunction::is_final`'s own doc comment).
                            // `type-subtyping.wast` lines 594-617 needs exactly
                            // this. Same "strictly additive, safe for every
                            // pre-existing import" reasoning as the group-shape
                            // guard above.
                            if host_func.is_final() != module.type_subtyping_at(*type_idx).is_final {
                                return Err(link_error("incompatible import type", imp));
                            }
                        }
                    }

                    func_types.push(ft);
                    func_type_indices.push(*type_idx);
                    func_bodies.push(None);
                    // W35 third slice, design §2: an IMPORTED function
                    // adopts the exporter's own already-minted identity
                    // verbatim (`HostFunction::identity()`), mirroring
                    // `tag_identities`'s own import arm exactly. Read
                    // BEFORE `host_func` moves into `Rc::from` below
                    // (`Box<dyn HostFunction>` derefs to call `&self`
                    // methods fine either way, but the move happens on the
                    // very next line).
                    func_identities.push(host_func.identity());
                    // `resolve_function` returns `Box<dyn HostFunction>`
                    // (unchanged, per W35 first slice's scope -- only
                    // `host_functions`' own storage moved to `Rc`);
                    // `Rc::from` converts the owned `Box` into an `Rc`
                    // with no behavior change (`HostFunction`'s methods
                    // are all `&self`).
                    host_functions.push(Some(Rc::from(host_func)));
                }
                ImportTypeInfo::Memory(mem_type) => {
                    let imported_mem = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_memory(&imp.module_name, &imp.name))
                        .ok_or_else(|| link_error("unknown import", imp))?;
                    // W25 (memory64): an is64 mismatch between the actual
                    // memory and the declared import type is always
                    // incompatible, the same "real type mismatch" shape
                    // every other import-compat check here already uses
                    // -- checked BEFORE `limits_compatible` (which is
                    // is64-agnostic; both sides' `Limits` are `u64` now
                    // regardless of `is64`, so a mismatch wouldn't
                    // otherwise be caught by it at all).
                    if imported_mem.is64() != mem_type.is64 {
                        return Err(link_error("incompatible import type", imp));
                    }
                    let actual = Limits { min: imported_mem.size() as u64, max: imported_mem.max_pages().map(|m| m as u64) };
                    if !limits_compatible(&actual, &mem_type.limits) {
                        return Err(link_error("incompatible import type", imp));
                    }
                    memories.push(imported_mem);
                }
                ImportTypeInfo::Table(table_type) => {
                    let imported_table = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_table(&imp.module_name, &imp.name))
                        .ok_or_else(|| link_error("unknown import", imp))?;
                    // W37 addendum: `Table` now DOES track its declared
                    // element-type tag at runtime (`Table::element_type`,
                    // set from the exporting module's own real
                    // `wasm_types::TableType::element_type` at
                    // construction time, above) -- this arm used to skip
                    // this check entirely, with a comment explicitly
                    // naming the gap ("a table import mismatched purely on
                    // element type would incorrectly link here rather
                    // than fail... revisit if a future PR gives `Table` a
                    // real element-type field"). GC reftype table
                    // DECLARATIONS (W37 proper) made this reachable for
                    // real: `linking.wast`'s own `t-funcnull`/`t-refnull`
                    // (funcref-family) tables, each incorrectly linkable
                    // as a declared `externref` import before this fix.
                    //
                    // A single byte-equality check (not a subtype check)
                    // is the CORRECT real-spec rule here, not merely the
                    // simplest one: table types are matched INVARIANTLY on
                    // element type (unlike a function import, which the
                    // function-references proposal lets a nominal SUBTYPE
                    // satisfy) -- a table supports `table.set` as well as
                    // `table.get`, so a covariant or contravariant element
                    // type would let code typed against the DECLARED
                    // import type read or write a value the REAL
                    // underlying table cannot actually hold. It is also
                    // the only check this call site's own import syntax
                    // can ever need: `build_import_shell`'s "table" arm
                    // (`wasm-wast-parser/src/module.rs`) only ever parses
                    // a bare `funcref`/`externref` atom for an imported
                    // table's declared reftype -- no concrete/GC-typed
                    // table IMPORT syntax exists in this crate's text
                    // format at all (a separate, deliberate, already-
                    // documented scope boundary, `code/specs/
                    // W37-wasm-gc-reftype-tables.md`'s own "left
                    // unchanged, deliberately" note) -- so `table_type.
                    // element_type` here is always exactly `wasm_types::
                    // FUNCREF` or `wasm_types::EXTERNREF`, never anything
                    // more specific a subtype relation could apply to.
                    if imported_table.element_type() != table_type.element_type {
                        return Err(link_error("incompatible import type", imp));
                    }
                    // W26 (table64 proposal): an `is64` mismatch between
                    // the actual table and the declared import type is
                    // always incompatible, the same "real type mismatch"
                    // shape the memory-import arm above already uses --
                    // checked BEFORE `limits_compatible` (which is
                    // `is64`-agnostic; both sides' `Limits` are `u64` now
                    // regardless of `is64`, so a mismatch wouldn't
                    // otherwise be caught by it at all).
                    if imported_table.is64() != table_type.is64 {
                        return Err(link_error("incompatible import type", imp));
                    }
                    let actual = Limits { min: imported_table.size() as u64, max: imported_table.max_size().map(|m| m as u64) };
                    if !limits_compatible(&actual, &table_type.limits) {
                        return Err(link_error("incompatible import type", imp));
                    }
                    tables.push(imported_table);
                }
                ImportTypeInfo::Global(gt) => {
                    // `gval` is the exporting instance's own SHARED
                    // `Rc<RefCell<GlobalStorage>>` cell (W35 third slice;
                    // see `HostInterface::resolve_global`'s own doc
                    // comment) -- pushed here as-is, not dereferenced/
                    // copied, so a `global.set` through EITHER this
                    // importing instance or the exporting one is visible
                    // through the other -- and, for a funcref-typed
                    // global, already fully resolved by the EXPORTING
                    // instance's own `instantiate()` fixup pass (see that
                    // function's own doc comment), so this importing
                    // instance never needs to (and never does) re-resolve
                    // it itself.
                    let (gtype, gval) = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_global(&imp.module_name, &imp.name))
                        .ok_or_else(|| link_error("unknown import", imp))?;
                    if &gtype != gt {
                        return Err(link_error("incompatible import type", imp));
                    }
                    global_types.push(gtype);
                    globals.push(gval);
                }
                // Tag imports (exceptions proposal; W21 added the
                // structural bookkeeping, W-next adds real resolution) --
                // same shape as `Function` above: ask the host for the
                // real tag type, then check it against what THIS module's
                // own import declaration expects. Unlike `func_types`/
                // `global_types`/etc. above, nothing needs accumulating
                // into a fresh local Vec here -- `module.tags` (the
                // combined imported+defined tag index space W21 already
                // has the parser build, "imports first, then declaration
                // order") is already complete; this arm exists purely to
                // perform the real LINK compatibility check a tag import
                // needs, matching every other import kind.
                ImportTypeInfo::Tag(type_idx) => {
                    let expected = module.types[*type_idx as usize].clone();
                    // (W23) `resolve_tag` now also returns the exporting
                    // instance's own already-minted canonical identity for
                    // this tag -- adopted here VERBATIM (not re-minted),
                    // so this instance's own throw/catch of the imported
                    // tag compares equal to the exporter's, across the
                    // instance boundary. See `WasmInstance::tag_identities`'s
                    // own doc comment.
                    let (actual, identity) = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_tag(&imp.module_name, &imp.name))
                        .ok_or_else(|| link_error("unknown import", imp))?;
                    if actual != expected {
                        return Err(link_error("incompatible import type", imp));
                    }
                    // W33 first slice: same conservative `rec`-group
                    // shape guard as the `Function` arm above, and for
                    // the same reason -- `tag.wast`'s own
                    // `assert_unlinkable` case is a TAG import using
                    // exactly this shape (`(rec (type $t1 (func)) (type
                    // $t2 (func)))`, importing under `$t2` when the
                    // export was declared `$t1`).
                    let importer_shape = module.type_group_shape(*type_idx);
                    let exporter_shape = self.host.as_ref().map(|h| h.resolve_tag_group_shape(&imp.module_name, &imp.name)).unwrap_or((1, 0));
                    if exporter_shape != importer_shape {
                        return Err(link_error("incompatible import type", imp));
                    }
                    // W33 first slice: same finality guard as the
                    // `Function` arm above.
                    let importer_is_final = module.type_subtyping_at(*type_idx).is_final;
                    let exporter_is_final = self.host.as_ref().map(|h| h.resolve_tag_is_final(&imp.module_name, &imp.name)).unwrap_or(true);
                    if exporter_is_final != importer_is_final {
                        return Err(link_error("incompatible import type", imp));
                    }
                    tags.push(*type_idx);
                    tag_identities.push(identity);
                }
            }
        }

        // Add module-defined functions. W35 third slice, design §2: each
        // mints a fresh, never-repeating identity from the SAME process-
        // wide `NEXT_TAG_IDENTITY` counter `tag_identities` (just below)
        // already uses -- the spec's own explicit reasoning: "tags and
        // functions are never compared against each other, so sharing one
        // counter is harmless."
        for (i, &type_idx) in module.functions.iter().enumerate() {
            func_types.push(module.types[type_idx as usize].clone());
            func_type_indices.push(type_idx);
            func_bodies.push(module.code.get(i).cloned());
            host_functions.push(None);
            func_identities.push(NEXT_TAG_IDENTITY.fetch_add(1, Ordering::Relaxed));
        }

        // Add module-defined tags, completing the combined index space
        // `tags` above started with imports. Each gets a freshly minted,
        // never-repeating canonical identity (W23) -- see
        // `NEXT_TAG_IDENTITY`'s own doc comment for why this must happen
        // exactly once per real instantiation, not once per call.
        for &type_idx in &module.tags {
            tags.push(type_idx);
            tag_identities.push(NEXT_TAG_IDENTITY.fetch_add(1, Ordering::Relaxed));
        }

        // Allocate every locally-declared memory (multi-memory, W16, task
        // #85) -- imported memories (pushed above) occupy the low
        // indices, matching every other index space's import-then-declared
        // ordering in this same function.
        //
        // W25 (memory64): `new_with_is64` is fallible -- a memory64
        // memory's spec-valid declaration ceiling (`2^48` pages,
        // `wasm-validator`'s own Check 1b) is far larger than this
        // interpreter will actually allocate (`MAX_MEMORY64_INITIAL_
        // PAGES`, `wasm-execution`'s own practical cap) -- so a module
        // that only ever DECLARES such a memory validates successfully,
        // and only an actual instantiation attempt like this one hits
        // the cap, as a real `TrapError` (never a panic/allocator
        // abort). `total_is64_pages` mirrors `wasm-validator`'s own
        // Check 1b aggregate reasoning (many individually-under-cap
        // memories still totaling an unreasonable amount) for the is64
        // case specifically -- Check 1b's own aggregate only covers
        // 32-bit memories today.
        let mut total_is64_pages: u64 = 0;
        for mem_type in &module.memories {
            if mem_type.is64 {
                total_is64_pages += mem_type.limits.min;
                if total_is64_pages > wasm_execution::MAX_MEMORY64_INITIAL_PAGES {
                    return Err(TrapError::new(format!(
                        "total declared 64-bit memory across this module is at least {total_is64_pages} pages, exceeding this interpreter's practical aggregate cap of {} pages",
                        wasm_execution::MAX_MEMORY64_INITIAL_PAGES
                    )));
                }
            }
            memories.push(LinearMemory::new_with_is64(mem_type.limits.min, mem_type.limits.max, mem_type.is64)?);
        }

        // Allocate tables. W26 (table64 proposal): `table_type.limits.min`
        // is `u64` (already widened in W25, table-agnostic), and an `is64`
        // table's own real spec ceiling is `u64::MAX` -- far larger than
        // this interpreter will actually allocate. A plain `as u32`
        // narrowing here would silently TRUNCATE/wrap an out-of-
        // practical-range `min` into an arbitrary, wrong-sized table
        // instead of failing loudly, for any `is64` table whose declared
        // `min` exceeds `u32::MAX`. `Table::new_with_is64` mirrors
        // `LinearMemory::new_with_is64` (W25) exactly: fallible, returning
        // a real, gracefully-propagated `TrapError` (never a panic/
        // allocator abort) if `min` exceeds `MAX_TABLE_ELEMENTS`, this
        // interpreter's own practical resource cap, UNCONDITIONALLY --
        // not just for `is64` tables (see that constructor's own doc
        // comment). Gap 2 of the W-next `elem.wast`/`table.wast`
        // investigation pass (`code/specs/W07-wasm-post-mvp-epics.md`'s
        // addendum) moved this same per-table cap's enforcement for
        // 32-bit tables here too -- `wasm-validator` used to reject an
        // oversized 32-bit `min` at STRUCTURAL VALIDATION time (its old
        // Check 2b), which `table.wast`'s own real corpus case proved
        // wrong: the real spec allows declaring (never allocating) a
        // 32-bit table `min` up to `2^32 - 1`. `Table::new_with_is64`'s
        // own per-table cap below already covered the single-table case
        // for every table regardless of `is64` even before this move (the
        // validator's old check was redundant with it, not this
        // constructor's only guard) -- what's NEW here is `total_table_
        // elements` immediately below, generalized from the previous
        // `is64`-only `total_is64_table_elements` to cover EVERY declared
        // table, closing the aggregate gap the validator's removed Check
        // 2b used to close for 32-bit tables specifically.
        //
        // `total_table_elements`: without this aggregate, a module could
        // declare up to `MAX_TABLES` (64) separate tables each
        // individually AT the per-table `MAX_TABLE_ELEMENTS` cap
        // (10,000,000) and still instantiate all of them -- 64 *
        // 10,000,000 * (8 bytes/entry, `Vec<Option<u32>>`) ~= 5.1GB from
        // one small module, the exact "many individually-under-cap
        // tables still totaling too much" shape this aggregate exists to
        // prevent (same reasoning `total_is64_pages` above already
        // applies to 64-bit memories). `saturating_add`, NOT `+=`: a
        // table's own `min` can be as large as `u64::MAX` (an `is64`
        // table's real spec ceiling), so a `+=` overflow here isn't
        // independently exploitable today (any single addend large
        // enough to wrap the running total is, by construction, ALSO
        // large enough to trip `Table::new_with_is64`'s own per-table cap
        // a few lines below, before any allocation happens), but
        // `saturating_add` costs nothing and makes this aggregate check
        // self-sufficient rather than silently relying on that other
        // check to save it.
        let mut total_table_elements: u64 = 0;
        for table_type in &module.tables {
            total_table_elements = total_table_elements.saturating_add(table_type.limits.min);
            if total_table_elements > wasm_execution::MAX_TABLE_ELEMENTS as u64 {
                return Err(TrapError::new(format!(
                    "total declared table elements across this module is at least {total_table_elements}, exceeding this interpreter's practical aggregate cap of {}",
                    wasm_execution::MAX_TABLE_ELEMENTS
                )));
            }
            tables.push(
                Table::new_with_is64(table_type.limits.min, table_type.limits.max, table_type.is64)?
                    .with_element_type(table_type.element_type),
            );
        }

        // The instance's persistent v128 heap (see `code/specs/
        // W15-wasm-v128-persistent-storage.md`) -- built up here, during
        // instantiation, so a `v128.const` in a global/data/elem
        // initializer allocates directly into the SAME `Vec` this
        // instance will keep for its whole lifetime, not a throwaway one.
        // Index 0 reserved as the all-zero entry, matching
        // `wasm_execution::WasmExecutionContext::v128_heap`'s convention.
        let mut v128_heap: Vec<[u8; 16]> = vec![[0u8; 16]];

        // The instance's persistent GC object heap (W33 fourth slice) --
        // see `WasmInstance::gc_heap`'s own doc comment. Only a GLOBAL
        // initializer can allocate into it (data/elem segment offset
        // expressions are always plain integers in the real corpus, never
        // `struct.new`/`array.new`), so only the globals loop below uses
        // the GC-capable evaluator; data/elem offsets keep using the plain
        // `evaluate_const_expr`, unaffected.
        let mut gc_heap: Vec<Option<GcObject>> = Vec::new();
        let (struct_field_counts, struct_field_storage, array_element_storage) = struct_array_runtime_tables(module);

        // Initialize globals. `evaluate_const_expr_gc` itself is
        // unchanged -- it still takes a plain `&[WasmValue]` snapshot --
        // but `globals` is now `Vec<Rc<RefCell<GlobalStorage>>>` (W35
        // third slice; real cross-instance sharing was already true via
        // W28's own `Rc<RefCell<..>>` wrapping -- see `WasmInstance::
        // globals`'s own doc comment), so each iteration derives a fresh
        // snapshot of whatever globals' `value`s are already defined
        // (imports, plus every earlier module-defined global processed so
        // far in this same loop -- a later global's own init expr can
        // `global.get` an earlier one) before wrapping the newly computed
        // value in its own shared cell.
        //
        // A funcref-typed global's `ref.func`-produced `value` here is
        // DELIBERATELY LEFT UNRESOLVED (`func_ref: None`, a raw index in
        // `value`, exactly as `evaluate_const_expr_gc`'s own `0xD2` arm
        // produces it -- see that function's own doc comment: it has no
        // access to `host_functions`/a resolver at all) -- this is the
        // exact construction-order problem this function's own doc
        // comment names: no `Rc<RefCell<WasmInstance>>` exists yet at this
        // point for a `LocalFunctionRef` to hold. A SEPARATE fixup pass,
        // below, resolves every module-defined funcref-typed global's
        // initial value for real, once `instance_rc` exists.
        for global in &module.globals {
            global_types.push(global.global_type.clone());
            let globals_so_far: Vec<WasmValue> = globals.iter().map(|g| g.borrow().value).collect();
            let value = evaluate_const_expr_gc(
                &global.init_expr,
                &globals_so_far,
                &mut v128_heap,
                &mut gc_heap,
                &struct_field_counts,
                &struct_field_storage,
                &array_element_storage,
            )?;
            globals.push(Rc::new(RefCell::new(GlobalStorage { value, func_ref: None })));
        }

        // A fixed, post-initialization snapshot of every global's value,
        // for the element/data segment offset expressions below --
        // active segment offsets are evaluated ONCE, after every global
        // is already in its final initial value, and never observe a
        // LATER `global.set` (there isn't one yet at this point in
        // instantiation), so one shared snapshot is exact for both loops
        // -- no need to re-snapshot per segment.
        let global_values: Vec<WasmValue> = globals.iter().map(|g| g.borrow().value).collect();

        // Apply element segments BEFORE data segments -- the order the
        // official spec's own instantiation algorithm mandates (step 27,
        // "Execute the sequence instr*_e" for active element segments,
        // strictly before step 28, "Execute the sequence instr*_d" for
        // active data segments -- see `code/specs/
        // W10-wasm-real-linking-and-unlinkable.md`'s addendum for this
        // fix). This is a fixed TWO-PHASE order (all elem, then all
        // data), not a per-segment interleaving by declaration order --
        // it holds regardless of which kind of segment a module's own
        // text happens to declare first.
        //
        // This block and the data-segment block below it USED to run in
        // the opposite order (data first, then elem) -- a real,
        // spec-violating bug found vendoring `linking0.wast`: when a
        // module declares an in-bounds ACTIVE ELEMENT segment together
        // with a data segment that traps (e.g. an out-of-bounds write),
        // the elem segment's own effect must already be applied and must
        // PERSIST past that later data-segment trap -- the same "earlier
        // segments persist past a later trap" rule this crate's own
        // per-segment-atomicity fix below already established WITHIN one
        // kind, applied ACROSS kinds. With data applied first, a
        // data-segment trap returned via `?` before this elem loop ever
        // ran, so the elem write was silently lost entirely --
        // `linking0.wast`'s own `(assert_return (invoke $Mt "call"
        // (i32.const 7)) (i32.const 0))` (after an earlier, separately
        // instantiated module writes an in-bounds `(elem (i32.const 7)
        // $f)` into `$Mt`'s already-exported, shared table, then traps on
        // an out-of-bounds data write) caught this directly: instead of
        // returning the already-applied elem write's value, it trapped
        // with "uninitialized table element", proving the elem write
        // never happened. Confirmed via this crate's own baseline diff
        // (see `wasm-conformance`'s CHANGELOG) that swapping these two
        // blocks fixes exactly that case with zero regressions elsewhere
        // in the 257-file corpus -- each block's OWN internal
        // per-segment-atomicity/bounds-checking logic is unchanged, only
        // their RELATIVE order moved.
        // W35 fourth slice, security-review finding: `(table_index,
        // offset, count)` for every entry an ACTIVE elem segment writes
        // below -- the EXACT set of slots this call's own fixup (success
        // path: `wasm-conformance`'s registry-driven `resolve_all_table_
        // funcrefs`; error path: immediately below) is safe to resolve.
        //
        // This replaces an earlier, LESS precise design (scan every
        // currently-`Raw` entry in every visible table) that a security
        // review found unsound: a table's `Raw` entries are NOT
        // exclusively "unresolved writes THIS instance's own instantiate()
        // call just made" -- a LIVE `table.init`/`table.set`/`table.fill`/
        // `table.grow` on some OTHER, ALREADY-registered instance sharing
        // the SAME table can leave a `Raw` entry there at any LATER point
        // in the script, well after this instance's own fixup already
        // ran. Scanning "every currently-`Raw` entry" at THIS instance's
        // own fixup time could not distinguish that stale, foreign entry
        // from one this instance itself just wrote -- misattributing it
        // to the WRONG instance's combined index space, a silent wrong
        // dispatch, exactly the class of bug this whole spec exists to
        // fix, not reintroduce. Recording the PRECISE ranges here, at the
        // exact moment this instance's own active elem segment writes
        // them, makes that misattribution structurally impossible: no
        // scanning, no guessing, only the slots this call is certain it
        // just wrote.
        let mut active_elem_writes: Vec<(u32, u32, u32)> = Vec::new();

        // Per-element-segment "already dropped" flags, computed as a
        // byproduct of the very same loop that applies active segments
        // below -- NOT the instance's own `dropped_elements` field yet
        // (that's built further down, at this function's normal success
        // return, from this exact `Vec`). Real spec text: "after an
        // active or declarative element segment is initialized, it is
        // dropped" -- so an ACTIVE segment (this loop's main branch, just
        // below) is marked dropped the moment its own bounds check and
        // writes succeed, and a DECLARATIVE segment (`elem.is_declarative`
        // -- see that field's own doc comment in `wasm_types::Element` for
        // why `is_passive` alone can't tell it apart from a genuinely
        // passive one) is marked dropped immediately, with no content ever
        // copied anywhere -- it was never live to begin with. A genuinely
        // PASSIVE segment (`is_passive: true`, `is_declarative: false`)
        // stays `false` here, completely unaffected: it remains resident
        // and `table.init`-able until an explicit `elem.drop` or a
        // consuming `table.init` call (see `wasm-execution`'s own opcode
        // handlers) changes its dropped state, same as before this fix.
        let mut dropped_elements: Vec<bool> = vec![false; module.elements.len()];

        // W35 fourth slice: the elem-segment and data-segment loops below
        // are wrapped in an IIFE returning `Result<(), TrapError>` --
        // their own bodies are BYTE-FOR-BYTE UNCHANGED from before this
        // slice (every `?` inside still propagates exactly where it always
        // did, just now out of this closure instead of `instantiate()`
        // itself) -- purely so a trap from EITHER loop can be intercepted
        // once, uniformly, right below, before propagating: see the
        // "ephemeral trap-discarded instance" handling immediately after
        // this closure's own call site for why.
        let elem_data_result: Result<(), TrapError> = (|| -> Result<(), TrapError> {
        for (elem_idx, elem) in module.elements.iter().enumerate() {
            if elem.is_passive {
                // A declarative segment (`is_declarative`, folded into
                // `is_passive: true` by `wasm_types::Element`'s own
                // documented convention -- see its doc comment) is never
                // applied to any table, same as a genuinely passive one --
                // but unlike a genuinely passive segment, the real spec
                // requires it be treated as already dropped from this
                // point on, so mark it here, before the `continue`. A
                // genuinely passive segment (`is_declarative: false`)
                // leaves `dropped_elements[elem_idx]` at its initial
                // `false` -- unaffected, exactly as before this fix.
                if elem.is_declarative {
                    dropped_elements[elem_idx] = true;
                }
                continue;
            }
            if let Some(table) = tables.get_mut(elem.table_index as usize) {
                // W26 (table64): an active element segment's offset
                // expression must match its TARGET table's own address
                // width -- `i64.const` for an `is64` table, `i32.const`
                // otherwise -- mirroring the active data segment's
                // identical `is64`-aware branch below (W25). Kept in
                // `u64` throughout (not narrowed to `u32` until AFTER the
                // upfront bounds check below): an is64 table's real spec
                // ceiling is `u64::MAX` (see `code/specs/
                // W26-wasm-table64-first-slice.md`), so a huge, clearly
                // out-of-range `i64` offset must not silently wrap into a
                // small, coincidentally-in-range `u32` before the bounds
                // check runs (the same "narrow only after checking, never
                // before" discipline `pop_table_operand`'s own doc comment
                // establishes in `wasm-execution`).
                let is64 = table.is64();
                let offset = evaluate_const_expr(&elem.offset_expr, &global_values, &mut v128_heap)?;
                let offset_num: u64 = if is64 {
                    offset.as_i64().map_err(|e| TrapError::new(e.message))? as u64
                } else {
                    offset.as_i32().map_err(|e| TrapError::new(e.message))? as u32 as u64
                };
                // Bounds-check the WHOLE segment before writing ANY entry
                // (W28) -- real per-segment atomicity, matching
                // `LinearMemory::write_bytes`'s own upfront-bounds-check
                // shape (a single `bounds_check` before the one
                // `copy_from_slice`, never a byte-at-a-time loop that could
                // partially write before trapping). The loop below used to
                // call `table.set` one entry at a time and propagate the
                // first out-of-bounds error via `?` -- correct for a
                // segment that's ENTIRELY out of bounds, but WRONG for one
                // that's only PARTIALLY out of bounds: entries before the
                // first bad index had already been written by the time the
                // trap fired. That partial write used to be unobservable
                // for an IMPORTED table (the whole failed `instantiate()`
                // call, including its local `tables` Vec holding an
                // independent CLONE of the import, was simply dropped on
                // error) but is a real, spec-violating bug now that a
                // shared table's storage (W28's `Rc<RefCell<TableStorage>>`)
                // genuinely persists past a failed `instantiate()` call --
                // the exporting instance still holds the same storage, so
                // a partial write would otherwise leak through. The real
                // spec's own per-segment atomicity (see the "unlike the v1
                // spec" comment on `linking.wast`'s later `assert_trap`
                // cases: EARLIER, already-fully-applied segments persist
                // past a LATER segment's trap, but a single segment is
                // itself all-or-nothing) requires exactly this upfront
                // check.
                let count = elem.function_indices.len() as u64;
                let table_size = table.size() as u64;
                if offset_num.checked_add(count).is_none_or(|end| end > table_size) {
                    return Err(TrapError::new(format!(
                        "out of bounds table access: elements {offset_num}..{}, table size={table_size}",
                        offset_num.saturating_add(count)
                    )));
                }
                // Safe to narrow to `u32` here: `offset_num + j` (for every
                // `j` in this loop, `j < count`) was just checked `<=
                // table_size` above, which is itself always `<=
                // MAX_TABLE_ELEMENTS` (far below `u32::MAX`) -- so this
                // cast never loses information for any value that reaches
                // this loop body.
                // W35 slice 2 mechanical fallout: `Table::set` now takes a
                // real `TableElement`, not a bare `u32` -- see
                // `code/specs/W35-wasm-cross-instance-function-identity.md`
                // §6. `TableElement::Raw` wraps this active elem segment's
                // entry verbatim, UNRESOLVED, exactly as it always was:
                // real resolution (via `WasmExecutionContext::resolve_
                // function_ref_for_dispatch`) happens lazily, at
                // `call_indirect`'s own read site, once this instance's
                // `WasmExecutionContext` actually exists (it doesn't yet,
                // here in `instantiate()`). This is a purely mechanical
                // type-following change, not new cross-instance logic --
                // resolving eagerly here, using the DECLARING instance's
                // own context, is W35's third slice (`LocalFunctionRef`),
                // deliberately out of this slice's scope.
                for (j, &func_idx) in elem.function_indices.iter().enumerate() {
                    table.set((offset_num + j as u64) as u32, func_idx.map(TableElement::Raw))?;
                }
                // W35 fourth slice: record exactly what was just written,
                // now that the whole segment's own bounds check and every
                // `table.set` above succeeded -- see `active_elem_writes`'s
                // own doc comment above this closure for why this
                // precision (not a post-hoc scan) is load-bearing.
                if count > 0 {
                    active_elem_writes.push((elem.table_index, offset_num as u32, count as u32));
                }
                // Real spec text: "after an active or declarative element
                // segment is initialized, it is dropped" -- this active
                // segment's own bounds check and every `table.set` above
                // just succeeded (an upfront trap on either would have
                // propagated via `?`/`return Err` before reaching here),
                // so mark it dropped now, unconditionally (even for a
                // zero-length segment, `count == 0`, which is still a real
                // active segment per spec, just an empty one) -- a LATER
                // `table.init` naming this same segment index must find it
                // already dropped and trap, matching `elem.wast`'s/
                // `table_init.wast`'s own "Implicitly dropped elements"
                // corpus cases exactly.
                dropped_elements[elem_idx] = true;
            }
        }

        // Apply data segments. Widened (real corpus vendoring pass, see
        // `wasm-conformance`'s CHANGELOG) to target each active segment's
        // OWN `seg.memory_index`, not unconditionally memory 0 -- the prior
        // "memory 0 only" restriction was `wasm-validator`'s own Check 8
        // rejecting any other index at validation time, not a real
        // multi-memory limitation; a module past validation here is
        // guaranteed `seg.memory_index` is in bounds (Check 8 bounds-checks
        // it against the real memory count), so `memories.get_mut` below
        // always succeeds for a well-formed caller -- the `continue`
        // fallback is defensive only, never reachable through the normal
        // validate-then-instantiate path this crate's own callers use.
        //
        // A PASSIVE segment (`is_passive`, task #95) is deliberately
        // skipped here -- applying it automatically would defeat the
        // entire point of `memory.init`/`data.drop`: a passive segment's
        // bytes stay resident, untouched, until an explicit `memory.init`
        // copies from it (any number of times, on demand), which is a
        // completely separate code path from this one-time instantiation-
        // time copy.
        for seg in &module.data {
            if seg.is_passive {
                continue;
            }
            let Some(mem) = memories.get_mut(seg.memory_index as usize) else {
                continue;
            };
            // W25 (memory64): the TARGET memory's own `is64`-ness
            // determines whether this active data segment's offset
            // expression is an `i32.const` or `i64.const` -- now resolved
            // per-segment (each segment can target a different memory),
            // not fixed to memory 0's.
            let is64 = mem.is64();
            let offset = evaluate_const_expr(&seg.offset_expr, &global_values, &mut v128_heap)?;
            let offset_num = if is64 {
                offset.as_i64().map_err(|e| TrapError::new(e.message))? as usize
            } else {
                offset.as_i32().map_err(|e| TrapError::new(e.message))? as usize
            };
            mem.write_bytes(offset_num, &seg.data)?;
        }
        Ok(())
        })();

        if let Err(e) = elem_data_result {
            // W35 fourth slice: a module whose OWN active elem segment
            // writes into a table (owned OR imported/SHARED, e.g.
            // `linking0.wast`/`linking3.wast`'s own anonymous
            // `assert_trap`-wrapped modules) before a LATER data-segment
            // trap discards the `WasmInstance` this function would
            // otherwise have returned -- see this function's own extended
            // doc comment on the "MAJOR deviation" for why NEITHER
            // `instantiate()`'s own SUCCESS path (this exact same
            // resolution, deferred to `wasm-conformance`'s registry-driven
            // fixup, per that section) NOR this ERROR path can use the
            // spec's own literal "wrap `instance`, fix up, `Rc::try_unwrap`
            // back to a bare value" recipe -- but this ERROR path doesn't
            // need `try_unwrap` to succeed at all, since it is about to
            // return `Err`, never a bare owned `WasmInstance`. A TEMPORARY
            // `Rc<RefCell<WasmInstance>>`, built from this call's own
            // current state and never unwrapped, is enough: any
            // `FuncRefTarget` this resolves and durably writes into a
            // SHARED table (via `resolve_all_table_funcrefs`) holds its
            // OWN `Rc` clone of this temporary instance, keeping it alive
            // via ordinary refcounting long after this `instantiate()`
            // call returns and its own local `temp_rc` variable is
            // dropped -- exactly the same "an `Rc` survives via whoever
            // still holds a clone, not via who declared it" reasoning
            // `wasm-conformance`'s own `ModuleRegistry` already relies on.
            // Best-effort (`let _ =`): a failure INSIDE the fixup itself
            // (only possible for a segment naming a genuinely out-of-range
            // function index -- unreachable for anything that passed
            // `wasm-validator::validate`) must never mask or replace the
            // REAL, original instantiation trap `e` -- that's still the
            // error this function promises to report.
            let temp_instance = WasmInstance {
                module: module.clone(),
                memories: memories.clone(),
                tables: tables.clone(),
                globals: globals.clone(),
                global_types: global_types.clone(),
                func_types: func_types.clone(),
                func_type_indices: func_type_indices.clone(),
                canonical_types: canonical_types.clone(),
                func_bodies: func_bodies.clone(),
                host_functions: host_functions.clone(),
                tags: tags.clone(),
                tag_identities: tag_identities.clone(),
                exports: module.exports.iter().map(|ex| (ex.name.clone(), ex.kind, ex.index)).collect(),
                v128_heap: v128_heap.clone(),
                gc_heap: gc_heap.clone(),
                dropped_data_segments: vec![false; module.data.len()],
                // Whatever this call's own closure successfully computed
                // before the trap that brought us here (active segments it
                // already applied, declarative segments it already
                // recognized) -- same "exactly the slots THIS instance
                // itself wrote, nothing more" reasoning `active_elem_
                // writes.clone()` below already documents. This temporary
                // instance is about to be discarded (an `Err` follows
                // immediately), so no `table.init` can ever read this
                // field again regardless -- cloned purely so the value
                // isn't silently wrong if that ever changes.
                dropped_elements: dropped_elements.clone(),
                func_identities: func_identities.clone(),
                instance_identity: NEXT_INSTANCE_IDENTITY.fetch_add(1, Ordering::Relaxed),
                // Whatever this call's own closure successfully recorded
                // before the trap that brought us here -- exactly the
                // slots THIS instance itself wrote, nothing more (an
                // elem segment that never got applied, because an
                // EARLIER one in the same loop already trapped, was
                // never pushed above).
                active_elem_writes: active_elem_writes.clone(),
            };
            let temp_rc = Rc::new(RefCell::new(temp_instance));
            let _ = resolve_all_table_funcrefs(&temp_rc);
            return Err(e);
        }

        // Build export list.
        let exports: Vec<(String, ExternalKind, u32)> = module
            .exports
            .iter()
            .map(|e| (e.name.clone(), e.kind, e.index))
            .collect();

        // One dropped-flag per data segment, all initially false (task
        // #95) -- `data.drop` flips one to true; nothing in this
        // instantiation path drops anything itself.
        let dropped_data_segments = vec![false; module.data.len()];

        // One dropped-flag per element segment (task #97). Unlike
        // `dropped_data_segments` above, NOT all-`false` here: `dropped_
        // elements` was already computed by the elem/data closure just
        // above, which marks an ACTIVE segment's entry `true` the moment
        // it finishes applying, and a DECLARATIVE segment's entry `true`
        // immediately (see that closure's own doc comments) -- real spec
        // behavior, "after an active or declarative element segment is
        // initialized, it is dropped". A genuinely passive segment's entry
        // stays `false` here, exactly as before this fix; `elem.drop`/a
        // consuming `table.init` are still the only things that can flip
        // one of those to `true` from this point on.

        let mut instance = WasmInstance {
            module: module.clone(),
            memories,
            tables,
            globals,
            global_types,
            func_types,
            func_type_indices,
            canonical_types,
            func_bodies,
            host_functions,
            tags,
            tag_identities,
            exports,
            v128_heap,
            gc_heap,
            dropped_data_segments,
            dropped_elements,
            func_identities,
            // W35 third slice: minted ONCE per real `instantiate()` call,
            // from the dedicated `NEXT_INSTANCE_IDENTITY` counter -- see
            // `WasmInstance::instance_identity`'s own doc comment.
            instance_identity: NEXT_INSTANCE_IDENTITY.fetch_add(1, Ordering::Relaxed),
            active_elem_writes,
        };

        // **Major, evidence-backed deviation from the spec's own literal
        // §6/"construction-order problem" design** (not a smaller
        // implementation detail -- flagged prominently because it changes
        // this slice's own delivered scope): the spec's own recommended
        // resolution -- build `instance`, `Rc::new(RefCell::new(..))` it,
        // run an elem/global "fixup" pass that resolves each entry into a
        // real `FuncRefTarget` via a `LocalFunctionRef` closing over that
        // SAME `Rc`, then `Rc::try_unwrap` back to the plain `WasmInstance`
        // this function promises, claiming that unwrap "MUST be
        // infallible... nothing else holds a reference yet" -- is
        // STRUCTURALLY UNSOUND, not merely bug-prone, for the
        // OVERWHELMINGLY COMMON case this exact machinery was built to
        // handle: a module's OWN active elem segment (or funcref-typed
        // global initializer) referencing one of ITS OWN local functions
        // (`linking.wast`'s own `$Mt`/`$g` example -- the spec's own
        // motivating case -- is exactly this shape).
        //
        // A `LocalFunctionRef` resolved for such an entry necessarily
        // holds `instance: instance_rc.clone()` (see
        // `resolve_func_ref_for_instance`'s own doc comment) -- and that
        // clone gets written into `instance`'s OWN `tables`/`globals`,
        // which are THEMSELVES part of the SAME `instance` `instance_rc`
        // wraps. This is a genuine, unavoidable SELF-referential `Rc`
        // cycle (not merely a two-instance cycle the spec's own "Security
        // and lifetime consideration" section already anticipated and
        // accepted for a REGISTRY's bounded lifetime) -- `instance_rc`'s
        // strong count becomes `1 + (however many local functions got
        // referenced by this module's own elem/global entries)`, NEVER
        // `1` again, so `Rc::try_unwrap` FAILS UNCONDITIONALLY the moment
        // even one such entry exists. Reproduced directly, not
        // theorized: implementing the spec's own literal design made
        // `table_init_copy_elem_drop.rs`'s pre-existing
        // `active_element_segment_on_an_is64_table_applies_at_
        // instantiation_time` test (`(elem (table $t0) (i64.const 1) func
        // $one $two)` -- two of the module's OWN local functions, written
        // by the module's OWN active elem segment) panic on exactly this
        // `Rc::try_unwrap` call, on every run.
        //
        // The root problem: `instantiate()`'s own signature promises to
        // return a bare, OWNED `WasmInstance` -- it does not, and
        // structurally CANNOT, own that instance's long-term lifetime.
        // A `LocalFunctionRef`'s `Rc<RefCell<WasmInstance>>` is only ever
        // SOUND when SOMETHING holds a genuinely long-lived, permanent
        // `Rc` for the instance's whole real lifetime -- exactly what
        // `wasm-conformance`'s `ModuleRegistry` already does (`Rc<RefCell<
        // HashMap<..., Rc<RefCell<WasmInstance>>>>>`, held for an entire
        // script's lifetime, cycle-tolerant BY DESIGN per the spec's own
        // "Security and lifetime consideration" section: "a cycle within
        // one registry is harmless there, since the WHOLE registry, cycle
        // and all, is freed together"). `instantiate()` itself has no
        // such permanent home to offer -- ANY `Rc` it builds internally is
        // torn down (via `try_unwrap`, or dropped on failure) before this
        // function ever returns, so a `LocalFunctionRef` minted from it
        // could never survive the trip even in a `Weak`-based redesign:
        // `Rc::try_unwrap`'s own `into_inner()` frees the allocation a
        // `Weak` would need to `upgrade()` from, forever, the moment this
        // function returns -- there would be nothing left to upgrade to,
        // ever again, even after a caller re-wraps the RETURNED
        // `WasmInstance` in its OWN, unrelated, brand-new `Rc`.
        //
        // Resolution: `instantiate()` itself does NOT attempt real
        // cross-instance funcref resolution for its own elem-segment/
        // global-initializer entries -- they stay exactly as slice 2 left
        // them (`TableElement::Raw`/an unresolved raw index in
        // `GlobalStorage::value`), resolved LAZILY, on read, against
        // whichever ctx actually dispatches them (`resolve_function_ref_
        // for_dispatch`) -- which is EXACTLY CORRECT for the single-
        // instance case (the only case this slice's own corpus baseline
        // is expected to move for -- see this slice's own verification
        // notes) and a KNOWN, PRE-EXISTING, still-open gap for the
        // genuinely cross-instance case (unchanged by this slice, exactly
        // as it was before it). `LocalFunctionRef`/
        // `resolve_func_ref_for_instance`/`InstanceSelfResolver`/
        // `WasmRuntime::call_by_index` remain fully implemented, real,
        // and directly unit-tested (see this crate's own new tests) --
        // this is genuinely additive, tested infrastructure, ready for
        // slice 4 to invoke SAFELY from `wasm-conformance`'s own
        // `ModuleRegistry`, which (per the spec's own reasoning) is the
        // one place in this campaign's own architecture that can
        // actually sustain the permanent `Rc` a real cross-instance
        // `LocalFunctionRef` needs.
        //
        // `func_identities`/`instance_identity` (both plain `u64`s, no
        // `Rc` involved, no cycle possible) ARE populated for real above,
        // unaffected by this deviation -- `wasm-execution`'s own
        // `effective_local_index`/`FuncRefTarget::owner_instance_identity`
        // machinery (see that field's own doc comment) is real,
        // functioning infrastructure TODAY, for the one case this slice
        // DOES safely wire end-to-end: `build_engine`'s unconditional
        // `set_instance_identity`/`set_func_identities` calls.

        // Real corpus vendoring pass (`start.wast`/`start0.wast`, see
        // `wasm-conformance`'s CHANGELOG): the spec requires invoking a
        // module's start function, if it has one, as the LAST step of
        // instantiation -- after every memory/table/global/data/elem is
        // already in place, exactly like a normal exported call. This was
        // a real, previously undetected gap: `module.start` was parsed and
        // carried on `WasmModule` (see `wasm-wast-parser`'s own `"start"`
        // build arm) but nothing ever read it here, so a module's start
        // function silently never ran. `linking.wast` (already vendored)
        // has exercised this gap all along -- its own `assert_return`
        // tally already carries real, pre-existing fails from exactly
        // this; this fix is expected to newly turn those into passes,
        // not introduce a regression.
        //
        // `call_engine` re-threads `instance`'s memories/tables/etc.
        // through a fresh engine and restores them afterward regardless
        // of outcome (see that method's own doc comment) -- if the start
        // function itself traps, that's a genuine instantiation-time
        // trap, the same `Err` path any other instantiation-time fault
        // (a data/elem segment out of bounds) already takes.
        if let Some(start_idx) = module.start {
            if let Err(e) = self.call_engine(&mut instance, start_idx as usize, &[]) {
                // W35 fourth slice: the identical "ephemeral trap-discarded
                // instance" case the elem/data-segment loops handle above,
                // just for a START FUNCTION's own trap instead --
                // `linking3.wast`'s own `$Ms`/`"get table[0]"` example is
                // exactly this shape: an anonymous module's ACTIVE elem
                // segment (applied above, successfully, before this point)
                // writes into `$Ms`'s SHARED table, and only THEN does its
                // `(start $main)` call `unreachable`, discarding the
                // `WasmInstance` this function would otherwise return.
                // `instance` is already the REAL, fully-built value here
                // (unlike the elem/data case, no need to reconstruct a
                // temporary clone of it) -- move it into a temporary `Rc`,
                // fix up, and let it drop (never `try_unwrap`ed) exactly
                // like the elem/data path above. See that path's own doc
                // comment for the full "an `Rc` survives via whoever still
                // holds a clone" rationale.
                let temp_rc = Rc::new(RefCell::new(instance));
                let _ = resolve_all_table_funcrefs(&temp_rc);
                return Err(e);
            }
        }

        Ok(instance)
    }

    /// Call an exported function by name.
    pub fn call(
        &self,
        instance: &mut WasmInstance,
        name: &str,
        args: &[i64],
    ) -> Result<Vec<i64>, TrapError> {
        let (_, kind, index) = instance
            .exports
            .iter()
            .find(|(n, _, _)| n == name)
            .ok_or_else(|| TrapError::new(format!("export \"{}\" not found", name)))?;

        if *kind != ExternalKind::Function {
            return Err(TrapError::new(format!(
                "export \"{}\" is not a function",
                name
            )));
        }

        let func_index = *index as usize;
        let func_type = instance.func_types[func_index].clone();

        // Convert args to WasmValues.
        let wasm_args: Vec<WasmValue> = args
            .iter()
            .zip(func_type.params.iter())
            .map(|(&arg, &param_type)| match param_type {
                ValueType::I32 => WasmValue::I32(arg as i32),
                ValueType::I64 => WasmValue::I64(arg),
                ValueType::F32 => WasmValue::F32(arg as f32),
                ValueType::F64 => WasmValue::F64(arg as f64),
                // GC and funcref/externref reference types: pass the raw i64
                // as a null-pointer sentinel until the runtime grows native
                // GC support. This lossy path is `call()`'s pre-existing
                // legacy behavior; `call_typed()` should be used instead
                // when a real `WasmValue::Ref` needs to be passed.
                // `Exnref` (W-next) joins this same lossy-legacy-path
                // placeholder group -- never a real value in this repo
                // (see its own doc comment), and no vendored corpus
                // directive ever passes one as a top-level `invoke`
                // argument (only ever appears as a `try_table` catch
                // target's declared block-result type).
                // The four W32-first-slice bottom reference types join
                // this same lossy-legacy-path placeholder group -- no
                // vendored corpus directive passes one as a top-level
                // `invoke` argument either (only appears as a global/
                // function-result declared type in `ref_null.wast`).
                // W32 second slice: `NonNullStructRef`/
                // `NonNullConcreteFuncRef` join this same lossy-legacy-path
                // placeholder group -- no vendored corpus directive passes
                // one as a top-level `invoke` argument here either (this
                // slice's own real corpus wins, `call_ref.wast`/
                // `return_call_ref.wast`, exercise non-null concrete refs
                // only as PARAMS/LOCALS/GLOBALS inside a function body,
                // never as a top-level `call()` boundary argument).
                // W33 fourth slice: `ArrayRef`/`NonNullArrayRef` join this
                // same lossy-legacy-path placeholder group -- no vendored
                // corpus directive passes one as a top-level `invoke`
                // argument (arrays only ever appear as params/locals/globals
                // inside a function body in `array.wast`).
                // W37 (`code/specs/W37-wasm-gc-reftype-tables.md`): `Eqref`/
                // `StructRefAny` join this same lossy-legacy-path
                // placeholder group -- no vendored corpus directive passes
                // one as a top-level `invoke` argument either (both only
                // ever appear as table/global/local/param/result declared
                // types).
                ValueType::Anyref
                | ValueType::Eqref
                | ValueType::StructRefAny
                | ValueType::I31ref
                | ValueType::StructRef(_)
                | ValueType::ConcreteFuncRef(_)
                | ValueType::NonNullStructRef(_)
                | ValueType::NonNullConcreteFuncRef(_)
                | ValueType::ArrayRef(_)
                | ValueType::NonNullArrayRef(_)
                | ValueType::NonNullArrayAny
                | ValueType::Funcref
                | ValueType::Externref
                | ValueType::Exnref
                | ValueType::NullFuncref
                | ValueType::NullExternref
                | ValueType::NullExnref
                | ValueType::NullRef => WasmValue::I32(arg as i32),
                // v128 (SIMD): same lossy-legacy-path placeholder as the
                // reference types above -- `call()`'s i64 round-trip
                // cannot represent a 128-bit value at all; use `call_typed()`
                // for real v128 arguments. Passing handle 0 (the reserved
                // all-zero vector) is a deterministic, non-panicking choice,
                // not a real conversion.
                ValueType::V128 => WasmValue::V128(0),
            })
            .collect();

        let results = self.call_engine(instance, func_index, &wasm_args)?;

        // Convert back to i64.
        Ok(results
            .iter()
            .map(|r| match r {
                WasmValue::I32(v) => *v as i64,
                WasmValue::I64(v) => *v,
                WasmValue::F32(v) => *v as i64,
                WasmValue::F64(v) => *v as i64,
                // A WasmGC reference result (LANG77 L3b-3a-3b).  In the lisp
                // value model the return boundary unboxes integer results to
                // i64, so a *reference* reaching here is a structural result
                // (a cons or nil).  This i64 path can't represent one yet:
                // surface a deterministic, non-panicking placeholder — null
                // (nil) as 0, a heap reference as its raw handle.  Proper
                // reference-return handling lands with the cons e2e (L3b-3a-3c).
                WasmValue::Ref(None) => 0,
                WasmValue::Ref(Some(h)) => *h as i64,
                // v128 (SIMD): same lossy-legacy-path placeholder as `Ref`
                // above -- surface the raw v128_heap handle as an i64,
                // deterministic and non-panicking, not a real conversion.
                // Use `call_typed()` for a real `WasmValue::V128` result.
                WasmValue::V128(h) => *h as i64,
            })
            .collect())
    }

    /// Call an exported function by name with fully typed, bit-exact
    /// arguments and results.
    ///
    /// `call()` round-trips every value through `i64`, which is lossy for
    /// floats — its result conversion does `WasmValue::F32(v) => *v as i64`,
    /// a numeric *truncation*, not a bit reinterpretation. A caller that
    /// needs the exact IEEE-754 bit pattern back (for example, a
    /// conformance harness grading `assert_return` against a testsuite's
    /// `nan:0x<payload>` literal) cannot use `call()` for that. This method
    /// is purely additive: it shares `call()`'s export-lookup and
    /// engine-execution plumbing but skips the i64 round trip entirely.
    pub fn call_typed(
        &self,
        instance: &mut WasmInstance,
        name: &str,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>, TrapError> {
        let (_, kind, index) = instance
            .exports
            .iter()
            .find(|(n, _, _)| n == name)
            .ok_or_else(|| TrapError::new(format!("export \"{}\" not found", name)))?;

        if *kind != ExternalKind::Function {
            return Err(TrapError::new(format!(
                "export \"{}\" is not a function",
                name
            )));
        }

        let func_index = *index as usize;
        self.call_engine(instance, func_index, args)
    }

    /// Like [`Self::call_typed`], but also returns each result's real
    /// v128 bytes (`Some` for a `WasmValue::V128` result, `None` for
    /// every other result shape) -- see `code/specs/
    /// W13-wasm-simd-v128-first-slice.md`'s follow-up scope, and
    /// `wasm_execution::WasmExecutionEngine::call_function_with_v128`'s
    /// own doc comment for why a bare post-return `V128` handle can't be
    /// used directly (the engine that produced it is dropped by the time
    /// this returns). The two returned `Vec`s are always the same length
    /// and index-aligned with each other.
    pub fn call_typed_with_v128(
        &self,
        instance: &mut WasmInstance,
        name: &str,
        args: &[WasmValue],
    ) -> Result<(Vec<WasmValue>, Vec<Option<wasm_execution::V128Bytes>>), TrapError> {
        let (_, kind, index) = instance
            .exports
            .iter()
            .find(|(n, _, _)| n == name)
            .ok_or_else(|| TrapError::new(format!("export \"{}\" not found", name)))?;

        if *kind != ExternalKind::Function {
            return Err(TrapError::new(format!(
                "export \"{}\" is not a function",
                name
            )));
        }

        let func_index = *index as usize;
        self.call_engine_with_v128(instance, func_index, args)
    }

    /// Call a function by its raw combined index, whether or not it is
    /// exported (W35 third slice, design §3) -- the primitive
    /// `call`/`call_typed` (both export-name lookups) don't provide, and
    /// [`LocalFunctionRef`]/a future embedder needing real funcref
    /// identity does (an active `elem` segment, or a `ref.func`-
    /// initialized global, can name a function that is never exported at
    /// all -- `linking.wast`'s own `$Mt`/`$g` example). Purely additive:
    /// `call`/`call_typed` are unchanged, both still resolve a name first
    /// and then delegate to the SAME `call_engine` internally.
    pub fn call_by_index(&self, instance: &mut WasmInstance, func_index: usize, args: &[WasmValue]) -> Result<Vec<WasmValue>, TrapError> {
        self.call_engine(instance, func_index, args)
    }

    /// Shared by `call()` and `call_typed()`: build a `WasmExecutionEngine`
    /// from `instance`'s state, run `func_index`, and write the engine's
    /// post-call state back into `instance`. Neither caller-facing method
    /// duplicates this plumbing (memory/tables/host-functions ownership
    /// transfer, WasmGC struct field count wiring).
    /// Shared by `call_engine`/`call_engine_with_v128`: build a
    /// `WasmExecutionEngine` from `instance`'s state (transferring
    /// memory/tables/host-functions ownership temporarily) and register
    /// its type section and WasmGC struct field counts. Neither caller
    /// re-derives this setup -- see the inline comments at each step
    /// (preserved from before this was split out) for why each one
    /// matters.
    fn build_engine(&self, instance: &mut WasmInstance) -> WasmExecutionEngine {
        // Build engine config, transferring ownership temporarily.
        let memories = std::mem::take(&mut instance.memories);
        let tables = std::mem::take(&mut instance.tables);
        let host_functions = std::mem::take(&mut instance.host_functions);

        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories,
            tables,
            globals: instance.globals.clone(),
            global_types: instance.global_types.clone(),
            func_types: instance.func_types.clone(),
            func_bodies: instance.func_bodies.clone(),
            host_functions,
        });

        // Register the module's real type section (indexed by TYPE index,
        // not function index — see `set_type_section`'s own doc comment) so
        // `call_indirect $type` checks the callee against what the call
        // site actually declared instead of skipping the check.
        engine.set_type_section(instance.module.types.clone());

        // Thread the module's GC-proposal nominal-subtyping metadata (W33
        // second slice, item 4) and the combined func_index -> declared
        // type-index space, so `call_indirect`'s real subtype check and
        // `ref.cast`/`ref.test`'s dynamic type check have what they need —
        // same optional-setter pattern as `set_type_section` immediately
        // above. Left unset, both fall back to their pre-W33 behavior (see
        // `wasm-execution`'s own `call_indirect_type_matches`/`ref_matches_
        // concrete_type` doc comments for why that fallback is exactly
        // right for a module that never declares `sub`).
        engine.set_type_subtyping(instance.module.type_subtyping.clone());
        engine.set_func_type_indices(instance.func_type_indices.clone());

        // Thread this module's own canonicalized type-group forms (W34
        // third slice: `code/specs/W34-wasm-gc-canonical-type-equivalence.md`),
        // same optional-setter pattern as `set_type_subtyping` immediately
        // above -- `instance.canonical_types` (NOT `instance.module`,
        // which never carries this data at all -- see `WasmInstance::
        // canonical_types`'s own doc comment for why) was cloned once from
        // `ValidatedModule` at `instantiate()` time. Left unset, `call_
        // indirect_type_matches`/`ref_matches_concrete_type` fall back to
        // nominal-only dispatch, unchanged from before this slice.
        engine.set_canonical_types(instance.canonical_types.clone());

        // Thread the module's tag section (W-next: real catch-clause
        // matching) so `throw`/`catch` know each tag's declared param
        // types -- same optional-setter pattern as `set_type_section`
        // immediately above.
        // `instance.tags` (the COMBINED imported+defined index space
        // built at `instantiate()` time), NOT `instance.module.tags`
        // (which, like `module.functions`, holds only module-DEFINED
        // tags' type indices -- imports live separately in `module.
        // imports`). Passing the module-only field here was a real bug
        // (W-next): `throw $tag`/`catch $tag` encode the COMBINED index
        // space, so a module with any tag IMPORTS looked up every
        // LOCALLY-declared tag's type at the WRONG (off-by-import-count)
        // slot -- silent until real tag/type lookups (this slice) started
        // reading `ctx.tags` for anything observable; W21 itself never
        // read this field at runtime, so this went undetected until now.
        engine.set_tags(instance.tags.clone());

        // Thread the module's canonical, cross-instance-safe tag
        // identities (W23), same combined index space as `set_tags`
        // immediately above -- see `WasmInstance::tag_identities`'s own
        // doc comment for why this must be `instance.tag_identities`
        // (persistent, minted once at `instantiate()` time) rather than
        // anything recomputed per call.
        engine.set_tag_identities(instance.tag_identities.clone());

        // Thread the instance's persistent v128 heap into the engine (see
        // `code/specs/W15-wasm-v128-persistent-storage.md`) -- same
        // optional-setter pattern as `set_type_section`/
        // `set_struct_field_counts` above, so a v128-typed global's
        // handle stays valid across this call instead of indexing into a
        // throwaway per-call heap.
        engine.set_v128_heap(instance.v128_heap.clone());

        // Thread the module's data segments' raw bytes -- `memory.init`'s
        // source (task #95) -- plus this instance's per-segment dropped
        // state, same optional-setter pattern as `set_v128_heap` just
        // above (content immutable, so no restore needed for the bytes
        // themselves; the dropped flags DO need restoring, same as
        // `v128_heap`, in `call_engine`/`call_engine_with_v128` below).
        engine.set_data_segments(instance.module.data.iter().map(|seg| seg.data.clone()).collect());
        engine.set_dropped_data_segments(instance.dropped_data_segments.clone());

        // Thread the module's element segments' function-index lists --
        // `table.init`'s source (task #97) -- plus this instance's
        // per-segment dropped state, same shape/reasoning as the data-
        // segment threading just above.
        engine.set_elements(instance.module.elements.iter().map(|elem| elem.function_indices.clone()).collect());
        engine.set_dropped_elements(instance.dropped_elements.clone());

        // Register the module's WasmGC struct field counts (LANG77 / McCarthy
        // L3b-3a-3c-2) so the engine knows how many fields each `struct.new`
        // allocates — without this, a `struct.new` traps with "no field count
        // registered". Previously the embedder had to call this by hand; now it
        // flows automatically from the parsed module's `struct_types`.
        //
        // W33 fourth slice: rebuilt on top of `WasmModule::struct_type_at`/
        // `array_type_at` (`type_kinds`-aware, see those methods' own doc
        // comments) instead of the OLD "pad `func_type_count` zeros, then
        // append every struct's field count in `struct_types` order" scheme
        // this comment used to describe. That old scheme's own documented
        // assumption — "struct types follow ALL function types" — is exactly
        // what a TEXT-format module (via `wasm-wast-parser`'s now-real struct/
        // array declarations) is free to violate: `struct.wast`'s/
        // `array.wast`'s own "Binding structure" modules declare a struct/array
        // type, THEN a function whose inline-only signature gets dedup'd into
        // `types` AFTER it — see `wasm_types::TypeKind`'s own doc comment for
        // the full mechanism. Iterating every flat index up to the total type
        // count and asking each one directly "are you a struct/array, and if
        // so what's your shape" is correct regardless of declaration order —
        // the LANG77/Twig binary-only modules this comment used to describe
        // (which never populate `type_kinds` at all) still resolve identically
        // via `struct_type_at`'s legacy-offset fallback, so this is a strict
        // generalization, not a behavior change for any pre-existing caller.
        let (struct_field_counts, struct_field_storage, array_element_storage) = struct_array_runtime_tables(&instance.module);
        if !struct_field_counts.is_empty() {
            engine.set_struct_field_counts(struct_field_counts);
            engine.set_struct_field_storage(struct_field_storage);
            engine.set_array_element_storage(array_element_storage);
        }

        // Seed the engine's persistent GC object heap from the instance's
        // own (W33 fourth slice) -- see `WasmInstance::gc_heap`'s own doc
        // comment for why a global initializer's struct/array must survive
        // into this call, not just the instantiation call that created it.
        engine.set_gc_heap(instance.gc_heap.clone());

        // Thread the module's canonical, cross-instance-safe FUNCTION
        // identities (W35 third slice), same combined index space and
        // same optional-setter pattern as `set_tag_identities` above --
        // see `WasmInstance::func_identities`'s own doc comment. A plain
        // `Vec<u64>` clone, no `Rc` needed, so this is safe to set
        // unconditionally on every call.
        engine.set_func_identities(instance.func_identities.clone());

        // Register which `WasmInstance` this engine's context belongs to
        // (W35 third slice) -- see `WasmInstance::instance_identity`'s own
        // doc comment. Also just a plain `u64` copy, safe unconditionally.
        engine.set_instance_identity(instance.instance_identity);

        // **Deliberately NOT calling `engine.set_self_resolver(..)` here**
        // (a further, documented deviation from the spec's own literal §4
        // text, which describes `build_engine` installing a resolver for
        // ordinary per-call execution too) -- see `wasm_execution::
        // WasmExecutionContext::self_resolver`'s own doc comment for the
        // full rationale: this method only ever has a plain `&mut
        // WasmInstance` to work with, never an `Rc<RefCell<WasmInstance>>`,
        // and constructing an `InstanceSelfResolver` needs to hold and
        // clone that `Rc`. Making that `Rc` available here would require
        // either (a) breaking `call`/`call_typed`'s own public signature
        // (explicitly out of this slice's stated scope -- "Purely
        // additive: `call`/`call_typed` are unchanged"), or (b) new
        // cross-module wiring in `wasm-conformance`'s `Executor`/
        // `ModuleRegistry` to re-establish a self-reference on every
        // `Rc`-wrap it performs (explicitly slice 4's job, not this
        // slice's). `instantiate()` itself ALSO never constructs an
        // `InstanceSelfResolver` (see that function's own doc comment: a
        // real, reproduced `Rc::try_unwrap` failure -- an unavoidable
        // self-referential cycle whenever a module's own elem/global entry
        // references its own local function -- forced that back out).
        // `InstanceSelfResolver`/`LocalFunctionRef` are exercised directly
        // by this slice's own unit tests instead (a long-lived, test-owned
        // `Rc<RefCell<WasmInstance>>`, never unwrapped), proving the
        // machinery works and is ready for slice 4 to wire up safely.
        engine
    }

    /// Shared by `call()` and `call_typed()`: build a `WasmExecutionEngine`
    /// from `instance`'s state, run `func_index`, and write the engine's
    /// post-call state back into `instance`. Neither caller-facing method
    /// duplicates this plumbing (memory/tables/host-functions ownership
    /// transfer, WasmGC struct field count wiring).
    fn call_engine(
        &self,
        instance: &mut WasmInstance,
        func_index: usize,
        wasm_args: &[WasmValue],
    ) -> Result<Vec<WasmValue>, TrapError> {
        let mut engine = self.build_engine(instance);

        // Run the call, but restore `instance`'s memory/tables/host-functions
        // from the engine's post-call state REGARDLESS of whether it trapped
        // (`call_function` takes `&mut self`, so `engine` — and everything
        // `mem::take` moved into it above — is still fully intact even on
        // `Err`). Using `?` directly on `call_function` here used to skip
        // this restoration on any trap, silently and permanently leaving
        // `instance.memories`/`instance.tables` empty (`vec![]`) for the
        // rest of that instance's lifetime: the
        // FIRST call that trapped for any reason (an intentionally-trapping
        // test, or a real bug) would make every subsequent call on the same
        // instance fail with a spurious "no memory available"/"undefined
        // table", masking whatever the test was actually checking.
        let result = engine.call_function(func_index, wasm_args);
        let state = engine.into_state();
        instance.memories = state.memories;
        instance.tables = state.tables;
        instance.globals = state.globals;
        instance.host_functions = state.host_functions;
        instance.v128_heap = state.v128_heap;
        instance.gc_heap = state.gc_heap;
        instance.dropped_data_segments = state.dropped_data_segments;
        instance.dropped_elements = state.dropped_elements;

        result
    }

    /// Like [`Self::call_engine`], but calls `call_function_with_v128`
    /// instead of `call_function`, threading its extra resolved-v128-
    /// bytes return value straight through. Same state-restore-
    /// regardless-of-trap discipline as `call_engine` -- see its own
    /// comment for why that matters.
    fn call_engine_with_v128(
        &self,
        instance: &mut WasmInstance,
        func_index: usize,
        wasm_args: &[WasmValue],
    ) -> Result<(Vec<WasmValue>, Vec<Option<wasm_execution::V128Bytes>>), TrapError> {
        let mut engine = self.build_engine(instance);

        let result = engine.call_function_with_v128(func_index, wasm_args);
        let state = engine.into_state();
        instance.memories = state.memories;
        instance.tables = state.tables;
        instance.globals = state.globals;
        instance.host_functions = state.host_functions;
        instance.v128_heap = state.v128_heap;
        instance.gc_heap = state.gc_heap;
        instance.dropped_data_segments = state.dropped_data_segments;
        instance.dropped_elements = state.dropped_elements;

        result
    }

    /// Parse, validate, instantiate, and call in one step.
    pub fn load_and_run(
        &self,
        wasm_bytes: &[u8],
        entry: &str,
        args: &[i64],
    ) -> Result<Vec<i64>, String> {
        let module = self.load(wasm_bytes)?;
        let validated = self.validate(&module).map_err(|e| format!("{}", e))?;
        let mut instance = self.instantiate(&validated).map_err(|e| format!("{}", e))?;
        self.call(&mut instance, entry, args)
            .map_err(|e| format!("{}", e))
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_types::*;

    struct TestHostFunction {
        func_type: FuncType,
    }

    impl HostFunction for TestHostFunction {
        fn func_type(&self) -> &FuncType {
            &self.func_type
        }

        fn call(
            &self,
            args: &[WasmValue],
            _memory: Option<&mut LinearMemory>,
        ) -> Result<Vec<WasmValue>, TrapError> {
            let value = args
                .first()
                .ok_or_else(|| TrapError::new("missing argument"))?
                .as_i32()?;
            Ok(vec![WasmValue::I32(value * 2)])
        }
    }

    struct TestHost;

    impl HostInterface for TestHost {
        fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>> {
            if module_name == "env" && name == "double" {
                Some(Box::new(TestHostFunction {
                    func_type: FuncType {
                        params: vec![ValueType::I32],
                        results: vec![ValueType::I32],
                    },
                }))
            } else {
                None
            }
        }

        fn resolve_global(
            &self,
            _module_name: &str,
            _name: &str,
        ) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
            None
        }

        fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
            None
        }

        fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
            None
        }
    }

    // ── W33 first slice: cross-module `rec`-group shape guard ───────────────

    /// A host function that reports a caller-chosen `(rec_group_size,
    /// rec_group_position)` (W33 first slice) instead of the trait's own
    /// `(1, 0)` default -- lets these tests simulate a `rec`-group member
    /// without going through a real cross-module `CrossModuleFunction`.
    struct GroupShapeHostFunction {
        func_type: FuncType,
        group_shape: (u32, u32),
        is_final: bool,
    }

    impl HostFunction for GroupShapeHostFunction {
        fn func_type(&self) -> &FuncType {
            &self.func_type
        }
        fn call(&self, _args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
            Ok(vec![])
        }
        fn type_group_shape(&self) -> (u32, u32) {
            self.group_shape
        }
        fn is_final(&self) -> bool {
            self.is_final
        }
    }

    struct GroupShapeHost {
        group_shape: (u32, u32),
        is_final: bool,
    }

    impl HostInterface for GroupShapeHost {
        fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>> {
            if module_name == "env" && name == "f" {
                Some(Box::new(GroupShapeHostFunction { func_type: FuncType { params: vec![], results: vec![] }, group_shape: self.group_shape, is_final: self.is_final }))
            } else {
                None
            }
        }
        fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
            None
        }
        fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
            None
        }
        fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
            None
        }
    }

    /// A module importing "env"."f" as a function whose OWN declared type
    /// (index 0) carries the given `(rec_group_size, rec_group_position)`
    /// and finality.
    fn module_importing_function_in_a_rec_group(group_shape: (u32, u32)) -> WasmModule {
        module_importing_function_in_a_rec_group_with_finality(group_shape, true)
    }

    fn module_importing_function_in_a_rec_group_with_finality(group_shape: (u32, u32), is_final: bool) -> WasmModule {
        WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![TypeSubtyping { rec_group_size: group_shape.0, rec_group_position: group_shape.1, is_final, ..Default::default() }],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "f".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn rejects_a_function_import_whose_rec_group_position_mismatches() {
        // `tag.wast`'s own `assert_unlinkable` shape, reduced to a
        // function import: the module declares its type as position 0 of
        // a 2-member group; the host reports position 1 of a
        // structurally-identical group. A PLAIN `FuncType` shape
        // comparison alone (both are `(func)`) would wrongly accept this
        // -- the new `type_group_shape` guard must catch it.
        let runtime = WasmRuntime::with_host(Box::new(GroupShapeHost { group_shape: (2, 1), is_final: true }));
        let module = module_importing_function_in_a_rec_group((2, 0));
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn rejects_a_function_import_whose_finality_mismatches() {
        // `type-subtyping.wast` lines 594-617: `(sub (func))` (open) and
        // `(sub final (func))` (final) are structurally IDENTICAL
        // `FuncType`s (both empty), yet distinct canonical types --
        // finality is as much a part of a type's real identity as its
        // shape. The importer declares its type as OPEN (non-final); the
        // host reports the exported function's real type as FINAL.
        let runtime = WasmRuntime::with_host(Box::new(GroupShapeHost { group_shape: (1, 0), is_final: true }));
        let module = module_importing_function_in_a_rec_group_with_finality((1, 0), false);
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn accepts_a_function_import_whose_finality_matches() {
        // The positive counterpart: both sides OPEN (non-final) links
        // fine, same as every pre-W33 import (which implicitly used the
        // final-by-default value on both sides).
        let runtime = WasmRuntime::with_host(Box::new(GroupShapeHost { group_shape: (1, 0), is_final: false }));
        let module = module_importing_function_in_a_rec_group_with_finality((1, 0), false);
        let validated = runtime.validate(&module).unwrap();
        assert!(runtime.instantiate(&validated).is_ok());
    }

    #[test]
    fn accepts_a_function_import_whose_rec_group_shape_matches() {
        // The positive counterpart: identical `(rec_group_size,
        // rec_group_position)` on both sides links fine, same as every
        // pre-W33 import (which implicitly used the singleton-group
        // default `(1, 0)` on both sides).
        let runtime = WasmRuntime::with_host(Box::new(GroupShapeHost { group_shape: (2, 0), is_final: true }));
        let module = module_importing_function_in_a_rec_group((2, 0));
        let validated = runtime.validate(&module).unwrap();
        assert!(runtime.instantiate(&validated).is_ok());
    }

    // ── W34 fourth slice: cross-module canonical type-group equivalence
    // (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`) ──────────────

    /// A host function that reports a real `CanonicalGroup` identity
    /// instead of the trait's own `None` default -- lets these tests
    /// exercise `wasm-runtime`'s new canonical-comparison path directly,
    /// without needing a real cross-module `CrossModuleFunction` (that
    /// full end-to-end path is covered separately by `wasm-conformance`'s
    /// own tests). Deliberately reports a `func_type()` with a raw
    /// `ConcreteFuncRef` index that does NOT match the importing module's
    /// own raw index -- proving these tests exercise REAL canonical
    /// equivalence, not a coincidental raw-structural match the old
    /// three-part guard could already have accepted.
    struct CanonicalHostFunction {
        func_type: FuncType,
        canonical_type: Option<(Rc<CanonicalGroup>, u32)>,
    }

    impl HostFunction for CanonicalHostFunction {
        fn func_type(&self) -> &FuncType {
            &self.func_type
        }
        fn call(&self, _args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
            Ok(vec![])
        }
        fn canonical_type(&self) -> Option<(Rc<CanonicalGroup>, u32)> {
            self.canonical_type.clone()
        }
    }

    struct CanonicalHost {
        func_type: FuncType,
        canonical_type: Option<(Rc<CanonicalGroup>, u32)>,
    }

    impl HostInterface for CanonicalHost {
        fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>> {
            if module_name == "env" && name == "f" {
                Some(Box::new(CanonicalHostFunction { func_type: self.func_type.clone(), canonical_type: self.canonical_type.clone() }))
            } else {
                None
            }
        }
        fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
            None
        }
        fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
            None
        }
        fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
            None
        }
    }

    /// A module declaring a single self-referencing singleton `rec` type
    /// at flat index 0 (`type-rec.wast`'s own simplest self-reference
    /// shape) and importing "env"."f" at it.
    fn module_importing_a_self_referencing_function() -> WasmModule {
        WasmModule {
            types: vec![FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![] }],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "f".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            ..Default::default()
        }
    }

    /// A throwaway `WasmModule` used only to derive a real, self-contained
    /// `CanonicalGroup` for a host function to report -- the SAME
    /// self-referencing shape `module_importing_a_self_referencing_function`
    /// declares, but at flat index `offset` in a completely unrelated
    /// table (no shared numbering with the importing module at all).
    fn host_side_self_referencing_canonical_type(offset: u32) -> (Rc<CanonicalGroup>, u32) {
        let padding = FuncType { params: vec![], results: vec![] };
        let mut types = vec![padding; offset as usize];
        types.push(FuncType { params: vec![ValueType::ConcreteFuncRef(offset)], results: vec![] });
        let module = WasmModule { types, ..Default::default() };
        wasm_types::canonicalize_types(&module)[offset as usize].clone().expect("self-referencing singleton must canonicalize")
    }

    #[test]
    fn accepts_a_cross_module_import_via_canonical_equivalence_when_raw_indices_differ() {
        // Host's OWN raw `func_type` self-references index 7 -- deliberately
        // NOT index 0 (the importing module's own raw index) -- so plain
        // `FuncType` equality (the pre-W34-fourth-slice-only fallback path)
        // would reject this. Real canonical equivalence must accept it
        // anyway, since both sides tie to the identical `Rec(0)` shape.
        let host_func_type = FuncType { params: vec![ValueType::ConcreteFuncRef(7)], results: vec![] };
        let host_canonical = host_side_self_referencing_canonical_type(3);
        let runtime = WasmRuntime::with_host(Box::new(CanonicalHost { func_type: host_func_type, canonical_type: Some(host_canonical) }));
        let module = module_importing_a_self_referencing_function();
        let validated = runtime.validate(&module).unwrap();
        assert!(runtime.instantiate(&validated).is_ok(), "canonically-equivalent self-referencing types at different raw indices must link");
    }

    #[test]
    fn rejects_a_cross_module_import_whose_canonical_type_genuinely_differs() {
        // The host reports a canonical type for a NON-self-referencing
        // empty `(func)` -- genuinely different from the importing
        // module's self-referencing `(func (param (ref 0)))` -- even
        // though the host's raw `func_type()` HAPPENS to structurally
        // equal neither (irrelevant here, since a real canonical type on
        // both sides bypasses the raw-`FuncType`-equality fallback
        // entirely; only the canonical comparison decides).
        let unrelated_module = WasmModule { types: vec![FuncType { params: vec![], results: vec![] }], ..Default::default() };
        let unrelated_canonical = wasm_types::canonicalize_types(&unrelated_module)[0].clone().unwrap();
        let host_func_type = FuncType { params: vec![], results: vec![] };
        let runtime = WasmRuntime::with_host(Box::new(CanonicalHost { func_type: host_func_type, canonical_type: Some(unrelated_canonical) }));
        let module = module_importing_a_self_referencing_function();
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn falls_back_to_the_conservative_guard_when_the_host_reports_no_canonical_type() {
        // The trait default (`canonical_type() -> None`) must still hit the
        // pre-existing three-part guard, byte-for-byte -- a plain,
        // non-self-referencing `(func)` on both sides links fine via the
        // fallback path alone, exactly as it did before this slice.
        let host_func_type = FuncType { params: vec![], results: vec![] };
        let runtime = WasmRuntime::with_host(Box::new(CanonicalHost { func_type: host_func_type, canonical_type: None }));
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "f".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        assert!(runtime.instantiate(&validated).is_ok());
    }

    /// Build the raw WASM binary for a square(x) = x * x function.
    ///
    /// This is a minimal valid .wasm file containing:
    /// - Type section: (i32) -> i32
    /// - Function section: function 0 uses type 0
    /// - Export section: exports "square" as function 0
    /// - Code section: local.get 0; local.get 0; i32.mul; end
    fn build_square_wasm() -> Vec<u8> {
        let mut wasm = Vec::new();

        // Magic + version
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // \0asm
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1

        // Type section (id=1)
        // 1 type: (i32) -> i32
        let type_section = vec![
            0x01, // 1 type
            0x60, // func type
            0x01, 0x7F, // 1 param: i32
            0x01, 0x7F, // 1 result: i32
        ];
        wasm.push(0x01); // section id
        wasm.push(type_section.len() as u8); // section size
        wasm.extend_from_slice(&type_section);

        // Function section (id=3)
        // 1 function referencing type 0
        let func_section = vec![
            0x01, // 1 function
            0x00, // type index 0
        ];
        wasm.push(0x03);
        wasm.push(func_section.len() as u8);
        wasm.extend_from_slice(&func_section);

        // Export section (id=7)
        // Export "square" as function 0
        let export_section = vec![
            0x01, // 1 export
            0x06, // name length 6
            b's', b'q', b'u', b'a', b'r', b'e', // "square"
            0x00, // export kind: function
            0x00, // function index 0
        ];
        wasm.push(0x07);
        wasm.push(export_section.len() as u8);
        wasm.extend_from_slice(&export_section);

        // Code section (id=10)
        // 1 function body: local.get 0; local.get 0; i32.mul; end
        let body = vec![
            0x00, // 0 local declarations
            0x20, 0x00, // local.get 0
            0x20, 0x00, // local.get 0
            0x6C, // i32.mul
            0x0B, // end
        ];
        let body_with_size = {
            let mut v = vec![body.len() as u8];
            v.extend_from_slice(&body);
            v
        };
        let code_section = {
            let mut v = vec![0x01u8]; // 1 body
            v.extend_from_slice(&body_with_size);
            v
        };
        wasm.push(0x0A);
        wasm.push(code_section.len() as u8);
        wasm.extend_from_slice(&code_section);

        wasm
    }

    #[test]
    fn test_runtime_square_end_to_end() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "square", &[5]);
        assert_eq!(result.unwrap(), vec![25]);
    }

    /// Hand-assemble a WasmGC module that computes `(CAR (CONS 7 9))`:
    ///
    /// ```wat
    /// (type $f (func (result i32)))
    /// (type $LispyPair (struct (field (mut anyref)) (field (mut anyref))))
    /// (func (export "main") (result i32)
    ///   i32.const 7  ref.i31          ;; car = box 7
    ///   i32.const 9  ref.i31          ;; cdr = box 9
    ///   struct.new $LispyPair         ;; (7 . 9)
    ///   struct.get $LispyPair 0       ;; car  -> i31ref
    ///   i31.get_s)                    ;; unbox -> 7
    /// ```
    ///
    /// The struct type is the **second** type (wasm type index 1, after the one
    /// function type), so `struct.new`/`struct.get` reference type index 1 — the
    /// exact case the L3b-3a-3c-2 arity wiring must resolve from the parsed
    /// `struct_types`.
    fn build_cons_car_wasm() -> Vec<u8> {
        let mut wasm = Vec::new();
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // magic
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1

        // Type section (id=1): a func type then the $LispyPair struct type.
        let type_section = vec![
            0x02, // 2 types
            // type 0: () -> i32
            0x60, 0x00, 0x01, 0x7F, // type 1: $LispyPair — struct { mut anyref, mut anyref }
            0x50, 0x00, 0x5F, // sub-type, 0 supers, struct marker
            0x02, // 2 fields
            0x6E, 0x01, // field 0: anyref, mutable
            0x6E, 0x01, // field 1: anyref, mutable
        ];
        wasm.push(0x01);
        wasm.push(type_section.len() as u8);
        wasm.extend_from_slice(&type_section);

        // Function section (id=3): 1 function of type 0.
        let func_section = vec![0x01, 0x00];
        wasm.push(0x03);
        wasm.push(func_section.len() as u8);
        wasm.extend_from_slice(&func_section);

        // Export section (id=7): export "main" as function 0.
        let export_section = vec![
            0x01, // 1 export
            0x04, b'm', b'a', b'i', b'n', // name "main"
            0x00, 0x00, // kind: function, index 0
        ];
        wasm.push(0x07);
        wasm.push(export_section.len() as u8);
        wasm.extend_from_slice(&export_section);

        // Code section (id=10): the (CAR (CONS 7 9)) body.
        let body = vec![
            0x00, // 0 local declarations
            0x41, 0x07, 0xFB, 0x1C, // i32.const 7 ; ref.i31  (box 7)
            0x41, 0x09, 0xFB, 0x1C, // i32.const 9 ; ref.i31  (box 9)
            0xFB, 0x00, 0x01, // struct.new $LispyPair (type 1)
            0xFB, 0x02, 0x01, 0x00, // struct.get $LispyPair 0  (car)
            0xFB, 0x1D, // i31.get_s  (unbox)
            0x0B, // end
        ];
        let code_section = {
            let mut v = vec![0x01u8]; // 1 body
            v.push(body.len() as u8);
            v.extend_from_slice(&body);
            v
        };
        wasm.push(0x0A);
        wasm.push(code_section.len() as u8);
        wasm.extend_from_slice(&code_section);

        wasm
    }

    #[test]
    fn test_runtime_runs_cons_car_struct_module() {
        // The L3b-3a-3c-2 capstone: a parsed WasmGC *struct* module runs on the
        // in-repo runtime with NO manual `set_struct_field_counts` — the arity
        // is derived from the module's parsed `struct_types`. `(CAR (CONS 7 9))`
        // → 7.
        let wasm = build_cons_car_wasm();
        let runtime = WasmRuntime::new();

        // Use the explicit load → instantiate → call path (exactly what the
        // arity wiring touches).
        let module = runtime.load(&wasm).expect("cons module must parse");
        assert_eq!(module.struct_types.len(), 1, "the $LispyPair struct is parsed");
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).expect("must instantiate");
        let result = runtime
            .call(&mut instance, "main", &[])
            .expect("cons module must run");
        assert_eq!(result, vec![7], "(CAR (CONS 7 9)) must evaluate to 7");
    }

    #[test]
    fn test_runtime_cons_module_traps_without_arity_wiring_is_now_wired() {
        // Regression guard: before L3b-3a-3c-2 this trapped with "no field count
        // registered for struct type 1". The arity wiring must make it succeed.
        let wasm = build_cons_car_wasm();
        let runtime = WasmRuntime::new();
        let result = runtime.load_and_run(&wasm, "main", &[]);
        assert_eq!(result.expect("must run end-to-end via load_and_run"), vec![7]);
    }

    #[test]
    fn test_runtime_square_negative() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "square", &[-3]);
        assert_eq!(result.unwrap(), vec![9]);
    }

    #[test]
    fn test_runtime_square_zero() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "square", &[0]);
        assert_eq!(result.unwrap(), vec![0]);
    }

    #[test]
    fn test_runtime_nonexistent_export() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "nonexistent", &[5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_validate_and_instantiate() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let module = runtime.load(&wasm).unwrap();
        let _validated = runtime.validate(&module).unwrap();
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).unwrap();
        let result = runtime.call(&mut instance, "square", &[7]).unwrap();
        assert_eq!(result, vec![49]);
    }

    #[test]
    fn test_wasi_stub_creation() {
        let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let output_clone = output.clone();
        let _wasi = WasiStub::new(move |text: &str| {
            output_clone.lock().unwrap().push(text.to_string());
        });
    }

    #[test]
    fn test_wasi_host_alias_creation() {
        let host = WasiHost::new(WasiConfig::default());
        assert!(host.memory.borrow().is_none());
    }

    #[test]
    fn test_proc_exit_error() {
        let err = ProcExitError { exit_code: 0 };
        assert_eq!(format!("{}", err), "proc_exit(0)");
    }

    #[test]
    fn test_proc_exit_error_nonzero() {
        let err = ProcExitError { exit_code: 1 };
        assert_eq!(format!("{}", err), "proc_exit(1)");
        assert_eq!(err.exit_code, 1);
    }

    #[test]
    fn test_proc_exit_is_error_trait() {
        let err = ProcExitError { exit_code: 42 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_runtime_default() {
        let runtime = WasmRuntime::default();
        // Default runtime should have no host
        let wasm = build_square_wasm();
        let result = runtime.load_and_run(&wasm, "square", &[3]);
        assert_eq!(result.unwrap(), vec![9]);
    }

    #[test]
    fn test_runtime_load_invalid_wasm() {
        let runtime = WasmRuntime::new();
        let result = runtime.load(&[0x00, 0x01, 0x02, 0x03]);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_validate_valid_module() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let module = runtime.load(&wasm).unwrap();
        assert!(runtime.validate(&module).is_ok());
    }

    #[test]
    fn test_runtime_instantiate() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let module = runtime.load(&wasm).unwrap();
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();

        // Check that exports were populated
        assert!(!instance.exports.is_empty());
        assert_eq!(instance.exports[0].0, "square");
        assert_eq!(instance.exports[0].1, ExternalKind::Function);
    }

    #[test]
    fn test_runtime_call_wrong_export_type() {
        // Build a module that exports a memory, then try to call it as a function
        let mut wasm = Vec::new();
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // magic
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version

        // Memory section (id=5): 1 memory, min=1, no max
        let mem_section = vec![0x01, 0x00, 0x01]; // 1 memory, limits flag 0, min 1
        wasm.push(0x05);
        wasm.push(mem_section.len() as u8);
        wasm.extend_from_slice(&mem_section);

        // Export section (id=7): export "mem" as memory 0
        let export_section = vec![
            0x01, // 1 export
            0x03, // name length
            b'm', b'e', b'm', 0x02, // memory export kind
            0x00, // memory index 0
        ];
        wasm.push(0x07);
        wasm.push(export_section.len() as u8);
        wasm.extend_from_slice(&export_section);

        let runtime = WasmRuntime::new();
        let result = runtime.load_and_run(&wasm, "mem", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a function"));
    }

    #[test]
    fn test_runtime_with_memory() {
        // Build a module with memory that stores and loads a value
        // func store_and_load(val: i32) -> i32:
        //   i32.const 0; local.get 0; i32.store; i32.const 0; i32.load; end
        let mut wasm = Vec::new();
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]);
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        // Type section: (i32) -> i32
        let type_section = vec![0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F];
        wasm.push(0x01);
        wasm.push(type_section.len() as u8);
        wasm.extend_from_slice(&type_section);

        // Function section
        let func_section = vec![0x01, 0x00];
        wasm.push(0x03);
        wasm.push(func_section.len() as u8);
        wasm.extend_from_slice(&func_section);

        // Memory section: 1 page min, no max
        let mem_section = vec![0x01, 0x00, 0x01];
        wasm.push(0x05);
        wasm.push(mem_section.len() as u8);
        wasm.extend_from_slice(&mem_section);

        // Export section
        let export_section = vec![0x01, 0x04, b't', b'e', b's', b't', 0x00, 0x00];
        wasm.push(0x07);
        wasm.push(export_section.len() as u8);
        wasm.extend_from_slice(&export_section);

        // Code section
        let body = vec![
            0x00, // 0 locals
            0x41, 0x00, // i32.const 0 (addr)
            0x20, 0x00, // local.get 0 (val)
            0x36, 0x02, 0x00, // i32.store align=2 offset=0
            0x41, 0x00, // i32.const 0 (addr)
            0x28, 0x02, 0x00, // i32.load align=2 offset=0
            0x0B, // end
        ];
        let body_with_size = {
            let mut v = vec![body.len() as u8];
            v.extend_from_slice(&body);
            v
        };
        let code_section = {
            let mut v = vec![0x01u8];
            v.extend_from_slice(&body_with_size);
            v
        };
        wasm.push(0x0A);
        wasm.push(code_section.len() as u8);
        wasm.extend_from_slice(&code_section);

        let runtime = WasmRuntime::new();
        let result = runtime.load_and_run(&wasm, "test", &[42]);
        assert_eq!(result.unwrap(), vec![42]);
    }

    #[test]
    fn test_runtime_with_global() {
        // Module with a mutable global initialized to 100
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x23, 0x00, 0x0B], // global.get 0; end
            }],
            globals: vec![Global {
                global_type: GlobalType {
                    value_type: ValueType::I32,
                    mutable: true,
                },
                init_expr: vec![0x41, 0xE4, 0x00, 0x0B], // i32.const 100; end (100 in signed LEB128)
            }],
            exports: vec![Export {
                name: "get_global".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            ..Default::default()
        };

        let runtime = WasmRuntime::new();
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).unwrap();
        let result = runtime.call(&mut instance, "get_global", &[]).unwrap();
        assert_eq!(result, vec![100]);
    }

    #[test]
    fn test_runtime_with_data_segment() {
        // Module with memory and a data segment that initializes bytes
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![
                    0x41, 0x00, // i32.const 0
                    0x28, 0x02, 0x00, // i32.load align=2 offset=0
                    0x0B,
                ],
            }],
            memories: vec![MemoryType {
                limits: Limits { min: 1, max: None },
                shared: false,
                is64: false,
            }],
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B], // i32.const 0; end
                data: vec![0x2A, 0x00, 0x00, 0x00],  // 42 in little-endian
                is_passive: false,
            }],
            exports: vec![Export {
                name: "read".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            ..Default::default()
        };

        let runtime = WasmRuntime::new();
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).unwrap();
        let result = runtime.call(&mut instance, "read", &[]).unwrap();
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_wasi_stub_proc_exit() {
        let wasi = WasiStub::new(|_| {});
        let func = wasi
            .resolve_function("wasi_snapshot_preview1", "proc_exit")
            .unwrap();
        assert_eq!(func.func_type().params, vec![ValueType::I32]);
        assert!(func.func_type().results.is_empty());
        // Calling proc_exit should return an error (trap)
        let result = func.call(&[WasmValue::I32(0)], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasi_stub_enosys_function() {
        let wasi = WasiStub::new(|_| {});
        let func = wasi
            .resolve_function("wasi_snapshot_preview1", "unknown_function")
            .unwrap();
        let result = func.call(&[], None).unwrap();
        assert_eq!(result, vec![WasmValue::I32(WASI_ENOSYS)]);
    }

    #[test]
    fn test_wasi_stub_wrong_module() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi.resolve_function("env", "some_func").is_none());
    }

    #[test]
    fn test_wasi_stub_resolve_global() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi.resolve_global("wasi_snapshot_preview1", "x").is_none());
    }

    #[test]
    fn test_wasi_stub_resolve_memory() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi
            .resolve_memory("wasi_snapshot_preview1", "memory")
            .is_none());
    }

    #[test]
    fn test_wasi_stub_resolve_table() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi
            .resolve_table("wasi_snapshot_preview1", "table")
            .is_none());
    }

    #[test]
    fn test_runtime_with_host() {
        let wasi = WasiStub::new(|_| {});
        let runtime = WasmRuntime::with_host(Box::new(wasi));
        let wasm = build_square_wasm();
        let result = runtime.load_and_run(&wasm, "square", &[4]);
        assert_eq!(result.unwrap(), vec![16]);
    }

    #[test]
    fn test_runtime_calls_imported_host_function() {
        let runtime = WasmRuntime::with_host(Box::new(TestHost));
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            }],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "double".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![
                    0x20, 0x00, // local.get 0
                    0x10, 0x00, // call imported function 0
                    0x0B, // end
                ],
            }],
            exports: vec![Export {
                name: "call_double".to_string(),
                kind: ExternalKind::Function,
                index: 1,
            }],
            ..Default::default()
        };

        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).unwrap();
        let result = runtime.call(&mut instance, "call_double", &[5]).unwrap();
        assert_eq!(result, vec![10]);
    }

    // ══════════════════════════════════════════════════════════════════════
    // W-next: real catch-clause matching -- tag import resolution +
    // combined tag index space
    // ══════════════════════════════════════════════════════════════════════

    /// Resolves a tag matching whatever `WasmModule` the test below
    /// declares itself importing (mirrors `TestHost`'s own
    /// `resolve_function`, but for `HostInterface::resolve_tag`).
    /// `identity` (W23) is the fixed canonical identity this host reports
    /// for the exported tag -- a real `wasm-runtime` embedder like
    /// `wasm-conformance`'s `RegistryHost` would read this from the
    /// exporting `WasmInstance::tag_identities`, but a hand-built test
    /// host can just supply a fixed value directly.
    struct TagTestHost {
        tag_type: FuncType,
        identity: u64,
    }

    impl HostInterface for TagTestHost {
        fn resolve_function(&self, _module_name: &str, _name: &str) -> Option<Box<dyn HostFunction>> {
            None
        }
        fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
            None
        }
        fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
            None
        }
        fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
            None
        }
        fn resolve_tag(&self, module_name: &str, name: &str) -> Option<(FuncType, u64)> {
            if module_name == "test" && name == "e0" {
                Some((self.tag_type.clone(), self.identity))
            } else {
                None
            }
        }
    }

    #[test]
    fn instantiate_builds_the_combined_tag_index_space_imports_first_then_declared() {
        // Regression test (W-next): a module importing ONE tag, then
        // declaring TWO more of its own with a DIFFERENT param shape, must
        // end up with `WasmInstance::tags` holding the FULL combined
        // index space (imports first, then declared) -- `module.tags`
        // ALONE (like `module.functions`) holds only the two
        // module-DEFINED entries, which is exactly the field a real bug
        // read directly before this test existed: any lookup by the
        // COMBINED index (what `throw`/`catch` actually encode) landed on
        // the wrong slot for every module with at least one tag import.
        let empty_type = FuncType { params: vec![], results: vec![] };
        let i32_type = FuncType { params: vec![ValueType::I32], results: vec![] };
        let runtime = WasmRuntime::with_host(Box::new(TagTestHost { tag_type: empty_type.clone(), identity: 999 }));
        let module = WasmModule {
            types: vec![empty_type.clone(), i32_type.clone()],
            imports: vec![Import {
                module_name: "test".to_string(),
                name: "e0".to_string(),
                kind: ExternalKind::Tag,
                type_info: ImportTypeInfo::Tag(0), // type 0 = empty_type
            }],
            // Two module-DEFINED tags: index 0 -> i32_type, index 1 -> empty_type
            // (deliberately NOT both the same type, so a wrong index
            // produces an observably wrong `tags` entry, not a
            // coincidentally-correct one).
            tags: vec![1, 0],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        // Combined space: [imported tag (type 0), local tag 0 (type 1),
        // local tag 1 (type 0)] -- imports first, then declared, exactly
        // matching `wasm-validator`'s own `tag_types` construction.
        assert_eq!(instance.tags, vec![0, 1, 0]);
        // Combined IDENTITY space (W23): the import adopts the host's
        // reported identity verbatim; the two LOCAL tags each get their
        // own freshly minted, non-zero, MUTUALLY DIFFERENT identity.
        assert_eq!(instance.tag_identities.len(), 3);
        assert_eq!(instance.tag_identities[0], 999, "an imported tag must adopt the exporter's own identity verbatim");
        assert_ne!(instance.tag_identities[1], 0, "a module-defined tag must get a real, non-zero identity");
        assert_ne!(instance.tag_identities[2], 0, "a module-defined tag must get a real, non-zero identity");
        assert_ne!(
            instance.tag_identities[1], instance.tag_identities[2],
            "two DIFFERENT module-defined tags must never share an identity"
        );
    }

    #[test]
    fn instantiate_mints_a_fresh_identity_per_instantiate_call_never_reused() {
        // Regression test (W23): two SEPARATE `instantiate()` calls on the
        // SAME module must NOT produce the same tag identity -- otherwise
        // an exception thrown by one completely unrelated instance could
        // be wrongly caught by another instance's `try_table`, just
        // because they happened to instantiate the same module.
        let empty_type = FuncType { params: vec![], results: vec![] };
        let runtime = WasmRuntime::new();
        let module = WasmModule {
            types: vec![empty_type],
            tags: vec![0],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance_a = runtime.instantiate(&validated).unwrap();
        let instance_b = runtime.instantiate(&validated).unwrap();
        assert_ne!(instance_a.tag_identities[0], 0);
        assert_ne!(instance_b.tag_identities[0], 0);
        assert_ne!(
            instance_a.tag_identities[0], instance_b.tag_identities[0],
            "two separate instantiations of the same module must never share a tag identity"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // W35 third slice: func_identities / LocalFunctionRef / InstanceSelfResolver
    // ══════════════════════════════════════════════════════════════════════

    /// A `HostFunction` double reporting a fixed, non-zero `identity()` --
    /// mirrors `TagTestHost`'s own "hand-supply a fixed identity a real
    /// embedder would read from the exporter" shape, for functions instead
    /// of tags.
    struct FuncIdentityTestHostFunction {
        func_type: FuncType,
        identity: u64,
    }

    impl HostFunction for FuncIdentityTestHostFunction {
        fn func_type(&self) -> &FuncType {
            &self.func_type
        }
        fn identity(&self) -> u64 {
            self.identity
        }
        fn call(&self, _args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
            Ok(vec![])
        }
    }

    struct FuncIdentityTestHost {
        func_type: FuncType,
        identity: u64,
    }

    impl HostInterface for FuncIdentityTestHost {
        fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>> {
            if module_name == "env" && name == "imported" {
                Some(Box::new(FuncIdentityTestHostFunction { func_type: self.func_type.clone(), identity: self.identity }))
            } else {
                None
            }
        }
        fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
            None
        }
        fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
            None
        }
        fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
            None
        }
    }

    #[test]
    fn instantiate_builds_func_identities_mirroring_tag_identities_imported_adopts_verbatim_module_defined_mints_fresh() {
        // Same shape and same assertions as `instantiate_builds_the_
        // combined_tag_index_space_imports_first_then_declared` (W23) --
        // W35's own design §2 explicitly calls for `func_identities` to
        // mirror `tag_identities`'s construction loop exactly.
        let empty_type = FuncType { params: vec![], results: vec![] };
        let runtime = WasmRuntime::with_host(Box::new(FuncIdentityTestHost { func_type: empty_type.clone(), identity: 777 }));
        let module = WasmModule {
            types: vec![empty_type.clone()],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "imported".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            // Two module-DEFINED functions, completing the combined
            // func-index space [imported, local0, local1].
            functions: vec![0, 0],
            code: vec![
                FunctionBody { locals: vec![], code: vec![0x0B] },
                FunctionBody { locals: vec![], code: vec![0x0B] },
            ],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert_eq!(instance.func_identities.len(), 3);
        assert_eq!(instance.func_identities[0], 777, "an imported function must adopt the exporter's own identity verbatim");
        assert_ne!(instance.func_identities[1], 0, "a module-defined function must get a real, non-zero identity");
        assert_ne!(instance.func_identities[2], 0, "a module-defined function must get a real, non-zero identity");
        assert_ne!(
            instance.func_identities[1], instance.func_identities[2],
            "two DIFFERENT module-defined functions must never share an identity"
        );
    }

    #[test]
    fn instantiate_mints_a_fresh_instance_identity_per_call_never_reused() {
        // Mirrors `instantiate_mints_a_fresh_identity_per_instantiate_call_
        // never_reused` (W23, for tags) -- W35's own `instance_identity`
        // (a further deviation from the spec's literal text, see that
        // field's own doc comment) needs the identical "never reused
        // across separate instantiations" property, since it's what
        // `effective_local_index` uses to decide dispatch safety.
        let runtime = WasmRuntime::new();
        let module = WasmModule::default();
        let validated = runtime.validate(&module).unwrap();
        let instance_a = runtime.instantiate(&validated).unwrap();
        let instance_b = runtime.instantiate(&validated).unwrap();
        assert_ne!(instance_a.instance_identity, 0);
        assert_ne!(instance_b.instance_identity, 0);
        assert_ne!(
            instance_a.instance_identity, instance_b.instance_identity,
            "two separate instantiations of the same module must never share an instance identity"
        );
    }

    #[test]
    fn local_function_ref_dispatches_to_the_right_function_body_via_a_raw_index_unrelated_to_any_export() {
        // W35 third slice, verification plan item (b): `LocalFunctionRef`
        // must work for a NON-exported function, called by raw combined
        // index -- not just an exported one `call_typed` could already
        // reach. Two module-defined functions: index 0 is EXPORTED
        // ("double", x*2); index 1 is NOT exported at all ("helper", x*10).
        // Resolving/calling index 1 directly (never going through any
        // export lookup) must run `helper`'s own body, not `double`'s --
        // a real, direct proof of dispatch, not merely "some function
        // ran."
        let runtime = WasmRuntime::new();
        let double_type = FuncType { params: vec![ValueType::I32], results: vec![ValueType::I32] };
        let module = wasm_wast_parser::parse_module(
            r#"(module
                 (func (export "double") (param i32) (result i32) (i32.mul (local.get 0) (i32.const 2)))
                 (func $helper (param i32) (result i32) (i32.mul (local.get 0) (i32.const 10))))"#,
        )
        .expect("module should parse");
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert_eq!(instance.func_types[1].params, double_type.params, "sanity: index 1 is $helper, same param shape as double");
        assert!(
            instance.exports.iter().all(|(name, _, idx)| !(name == "helper" || *idx == 1)),
            "sanity: index 1 must genuinely be unexported"
        );

        let instance_rc = Rc::new(RefCell::new(instance));
        let target = resolve_func_ref_for_instance(&instance_rc, 1).expect("resolving a local, unexported function must succeed");
        // `identity`/`owner_instance_identity` sanity: a LOCAL function's
        // target carries this instance's own real identity/ownership, not
        // the reserved `0`/`None` an import would.
        assert_ne!(target.identity, 0);
        assert_eq!(target.owner_instance_identity, Some(instance_rc.borrow().instance_identity));
        let results = target.callable.call(&[WasmValue::I32(7)], None).expect("dispatch through LocalFunctionRef should succeed");
        assert_eq!(results, vec![WasmValue::I32(70)], "must run $helper's OWN body (x*10), not double's (x*2)");
    }

    #[test]
    fn resolve_func_ref_for_instance_of_an_imported_function_reuses_its_existing_identity_and_tags_the_resolving_instance_as_owner() {
        // The import-branch counterpart to the local-function test above --
        // mirrors `wasm_execution::WasmExecutionContext::resolve_function_
        // ref`'s own import branch exactly (see that method's own already-
        // shipped unit test, `ref_func_of_an_imported_function_reuses_its_
        // existing_identity_and_callable`, for the `wasm-execution`-layer
        // half of this same contract).
        //
        // W35 fourth slice: `owner_instance_identity` is `Some(this
        // instance's own identity)`, NOT `None` -- a real, corpus-
        // verification-found correction to slice 3's own original claim
        // that an import's `local_index` is "dispatchable via local_index
        // in ANY ctx that holds it." That claim is false the moment this
        // target can be written into a table/global SHARED with a
        // genuinely different instance (this slice's own fixup pass makes
        // exactly that possible): `func_index` here (`0`) is only
        // meaningful as `THIS instance's` own `host_functions[0]` -- a
        // DIFFERENT instance reading this same target from a shared table
        // must fall through to `target.callable.call(..)` instead (see
        // `wasm_execution::FuncRefTarget::owner_instance_identity`'s own
        // doc comment for the full, reproduced `linking.wast` trace this
        // fixes).
        let func_type = FuncType { params: vec![], results: vec![] };
        let runtime = WasmRuntime::with_host(Box::new(FuncIdentityTestHost { func_type: func_type.clone(), identity: 555 }));
        let module = WasmModule {
            types: vec![func_type],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "imported".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        let instance_rc = Rc::new(RefCell::new(instance));
        let target = resolve_func_ref_for_instance(&instance_rc, 0).expect("resolving an import must succeed");
        assert_eq!(target.identity, 555, "an imported function's target must adopt the exporter's identity verbatim");
        assert_eq!(
            target.owner_instance_identity,
            Some(instance_rc.borrow().instance_identity),
            "an import's local_index is only meaningful for the instance that resolved it -- a different instance \
             reading this target from a shared table/global must dispatch via `callable.call(..)` instead"
        );
    }

    #[test]
    fn instance_self_resolver_installed_on_a_hand_built_engine_dispatches_a_local_function_via_ref_func_and_call_ref() {
        // W35 third slice, design §4: proves `InstanceSelfResolver`/
        // `wasm_execution::WasmExecutionEngine::set_self_resolver` work
        // end-to-end -- NOT exercised by `build_engine` in this slice (see
        // that method's own doc comment for why), but real, tested
        // infrastructure ready for slice 4. A LONG-LIVED, test-owned
        // `Rc<RefCell<WasmInstance>>` (never unwrapped -- this test never
        // calls `instantiate()`'s own `try_unwrap`-based construction, so
        // the self-referential-cycle problem that blocks wiring this into
        // `instantiate()` itself simply doesn't apply here) backs the
        // resolver for the engine's whole lifetime, exactly the shape a
        // real long-lived embedder (`wasm-conformance`'s `ModuleRegistry`)
        // would provide.
        let runtime = WasmRuntime::new();
        let module = wasm_wast_parser::parse_module(
            r#"(module
                 (type $ii (func (param i32) (result i32)))
                 (func $helper (param i32) (result i32) (i32.mul (local.get 0) (i32.const 10)))
                 (func (export "run") (param i32) (result i32)
                   (call_ref $ii (local.get 0) (ref.func $helper))))"#,
        )
        .expect("module should parse");
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        let instance_rc = Rc::new(RefCell::new(instance));

        // Build the engine by hand (mirroring `build_engine`'s own
        // plumbing) so `set_self_resolver` can be called -- `build_engine`
        // itself deliberately never does this in this slice.
        let (memories, tables, globals, global_types, func_types, func_bodies, host_functions, func_identities, instance_identity) = {
            let instance_ref = instance_rc.borrow();
            (
                instance_ref.memories.clone(),
                instance_ref.tables.clone(),
                instance_ref.globals.clone(),
                instance_ref.global_types.clone(),
                instance_ref.func_types.clone(),
                instance_ref.func_bodies.clone(),
                instance_ref.host_functions.clone(),
                instance_ref.func_identities.clone(),
                instance_ref.instance_identity,
            )
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig { memories, tables, globals, global_types, func_types, func_bodies, host_functions });
        engine.set_func_identities(func_identities);
        engine.set_instance_identity(instance_identity);
        engine.set_self_resolver(Box::new(InstanceSelfResolver { instance: instance_rc.clone() }));

        let run_index = instance_rc
            .borrow()
            .exports
            .iter()
            .find(|(name, _, _)| name == "run")
            .map(|(_, _, idx)| *idx as usize)
            .unwrap();
        let results = engine.call_function(run_index, &[WasmValue::I32(4)]).expect("call should succeed");
        assert_eq!(results, vec![WasmValue::I32(40)], "ref.func $helper + call_ref must dispatch through the real self-resolver, running $helper's own body");
    }

    #[test]
    fn instantiate_never_panics_on_a_module_whose_own_active_elem_segment_references_its_own_local_functions() {
        // Regression test (W35 third slice): this is EXACTLY the shape
        // that broke this slice's own first attempt at the spec's literal
        // "Rc::new(RefCell::new(..)) + resolve + Rc::try_unwrap" two-phase
        // construction -- an active elem segment referencing the SAME
        // module's OWN local functions (the common, expected case, and
        // the literal shape of `linking.wast`'s own `$Mt`/`$g` motivating
        // example) forces an unavoidable self-referential `Rc` cycle if
        // `instantiate()` tries to eagerly resolve it into a real
        // `LocalFunctionRef` internally -- `Rc::try_unwrap` then fails
        // UNCONDITIONALLY, not intermittently. `instantiate()`'s own doc
        // comment explains why this slice's own design deliberately does
        // NOT attempt that eager resolution; this test is the concrete,
        // reproducible proof that a real module exercising exactly that
        // shape instantiates cleanly, with no panic, under this slice's
        // actual (corrected) design.
        let runtime = WasmRuntime::new();
        let module = wasm_wast_parser::parse_module(
            r#"(module
                 (table $t 4 funcref)
                 (func $one (result i32) (i32.const 111))
                 (func $two (result i32) (i32.const 222))
                 (elem (i32.const 0) $one $two)
                 (func (export "call0") (result i32)
                   (call_indirect (type $i) (i32.const 0)))
                 (type $i (func (result i32))))"#,
        )
        .expect("module should parse");
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).expect("instantiate() must succeed, not panic, for a self-referencing active elem segment");
        let results = runtime.call_typed(&mut instance, "call0", &[]).expect("call_indirect through the elem-populated table must succeed");
        assert_eq!(results, vec![WasmValue::I32(111)], "must dispatch to $one, the function the elem segment actually wrote at slot 0");
    }

    /// Regression test for the exact bug `elem.wast`'s own "Implicitly
    /// dropped elements" corpus section and `table_init.wast`'s directive
    /// at byte offset 21455 caught (see `wasm-conformance`'s CHANGELOG
    /// 0.1.117 entry for the full corpus writeup): real spec text says
    /// "after an active or declarative element segment is initialized, it
    /// is dropped" -- so a `table.init` naming an ACTIVE segment, even one
    /// this exact module itself just applied during its own instantiation,
    /// must trap "out of bounds table access", never silently succeed.
    /// Before this fix, `instantiate()` never flipped `dropped_elements`
    /// for an active segment (only `elem.drop`/a consuming `table.init`
    /// on a PASSIVE segment ever did), so this exact shape wrongly
    /// succeeded.
    #[test]
    fn instantiate_marks_an_active_elem_segment_dropped_so_a_later_table_init_on_it_traps() {
        let runtime = WasmRuntime::new();
        let module = wasm_wast_parser::parse_module(
            r#"(module
                 (table 10 funcref)
                 (elem $e (i32.const 0) func $f)
                 (func $f)
                 (func (export "init")
                   (table.init $e (i32.const 0) (i32.const 0) (i32.const 1))))"#,
        )
        .expect("module should parse");
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).expect("instantiate() must succeed -- applying the active segment itself never traps");
        let err = runtime
            .call_typed(&mut instance, "init", &[])
            .expect_err("table.init against an already-dropped (via instantiation) active segment must trap");
        assert!(
            err.message.contains("out of bounds table access"),
            "trap message should name the real spec's own out-of-bounds-table-access rule, got: {}",
            err.message
        );
    }

    /// Companion to the active-segment regression above: a DECLARATIVE
    /// segment (`(elem $e declare func $f)`, reference-types proposal --
    /// this repo's `wasm-wast-parser` folds it into `is_passive: true`
    /// plus the new `is_declarative: true` flag, see `wasm_types::
    /// Element::is_declarative`'s own doc comment) is never applied to any
    /// table at all, but per spec must ALSO be treated as already dropped
    /// from the moment instantiation finishes -- it was never live to
    /// begin with. `elem.wast`'s own byte offset 20815 is exactly this
    /// shape. Before this fix, a declarative segment was indistinguishable
    /// from a genuinely passive one at runtime (both `is_passive: true`,
    /// `dropped_elements` initially `false`), so `table.init` against it
    /// wrongly succeeded.
    #[test]
    fn instantiate_marks_a_declarative_elem_segment_dropped_so_table_init_on_it_traps() {
        let runtime = WasmRuntime::new();
        let module = wasm_wast_parser::parse_module(
            r#"(module
                 (table 10 funcref)
                 (elem $e declare func $f)
                 (func $f)
                 (func (export "init")
                   (table.init $e (i32.const 0) (i32.const 0) (i32.const 1))))"#,
        )
        .expect("module should parse");
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime
            .instantiate(&validated)
            .expect("instantiate() must succeed -- a declarative segment copies no content, so there is nothing to trap on during instantiation itself");
        let err = runtime
            .call_typed(&mut instance, "init", &[])
            .expect_err("table.init against a declarative segment must trap: it is treated as already dropped, never live");
        assert!(
            err.message.contains("out of bounds table access"),
            "trap message should name the real spec's own out-of-bounds-table-access rule, got: {}",
            err.message
        );
    }

    /// Confirms this fix (marking active/declarative segments dropped at
    /// instantiation) does NOT accidentally touch a genuinely PASSIVE
    /// segment's own, completely separate, pre-existing dropped-tracking:
    /// a passive segment (`is_passive: true`, `is_declarative: false`)
    /// must stay live and `table.init`-able immediately after
    /// instantiation (unlike its active/declarative cousins above), a
    /// first `table.init` against it must succeed, and only an EXPLICIT
    /// `elem.drop` (never `table.init` itself, which does not auto-drop a
    /// passive segment -- see `wasm-execution`'s own `0x0C` opcode handler
    /// doc comment) may later make a SECOND `table.init` against the same
    /// segment trap. This is the exact interaction
    /// `table_init_after_elem_drop_traps_on_nonzero_length_but_succeeds_
    /// at_zero_length` already covers at the raw-opcode (`wasm-execution`)
    /// level; this test covers the same interaction through the real
    /// `instantiate()` -> `call_typed()` path this fix actually touches,
    /// so a bug that conflated "passive already-dropped" tracking with
    /// "active/declarative newly-dropped" tracking would be caught here
    /// even if it slipped past the opcode-level test.
    #[test]
    fn instantiate_leaves_a_genuinely_passive_elem_segment_undropped_first_table_init_succeeds_elem_drop_then_traps_the_next() {
        let runtime = WasmRuntime::new();
        let module = wasm_wast_parser::parse_module(
            r#"(module
                 (table 10 funcref)
                 (elem $e func $f)
                 (func $f)
                 (func (export "init")
                   (table.init $e (i32.const 0) (i32.const 0) (i32.const 1)))
                 (func (export "drop")
                   (elem.drop $e)))"#,
        )
        .expect("module should parse");
        let validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&validated).expect("instantiate() must succeed for a module with only a passive elem segment");

        runtime
            .call_typed(&mut instance, "init", &[])
            .expect("a genuinely passive segment must stay live immediately after instantiation -- this fix must not mark it dropped");

        runtime.call_typed(&mut instance, "drop", &[]).expect("elem.drop itself never traps");

        let err = runtime
            .call_typed(&mut instance, "init", &[])
            .expect_err("table.init against a passive segment explicitly elem.drop'd must trap, same as before this fix");
        assert!(
            err.message.contains("out of bounds table access"),
            "trap message should name the real spec's own out-of-bounds-table-access rule, got: {}",
            err.message
        );
    }

    #[test]
    fn instantiate_rejects_a_tag_import_with_an_incompatible_type() {
        let wrong_type = FuncType { params: vec![ValueType::I32], results: vec![] };
        let empty_type = FuncType { params: vec![], results: vec![] };
        // Host actually exports an EMPTY-param tag; the importing module
        // expects one with an i32 param -- must be rejected as a link
        // failure, not silently accepted.
        let runtime = WasmRuntime::with_host(Box::new(TagTestHost { tag_type: empty_type, identity: 1 }));
        let module = WasmModule {
            types: vec![wrong_type],
            imports: vec![Import {
                module_name: "test".to_string(),
                name: "e0".to_string(),
                kind: ExternalKind::Tag,
                type_info: ImportTypeInfo::Tag(0),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        assert!(runtime.instantiate(&validated).is_err(), "an incompatible tag import must fail to link");
    }

    // ══════════════════════════════════════════════════════════════════════
    // WASM05/W10: real link-failure path
    // ══════════════════════════════════════════════════════════════════════

    /// Resolves a memory, table, and global for real (unlike `TestHost`,
    /// whose `resolve_memory`/`resolve_table`/`resolve_global` always
    /// return `None`) -- lets these tests exercise the type-compatibility
    /// checks, not just "unresolved".
    struct LinkingTestHost;

    impl HostInterface for LinkingTestHost {
        fn resolve_function(&self, _module_name: &str, _name: &str) -> Option<Box<dyn HostFunction>> {
            None
        }

        fn resolve_global(&self, module_name: &str, name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
            if module_name == "env" && name == "g" {
                Some((
                    GlobalType { value_type: ValueType::I32, mutable: false },
                    Rc::new(RefCell::new(GlobalStorage { value: WasmValue::I32(42), func_ref: None })),
                ))
            } else {
                None
            }
        }

        fn resolve_memory(&self, module_name: &str, name: &str) -> Option<LinearMemory> {
            if module_name == "env" && name == "mem" {
                Some(LinearMemory::new(1, Some(2)))
            } else {
                None
            }
        }

        fn resolve_table(&self, module_name: &str, name: &str) -> Option<Table> {
            if module_name == "env" && name == "tab" {
                Some(Table::new(1, Some(2)))
            } else if module_name == "env" && name == "tab64" {
                // W26 (table64 proposal): a real `is64` table, for the
                // is64-mismatch import-linking tests below -- mirrors
                // `resolve_table`'s own plain 32-bit "tab" entry.
                Some(Table::new_with_is64(1, Some(2), true).unwrap())
            } else if module_name == "env" && name == "tab_externref" {
                // W37 addendum: a real EXTERNREF table, for the
                // element-type-mismatch import-linking tests below --
                // mirrors `resolve_table`'s own plain funcref "tab" entry,
                // but with `with_element_type` overriding the `Table::new`
                // default (`wasm_types::FUNCREF`) to `wasm_types::
                // EXTERNREF`, exactly like `wasm-runtime::instantiate()`'s
                // own declared-table construction loop now does for a
                // real module.
                Some(Table::new(1, Some(2)).with_element_type(wasm_types::EXTERNREF))
            } else {
                None
            }
        }
    }

    fn module_importing_function(module_name: &str, name: &str, ft: FuncType) -> WasmModule {
        WasmModule {
            types: vec![ft],
            imports: vec![Import {
                module_name: module_name.to_string(),
                name: name.to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_instantiate_fails_when_a_function_import_is_unresolved() {
        let runtime = WasmRuntime::with_host(Box::new(TestHost));
        let module = module_importing_function(
            "env",
            "no_such_function",
            FuncType { params: vec![ValueType::I32], results: vec![ValueType::I32] },
        );
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("unknown import"), "{err}");
    }

    #[test]
    fn test_instantiate_fails_when_a_function_import_type_mismatches() {
        let runtime = WasmRuntime::with_host(Box::new(TestHost));
        // The real "env"."double" host function is (i32) -> i32; declare
        // it here as (i64) -> i32 instead.
        let module = module_importing_function(
            "env",
            "double",
            FuncType { params: vec![ValueType::I64], results: vec![ValueType::I32] },
        );
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    /// Task #100 (security review follow-up to task #96): `instantiate()`
    /// used to take a bare `&WasmModule`, so a caller who skipped
    /// `validate()` could reach `Table::new`'s eager allocation with an
    /// unvalidated, over-cap table size -- silently bypassing every
    /// `validate()` check, including the memory/table allocation caps.
    /// Now `instantiate()` requires a `&ValidatedModule`, so this is a
    /// compile-time fact, not a caller convention: the only way to get a
    /// `ValidatedModule` for this module is through `validate()`, and
    /// `validate()` itself rejects it before `instantiate()` ever runs.
    ///
    /// The example module here used to be a 32-bit table declaring more
    /// than `MAX_TABLE_ELEMENTS` -- that was a real `validate()` rejection
    /// before gap 2 of the W-next `elem.wast`/`table.wast` investigation
    /// pass (`code/specs/W07-wasm-post-mvp-epics.md`'s addendum) moved
    /// that specific check to instantiation time (see `wasm-validator`'s
    /// own updated Check 2b doc comment and `test_instantiate_traps_
    /// gracefully_for_a_32bit_table_past_the_practical_cap` below for that
    /// case's own new coverage), so it no longer demonstrates THIS test's
    /// actual point. Swapped for a table whose `min` exceeds its own
    /// `max` -- a real, spec-mandated structural rule (Check 1c) this fix
    /// never touched -- which still genuinely fails validation and so
    /// still demonstrates the same "no `ValidatedModule` exists, so
    /// `instantiate()` is unreachable" guarantee.
    #[test]
    fn instantiate_is_unreachable_for_a_module_that_fails_validation() {
        let runtime = WasmRuntime::new();
        let module = WasmModule {
            tables: vec![TableType { element_type: 0x70, limits: Limits { min: 5, max: Some(1) }, is64: false }],
            ..Default::default()
        };
        let err = runtime.validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
        // There is no `ValidatedModule` to hand `instantiate()` here --
        // that absence, not a runtime check, is what protects it.
    }

    #[test]
    fn test_instantiate_succeeds_when_no_host_is_present_and_the_module_has_no_imports() {
        // A host-less runtime instantiating an import-free module must
        // still work -- the link-failure path only fires when there's an
        // actual import to resolve.
        let runtime = WasmRuntime::new();
        let module = WasmModule::default();
        let validated = runtime.validate(&module).unwrap();
        assert!(runtime.instantiate(&validated).is_ok());
    }

    #[test]
    fn test_instantiate_fails_when_a_memory_import_is_unresolved() {
        let runtime = WasmRuntime::with_host(Box::new(TestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "no_such_memory".to_string(),
                kind: ExternalKind::Memory,
                type_info: ImportTypeInfo::Memory(MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("unknown import"), "{err}");
    }

    #[test]
    fn test_instantiate_fails_when_a_memory_import_limits_are_incompatible() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        // The real "env"."mem" host memory is 1 page min, max 2 -- declare
        // a min of 5, which the actual memory doesn't satisfy.
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "mem".to_string(),
                kind: ExternalKind::Memory,
                type_info: ImportTypeInfo::Memory(MemoryType { limits: Limits { min: 5, max: None }, shared: false, is64: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn test_instantiate_succeeds_when_a_memory_import_is_compatible() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "mem".to_string(),
                kind: ExternalKind::Memory,
                type_info: ImportTypeInfo::Memory(MemoryType { limits: Limits { min: 1, max: Some(2) }, shared: false, is64: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert!(!instance.memories.is_empty());
    }

    #[test]
    fn test_instantiate_fails_when_a_table_import_is_unresolved() {
        let runtime = WasmRuntime::with_host(Box::new(TestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "no_such_table".to_string(),
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("unknown import"), "{err}");
    }

    #[test]
    fn test_instantiate_fails_when_a_table_import_limits_are_incompatible() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab".to_string(),
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType { element_type: 0x70, limits: Limits { min: 5, max: None }, is64: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn test_instantiate_succeeds_when_a_table_import_is_compatible() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab".to_string(),
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType { element_type: 0x70, limits: Limits { min: 1, max: Some(2) }, is64: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert_eq!(instance.tables.len(), 1);
    }

    // ── W37 addendum: element-type import-linking compatibility ──────────
    //
    // Regression coverage for the bug this addendum fixes: `linking.wast`'s
    // own `t-funcnull`/`t-refnull` cases (both funcref-family, exported by
    // `$Mtable_ex`) were importable as a declared `externref` table without
    // error before this fix -- `instantiate()`'s table-import arm checked
    // `is64` and `limits` but never the element-type tag itself. See this
    // arm's own doc comment (`ImportTypeInfo::Table` match arm, above) for
    // the full rationale, including why a plain byte-equality check (not a
    // subtype check) is the spec-correct rule for tables specifically.

    #[test]
    fn test_instantiate_fails_when_a_table_import_element_type_mismatches_declared_externref_actual_funcref() {
        // Exactly the shape of `linking.wast`'s own real, previously-
        // undetected bug: the actual host table is funcref-family ("tab"),
        // but the importing module declares it as `externref`.
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab".to_string(), // actual host table is funcref
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType {
                    element_type: wasm_types::EXTERNREF, // declared as externref -- mismatch
                    limits: Limits { min: 1, max: Some(2) },
                    is64: false,
                }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn test_instantiate_fails_when_a_table_import_element_type_mismatches_declared_funcref_actual_externref() {
        // The mirror-image mismatch: the actual host table is externref,
        // but the importing module declares it as `funcref`.
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab_externref".to_string(), // actual host table is externref
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType {
                    element_type: wasm_types::FUNCREF, // declared as funcref -- mismatch
                    limits: Limits { min: 1, max: Some(2) },
                    is64: false,
                }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn test_instantiate_succeeds_when_a_table_import_element_type_matches_externref() {
        // A genuinely compatible externref import must still succeed --
        // this fix must not start rejecting a VALID import just because it
        // now actually checks the element type.
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab_externref".to_string(),
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType {
                    element_type: wasm_types::EXTERNREF,
                    limits: Limits { min: 1, max: Some(2) },
                    is64: false,
                }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert_eq!(instance.tables.len(), 1);
    }

    // ── table64 (W26): is64 import-linking compatibility ─────────────────

    #[test]
    fn test_instantiate_fails_when_a_table_import_is64_mismatched_declared_is64_actual_32bit() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab".to_string(), // actual host table is 32-bit
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType {
                    element_type: 0x70,
                    limits: Limits { min: 1, max: Some(2) },
                    is64: true, // declared as 64-bit -- mismatch
                }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn test_instantiate_fails_when_a_table_import_is64_mismatched_declared_32bit_actual_is64() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab64".to_string(), // actual host table is 64-bit
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType {
                    element_type: 0x70,
                    limits: Limits { min: 1, max: Some(2) },
                    is64: false, // declared as 32-bit -- mismatch
                }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn test_instantiate_succeeds_when_a_table64_import_is_compatible() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tab64".to_string(),
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType { element_type: 0x70, limits: Limits { min: 1, max: Some(2) }, is64: true }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert_eq!(instance.tables.len(), 1);
        assert!(instance.tables[0].is64());
    }

    #[test]
    fn test_instantiate_builds_an_is64_declared_table_at_a_real_in_range_size() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            tables: vec![TableType { element_type: 0x70, limits: Limits { min: 3, max: Some(5) }, is64: true }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert_eq!(instance.tables.len(), 1);
        assert!(instance.tables[0].is64());
        assert_eq!(instance.tables[0].size(), 3);
    }

    /// The genuinely new DoS consideration `is64` introduces for tables
    /// (mirrors `W25`'s memory64 practical-cap rationale exactly): a
    /// spec-valid `is64` table declaration whose `min` this interpreter
    /// will not actually allocate must TRAP gracefully at instantiation,
    /// never panic/abort the process.
    #[test]
    fn test_instantiate_traps_gracefully_for_an_is64_table_past_the_practical_cap() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            tables: vec![TableType { element_type: 0x70, limits: Limits { min: u64::MAX, max: None }, is64: true }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        // A single is64 table this far over the cap trips the NEW
        // aggregate check (below) before `Table::new_with_is64`'s own
        // per-table check ever runs -- either way, a graceful `TrapError`,
        // never a panic/allocator abort.
        assert!(err.to_string().contains("practical aggregate cap"), "{err}");
    }

    /// `total_table_elements` (this crate's own aggregate, covering every
    /// declared table regardless of `is64` -- see that variable's own doc
    /// comment) must reject two tables each individually AT the per-table
    /// `MAX_TABLE_ELEMENTS` cap, even though neither is rejected by
    /// `Table::new_with_is64` alone. Originally written for `is64` tables
    /// specifically (back when `wasm-validator`'s own, now-removed, Check
    /// 2b covered 32-bit tables' aggregate instead); kept as an `is64`
    /// case for coverage continuity, alongside the 32-bit equivalent
    /// immediately below (gap 2 of the W-next `elem.wast`/`table.wast`
    /// investigation pass).
    #[test]
    fn test_instantiate_traps_when_is64_tables_combined_exceed_the_aggregate_cap() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            tables: vec![
                TableType {
                    element_type: 0x70,
                    limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64, max: None },
                    is64: true,
                },
                TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: true },
            ],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("practical aggregate cap"), "{err}");
    }

    /// Confirms the aggregate itself never wraps and reports the RIGHT
    /// reason: a table whose own `min` (`u64::MAX`) already exceeds
    /// `MAX_TABLE_ELEMENTS` is independently caught by
    /// `Table::new_with_is64`'s own per-table cap regardless of how the
    /// aggregate is summed, so a plain `+=` here wouldn't let this module
    /// wrongly instantiate -- but it WOULD wrap the running total (`+=`
    /// overflow on `u64::MAX` reads as a *release-mode* wraparound, not a
    /// panic) and misreport the failure as the per-table cap instead of
    /// the aggregate one, which is what this test's exact error-message
    /// assertion actually catches. `saturating_add` is kept anyway so this
    /// aggregate check is correct and self-sufficient on its own terms,
    /// not dependent on the per-table check happening to catch every case
    /// that could otherwise overflow it.
    #[test]
    fn test_instantiate_aggregate_cap_does_not_wrap_on_overflow() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            tables: vec![
                TableType {
                    element_type: 0x70,
                    limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64, max: None },
                    is64: true,
                },
                TableType { element_type: 0x70, limits: Limits { min: u64::MAX, max: None }, is64: true },
            ],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("practical aggregate cap"), "{err}");
    }

    /// Gap 2 of the W-next `elem.wast`/`table.wast` investigation pass
    /// (`code/specs/W07-wasm-post-mvp-epics.md`'s addendum): a PLAIN
    /// (`is64: false`) table declaring far more than `MAX_TABLE_ELEMENTS`
    /// -- `table.wast`'s own real corpus case, `(module definition (table
    /// 0xffff_ffff funcref))` -- must now validate (see
    /// `wasm-validator`'s own `accepts_a_32bit_table_declaring_far_more_
    /// than_max_table_elements` test) but still trap gracefully at
    /// INSTANTIATION, never panic/allocator-abort: the resource-limit
    /// heuristic didn't disappear, it just moved to the pipeline stage
    /// where real allocation actually happens.
    #[test]
    fn test_instantiate_traps_gracefully_for_a_32bit_table_past_the_practical_cap() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            tables: vec![TableType { element_type: 0x70, limits: Limits { min: u32::MAX as u64, max: None }, is64: false }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("practical aggregate cap"), "{err}");
    }

    /// The 32-bit counterpart to `test_instantiate_traps_when_is64_tables_
    /// combined_exceed_the_aggregate_cap` above -- confirms
    /// `total_table_elements` (generalized by gap 2's fix to cover EVERY
    /// table, not just `is64` ones) still closes the aggregate gap for
    /// plain 32-bit tables now that `wasm-validator`'s own 32-bit
    /// aggregate check (the old Check 2b) no longer exists: two 32-bit
    /// tables each individually AT the per-table cap must still be
    /// rejected in aggregate, at instantiation time.
    #[test]
    fn test_instantiate_traps_when_32bit_tables_combined_exceed_the_aggregate_cap() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            tables: vec![
                TableType {
                    element_type: 0x70,
                    limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64, max: None },
                    is64: false,
                },
                TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false },
            ],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("practical aggregate cap"), "{err}");
    }

    /// A mixed module (one `is64` table, one plain 32-bit table) whose
    /// COMBINED elements exceed the aggregate cap must also be rejected --
    /// confirms the two kinds now genuinely share ONE running total
    /// (gap 2's fix unified what used to be two separate aggregates,
    /// `wasm-validator`'s 32-bit-only Check 2b and this crate's
    /// `is64`-only one), not two independent budgets that could each stay
    /// under the cap while the module's real total allocation does not.
    #[test]
    fn test_instantiate_traps_when_mixed_is64_and_32bit_tables_combined_exceed_the_aggregate_cap() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            tables: vec![
                TableType {
                    element_type: 0x70,
                    limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64, max: None },
                    is64: true,
                },
                TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false },
            ],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("practical aggregate cap"), "{err}");
    }

    #[test]
    fn test_instantiate_fails_when_a_global_import_is_unresolved() {
        let runtime = WasmRuntime::with_host(Box::new(TestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "no_such_global".to_string(),
                kind: ExternalKind::Global,
                type_info: ImportTypeInfo::Global(GlobalType { value_type: ValueType::I32, mutable: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("unknown import"), "{err}");
    }

    #[test]
    fn test_instantiate_fails_when_a_global_import_type_mismatches() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        // The real "env"."g" host global is an immutable i32; declare it
        // here as mutable instead.
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "g".to_string(),
                kind: ExternalKind::Global,
                type_info: ImportTypeInfo::Global(GlobalType { value_type: ValueType::I32, mutable: true }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let err = runtime.instantiate(&validated).err().unwrap();
        assert!(err.to_string().contains("incompatible import type"), "{err}");
    }

    #[test]
    fn test_instantiate_succeeds_when_a_global_import_is_compatible() {
        let runtime = WasmRuntime::with_host(Box::new(LinkingTestHost));
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "g".to_string(),
                kind: ExternalKind::Global,
                type_info: ImportTypeInfo::Global(GlobalType { value_type: ValueType::I32, mutable: false }),
            }],
            ..Default::default()
        };
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();
        assert_eq!(instance.globals[0].borrow().value, WasmValue::I32(42));
    }

    #[test]
    fn test_instance_fields() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let module = runtime.load(&wasm).unwrap();
        let validated = runtime.validate(&module).unwrap();
        let instance = runtime.instantiate(&validated).unwrap();

        // No memory in square module
        assert!(instance.memories.is_empty());
        // No tables
        assert!(instance.tables.is_empty());
        // No globals
        assert!(instance.globals.is_empty());
        // One function type
        assert_eq!(instance.func_types.len(), 1);
        // One function body
        assert_eq!(instance.func_bodies.len(), 1);
    }

    #[test]
    fn test_runtime_load_and_run_nonexistent_export_error_message() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let err = runtime.load_and_run(&wasm, "no_such_fn", &[1]).unwrap_err();
        assert!(err.contains("not found"));
    }
}
