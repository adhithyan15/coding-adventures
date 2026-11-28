package ftlnandrelocation

import (
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"reflect"
	"testing"

	ftlerasevictim "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-victim"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestNewWithEraseBlocksValidationAndLegacySurface(t *testing.T) {
	if _, err := NewWithEraseBlocks(0, 2, 0, 0, 0); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("base geometry must precede later fields: %v", err)
	}
	if _, err := NewWithEraseBlocks(1, 2, 0, 0, 0); !errors.Is(err, ErrInvalidPageSize) {
		t.Fatalf("page size must precede block geometry: %v", err)
	}
	for _, pagesPerBlock := range []int64{0, -1, 3, 5} {
		if _, err := NewWithEraseBlocks(1, 4, 1, pagesPerBlock, 1); !errors.Is(err, ErrInvalidBlockGeometry) {
			t.Errorf("pagesPerBlock=%d error=%v", pagesPerBlock, err)
		}
	}
	if _, err := NewWithEraseBlocks(1, 4, 1, 2, 9_007_199_254_740_992); !errors.Is(err, nandblockerasecuts.ErrInvalidEraseCount) {
		t.Fatalf("unsafe ceiling error: %v", err)
	}
	legacy, err := New(1, 4, 1)
	if err != nil || !reflect.DeepEqual(legacy.ReclamationSnapshot(), ReclamationSnapshot{}) {
		t.Fatalf("legacy profile changed: %+v %v", legacy.ReclamationSnapshot(), err)
	}
	if got, receipt, err := legacy.EraseBlock(0, EraseCut{}); !errors.Is(err, ErrEraseDisabled) || receipt != (EraseReceipt{}) || !reflect.DeepEqual(got.Snapshot(), legacy.Snapshot()) {
		t.Fatalf("legacy erase = %+v %v", receipt, err)
	}
	device, err := NewWithEraseBlocks(1, 4, 1, 2, 0)
	if err != nil {
		t.Fatal(err)
	}
	if got := device.ReclamationSnapshot(); got.PagesPerBlock != 2 || got.MaxCompletedErases != 0 || len(got.Blocks) != 2 {
		t.Fatalf("new profile observation = %+v", got)
	}
}

func TestVictimSelectorChoosesExplicitEligibleBlock(t *testing.T) {
	device, model := prepareBlockDevice(t, 1, 4, 2, 2, 2, 3)
	status := device.Snapshot().Map.PhysicalStatus
	victim, found, err := ftlerasevictim.Select(status, 2)
	if err != nil || !found || victim.Block != 0 || victim.FirstPhysical != 0 || victim.PageCount != 2 || victim.StalePages != 2 {
		t.Fatalf("victim selection = %+v/%t/%v", victim, found, err)
	}
	next, nextModel := compareEraseBlock(t, device, model, victim.Block, EraseCut{Kind: nandblockerasecuts.Complete})
	if next.Snapshot().Map.PhysicalStatus[0] != ftlpagemap.Free || nextModel.status[0] != modelFree {
		t.Fatal("explicitly selected victim was not released")
	}
}

