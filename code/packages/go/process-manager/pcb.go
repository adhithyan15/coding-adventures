// Package processmanager implements advanced process management for the
// coding-adventures operating system stack.
//
// Every running program is represented in the kernel by a Process Control
// Block (PCB). Think of it as a "save file" for a process — when the kernel
// needs to switch from one process to another, it saves the current process's
// state into its PCB and loads the next process's state from its PCB. This
// is called a "context switch."
//
// This package extends the basic OS kernel's process model with:
//   - Parent/child relationships (fork creates a parent-child link)
//   - Signal delivery (SIGTERM, SIGKILL, SIGSTOP, etc.)
//   - Priority scheduling (0=highest priority, 39=lowest)
//   - Full process lifecycle: fork, exec, wait, kill, exit
//
// # State Machine
//
// A process moves through these states during its lifetime:
//
//	READY -------> RUNNING -------> BLOCKED
//	  ^              |                 |
//	  |              |                 |
//	  +--------------+---------<-------+
//	  |              |
//	  |              v
//	  |           ZOMBIE  --------> (reaped/removed)
//
//	READY:       Waiting for CPU time.
//	RUNNING:     Currently executing on the CPU.
//	BLOCKED:     Waiting for I/O or a signal (e.g., SIGSTOP).
//	TERMINATED:  Finished execution. All resources freed.
//	ZOMBIE:      Terminated but parent hasn't called Wait() yet.
package processmanager

// =============================================================================
// ProcessState — The Five States of a Process
// =============================================================================
//
// We use typed int constants so that process states are type-safe and have
// meaningful names in debug output.

// ProcessState represents the current state of a process in its lifecycle.
type ProcessState int

const (
	// Ready means the process is waiting for CPU time. The scheduler will
	// pick it when its priority comes up.
	Ready ProcessState = iota

	// Running means the process is currently executing on the CPU. Only one
	// process can be Running at a time on a single-core system.
	Running

	// Blocked means the process is waiting for an event (I/O, signal, etc.)
	// and cannot run until the event occurs. SIGSTOP puts a process here.
	Blocked

	// Terminated means the process has fully exited and all resources are
	// freed. This is the final state — the PCB can be deleted.
	Terminated

	// Zombie means the process has exited but its parent hasn't called
	// Wait() yet. The PCB is kept around so the parent can retrieve the
	// exit status. The process is dead (no code running, no memory), but
	// its PID and exit code are preserved.
	Zombie
)

// NumRegisters is the number of general-purpose registers in RISC-V.
// RISC-V has 32 registers: x0 (hardwired zero) through x31.
const NumRegisters = 32

// =============================================================================
// ProcessControlBlock — The Kernel's Record of a Process
// =============================================================================
//
// Every field in this struct corresponds to a piece of information the kernel
// needs to manage the process. When the CPU switches from process A to
// process B, the kernel:
//
//  1. Saves A's current registers and PC into A's PCB.
//  2. Loads B's saved registers and PC from B's PCB.
//  3. Jumps to B's saved PC — and B resumes exactly where it left off.

// ProcessControlBlock holds all kernel-managed state for a single process.
//
// Fields:
//
//   - PID: Unique process identifier, assigned sequentially starting from 0.
//
//   - Name: Human-readable name for debugging (e.g., "shell", "ls").
//
//   - State: Current state in the process lifecycle (Ready, Running, etc.).
//
//   - Registers: The 32 RISC-V general-purpose registers (x0-x31). When a
//     process is not running, its register values are saved here.
//
//   - PC: Program counter — the address of the next instruction to execute.
//
//   - SP: Stack pointer — points to the top of the process's stack.
//
//   - MemoryBase, MemorySize: The process's memory region.
//
//   - ParentPID: PID of the process that created this one via Fork().
//     The init process has ParentPID = -1 (no parent).
//
//   - Children: PIDs of all child processes. Updated by Fork() (add) and
//     Wait() (remove).
//
//   - PendingSignals: Signals sent to this process but not yet delivered.
//
//   - SignalHandlers: Map from signal number to handler address. If a signal
//     is not in this map, the default action is used.
//
//   - SignalMask: Set of signal numbers currently blocked from delivery.
//
//   - Priority: Scheduling priority, 0-39. Lower number = higher priority.
//
//   - CPUTime: Total CPU cycles consumed. Useful for profiling.
//
//   - ExitCode: Exit status. 0 = success, nonzero = error. Only meaningful
//     in Zombie state.
type ProcessControlBlock struct {
	PID  int
	Name string

	State ProcessState

	// Saved CPU state — these are restored on context switch.
	Registers [NumRegisters]int
	PC        int
	SP        int

	// Memory region boundaries.
	MemoryBase int
	MemorySize int

	// Process relationships.
	ParentPID int   // -1 means "no parent" (root process).
	Children  []int // PIDs of child processes.

	// Signal state.
	PendingSignals []int       // Signals waiting to be delivered.
	SignalHandlers map[int]int // Signal number -> handler address.
	SignalMask     map[int]bool // Signal number -> true if blocked.

	// Scheduling.
	Priority int // 0 (highest) to 39 (lowest). Default: 20.
	CPUTime  int // Total cycles consumed.

	// Exit info.
	ExitCode int
}

// NewPCB creates a new ProcessControlBlock with sensible defaults.
//
// The process starts in Ready state with all registers zeroed, priority 20
// (normal user process), and no parent (-1).
func NewPCB(pid int, name string) *ProcessControlBlock {
	return &ProcessControlBlock{
		PID:            pid,
		Name:           name,
		State:          Ready,
		ParentPID:      -1,
		Children:       make([]int, 0),
		PendingSignals: make([]int, 0),
		SignalHandlers: make(map[int]int),
		SignalMask:     make(map[int]bool),
		Priority:       DefaultPriority,
	}
}
