package ftlnandrelocation

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftleraseroleevidence "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-role-evidence"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestCaptureEnumeratesEveryStaleFreeLayoutAndRequalifiesEraseCut(t *testing.T) {
	checked := 0
	for pagesPerBlock := int64(2); pagesPerBlock <= 6; pagesPerBlock++ {
		for staleMask := 1; staleMask < 1<<pagesPerBlock; staleMask++ {
			device, err := NewWithEraseBlocks(1, 2*pagesPerBlock, 2, pagesPerBlock, 3)
			if err != nil {
				t.Fatal(err)
			}
			for page := int64(0); page < pagesPerBlock; page++ {
				if staleMask&(1<<page) != 0 {
					device, _, err = device.ProgramUnownedPage(0, page, []byte{0xff, byte(page)}, Cut{Kind: nandpageprogramcuts.Complete})
					if err != nil {
						t.Fatalf("pages=%d mask=%x page=%d: %v", pagesPerBlock, staleMask, page, err)
					}
				}
			}
			before := device.Snapshot()
			beforeReclamation := device.ReclamationSnapshot()
			intent, err := device.CaptureEraseIntent(0, 17)
			if err != nil || intent.Sequence != 17 || intent.Block != 0 || intent.CompletedErases != 0 {
				t.Fatalf("pages=%d mask=%x capture=%+v err=%v", pagesPerBlock, staleMask, intent, err)
			}
			roles := make([]ftlpagemap.PhysicalStatus, int(pagesPerBlock))
			for page := range roles {
				if staleMask&(1<<page) != 0 {
					roles[page] = ftlpagemap.Stale
				}
			}
			if !reflect.DeepEqual(intent.Stale, boolRoles(roles)) {
				t.Fatalf("pages=%d mask=%x stale mask=%v, want %v", pagesPerBlock, staleMask, intent.Stale, boolRoles(roles))
			}
			if !reflect.DeepEqual(before, device.Snapshot()) || !reflect.DeepEqual(beforeReclamation, device.ReclamationSnapshot()) {
				t.Fatal("capture changed its source device")
			}

			// The maximum legal prefix leaves the final physical slot untouched.
			device, receipt, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: int(pagesPerBlock - 1)})
			if err != nil || receipt.Released {
				t.Fatalf("partial cut receipt=%+v err=%v", receipt, err)
			}
			media, err := device.ObserveMedia()
			if err != nil {
				t.Fatal(err)
			}
			block := media.Blocks[0]
			programmed := make([]bool, int(pagesPerBlock))
			for page, value := range block.Pages {
				programmed[page] = value.Programmed
			}
			recoveredRoles, err := ftleraseroleevidence.Qualify(
				ftleraseroleevidence.Geometry{
					BlockCount: 2, PagesPerBlock: pagesPerBlock, MaxCompletedErases: 3,
				},
				17, intent,
				ftleraseroleevidence.BlockObservation{
					Block: 0, CompletedErases: block.CompletedErases,
					IncompleteErase: block.IncompleteErase, Programmed: programmed,
				},
			)
			if err != nil || !reflect.DeepEqual(recoveredRoles, roles) {
				t.Fatalf("qualifier roles=%v err=%v, want %v", recoveredRoles, err, roles)
			}
			snapshot := device.table.Snapshot()
			reopened, err := RestoreWithInterruptedErases(media, snapshot)
			if err != nil {
				t.Fatalf("raw restore after retained mask: %v", err)
			}
			if !reflect.DeepEqual(reopened.Snapshot().Map.PhysicalStatus[:int(pagesPerBlock)], recoveredRoles) {
				t.Fatalf("reopened roles %v differ from retained %v", reopened.Snapshot().Map.PhysicalStatus, recoveredRoles)
			}
			checked++
		}
	}
	if checked != 119 {
		t.Fatalf("checked %d stale/free layouts; want 119", checked)
	}
	t.Logf("captured and requalified %d nonempty stale/free layouts", checked)
}

