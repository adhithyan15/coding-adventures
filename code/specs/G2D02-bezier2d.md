# G2D02 — Bezier2D: Quadratic and Cubic Bezier Curves

## Overview

The `bezier2d` package provides the mathematical machinery for smooth 2D curves.
Bezier curves are the **universal primitive** for smooth shapes in digital
graphics: every path in SVG, PDF, HTML Canvas, Core Graphics (macOS/iOS),
Direct2D (Windows), and Skia (Chrome/Android/Flutter) ultimately decomposes
into sequences of cubic (or quadratic) Bezier segments.

Dependencies:

- `point2d` (G2D00) — Point, Rect, and lerp for all curve computations

No dependency on `trig` (PHY00) — Bezier curves are pure polynomial arithmetic.
This keeps the package maximally lightweight and embeddable.

```
bezier2d (G2D02)
└── point2d (G2D00)
```

### Where Bezier curves appear

**Rendering** (the obvious case):
- SVG `<path>` uses `Q` (quadratic) and `C` (cubic) commands.
- PDF uses only cubic beziers (`c` and `v` operators).
- TrueType font outlines use quadratic beziers.
- OpenType/PostScript font outlines use cubic beziers.

**Animation easing** (less obvious):
- CSS `cubic-bezier(0.4, 0, 0.2, 1)` is a cubic bezier defined over time.
  The curve maps $t_\text{time} \in [0,1]$ to $\text{progress} \in [0,1]$.
- Material Design's standard easing, bounce easing, spring easing — all cubic
  beziers.

**Path animation** (connecting the two):
- Moving an object along a path requires parameterizing the path by arc length,
  which requires evaluating the bezier at many parameter values.

By building bezier math as a standalone package, all three domains (rendering,
animation, font outlines) can import it without coupling to each other.

---

## Mathematical Background

### Bernstein Basis Polynomials

A degree-$n$ Bezier curve is defined as a weighted sum of control points, where
the weights are the **Bernstein basis polynomials**:

$$B_{i,n}(t) = \binom{n}{i} t^i (1-t)^{n-i}$$

These polynomials have three key properties:
1. **Partition of unity**: $\sum_{i=0}^{n} B_{i,n}(t) = 1$ for all $t$.
2. **Non-negativity**: $B_{i,n}(t) \ge 0$ for $t \in [0,1]$.
3. **Endpoint interpolation**: $B_{0,n}(0) = 1$ and $B_{n,n}(1) = 1$.

Because the weights sum to 1 and are non-negative, the curve point at parameter
$t$ is a _convex combination_ of the control points. This proves the
**convex hull property**: the curve lies entirely within the convex hull of its
control points. This is important for conservative bounding box tests.

### The Quadratic Bezier (n = 2)

Three control points: $P_0$, $P_1$, $P_2$.

$$B(t) = (1-t)^2 P_0 + 2(1-t)t P_1 + t^2 P_2, \quad t \in [0,1]$$

The curve starts at $P_0$ (when $t=0$), ends at $P_2$ (when $t=1$), and is
pulled toward $P_1$ without touching it. $P_1$ is called the _off-curve_ or
_control_ point.

**Analogy**: imagine you are walking from $P_0$ to $P_2$ along a curved path.
At every moment, you are always somewhere between $P_0$, $P_1$, and $P_2$ —
the convex hull acts like an invisible fence.

### The Cubic Bezier (n = 3)

Four control points: $P_0$, $P_1$, $P_2$, $P_3$.

$$B(t) = (1-t)^3 P_0 + 3(1-t)^2 t P_1 + 3(1-t) t^2 P_2 + t^3 P_3, \quad t \in [0,1]$$

The curve starts at $P_0$ and ends at $P_3$. The tangent direction at the start
is along $P_0 \to P_1$; the tangent direction at the end is along $P_2 \to P_3$.
The _interior_ control points $P_1$ and $P_2$ shape the curve without the curve
touching them.

---

## De Casteljau's Algorithm

Pierre de Casteljau discovered an elegant geometric construction for evaluating
a Bezier curve at any parameter $t$. Instead of computing the polynomial
directly (which requires careful numerical implementation to avoid catastrophic
cancellation), de Casteljau _repeatedly lerps_ between control points.

