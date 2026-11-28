package ftlnandrelocation

import (
	"bytes"
	"errors"
	"fmt"
	"math"
	"math/rand"
	"sync"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

type observedOwnerRecord struct {
	observation OwnerObservation
	symbol      uint64
}

func assertObservationModel(
	t *testing.T,
	device Device,
	model referenceState,
	symbols []uint64,
	records []observedOwnerRecord,
) []observedOwnerRecord {
	t.Helper()
	for logical, physical := range model.mapping {
		got, present, err := device.ObserveOwner(int64(logical))
		if err != nil {
			t.Fatalf("ObserveOwner(%d) error=%v", logical, err)
		}
		if physical < 0 {
			if got != (OwnerObservation{}) || present {
				t.Fatalf("absent ObserveOwner(%d)=%+v/%t, want zero/false", logical, got, present)
			}
			continue
		}
		want := PageOwner{Logical: int64(logical), Physical: physical}
		if !present || got.Owner != want || symbols[logical] == 0 {
			t.Fatalf("ObserveOwner(%d)=%+v/%t, want %+v with a fresh model symbol", logical, got, present, want)
		}
		records = append(records, observedOwnerRecord{observation: got, symbol: symbols[logical]})
	}
	for _, record := range records {
		logical := int(record.observation.Owner.Logical)
		want := logical >= 0 && logical < len(model.mapping) &&
			model.mapping[logical] == record.observation.Owner.Physical && symbols[logical] == record.symbol
		if got := device.MatchesOwner(record.observation); got != want {
			t.Fatalf("MatchesOwner(%+v)=%t, want %t (model symbol %d/current %d)", record.observation.Owner, got, want, record.symbol, symbols[logical])
		}
	}
	return records
}

func TestObserveOwnerMatchesLookupAndSignedValidation(t *testing.T) {
	var zero Device
	if observation, present, err := zero.ObserveOwner(math.MinInt64); observation != (OwnerObservation{}) || present || !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero-device ObserveOwner=%+v/%t/%v", observation, present, err)
	}
	if zero.MatchesOwner(OwnerObservation{}) {
		t.Fatal("zero observation matched an unconfigured device")
	}
	device, err := New(1, 4, 1)
	if err != nil {
		t.Fatal(err)
	}
	if observation, present, err := device.ObserveOwner(0); observation != (OwnerObservation{}) || present || err != nil {
		t.Fatalf("absent ObserveOwner=%+v/%t/%v, want zero/false/nil", observation, present, err)
	}
	for _, logical := range []int64{-1, math.MinInt64, 1, math.MaxInt64} {
		if observation, present, err := device.ObserveOwner(logical); observation != (OwnerObservation{}) || present || err != ftlpagemap.ErrInvalidLogicalPage {
			t.Errorf("ObserveOwner(%d)=%+v/%t/%v; want zero/false/invalid logical", logical, observation, present, err)
		}
	}
}

func TestOwnerPublicationIdentityChangesOnlyAtMediaPublication(t *testing.T) {
	base, err := New(1, 6, 2)
	if err != nil {
		t.Fatal(err)
	}
	base, _, err = base.Write(0, []byte{0x31, 0x32}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	original, present, err := base.ObserveOwner(0)
	if err != nil || !present || original.Owner != (PageOwner{Logical: 0, Physical: 0}) || !base.MatchesOwner(original) {
		t.Fatalf("initial owner observation=%+v/%t/%v", original.Owner, present, err)
	}

	cuts := []Cut{
		{Kind: nandpageprogramcuts.BeforeAccept},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1},
		{Kind: nandpageprogramcuts.AfterMedia},
		{Kind: nandpageprogramcuts.Complete},
	}
	for _, cut := range cuts {
		t.Run(fmt.Sprintf("kind_%d_prefix_%d", cut.Kind, cut.PrefixBytes), func(t *testing.T) {
			next, receipt, err := base.Write(0, []byte{0x31, 0x32}, cut)
			if err != nil {
				t.Fatal(err)
			}
			published := cut.Kind == nandpageprogramcuts.AfterMedia || cut.Kind == nandpageprogramcuts.Complete
			if receipt.Published != published {
				t.Fatalf("receipt %+v publication=%t", receipt, published)
			}
			if base.MatchesOwner(original) != true {
				t.Fatal("write changed retained predecessor identity")
			}
			if next.MatchesOwner(original) == published {
				t.Fatalf("old identity match=%t after cut %+v", next.MatchesOwner(original), cut)
			}
			if published {
				current, ok, observeErr := next.ObserveOwner(0)
				if observeErr != nil || !ok || current.Owner.Physical != receipt.Physical || !next.MatchesOwner(current) {
					t.Fatalf("published owner=%+v/%t/%v receipt=%+v", current.Owner, ok, observeErr, receipt)
				}
			}
		})
	}
	refused, receipt, err := base.Write(0, []byte{0x01}, Cut{Kind: nandpageprogramcuts.Complete})
	if !errors.Is(err, nandpageprogramcuts.ErrWrongPayloadLength) || receipt != (Receipt{}) || !refused.MatchesOwner(original) {
		t.Fatalf("refused write changed owner identity: receipt=%+v error=%v", receipt, err)
	}
	modified := original
	modified.Owner.Physical++
	if base.MatchesOwner(modified) {
		t.Fatal("modified physical coordinate matched current owner")
	}
	modified = original
	modified.Owner.Logical++
	if base.MatchesOwner(modified) {
		t.Fatal("modified logical coordinate matched current owner")
	}
}

