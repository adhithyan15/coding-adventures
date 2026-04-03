# G2D00 — Point2D: The Atomic 2D Geometric Primitive

## Overview

The `point2d` package defines the foundational data types for all 2D geometry
in the PaintVM rendering system. It provides two structures:

- **`Point`** — a 2D position _and_ a 2D vector. In Euclidean geometry, a
  position ("where is something?") and a direction+magnitude ("how far and
  which way?") are represented by the same mathematical object: a pair of real
  numbers (x, y). Context determines whether you treat the pair as a position
  or a vector.

- **`Rect`** — an axis-aligned bounding box (AABB) given by an origin corner
  plus a width and height. Bounding boxes appear everywhere: hit-testing, dirty-
  region tracking, clipping, conservative intersection tests.

This package is the **leaf node** of the G2D dependency tree. It depends only
on `trig` (PHY00) for trigonometric functions and on nothing else.

```
       affine2d (G2D01)
      /
point2d (G2D00) ← bezier2d (G2D02) ← arc2d (G2D03)
      \
       trig (PHY00)
```

### Why a separate package from ML03 matrix?

The `matrix` package (ML03) is a general N×M matrix algebra library designed
for machine learning: its primary operations are dot products over large arrays,
transpositions, and gradient computations on rectangular grids of floats.

`point2d` is a _fixed-size, specialized_ abstraction for 2D geometry. Using an
N×M matrix to represent a 2D point would be:

- **Wasteful in memory** — a 2×1 matrix object carries a `rows`, `cols`, and a
  heap-allocated `data` field. A Point is just two floats: 16 bytes, stack-
  allocated in every systems language.
- **Wrong vocabulary** — you don't "transpose" a point or "broadcast-add" two
  points. You compute a cross product, a perpendicular, or a lerp. These
  concepts don't exist in general matrix algebra.
- **Slower** — general matrix multiply is O(n³). Point operations are O(1).

The analogy: a general matrix library is to `point2d` what a spreadsheet is to
storing a phone number. Use the right tool for the right job.

---

## The Mathematics of 2D Vectors

Before the API, it is worth understanding _why_ a position and a vector are the
same type.

In the Cartesian plane, we fix an origin O and two unit axes. Any point P in
the plane can be described by two numbers (x, y) — the signed distances along
each axis. The _vector from O to P_ is also described by exactly (x, y). The
arithmetic of positions (translations, relative displacements) is identical to
the arithmetic of vectors (addition, scaling, dot products). Treating them as
one type eliminates entire classes of representation-mismatch bugs.

### Vector Addition

If point A is at (1, 2) and we add vector B = (3, 4), we arrive at C = (4, 6).
Geometrically: start at A, walk 3 units right and 4 units up, land at C.

$$P_C = P_A + P_B = (A_x + B_x, \; A_y + B_y)$$

### Scalar Multiplication

Scaling a vector by a scalar $s$ stretches (or shrinks, or flips) it:

$$s \cdot P = (s \cdot x, \; s \cdot y)$$

If $s = 0.5$, the vector is halved. If $s = -1$, the vector is negated
(reversed direction). If $s = 2$, the vector doubles in length.

### Dot Product

The dot product of two vectors $\mathbf{u} = (u_x, u_y)$ and
$\mathbf{v} = (v_x, v_y)$ is:

$$\mathbf{u} \cdot \mathbf{v} = u_x v_x + u_y v_y$$

Geometrically, the dot product encodes the angle $\theta$ between them:

$$\mathbf{u} \cdot \mathbf{v} = |\mathbf{u}| \, |\mathbf{v}| \cos\theta$$

- If the dot product is **positive**, the vectors point _roughly_ in the same
  direction ($\theta < 90°$).
- If it is **zero**, the vectors are **perpendicular** ($\theta = 90°$).
- If it is **negative**, they point in _opposite_ directions ($\theta > 90°$).

The dot product is used for projections (how much of $\mathbf{u}$ lies along
$\mathbf{v}$?) and for checking perpendicularity.

### Cross Product (2D scalar)

In 3D, the cross product of two vectors produces a third vector perpendicular
to both. In 2D, both input vectors lie in the XY plane, so their cross product
is purely a Z-component scalar:

$$\mathbf{u} \times \mathbf{v} = u_x v_y - u_y v_x$$

This scalar tells you the **orientation** of the turn from $\mathbf{u}$ to
$\mathbf{v}$:

- **Positive**: $\mathbf{v}$ is to the **left** of $\mathbf{u}$ (
  counterclockwise turn).
- **Negative**: $\mathbf{v}$ is to the **right** of $\mathbf{u}$ (clockwise
  turn).
- **Zero**: $\mathbf{u}$ and $\mathbf{v}$ are collinear (parallel or
  anti-parallel).

```
           v (to the left of u)
          /
         /  cross > 0
        /
       O ─────────────→ u
        \
         \  cross < 0
          \
           v (to the right of u)
```

**Use cases**: winding number algorithms (determining if a point is inside a
polygon), orientation tests (is a polygon clockwise or counterclockwise?), and
convex hull construction.

### Magnitude and the Pythagorean Theorem

The length of a vector $(x, y)$ is the Euclidean distance from the origin:

$$|\mathbf{v}| = \sqrt{x^2 + y^2}$$

Computing a square root is expensive (many CPU cycles). When you only need to
_compare_ lengths (is $|\mathbf{u}| > |\mathbf{v}|$?), you can compare the
squares instead and avoid the sqrt entirely:

$$|\mathbf{u}|^2 > |\mathbf{v}|^2 \iff |\mathbf{u}| > |\mathbf{v}|$$

This is `magnitude_squared()`. Use it whenever you do not need the actual
distance value.

### Normalization

A **unit vector** (or normalized vector) has magnitude exactly 1. To normalize:

$$\hat{\mathbf{v}} = \frac{\mathbf{v}}{|\mathbf{v}|} = \left(\frac{x}{|\mathbf{v}|}, \frac{y}{|\mathbf{v}|}\right)$$

Unit vectors represent _pure directions_ with no magnitude information. They
appear everywhere: face normals, tangent directions on curves, perpendicular
offsets.

Edge case: if $|\mathbf{v}| = 0$ (the zero vector has no direction), we return
the origin (0, 0) rather than dividing by zero.

### Linear Interpolation (Lerp)

Lerp is the Swiss Army knife of computer graphics. Given two points $A$ and $B$
and a parameter $t \in [0, 1]$:

$$\text{lerp}(A, B, t) = A + t(B - A) = (1-t)A + tB$$

- $t = 0$: returns $A$.
- $t = 1$: returns $B$.
- $t = 0.5$: returns the midpoint.
- $t$ outside $[0, 1]$: extrapolates beyond the segment.

Lerp is the building block of de Casteljau's algorithm for Bezier curves
(G2D02), of easing functions for animation, and of gradient interpolation in
rasterizers.

### Perpendicular Rotation

Rotating a vector 90° counterclockwise is a trivial formula:

$$\text{perp}(x, y) = (-y, x)$$

Proof: if the original vector has angle $\theta$ (i.e., $x = r\cos\theta$,
$y = r\sin\theta$), then $(-y, x) = (-r\sin\theta, r\cos\theta)$. Using the
trigonometric identities $\cos(\theta + 90°) = -\sin\theta$ and
$\sin(\theta + 90°) = \cos\theta$, this is the vector at angle $\theta + 90°$.

Perpendiculars are used for: curve normals (offsetting a path outward),
stroking (placing the stroke boundary perpendicular to the tangent), and
computing right-hand sides of directed line segments.

### Angle of a Vector

The angle of vector $(x, y)$ measured counterclockwise from the positive X
axis:

$$\theta = \text{atan2}(y, x)$$

The two-argument arctangent `atan2` handles all four quadrants correctly
(unlike `atan(y/x)` which is ambiguous in the third and fourth quadrants).
Result is in $[-\pi, \pi]$ radians.

**Implementation note**: always use `trig.atan2(y, x)` from PHY00, not the
host language's standard library, to maintain first-principles consistency.

---

## API Reference — Point

All operations produce **new** values and leave the original unchanged (the
immutable/value-type pattern). This makes concurrent use safe and eliminates
aliasing bugs.

### Construction

```
Point::new(x: f64, y: f64) → Point
```
Create a point at position (x, y) — or equivalently the vector (x, y) from
the origin.

```
Point::origin() → Point
```
The additive identity: (0.0, 0.0). The starting point of any coordinate system.

### Arithmetic

```
add(self, other: Point) → Point
```
Element-wise addition: $(x_1 + x_2, \; y_1 + y_2)$.

In graphics, adding a **displacement vector** to a **position** yields a new
position. Adding two displacement vectors yields a combined displacement.

```
subtract(self, other: Point) → Point
```
Element-wise subtraction: $(x_1 - x_2, \; y_1 - y_2)$.

The vector from point B to point A is `A.subtract(B)`. The length of that
vector is the distance from B to A.

```
scale(self, s: f64) → Point
```
Scalar multiplication: $(s \cdot x, \; s \cdot y)$.

```
negate(self) → Point
```
Additive inverse: $(-x, -y)$. Equivalent to `scale(-1.0)`.

### Vector Operations

```
dot(self, other: Point) → f64
```
$x_1 x_2 + y_1 y_2$. See mathematical background above.

```
cross(self, other: Point) → f64
```
$x_1 y_2 - y_1 x_2$. The 2D cross product scalar. Positive means `other` is
counterclockwise from (to the left of) `self`.

```
magnitude(self) → f64
```
$\sqrt{x^2 + y^2}$. Uses `trig` package for the square root if the trig
package provides it; otherwise may use the language's sqrt (which is
platform-provided arithmetic, not a transcendental function).

