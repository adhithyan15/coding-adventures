package ftlnandrelocation

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func referencePlanRetirementDrain(before referenceState, block int64, excludedBlocks []bool) (ftlevacuationplan.Plan, string) {
	if before.logicalCount < 1 || before.logicalCount > 64 || before.physicalCount < 2 || before.physicalCount > 128 || before.physicalCount <= before.logicalCount || before.pageSize < 1 || before.pageSize > maxPageSize {
		return ftlevacuationplan.Plan{}, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return ftlevacuationplan.Plan{}, "erase_disabled"
	}
	blockCount := int64(len(before.blocks))
	if block < 0 || block >= blockCount {
		return ftlevacuationplan.Plan{}, "invalid_block_index"
	}
	if len(excludedBlocks) != 0 && int64(len(excludedBlocks)) != blockCount {
		return ftlevacuationplan.Plan{}, "invalid_evacuation_block_mask"
	}
	if before.blocks[int(block)].IncompleteErase {
		return ftlevacuationplan.Plan{}, "evacuation_block_incomplete"
	}
	if before.evacuationActive && block == before.evacuationBlock {
		return ftlevacuationplan.Plan{}, "evacuation_active"
	}

	destinationFences := make([]bool, len(before.blocks))
	copy(destinationFences, excludedBlocks)
	for index, state := range before.blocks {
		if state.IncompleteErase {
			destinationFences[index] = true
		}
	}
	if before.evacuationActive {
		destinationFences[int(before.evacuationBlock)] = true
	}

	first := block * before.pagesPerBlock
	want := ftlevacuationplan.Plan{
		Block: block, FirstPhysical: first, PageCount: before.pagesPerBlock,
		Copies: make([]ftlevacuationplan.Copy, 0, before.pagesPerBlock),
	}
	for physical := first; physical < first+before.pagesPerBlock; physical++ {
		if before.status[int(physical)] != modelLive {
			continue
		}
		logical := int64(-1)
		for index, current := range before.mapping {
			if current == physical {
				logical = int64(index)
				break
			}
		}
		want.Copies = append(want.Copies, ftlevacuationplan.Copy{
			Logical: logical, SourcePhysical: physical,
		})
	}
	destination := 0
	for physical, state := range before.status {
		if state != modelFree {
			continue
		}
		candidateBlock := int64(physical) / before.pagesPerBlock
		if candidateBlock == block || destinationFences[int(candidateBlock)] {
			continue
		}
		if destination < len(want.Copies) {
			want.Copies[destination].DestinationPhysical = int64(physical)
		}
		destination++
	}
	if destination < len(want.Copies) {
		return ftlevacuationplan.Plan{}, "insufficient_capacity"
	}
	return want, ""
}

func compareRetirementDrainPlan(t *testing.T, device Device, model referenceState, block int64, excludedBlocks []bool) ftlevacuationplan.Plan {
	t.Helper()
	var maskBefore []bool
	if excludedBlocks != nil {
		maskBefore = append([]bool{}, excludedBlocks...)
	}
	before := device.Snapshot()
	beforeReclamation := device.ReclamationSnapshot()
	want, wantError := referencePlanRetirementDrain(model, block, excludedBlocks)
	got, err := device.PlanRetirementDrain(block, excludedBlocks)
	assertError(t, err, wantError)
	if !reflect.DeepEqual(excludedBlocks, maskBefore) {
		t.Fatal("PlanRetirementDrain changed the caller's block mask")
	}
	if wantError != "" {
		if !reflect.DeepEqual(got, ftlevacuationplan.Plan{}) || !reflect.DeepEqual(device.Snapshot(), before) || !reflect.DeepEqual(device.ReclamationSnapshot(), beforeReclamation) {
			t.Fatalf("refused retirement plan changed state or returned data: %+v, %v", got, err)
		}
		return ftlevacuationplan.Plan{}
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("retirement plan = %+v, want %+v", got, want)
	}
	if !reflect.DeepEqual(device.Snapshot(), before) || !reflect.DeepEqual(device.ReclamationSnapshot(), beforeReclamation) {
		t.Fatal("read-only retirement planning changed the device")
	}
	return got
}

