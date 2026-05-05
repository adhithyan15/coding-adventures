# SYSCALL00 — Host Syscall Library

**Status:** Draft  
**Series:** SYSCALL  
**Depends on:** LANG02 (vm-core), LANG05 (backend-protocol)  
**Extended by:** SYSCALL01 (condition integration)

---

## 1. Purpose

Every language that runs on the LANG VM chain eventually needs to interact with its
host environment: write a byte to stdout, read a byte from stdin, exit the process,
open a file. These operations differ across every execution context — native Linux,
native macOS, WASM/WASI, browser WASM, JVM, CLR, BEAM — but the *calling language*
should not have to know which host it is running on.

`lang-syscall` is the answer: a single Rust crate that provides a numbered syscall
table, a host-agnostic trait, and a set of platform backends, so that any LANG VM
implementation can delegate host I/O through one uniform interface.

**What this crate is:**
- The canonical numbering for every host operation the LANG VM exposes.
- A C-ABI entry point (`__lang_syscall`) for use by ahead-of-time compiled programs
  that are statically linked against the library.
- A `SyscallHost` Rust trait for use by the VM interpreter.
- Platform backends: native libc, WASM/WASI, and a standard-I/O fallback.

**What this crate is not:**
- Anything JVM-backend-specific, CLR-backend-specific, or BEAM-backend-specific. Those
  backends lower syscall opcodes to their own native calls at code-generation time.
  The table in this spec is the numbering they must agree on; the Rust library is not
  linked into those backends.
- Anything to do with condition signaling. That is SYSCALL01, which layers on top.

---

## 2. Canonical Syscall Table

This table is the single source of truth. Every backend — the Rust C-ABI, the Rust
trait, the JVM bytecode emitter, the CLR bytecode emitter, the BEAM emitter, and the
Python vm-core — must use the same numbers.

Numbers are grouped in decades by category. Gaps are reserved for future use within
the same category.

### 2.1 I/O — stdout / stdin (1–9)

| # | Name | Arg | Return | Description |
|---|------|-----|--------|-------------|
| 1 | `write-byte` | `u8` byte | `i64` (0 ok) | Write one byte to stdout |
| 2 | `read-byte` | *(none, pass 0)* | `i64` byte or -1 on EOF | Read one byte from stdin |
| 3 | `write-str` | `(ptr: *u8, len: usize)` encoded as `i64` | `i64` bytes written | Write `len` bytes from `ptr` to stdout |
| 4–9 | *(reserved)* | | | |

`write-str` packs the pointer and length into a single `i64` using the platform's
pointer width. On 64-bit platforms the upper 32 bits are the pointer (truncated), lower
32 are the length; this is a Phase 2 concern. Phase 1 only implements 1 and 2.

EOF convention: `read-byte` returns `-1` (as an `i64`) on EOF, not `255`. The `255`
convention in earlier Twig code was a byte-level sentinel; the syscall-level return is
a full integer where `-1` is unambiguous.

### 2.2 Process lifecycle (10–19)

| # | Name | Arg | Return | Description |
|---|------|-----|--------|-------------|
| 10 | `exit` | `i32` code | *(never returns)* | Terminate the process |
| 11–19 | *(reserved)* | | | |

### 2.3 Threads (20–29)

| # | Name | Arg | Return | Description |
|---|------|-----|--------|-------------|
| 20 | `thread-spawn` | fn pointer | `i64` handle | Spawn an OS thread |
| 21 | `thread-join` | `i64` handle | `i64` result | Join a thread |
| 22–29 | *(reserved)* | | | |

Phase 3 work. The `fn pointer` encoding and thread handle format will be specified
when implemented.

### 2.4 Filesystem (30–39)

| # | Name | Args | Return | Description |
|---|------|------|--------|-------------|
| 30 | `file-open` | path ptr+len, flags | `i64` fd or -errno | Open a file |
| 31 | `file-read` | fd, buf ptr+len | `i64` n or -errno | Read from fd |
| 32 | `file-write` | fd, buf ptr+len | `i64` n or -errno | Write to fd |
| 33 | `file-close` | fd | `i64` (0 ok or -errno) | Close fd |
| 34–39 | *(reserved)* | | | |

Phase 2 work.

---

## 3. Error Return Convention

The C ABI and the `SyscallHost` trait use a signed `i64` return for all syscalls:

- **≥ 0**: success, return value (bytes written, bytes read, fd, …)
- **-1**: EOF (for read operations only)
- **< -1**: negated errno — e.g., `EPERM` → `-1`, `EIO` → `-5`, `EBADF` → `-9`

The mapping of error names to numbers follows the platform-independent errno table
defined in SYSCALL01 Section 10. The C ABI never panics; it returns error codes.
Higher layers (SYSCALL01) turn those codes into conditions.

For `exit` (syscall 10): the process terminates; the function never returns. The C ABI
declares the return type as `!` (Rust) or `[[noreturn]]` (C). The VM trait returns `!`.

---

## 4. The C ABI