**Implementation note**: sqrt is not a transcendental function (it is computed
by hardware in one or two instructions on modern CPUs). Using the host
language's sqrt for magnitude is acceptable. Use `trig.sin`/`trig.cos`/
`trig.atan2` for the trigonometric operations.

```
magnitude_squared(self) → f64
```
$x^2 + y^2$. Cheaper than `magnitude()` — no sqrt. Prefer this whenever you
are comparing distances or checking if a vector is zero.

```
normalize(self) → Point
```
The unit vector in the same direction. Returns `Point::origin()` if the
magnitude is zero (rather than dividing by zero).

```
distance(self, other: Point) → f64
```
Euclidean distance between two positions: `self.subtract(other).magnitude()`.

```
distance_squared(self, other: Point) → f64
```
`self.subtract(other).magnitude_squared()`. No sqrt.

### Interpolation and Direction

```
lerp(self, other: Point, t: f64) → Point
```
Linear interpolation. $\text{self} + t \cdot (\text{other} - \text{self})$.
When $t = 0$, returns self. When $t = 1$, returns other. Values outside [0,1]
extrapolate beyond the segment.

**Worked example**:
```
A = Point::new(1.0, 0.0)
B = Point::new(5.0, 4.0)
A.lerp(B, 0.25) = (1.0 + 0.25*(5.0-1.0),  0.0 + 0.25*(4.0-0.0))
               = (1.0 + 1.0,  0.0 + 1.0)
               = (2.0, 1.0)
```

