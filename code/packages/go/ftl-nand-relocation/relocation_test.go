package ftlnandrelocation

import (
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"reflect"
	"testing"

	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestEveryProgramCutAndPrefixMatchesIndependentMediaModel(t *testing.T) {
	for _, kind := range []int{cutBeforeAccept, cutAfterMedia, cutComplete} {
		checkSingleWrite(t, 1, 3, 3, 0, []byte{0x10, 0x20, 0x30}, kind, 0)
	}
	for prefix := 0; prefix < 3; prefix++ {
		checkSingleWrite(t, 1, 3, 3, 0, []byte{0x10, 0x20, 0x30}, cutAfterPrefix, prefix)
	}
	for _, kind := range []int{cutBeforeAccept, cutAfterPrefix, cutAfterMedia, cutComplete} {
		prefix := 0
		if kind == cutAfterPrefix {
			prefix = 0
		}
		checkSingleWrite(t, 1, 3, 2, 0, []byte{0xff, 0xff}, kind, prefix)
	}
}

func TestZeroPrefixAndMediaCompleteAllOnesRemainDistinct(t *testing.T) {
	device, err := New(1, 3, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitial(1, 3, 2)
	device, model = assertWrite(t, device, model, 0, []byte{0x10, 0x20}, modelCut{kind: cutComplete})
	device, model = assertWrite(t, device, model, 0, []byte{0xff, 0xff}, modelCut{kind: cutAfterPrefix, prefix: 0})
	zeroPrefix := device.Snapshot()
	if zeroPrefix.Pages[1].Programmed != true || !allBytes(zeroPrefix.Pages[1].Bytes, 0xff) || zeroPrefix.Map.PhysicalStatus[1] != ftlpagemap.Stale {
		t.Fatalf("zero-prefix all-ones must spend the page without publishing: %+v", zeroPrefix)
	}
	if physical, present, err := device.Read(0); err != nil || !present || !bytes.Equal(physical, []byte{0x10, 0x20}) {
		t.Fatalf("zero-prefix changed old live value: %x/%t/%v", physical, present, err)
	}
	device, model = assertWrite(t, device, model, 0, []byte{0xff, 0xff}, modelCut{kind: cutAfterMedia})
	afterMedia := device.Snapshot()
	if afterMedia.Map.LogicalToPhysical[0] != 2 || afterMedia.Map.PhysicalStatus[2] != ftlpagemap.Live || !afterMedia.Pages[2].Programmed || !allBytes(afterMedia.Pages[2].Bytes, 0xff) {
		t.Fatalf("AfterMedia all-ones must publish despite unchanged bytes: %+v", afterMedia)
	}
	if physical, present, err := device.Read(0); err != nil || !present || !bytes.Equal(physical, []byte{0xff, 0xff}) {
		t.Fatalf("AfterMedia read = %x/%t/%v", physical, present, err)
	}
	if device, receipt, err := device.Write(0, []byte{0xff, 0xff}, Cut{Kind: nandpageprogramcuts.Complete}); !errors.Is(err, ftlpagemap.ErrNoFreePage) || receipt != (Receipt{}) || !reflect.DeepEqual(device.Snapshot(), afterMedia) {
		t.Fatalf("full capacity refusal changed device or precedence: %+v, %v", receipt, err)
	}
	_ = model
}

func TestBeforeAcceptFreshAbortAndAfterMediaPublicationAcknowledgement(t *testing.T) {
	device, _ := New(1, 3, 2)
	initial := device.Snapshot()
	device, receipt, err := device.Write(0, []byte{0x41, 0x42}, Cut{Kind: nandpageprogramcuts.BeforeAccept})
	if err != nil || receipt != (Receipt{Physical: 0}) {
		t.Fatalf("BeforeAccept = %+v, %v", receipt, err)
	}
	afterBefore := device.Snapshot()
	if afterBefore.Map.PhysicalStatus[0] != ftlpagemap.Free || afterBefore.Pages[0].Programmed || !allBytes(afterBefore.Pages[0].Bytes, 0xff) {
		t.Fatalf("BeforeAccept should leave destination erased and free: %+v", afterBefore)
	}
	if !reflect.DeepEqual(initial, afterBefore) {
		t.Fatal("BeforeAccept mutated the earlier device")
	}
	device, receipt, err = device.Write(0, []byte{0x41, 0x42}, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil || receipt != (Receipt{Physical: 0, Published: true, Acknowledged: false}) {
		t.Fatalf("AfterMedia publication receipt = %+v, %v", receipt, err)
	}
	if physical, present, err := device.Read(0); err != nil || !present || !bytes.Equal(physical, []byte{0x41, 0x42}) {
		t.Fatalf("AfterMedia read = %x/%t/%v", physical, present, err)
	}
	if !reflect.DeepEqual(afterBefore, initial) {
		t.Fatal("later write mutated the prior device")
	}
}

func TestPartialProgramIsNeverPublished(t *testing.T) {
	device, _ := New(1, 3, 2)
	device, receipt, err := device.Write(0, []byte{0x10, 0x20}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1})
	if err != nil {
		t.Fatal(err)
	}
	if receipt.Published || receipt.Acknowledged || receipt.Physical != 0 {
		t.Fatal("partial destination must remain unpublished")
	}
	snapshot := device.Snapshot()
	if snapshot.Map.LogicalToPhysical[0] != -1 || snapshot.Map.PhysicalStatus[0] != ftlpagemap.Stale || !snapshot.Pages[0].Programmed || !bytes.Equal(snapshot.Pages[0].Bytes, []byte{0x10, 0xff}) {
		t.Fatal("partial destination must remain unpublished")
	}
	if data, present, err := device.Read(0); err != nil || data != nil || present {
		t.Fatal("partial destination must remain unpublished")
	}
}

func TestSpentNANDPageIsNotSelectedAgain(t *testing.T) {
	device, _ := New(1, 2, 2)
	device, _, err := device.Write(0, []byte{0x10, 0x20}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if err != nil {
		t.Fatal(err)
	}
	device, receipt, err := device.Write(0, []byte{0x30, 0x40}, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil || receipt.Physical != 1 {
		t.Fatal("spent NAND page was selected again")
	}
	if snapshot := device.Snapshot(); snapshot.Map.PhysicalStatus[0] != ftlpagemap.Stale || snapshot.Map.PhysicalStatus[1] != ftlpagemap.Live {
		t.Fatal("spent NAND page was selected again")
	}
}

func TestAfterMediaPublicationDoesNotAcknowledge(t *testing.T) {
	device, _ := New(1, 2, 1)
	_, receipt, err := device.Write(0, []byte{0x42}, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil || !receipt.Published || receipt.Acknowledged {
		t.Fatal("AfterMedia publishes without caller acknowledgement")
	}
}

func TestRefusalOrderAndInternalReservationRollback(t *testing.T) {
	if _, err := New(0, 2, 0); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("geometry must precede page-size validation: %v", err)
	}
	if _, err := New(1, 2, 0); !errors.Is(err, nandpageprogramcuts.ErrInvalidPageSize) {
		t.Fatalf("invalid width error = %v", err)
	}
	var zero Device
	if _, _, err := zero.Write(-1, nil, Cut{}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero-device refusal = %v", err)
	}
	if data, present, err := zero.Read(0); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || data != nil || present {
		t.Fatalf("zero-device read = %x/%t/%v", data, present, err)
	}

	device, _ := New(1, 2, 2)
	before := device.Snapshot()
	deviceAfterRefusal, receipt, err := device.Write(0, []byte{0x01}, Cut{Kind: nandpageprogramcuts.CutKind(255)})
	if !errors.Is(err, nandpageprogramcuts.ErrWrongPayloadLength) || receipt != (Receipt{}) || !reflect.DeepEqual(deviceAfterRefusal.Snapshot(), before) {
		t.Fatalf("wrong length precedes invalid cut and rolls back reservation: %+v, %v", receipt, err)
	}
	device, receipt, err = device.Write(0, []byte{0x01, 0x02}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 2})
	if !errors.Is(err, nandpageprogramcuts.ErrInvalidCut) || receipt != (Receipt{}) || !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatalf("invalid cut refusal changed input: %+v, %v", receipt, err)
	}
	device, receipt, err = device.Write(0, []byte{0x01, 0x02}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || receipt.Physical != 0 || !receipt.Published || !receipt.Acknowledged {
		t.Fatalf("rolled-back destination was not available: %+v, %v", receipt, err)
	}
	if _, _, err := device.Write(1, []byte{}, Cut{Kind: nandpageprogramcuts.CutKind(255)}); !errors.Is(err, ftlpagemap.ErrInvalidLogicalPage) {
		t.Fatalf("logical address must precede table capacity: %v", err)
	}
}

func TestCapacityRefusalPrecedesPayloadAndCutValidation(t *testing.T) {
	device, _ := New(1, 2, 1)
	device, _, _ = device.Write(0, []byte{0x10}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	device, _, _ = device.Write(0, []byte{0x20}, Cut{Kind: nandpageprogramcuts.Complete})
	before := device.Snapshot()
	refused, receipt, err := device.Write(0, nil, Cut{Kind: nandpageprogramcuts.CutKind(255)})
	if !errors.Is(err, ftlpagemap.ErrNoFreePage) || receipt != (Receipt{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
		t.Fatalf("capacity should precede payload and cut validation: %+v, %v", receipt, err)
	}
	if _, _, err := device.Write(-1, nil, Cut{Kind: nandpageprogramcuts.CutKind(255)}); !errors.Is(err, ftlpagemap.ErrInvalidLogicalPage) {
		t.Fatalf("address should precede capacity: %v", err)
	}
}

func TestReadsSnapshotsAndPayloadsAreDetached(t *testing.T) {
	device, _ := New(1, 3, 2)
	payload := []byte{0x11, 0x22}
	newDevice, _, err := device.Write(0, payload, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	payload[0] = 0x99
	if data, present, err := newDevice.Read(0); err != nil || !present || !bytes.Equal(data, []byte{0x11, 0x22}) {
		t.Fatalf("payload mutation reached device: %x/%t/%v", data, present, err)
	}
	data, _, _ := newDevice.Read(0)
	data[1] = 0x88
	if again, _, _ := newDevice.Read(0); !bytes.Equal(again, []byte{0x11, 0x22}) {
		t.Fatalf("read output aliases device bytes: %x", again)
	}
	snapshot := newDevice.Snapshot()
	snapshot.Map.LogicalToPhysical[0] = 2
	snapshot.Map.PhysicalStatus[0] = ftlpagemap.Free
	snapshot.Pages[0].Bytes[0] = 0
	if again, _, _ := newDevice.Read(0); !bytes.Equal(again, []byte{0x11, 0x22}) {
		t.Fatalf("snapshot mutation reached device: %x", again)
	}
	if prior := device.Snapshot(); prior.Map.LogicalToPhysical[0] != -1 || prior.Map.PhysicalStatus[0] != ftlpagemap.Free {
		t.Fatalf("successor write mutated earlier device: %+v", prior)
	}
}

func TestReadAbsenceAndAddressValidation(t *testing.T) {
	device, _ := New(1, 2, 1)
	data, present, err := device.Read(0)
	if err != nil || present || data != nil {
		t.Fatalf("unmapped read = %x/%t/%v; want nil, absent, nil error", data, present, err)
	}
	for _, logical := range []int64{-1, 1} {
		if data, present, err := device.Read(logical); !errors.Is(err, ftlpagemap.ErrInvalidLogicalPage) || data != nil || present {
			t.Fatalf("Read(%d) = %x/%t/%v", logical, data, present, err)
		}
	}
}

func TestZeroAndMalformedInternalDeviceValuesAreUnconfigured(t *testing.T) {
	var zero Device
	if !reflect.DeepEqual(zero.Snapshot(), Snapshot{}) {
		t.Fatal("zero device must expose the zero snapshot")
	}
	if _, _, err := zero.Read(0); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero device Read error = %v", err)
	}
	if _, _, err := zero.Write(0, nil, Cut{}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero device Write error = %v", err)
	}
	invalidWidth := Device{pageSize: maxPageSize + 1}
	if !reflect.DeepEqual(invalidWidth.Snapshot(), Snapshot{}) {
		t.Fatal("invalid page width must expose the zero snapshot")
	}

	table, _ := ftlpagemap.New(1, 2)
	validPages := []nandpageprogramcuts.PageState{{Bytes: []byte{0xff}}, {Bytes: []byte{0xff}}}
	malformed := []Device{
		{table: table, pageSize: 1},
		{table: table, pages: []nandpageprogramcuts.PageState{{Bytes: nil}, {Bytes: []byte{0xff}}}, pageSize: 1},
		{table: table, pages: []nandpageprogramcuts.PageState{{Bytes: []byte{0xff}, Programmed: true}, {Bytes: []byte{0xff}}}, pageSize: 1},
	}
	reservedTable, _, _ := table.Begin(0)
	malformed = append(malformed, Device{table: reservedTable, pages: validPages, pageSize: 1})
	readyTable, _, _ := table.Begin(0)
	readyTable, _ = readyTable.Finish(ftlpagemap.MediaReady)
	malformed = append(malformed, Device{table: readyTable, pages: validPages, pageSize: 1})
	spentTable, _, _ := table.Begin(0)
	spentTable, _ = spentTable.Finish(ftlpagemap.AbortSpent)
	malformed = append(malformed, Device{table: spentTable, pages: validPages, pageSize: 1})
	for index, device := range malformed {
		if !reflect.DeepEqual(device.Snapshot(), Snapshot{}) {
			t.Errorf("malformed internal device %d exposed a snapshot", index)
		}
		if _, _, err := device.Read(0); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
			t.Errorf("malformed internal device %d Read error = %v", index, err)
		}
	}
}

func TestSmallDeviceStateGraphClosesUnderAllCuts(t *testing.T) {
	for _, geometry := range []struct{ logical, physical int64 }{{1, 2}, {2, 3}} {
		device, err := New(geometry.logical, geometry.physical, 1)
		if err != nil {
			t.Fatal(err)
		}
		initial := referenceInitial(geometry.logical, geometry.physical, 1)
		queue := []graphNode{{state: initial, device: device}}
		seen := map[string]bool{referenceKey(initial): true}
		for head := 0; head < len(queue); head++ {
			entry := queue[head]
			checkReferenceInvariants(t, entry.state)
			assertDeviceEquals(t, entry.device, entry.state)
			for logical := int64(-1); logical <= geometry.logical; logical++ {
				assertRead(t, entry.device, entry.state, logical)
				for _, payload := range [][]byte{{0x10}, {0xff}} {
					for _, cut := range []modelCut{{kind: cutBeforeAccept}, {kind: cutAfterPrefix, prefix: 0}, {kind: cutAfterMedia}, {kind: cutComplete}} {
						nextState, _, wantErr := referenceWrite(entry.state, logical, payload, cut)
						nextDevice, _, err := entry.device.Write(logical, payload, runtimeCut(cut))
						assertError(t, err, wantErr)
						if wantErr != "" {
							if !reflect.DeepEqual(nextDevice.Snapshot(), entry.device.Snapshot()) {
								t.Fatal("refused write changed device state")
							}
							continue
						}
						if key := referenceKey(nextState); !seen[key] {
							seen[key] = true
							queue = append(queue, graphNode{state: nextState, device: nextDevice})
						}
						assertDeviceEquals(t, nextDevice, nextState)
					}
				}
			}
		}
		t.Logf("geometry %d/%d reached %d stable states", geometry.logical, geometry.physical, len(seen))
	}
}

func TestTenThousandSeededWritesAndReadsMatchModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05B))
	model := referenceInitial(4, 10, 3)
	device, _ := New(4, 10, 3)
	for step := 0; step < 10_000; step++ {
		if step > 0 && step%31 == 0 {
			model = referenceInitial(4, 10, 3)
			device, _ = New(4, 10, 3)
		}
		logical := int64(random.Intn(6) - 1)
		payload := []byte{byte(random.Intn(256)), byte(random.Intn(256)), byte(random.Intn(256))}
		kind := modelCut{kind: random.Intn(5), prefix: random.Intn(5) - 1}
		device, model = compareTransition(t, device, model, logical, payload, kind)
		assertDeviceEquals(t, device, model)
		assertRead(t, device, model, int64(random.Intn(6)-1))
	}
}

func checkSingleWrite(t *testing.T, logicalCount, physicalCount int64, width int, logical int64, payload []byte, cutKind int, prefix int) {
	t.Helper()
	device, _ := New(logicalCount, physicalCount, width)
	model := referenceInitial(logicalCount, physicalCount, width)
	_, _ = assertWrite(t, device, model, logical, payload, modelCut{kind: cutKind, prefix: prefix})
}

func assertWrite(t *testing.T, device Device, model referenceState, logical int64, payload []byte, cut modelCut) (Device, referenceState) {
	t.Helper()
	nextDevice, nextModel := compareTransition(t, device, model, logical, payload, cut)
	return nextDevice, nextModel
}

func compareTransition(t *testing.T, device Device, model referenceState, logical int64, payload []byte, cut modelCut) (Device, referenceState) {
	t.Helper()
	wantState, wantReceipt, wantError := referenceWrite(model, logical, payload, cut)
	gotDevice, gotReceipt, err := device.Write(logical, payload, runtimeCut(cut))
	assertError(t, err, wantError)
	if wantError != "" {
		if gotReceipt != (Receipt{}) || !reflect.DeepEqual(gotDevice.Snapshot(), device.Snapshot()) {
			t.Fatalf("refused operation was not atomic: %+v, %v", gotReceipt, err)
		}
		assertLookupMatchesModel(t, device, model)
		return device, model
	}
	if gotReceipt != (Receipt{Physical: wantReceipt.physical, Published: wantReceipt.published, Acknowledged: wantReceipt.acknowledged}) {
		t.Fatalf("receipt = %+v, want %+v", gotReceipt, wantReceipt)
	}
	assertDeviceEquals(t, gotDevice, wantState)
	return gotDevice, wantState
}

func assertRead(t *testing.T, device Device, state referenceState, logical int64) {
	t.Helper()
	want, present, wantErr := referenceRead(state, logical)
	got, gotPresent, err := device.Read(logical)
	assertError(t, err, wantErr)
	if present != gotPresent || !bytes.Equal(want, got) {
		t.Fatalf("Read(%d) = %x/%t, want %x/%t", logical, got, gotPresent, want, present)
	}
}

func assertDeviceEquals(t *testing.T, device Device, state referenceState) {
	t.Helper()
	snapshot := device.Snapshot()
	if !reflect.DeepEqual(snapshot.Map.LogicalToPhysical, state.mapping) || len(snapshot.Pages) != len(state.pages) {
		t.Fatalf("mapping/page count mismatch: %+v", snapshot)
	}
	assertLookupMatchesModel(t, device, state)
	for index := range state.pages {
		if !bytes.Equal(snapshot.Pages[index].Bytes, state.pages[index]) || snapshot.Pages[index].Programmed != state.programmed[index] || snapshot.Map.PhysicalStatus[index] != runtimeStatus(state.status[index]) {
			t.Fatalf("physical page %d mismatch: %+v", index, snapshot)
		}
	}
	wantReclamation := ReclamationSnapshot{}
	if state.pagesPerBlock > 0 {
		wantReclamation = ReclamationSnapshot{
			PagesPerBlock:      state.pagesPerBlock,
			MaxCompletedErases: state.maxCompletedErases,
			Blocks:             append([]BlockSnapshot(nil), state.blocks...),
		}
	}
	if !reflect.DeepEqual(device.ReclamationSnapshot(), wantReclamation) {
		t.Fatalf("reclamation snapshot = %+v, want %+v", device.ReclamationSnapshot(), wantReclamation)
	}
	checkReferenceInvariants(t, state)
}

func assertLookupMatchesModel(t *testing.T, device Device, state referenceState) {
	t.Helper()
	for logical, wantPhysical := range state.mapping {
		wantPresent := wantPhysical >= 0
		wantAddress := int64(0)
		if wantPresent {
			wantAddress = wantPhysical
		}
		gotAddress, gotPresent, err := device.Lookup(int64(logical))
		if err != nil || gotAddress != wantAddress || gotPresent != wantPresent {
			t.Fatalf("Lookup(%d) = %d/%t/%v, want %d/%t", logical, gotAddress, gotPresent, err, wantAddress, wantPresent)
		}
	}
}

func assertError(t *testing.T, actual error, expected string) {
	t.Helper()
	if expected == "" {
		if actual != nil {
			t.Fatalf("unexpected error %v", actual)
		}
		return
	}
	if got := errorCode(actual); got != expected {
		t.Fatalf("error code = %q, want %q (%v)", got, expected, actual)
	}
}

func errorCode(err error) string {
	switch {
	case err == nil:
		return ""
	case errors.Is(err, ftlpagemap.ErrInvalidGeometry):
		return "invalid_geometry"
	case errors.Is(err, ftlpagemap.ErrInvalidLogicalPage):
		return "invalid_logical_page"
	case errors.Is(err, ftlpagemap.ErrInvalidPhysicalPage):
		return "invalid_physical_page"
	case errors.Is(err, ftlpagemap.ErrPhysicalPageUnavailable):
		return "physical_page_unavailable"
	case errors.Is(err, ftlpagemap.ErrInvalidFence):
		return "invalid_fence"
	case errors.Is(err, ftlpagemap.ErrReservationPending):
		return "reservation_pending"
	case errors.Is(err, ftlpagemap.ErrNoFreePage):
		return "no_free_page"
	case errors.Is(err, ftlevacuationplan.ErrInvalidBlockIndex):
		return "invalid_evacuation_block_index"
	case errors.Is(err, ftlevacuationplan.ErrInvalidBlockMask):
		return "invalid_evacuation_block_mask"
	case errors.Is(err, ftlevacuationplan.ErrExcludedBlock):
		return "excluded_block"
	case errors.Is(err, ftlevacuationplan.ErrNoStalePage):
		return "no_stale_page"
	case errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity):
		return "insufficient_capacity"
	case errors.Is(err, ErrInvalidBlockGeometry):
		return "invalid_block_geometry"
	case errors.Is(err, ErrEraseDisabled):
		return "erase_disabled"
	case errors.Is(err, ErrInvalidBlockIndex):
		return "invalid_block_index"
	case errors.Is(err, ErrIneligibleBlock):
		return "ineligible_block"
	case errors.Is(err, ErrEvacuationBlockIncomplete):
		return "evacuation_block_incomplete"
	case errors.Is(err, ErrInvalidCopySource):
		return "invalid_copy_source"
	case errors.Is(err, ErrCopySourceChanged):
		return "copy_source_changed"
	case errors.Is(err, ErrInvalidCopyDestination):
		return "invalid_copy_destination"
	case errors.Is(err, ErrCopyDestinationUnavailable):
		return "copy_destination_unavailable"
	case errors.Is(err, ErrEvacuationActive):
		return "evacuation_active"
	case errors.Is(err, ErrNoActiveEvacuation):
		return "no_active_evacuation"
	case errors.Is(err, ErrEvacuationNoLivePages):
		return "evacuation_no_live_pages"
	case errors.Is(err, ErrEvacuationStillLive):
		return "evacuation_still_live"
	case errors.Is(err, ftlpagemap.ErrInvalidExtent):
		return "invalid_extent"
	case errors.Is(err, ftlpagemap.ErrIneligibleExtent):
		return "ineligible_extent"
	case errors.Is(err, nandblockerasecuts.ErrInvalidEraseCount):
		return "invalid_erase_count"
	case errors.Is(err, nandblockerasecuts.ErrInvalidCut):
		return "invalid_erase_cut"
	case errors.Is(err, nandblockerasecuts.ErrEraseLimit):
		return "erase_limit"
	case errors.Is(err, nandblockerasecuts.ErrInvalidBlock):
		return "invalid_erase_block"
	case errors.Is(err, nandpageprogramcuts.ErrInvalidPageSize):
		return "invalid_page_size"
	case errors.Is(err, nandpageprogramcuts.ErrWrongPayloadLength):
		return "wrong_payload_length"
	case errors.Is(err, nandpageprogramcuts.ErrInvalidCut):
		return "invalid_cut"
	case errors.Is(err, nandpageprogramcuts.ErrInvalidPageState):
		return "invalid_page_state"
	case errors.Is(err, nandpageprogramcuts.ErrAlreadyProgrammed):
		return "already_programmed"
	default:
		return "unknown"
	}
}

