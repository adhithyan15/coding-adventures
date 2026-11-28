package nandblockerasecuts

import (
	"bytes"
	"errors"
	"math/rand"
	"reflect"
	"sync"
	"testing"
)

func TestAllCutsMatchDeclarativePageAndCounterModel(t *testing.T) {
	combinations := 0
	for pageCount := 1; pageCount <= 4; pageCount++ {
		before := patternedBlock(pageCount, 3)
		cuts := []Cut{{Kind: BeforeAccept}, {Kind: AfterMedia}, {Kind: Complete}}
		for prefix := 0; prefix < pageCount; prefix++ {
			cuts = append(cuts, Cut{Kind: AfterPrefix, PrefixPages: prefix})
		}
		for _, cut := range cuts {
			want, wantAck, wantErr := referenceApply(3, 4, before, cut)
			got, err := Apply(3, 4, before, cut)
			assertTransition(t, got, err, want, wantAck, wantErr)
			combinations++
		}
	}
	if combinations != 22 {
		t.Fatalf("checked %d cut combinations, want 22", combinations)
	}
}

func TestAfterPrefixPreservesSuffixPagesAndMarkers(t *testing.T) {
	before := patternedBlock(3, 2)
	before.Pages[1].Programmed = true
	got, err := Apply(2, 3, before, Cut{Kind: AfterPrefix, PrefixPages: 1})
	if err != nil {
		t.Fatalf("Apply partial erase: %v", err)
	}
	if got.Block.Pages[1].Programmed != before.Pages[1].Programmed {
		t.Fatal("after-prefix preserves suffix page markers")
	}
	if !bytes.Equal(got.Block.Pages[1].Bytes, before.Pages[1].Bytes) || !bytes.Equal(got.Block.Pages[2].Bytes, before.Pages[2].Bytes) {
		t.Fatal("after-prefix preserves suffix page bytes")
	}
	if got.Block.Pages[0].Programmed || !allBytes(got.Block.Pages[0].Bytes, 0xff) {
		t.Fatal("after-prefix erases and clears every completed prefix page")
	}
}

func TestPartialEraseKeepsCompletedEraseCount(t *testing.T) {
	before := patternedBlock(3, 2)
	before.CompletedErases = 2
	got, err := Apply(2, 4, before, Cut{Kind: AfterPrefix, PrefixPages: 2})
	if err != nil {
		t.Fatalf("Apply partial erase: %v", err)
	}
	if got.Block.CompletedErases != before.CompletedErases {
		t.Fatalf("partial erase changed completed count: got %d, want %d", got.Block.CompletedErases, before.CompletedErases)
	}
	if !got.Block.IncompleteErase || got.Acknowledged {
		t.Fatalf("partial erase outcome = %+v", got)
	}
}

func TestAfterMediaErasesFullyWithoutAcknowledgement(t *testing.T) {
	before := patternedBlock(4, 2)
	before.CompletedErases = 1
	got, err := Apply(2, 3, before, Cut{Kind: AfterMedia})
	if err != nil {
		t.Fatalf("Apply after-media erase: %v", err)
	}
	if got.Acknowledged {
		t.Fatal("after-media has no protocol acknowledgement")
	}
	if got.Block.CompletedErases != 2 || got.Block.IncompleteErase {
		t.Fatalf("after-media count/incomplete state = %d/%t", got.Block.CompletedErases, got.Block.IncompleteErase)
	}
	for index, page := range got.Block.Pages {
		if page.Programmed || !allBytes(page.Bytes, 0xff) {
			t.Fatalf("after-media page %d was not erased", index)
		}
	}
}

