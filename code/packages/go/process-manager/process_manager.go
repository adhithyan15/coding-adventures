package processmanager

// =============================================================================
// ProcessManager — Core Process Lifecycle Management
// =============================================================================
//
// This file implements the four fundamental Unix process operations:
//
//   Fork()  — Clone a running process.
//   Exec()  — Replace a process's program with a new one.
//   Wait()  — Wait for a child process to exit and retrieve its exit code.
//   Kill()  — Send a signal to a process.
//
// Together with ExitProcess(), these operations form the complete process
// lifecycle. Every program you have ever run on a Unix system was created
// by fork+exec and cleaned up by exit+wait.
//
// How Your Shell Works
//
// When you type "ls" in a terminal:
//
//  1. The shell calls Fork(). Now there are TWO copies of the shell.
//  2. The child calls Exec("ls"). The child is now the "ls" program.
//  3. "ls" runs, prints files, and calls exit(0). Child becomes a zombie.
//  4. The parent calls Wait(). Gets the child's exit code (0 = success).
//  5. The zombie is reaped (PCB removed). Shell prints the next prompt.

// ProcessManager manages the complete process lifecycle.
//
// It owns the process table (a map from PID to PCB) and provides the system
// calls that user programs invoke to create, manage, and terminate processes.
type ProcessManager struct {
	processes     map[int]*ProcessControlBlock
	nextPID       int
	signalManager *SignalManager
}

// NewProcessManager creates a new ProcessManager with an empty process table.
func NewProcessManager() *ProcessManager {
	result, _ := StartNew[*ProcessManager]("process-manager.NewProcessManager", nil,
		func(op *Operation[*ProcessManager], rf *ResultFactory[*ProcessManager]) *OperationResult[*ProcessManager] {
			return rf.Generate(true, false, &ProcessManager{
				processes:     make(map[int]*ProcessControlBlock),
				nextPID:       0,
				signalManager: NewSignalManager(),
			})
		}).GetResult()
	return result
}

// =============================================================================
// Process Creation
// =============================================================================

// CreateProcess creates a new process with a unique PID.
//
// This is the low-level process creation mechanism. It allocates a fresh PCB,
// assigns a unique PID, and adds it to the process table. Higher-level
// operations like Fork() use this internally.
//
// Parameters:
//   - name: Human-readable name for debugging (e.g., "shell", "ls").
//   - parentPID: PID of the parent process. -1 for the root process.
//   - priority: Scheduling priority, 0 (highest) to 39 (lowest).
//   - memoryBase: Starting address of the process's memory region.
//   - memorySize: Size of the process's memory region in bytes.
func (pm *ProcessManager) CreateProcess(name string, parentPID int, priority int, memoryBase int, memorySize int) *ProcessControlBlock {
	result, _ := StartNew[*ProcessControlBlock]("process-manager.ProcessManager.CreateProcess", nil,
		func(op *Operation[*ProcessControlBlock], rf *ResultFactory[*ProcessControlBlock]) *OperationResult[*ProcessControlBlock] {
			pid := pm.nextPID
			pm.nextPID++

			pcb := NewPCB(pid, name)
			pcb.ParentPID = parentPID
			pcb.Priority = priority
			pcb.MemoryBase = memoryBase
			pcb.MemorySize = memorySize

			pm.processes[pid] = pcb

			// If this process has a parent, add it to the parent's children list.
			if parentPID >= 0 {
				if parent, ok := pm.processes[parentPID]; ok {
					parent.Children = append(parent.Children, pid)
				}
			}

			return rf.Generate(true, false, pcb)
		}).GetResult()
	return result
}

// =============================================================================
// Fork — Clone a Process
// =============================================================================
//
// Fork() is the most unusual system call in computing. It creates a new
// process that is an EXACT COPY of the calling process. Both processes
// resume at the same point in the code, but they receive different return
// values:
//
//   - The parent receives the child's PID (a positive integer).
//   - The child receives 0.
//
// This is how the program knows which copy it is:
//
//   pid := fork()
//   if pid == 0 {
//       fmt.Println("I am the child!")
//   } else {
//       fmt.Printf("I am the parent. My child is PID %d.\n", pid)
//   }

// Fork creates a child process as a copy of the parent.
//
// The child gets:
//   - A new, unique PID.
//   - A copy of the parent's registers (including PC).
//   - The same priority as the parent.
//   - The same signal handlers.
//   - An empty children list and no pending signals.
//   - CPUTime reset to 0.
//
// Returns (childPID, 0, true) on success.
// Returns (0, 0, false) if parentPID does not exist.
//
// In a real kernel, fork() returns childPID to the parent and 0 to the
// child by writing different values into their respective a0 registers.
// Here we return both values so the caller can simulate this behavior.
// forkResult is an internal helper struct for returning multiple values from Fork.
type forkResult struct {
	childPID    int
	childReturn int
	ok          bool
}

