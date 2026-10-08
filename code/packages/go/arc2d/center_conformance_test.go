package arc2d

import (
	"encoding/json"
	"errors"
	"math"
	"os"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
)

type centerCase struct {
	ID             string     `json:"id"`
	Operation      string     `json:"operation"`
	Center         [2]float64 `json:"center"`
	Rx             float64    `json:"rx"`
	Ry             float64    `json:"ry"`
	StartAngle     float64    `json:"start_angle"`
	SweepAngle     float64    `json:"sweep_angle"`
	XRotation      float64    `json:"x_rotation"`
	ExpectedBounds [4]float64 `json:"expected_bounds"`
	ExpectedCount  int        `json:"expected_count"`
	ExpectedError  string     `json:"expected_error"`
}

func (c centerCase) arc() CenterArc {
	return CenterArc{Center: point2d.NewPoint(c.Center[0], c.Center[1]),
		Rx: c.Rx, Ry: c.Ry, StartAngle: c.StartAngle,
		SweepAngle: c.SweepAngle, XRotation: c.XRotation}
}

func requirePanicIs(t *testing.T, want error, call func()) {
	t.Helper()
	defer func() {
		got := recover()
		if got == nil {
			t.Fatalf("expected panic %v", want)
		}
		err, ok := got.(error)
		if !ok || !errors.Is(err, want) {
			t.Fatalf("panic = %v, want %v", got, want)
		}
	}()
	call()
}

func TestCenterArcNeutralFixtures(t *testing.T) {
	data, err := os.ReadFile("../../../specs/fixtures/geometry2d-v1/cases.json")
	if err != nil {
		t.Fatal(err)
	}
	var corpus struct {
		Version           int          `json:"version"`
		AbsoluteTolerance float64      `json:"absolute_tolerance"`
		Cases             []centerCase `json:"cases"`
	}
	if err := json.Unmarshal(data, &corpus); err != nil {
		t.Fatal(err)
	}
	if corpus.Version != 1 || corpus.AbsoluteTolerance != 1e-12 {
		t.Fatalf("unexpected corpus contract: %d / %g", corpus.Version, corpus.AbsoluteTolerance)
	}
	boundsCases, cubicCases := 0, 0
	for _, c := range corpus.Cases {
		switch c.Operation {
		case "center-arc-bounds":
			boundsCases++
			t.Run(c.ID, func(t *testing.T) {
				b := BboxArc(c.arc())
				got := [4]float64{b.X, b.Y, b.Width, b.Height}
				for i, want := range c.ExpectedBounds {
					if math.IsNaN(got[i]) || math.Abs(got[i]-want) > corpus.AbsoluteTolerance {
						t.Fatalf("bounds[%d] = %.17g, want %.17g", i, got[i], want)
					}
				}
			})
		case "center-arc-cubics":
			cubicCases++
			t.Run(c.ID, func(t *testing.T) {
				if c.ExpectedError == "invalid-sweep" {
					requirePanicIs(t, ErrInvalidSweep, func() { ToCubicBeziers(c.arc()) })
					return
				}
				curves := ToCubicBeziers(c.arc())
				if len(curves) != c.ExpectedCount {
					t.Fatalf("segments = %d, want %d", len(curves), c.ExpectedCount)
				}
				if c.SweepAngle == 0 {
					p := curves[0].P0
					if curves[0].P1 != p || curves[0].P2 != p || curves[0].P3 != p {
						t.Fatal("zero sweep must yield one degenerate cubic")
					}
				}
			})
		}
	}
	if boundsCases != 4 || cubicCases != 3 {
		t.Fatalf("fixture drift: %d bounds / %d cubics", boundsCases, cubicCases)
	}
}

