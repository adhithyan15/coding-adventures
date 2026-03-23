package core

import (
	branchpredictor "github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor"
	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
	hazarddetection "github.com/adhithyan15/coding-adventures/code/packages/go/hazard-detection"
)

// =========================================================================
// Core -- a configurable processor core
// =========================================================================

// Core is a complete processor core that composes all D-series sub-components:
//
//   - Pipeline (D04): manages instruction flow through stages
//   - Branch Predictor (D02): speculative fetch direction
//   - Hazard Unit (D03): data, control, and structural hazard detection
//   - Cache Hierarchy (D01): L1I + L1D + optional L2
//   - Register File: fast operand storage
//   - Clock: cycle-accurate timing
//   - Memory Controller: access to backing memory
//
// The Core wires these together by providing callback functions to the
// pipeline. When the pipeline needs to fetch an instruction, it calls the
// Core's fetch callback, which reads from the L1I cache. When it needs to
// decode, it calls the ISA decoder. And so on.
//
// # Construction
//
// The Core is constructed from a CoreConfig and an ISADecoder:
//
//	config := core.SimpleConfig()
//	decoder := core.NewMockDecoder()
//	c, err := core.NewCore(config, decoder)
//
// # Execution
//
// The Core runs one cycle at a time via Step(), or until halt via Run():
//
//	c.Step()              // advance one clock cycle
//	stats := c.Run(1000) // run up to 1000 cycles
//
// # ISA Independence
//
// The Core does not know what instructions mean. The ISADecoder provides
// instruction semantics. The same Core can run ARM, RISC-V, or any custom
// ISA by swapping the decoder.
type Core struct {
	// config is the core configuration.
	config CoreConfig

	// decoder is the injected ISA decoder.
	decoder ISADecoder

	// pipeline manages instruction flow through stages.
	pipeline *cpupipeline.Pipeline

	// predictor guesses branch directions.
	predictor branchpredictor.BranchPredictor

	// btb caches branch target addresses.
	btb *branchpredictor.BranchTargetBuffer

	// hazardUnit detects pipeline hazards.
	hazardUnit *hazarddetection.HazardUnit

	// cacheHierarchy provides L1I, L1D, and optional L2 caches.
	cacheHierarchy *cache.CacheHierarchy

	// regFile is the general-purpose register file.
	regFile *RegisterFile

	// memCtrl provides access to main memory.
	memCtrl *MemoryController

	// clk is the system clock.
	clk *clock.Clock

	// halted is true when a HALT instruction reaches writeback.
	halted bool

	// cycle tracks the current cycle number.
	cycle int

	// instructionsCompleted counts retired (non-bubble) instructions.
	instructionsCompleted int

	// forwardCount tracks total forwarding operations.
	forwardCount int

	// stallCount tracks total stall cycles.
	stallCount int

	// flushCount tracks total pipeline flush cycles.
	flushCount int
}

