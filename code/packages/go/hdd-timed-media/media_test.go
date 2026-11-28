package hddtimedmedia

import (
	"errors"
	"math"
	"math/rand"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
	hddseektiming "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-seek-timing"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
)

const referenceMaxTick uint64 = 9_007_199_254_740_991

type referenceMedium struct {
	geometry hddgeometry.Geometry
	state    hddseektiming.State
	sectors  map[int64][sectorBytes]byte
}

func newReference(geometry hddgeometry.Geometry) (*referenceMedium, error) {
	if !validReferenceGeometry(geometry) {
		return nil, blockdevicecontract.ErrInvalidGeometry
	}
	return &referenceMedium{geometry: geometry, sectors: make(map[int64][sectorBytes]byte)}, nil
}

func validReferenceGeometry(g hddgeometry.Geometry) bool {
	if g.Cylinders <= 0 || g.Surfaces <= 0 || g.SectorsPerTrack <= 0 || g.Cylinders > int64(referenceMaxTick)/g.Surfaces {
		return false
	}
	tracks := g.Cylinders * g.Surfaces
	return tracks <= int64(referenceMaxTick)/g.SectorsPerTrack
}

func (reference *referenceMedium) read(block int64) (ReadResult, error) {
	plan, err := referencePlan(reference.geometry, reference.state, block, true)
	if err != nil {
		return ReadResult{}, err
	}
	result := ReadResult{Data: reference.sectors[block], Timing: plan}
	reference.state = plan.End
	return result, nil
}

func (reference *referenceMedium) write(block int64, data []byte, cut hddwritecrashcuts.Cut) (WriteResult, error) {
	if !validReferenceGeometry(reference.geometry) {
		return WriteResult{}, blockdevicecontract.ErrInvalidGeometry
	}
	count := reference.geometry.Cylinders * reference.geometry.Surfaces * reference.geometry.SectorsPerTrack
	if block < 0 || block >= count {
		return WriteResult{}, blockdevicecontract.ErrInvalidBlock
	}
	if len(data) != sectorBytes {
		return WriteResult{}, blockdevicecontract.ErrInvalidBlockData
	}
	persisted, acknowledged, err := referenceDescribe(cut)
	if err != nil {
		return WriteResult{}, err
	}
	plan, err := referencePlan(reference.geometry, reference.state, block, persisted > 0)
	if err != nil {
		return WriteResult{}, err
	}
	sector := reference.sectors[block]
	for index := 0; index < persisted; index++ {
		sector[index] = data[index]
	}
	if sector == ([sectorBytes]byte{}) {
		delete(reference.sectors, block)
	} else {
		reference.sectors[block] = sector
	}
	reference.state = plan.End
	return WriteResult{Timing: plan, PersistedPrefixBytes: persisted, Acknowledged: acknowledged}, nil
}

