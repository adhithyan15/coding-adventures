package ftlnandrelocation

import (
	"bytes"
	"errors"
	"fmt"
	"reflect"
	"slices"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func compareWriteExcluding(t *testing.T, device Device, model referenceState, logical int64, payload []byte, cut Cut, excluded []bool) (Device, referenceState) {
	t.Helper()
	maskBefore := append([]bool(nil), excluded...)
	want, wantReceipt, wantError := referenceWriteExcluding(model, logical, payload, runtimeModelCut(cut), excluded)
	got, receipt, err := device.WriteExcluding(logical, payload, cut, excluded)
	if !slices.Equal(excluded, maskBefore) {
		t.Fatal("WriteExcluding changed the caller's exclusion mask")
	}
	assertError(t, err, wantError)
	if wantError != "" {
		if receipt != (Receipt{}) || !reflect.DeepEqual(got.Snapshot(), device.Snapshot()) || !reflect.DeepEqual(got.ReclamationSnapshot(), device.ReclamationSnapshot()) {
			t.Fatalf("refused fenced write mutated input: receipt=%+v error=%v", receipt, err)
		}
		assertLookupMatchesModel(t, device, model)
		return device, model
	}
	if receipt != (Receipt{Physical: wantReceipt.physical, Published: wantReceipt.published, Acknowledged: wantReceipt.acknowledged}) {
		if receipt.Physical != wantReceipt.physical {
			t.Fatalf("external fence write selected physical %d; model chooses %d", receipt.Physical, wantReceipt.physical)
		}
		t.Fatalf("fenced write receipt=%+v want=%+v", receipt, wantReceipt)
	}
	assertDeviceEquals(t, got, want)
	if len(excluded) != 0 {
		for physical, blocked := range excluded {
			if blocked && receipt.Physical == int64(physical) {
				t.Fatalf("allocator selected externally excluded page %d", physical)
			}
		}
	}
	return got, want
}

func TestEmptyFenceMatchesWriteAcrossBoundedStateGraph(t *testing.T) {
	for _, geometry := range []struct {
		logical, physical int64
		blocks            int64
	}{{logical: 1, physical: 2}, {logical: 1, physical: 4, blocks: 2}} {
		var initial Device
		var err error
		var model referenceState
		if geometry.blocks == 0 {
			initial, err = New(geometry.logical, geometry.physical, 1)
			model = referenceInitial(geometry.logical, geometry.physical, 1)
		} else {
			initial, err = NewWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.blocks, 1)
			model = referenceInitialWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.blocks, 1)
		}
		if err != nil {
			t.Fatal(err)
		}
		t.Run(fmt.Sprintf("physical-%d", geometry.physical), func(t *testing.T) {
			type node struct {
				device Device
				model  referenceState
			}
			seen := map[string]bool{referenceKey(model): true}
			queue := []node{{device: initial, model: model}}
			transitions := 0
			for len(queue) > 0 {
				current := queue[0]
				queue = queue[1:]
				for maskBits := 0; maskBits < 1<<geometry.physical; maskBits++ {
					mask := make([]bool, int(geometry.physical))
					for physical := range mask {
						mask[physical] = maskBits&(1<<physical) != 0
					}
					for _, payloadByte := range []byte{0x00, 0xff} {
						for _, cut := range []Cut{
							{Kind: nandpageprogramcuts.BeforeAccept},
							{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
							{Kind: nandpageprogramcuts.AfterMedia},
							{Kind: nandpageprogramcuts.Complete},
						} {
							payload := []byte{payloadByte}
							next, nextModel := compareWriteExcluding(t, current.device, current.model, 0, payload, cut, mask)
							// The empty exclusion is the compatibility definition of Write.
							if maskBits == 0 {
								oldNext, oldReceipt, oldErr := current.device.Write(0, payload, cut)
								newNext, newReceipt, newErr := current.device.WriteExcluding(0, payload, cut, nil)
								if errorCode(oldErr) != errorCode(newErr) || oldReceipt != newReceipt || !reflect.DeepEqual(oldNext.Snapshot(), newNext.Snapshot()) {
									t.Fatalf("empty-mask Write mismatch: old=%+v/%v new=%+v/%v", oldReceipt, oldErr, newReceipt, newErr)
								}
							}
							transitions++
							key := referenceKey(nextModel)
							if !seen[key] {
								seen[key] = true
								queue = append(queue, node{device: next, model: nextModel})
							}
						}
					}
				}
			}
			t.Logf("closed %d states with %d external-mask transitions", len(seen), transitions)
		})
	}
}

