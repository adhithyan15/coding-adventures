package ftlpagemap

import (
	"math"
	"math/rand"
	"reflect"
	"testing"
)

func restoreModel(logicalCount, physicalCount int64, snapshot Snapshot) error {
	if logicalCount < 1 || logicalCount > 64 || physicalCount < 2 || physicalCount > 128 || physicalCount <= logicalCount {
		return ErrInvalidGeometry
	}
	if len(snapshot.LogicalToPhysical) != int(logicalCount) || len(snapshot.PhysicalStatus) != int(physicalCount) {
		return ErrInvalidSnapshotShape
	}
	if snapshot.Reservation != nil {
		return ErrReservationPending
	}
	for _, role := range snapshot.PhysicalStatus {
		if role != Free && role != Live && role != Stale {
			return ErrInvalidPhysicalStatus
		}
	}
	owners := make(map[int64]int, logicalCount)
	for logical, physical := range snapshot.LogicalToPhysical {
		if physical == -1 {
			continue
		}
		if physical < 0 || physical >= physicalCount {
			return ErrInvalidPhysicalPage
		}
		if snapshot.PhysicalStatus[int(physical)] != Live {
			return ErrMappingRoleMismatch
		}
		if _, exists := owners[physical]; exists {
			return ErrDuplicatePhysicalMapping
		}
		owners[physical] = logical
	}
	for physical, role := range snapshot.PhysicalStatus {
		if role == Live {
			if _, exists := owners[int64(physical)]; !exists {
				return ErrUnownedLivePage
			}
		}
	}
	return nil
}

func checkRestoreMatchesModel(t *testing.T, logicalCount, physicalCount int64, snapshot Snapshot) Table {
	t.Helper()
	wantErr := restoreModel(logicalCount, physicalCount, snapshot)
	got, err := Restore(logicalCount, physicalCount, snapshot)
	if err != wantErr {
		t.Fatalf("Restore(%d,%d,%+v) error=%v want=%v", logicalCount, physicalCount, snapshot, err, wantErr)
	}
	if wantErr != nil {
		if !reflect.DeepEqual(got, Table{}) {
			t.Fatalf("Restore refusal returned a configured or nonzero table: %+v", got)
		}
		return Table{}
	}
	if !reflect.DeepEqual(got.Snapshot(), snapshot) {
		t.Fatalf("Restore snapshot=%+v want=%+v", got.Snapshot(), snapshot)
	}
	return got
}

func TestRestoreFiniteSettledAndInconsistentSnapshotMatrix(t *testing.T) {
	for _, geometry := range [][2]int64{{1, 2}, {2, 3}} {
		logicalCount, physicalCount := geometry[0], geometry[1]
		statusValues := []PhysicalStatus{Free, Reserved, Live, Stale, PhysicalStatus(4), PhysicalStatus(255)}
		mappingValues := []int64{math.MinInt64, -2, -1, 0, physicalCount - 1, physicalCount, math.MaxInt64}
		checked := 0
		var visitStatuses func(int, []PhysicalStatus)
		visitStatuses = func(index int, statuses []PhysicalStatus) {
			if index != len(statuses) {
				for _, status := range statusValues {
					statuses[index] = status
					visitStatuses(index+1, statuses)
				}
				return
			}
			var visitMappings func(int, []int64)
			visitMappings = func(logical int, mappings []int64) {
				if logical != len(mappings) {
					for _, physical := range mappingValues {
						mappings[logical] = physical
						visitMappings(logical+1, mappings)
					}
					return
				}
				for _, pending := range []bool{false, true} {
					snapshot := Snapshot{
						LogicalToPhysical: append([]int64(nil), mappings...),
						PhysicalStatus:    append([]PhysicalStatus(nil), statuses...),
					}
					if pending {
						snapshot.Reservation = &Reservation{Logical: 0, Physical: 0}
					}
					checkRestoreMatchesModel(t, logicalCount, physicalCount, snapshot)
					checked++
				}
			}
			visitMappings(0, make([]int64, int(logicalCount)))
		}
		visitStatuses(0, make([]PhysicalStatus, int(physicalCount)))
		t.Logf("checked %d snapshot combinations for %d/%d", checked, logicalCount, physicalCount)
	}
}