func referenceDescribe(cut hddwritecrashcuts.Cut) (int, bool, error) {
	switch cut.Kind {
	case hddwritecrashcuts.BeforeMediaWrite:
		if cut.WrittenPrefix != nil {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return 0, false, nil
	case hddwritecrashcuts.DuringMediaWrite:
		if cut.WrittenPrefix == nil || *cut.WrittenPrefix < 1 || *cut.WrittenPrefix > 7 {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return *cut.WrittenPrefix, false, nil
	case hddwritecrashcuts.AfterMediaWriteBeforeResponse:
		if cut.WrittenPrefix != nil {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return sectorBytes, false, nil
	case hddwritecrashcuts.Acknowledged:
		if cut.WrittenPrefix != nil {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return sectorBytes, true, nil
	default:
		return 0, false, hddwritecrashcuts.ErrInvalidCut
	}
}

// referencePlan advances one cylinder/tick at a time and does not call the
// production seek planner. A byte-reaching write consumes one transfer tick.
func referencePlan(g hddgeometry.Geometry, current hddseektiming.State, block int64, transfer bool) (hddseektiming.Result, error) {
	if !validReferenceGeometry(g) {
		return hddseektiming.Result{}, blockdevicecontract.ErrInvalidGeometry
	}
	count := g.Cylinders * g.Surfaces * g.SectorsPerTrack
	if block < 0 || block >= count {
		return hddseektiming.Result{}, blockdevicecontract.ErrInvalidBlock
	}
	target := hddgeometry.Address{
		Cylinder: (block / g.SectorsPerTrack) / g.Surfaces,
		Surface:  (block / g.SectorsPerTrack) % g.Surfaces,
		Sector:   block % g.SectorsPerTrack,
	}
	if current.HeadCylinder < 0 || current.HeadCylinder >= g.Cylinders {
		return hddseektiming.Result{}, hddseektiming.ErrHeadCylinder
	}
	if current.Tick > referenceMaxTick {
		return hddseektiming.Result{}, hddseektiming.ErrClockOverflow
	}
	state := current
	var seek uint64
	for state.HeadCylinder != target.Cylinder {
		if state.Tick > referenceMaxTick-3 {
			return hddseektiming.Result{}, hddseektiming.ErrClockOverflow
		}
		if state.HeadCylinder < target.Cylinder {
			state.HeadCylinder++
		} else {
			state.HeadCylinder--
		}
		state.Tick += 3
		seek += 3
	}
	var rotation uint64
	for state.Tick%uint64(g.SectorsPerTrack) != uint64(target.Sector) {
		if state.Tick == referenceMaxTick {
			return hddseektiming.Result{}, hddseektiming.ErrClockOverflow
		}
		state.Tick++
		rotation++
	}
	transferTicks := uint64(0)
	if transfer {
		if state.Tick == referenceMaxTick {
			return hddseektiming.Result{}, hddseektiming.ErrClockOverflow
		}
		state.Tick++
		transferTicks = 1
	}
	total := seek + rotation + transferTicks
	return hddseektiming.Result{
		Target: target, End: state, SeekTicks: seek, RotationTicks: rotation,
		TransferTicks: transferTicks, TotalTicks: total,
	}, nil
}

func testCuts() []hddwritecrashcuts.Cut {
	cuts := []hddwritecrashcuts.Cut{{Kind: hddwritecrashcuts.BeforeMediaWrite}}
	for prefix := 1; prefix < sectorBytes; prefix++ {
		value := prefix
		cuts = append(cuts, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value})
	}
	return append(cuts,
		hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse},
		hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged},
	)
}

func patternedData(block int64, step int) []byte {
	data := make([]byte, sectorBytes)
	for index := range data {
		data[index] = byte((int(block)*31+step*11+index*17)%251 + 1)
	}
	return data
}

func compareSnapshot(t testing.TB, actual *Medium, model *referenceMedium) {
	t.Helper()
	got, err := actual.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	want := Snapshot{Geometry: model.geometry, State: model.state, Sectors: model.sectors}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("snapshot got=%+v want=%+v", got, want)
	}
}

