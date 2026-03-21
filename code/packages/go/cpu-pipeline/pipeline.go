package cpupipeline

// =========================================================================
// Callback function types
// =========================================================================
//
// The pipeline uses dependency injection: instead of importing the cache,
// hazard detection, and branch predictor packages, the pipeline accepts
// callback functions that conform to these signatures.
//
// This decouples the pipeline from specific implementations. You can
// plug in a simple L1 cache or a complex 3-level hierarchy, a 1-bit
// predictor or a neural predictor -- the pipeline does not care.
//
// Analogy: the pipeline is like a conveyor belt. It does not care what
// is ON the belt (that is the callbacks' job). It only cares about
// MOVING items along the belt and handling stalls/flushes.

// FetchFunc fetches the raw instruction bits at the given program counter.
//
// In a real CPU, this reads from the instruction cache (L1I).
//
// Signature: (pc int) -> raw instruction bits
//
// Example implementation:
//
//	func fetch(pc int) int {
//	    return instructionMemory[pc/4]  // word-addressed
//	}
type FetchFunc func(pc int) int

// DecodeFunc decodes a raw instruction and fills in the token's fields.
//
// The decode callback receives the raw instruction bits and a token,
// and returns the token with all decoded fields filled in (opcode,
// registers, control signals, immediate value).
//
// Signature: (rawInstruction int, token *PipelineToken) -> decoded token
//
// Example:
//
//	func decode(raw int, tok *PipelineToken) *PipelineToken {
//	    tok.Opcode = decodeOpcode(raw)
//	    tok.Rd = (raw >> 7) & 0x1F
//	    tok.Rs1 = (raw >> 15) & 0x1F
//	    // ... fill in rest of fields
//	    return tok
//	}
type DecodeFunc func(rawInstruction int, token *PipelineToken) *PipelineToken

// ExecuteFunc performs the ALU operation for the instruction.
//
// The execute callback receives a decoded token and returns it with
// ALUResult, BranchTaken, and BranchTarget filled in.
//
// Signature: (token *PipelineToken) -> token with results
type ExecuteFunc func(token *PipelineToken) *PipelineToken

// MemoryFunc performs the memory access (load/store) for the instruction.
//
// For loads (MemRead=true): fills in MemData from the data cache.
// For stores (MemWrite=true): writes data to the data cache.
// For other instructions: passes the token through unchanged.
//
// Signature: (token *PipelineToken) -> token with memory data
type MemoryFunc func(token *PipelineToken) *PipelineToken

// WritebackFunc writes the instruction's result to the register file.
//
// This is the final stage. For register-writing instructions (RegWrite=true),
// WriteData is written to register Rd. The function returns nothing because
// the instruction is now complete.
//
// Signature: (token *PipelineToken) -> void
type WritebackFunc func(token *PipelineToken)

// =========================================================================
// HazardAction -- what the hazard detector tells the pipeline to do
// =========================================================================

// HazardAction represents the action the hazard unit tells the pipeline to take.
//
// These are "traffic signals" for the pipeline:
//
//	NONE:             Green light -- pipeline flows normally
//	STALL:            Red light -- freeze earlier stages, insert bubble
//	FLUSH:            Emergency stop -- discard speculative instructions
//	FORWARD_FROM_EX:  Shortcut -- grab value from EX stage output
//	FORWARD_FROM_MEM: Shortcut -- grab value from MEM stage output
//
// Priority: FLUSH > STALL > FORWARD > NONE
// (If multiple hazards are detected, the most severe one wins.)
type HazardAction int

const (
	HazardNone          HazardAction = iota // No hazard -- proceed normally
	HazardForwardFromEX                     // Forward value from EX stage
	HazardForwardFromMEM                    // Forward value from MEM stage
	HazardStall                             // Stall the pipeline (insert bubble)
	HazardFlush                             // Flush speculative stages
)

// String returns a human-readable name for the hazard action.
func (a HazardAction) String() string {
	switch a {
	case HazardNone:
		return "NONE"
	case HazardForwardFromEX:
		return "FORWARD_FROM_EX"
	case HazardForwardFromMEM:
		return "FORWARD_FROM_MEM"
	case HazardStall:
		return "STALL"
	case HazardFlush:
		return "FLUSH"
	default:
		return "UNKNOWN"
	}
}

