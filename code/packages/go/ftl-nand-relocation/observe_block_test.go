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

func TestObserveBlockValidationOrderAndSignedBounds(t *testing.T) {
	var zero Device
	if got, err := zero.ObserveBlock(math.MinInt64); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || !reflect.DeepEqual(got, nandblockerasecuts.Block{}) {
		t.Fatalf("zero ObserveBlock = %+v/%v, want zero/invalid geometry", got, err)
	}
	legacy, err := New(1, 4, 4)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := legacy.ObserveBlock(math.MinInt64); !errors.Is(err, ErrEraseDisabled) || !reflect.DeepEqual(got, nandblockerasecuts.Block{}) {
		t.Fatalf("legacy ObserveBlock = %+v/%v, want zero/erase disabled", got, err)
	}
	device, err := NewWithEraseBlocks(1, 8, 2, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	for _, block := range []int64{math.MinInt64, -1, 4, math.MaxInt64} {
		if got, err := device.ObserveBlock(block); !errors.Is(err, ErrInvalidBlockIndex) || !reflect.DeepEqual(got, nandblockerasecuts.Block{}) {
			t.Errorf("ObserveBlock(%d) = %+v/%v, want zero/invalid block", block, got, err)
		}
	}
}

func TestObserveBlockMatchesPhysicalSnapshotForEmptyLiveStaleAndActive(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 8, 4, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	assertObservedBlockMatchesSnapshot(t, device, 3)
	for index, payload := range [][]byte{{0x10, 0x20, 0x30, 0x40}, {0x11, 0x21, 0x31, 0x41}, {0x12, 0x22, 0x32, 0x42}} {
		device, _, err = device.Write(0, payload, Cut{Kind: nandpageprogramcuts.Complete})
		if err != nil {
			t.Fatal(err)
		}
		assertObservedBlockMatchesSnapshot(t, device, int64(index/2))
	}
	if device.Snapshot().Map.PhysicalStatus[0] != ftlpagemap.Stale {
		t.Fatal("setup did not leave a stale programmed page")
	}
	active, err := device.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	assertObservedBlockMatchesSnapshot(t, active, 0)
	if !active.evacuationActive {
		t.Fatal("setup did not activate evacuation")
	}
	if got, err := active.ObserveBlock(0); err != nil || !reflect.DeepEqual(got, observationFromSnapshot(active, 0)) {
		t.Fatalf("active block should remain observable: %+v %v", got, err)
	}
}

func TestObserveBlockPreservesPartiallyProgrammedPageBytes(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 4, 4, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0x11, 0x22, 0x33, 0x44}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 2})
	if err != nil {
		t.Fatal(err)
	}
	observed, err := device.ObserveBlock(0)
	if err != nil {
		t.Fatal(err)
	}
	if want := []byte{0x11, 0x22, 0xff, 0xff}; !reflect.DeepEqual(observed.Pages[0].Bytes, want) || !observed.Pages[0].Programmed {
		t.Fatalf("partially programmed page=%+v, want %x with marker", observed.Pages[0], want)
	}
	assertObservedBlockMatchesSnapshot(t, device, 0)
}

