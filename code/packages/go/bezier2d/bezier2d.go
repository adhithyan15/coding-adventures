// Package bezier2d provides quadratic and cubic Bezier curves.
//
// All evaluation uses de Casteljau's algorithm for numerical stability.
// No trig dependency for curve evaluation (trig.Sqrt used for bounding box).
package bezier2d

import (
	"errors"
	"math"

	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

// The neutral G2D02 fixture pins these limits so every implementation fails
// at the same boundary instead of depending on its host stack size.
const (
	FlatteningFixtureVersion  = 1
	FlatteningMaxDepth        = 32
	FlatteningMaxSubdivisions = 65_535
)

var (
	// ErrInvalidTolerance reports a zero, negative, NaN, or infinite tolerance.
	ErrInvalidTolerance = errors.New("bezier2d: tolerance must be finite and positive")
	// ErrNonFiniteGeometry reports non-finite input or derived control geometry.
	ErrNonFiniteGeometry = errors.New("bezier2d: non-finite geometry")
	// ErrFlatteningBudgetExceeded reports the pinned depth or split ceiling.
	ErrFlatteningBudgetExceeded = errors.New("bezier2d: flattening budget exceeded")
)

// ============================================================================
// QuadraticBezier
// ============================================================================

// QuadraticBezier is a degree-2 Bezier curve with three control points.
type QuadraticBezier struct {
	P0, P1, P2 point2d.Point
}

// EvalQuad evaluates the quadratic Bezier at t ∈ [0,1] using de Casteljau.
func EvalQuad(q QuadraticBezier, t float64) point2d.Point {
	q0 := q.P0.Lerp(q.P1, t)
	q1 := q.P1.Lerp(q.P2, t)
	return q0.Lerp(q1, t)
}

// DerivQuad returns the tangent vector at t.
func DerivQuad(q QuadraticBezier, t float64) point2d.Point {
	d0 := q.P1.Subtract(q.P0)
	d1 := q.P2.Subtract(q.P1)
	return d0.Lerp(d1, t).Scale(2)
}

// SplitQuad splits the curve at t into (left, right).
func SplitQuad(q QuadraticBezier, t float64) (QuadraticBezier, QuadraticBezier) {
	q0 := q.P0.Lerp(q.P1, t)
	q1 := q.P1.Lerp(q.P2, t)
	m := q0.Lerp(q1, t)
	return QuadraticBezier{q.P0, q0, m}, QuadraticBezier{m, q1, q.P2}
}

// PolylineQuad conservatively flattens q or returns an atomic error.
//
// Midpoint-only tests miss symmetric curves whose midpoint lies on the chord.
// The quadratic convex-hull property gives a stronger test: the entire curve
// stays within the maximum distance from its control point to the finite chord
// segment. An explicit stack avoids host recursion limits, and pushing right
// before left preserves increasing parameter order when popping.
func PolylineQuad(q QuadraticBezier, tolerance float64) ([]point2d.Point, error) {
	if !validTolerance(tolerance) {
		return nil, ErrInvalidTolerance
	}
	if !finitePoints(q.P0, q.P1, q.P2) {
		return nil, ErrNonFiniteGeometry
	}

	type frame struct {
		curve QuadraticBezier
		depth int
	}
	stack := []frame{{curve: q}}
	points := []point2d.Point{q.P0}
	subdivisions := 0
	for len(stack) > 0 {
		current := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		errorBound, ok := pointToSegmentDistance(current.curve.P1, current.curve.P0, current.curve.P2)
		if !ok {
			return nil, ErrNonFiniteGeometry
		}
		if errorBound <= tolerance {
			points = append(points, current.curve.P2)
			continue
		}
		if current.depth >= FlatteningMaxDepth || subdivisions >= FlatteningMaxSubdivisions {
			return nil, ErrFlatteningBudgetExceeded
		}
		left, right := SplitQuad(current.curve, 0.5)
		if !finitePoints(left.P0, left.P1, left.P2, right.P0, right.P1, right.P2) {
			return nil, ErrNonFiniteGeometry
		}
		subdivisions++
		nextDepth := current.depth + 1
		stack = append(stack, frame{curve: right, depth: nextDepth}, frame{curve: left, depth: nextDepth})
	}
	return points, nil
}

// BboxQuad returns the tight bounding box of the quadratic Bezier.
func BboxQuad(q QuadraticBezier) point2d.Rect {
	minX, maxX := minf(q.P0.X, q.P2.X), maxf(q.P0.X, q.P2.X)
	minY, maxY := minf(q.P0.Y, q.P2.Y), maxf(q.P0.Y, q.P2.Y)

	if dx := q.P0.X - 2*q.P1.X + q.P2.X; absf(dx) > 1e-12 {
		if tx := (q.P0.X - q.P1.X) / dx; tx > 0 && tx < 1 {
			px := EvalQuad(q, tx)
			minX, maxX = minf(minX, px.X), maxf(maxX, px.X)
		}
	}
	if dy := q.P0.Y - 2*q.P1.Y + q.P2.Y; absf(dy) > 1e-12 {
		if ty := (q.P0.Y - q.P1.Y) / dy; ty > 0 && ty < 1 {
			py := EvalQuad(q, ty)
			minY, maxY = minf(minY, py.Y), maxf(maxY, py.Y)
		}
	}
	return point2d.NewRect(minX, minY, maxX-minX, maxY-minY)
}

// ElevateQuad converts a quadratic to an equivalent cubic.
func ElevateQuad(q QuadraticBezier) CubicBezier {
	q1 := q.P0.Scale(1.0 / 3).Add(q.P1.Scale(2.0 / 3))
	q2 := q.P1.Scale(2.0 / 3).Add(q.P2.Scale(1.0 / 3))
	return CubicBezier{q.P0, q1, q2, q.P2}
}

// ============================================================================
// CubicBezier
// ============================================================================

// CubicBezier is a degree-3 Bezier curve with four control points.
type CubicBezier struct {
	P0, P1, P2, P3 point2d.Point
}

// EvalCubic evaluates the cubic Bezier at t ∈ [0,1] using de Casteljau.
func EvalCubic(c CubicBezier, t float64) point2d.Point {
	p01 := c.P0.Lerp(c.P1, t)
	p12 := c.P1.Lerp(c.P2, t)
	p23 := c.P2.Lerp(c.P3, t)
	p012 := p01.Lerp(p12, t)
	p123 := p12.Lerp(p23, t)
	return p012.Lerp(p123, t)
}

// DerivCubic returns the tangent vector at t.
func DerivCubic(c CubicBezier, t float64) point2d.Point {
	d0 := c.P1.Subtract(c.P0)
	d1 := c.P2.Subtract(c.P1)
	d2 := c.P3.Subtract(c.P2)
	oneT := 1 - t
	r := d0.Scale(oneT * oneT).Add(d1.Scale(2 * oneT * t)).Add(d2.Scale(t * t))
	return r.Scale(3)
}

// SplitCubic splits the cubic at t into (left, right).
func SplitCubic(c CubicBezier, t float64) (CubicBezier, CubicBezier) {
	p01 := c.P0.Lerp(c.P1, t)
	p12 := c.P1.Lerp(c.P2, t)
	p23 := c.P2.Lerp(c.P3, t)
	p012 := p01.Lerp(p12, t)
	p123 := p12.Lerp(p23, t)
	p0123 := p012.Lerp(p123, t)
	return CubicBezier{c.P0, p01, p012, p0123}, CubicBezier{p0123, p123, p23, c.P3}
}

// PolylineCubic conservatively flattens c or returns an atomic error.
// Both off-curve controls participate in the finite-segment error bound.
func PolylineCubic(c CubicBezier, tolerance float64) ([]point2d.Point, error) {
	if !validTolerance(tolerance) {
		return nil, ErrInvalidTolerance
	}
	if !finitePoints(c.P0, c.P1, c.P2, c.P3) {
		return nil, ErrNonFiniteGeometry
	}

	type frame struct {
		curve CubicBezier
		depth int
	}
	stack := []frame{{curve: c}}
	points := []point2d.Point{c.P0}
	subdivisions := 0
	for len(stack) > 0 {
		current := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		firstBound, firstOK := pointToSegmentDistance(current.curve.P1, current.curve.P0, current.curve.P3)
		secondBound, secondOK := pointToSegmentDistance(current.curve.P2, current.curve.P0, current.curve.P3)
		if !firstOK || !secondOK {
			return nil, ErrNonFiniteGeometry
		}
		if math.Max(firstBound, secondBound) <= tolerance {
			points = append(points, current.curve.P3)
			continue
		}
		if current.depth >= FlatteningMaxDepth || subdivisions >= FlatteningMaxSubdivisions {
			return nil, ErrFlatteningBudgetExceeded
		}
		left, right := SplitCubic(current.curve, 0.5)
		if !finitePoints(left.P0, left.P1, left.P2, left.P3, right.P0, right.P1, right.P2, right.P3) {
			return nil, ErrNonFiniteGeometry
		}
		subdivisions++
		nextDepth := current.depth + 1
		stack = append(stack, frame{curve: right, depth: nextDepth}, frame{curve: left, depth: nextDepth})
	}
	return points, nil
}

func validTolerance(tolerance float64) bool {
	return tolerance > 0 && !math.IsNaN(tolerance) && !math.IsInf(tolerance, 0)
}

func finitePoints(points ...point2d.Point) bool {
	for _, point := range points {
		if math.IsNaN(point.X) || math.IsNaN(point.Y) || math.IsInf(point.X, 0) || math.IsInf(point.Y, 0) {
			return false
		}
	}
	return true
}

// pointToSegmentDistance projects onto a unit-length chord before measuring the
// perpendicular residual. Scaling the point vector by the chord's largest
// component would erase valid mixed-scale offsets (for example, 1e-16 above a
// 1e308-wide chord). Compensated products retain cancellation-sensitive dot and
// cross products, and every returned magnitude is rounded outward by one ULP.
// A false result means finite inputs produced non-finite derived arithmetic and
// flattening must fail closed.
func pointToSegmentDistance(point, start, end point2d.Point) (float64, bool) {
	dx, dy := end.X-start.X, end.Y-start.Y
	px, py := point.X-start.X, point.Y-start.Y
	if math.IsNaN(dx) || math.IsNaN(dy) || math.IsNaN(px) || math.IsNaN(py) ||
		math.IsInf(dx, 0) || math.IsInf(dy, 0) || math.IsInf(px, 0) || math.IsInf(py, 0) {
		return 0, false
	}
	length := math.Hypot(dx, dy)
	pointLength := math.Hypot(px, py)
	if math.IsInf(length, 0) || math.IsInf(pointLength, 0) {
		return 0, false
	}
	if length == 0 {
		return outwardMagnitude(pointLength), true
	}

	chordScale := math.Max(math.Abs(dx), math.Abs(dy))
	sx, sy := dx/chordScale, dy/chordScale
	normalizedLength := math.Hypot(sx, sy)
	ux, uy := sx/normalizedLength, sy/normalizedLength
	projection, ok := compensatedProductDifference(px, ux, -py, uy)
	if !ok {
		return 0, false
	}
	// Near an endpoint, rounding ambiguity is resolved toward that endpoint.
	// Endpoint distance is an upper bound on finite-segment distance, so this
	// can subdivide more but can never accept an unsafe chord.
	margin := 8*(math.Nextafter(1, 2)-1)*math.Max(pointLength, length) + math.SmallestNonzeroFloat64
	if math.IsInf(margin, 0) {
		return 0, false
	}
	if projection <= margin {
		return outwardMagnitude(pointLength), true
	}
	if projection >= length-margin {
		ex, ey := point.X-end.X, point.Y-end.Y
		if math.IsNaN(ex) || math.IsNaN(ey) || math.IsInf(ex, 0) || math.IsInf(ey, 0) {
			return 0, false
		}
		endDistance := math.Hypot(ex, ey)
		if math.IsInf(endDistance, 0) {
			return 0, false
		}
		return outwardMagnitude(endDistance), true
	}

	// Compute the determinant with only the chord scaled. Scaling the point
	// vector as well would discard a small perpendicular component next to a
	// very large chord. Dividing by the scaled chord length leaves the result
	// in the point vector's original units.
	cross, ok := compensatedProductDifference(px, sy, py, sx)
	if !ok {
		return 0, false
	}
	// Widen for division-rounding in the scaled chord components, the final
	// compensated-product rounding, and the norm. This is intentionally an
	// upper bound: accepting a chord requires proving the true distance is no
	// larger than tolerance. The dominant scaled component is exactly +/-1.
	sxError := divisionErrorBound(sx, dx, chordScale)
	syError := divisionErrorBound(sy, dy, chordScale)
	productMagnitude := math.Abs(px*sy) + math.Abs(py*sx)
	numeratorError := math.Abs(px)*syError + math.Abs(py)*sxError +
		8*(math.Nextafter(1, 2)-1)*(math.Abs(cross)+productMagnitude)
	normError := math.Hypot(sxError, syError) + 8*(math.Nextafter(1, 2)-1)*normalizedLength
	lowerNorm := normalizedLength - normError
	if math.IsInf(numeratorError, 0) || lowerNorm <= 0 {
		return outwardMagnitude(pointLength), true
	}
	distance := (math.Abs(cross) + numeratorError) / lowerNorm
	if math.IsNaN(distance) || math.IsInf(distance, 0) {
		return outwardMagnitude(pointLength), true
	}
	return math.Min(outwardMagnitude(distance), outwardMagnitude(pointLength)), true
}

// compensatedProductDifference returns a*b-c*d with the product and
// subtraction roundoff folded back into the result. math.FMA exposes each
// product's exact residual without widening the public numeric type.
func compensatedProductDifference(a, b, c, d float64) (float64, bool) {
	ab := a * b
	cd := c * d
	if math.IsNaN(ab) || math.IsNaN(cd) || math.IsInf(ab, 0) || math.IsInf(cd, 0) {
		return 0, false
	}
	abTail := math.FMA(a, b, -ab)
	cdTail := math.FMA(c, d, -cd)
	difference := ab - cd
	if math.IsNaN(difference) || math.IsInf(difference, 0) {
		return 0, false
	}
	bVirtual := ab - difference
	aVirtual := difference + bVirtual
	bRoundoff := bVirtual - cd
	aRoundoff := ab - aVirtual
	correction := (aRoundoff + bRoundoff) + abTail - cdTail
	result := difference + correction
	if math.IsNaN(result) || math.IsInf(result, 0) {
		return 0, false
	}
	return result, true
}

func divisionErrorBound(ratio, numerator, denominator float64) float64 {
	if numerator == 0 || math.Abs(numerator) == denominator {
		return 0
	}
	return (math.Nextafter(1, 2)-1)*math.Abs(ratio) + math.SmallestNonzeroFloat64
}

func outwardMagnitude(value float64) float64 {
	if value == 0 {
		return 0
	}
	return math.Nextafter(value, math.Inf(1))
}

// BboxCubic returns the tight bounding box of the cubic Bezier.
func BboxCubic(c CubicBezier) point2d.Rect {
	minX, maxX := minf(c.P0.X, c.P3.X), maxf(c.P0.X, c.P3.X)
	minY, maxY := minf(c.P0.Y, c.P3.Y), maxf(c.P0.Y, c.P3.Y)

	for _, tx := range extrema(c.P0.X, c.P1.X, c.P2.X, c.P3.X) {
		px := EvalCubic(c, tx)
		minX, maxX = minf(minX, px.X), maxf(maxX, px.X)
	}
	for _, ty := range extrema(c.P0.Y, c.P1.Y, c.P2.Y, c.P3.Y) {
		py := EvalCubic(c, ty)
		minY, maxY = minf(minY, py.Y), maxf(maxY, py.Y)
	}
	return point2d.NewRect(minX, minY, maxX-minX, maxY-minY)
}

// extrema finds t in (0,1) where the cubic's derivative in one coordinate is zero.
func extrema(v0, v1, v2, v3 float64) []float64 {
	a := -3*v0 + 9*v1 - 9*v2 + 3*v3
	b := 6*v0 - 12*v1 + 6*v2
	c := -3*v0 + 3*v1
	var roots []float64
	if absf(a) < 1e-12 {
		if absf(b) > 1e-12 {
			if tx := -c / b; tx > 0 && tx < 1 {
				roots = append(roots, tx)
			}
		}
	} else {
		disc := b*b - 4*a*c
		if disc >= 0 {
			sq := trig.Sqrt(disc)
			for _, tx := range []float64{(-b + sq) / (2 * a), (-b - sq) / (2 * a)} {
				if tx > 0 && tx < 1 {
					roots = append(roots, tx)
				}
			}
		}
	}
	return roots
}

func minf(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}
func maxf(a, b float64) float64 {
	if a > b {
		return a
	}
	return b
}
func absf(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}