func TestCapturePrecedenceAndActiveEvacuation(t *testing.T) {
	legacy, err := New(1, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := legacy.CaptureEraseIntent(-1, 1); !errors.Is(err, ErrEraseDisabled) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("legacy profile order: %+v/%v", got, err)
	}
	device, err := NewWithEraseBlocks(1, 4, 2, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := device.CaptureEraseIntent(-1, 1); !errors.Is(err, ErrInvalidBlockIndex) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("invalid target: %+v/%v", got, err)
	}
	if got, err := device.CaptureEraseIntent(0, 1); !errors.Is(err, ErrIneligibleBlock) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("free block eligibility: %+v/%v", got, err)
	}
	if _, _, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete}); !errors.Is(err, ErrIneligibleBlock) {
		t.Fatalf("raw erase and capture role eligibility diverged for an all-free block: %v", err)
	}

	// A live owner is rejected by the shared predicate even when stale work
	// exists and the target is currently the active evacuation block.
	live, _, err := device.Write(0, []byte{0x11, 0x22}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	live, _, err = live.Write(0, []byte{0x33, 0x44}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	if got, err := live.CaptureEraseIntent(0, 1); !errors.Is(err, ErrIneligibleBlock) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("live/stale capture must refuse: %+v/%v", got, err)
	}
	if _, _, err := live.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete}); !errors.Is(err, ErrIneligibleBlock) {
		t.Fatalf("raw erase and capture role eligibility diverged for live/stale block: %v", err)
	}
	activeLive, err := live.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := activeLive.CaptureEraseIntent(0, 1); !errors.Is(err, ErrIneligibleBlock) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("active live target: %+v/%v", got, err)
	}
	activeDrained, _, err := activeLive.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	progress, active := activeDrained.EvacuationProgress()
	if !active || progress.RemainingLivePages != 0 {
		t.Fatalf("drained active progress=%+v active=%t", progress, active)
	}
	intent, err := activeDrained.CaptureEraseIntent(0, 23)
	if err != nil || intent.Sequence != 23 || intent.Block != 0 {
		t.Fatalf("drained active target capture=%+v err=%v", intent, err)
	}

	// An active target elsewhere does not change direct-erase eligibility.
	other, err := NewWithEraseBlocks(1, 4, 2, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	other, _, err = other.ProgramUnownedPage(0, 0, []byte{0x51, 0x61}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	other, _, err = other.ProgramUnownedPage(1, 0, []byte{0xff, 0xff}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	otherActive, err := other.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := otherActive.CaptureEraseIntent(1, 24); err != nil || got.Block != 1 || !got.Stale[0] {
		t.Fatalf("different block with active target capture=%+v err=%v", got, err)
	}
}

func TestCaptureSequenceBudgetIncompleteAndGeneration(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 4, 2, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.ProgramUnownedPage(0, 0, []byte{0xff, 0xff}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	if !device.pages[0].Programmed || !reflect.DeepEqual(device.pages[0].Bytes, []byte{0xff, 0xff}) {
		t.Fatal("all-ones program did not retain a set marker with erased-value bytes")
	}
	before := device.Snapshot()
	if got, err := device.CaptureEraseIntent(0, 0); !errors.Is(err, ErrInvalidEraseIntentSequence) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("zero sequence capture=%+v/%v", got, err)
	}
	maxSequence := ^uint64(0)
	intent, err := device.CaptureEraseIntent(0, maxSequence)
	if err != nil || intent.Sequence != maxSequence || intent.Stale[0] == false {
		t.Fatalf("max sequence capture=%+v/%v", intent, err)
	}
	intent.Stale[0] = false
	if !reflect.DeepEqual(before, device.Snapshot()) {
		t.Fatal("returned stale mask aliased device state")
	}
	erased, _, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	erased, _, err = erased.ProgramUnownedPage(0, 0, []byte{0xff, 0xff}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	higher, err := erased.CaptureEraseIntent(0, 25)
	if err != nil || higher.CompletedErases != 1 || !higher.Stale[0] {
		t.Fatalf("higher generation capture=%+v err=%v", higher, err)
	}

	partialSource, err := NewWithEraseBlocks(1, 4, 2, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	partialSource, _, err = partialSource.ProgramUnownedPage(0, 0, []byte{0x41, 0x42}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	partialSource, _, err = partialSource.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if err != nil {
		t.Fatal(err)
	}
	if got, err := partialSource.CaptureEraseIntent(0, 0); !errors.Is(err, ErrEraseIntentBlockIncomplete) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("prior incomplete erase must be refused: %+v/%v", got, err)
	}
	fullBudget, err := NewWithEraseBlocks(1, 4, 2, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	fullBudget, _, err = fullBudget.ProgramUnownedPage(0, 0, []byte{0x41, 0x42}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	fullBudget, _, err = fullBudget.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	fullBudget, _, err = fullBudget.ProgramUnownedPage(0, 0, []byte{0x51, 0x52}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	if got, err := fullBudget.CaptureEraseIntent(0, 0); !errors.Is(err, nandblockerasecuts.ErrEraseLimit) || !reflect.DeepEqual(got, ftleraseroleevidence.RetainedIntent{}) {
		t.Fatalf("erase budget must precede zero sequence: %+v/%v", got, err)
	}
}

func TestCaptureSeededModelAndFuzz(t *testing.T) {
	random := rand.New(rand.NewSource(0xe3b3e))
	for trial := 0; trial < 5000; trial++ {
		device, block := randomCaptureDevice(t, random)
		sequence := uint64(random.Intn(20))
		if random.Intn(4) == 0 {
			device, _, _ = device.EraseBlock(block, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
		}
		before := device.Snapshot()
		want, wantErr := modelCapture(device, block, sequence)
		got, err := device.CaptureEraseIntent(block, sequence)
		if !sameCaptureIntent(got, want) || !sameCaptureError(err, wantErr) {
			t.Fatalf("trial=%d got=%+v/%v want=%+v/%v", trial, got, err, want, wantErr)
		}
		if !reflect.DeepEqual(before, device.Snapshot()) {
			t.Fatalf("trial=%d capture changed old state", trial)
		}
	}
}

func FuzzCaptureEraseIntent(f *testing.F) {
	for _, seed := range [][]byte{{}, {0}, {1, 2, 3, 4}, {0xff, 0x55, 0xaa, 7}, {8, 1, 0, 9}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 4 {
			raw = raw[:4]
		}
		random := rand.New(rand.NewSource(captureSeed(raw)))
		device, block := randomCaptureDevice(t, random)
		if len(raw) > 0 && raw[0]&0x80 != 0 {
			prefix := int(raw[0] % byte(device.pagesPerBlock))
			device, _, _ = device.EraseBlock(block, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: prefix})
		}
		sequence := uint64(random.Intn(20))
		before := device.Snapshot()
		want, wantErr := modelCapture(device, block, sequence)
		got, err := device.CaptureEraseIntent(block, sequence)
		if !sameCaptureIntent(got, want) || !sameCaptureError(err, wantErr) {
			t.Fatalf("raw=%x got=%+v/%v want=%+v/%v", raw, got, err, want, wantErr)
		}
		if !reflect.DeepEqual(before, device.Snapshot()) {
			t.Fatal("capture changed source device")
		}
	})
}

func randomCaptureDevice(t *testing.T, random *rand.Rand) (Device, int64) {
	t.Helper()
	pages := int64(2 + random.Intn(5))
	device, err := NewWithEraseBlocks(1, 2*pages, 2, pages, 3)
	if err != nil {
		t.Fatal(err)
	}
	block := int64(random.Intn(2))
	for page := int64(0); page < pages; page++ {
		if random.Intn(2) == 0 {
			continue
		}
		device, _, err = device.ProgramUnownedPage(block, page, []byte{byte(random.Intn(256)), byte(random.Intn(256))}, Cut{Kind: nandpageprogramcuts.Complete})
		if err != nil {
			t.Fatal(err)
		}
	}
	return device, block
}

func modelCapture(device Device, block int64, sequence uint64) (ftleraseroleevidence.RetainedIntent, error) {
	if !device.configured() {
		return ftleraseroleevidence.RetainedIntent{}, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return ftleraseroleevidence.RetainedIntent{}, ErrEraseDisabled
	}
	if block < 0 || block >= int64(len(device.blocks)) {
		return ftleraseroleevidence.RetainedIntent{}, ErrInvalidBlockIndex
	}
	status := device.table.Snapshot().PhysicalStatus
	first := block * device.pagesPerBlock
	staleCount := 0
	for page := first; page < first+device.pagesPerBlock; page++ {
		role := status[int(page)]
		if role == ftlpagemap.Stale {
			staleCount++
		} else if role != ftlpagemap.Free {
			return ftleraseroleevidence.RetainedIntent{}, ErrIneligibleBlock
		}
	}
	if staleCount == 0 {
		return ftleraseroleevidence.RetainedIntent{}, ErrIneligibleBlock
	}
	progress := device.blocks[int(block)]
	if progress.IncompleteErase {
		return ftleraseroleevidence.RetainedIntent{}, ErrEraseIntentBlockIncomplete
	}
	if progress.CompletedErases >= device.maxCompletedErases {
		return ftleraseroleevidence.RetainedIntent{}, nandblockerasecuts.ErrEraseLimit
	}
	if sequence == 0 {
		return ftleraseroleevidence.RetainedIntent{}, ErrInvalidEraseIntentSequence
	}
	intent := ftleraseroleevidence.RetainedIntent{
		Sequence: sequence, Block: block, CompletedErases: progress.CompletedErases,
		Stale: make([]bool, int(device.pagesPerBlock)),
	}
	for offset := range intent.Stale {
		intent.Stale[offset] = status[int(first)+offset] == ftlpagemap.Stale
	}
	return intent, nil
}

func sameCaptureIntent(left, right ftleraseroleevidence.RetainedIntent) bool {
	return left.Sequence == right.Sequence && left.Block == right.Block &&
		left.CompletedErases == right.CompletedErases && reflect.DeepEqual(left.Stale, right.Stale)
}

func sameCaptureError(left, right error) bool {
	if left == nil || right == nil {
		return left == right
	}
	return errors.Is(left, right) && errors.Is(right, left)
}

func boolRoles(roles []ftlpagemap.PhysicalStatus) []bool {
	result := make([]bool, len(roles))
	for index, role := range roles {
		result[index] = role == ftlpagemap.Stale
	}
	return result
}

func captureSeed(raw []byte) int64 {
	var seed uint64 = 1469598103934665603
	for _, value := range raw {
		seed = (seed ^ uint64(value)) * 1099511628211
	}
	return int64(seed)
}
