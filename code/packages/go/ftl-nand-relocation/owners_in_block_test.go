package ftlnandrelocation

import (
	"errors"
	"math"
	"math/rand"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func referenceOwnersInBlock(state referenceState, block int64) ([]PageOwner, string) {
	if state.logicalCount < 1 || state.logicalCount > 64 ||
		state.physicalCount < 2 || state.physicalCount > 128 ||
		state.physicalCount <= state.logicalCount || state.pageSize < 1 || state.pageSize > maxPageSize {
		return nil, "invalid_geometry"
	}
	if state.pagesPerBlock == 0 {
		return nil, "erase_disabled"
	}
	if block < 0 || block >= int64(len(state.blocks)) {
		return nil, "invalid_block_index"
	}
	first := block * state.pagesPerBlock
	end := first + state.pagesPerBlock
	owners := make([]PageOwner, 0)
	for logical, physical := range state.mapping {
		if physical >= first && physical < end && state.status[int(physical)] == modelLive {
			owners = append(owners, PageOwner{Logical: int64(logical), Physical: physical})
		}
	}
	return owners, ""
}

func compareOwnersInBlock(t *testing.T, device Device, model referenceState, block int64) []PageOwner {
	t.Helper()
	before := device.Snapshot()
	beforeBlocks := device.ReclamationSnapshot()
	want, wantError := referenceOwnersInBlock(model, block)
	got, err := device.OwnersInBlock(block)
	assertError(t, err, wantError)
	if wantError != "" {
		if got != nil {
			t.Fatalf("refused owner query returned %#v, want nil", got)
		}
	} else {
		if !reflect.DeepEqual(got, want) {
			t.Fatalf("OwnersInBlock(%d) = %+v, want %+v", block, got, want)
		}
		live, liveErr := device.LivePagesInBlock(block)
		if liveErr != nil || len(got) != int(live) {
			t.Fatalf("owner count=%d live count=%d error=%v", len(got), live, liveErr)
		}
	}
	if !reflect.DeepEqual(device.Snapshot(), before) || !reflect.DeepEqual(device.ReclamationSnapshot(), beforeBlocks) {
		t.Fatal("owner inventory changed device media or block progress")
	}
	return got
}

func TestOwnersInBlockValidationOrderAndSignedBoundaries(t *testing.T) {
	var zero Device
	if got, err := zero.OwnersInBlock(math.MinInt64); got != nil || !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("unconfigured precedence: owners=%v err=%v", got, err)
	}
	legacy, err := New(1, 4, 1)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := legacy.OwnersInBlock(math.MinInt64); got != nil || !errors.Is(err, ErrEraseDisabled) {
		t.Fatalf("erase profile must precede address: owners=%v err=%v", got, err)
	}
	device, err := NewWithEraseBlocks(1, 4, 1, 2, 0)
	if err != nil {
		t.Fatal(err)
	}
	for _, block := range []int64{math.MinInt64, -1, 2, math.MaxInt64} {
		if got, err := device.OwnersInBlock(block); got != nil || !errors.Is(err, ErrInvalidBlockIndex) {
			t.Errorf("invalid signed block %d: owners=%v err=%v", block, got, err)
		}
	}
	device.pagesPerBlock = -2
	if got, err := device.OwnersInBlock(0); got != nil || !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("malformed erase profile must precede address: owners=%v err=%v", got, err)
	}
}

func TestOwnerInventoryUsesCurrentLogicalMapAndAscendingLogicalOrder(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 6, 1, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 6, 1, 2, 1)
	compareOwnersInBlock(t, device, model, 0)

	// Reverse write order so physical order is opposite to logical order.
	device, model = compareTransition(t, device, model, 1, []byte{0xff}, modelCut{kind: cutComplete})
	device, model = compareTransition(t, device, model, 0, []byte{0x37}, modelCut{kind: cutComplete})
	if got, want := compareOwnersInBlock(t, device, model, 0), []PageOwner{{Logical: 0, Physical: 1}, {Logical: 1, Physical: 0}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("owner list did not sort by logical id: got=%+v want=%+v", got, want)
	}
	if got, err := device.OwnersInBlock(1); err != nil || got == nil || len(got) != 0 {
		t.Fatalf("empty valid block inventory=%#v error=%v; want detached empty slice", got, err)
	}

	// The current owner moves to the first physical slot of block 1. The old
	// logical mapping at physical 1 is stale and must disappear from block 0.
	device, model = compareTransition(t, device, model, 0, []byte{0x48}, modelCut{kind: cutComplete})
	if got, want := compareOwnersInBlock(t, device, model, 0), []PageOwner{{Logical: 1, Physical: 0}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("stale old owner remained in block 0: got=%+v want=%+v", got, want)
	}
	if got, want := compareOwnersInBlock(t, device, model, 1), []PageOwner{{Logical: 0, Physical: 2}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("first-page boundary owner omitted: got=%+v want=%+v", got, want)
	}
}

