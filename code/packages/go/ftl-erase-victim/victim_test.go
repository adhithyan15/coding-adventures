package ftlerasevictim

import (
	"errors"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

func TestAllSmallLayoutsMatchIndependentModel(t *testing.T) {
	checked := 0
	layoutsWithVictim := 0
	layoutsWithTopTie := 0
	for pageCount := 2; pageCount <= 6; pageCount++ {
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
			for pagesPerBlock := 1; pagesPerBlock <= pageCount; pagesPerBlock++ {
				if pageCount%pagesPerBlock != 0 {
					continue
				}
				want, found, maxTies := referenceSelect(rawStatus(status), int64(pagesPerBlock))
				got, gotFound, err := Select(status, int64(pagesPerBlock))
				if err != nil || gotFound != found || got != want {
					t.Fatalf("status=%v pages_per_block=%d => %+v/%t/%v; model %+v/%t", status, pagesPerBlock, got, gotFound, err, want, found)
				}
				if found {
					layoutsWithVictim++
				}
				if maxTies > 1 {
					layoutsWithTopTie++
				}
				checked++
			}
		}
	}
	if checked != 19_360 || layoutsWithVictim != 7_319 || layoutsWithTopTie != 2_597 {
		t.Fatalf("exhaustion census = %d checked, %d with victim, %d with top tie", checked, layoutsWithVictim, layoutsWithTopTie)
	}
}

func TestInvalidGeometryPrecedesUnknownStatuses(t *testing.T) {
	for _, testCase := range []struct {
		status []ftlpagemap.PhysicalStatus
		width  int64
	}{
		{status: []ftlpagemap.PhysicalStatus{255}, width: 1},
		{status: []ftlpagemap.PhysicalStatus{255, ftlpagemap.Free}, width: 0},
		{status: []ftlpagemap.PhysicalStatus{255, ftlpagemap.Free}, width: 3},
		{status: []ftlpagemap.PhysicalStatus{255, ftlpagemap.Free, ftlpagemap.Free}, width: 2},
		{status: make([]ftlpagemap.PhysicalStatus, maxPhysicalPages+1), width: 1},
	} {
		victim, found, err := Select(testCase.status, testCase.width)
		if !errors.Is(err, ErrInvalidGeometry) || found || victim != (Victim{}) {
			t.Fatalf("invalid geometry status=%v width=%d => %+v/%t/%v", testCase.status, testCase.width, victim, found, err)
		}
	}
}

func TestUnknownStatusIsRejectedEvenInBlockedOrNonCandidateBlock(t *testing.T) {
	for _, status := range [][]ftlpagemap.PhysicalStatus{
		{ftlpagemap.Live, 255, ftlpagemap.Stale, ftlpagemap.Free},
		{ftlpagemap.Free, ftlpagemap.Free, 254, ftlpagemap.Free},
	} {
		victim, found, err := Select(status, 2)
		if !errors.Is(err, ErrInvalidStatus) || found || victim != (Victim{}) {
			t.Fatalf("unknown status %v => %+v/%t/%v", status, victim, found, err)
		}
	}
}

func TestLiveBlockCannotBeVictim(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free}
	victim, found, err := Select(status, 2)
	if err != nil || found || victim != (Victim{}) {
		t.Fatal("block with a live page must be ineligible")
	}
}

func TestReservedBlockCannotBeVictim(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Reserved, ftlpagemap.Free, ftlpagemap.Free}
	victim, found, err := Select(status, 2)
	if err != nil || found || victim != (Victim{}) {
		t.Fatal("block with a reserved page must be ineligible")
	}
}

func TestFreeSlotsDoNotEarnVictimScore(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Free}
	victim, found, err := Select(status, 2)
	if err != nil || found || victim != (Victim{}) {
		t.Fatal("free slots do not earn victim score")
	}
}

func TestTiesChooseLowestBlockIndex(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Free}
	victim, found, err := Select(status, 2)
	if err != nil || !found || victim != (Victim{Block: 0, FirstPhysical: 0, PageCount: 2, StalePages: 1}) {
		t.Fatal("equal stale scores must choose the lowest block index")
	}
}

func TestMaximumExtentSelectsLastPhysicalBlockWithoutInputMutation(t *testing.T) {
	status := make([]ftlpagemap.PhysicalStatus, maxPhysicalPages)
	for index := 0; index < len(status); index++ {
		status[index] = ftlpagemap.Free
	}
	status[96], status[97], status[98] = ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Stale
	status[99] = ftlpagemap.Free
	status[80] = ftlpagemap.Live
	for index := 101; index < len(status); index++ {
		status[index] = ftlpagemap.Free
	}
	before := append([]ftlpagemap.PhysicalStatus(nil), status...)
	victim, found, err := Select(status, 32)
	want := Victim{Block: 3, FirstPhysical: 96, PageCount: 32, StalePages: 3}
	if err != nil || !found || victim != want {
		t.Fatalf("maximum extent = %+v/%t/%v; want %+v", victim, found, err, want)
	}
	if !reflect.DeepEqual(status, before) {
		t.Fatal("victim selection mutated the caller's status vector")
	}
}

func TestMaximumGeometryAndNoCandidateSentinel(t *testing.T) {
	status := make([]ftlpagemap.PhysicalStatus, maxPhysicalPages)
	for index := range status {
		status[index] = ftlpagemap.Live
	}
	victim, found, err := Select(status, 1)
	if err != nil || found || victim != (Victim{}) {
		t.Fatalf("valid maximum geometry with no candidate = %+v/%t/%v", victim, found, err)
	}
}

func rawStatus(status []ftlpagemap.PhysicalStatus) []uint8 {
	result := make([]uint8, len(status))
	for index, value := range status {
		result[index] = uint8(value)
	}
	return result
}

func referenceSelect(status []uint8, pagesPerBlock int64) (Victim, bool, int) {
	pageCount := int64(len(status))
	if pageCount < 2 || pageCount > 128 || pagesPerBlock < 1 || pagesPerBlock > pageCount || pageCount%pagesPerBlock != 0 {
		return Victim{}, false, 0
	}
	for _, value := range status {
		if value > 3 {
			return Victim{}, false, 0
		}
	}
	maximum := 0
	var best Victim
	found := false
	ties := 0
	blocks := int(pageCount / pagesPerBlock)
	for block := 0; block < blocks; block++ {
		first := block * int(pagesPerBlock)
		last := first + int(pagesPerBlock)
		hasStale, disqualified := false, false
		score := 0
		for physical := first; physical < last; physical++ {
			value := status[physical]
			switch value {
			case 1, 2:
				disqualified = true
			case 3:
				hasStale = true
				score++
			}
		}
		if disqualified || !hasStale {
			continue
		}
		if !found || score > maximum {
			found = true
			maximum = score
			ties = 1
			best = Victim{Block: int64(block), FirstPhysical: int64(first), PageCount: pagesPerBlock, StalePages: int64(score)}
		} else if score == maximum {
			ties++
		}
	}
	return best, found, ties
}
