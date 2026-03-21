// Package cpupipeline implements a configurable N-stage CPU instruction pipeline.
//
// # The Pipeline: a CPU's Assembly Line
//
// A CPU pipeline is the central execution engine of a processor core. Instead
// of completing one instruction fully before starting the next (like a
// single-cycle CPU), a pipelined CPU overlaps instruction execution -- while
// one instruction is being executed, the next is being decoded, and the one
// after that is being fetched.
//
// This is the same principle as a factory assembly line:
//
//	Single-cycle (no pipeline):
//	Instr 1: [IF][ID][EX][MEM][WB]
//	Instr 2:                       [IF][ID][EX][MEM][WB]
//	Instr 3:                                              [IF]...
//	Throughput: 1 instruction every 5 cycles
//
//	Pipelined:
//	Instr 1: [IF][ID][EX][MEM][WB]
//	Instr 2:     [IF][ID][EX][MEM][WB]
//	Instr 3:         [IF][ID][EX][MEM][WB]
//	Instr 4:             [IF][ID][EX][MEM][WB]
//	Throughput: 1 instruction every 1 cycle (after filling)
//
// # What This Package Does
//
// This package manages the FLOW of instructions through pipeline stages. It
// does NOT interpret instructions -- that is the ISA decoder's job. The
// pipeline moves "tokens" (representing instructions) through stages, handling:
//
//   - Normal advancement: tokens move one stage per clock cycle
//   - Stalls: freeze earlier stages and insert a "bubble" (NOP)
//   - Flushes: replace speculative instructions with bubbles
//   - Statistics: track IPC, stall cycles, flush cycles
//
// The actual work of each stage (fetching, decoding, executing, etc.) is
// performed by callback functions injected from the CPU core. This makes the
// pipeline ISA-independent -- the same pipeline can run ARM, RISC-V, x86, or
// any other instruction set.
//
// # The Classic 5-Stage Pipeline
//
//	Stage 1: IF  (Instruction Fetch)  -- read instruction from memory at PC
//	Stage 2: ID  (Instruction Decode) -- decode opcode, read registers
//	Stage 3: EX  (Execute)            -- ALU operation, branch resolution
//	Stage 4: MEM (Memory Access)      -- load/store data from/to memory
//	Stage 5: WB  (Write Back)         -- write result to register file
//
// Between each pair of stages sits a pipeline register that captures the
// output of one stage and feeds it as input to the next. All pipeline
// registers update simultaneously on the clock edge.
//
// # Configurable Depth
//
// The pipeline depth is configurable. The classic 5-stage pipeline is just
// one configuration. Modern CPUs use 8-20+ stages for higher clock frequencies.
// Deeper pipelines trade higher throughput for larger misprediction penalties:
//
//	Depth   Clock Speed   Misprediction Penalty   Sweet Spot?
//	5       1.0 GHz       2 cycles                Teaching
//	8       1.6 GHz       5 cycles                Efficiency cores
//	13      2.2 GHz       10 cycles               Modern performance
//	20      3.0 GHz       17 cycles               Intel Pentium 4 era
//	31      3.8 GHz       28 cycles               Too deep (Prescott)
//
// # Dependency Injection
//
// The pipeline uses callback functions instead of importing other packages
// directly. This keeps the pipeline decoupled from specific implementations
// of caches, hazard detectors, and branch predictors.
//
//	Pipeline callbacks:
//	  FetchFunc:     (pc) -> raw instruction bits
//	  DecodeFunc:    (raw, token) -> decoded token
//	  ExecuteFunc:   (token) -> token with ALU result
//	  MemoryFunc:    (token) -> token with memory data
//	  WritebackFunc: (token) -> void (writes register file)
//
//	Optional integration callbacks:
//	  HazardFunc:    (stages) -> stall/flush/forward signal
//	  ForwardFunc:   (token, source) -> token with forwarded value
//	  PredictFunc:   (pc) -> predicted next PC
package cpupipeline

import "fmt"

// =========================================================================
// PipelineStage -- definition of a single stage in the pipeline
// =========================================================================

// StageCategory classifies pipeline stages by their function.
//
// Every stage in a pipeline does one of these five jobs, regardless of
// how many stages the pipeline has. A 5-stage pipeline has one stage per
// category. A 13-stage pipeline might have 2 fetch stages, 2 decode
// stages, 3 execute stages, etc.
//
// This classification is used for:
//   - Determining which callback to invoke for each stage
//   - Knowing where to insert stall bubbles
//   - Knowing which stages to flush on a misprediction
type StageCategory int