func TestRestoreValidationPrecedenceAndSignedGeometry(t *testing.T) {
	validSnapshot := Snapshot{
		LogicalToPhysical: []int64{-1},
		PhysicalStatus:    []PhysicalStatus{Free, Stale},
	}
	pending := &Reservation{Logical: 99, Physical: 99}
	cases := []struct {
		name string
		l, p int64
		s    Snapshot
		want error
	}{
		{"invalid-geometry-before-shape-and-pending", math.MinInt64, math.MaxInt64, Snapshot{Reservation: pending}, ErrInvalidGeometry},
		{"shape-before-pending", 1, 2, Snapshot{Reservation: pending}, ErrInvalidSnapshotShape},
		{"pending-before-physical-status", 1, 2, Snapshot{LogicalToPhysical: []int64{-1}, PhysicalStatus: []PhysicalStatus{Reserved, 99}, Reservation: pending}, ErrReservationPending},
		{"physical-role-before-address", 1, 2, Snapshot{LogicalToPhysical: []int64{math.MaxInt64}, PhysicalStatus: []PhysicalStatus{Reserved, Free}}, ErrInvalidPhysicalStatus},
		{"address-before-role-mismatch-at-next-logical", 2, 3, Snapshot{LogicalToPhysical: []int64{3, 1}, PhysicalStatus: []PhysicalStatus{Free, Free, Live}}, ErrInvalidPhysicalPage},
		{"role-mismatch-before-unowned-live", 1, 2, Snapshot{LogicalToPhysical: []int64{0}, PhysicalStatus: []PhysicalStatus{Stale, Live}}, ErrMappingRoleMismatch},
		{"duplicate-before-unowned-live", 2, 3, Snapshot{LogicalToPhysical: []int64{0, 0}, PhysicalStatus: []PhysicalStatus{Live, Live, Free}}, ErrDuplicatePhysicalMapping},
		{"unowned-live-after-logical-pass", 1, 2, Snapshot{LogicalToPhysical: []int64{-1}, PhysicalStatus: []PhysicalStatus{Free, Live}}, ErrUnownedLivePage},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			_, err := Restore(tc.l, tc.p, tc.s)
			if err != tc.want {
				t.Fatalf("Restore error=%v want=%v", err, tc.want)
			}
		})
	}
	for _, geometry := range [][2]int64{{0, 2}, {1, 1}, {1, 129}, {65, 66}, {math.MinInt64, 128}, {1, math.MaxInt64}, {math.MaxInt64, math.MinInt64}} {
		if _, err := Restore(geometry[0], geometry[1], validSnapshot); err != ErrInvalidGeometry {
			t.Errorf("Restore(%d,%d) error=%v want=%v", geometry[0], geometry[1], err, ErrInvalidGeometry)
		}
	}
}

func TestRestoreRejectsReservedAndUnknownPhysicalRoles(t *testing.T) {
	for _, role := range []PhysicalStatus{Reserved, PhysicalStatus(4), PhysicalStatus(255)} {
		snapshot := Snapshot{LogicalToPhysical: []int64{-1}, PhysicalStatus: []PhysicalStatus{Free, role}}
		if _, err := Restore(1, 2, snapshot); err != ErrInvalidPhysicalStatus {
			t.Fatalf("Restore accepted invalid physical role %d: %v", role, err)
		}
	}
}

func TestRestoreRejectsUnownedLivePage(t *testing.T) {
	snapshot := Snapshot{LogicalToPhysical: []int64{-1, -1}, PhysicalStatus: []PhysicalStatus{Free, Live, Stale}}
	if _, err := Restore(2, 3, snapshot); err != ErrUnownedLivePage {
		t.Fatalf("Restore left live page without an owner: %v", err)
	}
}