// HazardResponse is the full response from the hazard detection callback.
//
// It tells the pipeline what to do and provides additional context
// (forwarded values, stall duration, flush target).
type HazardResponse struct {
	// Action is the hazard action to take.
	Action HazardAction

	// ForwardValue is the value to forward (only used for FORWARD actions).
	ForwardValue int

	// ForwardSource is the stage that provided the forwarded value.
	ForwardSource string

	// StallStages is the number of stages to stall (typically 1).
	StallStages int

	// FlushCount is the number of stages to flush on a misprediction.
	FlushCount int

	// RedirectPC is the correct PC to fetch from after a flush.
	// Only meaningful when Action == HazardFlush.
	RedirectPC int
}

// HazardFunc checks for hazards given the current pipeline stage contents.
//
// The function receives the tokens currently in each stage and returns
// a response indicating what the pipeline should do.
//
// Signature: (stages []*PipelineToken) -> HazardResponse
//
// The stages slice is ordered from first stage (IF) to last stage (WB),
// matching the pipeline's stage order. A nil entry means the stage is empty.
type HazardFunc func(stages []*PipelineToken) HazardResponse

// PredictFunc predicts the next PC given the current PC.
//
// Used by the IF stage to speculatively fetch the next instruction
// before the current branch is resolved.
//
// Signature: (pc int) -> predicted next PC
//
// A simple implementation might always predict PC+4 (not taken).
// A more sophisticated one would use a branch target buffer.
type PredictFunc func(pc int) int

// =========================================================================
// Pipeline -- the main pipeline struct
// =========================================================================

// Pipeline is a configurable N-stage instruction pipeline.
//
// # How it Works
//
// The pipeline is a slice of "slots", one per stage. Each slot holds a
// pointer to a PipelineToken (or nil if the stage is empty). On each
// clock cycle (call to Step()):
//
//  1. Check for hazards (via HazardFunc callback)
//  2. If stalled: freeze stages before the stall point, insert bubble
//  3. If flushing: replace speculative stages with bubbles
//  4. Otherwise: shift all tokens one stage forward
//  5. Execute stage callbacks (fetch, decode, execute, memory, writeback)
//  6. Record a snapshot for tracing
//
// All transitions happen "simultaneously" -- we compute the next state
// from the current state, then swap. This models the behavior of
// edge-triggered flip-flops in real pipeline registers.
//
// # Pipeline Registers
//
// In real hardware, pipeline registers sit BETWEEN stages and latch
// data on the clock edge. In our model, we represent this by computing
// the new state of all stages before committing any changes. The
// "stages" slice IS the set of pipeline registers.
//
// # Example: 5-cycle execution of ADD instruction
//
//	Cycle 1: IF  -- fetch instruction at PC, ask branch predictor for next PC
//	Cycle 2: ID  -- decode: extract opcode=ADD, Rd=1, Rs1=2, Rs2=3
//	Cycle 3: EX  -- execute: ALUResult = Reg[2] + Reg[3]
//	Cycle 4: MEM -- memory: pass through (ADD doesn't access memory)
//	Cycle 5: WB  -- writeback: Reg[1] = ALUResult
type Pipeline struct {
	// config is the pipeline configuration (stages, width).
	config PipelineConfig

	// stages holds the current token in each pipeline stage.
	// stages[0] is the first stage (IF), stages[N-1] is the last (WB).
	// A nil entry means the stage is empty.
	stages []*PipelineToken

	// pc is the current program counter (address of next instruction to fetch).
	pc int

	// cycle is the current clock cycle number (starts at 0, incremented by Step).
	cycle int

	// halted is true if a halt instruction has reached the last stage.
	halted bool

	// stats tracks execution statistics.
	stats PipelineStats

	// history stores a snapshot of the pipeline state at each cycle.
	history []PipelineSnapshot

	// --- Callbacks ---

	// fetchFn fetches raw instruction bits at a given PC.
	fetchFn FetchFunc

	// decodeFn decodes raw instruction bits into a token.
	decodeFn DecodeFunc

	// executeFn performs the ALU operation.
	executeFn ExecuteFunc

	// memoryFn performs memory access (loads/stores).
	memoryFn MemoryFunc

	// writebackFn writes results to the register file.
	writebackFn WritebackFunc

	// --- Optional integration callbacks ---

	// hazardFn checks for pipeline hazards. If nil, no hazard detection.
	hazardFn HazardFunc

	// predictFn predicts the next PC. If nil, defaults to PC + 4.
	predictFn PredictFunc
}

