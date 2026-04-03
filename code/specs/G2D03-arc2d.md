# G2D03 — Arc2D: Elliptical Arcs with Endpoint↔Center Form Conversion

## Overview

The `arc2d` package provides the mathematics of elliptical arcs — the hardest
2D primitive to implement correctly across rendering backends.

Dependencies:

- `point2d` (G2D00) — Point and Rect
- `bezier2d` (G2D02) — CubicBezier for arc approximation output
- `trig` (PHY00) — sin, cos, atan2 for all angular computations

```
arc2d (G2D03)
├── point2d (G2D00)
├── bezier2d (G2D02)
└── trig (PHY00)
```

### Why elliptical arcs are hard

A circle arc is the path swept by a point moving at constant angular speed
around a center. An elliptical arc is the same, but with different radii in X
and Y, and the ellipse may be rotated relative to the coordinate axes.

The mathematical operations on arcs (evaluation, bounding box, splitting,
converting to beziers) are most natural in **center form**: given the center
point, the two radii, and an angular range.

But 2D path commands (SVG, PDF, PostScript, Canvas) specify arcs in **endpoint
form**: given the start point (where the pen currently is), the end point, and
the ellipse parameters. You never specify the center in a path command because
you don't know it ahead of time — you know where you are and where you want to
go.

The core challenge: converting between these two forms requires solving a
system of equations whose geometry has four possible solutions. The SVG
specification includes a worked algorithm; this package implements it exactly.

---

## The Two Parameterizations

### Endpoint Form (SVG `A` command)

```
SvgArc {
    from:       Point,    // current path position (start of arc)
    to:         Point,    // end point
    rx:         f64,      // x-radius (≥ 0)
    ry:         f64,      // y-radius (≥ 0)
    x_rotation: f64,      // rotation of ellipse x-axis, radians
    large_arc:  bool,     // true = use the arc spanning > 180°
    sweep:      bool,     // true = counterclockwise arc, false = clockwise
}
```

**Why endpoint form exists**: when building a path with a sequence of commands,
you always know where the pen currently is (`from`) and where you want the arc
to end (`to`). The center of the ellipse is an intermediate calculation you
rarely care about. Specifying `center + angles` would require the user to solve
the geometry before issuing the command.

**The four-way ambiguity**: given two points and an ellipse shape, there are
generally _four_ possible arcs:

```
        ·─────────────·
       /  (large CCW)  \       ·─────────────·
      /                 \     / (large CW)    \
   from                 to from               to
      \                 /     \               /
       \ (small CCW)  /        \ (small CW) /
        ·─────────────·         ·───────────·

   sweep=true           sweep=false
   large_arc=true       large_arc=true

        from···to              from···to
         ╰─────╯                ╭─────╮
   (small CCW)              (small CW)
   sweep=true               sweep=false
   large_arc=false          large_arc=false
```

- `sweep=true` means counterclockwise (positive angular direction in math
  convention, where Y is up). Note: in screen coordinates with Y-down,
  `sweep=true` visually appears clockwise.
- `large_arc=true` selects the arc spanning more than 180°.

### Center Form

```
CenterArc {
    center:      Point,   // center of the ellipse
    rx:          f64,     // x-radius (semi-major axis along rotated x)
    ry:          f64,     // y-radius (semi-minor axis along rotated y)
    start_angle: f64,     // angle to start point, radians
    sweep_angle: f64,     // angular span: positive=CCW, negative=CW
    x_rotation:  f64,     // rotation of ellipse x-axis, radians
}
```

A point on the ellipse at angle $\phi$ (measured in the ellipse's local
coordinate system) is:

$$\begin{pmatrix} x \\ y \end{pmatrix} = \begin{pmatrix} \cos\theta & -\sin\theta \\ \sin\theta & \cos\theta \end{pmatrix} \begin{pmatrix} r_x \cos\phi \\ r_y \sin\phi \end{pmatrix} + \begin{pmatrix} c_x \\ c_y \end{pmatrix}$$

