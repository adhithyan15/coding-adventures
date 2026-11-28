package hddbarrierplan

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
	hddseektiming "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-seek-timing"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
)

const modelMaxTick uint64 = 9_007_199_254_740_991

func descriptorModel(dirty []int64, cut hddwritebackcache.BarrierCut) (int, error) {
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector, hddwritebackcache.AfterAllMedia, hddwritebackcache.BarrierComplete:
		if cut.SectorIndex != 0 || cut.MediaCut != (hddwritecrashcuts.Cut{}) {
			return 0, hddwritebackcache.ErrInvalidBarrierCut
		}
		return 0, nil
	case hddwritebackcache.AtSector:
		if cut.SectorIndex < 0 || cut.SectorIndex >= int64(len(dirty)) {
			return 0, hddwritebackcache.ErrInvalidBarrierCut
		}
		if cut.MediaCut.Kind == hddwritecrashcuts.Acknowledged {
			return 0, hddwritebackcache.ErrInvalidBarrierCut
		}
		switch cut.MediaCut.Kind {
		case hddwritecrashcuts.BeforeMediaWrite, hddwritecrashcuts.AfterMediaWriteBeforeResponse:
			if cut.MediaCut.WrittenPrefix != nil {
				return 0, hddwritecrashcuts.ErrInvalidPrefix
			}
			if cut.MediaCut.Kind == hddwritecrashcuts.BeforeMediaWrite {
				return 0, nil
			}
			return hddwritecrashcuts.BytesPerSector, nil
		case hddwritecrashcuts.DuringMediaWrite:
			if cut.MediaCut.WrittenPrefix == nil || *cut.MediaCut.WrittenPrefix < 1 || *cut.MediaCut.WrittenPrefix > hddwritecrashcuts.BytesPerSector-1 {
				return 0, hddwritecrashcuts.ErrInvalidPrefix
			}
			return *cut.MediaCut.WrittenPrefix, nil
		default:
			return 0, hddwritecrashcuts.ErrInvalidCut
		}
	default:
		return 0, hddwritebackcache.ErrInvalidBarrierCut
	}
}

func steppedPlan(g hddgeometry.Geometry, current hddseektiming.State, block int64) (hddseektiming.Result, error) {
	if g.Cylinders <= 0 || g.Surfaces <= 0 || g.SectorsPerTrack <= 0 {
		return hddseektiming.Result{}, blockdevicecontract.ErrInvalidGeometry
	}
	if g.Cylinders > int64(modelMaxTick)/g.Surfaces || g.Cylinders*g.Surfaces > int64(modelMaxTick)/g.SectorsPerTrack {
		return hddseektiming.Result{}, blockdevicecontract.ErrInvalidGeometry
	}
	count := g.Cylinders * g.Surfaces * g.SectorsPerTrack
	if block < 0 || block >= count {
		return hddseektiming.Result{}, blockdevicecontract.ErrInvalidBlock
	}
	target := hddgeometry.Address{Cylinder: (block / g.SectorsPerTrack) / g.Surfaces, Surface: (block / g.SectorsPerTrack) % g.Surfaces, Sector: block % g.SectorsPerTrack}
	if current.HeadCylinder < 0 || current.HeadCylinder >= g.Cylinders {
		return hddseektiming.Result{}, hddseektiming.ErrHeadCylinder
	}
	if current.Tick > modelMaxTick {
		return hddseektiming.Result{}, hddseektiming.ErrClockOverflow
	}
	state := current
	var seek uint64
	for state.HeadCylinder != target.Cylinder {
		if state.Tick > modelMaxTick-3 {
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
		if state.Tick == modelMaxTick {
			return hddseektiming.Result{}, hddseektiming.ErrClockOverflow
		}
		state.Tick++
		rotation++
	}
	if state.Tick == modelMaxTick {
		return hddseektiming.Result{}, hddseektiming.ErrClockOverflow
	}
	state.Tick++
	total := seek + rotation + 1
	return hddseektiming.Result{Target: target, End: state, SeekTicks: seek, RotationTicks: rotation, TransferTicks: 1, TotalTicks: total}, nil
}

func referencePrepare(g hddgeometry.Geometry, current hddseektiming.State, dirty []int64, cut hddwritebackcache.BarrierCut) (Plan, error) {
	if !validModelGeometry(g) {
		return Plan{}, blockdevicecontract.ErrInvalidGeometry
	}
	count := g.Cylinders * g.Surfaces * g.SectorsPerTrack
	for index, block := range dirty {
		if block < 0 || block >= count {
			return Plan{}, blockdevicecontract.ErrInvalidBlock
		}
		if index > 0 && block <= dirty[index-1] {
			return Plan{}, ErrDirtyOrder
		}
	}
	prefix, err := descriptorModel(dirty, cut)
	if err != nil {
		return Plan{}, err
	}
	stepCount := len(dirty)
	receipt := hddwritebackcache.BarrierReceipt{}
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector:
		return Plan{}, nil
	case hddwritebackcache.AtSector:
		stepCount = int(cut.SectorIndex) + 1
		receipt.PriorCompletedSectors = int(cut.SectorIndex)
		receipt.TerminalPrefixBytes = prefix
	case hddwritebackcache.AfterAllMedia:
		receipt.PriorCompletedSectors = len(dirty)
	case hddwritebackcache.BarrierComplete:
		receipt.PriorCompletedSectors = len(dirty)
		receipt.Acknowledged = true
	}
	if stepCount == 0 {
		return Plan{Receipt: receipt}, nil
	}
	steps := make([]Step, stepCount)
	state := current
	for ordinal := 0; ordinal < stepCount; ordinal++ {
		mediaCut := hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}
		if cut.Kind == hddwritebackcache.AtSector && ordinal == int(cut.SectorIndex) {
			mediaCut = cut.MediaCut
			if mediaCut.WrittenPrefix != nil {
				value := *mediaCut.WrittenPrefix
				mediaCut.WrittenPrefix = &value
			}
		}
		steps[ordinal] = Step{Block: dirty[ordinal], Cut: mediaCut}
		plan, err := steppedPlan(g, state, dirty[ordinal])
		if err != nil {
			return Plan{}, err
		}
		state = plan.End
	}
	return Plan{Steps: steps, Receipt: receipt}, nil
}