```
perpendicular(self) → Point
```
Rotate 90° counterclockwise: $(-y, x)$. Result has the same magnitude as
self. Calling `perpendicular()` twice returns the negation of the original
(180° total rotation): `p.perpendicular().perpendicular() == p.negate()`.

```
angle(self) → f64
```
Direction angle in radians: $\text{atan2}(y, x)$. Must call
`trig.atan2(y, x)` from PHY00. Result is in $(-\pi, \pi]$.

**Worked example**:
```
Point::new(1.0, 0.0).angle()  →  0.0       (points right, 0°)
Point::new(0.0, 1.0).angle()  →  π/2       (points up, 90°)
Point::new(-1.0, 0.0).angle() →  π         (points left, 180°)
Point::new(0.0, -1.0).angle() →  -π/2      (points down, -90°)
```

---

## API Reference — Rect

An axis-aligned bounding box (AABB). The `x` and `y` fields give the top-left
corner (in screen coordinates where Y increases downward). `width` and `height`
give the extent. All are `f64`.

```
Rect { x: f64, y: f64, width: f64, height: f64 }
```

**Coordinate convention**: top-left origin, Y increases downward (screen space).
This matches SVG, Canvas, Core Graphics, and every major 2D drawing API.

### Construction

```
Rect::new(x: f64, y: f64, width: f64, height: f64) → Rect
```