func TestExternalFenceValidationAndAtomicRefusals(t *testing.T) {
	device, err := New(1, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	before := device.Snapshot()
	var zero Device
	if _, receipt, callErr := zero.WriteExcluding(-1, nil, Cut{Kind: nandpageprogramcuts.CutKind(255)}, []bool{true}); !errors.Is(callErr, ftlpagemap.ErrInvalidGeometry) || receipt != (Receipt{}) {
		t.Fatalf("unconfigured device must precede every other refusal: receipt=%+v error=%v", receipt, callErr)
	}
	for _, testCase := range []struct {
		name    string
		logical int64
		mask    []bool
		want    string
	}{
		{name: "logical precedes malformed width", logical: -1, mask: []bool{true}, want: "invalid_logical_page"},
		{name: "minimum signed address precedes malformed width", logical: -1 << 63, mask: []bool{true}, want: "invalid_logical_page"},
		{name: "maximum signed address precedes malformed width", logical: 1<<63 - 1, mask: []bool{true}, want: "invalid_logical_page"},
		{name: "malformed width", logical: 0, mask: []bool{true}, want: "invalid_fence"},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			got, receipt, callErr := device.WriteExcluding(testCase.logical, []byte{0x42}, Cut{Kind: nandpageprogramcuts.CutKind(255)}, testCase.mask)
			assertError(t, callErr, testCase.want)
			if receipt != (Receipt{}) || !reflect.DeepEqual(got.Snapshot(), before) {
				t.Fatalf("refusal changed media: receipt=%+v", receipt)
			}
		})
	}
	pendingTable, err := ftlpagemap.New(1, 2)
	if err != nil {
		t.Fatal(err)
	}
	pendingTable, _, err = pendingTable.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	pending := Device{table: pendingTable, pages: []nandpageprogramcuts.PageState{
		{Bytes: []byte{0xff}}, {Bytes: []byte{0xff}},
	}, pageSize: 1}
	for _, testCase := range []struct {
		logical int64
		want    string
	}{{logical: -1, want: "invalid_logical_page"}, {logical: 0, want: "reservation_pending"}} {
		if _, receipt, callErr := pending.WriteExcluding(testCase.logical, []byte{0x42}, Cut{}, []bool{true}); errorCode(callErr) != testCase.want || receipt != (Receipt{}) {
			t.Fatalf("pending table order for logical %d: receipt=%+v error=%v want=%s", testCase.logical, receipt, callErr, testCase.want)
		}
	}
	allBlocked := []bool{true, true}
	got, receipt, err := device.WriteExcluding(0, []byte{0x42}, Cut{Kind: nandpageprogramcuts.CutKind(255)}, allBlocked)
	assertError(t, err, "no_free_page")
	if receipt != (Receipt{}) || !reflect.DeepEqual(got.Snapshot(), before) {
		t.Fatalf("all-blocked refusal changed media: receipt=%+v", receipt)
	}
	full, _ := New(1, 2, 1)
	full, _, _ = full.Write(0, []byte{0x10}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	full, _, _ = full.Write(0, []byte{0x20}, Cut{Kind: nandpageprogramcuts.Complete})
	fullBefore := full.Snapshot()
	_, receipt, err = full.WriteExcluding(0, nil, Cut{Kind: nandpageprogramcuts.CutKind(255)}, []bool{true})
	assertError(t, err, "invalid_fence")
	if receipt != (Receipt{}) || !reflect.DeepEqual(full.Snapshot(), fullBefore) {
		t.Fatal("bad fence must precede capacity, payload and cut refusals")
	}
	_, receipt, err = full.WriteExcluding(0, nil, Cut{Kind: nandpageprogramcuts.CutKind(255)}, []bool{false, false})
	assertError(t, err, "no_free_page")
	if receipt != (Receipt{}) || !reflect.DeepEqual(full.Snapshot(), fullBefore) {
		t.Fatal("capacity must precede payload and cut refusals")
	}
}

