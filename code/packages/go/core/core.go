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
	result, err := StartNew[*Core]("core.NewCore", nil,
		func(op *Operation[*Core], rf *ResultFactory[*Core]) *OperationResult[*Core] {
			op.AddProperty("config_name", config.Name)
			c := &Core{
				config:  config,
				decoder: decoder,
			}

			c.regFile = NewRegisterFile(config.RegisterFile)

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

			c.cacheHierarchy = c.buildCacheHierarchy(config, memLatency)

			c.predictor = createBranchPredictor(config.BranchPredictorType, config.BranchPredictorSize)
			btbSize := config.BTBSize
			if btbSize <= 0 {
				btbSize = 64
			}
			c.btb = branchpredictor.NewBranchTargetBuffer(btbSize)

			numFPUnits := 0
			if config.FPUnit != nil {
				numFPUnits = 1
			}
			c.hazardUnit = hazarddetection.NewHazardUnit(1, numFPUnits, true)

			pipelineConfig := config.Pipeline
			if len(pipelineConfig.Stages) == 0 {
				pipelineConfig = cpupipeline.Classic5Stage()
			}

			pipeline, pipelineErr := cpupipeline.NewPipeline(
				pipelineConfig,
				c.fetchCallback,
				c.decodeCallback,
				c.executeCallback,
				c.memoryCallback,
				c.writebackCallback,
			)
			if pipelineErr != nil {
				return rf.Fail(nil, pipelineErr)
			}

			if config.HazardDetection {
				pipeline.SetHazardFunc(c.hazardCallback)
			}
			pipeline.SetPredictFunc(c.predictCallback)

			c.pipeline = pipeline
			c.clk = clock.New(1000000000)

			return rf.Generate(true, false, c)
		}).GetResult()
	return result, err
}

