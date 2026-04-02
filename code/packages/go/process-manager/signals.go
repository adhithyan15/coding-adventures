package processmanager

// =============================================================================
// Signal Constants — The Six Essential POSIX Signals
// =============================================================================
//
// Signals are software interrupts sent between processes. They are the Unix
// mechanism for inter-process communication and process control.
//
// Real-World Analogy:
//
// Imagine you are working at your desk (a running process):
//   - Your phone rings (SIGINT) — you can answer or ignore it.
//   - Your boss says "you're fired" (SIGKILL) — you MUST leave immediately.
//   - Someone says "please finish up" (SIGTERM) — you can choose when to stop.
//   - Your child tugs your sleeve (SIGCHLD) — your child needs attention.
//   - Someone says "freeze!" (SIGSTOP) — you must stop immediately.
//   - Someone says "continue" (SIGCONT) — you can resume.
//
// Signal Numbers (POSIX standard):
//
//	Signal   Number  Default Action  Can Catch?  Purpose
//	------   ------  --------------  ----------  -------
//	SIGINT      2    Terminate       Yes         Ctrl+C pressed
//	SIGKILL     9    Terminate       NO          Force kill (uncatchable)
//	SIGTERM    15    Terminate       Yes         Polite termination request
//	SIGCHLD    17    Ignore          Yes         Child status changed
//	SIGCONT    18    Continue        Yes*        Resume stopped process
//	SIGSTOP    19    Stop            NO          Force stop (uncatchable)

const (
	// SIGINT is sent when the user presses Ctrl+C. Default action: terminate.
	// Can be caught (e.g., to save work before exiting).
	SIGINT = 2

	// SIGKILL unconditionally terminates the process. It CANNOT be caught,
	// blocked, or ignored. This is the "nuclear option."
	SIGKILL = 9

	// SIGTERM is a polite request to exit. Default action: terminate.
	// Can be caught (for graceful shutdown). This is what `kill <pid>` sends.
	SIGTERM = 15

	// SIGCHLD is sent to the parent when a child exits, stops, or continues.
	// Default action: ignore. Shells catch this to know when background jobs
	// finish.
	SIGCHLD = 17

	// SIGCONT resumes a stopped process. Sent by `fg` in the shell.
	SIGCONT = 18

	// SIGSTOP suspends the process. It CANNOT be caught, blocked, or ignored
	// (like SIGKILL but for stopping instead of killing).
	SIGSTOP = 19
)

// isFatalByDefault returns true if the given signal terminates a process by
// default (when no custom handler is registered).
//
// Fatal signals: SIGINT, SIGKILL, SIGTERM.
// Non-fatal signals: SIGCHLD (ignored), SIGCONT (resumes), SIGSTOP (stops).
func isFatalByDefault(signal int) bool {
	return signal == SIGINT || signal == SIGKILL || signal == SIGTERM
}

// isUncatchable returns true if the given signal cannot be caught, blocked,
// or ignored. These are SIGKILL and SIGSTOP — the kernel's safety valves
// for controlling runaway processes.
func isUncatchable(signal int) bool {
	return signal == SIGKILL || signal == SIGSTOP
}

// =============================================================================
// SignalManager — Handles Signal Delivery and Processing
// =============================================================================
//
// The SignalManager is the kernel's signal subsystem. It is responsible for:
//   1. Accepting signals sent to a process (SendSignal).
//   2. Delivering pending signals when a process is scheduled (DeliverPending).
//   3. Managing custom signal handlers (RegisterHandler).
//   4. Managing the signal mask (MaskSignal, UnmaskSignal).

// SignalManager handles signal delivery, masking, and handler registration.
//
// The SignalManager processes signals according to POSIX semantics:
//   - SIGKILL and SIGSTOP are always delivered immediately and cannot be
//     caught, masked, or ignored.
//   - Masked signals remain in the pending queue until unmasked.
//   - Signals with custom handlers redirect execution to the handler.
//   - Signals without handlers use the default action (terminate or ignore).
type SignalManager struct{}

// NewSignalManager creates a new SignalManager.
func NewSignalManager() *SignalManager {
	result, _ := StartNew[*SignalManager]("process-manager.NewSignalManager", nil,
		func(op *Operation[*SignalManager], rf *ResultFactory[*SignalManager]) *OperationResult[*SignalManager] {
			return rf.Generate(true, false, &SignalManager{})
		}).GetResult()
	return result
}

