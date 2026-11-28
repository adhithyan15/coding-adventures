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

func TestCopyNextEvacuationPageExcludingUsesCurrentFenceAndExactReceipt(t *testing.T) {
	device, model := prepareCopyFenceBase(t)
	mask := []bool{false, true, false}
	beforeMask := append([]bool(nil), mask...)
	plan, err := ftlevacuationplan.PlanEvacuation(device.table, 2, 0, mask)
	if err != nil || !reflect.DeepEqual(plan.Copies, []ftlevacuationplan.Copy{{Logical: 0, SourcePhysical: 1, DestinationPhysical: 5}}) {
		t.Fatalf("reserved pages 2/3 plan=%+v err=%v", plan, err)
	}
	device, model = compareEvacuationCopyExcluding(t, device, model, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, mask)
	if device.Snapshot().Map.PhysicalStatus[5] != ftlpagemap.Stale || device.Snapshot().Map.LogicalToPhysical[0] != 1 {
		t.Fatal("prefix did not spend page 5 while retaining its source owner")
	}
	before := device.Snapshot()
	got, receipt, err := device.CopyNextEvacuationPageExcluding(Cut{Kind: nandpageprogramcuts.Complete}, mask)
	if !errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("spent sole destination retry receipt=%+v err=%v", receipt, err)
	}
	assertEvacuationState(t, got, model)
	if !reflect.DeepEqual(got.Snapshot(), before) {
		t.Fatal("shortage refusal changed the device")
	}
	if !reflect.DeepEqual(mask, beforeMask) {
		t.Fatal("copy transition changed caller-owned block mask")
	}
}

func TestCopyNextEvacuationPageExcludingPreflightsEveryLiveSource(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 9, 1, 3, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 9, 1, 3, 2)
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x10}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x11}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationWrite(t, device, model, 1, []byte{0x21}, Cut{Kind: nandpageprogramcuts.Complete})
	for page := int64(0); page < 2; page++ {
		device, model = compareProgramUnowned(t, device, model, 2, page, []byte{byte(0x30 + page)}, Cut{Kind: nandpageprogramcuts.Complete})
	}
	device, model = compareEvacuationBegin(t, device, model, 0)
	mask := []bool{false, true, false}
	before := device.Snapshot()
	got, receipt, err := device.CopyNextEvacuationPageExcluding(Cut{Kind: nandpageprogramcuts.Complete}, mask)
	want, _, wantErr := referenceCopyNextExcluding(model, runtimeModelCut(Cut{Kind: nandpageprogramcuts.Complete}), mask)
	assertError(t, err, wantErr)
	if !errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("one spare for two live sources receipt=%+v err=%v", receipt, err)
	}
	assertEvacuationState(t, got, want)
	if !reflect.DeepEqual(got.Snapshot(), before) {
		t.Fatal("planner shortage partially copied a live source")
	}
}

