package ftlnandrelocation

import (
	"bytes"
	"errors"
	"math"
	"math/rand"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func referenceProgramUnowned(before referenceState, block, page int64, payload []byte, cut modelCut) (referenceState, UnownedProgramReceipt, string) {
	if before.logicalCount < 1 || before.logicalCount > 64 || before.physicalCount < 2 || before.physicalCount > 128 || before.physicalCount <= before.logicalCount || before.pageSize < 1 || before.pageSize > maxPageSize {
		return before, UnownedProgramReceipt{}, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return before, UnownedProgramReceipt{}, "erase_disabled"
	}
	blockCount := int64(len(before.blocks))
	if block < 0 || block >= blockCount {
		return before, UnownedProgramReceipt{}, "invalid_block_index"
	}
	if page < 0 || page >= before.pagesPerBlock {
		return before, UnownedProgramReceipt{}, "invalid_physical_page"
	}
	physical := block*before.pagesPerBlock + page
	if before.blocks[int(block)].IncompleteErase {
		return before, UnownedProgramReceipt{}, "evacuation_block_incomplete"
	}
	if before.evacuationActive && block == before.evacuationBlock {
		return before, UnownedProgramReceipt{}, "evacuation_active"
	}
	if before.status[int(physical)] != modelFree {
		return before, UnownedProgramReceipt{}, "physical_page_unavailable"
	}
	if len(payload) != before.pageSize {
		return before, UnownedProgramReceipt{}, "wrong_payload_length"
	}
	if !validModelCut(cut, before.pageSize) {
		return before, UnownedProgramReceipt{}, "invalid_cut"
	}
	result := cloneReference(before)
	receipt := UnownedProgramReceipt{Physical: physical}
	switch cut.kind {
	case cutBeforeAccept:
	case cutAfterPrefix:
		copy(result.pages[int(physical)][:cut.prefix], payload[:cut.prefix])
		result.programmed[int(physical)] = true
		result.status[int(physical)] = modelStale
		receipt.Programmed = true
	case cutAfterMedia, cutComplete:
		copy(result.pages[int(physical)], payload)
		result.programmed[int(physical)] = true
		result.status[int(physical)] = modelStale
		receipt.Programmed = true
		receipt.Acknowledged = cut.kind == cutComplete
	}
	return result, receipt, ""
}

func compareProgramUnowned(t *testing.T, device Device, model referenceState, block, page int64, payload []byte, cut Cut) (Device, referenceState) {
	t.Helper()
	payloadBefore := append([]byte(nil), payload...)
	want, wantReceipt, wantError := referenceProgramUnowned(model, block, page, payload, runtimeModelCut(cut))
	before := device.Snapshot()
	beforeBlocks := device.ReclamationSnapshot()
	got, receipt, err := device.ProgramUnownedPage(block, page, payload, cut)
	assertError(t, err, wantError)
	if !bytes.Equal(payload, payloadBefore) {
		t.Fatal("ProgramUnownedPage changed caller payload")
	}
	if wantError != "" {
		if receipt != (UnownedProgramReceipt{}) || !reflect.DeepEqual(got.Snapshot(), before) || !reflect.DeepEqual(got.ReclamationSnapshot(), beforeBlocks) {
			t.Fatalf("refused unowned program changed state: receipt=%+v err=%v", receipt, err)
		}
		return device, model
	}
	if receipt != wantReceipt {
		t.Fatalf("unowned program receipt=%+v want=%+v", receipt, wantReceipt)
	}
	assertDeviceEquals(t, got, want)
	if !reflect.DeepEqual(device.Snapshot(), before) || !reflect.DeepEqual(device.ReclamationSnapshot(), beforeBlocks) {
		t.Fatal("unowned program changed predecessor")
	}
	return got, want
}

func TestProgramUnownedPageEveryCutAndPrefixMatchesIndependentModel(t *testing.T) {
	cuts := []Cut{
		{Kind: nandpageprogramcuts.BeforeAccept},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1},
		{Kind: nandpageprogramcuts.AfterMedia},
		{Kind: nandpageprogramcuts.Complete},
	}
	for _, payload := range [][]byte{{0x23, 0x5a}, {0xff, 0xff}} {
		for _, cut := range cuts {
			device, err := NewWithEraseBlocks(1, 6, 2, 2, 3)
			if err != nil {
				t.Fatal(err)
			}
			model := referenceInitialWithEraseBlocks(1, 6, 2, 2, 3)
			device, model = compareProgramUnowned(t, device, model, 1, 0, payload, cut)
			physical := int64(2)
			snapshot := device.Snapshot()
			if cut.Kind == nandpageprogramcuts.BeforeAccept {
				if snapshot.Map.PhysicalStatus[physical] != ftlpagemap.Free || snapshot.Pages[physical].Programmed || !allBytes(snapshot.Pages[physical].Bytes, 0xff) {
					t.Fatalf("BeforeAccept consumed page: %+v", snapshot)
				}
			} else if snapshot.Map.PhysicalStatus[physical] != ftlpagemap.Stale || !snapshot.Pages[physical].Programmed {
				t.Fatalf("consumed cut did not retain unowned stale page: %+v", snapshot)
			}
		}
	}
}