type modelCut struct {
	kind   int
	prefix int
}

const (
	cutBeforeAccept = iota
	cutAfterPrefix
	cutAfterMedia
	cutComplete
)

type modelSlot uint8

const (
	modelFree modelSlot = iota
	modelLive
	modelStale
)

type referenceState struct {
	logicalCount       int64
	physicalCount      int64
	pageSize           int
	mapping            []int64
	status             []modelSlot
	pages              [][]byte
	programmed         []bool
	pagesPerBlock      int64
	maxCompletedErases uint64
	blocks             []BlockSnapshot
	evacuationActive   bool
	evacuationBlock    int64
}

type modelReceipt struct {
	physical     int64
	published    bool
	acknowledged bool
}

type graphNode struct {
	state  referenceState
	device Device
}

func referenceInitial(logicalCount, physicalCount int64, width int) referenceState {
	state := referenceState{
		logicalCount:  logicalCount,
		physicalCount: physicalCount,
		pageSize:      width,
		mapping:       make([]int64, int(logicalCount)),
		status:        make([]modelSlot, int(physicalCount)),
		pages:         make([][]byte, int(physicalCount)),
		programmed:    make([]bool, int(physicalCount)),
	}
	for index := range state.mapping {
		state.mapping[index] = -1
	}
	for index := range state.pages {
		state.pages[index] = referenceErased(width)
	}
	return state
}

