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

func referenceLivePagesInBlock(state referenceState, block int64) (int64, string) {
	if state.logicalCount < 1 || state.logicalCount > 64 ||
		state.physicalCount < 2 || state.physicalCount > 128 ||
		state.physicalCount <= state.logicalCount || state.pageSize < 1 || state.pageSize > maxPageSize {
		return 0, "invalid_geometry"
	}
	if state.pagesPerBlock == 0 {
		return 0, "erase_disabled"
	}
	if block < 0 || block >= int64(len(state.blocks)) {
		return 0, "invalid_block_index"
	}
	first := block * state.pagesPerBlock
	var count int64
	for physical := first; physical < first+state.pagesPerBlock; physical++ {
		if state.status[int(physical)] == modelLive {
			count++
		}
	}
	return count, ""
}

func compareLivePagesInBlock(t *testing.T, device Device, state referenceState, block int64) int64 {
	t.Helper()
	beforeMedia := device.Snapshot()
	beforeBlocks := device.ReclamationSnapshot()
	want, wantError := referenceLivePagesInBlock(state, block)
	got, err := device.LivePagesInBlock(block)
	assertError(t, err, wantError)
	if wantError != "" {
		if got != 0 {
			t.Fatalf("refused live-page query returned count %d, want zero", got)
		}
	} else if got != want {
		t.Fatalf("LivePagesInBlock(%d) = %d, want %d", block, got, want)
	}
	if !reflect.DeepEqual(device.Snapshot(), beforeMedia) || !reflect.DeepEqual(device.ReclamationSnapshot(), beforeBlocks) {
		t.Fatal("read-only live-page observation changed device media or block state")
	}
	return got
}

func TestLivePagesInBlockValidationOrderAndSignedBoundaries(t *testing.T) {
	var zero Device
	if got, err := zero.LivePagesInBlock(math.MinInt64); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || got != 0 {
		t.Fatalf("unconfigured precedence: got=%d err=%v", got, err)
	}
	legacy, err := New(1, 4, 1)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := legacy.LivePagesInBlock(math.MinInt64); !errors.Is(err, ErrEraseDisabled) || got != 0 {
		t.Fatalf("erase profile must precede address: got=%d err=%v", got, err)
	}
	device, err := NewWithEraseBlocks(1, 4, 1, 2, 0)
	if err != nil {
		t.Fatal(err)
	}
	for _, block := range []int64{math.MinInt64, -1, math.MaxInt64} {
		if got, err := device.LivePagesInBlock(block); !errors.Is(err, ErrInvalidBlockIndex) || got != 0 {
			t.Fatalf("invalid signed block %d: got=%d err=%v", block, got, err)
		}
	}
	var upperCount int64
	var upperErr error
	var upperPanic any
	func() {
		defer func() { upperPanic = recover() }()
		upperCount, upperErr = device.LivePagesInBlock(2)
	}()
	if upperPanic != nil || !errors.Is(upperErr, ErrInvalidBlockIndex) || upperCount != 0 {
		t.Fatalf("upper boundary refusal: count=%d error=%v panic=%v", upperCount, upperErr, upperPanic)
	}
	device.pagesPerBlock = -2
	if got, err := device.LivePagesInBlock(0); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || got != 0 {
		t.Fatalf("malformed erase profile must be invalid geometry: got=%d err=%v", got, err)
	}
	zeroBudget, err := NewWithEraseBlocks(1, 4, 1, 2, 0)
	if err != nil {
		t.Fatal(err)
	}
	zeroBudgetModel := referenceInitialWithEraseBlocks(1, 4, 1, 2, 0)
	zeroBudget, zeroBudgetModel = compareTransition(t, zeroBudget, zeroBudgetModel, 0, []byte{0x77}, modelCut{kind: cutComplete})
	if got := compareLivePagesInBlock(t, zeroBudget, zeroBudgetModel, 0); got != 1 {
		t.Fatalf("zero-budget live block count=%d, want 1", got)
	}
}

