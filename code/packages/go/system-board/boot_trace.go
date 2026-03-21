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
	switch p {
	case PhasePowerOn:
		return "PowerOn"
	case PhaseBIOS:
		return "BIOS"
	case PhaseBootloader:
		return "Bootloader"
	case PhaseKernelInit:
		return "KernelInit"
	case PhaseUserProgram:
		return "UserProgram"
	case PhaseIdle:
		return "Idle"
	default:
		return "Unknown"
	}
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
	t.Events = append(t.Events, BootEvent{
		Phase:       phase,
		Cycle:       cycle,
		Description: description,
	})
}

// Phases returns the distinct phases that occurred, in order.
func (t *BootTrace) Phases() []BootPhase {
	seen := make(map[BootPhase]bool)
	var phases []BootPhase
	for _, e := range t.Events {
		if !seen[e.Phase] {
			seen[e.Phase] = true
			phases = append(phases, e.Phase)
		}
	}
	return phases
}

// EventsInPhase returns all events belonging to the given phase.
func (t *BootTrace) EventsInPhase(phase BootPhase) []BootEvent {
	var result []BootEvent
	for _, e := range t.Events {
		if e.Phase == phase {
			result = append(result, e)
		}
	}
	return result
}

// TotalCycles returns the cycle count of the last event, or 0 if empty.
func (t *BootTrace) TotalCycles() int {
	if len(t.Events) == 0 {
		return 0
	}
	return t.Events[len(t.Events)-1].Cycle
}

// PhaseStartCycle returns the cycle at which the given phase began.
// Returns -1 if the phase was not found.
func (t *BootTrace) PhaseStartCycle(phase BootPhase) int {
	for _, e := range t.Events {
		if e.Phase == phase {
			return e.Cycle
		}
	}
	return -1
}