func referenceInitialWithEraseBlocks(logicalCount, physicalCount int64, width int, pagesPerBlock int64, ceiling uint64) referenceState {
	state := referenceInitial(logicalCount, physicalCount, width)
	state.pagesPerBlock = pagesPerBlock
	state.maxCompletedErases = ceiling
	state.blocks = make([]BlockSnapshot, int(physicalCount/pagesPerBlock))
	return state
}

func referenceWrite(before referenceState, logical int64, payload []byte, cut modelCut) (referenceState, modelReceipt, string) {
	return referenceWriteExcluding(before, logical, payload, cut, nil)
}

func referenceWriteExcluding(before referenceState, logical int64, payload []byte, cut modelCut, excludedPages []bool) (referenceState, modelReceipt, string) {
	if before.logicalCount < 1 || before.logicalCount > 64 || before.physicalCount < 2 || before.physicalCount > 128 || before.physicalCount <= before.logicalCount || before.pageSize < 1 || before.pageSize > 65_536 {
		return before, modelReceipt{}, "invalid_geometry"
	}
	if logical < 0 || logical >= before.logicalCount {
		return before, modelReceipt{}, "invalid_logical_page"
	}
	if len(excludedPages) != 0 && len(excludedPages) != int(before.physicalCount) {
		return before, modelReceipt{}, "invalid_fence"
	}
	physical := -1
	for index, status := range before.status {
		eligible := status == modelFree
		if eligible && len(excludedPages) != 0 && excludedPages[index] {
			eligible = false
		}
		if eligible && before.pagesPerBlock > 0 && before.blocks[index/int(before.pagesPerBlock)].IncompleteErase {
			eligible = false
		}
		if eligible && before.evacuationActive && int64(index)/before.pagesPerBlock == before.evacuationBlock {
			eligible = false
		}
		if eligible {
			physical = index
			break
		}
	}
	if physical < 0 {
		return before, modelReceipt{}, "no_free_page"
	}
	if len(payload) != before.pageSize {
		return before, modelReceipt{}, "wrong_payload_length"
	}
	if !validModelCut(cut, before.pageSize) {
		return before, modelReceipt{}, "invalid_cut"
	}
	result := cloneReference(before)
	receipt := modelReceipt{physical: int64(physical)}
	switch cut.kind {
	case cutBeforeAccept:
		result.status[physical] = modelFree
	case cutAfterPrefix:
		copy(result.pages[physical][:cut.prefix], payload[:cut.prefix])
		result.programmed[physical] = true
		result.status[physical] = modelStale
	case cutAfterMedia, cutComplete:
		copy(result.pages[physical], payload)
		result.programmed[physical] = true
		old := result.mapping[int(logical)]
		if old >= 0 {
			result.status[int(old)] = modelStale
		}
		result.mapping[int(logical)] = int64(physical)
		result.status[physical] = modelLive
		receipt.published = true
		receipt.acknowledged = cut.kind == cutComplete
	}
	return result, receipt, ""
}