func TestCopyNextEvacuationPageExcludingValidationOrderAndEmptyTarget(t *testing.T) {
	var zero Device
	if _, receipt, err := zero.CopyNextEvacuationPageExcluding(Cut{}, []bool{true}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("geometry refusal=%+v/%v", receipt, err)
	}
	legacy, _ := New(1, 4, 1)
	if _, receipt, err := legacy.CopyNextEvacuationPageExcluding(Cut{}, []bool{true}); !errors.Is(err, ErrEraseDisabled) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("profile refusal=%+v/%v", receipt, err)
	}
	configured, _ := NewWithEraseBlocks(1, 4, 1, 2, 2)
	if _, receipt, err := configured.CopyNextEvacuationPageExcluding(Cut{}, []bool{true}); !errors.Is(err, ErrNoActiveEvacuation) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("inactive device must precede mask width: %+v/%v", receipt, err)
	}

	empty, err := NewWithEraseBlocks(1, 4, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	empty, _, err = empty.Write(0, []byte{1}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	empty, _, err = empty.Write(0, []byte{2}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	empty, _, err = empty.Write(0, []byte{3}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	empty, err = empty.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	if progress, active := empty.EvacuationProgress(); !active || progress.RemainingLivePages != 0 {
		t.Fatalf("empty active target=%+v/%t", progress, active)
	}
	before := empty.Snapshot()
	if _, receipt, err := empty.CopyNextEvacuationPageExcluding(Cut{Kind: nandpageprogramcuts.Complete}, []bool{true}); !errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("active target validates mask before no-live check: %+v/%v", receipt, err)
	}
	if got, receipt, err := empty.CopyNextEvacuationPageExcluding(Cut{Kind: nandpageprogramcuts.Complete}, make([]bool, 2)); !errors.Is(err, ErrEvacuationNoLivePages) || receipt != (EvacuationCopyReceipt{}) || !reflect.DeepEqual(got.Snapshot(), before) {
		t.Fatalf("valid mask empty target=%+v/%v", receipt, err)
	}
}

func TestCopyNextEvacuationPageExcludingBoundedMasks(t *testing.T) {
	base, model := prepareCopyFenceBase(t)
	for bits := 0; bits < 1<<3; bits++ {
		mask := make([]bool, 3)
		for index := range mask {
			mask[index] = bits&(1<<index) != 0
		}
		for _, cut := range []Cut{
			{Kind: nandpageprogramcuts.BeforeAccept},
			{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
			{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1},
			{Kind: nandpageprogramcuts.AfterMedia},
			{Kind: nandpageprogramcuts.Complete},
			{Kind: nandpageprogramcuts.CutKind(255)},
		} {
			compareEvacuationCopyExcluding(t, base, model, cut, mask)
		}
	}
}

func FuzzCopyNextEvacuationPageExcludingHistory(f *testing.F) {
	f.Add([]byte{0, 0, 1, 0, 2, 1, 3, 2, 4, 0, 5, 1, 6, 2, 7, 3})
	f.Add([]byte{0xff, 0x34, 0x91, 0x02, 0x77, 0xa5, 0x8b, 0x10, 0x03, 0xfe})
	f.Add([]byte{0, 0, 0, 2, 1, 0, 2, 0, 2, 1, 2, 2, 2, 3, 2, 4, 2, 5})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			return
		}
		device, err := NewWithEraseBlocks(2, 6, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 2)
		for index := 0; index+1 < len(commands); index += 2 {
			op, value := commands[index]%8, commands[index+1]
			switch op {
			case 0:
				device, model = compareEvacuationWrite(t, device, model, int64(value%2), []byte{value, value ^ 0xff}, Cut{Kind: nandpageprogramcuts.Complete})
			case 1:
				device, model = compareEvacuationBegin(t, device, model, int64(value%3))
			case 2, 3:
				mask := make([]bool, len(model.blocks))
				for block := range mask {
					mask[block] = value&(1<<block) != 0
				}
				cuts := []Cut{
					{Kind: nandpageprogramcuts.BeforeAccept},
					{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
					{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1},
					{Kind: nandpageprogramcuts.AfterMedia},
					{Kind: nandpageprogramcuts.Complete},
					{Kind: nandpageprogramcuts.CutKind(255)},
				}
				device, model = compareEvacuationCopyExcluding(t, device, model, cuts[int(value)%len(cuts)], mask)
			case 4:
				mask := make([]bool, len(model.blocks)-1)
				got, receipt, gotErr := device.CopyNextEvacuationPageExcluding(Cut{Kind: nandpageprogramcuts.Complete}, mask)
				want, _, wantErr := referenceCopyNextExcluding(model, runtimeModelCut(Cut{Kind: nandpageprogramcuts.Complete}), mask)
				assertError(t, gotErr, wantErr)
				if gotErr == nil || receipt != (EvacuationCopyReceipt{}) {
					t.Fatalf("short mask result=%+v/%v", receipt, gotErr)
				}
				assertEvacuationState(t, got, want)
			case 5:
				device, model = compareEvacuationCancel(t, device, model)
			case 6:
				cuts := []EraseCut{{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}, {Kind: nandblockerasecuts.Complete}}
				want, receipt, wantErr := referenceFinishEvacuation(model, cuts[int(value)%len(cuts)])
				got, gotReceipt, gotErr := device.FinishEvacuation(cuts[int(value)%len(cuts)])
				assertError(t, gotErr, wantErr)
				if gotErr == nil && gotReceipt != receipt {
					t.Fatalf("finish receipt=%+v want %+v", gotReceipt, receipt)
				}
				assertEvacuationState(t, got, want)
				device, model = got, want
			case 7:
				device, model = compareEvacuationWrite(t, device, model, int64(value%2), []byte{value, value ^ 0xff}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
			}
		}
	})
}

func TestSeededCopyNextEvacuationPageExcludingHistories(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB05e2b2e3b))
	for _, geometry := range []struct{ logical, physical int64 }{{1, 4}, {2, 6}} {
		device, err := NewWithEraseBlocks(geometry.logical, geometry.physical, 1, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(geometry.logical, geometry.physical, 1, 2, 2)
		blocks := int(geometry.physical / 2)
		for step := 0; step < 5000; step++ {
			mask := make([]bool, blocks)
			for block := range mask {
				mask[block] = rng.Intn(2) == 1
			}
			switch rng.Intn(5) {
			case 0:
				device, model = compareEvacuationWrite(t, device, model, int64(rng.Intn(int(geometry.logical))), []byte{byte(rng.Intn(256))}, Cut{Kind: nandpageprogramcuts.Complete})
			case 1:
				device, model = compareEvacuationBegin(t, device, model, int64(rng.Intn(blocks)))
			case 2:
				cuts := []Cut{{Kind: nandpageprogramcuts.BeforeAccept}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, {Kind: nandpageprogramcuts.Complete}, {Kind: nandpageprogramcuts.AfterMedia}}
				device, model = compareEvacuationCopyExcluding(t, device, model, cuts[rng.Intn(len(cuts))], mask)
			case 3:
				device, model = compareEvacuationCancel(t, device, model)
			case 4:
				cut := EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}
				want, receipt, wantErr := referenceFinishEvacuation(model, cut)
				got, gotReceipt, gotErr := device.FinishEvacuation(cut)
				assertError(t, gotErr, wantErr)
				if gotErr == nil && gotReceipt != receipt {
					t.Fatalf("finish receipt=%+v want %+v", gotReceipt, receipt)
				}
				assertEvacuationState(t, got, want)
				device, model = got, want
			}
		}
	}
}

