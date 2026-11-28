package ftlpagemap

import (
	"errors"
	"math"
	"math/rand"
	"reflect"
	"testing"
)

func modelRecordUnowned(state modelState, physical int64) (modelState, error) {
	if len(state.logicalMap) == 0 || len(state.physical) == 0 {
		return state, ErrInvalidGeometry
	}
	if physical < 0 || physical >= int64(len(state.physical)) {
		return state, ErrInvalidPhysicalPage
	}
	if state.pending != nil {
		return state, ErrReservationPending
	}
	if state.physical[int(physical)] != Free {
		return state, ErrPhysicalPageUnavailable
	}
	result := state.clone()
	result.physical[int(physical)] = Stale
	return result, nil
}

func compareRecordUnowned(t *testing.T, state modelState, table Table, physical int64) (Table, modelState, bool) {
	t.Helper()
	want, wantErr := modelRecordUnowned(state, physical)
	got, err := table.RecordUnownedProgram(physical)
	if !errors.Is(err, wantErr) {
		t.Fatalf("RecordUnownedProgram(%d) error=%v, want %v", physical, err, wantErr)
	}
	if wantErr != nil {
		if !reflect.DeepEqual(got.Snapshot(), table.Snapshot()) {
			t.Fatalf("RecordUnownedProgram(%d) refusal changed predecessor", physical)
		}
		return table, state, false
	}
	if !reflect.DeepEqual(got.Snapshot(), modelSnapshot(want)) {
		t.Fatalf("RecordUnownedProgram(%d) snapshot=%+v want=%+v", physical, got.Snapshot(), modelSnapshot(want))
	}
	if !reflect.DeepEqual(table.Snapshot(), modelSnapshot(state)) {
		t.Fatal("RecordUnownedProgram changed its predecessor")
	}
	return got, want, true
}

func TestRecordUnownedProgramMarksStaleAndPreservesOwners(t *testing.T) {
	table, _ := New(2, 5)
	table = publishPage(t, table, 0)
	table = publishPage(t, table, 1)
	before := table.Snapshot()
	got, err := table.RecordUnownedProgram(4)
	if err != nil {
		t.Fatal(err)
	}
	want := table.Snapshot()
	want.PhysicalStatus[4] = Stale
	if !reflect.DeepEqual(got.Snapshot(), want) {
		t.Fatalf("unowned program transition mismatch: got=%+v want=%+v", got.Snapshot(), want)
	}
	if !reflect.DeepEqual(table.Snapshot(), before) {
		t.Fatal("accepted transition changed predecessor")
	}
	for logical, physical := range before.LogicalToPhysical {
		actual, present, lookupErr := got.Lookup(int64(logical))
		if lookupErr != nil || !present || actual != physical {
			t.Fatalf("logical owner %d changed: %d/%t/%v", logical, actual, present, lookupErr)
		}
	}
}

func TestRecordUnownedProgramRefusesEveryNonFreeStateAndPending(t *testing.T) {
	table, _ := New(1, 4)
	reserved, _, err := table.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := reserved.RecordUnownedProgram(0); err != ErrReservationPending {
		t.Fatalf("reserved page did not hit pending precedence: %v", err)
	}
	reserved, err = reserved.Finish(MediaReady)
	if err != nil {
		t.Fatal(err)
	}
	spent, _, err := reserved.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	spent, err = spent.Finish(AbortSpent)
	if err != nil {
		t.Fatal(err)
	}
	for name, state := range map[string]Table{"live": reserved, "stale": spent} {
		var physical int64
		if name == "stale" {
			physical = 1
		}
		before := state.Snapshot()
		got, err := state.RecordUnownedProgram(physical)
		if err != ErrPhysicalPageUnavailable || !reflect.DeepEqual(got.Snapshot(), before) {
			t.Fatalf("%s page refusal error=%v changed=%t", name, err, !reflect.DeepEqual(got.Snapshot(), before))
		}
	}
}