func TestZeroPrefixAndIncompleteToCompleteRetry(t *testing.T) {
	before := patternedBlock(3, 2)
	zero, err := Apply(2, 2, before, Cut{Kind: AfterPrefix, PrefixPages: 0})
	if err != nil {
		t.Fatalf("zero-prefix erase: %v", err)
	}
	if !reflect.DeepEqual(zero.Block.Pages, before.Pages) || zero.Block.CompletedErases != before.CompletedErases || !zero.Block.IncompleteErase || zero.Acknowledged {
		t.Fatalf("zero-prefix result = %+v; bytes/markers/count should be preserved and incomplete set", zero)
	}
	completed, err := Apply(2, 2, zero.Block, Cut{Kind: Complete})
	if err != nil {
		t.Fatalf("complete retry: %v", err)
	}
	if !completed.Acknowledged || completed.Block.CompletedErases != before.CompletedErases+1 || completed.Block.IncompleteErase {
		t.Fatalf("completed retry result = %+v", completed)
	}
	for index, page := range completed.Block.Pages {
		if page.Programmed || !allBytes(page.Bytes, 0xff) {
			t.Fatalf("retry did not erase suffix page %d", index)
		}
	}
}

func TestBeforeAcceptPreservesPriorIncompleteStateAndCompleteCanConsumeErasedCycle(t *testing.T) {
	incomplete := patternedBlock(2, 1)
	incomplete.IncompleteErase = true
	refused, err := Apply(1, 2, incomplete, Cut{Kind: BeforeAccept})
	if err != nil || !reflect.DeepEqual(refused.Block, incomplete) || refused.Acknowledged {
		t.Fatalf("before-accept should preserve prior incomplete state: %+v, %v", refused, err)
	}

	erased := Block{
		Pages: []Page{{Bytes: []byte{0xff}, Programmed: false}},
	}
	first, err := Apply(1, 1, erased, Cut{Kind: Complete})
	if err != nil || first.Block.CompletedErases != 1 || !first.Acknowledged {
		t.Fatalf("complete erase of already-erased state = %+v, %v", first, err)
	}
	second, err := Apply(1, 1, first.Block, Cut{Kind: Complete})
	assertRefusal(t, second, err, ErrEraseLimit)
}

func TestValidationPrecedenceZeroRefusalsAndBlockInvariants(t *testing.T) {
	valid := Block{Pages: []Page{{Bytes: []byte{0xff}, Programmed: false}}}
	invalidCut := Cut{Kind: CutKind(255), PrefixPages: -1}

	got, err := Apply(0, ^uint64(0), Block{}, invalidCut)
	assertRefusal(t, got, err, ErrInvalidPageSize)

	badErasedMarker := Block{Pages: []Page{{Bytes: []byte{0x00}, Programmed: false}}, CompletedErases: 1}
	got, err = Apply(1, 0, badErasedMarker, invalidCut)
	assertRefusal(t, got, err, ErrInvalidBlock)

	got, err = Apply(1, maxSafeErase+1, valid, invalidCut)
	assertRefusal(t, got, err, ErrInvalidEraseCount)
	tooMany := valid
	tooMany.CompletedErases = 1
	got, err = Apply(1, 0, tooMany, invalidCut)
	assertRefusal(t, got, err, ErrInvalidEraseCount)

	got, err = Apply(1, 0, valid, invalidCut)
	assertRefusal(t, got, err, ErrInvalidCut)
	got, err = Apply(1, 0, valid, Cut{Kind: Complete})
	assertRefusal(t, got, err, ErrEraseLimit)

	for _, block := range []Block{
		{},
		{Pages: make([]Page, maxPagesPerBlock+1)},
		{Pages: []Page{{Bytes: nil}}},
		{Pages: []Page{{Bytes: []byte{0xff, 0xff}}}},
	} {
		got, err = Apply(1, 2, block, Cut{Kind: Complete})
		assertRefusal(t, got, err, ErrInvalidBlock)
	}

	got, err = Apply(1, 2, valid, Cut{Kind: AfterPrefix, PrefixPages: -1})
	assertRefusal(t, got, err, ErrInvalidCut)
	got, err = Apply(1, 2, valid, Cut{Kind: AfterPrefix, PrefixPages: 1})
	assertRefusal(t, got, err, ErrInvalidCut)
	got, err = Apply(1, 2, valid, Cut{Kind: AfterMedia, PrefixPages: 1})
	assertRefusal(t, got, err, ErrInvalidCut)
}