// NewPipeline creates a new pipeline with the given configuration and callbacks.
//
// The configuration is validated before use. All five stage callbacks are
// required; hazard and predict callbacks are optional.
//
// Returns an error if the configuration is invalid.
func NewPipeline(
	config PipelineConfig,
	fetch FetchFunc,
	decode DecodeFunc,
	execute ExecuteFunc,
	memory MemoryFunc,
	writeback WritebackFunc,
) (*Pipeline, error) {
	if err := config.Validate(); err != nil {
		return nil, err
	}

	return &Pipeline{
		config:      config,
		stages:      make([]*PipelineToken, config.NumStages()),
		pc:          0,
		cycle:       0,
		halted:      false,
		fetchFn:     fetch,
		decodeFn:    decode,
		executeFn:   execute,
		memoryFn:    memory,
		writebackFn: writeback,
	}, nil
}

// SetHazardFunc sets the optional hazard detection callback.
//
// The hazard function is called at the beginning of each cycle to determine
// if the pipeline needs to stall or flush.
func (p *Pipeline) SetHazardFunc(fn HazardFunc) {
	p.hazardFn = fn
}

// SetPredictFunc sets the optional branch prediction callback.
//
// The predict function is called during the fetch stage to determine the
// next PC to fetch from (speculatively, before the branch is resolved).
func (p *Pipeline) SetPredictFunc(fn PredictFunc) {
	p.predictFn = fn
}

// SetPC sets the program counter (the address of the next instruction to fetch).
func (p *Pipeline) SetPC(pc int) {
	p.pc = pc
}

// PC returns the current program counter.
func (p *Pipeline) PC() int {
	return p.pc
}