func TestLivePagesInBlockCountsOnlyCurrentMapOwners(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 4, 1, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 4, 1, 2, 1)
	compareLivePagesInBlock(t, device, model, 0)
	compareLivePagesInBlock(t, device, model, 1)

	device, model = compareTransition(t, device, model, 0, []byte{0xff}, modelCut{kind: cutComplete})
	if got := compareLivePagesInBlock(t, device, model, 0); got != 1 {
		t.Fatalf("all-ones published page should count as one live page, got %d", got)
	}
	device, model = compareTransition(t, device, model, 0, []byte{0x00}, modelCut{kind: cutComplete})
	if got := compareLivePagesInBlock(t, device, model, 0); got != 1 {
		t.Fatalf("stale predecessor must not add occupancy, got %d", got)
	}
	if got := compareLivePagesInBlock(t, device, model, 1); got != 0 {
		t.Fatalf("empty sibling block count=%d", got)
	}

	// An active source remains observable, and the query does not join or
	// advance the evacuation. The map still owns exactly the latest page.
	device, model = compareEvacuationBegin(t, device, model, 0)
	if got := compareLivePagesInBlock(t, device, model, 0); got != 1 {
		t.Fatalf("active source live count=%d, want 1", got)
	}
	assertRead(t, device, model, 0)
	if !device.evacuationActive {
		t.Fatal("live-page observation changed evacuation state")
	}

	full, err := NewWithEraseBlocks(2, 4, 1, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	fullModel := referenceInitialWithEraseBlocks(2, 4, 1, 2, 1)
	full, fullModel = compareTransition(t, full, fullModel, 0, []byte{0x10}, modelCut{kind: cutComplete})
	full, fullModel = compareTransition(t, full, fullModel, 1, []byte{0x20}, modelCut{kind: cutComplete})
	if got := compareLivePagesInBlock(t, full, fullModel, 0); got != 2 {
		t.Fatalf("full block count=%d, want 2", got)
	}
}

func TestLivePagesInBlockCountsIncompleteAndExhaustedExtents(t *testing.T) {
	incomplete, model := prepareStaleFreeBlock(t)
	incomplete, model = compareEraseBlock(t, incomplete, model, 0, EraseCut{
		Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1,
	})
	if !incomplete.blocks[0].IncompleteErase {
		t.Fatal("test did not establish a partial block erase")
	}
	if got := compareLivePagesInBlock(t, incomplete, model, 0); got != 0 {
		t.Fatalf("incomplete block live count=%d", got)
	}

	exhausted, err := NewWithEraseBlocks(1, 4, 1, 2, 1)
	if err != nil {
		t.Fatal(err)
	}
	exhaustedModel := referenceInitialWithEraseBlocks(1, 4, 1, 2, 1)
	exhausted, exhaustedModel = compareTransition(t, exhausted, exhaustedModel, 0, []byte{0x31}, modelCut{kind: cutComplete})
	exhausted, exhaustedModel = compareTransition(t, exhausted, exhaustedModel, 0, []byte{0x32}, modelCut{kind: cutComplete})
	exhausted, exhaustedModel = compareTransition(t, exhausted, exhaustedModel, 0, []byte{0x33}, modelCut{kind: cutComplete})
	exhausted, exhaustedModel = compareEraseBlock(t, exhausted, exhaustedModel, 0, EraseCut{Kind: nandblockerasecuts.Complete})
	// Exhausting the erase budget does not fence host writes or erase the
	// semantic distinction between an empty block and its next live page.
	exhausted, exhaustedModel = compareTransition(t, exhausted, exhaustedModel, 0, []byte{0x44}, modelCut{kind: cutComplete})
	if exhausted.blocks[0].CompletedErases != exhausted.maxCompletedErases {
		t.Fatal("test did not exhaust the erase ceiling")
	}
	if got := compareLivePagesInBlock(t, exhausted, exhaustedModel, 0); got != 1 {
		t.Fatalf("exhausted but readable block live count=%d, want 1", got)
	}
	assertRead(t, exhausted, exhaustedModel, 0)
	compareLivePagesInBlock(t, exhausted, exhaustedModel, 1)
}