where $\theta$ is `x_rotation` and $(c_x, c_y)$ is the center. Written out:

$$x = \cos\theta \cdot r_x \cos\phi - \sin\theta \cdot r_y \sin\phi + c_x$$
$$y = \sin\theta \cdot r_x \cos\phi + \cos\theta \cdot r_y \sin\phi + c_y$$

The parameter $t \in [0,1]$ maps to angle $\phi = \text{start\_angle} + t \cdot \text{sweep\_angle}$.

---

## Endpoint → Center Form Conversion

This is the W3C SVG specification algorithm, Section F.6.5 (formerly B.2.4).
The derivation is involved; the spec walks through it carefully. We reproduce
it here with explanations.

### Step 0: Handle degenerate cases

Before any computation, handle these edge cases:

1. **`from == to`**: the arc has zero length. Return `None` (degenerate).
2. **`rx == 0` or `ry == 0`**: the ellipse degenerates to a line segment.
   Treat as a line segment, not an arc. Return `None`.

### Step 1: Transform to the rotated coordinate system

Rotate the start and end points by $-\theta$ (inverse of the ellipse rotation)
to work in the ellipse's local coordinate system. Let $\theta = \text{x\_rotation}$.

$$\begin{pmatrix} x_1' \\ y_1' \end{pmatrix} = \begin{pmatrix} \cos\theta & \sin\theta \\ -\sin\theta & \cos\theta \end{pmatrix} \cdot \frac{1}{2}\begin{pmatrix} f_x - t_x \\ f_y - t_y \end{pmatrix}$$

where $(f_x, f_y) = \text{from}$ and $(t_x, t_y) = \text{to}$.

In code:

```
dx = (from.x - to.x) / 2.0
dy = (from.y - to.y) / 2.0
cos_r = trig.cos(x_rotation)
sin_r = trig.sin(x_rotation)
x1p =  cos_r * dx + sin_r * dy
y1p = -sin_r * dx + cos_r * dy
```

### Step 2: Ensure radii are large enough

The two given radii may be too small to connect `from` and `to` on an ellipse.
The spec requires us to scale them up uniformly if so:

$$\Lambda = \frac{x_1'^2}{r_x^2} + \frac{y_1'^2}{r_y^2}$$

If $\Lambda > 1$, scale both radii:

$$r_x \leftarrow \sqrt{\Lambda} \cdot r_x, \quad r_y \leftarrow \sqrt{\Lambda} \cdot r_y$$

This scaling preserves the aspect ratio of the ellipse.

```
lambda = (x1p/rx)^2 + (y1p/ry)^2
if lambda > 1.0:
    sqrt_lambda = sqrt(lambda)
    rx *= sqrt_lambda
    ry *= sqrt_lambda
```

### Step 3: Compute the center in the rotated system

The center $(c_x', c_y')$ in the rotated coordinate system satisfies the
constraint that both `from'` and `to'` lie on the ellipse. The W3C formula:

$$\text{sign} = \begin{cases} +1 & \text{if large\_arc} \ne \text{sweep} \\ -1 & \text{otherwise} \end{cases}$$

$$\text{sq} = \sqrt{\max\!\left(0,\; \frac{r_x^2 r_y^2 - r_x^2 y_1'^2 - r_y^2 x_1'^2}{r_x^2 y_1'^2 + r_y^2 x_1'^2}\right)}$$

$$c_x' = \text{sign} \cdot \text{sq} \cdot \frac{r_x y_1'}{r_y}$$

$$c_y' = \text{sign} \cdot \text{sq} \cdot \frac{-r_y x_1'}{r_x}$$

The $\max(0, \cdot)$ guards against tiny negative values due to floating-point
error when $\Lambda$ is exactly 1 (or very slightly over 1 after scaling).

The `sign` formula: `large_arc` and `sweep` together select among the four
possible arcs. When they differ (one is true, the other false), the center is on
the positive side; when they agree, on the negative side.

