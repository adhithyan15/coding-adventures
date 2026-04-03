package systemboard

// =========================================================================
// Boot Phases -- the stages of the boot sequence
// =========================================================================

// BootPhase represents a stage in the boot sequence.
type BootPhase int

const (
	// PhasePowerOn is when the system just powered on.
	PhasePowerOn BootPhase = iota

	// PhaseBIOS is when BIOS firmware is executing POST and IDT setup.
	PhaseBIOS

	// PhaseBootloader is when the bootloader is copying the kernel.
	PhaseBootloader

	// PhaseKernelInit is when the kernel is initializing subsystems.
	PhaseKernelInit

	// PhaseUserProgram is when user program(s) are running.
	PhaseUserProgram

	// PhaseIdle is when all user programs have terminated.
	PhaseIdle
)

// String returns a human-readable name for the boot phase.
func (p BootPhase) String() string {
	result, _ := StartNew[string]("system-board.BootPhase.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			switch p {
			case PhasePowerOn:
				return rf.Generate(true, false, "PowerOn")
			case PhaseBIOS:
				return rf.Generate(true, false, "BIOS")
			case PhaseBootloader:
				return rf.Generate(true, false, "Bootloader")
			case PhaseKernelInit:
				return rf.Generate(true, false, "KernelInit")
			case PhaseUserProgram:
				return rf.Generate(true, false, "UserProgram")
			case PhaseIdle:
				return rf.Generate(true, false, "Idle")
			default:
				return rf.Generate(true, false, "Unknown")
			}
		}).GetResult()
	return result
}

// =========================================================================
// Boot Events and Trace
// =========================================================================

// BootEvent records a notable event during the boot sequence.
type BootEvent struct {
	// Phase is which boot phase this event belongs to.
	Phase BootPhase

	// Cycle is the CPU cycle when this event occurred.
	Cycle int

	// Description is a human-readable explanation of what happened.
	Description string
}

// BootTrace records the complete boot sequence.
type BootTrace struct {
	Events []BootEvent
}

// AddEvent appends a new event to the trace.
func (t *BootTrace) AddEvent(phase BootPhase, cycle int, description string) {
	_, _ = StartNew[struct{}]("system-board.BootTrace.AddEvent", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("phase", phase)
			op.AddProperty("cycle", cycle)
			t.Events = append(t.Events, BootEvent{
				Phase:       phase,
				Cycle:       cycle,
				Description: description,
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Phases returns the distinct phases that occurred, in order.
func (t *BootTrace) Phases() []BootPhase {
	result, _ := StartNew[[]BootPhase]("system-board.BootTrace.Phases", nil,
		func(op *Operation[[]BootPhase], rf *ResultFactory[[]BootPhase]) *OperationResult[[]BootPhase] {
			seen := make(map[BootPhase]bool)
			var phases []BootPhase
			for _, e := range t.Events {
				if !seen[e.Phase] {
					seen[e.Phase] = true
					phases = append(phases, e.Phase)
				}
			}
			return rf.Generate(true, false, phases)
		}).GetResult()
	return result
}

// EventsInPhase returns all events belonging to the given phase.
func (t *BootTrace) EventsInPhase(phase BootPhase) []BootEvent {
	result, _ := StartNew[[]BootEvent]("system-board.BootTrace.EventsInPhase", nil,
		func(op *Operation[[]BootEvent], rf *ResultFactory[[]BootEvent]) *OperationResult[[]BootEvent] {
			op.AddProperty("phase", phase)
			var events []BootEvent
			for _, e := range t.Events {
				if e.Phase == phase {
					events = append(events, e)
				}
			}
			return rf.Generate(true, false, events)
		}).GetResult()
	return result
}

// TotalCycles returns the cycle count of the last event, or 0 if empty.
func (t *BootTrace) TotalCycles() int {
	result, _ := StartNew[int]("system-board.BootTrace.TotalCycles", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(t.Events) == 0 {
				return rf.Generate(true, false, 0)
			}
			return rf.Generate(true, false, t.Events[len(t.Events)-1].Cycle)
		}).GetResult()
	return result
}

// PhaseStartCycle returns the cycle at which the given phase began.
// Returns -1 if the phase was not found.
func (t *BootTrace) PhaseStartCycle(phase BootPhase) int {
	result, _ := StartNew[int]("system-board.BootTrace.PhaseStartCycle", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("phase", phase)
			for _, e := range t.Events {
				if e.Phase == phase {
					return rf.Generate(true, false, e.Cycle)
				}
			}
			return rf.Generate(true, false, -1)
		}).GetResult()
	return result
}