func TestBlockProfileBoundedGraphsCloseUnderWritesAndEraseCuts(t *testing.T) {
	profiles := []struct {
		logical, physical, pagesPerBlock int64
		ceiling                          uint64
	}{
		{logical: 1, physical: 2, pagesPerBlock: 1, ceiling: 1},
		{logical: 1, physical: 4, pagesPerBlock: 2, ceiling: 1},
	}
	writeCuts := []modelCut{{kind: cutBeforeAccept}, {kind: cutAfterPrefix, prefix: 0}, {kind: cutAfterMedia}, {kind: cutComplete}}
	for _, profile := range profiles {
		device, err := NewWithEraseBlocks(profile.logical, profile.physical, 1, profile.pagesPerBlock, profile.ceiling)
		if err != nil {
			t.Fatal(err)
		}
		initial := referenceInitialWithEraseBlocks(profile.logical, profile.physical, 1, profile.pagesPerBlock, profile.ceiling)
		queue := []graphNode{{state: initial, device: device}}
		seen := map[string]bool{referenceKey(initial): true}
		for head := 0; head < len(queue); head++ {
			entry := queue[head]
			checkReferenceInvariants(t, entry.state)
			assertDeviceEquals(t, entry.device, entry.state)
			for logical := int64(-1); logical <= profile.logical; logical++ {
				data, present, readErr := entry.device.Read(logical)
				wantData, wantPresent, wantError := referenceRead(entry.state, logical)
				assertError(t, readErr, wantError)
				if wantError == "" && (present != wantPresent || !bytes.Equal(data, wantData)) {
					t.Fatalf("read (%d/%d) differs from independent model", profile.physical, profile.pagesPerBlock)
				}
				for _, payload := range [][]byte{{0x41}, {0xff}} {
					for _, cut := range writeCuts {
						wantState, wantReceipt, wantError := referenceWrite(entry.state, logical, payload, cut)
						gotDevice, gotReceipt, gotError := entry.device.Write(logical, payload, runtimeCut(cut))
						assertError(t, gotError, wantError)
						if wantError == "" {
							if gotReceipt != (Receipt{Physical: wantReceipt.physical, Published: wantReceipt.published, Acknowledged: wantReceipt.acknowledged}) {
								t.Fatal("write receipt differs from independent model")
							}
						} else if gotReceipt != (Receipt{}) || !reflect.DeepEqual(gotDevice.Snapshot(), entry.device.Snapshot()) {
							t.Fatal("refused write changed a graph state")
						}
						assertDeviceEquals(t, gotDevice, wantState)
						if key := referenceKey(wantState); !seen[key] {
							seen[key] = true
							queue = append(queue, graphNode{state: wantState, device: gotDevice})
						}
					}
				}
			}
			blockCount := int64(len(entry.state.blocks))
			for block := int64(-1); block <= blockCount; block++ {
				cuts := []nandblockerasecuts.Cut{{Kind: nandblockerasecuts.BeforeAccept}, {Kind: nandblockerasecuts.AfterMedia}, {Kind: nandblockerasecuts.Complete}}
				for prefix := 0; prefix < int(profile.pagesPerBlock); prefix++ {
					cuts = append(cuts, nandblockerasecuts.Cut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: prefix})
				}
				for _, cut := range cuts {
					wantState, wantReceipt, wantError := referenceEraseBlock(entry.state, block, cut)
					gotDevice, gotReceipt, gotError := entry.device.EraseBlock(block, cut)
					assertError(t, gotError, wantError)
					if wantError == "" {
						if gotReceipt != wantReceipt {
							t.Fatal("erase receipt differs from independent model")
						}
					} else if gotReceipt != (EraseReceipt{}) || !reflect.DeepEqual(gotDevice.Snapshot(), entry.device.Snapshot()) || !reflect.DeepEqual(gotDevice.ReclamationSnapshot(), entry.device.ReclamationSnapshot()) {
						t.Fatal("refused erase changed a graph state")
					}
					assertDeviceEquals(t, gotDevice, wantState)
					if key := referenceKey(wantState); !seen[key] {
						seen[key] = true
						queue = append(queue, graphNode{state: wantState, device: gotDevice})
					}
				}
			}
		}
		t.Logf("profile logical=%d physical=%d pages_per_block=%d ceiling=%d reached %d stable states", profile.logical, profile.physical, profile.pagesPerBlock, profile.ceiling, len(seen))
	}
}