const (
	// StageFetch -- stages that read instructions from the instruction cache.
	// In a 5-stage pipeline, this is the IF stage.
	// In deeper pipelines, this might be IF1 (TLB lookup) and IF2 (cache read).
	StageFetch StageCategory = iota

	// StageDecode -- stages that decode the instruction and read registers.
	// Extracts opcode, register numbers, immediate values from raw bits.
	StageDecode

	// StageExecute -- stages that perform computation (ALU, branch resolution).
	// Some pipelines split this into EX1 (ALU), EX2 (shift/multiply), EX3 (result).
	StageExecute

	// StageMemory -- stages that access data memory (loads and stores).
	// Some pipelines have separate stages for address calculation and data access.
	StageMemory

	// StageWriteback -- stages that write results back to the register file.
	// This is always the final stage -- the instruction is "retired" here.
	StageWriteback
)

// String returns a human-readable name for the stage category.
func (c StageCategory) String() string {
	switch c {
	case StageFetch:
		return "fetch"
	case StageDecode:
		return "decode"
	case StageExecute:
		return "execute"
	case StageMemory:
		return "memory"
	case StageWriteback:
		return "writeback"
	default:
		return "unknown"
	}
}

// PipelineStage defines a single stage in the pipeline.
//
// A stage has a short name (used in diagrams), a description (for humans),
// and a category (for the pipeline to know what callback to invoke).
//
// Example stages:
//
//	PipelineStage{Name: "IF",  Description: "Instruction Fetch", Category: StageFetch}
//	PipelineStage{Name: "EX1", Description: "Execute - ALU",     Category: StageExecute}
type PipelineStage struct {
	Name        string        // Short name like "IF", "ID", "EX1"
	Description string        // Human-readable description
	Category    StageCategory // What kind of work this stage does
}

// String returns the stage name for display in diagrams.
func (s PipelineStage) String() string {
	return s.Name
}

// =========================================================================
// PipelineToken -- a unit of work flowing through the pipeline
// =========================================================================

// PipelineToken represents one instruction moving through the pipeline.
//
// Think of it as a tray on an assembly line. The tray starts empty at the
// IF stage, gets filled with decoded information at ID, gets computed
// results at EX, gets memory data at MEM, and delivers results at WB.
//
// The token is ISA-independent. The ISA decoder fills in the fields via
// callbacks. The pipeline itself never looks at instruction semantics --
// it only moves tokens between stages and handles stalls/flushes.
//
// # Token Lifecycle
//
//	IF stage:  FetchFunc fills in PC and RawInstruction
//	ID stage:  DecodeFunc fills in opcode, registers, control signals
//	EX stage:  ExecuteFunc fills in ALUResult, BranchTaken, BranchTarget
//	MEM stage: MemoryFunc fills in MemData (for loads)
//	WB stage:  WritebackFunc uses WriteData to update register file
//
// # Bubbles
//
// A "bubble" is a special token that represents NO instruction. Bubbles
// are inserted when the pipeline stalls (to fill the gap left by frozen
// stages) or when the pipeline flushes (to replace discarded speculative
// instructions). A bubble flows through the pipeline like a normal token
// but does nothing at each stage.
//
// In hardware, a bubble is a NOP (no-operation) instruction. In our
// simulator, it is a token with IsBubble = true.
type PipelineToken struct {
	// --- Instruction identity ---

	// PC is the program counter -- the memory address of this instruction.
	// Set by the IF stage when the instruction is fetched.
	PC int

	// RawInstruction is the raw instruction bits as fetched from memory.
	// Set by the IF stage via the fetch callback.
	RawInstruction int

	// Opcode is the decoded instruction name (e.g., "ADD", "LDR", "BEQ").
	// Set by the ID stage for debugging and tracing. Not used by the pipeline.
	Opcode string

	// --- Decoded operands (set by ID stage callback) ---

	// Rs1 is the first source register number (-1 means unused).
	//
	// Example: in "ADD R1, R2, R3", Rs1 = 2 (register R2).
	Rs1 int

	// Rs2 is the second source register number (-1 means unused).
	//
	// Example: in "ADD R1, R2, R3", Rs2 = 3 (register R3).
	Rs2 int

	// Rd is the destination register number (-1 means unused).
	//
	// Example: in "ADD R1, R2, R3", Rd = 1 (register R1).
	Rd int

	// Immediate is the sign-extended immediate value from the instruction.
	// Used by I-type instructions like "ADDI R1, R2, #5" (Immediate = 5).
	Immediate int

	// --- Control signals (set by ID stage callback) ---

	// RegWrite is true if this instruction writes a register.
	// ADD, LDR, ADDI all write registers. STR, BEQ do not.
	//
	// Truth table:
	//   ADD  R1, R2, R3  -> RegWrite = true  (writes R1)
	//   STR  R1, [R2]    -> RegWrite = false (only writes memory)
	//   BEQ  R1, R2, L   -> RegWrite = false (only changes PC)
	//   LDR  R1, [R2]    -> RegWrite = true  (writes R1)
	RegWrite bool

	// MemRead is true if this instruction reads from data memory.
	// Only load instructions (LDR, LW, etc.) set this.
	MemRead bool

	// MemWrite is true if this instruction writes to data memory.
	// Only store instructions (STR, SW, etc.) set this.
	MemWrite bool

	// IsBranch is true if this instruction is a branch (conditional or unconditional).
	// The pipeline uses this to know when to check branch prediction accuracy.
	IsBranch bool

	// IsHalt is true if this is a halt/stop instruction.
	// When a halt token reaches the WB stage, the pipeline stops.
	IsHalt bool

	// --- Computed values (filled during execution) ---

	// ALUResult is the output of the ALU in the EX stage.
	//
	// For arithmetic: the computed value (e.g., R2 + R3 for ADD)
	// For loads/stores: the computed memory address (e.g., R2 + offset)
	// For branches: the branch target address (PC + offset)
	ALUResult int

	// MemData is the data read from memory in the MEM stage.
	// Only meaningful for load instructions (MemRead = true).
	MemData int

	// WriteData is the final value to write to the destination register.
	// Selected in the WB stage: either ALUResult (for ALU ops) or MemData (for loads).
	WriteData int

	// BranchTaken is true if the branch was actually taken (resolved in EX).
	// The pipeline compares this with the prediction to detect mispredictions.
	BranchTaken bool

	// BranchTarget is the actual branch target address (resolved in EX).
	// Used to redirect the PC if the branch predictor was wrong.
	BranchTarget int

	// --- Pipeline metadata ---

	// IsBubble is true if this token represents a NOP/bubble.
	//
	// Bubbles are inserted in two situations:
	//   1. Stall: a bubble is inserted into the stage AFTER the stall point
	//   2. Flush: bubbles replace all speculative instructions
	//
	// A bubble flows through the pipeline normally but does nothing.
	// It does NOT count as a completed instruction in statistics.
	IsBubble bool

	// StageEntered maps stage name to the cycle number when the token
	// entered that stage. Used for tracing and debugging.
	//
	// Example: {"IF": 1, "ID": 2, "EX": 4, "MEM": 5, "WB": 6}
	// (note the gap between ID and EX -- that was a stall cycle)
	StageEntered map[string]int

	// ForwardedFrom records which stage provided a forwarded value,
	// if forwarding was used. Empty string means no forwarding.
	//
	// Example: "EX" means the value was forwarded from the EX stage.
	ForwardedFrom string
}

