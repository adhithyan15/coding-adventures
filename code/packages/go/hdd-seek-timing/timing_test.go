package hddseektiming

import (
	"encoding/json"
	"errors"
	"math"
	"math/big"
	"os"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
)

const maxTick uint64 = 9_007_199_254_740_991

var resultSink Result

type timingVectorFile struct {
	Vectors []struct {
		Name            string `json:"name"`
		Cylinders       int64  `json:"cylinders"`
		Surfaces        int64  `json:"surfaces"`
		SectorsPerTrack int64  `json:"sectors_per_track"`
		Head            int64  `json:"head"`
		Tick            uint64 `json:"tick"`
		Block           int64  `json:"block"`
		Target          struct {
			Cylinder int64 `json:"cylinder"`
			Surface  int64 `json:"surface"`
			Sector   int64 `json:"sector"`
		} `json:"target"`
		Seek     uint64 `json:"seek"`
		Rotation uint64 `json:"rotation"`
		Transfer uint64 `json:"transfer"`
		Total    uint64 `json:"total"`
		EndHead  int64  `json:"end_head"`
		EndTick  uint64 `json:"end_tick"`
	} `json:"vectors"`
}

func TestPortableTimingVectors(t *testing.T) {
	data, err := os.ReadFile("testdata/timing-vectors.json")
	if err != nil {
		t.Fatal(err)
	}
	var file timingVectorFile
	if err := json.Unmarshal(data, &file); err != nil {
		t.Fatal(err)
	}
	if len(file.Vectors) != 5 {
		t.Fatalf("vector count=%d", len(file.Vectors))
	}
	for _, vector := range file.Vectors {
		t.Run(vector.Name, func(t *testing.T) {
			geometry := hddgeometry.Geometry{Cylinders: vector.Cylinders, Surfaces: vector.Surfaces, SectorsPerTrack: vector.SectorsPerTrack}
			got, err := Plan(geometry, State{HeadCylinder: vector.Head, Tick: vector.Tick}, vector.Block)
			want := Result{
				Target:    hddgeometry.Address{Cylinder: vector.Target.Cylinder, Surface: vector.Target.Surface, Sector: vector.Target.Sector},
				End:       State{HeadCylinder: vector.EndHead, Tick: vector.EndTick},
				SeekTicks: vector.Seek, RotationTicks: vector.Rotation, TransferTicks: vector.Transfer, TotalTicks: vector.Total,
			}
			if err != nil || got != want {
				t.Fatalf("Plan=(%+v,%v) want %+v", got, err, want)
			}
		})
	}
}

func TestFixedDB02AllHeadPhaseBlocks(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 4, Surfaces: 2, SectorsPerTrack: 4}
	checked := 0
	for head := int64(0); head < geometry.Cylinders; head++ {
		for phase := uint64(0); phase < uint64(geometry.SectorsPerTrack); phase++ {
			for block := int64(0); block < 32; block++ {
				current := State{HeadCylinder: head, Tick: phase}
				want, expectedErr := stepModel(geometry, current, block)
				got, err := Plan(geometry, current, block)
				if err != nil || expectedErr != nil || got != want {
					t.Fatalf("fixed profile head=%d phase=%d block=%d: got %+v,%v want %+v,%v", head, phase, block, got, err, want, expectedErr)
				}
				checked++
			}
		}
	}
	if checked != 512 {
		t.Fatalf("fixed timing states=%d", checked)
	}
}

func TestBoth512ProfilesMatchIndependentTickStepModel(t *testing.T) {
	profiles := []hddgeometry.Geometry{
		{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16},
		{Cylinders: 64, Surfaces: 2, SectorsPerTrack: 4},
	}
	checked := 0
	for _, geometry := range profiles {
		count := geometry.Cylinders * geometry.Surfaces * geometry.SectorsPerTrack
		for head := int64(0); head < geometry.Cylinders; head++ {
			for phase := uint64(0); phase < uint64(geometry.SectorsPerTrack); phase++ {
				current := State{HeadCylinder: head, Tick: phase + uint64(geometry.SectorsPerTrack)*3}
				for block := int64(0); block < count; block++ {
					want, expectedErr := stepModel(geometry, current, block)
					got, err := Plan(geometry, current, block)
					if err != nil || expectedErr != nil || got != want {
						t.Fatalf("profile=%+v head=%d phase=%d block=%d got=%+v,%v want=%+v,%v", geometry, head, phase, block, got, err, want, expectedErr)
					}
					checked++
				}
			}
		}
	}
	if checked != 262_144 {
		t.Fatalf("512-profile timing decisions=%d", checked)
	}
	t.Logf("tick-step comparisons across both 512-sector profiles: %d", checked)
}

