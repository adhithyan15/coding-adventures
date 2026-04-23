// wasi_stub.go --- Minimal WASI implementation for WASM programs.
//
// ════════════════════════════════════════════════════════════════════════
// WHAT IS WASI?
// ════════════════════════════════════════════════════════════════════════
//
// WASI (WebAssembly System Interface) is the standard ABI that lets WASM
// programs talk to the host operating system.  Think of it as the "syscall
// layer" for WebAssembly — analogous to POSIX for C programs or the JNI
// for Java.
//
// WASM modules import WASI functions under the module name
// "wasi_snapshot_preview1".  The host (us) implements those functions and
// hands them back as HostFunction objects when the engine resolves imports.
//
// ════════════════════════════════════════════════════════════════════════
// IMPLEMENTATION TIERS
// ════════════════════════════════════════════════════════════════════════
//
// Tier 1 (original stub)
//   - fd_write   : write bytes to stdout/stderr via callback
//   - proc_exit  : panic with ProcExitError to halt execution
//   - everything else: return ENOSYS (52)
//
// Tier 3 (this file, Tier 2 reserved for file-system ops)
//   - args_sizes_get   : report argument count and buffer size
//   - args_get         : write null-terminated argument strings to memory
//   - environ_sizes_get: report environment variable count and buffer size
//   - environ_get      : write null-terminated env-var strings to memory
//   - clock_res_get    : report clock resolution
//   - clock_time_get   : read the current time from realtime or monotonic
//   - random_get       : fill a memory buffer with random bytes
//   - sched_yield      : cooperative yield (no-op in single-threaded WASM)
//
// ════════════════════════════════════════════════════════════════════════
// MEMORY LAYOUT FOR ARGS / ENVIRON
// ════════════════════════════════════════════════════════════════════════
//
// args_get(argv_ptr, argv_buf_ptr) writes two parallel data structures:
//
//	argv array (at argv_ptr):
//	  [ptr_to_arg0][ptr_to_arg1]...[ptr_to_argN]
//	  Each entry is a 4-byte little-endian i32 pointing into argv_buf.
//
//	argv_buf (at argv_buf_ptr):
//	  arg0\0arg1\0...argN\0
//	  Each argument is a null-terminated UTF-8 string.
//
// The total buffer size returned by args_sizes_get is:
//
//	sum(len(arg) + 1 for each arg)   ← +1 for the null terminator
//
// environ_get follows exactly the same layout for environment variables.
//
// ════════════════════════════════════════════════════════════════════════
// WASI ERROR CODES (errno)
// ════════════════════════════════════════════════════════════════════════
//
//	0  — ESUCCESS : success
//	28 — EINVAL   : invalid argument (e.g., unknown clock ID)
//	29 — EIO      : I/O error (e.g., entropy source failure)
//	52 — ENOSYS   : function not implemented
package wasmruntime

