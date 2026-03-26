package cpupipeline

import (
	"math"
	"testing"
)

// =========================================================================
// Test helpers -- simple instruction memory and callbacks
// =========================================================================
//
// For testing, we create a tiny "instruction memory" -- just a slice of
// integers. Each integer represents one instruction's raw bits. The fetch
// callback reads from this slice using PC/4 as the index.
//
// The decode callback creates simple instructions:
//   - opcode 0x01 = ADD (register write)
//   - opcode 0x02 = LDR (load from memory, register write)
//   - opcode 0x03 = STR (store to memory)
//   - opcode 0x04 = BEQ (branch if equal)
//   - opcode 0xFF = HALT
//   - opcode 0x00 = NOP
//
// Encoding: raw = (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2
//
// This encoding is deliberately simple -- the focus is on testing the
// pipeline mechanics, not instruction decoding.

// makeInstruction encodes a test instruction.
//
//	opcode: 8 bits (bits 31-24)
//	rd:     8 bits (bits 23-16)
//	rs1:    8 bits (bits 15-8)
//	rs2:    8 bits (bits 7-0)
func makeInstruction(opcode, rd, rs1, rs2 int) int {
	return (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2
}

// Test opcode constants.
const (
	opNOP  = 0x00
	opADD  = 0x01
	opLDR  = 0x02
	opSTR  = 0x03
	opBEQ  = 0x04
	opHALT = 0xFF
)

// simpleFetch returns a fetch function that reads from the given instruction memory.
//
// If the PC is out of bounds, returns a NOP. This prevents crashes when
// the pipeline fetches past the end of the program.
func simpleFetch(instrs []int) FetchFunc {
	return func(pc int) int {
		idx := pc / 4
		if idx < 0 || idx >= len(instrs) {
			return makeInstruction(opNOP, 0, 0, 0) // NOP
		}
		return instrs[idx]
	}
}

// simpleDecode returns a decode function that parses our test encoding.
func simpleDecode() DecodeFunc {
	return func(raw int, tok *PipelineToken) *PipelineToken {
		opcode := (raw >> 24) & 0xFF
		rd := (raw >> 16) & 0xFF
		rs1 := (raw >> 8) & 0xFF
		rs2 := raw & 0xFF

		switch opcode {
		case opADD:
			tok.Opcode = "ADD"
			tok.Rd = rd
			tok.Rs1 = rs1
			tok.Rs2 = rs2
			tok.RegWrite = true
		case opLDR:
			tok.Opcode = "LDR"
			tok.Rd = rd
			tok.Rs1 = rs1
			tok.MemRead = true
			tok.RegWrite = true
		case opSTR:
			tok.Opcode = "STR"
			tok.Rs1 = rs1
			tok.Rs2 = rs2
			tok.MemWrite = true
		case opBEQ:
			tok.Opcode = "BEQ"
			tok.Rs1 = rs1
			tok.Rs2 = rs2
			tok.IsBranch = true
		case opHALT:
			tok.Opcode = "HALT"
			tok.IsHalt = true
		default:
			tok.Opcode = "NOP"
		}
		return tok
	}
}

// simpleExecute returns an execute callback that sets ALUResult.
func simpleExecute() ExecuteFunc {
	return func(tok *PipelineToken) *PipelineToken {
		switch tok.Opcode {
		case "ADD":
			tok.ALUResult = tok.Rs1 + tok.Rs2 // Simplified: use register numbers as values
		case "LDR":
			tok.ALUResult = tok.Rs1 + tok.Immediate // Address calculation
		case "STR":
			tok.ALUResult = tok.Rs1 + tok.Immediate
		case "BEQ":
			tok.BranchTarget = tok.PC + tok.Immediate
		}
		return tok
	}
}

// simpleMemory returns a memory callback that handles loads.
func simpleMemory() MemoryFunc {
	return func(tok *PipelineToken) *PipelineToken {
		if tok.MemRead {
			tok.MemData = 42 // Return a fixed value for testing
			tok.WriteData = tok.MemData
		} else {
			tok.WriteData = tok.ALUResult
		}
		return tok
	}
}

// completedInstructions tracks which instructions were written back.
type completedInstructions struct {
	pcs []int
}

// simpleWriteback returns a writeback callback that records completed instructions.
func simpleWriteback(completed *completedInstructions) WritebackFunc {
	return func(tok *PipelineToken) {
		if completed != nil {
			completed.pcs = append(completed.pcs, tok.PC)
		}
	}
}

// newTestPipeline creates a 5-stage pipeline with simple test callbacks.
func newTestPipeline(instrs []int, completed *completedInstructions) *Pipeline {
	config := Classic5Stage()
	p, err := NewPipeline(
		config,
		simpleFetch(instrs),
		simpleDecode(),
		simpleExecute(),
		simpleMemory(),
		simpleWriteback(completed),
	)
	if err != nil {
		panic(err)
	}
	return p
}

// =========================================================================
// Token tests
// =========================================================================

func TestNewToken(t *testing.T) {
	tok := NewToken()
	if tok.Rs1 != -1 || tok.Rs2 != -1 || tok.Rd != -1 {
		t.Errorf("expected default register values of -1, got Rs1=%d Rs2=%d Rd=%d",
			tok.Rs1, tok.Rs2, tok.Rd)
	}
	if tok.IsBubble {
		t.Error("new token should not be a bubble")
	}
	if tok.StageEntered == nil {
		t.Error("StageEntered map should be initialized")
	}
}

func TestNewBubble(t *testing.T) {
	b := NewBubble()
	if !b.IsBubble {
		t.Error("bubble should have IsBubble = true")
	}
	if b.String() != "---" {
		t.Errorf("bubble string should be '---', got %q", b.String())
	}
}

func TestTokenString(t *testing.T) {
	tok := NewToken()
	tok.Opcode = "ADD"
	tok.PC = 100
	s := tok.String()
	if s != "ADD@100" {
		t.Errorf("expected 'ADD@100', got %q", s)
	}

	tok2 := NewToken()
	tok2.PC = 200
	s2 := tok2.String()
	if s2 != "instr@200" {
		t.Errorf("expected 'instr@200', got %q", s2)
	}
}

func TestTokenClone(t *testing.T) {
	tok := NewToken()
	tok.PC = 100
	tok.Opcode = "ADD"
	tok.StageEntered["IF"] = 1
	tok.StageEntered["ID"] = 2

	clone := tok.Clone()
	if clone.PC != 100 || clone.Opcode != "ADD" {
		t.Error("clone should have same field values")
	}

	// Mutating the clone should not affect the original.
	clone.StageEntered["EX"] = 3
	if _, ok := tok.StageEntered["EX"]; ok {
		t.Error("modifying clone's StageEntered should not affect original")
	}
}

func TestTokenCloneNil(t *testing.T) {
	var tok *PipelineToken
	clone := tok.Clone()
	if clone != nil {
		t.Error("cloning nil should return nil")
	}
}

// =========================================================================
// PipelineConfig tests
// =========================================================================

func TestClassic5Stage(t *testing.T) {
	config := Classic5Stage()
	if config.NumStages() != 5 {
		t.Errorf("expected 5 stages, got %d", config.NumStages())
	}
	if err := config.Validate(); err != nil {
		t.Errorf("classic 5-stage should be valid: %v", err)
	}
	if config.Stages[0].Name != "IF" {
		t.Errorf("first stage should be IF, got %s", config.Stages[0].Name)
	}
	if config.Stages[4].Name != "WB" {
		t.Errorf("last stage should be WB, got %s", config.Stages[4].Name)
	}
}

func TestDeep13Stage(t *testing.T) {
	config := Deep13Stage()
	if config.NumStages() != 13 {
		t.Errorf("expected 13 stages, got %d", config.NumStages())
	}
	if err := config.Validate(); err != nil {
		t.Errorf("deep 13-stage should be valid: %v", err)
	}
}

func TestConfigValidation(t *testing.T) {
	// Too few stages.
	cfg := PipelineConfig{
		Stages:         []PipelineStage{{Name: "IF", Category: StageFetch}},
		ExecutionWidth: 1,
	}
	if err := cfg.Validate(); err == nil {
		t.Error("expected error for 1-stage pipeline")
	}

	// Zero execution width.
	cfg2 := PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF", Category: StageFetch},
			{Name: "WB", Category: StageWriteback},
		},
		ExecutionWidth: 0,
	}
	if err := cfg2.Validate(); err == nil {
		t.Error("expected error for zero execution width")
	}

	// Duplicate stage names.
	cfg3 := PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF", Category: StageFetch},
			{Name: "IF", Category: StageWriteback},
		},
		ExecutionWidth: 1,
	}
	if err := cfg3.Validate(); err == nil {
		t.Error("expected error for duplicate stage names")
	}

	// No fetch stage.
	cfg4 := PipelineConfig{
		Stages: []PipelineStage{
			{Name: "EX", Category: StageExecute},
			{Name: "WB", Category: StageWriteback},
		},
		ExecutionWidth: 1,
	}
	if err := cfg4.Validate(); err == nil {
		t.Error("expected error for no fetch stage")
	}

	// No writeback stage.
	cfg5 := PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF", Category: StageFetch},
			{Name: "EX", Category: StageExecute},
		},
		ExecutionWidth: 1,
	}
	if err := cfg5.Validate(); err == nil {
		t.Error("expected error for no writeback stage")
	}

	// Valid 2-stage pipeline.
	cfg6 := PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF", Category: StageFetch},
			{Name: "WB", Category: StageWriteback},
		},
		ExecutionWidth: 1,
	}
	if err := cfg6.Validate(); err != nil {
		t.Errorf("expected valid 2-stage pipeline, got error: %v", err)
	}
}

