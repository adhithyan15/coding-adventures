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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use wasm_execution::{
    evaluate_const_expr, HostFunction, HostInterface, LinearMemory, Table, TrapError,
    WasmEngineConfig, WasmExecutionEngine, WasmValue,
};
use wasm_module_parser::WasmModuleParser;
use wasm_types::{
    ExternalKind, FuncType, FunctionBody, GlobalType, Import, ImportTypeInfo, Limits, ValueType,
    WasmModule,
};
use wasm_validator::{validate, ValidatedModule, ValidationError};

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

    fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, WasmValue)> {
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
/// shared `Arc<Mutex<LinearMemory>>` that is populated by the runtime
/// **before** the first WASM call. See `WasiEnv::attach_memory`.
pub struct WasiEnv {
    /// Command-line arguments.
    pub args: Vec<String>,

    /// Environment variables in "KEY=VALUE" format.
    pub env: Vec<String>,

    /// Shared handle to WASM linear memory. Populated via `attach_memory`
    /// after instantiation.
    pub memory: Arc<Mutex<Option<LinearMemory>>>,

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
            memory: Arc::new(Mutex::new(None)),
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
        *self.memory.lock().unwrap() = Some(mem);
    }

    /// Retrieve the memory after execution (so the caller can inspect it or
    /// put it back into the `WasmInstance`).
    pub fn take_memory(&self) -> Option<LinearMemory> {
        self.memory.lock().unwrap().take()
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
                memory: Arc::clone(&self.memory),
                stdout_callback: Arc::clone(&self.stdout_callback),
                stderr_callback: Arc::clone(&self.stderr_callback),
            })),
            "fd_read" => Some(Box::new(FdReadFunc {
                memory: Arc::clone(&self.memory),
                stdin_callback: Arc::clone(&self.stdin_callback),
            })),
            "proc_exit" => Some(Box::new(ProcExitFunc)),

            // ── Tier 3: arguments ─────────────────────────────────────────
            "args_sizes_get" => Some(Box::new(ArgsSizesGetFunc {
                args: self.args.clone(),
                memory: Arc::clone(&self.memory),
            })),
            "args_get" => Some(Box::new(ArgsGetFunc {
                args: self.args.clone(),
                memory: Arc::clone(&self.memory),
            })),

            // ── Tier 3: environment ───────────────────────────────────────
            "environ_sizes_get" => Some(Box::new(EnvironSizesGetFunc {
                env: self.env.clone(),
                memory: Arc::clone(&self.memory),
            })),
            "environ_get" => Some(Box::new(EnvironGetFunc {
                env: self.env.clone(),
                memory: Arc::clone(&self.memory),
            })),

            // ── Tier 3: clock ─────────────────────────────────────────────
            "clock_res_get" => Some(Box::new(ClockResGetFunc {
                clock: Arc::clone(&self.clock),
                memory: Arc::clone(&self.memory),
            })),
            "clock_time_get" => Some(Box::new(ClockTimeGetFunc {
                clock: Arc::clone(&self.clock),
                memory: Arc::clone(&self.memory),
            })),

            // ── Tier 3: random ────────────────────────────────────────────
            "random_get" => Some(Box::new(RandomGetFunc {
                random: Arc::clone(&self.random),
                memory: Arc::clone(&self.memory),
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

    fn resolve_global(&self, _: &str, _: &str) -> Option<(GlobalType, WasmValue)> {
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
    shared: &Arc<Mutex<Option<LinearMemory>>>,
    action: impl FnOnce(&mut LinearMemory) -> Result<T, TrapError>,
) -> Result<T, TrapError> {
    if let Some(memory) = provided {
        return action(memory);
    }

    let mut guard = shared.lock().unwrap();
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    memory: Arc<Mutex<Option<LinearMemory>>>,
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
    /// Global variable values.
    pub globals: Vec<WasmValue>,
    /// Global type descriptors.
    pub global_types: Vec<GlobalType>,
    /// All function type signatures.
    pub func_types: Vec<FuncType>,
    /// Function bodies (None for imports).
    pub func_bodies: Vec<Option<FunctionBody>>,
    /// Resolved imported host functions.
    pub host_functions: Vec<Option<Box<dyn HostFunction>>>,
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
static NEXT_TAG_IDENTITY: AtomicU64 = AtomicU64::new(1);

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
        let mut func_types: Vec<FuncType> = Vec::new();
        let mut func_bodies: Vec<Option<FunctionBody>> = Vec::new();
        let mut host_functions: Vec<Option<Box<dyn HostFunction>>> = Vec::new();
        let mut global_types: Vec<GlobalType> = Vec::new();
        let mut globals: Vec<WasmValue> = Vec::new();
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
                    if host_func.func_type() != &ft {
                        return Err(link_error("incompatible import type", imp));
                    }

                    func_types.push(ft);
                    func_bodies.push(None);
                    host_functions.push(Some(host_func));
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
                    // `Table` doesn't track its declared element type at
                    // runtime (WASM 1.0 only ever has funcref tables, and
                    // this repo's reference-types slice hasn't grown a
                    // runtime-typed Table yet either) -- only limits (and,
                    // as of W26, `is64`) are checked here. Every table this
                    // repo can currently construct is funcref, so this
                    // doesn't lose real coverage against the vendored
                    // corpus, but a table import mismatched purely on
                    // element type (not limits) would incorrectly link
                    // here rather than fail. Named, not silent: revisit if
                    // a future PR gives `Table` a real element-type field.
                    //
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
                    tags.push(*type_idx);
                    tag_identities.push(identity);
                }
            }
        }

        // Add module-defined functions.
        for (i, &type_idx) in module.functions.iter().enumerate() {
            func_types.push(module.types[type_idx as usize].clone());
            func_bodies.push(module.code.get(i).cloned());
            host_functions.push(None);
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
        // table's own real spec ceiling is `u64::MAX` (`wasm-validator`'s
        // Check 2b) -- far larger than this interpreter will actually
        // allocate. A plain `as u32` narrowing here would silently
        // TRUNCATE/wrap an out-of-practical-range `min` into an
        // arbitrary, wrong-sized table instead of failing loudly, for any
        // `is64` table whose declared `min` exceeds `u32::MAX` (newly
        // reachable now that `is64` tables can validly declare such a
        // `min`). `Table::new_with_is64` mirrors `LinearMemory::
        // new_with_is64` (W25) exactly: fallible, returning a real,
        // gracefully-propagated `TrapError` (never a panic/allocator
        // abort) if `is64 && min` exceeds `MAX_TABLE_ELEMENTS`, this
        // interpreter's own practical resource cap reused as the is64
        // instantiation-time bound (same "reuse the existing bound" move
        // W25 made with `MAX_MEMORY64_INITIAL_PAGES`). A 32-bit table's
        // `min` is already validator-capped at `MAX_TABLE_ELEMENTS`
        // itself, so this is a pure behavior-preserving widening for
        // every existing `is64: false` table.
        for table_type in &module.tables {
            tables.push(Table::new_with_is64(table_type.limits.min, table_type.limits.max, table_type.is64)?);
        }

        // The instance's persistent v128 heap (see `code/specs/
        // W15-wasm-v128-persistent-storage.md`) -- built up here, during
        // instantiation, so a `v128.const` in a global/data/elem
        // initializer allocates directly into the SAME `Vec` this
        // instance will keep for its whole lifetime, not a throwaway one.
        // Index 0 reserved as the all-zero entry, matching
        // `wasm_execution::WasmExecutionContext::v128_heap`'s convention.
        let mut v128_heap: Vec<[u8; 16]> = vec![[0u8; 16]];

        // Initialize globals.
        for global in &module.globals {
            global_types.push(global.global_type.clone());
            let value = evaluate_const_expr(&global.init_expr, &globals, &mut v128_heap)?;
            globals.push(value);
        }

        // Apply data segments. Stays targeting memory 0 regardless of
        // `seg.memory_index` (W16 scopes multi-memory support to
        // `memory.size`/`memory.grow` only -- see the spec's "What does
        // NOT change"); `wasm-validator` already bounds-checks
        // `seg.memory_index` against the real memory count, so this is a
        // real scope boundary, not a missed check.
        //
        // A PASSIVE segment (`is_passive`, task #95) is deliberately
        // skipped here -- applying it automatically would defeat the
        // entire point of `memory.init`/`data.drop`: a passive segment's
        // bytes stay resident, untouched, until an explicit `memory.init`
        // copies from it (any number of times, on demand), which is a
        // completely separate code path from this one-time instantiation-
        // time copy.
        if let Some(mem) = memories.first_mut() {
            // W25 (memory64): memory 0's `is64`-ness determines whether
            // this active data segment's offset expression is an
            // `i32.const` or `i64.const` -- `wasm-wast-parser` emits the
            // matching const-expr opcode for whichever memory 0 actually
            // is (this repo's data segments always target memory 0 --
            // see `wasm-validator`'s own Check 8 doc comment -- so only
            // memory 0's `is64` is ever relevant here).
            let is64 = mem.is64();
            for seg in &module.data {
                if seg.is_passive {
                    continue;
                }
                let offset = evaluate_const_expr(&seg.offset_expr, &globals, &mut v128_heap)?;
                let offset_num = if is64 {
                    offset.as_i64().map_err(|e| TrapError::new(e.message))? as usize
                } else {
                    offset.as_i32().map_err(|e| TrapError::new(e.message))? as usize
                };
                mem.write_bytes(offset_num, &seg.data)?;
            }
        }

        // Apply element segments. A passive segment (task #97) is never
        // applied automatically -- same "applying one automatically would
        // defeat the entire point" reasoning task #95 established for
        // `memory.init`'s passive data segments -- it stays resident for
        // an explicit `table.init` to copy from later.
        for elem in &module.elements {
            if elem.is_passive {
                continue;
            }
            if let Some(table) = tables.get_mut(elem.table_index as usize) {
                let offset = evaluate_const_expr(&elem.offset_expr, &globals, &mut v128_heap)?;
                let offset_num = offset.as_i32().map_err(|e| TrapError::new(e.message))? as u32;
                for (j, &func_idx) in elem.function_indices.iter().enumerate() {
                    table.set(offset_num + j as u32, func_idx)?;
                }
            }
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

        // One dropped-flag per element segment, all initially false (task
        // #97) -- `elem.drop` flips one to true; nothing in this
        // instantiation path drops anything itself.
        let dropped_elements = vec![false; module.elements.len()];

        let instance = WasmInstance {
            module: module.clone(),
            memories,
            tables,
            globals,
            global_types,
            func_types,
            func_bodies,
            host_functions,
            tags,
            tag_identities,
            exports,
            v128_heap,
            dropped_data_segments,
            dropped_elements,
        };

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
                ValueType::Anyref
                | ValueType::I31ref
                | ValueType::StructRef(_)
                | ValueType::Funcref
                | ValueType::Externref
                | ValueType::Exnref => WasmValue::I32(arg as i32),
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
        // `set_struct_field_counts` is indexed by the **wasm type index**, and
        // function and struct types share one index space. The encoder emits all
        // function types first, then the struct types, so a struct's wasm index
        // is `func_type_count + its position in struct_types`. We therefore pad
        // the front with filler slots for the function types (which are never
        // the target of a `struct.new`) and append the struct field counts.
        //
        // `func_type_count` MUST be the number of entries in the **type section**
        // (`module.types` — the encoder's *deduplicated* function types), NOT
        // `instance.func_types.len()`, which is populated one-per-function and so
        // over-counts whenever two functions share a signature. A Twig `record`
        // emits a constructor + N same-shape accessors + a predicate, so several
        // functions collapse to one function type: using the per-function count
        // then padded the struct's field-count entry to the wrong (too-high) index,
        // leaving the real `struct.new`/`struct.set` type index registered as a
        // zero-field filler — the "struct.set: field 0 out of range" trap. Modules
        // whose functions all have distinct types (e.g. a single-function cons
        // program, or the list-op helpers) were unaffected because the two counts
        // coincided there.
        //
        // (This assumes struct types follow *all* function types — true for the
        // cons modules we emit today, which declare no host imports. A module
        // that interleaved imported-function types after the struct types would
        // need order-preserving type parsing; not yet emitted or consumed.)
        if !instance.module.struct_types.is_empty() {
            let func_type_count = instance.module.types.len();
            let mut struct_field_counts = vec![0u32; func_type_count];
            struct_field_counts.extend(
                instance
                    .module
                    .struct_types
                    .iter()
                    .map(|st| st.fields.len() as u32),
            );
            engine.set_struct_field_counts(struct_field_counts);
        }

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
        ) -> Option<(GlobalType, WasmValue)> {
            None
        }

        fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
            None
        }

        fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
            None
        }
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
        assert!(host.memory.lock().unwrap().is_none());
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
        fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, WasmValue)> {
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

        fn resolve_global(&self, module_name: &str, name: &str) -> Option<(GlobalType, WasmValue)> {
            if module_name == "env" && name == "g" {
                Some((GlobalType { value_type: ValueType::I32, mutable: false }, WasmValue::I32(42)))
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
    #[test]
    fn instantiate_is_unreachable_for_a_module_that_fails_validation() {
        let runtime = WasmRuntime::new();
        let module = WasmModule {
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64 + 1, max: None },
                is64: false,
            }],
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
        assert!(err.to_string().contains("practical 64-bit table allocation cap"), "{err}");
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
        assert_eq!(instance.globals, vec![WasmValue::I32(42)]);
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