### Geometric construction for a cubic

Given control points $P_0, P_1, P_2, P_3$ and parameter $t$:

**Level 1** — lerp adjacent control points:
```
Q0 = lerp(P0, P1, t)
Q1 = lerp(P1, P2, t)
Q2 = lerp(P2, P3, t)
```

**Level 2** — lerp the level-1 points:
```
R0 = lerp(Q0, Q1, t)
R1 = lerp(Q1, Q2, t)
```

**Level 3** — lerp the level-2 points:
```
S0 = lerp(R0, R1, t)   ← this is B(t), the curve point
```

```
P0          P1          P2          P3
  \        / \        / \        /
   Q0----Q1   Q1----Q2   Q2----Q3
       \     \     /     /
        R0----R1   R1----R2
            \          /
             S0  ← B(t)
```

Wait — let me correct the diagram for the cubic case (3 input points at level 1
become 2 at level 2, become 1 at level 3):

```
P0──────────P1──────────P2──────────P3   (control points)
   ↘t    ↗↘t           ↗↘t       ↗
    Q0─────Q1            Q1──────Q2      (level 1: 3 points)
       ↘t ↗                ↘t  ↗
        R0──────────────────R1           (level 2: 2 points)
              ↘t         ↗
               S0                        (level 3: the curve point)
```

### Why de Casteljau is preferred over direct evaluation

1. **Numerically stable**: repeated lerping (additions and multiplications by
   values in $[0,1]$) does not amplify floating-point errors the way direct
   polynomial evaluation can.
2. **Splits the curve for free**: the intermediate points $Q_0, R_0, S_0$
   (left side) and $S_0, R_1, Q_2$ (right side, reading backward) are exactly
   the control points of the two sub-curves obtained by splitting at $t$. The
   split comes at no extra cost — it falls out of the evaluation.

### De Casteljau split for quadratic

```
Q0 = lerp(P0, P1, t)
Q1 = lerp(P1, P2, t)
M  = lerp(Q0, Q1, t)   ← the split point on the curve

Left half:  QuadraticBezier { p0: P0, p1: Q0, p2: M }
Right half: QuadraticBezier { p0: M,  p1: Q1, p2: P2 }
```

### De Casteljau split for cubic

```
Q0 = lerp(P0, P1, t)
Q1 = lerp(P1, P2, t)
Q2 = lerp(P2, P3, t)
R0 = lerp(Q0, Q1, t)
R1 = lerp(Q1, Q2, t)
S  = lerp(R0, R1, t)   ← the split point on the curve

Left half:  CubicBezier { p0: P0, p1: Q0, p2: R0, p3: S  }
Right half: CubicBezier { p0: S,  p1: R1, p2: Q2, p3: P3 }
```

The left half's control points are: $P_0, Q_0, R_0, S$.
The right half's control points are: $S, R_1, Q_2, P_3$.

---

## Bounding Box via Derivative Roots

The **convex hull property** gives a conservative bounding box: take the
min/max of all control points. But the curve may not reach the extremes of its
convex hull, and it may _dip outside_ the axis-aligned bounding box of the
control points in specific directions.

Wait — actually, the curve lies _within_ the convex hull of control points by
definition. So why not just use the control point bounding box?

The control point bounding box is _conservative_: it may be much larger than
the tight bounding box of the curve itself. For example, a curve that dips
slightly left but whose control points span a wide range will have a too-wide
bounding box.

For a **tight** bounding box, we need the extrema of $B(t)$ — the values of
$t$ where the derivative is zero in each coordinate independently.

```
B'_x(t) = 0  → extremum in x at this t
B'_y(t) = 0  → extremum in y at this t
```

Evaluate $B(t)$ at each root (clamped to $[0,1]$) plus at $t=0$ and $t=1$.
Take the min and max of all evaluated points.

### Derivative of quadratic bezier

$$B'(t) = 2\big[(1-t)(P_1 - P_0) + t(P_2 - P_1)\big]$$

This is a linear function of $t$. Setting the x-component to zero:

$$0 = 2\big[(1-t)(P_{1x} - P_{0x}) + t(P_{2x} - P_{1x})\big]$$

Let $a = P_{1x} - P_{0x}$, $b = P_{2x} - P_{1x}$.

$$0 = (1-t)a + tb = a - at + bt = a + t(b-a)$$

$$t = \frac{-a}{b - a} = \frac{P_{0x} - P_{1x}}{P_{0x} - 2P_{1x} + P_{2x}}$$

(Similarly for y.) If the denominator is zero, the quadratic is actually linear
in that coordinate (monotone — no extremum). Clamp $t$ to $[0,1]$.

### Derivative of cubic bezier

$$B'(t) = 3\big[(1-t)^2(P_1-P_0) + 2(1-t)t(P_2-P_1) + t^2(P_3-P_2)\big]$$

Setting the x-component to zero gives a **quadratic equation** in $t$. Let:

$$A = P_{0x},\; B = P_{1x},\; C = P_{2x},\; D = P_{3x}$$

The derivative x-component (divided by 3):

$$f(t) = (1-t)^2(B-A) + 2(1-t)t(C-B) + t^2(D-C)$$

Expanding:

$$f(t) = (B-A) - 2(B-A)t + (B-A)t^2 + 2(C-B)t - 2(C-B)t^2 + (D-C)t^2$$

Collecting by powers of $t$:

$$f(t) = \underbrace{(B-A)}_{\alpha} + \underbrace{(2C - 2B - 2B + 2A)}_{\beta} t + \underbrace{(A - 2B + C) + (B - 2C + D)}_{\gamma} t^2$$

More carefully:

$$\alpha = B - A$$
$$\beta = 2(C - B) - 2(B - A) = 2(A - 2B + C)$$
$$\gamma = (B - A) - 2(C - B) + (D - C) = -A + 3B - 3C + D$$

So $f(t) = \alpha + \beta t + \gamma t^2 = 0$. Use the quadratic formula:

$$t = \frac{-\beta \pm \sqrt{\beta^2 - 4\gamma\alpha}}{2\gamma}$$

If $\gamma \approx 0$ (degenerate — actually linear), fall back to the linear
case $t = -\alpha / \beta$. If $|\gamma| < \epsilon$, check $|\beta| < \epsilon$
too (constant — no root). Clamp valid roots to $[0, 1]$.

Repeat the same computation for the y-component.

**Worked example** — cubic with control points (0,0), (2,3), (-1,3), (1,0):

```
x: A=0, B=2, C=-1, D=1
  α = 2-0 = 2
  β = 2*(0 - 2*2 + (-1)) = 2*(0-4-1) = -10
  γ = -0 + 3*2 - 3*(-1) + 1 = 0+6+3+1 = 10
  discriminant = 100 - 4*10*2 = 100 - 80 = 20
  t1 = (10 + √20) / 20 ≈ (10 + 4.47) / 20 ≈ 0.724
  t2 = (10 - √20) / 20 ≈ (10 - 4.47) / 20 ≈ 0.276
  Both in [0,1] — evaluate B(0.724) and B(0.276) for x extrema.

y: A=0, B=3, C=3, D=0
  α = 3
  β = 2*(0 - 2*3 + 3) = 2*(-3) = -6
  γ = -0 + 3*3 - 3*3 + 0 = 0
  γ≈0 → linear: t = -α/β = -3/-6 = 0.5
  Evaluate B(0.5) for y extremum.
```

---

## API Reference — QuadraticBezier

```
QuadraticBezier { p0: Point, p1: Point, p2: Point }
```

$P_0$ and $P_2$ are the endpoints (on the curve). $P_1$ is the off-curve
control point.

```
evaluate(self, t: f64) → Point
```
Point on the curve at parameter $t \in [0,1]$.

**Implementation using de Casteljau** (preferred for numerical stability):
```
q0 = p0.lerp(p1, t)
q1 = p1.lerp(p2, t)
result = q0.lerp(q1, t)
```

Or equivalently, using the Bernstein form:
$$B(t) = (1-t)^2 P_0 + 2(1-t)t P_1 + t^2 P_2$$