func TestBoth512GeometriesMatchSteppedByteAndTickModel(t *testing.T) {
	profiles := []hddgeometry.Geometry{
		{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16},
		{Cylinders: 64, Surfaces: 2, SectorsPerTrack: 4},
	}
	operations := 0
	for _, geometry := range profiles {
		actual, err := New(geometry)
		if err != nil {
			t.Fatal(err)
		}
		model, err := newReference(geometry)
		if err != nil {
			t.Fatal(err)
		}
		initial, err := actual.Snapshot()
		if err != nil || initial.State != (hddseektiming.State{}) || len(initial.Sectors) != 0 {
			t.Fatalf("new medium snapshot=%+v error=%v", initial, err)
		}
		for block := int64(0); block < geometry.Cylinders*geometry.Surfaces*geometry.SectorsPerTrack; block++ {
			wantRead, modelErr := model.read(block)
			gotRead, actualErr := actual.ReadSector(block)
			if actualErr != nil || modelErr != nil || gotRead != wantRead {
				t.Fatalf("profile=%+v read block=%d got=%+v,%v want=%+v,%v", geometry, block, gotRead, actualErr, wantRead, modelErr)
			}
			compareSnapshot(t, actual, model)
			for cutIndex, cut := range testCuts() {
				data := patternedData(block, cutIndex)
				before := append([]byte(nil), data...)
				want, modelErr := model.write(block, data, cut)
				got, actualErr := actual.WriteSector(block, data, cut)
				if actualErr != nil || modelErr != nil || got != want {
					t.Fatalf("profile=%+v write block=%d cut=%+v got=%+v,%v want=%+v,%v", geometry, block, cut, got, actualErr, want, modelErr)
				}
				if !reflect.DeepEqual(data, before) {
					t.Fatal("write changed caller bytes")
				}
				compareSnapshot(t, actual, model)
				wantRead, modelErr = model.read(block)
				gotRead, actualErr = actual.ReadSector(block)
				if actualErr != nil || modelErr != nil || gotRead != wantRead {
					t.Fatalf("post-cut read profile=%+v block=%d cut=%+v got=%+v,%v want=%+v,%v", geometry, block, cut, gotRead, actualErr, wantRead, modelErr)
				}
				compareSnapshot(t, actual, model)
				operations++
			}
		}
	}
	if operations != 10_240 {
		t.Fatalf("profile write/read transitions=%d", operations)
	}
	t.Logf("independent stepped byte/tick model checked %d write/read pairs", operations)
}

func TestSparseSnapshotsRestartZeroOverwriteAndSecondWrite(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 64, Surfaces: 2, SectorsPerTrack: 4}
	medium, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	data := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	if _, err := medium.WriteSector(77, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}); err != nil {
		t.Fatal(err)
	}
	first, err := medium.Snapshot()
	if err != nil || len(first.Sectors) != 1 {
		t.Fatalf("first snapshot=%+v err=%v", first, err)
	}
	first.Sectors[77] = [sectorBytes]byte{}
	delete(first.Sectors, 77)
	second, err := medium.Snapshot()
	if err != nil || second.Sectors[77] != [sectorBytes]byte{1, 2, 3, 4, 5, 6, 7, 8} || len(second.Sectors) != 1 {
		t.Fatalf("snapshot was not detached: %+v %v", second, err)
	}
	mutated := second.Sectors[77]
	mutated[0] = 99
	second.Sectors[77] = mutated
	read, err := medium.ReadSector(77)
	if err != nil || read.Data != [sectorBytes]byte{1, 2, 3, 4, 5, 6, 7, 8} {
		t.Fatalf("snapshot mutation escaped: %+v %v", read, err)
	}
	zeroes := make([]byte, sectorBytes)
	if _, err := medium.WriteSector(77, zeroes, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}); err != nil {
		t.Fatal(err)
	}
	empty, err := medium.Snapshot()
	if err != nil || len(empty.Sectors) != 0 {
		t.Fatalf("zero overwrite was not removed from sparse map: %+v %v", empty, err)
	}
	if _, err := medium.WriteSector(77, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}); err != nil {
		t.Fatal(err)
	}
	preRestart, _ := medium.Snapshot()
	restarted, err := medium.Restart()
	if err != nil {
		t.Fatal(err)
	}
	afterRestart, _ := restarted.Snapshot()
	if afterRestart.State != (hddseektiming.State{}) || !reflect.DeepEqual(afterRestart.Sectors, preRestart.Sectors) {
		t.Fatalf("restart state/data=%+v, want reset clock and preserved sparse data", afterRestart)
	}
	if _, err := restarted.WriteSector(78, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}); err != nil {
		t.Fatal(err)
	}
	original, _ := medium.Snapshot()
	independent, _ := restarted.Snapshot()
	if len(original.Sectors) != 1 || len(independent.Sectors) != 2 {
		t.Fatalf("restart medium shared state: original=%v restart=%v", original.Sectors, independent.Sectors)
	}
	if _, err := restarted.Restart(); err != nil {
		t.Fatal(err)
	}
}