```
num = rx^2 * ry^2 - rx^2 * y1p^2 - ry^2 * x1p^2
den = rx^2 * y1p^2 + ry^2 * x1p^2
sq = sqrt(max(0.0, num / den))
sign = if large_arc != sweep { 1.0 } else { -1.0 }
cxp = sign * sq * (rx * y1p / ry)
cyp = sign * sq * (-ry * x1p / rx)
```

### Step 4: Rotate center back to the original coordinate system

$$\begin{pmatrix} c_x \\ c_y \end{pmatrix} = \begin{pmatrix} \cos\theta & -\sin\theta \\ \sin\theta & \cos\theta \end{pmatrix} \begin{pmatrix} c_x' \\ c_y' \end{pmatrix} + \frac{1}{2}\begin{pmatrix} f_x + t_x \\ f_y + t_y \end{pmatrix}$$

```
cx = cos_r * cxp - sin_r * cyp + (from.x + to.x) / 2.0
cy = sin_r * cxp + cos_r * cyp + (from.y + to.y) / 2.0
center = Point::new(cx, cy)
```

### Step 5: Compute start_angle and sweep_angle

The angle of a vector $(u_x, u_y)$ from the ellipse's local x-axis is:

$$\text{angle}(\mathbf{u}) = \text{atan2}(u_y, u_x)$$

The start angle is the angle of the vector from the center (in rotated coords)
to the transformed start point:

```
start_angle = atan2((y1p - cyp) / ry, (x1p - cxp) / rx)
```

The total angular sweep is the angle of the vector from the start direction to
the end direction:

```
raw_sweep = atan2((-y1p - cyp) / ry, (-x1p - cxp) / rx) - start_angle
```

Adjust the raw sweep to match the `sweep` flag:

```
if sweep == false and raw_sweep > 0.0:
    raw_sweep -= 2π
if sweep == true and raw_sweep < 0.0:
    raw_sweep += 2π
```

Then clamp the magnitude to $2\pi$ (a full ellipse):

```
sweep_angle = clamp(raw_sweep, -2π, 2π)
```

All calls to `atan2` must use `trig.atan2` from PHY00.

### Complete result

```
CenterArc {
    center,
    rx,           // (possibly scaled up in step 2)
    ry,
    start_angle,
    sweep_angle,
    x_rotation,
}
```

**Worked example**: `from=(1,0)`, `to=(-1,0)`, `rx=1`, `ry=1`,
`x_rotation=0`, `large_arc=false`, `sweep=true`:

```
dx = (1 - (-1)) / 2 = 1.0,  dy = 0.0
x1p = cos(0)*1 + sin(0)*0 = 1.0
y1p = -sin(0)*1 + cos(0)*0 = 0.0

lambda = (1/1)^2 + (0/1)^2 = 1.0  → radii OK, no scaling

num = 1*1 - 1*0 - 1*1 = 0
sq = sqrt(0) = 0
sign = (false != true) = 1.0  (large_arc != sweep)
cxp = 1 * 0 * (1*0/1) = 0
cyp = 1 * 0 * (-1*1/1) = 0
→ center in rotated coords = (0, 0)

cx = cos(0)*0 - sin(0)*0 + (1 + (-1))/2 = 0
cy = sin(0)*0 + cos(0)*0 + (0+0)/2 = 0
center = (0, 0)  ✓ (the center of the unit circle is the origin)

start_angle = atan2((0-0)/1, (1-0)/1) = atan2(0, 1) = 0
raw_sweep = atan2((-0-0)/1, (-1-0)/1) - 0 = atan2(0, -1) = π
sweep=true and raw_sweep=π > 0: no adjustment needed
sweep_angle = π  (half circle CCW)
```

The arc goes CCW from angle 0 to angle π — from (1,0) to (-1,0) along the
top of the unit circle. Correct.

---

## API Reference — CenterArc

### Evaluation

```
evaluate(self, t: f64) → Point
```
Point at parameter $t \in [0,1]$. Maps $t$ to angle:

$$\phi = \text{start\_angle} + t \cdot \text{sweep\_angle}$$

Then evaluates the ellipse parametric equation:

```
phi = start_angle + t * sweep_angle
cos_r = trig.cos(x_rotation)
sin_r = trig.sin(x_rotation)
xp = rx * trig.cos(phi)
yp = ry * trig.sin(phi)
x = cos_r * xp - sin_r * yp + center.x
y = sin_r * xp + cos_r * yp + center.y
→ Point::new(x, y)
```

```
tangent(self, t: f64) → Point
```
The derivative with respect to $t$. Differentiating the parametric equation:

$$\frac{dx}{dt} = -r_x \sin(\phi) \cdot \frac{d\phi}{dt} \cdot \cos\theta - r_y \cos(\phi) \cdot \frac{d\phi}{dt} \cdot \sin\theta$$

$$\frac{dy}{dt} = -r_x \sin(\phi) \cdot \frac{d\phi}{dt} \cdot \sin\theta + r_y \cos(\phi) \cdot \frac{d\phi}{dt} \cdot \cos\theta$$

where $\frac{d\phi}{dt} = \text{sweep\_angle}$.

```
phi = start_angle + t * sweep_angle
dphi = sweep_angle
dxp = -rx * trig.sin(phi) * dphi
dyp =  ry * trig.cos(phi) * dphi
dx = cos_r * dxp - sin_r * dyp
dy = sin_r * dxp + cos_r * dyp
→ Point::new(dx, dy)
```

### Bounding Box

```
bounding_box(self) → Rect
```
The extrema of an elliptical arc occur where the tangent is horizontal (x
extremum) or vertical (y extremum). For the non-rotated ellipse ($\theta = 0$):

$$\frac{dx}{d\phi} = -r_x \sin\phi = 0 \implies \phi = 0, \pi$$

$$\frac{dy}{d\phi} = r_y \cos\phi = 0 \implies \phi = \pi/2, 3\pi/2$$

For a rotated ellipse, mix the two channels. The x-extrema occur at:

$$\tan\phi = -\frac{r_y \sin\theta}{r_x \cos\theta} = -\frac{r_y}{r_x} \tan\theta$$

Use `trig.atan2` to find the principal value, then add $\pi$ for the second
solution:

```
phi_x1 = trig.atan2(-ry * sin_r, rx * cos_r)
phi_x2 = phi_x1 + π
phi_y1 = trig.atan2( ry * cos_r, rx * sin_r)
phi_y2 = phi_y1 + π
```

For each candidate angle $\phi$, convert to parameter $t$:

```
t = (phi - start_angle) / sweep_angle
```

Keep only $t \in [0,1]$. Evaluate the arc at those $t$ values plus $t=0$ and
$t=1$. Take the component-wise min and max.

### Arc-to-Cubic-Bezier Approximation

```
to_cubic_beziers(self) → Vec<CubicBezier>
```
Convert the arc to a sequence of cubic bezier curves. This is required for
backends that do not natively support elliptical arcs: Metal, many PDF
generators, custom rasterizers, and any system that wants to reduce its
rendering primitive set to just bezier segments.

**Algorithm**: split the arc into segments of at most 90° each. Approximate
each segment with a single cubic bezier.

```
n_segments = ceil(|sweep_angle| / (π/2))
seg_sweep = sweep_angle / n_segments

for i in 0..n_segments:
    seg_start = start_angle + i * seg_sweep
    emit approximate_segment(center, rx, ry, x_rotation, seg_start, seg_sweep)
```

For each 90°-or-less segment, the four cubic bezier control points are:

```
// Start and end angles of this segment
a1 = seg_start_angle
a2 = seg_start_angle + seg_sweep

// Points on the ellipse at start and end
P0 = evaluate_at_angle(a1)
P3 = evaluate_at_angle(a2)

// Tangent vectors at start and end (unit tangent, direction only)
T1 = tangent_at_angle(a1).normalize()
T2 = tangent_at_angle(a2).normalize()

// The magic constant
k = (4.0 / 3.0) * trig.tan(seg_sweep / 4.0)

// Tangent magnitude at the endpoints (scaled by k * arc radius)
P1 = P0.add(T1.scale(k * rx))   // approximation for unit circle; see below
P2 = P3.subtract(T2.scale(k * rx))
```

The formula for `P1` and `P2` when the ellipse is **axis-aligned and
unrotated** ($\theta = 0$):

$$P_1 = P_0 + k \cdot (-r_x \sin(a_1), \; r_y \cos(a_1))$$
$$P_2 = P_3 - k \cdot (-r_x \sin(a_2), \; r_y \cos(a_2))$$

For a rotated ellipse, apply the $\theta$-rotation matrix to the tangent.

The complete formula using tangent evaluation:

```
tangent_at_angle(phi):
    dxp = -rx * trig.sin(phi)
    dyp =  ry * trig.cos(phi)
    dx = cos_r * dxp - sin_r * dyp
    dy = sin_r * dxp + cos_r * dyp
    return Point::new(dx, dy)    // NOT normalized — magnitude encodes speed

P1 = P0.add(tangent_at_angle(a1).scale(k))
P2 = P3.subtract(tangent_at_angle(a2).scale(k))
```

Note: the tangent vector here is _not_ normalized. Its magnitude already
encodes the arc speed $r_x$ and $r_y$, so `scale(k)` places the control
points at the right distance.

---

## The Magic Constant $k = \frac{4}{3}\tan\!\left(\frac{\theta}{4}\right)$

This constant is the soul of arc-to-bezier approximation. Here is where it
comes from.

### Setup

Consider a unit circle arc from angle 0 to angle $\theta$ (with $0 < \theta \le \pi/2$).
The arc starts at $(1, 0)$ and ends at $(\cos\theta, \sin\theta)$.

We want to approximate this with a single cubic bezier:
- $P_0 = (1, 0)$ — start (on the circle)
- $P_3 = (\cos\theta, \sin\theta)$ — end (on the circle)
- $P_1 = (1, \alpha)$ — control point at start (tangent to circle at P0 is vertical)
- $P_2 = (\cos\theta + \alpha\sin\theta, \sin\theta - \alpha\cos\theta)$ — rotated tangent

The tangent at $P_0 = (1,0)$ is $(0,1)$ (straight up). The tangent at
$P_3 = (\cos\theta, \sin\theta)$ is $(-\sin\theta, \cos\theta)$ (perpendicular
to the radius). We place the control points at distance $\alpha$ along these
tangents.

### Finding the optimal $\alpha$

The optimal $\alpha$ minimizes the maximum deviation of the bezier from the
true circle arc. By symmetry, the midpoint of the bezier (at $t=0.5$) should
lie on the circle.

The midpoint of the cubic bezier at $t = 0.5$:

$$B(0.5) = \frac{1}{8}(P_0 + 3P_1 + 3P_2 + P_3)$$

The midpoint of the arc is at angle $\theta/2$:
$M_\text{arc} = (\cos(\theta/2), \sin(\theta/2))$.

Setting $|B(0.5)| = 1$ (midpoint lies on unit circle) and solving for $\alpha$:

$$\alpha = \frac{4}{3} \cdot \frac{1 - \cos(\theta/2)}{\sin(\theta/2)}$$

Using the half-angle identity $1 - \cos(\theta/2) = 2\sin^2(\theta/4)$ and
$\sin(\theta/2) = 2\sin(\theta/4)\cos(\theta/4)$:

$$\alpha = \frac{4}{3} \cdot \frac{2\sin^2(\theta/4)}{2\sin(\theta/4)\cos(\theta/4)} = \frac{4}{3}\tan\!\left(\frac{\theta}{4}\right)$$

This is the magic constant. It is exact when $\theta = 0$ ($\alpha = 0$) and
is a very good approximation for $\theta \le \pi/2$.

