package ftlpagemap

import (
	"errors"
	"fmt"
	"math/rand"
	"reflect"
	"sync"
	"testing"
)

func TestReachableStatesCloseUnderCommands(t *testing.T) {
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
				state, actual := entry.model, entry.table
				checkModelInvariants(t, state)
				checkRuntimeInvariants(t, actual)

				for logical := int64(-1); logical <= geometry.logical; logical++ {
					checkLookup(t, state, actual, logical)
					if logical >= 0 && logical < geometry.logical {
						checkBegin(t, state, actual, logical)
						if next, ok := modelBegin(state, logical); ok {
							if key := modelKey(next); !seen[key] {
								nextTable, _, beginErr := actual.Begin(logical)
								if beginErr != nil {
									t.Fatalf("runtime Begin failed on model-accepted state: %v", beginErr)
								}
								seen[key] = true
								queue = append(queue, reachable{model: next, table: nextTable})
							}
						}
					}
				}

				for _, disposition := range []Disposition{MediaReady, AbortFresh, AbortSpent, Disposition(255)} {
					checkFinish(t, state, actual, disposition)
					if next, ok := modelFinish(state, disposition); ok {
						if key := modelKey(next); !seen[key] {
							nextTable, finishErr := actual.Finish(disposition)
							if finishErr != nil {
								t.Fatalf("runtime Finish failed on model-accepted state: %v", finishErr)
							}
							seen[key] = true
							queue = append(queue, reachable{model: next, table: nextTable})
						}
					}
				}

				for logical := int64(-1); logical <= geometry.logical; logical++ {
					for physical := int64(-1); physical <= geometry.physical; physical++ {
						nextTable, next, accepted := compareInvalidateCurrent(t, state, actual, logical, physical)
						if accepted {
							if key := modelKey(next); !seen[key] {
								seen[key] = true
								queue = append(queue, reachable{model: next, table: nextTable})
							}
						}
					}
				}

				// Also issue invalid address and command combinations from every
				// state to prove refusal precedence and atomicity.
				checkBegin(t, state, actual, -1)
				checkBegin(t, state, actual, geometry.logical)
				checkFinish(t, state, actual, Disposition(255))
			}
			t.Logf("enumerated %d reachable states", len(seen))
		})
	}
}

type reachable struct {
	model modelState
	table Table
}

type modelState struct {
	logicalCount int64
	logicalMap   []int64
	physical     []PhysicalStatus
	pending      *Reservation
}

func modelInitial(logicalCount, physicalCount int64) modelState {
	logical := make([]int64, int(logicalCount))
	for index := range logical {
		logical[index] = noMapping
	}
	return modelState{logicalCount: logicalCount, logicalMap: logical, physical: make([]PhysicalStatus, int(physicalCount))}
}

func (state modelState) clone() modelState {
	result := modelState{
		logicalCount: state.logicalCount,
		logicalMap:   append([]int64(nil), state.logicalMap...),
		physical:     append([]PhysicalStatus(nil), state.physical...),
	}
	if state.pending != nil {
		reservation := *state.pending
		result.pending = &reservation
	}
	return result
}

func modelBegin(state modelState, logical int64) (modelState, bool) {
	return modelBeginExcluding(state, logical, nil)
}

func modelBeginExcluding(state modelState, logical int64, excluded []bool) (modelState, bool) {
	if logical < 0 || logical >= state.logicalCount || state.pending != nil {
		return state, false
	}
	if len(excluded) != 0 && len(excluded) != len(state.physical) {
		return state, false
	}
	physical := -1
	for index, slot := range state.physical {
		if slot == Free && (len(excluded) == 0 || !excluded[index]) {
			physical = index
			break
		}
	}
	if physical == -1 {
		return state, false
	}
	result := state.clone()
	reservation := Reservation{Logical: logical, Physical: int64(physical)}
	result.pending = &reservation
	result.physical[physical] = Reserved
	return result, true
}

func modelReleaseErased(state modelState, first, count int64) (modelState, bool) {
	if first < 0 || first >= int64(len(state.physical)) || count <= 0 || count > int64(len(state.physical))-first {
		return state, false
	}
	end := first + count
	for index := first; index < end; index++ {
		status := state.physical[int(index)]
		if status != Free && status != Stale {
			return state, false
		}
	}
	result := state.clone()
	for index := first; index < end; index++ {
		result.physical[int(index)] = Free
	}
	return result, true
}