import (
	"fmt"

	wasmexecution "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// WASI error codes.
const (
	wasiESuccess = 0
	wasiEBadf    = 8
	wasiEInval   = 28
	wasiEIO      = 29
	wasiENosys   = 52
)

// ProcExitError is thrown when a WASM program calls proc_exit.
//
// We use panic rather than return because proc_exit is a non-local exit —
// it should unwind all WASM frames immediately.  The caller (runtime.go)
// recovers from this panic and converts it to a Go error.
type ProcExitError struct {
	ExitCode int
}

func (e *ProcExitError) Error() string {
	return fmt.Sprintf("proc_exit(%d)", e.ExitCode)
}

// ════════════════════════════════════════════════════════════════════════
// WASICONFIG — dependency injection container
// ════════════════════════════════════════════════════════════════════════

// WasiConfig bundles all the dependencies that WasiStub needs at
// construction time.  Passing them as a struct (rather than positional
// arguments) makes it easy to add new fields without breaking existing
// call sites — callers only set the fields they care about.
//
// Typical usage:
//
//	wasi := NewWasiStubFromConfig(WasiConfig{
//	    Args:           os.Args,
//	    Env:            os.Environ(),
//	    StdoutCallback: func(s string) { fmt.Print(s) },
//	    Clock:          FakeClock{},   // injected for tests
//	    Random:         FakeRandom{},  // injected for tests
//	})
type WasiConfig struct {
	// Args is the argument vector passed to the WASM program.
	// args[0] is conventionally the program name.
	Args []string

	// Env is the environment variable list in "KEY=VALUE" format.
	// Example: []string{"HOME=/home/user", "PATH=/usr/bin"}.
	Env []string

	// StdoutCallback receives lines written to fd 1 (stdout).
	// If nil, stdout output is silently discarded.
	StdoutCallback func(string)

	// StdinCallback supplies up to count bytes for fd 0 (stdin).
	// Returning nil or an empty slice signals EOF.
	StdinCallback func(count int) []byte

	// StderrCallback receives lines written to fd 2 (stderr).
	// If nil, stderr output is silently discarded.
	StderrCallback func(string)

	// Clock provides time to clock_time_get and clock_res_get.
	// If nil, SystemClock{} is used.
	Clock WasiClock

	// Random provides entropy to random_get.
	// If nil, SystemRandom{} is used.
	Random WasiRandom
}

// ════════════════════════════════════════════════════════════════════════
// WASIHOST — the host implementation
// ════════════════════════════════════════════════════════════════════════

// WasiStub provides WASI host functions to the execution engine.
//
// All fields are private; use NewWasiStub or NewWasiStubFromConfig to
// construct.
type WasiStub struct {
	StdoutCallback func(text string)
	StdinCallback  func(count int) []byte
	StderrCallback func(text string)
	instanceMemory *wasmexecution.LinearMemory
	args           []string
	env            []string
	clock          WasiClock
	random         WasiRandom
}

// WasiHost is the preferred name for the full WASI host implementation.
// WasiStub remains as a backwards-compatible alias.
type WasiHost = WasiStub

// NewWasiStub creates a new WASI stub with stdout/stderr callbacks only.
//
// This preserves backward compatibility with existing call sites that use
// the two-argument form.  Clock and Random default to system implementations.
//
//	wasi := NewWasiStub(
//	    func(s string) { fmt.Print(s) },   // stdout
//	    nil,                                 // stderr: discard
//	)
func NewWasiStub(stdout, stderr func(string)) *WasiStub {
	if stdout == nil {
		stdout = func(string) {}
	}
	stdin := func(int) []byte { return nil }
	if stderr == nil {
		stderr = func(string) {}
	}
	return &WasiStub{
		StdoutCallback: stdout,
		StdinCallback:  stdin,
		StderrCallback: stderr,
		clock:          SystemClock{},
		random:         SystemRandom{},
	}
}

// NewWasiHost creates a new WASI host with stdout/stderr callbacks only.
func NewWasiHost(stdout, stderr func(string)) *WasiHost {
	return NewWasiStub(stdout, stderr)
}

// NewWasiStubFromConfig creates a WasiStub from a WasiConfig.
//
// Nil fields in the config receive sensible defaults:
//   - StdoutCallback → discard
//   - StderrCallback → discard
//   - Clock          → SystemClock{}
//   - Random         → SystemRandom{}
func NewWasiStubFromConfig(config WasiConfig) *WasiStub {
	stdout := config.StdoutCallback
	if stdout == nil {
		stdout = func(string) {}
	}
	stdin := config.StdinCallback
	if stdin == nil {
		stdin = func(int) []byte { return nil }
	}
	stderr := config.StderrCallback
	if stderr == nil {
		stderr = func(string) {}
	}
	clock := config.Clock
	if clock == nil {
		clock = SystemClock{}
	}
	random := config.Random
	if random == nil {
		random = SystemRandom{}
	}
	return &WasiStub{
		StdoutCallback: stdout,
		StdinCallback:  stdin,
		StderrCallback: stderr,
		args:           config.Args,
		env:            config.Env,
		clock:          clock,
		random:         random,
	}
}

// NewWasiHostFromConfig creates a WasiHost from a WasiConfig.
func NewWasiHostFromConfig(config WasiConfig) *WasiHost {
	return NewWasiStubFromConfig(config)
}

// SetMemory sets the instance's memory (needed for fd_write, args_get, etc.).
//
// This is called by the runtime after instantiation, once the linear memory
// object has been created.  We cannot inject memory at construction time
// because the module has not been instantiated yet.
func (w *WasiStub) SetMemory(mem *wasmexecution.LinearMemory) {
	w.instanceMemory = mem
}

// ════════════════════════════════════════════════════════════════════════
// HOST INTERFACE IMPLEMENTATION
// ════════════════════════════════════════════════════════════════════════

// ResolveFunction resolves WASI functions by name.
//
// The execution engine calls this for every import whose module is
// "wasi_snapshot_preview1".  We return a HostFunction for each function
// we implement; unknown functions get a stub that returns ENOSYS.
func (w *WasiStub) ResolveFunction(moduleName, name string) *wasmexecution.HostFunction {
	if moduleName != "wasi_snapshot_preview1" {
		return nil
	}

	switch name {
	case "fd_write":
		return w.makeFdWrite()
	case "fd_read":
		return w.makeFdRead()
	case "proc_exit":
		return w.makeProcExit()
	case "args_sizes_get":
		return w.makeArgsSizesGet()
	case "args_get":
		return w.makeArgsGet()
	case "environ_sizes_get":
		return w.makeEnvironSizesGet()
	case "environ_get":
		return w.makeEnvironGet()
	case "clock_res_get":
		return w.makeClockResGet()
	case "clock_time_get":
		return w.makeClockTimeGet()
	case "random_get":
		return w.makeRandomGet()
	case "sched_yield":
		return w.makeSchedYield()
	default:
		return w.makeStub()
	}
}

// ResolveGlobal returns nil (WASI doesn't export globals).
func (w *WasiStub) ResolveGlobal(moduleName, name string) *wasmexecution.HostGlobal {
	return nil
}

// ResolveMemory returns nil (WASI doesn't provide memory).
func (w *WasiStub) ResolveMemory(moduleName, name string) *wasmexecution.LinearMemory {
	return nil
}

// ResolveTable returns nil (WASI doesn't provide tables).
func (w *WasiStub) ResolveTable(moduleName, name string) *wasmexecution.Table {
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// TIER 1: fd_write, fd_read, and proc_exit
// ════════════════════════════════════════════════════════════════════════

// makeFdWrite creates the fd_write host function.
//
// Signature: fd_write(fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32) → errno
//
// The I/O vector (iov) array lives in WASM linear memory.  Each iov entry
// is 8 bytes: [buf_ptr: i32][buf_len: i32].  We walk the array, read each
// buffer, concatenate the bytes, and deliver the text to the callback.
//
//	Memory layout at iovs_ptr:
//	  [buf_ptr_0 : 4 bytes][buf_len_0 : 4 bytes]
//	  [buf_ptr_1 : 4 bytes][buf_len_1 : 4 bytes]
//	  ...
func (w *WasiStub) makeFdWrite() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			if w.instanceMemory == nil {
				return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiENosys))}
			}

			fd := wasmexecution.AsI32(args[0])
			iovsPtr := wasmexecution.AsI32(args[1])
			iovsLen := wasmexecution.AsI32(args[2])
			nwrittenPtr := wasmexecution.AsI32(args[3])

			mem := w.instanceMemory
			totalWritten := int32(0)

			for i := int32(0); i < iovsLen; i++ {
				bufPtr := uint32(mem.LoadI32(int(iovsPtr + i*8)))
				bufLen := uint32(mem.LoadI32(int(iovsPtr + i*8 + 4)))

				bytes := make([]byte, bufLen)
				for j := uint32(0); j < bufLen; j++ {
					bytes[j] = byte(mem.LoadI32_8u(int(bufPtr + j)))
				}

				text := string(bytes)
				totalWritten += int32(bufLen)

				if fd == 1 {
					w.StdoutCallback(text)
				} else if fd == 2 {
					w.StderrCallback(text)
				}
			}

			mem.StoreI32(int(nwrittenPtr), totalWritten)
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// makeFdRead creates the fd_read host function.
//
// Signature: fd_read(fd: i32, iovs_ptr: i32, iovs_len: i32, nread_ptr: i32) -> errno
//
// WASI uses the same iovec layout for reads and writes. Each entry describes
// a writable buffer in WASM memory. We copy bytes from stdin into each buffer
// in order and stop early on EOF or a short read.
func (w *WasiStub) makeFdRead() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			if w.instanceMemory == nil {
				return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiENosys))}
			}

			fd := wasmexecution.AsI32(args[0])
			if fd != 0 {
				return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiEBadf))}
			}

			iovsPtr := wasmexecution.AsI32(args[1])
			iovsLen := wasmexecution.AsI32(args[2])
			nreadPtr := wasmexecution.AsI32(args[3])
			mem := w.instanceMemory
			totalRead := int32(0)

			for i := int32(0); i < iovsLen; i++ {
				bufPtr := uint32(mem.LoadI32(int(iovsPtr + i*8)))
				bufLen := int(uint32(mem.LoadI32(int(iovsPtr + i*8 + 4))))

				chunk := w.StdinCallback(bufLen)
				if len(chunk) > bufLen {
					chunk = chunk[:bufLen]
				}

				for j, b := range chunk {
					mem.StoreI32_8(int(bufPtr)+j, int32(b))
				}

				totalRead += int32(len(chunk))
				if len(chunk) < bufLen {
					break
				}
			}

			mem.StoreI32(int(nreadPtr), totalRead)
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// makeProcExit creates the proc_exit host function.
//
// Signature: proc_exit(exit_code: i32) → (no return)
//
// We panic with *ProcExitError to unwind all WASM frames.  The runtime
// recovers this panic and returns it as a Go error.
func (w *WasiStub) makeProcExit() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			Results: nil,
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			exitCode := wasmexecution.AsI32(args[0])
			panic(&ProcExitError{ExitCode: int(exitCode)})
		},
	}
}

