// Package hazarddetection detects pipeline hazards in a classic 5-stage CPU.
//
// === Why These Types Exist ===
//
// A CPU pipeline is like an assembly line: each stage works on a different
// instruction simultaneously. But sometimes instructions interfere with each
// other -- one instruction needs a result that another hasn't produced yet,
// or two instructions fight over the same hardware resource.
//
// The hazard detection unit needs to know what each pipeline stage is doing
// WITHOUT knowing the specifics of the instruction set. It only needs:
//
//  1. Which registers does this instruction READ?
//  2. Which register does it WRITE?
//  3. Is it a branch? Was it predicted correctly?
//  4. What hardware resources does it need (ALU, FP unit, memory)?
//
// === The Pipeline Stages (5-Stage Classic) ===
//
//	IF -> ID -> EX -> MEM -> WB
package hazarddetection

// HazardAction represents the action the hazard unit tells the pipeline to take.
//
// Think of these as traffic signals for the pipeline:
//   - NONE: Green light -- pipeline flows normally
//   - FORWARD_FROM_EX: Shortcut -- grab value from EX stage
//   - FORWARD_FROM_MEM: Shortcut -- grab value from MEM stage
//   - STALL: Red light -- pipeline must freeze (load-use hazard)
//   - FLUSH: Emergency stop -- branch misprediction, discard wrong instructions
type HazardAction int

const (
	ActionNone          HazardAction = iota // Green light
	ActionForwardFromMEM                     // Forward from MEM stage
	ActionForwardFromEX                      // Forward from EX stage
	ActionStall                              // Must stall (load-use hazard)
	ActionFlush                              // Must flush (branch misprediction)
)

// Priority returns the numeric priority of the action (higher = more severe).
func (a HazardAction) Priority() int {
	return int(a)
}

// String returns a human-readable name for the action.
func (a HazardAction) String() string {
	switch a {
	case ActionNone:
		return "NONE"
	case ActionForwardFromMEM:
		return "FORWARD_FROM_MEM"
	case ActionForwardFromEX:
		return "FORWARD_FROM_EX"
	case ActionStall:
		return "STALL"
	case ActionFlush:
		return "FLUSH"
	default:
		return "UNKNOWN"
	}
}

// PipelineSlot represents information about an instruction occupying a pipeline stage.
//
// This is ISA-independent -- whatever decoder is plugged in extracts this
// info from raw instruction bits. The hazard unit only cares about register
// numbers and resource usage, not opcodes.
//
// === Example: Encoding "ADD R1, R2, R3" ===
//
//	PipelineSlot{
//	    Valid: true, PC: 0x1000,
//	    SourceRegs: []int{2, 3},  // reads R2 and R3
//	    DestReg: intPtr(1),       // writes R1
//	    UsesALU: true,
//	}
type PipelineSlot struct {
	Valid                bool
	PC                   int
	SourceRegs           []int
	DestReg              *int // nil means no destination register
	DestValue            *int // nil means value not yet available
	IsBranch             bool
	BranchTaken          bool
	BranchPredictedTaken bool
	MemRead              bool
	MemWrite             bool
	UsesALU              bool
	UsesFP               bool
}

// HazardResult is the complete verdict from hazard detection.
//
// It includes what action to take, the forwarded value (if any),
// stall/flush counts, and a human-readable reason for debugging.
type HazardResult struct {
	Action         HazardAction
	ForwardedValue *int
	ForwardedFrom  string
	StallCycles    int
	FlushCount     int
	Reason         string
}

// IntPtr is a helper to create a pointer to an int. Useful for setting
// DestReg and DestValue fields on PipelineSlot, since Go doesn't allow
// taking the address of a literal.
func IntPtr(v int) *int {
	return &v
}
