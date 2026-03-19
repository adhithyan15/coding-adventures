package hazarddetection

import (
	"testing"
)

// =========================================================================
// Helper: shorthand for creating PipelineSlots
// =========================================================================

func emptySlot() PipelineSlot {
	return PipelineSlot{Valid: false}
}

// =========================================================================
// DataHazardDetector Tests
// =========================================================================

func TestDataHazard_NoHazardWhenIDEmpty(t *testing.T) {
	d := &DataHazardDetector{}
	id := emptySlot()
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1)}
	mem := emptySlot()
	result := d.Detect(id, ex, mem)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestDataHazard_NoHazardWhenNoSourceRegs(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1)}
	mem := emptySlot()
	result := d.Detect(id, ex, mem)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestDataHazard_NoHazardWhenNoDependency(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{2, 3}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(5)}
	mem := PipelineSlot{Valid: true, DestReg: IntPtr(6)}
	result := d.Detect(id, ex, mem)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestDataHazard_ForwardFromEX(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1, 5}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(42)}
	mem := emptySlot()
	result := d.Detect(id, ex, mem)
	if result.Action != ActionForwardFromEX {
		t.Errorf("expected FORWARD_FROM_EX, got %v", result.Action)
	}
	if result.ForwardedValue == nil || *result.ForwardedValue != 42 {
		t.Errorf("expected forwarded value 42")
	}
	if result.ForwardedFrom != "EX" {
		t.Errorf("expected forwarded from EX, got %s", result.ForwardedFrom)
	}
}

func TestDataHazard_ForwardFromMEM(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := emptySlot()
	mem := PipelineSlot{Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(99)}
	result := d.Detect(id, ex, mem)
	if result.Action != ActionForwardFromMEM {
		t.Errorf("expected FORWARD_FROM_MEM, got %v", result.Action)
	}
	if result.ForwardedValue == nil || *result.ForwardedValue != 99 {
		t.Errorf("expected forwarded value 99")
	}
}

func TestDataHazard_LoadUseStall(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1), MemRead: true}
	mem := emptySlot()
	result := d.Detect(id, ex, mem)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
	if result.StallCycles != 1 {
		t.Errorf("expected 1 stall cycle, got %d", result.StallCycles)
	}
}

func TestDataHazard_EXPriorityOverMEM(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(10)}
	mem := PipelineSlot{Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(20)}
	result := d.Detect(id, ex, mem)
	if result.Action != ActionForwardFromEX {
		t.Errorf("expected FORWARD_FROM_EX, got %v", result.Action)
	}
	if *result.ForwardedValue != 10 {
		t.Errorf("expected forwarded value 10, got %d", *result.ForwardedValue)
	}
}

func TestDataHazard_MultipleSourceRegsWorstWins(t *testing.T) {
	d := &DataHazardDetector{}
	// R1 forwards from MEM, R2 forwards from EX -> EX wins
	id := PipelineSlot{Valid: true, SourceRegs: []int{1, 2}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(2), DestValue: IntPtr(55)}
	mem := PipelineSlot{Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(77)}
	result := d.Detect(id, ex, mem)
	if result.Action != ActionForwardFromEX {
		t.Errorf("expected FORWARD_FROM_EX, got %v", result.Action)
	}
}

func TestDataHazard_StallBeatsForward(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1, 2}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1), MemRead: true}
	mem := PipelineSlot{Valid: true, DestReg: IntPtr(2), DestValue: IntPtr(77)}
	result := d.Detect(id, ex, mem)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
}