func validModelGeometry(g hddgeometry.Geometry) bool {
	if g.Cylinders <= 0 || g.Surfaces <= 0 || g.SectorsPerTrack <= 0 || g.Cylinders > int64(modelMaxTick)/g.Surfaces {
		return false
	}
	return g.Cylinders*g.Surfaces <= int64(modelMaxTick)/g.SectorsPerTrack
}

func duringCut(prefix int) hddwritecrashcuts.Cut {
	return hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}
}

func validBarrierCuts(count int) []hddwritebackcache.BarrierCut {
	cuts := []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BeforeFirstSector}, {Kind: hddwritebackcache.AfterAllMedia}, {Kind: hddwritebackcache.BarrierComplete}}
	for ordinal := 0; ordinal < count; ordinal++ {
		for _, media := range []hddwritecrashcuts.Cut{{Kind: hddwritecrashcuts.BeforeMediaWrite}, {Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}} {
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: media})
		}
		for prefix := 1; prefix <= hddwritecrashcuts.BytesPerSector-1; prefix++ {
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: duringCut(prefix)})
		}
	}
	return cuts
}

func TestAllSmallDirtyMasksAndEveryBarrierCutMatchModel(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: 4}
	cases := 0
	for mask := 0; mask < 1<<8; mask++ {
		dirty := make([]int64, 0, 8)
		for block := int64(0); block < 8; block++ {
			if mask&(1<<block) != 0 {
				dirty = append(dirty, block)
			}
		}
		for _, cut := range validBarrierCuts(len(dirty)) {
			want, wantErr := referencePrepare(geometry, hddseektiming.State{HeadCylinder: int64(mask % 2), Tick: uint64(mask % 7)}, dirty, cut)
			got, gotErr := Prepare(geometry, hddseektiming.State{HeadCylinder: int64(mask % 2), Tick: uint64(mask % 7)}, dirty, cut)
			if wantErr != nil || gotErr != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("mask=%02x dirty=%v cut=%+v got=(%+v,%v) want=(%+v,%v)", mask, dirty, cut, got, gotErr, want, wantErr)
			}
			cases++
		}
	}
	t.Logf("independent mask/cut oracle checked %d plans", cases)
}