// =========================================================================
// Basic Pipeline tests
// =========================================================================

func TestNewPipeline(t *testing.T) {
	instrs := []int{makeInstruction(opADD, 1, 2, 3)}
	p := newTestPipeline(instrs, nil)

	if p.IsHalted() {
		t.Error("new pipeline should not be halted")
	}
	if p.Cycle() != 0 {
		t.Errorf("new pipeline should be at cycle 0, got %d", p.Cycle())
	}
	if p.PC() != 0 {
		t.Errorf("new pipeline should have PC=0, got %d", p.PC())
	}
}

func TestNewPipelineInvalidConfig(t *testing.T) {
	cfg := PipelineConfig{
		Stages:         []PipelineStage{{Name: "IF", Category: StageFetch}},
		ExecutionWidth: 1,
	}
	_, err := NewPipeline(cfg, nil, nil, nil, nil, nil)
	if err == nil {
		t.Error("expected error for invalid config")
	}
}

// TestSingleInstructionFlowsThrough5Stages verifies that a single instruction
// progresses through all 5 stages in 5 cycles.
//
// Timeline:
//
//	Cycle 1: ADD enters IF
//	Cycle 2: ADD enters ID
//	Cycle 3: ADD enters EX
//	Cycle 4: ADD enters MEM
//	Cycle 5: ADD enters WB and retires
func TestSingleInstructionFlowsThrough5Stages(t *testing.T) {
	// Program: one ADD followed by NOPs.
	instrs := []int{
		makeInstruction(opADD, 1, 2, 3),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
	}

	completed := &completedInstructions{}
	p := newTestPipeline(instrs, completed)

	// Step 5 times -- the ADD should complete at cycle 5.
	for i := 0; i < 5; i++ {
		p.Step()
	}

	if len(completed.pcs) == 0 {
		t.Fatal("expected at least one instruction to complete after 5 cycles")
	}
	if completed.pcs[0] != 0 {
		t.Errorf("expected first completed instruction at PC=0, got PC=%d", completed.pcs[0])
	}
}