func referenceEraseBlock(before referenceState, blockIndex int64, cut nandblockerasecuts.Cut) (referenceState, EraseReceipt, string) {
	return referenceEraseBlockInternal(before, blockIndex, cut, false)
}

func referenceEraseBlockInternal(before referenceState, blockIndex int64, cut nandblockerasecuts.Cut, allowActive bool) (referenceState, EraseReceipt, string) {
	if before.logicalCount < 1 || before.logicalCount > 64 || before.physicalCount < 2 || before.physicalCount > 128 || before.physicalCount <= before.logicalCount || before.pageSize < 1 || before.pageSize > maxPageSize {
		return before, EraseReceipt{}, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return before, EraseReceipt{}, "erase_disabled"
	}
	blockCount := int64(len(before.blocks))
	if blockIndex < 0 || blockIndex >= blockCount {
		return before, EraseReceipt{}, "invalid_block_index"
	}
	if before.evacuationActive && blockIndex == before.evacuationBlock && !allowActive {
		return before, EraseReceipt{}, "evacuation_active"
	}
	first := blockIndex * before.pagesPerBlock
	stale := false
	for offset := int64(0); offset < before.pagesPerBlock; offset++ {
		switch before.status[int(first+offset)] {
		case modelStale:
			stale = true
		case modelFree:
		default:
			return before, EraseReceipt{}, "ineligible_block"
		}
	}
	if !stale {
		return before, EraseReceipt{}, "ineligible_block"
	}
	if before.maxCompletedErases > maxSafeEraseCount || before.blocks[int(blockIndex)].CompletedErases > before.maxCompletedErases {
		return before, EraseReceipt{}, "invalid_erase_count"
	}
	for _, page := range before.pages[int(first):int(first+before.pagesPerBlock)] {
		if len(page) != before.pageSize {
			return before, EraseReceipt{}, "invalid_erase_block"
		}
	}
	block := before.blocks[int(blockIndex)]
	if !referenceValidEraseCut(cut, before.pagesPerBlock) {
		return before, EraseReceipt{}, "invalid_erase_cut"
	}
	if block.CompletedErases >= before.maxCompletedErases {
		return before, EraseReceipt{}, "erase_limit"
	}
	result := cloneReference(before)
	receipt := EraseReceipt{Block: blockIndex}
	switch cut.Kind {
	case nandblockerasecuts.BeforeAccept:
	case nandblockerasecuts.AfterPrefix:
		for offset := 0; offset < cut.PrefixPages; offset++ {
			pageIndex := int(first) + offset
			for byteIndex := range result.pages[pageIndex] {
				result.pages[pageIndex][byteIndex] = 0xff
			}
			result.programmed[pageIndex] = false
		}
		result.blocks[int(blockIndex)].IncompleteErase = true
	case nandblockerasecuts.AfterMedia, nandblockerasecuts.Complete:
		for offset := 0; offset < int(before.pagesPerBlock); offset++ {
			pageIndex := int(first) + offset
			for byteIndex := range result.pages[pageIndex] {
				result.pages[pageIndex][byteIndex] = 0xff
			}
			result.programmed[pageIndex] = false
			result.status[pageIndex] = modelFree
		}
		result.blocks[int(blockIndex)].CompletedErases++
		result.blocks[int(blockIndex)].IncompleteErase = false
		receipt.Released = true
		receipt.Acknowledged = cut.Kind == nandblockerasecuts.Complete
	}
	return result, receipt, ""
}