func TestObserveBlockExposesEveryPartialEraseAndLostAckReuse(t *testing.T) {
	for prefix := 0; prefix < 2; prefix++ {
		device, _ := prepareStaleFreeBlock(t)
		// Create a second spent page in the same block so the erase has a
		// complete stale-only extent to operate on.
		device, _, err := device.Write(0, []byte{0x42}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
		if err != nil {
			t.Fatal(err)
		}
		partial, _, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: prefix})
		if err != nil {
			t.Fatal(err)
		}
		observed, err := partial.ObserveBlock(0)
		if err != nil || !observed.IncompleteErase || observed.CompletedErases != 0 || len(observed.Pages) != 2 {
			t.Fatalf("prefix %d observation metadata=%+v err=%v", prefix, observed, err)
		}
		for page := 0; page < 2; page++ {
			wantErased := page < prefix
			if observed.Pages[page].Programmed == wantErased {
				t.Fatalf("prefix %d page %d marker=%v", prefix, page, observed.Pages[page].Programmed)
			}
			if wantErased {
				for _, value := range observed.Pages[page].Bytes {
					if value != 0xff {
						t.Fatalf("prefix %d erased page %d has byte %02x", prefix, page, value)
					}
				}
			}
		}
		assertObservedBlockMatchesSnapshot(t, partial, 0)
		released, receipt, err := partial.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterMedia})
		if err != nil || !receipt.Released || receipt.Acknowledged {
			t.Fatalf("lost-ack erase receipt=%+v err=%v", receipt, err)
		}
		assertObservedBlockMatchesSnapshot(t, released, 0)
		afterErase, err := released.ObserveBlock(0)
		if err != nil || afterErase.CompletedErases != 1 || afterErase.IncompleteErase {
			t.Fatalf("media-complete observation=%+v err=%v", afterErase, err)
		}
		reused, _, err := released.Write(0, []byte{0x51}, Cut{Kind: nandpageprogramcuts.Complete})
		if err != nil {
			t.Fatal(err)
		}
		assertObservedBlockMatchesSnapshot(t, reused, 0)
		reusedBlock, err := reused.ObserveBlock(0)
		if err != nil || reusedBlock.CompletedErases != 1 || reusedBlock.IncompleteErase || !reusedBlock.Pages[0].Programmed || reusedBlock.Pages[0].Bytes[0] != 0x51 {
			t.Fatalf("reused page observation=%+v err=%v", reusedBlock, err)
		}
	}
}

func TestObserveBlockDeeplyDetachesPagesAndBytes(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 4, 3, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0x12, 0x34, 0x56}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	before := device.Snapshot()
	observed, err := device.ObserveBlock(0)
	if err != nil {
		t.Fatal(err)
	}
	observed.Pages[0].Bytes[0] ^= 0xff
	observed.Pages[0].Programmed = false
	observed.Pages[1].Bytes[0] ^= 0xff
	observed.Pages = append(observed.Pages, nandblockerasecuts.Page{Bytes: []byte{1}})
	if !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatal("mutating observation changed the device or its retained snapshot")
	}
	again, err := device.ObserveBlock(0)
	if err != nil || !reflect.DeepEqual(again, observationFromSnapshot(device, 0)) {
		t.Fatalf("fresh observation after mutation=%+v err=%v", again, err)
	}
}

func TestObserveBlockLastBlockAndMaximumPageWidth(t *testing.T) {
	for _, pagesPerBlock := range []int64{2, 4} {
		device, err := NewWithEraseBlocks(1, 8, 3, pagesPerBlock, 1)
		if err != nil {
			t.Fatal(err)
		}
		block := int64(8/pagesPerBlock - 1)
		assertObservedBlockMatchesSnapshot(t, device, block)
	}
	device, err := NewWithEraseBlocks(1, 128, maxPageSize, 128, 1)
	if err != nil {
		t.Fatal(err)
	}
	observed, err := device.ObserveBlock(0)
	if err != nil || len(observed.Pages) != 128 || len(observed.Pages[127].Bytes) != maxPageSize {
		t.Fatalf("maximum block observation pages=%d bytes=%d err=%v", len(observed.Pages), len(observed.Pages[127].Bytes), err)
	}
	for _, index := range []int{0, 63, 127} {
		for _, value := range []byte{observed.Pages[index].Bytes[0], observed.Pages[index].Bytes[maxPageSize-1]} {
			if value != 0xff {
				t.Fatalf("maximum page %d is not erased: %02x", index, value)
			}
		}
	}
}