func TestRetirementDrainPlansAcrossExhaustiveDeviceGraphs(t *testing.T) {
	queries := 0
	for _, geometry := range []struct{ logical, physical, ppb int64 }{{1, 4, 2}, {2, 4, 2}} {
		initial, err := NewWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.ppb, 1)
		if err != nil {
			t.Fatal(err)
		}
		initialModel := referenceInitialWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.ppb, 1)
		queue := []struct {
			device Device
			model  referenceState
		}{{initial, initialModel}}
		seen := map[string]bool{}
		for len(queue) > 0 {
			node := queue[0]
			queue = queue[1:]
			key := evacuationStateKey(node.model)
			if seen[key] {
				continue
			}
			seen[key] = true
			blockCount := len(node.model.blocks)
			for block := int64(0); block < int64(blockCount); block++ {
				for maskBits := 0; maskBits < 1<<blockCount; maskBits++ {
					mask := make([]bool, blockCount)
					for index := range mask {
						mask[index] = maskBits&(1<<index) != 0
					}
					compareRetirementDrainPlan(t, node.device, node.model, block, mask)
					queries++
				}
			}
			for _, next := range evacuationSuccessors(t, node.device, node.model) {
				nextKey := evacuationStateKey(next.model)
				if !seen[nextKey] {
					queue = append(queue, next)
				}
			}
		}
		t.Logf("retirement planner checked %d target/mask queries over %d closed states for %d/%d geometry", queries, len(seen), geometry.logical, geometry.physical)
	}
	if queries == 0 {
		t.Fatal("exhaustive retirement graph performed no planner queries")
	}
}

func TestRetirementDrainFixedSeedQueriesAndWriteSetup(t *testing.T) {
	rng := rand.New(rand.NewSource(0x5d02b2b1))
	queryCount := 0
	writeCount := 0
	for caseIndex := 0; caseIndex < 519; caseIndex++ {
		device, err := NewWithEraseBlocks(2, 8, 1, 2, 1)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 8, 1, 2, 1)
		commandCount := 35
		if caseIndex < 267 {
			commandCount++
		}
		for command := 0; command < commandCount; command++ {
			logical := int64(rng.Intn(2))
			value := byte(rng.Intn(256))
			cut := []Cut{
				{Kind: nandpageprogramcuts.Complete},
				{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
				{Kind: nandpageprogramcuts.AfterMedia},
				{Kind: nandpageprogramcuts.BeforeAccept},
			}[rng.Intn(4)]
			device, model = compareTransition(t, device, model, logical, []byte{value}, runtimeModelCut(cut))
			writeCount++
		}
		queriesForCase := 108
		if caseIndex < 364 {
			queriesForCase++
		}
		for query := 0; query < queriesForCase; query++ {
			block := int64(rng.Intn(len(model.blocks)))
			var mask []bool
			switch rng.Intn(4) {
			case 0:
				mask = nil
			case 1:
				mask = []bool{}
			case 2:
				mask = make([]bool, len(model.blocks))
				for index := range mask {
					mask[index] = rng.Intn(2) == 1
				}
			default:
				mask = []bool{true}
			}
			compareRetirementDrainPlan(t, device, model, block, mask)
			queryCount++
		}
	}
	if writeCount != 18_432 || queryCount != 56_416 {
		t.Fatalf("fixed-seed evidence counts writes=%d queries=%d", writeCount, queryCount)
	}
	t.Logf("fixed seed compared %d write setup transitions and %d read-only plans over 519 cases", writeCount, queryCount)
}