func modelFinish(state modelState, disposition Disposition) (modelState, bool) {
	if state.pending == nil || disposition > AbortSpent {
		return state, false
	}
	result := state.clone()
	reservation := *result.pending
	result.pending = nil
	slot := int(reservation.Physical)
	switch disposition {
	case MediaReady:
		old := result.logicalMap[int(reservation.Logical)]
		if old >= 0 {
			result.physical[int(old)] = Stale
		}
		result.logicalMap[int(reservation.Logical)] = reservation.Physical
		result.physical[slot] = Live
	case AbortFresh:
		result.physical[slot] = Free
	case AbortSpent:
		result.physical[slot] = Stale
	}
	return result, true
}

func modelInvalidateCurrent(state modelState, logical, physical int64) (modelState, error) {
	if len(state.logicalMap) == 0 || len(state.physical) == 0 {
		return state, ErrInvalidGeometry
	}
	if logical < 0 || logical >= state.logicalCount {
		return state, ErrInvalidLogicalPage
	}
	if physical < 0 || physical >= int64(len(state.physical)) {
		return state, ErrInvalidPhysicalPage
	}
	if state.pending != nil {
		return state, ErrReservationPending
	}
	if state.logicalMap[int(logical)] != physical {
		return state, ErrCurrentOwnerChanged
	}
	result := state.clone()
	result.logicalMap[int(logical)] = noMapping
	result.physical[int(physical)] = Stale
	return result, nil
}

func compareInvalidateCurrent(t *testing.T, state modelState, table Table, logical, physical int64) (Table, modelState, bool) {
	t.Helper()
	want, wantErr := modelInvalidateCurrent(state, logical, physical)
	got, err := table.InvalidateCurrent(logical, physical)
	if err != wantErr {
		t.Fatalf("InvalidateCurrent(%d,%d) error=%v want=%v", logical, physical, err, wantErr)
	}
	if wantErr != nil {
		if !reflect.DeepEqual(got.Snapshot(), table.Snapshot()) {
			t.Fatalf("InvalidateCurrent(%d,%d) refusal changed predecessor", logical, physical)
		}
		return table, state, false
	}
	if !reflect.DeepEqual(got.Snapshot(), modelSnapshot(want)) {
		t.Fatalf("InvalidateCurrent(%d,%d) result mismatch", logical, physical)
	}
	return got, want, true
}

func modelKey(state modelState) string {
	return fmt.Sprintf("%v/%v/%v", state.logicalMap, state.physical, state.pending)
}

func checkModelInvariants(t *testing.T, state modelState) {
	t.Helper()
	counts := []int{0, 0, 0, 0}
	for _, status := range state.physical {
		if int(status) >= len(counts) {
			t.Fatalf("unknown model slot status %d", status)
		}
		counts[status]++
	}
	if counts[Free]+counts[Reserved]+counts[Live]+counts[Stale] != len(state.physical) {
		t.Fatalf("model slot partition does not cover every page: %v", counts)
	}
	if counts[Reserved] > 1 || (counts[Reserved] == 1) != (state.pending != nil) {
		t.Fatalf("model reservation count mismatch: counts=%v pending=%v", counts, state.pending)
	}
	owners := make([]int, len(state.physical))
	for logical, physical := range state.logicalMap {
		if physical == noMapping {
			continue
		}
		if physical < 0 || physical >= int64(len(state.physical)) || state.physical[int(physical)] != Live {
			t.Fatalf("model logical %d maps to non-live page %d", logical, physical)
		}
		owners[int(physical)]++
	}
	for index, status := range state.physical {
		if (status == Live) != (owners[index] == 1) {
			t.Fatalf("model live ownership mismatch at physical page %d", index)
		}
	}
}

func checkRuntimeInvariants(t *testing.T, table Table) {
	t.Helper()
	snapshot := table.Snapshot()
	model := modelState{
		logicalCount: int64(len(snapshot.LogicalToPhysical)),
		logicalMap:   snapshot.LogicalToPhysical,
		physical:     snapshot.PhysicalStatus,
	}
	if snapshot.Reservation != nil {
		reservation := *snapshot.Reservation
		model.pending = &reservation
	}
	checkModelInvariants(t, model)
	for logical, physical := range snapshot.LogicalToPhysical {
		got, present, err := table.Lookup(int64(logical))
		if err != nil || present != (physical != noMapping) || (present && got != physical) || (!present && got != 0) {
			t.Fatalf("runtime lookup %d disagrees with snapshot %d/%t/%v", logical, got, present, err)
		}
	}
}

