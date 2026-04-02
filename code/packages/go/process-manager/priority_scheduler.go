package processmanager

// =============================================================================
// Priority Scheduler — Priority-Based Process Scheduling
// =============================================================================
//
// The S04 OS kernel uses simple round-robin scheduling: all processes take
// turns equally. This is fair but inefficient — a keyboard handler (which
// needs to respond in milliseconds) gets the same treatment as a background
// log rotator (which can wait seconds).
//
// This module replaces round-robin with priority-based scheduling.
//
// How It Works:
//
// Every process has a priority from 0 (highest) to 39 (lowest). The scheduler
// maintains one queue per priority level. When choosing the next process, it
// starts at priority 0 and works down until it finds a non-empty queue.
// Within the same priority level, processes are scheduled round-robin (FIFO).
//
//   Priority 0:  [kernel_timer]
//   Priority 2:  [keyboard_handler]
//   Priority 20: [user_shell, user_editor, user_browser]
//   Priority 39: [background_backup_job]
//
// When Schedule() is called:
//  1. Check priority 0 — kernel_timer is there. Pick it.
//  2. Next call: priority 0 empty. Check 1, 2... keyboard_handler found.
//  3. And so on down the priority levels.
//
// Starvation Risk:
//
// If high-priority processes never finish, low-priority ones NEVER run.
// Real schedulers address this with "aging" — boosting starved processes.
// We note this but do not implement it here.
//
// Time Quantum:
//
// Higher-priority processes get a larger time quantum (more CPU cycles).
// Formula: quantum = BaseQuantum - (priority * QuantumPerPriority)
//
//   Priority  0: 200 cycles
//   Priority 20: 120 cycles
//   Priority 39:  44 cycles

// Scheduling constants.
const (
	// MinPriority is the highest scheduling priority (runs first).
	// This maps to Unix nice -20.
	MinPriority = 0

	// MaxPriority is the lowest scheduling priority (runs last).
	// This maps to Unix nice 19.
	MaxPriority = 39

	// DefaultPriority is the default priority for user processes.
	// This maps to Unix nice 0.
	DefaultPriority = 20

	// BaseQuantum is the base time quantum in CPU cycles.
	BaseQuantum = 200

	// QuantumPerPriority is how many cycles to subtract per priority level.
	QuantumPerPriority = 4
)

// PriorityScheduler implements priority-based scheduling with round-robin
// within the same priority level.
//
// It maintains a set of ready queues (one per active priority level) and
// a mapping from PID to priority for O(1) lookups.
type PriorityScheduler struct {
	// readyQueues maps priority level to a slice of PIDs (used as a FIFO queue).
	// Only priority levels that have been used are created (lazy initialization).
	readyQueues map[int][]int

	// currentPID is the PID of the currently running process, or -1 if none.
	currentPID int

	// pidPriority maps PID to its current priority for O(1) lookup.
	pidPriority map[int]int
}

// NewPriorityScheduler creates a new, empty priority scheduler.
func NewPriorityScheduler() *PriorityScheduler {
	result, _ := StartNew[*PriorityScheduler]("process-manager.NewPriorityScheduler", nil,
		func(op *Operation[*PriorityScheduler], rf *ResultFactory[*PriorityScheduler]) *OperationResult[*PriorityScheduler] {
			return rf.Generate(true, false, &PriorityScheduler{
				readyQueues: make(map[int][]int),
				currentPID:  -1,
				pidPriority: make(map[int]int),
			})
		}).GetResult()
	return result
}

