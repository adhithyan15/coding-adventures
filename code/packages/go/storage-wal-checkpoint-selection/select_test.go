package storagewalcheckpointselection

import (
	"errors"
	"math"
	"reflect"
	"testing"

	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
	storagewalcheckpointslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot"
)

const portableMax uint64 = 9_007_199_254_740_991

func makePublished(record storagewalcheckpointrecord.Record) []byte {
	frame, err := storagewalcheckpointrecord.Encode(record)
	if err != nil {
		panic(err)
	}
	return frame
}
func makeStaged(record storagewalcheckpointrecord.Record) []byte {
	slot, err := storagewalcheckpointslot.Stage(record)
	if err != nil {
		panic(err)
	}
	return slot
}
func emptySlot() []byte { return make([]byte, storagewalcheckpointrecord.FrameBytes) }
func record(phase storagewalcheckpointrecord.Phase, epoch, lsn uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: phase, Epoch: epoch, PreviousLSN: lsn}
}

func modelSelection(records [2]storagewalcheckpointrecord.Record, published [2]bool) (Selection, error) {
	if !published[0] && !published[1] {
		return Selection{}, ErrNoPublishedCheckpoint
	}
	if published[0] && !published[1] {
		return Selection{Slot: 0, Record: records[0]}, nil
	}
	if published[1] && !published[0] {
		return Selection{Slot: 1, Record: records[1]}, nil
	}
	a, b := records[0], records[1]
	if a.Epoch != b.Epoch {
		if a.Epoch > b.Epoch {
			return Selection{Slot: 0, Record: a}, nil
		}
		return Selection{Slot: 1, Record: b}, nil
	}
	if a.PreviousLSN != b.PreviousLSN {
		return Selection{}, ErrCheckpointConflict
	}
	if a.Phase == storagewalcheckpointrecord.Ready && b.Phase != storagewalcheckpointrecord.Ready {
		return Selection{Slot: 0, Record: a}, nil
	}
	if b.Phase == storagewalcheckpointrecord.Ready && a.Phase != storagewalcheckpointrecord.Ready {
		return Selection{Slot: 1, Record: b}, nil
	}
	return Selection{Slot: 0, Record: a}, nil
}

func TestSelectionSingleSlotsAndFallback(t *testing.T) {
	ready := record(storagewalcheckpointrecord.Ready, 8, 19)
	resetting := record(storagewalcheckpointrecord.Resetting, 9, 20)
	cases := []struct {
		name    string
		slots   [2][]byte
		want    Selection
		wantErr error
	}{
		{"slot-zero-only", [2][]byte{makePublished(ready), emptySlot()}, Selection{Slot: 0, Record: ready}, nil},
		{"slot-one-only", [2][]byte{makeStaged(resetting), makePublished(ready)}, Selection{Slot: 1, Record: ready}, nil},
		{"staged-and-vacant", [2][]byte{makeStaged(ready), emptySlot()}, Selection{}, ErrNoPublishedCheckpoint},
		{"both-vacant", [2][]byte{emptySlot(), emptySlot()}, Selection{}, ErrNoPublishedCheckpoint},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			before := [2][]byte{append([]byte(nil), tc.slots[0]...), append([]byte(nil), tc.slots[1]...)}
			got, err := Select(tc.slots)
			if !reflect.DeepEqual(got, tc.want) || !errors.Is(err, tc.wantErr) {
				t.Fatalf("Select=(%+v,%v), want (%+v,%v)", got, err, tc.want, tc.wantErr)
			}
			if !reflect.DeepEqual(tc.slots, before) {
				t.Fatal("selection modified input slots")
			}
		})
	}
}

func TestSelectionCrossings(t *testing.T) {
	cases := []struct {
		name    string
		a, b    storagewalcheckpointrecord.Record
		want    Selection
		wantErr error
	}{
		{"higher-epoch-slot-1", record(storagewalcheckpointrecord.Resetting, 2, 4), record(storagewalcheckpointrecord.Ready, 3, 0), Selection{Slot: 1, Record: record(storagewalcheckpointrecord.Ready, 3, 0)}, nil},
		{"higher-epoch-slot-0", record(storagewalcheckpointrecord.Ready, 3, 0), record(storagewalcheckpointrecord.Resetting, 2, 4), Selection{Slot: 0, Record: record(storagewalcheckpointrecord.Ready, 3, 0)}, nil},
		{"ready-slot-0", record(storagewalcheckpointrecord.Ready, 5, 8), record(storagewalcheckpointrecord.Resetting, 5, 8), Selection{Slot: 0, Record: record(storagewalcheckpointrecord.Ready, 5, 8)}, nil},
		{"ready-slot-1", record(storagewalcheckpointrecord.Resetting, 5, 8), record(storagewalcheckpointrecord.Ready, 5, 8), Selection{Slot: 1, Record: record(storagewalcheckpointrecord.Ready, 5, 8)}, nil},
		{"identical-record-lower-ordinal", record(storagewalcheckpointrecord.Ready, 7, 9), record(storagewalcheckpointrecord.Ready, 7, 9), Selection{Slot: 0, Record: record(storagewalcheckpointrecord.Ready, 7, 9)}, nil},
		{"same-epoch-conflict", record(storagewalcheckpointrecord.Resetting, 7, 9), record(storagewalcheckpointrecord.Resetting, 7, 10), Selection{}, ErrCheckpointConflict},
		{"different-epoch-rank-does-not-conflict", record(storagewalcheckpointrecord.Ready, 7, 99), record(storagewalcheckpointrecord.Resetting, 8, 1), Selection{Slot: 1, Record: record(storagewalcheckpointrecord.Resetting, 8, 1)}, nil},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			slots := [2][]byte{makePublished(tc.a), makePublished(tc.b)}
			got, err := Select(slots)
			if !reflect.DeepEqual(got, tc.want) || !errors.Is(err, tc.wantErr) {
				t.Fatalf("Select=(%+v,%v), want (%+v,%v)", got, err, tc.want, tc.wantErr)
			}
		})
	}
}