// stepModel is an independent tick-at-a-time oracle for bounded geometries.
func stepModel(geometry hddgeometry.Geometry, current State, block int64) (Result, error) {
	if geometry.Cylinders <= 0 || geometry.Surfaces <= 0 || geometry.SectorsPerTrack <= 0 {
		return Result{}, blockdevicecontract.ErrInvalidGeometry
	}
	count := geometry.Cylinders * geometry.Surfaces * geometry.SectorsPerTrack
	if count > 9_007_199_254_740_991 {
		return Result{}, blockdevicecontract.ErrInvalidGeometry
	}
	if block < 0 || block >= count {
		return Result{}, blockdevicecontract.ErrInvalidBlock
	}
	target := hddgeometry.Address{
		Cylinder: (block / geometry.SectorsPerTrack) / geometry.Surfaces,
		Surface:  (block / geometry.SectorsPerTrack) % geometry.Surfaces,
		Sector:   block % geometry.SectorsPerTrack,
	}
	if current.HeadCylinder < 0 || current.HeadCylinder >= geometry.Cylinders {
		return Result{}, ErrHeadCylinder
	}
	if current.Tick > maxTick {
		return Result{}, ErrClockOverflow
	}
	state := current
	seek := uint64(0)
	for state.HeadCylinder != target.Cylinder {
		state.HeadCylinder++
		if state.HeadCylinder > target.Cylinder {
			state.HeadCylinder -= 2
		}
		state.Tick += seekTicksPerCylinder
		seek += seekTicksPerCylinder
	}
	rotation := uint64(0)
	for state.Tick%uint64(geometry.SectorsPerTrack) != uint64(target.Sector) {
		state.Tick++
		rotation++
	}
	state.Tick++
	total := seek + rotation + 1
	if state.Tick > maxTick {
		return Result{}, ErrClockOverflow
	}
	return Result{Target: target, End: state, SeekTicks: seek, RotationTicks: rotation, TransferTicks: 1, TotalTicks: total}, nil
}

func TestValidationOrderAndZeroResults(t *testing.T) {
	invalid := hddgeometry.Geometry{}
	if got, err := Plan(invalid, State{HeadCylinder: -1, Tick: maxTick + 1}, -1); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (Result{}) {
		t.Fatalf("geometry error order=(%+v,%v)", got, err)
	}
	overLimit := hddgeometry.Geometry{Cylinders: 9_007_199_254_740_991, Surfaces: 1, SectorsPerTrack: 2}
	if got, err := Plan(overLimit, State{HeadCylinder: -1, Tick: maxTick + 1}, 0); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) || got != (Result{}) {
		t.Fatalf("over-limit geometry error order=(%+v,%v)", got, err)
	}
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	if got, err := Plan(geometry, State{HeadCylinder: -1, Tick: maxTick + 1}, -1); !errors.Is(err, blockdevicecontract.ErrInvalidBlock) || got != (Result{}) {
		t.Fatalf("block error order=(%+v,%v)", got, err)
	}
	if got, err := Plan(geometry, State{HeadCylinder: -1, Tick: maxTick + 1}, 0); !errors.Is(err, ErrHeadCylinder) || got != (Result{}) {
		t.Fatalf("head error order=(%+v,%v)", got, err)
	}
	for _, head := range []int64{-1, geometry.Cylinders, math.MinInt64, math.MaxInt64} {
		if got, err := Plan(geometry, State{HeadCylinder: head}, 0); !errors.Is(err, ErrHeadCylinder) || got != (Result{}) {
			t.Fatalf("head %d=(%+v,%v)", head, got, err)
		}
	}
	for _, tick := range []uint64{maxTick + 1, math.MaxUint64} {
		if got, err := Plan(geometry, State{HeadCylinder: 0, Tick: tick}, 0); !errors.Is(err, ErrClockOverflow) || got != (Result{}) {
			t.Fatalf("tick %d=(%+v,%v)", tick, got, err)
		}
	}
}

func TestOneTickOverflowRefusesAtomically(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: 1}
	got, err := Plan(geometry, State{HeadCylinder: 0, Tick: maxTick}, 0)
	if !errors.Is(err, ErrClockOverflow) || got != (Result{}) {
		t.Fatalf("one-tick overflow=(%+v,%v)", got, err)
	}
}