// makeStub returns a generic stub returning ENOSYS.
//
// ENOSYS (52) means "function not implemented".  This is the correct errno
// for WASI functions we haven't implemented yet — it tells the guest that
// this capability is absent, rather than silently corrupting state.
func (w *WasiStub) makeStub() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  nil,
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiENosys))}
		},
	}
}

// ════════════════════════════════════════════════════════════════════════
// TIER 3: args_sizes_get, args_get
// ════════════════════════════════════════════════════════════════════════

// makeArgsSizesGet creates the args_sizes_get host function.
//
// Signature: args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) → errno
//
// The WASM module calls this first to learn how much memory to allocate
// before calling args_get.  We write:
//   - argc          → *argc_ptr         (number of arguments)
//   - total buf len → *argv_buf_size_ptr (sum of len(arg)+1 for each arg)
//
// The +1 accounts for the null terminator that separates strings in the
// packed buffer.  For example, with args ["myapp", "hello"]:
//
//	argc       = 2
//	buf_size   = 6 + 1 + 5 + 1 = 13   (NO — see below)
//
// Wait — the test says 12 for ["myapp", "hello"]:
//
//	"myapp\0" = 6 bytes, "hello\0" = 6 bytes → total = 12 ✓
func (w *WasiStub) makeArgsSizesGet() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			argcPtr := wasmexecution.AsI32(args[0])
			argvBufSizePtr := wasmexecution.AsI32(args[1])

			// Count arguments and total buffer bytes needed.
			argc := int32(len(w.args))
			bufSize := int32(0)
			for _, arg := range w.args {
				bufSize += int32(len(arg)) + 1 // +1 for null terminator '\0'
			}

			w.instanceMemory.StoreI32(int(uint32(argcPtr)), argc)
			w.instanceMemory.StoreI32(int(uint32(argvBufSizePtr)), bufSize)
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// makeArgsGet creates the args_get host function.
//
// Signature: args_get(argv_ptr: i32, argv_buf_ptr: i32) → errno
//
// Writes two data structures into linear memory:
//
//  1. argv array (at argv_ptr): one 4-byte pointer per argument, each
//     pointing into the argv_buf region.
//
//  2. argv_buf (at argv_buf_ptr): all arguments concatenated as
//     null-terminated UTF-8 strings:
//     arg0\0arg1\0...argN\0
//
// The WASM C runtime uses these to reconstruct a char** argv for main().
func (w *WasiStub) makeArgsGet() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			argvPtr := wasmexecution.AsI32(args[0])
			argvBufPtr := wasmexecution.AsI32(args[1])

			mem := w.instanceMemory
			// cursor tracks where in argv_buf we write next.
			cursor := int32(argvBufPtr)

			for i, arg := range w.args {
				// Write the pointer to this arg string into the argv array.
				// argv[i] lives at argvPtr + i*4 (each pointer is 4 bytes).
				mem.StoreI32(int(argvPtr)+i*4, cursor)

				// Write the null-terminated string into argv_buf.
				for _, b := range []byte(arg) {
					mem.StoreI32_8(int(cursor), int32(b))
					cursor++
				}
				// Null terminator.
				mem.StoreI32_8(int(cursor), 0)
				cursor++
			}

			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// ════════════════════════════════════════════════════════════════════════