// SendSignal sends a signal to a process.
//
// Special cases:
//   - SIGKILL: Immediately terminates (state -> Zombie). Cannot be blocked.
//   - SIGSTOP: Immediately stops (state -> Blocked). Cannot be blocked.
//   - SIGCONT: Resumes a blocked process (state -> Ready). Also queued for
//     handler delivery.
//
// For all other signals, the signal is added to PendingSignals and will be
// delivered when DeliverPending is called.
//
// Returns false if the process is Terminated (cannot receive signals).
func (sm *SignalManager) SendSignal(process *ProcessControlBlock, signal int) bool {
	result, _ := StartNew[bool]("process-manager.SignalManager.SendSignal", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			// A fully terminated process cannot receive signals.
			if process.State == Terminated {
				return rf.Generate(true, false, false)
			}

			// SIGKILL: unconditional termination. No handler. No negotiation.
			if signal == SIGKILL {
				process.State = Zombie
				return rf.Generate(true, false, true)
			}

			// SIGSTOP: unconditional stop. The process is suspended immediately.
			if signal == SIGSTOP {
				process.State = Blocked
				return rf.Generate(true, false, true)
			}

			// SIGCONT: resume a stopped process. Also queued for handler delivery.
			if signal == SIGCONT {
				if process.State == Blocked {
					process.State = Ready
				}
				process.PendingSignals = append(process.PendingSignals, signal)
				return rf.Generate(true, false, true)
			}

			// All other signals: add to the pending queue.
			process.PendingSignals = append(process.PendingSignals, signal)
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// DeliverPending delivers the next pending signal to a process.
//
// Called by the scheduler just before a process runs. It finds the first
// non-masked pending signal and processes it:
//
//   - If the process has a custom handler: returns (signal, handler_addr, true).
//     The caller should redirect the process's PC to the handler.
//   - If no handler and the signal is fatal: sets state to Zombie. Returns
//     (signal, 0, true).
//   - If no handler and non-fatal (e.g., SIGCHLD): silently discards.
//     Returns (0, 0, false).
//   - If no pending signals or all are masked: returns (0, 0, false).
// deliverResult is an internal helper struct for returning multiple values from DeliverPending.
type deliverResult struct {
	signal      int
	handlerAddr int
	delivered   bool
}

func (sm *SignalManager) DeliverPending(process *ProcessControlBlock) (signal int, handlerAddr int, delivered bool) {
	res, _ := StartNew[deliverResult]("process-manager.SignalManager.DeliverPending", deliverResult{},
		func(op *Operation[deliverResult], rf *ResultFactory[deliverResult]) *OperationResult[deliverResult] {
			for i, sig := range process.PendingSignals {
				// Skip masked signals — they stay pending.
				if process.SignalMask[sig] {
					continue
				}

				// Remove from pending list.
				process.PendingSignals = append(process.PendingSignals[:i], process.PendingSignals[i+1:]...)

				// Does the process have a custom handler?
				if addr, ok := process.SignalHandlers[sig]; ok {
					return rf.Generate(true, false, deliverResult{sig, addr, true})
				}

				// No handler — apply default action.
				if isFatalByDefault(sig) {
					process.State = Zombie
					return rf.Generate(true, false, deliverResult{sig, 0, true})
				}

				// Non-fatal, no handler (e.g., SIGCHLD): discard silently.
				return rf.Generate(true, false, deliverResult{0, 0, false})
			}

			// No deliverable signals.
			return rf.Generate(true, false, deliverResult{0, 0, false})
		}).GetResult()
	return res.signal, res.handlerAddr, res.delivered
}

// RegisterHandler registers a custom signal handler for a process.
//
// When the given signal is delivered, the process's PC will be redirected
// to handlerAddr. SIGKILL and SIGSTOP cannot have custom handlers — they
// are always handled by the kernel. Attempting to register a handler for
// them is silently ignored.
func (sm *SignalManager) RegisterHandler(process *ProcessControlBlock, signal int, handlerAddr int) {
	_, _ = StartNew[struct{}]("process-manager.SignalManager.RegisterHandler", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if isUncatchable(signal) {
				return rf.Generate(true, false, struct{}{})
			}
			process.SignalHandlers[signal] = handlerAddr
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MaskSignal blocks a signal from being delivered to a process.
//
// A masked signal stays in PendingSignals but is not delivered until
// UnmaskSignal is called. SIGKILL and SIGSTOP cannot be masked.
func (sm *SignalManager) MaskSignal(process *ProcessControlBlock, signal int) {
	_, _ = StartNew[struct{}]("process-manager.SignalManager.MaskSignal", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if isUncatchable(signal) {
				return rf.Generate(true, false, struct{}{})
			}
			process.SignalMask[signal] = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// UnmaskSignal unblocks a previously masked signal.
func (sm *SignalManager) UnmaskSignal(process *ProcessControlBlock, signal int) {
	_, _ = StartNew[struct{}]("process-manager.SignalManager.UnmaskSignal", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			delete(process.SignalMask, signal)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// IsFatal checks if a signal terminates the process by default.
//
// Returns true for SIGINT, SIGKILL, SIGTERM.
// Returns false for SIGCHLD, SIGCONT, SIGSTOP.
func (sm *SignalManager) IsFatal(signal int) bool {
	result, _ := StartNew[bool]("process-manager.SignalManager.IsFatal", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, isFatalByDefault(signal))
		}).GetResult()
	return result
}