// NewCore creates a fully-wired processor core from the given configuration
// and ISA decoder.
//
// # What Happens During Construction
//
// 1. The register file is created from the config.
// 2. Main memory is allocated and wrapped in a MemoryController.
// 3. Caches are created (L1I, L1D, optional L2) and assembled into a hierarchy.
// 4. The branch predictor and BTB are created.
// 5. The hazard unit is created.
// 6. The pipeline is created with callbacks wired to the Core's methods.
// 7. The clock is created.
//
// Returns an error if the pipeline configuration is invalid.
func NewCore(config CoreConfig, decoder ISADecoder) (*Core, error) {
	c := &Core{
		config:  config,
		decoder: decoder,
	}

	// --- 1. Register File ---
	c.regFile = NewRegisterFile(config.RegisterFile)

	// --- 2. Memory ---
	memSize := config.MemorySize
	if memSize <= 0 {
		memSize = 65536
	}
	memory := make([]byte, memSize)
	memLatency := config.MemoryLatency
	if memLatency <= 0 {
		memLatency = 100
	}
	c.memCtrl = NewMemoryController(memory, memLatency)

	// --- 3. Cache Hierarchy ---
	c.cacheHierarchy = c.buildCacheHierarchy(config, memLatency)

	// --- 4. Branch Predictor + BTB ---
	c.predictor = createBranchPredictor(config.BranchPredictorType, config.BranchPredictorSize)
	btbSize := config.BTBSize
	if btbSize <= 0 {
		btbSize = 64
	}
	c.btb = branchpredictor.NewBranchTargetBuffer(btbSize)

	// --- 5. Hazard Unit ---
	numFPUnits := 0
	if config.FPUnit != nil {
		numFPUnits = 1
	}
	c.hazardUnit = hazarddetection.NewHazardUnit(1, numFPUnits, true)

	// --- 6. Pipeline ---
	pipelineConfig := config.Pipeline
	if len(pipelineConfig.Stages) == 0 {
		pipelineConfig = cpupipeline.Classic5Stage()
	}

	pipeline, err := cpupipeline.NewPipeline(
		pipelineConfig,
		c.fetchCallback,
		c.decodeCallback,
		c.executeCallback,
		c.memoryCallback,
		c.writebackCallback,
	)
	if err != nil {
		return nil, err
	}

	// Wire optional callbacks.
	if config.HazardDetection {
		pipeline.SetHazardFunc(c.hazardCallback)
	}
	pipeline.SetPredictFunc(c.predictCallback)

	c.pipeline = pipeline

	// --- 7. Clock ---
	c.clk = clock.New(1000000000) // 1 GHz nominal

	return c, nil
}

// buildCacheHierarchy creates the L1I, L1D, and optional L2 caches.
func (c *Core) buildCacheHierarchy(config CoreConfig, memLatency int) *cache.CacheHierarchy {
	// Default L1I: 4KB direct-mapped, 64B lines, 1-cycle latency.
	l1iCfg := config.L1ICache
	if l1iCfg == nil {
		defaultCfg := cache.CacheConfig{
			Name: "L1I", TotalSize: 4096, LineSize: 64,
			Associativity: 1, AccessLatency: 1, WritePolicy: "write-back",
		}
		l1iCfg = &defaultCfg
	}
	l1i := cache.NewCache(*l1iCfg)

	// Default L1D: 4KB direct-mapped, 64B lines, 1-cycle latency.
	l1dCfg := config.L1DCache
	if l1dCfg == nil {
		defaultCfg := cache.CacheConfig{
			Name: "L1D", TotalSize: 4096, LineSize: 64,
			Associativity: 1, AccessLatency: 1, WritePolicy: "write-back",
		}
		l1dCfg = &defaultCfg
	}
	l1d := cache.NewCache(*l1dCfg)

	// Optional L2.
	var l2 *cache.Cache
	if config.L2Cache != nil {
		l2 = cache.NewCache(*config.L2Cache)
	}

	return cache.NewCacheHierarchy(l1i, l1d, l2, nil, memLatency)
}

// =========================================================================
// Pipeline Callbacks -- the Core provides these to the pipeline
// =========================================================================

// fetchCallback is called by the pipeline's IF stage.
//
// It reads the raw instruction bits from memory at the given PC.
// In a real CPU, this goes through the L1I cache. For simplicity,
// we read directly from the memory controller (the cache hierarchy
// tracks statistics separately).
func (c *Core) fetchCallback(pc int) int {
	// Read from instruction cache hierarchy for statistics.
	c.cacheHierarchy.Read(pc, true, c.cycle)

	// Read the actual instruction bits from memory.
	return c.memCtrl.ReadWord(pc)
}

// decodeCallback is called by the pipeline's ID stage.
//
// It delegates to the injected ISA decoder to fill in the token's
// decoded fields (opcode, registers, control signals).
func (c *Core) decodeCallback(raw int, token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	return c.decoder.Decode(raw, token)
}

// executeCallback is called by the pipeline's EX stage.
//
// It delegates to the ISA decoder's Execute method, which computes
// ALU results, resolves branches, and calculates effective addresses.
//
// After execution, if the instruction is a branch, we update the
// branch predictor and BTB with the actual outcome.
func (c *Core) executeCallback(token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	result := c.decoder.Execute(token, c.regFile)

	// Update branch predictor with actual outcome.
	if result.IsBranch {
		c.predictor.Update(result.PC, result.BranchTaken, result.BranchTarget)
		if result.BranchTaken {
			c.btb.Update(result.PC, result.BranchTarget, "conditional")
		}
	}

	return result
}