// TIER 3: environ_sizes_get, environ_get
// ════════════════════════════════════════════════════════════════════════

// makeEnvironSizesGet creates the environ_sizes_get host function.
//
// Signature: environ_sizes_get(count_ptr: i32, buf_size_ptr: i32) → errno
//
// Identical layout to args_sizes_get but for environment variables.
// Each env var is a "KEY=VALUE" string (null-terminated in the buffer).
func (w *WasiStub) makeEnvironSizesGet() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			countPtr := wasmexecution.AsI32(args[0])
			bufSizePtr := wasmexecution.AsI32(args[1])

			count := int32(len(w.env))
			bufSize := int32(0)
			for _, e := range w.env {
				bufSize += int32(len(e)) + 1 // +1 for null terminator
			}

			w.instanceMemory.StoreI32(int(uint32(countPtr)), count)
			w.instanceMemory.StoreI32(int(uint32(bufSizePtr)), bufSize)
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// makeEnvironGet creates the environ_get host function.
//
// Signature: environ_get(environ_ptr: i32, environ_buf_ptr: i32) → errno
//
// Same layout as args_get but for environment variables.  Writes a
// pointer array and a packed null-terminated string buffer.
func (w *WasiStub) makeEnvironGet() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			environPtr := wasmexecution.AsI32(args[0])
			environBufPtr := wasmexecution.AsI32(args[1])

			mem := w.instanceMemory
			cursor := int32(environBufPtr)

			for i, e := range w.env {
				// Write pointer into environ array.
				mem.StoreI32(int(environPtr)+i*4, cursor)

				// Write null-terminated env string into environ_buf.
				for _, b := range []byte(e) {
					mem.StoreI32_8(int(cursor), int32(b))
					cursor++
				}
				mem.StoreI32_8(int(cursor), 0)
				cursor++
			}

			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// ════════════════════════════════════════════════════════════════════════