// NewBubble creates a new bubble token.
//
// A bubble is a "do nothing" instruction that occupies a pipeline stage
// without performing any useful work. It is the pipeline equivalent of
// a "no-op" on an assembly line -- the stage runs through its motions
// but produces no output.
func NewBubble() *PipelineToken {
	return &PipelineToken{
		IsBubble:     true,
		Rs1:          -1,
		Rs2:          -1,
		Rd:           -1,
		StageEntered: make(map[string]int),
	}
}

// NewToken creates a new empty token with default register values.
//
// The token starts with all register fields set to -1 (unused) and
// all control signals set to false. The fetch callback will fill in
// the PC and raw instruction; the decode callback fills in everything else.
func NewToken() *PipelineToken {
	return &PipelineToken{
		Rs1:          -1,
		Rs2:          -1,
		Rd:           -1,
		StageEntered: make(map[string]int),
	}
}

// String returns a human-readable representation of the token.
//
// For debugging and pipeline diagrams:
//   - Bubbles display as "---" (like empty slots on the assembly line)
//   - Normal tokens display their opcode and PC
func (t *PipelineToken) String() string {
	if t.IsBubble {
		return "---"
	}
	if t.Opcode != "" {
		return fmt.Sprintf("%s@%d", t.Opcode, t.PC)
	}
	return fmt.Sprintf("instr@%d", t.PC)
}

// Clone returns a deep copy of the token.
//
// This is necessary because tokens are passed between pipeline stages
// via pipeline registers. Each register holds its own copy so that
// modifying a token in one stage does not affect the copy in the
// pipeline register.
func (t *PipelineToken) Clone() *PipelineToken {
	if t == nil {
		return nil
	}
	clone := *t
	// Deep copy the StageEntered map
	clone.StageEntered = make(map[string]int, len(t.StageEntered))
	for k, v := range t.StageEntered {
		clone.StageEntered[k] = v
	}
	return &clone
}