func TestRecordUnownedProgramValidationPrecedenceAndSignedBounds(t *testing.T) {
	var zero Table
	for _, physical := range []int64{math.MinInt64, -1, 0, math.MaxInt64} {
		got, err := zero.RecordUnownedProgram(physical)
		if err != ErrInvalidGeometry || !reflect.DeepEqual(got.Snapshot(), Snapshot{}) {
			t.Fatalf("zero table RecordUnownedProgram(%d)=%+v/%v", physical, got.Snapshot(), err)
		}
	}
	table, _ := New(1, 3)
	for _, physical := range []int64{math.MinInt64, -1, 3, math.MaxInt64} {
		before := table.Snapshot()
		got, err := table.RecordUnownedProgram(physical)
		if err != ErrInvalidPhysicalPage || !reflect.DeepEqual(got.Snapshot(), before) {
			t.Fatalf("invalid address %d error=%v or changed predecessor", physical, err)
		}
	}
	pending, _, err := table.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	for _, physical := range []int64{-1, 0, 3, math.MaxInt64} {
		before := pending.Snapshot()
		got, err := pending.RecordUnownedProgram(physical)
		want := ErrReservationPending
		if physical < 0 || physical >= 3 {
			want = ErrInvalidPhysicalPage
		}
		if err != want || !reflect.DeepEqual(got.Snapshot(), before) {
			t.Fatalf("pending address %d error=%v want=%v", physical, err, want)
		}
	}
}

func TestRecordUnownedProgramMatchesExactPositionAbortAndReusesAfterErase(t *testing.T) {
	table, _ := New(2, 5)
	table = publishPage(t, table, 0)
	table = publishPage(t, table, 1)
	for physical, status := range table.Snapshot().PhysicalStatus {
		if status != Free {
			continue
		}
		actual, err := table.RecordUnownedProgram(int64(physical))
		if err != nil {
			t.Fatal(err)
		}
		mask := make([]bool, len(table.Snapshot().PhysicalStatus))
		for index := range mask {
			mask[index] = index != physical
		}
		reserved, reservation, err := table.BeginExcluding(0, mask)
		if err != nil || reservation.Physical != int64(physical) {
			t.Fatalf("exact-position reserve %d: %+v/%v", physical, reservation, err)
		}
		aborted, err := reserved.Finish(AbortSpent)
		if err != nil {
			t.Fatal(err)
		}
		if !reflect.DeepEqual(actual.Snapshot(), aborted.Snapshot()) {
			t.Fatalf("unowned program differs from exact-position AbortSpent at %d", physical)
		}
	}
	consumed, err := table.RecordUnownedProgram(2)
	if err != nil {
		t.Fatal(err)
	}
	next, reservation, err := consumed.Begin(0)
	if err != nil || reservation.Physical != 3 {
		t.Fatalf("allocator did not skip consumed page: %+v/%v", reservation, err)
	}
	next, err = next.Finish(AbortFresh)
	if err != nil {
		t.Fatal(err)
	}
	released, err := next.ReleaseErased(2, 1)
	if err != nil {
		t.Fatal(err)
	}
	_, reservation, err = released.Begin(0)
	if err != nil || reservation.Physical != 2 {
		t.Fatalf("completed erase did not reuse consumed page: %+v/%v", reservation, err)
	}
}

func TestRecordUnownedProgramSmallStateClosure(t *testing.T) {
	for _, geometry := range [][2]int64{{1, 2}, {2, 3}} {
		initial, _ := New(geometry[0], geometry[1])
		queue := []reachable{{model: modelInitial(geometry[0], geometry[1]), table: initial}}
		seen := map[string]bool{modelKey(queue[0].model): true}
		for head := 0; head < len(queue); head++ {
			entry := queue[head]
			state, table := entry.model, entry.table
			for physical := int64(-1); physical <= geometry[1]; physical++ {
				nextTable, nextState, accepted := compareRecordUnowned(t, state, table, physical)
				if accepted {
					addUnownedReachable(t, &queue, seen, nextState, nextTable)
				}
			}
			for logical := int64(0); logical < geometry[0]; logical++ {
				if next, ok := modelBegin(state, logical); ok {
					nextTable, _, err := table.Begin(logical)
					if err != nil {
						t.Fatal(err)
					}
					addUnownedReachable(t, &queue, seen, next, nextTable)
				}
			}
			for _, disposition := range []Disposition{MediaReady, AbortFresh, AbortSpent} {
				if next, ok := modelFinish(state, disposition); ok {
					nextTable, err := table.Finish(disposition)
					if err != nil {
						t.Fatal(err)
					}
					addUnownedReachable(t, &queue, seen, next, nextTable)
				}
			}
			for first := int64(0); first < geometry[1]; first++ {
				for count := int64(1); count <= geometry[1]-first; count++ {
					if next, ok := modelReleaseErased(state, first, count); ok {
						nextTable, err := table.ReleaseErased(first, count)
						if err != nil {
							t.Fatal(err)
						}
						addUnownedReachable(t, &queue, seen, next, nextTable)
					}
				}
			}
		}
		t.Logf("geometry %s closed %d model states", geometryName(geometry[0], geometry[1]), len(seen))
	}
}

