// Package oskernel implements a minimal monolithic kernel for the simulated
// computer. It manages two processes (idle and hello-world), handles system
// calls, and drives a round-robin scheduler via timer interrupts.
//
// === Design Philosophy ===
//
// This kernel operates at the Go level -- it intercepts ecall traps and
// handles them in Go code rather than in RISC-V machine code. The hello-world
// program IS real RISC-V machine code, but the syscall handler, scheduler,
// and memory manager are Go functions. This is a pragmatic simplification
// that demonstrates the full concept without requiring a complete RISC-V
// kernel implementation.
//
// === The Two Processes ===
//
//	PID 0: Idle process -- infinite loop that calls sys_yield
//	PID 1: Hello-world -- prints "Hello World\n" via sys_write, then exits
package oskernel

// =========================================================================
// Process States
// =========================================================================
//
// Every process is in exactly one state at any time. The state machine:
//
//	(none) --CreateProcess--> Ready
//	Ready --Scheduled--> Running
//	Running --Timer tick / sys_yield--> Ready
//	Running --sys_exit--> Terminated
//
// The Blocked state exists for future I/O wait support but is unused
// in the hello-world demo.

// ProcessState represents the execution state of a process.
type ProcessState int

const (
	// ProcessReady means the process is waiting to be scheduled.
	ProcessReady ProcessState = iota

	// ProcessRunning means the process is currently executing on the CPU.
	ProcessRunning

	// ProcessBlocked means the process is waiting for I/O (future use).
	ProcessBlocked

	// ProcessTerminated means the process has finished execution.
	ProcessTerminated
)

// String returns a human-readable name for the process state.
func (s ProcessState) String() string {
	switch s {
	case ProcessReady:
		return "Ready"
	case ProcessRunning:
		return "Running"
	case ProcessBlocked:
		return "Blocked"
	case ProcessTerminated:
		return "Terminated"
	default:
		return "Unknown"
	}
}

// =========================================================================
// Process Control Block (PCB)
// =========================================================================
//
// Every process has a PCB -- a data structure that stores everything the
// kernel needs to know about it. When the kernel switches from one process
// to another (context switch), it saves the outgoing process's CPU registers
// into its PCB and loads the incoming process's registers from its PCB.

// ProcessControlBlock holds all state for a single process.
type ProcessControlBlock struct {
	// PID is the unique process identifier.
	PID int

	// State is the current execution state (Ready, Running, etc.).
	State ProcessState

	// SavedRegisters stores all 32 RISC-V registers for context switching.
	// When the process is preempted, its register values are saved here.
	// When it resumes, they are loaded back into the CPU.
	SavedRegisters [32]uint32

	// SavedPC is the program counter where this process should resume.
	SavedPC uint32

	// StackPointer is the top of this process's stack.
	StackPointer uint32

	// MemoryBase is the start address of this process's memory region.
	MemoryBase uint32

	// MemorySize is the size of this process's memory region in bytes.
	MemorySize uint32

	// Name is a human-readable identifier (e.g., "idle", "hello-world").
	Name string

	// ExitCode is set by sys_exit when the process terminates.
	ExitCode int
}

// ProcessInfo is a lightweight summary of a process for snapshots and debugging.
type ProcessInfo struct {
	PID   int
	Name  string
	State ProcessState
	PC    uint32
}