func TestCenterArcRejectsInvalidInputs(t *testing.T) {
	for _, tc := range []struct {
		name   string
		mutate func(*CenterArc)
		want   error
	}{
		{"nan-center", func(a *CenterArc) { a.Center.X = math.NaN() }, ErrInvalidCenterArc},
		{"infinite-radius", func(a *CenterArc) { a.Rx = math.Inf(1) }, ErrInvalidCenterArc},
		{"zero-radius", func(a *CenterArc) { a.Ry = 0 }, ErrInvalidCenterArc},
		{"negative-radius", func(a *CenterArc) { a.Rx = -1 }, ErrInvalidCenterArc},
		{"nan-start", func(a *CenterArc) { a.StartAngle = math.NaN() }, ErrInvalidCenterArc},
		{"infinite-rotation", func(a *CenterArc) { a.XRotation = math.Inf(-1) }, ErrInvalidCenterArc},
		{"nan-sweep", func(a *CenterArc) { a.SweepAngle = math.NaN() }, ErrInvalidCenterArc},
		{"infinite-sweep", func(a *CenterArc) { a.SweepAngle = math.Inf(1) }, ErrInvalidCenterArc},
		{"over-turn", func(a *CenterArc) { a.SweepAngle = 7 }, ErrInvalidSweep},
	} {
		t.Run(tc.name, func(t *testing.T) {
			a := unitArc
			tc.mutate(&a)
			for _, fn := range []struct {
				name string
				call func()
			}{
				{"evaluate", func() { EvalArc(a, 0.5) }},
				{"tangent", func() { TangentArc(a, 0.5) }},
				{"bounds", func() { BboxArc(a) }},
				{"cubics", func() { ToCubicBeziers(a) }},
			} {
				t.Run(fn.name, func(t *testing.T) { requirePanicIs(t, tc.want, fn.call) })
			}
		})
	}
	for _, value := range []float64{math.NaN(), math.Inf(1)} {
		requirePanicIs(t, ErrInvalidCenterArc, func() { EvalArc(unitArc, value) })
		requirePanicIs(t, ErrInvalidCenterArc, func() { TangentArc(unitArc, value) })
	}
}

func TestCenterArcRejectsNonfiniteDerivedOutput(t *testing.T) {
	a := unitArc
	a.Center.X, a.Rx = math.MaxFloat64, math.MaxFloat64
	requirePanicIs(t, ErrInvalidCenterArc, func() { EvalArc(a, 0) })
	requirePanicIs(t, ErrInvalidCenterArc, func() { BboxArc(a) })
	requirePanicIs(t, ErrInvalidCenterArc, func() { ToCubicBeziers(a) })
	requirePanicIs(t, ErrInvalidCenterArc, func() { EvalArc(unitArc, math.MaxFloat64) })
	a = unitArc
	a.Rx = math.MaxFloat64
	a.SweepAngle = 2 * math.Pi
	requirePanicIs(t, ErrInvalidCenterArc, func() { TangentArc(a, 0.25) })
	a = unitArc
	a.Rx = math.MaxFloat64
	a.SweepAngle = 2 * math.Pi
	requirePanicIs(t, ErrInvalidCenterArc, func() { BboxArc(a) })
}

func TestExistingEndpointConversionBranches(t *testing.T) {
	for _, tc := range []struct {
		name, from, to string
		start, end     point2d.Point
		rx, ry         float64
		large, sweep   bool
		wantPresent    bool
	}{
		{name: "scaled-negative-radius", start: point2d.NewPoint(0, 0), end: point2d.NewPoint(10, 0), rx: -0.1, ry: 1, sweep: true, wantPresent: true},
		{name: "large-clockwise", start: point2d.NewPoint(1, 0), end: point2d.NewPoint(0, 1), rx: 1, ry: 1, large: true, sweep: false, wantPresent: true},
		{name: "small-clockwise", start: point2d.NewPoint(1, 0), end: point2d.NewPoint(0, 1), rx: 1, ry: 1, sweep: false, wantPresent: true},
		{name: "large-counterclockwise", start: point2d.NewPoint(1, 0), end: point2d.NewPoint(0, 1), rx: 1, ry: 1, large: true, sweep: true, wantPresent: true},
		{name: "tiny-distance", start: point2d.NewPoint(0, 0), end: point2d.NewPoint(1e-13, 0), rx: 1, ry: 1, sweep: true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			a, ok := ToCenterArc(SvgArc{From: tc.start, To: tc.end, Rx: tc.rx, Ry: tc.ry, LargeArc: tc.large, Sweep: tc.sweep})
			if ok != tc.wantPresent {
				t.Fatalf("center presence = %v, want %v", ok, tc.wantPresent)
			}
			if ok && (!finite(a.Center.X, a.Center.Y, a.Rx, a.Ry, a.StartAngle, a.SweepAngle) || a.Rx <= 0 || a.Ry <= 0) {
				t.Fatalf("invalid center result: %+v", a)
			}
		})
	}
	if angleBetween(0, 0, 1, 0) != 0 {
		t.Fatal("zero vector angle")
	}
	if angleBetween(1, 0, 0, -1) >= 0 {
		t.Fatal("clockwise vector angle")
	}
}
