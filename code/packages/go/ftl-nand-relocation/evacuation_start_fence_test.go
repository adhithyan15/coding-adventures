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

func TestBeginEvacuationExcludingMatchesLegacyAndIgnoresVictimBit(t *testing.T) {
	device, model := prepareStartFenceCapacityCase(t)
	plan, err := ftlevacuationplan.PlanEvacuation(device.table, 2, 2, []bool{false, true, false, false})
	if err != nil || !reflect.DeepEqual(plan.Copies, []ftlevacuationplan.Copy{{Logical: 0, SourcePhysical: 5, DestinationPhysical: 7}}) {
		t.Fatalf("six-page exact-capacity plan=%+v err=%v", plan, err)
	}

	for name, mask := range map[string][]bool{
		"nil":                 nil,
		"all-clear":           {false, false, false, false},
		"victim-bit":          {false, false, true, false},
		"external-and-victim": {false, true, true, false},
	} {
		t.Run(name, func(t *testing.T) {
			beforeMask := append([]bool(nil), mask...)
			want, wantErr := referenceBeginEvacuationExcluding(model, 2, mask)
			got, err := device.BeginEvacuationExcluding(2, mask)
			assertError(t, err, wantErr)
			assertEvacuationState(t, got, want)
			if !reflect.DeepEqual(mask, beforeMask) {
				t.Fatal("begin changed caller-owned destination mask")
			}
		})
	}
	legacy, err := device.BeginEvacuation(2)
	if err != nil {
		t.Fatal(err)
	}
	allClear, err := device.BeginEvacuationExcluding(2, []bool{false, false, false, false})
	if err != nil || !reflect.DeepEqual(legacy.Snapshot(), allClear.Snapshot()) {
		t.Fatalf("legacy and all-clear successors differ: %v", err)
	}

	// Excluding the sole free destination makes the same source inadmissible.
	before := device.Snapshot()
	blocked := []bool{false, false, false, true}
	if _, err := device.BeginEvacuationExcluding(2, blocked); !errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity) {
		t.Fatalf("external destination fence error=%v", err)
	}
	if !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatal("capacity refusal changed the device")
	}
}

func TestBeginEvacuationExcludingValidationOrder(t *testing.T) {
	var zero Device
	if _, err := zero.BeginEvacuationExcluding(-1, []bool{true}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("geometry precedence=%v", err)
	}
	legacy, _ := New(1, 4, 1)
	if _, err := legacy.BeginEvacuationExcluding(-1, []bool{true}); !errors.Is(err, ErrEraseDisabled) {
		t.Fatalf("profile precedence=%v", err)
	}
	device, _ := NewWithEraseBlocks(1, 4, 1, 2, 0)
	if _, err := device.BeginEvacuationExcluding(-1, []bool{true}); !errors.Is(err, ErrInvalidBlockIndex) {
		t.Fatalf("signed block precedence=%v", err)
	}
	if _, err := device.BeginEvacuationExcluding(0, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("mask width must precede erase budget: %v", err)
	}
	if _, err := device.BeginEvacuationExcluding(0, make([]bool, 2)); !errors.Is(err, nandblockerasecuts.ErrEraseLimit) {
		t.Fatalf("erase budget after valid mask=%v", err)
	}
	incomplete, _ := prepareEvacuationCapacityCase(t)
	if _, err := incomplete.BeginEvacuationExcluding(1, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("mask width must precede incomplete target=%v", err)
	}
	if _, err := incomplete.BeginEvacuationExcluding(1, make([]bool, 3)); !errors.Is(err, ErrEvacuationBlockIncomplete) {
		t.Fatalf("incomplete target after valid mask=%v", err)
	}

	// An active device still validates the caller mask before active status.
	active, _ := NewWithEraseBlocks(1, 4, 1, 2, 2)
	for _, value := range []byte{1, 2, 3} {
		active, _, _ = active.Write(0, []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
	}
	active, err := active.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := active.BeginEvacuationExcluding(1, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("active plus malformed mask=%v", err)
	}
	if _, err := active.BeginEvacuationExcluding(1, make([]bool, 2)); !errors.Is(err, ErrEvacuationActive) {
		t.Fatalf("valid mask plus active=%v", err)
	}
}

func TestBeginEvacuationExcludingMaximumBlockMask(t *testing.T) {
	device, err := NewWithEraseBlocks(64, 128, 1, 1, 1)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{1}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{2}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	mask := make([]bool, 128)
	mask[127] = true
	next, err := device.BeginEvacuationExcluding(0, mask)
	if err != nil {
		t.Fatalf("maximum-width mask begin: %v", err)
	}
	if got, active := next.EvacuationProgress(); !active || got.Block != 0 {
		t.Fatalf("maximum-width active target=%+v/%t", got, active)
	}
	if _, err := device.BeginEvacuationExcluding(0, mask[:127]); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) {
		t.Fatalf("maximum geometry truncated mask=%v", err)
	}
}