```
derivative(self, t: f64) → Point
```
The tangent vector at $t$. This is the velocity of the curve — direction and
speed of travel along the curve.

$$B'(t) = 2\big[(1-t)(P_1 - P_0) + t(P_2 - P_1)\big]$$

```
q = p1.subtract(p0).scale(1.0 - t)
   .add(p2.subtract(p1).scale(t))
   .scale(2.0)
```

Note: this is a vector (a direction), not a normalized unit vector. The caller
can normalize it to get the unit tangent.

```
split(self, t: f64) → (QuadraticBezier, QuadraticBezier)
```
Split at parameter $t$ using de Casteljau. Returns two curves that together
trace the same path as the original.

```
q0 = p0.lerp(p1, t)
q1 = p1.lerp(p2, t)
m  = q0.lerp(q1, t)
left  = QuadraticBezier { p0, q0, m }
right = QuadraticBezier { m, q1, p2 }
```

```
to_polyline(self, tolerance: f64) → Vec<Point>
```
Adaptive subdivision into a polyline (list of points). The algorithm:

1. Test if the curve is "flat enough" by comparing the midpoint of the chord
   $P_0 P_2$ with the curve's actual midpoint at $t=0.5$.
2. If the distance is below `tolerance`, emit the chord endpoints.
3. Otherwise, split at $t=0.5$ and recurse on each half.

```
flatness_test(curve):
    midchord = curve.p0.lerp(curve.p2, 0.5)
    midcurve = curve.evaluate(0.5)
    return midchord.distance(midcurve) < tolerance

to_polyline_recursive(curve, result):
    if flatness_test(curve):
        result.push(curve.p2)   // p0 was already pushed by parent
    else:
        (left, right) = curve.split(0.5)
        to_polyline_recursive(left, result)
        to_polyline_recursive(right, result)

// Bootstrap: push p0, then recurse
```

The output includes both endpoints. A typical rendering system then draws line
segments between consecutive points.

```
bounding_box(self) → Rect
```
Tight axis-aligned bounding box. Algorithm:

1. Find $t_x$ where $B'_x(t) = 0$ (may not exist or may be outside $[0,1]$).
2. Find $t_y$ where $B'_y(t) = 0$.
3. Collect candidate points: $B(0)$, $B(1)$, $B(t_x)$ if valid, $B(t_y)$ if valid.
4. Take component-wise min and max.

```
elevate(self) → CubicBezier
```
Degree elevation: any quadratic bezier can be expressed _exactly_ as a cubic
bezier. The formula (derived from the requirement that the Bernstein
representation matches):

$$Q_0 = P_0$$
$$Q_1 = \frac{1}{3} P_0 + \frac{2}{3} P_1$$
$$Q_2 = \frac{2}{3} P_1 + \frac{1}{3} P_2$$
$$Q_3 = P_2$$

**Why is this exact?** The quadratic Bernstein polynomial $(1-t)^2 P_0 + 2(1-t)tP_1 + t^2 P_2$
can be rewritten by multiplying by $1 = (1-t) + t$:

$$= \big[(1-t)^3 P_0 + (1-t)^2 t \cdot 3 \cdot \tfrac{1}{3}P_0\big]
  + \big[3(1-t)^2 t \cdot \tfrac{2}{3} P_1 + 3(1-t)t^2 \cdot \tfrac{1}{3}P_1\big]
  + \big[t^3 P_2 + (1-t)t^2 \cdot 3 \cdot \tfrac{2}{3} P_2\big]$$

Grouping by cubic Bernstein basis: $Q_1 = \frac{1}{3}P_0 + \frac{2}{3}P_1$,
$Q_2 = \frac{2}{3}P_1 + \frac{1}{3}P_2$ exactly as above.

Use case: backends like PDF and PostScript support only cubic beziers. Degree
elevation converts quadratic font outlines (TrueType) for use in PDF paths
without approximation error.

---

## API Reference — CubicBezier

```
CubicBezier { p0: Point, p1: Point, p2: Point, p3: Point }
```

$P_0$ is the start, $P_3$ is the end. $P_1$ and $P_2$ are off-curve control
points. The tangent at the start points from $P_0$ toward $P_1$; the tangent
at the end points from $P_2$ toward $P_3$.