```
Rect::from_points(min: Point, max: Point) → Rect
```
Construct from the two corners. `min` is the top-left (smaller x, smaller y),
`max` is the bottom-right. Width = max.x - min.x, height = max.y - min.y.

```
Rect::zero() → Rect
```
The empty rect at the origin: `{0.0, 0.0, 0.0, 0.0}`.

### Corner Accessors

```
min(self) → Point
```
Top-left corner: `Point::new(x, y)`.

```
max(self) → Point
```
Bottom-right corner: `Point::new(x + width, y + height)`.

```
center(self) → Point
```
`Point::new(x + width/2.0, y + height/2.0)`.

### Geometric Predicates

```
is_empty(self) → bool
```
True if `width <= 0.0` or `height <= 0.0`. An empty rect has no area.

```
contains_point(self, p: Point) → bool
```
True if $x \le p.x < x + \text{width}$ and $y \le p.y < y + \text{height}$.

Note the **half-open interval**: the top-left edge is inclusive, the
bottom-right edge is exclusive. This convention (used by Java AWT, Direct2D,
and HTML Canvas) avoids double-counting pixels when adjacent rects tile a
surface.

### Set Operations

```
union(self, other: Rect) → Rect
```
The smallest axis-aligned rectangle that contains both `self` and `other`.

```
min_x = min(self.x, other.x)
min_y = min(self.y, other.y)
max_x = max(self.x + self.width, other.x + other.width)
max_y = max(self.y + self.height, other.y + other.height)
→ Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
```

If either rect is empty, the union is the other rect. Implementations should
handle the `is_empty()` case explicitly.

```
intersection(self, other: Rect) → Option<Rect>
```
The region where both rects overlap. Returns `None`/`null`/`nil` if they do
not overlap.

```
ix = max(self.x, other.x)
iy = max(self.y, other.y)
iw = min(self.x + self.width,  other.x + other.width)  - ix
ih = min(self.y + self.height, other.y + other.height) - iy
if iw <= 0 or ih <= 0: return None
→ Some(Rect::new(ix, iy, iw, ih))
```

**Worked example**:
```
A = Rect::new(0.0, 0.0, 10.0, 10.0)   // covers [0,10) × [0,10)
B = Rect::new(5.0, 5.0, 10.0, 10.0)   // covers [5,15) × [5,15)
A.intersection(B) = Some(Rect::new(5.0, 5.0, 5.0, 5.0))
                    // covers [5,10) × [5,10) — the 5×5 overlap
```

```
expand_by(self, amount: f64) → Rect
```
Grow all four edges outward by `amount`. The resulting rect is larger by
`2*amount` in each dimension, and its origin shifts by `(-amount, -amount)`:

```
Rect::new(x - amount, y - amount, width + 2*amount, height + 2*amount)
```

Used for: adding padding around a bounding box, computing stroke bounding
boxes (a stroke of width `w` expands the fill bounding box by `w/2` on each
side), conservative intersection tests.

---

## Cross-Language Implementation Notes

### Option / null / nil / Maybe

The `intersection` function returns an optional rect (no overlap = no value).
Each language expresses this differently:

| Language   | Return type                     | None value         |
|------------|---------------------------------|--------------------|
| Rust       | `Option<Rect>`                  | `None`             |
| TypeScript | `Rect \| null`                  | `null`             |
| Python     | `Optional[Rect]`                | `None`             |
| Go         | `(*Rect, bool)` or `*Rect`      | `nil` / `false`    |
| Ruby       | `Rect?` (returns nil on miss)   | `nil`              |
| Elixir     | `{:ok, Rect} \| :none`          | `:none`            |
| Lua        | `Rect \| nil`                   | `nil`              |
| Perl       | `Rect \| undef`                 | `undef`            |
| Swift      | `Rect?`                         | `nil`              |

### Struct vs. class vs. record

