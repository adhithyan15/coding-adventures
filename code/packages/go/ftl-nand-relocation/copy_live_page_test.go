package ftlnandrelocation

import (
	"bytes"
	"errors"
	"fmt"
	"reflect"
	"testing"

	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestCopyLivePageAllCutsAndPrefixesPreserveSource(t *testing.T) {
	base, model, plan := copyTestDevice(t)
	move := plan.Copies[0]
	before := base.Snapshot()
	cuts := []Cut{
		{Kind: nandpageprogramcuts.BeforeAccept},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1},
		{Kind: nandpageprogramcuts.AfterMedia},
		{Kind: nandpageprogramcuts.Complete},
	}
	for _, cut := range cuts {
		t.Run(fmt.Sprintf("kind-%d-prefix-%d", cut.Kind, cut.PrefixBytes), func(t *testing.T) {
			want, wantReceipt, wantErr := referenceCopyLive(model, 0, move, runtimeModelCut(cut))
			got, receipt, err := base.CopyLivePage(0, move, cut)
			assertError(t, err, wantErr)
			if err != nil {
				t.Fatalf("copy failed: %v", err)
			}
			if receipt != (Receipt{Physical: wantReceipt.physical, Published: wantReceipt.published, Acknowledged: wantReceipt.acknowledged}) {
				t.Fatalf("receipt=%+v want=%+v", receipt, wantReceipt)
			}
			assertDeviceEquals(t, got, want)
			snapshot := got.Snapshot()
			if !bytes.Equal(snapshot.Pages[int(move.SourcePhysical)].Bytes, before.Pages[int(move.SourcePhysical)].Bytes) || !snapshot.Pages[int(move.SourcePhysical)].Programmed {
				t.Fatal("copy changed or cleared its source page")
			}
			physical := move.SourcePhysical
			if cut.Kind == nandpageprogramcuts.AfterMedia || cut.Kind == nandpageprogramcuts.Complete {
				physical = move.DestinationPhysical
			}
			data, present, readErr := got.Read(move.Logical)
			if readErr != nil || !present || !bytes.Equal(data, before.Pages[int(move.SourcePhysical)].Bytes) {
				t.Fatalf("post-copy read=%x/%t/%v want source bytes at physical %d", data, present, readErr, physical)
			}
			if !reflect.DeepEqual(base.Snapshot(), before) {
				t.Fatal("copy mutated the predecessor device")
			}
		})
	}
}

func TestCopyAllOnesPrefixUsesProgramMarker(t *testing.T) {
	device, model, plan := copyTestDevice(t)
	move := plan.Copies[1]
	before := device.Snapshot()
	device, model = compareCopyLive(t, device, model, 0, move, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	snapshot := device.Snapshot()
	destination := snapshot.Pages[int(move.DestinationPhysical)]
	if !bytes.Equal(destination.Bytes, []byte{0xff, 0xff}) || !destination.Programmed {
		t.Fatalf("zero-prefix all-ones destination=%x/%t", destination.Bytes, destination.Programmed)
	}
	if snapshot.Map.LogicalToPhysical[int(move.Logical)] != move.SourcePhysical || !bytes.Equal(snapshot.Pages[int(move.SourcePhysical)].Bytes, before.Pages[int(move.SourcePhysical)].Bytes) {
		t.Fatal("partial all-ones copy published or changed the source")
	}
	assertDeviceEquals(t, device, model)
}

func TestCopyRechecksStaleSourceAndOccupiedDestination(t *testing.T) {
	base, model, plan := copyTestDevice(t)
	move := plan.Copies[0]

	changed, changedModel := compareTransition(t, base, model, move.Logical, []byte{0x71, 0x72}, modelCut{kind: cutComplete})
	before := changed.Snapshot()
	refused, receipt, err := changed.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.Complete})
	if !errors.Is(err, ErrCopySourceChanged) || receipt != (Receipt{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
		t.Fatalf("stale plan source result=%+v/%v", receipt, err)
	}
	assertDeviceEquals(t, changed, changedModel)

	unrelated, unrelatedModel := compareTransition(t, base, model, 3, []byte{0x81, 0x82}, modelCut{kind: cutComplete})
	before = unrelated.Snapshot()
	refused, receipt, err = unrelated.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.Complete})
	if !errors.Is(err, ErrCopyDestinationUnavailable) || receipt != (Receipt{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
		t.Fatalf("occupied planned destination result=%+v/%v", receipt, err)
	}
	assertDeviceEquals(t, unrelated, unrelatedModel)
}