// buildCacheHierarchy creates the L1I, L1D, and optional L2 caches.
func (c *Core) buildCacheHierarchy(config CoreConfig, memLatency int) *cache.CacheHierarchy {
	l1iCfg := config.L1ICache
	if l1iCfg == nil {
		defaultCfg := cache.CacheConfig{
			Name: "L1I", TotalSize: 4096, LineSize: 64,
			Associativity: 1, AccessLatency: 1, WritePolicy: "write-back",
		}
		l1iCfg = &defaultCfg
	}
	l1i := cache.NewCache(*l1iCfg)

	l1dCfg := config.L1DCache
	if l1dCfg == nil {
		defaultCfg := cache.CacheConfig{
			Name: "L1D", TotalSize: 4096, LineSize: 64,
			Associativity: 1, AccessLatency: 1, WritePolicy: "write-back",
		}
		l1dCfg = &defaultCfg
	}
	l1d := cache.NewCache(*l1dCfg)

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
func (c *Core) fetchCallback(pc int) int {
	c.cacheHierarchy.Read(pc, true, c.cycle)
	return c.memCtrl.ReadWord(pc)
}

// decodeCallback is called by the pipeline's ID stage.
func (c *Core) decodeCallback(raw int, token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	return c.decoder.Decode(raw, token)
}

// executeCallback is called by the pipeline's EX stage.
func (c *Core) executeCallback(token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	result := c.decoder.Execute(token, c.regFile)

	if result.IsBranch {
		c.predictor.Update(result.PC, result.BranchTaken, result.BranchTarget)
		if result.BranchTaken {
			c.btb.Update(result.PC, result.BranchTarget, "conditional")
		}
	}

	return result
}

// memoryCallback is called by the pipeline's MEM stage.
func (c *Core) memoryCallback(token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	if token.MemRead {
		c.cacheHierarchy.Read(token.ALUResult, false, c.cycle)
		token.MemData = c.memCtrl.ReadWord(token.ALUResult)
		token.WriteData = token.MemData
	} else if token.MemWrite {
		data := []int{token.WriteData & 0xFF}
		c.cacheHierarchy.Write(token.ALUResult, data, c.cycle)
		c.memCtrl.WriteWord(token.ALUResult, token.WriteData)
	}
	return token
}

// writebackCallback is called by the pipeline's WB stage.
func (c *Core) writebackCallback(token *cpupipeline.PipelineToken) {
	if token.RegWrite && token.Rd >= 0 {
		c.regFile.Write(token.Rd, token.WriteData)
	}
}

// hazardCallback is called at the start of each cycle to check for hazards.
func (c *Core) hazardCallback(stages []*cpupipeline.PipelineToken) cpupipeline.HazardResponse {
	numStages := len(stages)
	pipelineCfg := c.config.Pipeline
	if len(pipelineCfg.Stages) == 0 {
		pipelineCfg = cpupipeline.Classic5Stage()
	}

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

	ifSlot := tokenToSlot(ifTok)
	idSlot := tokenToSlot(idTok)
	exSlot := tokenToSlot(exTok)
	memSlot := tokenToSlot(memTok)

	result := c.hazardUnit.Check(ifSlot, idSlot, exSlot, memSlot)

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
func (c *Core) predictCallback(pc int) int {
	prediction := c.predictor.Predict(pc)
	instrSize := c.decoder.InstructionSize()

	if prediction.Taken {
		target := c.btb.Lookup(pc)
		if target != branchpredictor.NoTarget {
			return target
		}
	}

	return pc + instrSize
}

// tokenToSlot converts a PipelineToken to a hazard-detection PipelineSlot.
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
		UsesALU:  true,
	}

	if tok.Rs1 >= 0 {
		slot.SourceRegs = append(slot.SourceRegs, tok.Rs1)
	}
	if tok.Rs2 >= 0 {
		slot.SourceRegs = append(slot.SourceRegs, tok.Rs2)
	}

	if tok.Rd >= 0 && tok.RegWrite {
		slot.DestReg = hazarddetection.IntPtr(tok.Rd)
		if tok.ALUResult != 0 || tok.WriteData != 0 {
			val := tok.ALUResult
			if tok.WriteData != 0 {
				val = tok.WriteData
			}
			slot.DestValue = hazarddetection.IntPtr(val)
		}
	}

	if tok.IsBranch {
		slot.BranchTaken = tok.BranchTaken
		slot.BranchPredictedTaken = false
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
	_, _ = StartNew[struct{}]("core.Core.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("start_address", startAddress)
			op.AddProperty("program_size", len(program))
			c.memCtrl.LoadProgram(program, startAddress)
			c.pipeline.SetPC(startAddress)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
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
	result, _ := StartNew[cpupipeline.PipelineSnapshot]("core.Core.Step", cpupipeline.PipelineSnapshot{},
		func(op *Operation[cpupipeline.PipelineSnapshot], rf *ResultFactory[cpupipeline.PipelineSnapshot]) *OperationResult[cpupipeline.PipelineSnapshot] {
			if c.halted {
				return rf.Generate(true, false, c.pipeline.Snapshot())
			}

			c.cycle++
			snap := c.pipeline.Step()

			if c.pipeline.IsHalted() {
				c.halted = true
			}

			c.instructionsCompleted = c.pipeline.Stats().InstructionsCompleted

			return rf.Generate(true, false, snap)
		}).GetResult()
	return result
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
	result, _ := StartNew[CoreStats]("core.Core.Run", CoreStats{},
		func(op *Operation[CoreStats], rf *ResultFactory[CoreStats]) *OperationResult[CoreStats] {
			op.AddProperty("max_cycles", maxCycles)
			for c.cycle < maxCycles && !c.halted {
				c.Step()
			}
			return rf.Generate(true, false, c.Stats())
		}).GetResult()
	return result
}

// Stats returns aggregate statistics from all sub-components.
//
// This collects stats from the pipeline, branch predictor, hazard unit,
// and cache hierarchy into a single CoreStats struct.
func (c *Core) Stats() CoreStats {
	result, _ := StartNew[CoreStats]("core.Core.Stats", CoreStats{},
		func(op *Operation[CoreStats], rf *ResultFactory[CoreStats]) *OperationResult[CoreStats] {
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

			if c.cacheHierarchy.L1I != nil {
				stats.CacheStats["L1I"] = &c.cacheHierarchy.L1I.Stats
			}
			if c.cacheHierarchy.L1D != nil {
				stats.CacheStats["L1D"] = &c.cacheHierarchy.L1D.Stats
			}
			if c.cacheHierarchy.L2 != nil {
				stats.CacheStats["L2"] = &c.cacheHierarchy.L2.Stats
			}

			return rf.Generate(true, false, stats)
		}).GetResult()
	return result
}

// IsHalted returns true if a halt instruction has completed.
func (c *Core) IsHalted() bool {
	result, _ := StartNew[bool]("core.Core.IsHalted", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, c.halted)
		}).GetResult()
	return result
}

