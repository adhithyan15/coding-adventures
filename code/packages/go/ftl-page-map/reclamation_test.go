package ftlpagemap

import (
	"errors"
	"math"
	"reflect"
	"testing"
)

func TestAllocationFenceAndExtentReleaseReachableClosure(t *testing.T) {
	for _, geometry := range []struct{ logical, physical int64 }{{1, 2}, {2, 3}} {
		t.Run(geometryName(geometry.logical, geometry.physical), func(t *testing.T) {
			initial, err := New(geometry.logical, geometry.physical)
			if err != nil {
				t.Fatal(err)
			}
			queue := []reachable{{model: modelInitial(geometry.logical, geometry.physical), table: initial}}
			seen := map[string]bool{modelKey(queue[0].model): true}
			for head := 0; head < len(queue); head++ {
				entry := queue[head]
				state, table := entry.model, entry.table
				checkRuntimeInvariants(t, table)
				for logical := int64(0); logical < geometry.logical; logical++ {
					for _, mask := range allMasks(int(geometry.physical)) {
						checkBeginExcluding(t, state, table, logical, mask)
						if next, ok := modelBeginExcluding(state, logical, mask); ok && !seen[modelKey(next)] {
							got, _, beginErr := table.BeginExcluding(logical, mask)
							if beginErr != nil {
								t.Fatalf("model accepted fenced begin: %v", beginErr)
							}
							seen[modelKey(next)] = true
							queue = append(queue, reachable{model: next, table: got})
						}
					}
				}
				for _, disposition := range []Disposition{MediaReady, AbortFresh, AbortSpent} {
					if next, ok := modelFinish(state, disposition); ok && !seen[modelKey(next)] {
						got, finishErr := table.Finish(disposition)
						if finishErr != nil {
							t.Fatalf("model accepted finish: %v", finishErr)
						}
						seen[modelKey(next)] = true
						queue = append(queue, reachable{model: next, table: got})
					}
				}
				checkBeginExcluding(t, state, table, -1, nil)
				checkBeginExcluding(t, state, table, geometry.logical, nil)
				for _, mask := range invalidMasks(int(geometry.physical)) {
					checkBeginExcluding(t, state, table, 0, mask)
				}
				for first := int64(0); first < geometry.physical; first++ {
					for count := int64(1); count <= geometry.physical-first; count++ {
						checkReleaseErased(t, state, table, first, count)
						if next, ok := modelReleaseErased(state, first, count); ok && !seen[modelKey(next)] {
							got, releaseErr := table.ReleaseErased(first, count)
							if releaseErr != nil {
								t.Fatalf("model accepted release [%d,%d): %v", first, first+count, releaseErr)
							}
							seen[modelKey(next)] = true
							queue = append(queue, reachable{model: next, table: got})
						}
					}
				}
				for _, bounds := range [][2]int64{{-1, 1}, {0, 0}, {geometry.physical, 1}, {0, geometry.physical + 1}, {math.MinInt64, 1}, {math.MaxInt64, 1}, {1, math.MaxInt64}} {
					checkReleaseErased(t, state, table, bounds[0], bounds[1])
				}
			}
			t.Logf("closed %d reachable states over every mask and in-bounds extent", len(seen))
		})
	}
}

func allMasks(width int) [][]bool {
	masks := make([][]bool, 0, 1<<width+1)
	masks = append(masks, nil)
	for bits := 0; bits < 1<<width; bits++ {
		mask := make([]bool, width)
		for index := range mask {
			mask[index] = bits&(1<<index) != 0
		}
		masks = append(masks, mask)
	}
	return masks
}

func invalidMasks(width int) [][]bool {
	return [][]bool{make([]bool, 1), make([]bool, width-1), make([]bool, width+1)}
}