func TestSeededObserveBlockHistoriesMatchSnapshots(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05E2B2C1))
	widths := []int64{1, 2, 4}
	for sample := 0; sample < 1024; sample++ {
		pagesPerBlock := widths[random.Intn(len(widths))]
		device, err := NewWithEraseBlocks(2, 8, 8, pagesPerBlock, 2)
		if err != nil {
			t.Fatal(err)
		}
		for step := 0; step < 12; step++ {
			switch random.Intn(3) {
			case 0:
				logical := int64(random.Intn(2))
				payload := make([]byte, 8)
				for index := range payload {
					payload[index] = byte(random.Intn(256))
				}
				cut := Cut{Kind: nandpageprogramcuts.Complete}
				if random.Intn(4) == 0 {
					cut = Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: random.Intn(len(payload))}
				}
				device, _, err = device.Write(logical, payload, cut)
				if err != nil && !errors.Is(err, ftlpagemap.ErrNoFreePage) {
					t.Fatal(err)
				}
			case 1:
				block := int64(random.Intn(int(8 / pagesPerBlock)))
				cut := EraseCut{Kind: nandblockerasecuts.CutKind(random.Intn(4))}
				if cut.Kind == nandblockerasecuts.AfterPrefix {
					cut.PrefixPages = random.Intn(int(pagesPerBlock))
				}
				device, _, err = device.EraseBlock(block, cut)
				if err != nil && !errors.Is(err, ErrIneligibleBlock) && !errors.Is(err, nandblockerasecuts.ErrEraseLimit) {
					t.Fatal(err)
				}
			}
			block := int64(random.Intn(int(8 / pagesPerBlock)))
			assertObservedBlockMatchesSnapshot(t, device, block)
		}
	}
}

func FuzzObserveBlockHistories(f *testing.F) {
	f.Add([]byte{1, 0, 0, 0, 0, 2, 3, 1})
	f.Add([]byte{3, 2, 1, 4, 0, 2, 1, 0})
	f.Add([]byte{0, 0, 3, 1, 2, 0, 1, 2})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			t.Skip()
		}
		device, err := NewWithEraseBlocks(2, 8, 4, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		for index, command := range commands {
			switch command % 3 {
			case 0:
				logical := int64((command >> 2) % 2)
				payload := []byte{command, byte(index), command ^ 0x5a, 0x7f}
				cut := Cut{Kind: nandpageprogramcuts.CutKind((command >> 4) % 4)}
				if cut.Kind == nandpageprogramcuts.AfterPrefix {
					cut.PrefixBytes = int(command>>6) % len(payload)
				}
				device, _, err = device.Write(logical, payload, cut)
				if err != nil && !errors.Is(err, ftlpagemap.ErrNoFreePage) {
					t.Fatal(err)
				}
			case 1:
				block := int64((command >> 2) % 4)
				cut := EraseCut{Kind: nandblockerasecuts.CutKind((command >> 4) % 4)}
				if cut.Kind == nandblockerasecuts.AfterPrefix {
					cut.PrefixPages = int((command >> 6) % 2)
				}
				device, _, err = device.EraseBlock(block, cut)
				if err != nil && !errors.Is(err, ErrIneligibleBlock) && !errors.Is(err, nandblockerasecuts.ErrEraseLimit) {
					t.Fatal(err)
				}
			}
			assertObservedBlockMatchesSnapshot(t, device, int64((command>>2)%4))
		}
	})
}

func assertObservedBlockMatchesSnapshot(t *testing.T, device Device, block int64) {
	t.Helper()
	got, err := device.ObserveBlock(block)
	if err != nil {
		t.Fatal(err)
	}
	want := observationFromSnapshot(device, block)
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("ObserveBlock(%d)=%+v, want full snapshot range %+v", block, got, want)
	}
}

func observationFromSnapshot(device Device, block int64) nandblockerasecuts.Block {
	snapshot := device.Snapshot()
	progress := device.ReclamationSnapshot()
	first := int(block * progress.PagesPerBlock)
	end := first + int(progress.PagesPerBlock)
	want := nandblockerasecuts.Block{
		Pages:           make([]nandblockerasecuts.Page, end-first),
		CompletedErases: progress.Blocks[int(block)].CompletedErases,
		IncompleteErase: progress.Blocks[int(block)].IncompleteErase,
	}
	for physical := first; physical < end; physical++ {
		page := snapshot.Pages[physical]
		want.Pages[physical-first] = nandblockerasecuts.Page{Bytes: append([]byte(nil), page.Bytes...), Programmed: page.Programmed}
	}
	return want
}