func TestSeededBlockHistoriesMatchIndependentModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05C2B))
	device, err := NewWithEraseBlocks(1, 6, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 6, 1, 2, 2)
	var writes, erases, reads int
	for step := 0; step < 10_000; step++ {
		if step > 0 && step%113 == 0 {
			device, _ = NewWithEraseBlocks(1, 6, 1, 2, 2)
			model = referenceInitialWithEraseBlocks(1, 6, 1, 2, 2)
		}
		switch random.Intn(3) {
		case 0:
			logical := int64(random.Intn(3) - 1)
			payload := []byte{byte(random.Intn(256))}
			cut := modelCut{kind: random.Intn(4), prefix: random.Intn(2)}
			nextModel, want, wantErr := referenceWrite(model, logical, payload, cut)
			nextDevice, got, gotErr := device.Write(logical, payload, runtimeCut(cut))
			assertError(t, gotErr, wantErr)
			if wantErr == "" {
				writes++
				if got != (Receipt{Physical: want.physical, Published: want.published, Acknowledged: want.acknowledged}) {
					t.Fatalf("seeded write receipt mismatch at %d", step)
				}
			} else if got != (Receipt{}) {
				t.Fatalf("seeded refused write returned receipt at %d", step)
			}
			device, model = nextDevice, nextModel
		case 1:
			block := int64(random.Intn(4) - 1)
			kind := random.Intn(4)
			cut := nandblockerasecuts.Cut{Kind: nandblockerasecuts.CutKind(kind)}
			if kind == int(nandblockerasecuts.AfterPrefix) {
				cut.PrefixPages = random.Intn(2)
			}
			nextModel, want, wantErr := referenceEraseBlock(model, block, cut)
			nextDevice, got, gotErr := device.EraseBlock(block, cut)
			assertError(t, gotErr, wantErr)
			if wantErr == "" {
				erases++
				if got != want {
					t.Fatalf("seeded erase receipt mismatch at %d", step)
				}
			} else if got != (EraseReceipt{}) {
				t.Fatalf("seeded refused erase returned receipt at %d", step)
			}
			device, model = nextDevice, nextModel
		case 2:
			logical := int64(random.Intn(3) - 1)
			wantData, wantPresent, wantErr := referenceRead(model, logical)
			gotData, gotPresent, gotErr := device.Read(logical)
			assertError(t, gotErr, wantErr)
			if wantErr == "" && (gotPresent != wantPresent || !bytes.Equal(gotData, wantData)) {
				t.Fatalf("seeded read mismatch at %d", step)
			}
			reads++
		}
		assertDeviceEquals(t, device, model)
	}
	if writes < 100 || erases < 100 || reads < 100 {
		t.Fatalf("seed did not exercise all operation types: writes=%d erases=%d reads=%d", writes, erases, reads)
	}
	t.Logf("seeded block history completed 10000 commands: writes=%d erases=%d reads=%d", writes, erases, reads)
}

func TestZeroPrefixQuarantinesRawIdenticalBlockAndChangesNextAllocation(t *testing.T) {
	device, model := prepareStaleFreeBlock(t)
	beforeRaw := device.Snapshot()
	before, beforeModel := compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.BeforeAccept})
	zeroPrefix, zeroModel := compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if !reflect.DeepEqual(beforeRaw, zeroPrefix.Snapshot()) {
		t.Fatal("zero-prefix erase changed raw page or map observations")
	}
	if before.ReclamationSnapshot().Blocks[0].IncompleteErase || !zeroPrefix.ReclamationSnapshot().Blocks[0].IncompleteErase {
		t.Fatal("zero-prefix and BeforeAccept must differ only in quarantine metadata")
	}
	before, beforeModel = compareTransition(t, before, beforeModel, 0, []byte{0x42}, modelCut{kind: cutComplete})
	zeroPrefix, zeroModel = compareTransition(t, zeroPrefix, zeroModel, 0, []byte{0x52}, modelCut{kind: cutComplete})
	if before.Snapshot().Map.LogicalToPhysical[0] != 1 || zeroPrefix.Snapshot().Map.LogicalToPhysical[0] != 2 {
		t.Fatalf("allocation did not honor the fence: BeforeAccept=%+v AfterPrefix0=%+v", before.Snapshot().Map, zeroPrefix.Snapshot().Map)
	}
	assertDeviceEquals(t, before, beforeModel)
	assertDeviceEquals(t, zeroPrefix, zeroModel)
	if !reflect.DeepEqual(device.Snapshot(), beforeRaw) {
		t.Fatal("erase branch mutated its earlier device")
	}
}