func TestCopyRefusesDestinationInIncompleteBlock(t *testing.T) {
	device, model, _ := copyTestDevice(t)
	for _, payload := range [][]byte{{0x61, 0x62}, {0x71, 0x72}} {
		device, model = compareTransition(t, device, model, 1, payload, modelCut{kind: cutComplete})
	}
	planned, err := ftlevacuationplan.PlanEvacuation(device.table, 3, 1, []bool{false, false, false})
	if err != nil || len(planned.Copies) != 1 {
		t.Fatalf("block-one plan=%+v err=%v", planned, err)
	}
	device, model = compareCopyLive(t, device, model, 1, planned.Copies[0], Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEraseBlock(t, device, model, 1, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	move := ftlevacuationplan.Copy{Logical: 0, SourcePhysical: 0, DestinationPhysical: 4}
	before := device.Snapshot()
	refused, receipt, err := device.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.Complete})
	if !errors.Is(err, ErrCopyDestinationUnavailable) || receipt != (Receipt{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
		t.Fatalf("incomplete destination did not preserve the device: receipt=%+v err=%v", receipt, err)
	}
	assertDeviceEquals(t, device, model)
}

func TestSpentCopyDestinationCannotBeRetried(t *testing.T) {
	device, model, plan := copyTestDevice(t)
	move := plan.Copies[0]
	device, model = compareCopyLive(t, device, model, 0, move, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	before := device.Snapshot()
	refused, receipt, err := device.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.Complete})
	if !errors.Is(err, ErrCopyDestinationUnavailable) || receipt != (Receipt{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
		t.Fatalf("spent destination retry did not return unavailable refusal: receipt=%+v err=%v", receipt, err)
	}
	assertDeviceEquals(t, device, model)
}

func TestCopyAfterMediaPublishesWithoutAcknowledgement(t *testing.T) {
	device, model, plan := copyTestDevice(t)
	move := plan.Copies[0]
	device, model = compareCopyLive(t, device, model, 0, move, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if mapping := device.Snapshot().Map.LogicalToPhysical[int(move.Logical)]; mapping != move.DestinationPhysical {
		t.Fatalf("AfterMedia mapping=%d want destination %d", mapping, move.DestinationPhysical)
	}
	if page := device.Snapshot().Pages[int(move.SourcePhysical)]; !page.Programmed || !bytes.Equal(page.Bytes, []byte{0x10, 0x20}) {
		t.Fatalf("AfterMedia destroyed source page: %+v", page)
	}
	if _, _, err := device.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.Complete}); !errors.Is(err, ErrCopySourceChanged) {
		t.Fatalf("published copy retry source error=%v", err)
	}
	assertDeviceEquals(t, device, model)
}

func TestCopyUsesExactCurrentSourceBytes(t *testing.T) {
	device, _, plan := copyTestDevice(t)
	move := plan.Copies[0]
	source := append([]byte(nil), device.pages[int(move.SourcePhysical)].Bytes...)
	next, _, err := device.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil {
		t.Fatal(err)
	}
	if got := next.pages[int(move.DestinationPhysical)].Bytes; !bytes.Equal(got, source) {
		t.Fatalf("copy destination does not contain source page bytes: got %x want %x", got, source)
	}
}