func checkBeginExcluding(t *testing.T, state modelState, table Table, logical int64, mask []bool) {
	t.Helper()
	got, reservation, err := table.BeginExcluding(logical, mask)
	var wantErr error
	switch {
	case logical < 0 || logical >= state.logicalCount:
		wantErr = ErrInvalidLogicalPage
	case state.pending != nil:
		wantErr = ErrReservationPending
	case len(mask) != 0 && len(mask) != len(state.physical):
		wantErr = ErrInvalidFence
	default:
		if _, ok := modelBeginExcluding(state, logical, mask); !ok {
			wantErr = ErrNoFreePage
		}
	}
	if !errors.Is(err, wantErr) {
		t.Fatalf("BeginExcluding(%d,%v) error=%v want=%v", logical, mask, err, wantErr)
	}
	if wantErr != nil {
		if reservation != (Reservation{}) || !reflect.DeepEqual(got.Snapshot(), table.Snapshot()) {
			t.Fatalf("fenced begin refusal changed state/result")
		}
		return
	}
	want, _ := modelBeginExcluding(state, logical, mask)
	if reservation != *want.pending || !reflect.DeepEqual(got.Snapshot(), modelSnapshot(want)) {
		t.Fatalf("fenced begin result mismatch")
	}
}

func checkReleaseErased(t *testing.T, state modelState, table Table, first, count int64) {
	t.Helper()
	got, err := table.ReleaseErased(first, count)
	var wantErr error
	if first < 0 || first >= int64(len(state.physical)) || count <= 0 || count > int64(len(state.physical))-first {
		wantErr = ErrInvalidExtent
	} else {
		for index := first; index < first+count; index++ {
			if state.physical[int(index)] != Free && state.physical[int(index)] != Stale {
				wantErr = ErrIneligibleExtent
				break
			}
		}
	}
	if !errors.Is(err, wantErr) {
		t.Fatalf("ReleaseErased(%d,%d) error=%v want=%v", first, count, err, wantErr)
	}
	if wantErr != nil {
		if !reflect.DeepEqual(got.Snapshot(), table.Snapshot()) {
			t.Fatalf("release refusal changed state")
		}
		return
	}
	want, _ := modelReleaseErased(state, first, count)
	if !reflect.DeepEqual(got.Snapshot(), modelSnapshot(want)) {
		t.Fatalf("ReleaseErased(%d,%d) mismatch", first, count)
	}
}

func TestFenceAndReleasePrecedenceEndpointsAndPendingReservation(t *testing.T) {
	var zero Table
	if got, reservation, err := zero.BeginExcluding(-1, []bool{true}); !errors.Is(err, ErrInvalidGeometry) || reservation != (Reservation{}) || !reflect.DeepEqual(got.Snapshot(), Snapshot{}) {
		t.Fatalf("zero table fence precedence")
	}
	if got, err := zero.ReleaseErased(math.MaxInt64, math.MaxInt64); !errors.Is(err, ErrInvalidGeometry) || !reflect.DeepEqual(got.Snapshot(), Snapshot{}) {
		t.Fatalf("zero table release precedence")
	}
	table, _ := New(1, 3)
	pending, reservation, err := table.BeginExcluding(0, []bool{true, false, false})
	if err != nil || reservation.Physical != 1 {
		t.Fatalf("fence did not skip first free page: %+v %v", reservation, err)
	}
	if _, _, err := pending.BeginExcluding(-1, []bool{true}); !errors.Is(err, ErrInvalidLogicalPage) {
		t.Fatalf("logical precedes reservation: %v", err)
	}
	if _, _, err := pending.BeginExcluding(0, []bool{true}); !errors.Is(err, ErrReservationPending) {
		t.Fatalf("pending precedes mask: %v", err)
	}
	if _, _, err := table.BeginExcluding(0, []bool{true, true, true}); !errors.Is(err, ErrNoFreePage) {
		t.Fatalf("all-fenced capacity: %v", err)
	}
	if _, _, err := table.BeginExcluding(0, []bool{true, false}); !errors.Is(err, ErrInvalidFence) {
		t.Fatalf("invalid mask: %v", err)
	}
	if _, err := pending.ReleaseErased(1, 1); !errors.Is(err, ErrIneligibleExtent) {
		t.Fatalf("reserved page release: %v", err)
	}
}