// ReadRegister reads a general-purpose register.
func (c *Core) ReadRegister(index int) int {
	result, _ := StartNew[int]("core.Core.ReadRegister", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("index", index)
			return rf.Generate(true, false, c.regFile.Read(index))
		}).GetResult()
	return result
}

// WriteRegister writes a general-purpose register.
func (c *Core) WriteRegister(index int, value int) {
	_, _ = StartNew[struct{}]("core.Core.WriteRegister", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("index", index)
			c.regFile.Write(index, value)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RegisterFile returns the core's register file (for direct access).
func (c *Core) RegisterFile() *RegisterFile {
	result, _ := StartNew[*RegisterFile]("core.Core.RegisterFile", nil,
		func(op *Operation[*RegisterFile], rf *ResultFactory[*RegisterFile]) *OperationResult[*RegisterFile] {
			return rf.Generate(true, false, c.regFile)
		}).GetResult()
	return result
}

// MemoryController returns the core's memory controller (for direct access).
func (c *Core) MemoryController() *MemoryController {
	result, _ := StartNew[*MemoryController]("core.Core.MemoryController", nil,
		func(op *Operation[*MemoryController], rf *ResultFactory[*MemoryController]) *OperationResult[*MemoryController] {
			return rf.Generate(true, false, c.memCtrl)
		}).GetResult()
	return result
}

// Cycle returns the current cycle number.
func (c *Core) Cycle() int {
	result, _ := StartNew[int]("core.Core.Cycle", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, c.cycle)
		}).GetResult()
	return result
}

// Config returns the core configuration.
func (c *Core) Config() CoreConfig {
	result, _ := StartNew[CoreConfig]("core.Core.Config", CoreConfig{},
		func(op *Operation[CoreConfig], rf *ResultFactory[CoreConfig]) *OperationResult[CoreConfig] {
			return rf.Generate(true, false, c.config)
		}).GetResult()
	return result
}

// Pipeline returns the underlying pipeline (for advanced inspection).
func (c *Core) Pipeline() *cpupipeline.Pipeline {
	result, _ := StartNew[*cpupipeline.Pipeline]("core.Core.Pipeline", nil,
		func(op *Operation[*cpupipeline.Pipeline], rf *ResultFactory[*cpupipeline.Pipeline]) *OperationResult[*cpupipeline.Pipeline] {
			return rf.Generate(true, false, c.pipeline)
		}).GetResult()
	return result
}

// Predictor returns the branch predictor (for inspection).
func (c *Core) Predictor() branchpredictor.BranchPredictor {
	result, _ := StartNew[branchpredictor.BranchPredictor]("core.Core.Predictor", nil,
		func(op *Operation[branchpredictor.BranchPredictor], rf *ResultFactory[branchpredictor.BranchPredictor]) *OperationResult[branchpredictor.BranchPredictor] {
			return rf.Generate(true, false, c.predictor)
		}).GetResult()
	return result
}

// CacheHierarchy returns the cache hierarchy (for inspection).
func (c *Core) CacheHierarchy() *cache.CacheHierarchy {
	result, _ := StartNew[*cache.CacheHierarchy]("core.Core.CacheHierarchy", nil,
		func(op *Operation[*cache.CacheHierarchy], rf *ResultFactory[*cache.CacheHierarchy]) *OperationResult[*cache.CacheHierarchy] {
			return rf.Generate(true, false, c.cacheHierarchy)
		}).GetResult()
	return result
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
	result, _ := StartNew[[]byte]("core.EncodeProgram", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("num_instructions", len(instructions))
			encoded := make([]byte, len(instructions)*4)
			for i, instr := range instructions {
				offset := i * 4
				encoded[offset] = byte(instr & 0xFF)
				encoded[offset+1] = byte((instr >> 8) & 0xFF)
				encoded[offset+2] = byte((instr >> 16) & 0xFF)
				encoded[offset+3] = byte((instr >> 24) & 0xFF)
			}
			return rf.Generate(true, false, encoded)
		}).GetResult()
	return result
}