// memoryCallback is called by the pipeline's MEM stage.
//
// For load instructions (MemRead=true): reads data from the L1D cache
// (which may go to L2 or memory on a miss) and fills in MemData.
//
// For store instructions (MemWrite=true): writes data to the L1D cache.
//
// For other instructions: passes the token through unchanged.
func (c *Core) memoryCallback(token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	if token.MemRead {
		// Load: read from data cache hierarchy.
		c.cacheHierarchy.Read(token.ALUResult, false, c.cycle)

		// Read the actual word from memory.
		token.MemData = c.memCtrl.ReadWord(token.ALUResult)
		token.WriteData = token.MemData
	} else if token.MemWrite {
		// Store: write to data cache hierarchy.
		data := []int{token.WriteData & 0xFF}
		c.cacheHierarchy.Write(token.ALUResult, data, c.cycle)

		// Write the actual word to memory.
		c.memCtrl.WriteWord(token.ALUResult, token.WriteData)
	}
	return token
}

// writebackCallback is called by the pipeline's WB stage.
//
// For register-writing instructions (RegWrite=true), WriteData is written
// to the destination register Rd. The instruction is now "retired."
func (c *Core) writebackCallback(token *cpupipeline.PipelineToken) {
	if token.RegWrite && token.Rd >= 0 {
		c.regFile.Write(token.Rd, token.WriteData)
	}
}

// hazardCallback is called at the start of each cycle to check for hazards.
//
// It translates the pipeline's stage contents into PipelineSlots that the
// hazard unit can analyze, then converts the hazard result back into a
// HazardResponse that the pipeline understands.
func (c *Core) hazardCallback(stages []*cpupipeline.PipelineToken) cpupipeline.HazardResponse {
	numStages := len(stages)
	pipelineCfg := c.config.Pipeline
	if len(pipelineCfg.Stages) == 0 {
		pipelineCfg = cpupipeline.Classic5Stage()
	}

	// Find the IF, ID, EX, MEM stages by category.
	var ifTok, idTok, exTok, memTok *cpupipeline.PipelineToken
	for i, stage := range pipelineCfg.Stages {
		if i >= numStages {
			break
		}
		tok := stages[i]
		switch stage.Category {
		case cpupipeline.StageFetch:
			if ifTok == nil {
				ifTok = tok
			}
		case cpupipeline.StageDecode:
			// Use the LAST decode stage (closest to EX).
			idTok = tok
		case cpupipeline.StageExecute:
			if exTok == nil {
				exTok = tok
			}
		case cpupipeline.StageMemory:
			if memTok == nil {
				memTok = tok
			}
		}
	}

	// Convert PipelineTokens to PipelineSlots for the hazard unit.
	ifSlot := tokenToSlot(ifTok)
	idSlot := tokenToSlot(idTok)
	exSlot := tokenToSlot(exTok)
	memSlot := tokenToSlot(memTok)

	// Run hazard detection.
	result := c.hazardUnit.Check(ifSlot, idSlot, exSlot, memSlot)

	// Convert HazardResult to HazardResponse.
	response := cpupipeline.HazardResponse{
		Action: cpupipeline.HazardNone,
	}

	switch result.Action {
	case hazarddetection.ActionStall:
		response.Action = cpupipeline.HazardStall
		response.StallStages = result.StallCycles
		c.stallCount++

	case hazarddetection.ActionFlush:
		response.Action = cpupipeline.HazardFlush
		response.FlushCount = result.FlushCount
		// Redirect PC to the correct target.
		if exTok != nil && exTok.IsBranch {
			if exTok.BranchTaken {
				response.RedirectPC = exTok.BranchTarget
			} else {
				response.RedirectPC = exTok.PC + c.decoder.InstructionSize()
			}
		}
		c.flushCount++

	case hazarddetection.ActionForwardFromEX:
		response.Action = cpupipeline.HazardForwardFromEX
		if result.ForwardedValue != nil {
			response.ForwardValue = *result.ForwardedValue
		}
		response.ForwardSource = result.ForwardedFrom
		c.forwardCount++

	case hazarddetection.ActionForwardFromMEM:
		response.Action = cpupipeline.HazardForwardFromMEM
		if result.ForwardedValue != nil {
			response.ForwardValue = *result.ForwardedValue
		}
		response.ForwardSource = result.ForwardedFrom
		c.forwardCount++
	}

	return response
}