The C ABI is a single entry point for use by ahead-of-time compiled programs:

```c
/* Declared in lang_syscall.h */
int64_t __lang_syscall(uint32_t n, int64_t arg) __attribute__((noreturn_for_10));
```

In Rust:

```rust
/// The single C-ABI dispatch entry point.
///
/// `n` is the syscall number from the canonical table (Section 2).
/// `arg` is a syscall-specific argument.  For write-byte, it is the byte
/// value (low 8 bits used).  For read-byte, it is ignored (pass 0).
/// For exit, it is the exit code.
///
/// Returns an i64 whose meaning is syscall-specific:
/// - write-byte: 0 on success, negative errno on failure
/// - read-byte:  the byte read (0..=255), -1 on EOF, negative errno on error
/// - exit:       never returns
///
/// Unknown syscall numbers return -EINVAL (-22).
#[unsafe(no_mangle)]
pub extern "C" fn __lang_syscall(n: u32, arg: i64) -> i64 {
    // dispatches to the active backend (feature-selected at compile time)
}
```

One entry point rather than per-symbol exports (`__lang_write_byte`,
`__lang_read_byte`, …) keeps the ABI surface minimal and matches exactly how the
`SYSCALL <n> <arg>` IR opcode works — one call site regardless of which operation.
Backends that emit native code only need to emit one call instruction and one constant.

---

## 5. The `SyscallHost` Rust Trait

For the Rust VM interpreter, the syscall abstraction is a trait rather than a static
function, so tests can substitute mock implementations and the VM can be embedded in
environments where stdout is not a terminal.

```rust
/// Platform-agnostic host operations for the LANG VM interpreter.
///
/// Implementors provide the actual I/O operations; the VM interpreter
/// holds a `Box<dyn SyscallHost>` and calls through it.
///
/// Phase 1: write-byte, read-byte, exit.
/// Phase 2: write-str, file operations (added as default methods).
/// SYSCALL01: conditioned variants added as a separate trait extension.
pub trait SyscallHost: Send {
    // ── Phase 1 ───────────────────────────────────────────────────────────

    /// Write one byte to stdout.
    ///
    /// Returns `Ok(())` on success.  Returns `Err(errno)` on failure where
    /// `errno` is a value from the platform-independent errno table
    /// (SYSCALL01 §10).  The caller decides whether to trap or signal.
    fn write_byte(&mut self, b: u8) -> Result<(), i32>;

    /// Read one byte from stdin.
    ///
    /// Returns `Ok(Some(byte))` on success, `Ok(None)` on EOF,
    /// `Err(errno)` on error.
    fn read_byte(&mut self) -> Result<Option<u8>, i32>;

    /// Terminate the process with the given exit code.  Never returns.
    fn exit(&mut self, code: i32) -> !;

    // ── Phase 2 (default impls return Err(ENOSYS)) ────────────────────────

    /// Write bytes from a slice to stdout.
    fn write_str(&mut self, bytes: &[u8]) -> Result<usize, i32> {
        for &b in bytes {
            self.write_byte(b)?;
        }
        Ok(bytes.len())
    }
}
```

The `Result<(), i32>` return — rather than `Result<(), SyscallError>` — keeps the
trait object-safe and avoids an allocation on every call. The `i32` is an errno value
from the canonical table. SYSCALL01 wraps this into condition objects.

---

## 6. Platform Backends

### 6.1 `native` — libc (default)

Selected when `cfg(not(target_arch = "wasm32"))`. Uses `libc::write` and `libc::read`
directly via the `libc` crate.

```rust
pub struct NativeSyscallHost;

impl SyscallHost for NativeSyscallHost {
    fn write_byte(&mut self, b: u8) -> Result<(), i32> {
        let ret = unsafe { libc::write(1, &b as *const u8 as *const _, 1) };
        if ret == 1 { Ok(()) } else { Err(errno()) }
    }

    fn read_byte(&mut self) -> Result<Option<u8>, i32> {
        let mut b = 0u8;
        let ret = unsafe { libc::read(0, &mut b as *mut u8 as *mut _, 1) };
        match ret {
            1  => Ok(Some(b)),
            0  => Ok(None),   // EOF
            _  => Err(errno()),
        }
    }

    fn exit(&mut self, code: i32) -> ! {
        unsafe { libc::exit(code) }
    }
}
```

### 6.2 `wasi` — WASM/WASI preview 1

Selected when `cfg(target_arch = "wasm32")` and feature `wasi` is enabled. Uses
`wasi::fd_write`, `wasi::fd_read`, `wasi::proc_exit` from the `wasi` crate.

```rust
pub struct WasiSyscallHost;

impl SyscallHost for WasiSyscallHost {
    fn write_byte(&mut self, b: u8) -> Result<(), i32> {
        let iov = wasi::Ciovec { buf: &b as *const u8, buf_len: 1 };
        let n = wasi::fd_write(1, &[iov]).map_err(|e| e.raw() as i32)?;
        if n == 1 { Ok(()) } else { Err(-libc::EIO) }
    }
    // ... read_byte, exit analogously
}
```