func referenceValidEraseCut(cut nandblockerasecuts.Cut, pageCount int64) bool {
	switch cut.Kind {
	case nandblockerasecuts.BeforeAccept, nandblockerasecuts.AfterMedia, nandblockerasecuts.Complete:
		return cut.PrefixPages == 0
	case nandblockerasecuts.AfterPrefix:
		return cut.PrefixPages >= 0 && int64(cut.PrefixPages) < pageCount
	default:
		return false
	}
}

func referenceRead(state referenceState, logical int64) ([]byte, bool, string) {
	if logical < 0 || logical >= state.logicalCount {
		return nil, false, "invalid_logical_page"
	}
	physical := state.mapping[int(logical)]
	if physical < 0 {
		return nil, false, ""
	}
	return append([]byte(nil), state.pages[int(physical)]...), true, ""
}

func validModelCut(cut modelCut, pageSize int) bool {
	switch cut.kind {
	case cutBeforeAccept, cutAfterMedia, cutComplete:
		return cut.prefix == 0
	case cutAfterPrefix:
		return cut.prefix >= 0 && cut.prefix < pageSize
	default:
		return false
	}
}

func runtimeCut(cut modelCut) Cut {
	return Cut{Kind: nandpageprogramcuts.CutKind(cut.kind), PrefixBytes: cut.prefix}
}