func TestProgramUnownedPagePreservesOwnersViewsAndAcknowledgement(t *testing.T) {
	device, _ := NewWithEraseBlocks(1, 6, 2, 2, 3)
	device, _, err := device.Write(0, []byte{0x10, 0x20}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	observation, present, err := device.ObserveOwner(0)
	if err != nil || !present {
		t.Fatalf("ObserveOwner=%+v/%t/%v", observation.Owner, present, err)
	}
	retained := device.Snapshot()
	input := []byte{0xff, 0xff}
	device, receipt, err := device.ProgramUnownedPage(1, 1, input, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil || receipt != (UnownedProgramReceipt{Physical: 3, Programmed: true}) {
		t.Fatalf("AfterMedia receipt=%+v err=%v", receipt, err)
	}
	if !device.MatchesOwner(observation) {
		t.Fatal("unowned program changed a logical owner identity")
	}
	if !reflect.DeepEqual(device.Snapshot().Map.LogicalToPhysical, retained.Map.LogicalToPhysical) || !bytes.Equal(device.Snapshot().Pages[0].Bytes, []byte{0x10, 0x20}) {
		t.Fatal("unowned program changed logical mapping or other media")
	}
	if got := device.Snapshot().Pages[3]; !got.Programmed || !bytes.Equal(got.Bytes, input) || device.Snapshot().Map.PhysicalStatus[3] != ftlpagemap.Stale {
		t.Fatalf("AfterMedia did not preserve unowned page bytes/status: %+v", got)
	}
	if !bytes.Equal(retained.Pages[3].Bytes, []byte{0xff, 0xff}) || retained.Pages[3].Programmed {
		t.Fatal("unowned program changed the retained snapshot")
	}
	input[0] = 0
	view := device.Snapshot()
	view.Pages[3].Bytes[1] = 0
	if !bytes.Equal(device.Snapshot().Pages[3].Bytes, []byte{0xff, 0xff}) {
		t.Fatal("input or detached snapshot aliased device bytes")
	}
	device, receipt, err = device.ProgramUnownedPage(1, 0, []byte{0xff, 0xff}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || receipt != (UnownedProgramReceipt{Physical: 2, Programmed: true, Acknowledged: true}) {
		t.Fatalf("all-ones complete receipt=%+v err=%v", receipt, err)
	}
	if !device.MatchesOwner(observation) {
		t.Fatal("complete unowned program changed a logical owner identity")
	}
}

func TestProgramUnownedPageUsesConfiguredBlockGrouping(t *testing.T) {
	twoPageBlocks, err := NewWithEraseBlocks(1, 6, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	threePageBlocks, err := NewWithEraseBlocks(1, 6, 1, 3, 2)
	if err != nil {
		t.Fatal(err)
	}
	twoPageBlocks, first, err := twoPageBlocks.ProgramUnownedPage(1, 0, []byte{0x51}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || first.Physical != 2 {
		t.Fatalf("2-page grouping receipt=%+v error=%v", first, err)
	}
	threePageBlocks, second, err := threePageBlocks.ProgramUnownedPage(1, 0, []byte{0x51}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || second.Physical != 3 {
		t.Fatalf("3-page grouping receipt=%+v error=%v", second, err)
	}
	if twoPageBlocks.Snapshot().Map.PhysicalStatus[2] != ftlpagemap.Stale || threePageBlocks.Snapshot().Map.PhysicalStatus[3] != ftlpagemap.Stale {
		t.Fatal("block grouping did not select the exact absolute page")
	}
}

func TestProgramUnownedPageRefusalOrderAndSignedCoordinates(t *testing.T) {
	var zero Device
	if got, receipt, err := zero.ProgramUnownedPage(math.MinInt64, math.MinInt64, nil, Cut{Kind: 255}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || receipt != (UnownedProgramReceipt{}) || !reflect.DeepEqual(got.Snapshot(), Snapshot{}) {
		t.Fatalf("zero device precedence: receipt=%+v error=%v", receipt, err)
	}
	legacy, _ := New(1, 4, 1)
	if _, receipt, err := legacy.ProgramUnownedPage(-1, -1, nil, Cut{Kind: 255}); !errors.Is(err, ErrEraseDisabled) || receipt != (UnownedProgramReceipt{}) {
		t.Fatalf("legacy device must refuse missing block profile first: %+v/%v", receipt, err)
	}
	device, _ := NewWithEraseBlocks(1, 6, 2, 2, 3)
	for _, block := range []int64{math.MinInt64, -1, 3, math.MaxInt64} {
		got, receipt, err := device.ProgramUnownedPage(block, -1, []byte{1, 2}, Cut{Kind: 255})
		if !errors.Is(err, ErrInvalidBlockIndex) || receipt != (UnownedProgramReceipt{}) || !reflect.DeepEqual(got.Snapshot(), device.Snapshot()) {
			t.Fatalf("block %d precedence receipt=%+v err=%v", block, receipt, err)
		}
	}
	for _, page := range []int64{math.MinInt64, -1, 2, math.MaxInt64} {
		got, receipt, err := device.ProgramUnownedPage(1, page, []byte{1, 2}, Cut{Kind: 255})
		if !errors.Is(err, ftlpagemap.ErrInvalidPhysicalPage) || receipt != (UnownedProgramReceipt{}) || !reflect.DeepEqual(got.Snapshot(), device.Snapshot()) {
			t.Fatalf("page %d precedence receipt=%+v err=%v", page, receipt, err)
		}
	}
	if _, receipt, err := device.ProgramUnownedPage(1, 0, []byte{1}, Cut{Kind: 255}); !errors.Is(err, nandpageprogramcuts.ErrWrongPayloadLength) || receipt != (UnownedProgramReceipt{}) {
		t.Fatalf("payload length must precede cut: %+v/%v", receipt, err)
	}
	if _, receipt, err := device.ProgramUnownedPage(1, 0, []byte{1, 2}, Cut{Kind: 255}); !errors.Is(err, nandpageprogramcuts.ErrInvalidCut) || receipt != (UnownedProgramReceipt{}) {
		t.Fatalf("invalid cut refusal: %+v/%v", receipt, err)
	}
}

func TestProgramUnownedPageRejectsLiveStaleActiveAndIncompleteTargets(t *testing.T) {
	device, _ := NewWithEraseBlocks(1, 6, 1, 3, 3)
	device, _, err := device.Write(0, []byte{0x31}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0x32}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	if _, receipt, err := device.ProgramUnownedPage(0, 0, []byte{0x42}, Cut{Kind: 255}); !errors.Is(err, ftlpagemap.ErrPhysicalPageUnavailable) || receipt != (UnownedProgramReceipt{}) {
		t.Fatalf("live page target receipt=%+v error=%v", receipt, err)
	}
	device, _, err = device.ProgramUnownedPage(1, 0, []byte{0x41}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if err != nil {
		t.Fatal(err)
	}
	if _, receipt, err := device.ProgramUnownedPage(1, 0, []byte{0x41}, Cut{Kind: nandpageprogramcuts.Complete}); !errors.Is(err, ftlpagemap.ErrPhysicalPageUnavailable) || receipt != (UnownedProgramReceipt{}) {
		t.Fatalf("stale page target receipt=%+v error=%v", receipt, err)
	}
	device, err = device.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	if _, receipt, err := device.ProgramUnownedPage(0, 2, []byte{0xff}, Cut{Kind: nandpageprogramcuts.AfterMedia}); !errors.Is(err, ErrEvacuationActive) || receipt != (UnownedProgramReceipt{}) {
		t.Fatalf("active block refusal receipt=%+v error=%v", receipt, err)
	}
	device, _, err = device.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if err != nil {
		t.Fatal(err)
	}
	before := device.Snapshot()
	if _, receipt, err := device.ProgramUnownedPage(0, 2, []byte{0xff}, Cut{Kind: 255}); !errors.Is(err, ErrEvacuationBlockIncomplete) || receipt != (UnownedProgramReceipt{}) || !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatalf("incomplete must precede active/cut refusal: receipt=%+v error=%v", receipt, err)
	}
}

func TestProgramUnownedPageEraseAndReuse(t *testing.T) {
	device, model := prepareBlockDevice(t, 1, 6, 2, 2, 3, 0)
	device, model = compareProgramUnowned(t, device, model, 2, 1, []byte{0x20, 0x40}, Cut{Kind: nandpageprogramcuts.Complete})
	if device.Snapshot().Map.PhysicalStatus[5] != ftlpagemap.Stale {
		t.Fatal("completed unowned program did not leave a stale page")
	}
	device, model = compareEraseBlock(t, device, model, 2, EraseCut{Kind: nandblockerasecuts.AfterMedia})
	if device.Snapshot().Map.PhysicalStatus[5] != ftlpagemap.Free || device.Snapshot().Pages[5].Programmed {
		t.Fatal("completed erase did not free and erase unowned page")
	}
	device, model = compareProgramUnowned(t, device, model, 2, 1, []byte{0x20, 0x40}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1})
	if device.Snapshot().Map.PhysicalStatus[5] != ftlpagemap.Stale || !device.Snapshot().Pages[5].Programmed {
		t.Fatal("reused unowned page did not become spent again")
	}
}

func TestFiveThousandSeededUnownedProgramHistories(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05E2B2D2))
	device, err := NewWithEraseBlocks(2, 8, 2, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 8, 2, 2, 3)
	cuts := []Cut{{Kind: nandpageprogramcuts.BeforeAccept}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1}, {Kind: nandpageprogramcuts.AfterMedia}, {Kind: nandpageprogramcuts.Complete}}
	for step := 0; step < 5_000; step++ {
		switch random.Intn(3) {
		case 0:
			device, model = compareProgramUnowned(t, device, model, int64(random.Intn(4)-1), int64(random.Intn(3)-1), []byte{byte(random.Intn(256)), byte(random.Intn(256))}, cuts[random.Intn(len(cuts))])
		case 1:
			device, model = compareTransition(t, device, model, int64(random.Intn(3)-1), []byte{byte(random.Intn(256)), byte(random.Intn(256))}, modelCut{kind: random.Intn(4), prefix: random.Intn(2)})
		case 2:
			device, model = compareEraseBlock(t, device, model, int64(random.Intn(5)-1), EraseCut{Kind: nandblockerasecuts.CutKind(random.Intn(4)), PrefixPages: random.Intn(2)})
		}
	}
}

func FuzzProgramUnownedPageHistories(f *testing.F) {
	f.Add([]byte{0, 0, 1, 1, 0, 2, 0, 4, 2, 0, 1, 3, 0, 2})
	f.Add([]byte{1, 0, 0, 0, 2, 1, 3, 0, 0, 4, 2, 1})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 256 {
			return
		}
		device, err := NewWithEraseBlocks(1, 4, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(1, 4, 2, 2, 2)
		for index := 0; index+4 < len(raw); index += 5 {
			block := int64(int(raw[index]%3) - 1)
			page := int64(int(raw[index+1]%4) - 1)
			payload := []byte{raw[index+2], raw[index+3]}
			cut := Cut{Kind: nandpageprogramcuts.CutKind(raw[index+4] % 5)}
			if cut.Kind == nandpageprogramcuts.AfterPrefix {
				cut.PrefixBytes = int(raw[index+2] % 3)
			}
			device, model = compareProgramUnowned(t, device, model, block, page, payload, cut)
		}
	})
}