func TestEmptyBarrierFamiliesAndBeforeFirstSkipsTiming(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: 4}
	invalidState := hddseektiming.State{HeadCylinder: 0, Tick: modelMaxTick + 1}
	for _, cut := range []hddwritebackcache.BarrierCut{{Kind: hddwritebackcache.BeforeFirstSector}, {Kind: hddwritebackcache.AfterAllMedia}, {Kind: hddwritebackcache.BarrierComplete}} {
		got, err := Prepare(geometry, invalidState, nil, cut)
		if err != nil || got.Steps != nil {
			t.Fatalf("empty cut %+v=(%+v,%v)", cut, got, err)
		}
		if got.Receipt.Acknowledged != (cut.Kind == hddwritebackcache.BarrierComplete) || got.Receipt.PriorCompletedSectors != 0 || got.Receipt.TerminalPrefixBytes != 0 {
			t.Fatalf("empty receipt %+v for %+v", got.Receipt, cut)
		}
	}
	if got, err := Prepare(geometry, hddseektiming.State{}, nil, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector}); !errors.Is(err, hddwritebackcache.ErrInvalidBarrierCut) || !reflect.DeepEqual(got, Plan{}) {
		t.Fatalf("empty AtSector=(%+v,%v)", got, err)
	}
}

func TestPrecedenceTypedLowerErrorsAndZeroPlan(t *testing.T) {
	goodGeometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: 4}
	badGeometry := hddgeometry.Geometry{}
	unknown := hddwritebackcache.BarrierCut{Kind: 255}
	for _, test := range []struct {
		name     string
		geometry hddgeometry.Geometry
		dirty    []int64
		cut      hddwritebackcache.BarrierCut
		want     error
	}{
		{"geometry before block and descriptor", badGeometry, []int64{-1}, unknown, blockdevicecontract.ErrInvalidGeometry},
		{"block before descriptor", goodGeometry, []int64{-1}, unknown, blockdevicecontract.ErrInvalidBlock},
		{"block before ordering", goodGeometry, []int64{3, -1}, unknown, blockdevicecontract.ErrInvalidBlock},
		{"descending dirty blocks", goodGeometry, []int64{3, 2}, unknown, ErrDirtyOrder},
		{"duplicate dirty block", goodGeometry, []int64{2, 2}, unknown, ErrDirtyOrder},
		{"negative terminal ordinal", goodGeometry, []int64{1}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: -1, MediaCut: hddwritecrashcuts.Cut{Kind: "unknown"}}, hddwritebackcache.ErrInvalidBarrierCut},
		{"ordinal outside set", goodGeometry, []int64{1}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 1}, hddwritebackcache.ErrInvalidBarrierCut},
		{"ack tag before malformed prefix", goodGeometry, []int64{1}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged, WrittenPrefix: intPointer(0)}}, hddwritebackcache.ErrInvalidBarrierCut},
		{"lower unknown kind", goodGeometry, []int64{1}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, MediaCut: hddwritecrashcuts.Cut{Kind: "unknown"}}, hddwritecrashcuts.ErrInvalidCut},
		{"lower invalid prefix", goodGeometry, []int64{1}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, MediaCut: duringCut(0)}, hddwritecrashcuts.ErrInvalidPrefix},
		{"lower unexpected prefix", goodGeometry, []int64{1}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse, WrittenPrefix: intPointer(1)}}, hddwritecrashcuts.ErrInvalidPrefix},
		{"descriptor before exhausted clock", goodGeometry, []int64{1}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, MediaCut: duringCut(0)}, hddwritecrashcuts.ErrInvalidPrefix},
	} {
		t.Run(test.name, func(t *testing.T) {
			current := hddseektiming.State{HeadCylinder: 0, Tick: modelMaxTick + 1}
			got, err := Prepare(test.geometry, current, test.dirty, test.cut)
			if !errors.Is(err, test.want) || !reflect.DeepEqual(got, Plan{}) {
				t.Fatalf("Prepare=(%+v,%v), want zero plan and %v", got, err, test.want)
			}
		})
	}
}

func intPointer(value int) *int { return &value }

func TestTimingExhaustionOnLaterPlannedBlockIsAtomic(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: 4}
	current := hddseektiming.State{HeadCylinder: 0, Tick: modelMaxTick - 1}
	cut := hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AfterAllMedia}
	got, err := Prepare(geometry, current, []int64{2, 3}, cut)
	if !errors.Is(err, hddseektiming.ErrClockOverflow) || !reflect.DeepEqual(got, Plan{}) {
		t.Fatalf("later timing refusal=(%+v,%v)", got, err)
	}
	firstOnly, err := Prepare(geometry, current, []int64{2}, cut)
	if err != nil || len(firstOnly.Steps) != 1 {
		t.Fatalf("first block should fit exactly: (%+v,%v)", firstOnly, err)
	}
	terminal, err := Prepare(geometry, current, []int64{2, 3}, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 1, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}})
	if !errors.Is(err, hddseektiming.ErrClockOverflow) || !reflect.DeepEqual(terminal, Plan{}) {
		t.Fatalf("terminal full-service admission=(%+v,%v)", terminal, err)
	}
}

