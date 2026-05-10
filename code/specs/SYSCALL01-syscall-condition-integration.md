# SYSCALL01 — Syscall Library Condition Integration

**Status:** Draft  
**Series:** SYSCALL  
**Depends on:** SYSCALL00 (host syscall library, base), VMCOND00 (VM condition system)

---

## 1. Motivation

SYSCALL00 defines the base `lang-syscall` crate: a platform-agnostic library that
maps syscall numbers to host operations (write-byte, read-byte, exit, …) across
native, WASM/WASI, JVM, CLR, and BEAM targets. Its Phase 1 design is deliberately
simple: every syscall either succeeds or traps (aborts). This is correct and useful
for programs that don't need error recovery.

SYSCALL01 extends the syscall library to participate in the VM condition system
defined in VMCOND00. When the caller's module declares VMCOND00 Layer 3 or higher,
failing syscalls no longer trap — they signal structured conditions into the VM's
handler chain, giving calling code the opportunity to recover, retry, or substitute
a value. No change is needed in the syscall library's implementation for callers
that stay at Layer 0; the extension is purely additive.

The concrete goals are:

- Define the canonical condition type hierarchy for host I/O errors.
- Specify which syscall failures produce which condition types.
- Specify the standard restarts that the syscall library establishes, so handlers
  can invoke them without knowing the library's internals.
- Define the `SyscallHost` Rust trait additions that carry the VM callback.
- Describe how `lang-syscall` calls back into the VM to signal conditions.

---

## 2. Prerequisite: VMCOND00 Layers

SYSCALL01 uses VMCOND00 Layer 3 (dynamic handlers) as its minimum requirement.
Restarts (Layer 4) and non-local exits (Layer 5) are used by the standard restart
implementations but are optional for callers that only want to observe conditions
without recovering.

A module that imports `syscall/io` with checked semantics must declare at minimum
`LAYER_3` in its capability flags. A module that wants to use the standard restarts
(`use-value`, `retry`, `abort-with`) must also declare `LAYER_4` and `LAYER_5`.

The syscall library itself is responsible for emitting `PUSH_RESTART` before each
checkable syscall and `POP_RESTART` after. Callers only emit `PUSH_HANDLER` and the
`FIND_RESTART` / `INVOKE_RESTART` pair inside their handlers.

---

## 3. Condition Type Hierarchy

These types extend the VMCOND00 built-in hierarchy under `Error`. They are registered
by the `syscall/io` module in its module header's condition type table.

```
Error
└── IOCondition                     ; base for all I/O-related errors
    ├── WriteError                  ; write(2) / fd_write / OutputStream.write failed
    │     fields: fd:int, errno:int, bytes_written:int
    ├── ReadError                   ; read(2) / fd_read / InputStream.read failed
    │     fields: fd:int, errno:int
    ├── EOFCondition                ; read returned 0 bytes (end of stream)
    │     fields: fd:int
    │     note: subtype of IOCondition, NOT of ReadError — EOF is not an error
    ├── FileOpenError               ; open(2) / path_open failed
    │     fields: path:symbol, flags:int, errno:int
    ├── FileCloseError              ; close(2) / fd_close failed
    │     fields: fd:int, errno:int
    └── SeekError                   ; lseek(2) failed
          fields: fd:int, offset:int, errno:int

Warning
└── IOWarning                       ; base for non-fatal I/O advisories
    └── PartialWrite                ; write wrote fewer bytes than requested
          fields: fd:int, requested:int, written:int
```

```
Error
└── ExitRequest                     ; EXIT syscall (10) received
      fields: code:int
      note: this is not really an error — it's a signal that the program
            wants to exit. Handlers can inspect the exit code and either
            allow it (by not invoking a restart) or suppress it (by
            invoking the suppress-exit restart). Unhandled ExitRequest
            causes the VM to exit with the given code, not to abort.
```

All condition objects carry the fields defined in VMCOND00 Section 4 (`type_id`,
`fields`, `message`, `origin_ip`, `origin_fn`) in addition to the type-specific
fields listed above.

---

## 4. Syscall Number → Condition Mapping

This table specifies what happens when each syscall fails, for each capability layer
of the calling module.