func compareEvacuationCopyExcluding(t *testing.T, device Device, model referenceState, cut Cut, mask []bool) (Device, referenceState) {
	t.Helper()
	beforeMask := append([]bool(nil), mask...)
	want, wantReceipt, wantErr := referenceCopyNextExcluding(model, runtimeModelCut(cut), mask)
	got, receipt, err := device.CopyNextEvacuationPageExcluding(cut, mask)
	assertError(t, err, wantErr)
	if err == nil {
		if receipt != wantReceipt {
			t.Fatalf("fenced copy receipt=%+v want %+v", receipt, wantReceipt)
		}
	} else if receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("refused fenced copy returned receipt %+v", receipt)
	}
	assertEvacuationState(t, got, want)
	if !reflect.DeepEqual(mask, beforeMask) {
		t.Fatal("copy transition changed caller-owned block mask")
	}
	return got, want
}

func prepareCopyFenceBase(t *testing.T) (Device, referenceState) {
	t.Helper()
	device, err := NewWithEraseBlocks(1, 6, 2, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 6, 2, 2, 2)
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x10, 0xef}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x11, 0xee}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareProgramUnowned(t, device, model, 2, 0, []byte{0x22, 0xdd}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationBegin(t, device, model, 0)
	return device, model
}

func referenceCopyNextExcluding(before referenceState, cut modelCut, destinationExcluded []bool) (referenceState, EvacuationCopyReceipt, string) {
	if !referenceConfigured(before) {
		return before, EvacuationCopyReceipt{}, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return before, EvacuationCopyReceipt{}, "erase_disabled"
	}
	if !before.evacuationActive {
		return before, EvacuationCopyReceipt{}, "no_active_evacuation"
	}
	if len(destinationExcluded) != 0 && len(destinationExcluded) != len(before.blocks) {
		return before, EvacuationCopyReceipt{}, "invalid_evacuation_block_mask"
	}
	progress, _ := referenceEvacuationProgress(before)
	if progress.RemainingLivePages == 0 {
		return before, EvacuationCopyReceipt{}, "evacuation_no_live_pages"
	}
	fences := referenceIncompleteMask(before)
	for index, excluded := range destinationExcluded {
		if excluded {
			fences[index] = true
		}
	}
	fences[int(before.evacuationBlock)] = false
	copies, code := referenceCopyAssignments(before, before.pagesPerBlock, before.evacuationBlock, fences)
	if code != "" {
		return before, EvacuationCopyReceipt{}, code
	}
	if len(copies) == 0 {
		return before, EvacuationCopyReceipt{}, "evacuation_no_live_pages"
	}
	next, receipt, code := referenceCopyLiveInternal(before, before.evacuationBlock, copies[0], cut, true)
	if code != "" {
		return before, EvacuationCopyReceipt{}, code
	}
	return next, EvacuationCopyReceipt{Move: copies[0], Receipt: Receipt{Physical: receipt.physical, Published: receipt.published, Acknowledged: receipt.acknowledged}}, ""
}