### 6.3 `stdio` — Rust std::io (VM interpreter default)

A portable implementation using Rust's `std::io::stdout()` and `std::io::stdin()`.
Suitable for the Rust VM interpreter on any platform where `std` is available. This is
the implementation `Box<dyn SyscallHost>` defaults to in tests and the interpreter.

```rust
pub struct StdioSyscallHost {
    stdout: std::io::Stdout,
    stdin:  std::io::Stdin,
}

impl Default for StdioSyscallHost {
    fn default() -> Self {
        Self { stdout: std::io::stdout(), stdin: std::io::stdin() }
    }
}

impl SyscallHost for StdioSyscallHost {
    fn write_byte(&mut self, b: u8) -> Result<(), i32> {
        use std::io::Write;
        self.stdout.write_all(&[b]).map_err(|e| os_error_to_errno(e))
    }

    fn read_byte(&mut self) -> Result<Option<u8>, i32> {
        use std::io::Read;
        let mut buf = [0u8; 1];
        match self.stdin.read(&mut buf) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(buf[0])),
            Err(e) => Err(os_error_to_errno(e)),
        }
    }

    fn exit(&mut self, code: i32) -> ! {
        std::process::exit(code)
    }
}
```

---

## 7. Feature Flags

```toml
[features]
default = ["native"]

# Use libc for native platforms (Linux, macOS, Windows via MSVC libc)
native = ["dep:libc"]

# Use WASI preview 1 for wasm32 targets
wasi = ["dep:wasi"]

# Include the SyscallHost trait and StdioSyscallHost implementation.
# Always compiled; this flag is a no-op but documents intent.
vm = []

# Disable std — requires the embedding host to supply all I/O.
# Incompatible with `stdio`. Use with `native` or `wasi`.
no-std = []
```

---

## 8. Crate Layout

```
code/packages/rust/lang-syscall/
├── Cargo.toml
├── README.md
├── CHANGELOG.md
├── src/
│   ├── lib.rs          — re-exports, __lang_syscall entry point, SyscallHost trait
│   ├── table.rs        — pub const WRITE_BYTE: u32 = 1; … (canonical number table)
│   ├── errno.rs        — platform-independent errno values (shared with SYSCALL01)
│   ├── native.rs       — NativeSyscallHost  (#[cfg(not(target_arch = "wasm32"))])
│   ├── wasi.rs         — WasiSyscallHost    (#[cfg(target_arch = "wasm32")])
│   └── stdio.rs        — StdioSyscallHost   (always compiled when std is available)
└── tests/
    ├── test_write_byte.rs
    ├── test_read_byte.rs
    └── test_c_abi.rs
```

---

## 9. Relationship to the Python vm-core

The Python `vm-core` package (LANG02) dispatches `io_in` and `io_out` interpreter IR
opcodes and `call_builtin "syscall"` calls through Python's `sys.stdout` / `sys.stdin`
directly. The `lang-syscall` Rust crate **does not replace** the Python vm-core's I/O
mechanism; the two coexist during the transition to the Rust port.

What both share is the **syscall number table** (Section 2). When vm-core dispatches
`call_builtin "syscall" 1 arg`, the number `1` is the same canonical number defined
here. VMCOND00's `SYSCALL_CHECKED` opcode also uses these same numbers.

The Rust `lang-syscall` crate becomes the sole I/O implementation once the Rust vm-core
(the port of the Python vm-core) is complete. Until then, the Python and Rust
implementations are parallel.

---

## 10. Security

- `__lang_syscall` validates `n` against the known table before dispatching. Unknown
  values return `-EINVAL` rather than invoking undefined behaviour.
- `write-str` (syscall 3) will validate that the `ptr+len` pair is within the VM's
  memory bounds before calling `libc::write`. Bounds checking is a Phase 2 concern.
- No shell expansion, no `exec`, no spawning subprocesses. The table is intentionally
  minimal; operations outside it are not accessible through this interface.

---

## 11. Phase Plan

### Phase 1 — write-byte, read-byte, exit (this spec)
- `lang-syscall` Rust crate with `StdioSyscallHost` and `NativeSyscallHost`
- `__lang_syscall` C ABI (n=1,2,10 only)
- `SyscallHost` trait with `write_byte`, `read_byte`, `exit`
- `table.rs` constants: `WRITE_BYTE=1`, `READ_BYTE=2`, `EXIT=10`
- Tests: write a byte and read it back via a pipe; exit code propagation

### Phase 2 — write-str, file I/O
- Syscalls 3, 30–33
- `write_str` default method promoted to a real implementation
- WASI backend completed
- `errno.rs` populated with the full error code table

### Phase 3 — Threads
- Syscalls 20–21
- POSIX `pthread_create` / `pthread_join` on native
- WASM thread proposal on WASI (if available)
- Goroutine-style scheduling is out of scope (see future LANG28)