func TestBeginEvacuationExcludingQuarantinesIncompleteDestination(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 6, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 6, 1, 2, 2)
	for _, value := range []byte{1, 2, 3, 4, 5, 6} {
		device, model = compareEvacuationWrite(t, device, model, 0, []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
	}
	device, model = compareEraseBlock(t, device, model, 1, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	mask := make([]bool, 3)
	want, wantErr := referenceBeginEvacuationExcluding(model, 2, mask)
	got, err := device.BeginEvacuationExcluding(2, mask)
	assertError(t, err, wantErr)
	assertEvacuationState(t, got, want)
	if !errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity) {
		t.Fatalf("quarantined destination error=%v", err)
	}
}

func TestBeginEvacuationExcludingBoundedMaskDomain(t *testing.T) {
	base, baseModel := prepareStartFenceCapacityCase(t)
	for block := int64(0); block < 4; block++ {
		for bits := 0; bits < 1<<4; bits++ {
			mask := make([]bool, 4)
			for index := range mask {
				mask[index] = bits&(1<<index) != 0
			}
			want, wantErr := referenceBeginEvacuationExcluding(baseModel, block, mask)
			got, gotErr := base.BeginEvacuationExcluding(block, mask)
			assertError(t, gotErr, wantErr)
			assertEvacuationState(t, got, want)
		}
	}
}

func FuzzBeginEvacuationExcludingHistory(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11})
	f.Add([]byte{0xff, 0x01, 0x81, 0x22, 0x00, 0x75, 0xa5, 0x34, 0x91, 0x03})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			return
		}
		device, err := NewWithEraseBlocks(2, 6, 1, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 1, 2, 2)
		for index := 0; index+1 < len(commands); index += 2 {
			op, value := commands[index]%8, commands[index+1]
			switch op {
			case 0:
				device, model = compareEvacuationWrite(t, device, model, int64(value%2), []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
			case 1:
				mask := make([]bool, len(model.blocks))
				for i := range mask {
					mask[i] = value&(1<<i) != 0
				}
				block := int64((value >> 4) % byte(len(mask)))
				want, wantErr := referenceBeginEvacuationExcluding(model, block, mask)
				got, gotErr := device.BeginEvacuationExcluding(block, mask)
				assertError(t, gotErr, wantErr)
				assertEvacuationState(t, got, want)
				device, model = got, want
			case 2:
				device, model = compareEvacuationCancel(t, device, model)
			case 3:
				cuts := []Cut{{Kind: nandpageprogramcuts.BeforeAccept}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, {Kind: nandpageprogramcuts.Complete}}
				device, model = compareEvacuationCopy(t, device, model, cuts[int(value)%len(cuts)])
			case 4:
				cuts := []EraseCut{{Kind: nandblockerasecuts.BeforeAccept}, {Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}, {Kind: nandblockerasecuts.Complete}}
				want, receipt, wantErr := referenceFinishEvacuation(model, cuts[int(value)%len(cuts)])
				got, gotReceipt, gotErr := device.FinishEvacuation(cuts[int(value)%len(cuts)])
				assertError(t, gotErr, wantErr)
				if gotErr == nil && gotReceipt != receipt {
					t.Fatalf("finish receipt=%+v want %+v", gotReceipt, receipt)
				}
				assertEvacuationState(t, got, want)
				device, model = got, want
			case 5:
				mask := make([]bool, len(model.blocks)-1)
				want, wantErr := referenceBeginEvacuationExcluding(model, int64(value%byte(len(model.blocks))), mask)
				got, gotErr := device.BeginEvacuationExcluding(int64(value%byte(len(model.blocks))), mask)
				assertError(t, gotErr, wantErr)
				assertEvacuationState(t, got, want)
				device, model = got, want
			case 6:
				device, model = compareEraseBlock(t, device, model, int64(value%byte(len(model.blocks))), EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
			case 7:
				device, model = compareEvacuationWrite(t, device, model, int64(value%2), []byte{value}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
			}
		}
	})
}