// Step advances the pipeline by one clock cycle.
//
// This is the heart of the pipeline simulator. Each call to Step()
// corresponds to one rising clock edge in hardware.
//
// # Step Algorithm
//
//  1. If halted, return the current snapshot (do nothing).
//
//  2. Increment the cycle counter.
//
//  3. Check for hazards by calling hazardFn (if set).
//     The hazard function receives the current stage contents and
//     returns a HazardResponse (stall, flush, forward, or none).
//
//  4. Handle the hazard response:
//     a. FLUSH: Replace speculative stages with bubbles and redirect PC.
//     b. STALL: Freeze earlier stages and insert a bubble after the stall point.
//     c. FORWARD: Update the token with the forwarded value.
//     d. NONE: Normal advancement.
//
//  5. Advance tokens through stages:
//     - Shift all tokens one stage forward (from last to first).
//     - The token leaving the last stage is "retired" (completed).
//     - A new token is fetched into the first stage.
//
//  6. Execute stage callbacks on each token:
//     - Fetch stages: call fetchFn to get raw instruction bits.
//     - Decode stages: call decodeFn to decode the instruction.
//     - Execute stages: call executeFn to compute ALU result.
//     - Memory stages: call memoryFn for loads/stores.
//     - Writeback stages: call writebackFn to write register file.
//
//  7. Update statistics (completed instructions, bubble count).
//
//  8. Record a snapshot and return it.
//
// # Simultaneous Transitions
//
// In real hardware, all pipeline registers update simultaneously on the
// clock edge. We model this by first computing the new state of all
// stages, then committing the changes. This prevents one stage's update
// from affecting another stage's input within the same cycle.
func (p *Pipeline) Step() PipelineSnapshot {
	if p.halted {
		return p.takeSnapshot()
	}

	p.cycle++
	p.stats.TotalCycles++
	numStages := p.config.NumStages()

	// --- Phase 1: Check for hazards ---
	//
	// The hazard function examines the CURRENT pipeline state (before any
	// advancement) and returns a verdict: stall, flush, forward, or proceed.
	hazard := HazardResponse{Action: HazardNone}
	if p.hazardFn != nil {
		stagesCopy := make([]*PipelineToken, numStages)
		copy(stagesCopy, p.stages)
		hazard = p.hazardFn(stagesCopy)
	}

	// --- Phase 2: Compute next state ---
	//
	// We build the next state in a new slice, then swap it in at the end.
	// This ensures all transitions are "simultaneous" -- like real
	// edge-triggered flip-flops that all capture on the same clock edge.
	//
	// Key design: tokens that arrive in the LAST stage during this cycle
	// are "retired" (writeback callback called) at the END of this cycle.
	// This means: in a 5-stage pipeline, the first instruction completes
	// at cycle 5 (not cycle 6), matching real hardware behavior.
	nextStages := make([]*PipelineToken, numStages)
	stalled := false
	flushing := false

	switch hazard.Action {

	case HazardFlush:
		// FLUSH: Replace speculative stages with bubbles.
		//
		// A flush happens when a branch misprediction is detected. The
		// instructions fetched after the branch (which were fetched
		// speculatively based on the wrong prediction) must be discarded.
		//
		// Which stages to flush? Everything before the stage that detected
		// the misprediction. In a classic 5-stage pipeline with the branch
		// resolved at EX (stage 2), we flush IF (stage 0) and ID (stage 1).
		//
		// Visualization (branch resolved at EX, stage 2):
		//
		//   Before flush:
		//     IF: wrong_instr_3   <- FLUSH (replace with bubble)
		//     ID: wrong_instr_2   <- FLUSH (replace with bubble)
		//     EX: branch_instr    <- keeps executing (detected the mispredict)
		//     MEM: older_instr    <- keeps executing
		//     WB: oldest_instr    <- keeps executing
		//
		//   After flush:
		//     IF: correct_instr   <- fetched from redirect PC
		//     ID: ---             <- bubble
		//     EX: branch_instr    <- continues
		//     MEM: older_instr    <- continues
		//     WB: oldest_instr    <- continues

		flushing = true
		p.stats.FlushCycles++

		// Determine how many stages to flush (from the front).
		flushCount := hazard.FlushCount
		if flushCount <= 0 {
			for i, s := range p.config.Stages {
				if s.Category == StageExecute {
					flushCount = i
					break
				}
			}
			if flushCount <= 0 {
				flushCount = 1
			}
		}
		if flushCount > numStages {
			flushCount = numStages
		}

		// Shift non-flushed stages forward (from back to front).
		for i := numStages - 1; i >= flushCount; i-- {
			if i > 0 && i-1 >= flushCount {
				nextStages[i] = p.stages[i-1]
			} else if i > 0 {
				nextStages[i] = NewBubble()
				nextStages[i].StageEntered[p.config.Stages[i].Name] = p.cycle
			} else {
				nextStages[i] = p.stages[i]
			}
		}

		// Replace flushed stages with bubbles.
		for i := 0; i < flushCount; i++ {
			nextStages[i] = NewBubble()
			nextStages[i].StageEntered[p.config.Stages[i].Name] = p.cycle
		}

		// Redirect PC and fetch from the correct target.
		p.pc = hazard.RedirectPC
		tok := p.fetchNewInstruction()
		nextStages[0] = tok

	case HazardStall:
		// STALL: Freeze earlier stages and insert a bubble.
		//
		// A stall happens when a data hazard cannot be resolved by
		// forwarding -- typically a load-use hazard. The instruction
		// in the decode stage needs a value that is being loaded from
		// memory, but the load hasn't reached the MEM stage yet.
		//
		// The pipeline freezes the IF and ID stages (they keep their
		// current tokens) and inserts a bubble into the EX stage.
		// The MEM and WB stages continue normally.
		//
		// Visualization (load-use stall, stall point at EX / stage 2):
		//
		//   Before stall:
		//     IF: instr_3         <- FROZEN
		//     ID: dependent_instr <- FROZEN
		//     EX: load_instr      <- continues to MEM
		//     MEM: older_instr    <- continues to WB
		//     WB: oldest_instr    <- retires
		//
		//   After stall:
		//     IF: instr_3         <- same as before
		//     ID: dependent_instr <- same as before
		//     EX: ---             <- bubble inserted
		//     MEM: load_instr     <- was in EX
		//     WB: older_instr     <- was in MEM

		stalled = true
		p.stats.StallCycles++

		// Find the stall insertion point.
		stallPoint := hazard.StallStages
		if stallPoint <= 0 {
			for i, s := range p.config.Stages {
				if s.Category == StageExecute {
					stallPoint = i
					break
				}
			}
			if stallPoint <= 0 {
				stallPoint = 1
			}
		}
		if stallPoint >= numStages {
			stallPoint = numStages - 1
		}

		// Stages AFTER the stall point advance normally.
		for i := numStages - 1; i > stallPoint; i-- {
			nextStages[i] = p.stages[i-1]
		}

		// Insert bubble at the stall point.
		nextStages[stallPoint] = NewBubble()
		nextStages[stallPoint].StageEntered[p.config.Stages[stallPoint].Name] = p.cycle

		// Stages BEFORE the stall point are frozen.
		for i := 0; i < stallPoint; i++ {
			nextStages[i] = p.stages[i]
		}

		// PC does NOT advance during a stall.

	default:
		// NONE or FORWARD: Normal advancement.
		//
		// Every token moves one stage forward. The token that was in the
		// second-to-last stage moves to the last stage and will be retired.
		// A new token is fetched into the first stage.
		//
		// Visualization:
		//
		//   Before:
		//     IF: instr_5   ID: instr_4   EX: instr_3   MEM: instr_2   WB: instr_1
		//
		//   After:
		//     IF: instr_6   ID: instr_5   EX: instr_4   MEM: instr_3   WB: instr_2
		//     (instr_1 was retired last cycle; instr_2 will retire this cycle)

		// Handle forwarding if needed.
		if hazard.Action == HazardForwardFromEX || hazard.Action == HazardForwardFromMEM {
			for i, s := range p.config.Stages {
				if s.Category == StageDecode && p.stages[i] != nil && !p.stages[i].IsBubble {
					p.stages[i].ALUResult = hazard.ForwardValue
					p.stages[i].ForwardedFrom = hazard.ForwardSource
					break
				}
			}
		}

		// Shift tokens forward (from back to front).
		for i := numStages - 1; i > 0; i-- {
			nextStages[i] = p.stages[i-1]
		}

		// Fetch new instruction into IF stage.
		tok := p.fetchNewInstruction()
		nextStages[0] = tok
	}

	// --- Phase 3: Commit the new state ---
	p.stages = nextStages

	// --- Phase 4: Execute stage callbacks ---
	//
	// Now that all tokens are in their new positions, run the
	// stage-specific callbacks. We iterate from LAST to FIRST.
	for i := numStages - 1; i >= 0; i-- {
		tok := p.stages[i]
		if tok == nil || tok.IsBubble {
			continue
		}

		stage := p.config.Stages[i]

		// Record when this token entered this stage.
		if _, exists := tok.StageEntered[stage.Name]; !exists {
			tok.StageEntered[stage.Name] = p.cycle
		}

		switch stage.Category {
		case StageFetch:
			// Already handled by fetchNewInstruction().

		case StageDecode:
			if tok.Opcode == "" {
				p.stages[i] = p.decodeFn(tok.RawInstruction, tok)
			}

		case StageExecute:
			if tok.StageEntered[stage.Name] == p.cycle {
				p.stages[i] = p.executeFn(tok)
			}

		case StageMemory:
			if tok.StageEntered[stage.Name] == p.cycle {
				p.stages[i] = p.memoryFn(tok)
			}

		case StageWriteback:
			// Writeback is handled in Phase 5 (retirement).
		}
	}

	// --- Phase 5: Retire the instruction in the last stage ---
	//
	// The token that is NOW in the last stage (after advancement) gets
	// its writeback callback called. This is the "retirement" of the
	// instruction -- it has completed all pipeline stages.
	//
	// This happens at the END of the cycle, so in a 5-stage pipeline,
	// the first instruction completes at cycle 5 (enters WB at step 5,
	// writeback fires at step 5).
	lastTok := p.stages[numStages-1]
	if lastTok != nil && !lastTok.IsBubble {
		p.writebackFn(lastTok)
		p.stats.InstructionsCompleted++
		if lastTok.IsHalt {
			p.halted = true
		}
	}

	// Count bubbles across all stages.
	for _, tok := range p.stages {
		if tok != nil && tok.IsBubble {
			p.stats.BubbleCycles++
		}
	}

	// --- Phase 6: Take snapshot ---
	snap := PipelineSnapshot{
		Cycle:    p.cycle,
		Stages:   make(map[string]*PipelineToken, numStages),
		Stalled:  stalled,
		Flushing: flushing,
		PC:       p.pc,
	}
	for i, stage := range p.config.Stages {
		if p.stages[i] != nil {
			snap.Stages[stage.Name] = p.stages[i].Clone()
		}
	}
	p.history = append(p.history, snap)

	return snap
}

