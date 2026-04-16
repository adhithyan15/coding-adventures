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

use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use wasm_execution::{
    evaluate_const_expr, HostFunction, HostInterface, LinearMemory, Table, TrapError,
    WasmEngineConfig, WasmExecutionEngine, WasmValue,
};
use wasm_module_parser::WasmModuleParser;
use wasm_types::{
    ExternalKind, FuncType, FunctionBody, GlobalType, ImportTypeInfo, ValueType, WasmModule,
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
    pub stdout_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,

    /// Optional callback invoked for every line written to stderr (fd 2).
    pub stderr_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,

    /// Optional callback invoked when stdin bytes are requested (fd 0).
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
    /// Allocated linear memory.
    pub memory: Option<LinearMemory>,
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
    /// Export map: name -> (kind, index).
    pub exports: Vec<(String, ExternalKind, u32)>,
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
    pub fn instantiate(&self, module: &WasmModule) -> Result<WasmInstance, TrapError> {
        let mut func_types: Vec<FuncType> = Vec::new();
        let mut func_bodies: Vec<Option<FunctionBody>> = Vec::new();
        let mut host_functions: Vec<Option<Box<dyn HostFunction>>> = Vec::new();
        let mut global_types: Vec<GlobalType> = Vec::new();
        let mut globals: Vec<WasmValue> = Vec::new();
        let mut memory: Option<LinearMemory> = None;
        let mut tables: Vec<Table> = Vec::new();

        // Resolve imports.
        for imp in &module.imports {
            match &imp.type_info {
                ImportTypeInfo::Function(type_idx) => {
                    let ft = module.types[*type_idx as usize].clone();
                    func_types.push(ft);
                    func_bodies.push(None);

                    let host_func = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_function(&imp.module_name, &imp.name));
                    host_functions.push(host_func);
                }
                ImportTypeInfo::Memory(mem_type) => {
                    let imported_mem = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_memory(&imp.module_name, &imp.name));
                    if let Some(m) = imported_mem {
                        memory = Some(m);
                    } else {
                        memory = Some(LinearMemory::new(mem_type.limits.min, mem_type.limits.max));
                    }
                }
                ImportTypeInfo::Table(table_type) => {
                    let imported_table = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_table(&imp.module_name, &imp.name));
                    if let Some(t) = imported_table {
                        tables.push(t);
                    } else {
                        tables.push(Table::new(table_type.limits.min, table_type.limits.max));
                    }
                }
                ImportTypeInfo::Global(gt) => {
                    let imported_global = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_global(&imp.module_name, &imp.name));
                    if let Some((gtype, gval)) = imported_global {
                        global_types.push(gtype);
                        globals.push(gval);
                    } else {
                        global_types.push(gt.clone());
                        globals.push(WasmValue::default_for(gt.value_type));
                    }
                }
            }
        }

        // Add module-defined functions.
        for (i, &type_idx) in module.functions.iter().enumerate() {
            func_types.push(module.types[type_idx as usize].clone());
            func_bodies.push(module.code.get(i).cloned());
            host_functions.push(None);
        }

        // Allocate memory.
        if memory.is_none() && !module.memories.is_empty() {
            let mem_type = &module.memories[0];
            memory = Some(LinearMemory::new(mem_type.limits.min, mem_type.limits.max));
        }

        // Allocate tables.
        for table_type in &module.tables {
            tables.push(Table::new(table_type.limits.min, table_type.limits.max));
        }

        // Initialize globals.
        for global in &module.globals {
            global_types.push(global.global_type.clone());
            let value = evaluate_const_expr(&global.init_expr, &globals)?;
            globals.push(value);
        }

        // Apply data segments.
        if let Some(ref mut mem) = memory {
            for seg in &module.data {
                let offset = evaluate_const_expr(&seg.offset_expr, &globals)?;
                let offset_num = offset.as_i32().map_err(|e| TrapError::new(e.message))? as usize;
                mem.write_bytes(offset_num, &seg.data)?;
            }
        }

        // Apply element segments.
        for elem in &module.elements {
            if let Some(table) = tables.get_mut(elem.table_index as usize) {
                let offset = evaluate_const_expr(&elem.offset_expr, &globals)?;
                let offset_num = offset.as_i32().map_err(|e| TrapError::new(e.message))? as u32;
                for (j, &func_idx) in elem.function_indices.iter().enumerate() {
                    table.set(offset_num + j as u32, Some(func_idx))?;
                }
            }
        }

        // Build export list.
        let exports: Vec<(String, ExternalKind, u32)> = module
            .exports
            .iter()
            .map(|e| (e.name.clone(), e.kind, e.index))
            .collect();

        let instance = WasmInstance {
            module: module.clone(),
            memory,
            tables,
            globals,
            global_types,
            func_types,
            func_bodies,
            host_functions,
            exports,
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
            })
            .collect();

        // Build engine config, transferring ownership temporarily.
        let memory = instance.memory.take();
        let tables = std::mem::take(&mut instance.tables);
        let host_functions = std::mem::take(&mut instance.host_functions);

        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memory,
            tables,
            globals: instance.globals.clone(),
            global_types: instance.global_types.clone(),
            func_types: instance.func_types.clone(),
            func_bodies: instance.func_bodies.clone(),
            host_functions,
        });

        let results = engine.call_function(func_index, &wasm_args)?;
        let state = engine.into_state();
        instance.memory = state.memory;
        instance.tables = state.tables;
        instance.globals = state.globals;
        instance.host_functions = state.host_functions;

        // Convert back to i64.
        Ok(results
            .iter()
            .map(|r| match r {
                WasmValue::I32(v) => *v as i64,
                WasmValue::I64(v) => *v,
                WasmValue::F32(v) => *v as i64,
                WasmValue::F64(v) => *v as i64,
            })
            .collect())
    }

    /// Parse, validate, instantiate, and call in one step.
    pub fn load_and_run(
        &self,
        wasm_bytes: &[u8],
        entry: &str,
        args: &[i64],
    ) -> Result<Vec<i64>, String> {
        let module = self.load(wasm_bytes)?;
        self.validate(&module).map_err(|e| format!("{}", e))?;
        let mut instance = self.instantiate(&module).map_err(|e| format!("{}", e))?;
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
        let mut instance = runtime.instantiate(&module).unwrap();
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
        let instance = runtime.instantiate(&module).unwrap();

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
        let mut instance = runtime.instantiate(&module).unwrap();
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
            }],
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B], // i32.const 0; end
                data: vec![0x2A, 0x00, 0x00, 0x00],  // 42 in little-endian
            }],
            exports: vec![Export {
                name: "read".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            ..Default::default()
        };

        let runtime = WasmRuntime::new();
        let mut instance = runtime.instantiate(&module).unwrap();
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

        let mut instance = runtime.instantiate(&module).unwrap();
        let result = runtime.call(&mut instance, "call_double", &[5]).unwrap();
        assert_eq!(result, vec![10]);
    }

    #[test]
    fn test_instance_fields() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let module = runtime.load(&wasm).unwrap();
        let instance = runtime.instantiate(&module).unwrap();

        // No memory in square module
        assert!(instance.memory.is_none());
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