// =========================================================================
// PipelineConfig -- configuration for the pipeline
// =========================================================================

// PipelineConfig holds the configuration for a pipeline.
//
// The key insight: a pipeline's behavior is determined entirely by its
// stage configuration and execution width. Everything else (instruction
// semantics, hazard handling) is injected via callbacks.
type PipelineConfig struct {
	// Stages defines the pipeline stages in order, from first to last.
	// The pipeline will have len(Stages) stages. Tokens flow from
	// Stages[0] to Stages[len-1].
	Stages []PipelineStage

	// ExecutionWidth is the number of instructions the pipeline can
	// process per cycle. Width 1 = scalar pipeline. Width > 1 = superscalar.
	// (Superscalar is a future extension; for now we only support 1.)
	ExecutionWidth int
}

// Classic5Stage returns the standard 5-stage RISC pipeline configuration.
//
// This is the pipeline described in every computer architecture textbook:
//
//	IF -> ID -> EX -> MEM -> WB
//
// It matches the MIPS R2000 (1985) and is the foundation for understanding
// all modern CPU pipelines.
func Classic5Stage() PipelineConfig {
	return PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF", Description: "Instruction Fetch", Category: StageFetch},
			{Name: "ID", Description: "Instruction Decode", Category: StageDecode},
			{Name: "EX", Description: "Execute", Category: StageExecute},
			{Name: "MEM", Description: "Memory Access", Category: StageMemory},
			{Name: "WB", Description: "Write Back", Category: StageWriteback},
		},
		ExecutionWidth: 1,
	}
}

// Deep13Stage returns a 13-stage pipeline inspired by ARM Cortex-A78.
//
// Modern high-performance CPUs split the classic 5 stages into many
// sub-stages to enable higher clock frequencies. Each sub-stage does
// less work, so it completes faster, allowing a faster clock.
//
// The tradeoff: a branch misprediction now costs 10+ cycles instead of 2.
func Deep13Stage() PipelineConfig {
	return PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF1", Description: "Fetch 1 - TLB lookup", Category: StageFetch},
			{Name: "IF2", Description: "Fetch 2 - cache read", Category: StageFetch},
			{Name: "IF3", Description: "Fetch 3 - align/buffer", Category: StageFetch},
			{Name: "ID1", Description: "Decode 1 - pre-decode", Category: StageDecode},
			{Name: "ID2", Description: "Decode 2 - full decode", Category: StageDecode},
			{Name: "ID3", Description: "Decode 3 - register read", Category: StageDecode},
			{Name: "EX1", Description: "Execute 1 - ALU", Category: StageExecute},
			{Name: "EX2", Description: "Execute 2 - shift/multiply", Category: StageExecute},
			{Name: "EX3", Description: "Execute 3 - result select", Category: StageExecute},
			{Name: "MEM1", Description: "Memory 1 - address calc", Category: StageMemory},
			{Name: "MEM2", Description: "Memory 2 - cache access", Category: StageMemory},
			{Name: "MEM3", Description: "Memory 3 - data align", Category: StageMemory},
			{Name: "WB", Description: "Write Back", Category: StageWriteback},
		},
		ExecutionWidth: 1,
	}
}

// NumStages returns the number of stages in the pipeline.
func (c PipelineConfig) NumStages() int {
	return len(c.Stages)
}

// Validate checks that the configuration is well-formed.
//
// Rules:
//   - Must have at least 2 stages (a 1-stage "pipeline" is not a pipeline)
//   - Execution width must be at least 1
//   - All stage names must be unique
//   - There must be at least one fetch stage and one writeback stage
func (c PipelineConfig) Validate() error {
	if len(c.Stages) < 2 {
		return fmt.Errorf("pipeline must have at least 2 stages, got %d", len(c.Stages))
	}
	if c.ExecutionWidth < 1 {
		return fmt.Errorf("execution width must be at least 1, got %d", c.ExecutionWidth)
	}

	// Check for unique stage names
	seen := make(map[string]bool)
	for _, s := range c.Stages {
		if seen[s.Name] {
			return fmt.Errorf("duplicate stage name: %q", s.Name)
		}
		seen[s.Name] = true
	}

	// Check for required categories
	hasFetch := false
	hasWriteback := false
	for _, s := range c.Stages {
		if s.Category == StageFetch {
			hasFetch = true
		}
		if s.Category == StageWriteback {
			hasWriteback = true
		}
	}
	if !hasFetch {
		return fmt.Errorf("pipeline must have at least one fetch stage")
	}
	if !hasWriteback {
		return fmt.Errorf("pipeline must have at least one writeback stage")
	}

	return nil
}