func TestCopyPartialCutNeverPublishes(t *testing.T) {
	device, _, plan := copyTestDevice(t)
	move := plan.Copies[0]
	next, _, err := device.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1})
	if err != nil {
		t.Fatal(err)
	}
	if got := next.Snapshot().Map.LogicalToPhysical[int(move.Logical)]; got != move.SourcePhysical {
		t.Fatalf("partial copy changed logical mapping: got %d want %d", got, move.SourcePhysical)
	}
}

func TestCopyLivePageValidationOrderAndRefusals(t *testing.T) {
	base, _, plan := copyTestDevice(t)
	move := plan.Copies[0]
	var zero Device
	if _, receipt, err := zero.CopyLivePage(0, move, Cut{}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || receipt != (Receipt{}) {
		t.Fatalf("unconfigured copy precedence: %+v %v", receipt, err)
	}
	legacy, _ := New(1, 3, 2)
	if _, receipt, err := legacy.CopyLivePage(-1, move, Cut{}); !errors.Is(err, ErrEraseDisabled) || receipt != (Receipt{}) {
		t.Fatalf("legacy profile precedence: %+v %v", receipt, err)
	}
	if _, receipt, err := base.CopyLivePage(-1, ftlevacuationplan.Copy{}, Cut{}); !errors.Is(err, ErrInvalidBlockIndex) || receipt != (Receipt{}) {
		t.Fatalf("block index precedence: %+v %v", receipt, err)
	}

	partial, _ := NewWithEraseBlocks(1, 8, 1, 4, 2)
	model := referenceInitialWithEraseBlocks(1, 8, 1, 4, 2)
	for index := 0; index < 5; index++ {
		partial, model = compareTransition(t, partial, model, 0, []byte{byte(index + 1)}, modelCut{kind: cutComplete})
	}
	partial, model = compareEraseBlock(t, partial, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if _, receipt, err := partial.CopyLivePage(0, ftlevacuationplan.Copy{}, Cut{}); !errors.Is(err, ErrEvacuationBlockIncomplete) || receipt != (Receipt{}) {
		t.Fatalf("incomplete block precedence: %+v %v", receipt, err)
	}

	for _, testCase := range []struct {
		name string
		move ftlevacuationplan.Copy
		want error
	}{
		{"invalid logical before source", ftlevacuationplan.Copy{Logical: -1}, ftlpagemap.ErrInvalidLogicalPage},
		{"source outside target", ftlevacuationplan.Copy{Logical: move.Logical, SourcePhysical: 3, DestinationPhysical: move.DestinationPhysical}, ErrInvalidCopySource},
		{"source outside geometry", ftlevacuationplan.Copy{Logical: move.Logical, SourcePhysical: 99, DestinationPhysical: move.DestinationPhysical}, ErrInvalidCopySource},
		{"destination outside geometry", ftlevacuationplan.Copy{Logical: move.Logical, SourcePhysical: move.SourcePhysical, DestinationPhysical: 99}, ErrInvalidCopyDestination},
		{"destination inside target", ftlevacuationplan.Copy{Logical: move.Logical, SourcePhysical: move.SourcePhysical, DestinationPhysical: 1}, ErrInvalidCopyDestination},
		{"invalid cut after admission", move, nandpageprogramcuts.ErrInvalidCut},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			before := base.Snapshot()
			refused, receipt, err := base.CopyLivePage(0, testCase.move, Cut{Kind: nandpageprogramcuts.CutKind(255)})
			if !errors.Is(err, testCase.want) || receipt != (Receipt{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
				t.Fatalf("copy refusal=%+v/%v want=%v", receipt, err, testCase.want)
			}
		})
	}
}

func FuzzLivePageCopyHistory(f *testing.F) {
	f.Add([]byte{0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12})
	f.Add([]byte{1, 0xff, 0, 0xff, 2, 0xff, 3, 0xff, 4, 0xff})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			return
		}
		device, model, _ := copyTestDevice(t)
		for index := 0; index+1 < len(commands); index += 2 {
			op := commands[index] % 4
			value := commands[index+1]
			switch op {
			case 0:
				logical := int64(value % 4)
				payload := []byte{value, value ^ 0x5a}
				device, model = compareTransition(t, device, model, logical, payload, modelCut{kind: cutComplete})
			case 1:
				moves, code := referenceCopyAssignments(model, 3, 0, []bool{false, false, true})
				if code == "" && len(moves) != 0 {
					move := moves[int(value)%len(moves)]
					cut := fuzzCopyCut(commands[(index+2)%len(commands)])
					device, model = compareCopyLive(t, device, model, 0, move, cut)
				}
			case 2:
				staleMove := ftlevacuationplan.Copy{Logical: int64(value % 4), SourcePhysical: int64(value % 3), DestinationPhysical: 4 + int64(value%2)}
				device, model = compareCopyLive(t, device, model, 0, staleMove, Cut{Kind: nandpageprogramcuts.Complete})
			case 3:
				logical := int64(value % 4)
				got, present, err := device.Read(logical)
				want, wanted, code := referenceRead(model, logical)
				if code != "" || err != nil || present != wanted || !bytes.Equal(got, want) {
					t.Fatalf("read(%d)=%x/%t/%v want %x/%t/%s", logical, got, present, err, want, wanted, code)
				}
			}
		}
	})
}