| Syscall | Number | Layer 0 (trap) | Layer 1 (result) | Layer 3+ (signal) |
|---------|--------|----------------|-------------------|-------------------|
| write-byte | 1 | abort | `err = errno` | `SIGNAL WriteError{fd=1, errno, bytes_written=0}` |
| read-byte | 2 | abort | `err = errno` | `SIGNAL ReadError{fd=0, errno}` |
| read-byte (EOF) | 2 | return 255 | `err = EOF_SENTINEL` | `SIGNAL EOFCondition{fd=0}` |
| exit | 10 | process exits | process exits | `SIGNAL ExitRequest{code}` then exit if unhandled |
| write-str | 3 | abort | `err = errno` | `SIGNAL WriteError{fd=1, errno, bytes_written=n}` |
| write-str (partial) | 3 | abort | `(n, 0)` then check | `WARN PartialWrite{fd=1, requested, written=n}` |
| file-open | 30 | abort | `(fd, errno)` | `SIGNAL FileOpenError{path, flags, errno}` |
| file-read | 31 | abort | `(n, errno)` | `SIGNAL ReadError{fd, errno}` or `EOFCondition` |
| file-write | 32 | abort | `(n, errno)` | `SIGNAL WriteError{fd, errno, bytes_written=n}` |
| file-close | 33 | abort | `(0, errno)` | `SIGNAL FileCloseError{fd, errno}` |

**EOF distinction**: EOF on a read is not an error — it is a different kind of event.
At Layer 0 it returns the sentinel value 255 (for read-byte). At Layer 3 it signals
`EOFCondition` which is a subtype of `IOCondition` but NOT of `ReadError`. A handler
that handles `IOCondition` will catch both errors and EOF; a handler that only handles
`ReadError` will not catch EOF. This matches the Common Lisp convention where
`end-of-file` is distinct from stream errors.

**Exit**: `ExitRequest` is signaled before the process exits, giving handlers a chance
to clean up. If no handler handles `ExitRequest`, the VM exits with the given code
(this is the normal case). If a handler catches it and returns normally (without
invoking a restart), the exit is suppressed and execution continues. This allows
frameworks to intercept `exit` calls from library code.

---

## 5. Standard Restarts

The syscall library establishes the following named restarts around each checked
syscall call. These are what handlers can find and invoke.

### For write syscalls

**`return-zero`** — treat the failed write as having succeeded, return 0 to the caller.
Does not retry. Useful when the caller does not care about write failures (e.g.,
logging code that should never crash the program).

```
restart return-zero(condition: WriteError) -> int:
    EXIT_TO write-complete 0
```

**`retry-write`** — retry the write operation once. If it fails again, re-signal
the same condition (which may be caught by an outer handler or abort).

```
restart retry-write(condition: WriteError) -> int:
    result = host_write_byte_checked(fd, byte)
    EXIT_TO write-complete result
```

**`abort-write`** — raise an `Error` that cannot be silently ignored, forcing the
program to deal with the failure at a higher level.

```
restart abort-write(condition: WriteError):
    ERROR condition   // escalate to ERROR severity
```

### For read syscalls

**`use-value`** — substitute a caller-supplied value for the failed read. The handler
invokes this restart with the value it wants to substitute.

```
restart use-value(value: int) -> int:
    EXIT_TO read-complete value
```

**`return-eof`** — treat the read as EOF, returning 255 (the Layer 0 EOF sentinel).
Useful in handlers that want to unify error and EOF handling.

```
restart return-eof(_: ReadError) -> int:
    EXIT_TO read-complete 255
```

**`retry-read`** — retry the read once. If it fails again, re-signal.

```
restart retry-read(_: ReadError) -> int:
    result = host_read_byte_checked(fd)
    EXIT_TO read-complete result
```

### For exit

**`suppress-exit`** — cancel the requested exit and resume execution at the point
after the `exit` syscall. Useful for testing code that calls `exit()` or for
frameworks that want to restart a program component without restarting the process.

```
restart suppress-exit(_: ExitRequest):
    EXIT_TO exit-complete 0
```

**`change-exit-code`** — allow the exit but change the exit code. The handler invokes
this with the replacement code.

```
restart change-exit-code(new_code: int):
    host_exit(new_code)  // bypasses signaling, exits directly
```

---

## 6. The `SyscallHost` Trait — Rust Definition