func TestZeroOverwriteAcrossEveryValidCut(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 4, Surfaces: 2, SectorsPerTrack: 4}
	actual, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	model, _ := newReference(geometry)
	old := []byte{0x91, 0x82, 0x73, 0x64, 0x55, 0x46, 0x37, 0x28}
	seed := hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}
	if want, modelErr := model.write(5, old, seed); modelErr != nil {
		t.Fatal(modelErr)
	} else if got, actualErr := actual.WriteSector(5, old, seed); actualErr != nil || got != want {
		t.Fatalf("seed write=(%+v,%v), want %+v", got, actualErr, want)
	}
	zeroes := make([]byte, sectorBytes)
	for index, cut := range testCuts() {
		want, modelErr := model.write(5, zeroes, cut)
		got, actualErr := actual.WriteSector(5, zeroes, cut)
		if actualErr != nil || modelErr != nil || got != want {
			t.Fatalf("zero overwrite cut %d (%+v)=(%+v,%v), want %+v,%v", index, cut, got, actualErr, want, modelErr)
		}
		compareSnapshot(t, actual, model)
		wantRead, modelErr := model.read(5)
		gotRead, actualErr := actual.ReadSector(5)
		if actualErr != nil || modelErr != nil || gotRead != wantRead {
			t.Fatalf("zero overwrite read cut %d=(%+v,%v), want %+v,%v", index, gotRead, actualErr, wantRead, modelErr)
		}
		compareSnapshot(t, actual, model)
	}
}

func TestHugeSparseGeometryAndTimingCeiling(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: int64(referenceMaxTick)}
	medium, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	initial, err := medium.Snapshot()
	if err != nil || initial.Sectors == nil || len(initial.Sectors) != 0 {
		t.Fatalf("huge geometry allocated/stored media: %+v %v", initial, err)
	}
	read, err := medium.ReadSector(int64(referenceMaxTick - 2))
	if err != nil || read.Timing.End.Tick != referenceMaxTick-1 || read.Data != [sectorBytes]byte{} {
		t.Fatalf("near-ceiling sparse read=%+v error=%v", read, err)
	}
	data := []byte{9, 8, 7, 6, 5, 4, 3, 2}
	beforeMedia, err := medium.WriteSector(0, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite})
	if !errors.Is(err, hddseektiming.ErrClockOverflow) || beforeMedia != (WriteResult{}) {
		t.Fatalf("full-service ceiling cut=(%+v,%v)", beforeMedia, err)
	}
	snapshot, _ := medium.Snapshot()
	if snapshot.State.Tick != referenceMaxTick-1 || len(snapshot.Sectors) != 0 {
		t.Fatalf("refused before-media cut changed medium: %+v", snapshot)
	}
	if result, err := medium.ReadSector(0); !errors.Is(err, hddseektiming.ErrClockOverflow) || result != (ReadResult{}) {
		t.Fatalf("exhausted read=(%+v,%v)", result, err)
	}
	if result, err := medium.WriteSector(0, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}); !errors.Is(err, hddseektiming.ErrClockOverflow) || result != (WriteResult{}) {
		t.Fatalf("exhausted write=(%+v,%v)", result, err)
	}
	if result, err := medium.WriteSector(0, data, hddwritecrashcuts.Cut{Kind: "unknown"}); !errors.Is(err, hddwritecrashcuts.ErrInvalidCut) || result != (WriteResult{}) {
		t.Fatalf("cut should precede exhausted timing=(%+v,%v)", result, err)
	}
	reset, err := medium.Restart()
	if err != nil {
		t.Fatal(err)
	}
	if written, err := reset.WriteSector(0, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}); err != nil || !written.Acknowledged {
		t.Fatalf("post-restart write=(%+v,%v)", written, err)
	}
	stored, _ := reset.Snapshot()
	if len(stored.Sectors) != 1 || stored.Sectors[0] != [sectorBytes]byte{9, 8, 7, 6, 5, 4, 3, 2} {
		t.Fatalf("sparse write at high geometry=%+v", stored)
	}
	again, err := reset.Restart()
	if err != nil {
		t.Fatal(err)
	}
	read, err = again.ReadSector(0)
	if err != nil || read.Data != stored.Sectors[0] || read.Timing.End.Tick != 1 {
		t.Fatalf("repeat restart read=%+v err=%v", read, err)
	}
}