func TestRestoreRejectsDuplicateLogicalOwners(t *testing.T) {
	snapshot := Snapshot{LogicalToPhysical: []int64{1, 1}, PhysicalStatus: []PhysicalStatus{Free, Live, Stale}}
	if _, err := Restore(2, 3, snapshot); err != ErrDuplicatePhysicalMapping {
		t.Fatalf("Restore accepted duplicate logical owners: %v", err)
	}
}

func TestRestoreOwnsSnapshotArrays(t *testing.T) {
	snapshot := Snapshot{LogicalToPhysical: []int64{1}, PhysicalStatus: []PhysicalStatus{Stale, Live}}
	table, err := Restore(1, 2, snapshot)
	if err != nil {
		t.Fatal(err)
	}
	before := table.Snapshot()
	snapshot.LogicalToPhysical[0] = -1
	snapshot.PhysicalStatus[1] = Stale
	if !reflect.DeepEqual(table.Snapshot(), before) {
		t.Fatal("restore retained caller arrays")
	}
}

func TestRestoreMaximumGeometryAndSignedMappingExtrema(t *testing.T) {
	logical := make([]int64, 64)
	physical := make([]PhysicalStatus, 128)
	for index := range logical {
		logical[index] = int64(index + 64)
	}
	for index := 0; index < 64; index++ {
		physical[index] = Stale
		physical[index+64] = Live
	}
	snapshot := Snapshot{LogicalToPhysical: logical, PhysicalStatus: physical}
	table := checkRestoreMatchesModel(t, 64, 128, snapshot)
	for logicalPage := int64(0); logicalPage < 64; logicalPage++ {
		got, present, err := table.Lookup(logicalPage)
		if err != nil || !present || got != logicalPage+64 {
			t.Fatalf("max geometry lookup(%d)=(%d,%t,%v)", logicalPage, got, present, err)
		}
	}
	for _, invalid := range []int64{math.MinInt64, -2, math.MaxInt64} {
		bad := Snapshot{LogicalToPhysical: []int64{invalid}, PhysicalStatus: []PhysicalStatus{Free, Stale}}
		if _, err := Restore(1, 2, bad); err != ErrInvalidPhysicalPage {
			t.Errorf("invalid signed map ordinal %d error=%v", invalid, err)
		}
	}
}

func TestRestoreStaleOnlyAndDetachedSnapshotRoundTrip(t *testing.T) {
	stale := Snapshot{LogicalToPhysical: []int64{-1, -1}, PhysicalStatus: []PhysicalStatus{Stale, Stale, Stale}}
	table := checkRestoreMatchesModel(t, 2, 3, stale)
	first := table.Snapshot()
	first.LogicalToPhysical[0] = 2
	first.PhysicalStatus[0] = Free
	if !reflect.DeepEqual(table.Snapshot(), stale) {
		t.Fatal("restored stale-only table shared its snapshot arrays")
	}
}