func checkLookup(t *testing.T, state modelState, table Table, logical int64) {
	t.Helper()
	physical, present, err := table.Lookup(logical)
	wantErr := error(nil)
	wantPhysical, wantPresent := int64(0), false
	if logical < 0 || logical >= state.logicalCount {
		wantErr = ErrInvalidLogicalPage
	} else if mapping := state.logicalMap[int(logical)]; mapping != noMapping {
		wantPhysical, wantPresent = mapping, true
	}
	if !errors.Is(err, wantErr) || physical != wantPhysical || present != wantPresent {
		t.Fatalf("Lookup(%d) = %d/%t/%v, want %d/%t/%v", logical, physical, present, err, wantPhysical, wantPresent, wantErr)
	}
}

func checkBegin(t *testing.T, state modelState, table Table, logical int64) {
	t.Helper()
	got, reservation, err := table.Begin(logical)
	wantErr := error(nil)
	if logical < 0 || logical >= state.logicalCount {
		wantErr = ErrInvalidLogicalPage
	} else if state.pending != nil {
		wantErr = ErrReservationPending
	} else if _, ok := modelBegin(state, logical); !ok {
		wantErr = ErrNoFreePage
	}
	if !errors.Is(err, wantErr) {
		t.Fatalf("Begin(%d) error = %v, want %v", logical, err, wantErr)
	}
	if wantErr != nil {
		if reservation != (Reservation{}) || !reflect.DeepEqual(got.Snapshot(), table.Snapshot()) {
			t.Fatalf("Begin(%d) refusal changed state or returned a reservation", logical)
		}
		return
	}
	want, _ := modelBegin(state, logical)
	wantReservation := *want.pending
	if reservation != wantReservation || !reflect.DeepEqual(got.Snapshot(), modelSnapshot(want)) {
		t.Fatalf("Begin(%d) result disagrees: reservation=%+v snapshot=%+v", logical, reservation, got.Snapshot())
	}
}

func checkFinish(t *testing.T, state modelState, table Table, disposition Disposition) {
	t.Helper()
	got, err := table.Finish(disposition)
	wantErr := error(nil)
	if state.pending == nil {
		wantErr = ErrNoReservation
	} else if disposition > AbortSpent {
		wantErr = ErrInvalidDisposition
	}
	if !errors.Is(err, wantErr) {
		t.Fatalf("Finish(%d) error = %v, want %v", disposition, err, wantErr)
	}
	if wantErr != nil {
		if !reflect.DeepEqual(got.Snapshot(), table.Snapshot()) {
			t.Fatalf("Finish(%d) refusal changed state", disposition)
		}
		return
	}
	want, _ := modelFinish(state, disposition)
	if !reflect.DeepEqual(got.Snapshot(), modelSnapshot(want)) {
		t.Fatalf("Finish(%d) snapshot = %+v, want %+v", disposition, got.Snapshot(), modelSnapshot(want))
	}
}

func compareBegin(t *testing.T, state modelState, table Table, logical int64) Table {
	t.Helper()
	checkBegin(t, state, table, logical)
	if _, ok := modelBegin(state, logical); ok {
		got, _, _ := table.Begin(logical)
		return got
	}
	return table
}

func compareFinish(t *testing.T, state modelState, table Table, disposition Disposition) Table {
	t.Helper()
	checkFinish(t, state, table, disposition)
	if _, ok := modelFinish(state, disposition); ok {
		got, _ := table.Finish(disposition)
		return got
	}
	return table
}

func modelSnapshot(state modelState) Snapshot {
	snapshot := Snapshot{
		LogicalToPhysical: append([]int64(nil), state.logicalMap...),
		PhysicalStatus:    append([]PhysicalStatus(nil), state.physical...),
	}
	if state.pending != nil {
		reservation := *state.pending
		snapshot.Reservation = &reservation
	}
	return snapshot
}

func geometryName(logical, physical int64) string {
	return fmt.Sprintf("%d-logical-%d-physical", logical, physical)
}