func TestModuloWrapAndBeforeMediaSeekOverflow(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: 4}
	medium, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := medium.ReadSector(2); err != nil { // leave phase at tick 3
		t.Fatal(err)
	}
	wrapped, err := medium.ReadSector(4) // seek advances phase 3 across period 4
	if err != nil || wrapped.Timing.SeekTicks != 3 || wrapped.Timing.RotationTicks != 2 {
		t.Fatalf("modular phase wrap=(%+v,%v)", wrapped, err)
	}

	maxDistance := int64(referenceMaxTick / 3)
	far := hddgeometry.Geometry{Cylinders: maxDistance + 2, Surfaces: 1, SectorsPerTrack: 1}
	farMedium, err := New(far)
	if err != nil {
		t.Fatal(err)
	}
	result, err := farMedium.WriteSector(maxDistance+1, make([]byte, sectorBytes), hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite})
	if !errors.Is(err, hddseektiming.ErrClockOverflow) || result != (WriteResult{}) {
		t.Fatalf("seek overflow before media=(%+v,%v)", result, err)
	}
	state, _ := farMedium.Snapshot()
	if state.State != (hddseektiming.State{}) || len(state.Sectors) != 0 {
		t.Fatalf("seek overflow mutated medium: %+v", state)
	}
}

func TestBeforeMediaRequiresFullServiceAdmission(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: 2}
	medium, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	// Model a late point in a long serialized operation history.
	medium.state = hddseektiming.State{HeadCylinder: 0, Tick: referenceMaxTick - 1}
	result, err := medium.WriteSector(1, make([]byte, sectorBytes), hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite})
	if !errors.Is(err, hddseektiming.ErrClockOverflow) || result != (WriteResult{}) {
		t.Fatalf("full service overflow=(%+v,%v)", result, err)
	}
	state, err := medium.Snapshot()
	if err != nil || state.State.Tick != referenceMaxTick-1 || len(state.Sectors) != 0 {
		t.Fatalf("refused before-media write mutated medium: %+v,%v", state, err)
	}
}