// AddProcess adds a process to the appropriate ready queue.
//
// The process is placed at the END of its priority queue (FIFO ordering).
// Priority is clamped to the valid range [0, 39].
func (ps *PriorityScheduler) AddProcess(pid int, priority int) {
	_, _ = StartNew[struct{}]("process-manager.PriorityScheduler.AddProcess", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// Clamp priority to valid range.
			if priority < MinPriority {
				priority = MinPriority
			}
			if priority > MaxPriority {
				priority = MaxPriority
			}

			ps.readyQueues[priority] = append(ps.readyQueues[priority], pid)
			ps.pidPriority[pid] = priority
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RemoveProcess removes a process from the scheduler.
//
// Called when a process exits, blocks, or is otherwise no longer ready.
// If the process is the current process, currentPID is reset to -1.
func (ps *PriorityScheduler) RemoveProcess(pid int) {
	_, _ = StartNew[struct{}]("process-manager.PriorityScheduler.RemoveProcess", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			priority, exists := ps.pidPriority[pid]
			if exists {
				// Remove from the queue.
				queue := ps.readyQueues[priority]
				for i, p := range queue {
					if p == pid {
						ps.readyQueues[priority] = append(queue[:i], queue[i+1:]...)
						break
					}
				}

				// Clean up empty queues.
				if len(ps.readyQueues[priority]) == 0 {
					delete(ps.readyQueues, priority)
				}

				delete(ps.pidPriority, pid)
			}

			if ps.currentPID == pid {
				ps.currentPID = -1
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Schedule selects the next process to run.
//
// Scans from priority 0 (highest) to 39 (lowest). Returns the first PID
// found (front of the highest-priority non-empty queue).
//
// The selected process is REMOVED from the queue. If it should continue
// running after its time quantum, the caller must re-add it with AddProcess.
//
// Returns (pid, true) on success, or (0, false) if all queues are empty.
// scheduleResult is an internal helper struct for returning multiple values from Schedule.
type scheduleResult struct {
	pid int
	ok  bool
}

func (ps *PriorityScheduler) Schedule() (int, bool) {
	res, _ := StartNew[scheduleResult]("process-manager.PriorityScheduler.Schedule", scheduleResult{},
		func(op *Operation[scheduleResult], rf *ResultFactory[scheduleResult]) *OperationResult[scheduleResult] {
			for priority := MinPriority; priority <= MaxPriority; priority++ {
				queue, exists := ps.readyQueues[priority]
				if !exists || len(queue) == 0 {
					continue
				}

				// Pop the front of the queue (FIFO).
				pid := queue[0]
				ps.readyQueues[priority] = queue[1:]

				// Clean up empty queues.
				if len(ps.readyQueues[priority]) == 0 {
					delete(ps.readyQueues, priority)
				}

				ps.currentPID = pid
				return rf.Generate(true, false, scheduleResult{pid, true})
			}

			// All queues empty.
			ps.currentPID = -1
			return rf.Generate(true, false, scheduleResult{0, false})
		}).GetResult()
	return res.pid, res.ok
}

// SetPriority changes a process's priority.
//
// The process is moved from its current queue to the new one. It is placed
// at the END of the new queue (like a fresh arrival).
func (ps *PriorityScheduler) SetPriority(pid int, priority int) {
	_, _ = StartNew[struct{}]("process-manager.PriorityScheduler.SetPriority", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// Clamp priority.
			if priority < MinPriority {
				priority = MinPriority
			}
			if priority > MaxPriority {
				priority = MaxPriority
			}

			oldPriority, exists := ps.pidPriority[pid]
			if !exists {
				// Unknown PID — just record the priority.
				ps.pidPriority[pid] = priority
				return rf.Generate(true, false, struct{}{})
			}

			if oldPriority == priority {
				return rf.Generate(true, false, struct{}{}) // No change needed.
			}

			// Remove from old queue.
			queue := ps.readyQueues[oldPriority]
			for i, p := range queue {
				if p == pid {
					ps.readyQueues[oldPriority] = append(queue[:i], queue[i+1:]...)
					break
				}
			}
			if len(ps.readyQueues[oldPriority]) == 0 {
				delete(ps.readyQueues, oldPriority)
			}

			// Add to new queue.
			ps.readyQueues[priority] = append(ps.readyQueues[priority], pid)
			ps.pidPriority[pid] = priority
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GetPriority returns the current priority of a process.
//
// Returns DefaultPriority (20) if the PID is not known.
func (ps *PriorityScheduler) GetPriority(pid int) int {
	result, _ := StartNew[int]("process-manager.PriorityScheduler.GetPriority", DefaultPriority,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if priority, exists := ps.pidPriority[pid]; exists {
				return rf.Generate(true, false, priority)
			}
			return rf.Generate(true, false, DefaultPriority)
		}).GetResult()
	return result
}

// CurrentPID returns the PID of the currently running process, or -1 if none.
func (ps *PriorityScheduler) CurrentPID() int {
	result, _ := StartNew[int]("process-manager.PriorityScheduler.CurrentPID", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, ps.currentPID)
		}).GetResult()
	return result
}

// GetTimeQuantum calculates the time quantum for a given priority level.
//
// Higher priority (lower number) gets a larger time quantum:
//
//	quantum = BaseQuantum - (priority * QuantumPerPriority)
//	Priority  0: 200 cycles
//	Priority 20: 120 cycles
//	Priority 39:  44 cycles
func (ps *PriorityScheduler) GetTimeQuantum(priority int) int {
	result, _ := StartNew[int]("process-manager.PriorityScheduler.GetTimeQuantum", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if priority < MinPriority {
				priority = MinPriority
			}
			if priority > MaxPriority {
				priority = MaxPriority
			}
			return rf.Generate(true, false, BaseQuantum-(priority*QuantumPerPriority))
		}).GetResult()
	return result
}

// IsEmpty returns true if the scheduler has no ready processes.
func (ps *PriorityScheduler) IsEmpty() bool {
	result, _ := StartNew[bool]("process-manager.PriorityScheduler.IsEmpty", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, len(ps.readyQueues) == 0)
		}).GetResult()
	return result
}