The `lang-syscall` crate's core trait grows to carry a VM callback. The callback
is how the syscall library reaches back into the VM to signal a condition. The VM
provides this callback when constructing a `SyscallHost` implementation; the syscall
library calls it when a condition needs to be raised.

```rust
/// Callback from the syscall library into the VM condition system.
/// The VM implements this; the syscall library calls it.
pub trait ConditionSignaler: Send + Sync {
    /// Signal a condition into the VM's handler chain.
    /// Returns when the handler completes normally (non-unwinding path).
    /// May not return if a restart calls EXIT_TO (non-local exit path).
    fn signal(&self, condition: SyscallCondition);

    /// Like signal but abort the thread if no handler is found.
    fn error(&self, condition: SyscallCondition);

    /// Like signal but emit to stderr and continue if no handler is found.
    fn warn(&self, condition: SyscallCondition);

    /// Register a named restart in the VM's restart chain.
    /// Returns a guard that pops the restart when dropped.
    fn push_restart(&self, name: &'static str, handler: RestartFn) -> RestartGuard;
}

/// The unified trait for all syscall backends (replaces the Phase 1 version).
pub trait SyscallHost: Send {
    // ── Layer 0: always present ───────────────────────────────────────────

    /// Write one byte to stdout. Traps on failure.
    fn write_byte(&mut self, b: u8);

    /// Read one byte from stdin. Returns 255 on EOF. Traps on error.
    fn read_byte(&mut self) -> u8;

    /// Terminate the process with the given exit code. Never returns.
    fn exit(&mut self, code: i32) -> !;

    // ── Layer 1: result-returning variants ───────────────────────────────

    /// Write one byte. Returns Ok(()) or Err(SyscallError).
    fn write_byte_checked(&mut self, b: u8) -> Result<(), SyscallError>;

    /// Read one byte. Returns Ok(byte), Ok(EOF_SENTINEL=255 on EOF),
    /// or Err(SyscallError).
    fn read_byte_checked(&mut self) -> Result<u8, SyscallError>;

    // ── Layer 3+: condition-signaling variants ────────────────────────────

    /// Attach a condition signaler (provided by the VM at runtime).
    /// Must be called before any _conditioned methods are used.
    fn set_signaler(&mut self, signaler: Arc<dyn ConditionSignaler>);

    /// Write one byte, establishing restarts and signaling on failure.
    /// Requires set_signaler to have been called.
    fn write_byte_conditioned(&mut self, b: u8);

    /// Read one byte, establishing restarts and signaling on failure or EOF.
    fn read_byte_conditioned(&mut self) -> u8;

    /// Exit, signaling ExitRequest before terminating.
    fn exit_conditioned(&mut self, code: i32) -> !;
}
```

`SyscallError` is a lightweight struct:

```rust
pub struct SyscallError {
    pub code: i32,          // platform errno or equivalent
    pub kind: SyscallErrorKind,
}

pub enum SyscallErrorKind {
    WriteError { fd: i32, bytes_written: usize },
    ReadError  { fd: i32 },
    Eof        { fd: i32 },
    FileError  { path: Option<String>, flags: i32 },
    Other,
}
```

`SyscallCondition` is the type the signaler carries across the boundary:

```rust
pub enum SyscallCondition {
    WriteError   { fd: i32, errno: i32, bytes_written: usize },
    ReadError    { fd: i32, errno: i32 },
    EofCondition { fd: i32 },
    FileOpenError{ path: String, flags: i32, errno: i32 },
    PartialWrite { fd: i32, requested: usize, written: usize },
    ExitRequest  { code: i32 },
}
```

---

## 7. Integration Protocol

This section describes the runtime sequence when a conditioned syscall fails. The
goal is to show how control flows across the boundary between the syscall library
and the VM condition system.

### 7.1 Setup (at VM startup or module load)

```
1. VM creates a concrete SyscallHost implementation
   (NativeSyscallHost, WasiSyscallHost, StdioSyscallHost, etc.)

2. VM creates a ConditionSignaler that is wired to the current thread's
   handler chain and restart chain.

3. VM calls host.set_signaler(Arc::new(vm_signaler))

4. VM stores the configured host in the thread context.
```

### 7.2 Normal execution path (no failure)

```
Twig/LANG program:  (host/write-byte 65)
Compiler emits:     SYSCALL 1, reg[65]        ; Layer 0 — no conditions
VM executes:        host.write_byte(65)        ; succeeds, returns
```