// TestPipelineFillTiming verifies that the first instruction completes
// at exactly cycle 5 (for a 5-stage pipeline), and subsequent instructions
// complete one per cycle.
//
// Timeline:
//
//	Cycle:  1    2    3    4    5    6    7
//	IF:    I1   I2   I3   I4   I5   I6   I7
//	ID:    --   I1   I2   I3   I4   I5   I6
//	EX:    --   --   I1   I2   I3   I4   I5
//	MEM:   --   --   --   I1   I2   I3   I4
//	WB:    --   --   --   --   I1   I2   I3
//	                          ^first  ^second  ^third
func TestPipelineFillTiming(t *testing.T) {
	instrs := make([]int, 20)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	completed := &completedInstructions{}
	p := newTestPipeline(instrs, completed)

	// After 4 cycles, nothing should have completed yet.
	for i := 0; i < 4; i++ {
		p.Step()
	}
	if len(completed.pcs) != 0 {
		t.Errorf("expected 0 completions after 4 cycles, got %d", len(completed.pcs))
	}

	// After cycle 5, exactly 1 instruction should have completed.
	p.Step()
	if len(completed.pcs) != 1 {
		t.Errorf("expected 1 completion after 5 cycles, got %d", len(completed.pcs))
	}

	// After cycle 6, 2 completions. After cycle 7, 3 completions. etc.
	p.Step()
	if len(completed.pcs) != 2 {
		t.Errorf("expected 2 completions after 6 cycles, got %d", len(completed.pcs))
	}

	p.Step()
	if len(completed.pcs) != 3 {
		t.Errorf("expected 3 completions after 7 cycles, got %d", len(completed.pcs))
	}
}

// TestSteadyStateIPC verifies that after the pipeline fills, the IPC
// approaches 1.0 for a stream of independent instructions.
func TestSteadyStateIPC(t *testing.T) {
	// 100 independent ADD instructions.
	instrs := make([]int, 100)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	// Run for 50 cycles.
	for i := 0; i < 50; i++ {
		p.Step()
	}

	stats := p.Stats()
	// After 50 cycles of a 5-stage pipeline, we should have completed 46 instructions
	// (50 - 4 for the fill latency, but the first completes at cycle 5).
	// Completed = 50 - 5 + 1 = 46
	expectedCompleted := 50 - 5 + 1
	if stats.InstructionsCompleted != expectedCompleted {
		t.Errorf("expected %d completions after 50 cycles, got %d",
			expectedCompleted, stats.InstructionsCompleted)
	}

	ipc := stats.IPC()
	// IPC should be close to 1.0 (slightly less because of fill latency).
	if ipc < 0.85 || ipc > 1.01 {
		t.Errorf("expected IPC near 1.0, got %.3f", ipc)
	}
}