func TestTypedValidationPrecedenceAndZeroTable(t *testing.T) {
	var zero Table
	if _, _, err := zero.Begin(-1); !errors.Is(err, ErrInvalidGeometry) {
		t.Fatalf("zero table Begin precedence: %v", err)
	}
	if _, _, err := zero.Lookup(0); !errors.Is(err, ErrInvalidGeometry) {
		t.Fatalf("zero table Lookup: %v", err)
	}
	if _, err := zero.Finish(Disposition(255)); !errors.Is(err, ErrInvalidGeometry) {
		t.Fatalf("zero table Finish precedence: %v", err)
	}
	if got := zero.Snapshot(); !reflect.DeepEqual(got, Snapshot{}) {
		t.Fatalf("zero table snapshot: %+v", got)
	}
	for _, geometry := range [][2]int64{{0, 2}, {1, 1}, {1, 129}, {65, 66}, {2, 2}, {64, 64}} {
		if _, err := New(geometry[0], geometry[1]); !errors.Is(err, ErrInvalidGeometry) {
			t.Fatalf("New(%d,%d) error = %v", geometry[0], geometry[1], err)
		}
	}
	if _, err := New(1, 2); err != nil {
		t.Fatalf("minimum valid geometry: %v", err)
	}
	if _, err := New(64, 128); err != nil {
		t.Fatalf("maximum valid geometry: %v", err)
	}

	table, _ := New(1, 2)
	if _, _, err := table.Begin(-1); !errors.Is(err, ErrInvalidLogicalPage) {
		t.Fatalf("invalid logical address on available table: %v", err)
	}
	table, _, _ = table.Begin(0)
	if _, _, err := table.Begin(1); !errors.Is(err, ErrInvalidLogicalPage) {
		t.Fatalf("logical address precedes pending reservation: %v", err)
	}
	if _, err := table.Finish(Disposition(255)); !errors.Is(err, ErrInvalidDisposition) {
		t.Fatalf("disposition after active reservation: %v", err)
	}
	table, err := table.Finish(AbortSpent)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := table.Finish(Disposition(255)); !errors.Is(err, ErrNoReservation) {
		t.Fatalf("reservation state precedes disposition: %v", err)
	}
}

func TestReservationKeepsOldMappingUntilReadyAndInvalidatesItAtomically(t *testing.T) {
	table, _ := New(1, 3)
	first, reservation, err := table.Begin(0)
	if err != nil || reservation != (Reservation{Logical: 0, Physical: 0}) {
		t.Fatalf("first reservation = %+v, %v", reservation, err)
	}
	first, err = first.Finish(MediaReady)
	if err != nil {
		t.Fatal(err)
	}
	second, reservation, err := first.Begin(0)
	if err != nil || reservation.Physical != 1 {
		t.Fatalf("second reservation = %+v, %v", reservation, err)
	}
	if physical, present, err := second.Lookup(0); err != nil || !present || physical != 0 {
		t.Fatalf("pending relocation published early: %d, %t, %v", physical, present, err)
	}
	third, err := second.Finish(MediaReady)
	if err != nil {
		t.Fatal(err)
	}
	snapshot := third.Snapshot()
	if snapshot.LogicalToPhysical[0] != 1 || snapshot.PhysicalStatus[0] != Stale || snapshot.PhysicalStatus[1] != Live {
		t.Fatalf("rewrite did not publish and stale old slot atomically: %+v", snapshot)
	}
}

func TestFreshAndSpentAbortsPreserveMappingAndCapacitySemantics(t *testing.T) {
	for _, disposition := range []Disposition{AbortFresh, AbortSpent} {
		table, _ := New(1, 3)
		table, _, _ = table.Begin(0)
		table, _ = table.Finish(MediaReady)
		reserved, reservation, err := table.Begin(0)
		if err != nil || reservation.Physical != 1 {
			t.Fatalf("begin for disposition %d: %+v, %v", disposition, reservation, err)
		}
		table, err = reserved.Finish(disposition)
		if err != nil {
			t.Fatal(err)
		}
		physical, present, err := table.Lookup(0)
		if err != nil || !present || physical != 0 {
			t.Fatalf("abort %d changed live mapping: %d, %t, %v", disposition, physical, present, err)
		}
		snapshot := table.Snapshot()
		want := Free
		if disposition == AbortSpent {
			want = Stale
		}
		if snapshot.PhysicalStatus[1] != want {
			t.Fatalf("abort %d slot status = %v, want %v", disposition, snapshot.PhysicalStatus[1], want)
		}
		if disposition == AbortFresh {
			next, selected, err := table.Begin(0)
			if err != nil || selected.Physical != 1 {
				t.Fatalf("fresh abort page was not reusable as lowest free: %+v, %v", selected, err)
			}
			table = next
		} else {
			last, _, err := table.Begin(0)
			if err != nil {
				t.Fatal(err)
			}
			last, err = last.Finish(AbortSpent)
			if err != nil {
				t.Fatal(err)
			}
			if _, _, err := last.Begin(0); !errors.Is(err, ErrNoFreePage) {
				t.Fatalf("spent abort should consume the final free page: %v", err)
			}
		}
	}
}

func TestSpentPhysicalSlotIsNeverSelectedAgain(t *testing.T) {
	table, _ := New(1, 3)
	table, first, err := table.Begin(0)
	if err != nil || first.Physical != 0 {
		t.Fatalf("first allocation = %+v, %v", first, err)
	}
	table, err = table.Finish(AbortSpent)
	if err != nil {
		t.Fatal(err)
	}
	_, next, err := table.Begin(0)
	if err != nil || next.Physical != 1 {
		t.Fatal("spent physical page was reused before reclamation")
	}
}