func TestPortableClockBoundariesAndWidePeriods(t *testing.T) {
	unit := hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: 1}
	atLimit, err := Plan(unit, State{HeadCylinder: 0, Tick: maxTick - 1}, 0)
	if err != nil || atLimit.TotalTicks != 1 || atLimit.End.Tick != maxTick {
		t.Fatalf("exact max tick=(%+v,%v)", atLimit, err)
	}

	fixed := hddgeometry.Geometry{Cylinders: 4, Surfaces: 2, SectorsPerTrack: 4}
	rotationOverflow, err := Plan(fixed, State{HeadCylinder: 0, Tick: maxTick - 1}, 3)
	if !errors.Is(err, ErrClockOverflow) || rotationOverflow != (Result{}) {
		t.Fatalf("rotation overflow=(%+v,%v)", rotationOverflow, err)
	}

	const maximumDistance = int64(maxTick / seekTicksPerCylinder)
	seekGeometry := hddgeometry.Geometry{Cylinders: maximumDistance + 1, Surfaces: 1, SectorsPerTrack: 1}
	seekLimit, err := Plan(seekGeometry, State{HeadCylinder: 0}, maximumDistance)
	if err != nil || seekLimit.SeekTicks != maxTick-1 || seekLimit.TotalTicks != maxTick || seekLimit.End.Tick != maxTick {
		t.Fatalf("wide seek exact limit=(%+v,%v)", seekLimit, err)
	}
	seekOverflow, err := Plan(hddgeometry.Geometry{Cylinders: maximumDistance + 2, Surfaces: 1, SectorsPerTrack: 1}, State{HeadCylinder: 0}, maximumDistance+1)
	if !errors.Is(err, ErrClockOverflow) || seekOverflow != (Result{}) {
		t.Fatalf("wide seek overflow=(%+v,%v)", seekOverflow, err)
	}

	const maximumPeriod int64 = 9_007_199_254_740_991
	wideRotation, err := Plan(hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: maximumPeriod}, State{HeadCylinder: 0}, maximumPeriod-1)
	if err != nil || wideRotation.RotationTicks != maxTick-1 || wideRotation.TotalTicks != maxTick || wideRotation.End.Tick != maxTick {
		t.Fatalf("wide rotation exact limit=(%+v,%v)", wideRotation, err)
	}
	if result, err := Plan(hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: maximumPeriod}, State{HeadCylinder: 0, Tick: 1}, 0); !errors.Is(err, ErrClockOverflow) || result != (Result{}) {
		t.Fatalf("wide period one-tick overflow=(%+v,%v)", result, err)
	}

	period := int64(maxTick / 2)
	wrapGeometry := hddgeometry.Geometry{Cylinders: 2, Surfaces: 1, SectorsPerTrack: period}
	wrapped, err := Plan(wrapGeometry, State{HeadCylinder: 0, Tick: uint64(period - 2)}, period)
	if err != nil || wrapped.Target != (hddgeometry.Address{Cylinder: 1}) || wrapped.SeekTicks != 3 ||
		wrapped.RotationTicks != uint64(period-1) || wrapped.TotalTicks != uint64(period+3) || wrapped.End.Tick != maxTick {
		t.Fatalf("wide modular wrap exact ceiling=(%+v,%v)", wrapped, err)
	}
}

func TestPlanAllocatesNothing(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	if allocations := testing.AllocsPerRun(1_000, func() {
		resultSink, _ = Plan(geometry, State{HeadCylinder: 5, Tick: 12345}, 511)
	}); allocations != 0 {
		t.Fatalf("Plan allocated %v times", allocations)
	}
}

func TestFuzzOracleRepresentativeBoundaries(t *testing.T) {
	profiles := []hddgeometry.Geometry{
		{Cylinders: 4, Surfaces: 2, SectorsPerTrack: 4},
		{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16},
		{Cylinders: 64, Surfaces: 2, SectorsPerTrack: 4},
	}
	checked := 0
	for _, geometry := range profiles {
		count := geometry.Cylinders * geometry.Surfaces * geometry.SectorsPerTrack
		for i := 0; i < 256; i++ {
			block := int64((i*73 + 19) % int(count))
			head := int64((i*11 + 3) % int(geometry.Cylinders))
			tick := uint64(i*991) + uint64(i)%uint64(geometry.SectorsPerTrack)
			want, modelErr := stepModel(geometry, State{HeadCylinder: head, Tick: tick}, block)
			got, err := Plan(geometry, State{HeadCylinder: head, Tick: tick}, block)
			if modelErr != nil || err != nil || got != want {
				t.Fatalf("seeded %v %d: got=%+v,%v want=%+v,%v", geometry, i, got, err, want, modelErr)
			}
			checked++
		}
	}
	if checked != 768 {
		t.Fatalf("seeded commands=%d", checked)
	}
}