func (pm *ProcessManager) Fork(parentPID int) (childPID int, childReturn int, ok bool) {
	res, _ := StartNew[forkResult]("process-manager.ProcessManager.Fork", forkResult{},
		func(op *Operation[forkResult], rf *ResultFactory[forkResult]) *OperationResult[forkResult] {
			parent, exists := pm.processes[parentPID]
			if !exists {
				return rf.Generate(true, false, forkResult{0, 0, false})
			}

			// Allocate a new PID.
			cPID := pm.nextPID
			pm.nextPID++

			// Create the child PCB as a copy of the parent.
			child := NewPCB(cPID, parent.Name)
			child.State = Ready
			child.ParentPID = parentPID
			child.Priority = parent.Priority
			child.MemoryBase = parent.MemoryBase
			child.MemorySize = parent.MemorySize

			// Copy registers — deep copy so modifications are independent.
			child.Registers = parent.Registers // Array copy (value semantics in Go).

			// Copy PC and SP.
			child.PC = parent.PC
			child.SP = parent.SP

			// Copy signal handlers (deep copy the map).
			for sig, handler := range parent.SignalHandlers {
				child.SignalHandlers[sig] = handler
			}

			// Copy signal mask (deep copy the map).
			for sig, blocked := range parent.SignalMask {
				child.SignalMask[sig] = blocked
			}

			// Child starts with: no children, no pending signals, CPUTime = 0.
			// These are already set by NewPCB.

			// Add child to process table.
			pm.processes[cPID] = child

			// Update parent's children list.
			parent.Children = append(parent.Children, cPID)

			return rf.Generate(true, false, forkResult{cPID, 0, true})
		}).GetResult()
	return res.childPID, res.childReturn, res.ok
}

// =============================================================================
// Exec — Replace Process Image
// =============================================================================
//
// Exec() throws away the current program and loads a new one. It is like
// erasing a whiteboard and drawing something completely different. The
// person holding the whiteboard (the PID) is the same, but the content
// is entirely new.
//
// What changes: registers (zeroed), PC (entry point), signal handlers (cleared).
// What stays: PID, ParentPID, Children, Priority, CPUTime.