func cloneReference(before referenceState) referenceState {
	result := referenceState{
		logicalCount:       before.logicalCount,
		physicalCount:      before.physicalCount,
		pageSize:           before.pageSize,
		mapping:            append([]int64(nil), before.mapping...),
		status:             append([]modelSlot(nil), before.status...),
		pages:              make([][]byte, len(before.pages)),
		programmed:         append([]bool(nil), before.programmed...),
		pagesPerBlock:      before.pagesPerBlock,
		maxCompletedErases: before.maxCompletedErases,
		blocks:             append([]BlockSnapshot(nil), before.blocks...),
		evacuationActive:   before.evacuationActive,
		evacuationBlock:    before.evacuationBlock,
	}
	for index := range before.pages {
		result.pages[index] = append([]byte(nil), before.pages[index]...)
	}
	return result
}

func checkReferenceInvariants(t *testing.T, state referenceState) {
	t.Helper()
	if len(state.mapping) != int(state.logicalCount) || len(state.pages) != int(state.physicalCount) || len(state.status) != int(state.physicalCount) || len(state.programmed) != int(state.physicalCount) {
		t.Fatal("reference geometry does not match media dimensions")
	}
	owners := make([]int, len(state.pages))
	for logical, physical := range state.mapping {
		if physical < 0 {
			continue
		}
		if physical >= int64(len(state.pages)) || state.status[int(physical)] != modelLive || !state.programmed[int(physical)] {
			t.Fatalf("reference logical %d points to non-live or unprogrammed page %d", logical, physical)
		}
		owners[int(physical)]++
	}
	for index, status := range state.status {
		if status == modelLive && owners[index] != 1 {
			t.Fatalf("reference live page %d has %d owners", index, owners[index])
		}
		if status != modelLive && owners[index] != 0 {
			t.Fatalf("reference non-live page %d has %d owners", index, owners[index])
		}
		if status == modelFree && (state.programmed[index] || !allBytes(state.pages[index], 0xff)) {
			t.Fatalf("reference free page %d is not freshly erased", index)
		}
		if status == modelStale && !state.programmed[index] {
			blockIndex := -1
			if state.pagesPerBlock > 0 {
				blockIndex = index / int(state.pagesPerBlock)
			}
			if blockIndex < 0 || !state.blocks[blockIndex].IncompleteErase || !allBytes(state.pages[index], 0xff) {
				t.Fatalf("reference stale page %d lost its program marker outside a cleared incomplete block", index)
			}
		}
	}
	if state.pagesPerBlock > 0 {
		if state.physicalCount%state.pagesPerBlock != 0 || len(state.blocks) != int(state.physicalCount/state.pagesPerBlock) {
			t.Fatal("reference erase-block geometry is inconsistent")
		}
		for blockIndex, block := range state.blocks {
			if block.CompletedErases > state.maxCompletedErases {
				t.Fatalf("reference block %d exceeds erase ceiling", blockIndex)
			}
			if block.IncompleteErase {
				first := blockIndex * int(state.pagesPerBlock)
				for offset := 0; offset < int(state.pagesPerBlock); offset++ {
					if state.status[first+offset] == modelLive {
						t.Fatalf("incomplete block %d still contains live page", blockIndex)
					}
				}
			}
		}
	} else if len(state.blocks) != 0 || state.maxCompletedErases != 0 {
		t.Fatal("legacy reference device has block metadata")
	}
}

func referenceKey(state referenceState) string {
	return fmt.Sprintf("%v/%v/%v/%v/%d/%d/%v", state.mapping, state.status, state.pages, state.programmed, state.pagesPerBlock, state.maxCompletedErases, state.blocks)
}

func runtimeStatus(status modelSlot) ftlpagemap.PhysicalStatus {
	switch status {
	case modelFree:
		return ftlpagemap.Free
	case modelLive:
		return ftlpagemap.Live
	case modelStale:
		return ftlpagemap.Stale
	default:
		return ftlpagemap.PhysicalStatus(255)
	}
}

func referenceErased(width int) []byte {
	page := make([]byte, width)
	for index := range page {
		page[index] = 0xff
	}
	return page
}

func allBytes(value []byte, expected byte) bool {
	for _, current := range value {
		if current != expected {
			return false
		}
	}
	return true
}