// TestHaltPropagation verifies that a HALT instruction eventually reaches
// the WB stage and stops the pipeline.
//
// Program: ADD, ADD, HALT
// The HALT is at PC=8. It enters IF at cycle 3, reaches WB at cycle 7.
func TestHaltPropagation(t *testing.T) {
	instrs := []int{
		makeInstruction(opADD, 1, 2, 3),
		makeInstruction(opADD, 4, 5, 6),
		makeInstruction(opHALT, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
	}

	completed := &completedInstructions{}
	p := newTestPipeline(instrs, completed)

	// Run until halted or max cycles.
	stats := p.Run(100)

	if !p.IsHalted() {
		t.Error("pipeline should be halted after HALT instruction")
	}

	// The HALT is at index 2 (PC=8), fetched at cycle 3.
	// It should reach WB at cycle 7.
	if p.Cycle() != 7 {
		t.Errorf("expected halt at cycle 7, got cycle %d", p.Cycle())
	}

	// Two ADD instructions and one HALT should have completed.
	if stats.InstructionsCompleted != 3 {
		t.Errorf("expected 3 completions (2 ADD + 1 HALT), got %d",
			stats.InstructionsCompleted)
	}
}

// TestEmptyPipeline verifies that stepping an empty pipeline (no program) works.
func TestEmptyPipeline(t *testing.T) {
	instrs := []int{} // empty program
	p := newTestPipeline(instrs, nil)

	// Should not panic.
	snap := p.Step()
	if snap.Cycle != 1 {
		t.Errorf("expected cycle 1, got %d", snap.Cycle)
	}
}

// =========================================================================
// Stall tests
// =========================================================================

// TestStallFreezesEarlierStages verifies that during a stall, the IF and ID
// stages are frozen (contain the same tokens) and a bubble is inserted at EX.
func TestStallFreezesEarlierStages(t *testing.T) {
	instrs := []int{
		makeInstruction(opLDR, 1, 2, 0), // LDR R1, [R2]
		makeInstruction(opADD, 3, 1, 4), // ADD R3, R1, R4 (depends on LDR result)
		makeInstruction(opADD, 5, 6, 7),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
	}

	completed := &completedInstructions{}
	p := newTestPipeline(instrs, completed)

	// Set up a hazard function that detects the load-use hazard.
	//
	// Load-use hazard: LDR is in EX (stage 2), ADD that reads LDR's
	// destination is in ID (stage 1). We need to stall for 1 cycle.
	stallInjected := false
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		// Only stall once: when LDR is in EX and ADD is in ID.
		if !stallInjected && len(stages) >= 3 {
			exTok := stages[2]  // EX stage
			idTok := stages[1]  // ID stage
			if exTok != nil && !exTok.IsBubble && exTok.Opcode == "LDR" &&
				idTok != nil && !idTok.IsBubble && idTok.Opcode == "ADD" {
				stallInjected = true
				return HazardResponse{
					Action:      HazardStall,
					StallStages: 2, // Insert bubble at EX (stage index 2)
				}
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	// Step enough cycles to see the stall.
	// Cycle 1: IF=LDR
	// Cycle 2: IF=ADD, ID=LDR
	// Cycle 3: IF=I3, ID=ADD, EX=LDR -> hazard detected, stall at cycle 3
	//   But stall is detected at the START of the cycle. Let's trace carefully.
	//   Actually, the hazard check looks at the current state BEFORE advancement.
	//   At the start of cycle 3, stages are:
	//     IF=I3 (just fetched), ID=ADD, EX=LDR
	//   Wait -- after 2 Steps, the state is:
	//     stages[0] (IF) = ADD (fetched at cycle 2)
	//     stages[1] (ID) = LDR (moved from IF to ID at cycle 2)
	//     stages[2] (EX) = nil (nothing there yet at cycle 2)
	//
	// Let me re-think. After Step 1: stages[0]=I1(LDR)
	// After Step 2: stages[0]=I2(ADD), stages[1]=I1(LDR)
	// At Step 3: hazard checks stages. EX (stages[2]) has nothing yet... so no stall.
	// After Step 3: stages[0]=I3, stages[1]=I2(ADD), stages[2]=I1(LDR)
	// At Step 4: hazard checks stages. EX has LDR, ID has ADD -> STALL!

	p.Step() // cycle 1
	p.Step() // cycle 2
	p.Step() // cycle 3

	// Now at the start of cycle 4, LDR should be in EX and ADD in ID.
	snap := p.Step() // cycle 4 -- stall should occur here

	if !snap.Stalled {
		t.Error("expected pipeline to be stalled at cycle 4")
	}

	// After the stall:
	// - IF and ID should be frozen (same tokens as before)
	// - EX should have a bubble
	exTok := p.StageContents("EX")
	if exTok == nil || !exTok.IsBubble {
		t.Error("expected bubble in EX stage after stall")
	}

	// ID should still contain the ADD instruction (frozen).
	idTok := p.StageContents("ID")
	if idTok == nil || idTok.Opcode != "ADD" {
		t.Error("expected ADD to remain in ID stage (frozen)")
	}

	stats := p.Stats()
	if stats.StallCycles != 1 {
		t.Errorf("expected 1 stall cycle, got %d", stats.StallCycles)
	}
}

// TestStallBubbleInsertion verifies that a bubble is inserted into the correct
// stage during a stall.
func TestStallBubbleInsertion(t *testing.T) {
	instrs := make([]int, 10)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	// Force a stall at cycle 3.
	stallCount := 0
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		stallCount++
		if stallCount == 3 {
			return HazardResponse{
				Action:      HazardStall,
				StallStages: 2, // Insert bubble at stage index 2 (EX)
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	for i := 0; i < 3; i++ {
		p.Step()
	}

	// After the stall at cycle 3, EX should have a bubble.
	exTok := p.StageContents("EX")
	if exTok == nil || !exTok.IsBubble {
		t.Error("expected bubble in EX after stall")
	}
}

// =========================================================================
// Flush tests
// =========================================================================

// TestFlushReplacesWithBubbles verifies that a flush replaces speculative
// stages with bubbles and redirects the PC.
func TestFlushReplacesWithBubbles(t *testing.T) {
	instrs := []int{
		makeInstruction(opBEQ, 0, 1, 2), // branch at PC=0
		makeInstruction(opADD, 1, 2, 3), // speculative (wrong path)
		makeInstruction(opADD, 4, 5, 6), // speculative (wrong path)
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		// ... target instructions at PC=20
		makeInstruction(opADD, 7, 8, 9), // PC=20 -- correct path
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
	}

	p := newTestPipeline(instrs, nil)

	// Flush when the branch reaches EX stage (cycle 3).
	flushed := false
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		if !flushed && len(stages) >= 3 {
			exTok := stages[2]
			if exTok != nil && !exTok.IsBubble && exTok.IsBranch {
				flushed = true
				return HazardResponse{
					Action:     HazardFlush,
					FlushCount: 2, // Flush IF and ID (2 stages)
					RedirectPC: 20,
				}
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	// Run until the flush happens.
	p.Step() // cycle 1: IF=BEQ
	p.Step() // cycle 2: IF=ADD, ID=BEQ
	p.Step() // cycle 3: IF=ADD2, ID=ADD, EX=BEQ -> flush!

	snap := p.Step() // cycle 4 -- flush should occur
	if !snap.Flushing {
		t.Error("expected flush at cycle 4")
	}

	// After flush, PC should be redirected to 20.
	if p.PC() != 24 { // 20 + 4 (advanced by fetch)
		t.Errorf("expected PC=24 after flush, got %d", p.PC())
	}

	stats := p.Stats()
	if stats.FlushCycles != 1 {
		t.Errorf("expected 1 flush cycle, got %d", stats.FlushCycles)
	}
}

// =========================================================================
// Forwarding integration tests
// =========================================================================

// TestForwardingApplied verifies that the forwarding callback updates the
// token with the forwarded value and records the source.
//
// Note: the forwarded value is set on the token's ALUResult before it
// enters the EX stage. The execute callback then overwrites ALUResult
// with its own computation. In real hardware, the forwarded value would
// be selected as an ALU *input* (via a mux), not as the ALU output.
// What we verify here is that the ForwardedFrom metadata is preserved,
// confirming the forwarding path was activated.
func TestForwardingApplied(t *testing.T) {
	instrs := make([]int, 10)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	// Inject a forward at cycle 4 (when there are tokens in both ID and EX).
	forwardCycle := 0
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		forwardCycle++
		if forwardCycle == 4 {
			return HazardResponse{
				Action:        HazardForwardFromEX,
				ForwardValue:  99,
				ForwardSource: "EX",
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	for i := 0; i < 4; i++ {
		p.Step()
	}

	// The token that was in ID during the forward should have ForwardedFrom set.
	// After 4 steps, the forwarded token has moved from ID to EX.
	exTok := p.StageContents("EX")
	if exTok == nil {
		t.Fatal("expected token in EX stage")
	}
	if exTok.ForwardedFrom != "EX" {
		t.Errorf("expected ForwardedFrom='EX', got %q", exTok.ForwardedFrom)
	}
	// Note: ALUResult is overwritten by the execute callback.
	// The ForwardedFrom field is the durable indicator that forwarding occurred.
}

// =========================================================================
// Statistics tests
// =========================================================================

func TestIPCCalculation(t *testing.T) {
	stats := PipelineStats{
		TotalCycles:           100,
		InstructionsCompleted: 80,
	}
	ipc := stats.IPC()
	if math.Abs(ipc-0.8) > 0.001 {
		t.Errorf("expected IPC=0.8, got %.3f", ipc)
	}
}

func TestCPICalculation(t *testing.T) {
	stats := PipelineStats{
		TotalCycles:           120,
		InstructionsCompleted: 100,
	}
	cpi := stats.CPI()
	if math.Abs(cpi-1.2) > 0.001 {
		t.Errorf("expected CPI=1.2, got %.3f", cpi)
	}
}

func TestIPCZeroCycles(t *testing.T) {
	stats := PipelineStats{}
	if stats.IPC() != 0.0 {
		t.Errorf("expected IPC=0 for zero cycles, got %.3f", stats.IPC())
	}
}

func TestCPIZeroInstructions(t *testing.T) {
	stats := PipelineStats{TotalCycles: 10}
	if stats.CPI() != 0.0 {
		t.Errorf("expected CPI=0 for zero instructions, got %.3f", stats.CPI())
	}
}

func TestStatsString(t *testing.T) {
	stats := PipelineStats{
		TotalCycles:           100,
		InstructionsCompleted: 80,
		StallCycles:           5,
		FlushCycles:           3,
		BubbleCycles:          10,
	}
	s := stats.String()
	if s == "" {
		t.Error("stats string should not be empty")
	}
}

// TestStallReducesIPC verifies that stalls reduce the IPC below 1.0.
func TestStallReducesIPC(t *testing.T) {
	instrs := make([]int, 50)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	// Inject stalls every 5 cycles.
	cycleCount := 0
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		cycleCount++
		if cycleCount%5 == 0 {
			return HazardResponse{
				Action:      HazardStall,
				StallStages: 2,
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	for i := 0; i < 30; i++ {
		p.Step()
	}

	stats := p.Stats()
	ipc := stats.IPC()
	if ipc >= 1.0 {
		t.Errorf("expected IPC < 1.0 with stalls, got %.3f", ipc)
	}
	if stats.StallCycles == 0 {
		t.Error("expected nonzero stall cycles")
	}
}

// =========================================================================
// Trace and Snapshot tests
// =========================================================================

// TestSnapshotAccuracy verifies that snapshots correctly reflect pipeline contents.
func TestSnapshotAccuracy(t *testing.T) {
	instrs := []int{
		makeInstruction(opADD, 1, 2, 3),
		makeInstruction(opADD, 4, 5, 6),
		makeInstruction(opNOP, 0, 0, 0),
	}

	p := newTestPipeline(instrs, nil)

	// After 1 cycle, only IF has a token.
	snap1 := p.Step()
	if snap1.Cycle != 1 {
		t.Errorf("expected cycle 1, got %d", snap1.Cycle)
	}
	ifTok := snap1.Stages["IF"]
	if ifTok == nil {
		t.Fatal("expected token in IF stage at cycle 1")
	}
	if ifTok.PC != 0 {
		t.Errorf("expected IF token PC=0, got %d", ifTok.PC)
	}

	// After 2 cycles, IF has second instruction, ID has first.
	snap2 := p.Step()
	if snap2.Cycle != 2 {
		t.Errorf("expected cycle 2, got %d", snap2.Cycle)
	}
	idTok := snap2.Stages["ID"]
	if idTok == nil {
		t.Fatal("expected token in ID stage at cycle 2")
	}
	if idTok.PC != 0 {
		t.Errorf("expected ID token PC=0 at cycle 2, got %d", idTok.PC)
	}
}

// TestTraceCompleteness verifies that trace records every cycle's state.
func TestTraceCompleteness(t *testing.T) {
	instrs := make([]int, 10)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	for i := 0; i < 7; i++ {
		p.Step()
	}

	trace := p.Trace()
	if len(trace) != 7 {
		t.Errorf("expected 7 trace entries, got %d", len(trace))
	}

	// Verify cycle numbering is sequential.
	for i, snap := range trace {
		expected := i + 1
		if snap.Cycle != expected {
			t.Errorf("trace[%d] should have cycle %d, got %d", i, expected, snap.Cycle)
		}
	}
}

// TestSnapshotDoesNotAdvance verifies that taking a snapshot does not
// modify the pipeline state.
func TestSnapshotDoesNotAdvance(t *testing.T) {
	instrs := []int{makeInstruction(opADD, 1, 2, 3)}
	p := newTestPipeline(instrs, nil)

	p.Step()
	snap1 := p.Snapshot()
	snap2 := p.Snapshot()

	if snap1.Cycle != snap2.Cycle {
		t.Error("snapshot should not advance the cycle counter")
	}
}

// =========================================================================
// Configuration preset tests
// =========================================================================

// TestDeepPipelineLongerFillTime verifies that a deeper pipeline takes
// more cycles to fill and produce the first completion.
func TestDeepPipelineLongerFillTime(t *testing.T) {
	config := Deep13Stage()
	instrs := make([]int, 30)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p, err := NewPipeline(
		config,
		simpleFetch(instrs),
		simpleDecode(),
		simpleExecute(),
		simpleMemory(),
		simpleWriteback(nil),
	)
	if err != nil {
		t.Fatal(err)
	}

	// Run for 12 cycles -- no instruction should have completed yet.
	for i := 0; i < 12; i++ {
		p.Step()
	}
	if p.Stats().InstructionsCompleted != 0 {
		t.Errorf("expected 0 completions after 12 cycles in 13-stage pipeline, got %d",
			p.Stats().InstructionsCompleted)
	}

	// After cycle 13, exactly 1 instruction should have completed.
	p.Step()
	if p.Stats().InstructionsCompleted != 1 {
		t.Errorf("expected 1 completion after 13 cycles in 13-stage pipeline, got %d",
			p.Stats().InstructionsCompleted)
	}
}

// TestCustomStageConfiguration verifies that custom stages work correctly.
func TestCustomStageConfiguration(t *testing.T) {
	// A 3-stage pipeline: IF, EX, WB
	config := PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF", Description: "Fetch", Category: StageFetch},
			{Name: "EX", Description: "Execute", Category: StageExecute},
			{Name: "WB", Description: "Writeback", Category: StageWriteback},
		},
		ExecutionWidth: 1,
	}

	instrs := make([]int, 10)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	completed := &completedInstructions{}
	p, err := NewPipeline(
		config,
		simpleFetch(instrs),
		simpleDecode(),
		simpleExecute(),
		simpleMemory(),
		simpleWriteback(completed),
	)
	if err != nil {
		t.Fatal(err)
	}

	// In a 3-stage pipeline, first completion should be at cycle 3.
	for i := 0; i < 2; i++ {
		p.Step()
	}
	if len(completed.pcs) != 0 {
		t.Errorf("expected 0 completions after 2 cycles in 3-stage pipeline, got %d",
			len(completed.pcs))
	}

	p.Step() // cycle 3
	if len(completed.pcs) != 1 {
		t.Errorf("expected 1 completion after 3 cycles in 3-stage pipeline, got %d",
			len(completed.pcs))
	}
}

// =========================================================================
// Branch prediction integration tests
// =========================================================================

// TestBranchPredictorIntegration verifies that the predict callback is
// used to determine the next PC during fetch.
func TestBranchPredictorIntegration(t *testing.T) {
	instrs := make([]int, 100) // large enough
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	// Set a predictor that always predicts PC+8 (skip one instruction).
	p.SetPredictFunc(func(pc int) int {
		return pc + 8
	})

	p.Step() // cycle 1: fetches PC=0, predicts next=8

	// After the first step, PC should be 8 (not the default 4).
	if p.PC() != 8 {
		t.Errorf("expected PC=8 after prediction, got %d", p.PC())
	}

	p.Step() // cycle 2: fetches PC=8, predicts next=16
	if p.PC() != 16 {
		t.Errorf("expected PC=16 after second prediction, got %d", p.PC())
	}
}

// =========================================================================
// SetPC test
// =========================================================================

func TestSetPC(t *testing.T) {
	instrs := make([]int, 10)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)
	p.SetPC(100)

	if p.PC() != 100 {
		t.Errorf("expected PC=100, got %d", p.PC())
	}
}

// =========================================================================
// Halted pipeline test
// =========================================================================

// TestHaltedPipelineDoesNotAdvance verifies that stepping a halted pipeline
// does not change the cycle count or complete additional instructions.
func TestHaltedPipelineDoesNotAdvance(t *testing.T) {
	instrs := []int{
		makeInstruction(opHALT, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
	}

	p := newTestPipeline(instrs, nil)
	p.Run(100)

	cycleAtHalt := p.Cycle()
	completedAtHalt := p.Stats().InstructionsCompleted

	// Step again -- nothing should change.
	p.Step()
	p.Step()

	if p.Cycle() != cycleAtHalt {
		t.Errorf("expected cycle to stay at %d after halt, got %d",
			cycleAtHalt, p.Cycle())
	}
	if p.Stats().InstructionsCompleted != completedAtHalt {
		t.Errorf("expected completions to stay at %d, got %d",
			completedAtHalt, p.Stats().InstructionsCompleted)
	}
}

// =========================================================================
// StageCategory tests
// =========================================================================

func TestStageCategoryString(t *testing.T) {
	tests := []struct {
		cat  StageCategory
		want string
	}{
		{StageFetch, "fetch"},
		{StageDecode, "decode"},
		{StageExecute, "execute"},
		{StageMemory, "memory"},
		{StageWriteback, "writeback"},
		{StageCategory(99), "unknown"},
	}
	for _, tt := range tests {
		if got := tt.cat.String(); got != tt.want {
			t.Errorf("StageCategory(%d).String() = %q, want %q", tt.cat, got, tt.want)
		}
	}
}

// =========================================================================
// HazardAction tests
// =========================================================================

func TestHazardActionString(t *testing.T) {
	tests := []struct {
		action HazardAction
		want   string
	}{
		{HazardNone, "NONE"},
		{HazardForwardFromEX, "FORWARD_FROM_EX"},
		{HazardForwardFromMEM, "FORWARD_FROM_MEM"},
		{HazardStall, "STALL"},
		{HazardFlush, "FLUSH"},
		{HazardAction(99), "UNKNOWN"},
	}
	for _, tt := range tests {
		if got := tt.action.String(); got != tt.want {
			t.Errorf("HazardAction(%d).String() = %q, want %q", tt.action, got, tt.want)
		}
	}
}

// =========================================================================
// PipelineStage String test
// =========================================================================

func TestPipelineStageString(t *testing.T) {
	stage := PipelineStage{Name: "IF", Description: "Instruction Fetch", Category: StageFetch}
	if stage.String() != "IF" {
		t.Errorf("expected 'IF', got %q", stage.String())
	}
}

// =========================================================================
// PipelineSnapshot String test
// =========================================================================

func TestPipelineSnapshotString(t *testing.T) {
	snap := PipelineSnapshot{Cycle: 7, PC: 28, Stalled: true}
	s := snap.String()
	if s == "" {
		t.Error("snapshot string should not be empty")
	}
}

// =========================================================================
// Config returns test
// =========================================================================

func TestPipelineConfig(t *testing.T) {
	instrs := []int{makeInstruction(opNOP, 0, 0, 0)}
	p := newTestPipeline(instrs, nil)
	cfg := p.Config()
	if cfg.NumStages() != 5 {
		t.Errorf("expected 5 stages from Config(), got %d", cfg.NumStages())
	}
}

// =========================================================================
// Multiple stall and flush cycles test
// =========================================================================

// TestMultipleStallsAndFlushes runs a scenario with both stalls and flushes
// to verify that statistics are tracked correctly.
func TestMultipleStallsAndFlushes(t *testing.T) {
	instrs := make([]int, 50)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	cycleCounter := 0
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		cycleCounter++
		// Stall at cycles 5, 10.
		if cycleCounter == 5 || cycleCounter == 10 {
			return HazardResponse{
				Action:      HazardStall,
				StallStages: 2,
			}
		}
		// Flush at cycle 15.
		if cycleCounter == 15 {
			return HazardResponse{
				Action:     HazardFlush,
				FlushCount: 2,
				RedirectPC: 0,
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	for i := 0; i < 20; i++ {
		p.Step()
	}

	stats := p.Stats()
	if stats.StallCycles != 2 {
		t.Errorf("expected 2 stall cycles, got %d", stats.StallCycles)
	}
	if stats.FlushCycles != 1 {
		t.Errorf("expected 1 flush cycle, got %d", stats.FlushCycles)
	}
}

// =========================================================================
// Run with max cycles test
// =========================================================================

// TestRunMaxCycles verifies that Run stops at the max cycle limit.
func TestRunMaxCycles(t *testing.T) {
	instrs := make([]int, 100)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3) // no HALT
	}

	p := newTestPipeline(instrs, nil)
	stats := p.Run(10)

	if stats.TotalCycles != 10 {
		t.Errorf("expected 10 total cycles, got %d", stats.TotalCycles)
	}
	if p.IsHalted() {
		t.Error("pipeline should not be halted when stopped by max cycles")
	}
}

// =========================================================================
// StageContents test
// =========================================================================

func TestStageContentsInvalidName(t *testing.T) {
	instrs := []int{makeInstruction(opNOP, 0, 0, 0)}
	p := newTestPipeline(instrs, nil)
	p.Step()

	tok := p.StageContents("NONEXISTENT")
	if tok != nil {
		t.Error("expected nil for nonexistent stage name")
	}
}

// =========================================================================
// Flush with default flush count test
// =========================================================================

func TestFlushDefaultFlushCount(t *testing.T) {
	instrs := make([]int, 20)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	flushed := false
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		if !flushed {
			// Wait until EX has a token (cycle 3+).
			if len(stages) >= 3 && stages[2] != nil && !stages[2].IsBubble {
				flushed = true
				return HazardResponse{
					Action:     HazardFlush,
					FlushCount: 0, // Use default
					RedirectPC: 100,
				}
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	for i := 0; i < 5; i++ {
		p.Step()
	}

	if p.Stats().FlushCycles != 1 {
		t.Errorf("expected 1 flush cycle, got %d", p.Stats().FlushCycles)
	}
}

// =========================================================================
// Stall with default stall point test
// =========================================================================

func TestStallDefaultStallPoint(t *testing.T) {
	instrs := make([]int, 20)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	stallCount := 0
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		stallCount++
		if stallCount == 3 {
			return HazardResponse{
				Action:      HazardStall,
				StallStages: 0, // Use default (first execute stage)
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	for i := 0; i < 5; i++ {
		p.Step()
	}

	if p.Stats().StallCycles != 1 {
		t.Errorf("expected 1 stall cycle, got %d", p.Stats().StallCycles)
	}
}

// =========================================================================
// ForwardFromMEM test
// =========================================================================

func TestForwardFromMEM(t *testing.T) {
	instrs := make([]int, 10)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	forwardCycle := 0
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		forwardCycle++
		if forwardCycle == 4 {
			return HazardResponse{
				Action:        HazardForwardFromMEM,
				ForwardValue:  77,
				ForwardSource: "MEM",
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	for i := 0; i < 4; i++ {
		p.Step()
	}

	exTok := p.StageContents("EX")
	if exTok == nil {
		t.Fatal("expected token in EX stage")
	}
	if exTok.ForwardedFrom != "MEM" {
		t.Errorf("expected ForwardedFrom='MEM', got %q", exTok.ForwardedFrom)
	}
}

// =========================================================================
// Edge cases
// =========================================================================

// TestFlushCountLargerThanPipeline verifies clamping.
func TestFlushCountLargerThanPipeline(t *testing.T) {
	instrs := make([]int, 20)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	flushed := false
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		if !flushed && stages[2] != nil && !stages[2].IsBubble {
			flushed = true
			return HazardResponse{
				Action:     HazardFlush,
				FlushCount: 100, // Way too many -- should be clamped.
				RedirectPC: 0,
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	// Should not panic.
	for i := 0; i < 10; i++ {
		p.Step()
	}
}

// TestStallPointLargerThanPipeline verifies clamping.
func TestStallPointLargerThanPipeline(t *testing.T) {
	instrs := make([]int, 20)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)

	stallCount := 0
	p.SetHazardFunc(func(stages []*PipelineToken) HazardResponse {
		stallCount++
		if stallCount == 3 {
			return HazardResponse{
				Action:      HazardStall,
				StallStages: 100, // Way too large -- should be clamped.
			}
		}
		return HazardResponse{Action: HazardNone}
	})

	// Should not panic.
	for i := 0; i < 10; i++ {
		p.Step()
	}
}

// TestPipelineWithNoHazardFunc verifies normal operation without hazard detection.
func TestPipelineWithNoHazardFunc(t *testing.T) {
	instrs := make([]int, 20)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	p := newTestPipeline(instrs, nil)
	// No hazard func set -- should run without issues.

	for i := 0; i < 10; i++ {
		p.Step()
	}

	stats := p.Stats()
	if stats.StallCycles != 0 {
		t.Errorf("expected 0 stall cycles without hazard func, got %d", stats.StallCycles)
	}
	if stats.FlushCycles != 0 {
		t.Errorf("expected 0 flush cycles without hazard func, got %d", stats.FlushCycles)
	}
}

// =========================================================================
// Two-stage pipeline test
// =========================================================================

func TestTwoStagePipeline(t *testing.T) {
	config := PipelineConfig{
		Stages: []PipelineStage{
			{Name: "IF", Description: "Fetch", Category: StageFetch},
			{Name: "WB", Description: "Writeback", Category: StageWriteback},
		},
		ExecutionWidth: 1,
	}

	instrs := make([]int, 10)
	for i := range instrs {
		instrs[i] = makeInstruction(opADD, 1, 2, 3)
	}

	completed := &completedInstructions{}
	p, err := NewPipeline(
		config,
		simpleFetch(instrs),
		simpleDecode(),
		simpleExecute(),
		simpleMemory(),
		simpleWriteback(completed),
	)
	if err != nil {
		t.Fatal(err)
	}

	// In a 2-stage pipeline, first completion at cycle 2.
	p.Step() // cycle 1
	if len(completed.pcs) != 0 {
		t.Errorf("expected 0 completions after 1 cycle, got %d", len(completed.pcs))
	}

	p.Step() // cycle 2
	if len(completed.pcs) != 1 {
		t.Errorf("expected 1 completion after 2 cycles, got %d", len(completed.pcs))
	}
}

// =========================================================================
// Decode test -- verify token gets decoded in ID stage
// =========================================================================

func TestDecodeStage(t *testing.T) {
	instrs := []int{
		makeInstruction(opLDR, 5, 3, 0),
		makeInstruction(opNOP, 0, 0, 0),
	}

	p := newTestPipeline(instrs, nil)

	p.Step() // cycle 1: LDR enters IF
	p.Step() // cycle 2: LDR moves to ID, gets decoded

	idTok := p.StageContents("ID")
	if idTok == nil {
		t.Fatal("expected token in ID stage")
	}
	if idTok.Opcode != "LDR" {
		t.Errorf("expected opcode 'LDR', got %q", idTok.Opcode)
	}
	if idTok.Rd != 5 {
		t.Errorf("expected Rd=5, got %d", idTok.Rd)
	}
	if !idTok.MemRead {
		t.Error("expected MemRead=true for LDR")
	}
	if !idTok.RegWrite {
		t.Error("expected RegWrite=true for LDR")
	}
}

// =========================================================================
// Instruction count verification
// =========================================================================

func TestInstructionCountMatchesCompletions(t *testing.T) {
	instrs := []int{
		makeInstruction(opADD, 1, 2, 3),
		makeInstruction(opADD, 4, 5, 6),
		makeInstruction(opADD, 7, 8, 9),
		makeInstruction(opHALT, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
		makeInstruction(opNOP, 0, 0, 0),
	}

	completed := &completedInstructions{}
	p := newTestPipeline(instrs, completed)
	stats := p.Run(100)

	if stats.InstructionsCompleted != len(completed.pcs) {
		t.Errorf("stats.InstructionsCompleted (%d) != len(completed.pcs) (%d)",
			stats.InstructionsCompleted, len(completed.pcs))
	}

	// Should be 4: ADD, ADD, ADD, HALT.
	if stats.InstructionsCompleted != 4 {
		t.Errorf("expected 4 completed instructions, got %d", stats.InstructionsCompleted)
	}
}