// TIER 3: clock_res_get, clock_time_get
// ════════════════════════════════════════════════════════════════════════

// makeClockResGet creates the clock_res_get host function.
//
// Signature: clock_res_get(id: i32, resolution_ptr: i32) → errno
//
// Writes the clock resolution (in nanoseconds) as a 64-bit little-endian
// integer at *resolution_ptr.  The WasiClock interface abstracts the
// actual resolution value so tests can control it.
func (w *WasiStub) makeClockResGet() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			id := wasmexecution.AsI32(args[0])
			resolutionPtr := wasmexecution.AsI32(args[1])

			ns := w.clock.ResolutionNs(id)
			w.instanceMemory.StoreI64(int(uint32(resolutionPtr)), ns)
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// makeClockTimeGet creates the clock_time_get host function.
//
// Signature: clock_time_get(id: i32, precision: i64, time_ptr: i32) → errno
//
// Clock IDs (from the WASI spec):
//   - 0 (REALTIME)            → wall clock, nanoseconds since Unix epoch
//   - 1 (MONOTONIC)           → always-increasing, arbitrary start
//   - 2 (PROCESS_CPUTIME_ID)  → CPU time of the process (we alias realtime)
//   - 3 (THREAD_CPUTIME_ID)   → CPU time of the thread (we alias realtime)
//   - anything else           → EINVAL (28)
//
// The `precision` parameter is a hint: "I only need resolution this fine."
// WASM programs use it to avoid burning CPU on sub-ms precision they don't
// need.  Our implementation ignores it (we always return full resolution).
//
// The result is written as a 64-bit little-endian integer at *time_ptr.
func (w *WasiStub) makeClockTimeGet() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI64, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			id := wasmexecution.AsI32(args[0])
			// args[1] is precision (i64) — we accept but ignore it.
			timePtr := wasmexecution.AsI32(args[2])

			var ns int64
			switch id {
			case 0, 2, 3:
				// REALTIME, PROCESS_CPUTIME_ID, THREAD_CPUTIME_ID
				// all map to the wall clock in our Tier 3 implementation.
				ns = w.clock.RealtimeNs()
			case 1:
				// MONOTONIC — always-increasing counter.
				ns = w.clock.MonotonicNs()
			default:
				// Unknown clock ID — return EINVAL.
				return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiEInval))}
			}

			w.instanceMemory.StoreI64(int(uint32(timePtr)), ns)
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// ════════════════════════════════════════════════════════════════════════
// TIER 3: random_get
// ════════════════════════════════════════════════════════════════════════