func addUnownedReachable(t *testing.T, queue *[]reachable, seen map[string]bool, state modelState, table Table) {
	t.Helper()
	key := modelKey(state)
	if seen[key] {
		return
	}
	if !reflect.DeepEqual(table.Snapshot(), modelSnapshot(state)) {
		t.Fatalf("reachable transition differs from model: %+v / %+v", table.Snapshot(), modelSnapshot(state))
	}
	seen[key] = true
	*queue = append(*queue, reachable{model: state, table: table})
}

func TestFiveThousandSeededUnownedProgramCommandsMatchModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05E2B2D1))
	model := modelInitial(3, 7)
	table, _ := New(3, 7)
	for step := 0; step < 5_000; step++ {
		if step > 0 && step%37 == 0 {
			model = modelInitial(3, 7)
			table, _ = New(3, 7)
		}
		physical := int64(random.Intn(9) - 1)
		switch random.Intn(4) {
		case 0:
			table, model, _ = compareRecordUnowned(t, model, table, physical)
		case 1:
			logical := int64(random.Intn(5) - 1)
			table = compareBegin(t, model, table, logical)
			if next, ok := modelBegin(model, logical); ok {
				model = next
			}
		case 2:
			disposition := Disposition(random.Intn(4))
			table = compareFinish(t, model, table, disposition)
			if next, ok := modelFinish(model, disposition); ok {
				model = next
			}
		case 3:
			first := int64(random.Intn(9) - 1)
			count := int64(random.Intn(5) + 1)
			next, ok := modelReleaseErased(model, first, count)
			got, err := table.ReleaseErased(first, count)
			if ok && err != nil || !ok && err == nil {
				t.Fatalf("ReleaseErased(%d,%d) err=%v accepted=%t", first, count, err, ok)
			}
			if ok {
				table, model = got, next
			}
		}
		if !reflect.DeepEqual(table.Snapshot(), modelSnapshot(model)) {
			t.Fatalf("seeded step %d diverged", step)
		}
	}
}

func FuzzRecordUnownedProgramHistories(f *testing.F) {
	f.Add([]byte{0, 0, 0, 1, 0, 2, 3, 2, 2, 1})
	f.Add([]byte{1, 0, 0, 2, 0, 0, 2, 1, 3, 1, 2, 2})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 256 {
			return
		}
		model := modelInitial(2, 4)
		table, _ := New(2, 4)
		for index := 0; index+1 < len(raw); index += 2 {
			value, command := raw[index], raw[index+1]
			switch command % 4 {
			case 0:
				table, model, _ = compareRecordUnowned(t, model, table, int64(int(value)%6-1))
			case 1:
				logical := int64(int(value)%4 - 1)
				table = compareBegin(t, model, table, logical)
				if next, ok := modelBegin(model, logical); ok {
					model = next
				}
			case 2:
				disposition := Disposition(value % 4)
				table = compareFinish(t, model, table, disposition)
				if next, ok := modelFinish(model, disposition); ok {
					model = next
				}
			case 3:
				first, count := int64(value%5), int64(command%4+1)
				next, ok := modelReleaseErased(model, first, count)
				got, err := table.ReleaseErased(first, count)
				if ok && err != nil || !ok && err == nil {
					t.Fatalf("ReleaseErased(%d,%d) err=%v accepted=%t", first, count, err, ok)
				}
				if ok {
					table, model = got, next
				}
			}
			if !reflect.DeepEqual(table.Snapshot(), modelSnapshot(model)) {
				t.Fatalf("history step %d diverged", index/2)
			}
		}
	})
}