// Exec replaces a process's program with a new one.
//
// Registers are zeroed, PC is set to entryPoint, SP is set to stackPointer,
// signal handlers and pending signals are cleared. PID, parent, children,
// and priority are preserved.
//
// Returns true on success, false if the PID does not exist.
func (pm *ProcessManager) Exec(pid int, entryPoint int, stackPointer int, memoryBase int, memorySize int) bool {
	result, _ := StartNew[bool]("process-manager.ProcessManager.Exec", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			pcb, exists := pm.processes[pid]
			if !exists {
				return rf.Generate(true, false, false)
			}

			// Reset all registers to zero — fresh start for the new program.
			pcb.Registers = [NumRegisters]int{}

			// Set the program counter to the new entry point.
			pcb.PC = entryPoint

			// Set the stack pointer.
			pcb.SP = stackPointer

			// Update memory region if new values provided.
			if memoryBase != 0 {
				pcb.MemoryBase = memoryBase
			}
			if memorySize != 0 {
				pcb.MemorySize = memorySize
			}

			// Clear signal handlers — the new program has no knowledge of the
			// old program's signal handling setup.
			pcb.SignalHandlers = make(map[int]int)

			// Clear pending signals.
			pcb.PendingSignals = make([]int, 0)

			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// =============================================================================
// Wait — Wait for a Child to Exit
// =============================================================================
//
// Wait() is how a parent process collects the exit status of its children.
// Without Wait(), terminated children become "zombies" — dead but their
// PCBs remain in the process table.

// Wait waits for a child to terminate and retrieves its exit code.
//
// If childPID is -1, waits for ANY child that is a zombie.
// If childPID is a specific PID, waits only for that child.
//
// Returns (reapedPID, exitCode, true) if a zombie child was found and reaped.
// Returns (0, 0, false) if no zombie children are available or parent doesn't
// exist.
// waitResult is an internal helper struct for returning multiple values from Wait.
type waitResult struct {
	reapedPID int
	exitCode  int
	ok        bool
}

func (pm *ProcessManager) Wait(parentPID int, childPID int) (reapedPID int, exitCode int, ok bool) {
	res, _ := StartNew[waitResult]("process-manager.ProcessManager.Wait", waitResult{},
		func(op *Operation[waitResult], rf *ResultFactory[waitResult]) *OperationResult[waitResult] {
			parent, exists := pm.processes[parentPID]
			if !exists {
				return rf.Generate(true, false, waitResult{0, 0, false})
			}

			// Search the parent's children for a zombie.
			for i, cPID := range parent.Children {
				// If waiting for a specific child, skip others.
				if childPID >= 0 && cPID != childPID {
					continue
				}

				child, childExists := pm.processes[cPID]
				if !childExists {
					continue
				}

				if child.State == Zombie {
					// Found a zombie child — reap it!
					code := child.ExitCode

					// Remove child from parent's children list.
					parent.Children = append(parent.Children[:i], parent.Children[i+1:]...)

					// Remove child from process table entirely.
					delete(pm.processes, cPID)

					return rf.Generate(true, false, waitResult{cPID, code, true})
				}
			}

			// No zombie children found.
			return rf.Generate(true, false, waitResult{0, 0, false})
		}).GetResult()
	return res.reapedPID, res.exitCode, res.ok
}

// =============================================================================
// Kill — Send a Signal to a Process
// =============================================================================
//
// Despite its name, Kill() does not necessarily kill a process. It sends
// a signal, which the process may catch, ignore, or be terminated by.

// Kill sends a signal to a process.
//
// Returns true if the signal was delivered, false if the PID does not exist.
func (pm *ProcessManager) Kill(pid int, signal int) bool {
	result, _ := StartNew[bool]("process-manager.ProcessManager.Kill", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			process, exists := pm.processes[pid]
			if !exists {
				return rf.Generate(true, false, false)
			}
			return rf.Generate(true, false, pm.signalManager.SendSignal(process, signal))
		}).GetResult()
	return result
}

// =============================================================================
// ExitProcess — Terminate a Process
// =============================================================================
//
// When a process exits:
//  1. State is set to Zombie.
//  2. Exit code is recorded.
//  3. Children are reparented to PID 0 (init).
//  4. SIGCHLD is sent to the parent.

// ExitProcess terminates a process.
//
// Sets state to Zombie, records the exit code, reparents children to PID 0,
// and sends SIGCHLD to the parent.
func (pm *ProcessManager) ExitProcess(pid int, exitCode int) {
	_, _ = StartNew[struct{}]("process-manager.ProcessManager.ExitProcess", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			pcb, exists := pm.processes[pid]
			if !exists {
				return rf.Generate(true, false, struct{}{})
			}

			pcb.State = Zombie
			pcb.ExitCode = exitCode

			// Reparent children to PID 0 (init).
			initProcess, initExists := pm.processes[0]
			for _, childPID := range pcb.Children {
				child, childExists := pm.processes[childPID]
				if childExists {
					child.ParentPID = 0
					if initExists && pid != 0 {
						initProcess.Children = append(initProcess.Children, childPID)
					}
				}
			}
			pcb.Children = make([]int, 0)

			// Send SIGCHLD to the parent.
			if pcb.ParentPID >= 0 {
				if parent, parentExists := pm.processes[pcb.ParentPID]; parentExists {
					pm.signalManager.SendSignal(parent, SIGCHLD)
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =============================================================================
// Query Methods
// =============================================================================

// GetProcess returns the PCB for the given PID, or nil if not found.
func (pm *ProcessManager) GetProcess(pid int) *ProcessControlBlock {
	result, _ := StartNew[*ProcessControlBlock]("process-manager.ProcessManager.GetProcess", nil,
		func(op *Operation[*ProcessControlBlock], rf *ResultFactory[*ProcessControlBlock]) *OperationResult[*ProcessControlBlock] {
			return rf.Generate(true, false, pm.processes[pid])
		}).GetResult()
	return result
}

// GetChildren returns the PIDs of all children of a process.
func (pm *ProcessManager) GetChildren(pid int) []int {
	result, _ := StartNew[[]int]("process-manager.ProcessManager.GetChildren", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			pcb, exists := pm.processes[pid]
			if !exists {
				return rf.Generate(true, false, nil)
			}
			children := make([]int, len(pcb.Children))
			copy(children, pcb.Children)
			return rf.Generate(true, false, children)
		}).GetResult()
	return result
}

// GetParent returns the parent PID of a process, or -1 if not found.
func (pm *ProcessManager) GetParent(pid int) int {
	result, _ := StartNew[int]("process-manager.ProcessManager.GetParent", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			pcb, exists := pm.processes[pid]
			if !exists {
				return rf.Generate(true, false, -1)
			}
			return rf.Generate(true, false, pcb.ParentPID)
		}).GetResult()
	return result
}

// SignalManager returns the process manager's signal manager.
func (pm *ProcessManager) SignalManager() *SignalManager {
	result, _ := StartNew[*SignalManager]("process-manager.ProcessManager.SignalManager", nil,
		func(op *Operation[*SignalManager], rf *ResultFactory[*SignalManager]) *OperationResult[*SignalManager] {
			return rf.Generate(true, false, pm.signalManager)
		}).GetResult()
	return result
}

// ProcessCount returns the number of processes in the process table.
func (pm *ProcessManager) ProcessCount() int {
	result, _ := StartNew[int]("process-manager.ProcessManager.ProcessCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(pm.processes))
		}).GetResult()
	return result
}