No overhead. The conditioned path is not taken.

### 7.3 Conditioned execution path (Layer 3 caller, failure)

```
Twig/LANG program:  (host/write-byte-conditioned 65)
Compiler emits:     PUSH_RESTART use-value, <fn>
                    PUSH_RESTART retry-write, <fn>
                    PUSH_RESTART return-zero, <fn>
                    SYSCALL_CONDITIONED 1, reg[65]
                    POP_RESTART
                    POP_RESTART
                    POP_RESTART

VM executes SYSCALL_CONDITIONED:
  result = host.write_byte_conditioned(65)

Inside write_byte_conditioned (syscall library):
  result = platform_write(1, &65, 1)
  if result.is_err():
      condition = SyscallCondition::WriteError { fd: 1, errno: result.errno(), bytes_written: 0 }
      self.signaler.error(condition)     // call back into VM

Inside VM's ConditionSignaler::error():
  // Walk handler chain (VMCOND00 Section 5, ERROR opcode semantics)
  // If a handler is found, call it non-unwinding
  // The handler may INVOKE_RESTART one of the restarts established above
  // If no handler is found, abort the thread
```

### 7.4 Restart invocation path

```
Handler (written in LANG):
  (define (my-handler condition)
    (let ((r (find-restart 'use-value)))
      (if r
        (invoke-restart r 65)    ; substitute 65
        (error condition))))     ; re-raise if no restart

Compiler emits for handler body:
  FIND_RESTART  sym[use-value],  reg[r]
  BRANCH_IF_NIL reg[r], @no_restart
  LOAD_IMM      65, reg[val]
  INVOKE_RESTART reg[r], reg[val]     ; calls restart fn, which calls EXIT_TO
@no_restart:
  LOAD reg[condition], reg[err]
  ERROR reg[err]
```

When `INVOKE_RESTART` calls the `use-value` restart, the restart executes:

```rust
// restart implementation (in lang-syscall)
fn use_value_restart(value: LangValue) {
    // This calls EXIT_TO in the VM, unwinding back to the ESTABLISH_EXIT
    // that was set up around the SYSCALL_CONDITIONED instruction.
    // The ExitPointNode records that the result register should receive `value`.
    vm_exit_to("write-complete", value);
}
```

Execution resumes at the instruction after `POP_RESTART` (the post-syscall code),
with the substituted value in the result register.

---

## 8. The `SYSCALL_CONDITIONED` Opcode

This is a new opcode added to the compiler IR for Layer 3 callers. It is the
Layer 3 equivalent of `SYSCALL` (Layer 0) and `SYSCALL_CHECKED` (Layer 1).

```
SYSCALL_CONDITIONED <n:imm> <arg:reg> → <result:reg>
```

The compiler emits surrounding `PUSH_RESTART` / `POP_RESTART` and
`ESTABLISH_EXIT` / (implicit exit point) instructions. The full template
the compiler generates for `(host/write-byte-conditioned expr)` is:

```
; Evaluate expr into reg[arg]
<expr bytecode>

; Establish the exit point that restarts jump to
ESTABLISH_EXIT  sym[write-complete], reg[result], @after_syscall

; Push standard restarts (outermost first — innermost is searched first)
PUSH_RESTART    sym[return-zero],  fn[return_zero_impl]
PUSH_RESTART    sym[retry-write],  fn[retry_write_impl]
PUSH_RESTART    sym[use-value],    fn[use_value_impl]

; The conditioned call
SYSCALL_CONDITIONED  1, reg[arg]      ; may signal, may invoke a restart, may abort

; Teardown (only reached on normal success path)
POP_RESTART     ; use-value
POP_RESTART     ; retry-write
POP_RESTART     ; return-zero

@after_syscall:
; reg[result] holds the return value (0 on success, substitute from restart on recovery)
```

The VM executes `SYSCALL_CONDITIONED` by:

1. Calling `host.write_byte_conditioned(arg)`.
2. If the call returns normally (success): continue.
3. If the call signals (via `ConditionSignaler::error`): walk the handler chain.
4. If a handler invokes a restart that calls `EXIT_TO sym[write-complete]`:
   unwind to `@after_syscall` with the restart's value in `reg[result]`.

---

## 9. VM-Side Implementation Notes for the `ConditionSignaler`