### For a quarter-circle ($\theta = \pi/2$):

$$k = \frac{4}{3}\tan\!\left(\frac{\pi}{8}\right) \approx \frac{4}{3} \times 0.41421 \approx 0.55228$$

```
Control points for a quarter-circle from (1,0) to (0,1):
  P0 = (1, 0)
  P1 = (1, 0.55228)          // k up from P0
  P2 = (0.55228, 1)          // k left from P3
  P3 = (0, 1)
```

The maximum error of this approximation compared to the true circle is
approximately $2.8 \times 10^{-4}$ radii — about 0.3 pixels at a radius of
1000 pixels. Four segments (four quarter-circles) approximate a full circle
with sub-pixel accuracy.

```
Quarter-circle approximation:

       P1=(1, k)
        |
  P0=(1,0)                     (center is at origin)
        ↑
        ╰─── tangent direction (up)

        (0,1)=P3
       /
    P2=(k, 1)
       ↑
       ╰─── tangent direction (left)

The bezier P0→P1→P2→P3 hugs the unit circle closely,
with the maximum deviation at ~54° ≈ 0.00028 radii.
```

---

## API Reference — SvgArc

```
SvgArc { from, to, rx, ry, x_rotation, large_arc, sweep }
```

```
to_center_arc(self) → Option<CenterArc>
```
Convert to center form. Returns `None` for degenerate arcs (`from == to`,
`rx == 0`, `ry == 0`). Implements the W3C algorithm above.

```
to_cubic_beziers(self) → Vec<CubicBezier>
```
Convenience: `to_center_arc()?.to_cubic_beziers()` or empty vec if degenerate.

```
evaluate(self, t: f64) → Point
```
Delegates to `to_center_arc()?.evaluate(t)` or `lerp(from, to, t)` for
degenerate (line segment) case.

```
bounding_box(self) → Rect
```
Delegates to `to_center_arc()?.bounding_box()` or `Rect::from_points(from, to)`
for degenerate case.

---

## Cross-Language Implementation Notes

### trig dependency

Every angular computation must use PHY00:
- `trig.sin`, `trig.cos` — for rotate/evaluate
- `trig.atan2` — for angle computation in `to_center_arc`
- `trig.tan` — for the magic constant `k` in `to_cubic_beziers`

PHY00 currently implements `sin`, `cos`, and the conversion helpers. If it does
not export `atan2` or `tan`, those functions may be implemented locally within
`arc2d`:

```
tan(x) = trig.sin(x) / trig.cos(x)
atan2(y, x) — must be provided by trig or implemented here
```

Check the current PHY00 API. If `atan2` is missing, file a task to add it to
PHY00 before implementing G2D03.

### Option / null / nil

`to_center_arc` returns an optional. Follow the same cross-language conventions
as G2D00 (see Point2D spec).

### Vec<CubicBezier>

An arc spanning 360° produces 4 cubic bezier segments. An arc spanning 91°
produces 2. An arc spanning 45° produces 1. The output list is never empty
for a non-degenerate arc — a zero-sweep arc produces a single degenerate cubic
where all four control points coincide.

### Floating-point edge cases

- `rx` and `ry` very close to zero (but not exactly zero): treat as degenerate
  if `rx < 1e-10 || ry < 1e-10`.
- `from` very close to `to`: degenerate if `from.distance_squared(to) < 1e-20`.
- `sqrt(max(0, ...))` in step 3: clamp to 0 explicitly to prevent `sqrt` of
  tiny negative values from producing NaN.

---

## Required Test Coverage

1. **Endpoint→center, quarter-circle**: `from=(1,0)`, `to=(0,1)`, `rx=ry=1`,
   `x_rotation=0`, `large_arc=false`, `sweep=true` → center=(0,0),
   `start_angle≈0`, `sweep_angle≈π/2`.

2. **Endpoint→center, half-circle**: worked example from conversion section.
   `from=(1,0)`, `to=(-1,0)` → center=(0,0), sweep_angle=π.