func fuzzCopyCut(selector byte) Cut {
	switch selector % 5 {
	case 0:
		return Cut{Kind: nandpageprogramcuts.BeforeAccept}
	case 1:
		return Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}
	case 2:
		return Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1}
	case 3:
		return Cut{Kind: nandpageprogramcuts.AfterMedia}
	default:
		return Cut{Kind: nandpageprogramcuts.Complete}
	}
}

func copyTestDevice(t *testing.T) (Device, referenceState, ftlevacuationplan.Plan) {
	t.Helper()
	device, err := NewWithEraseBlocks(4, 9, 2, 3, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(4, 9, 2, 3, 2)
	for _, item := range []struct {
		logical int64
		payload []byte
	}{
		{0, []byte{0x10, 0x20}},
		{1, []byte{0x30, 0x40}},
		{2, []byte{0xff, 0xff}},
		{1, []byte{0x50, 0x60}},
	} {
		device, model = compareTransition(t, device, model, item.logical, item.payload, modelCut{kind: cutComplete})
	}
	plan, err := ftlevacuationplan.PlanEvacuation(device.table, 3, 0, []bool{false, false, true})
	if err != nil || len(plan.Copies) != 2 || plan.Copies[0] != (ftlevacuationplan.Copy{Logical: 0, SourcePhysical: 0, DestinationPhysical: 4}) || plan.Copies[1] != (ftlevacuationplan.Copy{Logical: 2, SourcePhysical: 2, DestinationPhysical: 5}) {
		t.Fatalf("test evacuation plan=%+v err=%v", plan, err)
	}
	return device, model, plan
}

func compareCopyLive(t *testing.T, device Device, before referenceState, block int64, move ftlevacuationplan.Copy, cut Cut) (Device, referenceState) {
	t.Helper()
	want, receipt, wantError := referenceCopyLive(before, block, move, runtimeModelCut(cut))
	got, actualReceipt, err := device.CopyLivePage(block, move, cut)
	assertError(t, err, wantError)
	if wantError != "" {
		if actualReceipt != (Receipt{}) || !reflect.DeepEqual(got.Snapshot(), device.Snapshot()) {
			t.Fatal("refused copy changed the input device")
		}
		assertLookupMatchesModel(t, device, before)
		return device, before
	}
	if actualReceipt != (Receipt{Physical: receipt.physical, Published: receipt.published, Acknowledged: receipt.acknowledged}) {
		t.Fatalf("copy receipt=%+v want=%+v", actualReceipt, receipt)
	}
	assertDeviceEquals(t, got, want)
	return got, want
}

func runtimeModelCut(cut Cut) modelCut {
	kind := 255
	switch cut.Kind {
	case nandpageprogramcuts.BeforeAccept:
		kind = cutBeforeAccept
	case nandpageprogramcuts.AfterPrefix:
		kind = cutAfterPrefix
	case nandpageprogramcuts.AfterMedia:
		kind = cutAfterMedia
	case nandpageprogramcuts.Complete:
		kind = cutComplete
	}
	return modelCut{kind: kind, prefix: cut.PrefixBytes}
}

func referenceCopyLive(before referenceState, block int64, move ftlevacuationplan.Copy, cut modelCut) (referenceState, modelReceipt, string) {
	return referenceCopyLiveInternal(before, block, move, cut, false)
}

func referenceCopyLiveInternal(before referenceState, block int64, move ftlevacuationplan.Copy, cut modelCut, allowActive bool) (referenceState, modelReceipt, string) {
	if before.logicalCount < 1 || before.logicalCount > 64 || before.physicalCount < 2 || before.physicalCount > 128 || before.physicalCount <= before.logicalCount || before.pageSize < 1 || before.pageSize > maxPageSize {
		return before, modelReceipt{}, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return before, modelReceipt{}, "erase_disabled"
	}
	if block < 0 || block >= int64(len(before.blocks)) {
		return before, modelReceipt{}, "invalid_block_index"
	}
	if before.evacuationActive && block == before.evacuationBlock && !allowActive {
		return before, modelReceipt{}, "evacuation_active"
	}
	if before.blocks[int(block)].IncompleteErase {
		return before, modelReceipt{}, "evacuation_block_incomplete"
	}
	if move.Logical < 0 || move.Logical >= before.logicalCount {
		return before, modelReceipt{}, "invalid_logical_page"
	}
	first := block * before.pagesPerBlock
	if move.SourcePhysical < 0 || move.SourcePhysical >= before.physicalCount || move.SourcePhysical < first || move.SourcePhysical >= first+before.pagesPerBlock {
		return before, modelReceipt{}, "invalid_copy_source"
	}
	if before.mapping[int(move.Logical)] != move.SourcePhysical || before.status[int(move.SourcePhysical)] != modelLive {
		return before, modelReceipt{}, "copy_source_changed"
	}
	if move.DestinationPhysical < 0 || move.DestinationPhysical >= before.physicalCount ||
		(move.DestinationPhysical >= first && move.DestinationPhysical < first+before.pagesPerBlock) {
		return before, modelReceipt{}, "invalid_copy_destination"
	}
	destinationBlock := int(move.DestinationPhysical / before.pagesPerBlock)
	if (before.evacuationActive && int64(destinationBlock) == before.evacuationBlock) ||
		before.blocks[destinationBlock].IncompleteErase || before.status[int(move.DestinationPhysical)] != modelFree {
		return before, modelReceipt{}, "copy_destination_unavailable"
	}
	if !validModelCut(cut, before.pageSize) {
		return before, modelReceipt{}, "invalid_cut"
	}

	result := cloneReference(before)
	receipt := modelReceipt{physical: move.DestinationPhysical}
	sourceBytes := before.pages[int(move.SourcePhysical)]
	destination := int(move.DestinationPhysical)
	switch cut.kind {
	case cutBeforeAccept:
	case cutAfterPrefix:
		copy(result.pages[destination][:cut.prefix], sourceBytes[:cut.prefix])
		result.programmed[destination] = true
		result.status[destination] = modelStale
	case cutAfterMedia, cutComplete:
		copy(result.pages[destination], sourceBytes)
		result.programmed[destination] = true
		result.status[destination] = modelLive
		result.status[int(move.SourcePhysical)] = modelStale
		result.mapping[int(move.Logical)] = move.DestinationPhysical
		receipt.published = true
		receipt.acknowledged = cut.kind == cutComplete
	}
	return result, receipt, ""
}