func TestWidthCountAndPageCountExtrema(t *testing.T) {
	minimum := Block{Pages: []Page{{Bytes: []byte{0xff}}}}
	if got, err := Apply(1, 1, minimum, Cut{Kind: Complete}); err != nil || !got.Acknowledged {
		t.Fatalf("minimum page width: got %+v, err %v", got, err)
	}

	maximumWidth := Block{Pages: []Page{{Bytes: erasedBytes(maxPageSize)}}}
	if got, err := Apply(maxPageSize, 1, maximumWidth, Cut{Kind: AfterPrefix, PrefixPages: 0}); err != nil || len(got.Block.Pages[0].Bytes) != maxPageSize {
		t.Fatalf("maximum page width: got %d bytes, err %v", len(got.Block.Pages[0].Bytes), err)
	}

	maximumPages := Block{Pages: make([]Page, maxPagesPerBlock)}
	for index := range maximumPages.Pages {
		maximumPages.Pages[index].Bytes = []byte{0xff}
	}
	if got, err := Apply(1, 1, maximumPages, Cut{Kind: AfterPrefix, PrefixPages: maxPagesPerBlock - 1}); err != nil || len(got.Block.Pages) != maxPagesPerBlock {
		t.Fatalf("maximum page count: got %d pages, err %v", len(got.Block.Pages), err)
	}

	lastPermitted := Block{Pages: []Page{{Bytes: []byte{0xff}}}, CompletedErases: maxSafeErase - 1}
	final, err := Apply(1, maxSafeErase, lastPermitted, Cut{Kind: Complete})
	if err != nil || final.Block.CompletedErases != maxSafeErase {
		t.Fatalf("final portable erase should be permitted: %+v, %v", final, err)
	}
	refused, err := Apply(1, maxSafeErase, final.Block, Cut{Kind: Complete})
	assertRefusal(t, refused, err, ErrEraseLimit)
}

func TestSuccessfulOutcomesAndRefusalsDoNotAliasCallerPages(t *testing.T) {
	before := patternedBlock(3, 4)
	copyOfInput := cloneBlock(before)
	result, err := Apply(4, 2, before, Cut{Kind: AfterPrefix, PrefixPages: 1})
	if err != nil {
		t.Fatalf("Apply: %v", err)
	}
	result.Block.Pages[1].Bytes[0] ^= 0xff
	if !reflect.DeepEqual(before, copyOfInput) {
		t.Fatal("mutating the outcome changed caller-owned input")
	}
	before.Pages[2].Bytes[0] ^= 0xff
	if result.Block.Pages[2].Bytes[0] == before.Pages[2].Bytes[0] {
		t.Fatal("mutating caller input changed the detached outcome")
	}

	beforeRefusal := cloneForOracle(before)
	invalid := Cut{Kind: CutKind(255)}
	zero, err := Apply(4, 2, before, invalid)
	assertRefusal(t, zero, err, ErrInvalidCut)
	if !reflect.DeepEqual(before, beforeRefusal) {
		t.Fatal("refused call mutated caller input")
	}
}

func TestShortHistoriesAcrossPartialRepeatsAndFullRetries(t *testing.T) {
	cuts := []Cut{
		{Kind: BeforeAccept},
		{Kind: AfterPrefix, PrefixPages: 0},
		{Kind: AfterPrefix, PrefixPages: 1},
		{Kind: AfterMedia},
		{Kind: Complete},
	}
	histories := 0
	var visit func(Block, []Cut)
	visit = func(state Block, history []Cut) {
		histories++
		if len(history) == 3 {
			return
		}
		for _, cut := range cuts {
			want, wantAck, wantErr := referenceApply(2, 3, state, cut)
			got, err := Apply(2, 3, state, cut)
			assertTransition(t, got, err, want, wantAck, wantErr)
			next := state
			if wantErr == "" {
				next = want
			}
			visit(next, append(history, cut))
		}
	}
	visit(patternedBlock(2, 2), nil)
	if histories != 156 {
		t.Fatalf("enumerated %d prefixes, want 156", histories)
	}
}

func TestFiveThousandSeededTransitionsMatchModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB04D))
	state := randomValidBlock(random, 3)
	ceiling := uint64(1 + random.Intn(4))
	state.CompletedErases = uint64(random.Intn(int(ceiling)))
	for command := 0; command < 5_000; command++ {
		if command%19 == 0 {
			state = randomValidBlock(random, 1+random.Intn(4))
			ceiling = uint64(1 + random.Intn(4))
			state.CompletedErases = uint64(random.Intn(int(ceiling)))
		}
		kind := CutKind(random.Intn(5))
		prefix := random.Intn(len(state.Pages)+2) - 1
		if kind != AfterPrefix && random.Intn(5) == 0 {
			prefix = 1
		}
		cut := Cut{Kind: kind, PrefixPages: prefix}
		before := cloneBlock(state)
		want, wantAck, wantErr := referenceApply(3, ceiling, state, cut)
		got, err := Apply(3, ceiling, state, cut)
		assertTransition(t, got, err, want, wantAck, wantErr)
		if wantErr == "" {
			state = want
		} else if !reflect.DeepEqual(state, before) {
			t.Fatalf("command %d: model state changed while modeling refusal", command)
		}
	}
}

func TestIndependentBlocksCanTransitionConcurrently(t *testing.T) {
	const workers = 12
	var wait sync.WaitGroup
	errorsFound := make(chan error, workers)
	for worker := 0; worker < workers; worker++ {
		wait.Add(1)
		go func(seed int) {
			defer wait.Done()
			block := patternedBlock(4, 2)
			for iteration := 0; iteration < 250; iteration++ {
				cut := Cut{Kind: CutKind((seed + iteration) % 4)}
				if cut.Kind == AfterPrefix {
					cut.PrefixPages = (seed + iteration) % len(block.Pages)
				}
				out, err := Apply(2, 1_000, block, cut)
				if err != nil {
					errorsFound <- err
					return
				}
				block = out.Block
			}
		}(worker)
	}
	wait.Wait()
	close(errorsFound)
	for err := range errorsFound {
		t.Errorf("independent block transition: %v", err)
	}
}

func assertTransition(t *testing.T, got Outcome, gotErr error, want Block, wantAck bool, wantErr string) {
	t.Helper()
	if wantErr != "" {
		assertRefusal(t, got, gotErr, errorForCode(wantErr))
		return
	}
	if gotErr != nil {
		t.Fatalf("Apply returned unexpected error: %v", gotErr)
	}
	if !reflect.DeepEqual(got.Block, want) || got.Acknowledged != wantAck {
		t.Fatalf("Apply = %+v; want block %+v and acknowledged=%t", got, want, wantAck)
	}
}

func assertRefusal(t *testing.T, got Outcome, gotErr, want error) {
	t.Helper()
	if !errors.Is(gotErr, want) {
		t.Fatalf("refusal error = %v; want %v", gotErr, want)
	}
	if !reflect.DeepEqual(got, Outcome{}) {
		t.Fatalf("refusal returned nonzero outcome %+v", got)
	}
}

func errorForCode(code string) error {
	switch code {
	case "invalid_page_size":
		return ErrInvalidPageSize
	case "invalid_block_state":
		return ErrInvalidBlock
	case "invalid_erase_count":
		return ErrInvalidEraseCount
	case "invalid_cut":
		return ErrInvalidCut
	case "erase_limit":
		return ErrEraseLimit
	default:
		return errors.New("unknown oracle code")
	}
}

func patternedBlock(pageCount, pageSize int) Block {
	block := Block{Pages: make([]Page, pageCount), CompletedErases: 1}
	for index := range block.Pages {
		programmed := index%2 == 0
		page := Page{Bytes: make([]byte, pageSize), Programmed: programmed}
		for offset := range page.Bytes {
			page.Bytes[offset] = byte(17 + index*31 + offset*13)
		}
		if !programmed {
			for offset := range page.Bytes {
				page.Bytes[offset] = 0xff
			}
		}
		block.Pages[index] = page
	}
	// All-ones content still needs its independent programmed marker.
	if pageCount > 1 {
		for offset := range block.Pages[0].Bytes {
			block.Pages[0].Bytes[offset] = 0xff
		}
		block.Pages[0].Programmed = true
	}
	return block
}

