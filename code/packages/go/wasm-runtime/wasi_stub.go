// wasi_stub.go --- Minimal WASI implementation for WASM programs.
//
// WASI (WebAssembly System Interface) provides fd_write for stdout/stderr
// and proc_exit for clean termination.  Everything else returns ENOSYS.
package wasmruntime

import (
	"fmt"

	wasmexecution "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-execution"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// WASI error codes.
const (
	wasiESuccess = 0
	wasiENosys   = 52
)

// ProcExitError is thrown when a WASM program calls proc_exit.
type ProcExitError struct {
	ExitCode int
}

func (e *ProcExitError) Error() string {
	return fmt.Sprintf("proc_exit(%d)", e.ExitCode)
}

// WasiStub provides minimal WASI host functions.
type WasiStub struct {
	StdoutCallback func(text string)
	StderrCallback func(text string)
	instanceMemory *wasmexecution.LinearMemory
}

// NewWasiStub creates a new WASI stub.
func NewWasiStub(stdout, stderr func(string)) *WasiStub {
	if stdout == nil {
		stdout = func(string) {}
	}
	if stderr == nil {
		stderr = func(string) {}
	}
	return &WasiStub{StdoutCallback: stdout, StderrCallback: stderr}
}

// SetMemory sets the instance's memory (needed for fd_write).
func (w *WasiStub) SetMemory(mem *wasmexecution.LinearMemory) {
	w.instanceMemory = mem
}

// ResolveFunction resolves WASI functions.
func (w *WasiStub) ResolveFunction(moduleName, name string) *wasmexecution.HostFunction {
	if moduleName != "wasi_snapshot_preview1" {
		return nil
	}

	switch name {
	case "fd_write":
		return w.makeFdWrite()
	case "proc_exit":
		return w.makeProcExit()
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

// makeFdWrite creates the fd_write host function.
// fd_write(fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32) -> i32
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

// makeProcExit creates the proc_exit host function.
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