// fetchNewInstruction creates a new token by calling the fetch callback.
//
// This is called at the start of each cycle to fetch the instruction
// at the current PC. The PC is then advanced (either by the branch
// predictor's prediction or by the default PC+4).
func (p *Pipeline) fetchNewInstruction() *PipelineToken {
	tok := NewToken()
	tok.PC = p.pc
	tok.RawInstruction = p.fetchFn(p.pc)
	tok.StageEntered[p.config.Stages[0].Name] = p.cycle

	// Advance PC: use branch predictor if available, otherwise PC+4.
	if p.predictFn != nil {
		p.pc = p.predictFn(p.pc)
	} else {
		p.pc += 4
	}

	return tok
}

// Run executes the pipeline until a halt instruction is encountered or
// the maximum cycle count is reached.
//
// This is the main simulation loop. It calls Step() repeatedly until
// the pipeline halts or the cycle budget is exhausted.
//
// Returns the final execution statistics.
func (p *Pipeline) Run(maxCycles int) PipelineStats {
	for p.cycle < maxCycles && !p.halted {
		p.Step()
	}
	return p.stats
}

// Snapshot returns the current pipeline state without advancing the clock.
//
// This is useful for inspecting the pipeline between steps or after
// the simulation completes.
func (p *Pipeline) Snapshot() PipelineSnapshot {
	return p.takeSnapshot()
}