func TestSiblingPublicationsUseFreshIdentityAndPreserveCommonAncestors(t *testing.T) {
	base, err := New(2, 6, 1)
	if err != nil {
		t.Fatal(err)
	}
	base, _, err = base.Write(0, []byte{0x19}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	ancestor, _, err := base.ObserveOwner(0)
	if err != nil {
		t.Fatal(err)
	}
	branchLeft, _, err := base.Write(1, []byte{0x21}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	branchRight, _, err := base.Write(1, []byte{0x21}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	if !branchLeft.MatchesOwner(ancestor) || !branchRight.MatchesOwner(ancestor) {
		t.Fatal("common ancestor owner identity did not remain current on both branches")
	}

	left, _, err := base.Write(0, []byte{0x55}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	right, _, err := base.Write(0, []byte{0x55}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	leftOwner, leftPresent, err := left.ObserveOwner(0)
	if err != nil || !leftPresent {
		t.Fatalf("left branch observation=%+v/%t/%v", leftOwner.Owner, leftPresent, err)
	}
	rightOwner, rightPresent, err := right.ObserveOwner(0)
	if err != nil || !rightPresent {
		t.Fatalf("right branch observation=%+v/%t/%v", rightOwner.Owner, rightPresent, err)
	}
	if leftOwner.Owner != rightOwner.Owner || !bytes.Equal(left.Snapshot().Pages[1].Bytes, right.Snapshot().Pages[1].Bytes) {
		t.Fatal("sibling publications should have equal coordinates and bytes")
	}
	if !left.MatchesOwner(leftOwner) || !right.MatchesOwner(rightOwner) || left.MatchesOwner(rightOwner) || right.MatchesOwner(leftOwner) {
		t.Fatal("independent sibling publications shared a current-owner identity")
	}
	differentBytes, _, err := base.Write(0, []byte{0x56}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	differentOwner, present, err := differentBytes.ObserveOwner(0)
	if err != nil || !present || differentOwner.Owner != leftOwner.Owner || bytes.Equal(differentBytes.Snapshot().Pages[1].Bytes, left.Snapshot().Pages[1].Bytes) || left.MatchesOwner(differentOwner) {
		t.Fatalf("different-byte sibling publication=%+v/%t/%v did not retain independent identity", differentOwner.Owner, present, err)
	}
	independent, err := New(2, 6, 1)
	if err != nil {
		t.Fatal(err)
	}
	independent, _, err = independent.Write(0, []byte{0x01}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	independent, _, err = independent.Write(0, []byte{0x55}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	independentOwner, _, err := independent.ObserveOwner(0)
	if err != nil || independentOwner.Owner != leftOwner.Owner || left.MatchesOwner(independentOwner) {
		t.Fatalf("independent equal-coordinate owner=%+v/%v matched sibling", independentOwner.Owner, err)
	}
}

func TestOwnerIdentitySurvivesEraseAndSameCoordinateReuse(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 4, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0x7a}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	first, _, _ := device.ObserveOwner(0)
	device, _, err = device.Write(0, []byte{0x7a}, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if err != nil || !device.MatchesOwner(first) {
		t.Fatalf("aborted replacement changed owner: %v", err)
	}
	device, _, err = device.Write(0, []byte{0x7a}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	middle, _, _ := device.ObserveOwner(0)
	if device.MatchesOwner(first) || !device.MatchesOwner(middle) {
		t.Fatal("successful equal-byte replacement did not establish a fresh identity")
	}
	device, _, err = device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if err != nil || !device.MatchesOwner(middle) {
		t.Fatalf("interrupted unrelated erase changed owner identity: %v", err)
	}
	device, _, err = device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil || !device.MatchesOwner(middle) {
		t.Fatalf("completed unrelated erase changed owner identity: %v", err)
	}
	device, receipt, err := device.Write(0, []byte{0x7a}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || receipt.Physical != 0 {
		t.Fatalf("write after released block=%+v/%v; want physical zero", receipt, err)
	}
	last, present, err := device.ObserveOwner(0)
	if err != nil || !present || last.Owner != first.Owner || !bytes.Equal(device.Snapshot().Pages[0].Bytes, []byte{0x7a}) {
		t.Fatalf("reused owner=%+v/%t/%v; want original coordinates and bytes", last.Owner, present, err)
	}
	if device.MatchesOwner(first) || device.MatchesOwner(middle) || !device.MatchesOwner(last) {
		t.Fatal("erase/reuse failed to distinguish same-coordinate owner publications")
	}
}

func TestCopyOwnerIdentityFollowsPublicationNotReceipt(t *testing.T) {
	base, _, plan := copyTestDevice(t)
	move := plan.Copies[0]
	source, _, err := base.ObserveOwner(move.Logical)
	if err != nil {
		t.Fatal(err)
	}
	other, _, err := base.ObserveOwner(1)
	if err != nil {
		t.Fatal(err)
	}
	partial, _, err := base.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if err != nil || !partial.MatchesOwner(source) || !partial.MatchesOwner(other) {
		t.Fatalf("partial copy changed owner identities: %v", err)
	}
	published, receipt, err := base.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil || !receipt.Published || receipt.Acknowledged {
		t.Fatalf("AfterMedia copy receipt=%+v error=%v", receipt, err)
	}
	if published.MatchesOwner(source) || !published.MatchesOwner(other) {
		t.Fatal("copy publication did not replace only the moved owner's identity")
	}
	destination, present, err := published.ObserveOwner(move.Logical)
	if err != nil || !present || destination.Owner.Physical != move.DestinationPhysical || !published.MatchesOwner(destination) {
		t.Fatalf("published copy destination=%+v/%t/%v", destination.Owner, present, err)
	}
	siblingCopy, _, err := base.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil {
		t.Fatal(err)
	}
	siblingOwner, present, err := siblingCopy.ObserveOwner(move.Logical)
	if err != nil || !present || siblingOwner.Owner != destination.Owner || published.MatchesOwner(siblingOwner) || siblingCopy.MatchesOwner(destination) {
		t.Fatalf("sibling copy publication=%+v/%t/%v shared an owner identity", siblingOwner.Owner, present, err)
	}
}

func TestObserveOwnerIsSafeForConcurrentReaders(t *testing.T) {
	device, err := New(2, 4, 1)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0xa5}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	observation, _, _ := device.ObserveOwner(0)
	var workers sync.WaitGroup
	for worker := 0; worker < 16; worker++ {
		workers.Add(1)
		go func() {
			defer workers.Done()
			for iteration := 0; iteration < 128; iteration++ {
				got, present, observeErr := device.ObserveOwner(0)
				if observeErr != nil || !present || got != observation || !device.MatchesOwner(observation) {
					t.Errorf("concurrent observation=%+v/%t/%v", got.Owner, present, observeErr)
					return
				}
			}
		}()
	}
	workers.Wait()
}

func TestBoundedBranchHistoriesUseFreshPublicationSymbols(t *testing.T) {
	initial, err := NewWithEraseBlocks(2, 4, 2, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	initialModel := referenceInitialWithEraseBlocks(2, 4, 2, 2, 2)
	commands := []struct {
		logical int64
		bytes   []byte
		cut     modelCut
	}{
		{0, []byte{0x11, 0x22}, modelCut{kind: cutBeforeAccept}},
		{0, []byte{0x11, 0x22}, modelCut{kind: cutAfterPrefix, prefix: 0}},
		{0, []byte{0x11, 0x22}, modelCut{kind: cutAfterPrefix, prefix: 1}},
		{0, []byte{0x11, 0x22}, modelCut{kind: cutAfterMedia}},
		{0, []byte{0x11, 0x22}, modelCut{kind: cutComplete}},
		{1, []byte{0x33, 0x44}, modelCut{kind: cutBeforeAccept}},
		{1, []byte{0x33, 0x44}, modelCut{kind: cutAfterPrefix, prefix: 0}},
		{1, []byte{0x33, 0x44}, modelCut{kind: cutAfterPrefix, prefix: 1}},
		{1, []byte{0x33, 0x44}, modelCut{kind: cutAfterMedia}},
		{1, []byte{0x33, 0x44}, modelCut{kind: cutComplete}},
	}
	fresh := uint64(0)
	visits, checks := 0, 0
	var visit func(Device, referenceState, []uint64, []observedOwnerRecord, int)
	visit = func(device Device, model referenceState, symbols []uint64, records []observedOwnerRecord, depth int) {
		visits++
		records = assertObservationModel(t, device, model, symbols, records)
		checks += len(model.mapping)
		if depth == 3 {
			return
		}
		for _, command := range commands {
			_, expectedReceipt, expectedErr := referenceWrite(model, command.logical, command.bytes, command.cut)
			next, nextModel := compareTransition(t, device, model, command.logical, command.bytes, command.cut)
			nextSymbols := append([]uint64(nil), symbols...)
			if expectedErr == "" && expectedReceipt.published {
				fresh++
				nextSymbols[int(command.logical)] = fresh
			}
			visit(next, nextModel, nextSymbols, append([]observedOwnerRecord(nil), records...), depth+1)
		}
	}
	visit(initial, initialModel, []uint64{0, 0}, nil, 0)
	if visits != 1+10+100+1000 || checks != 2*visits || fresh == 0 {
		t.Fatalf("fresh-symbol graph visits=%d owner checks=%d publications=%d", visits, checks, fresh)
	}
	t.Logf("fresh-symbol branches: %d nodes, %d owner checks, %d publications", visits, checks, fresh)
}

func TestSeededOwnerObservationHistories(t *testing.T) {
	const seed int64 = 0x4f574e4552
	rng := rand.New(rand.NewSource(seed))
	fresh := uint64(0)
	for history := 0; history < 256; history++ {
		device, err := NewWithEraseBlocks(2, 6, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 2)
		symbols := []uint64{0, 0}
		var records []observedOwnerRecord
		for step := 0; step < 32; step++ {
			logical := int64(rng.Intn(2))
			payload := []byte{byte(rng.Intn(256)), byte(rng.Intn(256))}
			cut := symbolicCut(byte(rng.Intn(5)))
			_, expectedReceipt, expectedErr := referenceWrite(model, logical, payload, cut)
			device, model = compareTransition(t, device, model, logical, payload, cut)
			if expectedErr == "" && expectedReceipt.published {
				fresh++
				symbols[int(logical)] = fresh
			}
			records = assertObservationModel(t, device, model, symbols, records)
		}
	}
	t.Logf("seed 0x%x: 256 x 32 writes, fresh publications=%d", seed, fresh)
}

func FuzzOwnerObservationHistories(f *testing.F) {
	f.Add([]byte{0, 0x31, 0x32, 4, 1, 0x41, 0x42, 2, 0, 0x31, 0x32, 3})
	f.Add([]byte{1, 0xff, 0xff, 0, 1, 0x20, 0x21, 1})
	f.Add([]byte{})
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 128 {
			t.Skip()
		}
		device, err := NewWithEraseBlocks(2, 6, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 2)
		symbols := []uint64{0, 0}
		var records []observedOwnerRecord
		var fresh uint64
		for offset := 0; offset+3 < len(data); offset += 4 {
			logical := int64(data[offset] % 2)
			payload := []byte{data[offset+1], data[offset+2]}
			cut := symbolicCut(data[offset+3])
			_, expectedReceipt, expectedErr := referenceWrite(model, logical, payload, cut)
			device, model = compareTransition(t, device, model, logical, payload, cut)
			if expectedErr == "" && expectedReceipt.published {
				fresh++
				symbols[int(logical)] = fresh
			}
			records = assertObservationModel(t, device, model, symbols, records)
		}
	})
}

func symbolicCut(value byte) modelCut {
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