func TestAllCutsAndPrefixesOnEachEligibleBlockThenReadAndWrite(t *testing.T) {
	base, state := prepareBlockDevice(t, 1, 6, 1, 2, 2, 5)
	cuts := []EraseCut{{Kind: nandblockerasecuts.BeforeAccept}, {Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}, {Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1}, {Kind: nandblockerasecuts.AfterMedia}, {Kind: nandblockerasecuts.Complete}}
	for _, block := range []int64{0, 1} {
		for _, cut := range cuts {
			t.Run(blockCutName(block, cut), func(t *testing.T) {
				afterErase, afterState := compareEraseBlock(t, base, state, block, cut)
				assertRead(t, afterErase, afterState, 0)
				payload := []byte{0x35}
				afterWrite, writtenState := compareTransition(t, afterErase, afterState, 0, payload, modelCut{kind: cutComplete})
				assertRead(t, afterWrite, writtenState, 0)
				assertDeviceEquals(t, afterWrite, writtenState)
			})
		}
	}
}

func TestPartialEraseClearsPrefixMarkerButPreservesMappingAndSuffix(t *testing.T) {
	device, model := prepareBlockDevice(t, 1, 4, 2, 2, 2, 3)
	originalDevice := device
	before := device.Snapshot()
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	snapshot := device.Snapshot()
	if snapshot.Map.PhysicalStatus[0] != ftlpagemap.Stale || snapshot.Map.PhysicalStatus[1] != ftlpagemap.Stale ||
		snapshot.Pages[0].Programmed || !allBytes(snapshot.Pages[0].Bytes, 0xff) || !snapshot.Pages[1].Programmed ||
		!bytes.Equal(snapshot.Pages[1].Bytes, before.Pages[1].Bytes) {
		t.Fatalf("partial erase must clear prefix media while preserving stale map classifications: %+v", snapshot)
	}
	if snapshot.Map.LogicalToPhysical[0] != 2 {
		t.Fatalf("partial erase repointed the live mapping: %+v", snapshot.Map)
	}
	assertRead(t, device, model, 0)
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if device.ReclamationSnapshot().Blocks[0].CompletedErases != 0 || !device.ReclamationSnapshot().Blocks[0].IncompleteErase {
		t.Fatalf("shorter retry changed count/quarantine: %+v", device.ReclamationSnapshot())
	}
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.BeforeAccept})
	if !device.ReclamationSnapshot().Blocks[0].IncompleteErase || device.ReclamationSnapshot().Blocks[0].CompletedErases != 0 {
		t.Fatalf("BeforeAccept cleared earlier quarantine: %+v", device.ReclamationSnapshot())
	}
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterMedia})
	if receipt := device.ReclamationSnapshot().Blocks[0]; receipt.CompletedErases != 1 || receipt.IncompleteErase {
		t.Fatalf("AfterMedia lost completed-but-unacknowledged media result: %+v", receipt)
	}
	if device.Snapshot().Map.PhysicalStatus[0] != ftlpagemap.Free || device.Snapshot().Map.PhysicalStatus[1] != ftlpagemap.Free {
		t.Fatal("AfterMedia did not release the complete stale/free map extent")
	}
	device, model = compareTransition(t, device, model, 0, []byte{0x61, 0x62}, modelCut{kind: cutComplete})
	if device.Snapshot().Map.LogicalToPhysical[0] != 0 {
		t.Fatal("first write after release did not use the lowest released slot")
	}
	assertRead(t, device, model, 0)
	if !reflect.DeepEqual(originalDevice.Snapshot(), before) {
		t.Fatal("later erase transitions mutated the earlier device")
	}
}

func TestPartialEraseDoesNotReleaseQuarantinedExtent(t *testing.T) {
	device, _ := prepareStaleFreeBlock(t)
	device, receipt, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if err != nil {
		t.Fatal(err)
	}
	if receipt.Released || device.Snapshot().Map.PhysicalStatus[0] != ftlpagemap.Stale || !device.ReclamationSnapshot().Blocks[0].IncompleteErase {
		t.Fatal("partial erase released quarantined extent")
	}
}

func TestAfterMediaReleasesExtentWithoutAcknowledgement(t *testing.T) {
	device, _ := prepareStaleFreeBlock(t)
	device, receipt, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterMedia})
	if err != nil {
		t.Fatal(err)
	}
	if !receipt.Released || receipt.Acknowledged || device.Snapshot().Map.PhysicalStatus[0] != ftlpagemap.Free {
		t.Fatal("AfterMedia did not release the unacknowledged completed extent")
	}
}