// Stats returns a copy of the current execution statistics.
func (p *Pipeline) Stats() PipelineStats {
	return p.stats
}

// IsHalted returns true if a halt instruction has reached the last stage.
func (p *Pipeline) IsHalted() bool {
	return p.halted
}

// Cycle returns the current cycle number.
func (p *Pipeline) Cycle() int {
	return p.cycle
}

// Trace returns the complete history of pipeline snapshots.
//
// The trace includes one snapshot per cycle, in chronological order.
// This is used for visualization and debugging.
func (p *Pipeline) Trace() []PipelineSnapshot {
	result := make([]PipelineSnapshot, len(p.history))
	copy(result, p.history)
	return result
}

// StageContents returns the token currently occupying the given stage.
//
// Returns nil if the stage is empty or the stage name is invalid.
func (p *Pipeline) StageContents(stageName string) *PipelineToken {
	for i, s := range p.config.Stages {
		if s.Name == stageName {
			return p.stages[i]
		}
	}
	return nil
}

// Config returns the pipeline configuration.
func (p *Pipeline) Config() PipelineConfig {
	return p.config
}

// takeSnapshot creates a snapshot of the current pipeline state.
func (p *Pipeline) takeSnapshot() PipelineSnapshot {
	numStages := p.config.NumStages()
	snap := PipelineSnapshot{
		Cycle:  p.cycle,
		Stages: make(map[string]*PipelineToken, numStages),
		PC:     p.pc,
	}
	for i, stage := range p.config.Stages {
		if p.stages[i] != nil {
			snap.Stages[stage.Name] = p.stages[i].Clone()
		}
	}
	return snap
}
