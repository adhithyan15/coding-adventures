package ftlerasevictim

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

func TestCopyAwareSeparateVictimAndDestinationFencesWitness(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Live,
		ftlpagemap.Free, ftlpagemap.Live,
		ftlpagemap.Live, ftlpagemap.Live,
	}
	victimFence := []bool{false, true, false}
	got, found, err := SelectCopyAwareWithFences(status, 2, victimFence, nil)
	want := CopyAwareVictim{Block: 0, FirstPhysical: 0, PageCount: 2, StalePages: 1, LivePages: 1, FreeDestinations: 1}
	if err != nil || !found || got != want {
		t.Fatalf("separate fence witness = %+v/%t/%v; want %+v", got, found, err, want)
	}
	shared, sharedFound, sharedErr := SelectCopyAware(status, 2, victimFence)
	if sharedErr != nil || sharedFound || shared != (CopyAwareVictim{}) {
		t.Fatalf("shared fence should remove the only external spare: %+v/%t/%v", shared, sharedFound, sharedErr)
	}
}

func TestCopyAwareCandidateCannotUseOwnFreeSlotsAsDestinations(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free,
		ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free,
	}
	got, found, err := SelectCopyAwareWithFences(status, 3, nil, []bool{false, true})
	if err != nil || found || got != (CopyAwareVictim{}) {
		t.Fatalf("candidate-local free page counted as external destination: %+v/%t/%v", got, found, err)
	}
}

func TestCopyAwareVictimFenceSkipsAnOtherwiseEligibleBlock(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Free,
		ftlpagemap.Stale, ftlpagemap.Free,
		ftlpagemap.Free, ftlpagemap.Free,
	}
	got, found, err := SelectCopyAwareWithFences(status, 2, []bool{true, false, false}, nil)
	if err != nil || !found || got.Block != 1 {
		t.Fatalf("victim fence should leave block one as candidate: %+v/%t/%v", got, found, err)
	}
}

func TestCopyAwareDestinationFenceRemovesCandidateCapacity(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Live,
		ftlpagemap.Free, ftlpagemap.Live,
		ftlpagemap.Live, ftlpagemap.Live,
	}
	victimFence := []bool{false, true, false}
	destinationFence := []bool{false, true, false}
	got, found, err := SelectCopyAwareWithFences(status, 2, victimFence, destinationFence)
	if err != nil || found || got != (CopyAwareVictim{}) {
		t.Fatalf("excluded destination block still contributed free capacity: %+v/%t/%v", got, found, err)
	}
}

func TestCopyAwareSeparateFenceRefusalPrecedenceAndZeroResults(t *testing.T) {
	unknown := []ftlpagemap.PhysicalStatus{ftlpagemap.Reserved, 255}
	for _, tc := range []struct {
		name        string
		status      []ftlpagemap.PhysicalStatus
		width       int64
		victimFence []bool
		destFence   []bool
		want        error
	}{
		{"geometry before both masks", unknown, 0, []bool{true}, []bool{true}, ErrInvalidGeometry},
		{"victim mask before destination mask and roles", unknown, 1, []bool{true}, []bool{true, false}, ErrInvalidBlockMask},
		{"destination mask before status", unknown, 1, nil, []bool{true}, ErrInvalidBlockMask},
		{"unknown status before pending reservation", unknown, 1, nil, nil, ErrInvalidStatus},
		{"pending reservation after valid masks", []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Reserved}, 1, []bool{true, false}, []bool{false, true}, ErrReservationPending},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got, found, err := SelectCopyAwareWithFences(tc.status, tc.width, tc.victimFence, tc.destFence)
			if !errors.Is(err, tc.want) || found || got != (CopyAwareVictim{}) {
				t.Fatalf("refusal=%+v/%t/%v; want zero/%v", got, found, err, tc.want)
			}
		})
	}
}