func TestSlotInspectionErrorsPrecedeSelection(t *testing.T) {
	valid := makePublished(record(storagewalcheckpointrecord.Ready, 10, 5))
	badGuard := emptySlot()
	badGuard[storagewalcheckpointslot.GuardOffset] = 0x33
	badFrame := append([]byte(nil), valid...)
	badFrame[0] ^= 1
	cases := []struct {
		name  string
		slots [2][]byte
		want  error
	}{
		{"first-slot-width-before-second-slot-guard", [2][]byte{[]byte{1}, badGuard}, storagewalcheckpointslot.ErrSlotWidth},
		{"second-slot-guard-after-first-published", [2][]byte{valid, badGuard}, storagewalcheckpointslot.ErrSlotGuard},
		{"first-slot-guard-before-second-invalid-frame", [2][]byte{badGuard, badFrame}, storagewalcheckpointslot.ErrSlotGuard},
		{"second-invalid-frame-before-conflict", [2][]byte{makePublished(record(storagewalcheckpointrecord.Ready, 4, 1)), badFrame}, storagerecordframe.ErrMagic},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got, err := Select(tc.slots)
			if err == nil || !reflect.DeepEqual(got, Selection{}) {
				t.Fatalf("error result=(%+v,%v)", got, err)
			}
			if !errors.Is(err, tc.want) {
				t.Fatalf("error=%v want %v", err, tc.want)
			}
		})
	}
}

func TestBoundaryEpochAndRankCrossingsAgainstModel(t *testing.T) {
	phases := []storagewalcheckpointrecord.Phase{storagewalcheckpointrecord.Resetting, storagewalcheckpointrecord.Ready}
	values := []uint64{1, 2, portableMax}
	cases := 0
	for _, phaseA := range phases {
		for _, epochA := range values {
			for _, rankA := range []uint64{0, 1, portableMax} {
				for _, phaseB := range phases {
					for _, epochB := range values {
						for _, rankB := range []uint64{0, 1, portableMax} {
							a := record(phaseA, epochA, rankA)
							b := record(phaseB, epochB, rankB)
							got, err := Select([2][]byte{makePublished(a), makePublished(b)})
							want, wantErr := modelSelection([2]storagewalcheckpointrecord.Record{a, b}, [2]bool{true, true})
							if !reflect.DeepEqual(got, want) || !errors.Is(err, wantErr) {
								t.Fatalf("records=%+v/%+v got=(%+v,%v) want=(%+v,%v)", a, b, got, err, want, wantErr)
							}
							cases++
						}
					}
				}
			}
		}
	}
	if cases != 324 {
		t.Fatalf("model crossings=%d", cases)
	}
	if math.MaxUint64 <= portableMax {
		t.Fatal("test bound assumption failed")
	}
}

func FuzzSelectCheckpointSlots(f *testing.F) {
	f.Add([]byte{1, 3, 1, 0, 1, 4, 2, 0})
	f.Add([]byte{2, 7, 0, 2, 1, 7, 0, 2})
	f.Add([]byte{0, 0, 0, 0, 0, 0, 0, 0})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) < 8 || len(raw) > 64 {
			t.Skip()
		}
		var records [2]storagewalcheckpointrecord.Record
		var published [2]bool
		var slots [2][]byte
		for i := 0; i < 2; i++ {
			base := i * 4
			phase := storagewalcheckpointrecord.Resetting
			if raw[base+2]&1 != 0 {
				phase = storagewalcheckpointrecord.Ready
			}
			record := storagewalcheckpointrecord.Record{Phase: phase, Epoch: uint64(raw[base]) + 1, PreviousLSN: uint64(raw[base+1])}
			records[i] = record
			switch raw[base+3] % 3 {
			case 0:
				slots[i] = emptySlot()
			case 1:
				slots[i] = makeStaged(record)
			case 2:
				published[i] = true
				slots[i] = makePublished(record)
			}
		}
		before := [2][]byte{append([]byte(nil), slots[0]...), append([]byte(nil), slots[1]...)}
		got, err := Select(slots)
		want, wantErr := modelSelection(records, published)
		if !reflect.DeepEqual(got, want) || !errors.Is(err, wantErr) {
			t.Fatalf("Select=(%+v,%v), model=(%+v,%v)", got, err, want, wantErr)
		}
		if !reflect.DeepEqual(slots, before) {
			t.Fatal("Select modified slot bytes")
		}
	})
}