// predictCallback is called by the pipeline's IF stage to predict the next PC.
//
// It consults the branch predictor for direction and the BTB for target:
//   - If the predictor says "taken" and the BTB has a target, fetch from target.
//   - Otherwise, fetch sequentially (PC + instruction_size).
func (c *Core) predictCallback(pc int) int {
	prediction := c.predictor.Predict(pc)
	instrSize := c.decoder.InstructionSize()

	if prediction.Taken {
		// Check BTB for target address.
		target := c.btb.Lookup(pc)
		if target != branchpredictor.NoTarget {
			return target
		}
	}

	// Default: sequential fetch.
	return pc + instrSize
}

// tokenToSlot converts a PipelineToken to a hazard-detection PipelineSlot.
//
// This bridges the gap between the pipeline package (which uses PipelineToken)
// and the hazard-detection package (which uses PipelineSlot). The Core must
// translate between the two because the packages are deliberately decoupled.
func tokenToSlot(tok *cpupipeline.PipelineToken) hazarddetection.PipelineSlot {
	if tok == nil || tok.IsBubble {
		return hazarddetection.PipelineSlot{Valid: false}
	}

	slot := hazarddetection.PipelineSlot{
		Valid:    true,
		PC:       tok.PC,
		IsBranch: tok.IsBranch,
		MemRead:  tok.MemRead,
		MemWrite: tok.MemWrite,
		UsesALU:  true, // Most instructions use the ALU
	}

	// Source registers.
	if tok.Rs1 >= 0 {
		slot.SourceRegs = append(slot.SourceRegs, tok.Rs1)
	}
	if tok.Rs2 >= 0 {
		slot.SourceRegs = append(slot.SourceRegs, tok.Rs2)
	}

	// Destination register.
	if tok.Rd >= 0 && tok.RegWrite {
		slot.DestReg = hazarddetection.IntPtr(tok.Rd)
		// Provide the computed value for forwarding.
		if tok.ALUResult != 0 || tok.WriteData != 0 {
			val := tok.ALUResult
			if tok.WriteData != 0 {
				val = tok.WriteData
			}
			slot.DestValue = hazarddetection.IntPtr(val)
		}
	}

	// Branch prediction tracking.
	if tok.IsBranch {
		slot.BranchTaken = tok.BranchTaken
		// BranchPredictedTaken is set based on whether we fetched sequentially.
		// If the pipeline fetched from a non-sequential address, the prediction
		// was "taken". This is approximated here.
		slot.BranchPredictedTaken = false // Default assumption
	}

	return slot
}

// =========================================================================
// Public API -- Step, Run, LoadProgram, etc.
// =========================================================================

// LoadProgram loads machine code into memory starting at the given address.
//
// The program bytes are written to main memory. Each instruction is
// 4 bytes (for the MockDecoder). The PC should be set to startAddress
// before calling Run() or Step().
//
// Example:
//
//	program := encodeProgram(EncodeADDI(1, 0, 42), EncodeHALT())
//	core.LoadProgram(program, 0)
//	core.Run(100)
func (c *Core) LoadProgram(program []byte, startAddress int) {
	c.memCtrl.LoadProgram(program, startAddress)
	c.pipeline.SetPC(startAddress)
}

// Step executes one clock cycle.
//
// This advances the pipeline by one step, which:
//   - Checks for hazards (stalls, flushes, forwards)
//   - Moves tokens through pipeline stages
//   - Executes stage callbacks (fetch, decode, execute, memory, writeback)
//   - Updates statistics
//
// Returns the pipeline snapshot for this cycle (useful for tracing).
func (c *Core) Step() cpupipeline.PipelineSnapshot {
	if c.halted {
		return c.pipeline.Snapshot()
	}

	c.cycle++
	snap := c.pipeline.Step()

	// Check if the pipeline halted this cycle.
	if c.pipeline.IsHalted() {
		c.halted = true
	}

	// Track completed instructions.
	c.instructionsCompleted = c.pipeline.Stats().InstructionsCompleted

	return snap
}