func TestCopyAwareSeparateFencesDoNotMutateInputsAndSharedParity(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free}
	victimFence := []bool{false, false}
	destinationFence := []bool{true, false}
	statusBefore := append([]ftlpagemap.PhysicalStatus(nil), status...)
	victimBefore := append([]bool(nil), victimFence...)
	destinationBefore := append([]bool(nil), destinationFence...)
	got, found, err := SelectCopyAwareWithFences(status, 2, victimFence, destinationFence)
	if err != nil || !found || got.FreeDestinations != 2 {
		t.Fatalf("separate fence result=%+v/%t/%v", got, found, err)
	}
	if !reflect.DeepEqual(status, statusBefore) || !reflect.DeepEqual(victimFence, victimBefore) || !reflect.DeepEqual(destinationFence, destinationBefore) {
		t.Fatal("selection mutated status or fence inputs")
	}
	for _, mask := range [][]bool{nil, {}, {false, false}, {true, false}} {
		legacy, legacyFound, legacyErr := SelectCopyAware(status, 2, mask)
		withFences, withFound, withErr := SelectCopyAwareWithFences(status, 2, mask, mask)
		if !sameErrorClass(legacyErr, withErr) || legacyFound != withFound || legacy != withFences {
			t.Fatalf("shared-mask parity mask=%v: legacy=%+v/%t/%v new=%+v/%t/%v", mask, legacy, legacyFound, legacyErr, withFences, withFound, withErr)
		}
	}
}

func TestAllSmallSeparateFenceLayoutsMatchIndependentEnumerator(t *testing.T) {
	checked := 0
	selected := 0
	for pageCount := 2; pageCount <= 5; pageCount++ {
		vectorCount := 1
		for range pageCount {
			vectorCount *= 4
		}
		for encoded := 0; encoded < vectorCount; encoded++ {
			status := make([]ftlpagemap.PhysicalStatus, pageCount)
			remaining := encoded
			for index := range status {
				status[index] = ftlpagemap.PhysicalStatus(remaining % 4)
				remaining /= 4
			}
			for width := 1; width <= pageCount; width++ {
				if pageCount%width != 0 {
					continue
				}
				blockCount := pageCount / width
				maskCount := 1 << blockCount
				for victimBits := 0; victimBits < maskCount; victimBits++ {
					victimFence := fenceFromBits(blockCount, victimBits)
					for destinationBits := 0; destinationBits < maskCount; destinationBits++ {
						destinationFence := fenceFromBits(blockCount, destinationBits)
						statusBefore := append([]ftlpagemap.PhysicalStatus(nil), status...)
						victimBefore := append([]bool(nil), victimFence...)
						destinationBefore := append([]bool(nil), destinationFence...)
						want, wantFound, wantErr := referenceCopyAwareFences(rawStatus(status), int64(width), victimFence, destinationFence)
						got, gotFound, gotErr := SelectCopyAwareWithFences(status, int64(width), victimFence, destinationFence)
						if !sameErrorClass(gotErr, wantErr) || gotFound != wantFound || got != want {
							t.Fatalf("status=%v width=%d victim=%v destination=%v => %+v/%t/%v; model %+v/%t/%v", status, width, victimFence, destinationFence, got, gotFound, gotErr, want, wantFound, wantErr)
						}
						if gotFound {
							selected++
						}
						if !reflect.DeepEqual(status, statusBefore) || !reflect.DeepEqual(victimFence, victimBefore) || !reflect.DeepEqual(destinationFence, destinationBefore) {
							t.Fatal("separate-fence enumeration mutated caller input")
						}
						checked++
					}
				}
			}
		}
	}
	if checked < 1_000_000 || selected == 0 {
		t.Fatalf("bounded separate-fence enumeration too small: %d cases, %d selected", checked, selected)
	}
	t.Logf("separate-fence exhaustive domain: %d role/status cases, %d selected", checked, selected)
}

func TestSeededSeparateFenceHistoriesMatchIndependentEnumerator(t *testing.T) {
	const seed int64 = 0x46454e4345
	rng := rand.New(rand.NewSource(seed))
	for step := 0; step < 12_000; step++ {
		pageCount := 2 + rng.Intn(31)
		status := make([]ftlpagemap.PhysicalStatus, pageCount)
		for index := range status {
			status[index] = ftlpagemap.PhysicalStatus(rng.Intn(4))
		}
		width := 1 + rng.Intn(pageCount)
		if pageCount%width != 0 {
			width = 1
		}
		blockCount := pageCount / width
		victimFence := make([]bool, blockCount)
		destinationFence := make([]bool, blockCount)
		for index := range victimFence {
			victimFence[index] = rng.Intn(3) == 0
			destinationFence[index] = rng.Intn(3) == 0
		}
		want, wantFound, wantErr := referenceCopyAwareFences(rawStatus(status), int64(width), victimFence, destinationFence)
		got, gotFound, gotErr := SelectCopyAwareWithFences(status, int64(width), victimFence, destinationFence)
		if !sameErrorClass(gotErr, wantErr) || gotFound != wantFound || got != want {
			t.Fatalf("seed=%d step=%d status=%v width=%d victim=%v destination=%v got=%+v/%t/%v want=%+v/%t/%v", seed, step, status, width, victimFence, destinationFence, got, gotFound, gotErr, want, wantFound, wantErr)
		}
	}
}