func randomValidBlock(random *rand.Rand, pageCount int) Block {
	block := Block{Pages: make([]Page, pageCount), IncompleteErase: random.Intn(2) == 0}
	for index := range block.Pages {
		page := Page{Bytes: make([]byte, 3), Programmed: random.Intn(2) == 0}
		if page.Programmed {
			for offset := range page.Bytes {
				page.Bytes[offset] = byte(random.Intn(256))
			}
		} else {
			for offset := range page.Bytes {
				page.Bytes[offset] = 0xff
			}
		}
		block.Pages[index] = page
	}
	return block
}

func erasedBytes(size int) []byte {
	page := make([]byte, size)
	for index := range page {
		page[index] = 0xff
	}
	return page
}

func allBytes(data []byte, value byte) bool {
	for _, actual := range data {
		if actual != value {
			return false
		}
	}
	return true
}

// referenceApply describes the protocol table independently: it validates
// inputs, creates its own page copies, then edits only pages covered by the cut.
func referenceApply(pageSize int, ceiling uint64, block Block, cut Cut) (Block, bool, string) {
	if pageSize < 1 || pageSize > 65_536 {
		return Block{}, false, "invalid_page_size"
	}
	if len(block.Pages) == 0 || len(block.Pages) > 256 {
		return Block{}, false, "invalid_block_state"
	}
	for _, page := range block.Pages {
		if len(page.Bytes) != pageSize {
			return Block{}, false, "invalid_block_state"
		}
		if !page.Programmed {
			for _, b := range page.Bytes {
				if b != 0xff {
					return Block{}, false, "invalid_block_state"
				}
			}
		}
	}
	if ceiling > 9_007_199_254_740_991 || block.CompletedErases > ceiling {
		return Block{}, false, "invalid_erase_count"
	}
	validCut := false
	switch cut.Kind {
	case BeforeAccept, AfterMedia, Complete:
		validCut = cut.PrefixPages == 0
	case AfterPrefix:
		validCut = cut.PrefixPages >= 0 && cut.PrefixPages < len(block.Pages)
	}
	if !validCut {
		return Block{}, false, "invalid_cut"
	}
	if block.CompletedErases >= ceiling {
		return Block{}, false, "erase_limit"
	}

	out := cloneForOracle(block)
	acknowledged := false
	if cut.Kind == AfterPrefix {
		for page := 0; page < cut.PrefixPages; page++ {
			out.Pages[page].Bytes = make([]byte, pageSize)
			for index := range out.Pages[page].Bytes {
				out.Pages[page].Bytes[index] = 0xff
			}
			out.Pages[page].Programmed = false
		}
		out.IncompleteErase = true
	} else if cut.Kind == AfterMedia || cut.Kind == Complete {
		for page := range out.Pages {
			out.Pages[page].Bytes = make([]byte, pageSize)
			for index := range out.Pages[page].Bytes {
				out.Pages[page].Bytes[index] = 0xff
			}
			out.Pages[page].Programmed = false
		}
		out.CompletedErases = block.CompletedErases + 1
		out.IncompleteErase = false
		acknowledged = cut.Kind == Complete
	}
	return out, acknowledged, ""
}

func cloneForOracle(block Block) Block {
	result := Block{
		Pages:           make([]Page, len(block.Pages)),
		CompletedErases: block.CompletedErases,
		IncompleteErase: block.IncompleteErase,
	}
	for index := range block.Pages {
		result.Pages[index] = Page{
			Bytes:      make([]byte, len(block.Pages[index].Bytes)),
			Programmed: block.Pages[index].Programmed,
		}
		copy(result.Pages[index].Bytes, block.Pages[index].Bytes)
	}
	return result
}