func TestDataHazard_NoHazardWhenEXDestRegNil(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := PipelineSlot{Valid: true, DestReg: nil}
	mem := emptySlot()
	result := d.Detect(id, ex, mem)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestDataHazard_NoHazardWhenEXInvalid(t *testing.T) {
	d := &DataHazardDetector{}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := PipelineSlot{Valid: false, DestReg: IntPtr(1)}
	mem := emptySlot()
	result := d.Detect(id, ex, mem)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

// =========================================================================
// ControlHazardDetector Tests
// =========================================================================

func TestControlHazard_NoHazardWhenEXEmpty(t *testing.T) {
	c := &ControlHazardDetector{}
	result := c.Detect(emptySlot())
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestControlHazard_NoHazardWhenNotBranch(t *testing.T) {
	c := &ControlHazardDetector{}
	ex := PipelineSlot{Valid: true, IsBranch: false}
	result := c.Detect(ex)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestControlHazard_CorrectlyPredictedTaken(t *testing.T) {
	c := &ControlHazardDetector{}
	ex := PipelineSlot{Valid: true, IsBranch: true, BranchTaken: true, BranchPredictedTaken: true}
	result := c.Detect(ex)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestControlHazard_CorrectlyPredictedNotTaken(t *testing.T) {
	c := &ControlHazardDetector{}
	ex := PipelineSlot{Valid: true, IsBranch: true, BranchTaken: false, BranchPredictedTaken: false}
	result := c.Detect(ex)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestControlHazard_MispredictionNotTakenButTaken(t *testing.T) {
	c := &ControlHazardDetector{}
	ex := PipelineSlot{Valid: true, IsBranch: true, PC: 0x100, BranchTaken: true, BranchPredictedTaken: false}
	result := c.Detect(ex)
	if result.Action != ActionFlush {
		t.Errorf("expected FLUSH, got %v", result.Action)
	}
	if result.FlushCount != 2 {
		t.Errorf("expected flush count 2, got %d", result.FlushCount)
	}
}

func TestControlHazard_MispredictionTakenButNotTaken(t *testing.T) {
	c := &ControlHazardDetector{}
	ex := PipelineSlot{Valid: true, IsBranch: true, PC: 0x200, BranchTaken: false, BranchPredictedTaken: true}
	result := c.Detect(ex)
	if result.Action != ActionFlush {
		t.Errorf("expected FLUSH, got %v", result.Action)
	}
	if result.FlushCount != 2 {
		t.Errorf("expected flush count 2, got %d", result.FlushCount)
	}
}

// =========================================================================
// StructuralHazardDetector Tests
// =========================================================================

func TestStructural_NoHazardWithEnoughALUs(t *testing.T) {
	s := NewStructuralHazardDetector(2, 1, true)
	id := PipelineSlot{Valid: true, UsesALU: true}
	ex := PipelineSlot{Valid: true, UsesALU: true}
	result := s.Detect(id, ex, nil, nil)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestStructural_ALUConflictWithOneALU(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, true)
	id := PipelineSlot{Valid: true, UsesALU: true}
	ex := PipelineSlot{Valid: true, UsesALU: true}
	result := s.Detect(id, ex, nil, nil)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
}

func TestStructural_FPConflictWithOneFPUnit(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, true)
	id := PipelineSlot{Valid: true, UsesALU: false, UsesFP: true}
	ex := PipelineSlot{Valid: true, UsesALU: false, UsesFP: true}
	result := s.Detect(id, ex, nil, nil)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
}

func TestStructural_NoFPConflictWithTwoFPUnits(t *testing.T) {
	s := NewStructuralHazardDetector(1, 2, true)
	id := PipelineSlot{Valid: true, UsesALU: false, UsesFP: true}
	ex := PipelineSlot{Valid: true, UsesALU: false, UsesFP: true}
	result := s.Detect(id, ex, nil, nil)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestStructural_NoConflictWhenIDEmpty(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, true)
	id := emptySlot()
	ex := PipelineSlot{Valid: true, UsesALU: true}
	result := s.Detect(id, ex, nil, nil)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestStructural_NoConflictWhenEXEmpty(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, true)
	id := PipelineSlot{Valid: true, UsesALU: true}
	ex := emptySlot()
	result := s.Detect(id, ex, nil, nil)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestStructural_MemoryPortConflictSharedCache(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, false)
	id := PipelineSlot{Valid: true, UsesALU: false}
	ex := PipelineSlot{Valid: true, UsesALU: false}
	ifStage := PipelineSlot{Valid: true, PC: 0x10}
	memStage := PipelineSlot{Valid: true, PC: 0x04, MemRead: true}
	result := s.Detect(id, ex, &ifStage, &memStage)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
}

func TestStructural_NoMemoryConflictSplitCache(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, true)
	id := PipelineSlot{Valid: true, UsesALU: false}
	ex := PipelineSlot{Valid: true, UsesALU: false}
	ifStage := PipelineSlot{Valid: true}
	memStage := PipelineSlot{Valid: true, MemRead: true}
	result := s.Detect(id, ex, &ifStage, &memStage)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestStructural_MemoryPortConflictStore(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, false)
	id := PipelineSlot{Valid: true, UsesALU: false}
	ex := PipelineSlot{Valid: true, UsesALU: false}
	ifStage := PipelineSlot{Valid: true}
	memStage := PipelineSlot{Valid: true, MemWrite: true}
	result := s.Detect(id, ex, &ifStage, &memStage)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
}

func TestStructural_NoMemoryConflictWhenMEMNotAccessing(t *testing.T) {
	s := NewStructuralHazardDetector(1, 1, false)
	id := PipelineSlot{Valid: true, UsesALU: false}
	ex := PipelineSlot{Valid: true, UsesALU: false}
	ifStage := PipelineSlot{Valid: true}
	memStage := PipelineSlot{Valid: true}
	result := s.Detect(id, ex, &ifStage, &memStage)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

// =========================================================================
// HazardUnit Tests
// =========================================================================

func TestHazardUnit_NoHazard(t *testing.T) {
	unit := NewHazardUnit(2, 1, true)
	ifS := PipelineSlot{Valid: true}
	id := PipelineSlot{Valid: true, SourceRegs: []int{2}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(5)}
	mem := emptySlot()
	result := unit.Check(ifS, id, ex, mem)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

func TestHazardUnit_DataForwarding(t *testing.T) {
	unit := NewHazardUnit(2, 1, true)
	ifS := PipelineSlot{Valid: true}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(42)}
	mem := emptySlot()
	result := unit.Check(ifS, id, ex, mem)
	if result.Action != ActionForwardFromEX {
		t.Errorf("expected FORWARD_FROM_EX, got %v", result.Action)
	}
	if *result.ForwardedValue != 42 {
		t.Errorf("expected 42, got %d", *result.ForwardedValue)
	}
}

func TestHazardUnit_FlushBeatsForward(t *testing.T) {
	unit := NewHazardUnit(2, 1, true)
	ifS := PipelineSlot{Valid: true}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := PipelineSlot{
		Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(42),
		IsBranch: true, BranchTaken: true, BranchPredictedTaken: false,
	}
	mem := emptySlot()
	result := unit.Check(ifS, id, ex, mem)
	if result.Action != ActionFlush {
		t.Errorf("expected FLUSH, got %v", result.Action)
	}
}

func TestHazardUnit_StallBeatsForward(t *testing.T) {
	unit := NewHazardUnit(2, 1, true)
	ifS := PipelineSlot{Valid: true}
	id := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(1), MemRead: true}
	mem := emptySlot()
	result := unit.Check(ifS, id, ex, mem)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
}

func TestHazardUnit_Statistics(t *testing.T) {
	unit := NewHazardUnit(2, 1, true)
	empty := emptySlot()
	ifS := PipelineSlot{Valid: true}

	// Cycle 1: no hazard
	id1 := PipelineSlot{Valid: true, SourceRegs: []int{2}}
	ex1 := PipelineSlot{Valid: true, DestReg: IntPtr(5)}
	unit.Check(ifS, id1, ex1, empty)

	// Cycle 2: forward from EX
	id2 := PipelineSlot{Valid: true, SourceRegs: []int{1}}
	ex2 := PipelineSlot{Valid: true, DestReg: IntPtr(1), DestValue: IntPtr(10)}
	unit.Check(ifS, id2, ex2, empty)

	// Cycle 3: flush
	ex3 := PipelineSlot{Valid: true, IsBranch: true, BranchTaken: true, BranchPredictedTaken: false}
	unit.Check(ifS, empty, ex3, empty)

	if len(unit.History()) != 3 {
		t.Errorf("expected 3 history entries, got %d", len(unit.History()))
	}
	if unit.StallCount() != 0 {
		t.Errorf("expected 0 stall count, got %d", unit.StallCount())
	}
	if unit.FlushCount() != 1 {
		t.Errorf("expected 1 flush count, got %d", unit.FlushCount())
	}
	if unit.ForwardCount() != 1 {
		t.Errorf("expected 1 forward count, got %d", unit.ForwardCount())
	}
}

func TestHazardUnit_StructuralStallWithOneALU(t *testing.T) {
	unit := NewHazardUnit(1, 1, true)
	ifS := PipelineSlot{Valid: true}
	id := PipelineSlot{Valid: true, SourceRegs: []int{}, UsesALU: true}
	ex := PipelineSlot{Valid: true, DestReg: IntPtr(5), UsesALU: true}
	mem := emptySlot()
	result := unit.Check(ifS, id, ex, mem)
	if result.Action != ActionStall {
		t.Errorf("expected STALL, got %v", result.Action)
	}
}

func TestHazardUnit_ForwardFromMEM(t *testing.T) {
	unit := NewHazardUnit(2, 1, true)
	ifS := PipelineSlot{Valid: true}
	id := PipelineSlot{Valid: true, SourceRegs: []int{3}}
	ex := emptySlot()
	mem := PipelineSlot{Valid: true, DestReg: IntPtr(3), DestValue: IntPtr(88)}
	result := unit.Check(ifS, id, ex, mem)
	if result.Action != ActionForwardFromMEM {
		t.Errorf("expected FORWARD_FROM_MEM, got %v", result.Action)
	}
	if *result.ForwardedValue != 88 {
		t.Errorf("expected 88, got %d", *result.ForwardedValue)
	}
}

func TestHazardUnit_AllEmptyStages(t *testing.T) {
	unit := NewHazardUnit(1, 1, true)
	empty := emptySlot()
	result := unit.Check(empty, empty, empty, empty)
	if result.Action != ActionNone {
		t.Errorf("expected NONE, got %v", result.Action)
	}
}

// Test HazardAction.String() for coverage
func TestHazardAction_String(t *testing.T) {
	cases := []struct {
		action HazardAction
		want   string
	}{
		{ActionNone, "NONE"},
		{ActionForwardFromMEM, "FORWARD_FROM_MEM"},
		{ActionForwardFromEX, "FORWARD_FROM_EX"},
		{ActionStall, "STALL"},
		{ActionFlush, "FLUSH"},
		{HazardAction(99), "UNKNOWN"},
	}
	for _, tc := range cases {
		if got := tc.action.String(); got != tc.want {
			t.Errorf("HazardAction(%d).String() = %q, want %q", tc.action, got, tc.want)
		}
	}
}
