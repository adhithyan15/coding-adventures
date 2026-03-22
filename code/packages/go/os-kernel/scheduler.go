package oskernel

// =========================================================================
// Scheduler -- round-robin process scheduling
// =========================================================================
//
// The scheduler decides which process runs next. We use the simplest
// possible algorithm: round-robin. Each process gets an equal time slice
// (driven by timer interrupts), and processes take turns in order.
//
// When the timer fires:
//   1. Save current process registers to its PCB
//   2. Set current process state = Ready
//   3. Pick next Ready process (round-robin order)
//   4. Load next process registers from its PCB
//   5. Set next process state = Running
//   6. Return from interrupt -> CPU now runs the next process

// Scheduler manages the process table and selects the next process to run.
type Scheduler struct {
	// ProcessTable holds all process control blocks, indexed by PID.
	ProcessTable []*ProcessControlBlock

	// Current is the PID of the currently running process.
	Current int
}

// NewScheduler creates a scheduler with the given process table.
func NewScheduler(processTable []*ProcessControlBlock) *Scheduler {
	return &Scheduler{
		ProcessTable: processTable,
		Current:      0,
	}
}

// Schedule picks the next Ready process using round-robin.
// It searches from Current+1 forward (wrapping around) for the next
// Ready process. If only the idle process (PID 0) is Ready, returns 0.
//
// Returns the PID of the next process to run.
func (s *Scheduler) Schedule() int {
	n := len(s.ProcessTable)
	if n == 0 {
		return 0
	}

	// Search for the next Ready process starting after the current one.
	for i := 1; i <= n; i++ {
		idx := (s.Current + i) % n
		if s.ProcessTable[idx].State == ProcessReady {
			return idx
		}
	}

	// If no other process is Ready, check if current is still Ready.
	if s.Current < n && s.ProcessTable[s.Current].State == ProcessReady {
		return s.Current
	}

	// Fall back to idle process (PID 0).
	return 0
}

// ContextSwitch saves the CPU state to the outgoing process's PCB and
// prepares the incoming process. The actual register loading is done by
// the caller (SystemBoard) which has access to the CPU.
//
// This method updates the process states:
//   - Outgoing: Running -> Ready (unless Terminated)
//   - Incoming: Ready -> Running
func (s *Scheduler) ContextSwitch(from, to int) {
	if from >= 0 && from < len(s.ProcessTable) {
		if s.ProcessTable[from].State == ProcessRunning {
			s.ProcessTable[from].State = ProcessReady
		}
	}
	if to >= 0 && to < len(s.ProcessTable) {
		s.ProcessTable[to].State = ProcessRunning
	}
	s.Current = to
}
