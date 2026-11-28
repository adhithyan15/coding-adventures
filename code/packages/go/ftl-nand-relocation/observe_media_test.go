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

func TestObserveMediaValidationOrderAndBounds(t *testing.T) {
	var zero Device
	if got, err := zero.ObserveMedia(); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || !reflect.DeepEqual(got, MediaObservation{}) {
		t.Fatalf("zero ObserveMedia = %+v/%v, want zero/invalid geometry", got, err)
	}
	legacy, err := New(1, 4, 4)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := legacy.ObserveMedia(); !errors.Is(err, ErrEraseDisabled) || !reflect.DeepEqual(got, MediaObservation{}) {
		t.Fatalf("legacy ObserveMedia = %+v/%v, want zero/erase disabled", got, err)
	}
	device, err := NewWithEraseBlocks(2, 8, 2, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	got, err := device.ObserveMedia()
	if err != nil || !reflect.DeepEqual(got, expectedMediaFromSnapshot(device)) {
		t.Fatalf("empty media = %+v/%v", got, err)
	}
}

func TestObserveMediaMatchesEveryBlockThroughProgramAndEraseCuts(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 8, 4, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0xff, 0xff, 0xff, 0xff}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	owner, present, err := device.ObserveOwner(0)
	if err != nil || !present {
		t.Fatalf("ObserveOwner setup = %+v/%v/%v", owner, present, err)
	}
	assertObserveMediaMatches(t, device)
	if !device.MatchesOwner(owner) {
		t.Fatal("capture changed the current owner identity")
	}
	// A consumed short program retains its exact prefix and marker.
	device, _, err = device.Write(1, []byte{0x12, 0x34, 0x56, 0x78}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 2})
	if err != nil {
		t.Fatal(err)
	}
	assertObserveMediaMatches(t, device)
	// Turn a stale-only extent into an active target, then a partial erase.
	device, _, err = device.Write(0, []byte{0xaa, 0xbb, 0xcc, 0xdd}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	active, err := device.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	assertObserveMediaMatches(t, active)
	if !active.evacuationActive {
		t.Fatal("setup did not retain an active evacuation")
	}
	partial, _, err := active.eraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1}, true)
	if err != nil {
		t.Fatal(err)
	}
	assertObserveMediaMatches(t, partial)
	if !partial.blocks[0].IncompleteErase || !partial.evacuationActive {
		t.Fatal("setup did not retain active incomplete erase state")
	}
	resumed, receipt, err := partial.eraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterMedia}, true)
	if err != nil || !receipt.Released || receipt.Acknowledged {
		t.Fatalf("resumed lost-ack erase receipt=%+v err=%v", receipt, err)
	}
	assertObserveMediaMatches(t, resumed)
	if resumed.blocks[0].CompletedErases != 1 || resumed.blocks[0].IncompleteErase {
		t.Fatalf("media-ready generation was not captured: %+v", resumed.blocks[0])
	}
}

func TestObserveMediaDeeplyDetachesEveryBlockAndCall(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 6, 3, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0x01, 0x02, 0x03}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	before := expectedMediaFromSnapshot(device)
	first, err := device.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	first.Blocks[0].Pages[0].Bytes[0] ^= 0xff
	first.Blocks[0].Pages[0].Programmed = false
	first.Blocks[0].CompletedErases++
	first.Blocks[1].Pages[1].Bytes[1] ^= 0xff
	first.Blocks[2].Pages[0].Bytes[2] ^= 0xff
	second, err := device.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(second, before) {
		t.Fatalf("mutating returned block or page changed device/next result: got=%+v want=%+v", second, before)
	}
	block, err := device.ObserveBlock(0)
	if err != nil || !reflect.DeepEqual(block, before.Blocks[0]) {
		t.Fatalf("single block capture after whole-media mutation = %+v/%v, want %+v", block, err, before.Blocks[0])
	}
}