func TestWriteFenceSkipsEveryIncompleteBlock(t *testing.T) {
	device, _ := prepareStaleFreeBlock(t)
	device, _, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if err != nil {
		t.Fatal(err)
	}
	device, receipt, err := device.Write(0, []byte{0x51}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	if receipt.Physical != 2 || device.Snapshot().Map.LogicalToPhysical[0] != 2 {
		t.Fatal("write allocation entered a quarantined erase block")
	}
}

func TestMultipleIncompleteBlocksFenceAllFreeCapacity(t *testing.T) {
	device, model := prepareBlockDevice(t, 1, 6, 1, 2, 2, 4)
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	device, model = compareTransition(t, device, model, 0, []byte{0x75}, modelCut{kind: cutComplete})
	device, model = compareEraseBlock(t, device, model, 1, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	device, model = compareTransition(t, device, model, 0, []byte{0x76}, modelCut{kind: cutComplete})
	reclamation := device.ReclamationSnapshot()
	if !reclamation.Blocks[0].IncompleteErase || !reclamation.Blocks[1].IncompleteErase || reclamation.Blocks[2].IncompleteErase {
		t.Fatalf("multiple incomplete block flags: %+v", reclamation)
	}
	before := device.Snapshot()
	refused, receipt, err := device.Write(0, []byte{0x01}, Cut{Kind: nandpageprogramcuts.Complete})
	if !errors.Is(err, ftlpagemap.ErrNoFreePage) || receipt != (Receipt{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
		t.Fatalf("all unfenced capacity exhausted: %+v %v", receipt, err)
	}
	if _, receipt, err := device.EraseBlock(2, EraseCut{Kind: nandblockerasecuts.Complete}); !errors.Is(err, ErrIneligibleBlock) || receipt != (EraseReceipt{}) {
		t.Fatalf("block with a live slot must be ineligible: %+v %v", receipt, err)
	}
	assertRead(t, device, model, 0)
}

func TestEraseValidationOrderCeilingAndDetachedValues(t *testing.T) {
	var zero Device
	if got, receipt, err := zero.EraseBlock(-1, EraseCut{Kind: nandblockerasecuts.CutKind(255)}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || receipt != (EraseReceipt{}) || !reflect.DeepEqual(got.Snapshot(), Snapshot{}) {
		t.Fatalf("zero-device erase precedence: %+v %v", receipt, err)
	}
	legacy, _ := New(1, 4, 1)
	if _, _, err := legacy.EraseBlock(-1, EraseCut{}); !errors.Is(err, ErrEraseDisabled) {
		t.Fatalf("legacy profile must precede block index: %v", err)
	}
	device, model := prepareStaleFreeBlock(t)
	for _, block := range []int64{-1, 2} {
		if _, receipt, err := device.EraseBlock(block, EraseCut{Kind: nandblockerasecuts.CutKind(255)}); !errors.Is(err, ErrInvalidBlockIndex) || receipt != (EraseReceipt{}) {
			t.Fatalf("block index precedence: block=%d receipt=%+v err=%v", block, receipt, err)
		}
	}
	if _, receipt, err := device.EraseBlock(1, EraseCut{Kind: nandblockerasecuts.CutKind(255)}); !errors.Is(err, ErrIneligibleBlock) || receipt != (EraseReceipt{}) {
		t.Fatalf("eligibility must precede DB04d cut checks: %+v %v", receipt, err)
	}
	before := device.Snapshot()
	if _, receipt, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.CutKind(255)}); !errors.Is(err, nandblockerasecuts.ErrInvalidCut) || receipt != (EraseReceipt{}) || !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatalf("valid target invalid cut refusal: %+v %v", receipt, err)
	}
	if _, receipt, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 2}); !errors.Is(err, nandblockerasecuts.ErrInvalidCut) || receipt != (EraseReceipt{}) {
		t.Fatalf("invalid prefix refusal: %+v %v", receipt, err)
	}
	_ = model

	zeroCeiling, _ := prepareBlockDevice(t, 1, 4, 1, 2, 0, 3)
	if _, receipt, err := zeroCeiling.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}); !errors.Is(err, nandblockerasecuts.ErrEraseLimit) || receipt != (EraseReceipt{}) {
		t.Fatalf("zero erase ceiling: %+v %v", receipt, err)
	}

	limited, limitedModel := prepareBlockDevice(t, 1, 4, 1, 2, 1, 3)
	limited, limitedModel = compareEraseBlock(t, limited, limitedModel, 0, EraseCut{Kind: nandblockerasecuts.AfterMedia})
	limited, limitedModel = compareTransition(t, limited, limitedModel, 0, []byte{0x11}, modelCut{kind: cutComplete})
	limited, limitedModel = compareTransition(t, limited, limitedModel, 0, []byte{0x12}, modelCut{kind: cutComplete})
	limited, limitedModel = compareTransition(t, limited, limitedModel, 0, []byte{0x13}, modelCut{kind: cutComplete})
	beforeLimit := limited.Snapshot()
	if _, receipt, err := limited.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.BeforeAccept}); !errors.Is(err, nandblockerasecuts.ErrEraseLimit) || receipt != (EraseReceipt{}) || !reflect.DeepEqual(limited.Snapshot(), beforeLimit) {
		t.Fatalf("erase ceiling did not refuse atomically: %+v %v", receipt, err)
	}
	if limited.ReclamationSnapshot().Blocks[0].CompletedErases != 1 {
		t.Fatal("completed media count changed on refused second erase")
	}
}