func TestRetirementDrainValidationOrderAndReadOnlyBudgetSemantics(t *testing.T) {
	var zero Device
	if plan, err := zero.PlanRetirementDrain(-1, []bool{true}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || !reflect.DeepEqual(plan, ftlevacuationplan.Plan{}) {
		t.Fatalf("zero device precedence: plan=%+v error=%v", plan, err)
	}
	legacy, _ := New(1, 3, 1)
	if plan, err := legacy.PlanRetirementDrain(-1, []bool{true}); !errors.Is(err, ErrEraseDisabled) || !reflect.DeepEqual(plan, ftlevacuationplan.Plan{}) {
		t.Fatalf("legacy profile precedence: plan=%+v error=%v", plan, err)
	}
	device, _ := NewWithEraseBlocks(1, 4, 1, 2, 0)
	if plan, err := device.PlanRetirementDrain(-1, []bool{true}); !errors.Is(err, ErrInvalidBlockIndex) {
		t.Fatalf("block address must precede mask width: plan=%+v error=%v", plan, err)
	}
	if plan, err := device.PlanRetirementDrain(0, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("mask width refusal: plan=%+v error=%v", plan, err)
	}
	device, _, _ = device.Write(0, []byte{0x42}, Cut{Kind: nandpageprogramcuts.Complete})
	model := referenceInitialWithEraseBlocks(1, 4, 1, 2, 0)
	model, _, _ = referenceWrite(model, 0, []byte{0x42}, modelCut{kind: cutComplete})
	plan, err := device.PlanRetirementDrain(0, []bool{true, false})
	if err != nil {
		t.Fatalf("zero-budget all-live source planning must be allowed: %v", err)
	}
	plan = compareRetirementDrainPlan(t, device, model, 0, []bool{true, false})
	if len(plan.Copies) != 1 || plan.Copies[0] != (ftlevacuationplan.Copy{Logical: 0, SourcePhysical: 0, DestinationPhysical: 2}) {
		t.Fatalf("zero-budget all-live source plan=%+v", plan)
	}
	if plan, err := device.PlanRetirementDrain(0, []bool{false, true}); !errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity) || !reflect.DeepEqual(plan, ftlevacuationplan.Plan{}) {
		t.Fatalf("caller-fenced destination did not produce shortage: plan=%+v error=%v", plan, err)
	}
	empty, _ := NewWithEraseBlocks(1, 4, 1, 2, 0)
	emptyPlan, err := empty.PlanRetirementDrain(0, nil)
	if err != nil || emptyPlan.Block != 0 || emptyPlan.FirstPhysical != 0 || emptyPlan.PageCount != 2 || len(emptyPlan.Copies) != 0 {
		t.Fatalf("zero-budget empty-source plan=%+v error=%v", emptyPlan, err)
	}
	if !reflect.DeepEqual(emptyPlan.Copies, []ftlevacuationplan.Copy{}) {
		t.Fatal("empty plan must return an owned empty copy slice")
	}
}

func TestRetirementDrainRejectsActiveAndIncompleteSourcesAfterMaskValidation(t *testing.T) {
	active, model := prepareStaleFreeBlock(t)
	active, model = compareEvacuationBegin(t, active, model, 0)
	if _, err := active.PlanRetirementDrain(0, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("active source with bad mask must report width first: %v", err)
	}
	if plan, err := active.PlanRetirementDrain(0, []bool{false, false}); !errors.Is(err, ErrEvacuationActive) || !reflect.DeepEqual(plan, ftlevacuationplan.Plan{}) {
		t.Fatalf("active source refusal: plan=%+v error=%v", plan, err)
	}

	incomplete, incompleteModel := prepareStaleFreeBlock(t)
	incomplete, incompleteModel = compareEraseBlock(t, incomplete, incompleteModel, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if _, err := incomplete.PlanRetirementDrain(0, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("incomplete source with bad mask must report width first: %v", err)
	}
	if plan, err := incomplete.PlanRetirementDrain(0, []bool{false, false}); !errors.Is(err, ErrEvacuationBlockIncomplete) || !reflect.DeepEqual(plan, ftlevacuationplan.Plan{}) {
		t.Fatalf("incomplete source refusal: plan=%+v error=%v", plan, err)
	}
}

func TestRetirementDrainIncompleteSourcePrecedesActiveSource(t *testing.T) {
	device, model := prepareStaleFreeBlock(t)
	device, model = compareEvacuationBegin(t, device, model, 0)
	device, receipt, err := device.FinishEvacuation(EraseCut{
		Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1,
	})
	if err != nil || receipt.Released || receipt.Acknowledged {
		t.Fatalf("partial active erase receipt=%+v error=%v", receipt, err)
	}
	model, _, modelError := referenceFinishEvacuation(model, nandblockerasecuts.Cut{
		Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1,
	})
	if modelError != "" {
		t.Fatalf("partial active model erase: %s", modelError)
	}
	assertEvacuationState(t, device, model)
	if !device.evacuationActive || !device.blocks[0].IncompleteErase {
		t.Fatalf("test did not establish active and incomplete source: active=%t block=%+v", device.evacuationActive, device.blocks[0])
	}
	if _, err := device.PlanRetirementDrain(0, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("mask width must precede source-state refusals: %v", err)
	}
	if plan, err := device.PlanRetirementDrain(0, []bool{false, false}); !errors.Is(err, ErrEvacuationBlockIncomplete) || !reflect.DeepEqual(plan, ftlevacuationplan.Plan{}) {
		t.Fatalf("incomplete source must precede active source: plan=%+v error=%v", plan, err)
	}
}

func TestRetirementDrainCombinesActiveAndIncompleteDestinationFences(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 12, 1, 3, 3)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 12, 1, 3, 3)
	device, model = compareTransition(t, device, model, 0, []byte{0x10}, modelCut{kind: cutComplete})
	device, model = compareTransition(t, device, model, 0, []byte{0x11}, modelCut{kind: cutComplete})
	device, model = compareEvacuationBegin(t, device, model, 0)
	device, model = compareTransition(t, device, model, 1, []byte{0x20}, modelCut{kind: cutComplete})
	device, model = compareTransition(t, device, model, 1, []byte{0x21}, modelCut{kind: cutComplete})
	externalPages := make([]bool, 12)
	for physical := 3; physical < 6; physical++ {
		externalPages[physical] = true
	}
	device, _, err = device.WriteExcluding(1, []byte{0x22}, Cut{Kind: nandpageprogramcuts.Complete}, externalPages)
	if err != nil {
		t.Fatal(err)
	}
	model, _, _ = referenceWriteExcluding(model, 1, []byte{0x22}, modelCut{kind: cutComplete}, externalPages)
	device, model = compareEraseBlock(t, device, model, 1, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	if !device.evacuationActive || !device.blocks[1].IncompleteErase {
		t.Fatalf("test source did not establish both internal fences: active=%t reclamation=%+v", device.evacuationActive, device.blocks)
	}
	plan := compareRetirementDrainPlan(t, device, model, 2, nil)
	if len(plan.Copies) != 1 || plan.Copies[0] != (ftlevacuationplan.Copy{Logical: 1, SourcePhysical: 6, DestinationPhysical: 9}) {
		t.Fatalf("planner did not select the first spare after both internal fences: %+v", plan)
	}
	callerFenced, err := device.PlanRetirementDrain(2, []bool{false, false, false, true})
	if !errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity) || !reflect.DeepEqual(callerFenced, ftlevacuationplan.Plan{}) {
		t.Fatalf("external mask did not fence the last destination: plan=%+v error=%v", callerFenced, err)
	}
}

