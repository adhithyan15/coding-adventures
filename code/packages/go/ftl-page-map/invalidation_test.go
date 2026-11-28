package ftlpagemap

import (
	"errors"
	"math"
	"reflect"
	"testing"
)

func TestInvalidateCurrentRemovesOnlyExactOwnerAndRetainsStaleSlot(t *testing.T) {
	table, err := New(2, 5)
	if err != nil {
		t.Fatal(err)
	}
	table = publishPage(t, table, 0)
	table = publishPage(t, table, 0)
	before := table.Snapshot()
	if before.LogicalToPhysical[0] != 1 || before.PhysicalStatus[0] != Stale || before.PhysicalStatus[1] != Live {
		t.Fatalf("setup snapshot=%+v", before)
	}

	invalidated, err := table.InvalidateCurrent(0, 1)
	if err != nil {
		t.Fatal(err)
	}
	want := table.Snapshot()
	want.LogicalToPhysical[0] = noMapping
	want.PhysicalStatus[1] = Stale
	if got := invalidated.Snapshot(); !reflect.DeepEqual(got, want) {
		t.Fatalf("invalidation snapshot=%+v want=%+v", got, want)
	}
	if got := table.Snapshot(); !reflect.DeepEqual(got, before) {
		t.Fatalf("invalidation changed predecessor: %+v", got)
	}
	if _, present, err := invalidated.Lookup(0); err != nil || present {
		t.Fatalf("invalidated logical page lookup present=%t err=%v", present, err)
	}

	reserved, reservation, err := invalidated.Begin(0)
	if err != nil || reservation.Physical != 2 {
		t.Fatalf("stale pages were reused before erase: %+v %v", reservation, err)
	}
	reserved, err = reserved.Finish(AbortFresh)
	if err != nil {
		t.Fatal(err)
	}
	released, err := reserved.ReleaseErased(1, 1)
	if err != nil {
		t.Fatal(err)
	}
	_, reservation, err = released.Begin(0)
	if err != nil || reservation.Physical != 1 {
		t.Fatalf("completed release did not make invalidated slot reusable: %+v %v", reservation, err)
	}
}

func TestInvalidateCurrentSignedValidationAndPrecedence(t *testing.T) {
	var zero Table
	for _, pair := range [][2]int64{{math.MinInt64, math.MinInt64}, {0, 0}} {
		got, err := zero.InvalidateCurrent(pair[0], pair[1])
		if !errors.Is(err, ErrInvalidGeometry) || !reflect.DeepEqual(got.Snapshot(), Snapshot{}) {
			t.Fatalf("zero InvalidateCurrent%v = %+v/%v", pair, got.Snapshot(), err)
		}
	}
	table, err := New(1, 3)
	if err != nil {
		t.Fatal(err)
	}
	for _, pair := range [][2]int64{
		{math.MinInt64, math.MinInt64},
		{math.MaxInt64, math.MaxInt64},
		{-1, 0},
		{1, -1},
		{0, math.MinInt64},
		{0, math.MaxInt64},
	} {
		before := table.Snapshot()
		_, err := table.InvalidateCurrent(pair[0], pair[1])
		want := ErrCurrentOwnerChanged
		switch {
		case pair[0] < 0 || pair[0] >= 1:
			want = ErrInvalidLogicalPage
		case pair[1] < 0 || pair[1] >= 3:
			want = ErrInvalidPhysicalPage
		}
		if err != want || !reflect.DeepEqual(table.Snapshot(), before) {
			t.Errorf("InvalidateCurrent%v error=%v want=%v", pair, err, want)
		}
	}

	pending, _, err := table.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	for _, pair := range [][2]int64{{-1, -1}, {0, -1}, {0, 2}} {
		before := pending.Snapshot()
		got, err := pending.InvalidateCurrent(pair[0], pair[1])
		want := ErrReservationPending
		if pair[0] < 0 {
			want = ErrInvalidLogicalPage
		} else if pair[1] < 0 {
			want = ErrInvalidPhysicalPage
		}
		if err != want || !reflect.DeepEqual(got.Snapshot(), before) {
			t.Errorf("pending InvalidateCurrent%v error=%v want=%v or changed state", pair, err, want)
		}
	}
}

func TestInvalidateCurrentRejectsAbsentAndStaleCoordinates(t *testing.T) {
	table, err := New(2, 4)
	if err != nil {
		t.Fatal(err)
	}
	for _, pair := range [][2]int64{{0, 0}, {1, 3}} {
		before := table.Snapshot()
		got, err := table.InvalidateCurrent(pair[0], pair[1])
		if err != ErrCurrentOwnerChanged || !reflect.DeepEqual(got.Snapshot(), before) {
			t.Fatalf("absent owner invalidation (%d,%d)=%v", pair[0], pair[1], err)
		}
	}
	table = publishPage(t, table, 0)
	before := table.Snapshot()
	got, err := table.InvalidateCurrent(0, 1)
	if err != ErrCurrentOwnerChanged || !reflect.DeepEqual(got.Snapshot(), before) {
		t.Fatalf("stale coordinate accepted or changed state: %v", err)
	}
	got, err = table.InvalidateCurrent(0, 0)
	if err != nil || got.Snapshot().LogicalToPhysical[0] != noMapping || got.Snapshot().PhysicalStatus[0] != Stale {
		t.Fatalf("matching current owner was not invalidated: %+v %v", got.Snapshot(), err)
	}
}

func publishPage(t *testing.T, table Table, logical int64) Table {
	t.Helper()
	reserved, _, err := table.Begin(logical)
	if err != nil {
		t.Fatal(err)
	}
	published, err := reserved.Finish(MediaReady)
	if err != nil {
		t.Fatal(err)
	}
	return published
}