func TestRefusalPrecedenceNilAndZeroReceivers(t *testing.T) {
	if _, err := New(hddgeometry.Geometry{}); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("invalid constructor error=%v", err)
	}
	var nilMedium *Medium
	if got, err := nilMedium.ReadSector(-1); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (ReadResult{}) {
		t.Fatalf("nil read=(%+v,%v)", got, err)
	}
	if got, err := nilMedium.WriteSector(-1, nil, hddwritecrashcuts.Cut{Kind: "unknown"}); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (WriteResult{}) {
		t.Fatalf("nil write=(%+v,%v)", got, err)
	}
	if got, err := nilMedium.Snapshot(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || !reflect.DeepEqual(got, Snapshot{}) {
		t.Fatalf("nil snapshot=(%+v,%v)", got, err)
	}
	if got, err := nilMedium.Restart(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != nil {
		t.Fatalf("nil restart=(%v,%v)", got, err)
	}
	zero := &Medium{}
	if _, err := zero.ReadSector(0); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("zero receiver read error=%v", err)
	}
	if _, err := zero.WriteSector(0, make([]byte, 8), hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("zero receiver write error=%v", err)
	}
	if _, err := zero.Snapshot(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("zero receiver snapshot error=%v", err)
	}
	if restarted, err := zero.Restart(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || restarted != nil {
		t.Fatalf("zero receiver restart=(%v,%v)", restarted, err)
	}
	medium, err := New(hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: 4})
	if err != nil {
		t.Fatal(err)
	}
	data := make([]byte, sectorBytes)
	badCut := hddwritecrashcuts.Cut{Kind: "unknown"}
	if _, err := medium.WriteSector(-1, nil, badCut); !errors.Is(err, blockdevicecontract.ErrInvalidBlock) {
		t.Fatalf("address should precede payload/cut, got %v", err)
	}
	if _, err := medium.WriteSector(0, data[:7], badCut); !errors.Is(err, blockdevicecontract.ErrInvalidBlockData) {
		t.Fatalf("payload should precede cut, got %v", err)
	}
	if _, err := medium.WriteSector(0, data, badCut); !errors.Is(err, hddwritecrashcuts.ErrInvalidCut) {
		t.Fatalf("cut refusal=%v", err)
	}
	if _, err := medium.WriteSector(0, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite}); !errors.Is(err, hddwritecrashcuts.ErrInvalidPrefix) {
		t.Fatalf("prefix refusal=%v", err)
	}
	for _, prefix := range []int{0, 8, math.MinInt, math.MaxInt} {
		if _, err := medium.WriteSector(0, data, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}); !errors.Is(err, hddwritecrashcuts.ErrInvalidPrefix) {
			t.Fatalf("invalid prefix %d error=%v", prefix, err)
		}
	}
	if got, err := medium.ReadSector(math.MinInt64); !errors.Is(err, blockdevicecontract.ErrInvalidBlock) || got != (ReadResult{}) {
		t.Fatalf("signed read address=(%+v,%v)", got, err)
	}
}

func TestFixedSeededHistoryAgainstIndependentModel(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	actual, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	model, _ := newReference(geometry)
	random := rand.New(rand.NewSource(0x6d06b2d3))
	cuts := testCuts()
	for operation := 0; operation < 10_000; operation++ {
		block := int64(random.Intn(512))
		if random.Intn(4) == 0 {
			want, modelErr := model.read(block)
			got, actualErr := actual.ReadSector(block)
			if actualErr != nil || modelErr != nil || got != want {
				t.Fatalf("seeded read %d: got=%+v,%v want=%+v,%v", operation, got, actualErr, want, modelErr)
			}
		} else {
			data := patternedData(block, operation)
			cut := cuts[random.Intn(len(cuts))]
			want, modelErr := model.write(block, data, cut)
			got, actualErr := actual.WriteSector(block, data, cut)
			if actualErr != nil || modelErr != nil || got != want {
				t.Fatalf("seeded write %d: got=%+v,%v want=%+v,%v", operation, got, actualErr, want, modelErr)
			}
		}
		compareSnapshot(t, actual, model)
	}
	t.Log("independent deterministic history checked 10000 operations")
}

func FuzzTimedMediaHistory(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9})
	f.Add([]byte{0xff, 0, 0x40, 0x22, 0x18, 0x91})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 128 {
			return
		}
		geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
		actual, err := New(geometry)
		if err != nil {
			t.Fatal(err)
		}
		model, _ := newReference(geometry)
		cuts := testCuts()
		for index, command := range commands {
			block := int64(command) % 512
			if command&1 == 0 {
				want, modelErr := model.read(block)
				got, actualErr := actual.ReadSector(block)
				if actualErr != nil || modelErr != nil || got != want {
					t.Fatalf("fuzz read %d got=%+v,%v want=%+v,%v", index, got, actualErr, want, modelErr)
				}
			} else {
				data := patternedData(block, index+int(command))
				cut := cuts[int(command)%len(cuts)]
				want, modelErr := model.write(block, data, cut)
				got, actualErr := actual.WriteSector(block, data, cut)
				if actualErr != nil || modelErr != nil || got != want {
					t.Fatalf("fuzz write %d got=%+v,%v want=%+v,%v", index, got, actualErr, want, modelErr)
				}
			}
			compareSnapshot(t, actual, model)
		}
	})
}