func FuzzPlanMatchesBigIntegerOracle(f *testing.F) {
	for _, seed := range [][5]uint64{
		{4, 2, 4, 0, 31}, {16, 2, 16, 15, 511}, {64, 2, 4, 63, 511},
		{1, 1, 1, 0, 0}, {1, 1, 9_007_199_254_740_991, 0, 9_007_199_254_740_990},
	} {
		f.Add(seed[0], seed[1], seed[2], seed[3], seed[4], uint64(0))
	}
	f.Fuzz(func(t *testing.T, cylinders, surfaces, sectors, head, block, tick uint64) {
		// Go fuzz converts uint64 inputs to signed dimensions without overflow by
		// clamping each to a host-portable positive test range.
		limit := uint64(maxTick) + 2
		g := hddgeometry.Geometry{
			Cylinders:       int64(cylinders % limit),
			Surfaces:        int64(surfaces % limit),
			SectorsPerTrack: int64(sectors % limit),
		}
		current := State{HeadCylinder: int64(head % (uint64(maxTick) + 2)), Tick: tick}
		blockArg := int64(block % (uint64(maxTick) + 2))
		want, expectedErr := bigModel(g, current, blockArg)
		got, err := Plan(g, current, blockArg)
		if expectedErr != nil {
			if !errors.Is(err, expectedErr) || got != (Result{}) {
				t.Fatalf("model error=%v actual=(%+v,%v)", expectedErr, got, err)
			}
			return
		}
		if err != nil || got != want {
			t.Fatalf("actual=(%+v,%v) want %+v", got, err, want)
		}
	})
}

func bigModel(g hddgeometry.Geometry, current State, block int64) (Result, error) {
	if g.Cylinders <= 0 || g.Surfaces <= 0 || g.SectorsPerTrack <= 0 {
		return Result{}, blockdevicecontract.ErrInvalidGeometry
	}
	count := new(big.Int).Mul(big.NewInt(g.Cylinders), big.NewInt(g.Surfaces))
	count.Mul(count, big.NewInt(g.SectorsPerTrack))
	max := new(big.Int).SetUint64(maxTick)
	if count.Cmp(max) > 0 {
		return Result{}, blockdevicecontract.ErrInvalidGeometry
	}
	if block < 0 || big.NewInt(block).Cmp(count) >= 0 {
		return Result{}, blockdevicecontract.ErrInvalidBlock
	}
	track := block / g.SectorsPerTrack
	target := hddgeometry.Address{Cylinder: track / g.Surfaces, Surface: track % g.Surfaces, Sector: block % g.SectorsPerTrack}
	if current.HeadCylinder < 0 || current.HeadCylinder >= g.Cylinders {
		return Result{}, ErrHeadCylinder
	}
	if current.Tick > maxTick {
		return Result{}, ErrClockOverflow
	}
	distance := new(big.Int).Sub(big.NewInt(target.Cylinder), big.NewInt(current.HeadCylinder))
	distance.Abs(distance)
	seek := new(big.Int).Mul(distance, big.NewInt(3))
	tick := new(big.Int).SetUint64(current.Tick)
	period := big.NewInt(g.SectorsPerTrack)
	phase := new(big.Int).Add(new(big.Int).Mod(tick, period), new(big.Int).Mod(seek, period))
	phase.Mod(phase, period)
	rotation := new(big.Int).Sub(big.NewInt(target.Sector), phase)
	rotation.Mod(rotation, period)
	if rotation.Sign() < 0 {
		rotation.Add(rotation, period)
	}
	total := new(big.Int).Add(new(big.Int).Set(seek), rotation)
	total.Add(total, big.NewInt(1))
	end := new(big.Int).Add(tick, total)
	if end.Cmp(max) > 0 {
		return Result{}, ErrClockOverflow
	}
	return Result{
		Target:    target,
		End:       State{HeadCylinder: target.Cylinder, Tick: end.Uint64()},
		SeekTicks: seek.Uint64(), RotationTicks: rotation.Uint64(), TransferTicks: 1, TotalTicks: total.Uint64(),
	}, nil
}