func TestReclamationSnapshotsAreDetached(t *testing.T) {
	device, model := prepareStaleFreeBlock(t)
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	snapshot := device.ReclamationSnapshot()
	snapshot.Blocks[0].IncompleteErase = false
	snapshot.Blocks[0].CompletedErases = 99
	snapshot.PagesPerBlock = 1
	if !device.ReclamationSnapshot().Blocks[0].IncompleteErase || device.ReclamationSnapshot().Blocks[0].CompletedErases != 0 || device.ReclamationSnapshot().PagesPerBlock != 2 {
		t.Fatal("reclamation snapshot aliases device metadata")
	}
	_ = model
}

func prepareBlockDevice(t *testing.T, logical, physical int64, pageSize int, pagesPerBlock int64, ceiling uint64, completeWrites int) (Device, referenceState) {
	t.Helper()
	device, err := NewWithEraseBlocks(logical, physical, pageSize, pagesPerBlock, ceiling)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(logical, physical, pageSize, pagesPerBlock, ceiling)
	for index := 0; index < completeWrites; index++ {
		payload := make([]byte, pageSize)
		for offset := range payload {
			payload[offset] = byte(0x20 + index + offset)
		}
		device, model = compareTransition(t, device, model, 0, payload, modelCut{kind: cutComplete})
	}
	return device, model
}

func prepareStaleFreeBlock(t *testing.T) (Device, referenceState) {
	t.Helper()
	device, err := NewWithEraseBlocks(1, 4, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 4, 1, 2, 2)
	return compareTransition(t, device, model, 0, []byte{0x41}, modelCut{kind: cutAfterPrefix, prefix: 0})
}

func compareEraseBlock(t *testing.T, device Device, model referenceState, block int64, cut EraseCut) (Device, referenceState) {
	t.Helper()
	wantState, wantReceipt, wantError := referenceEraseBlock(model, block, cut)
	gotDevice, gotReceipt, err := device.EraseBlock(block, cut)
	assertError(t, err, wantError)
	if wantError != "" {
		if gotReceipt != (EraseReceipt{}) || !reflect.DeepEqual(gotDevice.Snapshot(), device.Snapshot()) || !reflect.DeepEqual(gotDevice.ReclamationSnapshot(), device.ReclamationSnapshot()) {
			t.Fatalf("refused erase changed device: %+v %v", gotReceipt, err)
		}
		assertLookupMatchesModel(t, device, model)
		return device, model
	}
	if gotReceipt != wantReceipt {
		t.Fatalf("erase receipt = %+v, want %+v", gotReceipt, wantReceipt)
	}
	assertDeviceEquals(t, gotDevice, wantState)
	return gotDevice, wantState
}

func blockCutName(block int64, cut EraseCut) string {
	return fmt.Sprintf("block-%d-kind-%d-prefix-%d", block, cut.Kind, cut.PrefixPages)
}