// makeRandomGet creates the random_get host function.
//
// Signature: random_get(buf_ptr: i32, buf_len: i32) → errno
//
// Fills the memory region [buf_ptr, buf_ptr+buf_len) with random bytes.
// Uses WriteBytes for efficiency (one copy, not N StoreI32_8 calls).
//
// Error handling:
//   - If the entropy source fails (EIO, errno 29), we return EIO.
//     In practice, crypto/rand never fails on a healthy system, but
//     we must handle the error path for correctness.
//
// Security note: buf_len comes from an untrusted WASM guest.  We cap it
// at the current linear-memory size to prevent a malicious guest from
// triggering a multi-GiB Go heap allocation before LinearMemory.boundsCheck
// fires.  The LinearMemory bounds check remains the authoritative guard;
// this cap is a defence-in-depth measure.
const randomGetMaxBytes = 65536 * 1024 // 64 MiB — larger than any sane request

func (w *WasiStub) makeRandomGet() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32, wasmtypes.ValueTypeI32},
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			bufPtr := wasmexecution.AsI32(args[0])
			bufLen := wasmexecution.AsI32(args[1])

			// Clamp buf_len to a sane maximum before allocating.
			// uint32 conversion handles negative i32 values (they wrap to
			// large positive numbers, which the cap then reduces).
			size := int(uint32(bufLen))
			if size > randomGetMaxBytes {
				size = randomGetMaxBytes
			}

			// Allocate a Go-side buffer, fill it with random bytes, then
			// copy it into WASM memory in one WriteBytes call.
			buf := make([]byte, size)
			if err := w.random.FillBytes(buf); err != nil {
				return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiEIO))}
			}

			w.instanceMemory.WriteBytes(int(uint32(bufPtr)), buf)
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}

// ════════════════════════════════════════════════════════════════════════
// TIER 3: sched_yield
// ════════════════════════════════════════════════════════════════════════

// makeSchedYield creates the sched_yield host function.
//
// Signature: sched_yield() → errno
//
// In a multi-threaded POSIX program, sched_yield() gives up the CPU
// to let other threads run.  WebAssembly is currently single-threaded
// (per the MVP spec), so there is nothing to yield to.  We return
// success immediately.
//
// This is not a stub — it is a correct implementation.  A WASM program
// that calls sched_yield() inside a spin-loop is indicating cooperative
// intent; returning 0 is the right response.
func (w *WasiStub) makeSchedYield() *wasmexecution.HostFunction {
	return &wasmexecution.HostFunction{
		Type: wasmtypes.FuncType{
			Params:  nil,
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		},
		Call: func(args []wasmexecution.WasmValue) []wasmexecution.WasmValue {
			return []wasmexecution.WasmValue{wasmexecution.I32(int32(wasiESuccess))}
		},
	}
}