func TestSeededBeginEvacuationExcludingHistories(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB05e2b2e3a))
	for _, geometry := range []struct{ logical, physical int64 }{{1, 4}, {2, 6}} {
		device, err := NewWithEraseBlocks(geometry.logical, geometry.physical, 1, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(geometry.logical, geometry.physical, 1, 2, 2)
		blocks := int(geometry.physical / 2)
		for step := 0; step < 5000; step++ {
			value := rng.Intn(1 << blocks)
			mask := make([]bool, blocks)
			for i := range mask {
				mask[i] = value&(1<<i) != 0
			}
			block := int64(rng.Intn(blocks))
			want, wantErr := referenceBeginEvacuationExcluding(model, block, mask)
			got, gotErr := device.BeginEvacuationExcluding(block, mask)
			assertError(t, gotErr, wantErr)
			assertEvacuationState(t, got, want)
			device, model = got, want
			if step%11 == 0 {
				device, model = compareEvacuationCancel(t, device, model)
			}
			if step%7 == 0 {
				device, model = compareEvacuationWrite(t, device, model, int64(rng.Intn(int(geometry.logical))), []byte{byte(rng.Intn(256))}, Cut{Kind: nandpageprogramcuts.Complete})
			}
		}
	}
}

func referenceBeginEvacuationExcluding(before referenceState, block int64, destinationExcluded []bool) (referenceState, string) {
	if !referenceConfigured(before) {
		return before, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return before, "erase_disabled"
	}
	if block < 0 || block >= int64(len(before.blocks)) {
		return before, "invalid_block_index"
	}
	if len(destinationExcluded) != 0 && len(destinationExcluded) != len(before.blocks) {
		return before, "invalid_evacuation_block_mask"
	}
	if before.evacuationActive {
		return before, "evacuation_active"
	}
	if before.blocks[int(block)].IncompleteErase {
		return before, "evacuation_block_incomplete"
	}
	if before.blocks[int(block)].CompletedErases >= before.maxCompletedErases {
		return before, "erase_limit"
	}
	fences := referenceIncompleteMask(before)
	for index, excluded := range destinationExcluded {
		if excluded && int64(index) != block {
			fences[index] = true
		}
	}
	fences[int(block)] = false
	if _, code := referenceCopyAssignments(before, before.pagesPerBlock, block, fences); code != "" {
		return before, code
	}
	next := cloneReference(before)
	next.evacuationActive = true
	next.evacuationBlock = block
	return next, ""
}

func prepareStartFenceCapacityCase(t *testing.T) (Device, referenceState) {
	t.Helper()
	device, err := NewWithEraseBlocks(1, 8, 1, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 8, 1, 2, 3)
	for _, value := range []byte{1, 2, 3, 4, 5, 6} {
		device, model = compareEvacuationWrite(t, device, model, 0, []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
	}
	device, _, err = device.ProgramUnownedPage(3, 0, []byte{0x77}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	model, _, code := referenceProgramUnowned(model, 3, 0, []byte{0x77}, runtimeModelCut(Cut{Kind: nandpageprogramcuts.Complete}))
	if code != "" {
		t.Fatal(code)
	}
	assertEvacuationState(t, device, model)
	return device, model
}