// Run executes the core until it halts or maxCycles is reached.
//
// Returns aggregate statistics for the entire run. This is the main
// entry point for running a program on the core.
//
// Example:
//
//	stats := core.Run(10000)
//	fmt.Printf("IPC: %.3f\n", stats.IPC())
func (c *Core) Run(maxCycles int) CoreStats {
	for c.cycle < maxCycles && !c.halted {
		c.Step()
	}
	return c.Stats()
}

// Stats returns aggregate statistics from all sub-components.
//
// This collects stats from the pipeline, branch predictor, hazard unit,
// and cache hierarchy into a single CoreStats struct.
func (c *Core) Stats() CoreStats {
	pStats := c.pipeline.Stats()

	stats := CoreStats{
		InstructionsCompleted: pStats.InstructionsCompleted,
		TotalCycles:           pStats.TotalCycles,
		PipelineStats:         pStats,
		PredictorStats:        c.predictor.Stats(),
		CacheStats:            make(map[string]*cache.CacheStats),
		ForwardCount:          c.forwardCount,
		StallCount:            c.stallCount,
		FlushCount:            c.flushCount,
	}

	// Collect cache stats.
	if c.cacheHierarchy.L1I != nil {
		stats.CacheStats["L1I"] = &c.cacheHierarchy.L1I.Stats
	}
	if c.cacheHierarchy.L1D != nil {
		stats.CacheStats["L1D"] = &c.cacheHierarchy.L1D.Stats
	}
	if c.cacheHierarchy.L2 != nil {
		stats.CacheStats["L2"] = &c.cacheHierarchy.L2.Stats
	}

	return stats
}

// IsHalted returns true if a halt instruction has completed.
func (c *Core) IsHalted() bool {
	return c.halted
}

// ReadRegister reads a general-purpose register.
func (c *Core) ReadRegister(index int) int {
	return c.regFile.Read(index)
}

// WriteRegister writes a general-purpose register.
func (c *Core) WriteRegister(index int, value int) {
	c.regFile.Write(index, value)
}

// RegisterFile returns the core's register file (for direct access).
func (c *Core) RegisterFile() *RegisterFile {
	return c.regFile
}

// MemoryController returns the core's memory controller (for direct access).
func (c *Core) MemoryController() *MemoryController {
	return c.memCtrl
}

// Cycle returns the current cycle number.
func (c *Core) Cycle() int {
	return c.cycle
}

// Config returns the core configuration.
func (c *Core) Config() CoreConfig {
	return c.config
}

// Pipeline returns the underlying pipeline (for advanced inspection).
func (c *Core) Pipeline() *cpupipeline.Pipeline {
	return c.pipeline
}

// Predictor returns the branch predictor (for inspection).
func (c *Core) Predictor() branchpredictor.BranchPredictor {
	return c.predictor
}

// CacheHierarchy returns the cache hierarchy (for inspection).
func (c *Core) CacheHierarchy() *cache.CacheHierarchy {
	return c.cacheHierarchy
}

// =========================================================================
// Helpers for encoding programs as byte slices
// =========================================================================

// EncodeProgram converts a sequence of raw instruction ints into a byte slice
// suitable for LoadProgram.
//
// Each instruction is encoded as 4 bytes in little-endian order.
//
// Example:
//
//	program := EncodeProgram(EncodeADDI(1, 0, 42), EncodeHALT())
//	core.LoadProgram(program, 0)
func EncodeProgram(instructions ...int) []byte {
	result := make([]byte, len(instructions)*4)
	for i, instr := range instructions {
		offset := i * 4
		result[offset] = byte(instr & 0xFF)
		result[offset+1] = byte((instr >> 8) & 0xFF)
		result[offset+2] = byte((instr >> 16) & 0xFF)
		result[offset+3] = byte((instr >> 24) & 0xFF)
	}
	return result
}