func TestTerminalPrefixPointerIsDetached(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: 4}
	prefix := 5
	cut := hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 1, MediaCut: duringCut(prefix)}
	plan, err := Prepare(geometry, hddseektiming.State{}, []int64{1, 6}, cut)
	if err != nil || len(plan.Steps) != 2 || plan.Receipt.PriorCompletedSectors != 1 || plan.Receipt.TerminalPrefixBytes != 5 || plan.Receipt.Acknowledged {
		t.Fatalf("terminal plan=(%+v,%v)", plan, err)
	}
	if plan.Steps[1].Cut.WrittenPrefix == nil || *plan.Steps[1].Cut.WrittenPrefix != 5 {
		t.Fatalf("terminal cut=%+v", plan.Steps[1].Cut)
	}
	prefix = 2
	if *plan.Steps[1].Cut.WrittenPrefix != 5 {
		t.Fatal("plan retained caller prefix pointer")
	}
	*plan.Steps[1].Cut.WrittenPrefix = 7
	if prefix != 2 {
		t.Fatal("caller prefix aliases returned plan pointer")
	}
}

func TestFixedSeededPlansMatchIndependentModel(t *testing.T) {
	random := rand.New(rand.NewSource(0x6d06b2d3c1))
	geometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: 4}
	for iteration := 0; iteration < 5_000; iteration++ {
		dirty := make([]int64, 0, 4)
		for block := int64(0); block < 8; block++ {
			if random.Intn(2) == 0 {
				dirty = append(dirty, block)
			}
		}
		cuts := validBarrierCuts(len(dirty))
		cut := cuts[random.Intn(len(cuts))]
		current := hddseektiming.State{HeadCylinder: int64(random.Intn(2)), Tick: uint64(random.Intn(32))}
		want, wantErr := referencePrepare(geometry, current, dirty, cut)
		got, gotErr := Prepare(geometry, current, dirty, cut)
		if (wantErr == nil) != (gotErr == nil) || wantErr != nil && !errors.Is(gotErr, wantErr) || !reflect.DeepEqual(got, want) {
			t.Fatalf("seeded %d dirty=%v cut=%+v got=(%+v,%v) want=(%+v,%v)", iteration, dirty, cut, got, gotErr, want, wantErr)
		}
	}
}

func FuzzBarrierPrepare(f *testing.F) {
	f.Add([]byte{0x95, 1, 3, 7, 2, 0})
	f.Add([]byte{0xff, 0, 4, 8, 9, 11, 1})
	f.Add([]byte{0x33, 3, 2, 1, 0, 7})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 128 {
			return
		}
		geometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: 4}
		dirty := make([]int64, 0, 8)
		if len(commands) > 0 {
			mask := commands[0]
			for block := int64(0); block < 8; block++ {
				if mask&(1<<block) != 0 {
					dirty = append(dirty, block)
				}
			}
		}
		kind := hddwritebackcache.BarrierCutKind(0)
		ordinal := int64(0)
		prefix := 1
		cut := hddwritebackcache.BarrierCut{}
		if len(commands) > 1 {
			kind = hddwritebackcache.BarrierCutKind(commands[1] % 5)
			cut.Kind = kind
		}
		if len(commands) > 2 {
			ordinal = int64(int8(commands[2]))
			cut.SectorIndex = ordinal
		}
		if len(commands) > 3 {
			prefix = int(int8(commands[3]))
		}
		if len(commands) > 4 {
			lowerKind := commands[4] % 5
			switch lowerKind {
			case 0:
				cut.MediaCut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}
			case 1:
				cut.MediaCut = duringCut(prefix)
			case 2:
				cut.MediaCut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}
			case 3:
				cut.MediaCut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}
			default:
				cut.MediaCut = hddwritecrashcuts.Cut{Kind: "unknown"}
			}
		}
		current := hddseektiming.State{HeadCylinder: 0, Tick: uint64(len(commands))}
		want, wantErr := referencePrepare(geometry, current, dirty, cut)
		got, gotErr := Prepare(geometry, current, dirty, cut)
		if (wantErr == nil) != (gotErr == nil) || wantErr != nil && !errors.Is(gotErr, wantErr) || !reflect.DeepEqual(got, want) {
			t.Fatalf("fuzz dirty=%v cut=%+v got=(%+v,%v) want=(%+v,%v)", dirty, cut, got, gotErr, want, wantErr)
		}
	})
}