func TestRestoreReachableSettledStatesAndEquivalentContinuations(t *testing.T) {
	initial, err := New(2, 4)
	if err != nil {
		t.Fatal(err)
	}
	states := []Table{initial}
	current, _, err := initial.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	current, err = current.Finish(MediaReady)
	if err != nil {
		t.Fatal(err)
	}
	states = append(states, current)
	current, _, err = current.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	current, err = current.Finish(MediaReady)
	if err != nil {
		t.Fatal(err)
	}
	states = append(states, current)
	current, _, err = current.Begin(1)
	if err != nil {
		t.Fatal(err)
	}
	current, err = current.Finish(AbortSpent)
	if err != nil {
		t.Fatal(err)
	}
	states = append(states, current)
	current, err = current.InvalidateCurrent(0, 1)
	if err != nil {
		t.Fatal(err)
	}
	states = append(states, current)

	commands := []struct {
		name string
		call func(Table) (Table, error)
	}{
		{"begin", func(table Table) (Table, error) { next, _, err := table.Begin(1); return next, err }},
		{"finish-ready", func(table Table) (Table, error) { return table.Finish(MediaReady) }},
		{"invalidate", func(table Table) (Table, error) { return table.InvalidateCurrent(0, 1) }},
		{"release", func(table Table) (Table, error) { return table.ReleaseErased(0, 2) }},
	}
	for index, predecessor := range states {
		snapshot := predecessor.Snapshot()
		restored, restoreErr := Restore(2, 4, snapshot)
		if restoreErr != nil {
			t.Fatalf("reachable state %d restore: %v", index, restoreErr)
		}
		if !reflect.DeepEqual(restored.Snapshot(), snapshot) {
			t.Fatalf("reachable state %d did not round-trip", index)
		}
		left, right := predecessor, restored
		for _, command := range commands {
			left, err = command.call(left)
			leftErr := err
			right, err = command.call(right)
			if leftErr != err || !reflect.DeepEqual(left.Snapshot(), right.Snapshot()) {
				t.Fatalf("continuation %s differs after state %d: left err=%v right err=%v", command.name, index, leftErr, err)
			}
		}
	}
}

func TestRestoreSeededSnapshotsMatchIndependentModel(t *testing.T) {
	rng := rand.New(rand.NewSource(0x05e3b3a))
	for index := 0; index < 5000; index++ {
		logicalCount := int64(1 + rng.Intn(4))
		physicalCount := logicalCount + 1 + int64(rng.Intn(4))
		snapshot := Snapshot{
			LogicalToPhysical: make([]int64, int(logicalCount)),
			PhysicalStatus:    make([]PhysicalStatus, int(physicalCount)),
		}
		for physical := range snapshot.PhysicalStatus {
			snapshot.PhysicalStatus[physical] = PhysicalStatus(rng.Intn(6))
		}
		for logical := range snapshot.LogicalToPhysical {
			snapshot.LogicalToPhysical[logical] = int64(rng.Intn(int(physicalCount)+4) - 3)
		}
		if rng.Intn(5) == 0 {
			snapshot.Reservation = &Reservation{Logical: int64(rng.Intn(5)), Physical: int64(rng.Intn(6))}
		}
		checkRestoreMatchesModel(t, logicalCount, physicalCount, snapshot)
	}
}

func FuzzRestoreSnapshots(f *testing.F) {
	f.Add([]byte{1, 2, byte(Free), byte(Stale), 255})
	f.Add([]byte{1, 2, byte(Free), byte(Live), 0})
	f.Add([]byte{2, 3, byte(Live), byte(Stale), byte(Live), 0, 2})
	f.Add([]byte{1, 2, byte(Reserved), byte(Free), 255})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 32 || len(raw) < 2 {
			return
		}
		logicalCount := int64(1 + int(raw[0]%4))
		physicalCount := logicalCount + 1 + int64(raw[1]%byte(6-logicalCount))
		need := 2 + int(physicalCount) + int(logicalCount)
		if len(raw) < need {
			return
		}
		snapshot := Snapshot{
			LogicalToPhysical: make([]int64, int(logicalCount)),
			PhysicalStatus:    make([]PhysicalStatus, int(physicalCount)),
		}
		cursor := 2
		for index := range snapshot.PhysicalStatus {
			snapshot.PhysicalStatus[index] = PhysicalStatus(raw[cursor])
			cursor++
		}
		for index := range snapshot.LogicalToPhysical {
			snapshot.LogicalToPhysical[index] = int64(int8(raw[cursor]))
			cursor++
		}
		if raw[1]&0x80 != 0 {
			snapshot.Reservation = &Reservation{Logical: 0, Physical: 0}
		}
		checkRestoreMatchesModel(t, logicalCount, physicalCount, snapshot)
	})
}