Point and Rect are **value types** — copying them is the right default behavior.
In Rust, they derive `Copy`. In Go, they are bare structs. In TypeScript,
Elixir, and Lua they are plain objects/maps. In Ruby they may be `Struct` or
`Data` (frozen value objects). In Python they should be `@dataclass(frozen=True)`
or a namedtuple-style class.

### trig dependency

The `angle()` function must call `trig.atan2(y, x)` from PHY00. No other
function in this package requires `trig`. Do not call the host language's
`Math.atan2` / `math.atan2` directly — the package dependency structure
requires all trigonometric computation to flow through PHY00.

---

## Required Test Coverage

Every language implementation must include tests validating:

1. **Origin**: `Point::origin()` has x=0.0, y=0.0.
2. **Add/subtract**: `Point::new(1,2).add(Point::new(3,4)) == Point::new(4,6)`.
3. **Scale**: `Point::new(3,4).scale(2.0) == Point::new(6,8)`.
4. **Dot product**: `Point::new(1,0).dot(Point::new(0,1)) == 0.0` (perpendicular).
5. **Cross product sign**: `Point::new(1,0).cross(Point::new(0,1)) == 1.0` (CCW).
6. **Magnitude**: `Point::new(3,4).magnitude() ≈ 5.0`.
7. **Magnitude squared**: `Point::new(3,4).magnitude_squared() == 25.0` (exact).
8. **Normalize**: `Point::new(3,4).normalize() ≈ Point::new(0.6, 0.8)`.
9. **Normalize zero**: `Point::origin().normalize() == Point::origin()`.
10. **Distance**: `Point::new(0,0).distance(Point::new(3,4)) ≈ 5.0`.
11. **Lerp midpoint**: `Point::new(0,0).lerp(Point::new(10,10), 0.5) == Point::new(5,5)`.
12. **Perpendicular**: `Point::new(1,0).perpendicular() == Point::new(0,1)`.
13. **Angle right**: `Point::new(1,0).angle() ≈ 0.0`.
14. **Angle up**: `Point::new(0,1).angle() ≈ π/2`.
15. **Rect contains**: `Rect::new(0,0,10,10).contains_point(Point::new(5,5)) == true`.
16. **Rect boundary**: `Rect::new(0,0,10,10).contains_point(Point::new(10,10)) == false` (exclusive).
17. **Rect union**: union of two non-overlapping rects encompasses both.
18. **Rect intersection overlap**: two overlapping rects return `Some` with correct dimensions.
19. **Rect intersection miss**: two non-overlapping rects return `None`.
20. **Rect expand**: `Rect::new(1,1,8,8).expand_by(1.0) == Rect::new(0,0,10,10)`.

Coverage threshold: ≥ 95% lines.

---

## Package Matrix

| Language   | Directory                                     | Module/Namespace                          |
|------------|-----------------------------------------------|-------------------------------------------|
| Rust       | `code/packages/rust/point2d/`                 | `point2d`                                 |
| TypeScript | `code/packages/typescript/point2d/`           | `@coding-adventures/point2d`              |
| Python     | `code/packages/python/point2d/`               | `point2d`                                 |
| Ruby       | `code/packages/ruby/point2d/`                 | `CodingAdventures::Point2D`               |
| Go         | `code/packages/go/point2d/`                   | `point2d`                                 |
| Elixir     | `code/packages/elixir/point2d/`               | `CodingAdventures.Point2D`                |
| Lua        | `code/packages/lua/point2d/`                  | `coding_adventures.point2d`               |
| Perl       | `code/packages/perl/point2d/`                 | `CodingAdventures::Point2D`               |
| Swift      | `code/packages/swift/point2d/`                | `Point2D`                                 |

## G2D Series Roadmap

| Spec   | Package    | Description                                              |
|--------|------------|----------------------------------------------------------|
| G2D00  | `point2d`  | 2D point/vector primitive and axis-aligned bounding box  |
| G2D01  | `affine2d` | 6-float affine transformation matrix                     |
| G2D02  | `bezier2d` | Quadratic and cubic Bezier curves                        |
| G2D03  | `arc2d`    | Elliptical arcs, endpoint↔center form conversion         |