The VM provides the `ConditionSignaler` implementation. The key constraint is that
`ConditionSignaler::signal` (and `::error`) must be callable from within Rust code
running on behalf of the syscall library, but they need to walk and invoke VM data
structures (the handler chain, restart chain).

Two viable implementation patterns:

### Pattern A: Shared state with a thread-local VM context

```rust
struct VmConditionSignaler {
    // Arc to the thread's execution context, shared with the VM interpreter
    context: Arc<Mutex<ThreadContext>>,
}

impl ConditionSignaler for VmConditionSignaler {
    fn error(&self, condition: SyscallCondition) {
        let mut ctx = self.context.lock().unwrap();
        let lang_cond = ctx.make_condition(condition);
        ctx.execute_error(lang_cond);  // walks handler chain, may not return
    }
}
```

### Pattern B: Callback function pointer (suitable for FFI / no_std)

```rust
pub type SignalFn = unsafe extern "C" fn(condition: *const RawCondition) -> SignalOutcome;
pub enum SignalOutcome { Handled, Unhandled, NonLocalExit }

struct FfiConditionSignaler {
    signal_fn: SignalFn,
    error_fn:  SignalFn,
    warn_fn:   SignalFn,
}
```

Pattern B is better for embedding scenarios (the VM is a C library, or the syscall
library is compiled without knowing the VM type). Pattern A is better for the
all-Rust monolithic VM. Both can be supported via the `ConditionSignaler` trait.

---

## 10. Errno Codes

The `errno` field in condition objects uses platform-independent codes defined in the
`lang-syscall` crate. These map to platform errno values in the native backend:

```rust
pub mod errno {
    pub const OK:          i32 = 0;
    pub const EPERM:       i32 = 1;   // Operation not permitted
    pub const ENOENT:      i32 = 2;   // No such file or directory
    pub const EIO:         i32 = 5;   // I/O error
    pub const EBADF:       i32 = 9;   // Bad file descriptor
    pub const EAGAIN:      i32 = 11;  // Resource temporarily unavailable
    pub const EACCES:      i32 = 13;  // Permission denied
    pub const EEXIST:      i32 = 17;  // File exists
    pub const ENOTDIR:     i32 = 20;  // Not a directory
    pub const EISDIR:      i32 = 21;  // Is a directory
    pub const EINVAL:      i32 = 22;  // Invalid argument
    pub const ENOSPC:      i32 = 28;  // No space left on device
    pub const EPIPE:       i32 = 32;  // Broken pipe
    pub const EOF_SENTINEL:i32 = -1;  // Not an OS errno — signals EOF
}
```

JVM, CLR, and BEAM backends translate their native exception types to these codes when
populating condition objects. The layer 1 `err` register uses the same values.

---

## 11. Phase Plan

### Phase 1 — Condition type registration + EOFCondition
- Register the `IOCondition` hierarchy in the `syscall/io` module's type table
- Implement `EOFCondition` signaling for `read-byte` at Layer 3
- Add `set_signaler` and `read_byte_conditioned` to the `SyscallHost` trait
- Acceptance: a Twig program with `(import stdlib/io)` and a `PUSH_HANDLER` for
  `EOFCondition` can catch EOF from `(host/read-byte-conditioned)` without aborting

### Phase 2 — WriteError + standard restarts
- Implement `write_byte_conditioned` with `WriteError` signaling
- Emit `PUSH_RESTART` / `POP_RESTART` in the compiler for `SYSCALL_CONDITIONED`
- Implement `use-value`, `retry-write`, `return-zero` restart functions
- Acceptance: a handler that catches `WriteError` and invokes `return-zero` allows
  the program to continue past a broken-pipe write

### Phase 3 — ExitRequest
- Signal `ExitRequest` from `exit_conditioned` (syscall 10)
- Implement `suppress-exit` and `change-exit-code` restarts
- Acceptance: a test-framework module can catch `ExitRequest` from library code that
  calls `exit(1)` and suppress it, continuing the test run

### Phase 4 — Full IOCondition hierarchy
- FileOpenError, FileCloseError, SeekError
- PartialWrite warning
- Layer 1 error codes standardised across all backends
- Acceptance: all condition types are registered; backends populate errno fields
  correctly on native, WASM/WASI, JVM, and CLR