```
evaluate(self, t: f64) → Point
```
De Casteljau evaluation (three levels of lerp):

```
q0 = p0.lerp(p1, t)
q1 = p1.lerp(p2, t)
q2 = p2.lerp(p3, t)
r0 = q0.lerp(q1, t)
r1 = q1.lerp(q2, t)
return r0.lerp(r1, t)
```

Or using the Bernstein expansion:
$$B(t) = (1-t)^3 P_0 + 3(1-t)^2 t P_1 + 3(1-t)t^2 P_2 + t^3 P_3$$

```
derivative(self, t: f64) → Point
```
The tangent vector:

$$B'(t) = 3\big[(1-t)^2(P_1-P_0) + 2(1-t)t(P_2-P_1) + t^2(P_3-P_2)\big]$$

```
d0 = p1.subtract(p0)
d1 = p2.subtract(p1)
d2 = p3.subtract(p2)
u  = 1.0 - t
return d0.scale(u*u).add(d1.scale(2.0*u*t)).add(d2.scale(t*t)).scale(3.0)
```

Note: the derivative at $t=0$ is $3(P_1 - P_0)$ and at $t=1$ is $3(P_3 - P_2)$.
This is why the control polygon's first and last edges determine the tangent
direction at the endpoints.

```
split(self, t: f64) → (CubicBezier, CubicBezier)
```
De Casteljau split:

```
q0 = p0.lerp(p1, t);  q1 = p1.lerp(p2, t);  q2 = p2.lerp(p3, t)
r0 = q0.lerp(q1, t);  r1 = q1.lerp(q2, t)
s  = r0.lerp(r1, t)
left  = CubicBezier { p0, q0, r0, s  }
right = CubicBezier { s,  r1, q2, p3 }
```

```
to_polyline(self, tolerance: f64) → Vec<Point>
```
Same adaptive subdivision as quadratic — test flatness by comparing midchord
with mid-curve, split at $t=0.5$ and recurse. The flatness test for cubics
can use a slightly tighter criterion: compare _both_ off-curve control points
against the chord:

```
// Tight flatness test for cubics:
chord = p3.subtract(p0)
if chord.magnitude_squared() < ε²:
    return max(p1.distance(p0), p2.distance(p3)) < tolerance
d1 = p1.subtract(p0)   // deviation of p1 from chord direction
d2 = p2.subtract(p0)
// Check the perpendicular component of each control point from the chord
error = max(|d1.cross(chord.normalize())|, |d2.cross(chord.normalize())|)
return error < tolerance
```

Or use the simpler midpoint test (same as quadratic) — it is slightly more
conservative but easier to implement correctly.

```
bounding_box(self) → Rect
```
Find roots of the cubic derivative (which is a quadratic equation in $t$) in
the x and y components independently. For each axis, solve using the quadratic
formula derived in the mathematical background section. Clamp roots to $[0,1]$.
Evaluate $B$ at all roots plus at $t=0$ and $t=1$. Return the min/max rect.

---

## Worked Example: Cubic Bezier Evaluation

Control points: $P_0 = (0, 0)$, $P_1 = (1, 2)$, $P_2 = (3, 2)$, $P_3 = (4, 0)$.

Evaluate at $t = 0.5$:

```
Q0 = lerp((0,0), (1,2), 0.5) = (0.5, 1.0)
Q1 = lerp((1,2), (3,2), 0.5) = (2.0, 2.0)
Q2 = lerp((3,2), (4,0), 0.5) = (3.5, 1.0)

R0 = lerp((0.5,1.0), (2.0,2.0), 0.5) = (1.25, 1.5)
R1 = lerp((2.0,2.0), (3.5,1.0), 0.5) = (2.75, 1.5)

S  = lerp((1.25,1.5), (2.75,1.5), 0.5) = (2.0, 1.5)
```

The curve passes through $(2.0, 1.5)$ at its midpoint. This symmetric result
makes sense: the control polygon is symmetric about $x=2$.

---

## Cross-Language Implementation Notes

### Return types for split

`split` returns two values. Languages differ in how they express this:

| Language   | Return type                                   |
|------------|-----------------------------------------------|
| Rust       | `(CubicBezier, CubicBezier)`                 |
| TypeScript | `[CubicBezier, CubicBezier]` (tuple)         |
| Python     | `tuple[CubicBezier, CubicBezier]`            |
| Go         | `(CubicBezier, CubicBezier)` (multi-return)  |
| Ruby       | `[CubicBezier, CubicBezier]` (array)         |
| Elixir     | `{CubicBezier, CubicBezier}` (tuple)         |
| Lua        | `CubicBezier, CubicBezier` (multi-return)    |
| Perl       | `($left, $right)` (list return)              |
| Swift      | `(CubicBezier, CubicBezier)` (tuple)         |

### Vec<Point> / list / array

`to_polyline` returns a list of points. In Rust: `Vec<Point>`. In TypeScript:
`Point[]`. In Python: `list[Point]`. In Go: `[]Point`. In Ruby: `Array`. In
Elixir: `[Point.t()]`. In Lua: a table with integer keys. In Perl: an array ref.

### Tolerance for to_polyline

A typical pixel-level tolerance for screen rendering is `0.5` (half a pixel).
For print at 300 DPI with a 72 DPI design unit, use `0.1`. The caller chooses
based on their output resolution.

---

## Required Test Coverage

1. **Quadratic evaluate at endpoints**: `q.evaluate(0.0) == p0`, `q.evaluate(1.0) == p2`.
2. **Quadratic evaluate at midpoint**: known value for a specific control polygon.
3. **Quadratic split consistency**: the two halves rejoin at the split point.
4. **Quadratic split: left curve reaches p0**: `left.p0 == original.p0`.
5. **Quadratic split: right curve reaches p2**: `right.p2 == original.p2`.
6. **Quadratic derivative at t=0**: `q.derivative(0.0) ≈ (p1 - p0).scale(2)`.
7. **Quadratic bounding_box includes endpoints**: endpoints are inside the box.
8. **Quadratic elevate**: `q.elevate().evaluate(0.5) ≈ q.evaluate(0.5)`.
9. **Cubic evaluate at endpoints**: `c.evaluate(0.0) == p0`, `c.evaluate(1.0) == p3`.
10. **Cubic evaluate midpoint**: de Casteljau worked example above = `(2.0, 1.5)`.
11. **Cubic split consistency**: midpoints of the two halves meet at the split.
12. **Cubic split: all intermediate points on curve**: `left.p3 == right.p0`.
13. **Cubic derivative at endpoints**: `3*(p1-p0)` at t=0, `3*(p3-p2)` at t=1.
14. **Cubic bounding_box: extrema inside box**: evaluate at multiple t values.
15. **Cubic to_polyline: endpoints included**: first point is p0, last is p3.
16. **Cubic to_polyline: tight tolerance gives more points**: small tolerance → more segments.
17. **Cubic to_polyline: large tolerance gives fewer points**: coarse tolerance → fewer.
18. **Quadratic to_polyline: both endpoints present**.

Coverage threshold: ≥ 95% lines.

---

## Package Matrix

| Language   | Directory                                      | Module/Namespace                          |
|------------|------------------------------------------------|-------------------------------------------|
| Rust       | `code/packages/rust/bezier2d/`                 | `bezier2d`                                |
| TypeScript | `code/packages/typescript/bezier2d/`           | `@coding-adventures/bezier2d`             |
| Python     | `code/packages/python/bezier2d/`               | `bezier2d`                                |
| Ruby       | `code/packages/ruby/bezier2d/`                 | `CodingAdventures::Bezier2D`              |
| Go         | `code/packages/go/bezier2d/`                   | `bezier2d`                                |
| Elixir     | `code/packages/elixir/bezier2d/`               | `CodingAdventures.Bezier2D`               |
| Lua        | `code/packages/lua/bezier2d/`                  | `coding_adventures.bezier2d`              |
| Perl       | `code/packages/perl/bezier2d/`                 | `CodingAdventures::Bezier2D`              |
| Swift      | `code/packages/swift/bezier2d/`                | `Bezier2D`                                |