func TestLivePagesInBlockExhaustiveClosedDeviceGraphs(t *testing.T) {
	queries := 0
	for _, geometry := range []struct{ logical, physical, ppb int64 }{{1, 4, 2}, {2, 4, 2}} {
		device, err := NewWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.ppb, 1)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.ppb, 1)
		queue := []struct {
			device Device
			model  referenceState
		}{{device, model}}
		seen := make(map[string]bool)
		for len(queue) > 0 {
			node := queue[0]
			queue = queue[1:]
			key := evacuationStateKey(node.model)
			if seen[key] {
				continue
			}
			seen[key] = true
			for block := int64(-1); block <= int64(len(node.model.blocks)); block++ {
				compareLivePagesInBlock(t, node.device, node.model, block)
				queries++
			}
			for _, next := range evacuationSuccessors(t, node.device, node.model) {
				nextKey := evacuationStateKey(next.model)
				if !seen[nextKey] {
					queue = append(queue, next)
				}
			}
		}
		t.Logf("live-page observation checked %d states for %d/%d geometry", len(seen), geometry.logical, geometry.physical)
	}
	if queries == 0 {
		t.Fatal("closed-state live-page checks executed no queries")
	}
}

func TestLivePagesInBlockSeededHistories(t *testing.T) {
	rng := rand.New(rand.NewSource(0x5d02b2c1))
	queryCount := 0
	for caseIndex := 0; caseIndex < 128; caseIndex++ {
		device, err := NewWithEraseBlocks(2, 8, 1, 2, 1)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 8, 1, 2, 1)
		for command := 0; command < 64; command++ {
			logical := int64(rng.Intn(2))
			value := byte(rng.Intn(256))
			cut := []modelCut{{kind: cutComplete}, {kind: cutAfterPrefix, prefix: 0}, {kind: cutAfterMedia}, {kind: cutBeforeAccept}}[rng.Intn(4)]
			device, model = compareTransition(t, device, model, logical, []byte{value}, cut)
			block := int64(rng.Intn(len(model.blocks)))
			compareLivePagesInBlock(t, device, model, block)
			queryCount++
		}
	}
	if queryCount != 8192 {
		t.Fatalf("seeded live-page query count=%d, want 8192", queryCount)
	}
}

func FuzzLivePagesInBlockHistory(f *testing.F) {
	f.Add([]byte{0, 0, 0xff, 0, 1, 0, 0x31, 1, 2, 0, 0, 2, 3, 1, 0x42, 2, 4, 0, 0, 0xff})
	f.Add([]byte{1, 0, 0x01, 0, 4, 0, 0x02, 1, 2, 1, 0, 0, 3, 0, 0x00, 3})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			return
		}
		device, err := NewWithEraseBlocks(2, 8, 1, 2, 1)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 8, 1, 2, 1)
		for offset := 0; offset+3 < len(commands); offset += 4 {
			selector, address, value, cutByte := commands[offset], commands[offset+1], commands[offset+2], commands[offset+3]
			switch selector % 5 {
			case 0:
				device, model = compareTransition(t, device, model, int64(address%2), []byte{value}, modelCut{kind: int(cutByte % 4), prefix: 0})
			case 1:
				device, model = compareEvacuationBegin(t, device, model, int64(address%4))
			case 2:
				device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.CutKind(cutByte % 4)})
			case 3:
				device, model = compareEvacuationCancel(t, device, model)
			case 4:
				device, model = compareEraseBlock(t, device, model, int64(address%4), EraseCut{
					Kind: nandblockerasecuts.CutKind(cutByte % 4), PrefixPages: int(value % 2),
				})
			}
			block := int64(address % 4)
			compareLivePagesInBlock(t, device, model, block)
		}
	})
}