func TestOwnerInventoryTracksPublishedCopyAndEraseCuts(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 6, 2, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 1)
	device, model = compareTransition(t, device, model, 1, []byte{0xff, 0xff}, modelCut{kind: cutComplete})
	device, model = compareTransition(t, device, model, 0, []byte{0x10, 0x11}, modelCut{kind: cutComplete})
	before := device
	beforeModel := model
	beforeOwners := compareOwnersInBlock(t, before, model, 0)

	// An interrupted replacement consumes a page but does not publish it.
	device, model = compareTransition(t, device, model, 1, []byte{0x20, 0x21}, modelCut{kind: cutAfterPrefix, prefix: 0})
	if got := compareOwnersInBlock(t, device, model, 0); !reflect.DeepEqual(got, beforeOwners) {
		t.Fatalf("partial replacement changed source owner list: %+v", got)
	}

	// AfterMedia publishes the new current owner without acknowledgement.
	device, model = compareTransition(t, device, model, 1, []byte{0x30, 0x31}, modelCut{kind: cutAfterMedia})
	if got, want := compareOwnersInBlock(t, device, model, 0), []PageOwner{{Logical: 0, Physical: 1}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("old owner remained after media publication: got=%+v want=%+v", got, want)
	}
	if got, want := compareOwnersInBlock(t, device, model, 1), []PageOwner{{Logical: 1, Physical: 3}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("unacknowledged published owner missing: got=%+v want=%+v", got, want)
	}

	// A separate publication empties block 0, leaving only stale pages there.
	device, model = compareTransition(t, device, model, 0, []byte{0x40, 0x41}, modelCut{kind: cutComplete})
	if got := compareOwnersInBlock(t, device, model, 0); len(got) != 0 {
		t.Fatalf("stale-only block returned owners: %+v", got)
	}
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.Complete})
	if got := compareOwnersInBlock(t, device, model, 0); len(got) != 0 {
		t.Fatalf("erased block acquired owners: %+v", got)
	}

	// The source can be inventoried while an evacuation is active. A partial
	// copy keeps its source owner, and media publication transfers ownership.
	device, model = compareEvacuationBegin(t, device, model, 1)
	if got, want := compareOwnersInBlock(t, device, model, 1), []PageOwner{{Logical: 1, Physical: 3}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("active source inventory=%+v want=%+v", got, want)
	}
	device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if got, want := compareOwnersInBlock(t, device, model, 1), []PageOwner{{Logical: 1, Physical: 3}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("partial copy changed source owner list: got=%+v want=%+v", got, want)
	}
	device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if got := compareOwnersInBlock(t, device, model, 1); len(got) != 0 {
		t.Fatalf("published copy left source owner: %+v", got)
	}
	device, model = compareFinishEvacuation(t, device, model, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	if got := compareOwnersInBlock(t, device, model, 1); got == nil || len(got) != 0 {
		t.Fatalf("incomplete-erase inventory=%#v; want valid empty list", got)
	}
	compareOwnersInBlock(t, device, model, 0)
	compareOwnersInBlock(t, device, model, 2)

	// Retained predecessor values still report their own historical owners.
	if got := compareOwnersInBlock(t, before, beforeModel, 0); !reflect.DeepEqual(got, beforeOwners) {
		t.Fatalf("later cut sequence changed predecessor inventory: got=%+v want=%+v", got, beforeOwners)
	}
}

func compareFinishEvacuation(t *testing.T, device Device, model referenceState, cut EraseCut) (Device, referenceState) {
	t.Helper()
	want, wantReceipt, wantError := referenceFinishEvacuation(model, cut)
	got, receipt, err := device.FinishEvacuation(cut)
	assertError(t, err, wantError)
	if wantError != "" {
		if receipt != (EraseReceipt{}) || !reflect.DeepEqual(got.Snapshot(), device.Snapshot()) {
			t.Fatalf("refused FinishEvacuation changed state: %+v/%v", receipt, err)
		}
		return device, model
	}
	if receipt != wantReceipt {
		t.Fatalf("FinishEvacuation receipt=%+v want=%+v", receipt, wantReceipt)
	}
	assertDeviceEquals(t, got, want)
	return got, want
}

func TestBoundedWriteCutHistoriesMatchOwnerModel(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 6, 2, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 1)
	cuts := []struct {
		cut   modelCut
		bytes []byte
	}{
		{modelCut{kind: cutBeforeAccept}, []byte{0xff, 0xff}},
		{modelCut{kind: cutAfterPrefix, prefix: 0}, []byte{0xff, 0xff}},
		{modelCut{kind: cutAfterPrefix, prefix: 1}, []byte{0x37, 0xff}},
		{modelCut{kind: cutAfterMedia}, []byte{0x71, 0x72}},
		{modelCut{kind: cutComplete}, []byte{0x81, 0x82}},
	}
	commands := make([]struct {
		logical int64
		payload []byte
		cut     modelCut
	}, 0, 10)
	for logical := int64(0); logical < 2; logical++ {
		for _, item := range cuts {
			commands = append(commands, struct {
				logical int64
				payload []byte
				cut     modelCut
			}{logical, item.bytes, item.cut})
		}
	}
	comparisons := 0
	var visit func(Device, referenceState, int)
	visit = func(current Device, state referenceState, depth int) {
		for block := int64(0); block < 3; block++ {
			compareOwnersInBlock(t, current, state, block)
			comparisons++
		}
		if depth == 3 {
			return
		}
		for _, command := range commands {
			next, nextState := compareTransition(t, current, state, command.logical, command.payload, command.cut)
			visit(next, nextState, depth+1)
		}
	}
	visit(device, model, 0)
	if comparisons != 3*(1+10+100+1000) {
		t.Fatalf("bounded owner/model comparisons=%d", comparisons)
	}
	t.Logf("bounded histories: 1,000 leaves, 1,111 nodes, %d block inventories", comparisons)
}