func FuzzCopyAwareFenceStatusVectors(f *testing.F) {
	f.Add([]byte{2, 0, 2, 1, 2, 1, 1, 0, 2, 2, 0, 0})
	f.Add([]byte{3, 1, 2, 0, 1, 1, 0, 2, 2, 0})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) < 2 || len(raw) > maxPhysicalPages+2 {
			return
		}
		width := int64(raw[0])
		status := make([]ftlpagemap.PhysicalStatus, len(raw)-1)
		for index, value := range raw[1:] {
			status[index] = ftlpagemap.PhysicalStatus(value & 7)
		}
		if width < 1 || width > int64(len(status)) || int64(len(status))%width != 0 {
			got, found, err := SelectCopyAwareWithFences(status, width, nil, nil)
			if !errors.Is(err, ErrInvalidGeometry) || found || got != (CopyAwareVictim{}) {
				t.Fatalf("invalid geometry refused as %+v/%t/%v", got, found, err)
			}
			return
		}
		blocks := len(status) / int(width)
		victimFence := make([]bool, blocks)
		destinationFence := make([]bool, blocks)
		for block := 0; block < blocks; block++ {
			at := (block*2 + 1) % (len(raw) - 1)
			victimFence[block] = raw[at]&8 != 0
			destinationFence[block] = raw[at]&16 != 0
		}
		want, wantFound, wantErr := referenceCopyAwareFences(rawStatus(status), width, victimFence, destinationFence)
		got, gotFound, gotErr := SelectCopyAwareWithFences(status, width, victimFence, destinationFence)
		if !sameErrorClass(gotErr, wantErr) || gotFound != wantFound || got != want {
			t.Fatalf("fuzz vector disagreement: got=%+v/%t/%v want=%+v/%t/%v", got, gotFound, gotErr, want, wantFound, wantErr)
		}
	})
}

func fenceFromBits(count, bits int) []bool {
	mask := make([]bool, count)
	for block := range mask {
		mask[block] = bits&(1<<block) != 0
	}
	return mask
}

func referenceCopyAwareFences(status []uint8, width int64, victimExcluded, destinationExcluded []bool) (CopyAwareVictim, bool, error) {
	pages := int64(len(status))
	if pages < minPhysicalPages || pages > maxPhysicalPages || width < 1 || width > pages || pages%width != 0 {
		return CopyAwareVictim{}, false, ErrInvalidGeometry
	}
	blocks := pages / width
	if len(victimExcluded) != 0 && int64(len(victimExcluded)) != blocks || len(destinationExcluded) != 0 && int64(len(destinationExcluded)) != blocks {
		return CopyAwareVictim{}, false, ErrInvalidBlockMask
	}
	pending := false
	for _, slot := range status {
		if slot > uint8(ftlpagemap.Stale) {
			return CopyAwareVictim{}, false, ErrInvalidStatus
		}
		pending = pending || slot == uint8(ftlpagemap.Reserved)
	}
	if pending {
		return CopyAwareVictim{}, false, ErrReservationPending
	}
	best := CopyAwareVictim{}
	bestRatioStale, bestRatioLive := int64(0), int64(1)
	found := false
	for block := int64(0); block < blocks; block++ {
		if len(victimExcluded) != 0 && victimExcluded[int(block)] {
			continue
		}
		stale, live, free := int64(0), int64(0), int64(0)
		for physical := block * width; physical < (block+1)*width; physical++ {
			switch status[int(physical)] {
			case uint8(ftlpagemap.Stale):
				stale++
			case uint8(ftlpagemap.Live):
				live++
			}
		}
		if stale == 0 {
			continue
		}
		for physical, slot := range status {
			candidateBlock := int64(physical) / width
			if candidateBlock == block || slot != uint8(ftlpagemap.Free) || len(destinationExcluded) != 0 && destinationExcluded[int(candidateBlock)] {
				continue
			}
			free++
		}
		if free < live {
			continue
		}
		denominator := live + 1
		better := !found || stale*bestRatioLive > bestRatioStale*denominator
		if found && stale*bestRatioLive == bestRatioStale*denominator {
			better = stale > bestRatioStale || stale == bestRatioStale && block < best.Block
		}
		if better {
			best = CopyAwareVictim{Block: block, FirstPhysical: block * width, PageCount: width, StalePages: stale, LivePages: live, FreeDestinations: free}
			bestRatioStale, bestRatioLive, found = stale, denominator, true
		}
	}
	return best, found, nil
}