func TestBeginExcludingSkipsFencedFreePage(t *testing.T) {
	table, _ := New(1, 3)
	plain, plainReservation, plainErr := table.Begin(0)
	emptyMask, emptyReservation, emptyErr := table.BeginExcluding(0, []bool{})
	if plainErr != nil || emptyErr != nil || plainReservation != emptyReservation || !reflect.DeepEqual(plain.Snapshot(), emptyMask.Snapshot()) {
		t.Fatal("zero-length mask differs from Begin")
	}
	// Continue from a fresh table for the nonempty fence assertion.
	table, _ = New(1, 3)
	mask := []bool{true, false, false}
	got, reservation, err := table.BeginExcluding(0, mask)
	mask[1] = true
	if err != nil || reservation.Physical != 1 || got.Snapshot().PhysicalStatus[0] != Free {
		t.Fatal("fenced lowest free page was selected")
	}
	if got.Snapshot().PhysicalStatus[1] != Reserved || table.Snapshot().PhysicalStatus[1] != Free {
		t.Fatal("the caller's mask was retained or mutated")
	}
}

func TestReleaseRefusalKeepsEligiblePrefixUntouched(t *testing.T) {
	table, _ := New(1, 3)
	for range 2 {
		table, _, _ = table.Begin(0)
		table, _ = table.Finish(MediaReady)
	}
	before := table.Snapshot()
	got, err := table.ReleaseErased(0, 2)
	if !errors.Is(err, ErrIneligibleExtent) || !reflect.DeepEqual(got.Snapshot(), before) {
		t.Fatal("eligible prefix changed on refused extent")
	}
}

func TestReleasePreservesOutsideReservationAndReusesLowestPage(t *testing.T) {
	table, _ := New(1, 5)
	for range 3 {
		table, _, _ = table.Begin(0)
		table, _ = table.Finish(MediaReady)
	}
	pending, reservation, err := table.Begin(0)
	if err != nil || reservation.Physical != 3 {
		t.Fatalf("reservation setup: %+v %v", reservation, err)
	}
	before := pending.Snapshot()
	released, err := pending.ReleaseErased(0, 2)
	if err != nil {
		t.Fatal(err)
	}
	snapshot := released.Snapshot()
	if snapshot.Reservation == nil || *snapshot.Reservation != reservation || snapshot.PhysicalStatus[0] != Free || snapshot.PhysicalStatus[1] != Free || snapshot.PhysicalStatus[3] != Reserved {
		t.Fatalf("release did not preserve outside reservation: %+v", snapshot)
	}
	if before.PhysicalStatus[0] != Stale || pending.Snapshot().PhysicalStatus[0] != Stale {
		t.Fatal("release mutated predecessor")
	}
	resolved, err := released.Finish(AbortSpent)
	if err != nil {
		t.Fatal(err)
	}
	_, selected, err := resolved.Begin(0)
	if err != nil || selected.Physical != 0 {
		t.Fatalf("lowest released page not reused: %+v %v", selected, err)
	}
}

func TestMaximumGeometryAndFinalSlotExtent(t *testing.T) {
	table, err := New(64, 128)
	if err != nil {
		t.Fatal(err)
	}
	last, err := table.ReleaseErased(127, 1)
	if err != nil || !reflect.DeepEqual(last.Snapshot(), table.Snapshot()) {
		t.Fatalf("final all-free extent: %v", err)
	}
	for _, bounds := range [][2]int64{{math.MaxInt64, math.MaxInt64}, {127, math.MaxInt64}} {
		if _, err := table.ReleaseErased(bounds[0], bounds[1]); !errors.Is(err, ErrInvalidExtent) {
			t.Fatalf("bounds %v: %v", bounds, err)
		}
	}
}