func TestSeededOwnerInventoriesMatchSetModel(t *testing.T) {
	const seed = 0x4f574e52
	rng := rand.New(rand.NewSource(seed))
	for history := 0; history < 512; history++ {
		device, err := NewWithEraseBlocks(3, 8, 2, 2, 1)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(3, 8, 2, 2, 1)
		for step := 0; step < 32; step++ {
			logical := int64(rng.Intn(3))
			payload := []byte{byte(rng.Intn(256)), byte(rng.Intn(256))}
			cut := randomInventoryWriteCut(rng)
			device, model = compareTransition(t, device, model, logical, payload, cut)
			for block := int64(0); block < 4; block++ {
				compareOwnersInBlock(t, device, model, block)
			}
		}
	}
}

func FuzzOwnersInBlockHistories(f *testing.F) {
	f.Add([]byte{0, 0xff, 0xff, 4, 1, 0x21, 0x32, 3, 0, 0x44, 0x55, 2})
	f.Add([]byte{1, 0x12, 0x34, 3, 0, 0xff, 0xff, 1})
	f.Add([]byte{})
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 128 {
			t.Skip()
		}
		device, err := NewWithEraseBlocks(2, 6, 2, 2, 1)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 1)
		for offset := 0; offset+3 < len(data); offset += 4 {
			logical := int64(data[offset] % 2)
			payload := []byte{data[offset+1], data[offset+2]}
			device, model = compareTransition(t, device, model, logical, payload, randomInventoryWriteCutValue(data[offset+3]))
			for block := int64(0); block < 3; block++ {
				compareOwnersInBlock(t, device, model, block)
			}
		}
	})
}

func randomInventoryWriteCut(rng *rand.Rand) modelCut {
	return randomInventoryWriteCutValue(byte(rng.Intn(5)))
}

func randomInventoryWriteCutValue(value byte) modelCut {
	switch value % 5 {
	case 0:
		return modelCut{kind: cutBeforeAccept}
	case 1:
		return modelCut{kind: cutAfterPrefix, prefix: 0}
	case 2:
		return modelCut{kind: cutAfterPrefix, prefix: 1}
	case 3:
		return modelCut{kind: cutAfterMedia}
	default:
		return modelCut{kind: cutComplete}
	}
}

func TestOwnerInventoryListIsDetached(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 4, 1, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 4, 1, 2, 1)
	device, model = compareTransition(t, device, model, 0, []byte{0x13}, modelCut{kind: cutComplete})
	got := compareOwnersInBlock(t, device, model, 0)
	got[0] = PageOwner{Logical: 1, Physical: 1}
	if again, err := device.OwnersInBlock(0); err != nil || !reflect.DeepEqual(again, []PageOwner{{Logical: 0, Physical: 0}}) {
		t.Fatalf("mutating returned owner list changed device: %+v, %v", again, err)
	}
}

func TestOwnerInventoryMaximumBlockAndZeroEraseBudget(t *testing.T) {
	device, err := NewWithEraseBlocks(64, 128, 1, 64, 0)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(64, 128, 1, 64, 0)
	for logical := int64(0); logical < 64; logical++ {
		device, model = compareTransition(t, device, model, logical, []byte{byte(logical)}, modelCut{kind: cutComplete})
	}
	owners := compareOwnersInBlock(t, device, model, 0)
	if len(owners) != 64 {
		t.Fatalf("maximum block owners=%d, want 64", len(owners))
	}
	for logical, owner := range owners {
		if owner.Logical != int64(logical) || owner.Physical != int64(logical) {
			t.Fatalf("owner[%d]=%+v; want logical and physical %d", logical, owner, logical)
		}
	}
	compareOwnersInBlock(t, device, model, 1)
}

func TestOwnerInventoryOnIncompleteBlockIsValidAndEmpty(t *testing.T) {
	device, model := prepareStaleFreeBlock(t)
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	if !device.ReclamationSnapshot().Blocks[0].IncompleteErase {
		t.Fatal("test did not establish an incomplete erase")
	}
	if got := compareOwnersInBlock(t, device, model, 0); got == nil || len(got) != 0 {
		t.Fatalf("incomplete erase inventory=%#v; want valid empty owner list", got)
	}
	compareOwnersInBlock(t, device, model, 1)
}