3. **Degenerate: from==to**: `to_center_arc()` returns None.

4. **Degenerate: rx=0**: `to_center_arc()` returns None.

5. **Large_arc flag**: same `from`, `to`, `rx`, `ry` with `large_arc=true`
   and `large_arc=false` produce arcs with different sweep angles (one > π,
   one < π).

6. **Sweep flag**: `sweep=true` gives `sweep_angle > 0`; `sweep=false` gives
   `sweep_angle < 0`.

7. **CenterArc evaluate at t=0**: returns the start point.

8. **CenterArc evaluate at t=1**: returns the end point.

9. **CenterArc evaluate at t=0.5**: midpoint of a semicircular arc on a unit
   circle is at (0, 1) or the appropriate perpendicular.

10. **Radius scaling**: arc with `rx=0.1`, `from=(0,0)`, `to=(10,0)` — radii
    are too small; they get scaled up and `to_center_arc` returns Some.

11. **to_cubic_beziers for quarter-circle**: returns exactly 1 CubicBezier.

12. **to_cubic_beziers for full-circle**: returns exactly 4 CubicBeziers.

13. **to_cubic_beziers continuity**: the end point of segment $i$ equals the
    start point of segment $i+1$ (within floating-point tolerance).

14. **to_cubic_beziers endpoints**: the first bezier starts at the arc start;
    the last bezier ends at the arc end.

15. **bounding_box contains start and end**: both `from` and `to` are inside
    the bounding box.

16. **bounding_box for axis-aligned quarter-circle**: unit circle from (1,0) to
    (0,1) CCW → bounding box is approximately [0,1]×[0,1].

17. **SvgArc.to_cubic_beziers delegates correctly**: result matches CenterArc
    path.

18. **Tangent at endpoints**: `tangent(0)` is perpendicular to the ellipse
    radius at `start_angle`; `tangent(1)` is perpendicular at the end angle.

19. **Magic constant k**: `k = (4/3) * tan(π/8) ≈ 0.5523` for quarter-circle.

20. **x_rotation non-zero**: arc with `x_rotation=π/4` — start/end points and
    bounding box rotate accordingly.

Coverage threshold: ≥ 95% lines.

---

## Package Matrix

| Language   | Directory                                      | Module/Namespace                         |
|------------|------------------------------------------------|------------------------------------------|
| Rust       | `code/packages/rust/arc2d/`                    | `arc2d`                                  |
| TypeScript | `code/packages/typescript/arc2d/`              | `@coding-adventures/arc2d`               |
| Python     | `code/packages/python/arc2d/`                  | `arc2d`                                  |
| Ruby       | `code/packages/ruby/arc2d/`                    | `CodingAdventures::Arc2D`                |
| Go         | `code/packages/go/arc2d/`                      | `arc2d`                                  |
| Elixir     | `code/packages/elixir/arc2d/`                  | `CodingAdventures.Arc2D`                 |
| Lua        | `code/packages/lua/arc2d/`                     | `coding_adventures.arc2d`                |
| Perl       | `code/packages/perl/arc2d/`                    | `CodingAdventures::Arc2D`                |
| Swift      | `code/packages/swift/arc2d/`                   | `Arc2D`                                  |

---

## Implementation Checklist

Before marking any language implementation complete, verify:

- [ ] `SvgArc::to_center_arc` implements all 5 W3C steps with the degenerate
      guards in place.
- [ ] All `trig.*` calls go through PHY00 — no direct stdlib math.
- [ ] `sqrt(max(0, ...))` clamp present in step 3 of conversion.
- [ ] `to_cubic_beziers` splits into ceil(|sweep|/(π/2)) segments, never more.
- [ ] The magic constant uses `trig.tan(sweep/4)` — not a hardcoded 0.5523.
- [ ] Test suite includes a non-zero `x_rotation` case.
- [ ] `bounding_box` finds both x-extrema and y-extrema angles, not just the
      four cardinal points.