func TestRetirementDrainPlanCopiesAreDetached(t *testing.T) {
	device, _ := NewWithEraseBlocks(1, 4, 1, 2, 0)
	device, _, _ = device.Write(0, []byte{0x42}, Cut{Kind: nandpageprogramcuts.Complete})
	first, err := device.PlanRetirementDrain(0, nil)
	if err != nil || len(first.Copies) != 1 {
		t.Fatalf("initial plan=%+v error=%v", first, err)
	}
	first.Copies[0].DestinationPhysical = 99
	second, err := device.PlanRetirementDrain(0, nil)
	if err != nil || second.Copies[0].DestinationPhysical != 2 {
		t.Fatalf("caller mutation reached device or later plan: %+v error=%v", second, err)
	}
}

func FuzzRetirementDrainPlanHistory(f *testing.F) {
	f.Add([]byte{0, 0, 0, 1, 1, 2, 0, 0xff, 3, 1, 1, 2, 0, 0x42})
	f.Add([]byte{1, 3, 2, 0, 0, 1, 4, 0xff, 0, 0, 2, 1})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			return
		}
		device, err := NewWithEraseBlocks(2, 8, 1, 2, 1)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 8, 1, 2, 1)
		for offset := 0; offset+3 < len(commands); offset += 4 {
			selector, logicalByte, value, maskByte := commands[offset], commands[offset+1], commands[offset+2], commands[offset+3]
			switch selector % 5 {
			case 0:
				device, model = compareTransition(t, device, model, int64(logicalByte%2), []byte{value}, runtimeModelCut(fuzzProgramCut(selector)))
			case 1:
				device, model = compareEvacuationBegin(t, device, model, int64(logicalByte%4))
			case 2:
				device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.CutKind(selector % 5)})
			case 3:
				device, model = compareEvacuationCancel(t, device, model)
			case 4:
				device, model = compareEraseBlock(t, device, model, int64(logicalByte%4), EraseCut{Kind: nandblockerasecuts.CutKind(value % 4), PrefixPages: int(maskByte % 3)})
			}
			block := int64(logicalByte % 4)
			var mask []bool
			switch selector % 3 {
			case 0:
				mask = nil
			case 1:
				mask = make([]bool, 4)
				for index := range mask {
					mask[index] = maskByte&(1<<index) != 0
				}
			default:
				mask = []bool{true}
			}
			compareRetirementDrainPlan(t, device, model, block, mask)
		}
	})
}