func TestExternalFenceCombinesWithInternalFencesAndPreservesLiveReads(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 9, 2, 3, 3)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 9, 2, 3, 3)
	device, model = compareTransition(t, device, model, 0, []byte{0x10, 0x20}, modelCut{kind: cutAfterPrefix, prefix: 0})
	device, model = compareEvacuationBegin(t, device, model, 0)
	device, model = compareTransition(t, device, model, 1, []byte{0x30, 0x40}, modelCut{kind: cutAfterPrefix, prefix: 0})
	device, model = compareEraseBlock(t, device, model, 1, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})

	ordinary, ordinaryModel := compareEvacuationWrite(t, device, model, 0, []byte{0x51, 0x52}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if physical := ordinary.Snapshot().Map.PhysicalStatus; physical[6] != ftlpagemap.Stale {
		t.Fatalf("ordinary allocator should choose page 6 first: %v", physical)
	}
	excluded := make([]bool, 9)
	excluded[6] = true
	fenced, fencedModel := compareWriteExcluding(t, device, model, 0, []byte{0x51, 0x52}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, excluded)
	if physical := fenced.Snapshot().Map.PhysicalStatus; physical[7] != ftlpagemap.Stale || physical[6] != ftlpagemap.Free {
		t.Fatalf("external mask did not combine with active and incomplete-block fences: %v", physical)
	}
	assertDeviceEquals(t, ordinary, ordinaryModel)
	assertDeviceEquals(t, fenced, fencedModel)

	liveDevice, _ := New(2, 4, 1)
	liveDevice, _, err = liveDevice.Write(0, []byte{0x71}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	mask := []bool{true, false, false, false}
	liveDevice, _, err = liveDevice.WriteExcluding(1, []byte{0x82}, Cut{Kind: nandpageprogramcuts.Complete}, mask)
	if err != nil {
		t.Fatal(err)
	}
	mask[0] = false
	if data, present, readErr := liveDevice.Read(0); readErr != nil || !present || !bytes.Equal(data, []byte{0x71}) {
		t.Fatalf("externally fenced live mapping stopped being readable: %x/%t/%v", data, present, readErr)
	}

	owned, _ := New(1, 3, 1)
	callerMask := []bool{true, false, false}
	owned, receipt, err := owned.WriteExcluding(0, []byte{0x91}, Cut{Kind: nandpageprogramcuts.Complete}, callerMask)
	if err != nil || receipt.Physical != 1 {
		t.Fatalf("first caller-masked write=%+v/%v", receipt, err)
	}
	callerMask[0] = false
	owned, receipt, err = owned.Write(0, []byte{0x92}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || receipt.Physical != 0 {
		t.Fatalf("later ordinary write retained caller mask: receipt=%+v error=%v", receipt, err)
	}
}

func FuzzWriteExcludingHistory(f *testing.F) {
	f.Add([]byte{0, 0, 0, 1, 1, 0xff, 2, 3, 0xff, 0, 4, 0x55})
	f.Add([]byte{1, 1, 0xff, 0, 2, 0, 0, 1})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			return
		}
		device, err := New(2, 4, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitial(2, 4, 2)
		for index := 0; index+2 < len(commands); index += 3 {
			logical := int64(commands[index] % 2)
			payload := []byte{commands[index+1], commands[index+1] ^ 0xa5}
			cut := fuzzProgramCut(commands[index+2])
			var mask []bool
			switch commands[index] % 4 {
			case 0:
				mask = nil
			case 1:
				mask = []bool{}
			case 2:
				mask = make([]bool, 4)
				for physical := range mask {
					mask[physical] = commands[index+1]&(1<<physical) != 0
				}
			case 3:
				mask = []bool{true}
			}
			device, model = compareWriteExcluding(t, device, model, logical, payload, cut, mask)
			if len(mask) == 0 {
				legacy, legacyReceipt, legacyErr := device.Write(logical, payload, cut)
				newPath, newReceipt, newErr := device.WriteExcluding(logical, payload, cut, mask)
				if errorCode(legacyErr) != errorCode(newErr) || legacyReceipt != newReceipt || !reflect.DeepEqual(legacy.Snapshot(), newPath.Snapshot()) {
					t.Fatal("empty caller mask diverged from Write")
				}
			}
		}
	})
}

func fuzzProgramCut(selector byte) Cut {
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