func TestRefusalsAndSnapshotsAreDetached(t *testing.T) {
	table, _ := New(2, 3)
	before := table.Snapshot()
	pending, reservation, err := table.Begin(1)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(table.Snapshot(), before) {
		t.Fatal("Begin mutated the prior value-owned table")
	}
	if reservation.Physical != 0 {
		t.Fatalf("lowest free physical slot = %d", reservation.Physical)
	}
	before.PhysicalStatus[0] = Stale
	if pending.Snapshot().PhysicalStatus[0] != Reserved {
		t.Fatal("snapshot mutation changed table")
	}
	pending.Snapshot().Reservation.Logical = 0
	if pending.Snapshot().Reservation.Logical != 1 {
		t.Fatal("reservation snapshot aliases private reservation")
	}
	refused, reservation, err := pending.Begin(-1)
	if !errors.Is(err, ErrInvalidLogicalPage) || reservation != (Reservation{}) || !reflect.DeepEqual(refused.Snapshot(), pending.Snapshot()) {
		t.Fatalf("invalid-address refusal changed state or precedence: %+v, %+v, %v", refused.Snapshot(), reservation, err)
	}
	refused, err = pending.Finish(Disposition(255))
	if !errors.Is(err, ErrInvalidDisposition) || !reflect.DeepEqual(refused.Snapshot(), pending.Snapshot()) {
		t.Fatalf("invalid-disposition refusal changed state: %+v, %v", refused.Snapshot(), err)
	}
}

func TestEightThousandSeededCommandsMatchIndependentModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05A))
	model := modelInitial(3, 7)
	table, _ := New(3, 7)
	for step := 0; step < 8_000; step++ {
		if step > 0 && step%23 == 0 {
			model = modelInitial(3, 7)
			table, _ = New(3, 7)
		}
		logical := int64(random.Intn(5) - 1)
		switch random.Intn(3) {
		case 0:
			checkLookup(t, model, table, logical)
		case 1:
			table = compareBegin(t, model, table, logical)
			if logical >= 0 && logical < model.logicalCount {
				if next, ok := modelBegin(model, logical); ok {
					model = next
				}
			}
		case 2:
			disposition := Disposition(random.Intn(4))
			table = compareFinish(t, model, table, disposition)
			if next, ok := modelFinish(model, disposition); ok {
				model = next
			}
		}
		checkRuntimeInvariants(t, table)
		checkModelInvariants(t, model)
	}
}

func TestFourThousandSeededInvalidationCommandsMatchIndependentModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05C6B2A))
	model := modelInitial(3, 7)
	table, _ := New(3, 7)
	for step := 0; step < 4_000; step++ {
		if step > 0 && step%29 == 0 {
			model = modelInitial(3, 7)
			table, _ = New(3, 7)
		}
		logical := int64(random.Intn(5) - 1)
		physical := int64(random.Intn(9) - 1)
		table, model, _ = compareInvalidateCurrent(t, model, table, logical, physical)
		checkRuntimeInvariants(t, table)
		checkModelInvariants(t, model)
	}
}

func TestIndependentTablesCanAdvanceConcurrently(t *testing.T) {
	const workers = 8
	var wait sync.WaitGroup
	errorsFound := make(chan error, workers)
	for worker := 0; worker < workers; worker++ {
		wait.Add(1)
		go func() {
			defer wait.Done()
			table, _ := New(2, 4)
			for step := 0; step < 200; step++ {
				var err error
				table, _, err = table.Begin(int64(step % 2))
				if err != nil {
					continue
				}
				table, err = table.Finish(Disposition(step % 3))
				if err != nil {
					errorsFound <- err
					return
				}
			}
		}()
	}
	wait.Wait()
	close(errorsFound)
	for err := range errorsFound {
		t.Errorf("independent table transition failed: %v", err)
	}
}

func TestExhaustionAfterSpentPageIsAtomic(t *testing.T) {
	table, _ := New(1, 2)
	table, _, _ = table.Begin(0)
	table, _ = table.Finish(AbortSpent)
	table, _, _ = table.Begin(0)
	table, _ = table.Finish(MediaReady)
	before := table.Snapshot()
	refused, reservation, err := table.Begin(0)
	if !errors.Is(err, ErrNoFreePage) || reservation != (Reservation{}) || !reflect.DeepEqual(refused.Snapshot(), before) {
		t.Fatalf("exhausted map refusal not atomic: %+v, %v", reservation, err)
	}
}
