package bezier2d

import (
	"encoding/json"
	"errors"
	"math"
	"os"
	"path/filepath"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
)

type flatteningFixture struct {
	Version         int              `json:"version"`
	MaxDepth        int              `json:"max_depth"`
	MaxSubdivisions int              `json:"max_subdivisions"`
	Cases           []flatteningCase `json:"cases"`
}

type flatteningCase struct {
	ID                  string       `json:"id"`
	Degree              int          `json:"degree"`
	ControlPoints       [][2]float64 `json:"control_points"`
	Tolerance           float64      `json:"tolerance"`
	ExpectedDisposition string       `json:"expected_disposition"`
	ExpectedTermination string       `json:"expected_termination"`
}

func loadFlatteningFixture(t *testing.T) flatteningFixture {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "bezier2d-flattening-v1", "cases.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read flattening fixture: %v", err)
	}
	var fixture flatteningFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode flattening fixture: %v", err)
	}
	return fixture
}

func fixturePoint(coordinates [2]float64) point2d.Point {
	return point2d.NewPoint(coordinates[0], coordinates[1])
}

func TestFlatteningFixtureConformance(t *testing.T) {
	fixture := loadFlatteningFixture(t)
	if fixture.Version != FlatteningFixtureVersion || fixture.MaxDepth != FlatteningMaxDepth || fixture.MaxSubdivisions != FlatteningMaxSubdivisions {
		t.Fatalf("fixture metadata does not match implementation limits: %+v", fixture)
	}
	if len(fixture.Cases) != 9 {
		t.Fatalf("expected all 9 fixture cases, got %d", len(fixture.Cases))
	}

	for _, testCase := range fixture.Cases {
		t.Run(testCase.ID, func(t *testing.T) {
			var points []point2d.Point
			var err error
			switch testCase.Degree {
			case 2:
				curve := QuadraticBezier{fixturePoint(testCase.ControlPoints[0]), fixturePoint(testCase.ControlPoints[1]), fixturePoint(testCase.ControlPoints[2])}
				points, err = PolylineQuad(curve, testCase.Tolerance)
			case 3:
				curve := CubicBezier{fixturePoint(testCase.ControlPoints[0]), fixturePoint(testCase.ControlPoints[1]), fixturePoint(testCase.ControlPoints[2]), fixturePoint(testCase.ControlPoints[3])}
				points, err = PolylineCubic(curve, testCase.Tolerance)
			default:
				t.Fatalf("unsupported fixture degree %d", testCase.Degree)
			}

			switch {
			case testCase.ExpectedDisposition == "invalid-tolerance":
				if !errors.Is(err, ErrInvalidTolerance) || points != nil {
					t.Fatalf("expected atomic invalid-tolerance error, got points=%v err=%v", points, err)
				}
			case testCase.ExpectedTermination == "budget-error":
				if !errors.Is(err, ErrFlatteningBudgetExceeded) || points != nil {
					t.Fatalf("expected atomic budget error, got points=%v err=%v", points, err)
				}
			default:
				if err != nil {
					t.Fatalf("unexpected flattening error: %v", err)
				}
				if len(points) < 2 || points[0] != fixturePoint(testCase.ControlPoints[0]) || points[len(points)-1] != fixturePoint(testCase.ControlPoints[len(testCase.ControlPoints)-1]) {
					t.Fatalf("polyline does not preserve endpoints: %v", points)
				}
				for _, point := range points {
					if math.IsNaN(point.X) || math.IsNaN(point.Y) || math.IsInf(point.X, 0) || math.IsInf(point.Y, 0) {
						t.Fatalf("polyline contains non-finite point: %v", point)
					}
				}
				if testCase.ExpectedDisposition == "chord" && len(points) != 2 {
					t.Fatalf("expected a chord, got %d points", len(points))
				}
				if testCase.ExpectedDisposition == "subdivide" && len(points) <= 2 {
					t.Fatalf("expected subdivision, got %d points", len(points))
				}
			}
		})
	}
}

func TestFlatteningRejectsNonFiniteInputsAtomically(t *testing.T) {
	straight := QuadraticBezier{point2d.NewPoint(0, 0), point2d.NewPoint(0.5, 0), point2d.NewPoint(1, 0)}
	for _, tolerance := range []float64{math.NaN(), math.Inf(1), math.Inf(-1)} {
		points, err := PolylineQuad(straight, tolerance)
		if !errors.Is(err, ErrInvalidTolerance) || points != nil {
			t.Fatalf("tolerance %v: expected atomic invalid-tolerance error, got points=%v err=%v", tolerance, points, err)
		}
	}

	badControl := straight
	badControl.P1.X = math.Inf(1)
	points, err := PolylineQuad(badControl, 0.1)
	if !errors.Is(err, ErrNonFiniteGeometry) || points != nil {
		t.Fatalf("expected atomic non-finite control error, got points=%v err=%v", points, err)
	}
}

func TestFlatteningFailsClosedOnDerivedOverflowAndSubnormalGeometry(t *testing.T) {
	max := math.MaxFloat64
	overflow := CubicBezier{point2d.NewPoint(-max, 0), point2d.NewPoint(0, 1), point2d.NewPoint(0, -1), point2d.NewPoint(max, 0)}
	points, err := PolylineCubic(overflow, 0.1)
	if !errors.Is(err, ErrNonFiniteGeometry) || points != nil {
		t.Fatalf("expected atomic derived-overflow error, got points=%v err=%v", points, err)
	}

	tiny := 1e-200
	subnormal := QuadraticBezier{point2d.NewPoint(0, 0), point2d.NewPoint(0, tiny), point2d.NewPoint(tiny, 0)}
	points, err = PolylineQuad(subnormal, math.SmallestNonzeroFloat64)
	if !errors.Is(err, ErrFlatteningBudgetExceeded) || points != nil {
		t.Fatalf("expected subnormal curve to exhaust its budget, got points=%v err=%v", points, err)
	}
}