func TestObserveMediaMaximumPageWidthAndSignedGeometry(t *testing.T) {
	device, err := NewWithEraseBlocks(64, 128, maxPageSize, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	observed, err := device.ObserveMedia()
	if err != nil || len(observed.Blocks) != 64 || len(observed.Blocks[63].Pages) != 2 || len(observed.Blocks[63].Pages[1].Bytes) != maxPageSize {
		t.Fatalf("max geometry capture blocks=%d last=%+v err=%v", len(observed.Blocks), observed.Blocks[len(observed.Blocks)-1], err)
	}
	for _, block := range []int64{math.MinInt64, -1, 64, math.MaxInt64} {
		if _, err := device.ObserveBlock(block); !errors.Is(err, ErrInvalidBlockIndex) {
			t.Errorf("ObserveBlock(%d) error=%v, want invalid index", block, err)
		}
	}
	assertObserveMediaMatches(t, device)
}

func TestSeededObserveMediaHistoriesMatchDetachedPhysicalModel(t *testing.T) {
	for seed := int64(0); seed < 32; seed++ {
		rng := rand.New(rand.NewSource(seed))
		device, err := NewWithEraseBlocks(2, 8, 3, 2, 3)
		if err != nil {
			t.Fatal(err)
		}
		for step := 0; step < 96; step++ {
			switch rng.Intn(4) {
			case 0, 1:
				logical := int64(rng.Intn(2))
				payload := []byte{byte(rng.Intn(256)), byte(rng.Intn(256)), byte(rng.Intn(256))}
				cut := Cut{Kind: nandpageprogramcuts.CutKind(rng.Intn(4))}
				if cut.Kind == nandpageprogramcuts.AfterPrefix {
					cut.PrefixBytes = rng.Intn(3)
				}
				next, _, callErr := device.Write(logical, payload, cut)
				if callErr == nil || errors.Is(callErr, ftlpagemap.ErrNoFreePage) || errors.Is(callErr, ErrEvacuationActive) {
					device = next
				} else {
					t.Fatalf("seed %d step %d write error %v", seed, step, callErr)
				}
			case 2, 3:
				block := int64(rng.Intn(4))
				cut := EraseCut{Kind: nandblockerasecuts.CutKind(rng.Intn(4))}
				if cut.Kind == nandblockerasecuts.AfterPrefix {
					cut.PrefixPages = rng.Intn(3)
				}
				next, _, callErr := device.EraseBlock(block, cut)
				if callErr == nil || errors.Is(callErr, ErrIneligibleBlock) || errors.Is(callErr, ErrEvacuationBlockIncomplete) || errors.Is(callErr, nandblockerasecuts.ErrEraseLimit) {
					device = next
				}
			}
			assertObserveMediaMatches(t, device)
		}
	}
}

func FuzzObserveMediaHistories(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7})
	f.Add([]byte{0xff, 0x80, 0x20, 0x10, 0x09})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 512 {
			commands = commands[:512]
		}
		device, err := NewWithEraseBlocks(2, 8, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		for _, command := range commands {
			switch command % 4 {
			case 0, 1:
				payload := []byte{command, command ^ 0xff}
				cut := Cut{Kind: nandpageprogramcuts.CutKind((command >> 2) % 4)}
				if cut.Kind == nandpageprogramcuts.AfterPrefix {
					cut.PrefixBytes = int(command>>4) % 2
				}
				next, _, callErr := device.Write(int64((command>>6)%2), payload, cut)
				if callErr == nil || errors.Is(callErr, ftlpagemap.ErrNoFreePage) || errors.Is(callErr, ErrEvacuationActive) {
					device = next
				}
			case 2, 3:
				cut := EraseCut{Kind: nandblockerasecuts.CutKind((command >> 2) % 4)}
				if cut.Kind == nandblockerasecuts.AfterPrefix {
					cut.PrefixPages = int(command>>5) % 3
				}
				next, _, callErr := device.EraseBlock(int64((command>>6)%4), cut)
				if callErr == nil || errors.Is(callErr, ErrIneligibleBlock) || errors.Is(callErr, ErrEvacuationBlockIncomplete) || errors.Is(callErr, nandblockerasecuts.ErrEraseLimit) {
					device = next
				}
			}
			assertObserveMediaMatches(t, device)
		}
	})
}

func assertObserveMediaMatches(t *testing.T, device Device) {
	t.Helper()
	got, err := device.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	want := expectedMediaFromSnapshot(device)
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("ObserveMedia()=%+v, want physical snapshot %+v", got, want)
	}
	for block := range want.Blocks {
		one, err := device.ObserveBlock(int64(block))
		if err != nil || !reflect.DeepEqual(one, got.Blocks[block]) {
			t.Fatalf("ObserveBlock(%d)=%+v/%v differs from media row %+v", block, one, err, got.Blocks[block])
		}
	}
}

func expectedMediaFromSnapshot(device Device) MediaObservation {
	snapshot := device.Snapshot()
	progress := device.ReclamationSnapshot()
	blocks := make([]nandblockerasecuts.Block, len(progress.Blocks))
	for block, metadata := range progress.Blocks {
		first := block * int(progress.PagesPerBlock)
		pages := make([]nandblockerasecuts.Page, int(progress.PagesPerBlock))
		for offset := range pages {
			page := snapshot.Pages[first+offset]
			pages[offset] = nandblockerasecuts.Page{Bytes: append([]byte(nil), page.Bytes...), Programmed: page.Programmed}
		}
		blocks[block] = nandblockerasecuts.Block{Pages: pages, CompletedErases: metadata.CompletedErases, IncompleteErase: metadata.IncompleteErase}
	}
	return MediaObservation{
		LogicalPages:       int64(len(snapshot.Map.LogicalToPhysical)),
		PagesPerBlock:      progress.PagesPerBlock,
		PageBytes:          device.pageSize,
		MaxCompletedErases: progress.MaxCompletedErases,
		Blocks:             blocks,
	}
}
